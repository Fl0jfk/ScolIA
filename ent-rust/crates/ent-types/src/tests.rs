//! Aller-retour JSON sur des exemples au format d'`ent-api`.
//! Le contrôle contre le vrai backend est dans `ent-web-server/tests/contre_backend.rs`.

use crate::appel::*;
use crate::auth::*;
use crate::edt::*;
use crate::vie_scolaire::*;

fn aller_retour<T: serde::Serialize + serde::de::DeserializeOwned>(json: &str) {
    let attendu: serde_json::Value = serde_json::from_str(json).unwrap();
    let t: T = serde_json::from_str(json).unwrap();
    let obtenu = serde_json::to_value(&t).unwrap();
    assert_eq!(obtenu, attendu, "le JSON doit être reproduit à l'identique");
}

#[test]
fn appel_et_lignes() {
    aller_retour::<AppelDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","classe":"4B","jour":"2026-11-02",
            "statut":"ouvert","creneau_id":"6f1c3c80-0000-4000-8000-000000000003","matiere_libelle":"Potions",
            "lignes":[{"eleve_id":"6f1c3c80-0000-4000-8000-000000000002","eleve_label":"Harry Potter","etat":"present"},
                      {"eleve_id":"6f1c3c80-0000-4000-8000-000000000004","etat":"absent"}]}"#,
    );
    // Champs facultatifs à null : conservés tels quels (pas de skip côté ent-api).
    aller_retour::<AppelDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","classe":"4B","jour":"2026-11-02",
            "statut":"clos","creneau_id":null,"matiere_libelle":null,"lignes":[]}"#,
    );
    let a: AppelDto = serde_json::from_str(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","classe":"4B","jour":"2026-11-02",
            "statut":"clos","creneau_id":null,"matiere_libelle":null,"lignes":[]}"#,
    )
    .unwrap();
    assert!(a.est_clos());
    aller_retour::<OuvrirAppelBody>(
        r#"{"creneau_id":"6f1c3c80-0000-4000-8000-000000000003","jour":"2026-11-02"}"#,
    );
    aller_retour::<MarquerAppelLigneBody>(
        r#"{"eleve_id":"6f1c3c80-0000-4000-8000-000000000002","etat":"retard"}"#,
    );
}

#[test]
fn creneau() {
    aller_retour::<EdtCreneauDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000003","classe":"4B","matiere_libelle":"Potions",
            "prof_user_id":"6f1c3c80-0000-4000-8000-000000000005","prof_label":"S. Rogue","jour_semaine":1,
            "heure_debut":"08:00:00","heure_fin":"09:00:00","salle":"Donjons","semaine_ab":"toutes"}"#,
    );
    aller_retour::<EdtCreneauDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000003","classe":"4B","matiere_libelle":"Potions",
            "prof_user_id":"6f1c3c80-0000-4000-8000-000000000005","jour_semaine":1,
            "heure_debut":"08:00:00","heure_fin":"09:00:00","salle":null,"semaine_ab":"A"}"#,
    );
}

#[test]
fn session_aplatie_et_defi_passkey() {
    aller_retour::<LoginResponseDto>(
        r#"{"mfa_required":false,"token":"t","user_id":"6f1c3c80-0000-4000-8000-000000000001",
            "email":"a@exemple.test","etablissement_id":"6f1c3c80-0000-4000-8000-000000000002",
            "role":"prof","expires_at":"2026-11-03T08:00:00Z"}"#,
    );
    aller_retour::<LoginResponseDto>(
        r#"{"mfa_required":true,"passkey":{"token":"x","options":{"publicKey":{}},"expires_at":"2026-11-03T08:00:00Z"}}"#,
    );
    aller_retour::<MeDto>(
        r#"{"user_id":"6f1c3c80-0000-4000-8000-000000000001","email":"a@exemple.test",
            "etablissement_id":"6f1c3c80-0000-4000-8000-000000000002","role":"eleve",
            "eleve_id":"6f1c3c80-0000-4000-8000-000000000009","passkey_count":0,
            "portee":"complete","assurance":"mot_de_passe","client":"web",
            "passkey_obligatoire":false}"#,
    );
    // Ancien format (avant le socle 1) : toujours lisible, valeurs par défaut sûres.
    let ancien: MeDto = serde_json::from_str(
        r#"{"user_id":"6f1c3c80-0000-4000-8000-000000000001","email":"a@exemple.test",
            "etablissement_id":"6f1c3c80-0000-4000-8000-000000000002","role":"prof",
            "passkey_count":1}"#,
    )
    .unwrap();
    assert_eq!(ancien.portee, "complete");
    assert!(!ancien.passkey_obligatoire);
}

#[test]
fn secrets_masques_dans_debug() {
    let b = LoginBody {
        email: "a@exemple.test".into(),
        password: "ne-pas-afficher".into(),
        etablissement_id: uuid::Uuid::nil(),
    };
    assert!(!format!("{b:?}").contains("ne-pas-afficher"));
    let s: LoginResponseDto = serde_json::from_str(
        r#"{"mfa_required":false,"token":"jeton-secret","user_id":"6f1c3c80-0000-4000-8000-000000000001",
            "email":"a@exemple.test","etablissement_id":"6f1c3c80-0000-4000-8000-000000000002",
            "role":"prof","expires_at":"2026-11-03T08:00:00Z"}"#,
    )
    .unwrap();
    assert!(!format!("{s:?}").contains("jeton-secret"));
}

#[test]
fn vie_scolaire() {
    aller_retour::<AbsenceDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","etablissement_id":"6f1c3c80-0000-4000-8000-000000000009",
            "eleve_id":"6f1c3c80-0000-4000-8000-000000000002","eleve_label":"Harry Potter","debut":"2026-11-03",
            "fin":"2026-11-04","motif":"Absence à l'appel","statut":"a_traiter","motif_justif":"Maladie",
            "decision_note":null}"#,
    );
    // Sans libellé (routes non enrichies) : champ absent conservé tel quel.
    aller_retour::<AbsenceDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","etablissement_id":"6f1c3c80-0000-4000-8000-000000000009",
            "eleve_id":"6f1c3c80-0000-4000-8000-000000000002","debut":"2026-11-03","fin":"2026-11-03",
            "motif":"x","statut":"justifiee","motif_justif":null,"decision_note":"Certificat reçu"}"#,
    );
    aller_retour::<SignalAbsenceDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","eleve_id":"6f1c3c80-0000-4000-8000-000000000002",
            "eleve_label":"Harry Potter","annee_scolaire":"2026-2027","seuil":10,"compteur":10,"statut":"ouvert",
            "note_vs":null,"traite_par_user_id":null,"fil_alerte_id":null,"commission_dossier_id":null}"#,
    );
    aller_retour::<RetardDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","eleve_id":"6f1c3c80-0000-4000-8000-000000000002",
            "jour":"2026-11-03","minutes":7,"motif":"Bus"}"#,
    );
    aller_retour::<AbsenceMotifDto>(
        r#"{"code":"maladie","libelle":"Maladie","etablissement_id":null}"#,
    );
    aller_retour::<EnfantFamilleDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000002","nom":"Potter","prenom":"Harry","classe":"4B",
            "status":"actif","lien":"mere","peut_voir_notes":true,"peut_voir_vs":true,
            "peut_justifier_absence":true,"peut_autoriser_sorties":false}"#,
    );
    aller_retour::<JustifierAbsenceBody>(r#"{"motif_justif":"Maladie"}"#);
    aller_retour::<DecisionAbsenceBody>(r#"{"note":null}"#);
    aller_retour::<CloreSignalBody>(r#"{"note":"Famille contactée"}"#);
    // Lectures partielles : les champs inconnus sont ignorés.
    let r: FamilleResumeDto = serde_json::from_str(
        r#"{"eleve_id":"6f1c3c80-0000-4000-8000-000000000002","absences_a_justifier":[],
            "carnet_a_signer":[],"notes":[]}"#,
    )
    .unwrap();
    assert!(r.absences_a_justifier.is_empty());
    let f: EleveFicheDto = serde_json::from_str(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000002","nom":"Potter","prenom":"Harry","classe":"4B",
            "ine":null,"status":"actif"}"#,
    )
    .unwrap();
    assert_eq!(f.classe.as_deref(), Some("4B"));
}

#[test]
fn referentiel_aller_retour() {
    use crate::referentiel::*;
    aller_retour::<ClasseDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","annee_id":"6f1c3c80-0000-4000-8000-000000000002",
            "code":"4B","libelle":"4B","niveau":"4e","effectif":12,
            "professeur_principal_id":"6f1c3c80-0000-4000-8000-000000000003",
            "professeur_principal_label":"Severus Rogue"}"#,
    );
    aller_retour::<ClasseDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","annee_id":"6f1c3c80-0000-4000-8000-000000000002",
            "code":"ULIS","libelle":"ULIS","niveau":null,"effectif":0,"professeur_principal_id":null}"#,
    );
    aller_retour::<EleveClasseDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","nom":"Potter","prenom":"Harry",
            "ine":null,"date_naissance":null,"status":"inscrit"}"#,
    );
    aller_retour::<RapportAmorcageDto>(
        r#"{"annee_id":"6f1c3c80-0000-4000-8000-000000000002","annee_libelle":"2026-2027",
            "classes_creees":3,"affectations_creees":40,"services_crees":5,"divergences":0}"#,
    );
    // Champs facultatifs absents des corps de création.
    let c: CreerClasseBody = serde_json::from_str(r#"{"code":"6A"}"#).unwrap();
    assert!(c.libelle.is_none() && c.niveau.is_none() && c.annee_id.is_none());
    let s: CreerServiceBody = serde_json::from_str(
        r#"{"prof_user_id":"6f1c3c80-0000-4000-8000-000000000001","classe_id":"6f1c3c80-0000-4000-8000-000000000002"}"#,
    )
    .unwrap();
    assert!(!s.professeur_principal && s.groupe_id.is_none() && s.matiere_id.is_none());
    let a: CreerAnneeBody = serde_json::from_str(
        r#"{"libelle":"2026-2027","date_debut":"2026-09-01","date_fin":"2027-08-31"}"#,
    )
    .unwrap();
    assert!(!a.courante);
}

#[test]
fn dossier_fiche_et_chronologie() {
    use crate::dossier::*;
    let personne = r#"{"id":"6f1c3c80-0000-4000-8000-000000000010","nom":"Potter","prenom":"James",
        "civilite":null,"email":null,"telephone":"06 11 22 33 44","telephone_2":null,"adresse":null,
        "a_compte":false,"represente_eleve":false,"fusionnee_dans":null}"#;
    aller_retour::<PersonneDto>(personne);
    let resp = format!(
        r#"{{"lien_id":"6f1c3c80-0000-4000-8000-000000000011","eleve_id":"6f1c3c80-0000-4000-8000-000000000002",
        "personne":{personne},"foyer_id":null,"foyer_libelle":null,"lien":"père","autorite_parentale":true,
        "garde":"alternee","date_debut":null,"date_fin":"2027-07-04","suspendu_le":"2026-11-05T10:00:00Z",
        "suspension_motif":null,"suspension_reference":null,"actif":false,"peut_voir_notes":true,
        "peut_voir_vs":true,"peut_justifier_absence":false,"peut_autoriser_sorties":false,
        "est_payeur":true,"part_paiement":50}}"#
    );
    aller_retour::<ResponsableDto>(&resp);
    aller_retour::<EntreeChronologieDto>(
        r#"{"quand":"2026-11-05T10:00:00Z","source":"evenement","domaine":"vie_scolaire","titre":"Retard"}"#,
    );
    let c: ContactBody = serde_json::from_str(r#"{"nouvelle_personne":{"nom":"Dursley"},"relation":"tante"}"#).unwrap();
    assert_eq!(c.ordre, 1);
    assert!(c.personne_id.is_none() && !c.peut_recuperer);
    aller_retour::<PayeursBody>(r#"{"payeurs":[{"lien_id":"6f1c3c80-0000-4000-8000-000000000011","part":100}]}"#);
}
