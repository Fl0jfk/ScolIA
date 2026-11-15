//! Socle 27, lot G2 : registre généré, écarts, sous-traitants, mentions, page « Vos données », points ouverts,
//! état « prêt pour les données réelles », port de reprise. Base exigée sous `CI` ou `ENT_EXIGER_BASE` (RG-D03).
//!
//! La liste des sous-traitants de la plateforme est **globale** (sans établissement) : les tests qui la lisent
//! ou l'écrivent prennent un verrou consultatif de base de données (`nextest` lance un processus par test).

use super::*;
use crate::droits_repo::{Droit, RoleSaisie};
use crate::gouvernance_rgpd::{Gravite, SaisieSousTraitant};
use crate::registre_rgpd::TRAITEMENTS_BASE;
use chrono::Duration;
use ent_auth::{Action, Portee};
use sqlx::Row;
use uuid::Uuid;

const DIR: &str = "direction";

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
    })
    .await;
    Some((ent, cast))
}

/// Verrou consultatif tenu par une connexion détachée : libéré à la fermeture de la connexion (même en cas de panique).
struct Verrou(#[allow(dead_code)] sqlx::PgConnection);

async fn verrou(ent: &PgEnt) -> Verrou {
    let mut c = ent.pool().acquire().await.expect("connexion").detach();
    // `pg_try_advisory_lock` en boucle : un `pg_advisory_lock` bloquant dépasserait le délai d'attente des requêtes.
    for _ in 0..3000 {
        let pris: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(2710)")
            .fetch_one(&mut c)
            .await
            .expect("verrou");
        if pris {
            return Verrou(c);
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    panic!("verrou G2 non obtenu après 10 minutes");
}

async fn completer_tout(ent: &PgEnt, etab: Uuid, dir: Uuid) {
    for b in TRAITEMENTS_BASE {
        ent.completer_fiche(
            etab,
            dir,
            DIR,
            b.code,
            "mission_interet_public",
            None,
            "etablissement",
            None,
        )
        .await
        .unwrap_or_else(|e| panic!("{} : {e}", b.code));
    }
}

async fn sous_traitant_de_test(ent: &PgEnt, etab: Uuid) -> Uuid {
    let jour = ent
        .aujourdhui_etablissement(etab)
        .await
        .expect("jour de l'établissement");
    ent.declarer_soustraitant_plateforme(SaisieSousTraitant {
        nom: "Hébergeur de test",
        service: "Base de données",
        region: Some("Paris"),
        pays: "France",
        garanties: Some("accord de traitement signé"),
        accord_version: Some("v1"),
        accord_le: Some(jour),
        entree_le: jour - Duration::days(30),
        categories: &["fin.piece", "scol.dossier"],
    })
    .await
    .expect("sous-traitant")
}

async fn retirer_sous_traitant(ent: &PgEnt, id: Uuid) {
    sqlx::query("DELETE FROM rgp_soustraitant_objection WHERE annonce_id IN (SELECT id FROM rgp_soustraitant_annonce WHERE soustraitant_id = $1)")
        .bind(id)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("DELETE FROM rgp_soustraitant_annonce WHERE soustraitant_id = $1")
        .bind(id)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("DELETE FROM rgp_soustraitant_donnee WHERE soustraitant_id = $1")
        .bind(id)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("DELETE FROM rgp_soustraitant WHERE id = $1")
        .bind(id)
        .execute(ent.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn g2_registre_non_initialise_et_droits() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let _v = verrou(&ent).await;
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let r = ent.registre_genere(etab, dir, DIR).await.unwrap();
    assert_eq!(r.fiches.len(), TRAITEMENTS_BASE.len());
    assert!(
        r.fiches.iter().all(|f| f.etat == "non_initialise"),
        "jamais présenté comme rempli"
    );
    assert!(
        !r.synthese.to_lowercase().contains("est conforme"),
        "jamais « conforme » (R-RG9)"
    );
    // Les destinataires sont calculés (la direction lit les finances) même avant initialisation.
    let fin = r
        .fiches
        .iter()
        .find(|f| f.fiche.code == "finances_familles")
        .unwrap();
    assert!(fin
        .fiche
        .destinataires_internes
        .iter()
        .any(|d| d == "rôle direction"));
    assert!(!fin
        .fiche
        .destinataires_internes
        .iter()
        .any(|d| d == "rôle prof"));
    // Droits : ni l'administrateur technique, ni un professeur, ni une famille.
    for (u, role) in [
        (cast.admin.user_id, "admin"),
        (cast.prof.user_id, "prof"),
        (cast.parent_lea.user_id, "parent"),
    ] {
        assert!(
            matches!(
                ent.registre_genere(etab, u, role).await,
                Err(EntDbError::Forbidden)
            ),
            "{role}"
        );
        assert!(
            matches!(
                ent.initialiser_registre(etab, u, role).await,
                Err(EntDbError::Forbidden)
            ),
            "{role}"
        );
        assert!(
            matches!(
                ent.points_ouverts(etab, u, role).await,
                Err(EntDbError::Forbidden)
            ),
            "{role}"
        );
    }
    // Initialisation idempotente.
    assert_eq!(
        ent.initialiser_registre(etab, dir, DIR).await.unwrap(),
        TRAITEMENTS_BASE.len()
    );
    assert_eq!(ent.initialiser_registre(etab, dir, DIR).await.unwrap(), 0);
    let r = ent.registre_genere(etab, dir, DIR).await.unwrap();
    assert!(r.fiches.iter().all(|f| f.etat == "a_completer"));
    // État initial : pas prêt, avec les points bloquants attendus et le DPO absent en simple avertissement.
    let e = ent.points_ouverts(etab, dir, DIR).await.unwrap();
    assert!(!e.pret);
    let g = |c: &str| e.points.iter().find(|p| p.code == c).map(|p| p.gravite);
    assert_eq!(g("dpo_absent"), Some(Gravite::Avertissement));
    assert_eq!(g("catalogue_non_charge"), Some(Gravite::Bloquant));
    assert_eq!(g("fiches_a_completer"), Some(Gravite::Bloquant));
    assert_eq!(g("mention_non_publiee"), Some(Gravite::Bloquant));
    // Cloisonnement : un autre établissement n'a rien d'initialisé.
    let autre = ent.seed_labo_cast().await.unwrap();
    let r = ent
        .registre_genere(autre.etablissement_id, autre.direction.user_id, DIR)
        .await
        .unwrap();
    assert!(r.fiches.iter().all(|f| f.etat == "non_initialise"));
}

#[tokio::test]
async fn g2_completer_valider_et_detecter_les_ecarts() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let _v = verrou(&ent).await;
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    ent.initialiser_registre(etab, dir, DIR).await.unwrap();
    // Validation refusée tant qu'une fiche reste à compléter.
    assert!(matches!(
        ent.valider_registre(etab, dir, DIR).await,
        Err(EntDbError::ConflitDossierDetail(_))
    ));
    // Saisies invalides.
    for (base, motif, resp, lib) in [
        ("inconnue", None, "etablissement", None),
        ("consentement", None, "etablissement", None),
        ("mission_interet_public", None, "ogec", None),
        ("mission_interet_public", None, "tiers", None),
        ("consentement", Some("  "), "etablissement", None),
    ] {
        assert!(
            ent.completer_fiche(etab, dir, DIR, "finances_familles", base, motif, resp, lib)
                .await
                .is_err(),
            "{base}/{motif:?}/{resp}"
        );
    }
    assert!(ent
        .completer_fiche(
            etab,
            dir,
            DIR,
            "fiche_inconnue",
            "mission_interet_public",
            None,
            "etablissement",
            None
        )
        .await
        .is_err());
    // Un professeur ne peut ni compléter ni valider.
    assert!(matches!(
        ent.completer_fiche(
            etab,
            cast.prof.user_id,
            "prof",
            "finances_familles",
            "mission_interet_public",
            None,
            "etablissement",
            None
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    completer_tout(&ent, etab, dir).await;
    // La santé (module désactivé) n'empêche pas la validation : son traitement n'existe pas encore.
    let n = ent.valider_registre(etab, dir, DIR).await.unwrap();
    assert!(n >= TRAITEMENTS_BASE.len() - 1);
    let r = ent.registre_genere(etab, dir, DIR).await.unwrap();
    let fin = r
        .fiches
        .iter()
        .find(|f| f.fiche.code == "finances_familles")
        .unwrap();
    assert_eq!(fin.etat, "valide");
    assert_eq!(fin.version_validee, Some(1));
    // Aucun écart tant que rien ne change.
    assert!(ent
        .detecter_ecarts(etab, dir, DIR)
        .await
        .unwrap()
        .is_empty());
    // Un destinataire externe est ajouté : la fiche passe « à revoir » (écart de destinataire).
    ent.declarer_destinataire_externe(
        etab,
        dir,
        DIR,
        "finances_familles",
        "Trésorerie académique",
        "administration",
    )
    .await
    .unwrap();
    let e = ent.detecter_ecarts(etab, dir, DIR).await.unwrap();
    assert_eq!(
        e,
        vec![(
            "finances_familles".to_string(),
            "destinataire_ajoute".to_string()
        )]
    );
    // Idempotent.
    assert_eq!(ent.detecter_ecarts(etab, dir, DIR).await.unwrap().len(), 1);
    let r = ent.registre_genere(etab, dir, DIR).await.unwrap();
    let fin = r
        .fiches
        .iter()
        .find(|f| f.fiche.code == "finances_familles")
        .unwrap();
    assert_eq!(fin.etat, "a_revoir");
    assert_eq!(fin.ecarts_ouverts, vec!["destinataire_ajoute".to_string()]);
    assert!(fin
        .fiche
        .destinataires_externes
        .iter()
        .any(|d| d.contains("Trésorerie")));
    assert_eq!(r.nb_a_revoir, 1);
    // La validation est refusée tant que la fiche n'est pas revue, puis acceptée ; l'écart est clos.
    assert!(ent.valider_registre(etab, dir, DIR).await.is_err());
    ent.completer_fiche(
        etab,
        dir,
        DIR,
        "finances_familles",
        "mission_interet_public",
        None,
        "etablissement",
        None,
    )
    .await
    .unwrap();
    assert_eq!(ent.valider_registre(etab, dir, DIR).await.unwrap(), 1);
    assert!(ent
        .detecter_ecarts(etab, dir, DIR)
        .await
        .unwrap()
        .is_empty());
    let r = ent.registre_genere(etab, dir, DIR).await.unwrap();
    assert_eq!(
        r.fiches
            .iter()
            .find(|f| f.fiche.code == "finances_familles")
            .unwrap()
            .version_validee,
        Some(2)
    );
    // Un rôle de l'établissement qui lit les finances est un écart de destinataire, calculé sur les droits réels.
    let role = ent
        .creer_role_for_actor(
            etab,
            cast.admin.user_id,
            "admin",
            RoleSaisie {
                nom: format!("Régisseur {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::FacturationLire,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    let e = ent.detecter_ecarts(etab, dir, DIR).await.unwrap();
    assert!(
        e.contains(&(
            "finances_familles".to_string(),
            "destinataire_ajoute".to_string()
        )),
        "{e:?}"
    );
    let _ = role;
    // Activation du module santé : le traitement apparaît « à compléter » et bloque la prochaine validation.
    ent.activer_module_sante_test(etab).await;
    let r = ent.registre_genere(etab, dir, DIR).await.unwrap();
    let sante = r.fiches.iter().find(|f| f.fiche.code == "sante").unwrap();
    assert!(sante.fiche.actif);
    assert_ne!(sante.etat, "valide");
}

#[tokio::test]
async fn g2_traitement_hors_scolia_et_documents() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let _v = verrou(&ent).await;
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    ent.initialiser_registre(etab, dir, DIR).await.unwrap();
    let code = ent
        .ajouter_traitement_local(
            etab,
            dir,
            DIR,
            "Charlemagne (logiciel de vie scolaire)",
            "Gestion historique hors ScolIA",
        )
        .await
        .unwrap();
    assert!(code.starts_with("local_charlemagne"));
    assert!(
        ent.ajouter_traitement_local(
            etab,
            dir,
            DIR,
            "Charlemagne (logiciel de vie scolaire)",
            "autre"
        )
        .await
        .is_err(),
        "doublon"
    );
    assert!(ent
        .ajouter_traitement_local(etab, dir, DIR, " ", "x")
        .await
        .is_err());
    let r = ent.registre_genere(etab, dir, DIR).await.unwrap();
    assert_eq!(r.traitements_hors_scolia.len(), 1);
    // Le traitement local est validable avec le reste.
    completer_tout(&ent, etab, dir).await;
    ent.completer_fiche(
        etab,
        dir,
        DIR,
        &code,
        "mission_interet_public",
        None,
        "etablissement",
        None,
    )
    .await
    .unwrap();
    ent.valider_registre(etab, dir, DIR).await.unwrap();
    assert_eq!(
        ent.registre_genere(etab, dir, DIR)
            .await
            .unwrap()
            .traitements_hors_scolia[0]
            .etat,
        "valide"
    );
    // Documents : « sans objet » exige sa raison ; code inconnu refusé.
    assert!(ent
        .marquer_document(etab, dir, DIR, "aipd", "sans_objet", None)
        .await
        .is_err());
    assert!(ent
        .marquer_document(etab, dir, DIR, "aipd", "valide", Some("raison"))
        .await
        .is_err());
    assert!(ent
        .marquer_document(etab, dir, DIR, "inconnu", "valide", None)
        .await
        .is_err());
    ent.marquer_document(
        etab,
        dir,
        DIR,
        "aipd",
        "sans_objet",
        Some("module santé désactivé"),
    )
    .await
    .unwrap();
    ent.marquer_document(etab, dir, DIR, "registre", "valide", None)
        .await
        .unwrap();
    let docs = ent.documents_conformite(etab, dir, DIR).await.unwrap();
    assert_eq!(docs.iter().find(|d| d.0 == "aipd").unwrap().2, "sans_objet");
    assert_eq!(docs.iter().find(|d| d.0 == "registre").unwrap().2, "valide");
    assert_eq!(
        docs.iter().find(|d| d.0 == "procedure_droits").unwrap().2,
        "manquant"
    );
}

#[tokio::test]
async fn g2_sous_traitants_annonces_et_objection() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let _v = verrou(&ent).await;
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let st = sous_traitant_de_test(&ent, etab).await;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    // Catégorie inconnue refusée.
    assert!(ent
        .declarer_soustraitant_plateforme(SaisieSousTraitant {
            nom: "X",
            service: "Y",
            region: None,
            pays: "France",
            garanties: None,
            accord_version: None,
            accord_le: None,
            entree_le: jour,
            categories: &["inconnue.x"],
        })
        .await
        .is_err());
    // L'annonce respecte le délai d'objection de 30 jours.
    assert!(ent
        .annoncer_soustraitant(st, "changement", jour, jour + Duration::days(10))
        .await
        .is_err());
    let annonce = ent
        .annoncer_soustraitant(st, "changement", jour, jour + Duration::days(30))
        .await
        .unwrap();
    let v = ent.sous_traitants(etab, dir, DIR).await.unwrap();
    assert!(v
        .plateforme
        .iter()
        .any(|s| s.nom == "Hébergeur de test" && s.categories.contains(&"fin.piece".to_string())));
    let a = v
        .annonces
        .iter()
        .find(|a| a.id == annonce)
        .expect("annonce");
    assert!(a.objection_possible && !a.objection_deposee);
    // Objection : motif obligatoire, une seule par établissement, droit dédié.
    assert!(ent
        .objecter_annonce(etab, dir, DIR, annonce, " ")
        .await
        .is_err());
    assert!(matches!(
        ent.objecter_annonce(etab, cast.prof.user_id, "prof", annonce, "motif")
            .await,
        Err(EntDbError::Forbidden)
    ));
    ent.objecter_annonce(
        etab,
        dir,
        DIR,
        annonce,
        "Nous demandons des précisions sur l'hébergement",
    )
    .await
    .unwrap();
    assert!(ent
        .objecter_annonce(etab, dir, DIR, annonce, "encore")
        .await
        .is_err());
    assert!(
        ent.sous_traitants(etab, dir, DIR)
            .await
            .unwrap()
            .annonces
            .iter()
            .find(|a| a.id == annonce)
            .unwrap()
            .objection_deposee
    );
    // L'objection de A est invisible de B.
    let autre = ent.seed_labo_cast().await.unwrap();
    let vb = ent
        .sous_traitants(autre.etablissement_id, autre.direction.user_id, DIR)
        .await
        .unwrap();
    assert!(
        !vb.annonces
            .iter()
            .find(|a| a.id == annonce)
            .unwrap()
            .objection_deposee
    );
    assert!(ent
        .objecter_annonce(etab, dir, DIR, Uuid::new_v4(), "motif")
        .await
        .is_err());
    // Sous-traitant local.
    ent.declarer_soustraitant_local(
        etab,
        dir,
        DIR,
        "Cabinet comptable",
        "Paie",
        Some("Lyon"),
        false,
    )
    .await
    .unwrap();
    assert!(ent
        .declarer_soustraitant_local(etab, dir, DIR, "Cabinet comptable", "Paie", None, true)
        .await
        .is_err());
    assert!(ent
        .declarer_soustraitant_local(etab, dir, DIR, "  ", "Paie", None, true)
        .await
        .is_err());
    assert!(matches!(
        ent.declarer_soustraitant_local(
            etab,
            cast.prof.user_id,
            "prof",
            "Autre",
            "Service",
            None,
            true
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    let v = ent.sous_traitants(etab, dir, DIR).await.unwrap();
    assert_eq!(v.locaux.len(), 1);
    assert!(ent
        .sous_traitants(autre.etablissement_id, autre.direction.user_id, DIR)
        .await
        .unwrap()
        .locaux
        .is_empty());
    let e = ent.points_ouverts(etab, dir, DIR).await.unwrap();
    assert!(e.points.iter().any(
        |p| p.code == "soustraitants_locaux_sans_accord" && p.gravite == Gravite::Avertissement
    ));
    retirer_sous_traitant(&ent, st).await;
}

#[tokio::test]
async fn g2_un_sous_traitant_ajoute_est_un_ecart() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let _v = verrou(&ent).await;
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    ent.initialiser_registre(etab, dir, DIR).await.unwrap();
    completer_tout(&ent, etab, dir).await;
    ent.valider_registre(etab, dir, DIR).await.unwrap();
    assert!(ent
        .detecter_ecarts(etab, dir, DIR)
        .await
        .unwrap()
        .is_empty());
    let st = sous_traitant_de_test(&ent, etab).await;
    let e = ent.detecter_ecarts(etab, dir, DIR).await.unwrap();
    retirer_sous_traitant(&ent, st).await;
    assert!(
        e.contains(&(
            "finances_familles".to_string(),
            "soustraitant_change".to_string()
        )),
        "{e:?}"
    );
    assert!(
        e.contains(&(
            "scolarite_dossier".to_string(),
            "soustraitant_change".to_string()
        )),
        "{e:?}"
    );
    // Les traitements dont aucune catégorie n'est touchée restent valides.
    let r = ent.registre_genere(etab, dir, DIR).await.unwrap();
    assert_eq!(
        r.fiches
            .iter()
            .find(|f| f.fiche.code == "personnels")
            .unwrap()
            .etat,
        "valide"
    );
}

#[tokio::test]
async fn g2_mentions_vos_donnees_et_pret_pour_les_donnees_reelles() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let _v = verrou(&ent).await;
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let st = sous_traitant_de_test(&ent, etab).await;
    ent.initialiser_registre(etab, dir, DIR).await.unwrap();
    // « Vos données » avant toute publication : honnête, sans donnée d'autrui.
    let p = ent.vos_donnees(etab, "parent").await.unwrap();
    assert_eq!(p.public, "familles");
    assert!(!p.mention_publiee && p.avertissement.is_some());
    assert!(p.rubriques.iter().any(|r| r.titre.contains("Finances")));
    assert!(p.rubriques.iter().all(|r| !r
        .lignes
        .iter()
        .any(|l| l.to_lowercase().contains("conforme"))));
    assert!(ent.vos_donnees(etab, "role_inconnu").await.is_err());
    assert_eq!(
        ent.vos_donnees(etab, "eleve").await.unwrap().public,
        "eleves"
    );
    assert_eq!(
        ent.vos_donnees(etab, "prof").await.unwrap().public,
        "personnels"
    );
    // Les finances ne sont pas dans la page d'un élève.
    assert!(!ent
        .vos_donnees(etab, "eleve")
        .await
        .unwrap()
        .rubriques
        .iter()
        .any(|r| r.titre.contains("Finances")));
    // Publication refusée tant que le registre n'est pas validé.
    assert!(matches!(
        ent.publier_mention(etab, dir, DIR, "familles").await,
        Err(EntDbError::ConflitDossierDetail(_))
    ));
    assert!(ent
        .publier_mention(etab, dir, DIR, "inconnu")
        .await
        .is_err());
    completer_tout(&ent, etab, dir).await;
    ent.valider_registre(etab, dir, DIR).await.unwrap();
    // Un DPO nommé apparaît dans la mention ; sinon la mention le dit.
    let m = ent
        .mention_generee(etab, dir, DIR, "familles")
        .await
        .unwrap();
    assert!(m.sections[0]
        .lignes
        .iter()
        .any(|l| l.contains("Aucun délégué")));
    ent.designer_dpo(etab, dir, DIR, "Maître Exemple", "dpo@exemple.test", true)
        .await
        .unwrap();
    let m = ent
        .mention_generee(etab, dir, DIR, "familles")
        .await
        .unwrap();
    assert!(m.sections[0]
        .lignes
        .iter()
        .any(|l| l.contains("Maître Exemple") && l.contains("dpo@exemple.test")));
    assert!(
        m.sections
            .iter()
            .flat_map(|s| &s.lignes)
            .any(|l| l.contains("non validée")),
        "durées non validées signalées"
    );
    ent.retirer_dpo(etab, dir, DIR).await.unwrap();
    // Publication : version 1, idempotente ; droit `conformite.valider` exigé.
    assert!(matches!(
        ent.publier_mention(etab, cast.prof.user_id, "prof", "familles")
            .await,
        Err(EntDbError::Forbidden)
    ));
    let m1 = ent
        .publier_mention(etab, dir, DIR, "familles")
        .await
        .unwrap();
    assert_eq!(m1.version_publiee, Some(1));
    assert!(m1.a_jour);
    assert_eq!(
        ent.publier_mention(etab, dir, DIR, "familles")
            .await
            .unwrap()
            .version_publiee,
        Some(1),
        "même texte : même version"
    );
    ent.publier_mention(etab, dir, DIR, "personnels")
        .await
        .unwrap();
    let p = ent.vos_donnees(etab, "parent").await.unwrap();
    assert!(p.mention_publiee && p.avertissement.is_none());
    // État « prêt » : le DPO absent n'empêche rien ; seuls des avertissements restent.
    ent.charger_catalogue_conservation(etab, dir, DIR)
        .await
        .unwrap();
    let e = ent.points_ouverts(etab, dir, DIR).await.unwrap();
    assert!(
        e.pret,
        "points bloquants : {:?}",
        e.points
            .iter()
            .filter(|p| p.gravite == Gravite::Bloquant)
            .collect::<Vec<_>>()
    );
    assert!(e
        .points
        .iter()
        .any(|p| p.code == "dpo_absent" && p.gravite == Gravite::Avertissement));
    assert!(e.points.iter().any(|p| p.code == "regles_non_validees"));
    assert!(!e.texte.to_lowercase().contains("est conforme"));
    // Un changement du registre périme la mention et repasse l'état à « pas prêt ».
    ent.declarer_destinataire_externe(
        etab,
        dir,
        DIR,
        "scolarite_dossier",
        "Rectorat",
        "administration",
    )
    .await
    .unwrap();
    let e = ent.points_ouverts(etab, dir, DIR).await.unwrap();
    assert!(!e.pret);
    assert!(e.points.iter().any(|p| p.code == "mention_perimee"));
    retirer_sous_traitant(&ent, st).await;
}

#[tokio::test]
async fn g2_la_reprise_nextjs_est_un_port_non_branche() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let avant = |t: &'static str| {
        let ent = &ent;
        async move {
            sqlx::query(&format!(
                "SELECT count(*) AS n FROM {t} WHERE etablissement_id = $1"
            ))
            .bind(etab)
            .fetch_one(ent.pool())
            .await
            .unwrap()
            .get::<i64, _>("n")
        }
    };
    let (t0, d0) = (avant("rgp_traitement").await, avant("rgp_document").await);
    let r = ent.reprendre_rgpd(etab, dir, DIR, None).await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(m)) if m.contains("socle 28")));
    assert!(matches!(
        ent.reprendre_rgpd(etab, cast.prof.user_id, "prof", None)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert_eq!(
        (avant("rgp_traitement").await, avant("rgp_document").await),
        (t0, d0),
        "rien n'est écrit"
    );
}
