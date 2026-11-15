//! Rapport d'anonymisation : ce qui a été transformé, **sans aucune valeur réelle**.

use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Default)]
pub struct ColonneRapport {
    pub regle: String,
    pub cellules_transformees: u64,
    pub cellules_nulles: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct TableRapportAnon {
    pub lignes: u64,
    pub exclue_sante: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct S3Rapport {
    pub objets_lus: u64,
    pub objets_secrets_ecartes: u64,
    pub objets_reecrits: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FuiteRapport {
    pub colonne: String,
    pub cellules: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ControleRapport {
    pub valeurs_sensibles_controlees: u64,
    pub cellules_controlees: u64,
    pub fuites: Vec<FuiteRapport>,
    pub resultat: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FichierRapport {
    pub nom: String,
    pub octets: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RapportAnonymisation {
    pub outil_version: String,
    pub cle_empreinte: String,
    pub tables: BTreeMap<String, TableRapportAnon>,
    pub lignes_total: u64,
    pub colonnes: BTreeMap<String, ColonneRapport>,
    pub cles_json_inconnues: BTreeMap<String, u64>,
    pub feuilles_json_traitees: u64,
    pub s3: Option<S3Rapport>,
    pub controle: ControleRapport,
    pub fichiers: Vec<FichierRapport>,
}

impl RapportAnonymisation {
    pub fn texte(&self) -> String {
        let mut s = String::new();
        s.push_str("=== Rapport d'anonymisation ===\n");
        s.push_str("Ce rapport ne contient AUCUNE valeur réelle : seulement des noms de tables, de colonnes et des nombres.\n\n");
        s.push_str(&format!("Outil : ent-reprise {}\n", self.outil_version));
        s.push_str(&format!(
            "Empreinte de la clé utilisée (non réversible) : {}\n",
            self.cle_empreinte
        ));
        s.push_str(&format!(
            "Tables : {} — lignes traitées : {}\n\n",
            self.tables.len(),
            self.lignes_total
        ));
        let exclues: Vec<&String> = self
            .tables
            .iter()
            .filter(|(_, t)| t.exclue_sante)
            .map(|(n, _)| n)
            .collect();
        if !exclues.is_empty() {
            s.push_str(&format!(
                "Santé : {} table(s) EXCLUE(S) (structure gardée, aucune ligne) : {}\n\n",
                exclues.len(),
                exclues
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        s.push_str("--- Colonnes transformées (règle, cellules) ---\n");
        for (c, x) in &self.colonnes {
            if x.regle != "garder" && (x.cellules_transformees > 0 || x.cellules_nulles > 0) {
                s.push_str(&format!(
                    "{:>9} {:<18} {}\n",
                    x.cellules_transformees, x.regle, c
                ));
            }
        }
        let gardees = self
            .colonnes
            .values()
            .filter(|x| x.regle == "garder")
            .count();
        s.push_str(&format!("\n{gardees} colonne(s) gardée(s) telles quelles (identifiants, dates, montants, statuts…).\n"));
        s.push_str(&format!(
            "\n--- JSON : {} feuille(s) traitée(s) ---\n",
            self.feuilles_json_traitees
        ));
        if self.cles_json_inconnues.is_empty() {
            s.push_str("Aucune clé JSON hors dictionnaire.\n");
        } else {
            s.push_str("Clés hors dictionnaire (valeurs texte remplacées par défaut ; à classer dans le dictionnaire) :\n");
            for (k, n) in &self.cles_json_inconnues {
                s.push_str(&format!("{n:>9} {k}\n"));
            }
        }
        if let Some(x) = &self.s3 {
            s.push_str(&format!(
                "\n--- Objets S3 ---\n{} lu(s), {} réécrit(s), {} secret(s)/registre écarté(s)\n",
                x.objets_lus, x.objets_reecrits, x.objets_secrets_ecartes
            ));
        }
        s.push_str("\n--- Contrôle « zéro occurrence » ---\n");
        s.push_str(&format!(
            "{} valeur(s) sensible(s) empreintée(s), {} cellule(s) relue(s).\n",
            self.controle.valeurs_sensibles_controlees, self.controle.cellules_controlees
        ));
        if self.controle.fuites.is_empty() {
            s.push_str("Aucune valeur réelle retrouvée dans l'export.\n");
        } else {
            for f in &self.controle.fuites {
                s.push_str(&format!(
                    "ÉCART : {} cellule(s) dans {}\n",
                    f.cellules, f.colonne
                ));
            }
        }
        s.push_str(&format!(
            "Résultat : {}\n\n--- Fichiers livrés ---\n",
            self.controle.resultat
        ));
        for f in &self.fichiers {
            s.push_str(&format!("{}  {}  ({} octets)\n", f.sha256, f.nom, f.octets));
        }
        s
    }
}
