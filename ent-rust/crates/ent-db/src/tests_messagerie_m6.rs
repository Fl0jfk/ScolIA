//! Messagerie, lot M6 (`docs/socles/16_MESSAGERIE.md` §13, §23 ; R-M15, R-M16, R-M19, R-M22 ; CM-46, CM-61, CM-62,
//! Q-M8) : lecture et avis, contre la base locale de test (feature `labo`). Même discipline que M0a à M5 : un **témoin**
//! précède chaque attaque, chaque scénario sème son propre établissement.
//!
//! | Exigence | Test(s) |
//! |---|---|
//! | R-M19 : « lu » seulement par un destinataire ; premier membre d'une boîte ; hors participation : rien | `m6_r_m19_premier_membre_boite_et_hors_participation` |
//! | CM-62 / Q-M8 : « lu par l'établissement » / « lu par Prénom X. », sans heure ; R-M22 | `m6_cm62_famille_voit_lu_sans_heure_ni_nom_de_membre`, `m6_r_m19_premier_membre_boite_et_hors_participation` |
//! | CM-61 : « non lu après 3 jours » côté personnel | `m6_cm61_non_lu_apres_trois_jours` |
//! | « lu jusqu'à » ne recule jamais ; message inconnu refusé | `m6_lu_jusqua_ne_recule_pas` |
//! | prise en charge visible : neutre pour la famille, nominative pour les membres | `m6_prise_en_charge_visible_et_reponse_attendue_neutre` |
//! | disponibilité d'une boîte (CM-46) | `m6_disponibilite_horaires_de_la_boite`, `disponibilite_a` (tests unitaires) |
//! | natures du §23 déclarées au socle 8, enveloppes neutres, outbox sans contenu | `m6_natures_declarees_et_aucun_contenu_dans_l_outbox` |
//! | séparation entre établissements | `m6_isolation_entre_etablissements` |

use super::tests_messagerie_m1::{ecrit, labo, lit, role};
use super::tests_messagerie_m3::{boite, ouvrir_boite, personne_de, rep};
use super::*;
use crate::msg_m3::HoraireSaisie;
use crate::ntf::assemblage::verifier_assemblage;
use crate::ntf::catalogue::{AVIS, NATURES};
use chrono::{Duration, NaiveTime, TimeZone, Utc};
use ent_auth::Role;
use uuid::Uuid;

fn est_introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(domain_eleve::DomainError::MsgFilIntrouvable | domain_eleve::DomainError::NotFound))
}

async fn pointeurs(ent: &PgEnt, fil: Uuid) -> Vec<(Uuid, Option<Uuid>)> {
    sqlx::query_as(
        "SELECT lu_jusqua_message_id, lu_par_personne_id FROM ent_msg_lecture WHERE fil_id = $1 ORDER BY premiere_lecture_le",
    )
    .bind(fil)
    .fetch_all(ent.pool())
    .await
    .unwrap()
}

async fn cree_le(ent: &PgEnt, message: Uuid) -> chrono::DateTime<Utc> {
    sqlx::query_scalar("SELECT created_at FROM ent_msg WHERE id = $1")
        .bind(message)
        .fetch_one(ent.pool())
        .await
        .unwrap()
}

async fn dernier_message(ent: &PgEnt, etab: Uuid, fil: Uuid, user: Uuid) -> domain_eleve::Message {
    ent.messages
        .list_messages(etab, fil, user)
        .await
        .unwrap()
        .into_iter()
        .max_by_key(|m| (m.created_at, m.id))
        .expect("un message")
}

// ───────────────────────── R-M19 : lu par le destinataire ─────────────────────────

#[tokio::test]
async fn m6_r_m19_premier_membre_boite_et_hors_participation() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, adm, prof) = (cast.direction.user_id, cast.cpe.user_id, cast.administratif.user_id, cast.prof.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe, adm]).await;
    let (_autre, _) = boite(&ent, etab, dir, "secretariat", &[prof]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "Entretien").await;
    let t = cree_le(&ent, o.message_id).await + Duration::hours(1);
    let vue_famille = |t| ent.lecture_du_fil_a(etab, lily, role(Role::Parent), o.fil_id, t);

    // Avant toute lecture : « non lu » pour la famille.
    let v = vue_famille(t).await.expect("témoin : la famille voit l'état de son fil");
    assert_eq!(v.cote_lecteur, "famille");
    assert_eq!(v.messages.len(), 1);
    assert!(!v.messages[0].lu);
    assert!(pointeurs(&ent, o.fil_id).await.is_empty(), "aucun pointeur avant lecture");

    // Attaques : un membre d'une AUTRE boîte, et la direction (non membre : ni pont réservé, ni supervision en M6),
    // n'ont pas même accès (404 uniforme) et ne posent rien.
    for (u, r) in [(prof, Role::Prof), (dir, Role::Direction)] {
        let e = ent.lecture_du_fil_a(etab, u, role(r), o.fil_id, t).await.expect_err("hors boîte");
        assert!(est_introuvable(&e), "{r:?} : {e:?}");
        let e = ent.marquer_lu_for_actor(etab, u, role(r), o.fil_id, None).await.expect_err("hors boîte");
        assert!(est_introuvable(&e), "{r:?} : {e:?}");
        assert!(!lit(&ent, etab, u, r, o.fil_id).await);
    }
    assert!(vue_famille(t).await.unwrap().messages.iter().all(|m| !m.lu), "personne n'a posé d'accusé");

    // Le premier membre qui ouvre pose la lecture **de la boîte** (§13).
    assert!(lit(&ent, etab, cpe, Role::Cpe, o.fil_id).await, "témoin : un membre lit");
    let p = pointeurs(&ent, o.fil_id).await;
    assert_eq!(p.len(), 1, "une ligne : celle de la boîte");
    let cpe_personne = personne_de(&ent, etab, cpe).await;
    assert_eq!(p[0].1, Some(cpe_personne), "le premier membre est gardé pour le personnel de la boîte");
    // La famille : « lu par l'établissement », jamais le membre, jamais l'heure (R-M22, §21.3 point 4).
    let v = vue_famille(t).await.unwrap();
    assert!(v.messages[0].lu);
    assert_eq!(v.messages[0].lu_par.as_deref(), Some("l'établissement"));
    let json = serde_json::to_string(&v).unwrap();
    for interdit in ["lu_depuis", "lu_le", "premiere_lecture", "pris_par"] {
        assert!(!json.contains(interdit), "la famille ne voit pas « {interdit} » : {json}");
    }
    // Un second membre qui ouvre ensuite ne change ni la ligne ni le « premier membre ».
    assert!(lit(&ent, etab, adm, Role::Administratif, o.fil_id).await);
    assert_eq!(pointeurs(&ent, o.fil_id).await, p);
}

#[tokio::test]
async fn m6_cm62_famille_voit_lu_sans_heure_ni_nom_de_membre() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, lily) = (cast.direction.user_id, cast.parent_lea.user_id);
    let fils = super::tests_messagerie_m1::ouvrir(&ent, etab, Role::Direction, dir, cast.eleve_lea_id, "Rendez-vous").await;
    let fil = fils[0].fil.id;
    // La famille répond : son message attend d'être lu par l'établissement.
    assert!(ecrit(&ent, etab, lily, Role::Parent, fil, "Bien reçu, merci.").await, "témoin : la famille répond");
    let t = cree_le(&ent, dernier_message(&ent, etab, fil, lily).await.id).await + Duration::minutes(5);
    let v = ent.lecture_du_fil_a(etab, lily, role(Role::Parent), fil, t).await.unwrap();
    let sien: Vec<_> = v.messages.iter().filter(|m| !m.lu).collect();
    assert_eq!(sien.len(), 1, "sa réponse n'est pas lue : {v:?}");
    // Le personnel du fil lit : la réponse est lue ; la famille voit un nom court ou « l'établissement », jamais l'heure.
    assert!(lit(&ent, etab, dir, Role::Direction, fil).await, "témoin : l'auteur du fil lit la réponse");
    let v = ent.lecture_du_fil_a(etab, lily, role(Role::Parent), fil, t).await.unwrap();
    assert!(v.messages.iter().all(|m| m.lu), "{v:?}");
    let par = v.messages[0].lu_par.clone().expect("lu par");
    assert!(par == "l'établissement" || par.ends_with('.'), "« Prénom X. » ou « l'établissement » : {par}");
    assert!(!serde_json::to_string(&v).unwrap().contains("lu_depuis"));
    // L'auteur ne s'accuse pas lui-même : le message de la direction n'est pas « lu » par la direction (ses messages
    // ne figurent pas dans la vue de la famille, et le pointeur du personnel ne compte que pour les messages d'en face).
    let pers = ent.lecture_du_fil_a(etab, dir, role(Role::Direction), fil, t).await.unwrap();
    let lily_vue = pers.participants.iter().find(|p| p.cote == "famille").expect("Lily côté famille");
    assert!(lily_vue.lu_depuis.is_none(), "Lily n'a rien lu : aucun accusé posé à sa place");
    // Elle lit : le personnel voit jusqu'où et depuis quand (pour lui seul).
    assert!(lit(&ent, etab, lily, Role::Parent, fil).await);
    let pers = ent.lecture_du_fil_a(etab, dir, role(Role::Direction), fil, t).await.unwrap();
    let lily_vue = pers.participants.iter().find(|p| p.cote == "famille").unwrap();
    assert!(lily_vue.lu_depuis.is_some() && lily_vue.lu_jusqua_message_id.is_some());
}

#[tokio::test]
async fn m6_cm61_non_lu_apres_trois_jours() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, lily) = (cast.direction.user_id, cast.parent_lea.user_id);
    let fils = super::tests_messagerie_m1::ouvrir(&ent, etab, Role::Direction, dir, cast.eleve_lea_id, "Convocation").await;
    let fil = fils[0].fil.id;
    let t0 = cree_le(&ent, fils[0].message.id).await;
    async fn non_lu(ent: &PgEnt, etab: Uuid, dir: Uuid, fil: Uuid, t: chrono::DateTime<Utc>) -> bool {
        let v = ent.lecture_du_fil_a(etab, dir, role(Role::Direction), fil, t).await.unwrap();
        v.participants.iter().find(|p| p.cote == "famille").unwrap().non_lu_apres_3_jours
    }
    assert!(!non_lu(&ent, etab, dir, fil, t0 + Duration::days(2)).await, "témoin : deux jours, pas encore");
    assert!(non_lu(&ent, etab, dir, fil, t0 + Duration::days(4)).await, "quatre jours sans lecture : « non lu après 3 jours »");
    // Elle lit : plus d'alerte, même longtemps après.
    assert!(lit(&ent, etab, lily, Role::Parent, fil).await);
    assert!(!non_lu(&ent, etab, dir, fil, t0 + Duration::days(30)).await);
}

#[tokio::test]
async fn m6_lu_jusqua_ne_recule_pas() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, lily) = (cast.direction.user_id, cast.parent_lea.user_id);
    let fils = super::tests_messagerie_m1::ouvrir(&ent, etab, Role::Direction, dir, cast.eleve_lea_id, "Deux messages").await;
    let fil = fils[0].fil.id;
    let m1 = fils[0].message.id;
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Second message").await);
    let m2 = dernier_message(&ent, etab, fil, lily).await.id;
    assert_ne!(m1, m2);
    // Lily marque lu jusqu'au second, puis (client en retard) jusqu'au premier : le pointeur reste au second.
    assert!(ent.marquer_lu_for_actor(etab, lily, role(Role::Parent), fil, Some(m2)).await.unwrap(), "avance");
    assert!(!ent.marquer_lu_for_actor(etab, lily, role(Role::Parent), fil, Some(m1)).await.unwrap(), "ne recule pas");
    assert_eq!(pointeurs(&ent, fil).await[0].0, m2);
    // Un message d'un autre fil est refusé (jamais un pointeur vers un message qu'on ne voit pas).
    let autres = super::tests_messagerie_m1::ouvrir(&ent, etab, Role::Direction, dir, cast.eleve_lea_id, "Autre fil").await;
    let e = ent
        .marquer_lu_for_actor(etab, lily, role(Role::Parent), fil, Some(autres[0].message.id))
        .await
        .expect_err("message inconnu");
    assert!(matches!(e, EntDbError::SaisieInvalide(_)), "{e:?}");
    // Un tiers (parent d'un autre élève) : 404 uniforme.
    let e = ent
        .marquer_lu_for_actor(etab, cast.parent_tom.user_id, role(Role::Parent), fil, None)
        .await
        .expect_err("tiers");
    assert!(est_introuvable(&e), "{e:?}");
}

// ───────────────────────── Prise en charge, disponibilité, avis ─────────────────────────

#[tokio::test]
async fn m6_prise_en_charge_visible_et_reponse_attendue_neutre() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, adm) = (cast.direction.user_id, cast.cpe.user_id, cast.administratif.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe, adm]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "Sujet-secret-M6").await;
    let t = cree_le(&ent, o.message_id).await + Duration::hours(1);
    let famille = || ent.lecture_du_fil_a(etab, lily, role(Role::Parent), o.fil_id, t);
    let membre = || ent.lecture_du_fil_a(etab, adm, role(Role::Administratif), o.fil_id, t);

    assert_eq!(famille().await.unwrap().prise_en_charge.unwrap().statut, "nouveau", "témoin : reçu, pas encore pris");
    ent.prendre_en_charge_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, vs).await.unwrap();
    // La famille : « pris en charge », sans nom ; un membre de la boîte voit qui s'en occupe.
    let pf = famille().await.unwrap().prise_en_charge.unwrap();
    assert_eq!((pf.statut, pf.libelle), ("pris", "pris en charge"));
    assert!(pf.pris_par.is_none(), "jamais le nom d'un membre côté famille");
    let pm = membre().await.unwrap().prise_en_charge.unwrap();
    assert!(pm.pris_par.is_some(), "un membre voit qui s'en occupe : {pm:?}");
    assert!(!serde_json::to_string(&famille().await.unwrap()).unwrap().contains("pris_par\":\""));

    // Réponse puis « en attente de votre réponse » : avis `messagerie.reponse_attendue`, enveloppe du gabarit.
    ent.repondre_au_nom_de_boite_for_actor(role(Role::Cpe), rep(etab, cpe, o.fil_id, "Corps-secret-M6 : jeudi 17 h."), vs)
        .await
        .unwrap();
    ent.attendre_famille_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, vs).await.unwrap();
    assert_eq!(famille().await.unwrap().prise_en_charge.unwrap().libelle, "en attente de votre réponse");
    let (titre, corps, eleve): (String, String, Option<Uuid>) = sqlx::query_as(
        "SELECT titre, corps, eleve_id FROM ent_notification
          WHERE destinataire_user_id = $1 AND kind = 'messagerie_famille' AND titre = 'Réponse attendue'",
    )
    .bind(lily)
    .fetch_one(ent.pool())
    .await
    .expect("avis « réponse attendue » à la famille");
    let g = crate::ntf::catalogue::MESSAGERIE_REPONSE_ATTENDUE.gabarit;
    assert_eq!((titre.as_str(), corps.as_str()), (g.titre, g.corps), "le texte est celui du gabarit déclaré");
    assert!(eleve.is_some());
    // Une seconde fois : une relance au plus (dédoublonnage).
    ent.traiter_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, vs).await.unwrap();
    assert!(ecrit(&ent, etab, lily, Role::Parent, o.fil_id, "Merci.").await);
    ent.prendre_en_charge_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, vs).await.ok();
    ent.attendre_famille_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, vs).await.ok();
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND titre = 'Réponse attendue'",
    )
    .bind(lily)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "une relance au plus");
}

#[tokio::test]
async fn m6_disponibilite_horaires_de_la_boite() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, lily, harry) = (cast.direction.user_id, cast.cpe.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "Horaires").await;
    // Sans horaire déclaré : aucune promesse de délai (R-M26).
    let sans = ent.lecture_du_fil_a(etab, lily, role(Role::Parent), o.fil_id, Utc::now()).await.unwrap();
    assert!(sans.disponibilite.is_none());
    let h = |j| HoraireSaisie {
        jour_iso: j,
        debut: NaiveTime::from_hms_opt(7, 30, 0).unwrap(),
        fin: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
    };
    ent.definir_horaires_boite_for_actor(etab, dir, role(Role::Direction), vs, (1..=5).map(h).collect())
        .await
        .unwrap();
    // Un dimanche (7 avril 2030, midi UTC) : fermé quel que soit le fuseau de l'établissement.
    let dimanche = Utc.with_ymd_and_hms(2030, 4, 7, 12, 0, 0).unwrap();
    let d = ent.lecture_du_fil_a(etab, lily, role(Role::Parent), o.fil_id, dimanche).await.unwrap().disponibilite.unwrap();
    assert!(!d.ouverte);
    let libelle = d.libelle.expect("libellé");
    assert!(libelle.contains("lira votre message à partir de lundi à 7 h 30"), "{libelle}");
    // Un mercredi (3 avril 2030, 10 h UTC) : ouvert, rien à annoncer.
    let mercredi = Utc.with_ymd_and_hms(2030, 4, 3, 10, 0, 0).unwrap();
    let d = ent.lecture_du_fil_a(etab, lily, role(Role::Parent), o.fil_id, mercredi).await.unwrap().disponibilite.unwrap();
    assert!(d.ouverte && d.libelle.is_none());
    // Le personnel ne reçoit pas cette mention sur lui-même.
    let v = ent.lecture_du_fil_a(etab, cpe, role(Role::Cpe), o.fil_id, dimanche).await.unwrap();
    assert!(v.disponibilite.is_none());
}

#[tokio::test]
async fn m6_natures_declarees_et_aucun_contenu_dans_l_outbox() {
    // Déclaration au socle 8 : assemblage et lint de minimisation valides, enveloppes neutres (aucune variable).
    let v = verifier_assemblage(NATURES, AVIS);
    assert!(v.is_empty(), "assemblage invalide : {v:#?}");
    for code in [
        "messagerie.nouveau_message",
        "messagerie.nouveau_message_boite",
        "messagerie.reponse_attendue",
        "messagerie.participant_ajoute",
    ] {
        let n = NATURES.iter().find(|n| n.code == code).unwrap_or_else(|| panic!("{code} non déclarée"));
        assert!(n.gabarit.variables.is_empty(), "{code} : aucune variable, donc aucun contenu ni nom (R-M16)");
        for texte in [n.gabarit.titre, n.gabarit.corps, n.gabarit.sms] {
            assert!(!texte.contains('{'), "{code} : {texte}");
        }
    }

    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, lily, harry) = (cast.direction.user_id, cast.cpe.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "Sujet-secret-M6").await;
    ent.prendre_en_charge_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, vs).await.unwrap();
    assert!(lit(&ent, etab, cpe, Role::Cpe, o.fil_id).await);
    ent.repondre_au_nom_de_boite_for_actor(role(Role::Cpe), rep(etab, cpe, o.fil_id, "Corps-secret-M6 : jeudi."), vs)
        .await
        .unwrap();
    ent.attendre_famille_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, vs).await.unwrap();
    assert!(lit(&ent, etab, lily, Role::Parent, o.fil_id).await);
    // Ni l'outbox (événements immuables), ni les notifications (ENT, push, e-mail, SMS) ne portent un contenu, un objet
    // ou un nom (R-M14, R-M16).
    let evenements: Vec<String> = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2",
    )
    .bind(etab)
    .bind(o.fil_id)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(evenements.iter().any(|e| !e.is_empty()), "des événements ont bien été publiés");
    let notifs: Vec<String> = sqlx::query_scalar(
        "SELECT titre || ' ' || corps FROM ent_notification WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(!notifs.is_empty());
    for texte in evenements.iter().chain(notifs.iter()) {
        for interdit in ["secret-M6", "Entretien", "Lily", "jeudi"] {
            assert!(!texte.contains(interdit), "contenu, objet ou nom dans l'outbox ou une notification : {texte}");
        }
    }
    // Les événements M6 sont neutres : le message, jamais qui a lu (R-M22).
    let lus: Vec<String> = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type_name = 'messagerie.lu_jusqua'",
    )
    .bind(etab)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(!lus.is_empty());
    assert!(lus.iter().all(|p| p.contains("message_id") && !p.contains("lecteur") && !p.contains("personne")), "{lus:?}");
}

#[tokio::test]
async fn m6_isolation_entre_etablissements() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, lily, harry) = (cast.direction.user_id, cast.cpe.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "Isolation").await;
    assert!(ent.lecture_du_fil_for_actor(etab, lily, role(Role::Parent), o.fil_id).await.is_ok(), "témoin");
    let autre = ent.seed_labo_cast().await.unwrap();
    // Le même compte, pris par un autre établissement : le fil n'existe pas.
    let e = ent
        .lecture_du_fil_for_actor(autre.etablissement_id, lily, role(Role::Parent), o.fil_id)
        .await
        .expect_err("autre établissement");
    assert!(est_introuvable(&e), "{e:?}");
    let e = ent
        .marquer_lu_for_actor(autre.etablissement_id, autre.direction.user_id, role(Role::Direction), o.fil_id, None)
        .await
        .expect_err("autre établissement");
    assert!(est_introuvable(&e), "{e:?}");
    // Rien n'a été posé par ces tentatives.
    assert!(pointeurs(&ent, o.fil_id).await.is_empty());
}
