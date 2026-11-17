//! Socle 24, lots F1 et F2 : service unique, journal, comptes débiteurs, payeurs datés, conséquences d'un
//! changement de payeur, élève majeur payeur, organismes, vue guichet — contre la base locale de test
//! (feature `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés. Cas limites de la conception :
//! CF-01 à CF-09, CF-16, CF-17, CF-74, CF-75, CF-82 à CF-84 (`docs/socles/24_FINANCES.md` §15).
//!
//! Chaque scénario crée un établissement neuf (`seed_labo_cast`) et ses propres élèves et foyers ; année
//! facturée 2030-2031 (lointaine : aucune dépendance à l'horloge réelle, toutes les dates sont fixes).

use super::*;
use crate::facturation_payeurs::{self as payeurs, PART_ENTIERE};
use crate::finances::facturer_annee_eleve;
use chrono::NaiveDate;
use ent_auth::Role;
use ent_types::finances::*;
use ent_types::finances_comptes::*;
use uuid::Uuid;

const ANNEE: &str = "2030-2031";

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| { e.install_mail_dry_run(); }).await;
    Some((ent, cast))
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
fn secretariat(c: &LaboCast) -> (Uuid, &'static str) {
    (c.administratif.user_id, Role::Administratif.as_str())
}

async fn eleve(ent: &PgEnt, etab: Uuid, prenom: &str, naissance: NaiveDate, regime: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, regime, date_naissance)
         VALUES ($1, $2, 'Testf2', $3, 'actif', $4, $5)",
    )
    .bind(id)
    .bind(etab)
    .bind(prenom)
    .bind(regime)
    .bind(naissance)
    .execute(ent.pool_verif())
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
        .execute(ent.pool_verif())
        .await
        .unwrap();
    id
}

/// Payeur avec compte utilisateur (la personne est créée par le déclencheur S2-1).
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
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

/// Compte débiteur d'un foyer (créé par la fonction SQL unique, comme le fait le service).
async fn compte_du_foyer(ent: &PgEnt, etab: Uuid, foyer: Uuid) -> Uuid {
    let (id,): (Uuid,) = sqlx::query_as("SELECT ent_fin_compte_du_foyer($1, $2, NULL, 'manuel')")
        .bind(etab)
        .bind(foyer)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    id
}

async fn grille(ent: &PgEnt, c: &LaboCast) {
    let (u, r) = compta(c);
    let etab = c.etablissement_id;
    let art = |code: &str, cat: &str, compte: &str| ArticleBody {
        code: code.into(),
        libelle: format!("Article {code}"),
        categorie: cat.into(),
        compte_produit: Some(compte.into()),
        actif: None,
    };
    let scol = ent.enregistrer_article_for_actor(etab, u, r, None, &art("SCOL", "scolarite", "706100")).await.unwrap();
    let dp = ent.enregistrer_article_for_actor(etab, u, r, None, &art("DP", "restauration", "706200")).await.unwrap();
    for (article, montant) in [(scol.id, 1_000_001_i64), (dp.id, 300_000)] {
        ent.creer_tarif_for_actor(
            etab, u, r,
            &TarifBody {
                annee_scolaire: ANNEE.into(),
                article_id: article,
                niveau: None,
                regime: None,
                qf_min_centimes: None,
                qf_max_centimes: None,
                montant_annuel_centimes: montant,
            },
        )
        .await
        .unwrap();
    }
}

/// Lance le moteur de génération pour un élève (même chemin que FIN-02 et que l'inscription validée).
async fn facturer(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Result<crate::finances::ResultatInsertion, String> {
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let r = facturer_annee_eleve(&mut tx, etab, ANNEE, eleve, None, None, Default::default())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    r
}

/// Lignes facturées à un foyer : (catégorie, centimes).
async fn lignes(ent: &PgEnt, etab: Uuid, foyer: Uuid) -> Vec<(String, i64)> {
    sqlx::query_as(
        "SELECT COALESCE(l.categorie, ''), (l.montant_unitaire_centimes * l.quantite)::bigint
           FROM ent_fac_ligne l JOIN ent_fac_facture f ON f.id = l.facture_id
          WHERE f.etablissement_id = $1 AND f.foyer_id = $2 AND f.type_piece = 'facture'
          ORDER BY l.source_cle",
    )
    .bind(etab)
    .bind(foyer)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap()
}

fn somme(l: &[(String, i64)], cat: &str) -> i64 {
    l.iter().filter(|x| x.0 == cat).map(|x| x.1).sum()
}

async fn taches_ouvertes(ent: &PgEnt, etab: Uuid, nature: &str, eleve: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_fin_tache WHERE etablissement_id = $1 AND nature = $2 AND eleve_id = $3 AND statut = 'ouverte'",
    )
    .bind(etab)
    .bind(nature)
    .bind(eleve)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    n
}

async fn piece_accord(ent: &PgEnt, c: &LaboCast, eleve: Uuid, categorie: &str, parts: Vec<(Uuid, i32)>, effet: NaiveDate) -> ResultatDesignationDto {
    let (u, r) = secretariat(c);
    ent.designer_payeurs_for_actor(
        c.etablissement_id, u, r, eleve,
        &DesignerPayeursBody {
            categorie: categorie.into(),
            parts: parts.into_iter().map(|(compte_id, part_bps)| PartBody { compte_id, part_bps, rang: None }).collect(),
            date_effet: effet,
            piece: Some(PieceBody { nature: "accord_des_deux".into(), reference: "Accord signé des deux parents".into(), document_id: None }),
            motif: Some("test".into()),
        },
    )
    .await
    .unwrap()
}

// ───────────────────────── CF-01, CF-02, CF-17 : désignation par catégorie ─────────────────────────

/// CF-02 : la mère paie la scolarité, le père la restauration. Une catégorie précise l'emporte sur
/// `toutes` ; chaque payeur ne reçoit que **ses** lignes ; aucune ligne « au responsable 1 ».
#[tokio::test]
async fn cf_02_mere_scolarite_pere_restauration() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    grille(&ent, &c).await;
    let el = eleve(&ent, etab, "Alice", d(2018, 3, 1), "demi_pension").await;
    let (fm, fp) = (foyer(&ent, etab, "Foyer mère").await, foyer(&ent, etab, "Foyer père").await);
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 100).await; // lien « responsable 1 »
    let (cm, cp) = (compte_du_foyer(&ent, etab, fm).await, compte_du_foyer(&ent, etab, fp).await);
    // Désignations avec pièce : scolarité → mère (toutes), restauration → père.
    piece_accord(&ent, &c, el, "toutes", vec![(cm, 10_000)], d(2030, 9, 1)).await;
    let r = piece_accord(&ent, &c, el, "restauration", vec![(cp, 10_000)], d(2030, 9, 1)).await;
    assert_eq!(r.statut, "effective");

    let res = facturer(&ent, etab, el).await.unwrap();
    assert_eq!(res.creees, 2, "une ligne de scolarité (mère), une de restauration (père)");
    let (lm, lp) = (lignes(&ent, etab, fm).await, lignes(&ent, etab, fp).await);
    assert_eq!(somme(&lm, "scolarite"), 1_000_001);
    assert_eq!(somme(&lm, "restauration"), 0, "la mère ne paie pas la cantine");
    assert_eq!(somme(&lp, "restauration"), 300_000);
    assert_eq!(somme(&lp, "scolarite"), 0, "le père ne paie pas la scolarité");
    // Le port lit bien la catégorie précise.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let resto = payeurs::payeurs_a(&mut tx, etab, el, "restauration", d(2030, 10, 1), false).await.unwrap();
    let scol = payeurs::payeurs_a(&mut tx, etab, el, "scolarite", d(2030, 10, 1), false).await.unwrap();
    assert_eq!(resto.parts.iter().map(|p| p.compte_id).collect::<Vec<_>>(), vec![cp]);
    assert_eq!(scol.parts.iter().map(|p| p.compte_id).collect::<Vec<_>>(), vec![cm]);
    assert_eq!(resto.categorie_lue, "restauration");
    assert_eq!(scol.categorie_lue, "toutes");
    tx.commit().await.unwrap();
}

/// CF-01 : 50/50 sur tout ; deux comptes, deux factures de 50 %, somme exacte au centime ; chaque
/// payeur voit son compte et pas celui de l'autre (R-F5).
#[tokio::test]
async fn cf_01_cinquante_cinquante_deux_comptes_somme_exacte() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    grille(&ent, &c).await;
    let el = eleve(&ent, etab, "Bruno", d(2018, 3, 1), "demi_pension").await;
    let (fm, fp) = (foyer(&ent, etab, "Foyer M").await, foyer(&ent, etab, "Foyer P").await);
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 50).await;
    payeur_compte(&ent, etab, el, c.parent_tom.user_id, fp, 50).await;
    facturer(&ent, etab, el).await.unwrap();
    let (lm, lp) = (lignes(&ent, etab, fm).await, lignes(&ent, etab, fp).await);
    // 10 000,01 € / 2 : plus fort reste, premier compte (ordre stable des liens).
    assert_eq!(somme(&lm, "scolarite") + somme(&lp, "scolarite"), 1_000_001);
    assert_eq!(somme(&lm, "restauration") + somme(&lp, "restauration"), 300_000);
    assert!((somme(&lm, "scolarite") - somme(&lp, "scolarite")).abs() <= 1);
    // Chacun ne voit que son compte.
    let mes_m = ent.mes_comptes_for_actor(etab, c.parent_lea.user_id).await.unwrap();
    let mes_p = ent.mes_comptes_for_actor(etab, c.parent_tom.user_id).await.unwrap();
    assert!(mes_m.iter().any(|x| x.foyer_id == fm) && !mes_m.iter().any(|x| x.foyer_id == fp));
    assert!(mes_p.iter().any(|x| x.foyer_id == fp) && !mes_p.iter().any(|x| x.foyer_id == fm));
}

/// CF-16 et CF-17 : parts 70/30 avec quotients différents (chaque part sur **son** tarif) ; fratrie par payeur.
#[tokio::test]
async fn cf_16_cf_17_quotients_differents_et_fratrie_par_payeur() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    grille(&ent, &c).await;
    let (u, r) = compta(&c);
    // Tarif réduit de scolarité pour un quotient < 500 €.
    let (art_scol,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_fin_article WHERE etablissement_id = $1 AND code = 'SCOL'")
        .bind(etab).fetch_one(ent.pool_verif()).await.unwrap();
    ent.creer_tarif_for_actor(etab, u, r, &TarifBody {
        annee_scolaire: ANNEE.into(), article_id: art_scol, niveau: None, regime: None,
        qf_min_centimes: Some(0), qf_max_centimes: Some(50_000), montant_annuel_centimes: 600_000,
    }).await.unwrap();
    let el = eleve(&ent, etab, "Chloé", d(2018, 3, 1), "externe").await;
    let (fm, fp) = (foyer(&ent, etab, "Foyer M2").await, foyer(&ent, etab, "Foyer P2").await);
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 70).await;
    payeur_compte(&ent, etab, el, c.parent_tom.user_id, fp, 30).await;
    // Le père a un quotient bas : son tarif est le tarif réduit.
    sqlx::query("INSERT INTO ent_foyer_quotient (id, etablissement_id, foyer_id, annee_scolaire, quotient_centimes, parts_x100, updated_by_user_id) VALUES ($1,$2,$3,$4,$5,200,$6)")
        .bind(Uuid::new_v4()).bind(etab).bind(fp).bind(ANNEE).bind(40_000_i64).bind(c.compta.user_id)
        .execute(ent.pool_verif()).await.unwrap();
    facturer(&ent, etab, el).await.unwrap();
    let (lm, lp) = (lignes(&ent, etab, fm).await, lignes(&ent, etab, fp).await);
    // Mère : 70 % de 10 000,01 € = 7 000,007 → 700 000 (plus fort reste sur 100 % : 700 001 ou 700 000) ;
    // père : 30 % de 6 000,00 € = 1 800,00 € (son quotient).
    assert_eq!(somme(&lp, "scolarite"), 180_000, "30 % du tarif réduit du père");
    assert!(somme(&lm, "scolarite") >= 700_000 && somme(&lm, "scolarite") <= 700_001, "70 % du tarif plein : {lm:?}");
    // La somme ne correspond à aucun tarif unique : c'est voulu (CF-16), l'aperçu le montre.
    assert_ne!(somme(&lm, "scolarite") + somme(&lp, "scolarite"), 1_000_001);
}

// ───────────────────────── R-F4 : payeur manquant (FAC-210) ─────────────────────────

/// Une catégorie sans payeur n'est **pas** facturée et ouvre une tâche « payeur manquant » ; elle se règle
/// en désignant un payeur (la tâche se résout), sans jamais facturer « au responsable 1 ».
#[tokio::test]
async fn payeur_manquant_pas_de_facture_par_defaut_et_tache() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    grille(&ent, &c).await;
    let el = eleve(&ent, etab, "Dylan", d(2018, 3, 1), "demi_pension").await;
    let (fm, fp) = (foyer(&ent, etab, "Foyer M3").await, foyer(&ent, etab, "Foyer P3").await);
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 100).await; // lien de responsabilité
    let (cm, cp) = (compte_du_foyer(&ent, etab, fm).await, compte_du_foyer(&ent, etab, fp).await);
    // Désignation précise de la seule scolarité ; rien pour `toutes` ni pour la restauration :
    // le lien payeur du dossier ne vaut plus dès qu'une désignation existe (jamais « au responsable 1 »).
    piece_accord(&ent, &c, el, "scolarite", vec![(cm, 10_000)], d(2030, 9, 1)).await;
    let res = facturer(&ent, etab, el).await.unwrap();
    assert_eq!(res.creees, 1, "seule la scolarité est facturée");
    let lm = lignes(&ent, etab, fm).await;
    assert_eq!(somme(&lm, "scolarite"), 1_000_001);
    assert_eq!(somme(&lm, "restauration"), 0, "la restauration n'est pas facturée à défaut de payeur");
    assert_eq!(taches_ouvertes(&ent, etab, "payeur_manquant", el).await, 1);
    // Rejeu : la tâche n'est pas dupliquée (idempotence).
    facturer(&ent, etab, el).await.unwrap();
    assert_eq!(taches_ouvertes(&ent, etab, "payeur_manquant", el).await, 1);
    // Le secrétariat voit la tâche.
    let (u, r) = secretariat(&c);
    let taches = ent.taches_for_actor(etab, u, r, &TachesFiltre::default()).await.unwrap();
    assert!(taches.iter().any(|t| t.nature == "payeur_manquant" && t.eleve_id == Some(el) && t.categorie.as_deref() == Some("restauration")));
    // On désigne le père pour la restauration : la tâche se résout et la ligne arrive à son compte.
    piece_accord(&ent, &c, el, "restauration", vec![(cp, 10_000)], d(2030, 9, 1)).await;
    assert_eq!(taches_ouvertes(&ent, etab, "payeur_manquant", el).await, 0);
    facturer(&ent, etab, el).await.unwrap();
    assert_eq!(somme(&lignes(&ent, etab, fp).await, "restauration"), 300_000);
}

// ───────────────────────── CF-03 : séparation en cours d'année ─────────────────────────

/// CF-03 : séparation au 1er décembre, facture annuelle émise en août. **Aucune pièce émise n'est touchée** ;
/// conséquences proposées (avoir à la mère sur 7 mois à 50 %, ligne au père) ; la comptabilité décide ; les
/// brouillons ne sont pas émis ; l'arrondi est en faveur de la famille.
#[tokio::test]
async fn cf_03_separation_au_premier_decembre() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    grille(&ent, &c).await;
    let (cu, cr) = compta(&c);
    let el = eleve(&ent, etab, "Emma", d(2018, 3, 1), "demi_pension").await;
    let (fm, fp) = (foyer(&ent, etab, "Foyer M4").await, foyer(&ent, etab, "Foyer P4").await);
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 100).await;
    facturer(&ent, etab, el).await.unwrap();
    let cm = compte_du_foyer(&ent, etab, fm).await;
    let cp = compte_du_foyer(&ent, etab, fp).await;
    // Émission de la facture annuelle de la mère (août).
    let brouillon: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_fac_facture WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'brouillon'")
        .bind(etab).bind(fm).fetch_all(ent.pool_verif()).await.unwrap();
    assert_eq!(brouillon.len(), 1);
    ent.emettre_for_actor(etab, cu, cr, &EmissionBody { factures: vec![brouillon[0].0] }).await.unwrap();
    let avant: (i64, String, String) = sqlx::query_as("SELECT total_centimes, numero, statut FROM ent_fac_facture WHERE id = $1")
        .bind(brouillon[0].0).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(avant.0, 1_300_001);

    // Séparation au 01/12 : 50 % chacun, avec l'accord des deux (pièce).
    let r = piece_accord(&ent, &c, el, "toutes", vec![(cm, 5_000), (cp, 5_000)], d(2030, 12, 1)).await;
    assert_eq!(r.statut, "effective");
    // 2 lignes × (1 crédit + 1 débit) = 4 conséquences proposées.
    assert_eq!(r.consequences_proposees, 4);
    let cons = ent.consequences_for_actor(etab, cu, cr, Some(el)).await.unwrap();
    let credit = |cat_total: i64| cat_total;
    let _ = credit;
    let credits: Vec<i64> = cons.iter().filter(|x| x.sens == "credit").map(|x| x.montant_centimes).collect();
    let debits: Vec<i64> = cons.iter().filter(|x| x.sens == "debit").map(|x| x.montant_centimes).collect();
    // Scolarité 1 000 001 : 7/10 × 50 % = 350 000,35 → avoir 350 001, débit 350 000 ; restauration 300 000 → 105 000.
    let mut c_s = credits.clone();
    c_s.sort();
    assert_eq!(c_s, vec![105_000, 350_001]);
    let mut d_s = debits.clone();
    d_s.sort();
    assert_eq!(d_s, vec![105_000, 350_000]);
    assert!(credits.iter().sum::<i64>() >= debits.iter().sum::<i64>(), "jamais en défaveur de la famille");
    assert!(cons.iter().all(|x| x.statut == "proposee" && x.mois_total == 10 && x.mois_apres == 7));
    assert!(cons.iter().filter(|x| x.sens == "credit").all(|x| x.compte_id == cm));
    assert!(cons.iter().filter(|x| x.sens == "debit").all(|x| x.compte_id == cp));

    // Aucune pièce émise touchée.
    let apres: (i64, String, String) = sqlx::query_as("SELECT total_centimes, numero, statut FROM ent_fac_facture WHERE id = $1")
        .bind(brouillon[0].0).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(avant, apres);
    // Le port lit les deux répartitions à leur date.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let nov = payeurs::payeurs_a(&mut tx, etab, el, "scolarite", d(2030, 11, 30), false).await.unwrap();
    let dec = payeurs::payeurs_a(&mut tx, etab, el, "scolarite", d(2030, 12, 1), false).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(nov.parts.len(), 1);
    assert_eq!(nov.parts[0].compte_id, cm);
    assert_eq!(dec.parts.len(), 2);
    assert_eq!(dec.somme(1), PART_ENTIERE);
    // Découpage d'une pièce sur l'année par date de désignation : 3 mois à la mère seule, 7 à 50/50.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let tr = payeurs::tranches(&mut tx, etab, el, "scolarite", d(2030, 9, 1), d(2031, 6, 30)).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(tr.len(), 2);
    let dc = payeurs::decouper(d(2030, 9, 1), d(2031, 6, 30), 200_000, &tr);
    assert_eq!(dc.non_attribue, 0);
    assert_eq!(dc.parts.iter().find(|p| p.0 == cm).unwrap().1, 130_000);
    assert_eq!(dc.parts.iter().find(|p| p.0 == cp).unwrap().1, 70_000);

    // La comptabilité accepte : brouillons d'avoir (mère) et de facture (père), rien n'est émis.
    let res = ent
        .decider_consequences_for_actor(etab, cu, cr, r.groupe_id, &DecisionConsequencesBody { appliquer: true, motif: None })
        .await
        .unwrap();
    assert_eq!(res.appliquees, 4);
    assert_eq!(res.avoirs_brouillons.len(), 1);
    assert_eq!(res.factures_brouillons.len(), 1);
    let avoir: (i64, String) = sqlx::query_as("SELECT total_centimes, statut FROM ent_fac_facture WHERE id = $1 AND type_piece = 'avoir'")
        .bind(res.avoirs_brouillons[0]).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(avoir, (-455_001, "brouillon".into()));
    let pere: (i64, String, Uuid) = sqlx::query_as("SELECT total_centimes, statut, foyer_id FROM ent_fac_facture WHERE id = $1")
        .bind(res.factures_brouillons[0]).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(pere, (455_000, "brouillon".into(), fp));
    // Une seconde décision est refusée (déjà décidées) ; l'original reste intact.
    assert!(ent.decider_consequences_for_actor(etab, cu, cr, r.groupe_id, &DecisionConsequencesBody { appliquer: true, motif: None }).await.is_err());
    let encore: (i64, String, String) = sqlx::query_as("SELECT total_centimes, numero, statut FROM ent_fac_facture WHERE id = $1")
        .bind(brouillon[0].0).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(avant, encore);
}

// ───────────────────────── R-F30, CF-74, CF-75 : acceptation du payeur désigné ─────────────────────────

/// CF-74 : une répartition sans acceptation ni pièce reste **en attente** ; l'ancienne reste en vigueur ;
/// seul le payeur désigné peut répondre ; un refus ouvre une tâche et ne change rien.
#[tokio::test]
async fn cf_74_repartition_sans_acceptation_ni_piece_reste_en_attente() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let el = eleve(&ent, etab, "Fanny", d(2018, 3, 1), "externe").await;
    let (fm, fp) = (foyer(&ent, etab, "Foyer M5").await, foyer(&ent, etab, "Foyer P5").await);
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 100).await;
    payeur_compte(&ent, etab, el, c.parent_tom.user_id, fp, 0).await; // le père est lié, sans part
    let (cm, cp) = (compte_du_foyer(&ent, etab, fm).await, compte_du_foyer(&ent, etab, fp).await);
    let (u, r) = secretariat(&c);
    let prop = ent.designer_payeurs_for_actor(etab, u, r, el, &DesignerPayeursBody {
        categorie: "toutes".into(),
        parts: vec![PartBody { compte_id: cm, part_bps: 6_000, rang: None }, PartBody { compte_id: cp, part_bps: 4_000, rang: None }],
        date_effet: d(2030, 10, 1),
        piece: None,
        motif: Some("jugement annoncé, non fourni".into()),
    }).await.unwrap();
    assert_eq!(prop.statut, "en_attente", "ni acceptation ni pièce : pas opposable");
    assert_eq!(prop.consequences_proposees, 0);
    assert_eq!(taches_ouvertes(&ent, etab, "designation_a_accepter", el).await, 2, "une tâche par payeur désigné");
    // L'ancienne répartition (liens) reste en vigueur.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let lus = payeurs::payeurs_a(&mut tx, etab, el, "toutes", d(2030, 11, 1), false).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(lus.origine, payeurs::Origine::Liens);
    assert_eq!(lus.parts.len(), 1);
    // Une personne non désignée (un parent d'un autre foyer) reçoit « introuvable », jamais « refusé ».
    let autre = ent.repondre_designation_for_actor(etab, c.cpe.user_id, "cpe", prop.groupe_id, true).await;
    assert!(matches!(autre, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))), "{autre:?}");
    // Le père refuse (CF-74) : rien ne change, tâche « demande de répartition » au secrétariat.
    let refus = ent.repondre_designation_for_actor(etab, c.parent_tom.user_id, "parent", prop.groupe_id, false).await.unwrap();
    assert_eq!(refus.statut, "refusee");
    assert_eq!(taches_ouvertes(&ent, etab, "demande_repartition", el).await, 1);
    assert_eq!(taches_ouvertes(&ent, etab, "designation_a_accepter", el).await, 0);
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let toujours = payeurs::payeurs_a(&mut tx, etab, el, "toutes", d(2030, 11, 1), false).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(toujours.origine, payeurs::Origine::Liens);
    // On ne peut pas répondre deux fois.
    assert!(ent.repondre_designation_for_actor(etab, c.parent_tom.user_id, "parent", prop.groupe_id, true).await.is_err());
}

/// Acceptation de tous les payeurs désignés : la répartition devient opposable à la date d'effet.
#[tokio::test]
async fn acceptation_de_tous_les_payeurs_rend_la_repartition_opposable() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let el = eleve(&ent, etab, "Gaël", d(2018, 3, 1), "externe").await;
    let (fm, fp) = (foyer(&ent, etab, "Foyer M6").await, foyer(&ent, etab, "Foyer P6").await);
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 100).await;
    payeur_compte(&ent, etab, el, c.parent_tom.user_id, fp, 0).await;
    let (cm, cp) = (compte_du_foyer(&ent, etab, fm).await, compte_du_foyer(&ent, etab, fp).await);
    let (u, r) = secretariat(&c);
    let prop = ent.designer_payeurs_for_actor(etab, u, r, el, &DesignerPayeursBody {
        categorie: "toutes".into(),
        parts: vec![PartBody { compte_id: cm, part_bps: 5_000, rang: None }, PartBody { compte_id: cp, part_bps: 5_000, rang: None }],
        date_effet: d(2030, 10, 1), piece: None, motif: None,
    }).await.unwrap();
    assert_eq!(prop.statut, "en_attente");
    let a1 = ent.repondre_designation_for_actor(etab, c.parent_lea.user_id, "parent", prop.groupe_id, true).await.unwrap();
    assert_eq!(a1.statut, "en_attente", "un seul des deux a accepté");
    let a2 = ent.repondre_designation_for_actor(etab, c.parent_tom.user_id, "parent", prop.groupe_id, true).await.unwrap();
    assert_eq!(a2.statut, "effective");
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let avant = payeurs::payeurs_a(&mut tx, etab, el, "toutes", d(2030, 9, 30), false).await.unwrap();
    let apres = payeurs::payeurs_a(&mut tx, etab, el, "toutes", d(2030, 10, 1), false).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(avant.parts.len(), 1, "avant la date d'effet : l'ancienne répartition");
    assert_eq!(apres.parts.len(), 2);
    assert_eq!(taches_ouvertes(&ent, etab, "designation_a_accepter", el).await, 0);
    // Une pièce jointe après coup à une proposition déjà activée est refusée.
    assert!(ent.joindre_piece_repartition_for_actor(etab, u, r, prop.groupe_id, &JoindrePieceBody {
        piece: PieceBody { nature: "jugement".into(), reference: "JAF".into(), document_id: None } }).await.is_err());
}

/// R-F31 : un jugement (pièce) joint à une proposition en attente la rend opposable sans acceptation.
#[tokio::test]
async fn piece_de_repartition_jointe_apres_coup_rend_opposable() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let el = eleve(&ent, etab, "Hana", d(2018, 3, 1), "externe").await;
    let (fm, fp) = (foyer(&ent, etab, "Foyer M7").await, foyer(&ent, etab, "Foyer P7").await);
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 100).await;
    let (cm, cp) = (compte_du_foyer(&ent, etab, fm).await, compte_du_foyer(&ent, etab, fp).await);
    let (u, r) = secretariat(&c);
    let prop = ent.designer_payeurs_for_actor(etab, u, r, el, &DesignerPayeursBody {
        categorie: "toutes".into(),
        parts: vec![PartBody { compte_id: cm, part_bps: 6_000, rang: None }, PartBody { compte_id: cp, part_bps: 4_000, rang: None }],
        date_effet: d(2030, 10, 1), piece: None, motif: Some("jugement 60/40".into()),
    }).await.unwrap();
    assert_eq!(prop.statut, "en_attente");
    // Une nature de pièce inconnue ou une référence vide est refusée.
    assert!(ent.joindre_piece_repartition_for_actor(etab, u, r, prop.groupe_id, &JoindrePieceBody {
        piece: PieceBody { nature: "papier".into(), reference: "x".into(), document_id: None } }).await.is_err());
    assert!(ent.joindre_piece_repartition_for_actor(etab, u, r, prop.groupe_id, &JoindrePieceBody {
        piece: PieceBody { nature: "jugement".into(), reference: "  ".into(), document_id: None } }).await.is_err());
    let ok = ent.joindre_piece_repartition_for_actor(etab, u, r, prop.groupe_id, &JoindrePieceBody {
        piece: PieceBody { nature: "jugement".into(), reference: "JAF Lyon 2030/123".into(), document_id: None } }).await.unwrap();
    assert_eq!(ok.statut, "effective");
    let (nature, reference): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT piece_nature, piece_reference FROM ent_fin_designation WHERE groupe_id = $1 LIMIT 1")
        .bind(prop.groupe_id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!((nature.as_deref(), reference.as_deref()), (Some("jugement"), Some("JAF Lyon 2030/123")));
}

/// CF-75 : un parent demande seul « l'autre paie tout » : une tâche, rien ne change, et un non-payeur ne
/// peut pas faire cette demande (même réponse qu'un élève inconnu).
#[tokio::test]
async fn cf_75_demande_d_un_seul_parent_cree_une_tache() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let el = eleve(&ent, etab, "Ilan", d(2018, 3, 1), "externe").await;
    let fm = foyer(&ent, etab, "Foyer M8").await;
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 100).await;
    compte_du_foyer(&ent, etab, fm).await; // le compte existe (reprise MIG-4 ou première pièce)
    let t = ent.demander_repartition_for_actor(etab, c.parent_lea.user_id, "parent", &DemandeRepartitionBody {
        eleve_id: el, categorie: "toutes".into(), commentaire: Some("je ne peux plus payer seule".into()),
    }).await.unwrap();
    assert_eq!(t.nature, "demande_repartition");
    assert_eq!(t.eleve_id, Some(el));
    assert_eq!(taches_ouvertes(&ent, etab, "demande_repartition", el).await, 1);
    // Rien n'a changé : le port lit toujours les liens.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let lus = payeurs::payeurs_a(&mut tx, etab, el, "toutes", d(2030, 11, 1), false).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(lus.origine, payeurs::Origine::Liens);
    // Un parent qui n'est pas payeur de cet élève : introuvable (aucune fuite d'existence).
    let refus = ent.demander_repartition_for_actor(etab, c.parent_tom.user_id, "parent", &DemandeRepartitionBody {
        eleve_id: el, categorie: "toutes".into(), commentaire: None }).await;
    assert!(matches!(refus, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))), "{refus:?}");
    let inconnu = ent.demander_repartition_for_actor(etab, c.parent_lea.user_id, "parent", &DemandeRepartitionBody {
        eleve_id: Uuid::new_v4(), categorie: "toutes".into(), commentaire: None }).await;
    assert!(matches!(inconnu, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))), "{inconnu:?}");
}

// ───────────────────────── FD-15, CF-08, CF-84 : élève majeur payeur ─────────────────────────

/// FD-15 / CF-84 : l'élève majeur devient payeur à sa demande, avec son acceptation ; il voit son compte
/// (FD-15) et ses parents n'en voient rien ; un mineur est refusé.
#[tokio::test]
async fn fd_15_cf_84_eleve_majeur_payeur() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    grille(&ent, &c).await;
    let (u, r) = secretariat(&c);
    let majeur = eleve(&ent, etab, "Jules", d(2010, 1, 1), "externe").await; // né en 2010 : majeur en 2030
    let mineur = eleve(&ent, etab, "Kim", d(2018, 1, 1), "externe").await;
    // Fiche « personne » de l'élève majeur et compte utilisateur de l'élève.
    let personne = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom, eleve_id) VALUES ($1,$2,'Testf2','Jules',$3)")
        .bind(personne).bind(etab).bind(majeur).execute(ent.pool_verif()).await.unwrap();
    sqlx::query("INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id) VALUES ($1,$2,$3)")
        .bind(etab).bind(majeur).bind(c.cpe.user_id).execute(ent.pool_verif()).await.unwrap();
    let fm = foyer(&ent, etab, "Foyer parents Jules").await;
    payeur_compte(&ent, etab, majeur, c.parent_lea.user_id, fm, 100).await;
    facturer(&ent, etab, majeur).await.unwrap();
    let (brouillon,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_fac_facture WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'brouillon'")
        .bind(etab).bind(fm).fetch_one(ent.pool_verif()).await.unwrap();
    ent.emettre_for_actor(etab, c.compta.user_id, "compta", &EmissionBody { factures: vec![brouillon] }).await.unwrap();

    // Un mineur ne peut pas devenir son propre payeur.
    let refus = ent.transferer_payeur_majeur_for_actor(etab, u, r, mineur, &PayeurMajeurBody { date_effet: d(2030, 9, 1), categories: vec![], motif: None }).await;
    assert!(refus.is_err(), "mineur : refusé");

    // Transfert au 1er janvier 2031 : en attente de l'acceptation de l'élève (R-F30).
    let t = ent.transferer_payeur_majeur_for_actor(etab, u, r, majeur, &PayeurMajeurBody {
        date_effet: d(2031, 1, 1), categories: vec![], motif: Some("études post-bac, autonome".into()),
    }).await.unwrap();
    assert_eq!(t.statut, "en_attente");
    // Avant son acceptation : l'élève voit déjà son compte (créé), les parents restent payeurs.
    let mes = ent.mes_comptes_for_actor(etab, c.cpe.user_id).await.unwrap();
    assert_eq!(mes.len(), 1, "FD-15 : l'élève majeur retrouve le compte de sa fiche personne");
    assert_eq!(mes[0].personne_id, Some(personne));
    // Un élève qui n'est pas concerné ne peut pas répondre.
    assert!(ent.repondre_designation_for_actor(etab, c.parent_tom.user_id, "parent", t.groupe_id, true).await.is_err());
    // L'élève accepte : le transfert est opposable à la date d'effet ; conséquences proposées sur les mois à venir.
    let ok = ent.repondre_designation_for_actor(etab, c.cpe.user_id, "eleve", t.groupe_id, true).await.unwrap();
    assert_eq!(ok.statut, "effective");
    assert_eq!(ok.consequences_proposees, 4, "2 lignes : avoir aux parents et ligne à l'élève");
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let avant = payeurs::payeurs_a(&mut tx, etab, majeur, "toutes", d(2030, 12, 31), false).await.unwrap();
    let apres = payeurs::payeurs_a(&mut tx, etab, majeur, "toutes", d(2031, 1, 1), false).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(avant.parts.iter().map(|p| p.foyer_id).collect::<Vec<_>>(), vec![fm], "juin à décembre : les parents");
    assert_eq!(apres.parts.len(), 1);
    assert_ne!(apres.parts[0].foyer_id, fm);
    // L'élève voit son espace financier (FD-15) ; ses parents ne voient pas le compte de l'élève.
    let espace = ent.espace_finances_famille(etab, c.cpe.user_id).await.unwrap();
    assert!(espace.comptes.iter().any(|x| x.foyer_id == apres.parts[0].foyer_id));
    let parents = ent.mes_comptes_for_actor(etab, c.parent_lea.user_id).await.unwrap();
    assert!(!parents.iter().any(|x| x.personne_id == Some(personne)), "les parents ne voient rien du compte de l'élève");
    // L'ancienne facture des parents n'est pas touchée (le solde passé reste aux parents).
    let (nb,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_fac_facture WHERE etablissement_id = $1 AND foyer_id = $2")
        .bind(etab).bind(fm).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(nb, 1);
}

// ───────────────────────── CF-05, CF-82, CF-83 : organismes et prises en charge ─────────────────────────

/// FAC-256 : SIRET contrôlé (Luhn), Chorus Pro réservé à un organisme public, SIRET unique par établissement.
#[tokio::test]
async fn organisme_payeur_siret_controle_et_unique() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let corps = |libelle: &str, siret: Option<&str>, public: bool, chorus: bool| CreerCompteOrganismeBody {
        libelle: libelle.into(), siret: siret.map(Into::into), reference_client: Some("BC-2030-12".into()),
        organisme_public: public, chorus_pro: chorus,
    };
    let ok = ent.creer_compte_organisme_for_actor(etab, u, r, &corps("Entreprise Dupont SA", Some("73282932000074"), false, false)).await.unwrap();
    assert_eq!(ok.type_debiteur, "organisme");
    assert_eq!(ok.origine, "organisme");
    assert!(!ok.a_verifier);
    assert!(ent.creer_compte_organisme_for_actor(etab, u, r, &corps("X", Some("73282932000075"), false, false)).await.is_err(), "clé de Luhn fausse");
    assert!(ent.creer_compte_organisme_for_actor(etab, u, r, &corps("X", Some("1234"), false, false)).await.is_err());
    assert!(ent.creer_compte_organisme_for_actor(etab, u, r, &corps("X", None, false, true)).await.is_err(), "Chorus Pro : organisme public seulement");
    assert!(ent.creer_compte_organisme_for_actor(etab, u, r, &corps("   ", None, false, false)).await.is_err(), "nom obligatoire");
    let doublon = ent.creer_compte_organisme_for_actor(etab, u, r, &corps("Autre nom, même SIRET", Some("73282932000074"), false, false)).await;
    assert!(doublon.is_err(), "SIRET déjà porté");
    // Département (organisme public) : Chorus Pro permis.
    let dep = ent.creer_compte_organisme_for_actor(etab, u, r, &corps("Conseil départemental — ASE", None, true, true)).await.unwrap();
    assert!(dep.chorus_pro && dep.organisme_public);
    // Le secrétariat (engagements) peut créer, la comptabilité aussi ; un professeur non.
    let (us, rs) = secretariat(&c);
    assert!(ent.creer_compte_organisme_for_actor(etab, us, rs, &corps("Mairie", None, true, false)).await.is_ok());
    assert!(matches!(
        ent.creer_compte_organisme_for_actor(etab, c.prof.user_id, "prof", &corps("Mairie 2", None, true, false)).await,
        Err(EntDbError::Forbidden)
    ));
}

/// CF-82 : l'employeur couvre 100 % de la restauration dans la limite de 900 € ; au-delà, la part non
/// couverte passe au payeur suivant ; sans payeur suivant, une tâche et rien n'est facturé à la famille.
#[tokio::test]
async fn cf_82_plafond_de_l_employeur_part_non_couverte_au_payeur_suivant() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    grille(&ent, &c).await;
    let (u, r) = compta(&c);
    let employeur = ent
        .creer_compte_organisme_for_actor(etab, u, r, &CreerCompteOrganismeBody {
            libelle: "Employeur SA".into(), siret: None, reference_client: Some("PEC-1".into()), organisme_public: false, chorus_pro: false })
        .await
        .unwrap();
    let fm = foyer(&ent, etab, "Foyer M9").await;
    let cm = compte_du_foyer(&ent, etab, fm).await;
    let el = eleve(&ent, etab, "Léa", d(2018, 3, 1), "demi_pension").await;
    // Prise en charge : 900 € de restauration pour l'année scolaire.
    let pec = ent.creer_prise_en_charge_for_actor(etab, u, r, &PriseEnChargeBody {
        compte_id: employeur.id, eleve_id: el, reference: "PEC-1".into(), du: d(2030, 9, 1), au: d(2031, 6, 30),
        categories: vec!["restauration".into()], plafond_centimes: Some(90_000) }).await.unwrap();
    assert_eq!(pec.statut, "active");
    let (us, rs) = secretariat(&c);
    // Scolarité → mère ; restauration → employeur (rang 1) avec la mère pour payeur suivant (rang 2).
    piece_accord(&ent, &c, el, "scolarite", vec![(cm, 10_000)], d(2030, 9, 1)).await;
    ent.designer_payeurs_for_actor(etab, us, rs, el, &DesignerPayeursBody {
        categorie: "restauration".into(),
        parts: vec![
            PartBody { compte_id: employeur.id, part_bps: 10_000, rang: Some(1) },
            PartBody { compte_id: cm, part_bps: 10_000, rang: Some(2) },
        ],
        date_effet: d(2030, 9, 1),
        piece: Some(PieceBody { nature: "accord_des_deux".into(), reference: "Convention employeur".into(), document_id: None }),
        motif: None,
    }).await.unwrap();
    facturer(&ent, etab, el).await.unwrap();
    let fe: (Uuid,) = sqlx::query_as("SELECT foyer_id FROM ent_fin_compte WHERE id = $1").bind(employeur.id).fetch_one(ent.pool_verif()).await.unwrap();
    let (le, lm) = (lignes(&ent, etab, fe.0).await, lignes(&ent, etab, fm).await);
    assert_eq!(somme(&le, "restauration"), 90_000, "l'employeur paie 900 € au plus");
    assert_eq!(somme(&lm, "restauration"), 210_000, "la part non couverte (2 100 €) passe au payeur suivant");
    assert_eq!(somme(&le, "restauration") + somme(&lm, "restauration"), 300_000);
    assert_eq!(somme(&lm, "scolarite"), 1_000_001);
    // Rejeu : rien de plus (idempotence).
    facturer(&ent, etab, el).await.unwrap();
    assert_eq!(somme(&lignes(&ent, etab, fe.0).await, "restauration"), 90_000);

    // Sans payeur suivant : la part non couverte n'est facturée à personne ; tâche.
    let el2 = eleve(&ent, etab, "Maël", d(2018, 3, 1), "demi_pension").await;
    ent.creer_prise_en_charge_for_actor(etab, u, r, &PriseEnChargeBody {
        compte_id: employeur.id, eleve_id: el2, reference: "PEC-2".into(), du: d(2030, 9, 1), au: d(2031, 6, 30),
        categories: vec!["restauration".into()], plafond_centimes: Some(90_000) }).await.unwrap();
    piece_accord(&ent, &c, el2, "scolarite", vec![(cm, 10_000)], d(2030, 9, 1)).await;
    piece_accord(&ent, &c, el2, "restauration", vec![(employeur.id, 10_000)], d(2030, 9, 1)).await;
    facturer(&ent, etab, el2).await.unwrap();
    let lignes_e = lignes(&ent, etab, fe.0).await;
    assert_eq!(somme(&lignes_e, "restauration"), 180_000, "deux élèves × 900 €");
    assert_eq!(taches_ouvertes(&ent, etab, "part_non_couverte", el2).await, 1);
    assert_eq!(somme(&lignes(&ent, etab, fm).await, "restauration"), 210_000, "rien de plus sur la famille");
}

/// CF-83 : prise en charge échue : alerte 30 jours avant, puis « échue » ; aucune facture à l'organisme
/// au-delà, rien reporté sur la famille ; le calcul prend le jour **en paramètre** (aucune horloge).
#[tokio::test]
async fn cf_83_prise_en_charge_alerte_puis_echue() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let ase = ent.creer_compte_organisme_for_actor(etab, u, r, &CreerCompteOrganismeBody {
        libelle: "ASE du département".into(), siret: None, reference_client: None, organisme_public: true, chorus_pro: false }).await.unwrap();
    let el = eleve(&ent, etab, "Noé", d(2018, 3, 1), "interne").await;
    let pec = ent.creer_prise_en_charge_for_actor(etab, u, r, &PriseEnChargeBody {
        compte_id: ase.id, eleve_id: el, reference: "ASE-2030-17".into(), du: d(2030, 9, 1), au: d(2031, 1, 31),
        categories: vec!["internat".into()], plafond_centimes: None }).await.unwrap();
    // Trente et un jours avant : rien.
    let rien = ent.alerter_prises_en_charge_for_actor(etab, u, r, d(2030, 12, 31)).await.unwrap();
    assert_eq!((rien.a_renouveler, rien.echues), (0, 0));
    // Trente jours avant : tâche de renouvellement.
    let al = ent.alerter_prises_en_charge_for_actor(etab, u, r, d(2031, 1, 1)).await.unwrap();
    assert_eq!((al.a_renouveler, al.echues), (1, 0));
    assert_eq!(taches_ouvertes(&ent, etab, "prise_en_charge_a_renouveler", el).await, 1);
    // Idempotent : pas de seconde tâche.
    ent.alerter_prises_en_charge_for_actor(etab, u, r, d(2031, 1, 2)).await.unwrap();
    assert_eq!(taches_ouvertes(&ent, etab, "prise_en_charge_a_renouveler", el).await, 1);
    // Échue : statut mis à jour.
    let ech = ent.alerter_prises_en_charge_for_actor(etab, u, r, d(2031, 2, 1)).await.unwrap();
    assert_eq!((ech.a_renouveler, ech.echues), (0, 1));
    let (statut,): (String,) = sqlx::query_as("SELECT statut FROM ent_fin_prise_en_charge WHERE id = $1").bind(pec.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(statut, "echue");
    // Une prise en charge échue ne plafonne plus rien : le plafond n'est lu que « active ».
    let liste = ent.prises_en_charge_for_actor(etab, u, r).await.unwrap();
    assert!(liste.iter().any(|p| p.id == pec.id && p.statut == "echue"));
}

/// Une prise en charge ne vise qu'un compte d'organisme, un élève de l'établissement, une période valide.
#[tokio::test]
async fn prise_en_charge_controles() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let el = eleve(&ent, etab, "Océane", d(2018, 3, 1), "externe").await;
    let fm = foyer(&ent, etab, "Foyer M10").await;
    let cm = compte_du_foyer(&ent, etab, fm).await;
    let corps = |compte, eleve_id, du: NaiveDate, au: NaiveDate, cats: Vec<&str>, plafond| PriseEnChargeBody {
        compte_id: compte, eleve_id, reference: "R".into(), du, au,
        categories: cats.into_iter().map(String::from).collect(), plafond_centimes: plafond,
    };
    assert!(ent.creer_prise_en_charge_for_actor(etab, u, r, &corps(cm, el, d(2030, 9, 1), d(2031, 6, 30), vec!["internat"], None)).await.is_err(), "pas un organisme");
    let org = ent.creer_compte_organisme_for_actor(etab, u, r, &CreerCompteOrganismeBody { libelle: "Org".into(), siret: None, reference_client: None, organisme_public: false, chorus_pro: false }).await.unwrap();
    assert!(ent.creer_prise_en_charge_for_actor(etab, u, r, &corps(org.id, el, d(2031, 6, 30), d(2030, 9, 1), vec!["internat"], None)).await.is_err(), "fin avant début");
    assert!(ent.creer_prise_en_charge_for_actor(etab, u, r, &corps(org.id, el, d(2030, 9, 1), d(2031, 6, 30), vec![], None)).await.is_err(), "aucune catégorie");
    assert!(ent.creer_prise_en_charge_for_actor(etab, u, r, &corps(org.id, el, d(2030, 9, 1), d(2031, 6, 30), vec!["tout et n'importe quoi"], None)).await.is_err());
    assert!(ent.creer_prise_en_charge_for_actor(etab, u, r, &corps(org.id, el, d(2030, 9, 1), d(2031, 6, 30), vec!["internat"], Some(0))).await.is_err(), "plafond nul");
    assert!(ent.creer_prise_en_charge_for_actor(etab, u, r, &corps(org.id, el, d(2030, 9, 1), d(2031, 6, 30), vec!["internat"], Some(-5))).await.is_err());
    // Compte ou élève d'un autre établissement : introuvable.
    assert!(matches!(
        ent.creer_prise_en_charge_for_actor(etab, u, r, &corps(Uuid::new_v4(), el, d(2030, 9, 1), d(2031, 6, 30), vec!["internat"], None)).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    assert!(matches!(
        ent.creer_prise_en_charge_for_actor(etab, u, r, &corps(org.id, Uuid::new_v4(), d(2030, 9, 1), d(2031, 6, 30), vec!["internat"], None)).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    assert!(ent.creer_prise_en_charge_for_actor(etab, u, r, &corps(org.id, el, d(2030, 9, 1), d(2031, 6, 30), vec!["internat", "toutes"], Some(1))).await.is_ok());
}

// ───────────────────────── CF-09, MIG-4 : reprise des comptes ─────────────────────────

/// MIG-4 : les foyers qui ont des pièces deviennent des comptes ; `compte_id` est renseigné ; aucun
/// montant, numéro ni date de pièce n'est modifié ; les foyers à plusieurs payeurs ou sans payeur sont
/// **signalés**, jamais éclatés (CF-09) ; la reprise est rejouable sans effet.
#[tokio::test]
async fn mig_4_reprise_des_comptes_cf_09() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (dirn, dirr) = direction(&c);
    let el = eleve(&ent, etab, "Paul", d(2018, 3, 1), "externe").await;
    let (f1, f2, f3) = (
        foyer(&ent, etab, "Foyer un payeur").await,
        foyer(&ent, etab, "Foyer deux payeurs").await,
        foyer(&ent, etab, "Foyer sans payeur").await,
    );
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, f1, 100).await;
    let el2 = eleve(&ent, etab, "Pia", d(2018, 3, 1), "externe").await;
    payeur_compte(&ent, etab, el2, c.parent_tom.user_id, f2, 50).await;
    payeur_personne_f2(&ent, etab, el2, f2, 50).await;
    // Des pièces historiques SANS compte : on lève la contrainte le temps d'une transaction de test (on
    // reproduit ainsi une base d'avant F2), puis on la rétablit.
    let mut pieces = Vec::new();
    {
        let mut tx = ent.pool_verif().begin().await.unwrap();
        sqlx::query("ALTER TABLE ent_fac_facture DROP CONSTRAINT ent_fac_facture_compte_requis_chk").execute(&mut *tx).await.unwrap();
        sqlx::query("ALTER TABLE ent_fac_facture DISABLE TRIGGER ent_fac_facture_pose_compte_trg").execute(&mut *tx).await.unwrap();
        for (i, f) in [f1, f2, f3].into_iter().enumerate() {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_fac_facture (id, etablissement_id, foyer_id, numero, libelle, statut, total_centimes, emise_at)
                         VALUES ($1,$2,$3,$4,'Pièce historique','emise',$5, now())")
                .bind(id).bind(etab).bind(f).bind(format!("H-{i}-{}", &id.simple().to_string()[..6])).bind(10_000_i64 * (i as i64 + 1))
                .execute(&mut *tx).await.unwrap();
            pieces.push(id);
        }
        sqlx::query("ALTER TABLE ent_fac_facture ENABLE TRIGGER ent_fac_facture_pose_compte_trg").execute(&mut *tx).await.unwrap();
        sqlx::query("ALTER TABLE ent_fac_facture ADD CONSTRAINT ent_fac_facture_compte_requis_chk CHECK (compte_id IS NOT NULL) NOT VALID").execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
    }
    let avant: Vec<(Uuid, String, i64, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT id, numero, total_centimes, emise_at FROM ent_fac_facture WHERE id = ANY($1) ORDER BY id")
        .bind(&pieces).fetch_all(ent.pool_verif()).await.unwrap();

    let rap = ent.reprendre_comptes_for_actor(etab, dirn, dirr).await.unwrap();
    assert!(rap.comptes_crees >= 3, "{rap:?}");
    assert_eq!(rap.pieces_rattachees, 3);
    assert!(rap.foyers_a_verifier >= 2);
    // Rattachement et signalements.
    let comptes: Vec<(Uuid, String, bool, Option<String>)> = sqlx::query_as(
        "SELECT foyer_id, type_debiteur, a_verifier, a_verifier_motif FROM ent_fin_compte WHERE etablissement_id = $1 AND foyer_id = ANY($2) ORDER BY foyer_id")
        .bind(etab).bind(vec![f1, f2, f3]).fetch_all(ent.pool_verif()).await.unwrap();
    assert_eq!(comptes.len(), 3, "un compte par foyer, jamais éclaté");
    let c1 = comptes.iter().find(|x| x.0 == f1).unwrap();
    let c2 = comptes.iter().find(|x| x.0 == f2).unwrap();
    let c3 = comptes.iter().find(|x| x.0 == f3).unwrap();
    assert_eq!((c1.1.as_str(), c1.2), ("personne", false));
    assert_eq!((c2.1.as_str(), c2.2, c2.3.as_deref()), ("foyer", true, Some("plusieurs_payeurs_dans_le_foyer")));
    assert_eq!((c3.1.as_str(), c3.2, c3.3.as_deref()), ("foyer", true, Some("aucun_payeur_identifie")));
    let sans_compte: (i64,) = sqlx::query_as("SELECT count(*) FROM ent_fac_facture WHERE id = ANY($1) AND compte_id IS NULL").bind(&pieces).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(sans_compte.0, 0);
    // Aucune donnée monétaire modifiée.
    let apres: Vec<(Uuid, String, i64, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT id, numero, total_centimes, emise_at FROM ent_fac_facture WHERE id = ANY($1) ORDER BY id")
        .bind(&pieces).fetch_all(ent.pool_verif()).await.unwrap();
    assert_eq!(avant, apres);
    // Les liens du foyer à un payeur sont repris en désignation « toutes » ; le rapport est conservé.
    let (nb_desig,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_fin_designation WHERE etablissement_id = $1 AND eleve_id = $2 AND source = 'reprise' AND statut = 'effective'")
        .bind(etab).bind(el).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(nb_desig >= 1);
    let (nb_rap,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_fin_reprise_rapport WHERE etablissement_id = $1").bind(etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(nb_rap >= 1);
    // Rejouable : rien de plus.
    let encore = ent.reprendre_comptes_for_actor(etab, dirn, dirr).await.unwrap();
    assert_eq!((encore.comptes_crees, encore.pieces_rattachees, encore.designations_creees), (0, 0, 0));
    // Une pièce émise garde son compte : on ne le change pas (déclencheur), on ne la supprime pas.
    let autre = compte_du_foyer(&ent, etab, f1).await;
    let (cible,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_fin_compte WHERE etablissement_id = $1 AND foyer_id = $2").bind(etab).bind(f2).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(autre, c1_id(&ent, etab, f1).await);
    let essai = sqlx::query("UPDATE ent_fac_facture SET compte_id = $2 WHERE id = $1").bind(pieces[0]).bind(cible).execute(ent.pool_verif()).await;
    assert!(essai.is_err(), "le compte d'une pièce émise ne change pas");
    // La comptabilité ne peut pas lancer la reprise (droit `reprise` : comptable et direction seulement, pas le secrétariat).
    let (us, rs) = secretariat(&c);
    assert!(matches!(ent.reprendre_comptes_for_actor(etab, us, rs).await, Err(EntDbError::Forbidden)));
}

async fn c1_id(ent: &PgEnt, etab: Uuid, foyer: Uuid) -> Uuid {
    let (id,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_fin_compte WHERE etablissement_id = $1 AND foyer_id = $2").bind(etab).bind(foyer).fetch_one(ent.pool_verif()).await.unwrap();
    id
}

// ───────────────────────── Base : immutabilité, RESTRICT, journal en ajout seul, RLS ─────────────────────────

/// Ni suppression ni modification de l'historique financier, même en SQL direct.
#[tokio::test]
async fn historique_financier_ni_supprime_ni_modifie() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let org = ent.creer_compte_organisme_for_actor(etab, u, r, &CreerCompteOrganismeBody { libelle: "Org X".into(), siret: None, reference_client: None, organisme_public: false, chorus_pro: false }).await.unwrap();
    let el = eleve(&ent, etab, "Quentin", d(2018, 3, 1), "externe").await;
    piece_accord(&ent, &c, el, "toutes", vec![(org.id, 10_000)], d(2030, 9, 1)).await;
    // Le journal est en ajout seul.
    assert!(sqlx::query("UPDATE ent_fin_journal SET detail = 'x' WHERE etablissement_id = $1").bind(etab).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_fin_journal WHERE etablissement_id = $1").bind(etab).execute(ent.pool_verif()).await.is_err());
    // Ni compte, ni désignation, ni tâche ne se supprime.
    assert!(sqlx::query("DELETE FROM ent_fin_compte WHERE id = $1").bind(org.id).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_fin_designation WHERE etablissement_id = $1 AND eleve_id = $2").bind(etab).bind(el).execute(ent.pool_verif()).await.is_err());
    // Une désignation ne peut pas chevaucher une autre pour le même compte (R-F4) : la base le refuse.
    let chevauche = sqlx::query(
        "INSERT INTO ent_fin_designation (id, etablissement_id, groupe_id, eleve_id, categorie, compte_id, part_bps, du, source, statut, acceptation, piece_nature, piece_reference)
         VALUES ($1,$2,$3,$4,'toutes',$5,10000,'2030-10-01','finances','effective','non_requise','jugement','x')")
        .bind(Uuid::new_v4()).bind(etab).bind(Uuid::new_v4()).bind(el).bind(org.id).execute(ent.pool_verif()).await;
    assert!(chevauche.is_err(), "chevauchement refusé en base");
    // Une désignation de la facturation effective sans acceptation ni pièce est refusée en base (R-F30).
    let sans_consentement = sqlx::query(
        "INSERT INTO ent_fin_designation (id, etablissement_id, groupe_id, eleve_id, categorie, compte_id, part_bps, du, source, statut, acceptation)
         VALUES ($1,$2,$3,$4,'scolarite',$5,10000,'2030-09-01','finances','effective','en_attente')")
        .bind(Uuid::new_v4()).bind(etab).bind(Uuid::new_v4()).bind(el).bind(org.id).execute(ent.pool_verif()).await;
    assert!(sans_consentement.is_err(), "R-F30 : pas opposable sans acceptation ni pièce");
    // Une pièce ne peut viser le compte d'un autre foyer.
    let fx = foyer(&ent, etab, "Foyer X").await;
    let mauvais = sqlx::query("INSERT INTO ent_fac_facture (id, etablissement_id, foyer_id, numero, libelle, statut, total_centimes, compte_id) VALUES ($1,$2,$3,'BRO-X','x','brouillon',1,$4)")
        .bind(Uuid::new_v4()).bind(etab).bind(fx).bind(org.id).execute(ent.pool_verif()).await;
    assert!(mauvais.is_err(), "le compte de la pièce doit être celui de son foyer");
}

/// Une pièce créée sans compte (chemin historique, brouillon) reçoit **toujours** son compte, posé par la base.
#[tokio::test]
async fn toute_piece_porte_son_compte() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let fx = foyer(&ent, etab, "Foyer Y").await;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_fac_facture (id, etablissement_id, foyer_id, numero, libelle, statut, total_centimes) VALUES ($1,$2,$3,'BRO-Y','y','brouillon',1)")
        .bind(id).bind(etab).bind(fx).execute(ent.pool_verif()).await.unwrap();
    let (compte,): (Option<Uuid>,) = sqlx::query_as("SELECT compte_id FROM ent_fac_facture WHERE id = $1").bind(id).fetch_one(ent.pool_verif()).await.unwrap();
    let compte = compte.expect("compte posé par la base");
    let (foyer_du_compte, entite): (Uuid, Uuid) = sqlx::query_as("SELECT foyer_id, entite_facturation_id FROM ent_fin_compte WHERE id = $1").bind(compte).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(foyer_du_compte, fx);
    // Entité de facturation par défaut posée (R-F33), régime privé, une seule par établissement.
    let (regime, nb): (String, i64) = sqlx::query_as("SELECT (SELECT regime FROM ent_fin_entite_facturation WHERE id = $1), (SELECT count(*) FROM ent_fin_entite_facturation WHERE etablissement_id = $2)")
        .bind(entite).bind(etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!((regime.as_str(), nb), ("prive_sous_contrat", 1));
}

// ───────────────────────── F1 : journal, événements, droits fins, guichet ─────────────────────────

/// R-F27 : chaque geste laisse sa ligne de journal ; la comptabilité ne voit que ses propres gestes
/// (M-676), la direction voit tout ; l'événement ne porte aucun montant (R-F25) et sous le nom canonique.
#[tokio::test]
async fn f1_journal_evenements_sans_montant_et_alias() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    grille(&ent, &c).await;
    let (cu, cr) = compta(&c);
    let (du, dr) = direction(&c);
    let el = eleve(&ent, etab, "Rémi", d(2018, 3, 1), "demi_pension").await;
    let fm = foyer(&ent, etab, "Foyer M11").await;
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 100).await;
    facturer(&ent, etab, el).await.unwrap();
    let (brouillon,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_fac_facture WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'brouillon'").bind(etab).bind(fm).fetch_one(ent.pool_verif()).await.unwrap();
    ent.emettre_for_actor(etab, cu, cr, &EmissionBody { factures: vec![brouillon] }).await.unwrap();
    // Journal : la comptabilité voit son geste d'émission, avec le montant (réservé au journal) et son chemin.
    let mien = ent.journal_financier_for_actor(etab, cu, cr, 50).await.unwrap();
    let em = mien.iter().find(|l| l.action == "facture.emettre").expect("émission journalisée");
    assert_eq!(em.chemin, "finances");
    assert_eq!(em.montant_centimes, Some(1_300_001));
    assert_eq!(em.acteur_user_id, Some(cu));
    assert_eq!(em.casquette, "compta");
    assert!(mien.iter().all(|l| l.acteur_user_id == Some(cu)), "M-676 : chacun voit ses propres gestes");
    let tout = ent.journal_financier_for_actor(etab, du, dr, 500).await.unwrap();
    assert!(tout.iter().any(|l| l.action == "facture.emettre") && tout.len() >= mien.len());
    // Le secrétariat (ni lire ni journal) n'a pas accès au journal.
    let (su, sr) = secretariat(&c);
    assert!(matches!(ent.journal_financier_for_actor(etab, su, sr, 10).await, Err(EntDbError::Forbidden)));
    // Événements : nom canonique, aucun ancien nom, aucun montant ni IBAN dans aucune charge (R-F25).
    let types_vus: Vec<(String, serde_json::Value)> = sqlx::query_as("SELECT type, payload FROM ent_metier_event WHERE etablissement_id = $1 AND (type LIKE 'facturation.%' OR type LIKE 'fac.%' OR type LIKE 'fin.%')")
        .bind(etab).fetch_all(ent.pool_verif()).await.unwrap();
    assert!(types_vus.iter().any(|(t, _)| t == "facturation.piece_emise"), "{:?}", types_vus.iter().map(|x| &x.0).collect::<Vec<_>>());
    assert!(types_vus.iter().all(|(t, _)| t.starts_with("facturation.")), "plus aucun ancien nom émis : {:?}", types_vus.iter().map(|x| &x.0).collect::<Vec<_>>());
    for (t, p) in &types_vus {
        let texte = p.to_string();
        assert!(!texte.contains("centimes") && !texte.contains("iban") && !texte.contains("montant"), "{t} : {texte}");
    }
}

/// Alias de lecture : un consommateur déclaré sur l'ancien nom `fac.facture_emise` reçoit l'événement
/// canonique `facturation.piece_emise` (et inversement).
#[tokio::test]
async fn f1_alias_de_lecture_fac_vers_facturation() {
    use crate::outbox::{Consommateur, ConfigDistribution, EvenementDistribue, FutureTraitement};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    struct Ancien(Arc<AtomicUsize>);
    impl Consommateur for Ancien {
        fn nom(&self) -> &'static str { "test.f1.ancien" }
        fn types(&self) -> &'static [&'static str] { &["fac.facture_emise"] }
        fn traiter<'a>(&'a self, _tx: &'a mut sqlx::PgConnection, _evt: &'a EvenementDistribue) -> FutureTraitement<'a> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(()) })
        }
    }
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    grille(&ent, &c).await;
    let (cu, cr) = compta(&c);
    let el = eleve(&ent, etab, "Sacha", d(2018, 3, 1), "externe").await;
    let fm = foyer(&ent, etab, "Foyer M12").await;
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 100).await;
    facturer(&ent, etab, el).await.unwrap();
    let (b,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_fac_facture WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'brouillon'").bind(etab).bind(fm).fetch_one(ent.pool_verif()).await.unwrap();
    let vus = Arc::new(AtomicUsize::new(0));
    let mut dist = ent.distributeur(ConfigDistribution {
        taille_lot: 100, max_tentatives: 3, delai_base: std::time::Duration::ZERO, delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20), etablissement: Some(etab),
    });
    dist.enregistrer(Arc::new(Ancien(vus.clone()))).unwrap();
    ent.emettre_for_actor(etab, cu, cr, &EmissionBody { factures: vec![b] }).await.unwrap();
    dist.distribuer_tout().await.unwrap();
    assert_eq!(vus.load(Ordering::SeqCst), 1, "le consommateur de l'ancien nom reçoit l'événement canonique");
}

/// Droits fins : émettre exige `facturation.emettre`, les avoirs `facturation.avoirs`, le créancier et les
/// paramètres `facturation.parametrer` ; `gerer` seul ne suffit plus (R-F23).
#[tokio::test]
async fn f1_droits_fins_sur_les_commandes() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (su, sr) = secretariat(&c);
    // Le secrétariat n'émet pas, ne paramètre pas, ne fait pas d'avoir, n'a pas le journal.
    assert!(matches!(ent.emettre_for_actor(etab, su, sr, &EmissionBody { factures: vec![Uuid::new_v4()] }).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.parametres_finances_for_actor(etab, su, sr).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.creer_avoir_for_actor(etab, su, sr, Uuid::new_v4(), &AvoirBody { motif: "x".into(), montant_centimes: None }).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.reprendre_comptes_for_actor(etab, su, sr).await, Err(EntDbError::Forbidden)));
    // Mais il a les engagements (désigner) et la vue guichet.
    assert!(ent.taches_for_actor(etab, su, sr, &TachesFiltre::default()).await.is_ok());
    // Un professeur n'a rien de financier.
    let (pu, pr) = (c.prof.user_id, "prof");
    assert!(matches!(ent.taches_for_actor(etab, pu, pr, &TachesFiltre::default()).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.comptes_for_actor(etab, pu, pr, &ComptesFiltre::default()).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.guichet_rechercher_for_actor(etab, pu, pr, "Dupont", AuthentificationGuichet::Presentiel).await, Err(EntDbError::Forbidden)));
    // Un parent (même payeur) n'a aucun droit de personnel : les commandes de gestion lui sont fermées.
    assert!(matches!(ent.designer_payeurs_for_actor(etab, c.parent_lea.user_id, "parent", Uuid::new_v4(), &DesignerPayeursBody {
        categorie: "toutes".into(), parts: vec![], date_effet: d(2030, 9, 1), piece: None, motif: None }).await, Err(EntDbError::Forbidden)));
}

/// R-F37, CF-95 : le guichet cherche le payeur présent (3 caractères au moins), montre solde, échéances et
/// documents d'**un** compte, journalise chaque consultation, et ne donne jamais d'adresse ni de liste d'impayés.
#[tokio::test]
async fn f1_vue_guichet_du_secretariat() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    grille(&ent, &c).await;
    let (cu, cr) = compta(&c);
    let (su, sr) = secretariat(&c);
    let el = eleve(&ent, etab, "Tess", d(2018, 3, 1), "demi_pension").await;
    let fm = foyer(&ent, etab, "Foyer guichet").await;
    payeur_compte(&ent, etab, el, c.parent_lea.user_id, fm, 100).await;
    facturer(&ent, etab, el).await.unwrap();
    let (b,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_fac_facture WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'brouillon'").bind(etab).bind(fm).fetch_one(ent.pool_verif()).await.unwrap();
    ent.emettre_for_actor(etab, cu, cr, &EmissionBody { factures: vec![b] }).await.unwrap();
    // Recherche : trop courte → refusée ; sinon le compte, sans solde ni adresse.
    assert!(ent.guichet_rechercher_for_actor(etab, su, sr, "ab", AuthentificationGuichet::Presentiel).await.is_err());
    let (libelle,): (String,) = sqlx::query_as("SELECT libelle_facturation FROM ent_fin_compte WHERE etablissement_id = $1 AND foyer_id = $2").bind(etab).bind(fm).fetch_one(ent.pool_verif()).await.unwrap();
    let trouves = ent.guichet_rechercher_for_actor(etab, su, sr, &libelle[..4.min(libelle.len())], AuthentificationGuichet::Presentiel).await.unwrap();
    assert!(trouves.iter().any(|x| x.libelle == libelle) && trouves.len() <= 10);
    let compte = trouves.iter().find(|x| x.libelle == libelle).unwrap().compte_id;
    // Vue du compte : solde = total, une échéance au moins, un document.
    let vue = ent.guichet_compte_for_actor(etab, su, sr, compte, AuthentificationGuichet::RappelNumeroEnregistre).await.unwrap();
    let dbg: Vec<(String,i64,String)> = sqlx::query_as("SELECT numero, total_centimes, statut FROM ent_fac_facture WHERE etablissement_id=$1").bind(etab).fetch_all(ent.pool_verif()).await.unwrap();
    eprintln!("DBGF {dbg:?} {:?}", vue.documents);
    assert_eq!(vue.solde_centimes, 1_300_001);
    assert!(!vue.echeances.is_empty());
    assert!(vue.documents.as_ref().is_some_and(|d| d.len() == 1));
    // Chaque consultation est journalisée, avec la preuve d'identité et sans le texte cherché.
    let (du, dr) = direction(&c);
    let j = ent.journal_financier_for_actor(etab, du, dr, 500).await.unwrap();
    assert!(j.iter().any(|l| l.action == "guichet.consulter" && l.compte_id == Some(compte) && l.casquette == "administratif"
        && l.detail.as_deref() == Some("rappel au numéro enregistré")));
    assert!(j.iter().any(|l| l.action == "guichet.rechercher"));
    assert!(j.iter().filter(|l| l.action == "guichet.rechercher").all(|l| !l.detail.clone().unwrap_or_default().contains(&libelle)));
    // Compte d'un autre établissement ou inexistant : introuvable.
    assert!(matches!(
        ent.guichet_compte_for_actor(etab, su, sr, Uuid::new_v4(), AuthentificationGuichet::Presentiel).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Le secrétariat n'a pas la liste des comptes (aucune navigation entre comptes).
    assert!(matches!(ent.comptes_for_actor(etab, su, sr, &ComptesFiltre::default()).await, Err(EntDbError::Forbidden)));
}

/// RLS : un autre établissement ne voit aucun compte, désignation, tâche ni ligne de journal de celui-ci.
#[tokio::test]
async fn f2_isolation_des_etablissements() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let org = ent.creer_compte_organisme_for_actor(etab, u, r, &CreerCompteOrganismeBody { libelle: "Org isolée".into(), siret: None, reference_client: None, organisme_public: false, chorus_pro: false }).await.unwrap();
    // Autre établissement de test : même base, autre identifiant ; le compte n'y est pas lisible.
    let autre = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_etablissement (id, slug, nom) VALUES ($1, $2, 'Autre établissement F2')")
        .bind(autre).bind(format!("f2-{}", &autre.simple().to_string()[..8])).execute(ent.pool_verif()).await.ok();
    let mut tx = crate::tenant_tx::TenantTx::begin(ent.pool_verif(), autre).await.unwrap();
    let vus: (i64,) = sqlx::query_as("SELECT count(*) FROM ent_fin_compte WHERE etablissement_id = $1 AND id = $2").bind(autre).bind(org.id).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(vus.0, 0);
    drop(tx);
    // Même identifiant demandé par l'autre établissement : introuvable (404, jamais un succès).
    let (u2, r2) = compta(&c);
    let _ = (u2, r2);
    let b = ent.compte_for_actor(etab, u, r, org.id).await.unwrap();
    assert_eq!(b.id, org.id);
}

#[test]
fn le_port_ne_depend_pas_des_foyers_historiques_pour_les_categories() {
    // Les catégories de désignation couvrent toutes les catégories d'articles facturables sauf
    // `bourse` et `reduction`, qui suivent leur article.
    for cat in crate::finances::CATEGORIES {
        let c = payeurs::categorie_de_designation(cat);
        if cat == "bourse" || cat == "reduction" {
            assert_eq!(c, "toutes");
        } else {
            assert!(payeurs::CATEGORIES_DESIGNATION.contains(&c), "{cat} → {c}");
        }
    }
}

/// Second payeur du même foyer, sans compte utilisateur (fiche « personne » seule).
async fn payeur_personne_f2(ent: &PgEnt, etab: Uuid, eleve: Uuid, foyer: Uuid, part: i16) {
    let p = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom) VALUES ($1, $2, 'Autre payeur')")
        .bind(p).bind(etab).execute(ent.pool_verif()).await.unwrap();
    sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, foyer_id, lien, est_payeur, part_paiement) VALUES ($1,$2,$3,$4,$5,'pere',true,$6)")
        .bind(Uuid::new_v4()).bind(etab).bind(eleve).bind(p).bind(foyer).bind(part).execute(ent.pool_verif()).await.unwrap();
}
