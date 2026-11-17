//! **Simulation** de l'import (lot R2) : rejoue la décision de reprise de chaque ligne comprise **sans rien
//! écrire** dans les tables métier, et produit le rapport de ce qui serait créé, mis à jour, laissé
//! inchangé, archivé, ignoré, refusé ou laissé « à décider ».
//!
//! Fonction pure : entrée = le décodage et les correspondances déjà enregistrées, sortie = un rapport
//! déterministe. Invariant du rapport (T0 §7.2, par objet) :
//! `lues = créées + mises à jour + inchangées + archivées + ignorées + refusées + à décider`.
//!
//! Règles :
//! - `reprise` : sans correspondance → **créée** ; avec correspondance et même empreinte → **inchangée** ;
//!   empreinte différente → **mise à jour** ; objet « modifié depuis la reprise » → **à décider** (jamais
//!   écrasé) ;
//! - `rapprochement` : jamais de fiche du cœur sans décision (T0 §8.6) : sans correspondance, **à décider**
//!   (file préparée pour R3) ; avec correspondance, mêmes règles que `reprise` ;
//! - `archive` : **archivée** (conservée en lecture, non reprise).

use crate::correspondance::{resoudre, Existante, Existantes, Statut};
use crate::decodage::{Decodage, ObjetRapport};
use crate::perimetre::{domaine_collection, Disposition};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct CompteSimulation {
    pub lues: u64,
    pub creees: u64,
    pub mises_a_jour: u64,
    pub inchangees: u64,
    pub archivees: u64,
    pub ignorees: u64,
    pub refusees: u64,
    pub a_decider: u64,
}

impl CompteSimulation {
    pub fn reprises(&self) -> u64 {
        self.creees + self.mises_a_jour + self.inchangees
    }

    pub fn invariant(&self) -> bool {
        self.lues
            == self.reprises() + self.archivees + self.ignorees + self.refusees + self.a_decider
    }

    fn ajouter(&mut self, o: &CompteSimulation) {
        self.lues += o.lues;
        self.creees += o.creees;
        self.mises_a_jour += o.mises_a_jour;
        self.inchangees += o.inchangees;
        self.archivees += o.archivees;
        self.ignorees += o.ignorees;
        self.refusees += o.refusees;
        self.a_decider += o.a_decider;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Creee,
    MiseAJour,
}

/// Création ou mise à jour que l'application ferait (jamais faite par la simulation).
#[derive(Debug, Clone, Serialize)]
pub struct Proposition {
    pub objet: String,
    /// Identifiant source réel : en mémoire seulement (annexe nominative), pas dans le rapport.
    #[serde(skip)]
    pub source_id: String,
    pub source_ref: String,
    pub cible_objet: String,
    pub cible_id: String,
    pub action: Action,
    #[serde(skip)]
    pub empreinte: String,
}

/// Ligne laissée à la décision humaine (R3).
#[derive(Debug, Clone, Serialize)]
pub struct ADecider {
    pub objet: String,
    #[serde(skip)]
    pub source_id: String,
    pub source_ref: String,
    pub cible_objet: String,
    pub cible_id: String,
    pub motif: String,
    #[serde(skip)]
    pub empreinte: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RapportSimulation {
    pub etablissement_cible: String,
    pub par_objet: BTreeMap<String, CompteSimulation>,
    pub total: CompteSimulation,
    pub propositions: Vec<Proposition>,
    pub a_decider: Vec<ADecider>,
    /// Aucune écriture métier : constante affirmée dans le rapport (vérifiée par les tests de `ent-db`).
    pub ecritures_metier: u64,
}

impl RapportSimulation {
    pub fn invariants_ok(&self) -> bool {
        self.par_objet.values().all(CompteSimulation::invariant) && self.total.invariant()
    }
}

/// Forme citable d'un identifiant source : tel quel s'il est sûr, sinon une empreinte courte.
pub fn reference_sure(source_id: &str) -> String {
    let ok = !source_id.is_empty()
        && source_id.len() <= 200
        && source_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.' | ':'));
    if ok {
        source_id.to_string()
    } else {
        format!(
            "h:{}",
            &hex::encode(Sha256::digest(source_id.as_bytes()))[..16]
        )
    }
}

enum Decision {
    Creee,
    MiseAJour,
    Inchangee,
    ADecider(&'static str),
}

fn decider(
    disposition: &Disposition,
    existante: Option<&Existante>,
    empreinte: &str,
) -> Option<Decision> {
    match (disposition, existante) {
        (Disposition::Reprise, None) => Some(Decision::Creee),
        (Disposition::Rapprochement, None) => Some(Decision::ADecider("rapprochement_a_decider")),
        (Disposition::Reprise | Disposition::Rapprochement, Some(e)) => Some(match e.statut {
            Statut::ADecider => Decision::ADecider("rapprochement_a_decider"),
            Statut::Ignoree => return None,
            _ if e.modifie_depuis_reprise => Decision::ADecider("modifie_depuis_reprise"),
            _ if e.empreinte_source.as_deref() == Some(empreinte) => Decision::Inchangee,
            _ => Decision::MiseAJour,
        }),
        _ => None,
    }
}

struct Ligne<'a> {
    objet: &'a str,
    disposition: &'a Disposition,
    cible_objet: String,
    source_id: &'a str,
    empreinte: &'a str,
}

fn traiter(l: &Ligne, etab: &str, existantes: &Existantes, r: &mut RapportSimulation) {
    let compte = r.par_objet.entry(l.objet.to_string()).or_default();
    if *l.disposition == Disposition::Archive {
        compte.archivees += 1;
        return;
    }
    let existante = existantes.get(&(l.objet.to_string(), l.source_id.to_string()));
    let cible_id = resoudre(etab, l.objet, l.source_id, existantes);
    match decider(l.disposition, existante, l.empreinte) {
        // Correspondance marquée « ignorée » par une décision humaine : comptée ignorée.
        None => compte.ignorees += 1,
        Some(Decision::Inchangee) => compte.inchangees += 1,
        Some(Decision::Creee) => {
            compte.creees += 1;
            r.propositions.push(Proposition {
                objet: l.objet.to_string(),
                source_id: l.source_id.to_string(),
                source_ref: reference_sure(l.source_id),
                cible_objet: l.cible_objet.clone(),
                cible_id,
                action: Action::Creee,
                empreinte: l.empreinte.to_string(),
            });
        }
        Some(Decision::MiseAJour) => {
            compte.mises_a_jour += 1;
            r.propositions.push(Proposition {
                objet: l.objet.to_string(),
                source_id: l.source_id.to_string(),
                source_ref: reference_sure(l.source_id),
                cible_objet: l.cible_objet.clone(),
                cible_id,
                action: Action::MiseAJour,
                empreinte: l.empreinte.to_string(),
            });
        }
        Some(Decision::ADecider(motif)) => {
            compte.a_decider += 1;
            r.a_decider.push(ADecider {
                objet: l.objet.to_string(),
                source_id: l.source_id.to_string(),
                source_ref: reference_sure(l.source_id),
                cible_objet: l.cible_objet.clone(),
                cible_id,
                motif: motif.to_string(),
                empreinte: l.empreinte.to_string(),
            });
        }
    }
}

fn base(o: &ObjetRapport) -> CompteSimulation {
    CompteSimulation {
        lues: o.lues,
        refusees: o.refusees,
        ignorees: o.ignorees,
        ..Default::default()
    }
}

/// Identifiant source d'un document S3 : empreinte de sa clé (la clé contient un jeton secret).
pub fn reference_document_s3(cle: &str) -> String {
    hex::encode(Sha256::digest(cle.as_bytes()))
}

/// Rejoue la reprise sans rien écrire. `etablissement_cible` : UUID de l'établissement ScolIA cible.
pub fn simuler(
    etablissement_cible: &str,
    d: &Decodage,
    existantes: &Existantes,
) -> RapportSimulation {
    let mut r = RapportSimulation {
        etablissement_cible: etablissement_cible.to_string(),
        ..Default::default()
    };
    // Bases : lues / ignorées / refusées du décodage, par objet de simulation.
    let conteneurs = ["ent_collection_record", "ent_collection_attr"];
    for (t, o) in &d.rapport.tables {
        if o.lues > 0 && !conteneurs.contains(&t.as_str()) {
            r.par_objet.insert(t.clone(), base(o));
        }
    }
    let mut reste = CompteSimulation::default();
    for t in conteneurs {
        if let Some(o) = d.rapport.tables.get(t) {
            reste.ajouter(&base(o));
        }
    }
    for (c, o) in &d.rapport.collections {
        let b = base(o);
        reste.lues = reste.lues.saturating_sub(b.lues);
        reste.refusees = reste.refusees.saturating_sub(b.refusees);
        reste.ignorees = reste.ignorees.saturating_sub(b.ignorees);
        r.par_objet.insert(format!("collection:{c}"), b);
    }
    if reste.lues > 0 {
        r.par_objet.insert("collection:(sans nom)".into(), reste);
    }
    // Lignes comprises du dump.
    for t in d.tables.values() {
        for l in &t.lignes {
            let cible = match l.objet.strip_prefix("collection:") {
                Some(c) => domaine_collection(c).to_string(),
                None => l.domaine.clone(),
            };
            traiter(
                &Ligne {
                    objet: &l.objet,
                    disposition: &l.disposition,
                    cible_objet: cible,
                    source_id: &l.cle,
                    empreinte: &l.empreinte,
                },
                etablissement_cible,
                existantes,
                &mut r,
            );
        }
    }
    // Objets S3 : dépôts en attente décodés ; les autres natures sont rapportées telles que décodées.
    for (nature, o) in &d.rapport.s3 {
        let nom = format!("s3:{nature}");
        let mut b = base(o);
        // Les fichiers sont copiés par le lot « fichiers » (R7) : explicitement ignorés ici.
        if nature == "Fichier" {
            b.ignorees += o.comprises;
        }
        r.par_objet.insert(nom, b);
    }
    for doc in d.documents_s3.iter().filter(|x| !x.expire) {
        let (objet, domaine) = match doc.nature {
            crate::decodage::s3json::NatureS3::RentreeEnAttente => {
                ("s3:RentreeEnAttente", "inscriptions")
            }
            _ => ("s3:DemandeEnAttente", "demandes"),
        };
        let sid = reference_document_s3(&doc.cle);
        let empreinte = hex::encode(Sha256::digest(doc.valeur.to_string().as_bytes()));
        traiter(
            &Ligne {
                objet,
                disposition: &Disposition::Reprise,
                cible_objet: domaine.to_string(),
                source_id: &sid,
                empreinte: &empreinte,
            },
            etablissement_cible,
            existantes,
            &mut r,
        );
    }
    // Les objets sans ligne ne figurent pas ; total.
    let comptes: Vec<CompteSimulation> = r.par_objet.values().cloned().collect();
    for c in &comptes {
        r.total.ajouter(c);
    }
    r.propositions
        .sort_by(|a, b| (&a.objet, &a.source_id).cmp(&(&b.objet, &b.source_id)));
    r.a_decider
        .sort_by(|a, b| (&a.objet, &a.source_id).cmp(&(&b.objet, &b.source_id)));
    r
}

/// Ce que l'**application** enregistrerait dans la table de correspondance (sans rien créer dans le
/// métier) : pour chaque proposition, une correspondance `creee` ou mise à jour ; pour chaque ligne à
/// décider, une correspondance `a_decider`. Fonction pure, utilisée pour vérifier l'idempotence.
pub fn correspondances_a_enregistrer(r: &RapportSimulation, existantes: &Existantes) -> Existantes {
    let mut m = existantes.clone();
    for p in &r.propositions {
        m.insert(
            (p.objet.clone(), p.source_id.clone()),
            Existante {
                cible_id: p.cible_id.clone(),
                empreinte_source: Some(p.empreinte.clone()),
                modifie_depuis_reprise: false,
                statut: Statut::Creee,
            },
        );
    }
    for a in &r.a_decider {
        m.entry((a.objet.clone(), a.source_id.clone()))
            .or_insert(Existante {
                cible_id: a.cible_id.clone(),
                empreinte_source: Some(a.empreinte.clone()),
                modifie_depuis_reprise: false,
                statut: Statut::ADecider,
            });
    }
    m
}

/// Rapport lisible, sans aucune valeur de donnée.
pub fn texte(r: &RapportSimulation) -> String {
    let mut s = String::from("=== Simulation de la reprise (aucune écriture dans le métier) ===\n");
    s.push_str(&format!(
        "Établissement cible : {}\n\n",
        r.etablissement_cible
    ));
    s.push_str("objet : lues = créées + mises à jour + inchangées + archivées + ignorées + refusées + à décider\n");
    for (o, c) in &r.par_objet {
        s.push_str(&format!(
            "{o} : {} = {} + {} + {} + {} + {} + {} + {}\n",
            c.lues,
            c.creees,
            c.mises_a_jour,
            c.inchangees,
            c.archivees,
            c.ignorees,
            c.refusees,
            c.a_decider
        ));
    }
    let t = &r.total;
    s.push_str(&format!(
        "\nTOTAL : {} = {} + {} + {} + {} + {} + {} + {}\n",
        t.lues,
        t.creees,
        t.mises_a_jour,
        t.inchangees,
        t.archivees,
        t.ignorees,
        t.refusees,
        t.a_decider
    ));
    s.push_str(&format!(
        "À décider (file préparée pour R3) : {}\n",
        r.a_decider.len()
    ));
    s.push_str(&format!(
        "Écritures dans les tables métier : {}\n",
        r.ecritures_metier
    ));
    s.push_str(if r.invariants_ok() {
        "Invariant : respecté\n"
    } else {
        "Invariant : VIOLÉ\n"
    });
    s
}
