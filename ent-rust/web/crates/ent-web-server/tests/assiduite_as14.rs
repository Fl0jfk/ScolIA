//! Assiduité (socle 12, AS14a à AS14c) contre le **vrai** backend (`ent_api::app`) et une base
//! PostgreSQL **locale** de test : l'appel v2 du professeur (pointage, validation, corrections),
//! la file de la vie scolaire (épisodes, qualification, masse), la famille (voir, donner le motif),
//! l'accueil (situation d'un élève), et les **permissions** (rôles refusés, famille d'un autre élève).
//!
//! Lancement (voir `contre_backend.rs`) :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test assiduite_as14 -- --test-threads=1
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use chrono::{Duration, NaiveDate};
use ent_types::assiduite::{CorrigerLigneBody, DeposerJustificatifBody, PointerLigneBody};
use ent_types::vie_scolaire::DeclarerAbsenceBody;
use ent_web_api::{ApiClient, ApiError, FiltreEpisodesApi};
use ent_web_app::composants_metier::declaration::{
    composer_declaration, Origine, SaisieDeclaration,
};
use ent_web_app::modules::assiduite_vs::modele::{corps_qualification, SaisieQualification};
use ent_web_server::labo::{self, Labo};
use uuid::Uuid;

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

/// Un jour futur à moins de 200 jours, même jour de semaine que le jour de Paris (le créneau du
/// seed de labo est posé au jour de semaine de Paris). `k` sépare les tests entre eux.
fn jour_neuf(k: i64) -> NaiveDate {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;
    ent_db::jour_paris() + Duration::weeks(30 + k * 30 + n.rem_euclid(20))
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

/// Ouvre l'appel de 4B le `jour` et, s'il est encore en cours, pointe les `absents` puis valide.
/// Rend l'identifiant de l'appel (idempotent : un appel déjà validé est repris tel quel).
async fn appel_fait(prof: &ApiClient, jour: NaiveDate, absents: &[Uuid]) -> Uuid {
    let c = prof
        .mes_creneaux(jour)
        .await
        .expect("créneaux")
        .into_iter()
        .find(|c| c.classe == "4B")
        .expect("séance 4B");
    let ouvert = prof.ouvrir_appel(c.id, jour).await.expect("ouvrir");
    let a = prof.appel_v2(ouvert.id).await.expect("lecture v2");
    if a.etat == "en_cours" {
        for e in absents {
            prof.pointer_ligne(
                ouvert.id,
                &PointerLigneBody {
                    eleve_id: *e,
                    etat: "absent".into(),
                    minutes: None,
                    lieu_code: None,
                },
            )
            .await
            .expect("pointage absent");
        }
        prof.valider_appel_v2(ouvert.id, false)
            .await
            .expect("appel fait");
    }
    ouvert.id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pages_assiduite_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    for (chemin, titre) in [
        ("/web/appel", "Faire l"),
        ("/web/assiduite", "Assiduité"),
        ("/web/assiduite/accueil", "Accueil"),
        ("/web/absences", "Absences"),
    ] {
        let r = reqwest::get(format!("{base}{chemin}")).await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(
            r.headers().contains_key("content-security-policy"),
            "CSP {chemin}"
        );
        let html = r.text().await.unwrap();
        assert!(html.contains(titre), "{chemin} : titre « {titre} »");
        assert!(
            !html.contains(" style=\""),
            "{chemin} : aucun style en ligne (CSP)"
        );
    }
}

/// AS14a : appel v2 du professeur — pointage, validation, récapitulatif, corrections et leurs gardes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn as14a_appel_pointage_validation_corrections() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let harry = l.eleve("Harry").expect("Harry").id;
    let hermione = l.eleve("Hermione").expect("Hermione").id;
    let jour = jour_neuf(1);
    let prof = personnel(&base, &l, "prof").await;

    let c = prof
        .mes_creneaux(jour)
        .await
        .unwrap()
        .into_iter()
        .find(|c| c.classe == "4B")
        .expect("séance 4B");
    let ouvert = prof.ouvrir_appel(c.id, jour).await.expect("ouvrir");
    let a = prof.appel_v2(ouvert.id).await.expect("lecture v2");
    assert_eq!(a.etat, "en_cours");
    assert!(a.lignes.iter().any(|x| x.eleve_id == harry));
    assert!(
        a.lignes.iter().all(|x| x.eleve_label.is_some()),
        "chaque ligne porte le nom affiché"
    );

    // Sans minutes, le serveur les calcule depuis le début de la séance (l'écran les demande tout de
    // même) ; avec minutes, elles sont conservées.
    let sans_minutes = prof
        .pointer_ligne(
            ouvert.id,
            &PointerLigneBody {
                eleve_id: hermione,
                etat: "retard".into(),
                minutes: None,
                lieu_code: None,
            },
        )
        .await;
    assert!(
        sans_minutes.is_ok(),
        "retard sans minutes : calculé par le serveur"
    );
    let l1 = prof
        .pointer_ligne(
            ouvert.id,
            &PointerLigneBody {
                eleve_id: hermione,
                etat: "retard".into(),
                minutes: Some(12),
                lieu_code: None,
            },
        )
        .await
        .expect("retard de 12 minutes");
    assert_eq!((l1.etat.as_str(), l1.minutes_retard), ("retard", Some(12)));
    prof.pointer_ligne(
        ouvert.id,
        &PointerLigneBody {
            eleve_id: harry,
            etat: "absent".into(),
            minutes: None,
            lieu_code: None,
        },
    )
    .await
    .expect("absent");
    let fait = prof
        .valider_appel_v2(ouvert.id, false)
        .await
        .expect("appel fait");
    assert_ne!(fait.etat, "en_cours");
    assert!(fait.recapitulatif.absents >= 1 && fait.recapitulatif.retards >= 1);

    // Une ligne validée ne se pointe plus : seule la correction (avec motif) la change.
    let apres = prof
        .pointer_ligne(
            ouvert.id,
            &PointerLigneBody {
                eleve_id: harry,
                etat: "present".into(),
                minutes: None,
                lieu_code: None,
            },
        )
        .await;
    assert!(apres.is_err(), "pointage refusé après l'appel fait");

    // Correction de l'intervenant : jour même seulement. L'appel est dans le futur : refusée.
    let corr = |eleve: Uuid, version: Option<i32>| CorrigerLigneBody {
        eleve_id: eleve,
        etat: "present".into(),
        minutes: None,
        lieu_code: None,
        motif_code: Some("erreur_saisie".into()),
        motif_texte: Some("Mauvaise ligne cochée".into()),
        version_base: version,
    };
    let refus = prof.corriger_ligne(ouvert.id, &corr(harry, None)).await;
    assert!(matches!(refus, Err(ApiError::Interdit(_))), "{refus:?}");

    // La vie scolaire corrige à tout moment, avec motif ; une version périmée est un conflit.
    let cpe = personnel(&base, &l, "cpe").await;
    let lu = cpe.appel_v2(ouvert.id).await;
    let version = fait
        .lignes
        .iter()
        .find(|x| x.eleve_id == harry)
        .map(|x| x.version)
        .unwrap();
    drop(lu);
    let ok = cpe
        .corriger_ligne(ouvert.id, &corr(harry, Some(version)))
        .await
        .expect("correction de la vie scolaire");
    assert_eq!(ok.etat, "present");
    let perime = cpe
        .corriger_ligne(ouvert.id, &corr(harry, Some(version)))
        .await;
    assert!(matches!(perime, Err(ApiError::Conflit(_))), "{perime:?}");
    let histo = prof
        .historique_ligne(ouvert.id, harry)
        .await
        .expect("historique de la ligne");
    assert!(histo.len() >= 2, "pointage puis correction");
    assert!(histo
        .iter()
        .any(|h| h.motif_code.as_deref() == Some("erreur_saisie")));

    // Incohérences : lecture sans erreur.
    prof.incoherences_appel(ouvert.id)
        .await
        .expect("incohérences");

    // Permissions : la famille et l'élève n'ouvrent pas l'appel ; un autre parent ne le lit pas.
    let lily = parent_de(&base, &l, "Harry").await;
    assert!(lily.appel_v2(ouvert.id).await.is_err());
    let narcissa = parent_de(&base, &l, "Draco").await;
    assert!(narcissa.appel_v2(ouvert.id).await.is_err());
    assert!(narcissa
        .pointer_ligne(
            ouvert.id,
            &PointerLigneBody {
                eleve_id: harry,
                etat: "absent".into(),
                minutes: None,
                lieu_code: None
            }
        )
        .await
        .is_err());
}

/// AS14b et AS14c : épisodes d'absence, justificatif de la famille, qualification, masse, avis.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn as14bc_episodes_justificatif_qualification_et_famille() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let harry = l.eleve("Harry").expect("Harry").id;
    let hermione = l.eleve("Hermione").expect("Hermione").id;
    let jour = jour_neuf(2);
    let prof = personnel(&base, &l, "prof").await;
    let cpe = personnel(&base, &l, "cpe").await;
    let lily = parent_de(&base, &l, "Harry").await;
    let narcissa = parent_de(&base, &l, "Draco").await;

    appel_fait(&prof, jour, &[harry, hermione]).await;

    // Vie scolaire : la file « à qualifier » porte les noms et les séances.
    let file = cpe
        .episodes(&FiltreEpisodesApi {
            etat: Some("tous"),
            eleve_id: Some(harry),
            ..Default::default()
        })
        .await
        .expect("épisodes");
    let ep = file
        .iter()
        .find(|e| e.jour_debut == jour)
        .expect("épisode de Harry");
    assert!(ep
        .eleve_label
        .as_deref()
        .is_some_and(|n| n.contains("Harry")));
    assert!(!ep.seances.is_empty());
    let detail = cpe.episode(ep.id).await.expect("détail");
    assert_eq!(detail.episode.id, ep.id);

    // Famille : voit l'absence « à justifier », sans note interne ; donne le motif en un geste.
    let vue = lily.episodes_famille(harry).await.expect("vue famille");
    let v = vue
        .iter()
        .find(|e| e.id == ep.id)
        .expect("épisode vu par la famille");
    assert!(v.a_justifier);
    let motifs = lily.motifs_famille().await.expect("motifs famille");
    assert!(
        motifs.iter().all(|m| m.visible_famille && m.actif),
        "jamais de motif réservé à l'établissement"
    );
    let depot = lily
        .deposer_justificatif(
            ep.id,
            &DeposerJustificatifBody {
                texte: Some("Rendez-vous chez le médecin".into()),
                medical: None,
                vu_non_conserve: false,
            },
        )
        .await
        .expect("motif donné par la famille");
    assert_eq!(depot.presente, "texte");
    // Un texte vide est refusé.
    assert!(lily
        .deposer_justificatif(ep.id, &DeposerJustificatifBody::default())
        .await
        .is_err());

    // Permissions : un autre parent ne voit ni ne justifie l'absence de Harry ; le professeur
    // ne voit ni épisodes ni justificatifs ni avis ; mêmes réponses que pour un identifiant inexistant.
    let inexistant = narcissa.episodes_famille(Uuid::new_v4()).await;
    let autre = narcissa.episodes_famille(harry).await;
    assert!(autre.is_err());
    assert_eq!(
        format!("{inexistant:?}"),
        format!("{autre:?}"),
        "même réponse pour « interdit » et « inexistant » (R-AS35)"
    );
    assert!(narcissa
        .deposer_justificatif(
            ep.id,
            &DeposerJustificatifBody {
                texte: Some("intrus".into()),
                ..Default::default()
            }
        )
        .await
        .is_err());
    assert!(prof.episodes(&FiltreEpisodesApi::default()).await.is_err());
    assert!(prof.episode(ep.id).await.is_err());
    assert!(prof.justificatifs_episode(ep.id).await.is_err());
    assert!(prof.avis_eleve(harry, None).await.is_err());
    assert!(lily.episodes(&FiltreEpisodesApi::default()).await.is_err());
    // Le professeur voit seulement l'état de régularisation de ses séances (R-AS21).
    let miens = prof.mes_episodes(None).await.expect("mes épisodes");
    assert!(miens
        .iter()
        .all(|m| { m.regularisation == "a_regulariser" || m.regularisation == "regularisee" }));

    // La vie scolaire lit le texte déposé, puis qualifie avec la version lue.
    let justifs = cpe
        .justificatifs_episode(ep.id)
        .await
        .expect("justificatifs");
    assert!(justifs
        .iter()
        .any(|j| j.texte.as_deref() == Some("Rendez-vous chez le médecin")));
    let natures = cpe.natures_motif().await.expect("natures");
    let legitime = natures
        .iter()
        .find(|n| n.legitime_l131_8)
        .expect("une nature légitime")
        .code
        .clone();
    // Contrôle local du modèle : motif légitime sans nature refusé avant l'envoi.
    assert!(corps_qualification(
        &SaisieQualification {
            qualification: "reputee_legitime".into(),
            ..Default::default()
        },
        1,
        None
    )
    .is_err());
    let corps = corps_qualification(
        &SaisieQualification {
            qualification: "reputee_legitime".into(),
            nature_code: legitime,
            ..Default::default()
        },
        detail.episode.version,
        None,
    )
    .expect("corps valide");
    let q = cpe
        .qualifier_episode(ep.id, &corps)
        .await
        .expect("qualification");
    assert_eq!(q.etat, "qualifie");
    // Version périmée : conflit (la qualification ne s'écrase pas).
    let encore = cpe.qualifier_episode(ep.id, &corps).await;
    assert!(encore.is_err(), "déjà qualifié ou version périmée");
    // La famille voit le libellé à liste fermée, plus rien à justifier.
    let apres = lily.episodes_famille(harry).await.unwrap();
    let v = apres.iter().find(|e| e.id == ep.id).unwrap();
    assert!(!v.a_justifier);
    assert_eq!(v.libelle, "Absence justifiée");

    // Qualification en masse : aperçu sans écriture, puis confirmation avec l'empreinte.
    let h = file_de(&cpe, hermione, jour).await;
    let apercu = cpe
        .apercu_qualification_masse(
            &ent_web_app::modules::assiduite_vs::modele::corps_masse(&[h], "sans_motif", "", None)
                .unwrap(),
        )
        .await
        .expect("aperçu");
    assert_eq!(apercu.retenus, vec![h]);
    let masse = ent_web_app::modules::assiduite_vs::modele::corps_masse(
        &apercu.retenus,
        "sans_motif",
        "",
        Some(apercu.empreinte.clone()),
    )
    .unwrap();
    // Sans l'empreinte de l'aperçu, la qualification en masse est refusée.
    let mut sans = masse.clone();
    sans.empreinte = None;
    assert!(cpe.qualifier_masse(&sans).await.is_err());
    let bilan = cpe.qualifier_masse(&masse).await.expect("masse");
    assert_eq!(bilan.qualifies, vec![h]);

    // Avis aux familles et relances : lisibles par la vie scolaire (peuvent être vides).
    cpe.avis_eleve(harry, None).await.expect("avis");
    cpe.parametres_famille().await.expect("paramètres");
    cpe.sans_reponse().await.expect("sans réponse");
    // Communications reçues par la famille : sans note ni auteur ; fermées à un autre parent.
    lily.communications_famille(harry)
        .await
        .expect("communications");
    assert!(narcissa.communications_famille(harry).await.is_err());
}

async fn file_de(cpe: &ApiClient, eleve: Uuid, jour: NaiveDate) -> Uuid {
    cpe.episodes(&FiltreEpisodesApi {
        etat: Some("tous"),
        eleve_id: Some(eleve),
        ..Default::default()
    })
    .await
    .expect("épisodes")
    .into_iter()
    .find(|e| e.jour_debut == jour)
    .expect("épisode")
    .id
}

fn declaration(eleve: Uuid, jour: NaiveDate) -> DeclarerAbsenceBody {
    composer_declaration(
        &SaisieDeclaration {
            eleve: Some(eleve),
            debut_jour: jour.to_string(),
            motif: "maladie".into(),
            declarant_nom: "Mme Potter".into(),
            declarant_qualite: "mère".into(),
            ..Default::default()
        },
        Origine::Accueil,
    )
    .expect("saisie valide")
}

/// AS14b, AS-216 : l'accueil voit « prévenu » pour un seul élève à la fois ; les autres rôles sont refusés.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn as14b_accueil_situation_d_un_eleve() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let hermione = l.eleve("Hermione").expect("Hermione").id;
    let ron = l.eleve("Ron").expect("Ron").id;
    let accueil = personnel(&base, &l, "administratif").await;
    let prof = personnel(&base, &l, "prof").await;
    let lily = parent_de(&base, &l, "Harry").await;

    // Avant toute annonce : ni prévenu ni en sortie.
    let avant = accueil
        .situation_accueil(ron)
        .await
        .expect("situation de Ron");
    assert!(!avant.prevenu && !avant.en_sortie);
    assert!(avant
        .eleve_label
        .as_deref()
        .is_some_and(|n| n.contains("Ron")));

    // L'accueil enregistre une absence annoncée pour aujourd'hui : « prévenu ».
    let aujourdhui = ent_db::jour_paris();
    let _ = accueil
        .declarer_absence_accueil(&declaration(hermione, aujourdhui))
        .await;
    let s = accueil
        .situation_accueil(hermione)
        .await
        .expect("situation d'Hermione");
    assert!(s.prevenu, "absence annoncée à l'accueil : prévenu");
    assert_eq!(s.eleve_id, hermione);

    // Permissions : professeur, famille refusés ; un identifiant inconnu ne se distingue pas d'un refus.
    assert!(prof.situation_accueil(hermione).await.is_err());
    assert!(lily.situation_accueil(hermione).await.is_err());
    let inconnu = accueil.situation_accueil(Uuid::new_v4()).await;
    assert!(
        matches!(inconnu, Err(ApiError::Introuvable(_))),
        "{inconnu:?}"
    );
}
