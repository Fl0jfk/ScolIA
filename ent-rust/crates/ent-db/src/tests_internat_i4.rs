//! Socle 41 (internat), lot I4 « Vie quotidienne » : règles testées contre la base locale de test (feature `labo`) ;
//! sans `ENT_DATABASE_URL`, elles sont ignorées en local (échec explicite en CI).
//!
//! **Aucun test ne dépend de l'heure réelle** (règle 33) : chaque commande reçoit son instant, les jours sont des dates
//! fixes d'avril 2026 (heure d'été : Paris = UTC + 2). Chaque test crée son propre établissement.
//!
//! | Règle / cas | Test(s) |
//! |---|---|
//! | INT-170 à INT-179 (sorties de groupe, cycle complet) | `groupe_cycle_complet`, `groupe_duplication_*`, `groupe_annulation_*` |
//! | R-IT21 (un refus n'exclut que l'élève) | `groupe_refus_n_exclut_que_l_eleve` |
//! | Refus par défaut | `groupe_refus_par_defaut_*` |
//! | Un acte tombe avec son auteur | `groupe_acte_tombe_avec_son_auteur_*` |
//! | Une interdiction bloque toujours (chaîne de fusions comprise) | `groupe_interdiction_bloque_toujours_*` |
//! | Pas de départ constaté = élève attendu | `groupe_ne_retire_jamais_de_l_attendu` |
//! | R-IT24 (accord hors ligne daté, nommé) | `groupe_accord_hors_ligne_*` |
//! | R-IT25 (accompagnateurs du personnel) | `groupe_accompagnateurs_*` |
//! | R-IT32 (une famille ne voit que son enfant) | `groupe_famille_*` |
//! | DI-06, DI-14 impossibles | `groupe_accord_hors_ligne_*`, `groupe_refus_n_exclut_que_l_eleve` |
//! | R-IT7, R-IT8, R-IT9, R-IT30, R-IT38 (liaison codée) | `liaison_*` |
//! | DI-17 (suppression d'un événement) impossible, R-IT29 | `evenement_*`, `preoccupation_*`, `gardes_sql_*` |
//! | NI-39, NI-41 (études, activités) | `temps_*` |

use super::tests_internat_i2::{alea, banc, classes_des_deux_zones, est_interdit, eleve_fixe, refus_de, Banc, CPE, DIR};
use super::tests_internat_i3::{avril, compte_eleve, desaccord_connu, interdire, parent, paris, tiers, ELEVE, PARENT};
use super::*;
use crate::droits_repo::{Droit, RoleSaisie};
use crate::tests_voyages_v0a::compte;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::{Action, Portee};
use ent_types::internat_vie::*;
use uuid::Uuid;

const PROF: &str = "prof";
const PERSONNEL: &str = "personnel";

/// Mardi 14 avril 2026, 9 h (Paris) : « maintenant » des créations.
fn t0() -> DateTime<Utc> {
    paris(avril(14), 9, 0)
}
/// Jeudi 16 avril, 14 h – 18 h : la sortie.
fn debut() -> DateTime<Utc> {
    paris(avril(16), 14, 0)
}
fn fin() -> DateTime<Utc> {
    paris(avril(16), 18, 0)
}
fn jour_j(mi: i64) -> DateTime<Utc> {
    debut() - Duration::minutes(30) + Duration::minutes(mi)
}
fn h(n: i64) -> DateTime<Utc> {
    t0() + Duration::hours(n)
}

/// Compte du personnel de base (aucun droit propre) auquel on attache un rôle réglable aux droits donnés.
async fn personnel_avec(b: &Banc, actions: &[Action]) -> Uuid {
    personnel_portee(b, actions, Portee::Etablissement, vec![]).await
}

async fn personnel_portee(b: &Banc, actions: &[Action], portee: Portee, cibles: Vec<Uuid>) -> Uuid {
    let u = compte(&b.ent, b.etab, PERSONNEL).await;
    let role = b
        .ent
        .creer_role_for_actor(
            b.etab,
            b.dir,
            DIR,
            RoleSaisie {
                nom: format!("Rôle I4 {}", alea()),
                description: None,
                modele: None,
                droits: actions.iter().map(|a| Droit { action: *a, portee, cibles: cibles.clone() }).collect(),
            },
        )
        .await
        .unwrap();
    b.ent.attribuer_role_for_actor(b.etab, b.dir, DIR, role.id, u, None).await.unwrap();
    u
}

fn saisie_groupe(encadrants: Vec<Uuid>, eleves: Vec<Uuid>) -> SortieGroupeSaisieDto {
    SortieGroupeSaisieDto { titre: "Sortie cinéma".into(), lieu: "Cinéma municipal".into(), debut: debut(), fin: fin(), encadrants, eleves }
}

async fn brouillon(b: &Banc, encadrants: Vec<Uuid>, eleves: Vec<Uuid>) -> SortieGroupeDto {
    b.ent.creer_sortie_groupe_a(b.etab, b.cpe, CPE, &saisie_groupe(encadrants, eleves), t0()).await.unwrap()
}

async fn soumettre(b: &Banc, id: Uuid) -> SortieGroupeDto {
    b.ent.soumettre_sortie_groupe_a(b.etab, b.cpe, CPE, id, h(1)).await.unwrap()
}

async fn repondre(b: &Banc, user: Uuid, id: Uuid, eleve: Uuid, accord: bool) -> Result<SortieGroupeFamilleDto, EntDbError> {
    b.ent.repondre_sortie_groupe_a(b.etab, user, PARENT, id, eleve, &ReponseGroupeDto { accord }, h(2)).await
}

async fn accepter(b: &Banc, id: Uuid) {
    b.ent
        .decider_sortie_groupe_a(b.etab, b.dir, DIR, id, &DecisionGroupeDto { accepte: true, motif_code: None }, h(3))
        .await
        .unwrap();
}

fn etat_de(d: &SortieGroupeDto, eleve: Uuid) -> String {
    d.participants.iter().find(|p| p.eleve_id == eleve).map(|p| p.etat.clone()).unwrap_or_else(|| "absent".into())
}

async fn detail(b: &Banc, id: Uuid) -> SortieGroupeDto {
    b.ent.sortie_groupe_a(b.etab, b.cpe, CPE, id, jour_j(0)).await.unwrap()
}

async fn depart(b: &Banc, id: Uuid, eleves: Vec<Uuid>) -> Vec<ResultatAppelDto> {
    b.ent.depart_groupe_a(b.etab, b.prof, PROF, id, &AppelGroupeDto { eleves }, jour_j(10)).await.unwrap()
}

fn resultat(r: &[ResultatAppelDto], eleve: Uuid) -> String {
    r.iter().find(|x| x.eleve_id == eleve).map(|x| x.resultat.clone()).unwrap_or_else(|| "absent".into())
}

// ───────────────────────── Sorties de groupe ─────────────────────────

#[tokio::test]
async fn groupe_cycle_complet() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let g = brouillon(&b, vec![b.prof], vec![e1, e2]).await;
    assert_eq!(g.statut, "brouillon");
    assert_eq!(etat_de(&g, e1), "invite");
    // Soumise : les familles sont interrogées (jamais un accord par défaut).
    let g = soumettre(&b, g.id).await;
    assert_eq!(g.statut, "soumise");
    assert_eq!(etat_de(&g, e1), "en_attente");
    // Réponses des familles, puis décision de la direction (pas l'auteur).
    assert_eq!(repondre(&b, p1, g.id, e1, true).await.unwrap().etat, "autorise");
    assert_eq!(repondre(&b, p2, g.id, e2, true).await.unwrap().etat, "autorise");
    accepter(&b, g.id).await;
    // Le jour J : l'accompagnateur fait l'appel de départ puis de retour.
    let r = depart(&b, g.id, vec![e1, e2]).await;
    assert_eq!((resultat(&r, e1), resultat(&r, e2)), ("ok".into(), "ok".into()));
    let d = detail(&b, g.id).await;
    assert_eq!(etat_de(&d, e1), "parti");
    assert!(d.participants.iter().all(|p| p.parti_le.is_some()));
    // La clôture exige que tout le monde soit rentré.
    let bilan = ClotureGroupeDto { bilan_code: "sans_incident".into() };
    let clore = |mi| b.ent.clore_sortie_groupe_a(b.etab, b.prof, PROF, g.id, &bilan, jour_j(mi));
    assert!(refus_de(&clore(240).await, "pas rentrés"));
    let r = b.ent.retour_groupe_a(b.etab, b.prof, PROF, g.id, &AppelGroupeDto { eleves: vec![e1, e2] }, jour_j(250)).await.unwrap();
    assert_eq!((resultat(&r, e1), resultat(&r, e2)), ("ok".into(), "ok".into()));
    let clos = clore(260).await.unwrap();
    assert_eq!(clos.statut, "close");
    assert_eq!(clos.bilan_code.as_deref(), Some("sans_incident"));
    // Plus aucun geste sur une sortie close.
    assert!(b.ent.annuler_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, &AnnulationGroupeDto { motif_code: "meteo".into() }, jour_j(270)).await.is_err());
    // Journal des faits : de la création à la clôture, avec un auteur à chaque ligne.
    let faits: Vec<(String, Option<Uuid>)> = sqlx::query_as(
        "SELECT acte, par_user_id FROM ent_internat_sortie_groupe_fait WHERE etablissement_id = $1 AND sortie_groupe_id = $2 ORDER BY cree_le, id",
    )
    .bind(b.etab)
    .bind(g.id)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap();
    let actes: Vec<&str> = faits.iter().map(|f| f.0.as_str()).collect();
    for attendu in ["creation", "soumission", "accord_famille", "acceptation", "depart", "retour", "cloture"] {
        assert!(actes.contains(&attendu), "fait manquant : {attendu} dans {actes:?}");
    }
    assert!(faits.iter().all(|f| f.1.is_some()), "chaque fait nomme son auteur");
    // L'outbox ne porte ni titre, ni lieu, ni nom.
    for t in ["internat.sortie_groupe_creee", "internat.sortie_groupe_decidee", "internat.sortie_groupe_autorisation", "internat.sortie_groupe_presence", "internat.sortie_groupe_close"] {
        let evts = b.evenements(t).await;
        assert!(!evts.is_empty(), "événement manquant : {t}");
        for e in evts {
            let s = e.to_string();
            assert!(!s.contains("cinéma") && !s.contains("Cinéma") && !s.contains("Nuit"), "texte ou nom dans l'outbox : {s}");
        }
    }
}

#[tokio::test]
async fn groupe_refus_n_exclut_que_l_eleve() {
    // R-IT21 (corrige DI-14) : le refus d'une famille ne retire que son enfant ; la sortie suit pour les autres.
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (b.a[0], b.a[1], b.a[2]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let (p3, _) = parent(&b, e3, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1, e2, e3]).await.id).await;
    repondre(&b, p1, g.id, e1, true).await.unwrap();
    assert_eq!(repondre(&b, p2, g.id, e2, false).await.unwrap().etat, "refuse");
    repondre(&b, p3, g.id, e3, true).await.unwrap();
    accepter(&b, g.id).await;
    let r = depart(&b, g.id, vec![e1, e2, e3]).await;
    assert_eq!(resultat(&r, e1), "ok");
    assert_eq!(resultat(&r, e2), "non_autorise", "le refusé ne part pas");
    assert_eq!(resultat(&r, e3), "ok");
    // Le refus est définitif : un accord ultérieur du même foyer ne le défait pas (le plus restrictif l'emporte).
    assert!(refus_de(&repondre(&b, p2, g.id, e2, true).await, "définitif"));
}

#[tokio::test]
async fn groupe_le_plus_restrictif_l_emporte_entre_titulaires() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (pa, _) = parent(&b, e, true).await;
    let (pb, _) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    assert_eq!(repondre(&b, pa, g.id, e, true).await.unwrap().etat, "autorise");
    // Un second accord n'ajoute rien ; un refus du second titulaire l'emporte sur l'accord du premier.
    assert_eq!(repondre(&b, pb, g.id, e, true).await.unwrap().etat, "autorise");
    assert_eq!(repondre(&b, pb, g.id, e, false).await.unwrap().etat, "refuse");
    accepter(&b, g.id).await;
    assert_eq!(resultat(&depart(&b, g.id, vec![e]).await, e), "non_autorise");
}

#[tokio::test]
async fn groupe_refus_par_defaut_sans_reponse_ni_invitation() {
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (b.a[0], b.a[1], b.a[2]);
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1, e2]).await.id).await;
    let (p1, _) = parent(&b, e1, true).await;
    repondre(&b, p1, g.id, e1, true).await.unwrap();
    accepter(&b, g.id).await;
    let r = depart(&b, g.id, vec![e1, e2, e3]).await;
    assert_eq!(resultat(&r, e1), "ok");
    assert_eq!(resultat(&r, e2), "non_autorise", "sans réponse de la famille, l'élève ne part pas");
    assert_eq!(resultat(&r, e3), "pas_invite", "un élève jamais invité ne part pas");
    // Hors de la plage de départ : refusé (trop tôt, ou après la fin).
    let tot = b.ent.depart_groupe_a(b.etab, b.prof, PROF, g.id, &AppelGroupeDto { eleves: vec![e1] }, debut() - Duration::hours(2)).await;
    assert!(refus_de(&tot, "plage"));
    // Un parent qui n'est pas titulaire de CET élève n'a aucune voix.
    let (autre, _) = parent(&b, b.b[0], true).await;
    assert!(est_interdit(&repondre(&b, autre, g.id, e2, true).await));
    // Un titulaire sans droit d'autoriser les sorties est refusé.
    let (sans_droit, _) = parent(&b, e2, false).await;
    assert!(refus_de(&repondre(&b, sans_droit, g.id, e2, true).await, "ne vous permet pas"));
}

#[tokio::test]
async fn groupe_decision_pas_par_l_auteur_et_motif_de_refus() {
    let Some(b) = banc().await else { return };
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![b.a[0]]).await.id).await;
    let cpe_decide = b.ent.decider_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, &DecisionGroupeDto { accepte: true, motif_code: None }, h(3)).await;
    assert!(refus_de(&cpe_decide, "ne la décide pas"));
    let sans_motif = b.ent.decider_sortie_groupe_a(b.etab, b.dir, DIR, g.id, &DecisionGroupeDto { accepte: false, motif_code: None }, h(3)).await;
    assert!(refus_de(&sans_motif, "motif"));
    // Un accompagnateur sans droit de décision ne décide pas.
    assert!(est_interdit(&b.ent.decider_sortie_groupe_a(b.etab, b.prof, PROF, g.id, &DecisionGroupeDto { accepte: true, motif_code: None }, h(3)).await));
    let refusee = b
        .ent
        .decider_sortie_groupe_a(b.etab, b.dir, DIR, g.id, &DecisionGroupeDto { accepte: false, motif_code: Some("securite".into()) }, h(3))
        .await
        .unwrap();
    assert_eq!((refusee.statut.as_str(), refusee.decision_motif_code.as_deref()), ("refusee", Some("securite")));
    // Refusée : aucun départ, plus aucune réponse, plus de décision.
    assert!(b.ent.depart_groupe_a(b.etab, b.prof, PROF, g.id, &AppelGroupeDto { eleves: vec![b.a[0]] }, jour_j(10)).await.is_err());
    assert!(b.ent.decider_sortie_groupe_a(b.etab, b.dir, DIR, g.id, &DecisionGroupeDto { accepte: true, motif_code: None }, h(4)).await.is_err());
}

#[tokio::test]
async fn groupe_depart_exige_une_sortie_acceptee() {
    let Some(b) = banc().await else { return };
    let (e1, _) = (b.a[0], 0);
    let (p1, _) = parent(&b, e1, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1]).await.id).await;
    repondre(&b, p1, g.id, e1, true).await.unwrap();
    // Soumise mais pas décidée : aucun départ, même avec l'accord de la famille.
    let r = b.ent.depart_groupe_a(b.etab, b.prof, PROF, g.id, &AppelGroupeDto { eleves: vec![e1] }, jour_j(10)).await;
    assert!(refus_de(&r, "acceptée"));
}

#[tokio::test]
async fn groupe_acte_tombe_avec_son_auteur_titulaire_interdit() {
    // Règle générale : l'accord d'un titulaire désormais visé par une interdiction ne fait pas partir l'élève, et la
    // cause n'est ni dite à la famille ni relayée.
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, pers) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    repondre(&b, p, g.id, e, true).await.unwrap();
    accepter(&b, g.id).await;
    interdire(&b, e, Some(pers)).await;
    let r = depart(&b, g.id, vec![e]).await;
    assert_eq!(resultat(&r, e), "accord_tombe");
    let d = detail(&b, g.id).await;
    assert_eq!(etat_de(&d, e), "non_autorise");
    // Aucune cause d'interdiction en clair dans l'outbox côté familles : le code est « interdiction_auteur » (préfixe
    // « interdiction_ » = jamais relayé, contrat des causes du §22).
    let evts = b.evenements("internat.sortie_groupe_autorisation").await;
    assert!(evts.iter().any(|e| e["cause"] == "interdiction_auteur"));
    assert!(evts.iter().all(|e| !e.to_string().contains("remise")), "la nature de la mesure n'est jamais dite");
    // L'élève ne repart pas non plus au second appel.
    assert_eq!(resultat(&depart(&b, g.id, vec![e]).await, e), "non_autorise");
    // Vue famille : un simple « non autorisé », sans cause.
    let (autre, _) = parent(&b, e, true).await;
    let vue = b.ent.sorties_groupe_eleve_a(b.etab, autre, PARENT, e, jour_j(20)).await.unwrap();
    assert_eq!(vue.len(), 1);
    assert_eq!(vue[0].etat, "non_autorise");
}

#[tokio::test]
async fn groupe_interdiction_bloque_toujours_sur_la_chaine_de_fusions() {
    // Une interdiction posée sur la fiche d'une personne fusionnée dans le titulaire vise la même personne réelle.
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, pers) = parent(&b, e, true).await;
    let doublon = tiers(&b, "Doublon").await;
    sqlx::query("UPDATE ent_personne SET fusionnee_dans = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(doublon)
        .bind(pers)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    repondre(&b, p, g.id, e, true).await.unwrap();
    accepter(&b, g.id).await;
    interdire(&b, e, Some(doublon)).await;
    assert_eq!(resultat(&depart(&b, g.id, vec![e]).await, e), "accord_tombe");
}

#[tokio::test]
async fn groupe_desaccord_connu_bloque_sans_exclure_les_autres() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    desaccord_connu(&b, e1, p1).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1, e2]).await.id).await;
    assert_eq!(etat_de(&detail(&b, g.id).await, e1), "non_autorise", "désaccord connu : bloqué dès la soumission");
    // Le titulaire qui répond « accord » n'y change rien, et ne voit pas la cause (R-IT20).
    assert!(repondre(&b, p1, g.id, e1, true).await.is_err());
    assert_eq!(repondre(&b, p2, g.id, e2, true).await.unwrap().etat, "autorise");
    accepter(&b, g.id).await;
    let r = depart(&b, g.id, vec![e1, e2]).await;
    assert_eq!((resultat(&r, e1), resultat(&r, e2)), ("non_autorise".into(), "ok".into()));
    // Un désaccord déclaré APRÈS l'accord fait aussi tomber l'accord au départ.
    let b2 = banc().await.unwrap();
    let e = b2.a[0];
    let (p, _) = parent(&b2, e, true).await;
    let g2 = soumettre(&b2, brouillon(&b2, vec![b2.prof], vec![e]).await.id).await;
    repondre(&b2, p, g2.id, e, true).await.unwrap();
    accepter(&b2, g2.id).await;
    desaccord_connu(&b2, e, p).await;
    assert_eq!(resultat(&depart(&b2, g2.id, vec![e]).await, e), "non_autorise");
}

#[tokio::test]
async fn groupe_eleve_majeur_repond_lui_meme_et_son_refus_est_le_sien() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let u = compte_eleve(&b, e, NaiveDate::from_ymd_opt(2007, 1, 1).unwrap()).await;
    let (p, _) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    // Ses parents ne répondent plus (R-IT22) ; lui, oui.
    assert!(est_interdit(&repondre(&b, p, g.id, e, true).await));
    let r = b.ent.repondre_sortie_groupe_a(b.etab, u, ELEVE, g.id, e, &ReponseGroupeDto { accord: true }, h(2)).await.unwrap();
    assert_eq!(r.etat, "autorise");
    let d = detail(&b, g.id).await;
    assert_eq!(d.participants[0].reponse_qualite.as_deref(), Some("eleve_majeur"));
    // Un mineur ne répond pas à la place de ses parents.
    let b3 = banc().await.unwrap();
    let m = b3.a[0];
    let um = compte_eleve(&b3, m, NaiveDate::from_ymd_opt(2010, 1, 1).unwrap()).await;
    let g3 = soumettre(&b3, brouillon(&b3, vec![b3.prof], vec![m]).await.id).await;
    let r = b3.ent.repondre_sortie_groupe_a(b3.etab, um, ELEVE, g3.id, m, &ReponseGroupeDto { accord: true }, h(2)).await;
    assert!(est_interdit(&r), "un mineur n'autorise pas sa propre sortie");
}

#[tokio::test]
async fn groupe_accord_hors_ligne_nomme_l_auteur_la_date_et_le_canal() {
    // R-IT24 (corrige DI-06) : jamais un « autorisé par l'équipe » anonyme.
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (_, pers) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    let saisie = |personne: Uuid, recu: DateTime<Utc>, canal: &str| AccordHorsLigneDto { donne_par_personne_id: personne, recu_le: recu, canal: canal.into(), piece_id: None };
    let hl = |s: AccordHorsLigneDto, qui: Uuid, role: &'static str| {
        let b = &b;
        async move { b.ent.accord_hors_ligne_groupe_a(b.etab, qui, role, g.id, e, &s, h(2)).await }
    };
    // Canal inconnu, date future ou trop ancienne : refusés.
    assert!(refus_de(&hl(saisie(pers, h(1), "sms"), b.cpe, CPE).await, "canal"));
    assert!(refus_de(&hl(saisie(pers, h(5), "telephone"), b.cpe, CPE).await, "date"));
    assert!(refus_de(&hl(saisie(pers, t0() - Duration::days(120), "telephone"), b.cpe, CPE).await, "date"));
    // Une personne qui n'est pas titulaire (voisine, simple contact) ne donne pas d'accord.
    let voisine = tiers(&b, "Voisine").await;
    assert!(refus_de(&hl(saisie(voisine, h(1), "telephone"), b.cpe, CPE).await, "titulaire"));
    // Sans droit de décision (accompagnateur, professeur) : interdit.
    assert!(est_interdit(&hl(saisie(pers, h(1), "telephone"), b.prof, PROF).await));
    // Valide : titulaire, canal, date, et le compte qui l'a reçu est conservé.
    let d = hl(saisie(pers, h(1), "telephone"), b.cpe, CPE).await.unwrap();
    assert_eq!(etat_de(&d, e), "autorise");
    assert_eq!(d.participants[0].reponse_qualite.as_deref(), Some("hors_ligne"));
    let ligne: (Uuid, Uuid, String) = sqlx::query_as(
        "SELECT donne_par_personne_id, recu_par_user_id, canal FROM ent_internat_sortie_groupe_accord_hors_ligne \
          WHERE etablissement_id = $1 AND sortie_groupe_id = $2 AND eleve_id = $3",
    )
    .bind(b.etab)
    .bind(g.id)
    .bind(e)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(ligne, (pers, b.cpe, "telephone".to_string()));
    // Ajout seul : la preuve ne se modifie ni ne se supprime.
    assert!(sqlx::query("DELETE FROM ent_internat_sortie_groupe_accord_hors_ligne WHERE etablissement_id = $1").bind(b.etab).execute(b.ent.pool_verif()).await.is_err());
    // Et il tombe avec la personne si une interdiction la vise avant le départ.
    accepter(&b, g.id).await;
    interdire(&b, e, Some(pers)).await;
    assert_eq!(resultat(&depart(&b, g.id, vec![e]).await, e), "accord_tombe");
}

#[tokio::test]
async fn groupe_accord_hors_ligne_refuse_une_personne_interdite_et_un_majeur() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (_, pers) = parent(&b, e, true).await;
    interdire(&b, e, Some(pers)).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    let s = AccordHorsLigneDto { donne_par_personne_id: pers, recu_le: h(1), canal: "papier".into(), piece_id: None };
    let r = b.ent.accord_hors_ligne_groupe_a(b.etab, b.cpe, CPE, g.id, e, &s, h(2)).await;
    assert!(refus_de(&r, "ne peut pas être enregistré"), "la cause n'est pas détaillée");
    // Un élève majeur répond lui-même.
    let b2 = banc().await.unwrap();
    let m = b2.a[0];
    compte_eleve(&b2, m, NaiveDate::from_ymd_opt(2007, 1, 1).unwrap()).await;
    let (_, pm) = parent(&b2, m, true).await;
    let g2 = soumettre(&b2, brouillon(&b2, vec![b2.prof], vec![m]).await.id).await;
    let s2 = AccordHorsLigneDto { donne_par_personne_id: pm, recu_le: h(1), canal: "papier".into(), piece_id: None };
    assert!(refus_de(&b2.ent.accord_hors_ligne_groupe_a(b2.etab, b2.cpe, CPE, g2.id, m, &s2, h(2)).await, "majeur"));
}

#[tokio::test]
async fn groupe_accord_annuel_du_socle_18_autorise_sans_reponse() {
    // INT-178 : les internes compatibles (autorisation annuelle, pas d'exclusion) sont autorisés d'office ; les autres
    // attendent leur famille. L'accord annuel tombe aussi au départ si ses décideurs sont devenus interdits.
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, pers1) = parent(&b, e1, true).await;
    let (_p2, _) = parent(&b, e2, true).await;
    super::tests_internat_i3::accorder_annuellement(&b, p1, e1, &["sortie_soiree"]).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1, e2]).await.id).await;
    let d = detail(&b, g.id).await;
    assert_eq!(etat_de(&d, e1), "autorise");
    assert_eq!(d.participants.iter().find(|p| p.eleve_id == e1).unwrap().reponse_qualite.as_deref(), Some("annuelle"));
    assert_eq!(etat_de(&d, e2), "en_attente", "pas d'accord annuel : la famille répond");
    accepter(&b, g.id).await;
    interdire(&b, e1, Some(pers1)).await;
    assert_eq!(resultat(&depart(&b, g.id, vec![e1]).await, e1), "accord_tombe");
}

#[tokio::test]
async fn groupe_accompagnateurs_du_personnel_seulement_et_encadrement_suffisant() {
    let Some(b) = banc().await else { return };
    // R-IT25 : un parent, un élève, un inconnu ne sont pas accompagnateurs.
    let (parent_u, _) = parent(&b, b.a[0], true).await;
    let r = b.ent.creer_sortie_groupe_a(b.etab, b.cpe, CPE, &saisie_groupe(vec![parent_u], vec![b.a[0]]), t0()).await;
    assert!(refus_de(&r, "personnel"));
    assert!(b.ent.creer_sortie_groupe_a(b.etab, b.cpe, CPE, &saisie_groupe(vec![Uuid::new_v4()], vec![]), t0()).await.is_err());
    // Sans accompagnateur : on ne soumet pas.
    let sans = brouillon(&b, vec![], vec![b.a[0]]).await;
    let r = b.ent.soumettre_sortie_groupe_a(b.etab, b.cpe, CPE, sans.id, h(1)).await;
    assert!(refus_de(&r, "encadrement"));
    // Taux d'encadrement réglé à 2 élèves par accompagnateur : trois élèves, un accompagnateur = insuffisant.
    sqlx::query("INSERT INTO ent_internat_parametres_vie (etablissement_id, groupe_eleves_par_encadrant) VALUES ($1, 2) ON CONFLICT (etablissement_id) DO UPDATE SET groupe_eleves_par_encadrant = 2")
        .bind(b.etab)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let g = brouillon(&b, vec![b.prof], vec![b.a[0], b.a[1], b.a[2]]).await;
    assert!(refus_de(&b.ent.soumettre_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, h(1)).await, "encadrement"));
    // Deux accompagnateurs : suffisant ; on ne peut plus en retirer un une fois soumise.
    let g = b.ent.ajouter_encadrant_groupe_a(b.etab, b.cpe, CPE, g.id, b.compta, h(1)).await.unwrap();
    assert_eq!(g.encadrants.len(), 2);
    soumettre(&b, g.id).await;
    let r = b.ent.retirer_encadrant_groupe_a(b.etab, b.cpe, CPE, g.id, b.compta, h(2)).await;
    assert!(refus_de(&r, "insuffisant"));
}

#[tokio::test]
async fn groupe_preavis_et_horaires() {
    let Some(b) = banc().await else { return };
    let g = brouillon(&b, vec![b.prof], vec![b.a[0]]).await;
    // Préavis de 24 h par défaut : soumise la veille au soir pour le lendemain 14 h, c'est trop tard.
    let r = b.ent.soumettre_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, debut() - Duration::hours(10)).await;
    assert!(refus_de(&r, "préavis"));
    // Une sortie déjà commencée, ou de plus de 14 jours, ne se crée pas ; pas de santé dans le titre.
    let mut s = saisie_groupe(vec![b.prof], vec![]);
    s.debut = t0() - Duration::hours(1);
    s.fin = t0() + Duration::hours(2);
    assert!(refus_de(&b.ent.creer_sortie_groupe_a(b.etab, b.cpe, CPE, &s, t0()).await, "avant son début"));
    let mut s = saisie_groupe(vec![b.prof], vec![]);
    s.fin = s.debut + Duration::days(20);
    assert!(refus_de(&b.ent.creer_sortie_groupe_a(b.etab, b.cpe, CPE, &s, t0()).await, "horaires"));
    let mut s = saisie_groupe(vec![b.prof], vec![]);
    s.titre = "Sortie pour le traitement de l'asthme".into();
    assert!(refus_de(&b.ent.creer_sortie_groupe_a(b.etab, b.cpe, CPE, &s, t0()).await, "santé"));
}

#[tokio::test]
async fn groupe_figee_apres_la_soumission() {
    let Some(b) = banc().await else { return };
    let g = brouillon(&b, vec![b.prof], vec![b.a[0]]).await;
    let nouveau = SortieGroupeSaisieDto { titre: "Autre titre".into(), ..saisie_groupe(vec![], vec![]) };
    assert_eq!(b.ent.modifier_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, &nouveau, t0()).await.unwrap().titre, "Autre titre");
    soumettre(&b, g.id).await;
    assert!(refus_de(&b.ent.modifier_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, &nouveau, h(2)).await, "soumise"));
    // Même par SQL direct : horaires, titre, lieu figés ; pas de suppression ; décision figée.
    let pool = b.ent.pool_verif();
    assert!(sqlx::query("UPDATE ent_internat_sortie_groupe SET titre = 'X' WHERE etablissement_id = $1 AND id = $2").bind(b.etab).bind(g.id).execute(pool).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_internat_sortie_groupe WHERE etablissement_id = $1").bind(b.etab).execute(pool).await.is_err());
    assert!(sqlx::query("UPDATE ent_internat_sortie_groupe SET statut = 'close' WHERE etablissement_id = $1 AND id = $2").bind(b.etab).bind(g.id).execute(pool).await.is_err(), "soumise → close : transition refusée");
    assert!(sqlx::query("DELETE FROM ent_internat_sortie_groupe_participant WHERE etablissement_id = $1").bind(b.etab).execute(pool).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_internat_sortie_groupe_fait WHERE etablissement_id = $1").bind(b.etab).execute(pool).await.is_err());
    assert!(sqlx::query("TRUNCATE ent_internat_sortie_groupe_fait").execute(pool).await.is_err(), "TRUNCATE interdit (E2)");
}

#[tokio::test]
async fn groupe_duplication_sans_reprendre_les_accords() {
    // INT-177 : la piscine du mardi se duplique ; chaque sortie a ses propres réponses.
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e, b.a[1]]).await.id).await;
    repondre(&b, p, g.id, e, true).await.unwrap();
    let semaine_suivante = DuplicationGroupeDto { debut: debut() + Duration::days(7), fin: fin() + Duration::days(7) };
    let d = b.ent.dupliquer_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, &semaine_suivante, h(3)).await.unwrap();
    assert_eq!((d.statut.as_str(), d.modele_id), ("brouillon", Some(g.id)));
    assert_eq!(d.encadrants, vec![b.prof]);
    assert_eq!(d.participants.len(), 2);
    assert!(d.participants.iter().all(|p| p.etat == "invite"), "aucun accord repris");
    assert!(b.ent.dupliquer_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, &DuplicationGroupeDto { debut: debut() - Duration::days(30), fin: fin() - Duration::days(30) }, h(3)).await.is_err());
    // Un élève qui a quitté l'internat entre-temps est écarté, jamais invité en silence.
    sqlx::query("UPDATE ent_internat_sejour SET au = '2026-04-20' WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(b.etab)
        .bind(b.a[1])
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let d2 = b.ent.dupliquer_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, &DuplicationGroupeDto { debut: debut() + Duration::days(14), fin: fin() + Duration::days(14) }, h(3)).await.unwrap();
    assert_eq!(d2.participants.len(), 1);
    assert_eq!(d2.ecartes, vec![EcarteDto { eleve_id: b.a[1], motif: "plus_interne".into() }]);
}

#[tokio::test]
async fn groupe_annulation_prevenue_et_impossible_apres_un_depart() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1, e2]).await.id).await;
    repondre(&b, p1, g.id, e1, true).await.unwrap();
    assert!(refus_de(&b.ent.annuler_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, &AnnulationGroupeDto { motif_code: "pluie".into() }, h(3)).await, "motif"));
    let a = b.ent.annuler_sortie_groupe_a(b.etab, b.cpe, CPE, g.id, &AnnulationGroupeDto { motif_code: "meteo".into() }, h(3)).await.unwrap();
    assert_eq!((a.statut.as_str(), a.annule_motif_code.as_deref()), ("annulee", Some("meteo")));
    // Une famille est prévenue par élève (identifiants et code, pas de texte).
    let evts: Vec<_> = b.evenements("internat.sortie_groupe_autorisation").await.into_iter().filter(|e| e["etat"] == "annulee").collect();
    assert_eq!(evts.len(), 2);
    // Pas d'annulation une fois que des élèves sont partis.
    let b2 = banc().await.unwrap();
    let e = b2.a[0];
    let (p, _) = parent(&b2, e, true).await;
    let g2 = soumettre(&b2, brouillon(&b2, vec![b2.prof], vec![e]).await.id).await;
    repondre(&b2, p, g2.id, e, true).await.unwrap();
    accepter(&b2, g2.id).await;
    depart(&b2, g2.id, vec![e]).await;
    let r = b2.ent.annuler_sortie_groupe_a(b2.etab, b2.cpe, CPE, g2.id, &AnnulationGroupeDto { motif_code: "meteo".into() }, jour_j(20)).await;
    assert!(refus_de(&r, "partis"));
}

#[tokio::test]
async fn groupe_ne_retire_jamais_de_l_attendu() {
    // « Pas de départ constaté = élève attendu » : une sortie de groupe, même partie, ne retire pas l'élève de l'attendu
    // de la nuit (il se pointe « en activité »).
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    repondre(&b, p, g.id, e, true).await.unwrap();
    accepter(&b, g.id).await;
    depart(&b, g.id, vec![e]).await;
    let statut: Option<String> = sqlx::query_scalar("SELECT statut FROM ent_internat_attendu_calcule($1, $2) WHERE eleve_id = $3")
        .bind(b.etab)
        .bind(avril(16))
        .bind(e)
        .fetch_optional(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(statut.as_deref(), Some("attendu"));
}

#[tokio::test]
async fn groupe_famille_ne_voit_que_son_enfant() {
    // R-IT32 : jamais un autre participant, jamais le nombre d'inscrits.
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1, e2]).await.id).await;
    let vue = b.ent.sorties_groupe_eleve_a(b.etab, p1, PARENT, e1, h(2)).await.unwrap();
    assert_eq!(vue.len(), 1);
    assert_eq!((vue[0].eleve_id, vue[0].etat.as_str(), vue[0].peut_repondre), (e1, "en_attente", true));
    let json = serde_json::to_string(&vue).unwrap();
    assert!(!json.contains(&e2.to_string()), "aucun autre élève dans la réponse");
    // Un parent ne lit pas la sortie d'un autre enfant, ni le détail du personnel, ni la liste.
    assert!(est_interdit(&b.ent.sorties_groupe_eleve_a(b.etab, p1, PARENT, e2, h(2)).await));
    assert!(est_interdit(&b.ent.sortie_groupe_a(b.etab, p1, PARENT, g.id, h(2)).await));
    assert!(est_interdit(&b.ent.lister_sorties_groupe_a(b.etab, p2, PARENT, avril(1), avril(30)).await));
    // Un brouillon n'est pas visible des familles (elles ne sont pas encore interrogées).
    let b2 = banc().await.unwrap();
    let (pp, _) = parent(&b2, b2.a[0], true).await;
    brouillon(&b2, vec![b2.prof], vec![b2.a[0]]).await;
    assert!(b2.ent.sorties_groupe_eleve_a(b2.etab, pp, PARENT, b2.a[0], h(2)).await.unwrap().is_empty());
}

#[tokio::test]
async fn groupe_lecture_du_personnel_bornee_et_cloisonnee() {
    let Some(b) = banc().await else { return };
    let g = brouillon(&b, vec![b.prof], vec![b.a[0], b.b[0]]).await;
    // L'accompagnateur lit SA sortie ; un autre professeur (sans droit d'internat) ne la lit pas ; il ne l'organise pas.
    assert_eq!(b.ent.sortie_groupe_a(b.etab, b.prof, PROF, g.id, h(1)).await.unwrap().participants.len(), 2);
    let tiers_prof = compte(&b.ent, b.etab, PROF).await;
    assert!(est_interdit(&b.ent.sortie_groupe_a(b.etab, tiers_prof, PROF, g.id, h(1)).await));
    assert!(est_interdit(&b.ent.creer_sortie_groupe_a(b.etab, b.prof, PROF, &saisie_groupe(vec![b.prof], vec![]), t0()).await));
    assert!(est_interdit(&b.ent.soumettre_sortie_groupe_a(b.etab, b.prof, PROF, g.id, h(1)).await));
    // La liste : l'accompagnateur ne voit que ses sorties, sans nom ; un compte sans rapport est refusé.
    let liste = b.ent.lister_sorties_groupe_a(b.etab, b.prof, PROF, avril(1), avril(30)).await.unwrap();
    assert_eq!(liste.len(), 1);
    assert!(est_interdit(&b.ent.lister_sorties_groupe_a(b.etab, tiers_prof, PROF, avril(1), avril(30)).await));
    assert!(b.ent.lister_sorties_groupe_a(b.etab, b.cpe, CPE, avril(30), avril(1)).await.is_err());
    // Un accompagnateur n'appelle pas une sortie qu'il n'accompagne pas.
    let autre = brouillon(&b, vec![b.compta], vec![b.a[1]]).await;
    assert!(est_interdit(&b.ent.depart_groupe_a(b.etab, b.prof, PROF, autre.id, &AppelGroupeDto { eleves: vec![b.a[1]] }, jour_j(10)).await));
    // Cloisonnement d'établissement : un autre établissement ne voit rien de celle-ci.
    let b2 = banc().await.unwrap();
    let r = b2.ent.sortie_groupe_a(b2.etab, b2.cpe, CPE, g.id, h(1)).await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))), "{r:?}");
    assert!(est_interdit(&b2.ent.sortie_groupe_a(b.etab, b2.cpe, CPE, g.id, h(1)).await), "ni avec le compte d'un autre établissement");
}

#[tokio::test]
async fn groupe_direction_de_formation_ne_decide_que_ses_eleves() {
    // R-IT30 : une direction de formation (portée désignée) ne décide pas d'une sortie qui mêle d'autres formations.
    let Some(b) = banc().await else { return };
    let classes = classes_des_deux_zones(&b).await;
    let formation_a = personnel_portee(&b, &[Action::InternatSortiesDecider], Portee::Designes, vec![classes[0]]).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![b.a[0], b.b[0]]).await.id).await;
    let r = b.ent.decider_sortie_groupe_a(b.etab, formation_a, PERSONNEL, g.id, &DecisionGroupeDto { accepte: true, motif_code: None }, h(3)).await;
    assert!(est_interdit(&r), "une sortie qui déborde de sa formation lui est fermée");
    // Une sortie de sa seule formation : permis.
    let b2 = banc().await.unwrap();
    let c2 = classes_des_deux_zones(&b2).await;
    let f2 = personnel_portee(&b2, &[Action::InternatSortiesDecider], Portee::Designes, vec![c2[0]]).await;
    let g2 = soumettre(&b2, brouillon(&b2, vec![b2.prof], vec![b2.a[0]]).await.id).await;
    let d = b2.ent.decider_sortie_groupe_a(b2.etab, f2, PERSONNEL, g2.id, &DecisionGroupeDto { accepte: true, motif_code: None }, h(3)).await.unwrap();
    assert_eq!(d.statut, "acceptee");
}

// ───────────────────────── Liaison codée ─────────────────────────

/// Soir du mardi 14 avril, 22 h 30 (Paris) : nuit courante = 14 avril.
fn soir_14(mi: i64) -> DateTime<Utc> {
    paris(avril(14), 22, 30) + Duration::minutes(mi)
}

fn fait(eleve: Uuid, code: &str, texte: Option<&str>) -> LiaisonSaisieDto {
    LiaisonSaisieDto { eleve_id: eleve, code: code.into(), texte: texte.map(str::to_string), nuit: Some(avril(14)) }
}

#[tokio::test]
async fn liaison_catalogue_ecriture_et_bornes() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let codes = b.ent.lister_codes_liaison_a(b.etab, b.cpe, CPE).await.unwrap();
    assert!(codes.iter().any(|c| c.code == "demande_cpe" && c.pour_le_jour));
    assert!(codes.iter().all(|c| !c.libelle.to_lowercase().contains("sant")), "aucune catégorie de santé");
    // Écriture normale : texte court conservé, photographie du réglage « pour le jour ».
    let l = b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &fait(e, "demande_cpe", Some("Souhaite parler demain")), soir_14(0)).await.unwrap();
    assert_eq!((l.texte.as_deref(), l.pour_le_jour, l.nuit), (Some("Souhaite parler demain"), true, avril(14)));
    // Texte de santé : refusé avec l'invite vers le socle santé (R-IT6, R-IT7).
    let r = b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &fait(e, "fatigue", Some("Prend un traitement contre l'asthme")), soir_14(1)).await;
    assert!(refus_de(&r, "santé"));
    // 300 caractères au plus.
    let long = "a".repeat(301);
    assert!(refus_de(&b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &fait(e, "fatigue", Some(&long)), soir_14(1)).await, "trop long"));
    // Code inconnu ou désactivé.
    assert!(refus_de(&b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &fait(e, "inventé", None), soir_14(1)).await, "inconnu"));
    b.ent.modifier_code_liaison_a(b.etab, b.cpe, CPE, "fatigue", &LiaisonCodeSaisieDto { actif: Some(false), ..Default::default() }, soir_14(2)).await.unwrap();
    assert!(refus_de(&b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &fait(e, "fatigue", None), soir_14(3)).await, "plus utilisé"));
    // Une nuit future, ou de plus de 7 nuits en arrière : refusée.
    let mut f = fait(e, "objet_perdu", None);
    f.nuit = Some(avril(15));
    assert!(refus_de(&b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &f, soir_14(3)).await, "nuit"));
    f.nuit = Some(avril(2));
    assert!(refus_de(&b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &f, soir_14(3)).await, "nuit"));
    // Un élève qui n'est pas interne cette nuit-là n'a pas de fait de liaison.
    let externe = eleve_fixe(&b.ent, b.etab, "Externe").await;
    assert!(refus_de(&b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &fait(externe, "objet_perdu", None), soir_14(3)).await, "interne"));
    // Un code propre à l'établissement : jamais de santé.
    let nouveau = LiaisonCodeCreationDto { code: "retard_douche".into(), libelle: "Retard à la douche".into(), pour_le_jour: false };
    assert_eq!(b.ent.creer_code_liaison_a(b.etab, b.cpe, CPE, &nouveau, soir_14(4)).await.unwrap().code, "retard_douche");
    let sante = LiaisonCodeCreationDto { code: "allergie".into(), libelle: "Allergie signalée".into(), pour_le_jour: false };
    assert!(refus_de(&b.ent.creer_code_liaison_a(b.etab, b.cpe, CPE, &sante, soir_14(4)).await, "santé"));
    assert!(refus_de(&b.ent.creer_code_liaison_a(b.etab, b.cpe, CPE, &nouveau, soir_14(5)).await, "existe"));
    // L'outbox ne porte jamais le texte, seulement le code et la nuit.
    let evts = b.evenements("internat.liaison_ecrite").await;
    assert_eq!(evts.len(), 1);
    assert!(!evts[0].to_string().contains("parler"), "aucun texte libre dans l'outbox");
}

#[tokio::test]
async fn liaison_lecture_reservee_a_l_equipe_r_it8() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &fait(e, "conflit_regle", Some("Réglé entre eux")), soir_14(0)).await.unwrap();
    // Équipe, CPE, direction : oui, et chaque lecture est journalisée (R-IT38).
    assert_eq!(b.ent.liaison_eleve_a(b.etab, b.cpe, CPE, e, soir_14(5)).await.unwrap().len(), 1);
    assert_eq!(b.ent.liaison_eleve_a(b.etab, b.dir, DIR, e, soir_14(6)).await.unwrap().len(), 1);
    let lectures: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_lecture_sensible WHERE etablissement_id = $1 AND objet = 'liaison'")
        .bind(b.etab)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert!(lectures >= 2);
    // Ni professeur, ni comptabilité, ni famille, ni élève, ni accompagnateur sans droit : refusé (refus par défaut).
    let (pere, _) = parent(&b, e, true).await;
    let eleve_u = compte_eleve(&b, e, NaiveDate::from_ymd_opt(2007, 1, 1).unwrap()).await;
    for (u, r) in [(b.prof, PROF), (b.compta, "compta"), (pere, PARENT), (eleve_u, ELEVE)] {
        assert!(est_interdit(&b.ent.liaison_eleve_a(b.etab, u, r, e, soir_14(7)).await), "lecture refusée à {r}");
        assert!(est_interdit(&b.ent.liaison_nuit_a(b.etab, u, r, avril(14), soir_14(7)).await), "liste refusée à {r}");
        assert!(est_interdit(&b.ent.ecrire_liaison_a(b.etab, u, r, &fait(e, "objet_perdu", None), soir_14(7)).await), "écriture refusée à {r}");
    }
    // Aucun professeur, mais le professeur principal est prévenu SANS texte quand le CPE le décide (INT-183).
    let id = b.ent.liaison_eleve_a(b.etab, b.cpe, CPE, e, soir_14(8)).await.unwrap()[0].id;
    let g = |geste: &str, motif: Option<&str>| LiaisonGesteDto { geste: geste.into(), motif_code: motif.map(str::to_string) };
    let l = b.ent.geste_liaison_a(b.etab, b.cpe, CPE, id, &g("signaler_pp", None), soir_14(9)).await.unwrap();
    assert!(l.signale_pp);
    assert!(refus_de(&b.ent.geste_liaison_a(b.etab, b.cpe, CPE, id, &g("signaler_pp", None), soir_14(9)).await, "déjà"));
    let evts = b.evenements("internat.signal_pp").await;
    assert_eq!(evts.len(), 1);
    assert!(!evts[0].to_string().contains("Réglé"), "le signalement ne porte aucun texte");
}

#[tokio::test]
async fn liaison_vie_scolaire_de_jour_ne_voit_que_les_faits_pour_le_jour_sans_texte() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let jour = personnel_avec(&b, &[Action::InternatLiaisonJour]).await;
    b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &fait(e, "demande_cpe", Some("Souhaite parler")), soir_14(0)).await.unwrap(); // pour le jour
    b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &fait(e, "fatigue", Some("Couché tôt")), soir_14(1)).await.unwrap(); // pas pour le jour
    b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &fait(b.a[1], "objet_perdu", None), soir_14(2)).await.unwrap(); // pour le jour
    let vue = b.ent.liaison_nuit_a(b.etab, jour, PERSONNEL, avril(14), soir_14(10)).await.unwrap();
    assert_eq!(vue.len(), 2, "les seuls faits « pour le jour »");
    assert!(vue.iter().all(|l| l.pour_le_jour && l.texte.is_none()), "jamais le texte libre");
    // Aucune chronologie d'élève, aucune écriture, aucun événement pour ce profil.
    assert!(est_interdit(&b.ent.liaison_eleve_a(b.etab, jour, PERSONNEL, e, soir_14(11)).await));
    assert!(est_interdit(&b.ent.ecrire_liaison_a(b.etab, jour, PERSONNEL, &fait(e, "objet_perdu", None), soir_14(11)).await));
    assert!(est_interdit(&b.ent.evenements_nuit_a(b.etab, jour, PERSONNEL, avril(14), soir_14(11)).await));
    // L'équipe complète, elle, lit les trois faits avec leur texte.
    let tout = b.ent.liaison_nuit_a(b.etab, b.cpe, CPE, avril(14), soir_14(12)).await.unwrap();
    assert_eq!(tout.len(), 3);
    assert!(tout.iter().any(|l| l.texte.as_deref() == Some("Couché tôt")));
    // Masqué avec un motif : disparaît aussi du matin ; le « à revoir » se pose et se lève ; le masquage est définitif.
    let id = vue[0].id;
    let g = |geste: &str, motif: Option<&str>| LiaisonGesteDto { geste: geste.into(), motif_code: motif.map(str::to_string) };
    assert!(refus_de(&b.ent.geste_liaison_a(b.etab, b.cpe, CPE, id, &g("masquer", None), soir_14(13)).await, "motif"));
    assert!(est_interdit(&b.ent.geste_liaison_a(b.etab, jour, PERSONNEL, id, &g("masquer", Some("erreur")), soir_14(13)).await));
    let m = b.ent.geste_liaison_a(b.etab, b.cpe, CPE, id, &g("masquer", Some("erreur")), soir_14(14)).await.unwrap();
    assert!(m.masque);
    assert_eq!(b.ent.liaison_nuit_a(b.etab, jour, PERSONNEL, avril(14), soir_14(15)).await.unwrap().len(), 1);
    assert!(refus_de(&b.ent.geste_liaison_a(b.etab, b.cpe, CPE, id, &g("a_revoir", None), soir_14(16)).await, "masqué"));
    // Même par SQL direct : le contenu ne change pas, rien ne se supprime.
    let pool = b.ent.pool_verif();
    assert!(sqlx::query("UPDATE ent_internat_liaison SET texte = 'autre' WHERE etablissement_id = $1").bind(b.etab).execute(pool).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_internat_liaison WHERE etablissement_id = $1").bind(b.etab).execute(pool).await.is_err());
    assert!(sqlx::query("UPDATE ent_internat_liaison SET masque_le = NULL, masque_par_user_id = NULL WHERE etablissement_id = $1 AND id = $2").bind(b.etab).bind(id).execute(pool).await.is_err());
    // « À revoir » : posé puis levé, tracé.
    let autre = tout.iter().find(|l| l.code == "fatigue").unwrap().id;
    assert!(b.ent.geste_liaison_a(b.etab, b.cpe, CPE, autre, &g("a_revoir", None), soir_14(17)).await.unwrap().a_revoir);
    assert!(!b.ent.geste_liaison_a(b.etab, b.cpe, CPE, autre, &g("lever_a_revoir", None), soir_14(18)).await.unwrap().a_revoir);
    let gestes: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_liaison_fait WHERE etablissement_id = $1 AND liaison_id = $2")
        .bind(b.etab)
        .bind(autre)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(gestes, 2);
}

#[tokio::test]
async fn liaison_perimetre_verifie_au_serveur_r_it30() {
    let Some(b) = banc().await else { return };
    let classes = classes_des_deux_zones(&b).await;
    let formation_a = personnel_portee(&b, &[Action::InternatLiaisonEcrire, Action::InternatLiaisonLire], Portee::Designes, vec![classes[0]]).await;
    // Il écrit et lit pour SES élèves seulement.
    let l = b.ent.ecrire_liaison_a(b.etab, formation_a, PERSONNEL, &fait(b.a[0], "objet_perdu", None), soir_14(0)).await.unwrap();
    assert_eq!(l.eleve_id, b.a[0]);
    assert!(est_interdit(&b.ent.ecrire_liaison_a(b.etab, formation_a, PERSONNEL, &fait(b.b[0], "objet_perdu", None), soir_14(1)).await));
    b.ent.ecrire_liaison_a(b.etab, b.cpe, CPE, &fait(b.b[0], "objet_perdu", None), soir_14(2)).await.unwrap();
    assert!(est_interdit(&b.ent.liaison_eleve_a(b.etab, formation_a, PERSONNEL, b.b[0], soir_14(3)).await));
    let nuit = b.ent.liaison_nuit_a(b.etab, formation_a, PERSONNEL, avril(14), soir_14(3)).await.unwrap();
    assert_eq!(nuit.len(), 1, "les faits de l'autre formation ne sont pas listés");
    assert_eq!(nuit[0].eleve_id, b.a[0]);
}

// ───────────────────────── Événements de nuit ─────────────────────────

fn evt(nature: &str) -> EvenementSaisieDto {
    EvenementSaisieDto { nature: nature.into(), niveau: None, a: Some(soir_14(0)), zone_id: None, chambre_id: None, mesures_code: None, participants: vec![], complement: None }
}

fn pe(eleve: Uuid, role: &str) -> ParticipantEvenementSaisieDto {
    ParticipantEvenementSaisieDto { eleve_id: Some(eleve), user_id: None, role: role.into() }
}

#[tokio::test]
async fn evenement_declaration_completion_sans_suppression() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let mut s = evt("incident");
    s.zone_id = Some(b.zone_a);
    s.chambre_id = Some(b.chambre_a);
    s.mesures_code = Some("adultes_sur_place".into());
    s.participants = vec![pe(e1, "auteur_presume"), pe(e2, "victime"), ParticipantEvenementSaisieDto { eleve_id: None, user_id: Some(b.prof), role: "temoin".into() }];
    s.complement = Some("Dispute à propos d'une console".into());
    let d = b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1)).await.unwrap();
    assert_eq!((d.nature.as_str(), d.niveau.as_str(), d.nuit), ("incident", "information", avril(14)));
    assert_eq!(d.participants.len(), 3);
    assert_eq!(d.complements.len(), 1);
    // Complété par un fait ajouté (l'auteur ou la gestion), jamais modifié ; un tiers sans droit ne complète pas.
    let c = b.ent.completer_evenement_a(b.etab, b.cpe, CPE, d.id, &ComplementSaisieDto { texte: "Réglé le lendemain".into() }, soir_14(30)).await.unwrap();
    assert_eq!(c.complements.len(), 2);
    assert!(est_interdit(&b.ent.completer_evenement_a(b.etab, b.prof, PROF, d.id, &ComplementSaisieDto { texte: "x".into() }, soir_14(31)).await));
    assert!(refus_de(&b.ent.completer_evenement_a(b.etab, b.cpe, CPE, d.id, &ComplementSaisieDto { texte: "Il a une allergie".into() }, soir_14(31)).await, "santé"));
    let pool = b.ent.pool_verif();
    assert!(sqlx::query("DELETE FROM ent_internat_evenement WHERE etablissement_id = $1").bind(b.etab).execute(pool).await.is_err(), "DI-17 : un événement ne se supprime pas");
    assert!(sqlx::query("UPDATE ent_internat_evenement SET nature = 'alarme' WHERE etablissement_id = $1").bind(b.etab).execute(pool).await.is_err());
    assert!(sqlx::query("UPDATE ent_internat_evenement_complement SET texte = 'autre' WHERE etablissement_id = $1").bind(b.etab).execute(pool).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_internat_evenement_participant WHERE etablissement_id = $1").bind(b.etab).execute(pool).await.is_err());
    // Transmission au 13 : une fois, par la gestion ; l'outbox ne porte aucun texte ni aucun nom.
    let t = b.ent.transmettre_evenement_a(b.etab, b.cpe, CPE, d.id, soir_14(40)).await.unwrap();
    assert!(t.transmis_13_le.is_some());
    assert!(refus_de(&b.ent.transmettre_evenement_a(b.etab, b.cpe, CPE, d.id, soir_14(41)).await, "déjà"));
    assert!(est_interdit(&b.ent.transmettre_evenement_a(b.etab, b.prof, PROF, d.id, soir_14(41)).await));
    for t in ["internat.evenement_declare", "internat.evenement_complete", "internat.evenement_transmis"] {
        for e in b.evenements(t).await {
            let s = e.to_string();
            assert!(!s.contains("console") && !s.contains("lendemain") && !s.contains("Nuit"), "texte ou nom dans l'outbox : {s}");
        }
    }
    // Lecture de l'équipe, journalisée ; refusée aux autres.
    assert_eq!(b.ent.evenements_nuit_a(b.etab, b.cpe, CPE, avril(14), soir_14(50)).await.unwrap().len(), 1);
    assert!(b.ent.evenement_a(b.etab, b.dir, DIR, d.id, soir_14(51)).await.is_ok());
    let (pere, _) = parent(&b, e2, true).await;
    for (u, r) in [(b.prof, PROF), (pere, PARENT), (b.compta, "compta")] {
        assert!(est_interdit(&b.ent.evenement_a(b.etab, u, r, d.id, soir_14(52)).await), "lecture refusée à {r}");
        assert!(est_interdit(&b.ent.evenements_nuit_a(b.etab, u, r, avril(14), soir_14(52)).await));
        assert!(est_interdit(&b.ent.declarer_evenement_a(b.etab, u, r, &evt("alarme"), soir_14(52)).await));
    }
    let journal: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_lecture_sensible WHERE etablissement_id = $1 AND objet = 'evenement'")
        .bind(b.etab)
        .fetch_one(pool)
        .await
        .unwrap();
    assert!(journal >= 2);
}

#[tokio::test]
async fn evenement_validations_et_participants() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    assert!(refus_de(&b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &evt("fête"), soir_14(1)).await, "nature"));
    let mut s = evt("incident");
    s.niveau = Some("urgent".into());
    assert!(refus_de(&b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1)).await, "niveau"));
    let mut s = evt("incident");
    s.a = Some(soir_14(1) + Duration::hours(2));
    assert!(refus_de(&b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1)).await, "heure"));
    s.a = Some(soir_14(1) - Duration::hours(60));
    assert!(refus_de(&b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1)).await, "heure"));
    // Un adulte n'est que témoin ou présent, et du personnel ; un participant est un élève OU un adulte.
    let (pere, _) = parent(&b, e, true).await;
    let mut s = evt("incident");
    s.participants = vec![ParticipantEvenementSaisieDto { eleve_id: None, user_id: Some(pere), role: "temoin".into() }];
    assert!(refus_de(&b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1)).await, "personnel"));
    s.participants = vec![ParticipantEvenementSaisieDto { eleve_id: None, user_id: Some(b.prof), role: "victime".into() }];
    assert!(refus_de(&b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1)).await, "adulte"));
    s.participants = vec![ParticipantEvenementSaisieDto { eleve_id: Some(e), user_id: Some(b.prof), role: "temoin".into() }];
    assert!(refus_de(&b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1)).await, "jamais les deux"));
    s.participants = vec![pe(e, "victime"), pe(e, "victime")];
    assert!(refus_de(&b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1)).await, "double"));
    // Zone ou chambre d'un autre établissement : inconnue.
    let mut s = evt("degradation");
    s.chambre_id = Some(Uuid::new_v4());
    assert!(refus_de(&b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1)).await, "inconnue"));
    // Périmètre : un déclarant de formation ne cite pas un élève d'une autre formation.
    let classes = classes_des_deux_zones(&b).await;
    let formation_a = personnel_portee(&b, &[Action::InternatEvenementDeclarer, Action::InternatLiaisonLire], Portee::Designes, vec![classes[0]]).await;
    let mut s = evt("comportement");
    s.participants = vec![pe(b.b[0], "temoin")];
    assert!(est_interdit(&b.ent.declarer_evenement_a(b.etab, formation_a, PERSONNEL, &s, soir_14(1)).await));
    s.participants = vec![pe(b.a[0], "temoin")];
    let ok = b.ent.declarer_evenement_a(b.etab, formation_a, PERSONNEL, &s, soir_14(1)).await.unwrap();
    // Un événement qui mêle une autre formation n'est pas montré à ce lecteur (R-IT30).
    let mut mixte = evt("incident");
    mixte.participants = vec![pe(b.a[0], "auteur_presume"), pe(b.b[0], "victime")];
    let m = b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &mixte, soir_14(2)).await.unwrap();
    let vus = b.ent.evenements_nuit_a(b.etab, formation_a, PERSONNEL, avril(14), soir_14(3)).await.unwrap();
    assert_eq!(vus.iter().map(|v| v.id).collect::<Vec<_>>(), vec![ok.id]);
    assert!(matches!(b.ent.evenement_a(b.etab, formation_a, PERSONNEL, m.id, soir_14(3)).await, Err(EntDbError::Domain(DomainError::NotFound))));
}

#[tokio::test]
async fn evenement_malaise_sans_texte_libre_et_signal_vers_le_socle_sante() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let mut s = evt("malaise");
    s.participants = vec![pe(e, "victime")];
    s.zone_id = Some(b.zone_a);
    s.complement = Some("A vomi".into());
    assert!(refus_de(&b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1)).await, "malaise"));
    s.complement = None;
    let d = b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(2)).await.unwrap();
    assert!(d.complements.is_empty());
    // Aucun complément libre sur un malaise, aucune transmission à la discipline.
    assert!(refus_de(&b.ent.completer_evenement_a(b.etab, b.cpe, CPE, d.id, &ComplementSaisieDto { texte: "Va mieux".into() }, soir_14(3)).await, "malaise"));
    assert!(refus_de(&b.ent.transmettre_evenement_a(b.etab, b.cpe, CPE, d.id, soir_14(3)).await, "discipline"));
    // Signal codé vers le 34 : l'élève et la position, rien d'autre.
    let sig = b.evenements("internat.malaise_signale").await;
    assert_eq!(sig.len(), 1);
    assert_eq!(sig[0].as_object().unwrap().keys().cloned().collect::<std::collections::BTreeSet<_>>(), ["chambre_id", "nuit", "zone_id"].into_iter().map(String::from).collect());
}

#[tokio::test]
async fn evenement_alerte_critique_position_seulement_et_intervention_sans_nom() {
    let Some(b) = banc().await else { return };
    let mut s = evt("alarme");
    s.niveau = Some("critique".into());
    s.zone_id = Some(b.zone_b);
    s.chambre_id = Some(b.chambre_b);
    s.participants = vec![pe(b.b[0], "present")];
    let d = b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &s, soir_14(1)).await.unwrap();
    let alertes = b.evenements("internat.alerte").await;
    assert_eq!(alertes.len(), 1);
    assert_eq!(alertes[0]["zone_id"], b.zone_b.to_string());
    assert!(!alertes[0].to_string().contains(&b.b[0].to_string()), "la position, rien d'autre : aucun élève");
    // Un événement d'information n'alerte personne.
    b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &evt("intrusion"), soir_14(2)).await.unwrap();
    assert_eq!(b.evenements("internat.alerte").await.len(), 1);
    // INT-188 : intervention technique sur la chambre, sans nom d'élève, une fois, avec position.
    let i = b.ent.demander_intervention_a(b.etab, b.cpe, CPE, d.id, soir_14(3)).await.unwrap();
    assert!(i.intervention_demandee_le.is_some());
    let evts = b.evenements("internat.intervention_demandee").await;
    assert_eq!(evts.len(), 1);
    assert!(!evts[0].to_string().contains(&b.b[0].to_string()));
    assert!(refus_de(&b.ent.demander_intervention_a(b.etab, b.cpe, CPE, d.id, soir_14(4)).await, "déjà"));
    let sans_position = b.ent.declarer_evenement_a(b.etab, b.cpe, CPE, &evt("degradation"), soir_14(5)).await.unwrap();
    assert!(refus_de(&b.ent.demander_intervention_a(b.etab, b.cpe, CPE, sans_position.id, soir_14(6)).await, "position"));
    assert!(est_interdit(&b.ent.demander_intervention_a(b.etab, b.prof, PROF, d.id, soir_14(7)).await));
}

#[tokio::test]
async fn preoccupation_ne_garde_aucun_contenu_r_it29() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let accuse = b.ent.ouvrir_preoccupation_a(b.etab, b.cpe, CPE, &PreoccupationDto { eleve_id: e }, soir_14(1)).await.unwrap();
    assert_eq!(accuse.a, soir_14(1));
    // La table n'a aucune colonne de contenu : un adulte, un élève, un instant.
    let colonnes: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::text FROM information_schema.columns WHERE table_name = 'ent_internat_preoccupation' ORDER BY column_name",
    )
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(colonnes, ["a", "cree_le", "eleve_id", "etablissement_id", "id", "par_user_id"]);
    let evts = b.evenements("internat.preoccupation_signalee").await;
    assert_eq!(evts.len(), 1);
    assert_eq!(evts[0].as_object().unwrap().keys().collect::<Vec<_>>(), ["a"]);
    // Tout adulte de l'internat (droit de déclarer) peut l'ouvrir ; ni professeur, ni famille, ni élève ne le peuvent ici.
    let (pere, _) = parent(&b, e, true).await;
    for (u, r) in [(b.prof, PROF), (pere, PARENT)] {
        assert!(est_interdit(&b.ent.ouvrir_preoccupation_a(b.etab, u, r, &PreoccupationDto { eleve_id: e }, soir_14(2)).await));
    }
    assert!(sqlx::query("DELETE FROM ent_internat_preoccupation WHERE etablissement_id = $1").bind(b.etab).execute(b.ent.pool_verif()).await.is_err());
}

// ───────────────────────── Temps du soir : études et activités ─────────────────────────

fn etude(places: Option<i16>) -> TempsSaisieDto {
    TempsSaisieDto {
        nature: "etude".into(),
        libelle: "Étude du soir".into(),
        lieu: Some("Salle 12".into()),
        debut: paris(avril(15), 19, 0),
        fin: paris(avril(15), 20, 30),
        encadrant_user_id: None,
        places,
    }
}

async fn creer_etude(b: &Banc, places: Option<i16>) -> TempsDto {
    let mut s = etude(places);
    s.encadrant_user_id = Some(b.prof);
    b.ent.creer_temps_a(b.etab, b.cpe, CPE, &s, t0()).await.unwrap()
}

fn mi_etude(mi: i64) -> DateTime<Utc> {
    paris(avril(15), 19, 0) - Duration::hours(2) + Duration::minutes(mi)
}

#[tokio::test]
async fn temps_places_inscription_et_chevauchement() {
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (b.a[0], b.a[1], b.a[2]);
    let t = creer_etude(&b, Some(2)).await;
    assert_eq!((t.nature.as_str(), t.statut.as_str(), t.nb_inscrits), ("etude", "prevu", 0));
    let ins = |u: Uuid, r: &'static str, e: Uuid, mi: i64| {
        let b = &b;
        async move { b.ent.inscrire_temps_a(b.etab, u, r, t.id, &InscriptionTempsDto { eleve_id: e }, mi_etude(mi)).await }
    };
    // Le personnel d'internat inscrit ; les places sont comptées sous verrou.
    assert_eq!(ins(b.cpe, CPE, e1, 0).await.unwrap().nb_inscrits, 1);
    assert_eq!(ins(b.cpe, CPE, e2, 1).await.unwrap().nb_inscrits, 2);
    assert!(refus_de(&ins(b.cpe, CPE, e3, 2).await, "place"));
    assert!(refus_de(&ins(b.cpe, CPE, e1, 3).await, "déjà inscrit"));
    // Un professeur sans droit d'internat n'inscrit personne ; une famille non plus.
    assert!(est_interdit(&ins(b.prof, PROF, e3, 4).await));
    let (pere, _) = parent(&b, e3, true).await;
    assert!(est_interdit(&ins(pere, PARENT, e3, 4).await));
    // Désinscription datée libère la place.
    let d = b.ent.desinscrire_temps_a(b.etab, b.cpe, CPE, t.id, e2, mi_etude(5)).await.unwrap();
    assert_eq!(d.nb_inscrits, 1);
    assert_eq!(ins(b.cpe, CPE, e3, 6).await.unwrap().nb_inscrits, 2);
    assert!(sqlx::query("DELETE FROM ent_internat_temps_participant WHERE etablissement_id = $1").bind(b.etab).execute(b.ent.pool_verif()).await.is_err());
    // Pas de chevauchement avec une autre étude du même soir (même élève).
    let mut autre = etude(None);
    autre.encadrant_user_id = Some(b.prof);
    autre.debut = paris(avril(15), 19, 30);
    autre.fin = paris(avril(15), 21, 0);
    let t2 = b.ent.creer_temps_a(b.etab, b.cpe, CPE, &autre, t0()).await.unwrap();
    let r = b.ent.inscrire_temps_a(b.etab, b.cpe, CPE, t2.id, &InscriptionTempsDto { eleve_id: e1 }, mi_etude(7)).await;
    assert!(refus_de(&r, "déjà un temps"));
    assert_eq!(b.ent.inscrire_temps_a(b.etab, b.cpe, CPE, t2.id, &InscriptionTempsDto { eleve_id: b.a[3] }, mi_etude(7)).await.unwrap().nb_inscrits, 1);
}

#[tokio::test]
async fn temps_l_eleve_s_inscrit_lui_meme_avant_le_debut() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let u = compte_eleve(&b, e, NaiveDate::from_ymd_opt(2010, 1, 1).unwrap()).await;
    let t = creer_etude(&b, None).await;
    let moi = InscriptionTempsDto { eleve_id: e };
    // Un autre élève n'inscrit pas son camarade ; lui-même, oui (mineur compris), jusqu'au début.
    let autre = b.a[1];
    assert!(est_interdit(&b.ent.inscrire_temps_a(b.etab, u, ELEVE, t.id, &InscriptionTempsDto { eleve_id: autre }, mi_etude(0)).await));
    let r = b.ent.inscrire_temps_a(b.etab, u, ELEVE, t.id, &moi, mi_etude(1)).await.unwrap();
    assert!(r.inscrits.is_empty(), "un élève ne voit jamais la liste des inscrits");
    assert_eq!(r.nb_inscrits, 1);
    assert!(refus_de(&b.ent.desinscrire_temps_a(b.etab, u, ELEVE, t.id, e, paris(avril(15), 19, 5)).await, "closes"));
    // Un élève qui n'est pas interne ce jour-là n'est pas inscrit par le personnel.
    let externe = eleve_fixe(&b.ent, b.etab, "Externe").await;
    assert!(refus_de(&b.ent.inscrire_temps_a(b.etab, b.cpe, CPE, t.id, &InscriptionTempsDto { eleve_id: externe }, mi_etude(2)).await, "interne"));
    // Annulation : les inscrits sont prévenus ; plus d'inscription ; plus de modification.
    let a = b.ent.annuler_temps_a(b.etab, b.cpe, CPE, t.id, mi_etude(10)).await.unwrap();
    assert_eq!(a.statut, "annule");
    assert_eq!(b.evenements("internat.temps_annule").await.len(), 1);
    assert!(refus_de(&b.ent.inscrire_temps_a(b.etab, b.cpe, CPE, t.id, &InscriptionTempsDto { eleve_id: autre }, mi_etude(11)).await, "annulé"));
    assert!(refus_de(&b.ent.annuler_temps_a(b.etab, b.cpe, CPE, t.id, mi_etude(12)).await, "déjà"));
}

#[tokio::test]
async fn temps_creation_bornee_et_sans_sante() {
    let Some(b) = banc().await else { return };
    let mut s = etude(None);
    s.encadrant_user_id = Some(b.prof);
    assert!(est_interdit(&b.ent.creer_temps_a(b.etab, b.prof, PROF, &s, t0()).await), "créer : gestion de l'internat");
    let (pere, _) = parent(&b, b.a[0], true).await;
    let mut p = s.clone();
    p.encadrant_user_id = Some(pere);
    assert!(refus_de(&b.ent.creer_temps_a(b.etab, b.cpe, CPE, &p, t0()).await, "personnel"));
    let mut n = s.clone();
    n.nature = "sortie".into();
    assert!(refus_de(&b.ent.creer_temps_a(b.etab, b.cpe, CPE, &n, t0()).await, "nature"));
    let mut l = s.clone();
    l.libelle = "Étude : suivi du traitement".into();
    assert!(refus_de(&b.ent.creer_temps_a(b.etab, b.cpe, CPE, &l, t0()).await, "santé"));
    let mut long = s.clone();
    long.fin = long.debut + Duration::hours(13);
    assert!(refus_de(&b.ent.creer_temps_a(b.etab, b.cpe, CPE, &long, t0()).await, "horaires"));
    let mut vieux = s.clone();
    vieux.debut = t0() - Duration::days(2);
    vieux.fin = vieux.debut + Duration::hours(1);
    assert!(refus_de(&b.ent.creer_temps_a(b.etab, b.cpe, CPE, &vieux, t0()).await, "rétroactivement"));
    let mut places = s.clone();
    places.places = Some(0);
    assert!(refus_de(&b.ent.creer_temps_a(b.etab, b.cpe, CPE, &places, t0()).await, "places"));
}

#[tokio::test]
async fn temps_absence_a_l_etude_est_un_fait_de_liaison_pas_une_absence_de_cours() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let t = creer_etude(&b, None).await;
    b.ent.inscrire_temps_a(b.etab, b.cpe, CPE, t.id, &InscriptionTempsDto { eleve_id: e1 }, mi_etude(0)).await.unwrap();
    let apres = paris(avril(15), 19, 20);
    // Pas avant le début, pas pour un élève non inscrit, pas par un compte sans droit de liaison.
    assert!(refus_de(&b.ent.absence_etude_a(b.etab, b.cpe, CPE, t.id, &InscriptionTempsDto { eleve_id: e1 }, mi_etude(5)).await, "commencé"));
    assert!(refus_de(&b.ent.absence_etude_a(b.etab, b.cpe, CPE, t.id, &InscriptionTempsDto { eleve_id: e2 }, apres).await, "inscrit"));
    assert!(est_interdit(&b.ent.absence_etude_a(b.etab, b.prof, PROF, t.id, &InscriptionTempsDto { eleve_id: e1 }, apres).await));
    let l = b.ent.absence_etude_a(b.etab, b.cpe, CPE, t.id, &InscriptionTempsDto { eleve_id: e1 }, apres).await.unwrap();
    assert_eq!((l.code.as_str(), l.nuit, l.pour_le_jour), ("absence_etude", avril(15), true));
    let rattache: Option<Uuid> = sqlx::query_scalar("SELECT temps_id FROM ent_internat_liaison WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(l.id)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(rattache, Some(t.id));
    // Ce n'est PAS une absence de cours : aucune absence d'assiduité n'est créée.
    let absences: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(b.etab)
        .bind(e1)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(absences, 0);
}

#[tokio::test]
async fn temps_planning_de_l_eleve_sans_les_autres_r_it32() {
    let Some(b) = banc().await else { return };
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let t = creer_etude(&b, None).await;
    for e in [e1, e2] {
        b.ent.inscrire_temps_a(b.etab, b.cpe, CPE, t.id, &InscriptionTempsDto { eleve_id: e }, mi_etude(0)).await.unwrap();
    }
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e1, e2]).await.id).await;
    // La famille de e1 voit l'étude et la sortie de SON enfant ; rien de e2 ; pas de compteur.
    let p = b.ent.planning_eleve_a(b.etab, p1, PARENT, e1, h(2)).await.unwrap();
    assert_eq!(p.etudes_et_activites.len(), 1);
    assert_eq!(p.sorties_groupe.len(), 1);
    assert!(p.sorties_groupe[0].peut_repondre);
    let json = serde_json::to_string(&p).unwrap();
    assert!(!json.contains(&e2.to_string()));
    assert!(est_interdit(&b.ent.planning_eleve_a(b.etab, p1, PARENT, e2, h(2)).await));
    // Un professeur sans rapport est refusé ; l'internat lit le planning d'un élève de sa portée.
    assert!(est_interdit(&b.ent.planning_eleve_a(b.etab, b.prof, PROF, e1, h(2)).await));
    assert_eq!(b.ent.planning_eleve_a(b.etab, b.cpe, CPE, e2, h(2)).await.unwrap().sorties_groupe.len(), 1);
    let _ = (p2, g);
}

#[tokio::test]
async fn temps_une_sortie_de_groupe_autorisee_empeche_l_inscription_au_meme_moment() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    repondre(&b, p, g.id, e, true).await.unwrap();
    let mut s = etude(None);
    s.encadrant_user_id = Some(b.prof);
    s.debut = paris(avril(16), 15, 0);
    s.fin = paris(avril(16), 16, 0);
    let t = b.ent.creer_temps_a(b.etab, b.cpe, CPE, &s, t0()).await.unwrap();
    let r = b.ent.inscrire_temps_a(b.etab, b.cpe, CPE, t.id, &InscriptionTempsDto { eleve_id: e }, h(3)).await;
    assert!(refus_de(&r, "sortie"));
}

// ───────────────────────── Vigilance et référent (INT-257) ─────────────────────────

#[tokio::test]
async fn vigilance_codee_datee_revue_et_levee_sans_texte() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let saisie = |code: &str, referent: Option<Uuid>| VigilanceSaisieDto { eleve_id: e, code: code.into(), referent_user_id: referent, du: None, au: None };
    // Posée par la gestion, avec un référent du personnel ; refusée à un professeur, à une famille.
    let v = b.ent.poser_vigilance_a(b.etab, b.cpe, CPE, &saisie("premier_mois", Some(b.prof)), t0()).await.unwrap();
    assert!(v.active && !v.a_reviser);
    assert_eq!(v.du, avril(14));
    assert!(est_interdit(&b.ent.poser_vigilance_a(b.etab, b.prof, PROF, &saisie("suivi_rapproche", None), t0()).await));
    assert!(refus_de(&b.ent.poser_vigilance_a(b.etab, b.cpe, CPE, &saisie("surveillance_totale", None), t0()).await, "inconnu"));
    let (pere, _) = parent(&b, e, true).await;
    assert!(refus_de(&b.ent.poser_vigilance_a(b.etab, b.cpe, CPE, &saisie("suivi_rapproche", Some(pere)), t0()).await, "personnel"));
    // Pas de chevauchement du même code ; un autre code, oui.
    assert!(refus_de(&b.ent.poser_vigilance_a(b.etab, b.cpe, CPE, &saisie("premier_mois", None), t0()).await, "déjà en vigueur"));
    b.ent.poser_vigilance_a(b.etab, b.cpe, CPE, &saisie("accompagner_coucher", None), t0()).await.unwrap();
    // Sans revue depuis 8 jours : « à réviser » ; la revue remet le compteur à zéro.
    let tard = t0() + Duration::days(8);
    assert!(b.ent.vigilances_eleve_a(b.etab, b.cpe, CPE, e, tard).await.unwrap().iter().all(|v| v.a_reviser));
    let r = b.ent.reviser_vigilance_a(b.etab, b.cpe, CPE, v.id, tard).await.unwrap();
    assert!(!r.a_reviser);
    // Levée : fin datée, jamais supprimée ; lue de l'équipe seulement.
    let l = b.ent.lever_vigilance_a(b.etab, b.cpe, CPE, v.id, tard).await.unwrap();
    assert_eq!(l.au, Some(avril(22)));
    assert!(refus_de(&b.ent.lever_vigilance_a(b.etab, b.cpe, CPE, v.id, tard + Duration::days(3)).await, "terminée"));
    for (u, r) in [(b.prof, PROF), (pere, PARENT)] {
        assert!(est_interdit(&b.ent.vigilances_eleve_a(b.etab, u, r, e, tard).await));
    }
    assert!(sqlx::query("DELETE FROM ent_internat_vigilance WHERE etablissement_id = $1").bind(b.etab).execute(b.ent.pool_verif()).await.is_err());
    assert!(sqlx::query("UPDATE ent_internat_vigilance SET code = 'suivi_rapproche' WHERE etablissement_id = $1").bind(b.etab).execute(b.ent.pool_verif()).await.is_err());
    let evts = b.evenements("internat.vigilance").await;
    assert!(evts.len() >= 3 && evts.iter().all(|e| e.as_object().unwrap().keys().all(|k| ["code", "du", "au", "acte"].contains(&k.as_str()))));
}
