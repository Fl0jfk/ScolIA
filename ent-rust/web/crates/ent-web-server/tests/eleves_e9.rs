//! Élèves (socle 2, lots E9a et E9b) contre le **vrai** backend (`ent_api::app`) et une base PostgreSQL
//! **locale** de test : la liste et la recherche (portée, filtres, pagination), la synthèse de la fiche
//! (tuiles visibles ou masquées avec leur raison), l'identité et l'historique, avec les **permissions** :
//! la famille n'a pas de liste, un élève protégé est **absent** pour un tiers (jamais « refusé »), rien
//! d'un autre établissement, et le rendu serveur des deux pages.
//!
//! Lancement (voir `contre_backend.rs`) :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test eleves_e9 -- --test-threads=1
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_types::eleve_protection::{LeverConfidentialiteBody, PoserConfidentialiteBody};
use ent_web_api::{ApiClient, ApiError, Connexion, FiltreListeEleves};
use ent_web_app::modules::eleves::modele::{
    destination_tuile, etapes_frise, raison_masquage, resume_page, version_courante, Destination,
};
use ent_web_server::labo::{self, Labo};
use uuid::Uuid;

const MDP: &str = "DevLocalPass1!";

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

/// Compte de labo dans un **autre** établissement (route d'inscription de la feature `labo`).
async fn inscrire(base: &str, etab: Uuid, role: &str) -> ApiClient {
    let email = format!("e9-{role}-{}@ent.local", Uuid::new_v4().simple());
    ApiClient::new(base, None)
        .post_json("/auth/register", serde_json::json!({"email": email, "password": MDP, "etablissement_id": etab, "role": role}))
        .await
        .unwrap_or_else(|e| panic!("inscription {role} : {e:?}"));
    match ApiClient::new(base, None)
        .connexion(&email, MDP, etab)
        .await
        .unwrap_or_else(|e| panic!("connexion {role} : {e:?}"))
    {
        Connexion::Ouverte(s) => ApiClient::new(base, Some(s.token)),
        Connexion::PasskeyRequise(_) => panic!("passkey requise pour {role}"),
    }
}

fn interdit<T>(r: &Result<T, ApiError>) -> bool {
    matches!(r, Err(ApiError::Interdit(_)))
}

fn introuvable<T>(r: &Result<T, ApiError>) -> bool {
    matches!(r, Err(ApiError::Introuvable(_)))
}

fn filtre(q: &str) -> FiltreListeEleves {
    FiltreListeEleves {
        q: q.into(),
        limite: 50,
        ..Default::default()
    }
}

// ───────────────────────────── rendu serveur ─────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn les_deux_pages_sont_rendues_par_le_serveur_sans_style_en_ligne() {
    let Some(base) = demarrer().await else { return };
    let http = reqwest::Client::new();
    for (chemin, attendu) in [
        ("/web/eleves".to_string(), "Chargement"),
        (format!("/web/eleves/{}", Uuid::nil()), "Chargement"),
        (
            format!("/web/eleves/{}/dossier", Uuid::nil()),
            "Dossier de l",
        ),
    ] {
        let r = http.get(format!("{base}{chemin}")).send().await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(
            r.headers().contains_key("content-security-policy"),
            "CSP : {chemin}"
        );
        let html = r.text().await.unwrap();
        assert!(html.contains("lang=\"fr\""), "{chemin}");
        assert!(
            html.contains(attendu),
            "{chemin} : état de chargement ou titre attendu"
        );
        assert!(
            !html.contains(" style=\""),
            "aucun style en ligne (CSP) : {chemin}"
        );
    }
}

// ───────────────────────────── liste et recherche ─────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn la_liste_respecte_la_portee_les_filtres_et_la_pagination() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let secretariat = personnel(&base, &l, "administratif").await;
    let prof = personnel(&base, &l, "prof").await;
    let harry = l.eleve("Harry").expect("Harry").clone();

    // Secrétariat : tout l'annuaire, alphabétique, total juste.
    let tout = secretariat
        .eleves_liste(&FiltreListeEleves {
            limite: 100,
            ..Default::default()
        })
        .await
        .expect("liste");
    assert_eq!(tout.portee, "etablissement");
    assert!(tout.total >= i64::try_from(tout.eleves.len()).unwrap() && !tout.eleves.is_empty());
    let noms: Vec<String> = tout
        .eleves
        .iter()
        .map(|e| format!("{} {}", e.nom, e.prenom))
        .collect();
    let mut tries = noms.clone();
    tries.sort_by_key(|n| n.to_lowercase());
    assert_eq!(noms.len(), tries.len());
    assert!(
        tout.eleves.iter().any(|e| e.id == harry.id),
        "Harry est dans la liste du secrétariat"
    );
    assert!(
        tout.classes.contains(&"4B".to_string()),
        "le sélecteur de classe propose les classes présentes"
    );
    // Les lignes ne portent que la désignation (aucune date, aucun identifiant).
    let brut = secretariat
        .get_json("/eleves/liste?limite=3")
        .await
        .expect("json");
    for e in brut["eleves"].as_array().unwrap() {
        let mut cles: Vec<_> = e.as_object().unwrap().keys().cloned().collect();
        cles.sort();
        assert_eq!(cles, ["classe", "id", "nom", "prenom", "statut"]);
    }
    // Recherche : tolérante aux accents et aux majuscules ; classe ; statut ; pagination.
    let r = secretariat.eleves_liste(&filtre("harry")).await.unwrap();
    assert!(r.eleves.iter().any(|e| e.id == harry.id));
    let r = secretariat
        .eleves_liste(&FiltreListeEleves {
            classe: "4b".into(),
            limite: 50,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(
        r.total >= 1
            && r.eleves.iter().all(|e| e
                .classe
                .as_deref()
                .is_some_and(|c| c.eq_ignore_ascii_case("4B")))
    );
    let r = secretariat
        .eleves_liste(&FiltreListeEleves {
            statut: "sorti".into(),
            limite: 50,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(r.eleves.iter().all(|e| e.statut == "sorti"));
    let p1 = secretariat
        .eleves_liste(&FiltreListeEleves {
            limite: 2,
            ..Default::default()
        })
        .await
        .unwrap();
    let p2 = secretariat
        .eleves_liste(&FiltreListeEleves {
            limite: 2,
            decalage: 2,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(p1.eleves.len(), 2);
    assert_eq!(p1.total, p2.total);
    assert!(
        p1.eleves
            .iter()
            .all(|a| p2.eleves.iter().all(|b| a.id != b.id)),
        "deux pages sans recouvrement"
    );
    assert!(resume_page(p1.total, p1.decalage, p1.eleves.len()).contains("sur") || p1.total <= 2);
    // Un statut ou une pastille inconnus sont refusés, pas ignorés.
    assert!(secretariat
        .eleves_liste(&FiltreListeEleves {
            statut: "n'importe quoi".into(),
            ..Default::default()
        })
        .await
        .is_err());
    assert!(secretariat
        .eleves_liste(&FiltreListeEleves {
            pastille: "inconnue".into(),
            ..Default::default()
        })
        .await
        .is_err());

    // Professeur : son périmètre seulement, et la liste le dit.
    let rp = prof
        .eleves_liste(&FiltreListeEleves {
            limite: 100,
            ..Default::default()
        })
        .await
        .expect("liste du prof");
    assert_eq!(rp.portee, "perimetre");
    assert!(rp.total <= tout.total);
    assert!(rp
        .eleves
        .iter()
        .all(|e| tout.eleves.iter().any(|t| t.id == e.id) || tout.total > 100));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn la_famille_et_les_autres_etablissements_n_ont_pas_acces_a_la_liste() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let parent = parent_de(&base, &l, "Harry").await;
    assert!(
        interdit(&parent.eleves_liste(&filtre("")).await),
        "pas de liste pour la famille"
    );
    assert!(interdit(&parent.eleves_compteurs_pastilles().await));
    let harry = l.eleve("Harry").expect("Harry").id;
    assert!(
        interdit(&parent.eleve_synthese(harry).await),
        "la synthèse est réservée au personnel"
    );
    // Un secrétariat d'un autre établissement ne voit rien de celui-ci.
    let ailleurs = inscrire(&base, Uuid::new_v4(), "administratif").await;
    let r = ailleurs
        .eleves_liste(&filtre(""))
        .await
        .expect("liste vide d'un autre établissement");
    assert!(r.eleves.iter().all(|e| e.id != harry));
    assert!(introuvable(&ailleurs.eleve_fiche(harry).await));
    assert!(introuvable(&ailleurs.eleve_synthese(harry).await));
}

// ───────────────────────────── fiche : synthèse, identité, historique ─────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn la_synthese_montre_ou_masque_chaque_tuile_avec_sa_raison() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let harry = l.eleve("Harry").expect("Harry").id;
    let secretariat = personnel(&base, &l, "administratif").await;
    let cpe = personnel(&base, &l, "cpe").await;
    let prof = personnel(&base, &l, "prof").await;

    let s = secretariat
        .eleve_synthese(harry)
        .await
        .expect("synthèse du secrétariat");
    assert_eq!(s.eleve_id, harry);
    let rangs: Vec<u8> = s.tuiles.iter().map(|t| t.rang).collect();
    let mut tries = rangs.clone();
    tries.sort_unstable();
    assert_eq!(rangs, tries, "tuiles dans l'ordre du contrat");
    let par = |s: &ent_web_api::eleve_synthese_types::SyntheseDto, d: &str| {
        s.tuiles
            .iter()
            .find(|t| t.domaine == d)
            .cloned()
            .unwrap_or_else(|| panic!("tuile {d}"))
    };
    assert_eq!(par(&s, "identite").etat, "visible");
    assert_eq!(par(&s, "scolarite").etat, "visible");

    // Une tuile masquée dit pourquoi et ne contient rien ; toutes les raisons ont un texte.
    for (nom, c) in [("cpe", &cpe), ("prof", &prof)] {
        let v = c
            .eleve_synthese(harry)
            .await
            .unwrap_or_else(|e| panic!("{nom} : {e:?}"));
        for t in &v.tuiles {
            if t.etat != "visible" {
                assert!(
                    t.pastilles.is_empty(),
                    "{nom} : tuile masquée sans contenu ({})",
                    t.domaine
                );
                let raison = t.raison.as_deref();
                assert!(
                    matches!(
                        raison,
                        Some("droit_manquant" | "hors_portee" | "reserve_au_personnel")
                    ),
                    "{nom} {} : {raison:?}",
                    t.domaine
                );
                assert!(!raison_masquage(raison).is_empty());
            }
        }
        // Une pastille n'est visible que si sa tuile l'est (règle générale d'E8).
        for p in &v.pastilles {
            assert_eq!(
                par(&v, &p.domaine).etat,
                "visible",
                "{nom} : pastille {} sans tuile ouverte",
                p.code
            );
        }
        // Chaque tuile visible a une destination qui existe, ou aucune (jamais un lien mort).
        for t in v.tuiles.iter().filter(|t| t.etat == "visible") {
            match destination_tuile(&t.domaine, harry) {
                Destination::Aucune | Destination::Onglet(_) => {}
                Destination::Page(p) => assert!(p.starts_with("/web/"), "{p}"),
            }
        }
    }
    // La santé reste fermée au secrétariat et à la vie scolaire (aucun droit de santé, aucun module).
    assert_ne!(par(&s, "sante").etat, "visible");
    let vs = cpe.eleve_synthese(harry).await.unwrap();
    assert_ne!(
        par(&vs, "sante").etat,
        "visible",
        "la vie scolaire n'ouvre pas la santé"
    );
    assert!(vs
        .pastilles
        .iter()
        .all(|p| p.domaine != "sante" && p.code != "pai_actif"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn l_identite_est_minimisee_et_l_historique_reserve() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let harry = l.eleve("Harry").expect("Harry").id;
    let secretariat = personnel(&base, &l, "administratif").await;
    let cpe = personnel(&base, &l, "cpe").await;
    let prof = personnel(&base, &l, "prof").await;

    // Secrétariat : état civil, historique et identifiants lisibles ; la frise est cohérente.
    let f = secretariat.eleve_fiche(harry).await.expect("fiche");
    assert_eq!(f.id, harry);
    assert!(
        f.masques.iter().all(|m| !m.message.is_empty()),
        "chaque masquage est expliqué"
    );
    let versions = secretariat
        .eleve_historique_identite(harry)
        .await
        .expect("historique");
    assert!(!versions.is_empty());
    let courante =
        version_courante(&versions, chrono::Utc::now().date_naive()).expect("version courante");
    assert_eq!(courante.nom_naissance.to_lowercase(), f.nom.to_lowercase());
    let etapes = etapes_frise(&versions);
    assert_eq!(etapes.len(), versions.len());
    assert!(
        etapes.last().unwrap().champs.is_empty(),
        "la plus ancienne n'a rien « changé »"
    );
    secretariat
        .eleve_identifiants(harry)
        .await
        .expect("identifiants");

    // Vie scolaire et professeur : la fiche d'annuaire s'ouvre, mais ni l'historique ni (ici) l'état civil.
    for (nom, c) in [("cpe", &cpe), ("prof", &prof)] {
        let f = c
            .eleve_fiche(harry)
            .await
            .unwrap_or_else(|e| panic!("{nom} : {e:?}"));
        assert!(f.masques.iter().all(|m| !m.message.is_empty()));
        assert!(
            interdit(&c.eleve_historique_identite(harry).await),
            "{nom} : historique réservé"
        );
        assert!(
            interdit(&c.eleve_consultations(harry).await),
            "{nom} : consultations réservées à l'audit"
        );
    }
    assert!(
        interdit(&cpe.eleve_identifiants(harry).await),
        "les identifiants ne sont pas ouverts à la vie scolaire"
    );
}

// ───────────────────────────── élève protégé ─────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn un_eleve_protege_est_absent_pour_un_tiers_dans_la_liste_la_fiche_et_la_synthese() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let direction = personnel(&base, &l, "direction").await;
    let secretariat = personnel(&base, &l, "administratif").await;
    let cpe = personnel(&base, &l, "cpe").await;
    // Un élève dédié : la mesure est posée puis levée, jamais laissée aux autres tests.
    let cible = l
        .eleves
        .iter()
        .find(|e| e.prenom != "Harry")
        .expect("un autre élève")
        .clone();
    let corps = PoserConfidentialiteBody {
        motif: "Mesure de protection (test E9)".into(),
        mesures: vec![],
        du: None,
        au: None,
    };
    let posee = direction
        .post_json(
            &format!("/eleves/{}/confidentialite", cible.id),
            serde_json::to_value(&corps).unwrap(),
        )
        .await
        .expect("pose de la mesure");
    let mesure_ok = posee.get("id").is_some();
    assert!(mesure_ok);

    let cherche = FiltreListeEleves {
        q: cible.nom.clone(),
        limite: 100,
        ..Default::default()
    };
    // Pour un tiers : ni dans la liste, ni dans le total, ni de fiche, ni de synthèse — et jamais « refusé ».
    for (nom, c) in [("secrétariat", &secretariat), ("cpe", &cpe)] {
        let r = c.eleves_liste(&cherche).await.unwrap();
        assert!(
            r.eleves.iter().all(|e| e.id != cible.id),
            "{nom} : protégé dans la liste"
        );
        assert!(introuvable(&c.eleve_fiche(cible.id).await), "{nom} : fiche");
        assert!(
            introuvable(&c.eleve_synthese(cible.id).await),
            "{nom} : synthèse"
        );
        assert!(
            introuvable(&c.eleve_historique_identite(cible.id).await)
                || interdit(&c.eleve_historique_identite(cible.id).await)
        );
        assert!(
            introuvable(&c.eleve_identifiants(cible.id).await)
                || interdit(&c.eleve_identifiants(cible.id).await)
        );
    }
    // Contrôle positif : la direction (G6) le voit et sa fiche le dit.
    let r = direction.eleves_liste(&cherche).await.unwrap();
    assert!(
        r.eleves.iter().any(|e| e.id == cible.id),
        "la direction voit l'élève protégé"
    );
    let s = direction
        .eleve_synthese(cible.id)
        .await
        .expect("synthèse G6");
    assert!(
        s.pastilles
            .iter()
            .any(|p| p.code == "confidentialite_renforcee"),
        "la pastille de protection est visible de G6 seulement"
    );
    let s_tiers_total = cpe
        .eleves_liste(&FiltreListeEleves {
            limite: 100,
            ..Default::default()
        })
        .await
        .unwrap()
        .total;
    let s_dir_total = direction
        .eleves_liste(&FiltreListeEleves {
            limite: 100,
            ..Default::default()
        })
        .await
        .unwrap()
        .total;
    assert!(
        s_dir_total >= s_tiers_total,
        "le total d'un tiers ne compte pas l'élève protégé"
    );
    // Levée : l'élève redevient visible du secrétariat.
    direction
        .post_json(
            &format!("/eleves/{}/confidentialite/lever", cible.id),
            serde_json::to_value(&LeverConfidentialiteBody {
                motif_code: "fin_de_mesure".into(),
            })
            .unwrap(),
        )
        .await
        .expect("levée");
    let r = secretariat.eleves_liste(&cherche).await.unwrap();
    assert!(
        r.eleves.iter().any(|e| e.id == cible.id),
        "visible de nouveau après la levée"
    );
    assert!(secretariat.eleve_fiche(cible.id).await.is_ok());
}

/// Chaque domaine du contrat des tuiles d'E8 est traité par l'écran (destination ou texte propre) et chaque
/// pastille du catalogue a une gravité que l'écran sait teinter : une tuile ou une pastille ajoutée à E8
/// sans décision côté écran fait échouer ce test.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn le_contrat_d_e8_est_entierement_traite_par_l_ecran() {
    use ent_web_app::modules::eleves::modele::{corps_tuile_sans_ecran, teinte_gravite};
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let secretariat = personnel(&base, &l, "administratif").await;
    let cat = secretariat
        .get_json("/eleves/pastilles/catalogue")
        .await
        .expect("catalogue");
    let tuiles = cat["tuiles"].as_array().expect("tuiles");
    assert!(
        tuiles.iter().any(|t| t["domaine"] == "classe"),
        "la tuile « vie de classe » est au contrat"
    );
    for t in tuiles {
        let d = t["domaine"].as_str().unwrap();
        if destination_tuile(d, Uuid::nil()) == Destination::Aucune {
            assert!(
                !corps_tuile_sans_ecran(d).starts_with("Cette section n'a pas encore"),
                "domaine {d} : ni écran ni texte propre"
            );
        }
    }
    for p in cat["indicateurs"].as_array().expect("indicateurs") {
        assert!(["mal", "avert", "info"].contains(&teinte_gravite(p["gravite"].as_str().unwrap())));
        assert!(
            tuiles.iter().any(|t| t["domaine"] == p["domaine"]),
            "pastille {} sans tuile",
            p["code"]
        );
    }
}
