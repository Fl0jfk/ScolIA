//! Non-régression des correctifs de l'audit n° 1 (`docs/audits/AUDIT_FABLE_1.md`) : un test par
//! constat, joué contre le routeur réel et le service réel (PostgreSQL local de test).
//!
//!  - D-01 : `PUT /eleves/{id}/fiche-sante` par un parent ne renvoie plus la note confidentielle.
//!  - D-02 : `/psy/*` et `/dossier-psy` sont sous le verrou « module santé » ; le verrou vit aussi
//!    dans `ent-db` (appel direct du service, sans passer par HTTP).
//!  - D-03 : `GET /rh/absences` : le personnel hors gestion ne voit que ses absences, sans le
//!    motif des autres.
//!  - A-02 : `documents.lire` n'ouvre plus les pièces `sante` / `restreint`.
//!  - A-03 : la liste des documents ne révèle pas aux familles les pièces illisibles pour elles.
//!
//! Lancement : voir `recette_securite_3.rs` (mêmes variables ; sans base : SKIP).

#![allow(dead_code)]

#[path = "separation/harness.rs"]
mod harness;

use harness::*;
use serde_json::{json, Value};
use uuid::Uuid;

use Acteur::*;

macro_rules! monde_ou_skip {
    () => {
        match monde() {
            Some(m) => m,
            None => return eprintln!("SKIP : pas de base de test"),
        }
    };
}

fn fiche(note: &str) -> Value {
    json!({
        "allergies": "arachides", "traitements": "", "contre_indications": "",
        "pai_actif": false, "medecin_nom": null, "urgence_tel": null,
        "note_confidentielle": note,
    })
}

/// D-01.
#[test]
fn d01_le_parent_ne_recoit_pas_la_note_confidentielle_en_ecrivant_la_fiche() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let eleve = m.a.objets.eleve;
        let chemin = format!("/eleves/{eleve}/fiche-sante");
        let secret = "NOTE-CONFIDENTIELLE-D01";
        let r = m
            .appel("PUT", &chemin, Some(m.jeton(Infirmiere)), Some(&fiche(secret)))
            .await;
        assert!(r.est_succes(), "infirmière : {}", r.texte);
        assert!(r.texte.contains(secret), "l'infirmière voit sa note");

        let r = m
            .appel("PUT", &chemin, Some(m.jeton(Parent)), Some(&fiche("tentative parent")))
            .await;
        assert!(r.est_succes(), "parent lié : {}", r.texte);
        assert!(!r.texte.contains(secret), "note confidentielle rendue au parent : {}", r.texte);
        assert!(!r.texte.contains("tentative parent"), "{}", r.texte);

        // La note n'a pas été écrasée par le parent, et la lecture reste filtrée.
        let r = m.appel("GET", &chemin, Some(m.jeton(Infirmiere)), None).await;
        assert!(r.texte.contains(secret), "note conservée : {}", r.texte);
        let r = m.appel("GET", &chemin, Some(m.jeton(Parent)), None).await;
        assert!(!r.texte.contains(secret), "{}", r.texte);
    });
}

/// D-02 : par HTTP (préfixe `/psy`, segment `dossier-psy`) et dans `ent-db`.
#[test]
fn d02_les_routes_psy_sont_sous_le_verrou_module_sante() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let eleve = m.b.objets.eleve;
        let tok = m.b.jetons.get(&Direction).expect("direction B").clone();
        let psy = m.b.jetons.get(&Psy).expect("psy B").clone();
        let psy_id = *m.b.user_ids.get(&Psy).expect("psy B");
        let etab = m.b.objets.etab;
        let off = m
            .appel("PUT", "/sante/module", Some(&tok), Some(&json!({"actif": false})))
            .await;
        assert!(off.est_succes(), "désactivation : {}", off.texte);

        let chemins = [
            ("GET", "/psy/entretiens".to_string()),
            ("GET", "/psy/rdvs".to_string()),
            ("GET", format!("/eleves/{eleve}/dossier-psy")),
            ("GET", format!("/eleves/{eleve}/dossier-psy/courrier.pdf")),
        ];
        for (methode, chemin) in &chemins {
            let r = m.appel(methode, chemin, Some(&psy), None).await;
            assert_eq!(r.statut, 403, "{chemin} doit être verrouillé : {}", r.texte);
        }

        // Couche métier (`ent-db`) : le verrou ne dépend pas du chemin HTTP.
        let (state, _) = ent_api::boot_from_env().await.expect("état");
        let ent = &state.ent;
        let r = ent
            .get_dossier_psy_for_actor(etab, psy_id, "psychologue", eleve)
            .await;
        assert!(matches!(r, Err(ent_db::EntDbError::Forbidden)), "dossier psy : {r:?}");
        let r = ent
            .list_entretiens_psy_for_actor(etab, psy_id, "psychologue", None, None)
            .await;
        assert!(matches!(r, Err(ent_db::EntDbError::Forbidden)), "entretiens psy : {r:?}");
        let r = ent
            .list_rdvs_psy_for_actor(etab, psy_id, "psychologue", None, None)
            .await;
        assert!(matches!(r, Err(ent_db::EntDbError::Forbidden)), "rdv psy : {r:?}");
        let r = ent.exiger_module_sante(etab).await;
        assert!(matches!(r, Err(ent_db::EntDbError::Forbidden)));

        // Réactivé : le psychologue retrouve l'accès (le verrou ne casse pas le cas normal).
        let on = m
            .appel("PUT", "/sante/module", Some(&tok), Some(&json!({"actif": true})))
            .await;
        assert!(on.est_succes(), "{}", on.texte);
        let r = m.appel("GET", "/psy/entretiens", Some(&psy), None).await;
        assert!(r.est_succes(), "module actif : {} {}", r.statut, r.texte);
        assert!(ent.exiger_module_sante(etab).await.is_ok());
    });
}

/// D-03.
#[test]
fn d03_les_absences_rh_ne_sont_pas_lisibles_par_tout_le_personnel() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let cpe = m.a.user_ids[&Cpe];
        let motif = "MOTIF-MEDICAL-D03";
        let r = m
            .appel(
                "POST",
                "/rh/absences",
                Some(m.jeton(Direction)),
                Some(&json!({"user_id": cpe, "debut": "2031-03-03", "fin": "2031-03-04", "motif": motif})),
            )
            .await;
        assert!(r.est_succes(), "déclaration : {}", r.texte);
        let q = "/rh/absences?debut=2031-03-01&fin=2031-03-31";

        // Direction (gestion) : tout, motif compris.
        let r = m.appel("GET", q, Some(m.jeton(Direction)), None).await;
        assert!(r.texte.contains(motif), "direction : {}", r.texte);

        // Prof et accueil (administratif) ne voient ni l'absence du CPE ni son motif.
        for a in [Prof, Compta, Psy] {
            let r = m.appel("GET", q, Some(m.jeton(a)), None).await;
            assert!(!r.texte.contains(motif), "{a:?} voit le motif : {}", r.texte);
            assert!(!r.texte.contains(&cpe.to_string()), "{a:?} voit l'absence d'un collègue : {}", r.texte);
        }

        // Le CPE retrouve la sienne, avec son motif.
        let r = m.appel("GET", q, Some(m.jeton(Cpe)), None).await;
        assert!(r.texte.contains(motif), "le CPE voit sa propre absence : {}", r.texte);
    });
}

async fn deposer(m: &Monde, eleve: Uuid, conf: &str, titre: &str) -> Uuid {
    let r = m
        .appel(
            "POST",
            &format!("/eleves/{eleve}/documents"),
            Some(m.jeton(Direction)),
            Some(&json!({
                "titre": titre, "tiroir": "scolaire", "confidentialite": conf,
                "code": "CODE-A03", "mime_type": "text/plain",
                "contenu_base64": "Y29udGVudQ==",
            })),
        )
        .await;
    assert!(r.est_succes(), "dépôt {conf} : {}", r.texte);
    r.json["id"].as_str().expect("id").parse().expect("uuid")
}

/// A-02 : un rôle qui n'a que `documents.lire` (profil paramétré) ne lit ni `sante` ni `restreint`.
#[test]
fn a02_documents_lire_n_ouvre_que_les_documents_standard() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let eleve = m.a.objets.eleve;
        let std = deposer(m, eleve, "standard", "Standard A02").await;
        let restreint = deposer(m, eleve, "restreint", "Restreint A02").await;
        let sante = deposer(m, eleve, "sante", "Santé A02").await;

        let (state, _) = ent_api::boot_from_env().await.expect("état");
        let ent = &state.ent;
        let etab = m.a.objets.etab;
        let admin = m.a.user_ids[&Direction];

        // Rôle paramétré « documents.lire seul », attribué au professeur du labo.
        let role = ent
            .creer_role_for_actor(
                etab,
                admin,
                "direction",
                ent_db::RoleSaisie {
                    nom: format!("Lecteur documents {}", &Uuid::new_v4().simple().to_string()[..6]),
                    description: None,
                    modele: None,
                    droits: vec![ent_db::Droit {
                        action: ent_auth::Action::DocumentsLire,
                        portee: ent_auth::Portee::Etablissement,
                        cibles: vec![],
                    }],
                },
            )
            .await
            .expect("création du rôle");
        let user = m.a.user_ids[&Prof];
        ent.attribuer_role_for_actor(etab, admin, "direction", role.id, user, None)
            .await
            .expect("attribution");
        let role = "prof".to_string();
        let peut = |id: Uuid| {
            let ent = ent.clone();
            async move {
                ent.telecharger_document_bytes_for_actor(etab, user, &role, id)
                    .await
                    .is_ok()
            }
        };
        assert!(peut(std).await, "standard lisible avec documents.lire");
        assert!(!peut(restreint).await, "restreint ne doit pas s'ouvrir avec documents.lire seul");
        assert!(!peut(sante).await, "santé ne doit pas s'ouvrir avec documents.lire seul");

        // Profils par défaut (direction) : inchangés — standard, restreint (gerer_droits) et santé.
        for id in [std, restreint, sante] {
            assert!(
                ent.telecharger_document_bytes_for_actor(etab, admin, "direction", id)
                    .await
                    .is_ok(),
                "la direction lit toujours {id}"
            );
        }
    });
}

/// A-03.
#[test]
fn a03_la_liste_des_documents_ne_revele_pas_les_pieces_restreintes_aux_familles() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let eleve = m.a.objets.eleve;
        let restreint = deposer(m, eleve, "restreint", "Information préoccupante A03").await;
        let standard = deposer(m, eleve, "standard", "Note interne A03").await;
        let chemin = format!("/eleves/{eleve}/documents");

        for a in [Parent, Eleve] {
            let r = m.appel("GET", &chemin, Some(m.jeton(a)), None).await;
            assert!(r.est_succes(), "{a:?} : {}", r.texte);
            let ids: Vec<String> = r
                .json
                .as_array()
                .expect("liste")
                .iter()
                .map(|d| d["id"].as_str().unwrap_or("").to_string())
                .collect();
            assert!(!ids.contains(&restreint.to_string()), "{a:?} voit l'existence d'une pièce restreinte");
            assert!(!ids.contains(&standard.to_string()), "{a:?} voit une pièce interne hors tiroir famille");
            assert!(!r.texte.contains("CODE-A03"), "{a:?} : code fuité");
            assert!(!r.texte.contains("A03"), "{a:?} : titre fuité : {}", r.texte);
        }

        // Le personnel autorisé voit tout ; le personnel sans droit voit l'existence mais ni code ni source.
        let r = m.appel("GET", &chemin, Some(m.jeton(Direction)), None).await;
        assert!(r.texte.contains("Information préoccupante A03"), "{}", r.texte);
        let r = m.appel("GET", &chemin, Some(m.jeton(Compta)), None).await;
        if r.est_succes() {
            assert!(!r.texte.contains("CODE-A03"), "code visible du personnel sans droit : {}", r.texte);
            assert!(!r.texte.contains("Information préoccupante A03"), "{}", r.texte);
        }
    });
}
