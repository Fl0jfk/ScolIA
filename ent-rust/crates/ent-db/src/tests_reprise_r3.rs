//! Socle 28, lot R3 : rapprochement, file « à décider » et décisions, contre une vraie base. Export fictif décodable
//! (aucune donnée réelle). **Aucun test ne lit l'horloge** : le jour de référence vient de l'instant fourni aux options.
//! Sans `ENT_DATABASE_URL`, ignoré en local (échec en CI).

use crate::reprise::{self, CibleExecution, ModeExecution};
use crate::reprise_file::{DemandeDecision, EtatFile, FiltreFile, ModeLot};
use crate::reprise_rapprochement::charger_cible;
use crate::reprise_zone::executer_simulation;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::tests_reprise_r2::instantane_metier;
use crate::tests_voyages_v0a::{compte, connecter, est_forbidden};
use chrono::{NaiveDate, TimeZone, Utc};
use ent_reprise::decodage::{decoder_export, OptionsDecodage};
use ent_reprise::jeu_fictif;
use ent_reprise::rapprochement::donnees::SourceRapprochement;
use ent_reprise::rapprochement::ActionDecision;
use sqlx::{PgPool, Row};
use uuid::Uuid;

fn options() -> OptionsDecodage {
    let d = std::env::temp_dir().join(format!("ent-r3-db-{}", Uuid::new_v4()));
    jeu_fictif::generer_decodable(&d, 3).expect("jeu fictif");
    OptionsDecodage {
        dump: d.join("dump.sql"),
        objets_s3: Some(d.join("objets_s3.tsv")),
        contenus_s3: Some(d.join("contenus_s3")),
        maintenant: Utc.with_ymd_and_hms(2026, 11, 16, 12, 0, 0).unwrap(),
    }
}

fn source(o: &OptionsDecodage) -> SourceRapprochement {
    SourceRapprochement::extraire(&decoder_export(o).expect("décodage"))
}

async fn seed_eleve(
    pool: &PgPool,
    etab: Uuid,
    nom: &str,
    prenom: &str,
    ine: Option<&str>,
    naissance: Option<NaiveDate>,
    classe: Option<&str>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, ine, status, classe, date_naissance)
         VALUES ($1, $2, $3, $4, $5, 'actif', $6, $7)",
    )
    .bind(id)
    .bind(etab)
    .bind(nom)
    .bind(prenom)
    .bind(ine)
    .bind(classe)
    .bind(naissance)
    .execute(pool)
    .await
    .expect("élève");
    id
}

/// ScolIA fictif : l'élève 0 de l'export y existe avec le même INE, l'élève 1 y existe sans INE (mêmes nom, prénom,
/// naissance), l'élève 2 n'y existe pas.
async fn semer(pool: &PgPool, etab: Uuid, src: &SourceRapprochement) -> (Uuid, Uuid) {
    let e = &src.eleves;
    let a = seed_eleve(
        pool,
        etab,
        &e[0].nom,
        &e[0].prenom,
        e[0].ine.as_deref(),
        e[0].date_naissance,
        None,
    )
    .await;
    let b = seed_eleve(
        pool,
        etab,
        &e[1].nom.to_uppercase(),
        &e[1].prenom,
        None,
        e[1].date_naissance,
        None,
    )
    .await;
    (a, b)
}

async fn compter(pool: &PgPool, etab: Uuid, table: &str) -> i64 {
    let mut tx = TenantTx::begin(pool, etab).await.unwrap();
    let n = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM {table} WHERE etablissement_id = $1"
    ))
    .bind(etab)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    n
}

fn conflit(e: &EntDbError) -> bool {
    matches!(
        e,
        EntDbError::ConflitDossier(_) | EntDbError::ConflitDossierDetail(_)
    )
}

fn saisie_invalide(e: &EntDbError) -> bool {
    matches!(e, EntDbError::SaisieInvalide(_))
}

struct Cadre {
    ent: PgEnt,
    etab: Uuid,
    dir: Uuid,
    sec: Uuid,
    o: OptionsDecodage,
    src: SourceRapprochement,
    eleve_ine: Uuid,
    eleve_nom: Uuid,
}

impl Cadre {
    async fn nouveau() -> Option<Cadre> {
        let ent = connecter().await?;
        let etab = Uuid::new_v4();
        let dir = compte(&ent, etab, "direction").await;
        let sec = compte(&ent, etab, "administratif").await;
        let o = options();
        let src = source(&o);
        let (eleve_ine, eleve_nom) = semer(ent.pool(), etab, &src).await;
        Some(Cadre {
            ent,
            etab,
            dir,
            sec,
            o,
            src,
            eleve_ine,
            eleve_nom,
        })
    }

    async fn simuler(&self) -> crate::reprise_zone::ResultatSimulation {
        executer_simulation(
            self.ent.pool(),
            self.etab,
            &self.o,
            CibleExecution::Jetable,
            None,
        )
        .await
        .expect("simulation")
    }

    async fn file(
        &self,
        objet: Option<&str>,
        etat: Option<EtatFile>,
    ) -> crate::reprise_file::PageFile {
        self.ent
            .reprise_file_lister(
                self.etab,
                self.dir,
                "direction",
                FiltreFile {
                    objet: objet.map(str::to_string),
                    etat,
                    limite: 200,
                    decalage: 0,
                },
            )
            .await
            .expect("file")
    }

    async fn ligne_eleve_avec_candidat(&self) -> crate::reprise_file::LigneFileVue {
        self.file(Some("eleve"), Some(EtatFile::ADecider))
            .await
            .lignes
            .into_iter()
            .find(|l| !l.candidats.is_empty())
            .expect("une ligne d'élève avec candidat")
    }

    async fn ligne_eleve_sans_candidat(&self) -> crate::reprise_file::LigneFileVue {
        self.file(Some("eleve"), Some(EtatFile::ADecider))
            .await
            .lignes
            .into_iter()
            .find(|l| l.candidats.is_empty())
            .expect("une ligne d'élève sans candidat")
    }
}

#[tokio::test]
async fn r3_simulation_rapproche_par_ine_alimente_la_file_et_n_ecrit_rien_dans_le_metier() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let avant = instantane_metier(c.ent.pool()).await;
    let r = c.simuler().await;
    let apres = instantane_metier(c.ent.pool()).await;
    assert_eq!(avant, apres, "la simulation a écrit dans une table métier");
    assert_eq!(r.simulation.ecritures_metier, 0);
    assert!(r.simulation.invariants_ok());
    // L'élève 0 est rapproché par son INE (seule règle automatique) ; jamais une fiche créée.
    assert_eq!(r.simulation.par_objet["eleve"].rapprochees, 1);
    assert_eq!(r.simulation.par_objet["eleve"].creees, 0);
    // La trace porte les comptes (l'invariant est une contrainte CHECK) et la file.
    let mut tx = TenantTx::begin(c.ent.pool(), c.etab).await.unwrap();
    let n: i64 = sqlx::query_scalar(
        "SELECT rapprochees FROM rep_execution_compte WHERE etablissement_id = $1 AND execution_id = $2 AND objet = 'eleve'",
    )
    .bind(c.etab)
    .bind(r.execution.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(n, 1);
    tx.rollback().await.unwrap();
    assert_eq!(
        compter(c.ent.pool(), c.etab, "rep_proposition").await,
        r.simulation.file.len() as i64
    );
    assert_eq!(
        compter(c.ent.pool(), c.etab, "rep_correspondance").await,
        0,
        "une simulation n'enregistre aucune correspondance"
    );
    // Chaque candidat de la file porte au moins une raison et une confiance ; la ligne de l'élève 1 est « forte ».
    let l = c.ligne_eleve_avec_candidat().await;
    assert_eq!(l.meilleure_confiance.as_deref(), Some("forte"));
    let cand = &l.candidats[0];
    assert_eq!(cand.cible_id, c.eleve_nom);
    let codes: Vec<&str> = cand.raisons.iter().map(|r| r.code.as_str()).collect();
    assert!(
        codes.contains(&"nom_identique")
            && codes.contains(&"prenom_identique")
            && codes.contains(&"naissance_identique")
    );
    assert!(cand.raisons.iter().all(|r| !r.explication.is_empty()));
    assert!(cand.lot_possible);
    // Minimisation : l'INE n'est jamais entier dans la file.
    let ine = c.src.eleves[1].ine.clone().unwrap();
    assert!(l
        .source
        .ine_fin
        .as_deref()
        .is_some_and(|f| f.starts_with('…') && !f.contains(&ine)));
}

#[tokio::test]
async fn r3_simulation_rejouable_meme_empreinte() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let a = c.simuler().await;
    let b = c.simuler().await;
    assert_eq!(a.rapport_empreinte, b.rapport_empreinte);
    assert_eq!(a.simulation.file.len(), b.simulation.file.len());
    assert_ne!(a.execution.numero, b.execution.numero);
}

#[tokio::test]
async fn r3_droits_cote_serveur_et_cloisonnement_d_etablissement() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    c.simuler().await;
    let (prof, compta, parent) = (
        compte(&c.ent, c.etab, "prof").await,
        compte(&c.ent, c.etab, "compta").await,
        compte(&c.ent, c.etab, "parent").await,
    );
    for (u, role) in [(prof, "prof"), (compta, "compta"), (parent, "parent")] {
        let e = c
            .ent
            .reprise_file_lister(c.etab, u, role, FiltreFile::default())
            .await
            .expect_err(role);
        assert!(est_forbidden(&e), "{role} ne voit pas la file");
        assert!(est_forbidden(
            &c.ent
                .reprise_file_resume(c.etab, u, role)
                .await
                .expect_err(role)
        ));
        assert!(est_forbidden(
            &c.ent
                .reprise_synonymes_lister(c.etab, u, role)
                .await
                .expect_err(role)
        ));
        let e = c
            .ent
            .reprise_decider(
                c.etab,
                u,
                role,
                DemandeDecision {
                    proposition_id: Uuid::new_v4(),
                    action: ActionDecision::Ignorer,
                    cible_id: None,
                    motif: Some("test".into()),
                },
            )
            .await
            .expect_err(role);
        assert!(est_forbidden(&e), "{role} ne décide pas");
        assert!(est_forbidden(
            &c.ent
                .reprise_decider_lot(
                    c.etab,
                    u,
                    role,
                    vec![Uuid::new_v4()],
                    ModeLot::CreerSansCandidat
                )
                .await
                .expect_err(role)
        ));
        assert!(est_forbidden(
            &c.ent
                .reprise_decision_annuler(c.etab, u, role, Uuid::new_v4(), "motif")
                .await
                .expect_err(role)
        ));
    }
    // Secrétariat et direction voient la file.
    assert!(!c.file(None, None).await.lignes.is_empty());
    let page = c
        .ent
        .reprise_file_lister(c.etab, c.sec, "administratif", FiltreFile::default())
        .await
        .unwrap();
    assert!(page.total > 0);
    // Un autre établissement, même direction, ne voit rien de cet établissement.
    let autre = Uuid::new_v4();
    let dir2 = compte(&c.ent, autre, "direction").await;
    let p2 = c
        .ent
        .reprise_file_lister(autre, dir2, "direction", FiltreFile::default())
        .await
        .unwrap();
    assert_eq!(
        (p2.execution_numero, p2.total, p2.lignes.len()),
        (None, 0, 0)
    );
    // Et ne peut pas décider une ligne d'un autre établissement : la ligne lui est inconnue.
    let ligne = c.ligne_eleve_avec_candidat().await;
    let e = c
        .ent
        .reprise_decider(
            autre,
            dir2,
            "direction",
            DemandeDecision {
                proposition_id: ligne.id,
                action: ActionDecision::Ignorer,
                cible_id: None,
                motif: Some("tentative".into()),
            },
        )
        .await
        .expect_err("autre établissement");
    assert!(saisie_invalide(&e));
    assert_eq!(
        c.file(Some("eleve"), Some(EtatFile::Decidee))
            .await
            .lignes
            .len(),
        0
    );
}

#[tokio::test]
async fn r3_decision_tracee_rejouee_et_reversible() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let r1 = c.simuler().await;
    let l = c.ligne_eleve_avec_candidat().await;
    let cible = l.candidats[0].cible_id;
    let id = c
        .ent
        .reprise_decider(
            c.etab,
            c.sec,
            "administratif",
            DemandeDecision {
                proposition_id: l.id,
                action: ActionDecision::Rattacher,
                cible_id: Some(cible),
                motif: None,
            },
        )
        .await
        .expect("décision");
    // Tracée : qui, quand, quoi, raisons affichées.
    let mut tx = TenantTx::begin(c.ent.pool(), c.etab).await.unwrap();
    let d = sqlx::query("SELECT decision, cible_id, par_user_id, le, confiance_vue, en_lot FROM rep_decision WHERE etablissement_id = $1 AND id = $2")
        .bind(c.etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(d.get::<String, _>("decision"), "rattacher");
    assert_eq!(d.get::<Option<Uuid>, _>("cible_id"), Some(cible));
    assert_eq!(d.get::<Uuid, _>("par_user_id"), c.sec);
    assert_eq!(
        d.get::<Option<String>, _>("confiance_vue").as_deref(),
        Some("forte")
    );
    assert!(!d.get::<bool, _>("en_lot"));
    let raisons: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rep_decision_raison WHERE etablissement_id = $1 AND decision_id = $2",
    )
    .bind(c.etab)
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(raisons >= 3);
    let statut: String = sqlx::query_scalar("SELECT statut FROM rep_correspondance WHERE etablissement_id = $1 AND source_objet = 'eleve' AND cible_id = $2")
        .bind(c.etab)
        .bind(cible)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(statut, "rapprochee");
    // Ajout seul : ni suppression ni modification de la décision.
    assert!(
        sqlx::query("DELETE FROM rep_decision WHERE etablissement_id = $1")
            .bind(c.etab)
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    let mut tx = TenantTx::begin(c.ent.pool(), c.etab).await.unwrap();
    assert!(
        sqlx::query("UPDATE rep_decision SET motif = 'réécrit' WHERE etablissement_id = $1")
            .bind(c.etab)
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    // La ligne est décidée : une seconde décision est un conflit.
    let e = c
        .ent
        .reprise_decider(
            c.etab,
            c.dir,
            "direction",
            DemandeDecision {
                proposition_id: l.id,
                action: ActionDecision::Rattacher,
                cible_id: Some(cible),
                motif: None,
            },
        )
        .await
        .expect_err("déjà décidée");
    assert!(conflit(&e));
    let vue = c.file(Some("eleve"), Some(EtatFile::Decidee)).await;
    assert_eq!(vue.lignes.len(), 1);
    assert_eq!(vue.lignes[0].decision.as_ref().unwrap().par_user_id, c.sec);
    // Rejouée : la simulation suivante ne repose plus la question et compte un rapprochement de plus.
    let r2 = c.simuler().await;
    assert!(r2.simulation.invariants_ok());
    assert_eq!(
        r2.simulation.par_objet["eleve"].rapprochees,
        r1.simulation.par_objet["eleve"].rapprochees + 1
    );
    assert_eq!(
        r2.simulation.par_objet["eleve"].a_decider,
        r1.simulation.par_objet["eleve"].a_decider - 1
    );
    // L'ancienne proposition appartient à la première exécution : on peut encore annuler sa décision.
    let e = c
        .ent
        .reprise_decision_annuler(c.etab, c.dir, "direction", l.id, "essai")
        .await;
    assert!(
        e.is_ok(),
        "annuler reste possible sur l'ancienne ligne : {e:?}"
    );
    // Réversible : la ligne redevient « à décider » à l'exécution suivante.
    let r3 = c.simuler().await;
    assert_eq!(
        r3.simulation.par_objet["eleve"].a_decider,
        r1.simulation.par_objet["eleve"].a_decider
    );
    let mut tx = TenantTx::begin(c.ent.pool(), c.etab).await.unwrap();
    let a: (Option<chrono::DateTime<Utc>>, Option<Uuid>, Option<String>) = sqlx::query_as(
        "SELECT annulee_le, annulee_par, annulee_motif FROM rep_decision WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(c.etab)
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(a.0.is_some());
    assert_eq!((a.1, a.2.as_deref()), (Some(c.dir), Some("essai")));
    tx.rollback().await.unwrap();
    // Annuler sans motif, ou une ligne sans décision : refusé.
    let nouvelle = c.ligne_eleve_avec_candidat().await;
    assert!(saisie_invalide(
        &c.ent
            .reprise_decision_annuler(c.etab, c.dir, "direction", nouvelle.id, "  ")
            .await
            .unwrap_err()
    ));
    assert!(conflit(
        &c.ent
            .reprise_decision_annuler(c.etab, c.dir, "direction", nouvelle.id, "pas de décision")
            .await
            .unwrap_err()
    ));
}

#[tokio::test]
async fn r3_decisions_validees_cote_serveur() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    c.simuler().await;
    let avec = c.ligne_eleve_avec_candidat().await;
    let sans = c.ligne_eleve_sans_candidat().await;
    let dec = |id: Uuid, action, cible: Option<Uuid>, motif: Option<&str>| DemandeDecision {
        proposition_id: id,
        action,
        cible_id: cible,
        motif: motif.map(str::to_string),
    };
    let tenter = |d: DemandeDecision| {
        let (ent, etab, u) = (&c.ent, c.etab, c.dir);
        async move { ent.reprise_decider(etab, u, "direction", d).await }
    };
    // Rattacher à une fiche qui n'est pas proposée : refusé.
    assert!(saisie_invalide(
        &tenter(dec(
            avec.id,
            ActionDecision::Rattacher,
            Some(Uuid::new_v4()),
            Some("motif")
        ))
        .await
        .unwrap_err()
    ));
    // Rattacher sans désigner : refusé.
    assert!(saisie_invalide(
        &tenter(dec(avec.id, ActionDecision::Rattacher, None, None))
            .await
            .unwrap_err()
    ));
    // Ignorer sans motif : refusé ; créer alors qu'une fiche forte existe sans motif : refusé.
    assert!(saisie_invalide(
        &tenter(dec(sans.id, ActionDecision::Ignorer, None, None))
            .await
            .unwrap_err()
    ));
    assert!(saisie_invalide(
        &tenter(dec(avec.id, ActionDecision::Creer, None, None))
            .await
            .unwrap_err()
    ));
    assert!(saisie_invalide(
        &tenter(dec(
            sans.id,
            ActionDecision::Ignorer,
            None,
            Some(&"x".repeat(501))
        ))
        .await
        .unwrap_err()
    ));
    assert_eq!(
        compter(c.ent.pool(), c.etab, "rep_decision").await,
        0,
        "aucune décision refusée n'a laissé de trace"
    );
    // Créer sans candidat : libre. Créer malgré un candidat fort : avec motif.
    tenter(dec(sans.id, ActionDecision::Creer, None, None))
        .await
        .expect("création décidée");
    tenter(dec(
        avec.id,
        ActionDecision::Creer,
        None,
        Some("frère jumeau, pas le même élève"),
    ))
    .await
    .expect("avec motif");
    assert_eq!(compter(c.ent.pool(), c.etab, "rep_decision").await, 2);
    // Une création décidée est proposée par la simulation suivante, jamais faite.
    let avant = instantane_metier(c.ent.pool()).await;
    let r = c.simuler().await;
    assert_eq!(instantane_metier(c.ent.pool()).await, avant);
    assert_eq!(r.simulation.par_objet["eleve"].creees, 2);
}

#[tokio::test]
async fn r3_decision_en_lot_seulement_pour_les_cas_eligibles() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    c.simuler().await;
    let page = c.file(Some("eleve"), Some(EtatFile::ADecider)).await;
    let ids: Vec<Uuid> = page.lignes.iter().map(|l| l.id).collect();
    assert!(ids.len() >= 2);
    let r = c
        .ent
        .reprise_decider_lot(
            c.etab,
            c.dir,
            "direction",
            ids.clone(),
            ModeLot::RattacherMeilleurCandidat,
        )
        .await
        .unwrap();
    assert_eq!(
        r.decidees, 1,
        "seule la proposition forte sans contradiction est décidée"
    );
    assert_eq!(r.ecartees.len(), ids.len() - 1);
    assert!(r.ecartees.iter().all(|(_, m)| m == "non_eligible_lot"));
    // Chaque décision du lot est tracée à l'unité, marquée « en lot ».
    assert_eq!(compter(c.ent.pool(), c.etab, "rep_decision").await, 1);
    let mut tx = TenantTx::begin(c.ent.pool(), c.etab).await.unwrap();
    let (lot, motif): (bool, Option<String>) =
        sqlx::query_as("SELECT en_lot, motif FROM rep_decision WHERE etablissement_id = $1")
            .bind(c.etab)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    tx.rollback().await.unwrap();
    assert!(lot && motif.is_some());
    // Créer en lot : seulement les lignes sans candidat.
    let page = c.file(Some("eleve"), Some(EtatFile::ADecider)).await;
    let ids: Vec<Uuid> = page.lignes.iter().map(|l| l.id).collect();
    let r = c
        .ent
        .reprise_decider_lot(c.etab, c.dir, "direction", ids, ModeLot::CreerSansCandidat)
        .await
        .unwrap();
    assert!(r.decidees >= 1);
    // Lot vide ou trop gros : refusé.
    assert!(saisie_invalide(
        &c.ent
            .reprise_decider_lot(
                c.etab,
                c.dir,
                "direction",
                vec![],
                ModeLot::CreerSansCandidat
            )
            .await
            .unwrap_err()
    ));
    let trop: Vec<Uuid> = (0..501).map(|_| Uuid::new_v4()).collect();
    assert!(saisie_invalide(
        &c.ent
            .reprise_decider_lot(c.etab, c.dir, "direction", trop, ModeLot::CreerSansCandidat)
            .await
            .unwrap_err()
    ));
}

#[tokio::test]
async fn r3_une_reprise_appliquee_fige_les_decisions() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    c.simuler().await;
    let l = c.ligne_eleve_avec_candidat().await;
    let id = c
        .ent
        .reprise_decider(
            c.etab,
            c.dir,
            "direction",
            DemandeDecision {
                proposition_id: l.id,
                action: ActionDecision::Rattacher,
                cible_id: Some(l.candidats[0].cible_id),
                motif: None,
            },
        )
        .await
        .unwrap();
    let _ = id;
    let autre = c.ligne_eleve_sans_candidat().await;
    // Une exécution d'application terminée : plus aucune décision ne se modifie.
    let exec = reprise::ouvrir_execution(
        c.ent.pool(),
        c.etab,
        "next-schema-v1",
        ModeExecution::Application,
        CibleExecution::Jetable,
        Some(c.dir),
    )
    .await
    .unwrap();
    let mut tx = TenantTx::begin(c.ent.pool(), c.etab).await.unwrap();
    sqlx::query("UPDATE rep_execution SET etat = 'terminee', fin_le = now() WHERE etablissement_id = $1 AND id = $2")
        .bind(c.etab)
        .bind(exec.id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let e = c
        .ent
        .reprise_decision_annuler(c.etab, c.dir, "direction", l.id, "trop tard")
        .await
        .unwrap_err();
    assert!(conflit(&e), "{e:?}");
    let e = c
        .ent
        .reprise_decider(
            c.etab,
            c.dir,
            "direction",
            DemandeDecision {
                proposition_id: autre.id,
                action: ActionDecision::Ignorer,
                cible_id: None,
                motif: Some("test".into()),
            },
        )
        .await
        .unwrap_err();
    assert!(conflit(&e), "{e:?}");
}

#[tokio::test]
async fn r3_classes_synonyme_valide_rejoue_puis_retire() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    // Année courante et une classe dont le libellé équivaut à celui de l'export (« class_name-0 »).
    let annee = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante) VALUES ($1, $2, '2026-2027', '2026-09-01', '2027-07-04', true)")
        .bind(annee)
        .bind(c.etab)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let classe = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle) VALUES ($1, $2, $3, 'CN0', 'Class Name 0')")
        .bind(classe)
        .bind(c.etab)
        .bind(annee)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let cible = charger_cible(c.ent.pool(), c.etab).await.unwrap();
    assert_eq!(cible.classes.len(), 1);
    assert_eq!(cible.annees.len(), 1);
    let r1 = c.simuler().await;
    let l = c
        .file(Some("classe_site_mapping"), Some(EtatFile::ADecider))
        .await
        .lignes
        .into_iter()
        .find(|l| l.candidats.first().is_some_and(|k| k.cible_id == classe))
        .expect("la classe est proposée sur son libellé");
    assert!(l.candidats[0]
        .raisons
        .iter()
        .any(|r| r.code == "libelle_identique"));
    let avant = r1.simulation.par_objet["classe_site_mapping"].a_decider;
    c.ent
        .reprise_decider(
            c.etab,
            c.dir,
            "direction",
            DemandeDecision {
                proposition_id: l.id,
                action: ActionDecision::Rattacher,
                cible_id: Some(classe),
                motif: None,
            },
        )
        .await
        .expect("rattacher la classe");
    // L'équivalence est mémorisée, listée, et rejouée.
    let syn = c
        .ent
        .reprise_synonymes_lister(c.etab, c.dir, "direction")
        .await
        .unwrap();
    assert_eq!(syn.len(), 1);
    assert_eq!(
        (syn[0].domaine.as_str(), syn[0].cible_id, syn[0].valide_par),
        ("classe", classe, Some(c.dir))
    );
    assert_eq!(
        charger_cible(c.ent.pool(), c.etab)
            .await
            .unwrap()
            .synonymes
            .len(),
        1
    );
    let r2 = c.simuler().await;
    assert_eq!(
        r2.simulation.par_objet["classe_site_mapping"].a_decider,
        avant - 1
    );
    // Retirer l'équivalence : plus rejouée.
    c.ent
        .reprise_synonyme_retirer(c.etab, c.dir, "direction", syn[0].id)
        .await
        .unwrap();
    assert!(c
        .ent
        .reprise_synonymes_lister(c.etab, c.dir, "direction")
        .await
        .unwrap()
        .is_empty());
    assert!(charger_cible(c.ent.pool(), c.etab)
        .await
        .unwrap()
        .synonymes
        .is_empty());
    assert!(saisie_invalide(
        &c.ent
            .reprise_synonyme_retirer(c.etab, c.dir, "direction", syn[0].id)
            .await
            .unwrap_err()
    ));
}

#[tokio::test]
async fn r3_cible_lue_pour_l_etablissement_seulement_et_zone_videe_purge_les_resumes() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    // Un élève d'un autre établissement ne doit jamais apparaître comme candidat.
    let autre = Uuid::new_v4();
    seed_eleve(
        c.ent.pool(),
        autre,
        &c.src.eleves[2].nom,
        &c.src.eleves[2].prenom,
        None,
        c.src.eleves[2].date_naissance,
        None,
    )
    .await;
    let cible = charger_cible(c.ent.pool(), c.etab).await.unwrap();
    assert_eq!(cible.eleves.len(), 2);
    assert!(cible
        .eleves
        .iter()
        .all(|e| e.id == c.eleve_ine.to_string() || e.id == c.eleve_nom.to_string()));
    let r = c.simuler().await;
    assert!(r
        .simulation
        .file
        .iter()
        .filter(|l| l.objet == "eleve")
        .all(|l| l.candidats.iter().all(
            |k| k.cible_id == c.eleve_ine.to_string() || k.cible_id == c.eleve_nom.to_string()
        )));
    // Vider la zone purge aussi les résumés minimisés de la file ; les raisons et décisions restent.
    let l = c.ligne_eleve_avec_candidat().await;
    assert!(l.source.nom.is_some());
    c.ent
        .reprise_decider(
            c.etab,
            c.dir,
            "direction",
            DemandeDecision {
                proposition_id: l.id,
                action: ActionDecision::Rattacher,
                cible_id: Some(l.candidats[0].cible_id),
                motif: None,
            },
        )
        .await
        .unwrap();
    crate::reprise_zone::vider_zone(c.ent.pool(), c.etab, r.execution.id)
        .await
        .unwrap();
    reprise::noter_zone_videe(c.ent.pool(), c.etab, r.execution)
        .await
        .unwrap();
    let apres = c.file(Some("eleve"), None).await;
    assert!(!apres.lignes.is_empty());
    for l in &apres.lignes {
        assert_eq!(
            l.source,
            Default::default(),
            "résumé de la fiche de l'export purgé"
        );
        for k in &l.candidats {
            assert_eq!(k.fiche, Default::default(), "résumé du candidat purgé");
            assert!(!k.raisons.is_empty(), "les raisons restent comme preuve");
        }
    }
    assert_eq!(compter(c.ent.pool(), c.etab, "rep_decision").await, 1);
}

#[tokio::test]
async fn r3_resume_de_la_file_sans_donnee_personnelle() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let r = c.simuler().await;
    let res = c
        .ent
        .reprise_file_resume(c.etab, c.dir, "direction")
        .await
        .unwrap();
    let total: i64 = res.iter().map(|l| l.nombre).sum();
    assert_eq!(total, r.simulation.file.len() as i64);
    assert!(res.iter().all(|l| !l.decidee));
    assert!(res
        .iter()
        .any(|l| l.objet == "eleve" && l.meilleure_confiance == "forte"));
    assert!(res
        .iter()
        .any(|l| l.objet == "eleve" && l.meilleure_confiance == "aucune"));
}
