//! Stages, lot ST0 — défauts du Next.js fermés par la version Rust (PILOTAGE règle 14, D-051).
//!
//! Le routeur réel `ent_api::app` est appelé en process sur un PostgreSQL **local de test**, avec
//! les établissements de labo A et B (même principe que `recette_finances_f0.rs`). Chaque test
//! porte le numéro du défaut de `docs/socles/39_STAGES.md` §2.3 (DN-xx) qu'il verrouille ; ce sont
//! les actions passent par le **compte connecté** (aucun contournement d'authentification).
//!
//! Lancement : voir `recette_finances_f0.rs` (même base, `--features ent-api/labo`).

#![allow(dead_code)]

#[path = "separation/harness.rs"]
mod harness;
#[path = "separation/routes_src.rs"]
mod routes_src;

use chrono::{Datelike, NaiveDate};
use harness::*;
use serde_json::{json, Value};
use uuid::Uuid;

use Acteur::*;

fn uuid_de(v: &Value) -> Uuid {
    Uuid::parse_str(v.as_str().unwrap_or_else(|| panic!("uuid attendu : {v}"))).expect("uuid")
}

/// Jour de novembre de l'année scolaire courante (toujours dans l'année).
fn nov(jour: u32) -> NaiveDate {
    let auj = domain_eleve::horloge::jour_paris();
    let an = if auj.month() >= 9 { auj.year() } else { auj.year() - 1 };
    NaiveDate::from_ymd_opt(an, 11, jour).unwrap()
}

async fn creer_organisme(m: &Monde) -> Uuid {
    let adm = m.jeton(Administratif).to_string();
    let r = m
        .appel(
            "POST",
            "/entreprises",
            Some(&adm),
            Some(&json!({ "raison_sociale": "Weasleys' Wizard Wheezes", "adresse": "93 Diagon Alley" })),
        )
        .await;
    assert!(r.est_succes(), "organisme : {} {}", r.statut, r.texte);
    uuid_de(&r.json["id"])
}

async fn creer_stage(m: &Monde, jeton: &str, eleve: Uuid, org: Uuid, du: NaiveDate, au: NaiveDate) -> Rep {
    m.appel(
        "POST",
        "/conventions-stage",
        Some(jeton),
        Some(&json!({ "eleve_id": eleve, "entreprise_id": org, "debut": du, "fin": au, "tuteur_nom": "Fred Weasley" })),
    )
    .await
}

/// DN-02 : un parent ne lit pas n'importe quelle convention. Il lit les stages de son enfant par
/// son lien (droit du lien), jamais ceux d'un autre foyer ni ceux d'un enfant dont le lien ne porte
/// pas le droit ; l'élève lit les siens seulement. DN-01 : la réponse ne contient aucun jeton, code
/// ni lien de signature — la forme exacte de la réponse est figée.
#[test]
fn dn01_dn02_lecture_par_le_lien_et_reponse_sans_jeton() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let org = creer_organisme(m).await;
        let adm = m.jeton(Administratif).to_string();
        let harry = m.a.objets.eleve;
        let c = creer_stage(m, &adm, harry, org, nov(2), nov(6)).await;
        assert_eq!(c.statut, 201, "{}", c.texte);
        let chemin = format!("/eleves/{harry}/conventions-stage");

        // Lecture autorisée : parent de Harry (lien avec le droit), élève Harry, secrétariat.
        for (qui, jeton) in [("parent", m.jeton(Parent)), ("élève", m.jeton(Eleve)), ("secrétariat", m.jeton(Administratif))] {
            let r = m.appel("GET", &chemin, Some(jeton), None).await;
            assert_eq!(r.statut, 200, "{qui} : {}", r.texte);
            let liste = r.json.as_array().expect("liste");
            assert!(!liste.is_empty());
            for stage in liste {
                // DN-01 : forme figée, aucune clé de jeton, de code ou de signature.
                let cles: Vec<&str> = stage.as_object().unwrap().keys().map(String::as_str).collect();
                for interdit in ["jeton", "token", "code", "signature", "lien", "secret", "otp"] {
                    assert!(
                        !cles.iter().any(|k| k.to_lowercase().contains(interdit)),
                        "{qui} : clé suspecte `{interdit}` dans {cles:?}"
                    );
                }
                let attendues = ["id", "eleve_id", "eleve_label", "entreprise_id", "entreprise_label", "debut", "fin", "tuteur_nom", "statut", "mention", "version"];
                assert!(cles.iter().all(|k| attendues.contains(k)), "{qui} : {cles:?}");
                // RS-01 : un stage déclaré n'est pas une convention signée.
                assert_eq!(stage["statut"], "declaree");
                assert_eq!(stage["mention"], "convention non signée dans ScolIA");
            }
        }

        // DN-02 : refus pour le parent d'un autre foyer, pour le parent dont le lien ne porte pas
        // le droit, pour l'élève d'un autre élève, et pour les personnels sans `stages.lire`.
        for (qui, jeton, eleve) in [
            ("parent d'un autre foyer", m.jeton(ParentAutreFoyer), harry),
            ("parent sans drapeaux", m.jeton(ParentSansDrapeaux), m.a.objets.eleve_ginny),
            ("parent de Harry sur Draco", m.jeton(Parent), m.a.objets.eleve_autre),
            ("élève sur un autre élève", m.jeton(Eleve), m.a.objets.eleve_autre),
        ] {
            let r = m.appel("GET", &format!("/eleves/{eleve}/conventions-stage"), Some(jeton), None).await;
            assert_eq!(r.statut, 403, "{qui} : {} {}", r.statut, r.texte);
        }
        for (qui, jeton) in [("CPE", m.jeton(Cpe)), ("comptabilité", m.jeton(Compta))] {
            let r = m.appel("GET", &chemin, Some(jeton), None).await;
            assert_eq!(r.statut, 403, "{qui} : {}", r.texte);
        }
        // L'établissement B ne voit rien de A.
        let b = m.b.jetons.get(&Direction).expect("direction B").clone();
        let r = m.appel("GET", &chemin, Some(&b), None).await;
        assert!(r.est_refus() || r.est_vide(), "B lit un stage de A : {} {}", r.statut, r.texte);
    });
}

/// DN-03 : personne d'autre que le personnel habilité ne modifie un stage. Ici : déclarer,
/// clôturer, annuler exigent `stages.preparer` ; la version lue est exigée (pas de « dernier
/// gagne ») ; un stage annulé ne revient pas ; un autre établissement ne touche à rien.
#[test]
fn dn03_modification_reservee_au_personnel_habilite() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let org = creer_organisme(m).await;
        let adm = m.jeton(Administratif).to_string();
        let ginny = m.a.objets.eleve_ginny;
        // Création : refusée au CPE, au professeur, au parent, à l'élève, à la comptabilité.
        for (qui, jeton) in [
            ("CPE", m.jeton(Cpe)),
            ("professeur", m.jeton(Prof)),
            ("parent", m.jeton(Parent)),
            ("élève", m.jeton(Eleve)),
            ("comptabilité", m.jeton(Compta)),
        ] {
            let r = creer_stage(m, jeton, m.a.objets.eleve, org, nov(16), nov(20)).await;
            assert_eq!(r.statut, 403, "{qui} crée un stage : {} {}", r.statut, r.texte);
        }
        // Établissement B : l'élève de A est inconnu de lui (404), son organisme aussi.
        let b = m.b.jetons.get(&Administratif).expect("secrétariat B").clone();
        let r = creer_stage(m, &b, ginny, org, nov(16), nov(20)).await;
        assert!(r.est_refus(), "B crée un stage sur un élève de A : {} {}", r.statut, r.texte);

        let c = creer_stage(m, &adm, ginny, org, nov(9), nov(13)).await;
        assert_eq!(c.statut, 201, "{}", c.texte);
        let id = uuid_de(&c.json["id"]);
        assert_eq!(c.json["version"], 1);
        // Chevauchement refusé avec un motif lisible (409).
        let r = creer_stage(m, &adm, ginny, org, nov(12), nov(14)).await;
        assert_eq!(r.statut, 409, "{}", r.texte);
        assert!(r.texte.contains("déjà un stage"), "{}", r.texte);

        let statut = |jeton: &str, version: i32, vers: &str| {
            let corps = json!({ "statut": vers, "version": version });
            let chemin = format!("/conventions-stage/{id}/statut");
            let jeton = jeton.to_string();
            async move { m.appel("POST", &chemin, Some(&jeton), Some(&corps)).await }
        };
        for (qui, jeton) in [
            ("CPE", m.jeton(Cpe)),
            ("professeur", m.jeton(Prof)),
            ("parent", m.jeton(Parent)),
            ("élève", m.jeton(Eleve)),
            ("comptabilité", m.jeton(Compta)),
        ] {
            let r = statut(jeton, 1, "annulee").await;
            assert_eq!(r.statut, 403, "{qui} change le statut : {} {}", r.statut, r.texte);
        }
        let r = statut(&b, 1, "annulee").await;
        assert!(r.est_refus(), "B change le statut d'un stage de A : {} {}", r.statut, r.texte);
        // Version périmée : 409 ; version juste : 200 et version suivante.
        let r = statut(&adm, 7, "cloturee").await;
        assert_eq!(r.statut, 409, "{}", r.texte);
        let r = statut(&adm, 1, "annulee").await;
        assert_eq!(r.statut, 200, "{}", r.texte);
        assert_eq!((r.json["statut"].as_str(), r.json["version"].as_i64()), (Some("annulee"), Some(2)));
        // Un stage annulé ne revient pas à « déclaré ».
        let r = statut(&adm, 2, "declaree").await;
        assert_eq!(r.statut, 409, "{}", r.texte);
        // Valeur de statut inconnue : refusée.
        let r = statut(&adm, 2, "signee").await;
        assert_eq!(r.statut, 409, "{}", r.texte);
    });
}

/// DN-09 : le carnet d'organismes et surtout les **contacts** (personnes extérieures) ne sont pas
/// ouverts à tout compte authentifié.
#[test]
fn dn09_carnet_et_contacts_reserves() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let org = creer_organisme(m).await;
        let adm = m.jeton(Administratif).to_string();
        let r = m
            .appel(
                "POST",
                &format!("/stages/organismes/{org}/contacts"),
                Some(&adm),
                Some(&json!({ "nom": "Weasley", "prenom": "Fred", "email": "fred@wwws.example", "est_tuteur": true, "est_signataire": true })),
            )
            .await;
        assert_eq!(r.statut, 201, "{}", r.texte);
        for (qui, jeton) in [
            ("professeur", m.jeton(Prof)),
            ("CPE", m.jeton(Cpe)),
            ("comptabilité", m.jeton(Compta)),
            ("parent", m.jeton(Parent)),
            ("élève", m.jeton(Eleve)),
        ] {
            for chemin in [format!("/stages/organismes/{org}/contacts"), format!("/stages/organismes/{org}"), "/entreprises".to_string()] {
                let r = m.appel("GET", &chemin, Some(jeton), None).await;
                assert_eq!(r.statut, 403, "{qui} lit {chemin} : {} {}", r.statut, r.texte);
            }
        }
        let r = m.appel("GET", &format!("/stages/organismes/{org}/contacts"), Some(&adm), None).await;
        assert_eq!(r.statut, 200);
        assert_eq!(r.json.as_array().map(Vec::len), Some(1));
        let b = m.b.jetons.get(&Administratif).expect("secrétariat B").clone();
        let r = m.appel("GET", &format!("/stages/organismes/{org}/contacts"), Some(&b), None).await;
        assert!(r.est_refus() || r.est_vide(), "B lit les contacts de A : {} {}", r.statut, r.texte);
    });
}

/// DN-18 : l'assurance de l'établissement est un paramètre, jamais codée en dur ; DN-01 et DN-04
/// ne sont pas encore atteignables dans ce lot (aucun lien ni jeton n'existe : ST1 (c)) — garde-fou
/// de schéma : aucune colonne de jeton, de code ou d'empreinte dans les tables des stages.
#[test]
fn dn18_assurance_parametree_et_schema_sans_jeton() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        let r = m.appel("GET", "/stages/parametres", Some(&dir), None).await;
        assert_eq!(r.statut, 200, "{}", r.texte);
        assert_eq!(r.json["assurance_renseignee"], false);
        assert!(r.json["assureur_nom"].is_null());
        let r = m
            .appel(
                "PUT",
                "/stages/parametres",
                Some(&dir),
                Some(&json!({ "assurance": { "assureur_nom": "MAE", "contrat": "C-42" } })),
            )
            .await;
        assert_eq!(r.statut, 200, "{}", r.texte);
        assert_eq!(r.json["assurance_renseignee"], true);
        let adm = m.jeton(Administratif).to_string();
        let r = m.appel("PUT", "/stages/parametres", Some(&adm), Some(&json!({ "relance_ouverture_j": 30 }))).await;
        assert_eq!(r.statut, 403, "{}", r.texte);

        let url = std::env::var("ENT_DATABASE_URL").expect("base de test");
        let pool = sqlx::postgres::PgPoolOptions::new().max_connections(1).connect(&url).await.expect("pool");
        let colonnes: Vec<(String, String)> = sqlx::query_as(
            "SELECT table_name::text, column_name::text FROM information_schema.columns
              WHERE table_schema = 'public'
                AND (table_name LIKE 'ent\\_stage\\_%' OR table_name LIKE 'ent\\_entreprise%' OR table_name = 'ent_convention_stage')
                AND column_name ~* '(jeton|token|secret|otp|empreinte|hash|code_unique|mot_de_passe)'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(colonnes.is_empty(), "colonnes de secret dans les tables des stages : {colonnes:?}");
    });
}
