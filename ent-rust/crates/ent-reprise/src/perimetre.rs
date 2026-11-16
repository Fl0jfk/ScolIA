//! Périmètre de la reprise : ce qu'on fait de chaque table et de chaque collection du Next.js.
//!
//! Source : `donnees/perimetre_reprise.tsv` (une ligne par table du schéma attendu, vérifiée par test) et
//! la table [`FAMILLES_COLLECTIONS`]. Dispositions (d'après T0 §4–5 et `PLAN_BASCULE_FEVRIER.md` §1.2) :
//! - `reprise` : donnée d'un module, reprise par les lots de module (R5 et suivants) ;
//! - `rapprochement` : entrée du rapprochement des référentiels et des personnes (R3) ; jamais de fiche du
//!   cœur sans décision : tant que rien n'est rapproché, la ligne est « à décider » ;
//! - `archive` : conservée en archive de lecture, non reprise ;
//! - `ignore:<raison>` : technique, secret, jeton, état d'interface, perdable : non chargée ;
//! - `sante` : refusée (aucune reprise sans hébergeur HDS, R-MSE5) et même pas lue ;
//! - `collections` : conteneur clé-valeur, ventilé par famille de collection.

use crate::{err, Res};
use std::collections::BTreeMap;

pub const PERIMETRE_TSV: &str = include_str!("../donnees/perimetre_reprise.tsv");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposition {
    Reprise,
    Rapprochement,
    Archive,
    Ignore(String),
    Sante,
    Collections,
}

impl Disposition {
    fn depuis(s: &str) -> Option<Disposition> {
        Some(match s {
            "reprise" => Disposition::Reprise,
            "rapprochement" => Disposition::Rapprochement,
            "archive" => Disposition::Archive,
            "sante" => Disposition::Sante,
            "collections" => Disposition::Collections,
            _ => Disposition::Ignore(s.strip_prefix("ignore:")?.to_string()),
        })
    }

    /// Les cellules de la table sont décodées et chargées en zone.
    pub fn chargee(&self) -> bool {
        matches!(
            self,
            Disposition::Reprise | Disposition::Rapprochement | Disposition::Archive | Disposition::Collections
        )
    }
}

#[derive(Debug, Clone)]
pub struct EntreeTable {
    pub disposition: Disposition,
    pub domaine: String,
    /// Colonnes qui forment l'identifiant source.
    pub cle: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Perimetre {
    pub tables: BTreeMap<String, EntreeTable>,
}

impl Perimetre {
    pub fn embarque() -> Perimetre {
        Self::depuis_tsv(PERIMETRE_TSV).expect("perimetre_reprise.tsv embarqué valide")
    }

    pub fn depuis_tsv(tsv: &str) -> Res<Perimetre> {
        let mut tables = BTreeMap::new();
        for l in tsv.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
            let p: Vec<&str> = l.split('\t').collect();
            if p.len() != 4 {
                return Err(err!("perimetre_reprise.tsv : ligne mal formée"));
            }
            let disposition = Disposition::depuis(p[1])
                .ok_or_else(|| err!("perimetre_reprise.tsv : disposition inconnue pour {}", p[0]))?;
            let e = EntreeTable {
                disposition,
                domaine: p[2].to_string(),
                cle: p[3].split('+').map(str::to_string).collect(),
            };
            if tables.insert(p[0].to_string(), e).is_some() {
                return Err(err!("perimetre_reprise.tsv : table déclarée deux fois : {}", p[0]));
            }
        }
        Ok(Perimetre { tables })
    }

    pub fn table(&self, nom: &str) -> Option<&EntreeTable> {
        self.tables.get(nom)
    }
}

/// Famille d'une collection : premier segment du nom (`internat__rooms` → `internat`).
pub fn famille(collection: &str) -> &str {
    collection.split("__").next().unwrap_or(collection)
}

/// (famille, disposition, domaine). Une famille absente = collection inconnue (R-01 : elle bloque).
pub const FAMILLES_COLLECTIONS: &[(&str, &str, &str)] = &[
    ("internat", "reprise", "internat"),
    ("reservation-rooms", "reprise", "salles"),
    ("certificates", "reprise", "certificats"),
    ("stages", "reprise", "stages"),
    ("stage", "reprise", "stages"),
    ("demandes-hse", "reprise", "demandes"),
    ("rgpd", "reprise", "rgpd"),
    ("rentree", "reprise", "inscriptions"),
    ("documents", "reprise", "documents"),
    ("pilotage", "rapprochement", "coeur"),
    ("personnel-index", "rapprochement", "coeur"),
    ("personnel-ogec", "rapprochement", "coeur"),
    ("eleves", "rapprochement", "coeur"),
    ("siecle", "rapprochement", "coeur"),
    ("requests", "archive", "demandes"),
    ("domain-planning", "archive", "tables_mortes"),
    ("rh", "archive", "archives"),
    ("settings", "archive", "parametrages"),
    ("photocopies-couleur", "archive", "archives"),
    ("posters", "archive", "archives"),
    ("class-allocation", "archive", "archives"),
    ("dashboard", "archive", "archives"),
    ("travels", "archive", "voyages"),
    ("chat", "ignore:perdable", "technique"),
    ("channels", "ignore:perdable", "technique"),
    ("site-cms", "ignore:perdable", "technique"),
    ("microsoft-license-request", "ignore:plateforme", "technique"),
    ("tenants", "ignore:secret", "technique"),
    ("bien-etre", "sante", "sante"),
];

/// Domaine d'une collection (socle ou module propriétaire).
pub fn domaine_collection(collection: &str) -> &'static str {
    let f = famille(collection);
    FAMILLES_COLLECTIONS
        .iter()
        .find(|(n, _, _)| *n == f)
        .map_or("inconnu", |(_, _, d)| d)
}

/// Disposition d'une collection : `None` si la famille est inconnue.
pub fn disposition_collection(collection: &str) -> Option<Disposition> {
    if collection.contains("tokens") {
        return Some(Disposition::Ignore("jeton".into()));
    }
    if collection == "settings__bien-etre" {
        return Some(Disposition::Sante);
    }
    let f = famille(collection);
    FAMILLES_COLLECTIONS
        .iter()
        .find(|(n, _, _)| *n == f)
        .and_then(|(_, d, _)| Disposition::depuis(d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{SchemaAttendu, TABLES_SANTE};

    #[test]
    fn le_perimetre_couvre_exactement_le_schema() {
        let p = Perimetre::embarque();
        let s = SchemaAttendu::embarque();
        for t in s.tables.keys() {
            assert!(p.tables.contains_key(t), "table du schéma sans disposition : {t}");
        }
        for t in p.tables.keys() {
            assert!(s.tables.contains_key(t), "disposition sans table du schéma : {t}");
        }
    }

    #[test]
    fn les_cles_sources_existent_dans_le_schema() {
        let p = Perimetre::embarque();
        let s = SchemaAttendu::embarque();
        for (t, e) in &p.tables {
            for c in &e.cle {
                assert!(s.tables[t].iter().any(|x| &x.nom == c), "{t} : colonne de clé inconnue {c}");
            }
        }
    }

    #[test]
    fn la_sante_est_toujours_refusee() {
        let p = Perimetre::embarque();
        for t in TABLES_SANTE {
            assert_eq!(p.table(t).unwrap().disposition, Disposition::Sante, "{t}");
        }
    }

    #[test]
    fn les_tables_mortes_ne_sont_jamais_perdues_en_silence() {
        let p = Perimetre::embarque();
        for t in crate::inspection::TABLES_MORTES {
            assert_eq!(p.table(t).unwrap().disposition, Disposition::Archive, "{t}");
        }
    }

    #[test]
    fn collections() {
        assert_eq!(disposition_collection("internat__rooms"), Some(Disposition::Reprise));
        assert_eq!(disposition_collection("stages__student-tokens"), Some(Disposition::Ignore("jeton".into())));
        assert_eq!(disposition_collection("settings__bien-etre"), Some(Disposition::Sante));
        assert_eq!(disposition_collection("inconnue__x"), None);
    }
}
