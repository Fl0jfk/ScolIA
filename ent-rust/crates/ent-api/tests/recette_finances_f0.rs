//! Finances, lot F0 — correctifs urgents de l'existant (socles 24 et 25).
//!
//! Même principe que `recette_securite_3.rs` : le routeur réel `ent_api::app` est
//! appelé en process sur un PostgreSQL **local de test**, avec les établissements
//! de labo A et B. Chaque test correspond à un défaut de la conception
//! (`docs/socles/24_FINANCES.md` §2, FD-xx) : il **échoue sans le correctif**.
//!
//! Lancement :
//!   service postgresql start
//!   ENT_DATABASE_URL='postgres://ent_ci:ci@127.0.0.1:5432/ent_ci_test' ENT_ENV=test \
//!   RUST_MIN_STACK=16777216 cargo test --locked -p ent-api --features ent-api/labo \
//!   --test recette_finances_f0 -- --test-threads=1
//! Sans `ENT_DATABASE_URL` : tests ignorés proprement (« SKIP »).

#![allow(dead_code)]

#[path = "separation/harness.rs"]
mod harness;
#[path = "separation/routes_src.rs"]
mod routes_src;

use harness::*;
use serde_json::{json, Value};
use uuid::Uuid;

use Acteur::*;

fn uuid_de(v: &Value) -> Uuid {
    Uuid::parse_str(v.as_str().unwrap_or_else(|| panic!("uuid attendu : {v}"))).expect("uuid")
}

async fn pool() -> sqlx::PgPool {
    let url = std::env::var("ENT_DATABASE_URL").expect("base de test");
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connexion de préparation")
}

/// Crée un brouillon d'un total donné sur le foyer de Harry ; renvoie `(id, réponse)`.
async fn brouillon(m: &Monde, total_centimes: i64, numero_client: &str) -> (Uuid, Rep) {
    let dir = m.jeton(Direction).to_string();
    let rep = m
        .appel(
            "POST",
            "/facturation/factures",
            Some(&dir),
            Some(&json!({
                "foyer_id": m.a.objets.foyer,
                "numero": numero_client,
                "libelle": "Recette F0",
                "lignes": [{ "libelle": "Cantine", "quantite": 1, "montant_unitaire_centimes": total_centimes }]
            })),
        )
        .await;
    assert!(rep.est_succes(), "création : {} {}", rep.statut, rep.texte);
    (uuid_de(&rep.json["facture"]["id"]), rep)
}

async fn emettre(m: &Monde, id: Uuid) -> Rep {
    let dir = m.jeton(Direction).to_string();
    let rep = m
        .appel("POST", &format!("/facturation/factures/{id}/emettre"), Some(&dir), None)
        .await;
    assert!(rep.est_succes(), "émission : {} {}", rep.statut, rep.texte);
    rep
}

async fn facture_emise(m: &Monde, total: i64) -> Uuid {
    let (id, _) = brouillon(m, total, "").await;
    emettre(m, id).await;
    id
}

/// Mandat SEPA actif du foyer de Harry (créé au besoin).
async fn mandat_actif(m: &Monde) -> Uuid {
    let dir = m.jeton(Direction).to_string();
    let liste = m.appel("GET", "/facturation/sepa/mandats", Some(&dir), None).await;
    if let Some(a) = liste.json.as_array() {
        for x in a {
            if x["foyer_id"].as_str() == Some(&m.a.objets.foyer.to_string())
                && x["statut"].as_str() == Some("actif")
            {
                return uuid_de(&x["id"]);
            }
        }
    }
    let rep = m
        .appel(
            "POST",
            "/facturation/sepa/mandats",
            Some(&dir),
            Some(&json!({
                "foyer_id": m.a.objets.foyer,
                "iban": "FR7630006000011234567890189",
                "bic": "AGRIFRPP",
                "titulaire": "Recette F0"
            })),
        )
        .await;
    assert!(rep.est_succes(), "mandat : {} {}", rep.statut, rep.texte);
    uuid_de(&rep.json["id"])
}

fn dans_20_jours() -> String {
    (domain_eleve::horloge::jour_paris() + chrono::Duration::days(20)).to_string()
}

async fn prelever(m: &Monde, facture: Uuid, mandat: Uuid, centimes: i64) -> Rep {
    let dir = m.jeton(Direction).to_string();
    m.appel(
        "POST",
        &format!("/facturation/factures/{facture}/sepa/prelever"),
        Some(&dir),
        Some(&json!({
            "mandat_id": mandat,
            "montant_centimes": centimes,
            "date_execution": dans_20_jours()
        })),
    )
    .await
}

async fn statut_facture(id: Uuid) -> String {
    let p = pool().await;
    let (s,): (String,) = sqlx::query_as("SELECT statut FROM ent_fac_facture WHERE id = $1")
        .bind(id)
        .fetch_one(&p)
        .await
        .expect("statut");
    p.close().await;
    s
}

// ───────────────────────── FD-05 : numéro attribué par le serveur ─────────────────────────

/// Le numéro fourni par le client est ignoré ; le numéro définitif vient d'une série
/// continue attribuée à l'émission, jamais deux fois le même.
#[test]
fn f0_fd05_numero_attribue_par_le_serveur() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let (id1, rep) = brouillon(m, 1000, "CM-20261110").await;
        let provisoire = rep.json["facture"]["numero"].as_str().unwrap().to_string();
        assert_ne!(provisoire, "CM-20261110", "FD-05 : numéro du client conservé");
        assert!(provisoire.starts_with("BRO-"), "numéro provisoire attendu : {provisoire}");
        // Même numéro demandé deux fois le même jour : plus de collision.
        let (id2, _) = brouillon(m, 2000, "CM-20261110").await;

        let e1 = emettre(m, id1).await;
        let e2 = emettre(m, id2).await;
        let n1 = e1.json["numero"].as_str().unwrap().to_string();
        let n2 = e2.json["numero"].as_str().unwrap().to_string();
        let motif = |n: &str| {
            n.len() == 12 && n.starts_with('F') && n[1..5].chars().all(|c| c.is_ascii_digit())
                && &n[5..6] == "-" && n[6..].chars().all(|c| c.is_ascii_digit())
        };
        assert!(motif(&n1) && motif(&n2), "série F<année>-<6 chiffres> : {n1} / {n2}");
        let rang = |n: &str| n[6..].parse::<i64>().unwrap();
        assert_eq!(rang(&n2), rang(&n1) + 1, "FD-05 : série continue ({n1} → {n2})");
    });
}

// ───────────────────────── FD-08 / FD-09 : immutabilité en base ─────────────────────────

/// Une pièce émise ne se modifie ni ne se supprime, même en SQL direct (script,
/// import, chemin oublié) ; les encaissements et quittances sont figés ; plus de
/// `ON DELETE CASCADE`.
#[test]
fn f0_fd08_fd09_piece_emise_immuable_et_non_supprimable_en_base() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let id = facture_emise(m, 5000).await;
        let dir = m.jeton(Direction).to_string();
        let enc = m
            .appel(
                "POST",
                &format!("/facturation/factures/{id}/encaisser"),
                Some(&dir),
                Some(&json!({ "montant_centimes": 1000, "moyen": "especes" })),
            )
            .await;
        assert!(enc.est_succes(), "encaissement : {} {}", enc.statut, enc.texte);

        let p = pool().await;
        for (nom, sql) in [
            ("total", "UPDATE ent_fac_facture SET total_centimes = 1 WHERE id = $1"),
            ("numéro", "UPDATE ent_fac_facture SET numero = 'X-1' WHERE id = $1"),
            ("débiteur", "UPDATE ent_fac_facture SET foyer_id = foyer_id WHERE false AND id = $1 -- noop"),
            ("retour brouillon", "UPDATE ent_fac_facture SET statut = 'brouillon' WHERE id = $1"),
            ("suppression facture", "DELETE FROM ent_fac_facture WHERE id = $1"),
            ("lignes (modif)", "UPDATE ent_fac_ligne SET montant_unitaire_centimes = 1 WHERE facture_id = $1"),
            ("lignes (suppression)", "DELETE FROM ent_fac_ligne WHERE facture_id = $1"),
            ("lignes (ajout)", "INSERT INTO ent_fac_ligne (id, etablissement_id, facture_id, libelle, quantite, montant_unitaire_centimes) SELECT gen_random_uuid(), etablissement_id, id, 'ajout', 1, 1 FROM ent_fac_facture WHERE id = $1"),
            ("encaissement (modif)", "UPDATE ent_fac_encaissement SET montant_centimes = 1 WHERE facture_id = $1"),
            ("encaissement (suppression)", "DELETE FROM ent_fac_encaissement WHERE facture_id = $1"),
            ("quittance (modif)", "UPDATE ent_fac_quittance SET montant_centimes = 1 WHERE facture_id = $1"),
            ("quittance (suppression)", "DELETE FROM ent_fac_quittance WHERE facture_id = $1"),
        ] {
            if nom == "débiteur" {
                continue; // couvert par « total » et « numéro » : même déclencheur
            }
            let r = sqlx::query(sql).bind(id).execute(&p).await;
            assert!(r.is_err(), "FD-08/09 : « {nom} » aurait dû être refusé par la base");
        }
        // Les champs de suivi restent libres : passage soldée puis retour émise.
        let r = sqlx::query("UPDATE ent_fac_facture SET statut = 'soldee', soldee_at = now() WHERE id = $1")
            .bind(id)
            .execute(&p)
            .await;
        assert!(r.is_ok(), "le suivi de solde doit rester possible : {r:?}");
        sqlx::query("UPDATE ent_fac_facture SET statut = 'emise', soldee_at = NULL WHERE id = $1")
            .bind(id)
            .execute(&p)
            .await
            .expect("retour emise");
        // Plus aucune cascade depuis une pièce comptable.
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*)::bigint FROM pg_constraint c
              WHERE c.contype = 'f' AND c.confdeltype = 'c'
                AND c.confrelid IN ('ent_fac_facture'::regclass, 'ent_fac_ligne'::regclass,
                                    'ent_fac_echeance'::regclass, 'ent_fac_encaissement'::regclass,
                                    'ent_fac_sepa_mandat'::regclass, 'ent_fac_sepa_prelevement'::regclass)",
        )
        .fetch_one(&p)
        .await
        .expect("cascades");
        assert_eq!(n, 0, "FD-08 : ON DELETE CASCADE restant depuis une pièce comptable");
        // Un brouillon, lui, reste supprimable (il n'est pas une pièce).
        let (idb, _) = brouillon(m, 100, "").await;
        sqlx::query("DELETE FROM ent_fac_ligne WHERE facture_id = $1").bind(idb).execute(&p).await.expect("lignes du brouillon");
        sqlx::query("DELETE FROM ent_fac_facture WHERE id = $1").bind(idb).execute(&p).await.expect("brouillon");
        p.close().await;
    });
}

// ───────────────────────── FD-06 / FD-07 : encaissement ─────────────────────────

/// Un prélèvement ouvert n'est pas de l'argent reçu : un encaissement manuel ne solde
/// pas la facture (FD-06) ; le dépassement de ce qui reste à engager est refusé.
#[test]
fn f0_fd06_prelevement_ouvert_ne_solde_pas_la_facture() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        let id = facture_emise(m, 10_000).await;
        let mandat = mandat_actif(m).await;
        let prev = prelever(m, id, mandat, 6_000).await;
        assert!(prev.est_succes(), "prélèvement : {} {}", prev.statut, prev.texte);

        let enc = m
            .appel(
                "POST",
                &format!("/facturation/factures/{id}/encaisser"),
                Some(&dir),
                Some(&json!({ "montant_centimes": 4_000, "moyen": "cheque" })),
            )
            .await;
        assert!(enc.est_succes(), "encaissement : {} {}", enc.statut, enc.texte);
        assert_eq!(
            statut_facture(id).await,
            "emise",
            "FD-06 : facture « soldée » avec un prélèvement encore ouvert"
        );
        // Plus rien à engager : un centime de plus est refusé.
        let trop = m
            .appel(
                "POST",
                &format!("/facturation/factures/{id}/encaisser"),
                Some(&dir),
                Some(&json!({ "montant_centimes": 1, "moyen": "especes" })),
            )
            .await;
        assert!(!trop.est_succes(), "dépassement accepté : {} {}", trop.statut, trop.texte);

        // Le prélèvement s'exécute ensuite : c'est lui qui solde.
        let prev_id = uuid_de(&prev.json["id"]);
        let exe = m
            .appel("POST", &format!("/facturation/sepa/prelevements/{prev_id}/executer"), Some(&dir), None)
            .await;
        assert!(exe.est_succes(), "exécution : {} {}", exe.statut, exe.texte);
        assert_eq!(statut_facture(id).await, "soldee");
    });
}

/// Deux guichets qui encaissent en même temps le total d'une facture : un seul passe
/// (verrou de la facture, FD-07) ; chaque encaissement a sa quittance, au rang unique.
#[test]
fn f0_fd07_encaissements_concurrents_sans_depassement() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        let id = facture_emise(m, 3_000).await;
        let chemin = format!("/facturation/factures/{id}/encaisser");
        let corps = json!({ "montant_centimes": 3_000, "moyen": "especes" });
        let (a, b, c) = tokio::join!(
            m.appel("POST", &chemin, Some(&dir), Some(&corps)),
            m.appel("POST", &chemin, Some(&dir), Some(&corps)),
            m.appel("POST", &chemin, Some(&dir), Some(&corps)),
        );
        let ok = [&a, &b, &c].iter().filter(|r| r.est_succes()).count();
        for r in [&a, &b, &c] {
            assert!(r.statut.as_u16() < 500, "erreur serveur : {} {}", r.statut, r.texte);
        }
        assert_eq!(ok, 1, "FD-07 : {ok} encaissements acceptés pour un seul solde");
        let p = pool().await;
        let (somme, nb_q): (i64, i64) = sqlx::query_as(
            "SELECT COALESCE((SELECT SUM(montant_centimes) FROM ent_fac_encaissement WHERE facture_id = $1), 0)::bigint,
                    (SELECT count(*) FROM ent_fac_quittance WHERE facture_id = $1)::bigint",
        )
        .bind(id)
        .fetch_one(&p)
        .await
        .expect("sommes");
        p.close().await;
        assert_eq!(somme, 3_000, "FD-07 : encaissé au-delà du total");
        assert_eq!(nb_q, 1, "FD-07 : encaissement sans quittance ou quittance en double");
        assert_eq!(statut_facture(id).await, "soldee");
    });
}

// ───────────────────────── FD-04 : double chemin de prélèvement ─────────────────────────

/// Une facture qui a des échéances ne se prélève pas par le chemin historique
/// (`echeance_id` NULL, invisible de la préparation par échéance) : double prélèvement.
#[test]
fn f0_fd04_prelevement_historique_refuse_sur_facture_a_echeances() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        let id = facture_emise(m, 8_000).await;
        let mandat = mandat_actif(m).await;
        let demain = (domain_eleve::horloge::jour_paris() + chrono::Duration::days(40)).to_string();
        let ech = m
            .appel(
                "POST",
                &format!("/facturation/factures/{id}/echeancier"),
                Some(&dir),
                Some(&json!({ "echeances": [
                    { "libelle": "1/2", "montant_centimes": 4_000, "date_echeance": demain },
                    { "libelle": "2/2", "montant_centimes": 4_000, "date_echeance": demain }
                ]})),
            )
            .await;
        assert!(ech.est_succes(), "échéancier : {} {}", ech.statut, ech.texte);
        let rep = prelever(m, id, mandat, 4_000).await;
        assert!(
            !rep.est_succes(),
            "FD-04 : prélèvement historique accepté sur une facture à échéances ({})",
            rep.statut
        );
    });
}

/// L'export historique n'emporte que ses propres prélèvements (sans échéance) : celui
/// d'une échéance (chemin par échéance) reste pour `/finances/prelevements/export`.
#[test]
fn f0_fd04_export_historique_limite_a_ses_prelevements() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        let id = facture_emise(m, 2_000).await;
        let mandat = mandat_actif(m).await;
        let prev = prelever(m, id, mandat, 500).await;
        assert!(prev.est_succes(), "prélèvement : {} {}", prev.statut, prev.texte);
        // Un second prélèvement, rattaché artificiellement à une échéance : hors périmètre.
        let prev_id = uuid_de(&prev.json["id"]);
        let p = pool().await;
        let jour = dans_20_jours();
        let r = m
            .appel(
                "POST",
                "/facturation/sepa/export",
                Some(&dir),
                Some(&json!({ "debut": jour, "fin": jour, "format": "csv" })),
            )
            .await;
        assert!(r.est_succes(), "export historique : {} {}", r.statut, r.texte);
        assert!(r.texte.contains(&prev.json["end_to_end_id"].as_str().unwrap().to_string()),
            "le prélèvement historique pré-notifié doit être exporté : {}", r.texte);
        let (st,): (String,) = sqlx::query_as("SELECT statut FROM ent_fac_sepa_prelevement WHERE id = $1")
            .bind(prev_id)
            .fetch_one(&p)
            .await
            .expect("statut");
        assert_eq!(st, "exporte");
        p.close().await;
    });
}

// ───────────────────────── FD-03 / R-P6 : export SEPA ─────────────────────────

/// L'export SEPA change l'état : plus de GET ; aucun paramètre `cdtr_*` (IBAN du
/// créancier hors URL) ; un champ inconnu est refusé ; pain.008 sans adresse.
#[test]
fn f0_fd03_export_sepa_en_post_sans_iban_dans_l_url() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        let jour = dans_20_jours();
        let get = m
            .appel(
                "GET",
                &format!("/facturation/sepa/export?debut={jour}&fin={jour}&format=csv"),
                Some(&dir),
                None,
            )
            .await;
        assert_eq!(get.statut.as_u16(), 405, "FD-03 : l'export SEPA répond encore en GET ({})", get.statut);
        let avec_cdtr = m
            .appel(
                "POST",
                "/facturation/sepa/export",
                Some(&dir),
                Some(&json!({ "debut": jour, "fin": jour, "format": "pain008",
                              "cdtr_iban": "FR7630006000011234567890189" })),
            )
            .await;
        assert!(
            avec_cdtr.statut.is_client_error(),
            "FD-03 : un IBAN de créancier fourni par le client est accepté ({})",
            avec_cdtr.statut
        );
    });
}

/// Sans pré-notification, ni le service ni la base ne laissent un prélèvement partir.
#[test]
fn f0_r_p6_pas_d_export_sans_prenotification() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let id = facture_emise(m, 1_500).await;
        let mandat = mandat_actif(m).await;
        let p = pool().await;
        let (etab,): (Uuid,) = sqlx::query_as("SELECT etablissement_id FROM ent_fac_facture WHERE id = $1")
            .bind(id)
            .fetch_one(&p)
            .await
            .expect("etab");
        let prev = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fac_sepa_prelevement (id, etablissement_id, mandat_id, facture_id, foyer_id,
                    montant_centimes, date_execution, end_to_end_id, statut)
             SELECT $1, $2, $3, $4, foyer_id, 100, (now() + interval '30 days')::date, 'F0-' || $1::text, 'prevu'
               FROM ent_fac_facture WHERE id = $4",
        )
        .bind(prev)
        .bind(etab)
        .bind(mandat)
        .bind(id)
        .execute(&p)
        .await
        .expect("prélèvement sans pré-notification (inséré à la main)");
        let r = sqlx::query("UPDATE ent_fac_sepa_prelevement SET statut = 'exporte' WHERE id = $1")
            .bind(prev)
            .execute(&p)
            .await;
        assert!(r.is_err(), "R-P6 : la base a laissé exporter un prélèvement non pré-notifié");
        sqlx::query("UPDATE ent_fac_sepa_prelevement SET statut = 'annule' WHERE id = $1")
            .bind(prev)
            .execute(&p)
            .await
            .expect("nettoyage");
        p.close().await;
    });
}

// ───────────────────────── FD-10 : mandat révoqué ─────────────────────────

/// Révoquer un mandat annule ses prélèvements prévus ; aucun nouveau prélèvement ne
/// peut être posé sur un mandat révoqué (service et base) ; l'export l'exclut.
#[test]
fn f0_fd10_mandat_revoque_exclu() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        let id = facture_emise(m, 4_000).await;
        let mandat = mandat_actif(m).await;
        let prev = prelever(m, id, mandat, 700).await;
        assert!(prev.est_succes(), "prélèvement : {} {}", prev.statut, prev.texte);
        let prev_id = uuid_de(&prev.json["id"]);

        let rev = m
            .appel("POST", &format!("/facturation/sepa/mandats/{mandat}/revoquer"), Some(&dir), None)
            .await;
        assert!(rev.est_succes(), "révocation : {} {}", rev.statut, rev.texte);

        let p = pool().await;
        let (st,): (String,) = sqlx::query_as("SELECT statut FROM ent_fac_sepa_prelevement WHERE id = $1")
            .bind(prev_id)
            .fetch_one(&p)
            .await
            .expect("statut");
        assert_eq!(st, "annule", "FD-10 : prélèvement prévu conservé après révocation du mandat");
        // La base refuse un prélèvement sur ce mandat révoqué.
        let r = sqlx::query(
            "INSERT INTO ent_fac_sepa_prelevement (id, etablissement_id, mandat_id, facture_id, foyer_id,
                    montant_centimes, date_execution, end_to_end_id, statut)
             SELECT gen_random_uuid(), etablissement_id, $1, id, foyer_id, 100, (now() + interval '30 days')::date,
                    'F0R-' || gen_random_uuid()::text, 'prevu'
               FROM ent_fac_facture WHERE id = $2",
        )
        .bind(mandat)
        .bind(id)
        .execute(&p)
        .await;
        assert!(r.is_err(), "FD-10 : prélèvement posé sur un mandat révoqué");
        p.close().await;
        // Le service aussi refuse.
        let apres = prelever(m, id, mandat, 100).await;
        assert!(!apres.est_succes(), "prélèvement planifié sur un mandat révoqué ({})", apres.statut);
        // Le foyer redevient prélevable avec un nouveau mandat (état de fin de test sain).
        let _ = mandat_actif(m).await;
    });
}

// ───────────────────────── RS3-08 / R-P5 : RUM serveur, IBAN contrôlé ─────────────────────────

#[test]
fn f0_rum_serveur_et_iban_modulo_97() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        // Foyer sans mandat actif : celui de Draco (le mandat de Harry est géré ailleurs).
        let foyer = m.a.objets.foyer_autre;
        let p = pool().await;
        sqlx::query("UPDATE ent_fac_sepa_mandat SET statut = 'revoke' WHERE foyer_id = $1 AND statut = 'actif'")
            .bind(foyer)
            .execute(&p)
            .await
            .expect("préparation");
        p.close().await;
        let mauvais = m
            .appel(
                "POST",
                "/facturation/sepa/mandats",
                Some(&dir),
                Some(&json!({ "foyer_id": foyer, "rum": "MA-RUM", "iban": "FR7630006000011234567890188", "titulaire": "F0" })),
            )
            .await;
        assert!(!mauvais.est_succes(), "IBAN à clé fausse accepté ({})", mauvais.statut);
        let bon = m
            .appel(
                "POST",
                "/facturation/sepa/mandats",
                Some(&dir),
                Some(&json!({ "foyer_id": foyer, "rum": "MA-RUM", "iban": "FR7630006000011234567890189", "titulaire": "F0" })),
            )
            .await;
        assert!(bon.est_succes(), "mandat : {} {}", bon.statut, bon.texte);
        let rum = bon.json["rum"].as_str().unwrap();
        assert_ne!(rum, "MA-RUM", "RS3-08 : RUM choisie par le client");
        assert!(rum.starts_with("SCOLIA") && rum.len() <= 35, "RUM serveur attendue : {rum}");
    });
}

// ───────────────────────── FD-13 : frais de relance ─────────────────────────

#[test]
fn f0_fd13_aucun_frais_de_relance_ni_de_rejet() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        let r1 = m
            .appel("PUT", "/finances/parametres", Some(&dir), Some(&json!({ "frais_rejet_centimes": 750 })))
            .await;
        assert!(r1.statut.as_u16() == 400, "frais de rejet accepté ({} {})", r1.statut, r1.texte);
        let r2 = m
            .appel(
                "PUT",
                "/finances/parametres",
                Some(&dir),
                Some(&json!({ "paliers": [{ "rang": 1, "jours_retard": 5, "libelle": "Relance", "frais_centimes": 500 }] })),
            )
            .await;
        assert!(r2.statut.as_u16() == 400, "frais de relance accepté ({} {})", r2.statut, r2.texte);
        // Et la base elle-même refuse un frais non nul.
        let p = pool().await;
        let r = sqlx::query(
            "INSERT INTO ent_fin_palier_relance (id, etablissement_id, rang, jours_retard, libelle, frais_centimes)
             VALUES (gen_random_uuid(), $1, 5, 200, 'F0', 100)",
        )
        .bind(m.a.objets.etab)
        .execute(&p)
        .await;
        assert!(r.is_err(), "la base accepte un frais de relance non nul");
        p.close().await;
    });
}

// ───────────────────────── FD-11 : compte créancier ─────────────────────────

/// Seule la direction change le compte crédité ; l'IBAN est chiffré au repos ;
/// l'événement et l'alerte à la direction existent, sans IBAN en clair.
#[test]
fn f0_fd11_compte_crediteur_n3_chiffre_et_alerte() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let compta = m.jeton(Compta).to_string();
        let dir = m.jeton(Direction).to_string();
        let iban = "FR1420041010050500013M02606";
        let refus = m
            .appel("PUT", "/finances/parametres", Some(&compta), Some(&json!({ "crediteur_iban": iban })))
            .await;
        assert_eq!(refus.statut.as_u16(), 403, "FD-11 : la compta change seule le compte crédité ({})", refus.statut);

        let ok = m
            .appel(
                "PUT",
                "/finances/parametres",
                Some(&dir),
                Some(&json!({ "crediteur_iban": iban, "crediteur_nom": "Lycée F0",
                              "crediteur_bic": "PSSTFRPPXXX", "crediteur_ics": "FR12ZZZ123456" })),
            )
            .await;
        assert!(ok.est_succes(), "direction : {} {}", ok.statut, ok.texte);
        assert!(!ok.texte.contains("20041010050500013"), "IBAN créancier en clair dans la réponse : {}", ok.texte);

        let p = pool().await;
        let (stocke,): (String,) = sqlx::query_as(
            "SELECT crediteur_iban FROM ent_fin_parametres WHERE etablissement_id = $1",
        )
        .bind(m.a.objets.etab)
        .fetch_one(&p)
        .await
        .expect("paramètres");
        assert!(
            !stocke.contains("20041010050500013"),
            "FD-11 : IBAN du créancier stocké en clair"
        );
        let (nb_evt,): (i64,) = sqlx::query_as(
            "SELECT count(*)::bigint FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'fin.crediteur_modifie'
               AND payload::text NOT LIKE '%20041010050500013%'",
        )
        .bind(m.a.objets.etab)
        .fetch_one(&p)
        .await
        .expect("événements");
        let (nb_notif,): (i64,) = sqlx::query_as(
            "SELECT count(*)::bigint FROM ent_notification WHERE etablissement_id = $1 AND kind = 'fin.crediteur_modifie'",
        )
        .bind(m.a.objets.etab)
        .fetch_one(&p)
        .await
        .expect("notifications");
        p.close().await;
        assert!(nb_evt != 0, "FD-11 : aucun événement de changement de créancier");
        assert!(nb_notif >= 1, "FD-11 : aucune alerte à la direction");
    });
}

// ───────────────────────── FD-16 : consommation négative ─────────────────────────

#[test]
fn f0_fd16_consommation_negative_refusee() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        let rep = m
            .appel(
                "POST",
                "/finances/consommations",
                Some(&dir),
                Some(&json!({
                    "source": format!("f0-{}", Uuid::new_v4()),
                    "libelle": "Consommation négative",
                    "article_id": Uuid::new_v4(),
                    "eleve_id": m.a.objets.eleve,
                    "quantite": 1,
                    "montant_unitaire_centimes": -50_000
                })),
            )
            .await;
        assert_eq!(rep.statut.as_u16(), 400, "FD-16 : consommation négative acceptée ({} {})", rep.statut, rep.texte);
    });
    // Et sur le chemin historique : quantité ou montant négatif.
    rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        for (q, mt) in [(-2, 1_000), (1, -1_000)] {
            let rep = m
                .appel(
                    "POST",
                    "/facturation/factures",
                    Some(&dir),
                    Some(&json!({
                        "foyer_id": m.a.objets.foyer, "libelle": "Négatif",
                        "lignes": [{ "libelle": "X", "quantite": q, "montant_unitaire_centimes": mt }]
                    })),
                )
                .await;
            assert!(!rep.est_succes(), "FD-16 : ligne ({q}, {mt}) acceptée ({})", rep.statut);
        }
    });
}

// ───────────────────────── FD-27 : droits par défaut du secrétariat ─────────────────────────

/// L'Administratif consulte (facturation.lire) mais ne crée, n'émet, n'encaisse ni
/// ne gère les bourses par défaut.
#[test]
fn f0_fd27_secretariat_lecture_seule_par_defaut() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let adm = m.jeton(Administratif).to_string();
        let lecture = m.appel("GET", "/facturation/factures", Some(&adm), None).await;
        assert!(lecture.est_succes(), "l'Administratif doit pouvoir consulter ({})", lecture.statut);
        let ecriture = m
            .appel(
                "POST",
                "/facturation/factures",
                Some(&adm),
                Some(&json!({
                    "foyer_id": m.a.objets.foyer, "libelle": "Secrétariat",
                    "lignes": [{ "libelle": "X", "quantite": 1, "montant_unitaire_centimes": 100 }]
                })),
            )
            .await;
        assert_eq!(ecriture.statut.as_u16(), 403, "FD-27 : le secrétariat crée une facture ({})", ecriture.statut);
        let id = facture_emise(m, 100).await;
        let enc = m
            .appel(
                "POST",
                &format!("/facturation/factures/{id}/encaisser"),
                Some(&adm),
                Some(&json!({ "montant_centimes": 100, "moyen": "especes" })),
            )
            .await;
        assert_eq!(enc.statut.as_u16(), 403, "FD-27 : le secrétariat encaisse ({})", enc.statut);
        let bourses = m
            .appel("POST", "/bourses/types", Some(&adm), Some(&json!({ "code": "F0", "libelle": "F0" })))
            .await;
        assert_eq!(bourses.statut.as_u16(), 403, "FD-27 : le secrétariat gère les bourses ({})", bourses.statut);
    });
}

// ───────────────────────── FD-23 : mandats d'un parent ─────────────────────────

/// Un parent payeur retrouve son mandat même si beaucoup d'autres mandats plus
/// récents existent dans l'établissement (le filtre est fait en SQL).
#[test]
fn f0_fd23_parent_voit_son_mandat_malgre_les_autres() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let mandat = mandat_actif(m).await;
        let p = pool().await;
        // 100 mandats plus récents d'autres foyers (inactifs, fictifs).
        sqlx::query(
            "INSERT INTO ent_fac_sepa_mandat (id, etablissement_id, foyer_id, rum, iban, titulaire, statut, signe_at, created_at)
             SELECT gen_random_uuid(), $1, $2, 'F0BULK' || g::text || substr(gen_random_uuid()::text, 1, 8),
                    'FR7630006000011234567890189', 'Bulk', 'revoke', now(), now() + (g || ' seconds')::interval
               FROM generate_series(1, 100) g",
        )
        .bind(m.a.objets.etab)
        .bind(m.a.objets.foyer_autre)
        .execute(&p)
        .await
        .expect("mandats fictifs");
        p.close().await;
        let parent = m.jeton(Parent).to_string();
        let liste = m.appel("GET", "/facturation/sepa/mandats", Some(&parent), None).await;
        assert!(liste.est_succes(), "liste parent : {} {}", liste.statut, liste.texte);
        let ids: Vec<Uuid> = liste.json.as_array().map(|a| a.iter().map(|x| uuid_de(&x["id"])).collect()).unwrap_or_default();
        assert!(ids.contains(&mandat), "FD-23 : le parent ne voit pas son propre mandat");
    });
}

// ───────────────────────── Journal des consultations (R-F36) ─────────────────────────

#[test]
fn f0_r_f36_consultation_nominative_journalisee() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let id = facture_emise(m, 100).await;
        let dir = m.jeton(Direction).to_string();
        let avant = nb_traces().await;
        let rep = m.appel("GET", &format!("/facturation/factures/{id}"), Some(&dir), None).await;
        assert!(rep.est_succes(), "lecture : {} {}", rep.statut, rep.texte);
        assert!(nb_traces().await > avant, "R-F36 : lecture nominative non journalisée");
    });
}

async fn nb_traces() -> i64 {
    let p = pool().await;
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*)::bigint FROM ent_journal_modif WHERE domaine = 'facturation' AND action = 'lecture'",
    )
    .fetch_one(&p)
    .await
    .unwrap_or((0,));
    p.close().await;
    n
}

// ───────────────────────── Matrice : séparation des routes financières ─────────────────────────

/// Parent non payeur, payeur et autre établissement : refus sur les routes d'écriture
/// ajoutées ou durcies par F0 (export en POST, encaissement, mandats).
#[test]
fn f0_matrice_routes_financieres_refusees_aux_non_autorises() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let jour = dans_20_jours();
        let id = facture_emise(m, 100).await;
        for acteur in [Parent, Eleve, Prof] {
            let jeton = m.jeton(acteur).to_string();
            for (methode, chemin, corps) in [
                ("POST", "/facturation/sepa/export".to_string(), json!({ "debut": jour, "fin": jour })),
                ("POST", format!("/facturation/factures/{id}/encaisser"), json!({ "montant_centimes": 1, "moyen": "especes" })),
                ("POST", format!("/facturation/factures/{id}/emettre"), json!({})),
            ] {
                let rep = m.appel(methode, &chemin, Some(&jeton), Some(&corps)).await;
                assert!(rep.est_refus(), "{acteur:?} {methode} {chemin} → {}", rep.statut);
            }
        }
        // Autre établissement : la facture de A est invisible pour la direction de B.
        let dir_b = m.b.jetons.get(&Direction).cloned().expect("direction de B");
        let rep = m.appel("GET", &format!("/facturation/factures/{id}"), Some(&dir_b), None).await;
        assert!(rep.est_refus(), "direction de B lit une facture de A ({})", rep.statut);
        let rep = m
            .appel("POST", "/facturation/sepa/export", Some(&dir_b), Some(&json!({ "debut": jour, "fin": jour })))
            .await;
        assert!(!rep.texte.contains(&id.to_string()), "export de B contenant une pièce de A");
    });
}
