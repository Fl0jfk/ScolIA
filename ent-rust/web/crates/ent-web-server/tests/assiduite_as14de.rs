//! Assiduité (socle 12, AS14d et AS14e) contre le **vrai** backend (`ent_api::app`) et une base
//! PostgreSQL **locale** de test : le dossier d'assiduité d'un élève (épisodes, qualification,
//! requalification, décompte expliqué, contacts, rectifications) et le signalement (élèves au seuil,
//! proposition, relecture du dossier, décision, rectification après arrêt du mois), avec les
//! **permissions** : jamais de motif médical hors du cercle autorisé, aucune donnée d'un autre
//! établissement, mêmes réponses pour « interdit » et « inexistant ».
//!
//! Lancement (voir `contre_backend.rs`) :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test assiduite_as14de -- --test-threads=1
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};
use ent_types::assiduite::{DeposerJustificatifBody, PointerLigneBody, QualifierEpisodeBody};
use ent_types::edt_s2::CreneauSaisieDto;
use ent_web_api::{ApiClient, ApiError, Connexion, FiltreEpisodesApi};
use ent_web_app::modules::assiduite_vs::dossier_modele::{
    chronologie, corps_requalification, grouper_par_mois, lignes_dossier_signalement, EtatSeuil,
    SaisieRequalification,
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
    let email = format!("as14de-{role}-{}@ent.local", Uuid::new_v4().simple());
    ApiClient::new(base, None)
        .post_json(
            "/auth/register",
            serde_json::json!({"email": email, "password": MDP, "etablissement_id": etab, "role": role}),
        )
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

/// Ouvre l'appel de 4B le `jour` et, s'il est encore en cours, pointe les `absents` puis valide.
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

/// Un jour futur éloigné, même jour de semaine qu'aujourd'hui (le créneau du seed de labo est posé
/// au jour de semaine de Paris). `k` sépare les tests entre eux.
fn jour_neuf(k: i64) -> NaiveDate {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;
    ent_db::jour_paris() + Duration::weeks(30 + k * 30 + n.rem_euclid(20))
}

async fn episode_du(
    cpe: &ApiClient,
    eleve: Uuid,
    jour: NaiveDate,
) -> ent_types::assiduite::EpisodeDto {
    cpe.episodes(&FiltreEpisodesApi {
        etat: Some("tous"),
        eleve_id: Some(eleve),
        ..Default::default()
    })
    .await
    .expect("épisodes")
    .into_iter()
    .find(|e| e.jour_debut == jour)
    .expect("épisode du jour")
}

fn premier_du_mois(d: NaiveDate) -> NaiveDate {
    d.with_day(1).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pages_dossier_et_signalements_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let eleve = Uuid::nil();
    for (chemin, titre) in [
        (format!("/web/assiduite/eleves/{eleve}"), "Dossier d"),
        ("/web/signalements".to_string(), "Signalements d"),
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

/// AS14d : le dossier d'un élève contre le vrai backend, du pointage à la requalification tracée, et
/// toutes les gardes (professeur, famille, secrétariat sur le médical, autre établissement).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn as14d_dossier_qualification_requalification_et_droits() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let harry = l.eleve("Harry").expect("Harry").id;
    let jour = jour_neuf(1);
    let prof = personnel(&base, &l, "prof").await;
    let cpe = personnel(&base, &l, "cpe").await;
    let secretariat = personnel(&base, &l, "administratif").await;
    let lily = parent_de(&base, &l, "Harry").await;
    let narcissa = parent_de(&base, &l, "Draco").await;

    appel_fait(&prof, jour, &[harry]).await;
    let ep = episode_du(&cpe, harry, jour).await;

    // En-tête du dossier : identité et classe par la fiche élève.
    let fiche = cpe.fiche_eleve(harry).await.expect("fiche élève");
    assert!(fiche.prenom.contains("Harry"));

    // Décompte expliqué : chaque demi-journée porte sa raison et une phrase tirée des champs codés,
    // jamais le motif. Regroupement par mois et état par rapport au seuil, côté écran.
    let regle = cpe.regle_assiduite().await.expect("règle");
    let demi = cpe
        .demi_journees_eleve(harry, jour - Duration::days(3), jour + Duration::days(3))
        .await
        .expect("demi-journées");
    assert!(
        !demi.is_empty(),
        "l'absence de Harry produit au moins une demi-journée lue"
    );
    for d in &demi {
        assert!(!d.raison.is_empty(), "raison codée de la ligne {d:?}");
        assert!(!d.explication.is_empty(), "explication de la ligne {d:?}");
        assert!(
            !d.explication.to_lowercase().contains("médecin"),
            "l'explication ne reprend jamais le motif"
        );
    }
    let mois = grouper_par_mois(&demi, regle.seuil_signalement);
    assert!(mois.iter().any(|m| m.lignes.iter().any(|x| x.jour == jour)));
    assert!(mois
        .iter()
        .all(|m| m.etat == EtatSeuil::de(m.non_justifiees, regle.seuil_signalement)));

    // Famille : donne un motif médical ; seul le cercle autorisé (CPE) le lit.
    lily.deposer_justificatif(
        ep.id,
        &DeposerJustificatifBody {
            texte: Some("Certificat du docteur Rogue, angine".into()),
            medical: Some(true),
            vu_non_conserve: false,
        },
    )
    .await
    .expect("justificatif médical déposé par la famille");
    let vu_cpe = cpe.justificatifs_episode(ep.id).await.expect("CPE");
    assert!(
        vu_cpe.iter().any(|j| j.medical),
        "le CPE lit le justificatif médical"
    );
    let vu_secretariat = secretariat
        .justificatifs_episode(ep.id)
        .await
        .expect("le secrétariat lit la liste non médicale");
    assert!(
        vu_secretariat.iter().all(|j| !j.medical),
        "le secrétariat ne reçoit jamais une pièce médicale : {vu_secretariat:?}"
    );
    let detail_secretariat = secretariat
        .episode(ep.id)
        .await
        .expect("détail secrétariat");
    assert!(
        !format!("{detail_secretariat:?}").contains("angine"),
        "aucun texte médical dans le détail rendu au secrétariat"
    );

    // Qualification puis requalification tracée (motif codé obligatoire, version lue).
    let v1 = cpe.episode(ep.id).await.unwrap().episode.version;
    let q = cpe
        .qualifier_episode(
            ep.id,
            &QualifierEpisodeBody {
                qualification: "sans_motif".into(),
                version: v1,
                nature_code: None,
                motif_id: None,
                libelle_famille_code: None,
                note_interne: Some("pas de réponse".into()),
                proposition_id: None,
            },
        )
        .await
        .expect("qualification");
    assert_eq!(q.qualification, "sans_motif");
    let saisie = SaisieRequalification {
        qualification: "excuse_valable".into(),
        motif_code: "nouvelle_piece".into(),
        ..Default::default()
    };
    // Contrôle local : sans motif codé, rien ne part ; avec une version périmée, le serveur refuse.
    assert!(corps_requalification(
        "sans_motif",
        &SaisieRequalification {
            motif_code: String::new(),
            ..saisie.clone()
        },
        q.version
    )
    .is_err());
    let perime = cpe
        .requalifier_episode(
            ep.id,
            &corps_requalification("sans_motif", &saisie, q.version - 1).unwrap(),
        )
        .await;
    assert!(matches!(perime, Err(ApiError::Conflit(_))), "{perime:?}");
    let hors_liste = cpe
        .requalifier_episode(
            ep.id,
            &ent_types::assiduite::RequalifierEpisodeBody {
                motif_code: "inventé".into(),
                ..corps_requalification("sans_motif", &saisie, q.version).unwrap()
            },
        )
        .await;
    assert!(
        hors_liste.is_err(),
        "un motif de requalification hors liste est refusé"
    );
    let req = cpe
        .requalifier_episode(
            ep.id,
            &corps_requalification("sans_motif", &saisie, q.version).unwrap(),
        )
        .await
        .expect("requalification");
    assert_eq!(req.qualification, "excuse_valable");
    let apres = cpe.episode(ep.id).await.unwrap();
    assert!(
        apres.historique.len() >= 3,
        "ouvert, qualifié, requalifié : l'historique garde tout ({:?})",
        apres.historique
    );
    // Un professeur et la famille ne requalifient pas.
    let corps = corps_requalification(
        "excuse_valable",
        &SaisieRequalification {
            qualification: "non_valable".into(),
            motif_code: "autre".into(),
            ..Default::default()
        },
        req.version,
    )
    .unwrap();
    assert!(prof.requalifier_episode(ep.id, &corps).await.is_err());
    assert!(lily.requalifier_episode(ep.id, &corps).await.is_err());

    // Contacts et courriers : le suivi est réservé à `assiduite.suivre` ; mêmes réponses pour
    // « interdit » et « inexistant ».
    cpe.contacts_eleve(harry).await.expect("contacts");
    cpe.courriers_eleve(harry).await.expect("courriers");
    assert!(prof.contacts_eleve(harry).await.is_err());
    assert!(prof.courriers_eleve(harry).await.is_err());
    assert!(lily.contacts_eleve(harry).await.is_err());
    assert!(narcissa.contacts_eleve(harry).await.is_err());
    let inconnu = narcissa.contacts_eleve(Uuid::new_v4()).await;
    let interdit = narcissa.contacts_eleve(harry).await;
    assert_eq!(
        format!("{inconnu:?}"),
        format!("{interdit:?}"),
        "même réponse pour « interdit » et « inexistant »"
    );

    // Rectifications : lecture réservée à `assiduite.signaler` ; le mois est identifié par son 1er jour.
    cpe.rectifications_mois(premier_du_mois(jour))
        .await
        .expect("rectifications du mois");
    assert!(prof
        .rectifications_mois(premier_du_mois(jour))
        .await
        .is_err());
    assert!(lily
        .rectifications_mois(premier_du_mois(jour))
        .await
        .is_err());
    assert!(cpe
        .rectifications_mois(jour.with_day(2).unwrap())
        .await
        .is_err());
    assert!(cpe.rectification_vue(Uuid::new_v4()).await.is_err());

    // Chronologie : l'absence qualifiée apparaît ; la note interne reste au personnel habilité.
    let eps = cpe
        .episodes(&FiltreEpisodesApi {
            etat: Some("tous"),
            eleve_id: Some(harry),
            ..Default::default()
        })
        .await
        .unwrap();
    let chrono = chronologie(&eps, &[], &[], &[], &[]);
    assert!(!chrono.is_empty());

    // Autre établissement : rien de Harry, ni identité, ni absences, ni décompte, ni suivi.
    let etab_b = Uuid::new_v4();
    let dir_b = inscrire(&base, etab_b, "direction").await;
    assert!(dir_b.fiche_eleve(harry).await.is_err());
    assert!(dir_b
        .demi_journees_eleve(harry, jour - Duration::days(3), jour + Duration::days(3))
        .await
        .is_err());
    assert!(dir_b.contacts_eleve(harry).await.is_err());
    assert!(dir_b.courriers_eleve(harry).await.is_err());
    assert!(dir_b.episode(ep.id).await.is_err());
    assert!(dir_b.justificatifs_episode(ep.id).await.is_err());
    let chez_b = dir_b
        .episodes(&FiltreEpisodesApi {
            etat: Some("tous"),
            eleve_id: Some(harry),
            ..Default::default()
        })
        .await;
    assert!(
        chez_b.map(|v| v.is_empty()).unwrap_or(true),
        "aucun épisode de Harry dans l'autre établissement"
    );
    let rect_b = dir_b.rectifications_mois(premier_du_mois(jour)).await;
    assert!(rect_b.map(|v| v.is_empty()).unwrap_or(true));
}

/// Jour de semaine fixe des appels de ce test (règle 33) : un jour de cours, quel que soit le jour où le test tourne.
/// Le créneau 4B du seed de labo est posé au jour de semaine de « aujourd'hui » (un samedi, il tombe sur un jour sans
/// cours du calendrier et le décompte ne compte rien) : le test crée donc le sien, ce jour-là, par l'administration.
const JOUR_DES_APPELS: Weekday = Weekday::Tue;

/// Crée (ou retrouve) le créneau 4B du mardi 15 h–16 h du professeur du labo, par l'administrateur.
async fn creneau_4b_des_appels(admin: &ApiClient, prof_user_id: Uuid) {
    let h = |heure| NaiveTime::from_hms_opt(heure, 0, 0).expect("heure");
    let jour_semaine = JOUR_DES_APPELS.number_from_monday() as i16;
    let saisie = CreneauSaisieDto {
        classe: "4B".into(),
        groupe_id: None,
        matiere_libelle: "AS14e".into(),
        prof_user_id,
        jour_semaine,
        heure_debut: h(15),
        heure_fin: h(16),
        salle: None,
        site: String::new(),
        semaine_ab: "toutes".into(),
    };
    let existants = admin
        .creneaux_admin(Some("4B"), Some(prof_user_id), None)
        .await
        .expect("créneaux");
    if !existants
        .iter()
        .any(|c| c.matiere_libelle == saisie.matiere_libelle && c.jour_semaine == jour_semaine)
    {
        admin
            .creer_creneau(&saisie, false)
            .await
            .expect("créneau 4B créé");
    }
}

/// Les quatre derniers mardis du mois précédent (voir [`JOUR_DES_APPELS`]), pour avoir un mois **terminé** que la
/// direction peut arrêter.
fn quatre_jours_du_mois_precedent() -> (NaiveDate, [NaiveDate; 4]) {
    let aujourd = ent_db::jour_paris();
    let mois = premier_du_mois(premier_du_mois(aujourd) - Duration::days(1));
    let mut j = mois;
    while j.weekday() != JOUR_DES_APPELS {
        j += Duration::days(1);
    }
    (
        mois,
        [
            j,
            j + Duration::weeks(1),
            j + Duration::weeks(2),
            j + Duration::weeks(3),
        ],
    )
}

/// AS14e : un élève atteint le seuil du mois ; la vie scolaire propose, la direction relit et décide,
/// le mois est arrêté ; une requalification postérieure inscrit une rectification qui touche le
/// signalement ; les droits et le cloisonnement tiennent.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn as14e_seuil_proposition_decision_et_rectification() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let hermione = l.eleve("Hermione").expect("Hermione").id;
    let prof = personnel(&base, &l, "prof").await;
    let cpe = personnel(&base, &l, "cpe").await;
    let direction = personnel(&base, &l, "direction").await;
    let lily = parent_de(&base, &l, "Harry").await;
    let (mois, jours) = quatre_jours_du_mois_precedent();
    let prof_id = l.premier("prof").expect("professeur de labo").user_id;
    let admin = personnel(&base, &l, "admin").await;
    creneau_4b_des_appels(&admin, prof_id).await;

    for j in jours {
        appel_fait(&prof, j, &[hermione]).await;
    }
    let regle = cpe.regle_assiduite().await.expect("règle");
    let decompte = cpe.decompte_mensuel(mois).await.expect("décompte du mois");
    let h = decompte
        .eleves
        .iter()
        .find(|e| e.eleve_id == hermione)
        .expect("Hermione au décompte");
    assert!(
        h.demi_journees_non_justifiees >= i64::from(regle.seuil_signalement.min(4)),
        "quatre absences non justifiées dans le mois : {h:?} (seuil {})",
        regle.seuil_signalement
    );
    assert_eq!(
        h.seuil_atteint,
        h.demi_journees_non_justifiees >= i64::from(decompte.seuil_signalement)
    );

    // Préparation : le CPE propose, la proposition est idempotente et porte le dossier pré-rempli.
    let propose = cpe.proposer_signalements(mois).await.expect("proposition");
    assert!(propose.iter().any(|s| s.eleve_id == hermione));
    let encore = cpe
        .proposer_signalements(mois)
        .await
        .expect("proposition rejouée");
    assert_eq!(
        encore.iter().filter(|s| s.eleve_id == hermione).count(),
        1,
        "un seul signalement par élève et par mois"
    );
    let s = cpe
        .signalements(mois)
        .await
        .expect("signalements du mois")
        .into_iter()
        .find(|s| s.eleve_id == hermione)
        .expect("signalement d'Hermione");
    assert_eq!(s.statut, "propose");
    let lignes = lignes_dossier_signalement(&s);
    assert!(
        lignes.len() >= 4,
        "le dossier pré-rempli liste les demi-journées retenues : {lignes:?}"
    );

    // Décision : la proposition ne se décide pas soi-même sans le droit de la direction ; une
    // décision rejouée ou hors ordre est un conflit ; la famille et le professeur n'y accèdent pas.
    assert!(prof.signalements(mois).await.is_err());
    assert!(lily.signalements(mois).await.is_err());
    assert!(prof.proposer_signalements(mois).await.is_err());
    assert!(lily.decider_signalement(s.id, "valider").await.is_err());
    assert!(
        direction
            .decider_signalement(s.id, "emettre")
            .await
            .is_err(),
        "on ne note pas comme transmis un signalement non validé"
    );
    let valide = direction
        .decider_signalement(s.id, "valider")
        .await
        .expect("validation");
    assert_eq!(valide.statut, "valide");
    assert!(matches!(
        direction.decider_signalement(s.id, "valider").await,
        Err(ApiError::Conflit(_))
    ));
    assert!(direction
        .decider_signalement(s.id, "n'importe quoi")
        .await
        .is_err());

    // Arrêt du mois par la direction (le CPE n'a pas la règle de gestion), puis requalification
    // postérieure : le mois figé ne bouge pas, la différence est une rectification qui touche le
    // signalement.
    assert!(cpe
        .post_json(
            &format!("/assiduite/mois/{mois}/arreter"),
            serde_json::json!({})
        )
        .await
        .is_err());
    direction
        .post_json(
            &format!("/assiduite/mois/{mois}/arreter"),
            serde_json::json!({}),
        )
        .await
        .expect("arrêt du mois");
    let fige = cpe.decompte_mensuel(mois).await.unwrap();
    assert!(fige.arrete, "le mois arrêté est dit arrêté");
    let avant = fige
        .eleves
        .iter()
        .find(|e| e.eleve_id == hermione)
        .unwrap()
        .demi_journees_non_justifiees;
    let ep = episode_du(&cpe, hermione, jours[0]).await;
    let v = cpe.episode(ep.id).await.unwrap().episode;
    let qualifie = if v.etat == "qualifie" {
        v
    } else {
        let e = cpe
            .qualifier_episode(
                ep.id,
                &QualifierEpisodeBody {
                    qualification: "sans_motif".into(),
                    version: v.version,
                    nature_code: None,
                    motif_id: None,
                    libelle_famille_code: None,
                    note_interne: None,
                    proposition_id: None,
                },
            )
            .await
            .expect("qualification");
        e
    };
    let saisie = SaisieRequalification {
        qualification: "excuse_valable".into(),
        motif_code: "nouvelle_piece".into(),
        ..Default::default()
    };
    cpe.requalifier_episode(
        ep.id,
        &corps_requalification(&qualifie.qualification, &saisie, qualifie.version).unwrap(),
    )
    .await
    .expect("requalification après arrêt du mois");
    let apres = cpe.decompte_mensuel(mois).await.unwrap();
    assert_eq!(
        apres
            .eleves
            .iter()
            .find(|e| e.eleve_id == hermione)
            .unwrap()
            .demi_journees_non_justifiees,
        avant,
        "un mois arrêté n'est jamais réécrit"
    );
    let rectifs = cpe.rectifications_mois(mois).await.expect("rectifications");
    let r = rectifs
        .iter()
        .find(|r| r.eleve_id == hermione && r.touche_signalement)
        .expect("la rectification touche le signalement");
    assert!(r.apres_non_justifiees < r.avant_non_justifiees);
    assert!(r.vue_par.is_none());
    cpe.rectification_vue(r.id).await.expect("vue");
    let vue = cpe.rectifications_mois(mois).await.unwrap();
    assert!(vue.iter().any(|x| x.id == r.id && x.vue_par.is_some()));

    // Direction : « transmis » est la trace d'un geste humain, après validation.
    let emis = direction
        .decider_signalement(s.id, "emettre")
        .await
        .expect("noté comme transmis");
    assert_eq!(emis.statut, "emis");

    // Cloisonnement : un autre établissement ne voit ni le décompte, ni le signalement, ni la rectification.
    let dir_b = inscrire(&base, Uuid::new_v4(), "direction").await;
    let chez_b = dir_b.decompte_mensuel(mois).await.expect("décompte de B");
    assert!(chez_b.eleves.iter().all(|e| e.eleve_id != hermione));
    let sig_b = dir_b.signalements(mois).await.expect("signalements de B");
    assert!(sig_b.iter().all(|x| x.eleve_id != hermione));
    assert!(dir_b.decider_signalement(s.id, "valider").await.is_err());
    let rect_b = dir_b
        .rectifications_mois(mois)
        .await
        .expect("rectifications de B");
    assert!(rect_b.iter().all(|x| x.eleve_id != hermione));
    assert!(dir_b.rectification_vue(r.id).await.is_err());
}
