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

use super::tests_internat_i2::{alea, banc, est_interdit, nuit_dst, refus_de, soir, soir_zone_valide, utc, Banc, CPE, DIR};
use super::*;
use crate::tests_voyages_v0a::{compte, personne_de};  // personne_de : fiche d'un compte du personnel
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use ent_types::internat_nuit::*;
use ent_types::internat_sorties::*;
use uuid::Uuid;

/// Instant en heure d'été de Paris (UTC + 2) : avril 2026 est après le passage du 29 mars.
fn paris(jour: NaiveDate, h: u32, mi: u32) -> DateTime<Utc> {
    utc(jour.year(), jour.month(), jour.day(), h - 2, mi)
}

/// Un jour d'avril 2026 : le 14 est un mardi, le 17 un vendredi, le 19 un dimanche, le 20 un lundi.
fn avril(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 4, d).unwrap()
}

const PARENT: &str = "parent";
const ELEVE: &str = "eleve";

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
async fn parent(b: &Banc, eleve: Uuid, sorties: bool) -> (Uuid, Uuid) {
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
async fn tiers(b: &Banc, prenom: &str) -> Uuid {
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
async fn interdire(b: &Banc, eleve: Uuid, personne: Option<Uuid>) {
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
async fn desaccord_connu(b: &Banc, eleve: Uuid, declarant: Uuid) {
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
async fn compte_eleve(b: &Banc, eleve: Uuid, naissance: NaiveDate) -> Uuid {
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

async fn annoncer(b: &Banc, user: Uuid, role: &str, eleve: Uuid, du: NaiveDate, au: NaiveDate, a: DateTime<Utc>) -> Result<AbsenceDto, EntDbError> {
    b.ent
        .annoncer_absence_a(
            b.etab,
            user,
            role,
            &AbsenceAnnonceDto { eleve_id: eleve, du_nuit: du, au_nuit: au, recupere_par_personne_id: None, retour_prevu: None, regularisation: false, motif_texte: None },
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
    assert!(est_retire(&b, mardi, eleve, "absence_annoncee").await, "attendu recalculé avec le motif codé");
    assert!(est_attendu(&b, mardi, b.a[1]).await, "les autres élèves ne bougent pas");
    assert!(est_attendu(&b, avril(15), eleve).await, "la nuit suivante non plus");
    assert!(est_attendu(&b, avril(13), eleve).await, "ni la précédente");
    let evts = b.evenements("internat.absence_annoncee").await;
    assert_eq!(evts.len(), 1);
    assert_eq!((evts[0]["statut"].as_str(), evts[0]["tardive"].as_bool()), (Some("annoncee"), Some(false)));
    assert!(!b.evenements("internat.attendu_modifie").await.is_empty());
    // Trace : qui, quand, quoi.
    let f = faits(&b, "absence_id", a.id).await;
    assert_eq!(f, vec![("annonce".to_string(), "titulaire".to_string(), Some(p))]);
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
    let (p2, _) = parent(&b, b.a[3], false).await;
    annoncer(&b, p2, PARENT, b.a[3], avril(20), avril(22), paris(mardi, 9, 0)).await.unwrap();
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
    assert!(est_retire(&b, mardi, b.a[0], "absence_annoncee").await);
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
    assert!(est_retire(&b, avril(16), eleve, "absence_annoncee").await);
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
    assert!(est_retire(&b, mardi, b.a[0], "absence_annoncee").await);
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
    assert!(est_retire(&b, avril(15), eleve, "absence_annoncee").await);
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
    assert!(est_retire(&b, avril(16), eleve, "absence_annoncee").await);
    b.ent.opposer_absence_a(b.etab, p2, PARENT, a2.id, &opp(None), paris(mardi, 15, 0)).await.unwrap();
    assert!(est_attendu(&b, avril(16), eleve).await);
    let l = b.ent.trancher_absence_a(b.etab, b.dir, DIR, a2.id, &TrancheDto { decision: "lever".into(), motif_texte: None }, paris(mardi, 15, 5)).await.unwrap();
    assert_eq!(l.statut, "acceptee", "l'état d'avant le blocage est rétabli");
    assert!(est_retire(&b, avril(16), eleve, "absence_annoncee").await);
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
    // Règlement par défaut « si à charge » : les parents sont informés (événement sans nom, politique rappelée).
    let evts = b.evenements("internat.majeur_informe").await;
    assert_eq!(evts.len(), 1);
    assert_eq!((evts[0]["nature"].as_str(), evts[0]["politique"].as_str()), (Some("absence"), Some("si_a_charge")));
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
async fn accorder_annuellement(b: &Banc, parent: Uuid, eleve: Uuid, usages: &[&str]) {
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
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(None, debut - Duration::hours(5)), debut).await;
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
    assert!(est_retire(&b, vendredi, eleve, "sortie").await && est_retire(&b, avril(18), eleve, "sortie").await);
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
    let r = b.ent.controler_depart_a(b.etab, b.cpe, CPE, s.id, &depart(Some(grand), debut), debut + Duration::minutes(11)).await;
    assert!(refus_de(&r, "déjà parti"), "{r:?}");
    // Un retour avant le départ est absurde ; le retour réel clôt la sortie.
    let r = b.ent.controler_retour_a(b.etab, b.cpe, CPE, s.id, &ControleRetourDto { a: Some(debut) }, fin).await;
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
    assert!(est_retire(&b, avril(17), eleve, "sortie").await);
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
    let (p3, _) = parent(&b, e3, false).await;
    annoncer(&b, p3, PARENT, e3, vendredi, vendredi, maintenant).await.unwrap();
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
    assert_eq!(liste.len(), 2);
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
    assert!(est_retire(&b, mardi, eleve, "sortie").await);
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
    assert!(est_retire(&b, mardi, eleve, "sortie").await, "reprise du correspondant, reprise de la sortie");
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
    assert!(est_retire(&b, avril(17), eleve, "sortie").await);
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
    assert!(matches!(r, Err(EntDbError::Domain(_)) | Err(EntDbError::Forbidden)) || r.as_ref().is_ok_and(Vec::is_empty), "{r:?}");
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
