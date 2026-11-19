//! Socle 21, lot V3 : collage depuis un tableur et import d'un fichier du professeur.
//!
//! Même chemin pour les deux : **aperçu** (rien n'est écrit : rapprochement des élèves, lecture de chaque
//! case, comparaison avec l'existant, erreurs ligne à ligne) puis **confirmation** avec l'empreinte de
//! l'aperçu. Si une case a changé entre-temps, l'empreinte ne correspond plus et rien n'est écrit : le
//! professeur refait l'aperçu (jamais d'écrasement silencieux). Les lignes non rapprochées ou ambiguës ne
//! sont jamais devinées ; elles sont montrées et laissées de côté.
//!
//! L'import porte sur **les évaluations du cours du professeur, à sa portée** (M-447) : le droit de saisie
//! sur le cours est exigé, comme pour la grille. La lecture des textes est celle de la grille
//! (`analyser_cellule`) : un seul analyseur.

use std::collections::{BTreeMap, HashMap};

use chrono::NaiveDate;
use ent_types::evaluations_v3::{
    analyser_cellule, Analyse, CelluleSaisieDto, ImportApercuDto, ImportBody,
    ImportCelluleApercuDto, ImportEleveRapprocheDto, ImportLigneApercuDto, ImportTotauxDto,
    SaisieLotBody, SaisieLotRetourDto,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::evaluations::{conflit, introuvable, refus};
use crate::evaluations_v3::{EleveGrille, GrilleInterne};
use crate::service::{EntDbError, PgEnt};

/// Lignes et cases maximales d'un import.
const MAX_LIGNES: usize = 500;
const MAX_CIBLES: usize = 60;

/// Clé de rapprochement d'un nom : minuscules, sans accent, ponctuation en espaces, mots **triés** (donc
/// « Léa Martin », « MARTIN Léa » et « martin, lea » se rapprochent ; « Jean-Pierre » = « Jean Pierre »).
pub fn cle_nom(texte: &str) -> String {
    let mut mots: Vec<String> = texte
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'œ' => 'o',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    mots.sort();
    mots.join(" ")
}

/// Résultat d'un rapprochement par nom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rapprochement {
    Un(usize),
    Ambigu,
    Aucun,
}

/// Rapproche un nom saisi d'un élève de la liste, par clé de nom (nom + prénom, dans n'importe quel ordre).
pub fn rapprocher(eleves: &[(String, String)], saisi: &str) -> Rapprochement {
    let cle = cle_nom(saisi);
    if cle.is_empty() {
        return Rapprochement::Aucun;
    }
    let mut trouves = eleves
        .iter()
        .enumerate()
        .filter(|(_, (nom, prenom))| cle_nom(&format!("{nom} {prenom}")) == cle)
        .map(|(i, _)| i);
    match (trouves.next(), trouves.next()) {
        (None, _) => Rapprochement::Aucun,
        (Some(i), None) => Rapprochement::Un(i),
        _ => Rapprochement::Ambigu,
    }
}

/// Une case analysée, avec ce qu'elle remplacerait (interne : porte la version lue).
#[derive(Debug, Clone)]
struct CaseAnalysee {
    dto: ImportCelluleApercuDto,
    version_avant: Option<i32>,
    ecrire: bool,
}

#[derive(Debug, Clone)]
struct LigneAnalysee {
    dto: ImportLigneApercuDto,
    cases: Vec<CaseAnalysee>,
}

fn empreinte(cours_id: Uuid, mode: &str, lignes: &[LigneAnalysee]) -> String {
    let mut h = Sha256::new();
    h.update(format!("v3|{cours_id}|{mode}|").as_bytes());
    for l in lignes {
        h.update(
            format!(
                "L{}|{}|{}|",
                l.dto.numero,
                l.dto
                    .eleve
                    .as_ref()
                    .map_or(String::from("-"), |e| e.eleve_id.to_string()),
                l.dto.rapprochement
            )
            .as_bytes(),
        );
        for c in &l.cases {
            h.update(
                format!(
                    "{}|{}|{:?}|{:?}|{:?}|{};",
                    c.dto.evaluation_id,
                    c.dto.etat,
                    c.dto.statut,
                    c.dto.valeur_m,
                    c.version_avant,
                    c.ecrire
                )
                .as_bytes(),
            );
        }
    }
    h.finalize().iter().fold(String::new(), |mut s, b| {
        s.push_str(&format!("{b:02x}"));
        s
    })
}

impl PgEnt {
    /// Analyse (sans rien écrire) un collage ou un fichier.
    async fn eva_v3_analyser_import(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &ImportBody,
    ) -> Result<(Vec<LigneAnalysee>, usize, GrilleInterne), EntDbError> {
        if !matches!(body.mode.as_str(), "collage" | "fichier") {
            return Err(refus("mode d'import inconnu"));
        }
        if body.lignes.is_empty() || body.lignes.len() > MAX_LIGNES {
            return Err(refus("un import compte de 1 à 500 lignes"));
        }
        if body.cibles.is_empty() || body.cibles.len() > MAX_CIBLES {
            return Err(refus("un import vise de 1 à 60 évaluations"));
        }
        let acces = self
            .eva_v3_acces(etab, acteur, role, body.cours_id, jour)
            .await?;
        if !acces.saisie {
            return Err(EntDbError::Forbidden);
        }
        // Les évaluations visées doivent être du cours ; la période est celle de la première (collage et
        // fichier ne mélangent pas des périodes : une période par aperçu).
        let premiere = body.cibles[0];
        let (cours_de, date): (Uuid, NaiveDate) = {
            let mut tx = crate::tenant_tx::TenantTx::begin(self.pool(), etab)
                .await
                .map_err(crate::evaluations::db)?;
            let r: Option<(Uuid, NaiveDate)> = sqlx::query_as(
                "SELECT cours_id, date_eval FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(premiere)
            .fetch_optional(&mut *tx)
            .await
            .map_err(crate::evaluations::db)?;
            tx.commit().await.map_err(crate::evaluations::db)?;
            r.ok_or_else(introuvable)?
        };
        if cours_de != body.cours_id {
            return Err(introuvable());
        }
        let periodes = self
            .eva_v3_periodes_for_actor(etab, acteur, role, body.cours_id, jour)
            .await?;
        let periode = periodes
            .iter()
            .find(|p| p.date_debut <= date && date <= p.date_fin)
            .ok_or(refus(
                "la date de l'évaluation ne tombe dans aucune période de ce cours",
            ))?;
        let g = self
            .eva_v3_charger(etab, acteur, role, body.cours_id, periode.id, jour, true)
            .await?;
        let mut cibles = Vec::with_capacity(body.cibles.len());
        for id in &body.cibles {
            let ev = g.evals.iter().find(|e| e.id == *id).ok_or(refus(
                "une évaluation visée n'est pas dans la période de la première",
            ))?;
            if ev.etat != "ouverte" {
                return Err(refus("une évaluation visée est annulée ou non ouverte"));
            }
            cibles.push(ev.clone());
        }
        let noms: Vec<(String, String)> = g
            .eleves
            .iter()
            .map(|e| (e.nom.clone(), e.prenom.clone()))
            .collect();
        let pas_m = g.reglage.pas_saisie_m;
        let mut vus: HashMap<Uuid, usize> = HashMap::new();
        let mut lignes: Vec<LigneAnalysee> = Vec::with_capacity(body.lignes.len());
        for (i, l) in body.lignes.iter().enumerate() {
            let numero = i + 1;
            let eleve: Option<&EleveGrille>;
            let mut rapprochement = "ok";
            if let Some(id) = l.eleve_id {
                eleve = g.eleves.iter().find(|e| e.id == id);
                if eleve.is_none() {
                    rapprochement = "hors_public";
                }
            } else if let Some(nom) = l.nom.as_deref().filter(|n| !n.trim().is_empty()) {
                match rapprocher(&noms, nom) {
                    Rapprochement::Un(k) => eleve = g.eleves.get(k),
                    Rapprochement::Ambigu => {
                        eleve = None;
                        rapprochement = "ambigu";
                    }
                    Rapprochement::Aucun => {
                        eleve = None;
                        rapprochement = "non_rapproche";
                    }
                }
            } else if let Some(rang) = l.rang.filter(|r| *r >= 1) {
                eleve = g.eleves.get(rang - 1);
                if eleve.is_none() {
                    rapprochement = "non_rapproche";
                }
            } else {
                eleve = None;
                rapprochement = "non_rapproche";
            }
            // Deux lignes pour le même élève : la seconde est écartée (jamais deux écritures concurrentes).
            if let Some(e) = eleve {
                if let Some(deja) = vus.insert(e.id, numero) {
                    let _ = deja;
                    rapprochement = "ambigu";
                }
            }
            let eleve_ok = if rapprochement == "ok" { eleve } else { None };
            let mut cases = Vec::new();
            if let Some(e) = eleve_ok {
                for (k, ev) in cibles.iter().enumerate() {
                    let texte = l.cellules.get(k).map(String::as_str).unwrap_or("");
                    let case = g.cases.get(&(ev.id, e.id)).copied().unwrap_or_default();
                    let avant = g.resultats.get(&(ev.id, e.id));
                    let mut dto = ImportCelluleApercuDto {
                        evaluation_id: ev.id,
                        texte: texte.chars().take(40).collect(),
                        etat: "vide".to_string(),
                        statut: None,
                        valeur_m: None,
                        avant_statut: avant.map(|r| r.statut.clone()),
                        avant_valeur_m: avant.and_then(|r| r.valeur_m),
                        erreur: None,
                    };
                    let mut ecrire = false;
                    match analyser_cellule(texte, &ev.contrainte(pas_m)) {
                        Analyse::Vide => {}
                        Analyse::Erreur(m) => {
                            dto.etat = "erreur".to_string();
                            dto.erreur = Some(m);
                        }
                        a => {
                            let (statut, valeur) = match a {
                                Analyse::Statut(s) => (s.to_string(), None),
                                Analyse::Note(m) => ("note".to_string(), Some(m)),
                                _ => unreachable!(),
                            };
                            dto.statut = Some(statut.clone());
                            dto.valeur_m = valeur;
                            if !case.concerne {
                                dto.etat = "erreur".to_string();
                                dto.erreur = Some(
                                    "élève non concerné par cette évaluation à sa date".to_string(),
                                );
                            } else if !case.saisie_ouverte && !case.rattrapage_autorise {
                                dto.etat = "verrouille".to_string();
                                dto.erreur = Some("période arrêtée pour cet élève".to_string());
                            } else if avant
                                .is_some_and(|r| r.statut == statut && r.valeur_m == valeur)
                            {
                                dto.etat = "inchange".to_string();
                            } else if avant.is_some() {
                                dto.etat = "modifie".to_string();
                                ecrire = true;
                            } else {
                                dto.etat = "nouveau".to_string();
                                ecrire = true;
                            }
                        }
                    }
                    cases.push(CaseAnalysee {
                        dto,
                        version_avant: avant.map(|r| r.version),
                        ecrire,
                    });
                }
            }
            lignes.push(LigneAnalysee {
                dto: ImportLigneApercuDto {
                    numero,
                    nom_saisi: l.nom.as_deref().map(|n| n.chars().take(80).collect()),
                    eleve: eleve_ok.map(|e| ImportEleveRapprocheDto {
                        eleve_id: e.id,
                        nom: e.nom.clone(),
                        prenom: e.prenom.clone(),
                    }),
                    rapprochement: rapprochement.to_string(),
                    cellules: cases.iter().map(|c| c.dto.clone()).collect(),
                },
                cases,
            });
        }
        let absents = g.eleves.len().saturating_sub(vus.len());
        Ok((lignes, absents, g))
    }

    /// Aperçu d'un collage ou d'un import : **rien n'est écrit**.
    pub async fn eva_v3_apercu_import_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &ImportBody,
    ) -> Result<ImportApercuDto, EntDbError> {
        let (lignes, absents, _g) = self
            .eva_v3_analyser_import(etab, acteur, role, jour, body)
            .await?;
        let mut t = ImportTotauxDto {
            lignes: lignes.len(),
            ..Default::default()
        };
        for l in &lignes {
            if l.dto.rapprochement == "ok" {
                t.rapprochees += 1;
            } else {
                t.non_rapprochees += 1;
            }
            for c in &l.cases {
                match c.dto.etat.as_str() {
                    "nouveau" => t.nouvelles += 1,
                    "modifie" => t.modifiees += 1,
                    "inchange" => t.inchangees += 1,
                    "erreur" | "verrouille" => t.erreurs += 1,
                    _ => {}
                }
            }
        }
        Ok(ImportApercuDto {
            empreinte: empreinte(body.cours_id, &body.mode, &lignes),
            lignes: lignes.into_iter().map(|l| l.dto).collect(),
            totaux: t,
            eleves_absents_du_fichier: absents,
        })
    }

    /// Confirme un import : refait l'analyse, exige la **même empreinte** que l'aperçu, puis écrit les seules
    /// cases « nouveau » et « modifié » des lignes rapprochées, avec la version lue (conflit si elle a changé
    /// depuis). Journal des imports : des comptes, jamais de nom ni de note.
    pub async fn eva_v3_confirmer_import_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &ImportBody,
    ) -> Result<SaisieLotRetourDto, EntDbError> {
        let Some(attendue) = body.empreinte.as_deref() else {
            return Err(refus(
                "l'empreinte de l'aperçu est obligatoire pour confirmer",
            ));
        };
        let (lignes, _absents, _g) = self
            .eva_v3_analyser_import(etab, acteur, role, jour, body)
            .await?;
        if empreinte(body.cours_id, &body.mode, &lignes) != attendue {
            return Err(conflit(
                "l'aperçu n'est plus à jour : les notes ont changé, refaites l'aperçu",
            ));
        }
        let mut cellules: Vec<CelluleSaisieDto> = Vec::new();
        for l in &lignes {
            let Some(e) = &l.dto.eleve else { continue };
            for c in &l.cases {
                if !c.ecrire {
                    continue;
                }
                cellules.push(CelluleSaisieDto {
                    evaluation_id: c.dto.evaluation_id,
                    eleve_id: e.eleve_id,
                    // Le texte analysé est rejoué par la saisie : on renvoie la forme canonique.
                    texte: forme_canonique(&c.dto),
                    version_attendue: c.version_avant,
                    motif: None,
                });
            }
        }
        let lot = SaisieLotBody {
            cours_id: body.cours_id,
            cellules,
            origine: Some(
                if body.mode == "fichier" {
                    "import"
                } else {
                    "saisie"
                }
                .to_string(),
            ),
        };
        self.eva_v3_saisir_lot_interne(etab, acteur, role, jour, &lot, Some(&body.mode))
            .await
    }
}

/// Texte canonique d'une case analysée (code rapide ou nombre à la française), rejoué par la saisie.
fn forme_canonique(c: &ImportCelluleApercuDto) -> String {
    match (c.statut.as_deref(), c.valeur_m) {
        (Some("note"), Some(m)) => ent_types::evaluations_v3::formater_milliemes(m),
        (Some(s), _) => ent_types::evaluations_v3::code_statut(s).to_string(),
        _ => String::new(),
    }
}

/// Regroupe des cellules brutes par élève (utilitaire des tests).
#[allow(dead_code)]
pub(crate) fn par_eleve(c: &[CelluleSaisieDto]) -> BTreeMap<Uuid, usize> {
    let mut m = BTreeMap::new();
    for x in c {
        *m.entry(x.eleve_id).or_insert(0) += 1;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(n: &str, p: &str) -> (String, String) {
        (n.to_string(), p.to_string())
    }

    #[test]
    fn cle_ignore_ordre_accents_et_casse() {
        assert_eq!(cle_nom("MARTIN Léa"), cle_nom("lea martin"));
        assert_eq!(cle_nom("martin, léa"), cle_nom("Léa MARTIN"));
        assert_eq!(cle_nom("Jean-Pierre Dupont"), cle_nom("DUPONT Jean Pierre"));
        assert_ne!(cle_nom("Léa Martin"), cle_nom("Léo Martin"));
        assert_eq!(cle_nom("   "), "");
    }

    #[test]
    fn rapprochement_un_ambigu_aucun() {
        let e = vec![
            el("Martin", "Léa"),
            el("Martin", "Léo"),
            el("Durand", "Léa"),
            el("Durand", "Léa"),
        ];
        assert_eq!(rapprocher(&e, "martin lea"), Rapprochement::Un(0));
        assert_eq!(rapprocher(&e, "Léo MARTIN"), Rapprochement::Un(1));
        assert_eq!(rapprocher(&e, "Durand Léa"), Rapprochement::Ambigu);
        assert_eq!(rapprocher(&e, "Petit Zoé"), Rapprochement::Aucun);
        assert_eq!(rapprocher(&e, ""), Rapprochement::Aucun);
        // Un nom seul ne devine pas : « Martin » est ambigu avec « Martin Léa » / « Martin Léo », donc refusé.
        assert_eq!(rapprocher(&e, "Martin"), Rapprochement::Aucun);
    }

    #[test]
    fn empreinte_stable_et_sensible() {
        let c = Uuid::nil();
        let l = vec![];
        assert_eq!(empreinte(c, "collage", &l), empreinte(c, "collage", &l));
        assert_ne!(empreinte(c, "collage", &l), empreinte(c, "fichier", &l));
    }
}
