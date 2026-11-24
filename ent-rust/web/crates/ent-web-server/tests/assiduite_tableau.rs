//! Tableau de bord de la vie scolaire en panneaux (socle 12, AS14b-bis) contre le **vrai** backend
//! (`ent_api::app`) et une base PostgreSQL **locale** de test : le client typé de l'écran lit la file du matin
//! (AS6), les exclus du jour, l'heure de bascule et la disposition personnelle, et le **modèle d'écran** range
//! le vrai JSON du serveur. Permissions : professeur, famille et vie scolaire seule face au réglage.
//!
//! Lancement (voir `contre_backend.rs`) :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test assiduite_tableau -- --test-threads=1
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »). Les écritures sont rendues à leur état
//! initial (heure de bascule 11 h, disposition effacée, arrivée confirmée) : le test est rejouable.
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use chrono::NaiveTime;
use ent_types::assiduite::PreferencePanneauDto;
use ent_web_api::{ApiClient, ApiError};
use ent_web_app::composants_metier::tableau_vs::modele::{
    disposition, moment, regrouper, vers_preference, Moment, Panneau,
};
use ent_web_server::labo::{self, Labo};
use serde_json::json;

async fn demarrer() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent (sourcer scripts/env.sh)");
        return None;
    };
    let parsed = ent_db::assert_local_postgres_url(&url)
        .unwrap_or_else(|e| panic!("base de test locale uniquement : {e}"));
    let nom_base = parsed.path().trim_start_matches('/').to_ascii_lowercase();
    assert!(
        ["test", "proto", "lab"]
            .iter()
            .any(|m| nom_base.contains(m)),
        "le nom de la base doit contenir test, proto ou lab (refusé : {nom_base})"
    );
    let (app, _) = ent_web_server::routeur_complet()
        .await
        .expect("démarrage ent-api + web");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    Some(format!("http://{addr}"))
}

async fn client_de(base: &str, l: &Labo, email: &str) -> ApiClient {
    let s = labo::connecter(base, l, email)
        .await
        .unwrap_or_else(|e| panic!("connexion {email} : {e:?}"));
    ApiClient::new(base, Some(s.token))
}

async fn personnel(base: &str, l: &Labo, role: &str) -> ApiClient {
    let p = l
        .personnel
        .iter()
        .find(|p| p.role == role)
        .unwrap_or_else(|| panic!("personnel {role} de labo"));
    client_de(base, l, &p.email).await
}

async fn parent_de(base: &str, l: &Labo, prenom: &str) -> ApiClient {
    let p = l
        .personas
        .iter()
        .find(|p| p.role == "parent" && p.label.contains(prenom))
        .unwrap_or_else(|| panic!("parent de {prenom}"));
    client_de(base, l, &p.email).await
}

fn h(a: u32, b: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(a, b, 0).unwrap()
}

fn ordre(codes: [&str; 5]) -> Vec<PreferencePanneauDto> {
    codes
        .iter()
        .map(|c| PreferencePanneauDto {
            panneau: (*c).into(),
            replie: *c == "incoherences",
            masque: *c == "familles",
        })
        .collect()
}

/// Les droits du tableau : la vie scolaire lit, ni le professeur ni la famille ; régler l'heure : la direction
/// seulement (comme la règle d'assiduité).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tableau_permissions() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let (cpe, dir, prof) = (
        personnel(&base, &l, "cpe").await,
        personnel(&base, &l, "direction").await,
        personnel(&base, &l, "prof").await,
    );
    let parent = parent_de(&base, &l, "Harry").await;

    for (nom, c) in [("prof", &prof), ("parent", &parent)] {
        assert!(
            matches!(c.exclus_du_jour().await, Err(ApiError::Interdit(_))),
            "{nom} : exclus"
        );
        assert!(
            matches!(c.config_tableau().await, Err(ApiError::Interdit(_))),
            "{nom} : configuration"
        );
        assert!(
            matches!(c.file_du_matin(None).await, Err(ApiError::Interdit(_))),
            "{nom} : file"
        );
        assert!(
            matches!(
                c.poser_preference_tableau(ordre([
                    "absents",
                    "exclus",
                    "appels",
                    "incoherences",
                    "familles"
                ]))
                .await,
                Err(ApiError::Interdit(_))
            ),
            "{nom} : disposition"
        );
        assert!(
            matches!(
                c.effacer_preference_tableau().await,
                Err(ApiError::Interdit(_))
            ),
            "{nom} : effacer"
        );
        assert!(
            matches!(
                c.regler_heure_bascule(h(10, 0)).await,
                Err(ApiError::Interdit(_))
            ),
            "{nom} : heure"
        );
    }
    // La vie scolaire lit, mais ne règle pas l'heure de l'établissement.
    assert!(cpe.config_tableau().await.is_ok());
    assert!(matches!(
        cpe.regler_heure_bascule(h(10, 0)).await,
        Err(ApiError::Interdit(_))
    ));
    // Hors bornes (6 h à 18 h) : refusé par le serveur.
    for hh in [h(5, 59), h(18, 1)] {
        assert!(
            matches!(
                dir.regler_heure_bascule(hh).await,
                Err(ApiError::Invalide(_))
            ),
            "{hh}"
        );
    }
}

/// Réglage de l'heure de bascule et disposition personnelle : aller-retour par le client typé, séparation entre
/// utilisateurs, et valeurs refusées.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tableau_heure_de_bascule_et_disposition_personnelle() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let (cpe, dir) = (
        personnel(&base, &l, "cpe").await,
        personnel(&base, &l, "direction").await,
    );
    // État initial rendu propre (rejouable).
    cpe.effacer_preference_tableau().await.unwrap();
    dir.effacer_preference_tableau().await.unwrap();
    dir.regler_heure_bascule(h(11, 0)).await.unwrap();

    let c = cpe.config_tableau().await.unwrap();
    assert_eq!(
        (c.heure_bascule, c.preference),
        (h(11, 0), None),
        "défaut : 11 h, automatique"
    );

    // Heure réglable par l'établissement ; la vie scolaire la voit.
    assert_eq!(
        dir.regler_heure_bascule(h(10, 30)).await.unwrap(),
        h(10, 30)
    );
    assert_eq!(cpe.config_tableau().await.unwrap().heure_bascule, h(10, 30));
    // … et la disposition de l'écran suit (10 h 45 : on est après la bascule réglée à 10 h 30).
    assert_eq!(moment(h(10, 45), h(10, 30)), Moment::ApresBascule);

    // Disposition épinglée : ordre, repli, masquage conservés ; la direction ne voit pas celle de la CPE.
    let voulue = ordre(["exclus", "appels", "absents", "familles", "incoherences"]);
    assert_eq!(
        cpe.poser_preference_tableau(voulue.clone()).await.unwrap(),
        voulue
    );
    assert_eq!(
        cpe.config_tableau().await.unwrap().preference,
        Some(voulue.clone())
    );
    assert_eq!(
        dir.config_tableau().await.unwrap().preference,
        None,
        "par utilisateur"
    );
    // Le modèle d'écran la restitue telle quelle, à toute heure.
    let d = disposition(Moment::AvantBascule, Some(&voulue));
    assert_eq!(d[0].panneau, Panneau::Exclus);
    assert_eq!(vers_preference(&d), voulue);

    // Valeurs refusées : un panneau manquant, un doublon, le bandeau Urgent (qui n'est pas un panneau).
    for mauvaise in [
        voulue[..4].to_vec(),
        ordre(["exclus", "exclus", "appels", "absents", "familles"]),
        ordre(["urgent", "appels", "absents", "familles", "incoherences"]),
    ] {
        assert!(
            matches!(
                cpe.poser_preference_tableau(mauvaise).await,
                Err(ApiError::Invalide(_))
            ),
            "disposition invalide refusée"
        );
    }
    assert_eq!(
        cpe.config_tableau().await.unwrap().preference,
        Some(voulue),
        "un refus n'écrit rien"
    );

    // Retour à « automatique » (idempotent) et état rendu.
    cpe.effacer_preference_tableau().await.unwrap();
    cpe.effacer_preference_tableau().await.unwrap();
    assert_eq!(cpe.config_tableau().await.unwrap().preference, None);
    dir.regler_heure_bascule(h(11, 0)).await.unwrap();
}

/// Exclus du jour : l'élève exclu par le professeur apparaît dans le panneau, y reste tant que son arrivée n'est
/// pas confirmée, puis en sort. Le JSON réel de la file du matin se range sans perte dans les panneaux.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tableau_exclus_du_jour_et_file_du_matin() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let harry = l.eleve("Harry").expect("Harry").id;
    let (cpe, prof) = (
        personnel(&base, &l, "cpe").await,
        personnel(&base, &l, "prof").await,
    );
    // Jour de l'établissement lu auprès du serveur (le même que celui du panneau « exclus du jour », celui de
    // `ent_aujourdhui_etab`) : ni l'heure réelle ni le jour de Paris ne sont supposés (entre 23 h et minuit UTC,
    // les deux peuvent différer du jour de l'établissement).
    let jour = cpe
        .portail_aujourdhui()
        .await
        .expect("jour de l'établissement")
        .jour;

    let c = prof
        .mes_creneaux(jour)
        .await
        .expect("créneaux")
        .into_iter()
        .find(|c| c.classe == "4B")
        .expect("séance 4B du jour");
    let ouvert = prof
        .ouvrir_appel(c.id, jour)
        .await
        .expect("ouvrir l'appel du jour");
    let _: serde_json::Value = prof
        .post_json(
            &format!("/appels/{}/exclusions", ouvert.id),
            json!({ "eleve_id": harry }),
        )
        .await
        .expect("exclusion de cours");

    let x = cpe.exclus_du_jour().await.expect("exclus du jour");
    let e = x
        .exclus
        .iter()
        .find(|e| e.eleve_id == harry)
        .expect("Harry est dans le panneau des exclus");
    assert_eq!(e.appel_id, ouvert.id);
    assert!(!e.nom.is_empty() && e.minutes_depuis >= 0);

    // Le vrai JSON de la file du matin est lu par le client typé et rangé par le modèle d'écran.
    let file = cpe.file_du_matin(None).await.expect("file du matin");
    let r = regrouper(&file, Some(&x));
    assert!(r.compte(Panneau::Exclus) >= 1);
    assert_eq!(r.ignorees, 0, "aucune catégorie inconnue dans le vrai JSON");
    let total =
        r.urgent.len() + r.absents.len() + r.appels.len() + r.incoherences.len() + r.familles.len();
    assert_eq!(
        total,
        file.entrees.len(),
        "chaque ligne de la file est rangée une seule fois"
    );

    // Arrivée confirmée : l'élève quitte le panneau (le serveur répond 204 une seule fois).
    cpe.confirmer_arrivee_vs(ouvert.id, harry)
        .await
        .expect("arrivée confirmée");
    let x = cpe.exclus_du_jour().await.unwrap();
    assert!(
        x.exclus.iter().all(|e| e.eleve_id != harry),
        "plus dans les exclus"
    );
    // Le professeur ne peut pas confirmer l'arrivée (droit `localisation.poser`).
    assert!(matches!(
        prof.confirmer_arrivee_vs(ouvert.id, harry).await,
        Err(ApiError::Interdit(_)) | Err(ApiError::Introuvable(_))
    ));
}
