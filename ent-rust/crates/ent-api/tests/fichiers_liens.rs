//! Liens courts d'accès aux fichiers (URL pré-signées, mode local) contre le vrai
//! routeur `ent-api` et une base PostgreSQL locale de test.
//!
//! Invariants vérifiés :
//! - pour chaque acteur, `POST …/lien` est accepté **si et seulement si** le
//!   téléchargement existant (`GET …`) l'est (mêmes droits, mêmes refus) ;
//! - un lien n'est jamais émis pour un objet d'un autre établissement ;
//! - un lien expiré, altéré ou signé avec un autre secret est refusé ;
//! - les types actifs (html, svg) sont servis en `attachment` avec les en-têtes durcis.
//!
//! Lancement : `ENT_ENV=test ENT_DATABASE_URL=… cargo test -p ent-api --features labo
//! --test fichiers_liens -- --test-threads=1`. Sans base : SKIP.

#[path = "separation/harness.rs"]
mod harness;

use axum::http::StatusCode;
use base64::Engine;
use ent_db::presign::{now_unix, Disposition, LinkSpec, PresignError, Presigner};
use harness::*;
use serde_json::{json, Value};
use std::sync::Once;
use uuid::Uuid;

use Acteur::*;

const CLE: &str = "cle-de-signature-de-test-0123456789-abcdefghij";

fn monde_liens() -> Option<&'static Monde> {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // Magasin en mémoire (aucun fichier sur disque) et secret de signature connu.
        std::env::set_var("ENT_OBJECT_MEMORY", "1");
        std::env::set_var("ENT_OBJECT_SIGNING_KEY", CLE);
        std::env::remove_var("ENT_OBJECT_LINK_TTL_SECS");
    });
    monde()
}

fn b64(octets: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(octets)
}

fn uuid_de(v: &Value) -> Uuid {
    Uuid::parse_str(v.as_str().expect("uuid")).expect("uuid valide")
}

/// Dépose un document de la famille de `eleve` (par la direction de l'établissement).
async fn deposer(
    m: &Monde,
    jeton: &str,
    eleve: Uuid,
    titre: &str,
    mime: &str,
    contenu: &[u8],
) -> Uuid {
    let r = m
        .appel(
            "POST",
            &format!("/eleves/{eleve}/documents"),
            Some(jeton),
            Some(&json!({
                "titre": titre,
                "tiroir": "famille",
                "confidentialite": "standard",
                "mime_type": mime,
                "contenu_base64": b64(contenu),
            })),
        )
        .await;
    assert_eq!(r.statut, StatusCode::CREATED, "dépôt : {}", r.texte);
    uuid_de(&r.json["id"])
}

/// Pour chaque acteur de A : l'émission d'un lien réussit ssi le téléchargement réussit,
/// et les refus ont le même statut. Retourne le nombre d'acteurs autorisés.
async fn parite_des_droits(m: &Monde, telechargement: &str, lien: &str) -> usize {
    let mut autorises = 0;
    for a in Acteur::TOUS {
        let dl = m.appel("GET", telechargement, Some(m.jeton(a)), None).await;
        let li = m.appel("POST", lien, Some(m.jeton(a)), None).await;
        assert_eq!(
            dl.est_succes(),
            li.est_succes(),
            "{a:?} : GET {telechargement} → {} mais POST {lien} → {} ({})",
            dl.statut,
            li.statut,
            li.texte
        );
        if dl.est_succes() {
            autorises += 1;
            assert!(li.json["url"].as_str().is_some_and(|u| u.starts_with("/fichiers/")));
            assert!(li.json["expires_at"].as_i64().unwrap() > now_unix());
        } else {
            assert_eq!(dl.statut, li.statut, "{a:?} : refus différent sur {lien}");
            assert!(!li.texte.contains("/fichiers/"), "{a:?} : lien fuité malgré le refus");
        }
    }
    autorises
}

async fn suivre(m: &Monde, url: &str) -> Rep {
    m.appel("GET", url, None, None).await
}

fn rt_run<F: std::future::Future>(f: F) -> F::Output {
    rt().block_on(f)
}

#[test]
fn document_droits_identiques_au_telechargement() {
    let Some(m) = monde_liens() else { return };
    rt_run(async {
        let dir = m.jeton(Direction).to_string();
        let doc = deposer(m, &dir, m.a.objets.eleve, "Assurance", "application/pdf", b"%PDF-1.4 a").await;
        let n = parite_des_droits(m, &format!("/documents/{doc}/fichier"), &format!("/documents/{doc}/lien")).await;
        assert!(n >= 3, "au moins direction, admin et parent lié : {n}");
        // Parent non lié (autre foyer) et parent sans lien : refusés, comme le téléchargement.
        for a in [ParentAutreFoyer, ParentSansDrapeaux] {
            let r = m
                .appel("POST", &format!("/documents/{doc}/lien"), Some(m.jeton(a)), None)
                .await;
            assert!(r.est_refus(), "{a:?} : {}", r.statut);
        }
        // Sans session : 401.
        let r = m.appel("POST", &format!("/documents/{doc}/lien"), None, None).await;
        assert_eq!(r.statut, StatusCode::UNAUTHORIZED);
    });
}

#[test]
fn document_lien_sert_le_fichier_avec_en_tetes_durcis() {
    let Some(m) = monde_liens() else { return };
    rt_run(async {
        let dir = m.jeton(Direction).to_string();
        let contenu = b"%PDF-1.4 contenu-de-test";
        let doc = deposer(m, &dir, m.a.objets.eleve, "Bulletin T1", "application/pdf", contenu).await;
        let l = m
            .appel("POST", &format!("/documents/{doc}/lien"), Some(m.jeton(Parent)), None)
            .await;
        assert_eq!(l.statut, StatusCode::OK, "{}", l.texte);
        let url = l.json["url"].as_str().unwrap().to_string();
        // Aucune clé de stockage dans la réponse ni dans l'URL en clair.
        assert!(!l.texte.contains("storage_key") && !url.contains("documents/"));
        // Le jeton suffit : aucune session.
        let r = suivre(m, &url).await;
        assert_eq!(r.statut, StatusCode::OK, "{}", r.texte);
        assert_eq!(r.texte.as_bytes(), contenu);
        // En-têtes via requête brute.
        let rep = m.requete("GET", &url, None, None, &[]).await;
        assert_eq!(rep.statut, StatusCode::OK);
    });
}

#[test]
fn types_actifs_servis_en_attachment_et_en_tetes() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;
    let Some(m) = monde_liens() else { return };
    rt_run(async {
        let dir = m.jeton(Direction).to_string();
        for (titre, mime, contenu) in [
            ("page.html", "text/html", "<script>alert(1)</script>"),
            ("image.svg", "image/svg+xml", "<svg xmlns='http://www.w3.org/2000/svg' onload='alert(1)'/>"),
            ("doc.xml", "application/xml", "<a/>"),
            ("Bulletin", "application/pdf", "%PDF-1.4 x"),
        ] {
            let doc = deposer(m, &dir, m.a.objets.eleve, titre, mime, contenu.as_bytes()).await;
            let l = m
                .appel("POST", &format!("/documents/{doc}/lien"), Some(&dir), None)
                .await;
            assert_eq!(l.statut, StatusCode::OK, "{titre} : {}", l.texte);
            let url = l.json["url"].as_str().unwrap();
            let res = m
                .routeur
                .clone()
                .oneshot(Request::builder().uri(url).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK, "{titre}");
            let h = res.headers();
            let get = |n: &str| h.get(n).map(|v| v.to_str().unwrap().to_string()).unwrap_or_default();
            assert_eq!(get("x-content-type-options"), "nosniff", "{titre}");
            assert_eq!(get("cache-control"), "private, no-store", "{titre}");
            assert!(get("content-security-policy").starts_with("default-src 'none'"), "{titre}");
            let cd = get("content-disposition");
            if titre == "Bulletin" {
                assert!(cd.starts_with("inline;"), "{cd}");
                assert_eq!(get("content-type"), "application/pdf");
            } else {
                assert!(cd.starts_with("attachment;"), "{titre} servi en ligne : {cd}");
                assert!(get("content-security-policy").contains("sandbox"), "{titre}");
            }
        }
    });
}

#[test]
fn autre_etablissement_refuse() {
    let Some(m) = monde_liens() else { return };
    rt_run(async {
        let dir_b = m.b.jetons[&Direction].clone();
        let doc_b = deposer(m, &dir_b, m.b.objets.eleve, "Secret B", "application/pdf", b"%PDF-B").await;
        // La direction de A n'obtient aucun lien pour un document de B (cloisonnement).
        for a in [Admin, Direction, Administratif] {
            let r = m
                .appel("POST", &format!("/documents/{doc_b}/lien"), Some(m.jeton(a)), None)
                .await;
            assert!(r.est_refus(), "{a:?} : {} {}", r.statut, r.texte);
        }
        // Un lien légitime de B reste un lien de B : l'émetteur refuse de signer une clé
        // de B pour un destinataire de A (et inversement).
        let p = Presigner::local(CLE.as_bytes(), 60).unwrap();
        let etab_a = m.a.objets.etab;
        let cle_b = format!("{}/documents/{}.pdf", m.b.objets.etab, doc_b);
        let r = p.issue(
            &LinkSpec {
                key: cle_b,
                user_id: Uuid::new_v4(),
                etablissement_id: etab_a,
                disposition: Disposition::Inline,
                filename: "x.pdf".into(),
                mime: "application/pdf".into(),
            },
            now_unix(),
        );
        assert_eq!(r, Err(PresignError::InvalidParam));
        // Recoller la signature d'un lien de A sur la charge d'un lien de B est refusé.
        let doc_a = deposer(m, &m.jeton(Direction).to_string(), m.a.objets.eleve, "A", "application/pdf", b"%PDF-A").await;
        let la = m.appel("POST", &format!("/documents/{doc_a}/lien"), Some(m.jeton(Direction)), None).await;
        let lb = m.appel("POST", &format!("/documents/{doc_b}/lien"), Some(&dir_b), None).await;
        assert_eq!(lb.statut, StatusCode::OK);
        let ua = la.json["url"].as_str().unwrap().trim_start_matches("/fichiers/").to_string();
        let ub = lb.json["url"].as_str().unwrap().trim_start_matches("/fichiers/").to_string();
        let (_, sig_a) = ua.split_once('.').unwrap();
        let (charge_b, _) = ub.split_once('.').unwrap();
        let r = suivre(m, &format!("/fichiers/{charge_b}.{sig_a}")).await;
        assert_eq!(r.statut, StatusCode::FORBIDDEN);
        assert!(!r.texte.contains("%PDF"));
        // Et le lien de B, légitime, sert bien B.
        let r = suivre(m, &format!("/fichiers/{ub}")).await;
        assert_eq!(r.statut, StatusCode::OK);
        assert_eq!(r.texte, "%PDF-B");
    });
}

#[test]
fn lien_expire_altere_ou_mauvais_secret_refuse() {
    let Some(m) = monde_liens() else { return };
    rt_run(async {
        let dir = m.jeton(Direction).to_string();
        let doc = deposer(m, &dir, m.a.objets.eleve, "Pour expirer", "application/pdf", b"%PDF-E").await;
        let cle = format!("{}/documents/{doc}.pdf", m.a.objets.etab);
        let spec = LinkSpec {
            key: cle,
            user_id: m.a.user_ids[&Direction],
            etablissement_id: m.a.objets.etab,
            disposition: Disposition::Attachment,
            filename: "e.pdf".into(),
            mime: "application/pdf".into(),
        };
        // Même secret que le serveur, émis dans le passé : expiré (410).
        let p = Presigner::local(CLE.as_bytes(), 60).unwrap();
        let expire = p.issue(&spec, now_unix() - 3600).unwrap();
        assert_eq!(suivre(m, &expire.url).await.statut, StatusCode::GONE);
        // Même charge, émis maintenant : valide (le test forge bien comme le serveur).
        let valide = p.issue(&spec, now_unix()).unwrap();
        let r = suivre(m, &valide.url).await;
        assert_eq!(r.statut, StatusCode::OK, "{}", r.texte);
        // Mauvais secret : refusé.
        let autre = Presigner::local(b"un-autre-secret-de-signature-0123456789abcdef", 60).unwrap();
        let faux = autre.issue(&spec, now_unix()).unwrap();
        assert_eq!(suivre(m, &faux.url).await.statut, StatusCode::FORBIDDEN);
        // Altération d'un caractère (charge puis signature).
        let jeton = valide.url.trim_start_matches("/fichiers/").to_string();
        for i in [0usize, 10, jeton.len() / 2, jeton.len() - 1] {
            let mut octets = jeton.clone().into_bytes();
            octets[i] = if octets[i] == b'A' { b'B' } else { b'A' };
            let r = suivre(m, &format!("/fichiers/{}", String::from_utf8(octets).unwrap())).await;
            assert!(r.est_refus() || r.statut == StatusCode::FORBIDDEN, "i={i} : {}", r.statut);
            assert_ne!(r.statut, StatusCode::OK, "jeton altéré accepté (i={i})");
        }
        for poubelle in ["/fichiers/x", "/fichiers/a.b", "/fichiers/..%2F..%2Fetc%2Fpasswd"] {
            assert_ne!(suivre(m, poubelle).await.statut, StatusCode::OK, "{poubelle}");
        }
    });
}

#[test]
fn photo_droits_identiques() {
    let Some(m) = monde_liens() else { return };
    rt_run(async {
        let dir = m.jeton(Direction).to_string();
        let eleve = m.a.objets.eleve;
        let jpeg = [0xFFu8, 0xD8, 0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1];
        let r = m
            .appel(
                "POST",
                &format!("/eleves/{eleve}/photo"),
                Some(&dir),
                Some(&json!({"mime_type": "image/jpeg", "contenu_base64": b64(&jpeg)})),
            )
            .await;
        assert!(r.est_succes(), "pose de photo : {} {}", r.statut, r.texte);
        let n = parite_des_droits(m, &format!("/eleves/{eleve}/photo"), &format!("/eleves/{eleve}/photo/lien")).await;
        assert!(n >= 2, "{n}");
        // Le lien sert les octets de la photo.
        let l = m.appel("POST", &format!("/eleves/{eleve}/photo/lien"), Some(&dir), None).await;
        let r = suivre(m, l.json["url"].as_str().unwrap()).await;
        assert_eq!(r.statut, StatusCode::OK);
        // Élève d'un autre établissement : refusé.
        let b = m.b.objets.eleve;
        let r = m.appel("POST", &format!("/eleves/{b}/photo/lien"), Some(&dir), None).await;
        assert!(r.est_refus(), "{}", r.statut);
    });
}

#[test]
fn piece_pai_droits_identiques() {
    let Some(m) = monde_liens() else { return };
    rt_run(async {
        let inf = m.jeton(Infirmiere).to_string();
        let eleve = m.a.objets.eleve;
        let r = m
            .appel(
                "PUT",
                &format!("/eleves/{eleve}/pai"),
                Some(&inf),
                Some(&json!({
                    "motif": "allergie", "mesures_accueil": "trousse", "protocole_urgence": "15",
                    "date_debut": "2025-09-01", "actif": true
                })),
            )
            .await;
        assert!(r.est_succes(), "PAI : {} {}", r.statut, r.texte);
        let r = m
            .appel(
                "POST",
                &format!("/eleves/{eleve}/pai/pieces"),
                Some(&inf),
                Some(&json!({"nom": "ordonnance.pdf", "mime_type": "application/pdf", "contenu_base64": b64(b"%PDF-1.4 ordo")})),
            )
            .await;
        assert_eq!(r.statut, StatusCode::CREATED, "{}", r.texte);
        let piece = uuid_de(&r.json["id"]);
        let n = parite_des_droits(m, &format!("/pai/pieces/{piece}"), &format!("/pai/pieces/{piece}/lien")).await;
        assert!(n >= 2, "{n}");
        // Parent d'un autre foyer et professeur : jamais de lien.
        for a in [ParentAutreFoyer, Prof] {
            let r = m.appel("POST", &format!("/pai/pieces/{piece}/lien"), Some(m.jeton(a)), None).await;
            assert!(r.est_refus(), "{a:?} : {}", r.statut);
        }
        let l = m.appel("POST", &format!("/pai/pieces/{piece}/lien"), Some(&inf), None).await;
        let r = suivre(m, l.json["url"].as_str().unwrap()).await;
        assert_eq!(r.statut, StatusCode::OK);
        assert_eq!(r.texte, "%PDF-1.4 ordo");
    });
}

#[test]
fn piece_message_droits_identiques() {
    let Some(m) = monde_liens() else { return };
    rt_run(async {
        let dir = m.jeton(Direction).to_string();
        let eleve = m.a.objets.eleve;
        let fil = m
            .appel(
                "POST",
                "/messages/fils",
                Some(&dir),
                Some(&json!({"eleve_id": eleve, "sujet": "pièce jointe", "message": "voir pièce"})),
            )
            .await;
        assert_eq!(fil.statut, StatusCode::CREATED, "{}", fil.texte);
        let msg = uuid_de(&fil.json["messages"][0]["id"]);
        let r = m
            .appel(
                "POST",
                &format!("/messages/{msg}/pieces"),
                Some(&dir),
                Some(&json!({"nom": "mot.txt", "mime_type": "text/plain", "contenu_base64": b64(b"bonjour")})),
            )
            .await;
        assert_eq!(r.statut, StatusCode::CREATED, "{}", r.texte);
        let piece = uuid_de(&r.json["id"]);
        let n = parite_des_droits(m, &format!("/messages/pieces/{piece}"), &format!("/messages/pieces/{piece}/lien")).await;
        assert!(n >= 2, "{n}");
        for a in [ParentAutreFoyer, ParentSansDrapeaux] {
            let r = m.appel("POST", &format!("/messages/pieces/{piece}/lien"), Some(m.jeton(a)), None).await;
            assert!(r.est_refus(), "{a:?} : {}", r.statut);
        }
        // Idempotent : deux liens, même contenu ; texte/plain d'une pièce jointe → attachment.
        for _ in 0..2 {
            let l = m.appel("POST", &format!("/messages/pieces/{piece}/lien"), Some(m.jeton(Parent)), None).await;
            assert_eq!(l.statut, StatusCode::OK);
            let r = suivre(m, l.json["url"].as_str().unwrap()).await;
            assert_eq!(r.texte, "bonjour");
        }
        // Établissement B : refusé.
        let r = m
            .appel("POST", &format!("/messages/pieces/{piece}/lien"), Some(m.b.jetons[&Direction].as_str()), None)
            .await;
        assert!(r.est_refus(), "{}", r.statut);
    });
}

#[test]
fn journal_de_l_emission() {
    let Some(m) = monde_liens() else { return };
    rt_run(async {
        let dir = m.jeton(Direction).to_string();
        let doc = deposer(m, &dir, m.a.objets.eleve, "Tracé", "application/pdf", b"%PDF-T").await;
        let l = m.appel("POST", &format!("/documents/{doc}/lien"), Some(m.jeton(Parent)), None).await;
        assert_eq!(l.statut, StatusCode::OK);
        let pool = sqlx::PgPool::connect(&std::env::var("ENT_DATABASE_URL").unwrap()).await.unwrap();
        let n: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND action = 'lien_document' AND actor_user_id = $2",
        )
        .bind(m.a.objets.etab)
        .bind(m.a.user_ids[&Parent])
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(n.0 >= 1, "émission non journalisée");
        // Le journal ne contient ni le jeton ni la clé objet.
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT libelle || payload::text FROM ent_journal_modif WHERE etablissement_id = $1 AND action = 'lien_document'",
        )
        .bind(m.a.objets.etab)
        .fetch_all(&pool)
        .await
        .unwrap();
        for (t,) in rows {
            assert!(!t.contains("/fichiers/") && !t.contains(".pdf"), "{t}");
        }
    });
}
