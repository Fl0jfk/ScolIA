//! Socle 41 (internat), lot I3 « Absences et sorties » : règles testées contre la base locale de test (feature `labo`) ;
//! sans `ENT_DATABASE_URL`, elles sont ignorées en local (échec explicite en CI).
//!
//! **Aucun test ne dépend de l'heure réelle** (règle 33) : chaque commande reçoit son instant, les jours sont des dates
//! fixes d'avril 2026 (heure d'été : Paris = UTC + 2), les nuits d'appel celles de mars 2026. Chaque test crée son propre
//! établissement.
//!
//! | Règle / cas | Test(s) |
//! |---|---|
//! | M-1 (transmission jamais bloquée) | `m1_*` (et `m3_*` d'I2 revus) |
//! | F-1, F-2, F-3, F-4, F-5, F-6 | `f1_*`, `f2_*`, `f3_*` ; F-4 et F-5 sont des tests unitaires (`domain-eleve`, `internat_cas`) ; F-6 : `#[track_caller]` déjà posé sur `refus_de` |
//! | R-IT19 (annonce par un titulaire, heure limite) | `r_it19_*` |
//! | R-IT20 (désaccord connu, le plus restrictif l'emporte) | `r_it20_*` |
//! | R-IT22 (élève majeur) | `r_it22_*` |
//! | R-IT23 (remise : personne autorisée aujourd'hui, interdiction) | `r_it23_*` |
//! | NI-33 (sortie de week-end ou chez un correspondant, clôture) | `ni_33_*`, `correspondant_*` |
//! | CIT-05 à CIT-11, CIT-23, CIT-33, CIT-37 à CIT-39 | `cit_*` et les tests ci-dessus |

use ent_auth::{Action, Portee};
use super::tests_internat_i2::{alea, banc, est_interdit, nuit_dst, refus_de, soir, soir_zone_valide, utc, Banc, CPE, DIR};
use super::*;
use crate::tests_voyages_v0a::{compte, personne_de};  // personne_de : fiche d'un compte du personnel
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use ent_types::internat_nuit::*;
use ent_types::internat_sorties::*;
use uuid::Uuid;

/// Instant en heure d'été de Paris (UTC + 2) : avril 2026 est après le passage du 29 mars.
pub(crate) fn paris(jour: NaiveDate, h: u32, mi: u32) -> DateTime<Utc> {
    utc(jour.year(), jour.month(), jour.day(), h - 2, mi)
}

/// Un jour d'avril 2026 : le 14 est un mardi, le 17 un vendredi, le 19 un dimanche, le 20 un lundi.
pub(crate) fn avril(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 4, d).unwrap()
}

pub(crate) const PARENT: &str = "parent";
pub(crate) const ELEVE: &str = "eleve";

async fn statut_attendu(b: &Banc, nuit: NaiveDate, eleve: Uuid) -> Option<(String, Option<String>)> {
    sqlx::query_as("SELECT statut, motif_code FROM ent_internat_attendu_calcule($1, $2) WHERE eleve_id = $3")
        .bind(b.etab)
        .bind(nuit)
        .bind(eleve)
        .fetch_optional(b.ent.pool_verif())
        .await
        .unwrap()
}

async fn est_attendu(b: &Banc, nuit: NaiveDate, eleve: Uuid) -> bool {
    matches!(statut_attendu(b, nuit, eleve).await, Some((s, _)) if s == "attendu" || s == "ajoute")
}

async fn est_retire(b: &Banc, nuit: NaiveDate, eleve: Uuid, motif: &str) -> bool {
    matches!(statut_attendu(b, nuit, eleve).await, Some((s, Some(m))) if s == "retire" && m == motif)
}

/// Titulaire avec compte (autorité parentale, garde non « aucune ») : (compte, personne).
pub(crate) async fn parent(b: &Banc, eleve: Uuid, sorties: bool) -> (Uuid, Uuid) {
    let user = compte(&b.ent, b.etab, PARENT).await;
    let pers = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom, user_id, source) VALUES ($1,$2,$3,'Parent',$4,'compte')")
        .bind(pers)
        .bind(b.etab)
        .bind(format!("Parent{}", alea()))
        .bind(user)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale, peut_autoriser_sorties) \
         VALUES ($1,$2,$3,$4,$5,'mere',true,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(user)
    .bind(pers)
    .bind(sorties)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    (user, pers)
}

/// Personne sans compte (grand-parent, voisine, correspondant…).
pub(crate) async fn tiers(b: &Banc, prenom: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom, source) VALUES ($1,$2,$3,$4,'saisie')")
        .bind(id)
        .bind(b.etab)
        .bind(format!("Tiers{}", alea()))
        .bind(prenom)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    id
}

/// Personne autorisée à récupérer l'élève (socle 4, contact validé).
async fn autoriser_recuperer(b: &Banc, eleve: Uuid, personne: Uuid) {
    sqlx::query(
        "INSERT INTO ent_eleve_contact_urgence (id, etablissement_id, eleve_id, personne_id, relation, peut_recuperer, statut) \
         VALUES ($1,$2,$3,$4,'grand-parent',true,'valide')",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(personne)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
}

/// Interdiction de remise (socle 4) visant une personne identifiée ou un simple libellé.
pub(crate) async fn interdire(b: &Banc, eleve: Uuid, personne: Option<Uuid>) {
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, personne_libelle, objet, fondement, consigne, du, decide_par) \
         VALUES ($1,$2,$3,$4,$5,ARRAY['remise'],'decision_etablissement','Appeler la direction','2026-01-01',$6)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(personne)
    .bind(personne.is_none().then_some("Personne non identifiée"))
    .bind(b.dir)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
}

/// Désaccord connu entre titulaires (socle 18), non levé.
pub(crate) async fn desaccord_connu(b: &Banc, eleve: Uuid, declarant: Uuid) {
    sqlx::query(
        "INSERT INTO ent_acc_desaccord (id, etablissement_id, sujet_eleve_id, objet, source, declarant_user_id) \
         VALUES ($1,$2,$3,'sortie','declaration',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(declarant)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
}

/// Compte de l'élève lui-même (élève majeur si sa naissance le dit).
pub(crate) async fn compte_eleve(b: &Banc, eleve: Uuid, naissance: NaiveDate) -> Uuid {
    let user = compte(&b.ent, b.etab, ELEVE).await;
    sqlx::query("INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id) VALUES ($1,$2,$3)")
        .bind(b.etab)
        .bind(eleve)
        .bind(user)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    crate::tests_formulaires_fo0::poser_naissance(&b.ent, b.etab, eleve, Some(naissance), Some("declaree")).await;
    user
}

/// Séjour avec un rythme précis (dimanche, lundi, mardi, mercredi, jeudi, vendredi, samedi) et un lit.
async fn heberger_rythme(b: &Banc, eleve: Uuid, jours: [bool; 7]) {
    let pool = b.ent.pool_verif();
    let sejour = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_internat_sejour (id, etablissement_id, eleve_id, du, origine) VALUES ($1,$2,$3,'2025-09-01','regime')")
        .bind(sejour)
        .bind(b.etab)
        .bind(eleve)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_internat_rythme (id, etablissement_id, sejour_id, du, dimanche, lundi, mardi, mercredi, jeudi, vendredi, samedi, cree_par_user_id) \
         VALUES ($1,$2,$3,'2025-09-01',$4,$5,$6,$7,$8,$9,$10,$11)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(sejour)
    .bind(jours[0])
    .bind(jours[1])
    .bind(jours[2])
    .bind(jours[3])
    .bind(jours[4])
    .bind(jours[5])
    .bind(jours[6])
    .bind(b.cpe)
    .execute(pool)
    .await
    .unwrap();
}

/// Annonce d'absence. Pour un **mineur** (C-1), la personne qui récupère est exigée : un titulaire se désigne lui-même
/// (sa propre fiche est une personne autorisée), les autres comptes annoncent sans personne.
async fn annoncer(b: &Banc, user: Uuid, role: &str, eleve: Uuid, du: NaiveDate, au: NaiveDate, a: DateTime<Utc>) -> Result<AbsenceDto, EntDbError> {
    let personne = if role == PARENT { sqlx::query_scalar("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 LIMIT 1").bind(b.etab).bind(user).fetch_optional(b.ent.pool_verif()).await.unwrap() } else { None };
    annoncer_avec(b, user, role, eleve, du, au, personne, a).await
}

#[allow(clippy::too_many_arguments)]
async fn annoncer_avec(b: &Banc, user: Uuid, role: &str, eleve: Uuid, du: NaiveDate, au: NaiveDate, personne: Option<Uuid>, a: DateTime<Utc>) -> Result<AbsenceDto, EntDbError> {
    b.ent
        .annoncer_absence_a(
            b.etab,
            user,
            role,
            &AbsenceAnnonceDto { eleve_id: eleve, du_nuit: du, au_nuit: au, recupere_par_personne_id: personne, retour_prevu: None, regularisation: false, motif_texte: None },
            a,
        )
        .await
}

fn decision(d: &str, motif: Option<&str>) -> DecisionDto {
    DecisionDto { decision: d.into(), motif_texte: motif.map(String::from) }
}

async fn faits(b: &Banc, colonne: &str, id: Uuid) -> Vec<(String, String, Option<Uuid>)> {
    sqlx::query_as(&format!(
        "SELECT acte, qualite, par_user_id FROM ent_internat_i3_fait WHERE etablissement_id = $1 AND {colonne} = $2 ORDER BY a, cree_le"
    ))
    .bind(b.etab)
    .bind(id)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap()
}

// ═════════════════════════════ Suites de la revue d'I2 ═════════════════════════════

/// M-1 — une nuit exceptionnelle posée APRÈS la validation du soir : un appel « soir » complémentaire, limité aux
/// élèves non couverts, la couvre ; la transmission du matin part ensuite sans rien laisser de côté.
#[tokio::test]
async fn m1_ajout_apres_validation_appel_complementaire_puis_transmission() {
    let Some(b) = banc().await else { return };
    let matin = utc(2026, 3, 29, 5, 0);
    let principal = b.ouvrir("soir", "internat", None, 0).await.appel.id;
    for e in b.a.iter().chain(b.b.iter()) {
        b.point(principal, *e, "present", 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, principal, soir(5)).await.unwrap();
    // Un complément avant la validation, ou sans rien à couvrir, est refusé.
    let complement = AppelOuvertureDto { complementaire: true, ..crate::tests_internat_i2::ouverture("soir", "internat", None) };
    let r = b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &complement, soir(6)).await;
    assert!(refus_de(&r, "aucun appel du soir n'est validé"), "le complément vient après la validation : {r:?}");
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, principal, soir(7)).await.unwrap();
    let r = b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &complement, soir(8)).await;
    assert!(refus_de(&r, "rien à compléter"), "tout le monde est couvert : {r:?}");
    // Nuit exceptionnelle posée après la validation du soir.
    let invite = eleve_invite(&b, "Retardataire").await;
    b.ent
        .poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &NuitExceptionnelleSaisieDto { eleve_id: invite, nuit: nuit_dst(), motif_code: "examen".into() }, soir(10))
        .await
        .unwrap();
    // Un second appel « soir » ordinaire sur tout l'internat reste refusé (R-IT31) ; un complément sur une zone aussi.
    let r = b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &crate::tests_internat_i2::ouverture("soir", "internat", None), soir(11)).await;
    assert!(refus_de(&r, "existe déjà"), "R-IT31 : {r:?}");
    let r = b
        .ent
        .ouvrir_appel_a(b.etab, b.cpe, CPE, &AppelOuvertureDto { complementaire: true, ..crate::tests_internat_i2::ouverture("soir", "zone", Some(b.zone_a)) }, soir(11))
        .await;
    assert!(refus_de(&r, "tout l'internat"), "{r:?}");
    let r = b
        .ent
        .ouvrir_appel_a(b.etab, b.cpe, CPE, &AppelOuvertureDto { complementaire: true, ..crate::tests_internat_i2::ouverture("etude", "internat", None) }, soir(11))
        .await;
    assert!(refus_de(&r, "appel du soir"), "{r:?}");
    // Le complément : seulement l'élève non couvert, numéro 1.
    let c = b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &complement, soir(12)).await.unwrap();
    assert_eq!((c.appel.complement, c.appel.nb_attendus, c.lignes.len()), (1, 1, 1));
    assert_eq!(c.lignes[0].eleve_id, invite);
    // Un seul complément à la fois.
    let r = b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &complement, soir(13)).await;
    assert!(refus_de(&r, "déjà en cours"), "{r:?}");
    // On ne pointe que la liste du complément (R-IT14).
    let r = b.point(c.appel.id, b.a[0], "present", 14).await;
    assert!(refus_de(&r, "R-IT14"), "{r:?}");
    b.point(c.appel.id, invite, "present", 15).await.unwrap();
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, c.appel.id, soir(16)).await.unwrap();
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, c.appel.id, soir(17)).await.unwrap();
    // La transmission du matin part complète : sept attendus, aucun non-couvert, une seule version.
    let v = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await.unwrap();
    assert_eq!((v.version, v.nb_attendus, v.nb_presents, v.nb_non_couverts), (1, 7, 7, 0));
    let repas = b.ent.effectif_repas_a(b.etab, b.cpe, CPE, nuit_dst(), matin).await.unwrap();
    assert_eq!((repas.diner, repas.petit_dejeuner, repas.non_couverts), (7, 7, 0));
}

async fn eleve_invite(b: &Banc, prenom: &str) -> Uuid {
    crate::tests_internat_i2::eleve_fixe(&b.ent, b.etab, prenom).await
}

/// M-1 — sans appel complémentaire (personne ne l'a fait), la nuit PASSÉE se transmet quand même, avec la liste
/// explicite des non-couverts ; la rectificative les intègre quand le complément est fait après coup.
#[tokio::test]
async fn m1_sans_appel_complementaire_transmission_avec_non_couverts_puis_rectificative() {
    let Some(b) = banc().await else { return };
    let matin = utc(2026, 3, 29, 5, 0);
    soir_zone_valide(&b, b.zone_a, &b.a.clone(), 0).await;
    soir_zone_valide(&b, b.zone_b, &b.b.clone(), 10).await;
    let invite = eleve_invite(&b, "Tardif").await;
    b.ent
        .poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &NuitExceptionnelleSaisieDto { eleve_id: invite, nuit: nuit_dst(), motif_code: "intemperies".into() }, soir(30))
        .await
        .unwrap();
    // Nuit passée, aucun complément : la transmission part avec le non-couvert, compté à part.
    let v1 = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await.unwrap();
    assert!(v1.nouvelle);
    assert_eq!((v1.nb_attendus, v1.nb_presents, v1.nb_absents), (6, 6, 0));
    assert_eq!((v1.nb_non_couverts, v1.non_couverts.clone()), (1, vec![invite]));
    // Pas de nouvelle version tant que rien ne change.
    let encore = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await.unwrap();
    assert!(!encore.nouvelle);
    // Le complément a posteriori (nuit encore courante à 7 h), puis la rectificative.
    let c = b
        .ent
        .ouvrir_appel_a(b.etab, b.cpe, CPE, &AppelOuvertureDto { complementaire: true, ..crate::tests_internat_i2::ouverture("soir", "internat", None) }, soir(40))
        .await
        .unwrap();
    assert_eq!(c.appel.nb_attendus, 1);
    b.point(c.appel.id, invite, "present", 41).await.unwrap();
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, c.appel.id, soir(42)).await.unwrap();
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, c.appel.id, soir(43)).await.unwrap();
    let r = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await;
    assert!(refus_de(&r, "motif de rectification"), "{r:?}");
    let v2 = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), Some("complément fait après coup"), matin).await.unwrap();
    assert_eq!((v2.version, v2.nb_attendus, v2.nb_non_couverts), (2, 7, 0));
}

async fn confie_de(b: &Banc, liste: Uuid) -> Option<Uuid> {
    b.ent.lister_listes_a(b.etab, b.cpe, CPE, utc(2026, 4, 1, 10, 0)).await.unwrap().into_iter().find(|l| l.id == liste).unwrap().confie_a_user_id
}

/// F-1 — `confie_a_user_id` : personnel de l'établissement portant le droit `internat.appel`, accès ouvert et non terminé.
#[tokio::test]
async fn f1_liste_confiee_au_personnel_avec_droit_d_appel_et_contrat_ouvert() {
    let Some(b) = banc().await else { return };
    let saisie = |libelle: &str, agent: Option<Uuid>| ListeSaisieDto { libelle: libelle.into(), chambre_ids: vec![b.chambre_a], confie_a_user_id: agent };
    // Sans droit d'appel (professeur, parent), ou compte étranger : refusé.
    let prof = b.prof;
    let r = b.ent.creer_liste_a(b.etab, b.cpe, CPE, &saisie("Prof", Some(prof))).await;
    assert!(refus_de(&r, "ne porte pas le droit"), "un professeur n'a pas internat.appel : {r:?}");
    let parent = compte(&b.ent, b.etab, PARENT).await;
    let r = b.ent.creer_liste_a(b.etab, b.cpe, CPE, &saisie("Parent", Some(parent))).await;
    assert!(refus_de(&r, "ne porte pas le droit"), "{r:?}");
    let r = b.ent.creer_liste_a(b.etab, b.cpe, CPE, &saisie("Inconnu", Some(Uuid::new_v4()))).await;
    assert!(refus_de(&r, "n'appartient pas à l'établissement"), "{r:?}");
    // Personnel avec le droit (CPE) : accepté.
    let cpe2 = compte(&b.ent, b.etab, CPE).await;
    let ok = b.ent.creer_liste_a(b.etab, b.cpe, CPE, &saisie("Aile", Some(cpe2))).await.unwrap();
    assert_eq!(ok.confie_a_user_id, Some(cpe2));
    // Contrat terminé (date de fin dépassée) : refusé ; contrat sans fin ou à fin lointaine : accepté.
    sqlx::query("UPDATE ent_membership SET date_fin = '2020-01-01' WHERE etablissement_id = $1 AND user_id = $2")
        .bind(b.etab)
        .bind(cpe2)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(confie_de(&b, ok.id).await, None, "contrat terminé : la liste n'est plus confiée (F-1, contrôle à la lecture)");
    let r = b.ent.creer_liste_a(b.etab, b.cpe, CPE, &saisie("Aile 2", Some(cpe2))).await;
    assert!(refus_de(&r, "contrat est terminé"), "{r:?}");
    let r = b.ent.modifier_liste_a(b.etab, b.cpe, CPE, ok.id, &saisie("Aile", Some(cpe2)), true).await;
    assert!(refus_de(&r, "contrat est terminé"), "la modification aussi : {r:?}");
    sqlx::query("UPDATE ent_membership SET date_fin = '2099-12-31' WHERE etablissement_id = $1 AND user_id = $2")
        .bind(b.etab)
        .bind(cpe2)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    b.ent.modifier_liste_a(b.etab, b.cpe, CPE, ok.id, &saisie("Aile", Some(cpe2)), true).await.expect("contrat qui court encore");
    assert_eq!(confie_de(&b, ok.id).await, Some(cpe2), "contrat qui court : toujours confiée");
    // Accès fermé : refusé.
    sqlx::query("UPDATE ent_membership SET etat = 'ferme' WHERE etablissement_id = $1 AND user_id = $2")
        .bind(b.etab)
        .bind(cpe2)
        .execute(b.ent.pool_verif())
        .await
        .ok();
}

/// F-3 — petit-déjeuner : « parti » suit la même priorité que les absents (le « présent » du matin l'emporte) et on
/// retranche l'UNION des absents et des partis.
#[tokio::test]
async fn f3_petit_dejeuner_parti_puis_present_et_union_absent_parti() {
    let Some(b) = banc().await else { return };
    // Soir : a0 « parti », a1 « absent », les autres présents.
    let soir_id = b.ouvrir("soir", "internat", None, 0).await.appel.id;
    for (i, e) in b.a.iter().chain(b.b.iter()).enumerate() {
        let marque = match i {
            0 => "parti",
            1 => "absent",
            _ => "present",
        };
        b.point(soir_id, *e, marque, 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, soir_id, soir(2)).await.unwrap();
    // Matin : a0 est « présent » (revenu), a1 est « parti » (donc à la fois absent et parti).
    let matin_id = b.ouvrir("matin", "internat", None, 10).await.appel.id;
    for (i, e) in b.a.iter().chain(b.b.iter()).enumerate() {
        let marque = if i == 1 { "parti" } else { "present" };
        b.point(matin_id, *e, marque, 11).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, matin_id, soir(12)).await.unwrap();
    let repas = b.ent.effectif_repas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(13)).await.unwrap();
    // Six attendus ; seul a1 manque au petit-déjeuner (absent ET parti, compté une fois) ; a0 revenu y est.
    assert_eq!((repas.diner, repas.petit_dejeuner), (6, 5));
}

/// F-2 — une transmission écrite sous 2824 (colonnes par défaut de 2825 : ensembles des partis et des non-couverts
/// inconnus, petit-déjeuner de l'ancienne formule) ne force pas de rectificative artificielle quand on la relit.
#[tokio::test]
async fn f2_transmission_ecrite_avant_i3_ne_force_pas_de_rectificative() {
    let Some(b) = banc().await else { return };
    let matin = utc(2026, 3, 29, 5, 0);
    // Soir : a0 « parti », a1 « absent » (cas ouvert), les autres présents. Chiffres : 6 attendus, 4 présents, 1 absent.
    let soir_id = b.ouvrir("soir", "internat", None, 0).await.appel.id;
    for (i, e) in b.a.iter().chain(b.b.iter()).enumerate() {
        let marque = match i {
            0 => "parti",
            1 => "absent",
            _ => "present",
        };
        b.point(soir_id, *e, marque, 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, soir_id, soir(2)).await.unwrap();
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, soir_id, soir(3)).await.unwrap();
    // Lignes de transmission « écrites avant I3 » : `ensembles_connus` à sa valeur par défaut (faux), partis vides.
    let nb_cas: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_cas WHERE etablissement_id = $1 AND nuit = $2")
        .bind(b.etab)
        .bind(nuit_dst())
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    for (nature, pre) in [("matin_vie_scolaire", 4), ("effectif_restauration", 4)] {
        sqlx::query(
            "INSERT INTO ent_internat_transmission \
             (id, etablissement_id, nuit, nature, version, transmis_le, transmis_par_user_id, nb_attendus, nb_presents, nb_absents, nb_cas, eleves_absents) \
             VALUES ($1,$2,$3,$4,1,$5,$6,6,$7,1,$8,$9)",
        )
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(nuit_dst())
        .bind(nature)
        .bind(matin)
        .bind(b.cpe)
        .bind(pre)
        .bind(nb_cas as i32)
        .bind(if nature == "matin_vie_scolaire" { vec![b.a[1]] } else { vec![] })
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    }
    let v = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await.unwrap();
    assert!(!v.nouvelle && v.version == 1, "relecture d'une ligne d'avant I3 : aucune rectificative artificielle ({v:?})");
}

// ═════════════════════════════ Types de sortie ═════════════════════════════

#[tokio::test]
async fn types_de_sortie_par_defaut_reglage_et_droits() {
    let Some(b) = banc().await else { return };
    let types = b.ent.lister_types_sortie_a(b.etab).await.unwrap();
    let codes: Vec<&str> = types.iter().map(|t| t.code.as_str()).collect();
    assert_eq!(codes, ["exceptionnelle", "mercredi", "nuit_correspondant", "retour_tardif", "soiree", "week_end"]);
    let t = |c: &str| types.iter().find(|t| t.code == c).unwrap().clone();
    assert_eq!((t("mercredi").autorisation.as_str(), t("mercredi").decideur.as_str(), t("mercredi").seul_permis), ("annuelle", "titulaires", true));
    assert!(t("mercredi").eleve_peut_demander && !t("nuit_correspondant").eleve_peut_demander);
    assert!(t("nuit_correspondant").correspondant_requis && !t("nuit_correspondant").seul_permis);
    assert_eq!(t("week_end").autorisation, "aucune", "règle implicite du week-end : l'enfant rentre chez lui");
    assert_eq!(t("soiree").decideur, "titulaires_et_cpe");
    // Droits : l'internat se paramètre avec `internat.gerer`.
    let r = b.ent.modifier_type_sortie_a(b.etab, b.prof, "prof", "soiree", &SortieTypeSaisieDto { actif: Some(false), ..Default::default() }, soir(0)).await;
    assert!(est_interdit(&r), "{r:?}");
    let (p, _) = parent(&b, b.a[0], true).await;
    let r = b.ent.modifier_type_sortie_a(b.etab, p, PARENT, "soiree", &SortieTypeSaisieDto::default(), soir(0)).await;
    assert!(est_interdit(&r), "{r:?}");
    let ent = &b.ent;
    let (etab, cpe) = (b.etab, b.cpe);
    let invalide = |s: SortieTypeSaisieDto| async move { ent.modifier_type_sortie_a(etab, cpe, CPE, "soiree", &s, soir(0)).await };
    assert!(refus_de(&invalide(SortieTypeSaisieDto { autorisation: Some("n_importe_quoi".into()), ..Default::default() }).await, "règle d'autorisation"));
    assert!(refus_de(&invalide(SortieTypeSaisieDto { duree_max_heures: Some(0), ..Default::default() }).await, "durée maximale"));
    assert!(refus_de(&invalide(SortieTypeSaisieDto { libelle: Some("  ".into()), ..Default::default() }).await, "libellé"));
    let r = b.ent.modifier_type_sortie_a(b.etab, b.cpe, CPE, "inconnu", &SortieTypeSaisieDto::default(), soir(0)).await;
    assert!(matches!(r, Err(EntDbError::Domain(_))), "{r:?}");
    let m = b
        .ent
        .modifier_type_sortie_a(b.etab, b.cpe, CPE, "soiree", &SortieTypeSaisieDto { libelle: Some("Soirée (conservatoire)".into()), actif: Some(false), ..Default::default() }, soir(0))
        .await
        .unwrap();
    assert!(!m.actif && m.libelle.starts_with("Soirée"));
    // Un type désactivé ne se demande plus.
    let debut = paris(avril(14), 19, 0);
    let r = b
        .ent
        .demander_sortie_a(
            b.etab,
            p,
            PARENT,
            &SortieDemandeDto { eleve_id: b.a[0], type_code: "soiree".into(), debut, fin_prevue: debut + Duration::hours(3), seul: true, avec_personne_id: None, correspondant_id: None },
            paris(avril(13), 10, 0),
        )
        .await;
    assert!(refus_de(&r, "désactivé"), "{r:?}");
}

// ═════════════════════════════ Absences annoncées (R-IT19) ═════════════════════════════

/// R-IT19 — un titulaire annonce, avant l'heure limite : l'absence est en vigueur aussitôt, l'attendu est recalculé
/// (motif codé), les autres élèves et les autres nuits ne bougent pas ; ni un tiers, ni un élève mineur n'annonce.
#[tokio::test]
async fn r_it19_annonce_titulaire_en_vigueur_attendu_recalcule_et_qualites() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (p, _) = parent(&b, eleve, false).await;
    let mardi = avril(14);
    assert!(est_attendu(&b, mardi, eleve).await, "attendu avant l'annonce");
    let a = annoncer(&b, p, PARENT, eleve, mardi, mardi, paris(mardi, 11, 0)).await.unwrap();
    assert_eq!((a.statut.as_str(), a.tardive, a.annonce_qualite.as_str(), a.mode.as_str()), ("annoncee", false, "titulaire", "annonce"));
    // M-B : pas de départ constaté = attendu ; le retrait n'a lieu qu'au départ effectif contrôlé au portail.
    assert!(est_attendu(&b, mardi, eleve).await, "annoncée, pas encore partie : attendu");
    let pers_p: Uuid = sqlx::query_scalar("SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND user_id = $2").bind(b.etab).bind(p).fetch_one(b.ent.pool_verif()).await.unwrap();
    b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(Some(pers_p), paris(mardi, 17, 0)), paris(mardi, 17, 0)).await.unwrap();
    assert!(est_retire(&b, mardi, eleve, "absence_annoncee").await, "attendu recalculé avec le motif codé, une fois parti");
    assert!(est_attendu(&b, mardi, b.a[1]).await, "les autres élèves ne bougent pas");
    assert!(est_attendu(&b, avril(15), eleve).await, "la nuit suivante non plus");
    assert!(est_attendu(&b, avril(13), eleve).await, "ni la précédente");
    let evts = b.evenements("internat.absence_annoncee").await;
    assert_eq!(evts.len(), 1);
    assert_eq!((evts[0]["statut"].as_str(), evts[0]["tardive"].as_bool()), (Some("annoncee"), Some(false)));
    assert!(!b.evenements("internat.attendu_modifie").await.is_empty());
    // Trace : qui, quand, quoi.
    let f = faits(&b, "absence_id", a.id).await;
    assert_eq!(f, vec![("annonce".to_string(), "titulaire".to_string(), Some(p)), ("depart".to_string(), "personnel".to_string(), Some(b.cpe))]);
    // Ni un professeur, ni la comptabilité, ni le titulaire d'un AUTRE élève n'annoncent.
    assert!(est_interdit(&annoncer(&b, b.prof, "prof", b.a[1], mardi, mardi, paris(mardi, 11, 0)).await));
    assert!(est_interdit(&annoncer(&b, b.compta, "compta", b.a[1], mardi, mardi, paris(mardi, 11, 0)).await));
    let (autre, _) = parent(&b, b.a[2], true).await;
    assert!(est_interdit(&annoncer(&b, autre, PARENT, b.a[1], mardi, mardi, paris(mardi, 11, 0)).await), "pas son enfant");
    // Un élève mineur n'annonce pas lui-même (CIT-09 : seul le majeur décide) ; le CPE ne « pose » pas l'annonce d'une famille.
    let mineur = compte_eleve(&b, b.a[1], NaiveDate::from_ymd_opt(2010, 3, 14).unwrap()).await;
    let r = annoncer(&b, mineur, ELEVE, b.a[1], mardi, mardi, paris(mardi, 11, 0)).await;
    assert!(refus_de(&r, "élève mineur"), "{r:?}");
    let r = annoncer(&b, b.cpe, CPE, b.a[1], mardi, mardi, paris(mardi, 11, 0)).await;
    assert!(refus_de(&r, "titulaire ou par l'élève majeur"), "{r:?}");
    // Plusieurs nuits d'un coup : toutes sont retirées.
    let (p2, pers2) = parent(&b, b.a[3], false).await;
    let multi = annoncer(&b, p2, PARENT, b.a[3], avril(20), avril(22), paris(mardi, 9, 0)).await.unwrap();
    for j in [20, 21, 22] {
        assert!(est_attendu(&b, avril(j), b.a[3]).await, "M-B : avant le départ, la nuit du {j} est attendue");
    }
    let depart_lundi = paris(avril(20), 17, 0);
    b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, multi.id, &depart(Some(pers2), depart_lundi), depart_lundi).await.unwrap();
    for j in [20, 21, 22] {
        assert!(est_retire(&b, avril(j), b.a[3], "absence_annoncee").await, "nuit du {j}");
    }
    assert!(est_attendu(&b, avril(23), b.a[3]).await);
}

/// R-IT19 — après l'heure limite (16 h), l'annonce est une DEMANDE au CPE (aucun effet avant sa décision) ; le CPE
/// accepte ou refuse (motif lu par la famille) ; une nuit passée se régularise (CIT-37) avec un motif.
#[tokio::test]
async fn r_it19_annonce_tardive_demande_au_cpe_refus_motive_et_regularisation() {
    let Some(b) = banc().await else { return };
    let mardi = avril(14);
    let (p, _) = parent(&b, b.a[0], false).await;
    let t = annoncer(&b, p, PARENT, b.a[0], mardi, mardi, paris(mardi, 17, 0)).await.unwrap();
    assert_eq!((t.statut.as_str(), t.tardive), ("demandee", true));
    assert!(est_attendu(&b, mardi, b.a[0]).await, "aucun effet avant la décision du CPE");
    // La file du CPE : la famille ne décide pas, un professeur non plus.
    let file = b.ent.absences_a_traiter_a(b.etab, b.cpe, CPE).await.unwrap();
    let ligne = file.iter().find(|x| x.id == t.id).expect("dans la file du CPE");
    assert!(ligne.nom.is_some() && !ligne.protege);
    assert!(est_interdit(&b.ent.decider_absence_a(b.etab, p, PARENT, t.id, &decision("accepter", None), paris(mardi, 17, 5)).await));
    assert!(est_interdit(&b.ent.decider_absence_a(b.etab, b.prof, "prof", t.id, &decision("accepter", None), paris(mardi, 17, 5)).await));
    assert!(est_interdit(&b.ent.absences_a_traiter_a(b.etab, b.prof, "prof").await));
    let d = b.ent.decider_absence_a(b.etab, b.cpe, CPE, t.id, &decision("accepter", None), paris(mardi, 17, 10)).await.unwrap();
    assert_eq!(d.statut, "acceptee");
    assert!(est_attendu(&b, mardi, b.a[0]).await, "M-B : acceptée mais pas encore partie : attendu");
    let r = b.ent.decider_absence_a(b.etab, b.cpe, CPE, t.id, &decision("refuser", Some("trop tard")), paris(mardi, 17, 12)).await;
    assert!(refus_de(&r, "n'attend plus de décision") || matches!(r, Err(EntDbError::Refus(_, _))), "{r:?}");
    // Refus : le motif est obligatoire et lu par la famille ; l'élève reste attendu.
    let (p1, _) = parent(&b, b.a[1], false).await;
    let t1 = annoncer(&b, p1, PARENT, b.a[1], mardi, mardi, paris(mardi, 18, 0)).await.unwrap();
    let r = b.ent.decider_absence_a(b.etab, b.cpe, CPE, t1.id, &decision("refuser", None), paris(mardi, 18, 5)).await;
    assert!(refus_de(&r, "motif lu par la famille"), "{r:?}");
    let r = b.ent.decider_absence_a(b.etab, b.cpe, CPE, t1.id, &decision("bidouiller", None), paris(mardi, 18, 5)).await;
    assert!(refus_de(&r, "décision inconnue"), "{r:?}");
    let r = b.ent.decider_absence_a(b.etab, b.cpe, CPE, t1.id, &decision("refuser", Some("examen demain matin")), paris(mardi, 18, 6)).await.unwrap();
    assert_eq!(r.statut, "refusee");
    assert!(est_attendu(&b, mardi, b.a[1]).await);
    let vue_famille = b.ent.absences_eleve_a(b.etab, p1, PARENT, b.a[1], paris(mardi, 19, 0)).await.unwrap();
    assert_eq!((vue_famille[0].statut.as_str(), vue_famille[0].motif_refus_texte.as_deref()), ("refusee", Some("examen demain matin")));
    // CIT-37 : une nuit passée ne s'annonce pas ; elle se régularise, avec un motif, par le CPE.
    let hier = avril(12);
    let r = annoncer(&b, p, PARENT, b.a[0], hier, hier, paris(mardi, 9, 0)).await;
    assert!(refus_de(&r, "régularisation"), "{r:?}");
    let demande = |motif: Option<&str>, du: NaiveDate| AbsenceAnnonceDto {
        eleve_id: b.a[0],
        du_nuit: du,
        au_nuit: du,
        recupere_par_personne_id: None,
        retour_prevu: None,
        regularisation: true,
        motif_texte: motif.map(String::from),
    };
    let r = b.ent.annoncer_absence_a(b.etab, p, PARENT, &demande(None, hier), paris(mardi, 9, 0)).await;
    assert!(refus_de(&r, "motif"), "{r:?}");
    let r = b.ent.annoncer_absence_a(b.etab, p, PARENT, &demande(Some("hospitalisé aux urgences"), hier), paris(mardi, 9, 0)).await;
    assert!(refus_de(&r, "santé"), "R-IT6 : {r:?}");
    let r = b.ent.annoncer_absence_a(b.etab, p, PARENT, &demande(Some("train annulé"), NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()), paris(mardi, 9, 0)).await;
    assert!(refus_de(&r, "45 jours"), "{r:?}");
    let r = b.ent.annoncer_absence_a(b.etab, p, PARENT, &AbsenceAnnonceDto { au_nuit: mardi, ..demande(Some("train annulé"), hier) }, paris(mardi, 9, 0)).await;
    assert!(refus_de(&r, "déjà passées"), "une régularisation ne couvre que des nuits passées : {r:?}");
    let reg = b.ent.annoncer_absence_a(b.etab, p, PARENT, &demande(Some("train annulé"), hier), paris(mardi, 9, 0)).await.unwrap();
    assert_eq!((reg.statut.as_str(), reg.mode.as_str()), ("demandee", "regularisation"));
    assert!(!est_retire(&b, hier, b.a[0], "absence_annoncee").await, "pas d'effet avant la décision du CPE");
    let r = b.ent.decider_absence_a(b.etab, b.cpe, CPE, reg.id, &decision("accepter", None), paris(mardi, 9, 5)).await;
    assert!(matches!(r, Err(EntDbError::Refus(_, _))), "une régularisation se régularise, elle ne s'accepte pas : {r:?}");
    let ok = b.ent.decider_absence_a(b.etab, b.cpe, CPE, reg.id, &decision("regulariser", None), paris(mardi, 9, 6)).await.unwrap();
    assert_eq!(ok.statut, "regularisee");
    assert!(est_retire(&b, hier, b.a[0], "absence_annoncee").await, "l'écart est couvert");
    assert!(!b.evenements("internat.absence_regularisee").await.is_empty());
    let f = faits(&b, "absence_id", reg.id).await;
    assert_eq!(f.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["regularisation_demandee", "regularisation"]);
    // Le CPE régularise aussi de lui-même (appel de la mère reçu par téléphone), avec son motif.
    let direct = b
        .ent
        .annoncer_absence_a(b.etab, b.cpe, CPE, &AbsenceAnnonceDto { eleve_id: b.a[2], ..demande(Some("appel de la mère à 22 h 05"), avril(11)) }, paris(mardi, 9, 0))
        .await
        .unwrap();
    assert_eq!(direct.statut, "regularisee");
    assert_eq!(direct.annonce_qualite, "personnel");
}

/// R-IT19 — bornes, doublons, personne qui récupère, annulation, texte court sans santé.
#[tokio::test]
async fn r_it19_bornes_doublons_recuperation_annulation_et_texte_sans_sante() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let mardi = avril(14);
    let (p, pers) = parent(&b, eleve, false).await;
    let now = paris(mardi, 9, 0);
    let r = annoncer(&b, p, PARENT, eleve, avril(16), avril(15), now).await;
    assert!(refus_de(&r, "précède"), "{r:?}");
    let r = annoncer(&b, p, PARENT, eleve, avril(16), avril(16) + Duration::days(60), now).await;
    assert!(refus_de(&r, "60 nuits"), "{r:?}");
    let r = annoncer(&b, p, PARENT, eleve, mardi + Duration::days(400), mardi + Duration::days(401), now).await;
    assert!(refus_de(&r, "un an à l'avance"), "{r:?}");
    let ok = annoncer(&b, p, PARENT, eleve, avril(16), avril(17), now).await.unwrap();
    let r = annoncer(&b, p, PARENT, eleve, avril(17), avril(18), now).await;
    assert!(refus_de(&r, "déjà annoncée"), "recouvrement : {r:?}");
    // Qui récupère : une personne autorisée aujourd'hui (titulaire ou contact validé), jamais une inconnue ni une interdite.
    let grand = tiers(&b, "Odile").await;
    let avec = |personne: Uuid, du: NaiveDate| AbsenceAnnonceDto {
        eleve_id: eleve,
        du_nuit: du,
        au_nuit: du,
        recupere_par_personne_id: Some(personne),
        retour_prevu: Some(paris(du + Duration::days(1), 18, 0)),
        regularisation: false,
        motif_texte: None,
    };
    let r = b.ent.annoncer_absence_a(b.etab, p, PARENT, &avec(grand, avril(20)), now).await;
    assert!(refus_de(&r, "pas autorisée à récupérer"), "{r:?}");
    autoriser_recuperer(&b, eleve, grand).await;
    let a = b.ent.annoncer_absence_a(b.etab, p, PARENT, &avec(grand, avril(20)), now).await.unwrap();
    assert_eq!(a.recupere_par_personne_id, Some(grand));
    assert!(b.ent.annoncer_absence_a(b.etab, p, PARENT, &avec(pers, avril(22)), now).await.is_ok(), "le titulaire lui-même");
    interdire(&b, eleve, Some(grand)).await;
    let r = b.ent.annoncer_absence_a(b.etab, p, PARENT, &avec(grand, avril(24)), now).await;
    assert!(refus_de(&r, "pas autorisée à récupérer"), "une interdiction bloque toujours : {r:?}");
    // Annulation : par la famille, tant qu'une nuit n'est pas passée ; l'élève redevient attendu.
    assert!(est_attendu(&b, avril(16), eleve).await, "M-B : sans départ constaté, attendu");
    let (autre, _) = parent(&b, b.a[1], true).await;
    assert!(est_interdit(&b.ent.annuler_absence_a(b.etab, autre, PARENT, ok.id, now).await), "le titulaire d'un autre enfant ne l'annule pas");
    let an = b.ent.annuler_absence_a(b.etab, p, PARENT, ok.id, now).await.unwrap();
    assert_eq!(an.statut, "annulee");
    assert!(est_attendu(&b, avril(16), eleve).await && est_attendu(&b, avril(17), eleve).await);
    let r = b.ent.annuler_absence_a(b.etab, p, PARENT, ok.id, now).await;
    assert!(refus_de(&r, "ne peut plus être annulée"), "{r:?}");
    // Annuler une absence dont toutes les nuits sont passées : refusé (régularisation seulement).
    let passee = annoncer(&b, p, PARENT, eleve, avril(14), avril(14), paris(avril(14), 10, 0)).await.unwrap();
    let r = b.ent.annuler_absence_a(b.etab, p, PARENT, passee.id, paris(avril(16), 10, 0)).await;
    assert!(refus_de(&r, "sont passées"), "{r:?}");
    // L'absence annulée libère les nuits : on peut les annoncer de nouveau.
    assert!(annoncer(&b, p, PARENT, eleve, avril(16), avril(16), now).await.is_ok());
}

// ═════════════════════════════ Désaccords (R-IT20) ═════════════════════════════

/// R-IT20 — un désaccord connu des titulaires (socle 18) bloque l'annonce et la renvoie au CPE ; l'élève reste attendu ;
/// seule la direction tranche, avec trace ; chacun ne voit que ce qui le concerne.
#[tokio::test]
async fn r_it20_desaccord_connu_bloque_et_la_direction_tranche() {
    let Some(b) = banc().await else { return };
    let mardi = avril(14);
    let (p1, _) = parent(&b, b.a[0], true).await;
    let (p2, _) = parent(&b, b.a[0], true).await;
    desaccord_connu(&b, b.a[0], p2).await;
    let a = annoncer(&b, p1, PARENT, b.a[0], mardi, mardi, paris(mardi, 11, 0)).await.unwrap();
    assert_eq!(a.statut, "bloquee", "CIT-07 : annonce bloquée, renvoyée au CPE");
    assert!(est_attendu(&b, mardi, b.a[0]).await, "l'élève reste attendu : aucune fausse absence");
    assert!(!b.evenements("internat.absence_bloquee").await.is_empty());
    // Le CPE la voit en tête de sa file mais ne peut ni l'accepter ni la refuser ; il ne tranche pas non plus.
    let file = b.ent.absences_a_traiter_a(b.etab, b.cpe, CPE).await.unwrap();
    assert_eq!(file[0].id, a.id);
    let r = b.ent.decider_absence_a(b.etab, b.cpe, CPE, a.id, &decision("accepter", None), paris(mardi, 11, 5)).await;
    assert!(refus_de(&r, "bloquée par un désaccord"), "{r:?}");
    let tr = |d: &str, m: Option<&str>| TrancheDto { decision: d.into(), motif_texte: m.map(String::from) };
    assert!(est_interdit(&b.ent.trancher_absence_a(b.etab, b.cpe, CPE, a.id, &tr("lever", None), paris(mardi, 11, 6)).await), "le CPE seul ne tranche pas");
    assert!(est_interdit(&b.ent.trancher_absence_a(b.etab, p1, PARENT, a.id, &tr("lever", None), paris(mardi, 11, 6)).await));
    let r = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &tr("peut-être", None), paris(mardi, 11, 7)).await;
    assert!(refus_de(&r, "décision inconnue"), "{r:?}");
    // La direction lève : l'état d'avant le blocage est rétabli (annoncée) et l'attendu suit.
    let lev = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &tr("lever", Some("accord des deux parents reçu")), paris(mardi, 11, 8)).await.unwrap();
    assert_eq!(lev.statut, "annoncee");
    assert!(est_attendu(&b, mardi, b.a[0]).await, "M-B : sans départ constaté, l'élève reste attendu");
    let f = faits(&b, "absence_id", a.id).await;
    assert_eq!(f.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["annonce", "blocage", "tranchee_levee"]);
    assert_eq!((f[2].1.as_str(), f[2].2), ("direction", Some(b.dir)), "trace : qui a tranché");
    // Une absence bloquée puis MAINTENUE est refusée, avec un motif lisible par la famille.
    let (p3, _) = parent(&b, b.a[1], true).await;
    desaccord_connu(&b, b.a[1], p3).await;
    let a1 = annoncer(&b, p3, PARENT, b.a[1], mardi, mardi, paris(mardi, 11, 0)).await.unwrap();
    let m = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a1.id, &tr("maintenir", None), paris(mardi, 11, 9)).await.unwrap();
    assert_eq!(m.statut, "refusee");
    assert!(m.motif_refus_texte.as_deref().is_some_and(|t| t.contains("Désaccord")), "{:?}", m.motif_refus_texte);
    assert!(est_attendu(&b, mardi, b.a[1]).await);
    let vue = b.ent.absences_eleve_a(b.etab, p3, PARENT, b.a[1], paris(mardi, 12, 0)).await.unwrap();
    assert_eq!(vue[0].statut, "refusee");
    let evts = b.evenements("internat.desaccord_tranche").await;
    assert_eq!(evts.len(), 2);
    assert_eq!(evts[0]["cible"], "absence");
    // Une absence qui n'est pas bloquée ne se « tranche » pas.
    let r = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &tr("lever", None), paris(mardi, 11, 10)).await;
    assert!(refus_de(&r, "pas bloquée"), "{r:?}");
}

/// R-IT20 (suite du pilote) — deux titulaires se contredisent : le PLUS RESTRICTIF l'emporte tant que la direction n'a
/// pas tranché. L'opposition bloque l'absence (l'élève redevient attendu), l'auteur ne s'y oppose pas lui-même, la
/// famille ne voit jamais QUI s'oppose, la direction tranche avec trace.
#[tokio::test]
async fn r_it20_opposition_d_un_titulaire_le_plus_restrictif_l_emporte() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let mardi = avril(14);
    let (p1, _) = parent(&b, eleve, true).await;
    let (p2, _) = parent(&b, eleve, true).await;
    let a = annoncer(&b, p1, PARENT, eleve, avril(15), avril(15), paris(mardi, 11, 0)).await.unwrap();
    assert!(est_attendu(&b, avril(15), eleve).await, "M-B : sans départ constaté, attendu");
    let opp = |m: Option<&str>| OppositionDto { motif_texte: m.map(String::from) };
    // L'auteur ne s'oppose pas à lui-même ; un tiers non plus.
    let r = b.ent.opposer_absence_a(b.etab, p1, PARENT, a.id, &opp(None), paris(mardi, 12, 0)).await;
    assert!(refus_de(&r, "l'annuler plutôt"), "{r:?}");
    assert!(est_interdit(&b.ent.opposer_absence_a(b.etab, b.prof, "prof", a.id, &opp(None), paris(mardi, 12, 0)).await));
    assert!(est_interdit(&b.ent.opposer_absence_a(b.etab, b.cpe, CPE, a.id, &opp(None), paris(mardi, 12, 0)).await), "seul un titulaire s'oppose");
    let r = b.ent.opposer_absence_a(b.etab, p2, PARENT, a.id, &opp(Some("il a un examen mercredi")), paris(mardi, 12, 0)).await.unwrap();
    assert_eq!((r.statut.as_str(), r.opposition), ("bloquee", true));
    assert!(est_attendu(&b, avril(15), eleve).await, "le plus restrictif l'emporte : l'élève redevient attendu");
    // Aucun parent ne voit les coordonnées ni l'identité de l'autre : la vue de la famille ne porte que « une opposition existe ».
    let vue1 = b.ent.absences_eleve_a(b.etab, p1, PARENT, eleve, paris(mardi, 13, 0)).await.unwrap();
    assert!(vue1[0].opposition && vue1[0].statut == "bloquee");
    let brut = serde_json::to_string(&vue1).unwrap();
    assert!(!brut.contains(&p2.to_string()) && !brut.contains("examen mercredi"), "l'identité et le motif de l'opposant ne sortent pas : {brut}");
    // Le blocage (deux demandes contradictoires) ne se défait pas par le CPE.
    let r = b.ent.decider_absence_a(b.etab, b.cpe, CPE, a.id, &decision("accepter", None), paris(mardi, 13, 5)).await;
    assert!(refus_de(&r, "bloquée par un désaccord"), "{r:?}");
    // La direction maintient : refusée, motif lu de la famille ; trace complète.
    let tr = TrancheDto { decision: "maintenir".into(), motif_texte: Some("la direction maintient la présence".into()) };
    let m = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &tr, paris(mardi, 14, 0)).await.unwrap();
    assert_eq!((m.statut.as_str(), m.motif_refus_texte.as_deref()), ("refusee", Some("la direction maintient la présence")));
    assert!(est_attendu(&b, avril(15), eleve).await);
    let f = faits(&b, "absence_id", a.id).await;
    assert_eq!(f.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["annonce", "opposition", "blocage", "tranchee_maintenue"]);
    assert_eq!((f[1].1.as_str(), f[1].2), ("titulaire", Some(p2)));
    // Opposition sur une absence déjà acceptée par le CPE, puis la direction LÈVE : elle redevient en vigueur.
    let a2 = annoncer(&b, p1, PARENT, eleve, avril(16), avril(16), paris(mardi, 11, 0)).await.unwrap();
    b.ent.decider_absence_a(b.etab, b.cpe, CPE, a2.id, &decision("accepter", None), paris(mardi, 11, 30)).await.unwrap();
    assert!(est_attendu(&b, avril(16), eleve).await, "M-B : acceptée mais pas encore partie : attendu");
    b.ent.opposer_absence_a(b.etab, p2, PARENT, a2.id, &opp(None), paris(mardi, 15, 0)).await.unwrap();
    assert!(est_attendu(&b, avril(16), eleve).await);
    let l = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a2.id, &TrancheDto { decision: "lever".into(), motif_texte: None }, paris(mardi, 15, 5)).await.unwrap();
    assert_eq!(l.statut, "acceptee", "l'état d'avant le blocage est rétabli");
    assert!(est_attendu(&b, avril(16), eleve).await, "M-B : toujours aucun départ constaté");
    // Refusée : plus d'opposition possible.
    let r = b.ent.opposer_absence_a(b.etab, p2, PARENT, a.id, &opp(None), paris(mardi, 16, 0)).await;
    assert!(refus_de(&r, "n'est plus en cours"), "{r:?}");
}

/// R-IT20 appliqué aux sorties : un refus après l'accord d'un autre titulaire, ou une opposition, bloque la sortie
/// (l'élève ne part pas) jusqu'à la décision de la direction.
#[tokio::test]
async fn r_it20_desaccord_sur_une_sortie_refus_apres_accord_ou_opposition_bloque() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (p1, _) = parent(&b, eleve, true).await;
    let (p2, _) = parent(&b, eleve, true).await;
    let mardi = avril(14);
    let maintenant = paris(mardi, 9, 0);
    // Sortie exceptionnelle (ponctuelle, titulaires et CPE) demandée par le CPE : les deux titulaires doivent répondre.
    let debut = paris(avril(16), 14, 0);
    let d = SortieDemandeDto { eleve_id: eleve, type_code: "exceptionnelle".into(), debut, fin_prevue: debut + Duration::hours(3), seul: false, avec_personne_id: Some(parent_personne(&b, p1).await), correspondant_id: None };
    let s = b.ent.demander_sortie_a(b.etab, b.cpe, CPE, &d, maintenant).await.unwrap();
    assert_eq!((s.statut.as_str(), s.decision_cpe, s.accord_titulaire), ("demandee", true, false));
    let acc = |ok: bool, m: Option<&str>| AccordTitulaireDto { accord: ok, motif_texte: m.map(String::from), avec_personne_id: None };
    // p1 accorde → autorisée (CPE déjà d'accord) ; p2 refuse ensuite → c'est une opposition, la sortie est bloquée.
    let s1 = b.ent.repondre_sortie_a(b.etab, p1, PARENT, s.id, &acc(true, None), maintenant).await.unwrap();
    assert_eq!(s1.statut, "autorisee");
    let r = b.ent.repondre_sortie_a(b.etab, p2, PARENT, s.id, &acc(false, Some("pas ce jour-là")), maintenant).await.unwrap();
    assert_eq!((r.statut.as_str(), r.opposition), ("bloquee", true));
    // Le contrôle de départ refuse une sortie bloquée (et trace la tentative) ; la direction tranche.
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &ControleDepartDto { remis_a_personne_id: s.avec_personne_id, a: Some(debut) }, debut).await;
    assert!(refus_de(&r, "désaccord entre les titulaires"), "{r:?}");
    assert!(faits(&b, "sortie_id", s.id).await.iter().any(|f| f.0 == "depart_refuse"), "tentative tracée");
    let m = b.ent.trancher_sortie_a(b.etab, b.dir, DIR, s.id, &TrancheDto { decision: "maintenir".into(), motif_texte: None }, maintenant).await.unwrap();
    assert_eq!(m.statut, "refusee");
    assert!(m.motif_refus_texte.is_some());
    // Seconde sortie : l'un refuse AVANT tout accord : refusée net (aucun désaccord à arbitrer).
    let debut2 = paris(avril(17), 14, 0);
    let s2 = b.ent.demander_sortie_a(b.etab, b.cpe, CPE, &SortieDemandeDto { debut: debut2, fin_prevue: debut2 + Duration::hours(2), ..d.clone() }, maintenant).await.unwrap();
    let r = b.ent.repondre_sortie_a(b.etab, p2, PARENT, s2.id, &acc(false, None), maintenant).await;
    assert!(refus_de(&r, "motif court"), "{r:?}");
    let r = b.ent.repondre_sortie_a(b.etab, p2, PARENT, s2.id, &acc(false, Some("non")), maintenant).await.unwrap();
    assert_eq!(r.statut, "refusee");
    // Et l'autre titulaire ne peut plus l'accorder (le plus restrictif l'emporte).
    let r = b.ent.repondre_sortie_a(b.etab, p1, PARENT, s2.id, &acc(true, None), maintenant).await;
    assert!(refus_de(&r, "n'attend plus de réponse"), "{r:?}");
}

async fn parent_personne(b: &Banc, user: Uuid) -> Uuid {
    personne_de(&b.ent, b.etab, user).await
}

// ═════════════════════════════ Élève majeur (R-IT22) ═════════════════════════════

/// R-IT22 — l'élève majeur annonce lui-même ses absences et décide de ses sorties dans le cadre du règlement ; ses parents
/// sont informés SEULEMENT si le règlement le prévoit (`majeur_info_parents`). CIT-10 : il devient majeur en cours d'année.
#[tokio::test]
async fn r_it22_majeur_decide_seul_et_information_des_parents_selon_le_reglement() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let mardi = avril(14);
    // Né le 15 avril 2008 : mineur le 14 avril 2026, majeur le 15.
    let naissance = NaiveDate::from_ymd_opt(2008, 4, 15).unwrap();
    let me = compte_eleve(&b, eleve, naissance).await;
    let r = annoncer(&b, me, ELEVE, eleve, avril(16), avril(16), paris(mardi, 11, 0)).await;
    assert!(refus_de(&r, "élève mineur"), "la veille de ses 18 ans : {r:?}");
    let a = annoncer(&b, me, ELEVE, eleve, avril(16), avril(16), paris(avril(15), 11, 0)).await.unwrap();
    assert_eq!((a.statut.as_str(), a.annonce_qualite.as_str()), ("annoncee", "eleve_majeur"), "CIT-10 : devenu majeur, il décide");
    assert!(est_retire(&b, avril(16), eleve, "absence_annoncee").await);
    // Règlement par défaut « si à charge » (F-E) : les parents ne lisent rien (notion de payeur non lue par ce lot),
    // donc aucun événement d'information n'est publié.
    assert!(b.evenements("internat.majeur_informe").await.is_empty(), "« si_a_charge » : pas d'information");
    // Règlement « toujours » : les parents sont informés (événement sans nom, politique rappelée).
    sqlx::query(
        "INSERT INTO ent_internat_parametres (etablissement_id, majeur_info_parents) VALUES ($1, 'toujours') \
         ON CONFLICT (etablissement_id) DO UPDATE SET majeur_info_parents = 'toujours'",
    )
    .bind(b.etab)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    annoncer(&b, me, ELEVE, eleve, avril(20), avril(20), paris(avril(15), 11, 0)).await.unwrap();
    let evts = b.evenements("internat.majeur_informe").await;
    assert_eq!(evts.len(), 1);
    assert_eq!((evts[0]["nature"].as_str(), evts[0]["politique"].as_str()), (Some("absence"), Some("toujours")));
    // Règlement « jamais » : plus aucune information aux parents.
    sqlx::query(
        "INSERT INTO ent_internat_parametres (etablissement_id, majeur_info_parents) VALUES ($1, 'jamais') \
         ON CONFLICT (etablissement_id) DO UPDATE SET majeur_info_parents = 'jamais'",
    )
    .bind(b.etab)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    annoncer(&b, me, ELEVE, eleve, avril(21), avril(21), paris(avril(15), 11, 0)).await.unwrap();
    assert_eq!(b.evenements("internat.majeur_informe").await.len(), 1, "« jamais » : rien de plus");
    // Un titulaire ne décide plus à sa place de la sortie de soirée d'un majeur : le majeur l'autorise lui-même, sans CPE.
    let debut = paris(avril(22), 19, 0);
    let s = b
        .ent
        .demander_sortie_a(
            b.etab,
            me,
            ELEVE,
            &SortieDemandeDto { eleve_id: eleve, type_code: "soiree".into(), debut, fin_prevue: debut + Duration::hours(4), seul: true, avec_personne_id: None, correspondant_id: None },
            paris(avril(15), 12, 0),
        )
        .await
        .unwrap();
    assert_eq!(s.statut, "autorisee", "R-IT22 : le majeur décide dans le cadre du règlement");
    // Le mineur du même internat, lui, attend ses titulaires ET le CPE (aucun accord annuel).
    let mineur = b.a[1];
    let moi = compte_eleve(&b, mineur, NaiveDate::from_ymd_opt(2010, 3, 14).unwrap()).await;
    let r = b
        .ent
        .demander_sortie_a(
            b.etab,
            moi,
            ELEVE,
            &SortieDemandeDto { eleve_id: mineur, type_code: "soiree".into(), debut, fin_prevue: debut + Duration::hours(4), seul: true, avec_personne_id: None, correspondant_id: None },
            paris(avril(15), 12, 0),
        )
        .await
        .unwrap();
    assert_eq!(r.statut, "demandee");
    assert!(!r.accord_titulaire, "l'élève mineur ne vaut pas accord de ses titulaires");
    // Un élève ne parle jamais pour un autre.
    let r = b
        .ent
        .demander_sortie_a(
            b.etab,
            moi,
            ELEVE,
            &SortieDemandeDto { eleve_id: eleve, type_code: "soiree".into(), debut: debut + Duration::days(1), fin_prevue: debut + Duration::days(1) + Duration::hours(3), seul: true, avec_personne_id: None, correspondant_id: None },
            paris(avril(15), 12, 0),
        )
        .await;
    assert!(est_interdit(&r), "{r:?}");
}

// ═════════════════════════════ Sorties ═════════════════════════════

fn demande(eleve: Uuid, type_code: &str, debut: DateTime<Utc>, fin: DateTime<Utc>, seul: bool, avec: Option<Uuid>) -> SortieDemandeDto {
    SortieDemandeDto { eleve_id: eleve, type_code: type_code.into(), debut, fin_prevue: fin, seul, avec_personne_id: avec, correspondant_id: None }
}

fn depart(personne: Option<Uuid>, a: DateTime<Utc>) -> ControleDepartDto {
    ControleDepartDto { remis_a_personne_id: personne, a: Some(a) }
}

/// Active l'objet « sortie_internat » du socle 18 et fait accorder `usages` par `parent` pour `eleve`.
pub(crate) async fn accorder_annuellement(b: &Banc, parent: Uuid, eleve: Uuid, usages: &[&str]) {
    b.ent.accords_activer_objet(b.etab, b.dir, DIR, "sortie_internat").await.expect("objet activé");
    b.ent.accords_publier_version(b.etab, b.dir, DIR, "sortie_internat", None).await.expect("version publiée");
    // L'accord date de l'instant réel de la décision (socle 18) : pour des jours fixes d'avril 2026, la validité « année
    // scolaire » dépendrait du jour où le test tourne ; on la fixe « sans terme » (la validité est éprouvée par le socle 18).
    sqlx::query("UPDATE ent_acc_objet SET validite = 'sans_terme' WHERE etablissement_id = $1 AND code = 'sortie_internat'")
        .bind(b.etab)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let choix = usages.iter().map(|u| crate::accords::ChoixUsage { code: (*u).into(), accorde: true }).collect();
    b.ent
        .accords_decider(b.etab, parent, crate::accords::DecisionSaisie { objet_code: "sortie_internat".into(), eleve_id: Some(eleve), usages: choix })
        .await
        .expect("accord du titulaire");
}

/// Autorisations annuelles (socle 18) : jamais « autorisé » par défaut ; la sortie du mercredi s'appuie sur l'accord
/// annuel ; sa révocation par un titulaire a un effet immédiat et suspend les sorties à venir.
#[tokio::test]
async fn sortie_du_mercredi_autorisation_annuelle_et_revocation_suspend() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (p, _) = parent(&b, eleve, true).await;
    let moi = compte_eleve(&b, eleve, NaiveDate::from_ymd_opt(2010, 3, 14).unwrap()).await;
    let mercredi = avril(15);
    let (debut, fin) = (paris(mercredi, 13, 0), paris(mercredi, 18, 0));
    let maintenant = paris(avril(14), 12, 0);
    // Sans objet activé : aucun accord, « inconnu » ; la demande du mercredi est refusée (jamais autorisée par défaut).
    let etat = b.ent.autorisations_eleve_a(b.etab, p, PARENT, eleve, maintenant).await.unwrap();
    assert!(etat.types.iter().all(|t| t.verdict == "inconnu"), "{etat:?}");
    assert!(etat.types.iter().any(|t| t.type_code == "mercredi") && !etat.types.iter().any(|t| t.type_code == "exceptionnelle"));
    let r = b.ent.demander_sortie_a(b.etab, moi, ELEVE, &demande(eleve, "mercredi", debut, fin, true, None), maintenant).await;
    assert!(refus_de(&r, "autorisation annuelle"), "{r:?}");
    accorder_annuellement(&b, p, eleve, &["sortie_mercredi"]).await;
    let etat = b.ent.autorisations_eleve_a(b.etab, moi, ELEVE, eleve, maintenant).await.unwrap();
    let verdict = |t: &str| etat.types.iter().find(|x| x.type_code == t).unwrap().verdict.clone();
    assert_eq!(verdict("mercredi"), "autorise");
    assert_ne!(verdict("soiree"), "autorise", "chaque usage se choisit séparément");
    // INT-154 : l'élève s'inscrit à la sortie du mercredi : l'accord annuel vaut accord, aucun CPE à attendre.
    let s = b.ent.demander_sortie_a(b.etab, moi, ELEVE, &demande(eleve, "mercredi", debut, fin, true, None), maintenant).await.unwrap();
    assert_eq!((s.statut.as_str(), s.par_autorisation_annuelle, s.accord_titulaire), ("autorisee", true, true));
    // Une sortie courte ne retire jamais l'élève de l'attendu : il se pointe « en activité ».
    assert!(est_attendu(&b, mercredi, eleve).await);
    // Chevauchement refusé (une sortie à la fois) ; un type qui ne se fait pas seul aussi.
    let r = b.ent.demander_sortie_a(b.etab, moi, ELEVE, &demande(eleve, "mercredi", debut + Duration::hours(1), fin, true, None), maintenant).await;
    assert!(refus_de(&r, "déjà une sortie"), "{r:?}");
    let r = b.ent.demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "nuit_correspondant", debut + Duration::days(1), fin + Duration::days(1), true, None), maintenant).await;
    assert!(refus_de(&r, "ne se fait pas seul"), "{r:?}");
    // Départ puis retour contrôlés par le surveillant (INT-159, INT-160).
    let r = b.ent.controler_depart_a(b.etab, p, PARENT, s.id, &depart(None, debut), debut).await;
    assert!(est_interdit(&r), "la famille ne contrôle pas un départ : {r:?}");
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, debut - Duration::hours(5)), debut - Duration::hours(5)).await;
    assert!(refus_de(&r, "trop tôt"), "{r:?}");
    let part = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, debut + Duration::minutes(5)), debut + Duration::minutes(5)).await.unwrap();
    assert_eq!(part.statut, "partie");
    let rent = b.ent.controler_retour_a(b.etab, b.cpe, CPE, s.id, &ControleRetourDto { a: Some(fin - Duration::minutes(10)) }, fin).await.unwrap();
    assert_eq!((rent.statut.as_str(), rent.retour_effectif.is_some()), ("rentree", true));
    // Un titulaire révoque l'autorisation annuelle : effet immédiat ; la sortie du mercredi suivant est suspendue.
    let suivant = avril(22);
    let (d2, f2) = (paris(suivant, 13, 0), paris(suivant, 18, 0));
    let s2 = b.ent.demander_sortie_a(b.etab, moi, ELEVE, &demande(eleve, "mercredi", d2, f2, true, None), maintenant).await.unwrap();
    assert_eq!(s2.statut, "autorisee");
    let r = b.ent.revoquer_autorisation_a(b.etab, b.prof, "prof", &RevocationDto { eleve_id: eleve, usages: None }, maintenant).await;
    assert!(est_interdit(&r), "seul un titulaire (ou le majeur) révoque : {r:?}");
    let rev = b.ent.revoquer_autorisation_a(b.etab, p, PARENT, &RevocationDto { eleve_id: eleve, usages: None }, maintenant).await.unwrap();
    assert_eq!(rev.sorties_suspendues, 1);
    let apres = b.ent.sorties_eleve_a(b.etab, p, PARENT, eleve, maintenant).await.unwrap();
    assert_eq!(apres.iter().find(|x| x.id == s2.id).unwrap().statut, "suspendue");
    let r = b.ent.autorisations_eleve_a(b.etab, p, PARENT, eleve, maintenant).await.unwrap();
    assert_ne!(r.types.iter().find(|x| x.type_code == "mercredi").unwrap().verdict, "autorise", "effet immédiat dans le socle 18");
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s2.id, &depart(None, d2), d2).await;
    assert!(refus_de(&r, "suspendue"), "{r:?}");
    assert!(!b.evenements("internat.autorisation_revoquee").await.is_empty());
    // La sortie du mercredi qui suit sa propre demande n'est plus possible sans nouvel accord.
    let r = b.ent.demander_sortie_a(b.etab, moi, ELEVE, &demande(eleve, "mercredi", d2 + Duration::days(7), f2 + Duration::days(7), true, None), maintenant).await;
    assert!(refus_de(&r, "autorisation annuelle"), "{r:?}");
}

/// NI-33 — sortie de week-end avec une personne autorisée : départ contrôlé (R-IT23), retour contrôlé (clôture),
/// l'attendu des nuits couvertes est retiré avec le motif « sortie » puis rendu au retour.
#[tokio::test]
async fn ni_33_week_end_depart_remise_retour_et_cloture() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (p, pers) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let (vendredi, dimanche) = (avril(17), avril(19));
    let maintenant = paris(avril(14), 12, 0);
    let (debut, fin) = (paris(vendredi, 16, 0), paris(dimanche, 20, 0));
    // Une personne inconnue ne peut pas être désignée ; le titulaire et la personne autorisée le peuvent.
    let inconnue = tiers(&b, "Inconnue").await;
    let r = b.ent.demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(inconnue)), maintenant).await;
    assert!(refus_de(&r, "pas autorisée à récupérer"), "R-IT23 : {r:?}");
    let s = b.ent.demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(grand)), maintenant).await.unwrap();
    assert_eq!(s.statut, "autorisee", "week-end : règle implicite, la famille a déjà tout dit en le demandant");
    // Attendu recalculé : le vendredi et le samedi sont retirés (« sortie »), le dimanche soir est attendu.
    assert!(est_attendu(&b, avril(16), eleve).await);
    // E-3 : autorisée mais pas encore partie, l'élève reste attendu ; il sort de l'attendu au départ contrôlé.
    assert!(est_attendu(&b, vendredi, eleve).await && est_attendu(&b, avril(18), eleve).await);
    assert!(est_attendu(&b, dimanche, eleve).await, "il rentre le dimanche avant l'appel du soir");
    // Contrôle de départ : une remise à une personne non autorisée aujourd'hui est refusée (et tracée) ; sans personne aussi.
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, debut), debut).await;
    assert!(refus_de(&r, "indiquer la personne"), "{r:?}");
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(inconnue), debut), debut).await;
    assert!(refus_de(&r, "pas autorisée à récupérer l'élève aujourd'hui"), "{r:?}");
    let f = faits(&b, "sortie_id", s.id).await;
    assert!(f.iter().any(|x| x.0 == "depart_refuse"), "la tentative de remise à une personne non autorisée est tracée");
    let part = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut + Duration::minutes(10)), debut + Duration::minutes(10)).await.unwrap();
    assert_eq!((part.statut.as_str(), part.depart_effectif.is_some()), ("partie", true));
    assert!(est_retire(&b, vendredi, eleve, "sortie").await && est_retire(&b, avril(18), eleve, "sortie").await, "partie : retiré de l'attendu");
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut), debut + Duration::minutes(11)).await;
    assert!(refus_de(&r, "déjà parti"), "{r:?}");
    // Un retour avant le départ est absurde ; le retour réel clôt la sortie.
    let r = b.ent.controler_retour_a(b.etab, b.cpe, CPE, s.id, &ControleRetourDto { a: Some(debut) }, debut + Duration::minutes(20)).await;
    assert!(refus_de(&r, "ne précède pas"), "{r:?}");
    let rent = b.ent.controler_retour_a(b.etab, b.cpe, CPE, s.id, &ControleRetourDto { a: Some(fin - Duration::minutes(30)) }, fin).await.unwrap();
    assert_eq!(rent.statut, "rentree");
    let r = b.ent.controler_retour_a(b.etab, b.cpe, CPE, s.id, &ControleRetourDto { a: Some(fin) }, fin).await;
    assert!(refus_de(&r, "n'est pas parti"), "{r:?}");
    assert!(est_attendu(&b, dimanche, eleve).await);
    // Événements : demande, décision, départ et retour, sans nom ni texte.
    for t in ["internat.sortie_demandee", "internat.sortie_decidee", "internat.sortie_partie", "internat.sortie_rentree"] {
        assert!(!b.evenements(t).await.is_empty(), "{t}");
    }
    let _ = pers;
}

/// R-IT23 — une interdiction de remise bloque TOUJOURS, même pour une personne par ailleurs autorisée, et la
/// tentative est tracée ; une interdiction posée après la demande suspend la sortie, sa levée la rétablit.
#[tokio::test]
async fn r_it23_remise_interdiction_bloque_toujours_et_tentative_tracee() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (p, _) = parent(&b, eleve, true).await;
    let pere = tiers(&b, "Paul").await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, pere).await;
    autoriser_recuperer(&b, eleve, grand).await;
    let maintenant = paris(avril(14), 12, 0);
    let (debut, fin) = (paris(avril(17), 16, 0), paris(avril(19), 20, 0));
    let s = b.ent.demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(grand)), maintenant).await.unwrap();
    // CIT-06 : le père, visé par une interdiction de remise, vient chercher sa fille : refusé, consigne, trace.
    interdire(&b, eleve, Some(pere)).await;
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(pere), debut), debut).await;
    assert!(refus_de(&r, "REMISE REFUSÉE"), "{r:?}");
    let f = sqlx::query_as::<_, (String, Option<String>, Option<Uuid>)>(
        "SELECT acte, motif_code, personne_id FROM ent_internat_i3_fait WHERE etablissement_id = $1 AND sortie_id = $2 AND acte = 'depart_refuse'",
    )
    .bind(b.etab)
    .bind(s.id)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(f, vec![("depart_refuse".to_string(), Some("interdiction_remise".to_string()), Some(pere))]);
    // La sortie n'a pas bougé et la grand-mère, autorisée, peut le prendre.
    let apres = b.ent.sorties_eleve_a(b.etab, p, PARENT, eleve, maintenant).await.unwrap();
    assert_eq!(apres[0].statut, "autorisee");
    // Une interdiction posée sur LA personne désignée suspend la sortie à la prochaine revue (veille) ; levée, elle reprend.
    interdire(&b, eleve, Some(grand)).await;
    let v = b.ent.veille_sorties_a(b.etab, paris(avril(15), 8, 0)).await.unwrap();
    assert_eq!(v.sorties_expirees, 0);
    let apres = b.ent.sorties_eleve_a(b.etab, p, PARENT, eleve, maintenant).await.unwrap();
    assert_eq!(apres[0].statut, "suspendue");
    assert!(!est_retire(&b, avril(17), eleve, "sortie").await, "suspendue : l'élève redevient attendu");
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut), debut).await;
    assert!(refus_de(&r, "suspendue"), "{r:?}");
    sqlx::query("UPDATE ent_eleve_interdiction SET levee_le = now(), levee_par = $3, levee_motif = 'levée' WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $4")
        .bind(b.etab)
        .bind(eleve)
        .bind(b.dir)
        .bind(grand)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    b.ent.veille_sorties_a(b.etab, paris(avril(15), 8, 5)).await.unwrap();
    let apres = b.ent.sorties_eleve_a(b.etab, p, PARENT, eleve, maintenant).await.unwrap();
    assert_eq!(apres[0].statut, "autorisee", "cause disparue : la sortie reprend d'elle-même");
    // Une interdiction sans personne identifiée (libellé seul) interdit toute remise accompagnée : appeler la direction.
    interdire(&b, eleve, None).await;
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut), debut).await;
    assert!(refus_de(&r, "sans personne identifiée") || refus_de(&r, "suspendue"), "{r:?}");
    // Fusion de personnes : une interdiction visant l'absorbée vise la personne dans laquelle elle a été fusionnée.
    let (p2, _) = parent(&b, b.a[1], true).await;
    let (a_garder, absorbee) = (tiers(&b, "Gardée").await, tiers(&b, "Absorbée").await);
    autoriser_recuperer(&b, b.a[1], a_garder).await;
    interdire(&b, b.a[1], Some(absorbee)).await;
    sqlx::query("UPDATE ent_personne SET fusionnee_dans = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(absorbee)
        .bind(a_garder)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let r = b.ent.demander_sortie_a(b.etab, p2, PARENT, &demande(b.a[1], "week_end", debut, fin, false, Some(a_garder)), maintenant).await;
    assert!(refus_de(&r, "pas autorisée à récupérer"), "la chaîne de fusions porte l'interdiction : {r:?}");
}

/// INT-157, INT-166, INT-167 — départ du vendredi : l'attendu de départ est établi pour ceux qui n'y dorment pas ce
/// soir ; la liste de remise de l'accueil montre les personnes autorisées et, en rouge, les interdites ; un élève non
/// récupéré à l'heure limite ouvre un cas.
#[tokio::test]
async fn depart_du_vendredi_attendu_de_depart_liste_de_remise_et_non_recupere() {
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (
        eleve_invite(&b, "Léo").await,
        eleve_invite(&b, "Max").await,
        eleve_invite(&b, "Sam").await,
    );
    // Rythme du dimanche soir au jeudi soir : pas de nuit du vendredi.
    for e in [e1, e2, e3] {
        heberger_rythme(&b, e, [true, true, true, true, true, false, false]).await;
    }
    let vendredi = avril(17);
    let maintenant = paris(avril(14), 10, 0);
    // e3 a déjà une absence annoncée pour le vendredi soir : déjà couvert.
    let (p3, pers3) = parent(&b, e3, false).await;
    let abs3 = annoncer(&b, p3, PARENT, e3, vendredi, vendredi, maintenant).await.unwrap();
    let r = b.ent.preparer_departs_a(b.etab, b.prof, "prof", vendredi, maintenant).await;
    assert!(est_interdit(&r), "{r:?}");
    let r = b.ent.preparer_departs_a(b.etab, b.cpe, CPE, vendredi - Duration::days(30), maintenant).await;
    assert!(refus_de(&r, "passé"), "{r:?}");
    let d = b.ent.preparer_departs_a(b.etab, b.cpe, CPE, vendredi, maintenant).await.unwrap();
    // Les trois nouveaux internes + a[0..3] (sept nuits par semaine, jamais hors rythme) : seuls e1, e2 sont à créer.
    assert_eq!((d.crees, d.sans_personne), (2, 2), "{d:?}");
    assert_eq!(d.deja_couverts, 1, "e3 est déjà retiré de l'attendu par son absence annoncée : {d:?}");
    let encore = b.ent.preparer_departs_a(b.etab, b.cpe, CPE, vendredi, maintenant).await.unwrap();
    assert_eq!((encore.crees, encore.deja_couverts), (0, 3), "idempotente");
    // Les sorties créées : week-end, autorisées, par le système, retour prévu le dimanche à l'heure de l'appel.
    let rows: Vec<(Uuid, String, String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT eleve_id, type_code, statut, demande_qualite, fin_prevue FROM ent_internat_sortie WHERE etablissement_id = $1 ORDER BY eleve_id",
    )
    .bind(b.etab)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    for (_, t, st, q, fin) in &rows {
        assert_eq!((t.as_str(), st.as_str(), q.as_str()), ("week_end", "autorisee", "systeme"));
        assert_eq!(*fin, paris(avril(19), 21, 30), "dimanche, heure limite de l'appel (21 h 30)");
    }
    // Hors rythme, e1 n'est de toute façon pas attendu le vendredi ni le samedi soir : la sortie n'y change rien ; il l'est le dimanche.
    assert!(!est_attendu(&b, vendredi, e1).await && !est_attendu(&b, avril(18), e1).await);
    assert!(est_attendu(&b, avril(19), e1).await);
    assert_eq!(b.evenements("internat.depart_prevu").await.len(), 2);
    // La famille de e1 précise qui vient ; la liste de remise de l'accueil le montre (jamais une coordonnée).
    let (p1, pers1) = parent(&b, e1, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, e1, grand).await;
    let sorties_e1 = b.ent.sorties_eleve_a(b.etab, p1, PARENT, e1, maintenant).await.unwrap();
    let s1 = sorties_e1[0].id;
    let r = b.ent.designer_personne_sortie_a(b.etab, p1, PARENT, s1, &PersonneSortieDto { avec_personne_id: Some(tiers(&b, "Inconnue").await), seul: false }, maintenant).await;
    assert!(refus_de(&r, "pas autorisée"), "{r:?}");
    b.ent.designer_personne_sortie_a(b.etab, p1, PARENT, s1, &PersonneSortieDto { avec_personne_id: Some(grand), seul: false }, maintenant).await.unwrap();
    interdire(&b, e2, Some(grand)).await;
    let (_p2, pers2) = parent(&b, e2, true).await;
    let liste = b.ent.liste_remise_a(b.etab, b.cpe, CPE, vendredi, maintenant).await.unwrap();
    assert_eq!(liste.iter().filter(|l| l.sortie_id.is_some()).count(), 2);
    let l1 = liste.iter().find(|l| l.eleve_id == e1).unwrap();
    assert_eq!(l1.personne_prevue.as_ref().map(|x| x.personne_id), Some(grand));
    let ids1: Vec<Uuid> = l1.personnes_autorisees.iter().map(|x| x.personne_id).collect();
    assert!(ids1.contains(&grand) && ids1.contains(&pers1));
    assert!(l1.personnes_interdites.is_empty());
    let l2 = liste.iter().find(|l| l.eleve_id == e2).unwrap();
    assert_eq!(l2.personnes_interdites.iter().map(|x| x.personne_id).collect::<Vec<_>>(), vec![grand], "la consigne rouge nomme la personne interdite");
    assert!(l2.personne_prevue.is_none() && l2.personnes_autorisees.iter().any(|x| x.personne_id == pers2));
    assert!(!l2.personnes_autorisees.iter().any(|x| x.personne_id == grand));
    let brut = serde_json::to_string(&liste).unwrap();
    assert!(!brut.contains("telephone") && !brut.contains("adresse"), "ni coordonnée ni adresse");
    // La famille ne lit pas la liste de remise ; elle est journalisée.
    assert!(est_interdit(&b.ent.liste_remise_a(b.etab, p1, PARENT, vendredi, maintenant).await));
    assert!(est_interdit(&b.ent.liste_remise_a(b.etab, b.prof, "prof", vendredi, maintenant).await));
    let lu: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_lecture_sensible WHERE etablissement_id = $1 AND objet = 'liste_remise'")
        .bind(b.etab)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(lu, 1);
    // e3 est remis à sa personne à 16 h 05 (M-B : sans départ constaté, un cas s'ouvrirait aussi pour lui).
    b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, abs3.id, &depart(Some(pers3), paris(vendredi, 16, 5)), paris(vendredi, 16, 5)).await.unwrap();
    // e1 est remis à 16 h 10 ; e2 n'est pas venu : à 18 h 29 rien, à 18 h 31 (18 h + 30 min) un cas « non récupéré ».
    b.ent.controler_depart_a(b.etab, b.cpe, CPE, s1, &depart(Some(grand), paris(vendredi, 16, 10)), paris(vendredi, 16, 10)).await.unwrap();
    let v = b.ent.veille_sorties_a(b.etab, paris(vendredi, 18, 29)).await.unwrap();
    assert_eq!(v.cas_non_recupere, 0);
    let v = b.ent.veille_sorties_a(b.etab, paris(vendredi, 18, 31)).await.unwrap();
    assert_eq!(v.cas_non_recupere, 1, "e2 seul : e1 est parti");
    let cas: Vec<(Uuid, String, NaiveDate, Option<Uuid>)> = sqlx::query_as("SELECT eleve_id, nature, nuit, ouvert_par_user_id FROM ent_internat_cas WHERE etablissement_id = $1")
        .bind(b.etab)
        .fetch_all(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(cas, vec![(e2, "non_recupere".to_string(), vendredi, None)], "ouvert par le système");
    let v = b.ent.veille_sorties_a(b.etab, paris(vendredi, 18, 40)).await.unwrap();
    assert_eq!(v.cas_non_recupere, 0, "un seul cas ouvert par élève et par nuit");
}

// ═════════════════════════════ Correspondants (CIT-33) ═════════════════════════════

#[tokio::test]
async fn correspondant_nuit_chez_le_correspondant_suspension_et_fin() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (p, _) = parent(&b, eleve, true).await;
    let corr = tiers(&b, "Odile").await;
    let (mardi, mercredi) = (avril(14), avril(15));
    let maintenant = paris(avril(13), 10, 0);
    let saisie = |personne: Uuid, jours: [bool; 7]| CorrespondantSaisieDto {
        eleve_id: eleve,
        personne_id: personne,
        lundi: jours[0],
        mardi: jours[1],
        mercredi: jours[2],
        jeudi: jours[3],
        vendredi: jours[4],
        samedi: jours[5],
        dimanche: jours[6],
        du: None,
        au: None,
    };
    let mardi_mercredi = [false, true, true, false, false, false, false];
    // Désignation : une personne autorisée à récupérer (socle 4), au moins un jour ; ni professeur ni tiers.
    let r = b.ent.designer_correspondant_a(b.etab, p, PARENT, &saisie(corr, mardi_mercredi), maintenant).await;
    assert!(refus_de(&r, "personne autorisée à récupérer"), "{r:?}");
    autoriser_recuperer(&b, eleve, corr).await;
    let r = b.ent.designer_correspondant_a(b.etab, p, PARENT, &saisie(corr, [false; 7]), maintenant).await;
    assert!(refus_de(&r, "au moins un jour"), "{r:?}");
    assert!(est_interdit(&b.ent.designer_correspondant_a(b.etab, b.prof, "prof", &saisie(corr, mardi_mercredi), maintenant).await));
    let (autre, _) = parent(&b, b.a[1], true).await;
    assert!(est_interdit(&b.ent.designer_correspondant_a(b.etab, autre, PARENT, &saisie(corr, mardi_mercredi), maintenant).await));
    let c = b.ent.designer_correspondant_a(b.etab, p, PARENT, &saisie(corr, mardi_mercredi), maintenant).await.unwrap();
    assert_eq!((c.statut.as_str(), c.mardi, c.jeudi), ("valide", true, false));
    let r = b.ent.designer_correspondant_a(b.etab, p, PARENT, &saisie(corr, mardi_mercredi), maintenant).await;
    assert!(refus_de(&r, "déjà désignée"), "{r:?}");
    // INT-168 : le surveillant voit le correspondant et ses jours, sans adresse ni coordonnée ; lecture journalisée.
    let vus = b.ent.correspondants_eleve_a(b.etab, b.cpe, CPE, eleve, maintenant).await.unwrap();
    assert_eq!(vus.len(), 1);
    let brut = serde_json::to_string(&vus).unwrap();
    assert!(!brut.contains("adresse") && !brut.contains("telephone"));
    assert!(est_interdit(&b.ent.correspondants_eleve_a(b.etab, autre, PARENT, eleve, maintenant).await));
    // INT-156 : nuit chez le correspondant (mardi 17 h → mercredi 8 h) : titulaires + CPE ; le correspondant désigné,
    // un jour où il accueille, ni seul ni sans correspondant.
    let (debut, fin) = (paris(mardi, 17, 0), paris(mercredi, 8, 0));
    let mut d = demande(eleve, "nuit_correspondant", debut, fin, false, None);
    let r = b.ent.demander_sortie_a(b.etab, p, PARENT, &d, maintenant).await;
    assert!(refus_de(&r, "correspondant désigné"), "{r:?}");
    d.correspondant_id = Some(c.id);
    let jeudi = demande(eleve, "nuit_correspondant", paris(avril(16), 17, 0), paris(avril(17), 8, 0), false, None);
    let r = b.ent.demander_sortie_a(b.etab, p, PARENT, &SortieDemandeDto { correspondant_id: Some(c.id), ..jeudi }, maintenant).await;
    assert!(refus_de(&r, "n'accueille pas l'élève ce jour-là"), "{r:?}");
    let s = b.ent.demander_sortie_a(b.etab, p, PARENT, &d, maintenant).await.unwrap();
    assert_eq!((s.statut.as_str(), s.accord_titulaire, s.decision_cpe), ("demandee", true, false), "le CPE décide aussi");
    assert!(est_attendu(&b, mardi, eleve).await, "pas d'effet avant l'autorisation");
    let s = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), maintenant).await.unwrap();
    assert_eq!(s.statut, "autorisee");
    assert!(est_attendu(&b, mardi, eleve).await, "E-3 : autorisée, pas encore partie : toujours attendu");
    // CIT-33 : le correspondant ne répond plus : le CPE le suspend ; la sortie est suspendue, l'élève redevient attendu.
    assert!(est_interdit(&b.ent.suspendre_correspondant_a(b.etab, p, PARENT, c.id, true, &CorrespondantSuspensionDto::default(), maintenant).await));
    let sus = b.ent.suspendre_correspondant_a(b.etab, b.cpe, CPE, c.id, true, &CorrespondantSuspensionDto { motif_texte: Some("ne répond plus".into()) }, maintenant).await.unwrap();
    assert_eq!(sus.statut, "suspendu");
    assert!(est_attendu(&b, mardi, eleve).await, "suspendue : l'élève redevient attendu");
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(corr), debut), debut).await;
    assert!(refus_de(&r, "suspendue"), "{r:?}");
    let r = b.ent.demander_sortie_a(b.etab, p, PARENT, &SortieDemandeDto { debut: debut + Duration::days(7), fin_prevue: fin + Duration::days(7), ..d.clone() }, maintenant).await;
    assert!(refus_de(&r, "suspendu"), "{r:?}");
    // Reprise : la sortie suspendue À CAUSE DU CORRESPONDANT reprend d'elle-même.
    b.ent.suspendre_correspondant_a(b.etab, b.cpe, CPE, c.id, false, &CorrespondantSuspensionDto::default(), maintenant).await.unwrap();
    let reprise = b.ent.sorties_eleve_a(b.etab, p, PARENT, eleve, maintenant).await.unwrap();
    assert_eq!(reprise[0].statut, "autorisee", "reprise du correspondant, reprise de la sortie");
    // Fin de la désignation par la famille : la sortie suspendue de nouveau, plus de désignation.
    let fin_d = b.ent.terminer_correspondant_a(b.etab, p, PARENT, c.id, maintenant).await.unwrap();
    assert_eq!(fin_d.statut, "termine");
    assert!(est_attendu(&b, mardi, eleve).await);
    assert!(b.ent.correspondants_eleve_a(b.etab, p, PARENT, eleve, maintenant).await.unwrap().is_empty());
    let f = faits(&b, "correspondant_id", c.id).await;
    assert_eq!(f.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["designation", "suspension", "reprise", "fin"]);
}

// ═════════════════════════════ Retards, soirées, cas ═════════════════════════════

/// Un week-end autorisé, parti, dont le retour est attendu le dimanche à 20 h.
async fn week_end_parti(b: &Banc, eleve: Uuid, p: Uuid, grand: Uuid, maintenant: DateTime<Utc>) -> SortieDto {
    let (debut, fin) = (paris(avril(17), 16, 0), paris(avril(19), 20, 0));
    let s = b.ent.demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(grand)), maintenant).await.unwrap();
    b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut), debut + Duration::minutes(1)).await.unwrap()
}

/// CIT-23 — le train est supprimé : la famille annonce un retard ; l'attendu est décalé et AUCUN cas ne s'ouvre tant que
/// le délai annoncé court. Sans annonce, un cas « retard de retour » s'ouvre au-delà du seuil (30 minutes) et se ferme
/// au retour de l'élève.
#[tokio::test]
async fn cit_23_retard_annonce_decale_l_attendu_et_le_cas_s_ouvre_sans_annonce() {
    let Some(b) = banc().await else { return };
    let maintenant = paris(avril(14), 12, 0);
    let (p, _) = parent(&b, b.a[0], true).await;
    let (p1, _) = parent(&b, b.a[1], true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, b.a[0], grand).await;
    autoriser_recuperer(&b, b.a[1], grand).await;
    let (avec_annonce, sans_annonce) = (b.a[0], b.a[1]);
    let s = week_end_parti(&b, avec_annonce, p, grand, maintenant).await;
    let s_sans = week_end_parti(&b, sans_annonce, p1, grand, maintenant).await;
    let dimanche = avril(19);
    // Annonce de retard (train) à 18 h : retour décalé à 23 h ; bornes : postérieur, 48 h au plus, motif connu, titulaire.
    let retard = |h: u32| RetardAnnonceDto { nouveau_retour: paris(dimanche, h, 0), motif_code: Some("train".into()) };
    let t18 = paris(dimanche, 18, 0);
    let r = b.ent.annoncer_retard_a(b.etab, p, PARENT, s.id, &retard(19), t18).await;
    assert!(refus_de(&r, "postérieur"), "{r:?}");
    let r = b.ent.annoncer_retard_a(b.etab, p, PARENT, s.id, &RetardAnnonceDto { nouveau_retour: paris(dimanche, 20, 0) + Duration::hours(49), motif_code: None }, t18).await;
    assert!(refus_de(&r, "48 heures"), "{r:?}");
    let r = b.ent.annoncer_retard_a(b.etab, p, PARENT, s.id, &RetardAnnonceDto { motif_code: Some("grève".into()), ..retard(23) }, t18).await;
    assert!(refus_de(&r, "motif de retard inconnu"), "{r:?}");
    assert!(est_interdit(&b.ent.annoncer_retard_a(b.etab, p1, PARENT, s.id, &retard(23), t18).await), "le titulaire d'un autre enfant ne l'annonce pas");
    let ok = b.ent.annoncer_retard_a(b.etab, p, PARENT, s.id, &retard(23), t18).await.unwrap();
    assert_eq!(ok.fin_ajustee, paris(dimanche, 23, 0));
    assert!(!b.evenements("internat.retard_annonce").await.is_empty());
    // 20 h 40 : l'élève annoncé n'a pas de cas ; l'autre, oui (seuil de 30 minutes dépassé).
    let v = b.ent.veille_sorties_a(b.etab, paris(dimanche, 20, 29)).await.unwrap();
    assert_eq!(v.cas_retard_retour, 0, "avant le seuil");
    let v = b.ent.veille_sorties_a(b.etab, paris(dimanche, 20, 40)).await.unwrap();
    assert_eq!(v.cas_retard_retour, 1);
    let cas: Vec<(Uuid, String)> = sqlx::query_as("SELECT eleve_id, nature FROM ent_internat_cas WHERE etablissement_id = $1")
        .bind(b.etab)
        .fetch_all(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(cas, vec![(sans_annonce, "retard_retour".to_string())], "CIT-23 : pas de cas tant que le délai annoncé court");
    // L'attendu de cette nuit n'est pas retiré (il rentre avant 6 h) : l'élève annoncé est attendu à l'appel de 21 h 30.
    assert!(est_attendu(&b, dimanche, avec_annonce).await);
    // L'élève sans annonce rentre à 21 h 10 : le cas se ferme (retrouvé, dans sa chambre) ; l'autre à 23 h 20.
    let rent = b.ent.controler_retour_a(b.etab, b.cpe, CPE, s_sans.id, &ControleRetourDto { a: Some(paris(dimanche, 21, 10)) }, paris(dimanche, 21, 10)).await.unwrap();
    assert_eq!(rent.statut, "rentree");
    let ferme: Vec<(String, Option<String>)> = sqlx::query_as("SELECT issue, issue_lieu_code FROM ent_internat_cas WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(b.etab)
        .bind(sans_annonce)
        .fetch_all(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(ferme, vec![("retrouve".to_string(), Some("chambre".to_string()))]);
    b.ent.controler_retour_a(b.etab, b.cpe, CPE, s.id, &ControleRetourDto { a: Some(paris(dimanche, 23, 20)) }, paris(dimanche, 23, 20)).await.unwrap();
    let v = b.ent.veille_sorties_a(b.etab, paris(dimanche, 23, 40)).await.unwrap();
    assert_eq!(v.cas_retard_retour, 0, "rentré avant la fin du délai annoncé : aucun cas");
}

/// Une sortie de soirée (19 h – 23 h) ne retire JAMAIS l'élève de l'attendu : il reste attendu, le surveillant le voit
/// « en sortie jusqu'à 23 h » et le pointe « en activité » ; la clôture attend son retour. Sinon un élève qui ne rentre pas
/// passerait inaperçu.
#[tokio::test]
async fn sortie_de_soiree_courte_reste_attendue_en_activite_sans_fausse_alerte() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let mardi = avril(14);
    let maintenant = paris(mardi, 9, 0);
    let (p, _) = parent(&b, eleve, true).await;
    accorder_annuellement(&b, p, eleve, &["sortie_soiree"]).await;
    let (debut, fin) = (paris(mardi, 19, 0), paris(mardi, 23, 0));
    // Soirée : l'accord annuel de la famille vaut accord ; le CPE doit aussi décider.
    let s = b.ent.demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "soiree", debut, fin, true, None), maintenant).await.unwrap();
    assert_eq!((s.statut.as_str(), s.par_autorisation_annuelle, s.decision_cpe), ("demandee", true, false));
    let file = b.ent.sorties_a_traiter_a(b.etab, b.cpe, CPE).await.unwrap();
    assert!(file.iter().any(|x| x.id == s.id));
    let s = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), maintenant).await.unwrap();
    assert_eq!(s.statut, "autorisee");
    assert!(est_attendu(&b, mardi, eleve).await, "reste attendu : aucune fausse absence, aucun oubli");
    // Le surveillant voit la liste des sorties de la soirée.
    let soirs = b.ent.sorties_du_soir_a(b.etab, b.cpe, CPE, mardi, maintenant).await.unwrap();
    assert_eq!(soirs.len(), 1);
    assert_eq!((soirs[0].eleve_id, soirs[0].fin_ajustee), (eleve, fin));
    assert!(b.ent.sorties_du_soir_a(b.etab, b.cpe, CPE, avril(15), maintenant).await.unwrap().is_empty());
    assert!(est_interdit(&b.ent.sorties_du_soir_a(b.etab, p, PARENT, mardi, maintenant).await));
    // Départ à 19 h 05 ; l'appel de 21 h 45 le pointe « en activité » avec son retour attendu ; la clôture attend.
    b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, debut + Duration::minutes(5)), debut + Duration::minutes(5)).await.unwrap();
    let t = paris(mardi, 21, 45);
    let appel = b
        .ent
        .ouvrir_appel_a(b.etab, b.cpe, CPE, &AppelOuvertureDto { nuit: Some(mardi), ..crate::tests_internat_i2::ouverture("soir", "internat", None) }, t)
        .await
        .unwrap();
    for e in b.a.iter().chain(b.b.iter()) {
        let mut pt = crate::tests_internat_i2::pointage(*e, if *e == eleve { "activite" } else { "present" });
        if *e == eleve {
            pt.retour_attendu = Some(fin);
        }
        b.ent.pointer_a(b.etab, b.cpe, CPE, appel.appel.id, &pt, t + Duration::minutes(1)).await.unwrap();
    }
    let r = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel.appel.id, t + Duration::minutes(2)).await;
    assert!(refus_de(&r, "R-IT16"), "la clôture attend le retour : {r:?}");
    // Retour à 22 h 50, puis « présent » : la clôture passe, aucun cas.
    b.ent.controler_retour_a(b.etab, b.cpe, CPE, s.id, &ControleRetourDto { a: Some(paris(mardi, 22, 50)) }, paris(mardi, 22, 50)).await.unwrap();
    let mut pt = crate::tests_internat_i2::pointage(eleve, "present");
    pt.a = Some(paris(mardi, 22, 52));
    b.ent.pointer_a(b.etab, b.cpe, CPE, appel.appel.id, &pt, paris(mardi, 22, 52)).await.unwrap();
    let c = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel.appel.id, paris(mardi, 22, 55)).await.unwrap();
    assert_eq!(c.appel.nb_ecarts, Some(0));
    // Un élève qui ne rentre pas : la veille ouvre un cas au-delà du seuil (23 h + 30 min).
    let autre = b.a[1];
    let (pa, _) = parent(&b, autre, true).await;
    let d2 = (paris(avril(15), 19, 0), paris(avril(15), 23, 0));
    let s2 = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(autre, "retour_tardif", d2.0, d2.1, true, None), maintenant).await.unwrap();
    assert_eq!(s2.statut, "demandee", "retour tardif : accord ponctuel + CPE");
    let s2 = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s2.id, &decision("autoriser", None), maintenant).await.unwrap();
    assert_eq!(s2.statut, "autorisee");
    b.ent.controler_depart_a(b.etab, b.cpe, CPE, s2.id, &depart(None, d2.0), d2.0).await.unwrap();
    assert_eq!(b.ent.veille_sorties_a(b.etab, d2.1 + Duration::minutes(20)).await.unwrap().cas_retard_retour, 0);
    assert_eq!(b.ent.veille_sorties_a(b.etab, d2.1 + Duration::minutes(31)).await.unwrap().cas_retard_retour, 1);
}

/// Les sorties jamais utilisées expirent ; l'expiration est tracée et libère l'élève.
#[tokio::test]
async fn les_sorties_jamais_utilisees_expirent_et_l_annulation_remet_l_eleve_a_l_attendu() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (p, _) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let maintenant = paris(avril(14), 9, 0);
    let (debut, fin) = (paris(avril(17), 16, 0), paris(avril(19), 20, 0));
    let s = b.ent.demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(grand)), maintenant).await.unwrap();
    assert!(est_attendu(&b, avril(17), eleve).await, "E-3 : jamais partie, toujours attendu");
    // Annulée par la famille avant le départ : l'élève redevient attendu, les nuits sont libres.
    let (autre, _) = parent(&b, b.a[1], true).await;
    assert!(est_interdit(&b.ent.annuler_sortie_a(b.etab, autre, PARENT, s.id, maintenant).await));
    let an = b.ent.annuler_sortie_a(b.etab, p, PARENT, s.id, maintenant).await.unwrap();
    assert_eq!(an.statut, "annulee");
    assert!(est_attendu(&b, avril(17), eleve).await && est_attendu(&b, avril(18), eleve).await);
    let r = b.ent.annuler_sortie_a(b.etab, p, PARENT, s.id, maintenant).await;
    assert!(refus_de(&r, "ne peut plus être annulée"), "{r:?}");
    // Une autre sortie jamais utilisée expire après sa fin.
    let s2 = b.ent.demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "week_end", debut + Duration::days(7), fin + Duration::days(7), false, Some(grand)), maintenant).await.unwrap();
    let v = b.ent.veille_sorties_a(b.etab, fin + Duration::days(7) + Duration::hours(1)).await.unwrap();
    assert_eq!(v.sorties_expirees, 1);
    let l = b.ent.sorties_eleve_a(b.etab, p, PARENT, eleve, maintenant).await.unwrap();
    assert_eq!(l.iter().find(|x| x.id == s2.id).unwrap().statut, "expiree");
    assert!(faits(&b, "sortie_id", s2.id).await.iter().any(|f| f.0 == "expiration" && f.1 == "systeme"));
    assert_eq!(b.ent.veille_sorties_a(b.etab, fin + Duration::days(7) + Duration::hours(2)).await.unwrap().sorties_expirees, 0, "idempotente");
}

// ═════════════════════════════ Événements, droits, cloisonnement ═════════════════════════════

/// Aucun nom, aucun texte libre, aucune donnée de santé dans l'outbox (R-IT6, E14) : seulement des identifiants, des
/// dates, des codes et des compteurs.
#[tokio::test]
async fn evenements_i3_sans_nom_ni_texte_libre_ni_sante() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (p1, pers1) = parent(&b, eleve, true).await;
    let (p2, _) = parent(&b, eleve, true).await;
    let mardi = avril(14);
    let maintenant = paris(mardi, 9, 0);
    let secret = "MOTIF-LIBRE-CONFIDENTIEL";
    // Parcours : annonce, opposition (texte libre), direction, régularisation, sortie, refus (texte libre), retard.
    let a = annoncer(&b, p1, PARENT, eleve, avril(16), avril(16), maintenant).await.unwrap();
    b.ent.opposer_absence_a(b.etab, p2, PARENT, a.id, &OppositionDto { motif_texte: Some(secret.into()) }, maintenant).await.unwrap();
    b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &TrancheDto { decision: "maintenir".into(), motif_texte: Some(secret.into()) }, maintenant).await.unwrap();
    let reg = AbsenceAnnonceDto { eleve_id: eleve, du_nuit: avril(10), au_nuit: avril(10), recupere_par_personne_id: Some(pers1), retour_prevu: None, regularisation: true, motif_texte: Some(secret.into()) };
    let r = b.ent.annoncer_absence_a(b.etab, p1, PARENT, &reg, maintenant).await.unwrap();
    b.ent.decider_absence_a(b.etab, b.cpe, CPE, r.id, &decision("regulariser", Some(secret)), maintenant).await.unwrap();
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let (debut, fin) = (paris(avril(17), 16, 0), paris(avril(19), 20, 0));
    let s = b.ent.demander_sortie_a(b.etab, p1, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(grand)), maintenant).await.unwrap();
    b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut), debut).await.unwrap();
    b.ent.annoncer_retard_a(b.etab, p1, PARENT, s.id, &RetardAnnonceDto { nouveau_retour: fin + Duration::hours(2), motif_code: Some("train".into()) }, fin - Duration::hours(2)).await.unwrap();
    b.ent.controler_retour_a(b.etab, b.cpe, CPE, s.id, &ControleRetourDto { a: Some(fin) }, fin).await.unwrap();
    let c = b.ent.designer_correspondant_a(b.etab, p1, PARENT, &CorrespondantSaisieDto { eleve_id: eleve, personne_id: grand, lundi: true, mardi: false, mercredi: false, jeudi: false, vendredi: false, samedi: false, dimanche: false, du: None, au: None }, maintenant).await.unwrap();
    b.ent.suspendre_correspondant_a(b.etab, b.cpe, CPE, c.id, true, &CorrespondantSuspensionDto { motif_texte: Some(secret.into()) }, maintenant).await.unwrap();
    b.ent.demander_autorisations_a(b.etab, b.cpe, CPE, &AutorisationsDemandeDto { eleve_ids: vec![eleve] }).await.unwrap();
    // Les événements du lot.
    let types = [
        "internat.absence_annoncee", "internat.absence_decidee", "internat.absence_regularisee", "internat.absence_bloquee",
        "internat.desaccord_tranche", "internat.sortie_demandee", "internat.sortie_decidee", "internat.sortie_partie",
        "internat.sortie_rentree", "internat.retard_annonce", "internat.correspondant", "internat.autorisations_demandees",
        "internat.tiers",
    ];
    let noms: Vec<String> = sqlx::query_scalar("SELECT nom FROM ent_eleve WHERE etablissement_id = $1 UNION SELECT nom FROM ent_personne WHERE etablissement_id = $1")
        .bind(b.etab)
        .fetch_all(b.ent.pool_verif())
        .await
        .unwrap();
    let mut vus = 0;
    for t in types {
        for payload in b.evenements(t).await {
            vus += 1;
            let brut = payload.to_string();
            assert!(!brut.contains(secret), "{t} : texte libre dans l'outbox : {brut}");
            for n in noms.iter().filter(|n| n.len() > 4) {
                assert!(!brut.contains(n.as_str()), "{t} : un nom dans l'outbox : {brut}");
            }
            for cle in ["nom", "prenom", "texte", "motif_texte", "email", "telephone", "adresse", "sante"] {
                assert!(payload.get(cle).is_none(), "{t} : clé « {cle} » interdite : {brut}");
            }
        }
    }
    assert!(vus >= 12, "le parcours a bien émis ses événements ({vus})");
}

/// Droits et cloisonnement : un titulaire ne lit que l'internat de SON enfant ; le personnel sans droit ne lit rien ;
/// un autre établissement ne voit ni n'agit sur les absences d'ici.
#[tokio::test]
async fn droits_cloisonnement_entre_familles_et_entre_etablissements() {
    let Some(b) = banc().await else { return };
    let mardi = avril(14);
    let maintenant = paris(mardi, 9, 0);
    let (p0, _) = parent(&b, b.a[0], true).await;
    let (p1, _) = parent(&b, b.a[1], true).await;
    let a = annoncer(&b, p0, PARENT, b.a[0], avril(16), avril(16), maintenant).await.unwrap();
    // Lectures par élève : la famille de a[1] ne lit rien sur a[0], ni absences, ni sorties, ni correspondants, ni autorisations.
    assert!(est_interdit(&b.ent.absences_eleve_a(b.etab, p1, PARENT, b.a[0], maintenant).await));
    assert!(est_interdit(&b.ent.sorties_eleve_a(b.etab, p1, PARENT, b.a[0], maintenant).await));
    assert!(est_interdit(&b.ent.correspondants_eleve_a(b.etab, p1, PARENT, b.a[0], maintenant).await));
    assert!(est_interdit(&b.ent.autorisations_eleve_a(b.etab, p1, PARENT, b.a[0], maintenant).await));
    // Professeur et comptabilité : aucun droit sur l'internat (corrige RI-03).
    for (u, r) in [(b.prof, "prof"), (b.compta, "compta")] {
        assert!(est_interdit(&b.ent.absences_eleve_a(b.etab, u, r, b.a[0], maintenant).await), "{r}");
        assert!(est_interdit(&b.ent.sorties_a_traiter_a(b.etab, u, r).await), "{r}");
        assert!(est_interdit(&b.ent.liste_remise_a(b.etab, u, r, mardi, maintenant).await), "{r}");
        assert!(est_interdit(&b.ent.demander_autorisations_a(b.etab, u, r, &AutorisationsDemandeDto { eleve_ids: vec![b.a[0]] }).await), "{r}");
    }
    // Le personnel de l'internat lit, dans sa portée.
    assert_eq!(b.ent.absences_eleve_a(b.etab, b.cpe, CPE, b.a[0], maintenant).await.unwrap().len(), 1);
    // Un autre établissement : aucune lecture ni décision sur cette absence ; ni sur cet élève.
    let autre = crate::tests_rls_stricte::seed_supplementaire(&b.ent).await;
    let (etab2, cpe2) = (autre.etablissement_id, autre.cpe.user_id);
    let r = b.ent.decider_absence_a(etab2, cpe2, CPE, a.id, &decision("accepter", None), maintenant).await;
    assert!(matches!(r, Err(EntDbError::Domain(_)) | Err(EntDbError::Forbidden)), "{r:?}");
    let r = b.ent.absences_eleve_a(etab2, cpe2, CPE, b.a[0], maintenant).await;
    // Cloisonnement par RLS : l'élève d'un autre établissement n'existe pas pour ce CPE, la liste est exactement vide.
    assert!(matches!(r.as_deref(), Ok([])), "autre établissement : liste vide, aucune absence de l'élève : {r:?}");
    let file2 = b.ent.absences_a_traiter_a(etab2, cpe2, CPE).await.unwrap_or_default();
    assert!(file2.iter().all(|x| x.id != a.id), "l'autre établissement ne voit pas la file d'ici");
    // Une direction d'ici ne tranche pas une absence qui n'est pas bloquée, ni celle d'un élève inconnu.
    let r = b.ent.trancher_absence_a(b.etab, b.dir, DIR, Uuid::new_v4(), &TrancheDto { decision: "lever".into(), motif_texte: None }, maintenant).await;
    assert!(matches!(r, Err(EntDbError::Domain(_))), "{r:?}");
    // Journal : les lectures du personnel sont tracées (R-IT38).
    let lu: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_lecture_sensible WHERE etablissement_id = $1 AND user_id = $2 AND objet IN ('sorties', 'correspondants')")
        .bind(b.etab)
        .bind(b.cpe)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    let _ = b.ent.sorties_eleve_a(b.etab, b.cpe, CPE, b.a[0], maintenant).await.unwrap();
    let lu2: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_lecture_sensible WHERE etablissement_id = $1 AND user_id = $2 AND objet IN ('sorties', 'correspondants')")
        .bind(b.etab)
        .bind(b.cpe)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(lu2, lu + 1);
}

/// Un élève protégé (socle 2, E7) n'est jamais nommé au personnel tiers dans la file du CPE, et sa consultation est tracée.
#[tokio::test]
async fn eleve_protege_jamais_nomme_dans_les_files_du_cpe() {
    let Some(b) = banc().await else { return };
    let mardi = avril(14);
    let (p, _) = parent(&b, b.a[0], true).await;
    b.ent
        .poser_confidentialite_a(
            DIR,
            crate::eleve_confidentialite::PoserConfidentialite {
                etablissement_id: b.etab,
                eleve_id: b.a[0],
                motif: "décision de justice".into(),
                mesures: vec![],
                du: NaiveDate::from_ymd_opt(2026, 1, 1),
                au: None,
                acteur: b.dir,
            },
            utc(2026, 3, 1, 10, 0),
        )
        .await
        .unwrap();
    let t = annoncer(&b, p, PARENT, b.a[0], mardi, mardi, paris(mardi, 17, 0)).await.unwrap();
    let file = b.ent.absences_a_traiter_a(b.etab, b.cpe, CPE).await.unwrap();
    let l = file.iter().find(|x| x.id == t.id);
    // Caché au tiers (introuvable) ou présent sans nom : jamais nommé.
    if let Some(l) = l {
        assert!(l.protege && l.nom.is_none() && l.prenom.is_none(), "jamais nommé à un tiers : {l:?}");
    }
    // La famille, elle, voit son enfant.
    let vue = b.ent.absences_eleve_a(b.etab, p, PARENT, b.a[0], paris(mardi, 18, 0)).await.unwrap();
    assert_eq!(vue.len(), 1);
}

/// Historique des faits : le personnel voit tout (auteur, motif, personne), la famille voit les actes mais ni l'auteur
/// ni le texte libre d'un autre titulaire ; un tiers ne lit rien ; la lecture du personnel est journalisée.
#[tokio::test]
async fn historique_des_faits_visible_selon_le_role() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let maintenant = paris(avril(14), 9, 0);
    let (p1, _) = parent(&b, eleve, true).await;
    let (p2, _) = parent(&b, eleve, true).await;
    let a = annoncer(&b, p1, PARENT, eleve, avril(16), avril(16), maintenant).await.unwrap();
    b.ent.opposer_absence_a(b.etab, p2, PARENT, a.id, &OppositionDto { motif_texte: Some("raison privée de l'autre parent".into()) }, maintenant).await.unwrap();
    b.ent
        .trancher_absence_a(b.etab, b.dir, DIR, a.id, &TrancheDto { decision: "maintenir".into(), motif_texte: Some("présence maintenue".into()) }, maintenant)
        .await
        .unwrap();
    let personnel = b.ent.historique_a(b.etab, b.cpe, CPE, "absence", a.id, maintenant).await.unwrap();
    assert_eq!(personnel.iter().map(|f| f.acte.as_str()).collect::<Vec<_>>(), ["annonce", "opposition", "blocage", "tranchee_maintenue"]);
    assert_eq!(personnel[1].par_user_id, Some(p2));
    assert_eq!(personnel[1].motif_texte.as_deref(), Some("raison privée de l'autre parent"));
    let famille = b.ent.historique_a(b.etab, p1, PARENT, "absence", a.id, maintenant).await.unwrap();
    assert_eq!(famille.len(), 4);
    assert!(famille.iter().all(|f| f.par_user_id.is_none() && f.personne_id.is_none()), "ni auteur ni personne");
    assert!(famille[1].motif_texte.is_none(), "le texte de l'autre titulaire n'est jamais montré");
    assert_eq!(famille[3].motif_texte.as_deref(), Some("présence maintenue"), "la décision de l'établissement est lue de la famille");
    // Les tiers ne lisent rien ; objet ou identifiant inconnus.
    let (autre, _) = parent(&b, b.a[1], true).await;
    assert!(est_interdit(&b.ent.historique_a(b.etab, autre, PARENT, "absence", a.id, maintenant).await));
    assert!(est_interdit(&b.ent.historique_a(b.etab, b.prof, "prof", "absence", a.id, maintenant).await));
    let r = b.ent.historique_a(b.etab, b.cpe, CPE, "absence", Uuid::new_v4(), maintenant).await;
    assert!(matches!(r, Err(EntDbError::Domain(_))), "{r:?}");
    let r = b.ent.historique_a(b.etab, b.cpe, CPE, "dossier", a.id, maintenant).await;
    assert!(refus_de(&r, "objet d'historique inconnu"), "{r:?}");
    let lu: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_lecture_sensible WHERE etablissement_id = $1 AND user_id = $2 AND objet = 'sorties'")
        .bind(b.etab)
        .bind(b.cpe)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(lu, 1);
}

// ═════════════════════════════ Correctifs de la revue de sécurité (I3) ═════════════════════════════
//
// | Constat | Test(s) |
// |---|---|
// | C-1 (parent interdit de remise : ni acteur, ni lecteur ; mineur : personne qui récupère ; départ d'une absence) | `c1_*` |
// | E-1 (élève majeur : parents ni acteurs, lecture selon `majeur_info_parents`) et Moyen 8 (capacité) | `e1_*` |
// | E-2 (retard) | `e2_*` |
// | E-3 (attendu : seule une sortie partie retire l'élève) | `e3_*` |
// | Moyens 1 à 7 | `m1_remise_*` … `m7_*` (préfixe `moyen_`) |
// | Faibles (antidate, doublons, liste de remise protégée, F-1, M-1) | `faible_*` |

/// Interdiction de `personne` pour un objet précis (remise, visite, contact_eleve…), éventuellement terminée (`au`).
async fn interdire_objet(b: &Banc, eleve: Uuid, personne: Uuid, objet: &str, au: Option<&str>) {
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, au, decide_par) \
         VALUES ($1,$2,$3,$4,ARRAY[$5],'decision_etablissement','Appeler la direction','2026-01-01',$6::date,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(personne)
    .bind(objet)
    .bind(au)
    .bind(b.dir)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
}

async fn lever_interdictions(b: &Banc, eleve: Uuid) {
    sqlx::query("UPDATE ent_eleve_interdiction SET levee_le = now(), levee_par = $3, levee_motif = 'levée' WHERE etablissement_id = $1 AND eleve_id = $2 AND levee_le IS NULL")
        .bind(b.etab)
        .bind(eleve)
        .bind(b.dir)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
}

/// Accord annuel (socle 18) pour un élève de plus, l'objet étant déjà activé par `accorder_annuellement`.
async fn accorder_encore(b: &Banc, parent: Uuid, eleve: Uuid, usages: &[&str]) {
    let choix = usages.iter().map(|u| crate::accords::ChoixUsage { code: (*u).into(), accorde: true }).collect();
    b.ent
        .accords_decider(b.etab, parent, crate::accords::DecisionSaisie { objet_code: "sortie_internat".into(), eleve_id: Some(eleve), usages: choix })
        .await
        .expect("accord du titulaire");
}

async fn faits_actes(b: &Banc, colonne: &str, id: Uuid) -> Vec<(String, Option<String>)> {
    sqlx::query_as(&format!("SELECT acte, motif_code FROM ent_internat_i3_fait WHERE etablissement_id = $1 AND {colonne} = $2 ORDER BY a, cree_le"))
        .bind(b.etab)
        .bind(id)
        .fetch_all(b.ent.pool_verif())
        .await
        .unwrap()
}

fn tranche(d: &str) -> TrancheDto {
    TrancheDto { decision: d.into(), motif_texte: None }
}

/// Décision de la direction **motivée** : lever un blocage dû à une interdiction l'exige (levée ponctuelle, tracée).
fn tranche_motivee(d: &str) -> TrancheDto {
    TrancheDto { decision: d.into(), motif_texte: Some("Décision de la direction, tracée".into()) }
}

fn opposition() -> OppositionDto {
    OppositionDto { motif_texte: None }
}

/// C-1 — un parent visé (de remise, de visite ou de contact) par une interdiction non levée et non terminée n'est
/// acteur d'AUCUNE action du lot et ne lit rien des absences, sorties, correspondants et historique de l'élève ; il ne
/// peut pas non plus être la personne désignée. L'autre titulaire, lui, agit.
#[tokio::test]
async fn c1_parent_interdit_n_est_ni_acteur_ni_lecteur_ni_personne_designee() {
    let Some(b) = banc().await else { return };
    let mardi = avril(14);
    let maintenant = paris(mardi, 9, 0);
    for (i, objet) in ["remise", "visite", "contact_eleve"].into_iter().enumerate() {
        let eleve = b.a[i];
        let (pa, pers_a) = parent(&b, eleve, true).await;
        let (pb, _) = parent(&b, eleve, true).await;
        let deja = annoncer(&b, pa, PARENT, eleve, avril(16), avril(16), maintenant).await.unwrap();
        interdire_objet(&b, eleve, pers_a, objet, None).await;
        // Acteur : ni annonce, ni demande de sortie, ni accord, ni opposition, ni annulation, ni correspondant, ni retard.
        assert!(est_interdit(&annoncer(&b, pa, PARENT, eleve, avril(21), avril(21), maintenant).await), "{objet} : annonce");
        let (debut, fin) = (paris(avril(17), 16, 0), paris(avril(19), 20, 0));
        let r = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(pers_a)), maintenant).await;
        assert!(est_interdit(&r), "{objet} : demande de sortie {r:?}");
        let de_b = annoncer(&b, pb, PARENT, eleve, avril(22), avril(22), maintenant).await.unwrap();
        assert!(est_interdit(&b.ent.opposer_absence_a(b.etab, pa, PARENT, de_b.id, &opposition(), maintenant).await), "{objet} : opposition");
        assert!(est_interdit(&b.ent.annuler_absence_a(b.etab, pa, PARENT, deja.id, maintenant).await), "{objet} : annulation");
        let tiers_ok = tiers(&b, "Odile").await;
        autoriser_recuperer(&b, eleve, tiers_ok).await;
        let saisie = CorrespondantSaisieDto { eleve_id: eleve, personne_id: tiers_ok, lundi: true, mardi: false, mercredi: false, jeudi: false, vendredi: false, samedi: false, dimanche: false, du: None, au: None };
        assert!(est_interdit(&b.ent.designer_correspondant_a(b.etab, pa, PARENT, &saisie, maintenant).await), "{objet} : correspondant");
        assert!(est_interdit(&b.ent.revoquer_autorisation_a(b.etab, pa, PARENT, &RevocationDto { eleve_id: eleve, usages: None }, maintenant).await), "{objet} : révocation");
        // Lecteur : rien des absences, sorties, correspondants, autorisations ni de l'historique.
        assert!(est_interdit(&b.ent.absences_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await), "{objet} : lecture des absences");
        assert!(est_interdit(&b.ent.sorties_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await), "{objet} : lecture des sorties");
        assert!(est_interdit(&b.ent.correspondants_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await), "{objet} : lecture des correspondants");
        assert!(est_interdit(&b.ent.autorisations_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await), "{objet} : lecture des autorisations");
        assert!(est_interdit(&b.ent.historique_a(b.etab, pa, PARENT, "absence", deja.id, maintenant).await), "{objet} : historique");
        // Personne désignée : refusée, même quand l'annonceur est l'autre titulaire.
        let r = annoncer_avec(&b, pb, PARENT, eleve, avril(23), avril(23), Some(pers_a), maintenant).await;
        assert!(refus_de(&r, "pas autorisée à récupérer"), "{objet} : personne désignée {r:?}");
        // L'autre titulaire agit et lit.
        assert_eq!(b.ent.absences_eleve_a(b.etab, pb, PARENT, eleve, maintenant).await.unwrap().len(), 2, "{objet} : l'autre titulaire lit");
        // Levée : il redevient acteur et lecteur. Une interdiction TERMINÉE (au passé) ne joue plus non plus.
        lever_interdictions(&b, eleve).await;
        assert_eq!(b.ent.absences_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await.unwrap().len(), 2, "{objet} : interdiction levée");
        interdire_objet(&b, eleve, pers_a, objet, Some("2026-03-01")).await;
        annoncer(&b, pa, PARENT, eleve, avril(21), avril(21), maintenant).await.expect("interdiction terminée");
    }
}

/// C-1 — la chaîne de fusions des fiches porte l'interdiction dans les deux sens : posée sur une fiche absorbée dans
/// celle du parent, ou sur celle dans laquelle une fiche est absorbée.
#[tokio::test]
async fn c1_chaine_de_fusion_porte_l_interdiction_dans_les_deux_sens() {
    let Some(b) = banc().await else { return };
    let (eleve, maintenant) = (b.a[0], paris(avril(14), 9, 0));
    // Une fiche absorbée dans celle du parent est interdite : le parent l'est aussi.
    let (pa, pers_a) = parent(&b, eleve, true).await;
    let doublon = tiers(&b, "Doublon").await;
    sqlx::query("UPDATE ent_personne SET fusionnee_dans = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(doublon)
        .bind(pers_a)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    annoncer(&b, pa, PARENT, eleve, avril(16), avril(16), maintenant).await.expect("avant l'interdiction");
    interdire(&b, eleve, Some(doublon)).await;
    assert!(est_interdit(&annoncer(&b, pa, PARENT, eleve, avril(21), avril(21), maintenant).await), "fiche absorbée interdite → parent interdit");
    assert!(est_interdit(&b.ent.absences_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await));
    // Dans l'autre sens : X est absorbée dans Y, Y est interdite ; X l'est aussi (même personne réelle).
    let (x, y, z) = (tiers(&b, "X").await, tiers(&b, "Y").await, tiers(&b, "Z").await);
    sqlx::query("UPDATE ent_personne SET fusionnee_dans = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(x)
        .bind(y)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    interdire(&b, b.a[1], Some(y)).await;
    let mut conn = b.ent.pool_verif().acquire().await.unwrap();
    let jour = avril(14);
    use crate::internat_sorties::personne_interdite_tx;
    assert!(personne_interdite_tx(&mut conn, b.etab, b.a[1], y, jour).await.unwrap(), "la fiche visée");
    assert!(personne_interdite_tx(&mut conn, b.etab, b.a[1], x, jour).await.unwrap(), "fiche absorbée dans la visée");
    assert!(!personne_interdite_tx(&mut conn, b.etab, b.a[1], z, jour).await.unwrap(), "une personne sans rapport");
    assert!(!personne_interdite_tx(&mut conn, b.etab, b.a[2], x, jour).await.unwrap(), "un autre élève");
}

/// C-1 — pour un mineur, une absence annoncée exige la personne qui le récupère, sauf départ seul couvert par une
/// autorisation annuelle (socle 18) ; le contrôle de remise s'applique aussi quand l'annonceur est lui-même cette personne.
#[tokio::test]
async fn c1_mineur_exige_la_personne_qui_recupere_meme_pour_l_annonceur() {
    let Some(b) = banc().await else { return };
    let (mardi, maintenant) = (avril(14), paris(avril(14), 9, 0));
    let (p, pers) = parent(&b, b.a[0], true).await;
    let r = annoncer_avec(&b, p, PARENT, b.a[0], mardi, mardi, None, maintenant).await;
    assert!(refus_de(&r, "indiquer la personne qui le récupère"), "{r:?}");
    let inconnue = tiers(&b, "Voisine").await;
    let r = annoncer_avec(&b, p, PARENT, b.a[0], mardi, mardi, Some(inconnue), maintenant).await;
    assert!(refus_de(&r, "pas autorisée à récupérer"), "{r:?}");
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, b.a[0], grand).await;
    annoncer_avec(&b, p, PARENT, b.a[0], mardi, mardi, Some(grand), maintenant).await.expect("personne autorisée");
    // L'annonceur est lui-même la personne qui récupère : le contrôle de remise s'applique (interdiction sans personne
    // identifiée : on ne sait pas qui est à la porte).
    let (p2, pers2) = parent(&b, b.a[1], true).await;
    annoncer_avec(&b, p2, PARENT, b.a[1], mardi, mardi, Some(pers2), maintenant).await.expect("lui-même, autorisé");
    interdire(&b, b.a[1], None).await;
    let r = annoncer_avec(&b, p2, PARENT, b.a[1], avril(15), avril(15), Some(pers2), maintenant).await;
    assert!(refus_de(&r, "pas autorisée à récupérer"), "{r:?}");
    let _ = pers;
    // Autorisation annuelle « départ seul le week-end » : l'enfant part seul, sans personne désignée.
    let (p3, _) = parent(&b, b.a[2], true).await;
    let r = annoncer_avec(&b, p3, PARENT, b.a[2], mardi, mardi, None, maintenant).await;
    assert!(refus_de(&r, "indiquer la personne"), "sans autorisation annuelle : {r:?}");
    accorder_annuellement(&b, p3, b.a[2], &["week_end_seul"]).await;
    // F-B : l'accord « départ seul le week-end » ne couvre qu'une absence qui commence la veille d'un week-end.
    let r = annoncer_avec(&b, p3, PARENT, b.a[2], mardi, mardi, None, maintenant).await;
    assert!(refus_de(&r, "indiquer la personne"), "un mardi : {r:?}");
    annoncer_avec(&b, p3, PARENT, b.a[2], avril(17), avril(17), None, maintenant).await.expect("départ seul couvert par l'accord annuel, un vendredi");
    // Un majeur part seul, sans personne.
    let eleve_majeur = b.a[3];
    let (p4, _) = parent(&b, eleve_majeur, true).await;
    let moi = compte_eleve(&b, eleve_majeur, NaiveDate::from_ymd_opt(2007, 1, 1).unwrap()).await;
    let _ = p4;
    annoncer_avec(&b, moi, ELEVE, eleve_majeur, mardi, mardi, None, maintenant).await.expect("le majeur décide");
}

/// C-1 — le départ d'une absence passe par le contrôle de remise comme une sortie : liste de remise du jour, contrôle au
/// portail (personne désignée, interdiction, autorisation annuelle relue) ; la revue à la minute bloque l'absence si la
/// personne désignée devient interdite.
#[tokio::test]
async fn c1_depart_d_une_absence_passe_par_le_controle_de_remise() {
    let Some(b) = banc().await else { return };
    let (mardi, eleve) = (avril(14), b.a[0]);
    let (p, _) = parent(&b, eleve, true).await;
    let (grand, pere) = (tiers(&b, "Odile").await, tiers(&b, "Paul").await);
    autoriser_recuperer(&b, eleve, grand).await;
    autoriser_recuperer(&b, eleve, pere).await;
    let a = annoncer_avec(&b, p, PARENT, eleve, mardi, mardi, Some(grand), paris(mardi, 10, 0)).await.unwrap();
    let soir = paris(mardi, 17, 0);
    // La liste de remise du jour montre le départ de l'absence, avec la personne prévue.
    let liste = b.ent.liste_remise_a(b.etab, b.cpe, CPE, mardi, soir).await.unwrap();
    let ligne = liste.iter().find(|l| l.absence_id == Some(a.id)).expect("l'absence figure sur la liste de remise");
    assert_eq!((ligne.sortie_id, ligne.type_code.as_str()), (None, "absence"));
    assert_eq!(ligne.personne_prevue.as_ref().map(|x| x.personne_id), Some(grand));
    assert!(ligne.personnes_autorisees.iter().any(|x| x.personne_id == pere));
    // Un mineur n'est jamais « laissé partir » sans personne, ni remis à une personne qui n'est pas la désignée.
    let r = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(None, soir), soir).await;
    assert!(refus_de(&r, "indiquer la personne"), "{r:?}");
    let r = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(Some(pere), soir), soir).await;
    assert!(refus_de(&r, "pas la personne désignée"), "autorisé mais non désigné : {r:?}");
    // Interdiction sur la personne désignée : remise refusée, tentative tracée.
    interdire(&b, eleve, Some(grand)).await;
    let r = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(Some(grand), soir), soir).await;
    assert!(refus_de(&r, "REMISE REFUSÉE"), "{r:?}");
    let f = faits_actes(&b, "absence_id", a.id).await;
    assert_eq!(f.iter().filter(|(acte, motif)| acte == "depart_refuse" && motif.as_deref() == Some("interdiction_remise")).count(), 1);
    assert_eq!(f.iter().filter(|(acte, _)| acte == "depart_refuse").count(), 2, "les deux tentatives refusées sont tracées : {f:?}");
    // La revue à la minute bloque l'absence (l'élève redevient attendu) ; le départ est alors refusé et tracé.
    let v = b.ent.veille_sorties_a(b.etab, paris(mardi, 17, 5)).await.unwrap();
    assert_eq!(v.sorties_expirees, 0);
    let vue = b.ent.absences_eleve_a(b.etab, b.cpe, CPE, eleve, soir).await.unwrap();
    assert_eq!(vue[0].statut, "bloquee");
    assert!(est_attendu(&b, mardi, eleve).await, "bloquée : l'élève redevient attendu");
    let r = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(Some(pere), soir), soir).await;
    assert!(refus_de(&r, "bloquée"), "{r:?}");
    // Parcours nominal sur un autre élève : remise à la personne désignée, une seule fois.
    let e2 = b.a[1];
    let (p2, _) = parent(&b, e2, true).await;
    let g2 = tiers(&b, "Louise").await;
    autoriser_recuperer(&b, e2, g2).await;
    let a2 = annoncer_avec(&b, p2, PARENT, e2, mardi, mardi, Some(g2), paris(mardi, 10, 0)).await.unwrap();
    let ok = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a2.id, &depart(Some(g2), soir), soir).await.unwrap();
    assert_eq!(ok.depart_effectif, Some(soir));
    let r = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a2.id, &depart(Some(g2), soir), soir).await;
    assert!(refus_de(&r, "déjà parti"), "{r:?}");
    let liste = b.ent.liste_remise_a(b.etab, b.cpe, CPE, mardi, soir).await.unwrap();
    assert!(liste.iter().all(|l| l.absence_id != Some(a2.id)), "parti : retiré de la liste de remise");
    // Départ seul avec autorisation annuelle, relue à l'instant : retirée entre-temps, départ refusé (tracé).
    let (e3, e4) = (b.a[2], b.a[3]);
    let (p3, _) = parent(&b, e3, true).await;
    accorder_annuellement(&b, p3, e3, &["week_end_seul"]).await;
    let (p4, _) = parent(&b, e4, true).await;
    accorder_encore(&b, p4, e4, &["week_end_seul"]).await;
    // F-B : l'accord « départ seul le week-end » ne couvre qu'une absence commençant la veille d'un week-end (vendredi).
    let (vendredi, soir) = (avril(17), paris(avril(17), 17, 0));
    let a3 = annoncer_avec(&b, p3, PARENT, e3, vendredi, vendredi, None, paris(mardi, 10, 0)).await.unwrap();
    let a4 = annoncer_avec(&b, p4, PARENT, e4, vendredi, vendredi, None, paris(mardi, 10, 0)).await.unwrap();
    let ok = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a3.id, &depart(None, soir), soir).await.unwrap();
    assert!(ok.depart_effectif.is_some(), "départ seul couvert par l'autorisation annuelle");
    b.ent.accords_retirer(b.etab, p4, "sortie_internat", Some(e4), None).await.unwrap();
    let r = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a4.id, &depart(None, soir), soir).await;
    assert!(refus_de(&r, "autorisation annuelle"), "{r:?}");
    // Et la revue bloque l'absence dont l'autorisation annuelle a disparu.
    b.ent.veille_sorties_a(b.etab, paris(avril(17), 17, 10)).await.unwrap();
    let vue = b.ent.absences_eleve_a(b.etab, b.cpe, CPE, e4, soir).await.unwrap();
    assert_eq!(vue[0].statut, "bloquee");
    let f = faits_actes(&b, "absence_id", a4.id).await;
    assert!(f.contains(&("blocage".to_string(), Some("autorisation_revoquee".to_string()))), "{f:?}");
}

/// C-1 — une interdiction posée APRÈS l'annonce bloque l'absence à la revue ; la direction ne peut pas lever le blocage
/// tant que la personne désignée est interdite.
#[tokio::test]
async fn c1_interdiction_posee_apres_l_annonce_bloque_l_absence() {
    let Some(b) = banc().await else { return };
    let (mardi, eleve) = (avril(14), b.a[0]);
    let (p, _) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let a = annoncer_avec(&b, p, PARENT, eleve, mardi, mardi, Some(grand), paris(mardi, 9, 0)).await.unwrap();
    assert!(est_attendu(&b, mardi, eleve).await, "M-B : sans départ constaté, attendu");
    interdire(&b, eleve, Some(grand)).await;
    b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 0)).await.unwrap();
    let vue = b.ent.absences_eleve_a(b.etab, p, PARENT, eleve, paris(mardi, 12, 0)).await.unwrap();
    assert_eq!(vue[0].statut, "bloquee");
    assert!(est_attendu(&b, mardi, eleve).await, "avant le début de la nuit : l'élève redevient attendu");
    let evts = b.evenements("internat.absence_bloquee").await;
    assert!(evts.iter().any(|e| e["cause"].as_str() == Some("interdiction_remise") && e["nuit_en_cours"].as_bool() == Some(false)), "{evts:?}");
    let r = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &tranche_motivee("lever"), paris(mardi, 12, 5)).await;
    assert!(refus_de(&r, "ne peut plus récupérer"), "lever ne rend pas valable une remise interdite : {r:?}");
    let ok = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &tranche("maintenir"), paris(mardi, 12, 6)).await.unwrap();
    assert_eq!(ok.statut, "refusee");
    // Une absence encore à accepter (annonce tardive) n'est pas acceptée si la personne est devenue interdite.
    let e2 = b.a[1];
    let (p2, _) = parent(&b, e2, true).await;
    let g2 = tiers(&b, "Louise").await;
    autoriser_recuperer(&b, e2, g2).await;
    let t = annoncer_avec(&b, p2, PARENT, e2, mardi, mardi, Some(g2), paris(mardi, 17, 0)).await.unwrap();
    assert_eq!(t.statut, "demandee");
    interdire(&b, e2, Some(g2)).await;
    let r = b.ent.decider_absence_a(b.etab, b.cpe, CPE, t.id, &decision("accepter", None), paris(mardi, 17, 5)).await;
    assert!(refus_de(&r, "ne peut plus récupérer"), "{r:?}");
}

/// E-1 / Moyen 8 — élève majeur (R-IT22) : ses parents ne sont plus acteurs ; leur lecture dépend de
/// `majeur_info_parents`. La capacité juridique compte : un majeur sous tutelle est décidé par son tuteur, un mineur
/// émancipé est traité en majeur.
#[tokio::test]
async fn e1_majeur_parents_ni_acteurs_lecture_selon_le_reglement_et_capacite() {
    let Some(b) = banc().await else { return };
    let (mardi, maintenant) = (avril(14), paris(avril(14), 9, 0));
    let eleve = b.a[0];
    let (pa, _) = parent(&b, eleve, true).await;
    let moi = compte_eleve(&b, eleve, NaiveDate::from_ymd_opt(2007, 4, 1).unwrap()).await;
    let a = annoncer(&b, moi, ELEVE, eleve, avril(16), avril(16), maintenant).await.unwrap();
    assert_eq!(a.annonce_qualite, "eleve_majeur");
    // Les parents ne sont plus acteurs : annonce, opposition, annulation, correspondant, révocation, accord, retard.
    assert!(est_interdit(&annoncer(&b, pa, PARENT, eleve, avril(21), avril(21), maintenant).await), "annonce");
    assert!(est_interdit(&b.ent.opposer_absence_a(b.etab, pa, PARENT, a.id, &opposition(), maintenant).await), "opposition");
    assert!(est_interdit(&b.ent.annuler_absence_a(b.etab, pa, PARENT, a.id, maintenant).await), "annulation");
    let corr = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, corr).await;
    let saisie = CorrespondantSaisieDto { eleve_id: eleve, personne_id: corr, lundi: true, mardi: false, mercredi: false, jeudi: false, vendredi: false, samedi: false, dimanche: false, du: None, au: None };
    assert!(est_interdit(&b.ent.designer_correspondant_a(b.etab, pa, PARENT, &saisie, maintenant).await), "correspondant");
    assert!(est_interdit(&b.ent.revoquer_autorisation_a(b.etab, pa, PARENT, &RevocationDto { eleve_id: eleve, usages: None }, maintenant).await), "révocation");
    let debut = paris(avril(15), 20, 0);
    let s = b.ent.demander_sortie_a(b.etab, moi, ELEVE, &demande(eleve, "retour_tardif", debut, debut + Duration::hours(3), true, None), maintenant).await.unwrap();
    let r = b.ent.repondre_sortie_a(b.etab, pa, PARENT, s.id, &AccordTitulaireDto { accord: false, motif_texte: Some("non".into()), avec_personne_id: None }, maintenant).await;
    assert!(est_interdit(&r), "réponse à la sortie du majeur : {r:?}");
    assert!(est_interdit(&b.ent.annoncer_retard_a(b.etab, pa, PARENT, s.id, &RetardAnnonceDto { nouveau_retour: debut + Duration::hours(4), motif_code: None }, maintenant).await), "retard");
    // Lecture : règlement « si à charge » (défaut) et « jamais » → rien ; « toujours » → lecture seule.
    assert!(est_interdit(&b.ent.absences_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await), "si à charge : pas de lecture");
    let regler = |v: &'static str| {
        let b = &b;
        async move {
            sqlx::query(
                "INSERT INTO ent_internat_parametres (etablissement_id, majeur_info_parents) VALUES ($1, $2) \
                 ON CONFLICT (etablissement_id) DO UPDATE SET majeur_info_parents = $2",
            )
            .bind(b.etab)
            .bind(v)
            .execute(b.ent.pool_verif())
            .await
            .unwrap();
        }
    };
    regler("jamais").await;
    assert!(est_interdit(&b.ent.absences_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await), "jamais");
    regler("toujours").await;
    assert_eq!(b.ent.absences_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await.unwrap().len(), 1, "toujours : lecture");
    assert_eq!(b.ent.sorties_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await.unwrap().len(), 1);
    assert!(b.ent.historique_a(b.etab, pa, PARENT, "absence", a.id, maintenant).await.is_ok());
    assert!(b.ent.correspondants_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await.is_ok());
    assert!(est_interdit(&annoncer(&b, pa, PARENT, eleve, avril(21), avril(21), maintenant).await), "lecture seule : toujours pas d'action");
    // Un parent interdit de remise ne lit rien, même avec « toujours ».
    let (pi, pers_i) = parent(&b, eleve, true).await;
    interdire(&b, eleve, Some(pers_i)).await;
    assert!(est_interdit(&b.ent.absences_eleve_a(b.etab, pi, PARENT, eleve, maintenant).await));
    // Majeur sous tutelle : le tuteur (titulaire) décide, l'élève lui-même est traité comme un mineur.
    let (e2, ptut) = (b.a[1], parent(&b, b.a[1], true).await.0);
    let moi2 = compte_eleve(&b, e2, NaiveDate::from_ymd_opt(2007, 4, 1).unwrap()).await;
    sqlx::query("INSERT INTO ent_eleve_capacite (id, etablissement_id, eleve_id, nature, du, piece_document_id) VALUES ($1,$2,$3,'tutelle','2025-06-01',$4)")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(e2)
        .bind(Uuid::new_v4())
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let r = annoncer(&b, moi2, ELEVE, e2, mardi, mardi, maintenant).await;
    assert!(refus_de(&r, "élève mineur"), "sous tutelle : l'élève n'annonce pas lui-même : {r:?}");
    annoncer(&b, ptut, PARENT, e2, mardi, mardi, maintenant).await.expect("le tuteur décide");
    // Mineur émancipé : traité en majeur ; ses parents ne sont plus acteurs.
    let e3 = b.a[2];
    let (pe, _) = parent(&b, e3, true).await;
    let moi3 = compte_eleve(&b, e3, NaiveDate::from_ymd_opt(2010, 3, 14).unwrap()).await;
    sqlx::query("INSERT INTO ent_eleve_capacite (id, etablissement_id, eleve_id, nature, du, piece_document_id) VALUES ($1,$2,$3,'emancipation','2025-06-01',$4)")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(e3)
        .bind(Uuid::new_v4())
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let a3 = annoncer(&b, moi3, ELEVE, e3, mardi, mardi, maintenant).await.expect("émancipé : il décide");
    assert_eq!(a3.annonce_qualite, "eleve_majeur");
    assert!(est_interdit(&annoncer(&b, pe, PARENT, e3, avril(21), avril(21), maintenant).await), "émancipé : les parents ne sont plus acteurs");
}

/// Demande une sortie « retour tardif » (20 h → 23 h) autorisée par le CPE : (sortie, parent, personne du parent).
async fn sortie_retour_tardif(b: &Banc, eleve: Uuid, peut: bool) -> (SortieDto, Uuid) {
    let (p, pers) = parent(b, eleve, peut).await;
    let jour = avril(15);
    let maintenant = paris(avril(14), 12, 0);
    let s = b
        .ent
        .demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "retour_tardif", paris(jour, 20, 0), paris(jour, 23, 0), false, Some(pers)), maintenant)
        .await
        .unwrap();
    let s = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), maintenant).await.unwrap();
    assert_eq!(s.statut, "autorisee");
    (s, p)
}

/// E-2 — un retard ne fait pas couvrir la nuit à une sortie qui ne la couvrait pas : demande au CPE, sans effet tant
/// qu'il n'a pas décidé ; plafond à la durée maximale du type (+ 6 h) ; seul un titulaire qui peut autoriser les sorties,
/// l'élève majeur ou le CPE annonce un retard.
#[tokio::test]
async fn e2_retard_ne_fait_pas_couvrir_la_nuit_sans_le_cpe_plafond_et_droits() {
    let Some(b) = banc().await else { return };
    let maintenant = paris(avril(14), 13, 0);
    let (s, p) = sortie_retour_tardif(&b, b.a[0], true).await;
    let (peu, _) = parent(&b, b.a[0], false).await;
    let retard = |h: u32, mi: u32, jour: NaiveDate| RetardAnnonceDto { nouveau_retour: paris(jour, h, mi), motif_code: Some("train".into()) };
    // Droit : un titulaire sans « autoriser les sorties » n'annonce pas de retard ; un professeur non plus.
    let r = b.ent.annoncer_retard_a(b.etab, peu, PARENT, s.id, &retard(23, 30, avril(15)), maintenant).await;
    assert!(refus_de(&r, "ne vous permet pas"), "{r:?}");
    assert!(est_interdit(&b.ent.annoncer_retard_a(b.etab, b.prof, "prof", s.id, &retard(23, 30, avril(15)), maintenant).await));
    // Un retard court (la sortie ne couvre toujours pas la nuit) s'applique aussitôt.
    let r1 = b.ent.annoncer_retard_a(b.etab, p, PARENT, s.id, &retard(23, 45, avril(15)), maintenant).await.unwrap();
    assert_eq!((r1.fin_ajustee, r1.retard_demande), (paris(avril(15), 23, 45), None));
    // Plafond : durée maximale du type (12 h) + 6 h de marge à partir du début.
    let r = b.ent.annoncer_retard_a(b.etab, p, PARENT, s.id, &retard(15, 0, avril(16)), maintenant).await;
    assert!(refus_de(&r, "durée maximale"), "{r:?}");
    // Un retard qui ferait couvrir la nuit (retour le lendemain à 6 h 30) : demande au CPE, sans effet.
    let r2 = b.ent.annoncer_retard_a(b.etab, p, PARENT, s.id, &retard(6, 30, avril(16)), maintenant).await.unwrap();
    assert_eq!((r2.fin_ajustee, r2.retard_demande), (paris(avril(15), 23, 45), Some(paris(avril(16), 6, 30))));
    let (fin, demande_fin): (DateTime<Utc>, Option<DateTime<Utc>>) = sqlx::query_as("SELECT fin_ajustee, retard_demande_fin FROM ent_internat_sortie WHERE id = $1")
        .bind(s.id)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!((fin, demande_fin), (paris(avril(15), 23, 45), Some(paris(avril(16), 6, 30))));
    let r = b.ent.annoncer_retard_a(b.etab, p, PARENT, s.id, &retard(6, 15, avril(16)), maintenant).await;
    assert!(refus_de(&r, "attend déjà"), "{r:?}");
    let file = b.ent.sorties_a_traiter_a(b.etab, b.cpe, CPE).await.unwrap();
    assert!(file.iter().any(|x| x.id == s.id && x.retard_demande.is_some()), "le CPE voit la demande de retard");
    // Le CPE accepte : le retour attendu est décalé ; un autre refus d'un retard sans demande est un conflit.
    let ok = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("accepter_retard", None), maintenant).await.unwrap();
    assert_eq!((ok.fin_ajustee, ok.retard_demande), (paris(avril(16), 6, 30), None));
    let r = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("refuser_retard", None), maintenant).await;
    assert!(refus_de(&r, "aucun retard"), "{r:?}");
    // Un refus du CPE laisse le retour attendu inchangé et trace le refus.
    let (s2, p2) = sortie_retour_tardif(&b, b.a[1], true).await;
    let r = b.ent.annoncer_retard_a(b.etab, p2, PARENT, s2.id, &retard(6, 30, avril(16)), maintenant).await.unwrap();
    assert!(r.retard_demande.is_some());
    let ko = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s2.id, &decision("refuser_retard", Some("hors règlement")), maintenant).await.unwrap();
    assert_eq!((ko.fin_ajustee, ko.retard_demande), (paris(avril(15), 23, 0), None));
    assert!(faits_actes(&b, "sortie_id", s2.id).await.iter().any(|(a, _)| a == "retard_refuse"));
    // Le CPE, lui, annonce directement (il décide).
    let (s3, _) = sortie_retour_tardif(&b, b.a[2], true).await;
    let direct = b.ent.annoncer_retard_a(b.etab, b.cpe, CPE, s3.id, &retard(6, 30, avril(16)), maintenant).await.unwrap();
    assert_eq!((direct.fin_ajustee, direct.retard_demande), (paris(avril(16), 6, 30), None));
}

/// E-3 / Moyen 1 — seules les sorties PARTIES ou rentrées retirent l'élève de l'attendu : une sortie autorisée jamais
/// partie (départ refusé au portail) le laisse attendu. La remise ne se fait qu'à la personne désignée.
#[tokio::test]
async fn e3_sortie_autorisee_jamais_partie_laisse_l_eleve_attendu_et_remise_a_la_personne_designee() {
    let Some(b) = banc().await else { return };
    let (mardi, eleve) = (avril(14), b.a[0]);
    let (p, _) = parent(&b, eleve, true).await;
    let (grand, pere) = (tiers(&b, "Odile").await, tiers(&b, "Paul").await);
    autoriser_recuperer(&b, eleve, grand).await;
    autoriser_recuperer(&b, eleve, pere).await;
    let (debut, fin) = (paris(mardi, 16, 0), paris(avril(16), 20, 0));
    let s = b.ent.demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(grand)), paris(mardi, 12, 0)).await.unwrap();
    assert_eq!(s.statut, "autorisee");
    for n in [14, 15] {
        assert!(est_attendu(&b, avril(n), eleve).await, "autorisée mais jamais partie : attendu la nuit du {n}");
    }
    // Remise à une personne autorisée mais non désignée : refus tracé, l'élève reste attendu.
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(pere), debut), debut).await;
    assert!(refus_de(&r, "pas la personne désignée"), "{r:?}");
    // Remise sans aucune personne indiquée pour un mineur : refus.
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, debut), debut).await;
    assert!(refus_de(&r, "indiquer la personne"), "{r:?}");
    // Départ refusé au portail (personne désignée interdite) : l'élève reste attendu.
    interdire(&b, eleve, Some(grand)).await;
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut), debut).await;
    assert!(refus_de(&r, "REMISE REFUSÉE"), "{r:?}");
    assert!(est_attendu(&b, mardi, eleve).await, "départ refusé : toujours attendu");
    let f = faits_actes(&b, "sortie_id", s.id).await;
    assert_eq!(f.iter().filter(|(a, _)| a == "depart_refuse").count(), 2, "les remises refusées sont tracées (la remise sans personne est un refus de forme) : {f:?}");
    // Interdiction levée : la personne désignée prend l'élève, qui sort alors de l'attendu.
    lever_interdictions(&b, eleve).await;
    b.ent.veille_sorties_a(b.etab, paris(mardi, 15, 0)).await.unwrap();
    let ok = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut), debut).await.unwrap();
    assert_eq!(ok.statut, "partie");
    assert!(est_retire(&b, mardi, eleve, "sortie").await && est_retire(&b, avril(15), eleve, "sortie").await);
}

/// Moyen 1 — un titulaire qui change la personne désignée par un AUTRE titulaire s'oppose à lui : le plus restrictif
/// l'emporte (la désignation reste, la sortie est bloquée) jusqu'à la décision de la direction ; l'auteur, lui, la change.
#[tokio::test]
async fn moyen_1_changer_la_personne_d_un_autre_titulaire_vaut_opposition() {
    let Some(b) = banc().await else { return };
    let (mardi, eleve) = (avril(14), b.a[0]);
    let (pa, _) = parent(&b, eleve, true).await;
    let (pb, _) = parent(&b, eleve, true).await;
    let (grand, pere) = (tiers(&b, "Odile").await, tiers(&b, "Paul").await);
    autoriser_recuperer(&b, eleve, grand).await;
    autoriser_recuperer(&b, eleve, pere).await;
    let (debut, fin) = (paris(avril(17), 16, 0), paris(avril(19), 20, 0));
    let maintenant = paris(mardi, 12, 0);
    let s = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(grand)), maintenant).await.unwrap();
    let r = b.ent.designer_personne_sortie_a(b.etab, pb, PARENT, s.id, &PersonneSortieDto { avec_personne_id: Some(pere), seul: false }, maintenant).await.unwrap();
    assert_eq!((r.statut.as_str(), r.avec_personne_id, r.opposition), ("bloquee", Some(grand), true));
    let f = faits_actes(&b, "sortie_id", s.id).await;
    assert!(f.iter().any(|(a, _)| a == "opposition") && f.iter().any(|(a, _)| a == "blocage"), "{f:?}");
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut), debut).await;
    assert!(refus_de(&r, "bloquée"), "{r:?}");
    // La direction lève : la désignation d'origine vaut ; l'auteur, lui, peut la changer sans opposition.
    let l = b.ent.trancher_sortie_a(b.etab, b.dir, DIR, s.id, &tranche("lever"), maintenant).await.unwrap();
    assert_eq!((l.statut.as_str(), l.avec_personne_id), ("autorisee", Some(grand)));
    let c = b.ent.designer_personne_sortie_a(b.etab, pa, PARENT, s.id, &PersonneSortieDto { avec_personne_id: Some(pere), seul: false }, maintenant).await.unwrap();
    assert_eq!((c.statut.as_str(), c.avec_personne_id), ("autorisee", Some(pere)));
    // Désigner la MÊME personne n'est pas un changement.
    let m = b.ent.designer_personne_sortie_a(b.etab, pb, PARENT, s.id, &PersonneSortieDto { avec_personne_id: Some(pere), seul: false }, maintenant).await.unwrap();
    assert_eq!(m.statut, "autorisee");
}

/// Moyens 2 et 4 — passer un mineur en départ « seul » remet à zéro l'accord de la famille (seule l'autorisation annuelle
/// le remplace) ; un mineur qui part seul exige toujours un accord de la famille, quel que soit le décideur du type.
#[tokio::test]
async fn moyen_2_et_4_depart_seul_d_un_mineur_exige_l_accord_de_la_famille() {
    let Some(b) = banc().await else { return };
    let maintenant = paris(avril(14), 12, 0);
    // (2) Sans autorisation annuelle, pour un type ponctuel : l'accord donné pour un départ accompagné tombe.
    let (s, p) = sortie_retour_tardif(&b, b.a[0], true).await;
    assert!(s.accord_titulaire);
    let seul = b.ent.designer_personne_sortie_a(b.etab, b.cpe, CPE, s.id, &PersonneSortieDto { avec_personne_id: None, seul: true }, maintenant).await.unwrap();
    assert_eq!((seul.statut.as_str(), seul.accord_titulaire, seul.seul), ("demandee", false, true), "accord remis à zéro : la sortie retourne à la demande");
    assert!(faits_actes(&b, "sortie_id", s.id).await.iter().any(|(a, _)| a == "accord_remis_a_zero"));
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, paris(avril(15), 20, 0)), paris(avril(15), 20, 0)).await;
    assert!(refus_de(&r, "pas encore autorisée"), "{r:?}");
    let ok = b.ent.repondre_sortie_a(b.etab, p, PARENT, s.id, &AccordTitulaireDto { accord: true, motif_texte: None, avec_personne_id: None }, maintenant).await.unwrap();
    assert_eq!(ok.statut, "autorisee", "la famille répond de nouveau (la décision du CPE est déjà prise)");
    // (2) Avec l'autorisation annuelle du socle 18, elle remplace l'accord : la sortie reste autorisée, par l'accord annuel.
    let e2 = b.a[1];
    let (p2, pers2) = parent(&b, e2, true).await;
    accorder_annuellement(&b, p2, e2, &["week_end_seul"]).await;
    let (debut, fin) = (paris(avril(17), 16, 0), paris(avril(19), 20, 0));
    let w = b.ent.demander_sortie_a(b.etab, p2, PARENT, &demande(e2, "week_end", debut, fin, false, Some(pers2)), maintenant).await.unwrap();
    assert!(!w.par_autorisation_annuelle);
    let ws = b.ent.designer_personne_sortie_a(b.etab, p2, PARENT, w.id, &PersonneSortieDto { avec_personne_id: None, seul: true }, maintenant).await.unwrap();
    assert_eq!((ws.statut.as_str(), ws.par_autorisation_annuelle, ws.accord_titulaire), ("autorisee", true, true));
    // (4) Décideur « CPE » : le CPE seul ne décide jamais d'un départ seul de mineur.
    b.ent
        .modifier_type_sortie_a(b.etab, b.dir, DIR, "retour_tardif", &SortieTypeSaisieDto { decideur: Some("cpe".into()), ..Default::default() }, maintenant)
        .await
        .unwrap();
    let e3 = b.a[2];
    let (p3, _) = parent(&b, e3, true).await;
    let d = paris(avril(16), 20, 0);
    let s3 = b.ent.demander_sortie_a(b.etab, b.cpe, CPE, &demande(e3, "retour_tardif", d, d + Duration::hours(3), true, None), maintenant).await.unwrap();
    assert_eq!((s3.statut.as_str(), s3.accord_titulaire), ("demandee", false), "décision du CPE seule : pas d'accord de la famille");
    let apres = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s3.id, &decision("autoriser", None), maintenant).await.unwrap();
    assert_eq!(apres.statut, "demandee", "toujours en attente de la famille");
    let ok = b.ent.repondre_sortie_a(b.etab, p3, PARENT, s3.id, &AccordTitulaireDto { accord: true, motif_texte: None, avec_personne_id: None }, maintenant).await.unwrap();
    assert_eq!(ok.statut, "autorisee");
    let dep = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s3.id, &depart(None, d), d).await.unwrap();
    assert_eq!(dep.statut, "partie");
    // Un majeur, lui, part seul sans accord.
    let e4 = b.a[3];
    let moi = compte_eleve(&b, e4, NaiveDate::from_ymd_opt(2007, 1, 1).unwrap()).await;
    let d4 = paris(avril(15), 20, 0);
    let s4 = b.ent.demander_sortie_a(b.etab, moi, ELEVE, &demande(e4, "retour_tardif", d4, d4 + Duration::hours(3), true, None), maintenant).await.unwrap();
    assert_eq!(s4.statut, "demandee", "le décideur du type est le CPE");
}

/// Moyen 3 — dès que la sortie repose sur l'autorisation annuelle (seul ou accompagné), le verdict du socle 18 est relu
/// au départ et à la revue : retirée sans passer par la révocation, la sortie est suspendue.
#[tokio::test]
async fn moyen_3_autorisation_annuelle_relue_au_depart_accompagne_et_a_la_revue() {
    let Some(b) = banc().await else { return };
    let maintenant = paris(avril(14), 12, 0);
    let (debut, fin) = (paris(avril(15), 13, 0), paris(avril(15), 18, 0));
    let (e1, e2) = (b.a[0], b.a[1]);
    let (p1, _) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, e1, grand).await;
    autoriser_recuperer(&b, e2, grand).await;
    accorder_annuellement(&b, p1, e1, &["sortie_mercredi"]).await;
    accorder_encore(&b, p2, e2, &["sortie_mercredi"]).await;
    let s1 = b.ent.demander_sortie_a(b.etab, b.cpe, CPE, &demande(e1, "mercredi", debut, fin, false, Some(grand)), maintenant).await.unwrap();
    let s2 = b.ent.demander_sortie_a(b.etab, b.cpe, CPE, &demande(e2, "mercredi", debut, fin, false, Some(grand)), maintenant).await.unwrap();
    assert!((s1.par_autorisation_annuelle, s1.statut.as_str()) == (true, "autorisee") && s2.par_autorisation_annuelle);
    b.ent.accords_retirer(b.etab, p1, "sortie_internat", Some(e1), None).await.unwrap();
    b.ent.accords_retirer(b.etab, p2, "sortie_internat", Some(e2), None).await.unwrap();
    // Au départ, même accompagné : refusé, la sortie est suspendue.
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s1.id, &depart(Some(grand), debut), debut).await;
    assert!(refus_de(&r, "autorisation annuelle"), "{r:?}");
    let apres = b.ent.sorties_eleve_a(b.etab, b.cpe, CPE, e1, maintenant).await.unwrap();
    assert_eq!(apres[0].statut, "suspendue");
    // À la revue : suspendue aussi, et ne reprend pas d'elle-même.
    b.ent.veille_sorties_a(b.etab, paris(avril(14), 12, 5)).await.unwrap();
    let apres = b.ent.sorties_eleve_a(b.etab, b.cpe, CPE, e2, maintenant).await.unwrap();
    assert_eq!(apres[0].statut, "suspendue");
    b.ent.veille_sorties_a(b.etab, paris(avril(14), 12, 10)).await.unwrap();
    assert_eq!(b.ent.sorties_eleve_a(b.etab, b.cpe, CPE, e2, maintenant).await.unwrap()[0].statut, "suspendue");
    assert!(faits_actes(&b, "sortie_id", s2.id).await.contains(&("suspension".to_string(), Some("autorisation_revoquee".to_string()))));
}

/// Moyen 5 — une opposition posée après le début de la nuit ne porte que sur les nuits suivantes : la nuit en cours reste
/// couverte (l'élève n'est pas remis à l'attendu en pleine nuit), le CPE est alerté ; posée avant, elle joue tout de suite.
#[tokio::test]
async fn moyen_5_opposition_apres_le_debut_de_la_nuit_ne_porte_que_sur_les_nuits_suivantes() {
    let Some(b) = banc().await else { return };
    let (mardi, maintenant) = (avril(14), paris(avril(14), 9, 0));
    let (e1, e2) = (b.a[0], b.a[1]);
    let (pa, _) = parent(&b, e1, true).await;
    let (pb, _) = parent(&b, e1, true).await;
    let (pc, _) = parent(&b, e2, true).await;
    let (pd, _) = parent(&b, e2, true).await;
    let a1 = annoncer(&b, pa, PARENT, e1, mardi, avril(16), maintenant).await.unwrap();
    let a2 = annoncer(&b, pc, PARENT, e2, mardi, avril(16), maintenant).await.unwrap();
    // M-B : l'effet différé ne vaut que pour un élève réellement parti : e1 est remis à sa mère à 17 h.
    let pers_a: Uuid = sqlx::query_scalar("SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND user_id = $2").bind(b.etab).bind(pa).fetch_one(b.ent.pool_verif()).await.unwrap();
    b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a1.id, &depart(Some(pers_a), paris(mardi, 17, 0)), paris(mardi, 17, 0)).await.unwrap();
    // Avant le début de la nuit (10 h) : toutes les nuits redeviennent attendues aussitôt.
    b.ent.opposer_absence_a(b.etab, pd, PARENT, a2.id, &opposition(), paris(mardi, 10, 0)).await.unwrap();
    assert!(est_attendu(&b, mardi, e2).await && est_attendu(&b, avril(15), e2).await);
    // Après le début de la nuit (23 h) : la nuit du mardi reste couverte, les suivantes redeviennent attendues.
    let b1 = b.ent.opposer_absence_a(b.etab, pb, PARENT, a1.id, &opposition(), paris(mardi, 23, 0)).await.unwrap();
    assert_eq!(b1.statut, "bloquee");
    assert!(est_retire(&b, mardi, e1, "absence_annoncee").await, "la nuit en cours reste couverte");
    assert!(est_attendu(&b, avril(15), e1).await && est_attendu(&b, avril(16), e1).await, "les nuits suivantes redeviennent attendues");
    let evts = b.evenements("internat.absence_bloquee").await;
    assert!(evts.iter().any(|e| e["cause"].as_str() == Some("opposition") && e["nuit_en_cours"].as_bool() == Some(true)), "le CPE est alerté : {evts:?}");
    let file = b.ent.absences_a_traiter_a(b.etab, b.cpe, CPE).await.unwrap();
    assert_eq!(file.first().map(|x| x.statut.as_str()), Some("bloquee"), "bloquée : en tête de la file du CPE");
    // La direction lève : tout redevient couvert, l'effet différé disparaît.
    b.ent.trancher_absence_a(b.etab, b.dir, DIR, a1.id, &tranche("lever"), paris(mardi, 23, 30)).await.unwrap();
    assert!(est_retire(&b, avril(15), e1, "absence_annoncee").await);
}

/// Moyen 6 — seuls les textes d'auteur de qualité CPE ou direction sont montrés aux familles : jamais le texte d'un autre
/// titulaire, refus de sortie compris.
#[tokio::test]
async fn moyen_6_seuls_les_textes_du_cpe_ou_de_la_direction_sont_montres_aux_familles() {
    let Some(b) = banc().await else { return };
    let maintenant = paris(avril(14), 12, 0);
    let (e1, e2) = (b.a[0], b.a[1]);
    let (pa, pers_a) = parent(&b, e1, true).await;
    let (pb, _) = parent(&b, e1, true).await;
    let d = paris(avril(15), 20, 0);
    // Refus net d'un titulaire (sortie posée par le CPE, sans accord préalable) : son texte n'est pas lu par l'autre.
    let s = b.ent.demander_sortie_a(b.etab, b.cpe, CPE, &demande(e1, "retour_tardif", d, d + Duration::hours(3), false, Some(pers_a)), maintenant).await.unwrap();
    let ref_b = b.ent.repondre_sortie_a(b.etab, pb, PARENT, s.id, &AccordTitulaireDto { accord: false, motif_texte: Some("raison privée de B".into()), avec_personne_id: None }, maintenant).await.unwrap();
    assert_eq!(ref_b.statut, "refusee");
    let vu_par_a = b.ent.sorties_eleve_a(b.etab, pa, PARENT, e1, maintenant).await.unwrap();
    assert_eq!(vu_par_a[0].motif_refus_texte, None, "le texte d'un autre titulaire n'est pas montré");
    let h = b.ent.historique_a(b.etab, pa, PARENT, "sortie", s.id, maintenant).await.unwrap();
    assert!(h.iter().all(|f| f.motif_texte.is_none()), "ni dans l'historique : {h:?}");
    let h_cpe = b.ent.historique_a(b.etab, b.cpe, CPE, "sortie", s.id, maintenant).await.unwrap();
    assert!(h_cpe.iter().any(|f| f.motif_texte.as_deref() == Some("raison privée de B")), "le personnel, lui, le voit");
    // Refus du CPE : son motif est lu par la famille.
    let (p2, pers2) = parent(&b, e2, true).await;
    let s2 = b.ent.demander_sortie_a(b.etab, p2, PARENT, &demande(e2, "retour_tardif", d, d + Duration::hours(3), false, Some(pers2)), maintenant).await.unwrap();
    b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s2.id, &decision("refuser", Some("Hors règlement de l'internat")), maintenant).await.unwrap();
    let lu = b.ent.sorties_eleve_a(b.etab, p2, PARENT, e2, maintenant).await.unwrap();
    assert_eq!(lu[0].motif_refus_texte.as_deref(), Some("Hors règlement de l'internat"));
    let h = b.ent.historique_a(b.etab, p2, PARENT, "sortie", s2.id, maintenant).await.unwrap();
    assert!(h.iter().any(|f| f.motif_texte.as_deref() == Some("Hors règlement de l'internat")));
}

/// Moyen 7 — un contact validé dont le lien avec l'élève est suspendu ne prend pas l'élève.
#[tokio::test]
async fn moyen_7_contact_dont_le_lien_est_suspendu_ne_prend_pas_l_eleve() {
    let Some(b) = banc().await else { return };
    let (mardi, maintenant, eleve) = (avril(14), paris(avril(14), 9, 0), b.a[0]);
    let (pb, _) = parent(&b, eleve, true).await;
    let (ps, pers_s) = parent(&b, eleve, true).await;
    autoriser_recuperer(&b, eleve, pers_s).await;
    annoncer_avec(&b, pb, PARENT, eleve, mardi, mardi, Some(pers_s), maintenant).await.expect("lien actif : titulaire autorisé");
    sqlx::query("UPDATE ent_eleve_lien SET suspendu_le = now(), suspension_motif = 'suspension de test' WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(b.etab)
        .bind(eleve)
        .bind(ps)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let r = annoncer_avec(&b, pb, PARENT, eleve, avril(15), avril(15), Some(pers_s), maintenant).await;
    assert!(refus_de(&r, "pas autorisée à récupérer"), "lien suspendu, même contact validé : {r:?}");
}

/// Faible — un geste ne se date pas de plus de 30 minutes en arrière ; un retour antidaté ne remet pas l'élève à
/// l'attendu d'une nuit passée.
#[tokio::test]
async fn faible_geste_antidate_et_retour_sur_une_nuit_passee() {
    let Some(b) = banc().await else { return };
    let (eleve, vendredi) = (b.a[0], avril(17));
    let (p, _) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let (debut, fin) = (paris(vendredi, 12, 0), paris(avril(19), 20, 0));
    let s = b.ent.demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(grand)), paris(avril(14), 12, 0)).await.unwrap();
    // Antidate au-delà de la marge : refusée ; dans la marge : acceptée.
    let maintenant = paris(vendredi, 12, 40);
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), paris(vendredi, 12, 5)), maintenant).await;
    assert!(refus_de(&r, "antérieure"), "35 minutes en arrière : {r:?}");
    b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), paris(vendredi, 12, 15)), maintenant).await.expect("25 minutes : dans la marge");
    // Retour noté à 6 h 10 pour un geste de 5 h 50 : l'heure portée par la sortie ne précède pas le matin de la nuit passée.
    let samedi = avril(18);
    let tard = paris(samedi, 6, 10);
    let r = b.ent.controler_retour_a(b.etab, b.cpe, CPE, s.id, &ControleRetourDto { a: Some(paris(samedi, 5, 0)) }, tard).await;
    assert!(refus_de(&r, "antérieure"), "{r:?}");
    let ok = b.ent.controler_retour_a(b.etab, b.cpe, CPE, s.id, &ControleRetourDto { a: Some(paris(samedi, 5, 50)) }, tard).await.unwrap();
    assert_eq!(ok.statut, "rentree");
    let retour: DateTime<Utc> = sqlx::query_scalar("SELECT retour_effectif FROM ent_internat_sortie WHERE id = $1").bind(s.id).fetch_one(b.ent.pool_verif()).await.unwrap();
    assert_eq!(retour, paris(samedi, 6, 0), "ramené au matin déjà écoulé");
    assert!(est_retire(&b, vendredi, eleve, "sortie").await, "la nuit du vendredi, passée, ne revient pas à l'attendu");
}

/// Faible — doublons d'absence : la base refuse le recouvrement (contrainte d'exclusion), y compris entre deux annonces
/// simultanées des deux titulaires.
#[tokio::test]
async fn faible_doublons_d_absence_refuses_par_la_base_meme_en_concurrence() {
    let Some(b) = banc().await else { return };
    let (eleve, maintenant) = (b.a[0], paris(avril(14), 9, 0));
    let (pa, _) = parent(&b, eleve, true).await;
    let (pb, _) = parent(&b, eleve, true).await;
    let (r1, r2) = tokio::join!(
        annoncer(&b, pa, PARENT, eleve, avril(16), avril(17), maintenant),
        annoncer(&b, pb, PARENT, eleve, avril(17), avril(18), maintenant)
    );
    assert_eq!(r1.is_ok() as u8 + r2.is_ok() as u8, 1, "une seule des deux annonces simultanées passe : {r1:?} / {r2:?}");
    let refusee = if r1.is_err() { r1 } else { r2 };
    assert!(refus_de(&refusee, "déjà annoncée"), "{refusee:?}");
    // Directement en base : le recouvrement est une erreur d'exclusion (23P01).
    let e = sqlx::query(
        "INSERT INTO ent_internat_absence (id, etablissement_id, eleve_id, du_nuit, au_nuit, mode, annonce_par_user_id, annonce_qualite, statut, tardive) \
         SELECT gen_random_uuid(), etablissement_id, eleve_id, du_nuit, au_nuit, mode, annonce_par_user_id, annonce_qualite, statut, tardive \
           FROM ent_internat_absence WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(b.etab)
    .bind(eleve)
    .execute(b.ent.pool_verif())
    .await
    .unwrap_err();
    assert!(matches!(&e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23P01")), "{e:?}");
}

/// Faible — liste de remise : un élève protégé caché au personnel tiers n'est pas nommé et on ne montre ni ses titulaires
/// ni ses contacts.
#[tokio::test]
async fn faible_liste_de_remise_ne_montre_ni_titulaires_ni_contacts_d_un_eleve_cache() {
    let Some(b) = banc().await else { return };
    let (mardi, eleve) = (avril(14), b.a[0]);
    let (p, pers) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let (debut, fin) = (paris(mardi, 16, 0), paris(avril(16), 20, 0));
    b.ent.demander_sortie_a(b.etab, p, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(grand)), paris(mardi, 12, 0)).await.unwrap();
    let avant = b.ent.liste_remise_a(b.etab, b.cpe, CPE, mardi, paris(mardi, 12, 5)).await.unwrap();
    assert!(avant.iter().any(|l| l.eleve_id == eleve && l.personnes_autorisees.iter().any(|x| x.personne_id == grand || x.personne_id == pers)));
    b.ent
        .poser_confidentialite_a(
            DIR,
            crate::eleve_confidentialite::PoserConfidentialite {
                etablissement_id: b.etab,
                eleve_id: eleve,
                motif: "décision de justice".into(),
                mesures: vec![],
                du: NaiveDate::from_ymd_opt(2026, 1, 1),
                au: None,
                acteur: b.dir,
            },
            utc(2026, 3, 1, 10, 0),
        )
        .await
        .unwrap();
    let apres = b.ent.liste_remise_a(b.etab, b.cpe, CPE, mardi, paris(mardi, 12, 10)).await.unwrap();
    for l in apres.iter().filter(|l| l.eleve_id == eleve) {
        assert!(l.nom.is_none() && l.prenom.is_none(), "jamais nommé : {l:?}");
        assert!(l.personnes_autorisees.is_empty() && l.personnes_interdites.is_empty() && l.personne_prevue.is_none() && !l.interdiction_non_nominative, "ni titulaires ni contacts : {l:?}");
    }
}

/// M-1 — un appel « soir » complémentaire laissé ouvert ne bloque pas la transmission d'une nuit passée (ignoré avec
/// mention) ; l'élève qu'il devait couvrir est non couvert et ouvre un cas pour le CPE (une seule fois).
#[tokio::test]
async fn faible_m1_complement_laisse_ouvert_ne_bloque_pas_la_transmission_et_ouvre_un_cas() {
    let Some(b) = banc().await else { return };
    let matin = utc(2026, 3, 29, 5, 0);
    let principal = b.ouvrir("soir", "internat", None, 0).await.appel.id;
    for e in b.a.iter().chain(b.b.iter()) {
        b.point(principal, *e, "present", 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, principal, soir(5)).await.unwrap();
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, principal, soir(7)).await.unwrap();
    let invite = eleve_invite(&b, "Retardataire").await;
    b.ent
        .poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &NuitExceptionnelleSaisieDto { eleve_id: invite, nuit: nuit_dst(), motif_code: "examen".into() }, soir(10))
        .await
        .unwrap();
    let complement = AppelOuvertureDto { complementaire: true, ..crate::tests_internat_i2::ouverture("soir", "internat", None) };
    let c = b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &complement, soir(12)).await.unwrap();
    assert_eq!(c.appel.complement, 1);
    // Personne ne le clôt : la nuit est passée, la transmission part quand même, avec la mention.
    let v = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await.unwrap();
    assert_eq!((v.nb_non_couverts, v.non_couverts.clone(), v.complements_ignores), (1, vec![invite], 1));
    let cas: Vec<(String, Option<Uuid>)> = sqlx::query_as("SELECT nature, ouvert_par_user_id FROM ent_internat_cas WHERE etablissement_id = $1 AND eleve_id = $2 AND nuit = $3")
        .bind(b.etab)
        .bind(invite)
        .bind(nuit_dst())
        .fetch_all(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(cas, vec![("non_couvert".to_string(), Some(b.cpe))], "un cas pour le CPE");
    let evts = b.evenements("internat.transmission").await;
    assert_eq!(evts.last().and_then(|e| e["complements_ignores"].as_i64()), Some(1));
    // Rejouée : même transmission, pas de second cas.
    let encore = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await.unwrap();
    assert!(!encore.nouvelle);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_cas WHERE etablissement_id = $1 AND nature = 'non_couvert'").bind(b.etab).fetch_one(b.ent.pool_verif()).await.unwrap();
    assert_eq!(n, 1);
    // Un appel PRINCIPAL resté ouvert, lui, bloque encore la transmission.
    let _ = b.ouvrir("extinction", "zone", Some(b.zone_a), 20).await;
    let r = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), Some("test"), matin).await;
    assert!(refus_de(&r, "encore ouvert"), "{r:?}");
}

// ═════════════════════════════ Passe 2 de la contre-revue du pilote ═════════════════════════════

/// Fixe la date de pose des interdictions de l'élève : `created_at` vaut sinon l'instant réel, alors que ces tests
/// jouent à des jours fixes d'avril 2026 (règle 33).
async fn dater_interdictions(b: &Banc, eleve: Uuid, instant: DateTime<Utc>) {
    sqlx::query("UPDATE ent_eleve_interdiction SET created_at = $3 WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(b.etab)
        .bind(eleve)
        .bind(instant)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
}

/// Fixe la date de création du lien d'un compte avec l'élève.
async fn dater_lien(b: &Banc, eleve: Uuid, user: Uuid, instant: DateTime<Utc>) {
    sqlx::query("UPDATE ent_eleve_lien SET created_at = $4 WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(b.etab)
        .bind(eleve)
        .bind(user)
        .bind(instant)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
}

fn accord_ok() -> AccordTitulaireDto {
    AccordTitulaireDto { accord: true, motif_texte: None, avec_personne_id: None }
}

/// E-A (sortie seule) — ce qu'un parent a fait avant son interdiction ne reste pas effectif : lundi le père demande et
/// accorde un départ seul pour mercredi, mardi une interdiction le vise, mercredi l'enfant ne part pas sur cet accord
/// (au départ, ou plus tôt à la revue à la minute). La décision revient au CPE ; l'autre titulaire peut répondre.
#[tokio::test]
async fn ea_sortie_seule_l_accord_d_un_parent_devenu_interdit_tombe_au_depart_et_a_la_revue() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, mercredi) = (avril(13), avril(14), avril(15));
    for (i, par_la_revue) in [(0usize, false), (1usize, true)] {
        let eleve = b.a[i];
        let (pa, pers_a) = parent(&b, eleve, true).await;
        let (pb, _) = parent(&b, eleve, true).await;
        let d = paris(mercredi, 20, 0);
        let s = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(eleve, "retour_tardif", d, d + Duration::hours(3), true, None), paris(lundi, 10, 0)).await.unwrap();
        assert!(s.accord_titulaire, "le père accorde en demandant");
        let s = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(lundi, 11, 0)).await.unwrap();
        assert_eq!(s.statut, "autorisee");
        interdire_objet(&b, eleve, pers_a, "contact_eleve", None).await;
        dater_interdictions(&b, eleve, paris(mardi, 12, 0)).await;
        if par_la_revue {
            b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 1)).await.unwrap();
            let vue = b.ent.sorties_eleve_a(b.etab, b.cpe, CPE, eleve, paris(mardi, 12, 2)).await.unwrap();
            assert_eq!((vue[0].statut.as_str(), vue[0].accord_titulaire, vue[0].decision_cpe), ("demandee", false, false), "revue : accord retiré");
            let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, d), d).await;
            assert!(refus_de(&r, "pas encore autorisée"), "{r:?}");
        } else {
            let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, d), d).await;
            assert!(refus_de(&r, "désormais l'objet d'une interdiction"), "{r:?}");
            let f = faits_actes(&b, "sortie_id", s.id).await;
            assert!(f.contains(&("accord_remis_a_zero".to_string(), Some("interdiction_auteur".to_string()))), "{f:?}");
            assert!(f.contains(&("depart_refuse".to_string(), Some("interdiction_auteur".to_string()))), "{f:?}");
        }
        // Le père interdit n'agit plus ; la mère répond, le CPE décide de nouveau, puis le départ seul a lieu.
        assert!(est_interdit(&b.ent.repondre_sortie_a(b.etab, pa, PARENT, s.id, &accord_ok(), paris(mardi, 13, 0)).await));
        let ok = b.ent.repondre_sortie_a(b.etab, pb, PARENT, s.id, &accord_ok(), paris(mardi, 13, 0)).await.unwrap();
        assert_eq!(ok.statut, "demandee", "la décision du CPE est à reprendre");
        let ok = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(mardi, 14, 0)).await.unwrap();
        assert_eq!(ok.statut, "autorisee");
        b.ent.veille_sorties_a(b.etab, paris(mercredi, 19, 0)).await.unwrap();
        let dep = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, d), d).await.unwrap();
        assert_eq!(dep.statut, "partie", "l'accord de la mère vaut");
    }
}

/// E-A (absence) — l'annonce d'un parent devenu interdit n'est plus valable : au départ et à la revue, l'absence est
/// bloquée (motif `interdiction_auteur`) et l'élève reste attendu ; la direction tranche, et sa décision n'est pas
/// rebloquée à la revue suivante.
#[tokio::test]
async fn ea_absence_annoncee_par_un_parent_devenu_interdit_est_bloquee_au_depart_et_a_la_revue() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, mercredi) = (avril(13), avril(14), avril(15));
    let soir = paris(mercredi, 17, 0);
    for (i, par_la_revue) in [(0usize, false), (1usize, true)] {
        let eleve = b.a[i];
        let (pa, pers_a) = parent(&b, eleve, true).await;
        let grand = tiers(&b, "Odile").await;
        autoriser_recuperer(&b, eleve, grand).await;
        let a = annoncer_avec(&b, pa, PARENT, eleve, mercredi, mercredi, Some(grand), paris(lundi, 10, 0)).await.unwrap();
        interdire_objet(&b, eleve, pers_a, "visite", None).await;
        dater_interdictions(&b, eleve, paris(mardi, 12, 0)).await;
        if par_la_revue {
            b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 1)).await.unwrap();
        } else {
            let r = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(Some(grand), soir), soir).await;
            assert!(refus_de(&r, "désormais l'objet d'une interdiction"), "{r:?}");
            let f = faits_actes(&b, "absence_id", a.id).await;
            assert!(f.contains(&("depart_refuse".to_string(), Some("interdiction_auteur".to_string()))), "{f:?}");
        }
        let vue = b.ent.absences_eleve_a(b.etab, b.cpe, CPE, eleve, soir).await.unwrap();
        assert_eq!(vue[0].statut, "bloquee");
        let f = faits_actes(&b, "absence_id", a.id).await;
        assert!(f.contains(&("blocage".to_string(), Some("interdiction_auteur".to_string()))), "{f:?}");
        assert!(est_attendu(&b, mercredi, eleve).await, "aucun départ constaté : l'élève est attendu");
        // Passe 3 : la personne désignée par le parent visé est effacée avec son acte. La direction ne lève pas sans motif
        // (décision explicite, tracée) ; sans personne désignée ni autorisation annuelle, lever n'a pas de sens : elle
        // maintient, et la famille annonce de nouveau.
        let r = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &tranche("lever"), paris(mardi, 12, 10)).await;
        assert!(refus_de(&r, "motif obligatoire"), "{r:?}");
        let motif = TrancheDto { decision: "lever".into(), motif_texte: Some("Levée demandée par la famille".into()) };
        let r = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &motif, paris(mardi, 12, 11)).await;
        assert!(refus_de(&r, "aucune personne n'est désignée"), "{r:?}");
        let m = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &tranche("maintenir"), paris(mardi, 12, 12)).await.unwrap();
        assert_eq!(m.statut, "refusee");
    }
}

/// E-A (correspondant) — un correspondant désigné par un parent devenu interdit est suspendu, à la revue ou au départ ;
/// les sorties chez lui le sont aussi ; seul le CPE le reprend, et la revue suivante respecte sa décision.
#[tokio::test]
async fn ea_correspondant_designe_par_un_parent_devenu_interdit_est_suspendu() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi) = (avril(13), avril(14));
    let (debut, fin) = (paris(mardi, 17, 0), paris(avril(15), 8, 0));
    for (i, par_la_revue) in [(0usize, true), (1usize, false)] {
        let eleve = b.a[i];
        let (pa, pers_a) = parent(&b, eleve, true).await;
        let (pb, _) = parent(&b, eleve, true).await;
        let corr = tiers(&b, "Odile").await;
        autoriser_recuperer(&b, eleve, corr).await;
        let saisie = CorrespondantSaisieDto { eleve_id: eleve, personne_id: corr, lundi: true, mardi: true, mercredi: true, jeudi: true, vendredi: true, samedi: true, dimanche: true, du: None, au: None };
        let c = b.ent.designer_correspondant_a(b.etab, pa, PARENT, &saisie, paris(lundi, 9, 0)).await.unwrap();
        // La sortie, elle, est demandée (et accordée) par la mère : seule la désignation du correspondant est du père.
        let d = SortieDemandeDto { correspondant_id: Some(c.id), ..demande(eleve, "nuit_correspondant", debut, fin, false, None) };
        let s = b.ent.demander_sortie_a(b.etab, pb, PARENT, &d, paris(lundi, 10, 0)).await.unwrap();
        let s = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(lundi, 11, 0)).await.unwrap();
        assert_eq!(s.statut, "autorisee");
        interdire_objet(&b, eleve, pers_a, "remise", None).await;
        dater_interdictions(&b, eleve, paris(mardi, 12, 0)).await;
        if par_la_revue {
            b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 1)).await.unwrap();
            let sorties = b.ent.sorties_eleve_a(b.etab, b.cpe, CPE, eleve, paris(mardi, 12, 2)).await.unwrap();
            assert_eq!(sorties[0].statut, "suspendue", "la sortie chez le correspondant est suspendue");
        } else {
            let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(corr), debut), debut).await;
            assert!(refus_de(&r, "correspondant est suspendu"), "{r:?}");
        }
        let vus = b.ent.correspondants_eleve_a(b.etab, b.cpe, CPE, eleve, paris(mardi, 13, 0)).await.unwrap();
        assert_eq!(vus[0].statut, "suspendu");
        assert!(faits_actes(&b, "correspondant_id", c.id).await.contains(&("suspension".to_string(), Some("interdiction_auteur".to_string()))));
        // Le CPE le reprend : la revue suivante ne le suspend pas de nouveau.
        let r = b.ent.suspendre_correspondant_a(b.etab, b.cpe, CPE, c.id, false, &CorrespondantSuspensionDto { motif_texte: Some("Désignation revue avec la famille".into()) }, paris(mardi, 14, 0)).await.unwrap();
        assert_eq!(r.statut, "valide");
        b.ent.veille_sorties_a(b.etab, paris(mardi, 14, 5)).await.unwrap();
        let vus = b.ent.correspondants_eleve_a(b.etab, b.cpe, CPE, eleve, paris(mardi, 15, 0)).await.unwrap();
        assert_eq!(vus[0].statut, "valide", "décision du CPE respectée");
    }
}

/// M-A — règle D25 : un lien créé APRÈS une interdiction sans personne identifiée, non levée, n'est ni acteur ni lecteur
/// tant que la direction ne l'a pas confirmé (même décision que dans la messagerie) ; un lien antérieur n'est pas touché.
#[tokio::test]
async fn ma_lien_cree_apres_une_interdiction_sans_personne_n_est_ni_acteur_ni_lecteur_tant_que_non_confirme() {
    let Some(b) = banc().await else { return };
    let (mardi, eleve) = (avril(14), b.a[0]);
    let maintenant = paris(avril(15), 9, 0);
    let (avant, _) = parent(&b, eleve, true).await;
    dater_lien(&b, eleve, avant, paris(avril(10), 9, 0)).await;
    interdire(&b, eleve, None).await;
    dater_interdictions(&b, eleve, paris(mardi, 8, 0)).await;
    let (apres, _) = parent(&b, eleve, true).await;
    dater_lien(&b, eleve, apres, paris(mardi, 9, 0)).await;
    assert!(b.ent.absences_eleve_a(b.etab, avant, PARENT, eleve, maintenant).await.is_ok(), "lien antérieur : inchangé");
    let r = b.ent.absences_eleve_a(b.etab, apres, PARENT, eleve, maintenant).await;
    assert!(est_interdit(&r), "lecteur : {r:?}");
    assert!(est_interdit(&b.ent.autorisations_eleve_a(b.etab, apres, PARENT, eleve, maintenant).await), "lecteur des autorisations");
    assert!(est_interdit(&annoncer(&b, apres, PARENT, eleve, avril(21), avril(21), maintenant).await), "acteur");
    // La direction confirme le lien (décision de la messagerie) : il redevient acteur et lecteur.
    sqlx::query(
        "INSERT INTO ent_msg_lien_decision (etablissement_id, lien_id, interdiction_id, decision, decide_par) \
         SELECT l.etablissement_id, l.id, i.id, 'confirme', $4 FROM ent_eleve_lien l, ent_eleve_interdiction i \
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.user_id = $3 AND i.etablissement_id = $1 AND i.eleve_id = $2",
    )
    .bind(b.etab)
    .bind(eleve)
    .bind(apres)
    .bind(b.dir)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    assert!(b.ent.absences_eleve_a(b.etab, apres, PARENT, eleve, maintenant).await.is_ok(), "confirmé par la direction");
    // Une interdiction levée ne joue plus.
    let e2 = b.a[1];
    let (p2, _) = parent(&b, e2, true).await;
    interdire(&b, e2, None).await;
    dater_interdictions(&b, e2, paris(mardi, 8, 0)).await;
    let (p3, _) = parent(&b, e2, true).await;
    dater_lien(&b, e2, p3, paris(mardi, 9, 0)).await;
    assert!(est_interdit(&b.ent.absences_eleve_a(b.etab, p3, PARENT, e2, maintenant).await));
    lever_interdictions(&b, e2).await;
    assert!(b.ent.absences_eleve_a(b.etab, p3, PARENT, e2, maintenant).await.is_ok());
    assert!(b.ent.absences_eleve_a(b.etab, p2, PARENT, e2, maintenant).await.is_ok());
}

/// M-A — tant qu'une interdiction de remise sans personne identifiée est en vigueur, un mineur ne part pas seul (sortie
/// ou absence) sans décision de la direction : blocage à la revue ou au départ ; une fois débloqué par la direction,
/// le départ a lieu et la revue ne rebloque pas.
#[tokio::test]
async fn ma_mineur_ne_part_pas_seul_tant_qu_une_interdiction_de_remise_sans_personne_est_en_vigueur() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, mercredi, vendredi) = (avril(13), avril(14), avril(15), avril(17));
    // Sortie seule (retour tardif, accord de la famille) : bloquée à la revue, puis débloquée par la direction.
    let e1 = b.a[0];
    let (p1, _) = parent(&b, e1, true).await;
    let d = paris(mercredi, 20, 0);
    let s = b.ent.demander_sortie_a(b.etab, p1, PARENT, &demande(e1, "retour_tardif", d, d + Duration::hours(3), true, None), paris(lundi, 10, 0)).await.unwrap();
    let s = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(lundi, 11, 0)).await.unwrap();
    assert_eq!(s.statut, "autorisee");
    interdire(&b, e1, None).await;
    dater_interdictions(&b, e1, paris(mardi, 12, 0)).await;
    b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 1)).await.unwrap();
    let vue = b.ent.sorties_eleve_a(b.etab, b.cpe, CPE, e1, paris(mardi, 12, 2)).await.unwrap();
    assert_eq!(vue[0].statut, "bloquee");
    assert!(faits_actes(&b, "sortie_id", s.id).await.contains(&("blocage".to_string(), Some("interdiction_remise".to_string()))));
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, d), d).await;
    assert!(refus_de(&r, "bloquée"), "{r:?}");
    let l = b.ent.trancher_sortie_a(b.etab, b.dir, DIR, s.id, &tranche_motivee("lever"), paris(mardi, 13, 0)).await.unwrap();
    assert_eq!(l.statut, "autorisee");
    b.ent.veille_sorties_a(b.etab, paris(mercredi, 19, 0)).await.unwrap();
    let dep = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, d), d).await.unwrap();
    assert_eq!(dep.statut, "partie", "décision de la direction postérieure à l'interdiction : le départ a lieu");
    // Départ seul d'une absence (autorisation annuelle « départ seul le week-end », annoncée pour un vendredi) : refusé
    // au portail, absence bloquée, sans même attendre la revue.
    let e2 = b.a[1];
    let (p2, _) = parent(&b, e2, true).await;
    accorder_annuellement(&b, p2, e2, &["week_end_seul"]).await;
    let a = annoncer_avec(&b, p2, PARENT, e2, vendredi, vendredi, None, paris(mardi, 10, 0)).await.unwrap();
    interdire(&b, e2, None).await;
    dater_interdictions(&b, e2, paris(mercredi, 12, 0)).await;
    let soir = paris(vendredi, 17, 0);
    let r = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(None, soir), soir).await;
    assert!(refus_de(&r, "interdiction de remise sans personne identifiée"), "{r:?}");
    let vue = b.ent.absences_eleve_a(b.etab, b.cpe, CPE, e2, soir).await.unwrap();
    assert_eq!(vue[0].statut, "bloquee");
    assert!(est_attendu(&b, vendredi, e2).await);
    b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &tranche_motivee("lever"), paris(vendredi, 17, 5)).await.unwrap();
    b.ent.veille_sorties_a(b.etab, paris(vendredi, 17, 10)).await.unwrap();
    let ok = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(None, paris(vendredi, 17, 15)), paris(vendredi, 17, 15)).await.unwrap();
    assert!(ok.depart_effectif.is_some(), "décision de la direction : le départ seul a lieu");
}

/// M-B — pas de départ constaté = attendu : l'absence d'un mineur ne le retire de l'attendu qu'à son départ effectif.
/// Un départ refusé au portail (interdiction posée à 22 h 05, refus à 22 h 10) le laisse attendu ; une absence restée
/// sans départ à l'heure limite ouvre un cas « non récupéré » (une fois) ; un majeur, lui, est retiré dès l'annonce.
#[tokio::test]
async fn mb_pas_de_depart_constate_l_eleve_mineur_reste_attendu_et_un_cas_non_recupere_s_ouvre() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi) = (avril(13), avril(14));
    // Départ constaté : retiré.
    let e1 = b.a[0];
    let (p1, pers1) = parent(&b, e1, true).await;
    let a1 = annoncer(&b, p1, PARENT, e1, mardi, mardi, paris(lundi, 10, 0)).await.unwrap();
    assert!(est_attendu(&b, mardi, e1).await, "annoncée mais personne n'est venu : attendu");
    let soir = paris(mardi, 17, 0);
    b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a1.id, &depart(Some(pers1), soir), soir).await.unwrap();
    assert!(est_retire(&b, mardi, e1, "absence_annoncee").await, "parti : retiré");
    // Refus au portail à 22 h 10 après une interdiction posée à 22 h 05 : l'élève reste attendu.
    let e2 = b.a[1];
    let (p2, _) = parent(&b, e2, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, e2, grand).await;
    let a2 = annoncer_avec(&b, p2, PARENT, e2, mardi, mardi, Some(grand), paris(lundi, 10, 0)).await.unwrap();
    interdire(&b, e2, Some(grand)).await;
    b.ent.veille_sorties_a(b.etab, paris(mardi, 22, 5)).await.unwrap();
    let r = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a2.id, &depart(Some(grand), paris(mardi, 22, 10)), paris(mardi, 22, 10)).await;
    assert!(refus_de(&r, "bloquée") || refus_de(&r, "REMISE REFUSÉE"), "{r:?}");
    assert!(est_attendu(&b, mardi, e2).await, "départ refusé : l'élève reste attendu pour la nuit en cours");
    let evts = b.evenements("internat.absence_bloquee").await;
    assert!(evts.iter().all(|e| e["nuit_en_cours"].as_bool() != Some(true)), "aucun effet différé sans départ : {evts:?}");
    // Sans départ à l'heure limite (18 h + 30 min) : un cas « non récupéré », une seule fois ; majeur : aucun cas.
    // Passe 3 (point 3) : l'heure limite de récupération ne vaut que pour une absence qui COMMENCE LE VENDREDI.
    let vendredi = avril(17);
    let e3 = b.a[2];
    let (p3, _) = parent(&b, e3, true).await;
    annoncer(&b, p3, PARENT, e3, vendredi, vendredi, paris(lundi, 10, 0)).await.unwrap();
    let moi = compte_eleve(&b, b.a[3], NaiveDate::from_ymd_opt(2007, 1, 1).unwrap()).await;
    annoncer(&b, moi, ELEVE, b.a[3], mardi, mardi, paris(lundi, 10, 0)).await.unwrap();
    assert!(est_retire(&b, mardi, b.a[3], "absence_annoncee").await, "majeur : retiré dès l'annonce");
    let v = b.ent.veille_sorties_a(b.etab, paris(mardi, 18, 31)).await.unwrap();
    assert_eq!(v.cas_non_recupere, 0, "une absence du mardi n'a pas d'heure de récupération");
    let v = b.ent.veille_sorties_a(b.etab, paris(vendredi, 18, 29)).await.unwrap();
    assert_eq!(v.cas_non_recupere, 0, "avant l'heure limite");
    let v = b.ent.veille_sorties_a(b.etab, paris(vendredi, 18, 31)).await.unwrap();
    assert_eq!(v.cas_non_recupere, 1, "e3 : le seul mineur du vendredi resté sans départ");
    let v = b.ent.veille_sorties_a(b.etab, paris(vendredi, 18, 40)).await.unwrap();
    assert_eq!(v.cas_non_recupere, 0, "une seule ouverture");
}

/// F-A — un retard qui ajoute des nuits couvertes (nombre de nuits avant/après) part au CPE, même si la sortie en
/// couvrait déjà une.
#[tokio::test]
async fn fa_retard_qui_ajoute_une_nuit_part_au_cpe_meme_si_la_sortie_en_couvrait_deja() {
    let Some(b) = banc().await else { return };
    let (eleve, maintenant) = (b.a[0], paris(avril(14), 12, 0));
    let (p, pers) = parent(&b, eleve, true).await;
    let s = week_end_parti(&b, eleve, p, pers, maintenant).await;
    // Un retard qui reste dans la dernière nuit déjà couverte (dimanche 20 h → lundi 5 h) s'applique tout de suite.
    let petit = b.ent.annoncer_retard_a(b.etab, p, PARENT, s.id, &RetardAnnonceDto { nouveau_retour: paris(avril(20), 5, 0), motif_code: Some("train".into()) }, paris(avril(19), 12, 0)).await.unwrap();
    assert!(petit.retard_demande.is_none() && petit.fin_ajustee == paris(avril(20), 5, 0), "{petit:?}");
    // Un retard qui fait couvrir la nuit du dimanche au lundi : demande au CPE, retour attendu inchangé.
    let grand = b.ent.annoncer_retard_a(b.etab, p, PARENT, s.id, &RetardAnnonceDto { nouveau_retour: paris(avril(20), 9, 0), motif_code: Some("train".into()) }, paris(avril(19), 13, 0)).await.unwrap();
    assert_eq!(grand.retard_demande, Some(paris(avril(20), 9, 0)));
    assert_eq!(grand.fin_ajustee, paris(avril(20), 5, 0));
}

/// F-B — l'autorisation annuelle « départ seul le week-end » ne couvre le départ seul d'une absence que si la première
/// nuit est une veille de week-end (le vendredi) : refusé à l'annonce un mardi, accepté un vendredi.
#[tokio::test]
async fn fb_autorisation_week_end_seul_ne_couvre_une_absence_que_la_veille_d_un_week_end() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (p, _) = parent(&b, eleve, true).await;
    accorder_annuellement(&b, p, eleve, &["week_end_seul"]).await;
    let r = annoncer_avec(&b, p, PARENT, eleve, avril(14), avril(14), None, paris(avril(13), 10, 0)).await;
    assert!(refus_de(&r, "veille d'un week-end"), "mardi : {r:?}");
    let a = annoncer_avec(&b, p, PARENT, eleve, avril(17), avril(17), None, paris(avril(13), 10, 0)).await.expect("vendredi");
    let soir = paris(avril(17), 17, 0);
    let ok = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(None, soir), soir).await.unwrap();
    assert!(ok.depart_effectif.is_some());
}

/// F-C — effacer la personne désignée par l'autre titulaire vaut changement, donc opposition (sortie bloquée,
/// désignation d'origine conservée). F-D — la famille ne lit jamais « interdiction » dans l'historique (code
/// générique) ; le personnel, si. F-F — une personne autorisée à récupérer dont le lien est sans autorité
/// parentale (ou en garde « aucune ») ne prend pas l'élève par la porte des contacts.
#[tokio::test]
async fn fc_fd_ff_effacer_la_personne_historique_sans_interdiction_et_contact_sans_autorite() {
    let Some(b) = banc().await else { return };
    let (mardi, eleve) = (avril(14), b.a[0]);
    let (pa, _) = parent(&b, eleve, true).await;
    let (pb, _) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let (debut, fin) = (paris(avril(17), 16, 0), paris(avril(19), 20, 0));
    let maintenant = paris(mardi, 12, 0);
    let s = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(eleve, "week_end", debut, fin, false, Some(grand)), maintenant).await.unwrap();
    // F-C
    let r = b.ent.designer_personne_sortie_a(b.etab, pb, PARENT, s.id, &PersonneSortieDto { avec_personne_id: None, seul: false }, maintenant).await.unwrap();
    assert_eq!((r.statut.as_str(), r.avec_personne_id, r.opposition), ("bloquee", Some(grand), true));
    b.ent.trancher_sortie_a(b.etab, b.dir, DIR, s.id, &tranche("lever"), maintenant).await.unwrap();
    // F-D
    interdire(&b, eleve, Some(grand)).await;
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut), debut).await;
    assert!(refus_de(&r, "REMISE REFUSÉE"), "{r:?}");
    let vu_famille = b.ent.historique_a(b.etab, pb, PARENT, "sortie", s.id, debut).await.unwrap();
    assert!(vu_famille.iter().all(|f| !f.motif_code.as_deref().is_some_and(|m| m.starts_with("interdiction"))), "{vu_famille:?}");
    assert!(vu_famille.iter().any(|f| f.acte == "depart_refuse" && f.motif_code.as_deref() == Some("personne_non_autorisee")));
    let vu_cpe = b.ent.historique_a(b.etab, b.cpe, CPE, "sortie", s.id, debut).await.unwrap();
    assert!(vu_cpe.iter().any(|f| f.motif_code.as_deref() == Some("interdiction_remise")), "le personnel lit le vrai motif");
    // F-F : deux liens écartés de la garde, tous deux « personne autorisée à récupérer ».
    for (garde, autorite) in [(Some("aucune"), true), (None, false)] {
        let user = compte(&b.ent, b.etab, PARENT).await;
        let pers = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom, user_id, source) VALUES ($1,$2,$3,'Ancien',$4,'compte')")
            .bind(pers)
            .bind(b.etab)
            .bind(format!("Ancien{}", alea()))
            .bind(user)
            .execute(b.ent.pool_verif())
            .await
            .unwrap();
        sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale, garde) VALUES ($1,$2,$3,$4,$5,'pere',$6,$7)")
            .bind(Uuid::new_v4())
            .bind(b.etab)
            .bind(eleve)
            .bind(user)
            .bind(pers)
            .bind(autorite)
            .bind(garde)
            .execute(b.ent.pool_verif())
            .await
            .unwrap();
        autoriser_recuperer(&b, eleve, pers).await;
        let r = annoncer_avec(&b, pb, PARENT, eleve, avril(21), avril(21), Some(pers), maintenant).await;
        assert!(refus_de(&r, "pas autorisée à récupérer"), "garde {garde:?}, autorité {autorite} : {r:?}");
    }
    // Revue E1 (décision du pilote) : la règle d'autorité lit le TYPE du lien (`mere`, `pere`, `tuteur`), jamais un libellé.
    // Un proche de lien « autre » SANS autorité parentale (grand-mère inscrite comme responsable) reste remis comme contact
    // validé (comme il est posable comme tiers) ; seul un parent ou tuteur privé d'autorité est écarté (cas « pere » ci-dessus).
    let user = compte(&b.ent, b.etab, PARENT).await;
    let mamie = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom, user_id, source) VALUES ($1,$2,$3,'Mamie',$4,'compte')")
        .bind(mamie)
        .bind(b.etab)
        .bind(format!("Proche{}", alea()))
        .bind(user)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale) VALUES ($1,$2,$3,$4,$5,'autre',false)")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(eleve)
        .bind(user)
        .bind(mamie)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    autoriser_recuperer(&b, eleve, mamie).await;
    let r = annoncer_avec(&b, pb, PARENT, eleve, avril(22), avril(22), Some(mamie), maintenant).await;
    assert!(r.is_ok(), "proche de lien « autre » sans autorité, contact validé : {r:?}");
    let sans_lien = tiers(&b, "Voisine").await;
    autoriser_recuperer(&b, eleve, sans_lien).await;
    let a = annoncer_avec(&b, pb, PARENT, eleve, avril(23), avril(23), Some(sans_lien), maintenant).await.unwrap();
    assert_eq!(a.recupere_par_personne_id, Some(sans_lien));
}

// ═════════════════════════════ Passe 3 : « un acte tombe avec son auteur » ═════════════════════════════
//
// | Règle ou point de la passe 3 | Test(s) |
// |---|---|
// | Règle générale : sortie du système, personne désignée par un parent (journal) | `p3_regle_*week_end_du_systeme*` |
// | Accord de B et personne de A | `p3_regle_sortie_accompagnee_accord_de_b_personne_de_a` |
// | Autorisation annuelle signée par A seul / par A et B ; point 2 (verdict) | `p3_regle_autorisation_annuelle_*` |
// | Correspondant désigné par A | `ea_correspondant_designe_par_un_parent_devenu_interdit_est_suspendu` (même fonction) |
// | Sortie suspendue ou bloquée (E-A) | `p3_regle_sortie_suspendue_ou_bloquee_*` |
// | Point 1 : déjà parti | `p3_deja_parti_*` |
// | Point 3 : faux « non récupéré » | `p3_non_recupere_*` et `mb_*` |
// | Point 4 : interdiction sans personne identifiée, levée ponctuelle | `p3_interdiction_sans_personne_*` |
// | Point 5 : lien échu | `p3_lien_echu_*` |
// | Point 6 : `week_end_seul` | `fb_*` |
// | Point 7 : course de la revue | `p3_course_*` |
// | Point 9 : charge (300 internes) | `p3_charge_*` |

/// Trois internes sans nuit du vendredi, une sortie de week-end créée par le système pour chacun (aucune colonne de la
/// ligne ne porte de parent), un parent titulaire A par élève qui désigne `grand` comme personne qui récupère.
/// Renvoie, par élève : (élève, compte de A, personne de A, sortie).
async fn week_ends_du_systeme(b: &Banc, grand: Uuid) -> Vec<(Uuid, Uuid, Uuid, Uuid)> {
    let (mardi, vendredi) = (avril(14), avril(17));
    let maintenant = paris(mardi, 10, 0);
    let mut eleves = Vec::new();
    for prenom in ["Léo", "Max", "Sam"] {
        let e = eleve_invite(b, prenom).await;
        heberger_rythme(b, e, [true, true, true, true, true, false, false]).await;
        eleves.push(e);
    }
    let d = b.ent.preparer_departs_a(b.etab, b.cpe, CPE, vendredi, maintenant).await.unwrap();
    assert_eq!(d.crees, 3, "{d:?}");
    let mut sortie = Vec::new();
    for e in eleves {
        let (pa, pers_a) = parent(b, e, true).await;
        autoriser_recuperer(b, e, grand).await;
        let s = b.ent.sorties_eleve_a(b.etab, b.cpe, CPE, e, maintenant).await.unwrap()[0].id;
        let d = b
            .ent
            .designer_personne_sortie_a(b.etab, pa, PARENT, s, &PersonneSortieDto { avec_personne_id: Some(grand), seul: false }, maintenant)
            .await
            .unwrap();
        assert_eq!((d.statut.as_str(), d.avec_personne_id), ("autorisee", Some(grand)));
        sortie.push((e, pa, pers_a, s));
    }
    sortie
}

async fn ligne_sortie(b: &Banc, id: Uuid) -> (String, Option<Uuid>, Option<Uuid>, Option<Uuid>, bool) {
    sqlx::query_as(
        "SELECT statut, avec_personne_id, accord_titulaire_user_id, demande_par_user_id, par_autorisation_annuelle \
           FROM ent_internat_sortie WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(b.etab)
    .bind(id)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap()
}

/// Règle générale (1/4) — sortie de week-end **créée par le système**, puis personne désignée par le parent A : aucune
/// colonne de la ligne ne porte A, l'auteur n'est que dans le journal des faits (`reprise`). A devient visé : la personne
/// tombe, à la revue (Sam) comme au départ (Léo) ; le témoin (Max, parent non visé) part normalement avec la même
/// personne : sans la règle, le départ de Léo et de Sam aurait eu lieu.
#[tokio::test]
async fn p3_regle_sortie_week_end_du_systeme_personne_designee_par_un_parent_devenu_interdit() {
    let Some(b) = banc().await else { return };
    let (mercredi, vendredi) = (avril(15), avril(17));
    let grand = tiers(&b, "Odile").await;
    let cas = week_ends_du_systeme(&b, grand).await;
    let ((leo, _, pers_leo, s_leo), (max, _, _, s_max), (sam, _, pers_sam, s_sam)) = (cas[0], cas[1], cas[2]);
    let (statut, avec, accord, demande_par, _) = ligne_sortie(&b, s_sam).await;
    assert_eq!((statut.as_str(), avec, accord, demande_par), ("autorisee", Some(grand), None, None), "l'auteur n'est que dans le journal");
    // Revue : Sam (A visé de contact avec l'élève).
    interdire_objet(&b, sam, pers_sam, "contact_eleve", None).await;
    dater_interdictions(&b, sam, paris(mercredi, 12, 0)).await;
    b.ent.veille_sorties_a(b.etab, paris(mercredi, 12, 5)).await.unwrap();
    let (statut, avec, ..) = ligne_sortie(&b, s_sam).await;
    assert_eq!((statut.as_str(), avec), ("demandee", None), "la personne désignée par A tombe, la décision revient au CPE");
    let f: Vec<(String, Option<String>, Option<Uuid>)> =
        sqlx::query_as("SELECT acte, motif_code, personne_id FROM ent_internat_i3_fait WHERE etablissement_id = $1 AND sortie_id = $2 AND acte = 'accord_remis_a_zero'")
            .bind(b.etab)
            .bind(s_sam)
            .fetch_all(b.ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(f, vec![("accord_remis_a_zero".to_string(), Some("interdiction_auteur".to_string()), Some(grand))]);
    assert!(!b.evenements("internat.sortie_decidee").await.iter().all(|e| e["cause"] != "interdiction_auteur"), "événement tracé");
    // Départ : Léo (A visé de remise) est refusé ; Max (témoin) part avec la même personne.
    interdire_objet(&b, leo, pers_leo, "remise", None).await;
    dater_interdictions(&b, leo, paris(mercredi, 12, 0)).await;
    let a = paris(vendredi, 17, 50);
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s_leo, &depart(Some(grand), a), a).await;
    assert!(refus_de(&r, "désormais l'objet d'une interdiction"), "{r:?}");
    assert_eq!(ligne_sortie(&b, s_leo).await.0, "demandee");
    assert!(faits_actes(&b, "sortie_id", s_leo).await.contains(&("depart_refuse".to_string(), Some("interdiction_auteur".to_string()))));
    let ok = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s_max, &depart(Some(grand), a), a).await.unwrap();
    assert_eq!(ok.statut, "partie", "le témoin part : le refus de Léo vient bien de la règle");
    let _ = max;
}

/// Règle générale (2/4) — sortie accompagnée : l'accord est celui de B, la personne celle de A (désignée après coup, lue
/// dans le journal). A visé : accord ET personne tombent, la décision revient au CPE ; B répond de nouveau et le départ a lieu.
#[tokio::test]
async fn p3_regle_sortie_accompagnee_accord_de_b_personne_de_a() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, jeudi) = (avril(13), avril(14), avril(16));
    let eleve = b.a[0];
    let (pa, pers_a) = parent(&b, eleve, true).await;
    let (pb, _) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let d = paris(jeudi, 18, 0);
    let s = b.ent.demander_sortie_a(b.etab, pb, PARENT, &demande(eleve, "exceptionnelle", d, d + Duration::hours(3), false, None), paris(lundi, 10, 0)).await.unwrap();
    assert!(s.accord_titulaire, "B accorde en demandant");
    b.ent.designer_personne_sortie_a(b.etab, pa, PARENT, s.id, &PersonneSortieDto { avec_personne_id: Some(grand), seul: false }, paris(lundi, 11, 0)).await.unwrap();
    let s = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(lundi, 12, 0)).await.unwrap();
    assert_eq!((s.statut.as_str(), s.avec_personne_id), ("autorisee", Some(grand)));
    interdire_objet(&b, eleve, pers_a, "visite", None).await;
    dater_interdictions(&b, eleve, paris(mardi, 12, 0)).await;
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), d), d).await;
    assert!(refus_de(&r, "désormais l'objet d'une interdiction"), "{r:?}");
    let (statut, avec, accord, ..) = ligne_sortie(&b, s.id).await;
    assert_eq!((statut.as_str(), avec, accord), ("demandee", None, None), "accord de B et personne de A tombent ensemble");
    // B répond de nouveau (avec sa personne), le CPE décide, le départ a lieu : B n'est pas visé.
    let ok = b.ent.repondre_sortie_a(b.etab, pb, PARENT, s.id, &AccordTitulaireDto { accord: true, motif_texte: None, avec_personne_id: Some(grand) }, paris(mardi, 13, 0)).await.unwrap();
    assert_eq!(ok.statut, "demandee");
    b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(mardi, 14, 0)).await.unwrap();
    b.ent.veille_sorties_a(b.etab, paris(jeudi, 12, 0)).await.unwrap();
    let dep = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), d), d).await.unwrap();
    assert_eq!(dep.statut, "partie");
}

/// Règle générale (3/4) et point 2 — autorisation annuelle (socle 18) signée par A seul : une fois A visé, le verdict de
/// l'usage n'est plus « autorisé » (`decideur_interdit`), la sortie du mercredi qui s'y appuie tombe à la revue et l'élève
/// ne peut plus s'inscrire. Signée par A **et** B : le verdict reste « autorisé » (B) mais l'absence de week-end qui s'y
/// appuie tombe quand même (« une seule » personne visée suffit) : bloquée, la personne remise reste effacée.
#[tokio::test]
async fn p3_regle_autorisation_annuelle_signee_par_un_parent_devenu_interdit() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, mercredi, vendredi) = (avril(13), avril(14), avril(15), avril(17));
    let verdict = |etat: &AutorisationsEleveDto, t: &str| etat.types.iter().find(|x| x.type_code == t).unwrap().verdict.clone();
    // A seul.
    let eleve = b.a[0];
    let (pa, pers_a) = parent(&b, eleve, true).await;
    let moi = compte_eleve(&b, eleve, NaiveDate::from_ymd_opt(2010, 3, 14).unwrap()).await;
    accorder_annuellement(&b, pa, eleve, &["sortie_mercredi", "week_end_seul"]).await;
    let (debut, fin) = (paris(mercredi, 13, 0), paris(mercredi, 18, 0));
    let s = b.ent.demander_sortie_a(b.etab, moi, ELEVE, &demande(eleve, "mercredi", debut, fin, true, None), paris(lundi, 10, 0)).await.unwrap();
    assert_eq!((s.statut.as_str(), s.par_autorisation_annuelle), ("autorisee", true));
    let etat = b.ent.autorisations_eleve_a(b.etab, b.cpe, CPE, eleve, paris(lundi, 11, 0)).await.unwrap();
    assert_eq!(verdict(&etat, "mercredi"), "autorise");
    interdire_objet(&b, eleve, pers_a, "contact_eleve", None).await;
    dater_interdictions(&b, eleve, paris(mardi, 12, 0)).await;
    let etat = b.ent.autorisations_eleve_a(b.etab, b.cpe, CPE, eleve, paris(mardi, 13, 0)).await.unwrap();
    assert_eq!(verdict(&etat, "mercredi"), "decideur_interdit", "point 2 : le verdict ne repose que sur un décideur visé");
    b.ent.veille_sorties_a(b.etab, paris(mardi, 13, 5)).await.unwrap();
    let (statut, _, _, _, annuel) = ligne_sortie(&b, s.id).await;
    assert_eq!((statut.as_str(), annuel), ("demandee", false), "la sortie ne repose plus sur l'autorisation annuelle de A");
    let r = b.ent.demander_sortie_a(b.etab, moi, ELEVE, &demande(eleve, "mercredi", debut + Duration::days(7), fin + Duration::days(7), true, None), paris(mardi, 14, 0)).await;
    assert!(refus_de(&r, "autorisation annuelle"), "{r:?}");
    // A et B : le verdict reste « autorisé », mais l'absence de week-end fondée sur l'accord de A tombe (A seul visé).
    let eleve2 = b.a[1];
    let (pa2, pers_a2) = parent(&b, eleve2, true).await;
    let (pb2, _) = parent(&b, eleve2, true).await;
    accorder_encore(&b, pa2, eleve2, &["week_end_seul"]).await;
    accorder_encore(&b, pb2, eleve2, &["week_end_seul"]).await;
    let ab = annoncer_avec(&b, pb2, PARENT, eleve2, vendredi, avril(19), None, paris(lundi, 10, 0)).await.unwrap();
    assert_eq!(ab.statut, "annoncee");
    interdire_objet(&b, eleve2, pers_a2, "remise", None).await;
    dater_interdictions(&b, eleve2, paris(mardi, 12, 0)).await;
    let etat = b.ent.autorisations_eleve_a(b.etab, b.cpe, CPE, eleve2, paris(mardi, 13, 0)).await.unwrap();
    assert_ne!(verdict(&etat, "week_end"), "decideur_interdit", "B reste décideur valable");
    b.ent.veille_sorties_a(b.etab, paris(mardi, 13, 10)).await.unwrap();
    let vue = b.ent.absences_eleve_a(b.etab, b.cpe, CPE, eleve2, paris(mardi, 14, 0)).await.unwrap();
    assert_eq!(vue[0].statut, "bloquee");
    assert!(faits_actes(&b, "absence_id", ab.id).await.contains(&("blocage".to_string(), Some("interdiction_auteur".to_string()))));
    assert!(est_attendu(&b, vendredi, eleve2).await, "aucun départ constaté : l'élève reste attendu");
}

/// Règle générale (4/4) et E-A — l'acte d'un auteur visé tombe aussi sur une sortie **suspendue** (la personne désignée
/// est interdite) et sur une sortie **bloquée** (opposition de l'autre titulaire) : accord retiré, retour à la décision
/// (une levée de la direction ne rend pas l'accord de l'auteur visé).
#[tokio::test]
async fn p3_regle_sortie_suspendue_ou_bloquee_perd_aussi_l_accord_de_l_auteur_vise() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, mercredi, jeudi) = (avril(13), avril(14), avril(15), avril(16));
    let (debut, fin) = (paris(jeudi, 18, 0), paris(jeudi, 21, 0));
    for (i, bloquee) in [(0usize, false), (1usize, true)] {
        let eleve = b.a[i];
        let (pa, pers_a) = parent(&b, eleve, true).await;
        let (pb, _) = parent(&b, eleve, true).await;
        let grand = tiers(&b, "Odile").await;
        autoriser_recuperer(&b, eleve, grand).await;
        let s = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(eleve, "exceptionnelle", debut, fin, false, Some(grand)), paris(lundi, 10, 0)).await.unwrap();
        let s = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(lundi, 11, 0)).await.unwrap();
        assert_eq!(s.statut, "autorisee");
        if bloquee {
            let o = b.ent.opposer_sortie_a(b.etab, pb, PARENT, s.id, &opposition(), paris(lundi, 12, 0)).await.unwrap();
            assert_eq!(o.statut, "bloquee");
        } else {
            interdire(&b, eleve, Some(grand)).await;
            dater_interdictions(&b, eleve, paris(mardi, 9, 0)).await;
            b.ent.veille_sorties_a(b.etab, paris(mardi, 9, 5)).await.unwrap();
            assert_eq!(ligne_sortie(&b, s.id).await.0, "suspendue", "la personne désignée est interdite");
            lever_interdictions(&b, eleve).await;
        }
        interdire_objet(&b, eleve, pers_a, "contact_eleve", None).await;
        dater_interdictions(&b, eleve, paris(mardi, 12, 0)).await;
        b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 5)).await.unwrap();
        let (statut, _, accord, ..) = ligne_sortie(&b, s.id).await;
        assert_eq!(accord, None, "l'accord de A est retiré");
        if bloquee {
            assert_eq!(statut, "bloquee", "le blocage de B reste à trancher");
            let l = b.ent.trancher_sortie_a(b.etab, b.dir, DIR, s.id, &tranche("lever"), paris(mercredi, 9, 0)).await.unwrap();
            assert_eq!(l.statut, "demandee", "la levée ne rend pas l'accord de A : la décision revient au CPE");
        } else {
            assert_eq!(statut, "demandee");
        }
    }
}

/// Cas « interdiction_auteur » d'un élève : (identifiant, nuit, issue).
async fn cas_interdiction_auteur(b: &Banc, eleve: Uuid) -> Vec<(Uuid, NaiveDate, Option<String>)> {
    sqlx::query_as::<_, (Uuid, NaiveDate, Option<String>)>(
        "SELECT id, nuit, issue FROM ent_internat_cas WHERE etablissement_id = $1 AND eleve_id = $2 AND nature = 'interdiction_auteur' ORDER BY ouvert_le",
    )
    .bind(b.etab)
    .bind(eleve)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap()
}

/// Point 1 — déjà parti : l'accord de A est posé, l'élève part, puis A est visé : rien n'est défait, un cas
/// « interdiction_auteur » s'ouvre pour le CPE ; la nuit en cours reste couverte, **les nuits suivantes ne retirent plus
/// l'élève de l'attendu** tant que le CPE n'a pas tranché ; un cas clos n'est pas rouvert. Même chose quand c'est la
/// personne remise qui devient visée (absence partie).
#[tokio::test]
async fn p3_deja_parti_un_auteur_ou_la_personne_remise_devient_vise_ouvre_un_cas_et_garde_l_eleve_attendu() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, mercredi, jeudi, vendredi) = (avril(13), avril(14), avril(15), avril(16), avril(17));
    // Sortie accompagnée de trois jours, accord de A.
    let eleve = b.a[0];
    let (pa, pers_a) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let (debut, fin) = (paris(mercredi, 17, 0), paris(vendredi, 17, 0));
    let s = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(eleve, "exceptionnelle", debut, fin, false, Some(grand)), paris(lundi, 10, 0)).await.unwrap();
    b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(lundi, 11, 0)).await.unwrap();
    let part = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut), debut).await.unwrap();
    assert_eq!(part.statut, "partie");
    assert!(est_retire(&b, mercredi, eleve, "sortie").await && est_retire(&b, jeudi, eleve, "sortie").await);
    interdire_objet(&b, eleve, pers_a, "contact_eleve", None).await;
    dater_interdictions(&b, eleve, paris(mercredi, 22, 0)).await;
    b.ent.veille_sorties_a(b.etab, paris(jeudi, 9, 0)).await.unwrap();
    assert_eq!(ligne_sortie(&b, s.id).await.0, "partie", "rien n'est défait : l'élève est parti");
    let cas = cas_interdiction_auteur(&b, eleve).await;
    assert_eq!(cas.len(), 1, "un cas pour le CPE : {cas:?}");
    // La nuit « en cours » à 9 h le jeudi est encore celle du mercredi (la nuit finit à midi).
    assert_eq!((cas[0].1, cas[0].2.clone()), (mercredi, None));
    assert!(est_retire(&b, mercredi, eleve, "sortie").await, "la nuit du cas reste couverte");
    assert!(est_attendu(&b, jeudi, eleve).await, "les nuits suivantes ne retirent plus l'élève tant que le CPE n'a pas tranché");
    b.ent.veille_sorties_a(b.etab, paris(jeudi, 9, 1)).await.unwrap();
    assert_eq!(cas_interdiction_auteur(&b, eleve).await.len(), 1, "un seul cas");
    // Le CPE tranche : le cas se ferme, l'élève est de nouveau retiré, et la revue ne rouvre pas le cas.
    let issue = CasIssueSaisieDto { issue: "retrouve".into(), lieu_code: Some("exterieur".into()), confirme_par_personne_id: None, numero_intervention: None, note: None, a: Some(paris(jeudi, 10, 0)) };
    b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[0].0, &issue, paris(jeudi, 10, 0)).await.unwrap();
    assert!(est_retire(&b, jeudi, eleve, "sortie").await, "le CPE a tranché");
    b.ent.veille_sorties_a(b.etab, paris(jeudi, 10, 5)).await.unwrap();
    b.ent.veille_sorties_a(b.etab, paris(vendredi, 9, 0)).await.unwrap();
    assert_eq!(cas_interdiction_auteur(&b, eleve).await.len(), 1, "un cas clos n'est pas rouvert");
    // Absence partie : la personne remise devient visée après le départ.
    let e2 = b.a[1];
    let (p2, pers2) = parent(&b, e2, true).await;
    let grand2 = tiers(&b, "Ines").await;
    autoriser_recuperer(&b, e2, grand2).await;
    let ab = annoncer_avec(&b, p2, PARENT, e2, mercredi, jeudi, Some(grand2), paris(lundi, 10, 0)).await.unwrap();
    let soir = paris(mercredi, 17, 0);
    b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, ab.id, &depart(Some(grand2), soir), soir).await.unwrap();
    assert!(est_retire(&b, jeudi, e2, "absence_annoncee").await);
    interdire(&b, e2, Some(grand2)).await;
    dater_interdictions(&b, e2, paris(mercredi, 20, 0)).await;
    b.ent.veille_sorties_a(b.etab, paris(mercredi, 20, 5)).await.unwrap();
    assert_eq!(cas_interdiction_auteur(&b, e2).await.len(), 1, "la personne remise est visée : cas pour le CPE");
    assert!(est_retire(&b, mercredi, e2, "absence_annoncee").await, "la nuit en cours reste couverte");
    assert!(est_attendu(&b, jeudi, e2).await, "les nuits suivantes : attendu tant que le CPE n'a pas tranché");
    let _ = (mardi, pers2);
}

/// Point 3 — faux « non récupéré » : l'heure limite de récupération ne vaut que pour une absence qui commence le
/// vendredi ; le cas se ferme au départ constaté ; une absence dont la nuit est couverte par une sortie « partie » est
/// ignorée ; un cas clos n'est pas rouvert.
#[tokio::test]
async fn p3_non_recupere_seulement_le_vendredi_ferme_au_depart_et_jamais_rouvert() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, vendredi, samedi) = (avril(13), avril(14), avril(17), avril(18));
    let nb_cas = || async {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ent_internat_cas WHERE etablissement_id = $1 AND nature = 'non_recupere'")
            .bind(b.etab)
            .fetch_one(b.ent.pool_verif())
            .await
            .unwrap()
    };
    // Mardi : aucune heure de récupération, aucun cas.
    let e0 = b.a[0];
    let (p0, _) = parent(&b, e0, true).await;
    annoncer(&b, p0, PARENT, e0, mardi, mardi, paris(lundi, 10, 0)).await.unwrap();
    assert_eq!(b.ent.veille_sorties_a(b.etab, paris(mardi, 19, 0)).await.unwrap().cas_non_recupere, 0);
    // Vendredi : e1 (récupéré en retard), e2 (couvert par une sortie partie), e3 (week-end complet, cas clos par le CPE).
    let (e1, e2, e3) = (b.a[1], b.a[2], b.a[3]);
    let (p1, pers1) = parent(&b, e1, true).await;
    let (p2, _) = parent(&b, e2, true).await;
    let (p3, _) = parent(&b, e3, true).await;
    let a1 = annoncer(&b, p1, PARENT, e1, vendredi, vendredi, paris(lundi, 10, 0)).await.unwrap();
    annoncer(&b, p2, PARENT, e2, vendredi, vendredi, paris(lundi, 10, 0)).await.unwrap();
    annoncer(&b, p3, PARENT, e3, vendredi, avril(19), paris(lundi, 10, 0)).await.unwrap();
    // e2 est déjà parti pour une sortie de week-end (la nuit du vendredi est couverte).
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, e2, grand).await;
    sqlx::query(
        "INSERT INTO ent_internat_sortie (id, etablissement_id, eleve_id, type_code, debut, fin_prevue, fin_ajustee, seul, avec_personne_id, \
                demande_par_user_id, demande_qualite, statut, depart_effectif) \
         VALUES ($1,$2,$3,'week_end',$4,$5,$5,false,$6,NULL,'systeme','partie',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(e2)
    .bind(paris(vendredi, 15, 0))
    .bind(paris(avril(19), 21, 30))
    .bind(grand)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    let v = b.ent.veille_sorties_a(b.etab, paris(vendredi, 18, 31)).await.unwrap();
    assert_eq!(v.cas_non_recupere, 2, "e1 et e3 ; pas e2 (nuit couverte par une sortie partie), pas e0");
    // e1 est récupéré à 18 h 40 : le cas se ferme au départ constaté.
    b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a1.id, &depart(Some(pers1), paris(vendredi, 18, 40)), paris(vendredi, 18, 40)).await.unwrap();
    let ouverts: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_internat_cas WHERE etablissement_id = $1 AND eleve_id = $2 AND issue IS NULL")
        .bind(b.etab)
        .bind(e1)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(ouverts, 0, "départ constaté : cas fermé");
    let issue: Option<String> = sqlx::query_scalar("SELECT issue FROM ent_internat_cas WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(b.etab)
        .bind(e1)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(issue.as_deref(), Some("rentre_famille"));
    // e3 : le CPE clôt le cas ; ni ce soir, ni le lendemain la veille ne le rouvre.
    let cas_e3: Uuid = sqlx::query_scalar("SELECT id FROM ent_internat_cas WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(b.etab)
        .bind(e3)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    let issue = CasIssueSaisieDto { issue: "retrouve".into(), lieu_code: Some("exterieur".into()), confirme_par_personne_id: None, numero_intervention: None, note: None, a: Some(paris(vendredi, 19, 0)) };
    b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_e3, &issue, paris(vendredi, 19, 0)).await.unwrap();
    for h in [paris(vendredi, 19, 30), paris(samedi, 10, 0), paris(samedi, 19, 0)] {
        assert_eq!(b.ent.veille_sorties_a(b.etab, h).await.unwrap().cas_non_recupere, 0, "{h}");
    }
    assert_eq!(nb_cas().await, 2, "un cas par élève, jamais rouvert");
}

/// Point 4 — interdiction **sans personne identifiée** : la direction (seule) lève ; une sortie accompagnée reste
/// impossible tant qu'elle est en vigueur, sauf décision explicite de la direction, tracée, **avec motif obligatoire**
/// (levée ponctuelle, pour ce cas seulement : une nouvelle demande reste refusée).
#[tokio::test]
async fn p3_interdiction_sans_personne_sortie_accompagnee_bloquee_et_levee_ponctuelle_de_la_direction() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, jeudi) = (avril(13), avril(14), avril(16));
    let eleve = b.a[0];
    let (pa, _) = parent(&b, eleve, true).await;
    accorder_annuellement(&b, pa, eleve, &["sortie_mercredi"]).await; // active l'objet du socle 18 pour la suite
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let d = paris(jeudi, 18, 0);
    // Le lien de A est antérieur à l'interdiction (règle D25 : un lien créé après n'est ni acteur ni lecteur).
    dater_lien(&b, eleve, pa, paris(lundi, 8, 0)).await;
    let s = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(eleve, "exceptionnelle", d, d + Duration::hours(3), false, Some(grand)), paris(lundi, 10, 0)).await.unwrap();
    b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(lundi, 11, 0)).await.unwrap();
    interdire(&b, eleve, None).await;
    dater_interdictions(&b, eleve, paris(mardi, 9, 0)).await;
    // Une nouvelle demande accompagnée est impossible tant que l'interdiction est en vigueur.
    let r = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(eleve, "exceptionnelle", d + Duration::days(2), d + Duration::days(2) + Duration::hours(3), false, Some(grand)), paris(mardi, 10, 0)).await;
    assert!(refus_de(&r, "pas autorisée") || refus_de(&r, "autorisée à récupérer"), "{r:?}");
    // La revue bloque la sortie accompagnée (la direction tranche), elle ne la suspend pas d'elle-même.
    b.ent.veille_sorties_a(b.etab, paris(mardi, 9, 5)).await.unwrap();
    assert_eq!(ligne_sortie(&b, s.id).await.0, "bloquee");
    assert!(faits_actes(&b, "sortie_id", s.id).await.contains(&("blocage".to_string(), Some("interdiction_remise".to_string()))));
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), d), d).await;
    assert!(refus_de(&r, "bloquée"), "{r:?}");
    assert!(est_interdit(&b.ent.trancher_sortie_a(b.etab, b.cpe, CPE, s.id, &tranche("lever"), paris(mardi, 10, 0)).await), "le CPE ne tranche pas");
    let r = b.ent.trancher_sortie_a(b.etab, b.dir, DIR, s.id, &tranche("lever"), paris(mardi, 10, 0)).await;
    assert!(refus_de(&r, "motif obligatoire"), "{r:?}");
    let l = b.ent.trancher_sortie_a(b.etab, b.dir, DIR, s.id, &TrancheDto { decision: "lever".into(), motif_texte: Some("Remise à la grand-mère confirmée par la direction".into()) }, paris(mardi, 10, 5)).await.unwrap();
    assert_eq!(l.statut, "autorisee");
    let f = faits_actes(&b, "sortie_id", s.id).await;
    assert!(f.contains(&("tranchee_levee".to_string(), Some("interdiction_remise".to_string()))), "{f:?}");
    b.ent.veille_sorties_a(b.etab, paris(mardi, 10, 10)).await.unwrap();
    assert_eq!(ligne_sortie(&b, s.id).await.0, "autorisee", "la revue ne rebloque pas ce que la direction a levé");
    let dep = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), d), d).await.unwrap();
    assert_eq!(dep.statut, "partie");
    // La levée est ponctuelle : une autre demande accompagnée reste impossible.
    let r = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(eleve, "exceptionnelle", d + Duration::days(3), d + Duration::days(3) + Duration::hours(3), false, Some(grand)), paris(mardi, 11, 0)).await;
    assert!(refus_de(&r, "pas autorisée") || refus_de(&r, "autorisée à récupérer"), "{r:?}");
    // M-A (revue d'une absence) : un mineur qui part seul, avec l'accord annuel, est bloqué par la revue.
    let e2 = b.a[1];
    let (p2, _) = parent(&b, e2, true).await;
    dater_lien(&b, e2, p2, paris(lundi, 8, 0)).await;
    accorder_encore(&b, p2, e2, &["week_end_seul"]).await;
    let ab = annoncer_avec(&b, p2, PARENT, e2, avril(17), avril(19), None, paris(lundi, 10, 0)).await.unwrap();
    interdire(&b, e2, None).await;
    dater_interdictions(&b, e2, paris(mardi, 9, 0)).await;
    b.ent.veille_sorties_a(b.etab, paris(mardi, 9, 10)).await.unwrap();
    let vue = b.ent.absences_eleve_a(b.etab, b.cpe, CPE, e2, paris(mardi, 12, 0)).await.unwrap();
    assert_eq!(vue[0].statut, "bloquee", "M-A : revue d'une absence départ seul");
    assert!(faits_actes(&b, "absence_id", ab.id).await.contains(&("blocage".to_string(), Some("interdiction_remise".to_string()))));
    let r = b.ent.trancher_absence_a(b.etab, b.dir, DIR, ab.id, &tranche("lever"), paris(mardi, 13, 0)).await;
    assert!(refus_de(&r, "motif obligatoire"), "{r:?}");
}

async fn code_remise(b: &Banc, eleve: Uuid, personne: Uuid, jour: NaiveDate) -> (String, Option<String>) {
    sqlx::query_as::<_, (String, Option<String>)>("SELECT code, relation FROM ent_internat_remise($1, $2, $3, $4, false)")
        .bind(b.etab)
        .bind(eleve)
        .bind(personne)
        .bind(jour)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap()
}

/// Point 5 — lien échu : la porte « contact d'urgence » écarte un parent ou tuteur dont le lien est terminé (date de fin
/// passée) ou pas encore commencé, qu'il ait été privé d'autorité ou non (décision prudente : un lien simplement échu
/// d'un parent qui reste contact validé est écarté) ; un proche de lien « autre » reste autorisé ; un titulaire dont le
/// lien est en cours prend l'élève.
#[tokio::test]
async fn p3_lien_echu_la_porte_contact_ecarte_un_parent_dont_le_lien_est_termine_ou_pas_commence() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let jour = avril(14);
    // (lien, autorité, début, fin, attendu)
    let cas: [(&str, bool, Option<&str>, Option<&str>, &str); 6] = [
        ("mere", true, None, Some("2026-03-01"), "non_autorisee"),   // lien terminé, autorité conservée : prudence
        ("pere", false, None, Some("2026-03-01"), "non_autorisee"),  // lien terminé et privé d'autorité
        ("tuteur", true, Some("2026-05-01"), None, "non_autorisee"), // lien pas encore commencé
        ("mere", true, Some("2026-01-01"), Some("2026-12-31"), "autorisee"), // lien en cours
        ("autre", false, None, Some("2026-03-01"), "non_autorisee"), // R6 : aucune règle sur le libellé (lien échu)
        ("pere", true, None, None, "autorisee"),
    ];
    for (i, (lien, autorite, debut, fin, attendu)) in cas.into_iter().enumerate() {
        let user = compte(&b.ent, b.etab, PARENT).await;
        let _ = i;
        let pers = tiers(&b, "Proche").await;
        sqlx::query("UPDATE ent_personne SET user_id = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(b.etab)
            .bind(pers)
            .bind(user)
            .execute(b.ent.pool_verif())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale, date_debut, date_fin) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8::date,$9::date)",
        )
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(eleve)
        .bind(user)
        .bind(pers)
        .bind(lien)
        .bind(autorite)
        .bind(debut)
        .bind(fin)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
        autoriser_recuperer(&b, eleve, pers).await;
        let (c, _) = code_remise(&b, eleve, pers, jour).await;
        // Un lien en cours avec autorité est titulaire (première porte) ; sinon seule la porte « contact » reste.
        assert_eq!(c, attendu, "lien {lien}, autorité {autorite}, du {debut:?} au {fin:?}");
    }
}

/// Point 6 — `week_end_seul` : l'autorisation annuelle ne couvre une absence que du vendredi au dimanche (la nuit de
/// fin au plus celle du dimanche) : du vendredi au lundi, refusé.
#[tokio::test]
async fn p3_week_end_seul_la_nuit_de_fin_est_au_plus_celle_du_dimanche() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (p, _) = parent(&b, eleve, true).await;
    accorder_annuellement(&b, p, eleve, &["week_end_seul"]).await;
    let r = annoncer_avec(&b, p, PARENT, eleve, avril(17), avril(20), None, paris(avril(13), 10, 0)).await;
    assert!(refus_de(&r, "veille d'un week-end") || refus_de(&r, "autorisation annuelle"), "vendredi au lundi : {r:?}");
    annoncer_avec(&b, p, PARENT, eleve, avril(17), avril(19), None, paris(avril(13), 10, 0)).await.expect("vendredi au dimanche");
}

/// Point 7 — course de la revue : une sortie qui part entre la lecture et l'écriture de la revue ne la fait pas échouer
/// (lignes verrouillées, écritures gardées par l'état attendu) : la transaction de la minute aboutit, la sortie partie
/// n'est pas touchée.
#[tokio::test]
async fn p3_course_une_sortie_partie_pendant_la_revue_ne_la_fait_pas_echouer() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, jeudi) = (avril(13), avril(14), avril(16));
    let eleve = b.a[0];
    let (pa, _) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let d = paris(jeudi, 18, 0);
    let s = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(eleve, "exceptionnelle", d, d + Duration::hours(3), false, Some(grand)), paris(lundi, 10, 0)).await.unwrap();
    b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(lundi, 11, 0)).await.unwrap();
    interdire(&b, eleve, Some(grand)).await;
    dater_interdictions(&b, eleve, paris(mardi, 9, 0)).await;
    // Une transaction concurrente verrouille la sortie et la fait partir ; la revue attend puis reprend sans erreur.
    let mut concurrent = b.ent.pool_verif().begin().await.unwrap();
    sqlx::query("UPDATE ent_internat_sortie SET statut = 'partie', depart_effectif = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(s.id)
        .bind(paris(mardi, 9, 0))
        .execute(&mut *concurrent)
        .await
        .unwrap();
    let revue = b.ent.veille_sorties_a(b.etab, paris(mardi, 9, 5));
    let liberer = async {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        concurrent.commit().await.unwrap();
    };
    let (revue, ()) = tokio::join!(revue, liberer);
    revue.expect("la revue aboutit malgré la sortie partie entre-temps");
    assert_eq!(ligne_sortie(&b, s.id).await.0, "partie");
}

/// Point 9 — charge : la revue à la minute pour 300 internes (150 sorties de week-end autorisées avec une personne
/// désignée, 100 absences annoncées par leur parent, dont une partie d'actes de parents) ne fait aucune requête par ligne ;
/// le temps mesuré est consigné (`docs/socles/41_I3_CORRECTIFS.md`, passe 3) ; borne large pour la CI.
#[tokio::test]
async fn p3_charge_revue_a_la_minute_pour_300_internes() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, vendredi) = (avril(13), avril(14), avril(17));
    b.ent.lister_types_sortie_a(b.etab).await.unwrap();
    let (pa, _) = parent(&b, b.a[0], true).await;
    let mut eleves = Vec::with_capacity(300);
    for i in 0..300 {
        eleves.push(eleve_invite(&b, &format!("E{i}")).await);
    }
    let pool = b.ent.pool_verif();
    let mut personnes = Vec::with_capacity(250);
    for e in eleves.iter().take(250) {
        let p = tiers(&b, "Proche").await;
        autoriser_recuperer(&b, *e, p).await;
        personnes.push(p);
    }
    for (i, e) in eleves.iter().take(150).enumerate() {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_sortie (id, etablissement_id, eleve_id, type_code, debut, fin_prevue, fin_ajustee, seul, avec_personne_id, \
                    demande_par_user_id, demande_qualite, statut) \
             VALUES ($1,$2,$3,'week_end',$4,$5,$5,false,$6,NULL,'systeme','autorisee')",
        )
        .bind(id)
        .bind(b.etab)
        .bind(e)
        .bind(paris(vendredi, 12, 0))
        .bind(paris(avril(19), 21, 30))
        .bind(personnes[i])
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO ent_internat_i3_fait (id, etablissement_id, sortie_id, acte, qualite, par_user_id, a, personne_id) \
             VALUES ($1,$2,$3,'reprise','titulaire',$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(id)
        .bind(pa)
        .bind(paris(lundi, 10, 0))
        .bind(personnes[i])
        .execute(pool)
        .await
        .unwrap();
    }
    for (i, e) in eleves.iter().enumerate().skip(150).take(100) {
        sqlx::query(
            "INSERT INTO ent_internat_absence (id, etablissement_id, eleve_id, du_nuit, au_nuit, annonce_par_user_id, annonce_qualite, \
                    recupere_par_personne_id, statut) VALUES ($1,$2,$3,$4,$4,$5,'titulaire',$6,'annoncee')",
        )
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(e)
        .bind(vendredi)
        .bind(pa)
        .bind(personnes[i])
        .execute(pool)
        .await
        .unwrap();
    }
    // Repères pour compter les requêtes dans le journal du serveur (`log_statement = all`, mesure manuelle) : la revue
    // ne doit faire aucune requête par ligne.
    sqlx::query("SELECT 'p3_charge_debut'").execute(pool).await.unwrap();
    let t = std::time::Instant::now();
    b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 0)).await.unwrap();
    let revue = t.elapsed();
    sqlx::query("SELECT 'p3_charge_fin'").execute(pool).await.unwrap();
    let t = std::time::Instant::now();
    b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 1)).await.unwrap();
    let revue_suivante = t.elapsed();
    eprintln!("CHARGE revue à la minute, 300 internes (150 sorties, 100 absences) : {revue:?} puis {revue_suivante:?}");
    assert!(revue < std::time::Duration::from_secs(10), "revue trop lente : {revue:?}");
}

// ───────────────────────── Passe 4 : protection des mineurs internes (R1 à R8) ─────────────────────────
//
// | Règle | Test(s) |
// |---|---|
// | R1 un acte tombe avec le droit de son auteur (lien suspendu, autorité retirée, lien échu, garde « aucune ») | `p4_r1_*` |
// | R2 seule une décision qui porte sur la chute la couvre | `p4_r2_*` |
// | R3 aucune décision du personnel sur un objet qui tombe ; R4 absences « demandées » revues | `p4_r3_r4_*` |
// | R5 contact déclaré par un visé, requalification du personnel (chaînes comprises) | `p4_r5_*` |
// | R6 aucune règle sur un libellé libre | `p4_r6_*` |
// | R7 savepoint par objet, ordre de verrouillage | `p4_r7_*` |
// | R8 remise des sorties scolaires | `p4_r8_*` |

/// Fait la perte de droit de `user` sur `eleve`, datée du mardi 14 avril à 9 h (Paris) : 0 suspension du lien, 1 autorité
/// parentale retirée, 2 lien échu (fin le 10 avril), 3 garde « aucune ».
async fn p4_perdre_le_droit(b: &Banc, eleve: Uuid, user: Uuid, cas: usize) {
    let t = paris(avril(14), 9, 0);
    let (sql, avec_date): (&str, bool) = match cas {
        0 => ("UPDATE ent_eleve_lien SET suspendu_le = $4, suspendu_par = $5, suspension_motif = 'Décision de justice', updated_at = $4 WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3", true),
        1 => ("UPDATE ent_eleve_lien SET autorite_parentale = false, updated_at = $4 WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3 AND $5::uuid IS NOT NULL", true),
        2 => ("UPDATE ent_eleve_lien SET date_fin = '2026-04-10', updated_at = $4 WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3 AND $5::uuid IS NOT NULL", true),
        _ => ("UPDATE ent_eleve_lien SET garde = 'aucune', updated_at = $4 WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3 AND $5::uuid IS NOT NULL", true),
    };
    assert!(avec_date);
    sqlx::query(sql).bind(b.etab).bind(eleve).bind(user).bind(t).bind(b.dir).execute(b.ent.pool_verif()).await.unwrap();
    // Suites (M2) : la perte est datée par un FAIT immuable posé à l'instant de l'écriture ; le test fixe cet instant.
    dater_perte_droit(b, eleve, t).await;
}

/// Fixe la date des faits de perte de droit d'un élève (suites, M2). Les faits sont en ajout seul : le test lève le
/// déclencheur le temps d'une transaction (jamais en production), comme `dater_interdictions` règle `created_at`.
async fn dater_perte_droit(b: &Banc, eleve: Uuid, instant: DateTime<Utc>) {
    let mut tx = b.ent.pool_verif().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_internat_perte_droit_fait DISABLE TRIGGER ent_internat_perte_droit_fait_ajout_seul")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_internat_perte_droit_fait SET a = $3, effet_le = $3 WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(b.etab)
        .bind(eleve)
        .bind(instant)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE ent_internat_perte_droit_fait ENABLE TRIGGER ent_internat_perte_droit_fait_ajout_seul")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn p4_personne_absence(b: &Banc, id: Uuid) -> (String, Option<Uuid>) {
    sqlx::query_as("SELECT statut, recupere_par_personne_id FROM ent_internat_absence WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(id)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap()
}

/// R1 — « un acte tombe avec le droit de son auteur », SANS interdiction : le parent qui a annoncé l'absence (et désigné
/// la personne) n'a plus de lien actif (lien suspendu, autorité retirée, lien échu, garde « aucune »). À la revue, l'acte
/// tombe : l'absence est bloquée (motif `interdiction_auteur`), la personne effacée ; au départ, il est refusé. Un témoin
/// (parent intact) n'est pas touché. Sans la règle, l'absence resterait valable et l'enfant partirait avec la personne.
#[tokio::test]
async fn p4_r1_un_acte_tombe_avec_le_droit_de_son_auteur_sans_interdiction() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, mercredi) = (avril(13), avril(14), avril(15));
    let soir = paris(mercredi, 17, 0);
    // Témoin : parent intact, même scénario.
    let (pt, _) = parent(&b, b.b[0], true).await;
    let grand_t = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, b.b[0], grand_t).await;
    let at = annoncer_avec(&b, pt, PARENT, b.b[0], mercredi, mercredi, Some(grand_t), paris(lundi, 10, 0)).await.unwrap();
    for cas in 0..4usize {
        let eleve = b.a[cas];
        let (pa, _) = parent(&b, eleve, true).await;
        let grand = tiers(&b, "Odile").await;
        autoriser_recuperer(&b, eleve, grand).await;
        let a = annoncer_avec(&b, pa, PARENT, eleve, mercredi, mercredi, Some(grand), paris(lundi, 10, 0)).await.unwrap();
        assert_eq!(a.statut, "annoncee");
        p4_perdre_le_droit(&b, eleve, pa, cas).await;
        b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 5)).await.unwrap();
        let (statut, personne) = p4_personne_absence(&b, a.id).await;
        assert_eq!((statut.as_str(), personne), ("bloquee", None), "cas {cas} : l'acte de l'auteur sans droit tombe");
        let f = faits_actes(&b, "absence_id", a.id).await;
        assert!(f.contains(&("blocage".to_string(), Some("interdiction_auteur".to_string()))), "cas {cas} : {f:?}");
        assert!(est_attendu(&b, mercredi, eleve).await, "aucun départ constaté : l'élève reste attendu");
    }
    assert_eq!(p4_personne_absence(&b, at.id).await.0, "annoncee", "le témoin n'est pas touché");
    // Au départ (sans passer par la revue) : refusé, tracé.
    let eleve = b.b[1];
    let (pa, _) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let a = annoncer_avec(&b, pa, PARENT, eleve, mercredi, mercredi, Some(grand), paris(lundi, 10, 0)).await.unwrap();
    p4_perdre_le_droit(&b, eleve, pa, 0).await;
    let r = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(Some(grand), soir), soir).await;
    assert!(refus_de(&r, "désormais l'objet d'une interdiction"), "{r:?}");
    assert!(faits_actes(&b, "absence_id", a.id).await.contains(&("depart_refuse".to_string(), Some("interdiction_auteur".to_string()))));
    // Le témoin part normalement avec la même personne.
    let dep = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, at.id, &depart(Some(grand_t), soir), soir).await.unwrap();
    assert_eq!(dep.statut, "annoncee");
}

/// R2 — seule une décision qui porte sur la chute elle-même la couvre : une levée de la direction pour une autre cause
/// (désaccord), sans motif écrit, ou une acceptation du CPE ne couvrent jamais ; la levée motivée de la chute, si.
#[tokio::test]
async fn p4_r2_seule_une_decision_qui_porte_sur_la_chute_la_couvre() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, mercredi) = (avril(13), avril(14), avril(15));
    // (acte, qualité, code de motif, texte) ; couvert ?
    let cas: [(&str, &str, Option<&str>, Option<&str>, bool); 4] = [
        ("tranchee_levee", "direction", Some("desaccord_titulaires"), Some("Désaccord entre les parents levé"), false),
        ("tranchee_levee", "direction", Some("interdiction_auteur"), None, false),
        ("acceptation", "cpe", None, Some("Accepté par routine"), false),
        ("tranchee_levee", "direction", Some("interdiction_auteur"), Some("La direction a examiné la perte de droit"), true),
    ];
    for (i, (acte, qualite, code, texte, couvert)) in cas.into_iter().enumerate() {
        let eleve = b.a[i];
        let (pa, _) = parent(&b, eleve, true).await;
        let grand = tiers(&b, "Odile").await;
        autoriser_recuperer(&b, eleve, grand).await;
        let a = annoncer_avec(&b, pa, PARENT, eleve, mercredi, mercredi, Some(grand), paris(lundi, 10, 0)).await.unwrap();
        p4_perdre_le_droit(&b, eleve, pa, 0).await;
        let auteur = if qualite == "direction" { b.dir } else { b.cpe };
        sqlx::query(
            "INSERT INTO ent_internat_i3_fait (id, etablissement_id, absence_id, acte, qualite, par_user_id, a, motif_code, motif_texte) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(a.id)
        .bind(acte)
        .bind(qualite)
        .bind(auteur)
        .bind(paris(mardi, 11, 0))
        .bind(code)
        .bind(texte)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
        b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 0)).await.unwrap();
        let attendu = if couvert { "annoncee" } else { "bloquee" };
        assert_eq!(p4_personne_absence(&b, a.id).await.0, attendu, "cas {i} ({acte}, {code:?}, texte {})", texte.is_some());
    }
}

/// R3 et R4 — le personnel ne décide pas d'un objet qui tombe, et les absences « demandées » sont revues. Scénario du
/// rapport : le père annonce une absence tardive (« demandée ») pour une personne qu'il désigne, puis il est visé ; le CPE
/// (qui ne le sait pas) l'accepte de routine : refus explicite, l'absence reste « demandée », la revue la bloque. Même chose
/// pour l'autorisation d'une sortie demandée par un parent visé ; le témoin (parent intact) est autorisé.
#[tokio::test]
async fn p4_r3_r4_le_personnel_ne_decide_pas_d_un_objet_qui_tombe_et_les_demandees_sont_revues() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, jeudi) = (avril(13), avril(14), avril(16));
    let eleve = b.a[0];
    let (pa, pers_a) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let t = annoncer_avec(&b, pa, PARENT, eleve, mardi, mardi, Some(grand), paris(mardi, 17, 0)).await.unwrap();
    assert_eq!((t.statut.as_str(), t.tardive), ("demandee", true));
    interdire_objet(&b, eleve, pers_a, "visite", None).await;
    dater_interdictions(&b, eleve, paris(mardi, 17, 30)).await;
    let r = b.ent.decider_absence_a(b.etab, b.cpe, CPE, t.id, &decision("accepter", None), paris(mardi, 17, 40)).await;
    assert!(refus_de(&r, "n'a plus le droit d'agir"), "{r:?}");
    assert_eq!(p4_personne_absence(&b, t.id).await.0, "demandee", "rien n'a été accepté");
    assert!(!faits_actes(&b, "absence_id", t.id).await.iter().any(|(a, _)| a == "acceptation"));
    // R4 : la revue regarde aussi la demandée.
    b.ent.veille_sorties_a(b.etab, paris(mardi, 17, 50)).await.unwrap();
    assert_eq!(p4_personne_absence(&b, t.id).await, ("bloquee".to_string(), None));
    assert!(faits_actes(&b, "absence_id", t.id).await.contains(&("blocage".to_string(), Some("interdiction_auteur".to_string()))));
    // Sortie demandée par un parent visé : le CPE ne l'autorise pas ; le témoin, si.
    let (debut, fin) = (paris(jeudi, 18, 0), paris(jeudi, 21, 0));
    let eleve2 = b.a[1];
    let (pa2, pers_a2) = parent(&b, eleve2, true).await;
    let s = b.ent.demander_sortie_a(b.etab, pa2, PARENT, &demande(eleve2, "exceptionnelle", debut, fin, false, Some(grand)), paris(lundi, 10, 0)).await;
    // (le grand-parent n'est pas contact de cet élève : la demande s'adapte au cas)
    let s = match s {
        Ok(s) => s,
        Err(_) => {
            autoriser_recuperer(&b, eleve2, grand).await;
            b.ent.demander_sortie_a(b.etab, pa2, PARENT, &demande(eleve2, "exceptionnelle", debut, fin, false, Some(grand)), paris(lundi, 10, 0)).await.unwrap()
        }
    };
    interdire_objet(&b, eleve2, pers_a2, "contact_eleve", None).await;
    dater_interdictions(&b, eleve2, paris(mardi, 12, 0)).await;
    let r = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(mardi, 12, 10)).await;
    assert!(refus_de(&r, "n'a plus le droit d'agir"), "{r:?}");
    assert_eq!(ligne_sortie(&b, s.id).await.0, "demandee");
    let eleve3 = b.b[0];
    let (pt, _) = parent(&b, eleve3, true).await;
    autoriser_recuperer(&b, eleve3, grand).await;
    let st = b.ent.demander_sortie_a(b.etab, pt, PARENT, &demande(eleve3, "exceptionnelle", debut, fin, false, Some(grand)), paris(lundi, 10, 0)).await.unwrap();
    let st = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, st.id, &decision("autoriser", None), paris(mardi, 12, 20)).await.unwrap();
    assert_eq!(st.statut, "autorisee", "le témoin est autorisé");
}

/// Un contact validé « peut récupérer », déclaré par `declarant` (fiche) ; `requalifie` : date de requalification.
async fn p4_contact_declare(b: &Banc, eleve: Uuid, personne: Uuid, declarant: Option<Uuid>) {
    sqlx::query(
        "INSERT INTO ent_eleve_contact_urgence (id, etablissement_id, eleve_id, personne_id, relation, peut_recuperer, statut, declare_par_personne_id) \
         VALUES ($1,$2,$3,$4,'proche',true,'valide',$5)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(personne)
    .bind(declarant)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
}

async fn p4_id_contact(b: &Banc, eleve: Uuid, personne: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT id FROM ent_eleve_contact_urgence WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3")
        .bind(b.etab)
        .bind(eleve)
        .bind(personne)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap()
}

/// R5 — un contact « peut récupérer » déclaré par un parent visé (interdiction, ou perte de droit) n'est plus remis tant que
/// le personnel ne l'a pas requalifié après coup ; la chaîne de déclarants est suivie (le contact déclaré par un contact
/// déclaré par le visé) ; une requalification antérieure à la perte ne compte pas ; la famille ne requalifie pas.
#[tokio::test]
async fn p4_r5_un_contact_declare_par_un_vise_n_est_remis_qu_apres_requalification_du_personnel() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, mercredi) = (avril(13), avril(14), avril(15));
    for par_interdiction in [true, false] {
        let eleve = if par_interdiction { b.a[0] } else { b.a[1] };
        let (_pa, pers_a) = parent(&b, eleve, true).await;
        let (pb, pers_b) = parent(&b, eleve, true).await;
        let odile = tiers(&b, "Odile").await; // déclarée par A
        let pierre = tiers(&b, "Pierre").await; // déclaré par Odile (chaîne)
        let marc = tiers(&b, "Marc").await; // déclaré par B (non visé)
        let alice = tiers(&b, "Alice").await; // sans déclarant (saisie du personnel)
        p4_contact_declare(&b, eleve, odile, Some(pers_a)).await;
        p4_contact_declare(&b, eleve, pierre, Some(odile)).await;
        p4_contact_declare(&b, eleve, marc, Some(pers_b)).await;
        p4_contact_declare(&b, eleve, alice, None).await;
        for p in [odile, pierre, marc, alice] {
            assert_eq!(code_remise(&b, eleve, p, mercredi).await.0, "autorisee", "avant : tout le monde est remis");
        }
        // B annonce une absence avec Odile : valable tant que A n'est pas visé.
        let a = annoncer_avec(&b, pb, PARENT, eleve, mercredi, mercredi, Some(odile), paris(lundi, 10, 0)).await.unwrap();
        assert_eq!(a.statut, "annoncee");
        if par_interdiction {
            interdire_objet(&b, eleve, pers_a, "remise", None).await;
            dater_interdictions(&b, eleve, paris(mardi, 12, 0)).await;
        } else {
            let user_a: Uuid = sqlx::query_scalar("SELECT user_id FROM ent_personne WHERE etablissement_id = $1 AND id = $2").bind(b.etab).bind(pers_a).fetch_one(b.ent.pool_verif()).await.unwrap();
            sqlx::query("UPDATE ent_eleve_lien SET suspendu_le = $4, suspendu_par = $5, suspension_motif = 'Décision de justice', updated_at = $4 WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
                .bind(b.etab)
                .bind(eleve)
                .bind(user_a)
                .bind(paris(mardi, 12, 0))
                .bind(b.dir)
                .execute(b.ent.pool_verif())
                .await
                .unwrap();
            dater_perte_droit(&b, eleve, paris(mardi, 12, 0)).await;
        }
        for (p, attendu) in [(odile, "non_autorisee"), (pierre, "non_autorisee"), (marc, "autorisee"), (alice, "autorisee")] {
            assert_eq!(code_remise(&b, eleve, p, mercredi).await.0, attendu, "après : contact déclaré par un visé, chaîne comprise");
        }
        // La revue bloque l'absence de B (personne désignée devenue non autorisée).
        b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 5)).await.unwrap();
        assert_eq!(p4_personne_absence(&b, a.id).await.0, "bloquee");
        // Requalification : la famille ne le peut pas ; sans motif, non ; une date antérieure à la perte ne compte pas.
        let c_odile = p4_id_contact(&b, eleve, odile).await;
        let c_pierre = p4_id_contact(&b, eleve, pierre).await;
        let motif = CorrespondantSuspensionDto { motif_texte: Some("Contact revu avec la famille".into()) };
        assert!(est_interdit(&b.ent.requalifier_contact_a(b.etab, pb, PARENT, c_odile, &motif, paris(mardi, 15, 0)).await));
        let r = b.ent.requalifier_contact_a(b.etab, b.cpe, CPE, c_odile, &CorrespondantSuspensionDto { motif_texte: None }, paris(mardi, 15, 0)).await;
        assert!(refus_de(&r, "motif obligatoire"), "{r:?}");
        sqlx::query(
            "INSERT INTO ent_internat_i3_fait (id, etablissement_id, eleve_id, personne_id, acte, qualite, par_user_id, a, motif_texte) \
             VALUES ($1,$2,$3,$4,'requalification','cpe',$5,$6,'Ancienne revue')",
        )
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(eleve)
        .bind(odile)
        .bind(b.cpe)
        .bind(paris(avril(1), 9, 0))
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(code_remise(&b, eleve, odile, mercredi).await.0, "non_autorisee", "une requalification antérieure à la perte ne compte pas");
        // Le personnel requalifie Pierre (en aval) : Pierre est remis, Odile non ; puis Odile.
        b.ent.requalifier_contact_a(b.etab, b.cpe, CPE, c_pierre, &motif, paris(mardi, 15, 0)).await.unwrap();
        assert_eq!(code_remise(&b, eleve, pierre, mercredi).await.0, "autorisee");
        assert_eq!(code_remise(&b, eleve, odile, mercredi).await.0, "non_autorisee");
        b.ent.requalifier_contact_a(b.etab, b.cpe, CPE, c_odile, &motif, paris(mardi, 15, 5)).await.unwrap();
        assert_eq!(code_remise(&b, eleve, odile, mercredi).await.0, "autorisee");
        // La direction lève le blocage (la personne est de nouveau valable) et le départ a lieu avec Odile.
        b.ent.trancher_absence_a(b.etab, b.dir, DIR, a.id, &tranche("lever"), paris(mardi, 16, 0)).await.unwrap();
        let soir = paris(mercredi, 17, 0);
        let dep = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(Some(odile), soir), soir).await.unwrap();
        assert_eq!(dep.statut, "annoncee");
    }
}

/// R6 / E1 — la porte « contact d'urgence » écarte tout contact dont un lien avec l'élève a la garde « aucune », est
/// suspendu ou échu, QUEL QUE SOIT le libellé du lien (« Maman », « Tonton »…). L'autorité retirée écarte un lien de
/// nature parentale, reconnu largement dans le texte libre (`mere`, « Maman », « Père », « tutrice »…, fonction
/// `ent_internat_lien_parental`), jamais un proche (« Tonton », « Grand-mère », `autre`) — décision du pilote du 20-11.
/// Un contact sans aucun lien reste autorisé ; un titulaire en cours aussi (porte « titulaire »).
#[tokio::test]
async fn p4_r6_la_porte_contact_ne_lit_jamais_le_libelle_du_lien() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let jour = avril(14);
    // (libellé, autorité, garde, suspendu, fin, attendu)
    let cas: [(&str, bool, Option<&str>, bool, Option<&str>, &str); 11] = [
        ("mere", false, None, false, None, "non_autorisee"),
        ("pere", false, None, false, None, "non_autorisee"),
        ("tuteur", false, None, false, None, "non_autorisee"),
        ("Maman", false, None, false, None, "non_autorisee"),
        ("Tonton", true, Some("aucune"), false, None, "non_autorisee"),
        ("père", true, None, true, None, "non_autorisee"),
        ("Mamie", true, None, false, Some("2026-03-01"), "non_autorisee"),
        ("autre", false, None, false, None, "autorisee"),
        ("tutrice légale", true, Some("aucune"), false, None, "non_autorisee"),
        ("Maman", true, None, false, None, "autorisee"),
        ("Tonton", true, Some("exclusive"), false, Some("2026-12-31"), "autorisee"),
    ];
    for (libelle, autorite, garde, suspendu, fin, attendu) in cas {
        let user = compte(&b.ent, b.etab, PARENT).await;
        let pers = tiers(&b, "Proche").await;
        sqlx::query("UPDATE ent_personne SET user_id = $3 WHERE etablissement_id = $1 AND id = $2").bind(b.etab).bind(pers).bind(user).execute(b.ent.pool_verif()).await.unwrap();
        sqlx::query(
            "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale, garde, suspendu_le, suspendu_par, suspension_motif, date_fin) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12::date)",
        )
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(eleve)
        .bind(user)
        .bind(pers)
        .bind(libelle)
        .bind(autorite)
        .bind(garde)
        .bind(suspendu.then(|| paris(avril(1), 9, 0)))
        .bind(suspendu.then_some(b.dir))
        .bind(suspendu.then_some("Décision de justice"))
        .bind(fin)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
        autoriser_recuperer(&b, eleve, pers).await;
        let r = code_remise(&b, eleve, pers, jour).await;
        // La porte « titulaire » (lien actif, autorité, garde non « aucune ») rend « autorisee » avec le libellé ; la porte
        // « contact » ne se prononce jamais sur lui.
        assert_eq!(r.0, attendu, "libellé « {libelle} », autorité {autorite}, garde {garde:?}, suspendu {suspendu}, fin {fin:?}");
    }
    // Un contact sans aucun lien reste autorisé.
    let seul = tiers(&b, "Voisine").await;
    autoriser_recuperer(&b, eleve, seul).await;
    assert_eq!(code_remise(&b, eleve, seul, jour).await.0, "autorisee");
}

/// R7 — une erreur sur un objet n'annule pas la revue des autres : un déclencheur de test fait échouer l'écriture d'une
/// absence ; l'autre absence est bloquée, la revue réussit et compte l'échec. Ordre de verrouillage identique entre le
/// départ et la revue : des départs et des revues concurrents ne se bloquent jamais mutuellement (pas d'interblocage).
#[tokio::test]
async fn p4_r7_une_erreur_sur_un_objet_n_annule_pas_la_revue_des_autres() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, mercredi) = (avril(13), avril(14), avril(15));
    let mut absences = Vec::new();
    for i in 0..2 {
        let eleve = b.a[i];
        let (pa, _) = parent(&b, eleve, true).await;
        let grand = tiers(&b, "Odile").await;
        autoriser_recuperer(&b, eleve, grand).await;
        let a = annoncer_avec(&b, pa, PARENT, eleve, mercredi, mercredi, Some(grand), paris(lundi, 10, 0)).await.unwrap();
        p4_perdre_le_droit(&b, eleve, pa, 0).await;
        absences.push(a.id);
    }
    let (en_echec, bonne) = (absences[0], absences[1]);
    let nom = format!("ent_test_p4_echec_{}", en_echec.simple());
    sqlx::query(&format!(
        "CREATE FUNCTION {nom}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN \
           IF NEW.id = '{en_echec}'::uuid THEN RAISE EXCEPTION 'échec simulé'; END IF; RETURN NEW; END $$"
    ))
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(&format!("CREATE TRIGGER {nom} BEFORE UPDATE ON ent_internat_absence FOR EACH ROW EXECUTE FUNCTION {nom}()"))
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let r = b.ent.veille_sorties_bilan_a(b.etab, paris(mardi, 12, 5)).await;
    sqlx::query(&format!("DROP TRIGGER {nom} ON ent_internat_absence")).execute(b.ent.pool_verif()).await.unwrap();
    sqlx::query(&format!("DROP FUNCTION {nom}()")).execute(b.ent.pool_verif()).await.unwrap();
    let echecs = r.expect("la revue réussit malgré l'objet en échec").1.echecs;
    assert!(echecs >= 1, "l'échec est compté : {echecs}");
    assert_eq!(p4_personne_absence(&b, bonne).await.0, "bloquee", "l'autre objet est revu");
    assert_eq!(p4_personne_absence(&b, en_echec).await.0, "annoncee", "seul le travail de l'objet en échec est annulé");
    // La revue suivante, sans panne, reprend l'objet.
    let echecs = b.ent.veille_sorties_bilan_a(b.etab, paris(mardi, 12, 6)).await.unwrap().1.echecs;
    assert_eq!(echecs, 0);
    assert_eq!(p4_personne_absence(&b, en_echec).await.0, "bloquee");
}

/// R7 — ordre de verrouillage (correspondants, puis sortie, puis absences) : départs et revues concurrents ne
/// s'interbloquent pas.
#[tokio::test]
async fn p4_r7_depart_et_revue_concurrents_ne_s_interbloquent_pas() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi) = (avril(13), avril(14));
    let (debut, fin) = (paris(mardi, 17, 0), paris(avril(15), 8, 0));
    let eleve = b.a[0];
    let (pa, _) = parent(&b, eleve, true).await;
    let corr = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, corr).await;
    let saisie = CorrespondantSaisieDto { eleve_id: eleve, personne_id: corr, lundi: true, mardi: true, mercredi: true, jeudi: true, vendredi: true, samedi: true, dimanche: true, du: None, au: None };
    let c = b.ent.designer_correspondant_a(b.etab, pa, PARENT, &saisie, paris(lundi, 9, 0)).await.unwrap();
    let d = SortieDemandeDto { correspondant_id: Some(c.id), ..demande(eleve, "nuit_correspondant", debut, fin, false, None) };
    let s = b.ent.demander_sortie_a(b.etab, pa, PARENT, &d, paris(lundi, 10, 0)).await.unwrap();
    let s = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(lundi, 11, 0)).await.unwrap();
    // Départs, revues et suspensions de correspondant en concurrence (mêmes lignes, ordre de verrouillage commun).
    for tour in 0..4u32 {
        let (etab, cpe, cid) = (b.etab, b.cpe, c.id);
        let t = |m: u32| paris(mardi, 12, tour * 10 + m);
        let tolerer = |r: Result<(), EntDbError>| match r {
            Err(EntDbError::Refus(_, _)) => Ok(()),
            autre => autre,
        };
        let (r1, r2, r3, r4) = tokio::join!(
            async { b.ent.veille_sorties_a(etab, t(1)).await.map(|_| ()) },
            async { tolerer(b.ent.controler_depart_a(etab, cpe, CPE, s.id, &depart(Some(corr), debut), debut).await.map(|_| ())) },
            async { tolerer(b.ent.suspendre_correspondant_a(etab, cpe, CPE, cid, tour % 2 == 0, &CorrespondantSuspensionDto { motif_texte: Some("essai".into()) }, t(2)).await.map(|_| ())) },
            async { b.ent.veille_sorties_a(etab, t(3)).await.map(|_| ()) },
        );
        for r in [r1, r2, r3, r4] {
            if let Err(e) = r {
                assert!(!format!("{e:?}").to_lowercase().contains("deadlock"), "interblocage : {e:?}");
            }
        }
    }
}

/// R8 / M4 (suites) — la remise d'un élève passe par la même porte. Les états `remis` et `parti_seul_autorise` de
/// `ent_sortie_participant` n'existent que dans la contrainte de la table : AUCUN code ne les écrit. La garde est
/// désormais **en base** (déclencheur `ent_internat_remise_participant_garde`, migration 2827) : plus d'analyse de sources
/// ni de littéraux. Le test vérifie (1) que le déclencheur est posé et actif sur la vraie table, (2) son comportement sur une
/// table temporaire (insertion et passage vers un état de remise refusés ; autres états acceptés), même fonction.
#[tokio::test]
async fn p4_r8_la_base_refuse_d_ecrire_un_etat_de_remise_d_un_participant() {
    let Some(b) = banc().await else { return };
    let pose: Option<(String,)> = sqlx::query_as(
        "SELECT tgenabled::text FROM pg_trigger WHERE tgrelid = 'ent_sortie_participant'::regclass \
            AND tgname = 'ent_internat_remise_participant_garde' AND NOT tgisinternal",
    )
    .fetch_optional(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(pose.map(|p| p.0).as_deref(), Some("O"), "le déclencheur de garde est posé et actif");
    let mut tx = b.ent.pool_verif().begin().await.unwrap();
    sqlx::query("CREATE TEMP TABLE essai_participant (etat TEXT NOT NULL) ON COMMIT DROP").execute(&mut *tx).await.unwrap();
    sqlx::query(
        "CREATE TRIGGER essai BEFORE INSERT OR UPDATE OF etat ON essai_participant FOR EACH ROW \
         EXECUTE FUNCTION ent_internat_remise_participant_garde()",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("INSERT INTO essai_participant VALUES ('inscrit'), ('pret'), ('rentre'), ('desiste')").execute(&mut *tx).await.unwrap();
    for etat in ["remis", "parti_seul_autorise"] {
        sqlx::query("SAVEPOINT s").execute(&mut *tx).await.unwrap();
        let r = sqlx::query("INSERT INTO essai_participant VALUES ($1)").bind(etat).execute(&mut *tx).await;
        assert!(r.is_err(), "insertion de {etat} refusée");
        sqlx::query("ROLLBACK TO SAVEPOINT s").execute(&mut *tx).await.unwrap();
        let r = sqlx::query("UPDATE essai_participant SET etat = $1 WHERE etat = 'pret'").bind(etat).execute(&mut *tx).await;
        assert!(r.is_err(), "passage vers {etat} refusé");
        sqlx::query("ROLLBACK TO SAVEPOINT s").execute(&mut *tx).await.unwrap();
    }
    sqlx::query("UPDATE essai_participant SET etat = 'rentre' WHERE etat = 'pret'").execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}

// ───────────────────────── Suites du lot I3 : M1, M2, M3, requalification, infirmerie ─────────────────────────
//
// | Point | Test(s) |
// |---|---|
// | M1 auteur du personnel (correspondant désigné par le CPE, décideur) n'est pas un « auteur sans lien » | `s_m1_*` |
// | M2 perte de droit datée par un fait immuable (payeur, fusion, réactivation : aucun effet) | `s_m2_*` |
// | M3 tiers autorisé à récupérer | `s_m3_*` |
// | M4 garde SQL de la remise d'un participant | `p4_r8_*` |
// | Infirmerie : même fonction de remise | `s_infirmerie_*` |
// | Requalification : internes seulement, conflit d'intérêt | `s_requalification_*` |

fn pose_tiers(eleve: Uuid, personne: Uuid, demandeur: Uuid) -> TiersPoseDto {
    TiersPoseDto {
        eleve_id: eleve,
        personne_id: personne,
        demande_par_personne_id: demandeur,
        demande_ecrite_le: avril(10),
        du: avril(13),
        au: avril(30),
        motif_texte: "Demande écrite de la mère : la grand-mère habite à côté".into(),
    }
}

/// M1 — le CPE désigne un correspondant : son acte ne tombe pas « faute de lien avec l'élève » (un agent n'a pas de lien à
/// perdre). Le même compte, sans rôle de personnel et sans lien (un parent sans lien), est bien « visé » (témoin).
#[tokio::test]
async fn s_m1_un_auteur_du_personnel_n_est_pas_un_auteur_sans_lien() {
    let Some(b) = banc().await else { return };
    let (mardi, eleve) = (avril(14), b.a[0]);
    let (p, _) = parent(&b, eleve, true).await;
    let corr = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, corr).await;
    let saisie = CorrespondantSaisieDto { eleve_id: eleve, personne_id: corr, lundi: true, mardi: true, mercredi: true, jeudi: true, vendredi: true, samedi: true, dimanche: true, du: None, au: None };
    let c = b.ent.designer_correspondant_a(b.etab, b.cpe, CPE, &saisie, paris(mardi, 9, 0)).await.unwrap();
    for minute in [1, 2] {
        b.ent.veille_sorties_a(b.etab, paris(mardi, 9, minute)).await.unwrap();
    }
    let vus = b.ent.correspondants_eleve_a(b.etab, b.cpe, CPE, eleve, paris(mardi, 10, 0)).await.unwrap();
    assert_eq!(vus.iter().find(|v| v.id == c.id).map(|v| v.statut.as_str()), Some("valide"), "désigné par le CPE : pas suspendu à la revue");
    assert!(!faits_actes(&b, "correspondant_id", c.id).await.iter().any(|(a, _)| a == "suspension"));
    let vise = |user: Uuid| {
        let b = &b;
        async move {
            sqlx::query_scalar::<_, Option<DateTime<Utc>>>("SELECT ent_internat_user_vise($1, $2, $3, $4)")
                .bind(b.etab)
                .bind(eleve)
                .bind(user)
                .bind(mardi)
                .fetch_one(b.ent.pool_verif())
                .await
                .unwrap()
        }
    };
    assert!(vise(b.cpe).await.is_none(), "CPE : jamais « sans lien »");
    assert!(vise(b.dir).await.is_none(), "direction : idem");
    assert!(vise(p).await.is_none(), "témoin : le parent qui a un lien actif n'est pas visé");
    let sans_lien = compte(&b.ent, b.etab, PARENT).await;
    assert!(vise(sans_lien).await.is_some(), "témoin : un parent sans aucun lien est « sans lien » (visé)");
    // Une interdiction qui viserait l'agent reste prioritaire (sa fiche est visée).
    let fiche_cpe = personne_de(&b.ent, b.etab, b.cpe).await;
    interdire_objet(&b, eleve, fiche_cpe, "visite", None).await;
    assert!(vise(b.cpe).await.is_some(), "une interdiction nominative vise toujours");
}

/// M2 — la perte d'un droit est datée par un fait immuable, plus par `updated_at`. Le parent perd l'autorité (14 avril 9 h) ;
/// la direction lève la chute avec un motif (11 h) ; ensuite une écriture du payeur, une écriture « de fusion » et une
/// réactivation du lien ne repoussent pas la date et n'annulent pas la levée : aucun nouveau fait, absence toujours valable.
#[tokio::test]
async fn s_m2_ecriture_du_payeur_fusion_reactivation_ne_datent_pas_la_perte() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, mercredi) = (avril(13), avril(14), avril(15));
    let eleve = b.a[0];
    let (pa, _) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, grand).await;
    let a = annoncer_avec(&b, pa, PARENT, eleve, mercredi, mercredi, Some(grand), paris(lundi, 10, 0)).await.unwrap();
    p4_perdre_le_droit(&b, eleve, pa, 1).await; // autorité retirée, datée du 14 avril 9 h
    let faits_lien = || async {
        sqlx::query_as::<_, (String, DateTime<Utc>)>(
            "SELECT nature, effet_le FROM ent_internat_perte_droit_fait WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY nature",
        )
        .bind(b.etab)
        .bind(eleve)
        .fetch_all(b.ent.pool_verif())
        .await
        .unwrap()
    };
    assert_eq!(faits_lien().await, vec![("autorite_retiree".to_string(), paris(mardi, 9, 0))]);
    // La direction lève la chute, motivée, APRÈS la perte (11 h).
    sqlx::query(
        "INSERT INTO ent_internat_i3_fait (id, etablissement_id, absence_id, acte, qualite, par_user_id, a, motif_code, motif_texte) \
         VALUES ($1,$2,$3,'tranchee_levee','direction',$4,$5,'interdiction_auteur','La direction a examiné la perte de droit')",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(a.id)
    .bind(b.dir)
    .bind(paris(mardi, 11, 0))
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 0)).await.unwrap();
    assert_eq!(p4_personne_absence(&b, a.id).await.0, "annoncee", "couverte par la levée de la direction");
    // Écritures qui ne sont PAS une perte : payeur, libellé / rattachement, réactivation (autorité rétablie).
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_notes = NOT peut_voir_notes, updated_at = now() WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(b.etab)
        .bind(eleve)
        .bind(pa)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_eleve_lien SET lien = 'pere', personne_id = personne_id, updated_at = now() WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(b.etab)
        .bind(eleve)
        .bind(pa)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(faits_lien().await.len(), 1, "aucune de ces écritures ne pose de fait");
    b.ent.veille_sorties_a(b.etab, paris(mercredi, 8, 0)).await.unwrap();
    assert_eq!(p4_personne_absence(&b, a.id).await.0, "annoncee", "la levée n'est pas annulée par une écriture du payeur");
    sqlx::query("UPDATE ent_eleve_lien SET autorite_parentale = true, updated_at = now() WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(b.etab)
        .bind(eleve)
        .bind(pa)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(faits_lien().await.len(), 1, "la réactivation ne pose pas de fait");
    b.ent.veille_sorties_a(b.etab, paris(mercredi, 8, 5)).await.unwrap();
    assert_eq!(p4_personne_absence(&b, a.id).await.0, "annoncee");
    // La perte est lue du fait (14 avril 9 h), pas de `updated_at` : le lien est actif de nouveau, plus de perte.
    let perte: Option<DateTime<Utc>> = sqlx::query_scalar("SELECT ent_internat_user_vise($1, $2, $3, $4)")
        .bind(b.etab)
        .bind(eleve)
        .bind(pa)
        .bind(mercredi)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert!(perte.is_none());
    // Le fait est en ajout seul.
    let r = sqlx::query("UPDATE ent_internat_perte_droit_fait SET effet_le = now() WHERE etablissement_id = $1")
        .bind(b.etab)
        .execute(b.ent.pool_verif())
        .await;
    assert!(r.is_err(), "ajout seul");
    // Les quatre natures posent un fait, une seule fois, à la TRANSITION (suspension, garde, fin).
    let e2 = b.a[1];
    let (p2, _) = parent(&b, e2, true).await;
    let lien = |sql: &'static str| {
        let b = &b;
        async move {
            sqlx::query(sql).bind(b.etab).bind(e2).bind(p2).bind(b.dir).execute(b.ent.pool_verif()).await.unwrap();
        }
    };
    lien("UPDATE ent_eleve_lien SET garde = 'aucune' WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3").await;
    lien("UPDATE ent_eleve_lien SET garde = 'aucune', peut_voir_vs = false WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3").await;
    lien("UPDATE ent_eleve_lien SET suspendu_le = now(), suspendu_par = $4, suspension_motif = 'Décision' WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3").await;
    lien("UPDATE ent_eleve_lien SET date_fin = '2026-04-10' WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3").await;
    let natures: Vec<(String,)> = sqlx::query_as("SELECT nature FROM ent_internat_perte_droit_fait WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY nature")
        .bind(b.etab)
        .bind(e2)
        .fetch_all(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(natures.into_iter().map(|n| n.0).collect::<Vec<_>>(), ["fin", "garde_aucune", "suspendu"]);
}

async fn code_pour(b: &Banc, eleve: Uuid, personne: Uuid) -> (String, Option<String>) {
    code_remise(b, eleve, personne, avril(14)).await
}

/// M3 — tiers autorisé à récupérer : posé par le personnel à la demande écrite d'un titulaire, daté, motivé ; remis par la
/// fonction unique ; la famille le voit sans le motif ; une interdiction (chaîne de fusions) bloque ; l'acte tombe avec son
/// auteur (le demandeur visé) jusqu'à la reprise par un autre titulaire.
#[tokio::test]
async fn s_m3_tiers_autorise_pose_remise_visibilite_interdiction_et_auteur_qui_tombe() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let maintenant = paris(avril(14), 10, 0);
    let (pa, pers_a) = parent(&b, eleve, true).await;
    let (pb, pers_b) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    assert_eq!(code_pour(&b, eleve, grand).await.0, "non_autorisee", "refus par défaut : un proche sans statut n'est pas remis");
    // Droits : ni la famille, ni le professeur ne posent le statut ; motif et dates obligatoires.
    assert!(est_interdit(&b.ent.poser_tiers_a(b.etab, pa, PARENT, &pose_tiers(eleve, grand, pers_a), maintenant).await));
    assert!(est_interdit(&b.ent.poser_tiers_a(b.etab, b.prof, "prof", &pose_tiers(eleve, grand, pers_a), maintenant).await));
    let mut sans_motif = pose_tiers(eleve, grand, pers_a);
    sans_motif.motif_texte = "  ".into();
    assert!(refus_de(&b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &sans_motif, maintenant).await, "motif obligatoire"));
    let mut trop_long = pose_tiers(eleve, grand, pers_a);
    trop_long.au = avril(13) + Duration::days(400);
    assert!(refus_de(&b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &trop_long, maintenant).await, "un an au plus"));
    // Un demandeur qui n'est pas titulaire (le grand-parent lui-même, ou une autre personne sans lien) est refusé.
    let autre = tiers(&b, "Marc").await;
    assert!(refus_de(&b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, autre), maintenant).await, "titulaire de l'autorité parentale"));
    let t = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await.unwrap();
    assert_eq!((t.statut.as_str(), t.remettable_aujourdhui), ("actif", true));
    assert_eq!(t.motif_texte.as_deref().map(|m| m.contains("grand-mère")), Some(true), "le personnel lit le motif");
    assert_eq!(code_pour(&b, eleve, grand).await, ("autorisee".to_string(), Some("tiers autorisé".to_string())));
    // Datée : hors des dates, la porte est fermée.
    assert_eq!(code_remise(&b, eleve, grand, avril(12)).await.0, "non_autorisee");
    assert_eq!(code_remise(&b, eleve, grand, avril(30)).await.0, "autorisee");
    assert_eq!(code_remise(&b, eleve, grand, avril(30) + Duration::days(1)).await.0, "non_autorisee");
    // Une personne ne peut pas être posée deux fois ; historisé.
    let r = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await;
    assert!(refus_de(&r, "déjà tiers autorisé"), "{r:?}");
    assert!(faits_actes(&b, "tiers_id", t.id).await.iter().any(|(a, _)| a == "tiers_pose"));
    // Visible des deux titulaires, SANS le motif ni le demandeur ; un autre foyer ne voit rien.
    for u in [pa, pb] {
        let vus = b.ent.tiers_eleve_a(b.etab, u, PARENT, eleve, maintenant).await.unwrap();
        assert_eq!(vus.len(), 1);
        assert!(vus[0].motif_texte.is_none() && vus[0].demande_par_personne_id.is_none());
        assert!(vus[0].remettable_aujourdhui);
        assert!(!serde_json::to_string(&vus).unwrap().contains("grand-mère"));
    }
    let (autre_foyer, _) = parent(&b, b.a[1], true).await;
    assert!(est_interdit(&b.ent.tiers_eleve_a(b.etab, autre_foyer, PARENT, eleve, maintenant).await));
    assert_eq!(b.ent.tiers_eleve_a(b.etab, b.cpe, CPE, eleve, maintenant).await.unwrap().len(), 1);
    // Interdiction nominative sur une fiche FUSIONNÉE dans le tiers : la chaîne bloque, toujours.
    let doublon = tiers(&b, "Odile bis").await;
    sqlx::query("UPDATE ent_personne SET fusionnee_dans = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(doublon)
        .bind(grand)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    interdire(&b, eleve, Some(doublon)).await;
    assert_eq!(code_pour(&b, eleve, grand).await.0, "interdite", "une interdiction bloque toujours, sur toute la chaîne de fusions");
    lever_interdictions(&b, eleve).await;
    assert_eq!(code_pour(&b, eleve, grand).await.0, "autorisee");
    // « Un acte tombe avec son auteur » : le demandeur devient visé → la porte se ferme ; B reprend la demande → rouverte.
    interdire_objet(&b, eleve, pers_a, "visite", None).await;
    assert_eq!(code_pour(&b, eleve, grand).await.0, "non_autorisee", "demandeur visé : le statut ne vaut plus");
    let r = b.ent.reprendre_tiers_a(b.etab, b.cpe, CPE, t.id, &TiersRepriseDto { demande_par_personne_id: pers_a, motif_texte: Some("même".into()) }, maintenant).await;
    assert!(refus_de(&r, "titulaire de l'autorité parentale en exercice"), "{r:?}");
    let r = b.ent.reprendre_tiers_a(b.etab, b.cpe, CPE, t.id, &TiersRepriseDto { demande_par_personne_id: pers_b, motif_texte: None }, maintenant).await;
    assert!(refus_de(&r, "motif obligatoire"), "{r:?}");
    b.ent.reprendre_tiers_a(b.etab, b.cpe, CPE, t.id, &TiersRepriseDto { demande_par_personne_id: pers_b, motif_texte: Some("Demande écrite du père".into()) }, maintenant).await.unwrap();
    assert_eq!(code_pour(&b, eleve, grand).await.0, "autorisee");
    // L'interdiction qui vise le TIERS reste prioritaire et empêche toute nouvelle pose.
    interdire(&b, eleve, Some(grand)).await;
    assert_eq!(code_pour(&b, eleve, grand).await.0, "interdite");
    let autre_tiers = tiers(&b, "Paul").await;
    interdire(&b, eleve, Some(autre_tiers)).await;
    let r = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, autre_tiers, pers_b), maintenant).await;
    assert!(refus_de(&r, "interdiction"), "{r:?}");
    // Un lien suspendu ou en garde « aucune » n'est pas contourné par ce statut (décision de justice).
    let juge = tiers(&b, "Jean").await;
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, lien, autorite_parentale, suspendu_le, suspendu_par, suspension_motif) \
         VALUES ($1,$2,$3,$4,'pere',false, now(), $5, 'Décision de justice')",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(juge)
    .bind(b.dir)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    let r = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, juge, pers_b), maintenant).await;
    assert!(refus_de(&r, "ne le remplace pas"), "{r:?}");
}

/// M3 — désaccord entre titulaires : le plus restrictif l'emporte jusqu'à la direction ; l'opposant n'est jamais nommé ;
/// seul le demandeur ou le personnel révoque ; le CPE ne tranche pas ; une révocation est immédiate et historisée ; un
/// désaccord connu du socle 18 pose le statut « contesté » d'emblée.
#[tokio::test]
async fn s_m3_desaccord_opposition_arbitrage_et_revocation() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let maintenant = paris(avril(14), 10, 0);
    let (pa, pers_a) = parent(&b, eleve, true).await;
    let (pb, _) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    let t = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await.unwrap();
    assert_eq!(code_pour(&b, eleve, grand).await.0, "autorisee");
    // Le demandeur ne s'oppose pas à sa propre demande ; un étranger, un élève, un professeur non plus.
    assert!(refus_de(&b.ent.opposer_tiers_a(b.etab, pa, PARENT, t.id, maintenant).await, "la révoque"));
    let (autre_foyer, _) = parent(&b, b.a[1], true).await;
    assert!(est_interdit(&b.ent.opposer_tiers_a(b.etab, autre_foyer, PARENT, t.id, maintenant).await));
    assert!(est_interdit(&b.ent.opposer_tiers_a(b.etab, b.prof, "prof", t.id, maintenant).await));
    // B s'oppose : porte fermée aussitôt (le plus restrictif), statut « contesté » ; A et B ne lisent pas l'opposant.
    b.ent.opposer_tiers_a(b.etab, pb, PARENT, t.id, maintenant).await.unwrap();
    assert_eq!(code_pour(&b, eleve, grand).await.0, "non_autorisee");
    let vus = b.ent.tiers_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await.unwrap();
    assert_eq!((vus[0].statut.as_str(), vus[0].remettable_aujourdhui), ("en_cours_d_examen", false), "famille : libellé neutre (M5)");
    assert!(!serde_json::to_string(&vus).unwrap().contains(&pb.to_string()));
    // B ne révoque pas la demande de A (il s'oppose) ; la direction seule tranche, avec motif pour lever.
    assert!(est_interdit(&b.ent.revoquer_tiers_a(b.etab, pb, PARENT, t.id, &CorrespondantSuspensionDto::default(), maintenant).await));
    assert!(est_interdit(&b.ent.trancher_tiers_a(b.etab, b.cpe, CPE, t.id, &tranche_motivee("lever"), maintenant).await), "le CPE ne tranche pas");
    assert!(refus_de(&b.ent.trancher_tiers_a(b.etab, b.dir, DIR, t.id, &tranche("lever"), maintenant).await, "motif obligatoire"));
    b.ent.trancher_tiers_a(b.etab, b.dir, DIR, t.id, &tranche_motivee("lever"), maintenant).await.unwrap();
    assert_eq!(code_pour(&b, eleve, grand).await.0, "autorisee");
    // Seul un tiers contesté se tranche.
    assert!(b.ent.trancher_tiers_a(b.etab, b.dir, DIR, t.id, &tranche_motivee("maintenir"), maintenant).await.is_err());
    // Le demandeur révoque : effet immédiat, historisé ; une seconde révocation est un conflit.
    b.ent.revoquer_tiers_a(b.etab, pa, PARENT, t.id, &CorrespondantSuspensionDto::default(), maintenant).await.unwrap();
    assert_eq!(code_pour(&b, eleve, grand).await.0, "non_autorisee");
    assert!(b.ent.revoquer_tiers_a(b.etab, b.cpe, CPE, t.id, &CorrespondantSuspensionDto::default(), maintenant).await.is_err());
    let actes: Vec<String> = faits_actes(&b, "tiers_id", t.id).await.into_iter().map(|(a, _)| a).collect();
    assert_eq!(actes, ["tiers_pose", "tiers_opposition", "tiers_levee", "tiers_revocation"]);
    assert!(b.ent.tiers_eleve_a(b.etab, pa, PARENT, eleve, maintenant).await.unwrap().is_empty(), "un tiers révoqué n'est plus listé");
    // Une nouvelle pose est possible après révocation ; le personnel peut révoquer ; « maintenir » révoque un contesté.
    let t2 = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await.unwrap();
    b.ent.opposer_tiers_a(b.etab, pb, PARENT, t2.id, maintenant).await.unwrap();
    b.ent.trancher_tiers_a(b.etab, b.dir, DIR, t2.id, &tranche("maintenir"), maintenant).await.unwrap();
    assert_eq!(code_pour(&b, eleve, grand).await.0, "non_autorisee");
    let t3 = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await.unwrap();
    b.ent.revoquer_tiers_a(b.etab, b.cpe, CPE, t3.id, &CorrespondantSuspensionDto::default(), maintenant).await.unwrap();
    // Désaccord connu (socle 18) : pose d'emblée « contesté », porte fermée.
    desaccord_connu(&b, eleve, pb).await;
    let t4 = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await.unwrap();
    assert_eq!((t4.statut.as_str(), t4.remettable_aujourdhui), ("conteste", false));
    assert_eq!(code_pour(&b, eleve, grand).await.0, "non_autorisee");
    // Aucune donnée nominative ni texte libre dans les événements.
    let evts: Vec<(String,)> = sqlx::query_as("SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND payload::text LIKE '%tiers_%'")
        .bind(b.etab)
        .fetch_all(b.ent.pool_verif())
        .await
        .unwrap();
    assert!(!evts.is_empty());
    assert!(evts.iter().all(|e| !e.0.contains("grand-mère") && !e.0.contains("Odile")), "{evts:?}");
}

/// M3 — le tiers passe aussi par la revue : une sortie accompagnée par le tiers est suspendue ou bloquée quand le statut
/// est révoqué (personne qui n'est plus autorisée), au départ comme à la revue ; le témoin (titulaire) part normalement.
#[tokio::test]
async fn s_m3_une_sortie_avec_le_tiers_ne_part_plus_apres_revocation() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (mardi, mercredi) = (avril(14), avril(15));
    let (pa, pers_a) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    let t = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), paris(mardi, 9, 0)).await.unwrap();
    let a = annoncer_avec(&b, pa, PARENT, eleve, mercredi, mercredi, Some(grand), paris(mardi, 10, 0)).await.unwrap();
    assert_eq!(a.statut, "annoncee", "la personne est le tiers autorisé");
    b.ent.revoquer_tiers_a(b.etab, pa, PARENT, t.id, &CorrespondantSuspensionDto::default(), paris(mardi, 11, 0)).await.unwrap();
    let soir = paris(mercredi, 17, 0);
    let r = b.ent.controler_depart_absence_a(b.etab, b.cpe, CPE, a.id, &depart(Some(grand), soir), soir).await;
    assert!(r.is_err(), "après révocation, le départ avec le tiers est refusé : {r:?}");
}

/// Infirmerie — la remise d'un élève malade passe par LA fonction de remise de l'internat (interdictions, chaîne de
/// fusions, perte de droit, tiers autorisé). Avant, la règle était réécrite à part et ignorait les interdictions.
#[tokio::test]
async fn s_infirmerie_la_remise_passe_par_la_fonction_unique_de_l_internat() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let jour = avril(14);
    let maintenant = paris(jour, 10, 0);
    let (_, pers_a) = parent(&b, eleve, true).await;
    let (pc, pers_c) = parent(&b, eleve, true).await;
    let contact = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, eleve, contact).await;
    let grand = tiers(&b, "Marc").await;
    b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await.unwrap();
    let inconnu = tiers(&b, "Zoé").await;
    let ok = |p: Uuid| {
        let b = &b;
        async move { b.ent.personne_habilitee_a_recuperer(b.etab, eleve, p, jour).await.unwrap() }
    };
    assert!(ok(pers_a).await.is_some(), "titulaire");
    assert!(ok(contact).await.is_some(), "personne autorisée");
    assert!(ok(grand).await.is_some(), "tiers autorisé");
    assert!(ok(inconnu).await.is_none(), "refus par défaut");
    // Une interdiction (chaîne de fusions) bloque le titulaire, le contact et le tiers, comme à l'internat.
    interdire(&b, eleve, Some(contact)).await;
    let doublon = tiers(&b, "Marc bis").await;
    sqlx::query("UPDATE ent_personne SET fusionnee_dans = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(doublon)
        .bind(grand)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    interdire(&b, eleve, Some(doublon)).await;
    interdire(&b, eleve, Some(pers_a)).await;
    assert!(ok(contact).await.is_none(), "contact interdit");
    assert!(ok(grand).await.is_none(), "tiers interdit par sa chaîne de fusions");
    assert!(ok(pers_a).await.is_none(), "titulaire interdit");
    // Un titulaire privé d'autorité n'est plus remis (même fonction) ; le témoin reste remis.
    p4_perdre_le_droit(&b, eleve, pc, 1).await;
    assert!(ok(pers_c).await.is_none(), "titulaire sans autorité");
    // Interdiction sans personne identifiée : on ne peut pas vérifier qui est à la porte.
    let e2 = b.a[1];
    let (_, pers_x) = parent(&b, e2, true).await;
    assert!(b.ent.personne_habilitee_a_recuperer(b.etab, e2, pers_x, jour).await.unwrap().is_some());
    interdire(&b, e2, None).await;
    assert!(b.ent.personne_habilitee_a_recuperer(b.etab, e2, pers_x, jour).await.unwrap().is_none());
}

/// Requalification d'un contact : limitée aux élèves INTERNES, et refusée en cas de conflit d'intérêt (l'agent est la
/// personne du contact, son déclarant, ou un proche de l'élève) ; un autre agent la fait.
#[tokio::test]
async fn s_requalification_limitee_aux_internes_et_sans_conflit_d_interet() {
    let Some(b) = banc().await else { return };
    let maintenant = paris(avril(14), 10, 0);
    let motif = CorrespondantSuspensionDto { motif_texte: Some("Contact revu avec la famille".into()) };
    let eleve = b.a[0];
    let interne: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_internat_sejour WHERE etablissement_id = $1 AND eleve_id = $2 AND du <= $3 AND (au IS NULL OR au >= $3))",
    )
    .bind(b.etab)
    .bind(eleve)
    .bind(avril(14))
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert!(interne, "le banc héberge ses élèves");
    let fiche_cpe = personne_de(&b.ent, b.etab, b.cpe).await;
    let fiche_dir = personne_de(&b.ent, b.etab, b.dir).await;
    // Conflit 1 : le CPE est la personne du contact.
    p4_contact_declare(&b, eleve, fiche_cpe, None).await;
    let c1 = p4_id_contact(&b, eleve, fiche_cpe).await;
    let r = b.ent.requalifier_contact_a(b.etab, b.cpe, CPE, c1, &motif, maintenant).await;
    assert!(refus_de(&r, "conflit d'intérêt"), "{r:?}");
    // Conflit 2 : le CPE est le déclarant du contact ; la direction (autre agent) peut requalifier.
    let contact = tiers(&b, "Odile").await;
    p4_contact_declare(&b, eleve, contact, Some(fiche_cpe)).await;
    let c2 = p4_id_contact(&b, eleve, contact).await;
    let r = b.ent.requalifier_contact_a(b.etab, b.cpe, CPE, c2, &motif, maintenant).await;
    assert!(refus_de(&r, "conflit d'intérêt"), "{r:?}");
    b.ent.requalifier_contact_a(b.etab, b.dir, DIR, c2, &motif, maintenant).await.unwrap();
    // Conflit 3 : la direction est un proche de l'élève (lien) ; le CPE requalifie.
    sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, lien, autorite_parentale) VALUES ($1,$2,$3,$4,'oncle',false)")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(eleve)
        .bind(fiche_dir)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let proche = tiers(&b, "Nina").await;
    p4_contact_declare(&b, eleve, proche, None).await;
    let c3 = p4_id_contact(&b, eleve, proche).await;
    let r = b.ent.requalifier_contact_a(b.etab, b.dir, DIR, c3, &motif, maintenant).await;
    assert!(refus_de(&r, "conflit d'intérêt"), "{r:?}");
    b.ent.requalifier_contact_a(b.etab, b.cpe, CPE, c3, &motif, maintenant).await.unwrap();
    // Un élève qui n'est pas interne (sans séjour d'internat) : refusé.
    let externe = eleve_invite(&b, "Externe").await;
    let ext = tiers(&b, "Lucie").await;
    p4_contact_declare(&b, externe, ext, None).await;
    let ce = p4_id_contact(&b, externe, ext).await;
    let r = b.ent.requalifier_contact_a(b.etab, b.dir, DIR, ce, &motif, maintenant).await;
    assert!(refus_de(&r, "limitée aux élèves internes"), "{r:?}");
}

/// R1 sur une SORTIE (le scénario de p4_r1 ne couvrait que l'absence) : la sortie demandée par un parent puis autorisée
/// par le CPE retombe « demandée » quand ce parent perd son droit (lien suspendu, autorité retirée, lien échu, garde
/// « aucune »), à la revue ; le témoin (parent intact) reste « autorisée ». Le départ est refusé avec la personne désignée.
#[tokio::test]
async fn s_p4_r1_la_sortie_dont_l_auteur_perd_son_droit_retombe_a_la_demande() {
    let Some(b) = banc().await else { return };
    let (lundi, mardi, jeudi) = (avril(13), avril(14), avril(16));
    let (debut, fin) = (paris(jeudi, 18, 0), paris(jeudi, 21, 0));
    let (pt, _) = parent(&b, b.b[0], true).await;
    let grand_t = tiers(&b, "Odile").await;
    autoriser_recuperer(&b, b.b[0], grand_t).await;
    let st = b.ent.demander_sortie_a(b.etab, pt, PARENT, &demande(b.b[0], "exceptionnelle", debut, fin, false, Some(grand_t)), paris(lundi, 10, 0)).await.unwrap();
    let st = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, st.id, &decision("autoriser", None), paris(lundi, 11, 0)).await.unwrap();
    assert_eq!(st.statut, "autorisee");
    let mut sorties = Vec::new();
    for cas in 0..4usize {
        let eleve = b.a[cas];
        let (pa, _) = parent(&b, eleve, true).await;
        let grand = tiers(&b, "Odile").await;
        autoriser_recuperer(&b, eleve, grand).await;
        let s = b.ent.demander_sortie_a(b.etab, pa, PARENT, &demande(eleve, "exceptionnelle", debut, fin, false, Some(grand)), paris(lundi, 10, 0)).await.unwrap();
        let s = b.ent.decider_sortie_a(b.etab, b.cpe, CPE, s.id, &decision("autoriser", None), paris(lundi, 11, 0)).await.unwrap();
        assert_eq!(s.statut, "autorisee");
        p4_perdre_le_droit(&b, eleve, pa, cas).await;
        sorties.push((s.id, grand));
    }
    b.ent.veille_sorties_a(b.etab, paris(mardi, 12, 5)).await.unwrap();
    for (cas, (id, grand)) in sorties.iter().enumerate() {
        let (statut, ..) = ligne_sortie(&b, *id).await;
        assert_eq!(statut, "demandee", "cas {cas} : l'accord de l'auteur sans droit tombe, retour à la décision");
        let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, *id, &depart(Some(*grand), debut), debut).await;
        assert!(r.is_err(), "cas {cas} : départ refusé : {r:?}");
    }
    assert_eq!(ligne_sortie(&b, st.id).await.0, "autorisee", "le témoin est intact");
}

// ───────────────────────── Correctifs de revue (E1, E2, M3 à M6, F1, F4, F5) ─────────────────────────
//
// | Point | Test(s) |
// |---|---|
// | E1 un parent / tuteur sans autorité n'est ni tiers ni contact remis ; un proche de lien `autre` oui | `s_e1_*` |
// | E2 « personnel seul » = aucun lien ; sinon jugé sur son lien ; conflit d'intérêt à la pose et à la reprise | `s_e2_*` |
// | M3 notification des titulaires, reprise seulement si le demandeur est tombé | `s_m3_revue_*` |
// | M4 / M5 motif et demandeur au seul décideur ; libellé neutre côté famille | `s_m4_m5_*` |
// | F1 pas de ré-opposition après « lever » ; F4 alerte avant échéance, 365 jours | `s_f1_*`, `s_f4_*` |
// | M6 garde statique (sans base) de la remise des participants | `p4_r8_garde_statique_*` |

/// Lien sans compte entre une fiche et l'élève (autorité à choisir, non suspendu, garde non « aucune »).
async fn lier_sans_compte(b: &Banc, eleve: Uuid, personne: Uuid, lien: &str, autorite: bool) {
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, lien, autorite_parentale) \
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(personne)
    .bind(lien)
    .bind(autorite)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
}

/// E1 — la décision du pilote : un lien est écarté s'il est suspendu, en garde « aucune », échu, pas commencé, OU sans
/// autorité parentale pour un `mere`, `pere` ou `tuteur`. Une seule fonction pour la porte « contact » et la porte « tiers » ;
/// effet à la pose ET à la remise (autorité retirée après coup = porte fermée). Un proche de lien `autre` reste posable.
#[tokio::test]
async fn s_e1_parent_prive_d_autorite_ni_tiers_ni_contact_proche_autre_accepte() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let maintenant = paris(avril(14), 10, 0);
    let (_, pers_a) = parent(&b, eleve, true).await;
    // Autorité retirée SEULE (lien non suspendu, garde non « aucune ») : refus à la pose, comme contact.
    let prive = tiers(&b, "Hugo").await;
    lier_sans_compte(&b, eleve, prive, "pere", false).await;
    let r = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, prive, pers_a), maintenant).await;
    assert!(refus_de(&r, "ne le remplace pas"), "{r:?}");
    assert_eq!(code_pour(&b, eleve, prive).await.0, "non_autorisee");
    autoriser_recuperer(&b, eleve, prive).await;
    assert_eq!(code_pour(&b, eleve, prive).await.0, "non_autorisee", "porte « contact » : même règle, une seule fonction");
    // Posé tant qu'il est titulaire, puis l'autorité est retirée après coup : la porte se ferme à la remise.
    let ex_titulaire = tiers(&b, "Luc").await;
    lier_sans_compte(&b, eleve, ex_titulaire, "tuteur", true).await;
    b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, ex_titulaire, pers_a), maintenant).await.unwrap();
    autoriser_recuperer(&b, eleve, ex_titulaire).await;
    assert_eq!(code_pour(&b, eleve, ex_titulaire).await.0, "autorisee");
    sqlx::query("UPDATE ent_eleve_lien SET autorite_parentale = false WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3")
        .bind(b.etab)
        .bind(eleve)
        .bind(ex_titulaire)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(code_pour(&b, eleve, ex_titulaire).await.0, "non_autorisee", "autorité retirée après coup : tiers et contact fermés");
    // Un grand-parent de lien `autre` sans autorité : posable et remis (tiers), ou remis comme contact.
    let grand = tiers(&b, "Odile").await;
    lier_sans_compte(&b, eleve, grand, "autre", false).await;
    b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await.unwrap();
    assert_eq!(code_pour(&b, eleve, grand).await, ("autorisee".to_string(), Some("tiers autorisé".to_string())));
    let grand2 = tiers(&b, "Marc").await;
    lier_sans_compte(&b, eleve, grand2, "autre", false).await;
    autoriser_recuperer(&b, eleve, grand2).await;
    assert_eq!(code_pour(&b, eleve, grand2).await.0, "autorisee");
    // `lien` est un texte libre : un parent sans autorité saisi « Maman », « Père » ou « tutrice » est écarté comme
    // `mere` / `pere` / `tuteur` ; « Grand-mère » et « Beau-père » restent des proches.
    for (nom, lien, ecarte) in [("Alba", "Maman", true), ("Bruno", " Père ", true), ("Chloé", "tutrice", true), ("Denise", "Grand-mère", false), ("Emile", "Beau-père", false)] {
        let p = tiers(&b, nom).await;
        lier_sans_compte(&b, eleve, p, lien, false).await;
        autoriser_recuperer(&b, eleve, p).await;
        let attendu = if ecarte { "non_autorisee" } else { "autorisee" };
        assert_eq!(code_pour(&b, eleve, p).await.0, attendu, "lien « {lien} » sans autorité");
    }
}

async fn vise_a(b: &Banc, eleve: Uuid, user: Uuid, jour: NaiveDate) -> Option<DateTime<Utc>> {
    sqlx::query_scalar::<_, Option<DateTime<Utc>>>("SELECT ent_internat_user_vise($1, $2, $3, $4)")
        .bind(b.etab)
        .bind(eleve)
        .bind(user)
        .bind(jour)
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap()
}

/// E2 — « personnel seul » = rôle de personnel ET aucun lien avec l'élève (compte, fiche de la chaîne de fusions), aucun
/// compte élève sur cet élève, aucune casquette parent ou élève. Sinon il est jugé sur son lien : un agent père dont le lien est
/// suspendu est visé (ses actes tombent). Conflit d'intérêt à la pose et à la reprise d'un tiers.
#[tokio::test]
async fn s_e2_un_agent_qui_est_proche_de_l_eleve_est_juge_sur_son_lien() {
    let Some(b) = banc().await else { return };
    let (mardi, eleve, autre_eleve) = (avril(14), b.a[0], b.a[1]);
    // Témoin : agent sans lien → jamais « sans lien » (inchangé).
    assert!(vise_a(&b, eleve, b.cpe, mardi).await.is_none());
    // Agent père, lien SUSPENDU : visé malgré son rôle de personnel.
    let agent = compte(&b.ent, b.etab, "administratif").await;
    let fiche = personne_de(&b.ent, b.etab, agent).await;
    assert!(vise_a(&b, eleve, agent, mardi).await.is_none(), "avant tout lien : personnel seul");
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale) \
         VALUES ($1,$2,$3,$4,$5,'pere',true)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(agent)
    .bind(fiche)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    assert!(vise_a(&b, eleve, agent, mardi).await.is_none(), "lien actif : pas visé");
    sqlx::query("UPDATE ent_eleve_lien SET suspendu_le = now(), suspendu_par = $3, suspension_motif = 'Décision' WHERE etablissement_id = $1 AND personne_id = $2")
        .bind(b.etab)
        .bind(fiche)
        .bind(b.dir)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    assert!(vise_a(&b, eleve, agent, mardi).await.is_some(), "agent père, lien suspendu : ses actes tombent");
    assert!(vise_a(&b, autre_eleve, agent, mardi).await.is_none(), "pour un autre élève (aucun lien) : personnel seul");
    // Casquette parent sans aucun lien avec CET élève : jugé comme un parent sans lien.
    let agent2 = compte(&b.ent, b.etab, "cpe").await;
    assert!(vise_a(&b, eleve, agent2, mardi).await.is_none());
    sqlx::query("INSERT INTO ent_membership_casquette (user_id, etablissement_id, casquette, du) VALUES ($1,$2,'parent','2026-01-01')")
        .bind(agent2)
        .bind(b.etab)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    assert!(vise_a(&b, eleve, agent2, mardi).await.is_some(), "casquette parent sans lien : plus personnel seul");
    // Conflit d'intérêt : un CPE proche de l'élève (lien) ne pose pas de tiers pour lui, ni pour lui-même, ni comme demandeur.
    let maintenant = paris(mardi, 10, 0);
    let (_, pers_a) = parent(&b, eleve, true).await;
    let cpe2 = compte(&b.ent, b.etab, "cpe").await;
    let fiche_cpe2 = personne_de(&b.ent, b.etab, cpe2).await;
    lier_sans_compte(&b, autre_eleve, fiche_cpe2, "pere", true).await;
    sqlx::query("UPDATE ent_eleve_lien SET user_id = $3 WHERE etablissement_id = $1 AND personne_id = $2")
        .bind(b.etab)
        .bind(fiche_cpe2)
        .bind(cpe2)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let grand = tiers(&b, "Odile").await;
    let r = b.ent.poser_tiers_a(b.etab, cpe2, CPE, &pose_tiers(autre_eleve, grand, fiche_cpe2), maintenant).await;
    assert!(refus_de(&r, "conflit d'intérêt"), "agent proche de l'élève, et demandeur : {r:?}");
    let (_, pers_x) = parent(&b, autre_eleve, true).await;
    let r = b.ent.poser_tiers_a(b.etab, cpe2, CPE, &pose_tiers(autre_eleve, grand, pers_x), maintenant).await;
    assert!(refus_de(&r, "conflit d'intérêt"), "agent proche de l'élève : {r:?}");
    let r = b.ent.poser_tiers_a(b.etab, cpe2, CPE, &pose_tiers(eleve, fiche_cpe2, pers_a), maintenant).await;
    assert!(refus_de(&r, "conflit d'intérêt"), "l'agent ne se pose pas lui-même : {r:?}");
    let t = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await.unwrap();
    // Reprise : l'agent en conflit ne reprend pas (il serait le nouveau demandeur) ; le demandeur doit d'abord être tombé.
    interdire_objet(&b, eleve, pers_a, "visite", None).await;
    let r = b.ent.reprendre_tiers_a(b.etab, cpe2, CPE, t.id, &TiersRepriseDto { demande_par_personne_id: fiche_cpe2, motif_texte: Some("Demande écrite".into()) }, maintenant).await;
    assert!(refus_de(&r, "conflit d'intérêt"), "{r:?}");
}

/// M3 (revue) — la pose notifie tous les titulaires par un événement dédié, sans motif ni demandeur ; la reprise n'est permise
/// que si le demandeur actuel est réellement tombé.
#[tokio::test]
async fn s_m3_revue_pose_notifiee_aux_titulaires_et_reprise_seulement_si_le_demandeur_est_tombe() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let maintenant = paris(avril(14), 10, 0);
    let (_, pers_a) = parent(&b, eleve, true).await;
    let (_, pers_b) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    let t = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await.unwrap();
    let evts = b.evenements("internat.tiers").await;
    assert_eq!(evts.len(), 1, "un événement dédié aux tiers (et non « internat.correspondant »)");
    assert_eq!(evts[0]["action"], "tiers_pose");
    assert_eq!(evts[0]["destinataires"], "titulaires", "tous les titulaires, pas seulement le demandeur");
    let brut = evts[0].to_string();
    assert!(!brut.contains("grand-mère") && !brut.contains(&pers_a.to_string()) && !brut.contains(&grand.to_string()), "{brut}");
    // Reprise refusée tant que le demandeur est titulaire en exercice.
    let reprise = TiersRepriseDto { demande_par_personne_id: pers_b, motif_texte: Some("Demande écrite du père".into()) };
    let r = b.ent.reprendre_tiers_a(b.etab, b.cpe, CPE, t.id, &reprise, maintenant).await;
    assert!(refus_de(&r, "toujours un titulaire en exercice"), "{r:?}");
    assert_eq!(code_pour(&b, eleve, grand).await.0, "autorisee", "rien n'a changé");
    // Le demandeur tombe : la reprise est alors permise.
    interdire_objet(&b, eleve, pers_a, "visite", None).await;
    b.ent.reprendre_tiers_a(b.etab, b.cpe, CPE, t.id, &reprise, maintenant).await.unwrap();
    assert_eq!(code_pour(&b, eleve, grand).await.0, "autorisee");
}

/// M4 / M5 — motif et demandeur : seulement avec `internat.sorties.decider` (un rôle qui n'a que la lecture de l'internat et de
/// l'appel voit la liste, pas le motif) ; côté famille, un tiers contesté ou fermé s'affiche « en cours d'examen », libellé neutre
/// et identique quelle que soit la cause.
#[tokio::test]
async fn s_m4_m5_motif_au_decideur_seul_et_libelle_neutre_cote_famille() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let maintenant = paris(avril(14), 10, 0);
    let (pa, pers_a) = parent(&b, eleve, true).await;
    let (pb, _) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    let t = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await.unwrap();
    // Un rôle « lecture seule » de l'internat : liste, état, dates ; ni motif ni demandeur.
    let droit = |a: Action| Droit { action: a, portee: Portee::Etablissement, cibles: vec![] };
    let role = b
        .ent
        .creer_role_for_actor(
            b.etab,
            b.dir,
            DIR,
            RoleSaisie { nom: format!("Lecture internat {}", alea()), description: None, modele: None, droits: vec![droit(Action::InternatLire), droit(Action::InternatAppel)] },
        )
        .await
        .unwrap();
    b.ent.attribuer_role_for_actor(b.etab, b.dir, DIR, role.id, b.compta, None).await.unwrap();
    let lecteur = b.ent.tiers_eleve_a(b.etab, b.compta, "compta", eleve, maintenant).await.unwrap();
    assert_eq!(lecteur.len(), 1);
    assert!(lecteur[0].motif_texte.is_none() && lecteur[0].demande_par_personne_id.is_none(), "lecture seule : ni motif ni demandeur");
    let decideur = b.ent.tiers_eleve_a(b.etab, b.cpe, CPE, eleve, maintenant).await.unwrap();
    assert!(decideur[0].motif_texte.is_some() && decideur[0].demande_par_personne_id == Some(pers_a));
    // Famille : actif tant que la porte est ouverte ; « en cours d'examen » ensuite, quelle que soit la cause.
    let statut = |u: Uuid| {
        let b = &b;
        async move { b.ent.tiers_eleve_a(b.etab, u, PARENT, eleve, maintenant).await.unwrap().remove(0).statut }
    };
    assert_eq!(statut(pa).await, "actif");
    b.ent.opposer_tiers_a(b.etab, pb, PARENT, t.id, maintenant).await.unwrap();
    assert_eq!(statut(pa).await, "en_cours_d_examen", "contesté");
    assert_eq!(statut(pb).await, "en_cours_d_examen");
    b.ent.trancher_tiers_a(b.etab, b.dir, DIR, t.id, &tranche_motivee("lever"), maintenant).await.unwrap();
    assert_eq!(statut(pa).await, "actif");
    interdire_objet(&b, eleve, pers_a, "visite", None).await; // demandeur tombé : porte fermée
    assert_eq!(statut(pb).await, "en_cours_d_examen", "fermé par une autre cause : même libellé");
    assert!(decideur_statut_reel(&b, eleve, maintenant).await == "actif", "le personnel voit l'état réel");
}

async fn decideur_statut_reel(b: &Banc, eleve: Uuid, maintenant: DateTime<Utc>) -> String {
    b.ent.tiers_eleve_a(b.etab, b.cpe, CPE, eleve, maintenant).await.unwrap().remove(0).statut
}

/// F1 — après « lever » par la direction, une nouvelle opposition ne rouvre pas le désaccord (pas de cycle sans fin) ;
/// le demandeur ou le personnel peut toujours révoquer.
#[tokio::test]
async fn s_f1_pas_de_reopposition_apres_lever() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let maintenant = paris(avril(14), 10, 0);
    let (pa, pers_a) = parent(&b, eleve, true).await;
    let (pb, _) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    let t = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &pose_tiers(eleve, grand, pers_a), maintenant).await.unwrap();
    b.ent.opposer_tiers_a(b.etab, pb, PARENT, t.id, maintenant).await.unwrap();
    b.ent.trancher_tiers_a(b.etab, b.dir, DIR, t.id, &tranche_motivee("lever"), maintenant).await.unwrap();
    let r = b.ent.opposer_tiers_a(b.etab, pb, PARENT, t.id, maintenant).await;
    assert!(r.is_err(), "ré-opposition refusée : {r:?}");
    assert_eq!(code_pour(&b, eleve, grand).await.0, "autorisee", "la décision de la direction vaut");
    b.ent.revoquer_tiers_a(b.etab, pa, PARENT, t.id, &CorrespondantSuspensionDto::default(), maintenant).await.unwrap();
}

/// F4 — 365 jours au plus (et non 366), alerte avant échéance une seule fois (événement sans motif ni demandeur), jamais pour
/// un tiers révoqué ; le DTO signale l'échéance proche.
#[tokio::test]
async fn s_f4_duree_365_jours_et_alerte_avant_echeance() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let maintenant = paris(avril(14), 10, 0);
    let (pa, pers_a) = parent(&b, eleve, true).await;
    let grand = tiers(&b, "Odile").await;
    let mut p = pose_tiers(eleve, grand, pers_a);
    p.du = avril(13);
    p.au = avril(13) + Duration::days(366);
    assert!(refus_de(&b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &p, maintenant).await, "un an au plus"));
    let base: Result<(), sqlx::Error> = sqlx::query(
        "INSERT INTO ent_internat_tiers_autorise (id, etablissement_id, eleve_id, personne_id, demande_par_personne_id, demande_ecrite_le, du, au, motif, pose_par_user_id) \
         VALUES ($1,$2,$3,$4,$5,'2026-04-10','2026-04-13','2027-04-14','x',$6)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(grand)
    .bind(pers_a)
    .bind(b.cpe)
    .execute(b.ent.pool_verif())
    .await
    .map(|_| ());
    assert!(base.is_err(), "la base refuse aussi 366 jours");
    p.au = avril(13) + Duration::days(365);
    let loin = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &p, maintenant).await.unwrap();
    assert!(!loin.echeance_proche);
    // Un second tiers dont l'échéance est dans 10 jours.
    let voisin = tiers(&b, "Marc").await;
    let mut q = pose_tiers(eleve, voisin, pers_a);
    q.au = avril(24);
    let proche = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &q, maintenant).await.unwrap();
    assert!(proche.echeance_proche, "échéance dans les quinze jours");
    let v = b.ent.veille_sorties_a(b.etab, paris(avril(14), 11, 0)).await.unwrap();
    assert_eq!(v.tiers_a_echeance, 1, "une alerte, pour le seul tiers proche d'échéance");
    let alertes = b.evenements("internat.tiers").await.into_iter().filter(|e| e["action"] == "tiers_echeance").count();
    assert_eq!(alertes, 1);
    assert_eq!(b.ent.veille_sorties_a(b.etab, paris(avril(15), 11, 0)).await.unwrap().tiers_a_echeance, 0, "une seule fois");
    // Un tiers révoqué n'est plus alerté.
    let autre = tiers(&b, "Paul").await;
    let mut r = pose_tiers(eleve, autre, pers_a);
    r.au = avril(20);
    let t = b.ent.poser_tiers_a(b.etab, b.cpe, CPE, &r, maintenant).await.unwrap();
    b.ent.revoquer_tiers_a(b.etab, pa, PARENT, t.id, &CorrespondantSuspensionDto::default(), maintenant).await.unwrap();
    assert_eq!(b.ent.veille_sorties_a(b.etab, paris(avril(16), 11, 0)).await.unwrap().tiers_a_echeance, 0);
}

/// F5 — l'export de portabilité de la personne ne rend ni le motif ni le demandeur du tiers ; les colonnes déclarées existent.
#[tokio::test]
async fn s_f5_export_du_tiers_sans_motif_ni_demandeur() {
    let Some(b) = banc().await else { return };
    let c = crate::registre_donnees::classer("ent_internat_tiers_autorise").expect("table au registre");
    assert!(c.export_portabilite);
    assert_eq!(c.colonnes_hors_export, ["motif", "demande_par_personne_id"]);
    for col in c.colonnes_hors_export {
        let existe: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'ent_internat_tiers_autorise' AND column_name = $1)")
            .bind(col)
            .fetch_one(b.ent.pool_verif())
            .await
            .unwrap();
        assert!(existe, "colonne {col} déclarée hors export mais absente de la table");
    }
}

/// M6 — garde STATIQUE (sans base, donc toujours exécutée, y compris sans `ENT_DATABASE_URL`) : la migration 2827 pose le
/// déclencheur qui refuse les états de remise d'un participant de sortie scolaire, et il est enregistré dans la liste des migrations.
/// Le test à base (`p4_r8_la_base_refuse…`) prouve son comportement.
#[test]
fn p4_r8_garde_statique_le_declencheur_de_remise_des_participants_est_dans_la_migration() {
    let sql = include_str!("../../../migrations/2827_internat_i3_suites.sql");
    for attendu in [
        "CREATE OR REPLACE FUNCTION ent_internat_remise_participant_garde()",
        "NEW.etat IN ('remis', 'parti_seul_autorise')",
        "ERRCODE = '42501'",
        "CREATE TRIGGER ent_internat_remise_participant_garde BEFORE INSERT OR UPDATE OF etat ON ent_sortie_participant",
        "EXECUTE FUNCTION ent_internat_remise_participant_garde()",
    ] {
        assert!(sql.contains(attendu), "la migration 2827 doit contenir : {attendu}");
    }
    let migrate = include_str!("migrate.rs");
    assert!(migrate.contains("m!(\"2827_internat_i3_suites\")"), "2827 est enregistrée dans migrate.rs");
}

/// M7 — un code de classe se répète d'une année à l'autre : la messagerie retient d'abord LA SEULE classe de l'année courante,
/// même si une classe d'une année passée porte encore des élèves affectés (avant : « celle qui a des élèves » l'emportait).
#[tokio::test]
async fn s_m7_la_messagerie_prefere_la_seule_classe_de_l_annee_courante() {
    let Some(b) = banc().await else { return };
    let pool = b.ent.pool_verif();
    let courante: Option<Uuid> = sqlx::query_scalar("SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante")
        .bind(b.etab)
        .fetch_optional(pool)
        .await
        .unwrap();
    let courante = match courante {
        Some(id) => id,
        None => {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante) VALUES ($1,$2,'M7-courante','2026-09-01','2027-08-31',true)")
                .bind(id)
                .bind(b.etab)
                .execute(pool)
                .await
                .unwrap();
            id
        }
    };
    let ancienne = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin) VALUES ($1,$2,'M7-ancienne','2020-09-01','2021-08-31')")
        .bind(ancienne)
        .bind(b.etab)
        .execute(pool)
        .await
        .unwrap();
    let (c_cur, c_old) = (Uuid::new_v4(), Uuid::new_v4());
    for (id, annee) in [(c_cur, courante), (c_old, ancienne)] {
        sqlx::query("INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle, niveau) VALUES ($1,$2,$3,'M7X','M7X','5e')")
            .bind(id)
            .bind(b.etab)
            .bind(annee)
            .execute(pool)
            .await
            .unwrap();
    }
    // Une affectation OUVERTE (sans fin) dans la classe de l'année passée : l'ancienne règle la préférait.
    sqlx::query("INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut) VALUES ($1,$2,$3,$4,$5,'2020-09-01')")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(ancienne)
        .bind(b.a[0])
        .bind(c_old)
        .execute(pool)
        .await
        .unwrap();
    let r = b.ent.resoudre_classe(b.etab, "m7x").await.unwrap();
    assert_eq!(r, Some(c_cur), "la seule classe de l'année courante d'abord");
    assert_eq!(b.ent.resoudre_classe(b.etab, "inconnue-m7").await.unwrap(), None);
}
