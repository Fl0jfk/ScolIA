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
use crate::rapprochement::{
    est_objet_r3, rapprocher, Candidat, CibleRapprochement, Contexte, Motif, Resume,
    SourceRapprochement, Verdict,
};
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
    /// Fiches de l'export reliées à une fiche **existante** de ScolIA (rapprochement R3) : un lien à
    /// enregistrer, aucune fiche créée ni modifiée.
    pub rapprochees: u64,
}

impl CompteSimulation {
    pub fn reprises(&self) -> u64 {
        self.creees + self.mises_a_jour + self.inchangees
    }

    pub fn invariant(&self) -> bool {
        self.lues
            == self.reprises()
                + self.archivees
                + self.ignorees
                + self.refusees
                + self.a_decider
                + self.rapprochees
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
        self.rapprochees += o.rapprochees;
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

/// Lien « cette fiche de l'export est cette fiche de ScolIA » que l'application enregistrerait (règle forte
/// ou décision humaine rejouée). N'écrit rien dans le métier.
#[derive(Debug, Clone, Serialize)]
pub struct Rapprochee {
    pub objet: String,
    #[serde(skip)]
    pub source_id: String,
    pub source_ref: String,
    pub cible_objet: String,
    #[serde(skip)]
    pub cible_id: String,
    /// Règle qui a rapproché (`ine_unique`, `dates_identiques`, `decision_rattacher`…).
    pub regle: String,
    #[serde(skip)]
    pub empreinte: String,
}

/// Ligne de la file « à décider » avec ses candidats, ses raisons et ses niveaux de confiance. Contient des
/// données personnelles minimisées : **jamais sérialisée dans le rapport** (elle va dans la trace d'`ent-db`).
#[derive(Debug, Clone)]
pub struct LigneFile {
    pub objet: String,
    pub source_id: String,
    pub source_ref: String,
    pub cible_objet: String,
    pub motif: Motif,
    pub candidats: Vec<Candidat>,
    pub resume_source: Resume,
    pub empreinte: String,
}

/// Fiche de l'export conservée en archive par le rapprochement (élève sorti absent du cœur, année passée).
#[derive(Debug, Clone, Serialize)]
pub struct ArchiveeRapprochement {
    pub objet: String,
    pub source_ref: String,
    pub raison: String,
}

/// Synthèse du rapprochement, **sans aucune valeur de donnée** : va dans le rapport.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct SyntheseRapprochement {
    pub rapprochees_par_regle: BTreeMap<String, u64>,
    pub a_decider_par_motif: BTreeMap<String, u64>,
    pub a_decider_par_confiance: BTreeMap<String, u64>,
    pub archivees_par_raison: BTreeMap<String, u64>,
    /// Lignes que le rapprochement R3 ne traite pas (dépendent d'un autre lot) : restent « à décider ».
    pub hors_r3: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RapportSimulation {
    pub etablissement_cible: String,
    pub par_objet: BTreeMap<String, CompteSimulation>,
    pub total: CompteSimulation,
    pub propositions: Vec<Proposition>,
    pub rapprochees: Vec<Rapprochee>,
    pub a_decider: Vec<ADecider>,
    #[serde(skip)]
    pub file: Vec<LigneFile>,
    pub archivees_rapprochement: Vec<ArchiveeRapprochement>,
    pub rapprochement: SyntheseRapprochement,
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
    /// Lien posé par une décision humaine, jamais encore enregistré par une exécution : aucune fiche
    /// créée ni modifiée dans ScolIA (le cœur fait foi).
    Rapprochee,
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
            Statut::Rapprochee
                if *disposition == Disposition::Rapprochement && e.empreinte_source.is_none() =>
            {
                Decision::Rapprochee
            }
            Statut::Creee
                if *disposition == Disposition::Rapprochement && e.empreinte_source.is_none() =>
            {
                Decision::Creee
            }
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
        Some(Decision::Rapprochee) => {
            compte.rapprochees += 1;
            r.rapprochees.push(Rapprochee {
                objet: l.objet.to_string(),
                source_id: l.source_id.to_string(),
                source_ref: reference_sure(l.source_id),
                cible_objet: l.cible_objet.clone(),
                cible_id,
                regle: "decision_rattacher".to_string(),
                empreinte: l.empreinte.to_string(),
            });
        }
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

/// **Simulation avec rapprochement** (lot R3) : la simulation de R2, puis le rapprochement des référentiels
/// et des personnes. Les lignes de rapprochement sans décision deviennent :
/// * **rapprochées** (identité forte ou équivalence validée) : un lien à enregistrer, aucune fiche touchée ;
/// * **archivées** (élève sorti absent du cœur, année passée absente) ;
/// * **à décider**, avec leurs candidats, leurs raisons et leur confiance (file `rep_proposition*`).
///
/// `jour` est le jour de référence **fourni** (jamais l'horloge). Aucune écriture, déterministe.
pub fn simuler_avec_rapprochement(
    etablissement_cible: &str,
    d: &Decodage,
    existantes: &Existantes,
    cible: &CibleRapprochement,
    jour: chrono::NaiveDate,
) -> RapportSimulation {
    let mut r = simuler(etablissement_cible, d, existantes);
    let source = SourceRapprochement::extraire(d);
    let verdicts = rapprocher(&source, cible, &Contexte::new(jour, existantes));
    let a_traiter = std::mem::take(&mut r.a_decider);
    for mut a in a_traiter {
        let cle = (a.objet.clone(), a.source_id.clone());
        let en_attente_de_regle = a.motif == "rapprochement_a_decider";
        if !en_attente_de_regle {
            r.a_decider.push(a);
            continue;
        }
        if !est_objet_r3(&a.objet) {
            a.motif = "rapprochement_hors_r3".to_string();
            r.rapprochement.hors_r3 += 1;
            r.a_decider.push(a);
            continue;
        }
        match verdicts.get(&cle) {
            Some(Verdict::Automatique {
                cible_id, regle, ..
            }) => {
                retirer_a_decider(&mut r, &a.objet);
                let compte = r.par_objet.entry(a.objet.clone()).or_default();
                compte.rapprochees += 1;
                r.total.rapprochees += 1;
                *r.rapprochement
                    .rapprochees_par_regle
                    .entry((*regle).to_string())
                    .or_default() += 1;
                r.rapprochees.push(Rapprochee {
                    objet: a.objet,
                    source_id: a.source_id,
                    source_ref: a.source_ref,
                    cible_objet: a.cible_objet,
                    cible_id: cible_id.clone(),
                    regle: (*regle).to_string(),
                    empreinte: a.empreinte,
                });
            }
            Some(Verdict::Archive { raison }) => {
                retirer_a_decider(&mut r, &a.objet);
                let compte = r.par_objet.entry(a.objet.clone()).or_default();
                compte.archivees += 1;
                r.total.archivees += 1;
                *r.rapprochement
                    .archivees_par_raison
                    .entry((*raison).to_string())
                    .or_default() += 1;
                r.archivees_rapprochement.push(ArchiveeRapprochement {
                    objet: a.objet,
                    source_ref: a.source_ref,
                    raison: (*raison).to_string(),
                });
            }
            Some(Verdict::ADecider {
                motif,
                candidats,
                resume_source,
            }) => {
                a.motif = motif.code().to_string();
                *r.rapprochement
                    .a_decider_par_motif
                    .entry(a.motif.clone())
                    .or_default() += 1;
                let meilleure = candidats
                    .iter()
                    .map(|c| c.confiance.code())
                    .next()
                    .unwrap_or("aucune");
                *r.rapprochement
                    .a_decider_par_confiance
                    .entry(meilleure.to_string())
                    .or_default() += 1;
                r.file.push(LigneFile {
                    objet: a.objet.clone(),
                    source_id: a.source_id.clone(),
                    source_ref: a.source_ref.clone(),
                    cible_objet: a.cible_objet.clone(),
                    motif: *motif,
                    candidats: candidats.clone(),
                    resume_source: resume_source.clone(),
                    empreinte: a.empreinte.clone(),
                });
                r.a_decider.push(a);
            }
            None => r.a_decider.push(a),
        }
    }
    r.rapprochees
        .sort_by(|a, b| (&a.objet, &a.source_id).cmp(&(&b.objet, &b.source_id)));
    r.file
        .sort_by(|a, b| (&a.objet, &a.source_id).cmp(&(&b.objet, &b.source_id)));
    r.archivees_rapprochement
        .sort_by(|a, b| (&a.objet, &a.source_ref).cmp(&(&b.objet, &b.source_ref)));
    r
}

/// Une ligne cesse d'être « à décider » : décompte par objet et en total.
fn retirer_a_decider(r: &mut RapportSimulation, objet: &str) {
    if let Some(c) = r.par_objet.get_mut(objet) {
        c.a_decider = c.a_decider.saturating_sub(1);
    }
    r.total.a_decider = r.total.a_decider.saturating_sub(1);
}

/// Ce que l'**application** enregistrerait dans la table de correspondance (sans rien créer dans le
/// métier) : pour chaque proposition, une correspondance `creee` ou mise à jour ; pour chaque ligne à
/// décider, une correspondance `a_decider`. Fonction pure, utilisée pour vérifier l'idempotence.
pub fn correspondances_a_enregistrer(r: &RapportSimulation, existantes: &Existantes) -> Existantes {
    let mut m = existantes.clone();
    for p in &r.propositions {
        let cle = (p.objet.clone(), p.source_id.clone());
        // Un lien déjà posé par une décision garde son statut : on n'enregistre que l'empreinte.
        let statut = match m.get(&cle).map(|e| e.statut) {
            Some(s @ (Statut::Rapprochee | Statut::Creee)) => s,
            _ => Statut::Creee,
        };
        m.insert(
            cle,
            Existante {
                cible_id: p.cible_id.clone(),
                empreinte_source: Some(p.empreinte.clone()),
                modifie_depuis_reprise: false,
                statut,
            },
        );
    }
    for x in &r.rapprochees {
        m.insert(
            (x.objet.clone(), x.source_id.clone()),
            Existante {
                cible_id: x.cible_id.clone(),
                empreinte_source: Some(x.empreinte.clone()),
                modifie_depuis_reprise: false,
                statut: Statut::Rapprochee,
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
    s.push_str("objet : lues = créées + mises à jour + inchangées + archivées + ignorées + refusées + à décider + rapprochées\n");
    for (o, c) in &r.par_objet {
        s.push_str(&format!(
            "{o} : {} = {} + {} + {} + {} + {} + {} + {} + {}\n",
            c.lues,
            c.creees,
            c.mises_a_jour,
            c.inchangees,
            c.archivees,
            c.ignorees,
            c.refusees,
            c.a_decider,
            c.rapprochees
        ));
    }
    let t = &r.total;
    s.push_str(&format!(
        "\nTOTAL : {} = {} + {} + {} + {} + {} + {} + {} + {}\n",
        t.lues,
        t.creees,
        t.mises_a_jour,
        t.inchangees,
        t.archivees,
        t.ignorees,
        t.refusees,
        t.a_decider,
        t.rapprochees
    ));
    let z = &r.rapprochement;
    let liste = |m: &BTreeMap<String, u64>| {
        if m.is_empty() {
            "aucune".to_string()
        } else {
            m.iter()
                .map(|(k, v)| format!("{k} {v}"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    s.push_str(&format!(
        "Rapprochement (R3) — rapprochées par règle : {} ; à décider par motif : {} ; à décider par meilleure confiance : {} ; archivées : {} ; lignes hors R3 : {}\n",
        liste(&z.rapprochees_par_regle),
        liste(&z.a_decider_par_motif),
        liste(&z.a_decider_par_confiance),
        liste(&z.archivees_par_raison),
        z.hors_r3
    ));
    s.push_str(&format!("À décider : {}\n", r.a_decider.len()));
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
