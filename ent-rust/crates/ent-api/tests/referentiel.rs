//! Référentiel scolaire : tests HTTP (droits par rôle, établissement croisé, minimisation
//! d'identité, synchronisation avec le changement de classe) contre le vrai routeur et la
//! base locale de test. Sans `ENT_DATABASE_URL` ou sans labo, les tests sont ignorés.
//!
//!   cd ent-rust && cargo test -p ent-api --features labo --test referentiel -- --test-threads=1

#[path = "separation/harness.rs"]
mod harness;

use harness::*;
use serde_json::{json, Value};
use uuid::Uuid;

use Acteur::*;

fn aujourdhui() -> String {
    domain_eleve::horloge::jour_paris().to_string()
}

fn code_unique() -> String {
    format!("T{}", &Uuid::new_v4().simple().to_string()[..6])
}

async fn classe_id(m: &Monde, jeton: &str, code: &str) -> Uuid {
    let r = m
        .appel("GET", "/referentiel/classes", Some(jeton), None)
        .await;
    assert!(r.est_succes(), "liste des classes : {}", r.statut);
    let id = r.json.as_array().unwrap().iter().find(|c| c["code"] == code);
    Uuid::parse_str(id.unwrap_or_else(|| panic!("classe {code} absente"))["id"].as_str().unwrap())
        .unwrap()
}

#[test]
fn droits_par_role_et_sans_session() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        // Sans session ou avec un jeton inventé : 401 sur toutes les routes.
        for (methode, chemin) in [
            ("GET", "/referentiel/annees"),
            ("POST", "/referentiel/annees"),
            ("GET", "/referentiel/classes"),
            ("POST", "/referentiel/classes"),
            ("GET", "/referentiel/classes/00000000-0000-0000-0000-000000000001"),
            ("POST", "/referentiel/classes/00000000-0000-0000-0000-000000000001/affectations"),
            ("POST", "/referentiel/groupes"),
            ("POST", "/referentiel/services"),
            ("POST", "/referentiel/amorcer"),
            ("GET", "/referentiel/mes-classes"),
        ] {
            let corps = (methode == "POST").then(|| json!({}));
            for jeton in [None, Some("jeton-invente")] {
                let r = m.appel(methode, chemin, jeton, corps.as_ref()).await;
                assert_eq!(r.statut.as_u16(), 401, "{methode} {chemin}");
            }
        }

        // Lecture : tout le personnel ; famille refusée.
        for a in Acteur::STAFF {
            for chemin in ["/referentiel/annees", "/referentiel/classes", "/referentiel/mes-classes"] {
                let r = m.appel("GET", chemin, Some(m.jeton(a)), None).await;
                assert!(r.est_succes(), "{a:?} GET {chemin} → {}", r.statut);
            }
        }
        for a in Acteur::FAMILLE {
            for chemin in ["/referentiel/annees", "/referentiel/classes", "/referentiel/mes-classes"] {
                let r = m.appel("GET", chemin, Some(m.jeton(a)), None).await;
                assert_eq!(r.statut.as_u16(), 403, "{a:?} GET {chemin}");
            }
        }

        // Écriture : Admin / Direction / Administratif seulement.
        for a in Acteur::TOUS {
            let gestion = matches!(a, Admin | Direction | Administratif);
            let r = m
                .appel(
                    "POST",
                    "/referentiel/classes",
                    Some(m.jeton(a)),
                    Some(&json!({"code": code_unique()})),
                )
                .await;
            if gestion {
                assert_eq!(r.statut.as_u16(), 201, "{a:?} crée une classe : {}", r.texte);
            } else {
                assert_eq!(r.statut.as_u16(), 403, "{a:?} ne crée pas de classe");
            }
            let r = m
                .appel("POST", "/referentiel/amorcer", Some(m.jeton(a)), None)
                .await;
            assert_eq!(
                r.statut.as_u16(),
                if gestion { 200 } else { 403 },
                "{a:?} amorcer : {}",
                r.texte
            );
            let r = m
                .appel(
                    "POST",
                    "/referentiel/groupes",
                    Some(m.jeton(a)),
                    Some(&json!({"code": code_unique()})),
                )
                .await;
            assert_eq!(r.statut.as_u16(), if gestion { 201 } else { 403 }, "{a:?} groupe");
        }
    });
}

#[test]
fn amorcage_idempotent_et_mes_classes() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let r = m
            .appel("POST", "/referentiel/amorcer", Some(m.jeton(Administratif)), None)
            .await;
        assert_eq!(r.statut.as_u16(), 200);
        assert_eq!(r.json["classes_creees"], 0, "idempotent : {}", r.texte);
        assert_eq!(r.json["affectations_creees"], 0);
        assert_eq!(r.json["services_crees"], 0);

        // Professeur : seulement ses classes (Prof 1 enseigne en 4B, pas en 3C) ; personnel de gestion : toutes.
        let prof = m.appel("GET", "/referentiel/mes-classes", Some(m.jeton(Prof)), None).await;
        let codes: Vec<&str> = prof.json.as_array().unwrap().iter().map(|c| c["code"].as_str().unwrap()).collect();
        assert!(codes.contains(&"4B") && !codes.contains(&"3C"), "{codes:?}");
        let gestion = m.appel("GET", "/referentiel/mes-classes", Some(m.jeton(Administratif)), None).await;
        let toutes: Vec<&str> = gestion.json.as_array().unwrap().iter().map(|c| c["code"].as_str().unwrap()).collect();
        assert!(toutes.contains(&"4B") && toutes.contains(&"3C"), "{toutes:?}");
        let c4b = prof.json.as_array().unwrap().iter().find(|c| c["code"] == "4B").unwrap();
        assert!(c4b["effectif"].as_i64().unwrap() >= 1);
    });
}

#[test]
fn detail_de_classe_minimisation_et_cloisonnement() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let id = classe_id(m, m.jeton(Administratif), "4B").await;
        let chemin = format!("/referentiel/classes/{id}");

        // Même liste d'élèves pour tous les rôles du personnel, identité minimisée selon le rôle.
        let admin = m.appel("GET", &chemin, Some(m.jeton(Administratif)), None).await;
        let prof = m.appel("GET", &chemin, Some(m.jeton(Prof)), None).await;
        assert!(admin.est_succes() && prof.est_succes());
        let noms = |v: &Value| -> Vec<String> {
            let mut n: Vec<String> = v["eleves"].as_array().unwrap().iter()
                .map(|e| format!("{} {}", e["nom"].as_str().unwrap(), e["prenom"].as_str().unwrap()))
                .collect();
            n.sort();
            n
        };
        assert_eq!(noms(&admin.json), noms(&prof.json));
        assert!(!noms(&prof.json).is_empty());
        for e in prof.json["eleves"].as_array().unwrap() {
            assert!(e["ine"].is_null(), "INE masqué pour un enseignant : {e}");
            assert!(e["date_naissance"].is_null(), "date de naissance masquée : {e}");
        }
        assert!(admin.json["eleves"].as_array().unwrap().iter().any(|e| !e["date_naissance"].is_null()),
            "le personnel de gestion voit la date de naissance");
        // Services : professeur et matière lisibles.
        let services = prof.json["services"].as_array().unwrap();
        assert!(services.iter().any(|s| s["prof_label"].as_str().is_some_and(|l| !l.is_empty())
            && s["matiere_libelle"] == "Potions"), "{services:?}");
        let texte = prof.texte.clone();
        for interdit in ["password", "numero_badge", "storage_key"] {
            assert!(!texte.contains(interdit), "{interdit} ne doit pas apparaître");
        }

        // Famille : refus. Autre établissement : 404 (comme une classe inconnue).
        for a in [Parent, Eleve] {
            let r = m.appel("GET", &chemin, Some(m.jeton(a)), None).await;
            assert_eq!(r.statut.as_u16(), 403, "{a:?}");
        }
        let id_b = {
            let jeton_b = m.b.jetons.get(&Administratif).expect("administratif de B");
            classe_id(m, jeton_b, "4B").await
        };
        for a in [Administratif, Prof, Admin] {
            let r = m.appel("GET", &format!("/referentiel/classes/{id_b}"), Some(m.jeton(a)), None).await;
            assert_eq!(r.statut.as_u16(), 404, "{a:?} lit la classe de B");
        }
        let inconnue = m
            .appel("GET", &format!("/referentiel/classes/{}", Uuid::new_v4()), Some(m.jeton(Administratif)), None)
            .await;
        assert_eq!(inconnue.statut.as_u16(), 404);
    });
}

#[test]
fn affectation_service_et_synchronisation() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let adm = m.jeton(Administratif);
        let luna = m.a.objets.eleve_neutre;
        let id_5a = classe_id(m, adm, "5A").await;

        // Droits et références d'un autre établissement pour l'affectation.
        let corps = json!({"eleve_id": luna});
        let r = m.appel("POST", &format!("/referentiel/classes/{id_5a}/affectations"), Some(m.jeton(Prof)), Some(&corps)).await;
        assert_eq!(r.statut.as_u16(), 403);
        let eleve_b = m.b.objets.eleve;
        let r = m.appel("POST", &format!("/referentiel/classes/{id_5a}/affectations"), Some(adm), Some(&json!({"eleve_id": eleve_b}))).await;
        assert_eq!(r.statut.as_u16(), 404, "élève d'un autre établissement : {}", r.texte);
        let id_b = {
            let jeton_b = m.b.jetons.get(&Administratif).expect("administratif de B");
            classe_id(m, jeton_b, "5A").await
        };
        let r = m.appel("POST", &format!("/referentiel/classes/{id_b}/affectations"), Some(adm), Some(&corps)).await;
        assert_eq!(r.statut.as_u16(), 404, "classe d'un autre établissement : {}", r.texte);

        // Affectation valide, puis lecture.
        let r = m.appel("POST", &format!("/referentiel/classes/{id_5a}/affectations"), Some(adm), Some(&corps)).await;
        assert_eq!(r.statut.as_u16(), 201, "{}", r.texte);
        let detail = m.appel("GET", &format!("/referentiel/classes/{id_5a}"), Some(adm), None).await;
        let present = |d: &Value, id: Uuid| d["eleves"].as_array().unwrap().iter().any(|e| e["id"] == id.to_string());
        assert!(present(&detail.json, luna));

        // Changement de classe par la route existante : l'affectation suit.
        let id_4b = classe_id(m, adm, "4B").await;
        let r = m
            .appel("POST", &format!("/eleves/{luna}/classe"), Some(adm),
                Some(&json!({"nouvelle_classe": "4B", "date_effet": aujourdhui()})))
            .await;
        assert!(r.est_succes(), "changement de classe : {} {}", r.statut, r.texte);
        let d4 = m.appel("GET", &format!("/referentiel/classes/{id_4b}"), Some(adm), None).await;
        let d5 = m.appel("GET", &format!("/referentiel/classes/{id_5a}"), Some(adm), None).await;
        assert!(present(&d4.json, luna), "Luna est en 4B dans le référentiel");
        assert!(!present(&d5.json, luna), "Luna a quitté la 5A");

        // Service : seule la gestion crée ; un professeur de l'autre établissement est introuvable.
        let prof = m.a.user_ids[&Prof];
        let corps = json!({"prof_user_id": prof, "classe_id": id_5a, "professeur_principal": true});
        let r = m.appel("POST", "/referentiel/services", Some(m.jeton(Prof)), Some(&corps)).await;
        assert_eq!(r.statut.as_u16(), 403);
        let r = m.appel("POST", "/referentiel/services", Some(adm), Some(&corps)).await;
        assert_eq!(r.statut.as_u16(), 201, "{}", r.texte);
        assert_eq!(r.json["professeur_principal"], true);
        let r = m.appel("POST", "/referentiel/services", Some(adm), Some(&corps)).await;
        assert_eq!(r.statut.as_u16(), 409, "service en double");
        let prof_b = m.b.user_ids[&Prof];
        let r = m
            .appel("POST", "/referentiel/services", Some(adm), Some(&json!({"prof_user_id": prof_b, "classe_id": id_5a})))
            .await;
        assert_eq!(r.statut.as_u16(), 404, "professeur d'un autre établissement : {}", r.texte);
        let r = m
            .appel("POST", "/referentiel/services", Some(adm), Some(&json!({"prof_user_id": prof})))
            .await;
        assert_eq!(r.statut.as_u16(), 400, "ni classe ni groupe");
        let liste = m.appel("GET", "/referentiel/classes", Some(adm), None).await;
        let c5a = liste.json.as_array().unwrap().iter().find(|c| c["code"] == "5A").unwrap();
        assert_eq!(c5a["professeur_principal_id"], prof.to_string());
        assert!(c5a["professeur_principal_label"].is_string());
    });
}
