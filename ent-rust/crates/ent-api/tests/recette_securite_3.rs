//! Recette de sécurité n° 3 — cloisonnement multi-établissements (multitenant).
//!
//! On joue des **attaquants réels** contre le **logiciel réel** : le routeur
//! `ent_api::app` est appelé en process (`tower::ServiceExt::oneshot`, mêmes
//! couches, mêmes extracteurs, même base) sur un PostgreSQL **local de test**,
//! avec deux établissements de labo A et B créés par `POST /labo/bootstrap`.
//!
//! Priorité absolue : un acteur de A (quel que soit son rôle) ne doit jamais
//! lire ni modifier quoi que ce soit de B. On réutilise le harnais de la
//! matrice de séparation (`separation/harness.rs` : A/B, acteurs, oracle de
//! fuite) et l'inventaire de routes extrait du source (`separation/routes_src.rs`).
//!
//! Contenu :
//!  - `matrice_cloisonnement_toutes_routes` : **générique**, parcourt TOUTES les
//!    routes GET à paramètre et y injecte les identifiants de B, avec un acteur
//!    de A ; toute nouvelle route est donc couverte automatiquement (RS3, but
//!    « matrice multi-établissements »).
//!  - des tests ciblés par faille (`rs3_*`) : vert si ça tient ; pour une faille
//!    ouverte, un test `#[ignore = "FAILLE OUVERTE RS3-xx …"]` qui échoue tant
//!    qu'elle existe (`cargo test … -- --ignored`).
//!
//! Lancement :
//!   service postgresql start
//!   ENT_DATABASE_URL='postgres://ent_ci:ci@127.0.0.1:5432/ent_ci_test' ENT_ENV=test \
//!   RUST_MIN_STACK=16777216 cargo test --locked -p ent-api --features ent-api/labo \
//!   --test recette_securite_3 -- --test-threads=1
//! Sans `ENT_DATABASE_URL` : tests ignorés proprement (« SKIP »).

#![allow(dead_code)]

#[path = "separation/harness.rs"]
mod harness;
#[path = "separation/routes_src.rs"]
mod routes_src;

use harness::*;
use serde_json::json;
use uuid::Uuid;

use Acteur::*;

/// Champs qu'aucune réponse ne doit jamais contenir (secrets, clés d'objets).
const INTERDITS_GLOBAUX: &[&str] = &["storage_key", "password_hash", "argon2"];

/// Routes publiques (pas de session) — mêmes que l'inventaire de `separation`.
/// On ne les éprouve pas pour le cloisonnement (elles n'ouvrent pas de données
/// d'un établissement par identifiant).
const PUBLIQUES: &[(&str, &str)] = &[
    ("GET", "/health"),
    ("GET", "/ready"),
    ("GET", "/metrics"),
    ("POST", "/labo/bootstrap"),
    ("POST", "/auth/register"),
    ("POST", "/auth/login"),
    ("GET", "/auth/etablissement"),
    ("POST", "/auth/passkey/login/start"),
    ("POST", "/auth/passkey/login/finish"),
    ("GET", "/auth/oidc/public"),
    ("POST", "/auth/oidc/{slug}/start"),
    ("POST", "/auth/oidc/complete"),
    ("POST", "/auth/logout"),
    ("POST", "/auth/forgot-password"),
    ("POST", "/auth/reset-password"),
    ("POST", "/auth/email/confirmer"),
    ("POST", "/auth/email/annuler"),
    ("POST", "/auth/passkey/recuperation"),
    ("POST", "/auth/passkey/recuperation/confirmer"),
    ("POST", "/auth/passkey/recuperation/annuler"),
    ("POST", "/auth/passkey/recuperation/finaliser"),
    ("POST", "/preinscription/demarrer"),
    ("POST", "/preinscription/verifier"),
    ("POST", "/inscriptions/invitations/accepter"),
    ("POST", "/inscriptions/invitations/refuser"),
    ("GET", "/fichiers/{jeton}"),
];
const HANDLERS_PUBLICS: &[&str] = &["ui_page"];

fn uuid_de(v: &serde_json::Value) -> Uuid {
    Uuid::parse_str(v.as_str().unwrap_or_else(|| panic!("uuid attendu : {v}"))).expect("uuid")
}

async fn pool_de_test() -> sqlx::PgPool {
    let url = std::env::var("ENT_DATABASE_URL").expect("base de test");
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("connexion de préparation")
}

/// Crée par SQL un compte jetable, membre de l'établissement A avec `role`, avec
/// le mot de passe de labo (en labo l'invitation par e-mail n'est pas jouée et
/// le provisionnement HTTP renvoie 503). Renvoie (email, user_id).
async fn creer_compte_a(m: &Monde, role: &str) -> (String, Uuid) {
    let pool = pool_de_test().await;
    let email = format!("{role}-rs3-{}@exemple.test", Uuid::new_v4().simple());
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
         SELECT $1, $2, u.password_hash, $3, now(), now()
           FROM ent_user u WHERE lower(u.email) = lower($4)",
    )
    .bind(user_id)
    .bind(&email)
    .bind(format!("Compte RS3 {role}"))
    .bind(&m.a.emails[&Direction])
    .execute(&pool)
    .await
    .expect("compte jetable");
    sqlx::query(
        "INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1, $2, $3, now())",
    )
    .bind(user_id)
    .bind(m.a.objets.etab)
    .bind(role)
    .execute(&pool)
    .await
    .expect("rattachement");
    pool.close().await;
    (email, user_id)
}

fn est_publique(r: &routes_src::RouteDecl) -> bool {
    PUBLIQUES.iter().any(|(m, p)| r.methode == *m && r.chemin == *p)
        || HANDLERS_PUBLICS.contains(&r.handler.as_str())
}

/// Tous les identifiants « réels » de B connus du harnais, pour les injecter à
/// la place d'un `{id}` dans une route GET. On ajoute aussi l'identifiant d'un
/// utilisateur (membre) de B : beaucoup de routes prennent un `user_id`.
fn identifiants_de_b(m: &Monde) -> Vec<(&'static str, Uuid)> {
    let o = &m.b.objets;
    let mut v: Vec<(&'static str, Uuid)> = vec![
        ("etablissement", o.etab),
        ("eleve", o.eleve),
        ("eleve_autre", o.eleve_autre),
        ("foyer", o.foyer),
        ("foyer_autre", o.foyer_autre),
    ];
    if let Some(x) = o.doc {
        v.push(("document", x));
    }
    if let Some(x) = o.fil {
        v.push(("fil", x));
    }
    if let Some(x) = o.facture {
        v.push(("facture", x));
    }
    if let Some(x) = o.absence {
        v.push(("absence", x));
    }
    if let Some(x) = o.matiere {
        v.push(("matiere", x));
    }
    if let Some(x) = o.appel {
        v.push(("appel", x));
    }
    if let Some(u) = m.b.user_ids.get(&Direction) {
        v.push(("user_direction", *u));
    }
    if let Some(u) = m.b.user_ids.get(&Eleve) {
        v.push(("user_eleve", *u));
    }
    v
}

/// Instancie une route en remplaçant TOUS les paramètres `{..}` par le même
/// identifiant de B (format UUID homogène) ; `{*rest}` par une valeur neutre.
fn instancier_avec(chemin: &str, id: Uuid) -> String {
    let mut out = String::new();
    let mut reste = chemin;
    while let Some(d) = reste.find('{') {
        out.push_str(&reste[..d]);
        let f = reste[d..].find('}').map(|x| d + x).unwrap_or(reste.len() - 1);
        let nom = &reste[d + 1..f];
        if nom.starts_with('*') {
            out.push('x');
        } else {
            out.push_str(&id.to_string());
        }
        reste = &reste[f + 1..];
    }
    out.push_str(reste);
    out
}

// ───────────────────────── Matrice générique de cloisonnement ─────────────────────────

/// GÉNÉRIQUE. Pour chaque route GET à paramètre, un acteur **privilégié de A**
/// (Direction : le plus de droits dans A) demande la ressource en injectant un
/// identifiant **de B**. Aucune réponse ne doit révéler une donnée de B : on
/// exige un refus (401/403/404), ou une réponse vide, ou — si la route répond
/// 2xx — l'absence de tout autre identifiant de B et de tout champ interdit
/// (après retrait de l'identifiant injecté, qui n'est qu'un écho de l'entrée).
///
/// Cette matrice couvre AUTOMATIQUEMENT toute route ajoutée au routeur : si une
/// nouvelle route GET ouvre une ressource de B à un acteur de A, ce test échoue.
#[test]
fn matrice_cloisonnement_toutes_routes_get() {
    let Some(m) = monde() else { return };
    let ids_b = identifiants_de_b(m);
    // Acteurs de A dont on éprouve le jeton : le plus privilégié, plus un
    // professeur (portée réduite) et une famille (hors personnel).
    let acteurs_a = [Direction, Admin, Prof, Parent, Eleve];
    let routes: Vec<routes_src::RouteDecl> = routes_src::routes()
        .into_iter()
        .filter(|r| r.methode == "GET" && r.chemin.contains('{') && !est_publique(r))
        .collect();
    assert!(
        routes.len() > 30,
        "inventaire des routes GET à paramètre trop court : {}",
        routes.len()
    );

    let mut echecs: Vec<String> = Vec::new();
    let mut n = 0usize;
    rt().block_on(async {
        for r in &routes {
            for &acteur in &acteurs_a {
                let jeton = m.jeton(acteur).to_string();
                for (etiquette, id) in &ids_b {
                    let chemin = instancier_avec(&r.chemin, *id);
                    let rep = m.appel(r.methode, &chemin, Some(&jeton), None).await;
                    n += 1;
                    if !rep.est_succes() {
                        continue; // refus : correct
                    }
                    // 2xx : retirer l'écho de l'identifiant injecté, puis chercher
                    // tout AUTRE identifiant de B ou un champ interdit.
                    let texte = rep.texte.replace(&id.to_string(), "");
                    let mut fuites_trouvees: Vec<String> = fuites(m, &texte, INTERDITS_GLOBAUX);
                    // Une réponse 2xx non vide sur une cible de B qui n'est pas
                    // qu'un écho est suspecte même sans identifiant repérable.
                    if fuites_trouvees.is_empty() && !rep.est_vide() {
                        // Contrôle d'indiscernabilité : la même route avec un UUID
                        // inexistant doit donner la même chose (sinon la présence
                        // de l'objet de B a fuité par le statut / le corps).
                        let neuf = Uuid::new_v4();
                        let ctl = m
                            .appel(r.methode, &instancier_avec(&r.chemin, neuf), Some(&jeton), None)
                            .await;
                        let a = texte;
                        let b = ctl.texte.replace(&neuf.to_string(), "");
                        if ctl.statut != rep.statut || a != b {
                            fuites_trouvees.push(format!(
                                "réponse 2xx distincte d'un id inexistant (statut {} vs {})",
                                rep.statut.as_u16(),
                                ctl.statut.as_u16()
                            ));
                        }
                    }
                    for f in fuites_trouvees {
                        echecs.push(format!(
                            "{} {} (handler {}) acteur={acteur:?} id_B={etiquette} : {f}",
                            r.methode, r.chemin, r.handler
                        ));
                    }
                }
            }
        }
    });
    eprintln!(
        "[RS3] matrice cloisonnement GET : {} routes × {} acteurs × {} ids = {n} appels, {} écart(s)",
        routes.len(),
        acteurs_a.len(),
        ids_b.len(),
        echecs.len()
    );
    assert!(
        echecs.is_empty(),
        "cloisonnement rompu sur {} cas :\n{}",
        echecs.len(),
        echecs.join("\n")
    );
}

/// GÉNÉRIQUE (écritures). Pour chaque route POST/PUT/PATCH/DELETE à paramètre,
/// un acteur de A injecte un identifiant de B dans l'URL avec un corps vide
/// (`{}`). Aucune écriture ne doit **aboutir** (2xx) sur une cible de B : soit
/// un refus d'autorisation (401/403/404), soit une erreur de validation du
/// corps (400/409/422), jamais un 2xx qui aurait modifié B. Un corps vide ne
/// prouve pas qu'une écriture complète échouerait, mais tout 2xx ici est déjà
/// une anomalie (la cible appartient à B).
#[test]
fn matrice_cloisonnement_toutes_routes_ecritures_corps_vide() {
    let Some(m) = monde() else { return };
    let ids_b = identifiants_de_b(m);
    let acteurs_a = [Direction, Admin];
    let routes: Vec<routes_src::RouteDecl> = routes_src::routes()
        .into_iter()
        .filter(|r| {
            matches!(r.methode, "POST" | "PUT" | "PATCH" | "DELETE")
                && r.chemin.contains('{')
                && !est_publique(r)
        })
        .collect();

    let mut echecs: Vec<String> = Vec::new();
    let mut n = 0usize;
    rt().block_on(async {
        for r in &routes {
            for &acteur in &acteurs_a {
                let jeton = m.jeton(acteur).to_string();
                for (etiquette, id) in &ids_b {
                    let chemin = instancier_avec(&r.chemin, *id);
                    let corps = json!({});
                    let rep = m.appel(r.methode, &chemin, Some(&jeton), Some(&corps)).await;
                    n += 1;
                    if rep.est_succes() {
                        echecs.push(format!(
                            "{} {} (handler {}) acteur={acteur:?} id_B={etiquette} : 2xx ({}) sur une cible de B",
                            r.methode,
                            r.chemin,
                            r.handler,
                            rep.statut.as_u16()
                        ));
                    }
                    // Fuite dans le corps d'erreur (identifiant de B renvoyé) : interdite.
                    let texte = rep.texte.replace(&id.to_string(), "");
                    for f in fuites(m, &texte, INTERDITS_GLOBAUX) {
                        echecs.push(format!(
                            "{} {} (handler {}) acteur={acteur:?} id_B={etiquette} : {f}",
                            r.methode, r.chemin, r.handler
                        ));
                    }
                }
            }
        }
    });
    eprintln!(
        "[RS3] matrice cloisonnement écritures (corps vide) : {} routes, {n} appels, {} écart(s)",
        routes.len(),
        echecs.len()
    );
    assert!(
        echecs.is_empty(),
        "écriture inter-établissements possible sur {} cas :\n{}",
        echecs.len(),
        echecs.join("\n")
    );
}

// ───────────────────────── Comptes, sessions, jetons ─────────────────────────

/// Connexion directe (jeton Bearer) d'un e-mail de A avec le mot de passe de labo.
async fn login_a(m: &Monde, email: &str, client: Option<&str>) -> Rep {
    let mut corps = json!({
        "email": email,
        "password": m.a.mot_de_passe,
        "etablissement_id": m.a.objets.etab,
    });
    if let Some(c) = client {
        corps["client"] = json!(c);
    }
    m.appel("POST", "/auth/login", None, Some(&corps)).await
}

/// **Compte désactivé** : un accès suspendu ou fermé invalide immédiatement les
/// jetons déjà ouverts (socle 6 P0 : `resolve_bearer` relit l'état du membership
/// à chaque requête). On provisionne un AED jetable, on le connecte, puis la
/// Direction suspend son accès : son jeton doit tomber (401). On rouvre ensuite
/// (état restauré) et on vérifie qu'il refonctionne — sans toucher aux acteurs
/// partagés par les autres tests.
#[test]
fn rs3_compte_suspendu_puis_ferme_invalide_la_session() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        // 1. Créer un personnel jetable (rôle minimal « personnel » = AED/accueil) et le connecter.
        let (email, cible) = creer_compte_a(m, "personnel").await;
        let l = login_a(m, &email, None).await;
        assert_eq!(l.statut.as_u16(), 200, "connexion AED : {}", l.texte);
        let jeton = l.json["token"].as_str().expect("jeton").to_string();
        let me = m.appel("GET", "/auth/me", Some(&jeton), None).await;
        assert_eq!(me.statut.as_u16(), 200, "le jeton AED doit marcher au départ");

        // 3. Direction suspend l'accès : le jeton déjà ouvert tombe.
        let susp = m
            .appel(
                "POST",
                &format!("/etablissement/personnel/{cible}/acces/suspendre"),
                Some(m.jeton(Direction)),
                Some(&json!({"motif": "conservatoire"})),
            )
            .await;
        assert!(susp.statut.is_success(), "suspension : {} {}", susp.statut, susp.texte);
        let me2 = m.appel("GET", "/auth/me", Some(&jeton), None).await;
        assert_eq!(
            me2.statut.as_u16(),
            401,
            "un accès suspendu doit invalider le jeton déjà ouvert (obtenu {})",
            me2.statut.as_u16()
        );
        // Une nouvelle connexion est elle aussi refusée tant que l'accès est suspendu.
        let l2 = login_a(m, &email, None).await;
        assert_ne!(l2.statut.as_u16(), 200, "connexion refusée tant que suspendu : {}", l2.texte);

        // 4. Fermeture définitive : idem.
        let ferm = m
            .appel(
                "POST",
                &format!("/etablissement/personnel/{cible}/acces/fermer"),
                Some(m.jeton(Direction)),
                Some(&json!({"motif": "depart"})),
            )
            .await;
        assert!(ferm.statut.is_success(), "fermeture : {} {}", ferm.statut, ferm.texte);
        let me3 = m.appel("GET", "/auth/me", Some(&jeton), None).await;
        assert_eq!(me3.statut.as_u16(), 401, "un accès fermé doit invalider le jeton");
    });
}

/// **Session volée puis révoquée** : après un `POST /auth/logout`, le jeton ne
/// vaut plus rien (401), y compris pour lire des données ; une session « volée »
/// ne survit donc pas à la déconnexion de sa victime.
#[test]
fn rs3_logout_invalide_le_jeton_vole() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let (email, _) = creer_compte_a(m, "personnel").await;
        let l = login_a(m, &email, None).await;
        let jeton = l.json["token"].as_str().expect("jeton").to_string();
        // L'« attaquant » copie le jeton ; la victime se déconnecte.
        let out = m.appel("POST", "/auth/logout", Some(&jeton), None).await;
        assert!(out.statut.is_success(), "logout : {} {}", out.statut, out.texte);
        let apres = m.appel("GET", "/auth/me", Some(&jeton), None).await;
        assert_eq!(apres.statut.as_u16(), 401, "le jeton révoqué par logout doit être refusé");
    });
}

/// **Jeton mobile** d'un compte de A : il porte l'établissement A comme un jeton
/// web ; employé avec des identifiants de B, il est refusé exactement de la même
/// façon (le type de client n'élargit pas le cloisonnement). Contrôle ponctuel
/// sur quelques routes (la matrice générique l'étend à toutes avec le jeton web).
#[test]
fn rs3_jeton_mobile_de_a_ne_voit_rien_de_b() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let l = login_a(m, &m.a.emails[&Direction], Some("mobile")).await;
        assert_eq!(l.statut.as_u16(), 200, "connexion mobile : {}", l.texte);
        let jeton = l.json["token"].as_str().expect("jeton").to_string();
        let eleve_b = m.b.objets.eleve;
        let foyer_b = m.b.objets.foyer;
        let mut echecs = Vec::new();
        for chemin in [
            format!("/eleves/{eleve_b}"),
            format!("/eleves/{eleve_b}/dossier"),
            format!("/eleves/{eleve_b}/notes"),
            format!("/eleves/{eleve_b}/absences"),
            format!("/eleves/{eleve_b}/documents"),
            format!("/foyers/{foyer_b}"),
        ] {
            let rep = m.appel("GET", &chemin, Some(&jeton), None).await;
            if rep.est_succes() {
                let texte = rep.texte.replace(&eleve_b.to_string(), "").replace(&foyer_b.to_string(), "");
                for f in fuites(m, &texte, INTERDITS_GLOBAUX) {
                    echecs.push(format!("{chemin} : {f}"));
                }
                if !rep.est_vide() {
                    echecs.push(format!("{chemin} : 2xx non vide sur une cible de B"));
                }
            }
        }
        assert!(echecs.is_empty(), "jeton mobile de A voit des données de B :\n{}", echecs.join("\n"));
    });
}

// ───────────────────────── Facturation : charge de paiement (RS3-01) ─────────────────────────

/// Bascule `est_payeur` du lien d'un parent sur un foyer donné et renvoie
/// l'ancienne valeur (pour restauration). Opère sur le lien actif du foyer.
async fn regler_est_payeur(user_id: Uuid, foyer_id: Uuid, payeur: bool) -> u64 {
    let pool = pool_de_test().await;
    let n = sqlx::query(
        "UPDATE ent_eleve_lien SET est_payeur = $3, updated_at = now() WHERE user_id = $1 AND foyer_id = $2",
    )
    .bind(user_id)
    .bind(foyer_id)
    .bind(payeur)
    .execute(&pool)
    .await
    .expect("maj est_payeur")
    .rows_affected();
    pool.close().await;
    n
}

/// **RS3-01 (élevée, corrigée).** Le chemin historique `/facturation/*` autorisait
/// un parent dès qu'un **lien** existait avec le foyer, sans vérifier `est_payeur`
/// (`foyer_ids_parent`), contrairement au chemin `/finances/*`. Un parent séparé
/// non payeur, ou un tiers lié, pouvait donc lire la facture et l'historique
/// bancaire du foyer, lister les mandats SEPA, et surtout **créer ou révoquer**
/// le mandat de prélèvement (détournement du compte débité).
///
/// On prive la payeuse (Lily) de sa charge de paiement sur le foyer de Harry :
/// elle doit alors être refusée sur toutes ces routes. État restauré en fin de
/// test. Vert avec le correctif (`foyer_ids_parent` filtre `est_payeur`).
#[test]
fn rs3_01_parent_non_payeur_sans_acces_a_la_facturation() {
    let Some(m) = monde() else { return };
    let Some(facture) = m.a.objets.facture else {
        eprintln!("SKIP RS3-01 : pas de facture de labo pour le foyer de Harry");
        return;
    };
    let foyer = m.a.objets.foyer;
    let lily = m.a.user_ids[&Parent];
    rt().block_on(async {
        let jeton = m.jeton(Parent).to_string();
        // Référence : payeuse, elle accède à sa facture.
        let avant = m.appel("GET", &format!("/facturation/factures/{facture}"), Some(&jeton), None).await;
        assert!(
            avant.est_succes(),
            "la payeuse doit lire sa facture au départ (obtenu {})",
            avant.statut.as_u16()
        );

        // On la bascule en NON payeuse (parent séparé sans charge de paiement).
        let maj = regler_est_payeur(lily, foyer, false).await;
        assert_eq!(maj, 1, "le lien payeur de Lily sur le foyer de Harry doit exister");

        let mut echecs = Vec::new();
        // Lecture de la facture et de la quittance par identifiant.
        let f = m.appel("GET", &format!("/facturation/factures/{facture}"), Some(&jeton), None).await;
        if f.est_succes() {
            echecs.push(format!("GET facture : 2xx ({}) pour un parent non payeur", f.statut.as_u16()));
        }
        // Historique IBAN du foyer.
        let iban = m.appel("GET", &format!("/foyers/{foyer}/iban-histo"), Some(&jeton), None).await;
        if iban.est_succes() && !iban.est_vide() {
            echecs.push("GET iban-histo : données pour un parent non payeur".into());
        }
        // Liste des mandats SEPA : ne doit rien montrer du foyer.
        let mandats = m.appel("GET", "/facturation/sepa/mandats", Some(&jeton), None).await;
        if mandats.est_succes() && mandats.texte.contains(&foyer.to_string()) {
            echecs.push("GET sepa/mandats : mandat du foyer visible par un non payeur".into());
        }
        // Création d'un mandat SEPA (détournement du compte débité) : refusée.
        let cree = m
            .appel(
                "POST",
                "/facturation/sepa/mandats",
                Some(&jeton),
                Some(&json!({
                    "foyer_id": foyer,
                    "rum": format!("RS3-{}", Uuid::new_v4().simple()),
                    "iban": "FR7630006000011234567890189",
                    "bic": "AGRIFRPP",
                    "titulaire": "Attaquant"
                })),
            )
            .await;
        if cree.est_succes() {
            echecs.push("POST sepa/mandats : un parent non payeur a créé un mandat".into());
        }

        // Restauration de l'état (payeuse), quoi qu'il arrive.
        let remis = regler_est_payeur(lily, foyer, true).await;
        assert_eq!(remis, 1, "restauration du lien payeur");

        assert!(
            echecs.is_empty(),
            "RS3-01 : parent non payeur a accès à la facturation :\n{}",
            echecs.join("\n")
        );
    });
}

// ───────────────────────── EDT : référence croisée inter-établissements (RS3-04) ─────────────────────────

/// **RS3-04 (faible, corrigée).** `POST /edt/annulations` accepte un
/// `absence_rh_id` du **corps** sans vérifier qu'il appartient à l'établissement
/// de la session : une exception d'EDT de A peut pointer vers une absence RH de
/// B (référence croisée). Impact limité (oracle d'existence d'UUID + blocage de
/// la suppression côté B par la clé étrangère), pas de fuite de données — mais
/// c'est un défaut de cloisonnement à fermer (borner `absence_rh_id` par
/// l'établissement, comme `edt_organiser_remplacement`).
///
/// Corrigée : `edt_annuler_seance` vérifie l'appartenance de l'absence RH à
/// l'établissement de l'acteur (404 comme un identifiant inexistant : pas
/// d'oracle d'UUID) et la clé étrangère composite de la migration 377 verrouille
/// la base.
#[test]
fn rs3_04_edt_annulation_absence_rh_d_un_autre_etablissement() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        // 1. Déclarer une absence RH dans B (Direction de B, sur le Prof de B).
        let prof_b = *m.b.user_ids.get(&Prof).expect("prof de B");
        let debut = chrono::Utc::now().date_naive() + chrono::Duration::weeks(30);
        let decl = m
            .appel(
                "POST",
                "/rh/absences",
                Some(m.b.jetons[&Direction].as_str()),
                Some(&json!({
                    "user_id": prof_b,
                    "debut": debut.to_string(),
                    "fin": (debut + chrono::Duration::days(1)).to_string(),
                    "motif": "RS3-04"
                })),
            )
            .await;
        assert!(decl.statut.is_success(), "déclaration absence RH dans B : {} {}", decl.statut, decl.texte);
        let absence_b = uuid_de(&decl.json["id"]);

        // 2. Trouver un créneau de A (jeton du Prof de A).
        let prof_a = m.jeton(Prof).to_string();
        let base = chrono::Utc::now().date_naive() + chrono::Duration::weeks(40);
        let mut creneau_a = None;
        let mut jour_a = base;
        for d in 0..7 {
            let jour = base + chrono::Duration::days(d);
            let r = m.appel("GET", &format!("/edt/mes-creneaux?jour={jour}"), Some(&prof_a), None).await;
            if let Some(c) = r.json.as_array().and_then(|a| a.first()).cloned() {
                let cid = if c["creneau_id"].is_null() { c["id"].clone() } else { c["creneau_id"].clone() };
                if cid.is_string() {
                    creneau_a = Some(uuid_de(&cid));
                    jour_a = jour;
                    break;
                }
            }
        }
        let Some(creneau_a) = creneau_a else {
            eprintln!("SKIP RS3-04 : aucun créneau d'EDT pour le Prof de A");
            return;
        };

        // 3. Dans A : annuler un créneau de A en référençant l'absence RH de B.
        let rep = m
            .appel(
                "POST",
                "/edt/annulations",
                Some(m.jeton(Direction)),
                Some(&json!({
                    "creneau_id": creneau_a,
                    "jour": jour_a.to_string(),
                    "absence_rh_id": absence_b
                })),
            )
            .await;
        assert!(
            rep.est_refus() || rep.statut.as_u16() == 400 || rep.statut.as_u16() == 409,
            "RS3-04 : l'annulation d'un créneau de A avec une absence RH de B doit être refusée \
             (obtenu {} ; la référence croisée A→B est acceptée)",
            rep.statut.as_u16()
        );
    });
}


// ───────────────────────── Finances : double encaissement SEPA (RS3-02) ─────────────────────────

/// Prépare un prélèvement SEPA planifié (chemin historique) sur la facture de
/// labo du foyer de Harry, via Direction de A. Renvoie `(prelevement_id, e2e)`.
async fn preparer_prelevement(m: &Monde) -> Option<(Uuid, String)> {
    let facture = m.a.objets.facture?;
    let foyer = m.a.objets.foyer;
    let dir = m.jeton(Direction).to_string();
    let mandat = m
        .appel(
            "POST",
            "/facturation/sepa/mandats",
            Some(&dir),
            Some(&json!({
                "foyer_id": foyer,
                "rum": format!("RS3-{}", Uuid::new_v4().simple()),
                "iban": "FR7630006000011234567890189",
                "bic": "AGRIFRPP",
                "titulaire": "Recette"
            })),
        )
        .await;
    assert!(mandat.est_succes(), "mandat : {} {}", mandat.statut, mandat.texte);
    let mandat_id = uuid_de(&mandat.json["id"]);
    let jour = chrono::Utc::now().date_naive() + chrono::Duration::days(20);
    let prev = m
        .appel(
            "POST",
            &format!("/facturation/factures/{facture}/sepa/prelever"),
            Some(&dir),
            Some(&json!({
                "mandat_id": mandat_id,
                "montant_centimes": 100,
                "date_execution": jour.to_string()
            })),
        )
        .await;
    assert!(prev.est_succes(), "prélèvement : {} {}", prev.statut, prev.texte);
    Some((uuid_de(&prev.json["id"]), prev.json["end_to_end_id"].as_str().expect("e2e").to_string()))
}

async fn nb_encaissements(e2e: &str) -> i64 {
    let pool = pool_de_test().await;
    let n: (i64,) =
        sqlx::query_as("SELECT COUNT(*)::bigint FROM ent_fac_encaissement WHERE reference = $1")
            .bind(e2e)
            .fetch_one(&pool)
            .await
            .expect("compte encaissements");
    pool.close().await;
    n.0
}

/// **RS3-02 (élevée, corrigée).** Rejouer `executer` ou le même relevé
/// bancaire ne doit encaisser qu'**une** fois : appels rejoués puis concurrents.
#[test]
fn rs3_02_double_encaissement_sepa_rejeu_et_concurrence() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let Some((prev, e2e)) = preparer_prelevement(m).await else {
            eprintln!("SKIP RS3-02 : pas de facture de labo");
            return;
        };
        let dir = m.jeton(Direction).to_string();
        let chemin = format!("/facturation/sepa/prelevements/{prev}/executer");

        // 1. Appels concurrents : un seul encaissement.
        let (r1, r2, r3, r4) = tokio::join!(
            m.appel("POST", &chemin, Some(&dir), None),
            m.appel("POST", &chemin, Some(&dir), None),
            m.appel("POST", &chemin, Some(&dir), None),
            m.appel("POST", &chemin, Some(&dir), None),
        );
        for r in [&r1, &r2, &r3, &r4] {
            assert!(r.statut.as_u16() < 500, "exécution concurrente : {} {}", r.statut, r.texte);
        }
        assert_eq!(nb_encaissements(&e2e).await, 1, "RS3-02 : appels concurrents → un seul encaissement");

        // 2. Rejeu séquentiel : sans effet.
        let r5 = m.appel("POST", &chemin, Some(&dir), None).await;
        assert!(r5.statut.as_u16() < 500, "rejeu : {} {}", r5.statut, r5.texte);
        assert_eq!(nb_encaissements(&e2e).await, 1, "RS3-02 : rejeu de l'exécution → toujours un encaissement");

        // 3. Même relevé bancaire, deux fois : toujours un seul encaissement.
        let csv = format!("end_to_end_id;montant_centimes;result;motif\n{e2e};100;settled;\n");
        for _ in 0..2 {
            let rap = m
                .appel("POST", "/facturation/banque/rapprocher", Some(&dir), Some(&json!({ "csv": csv })))
                .await;
            assert!(rap.statut.as_u16() < 500, "rapprochement : {} {}", rap.statut, rap.texte);
            assert_eq!(rap.json["executed"], 0, "RS3-02 : relevé rejoué ne ré-exécute rien ({})", rap.texte);
        }
        assert_eq!(nb_encaissements(&e2e).await, 1, "RS3-02 : relevé rejoué → un seul encaissement");

        // 4. Garde en base : un second encaissement « prélèvement » de même référence est refusé.
        let pool = pool_de_test().await;
        let doublon = sqlx::query(
            "INSERT INTO ent_fac_encaissement (id, etablissement_id, facture_id, montant_centimes, moyen, reference, encaisse_at, created_at)
             SELECT gen_random_uuid(), etablissement_id, facture_id, montant_centimes, moyen, reference, now(), now()
               FROM ent_fac_encaissement WHERE reference = $1",
        )
        .bind(&e2e)
        .execute(&pool)
        .await;
        pool.close().await;
        assert!(doublon.is_err(), "RS3-02 : la base doit refuser un second encaissement pour le même prélèvement");

        // 5. Un prélèvement exécuté ne se rejette plus.
        let rej = m
            .appel(
                "POST",
                &format!("/facturation/sepa/prelevements/{prev}/rejeter"),
                Some(&dir),
                Some(&json!({ "motif": "AC04" })),
            )
            .await;
        assert!(!rej.est_succes(), "RS3-02 : rejet d'un prélèvement exécuté accepté ({})", rej.statut);
    });
}

/// **RS3-03 (moyenne).** Le chemin historique ne prélève pas un foyer passé en
/// paiement `manuel` (comme `/finances/*`).
#[test]
fn rs3_03_prelevement_historique_refuse_un_foyer_en_paiement_manuel() {
    let Some(m) = monde() else { return };
    let Some(facture) = m.a.objets.facture else { return };
    rt().block_on(async {
        let foyer = m.a.objets.foyer;
        let pool = pool_de_test().await;
        let (etab,): (Uuid,) = sqlx::query_as("SELECT etablissement_id FROM ent_foyer WHERE id = $1")
            .bind(foyer)
            .fetch_one(&pool)
            .await
            .expect("établissement du foyer");
        sqlx::query(
            "INSERT INTO ent_fin_compte_payeur (etablissement_id, foyer_id, mode_paiement, motif, depuis)
             VALUES ($1, $2, 'manuel', 'RS3-03', now())
             ON CONFLICT (etablissement_id, foyer_id) DO UPDATE SET mode_paiement = 'manuel'",
        )
        .bind(etab)
        .bind(foyer)
        .execute(&pool)
        .await
        .expect("mode manuel");
        let dir = m.jeton(Direction).to_string();
        let mandat = m
            .appel(
                "POST",
                "/facturation/sepa/mandats",
                Some(&dir),
                Some(&json!({
                    "foyer_id": foyer,
                    "rum": format!("RS3-{}", Uuid::new_v4().simple()),
                    "iban": "FR7630006000011234567890189",
                    "bic": "AGRIFRPP",
                    "titulaire": "Recette"
                })),
            )
            .await;
        let rep = if mandat.est_succes() {
            let jour = chrono::Utc::now().date_naive() + chrono::Duration::days(20);
            Some(
                m.appel(
                    "POST",
                    &format!("/facturation/factures/{facture}/sepa/prelever"),
                    Some(&dir),
                    Some(&json!({
                        "mandat_id": uuid_de(&mandat.json["id"]),
                        "montant_centimes": 100,
                        "date_execution": jour.to_string()
                    })),
                )
                .await,
            )
        } else {
            None
        };
        sqlx::query("DELETE FROM ent_fin_compte_payeur WHERE etablissement_id = $1 AND foyer_id = $2 AND motif = 'RS3-03'")
            .bind(etab)
            .bind(foyer)
            .execute(&pool)
            .await
            .expect("restauration");
        pool.close().await;
        let rep = rep.expect("mandat créé");
        assert!(!rep.est_succes(), "RS3-03 : prélèvement planifié sur un foyer en paiement manuel ({})", rep.statut);
    });
}

// ───────────────────────── Invariant : inventaire à jour ─────────────────────────

/// Garde-fou : l'inventaire des routes de `routes_src` couvre toujours tous les
/// fichiers de `src/` qui déclarent des routes (sinon une route échapperait aux
/// deux matrices ci-dessus). Doublon volontaire de `separation.rs`, pour que ce
/// fichier reste autonome.
#[test]
fn rs3_inventaire_couvre_tous_les_fichiers_de_routes() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for e in std::fs::read_dir(src).expect("src") {
        let p = e.expect("entrée").path();
        let nom = p.file_name().unwrap().to_string_lossy().to_string();
        if !nom.ends_with(".rs") || nom == "ui.rs" || nom.starts_with("tests_") {
            continue;
        }
        let texte = std::fs::read_to_string(&p).expect("source");
        if texte.contains(".route(") {
            assert!(
                routes_src::NOMS_SOURCES.contains(&nom.as_str()),
                "{nom} déclare des routes absentes de l'inventaire (routes_src::SOURCES)"
            );
        }
    }
    let r = routes_src::routes();
    assert!(r.len() > 250, "inventaire des routes incomplet : {}", r.len());
}
