//! Finances, lot F1 (MIG-3) — **test d'or des routes historiques** `/facturation/*`.
//!
//! Les routes historiques deviennent des adaptateurs du service unique : leur comportement ne
//! change pas (hors corrections listées dans `docs/socles/24_FINANCES_F1_F2_LIVRAISON.md`). Ce test
//! rejoue un scénario de bout en bout, enregistre **statut et corps JSON** de chaque appel
//! (identifiants, horodatages, numéros de série et RUM remplacés par des marqueurs stables) et les
//! compare à `tests/or/facturation_historique.json`, produit sur le code d'avant F1.
//!
//! * Comparaison : `cargo test -p ent-api --features ent-api/labo --test or_facturation_historique`.
//! * Régénération (une correction listée **seulement**) : `ENT_OR_REGENERER=1` avant la commande.
//! * Sans `ENT_DATABASE_URL` : la comparaison est ignorée proprement (« SKIP », comme les autres
//!   recettes) ; en CI la base existe.
//!
//! Aucune dépendance à l'heure réelle : toutes les dates du scénario sont fixes (2030).

#![allow(dead_code)]

#[path = "separation/harness.rs"]
mod harness;
#[path = "separation/routes_src.rs"]
mod routes_src;

use harness::*;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use uuid::Uuid;

use Acteur::*;

const FICHIER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/or/facturation_historique.json"
);

/// Normalise un JSON : identifiants → `<uuid:n>` (par ordre d'apparition), horodatages et dates →
/// marqueurs, numéros de série, RUM et références de bout en bout → marqueurs.
struct Normaliseur {
    uuids: BTreeMap<String, usize>,
}

impl Normaliseur {
    fn new() -> Self {
        Self { uuids: BTreeMap::new() }
    }

    fn texte(&mut self, s: &str) -> String {
        if Uuid::parse_str(s).is_ok() {
            let n = self.uuids.len();
            let i = *self.uuids.entry(s.to_string()).or_insert(n);
            return format!("<uuid:{i}>");
        }
        let b = s.as_bytes();
        let chiffres = |a: usize, z: usize| b.len() >= z && b[a..z].iter().all(u8::is_ascii_digit);
        // 2030-01-05T10:11:12.123+00:00
        if b.len() >= 19 && chiffres(0, 4) && b[4] == b'-' && b[10] == b'T' {
            return "<ts>".into();
        }
        // 2030-01-05 : date du scénario (fixe) conservée ; date du jour : le scénario n'en émet pas.
        // F2030-000012 / A2026-000003 : numéro de série (dépend de l'historique de la base de test).
        if b.len() == 12 && (b[0] == b'F' || b[0] == b'A') && chiffres(1, 5) && b[5] == b'-' && chiffres(6, 12) {
            return "<numero>".into();
        }
        if s.starts_with("BRO-") {
            return "<provisoire>".into();
        }
        if s.starts_with("SCOLIA") || s.starts_with("E2E") || s.starts_with("INS-") {
            return "<ref>".into();
        }
        // Référence de bout en bout d'un prélèvement : contient le numéro de pièce et un aléa.
        if s.starts_with("SEPA-") {
            return "<e2e>".into();
        }
        s.to_string()
    }

    fn valeur(&mut self, v: &Value) -> Value {
        match v {
            Value::String(s) => Value::String(self.texte(s)),
            Value::Array(a) => Value::Array(a.iter().map(|x| self.valeur(x)).collect()),
            Value::Object(o) => {
                let mut m = Map::new();
                for (k, x) in o {
                    m.insert(k.clone(), self.valeur(x));
                }
                Value::Object(m)
            }
            autre => autre.clone(),
        }
    }
}

/// Résumé d'une liste partagée avec d'autres tests : statut, nombre ≥ 1 et **clés** du premier
/// élément (le contenu dépend de ce que les autres tests ont créé).
fn forme_liste(rep: &Rep) -> Value {
    let cles = |v: &Value| -> Vec<String> {
        v.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default()
    };
    match &rep.json {
        Value::Array(a) => json!({
            "statut": rep.statut.as_u16(),
            "non_vide": !a.is_empty(),
            "cles": a.first().map(cles).unwrap_or_default(),
        }),
        autre => json!({ "statut": rep.statut.as_u16(), "corps": autre }),
    }
}

/// Une réponse d'objet : statut + JSON normalisé.
fn objet(n: &mut Normaliseur, rep: &Rep) -> Value {
    json!({ "statut": rep.statut.as_u16(), "json": n.valeur(&rep.json) })
}

/// Une réponse binaire ou texte : statut et gabarit du début seulement.
fn binaire(rep: &Rep, prefixe: usize) -> Value {
    json!({
        "statut": rep.statut.as_u16(),
        "debut": rep.texte.chars().take(prefixe).collect::<String>(),
        "non_vide": !rep.texte.is_empty(),
    })
}

fn uuid_de(v: &Value) -> Uuid {
    Uuid::parse_str(v.as_str().unwrap_or_else(|| panic!("uuid attendu : {v}"))).expect("uuid")
}

/// Identifiant d'une réponse de création ; un échec dit quelle étape a répondu quoi.
fn id_de(rep: &Rep, etape: &str, chemin: &[&str]) -> Uuid {
    let mut v = &rep.json;
    for c in chemin {
        v = &v[*c];
    }
    match v.as_str().and_then(|s| Uuid::parse_str(s).ok()) {
        Some(u) => u,
        None => panic!("étape « {etape} » : identifiant absent (statut {} — {})", rep.statut, rep.texte),
    }
}

async fn creer(m: &Monde, jeton: &str, lignes: Value) -> Rep {
    m.appel(
        "POST",
        "/facturation/factures",
        Some(jeton),
        Some(&json!({
            "foyer_id": m.a.objets.foyer,
            "libelle": "Scénario d'or F1",
            "lignes": lignes,
        })),
    )
    .await
}

/// Le scénario complet ; renvoie le journal `{ étape: réponse normalisée }`.
async fn scenario(m: &Monde) -> Value {
    let dir = m.jeton(Direction).to_string();
    let parent = m.jeton(Parent).to_string();
    let prof = m.jeton(Prof).to_string();
    let autre = m.jeton(ParentAutreFoyer).to_string();
    let mut n = Normaliseur::new();
    let mut j = Map::new();
    let mut note = |nom: &str, v: Value| {
        j.insert(nom.to_string(), v);
    };

    // 1. Création, lecture, émission.
    let rep = creer(
        m,
        &dir,
        json!([
            { "libelle": "Cantine novembre", "quantite": 2, "montant_unitaire_centimes": 1250 },
            { "libelle": "Sortie scolaire", "quantite": 1, "montant_unitaire_centimes": 3000 }
        ]),
    )
    .await;
    note("creer", objet(&mut n, &rep));
    let f1 = id_de(&rep, "créer", &["facture", "id"]);
    note("creer_refuse_prof", json!(creer(m, &prof, json!([])).await.statut.as_u16()));
    let rep = m.appel("GET", &format!("/facturation/factures/{f1}"), Some(&dir), None).await;
    note("detail_brouillon", objet(&mut n, &rep));
    let rep = m.appel("GET", &format!("/facturation/factures/{f1}"), Some(&parent), None).await;
    note("detail_brouillon_parent", json!(rep.statut.as_u16()));
    let rep = m.appel("POST", &format!("/facturation/factures/{f1}/emettre"), Some(&dir), None).await;
    note("emettre", objet(&mut n, &rep));
    let rep = m.appel("POST", &format!("/facturation/factures/{f1}/emettre"), Some(&dir), None).await;
    note("emettre_deux_fois", json!(rep.statut.as_u16()));
    let rep = m.appel("GET", &format!("/facturation/factures/{f1}"), Some(&parent), None).await;
    note("detail_emise_parent", objet(&mut n, &rep));
    let rep = m.appel("GET", &format!("/facturation/factures/{f1}"), Some(&autre), None).await;
    note("detail_emise_autre_foyer", json!(rep.statut.as_u16()));
    let rep = m.appel("GET", &format!("/facturation/factures/{f1}/pdf"), Some(&dir), None).await;
    note("pdf", binaire(&rep, 5));

    // 2. Échéancier libre sur la facture 1 (5 500 = 3 000 + 2 500), puis refus d'un second.
    let rep = m
        .appel(
            "POST",
            &format!("/facturation/factures/{f1}/echeancier"),
            Some(&dir),
            Some(&json!({ "echeances": [
                { "libelle": "Acompte", "montant_centimes": 3000, "date_echeance": "2030-01-05" },
                { "libelle": "Solde", "montant_centimes": 2500, "date_echeance": "2030-02-05" }
            ]})),
        )
        .await;
    note("echeancier", objet(&mut n, &rep));
    let rep = m
        .appel(
            "POST",
            &format!("/facturation/factures/{f1}/echeancier"),
            Some(&dir),
            Some(&json!({ "echeances": [
                { "libelle": "Total", "montant_centimes": 5500, "date_echeance": "2030-03-05" }
            ]})),
        )
        .await;
    note("echeancier_second", json!(rep.statut.as_u16()));

    // 3. Encaissements : partiel puis solde ; quittances côté famille.
    let rep = m
        .appel(
            "POST",
            &format!("/facturation/factures/{f1}/encaisser"),
            Some(&dir),
            Some(&json!({ "montant_centimes": 1000, "moyen": "especes" })),
        )
        .await;
    note("encaisser_partiel", objet(&mut n, &rep));
    let rep = m
        .appel(
            "POST",
            &format!("/facturation/factures/{f1}/encaisser"),
            Some(&dir),
            Some(&json!({ "montant_centimes": 999_999, "moyen": "cheque", "reference": "CHQ-1" })),
        )
        .await;
    note("encaisser_depassement", json!(rep.statut.as_u16()));
    let rep = m
        .appel(
            "POST",
            &format!("/facturation/factures/{f1}/encaisser"),
            Some(&dir),
            Some(&json!({ "montant_centimes": 4500, "moyen": "virement", "reference": "VIR-1" })),
        )
        .await;
    note("encaisser_solde", objet(&mut n, &rep));
    let quittance = rep.json["quittances"][0]["id"].as_str().map(|s| Uuid::parse_str(s).unwrap());
    let rep = m.appel("GET", "/facturation/famille", Some(&parent), None).await;
    note("famille", forme_liste(&rep));
    let rep = m.appel("GET", "/facturation/famille/quittances", Some(&parent), None).await;
    note("famille_quittances", forme_liste(&rep));
    if let Some(q) = quittance {
        let rep = m.appel("GET", &format!("/facturation/quittances/{q}"), Some(&parent), None).await;
        note("quittance", objet(&mut n, &rep));
        let rep = m.appel("GET", &format!("/facturation/quittances/{q}/pdf"), Some(&parent), None).await;
        note("quittance_pdf", binaire(&rep, 5));
    }

    // 4. Échéancier en parts sur une seconde facture (3 000 en 3 parts, le reste sur la première).
    let rep = creer(m, &dir, json!([{ "libelle": "Fournitures", "quantite": 1, "montant_unitaire_centimes": 3001 }])).await;
    let f2 = id_de(&rep, "créer f2", &["facture", "id"]);
    m.appel("POST", &format!("/facturation/factures/{f2}/emettre"), Some(&dir), None).await;
    let rep = m
        .appel(
            "POST",
            &format!("/facturation/factures/{f2}/echeancier/parts"),
            Some(&dir),
            Some(&json!({ "dates": ["2030-03-05", "2030-04-05", "2030-05-05"] })),
        )
        .await;
    note("echeancier_parts", objet(&mut n, &rep));

    // 5. SEPA : mandat, prélèvement par facture (sans échéancier), export, exécution, rejet, révocation.
    // Un mandat actif du foyer (monde partagé) serait refusé : on le révoque d'abord (étape hors or).
    let liste = m.appel("GET", "/facturation/sepa/mandats", Some(&dir), None).await;
    for x in liste.json.as_array().cloned().unwrap_or_default() {
        if x["foyer_id"].as_str() == Some(&m.a.objets.foyer.to_string()) && x["statut"].as_str() == Some("actif") {
            let a_revoquer = uuid_de(&x["id"]);
            m.appel("POST", &format!("/facturation/sepa/mandats/{a_revoquer}/revoquer"), Some(&dir), None).await;
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
                "titulaire": "Scénario d'or"
            })),
        )
        .await;
    note("mandat_creer", objet(&mut n, &rep));
    let mandat = id_de(&rep, "mandat", &["id"]);
    let rep = m.appel("GET", "/facturation/sepa/mandats", Some(&dir), None).await;
    note("mandats_liste", forme_liste(&rep));
    let rep = m.appel("GET", "/facturation/sepa/mandats", Some(&parent), None).await;
    note("mandats_parent", forme_liste(&rep));
    let rep = m.appel("GET", "/facturation/sepa/mandats", Some(&prof), None).await;
    note("mandats_prof", json!(rep.statut.as_u16()));

    let mut prelev = Vec::new();
    for (i, montant) in [1000_i64, 1500].into_iter().enumerate() {
        let rep = creer(m, &dir, json!([{ "libelle": format!("Prélevée {i}"), "quantite": 1, "montant_unitaire_centimes": 4000 }])).await;
        let f = id_de(&rep, "créer prélevée", &["facture", "id"]);
        m.appel("POST", &format!("/facturation/factures/{f}/emettre"), Some(&dir), None).await;
        let rep = m
            .appel(
                "POST",
                &format!("/facturation/factures/{f}/sepa/prelever"),
                Some(&dir),
                Some(&json!({ "mandat_id": mandat, "montant_centimes": montant, "date_execution": "2030-06-05" })),
            )
            .await;
        note(&format!("prelever_{i}"), objet(&mut n, &rep));
        prelev.push(id_de(&rep, "prélever", &["id"]));
    }
    // Prélèvement sur une facture qui a des échéances : refusé (FD-04, F0).
    let rep = m
        .appel(
            "POST",
            &format!("/facturation/factures/{f2}/sepa/prelever"),
            Some(&dir),
            Some(&json!({ "mandat_id": mandat, "montant_centimes": 500, "date_execution": "2030-06-05" })),
        )
        .await;
    note("prelever_facture_a_echeances", json!(rep.statut.as_u16()));
    let rep = m
        .appel(
            "POST",
            "/facturation/sepa/export",
            Some(&dir),
            Some(&json!({ "debut": "2030-06-01", "fin": "2030-06-30", "format": "csv" })),
        )
        .await;
    note("export_sepa", binaire(&rep, 0));
    let rep = m
        .appel(
            "POST",
            &format!("/facturation/sepa/prelevements/{}/executer", prelev[0]),
            Some(&dir),
            None,
        )
        .await;
    note("executer", objet(&mut n, &rep));
    let rep = m
        .appel(
            "POST",
            &format!("/facturation/sepa/prelevements/{}/executer", prelev[0]),
            Some(&dir),
            None,
        )
        .await;
    note("executer_deux_fois", json!(rep.statut.as_u16()));
    let rep = m
        .appel(
            "POST",
            &format!("/facturation/sepa/prelevements/{}/rejeter", prelev[1]),
            Some(&dir),
            Some(&json!({ "motif": "AC04" })),
        )
        .await;
    note("rejeter", objet(&mut n, &rep));
    let rep = m.appel("POST", &format!("/facturation/sepa/mandats/{mandat}/revoquer"), Some(&dir), None).await;
    note("mandat_revoquer", objet(&mut n, &rep));
    let rep = m
        .appel("POST", "/facturation/banque/rapprocher", Some(&dir), Some(&json!({ "csv": "" })))
        .await;
    note("rapprocher_vide", json!(rep.statut.as_u16()));

    // 6. Rappels, export OGEC, refus de la famille et d'un autre établissement.
    let rep = m.appel("GET", "/facturation/rappels", Some(&dir), None).await;
    note("rappels", forme_liste(&rep));
    let rep = m.appel("POST", "/facturation/rappels", Some(&dir), None).await;
    note("rappels_lancer", json!(rep.statut.as_u16()));
    let rep = m
        .appel("GET", "/facturation/export/ogec?debut=2030-01-01&fin=2030-12-31", Some(&dir), None)
        .await;
    note("export_ogec", binaire(&rep, 0));
    let rep = m.appel("GET", "/facturation/factures", Some(&parent), None).await;
    note("liste_parent_refusee", json!(rep.statut.as_u16()));
    let rep = m.appel("GET", "/facturation/factures", Some(&dir), None).await;
    note("liste_direction", forme_liste(&rep));
    let rep = m
        .appel("GET", &format!("/facturation/factures/{}", Uuid::new_v4()), Some(&dir), None)
        .await;
    note("detail_inconnu", json!(rep.statut.as_u16()));
    let rep = m
        .appel("POST", &format!("/facturation/factures/{}/emettre", Uuid::new_v4()), Some(&dir), None)
        .await;
    note("emettre_inconnu", json!(rep.statut.as_u16()));
    let rep = m
        .appel(
            "POST",
            &format!("/facturation/factures/{}/encaisser", Uuid::new_v4()),
            Some(&dir),
            Some(&json!({ "montant_centimes": 100, "moyen": "especes" })),
        )
        .await;
    note("encaisser_inconnu", json!(rep.statut.as_u16()));
    Value::Object(j)
}

#[test]
fn or_routes_historiques_facturation() {
    let Some(m) = monde() else { return };
    let obtenu = rt().block_on(scenario(m));
    let texte = serde_json::to_string_pretty(&obtenu).expect("json") + "\n";
    if std::env::var("ENT_OR_REGENERER").as_deref() == Ok("1") {
        std::fs::create_dir_all(std::path::Path::new(FICHIER).parent().unwrap()).unwrap();
        std::fs::write(FICHIER, &texte).expect("écriture de l'or");
        eprintln!("or régénéré : {FICHIER}");
        return;
    }
    let attendu = std::fs::read_to_string(FICHIER)
        .unwrap_or_else(|e| panic!("or absent ({FICHIER}) : {e} — le produire avec ENT_OR_REGENERER=1"));
    if attendu != texte {
        // Différence étape par étape : le message nomme les routes qui ont changé.
        let a: Value = serde_json::from_str(&attendu).expect("or JSON");
        let mut ecarts = Vec::new();
        if let (Some(a), Some(o)) = (a.as_object(), obtenu.as_object()) {
            for (k, v) in a {
                if o.get(k) != Some(v) {
                    ecarts.push(format!("{k}\n  or      : {v}\n  obtenu  : {}", o.get(k).cloned().unwrap_or(Value::Null)));
                }
            }
            for k in o.keys().filter(|k| !a.contains_key(*k)) {
                ecarts.push(format!("{k} : étape absente de l'or"));
            }
        }
        panic!("les routes historiques ont changé de comportement :\n{}", ecarts.join("\n"));
    }
}
