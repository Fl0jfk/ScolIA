//! Socle 27, **lot G4-2** : contrôles internes (passkey récente), traces d'un sujet, alertes déterministes, consultation
//! différée, nom d'agent sur arbitrage, purge par partition, section « consultations » de l'extrait, réglages.
//! Aucune heure réelle dans les assertions (règle 33) : chaque instant est injecté ; les traces sont écrites avec une date
//! fixe. Base exigée sous `CI` ou `ENT_EXIGER_BASE`.

use crate::droits_repo::{Droit, RoleSaisie};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tests_rgpd_g3b2::{demande, labo, DIR};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use ent_auth::{Action, Portee};
use uuid::Uuid;
use webauthn_authenticator_rs::prelude::WebauthnAuthenticator;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;

const PROF: &str = "prof";

fn le(a: i32, m: u32, j: u32, h: u32, mi: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(a, m, j, h, mi, 0).unwrap()
}

fn origine() -> url::Url {
    url::Url::parse("http://localhost:3000").unwrap()
}

async fn enroler(ent: &PgEnt, etab: Uuid, user: Uuid) -> WebauthnAuthenticator<SoftPasskey> {
    let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let (token, options) = ent
        .passkey_register_start_for_actor(etab, user)
        .await
        .expect("start enrôlement");
    let ccr: ent_auth::CreationChallengeResponse = serde_json::from_value(options).unwrap();
    let reg = dev.do_registration(origine(), ccr).unwrap();
    ent.passkey_register_finish_for_actor(etab, user, &token, &reg, Some("Clé".into()))
        .await
        .expect("enrôlement");
    dev
}

fn assertion(
    dev: &mut WebauthnAuthenticator<SoftPasskey>,
    options: &serde_json::Value,
) -> ent_auth::PublicKeyCredential {
    let rcr: ent_auth::RequestChallengeResponse = serde_json::from_value(options.clone()).unwrap();
    dev.do_authentication(origine(), rcr).unwrap()
}

fn est_conflit(e: &EntDbError) -> bool {
    matches!(
        e,
        EntDbError::Refus(RefusMetier::Conflit, _) | EntDbError::ConflitDossier(_)
    )
}

fn est_introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound))
}

/// Écrit **directement** une trace datée (la partition de son mois est créée) : fonction et service fixes.
#[allow(clippy::too_many_arguments)]
async fn inserer_trace(
    ent: &PgEnt,
    etab: Uuid,
    quand: DateTime<Utc>,
    acteur: Uuid,
    role: &str,
    eleve: Option<Uuid>,
    section: &str,
    hors: bool,
    protegee: bool,
    affichable: bool,
) -> Uuid {
    let p = ent.pool_verif();
    sqlx::query("SELECT public.acc_trace_assurer_partition($1::date)")
        .bind(quand.date_naive())
        .execute(p)
        .await
        .unwrap();
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO acc_trace (id, etablissement_id, le, acteur_compte_id, acteur_role, fonction_code, fonction_libelle,
                                service_code, service_libelle, nature, section, action, sujet_eleve_id, hors_perimetre,
                                motif_code, affichable, protegee)
         VALUES ($1, $2, $3, $4, $5, 'enseignant', 'Enseignant', 'enseignement', 'Enseignement', 'consultation', $6,
                 'fiche', $7, $8, $9, $10, $11)",
    )
    .bind(id)
    .bind(etab)
    .bind(quand)
    .bind(acteur)
    .bind(role)
    .bind(section)
    .bind(eleve)
    .bind(hors)
    .bind(if hors { Some("urgence") } else { None })
    .bind(affichable)
    .bind(protegee)
    .execute(p)
    .await
    .unwrap();
    id
}

/// `n` traces d'un agent, une toutes les dix secondes à partir de `debut`.
async fn rafale(
    ent: &PgEnt,
    etab: Uuid,
    debut: DateTime<Utc>,
    acteur: Uuid,
    role: &str,
    n: i32,
    hors: bool,
) {
    sqlx::query("SELECT public.acc_trace_assurer_partition($1::date)")
        .bind(debut.date_naive())
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO acc_trace (id, etablissement_id, le, acteur_compte_id, acteur_role, fonction_code, fonction_libelle,
                                service_code, service_libelle, nature, section, action, hors_perimetre, motif_code,
                                affichable, protegee)
         SELECT gen_random_uuid(), $1, $2::timestamptz + g * interval '10 seconds', $3, $4, 'enseignant', 'Enseignant',
                'enseignement', 'Enseignement', 'consultation', 'dossier', 'fiche', $6, CASE WHEN $6 THEN 'urgence' END,
                true, false
           FROM generate_series(0, $5 - 1) g",
    )
    .bind(etab)
    .bind(debut)
    .bind(acteur)
    .bind(role)
    .bind(n)
    .bind(hors)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

async fn nb_alertes(ent: &PgEnt, etab: Uuid, regle: &str, agent: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM acc_alerte WHERE etablissement_id = $1 AND regle = $2 AND agent_compte_id = $3",
    )
    .bind(etab)
    .bind(regle)
    .bind(agent)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

async fn nb_notifs(ent: &PgEnt, etab: Uuid, kind: &str, user: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND kind = $2 AND destinataire_user_id = $3",
    )
    .bind(etab)
    .bind(kind)
    .bind(user)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

/// Ouvre un contrôle sur `agent` avec une passkey récente de la direction ; rend le contrôle.
async fn ouvrir_controle(
    ent: &PgEnt,
    etab: Uuid,
    dir: Uuid,
    dev: &mut WebauthnAuthenticator<SoftPasskey>,
    agent: Uuid,
    motif: &str,
    t: DateTime<Utc>,
) -> crate::acces_controle::ControleVue {
    let defi = ent
        .acces_controle_defi_a(
            etab,
            dir,
            DIR,
            agent,
            motif,
            "Vérification demandée par la direction",
            15,
            t,
        )
        .await
        .expect("défi");
    let cred = assertion(dev, &defi.options);
    ent.acces_controle_ouvrir_a(
        etab,
        dir,
        DIR,
        agent,
        motif,
        "Vérification demandée par la direction",
        15,
        defi.defi_id,
        &cred,
        t,
    )
    .await
    .expect("ouverture")
}

/// Contrôle interne : droits (`acces.controle_ouvrir`), passkey récente **liée** au contrôle demandé, durée bornée,
/// un seul contrôle ouvert par agent, clôture une seule fois, aucun contrôle sur soi-même.
#[tokio::test]
async fn g4b_controle_passkey_recente_borne_et_cloture() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, prof) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.prof.user_id,
    );
    let t = le(2027, 1, 15, 10, 0);
    // Le secrétariat n'ouvre pas de contrôle ; la direction sans passkey enregistrée non plus.
    let r = ent
        .acces_controle_defi_a(
            etab,
            cast.administratif.user_id,
            "administratif",
            prof,
            "plainte",
            "Plainte d'une famille",
            10,
            t,
        )
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    let r = ent
        .acces_controle_defi_a(
            etab,
            dir,
            DIR,
            prof,
            "plainte",
            "Plainte d'une famille",
            10,
            t,
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "sans passkey : {r:?}");
    let mut cle = enroler(&ent, etab, dir).await;
    // Durée et motif bornés ; pas de contrôle sur soi-même.
    for (motif, texte, jours) in [
        ("plainte", "Plainte d'une famille", 32),
        ("plainte", "Plainte d'une famille", 0),
        ("inconnu", "Plainte d'une famille", 5),
        ("plainte", "ok", 5),
    ] {
        let r = ent
            .acces_controle_defi_a(etab, dir, DIR, prof, motif, texte, jours, t)
            .await
            .unwrap_err();
        assert!(
            matches!(r, EntDbError::SaisieInvalide(_)),
            "{motif} {jours} : {r:?}"
        );
    }
    let r = ent
        .acces_controle_defi_a(
            etab,
            dir,
            DIR,
            dir,
            "plainte",
            "Plainte d'une famille",
            10,
            t,
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "{r:?}");
    // Un parent n'est pas un agent.
    let r = ent
        .acces_controle_defi_a(
            etab,
            dir,
            DIR,
            cast.parent_lea.user_id,
            "plainte",
            "Plainte d'une famille",
            10,
            t,
        )
        .await
        .unwrap_err();
    assert!(est_introuvable(&r), "{r:?}");

    // Le défi est lié au contrôle demandé : une autre demande avec le même défi est refusée.
    let defi = ent
        .acces_controle_defi_a(
            etab,
            dir,
            DIR,
            prof,
            "plainte",
            "Plainte d'une famille",
            10,
            t,
        )
        .await
        .unwrap();
    let cred = assertion(&mut cle, &defi.options);
    let r = ent
        .acces_controle_ouvrir_a(
            etab,
            dir,
            DIR,
            prof,
            "plainte",
            "Autre motif, autre contrôle",
            10,
            defi.defi_id,
            &cred,
            t,
        )
        .await
        .unwrap_err();
    assert!(est_introuvable(&r), "défi d'un autre contrôle : {r:?}");
    // Une assertion jouée sur un autre défi est refusée, et le défi ne sert qu'une fois.
    let c = ent
        .acces_controle_ouvrir_a(
            etab,
            dir,
            DIR,
            prof,
            "plainte",
            "Plainte d'une famille",
            10,
            defi.defi_id,
            &cred,
            t,
        )
        .await
        .expect("ouverture");
    assert!(c.ouvert);
    assert_eq!(c.echeance_le, t + Duration::days(10));
    let r = ent
        .acces_controle_ouvrir_a(
            etab,
            dir,
            DIR,
            prof,
            "plainte",
            "Plainte d'une famille",
            10,
            defi.defi_id,
            &cred,
            t,
        )
        .await
        .unwrap_err();
    assert!(
        est_conflit(&r) || est_introuvable(&r),
        "défi rejoué : {r:?}"
    );
    // Un seul contrôle ouvert par agent.
    let r = ent
        .acces_controle_defi_a(
            etab,
            dir,
            DIR,
            prof,
            "alerte",
            "Alerte de rafale à examiner",
            5,
            t + Duration::hours(1),
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "{r:?}");
    // Liste : visible de la direction, pas du professeur.
    let l = ent.acces_controles_a(etab, dir, DIR, t).await.unwrap();
    assert!(l.iter().any(|x| x.id == c.id && x.ouvert));
    let r = ent
        .acces_controles_a(etab, prof, PROF, t)
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    // Clôture : une issue connue, une seule fois ; échu = plus ouvert.
    let r = ent
        .acces_controle_clore_a(etab, dir, DIR, c.id, "peut-etre", t)
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::SaisieInvalide(_)), "{r:?}");
    let clos = ent
        .acces_controle_clore_a(etab, dir, DIR, c.id, "rien", t + Duration::days(2))
        .await
        .unwrap();
    assert!(!clos.ouvert);
    assert_eq!(clos.issue.as_deref(), Some("rien"));
    let r = ent
        .acces_controle_clore_a(etab, dir, DIR, c.id, "violation", t + Duration::days(3))
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "{r:?}");
    // Le contrôle clos ne donne plus accès aux traces de l'agent.
    let r = ent
        .acces_traces_controle_a(etab, dir, DIR, c.id, t + Duration::days(3))
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "{r:?}");
}

/// Traces d'un agent : sans contrôle ouvert → refus ; avec contrôle ouvert → lignes datées (fonction, service, élève
/// consulté), jamais de total ni de rang ; la lecture est tracée (non affichable aux familles).
#[tokio::test]
async fn g4b_traces_d_un_agent_seulement_sous_controle_ouvert() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, prof) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.prof.user_id,
    );
    let t = le(2027, 1, 15, 10, 0);
    inserer_trace(
        &ent,
        etab,
        le(2027, 1, 10, 9, 0),
        prof,
        PROF,
        Some(cast.eleve_lea_id),
        "dossier",
        false,
        false,
        true,
    )
    .await;
    inserer_trace(
        &ent,
        etab,
        le(2027, 1, 11, 9, 0),
        prof,
        PROF,
        Some(cast.eleve_tom_id),
        "sante",
        true,
        false,
        false,
    )
    .await;
    // Sans contrôle : le contrôle n'existe pas → introuvable ; le professeur n'a pas le droit.
    let r = ent
        .acces_traces_controle_a(etab, dir, DIR, Uuid::new_v4(), t)
        .await
        .unwrap_err();
    assert!(est_introuvable(&r), "{r:?}");
    let mut cle = enroler(&ent, etab, dir).await;
    let c = ouvrir_controle(
        &ent,
        etab,
        dir,
        &mut cle,
        prof,
        "verification_periodique",
        t,
    )
    .await;
    let r = ent
        .acces_traces_controle_a(etab, prof, PROF, c.id, t)
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    let (vue, lignes) = ent
        .acces_traces_controle_a(etab, dir, DIR, c.id, t + Duration::hours(1))
        .await
        .unwrap();
    assert!(vue.ouvert);
    assert_eq!(
        lignes.len(),
        2,
        "toutes les consultations de l'agent, santé comprise (libellé générique)"
    );
    assert!(lignes
        .iter()
        .all(|l| !l.fonction.is_empty() && !l.service.is_empty()));
    assert!(lignes.iter().any(|l| l.hors_perimetre && l.motif.is_some()));
    // La lecture est tracée, et jamais montrée à la famille.
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM acc_trace WHERE etablissement_id = $1 AND acteur_compte_id = $2
            AND action = 'acces_traces_lecture' AND NOT affichable",
    )
    .bind(etab)
    .bind(dir)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1);
    let page = ent
        .acces_consultations_a(
            etab,
            cast.parent_lea.user_id,
            "parent",
            t + Duration::hours(2),
        )
        .await
        .unwrap();
    let vues: usize = page
        .dossiers
        .iter()
        .map(|d| d.jours.iter().map(|j| j.lignes.len()).sum::<usize>())
        .sum();
    assert_eq!(
        vues, 1,
        "la famille ne voit que la consultation de l'agent, pas la lecture des traces"
    );
}

/// Traces des sujets d'une demande ouverte : jamais santé ni dossier de droits, jamais le nom de l'agent ; sans droit → 403 ;
/// la lecture est tracée. Le nom n'est donné que sur arbitrage motivé, pour **cette** demande.
#[tokio::test]
async fn g4b_traces_de_la_demande_nom_d_agent_sur_arbitrage() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, prof, parent) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.prof.user_id,
        cast.parent_lea.user_id,
    );
    let t = le(2027, 1, 15, 10, 0);
    let d = demande(&ent, &cast, "titulaire_autorite", Some(parent)).await;
    let a = inserer_trace(
        &ent,
        etab,
        le(2027, 1, 10, 9, 0),
        prof,
        PROF,
        Some(cast.eleve_lea_id),
        "dossier",
        false,
        false,
        true,
    )
    .await;
    let sante = inserer_trace(
        &ent,
        etab,
        le(2027, 1, 10, 10, 0),
        prof,
        PROF,
        Some(cast.eleve_lea_id),
        "sante",
        false,
        false,
        false,
    )
    .await;
    let droits = inserer_trace(
        &ent,
        etab,
        le(2027, 1, 10, 11, 0),
        prof,
        PROF,
        Some(cast.eleve_lea_id),
        "dossier_droits",
        false,
        false,
        false,
    )
    .await;
    let r = ent
        .acces_traces_demande_a(etab, prof, PROF, d.id, t)
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    let r = ent
        .acces_traces_demande_a(etab, dir, DIR, Uuid::new_v4(), t)
        .await
        .unwrap_err();
    assert!(est_introuvable(&r), "{r:?}");
    let v = ent
        .acces_traces_demande_a(etab, dir, DIR, d.id, t)
        .await
        .unwrap();
    let lignes: Vec<_> = v.iter().flat_map(|s| s.lignes.iter()).collect();
    assert!(lignes.iter().any(|l| l.id == a));
    assert!(
        lignes.iter().all(|l| l.id != sante && l.id != droits),
        "ni santé ni dossier de droits"
    );
    assert!(
        lignes
            .iter()
            .all(|l| l.agent_nom.is_none() && !l.nom_communique),
        "aucun nom sans arbitrage"
    );
    // Sans le droit d'arbitrer (secrétariat) : refus ; avec : motivé obligatoire.
    let r = ent
        .acces_nom_agent_arbitrer_a(
            etab,
            cast.administratif.user_id,
            "administratif",
            d.id,
            a,
            "Demande de la famille justifiée",
            t,
        )
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    let r = ent
        .acces_nom_agent_arbitrer_a(etab, dir, DIR, d.id, a, "x", t)
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::SaisieInvalide(_)), "{r:?}");
    // Sans nom : l'extrait ne porte ni nom d'agent ni colonne « agent » ; après arbitrage, le nom y figure.
    let ex = ent
        .droits_extrait_assembler_a(etab, dir, DIR, d.id, None, t)
        .await
        .unwrap();
    let contenu = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, ex[0].id)
        .await
        .unwrap();
    let cons = contenu
        .sections
        .iter()
        .flat_map(|s| s.tables.iter())
        .find(|tb| tb.nom == "acc_trace")
        .expect("section consultations");
    assert!(!cons.lignes.is_empty());
    assert!(cons
        .lignes
        .iter()
        .all(|l| l.valeurs.iter().all(|(c, _)| c != "agent")));
    assert!(
        cons.lignes.iter().all(|l| l
            .valeurs
            .iter()
            .any(|(c, v)| c == "fonction" && v == "Enseignant")),
        "fonction et service à la place du nom"
    );
    // Aucune ligne de santé ni de dossier de droits dans l'extrait.
    assert!(cons.lignes.iter().all(|l| l
        .valeurs
        .iter()
        .all(|(c, v)| !(c == "section" && v.contains("anté")))));
    ent.acces_nom_agent_arbitrer_a(
        etab,
        dir,
        DIR,
        d.id,
        a,
        "Demande de la famille justifiée par écrit",
        t,
    )
    .await
    .unwrap();
    let v = ent
        .acces_traces_demande_a(etab, dir, DIR, d.id, t)
        .await
        .unwrap();
    let l = v
        .iter()
        .flat_map(|s| s.lignes.iter())
        .find(|l| l.id == a)
        .unwrap();
    assert!(l.nom_communique && l.agent_nom.as_deref().is_some_and(|n| !n.is_empty()));
    // Une nouvelle assemblée (autre version) porte maintenant le nom.
    let ex2 = ent
        .droits_extrait_assembler_a(etab, dir, DIR, d.id, None, t)
        .await
        .unwrap();
    let c2 = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, ex2[0].id)
        .await
        .unwrap();
    let cons2 = c2
        .sections
        .iter()
        .flat_map(|s| s.tables.iter())
        .find(|tb| tb.nom == "acc_trace")
        .unwrap();
    assert!(cons2
        .lignes
        .iter()
        .any(|l| l.valeurs.iter().any(|(c, _)| c == "agent")));
    // La lecture des traces est tracée par la direction.
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM acc_trace WHERE etablissement_id = $1 AND acteur_compte_id = $2 AND action = 'acces_traces_lecture'",
    )
    .bind(etab)
    .bind(dir)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(n >= 2);
}

/// Extrait : une consultation protégée (R-RG46) ou différée (R-RG28) n'y figure pas ; le contenu relu est identique à
/// celui assemblé (les consultations qui s'ajoutent ensuite ne périment pas l'extrait).
#[tokio::test]
async fn g4b_extrait_sans_protegee_ni_differee_et_stable() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, prof, parent) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.prof.user_id,
        cast.parent_lea.user_id,
    );
    let t = le(2027, 1, 15, 10, 0);
    let d = demande(&ent, &cast, "titulaire_autorite", Some(parent)).await;
    let simple = inserer_trace(
        &ent,
        etab,
        le(2027, 1, 10, 9, 0),
        prof,
        PROF,
        Some(cast.eleve_lea_id),
        "dossier",
        false,
        false,
        true,
    )
    .await;
    let protegee = inserer_trace(
        &ent,
        etab,
        le(2027, 1, 10, 10, 0),
        prof,
        PROF,
        Some(cast.eleve_lea_id),
        "section_protegee",
        false,
        true,
        true,
    )
    .await;
    let differee = inserer_trace(
        &ent,
        etab,
        le(2027, 1, 10, 11, 0),
        prof,
        PROF,
        Some(cast.eleve_lea_id),
        "dossier",
        false,
        false,
        true,
    )
    .await;
    // La consultation différée exige un contrôle ouvert pour signalement sur l'agent, et l'arbitrage de la direction.
    let r = ent
        .acces_differer_a(
            etab,
            dir,
            DIR,
            differee,
            "Enquête en cours sur un signalement",
            30,
            t,
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "sans contrôle pour signalement : {r:?}");
    let mut cle = enroler(&ent, etab, dir).await;
    ouvrir_controle(&ent, etab, dir, &mut cle, prof, "signalement", t).await;
    let r = ent
        .acces_differer_a(
            etab,
            cast.administratif.user_id,
            "administratif",
            differee,
            "Enquête en cours sur un signalement",
            30,
            t,
        )
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    for jours in [0, 91] {
        let r = ent
            .acces_differer_a(
                etab,
                dir,
                DIR,
                differee,
                "Enquête en cours sur un signalement",
                jours,
                t,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(r, EntDbError::SaisieInvalide(_)),
            "{jours} : {r:?}"
        );
    }
    let jusqu = ent
        .acces_differer_a(
            etab,
            dir,
            DIR,
            differee,
            "Enquête en cours sur un signalement",
            30,
            t,
        )
        .await
        .unwrap();
    assert_eq!(jusqu, t + Duration::days(30));
    // La page de la famille ne montre pas la consultation différée (pendant la durée), l'extrait non plus.
    let page = ent
        .acces_consultations_a(etab, parent, "parent", t + Duration::days(1))
        .await
        .unwrap();
    let nb = |p: &crate::acces_traces::ConsultationsDossier| -> usize {
        p.dossiers
            .iter()
            .map(|d| d.jours.iter().map(|j| j.lignes.len()).sum::<usize>())
            .sum()
    };
    assert_eq!(
        nb(&page),
        1,
        "seule la consultation simple (ni protégée ni différée)"
    );
    // Le report est borné : après 30 jours, la consultation redevient visible (jamais effacée).
    let apres = ent
        .acces_consultations_a(etab, parent, "parent", t + Duration::days(31))
        .await
        .unwrap();
    assert_eq!(nb(&apres), 2);
    // Extrait : assemblé à t (différée masquée), relu ensuite à l'identique.
    let ex = ent
        .droits_extrait_assembler_a(etab, dir, DIR, d.id, None, t + Duration::days(1))
        .await
        .unwrap();
    inserer_trace(
        &ent,
        etab,
        le(2027, 1, 20, 9, 0),
        prof,
        PROF,
        Some(cast.eleve_lea_id),
        "dossier",
        false,
        false,
        true,
    )
    .await;
    let contenu = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, ex[0].id)
        .await
        .expect("relu à l'identique");
    let cons = contenu
        .sections
        .iter()
        .flat_map(|s| s.tables.iter())
        .find(|tb| tb.nom == "acc_trace")
        .unwrap();
    assert_eq!(
        cons.lignes.len(),
        1,
        "ni protégée, ni différée, ni postérieure à l'assemblage"
    );
    let _ = (simple, protegee);
}

/// Alertes : une alerte par règle, agent et fenêtre (idempotent), notification à la direction **jamais** à l'agent,
/// rafale, seuil journalier par rôle (le seuil de la vie scolaire est distinct de celui des enseignants), adresse
/// protégée consultée sans lien, élève archivé consulté ; suite tracée.
#[tokio::test]
async fn g4b_alertes_deterministes_notification_et_suite() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, prof, cpe) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.prof.user_id,
        cast.cpe.user_id,
    );
    let maintenant = le(2027, 1, 16, 20, 0);
    // Rafale : 16 consultations hors périmètre en moins de trois minutes (seuil 15 en 10 minutes).
    rafale(&ent, etab, le(2027, 1, 16, 10, 0), prof, PROF, 16, true).await;
    // Seuil journalier : 100 consultations d'un enseignant (seuil 80) ; 100 d'un CPE (seuil 200 : pas d'alerte).
    rafale(&ent, etab, le(2027, 1, 16, 12, 0), prof, PROF, 100, false).await;
    rafale(&ent, etab, le(2027, 1, 16, 14, 0), cpe, "cpe", 100, false).await;
    // Adresse protégée consultée hors périmètre.
    inserer_trace(
        &ent,
        etab,
        le(2027, 1, 16, 15, 0),
        cpe,
        "cpe",
        Some(cast.eleve_lea_id),
        "coordonnees_protegees",
        true,
        true,
        true,
    )
    .await;
    // Élève archivé consulté.
    sqlx::query("UPDATE ent_eleve SET statut = 'archive' WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(cast.eleve_tom_id)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    inserer_trace(
        &ent,
        etab,
        le(2027, 1, 16, 16, 0),
        prof,
        PROF,
        Some(cast.eleve_tom_id),
        "dossier",
        false,
        false,
        true,
    )
    .await;

    let n = ent.acces_alertes_evaluer_a(etab, maintenant).await.unwrap();
    assert!(
        n >= 4,
        "rafale, seuil, adresse protégée, conservation : {n}"
    );
    assert_eq!(
        nb_alertes(&ent, etab, "rafale_hors_perimetre", prof).await,
        1
    );
    assert_eq!(nb_alertes(&ent, etab, "seuil_jour", prof).await, 1);
    assert_eq!(
        nb_alertes(&ent, etab, "seuil_jour", cpe).await,
        0,
        "seuil de la vie scolaire distinct (200)"
    );
    assert_eq!(nb_alertes(&ent, etab, "adresse_protegee", cpe).await, 1);
    assert_eq!(nb_alertes(&ent, etab, "conservation", prof).await, 1);
    // Idempotence : une seconde passe ne crée rien et ne renotifie pas.
    let avant = nb_notifs(&ent, etab, "acces.alerte", dir).await;
    assert!(avant >= 4);
    assert_eq!(
        ent.acces_alertes_evaluer_a(etab, maintenant + Duration::hours(1))
            .await
            .unwrap(),
        0
    );
    assert_eq!(nb_notifs(&ent, etab, "acces.alerte", dir).await, avant);
    // Jamais à l'agent concerné.
    assert_eq!(nb_notifs(&ent, etab, "acces.alerte", prof).await, 0);
    assert_eq!(nb_notifs(&ent, etab, "acces.alerte", cpe).await, 0);
    // Aucun nom d'agent ni d'élève dans l'avis.
    let corps: Vec<String> = sqlx::query_scalar(
        "SELECT corps FROM ent_notification WHERE etablissement_id = $1 AND kind = 'acces.alerte' AND destinataire_user_id = $2",
    )
    .bind(etab)
    .bind(dir)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(corps
        .iter()
        .all(|c| !c.contains("Potter") && !c.contains("Malefoy") && !c.contains("Rogue")));

    // Liste et suite : réservées à `acces.consulter_traces`.
    let r = ent
        .acces_alertes_lister(etab, prof, PROF)
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    let l = ent.acces_alertes_lister(etab, dir, DIR).await.unwrap();
    let rafale_id = l
        .iter()
        .find(|a| a.regle == "rafale_hors_perimetre" && a.agent_compte_id == prof)
        .unwrap()
        .id;
    assert!(l.iter().all(|a| !a.libelle.is_empty()));
    // « Contrôle » exige un contrôle ouvert sur l'agent.
    let r = ent
        .acces_alerte_suite_a(etab, dir, DIR, rafale_id, "controle", maintenant)
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "{r:?}");
    let r = ent
        .acces_alerte_suite_a(etab, dir, DIR, rafale_id, "autre", maintenant)
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::SaisieInvalide(_)), "{r:?}");
    let mut cle = enroler(&ent, etab, dir).await;
    let c = ouvrir_controle(&ent, etab, dir, &mut cle, prof, "alerte", maintenant).await;
    ent.acces_alerte_suite_a(
        etab,
        dir,
        DIR,
        rafale_id,
        "controle",
        maintenant + Duration::minutes(5),
    )
    .await
    .unwrap();
    let apres = ent.acces_alertes_lister(etab, dir, DIR).await.unwrap();
    let a = apres.iter().find(|a| a.id == rafale_id).unwrap();
    assert_eq!(a.suite, "controle");
    assert_eq!(a.controle_id, Some(c.id));
    assert_eq!(a.suite_le, Some(maintenant + Duration::minutes(5)));
    let r = ent
        .acces_alerte_suite_a(etab, dir, DIR, rafale_id, "rien", maintenant)
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "déjà traitée : {r:?}");
}

/// Le DPO (rôle D-017 porteur de `acces.consulter_traces`) reçoit l'alerte comme la direction, et seul ce droit ouvre les
/// traces : l'agent concerné n'est jamais notifié.
#[tokio::test]
async fn g4b_alerte_notifie_le_dpo_par_role_d017() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let (prof, dpo) = (cast.prof.user_id, cast.compta.user_id);
    let role = ent
        .creer_role_for_actor(
            etab,
            cast.admin.user_id,
            "admin",
            RoleSaisie {
                nom: format!("DPO {}", Uuid::new_v4()),
                description: Some("délégué à la protection des données".into()),
                modele: None,
                droits: vec![Droit {
                    action: Action::AccesConsulterTraces,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .expect("rôle DPO")
        .id;
    ent.attribuer_role_for_actor(etab, cast.admin.user_id, "admin", role, dpo, None)
        .await
        .expect("attribution");
    rafale(&ent, etab, le(2027, 1, 16, 10, 0), prof, PROF, 16, true).await;
    ent.acces_alertes_evaluer_a(etab, le(2027, 1, 16, 20, 0))
        .await
        .unwrap();
    assert!(
        nb_notifs(&ent, etab, "acces.alerte", dpo).await >= 1,
        "le DPO est alerté"
    );
    assert!(nb_notifs(&ent, etab, "acces.alerte", cast.direction.user_id).await >= 1);
    assert_eq!(nb_notifs(&ent, etab, "acces.alerte", prof).await, 0);
    // Il lit les alertes, mais n'ouvre pas de contrôle (droit réservé à la direction).
    let l = ent
        .acces_alertes_lister(etab, dpo, cast.compta.role.as_str())
        .await
        .unwrap();
    assert!(!l.is_empty());
    let r = ent
        .acces_controle_defi_a(
            etab,
            dpo,
            cast.compta.role.as_str(),
            prof,
            "alerte",
            "Alerte de rafale à examiner",
            5,
            le(2027, 1, 16, 21, 0),
        )
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
}

/// Réglages : bornes, plancher de 12 mois, journal, valeurs par défaut, droit `conformite.gerer`.
#[tokio::test]
async fn g4b_reglages_bornes_journal_et_defauts() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, prof) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.prof.user_id,
    );
    let l = ent.acces_reglages_lire(etab, dir, DIR).await.unwrap();
    assert!(
        l.iter().all(|x| x.valeur == x.defaut),
        "valeurs par défaut sans ligne"
    );
    let par = |code: &str| l.iter().find(|x| x.code == code).unwrap().clone();
    assert_eq!(par("acces_trace_mois").defaut, 12);
    assert_eq!(par("acces_seuil_enseignant").defaut, 80);
    assert_eq!(par("acces_seuil_vie_scolaire").defaut, 200);
    let r = ent.acces_reglages_lire(etab, prof, PROF).await.unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    // Plancher : jamais moins de 12 mois (S-541) ; réglage inconnu refusé.
    for (code, v) in [
        ("acces_trace_mois", 11),
        ("acces_trace_mois", 61),
        ("acces_rafale_seuil", 2),
        ("inconnu", 5),
    ] {
        let r = ent
            .acces_reglage_modifier(etab, dir, DIR, code, v)
            .await
            .unwrap_err();
        assert!(
            matches!(r, EntDbError::SaisieInvalide(_)),
            "{code} {v} : {r:?}"
        );
    }
    let r = ent
        .acces_reglage_modifier(
            etab,
            cast.administratif.user_id,
            "administratif",
            "acces_trace_mois",
            24,
        )
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    let l = ent
        .acces_reglage_modifier(etab, dir, DIR, "acces_seuil_enseignant", 120)
        .await
        .unwrap();
    assert_eq!(
        l.iter()
            .find(|x| x.code == "acces_seuil_enseignant")
            .unwrap()
            .valeur,
        120
    );
    let h = ent.acces_reglages_historique(etab, dir, DIR).await.unwrap();
    assert!(h.iter().any(|c| c.reglage == "acces_seuil_enseignant"
        && c.ancienne.as_deref() == Some("80")
        && c.nouvelle == "120"));
    // Le seuil modifié s'applique : 100 consultations d'enseignant ne déclenchent plus d'alerte (seuil 120).
    rafale(&ent, etab, le(2027, 1, 16, 12, 0), prof, PROF, 100, false).await;
    ent.acces_alertes_evaluer_a(etab, le(2027, 1, 16, 20, 0))
        .await
        .unwrap();
    assert_eq!(nb_alertes(&ent, etab, "seuil_jour", prof).await, 0);
}

/// Purge par **partition** : le plancher de 12 mois tient en base, une partition d'un sujet visé par une demande ouverte est
/// différée (registre, alerte à la direction), une partition sans sujet suivi est supprimée entière et journalisée.
#[tokio::test]
async fn g4b_purge_par_partition_et_gel_par_demande_ouverte() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, prof, parent) = (
        cast.etablissement_id,
        cast.prof.user_id,
        cast.parent_lea.user_id,
    );
    let m_libre = NaiveDate::from_ymd_opt(2018, 4, 1).unwrap();
    let m_gele = NaiveDate::from_ymd_opt(2018, 3, 1).unwrap();
    let maintenant = le(2030, 1, 10, 9, 0);
    // Plancher en base : une partition de moins de 12 mois ne se purge pas, quelle que soit la demande.
    let r = sqlx::query("SELECT public.acc_trace_purger_partition($1::date, $2, 0, 'test')")
        .bind(NaiveDate::from_ymd_opt(2029, 6, 1).unwrap())
        .bind(maintenant)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "plancher de 12 mois");
    // Mars 2018 : consultation de Léa, sujet d'une demande ouverte → gelée. Avril 2018 : sujet sans demande → purgée.
    let d = demande(&ent, &cast, "titulaire_autorite", Some(parent)).await;
    inserer_trace(
        &ent,
        etab,
        le(2018, 3, 5, 9, 0),
        prof,
        PROF,
        Some(cast.eleve_lea_id),
        "dossier",
        false,
        false,
        true,
    )
    .await;
    for i in 0..3 {
        inserer_trace(
            &ent,
            etab,
            le(2018, 4, 5 + i, 9, 0),
            prof,
            PROF,
            Some(Uuid::new_v4()),
            "dossier",
            false,
            false,
            true,
        )
        .await;
    }
    let r = ent
        .acces_purger_filtre_a(maintenant, Some(&[m_libre, m_gele]))
        .await
        .unwrap();
    assert_eq!(r.purgees, vec![m_libre]);
    assert_eq!(r.differees, vec![m_gele]);
    let existe = |nom: &'static str| {
        let p = ent.pool_verif().clone();
        async move {
            sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass($1)::text")
                .bind(format!("public.{nom}"))
                .fetch_one(&p)
                .await
                .unwrap()
                .is_some()
        }
    };
    assert!(
        !existe("acc_trace_201804").await,
        "partition supprimée entière"
    );
    assert!(
        existe("acc_trace_201803").await,
        "partition gelée conservée"
    );
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM acc_trace WHERE etablissement_id = $1 AND le < '2018-05-01'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1, "seule la trace de la partition gelée reste");
    // Registre : une ligne « purgée » (3 lignes) et une « différée ».
    let reg: Vec<(NaiveDate, String, i64)> = sqlx::query_as(
        "SELECT partition_mois, issue, lignes FROM acc_purge WHERE partition_mois IN ($1, $2) ORDER BY partition_mois",
    )
    .bind(m_gele)
    .bind(m_libre)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(reg
        .iter()
        .any(|(m, i, n)| *m == m_libre && i == "purgee" && *n >= 3));
    assert!(reg.iter().any(|(m, i, _)| *m == m_gele && i == "differee"));
    // La direction est alertée du report ; une seconde passe, dans la semaine, n'écrit pas de nouvelle ligne ni d'avis.
    assert!(nb_notifs(&ent, etab, "acces.purge_differee", cast.direction.user_id).await >= 1);
    let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM acc_purge WHERE partition_mois = $1")
        .bind(m_gele)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let r2 = ent
        .acces_purger_filtre_a(maintenant + Duration::days(1), Some(&[m_gele]))
        .await
        .unwrap();
    assert_eq!(r2.differees, vec![m_gele]);
    let apres: i64 = sqlx::query_scalar("SELECT count(*) FROM acc_purge WHERE partition_mois = $1")
        .bind(m_gele)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(apres, avant);
    let _ = d;
}

/// Le cloisonnement vaut aussi pour les contrôles et les alertes : un autre établissement ne les voit pas.
#[tokio::test]
async fn g4b_cloisonnement_des_controles_et_alertes() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let Some((_, autre)) = labo().await else {
        return;
    };
    if autre.etablissement_id == cast.etablissement_id {
        return;
    }
    let (etab, dir, prof) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.prof.user_id,
    );
    rafale(&ent, etab, le(2027, 1, 16, 10, 0), prof, PROF, 16, true).await;
    ent.acces_alertes_evaluer_a(etab, le(2027, 1, 16, 20, 0))
        .await
        .unwrap();
    let mes = ent.acces_alertes_lister(etab, dir, DIR).await.unwrap();
    assert!(!mes.is_empty());
    let leur = ent
        .acces_alertes_lister(autre.etablissement_id, autre.direction.user_id, DIR)
        .await
        .unwrap();
    assert!(leur.iter().all(|a| mes.iter().all(|m| m.id != a.id)));
}
