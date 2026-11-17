//! Socle 18, lot FO1 — objets d'accord, versions à usages, décisions, avis de l'élève, retrait, « mes accords »,
//! port `verifier`, reprise de 193 et de 275, droits et cloisonnement, contre la base locale de test
//! (`docs/socles/18_FORMULAIRES.md` §6–§10, §19, §27, §31). Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en
//! CI**. Avec `ENT_RLS_MODE=strict`, le code testé tourne sous un rôle **non propriétaire, sans BYPASSRLS** : une requête
//! qui oublie le contexte d'établissement ne voit rien.
//!
//! Cas limites de la conception : `ck01`, `ck02`, `ck03`, `ck04`, `ck05`, `ck06`, `ck07`, `ck08`, `ck10` (hors lot : FO3),
//! `ck15`, `ck16`, `ck17`, `ck18`, `ck19`, `ck21`, `ck22`, `ck30`, `ck31`, `ck32`, `ck35`, `ck36`.

use super::*;
use crate::accords::*;
use crate::tests_formulaires_fo0::{
    compte_eleve, droit_sortie_de_lea, il_y_a, labo, lire_naissance, objet_aleatoire, parent_en_plus, poser_naissance,
    retirer_parent,
};
use chrono::{Duration, Utc};
use std::pin::Pin;
use uuid::Uuid;

const DIR: &str = "direction";

/// Repart d'un état propre : les accords sont en ajout seul, la purge volontaire est le seul moyen de les vider.
async fn vider_accords(ent: &PgEnt, etab: Uuid) {
    // Les formulaires, réponses, désaccords et preuves des lots FO2 et FO3 référencent les objets : ils partent d'abord.
    crate::tests_formulaires_fo2::vider_fo23(ent, etab).await;
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.purge_etablissement', 'on', true)").execute(&mut *tx).await.unwrap();
    for t in [
        "ent_acc_decision_usage",
        "ent_acc_decision",
        "ent_acc_version_usage",
        "ent_acc_version",
        "ent_acc_usage",
        "ent_acc_journal",
        "ent_acc_objet",
    ] {
        sqlx::query(&format!("DELETE FROM {t} WHERE etablissement_id = $1")).bind(etab).execute(&mut *tx).await.unwrap();
    }
    tx.commit().await.unwrap();
}

fn usage(code: &str) -> NouvelUsage {
    NouvelUsage { code: code.into(), libelle: format!("Usage {code}"), mission: false }
}

fn objet_propre(code: &str, nature: &str, regle: &str, droit: Option<&str>, usages: &[&str]) -> NouvelObjet {
    NouvelObjet {
        code: code.into(),
        libelle: format!("Objet {code}"),
        nature: nature.into(),
        regle: regle.into(),
        avis_eleve_des_ans: if nature == "consentement" { Some(11) } else { None },
        droit_lien_code: droit.map(String::from),
        preuve_min: "S1".into(),
        validite: "annee_scolaire".into(),
        valide_jusqu_au: None,
        evenement_fin: None,
        sujet_type: "eleve".into(),
        desaccord_objet: None,
        usages: usages.iter().map(|u| usage(u)).collect(),
        texte: "Texte d'accord de test".into(),
    }
}

/// Crée un objet propre et publie sa version 1.
async fn objet_publie(ent: &PgEnt, etab: Uuid, dir: Uuid, n: NouvelObjet) -> ObjetAccord {
    let code = n.code.clone();
    ent.accords_creer_objet(etab, dir, DIR, n).await.unwrap();
    ent.accords_publier_version(etab, dir, DIR, &code, None).await.unwrap();
    ent.accords_objets(etab, dir, DIR).await.unwrap().into_iter().find(|o| o.code == code).unwrap()
}

fn choix(codes: &[&str], accorde: bool) -> Vec<ChoixUsage> {
    codes.iter().map(|c| ChoixUsage { code: (*c).into(), accorde }).collect()
}

fn saisie(objet: &str, eleve: Uuid, codes: &[&str], accorde: bool) -> DecisionSaisie {
    DecisionSaisie { objet_code: objet.into(), eleve_id: Some(eleve), usages: choix(codes, accorde) }
}

async fn verdict(ent: &PgEnt, etab: Uuid, objet: &str, usage: &str, eleve: Uuid) -> Verdict {
    let v = ent
        .accords_verifier(
            etab,
            &DemandeVerification { objet_code: objet.into(), usage_code: usage.into(), sujets: vec![Sujet::Eleve(eleve)], jour: None },
        )
        .await;
    assert_eq!(v.len(), 1);
    v[0].verdict
}

async fn nb_evenements(ent: &PgEnt, etab: Uuid, type_name: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = $2")
        .bind(etab)
        .bind(type_name)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

async fn nb_notifs_formulaire(ent: &PgEnt, etab: Uuid, user: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'formulaire'")
        .bind(etab)
        .bind(user)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

// ─── Catalogue, activation, publication, relecture (FRM-100 à FRM-109) ───────────────────────────────

#[tokio::test]
async fn catalogue_activation_publication_relecture_et_droits() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    vider_accords(&ent, etab).await;

    let catalogue = ent.accords_catalogue(etab, dir, DIR).await.unwrap();
    assert!(catalogue.len() >= 18 && catalogue.iter().any(|c| c.code == "droit_image" && c.regle_min == "tous" && !c.active));
    // Droits : ni un parent, ni un professeur, ni la comptabilité ne gèrent le catalogue.
    for (u, r) in [(cast.parent_lea.user_id, "parent"), (cast.prof.user_id, "prof"), (cast.compta.user_id, "compta")] {
        assert!(matches!(ent.accords_catalogue(etab, u, r).await, Err(EntDbError::Forbidden)), "{r}");
        assert!(matches!(ent.accords_activer_objet(etab, u, r, "droit_image").await, Err(EntDbError::Forbidden)), "{r}");
    }
    // Le secrétariat lit (autorisations.gerer) mais ne paramètre pas.
    let adm = cast.administratif.user_id;
    assert!(ent.accords_catalogue(etab, adm, "administratif").await.is_ok());
    assert!(matches!(ent.accords_activer_objet(etab, adm, "administratif", "droit_image").await, Err(EntDbError::Forbidden)));

    let o = ent.accords_activer_objet(etab, dir, DIR, "droit_image").await.unwrap();
    assert_eq!((o.regle.as_str(), o.nature.as_str(), o.version_publiee), ("tous", "consentement", None));
    assert!(o.texte_provisoire, "R-FC72 : texte de départ provisoire");
    assert!(matches!(ent.accords_activer_objet(etab, dir, DIR, "droit_image").await, Err(EntDbError::ConflitDossier(_))));
    assert!(matches!(ent.accords_activer_objet(etab, dir, DIR, "inexistant_xyz").await, Err(EntDbError::Domain(_))));
    // Version 1 en brouillon : aucun verdict `autorise`, l'appelant refuse (R-FC3).
    let harry = cast.eleve_lea_id;
    assert_eq!(verdict(&ent, etab, "droit_image", "fil_classe", harry).await, Verdict::Inconnu);
    // Rédiger : un usage inconnu ou un texte vide sont refusés ; publier v1 sans qualification.
    let usages: Vec<String> = ent.accords_usages(etab, dir, DIR, "droit_image").await.unwrap().into_iter().map(|u| u.code).collect();
    assert!(usages.contains(&"interne_personnels".to_string()) && usages.len() == 12);
    let r = ent
        .accords_rediger_version(etab, dir, DIR, "droit_image", RedactionVersion { texte: " ".into(), usages: usages.clone(), nouveaux_usages: vec![] })
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))));
    let v1 = ent.accords_publier_version(etab, dir, DIR, "droit_image", Some("fond")).await;
    assert!(matches!(v1, Err(EntDbError::SaisieInvalide(_))), "une première version n'a pas de qualification");
    let v1 = ent.accords_publier_version(etab, dir, DIR, "droit_image", None).await.unwrap();
    assert_eq!((v1.numero, v1.type_changement.clone(), v1.publiee_par), (1, None, Some(dir)));
    assert!(v1.texte_relu_le.is_none());
    // Plus de brouillon : rien à publier.
    assert!(matches!(ent.accords_publier_version(etab, dir, DIR, "droit_image", None).await, Err(EntDbError::ConflitDossier(_))));
    let relue = ent.accords_relire_version(etab, dir, DIR, "droit_image", 1).await.unwrap();
    assert!(relue.texte_relu_le.is_some());
    let o = ent.accords_objets(etab, dir, DIR).await.unwrap().into_iter().find(|o| o.code == "droit_image").unwrap();
    assert!(!o.texte_provisoire && o.version_publiee == Some(1));
    // Aperçu par profil.
    let (texte, usages_vus, provisoire) = ent.accords_apercu(etab, dir, DIR, "droit_image", "eleve").await.unwrap();
    assert!(texte.contains("avis") && !usages_vus.iter().any(|u| u.mission) && !provisoire);
    // Historique et journal.
    assert_eq!(ent.accords_historique(etab, dir, DIR, "droit_image").await.unwrap().len(), 1);
    assert!(nb_evenements(&ent, etab, "autorisations.objet_active").await >= 1);
    assert!(nb_evenements(&ent, etab, "autorisations.version_publiee").await >= 1);
    // Désactiver : les décisions restent, plus aucun usage n'est permis.
    ent.accords_desactiver_objet(etab, dir, DIR, "droit_image").await.unwrap();
    assert_eq!(verdict(&ent, etab, "droit_image", "fil_classe", harry).await, Verdict::Inconnu);
    assert!(matches!(ent.accords_desactiver_objet(etab, dir, DIR, "droit_image").await, Err(EntDbError::ConflitDossier(_))));
    ent.accords_activer_objet(etab, dir, DIR, "droit_image").await.unwrap();
    vider_accords(&ent, etab).await;
}

// ─── R-FC1, R-FC6, R-FC23 : création d'un objet propre ───────────────────────────────────────────────

#[tokio::test]
async fn creation_d_objet_regle_nature_et_sante() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    vider_accords(&ent, etab).await;
    let neuf = |code: &str| objet_propre(code, "autorisation", "un", Some("autoriser_sorties"), &["participation"]);
    let ok = ent.accords_creer_objet(etab, dir, DIR, neuf("atelier_theatre")).await.unwrap();
    assert_eq!((ok.nature.as_str(), ok.catalogue_code), ("autorisation", None));
    assert!(matches!(ent.accords_creer_objet(etab, dir, DIR, neuf("atelier_theatre")).await, Err(EntDbError::ConflitDossier(_))), "code unique");
    // Code, nature, règle, soi/personne, usages : refus nets.
    let mut m = neuf("Mauvais Code");
    assert!(matches!(ent.accords_creer_objet(etab, dir, DIR, m.clone()).await, Err(EntDbError::SaisieInvalide(_))));
    m = neuf("atelier_x");
    m.regle = "soi".into();
    assert!(matches!(ent.accords_creer_objet(etab, dir, DIR, m).await, Err(EntDbError::SaisieInvalide(_))), "soi ⇒ sujet personne");
    m = neuf("atelier_y");
    m.usages.clear();
    assert!(matches!(ent.accords_creer_objet(etab, dir, DIR, m).await, Err(EntDbError::SaisieInvalide(_))));
    // R-FC23 : aucune donnée de santé tant que le module santé est désactivé.
    for code in ["sante_allergies", "fiche_medicale", "religion_cantine"] {
        assert!(matches!(ent.accords_creer_objet(etab, dir, DIR, neuf(code)).await, Err(EntDbError::SaisieInvalide(_))), "{code}");
    }
    // R-FC1 : un consentement ne porte pas sur la mission ; une collecte ne porte pas sur le facultatif.
    let mut c = objet_propre("liste_appel_x", "consentement", "un", None, &["liste_appel"]);
    c.usages[0].mission = true;
    assert!(matches!(ent.accords_creer_objet(etab, dir, DIR, c).await, Err(EntDbError::SaisieInvalide(m)) if m.contains("mission")));
    let mut col = objet_propre("fiche_rens_x", "collecte", "un", None, &["a", "b"]);
    col.usages[0].mission = true;
    assert!(matches!(ent.accords_creer_objet(etab, dir, DIR, col).await, Err(EntDbError::SaisieInvalide(_))));
    // Une information ne se refuse pas : règle `tous` interdite.
    let mut i = objet_propre("charte_x", "information", "tous", None, &["lecture"]);
    assert!(matches!(ent.accords_creer_objet(etab, dir, DIR, i.clone()).await, Err(EntDbError::SaisieInvalide(_))));
    i.regle = "eleve_et_titulaires".into();
    assert!(ent.accords_creer_objet(etab, dir, DIR, i).await.is_ok());
    // R-FC6 : monter, jamais baisser sous la règle du catalogue (même code).
    let baisse = objet_propre("droit_image", "consentement", "un", Some("signer_documents"), &["fil_classe"]);
    assert!(matches!(ent.accords_creer_objet(etab, dir, DIR, baisse).await, Err(EntDbError::ConflitDossier(_))));
    let monte = objet_propre("voyage_france", "autorisation", "tous", Some("autoriser_sorties"), &["participation"]);
    let v = ent.accords_creer_objet(etab, dir, DIR, monte).await.unwrap();
    assert_eq!((v.regle.as_str(), v.catalogue_code.as_deref()), ("tous", Some("voyage_france")));
    vider_accords(&ent, etab).await;
}

// ─── CK-03, CK-04, CK-05 : droit à l'image, deux titulaires, avis de l'enfant ───────────────────────

#[tokio::test]
async fn ck03_ck04_ck05_image_deux_titulaires_et_avis_de_l_enfant() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_accords(&ent, etab).await;
    ent.accords_activer_objet(etab, dir, DIR, "droit_image").await.unwrap();
    ent.accords_publier_version(etab, dir, DIR, "droit_image", None).await.unwrap();
    let origine = lire_naissance(&ent, etab, harry).await;
    poser_naissance(&ent, etab, harry, Some(il_y_a(13)), origine.1.as_deref()).await;
    let pere = parent_en_plus(&ent, etab, harry, true, false).await;
    let enfant = compte_eleve(&ent, etab, harry).await;

    // Mission : le trombinoscope des personnels n'exige aucun accord.
    assert_eq!(verdict(&ent, etab, "droit_image", "interne_personnels", harry).await, Verdict::NonRequis);
    // Aucun accord : jamais autorisé.
    assert_eq!(verdict(&ent, etab, "droit_image", "fil_classe", harry).await, Verdict::EnAttente);
    // Un titulaire sur deux : en attente (règle `tous`, CK-03). Décision par usage, décideur calculé.
    let m = ent.accords_decider(etab, lea, saisie("droit_image", harry, &["fil_classe", "site_public"], true)).await.unwrap();
    assert_eq!((m.decision.as_str(), m.qualite.as_str(), m.version), ("accorde", "titulaire", 1));
    assert_eq!(verdict(&ent, etab, "droit_image", "fil_classe", harry).await, Verdict::EnAttente);
    // Idempotence : répéter la même décision n'écrit rien.
    let m2 = ent.accords_decider(etab, lea, saisie("droit_image", harry, &["fil_classe", "site_public"], true)).await.unwrap();
    assert_eq!(m2.decision, "inchangee");
    ent.accords_decider(etab, pere, saisie("droit_image", harry, &["fil_classe"], true)).await.unwrap();
    assert_eq!(verdict(&ent, etab, "droit_image", "fil_classe", harry).await, Verdict::Autorise);
    assert_eq!(verdict(&ent, etab, "droit_image", "site_public", harry).await, Verdict::EnAttente, "le père n'a pas accordé le site public");
    assert_eq!(verdict(&ent, etab, "droit_image", "presse", harry).await, Verdict::EnAttente, "usage jamais accordé");
    // CK-04 : l'enfant de 13 ans refuse par son propre compte, son avis l'emporte.
    let avis = ent.accords_decider(etab, enfant, saisie("droit_image", harry, &["fil_classe"], false)).await.unwrap();
    assert_eq!(avis.qualite, "eleve_avis");
    assert_eq!(verdict(&ent, etab, "droit_image", "fil_classe", harry).await, Verdict::Refuse);
    // R-FC51 : un parent ne donne jamais l'avis de l'enfant ; l'enfant ne signe pas à la place de ses parents.
    // (l'avis est une décision de qualité `eleve_avis`, jamais comptée comme celle d'un titulaire)
    let (n_avis,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_acc_decision WHERE etablissement_id = $1 AND qualite = 'eleve_avis' AND decideur_user_id = $2")
        .bind(etab).bind(enfant).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(n_avis, 1);
    // L'enfant revient sur son avis : le verdict redevient celui des titulaires.
    ent.accords_decider(etab, enfant, saisie("droit_image", harry, &["fil_classe"], true)).await.unwrap();
    assert_eq!(verdict(&ent, etab, "droit_image", "fil_classe", harry).await, Verdict::Autorise);
    // CK-05 : en dessous de l'âge de l'avis (11 ans), l'enfant ne décide pas.
    poser_naissance(&ent, etab, harry, Some(il_y_a(9)), origine.1.as_deref()).await;
    assert!(ent.accords_decider(etab, enfant, saisie("droit_image", harry, &["fil_classe"], false)).await.is_err());
    // Un tiers ne décide jamais pour l'enfant.
    assert!(ent.accords_decider(etab, cast.parent_tom.user_id, saisie("droit_image", harry, &["fil_classe"], true)).await.is_err());
    // Un usage de mission, un usage hors version : refus nets.
    assert!(matches!(ent.accords_decider(etab, lea, saisie("droit_image", harry, &["interne_personnels"], true)).await, Err(EntDbError::SaisieInvalide(_))));
    assert!(matches!(ent.accords_decider(etab, lea, saisie("droit_image", harry, &["inconnu_x"], true)).await, Err(EntDbError::SaisieInvalide(_))));
    poser_naissance(&ent, etab, harry, origine.0, origine.1.as_deref()).await;
    retirer_parent(&ent, etab, pere).await;
    vider_accords(&ent, etab).await;
}

// ─── CK-01, CK-02 : sortie, règle `un`, désaccord ; retrait immédiat ; CK-35 ─────────────────────────

#[tokio::test]
async fn ck01_ck02_ck35_sortie_desaccord_retrait_et_evenement_passe() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_accords(&ent, etab).await;
    droit_sortie_de_lea(&ent, &cast, true).await;
    let pere = parent_en_plus(&ent, etab, harry, true, true).await;
    let code = objet_aleatoire();
    objet_publie(&ent, etab, dir, objet_propre(&code, "autorisation", "un", Some("autoriser_sorties"), &["participation"])).await;

    // CK-01 : la mère accepte, le père ne répond pas : autorisé.
    ent.accords_decider(etab, lea, saisie(&code, harry, &["participation"], true)).await.unwrap();
    assert_eq!(verdict(&ent, etab, &code, "participation", harry).await, Verdict::Autorise);
    // CK-02 : le père refuse ensuite : désaccord, événement pour la direction (sans motif).
    ent.accords_decider(etab, pere, saisie(&code, harry, &["participation"], false)).await.unwrap();
    assert_eq!(verdict(&ent, etab, &code, "participation", harry).await, Verdict::Desaccord);
    assert!(!verdict(&ent, etab, &code, "participation", harry).await.permet());
    assert!(nb_evenements(&ent, etab, "autorisations.desaccord").await >= 1);
    // Retrait de la mère : effet immédiat, sans passkey. Il ne reste qu'un refus.
    let r = ent.accords_retirer(etab, lea, &code, Some(harry), None).await.unwrap();
    assert_eq!(r.decision, "retire");
    assert_eq!(verdict(&ent, etab, &code, "participation", harry).await, Verdict::Refuse);
    assert!(nb_evenements(&ent, etab, "autorisations.retire").await >= 1);
    // Rien à retirer une seconde fois.
    assert!(matches!(ent.accords_retirer(etab, lea, &code, Some(harry), None).await, Err(EntDbError::ConflitDossier(_))));
    // Le père revient sur son refus : autorisé de nouveau (ajout seul, l'historique reste).
    ent.accords_decider(etab, pere, saisie(&code, harry, &["participation"], true)).await.unwrap();
    assert_eq!(verdict(&ent, etab, &code, "participation", harry).await, Verdict::Desaccord, "la mère a retiré, le père accorde : ils se contredisent");
    ent.accords_decider(etab, lea, saisie(&code, harry, &["participation"], true)).await.unwrap();
    assert_eq!(verdict(&ent, etab, &code, "participation", harry).await, Verdict::Autorise);
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_acc_decision WHERE etablissement_id = $1 AND sujet_eleve_id = $2")
        .bind(etab).bind(harry).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(n, 5);
    // CK-35 : l'événement a commencé : un retrait est une demande à l'établissement.
    sqlx::query("UPDATE ent_acc_objet SET evenement_fin = $2, validite = 'evenement' WHERE etablissement_id = $1 AND code = $3")
        .bind(etab).bind(Utc::now() - Duration::hours(1)).bind(&code).execute(ent.pool_verif()).await.unwrap();
    assert!(matches!(ent.accords_retirer(etab, pere, &code, Some(harry), None).await, Err(EntDbError::ConflitDossier(_))));
    // Les accords d'un événement passé ont expiré.
    assert_eq!(verdict(&ent, etab, &code, "participation", harry).await, Verdict::Expire);
    retirer_parent(&ent, etab, pere).await;
    droit_sortie_de_lea(&ent, &cast, false).await;
    vider_accords(&ent, etab).await;
}

// ─── CK-06, CK-07, CK-08 : interdit, sans autorité, sans droit de sortie ─────────────────────────────

#[tokio::test]
async fn ck06_ck07_ck08_titulaires_qualifies_seulement() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_accords(&ent, etab).await;
    droit_sortie_de_lea(&ent, &cast, false).await;
    let sortie = objet_aleatoire();
    let image = objet_aleatoire();
    objet_publie(&ent, etab, dir, objet_propre(&sortie, "autorisation", "un", Some("autoriser_sorties"), &["participation"])).await;
    objet_publie(&ent, etab, dir, objet_propre(&image, "consentement", "tous", Some("signer_documents"), &["fil_classe"])).await;
    // CK-08 : sans le droit `autoriser_sorties`, la mère ne signe pas la sortie ; elle signe le consentement.
    assert!(ent.accords_decider(etab, lea, saisie(&sortie, harry, &["participation"], true)).await.is_err());
    ent.accords_decider(etab, lea, saisie(&image, harry, &["fil_classe"], true)).await.unwrap();
    droit_sortie_de_lea(&ent, &cast, true).await;
    ent.accords_decider(etab, lea, saisie(&sortie, harry, &["participation"], true)).await.unwrap();
    assert_eq!(verdict(&ent, etab, &sortie, "participation", harry).await, Verdict::Autorise);
    droit_sortie_de_lea(&ent, &cast, false).await;
    assert_eq!(verdict(&ent, etab, &sortie, "participation", harry).await, Verdict::EnAttente, "droit retiré : sa décision ne compte plus");
    droit_sortie_de_lea(&ent, &cast, true).await;
    // CK-07 : un lien sans autorité parentale n'est pas décideur et n'entre pas dans les attendus de `tous`.
    let beau = parent_en_plus(&ent, etab, harry, false, true).await;
    assert!(ent.accords_decider(etab, beau, saisie(&image, harry, &["fil_classe"], true)).await.is_err());
    assert_eq!(verdict(&ent, etab, &image, "fil_classe", harry).await, Verdict::Autorise, "un seul titulaire attendu");
    // CK-06 : une interdiction de communication retire la qualité de décideur ; `tous` s'applique aux autres.
    let pere = parent_en_plus(&ent, etab, harry, true, true).await;
    assert_eq!(verdict(&ent, etab, &image, "fil_classe", harry).await, Verdict::EnAttente, "deux titulaires attendus");
    let personne: (Uuid,) = sqlx::query_as("SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3")
        .bind(etab).bind(pere).bind(harry).fetch_one(ent.pool_verif()).await.unwrap();
    let interdiction = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1,$2,$3,$4, ARRAY['communication'], 'decision_justice', 'fo1-test', ent_aujourdhui_etab($2) - 1, $5)",
    )
    .bind(interdiction).bind(etab).bind(harry).bind(personne.0).bind(dir)
    .execute(ent.pool_verif()).await.unwrap();
    assert!(ent.accords_decider(etab, pere, saisie(&image, harry, &["fil_classe"], true)).await.is_err(), "il ne signe plus");
    assert_eq!(verdict(&ent, etab, &image, "fil_classe", harry).await, Verdict::Autorise, "`tous` ne porte plus que sur la mère");
    sqlx::query("UPDATE ent_eleve_interdiction SET levee_le = now(), levee_par = $2 WHERE id = $1").bind(interdiction).bind(dir).execute(ent.pool_verif()).await.unwrap();
    retirer_parent(&ent, etab, pere).await;
    retirer_parent(&ent, etab, beau).await;
    droit_sortie_de_lea(&ent, &cast, false).await;
    vider_accords(&ent, etab).await;
}

// ─── CK-15 à CK-19 : capacité et majorité ────────────────────────────────────────────────────────────

#[tokio::test]
async fn ck15_ck16_ck17_ck18_ck19_capacite_et_majorite() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_accords(&ent, etab).await;
    let code = objet_aleatoire();
    objet_publie(&ent, etab, dir, objet_propre(&code, "consentement", "tous", Some("signer_documents"), &["fil_classe"])).await;
    let enfant = compte_eleve(&ent, etab, harry).await;
    let origine = lire_naissance(&ent, etab, harry).await;
    poser_naissance(&ent, etab, harry, Some(il_y_a(15)), origine.1.as_deref()).await;
    ent.accords_decider(etab, lea, saisie(&code, harry, &["fil_classe"], true)).await.unwrap();
    assert_eq!(verdict(&ent, etab, &code, "fil_classe", harry).await, Verdict::Autorise);

    // CK-19 : à la majorité, le consentement parental cesse ; l'élève est sollicité (il décide seul).
    poser_naissance(&ent, etab, harry, Some(il_y_a(19)), origine.1.as_deref()).await;
    assert_eq!(verdict(&ent, etab, &code, "fil_classe", harry).await, Verdict::EnAttente);
    assert!(ent.accords_decider(etab, lea, saisie(&code, harry, &["fil_classe"], false)).await.is_err(), "les parents ne décident plus");
    let m = ent.accords_decider(etab, enfant, saisie(&code, harry, &["fil_classe"], true)).await.unwrap();
    assert_eq!(m.qualite, "eleve_majeur");
    assert_eq!(verdict(&ent, etab, &code, "fil_classe", harry).await, Verdict::Autorise);

    // CK-15 : majeur sous tutelle, les titulaires (tuteur) décident, l'élève n'est pas décideur.
    let mesure = |nature: &'static str| {
        let ent = &ent;
        async move {
            sqlx::query("INSERT INTO ent_eleve_capacite (id, etablissement_id, eleve_id, nature, du, piece_document_id) VALUES ($1,$2,$3,$4, ent_aujourdhui_etab($2) - 1, $5)")
                .bind(Uuid::new_v4()).bind(etab).bind(harry).bind(nature).bind(Uuid::new_v4())
                .execute(ent.pool_verif()).await.unwrap();
        }
    };
    let leve = |nature: &'static str| {
        let ent = &ent;
        async move {
            sqlx::query("UPDATE ent_eleve_capacite SET levee_le = now(), levee_par = $4 WHERE etablissement_id = $1 AND eleve_id = $2 AND nature = $3 AND levee_le IS NULL")
                .bind(etab).bind(harry).bind(nature).bind(dir).execute(ent.pool_verif()).await.unwrap();
        }
    };
    mesure("tutelle").await;
    // L'élève sous tutelle est informé et donne son avis (jamais la décision) ; la décision de Lea (tuteur) compte de nouveau.
    assert_eq!(verdict(&ent, etab, &code, "fil_classe", harry).await, Verdict::Autorise, "la décision de Lea (tuteur de fait) compte de nouveau");
    assert_eq!(ent.accords_decider(etab, enfant, saisie(&code, harry, &["fil_classe"], false)).await.unwrap().qualite, "eleve_avis");
    assert_eq!(verdict(&ent, etab, &code, "fil_classe", harry).await, Verdict::Refuse, "son avis négatif l'emporte");
    ent.accords_decider(etab, enfant, saisie(&code, harry, &["fil_classe"], true)).await.unwrap();
    leve("tutelle").await;
    // CK-16 : curatelle, l'élève décide.
    mesure("curatelle").await;
    assert_eq!(ent.accords_decider(etab, enfant, saisie(&code, harry, &["fil_classe"], false)).await.unwrap().qualite, "eleve_majeur");
    assert_eq!(verdict(&ent, etab, &code, "fil_classe", harry).await, Verdict::Refuse);
    leve("curatelle").await;
    // CK-17 : mineur émancipé, il décide seul.
    poser_naissance(&ent, etab, harry, Some(il_y_a(16)), origine.1.as_deref()).await;
    mesure("emancipation").await;
    assert_eq!(ent.accords_decider(etab, enfant, saisie(&code, harry, &["fil_classe"], true)).await.unwrap().qualite, "eleve_majeur");
    assert_eq!(verdict(&ent, etab, &code, "fil_classe", harry).await, Verdict::Autorise);
    leve("emancipation").await;
    // CK-18 : date de naissance contestée, majorité non présumée : les titulaires décident.
    poser_naissance(&ent, etab, harry, Some(il_y_a(19)), Some("contestee")).await;
    // L'élève ne décide plus seul : il ne donne qu'un avis, et l'accord des titulaires redevient décisif.
    assert_eq!(ent.accords_decider(etab, enfant, saisie(&code, harry, &["fil_classe"], true)).await.unwrap().qualite, "eleve_avis");
    assert_eq!(verdict(&ent, etab, &code, "fil_classe", harry).await, Verdict::Autorise);
    poser_naissance(&ent, etab, harry, origine.0, origine.1.as_deref()).await;
    vider_accords(&ent, etab).await;
}

// ─── CK-30, CK-31, CK-32 : versions à usages ─────────────────────────────────────────────────────────

#[tokio::test]
async fn ck30_ck31_ck32_versions_usage_ajoute_force_le_fond() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_accords(&ent, etab).await;
    let code = objet_aleatoire();
    objet_publie(&ent, etab, dir, objet_propre(&code, "consentement", "un", Some("signer_documents"), &["journal_interne", "lettre"])).await;
    ent.accords_decider(etab, lea, saisie(&code, harry, &["journal_interne"], true)).await.unwrap();
    ent.accords_decider(etab, lea, saisie(&code, harry, &["lettre"], false)).await.unwrap();
    assert_eq!(verdict(&ent, etab, &code, "journal_interne", harry).await, Verdict::Autorise);
    assert_eq!(verdict(&ent, etab, &code, "lettre", harry).await, Verdict::Refuse);
    let avant = nb_notifs_formulaire(&ent, etab, lea).await;

    // CK-31 : une correction de forme ne demande rien à personne.
    let usages_v1 = vec!["journal_interne".to_string(), "lettre".to_string()];
    ent.accords_rediger_version(etab, dir, DIR, &code, RedactionVersion { texte: "Texte, orthographe corrigée".into(), usages: usages_v1.clone(), nouveaux_usages: vec![] }).await.unwrap();
    let v2 = ent.accords_publier_version(etab, dir, DIR, &code, Some("forme")).await.unwrap();
    assert_eq!((v2.numero, v2.type_changement.as_deref()), (2, Some("forme")));
    assert_eq!(nb_notifs_formulaire(&ent, etab, lea).await, avant, "forme : aucune sollicitation");
    assert_eq!(verdict(&ent, etab, &code, "journal_interne", harry).await, Verdict::Autorise);

    // CK-32 : un usage ajouté ne peut pas être qualifié « forme » ; une restriction ne peut pas en ajouter.
    ent.accords_rediger_version(
        etab, dir, DIR, &code,
        RedactionVersion { texte: "Texte avec réseaux sociaux".into(), usages: usages_v1.clone(), nouveaux_usages: vec![usage("reseaux")] },
    ).await.unwrap();
    for faux in [Some("forme"), Some("restriction")] {
        assert!(matches!(ent.accords_publier_version(etab, dir, DIR, &code, faux).await, Err(EntDbError::ConflitDossier(_))), "{faux:?}");
    }
    // CK-30 : version de fond (par défaut) : accords valables pour les usages couverts, `a_renouveler` pour le nouveau,
    // refus maintenus.
    let v3 = ent.accords_publier_version(etab, dir, DIR, &code, None).await.unwrap();
    assert_eq!((v3.numero, v3.type_changement.as_deref()), (3, Some("fond")));
    assert!(v3.usages.contains(&"reseaux".to_string()));
    assert_eq!(verdict(&ent, etab, &code, "journal_interne", harry).await, Verdict::Autorise, "accord de la v1 valable pour son usage");
    assert_eq!(verdict(&ent, etab, &code, "lettre", harry).await, Verdict::Refuse, "le refus reste protecteur");
    assert_eq!(verdict(&ent, etab, &code, "reseaux", harry).await, Verdict::ARenouveler);
    assert!(nb_notifs_formulaire(&ent, etab, lea).await > avant, "fond : nouvelle demande, enveloppe neutre");
    let notif = ent.mes_notifications(etab, lea, 20).await.unwrap().into_iter().find(|n| n.kind == "formulaire" && n.corps.contains("renouveler")).unwrap();
    assert!(!notif.titre.contains(&code) && !notif.corps.contains(&code), "jamais le code ni le titre de l'objet");
    // « Mes accords » : j'ai décidé, pas sur ce nouvel usage.
    let mes = ent.accords_mes_accords(etab, lea).await.unwrap();
    let a = mes.iter().find(|s| s.eleve_id == Some(harry)).unwrap().accords.iter().find(|a| a.objet_code == code).unwrap();
    assert_eq!(a.version_courante, 3);
    assert!(a.usages.iter().find(|u| u.code == "reseaux").unwrap().a_renouveler);
    assert_eq!(a.usages.iter().find(|u| u.code == "journal_interne").unwrap().ma_decision, Some(true));
    // Renouvellement de l'usage ajouté.
    ent.accords_decider(etab, lea, saisie(&code, harry, &["reseaux"], true)).await.unwrap();
    assert_eq!(verdict(&ent, etab, &code, "reseaux", harry).await, Verdict::Autorise);
    // Restriction : un usage retiré n'est plus couvert (inconnu : l'appelant refuse) ; les autres restent valables.
    ent.accords_rediger_version(etab, dir, DIR, &code, RedactionVersion { texte: "Sans lettre".into(), usages: vec!["journal_interne".into(), "reseaux".into()], nouveaux_usages: vec![] }).await.unwrap();
    ent.accords_publier_version(etab, dir, DIR, &code, Some("restriction")).await.unwrap();
    assert_eq!(verdict(&ent, etab, &code, "lettre", harry).await, Verdict::Inconnu);
    assert_eq!(verdict(&ent, etab, &code, "journal_interne", harry).await, Verdict::Autorise);
    // Numéros strictement croissants ; version publiée immuable ; décisions en ajout seul (déclencheurs).
    let h = ent.accords_historique(etab, dir, DIR, &code).await.unwrap();
    assert_eq!(h.iter().map(|v| v.numero).collect::<Vec<_>>(), vec![1, 2, 3, 4]);
    for sql in [
        "UPDATE ent_acc_version SET texte = 'x' WHERE numero = 2",
        "DELETE FROM ent_acc_version WHERE numero = 2",
        "UPDATE ent_acc_decision SET decision = 'refuse'",
        "DELETE FROM ent_acc_decision",
        "UPDATE ent_acc_decision_usage SET accorde = false",
        "DELETE FROM ent_acc_journal",
        "UPDATE ent_acc_objet SET nature = 'autorisation'",
        "DELETE FROM ent_acc_objet",
    ] {
        let r = sqlx::query(sql).execute(ent.pool_verif()).await;
        assert!(matches!(&r, Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("42501")), "{sql} : {r:?}");
    }
    vider_accords(&ent, etab).await;
}

// ─── CK-36 : expiration ; protégé ; verifier jamais « autorisé » par défaut ─────────────────────────

#[tokio::test]
async fn ck36_expiration_protege_et_verifier_sans_defaut() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_accords(&ent, etab).await;
    let code = objet_aleatoire();
    objet_publie(&ent, etab, dir, objet_propre(&code, "consentement", "un", Some("signer_documents"), &["fil_classe", "interne_personnels_x"])).await;
    ent.accords_decider(etab, lea, saisie(&code, harry, &["fil_classe"], true)).await.unwrap();
    let req = |jour| DemandeVerification { objet_code: code.clone(), usage_code: "fil_classe".into(), sujets: vec![Sujet::Eleve(harry)], jour };
    assert_eq!(ent.accords_verifier(etab, &req(None)).await[0].verdict, Verdict::Autorise);
    // CK-36 : année scolaire : valable jusqu'au 31 août, expiré le 1er septembre suivant.
    let jour = domain_eleve::horloge::jour_paris();
    let fin_annee = {
        use chrono::Datelike;
        let y = if jour.month() >= 9 { jour.year() + 1 } else { jour.year() };
        (chrono::NaiveDate::from_ymd_opt(y, 8, 31).unwrap(), chrono::NaiveDate::from_ymd_opt(y, 9, 1).unwrap())
    };
    assert_eq!(ent.accords_verifier(etab, &req(Some(fin_annee.0))).await[0].verdict, Verdict::Autorise);
    assert_eq!(ent.accords_verifier(etab, &req(Some(fin_annee.1))).await[0].verdict, Verdict::Expire);

    // R-FC11 : un élève protégé n'est jamais publié, quels que soient les accords.
    let protection = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, motif_scelle, du, decide_par) VALUES ($1,$2,$3,'enc:v1:test', ent_aujourdhui_etab($2) - 1, $4)")
        .bind(protection).bind(etab).bind(harry).bind(dir).execute(ent.pool_verif()).await.unwrap();
    let droit_image = objet_publie(&ent, etab, dir, objet_propre("droit_image", "consentement", "tous", Some("signer_documents"), &["site_public"])).await;
    let _ = droit_image;
    ent.accords_decider(etab, lea, saisie("droit_image", harry, &["site_public"], true)).await.unwrap();
    assert_eq!(verdict(&ent, etab, "droit_image", "site_public", harry).await, Verdict::Protege);
    sqlx::query("UPDATE ent_eleve_confidentialite SET leve_le = now(), leve_par = $2 WHERE id = $1").bind(protection).bind(dir).execute(ent.pool_verif()).await.unwrap();
    assert_eq!(verdict(&ent, etab, "droit_image", "site_public", harry).await, Verdict::Autorise);

    // R-FC3 : objet, usage, sujet inconnus ; sujet d'un autre établissement ; sujet incompatible : `inconnu`, jamais `autorise`.
    let inconnu = |d: DemandeVerification| {
        let ent = &ent;
        async move { ent.accords_verifier(etab, &d).await.into_iter().map(|v| v.verdict).collect::<Vec<_>>() }
    };
    let base = DemandeVerification { objet_code: code.clone(), usage_code: "fil_classe".into(), sujets: vec![Sujet::Eleve(harry)], jour: None };
    assert_eq!(inconnu(DemandeVerification { objet_code: "absent_xyz".into(), ..base.clone() }).await, vec![Verdict::Inconnu]);
    assert_eq!(inconnu(DemandeVerification { usage_code: "absent_xyz".into(), ..base.clone() }).await, vec![Verdict::Inconnu]);
    assert_eq!(inconnu(DemandeVerification { sujets: vec![Sujet::Eleve(Uuid::new_v4()), Sujet::Personne(Uuid::new_v4())], ..base.clone() }).await, vec![Verdict::Inconnu, Verdict::Inconnu]);
    assert_eq!(inconnu(DemandeVerification { sujets: vec![Sujet::Personne(Uuid::new_v4())], ..base.clone() }).await, vec![Verdict::Inconnu]);
    assert!(inconnu(DemandeVerification { sujets: vec![], ..base.clone() }).await.is_empty());
    // Depuis un autre établissement : l'objet et l'élève n'existent pas.
    let autre = Uuid::new_v4();
    let v = ent.accords_verifier(autre, &base).await;
    assert_eq!(v[0].verdict, Verdict::Inconnu);
    // Journal agrégé : une ligne par appel, jamais une ligne par élève.
    let (n_verif, nb_max): (i64, Option<i32>) = sqlx::query_as("SELECT count(*), max(nb) FROM ent_acc_journal WHERE etablissement_id = $1 AND acte = 'verification'")
        .bind(etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(n_verif >= 5 && nb_max.unwrap_or(0) >= 2);
    let (par_eleve,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_acc_journal WHERE etablissement_id = $1 AND acte = 'verification' AND sujet_eleve_id IS NOT NULL")
        .bind(etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(par_eleve, 0);
    vider_accords(&ent, etab).await;
}

// ─── Droits de lecture : verdicts sans motif, portée de classe (FD-09, R-FC38, R-FC39) ───────────────

#[tokio::test]
async fn verdicts_par_acteur_portee_sans_motif_et_lecture_journalisee() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_accords(&ent, etab).await;
    let code = objet_aleatoire();
    objet_publie(&ent, etab, dir, objet_propre(&code, "consentement", "un", Some("signer_documents"), &["fil_classe"])).await;
    ent.accords_decider(etab, lea, saisie(&code, harry, &["fil_classe"], true)).await.unwrap();
    let classe = crate::tests_formulaires_fo0::classe_4b(&ent, etab).await;
    let demande = DemandeVerification { objet_code: code.clone(), usage_code: "fil_classe".into(), sujets: vec![Sujet::Eleve(harry), Sujet::Personne(Uuid::new_v4())], jour: None };
    // Direction : verdict et raison codée ; un sujet hors établissement reste `inconnu`.
    let v = ent.accords_verifier_pour_acteur(etab, dir, DIR, &demande).await.unwrap();
    assert_eq!(v[0].verdict, Verdict::Autorise);
    assert!(v.iter().any(|x| x.raison.is_some()), "la raison codée n'est rendue qu'à autorisations.gerer");
    // CPE : verdicts de tout l'établissement, sans raison.
    let v = ent.accords_verifier_pour_acteur(etab, cast.cpe.user_id, "cpe", &demande).await.unwrap();
    assert_eq!(v[0].verdict, Verdict::Autorise);
    assert!(v.iter().all(|x| x.raison.is_none()), "R-FC39 : sans motif ni décideur");
    // Un parent, la comptabilité, l'infirmerie : aucun droit.
    for (u, r) in [(lea, "parent"), (cast.compta.user_id, "compta")] {
        assert!(matches!(ent.accords_verifier_pour_acteur(etab, u, r, &demande).await, Err(EntDbError::Forbidden)), "{r}");
        assert!(matches!(ent.accords_verdicts_classe(etab, u, r, &code, "fil_classe", classe).await, Err(EntDbError::Forbidden)), "{r}");
    }
    // Grille d'une classe : la direction voit chaque élève ; le professeur seulement dans sa portée.
    let grille = ent.accords_verdicts_classe(etab, dir, DIR, &code, "fil_classe", classe).await.unwrap();
    assert!(grille.iter().any(|g| g.eleve_id == harry && g.verdict == Verdict::Autorise));
    assert!(grille.iter().filter(|g| g.eleve_id != harry).all(|g| g.verdict == Verdict::EnAttente));
    let prof = cast.prof.user_id;
    let suivis = ent.eleves_autorises(etab, prof, "prof", ent_auth::Action::AutorisationsLire).await.unwrap();
    match suivis {
        Some(s) if s.contains(&harry) => {
            let g = ent.accords_verdicts_classe(etab, prof, "prof", &code, "fil_classe", classe).await.unwrap();
            assert!(g.iter().all(|e| s.contains(&e.eleve_id)), "seulement les élèves de sa portée");
            // Un élève hors portée est `inconnu` dans une demande directe.
            if let Some(hors) = cast.eleves.iter().map(|e| e.id).find(|id| !s.contains(id)) {
                let d = DemandeVerification { sujets: vec![Sujet::Eleve(hors)], ..demande.clone() };
                assert_eq!(ent.accords_verifier_pour_acteur(etab, prof, "prof", &d).await.unwrap()[0].verdict, Verdict::Inconnu);
            }
        }
        _ => assert!(matches!(ent.accords_verdicts_classe(etab, prof, "prof", &code, "fil_classe", classe).await, Err(EntDbError::Forbidden) | Ok(_))),
    }
    // Lecture détaillée des décisions : direction et secrétariat seulement, **journalisée** (FD-09).
    assert!(matches!(ent.accords_lire_decisions(etab, cast.cpe.user_id, "cpe", &code, harry).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.accords_lire_decisions(etab, prof, "prof", &code, harry).await, Err(EntDbError::Forbidden)));
    let det = ent.accords_lire_decisions(etab, cast.administratif.user_id, "administratif", &code, harry).await.unwrap();
    assert_eq!((det.len(), det[0].qualite.as_str(), det[0].canal.as_str()), (1, "titulaire", "en_ligne"));
    let (lectures,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_acc_journal WHERE etablissement_id = $1 AND acte = 'lecture_detaillee' AND acteur_user_id = $2")
        .bind(etab).bind(cast.administratif.user_id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(lectures, 1);
    vider_accords(&ent, etab).await;
}

// ─── R-FC16 : « mes accords » ne montre jamais la décision d'un autre ───────────────────────────────

#[tokio::test]
async fn mes_accords_jamais_la_decision_de_l_autre_ck21_fratrie() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_accords(&ent, etab).await;
    let code = objet_aleatoire();
    objet_publie(&ent, etab, dir, objet_propre(&code, "consentement", "tous", Some("signer_documents"), &["fil_classe"])).await;
    let pere = parent_en_plus(&ent, etab, harry, true, false).await;
    ent.accords_decider(etab, lea, saisie(&code, harry, &["fil_classe"], false)).await.unwrap();
    let vue_pere = ent.accords_mes_accords(etab, pere).await.unwrap();
    let a = vue_pere.iter().find(|s| s.eleve_id == Some(harry)).unwrap().accords.iter().find(|a| a.objet_code == code).unwrap();
    assert!(a.usages.iter().all(|u| u.ma_decision.is_none() && !u.a_renouveler), "il ne voit pas le refus de la mère");
    assert_eq!(a.qualite, "titulaire");
    assert!(a.texte_provisoire);
    let vue_lea = ent.accords_mes_accords(etab, lea).await.unwrap();
    let a = vue_lea.iter().find(|s| s.eleve_id == Some(harry)).unwrap().accords.iter().find(|a| a.objet_code == code).unwrap();
    assert_eq!(a.usages[0].ma_decision, Some(false));
    // La réponse à `decider` est la sienne, pas l'état global.
    let r = ent.accords_decider(etab, pere, saisie(&code, harry, &["fil_classe"], true)).await.unwrap();
    assert_eq!((r.decision.as_str(), r.usages.len()), ("accorde", 1));
    // CK-21 : une décision par enfant, explicite : l'autre enfant d'un autre foyer n'est pas touché.
    assert_eq!(verdict(&ent, etab, &code, "fil_classe", cast.eleve_tom_id).await, Verdict::EnAttente);
    // Un parent qui n'est lié à aucun enfant n'a rien.
    assert!(ent.accords_mes_accords(etab, cast.compta.user_id).await.unwrap().is_empty());
    // Un sujet personne : pas d'élève.
    assert!(ent.accords_decider(etab, lea, DecisionSaisie { objet_code: code.clone(), eleve_id: None, usages: choix(&["fil_classe"], true) }).await.is_err());
    retirer_parent(&ent, etab, pere).await;
    vider_accords(&ent, etab).await;
}

// ─── Port de signature (socle 7, lot D6) : jamais de preuve simulée ──────────────────────────────────

struct SignatureDeTest(Uuid);

impl PortSignature for SignatureDeTest {
    fn signer<'a>(
        &'a self,
        demande: DemandeSignature<'a>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Option<Uuid>, EntDbError>> + Send + 'a>> {
        let id = self.0;
        Box::pin(async move {
            assert!(!demande.empreinte_version.is_empty());
            Ok(Some(id))
        })
    }
}

#[tokio::test]
async fn le_port_de_signature_fournit_la_preuve_et_s2_sans_brique_est_refuse() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_accords(&ent, etab).await;
    droit_sortie_de_lea(&ent, &cast, true).await;
    let s1 = objet_aleatoire();
    let s2 = objet_aleatoire();
    objet_publie(&ent, etab, dir, objet_propre(&s1, "autorisation", "un", Some("autoriser_sorties"), &["participation"])).await;
    let mut n2 = objet_propre(&s2, "autorisation", "un", Some("autoriser_sorties"), &["participation"]);
    n2.preuve_min = "S2".into();
    objet_publie(&ent, etab, dir, n2).await;
    // S1 sans brique : décision en ligne, sans identifiant de preuve (le canal dit ce qu'on sait).
    ent.accords_decider(etab, lea, saisie(&s1, harry, &["participation"], true)).await.unwrap();
    let (canal, preuve): (String, Option<Uuid>) = sqlx::query_as("SELECT canal, preuve_signature_id FROM ent_acc_decision WHERE etablissement_id = $1 AND objet_id = (SELECT id FROM ent_acc_objet WHERE etablissement_id = $1 AND code = $2)")
        .bind(etab).bind(&s1).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!((canal.as_str(), preuve), ("en_ligne", None));
    // S2 sans brique de signature : refusé, jamais simulé.
    assert!(matches!(ent.accords_decider(etab, lea, saisie(&s2, harry, &["participation"], true)).await, Err(EntDbError::ConflitDossier(m)) if m.contains("S2")));
    assert_eq!(verdict(&ent, etab, &s2, "participation", harry).await, Verdict::EnAttente);
    // Avec une brique : l'identifiant de la preuve est conservé ; un retrait ne demande aucune preuve (R-FC15).
    let preuve_id = Uuid::new_v4();
    ent.accords_decider_avec(etab, lea, saisie(&s2, harry, &["participation"], true), &SignatureDeTest(preuve_id)).await.unwrap();
    let (p,): (Option<Uuid>,) = sqlx::query_as("SELECT preuve_signature_id FROM ent_acc_decision WHERE etablissement_id = $1 AND objet_id = (SELECT id FROM ent_acc_objet WHERE etablissement_id = $1 AND code = $2)")
        .bind(etab).bind(&s2).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(p, Some(preuve_id));
    assert_eq!(verdict(&ent, etab, &s2, "participation", harry).await, Verdict::Autorise);
    assert_eq!(ent.accords_retirer(etab, lea, &s2, Some(harry), None).await.unwrap().decision, "retire");
    let (empreinte,): (String,) = sqlx::query_as("SELECT empreinte FROM ent_acc_decision WHERE etablissement_id = $1 ORDER BY decide_le LIMIT 1").bind(etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(empreinte.len(), 64);
    droit_sortie_de_lea(&ent, &cast, false).await;
    vider_accords(&ent, etab).await;
}

// ─── Reprise de 193 : jamais « accordé » sans preuve ; refus hérité global ──────────────────────────

#[tokio::test]
async fn reprise_193_conserve_les_refus_et_ne_vaut_jamais_accord_pour_un_usage_precis() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_accords(&ent, etab).await;
    crate::tests_formulaires_fo0::vider_pour_tests(ent.pool_verif(), etab).await;
    let classe = crate::tests_formulaires_fo0::classe_4b(&ent, etab).await;
    let objet = objet_aleatoire();
    let pere = parent_en_plus(&ent, etab, harry, true, false).await;
    // Ancien module : v1 publiée ; la mère accepte, le père refuse ; v2 de fond.
    let f1 = ent.creer_formulaire(etab, dir, DIR, crate::tests_formulaires_fo0::nouveau("consentement", &objet, classe, "Ancien texte")).await.unwrap();
    ent.publier_formulaire(etab, dir, DIR, f1.id).await.unwrap();
    ent.repondre_formulaire(etab, lea, f1.id, crate::tests_formulaires_fo0::rep(harry, "accepte")).await.unwrap();
    ent.repondre_formulaire(etab, pere, f1.id, crate::tests_formulaires_fo0::rep(harry, "refuse")).await.unwrap();
    let f2 = ent.creer_formulaire(etab, dir, DIR, crate::tests_formulaires_fo0::nouveau("consentement", &objet, classe, "Nouveau texte")).await.unwrap();
    ent.publier_formulaire(etab, dir, DIR, f2.id).await.unwrap();

    assert!(matches!(ent.accords_reprendre_formulaires(etab, cast.administratif.user_id, "administratif").await, Err(EntDbError::Forbidden)));
    let bilan = ent.accords_reprendre_formulaires(etab, dir, DIR).await.unwrap();
    assert_eq!((bilan.objets, bilan.versions, bilan.decisions), (1, 2, 2));
    // Rejouable : rien n'est repris deux fois.
    let encore = ent.accords_reprendre_formulaires(etab, dir, DIR).await.unwrap();
    assert_eq!((encore.objets, encore.versions, encore.decisions, encore.deja_reprises), (0, 0, 0, 2));
    // Le père refusait : refus ; la mère acceptait (règle par défaut `un`) : refus et accord ⇒ désaccord.
    assert_eq!(verdict(&ent, etab, &objet, USAGE_REPRISE, harry).await, Verdict::Desaccord);
    // Un nouvel usage détaillé : le refus ancien protège (global), l'accord ancien ne vaut pas pour un usage précis.
    ent.accords_rediger_version(
        etab, dir, DIR, &objet,
        RedactionVersion { texte: "Texte détaillé".into(), usages: vec![USAGE_REPRISE.into(), "fil_etablissement".into()], nouveaux_usages: vec![usage("fil_etablissement")] },
    ).await.unwrap();
    ent.accords_publier_version(etab, dir, DIR, &objet, None).await.unwrap();
    assert_eq!(verdict(&ent, etab, &objet, "fil_etablissement", harry).await, Verdict::Refuse, "refus hérité : protège les usages détaillés");
    // Sans le refus du père (lien terminé), la seule décision restante est un accord ancien sans usage précis.
    retirer_parent(&ent, etab, pere).await;
    assert_eq!(verdict(&ent, etab, &objet, "fil_etablissement", harry).await, Verdict::ARenouveler, "R-FC43 : jamais accordé sans usage précis");
    // Les décisions reprises sont des décisions de dossier, datées.
    let (canaux,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_acc_decision WHERE etablissement_id = $1 AND canal = 'reprise_dossier' AND source_reprise = 'formulaire_193'")
        .bind(etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(canaux, 2);
    vider_accords(&ent, etab).await;
    crate::tests_formulaires_fo0::vider_pour_tests(ent.pool_verif(), etab).await;
}

// ─── Reprise de 275 : consentements administratifs du socle 4 ────────────────────────────────────────

#[tokio::test]
async fn reprise_275_consentements_des_personnes() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let lea = cast.parent_lea.user_id;
    vider_accords(&ent, etab).await;
    let personne: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL")
        .bind(etab).bind(lea).fetch_optional(ent.pool_verif()).await.unwrap();
    let Some((pid,)) = personne else { return eprintln!("skip: la persona Lea n'a pas de fiche personne") };
    sqlx::query("DELETE FROM ent_personne_consentement WHERE etablissement_id = $1 AND personne_id = $2").bind(etab).bind(pid).execute(ent.pool_verif()).await.unwrap();
    let ligne = |objet: &'static str, reponse: &'static str, canal: &'static str, piece: Option<&'static str>| {
        let ent = &ent;
        async move {
            sqlx::query("INSERT INTO ent_personne_consentement (id, etablissement_id, personne_id, objet, reponse, texte_version, canal, piece_reference, donne_par) VALUES ($1,$2,$3,$4,$5,'v1',$6,$7,$8)")
                .bind(Uuid::new_v4()).bind(etab).bind(pid).bind(objet).bind(reponse).bind(canal).bind(piece).bind(lea)
                .execute(ent.pool_verif()).await.unwrap();
        }
    };
    ligne("diffusion_transport", "accorde", "portail", None).await;
    ligne("diffusion_associations_parents", "refuse", "papier", Some("coupon-12")).await;
    ligne("photo_remise", "accorde", "guichet", None).await;

    let bilan = ent.accords_reprendre_consentements_personnes(etab, dir, DIR).await.unwrap();
    assert_eq!(bilan.decisions, 3);
    assert_eq!(ent.accords_reprendre_consentements_personnes(etab, dir, DIR).await.unwrap().decisions, 0, "rejouable");
    let v = |objet: &'static str, usage: &'static str| {
        let ent = &ent;
        async move {
            ent.accords_verifier(etab, &DemandeVerification { objet_code: objet.into(), usage_code: usage.into(), sujets: vec![Sujet::Personne(pid)], jour: None }).await[0].verdict
        }
    };
    assert_eq!(v("coordonnees_transport", "transporteur").await, Verdict::Autorise);
    assert_eq!(v("coordonnees_associations", "liste_parents_association").await, Verdict::Refuse);
    assert_eq!(v("photo_personne_autorisee", "remise_accueil").await, Verdict::Autorise, "saisie au guichet : l'agent a recueilli la décision");
    assert_eq!(v("liste_representants", "liste_classe").await, Verdict::EnAttente, "jamais repris, jamais accordé");
    // Le texte repris est le texte de départ : provisoire (R-FC72).
    let o = ent.accords_objets(etab, dir, DIR).await.unwrap().into_iter().find(|o| o.code == "coordonnees_transport").unwrap();
    assert!(o.texte_provisoire && o.sujet_type == "personne");
    // La personne voit ses accords, et seulement les siens ; elle retire sans passkey.
    let mes = ent.accords_mes_accords(etab, lea).await.unwrap();
    let moi = mes.iter().find(|s| s.eleve_id.is_none()).expect("accords personnels");
    assert!(moi.accords.iter().any(|a| a.objet_code == "coordonnees_transport" && a.usages[0].ma_decision == Some(true)));
    let r = ent.accords_retirer(etab, lea, "coordonnees_transport", None, None).await.unwrap();
    assert_eq!(r.decision, "retire");
    assert_eq!(v("coordonnees_transport", "transporteur").await, Verdict::Refuse);
    // Une personne décide pour elle-même, jamais pour une autre.
    assert!(ent.accords_decider(etab, lea, DecisionSaisie { objet_code: "coordonnees_transport".into(), eleve_id: Some(cast.eleve_lea_id), usages: choix(&["transporteur"], true) }).await.is_err());
    sqlx::query("DELETE FROM ent_personne_consentement WHERE etablissement_id = $1 AND personne_id = $2").bind(etab).bind(pid).execute(ent.pool_verif()).await.unwrap();
    vider_accords(&ent, etab).await;
}

// ─── Cloisonnement entre établissements (RLS, rôle sans BYPASSRLS en mode strict) ───────────────────

#[tokio::test]
async fn separation_entre_etablissements_aucun_filtre_oublie_ne_fuit() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (lea, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    vider_accords(&ent, etab).await;
    let code = objet_aleatoire();
    objet_publie(&ent, etab, dir, objet_propre(&code, "consentement", "un", Some("signer_documents"), &["fil_classe"])).await;
    ent.accords_decider(etab, lea, saisie(&code, harry, &["fil_classe"], true)).await.unwrap();
    assert_eq!(verdict(&ent, etab, &code, "fil_classe", harry).await, Verdict::Autorise);

    // Un second établissement, avec son propre objet de même code.
    let autre = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_etablissement (id, slug, nom) VALUES ($1, 'acc-sep-' || substr($1::text, 1, 8), 'Accords séparation')")
        .bind(autre).execute(ent.pool_verif()).await.unwrap();
    sqlx::query("INSERT INTO ent_acc_objet (id, etablissement_id, code, libelle, nature, regle, preuve_min, validite, sujet_type) VALUES ($1,$2,$3,'Autre','consentement','un','S1','annee_scolaire','eleve')")
        .bind(Uuid::new_v4()).bind(autre).bind(&code).execute(ent.pool_verif()).await.unwrap();

    // Sous le contexte de l'autre établissement, SANS filtre : aucune ligne de l'établissement de départ.
    // (En mode strict, rôle sans BYPASSRLS : c'est la base qui isole. Hors mode strict, le code filtre par établissement.)
    let strict = crate::tests_rls_stricte::mode_strict_demande();
    for table in ["ent_acc_decision", "ent_acc_decision_usage", "ent_acc_version", "ent_acc_version_usage", "ent_acc_usage", "ent_acc_journal"] {
        let mut tx = crate::tenant_tx::TenantTx::begin(ent.pool(), autre).await.unwrap();
        let sql = if strict { format!("SELECT count(*) FROM {table}") } else { format!("SELECT count(*) FROM {table} WHERE etablissement_id = '{autre}'") };
        let (n,): (i64,) = sqlx::query_as(&sql).fetch_one(&mut *tx).await.unwrap();
        assert_eq!(n, 0, "{table} : l'établissement voisin ne voit rien");
    }
    {
        let mut tx = crate::tenant_tx::TenantTx::begin(ent.pool(), autre).await.unwrap();
        let sql = if strict { "SELECT count(*) FROM ent_acc_objet".to_string() } else { format!("SELECT count(*) FROM ent_acc_objet WHERE etablissement_id = '{autre}'") };
        let (n,): (i64,) = sqlx::query_as(&sql).fetch_one(&mut *tx).await.unwrap();
        assert_eq!(n, 1, "seul son objet");
        // Écrire dans l'établissement de départ sous le contexte de l'autre : refusé par la base (WITH CHECK).
        let r = sqlx::query("INSERT INTO ent_acc_objet (id, etablissement_id, code, libelle, nature, regle, preuve_min, validite, sujet_type) VALUES (gen_random_uuid(), $1, 'intrus_x', 'I','consentement','un','S1','annee_scolaire','eleve')")
            .bind(etab).execute(&mut *tx).await;
        if strict {
            assert!(matches!(&r, Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("42501")), "{r:?}");
        }
    }
    // Le port : l'élève et la décision de l'établissement de départ n'existent pas ailleurs ; verdict `inconnu`, jamais `autorise`.
    let v = ent.accords_verifier(autre, &DemandeVerification { objet_code: code.clone(), usage_code: "fil_classe".into(), sujets: vec![Sujet::Eleve(harry)], jour: None }).await;
    assert_ne!(v[0].verdict, Verdict::Autorise);
    // La famille de l'établissement de départ n'a rien dans l'autre ; ses accords ne fuient pas.
    assert!(ent.accords_mes_accords(autre, lea).await.unwrap().is_empty());
    assert!(ent.accords_decider(autre, lea, saisie(&code, harry, &["fil_classe"], false)).await.is_err());
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.purge_etablissement', 'on', true)").execute(&mut *tx).await.unwrap();
    sqlx::query("DELETE FROM ent_acc_objet WHERE etablissement_id = $1").bind(autre).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    vider_accords(&ent, etab).await;
}
