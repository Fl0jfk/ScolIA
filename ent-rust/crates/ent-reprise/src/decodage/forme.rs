//! Formes JSON attendues des colonnes `jsonb` du Next.js (décodage strict, R-17 de T0).
//!
//! Chaque colonne `jsonb` d'une table décodée a une [`Forme`] déclarée ici, d'après les types TypeScript
//! du Next.js (`db/*.ts`). Une clé inconnue, un type inattendu ou une valeur hors énumération est un
//! refus **rapporté** : jamais ignoré, jamais « rangé en vrac ». Les colonnes dont le Next.js ne type pas
//! le contenu (`Record<string, unknown>`) ont une forme **fermée et vide** : tout contenu non vide est
//! refusé et ses noms de clés sont listés, ce qui produit le catalogue à compléter à partir de l'export
//! réel anonymisé. Seuls les instantanés d'audit figés (journaux, jamais relus comme source de vérité,
//! exception D-036) sont [`Forme::Instantane`] : JSON valide, borné, archivé tel quel.

use super::cellule::{refus, Refus};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Champ {
    pub nom: &'static str,
    pub forme: Forme,
    pub requis: bool,
}

#[derive(Debug, Clone)]
pub enum Forme {
    Texte,
    Nombre,
    Booleen,
    Nul,
    Enum(&'static [&'static str]),
    Tableau(Box<Forme>),
    /// Objet fermé : seules les clés déclarées sont admises.
    Objet(Vec<Champ>),
    /// Dictionnaire à clés libres, valeurs de la forme donnée.
    Dictionnaire(Box<Forme>),
    /// L'une des formes.
    Union(Vec<Forme>),
    /// Instantané figé (journal) : tout JSON borné (profondeur 12, 1 Mio) est archivé sans être interprété.
    Instantane,
}

pub const PROFONDEUR_MAX: usize = 12;
pub const TAILLE_INSTANTANE_MAX: usize = 1 << 20;

fn req(nom: &'static str, forme: Forme) -> Champ {
    Champ { nom, forme, requis: true }
}
fn opt(nom: &'static str, forme: Forme) -> Champ {
    Champ { nom, forme, requis: false }
}
fn tab(f: Forme) -> Forme {
    Forme::Tableau(Box::new(f))
}
fn ou_nul(f: Forme) -> Forme {
    Forme::Union(vec![f, Forme::Nul])
}
fn dico(f: Forme) -> Forme {
    Forme::Dictionnaire(Box::new(f))
}
/// Objet fermé et vide : tout contenu est à cataloguer.
fn a_cataloguer() -> Forme {
    Forme::Objet(Vec::new())
}
/// `string | string[] | boolean | null` (valeurs d'un formulaire famille).
fn valeur_formulaire() -> Forme {
    Forme::Union(vec![Forme::Texte, tab(Forme::Texte), Forme::Booleen, Forme::Nul])
}

fn document_appel() -> Forme {
    Forme::Objet(vec![
        req("label", Forme::Texte),
        opt("reference", Forme::Texte),
        opt("statut", Forme::Enum(&["attendu", "recu"])),
        opt("receivedAt", Forme::Texte),
        opt("notes", Forme::Texte),
    ])
}

fn visa_sst() -> Forme {
    Forme::Objet(vec![
        req("notes", Forme::Texte),
        req("userId", Forme::Texte),
        req("name", Forme::Texte),
        req("at", Forme::Texte),
        opt("signaturePngBase64", ou_nul(Forme::Texte)),
    ])
}

fn champ_catalogue() -> Forme {
    Forme::Objet(vec![
        req("id", Forme::Texte),
        req(
            "type",
            Forme::Enum(&[
                "select",
                "multiselect",
                "text",
                "textarea",
                "checkbox",
                "etablissement",
                "etablissement_multi",
                "wizard_branch",
            ]),
        ),
        req("label", Forme::Texte),
        opt("required", Forme::Booleen),
        opt("optionsFrom", Forme::Enum(&["destinations", "options"])),
        opt(
            "inlineOptions",
            tab(Forme::Objet(vec![req("id", Forme::Texte), req("label", Forme::Texte)])),
        ),
        opt("helpText", Forme::Texte),
        opt(
            "showWhen",
            Forme::Objet(vec![
                req("fieldId", Forme::Texte),
                req("equals", Forme::Union(vec![Forme::Texte, tab(Forme::Texte)])),
            ]),
        ),
        opt("maxSelect", Forme::Nombre),
    ])
}

fn catalogue_choix() -> Forme {
    Forme::Objet(vec![
        req(
            "destinations",
            tab(Forme::Objet(vec![
                req("id", Forme::Texte),
                req("label", Forme::Texte),
                opt("niveauCible", Forme::Texte),
                opt("interne", Forme::Booleen),
            ])),
        ),
        req(
            "options",
            tab(Forme::Objet(vec![
                req("id", Forme::Texte),
                req("label", Forme::Texte),
                req("kind", Forme::Enum(&["lv", "option_interne", "specialite", "autre"])),
            ])),
        ),
        req("fields", tab(champ_catalogue())),
        opt(
            "voiesOuverture",
            tab(Forme::Objet(vec![
                req("niveauActuel", Forme::Texte),
                req("destinationsIds", tab(Forme::Texte)),
            ])),
        ),
    ])
}

/// Forme attendue de la colonne `table.colonne`, si elle est déclarée.
pub fn forme_colonne(table: &str, colonne: &str, nullable: bool) -> Option<Forme> {
    let f = match (table, colonne) {
        ("eleve", "options") => tab(Forme::Texte),
        ("eleve_scolarite", "grille_repas") => a_cataloguer(),
        ("famille_messaging_settings", "roles_can_initiate") => tab(Forme::Texte),
        ("fd_campagne", "catalogue") => catalogue_choix(),
        ("fd_campagne", "appel_config") => Forme::Objet(vec![
            req("enabled", Forme::Booleen),
            opt("dateLimite", Forme::Texte),
            opt("procedureHtml", Forme::Texte),
            opt("documentsLabels", tab(Forme::Texte)),
            opt("contactPpLabel", Forme::Texte),
        ]),
        ("fd_campagne", "classes_cibles") | ("fd_campagne", "eleve_ids_cibles") => tab(Forme::Texte),
        ("fd_fiche", "options_actuelles") | ("fd_fiche", "parent_emails") => tab(Forme::Texte),
        ("fd_fiche", "acceptation") => Forme::Objet(vec![req("accepte", Forme::Booleen), opt("motifRefus", Forme::Texte)]),
        ("fd_fiche", "appel") => Forme::Objet(vec![
            opt("commissionLabel", Forme::Texte),
            opt("saisineDate", Forme::Texte),
            opt("commissionDate", Forme::Texte),
            opt("receivedAt", Forme::Texte),
            opt("decision", Forme::Enum(&["maintien", "reformation"])),
            opt("motif", Forme::Texte),
            opt("destinationProposee", Forme::Texte),
            opt("optionsProposees", tab(Forme::Texte)),
            opt("informations", Forme::Texte),
            opt("documents", tab(document_appel())),
            opt("decidedAt", Forme::Texte),
            opt("closedAt", Forme::Texte),
            opt("notifiedAt", Forme::Texte),
            opt("updatedAt", Forme::Texte),
            opt("updatedBy", Forme::Texte),
        ]),
        ("fd_fiche", "parent_accord") | ("fd_fiche", "conflict_payload") => Forme::Objet(vec![
            opt("deposantEmail", Forme::Texte),
            opt("deposantLabel", Forme::Texte),
            opt("depositedAt", Forme::Texte),
            opt("autreEmail", Forme::Texte),
            opt("statutAutre", Forme::Enum(&["en_attente", "confirme", "tacite", "contredit"])),
            opt("confirmeAt", Forme::Texte),
            opt("taciteAt", Forme::Texte),
            opt("accordDeadlineAt", Forme::Texte),
            opt("conflictMotif", Forme::Texte),
            opt("conflictValues", dico(valeur_formulaire())),
            opt("resolution", Forme::Enum(&["maintien_famille", "avis_conseil", "tacite"])),
            opt("resolutionNote", Forme::Texte),
            opt("resolvedAt", Forme::Texte),
            opt("resolvedBy", Forme::Texte),
            opt("conflictNotifiedAt", Forme::Texte),
            opt("taciteNotifiedAt", Forme::Texte),
        ]),
        ("fd_reponse", "payload") => Forme::Objet(vec![
            req("values", dico(valeur_formulaire())),
            opt("comment", Forme::Texte),
            opt("forceMalgreAvis", Forme::Booleen),
            opt(
                "etablissementsVoeux",
                tab(Forme::Objet(vec![
                    req("rang", Forme::Nombre),
                    req("codeRne", Forme::Texte),
                    req("label", Forme::Texte),
                    opt("chezNous", Forme::Booleen),
                ])),
            ),
        ]),
        ("partenariat_offre", "cycles") | ("partenariat_offre", "niveaux") => tab(Forme::Texte),
        ("partenariat_offre", "cta_links") => tab(Forme::Objet(vec![req("label", Forme::Texte), req("url", Forme::Texte)])),
        ("partenariat_offre", "tarifs") => tab(Forme::Objet(vec![
            req("label", Forme::Texte),
            opt("amountCents", ou_nul(Forme::Nombre)),
            opt("amountLabel", Forme::Texte),
            opt("note", Forme::Texte),
        ])),
        ("personnel", "payload") | ("preinscription", "payload") | ("ref_nomenclature", "metadata_json") => a_cataloguer(),
        ("portes_ouvertes_slot_staff", "meta") => dico(Forme::Texte),
        ("sst_fiche", "workflow") => Forme::Objet(vec![
            opt("responsable", visa_sst()),
            opt("cse", visa_sst()),
            opt("decision", visa_sst()),
            opt("realisation", visa_sst()),
            opt("closedAt", ou_nul(Forme::Texte)),
        ]),
        ("data_access_audit", "metadata") | ("eleve_access_audit", "metadata") | ("security_audit_log", "metadata") | ("metier_event", "payload") => {
            Forme::Instantane
        }
        _ => return None,
    };
    Some(if nullable { ou_nul(f) } else { f })
}

/// Colonnes dont la forme est « à cataloguer » (objet fermé vide) : le rapport le signale.
pub fn est_a_cataloguer(table: &str, colonne: &str) -> bool {
    matches!(
        forme_colonne(table, colonne, false),
        Some(Forme::Objet(c)) if c.is_empty()
    )
}

/// Nom de clé citable dans un rapport : identifiant de structure seulement.
fn nom_sur(k: &str) -> String {
    let ok = !k.is_empty()
        && k.len() <= 40
        && k.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ok {
        k.to_string()
    } else {
        "(clé non citable)".to_string()
    }
}

/// Valide `v` contre `forme`. Retourne les refus (au plus 20 par valeur).
pub fn valider(forme: &Forme, v: &Value) -> Vec<Refus> {
    let mut out = Vec::new();
    if matches!(forme, Forme::Instantane) {
        if let Err(r) = valider_instantane(v) {
            out.push(r);
        }
        return out;
    }
    valider_rec(forme, v, "$", 0, &mut out);
    out
}

fn valider_instantane(v: &Value) -> Result<(), Refus> {
    fn prof(v: &Value) -> usize {
        match v {
            Value::Array(a) => 1 + a.iter().map(prof).max().unwrap_or(0),
            Value::Object(o) => 1 + o.values().map(prof).max().unwrap_or(0),
            _ => 0,
        }
    }
    if !matches!(v, Value::Object(_) | Value::Null) {
        return Err(refus("json_type", "instantané : objet ou null attendu"));
    }
    if prof(v) > PROFONDEUR_MAX {
        return Err(refus("json_profondeur", "instantané : profondeur au-delà de 12"));
    }
    if v.to_string().len() > TAILLE_INSTANTANE_MAX {
        return Err(refus("json_taille", "instantané : taille au-delà de 1 Mio"));
    }
    Ok(())
}

fn nature(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "booléen",
        Value::Number(_) => "nombre",
        Value::String(_) => "texte",
        Value::Array(_) => "tableau",
        Value::Object(_) => "objet",
    }
}

fn valider_rec(forme: &Forme, v: &Value, chemin: &str, prof: usize, out: &mut Vec<Refus>) {
    if out.len() >= 20 {
        return;
    }
    if prof > PROFONDEUR_MAX {
        out.push(refus("json_profondeur", format!("{chemin} : profondeur au-delà de 12")));
        return;
    }
    let mauvais_type = |attendu: &str, out: &mut Vec<Refus>| {
        out.push(refus(
            "json_type",
            format!("{chemin} : {attendu} attendu, {} reçu", nature(v)),
        ));
    };
    match forme {
        Forme::Texte => {
            if !v.is_string() {
                mauvais_type("texte", out);
            }
        }
        Forme::Nombre => {
            if !v.is_number() {
                mauvais_type("nombre", out);
            }
        }
        Forme::Booleen => {
            if !v.is_boolean() {
                mauvais_type("booléen", out);
            }
        }
        Forme::Nul => {
            if !v.is_null() {
                mauvais_type("null", out);
            }
        }
        Forme::Enum(valeurs) => match v.as_str() {
            Some(s) if valeurs.contains(&s) => {}
            Some(_) => out.push(refus("json_enum", format!("{chemin} : valeur hors énumération"))),
            None => mauvais_type("texte (énumération)", out),
        },
        Forme::Tableau(f) => match v.as_array() {
            Some(a) => {
                for (i, e) in a.iter().enumerate() {
                    valider_rec(f, e, &format!("{chemin}[{i}]"), prof + 1, out);
                }
            }
            None => mauvais_type("tableau", out),
        },
        Forme::Dictionnaire(f) => match v.as_object() {
            Some(o) => {
                for e in o.values() {
                    valider_rec(f, e, &format!("{chemin}{{}}"), prof + 1, out);
                }
            }
            None => mauvais_type("objet", out),
        },
        Forme::Objet(champs) => match v.as_object() {
            Some(o) => {
                for k in o.keys() {
                    if !champs.iter().any(|c| c.nom == k) {
                        out.push(refus(
                            "json_cle_inconnue",
                            format!("{chemin} : clé inconnue « {} »", nom_sur(k)),
                        ));
                    }
                }
                for c in champs {
                    match o.get(c.nom) {
                        Some(x) => valider_rec(&c.forme, x, &format!("{chemin}.{}", c.nom), prof + 1, out),
                        None if c.requis => out.push(refus(
                            "json_cle_manquante",
                            format!("{chemin} : clé obligatoire « {} » absente", c.nom),
                        )),
                        None => {}
                    }
                }
            }
            None => mauvais_type("objet", out),
        },
        Forme::Union(formes) => {
            // Une seule forme non nulle : on valide contre elle (messages précis, clés inconnues listées).
            let non_nulles: Vec<&Forme> = formes.iter().filter(|f| !matches!(f, Forme::Nul)).collect();
            if non_nulles.len() == 1 && formes.len() == 2 && !v.is_null() {
                valider_rec(non_nulles[0], v, chemin, prof, out);
                return;
            }
            if !formes.iter().any(|f| {
                let mut tmp = Vec::new();
                valider_rec(f, v, chemin, prof, &mut tmp);
                tmp.is_empty()
            }) {
                out.push(refus("json_type", format!("{chemin} : {} ne correspond à aucune forme admise", nature(v))));
            }
        }
        Forme::Instantane => {
            if let Err(r) = valider_instantane(v) {
                out.push(r);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn codes(t: &str, c: &str, v: Value) -> Vec<&'static str> {
        let f = forme_colonne(t, c, false).unwrap();
        valider(&f, &v).iter().map(|r| r.code).collect()
    }

    #[test]
    fn objet_ferme_refuse_cle_inconnue_et_type() {
        assert!(codes("fd_campagne", "appel_config", json!({"enabled": true, "dateLimite": "2027-06-01"})).is_empty());
        assert_eq!(codes("fd_campagne", "appel_config", json!({"enabled": true, "x": 1})), ["json_cle_inconnue"]);
        assert_eq!(codes("fd_campagne", "appel_config", json!({})), ["json_cle_manquante"]);
        assert_eq!(codes("fd_campagne", "appel_config", json!({"enabled": "oui"})), ["json_type"]);
    }

    #[test]
    fn enumeration_et_tableaux() {
        assert_eq!(codes("fd_fiche", "appel", json!({"decision": "autre"})), ["json_enum"]);
        assert!(codes("fd_fiche", "appel", json!({"decision": "maintien", "optionsProposees": ["a"]})).is_empty());
        assert_eq!(codes("eleve", "options", json!(["a", 1])), ["json_type"]);
        assert_eq!(codes("eleve", "options", json!({})), ["json_type"]);
    }

    #[test]
    fn colonne_a_cataloguer_refuse_tout_contenu() {
        assert!(codes("personnel", "payload", json!({})).is_empty());
        assert_eq!(codes("personnel", "payload", json!({"conges": 1, "Nom Réel": 2})), ["json_cle_inconnue", "json_cle_inconnue"]);
        assert!(est_a_cataloguer("personnel", "payload"));
        assert!(!est_a_cataloguer("fd_fiche", "appel"));
    }

    #[test]
    fn la_cle_non_citable_n_est_jamais_reproduite() {
        let f = forme_colonne("personnel", "payload", false).unwrap();
        let r = valider(&f, &json!({"jean.dupont@exemple.test": 1}));
        assert!(!r[0].message.contains("dupont"));
    }

    #[test]
    fn instantane_borne() {
        assert!(codes("metier_event", "payload", json!({"a": {"b": [1, 2]}})).is_empty());
        assert_eq!(codes("metier_event", "payload", json!([1])), ["json_type"]);
        let mut v = json!(1);
        for _ in 0..14 {
            v = json!({ "a": v });
        }
        assert_eq!(codes("metier_event", "payload", v), ["json_profondeur"]);
    }

    #[test]
    fn toute_colonne_json_chargee_a_une_forme() {
        use crate::perimetre::Perimetre;
        use crate::schema::SchemaAttendu;
        let (s, p) = (SchemaAttendu::embarque(), Perimetre::embarque());
        for (t, cols) in &s.tables {
            if !p.table(t).is_some_and(|e| e.disposition.chargee()) {
                continue;
            }
            for c in cols.iter().filter(|c| c.type_sql == "jsonb") {
                assert!(forme_colonne(t, &c.nom, c.nullable).is_some(), "colonne JSON sans forme : {t}.{}", c.nom);
            }
        }
    }

    #[test]
    fn nullable_admet_null() {
        let f = forme_colonne("fd_fiche", "appel", true).unwrap();
        assert!(valider(&f, &Value::Null).is_empty());
        assert!(!valider(&forme_colonne("fd_fiche", "appel", false).unwrap(), &Value::Null).is_empty());
    }

    #[test]
    fn dictionnaire_de_formulaire() {
        assert!(codes("fd_reponse", "payload", json!({"values": {"a": "x", "b": ["y"], "c": true, "d": null}})).is_empty());
        assert_eq!(codes("fd_reponse", "payload", json!({"values": {"a": 3}})), ["json_type"]);
    }
}
