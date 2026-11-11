//! Socle 4, lot A2 : coordonnées datées de la fiche de la personne (ADM-05), historique, vérification
//! par code, E.164, préférences (ADM-07), consentements (ADM-12), divergences compte ↔ fiche,
//! identité versionnée (ADM-03) et séparation nom / prénom (ADM-10). Contre la base locale de test.

use super::tests_administratif_a0::{dir, labo, personne_avec_compte, saisie};
use super::*;
use crate::dossier_personnes::{
    ChoixDivergence, CoordonneeSaisie, IdentiteSaisie, NatureIdentite, NatureRemplacement,
    PreferencesPersonne, RaisonCoordonnee, SeparationNom,
};
use crate::seed::LaboCast;
use crate::dossier_personnes::Auteur;
use crate::service::RefusMetier;
use domain_eleve::DomainError;
use ent_auth::Role;
use uuid::Uuid;

async fn labo_mail() -> Option<(PgEnt, LaboCast)> {
    let (mut ent, cast) = labo().await?;
    ent.install_mail_dry_run();
    Some((ent, cast))
}

fn refus<T>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Refus(..)))
}

fn refus_de<T>(r: Result<T, EntDbError>, nature: RefusMetier) -> bool {
    matches!(r, Err(EntDbError::Refus(n, _)) if n == nature)
}

fn saisie_coord(type_: &str, valeur: &str) -> CoordonneeSaisie {
    CoordonneeSaisie {
        type_: type_.into(),
        valeur: valeur.into(),
        libelle: None,
        usages: vec![],
        principal: false,
        raison: RaisonCoordonnee::ChangementDeclare,
        motif: None,
    }
}

/// Valeurs de toutes les lignes (ouvertes ou fermées) d'une personne : (type, valeur, ouverte, principal).
async fn lignes(ent: &PgEnt, pid: Uuid) -> Vec<(String, String, bool, bool)> {
    sqlx::query_as(
        "SELECT type, valeur, au IS NULL, principal FROM ent_personne_coordonnee
          WHERE personne_id = $1 ORDER BY du, valeur",
    )
    .bind(pid)
    .fetch_all(ent.pool())
    .await
    .unwrap()
}

fn a_ligne(l: &[(String, String, bool, bool)], valeur: &str, ouverte: bool) -> bool {
    l.iter().any(|(_, v, o, _)| v == valeur && *o == ouverte)
}

async fn legacy(ent: &PgEnt, pid: Uuid) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    sqlx::query_as("SELECT email, telephone, telephone_2, adresse FROM ent_personne WHERE id = $1")
        .bind(pid)
        .fetch_one(ent.pool())
        .await
        .unwrap()
}

/// Journaux et événements d'une personne : texte complet, pour y chercher des valeurs.
async fn traces_de(ent: &PgEnt, pid: Uuid) -> String {
    let j: Vec<(String,)> = sqlx::query_as(
        "SELECT payload::text FROM ent_journal_modif WHERE payload->>'personne_id' = $1",
    )
    .bind(pid.to_string())
    .fetch_all(ent.pool())
    .await
    .unwrap();
    let e: Vec<(String,)> = sqlx::query_as(
        "SELECT payload::text FROM ent_metier_event WHERE aggregate_id = $1",
    )
    .bind(pid)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    j.into_iter().chain(e).map(|(t,)| t).collect::<Vec<_>>().join("\n")
}

fn code_email(ent: &PgEnt, to: &str) -> String {
    let (_, _, corps) = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .rev()
        .find(|(a, sujet, _)| a == to && sujet.contains("code de vérification"))
        .expect("code envoyé par e-mail");
    let i = corps.find("est : ").expect("code") + "est : ".len();
    corps[i..i + 6].to_string()
}

fn code_sms(ent: &PgEnt, to: &str) -> String {
    let r = ent
        .sms()
        .receipts()
        .into_iter()
        .rev()
        .find(|r| r.to == to)
        .expect("code envoyé par SMS");
    let i = r.body.find(": ").expect("code") + 2;
    r.body[i..i + 6].to_string()
}

/// Aucune coordonnée perdue : un écrivain historique (colonnes de `ent_personne`) comme les
/// commandes du lot laissent une ligne datée et une entrée d'historique ; l'historique, les
/// journaux et les événements ne portent aucune valeur.
#[tokio::test]
async fn coordonnees_aucune_perdue_historique_sans_valeur() {
    let Some((ent, cast)) = labo_mail().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let mut s = saisie("Coordonnees", Some("Marie.Coord@Exemple.FR"), Some("06 12 34 56 78"));
    s.telephone_2 = Some("01.02.03.04.05".into());
    s.adresse = Some("12 rue des Lilas, 75011 Paris".into());
    let p = ent.creer_personne_for_actor(etab, d, r, &s).await.unwrap();

    // Les colonnes héritées sont la projection normalisée des lignes datées.
    let (email, tel, tel2, adresse) = legacy(&ent, p.id).await;
    assert_eq!(email.as_deref(), Some("marie.coord@exemple.fr"));
    assert_eq!(tel.as_deref(), Some("+33612345678"));
    assert_eq!(tel2.as_deref(), Some("+33102030405"));
    assert_eq!(adresse.as_deref(), Some("12 rue des Lilas, 75011 Paris"));
    let c = ent.coordonnees_personne_for_actor(etab, d, r, p.id, false).await.unwrap();
    assert!(!c.adresses_masquees);
    assert_eq!(c.coordonnees.len(), 4);
    let mobile = c.coordonnees.iter().find(|c| c.type_ == "mobile").unwrap();
    assert!(mobile.principal && mobile.verifiee_le.is_none() && !mobile.a_verifier);
    assert_eq!(mobile.source, "ecriture_historique");
    let fixe = c.coordonnees.iter().find(|c| c.type_ == "fixe").unwrap();
    assert!(!fixe.principal);

    // Modification par la fiche (chemin historique `modifier_personne_for_actor`) : l'ancien
    // e-mail est fermé, jamais écrasé ; l'auteur et la raison sont tracés.
    let mut s2 = s.clone();
    s2.email = Some("nouvelle.adresse@exemple.fr".into());
    ent.modifier_personne_for_actor(etab, d, r, p.id, &s2).await.unwrap();
    let l = lignes(&ent, p.id).await;
    assert!(a_ligne(&l, "marie.coord@exemple.fr", false), "ancien e-mail conservé, fermé");
    assert!(a_ligne(&l, "nouvelle.adresse@exemple.fr", true));
    let h = ent.historique_coordonnees_for_actor(etab, d, r, p.id).await.unwrap();
    let remplacement = h.iter().find(|h| h.nature == "remplacement").expect("remplacement tracé");
    assert_eq!(remplacement.via, "personnel");
    assert_eq!(remplacement.raison, "changement_declare");
    assert!(remplacement.par_nom.is_some(), "le personnel voit qui a changé");

    // Un écrivain historique qui n'a jamais entendu parler des coordonnées (inscriptions, imports).
    sqlx::query("UPDATE ent_personne SET telephone = '07 11 22 33 44', adresse = NULL WHERE id = $1")
        .bind(p.id)
        .execute(ent.pool())
        .await
        .unwrap();
    let l = lignes(&ent, p.id).await;
    assert!(a_ligne(&l, "+33612345678", false), "ancien mobile conservé");
    assert!(a_ligne(&l, "+33711223344", true));
    assert!(a_ligne(&l, "12 rue des Lilas, 75011 Paris", false), "adresse fermée, pas perdue");
    let (_, tel, _, adresse) = legacy(&ent, p.id).await;
    assert_eq!(tel.as_deref(), Some("+33711223344"));
    assert_eq!(adresse, None);

    // Les valeurs ne sont dans aucun journal ni aucun événement ; l'historique n'en porte pas.
    let traces = traces_de(&ent, p.id).await;
    for valeur in ["exemple.fr", "+33", "0612", "Lilas", "marie"] {
        assert!(!traces.to_lowercase().contains(&valeur.to_lowercase()), "valeur « {valeur} » dans les traces");
    }
    assert!(traces.contains("champs") || traces.contains("types"));
    // Lecture « fermées comprises » : les anciennes valeurs restent consultables par qui a le droit.
    let tout = ent.coordonnees_personne_for_actor(etab, d, r, p.id, true).await.unwrap();
    assert!(tout.coordonnees.iter().any(|c| c.valeur == "marie.coord@exemple.fr" && c.au.is_some()));
    // Un professeur, un parent : pas de droit sur les coordonnées d'une personne.
    assert!(matches!(
        ent.coordonnees_personne_for_actor(etab, cast.prof.user_id, Role::Prof.as_str(), p.id, false)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.coordonnees_personne_for_actor(etab, cast.parent_lea.user_id, Role::Parent.as_str(), p.id, false)
            .await,
        Err(EntDbError::Forbidden)
    ));
}

/// Commandes : ajout, doublon, remplacement (correction ≠ changement), retrait qui promeut,
/// usages, principal, limite, motif ; une personne d'un autre établissement est introuvable.
#[tokio::test]
async fn coordonnees_commandes_et_regles() {
    let Some((ent, cast)) = labo_mail().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let p = ent.creer_personne_for_actor(etab, d, r, &saisie("Commandes", None, None)).await.unwrap();

    let e1 = ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &saisie_coord("email", " Un@Exemple.fr ")).await.unwrap();
    assert_eq!(e1.valeur, "un@exemple.fr");
    assert!(e1.principal, "la première coordonnée de son genre est principale");
    assert!(refus_de(
        ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &saisie_coord("email", "un@exemple.fr")).await,
        RefusMetier::Conflit
    ));
    assert!(ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &saisie_coord("email", "pas-un-email")).await.is_err());
    let mut e2s = saisie_coord("email", "deux@exemple.fr");
    e2s.usages = vec!["facturation".into()];
    let e2 = ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &e2s).await.unwrap();
    assert!(!e2.principal, "un autre e-mail n'est principal que s'il le demande");
    assert!(!e2.usages.contains(&"notifications".to_string()));
    let mut e3s = saisie_coord("email", "trois@exemple.fr");
    e3s.principal = true;
    let e3 = ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &e3s).await.unwrap();
    assert!(e3.principal);
    let l = lignes(&ent, p.id).await;
    assert_eq!(l.iter().filter(|(t, _, o, pr)| t == "email" && *o && *pr).count(), 1, "un seul principal");
    assert_eq!(legacy(&ent, p.id).await.0.as_deref(), Some("trois@exemple.fr"));

    // Mobile / fixe : cohérence du type et E.164.
    let m = ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &saisie_coord("mobile", "06 99 88 77 66")).await.unwrap();
    assert_eq!(m.valeur, "+33699887766");
    assert!(ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &saisie_coord("mobile", "01 02 03 04 05")).await.is_err());
    assert!(ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &saisie_coord("fixe", "06 12 34 56 78")).await.is_err());
    assert!(ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &saisie_coord("mobile", "123")).await.is_err());
    let etranger = ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &saisie_coord("professionnel", "+44 7911 123456")).await.unwrap();
    assert_eq!(etranger.valeur, "+447911123456");

    // Correction ≠ remplacement : même mécanique de datation, nature et raison distinctes.
    let corr = ent
        .remplacer_coordonnee_for_actor(
            etab, d, r, p.id, e3.id, "trios@exemple.fr", NatureRemplacement::Correction,
            RaisonCoordonnee::ErreurDeSaisie, &Some("faute de frappe".into()),
        )
        .await
        .unwrap();
    assert!(corr.principal && corr.remplace_id == Some(e3.id), "le principal passe à la valeur corrigée");
    let h = ent.historique_coordonnees_for_actor(etab, d, r, p.id).await.unwrap();
    let c = h.iter().find(|h| h.nature == "correction").unwrap();
    assert_eq!((c.raison.as_str(), c.motif.as_deref()), ("erreur_de_saisie", Some("faute de frappe")));
    assert!(ent.remplacer_coordonnee_for_actor(
        etab, d, r, p.id, corr.id, "trios@exemple.fr", NatureRemplacement::Remplacement,
        RaisonCoordonnee::Autre, &None,
    ).await.is_err(), "valeur inchangée");
    // L'ancienne valeur fermée n'est plus remplaçable.
    assert!(matches!(
        ent.remplacer_coordonnee_for_actor(
            etab, d, r, p.id, e3.id, "x@exemple.fr", NatureRemplacement::Correction,
            RaisonCoordonnee::Autre, &None,
        ).await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));

    // Retrait du principal : une autre coordonnée éligible prend la place, rien n'est supprimé.
    ent.retirer_coordonnee_for_actor(etab, d, r, p.id, corr.id, RaisonCoordonnee::DemandeDeLaPersonne, &None).await.unwrap();
    let ouvertes = ent.coordonnees_personne_for_actor(etab, d, r, p.id, false).await.unwrap().coordonnees;
    let principal_email = ouvertes.iter().find(|c| c.type_ == "email" && c.principal).expect("principal promu");
    assert_eq!(principal_email.valeur, "un@exemple.fr", "la plus ancienne coordonnée éligible");
    assert!(a_ligne(&lignes(&ent, p.id).await, "trios@exemple.fr", false));
    assert_eq!(legacy(&ent, p.id).await.0.as_deref(), Some("un@exemple.fr"));

    // Usages : `connexion` n'appartient qu'au compte ; un principal sert aux notifications.
    assert!(ent.usages_coordonnee_for_actor(etab, d, r, p.id, e2.id, &["connexion".into()], false).await.is_err());
    let u = ent.usages_coordonnee_for_actor(etab, d, r, p.id, e2.id, &["notifications".into(), "urgence".into()], true).await.unwrap();
    assert!(u.principal && u.usages.contains(&"urgence".to_string()));
    assert_eq!(legacy(&ent, p.id).await.0.as_deref(), Some("deux@exemple.fr"));
    let apres = ent.usages_coordonnee_for_actor(etab, d, r, p.id, e2.id, &["facturation".into()], false).await.unwrap();
    assert!(!apres.principal, "sans notifications, plus principal");
    assert_eq!(legacy(&ent, p.id).await.0.as_deref(), Some("un@exemple.fr"));

    // Motif trop court, limite de coordonnées, droits.
    assert!(ent.retirer_coordonnee_for_actor(etab, d, r, p.id, e2.id, RaisonCoordonnee::Autre, &Some("ab".into())).await.is_err());
    for i in 0..20 {
        let r = ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &saisie_coord("email", &format!("n{i}@exemple.fr"))).await;
        if r.is_err() {
            assert!(refus_de(r, RefusMetier::Conflit));
            break;
        }
        assert!(i < 19, "la limite de coordonnées doit s'appliquer");
    }
    assert!(matches!(
        ent.ajouter_coordonnee_for_actor(etab, cast.prof.user_id, Role::Prof.as_str(), p.id, &saisie_coord("email", "p@exemple.fr")).await,
        Err(EntDbError::Forbidden)
    ));
    // Personne d'un autre établissement : introuvable, rien n'est écrit.
    let autre = Uuid::new_v4();
    let autre_p = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, source) VALUES ($1,$2,'Ailleurs','saisie')")
        .bind(autre_p)
        .bind(autre)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(matches!(
        ent.ajouter_coordonnee_for_actor(etab, d, r, autre_p, &saisie_coord("email", "x@exemple.fr")).await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));
    assert!(lignes(&ent, autre_p).await.is_empty());
}

/// Vérification par code : e-mail et SMS, essais limités, empreinte seule en base, remplacement de
/// la famille effectif à la vérification, « ne pas solliciter » respecté, aucun nom d'enfant.
#[tokio::test]
async fn verification_par_code_et_remplacement_famille() {
    let Some((ent, cast)) = labo_mail().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let email_compte = format!("compte.{}@exemple.fr", Uuid::new_v4().simple());
    let (pid, user) = personne_avec_compte(&ent, etab, "Verification", &email_compte).await;

    // La famille ajoute un 2ᵉ e-mail : jamais principal avant vérification.
    let nouveau = format!("perso.{}@exemple.fr", Uuid::new_v4().simple());
    let c = ent.ajouter_ma_coordonnee(etab, user, &saisie_coord("email", &nouveau)).await.unwrap();
    assert!(!c.principal && c.verifiee_le.is_none() && c.source == "famille");
    assert!(matches!(
        ent.usages_ma_coordonnee(etab, user, c.id, &["notifications".into()], true).await,
        Err(EntDbError::Refus(RefusMetier::Conflit, _))
    ), "pas principal tant que non vérifié");
    let v = ent.demander_verification_ma_coordonnee(etab, user, c.id).await.unwrap();
    assert_eq!(v.canal, "email");
    let code = code_email(&ent, &nouveau);
    let corps = ent.mailer().dry_run_corps().into_iter().rev().find(|(a, _, _)| *a == nouveau).unwrap().2;
    assert!(!corps.to_lowercase().contains("scolia") || !corps.contains("élève"), "le code ne nomme ni enfant ni établissement");
    // En base : l'empreinte seule, jamais le code.
    let (hash,): (String,) = sqlx::query_as("SELECT code_hash FROM ent_coordonnee_verification WHERE coordonnee_id = $1 AND statut = 'envoye'")
        .bind(c.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(hash != code && !hash.contains(&code));
    // Mauvais code : erreur unique, essais comptés ; le bon code vérifie.
    let faux = if code == "000000" { "111111" } else { "000000" };
    assert!(refus_de(ent.verifier_ma_coordonnee(etab, user, c.id, faux).await, RefusMetier::Invalide));
    assert!(refus_de(ent.verifier_ma_coordonnee(etab, user, c.id, "abc").await, RefusMetier::Invalide));
    let ok = ent.verifier_ma_coordonnee(etab, user, c.id, &code).await.unwrap();
    assert!(ok.verifiee_le.is_some() && ok.verification_voie.as_deref() == Some("code_email"));
    assert!(refus(ent.verifier_ma_coordonnee(etab, user, c.id, &code).await), "code à usage unique");
    // Vérifiée, elle peut devenir la principale.
    let p = ent.usages_ma_coordonnee(etab, user, c.id, &["notifications".into()], true).await.unwrap();
    assert!(p.principal);
    assert_eq!(legacy(&ent, pid).await.0.as_deref(), Some(nouveau.as_str()));

    // Cinq essais manqués épuisent le code.
    let c2 = ent.ajouter_ma_coordonnee(etab, user, &saisie_coord("email", &format!("x.{}@exemple.fr", Uuid::new_v4().simple()))).await.unwrap();
    ent.demander_verification_ma_coordonnee(etab, user, c2.id).await.unwrap();
    for _ in 0..5 {
        assert!(ent.verifier_ma_coordonnee(etab, user, c2.id, "999999").await.is_err());
    }
    let (statut,): (String,) = sqlx::query_as("SELECT statut FROM ent_coordonnee_verification WHERE coordonnee_id = $1 ORDER BY envoye_le DESC LIMIT 1")
        .bind(c2.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(statut, "epuisee");

    // Remplacement par la famille : la nouvelle valeur attend sa vérification, l'ancienne reste.
    let mobile = ent.ajouter_ma_coordonnee(etab, user, &saisie_coord("mobile", "06 11 22 33 44")).await.unwrap();
    assert!(mobile.principal, "premier mobile de la fiche");
    let rempl = ent.remplacer_ma_coordonnee(etab, user, mobile.id, "07 55 66 77 88", &None).await.unwrap();
    assert!(!rempl.principal && rempl.remplace_id == Some(mobile.id));
    let l = lignes(&ent, pid).await;
    assert!(a_ligne(&l, "+33611223344", true) && a_ligne(&l, "+33755667788", true));
    ent.demander_verification_ma_coordonnee(etab, user, rempl.id).await.unwrap();
    let code = code_sms(&ent, "+33755667788");
    let ok = ent.verifier_ma_coordonnee(etab, user, rempl.id, &code).await.unwrap();
    assert!(ok.principal && ok.verification_voie.as_deref() == Some("code_sms"));
    let l = lignes(&ent, pid).await;
    assert!(a_ligne(&l, "+33611223344", false), "l'ancien mobile est fermé à la vérification");
    assert_eq!(legacy(&ent, pid).await.1.as_deref(), Some("+33755667788"));
    // Une famille ne corrige pas : c'est le secrétariat qui corrige.
    assert!(refus_de(
        ent.coord_remplacer(etab, &Auteur::Personne { actor: user }, None, ok.id, "07 55 66 77 99",
            NatureRemplacement::Correction, RaisonCoordonnee::ErreurDeSaisie, &None).await,
        RefusMetier::Interdit
    ));
    // Une famille ne change pas son adresse postale directement (proposition au secrétariat).
    assert!(refus_de(ent.ajouter_ma_coordonnee(etab, user, &saisie_coord("adresse", "1 rue de la Paix 75002 Paris")).await, RefusMetier::Interdit));
    // Fixe et adresse ne se vérifient pas par code.
    let fixe = ent.ajouter_coordonnee_for_actor(etab, d, r, pid, &saisie_coord("fixe", "01 02 03 04 05")).await.unwrap();
    assert!(refus(ent.demander_verification_for_actor(etab, d, r, pid, fixe.id).await));
    // Au plus trois codes par heure et par coordonnée.
    let c3 = ent.ajouter_ma_coordonnee(etab, user, &saisie_coord("email", &format!("y.{}@exemple.fr", Uuid::new_v4().simple()))).await.unwrap();
    for _ in 0..3 {
        ent.demander_verification_ma_coordonnee(etab, user, c3.id).await.unwrap();
    }
    assert!(refus_de(ent.demander_verification_ma_coordonnee(etab, user, c3.id).await, RefusMetier::Conflit));

    // « Ne pas solliciter » : aucun code à l'initiative de l'établissement, mais la personne
    // peut vérifier elle-même ce qu'elle a saisi.
    ent.definir_ne_pas_solliciter_for_actor(etab, d, r, pid, true, Some("ordonnance".into())).await.unwrap();
    let c4 = ent.ajouter_ma_coordonnee(etab, user, &saisie_coord("email", &format!("z.{}@exemple.fr", Uuid::new_v4().simple()))).await.unwrap();
    assert!(refus_de(ent.demander_verification_for_actor(etab, d, r, pid, c4.id).await, RefusMetier::Interdit));
    assert!(ent.demander_verification_ma_coordonnee(etab, user, c4.id).await.is_ok());
    // Aucune valeur, aucun code dans les traces.
    let traces = traces_de(&ent, pid).await;
    assert!(!traces.contains("exemple.fr") && !traces.contains("+33") && !traces.contains(&code));
}

/// Divergences compte ↔ fiche : rapportées (valeurs masquées), jamais écrasées ; décisions
/// tracées et valables tant que les valeurs ne changent pas ; reprise d'une base existante.
#[tokio::test]
async fn divergences_compte_fiche_rapportees_jamais_ecrasees() {
    let Some((ent, cast)) = labo_mail().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let email_compte = format!("login.{}@exemple.fr", Uuid::new_v4().simple());
    let (pid, user) = personne_avec_compte(&ent, etab, "Divergence", &email_compte).await;
    // Le compte devient une coordonnée de la fiche (usage connexion) sans écraser quoi que ce soit.
    let c = ent.coordonnees_personne_for_actor(etab, d, r, pid, false).await.unwrap();
    let compte = c.coordonnees.iter().find(|c| c.valeur == email_compte).expect("compte dans la fiche");
    assert!(compte.usages.contains(&"connexion".to_string()) && compte.verification_voie.as_deref() == Some("compte"));
    assert!(compte.principal, "seul e-mail de la fiche : principal");
    assert!(ent.divergences_compte_for_actor(etab, d, r).await.unwrap().iter().all(|x| x.personne_id != pid));

    // La fiche reçoit un autre e-mail principal : divergence « different_du_principal ».
    let mut s = saisie_coord("email", &format!("perso.{}@exemple.fr", Uuid::new_v4().simple()));
    s.principal = true;
    let perso = ent.ajouter_coordonnee_for_actor(etab, d, r, pid, &s).await.unwrap();
    let div = ent.divergences_compte_for_actor(etab, d, r).await.unwrap();
    let mine = div.iter().find(|x| x.personne_id == pid && x.champ == "email").expect("divergence rapportée");
    assert_eq!(mine.nature, "different_du_principal");
    assert!(!mine.valeur_compte_masquee.contains("exemple") || mine.valeur_compte_masquee.contains("•••"));
    assert!(!mine.valeur_compte_masquee.contains(&email_compte), "valeur masquée");
    // Le compte n'a pas bougé et la fiche non plus.
    let (u,): (String,) = sqlx::query_as("SELECT email FROM ent_user WHERE id = $1").bind(user).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(u, email_compte);
    assert_eq!(legacy(&ent, pid).await.0.as_deref(), Some(perso.valeur.as_str()));
    // La personne voit sa propre divergence ; un autre compte ne la voit pas.
    assert_eq!(ent.mes_divergences_compte(etab, user).await.unwrap().len(), 1);
    let (autre_p, autre_user) = personne_avec_compte(&ent, etab, "Voisin", &format!("v.{}@exemple.fr", Uuid::new_v4().simple())).await;
    assert!(ent.mes_divergences_compte(etab, autre_user).await.unwrap().is_empty());
    let _ = autre_p;

    // Décision « volontairement distincts » : tracée, durable… jusqu'à ce qu'une valeur change.
    ent.resoudre_divergence_for_actor(etab, d, r, pid, "email", ChoixDivergence::DistinctVoulu, &Some("login pro".into())).await.unwrap();
    assert!(ent.divergences_compte_for_actor(etab, d, r).await.unwrap().iter().all(|x| x.personne_id != pid));
    assert!(refus(ent.resoudre_divergence_for_actor(etab, d, r, pid, "email", ChoixDivergence::DistinctVoulu, &None).await), "plus rien à résoudre");
    // L'e-mail du compte change (confirmation en deux temps) : la divergence revient.
    let nouveau_compte = format!("nouveau.{}@exemple.fr", Uuid::new_v4().simple());
    sqlx::query("UPDATE ent_user SET email = $2 WHERE id = $1").bind(user).bind(&nouveau_compte).execute(ent.pool()).await.unwrap();
    let div = ent.divergences_compte_for_actor(etab, d, r).await.unwrap();
    assert_eq!(div.iter().find(|x| x.personne_id == pid).map(|x| x.nature.as_str()), Some("absent_de_la_fiche"));
    // Adopter la valeur du compte : elle devient principale, l'ancienne reste dans la fiche.
    ent.resoudre_ma_divergence(etab, user, "email", ChoixDivergence::AdopterCompte, &None).await.unwrap();
    let l = lignes(&ent, pid).await;
    assert!(l.iter().any(|(t, v, o, p)| t == "email" && *v == nouveau_compte && *o && *p));
    assert!(a_ligne(&l, &perso.valeur, true), "l'ancien principal n'est pas supprimé");
    assert!(ent.divergences_compte_for_actor(etab, d, r).await.unwrap().iter().all(|x| x.personne_id != pid));
    let h = ent.mon_historique_coordonnees(etab, user).await.unwrap();
    assert!(h.iter().any(|x| x.nature == "divergence_resolue" && x.raison == "divergence_compte" && x.par_nom.is_none()));
    assert!(ent.resoudre_divergence_for_actor(etab, cast.prof.user_id, Role::Prof.as_str(), pid, "email", ChoixDivergence::ConserverFiche, &None).await.is_err());

    // Reprise d'une base existante : coordonnées de la fiche et du compte reprises, divergences
    // rapportées, rien d'écrasé (déclencheurs coupés pour simuler des données d'avant le lot).
    let u2 = Uuid::new_v4();
    let email_u2 = format!("u2.{}@exemple.fr", Uuid::new_v4().simple());
    sqlx::query("INSERT INTO ent_user (id, email, telephone, password_hash, display_name, created_at, updated_at) VALUES ($1,$2,'06 00 00 00 01','x','Ancien', now(), now())")
        .bind(u2).bind(&email_u2).execute(ent.pool()).await.unwrap();
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1,$2,'parent', now())")
        .bind(u2).bind(etab).execute(ent.pool()).await.unwrap();
    let ancien = Uuid::new_v4();
    let mut tx = ent.pool().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_personne DISABLE TRIGGER ent_personne_coordonnee_legacy").execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, email, telephone, telephone_2, adresse, user_id, source) VALUES ($1,$2,'Reprise','Fiche.Ancienne@Exemple.fr','06 22 33 44 55','bizarre 12',  '3 impasse X', $3, 'reprise')")
        .bind(ancien).bind(etab).bind(u2).execute(&mut *tx).await.unwrap();
    sqlx::query("ALTER TABLE ent_personne ENABLE TRIGGER ent_personne_coordonnee_legacy").execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    assert!(lignes(&ent, ancien).await.is_empty());
    sqlx::query("DELETE FROM ent_coord_reprise_marque WHERE cle = '273_reprise'").execute(ent.pool()).await.unwrap();
    ent.migrate().await.unwrap();
    let l = lignes(&ent, ancien).await;
    assert!(a_ligne(&l, "fiche.ancienne@exemple.fr", true), "e-mail de la fiche repris");
    assert!(a_ligne(&l, "+33622334455", true), "téléphone normalisé");
    assert!(a_ligne(&l, "bizarre 12", true), "numéro illisible gardé tel quel");
    assert!(a_ligne(&l, "3 impasse X", true));
    assert!(a_ligne(&l, &email_u2, true), "e-mail du compte ajouté (usage connexion)");
    assert!(a_ligne(&l, "+33600000001", true), "téléphone du compte ajouté");
    let (a_verifier,): (bool,) = sqlx::query_as("SELECT a_verifier FROM ent_personne_coordonnee WHERE personne_id = $1 AND valeur = 'bizarre 12'")
        .bind(ancien).fetch_one(ent.pool()).await.unwrap();
    assert!(a_verifier);
    let (email, tel, _, _) = legacy(&ent, ancien).await;
    assert_eq!(email.as_deref(), Some("fiche.ancienne@exemple.fr"));
    assert_eq!(tel.as_deref(), Some("+33622334455"), "la fiche reste la référence, normalisée");
    let div = ent.divergences_compte_for_actor(etab, d, r).await.unwrap();
    let champs: Vec<&str> = div.iter().filter(|x| x.personne_id == ancien).map(|x| x.champ.as_str()).collect();
    assert!(champs.contains(&"email") && champs.contains(&"telephone"), "divergences du rapport de reprise : {champs:?}");
    // Rejouer la reprise ne change rien (idempotente) : même nombre de lignes.
    let avant = lignes(&ent, ancien).await.len();
    sqlx::query("DELETE FROM ent_coord_reprise_marque WHERE cle = '273_reprise'").execute(ent.pool()).await.unwrap();
    ent.migrate().await.unwrap();
    assert_eq!(lignes(&ent, ancien).await.len(), avant);
}

/// Préférences et langue (ADM-07) : validation, jamais « oublié », versions, événement sans valeur.
#[tokio::test]
async fn preferences_langue_canaux_et_versions() {
    let Some((ent, cast)) = labo_mail().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let (pid, user) = personne_avec_compte(&ent, etab, "Preferences", &format!("pref.{}@exemple.fr", Uuid::new_v4().simple())).await;
    let defaut = ent.mes_preferences(etab, user).await.unwrap();
    assert_eq!(defaut, PreferencesPersonne::default());

    let voulu = PreferencesPersonne {
        langue: Some("FA-af".into()),
        canaux_refuses: vec!["sms".into()],
        courrier_papier: true,
        accessibilite: vec!["gros_caracteres".into(), "interprete".into()],
        joignable: Some(serde_json::json!([{"jours":[1,2],"de":"08:00","a":"12:00"}])),
        fuseau: Some("Asia/Kabul".into()),
        maj_le: None,
    };
    let p = ent.definir_mes_preferences(etab, user, &voulu).await.unwrap();
    assert_eq!(p.langue.as_deref(), Some("fa-af"));
    assert_eq!(p.canaux_refuses, vec!["sms".to_string()]);
    assert!(p.courrier_papier && p.maj_le.is_some());
    // Le personnel lit les mêmes préférences ; l'envoi aux familles respecte le canal refusé.
    assert_eq!(ent.preferences_personne_for_actor(etab, d, r, pid).await.unwrap().langue.as_deref(), Some("fa-af"));
    let (canal_sms,): (bool,) = sqlx::query_as("SELECT canal_sms FROM ent_personne_preference WHERE personne_id = $1").bind(pid).fetch_one(ent.pool()).await.unwrap();
    assert!(!canal_sms);
    // Refus ou valeurs invalides.
    for mauvais in [
        PreferencesPersonne { langue: Some("francais".into()), ..voulu.clone() },
        PreferencesPersonne { canaux_refuses: vec!["pigeon".into()], ..voulu.clone() },
        PreferencesPersonne { fuseau: Some("Mars/Olympus".into()), ..voulu.clone() },
        PreferencesPersonne { joignable: Some(serde_json::json!("jamais")), ..voulu.clone() },
        PreferencesPersonne { canaux_refuses: vec!["email".into(), "sms".into()], courrier_papier: false, ..voulu.clone() },
    ] {
        assert!(ent.definir_mes_preferences(etab, user, &mauvais).await.is_err());
    }
    assert_eq!(ent.mes_preferences(etab, user).await.unwrap().langue.as_deref(), Some("fa-af"), "rien d'écrit en cas de refus");
    // Sans changement : aucune version supplémentaire.
    let n = |ent: &PgEnt| {
        let pool = ent.pool().clone();
        async move {
            let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_personne_preference_version WHERE personne_id = $1").bind(pid).fetch_one(&pool).await.unwrap();
            n
        }
    };
    let avant = n(&ent).await;
    ent.definir_mes_preferences(etab, user, &voulu).await.unwrap();
    assert_eq!(n(&ent).await, avant);
    // Façade du lot A0 : même table, mêmes règles, version ajoutée.
    ent.definir_preferences_for_actor(etab, d, r, pid, true, true, false).await.unwrap();
    assert_eq!(n(&ent).await, avant + 1);
    assert!(ent.mes_preferences(etab, user).await.unwrap().canaux_refuses.is_empty());
    // Les traces portent les champs modifiés, jamais les valeurs.
    let traces = traces_de(&ent, pid).await;
    assert!(traces.contains("preferences_modifiees") || traces.contains("champs"));
    assert!(!traces.contains("fa-af") && !traces.contains("Kabul") && !traces.contains("08:00"));
    // Langue de la préinscription reprise comme repli.
    let (p2, u2) = personne_avec_compte(&ent, etab, "Repli", &format!("repli.{}@exemple.fr", Uuid::new_v4().simple())).await;
    sqlx::query("INSERT INTO ent_personne_reserve (id, etablissement_id, personne_id, langue) VALUES (gen_random_uuid(), $1, $2, 'ar')")
        .bind(etab).bind(p2).execute(ent.pool()).await.unwrap();
    assert_eq!(ent.mes_preferences(etab, u2).await.unwrap().langue.as_deref(), Some("ar"));
    // Un parent ne lit pas les préférences d'une autre personne.
    assert!(matches!(
        ent.preferences_personne_for_actor(etab, cast.parent_lea.user_id, Role::Parent.as_str(), pid).await,
        Err(EntDbError::Forbidden)
    ));
}

/// Consentements (ADM-12) : texte versionné, papier avec pièce, retrait, historique, codes seuls
/// dans l'événement.
#[tokio::test]
async fn consentements_administratifs_texte_versionne_et_retrait() {
    let Some((ent, cast)) = labo_mail().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let (pid, user) = personne_avec_compte(&ent, etab, "Consent", &format!("cons.{}@exemple.fr", Uuid::new_v4().simple())).await;
    let version = crate::dossier_personnes::CONSENTEMENTS[0].1;
    let objet = crate::dossier_personnes::CONSENTEMENTS[0].0;
    // Refusé par défaut : aucune ligne.
    assert!(ent.mes_consentements(etab, user).await.unwrap().is_empty());
    // Texte périmé, objet inconnu : refusés.
    assert!(refus_de(ent.enregistrer_mon_consentement(etab, user, objet, true, "1999-v0").await, RefusMetier::Conflit));
    assert!(ent.enregistrer_mon_consentement(etab, user, "pigeon", true, version).await.is_err());
    let c = ent.enregistrer_mon_consentement(etab, user, objet, true, version).await.unwrap();
    assert!(c.reponse == "accorde" && c.canal == "portail" && c.retire_le.is_none());
    // Une nouvelle réponse ferme la précédente, sans rien perdre.
    let c2 = ent.enregistrer_mon_consentement(etab, user, objet, false, version).await.unwrap();
    assert_eq!(c2.reponse, "refuse");
    let tout = ent.mes_consentements(etab, user).await.unwrap();
    assert_eq!(tout.len(), 2);
    assert!(tout.iter().filter(|c| c.retire_le.is_none()).count() == 1);
    // Le personnel recueille un consentement papier : la référence de la pièce est obligatoire.
    assert!(refus(ent.enregistrer_consentement_for_actor(etab, d, r, pid, objet, true, version, Some("papier"), None).await));
    let p = ent.enregistrer_consentement_for_actor(etab, d, r, pid, objet, true, version, Some("papier"), Some("classeur 2026-11 n°42")).await.unwrap();
    assert!(p.canal == "papier" && p.piece_reference.is_some());
    assert!(ent.enregistrer_consentement_for_actor(etab, d, r, pid, objet, true, version, Some("portail"), None).await.is_err(), "le personnel ne se fait pas passer pour le portail");
    // Retrait à tout moment ; retirer deux fois : rien à retirer.
    ent.retirer_mon_consentement(etab, user, objet).await.unwrap();
    assert!(refus_de(ent.retirer_mon_consentement(etab, user, objet).await, RefusMetier::Conflit));
    assert!(ent.mes_consentements(etab, user).await.unwrap().iter().all(|c| c.retire_le.is_some()));
    // Événement : objet et réponse sous forme de codes, aucune pièce, aucun nom.
    let evts: Vec<(String,)> = sqlx::query_as("SELECT payload::text FROM ent_metier_event WHERE aggregate_id = $1 AND type = 'personne.consentement_modifie'")
        .bind(pid).fetch_all(ent.pool()).await.unwrap();
    assert!(evts.len() >= 4);
    assert!(evts.iter().all(|(t,)| !t.contains("classeur") && !t.contains("Consent")));
    // Un parent ne lit ni n'écrit les consentements d'une autre personne.
    assert!(matches!(
        ent.consentements_personne_for_actor(etab, cast.parent_lea.user_id, Role::Parent.as_str(), pid).await,
        Err(EntDbError::Forbidden)
    ));
}

/// Identité versionnée (ADM-03) : correction ≠ changement, motif obligatoire, anciennes valeurs
/// sensibles scellées et réservées aux données protégées, tout écrivain historique laisse une version.
#[tokio::test]
async fn identite_versionnee_changement_sensible_scelle() {
    let Some((ent, cast)) = labo_mail().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let (adm, ra) = (cast.administratif.user_id, Role::Administratif.as_str());
    let mut s = saisie("Identite", None, None);
    s.prenom = Some("Sam".into());
    s.civilite = Some("m".into());
    let p = ent.creer_personne_for_actor(etab, d, r, &s).await.unwrap();
    let h = ent.historique_identite_for_actor(etab, d, r, p.id).await.unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].nature, "creation");
    let id = |nom: &str, prenom: &str, civ: Option<&str>| IdentiteSaisie {
        nom: nom.into(),
        prenom: Some(prenom.into()),
        civilite: civ.map(Into::into),
    };
    // Motif obligatoire ; rien à changer ; date future ; droits.
    assert!(refus(ent.modifier_identite_for_actor(etab, d, r, p.id, &id("Identite", "Samy", Some("m")), NatureIdentite::Correction, None, "").await));
    assert!(ent.modifier_identite_for_actor(etab, d, r, p.id, &id("Identite", "Sam", Some("m")), NatureIdentite::Correction, None, "rien changé").await.is_err());
    let demain = domain_eleve::horloge::jour_paris() + chrono::Duration::days(3);
    assert!(ent.modifier_identite_for_actor(etab, d, r, p.id, &id("Identite", "Samy", Some("m")), NatureIdentite::Correction, Some(demain), "futur").await.is_err());
    assert!(matches!(
        ent.modifier_identite_for_actor(etab, cast.prof.user_id, Role::Prof.as_str(), p.id, &id("Identite", "Samy", Some("m")), NatureIdentite::Correction, None, "droits").await,
        Err(EntDbError::Forbidden)
    ));
    // Correction : visible dans les versions, pas sensible.
    ent.modifier_identite_for_actor(etab, d, r, p.id, &id("Identite", "Samy", Some("m")), NatureIdentite::Correction, None, "faute de frappe").await.unwrap();
    // Changement de prénom et de civilité : sensible, anciennes valeurs scellées.
    let p2 = ent.modifier_identite_for_actor(etab, d, r, p.id, &id("Identite", "Samia", Some("mme")), NatureIdentite::Changement, None, "prénom d'usage").await.unwrap();
    assert_eq!((p2.prenom.as_deref(), p2.civilite.as_deref()), (Some("Samia"), Some("mme")));
    #[allow(clippy::type_complexity)]
    let brut: Vec<(Option<String>, Option<String>, bool, bool, Option<String>)> = sqlx::query_as(
        "SELECT prenom, civilite, masquee, sensible, valeurs_anterieures FROM ent_personne_version WHERE personne_id = $1 ORDER BY enregistre_le, id")
        .bind(p.id).fetch_all(ent.pool()).await.unwrap();
    assert_eq!(brut.len(), 3);
    for v in &brut[..2] {
        assert!(v.0.is_none() && v.1.is_none() && v.2, "anciennes valeurs retirées des versions précédentes");
    }
    assert!(brut[2].3 && brut[2].0.as_deref() == Some("Samia"));
    let scelle = brut[2].4.clone().expect("anciennes valeurs scellées");
    assert!(scelle.starts_with("enc:v1:") && !scelle.contains("Sam"));
    // Un second changement conserve toute la chaîne des anciennes valeurs.
    ent.modifier_identite_for_actor(etab, d, r, p.id, &id("Identite", "Sasha", Some("autre")), NatureIdentite::Changement, None, "second changement").await.unwrap();
    // Lecture : le secrétariat voit l'historique sans les anciennes valeurs ; la direction les lit.
    let vue_adm = ent.historique_identite_for_actor(etab, adm, ra, p.id).await.unwrap();
    assert!(vue_adm.iter().all(|v| v.anterieures.is_none()));
    assert!(vue_adm.iter().any(|v| v.sensible && v.nature == "changement"));
    let vue_dir = ent.historique_identite_for_actor(etab, d, r, p.id).await.unwrap();
    let ant = vue_dir.iter().find_map(|v| v.anterieures.clone()).expect("anciennes valeurs lues par la direction");
    let prenoms: Vec<Option<&str>> = ant.iter().map(|a| a.prenom.as_deref()).collect();
    assert!(prenoms.contains(&Some("Sam")) && prenoms.contains(&Some("Samy")) && prenoms.contains(&Some("Samia")), "{prenoms:?}");
    // Les traces : noms de champs, jamais une valeur.
    let traces = traces_de(&ent, p.id).await;
    assert!(traces.contains("\"champs\""));
    for v in ["Sam", "Samy", "Samia", "Sasha", "Identite"] {
        assert!(!traces.contains(v), "valeur « {v} » dans les traces");
    }
    // Un écrivain historique ne contourne pas les versions.
    sqlx::query("UPDATE ent_personne SET nom = 'Identite-Dupont' WHERE id = $1").bind(p.id).execute(ent.pool()).await.unwrap();
    let (derniere,): (String,) = sqlx::query_as("SELECT nature FROM ent_personne_version WHERE personne_id = $1 ORDER BY enregistre_le DESC, id DESC LIMIT 1")
        .bind(p.id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(derniere, "correction");
}

/// Séparation en lot du nom et du prénom (ADM-10) : proposition, validation humaine, garde-fous.
#[tokio::test]
async fn separation_nom_prenom_en_lot() {
    let Some((ent, cast)) = labo_mail().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let mk = |nom: String| {
        let ent = &ent;
        async move {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, source) VALUES ($1,$2,$3,'compte')")
                .bind(id).bind(etab).bind(nom).execute(ent.pool()).await.unwrap();
            id
        }
    };
    let tag = Uuid::new_v4().simple().to_string();
    let a = mk(format!("Marie Dupont{tag}")).await;
    let maj = tag.to_uppercase();
    let b = mk(format!("DURAND{maj} Paul")).await;
    let c = mk(format!("Jean Pierre Moreau{tag}")).await;
    let props = ent.propositions_separation_noms_for_actor(etab, d, r, 500).await.unwrap();
    let pa = props.iter().find(|p| p.personne_id == a).expect("proposée");
    assert_eq!((pa.prenom_propose.as_str(), pa.nom_propose.as_str()), ("Marie", &*format!("Dupont{tag}")));
    assert!(!pa.a_verifier);
    let pb = props.iter().find(|p| p.personne_id == b).unwrap();
    assert_eq!(pb.prenom_propose, "Paul");
    assert!(props.iter().find(|p| p.personne_id == c).unwrap().a_verifier);
    // Garde-fous : mots modifiés, doublon dans le lot, droits, fiche déjà dotée d'un prénom.
    let sep = |p: Uuid, nom: &str, prenom: &str| SeparationNom { personne_id: p, nom: nom.into(), prenom: prenom.into() };
    assert!(ent.separer_noms_for_actor(etab, d, r, &[sep(a, "Dupond", "Marie")]).await.is_err(), "lettres modifiées");
    assert!(ent.separer_noms_for_actor(etab, d, r, &[sep(a, &format!("Dupont{tag}"), "Marie"), sep(a, &format!("Dupont{tag}"), "Marie")]).await.is_err());
    assert!(ent.separer_noms_for_actor(etab, d, r, &[]).await.is_err());
    assert!(matches!(
        ent.separer_noms_for_actor(etab, cast.prof.user_id, Role::Prof.as_str(), &[sep(a, &format!("Dupont{tag}"), "Marie")]).await,
        Err(EntDbError::Forbidden)
    ));
    // Tout ou rien : une séparation fausse annule le lot.
    assert!(ent.separer_noms_for_actor(etab, d, r, &[sep(a, &format!("Dupont{tag}"), "Marie"), sep(b, "Autre", "Chose")]).await.is_err());
    let (nom_a,): (String,) = sqlx::query_as("SELECT nom FROM ent_personne WHERE id = $1").bind(a).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(nom_a, format!("Marie Dupont{tag}"), "rien d'écrit");
    // Validation humaine : le découpage de c est corrigé à la main (prénom composé).
    let n = ent
        .separer_noms_for_actor(etab, d, r, &[
            sep(a, &format!("Dupont{tag}"), "Marie"),
            sep(b, &format!("DURAND{maj}"), "Paul"),
            sep(c, &format!("Moreau{tag}"), "Jean Pierre"),
        ])
        .await
        .unwrap();
    assert_eq!(n, 3);
    let (nom_c, prenom_c): (String, Option<String>) = sqlx::query_as("SELECT nom, prenom FROM ent_personne WHERE id = $1").bind(c).fetch_one(ent.pool()).await.unwrap();
    assert_eq!((nom_c, prenom_c.as_deref()), (format!("Moreau{tag}"), Some("Jean Pierre")));
    let (nature,): (String,) = sqlx::query_as("SELECT nature FROM ent_personne_version WHERE personne_id = $1 ORDER BY enregistre_le DESC, id DESC LIMIT 1").bind(a).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(nature, "separation_noms");
    assert!(!ent.propositions_separation_noms_for_actor(etab, d, r, 500).await.unwrap().iter().any(|p| p.personne_id == a));
    assert!(ent.separer_noms_for_actor(etab, d, r, &[sep(a, &format!("Dupont{tag}"), "Marie")]).await.is_err(), "déjà un prénom");
}

/// Rattachement d'un compte : son e-mail devient une coordonnée de la fiche (usage connexion),
/// utilisée pour les envois seulement si la fiche n'en a pas d'autre ; adresse protégée jamais en clair.
#[tokio::test]
async fn compte_rattache_est_une_coordonnee_et_adresse_protegee_jamais_ecrite() {
    let Some((ent, cast)) = labo_mail().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    // Une fiche sans e-mail reçoit le compte : il sert aux notifications (comportement d'avant).
    let p = ent.creer_personne_for_actor(etab, d, r, &saisie("Rattache", None, None)).await.unwrap();
    let email = format!("rattache.{}@exemple.fr", Uuid::new_v4().simple());
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at) VALUES ($1,$2,'x','R', now(), now())").bind(user).bind(&email).execute(ent.pool()).await.unwrap();
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1,$2,'parent', now())").bind(user).bind(etab).execute(ent.pool()).await.unwrap();
    ent.rattacher_compte_for_actor(etab, d, r, p.id, user).await.unwrap();
    let c = ent.coordonnees_personne_for_actor(etab, d, r, p.id, false).await.unwrap();
    let ligne = c.coordonnees.iter().find(|c| c.valeur == email).expect("compte = coordonnée");
    assert!(ligne.principal && ligne.usages.contains(&"connexion".to_string()) && ligne.usages.contains(&"notifications".to_string()));
    assert_eq!(legacy(&ent, p.id).await.0.as_deref(), Some(email.as_str()));

    // Adresse protégée en attente de la direction : aucune adresse en clair n'est écrite.
    sqlx::query("INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine) VALUES (gen_random_uuid(), $1, $2, 'enc:v1:x', 'saisie')")
        .bind(etab).bind(p.id).execute(ent.pool()).await.unwrap();
    assert!(refus_de(ent.ajouter_coordonnee_for_actor(etab, d, r, p.id, &saisie_coord("adresse", "5 rue Secrète 75001 Paris")).await, RefusMetier::Interdit));
    assert!(lignes(&ent, p.id).await.iter().all(|(t, _, _, _)| t != "adresse"));
    // Le personnel sans droit sur les adresses (F2) ne les voit pas dans la fiche.
    let (autre_p, _) = personne_avec_compte(&ent, etab, "SansAdresse", &format!("sa.{}@exemple.fr", Uuid::new_v4().simple())).await;
    ent.ajouter_coordonnee_for_actor(etab, d, r, autre_p, &saisie_coord("adresse", "9 avenue des Fleurs 69001 Lyon")).await.unwrap();
    let vue = ent.coordonnees_personne_for_actor(etab, cast.cpe.user_id, Role::Cpe.as_str(), autre_p, false).await;
    match vue {
        Ok(v) => assert!(v.adresses_masquees && v.coordonnees.iter().all(|c| c.type_ != "adresse")),
        Err(EntDbError::Forbidden) => {}
        Err(e) => panic!("{e:?}"),
    }
}

/// L'équivalent Rust de la normalisation E.164 concorde avec la fonction SQL `ent_tel_e164`.
#[tokio::test]
async fn normalisation_e164_concorde_avec_sql() {
    let Some((ent, _)) = labo_mail().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    for brut in [
        "06 12 34 56 78", "06.12.34.56.78", "+33 6 12 34 56 78", "+33 (0)6 12 34 56 78", "0033612345678",
        "01 02 03 04 05", "+44 7911 123456", "0044 7911 123456", "+262 692 12 34 56", "12345", "+3361234",
        "+33 0 12 34 56 78", "00 12 34 56 78", "06 12 34 56 7a", "", "+0612345678", "0612345678", "0912345678",
        "+33912345678", "+1 202 555 0143", "(06) 12-34/56 78", "00 33 6 12 34 56 78", "+33 1234", "0212",
        "+3 12345678901234", "+3 123456789012345",
    ] {
        let (sql,): (Option<String>,) = sqlx::query_as("SELECT ent_tel_e164($1)").bind(brut).fetch_one(ent.pool()).await.unwrap();
        assert_eq!(crate::dossier_personnes::normaliser_telephone(brut), sql, "« {brut} »");
    }
}

/// Revue de sécurité du lot : une coordonnée non vérifiée de la famille n'est jamais promue
/// principale automatiquement, la coordonnée du compte ne se retire pas, le motif du personnel
/// reste interne, un e-mail multi-destinataires est refusé, les anciennes valeurs d'un prénom
/// renseigné sont scellées même en « correction ».
#[tokio::test]
async fn revue_securite_a2_promotion_motif_compte_et_correction_sensible() {
    let Some((ent, cast)) = labo_mail().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (d, r) = dir(&cast);
    let email_compte = format!("revue.{}@exemple.fr", Uuid::new_v4().simple());
    let (pid, user) = personne_avec_compte(&ent, etab, "Revue", &email_compte).await;
    // Le principal (compte) est retiré par le personnel ; la coordonnée non vérifiée de la famille
    // ne prend pas sa place.
    let c = ent.coordonnees_personne_for_actor(etab, d, r, pid, false).await.unwrap();
    let compte = c.coordonnees.iter().find(|c| c.valeur == email_compte).unwrap().clone();
    assert!(refus_de(
        ent.retirer_coordonnee_for_actor(etab, d, r, pid, compte.id, RaisonCoordonnee::Autre, &None).await,
        RefusMetier::Conflit
    ), "la coordonnée du compte ne se retire pas de la fiche");
    let mut s = saisie_coord("email", &format!("pro.{}@exemple.fr", Uuid::new_v4().simple()));
    s.principal = true;
    let pro = ent.ajouter_coordonnee_for_actor(etab, d, r, pid, &s).await.unwrap();
    let famille = ent.ajouter_ma_coordonnee(etab, user, &saisie_coord("email", &format!("fam.{}@exemple.fr", Uuid::new_v4().simple()))).await.unwrap();
    assert!(!famille.principal);
    ent.retirer_coordonnee_for_actor(etab, d, r, pid, pro.id, RaisonCoordonnee::Autre, &Some("motif interne sensible".into())).await.unwrap();
    let l = ent.mes_coordonnees(etab, user).await.unwrap().coordonnees;
    assert!(!l.iter().find(|c| c.id == famille.id).unwrap().principal, "non vérifiée : jamais promue");
    assert!(l.iter().find(|c| c.id == compte.id).unwrap().principal, "la plus ancienne éligible, vérifiée");
    // Le motif du personnel reste interne.
    let h = ent.mon_historique_coordonnees(etab, user).await.unwrap();
    assert!(h.iter().all(|x| x.motif.is_none() || x.via != "personnel"));
    let hp = ent.historique_coordonnees_for_actor(etab, d, r, pid).await.unwrap();
    assert!(hp.iter().any(|x| x.motif.as_deref() == Some("motif interne sensible")));
    // E-mails à plusieurs destinataires ou avec nom affiché : refusés.
    for mauvais in ["a@x.fr,b@y.fr", "<a@x.fr>", "a@x.fr;b@y.fr", "\"a\"@x.fr"] {
        assert!(ent.ajouter_coordonnee_for_actor(etab, d, r, pid, &saisie_coord("email", mauvais)).await.is_err(), "{mauvais}");
    }
    // Un prénom déjà renseigné modifié en « correction » est scellé lui aussi.
    let mut s = saisie("Revue2", None, None);
    s.prenom = Some("Ancien".into());
    let p = ent.creer_personne_for_actor(etab, d, r, &s).await.unwrap();
    ent.modifier_identite_for_actor(etab, d, r, p.id,
        &IdentiteSaisie { nom: "Revue2".into(), prenom: Some("Nouveau".into()), civilite: None },
        NatureIdentite::Correction, None, "correction d'un prénom").await.unwrap();
    let clair: Vec<(Option<String>,)> = sqlx::query_as("SELECT prenom FROM ent_personne_version WHERE personne_id = $1 AND prenom = 'Ancien'")
        .bind(p.id).fetch_all(ent.pool()).await.unwrap();
    assert!(clair.is_empty(), "l'ancien prénom n'est plus en clair");
    // Plafond par personne : retirer puis rajouter une coordonnée ne remet pas le compteur à zéro.
    let (p3, u3) = personne_avec_compte(&ent, etab, "Plafond", &format!("pl.{}@exemple.fr", Uuid::new_v4().simple())).await;
    let mut refuse = false;
    for i in 0..15 {
        let c = ent.ajouter_ma_coordonnee(etab, u3, &saisie_coord("email", &format!("rb{i}.{}@exemple.fr", Uuid::new_v4().simple()))).await.unwrap();
        if ent.demander_verification_ma_coordonnee(etab, u3, c.id).await.is_err() {
            refuse = true;
            break;
        }
        ent.retirer_ma_coordonnee(etab, u3, c.id, &None).await.unwrap();
    }
    assert!(refuse, "le plafond par personne doit s'appliquer");
    let _ = p3;
}
