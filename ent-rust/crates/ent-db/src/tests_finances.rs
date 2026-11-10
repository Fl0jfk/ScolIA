//! Socle 2, lot S2-9 : finances reliées, contre la base locale de test (feature
//! `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.
//!
//! Chaque scénario crée un établissement neuf (`seed_labo_cast`) et ses propres
//! élèves / foyers de test (aucune dépendance aux factures du jeu de labo).
//! Année facturée : 2030-2031 (lointaine : tout le calendrier est à venir).

use super::*;
use crate::finances_reactions::CONSOMMATEUR_FACTURATION;
use crate::outbox::{ConfigDistribution, Distributeur, Publier};
use chrono::{Duration, NaiveDate, Utc};
use domain_eleve::CreerMandatSepa;
use ent_auth::Role;
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::finances::*;
use serde_json::json;
use uuid::Uuid;

const ANNEE: &str = "2030-2031";

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_mail_dry_run();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn distributeur(ent: &PgEnt, etab: Uuid) -> Distributeur {
    ent.distributeur(ConfigDistribution {
        taille_lot: 100,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    })
}

fn d(a: i32, m: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, j).unwrap()
}

fn compta(c: &LaboCast) -> (Uuid, &'static str) {
    (c.compta.user_id, Role::Compta.as_str())
}

fn direction(c: &LaboCast) -> (Uuid, &'static str) {
    (c.direction.user_id, Role::Direction.as_str())
}

async fn exec(ent: &PgEnt, sql: &str, ids: &[Uuid]) {
    let mut q = sqlx::query(sql);
    for i in ids {
        q = q.bind(*i);
    }
    q.execute(ent.pool()).await.unwrap();
}

/// Correction de date de **test** sur une pièce figée (F0 : les déclencheurs d'immutabilité
/// la refuseraient) : on suspend les déclencheurs le temps de l'instruction, comme le
/// ferait un administrateur de base lors d'une reprise. Jamais utilisé par le code produit.
async fn exec_corrige(ent: &PgEnt, sql: &str, ids: &[Uuid]) {
    let mut tx = ent.pool().begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    let mut q = sqlx::query(sql);
    for i in ids {
        q = q.bind(*i);
    }
    q.execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}

async fn compter(ent: &PgEnt, sql: &str, id: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql)
        .bind(id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    n
}

async fn eleve(ent: &PgEnt, etab: Uuid, prenom: &str, naissance: NaiveDate, regime: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, regime, date_naissance)
         VALUES ($1, $2, 'Testfin', $3, 'actif', $4, $5)",
    )
    .bind(id)
    .bind(etab)
    .bind(prenom)
    .bind(regime)
    .bind(naissance)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

async fn foyer(ent: &PgEnt, etab: Uuid, libelle: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_foyer (id, etablissement_id, libelle) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(etab)
        .bind(libelle)
        .execute(ent.pool())
        .await
        .unwrap();
    id
}

/// Payeur avec compte (la personne est créée par le déclencheur S2-1).
async fn payeur_compte(ent: &PgEnt, etab: Uuid, eleve: Uuid, user: Uuid, foyer: Uuid, part: i16) {
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, foyer_id, lien, est_payeur, part_paiement)
         VALUES ($1, $2, $3, $4, $5, 'mere', true, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve)
    .bind(user)
    .bind(foyer)
    .bind(part)
    .execute(ent.pool())
    .await
    .unwrap();
}

/// Payeur sans compte (parent séparé, D-014 / D-020).
async fn payeur_personne(ent: &PgEnt, etab: Uuid, eleve: Uuid, nom: &str, foyer: Uuid, part: i16) {
    let p = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom) VALUES ($1, $2, $3)")
        .bind(p)
        .bind(etab)
        .bind(nom)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, foyer_id, lien, est_payeur, part_paiement)
         VALUES ($1, $2, $3, $4, $5, 'pere', true, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve)
    .bind(p)
    .bind(foyer)
    .bind(part)
    .execute(ent.pool())
    .await
    .unwrap();
}

struct Grille {
    scol: Uuid,
    dp: Uuid,
}

/// Articles et tarifs : scolarité 10 000,01 € (montant impair : arrondis),
/// demi-pension 3 000 €, internat 5 000 € ; scolarité réduite pour un quotient < 500 €.
async fn grille(ent: &PgEnt, c: &LaboCast) -> Grille {
    let (u, r) = compta(c);
    let etab = c.etablissement_id;
    let art = |code: &str, cat: &str, compte: &str| ArticleBody {
        code: code.into(),
        libelle: format!("Article {code}"),
        categorie: cat.into(),
        compte_produit: Some(compte.into()),
        actif: None,
    };
    let scol = ent
        .enregistrer_article_for_actor(etab, u, r, None, &art("SCOL", "scolarite", "706100"))
        .await
        .unwrap();
    let dp = ent
        .enregistrer_article_for_actor(etab, u, r, None, &art("DP", "restauration", "706200"))
        .await
        .unwrap();
    let int = ent
        .enregistrer_article_for_actor(etab, u, r, None, &art("INT", "internat", "706300"))
        .await
        .unwrap();
    for (article, montant, qf) in [
        (scol.id, 1_000_001, None),
        (scol.id, 600_000, Some((0, 50_000))),
        (dp.id, 300_000, None),
        (int.id, 500_000, None),
    ] {
        ent.creer_tarif_for_actor(
            etab,
            u,
            r,
            &TarifBody {
                annee_scolaire: ANNEE.into(),
                article_id: article,
                niveau: None,
                regime: None,
                qf_min_centimes: qf.map(|q: (i64, i64)| q.0),
                qf_max_centimes: qf.map(|q| q.1),
                montant_annuel_centimes: montant,
            },
        )
        .await
        .unwrap();
    }
    Grille {
        scol: scol.id,
        dp: dp.id,
    }
}

/// Publie `inscription.validee` comme le fait S2-3 (même transaction que la validation).
async fn inscription_validee(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Uuid {
    let mut tx = ent.pool().begin().await.unwrap();
    let e = tx
        .publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::INSCRIPTION_VALIDEE.into(),
            aggregate: AggregateKind::Eleve,
            aggregate_id: eleve,
            eleve_id: Some(eleve),
            payload: json!({ "eleve_id": eleve, "nature": "admission", "annee_cible": ANNEE }),
            actor_user_id: None,
        })
        .await
        .unwrap();
    tx.commit().await.unwrap();
    e.id
}

async fn lignes_foyer(ent: &PgEnt, etab: Uuid, foyer: Uuid) -> Vec<(String, i64, Option<String>)> {
    sqlx::query_as(
        "SELECT l.source_cle, l.montant_unitaire_centimes * l.quantite, l.categorie
           FROM ent_fac_ligne l JOIN ent_fac_facture f ON f.id = l.facture_id
          WHERE f.etablissement_id = $1 AND f.foyer_id = $2 AND f.type_piece = 'facture'
          ORDER BY l.source_cle",
    )
    .bind(etab)
    .bind(foyer)
    .fetch_all(ent.pool())
    .await
    .unwrap()
}

async fn brouillons(ent: &PgEnt, etab: Uuid, foyer: Uuid, type_piece: &str) -> Vec<Uuid> {
    sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM ent_fac_facture WHERE etablissement_id = $1 AND foyer_id = $2
            AND statut = 'brouillon' AND type_piece = $3 ORDER BY created_at",
    )
    .bind(etab)
    .bind(foyer)
    .bind(type_piece)
    .fetch_all(ent.pool())
    .await
    .unwrap()
    .into_iter()
    .map(|r| r.0)
    .collect()
}

/// Rentrée facturée sans ressaisie : inscription validée → lignes annuelles
/// réparties entre deux foyers payeurs (60/40) au centime ; rejeu, double
/// livraison et génération annuelle après coup ne doublent rien ; émission.
#[tokio::test]
async fn rentree_facturee_multi_payeurs_idempotente() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let g = grille(&ent, &c).await;
    let eleve = eleve(&ent, etab, "Alice", d(2018, 3, 1), "demi_pension").await;
    let (fa, fb) = (
        foyer(&ent, etab, "Foyer A").await,
        foyer(&ent, etab, "Foyer B").await,
    );
    payeur_compte(&ent, etab, eleve, c.parent_lea.user_id, fa, 60).await;
    payeur_personne(&ent, etab, eleve, "Martin", fb, 40).await;

    inscription_validee(&ent, etab, eleve).await;
    let dist = distributeur(&ent, etab);
    dist.distribuer_tout().await.unwrap();

    let la = lignes_foyer(&ent, etab, fa).await;
    let lb = lignes_foyer(&ent, etab, fb).await;
    assert_eq!(la.len(), 2, "scolarité + demi-pension pour A : {la:?}");
    assert_eq!(lb.len(), 2);
    let somme = |l: &[(String, i64, Option<String>)], cat: &str| -> i64 {
        l.iter()
            .filter(|x| x.2.as_deref() == Some(cat))
            .map(|x| x.1)
            .sum()
    };
    // 10 000,01 € × 60 % = 6 000,006 → plus fort reste : A 600 001, B 400 000.
    assert_eq!(somme(&la, "scolarite"), 600_001);
    assert_eq!(somme(&lb, "scolarite"), 400_000);
    assert_eq!(somme(&la, "scolarite") + somme(&lb, "scolarite"), 1_000_001);
    assert_eq!(
        somme(&la, "restauration") + somme(&lb, "restauration"),
        300_000
    );
    assert!(la
        .iter()
        .all(|l| l.0.starts_with(&format!("annuel:{ANNEE}:{eleve}"))));

    // Double distribution, relivraison forcée et rejeu du consommateur : rien de plus.
    dist.distribuer_tout().await.unwrap();
    sqlx::query("DELETE FROM ent_event_traitement WHERE consommateur = $1")
        .bind(CONSOMMATEUR_FACTURATION)
        .execute(ent.pool())
        .await
        .unwrap();
    dist.rejouer_consommateur(CONSOMMATEUR_FACTURATION, etab, 0)
        .await
        .unwrap();
    dist.distribuer_tout().await.unwrap();
    // Génération annuelle directe (même chemin que FIN-02) : rien de plus.
    let mut tx = ent.pool().begin().await.unwrap();
    let r = crate::finances::facturer_annee_eleve(
        &mut tx,
        etab,
        ANNEE,
        eleve,
        None,
        None,
        Default::default(),
    )
    .await
    .unwrap()
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!((r.creees, r.deja), (0, 4));
    assert_eq!(lignes_foyer(&ent, etab, fa).await.len(), 2);
    assert_eq!(lignes_foyer(&ent, etab, fb).await.len(), 2);
    assert_eq!(brouillons(&ent, etab, fa, "facture").await.len(), 1);

    // Aperçu de génération : aucune écriture.
    let (u, rl) = compta(&c);
    let ap = ent
        .generer_annee_for_actor(
            etab,
            u,
            rl,
            &GenerationBody {
                annee_scolaire: ANNEE.into(),
                apercu: true,
            },
        )
        .await
        .unwrap();
    assert!(ap.apercu);
    assert_eq!(lignes_foyer(&ent, etab, fa).await.len(), 2);

    // Émission (FIN-03 ★) : numéro continu, échéancier = total, notification au payeur avec compte.
    let pieces = [
        brouillons(&ent, etab, fa, "facture").await,
        brouillons(&ent, etab, fb, "facture").await,
    ]
    .concat();
    let em = ent
        .emettre_for_actor(
            etab,
            u,
            rl,
            &EmissionBody {
                factures: pieces.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(em.emises.len(), 2, "{:?}", em.refus);
    assert!(em
        .emises
        .iter()
        .all(|f| f.numero.starts_with('F') && f.statut == "emise"));
    for f in &em.emises {
        let det = ent
            .facture_detail_for_actor(etab, u, rl, f.id)
            .await
            .unwrap();
        assert_eq!(det.echeances.len(), 10, "10 mois à venir");
        assert_eq!(
            det.echeances
                .iter()
                .map(|e| e.montant_centimes)
                .sum::<i64>(),
            f.total_centimes
        );
    }
    let em2 = ent
        .emettre_for_actor(etab, u, rl, &EmissionBody { factures: pieces })
        .await
        .unwrap();
    assert!(
        em2.emises.is_empty() && em2.refus.len() == 2,
        "réémission refusée"
    );
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'fin.facture'", c.parent_lea.user_id).await,
        1
    );
    let _ = g.scol;
}

/// Prorata d'entrée, fratrie, quotient familial, bourse.
#[tokio::test]
async fn prorata_fratrie_quotient_bourse() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let _g = grille(&ent, &c).await;
    ent.creer_reduction_for_actor(
        etab,
        u,
        r,
        &ReductionBody {
            annee_scolaire: ANNEE.into(),
            code: "FRAT2".into(),
            libelle: "Fratrie".into(),
            nature: "fratrie".into(),
            rang_min: Some(2),
            taux_bps: 1_000,
            categories: vec!["scolarite".into()],
        },
    )
    .await
    .unwrap();
    let f = foyer(&ent, etab, "Foyer fratrie").await;
    let aine = eleve(&ent, etab, "Aine", d(2015, 1, 1), "externe").await;
    let cadet = eleve(&ent, etab, "Cadet", d(2018, 1, 1), "externe").await;
    payeur_personne(&ent, etab, aine, "Durand", f, 100).await;
    payeur_personne(&ent, etab, cadet, "Durand2", f, 100).await;
    // Quotient < 500 € : tarif réduit de scolarité (600 000).
    sqlx::query(
        "INSERT INTO ent_foyer_quotient (id, etablissement_id, foyer_id, annee_scolaire, quotient_centimes, parts_x100, updated_by_user_id)
         VALUES ($1, $2, $3, $4, 30000, 300, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(f)
    .bind(ANNEE)
    .bind(u)
    .execute(ent.pool())
    .await
    .unwrap();
    // Bourse acceptée avec procuration, barème 1 200 € (bourses.gerer).
    let bt = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_bourse_type (id, etablissement_id, code, libelle) VALUES ($1, $2, 'NAT', 'Bourse nationale')")
        .bind(bt)
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_bourse (id, etablissement_id, eleve_id, bourse_type_id, echelon, avec_procuration, status, date_debut)
         VALUES ($1, $2, $3, $4, '3', true, 'acceptee', '2030-09-01')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(aine)
    .bind(bt)
    .execute(ent.pool())
    .await
    .unwrap();
    ent.poser_bareme_bourse_for_actor(
        etab,
        u,
        r,
        &BaremeBourseBody {
            annee_scolaire: ANNEE.into(),
            bourse_type_id: bt,
            echelon: "3".into(),
            montant_annuel_centimes: 120_000,
        },
    )
    .await
    .unwrap();

    // F0 (FD-12) : deux demi-pensionnaires boursiers — bourse de 1 200 € (≤ restauration 3 000 €,
    // imputée en entier) et de 4 000 € (plafonnée à la restauration : 3 000 €, jamais sur la scolarité).
    sqlx::query(
        "INSERT INTO ent_fin_bourse_bareme (id, etablissement_id, annee_scolaire, bourse_type_id, echelon, montant_annuel_centimes)
         VALUES ($1, $2, $3, $4, '9', 400000)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(ANNEE)
    .bind(bt)
    .execute(ent.pool())
    .await
    .unwrap();
    let f_dp = foyer(&ent, etab, "Foyer DP bourse").await;
    let dp_petite = eleve(&ent, etab, "DpPetite", d(2016, 1, 1), "demi_pension").await;
    let dp_grande = eleve(&ent, etab, "DpGrande", d(2016, 2, 2), "demi_pension").await;
    payeur_personne(&ent, etab, dp_petite, "Dupont", f_dp, 100).await;
    payeur_personne(&ent, etab, dp_grande, "Dupont2", f_dp, 100).await;
    for (el, ech) in [(dp_petite, "3"), (dp_grande, "9")] {
        sqlx::query(
            "INSERT INTO ent_eleve_bourse (id, etablissement_id, eleve_id, bourse_type_id, echelon, avec_procuration, status, date_debut)
             VALUES ($1, $2, $3, $4, $5, true, 'acceptee', '2030-09-01')",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(el)
        .bind(bt)
        .bind(ech)
        .execute(ent.pool())
        .await
        .unwrap();
    }
    let mut tx = ent.pool().begin().await.unwrap();
    for el in [dp_petite, dp_grande] {
        crate::finances::facturer_annee_eleve(&mut tx, etab, ANNEE, el, None, None, Default::default())
            .await
            .unwrap()
            .unwrap();
    }
    tx.commit().await.unwrap();
    let ldp = lignes_foyer(&ent, etab, f_dp).await;
    let bourse_de = |el: Uuid| -> i64 {
        ldp.iter()
            .filter(|x| x.0.contains(&el.to_string()) && x.0.contains(":bourse:"))
            .map(|x| x.1)
            .sum()
    };
    assert_eq!(bourse_de(dp_petite), -120_000, "bourse de 1 200 € imputée en entier sur la restauration");
    assert_eq!(bourse_de(dp_grande), -300_000, "bourse de 4 000 € plafonnée à la restauration (3 000 €)");
    let restauration: i64 = ldp
        .iter()
        .filter(|x| x.2.as_deref() == Some("restauration"))
        .map(|x| x.1)
        .sum();
    let bourses: i64 = ldp
        .iter()
        .filter(|x| x.2.as_deref() == Some("bourse"))
        .map(|x| x.1)
        .sum();
    assert!(
        -bourses <= restauration,
        "FD-12 : bourses ({bourses}) au-delà des lignes de restauration/internat ({restauration})"
    );

    let mut tx = ent.pool().begin().await.unwrap();
    // Aîné : année entière. Cadet : entrée le 20 novembre → 8 mois sur 10.
    crate::finances::facturer_annee_eleve(
        &mut tx,
        etab,
        ANNEE,
        aine,
        None,
        None,
        Default::default(),
    )
    .await
    .unwrap()
    .unwrap();
    crate::finances::facturer_annee_eleve(
        &mut tx,
        etab,
        ANNEE,
        cadet,
        Some(d(2030, 11, 20)),
        None,
        Default::default(),
    )
    .await
    .unwrap()
    .unwrap();
    tx.commit().await.unwrap();
    let l = lignes_foyer(&ent, etab, f).await;
    let ligne = |prefixe: String| -> Vec<i64> {
        l.iter()
            .filter(|x| x.0.starts_with(&prefixe))
            .map(|x| x.1)
            .collect()
    };
    // Aîné : tarif quotient 6 000 € ; pas de fratrie (rang 1) ; F0 (FD-12, règle provisoire) :
    // externe, donc aucune ligne de restauration ni d'internat : la bourse (−1 200 €) n'est
    // plus imputée sur la scolarité.
    assert_eq!(
        l.iter()
            .filter(|x| x.0.contains(&aine.to_string()) && x.2.as_deref() == Some("scolarite"))
            .map(|x| x.1)
            .sum::<i64>(),
        600_000
    );
    assert_eq!(
        ligne(format!("annuel:{ANNEE}:{aine}:bourse")),
        Vec::<i64>::new(),
        "FD-12 : bourse imputée sur la scolarité d'un externe"
    );
    // Cadet : 6 000 € × 8/10 = 4 800 € ; fratrie 10 % = −480 €.
    let cadet_scol: i64 = l
        .iter()
        .filter(|x| x.0.contains(&cadet.to_string()) && x.2.as_deref() == Some("scolarite"))
        .map(|x| x.1)
        .sum();
    assert_eq!(cadet_scol, 480_000);
    let cadet_red: i64 = l
        .iter()
        .filter(|x| x.0.contains(&cadet.to_string()) && x.2.as_deref() == Some("reduction"))
        .map(|x| x.1)
        .sum();
    assert_eq!(cadet_red, -48_000);
    // Montants entiers en centimes partout.
    let (total,): (i64,) = sqlx::query_as(
        "SELECT total_centimes FROM ent_fac_facture WHERE etablissement_id = $1 AND foyer_id = $2",
    )
    .bind(etab)
    .bind(f)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(total, 600_000 + 480_000 - 48_000);
}

/// Changement de régime, sortie, absence prolongée : avoirs exacts, idempotents,
/// imputés à l'émission ; facture + avoirs = dû.
#[tokio::test]
async fn avoirs_regime_sortie_absence() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (du, dr) = direction(&c);
    let g = grille(&ent, &c).await;
    let eleve = eleve(&ent, etab, "Bob", d(2017, 5, 5), "demi_pension").await;
    let (fa, fb) = (
        foyer(&ent, etab, "Foyer A").await,
        foyer(&ent, etab, "Foyer B").await,
    );
    payeur_compte(&ent, etab, eleve, c.parent_lea.user_id, fa, 60).await;
    payeur_personne(&ent, etab, eleve, "Martin", fb, 40).await;
    inscription_validee(&ent, etab, eleve).await;
    let dist = distributeur(&ent, etab);
    dist.distribuer_tout().await.unwrap();
    let factures = [
        brouillons(&ent, etab, fa, "facture").await,
        brouillons(&ent, etab, fb, "facture").await,
    ]
    .concat();
    ent.emettre_for_actor(
        etab,
        u,
        r,
        &EmissionBody {
            factures: factures.clone(),
        },
    )
    .await
    .unwrap();

    // Demi-pension → externe le 15 décembre : sept → déc dus (4 mois sur 10).
    let ev = ent
        .changer_regime_for_actor(
            etab,
            du,
            dr,
            eleve,
            &RegimeBody {
                regime: "externe".into(),
                date_effet: d(2030, 12, 15),
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        ent.changer_regime_for_actor(
            etab,
            du,
            dr,
            eleve,
            &RegimeBody {
                regime: "externe".into(),
                date_effet: d(2030, 12, 15)
            }
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    dist.distribuer_tout().await.unwrap();
    let avoirs_a = brouillons(&ent, etab, fa, "avoir").await;
    assert_eq!(avoirs_a.len(), 1);
    let det = ent
        .facture_detail_for_actor(etab, u, r, avoirs_a[0])
        .await
        .unwrap();
    // DP de A = 180 000 ; dû 4/10 = 72 000 ; avoir 108 000.
    assert_eq!(det.facture.total_centimes, -108_000);
    assert_eq!(det.facture.facture_origine_id, Some(factures[0]));
    // Rejeu de l'événement de régime : rien de plus.
    sqlx::query("DELETE FROM ent_event_traitement WHERE event_id = $1")
        .bind(ev.evenement_id)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_outbox SET statut = 'a_distribuer' WHERE event_id = $1")
        .bind(ev.evenement_id)
        .execute(ent.pool())
        .await
        .unwrap();
    dist.distribuer_tout().await.unwrap();
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_fac_ligne WHERE facture_id = $1",
            avoirs_a[0]
        )
        .await,
        1
    );

    // Émission des avoirs : imputés sur la facture d'origine (lettrage « avoir »).
    let avoirs = [avoirs_a.clone(), brouillons(&ent, etab, fb, "avoir").await].concat();
    let em = ent
        .emettre_for_actor(etab, u, r, &EmissionBody { factures: avoirs })
        .await
        .unwrap();
    assert_eq!(em.emises.len(), 2, "{:?}", em.refus);
    let fa_det = ent
        .facture_detail_for_actor(etab, u, r, factures[0])
        .await
        .unwrap();
    assert_eq!(fa_det.facture.regle_centimes, 108_000);
    assert_eq!(
        fa_det.facture.reste_centimes,
        fa_det.facture.total_centimes - 108_000
    );

    // Sortie le 10 mars : scolarité sept → mars (7 mois) due.
    ent.enregistrer_sortie_for_actor(
        etab,
        du,
        dr,
        eleve,
        &SortieBody {
            date_sortie: d(2031, 3, 10),
            motif: Some("déménagement".into()),
        },
    )
    .await
    .unwrap();
    dist.distribuer_tout().await.unwrap();
    let avoir_sortie = brouillons(&ent, etab, fa, "avoir").await;
    assert_eq!(avoir_sortie.len(), 1);
    let det = ent
        .facture_detail_for_actor(etab, u, r, avoir_sortie[0])
        .await
        .unwrap();
    // Scolarité A 600 001 : dû 7/10 = 420 000 (arrondi inférieur) ; avoir 180 001.
    // Demi-pension A : déjà ramenée à 72 000 (4 mois) : rien à créditer de plus.
    assert_eq!(det.facture.total_centimes, -180_001);
    assert_eq!(det.lignes.len(), 1);
    let scol_a: i64 = lignes_foyer(&ent, etab, fa)
        .await
        .iter()
        .filter(|x| x.2.as_deref() == Some("scolarite"))
        .map(|x| x.1)
        .sum();
    assert_eq!(
        scol_a + det.facture.total_centimes,
        420_000,
        "facture + avoir = dû exact"
    );
    // Le motif de sortie n'est pas dans l'événement.
    let (payload,): (serde_json::Value,) = sqlx::query_as(
        "SELECT payload FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'eleve.sortie'",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(!payload.to_string().contains("déménagement"));

    // Absence prolongée justifiée (remise d'ordre) sur un autre élève demi-pensionnaire.
    let e2 = eleve_dp_facture(&ent, &c, fa).await;
    let abs = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_absence (id, etablissement_id, eleve_id, debut, fin, motif) VALUES ($1, $2, $3, '2030-10-01', '2030-10-20', (SELECT code FROM ent_absence_motif LIMIT 1))")
        .bind(abs)
        .bind(etab)
        .bind(e2)
        .execute(ent.pool())
        .await
        .unwrap();
    let mut tx = ent.pool().begin().await.unwrap();
    tx.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: types::ELEVE_ABSENCE_JUSTIFIED.into(),
        aggregate: AggregateKind::Absence,
        aggregate_id: abs,
        eleve_id: Some(e2),
        payload: json!({ "statut": "justifiee" }),
        actor_user_id: None,
    })
    .await
    .unwrap();
    tx.commit().await.unwrap();
    dist.distribuer_tout().await.unwrap();
    let (credit,): (i64,) = sqlx::query_as(
        "SELECT coalesce(sum(montant_unitaire_centimes), 0)::BIGINT FROM ent_fac_ligne
          WHERE etablissement_id = $1 AND source_cle LIKE 'absence:' || $2 || ':%'",
    )
    .bind(etab)
    .bind(abs.to_string())
    .fetch_one(ent.pool())
    .await
    .unwrap();
    // 3 000 € × 20 jours / 303 jours (1er sept → 30 juin) = 198,01 € (arrondi inférieur).
    assert_eq!(credit, -(300_000_i64 * 20 / 303));
    let _ = g.dp;
}

/// Élève demi-pensionnaire facturé à 100 % au foyer `f`, facture émise.
async fn eleve_dp_facture(ent: &PgEnt, c: &LaboCast, f: Uuid) -> Uuid {
    let etab = c.etablissement_id;
    let (u, r) = compta(c);
    let e = eleve(ent, etab, "Carla", d(2016, 6, 6), "demi_pension").await;
    payeur_personne(ent, etab, e, "Leroy", f, 100).await;
    let mut tx = ent.pool().begin().await.unwrap();
    crate::finances::facturer_annee_eleve(&mut tx, etab, ANNEE, e, None, None, Default::default())
        .await
        .unwrap()
        .unwrap();
    tx.commit().await.unwrap();
    let b = brouillons(ent, etab, f, "facture").await;
    ent.emettre_for_actor(etab, u, r, &EmissionBody { factures: b })
        .await
        .unwrap();
    e
}

/// SEPA de bout en bout : pré-notification, export pain.008, rejet (frais,
/// re-présentation puis retour au paiement manuel), exécution, règlement
/// manuel et lettrage, relances confirmées, export comptable équilibré.
#[tokio::test]
async fn sepa_rejets_reglements_relances_export() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    grille(&ent, &c).await;
    // F0 (FD-11) : le compte créancier ne se change qu'au niveau direction ; la compta ne peut pas.
    let (ud, rd) = direction(&c);
    assert!(matches!(
        ent.modifier_parametres_finances_for_actor(
            etab,
            u,
            r,
            &ParametresFinancesBody {
                crediteur_iban: Some("FR1420041010050500013M02606".into()),
                ..Default::default()
            },
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    ent.modifier_parametres_finances_for_actor(
        etab,
        ud,
        rd,
        &ParametresFinancesBody {
            crediteur_nom: Some("Lycée de test".into()),
            crediteur_iban: Some("FR1420041010050500013M02606".into()),
            crediteur_bic: Some("PSSTFRPPXXX".into()),
            crediteur_ics: Some("FR12ZZZ123456".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let eleve = eleve(&ent, etab, "Dora", d(2017, 2, 2), "externe").await;
    let (fa, fb) = (
        foyer(&ent, etab, "Foyer A").await,
        foyer(&ent, etab, "Foyer B").await,
    );
    payeur_compte(&ent, etab, eleve, c.parent_lea.user_id, fa, 50).await;
    payeur_personne(&ent, etab, eleve, "Martin", fb, 50).await;
    // Mandats du jeu de labo révoqués : seul le foyer A de ce test est prélevable.
    exec(
        &ent,
        "UPDATE ent_fac_sepa_mandat SET statut = 'revoke' WHERE etablissement_id = $1",
        &[etab],
    )
    .await;
    let mandat = ent
        .creer_mandat_sepa_for_actor(
            Role::Compta.as_str(),
            u,
            CreerMandatSepa {
                etablissement_id: etab,
                foyer_id: fa,
                rum: format!("RUM-{}", &Uuid::new_v4().simple().to_string()[..10]),
                iban: "FR7630006000011234567890189".into(),
                bic: None,
                titulaire: "Parent A".into(),
                signe_at: Utc::now(),
                actor_user_id: u,
            },
        )
        .await
        .unwrap();
    inscription_validee(&ent, etab, eleve).await;
    distributeur(&ent, etab).distribuer_tout().await.unwrap();
    let factures = [
        brouillons(&ent, etab, fa, "facture").await,
        brouillons(&ent, etab, fb, "facture").await,
    ]
    .concat();
    ent.emettre_for_actor(
        etab,
        u,
        r,
        &EmissionBody {
            factures: factures.clone(),
        },
    )
    .await
    .unwrap();

    // Délai de pré-notification : refusé à J+3.
    let trop_tot = Utc::now().date_naive() + Duration::days(3);
    assert!(matches!(
        ent.preparer_prelevements_for_actor(
            etab,
            u,
            r,
            &PreparationBody {
                date_execution: trop_tot
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let date = d(2030, 9, 30);
    let rap = ent
        .preparer_prelevements_for_actor(
            etab,
            u,
            r,
            &PreparationBody {
                date_execution: date,
            },
        )
        .await
        .unwrap();
    assert_eq!(rap.prelevements_crees, 1, "première échéance de A");
    assert_eq!(rap.prenotifications, 1);
    assert!(
        rap.ecartes
            .iter()
            .any(|e| e.foyer_id == fb && e.motif.contains("mandat")),
        "B sans mandat : écarté"
    );
    let again = ent
        .preparer_prelevements_for_actor(
            etab,
            u,
            r,
            &PreparationBody {
                date_execution: date,
            },
        )
        .await
        .unwrap();
    assert_eq!(again.prelevements_crees, 0, "idempotent");
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'fin.prenotification'", c.parent_lea.user_id).await,
        1
    );
    let prevs = ent
        .prelevements_for_actor(etab, u, r, None, Some(date))
        .await
        .unwrap();
    assert!(
        prevs[0].iban_masque.contains("••••") && !prevs[0].iban_masque.contains("300060000112345")
    );

    // Export pain.008 : IBAN complet seulement dans le fichier, BIC absent → NOTPROVIDED.
    let (xml, nature) = ent
        .exporter_prelevements_for_actor(
            etab,
            u,
            r,
            &ExportPrelevementsBody {
                date_execution: date,
                format: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(nature, "pain008");
    let xml = String::from_utf8(xml).unwrap();
    // Validation hors suite (xmllint + schéma ISO) : `ENT_PAIN008_SORTIE=/chemin/fichier.xml`.
    if let Ok(chemin) = std::env::var("ENT_PAIN008_SORTIE") {
        std::fs::write(chemin, &xml).unwrap();
    }
    assert!(xml.contains("<NbOfTxs>1</NbOfTxs>"));
    assert!(xml.contains("FR7630006000011234567890189"));
    assert!(xml.contains("<SeqTp>FRST</SeqTp>"));
    assert!(xml.contains("<Othr><Id>NOTPROVIDED</Id></Othr>"));
    assert!(xml.contains(&mandat.rum));
    assert!(matches!(
        ent.exporter_prelevements_for_actor(
            etab,
            u,
            r,
            &ExportPrelevementsBody {
                date_execution: date,
                format: None
            }
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));

    // Rejet AM04 (provision insuffisante) : re-présentation ; F0 (FD-13) : aucun frais facturé.
    let rej = ent
        .traiter_rejet_for_actor(
            etab,
            u,
            r,
            prevs[0].id,
            &RejetBody {
                code: "AM04".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(rej.suite, "representation");
    assert_eq!(rej.frais_centimes, 0);
    assert!(rej.facture_frais_id.is_none(), "FD-13 : facture de frais émise");
    assert!(matches!(
        ent.traiter_rejet_for_actor(
            etab,
            u,
            r,
            prevs[0].id,
            &RejetBody {
                code: "AM04".into()
            }
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(matches!(
        ent.traiter_rejet_for_actor(
            etab,
            u,
            r,
            prevs[0].id,
            &RejetBody {
                code: "ZZ99".into()
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'fin.rejet'", c.parent_lea.user_id).await,
        1
    );
    // Échéance rouverte : re-présentée (présentation 2), sans facture de frais.
    let rap2 = ent
        .preparer_prelevements_for_actor(
            etab,
            u,
            r,
            &PreparationBody {
                date_execution: date,
            },
        )
        .await
        .unwrap();
    assert_eq!(rap2.prelevements_crees, 1, "échéance rejetée seulement");
    let prevs2: Vec<PrelevementDto> = ent
        .prelevements_for_actor(etab, u, r, Some("prevu".into()), Some(date))
        .await
        .unwrap();
    assert!(prevs2.iter().any(|p| p.presentation == 2));
    ent.exporter_prelevements_for_actor(
        etab,
        u,
        r,
        &ExportPrelevementsBody {
            date_execution: date,
            format: Some("csv".into()),
        },
    )
    .await
    .unwrap();
    // Second rejet : seuil atteint (2) → retour au paiement manuel.
    let p2 = prevs2.iter().find(|p| p.presentation == 2).unwrap();
    let rej2 = ent
        .traiter_rejet_for_actor(
            etab,
            u,
            r,
            p2.id,
            &RejetBody {
                code: "AM04".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(rej2.suite, "manuel");
    let compte_a = ent.compte_foyer_for_actor(etab, u, r, fa).await.unwrap();
    assert_eq!(compte_a.mode_paiement, "manuel");
    let rap3 = ent
        .preparer_prelevements_for_actor(
            etab,
            u,
            r,
            &PreparationBody {
                date_execution: d(2030, 10, 30),
            },
        )
        .await
        .unwrap();
    assert_eq!(rap3.prelevements_crees, 0);
    assert!(rap3
        .ecartes
        .iter()
        .any(|e| e.foyer_id == fa && e.motif == "paiement manuel"));

    // Règlement manuel de B (chèque) : facture soldée, lettrée ; trop-perçu au crédit.
    let fb_total = ent
        .facture_detail_for_actor(etab, u, r, factures[1])
        .await
        .unwrap()
        .facture
        .total_centimes;
    let reg = ent
        .enregistrer_reglement_for_actor(
            etab,
            u,
            r,
            &ReglementBody {
                foyer_id: fb,
                montant_centimes: fb_total + 1_000,
                moyen: "cheque".into(),
                reference: Some("CHQ 123".into()),
                recu_le: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(reg.non_impute_centimes, 1_000);
    assert!(reg.imputations[0].lettrage.is_some());
    let compte_b = ent.compte_foyer_for_actor(etab, u, r, fb).await.unwrap();
    assert_eq!(compte_b.solde_centimes, 0);
    assert_eq!(compte_b.credit_centimes, 1_000);

    // Relances : échéance de A en retard de 40 jours → palier 2 (30 j) proposé ; jamais avant l'échéance.
    let aucune = ent.proposer_relances_for_actor(etab, u, r).await.unwrap();
    assert!(aucune.proposees.is_empty(), "aucune échéance passée");
    exec_corrige(&ent, "UPDATE ent_fac_echeance SET date_echeance = (now() - interval '40 days')::date WHERE facture_id = $1 AND rang = 1", &[factures[0]]).await;
    let rel = ent.proposer_relances_for_actor(etab, u, r).await.unwrap();
    assert_eq!(rel.proposees.len(), 1);
    assert_eq!(rel.proposees[0].palier, 2);
    assert!(
        ent.proposer_relances_for_actor(etab, u, r)
            .await
            .unwrap()
            .proposees
            .is_empty(),
        "pas de doublon"
    );
    // Rien n'est envoyé avant confirmation humaine.
    let avant = compter(&ent, "SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'fin.relance'", c.parent_lea.user_id).await;
    assert_eq!(avant, 0);
    let conf = ent
        .decider_relance_for_actor(etab, u, r, rel.proposees[0].id, true)
        .await
        .unwrap();
    assert_eq!(conf.statut, "envoyee");
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'fin.relance'", c.parent_lea.user_id).await,
        1
    );
    assert!(matches!(
        ent.decider_relance_for_actor(etab, u, r, rel.proposees[0].id, false)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));

    // Export comptable : période close seulement, équilibré, chaque pièce une fois.
    let hier = Utc::now().date_naive() - Duration::days(1);
    assert!(matches!(
        ent.exporter_comptabilite_for_actor(
            etab,
            u,
            r,
            &ExportComptableBody {
                debut: hier,
                fin: Utc::now().date_naive()
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    exec_corrige(&ent, "UPDATE ent_fac_facture SET emise_at = now() - interval '1 day' WHERE etablissement_id = $1 AND emise_at IS NOT NULL", &[etab]).await;
    exec_corrige(&ent, "UPDATE ent_fac_encaissement SET encaisse_at = now() - interval '1 day' WHERE etablissement_id = $1 AND encaisse_at > now() - interval '1 day'", &[etab]).await;
    let exp = ent
        .exporter_comptabilite_for_actor(
            etab,
            u,
            r,
            &ExportComptableBody {
                debut: hier - Duration::days(5),
                fin: hier,
            },
        )
        .await
        .unwrap();
    assert_eq!(exp.total_debit_centimes, exp.total_credit_centimes);
    assert!(exp.nb_ecritures >= 6);
    let contenu = exp.contenu.unwrap();
    assert!(contenu.starts_with(crate::finances_calcul::ENTETE_ECRITURES));
    assert!(
        contenu.contains("|706100|"),
        "compte de produit de l'article"
    );
    assert!(
        !contenu.contains("FR7630006000011234567890189"),
        "aucun IBAN dans la comptabilité"
    );
    assert_eq!(exp.empreinte.len(), 64);
    assert!(matches!(
        ent.exporter_comptabilite_for_actor(
            etab,
            u,
            r,
            &ExportComptableBody {
                debut: hier - Duration::days(5),
                fin: hier
            }
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));

    // Espace famille : le payeur A voit ses pièces émises, IBAN masqué, paiement en ligne désactivé.
    let fam = ent
        .espace_finances_famille(etab, c.parent_lea.user_id)
        .await
        .unwrap();
    let compte = fam
        .comptes
        .iter()
        .find(|x| x.foyer_id == fa)
        .expect("compte du foyer A");
    assert!(
        fam.comptes.iter().all(|x| x.foyer_id != fb),
        "foyer B (autre payeur) invisible"
    );
    assert!(compte.factures.iter().all(|f| f.statut != "brouillon"));
    assert!(!compte.echeances.is_empty());
    assert!(compte.mandats[0].iban_masque.starts_with("FR76"));
    assert!(!serde_json::to_string(&fam)
        .unwrap()
        .contains("300060000112345"));
    assert!(!fam.paiement_en_ligne.unwrap().actif);
    // Un autre parent ne voit pas ce foyer.
    let autre = ent
        .espace_finances_famille(etab, c.parent_tom.user_id)
        .await
        .unwrap();
    assert!(autre
        .comptes
        .iter()
        .all(|c| c.foyer_id != fa && c.foyer_id != fb));
}

/// Droits (moteur D-017) et isolation entre établissements.
#[tokio::test]
async fn droits_et_isolation() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let _g = grille(&ent, &c).await;
    let e = eleve(&ent, etab, "Eva", d(2017, 7, 7), "externe").await;
    let f = foyer(&ent, etab, "Foyer E").await;
    payeur_personne(&ent, etab, e, "Petit", f, 100).await;
    let mut tx = ent.pool().begin().await.unwrap();
    crate::finances::facturer_annee_eleve(&mut tx, etab, ANNEE, e, None, None, Default::default())
        .await
        .unwrap()
        .unwrap();
    tx.commit().await.unwrap();
    let facture = brouillons(&ent, etab, f, "facture").await[0];

    // Lecture et gestion : comptabilité, administratif, direction ; refus prof, CPE, parent.
    for (user, role) in [
        (c.compta.user_id, Role::Compta),
        (c.administratif.user_id, Role::Administratif),
        (c.direction.user_id, Role::Direction),
    ] {
        ent.factures_for_actor(etab, user, role.as_str(), None, None, None)
            .await
            .unwrap();
    }
    for (user, role) in [
        (c.prof.user_id, Role::Prof),
        (c.cpe.user_id, Role::Cpe),
        (c.parent_lea.user_id, Role::Parent),
    ] {
        assert!(matches!(
            ent.factures_for_actor(etab, user, role.as_str(), None, None, None)
                .await,
            Err(EntDbError::Forbidden)
        ));
        assert!(matches!(
            ent.emettre_for_actor(
                etab,
                user,
                role.as_str(),
                &EmissionBody {
                    factures: vec![facture]
                }
            )
            .await,
            Err(EntDbError::Forbidden)
        ));
        assert!(matches!(
            ent.preparer_prelevements_for_actor(
                etab,
                user,
                role.as_str(),
                &PreparationBody {
                    date_execution: d(2030, 9, 30)
                }
            )
            .await,
            Err(EntDbError::Forbidden)
        ));
    }
    // Régime / sortie : `eleves.gerer` (pas la comptabilité).
    assert!(matches!(
        ent.changer_regime_for_actor(
            etab,
            u,
            r,
            e,
            &RegimeBody {
                regime: "interne".into(),
                date_effet: d(2030, 10, 1)
            }
        )
        .await,
        Err(EntDbError::Forbidden)
    ));

    // Autre établissement : rien de A n'est visible ni modifiable.
    let c2 = ent.seed_labo_cast().await.unwrap();
    let (u2, r2) = compta(&c2);
    assert!(ent
        .facture_detail_for_actor(c2.etablissement_id, u2, r2, facture)
        .await
        .is_err());
    let em = ent
        .emettre_for_actor(
            c2.etablissement_id,
            u2,
            r2,
            &EmissionBody {
                factures: vec![facture],
            },
        )
        .await;
    assert!(
        em.is_err(),
        "facture d'un autre établissement : introuvable"
    );
    assert!(ent
        .compte_foyer_for_actor(c2.etablissement_id, u2, r2, f)
        .await
        .is_err());
    assert!(ent
        .factures_for_actor(c2.etablissement_id, u2, r2, None, None, None)
        .await
        .unwrap()
        .iter()
        .all(|x| x.id != facture));
    let (du2, dr2) = direction(&c2);
    assert!(ent
        .changer_regime_for_actor(
            c2.etablissement_id,
            du2,
            dr2,
            e,
            &RegimeBody {
                regime: "interne".into(),
                date_effet: d(2030, 10, 1)
            }
        )
        .await
        .is_err());
    // Événement forgé dans B citant l'élève de A : aucune ligne. Depuis la clé composée d'E0
    // (migration 240), la base refuse déjà la publication elle-même.
    let mut tx = ent.pool().begin().await.unwrap();
    let forge = tx
        .publier(MetierEventDraft {
        etablissement_id: c2.etablissement_id,
        type_name: types::INSCRIPTION_VALIDEE.into(),
        aggregate: AggregateKind::Eleve,
        aggregate_id: e,
        eleve_id: Some(e),
        payload: json!({ "eleve_id": e, "annee_cible": ANNEE }),
        actor_user_id: None,
    })
    .await;
    assert!(forge.is_err(), "événement inter-établissements accepté");
    drop(tx);
    distributeur(&ent, c2.etablissement_id)
        .distribuer_tout()
        .await
        .unwrap();
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_fac_facture WHERE etablissement_id = $1 AND origine = 'annuelle'", c2.etablissement_id).await,
        0
    );
    // Brouillons invisibles des familles même payeuses.
    payeur_compte(&ent, etab, e, c.parent_tom.user_id, f, 0).await;
    let fam = ent
        .espace_finances_famille(etab, c.parent_tom.user_id)
        .await
        .unwrap();
    assert!(fam
        .comptes
        .iter()
        .flat_map(|c| &c.factures)
        .all(|x| x.statut != "brouillon"));
}

/// RLS (migrations 100 et 206) : sous un rôle applicatif non propriétaire en
/// mode strict, les tables S2-9 ne montrent que l'établissement posé par TenantTx.
#[tokio::test]
async fn rls_tables_finances() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    grille(&ent, &c).await;
    ent.modifier_parametres_finances_for_actor(
        etab,
        u,
        r,
        &ParametresFinancesBody {
            jours_prenotification: Some(15),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    // Toutes les tables ent_fin_* portent la politique.
    let (sans,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM pg_tables t WHERE t.schemaname = 'public' AND t.tablename LIKE 'ent_fin\\_%'
            AND NOT EXISTS (SELECT 1 FROM pg_policies p WHERE p.tablename = t.tablename AND p.policyname = 'ent_rls_etablissement')",
    )
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(sans, 0, "table ent_fin_* sans politique RLS");

    let url = std::env::var("ENT_DATABASE_URL").unwrap();
    const ROLE: &str = "ent_rls_fin";
    for sql in [
        format!("DO $$ BEGIN CREATE ROLE {ROLE} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS; EXCEPTION WHEN duplicate_object THEN NULL; END $$"),
        format!("GRANT USAGE ON SCHEMA public TO {ROLE}"),
        format!("GRANT SELECT, INSERT ON ent_fin_parametres, ent_fin_article, ent_fin_tarif TO {ROLE}"),
        format!("ALTER ROLE {ROLE} SET app.rls_strict = 'on'"),
    ] {
        sqlx::query(&sql).execute(ent.pool()).await.unwrap();
    }
    let mut url = url::Url::parse(&url).unwrap();
    url.set_username(ROLE).unwrap();
    url.set_password(Some("rls")).unwrap();
    let app = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(url.as_str())
        .await
        .unwrap();
    use crate::tenant_tx::{RlsMode, TenantTx};
    // Sans établissement posé (mode strict) : rien.
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_fin_article")
        .fetch_one(&app)
        .await
        .unwrap();
    assert_eq!(n, 0);
    // Établissement A : ses articles ; établissement B : aucun.
    let mut tx = TenantTx::begin_mode(&app, etab, RlsMode::Strict)
        .await
        .unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_fin_article")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(n, 3);
    tx.commit().await.unwrap();
    let autre = Uuid::new_v4();
    let mut tx = TenantTx::begin_mode(&app, autre, RlsMode::Strict)
        .await
        .unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_fin_tarif")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(n, 0);
    // Écriture pour un autre établissement que celui posé : refusée (WITH CHECK).
    let r = sqlx::query("INSERT INTO ent_fin_parametres (etablissement_id) VALUES ($1)")
        .bind(Uuid::new_v4())
        .execute(&mut *tx)
        .await;
    assert!(r.is_err(), "écriture hors établissement refusée par la RLS");
}
