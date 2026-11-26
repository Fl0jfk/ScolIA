//! Socle 27, **lot G3a, passe de correctifs** (contre-revue du 2026-11-25) : destruction réelle de la pièce
//! d'identité (E1), qualité du titulaire (E2), garde-fous entre identité et décision (M1), qualité d'un
//! demandeur sans compte et rapprochement du sujet (M2), compte du demandeur (M3), anti-abus du dépôt
//! public (M4), `sans_objet` (M5), information des titulaires (M6), faibles F1 à F8.
//!
//! Aucun test ne dépend de l'heure à laquelle il tourne (règle 33) : les dates comparées sont dérivées du
//! jour de l'établissement, jamais de l'heure réelle. Base exigée sous `CI` ou `ENT_EXIGER_BASE` (RG-D03).

use super::*;
use crate::droits_demandes::{
    normaliser_contact, DepotConnecte, DepotSansCompte, SaisieDemande, ORIENTER_GUICHET,
    PLAFOND_DEPOTS_SANS_COMPTE_JOUR, REFERENCE_SANS_NUMERO,
};
use chrono::Duration;
use ent_object::provider::{FaultyProvider, MemoryProvider, Pannes};
use ent_object::ObjectStore;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use uuid::Uuid;

const DIR: &str = "direction";
const ADF: &str = "administratif";
const PDF: &[u8] = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF\n";

/// Labo avec un magasin d'objets **en mémoire** dont on peut provoquer la panne de suppression.
async fn labo() -> Option<(PgEnt, LaboCast, Arc<Pannes>)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (f, pannes) = FaultyProvider::new(Arc::new(MemoryProvider::new()));
    let magasin = ObjectStore::with_provider(Arc::new(f));
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, move |e| {
        e.install_brain_p0();
        e.installer_objets(magasin);
    })
    .await;
    Some((ent, cast, pannes))
}

/// Dépôt au guichet d'une demande d'accès de la famille de Léa, par le compte du parent (qualité de
/// titulaire à vérifier), reçue le jour de l'établissement.
async fn saisie_parent(ent: &PgEnt, cast: &LaboCast) -> Uuid {
    let etab = cast.etablissement_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    ent.droits_saisir_au_guichet(
        etab,
        cast.direction.user_id,
        DIR,
        SaisieDemande {
            canal: "courrier",
            recue_le: jour,
            demandeur_compte_id: Some(cast.parent_lea.user_id),
            demandeur_personne_id: None,
            demandeur_nom_saisi: Some("Parent de Léa"),
            demandeur_contact: Some("courrier postal"),
            qualite: "titulaire_autorite",
            sujet_type: "eleve",
            sujet_id: Some(cast.eleve_lea_id),
            sujet_nom_saisi: None,
            droits: &["acces"],
            perimetre_texte: None,
            perimetre_du: None,
            perimetre_au: None,
        },
    )
    .await
    .expect("saisie")
    .id
}

async fn ma_personne(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    sqlx::query_scalar(
        "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
    )
    .bind(etab)
    .bind(user)
    .fetch_one(ent.pool())
    .await
    .unwrap()
}

/// Une pièce d'identité du coffre (socle 7), avec ses deux objets (reçu et servi) réellement posés au
/// magasin. Rend l'identifiant de la pièce et ses deux clés.
pub(crate) async fn piece_identite(ent: &PgEnt, cast: &LaboCast) -> (Uuid, String, String) {
    let etab = cast.etablissement_id;
    let personne = ma_personne(ent, etab, cast.parent_lea.user_id).await;
    let original = ObjectStore::key_for(etab, "documents", Uuid::new_v4(), "pdf");
    let servi = ObjectStore::key_for(etab, "documents", Uuid::new_v4(), "pdf");
    let info = ent
        .stocker_objet(etab, &original, Some("application/pdf"), PDF)
        .await
        .unwrap();
    ent.stocker_objet(etab, &servi, Some("application/pdf"), PDF)
        .await
        .unwrap();
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_doc_piece
             (id, etablissement_id, type_code, personne_id, deposant_user_id, deposant_role, etat,
              cle_objet_original, sha256_original, taille_originale, mime_original,
              cle_objet_servi, sha256_servi, nb_pages)
         VALUES ($1, $2, 'piece_identite', $3, $4, 'administratif', 'acceptee', $5, $6, $7,
                 'application/pdf', $8, $6, 1)",
    )
    .bind(id)
    .bind(etab)
    .bind(personne)
    .bind(cast.administratif.user_id)
    .bind(&original)
    .bind(&info.sha256)
    .bind(PDF.len() as i64)
    .bind(&servi)
    .execute(ent.pool())
    .await
    .expect("pièce d'identité du coffre");
    (id, original, servi)
}

async fn objet_supprime_au_registre(ent: &PgEnt, etab: Uuid, cle: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT supprime_le IS NOT NULL FROM ent_objet WHERE etablissement_id = $1 AND cle_objet = $2",
    )
    .bind(etab)
    .bind(cle)
    .fetch_one(ent.pool())
    .await
    .unwrap()
}

fn evenements(v: &crate::droits_demandes::DemandeVue) -> Vec<String> {
    v.journal.iter().map(|l| l.evenement.clone()).collect()
}

// ─────────────────────────────── E1 : destruction réelle de la pièce ───────────────────────────────

/// E1 — la clôture détruit **réellement** la pièce : l'objet reçu et l'objet servi ont disparu du magasin
/// et sont marqués supprimés au registre ; « détruite » n'est journalisé qu'après cette confirmation.
#[tokio::test]
async fn g3a_c_e1_la_cloture_detruit_l_objet_du_magasin() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let id = saisie_parent(&ent, &cast).await;
    let (piece, original, servi) = piece_identite(&ent, &cast).await;
    assert!(ent.objet_existe(&original).await.unwrap());
    assert!(ent.objet_existe(&servi).await.unwrap());
    ent.droits_verifier_identite(etab, dir, DIR, id, "piece", true, None, Some(piece))
        .await
        .expect("vérification par pièce");
    ent.droits_decider(etab, dir, DIR, id, "acces", "accorde", Some("art15"), None)
        .await
        .unwrap();
    let v = ent
        .droits_clore(etab, dir, DIR, id, "extrait remis en main propre")
        .await
        .expect("clôture");
    assert_eq!(v.etat, "close");
    assert!(
        !ent.objet_existe(&original).await.unwrap(),
        "l'objet reçu a disparu du magasin"
    );
    assert!(
        !ent.objet_existe(&servi).await.unwrap(),
        "l'objet servi a disparu du magasin"
    );
    assert!(objet_supprime_au_registre(&ent, etab, &original).await);
    assert!(objet_supprime_au_registre(&ent, etab, &servi).await);
    assert!(!v.piece_a_detruire && !v.destruction_en_attente);
    let mention = v.identites.iter().find(|i| i.mode == "piece").unwrap();
    assert!(mention.piece_detruite_le.is_some() && !mention.piece_detenue);
    // L'ordre du journal : la demande précède la confirmation.
    let j = evenements(&v);
    let demandee = j
        .iter()
        .position(|e| e == "piece_destruction_demandee")
        .unwrap();
    let detruite = j.iter().position(|e| e == "piece_detruite").unwrap();
    assert!(demandee < detruite, "{j:?}");
    let close = v.journal.iter().find(|l| l.evenement == "close").unwrap();
    assert!(
        !close.detail.as_deref().unwrap_or("").contains("détruite"),
        "la clôture ne dit pas « détruite » : elle demande la destruction"
    );
}

/// E1 — le magasin est en panne : la demande se clôt, mais la pièce reste référencée, la fiche dit
/// « destruction en attente », l'objet existe encore ; la tâche de reprise la détruit quand le magasin
/// revient, et seulement alors « détruite » est journalisé.
#[tokio::test]
async fn g3a_c_e1_destruction_en_attente_puis_reprise_par_la_tache() {
    let Some((ent, cast, pannes)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let id = saisie_parent(&ent, &cast).await;
    let (piece, original, servi) = piece_identite(&ent, &cast).await;
    ent.droits_verifier_identite(etab, dir, DIR, id, "piece", true, None, Some(piece))
        .await
        .unwrap();
    ent.droits_decider(etab, dir, DIR, id, "acces", "accorde", Some("art15"), None)
        .await
        .unwrap();
    pannes.echec_suppression.store(true, SeqCst);
    let v = ent
        .droits_clore(etab, dir, DIR, id, "extrait remis en main propre")
        .await
        .expect("la clôture ne dépend pas du magasin");
    assert_eq!(v.etat, "close");
    assert!(v.destruction_en_attente, "visible : destruction en attente");
    assert!(v.piece_a_detruire);
    assert!(
        ent.objet_existe(&original).await.unwrap(),
        "l'objet est encore là"
    );
    let j = evenements(&v);
    assert!(
        j.contains(&"piece_destruction_en_attente".to_string()),
        "{j:?}"
    );
    assert!(
        !j.contains(&"piece_detruite".to_string()),
        "jamais « détruite » avant confirmation : {j:?}"
    );
    let tentatives: i32 = sqlx::query_scalar(
        "SELECT piece_destruction_tentatives FROM rgp_demande_identite WHERE demande_id = $1",
    )
    .bind(id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(tentatives >= 1, "les tentatives sont comptées");
    // Une seconde passe en panne ne remplit pas le journal.
    assert_eq!(ent.droits_destruction_pieces_tache().await.unwrap(), 0);
    let v = ent.droits_demande(etab, dir, DIR, id).await.unwrap();
    let nb_attente = v
        .journal
        .iter()
        .filter(|l| l.evenement == "piece_destruction_en_attente")
        .count();
    assert_eq!(nb_attente, 1, "une seule ligne de journal pour la panne");
    // Le magasin revient : la tâche détruit.
    pannes.echec_suppression.store(false, SeqCst);
    assert!(ent.droits_destruction_pieces_tache().await.unwrap() >= 1);
    let v = ent.droits_demande(etab, dir, DIR, id).await.unwrap();
    assert!(!v.destruction_en_attente && !v.piece_a_detruire);
    assert!(!ent.objet_existe(&original).await.unwrap());
    assert!(!ent.objet_existe(&servi).await.unwrap());
    assert!(evenements(&v).contains(&"piece_detruite".to_string()));
    // Idempotent : plus rien à faire.
    assert_eq!(ent.droits_destruction_pieces_tache().await.unwrap(), 0);
}

/// E1 et F8 — `piece_document_id` est validé : pièce de l'établissement (404 neutre sinon), de type
/// « pièce d'identité », jamais déjà rattachée, et dont aucun objet n'est partagé avec un autre dossier.
#[tokio::test]
async fn g3a_c_f8_la_piece_est_validee() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let id = saisie_parent(&ent, &cast).await;
    // Identifiant inventé : introuvable.
    let r = ent
        .droits_verifier_identite(
            etab,
            dir,
            DIR,
            id,
            "piece",
            true,
            None,
            Some(Uuid::new_v4()),
        )
        .await;
    assert!(matches!(
        r,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Un document qui n'est pas une pièce d'identité.
    let (piece, original, servi) = piece_identite(&ent, &cast).await;
    let autre = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_doc_piece
             (id, etablissement_id, type_code, eleve_id, deposant_user_id, deposant_role, etat,
              cle_objet_original, sha256_original, taille_originale, mime_original,
              cle_objet_servi, sha256_servi, nb_pages)
         SELECT $1, etablissement_id, 'attestation_assurance', $3, deposant_user_id, deposant_role, etat,
                cle_objet_original, sha256_original, taille_originale, mime_original,
                cle_objet_servi, sha256_servi, nb_pages
           FROM ent_doc_piece WHERE etablissement_id = $2 AND id = $4",
    )
    .bind(autre)
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(piece)
    .execute(ent.pool())
    .await
    .expect("pièce d'un autre type, mêmes objets");
    // Même objet qu'une autre pièce : refusé (la détruire priverait l'autre dossier de son fichier) ; et
    // le type est refusé avant, pour la pièce d'assurance.
    let r = ent
        .droits_verifier_identite(etab, dir, DIR, id, "piece", true, None, Some(autre))
        .await;
    assert!(
        matches!(r, Err(EntDbError::SaisieInvalide(_))),
        "type : {r:?}"
    );
    let r = ent
        .droits_verifier_identite(etab, dir, DIR, id, "piece", true, None, Some(piece))
        .await;
    assert!(
        matches!(r, Err(EntDbError::ConflitDossier(_))),
        "objet partagé : {r:?}"
    );
    assert!(
        ent.objet_existe(&original).await.unwrap(),
        "rien n'a été détruit"
    );
    assert!(ent.objet_existe(&servi).await.unwrap());
    // Une pièce propre est acceptée, une seule fois.
    let (propre, _, _) = piece_identite(&ent, &cast).await;
    ent.droits_verifier_identite(etab, dir, DIR, id, "piece", true, None, Some(propre))
        .await
        .expect("pièce valide");
    let id2 = saisie_parent(&ent, &cast).await;
    let r = ent
        .droits_verifier_identite(etab, dir, DIR, id2, "piece", true, None, Some(propre))
        .await;
    assert!(
        matches!(r, Err(EntDbError::ConflitDossier(_))),
        "déjà rattachée"
    );
    // Une pièce d'un autre établissement n'est pas visible : introuvable.
    // Une référence hors du mode « pièce » est refusée.
    let r = ent
        .droits_verifier_identite(
            etab,
            dir,
            DIR,
            id2,
            "code_courriel",
            true,
            None,
            Some(propre),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))));
}

// ───────────────────── E2 et M1 : qualité du titulaire, garde-fous de décision ─────────────────────

/// E2 — un parent dont le lien n'emporte pas l'autorité parentale, ou dont le dossier est protégé, ne
/// dépose **jamais** comme titulaire : même refus neutre que pour un élève inexistant (aucun motif ne
/// révèle une protection) ; au guichet, sa qualité n'est jamais « identité vérifiée » mais **à arbitrer**.
#[tokio::test]
async fn g3a_c_e2_sans_autorite_ou_restreint_jamais_verifie() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let adf = cast.administratif.user_id;
    let parent = cast.parent_lea.user_id;
    let depot = |sujet: Uuid| DepotConnecte {
        qualite: "titulaire_autorite",
        sujet_type: "eleve",
        sujet_id: Some(sujet),
        droits: &["acces"],
        perimetre_texte: None,
        perimetre_du: None,
        perimetre_au: None,
    };
    // Témoin : sujet inexistant.
    let inexistant = ent
        .droits_deposer_connecte(etab, parent, "parent", depot(Uuid::new_v4()))
        .await
        .expect_err("sujet inexistant");
    // Cas 1 : lien sans exercice de l'autorité parentale.
    sqlx::query(
        "UPDATE ent_eleve_lien SET autorite_parentale = false
          WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
    )
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(parent)
    .execute(ent.pool())
    .await
    .unwrap();
    let sans_autorite = ent
        .droits_deposer_connecte(etab, parent, "parent", depot(cast.eleve_lea_id))
        .await
        .expect_err("sans autorité parentale");
    assert_eq!(format!("{sans_autorite:?}"), format!("{inexistant:?}"));
    assert!(matches!(&sans_autorite, EntDbError::SaisieInvalide(m) if *m == ORIENTER_GUICHET));
    // Au guichet : identifié, mais qualité à arbitrer.
    let id = saisie_parent(&ent, &cast).await;
    let v = ent
        .droits_verifier_identite(etab, dir, DIR, id, "code_courriel", true, None, None)
        .await
        .unwrap();
    assert_eq!(v.etat, "identite_a_verifier", "bloquée");
    assert_eq!(v.identite_arbitrage_code.as_deref(), Some("sans_autorite"));
    assert!(v.identites.iter().all(|i| i.resultat == "a_arbitrer"
        && i.qualite_constatee.as_deref() == Some("confirmee_sans_autorite")));
    assert!(!v.identites.iter().any(|i| i.resultat == "verifiee"));
    assert!(evenements(&v).contains(&"identite_a_arbitrer".to_string()));
    let t = ent.droits_tableau(etab, dir, DIR, false).await.unwrap();
    assert!(t.identite_a_arbitrer >= 1);
    // Cas 2 : autorité rétablie, mais dossier protégé : restreinte, même refus neutre.
    sqlx::query(
        "UPDATE ent_eleve_lien SET autorite_parentale = true
          WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
    )
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(parent)
    .execute(ent.pool())
    .await
    .unwrap();
    let ok = ent
        .droits_deposer_connecte(etab, parent, "parent", depot(cast.eleve_lea_id))
        .await
        .expect("autorité confirmée : le dépôt passe");
    assert!(ok.identites.iter().any(|i| i.resultat == "verifiee"));
    let personne = ma_personne(&ent, etab, parent).await;
    sqlx::query(
        "INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine)
         VALUES (gen_random_uuid(), $1, $2, 'enc:v1:x', 'saisie')",
    )
    .bind(etab)
    .bind(personne)
    .execute(ent.pool())
    .await
    .unwrap();
    let restreinte = ent
        .droits_deposer_connecte(etab, parent, "parent", depot(cast.eleve_lea_id))
        .await
        .expect_err("dossier protégé");
    assert_eq!(
        format!("{restreinte:?}"),
        format!("{inexistant:?}"),
        "aucun motif ne révèle la protection"
    );
    let id2 = saisie_parent(&ent, &cast).await;
    let v2 = ent
        .droits_verifier_identite(etab, dir, DIR, id2, "code_courriel", true, None, None)
        .await
        .unwrap();
    assert_eq!(v2.identite_arbitrage_code.as_deref(), Some("restreinte"));
    assert_eq!(v2.etat, "identite_a_verifier");

    // M1 : tant que la qualité n'est pas arbitrée, on n'accorde pas, on ne clôt pas une réponse accordée.
    let r = ent
        .droits_decider(etab, adf, ADF, id, "acces", "accorde", Some("art15"), None)
        .await;
    assert!(
        matches!(r, Err(EntDbError::ConflitDossier(_))),
        "secrétariat : {r:?}"
    );
    let r = ent
        .droits_decider(etab, dir, DIR, id, "acces", "accorde", Some("art15"), None)
        .await;
    assert!(
        matches!(r, Err(EntDbError::ConflitDossier(_))),
        "direction sans motif écrit : {r:?}"
    );
    // L'arbitrage n'appartient qu'à la direction, avec un motif.
    assert!(matches!(
        ent.droits_arbitrer_identite(etab, adf, ADF, id, true, "j'ai vu le jugement")
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.droits_arbitrer_identite(etab, dir, DIR, id, true, "")
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Refus d'arbitrage : l'identité est close en échec, la demande reste bloquée.
    let v = ent
        .droits_arbitrer_identite(etab, dir, DIR, id2, false, "jugement non produit")
        .await
        .unwrap();
    assert_eq!(v.etat, "identite_a_verifier");
    assert!(v.identite_arbitrage_code.is_none());
    assert!(v.identites.iter().all(|i| i.resultat == "echouee"));
    assert!(matches!(
        ent.droits_arbitrer_identite(etab, dir, DIR, id2, true, "encore")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Acceptation : la qualité est reconnue sur motif, l'instruction reprend, la décision passe.
    let v = ent
        .droits_arbitrer_identite(etab, dir, DIR, id, true, "jugement vu, autorité confirmée")
        .await
        .unwrap();
    assert_eq!(v.etat, "en_cours");
    assert!(v.identites.iter().any(|i| i.resultat == "verifiee"));
    let j = evenements(&v);
    assert!(j.contains(&"identite_arbitree".to_string()), "{j:?}");
    ent.droits_decider(etab, adf, ADF, id, "acces", "accorde", Some("art15"), None)
        .await
        .expect("identité et qualité établies : le secrétariat peut accorder");
}

/// M1 — la clôture d'une réponse accordée exige une identité établie ; sans elle, seule la direction clôt
/// (le résumé tient lieu de motif), et le passage outre est journalisé. Une demande entièrement refusée,
/// elle, se clôt sans identité (« nous ne pouvons pas vous identifier » est une réponse).
#[tokio::test]
async fn g3a_c_m1_cloture_et_decision_sans_identite() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let adf = cast.administratif.user_id;
    // Demande sans identité vérifiée.
    let id = saisie_parent(&ent, &cast).await;
    // La direction passe outre à l'accord avec un motif écrit : tracé.
    let v = ent
        .droits_decider(
            etab,
            dir,
            DIR,
            id,
            "acces",
            "accorde",
            Some("art15"),
            Some("identité vue en main propre, saisie à compléter"),
        )
        .await
        .expect("la direction peut passer outre, motif écrit");
    assert!(evenements(&v).contains(&"decision_sans_identite".to_string()));
    // Le secrétariat ne clôt pas une réponse accordée sans identité.
    assert!(matches!(
        ent.droits_clore(etab, adf, ADF, id, "extrait remis").await,
        Err(EntDbError::ConflitDossier(_))
    ));
    let v = ent
        .droits_clore(
            etab,
            dir,
            DIR,
            id,
            "extrait remis après contrôle de la direction",
        )
        .await
        .expect("la direction clôt");
    assert!(
        evenements(&v)
            .iter()
            .filter(|e| *e == "decision_sans_identite")
            .count()
            >= 2
    );
    // Demande entièrement refusée : close sans identité.
    let id2 = saisie_parent(&ent, &cast).await;
    ent.droits_decider(
        etab,
        dir,
        DIR,
        id2,
        "acces",
        "refuse",
        Some("art12_6"),
        Some("identité non établie à partir des éléments reçus"),
    )
    .await
    .unwrap();
    let v = ent
        .droits_clore(etab, adf, ADF, id2, "réponse motivée envoyée")
        .await
        .expect("un refus pour identité non établie se clôt");
    assert_eq!(v.etat, "close");
}

// ─────────────────────────────── M2 : qualité sans compte, rapprochement ───────────────────────────────

/// M2 — un demandeur sans compte n'a pas une qualité « sans objet » mais **non vérifiable** ; le
/// rapprochement du sujet (`inconnu` → élève) et de la fiche du demandeur est tracé, et la qualité se
/// constate alors par le lien de la fiche rapprochée.
#[tokio::test]
async fn g3a_c_m2_qualite_non_verifiable_puis_rapprochement_trace() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let adf = cast.administratif.user_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    // Parent sans compte : saisie au guichet, sujet inconnu, aucune fiche.
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            SaisieDemande {
                canal: "guichet",
                recue_le: jour,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Un parent venu au guichet"),
                demandeur_contact: None,
                qualite: "titulaire_autorite",
                sujet_type: "inconnu",
                sujet_id: None,
                sujet_nom_saisi: Some("Léa"),
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .unwrap();
    let id = v.id;
    let v = ent
        .droits_verifier_identite(etab, dir, DIR, id, "piece", true, None, None)
        .await;
    assert!(
        matches!(v, Err(EntDbError::SaisieInvalide(_))),
        "pièce obligatoire"
    );
    let v = ent
        .droits_verifier_identite(
            etab,
            dir,
            DIR,
            id,
            "mandat",
            true,
            Some("mandat écrit vu"),
            None,
        )
        .await
        .unwrap();
    assert_eq!(v.identite_arbitrage_code.as_deref(), Some("non_verifiable"));
    assert!(v
        .identites
        .iter()
        .any(|i| i.qualite_constatee.as_deref() == Some("non_verifiable")));
    assert!(
        !v.identites
            .iter()
            .any(|i| i.qualite_constatee.as_deref() == Some("sans_objet")),
        "jamais « sans objet » pour un titulaire sans compte"
    );
    // Rapprochement : réservé au guichet, tracé, une fois.
    let r = ent
        .droits_rapprocher(
            etab,
            cast.prof.user_id,
            "enseignant",
            id,
            Some(("eleve", cast.eleve_lea_id)),
            None,
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)));
    assert!(matches!(
        ent.droits_rapprocher(etab, adf, ADF, id, None, None).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.droits_rapprocher(etab, adf, ADF, id, Some(("eleve", Uuid::new_v4())), None)
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    let personne = ma_personne(&ent, etab, cast.parent_lea.user_id).await;
    let v = ent
        .droits_rapprocher(
            etab,
            adf,
            ADF,
            id,
            Some(("eleve", cast.eleve_lea_id)),
            Some(personne),
        )
        .await
        .unwrap();
    assert_eq!(v.sujet_type, "eleve");
    assert_eq!(v.sujet_id, Some(cast.eleve_lea_id));
    assert!(evenements(&v).contains(&"sujet_rapproche".to_string()));
    assert!(matches!(
        ent.droits_rapprocher(etab, adf, ADF, id, Some(("eleve", cast.eleve_tom_id)), None)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Une nouvelle vérification constate maintenant le lien de la fiche rapprochée : confirmé.
    let v = ent
        .droits_verifier_identite(
            etab,
            dir,
            DIR,
            id,
            "mandat",
            true,
            Some("revue après rapprochement"),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        v.etat, "en_cours",
        "lien confirmé par la fiche rapprochée : {:?}",
        v.identites
    );
    assert!(v.identite_arbitrage_code.is_none());
}

// ─────────────────────────────── M3 : compte du demandeur ───────────────────────────────

/// M3 — `demandeur_compte_id` saisi au guichet est celui d'un **membre de l'établissement** : un compte
/// d'ailleurs est « introuvable », et aucun accusé ne lui part.
#[tokio::test]
async fn g3a_c_m3_compte_du_demandeur_membre_de_l_etablissement() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let etranger = Uuid::new_v4();
    let r = ent
        .droits_saisir_au_guichet(
            etab,
            cast.direction.user_id,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: jour,
                demandeur_compte_id: Some(etranger),
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Compte d'ailleurs"),
                demandeur_contact: None,
                qualite: "soi_meme",
                sujet_type: "inconnu",
                sujet_id: None,
                sujet_nom_saisi: Some("Quelqu'un"),
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await;
    assert!(matches!(
        r,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Le compteur n'a pas bougé : la saisie refusée n'a consommé aucun numéro.
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rgp_demande WHERE etablissement_id = $1 AND demandeur_compte_id = $2",
    )
    .bind(etab)
    .bind(etranger)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 0);
}

// ─────────────────────────────── M4 : dépôt public ───────────────────────────────

fn depot_public<'a>(contact: &'a str, nom: &'a str) -> DepotSansCompte<'a> {
    DepotSansCompte {
        nom_saisi: nom,
        contact,
        qualite: "ancien_eleve",
        sujet_nom_saisi: "Personne Introuvable",
        droits: &["acces"],
        perimetre_texte: None,
    }
}

/// M4 — contact normalisé (casse, espaces, étiquette « +xxx » : un seul contact), plafond quotidien par
/// établissement, et **pas de numéro officiel** tant que le guichet n'a pas confirmé le déposant : le spam
/// ne consomme ni numéro ni compteur.
#[tokio::test]
async fn g3a_c_m4_depot_public_normalise_plafonne_sans_numero_officiel() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let compter = |ent: &PgEnt| {
        let pool = ent.pool().clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM rgp_demande WHERE etablissement_id = $1 AND canal = 'en_ligne' AND demandeur_compte_id IS NULL",
            )
            .bind(etab)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    let compteur = |ent: &PgEnt| {
        let pool = ent.pool().clone();
        async move {
            sqlx::query_scalar::<_, Option<i32>>(
                "SELECT max(dernier) FROM rgp_demande_compteur WHERE etablissement_id = $1",
            )
            .bind(etab)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    let c0 = compter(&ent).await;
    let n0 = compteur(&ent).await;
    let m = ent
        .droits_deposer_sans_compte(
            etab,
            depot_public("Marie.Dupont@Exemple.test", "Marie Dupont"),
        )
        .await
        .unwrap();
    // Mêmes contact, autres écritures : même réponse ; M-b : chaque demande est **conservée**.
    for ecriture in [
        "marie.dupont@exemple.test",
        "  MARIE.DUPONT+spam@exemple.TEST ",
        "marie.dupont+2@exemple.test",
    ] {
        let m2 = ent
            .droits_deposer_sans_compte(etab, depot_public(ecriture, "Marie Dupont"))
            .await
            .unwrap();
        assert_eq!(m2, m, "même réponse invariable");
    }
    assert_eq!(
        compter(&ent).await,
        c0 + 4,
        "aucune demande abandonnée (M-b)"
    );
    assert_eq!(
        normaliser_contact("Marie.Dupont@Exemple.test"),
        normaliser_contact("marie.dupont+x@exemple.test")
    );
    // Pas de numéro officiel : le compteur de l'établissement n'a pas avancé.
    assert_eq!(
        compteur(&ent).await,
        n0,
        "le dépôt public ne consomme pas le compteur"
    );
    let t = ent.droits_tableau(etab, dir, DIR, false).await.unwrap();
    let sans_numero: Vec<_> = t
        .lignes
        .iter()
        .filter(|l| l.reference == REFERENCE_SANS_NUMERO)
        .collect();
    assert_eq!(sans_numero.len() as i64, 4);
    // Le guichet confirme (identité vérifiée) : le numéro est attribué, tracé.
    let id = sans_numero[0].id;
    // Pf7 : sans compte « pour soi », la vérification réussie exige un sujet rapproché d'un dossier.
    ent.droits_rapprocher(etab, dir, DIR, id, Some(("eleve", cast.eleve_lea_id)), None)
        .await
        .unwrap();
    let v = ent
        .droits_verifier_identite(etab, dir, DIR, id, "code_courriel", true, None, None)
        .await
        .unwrap();
    assert_ne!(
        v.reference, REFERENCE_SANS_NUMERO,
        "numéro attribué : {}",
        v.reference
    );
    assert!(evenements(&v).contains(&"numero_attribue".to_string()));
    assert_eq!(compteur(&ent).await, Some(n0.unwrap_or(0) + 1));
    // Une seconde vérification ne change pas le numéro.
    let v2 = ent
        .droits_verifier_identite(etab, dir, DIR, id, "code_courriel", true, None, None)
        .await
        .unwrap();
    assert_eq!(v2.reference, v.reference);
    assert_eq!(
        v2.journal
            .iter()
            .filter(|l| l.evenement == "numero_attribue")
            .count(),
        1
    );
    // Plafond quotidien par établissement : des contacts distincts jusqu'à la limite, puis refus neutre.
    let deja = compter(&ent).await;
    for i in 0..(PLAFOND_DEPOTS_SANS_COMPTE_JOUR - deja) {
        ent.droits_deposer_sans_compte(
            etab,
            depot_public(&format!("depot{i}@exemple.test"), "Un déposant"),
        )
        .await
        .unwrap_or_else(|e| panic!("dépôt {i} sous le plafond : {e:?}"));
    }
    let r = ent
        .droits_deposer_sans_compte(etab, depot_public("encore@exemple.test", "Un déposant"))
        .await;
    assert!(
        matches!(r, Err(EntDbError::ConflitDossier(_))),
        "au-delà du plafond : refus neutre, {r:?}"
    );
    assert_eq!(compter(&ent).await, PLAFOND_DEPOTS_SANS_COMPTE_JOUR);
}

// ─────────────────────────────── M5 : sans_objet ───────────────────────────────

/// M5 — « sans objet » est une décision d'arbitrage : `droits.arbitrer`, avec son motif ; jamais décidée
/// seule par le secrétariat, jamais sans motif (effacement et opposition compris).
#[tokio::test]
async fn g3a_c_m5_sans_objet_sous_arbitrage_et_motive() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let adf = cast.administratif.user_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: jour,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Famille Granger"),
                demandeur_contact: None,
                qualite: "titulaire_autorite",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                sujet_nom_saisi: None,
                droits: &["effacement", "opposition"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .unwrap();
    for droit in ["effacement", "opposition"] {
        // Le secrétariat ne décide pas « sans objet ».
        let r = ent
            .droits_decider(
                etab,
                adf,
                ADF,
                v.id,
                droit,
                "sans_objet",
                Some("hors_champ"),
                Some("rien à effacer"),
            )
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{droit} : {r:?}");
        // La direction non plus, sans motif.
        let r = ent
            .droits_decider(etab, dir, DIR, v.id, droit, "sans_objet", None, None)
            .await;
        assert!(
            matches!(r, Err(EntDbError::SaisieInvalide(_))),
            "{droit} : {r:?}"
        );
    }
    let v = ent
        .droits_decider(
            etab,
            dir,
            DIR,
            v.id,
            "opposition",
            "sans_objet",
            Some("hors_champ"),
            Some("la demande ne vise aucun traitement facultatif"),
        )
        .await
        .expect("la direction, avec motif");
    let d = v.droits.iter().find(|d| d.droit == "opposition").unwrap();
    assert_eq!(d.decision, "sans_objet");
    assert!(d.motif_texte.is_some());
}

// ─────────────────────────────── M6 : information des titulaires ───────────────────────────────

/// M6, CL27-04 — la clôture d'une demande d'élève mineur est **bloquée** tant que la décision d'informer
/// (ou non) ses titulaires n'est pas prise ; le compteur du tableau et le drapeau de la fiche le disent.
#[tokio::test]
async fn g3a_c_m6_cloture_bloquee_tant_que_les_titulaires_ne_sont_pas_decides() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let adf = cast.administratif.user_id;
    let eleve = cast
        .eleves
        .iter()
        .find(|e| e.compte.is_some())
        .expect("un élève avec un compte");
    let compte = eleve.compte.as_ref().unwrap().user_id;
    let v = ent
        .droits_deposer_connecte(
            etab,
            compte,
            "eleve",
            DepotConnecte {
                qualite: "eleve_mineur",
                sujet_type: "eleve",
                sujet_id: Some(eleve.id),
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .expect("dépôt de l'élève");
    assert!(
        v.titulaires_a_decider,
        "la fiche signale la décision à prendre"
    );
    let t = ent.droits_tableau(etab, dir, DIR, false).await.unwrap();
    assert!(t.titulaires_a_decider >= 1, "compteur au tableau");
    ent.droits_decider(
        etab,
        dir,
        DIR,
        v.id,
        "acces",
        "accorde",
        Some("art15"),
        None,
    )
    .await
    .unwrap();
    let r = ent
        .droits_clore(etab, dir, DIR, v.id, "extrait remis")
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    let r = ent
        .droits_refuser(etab, dir, DIR, v.id, "demande excessive")
        .await;
    assert!(
        matches!(r, Err(EntDbError::ConflitDossier(_))),
        "refus aussi : {r:?}"
    );
    ent.droits_titulaires_information(etab, adf, ADF, v.id, true, None)
        .await
        .unwrap();
    let t2 = ent.droits_tableau(etab, dir, DIR, false).await.unwrap();
    assert_eq!(t2.titulaires_a_decider, t.titulaires_a_decider - 1);
    let v = ent
        .droits_clore(etab, dir, DIR, v.id, "extrait remis")
        .await
        .expect("la décision est prise : la clôture passe");
    assert_eq!(v.etat, "close");
    assert!(!v.titulaires_a_decider);
}

// ─────────────────────────────── Faibles ───────────────────────────────

/// F2 — un accès suspendu ou fermé ne dépose pas.
#[tokio::test]
async fn g3a_c_f2_acces_suspendu_ou_ferme_ne_depose_pas() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let parent = cast.parent_lea.user_id;
    let depot = DepotConnecte {
        qualite: "titulaire_autorite",
        sujet_type: "eleve",
        sujet_id: Some(cast.eleve_lea_id),
        droits: &["acces"],
        perimetre_texte: None,
        perimetre_du: None,
        perimetre_au: None,
    };
    ent.droits_deposer_connecte(etab, parent, "parent", depot.clone())
        .await
        .expect("accès ouvert");
    for etat in ["suspendu", "ferme"] {
        sqlx::query(
            "UPDATE ent_membership SET etat = $3 WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etab)
        .bind(parent)
        .bind(etat)
        .execute(ent.pool())
        .await
        .unwrap();
        let r = ent
            .droits_deposer_connecte(etab, parent, "parent", depot.clone())
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{etat} : {r:?}");
    }
}

/// F3 — rouvrir une demande **refusée** remet en examen les droits que le refus général avait refusés ;
/// les décisions prises une à une sont conservées.
#[tokio::test]
async fn g3a_c_f3_reouverture_d_un_refus_remet_en_examen() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: jour,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Famille Granger"),
                demandeur_contact: None,
                qualite: "titulaire_autorite",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                sujet_nom_saisi: None,
                droits: &["acces", "effacement"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .unwrap();
    // Un droit est décidé un à un, l'autre reste en examen puis tombe avec le refus général.
    ent.droits_decider(
        etab,
        dir,
        DIR,
        v.id,
        "effacement",
        "refuse",
        Some("art17_3_b"),
        Some("obligation légale de conservation"),
    )
    .await
    .unwrap();
    let v = ent
        .droits_refuser(etab, dir, DIR, v.id, "demande manifestement excessive")
        .await
        .unwrap();
    assert_eq!(v.etat, "refusee");
    let acces = v.droits.iter().find(|d| d.droit == "acces").unwrap();
    assert_eq!(acces.decision, "refuse");
    let v = ent
        .droits_rouvrir(etab, dir, DIR, v.id, "réclamation du demandeur")
        .await
        .unwrap();
    assert_eq!(v.etat, "en_cours");
    assert!(
        v.refus_motif.is_none(),
        "le motif de refus ne reste pas sur une demande rouverte"
    );
    let acces = v.droits.iter().find(|d| d.droit == "acces").unwrap();
    assert_eq!(
        acces.decision, "en_examen",
        "à réexaminer, pas refusé d'avance"
    );
    assert!(acces.motif_code.is_none() && acces.decide_le.is_none());
    let effacement = v.droits.iter().find(|d| d.droit == "effacement").unwrap();
    assert_eq!(
        effacement.decision, "refuse",
        "la décision motivée prise droit par droit est conservée"
    );
    // On ne clôt pas tant que l'accès n'est pas réexaminé.
    assert!(matches!(
        ent.droits_clore(etab, dir, DIR, v.id, "réponse").await,
        Err(EntDbError::ConflitDossier(_))
    ));
}

/// F4 — le palier d'alerte se franchit, il ne se rencontre pas : une tâche arrêtée le jour exact ne fait pas
/// manquer l'alerte ; l'envoi reste unique par palier, et une échéance dépassée n'alerte plus.
#[tokio::test]
async fn g3a_c_f4_alerte_par_palier_franchi() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: jour - Duration::days(20),
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Famille Granger"),
                demandeur_contact: None,
                qualite: "titulaire_autorite",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                sujet_nom_saisi: None,
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .unwrap();
    let id = v.id;
    let caler = |jours: i64| {
        let pool = ent.pool().clone();
        async move {
            sqlx::query(
                "UPDATE rgp_demande SET echeance_le = $3 WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(jour + Duration::days(jours))
            .execute(&pool)
            .await
            .unwrap();
        }
    };
    let alertes = |ent: &PgEnt| {
        let pool = ent.pool().clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM rgp_demande_journal WHERE demande_id = $1 AND evenement = 'echeance_alertee'",
            )
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    // Échéance dans 20 jours : aucun palier atteint.
    caler(20).await;
    assert_eq!(ent.droits_alerter_echeances(etab).await.unwrap(), 0);
    // Dans 6 jours : la tâche n'a pas tourné à J-10 ; le palier J-10 est **franchi**, l'alerte part.
    caler(6).await;
    assert!(
        ent.droits_alerter_echeances(etab).await.unwrap() > 0,
        "J-10 franchi : alerte"
    );
    assert_eq!(alertes(&ent).await, 1);
    // Idempotent : rien de plus, rien au journal.
    assert_eq!(ent.droits_alerter_echeances(etab).await.unwrap(), 0);
    assert_eq!(alertes(&ent).await, 1);
    // Dans 2 jours : le palier J-3 est franchi à son tour, une seule fois.
    caler(2).await;
    assert!(
        ent.droits_alerter_echeances(etab).await.unwrap() > 0,
        "J-3 franchi : alerte"
    );
    assert_eq!(ent.droits_alerter_echeances(etab).await.unwrap(), 0);
    assert_eq!(alertes(&ent).await, 2);
    // Échéance dépassée : plus d'alerte (le retard est affiché, pas notifié en boucle).
    caler(-1).await;
    assert_eq!(ent.droits_alerter_echeances(etab).await.unwrap(), 0);
    assert_eq!(alertes(&ent).await, 2);
}

/// F5 — aucune logique de délai ne lit l'heure réelle : le tableau des demandes closes se calcule sur le
/// jour de l'établissement.
#[test]
fn g3a_c_f5_pas_d_heure_reelle_dans_la_logique_de_delai() {
    let src = include_str!("droits_demandes.rs");
    let logique = src.split("#[cfg(test)]").next().unwrap();
    // `now()` n'y sert plus qu'à **dater un fait** (journal, colonnes `*_le`), jamais à comparer un délai.
    for (i, l) in logique.lines().enumerate() {
        let l = l.trim();
        if l.contains("now() -") || l.contains("now() +") || l.contains("interval '") {
            panic!("ligne {} : calcul de délai sur l'heure réelle : {l}", i + 1);
        }
    }
}

// ─────────────────────────────── Contre-revue : M-a, M-b, M-c ───────────────────────────────

/// M-a — la pièce consultée lors d'une vérification **échouée** est rattachée aussi : la clôture la
/// détruit (R-RG16, « jamais gardée au-delà de la clôture »).
#[tokio::test]
async fn g3a_c_ma_la_piece_d_une_verification_echouee_est_detruite() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let id = saisie_parent(&ent, &cast).await;
    let (piece, original, servi) = piece_identite(&ent, &cast).await;
    let v = ent
        .droits_verifier_identite(etab, dir, DIR, id, "piece", false, None, Some(piece))
        .await
        .expect("vérification échouée, pièce consultée");
    assert!(v.piece_a_detruire, "la pièce est rattachée malgré l'échec");
    ent.droits_verifier_identite(etab, dir, DIR, id, "code_courriel", true, None, None)
        .await
        .unwrap();
    ent.droits_decider(etab, dir, DIR, id, "acces", "accorde", Some("art15"), None)
        .await
        .unwrap();
    let v = ent
        .droits_clore(etab, dir, DIR, id, "extrait remis")
        .await
        .unwrap();
    assert!(!ent.objet_existe(&original).await.unwrap());
    assert!(!ent.objet_existe(&servi).await.unwrap());
    assert!(evenements(&v).contains(&"piece_detruite".to_string()));
    // Une référence de pièce reste validée même en cas d'échec.
    let id2 = saisie_parent(&ent, &cast).await;
    let r = ent
        .droits_verifier_identite(
            etab,
            dir,
            DIR,
            id2,
            "piece",
            false,
            None,
            Some(Uuid::new_v4()),
        )
        .await;
    assert!(r.is_err(), "pièce inconnue refusée : {r:?}");
}

/// M-b — un second dépôt public distinct du même contact (autre droit, lendemain compris) n'est pas
/// abandonné : il est enregistré, la réponse reste identique.
#[tokio::test]
async fn g3a_c_mb_le_second_depot_public_distinct_est_conserve() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let compter = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM rgp_demande WHERE etablissement_id = $1 AND demandeur_contact_normalise = 'ancien@exemple.test'",
        )
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap()
    };
    let m1 = ent
        .droits_deposer_sans_compte(etab, depot_public("ancien@exemple.test", "Ancien Élève"))
        .await
        .unwrap();
    let mut second = depot_public("Ancien@exemple.test", "Ancien Élève");
    second.droits = &["effacement"];
    let m2 = ent.droits_deposer_sans_compte(etab, second).await.unwrap();
    assert_eq!(m1, m2, "même réponse");
    assert_eq!(compter().await, 2, "les deux demandes sont enregistrées");
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rgp_demande_droit d JOIN rgp_demande r ON r.id = d.demande_id
          WHERE r.etablissement_id = $1 AND r.demandeur_contact_normalise = 'ancien@exemple.test'
            AND d.droit = 'effacement'",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "le droit d'effacement n'est pas perdu");
}

/// M-c (CL27-04) — un élève mineur qui dépose en « soi-même » (qualité par défaut de l'écran), ou dont
/// le sujet est rapproché plus tard d'un mineur, est traité comme un mineur : la décision d'informer les
/// titulaires bloque la clôture.
#[tokio::test]
async fn g3a_c_mc_un_mineur_en_soi_meme_est_traite_comme_un_mineur() {
    let Some((ent, cast, _)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let eleve = cast
        .eleves
        .iter()
        .find(|e| e.compte.is_some())
        .expect("un élève avec un compte");
    let compte = eleve.compte.as_ref().unwrap().user_id;
    let v = ent
        .droits_deposer_connecte(
            etab,
            compte,
            "eleve",
            DepotConnecte {
                qualite: "soi_meme",
                sujet_type: "eleve",
                sujet_id: Some(eleve.id),
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .expect("dépôt de l'élève");
    assert!(v.titulaires_a_decider, "soi_meme d'un mineur : à décider");
    ent.droits_decider(
        etab,
        dir,
        DIR,
        v.id,
        "acces",
        "accorde",
        Some("art15"),
        None,
    )
    .await
    .unwrap();
    let r = ent
        .droits_clore(etab, dir, DIR, v.id, "extrait remis")
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    ent.droits_titulaires_information(etab, dir, DIR, v.id, false, Some("motif prudent"))
        .await
        .unwrap();
    // Dépôt public « ancien élève » rapproché ensuite d'un élève mineur.
    ent.droits_deposer_sans_compte(etab, depot_public("mineur@exemple.test", "Jeune Élève"))
        .await
        .unwrap();
    let id: Uuid = sqlx::query_scalar(
        "SELECT id FROM rgp_demande WHERE etablissement_id = $1 AND demandeur_contact_normalise = 'mineur@exemple.test'",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    let v = ent
        .droits_rapprocher(etab, dir, DIR, id, Some(("eleve", eleve.id)), None)
        .await
        .unwrap();
    assert!(v.titulaires_a_decider, "rapproché d'un mineur : à décider");
}
