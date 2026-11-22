//! Socle 24, lot F3 : catalogue d'articles qualifiés, grilles tarifaires versionnées et votées, tranches de
//! quotient, simulateur de scénarios, familles types, état des contributions — contre la base locale de test
//! (feature `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés (exigés en CI).
//!
//! Chaque scénario crée un établissement neuf (`seed_labo_cast`) ; année facturée 2030-2031 (lointaine :
//! aucune dépendance à l'horloge réelle, toutes les dates sont fixes).

use super::*;
use crate::service::RefusMetier;
use chrono::NaiveDate;
use ent_auth::Role;
use ent_types::finances::*;
use ent_types::finances_grilles::*;
use uuid::Uuid;

const ANNEE: &str = "2030-2031";

/// Jour de référence des tests (jamais l'horloge réelle, règle 33) : 1er juillet 2030, après le vote du 20 juin.
fn jour_test() -> NaiveDate {
    NaiveDate::from_ymd_opt(2030, 7, 1).unwrap()
}

/// Vote au jour de référence fixe (le service lit sinon le jour de l'établissement).
trait VoterAuJourFixe {
    async fn voter_grille_test(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        b: &VoteGrilleBody,
    ) -> Result<GrilleVersionDto, EntDbError>;
}
impl VoterAuJourFixe for PgEnt {
    async fn voter_grille_test(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        b: &VoteGrilleBody,
    ) -> Result<GrilleVersionDto, EntDbError> {
        self.voter_grille_au_jour(etab, actor, role, version, b, jour_test())
            .await
    }
}

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_mail_dry_run();
    })
    .await;
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

fn conflit<T>(r: Result<T, EntDbError>) -> bool {
    matches!(
        r,
        Err(EntDbError::Refus(RefusMetier::Conflit, _)) | Err(EntDbError::ConflitDossier(_))
    )
}
fn interdit<T>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Refus(RefusMetier::Interdit, _)))
}
fn invalide<T>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::SaisieInvalide(_)))
}
fn introuvable<T>(r: Result<T, EntDbError>) -> bool {
    matches!(
        r,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    )
}
fn refuse<T>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

async fn eleve(ent: &PgEnt, etab: Uuid, prenom: &str, regime: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, regime, date_naissance)
         VALUES ($1, $2, 'Testf3', $3, 'actif', $4, '2018-03-01')",
    )
    .bind(id)
    .bind(etab)
    .bind(prenom)
    .bind(regime)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

async fn foyer_payeur(ent: &PgEnt, etab: Uuid, eleve: Uuid, user: Uuid, libelle: &str) -> Uuid {
    let f = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_foyer (id, etablissement_id, libelle) VALUES ($1, $2, $3)")
        .bind(f)
        .bind(etab)
        .bind(libelle)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, foyer_id, lien, est_payeur, part_paiement)
         VALUES ($1, $2, $3, $4, $5, 'mere', true, 100)",
    )
    .bind(Uuid::new_v4()).bind(etab).bind(eleve).bind(user).bind(f)
    .execute(ent.pool_verif()).await.unwrap();
    f
}

/// Lance le moteur de génération pour un élève (même chemin que FIN-02 et que l'inscription validée).
async fn facturer(ent: &PgEnt, etab: Uuid, eleve: Uuid) {
    facturer_au(ent, etab, eleve, d(2030, 10, 1)).await;
}

/// Même génération à un jour fixé : seule la grille applicable ce jour-là tarife (règle 2).
async fn facturer_au(ent: &PgEnt, etab: Uuid, eleve: Uuid, jour: NaiveDate) {
    let mut tx = ent.pool_verif().begin().await.unwrap();
    crate::finances::facturer_annee_eleve_au(
        &mut tx,
        etab,
        ANNEE,
        eleve,
        None,
        None,
        Default::default(),
        jour,
    )
    .await
    .unwrap()
    .expect("facturation de l'élève");
    tx.commit().await.unwrap();
}

async fn article_modele(ent: &PgEnt, c: &LaboCast, modele: &str, code: &str) -> ArticleQualifieDto {
    let (u, r) = compta(c);
    ent.article_depuis_modele_for_actor(
        c.etablissement_id,
        u,
        r,
        &ArticleDepuisModeleBody {
            modele_code: modele.into(),
            code: Some(code.into()),
            libelle: None,
            compte_produit: None,
        },
    )
    .await
    .unwrap()
}

fn tarif_body(article: Uuid, montant: i64, defaut: bool) -> TarifGrilleBody {
    TarifGrilleBody {
        article_id: article,
        niveau: None,
        regime: None,
        qf_min_centimes: None,
        qf_max_centimes: None,
        montant_annuel_centimes: montant,
        tarif_par_defaut: Some(defaut),
    }
}

/// Grille de l'année avec deux articles qualifiés (contribution 1 200 €, demi-pension 450 €), proposée par la
/// comptabilité. Renvoie (version, article contribution, article demi-pension).
async fn grille_proposee(ent: &PgEnt, c: &LaboCast) -> (Uuid, Uuid, Uuid) {
    let (u, r) = compta(c);
    let etab = c.etablissement_id;
    let contrib = article_modele(ent, c, "CONTRIBUTION", "CONTRIB").await;
    let dp = article_modele(ent, c, "DEMI_PENSION", "DP").await;
    let v = ent
        .creer_grille_for_actor(
            etab,
            u,
            r,
            &CreerGrilleBody {
                annee_scolaire: ANNEE.into(),
                libelle: None,
                copier_de: None,
                reprendre_historique: None,
            },
        )
        .await
        .unwrap();
    ent.ajouter_tarif_grille_for_actor(etab, u, r, v.id, &tarif_body(contrib.id, 120_000, true))
        .await
        .unwrap();
    ent.ajouter_tarif_grille_for_actor(etab, u, r, v.id, &tarif_body(dp.id, 45_000, true))
        .await
        .unwrap();
    ent.proposer_grille_for_actor(etab, u, r, v.id)
        .await
        .unwrap();
    (v.id, contrib.id, dp.id)
}

/// Établissement **antérieur à la 523** : qualification non exigée (valeur transitoire, M7). Les scénarios qui
/// créent des articles « à l'ancienne » ou émettent sans catalogue qualifié se placent dans cet état ; le défaut
/// d'un établissement créé après la 523 (exigée) est éprouvé par `f3r_m7_*`.
async fn etablissement_anterieur(ent: &PgEnt, etab: Uuid) {
    sqlx::query(
        "INSERT INTO ent_fin_parametres (etablissement_id, exiger_qualification_articles) VALUES ($1, false)
         ON CONFLICT (etablissement_id) DO UPDATE SET exiger_qualification_articles = false",
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

fn vote(reference: &str) -> VoteGrilleBody {
    VoteGrilleBody {
        vote_le: d(2030, 6, 20),
        reference: reference.into(),
        document_id: None,
    }
}

async fn journal_contient(ent: &PgEnt, etab: Uuid, action: &str) -> i64 {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_fin_journal WHERE etablissement_id = $1 AND action = $2",
    )
    .bind(etab)
    .bind(action)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    n
}

// ───────────────────────── Catalogue (FAC-103 à FAC-106, R-F9, MIG-6) ─────────────────────────

/// Un modèle crée un article **déjà qualifié** (nature, TVA, unité, mode) ; un article existant reste non
/// qualifié avec une **proposition** ; la qualification est validée par la comptabilité ; nature et TVA se
/// figent dès qu'une ligne existe ; les rôles sans droit n'accèdent à rien.
#[tokio::test]
async fn f3_catalogue_modele_qualification_et_nature_figee() {
    let Some((ent, c)) = labo().await else { return };
    etablissement_anterieur(&ent, c.etablissement_id).await;
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (us, rs) = secretariat(&c);
    assert!(
        refuse(ent.modeles_articles_for_actor(etab, us, rs).await),
        "le secrétariat n'a pas le catalogue"
    );
    assert!(
        ent.modeles_articles_for_actor(etab, u, r)
            .await
            .unwrap()
            .len()
            >= 15
    );

    let repas = article_modele(&ent, &c, "DEMI_PENSION_REPAS", "REPAS").await;
    assert!(repas.qualifie && repas.proposition.is_none());
    assert_eq!(repas.nature_legale.as_deref(), Some("demi_pension"));
    assert_eq!(
        (repas.unite.as_deref(), repas.mode.as_str()),
        (Some("repas"), "reel")
    );
    // même code : conflit ; modèle inconnu : refus
    assert!(conflit(
        ent.article_depuis_modele_for_actor(
            etab,
            u,
            r,
            &ArticleDepuisModeleBody {
                modele_code: "DEMI_PENSION_REPAS".into(),
                code: Some("REPAS".into()),
                libelle: None,
                compte_produit: None
            }
        )
        .await
    ));
    assert!(invalide(
        ent.article_depuis_modele_for_actor(
            etab,
            u,
            r,
            &ArticleDepuisModeleBody {
                modele_code: "INCONNU".into(),
                code: None,
                libelle: None,
                compte_produit: None
            }
        )
        .await
    ));
    // le modèle « pour compte de tiers » (APEL) porte son régime
    let apel = article_modele(&ent, &c, "COTISATION_APEL", "APEL").await;
    assert_eq!(apel.regime_tva.as_deref(), Some("pour_compte_tiers"));

    // Article existant (chemin historique) : non qualifié, proposition par règle, jamais appliquée seule.
    let ancien = ent
        .enregistrer_article_for_actor(
            etab,
            u,
            r,
            None,
            &ArticleBody {
                code: "ANCIEN".into(),
                libelle: "Ancien".into(),
                categorie: "internat".into(),
                compte_produit: None,
                actif: None,
            },
        )
        .await
        .unwrap();
    let cat = ent.articles_qualifies_for_actor(etab, u, r).await.unwrap();
    let a = cat.iter().find(|x| x.id == ancien.id).unwrap();
    assert!(!a.qualifie);
    assert_eq!(a.proposition.as_ref().unwrap().nature_legale, "internat");
    // Qualification invalide : taxable sans taux ; exonéré avec taux ; nature inconnue.
    let q = |nature: &str, regime: &str, taux: Option<i32>| QualifierArticleBody {
        nature_legale: nature.into(),
        regime_tva: regime.into(),
        taux_tva_bps: taux,
        unite: "annee".into(),
        mode: None,
        remboursable: None,
        soumis_reductions: None,
        soumis_remise_ordre: None,
    };
    assert!(invalide(
        ent.qualifier_article_for_actor(etab, u, r, ancien.id, &q("internat", "taxable", None))
            .await
    ));
    assert!(invalide(
        ent.qualifier_article_for_actor(
            etab,
            u,
            r,
            ancien.id,
            &q("internat", "exonere_261_4_4", Some(550))
        )
        .await
    ));
    assert!(invalide(
        ent.qualifier_article_for_actor(
            etab,
            u,
            r,
            ancien.id,
            &q("n_importe_quoi", "exonere_261_4_4", None)
        )
        .await
    ));
    assert!(refuse(
        ent.qualifier_article_for_actor(
            etab,
            us,
            rs,
            ancien.id,
            &q("internat", "exonere_261_4_4", None)
        )
        .await
    ));
    let ok = ent
        .qualifier_article_for_actor(
            etab,
            u,
            r,
            ancien.id,
            &q("internat", "exonere_261_4_4", None),
        )
        .await
        .unwrap();
    assert!(ok.qualifie && ok.proposition.is_none());
    // Une ligne de pièce porte désormais l'article : nature et TVA ne bougent plus (FAC-104).
    let el = eleve(&ent, etab, "Noé", "interne").await;
    let _ = foyer_payeur(&ent, etab, el, c.parent_lea.user_id, "Foyer F3-1").await;
    ent.creer_tarif_for_actor(
        etab,
        u,
        r,
        &TarifBody {
            annee_scolaire: ANNEE.into(),
            article_id: ancien.id,
            niveau: None,
            regime: None,
            qf_min_centimes: None,
            qf_max_centimes: None,
            montant_annuel_centimes: 500_000,
        },
    )
    .await
    .unwrap();
    facturer(&ent, etab, el).await;
    let usage = ent
        .usage_article_for_actor(etab, u, r, ancien.id)
        .await
        .unwrap();
    assert_eq!(
        usage.nb_lignes, 0,
        "brouillons non comptés : seules les pièces émises"
    );
    assert!(conflit(
        ent.qualifier_article_for_actor(
            etab,
            u,
            r,
            ancien.id,
            &q("autre_motivee", "hors_champ", None)
        )
        .await
    ));
    let direct =
        sqlx::query("UPDATE ent_fin_article SET nature_legale = 'autre_motivee' WHERE id = $1")
            .bind(ancien.id)
            .execute(ent.pool_verif())
            .await;
    assert!(
        direct.is_err(),
        "déclencheur : nature figée dès qu'une ligne existe"
    );
    let retrait = sqlx::query("UPDATE ent_fin_article SET qualifie_le = NULL WHERE id = $1")
        .bind(repas.id)
        .execute(ent.pool_verif())
        .await;
    assert!(retrait.is_err(), "la qualification ne se retire pas");
    assert_eq!(journal_contient(&ent, etab, "article.qualifier").await, 1);
    assert_eq!(
        journal_contient(&ent, etab, "article.creer_depuis_modele").await,
        2
    );
    assert!(introuvable(
        ent.usage_article_for_actor(etab, u, r, Uuid::new_v4())
            .await
    ));
}

/// MIG-6 : quand l'établissement exige la qualification, une pièce dont un article n'est pas qualifié ne
/// s'émet pas ; elle s'émet une fois l'article qualifié ; la création d'un article « à l'ancienne » est refusée.
#[tokio::test]
async fn f3_emission_exige_un_article_qualifie() {
    let Some((ent, c)) = labo().await else { return };
    etablissement_anterieur(&ent, c.etablissement_id).await;
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let art = ent
        .enregistrer_article_for_actor(
            etab,
            u,
            r,
            None,
            &ArticleBody {
                code: "SCOL".into(),
                libelle: "Scolarité".into(),
                categorie: "scolarite".into(),
                compte_produit: None,
                actif: None,
            },
        )
        .await
        .unwrap();
    ent.creer_tarif_for_actor(
        etab,
        u,
        r,
        &TarifBody {
            annee_scolaire: ANNEE.into(),
            article_id: art.id,
            niveau: None,
            regime: None,
            qf_min_centimes: None,
            qf_max_centimes: None,
            montant_annuel_centimes: 900_000,
        },
    )
    .await
    .unwrap();
    let el = eleve(&ent, etab, "Lou", "externe").await;
    let f = foyer_payeur(&ent, etab, el, c.parent_lea.user_id, "Foyer F3-2").await;
    facturer(&ent, etab, el).await;
    let (brouillon,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_fac_facture WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'brouillon'")
        .bind(etab).bind(f).fetch_one(ent.pool_verif()).await.unwrap();
    // Défaut de transition : rien n'est exigé tant que l'établissement ne l'a pas décidé ; le compte des
    // articles à qualifier est annoncé.
    let rg = ent.reglages_grilles_for_actor(etab, u, r).await.unwrap();
    assert!(
        !rg.exiger_qualification_articles
            && rg.grille_second_regard
            && rg.articles_non_qualifies == 1
    );
    // Seule la comptabilité / direction (droit `parametrer`) règle l'exigence.
    let (us, rs) = secretariat(&c);
    assert!(refuse(
        ent.definir_reglages_grilles_for_actor(
            etab,
            us,
            rs,
            &ReglagesGrillesBody {
                exiger_qualification_articles: Some(true),
                grille_second_regard: None
            }
        )
        .await
    ));
    let rg = ent
        .definir_reglages_grilles_for_actor(
            etab,
            u,
            r,
            &ReglagesGrillesBody {
                exiger_qualification_articles: Some(true),
                grille_second_regard: None,
            },
        )
        .await
        .unwrap();
    assert!(
        rg.exiger_qualification_articles && rg.grille_second_regard,
        "le second regard reste actif"
    );
    let rapport = ent
        .emettre_for_actor(
            etab,
            u,
            r,
            &EmissionBody {
                factures: vec![brouillon],
            },
        )
        .await
        .unwrap();
    assert!(
        rapport.emises.is_empty()
            && rapport.refus.len() == 1
            && rapport.refus[0].motif.contains("non qualifié"),
        "{rapport:?}"
    );
    // Création « à l'ancienne » refusée : il faut un modèle ou la qualification.
    assert!(conflit(
        ent.enregistrer_article_for_actor(
            etab,
            u,
            r,
            None,
            &ArticleBody {
                code: "NEUF".into(),
                libelle: "Neuf".into(),
                categorie: "frais".into(),
                compte_produit: None,
                actif: None
            }
        )
        .await
    ));
    ent.qualifier_article_for_actor(
        etab,
        u,
        r,
        art.id,
        &QualifierArticleBody {
            nature_legale: "contribution_r442_48_2".into(),
            regime_tva: "exonere_261_4_4".into(),
            taux_tva_bps: None,
            unite: "annee".into(),
            mode: None,
            remboursable: None,
            soumis_reductions: None,
            soumis_remise_ordre: None,
        },
    )
    .await
    .unwrap();
    let rapport = ent
        .emettre_for_actor(
            etab,
            u,
            r,
            &EmissionBody {
                factures: vec![brouillon],
            },
        )
        .await
        .unwrap();
    assert_eq!(rapport.emises.len(), 1, "{rapport:?}");
    assert_eq!(journal_contient(&ent, etab, "grille.reglages").await, 1);
}

// ───────────────────────── Grilles (R-F10, R-F23, FAC-116 à FAC-129) ─────────────────────────

/// Cycle complet : brouillon → proposée → votée → en vigueur ; second regard ; gel par la base (tarifs, tranches,
/// transitions) ; la génération ne facture que la grille en vigueur ; correction par nouvelle version ; publication.
#[tokio::test]
async fn f3_cycle_de_vie_gel_correction_et_publication() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ud, rd) = direction(&c);
    // Tarif historique : valable tant qu'aucune grille n'est en vigueur pour l'année.
    let (v, contrib, dp) = grille_proposee(&ent, &c).await;
    let detail = ent.grille_for_actor(etab, u, r, v).await.unwrap();
    assert_eq!(detail.version.statut, "proposee");
    assert_eq!((detail.version.numero, detail.tarifs.len()), (1, 2));
    // Proposée : plus aucune modification possible par le service.
    assert!(conflit(
        ent.ajouter_tarif_grille_for_actor(etab, u, r, v, &tarif_body(contrib, 1, false))
            .await
    ));
    assert!(conflit(
        ent.appliquer_hausse_for_actor(
            etab,
            u,
            r,
            v,
            &HausseBody {
                mode: ModeHausse::Montant,
                valeur: 100,
                arrondi: Arrondi::Aucun,
                article_id: None,
                apercu: false
            }
        )
        .await
    ));
    assert!(conflit(
        ent.definir_tranches_for_actor(etab, u, r, v, &TranchesBody { tranches: vec![] })
            .await
    ));
    // R-F23 : la comptabilité ne vote pas (droit absent) ; celui qui a proposé ne vote pas non plus.
    assert!(refuse(
        ent.voter_grille_test(etab, u, r, v, &vote("CA du 20 juin"))
            .await
    ));
    // Direction : vote valide ; référence obligatoire, vote d'une grille déjà votée refusé.
    assert!(invalide(
        ent.voter_grille_test(etab, ud, rd, v, &vote("   "))
            .await
    ));
    let votee = ent
        .voter_grille_test(etab, ud, rd, v, &vote("CA du 20 juin"))
        .await
        .unwrap();
    assert_eq!(
        (votee.statut.as_str(), votee.vote_reference.as_deref()),
        ("votee", Some("CA du 20 juin"))
    );
    assert!(conflit(
        ent.voter_grille_test(etab, ud, rd, v, &vote("encore"))
            .await
    ));
    // Le gel est tenu par la base elle-même, pas seulement par le service (R-F10).
    let pool = ent.pool_verif();
    assert!(sqlx::query(
        "UPDATE ent_fin_tarif SET montant_annuel_centimes = 1 WHERE grille_version_id = $1"
    )
    .bind(v)
    .execute(pool)
    .await
    .is_err());
    assert!(
        sqlx::query("DELETE FROM ent_fin_tarif WHERE grille_version_id = $1")
            .bind(v)
            .execute(pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE ent_fin_tarif SET actif = false WHERE grille_version_id = $1")
            .bind(v)
            .execute(pool)
            .await
            .is_err()
    );
    assert!(sqlx::query("INSERT INTO ent_fin_tarif (id, etablissement_id, annee_scolaire, article_id, montant_annuel_centimes, grille_version_id) VALUES (gen_random_uuid(), $1, $2, $3, 5, $4)")
        .bind(etab).bind(ANNEE).bind(contrib).bind(v).execute(pool).await.is_err());
    assert!(
        sqlx::query("UPDATE ent_fin_grille_version SET libelle = 'autre' WHERE id = $1")
            .bind(v)
            .execute(pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE ent_fin_grille_version SET statut = 'brouillon' WHERE id = $1")
            .bind(v)
            .execute(pool)
            .await
            .is_err(),
        "transition interdite"
    );
    assert!(
        sqlx::query("DELETE FROM ent_fin_grille_version WHERE id = $1")
            .bind(v)
            .execute(pool)
            .await
            .is_err()
    );
    // Un tarif historique n'est plus saisi sur une année qui a une grille votée.
    assert!(conflit(
        ent.creer_tarif_for_actor(
            etab,
            u,
            r,
            &TarifBody {
                annee_scolaire: ANNEE.into(),
                article_id: contrib,
                niveau: None,
                regime: None,
                qf_min_centimes: None,
                qf_max_centimes: None,
                montant_annuel_centimes: 7
            }
        )
        .await
    ));
    // Mise en vigueur : par la comptabilité (`tarifs`), seulement une grille votée.
    let en_vigueur = ent
        .mettre_en_vigueur_for_actor(etab, u, r, v, &MiseEnVigueurBody { du: d(2030, 9, 1) })
        .await
        .unwrap();
    assert_eq!(
        (en_vigueur.statut.as_str(), en_vigueur.en_vigueur_du),
        ("en_vigueur", Some(d(2030, 9, 1)))
    );
    // La génération facture au tarif de la grille en vigueur (1 200 €), pas à un autre.
    let el = eleve(&ent, etab, "Mia", "externe").await;
    let f = foyer_payeur(&ent, etab, el, c.parent_lea.user_id, "Foyer F3-3").await;
    facturer(&ent, etab, el).await;
    let (total,): (i64,) = sqlx::query_as("SELECT COALESCE(sum(l.montant_unitaire_centimes * l.quantite), 0)::bigint FROM ent_fac_ligne l JOIN ent_fac_facture fa ON fa.id = l.facture_id WHERE fa.etablissement_id = $1 AND fa.foyer_id = $2 AND l.categorie = 'scolarite'")
        .bind(etab).bind(f).fetch_one(pool).await.unwrap();
    assert_eq!(total, 120_000);

    // Correction (FAC-128) : nouvelle version en brouillon, copie, motif obligatoire ; la comptabilité ne corrige pas.
    assert!(refuse(
        ent.corriger_grille_for_actor(
            etab,
            u,
            r,
            v,
            &CorrectionGrilleBody {
                motif: "erreur".into(),
                effet_retroactif: false
            }
        )
        .await
    ));
    assert!(invalide(
        ent.corriger_grille_for_actor(
            etab,
            ud,
            rd,
            v,
            &CorrectionGrilleBody {
                motif: "  ".into(),
                effet_retroactif: false
            }
        )
        .await
    ));
    let v2 = ent
        .corriger_grille_for_actor(
            etab,
            ud,
            rd,
            v,
            &CorrectionGrilleBody {
                motif: "erreur de saisie sur la contribution".into(),
                effet_retroactif: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (v2.numero, v2.statut.as_str(), v2.remplace_id, v2.nb_tarifs),
        (2, "brouillon", Some(v), 2)
    );
    assert!(
        conflit(
            ent.corriger_grille_for_actor(
                etab,
                ud,
                rd,
                v,
                &CorrectionGrilleBody {
                    motif: "bis".into(),
                    effet_retroactif: false
                }
            )
            .await
        ),
        "une seule correction en cours"
    );
    // Le brouillon de correction est modifiable ; la grille en vigueur reste intacte tant que v2 n'est pas votée.
    let det2 = ent.grille_for_actor(etab, u, r, v2.id).await.unwrap();
    let t_contrib = det2
        .tarifs
        .iter()
        .find(|t| t.article_id == contrib)
        .unwrap();
    ent.modifier_tarif_grille_for_actor(
        etab,
        u,
        r,
        v2.id,
        t_contrib.id,
        &tarif_body(contrib, 118_000, true),
    )
    .await
    .unwrap();
    ent.proposer_grille_for_actor(etab, u, r, v2.id)
        .await
        .unwrap();
    // la correction est rédigée par la direction : le second regard est une autre personne (administrateur)
    ent.voter_grille_test(
        etab,
        c.admin.user_id,
        Role::Admin.as_str(),
        v2.id,
        &vote("CA du 3 octobre"),
    )
    .await
    .unwrap();
    let nouvelle = ent
        .mettre_en_vigueur_for_actor(
            etab,
            u,
            r,
            v2.id,
            &MiseEnVigueurBody {
                du: d(2030, 10, 15),
            },
        )
        .await
        .unwrap();
    assert_eq!(nouvelle.statut, "en_vigueur");
    let (anciennes,): (String,) =
        sqlx::query_as("SELECT statut FROM ent_fin_grille_version WHERE id = $1")
            .bind(v)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        anciennes, "close",
        "la version précédente est close : une seule en vigueur par année"
    );
    assert!(
        sqlx::query("UPDATE ent_fin_grille_version SET statut = 'en_vigueur' WHERE id = $1")
            .bind(v)
            .execute(pool)
            .await
            .is_err()
    );
    // Les pièces déjà générées ne changent pas : aucune réécriture rétroactive (proposition au lot F4).
    let (total2,): (i64,) = sqlx::query_as("SELECT COALESCE(sum(l.montant_unitaire_centimes * l.quantite), 0)::bigint FROM ent_fac_ligne l JOIN ent_fac_facture fa ON fa.id = l.facture_id WHERE fa.etablissement_id = $1 AND fa.foyer_id = $2 AND l.categorie = 'scolarite'")
        .bind(etab).bind(f).fetch_one(pool).await.unwrap();
    assert_eq!(total2, 120_000);
    let _ = dp;

    // Publication (FAC-129) : droit `publier` (direction) ; aucune donnée personnelle ; date posée une fois.
    assert!(refuse(
        ent.publier_grille_for_actor(etab, u, r, nouvelle.id).await
    ));
    let p1 = ent
        .publier_grille_for_actor(etab, ud, rd, nouvelle.id)
        .await
        .unwrap();
    let p2 = ent
        .publier_grille_for_actor(etab, ud, rd, nouvelle.id)
        .await
        .unwrap();
    assert_eq!(p1.publiee_le, p2.publiee_le);
    assert_eq!(p1.lignes.len(), 2);
    assert!(p1
        .lignes
        .iter()
        .any(|l| l.montant_annuel_centimes == 118_000));
    assert_eq!(
        journal_contient(&ent, etab, "grille.publier").await,
        1,
        "publiée une seule fois"
    );
    for a in [
        "grille.creer",
        "grille.proposer",
        "grille.voter",
        "grille.en_vigueur",
        "grille.corriger",
    ] {
        assert!(journal_contient(&ent, etab, a).await >= 1, "journal : {a}");
    }
    // Aucune charge d'événement ne porte montant ni libellé ; les événements ont bien été émis (non-vacuité).
    let (emis,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type LIKE 'facturation.grille_%'")
        .bind(etab).fetch_one(pool).await.unwrap();
    assert!(emis >= 5, "événements de grille émis : {emis}");
    let (fuites,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type LIKE 'facturation.grille_%' AND payload::text ~* '(centimes|montant|libelle)'")
        .bind(etab).fetch_one(pool).await.unwrap();
    assert_eq!(fuites, 0);
}

/// R-F23 : avec le second regard (défaut), celui qui a proposé ne vote pas ; l'établissement peut le désactiver
/// (une seule direction) ; l'abandon et le retour en brouillon sont motivés.
#[tokio::test]
async fn f3_second_regard_retour_brouillon_et_abandon() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (ud, rd) = direction(&c);
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
    let v = ent
        .creer_grille_for_actor(
            etab,
            ud,
            rd,
            &CreerGrilleBody {
                annee_scolaire: ANNEE.into(),
                libelle: Some("Grille direction".into()),
                copier_de: None,
                reprendre_historique: None,
            },
        )
        .await
        .unwrap();
    ent.ajouter_tarif_grille_for_actor(etab, ud, rd, v.id, &tarif_body(contrib.id, 100_000, true))
        .await
        .unwrap();
    ent.proposer_grille_for_actor(etab, ud, rd, v.id)
        .await
        .unwrap();
    assert!(
        interdit(
            ent.voter_grille_test(etab, ud, rd, v.id, &vote("CA"))
                .await
        ),
        "même personne : second regard"
    );
    // Retour en brouillon motivé (le conseil demande un changement), puis nouvelle proposition.
    assert!(invalide(
        ent.retour_brouillon_for_actor(
            etab,
            ud,
            rd,
            v.id,
            &RetourBrouillonBody { motif: " ".into() }
        )
        .await
    ));
    let b = ent
        .retour_brouillon_for_actor(
            etab,
            ud,
            rd,
            v.id,
            &RetourBrouillonBody {
                motif: "le conseil demande +1 %".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(b.statut, "brouillon");
    // L'établissement n'a qu'une direction : il désactive le second regard (paramètre, journalisé) — hors
    // proposition en cours, jamais pendant le vote (règle 3).
    ent.definir_reglages_grilles_for_actor(
        etab,
        ud,
        rd,
        &ReglagesGrillesBody {
            exiger_qualification_articles: None,
            grille_second_regard: Some(false),
        },
    )
    .await
    .unwrap();
    ent.proposer_grille_for_actor(etab, ud, rd, v.id)
        .await
        .unwrap();
    let votee = ent
        .voter_grille_test(etab, ud, rd, v.id, &vote("CA"))
        .await
        .unwrap();
    assert_eq!(votee.statut, "votee");
    assert!(
        conflit(
            ent.retour_brouillon_for_actor(
                etab,
                ud,
                rd,
                v.id,
                &RetourBrouillonBody {
                    motif: "trop tard".into()
                }
            )
            .await
        ),
        "une grille votée ne revient pas en brouillon"
    );
    assert!(conflit(
        ent.abandonner_grille_for_actor(
            etab,
            ud,
            rd,
            v.id,
            &RetourBrouillonBody { motif: "x".into() }
        )
        .await
    ));
    // Un second brouillon abandonné reste visible (jamais supprimé).
    let v2 = ent
        .creer_grille_for_actor(
            etab,
            ud,
            rd,
            &CreerGrilleBody {
                annee_scolaire: ANNEE.into(),
                libelle: None,
                copier_de: Some(v.id),
                reprendre_historique: None,
            },
        )
        .await
        .unwrap();
    assert_eq!((v2.numero, v2.nb_tarifs), (2, 1), "copie d'une version");
    ent.abandonner_grille_for_actor(
        etab,
        ud,
        rd,
        v2.id,
        &RetourBrouillonBody {
            motif: "essai".into(),
        },
    )
    .await
    .unwrap();
    let toutes = ent
        .grilles_for_actor(etab, ud, rd, Some(ANNEE.into()))
        .await
        .unwrap();
    assert!(toutes
        .iter()
        .any(|g| g.id == v2.id && g.statut == "abandonnee"));
    // Pièce de la délibération : un document d'un autre établissement ou inexistant est introuvable.
    let v3 = ent
        .creer_grille_for_actor(
            etab,
            ud,
            rd,
            &CreerGrilleBody {
                annee_scolaire: "2031-2032".into(),
                libelle: None,
                copier_de: Some(v.id),
                reprendre_historique: None,
            },
        )
        .await
        .unwrap();
    ent.proposer_grille_for_actor(etab, ud, rd, v3.id)
        .await
        .unwrap();
    assert!(introuvable(
        ent.voter_grille_test(
            etab,
            ud,
            rd,
            v3.id,
            &VoteGrilleBody {
                vote_le: d(2030, 6, 1),
                reference: "CA".into(),
                document_id: Some(Uuid::new_v4())
            }
        )
        .await
    ));
}

/// Hausse (aperçu puis application), tranches sans trou, contrôles bloquants avant proposition, reprise de
/// l'historique (MIG-6) : tout se joue sur un brouillon.
#[tokio::test]
async fn f3_hausse_tranches_controles_et_reprise_historique() {
    let Some((ent, c)) = labo().await else { return };
    etablissement_anterieur(&ent, c.etablissement_id).await;
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
    // Tarifs historiques (avant les versions) repris dans un brouillon.
    ent.creer_tarif_for_actor(
        etab,
        u,
        r,
        &TarifBody {
            annee_scolaire: ANNEE.into(),
            article_id: contrib.id,
            niveau: None,
            regime: None,
            qf_min_centimes: None,
            qf_max_centimes: None,
            montant_annuel_centimes: 123_456,
        },
    )
    .await
    .unwrap();
    let v = ent
        .creer_grille_for_actor(
            etab,
            u,
            r,
            &CreerGrilleBody {
                annee_scolaire: ANNEE.into(),
                libelle: None,
                copier_de: None,
                reprendre_historique: Some(true),
            },
        )
        .await
        .unwrap();
    assert_eq!(v.nb_tarifs, 1);
    // Un tarif par tranche sans tranche définie, sans tarif par défaut : anomalies bloquantes.
    ent.ajouter_tarif_grille_for_actor(
        etab,
        u,
        r,
        v.id,
        &TarifGrilleBody {
            article_id: contrib.id,
            niveau: None,
            regime: None,
            qf_min_centimes: Some(0),
            qf_max_centimes: Some(50_000),
            montant_annuel_centimes: 80_000,
            tarif_par_defaut: None,
        },
    )
    .await
    .unwrap();
    let ctl = ent
        .controles_grille_for_actor(etab, u, r, v.id)
        .await
        .unwrap();
    assert!(
        !ctl.proposable
            && ctl
                .anomalies
                .iter()
                .any(|a| a.code == "tranches_manquantes"),
        "{ctl:?}"
    );
    assert!(conflit(
        ent.proposer_grille_for_actor(etab, u, r, v.id).await
    ));
    // Tranches : trou, chevauchement, ouverture ; puis bonnes tranches.
    let tr = |code: &str, min: i64, max: Option<i64>| TrancheQfBody {
        code: code.into(),
        libelle: format!("Tranche {code}"),
        qf_min_centimes: min,
        qf_max_centimes: max,
    };
    assert!(invalide(
        ent.definir_tranches_for_actor(
            etab,
            u,
            r,
            v.id,
            &TranchesBody {
                tranches: vec![tr("A", 0, Some(50_000)), tr("B", 60_000, None)]
            }
        )
        .await
    ));
    assert!(invalide(
        ent.definir_tranches_for_actor(
            etab,
            u,
            r,
            v.id,
            &TranchesBody {
                tranches: vec![tr("A", 0, Some(60_000)), tr("B", 50_000, None)]
            }
        )
        .await
    ));
    let tranches = ent
        .definir_tranches_for_actor(
            etab,
            u,
            r,
            v.id,
            &TranchesBody {
                tranches: vec![tr("A", 0, Some(50_000)), tr("B", 50_000, None)],
            },
        )
        .await
        .unwrap();
    assert_eq!(tranches.len(), 2);
    // Le tarif historique repris (123 456) est sans borne : on le désigne tarif par défaut.
    let det = ent.grille_for_actor(etab, u, r, v.id).await.unwrap();
    let pleins = det
        .tarifs
        .iter()
        .find(|t| t.qf_min_centimes.is_none())
        .unwrap();
    ent.modifier_tarif_grille_for_actor(
        etab,
        u,
        r,
        v.id,
        pleins.id,
        &tarif_body(contrib.id, pleins.montant_annuel_centimes, true),
    )
    .await
    .unwrap();
    assert!(
        ent.controles_grille_for_actor(etab, u, r, v.id)
            .await
            .unwrap()
            .proposable
    );
    // Aperçu : rien n'est écrit. Application : +3 % arrondi à l'euro ; le tarif d'un autre article est épargné.
    let apercu = ent
        .appliquer_hausse_for_actor(
            etab,
            u,
            r,
            v.id,
            &HausseBody {
                mode: ModeHausse::Pourcentage,
                valeur: 300,
                arrondi: Arrondi::Euro,
                article_id: None,
                apercu: true,
            },
        )
        .await
        .unwrap();
    assert!(apercu.apercu);
    let avant = ent.grille_for_actor(etab, u, r, v.id).await.unwrap();
    assert_eq!(
        avant
            .tarifs
            .iter()
            .map(|t| t.montant_annuel_centimes)
            .sum::<i64>(),
        123_456 + 80_000,
        "aperçu sans écriture"
    );
    let apres = ent
        .appliquer_hausse_for_actor(
            etab,
            u,
            r,
            v.id,
            &HausseBody {
                mode: ModeHausse::Pourcentage,
                valeur: 300,
                arrondi: Arrondi::Euro,
                article_id: None,
                apercu: false,
            },
        )
        .await
        .unwrap();
    let nouveaux: Vec<i64> = apres.lignes.iter().map(|l| l.apres_centimes).collect();
    assert!(
        nouveaux.contains(&127_200) && nouveaux.contains(&82_400),
        "{nouveaux:?}"
    );
    let det = ent.grille_for_actor(etab, u, r, v.id).await.unwrap();
    assert_eq!(
        det.tarifs
            .iter()
            .map(|t| t.montant_annuel_centimes)
            .sum::<i64>(),
        127_200 + 82_400
    );
    // Hausse sur un article absent de la grille : introuvable ; hors bornes : invalide.
    assert!(introuvable(
        ent.appliquer_hausse_for_actor(
            etab,
            u,
            r,
            v.id,
            &HausseBody {
                mode: ModeHausse::Montant,
                valeur: 1,
                arrondi: Arrondi::Aucun,
                article_id: Some(Uuid::new_v4()),
                apercu: true
            }
        )
        .await
    ));
    assert!(invalide(
        ent.appliquer_hausse_for_actor(
            etab,
            u,
            r,
            v.id,
            &HausseBody {
                mode: ModeHausse::Montant,
                valeur: 99_000_000,
                arrondi: Arrondi::Aucun,
                article_id: None,
                apercu: true
            }
        )
        .await
    ));
    // Un tarif par défaut en double est refusé par la base (un seul par article et par version).
    assert!(conflit(
        ent.ajouter_tarif_grille_for_actor(etab, u, r, v.id, &tarif_body(contrib.id, 5, true))
            .await
    ));
    ent.proposer_grille_for_actor(etab, u, r, v.id)
        .await
        .unwrap();
    // Un article non qualifié bloque la proposition (R-F9).
    let legacy = ent
        .enregistrer_article_for_actor(
            etab,
            u,
            r,
            None,
            &ArticleBody {
                code: "LEG".into(),
                libelle: "Legacy".into(),
                categorie: "frais".into(),
                compte_produit: None,
                actif: None,
            },
        )
        .await
        .unwrap();
    let v2 = ent
        .creer_grille_for_actor(
            etab,
            u,
            r,
            &CreerGrilleBody {
                annee_scolaire: ANNEE.into(),
                libelle: None,
                copier_de: Some(v.id),
                reprendre_historique: None,
            },
        )
        .await
        .unwrap();
    ent.ajouter_tarif_grille_for_actor(etab, u, r, v2.id, &tarif_body(legacy.id, 5_000, false))
        .await
        .unwrap();
    let ctl = ent
        .controles_grille_for_actor(etab, u, r, v2.id)
        .await
        .unwrap();
    assert!(ctl
        .anomalies
        .iter()
        .any(|a| a.code == "article_non_qualifie" && a.bloquante));
    assert!(conflit(
        ent.proposer_grille_for_actor(etab, u, r, v2.id).await
    ));
}

// ───────────────────────── Simulateur et familles types (FAC-121 à FAC-124) ─────────────────────────

/// Produit attendu sur des effectifs prévus, hausse par article, familles types avec tranche et fratrie,
/// comparaison de deux scénarios, export pour le conseil : tout en centimes entiers, sans aucun nom.
#[tokio::test]
async fn f3_simulation_effectifs_familles_types_comparaison_et_export() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (us, rs) = secretariat(&c);
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
    let dp = ent
        .article_depuis_modele_for_actor(
            etab,
            u,
            r,
            &ArticleDepuisModeleBody {
                modele_code: "DEMI_PENSION".into(),
                code: Some("DP".into()),
                libelle: Some("=SOMME(A1:A9)".into()),
                compte_produit: None,
            },
        )
        .await
        .unwrap();
    let v = ent
        .creer_grille_for_actor(
            etab,
            u,
            r,
            &CreerGrilleBody {
                annee_scolaire: ANNEE.into(),
                libelle: None,
                copier_de: None,
                reprendre_historique: None,
            },
        )
        .await
        .unwrap();
    let tarif = |article: Uuid,
                 niveau: Option<&str>,
                 regime: Option<&str>,
                 min: Option<i64>,
                 max: Option<i64>,
                 m: i64,
                 defaut: bool| TarifGrilleBody {
        article_id: article,
        niveau: niveau.map(Into::into),
        regime: regime.map(Into::into),
        qf_min_centimes: min,
        qf_max_centimes: max,
        montant_annuel_centimes: m,
        tarif_par_defaut: Some(defaut),
    };
    ent.ajouter_tarif_grille_for_actor(
        etab,
        u,
        r,
        v.id,
        &tarif(contrib.id, None, None, None, None, 100_000, true),
    )
    .await
    .unwrap();
    ent.ajouter_tarif_grille_for_actor(
        etab,
        u,
        r,
        v.id,
        &tarif(contrib.id, Some("6EME"), None, None, None, 120_000, false),
    )
    .await
    .unwrap();
    ent.ajouter_tarif_grille_for_actor(
        etab,
        u,
        r,
        v.id,
        &tarif(contrib.id, None, None, Some(0), Some(50_000), 60_000, false),
    )
    .await
    .unwrap();
    ent.ajouter_tarif_grille_for_actor(
        etab,
        u,
        r,
        v.id,
        &tarif(dp.id, None, Some("demi_pension"), None, None, 50_000, true),
    )
    .await
    .unwrap();
    ent.definir_tranches_for_actor(
        etab,
        u,
        r,
        v.id,
        &TranchesBody {
            tranches: vec![
                TrancheQfBody {
                    code: "A".into(),
                    libelle: "Tranche A".into(),
                    qf_min_centimes: 0,
                    qf_max_centimes: Some(50_000),
                },
                TrancheQfBody {
                    code: "B".into(),
                    libelle: "Tranche B".into(),
                    qf_min_centimes: 50_000,
                    qf_max_centimes: None,
                },
            ],
        },
    )
    .await
    .unwrap();
    // Fratrie : -10 % à partir du 2e enfant sur la scolarité.
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
    // Familles types fictives ; régime inconnu refusé ; droit `simuler` exigé.
    let ft = |libelle: &str, tranche: Option<&str>, enfants: Vec<(&str, &str)>| FamilleTypeBody {
        libelle: libelle.into(),
        tranche_code: tranche.map(Into::into),
        enfants: enfants
            .into_iter()
            .map(|(n, rg)| EnfantTypeBody {
                niveau: n.into(),
                regime: rg.into(),
            })
            .collect(),
    };
    assert!(refuse(
        ent.creer_famille_type_for_actor(etab, us, rs, &ft("X", None, vec![("6EME", "externe")]))
            .await
    ));
    assert!(invalide(
        ent.creer_famille_type_for_actor(
            etab,
            u,
            r,
            &ft("X", None, vec![("6EME", "pensionnaire")])
        )
        .await
    ));
    assert!(invalide(
        ent.creer_famille_type_for_actor(etab, u, r, &ft("X", None, vec![]))
            .await
    ));
    let f_deux = ent
        .creer_famille_type_for_actor(
            etab,
            u,
            r,
            &ft(
                "2 enfants DP, tranche A",
                Some("A"),
                vec![("6EME", "externe"), ("5EME", "demi_pension")],
            ),
        )
        .await
        .unwrap();
    assert_eq!(f_deux.enfants.len(), 2);
    assert!(conflit(
        ent.creer_famille_type_for_actor(
            etab,
            u,
            r,
            &ft("2 enfants DP, tranche A", None, vec![("6EME", "externe")])
        )
        .await
    ));
    ent.creer_famille_type_for_actor(
        etab,
        u,
        r,
        &ft("1 enfant plein tarif", None, vec![("6EME", "externe")]),
    )
    .await
    .unwrap();
    // Effet de la grille sur les familles types (FAC-124), sans hausse.
    let effet = ent
        .effet_familles_types_for_actor(etab, u, r, v.id)
        .await
        .unwrap();
    let e2 = effet
        .iter()
        .find(|e| e.libelle.starts_with("2 enfants"))
        .unwrap();
    // Spécificité de `choisir_tarif` : niveau > régime > tranche. Enfant 1 (6EME) : le tarif du niveau, 1 200 €.
    // Enfant 2 (5EME, demi-pension, quotient de la tranche A) : tarif de la tranche 600 €, -10 % de fratrie =
    // 540 €, plus demi-pension 500 €.
    assert_eq!(e2.total_base_centimes, 120_000 + 54_000 + 50_000);
    let e1 = effet
        .iter()
        .find(|e| e.libelle.starts_with("1 enfant"))
        .unwrap();
    assert_eq!(
        e1.total_base_centimes, 120_000,
        "plein tarif : le tarif du niveau 6EME"
    );

    // Scénario : +3 % arrondi à l'euro sur la contribution seulement ; effectifs prévus saisis.
    let sc = SimulationBody {
        annee_scolaire: ANNEE.into(),
        libelle: "Hausse 3 %".into(),
        base_version_id: v.id,
        regles: vec![RegleHausseBody {
            article_id: Some(contrib.id),
            mode: ModeHausse::Pourcentage,
            valeur: 300,
            arrondi: Arrondi::Euro,
        }],
        effectifs: vec![
            EffectifPrevuBody {
                niveau: "6EME".into(),
                regime: "externe".into(),
                effectif: 10,
            },
            EffectifPrevuBody {
                niveau: "5EME".into(),
                regime: "demi_pension".into(),
                effectif: 4,
            },
        ],
    };
    assert!(refuse(
        ent.creer_simulation_for_actor(etab, us, rs, &sc).await
    ));
    let res = ent
        .creer_simulation_for_actor(etab, u, r, &sc)
        .await
        .unwrap();
    assert_eq!(
        res.produit_base_centimes,
        10 * 120_000 + 4 * 100_000 + 4 * 50_000
    );
    assert_eq!(
        res.produit_simule_centimes,
        10 * 123_600 + 4 * 103_000 + 4 * 50_000
    );
    assert_eq!(
        res.ecart_centimes,
        res.produit_simule_centimes - res.produit_base_centimes
    );
    assert!(res.effectifs.iter().all(|e| e.origine == "prevu"));
    assert_eq!(res.par_article.len(), 2);
    // Écart par famille type : 1 200 € → 1 236 € (6EME) ; tranche A 600 € → 618 €, fratrie -10 % = 556,20 €.
    let f = res
        .familles
        .iter()
        .find(|f| f.libelle.starts_with("2 enfants"))
        .unwrap();
    assert_eq!(
        (f.total_base_centimes, f.total_simule_centimes),
        (224_000, 123_600 + 55_620 + 50_000)
    );
    // Doublon d'effectif, effectif négatif, année différente de la version, version abandonnée : refus.
    let mut mauvais = SimulationBody {
        effectifs: vec![
            EffectifPrevuBody {
                niveau: "6EME".into(),
                regime: "externe".into(),
                effectif: 1,
            },
            EffectifPrevuBody {
                niveau: "6EME".into(),
                regime: "externe".into(),
                effectif: 2,
            },
        ],
        ..clone_sim(&sc)
    };
    assert!(invalide(
        ent.creer_simulation_for_actor(etab, u, r, &mauvais).await
    ));
    mauvais.effectifs = vec![EffectifPrevuBody {
        niveau: "6EME".into(),
        regime: "externe".into(),
        effectif: -1,
    }];
    assert!(invalide(
        ent.creer_simulation_for_actor(etab, u, r, &mauvais).await
    ));
    mauvais.effectifs = vec![];
    mauvais.annee_scolaire = "2031-2032".into();
    assert!(invalide(
        ent.creer_simulation_for_actor(etab, u, r, &mauvais).await
    ));
    mauvais.annee_scolaire = ANNEE.into();
    mauvais.base_version_id = Uuid::new_v4();
    assert!(introuvable(
        ent.creer_simulation_for_actor(etab, u, r, &mauvais).await
    ));
    // Deuxième scénario sans effectifs prévus : effectifs actuels (dénombrement), origine « actuel ».
    let sc2 = SimulationBody {
        libelle: "Hausse 5 € par article".into(),
        regles: vec![RegleHausseBody {
            article_id: None,
            mode: ModeHausse::Montant,
            valeur: 500,
            arrondi: Arrondi::Aucun,
        }],
        effectifs: vec![],
        ..clone_sim(&sc)
    };
    let res2 = ent
        .creer_simulation_for_actor(etab, u, r, &sc2)
        .await
        .unwrap();
    assert!(res2.effectifs.iter().all(|e| e.origine == "actuel"));
    // Comparaison : écarts de produit et par famille type, calcul à la demande.
    let cmp = ent
        .comparer_simulations_for_actor(etab, u, r, res.simulation_id, res2.simulation_id)
        .await
        .unwrap();
    assert_eq!(
        cmp.ecart_produit_centimes,
        res2.produit_simule_centimes - res.produit_simule_centimes
    );
    assert!(cmp
        .familles
        .iter()
        .any(|e| e.libelle.starts_with("2 enfants")));
    assert!(introuvable(
        ent.comparer_simulations_for_actor(etab, u, r, res.simulation_id, Uuid::new_v4())
            .await
    ));
    assert_eq!(
        ent.simulations_for_actor(etab, u, r).await.unwrap().len(),
        2
    );
    // Export : tableau CSV sans donnée personnelle ; une formule dans un libellé est neutralisée.
    let exp = ent
        .exporter_conseil_for_actor(etab, u, r, res.simulation_id)
        .await
        .unwrap();
    assert!(exp.csv.starts_with("rubrique;libelle;") && exp.csv.contains("total;;"));
    assert!(!exp.csv.contains("Testf3"), "aucun nom");
    assert!(
        exp.csv.contains("'=SOMME(A1:A9)") && !exp.csv.contains(";=SOMME"),
        "formule neutralisée dans le CSV"
    );
    // Les simulations ne s'effacent pas (historique des scénarios présentés).
    assert!(sqlx::query("DELETE FROM ent_fin_simulation WHERE id = $1")
        .bind(res.simulation_id)
        .execute(ent.pool_verif())
        .await
        .is_err());
    assert!(
        sqlx::query("DELETE FROM ent_fin_famille_type WHERE id = $1")
            .bind(f_deux.id)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    ent.desactiver_famille_type_for_actor(etab, u, r, f_deux.id)
        .await
        .unwrap();
    assert!(introuvable(
        ent.desactiver_famille_type_for_actor(etab, u, r, f_deux.id)
            .await
    ));
    assert_eq!(
        ent.familles_types_for_actor(etab, u, r)
            .await
            .unwrap()
            .len(),
        1
    );
    let recalcul = ent
        .calculer_simulation_for_actor(etab, u, r, res.simulation_id)
        .await
        .unwrap();
    assert_eq!(
        recalcul.produit_simule_centimes, res.produit_simule_centimes,
        "recalcul identique"
    );
}

fn clone_sim(s: &SimulationBody) -> SimulationBody {
    SimulationBody {
        annee_scolaire: s.annee_scolaire.clone(),
        libelle: s.libelle.clone(),
        base_version_id: s.base_version_id,
        regles: s
            .regles
            .iter()
            .map(|r| RegleHausseBody {
                article_id: r.article_id,
                mode: r.mode,
                valeur: r.valeur,
                arrondi: r.arrondi,
            })
            .collect(),
        effectifs: s
            .effectifs
            .iter()
            .map(|e| EffectifPrevuBody {
                niveau: e.niveau.clone(),
                regime: e.regime.clone(),
                effectif: e.effectif,
            })
            .collect(),
    }
}

// ───────────────────────── État des contributions (FAC-196) ─────────────────────────

/// L'état agrège les pièces émises par **nature légale** ; un article non qualifié apparaît « non_qualifie » ;
/// les avoirs se soustraient ; aucun nom.
#[tokio::test]
async fn f3_etat_des_contributions_par_nature_legale() {
    let Some((ent, c)) = labo().await else { return };
    etablissement_anterieur(&ent, c.etablissement_id).await;
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let scol = ent
        .enregistrer_article_for_actor(
            etab,
            u,
            r,
            None,
            &ArticleBody {
                code: "SCOL".into(),
                libelle: "Scolarité".into(),
                categorie: "scolarite".into(),
                compte_produit: None,
                actif: None,
            },
        )
        .await
        .unwrap();
    let dp = article_modele(&ent, &c, "DEMI_PENSION", "DP").await;
    for (a, m) in [(scol.id, 800_000_i64), (dp.id, 300_000)] {
        ent.creer_tarif_for_actor(
            etab,
            u,
            r,
            &TarifBody {
                annee_scolaire: ANNEE.into(),
                article_id: a,
                niveau: None,
                regime: None,
                qf_min_centimes: None,
                qf_max_centimes: None,
                montant_annuel_centimes: m,
            },
        )
        .await
        .unwrap();
    }
    let el = eleve(&ent, etab, "Eli", "demi_pension").await;
    let f = foyer_payeur(&ent, etab, el, c.parent_lea.user_id, "Foyer F3-6").await;
    facturer(&ent, etab, el).await;
    let ids: Vec<Uuid> = sqlx::query_as::<_, (Uuid,)>("SELECT id FROM ent_fac_facture WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'brouillon'")
        .bind(etab).bind(f).fetch_all(ent.pool_verif()).await.unwrap().into_iter().map(|x| x.0).collect();
    assert!(
        ent.emettre_for_actor(etab, u, r, &EmissionBody { factures: ids })
            .await
            .unwrap()
            .emises
            .len()
            == 1
    );
    let etat = ent
        .etat_contributions_for_actor(etab, u, r, ANNEE)
        .await
        .unwrap();
    let nat = |n: &str| {
        etat.lignes
            .iter()
            .find(|l| l.nature_legale == n)
            .map(|l| l.net_centimes)
    };
    assert_eq!(
        nat("non_qualifie"),
        Some(800_000),
        "scolarité non qualifiée"
    );
    assert_eq!(nat("demi_pension"), Some(300_000));
    assert_eq!(etat.total_net_centimes, 1_100_000);
    assert_eq!(etat.articles_non_qualifies, 1);
    // Qualification de la scolarité (MIG-6 : première qualification permise malgré les lignes) : elle migre
    // vers sa nature ; le total ne change pas.
    ent.qualifier_article_for_actor(
        etab,
        u,
        r,
        scol.id,
        &QualifierArticleBody {
            nature_legale: "contribution_r442_48_2".into(),
            regime_tva: "exonere_261_4_4".into(),
            taux_tva_bps: None,
            unite: "annee".into(),
            mode: None,
            remboursable: None,
            soumis_reductions: None,
            soumis_remise_ordre: None,
        },
    )
    .await
    .unwrap();
    let etat = ent
        .etat_contributions_for_actor(etab, u, r, ANNEE)
        .await
        .unwrap();
    let nat = |n: &str| {
        etat.lignes
            .iter()
            .find(|l| l.nature_legale == n)
            .map(|l| l.net_centimes)
    };
    assert_eq!(
        (nat("non_qualifie"), nat("contribution_r442_48_2")),
        (None, Some(800_000))
    );
    assert_eq!(
        (etat.total_net_centimes, etat.articles_non_qualifies),
        (1_100_000, 0)
    );
    let (us, rs) = secretariat(&c);
    assert!(refuse(
        ent.etat_contributions_for_actor(etab, us, rs, ANNEE).await
    ));
    assert!(invalide(
        ent.etat_contributions_for_actor(etab, u, r, "2030").await
    ));
    assert!(ent
        .etat_contributions_for_actor(etab, u, r, ANNEE)
        .await
        .unwrap()
        .csv
        .starts_with("nature_legale;"));
}

// ───────────────────────── Isolation entre établissements ─────────────────────────

/// Un second établissement vise la grille, l'article, la simulation et la famille type du premier :
/// « introuvable » à chaque fois, et rien n'a changé en base.
#[tokio::test]
async fn f3_acces_inter_etablissements_refuse_sans_effet() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let c2 = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let etab2 = c2.etablissement_id;
    let (u, r) = compta(&c);
    let (u2, r2) = compta(&c2);
    let (ud2, rd2) = direction(&c2);
    let (v, contrib, _dp) = grille_proposee(&ent, &c).await;
    let fam = ent
        .creer_famille_type_for_actor(
            etab,
            u,
            r,
            &FamilleTypeBody {
                libelle: "Fam".into(),
                tranche_code: None,
                enfants: vec![EnfantTypeBody {
                    niveau: "6EME".into(),
                    regime: "externe".into(),
                }],
            },
        )
        .await
        .unwrap();
    let sim = ent
        .creer_simulation_for_actor(
            etab,
            u,
            r,
            &SimulationBody {
                annee_scolaire: ANNEE.into(),
                libelle: "S".into(),
                base_version_id: v,
                regles: vec![],
                effectifs: vec![EffectifPrevuBody {
                    niveau: "6EME".into(),
                    regime: "externe".into(),
                    effectif: 1,
                }],
            },
        )
        .await
        .unwrap();
    let compte = |ent: &PgEnt| {
        let p = ent.pool_verif().clone();
        async move {
            sqlx::query_as::<_, (i64, i64, i64, i64)>("SELECT (SELECT count(*) FROM ent_fin_grille_version), (SELECT count(*) FROM ent_fin_tarif), (SELECT count(*) FROM ent_fin_simulation), (SELECT count(*) FROM ent_fin_famille_type)")
                .fetch_one(&p).await.unwrap()
        }
    };
    let avant = compte(&ent).await;
    assert!(introuvable(
        ent.grille_for_actor(etab2, u2, r2, v).await.map(|_| ())
    ));
    assert!(introuvable(
        ent.ajouter_tarif_grille_for_actor(etab2, u2, r2, v, &tarif_body(contrib, 1, false))
            .await
            .map(|_| ())
    ));
    assert!(introuvable(
        ent.proposer_grille_for_actor(etab2, u2, r2, v)
            .await
            .map(|_| ())
    ));
    assert!(introuvable(
        ent.voter_grille_test(etab2, ud2, rd2, v, &vote("x"))
            .await
            .map(|_| ())
    ));
    assert!(introuvable(
        ent.corriger_grille_for_actor(
            etab2,
            ud2,
            rd2,
            v,
            &CorrectionGrilleBody {
                motif: "x".into(),
                effet_retroactif: false
            }
        )
        .await
        .map(|_| ())
    ));
    assert!(introuvable(
        ent.publier_grille_for_actor(etab2, ud2, rd2, v)
            .await
            .map(|_| ())
    ));
    assert!(introuvable(
        ent.controles_grille_for_actor(etab2, u2, r2, v)
            .await
            .map(|_| ())
    ));
    assert!(introuvable(
        ent.qualifier_article_for_actor(
            etab2,
            u2,
            r2,
            contrib,
            &QualifierArticleBody {
                nature_legale: "internat".into(),
                regime_tva: "hors_champ".into(),
                taux_tva_bps: None,
                unite: "annee".into(),
                mode: None,
                remboursable: None,
                soumis_reductions: None,
                soumis_remise_ordre: None
            }
        )
        .await
        .map(|_| ())
    ));
    assert!(introuvable(
        ent.calculer_simulation_for_actor(etab2, u2, r2, sim.simulation_id)
            .await
            .map(|_| ())
    ));
    assert!(introuvable(
        ent.exporter_conseil_for_actor(etab2, u2, r2, sim.simulation_id)
            .await
            .map(|_| ())
    ));
    assert!(introuvable(
        ent.desactiver_famille_type_for_actor(etab2, u2, r2, fam.id)
            .await
            .map(|_| ())
    ));
    assert!(introuvable(
        ent.creer_simulation_for_actor(
            etab2,
            u2,
            r2,
            &SimulationBody {
                annee_scolaire: ANNEE.into(),
                libelle: "S".into(),
                base_version_id: v,
                regles: vec![],
                effectifs: vec![]
            }
        )
        .await
        .map(|_| ())
    ));
    assert!(
        ent.grilles_for_actor(etab2, u2, r2, None)
            .await
            .unwrap()
            .is_empty(),
        "le second établissement ne voit aucune grille"
    );
    assert!(ent
        .simulations_for_actor(etab2, u2, r2)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(avant, compte(&ent).await, "aucun effet en base");
}

// ───────────────────────── Correctifs de revue (D-069) ─────────────────────────

fn tarif_historique(article: Uuid, montant: i64) -> TarifBody {
    TarifBody {
        annee_scolaire: ANNEE.into(),
        article_id: article,
        niveau: None,
        regime: None,
        qf_min_centimes: None,
        qf_max_centimes: None,
        montant_annuel_centimes: montant,
    }
}

async fn total_scolarite(ent: &PgEnt, eleve: Uuid) -> i64 {
    let (t,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(sum(l.montant_unitaire_centimes * l.quantite), 0)::bigint FROM ent_fac_ligne l
          WHERE l.eleve_id = $1 AND l.categorie = 'scolarite'",
    )
    .bind(eleve)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    t
}

/// Direction (créatrice) rédige, comptabilité propose ; renvoie la version proposée.
async fn grille_par_direction_proposee_par_compta(
    ent: &PgEnt,
    c: &LaboCast,
    copier_de: Option<Uuid>,
    reprendre: bool,
    montant: Option<i64>,
    article: Uuid,
) -> Uuid {
    let (ud, rd) = direction(c);
    let (u, r) = compta(c);
    let etab = c.etablissement_id;
    let v = ent
        .creer_grille_for_actor(
            etab,
            ud,
            rd,
            &CreerGrilleBody {
                annee_scolaire: ANNEE.into(),
                libelle: None,
                copier_de,
                reprendre_historique: Some(reprendre),
            },
        )
        .await
        .unwrap();
    if let Some(m) = montant {
        ent.ajouter_tarif_grille_for_actor(etab, ud, rd, v.id, &tarif_body(article, m, true))
            .await
            .unwrap();
    }
    ent.proposer_grille_for_actor(etab, u, r, v.id)
        .await
        .unwrap();
    v.id
}

/// M1 (règle 2) : une grille ne tarifie qu'à partir de sa date de vigueur ; la précédente reste applicable
/// jusqu'à la veille ; vote jamais dans le futur ; vigueur jamais avant le vote ni hors de l'année.
#[tokio::test]
async fn f3r_m1_grille_tarifie_a_partir_de_sa_date_de_vigueur() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ud, rd) = direction(&c);
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
    ent.creer_tarif_for_actor(etab, u, r, &tarif_historique(contrib.id, 100_000))
        .await
        .unwrap();
    // v1 : reprise de l'historique (1 000 €), +200 €, votée le 20 juin, en vigueur le 1er septembre.
    let v1 = grille_par_direction_proposee_par_compta(&ent, &c, None, true, None, contrib.id).await;
    // (la reprise s'est faite à la création ; la hausse se fait en brouillon : on repart d'un brouillon)
    let _ = v1;
    let b1 = ent
        .creer_grille_for_actor(
            etab,
            ud,
            rd,
            &CreerGrilleBody {
                annee_scolaire: ANNEE.into(),
                libelle: None,
                copier_de: None,
                reprendre_historique: Some(true),
            },
        )
        .await
        .unwrap();
    ent.appliquer_hausse_for_actor(
        etab,
        ud,
        rd,
        b1.id,
        &HausseBody {
            mode: ModeHausse::Montant,
            valeur: 20_000,
            arrondi: Arrondi::Aucun,
            article_id: None,
            apercu: false,
        },
    )
    .await
    .unwrap();
    ent.proposer_grille_for_actor(etab, u, r, b1.id).await.unwrap();
    let (ua, ra) = (c.admin.user_id, Role::Admin.as_str());
    // vote daté dans le futur (jour de référence : 1er juillet 2030) : refusé
    assert!(invalide(
        ent.voter_grille_test(
            etab,
            ua,
            ra,
            b1.id,
            &VoteGrilleBody {
                vote_le: d(2030, 7, 2),
                reference: "CA".into(),
                document_id: None
            }
        )
        .await
    ));
    ent.voter_grille_test(etab, ua, ra, b1.id, &vote("CA du 20 juin"))
        .await
        .unwrap();
    // vigueur hors de l'année scolaire visée : refusée
    assert!(invalide(
        ent.mettre_en_vigueur_for_actor(etab, u, r, b1.id, &MiseEnVigueurBody { du: d(2030, 8, 31) })
            .await
    ));
    assert!(invalide(
        ent.mettre_en_vigueur_for_actor(etab, u, r, b1.id, &MiseEnVigueurBody { du: d(2031, 9, 1) })
            .await
    ));
    ent.mettre_en_vigueur_for_actor(etab, u, r, b1.id, &MiseEnVigueurBody { du: d(2030, 9, 1) })
        .await
        .unwrap();
    // v2 : correction de v1 (+100 €), votée le 10 décembre, en vigueur le 1er janvier.
    let v2 = ent
        .corriger_grille_for_actor(
            etab,
            ud,
            rd,
            b1.id,
            &CorrectionGrilleBody {
                motif: "revalorisation".into(),
                effet_retroactif: false,
            },
        )
        .await
        .unwrap();
    ent.appliquer_hausse_for_actor(
        etab,
        ud,
        rd,
        v2.id,
        &HausseBody {
            mode: ModeHausse::Montant,
            valeur: 10_000,
            arrondi: Arrondi::Aucun,
            article_id: None,
            apercu: false,
        },
    )
    .await
    .unwrap();
    ent.proposer_grille_for_actor(etab, u, r, v2.id).await.unwrap();
    ent.voter_grille_au_jour(
        etab,
        ua,
        ra,
        v2.id,
        &VoteGrilleBody {
            vote_le: d(2030, 12, 10),
            reference: "CA du 10 décembre".into(),
            document_id: None,
        },
        d(2030, 12, 20),
    )
    .await
    .unwrap();
    // vigueur avant le vote : refusée ; avant la date de la version appliquée : refusée
    assert!(invalide(
        ent.mettre_en_vigueur_for_actor(etab, u, r, v2.id, &MiseEnVigueurBody { du: d(2030, 12, 5) })
            .await
    ));
    ent.mettre_en_vigueur_for_actor(etab, u, r, v2.id, &MiseEnVigueurBody { du: d(2031, 1, 1) })
        .await
        .unwrap();
    let statut_v1: (String,) = sqlx::query_as("SELECT statut FROM ent_fin_grille_version WHERE id = $1")
        .bind(b1.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(statut_v1.0, "close");
    // Génération : juillet → historique (1 000 €) ; septembre et décembre → v1 (1 200 €) ; janvier → v2 (1 300 €).
    let mut attendu = Vec::new();
    for (prenom, jour, montant) in [
        ("Jul", d(2030, 7, 15), 100_000),
        ("Sep", d(2030, 9, 1), 120_000),
        ("Dec", d(2030, 12, 31), 120_000),
        ("Jan", d(2031, 1, 1), 130_000),
    ] {
        let el = eleve(&ent, etab, prenom, "externe").await;
        let _ = foyer_payeur(&ent, etab, el, c.parent_lea.user_id, &format!("Foyer {prenom}")).await;
        facturer_au(&ent, etab, el, jour).await;
        attendu.push((el, montant));
    }
    for (el, montant) in attendu {
        assert_eq!(total_scolarite(&ent, el).await, montant);
    }
}

/// M2 (règle 3) : le second regard est une autre personne que l'auteur réel ; il ne se désactive pas pendant
/// qu'une grille est proposée ; sa désactivation exige `voter` et est signalée.
#[tokio::test]
async fn f3r_m2_second_regard_autre_personne_que_l_auteur() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ud, rd) = direction(&c);
    let (ua, ra) = (c.admin.user_id, Role::Admin.as_str());
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
    // direction rédige, comptabilité propose, direction vote → refus ; une autre personne vote.
    let v = grille_par_direction_proposee_par_compta(&ent, &c, None, false, Some(120_000), contrib.id).await;
    assert!(interdit(
        ent.voter_grille_test(etab, ud, rd, v, &vote("CA")).await
    ));
    let (statut,): (String,) = sqlx::query_as("SELECT statut FROM ent_fin_grille_version WHERE id = $1")
        .bind(v)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(statut, "proposee");
    // même chose vérifiée en base : un vote écrit directement par l'auteur est refusé
    assert!(sqlx::query(
        "UPDATE ent_fin_grille_version SET statut = 'votee', vote_le = '2030-06-20', vote_reference = 'x',
                vote_par = $2 WHERE id = $1"
    )
    .bind(v)
    .bind(ud)
    .execute(ent.pool_verif())
    .await
    .is_err());
    // le second regard ne se désactive pas pendant qu'une grille est proposée
    assert!(conflit(
        ent.definir_reglages_grilles_for_actor(
            etab,
            ud,
            rd,
            &ReglagesGrillesBody {
                exiger_qualification_articles: None,
                grille_second_regard: Some(false)
            }
        )
        .await
    ));
    assert!(sqlx::query(
        "INSERT INTO ent_fin_parametres (etablissement_id, grille_second_regard) VALUES ($1, false)
         ON CONFLICT (etablissement_id) DO UPDATE SET grille_second_regard = false"
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .is_err());
    ent.voter_grille_test(etab, ua, ra, v, &vote("CA")).await.unwrap();
    // la désactivation exige `voter` en plus de `parametrer` : la comptabilité est refusée
    assert!(refuse(
        ent.definir_reglages_grilles_for_actor(
            etab,
            u,
            r,
            &ReglagesGrillesBody {
                exiger_qualification_articles: None,
                grille_second_regard: Some(false)
            }
        )
        .await
    ));
    // direction (parametrer + voter) : possible, journalisée et signalée
    let apres = ent
        .definir_reglages_grilles_for_actor(
            etab,
            ud,
            rd,
            &ReglagesGrillesBody {
                exiger_qualification_articles: None,
                grille_second_regard: Some(false),
            },
        )
        .await
        .unwrap();
    assert!(!apres.grille_second_regard);
    assert!(apres.alertes.iter().any(|a| a.contains("second regard")));
    assert_eq!(
        journal_contient(&ent, etab, "grille.second_regard_desactive").await,
        1
    );
}

/// M3 : ce qui est voté est figé en base, colonne par colonne, sans échappatoire par la transition.
#[tokio::test]
async fn f3r_m3_version_figee_par_la_base() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ua, ra) = (c.admin.user_id, Role::Admin.as_str());
    let (v, _, _) = grille_proposee(&ent, &c).await;
    let pool = ent.pool_verif();
    let echoue = |sql: &'static str| {
        let id = v;
        async move {
            sqlx::query(sql).bind(id).execute(pool).await.is_err()
        }
    };
    // INSERT : une version naît brouillon, vierge de toute transition
    assert!(
        sqlx::query(
            "INSERT INTO ent_fin_grille_version (id, etablissement_id, annee_scolaire, numero, libelle, statut,
                    vote_le, vote_reference, vote_par)
             VALUES (gen_random_uuid(), $1, '2031-2032', 1, 'x', 'votee', '2031-06-01', 'x', $2)"
        )
        .bind(etab)
        .bind(ua)
        .execute(pool)
        .await
        .is_err(),
        "insertion directe d'une version votée"
    );
    assert!(sqlx::query(
        "INSERT INTO ent_fin_grille_version (id, etablissement_id, annee_scolaire, numero, libelle, close_le)
         VALUES (gen_random_uuid(), $1, '2031-2032', 1, 'x', now())"
    )
    .bind(etab)
    .execute(pool)
    .await
    .is_err());
    // proposée : transition qui change aussi une autre colonne (ancienne échappatoire `RETURN NEW`)
    assert!(
        echoue("UPDATE ent_fin_grille_version SET statut = 'brouillon', libelle = 'autre' WHERE id = $1").await
    );
    assert!(
        echoue("UPDATE ent_fin_grille_version SET propose_le = now() WHERE id = $1").await,
        "date de proposition figée"
    );
    // vote par le service puis tentatives directes
    ent.voter_grille_test(etab, ua, ra, v, &vote("CA du 20 juin")).await.unwrap();
    assert!(
        echoue("UPDATE ent_fin_grille_version SET statut = 'en_vigueur', en_vigueur_du = '2030-09-01', en_vigueur_par = vote_par, libelle = 'autre' WHERE id = $1").await,
        "transition + autre colonne"
    );
    assert!(
        echoue("UPDATE ent_fin_grille_version SET statut = 'en_vigueur', en_vigueur_du = '2030-06-01', en_vigueur_par = vote_par WHERE id = $1").await,
        "vigueur avant le vote"
    );
    assert!(
        echoue("UPDATE ent_fin_grille_version SET statut = 'close', close_le = now(), vote_reference = 'autre' WHERE id = $1").await,
        "clôture + référence du vote"
    );
    assert!(
        echoue("UPDATE ent_fin_grille_version SET statut = 'close' WHERE id = $1").await,
        "clôture sans date"
    );
    ent.mettre_en_vigueur_for_actor(etab, u, r, v, &MiseEnVigueurBody { du: d(2030, 9, 1) })
        .await
        .unwrap();
    for (nom, sql) in [
        ("en_vigueur_du", "UPDATE ent_fin_grille_version SET en_vigueur_du = '2030-10-01' WHERE id = $1"),
        ("en_vigueur_par", "UPDATE ent_fin_grille_version SET en_vigueur_par = vote_par WHERE id = $1"),
        ("vote_le", "UPDATE ent_fin_grille_version SET vote_le = '2030-06-01' WHERE id = $1"),
        ("vote_par", "UPDATE ent_fin_grille_version SET vote_par = cree_par WHERE id = $1"),
        ("close_le posé avec la clôture + libellé", "UPDATE ent_fin_grille_version SET statut = 'close', close_le = now(), libelle = 'z' WHERE id = $1"),
        ("effet_retroactif", "UPDATE ent_fin_grille_version SET effet_retroactif = true WHERE id = $1"),
    ] {
        assert!(echoue(sql).await, "{nom} doit rester figé");
    }
    // clôture légitime, puis close_le figé une fois posé
    sqlx::query("UPDATE ent_fin_grille_version SET statut = 'close', close_le = now() WHERE id = $1")
        .bind(v)
        .execute(pool)
        .await
        .unwrap();
    assert!(
        echoue("UPDATE ent_fin_grille_version SET close_le = now() + interval '1 day' WHERE id = $1").await,
        "close_le figé"
    );
    assert!(echoue("DELETE FROM ent_fin_grille_version WHERE id = $1").await);
    assert!(sqlx::query("TRUNCATE ent_fin_grille_version").execute(pool).await.is_err());
}

async fn brouillon_vide(ent: &PgEnt, etab: Uuid, u: Uuid, r: &str) -> GrilleVersionDto {
    ent.creer_grille_for_actor(
        etab,
        u,
        r,
        &CreerGrilleBody {
            annee_scolaire: ANNEE.into(),
            libelle: None,
            copier_de: None,
            reprendre_historique: None,
        },
    )
    .await
    .unwrap()
}

/// M4 : les tranches ne se déplacent pas d'une version à l'autre ; l'ancienne version est contrôlée aussi.
#[tokio::test]
async fn f3r_m4_tranche_ne_change_ni_de_version_ni_d_etablissement() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let tranches = TranchesBody {
        tranches: vec![
            TrancheQfBody {
                code: "T1".into(),
                libelle: "bas".into(),
                qf_min_centimes: 0,
                qf_max_centimes: Some(50_000),
            },
            TrancheQfBody {
                code: "T2".into(),
                libelle: "haut".into(),
                qf_min_centimes: 50_000,
                qf_max_centimes: None,
            },
        ],
    };
    let figee = brouillon_vide(&ent, etab, u, r).await;
    let brouillon = brouillon_vide(&ent, etab, u, r).await;
    ent.definir_tranches_for_actor(etab, u, r, figee.id, &tranches).await.unwrap();
    ent.definir_tranches_for_actor(etab, u, r, brouillon.id, &tranches).await.unwrap();
    // la grille figée porte un tarif pour être proposable
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
    ent.ajouter_tarif_grille_for_actor(etab, u, r, figee.id, &tarif_body(contrib.id, 100_000, true))
        .await
        .unwrap();
    ent.proposer_grille_for_actor(etab, u, r, figee.id).await.unwrap();
    let pool = ent.pool_verif();
    // déplacement d'une tranche de la version figée vers un brouillon : refusé (OLD contrôlé)
    assert!(sqlx::query(
        "UPDATE ent_fin_tranche_qf SET grille_version_id = $2, code = 'D1'
          WHERE grille_version_id = $1 AND code = 'T1'"
    )
    .bind(figee.id)
    .bind(brouillon.id)
    .execute(pool)
    .await
    .is_err());
    // déplacement d'un brouillon vers la version figée : refusé
    assert!(sqlx::query(
        "UPDATE ent_fin_tranche_qf SET grille_version_id = $2, code = 'D2'
          WHERE grille_version_id = $1 AND code = 'T1'"
    )
    .bind(brouillon.id)
    .bind(figee.id)
    .execute(pool)
    .await
    .is_err());
    // même entre deux brouillons : la version ne change jamais
    let autre = brouillon_vide(&ent, etab, u, r).await;
    assert!(sqlx::query(
        "UPDATE ent_fin_tranche_qf SET grille_version_id = $2, code = 'D3'
          WHERE grille_version_id = $1 AND code = 'T1'"
    )
    .bind(brouillon.id)
    .bind(autre.id)
    .execute(pool)
    .await
    .is_err());
    assert!(sqlx::query(
        "UPDATE ent_fin_tranche_qf SET etablissement_id = gen_random_uuid() WHERE grille_version_id = $1"
    )
    .bind(brouillon.id)
    .execute(pool)
    .await
    .is_err());
    // un brouillon garde la modification légitime de son libellé
    sqlx::query("UPDATE ent_fin_tranche_qf SET libelle = 'très bas' WHERE grille_version_id = $1 AND code = 'T1'")
        .bind(brouillon.id)
        .execute(pool)
        .await
        .unwrap();
    // un tarif ne change ni de version ni d'établissement
    assert!(sqlx::query("UPDATE ent_fin_tarif SET etablissement_id = gen_random_uuid() WHERE grille_version_id = $1")
        .bind(figee.id)
        .execute(pool)
        .await
        .is_err());
    // F4 : le vidage de la table des tarifs est bloqué
    assert!(sqlx::query("TRUNCATE ent_fin_tarif").execute(pool).await.is_err());
}

/// M5 : à la mise en vigueur, les autres versions votées de l'année sont supplantées et ne s'activent plus.
#[tokio::test]
async fn f3r_m5_versions_votees_concurrentes_supplantees() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ud, rd) = direction(&c);
    let (ua, ra) = (c.admin.user_id, Role::Admin.as_str());
    let (v1, _, _) = grille_proposee(&ent, &c).await;
    ent.voter_grille_test(etab, ud, rd, v1, &vote("CA du 20 juin")).await.unwrap();
    // deux corrections concurrentes d'une même version : impossible (une seule en cours) ; on en vote une,
    // puis l'autre grille votée de l'année doit être supplantée.
    let c1 = ent
        .corriger_grille_for_actor(
            etab,
            ud,
            rd,
            v1,
            &CorrectionGrilleBody {
                motif: "ajustement".into(),
                effet_retroactif: false,
            },
        )
        .await
        .unwrap();
    ent.proposer_grille_for_actor(etab, u, r, c1.id).await.unwrap();
    ent.voter_grille_test(etab, ua, ra, c1.id, &vote("CA du 21 juin")).await.unwrap();
    // v1 et c1 sont toutes deux votées
    ent.mettre_en_vigueur_for_actor(etab, u, r, c1.id, &MiseEnVigueurBody { du: d(2030, 9, 1) })
        .await
        .unwrap();
    let (statut,): (String,) = sqlx::query_as("SELECT statut FROM ent_fin_grille_version WHERE id = $1")
        .bind(v1)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(statut, "supplantee");
    // elle ne se met plus en vigueur, ni par le service, ni par la base
    assert!(conflit(
        ent.mettre_en_vigueur_for_actor(etab, u, r, v1, &MiseEnVigueurBody { du: d(2030, 10, 1) })
            .await
    ));
    assert!(sqlx::query("UPDATE ent_fin_grille_version SET statut = 'en_vigueur', en_vigueur_du = '2030-10-01', en_vigueur_par = vote_par WHERE id = $1")
        .bind(v1)
        .execute(ent.pool_verif())
        .await
        .is_err());
}

/// M6 (règle 4) : un article tarifé par l'historique ou la version précédente ne disparaît pas en silence.
#[tokio::test]
async fn f3r_m6_article_oublie_bloque_la_proposition() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
    let dp = article_modele(&ent, &c, "DEMI_PENSION", "DP").await;
    ent.creer_tarif_for_actor(etab, u, r, &tarif_historique(contrib.id, 100_000)).await.unwrap();
    ent.creer_tarif_for_actor(etab, u, r, &tarif_historique(dp.id, 40_000)).await.unwrap();
    let v = ent
        .creer_grille_for_actor(
            etab,
            u,
            r,
            &CreerGrilleBody {
                annee_scolaire: ANNEE.into(),
                libelle: None,
                copier_de: None,
                reprendre_historique: None,
            },
        )
        .await
        .unwrap();
    ent.ajouter_tarif_grille_for_actor(etab, u, r, v.id, &tarif_body(contrib.id, 120_000, true))
        .await
        .unwrap();
    // la demi-pension de l'historique est oubliée : blocage
    let ctrl = ent.controles_grille_for_actor(etab, u, r, v.id).await.unwrap();
    assert!(!ctrl.proposable);
    assert!(ctrl.anomalies.iter().any(|a| a.code == "article_oublie" && a.bloquante));
    assert!(conflit(ent.proposer_grille_for_actor(etab, u, r, v.id).await));
    ent.ajouter_tarif_grille_for_actor(etab, u, r, v.id, &tarif_body(dp.id, 45_000, true))
        .await
        .unwrap();
    assert!(ent.controles_grille_for_actor(etab, u, r, v.id).await.unwrap().proposable);
}

/// M7 : qualification exigée par défaut (ligne de paramètres absente) ; transitoire faux avec alerte ;
/// une ligne sans article compte comme non qualifiée ; un modèle taxable reste « à valider ».
#[tokio::test]
async fn f3r_m7_qualification_exigee_par_defaut() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ud, rd) = direction(&c);
    sqlx::query("DELETE FROM ent_fin_parametres WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let reglages = ent.reglages_grilles_for_actor(etab, u, r).await.unwrap();
    assert!(reglages.exiger_qualification_articles, "ligne absente : exigée");
    assert!(reglages.alertes.is_empty());
    // le défaut de la colonne est « vrai » pour un établissement créé après la 523
    sqlx::query("INSERT INTO ent_fin_parametres (etablissement_id) VALUES ($1)")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert!(ent.reglages_grilles_for_actor(etab, u, r).await.unwrap().exiger_qualification_articles);
    // transitoire (établissement antérieur) : faux, avec l'alerte à la direction
    sqlx::query("UPDATE ent_fin_parametres SET exiger_qualification_articles = false WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let transitoire = ent.reglages_grilles_for_actor(etab, ud, rd).await.unwrap();
    assert!(!transitoire.exiger_qualification_articles);
    assert!(transitoire.alertes.iter().any(|a| a.contains("2027")));
    ent.definir_reglages_grilles_for_actor(
        etab,
        u,
        r,
        &ReglagesGrillesBody {
            exiger_qualification_articles: Some(true),
            grille_second_regard: None,
        },
    )
    .await
    .unwrap();
    // un modèle taxable n'est pas qualifié d'emblée ; un modèle exonéré, si
    let loc = article_modele(&ent, &c, "LOCATION_SALLE", "LOC").await;
    assert!(!loc.qualifie && loc.a_valider, "taxable : à valider");
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
    assert!(contrib.qualifie && !contrib.a_valider);
    // une ligne sans article ne passe pas l'émission quand la qualification est exigée
    let el = eleve(&ent, etab, "Sans", "externe").await;
    let _ = foyer_payeur(&ent, etab, el, c.parent_lea.user_id, "Foyer sans article").await;
    ent.creer_tarif_for_actor(etab, u, r, &tarif_historique(contrib.id, 100_000)).await.unwrap();
    facturer(&ent, etab, el).await;
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_fac_ligne l WHERE l.eleve_id = $1 AND l.article_id IS NOT NULL",
    )
    .bind(el)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(n > 0);
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let (fid,): (Uuid,) = sqlx::query_as(
        "SELECT l.facture_id FROM ent_fac_ligne l WHERE l.eleve_id = $1 LIMIT 1",
    )
    .bind(el)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE ent_fac_ligne SET article_id = NULL WHERE facture_id = $1")
        .bind(fid)
        .execute(&mut *tx)
        .await
        .unwrap();
    let non_qualifiees =
        crate::facturation_grilles::articles_non_qualifies_de_piece_tx(&mut tx, etab, fid)
            .await
            .unwrap();
    assert!(non_qualifiees > 0, "une ligne sans article compte comme non qualifiée");
    tx.rollback().await.unwrap();
}

/// F1, F4, F5 : clé composite de l'article d'une règle, historique des recalculs, plafonds et suppression,
/// refus (422) plutôt que troncature.
#[tokio::test]
async fn f3r_f1_f4_f5_simulation_historique_plafond_et_suppression() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (v, contrib, _) = grille_proposee(&ent, &c).await;
    let sim = ent
        .creer_simulation_for_actor(
            etab,
            u,
            r,
            &SimulationBody {
                annee_scolaire: ANNEE.into(),
                libelle: "hausse 3 %".into(),
                base_version_id: v,
                regles: vec![RegleHausseBody {
                    article_id: Some(contrib),
                    mode: ModeHausse::Pourcentage,
                    valeur: 300,
                    arrondi: Arrondi::Euro,
                }],
                effectifs: vec![EffectifPrevuBody {
                    niveau: "6EME".into(),
                    regime: "externe".into(),
                    effectif: 10,
                }],
            },
        )
        .await
        .unwrap();
    // F4 : recalculer ajoute à l'historique, rien n'est écrasé en silence
    ent.calculer_simulation_for_actor(etab, u, r, sim.simulation_id).await.unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_fin_simulation_calcul WHERE simulation_id = $1")
        .bind(sim.simulation_id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 2);
    assert!(sqlx::query("UPDATE ent_fin_simulation_calcul SET produit_base_centimes = 0")
        .execute(ent.pool_verif())
        .await
        .is_err());
    // F1 : l'article d'une règle appartient à l'établissement (clé composite)
    let autre_etab = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_fin_article (id, etablissement_id, code, libelle, categorie, compte_produit) VALUES ($1, $2, 'AUTRE', 'autre', 'scolarite', '706000')")
        .bind(Uuid::new_v4())
        .bind(autre_etab)
        .execute(ent.pool_verif())
        .await
        .ok();
    let (art_autre,): (Option<Uuid>,) = sqlx::query_as("SELECT id FROM ent_fin_article WHERE etablissement_id = $1 LIMIT 1")
        .bind(autre_etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap_or((None,));
    if let Some(art) = art_autre {
        assert!(sqlx::query(
            "INSERT INTO ent_fin_simulation_regle (id, etablissement_id, simulation_id, article_id, hausse_mode, hausse_valeur)
             VALUES (gen_random_uuid(), $1, $2, $3, 'montant', 100)"
        )
        .bind(etab)
        .bind(sim.simulation_id)
        .bind(art)
        .execute(ent.pool_verif())
        .await
        .is_err());
    }
    // F5 : texte trop long refusé (422), jamais tronqué
    assert!(invalide(
        ent.article_depuis_modele_for_actor(
            etab,
            u,
            r,
            &ArticleDepuisModeleBody {
                modele_code: "DEMI_PENSION".into(),
                code: Some("C".repeat(31)),
                libelle: None,
                compte_produit: None,
            },
        )
        .await
        .map(|_| ())
    ));
    // suppression d'une simulation : ses règles, effectifs et calculs partent avec
    ent.supprimer_simulation_for_actor(etab, u, r, sim.simulation_id).await.unwrap();
    assert!(introuvable(ent.supprimer_simulation_for_actor(etab, u, r, sim.simulation_id).await));
    // familles types : plafond et suppression
    let fam = ent
        .creer_famille_type_for_actor(
            etab,
            u,
            r,
            &FamilleTypeBody {
                libelle: "Famille 1".into(),
                tranche_code: None,
                enfants: vec![EnfantTypeBody {
                    niveau: "6EME".into(),
                    regime: "externe".into(),
                }],
            },
        )
        .await
        .unwrap();
    ent.supprimer_famille_type_for_actor(etab, u, r, fam.id).await.unwrap();
    sqlx::query(
        "INSERT INTO ent_fin_famille_type (id, etablissement_id, libelle)
         SELECT gen_random_uuid(), $1, 'F' || g FROM generate_series(1, 200) g",
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    assert!(conflit(
        ent.creer_famille_type_for_actor(
            etab,
            u,
            r,
            &FamilleTypeBody {
                libelle: "Une de trop".into(),
                tranche_code: None,
                enfants: vec![EnfantTypeBody {
                    niveau: "6EME".into(),
                    regime: "externe".into(),
                }],
            },
        )
        .await
        .map(|_| ())
    ));
}
