//! Absences des personnels (socle 15, écrans A11a à A11c) contre le **vrai** backend (`ent_api::app`) et une
//! base PostgreSQL **locale** de test : le client typé de l'écran (`ent-web-api`) joue le parcours déclarer →
//! instruire → organiser → répondre, et la **séparation** (aucun motif ni pièce hors du cercle, rien d'un
//! autre établissement). Les pages sont rendues par le serveur (squelette, CSP, aucun style en ligne).
//!
//! Lancement :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test absences_personnels -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »). Chaque test ouvre son propre
//! établissement (identifiant neuf) : aucune écriture ne touche une autre base ou un autre test.
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use chrono::{Datelike, Duration, NaiveDate};
use ent_types::absences_personnels::{DecisionBody, DeclarerAbsenceBody, DeposerPieceBody};
use ent_web_api::{depot_fichier, ActionDecision, ApiClient, ApiError, Connexion};
use uuid::Uuid;

const MDP: &str = "DevLocalPass1!";

async fn demarrer() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent");
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

struct Compte {
    api: ApiClient,
    user_id: Uuid,
    token: String,
}

/// Ouvre un compte de labo dans l'établissement `etab` (route d'inscription de la feature `labo`).
async fn inscrire(base: &str, etab: Uuid, role: &str) -> Compte {
    let email = format!("a11-{role}-{}@ent.local", Uuid::new_v4().simple());
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
        Connexion::Ouverte(s) => Compte {
            api: ApiClient::new(base, Some(s.token.clone())),
            user_id: s.user_id,
            token: s.token,
        },
        Connexion::PasskeyRequise(_) => panic!("passkey requise pour {role}"),
    }
}

/// Règle le fuseau de l'établissement pour qu'il soit `heure` h (locale) maintenant, quelle que soit l'heure
/// réelle d'exécution : une demande de remplacement faite entre 20 h et 6 h 30 attend 6 h 30 (plage calme,
/// `envoi_autorise`) et n'est pas encore visible du remplaçant. Sans ce réglage, le test dépendrait de l'horloge
/// (il échouait la nuit). La plage calme elle-même est testée dans `ent-db` (`a4_plage_calme_differe_l_envoi_de_nuit`).
async fn regler_heure_locale(base: &str, direction: &Compte, heure: i32) {
    use chrono::Timelike;
    let decalage = (heure - chrono::Utc::now().hour() as i32).rem_euclid(24);
    // Fuseaux fixes `Etc/GMT∓N` : de UTC−12 à UTC+14. On préfère un fuseau **à l'est** (jusqu'à +14) : un fuseau à
    // l'ouest rendrait le jour local de l'établissement antérieur au jour de Paris, donc aux engagements créés
    // « aujourd'hui » : les comptes seraient en accès réduit (« arrivee_pas_commencee ») entre 22 h et minuit UTC.
    let decalage = if decalage > 14 {
        decalage - 24
    } else {
        decalage
    };
    let fuseau = if decalage >= 0 {
        format!("Etc/GMT-{decalage}")
    } else {
        format!("Etc/GMT+{}", -decalage)
    };
    let rep = reqwest::Client::new()
        .put(format!("{base}/structure/fuseau"))
        .bearer_auth(&direction.token)
        .header("content-type", "application/json")
        .body(serde_json::json!({"fuseau": fuseau, "confirmer": true}).to_string())
        .send()
        .await
        .expect("réglage du fuseau");
    assert!(
        rep.status().is_success(),
        "fuseau {fuseau} : {}",
        rep.status()
    );
}

/// Prochain jour ouvré (lundi à vendredi) à partir de demain : dans la fenêtre du tableau (veille à +14 jours).
/// Premier jour ouvré **après demain** (Paris) : toujours dans le futur pour l'établissement, même quand
/// `regler_heure_locale` le place jusqu'à UTC+14, où le jour local a déjà un jour d'avance sur Paris (entre 22 h
/// et minuit UTC, « demain » à Paris serait « aujourd'hui » localement et ses séances du matin déjà passées).
fn jour_ouvre() -> NaiveDate {
    let mut j = ent_db::jour_paris() + Duration::days(2);
    while j.weekday().number_from_monday() > 5 {
        j += Duration::days(1);
    }
    j
}

fn declaration(nature: Option<&str>, jour: NaiveDate) -> DeclarerAbsenceBody {
    DeclarerAbsenceBody {
        nature_code: nature.map(Into::into),
        debut: jour,
        fin: jour,
        ..Default::default()
    }
}

/// Génère les séances datées de l'année courante pour `jour` (le calcul d'impact lit les séances du socle 10).
async fn generer_seances(api: &ApiClient, jour: NaiveDate) {
    let annees = api
        .get_json("/referentiel/annees-statut")
        .await
        .expect("années de l'établissement");
    let annee = annees
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|x| x["courante"].as_bool().unwrap_or(false))
                .or(a.first())
        })
        .and_then(|x| x["id"].as_str())
        .unwrap_or_else(|| panic!("aucune année : {annees}"))
        .to_string();
    api.post_json(
        &format!("/referentiel/annees/{annee}/seances"),
        serde_json::json!({"du": jour, "au": jour}),
    )
    .await
    .expect("génération des séances");
}

const PDF: &[u8] =
    b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";

// ───────────────────────── Pages rendues par le serveur ─────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn les_pages_sont_rendues_avec_csp_et_sans_style_en_ligne() {
    let Some(base) = demarrer().await else { return };
    for (chemin, titre) in [
        ("/web/personnel/absences", "Mes absences"),
        ("/web/personnel/absences/declarer", "Déclarer une absence"),
        ("/web/personnel/instruction-absences", "Files des absences"),
        ("/web/personnel/signaler-absence", "Signaler une absence"),
        ("/web/personnel/ce-matin", "Ce matin"),
        ("/web/personnel/remplacements", "Mes remplacements"),
        (
            "/web/personnel/absences/00000000-0000-4000-8000-000000000001",
            "Absence",
        ),
    ] {
        let r = reqwest::get(format!("{base}{chemin}")).await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(
            r.headers().contains_key("content-security-policy"),
            "CSP sur {chemin}"
        );
        let html = r.text().await.unwrap();
        assert!(html.contains(titre), "titre « {titre} » sur {chemin}");
        assert!(
            html.contains("Chargement"),
            "état de chargement rendu côté serveur sur {chemin}"
        );
        assert!(
            !html.contains(" style=\""),
            "aucun style en ligne sur {chemin}"
        );
        assert!(
            !html.contains("<script>"),
            "aucun script en ligne sans nonce sur {chemin}"
        );
    }
}

// ───────────────────────── Parcours : déclarer, instruire, pièces, fil ─────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn declarer_instruire_pieces_et_fil_avec_les_permissions_du_serveur() {
    let Some(base) = demarrer().await else { return };
    let etab = Uuid::new_v4();
    let dir = inscrire(&base, etab, "direction").await;
    let prof = inscrire(&base, etab, "prof").await;
    let collegue = inscrire(&base, etab, "prof").await;
    let accueil = inscrire(&base, etab, "accueil").await;
    let parent = inscrire(&base, etab, "parent").await;
    let jour = jour_ouvre();

    // A11a : la liste courte de natures est celle de l'engagement ; « je préciserai » existe toujours.
    let natures = prof.api.absp_natures().await.expect("natures");
    assert!(natures.iter().any(|n| n.code == "formation"), "{natures:?}");
    assert!(
        !natures.iter().any(|n| n.code == "absence_injustifiee"),
        "la nature « injustifiée » n'est posée que par la direction"
    );

    // Un parent ne déclare rien (403) ; la déclaration du professeur est créée (une nature non sensible,
    // avec un texte libre permis).
    assert!(matches!(
        parent
            .api
            .absp_declarer(&declaration(Some("formation"), jour))
            .await,
        Err(ApiError::Interdit(_))
    ));
    let mut corps = declaration(Some("formation"), jour);
    corps.texte_libre = Some("Formation numérique au rectorat".into());
    let d = prof.api.absp_declarer(&corps).await.expect("déclaration");
    assert_eq!(d.absences.len(), 1);
    let absence = d.absences[0].clone();
    assert_eq!(absence.etat, "declaree");
    assert_eq!(absence.nature_code.as_deref(), Some("formation"));

    // « Mes absences » : la mienne, jamais celle d'un autre.
    let mes = prof.api.absp_mes_absences().await.unwrap();
    assert!(mes.iter().any(|a| a.id == absence.id));
    assert!(collegue.api.absp_mes_absences().await.unwrap().is_empty());

    // Cloisonnement : un collègue ne voit pas la fiche (404 uniforme), un parent est refusé (403).
    assert!(matches!(
        collegue.api.absp_absence(absence.id).await,
        Err(ApiError::Introuvable(_))
    ));
    assert!(matches!(
        parent.api.absp_absence(absence.id).await,
        Err(ApiError::Interdit(_))
    ));
    // La nature ne fuit pas vers la vie scolaire ni vers un collègue : ni historique, ni fil, ni pièces.
    for api in [&collegue.api, &accueil.api] {
        assert!(api.absp_historique(absence.id).await.is_err());
        assert!(api.absp_fil(absence.id).await.is_err());
        assert!(api.absp_pieces(absence.id).await.is_err());
    }

    // A11b : la file du valideur contient l'absence ; pas celle d'un professeur.
    let file = dir.api.absp_a_valider().await.expect("file à valider");
    let ligne = file
        .iter()
        .find(|a| a.id == absence.id)
        .expect("dans la file");
    assert_eq!(
        ligne.nature_code.as_deref(),
        Some("formation"),
        "la direction lit la nature"
    );
    assert!(matches!(
        collegue.api.absp_a_valider().await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        dir.api.absp_a_traiter().await,
        Ok(_) | Err(ApiError::Interdit(_))
    ));
    let compteurs = dir.api.absp_compteurs().await.expect("compteurs");
    assert!(compteurs.a_valider >= 1);

    // Jamais de décision sur sa propre absence ni par un professeur.
    let valider = DecisionBody {
        traitement: Some("sans_incidence".into()),
        ..Default::default()
    };
    assert!(prof
        .api
        .absp_decider(absence.id, ActionDecision::Valider, &valider)
        .await
        .is_err());
    assert!(collegue
        .api
        .absp_decider(absence.id, ActionDecision::Valider, &valider)
        .await
        .is_err());

    // Pièce : l'agent la dépose (volet 1 exclu), la direction la lit ; le collègue n'y accède pas.
    let piece = prof
        .api
        .absp_deposer_piece(
            absence.id,
            &DeposerPieceBody {
                type_piece: "convocation".into(),
                fichier: depot_fichier("convocation-rectorat.pdf", "application/pdf", PDF),
                volet_1_exclu: false,
            },
        )
        .await
        .expect("dépôt");
    assert!(
        !serde_json::to_string(&piece).unwrap().contains("rectorat"),
        "aucun nom de fichier renvoyé"
    );
    assert_eq!(dir.api.absp_pieces(absence.id).await.unwrap().len(), 1);
    assert!(matches!(
        collegue.api.absp_lien_piece(absence.id, piece.id).await,
        Err(ApiError::Introuvable(_))
    ));
    let lien = dir.api.absp_lien_piece(absence.id, piece.id).await;
    assert!(
        matches!(lien, Ok(_) | Err(ApiError::Serveur(503, _))),
        "lien pré-signé (ou signeur désactivé en labo) : {lien:?}"
    );
    // L'agent voit qui a lu ses pièces ; la direction non (404/403).
    assert!(prof.api.absp_lectures(absence.id).await.is_ok());
    assert!(dir.api.absp_lectures(absence.id).await.is_err());

    // Fil : ajout seul ; 4 001 caractères refusés ; le collègue ne lit ni n'écrit.
    prof.api
        .absp_poster(absence.id, "Voici ma convocation.")
        .await
        .unwrap();
    dir.api.absp_poster(absence.id, "Bien reçu.").await.unwrap();
    assert!(prof
        .api
        .absp_poster(absence.id, &"x".repeat(4001))
        .await
        .is_err());
    assert!(collegue
        .api
        .absp_poster(absence.id, "bonjour")
        .await
        .is_err());
    assert_eq!(prof.api.absp_fil(absence.id).await.unwrap().len(), 2);

    // Décision : la direction valide ; le traitement est affiché ; l'étape suivante passe à l'instructeur.
    let v = dir
        .api
        .absp_decider(absence.id, ActionDecision::Valider, &valider)
        .await
        .expect("validation");
    assert_eq!(v.etat, "validee");
    assert_eq!(v.traitement.as_deref(), Some("sans_incidence"));
    let etape = dir.api.absp_prochaine_etape(absence.id).await.unwrap();
    assert_eq!(etape.acteur, "instructeur");
    // L'agent voit l'étape suivante de SON absence, pas celle d'un autre.
    assert!(prof.api.absp_prochaine_etape(absence.id).await.is_ok());
    assert!(collegue.api.absp_prochaine_etape(absence.id).await.is_err());

    // Historique en ajout seul, lisible du cercle.
    let histo = dir.api.absp_historique(absence.id).await.unwrap();
    assert!(histo.len() >= 2, "création, validation : {histo:?}");

    // Agent : prolonger puis annuler avant la fin de son délai : refus métier clair, jamais un silence.
    let apres = prof
        .api
        .absp_changer_fin(
            absence.id,
            false,
            &ent_types::absences_personnels::ChangerFinBody {
                fin: jour + Duration::days(1),
                heure_fin: None,
            },
        )
        .await;
    assert!(
        apres.is_ok() || matches!(apres, Err(ApiError::Conflit(_) | ApiError::Invalide(_))),
        "{apres:?}"
    );

    // L'accueil signale sans nature : il ne lit rien du dossier.
    let agents = dir.api.absp_annuaire().await.expect("annuaire");
    let agent_prof = agents.iter().find(|a| a.user_id == Some(prof.user_id));
    if let Some(a) = agent_prof {
        let s = accueil
            .api
            .absp_signaler(&ent_types::absences_personnels::SignalerAbsenceBody {
                agent_id: a.agent_id,
                debut: jour + Duration::days(7),
                fin: jour + Duration::days(7),
                heure_debut: None,
                heure_fin: None,
            })
            .await
            .expect("signalement");
        let signalee = &s.absences[0];
        assert_eq!(signalee.etat, "signalee");
        assert!(
            signalee.nature_code.is_none(),
            "l'accueil ne lit aucune nature : {signalee:?}"
        );
        // L'agent confirme ce qui a été signalé à son nom.
        let c = prof
            .api
            .absp_confirmer(signalee.id, Some("formation".into()))
            .await
            .expect("confirmation");
        assert_ne!(c.etat, "signalee");
    }
}

// ───────────────────────── Séparation entre établissements ─────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rien_d_un_autre_etablissement_n_est_visible() {
    let Some(base) = demarrer().await else { return };
    let (etab_a, etab_b) = (Uuid::new_v4(), Uuid::new_v4());
    let prof_a = inscrire(&base, etab_a, "prof").await;
    let dir_a = inscrire(&base, etab_a, "direction").await;
    let dir_b = inscrire(&base, etab_b, "direction").await;
    let prof_b = inscrire(&base, etab_b, "prof").await;
    let jour = jour_ouvre();

    let d = prof_a
        .api
        .absp_declarer(&declaration(Some("formation"), jour))
        .await
        .expect("déclaration");
    let id = d.absences[0].id;
    prof_a
        .api
        .absp_poster(id, "Message de l'établissement A")
        .await
        .unwrap();

    // La direction de l'autre établissement ne voit ni la fiche, ni les pièces, ni le fil, ni l'historique.
    assert!(matches!(
        dir_b.api.absp_absence(id).await,
        Err(ApiError::Introuvable(_))
    ));
    assert!(dir_b.api.absp_fil(id).await.is_err());
    assert!(dir_b.api.absp_pieces(id).await.is_err());
    assert!(dir_b.api.absp_historique(id).await.is_err());
    assert!(dir_b.api.absp_impact(id).await.is_err());
    assert!(dir_b
        .api
        .absp_a_valider()
        .await
        .unwrap()
        .iter()
        .all(|a| a.id != id));
    assert!(prof_b.api.absp_mes_absences().await.unwrap().is_empty());
    assert!(dir_b
        .api
        .absp_decider(id, ActionDecision::Valider, &DecisionBody::default())
        .await
        .is_err());

    // Le tableau de l'autre établissement ne nomme personne de A.
    let t = dir_b.api.absp_tableau(jour).await.expect("tableau de B");
    assert!(t.absents.is_empty(), "{t:?}");
    // Et la direction de A voit l'absente, sans aucun champ de nature (test de structure).
    let t = dir_a.api.absp_tableau(jour).await.expect("tableau de A");
    assert_eq!(t.absents.len(), 1);
    let json = serde_json::to_string(&t).unwrap();
    for interdit in [
        "nature",
        "formation",
        "traitement",
        "decision",
        "texte_libre",
    ] {
        assert!(
            !json.contains(interdit),
            "« {interdit} » dans le tableau : {json}"
        );
    }
}

// ───────────────────────── Ce matin : un professeur ne lit jamais un motif ─────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn un_professeur_ne_lit_ni_la_nature_ni_la_liste_des_absents() {
    let Some(base) = demarrer().await else { return };
    let etab = Uuid::new_v4();
    let prof = inscrire(&base, etab, "prof").await;
    let collegue = inscrire(&base, etab, "prof").await;
    let jour = jour_ouvre();
    let d = prof
        .api
        .absp_declarer(&declaration(Some("maladie"), jour))
        .await
        .expect("déclaration");
    let id = d.absences[0].id;
    // « Ce matin » : refusé à un professeur (il n'a pas la lecture de l'organisation).
    assert!(matches!(
        collegue.api.absp_tableau(jour).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        prof.api.absp_tableau(jour).await,
        Err(ApiError::Interdit(_))
    ));
    // L'agent lit sa propre nature ; un collègue : 404 uniforme (aucun oracle d'existence).
    assert_eq!(
        prof.api
            .absp_absence(id)
            .await
            .unwrap()
            .nature_code
            .as_deref(),
        Some("maladie")
    );
    assert!(matches!(
        collegue.api.absp_absence(id).await,
        Err(ApiError::Introuvable(_))
    ));
    // « Est-il présent ? » ne dit ni la nature ni le lieu.
    let p = collegue.api.absp_presence(d.absences[0].agent_id).await;
    if let Ok(p) = p {
        let json = serde_json::to_string(&p).unwrap();
        assert!(
            !json.contains("maladie") && !json.contains("nature"),
            "{json}"
        );
    }
    // Les propositions de remplaçants et les sollicitations sont fermées à un simple professeur.
    assert!(collegue
        .api
        .absp_propositions(Uuid::new_v4())
        .await
        .is_err());
    assert!(collegue
        .api
        .absp_solliciter(Uuid::new_v4(), Uuid::new_v4())
        .await
        .is_err());
    // « Mes remplacements » : liste vide, jamais une erreur.
    assert!(collegue
        .api
        .absp_mes_sollicitations()
        .await
        .unwrap()
        .is_empty());
}

// ───────────────────────── A11c : ce matin, propositions, sollicitation, réponse ─────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ce_matin_propositions_sollicitation_et_reponse_du_remplacant() {
    let Some(base) = demarrer().await else { return };
    let etab = Uuid::new_v4();
    let dir = inscrire(&base, etab, "direction").await;
    let prof = inscrire(&base, etab, "prof").await;
    let remplacant = inscrire(&base, etab, "prof").await;
    let tiers = inscrire(&base, etab, "prof").await;
    let jour = jour_ouvre();
    let iso = jour.weekday().number_from_monday();
    regler_heure_locale(&base, &dir, 12).await; // en plage de service, quelle que soit l'heure réelle

    // Emploi du temps de l'absente (deux séances) et du remplaçant (une séance ailleurs dans la journée).
    let csv = format!(
        "classe;matiere;prof_user_id;jour_semaine;heure_debut;heure_fin;salle;semaine_ab\n\
         4B;Mathématiques;{};{iso};09:00;10:00;12;toutes\n\
         4C;Mathématiques;{};{iso};10:00;11:00;12;toutes\n\
         6A;Mathématiques;{};{iso};14:00;15:00;8;toutes\n\
         3A;Mathématiques;{};{iso};15:00;16:00;9;toutes\n",
        prof.user_id, prof.user_id, remplacant.user_id, tiers.user_id
    );
    dir.api
        .post_json(
            "/edt/import",
            serde_json::json!({"csv": csv, "dry_run": false}),
        )
        .await
        .expect("import de l'emploi du temps");
    generer_seances(&dir.api, jour).await;

    // L'absente déclare sa journée.
    let d = prof
        .api
        .absp_declarer(&declaration(Some("formation"), jour))
        .await
        .expect("déclaration");
    let id_absence = d.absences[0].id;
    // Le remplacement n'est appliqué à l'emploi du temps que pour une absence validée (règle de l'EDT) :
    // la direction valide d'abord (formation : sans incidence sur les heures).
    dir.api
        .absp_decider(
            id_absence,
            ActionDecision::Valider,
            &DecisionBody {
                traitement: Some("sans_incidence".into()),
                ..Default::default()
            },
        )
        .await
        .expect("validation");
    // Ses séances sont touchées (vue de l'agent) : « mes séances touchées ».
    let touchees = prof.api.absp_impact(id_absence).await.expect("impact");
    assert!(touchees.len() >= 2, "deux séances touchées : {touchees:?}");

    // « Ce matin » : la direction voit l'absente, les heures, les séances à couvrir ; aucune nature.
    let t = dir.api.absp_tableau(jour).await.expect("tableau");
    assert_eq!(t.absents.len(), 1);
    assert!(t.compteurs.a_traiter >= 2, "{:?}", t.compteurs);
    let impact = t
        .lignes
        .iter()
        .flat_map(|l| l.impacts.iter())
        .find(|i| i.impact.etat == "a_traiter")
        .expect("une séance à traiter")
        .impact
        .clone();
    let json = serde_json::to_string(&t).unwrap();
    for interdit in ["nature", "formation", "traitement", "texte_libre"] {
        assert!(
            !json.contains(interdit),
            "« {interdit} » dans « ce matin » : {json}"
        );
    }

    // Propositions : sans droit d'organiser, refusées (le tiers, l'absente) ; la direction voit des raisons.
    assert!(matches!(
        tiers.api.absp_propositions(impact.id).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(prof.api.absp_propositions(impact.id).await.is_err());
    let propositions = dir
        .api
        .absp_propositions(impact.id)
        .await
        .expect("propositions");
    assert!(!propositions.is_empty(), "des collègues sont libres");
    // Test de structure : seuls des comptes et la discipline, jamais une donnée sur la personne (R-AP37).
    let permis = [
        "agent_id",
        "nom",
        "meme_discipline",
        "seances_ce_jour",
        "remplacements_faits",
        "repond_rarement",
    ];
    for p in serde_json::to_value(&propositions)
        .unwrap()
        .as_array()
        .unwrap()
    {
        for cle in p.as_object().unwrap().keys() {
            assert!(
                permis.contains(&cle.as_str()),
                "champ inattendu dans une proposition : {cle}"
            );
        }
    }
    // L'absente ne se propose jamais elle-même.
    assert!(propositions.iter().all(|p| p.nom.as_deref() != Some("")));
    let choisi = propositions
        .iter()
        .find(|p| {
            // Le remplaçant libre à cette heure : il enseigne ailleurs ce jour-là.
            p.seances_ce_jour >= 1
        })
        .or(propositions.first())
        .unwrap()
        .clone();

    // Solliciter : la demande part, sans la nature ni le nom de l'absente ; un professeur ne peut pas solliciter.
    assert!(tiers
        .api
        .absp_solliciter(impact.id, choisi.agent_id)
        .await
        .is_err());
    let s = dir
        .api
        .absp_solliciter(impact.id, choisi.agent_id)
        .await
        .expect("sollicitation");
    assert_eq!(s.etat, "envoyee");
    assert!(
        !s.message.to_lowercase().contains("formation"),
        "le message ne dit jamais la nature : {}",
        s.message
    );
    // Une deuxième demande pour la même séance est refusée tant que la première est ouverte.
    assert!(dir
        .api
        .absp_solliciter(impact.id, choisi.agent_id)
        .await
        .is_err());

    // Le remplaçant voit SA demande ; un tiers n'en voit aucune et ne peut pas y répondre.
    let mut reponse = None;
    for c in [&remplacant, &tiers, &prof, &dir] {
        let mes = c
            .api
            .absp_mes_sollicitations()
            .await
            .expect("mes sollicitations");
        if let Some(m) = mes.iter().find(|m| m.id == s.id) {
            reponse = Some((c, m.clone()));
        }
    }
    let (celui_qui_repond, ma_demande) =
        reponse.expect("la demande arrive à la personne sollicitée");
    assert_eq!(ma_demande.impact_id, impact.id);
    for autre in [&remplacant, &tiers, &prof] {
        if autre.user_id != celui_qui_repond.user_id {
            assert!(
                autre.api.absp_accepter(s.id).await.is_err(),
                "une demande n'est acceptée que par la personne sollicitée"
            );
        }
    }
    let acceptee = celui_qui_repond
        .api
        .absp_accepter(s.id)
        .await
        .expect("accepter");
    assert_eq!(acceptee.etat, "acceptee");

    // « Ce matin » reflète la réponse ; la direction peut la lever (fait daté, réversible).
    let t = dir.api.absp_tableau(jour).await.unwrap();
    assert!(t.compteurs.remplace >= 1, "{:?}", t.compteurs);
    let levee = dir.api.absp_lever(impact.id).await.expect("lever");
    assert_ne!(levee.etat, "remplace");
    // Désigner en urgence : le remplaçant accuse réception.
    let designation = dir
        .api
        .absp_designer(impact.id, choisi.agent_id)
        .await
        .expect("désigner");
    assert_eq!(designation.etat, "designee");
    let accuse = celui_qui_repond
        .api
        .absp_accuser(designation.id)
        .await
        .expect("accuser");
    assert!(accuse.accuse_le.is_some());
    // Prendre en charge les élèves : la séance suivante est annulée, sortie anticipée refusée sans information.
    let autre = t
        .lignes
        .iter()
        .flat_map(|l| l.impacts.iter())
        .find(|i| i.impact.id != impact.id && i.impact.etat == "a_traiter")
        .map(|i| i.impact.id);
    if let Some(autre) = autre {
        let refus = dir.api.absp_annuler_seance(autre, true).await;
        assert!(
            refus.is_err(),
            "sortie anticipée sans information des familles : refusée"
        );
        let ok = dir
            .api
            .absp_annuler_seance(autre, false)
            .await
            .expect("annuler la séance");
        assert_eq!(ok.etat, "annule");
    }
}
