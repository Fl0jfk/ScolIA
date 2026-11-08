//! Socle 6, lot P2 — invitation, accueil, casquettes, côté HTTP (base locale).
//!
//! * invitation par e-mail : nouvelle identité, passkey obligatoire avant tout accès ;
//! * invitation expirée, annulée, relancée ou déjà utilisée : refusée, réponse uniforme ;
//! * identité existante (parent devenu professeur) : rattachée après confirmation explicite, jamais
//!   en silence, jamais fusionnée ;
//! * professeur-parent : une session = une casquette, aucune union de droits ;
//! * accès réduit avant la date d'arrivée ;
//! * aucun secret dans une réponse HTTP ; cloisonnement par établissement.

use super::tests_l3::{appel, connecter, enroler, etat, inscrire, lien_email, MDP};
use super::*;
use axum::http::StatusCode;
use chrono::{Duration as Jours, Utc};
use ent_db::LaboCast;
use serde_json::{json, Value};
use uuid::Uuid;

const NOUVEAU_MDP: &str = "Un-Mot-De-Passe-Neuf-9!";

async fn pool() -> sqlx::PgPool {
    sqlx::PgPool::connect(&super::tests_l3::url_locale().unwrap())
        .await
        .unwrap()
}

fn adresse(prefixe: &str) -> String {
    format!("p2-{prefixe}-{}@ent.local", Uuid::new_v4().simple())
}

/// Crée un agent avec une nouvelle identité (la direction) et rend son identifiant.
async fn agent_neuf(app: &axum::Router, jeton: &str, email: &str, nom: &str) -> Uuid {
    let (st, b) = appel(
        app,
        "POST",
        "/personnels/agents",
        Some(jeton),
        Some(json!({
            "personne": { "nom": nom, "prenom": "Camille", "email": email },
            "confirmer_nouvelle": true,
            "fonction_code": "enseignant"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    Uuid::parse_str(b["agent_id"].as_str().unwrap()).unwrap()
}

async fn inviter(
    app: &axum::Router,
    jeton: &str,
    agent: Uuid,
    canal: &str,
    profil: &str,
) -> (StatusCode, Value) {
    appel(
        app,
        "POST",
        &format!("/personnels/agents/{agent}/invitations"),
        Some(jeton),
        Some(json!({ "canal": canal, "profil": profil })),
    )
    .await
}

fn jeton_du_lien(lien: &str) -> String {
    lien.split("token=").nth(1).unwrap().to_string()
}

async fn accepter(
    app: &axum::Router,
    jeton: &str,
    session: Option<&str>,
    extra: Value,
) -> (StatusCode, Value) {
    let mut corps = json!({ "token": jeton });
    for (k, v) in extra.as_object().unwrap() {
        corps[k] = v.clone();
    }
    appel(app, "POST", "/invitations/personnel/accepter", session, Some(corps)).await
}

fn ne_contient_pas(v: &Value, secrets: &[&str]) {
    let t = v.to_string();
    for s in secrets {
        assert!(!t.contains(s), "secret dans une réponse : {s}");
    }
}

#[tokio::test]
async fn invitation_nouvelle_identite_passkey_obligatoire_et_aucun_secret() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip P2: ENT_DATABASE_URL");
        return;
    };
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let app = app(state);
    let courriel = adresse("nouveau");
    let agent = agent_neuf(&app, &cast.direction.token, &courriel, "Nouvelle").await;

    // Le secrétariat invite un professeur : 201, aucun jeton, adresse masquée.
    ent.mailer().clear_receipts();
    let (st, inv) = inviter(&app, &cast.administratif.token, agent, "email", "prof").await;
    assert_eq!(st, StatusCode::CREATED, "{inv}");
    assert_eq!(inv["etat"], "en_attente");
    assert_eq!(inv["canal"], "email");
    assert!(inv["email_masque"].as_str().unwrap().contains("***@ent.local"));
    let lien = lien_email(&ent, &courriel, "/invitation-personnel").expect("lien envoyé par e-mail");
    assert_eq!(lien.len(), 96, "jeton hexadécimal de 96 caractères");
    ne_contient_pas(&inv, &[&lien, &courriel]);

    // La base ne garde que l'empreinte : le jeton n'y figure nulle part.
    let p = pool().await;
    let (en_clair,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM ent_agent_invitation WHERE jeton_hash = $1")
            .bind(&lien)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(en_clair, 0, "le jeton n'est pas stocké en clair");
    let (ligne,): (String,) = sqlx::query_as(
        "SELECT to_jsonb(i)::text FROM ent_agent_invitation i WHERE id = $1",
    )
    .bind(Uuid::parse_str(inv["id"].as_str().unwrap()).unwrap())
    .fetch_one(&p)
    .await
    .unwrap();
    assert!(!ligne.contains(&lien), "aucune colonne ne contient le jeton");

    // La liste ne renvoie jamais le jeton.
    let (st, liste) = appel(
        &app,
        "GET",
        &format!("/personnels/agents/{agent}/invitations"),
        Some(&cast.administratif.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    ne_contient_pas(&liste, &[&lien, &courriel]);

    // Hiérarchie : le secrétariat n'invite ni un administrateur, ni une direction, ni l'infirmerie.
    for profil in ["admin", "direction", "infirmerie", "administratif"] {
        let (st, _) = inviter(&app, &cast.administratif.token, agent, "email", profil).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "secrétariat → {profil}");
    }
    // Un professeur n'invite personne ; une famille non plus.
    for jeton in [&cast.prof.token, &cast.parent_lea.token] {
        let (st, _) = inviter(&app, jeton, agent, "email", "prof").await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }

    // Aperçu public.
    let (st, ap) = appel(
        &app,
        "POST",
        "/invitations/personnel/consulter",
        None,
        Some(json!({ "token": lien })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{ap}");
    assert_eq!(ap["identite_existante"], false);
    assert_eq!(ap["profil"], "prof");

    // Acceptation : mot de passe obligatoire et conforme.
    let (st, _) = accepter(&app, &lien, None, json!({})).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, _) = accepter(&app, &lien, None, json!({ "mot_de_passe": "court" })).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, ok) = accepter(&app, &lien, None, json!({ "mot_de_passe": NOUVEAU_MDP })).await;
    assert_eq!(st, StatusCode::OK, "{ok}");
    assert_eq!(ok["resultat"], "compte_cree");
    assert_eq!(ok["passkey_a_enroler"], true);
    ne_contient_pas(&ok, &[&lien, NOUVEAU_MDP, &courriel]);
    assert!(ok.get("token").is_none() && ok.get("session").is_none(), "aucune session ouverte");

    // Usage unique : le même lien est désormais refusé, comme un jeton inconnu.
    let (st, deja) = accepter(&app, &lien, None, json!({ "mot_de_passe": NOUVEAU_MDP })).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, inconnu) = accepter(&app, &"0".repeat(96), None, json!({ "mot_de_passe": NOUVEAU_MDP })).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(deja, inconnu, "même réponse : aucun oracle sur l'état de l'invitation");

    // Connexion : passkey obligatoire pour la casquette personnel, même sans politique de
    // l'établissement. Session d'enrôlement seulement, puis session complète.
    let (jeton, user) = {
        let (st, b) = appel(
            &app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": courriel, "password": NOUVEAU_MDP, "etablissement_id": cast.etablissement_id})),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{b}");
        (
            b["token"].as_str().unwrap().to_string(),
            Uuid::parse_str(b["user_id"].as_str().unwrap()).unwrap(),
        )
    };
    let (_, moi) = appel(&app, "GET", "/auth/me", Some(&jeton), None).await;
    assert_eq!(moi["portee"], "enrolement");
    assert_eq!(moi["role"], "prof");
    assert_eq!(moi["passkey_obligatoire"], true);
    let (st, m) = appel(&app, "GET", "/personnels/annuaire", Some(&jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "{m}");
    let (st, r) = enroler(&app, &jeton, Some(NOUVEAU_MDP), "Clé de test").await;
    assert!(st.is_success(), "{r}");
    let (_, moi) = appel(&app, "GET", "/auth/me", Some(&jeton), None).await;
    assert_ne!(moi["portee"], "enrolement");
    let (st, _) = appel(&app, "GET", "/personnels/annuaire", Some(&jeton), None).await;
    assert_eq!(st, StatusCode::OK);

    // Le compte est bien rattaché à la fiche de l'agent invité : un seul agent, une seule personne.
    let (n_agents,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_agent a JOIN ent_personne p
           ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND p.user_id = $2",
    )
    .bind(cast.etablissement_id)
    .bind(user)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(n_agents, 1);
    // Le journal ne contient ni le jeton, ni l'adresse, ni le mot de passe.
    let lignes: Vec<(String,)> = sqlx::query_as(
        "SELECT payload::text FROM ent_journal_modif WHERE etablissement_id = $1 AND domaine = 'personnels'
            AND action IN ('invitation_emise', 'acces_rattache')",
    )
    .bind(cast.etablissement_id)
    .fetch_all(&p)
    .await
    .unwrap();
    assert!(lignes.len() >= 2);
    for (l,) in &lignes {
        assert!(!l.contains(&lien) && !l.contains(&courriel) && !l.contains(NOUVEAU_MDP), "{l}");
    }
    let (evenements,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1
            AND type IN ('personnel.invite', 'personnel.acces_rattache')
            AND (payload::text LIKE '%' || $2 || '%' OR payload::text LIKE '%' || $3 || '%')",
    )
    .bind(cast.etablissement_id)
    .bind(&lien)
    .bind(&courriel)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(evenements, 0, "aucun secret dans les événements");
    let _ = (MDP, connecter, inscrire);
}

#[tokio::test]
async fn invitation_expiree_annulee_relancee_ou_utilisee_refusee_et_qr() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip P2: ENT_DATABASE_URL");
        return;
    };
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let app = app(state);
    let p = pool().await;
    let dir = cast.direction.token.as_str();
    let refusee = |st: StatusCode| assert_eq!(st, StatusCode::BAD_REQUEST);

    // Expirée.
    let c1 = adresse("expiree");
    let a1 = agent_neuf(&app, dir, &c1, "Expiree").await;
    let (st, i1) = inviter(&app, dir, a1, "email", "prof").await;
    assert_eq!(st, StatusCode::CREATED);
    let t1 = lien_email(&ent, &c1, "/invitation-personnel").unwrap();
    sqlx::query("UPDATE ent_agent_invitation SET cree_le = now() - interval '9 days', expire_le = now() - interval '2 days' WHERE id = $1")
        .bind(Uuid::parse_str(i1["id"].as_str().unwrap()).unwrap())
        .execute(&p)
        .await
        .unwrap();
    refusee(accepter(&app, &t1, None, json!({"mot_de_passe": NOUVEAU_MDP})).await.0);
    refusee(
        appel(&app, "POST", "/invitations/personnel/consulter", None, Some(json!({"token": t1})))
            .await
            .0,
    );
    let (_, liste) = appel(&app, "GET", &format!("/personnels/agents/{a1}/invitations"), Some(dir), None).await;
    assert_eq!(liste[0]["etat"], "expiree");

    // Annulée.
    let c2 = adresse("annulee");
    let a2 = agent_neuf(&app, dir, &c2, "Annulee").await;
    let (_, i2) = inviter(&app, dir, a2, "email", "prof").await;
    let t2 = lien_email(&ent, &c2, "/invitation-personnel").unwrap();
    let (st, ann) = appel(
        &app,
        "POST",
        &format!("/personnels/invitations/{}/annuler", i2["id"].as_str().unwrap()),
        Some(dir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{ann}");
    assert_eq!(ann["etat"], "annulee");
    refusee(accepter(&app, &t2, None, json!({"mot_de_passe": NOUVEAU_MDP})).await.0);

    // Relancée : l'ancien lien cesse de fonctionner, le nouveau marche.
    let c3 = adresse("relance");
    let a3 = agent_neuf(&app, dir, &c3, "Relance").await;
    let (_, i3) = inviter(&app, dir, a3, "email", "prof").await;
    let ancien = lien_email(&ent, &c3, "/invitation-personnel").unwrap();
    ent.mailer().clear_receipts();
    let (st, r) = appel(
        &app,
        "POST",
        &format!("/personnels/invitations/{}/relancer", i3["id"].as_str().unwrap()),
        Some(&cast.administratif.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    let nouveau = lien_email(&ent, &c3, "/invitation-personnel").unwrap();
    assert_ne!(ancien, nouveau);
    refusee(accepter(&app, &ancien, None, json!({"mot_de_passe": NOUVEAU_MDP})).await.0);
    let (st, _) = accepter(&app, &nouveau, None, json!({"mot_de_passe": NOUVEAU_MDP})).await;
    assert_eq!(st, StatusCode::OK);
    // Une invitation utilisée ne s'annule ni ne se relance.
    for chemin in ["annuler", "relancer"] {
        let (st, _) = appel(
            &app,
            "POST",
            &format!("/personnels/invitations/{}/{chemin}", r["id"].as_str().unwrap()),
            Some(dir),
            None,
        )
        .await;
        assert_eq!(st, StatusCode::CONFLICT, "{chemin}");
    }
    // Un agent qui a déjà son accès n'est pas réinvité.
    let (st, _) = inviter(&app, dir, a3, "email", "prof").await;
    assert_eq!(st, StatusCode::CONFLICT);

    // QR code : 10 minutes, lien rendu une seule fois (émission et relance), jamais relu.
    let a4 = agent_neuf(&app, dir, &adresse("qr"), "Guichet").await;
    let (st, qr) = inviter(&app, &cast.administratif.token, a4, "qr", "prof").await;
    assert_eq!(st, StatusCode::CREATED, "{qr}");
    let lien = qr["lien"].as_str().expect("le QR porte son lien");
    let t4 = jeton_du_lien(lien);
    assert!(qr["invitation"]["email_masque"].is_null());
    let cree = chrono::DateTime::parse_from_rfc3339(qr["invitation"]["cree_le"].as_str().unwrap()).unwrap();
    let expire = chrono::DateTime::parse_from_rfc3339(qr["invitation"]["expire_le"].as_str().unwrap()).unwrap();
    assert_eq!((expire - cree).num_minutes(), 10);
    let (_, liste) = appel(&app, "GET", &format!("/personnels/agents/{a4}/invitations"), Some(dir), None).await;
    ne_contient_pas(&liste, &[&t4]);
    // Nouvelle identité par QR : l'adresse est saisie par la personne devant le secrétariat.
    let mail_qr = adresse("qr-saisie");
    let (st, ok) = accepter(
        &app,
        &t4,
        None,
        json!({"mot_de_passe": NOUVEAU_MDP, "email": mail_qr, "nom_affiche": "Camille Guichet"}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{ok}");
    let (st, _) = appel(
        &app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"email": mail_qr, "password": NOUVEAU_MDP, "etablissement_id": cast.etablissement_id})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
}

/// Cas du lot : un parent d'élève devient professeur. Rattachement après confirmation, jamais de
/// fusion silencieuse ; ensuite, une session = une casquette et aucune union de droits.
async fn parent_devenu_professeur(
    app: &axum::Router,
    ent: &std::sync::Arc<PgEnt>,
    cast: &LaboCast,
) -> (String, Uuid) {
    let p = pool().await;
    let etab = cast.etablissement_id;
    let parent = &cast.parent_lea;
    // Sa fiche « personne » (socle 4) : l'agent est créé SUR cette fiche (une seule personne).
    let personne: (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(parent.user_id)
    .fetch_one(&p)
    .await
    .expect("le parent du labo a une fiche personne");
    let (st, b) = appel(
        app,
        "POST",
        "/personnels/agents",
        Some(&cast.direction.token),
        Some(json!({ "personne_id": personne.0, "fonction_code": "enseignant" })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let agent = Uuid::parse_str(b["agent_id"].as_str().unwrap()).unwrap();

    ent.mailer().clear_receipts();
    let (st, inv) = appel(
        app,
        "POST",
        &format!("/personnels/agents/{agent}/invitations"),
        Some(&cast.direction.token),
        Some(json!({ "canal": "email", "profil": "prof", "email": parent.email })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{inv}");
    let lien = lien_email(ent, &parent.email, "/invitation-personnel").expect("lien");
    let (_, ap) = appel(
        app,
        "POST",
        "/invitations/personnel/consulter",
        None,
        Some(json!({ "token": lien })),
    )
    .await;
    assert_eq!(ap["identite_existante"], true);

    // Sans connexion : 401. Connecté en tant qu'un AUTRE compte : 401. Rien n'a changé.
    let (st, _) = accepter(app, &lien, None, json!({ "confirmer_rattachement": true })).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = accepter(
        app,
        &lien,
        Some(&cast.parent_tom.token),
        json!({ "confirmer_rattachement": true }),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    // Connecté avec le bon compte mais SANS confirmation : 409, aucun rattachement.
    let (st, m) = accepter(app, &lien, Some(&parent.token), json!({})).await;
    assert_eq!(st, StatusCode::CONFLICT, "{m}");
    let (role,): (String,) = sqlx::query_as(
        "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(parent.user_id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(role, "parent", "pas de rattachement sans confirmation explicite");
    let (_, en_attente): (StatusCode, Value) = appel(
        app,
        "GET",
        &format!("/personnels/agents/{agent}/invitations"),
        Some(&cast.direction.token),
        None,
    )
    .await;
    assert_eq!(en_attente[0]["etat"], "en_attente");

    // Confirmation explicite : rattachée, jamais fusionnée.
    let (st, ok) = accepter(app, &lien, Some(&parent.token), json!({ "confirmer_rattachement": true })).await;
    assert_eq!(st, StatusCode::OK, "{ok}");
    assert_eq!(ok["resultat"], "identite_rattachee");
    assert_eq!(ok["casquettes"], json!(["personnel", "parent"]));
    ne_contient_pas(&ok, &[&lien]);
    let (role,): (String,) = sqlx::query_as(
        "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(parent.user_id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(role, "prof");
    let (fiches, agents): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2),
                (SELECT count(*) FROM ent_agent a JOIN ent_personne p
                   ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
                  WHERE a.etablissement_id = $1 AND p.user_id = $2)",
    )
    .bind(etab)
    .bind(parent.user_id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!((fiches, agents), (1, 1), "une seule personne, un seul agent");
    // Ses sessions de parent ont été révoquées (le profil a changé).
    assert_eq!(
        appel(app, "GET", "/auth/me", Some(&parent.token), None).await.0,
        StatusCode::UNAUTHORIZED
    );
    // Le lien est consommé.
    let (st, _) = accepter(app, &lien, None, json!({ "confirmer_rattachement": true })).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Connexion → enrôlement obligatoire (casquette personnel) → session complète.
    let (jeton, user) = {
        let (st, b) = appel(
            app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": parent.email, "password": MDP, "etablissement_id": etab})),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{b}");
        (
            b["token"].as_str().unwrap().to_string(),
            Uuid::parse_str(b["user_id"].as_str().unwrap()).unwrap(),
        )
    };
    let (_, moi) = appel(app, "GET", "/auth/me", Some(&jeton), None).await;
    assert_eq!(moi["portee"], "enrolement");
    let (st, _) = appel(app, "POST", "/auth/casquette", Some(&jeton), Some(json!({"casquette": "parent"}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "pas de bascule avant l'enrôlement de la passkey");
    let (st, r) = enroler(app, &jeton, Some(MDP), "Clé de test").await;
    assert!(st.is_success(), "{r}");
    (jeton, user)
}

#[tokio::test]
async fn identite_existante_rattachee_apres_confirmation_et_aucune_union_de_droits() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip P2: ENT_DATABASE_URL");
        return;
    };
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let app = app(state);
    let harry = cast.eleve_lea_id;
    let draco = cast.eleve_tom_id;

    // Ce que voit le parent AVANT de devenir professeur : la référence « parent pur ».
    let routes_famille = [
        "/famille/enfants".to_string(),
        format!("/eleves/{harry}/dossier"),
        format!("/dossier/eleves/{harry}/fiche"),
        format!("/dossier/eleves/{harry}/responsables"),
        // Un autre enfant : refusé à un parent.
        format!("/eleves/{draco}/dossier"),
        format!("/dossier/eleves/{draco}/fiche"),
        // Routes d'équipe : jamais pour un parent.
        "/personnels/annuaire".to_string(),
        "/eleves/search?q=a".to_string(),
    ];
    let mut reference: Vec<(StatusCode, Value)> = Vec::new();
    for r in &routes_famille {
        reference.push(appel(&app, "GET", r, Some(&cast.parent_lea.token), None).await);
    }
    assert_eq!(reference[0].0, StatusCode::OK, "le parent voit ses enfants");
    assert_eq!(reference[1].0, StatusCode::OK);
    assert_ne!(reference[4].0, StatusCode::OK, "pas l'enfant d'un autre");
    assert_eq!(reference[6].0, StatusCode::FORBIDDEN);
    assert_eq!(
        appel(&app, "GET", "/personnels/moi/arrivee", Some(&cast.parent_lea.token), None).await.0,
        StatusCode::FORBIDDEN,
        "un parent pur ne lit pas « préparer mon arrivée »"
    );
    // Absence de Harry : le parent peut la justifier (route famille).
    let p = pool().await;
    let absence: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2 LIMIT 1",
    )
    .bind(cast.etablissement_id)
    .bind(harry)
    .fetch_optional(&p)
    .await
    .unwrap();

    let (jeton, _user) = parent_devenu_professeur(&app, &ent, &cast).await;

    // Casquettes disponibles.
    let (st, c) = appel(&app, "GET", "/auth/casquettes", Some(&jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{c}");
    assert_eq!(c["casquettes"][0]["code"], "principale");
    assert_eq!(c["casquettes"][0]["libelle"], "Espace professeur");
    assert_eq!(c["casquettes"][0]["active"], true);
    assert_eq!(c["casquettes"][1]["code"], "parent");
    assert_eq!(c["casquettes"][1]["active"], false);

    // --- Casquette personnel : un professeur sans service, rien d'un parent. -------------------
    let (_, moi) = appel(&app, "GET", "/auth/me", Some(&jeton), None).await;
    assert_eq!(moi["role"], "prof");
    assert_eq!(moi["casquette"], "principale");
    let (st, _) = appel(&app, "GET", "/personnels/annuaire", Some(&jeton), None).await;
    assert_eq!(st, StatusCode::OK, "droits de professeur");
    let (st, enfants) = appel(&app, "GET", "/famille/enfants", Some(&jeton), None).await;
    assert!(
        st != StatusCode::OK || !enfants.to_string().contains(&harry.to_string()),
        "casquette professeur : l'enfant n'est pas ouvert par le lien de parenté ({st} {enfants})"
    );
    for route in [format!("/dossier/eleves/{harry}/responsables"), format!("/dossier/eleves/{harry}/fiche")] {
        let (st, _) = appel(&app, "GET", &route, Some(&jeton), None).await;
        assert_ne!(st, StatusCode::OK, "{route} : pas d'union des droits du parent et du professeur");
    }
    if let Some((absence_id,)) = absence {
        let (st, _) = appel(
            &app,
            "POST",
            &format!("/absences/{absence_id}/justifier"),
            Some(&jeton),
            Some(json!({"motif_justif": "maladie"})),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "un professeur ne justifie pas, même l'absence de son enfant");
    }

    // --- Bascule vers la casquette parent. -----------------------------------------------------
    let (st, c) = appel(&app, "POST", "/auth/casquette", Some(&jeton), Some(json!({"casquette": "parent"}))).await;
    assert_eq!(st, StatusCode::OK, "{c}");
    assert_eq!(c["casquettes"][1]["active"], true);
    let (_, moi) = appel(&app, "GET", "/auth/me", Some(&jeton), None).await;
    assert_eq!(moi["role"], "parent");
    assert_eq!(moi["casquette"], "parent");
    // Mêmes réponses que le parent pur avant l'invitation : ni plus, ni moins.
    for (i, r) in routes_famille.iter().enumerate() {
        let (st, corps) = appel(&app, "GET", r, Some(&jeton), None).await;
        assert_eq!(st, reference[i].0, "{r} : statut identique à celui d'un parent pur");
        if r == "/famille/enfants" {
            assert_eq!(corps, reference[i].1, "{r} : mêmes enfants que le parent pur");
        }
    }
    // Les routes du personnel lui sont fermées dans cette casquette.
    for r in ["/personnels/annuaire", "/eleves/search?q=a", "/personnels/moi/arrivee"] {
        assert_eq!(appel(&app, "GET", r, Some(&jeton), None).await.0, StatusCode::FORBIDDEN, "{r}");
    }
    let (st, _) = appel(&app, "POST", "/personnels/agents", Some(&jeton), Some(json!({}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Idempotent, et retour à la casquette personnel : droits de professeur de nouveau, plus ceux du parent.
    let (st, _) = appel(&app, "POST", "/auth/casquette", Some(&jeton), Some(json!({"casquette": "parent"}))).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(&app, "POST", "/auth/casquette", Some(&jeton), Some(json!({"casquette": "principale"}))).await;
    assert_eq!(st, StatusCode::OK);
    let (_, moi) = appel(&app, "GET", "/auth/me", Some(&jeton), None).await;
    assert_eq!(moi["role"], "prof");
    assert_eq!(appel(&app, "GET", "/personnels/annuaire", Some(&jeton), None).await.0, StatusCode::OK);
    // Casquette inconnue.
    let (st, _) = appel(&app, "POST", "/auth/casquette", Some(&jeton), Some(json!({"casquette": "admin"}))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Journal : « a consulté en tant que parent » se lit dans le journal des changements de casquette.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND action = 'casquette_changee'",
    )
    .bind(cast.etablissement_id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(n, 2);

    // Casquette parent fermée : la session qui l'endossait est invalidée, elle ne retombe pas
    // silencieusement sur la casquette personnel.
    let (st, _) = appel(&app, "POST", "/auth/casquette", Some(&jeton), Some(json!({"casquette": "parent"}))).await;
    assert_eq!(st, StatusCode::OK);
    sqlx::query("UPDATE ent_membership_casquette SET etat = 'ferme' WHERE etablissement_id = $1 AND casquette = 'parent'")
        .bind(cast.etablissement_id)
        .execute(&p)
        .await
        .unwrap();
    assert_eq!(appel(&app, "GET", "/auth/me", Some(&jeton), None).await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn deux_fiches_pour_un_compte_exigent_la_fusion_pas_un_rattachement_silencieux() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip P2: ENT_DATABASE_URL");
        return;
    };
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let app = app(state);
    let p = pool().await;
    let etab = cast.etablissement_id;
    // La direction crée une NOUVELLE fiche (même adresse que le compte parent) au lieu de réutiliser
    // la fiche du parent : deux fiches pour une seule personne réelle.
    let agent = agent_neuf(&app, &cast.direction.token, &cast.parent_lea.email, "Doublon").await;
    ent.mailer().clear_receipts();
    let (st, inv) = inviter(&app, &cast.direction.token, agent, "email", "prof").await;
    assert_eq!(st, StatusCode::CREATED, "{inv}");
    let lien = lien_email(&ent, &cast.parent_lea.email, "/invitation-personnel").unwrap();
    let (st, m) = accepter(
        &app,
        &lien,
        Some(&cast.parent_lea.token),
        json!({ "confirmer_rattachement": true }),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{m}");
    // Rien n'a bougé : même profil, session toujours valable, invitation toujours en attente.
    let (role,): (String,) = sqlx::query_as(
        "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(cast.parent_lea.user_id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(role, "parent");
    assert_eq!(
        appel(&app, "GET", "/auth/me", Some(&cast.parent_lea.token), None).await.0,
        StatusCode::OK
    );
    let (_, liste) = appel(
        &app,
        "GET",
        &format!("/personnels/agents/{agent}/invitations"),
        Some(&cast.direction.token),
        None,
    )
    .await;
    assert_eq!(liste[0]["etat"], "en_attente");
}

#[tokio::test]
async fn acces_reduit_avant_la_date_d_arrivee_puis_ouverture_complete() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip P2: ENT_DATABASE_URL");
        return;
    };
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let app = app(state);
    let p = pool().await;
    let dir = cast.direction.token.as_str();
    let courriel = adresse("arrivee");
    let agent = agent_neuf(&app, dir, &courriel, "Arrivante").await;
    let dans_30_jours = (Utc::now() + Jours::days(30)).date_naive();
    let (st, e) = appel(
        &app,
        "POST",
        &format!("/personnels/agents/{agent}/engagements"),
        Some(dir),
        Some(json!({ "du": dans_30_jours, "nature": "contractuel", "employeur_type": "etat" })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{e}");
    let (st, _) = inviter(&app, dir, agent, "email", "prof").await;
    assert_eq!(st, StatusCode::CREATED);
    let lien = lien_email(&ent, &courriel, "/invitation-personnel").unwrap();
    let (_, ap) = appel(&app, "POST", "/invitations/personnel/consulter", None, Some(json!({"token": lien}))).await;
    assert_eq!(ap["avant_arrivee"], true);
    let (st, ok) = accepter(&app, &lien, None, json!({"mot_de_passe": NOUVEAU_MDP})).await;
    assert_eq!(st, StatusCode::OK, "{ok}");
    assert_eq!(ok["avant_arrivee"], true);
    let (jeton, _) = {
        let c = connecter_avec(&app, cast.etablissement_id, &courriel, NOUVEAU_MDP).await;
        (c.0, c.1)
    };
    let (st, r) = enroler(&app, &jeton, Some(NOUVEAU_MDP), "Clé").await;
    assert!(st.is_success(), "{r}");

    // Accès réduit : « préparer mon arrivée », collègues, aucune donnée d'élève.
    let (_, moi) = appel(&app, "GET", "/auth/me", Some(&jeton), None).await;
    assert_eq!(moi["portee"], "arrivee");
    let (st, prep) = appel(&app, "GET", "/personnels/moi/arrivee", Some(&jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{prep}");
    assert_eq!(prep["avant_arrivee"], true);
    assert_eq!(prep["arrivee_le"], json!(dans_30_jours));
    assert!(prep["collegues"].as_array().unwrap().len() >= 3);
    ne_contient_pas(&prep, &["telephone", "password"]);
    let (st, annuaire) = appel(&app, "GET", "/personnels/annuaire", Some(&jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    ne_contient_pas(&annuaire, &["telephone"]);
    for route in [
        "/eleves/search?q=a".to_string(),
        format!("/eleves/{}/dossier", cast.eleve_lea_id),
        "/famille/enfants".to_string(),
        "/foyers".to_string(),
    ] {
        let (st, m) = appel(&app, "GET", &route, Some(&jeton), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{route} : {m}");
        assert!(m.to_string().contains("arrivee_pas_commencee") || m == json!({}), "{route}");
    }
    // Le signalement d'arrivée n'a rien à signaler tant que l'engagement n'a pas commencé.
    assert_eq!(ent.signaler_arrivees_du_jour(cast.etablissement_id).await.unwrap(), 0);

    // Le matin de la date d'arrivée : la même session ouvre tout, sans nouvelle connexion.
    sqlx::query(
        "UPDATE ent_agent_engagement SET du = CURRENT_DATE - 1 WHERE etablissement_id = $1 AND agent_id = $2",
    )
    .bind(cast.etablissement_id)
    .bind(agent)
    .execute(&p)
    .await
    .unwrap();
    let (_, moi) = appel(&app, "GET", "/auth/me", Some(&jeton), None).await;
    assert_eq!(moi["portee"], "complete");
    let (st, _) = appel(&app, "GET", "/eleves/search?q=a", Some(&jeton), None).await;
    assert_ne!(st, StatusCode::FORBIDDEN);
    // Signalement : une seule fois (idempotent).
    assert_eq!(ent.signaler_arrivees_du_jour(cast.etablissement_id).await.unwrap(), 1);
    assert_eq!(ent.signaler_arrivees_du_jour(cast.etablissement_id).await.unwrap(), 0);
}

async fn connecter_avec(app: &axum::Router, etab: Uuid, email: &str, mdp: &str) -> (String, Uuid) {
    let (st, b) = appel(
        app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"email": email, "password": mdp, "etablissement_id": etab})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{b}");
    (
        b["token"].as_str().unwrap().to_string(),
        Uuid::parse_str(b["user_id"].as_str().unwrap()).unwrap(),
    )
}

#[tokio::test]
async fn invitations_cloisonnees_par_etablissement_et_lot() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip P2: ENT_DATABASE_URL");
        return;
    };
    let a = state.ent.seed_labo_cast().await.expect("cast A");
    let b = state.ent.seed_labo_cast().await.expect("cast B");
    let app = app(state);
    let courriel = adresse("iso");
    let agent = agent_neuf(&app, &a.direction.token, &courriel, "Isolee").await;
    let (_, inv) = inviter(&app, &a.direction.token, agent, "email", "prof").await;
    let lien = lien_email(&ent, &courriel, "/invitation-personnel").unwrap();
    let id = inv["id"].as_str().unwrap();

    // La direction de B ne voit, n'annule ni ne relance rien chez A (404 uniforme / 403).
    let (st, _) = appel(&app, "GET", &format!("/personnels/agents/{agent}/invitations"), Some(&b.direction.token), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    for chemin in ["annuler", "relancer"] {
        let (st, _) = appel(&app, "POST", &format!("/personnels/invitations/{id}/{chemin}"), Some(&b.direction.token), None).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{chemin}");
    }
    let (st, _) = inviter(&app, &b.direction.token, agent, "email", "prof").await;
    assert_eq!(st, StatusCode::NOT_FOUND, "inviter l'agent d'un autre établissement");
    // Un jeton dont le préfixe d'établissement est falsifié ne trouve rien.
    let falsifie = format!("{}{}", b.etablissement_id.simple(), &lien[32..]);
    assert_eq!(
        appel(&app, "POST", "/invitations/personnel/consulter", None, Some(json!({"token": falsifie}))).await.0,
        StatusCode::BAD_REQUEST
    );
    // L'invitation de A reste valable.
    assert_eq!(
        appel(&app, "POST", "/invitations/personnel/consulter", None, Some(json!({"token": lien}))).await.0,
        StatusCode::OK
    );

    // Lot : aperçu sans écriture, adresses manquantes listées, un refus n'arrête pas le lot.
    let avec = agent_neuf(&app, &a.direction.token, &adresse("lot1"), "Lot Un").await;
    let (st, sans) = appel(
        &app,
        "POST",
        "/personnels/agents",
        Some(&a.direction.token),
        Some(json!({ "personne": { "nom": "Sans Adresse" }, "confirmer_nouvelle": true })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    let sans = Uuid::parse_str(sans["agent_id"].as_str().unwrap()).unwrap();
    let inconnu = Uuid::new_v4();
    let corps = json!({ "agent_ids": [avec, sans, inconnu], "profil": "prof", "apercu": true });
    let (st, apercu) = appel(&app, "POST", "/personnels/invitations/lot", Some(&a.administratif.token), Some(corps)).await;
    assert_eq!(st, StatusCode::OK, "{apercu}");
    let res = |v: &Value, id: Uuid| {
        v.as_array().unwrap().iter().find(|l| l["agent_id"] == json!(id)).unwrap()["resultat"].clone()
    };
    assert_eq!(res(&apercu, avec), "a_envoyer");
    assert_eq!(res(&apercu, sans), "adresse_manquante");
    assert_eq!(res(&apercu, inconnu), "introuvable");
    let (_, liste) = appel(&app, "GET", &format!("/personnels/agents/{avec}/invitations"), Some(&a.direction.token), None).await;
    assert_eq!(liste.as_array().unwrap().len(), 0, "l'aperçu n'écrit rien");
    let corps = json!({ "agent_ids": [avec, sans, inconnu], "profil": "prof" });
    let (st, envoi) = appel(&app, "POST", "/personnels/invitations/lot", Some(&a.administratif.token), Some(corps)).await;
    assert_eq!(st, StatusCode::OK, "{envoi}");
    assert_eq!(res(&envoi, avec), "envoyee");
    assert_eq!(res(&envoi, sans), "adresse_manquante");
    let (_, liste) = appel(&app, "GET", &format!("/personnels/agents/{avec}/invitations"), Some(&a.direction.token), None).await;
    assert_eq!(liste.as_array().unwrap().len(), 1);
    // Lot : profil hors hiérarchie refusé en bloc.
    let (st, _) = appel(
        &app,
        "POST",
        "/personnels/invitations/lot",
        Some(&a.administratif.token),
        Some(json!({ "agent_ids": [avec], "profil": "direction" })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}
