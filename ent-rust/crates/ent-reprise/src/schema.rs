//! Schéma attendu du Next.js et liste blanche de l'anonymisation.
//!
//! Deux fichiers de données, embarqués dans le binaire :
//! - `donnees/schema_attendu.tsv` : toutes les colonnes de toutes les tables (extrait du schéma
//!   Drizzle de `origin/main` rejoué sur une base **locale vide**, voir `outils/`) ;
//! - `donnees/liste_blanche.tsv` : pour **chaque** colonne, une règle explicite.
//!
//! Le test `garde_liste_blanche_couvre_le_schema` échoue si une colonne du schéma manque à la liste
//! blanche (ou l'inverse) : ajouter une table au Next.js sans la classer casse la CI.

use crate::{err, Res};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const SCHEMA_TSV: &str = include_str!("../donnees/schema_attendu.tsv");
pub const BLANCHE_TSV: &str = include_str!("../donnees/liste_blanche.tsv");

/// Tables créées par des scripts du Next.js, absentes de ses déclarations Drizzle : tolérées si
/// elles manquent à l'export (état éphémère).
pub const TABLES_OPTIONNELLES: &[&str] = &["messaging_call_ring", "messaging_signal_relay"];

/// Tables ajoutées au Next.js après l'inventaire T0 (184 tables, `TRANSITION_T0_INVENTAIRE.md` §1).
pub const TABLES_DEPUIS_T0: &[&str] = &[
    "web_push_preference",
    "web_push_signal_state",
    "web_push_subscription",
];

/// Nombre de tables vivantes de l'inventaire T0.
pub const TABLES_INVENTAIRE_T0: usize = 184;

/// Tables de santé (exclues de l'export anonymisé par défaut, T0 §9 règle 7, R-MSE5).
pub const TABLES_SANTE: &[&str] = &[
    "sante_accident",
    "sante_extrait",
    "sante_inaptitude_eps",
    "sante_medicament_prise",
    "sante_pai",
    "infirmerie_fiche",
    "infirmerie_passage",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColSchema {
    pub nom: String,
    pub type_sql: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SchemaAttendu {
    pub tables: BTreeMap<String, Vec<ColSchema>>,
}

/// Nature d'un pseudonyme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Genre {
    Nom,
    Prenom,
    NomComplet,
    Email,
    Telephone,
    Ine,
    Iban,
    Bic,
    Rum,
    DateNaissance,
    Ville,
    Adresse,
    CodePostal,
    Ip,
    Agent,
    Identifiant,
    IdSouple,
    Jeton,
    Image,
    Pdf,
    NomFichier,
    Texte,
}

impl Genre {
    pub fn depuis(s: &str) -> Option<Genre> {
        Some(match s {
            "nom" => Genre::Nom,
            "prenom" => Genre::Prenom,
            "nom_complet" => Genre::NomComplet,
            "email" => Genre::Email,
            "telephone" => Genre::Telephone,
            "ine" => Genre::Ine,
            "iban" => Genre::Iban,
            "bic" => Genre::Bic,
            "rum" => Genre::Rum,
            "date_naissance" => Genre::DateNaissance,
            "ville" => Genre::Ville,
            "adresse" => Genre::Adresse,
            "code_postal" => Genre::CodePostal,
            "ip" => Genre::Ip,
            "agent" => Genre::Agent,
            "identifiant" => Genre::Identifiant,
            "id_souple" => Genre::IdSouple,
            "jeton" => Genre::Jeton,
            "image" => Genre::Image,
            "pdf" => Genre::Pdf,
            "nom_fichier" => Genre::NomFichier,
            "texte" => Genre::Texte,
            _ => return None,
        })
    }

    /// Genres dont les valeurs réelles servent au contrôle « zéro occurrence » (identités et coordonnées).
    pub fn identifie_une_personne(self) -> bool {
        matches!(
            self,
            Genre::Nom
                | Genre::Prenom
                | Genre::NomComplet
                | Genre::Email
                | Genre::Telephone
                | Genre::Ine
                | Genre::Iban
                | Genre::Adresse
        )
    }
}

/// Règle appliquée à une colonne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regle {
    /// Conservée telle quelle.
    Garder,
    /// Vidée (NULL si la colonne l'admet, sinon texte vide).
    Effacer,
    /// Remplacée de façon déterministe (HMAC avec la clé locale).
    Pseudo(Genre),
    /// Clé d'objet S3 (chemin réécrit, extension gardée).
    CleS3,
    /// Colonne JSON : décodée, chaque clé traitée.
    Json,
    /// Valeur d'une table clé-valeur (`*_attr`) : préfixe `s:`/`n:`/`b:`/`j:` décodé, clé = dernier segment du chemin.
    AttrValeur,
}

impl Regle {
    pub fn depuis(s: &str) -> Option<Regle> {
        Some(match s {
            "garder" => Regle::Garder,
            "effacer" => Regle::Effacer,
            "cle_s3" => Regle::CleS3,
            "json" => Regle::Json,
            "attr_valeur" => Regle::AttrValeur,
            _ => Regle::Pseudo(Genre::depuis(s.strip_prefix("pseudo:")?)?),
        })
    }
}

/// Normalise un type pour la comparaison (`_text` → `text[]`, `ARRAY` → `text[]`).
pub fn normaliser_type(t: &str) -> String {
    let t = t.trim();
    if let Some(base) = t.strip_prefix('_') {
        return format!("{base}[]");
    }
    if t == "ARRAY" {
        return "text[]".to_string();
    }
    t.to_string()
}

fn lignes_utiles(tsv: &str) -> impl Iterator<Item = &str> {
    tsv.lines().filter(|l| !l.is_empty() && !l.starts_with('#'))
}

impl SchemaAttendu {
    pub fn embarque() -> SchemaAttendu {
        Self::depuis_tsv(SCHEMA_TSV).expect("schema_attendu.tsv embarqué valide")
    }

    pub fn depuis_tsv(tsv: &str) -> Res<SchemaAttendu> {
        let mut tables: BTreeMap<String, Vec<ColSchema>> = BTreeMap::new();
        for l in lignes_utiles(tsv) {
            let p: Vec<&str> = l.split('\t').collect();
            if p.len() != 4 {
                return Err(err!("schema_attendu.tsv : ligne mal formée"));
            }
            tables.entry(p[0].to_string()).or_default().push(ColSchema {
                nom: p[1].to_string(),
                type_sql: normaliser_type(p[2]),
                nullable: p[3] == "1",
            });
        }
        Ok(SchemaAttendu { tables })
    }

    /// Tables que l'export doit contenir (hors optionnelles).
    pub fn tables_requises(&self) -> impl Iterator<Item = &String> {
        self.tables
            .keys()
            .filter(|t| !TABLES_OPTIONNELLES.contains(&t.as_str()))
    }
}

#[derive(Debug, Clone, Default)]
pub struct ListeBlanche {
    regles: HashMap<(String, String), Regle>,
}

impl ListeBlanche {
    pub fn embarquee() -> ListeBlanche {
        Self::depuis_tsv(BLANCHE_TSV).expect("liste_blanche.tsv embarquée valide")
    }

    pub fn depuis_tsv(tsv: &str) -> Res<ListeBlanche> {
        let mut regles = HashMap::new();
        for l in lignes_utiles(tsv) {
            let p: Vec<&str> = l.split('\t').collect();
            if p.len() != 3 {
                return Err(err!("liste_blanche.tsv : ligne mal formée"));
            }
            let r = Regle::depuis(p[2])
                .ok_or_else(|| err!("liste_blanche.tsv : règle inconnue pour {}.{}", p[0], p[1]))?;
            if regles
                .insert((p[0].to_string(), p[1].to_string()), r)
                .is_some()
            {
                return Err(err!(
                    "liste_blanche.tsv : colonne déclarée deux fois : {}.{}",
                    p[0],
                    p[1]
                ));
            }
        }
        Ok(ListeBlanche { regles })
    }

    pub fn regle(&self, table: &str, colonne: &str) -> Option<Regle> {
        self.regles
            .get(&(table.to_string(), colonne.to_string()))
            .copied()
    }

    pub fn nombre(&self) -> usize {
        self.regles.len()
    }

    /// Colonnes du schéma absentes de la liste blanche, et entrées de la liste absentes du schéma.
    pub fn ecarts(&self, schema: &SchemaAttendu) -> (Vec<String>, Vec<String>) {
        let mut manquantes = Vec::new();
        let mut connues: BTreeSet<(&str, &str)> = BTreeSet::new();
        for (t, cols) in &schema.tables {
            for c in cols {
                connues.insert((t.as_str(), c.nom.as_str()));
                if self.regle(t, &c.nom).is_none() {
                    manquantes.push(format!("{t}.{}", c.nom));
                }
            }
        }
        let mut en_trop: Vec<String> = self
            .regles
            .keys()
            .filter(|(t, c)| !connues.contains(&(t.as_str(), c.as_str())))
            .map(|(t, c)| format!("{t}.{c}"))
            .collect();
        en_trop.sort();
        (manquantes, en_trop)
    }
}

/// Écarts entre le schéma attendu et ce que contient le dump.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct EcartsSchema {
    pub tables_manquantes: Vec<String>,
    pub tables_inconnues: Vec<String>,
    pub colonnes_manquantes: Vec<String>,
    pub colonnes_inconnues: Vec<String>,
    pub types_differents: Vec<String>,
}

impl EcartsSchema {
    pub fn est_vide(&self) -> bool {
        self.tables_manquantes.is_empty()
            && self.tables_inconnues.is_empty()
            && self.colonnes_manquantes.is_empty()
            && self.colonnes_inconnues.is_empty()
            && self.types_differents.is_empty()
    }
}

/// Compare le schéma attendu aux tables lues dans le dump (`CREATE TABLE`).
pub fn comparer(
    attendu: &SchemaAttendu,
    vues: &[(String, Vec<crate::dump::ColonneDump>)],
) -> EcartsSchema {
    let mut e = EcartsSchema::default();
    let vues: BTreeMap<&str, &Vec<crate::dump::ColonneDump>> =
        vues.iter().map(|(t, c)| (t.as_str(), c)).collect();
    for t in attendu.tables_requises() {
        if !vues.contains_key(t.as_str()) {
            e.tables_manquantes.push(t.clone());
        }
    }
    for t in vues.keys() {
        if !attendu.tables.contains_key(*t) {
            e.tables_inconnues.push((*t).to_string());
        }
    }
    for (t, cols) in &attendu.tables {
        let Some(vc) = vues.get(t.as_str()) else {
            continue;
        };
        for c in cols {
            match vc.iter().find(|x| x.nom == c.nom) {
                None => e.colonnes_manquantes.push(format!("{t}.{}", c.nom)),
                Some(x) if x.type_sql != "?" && normaliser_type(&x.type_sql) != c.type_sql => {
                    e.types_differents.push(format!(
                        "{t}.{} (attendu {}, vu {})",
                        c.nom, c.type_sql, x.type_sql
                    ))
                }
                _ => {}
            }
        }
        for x in vc.iter() {
            if !cols.iter().any(|c| c.nom == x.nom) {
                e.colonnes_inconnues.push(format!("{t}.{}", x.nom));
            }
        }
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GARDE : toute colonne du schéma Next.js doit figurer dans la liste blanche, et réciproquement.
    /// Un nouveau champ ou une nouvelle table côté Next.js sans classement explicite fait échouer la CI.
    #[test]
    fn garde_liste_blanche_couvre_le_schema() {
        let schema = SchemaAttendu::embarque();
        let blanche = ListeBlanche::embarquee();
        let (manquantes, en_trop) = blanche.ecarts(&schema);
        assert!(
            manquantes.is_empty(),
            "colonnes du schéma Next.js absentes de la liste blanche : {manquantes:?}"
        );
        assert!(
            en_trop.is_empty(),
            "entrées de la liste blanche absentes du schéma : {en_trop:?}"
        );
    }

    #[test]
    fn le_schema_compte_les_tables_attendues() {
        let s = SchemaAttendu::embarque();
        let requises = s.tables_requises().count();
        assert_eq!(
            requises,
            TABLES_INVENTAIRE_T0 + TABLES_DEPUIS_T0.len(),
            "184 de l'inventaire T0 + web push"
        );
        for t in TABLES_DEPUIS_T0 {
            assert!(s.tables.contains_key(*t), "{t}");
        }
        for t in TABLES_OPTIONNELLES {
            assert!(s.tables.contains_key(*t), "{t}");
        }
        for t in TABLES_SANTE {
            assert!(s.tables.contains_key(*t), "{t}");
        }
    }

    #[test]
    fn regle_inconnue_ou_doublon_refuse() {
        assert!(ListeBlanche::depuis_tsv("t\tc\tgarde\n").is_err());
        assert!(ListeBlanche::depuis_tsv("t\tc\tgarder\nt\tc\teffacer\n").is_err());
        assert!(ListeBlanche::depuis_tsv("t\tc\tpseudo:inconnu\n").is_err());
        assert!(ListeBlanche::depuis_tsv("t\tc\tpseudo:nom\n").is_ok());
    }

    #[test]
    fn aucune_colonne_secrete_connue_n_est_gardee() {
        let b = ListeBlanche::embarquee();
        for (t, c) in [
            ("account", "password"),
            ("account", "access_token"),
            ("two_factor", "secret"),
            ("two_factor", "backup_codes"),
            ("rdv_inscription_config", "google_refresh_token"),
            ("session", "token"),
            ("fd_token", "token"),
        ] {
            let r = b.regle(t, c).unwrap();
            assert!(
                !matches!(r, Regle::Garder),
                "{t}.{c} ne doit pas être gardée"
            );
        }
    }
}
