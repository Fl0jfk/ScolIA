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

/// Erreur de base attendue : SQLSTATE exact et message (un échec pour une autre raison ne passe pas).
fn erreur_base(e: &sqlx::Error, sqlstate: &str, message: &str) {
    let db = e
        .as_database_error()
        .unwrap_or_else(|| panic!("erreur de base attendue, reçu : {e}"));
    assert_eq!(db.code().as_deref(), Some(sqlstate), "{e}");
    assert!(db.message().contains(message), "{e}");
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
    facturer_depuis(ent, etab, eleve, None).await;
}

/// Même génération pour un élève entré à `depuis` : le tarif se choisit **mois par mois**, au premier jour de chaque
/// mois dû (D-069, M-A) ; le jour de la génération n'intervient pas (donc aucune horloge dans les tests).
async fn facturer_depuis(ent: &PgEnt, etab: Uuid, eleve: Uuid, depuis: Option<NaiveDate>) {
    let mut tx = ent.pool_verif().begin().await.unwrap();
    crate::finances::facturer_annee_eleve(
        &mut tx,
        etab,
        ANNEE,
        eleve,
        depuis,
        None,
        Default::default(),
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
pub(crate) async fn etablissement_anterieur(ent: &PgEnt, etab: Uuid) {
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
        ent.voter_grille_test(etab, ud, rd, v, &vote("   ")).await
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
        interdit(ent.voter_grille_test(etab, ud, rd, v.id, &vote("CA")).await),
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
    // Les simulations ne s'effacent pas directement (historique des scénarios présentés) : la suppression passe par
    // le service, journalisée. Erreur attendue précisée (faible f) : un DELETE direct échoue pour la bonne raison.
    let e = sqlx::query("DELETE FROM ent_fin_simulation WHERE id = $1")
        .bind(res.simulation_id)
        .execute(ent.pool_verif())
        .await
        .unwrap_err();
    erreur_base(
        &e,
        "23503",
        "violates foreign key constraint \"ent_fin_simulation_regle_fk\"",
    );
    let e = sqlx::query("DELETE FROM ent_fin_famille_type WHERE id = $1")
        .bind(f_deux.id)
        .execute(ent.pool_verif())
        .await
        .unwrap_err();
    erreur_base(
        &e,
        "23503",
        "violates foreign key constraint \"ent_fin_famille_type_enfant_fk\"",
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
    ent.proposer_grille_for_actor(etab, u, r, b1.id)
        .await
        .unwrap();
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
        ent.mettre_en_vigueur_for_actor(
            etab,
            u,
            r,
            b1.id,
            &MiseEnVigueurBody { du: d(2030, 8, 31) }
        )
        .await
    ));
    assert!(invalide(
        ent.mettre_en_vigueur_for_actor(
            etab,
            u,
            r,
            b1.id,
            &MiseEnVigueurBody { du: d(2031, 9, 1) }
        )
        .await
    ));
    ent.mettre_en_vigueur_for_actor(etab, u, r, b1.id, &MiseEnVigueurBody { du: d(2030, 9, 1) })
        .await
        .unwrap();
    // Contre-revue du pilote (Élevé 1) : un élève facturé AVANT la mise en vigueur de v2 (une seule ligne, v1 sur
    // l'année entière). La génération rejouée après v2 ne doit rien refacturer (sinon janvier-juin deux fois).
    let avant = eleve(&ent, etab, "Avant", "externe").await;
    let _ = foyer_payeur(&ent, etab, avant, c.parent_lea.user_id, "Foyer Avant").await;
    facturer_depuis(&ent, etab, avant, None).await;
    assert_eq!(total_scolarite(&ent, avant).await, 120_000);
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
    ent.proposer_grille_for_actor(etab, u, r, v2.id)
        .await
        .unwrap();
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
        ent.mettre_en_vigueur_for_actor(
            etab,
            u,
            r,
            v2.id,
            &MiseEnVigueurBody { du: d(2030, 12, 5) }
        )
        .await
    ));
    ent.mettre_en_vigueur_for_actor(etab, u, r, v2.id, &MiseEnVigueurBody { du: d(2031, 1, 1) })
        .await
        .unwrap();
    let statut_v1: (String,) =
        sqlx::query_as("SELECT statut FROM ent_fin_grille_version WHERE id = $1")
            .bind(b1.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(statut_v1.0, "close");
    // Génération mois par mois (M-A, D-069) : le tarif est celui du premier jour de chaque mois dû.
    // Année facturée de septembre à juin (10 mois) : v1 (1 200 €) de septembre à décembre, v2 (1 300 €) ensuite.
    //   - année entière : 1 200 × 4/10 + 1 300 × 6/10 = 480 + 780 = 1 260 € ;
    //   - entré en novembre : 1 200 × 2/10 + 780 = 240 + 780 = 1 020 € ;
    //   - entré en janvier : 780 € (tout au tarif v2).
    let mut attendu = Vec::new();
    for (prenom, depuis, montant) in [
        ("Ann", None, 126_000),
        ("Sep", Some(d(2030, 9, 1)), 126_000),
        ("Nov", Some(d(2030, 11, 15)), 102_000),
        ("Jan", Some(d(2031, 1, 1)), 78_000),
    ] {
        let el = eleve(&ent, etab, prenom, "externe").await;
        let _ = foyer_payeur(
            &ent,
            etab,
            el,
            c.parent_lea.user_id,
            &format!("Foyer {prenom}"),
        )
        .await;
        facturer_depuis(&ent, etab, el, depuis).await;
        attendu.push((el, montant));
    }
    let premier = attendu[0].0;
    for (el, montant) in attendu {
        assert_eq!(total_scolarite(&ent, el).await, montant);
    }
    // Rejouer la génération ne double rien (clés de source stables par segment).
    let (rejoue,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_fac_ligne WHERE eleve_id = $1 AND categorie = 'scolarite'",
    )
    .bind(premier)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(rejoue, 2, "deux segments : deux lignes");
    // Rejeu réel : la même génération, une seconde fois, ne crée rien.
    facturer_depuis(&ent, etab, premier, None).await;
    let (rejoue,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_fac_ligne WHERE eleve_id = $1 AND categorie = 'scolarite'",
    )
    .bind(premier)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(rejoue, 2, "le rejeu ne double rien");
    assert_eq!(total_scolarite(&ent, premier).await, 126_000);
    // Élève facturé avant v2 : la génération après v2 est refusée (anomalie), rien n'est écrit ni doublé.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let refus = crate::finances::facturer_annee_eleve(
        &mut tx,
        etab,
        ANNEE,
        avant,
        None,
        None,
        Default::default(),
    )
    .await
    .unwrap()
    .expect_err("tarif changé depuis la génération : anomalie");
    tx.commit().await.unwrap();
    assert!(
        refus.contains("le tarif a changé depuis la génération précédente"),
        "{refus}"
    );
    assert_eq!(
        total_scolarite(&ent, avant).await,
        120_000,
        "aucun mois facturé deux fois"
    );

    // M-A : la liste des tarifs accepte une date (jamais l'horloge réelle dans ce test).
    let montants_au = |jour: NaiveDate| {
        let ent = &ent;
        async move {
            let rows = ent
                .tarifs_for_actor(etab, u, r, Some(ANNEE.into()), Some(jour))
                .await
                .unwrap();
            rows.iter()
                .filter(|t| t.article_id == contrib.id)
                .map(|t| t.montant_annuel_centimes)
                .collect::<Vec<_>>()
        }
    };
    assert_eq!(
        montants_au(d(2030, 8, 31)).await,
        vec![100_000],
        "la veille de v1 : historique"
    );
    assert_eq!(
        montants_au(d(2030, 9, 1)).await,
        vec![120_000],
        "v1 le 1er septembre"
    );
    assert_eq!(
        montants_au(d(2030, 12, 31)).await,
        vec![120_000],
        "v1 la veille de v2"
    );
    assert_eq!(
        montants_au(d(2031, 1, 1)).await,
        vec![130_000],
        "v2 le 1er janvier"
    );

    // M-B : une version votée mise en vigueur le même jour qu'une version déjà appliquée est refusée.
    let v3 = ent
        .corriger_grille_for_actor(
            etab,
            ud,
            rd,
            v2.id,
            &CorrectionGrilleBody {
                motif: "essai du même jour".into(),
                effet_retroactif: false,
            },
        )
        .await
        .unwrap();
    ent.proposer_grille_for_actor(etab, u, r, v3.id)
        .await
        .unwrap();
    ent.voter_grille_au_jour(
        etab,
        ua,
        ra,
        v3.id,
        &VoteGrilleBody {
            vote_le: d(2030, 12, 11),
            reference: "CA du 11 décembre".into(),
            document_id: None,
        },
        d(2030, 12, 20),
    )
    .await
    .unwrap();
    assert!(invalide(
        ent.mettre_en_vigueur_for_actor(
            etab,
            u,
            r,
            v3.id,
            &MiseEnVigueurBody { du: d(2031, 1, 1) }
        )
        .await
    ));
    // … et la base le refuse aussi (déclencheur), SQLSTATE 23514, sans passer par le service
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let e = sqlx::query(
        "UPDATE ent_fin_grille_version SET statut = 'en_vigueur', en_vigueur_du = $2, en_vigueur_par = $3 WHERE id = $1",
    )
    .bind(v3.id)
    .bind(d(2031, 1, 1))
    .bind(ua)
    .execute(&mut *tx)
    .await
    .unwrap_err();
    erreur_base(
        &e,
        "23514",
        "antérieure ou égale à celle d'une version déjà appliquée",
    );
    drop(tx);
    // un jour plus tard : accepté
    ent.mettre_en_vigueur_for_actor(etab, u, r, v3.id, &MiseEnVigueurBody { du: d(2031, 1, 2) })
        .await
        .unwrap();
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
    let v =
        grille_par_direction_proposee_par_compta(&ent, &c, None, false, Some(120_000), contrib.id)
            .await;
    assert!(interdit(
        ent.voter_grille_test(etab, ud, rd, v, &vote("CA")).await
    ));
    let (statut,): (String,) =
        sqlx::query_as("SELECT statut FROM ent_fin_grille_version WHERE id = $1")
            .bind(v)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(statut, "proposee");
    // même chose vérifiée en base : un vote écrit directement par l'auteur est refusé
    let e = sqlx::query(
        "UPDATE ent_fin_grille_version SET statut = 'votee', vote_le = '2030-06-20', vote_reference = 'x',
                vote_par = $2 WHERE id = $1",
    )
    .bind(v)
    .bind(ud)
    .execute(ent.pool_verif())
    .await
    .unwrap_err();
    erreur_base(
        &e,
        "23001",
        "second regard : le votant est l'auteur de la grille",
    );
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
    let e = sqlx::query(
        "INSERT INTO ent_fin_parametres (etablissement_id, grille_second_regard) VALUES ($1, false)
         ON CONFLICT (etablissement_id) DO UPDATE SET grille_second_regard = false",
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .unwrap_err();
    erreur_base(
        &e,
        "23001",
        "second regard : désactivation interdite pendant qu'une grille est proposée",
    );
    ent.voter_grille_test(etab, ua, ra, v, &vote("CA"))
        .await
        .unwrap();
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
        async move { sqlx::query(sql).bind(id).execute(pool).await.is_err() }
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
    ent.voter_grille_test(etab, ua, ra, v, &vote("CA du 20 juin"))
        .await
        .unwrap();
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
    sqlx::query(
        "UPDATE ent_fin_grille_version SET statut = 'close', close_le = now() WHERE id = $1",
    )
    .bind(v)
    .execute(pool)
    .await
    .unwrap();
    assert!(
        echoue(
            "UPDATE ent_fin_grille_version SET close_le = now() + interval '1 day' WHERE id = $1"
        )
        .await,
        "close_le figé"
    );
    assert!(echoue("DELETE FROM ent_fin_grille_version WHERE id = $1").await);
    assert!(sqlx::query("TRUNCATE ent_fin_grille_version")
        .execute(pool)
        .await
        .is_err());
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
    ent.definir_tranches_for_actor(etab, u, r, figee.id, &tranches)
        .await
        .unwrap();
    ent.definir_tranches_for_actor(etab, u, r, brouillon.id, &tranches)
        .await
        .unwrap();
    // la grille figée porte un tarif pour être proposable
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
    ent.ajouter_tarif_grille_for_actor(
        etab,
        u,
        r,
        figee.id,
        &tarif_body(contrib.id, 100_000, true),
    )
    .await
    .unwrap();
    ent.proposer_grille_for_actor(etab, u, r, figee.id)
        .await
        .unwrap();
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
    assert!(sqlx::query("TRUNCATE ent_fin_tarif")
        .execute(pool)
        .await
        .is_err());
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
    ent.voter_grille_test(etab, ud, rd, v1, &vote("CA du 20 juin"))
        .await
        .unwrap();
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
    ent.proposer_grille_for_actor(etab, u, r, c1.id)
        .await
        .unwrap();
    ent.voter_grille_test(etab, ua, ra, c1.id, &vote("CA du 21 juin"))
        .await
        .unwrap();
    // v1 et c1 sont toutes deux votées
    ent.mettre_en_vigueur_for_actor(etab, u, r, c1.id, &MiseEnVigueurBody { du: d(2030, 9, 1) })
        .await
        .unwrap();
    let (statut,): (String,) =
        sqlx::query_as("SELECT statut FROM ent_fin_grille_version WHERE id = $1")
            .bind(v1)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(statut, "supplantee");
    // faible 8 : le journal garde les identifiants des versions supplantées, pas seulement leur nombre
    let (detail,): (String,) = sqlx::query_as(
        "SELECT detail FROM ent_fin_journal WHERE etablissement_id = $1 AND action = 'grille.en_vigueur'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(detail.contains(&v1.to_string()), "{detail}");
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
    ent.creer_tarif_for_actor(etab, u, r, &tarif_historique(contrib.id, 100_000))
        .await
        .unwrap();
    ent.creer_tarif_for_actor(etab, u, r, &tarif_historique(dp.id, 40_000))
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
    ent.ajouter_tarif_grille_for_actor(etab, u, r, v.id, &tarif_body(contrib.id, 120_000, true))
        .await
        .unwrap();
    // la demi-pension de l'historique est oubliée : blocage
    let ctrl = ent
        .controles_grille_for_actor(etab, u, r, v.id)
        .await
        .unwrap();
    assert!(!ctrl.proposable);
    assert!(ctrl
        .anomalies
        .iter()
        .any(|a| a.code == "article_oublie" && a.bloquante));
    assert!(conflit(
        ent.proposer_grille_for_actor(etab, u, r, v.id).await
    ));
    ent.ajouter_tarif_grille_for_actor(etab, u, r, v.id, &tarif_body(dp.id, 45_000, true))
        .await
        .unwrap();
    assert!(
        ent.controles_grille_for_actor(etab, u, r, v.id)
            .await
            .unwrap()
            .proposable
    );
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
    assert!(
        reglages.exiger_qualification_articles,
        "ligne absente : exigée"
    );
    assert!(reglages.alertes.is_empty());
    // le défaut de la colonne est « vrai » pour un établissement créé après la 523
    sqlx::query("INSERT INTO ent_fin_parametres (etablissement_id) VALUES ($1)")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert!(
        ent.reglages_grilles_for_actor(etab, u, r)
            .await
            .unwrap()
            .exiger_qualification_articles
    );
    // transitoire (établissement antérieur) : faux, avec l'alerte à la direction
    sqlx::query("UPDATE ent_fin_parametres SET exiger_qualification_articles = false WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let transitoire = ent.reglages_grilles_for_actor(etab, ud, rd).await.unwrap();
    assert!(!transitoire.exiger_qualification_articles);
    assert!(transitoire
        .alertes
        .iter()
        .any(|a| a.contains("qualification des articles non exigée") && !a.contains("2027")));
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
    ent.creer_tarif_for_actor(etab, u, r, &tarif_historique(contrib.id, 100_000))
        .await
        .unwrap();
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
    let (fid,): (Uuid,) =
        sqlx::query_as("SELECT l.facture_id FROM ent_fac_ligne l WHERE l.eleve_id = $1 LIMIT 1")
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
    assert!(
        non_qualifiees > 0,
        "une ligne sans article compte comme non qualifiée"
    );
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
    ent.calculer_simulation_for_actor(etab, u, r, sim.simulation_id)
        .await
        .unwrap();
    let (n,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM ent_fin_simulation_calcul WHERE simulation_id = $1")
            .bind(sim.simulation_id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(n, 2);
    assert!(
        sqlx::query("UPDATE ent_fin_simulation_calcul SET produit_base_centimes = 0")
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // F1 : l'article d'une règle appartient à l'établissement (clé composite)
    let autre_etab = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_fin_article (id, etablissement_id, code, libelle, categorie, compte_produit) VALUES ($1, $2, 'AUTRE', 'autre', 'scolarite', '706000')")
        .bind(Uuid::new_v4())
        .bind(autre_etab)
        .execute(ent.pool_verif())
        .await
        .ok();
    let (art_autre,): (Option<Uuid>,) =
        sqlx::query_as("SELECT id FROM ent_fin_article WHERE etablissement_id = $1 LIMIT 1")
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
    ent.supprimer_simulation_for_actor(etab, u, r, sim.simulation_id)
        .await
        .unwrap();
    assert!(introuvable(
        ent.supprimer_simulation_for_actor(etab, u, r, sim.simulation_id)
            .await
    ));
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
    ent.supprimer_famille_type_for_actor(etab, u, r, fam.id)
        .await
        .unwrap();
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

// ───────────────────────── Suites de la contre-revue (D-069 : M-A à M-D, faibles 1 à 12) ─────────────────────────

/// M-A, scénario 1 : rentrée votée en juin pour une grille en vigueur au 1er septembre, sans aucun tarif
/// historique : l'élève est facturé (le jour de génération n'intervient pas).
#[tokio::test]
async fn f3s_ma_rentree_votee_en_juin_pour_une_grille_du_premier_septembre() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ua, ra) = (c.admin.user_id, Role::Admin.as_str());
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
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
    ent.proposer_grille_for_actor(etab, u, r, v.id)
        .await
        .unwrap();
    // vote du 20 juin 2030 (jour de référence : 1er juillet) ; vigueur au 1er septembre
    ent.voter_grille_test(etab, ua, ra, v.id, &vote("CA du 20 juin"))
        .await
        .unwrap();
    ent.mettre_en_vigueur_for_actor(etab, u, r, v.id, &MiseEnVigueurBody { du: d(2030, 9, 1) })
        .await
        .unwrap();
    let el = eleve(&ent, etab, "Rentree", "externe").await;
    let _ = foyer_payeur(&ent, etab, el, c.parent_lea.user_id, "Foyer rentrée").await;
    // génération « en juin » : aucune version applicable ce jour-là, mais chaque mois dû (septembre à juin)
    // est tarifé au 1er de son mois : l'élève est facturé, pas de « aucun tarif applicable »
    facturer(&ent, etab, el).await;
    assert_eq!(total_scolarite(&ent, el).await, 120_000);
}

/// M-C : le réglage du second regard et la proposition d'une grille se sérialisent par un verrou d'établissement
/// (deux connexions, ordre imposé par le verrou lui-même, sans `sleep`).
#[tokio::test]
async fn f3s_mc_verrou_proposition_et_second_regard() {
    let Some((ent, c)) = labo().await else { return };
    let ent = std::sync::Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ud, rd) = direction(&c);
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
    let brouillon = |montant: i64| {
        let ent = ent.clone();
        async move {
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
            ent.ajouter_tarif_grille_for_actor(
                etab,
                u,
                r,
                v.id,
                &tarif_body(contrib.id, montant, true),
            )
            .await
            .unwrap();
            v.id
        }
    };
    let cle = crate::facturation_grilles::cle_verrou_second_regard(etab);

    // 1. La désactivation attend le verrou ; pendant ce temps une proposition est validée : au réveil, la
    //    désactivation la voit (sans le verrou, elle lisait avant et passait : la direction aurait voté seule).
    let v1 = brouillon(120_000).await;
    let mut tenant = ent.pool_verif().begin().await.unwrap();
    verrou_pris(&mut tenant, &cle).await;
    let ent2 = ent.clone();
    let reglage = tokio::spawn(async move {
        ent2.definir_reglages_grilles_for_actor(
            etab,
            ud,
            rd,
            &ReglagesGrillesBody {
                exiger_qualification_articles: None,
                grille_second_regard: Some(false),
            },
        )
        .await
    });
    attendre_verrou(&ent, &cle).await;
    assert!(
        !reglage.is_finished(),
        "la désactivation doit attendre le verrou"
    );
    sqlx::query(
        "UPDATE ent_fin_grille_version SET statut = 'proposee', propose_le = now(), propose_par = $2 WHERE id = $1",
    )
    .bind(v1)
    .bind(u)
    .execute(&mut *tenant)
    .await
    .unwrap();
    tenant.commit().await.unwrap();
    assert!(
        conflit(reglage.await.unwrap()),
        "une proposition validée avant le réveil bloque la désactivation"
    );
    let (second,): (bool,) = sqlx::query_as(
        "SELECT COALESCE((SELECT grille_second_regard FROM ent_fin_parametres WHERE etablissement_id = $1), true)",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(second, "le second regard est resté actif");

    // 2. La proposition attend, elle aussi, le verrou (le même), puis passe quand il est levé.
    let v2 = brouillon(130_000).await;
    let mut tenant = ent.pool_verif().begin().await.unwrap();
    verrou_pris(&mut tenant, &cle).await;
    let ent2 = ent.clone();
    let proposition =
        tokio::spawn(async move { ent2.proposer_grille_for_actor(etab, u, r, v2).await });
    attendre_verrou(&ent, &cle).await;
    assert!(
        !proposition.is_finished(),
        "la proposition doit attendre le verrou"
    );
    tenant.commit().await.unwrap();
    assert_eq!(proposition.await.unwrap().unwrap().statut, "proposee");
}

/// Prend le verrou consultatif de transaction `cle` sur une connexion à part.
async fn verrou_pris(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, cle: &str) {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(cle)
        .execute(&mut **tx)
        .await
        .unwrap();
}

/// Attend (sans `sleep` : on rend la main au planificateur entre deux lectures de `pg_locks`) qu'une autre
/// connexion soit **en attente** du verrou `cle`.
async fn attendre_verrou(ent: &PgEnt, cle: &str) {
    for _ in 0..200_000 {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM pg_locks
              WHERE locktype = 'advisory' AND NOT granted
                AND ((classid::bigint << 32) | objid::bigint) = hashtextextended($1, 0)",
        )
        .bind(cle)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        if n > 0 {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("aucune connexion n'attend le verrou {cle}");
}

/// Faible 5 : les plafonds (familles types, simulations) se décomptent sous un verrou d'établissement.
#[tokio::test]
async fn f3s_plafonds_sous_verrou() {
    let Some((ent, c)) = labo().await else { return };
    let ent = std::sync::Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let cle = format!("fin:plafond:famille_type:{etab}");
    let mut tenant = ent.pool_verif().begin().await.unwrap();
    verrou_pris(&mut tenant, &cle).await;
    let ent2 = ent.clone();
    let creation = tokio::spawn(async move {
        ent2.creer_famille_type_for_actor(
            etab,
            u,
            r,
            &FamilleTypeBody {
                libelle: "Verrouillée".into(),
                tranche_code: None,
                enfants: vec![EnfantTypeBody {
                    niveau: "6EME".into(),
                    regime: "externe".into(),
                }],
            },
        )
        .await
        .map(|f| f.id)
    });
    attendre_verrou(&ent, &cle).await;
    assert!(!creation.is_finished());
    tenant.commit().await.unwrap();
    creation.await.unwrap().unwrap();
}

/// M-D : qualification exigée **par défaut** (établissement créé après la 523 : ligne absente) : refus de création
/// d'article « à l'ancienne » et d'émission d'une pièce non qualifiée, messages exacts.
#[tokio::test]
async fn f3s_md_qualification_exigee_par_defaut_refuse_creation_et_emission() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    // un brouillon portant un article non qualifié ne se crée que dans l'état « établissement antérieur »
    etablissement_anterieur(&ent, etab).await;
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
    ent.creer_tarif_for_actor(etab, u, r, &tarif_historique(art.id, 90_000))
        .await
        .unwrap();
    let el = eleve(&ent, etab, "Defaut", "externe").await;
    let f = foyer_payeur(&ent, etab, el, c.parent_lea.user_id, "Foyer défaut").await;
    facturer(&ent, etab, el).await;
    let (brouillon,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_fac_facture WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'brouillon'",
    )
    .bind(etab)
    .bind(f)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    // retour à l'état d'un établissement créé après la 523 : aucune ligne de paramètres (exigée par défaut)
    sqlx::query("DELETE FROM ent_fin_parametres WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert!(
        ent.reglages_grilles_for_actor(etab, u, r)
            .await
            .unwrap()
            .exiger_qualification_articles
    );
    // création d'article « à l'ancienne » : refusée, message exact
    match ent
        .enregistrer_article_for_actor(
            etab,
            u,
            r,
            None,
            &ArticleBody {
                code: "NEUF".into(),
                libelle: "Neuf".into(),
                categorie: "frais".into(),
                compte_produit: None,
                actif: None,
            },
        )
        .await
    {
        Err(EntDbError::Refus(RefusMetier::Conflit, m)) => assert_eq!(
            m,
            "la qualification des articles est exigée : créer l'article depuis un modèle (nature légale, TVA)"
        ),
        autre => panic!("refus attendu : {autre:?}"),
    }
    // émission d'une pièce portant un article non qualifié : refusée par pièce, message exact
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
    assert!(rapport.emises.is_empty(), "{rapport:?}");
    assert_eq!(rapport.refus.len(), 1, "{rapport:?}");
    assert_eq!(
        rapport.refus[0].motif,
        "article non qualifié (nature légale et TVA) : qualifier le catalogue avant d'émettre (R-F9)"
    );
}

/// Faibles 2 et 6 : alerte « qui, quand » à l'heure de l'établissement ; ordre personnel, droit, existence avant
/// la validation du corps.
#[tokio::test]
async fn f3s_alerte_second_regard_et_ordre_des_controles() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ud, rd) = direction(&c);
    // fuseau de l'établissement : loin de l'UTC, pour que l'heure affichée ne puisse pas être celle de l'UTC
    sqlx::query(
        "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, 'Pacific/Auckland')
         ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = 'Pacific/Auckland'",
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let reglages = ent
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
    let (attendu,): (String,) = sqlx::query_as(
        "SELECT to_char(second_regard_desactive_le AT TIME ZONE 'Pacific/Auckland', 'DD/MM/YYYY à HH24:MI')
           FROM ent_fin_parametres WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let alerte = reglages
        .alertes
        .iter()
        .find(|a| a.contains("second regard"))
        .expect("alerte du second regard");
    assert!(alerte.contains(&format!(" le {attendu}")), "{alerte}");
    assert!(
        alerte.contains(" par "),
        "l'auteur de la désactivation est nommé : {alerte}"
    );
    // faible a : jamais d'adresse e-mail en repli, jamais de date de rentrée en dur ; l'alerte ne va qu'à ceux qui
    // règlent ou votent (le secrétariat, qui lit seulement, ne la reçoit pas)
    assert!(
        !alerte.contains('@') && !alerte.contains("2027"),
        "{alerte}"
    );
    sqlx::query("UPDATE ent_user SET display_name = NULL WHERE id = $1")
        .bind(ud)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let direction_lit = ent.reglages_grilles_for_actor(etab, ud, rd).await.unwrap();
    assert!(
        direction_lit.alertes.iter().all(|a| !a.contains('@')),
        "sans nom, ni e-mail ni identifiant : {:?}",
        direction_lit.alertes
    );
    assert!(!direction_lit.alertes.is_empty());
    // un compte qui lit seulement (rôle délégué `facturation.lire`) voit les réglages mais pas l'alerte
    let (us, rs) = secretariat(&c);
    let lecteur = ent
        .creer_role_for_actor(
            etab,
            ud,
            rd,
            RoleSaisie {
                nom: format!(
                    "Lecteur facturation {}",
                    &Uuid::new_v4().simple().to_string()[..6]
                ),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: ent_auth::Action::FacturationLire,
                    portee: ent_auth::Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, ud, rd, lecteur.id, us, None)
        .await
        .unwrap();
    let lu = ent
        .avec_acteur(etab, us, rs, ent.reglages_grilles_for_actor(etab, us, rs))
        .await
        .unwrap()
        .unwrap();
    assert!(!lu.grille_second_regard, "les réglages restent lisibles");
    assert!(lu.alertes.is_empty(), "{:?}", lu.alertes);

    // ordre des contrôles : une grille inconnue répond « introuvable » avant toute erreur de saisie
    let inconnue = Uuid::new_v4();
    assert!(introuvable(
        ent.retour_brouillon_for_actor(
            etab,
            u,
            r,
            inconnue,
            &RetourBrouillonBody {
                motif: String::new()
            }
        )
        .await
    ));
    assert!(introuvable(
        ent.abandonner_grille_for_actor(
            etab,
            u,
            r,
            inconnue,
            &RetourBrouillonBody {
                motif: String::new()
            }
        )
        .await
    ));
    assert!(introuvable(
        ent.corriger_grille_for_actor(
            etab,
            ud,
            rd,
            inconnue,
            &CorrectionGrilleBody {
                motif: String::new(),
                effet_retroactif: false
            }
        )
        .await
    ));
    assert!(introuvable(
        ent.voter_grille_au_jour(
            etab,
            c.admin.user_id,
            Role::Admin.as_str(),
            inconnue,
            &VoteGrilleBody {
                vote_le: d(1999, 1, 1),
                reference: String::new(),
                document_id: None
            },
            jour_test()
        )
        .await
    ));
}

/// Faible 7 : le second regard garde ses auteurs : tarif retiré, tranches, proposant d'avant un retour en
/// brouillon ; le vote les refuse (service et base). Faible 9 : l'année d'un tarif versionné est figée.
#[tokio::test]
async fn f3s_contributeurs_du_second_regard_et_annee_du_tarif() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ud, rd) = direction(&c);
    let (ua, ra) = (c.admin.user_id, Role::Admin.as_str());
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
    let dp = article_modele(&ent, &c, "DEMI_PENSION", "DP").await;
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
    let t = ent
        .ajouter_tarif_grille_for_actor(etab, u, r, v.id, &tarif_body(contrib.id, 120_000, true))
        .await
        .unwrap();
    // faible 9 : l'année d'un tarif versionné ne change pas, même en brouillon
    let e = sqlx::query("UPDATE ent_fin_tarif SET annee_scolaire = '2031-2032' WHERE id = $1")
        .bind(t.id)
        .execute(ent.pool_verif())
        .await
        .unwrap_err();
    erreur_base(
        &e,
        "23514",
        "tarif : année différente de celle de la version",
    );
    // la direction ajoute puis retire un tarif, et pose les tranches : elle reste contributrice
    let extra = ent
        .ajouter_tarif_grille_for_actor(etab, ud, rd, v.id, &tarif_body(dp.id, 45_000, true))
        .await
        .unwrap();
    ent.retirer_tarif_grille_for_actor(etab, ud, rd, v.id, extra.id)
        .await
        .unwrap();
    let (contributeurs,): (Vec<Uuid>,) =
        sqlx::query_as("SELECT contributeurs FROM ent_fin_grille_version WHERE id = $1")
            .bind(v.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(
        contributeurs.contains(&ud) && contributeurs.contains(&u),
        "{contributeurs:?}"
    );
    // les contributeurs ne se retirent pas du brouillon
    let e = sqlx::query("UPDATE ent_fin_grille_version SET contributeurs = '{}' WHERE id = $1")
        .bind(v.id)
        .execute(ent.pool_verif())
        .await
        .unwrap_err();
    erreur_base(&e, "23001", "un contributeur ne se retire pas du brouillon");
    ent.proposer_grille_for_actor(etab, u, r, v.id)
        .await
        .unwrap();
    // la direction (contributrice) ne vote pas : service et base
    assert!(interdit(
        ent.voter_grille_test(etab, ud, rd, v.id, &vote("CA")).await
    ));
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let e = sqlx::query(
        "UPDATE ent_fin_grille_version SET statut = 'votee', vote_le = '2030-06-20', vote_reference = 'x',
                vote_par = $2 WHERE id = $1",
    )
    .bind(v.id)
    .bind(ud)
    .execute(&mut *tx)
    .await
    .unwrap_err();
    erreur_base(
        &e,
        "23001",
        "second regard : le votant est l'auteur de la grille",
    );
    drop(tx);
    // retour en brouillon : le proposant (direction, qui propose une autre fois) reste parmi les contributeurs
    ent.retour_brouillon_for_actor(
        etab,
        u,
        r,
        v.id,
        &RetourBrouillonBody {
            motif: "à revoir".into(),
        },
    )
    .await
    .unwrap();
    ent.proposer_grille_for_actor(etab, ud, rd, v.id)
        .await
        .unwrap();
    ent.retour_brouillon_for_actor(
        etab,
        ud,
        rd,
        v.id,
        &RetourBrouillonBody {
            motif: "encore".into(),
        },
    )
    .await
    .unwrap();
    let (propose_par, contributeurs): (Option<Uuid>, Vec<Uuid>) = sqlx::query_as(
        "SELECT propose_par, contributeurs FROM ent_fin_grille_version WHERE id = $1",
    )
    .bind(v.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(propose_par.is_none(), "le retour efface la proposition");
    assert!(
        contributeurs.contains(&ud),
        "… mais garde son auteur : {contributeurs:?}"
    );
    // un autre proposant, un autre votant : l'administrateur vote
    ent.proposer_grille_for_actor(etab, u, r, v.id)
        .await
        .unwrap();
    ent.voter_grille_test(etab, ua, ra, v.id, &vote("CA"))
        .await
        .unwrap();
}

/// Faible 3 : une version `close` (encore applicable jusqu'à la veille de la suivante) se publie ; faible 4 :
/// suppression d'une simulation réservée à son auteur ou à la direction, journalisée avec ses produits ; le
/// DELETE direct d'un calcul est refusé par la base.
#[tokio::test]
async fn f3s_publication_close_et_suppression_de_simulation() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ud, rd) = direction(&c);
    let (v1, contrib, _) = grille_proposee(&ent, &c).await;
    ent.voter_grille_test(etab, ud, rd, v1, &vote("CA du 20 juin"))
        .await
        .unwrap();
    ent.mettre_en_vigueur_for_actor(etab, u, r, v1, &MiseEnVigueurBody { du: d(2030, 9, 1) })
        .await
        .unwrap();
    let v2 = ent
        .corriger_grille_for_actor(
            etab,
            ud,
            rd,
            v1,
            &CorrectionGrilleBody {
                motif: "revalorisation".into(),
                effet_retroactif: false,
            },
        )
        .await
        .unwrap();
    ent.proposer_grille_for_actor(etab, u, r, v2.id)
        .await
        .unwrap();
    ent.voter_grille_au_jour(
        etab,
        c.admin.user_id,
        Role::Admin.as_str(),
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
    ent.mettre_en_vigueur_for_actor(etab, u, r, v2.id, &MiseEnVigueurBody { du: d(2031, 1, 1) })
        .await
        .unwrap();
    let (statut,): (String,) =
        sqlx::query_as("SELECT statut FROM ent_fin_grille_version WHERE id = $1")
            .bind(v1)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(statut, "close");
    // au jour de référence (v2 pas encore en effet) v1 est la version applicable la plus récente : elle se publie
    let pub1 = ent
        .publier_grille_au_jour(etab, ud, rd, v1, jour_test())
        .await
        .unwrap();
    assert_eq!(pub1.numero, 1);
    assert!(!pub1.lignes.is_empty());
    // faible b : une fois v2 en effet, v1 est supplantée et ne se publie plus ; v2 se publie
    assert!(conflit(
        ent.publier_grille_au_jour(etab, ud, rd, v1, d(2031, 1, 1))
            .await
    ));
    ent.publier_grille_au_jour(etab, ud, rd, v2.id, d(2031, 1, 1))
        .await
        .unwrap();
    // une version abandonnée, elle, ne se publie jamais
    let ab = ent
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
    ent.abandonner_grille_for_actor(
        etab,
        u,
        r,
        ab.id,
        &RetourBrouillonBody {
            motif: "essai".into(),
        },
    )
    .await
    .unwrap();
    assert!(conflit(
        ent.publier_grille_au_jour(etab, ud, rd, ab.id, jour_test())
            .await
    ));

    // ── simulations : auteur ou direction seulement ──
    let corps = |libelle: &str| SimulationBody {
        annee_scolaire: ANNEE.into(),
        libelle: libelle.into(),
        base_version_id: v1,
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
    };
    let par_direction = ent
        .creer_simulation_for_actor(etab, ud, rd, &corps("scénario de la direction"))
        .await
        .unwrap();
    let par_compta = ent
        .creer_simulation_for_actor(etab, u, r, &corps("scénario de la comptabilité"))
        .await
        .unwrap();
    ent.calculer_simulation_for_actor(etab, u, r, par_compta.simulation_id)
        .await
        .unwrap();
    // la comptabilité (simuler, sans voter) ne supprime pas le scénario d'un autre ; la direction oui
    assert!(refuse(
        ent.supprimer_simulation_for_actor(etab, u, r, par_direction.simulation_id)
            .await
    ));
    // le DELETE direct d'un calcul historisé est refusé par la base (seule la suppression de la simulation passe)
    let e = sqlx::query("DELETE FROM ent_fin_simulation_calcul WHERE simulation_id = $1")
        .bind(par_compta.simulation_id)
        .execute(ent.pool_verif())
        .await
        .unwrap_err();
    erreur_base(
        &e,
        "23001",
        "un calcul historisé ne se supprime qu'avec sa simulation",
    );
    ent.supprimer_simulation_for_actor(etab, ud, rd, par_compta.simulation_id)
        .await
        .unwrap();
    let (restants,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM ent_fin_simulation_calcul WHERE simulation_id = $1")
            .bind(par_compta.simulation_id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(restants, 0, "l'historique part avec la simulation");
    // le journal garde le libellé, le nombre de calculs et leurs produits avant → après
    let (detail,): (String,) = sqlx::query_as(
        "SELECT detail FROM ent_fin_journal WHERE etablissement_id = $1 AND action = 'simulation.supprimer'
          AND objet_id = $2",
    )
    .bind(etab)
    .bind(par_compta.simulation_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(detail.contains("scénario de la comptabilité"), "{detail}");
    assert!(
        detail.contains("calcul(s) supprimé(s)") && detail.contains('→'),
        "{detail}"
    );
    // l'auteur supprime son propre scénario sans autre droit que `simuler`
    ent.supprimer_simulation_for_actor(etab, ud, rd, par_direction.simulation_id)
        .await
        .unwrap();
}

// ───────────────────────── Suites, 2 (contre-revue des suites, D-069) ─────────────────────────

/// Génération annuelle sans rien écrire en cas d'anomalie : renvoie le motif de refus.
async fn facturer_motif(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Result<(), String> {
    let mut tx = ent.pool_verif().begin().await.unwrap();
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
    .unwrap();
    tx.commit().await.unwrap();
    r.map(|_| ())
}

async fn lignes_de(ent: &PgEnt, eleve: Uuid) -> Vec<(String, i64)> {
    sqlx::query_as(
        "SELECT source_cle, (montant_unitaire_centimes * quantite)::bigint FROM ent_fac_ligne
          WHERE eleve_id = $1 AND categorie <> 'reduction' ORDER BY source_cle",
    )
    .bind(eleve)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap()
}

/// Moyen 1 (suite), cas 1 : première grille de l'année en vigueur au 1er octobre, sans historique : septembre n'a
/// aucun tarif applicable, la génération refuse (rien d'écrit) au lieu de facturer zéro en silence.
#[tokio::test]
async fn f3s2_premiere_grille_au_premier_octobre_sans_historique() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ua, ra) = (c.admin.user_id, Role::Admin.as_str());
    let (v1, _, _) = grille_proposee(&ent, &c).await;
    ent.voter_grille_test(etab, ua, ra, v1, &vote("CA du 20 juin"))
        .await
        .unwrap();
    ent.mettre_en_vigueur_for_actor(etab, u, r, v1, &MiseEnVigueurBody { du: d(2030, 10, 1) })
        .await
        .unwrap();
    let el = eleve(&ent, etab, "Octobre", "demi_pension").await;
    let _ = foyer_payeur(&ent, etab, el, c.parent_lea.user_id, "Foyer octobre").await;
    let motif = facturer_motif(&ent, etab, el).await.unwrap_err();
    assert!(
        motif.contains("aucun tarif applicable à partir du 01/09/2030"),
        "{motif}"
    );
    assert!(lignes_de(&ent, el).await.is_empty(), "rien n'est écrit");
}

/// Faible c : un article inchangé (tarif recopié à l'identique dans la version suivante) n'est ni coupé en deux
/// lignes ni arrondi deux fois quand un autre article change de tarif ; l'article révisé a deux tronçons.
#[tokio::test]
async fn f3s2_article_inchange_non_coupe_quand_un_autre_change() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ud, rd) = direction(&c);
    let (ua, ra) = (c.admin.user_id, Role::Admin.as_str());
    let (v1, contrib, dp) = grille_proposee(&ent, &c).await;
    ent.voter_grille_test(etab, ua, ra, v1, &vote("CA du 20 juin"))
        .await
        .unwrap();
    ent.mettre_en_vigueur_for_actor(etab, u, r, v1, &MiseEnVigueurBody { du: d(2030, 9, 1) })
        .await
        .unwrap();
    // v2 : copie de v1 ; la demi-pension est portée à 500 € (inchangée pour la contribution, recopiée à l'identique)
    let v2 = ent
        .corriger_grille_for_actor(
            etab,
            ud,
            rd,
            v1,
            &CorrectionGrilleBody {
                motif: "revalorisation".into(),
                effet_retroactif: false,
            },
        )
        .await
        .unwrap();
    let (t_dp,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_fin_tarif WHERE grille_version_id = $1 AND article_id = $2",
    )
    .bind(v2.id)
    .bind(dp)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    ent.modifier_tarif_grille_for_actor(etab, ud, rd, v2.id, t_dp, &tarif_body(dp, 50_000, true))
        .await
        .unwrap();
    ent.proposer_grille_for_actor(etab, u, r, v2.id)
        .await
        .unwrap();
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
    ent.mettre_en_vigueur_for_actor(etab, u, r, v2.id, &MiseEnVigueurBody { du: d(2031, 1, 1) })
        .await
        .unwrap();
    let el = eleve(&ent, etab, "Inchange", "demi_pension").await;
    let _ = foyer_payeur(&ent, etab, el, c.parent_lea.user_id, "Foyer inchangé").await;
    facturer(&ent, etab, el).await;
    let lignes = lignes_de(&ent, el).await;
    let de = |a: Uuid| -> Vec<&(String, i64)> {
        lignes
            .iter()
            .filter(|(k, _)| k.contains(&a.to_string()))
            .collect()
    };
    let l_contrib = de(contrib);
    assert_eq!(
        l_contrib.len(),
        1,
        "article inchangé : une seule ligne {lignes:?}"
    );
    assert!(
        !l_contrib[0].0.contains(":m"),
        "clé historique : {lignes:?}"
    );
    assert_eq!(l_contrib[0].1, 120_000);
    let l_dp = de(dp);
    assert_eq!(l_dp.len(), 2, "article révisé : deux tronçons {lignes:?}");
    assert_eq!(l_dp.iter().map(|l| l.1).sum::<i64>(), 18_000 + 30_000);
}

/// Moyen 1 (suite), cas 2, revu par le pilote (contre-revue, Moyen 2, D-069) : un trou **en tête** propre à un article
/// (service arrivé en cours d'année) n'est pas bloquant : la demi-pension, sans tarif en septembre, est facturée à partir
/// d'octobre (9 mois sur 10), la contribution toute l'année.
#[tokio::test]
async fn f3s2_trou_en_tete_d_un_article_facture_a_partir_du_premier_mois_tarife() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ua, ra) = (c.admin.user_id, Role::Admin.as_str());
    let (v1, contrib, dp) = grille_proposee(&ent, &c).await;
    ent.creer_tarif_for_actor(etab, u, r, &tarif_historique(contrib, 100_000))
        .await
        .unwrap();
    ent.voter_grille_test(etab, ua, ra, v1, &vote("CA du 20 juin"))
        .await
        .unwrap();
    ent.mettre_en_vigueur_for_actor(etab, u, r, v1, &MiseEnVigueurBody { du: d(2030, 10, 1) })
        .await
        .unwrap();
    let el = eleve(&ent, etab, "Tete", "demi_pension").await;
    let _ = foyer_payeur(&ent, etab, el, c.parent_lea.user_id, "Foyer tete").await;
    facturer_motif(&ent, etab, el).await.expect("trou en tête : facturé");
    let (dp_total,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(sum(montant_unitaire_centimes * quantite), 0)::bigint FROM ent_fac_ligne WHERE eleve_id = $1 AND article_id = $2",
    )
    .bind(el)
    .bind(dp)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(dp_total, 45_000 * 9 / 10, "demi-pension d'octobre à juin");
}

/// Moyen 2 : le test « contributeurs » est discriminant. La direction n'a jamais touché un tarif ni créé la grille :
/// elle ne devient contributrice que parce qu'elle l'a **proposée**. Après un retour en brouillon et une nouvelle
/// proposition par la comptabilité, elle reste exclue du vote (service et SQL direct) ; avant sa proposition elle
/// n'était pas contributrice.
#[tokio::test]
async fn f3s2_proposant_sans_tarif_reste_exclu_du_vote_apres_retour_en_brouillon() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = compta(&c);
    let (ud, rd) = direction(&c);
    let contrib = article_modele(&ent, &c, "CONTRIBUTION", "CONTRIB").await;
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
    let contributeurs = async || -> Vec<Uuid> {
        let (c,): (Vec<Uuid>,) =
            sqlx::query_as("SELECT contributeurs FROM ent_fin_grille_version WHERE id = $1")
                .bind(v.id)
                .fetch_one(ent.pool_verif())
                .await
                .unwrap();
        c
    };
    assert!(
        !contributeurs().await.contains(&ud),
        "pas encore contributrice"
    );
    ent.proposer_grille_for_actor(etab, ud, rd, v.id)
        .await
        .unwrap();
    ent.retour_brouillon_for_actor(
        etab,
        ud,
        rd,
        v.id,
        &RetourBrouillonBody {
            motif: "à revoir".into(),
        },
    )
    .await
    .unwrap();
    let (propose_par,): (Option<Uuid>,) =
        sqlx::query_as("SELECT propose_par FROM ent_fin_grille_version WHERE id = $1")
            .bind(v.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(propose_par.is_none(), "le retour efface la proposition");
    assert!(
        contributeurs().await.contains(&ud),
        "le proposant d'avant le retour est conservé"
    );
    // la comptabilité propose à nouveau : la direction (aucun tarif touché) ne vote pas, la base non plus
    ent.proposer_grille_for_actor(etab, u, r, v.id)
        .await
        .unwrap();
    assert!(interdit(
        ent.voter_grille_test(etab, ud, rd, v.id, &vote("CA")).await
    ));
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let e = sqlx::query(
        "UPDATE ent_fin_grille_version SET statut = 'votee', vote_le = '2030-06-20', vote_reference = 'x',
                vote_par = $2 WHERE id = $1",
    )
    .bind(v.id)
    .bind(ud)
    .execute(&mut *tx)
    .await
    .unwrap_err();
    erreur_base(
        &e,
        "23001",
        "second regard : le votant est l'auteur de la grille",
    );
    drop(tx);
    // l'administrateur, étranger à la grille, vote
    ent.voter_grille_test(
        etab,
        c.admin.user_id,
        Role::Admin.as_str(),
        v.id,
        &vote("CA"),
    )
    .await
    .unwrap();
}
