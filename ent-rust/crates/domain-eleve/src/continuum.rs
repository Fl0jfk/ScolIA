//! Continuité N-1 / N / N+1 sur la fiche élève.
//!
//! Stance produit : **pas** de year-jump type Charlemagne « Fichier ».
//! Une base vivante ; l'historique se consulte depuis la fiche.
//! Montée rapide = projection N+1 (niveau + options proposées), **sans**
//! valeur juridique de passage — redoublement encore possible.

use crate::absence::annee_scolaire_de;
use crate::{DocTiroir, DomainError, EleveDocument, OptionNature};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScolariteAnneeStatut {
    Historique,
    Courante,
    /// Projection N+1 (montée rapide / préparation rentrée).
    Projetee,
    Reinscription,
}

impl ScolariteAnneeStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Historique => "historique",
            Self::Courante => "courante",
            Self::Projetee => "projetee",
            Self::Reinscription => "reinscription",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "historique" => Some(Self::Historique),
            "courante" => Some(Self::Courante),
            "projetee" => Some(Self::Projetee),
            "reinscription" => Some(Self::Reinscription),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScolariteSource {
    Import,
    Saisie,
    MonteeRapide,
    ReinscriptionParent,
    SyncCourante,
}

impl ScolariteSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Saisie => "saisie",
            Self::MonteeRapide => "montee_rapide",
            Self::ReinscriptionParent => "reinscription_parent",
            Self::SyncCourante => "sync_courante",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "import" => Some(Self::Import),
            "saisie" => Some(Self::Saisie),
            "montee_rapide" => Some(Self::MonteeRapide),
            "reinscription_parent" => Some(Self::ReinscriptionParent),
            "sync_courante" => Some(Self::SyncCourante),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScolariteAnnee {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub annee_scolaire: String,
    pub niveau: Option<String>,
    pub mef: Option<String>,
    pub classe: Option<String>,
    pub regime: Option<String>,
    pub statut: ScolariteAnneeStatut,
    pub source: ScolariteSource,
    pub passage_juridique: bool,
    pub redoublement_possible: bool,
    pub date_montee: Option<NaiveDate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionAnneeStatut {
    Proposee,
    Retenue,
    Retiree,
}

impl OptionAnneeStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposee => "proposee",
            Self::Retenue => "retenue",
            Self::Retiree => "retiree",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "proposee" => Some(Self::Proposee),
            "retenue" => Some(Self::Retenue),
            "retiree" => Some(Self::Retiree),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionAnnee {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub annee_scolaire: String,
    pub nature: OptionNature,
    pub code: String,
    pub libelle: String,
    pub statut: OptionAnneeStatut,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReinscriptionCampagne {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub annee_cible: String,
    pub ouverte: bool,
    /// 0 = pas de vanne paiement ; sinon montant d'acompte requis (centimes).
    pub acompte_centimes: i64,
}

#[derive(Debug, Clone)]
pub struct OuvrirCampagneReinscription {
    pub etablissement_id: Uuid,
    pub annee_cible: String,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigurerAcompteCampagne {
    pub etablissement_id: Uuid,
    pub annee_cible: String,
    pub acompte_centimes: i64,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EnregistrerScolariteAnnee {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub annee_scolaire: String,
    pub niveau: Option<String>,
    pub mef: Option<String>,
    pub classe: Option<String>,
    pub regime: Option<String>,
    pub statut: ScolariteAnneeStatut,
    pub source: ScolariteSource,
    pub passage_juridique: bool,
    pub redoublement_possible: bool,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MonteeRapide {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    /// Année courante de référence (N) — la cible est N+1.
    pub annee_courante: String,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlannedMontee {
    pub scolarite_n1: ScolariteAnnee,
    pub options_proposees: Vec<OptionAnnee>,
}

#[derive(Debug, Clone)]
pub struct ChoisirOptionN1 {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub annee_scolaire: String,
    pub nature: OptionNature,
    pub code: String,
    pub libelle: String,
    pub retenir: bool,
    pub actor_user_id: Option<String>,
}

/// Triplet continuum autour d'un jour (N-1, N, N+1).
pub fn annees_continuum(jour: NaiveDate) -> (String, String, String) {
    let n = annee_scolaire_de(jour);
    let nm1 = annee_relative(&n, -1).expect("N-1");
    let np1 = annee_relative(&n, 1).expect("N+1");
    (nm1, n, np1)
}

/// Décale une année `YYYY-YYYY+1` de `delta` années scolaires.
pub fn annee_relative(annee: &str, delta: i32) -> Result<String, DomainError> {
    let parts: Vec<&str> = annee.split('-').collect();
    if parts.len() != 2 {
        return Err(DomainError::AnneeScolaireInvalide);
    }
    let start: i32 = parts[0]
        .parse()
        .map_err(|_| DomainError::AnneeScolaireInvalide)?;
    let end: i32 = parts[1]
        .parse()
        .map_err(|_| DomainError::AnneeScolaireInvalide)?;
    if end != start + 1 {
        return Err(DomainError::AnneeScolaireInvalide);
    }
    let ns = start + delta;
    Ok(format!("{ns}-{}", ns + 1))
}

/// Infère un niveau lisible depuis classe (« 4B ») ou MEF (« MEF4 »).
pub fn niveau_depuis_classe_ou_mef(
    classe: Option<&str>,
    mef: Option<&str>,
) -> Option<String> {
    if let Some(c) = classe {
        let t = c.trim();
        if t.len() >= 2 {
            let prefix = &t[..t
                .char_indices()
                .find(|(_, ch)| !ch.is_ascii_digit() && *ch != 'e' && *ch != 'è')
                .map(|(i, _)| i)
                .unwrap_or(t.len())];
            // « 4B » → prendre le 1er caractère numérique + e
            if let Some(d) = t.chars().next().filter(|c| c.is_ascii_digit()) {
                return Some(format!("{d}e"));
            }
            let _ = prefix;
        }
        // 2nde / 1re / Tle
        let lower = t.to_ascii_lowercase();
        if lower.starts_with("2n") {
            return Some("2nde".into());
        }
        if lower.starts_with("1") {
            return Some("1re".into());
        }
        if lower.starts_with("t") {
            return Some("Tle".into());
        }
    }
    if let Some(m) = mef {
        let u = m.trim().to_ascii_uppercase();
        if let Some(rest) = u.strip_prefix("MEF") {
            return match rest {
                "6" => Some("6e".into()),
                "5" => Some("5e".into()),
                "4" => Some("4e".into()),
                "3" => Some("3e".into()),
                "2" => Some("2nde".into()),
                "1" => Some("1re".into()),
                "T" | "TLE" => Some("Tle".into()),
                _ => None,
            };
        }
    }
    None
}

pub fn niveau_suivant(niveau: &str) -> Option<String> {
    match normalize_niveau(niveau).as_str() {
        "6e" => Some("5e".into()),
        "5e" => Some("4e".into()),
        "4e" => Some("3e".into()),
        "3e" => Some("2nde".into()),
        "2nde" => Some("1re".into()),
        "1re" => Some("Tle".into()),
        "Tle" => None,
        _ => None,
    }
}

/// Niveau effectif d'une scolarité (champ explicite, sinon classe / MEF).
pub fn niveau_effectif(s: &ScolariteAnnee) -> Option<String> {
    s.niveau
        .clone()
        .or_else(|| niveau_depuis_classe_ou_mef(s.classe.as_deref(), s.mef.as_deref()))
        .map(|n| normalize_niveau(&n))
}

/// Filtre lot : la scolarité courante correspond-elle au niveau source demandé ?
pub fn scolarite_match_niveau(s: &ScolariteAnnee, niveau_filtre: &str) -> bool {
    let want = normalize_niveau(niveau_filtre);
    if want.is_empty() {
        return false;
    }
    niveau_effectif(s).is_some_and(|n| n == want)
}

#[derive(Debug, Clone)]
pub struct MonteeLot {
    pub etablissement_id: Uuid,
    pub annee_courante: String,
    /// Niveau source à monter en masse (ex. « 6e », « 4e »).
    pub niveau_source: String,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LotMonteeIgnore {
    pub eleve_id: Uuid,
    pub motif: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LotMonteeResultat {
    pub niveau_source: String,
    pub niveau_cible: String,
    pub annee_courante: String,
    pub annee_cible: String,
    pub montes: Vec<Uuid>,
    pub ignores: Vec<LotMonteeIgnore>,
}

/// Prépare le filtre lot : valide le niveau, calcule la cible, retient les candidats.
pub fn candidats_lot_montee<'a>(
    courantes: &'a [ScolariteAnnee],
    niveau_source: &str,
) -> Result<(String, String, Vec<&'a ScolariteAnnee>), DomainError> {
    let niveau_norm = normalize_niveau(niveau_source);
    if niveau_norm.is_empty() {
        return Err(DomainError::NiveauInconnu);
    }
    let niveau_cible = niveau_suivant(&niveau_norm).ok_or(DomainError::NiveauSansSuite)?;
    let candidats: Vec<&'a ScolariteAnnee> = courantes
        .iter()
        .filter(|s| scolarite_match_niveau(s, &niveau_norm))
        .collect();
    Ok((niveau_norm, niveau_cible, candidats))
}

pub fn mef_suivant(mef: &str) -> Option<String> {
    match mef.trim().to_ascii_uppercase().as_str() {
        "MEF6" => Some("MEF5".into()),
        "MEF5" => Some("MEF4".into()),
        "MEF4" => Some("MEF3".into()),
        "MEF3" => Some("MEF2".into()),
        "MEF2" => Some("MEF1".into()),
        "MEF1" => Some("MEFT".into()),
        _ => None,
    }
}

pub fn normalize_niveau(n: &str) -> String {
    let t = n.trim().to_ascii_lowercase().replace('è', "e");
    match t.as_str() {
        "6" | "6e" | "6eme" | "6ème" => "6e".into(),
        "5" | "5e" | "5eme" | "5ème" => "5e".into(),
        "4" | "4e" | "4eme" | "4ème" => "4e".into(),
        "3" | "3e" | "3eme" | "3ème" => "3e".into(),
        "2nde" | "2de" | "seconde" => "2nde".into(),
        "1re" | "1ere" | "premiere" | "première" => "1re".into(),
        "tle" | "terminale" | "term" => "Tle".into(),
        other => other.to_string(),
    }
}

pub fn plan_ouvrir_campagne(
    cmd: OuvrirCampagneReinscription,
) -> Result<ReinscriptionCampagne, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let annee = cmd.annee_cible.trim().to_string();
    annee_relative(&annee, 0)?; // valide le format
    Ok(ReinscriptionCampagne {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        annee_cible: annee,
        ouverte: true,
        acompte_centimes: 0,
    })
}

pub fn plan_configurer_acompte_campagne(
    existing: &ReinscriptionCampagne,
    cmd: ConfigurerAcompteCampagne,
) -> Result<ReinscriptionCampagne, DomainError> {
    if existing.etablissement_id != cmd.etablissement_id {
        return Err(DomainError::NotFound);
    }
    if existing.annee_cible != cmd.annee_cible.trim() {
        return Err(DomainError::AnneeScolaireInvalide);
    }
    if cmd.acompte_centimes < 0 {
        return Err(DomainError::AcompteReinscriptionInvalide);
    }
    let mut out = existing.clone();
    out.acompte_centimes = cmd.acompte_centimes;
    Ok(out)
}

/// Vanne paiement : encaisse sur factures acompte ≥ montant requis (0 = off).
pub fn assert_vanne_acompte(
    acompte_requis_centimes: i64,
    encaisse_acompte_centimes: i64,
) -> Result<(), DomainError> {
    if acompte_requis_centimes <= 0 {
        return Ok(());
    }
    if encaisse_acompte_centimes >= acompte_requis_centimes {
        Ok(())
    } else {
        Err(DomainError::AcompteReinscriptionManquant {
            requis: acompte_requis_centimes,
            encaisse: encaisse_acompte_centimes,
        })
    }
}

pub fn plan_enregistrer_scolarite_annee(
    cmd: EnregistrerScolariteAnnee,
) -> Result<ScolariteAnnee, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let annee = cmd.annee_scolaire.trim().to_string();
    annee_relative(&annee, 0)?;
    Ok(ScolariteAnnee {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        annee_scolaire: annee,
        niveau: cmd.niveau.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        mef: cmd.mef.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        classe: cmd.classe.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        regime: cmd.regime.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        statut: cmd.statut,
        source: cmd.source,
        passage_juridique: cmd.passage_juridique,
        redoublement_possible: cmd.redoublement_possible,
        date_montee: None,
    })
}

/// Montée rapide : N → N+1. Niveau monte ; classe fine souvent absente ;
/// **pas** de passage juridique ; redoublement encore possible.
pub fn plan_montee_rapide(
    courante: &ScolariteAnnee,
    options_annee_n: &[(OptionNature, String, String)],
    cmd: MonteeRapide,
    jour: NaiveDate,
) -> Result<PlannedMontee, DomainError> {
    if courante.etablissement_id != cmd.etablissement_id || courante.eleve_id != cmd.eleve_id {
        return Err(DomainError::NotFound);
    }
    if courante.annee_scolaire != cmd.annee_courante {
        return Err(DomainError::AnneeScolaireInvalide);
    }
    let annee_n1 = annee_relative(&cmd.annee_courante, 1)?;
    let niveau_src = courante
        .niveau
        .clone()
        .or_else(|| niveau_depuis_classe_ou_mef(courante.classe.as_deref(), courante.mef.as_deref()))
        .ok_or(DomainError::NiveauInconnu)?;
    let niveau_dst = niveau_suivant(&niveau_src).ok_or(DomainError::NiveauSansSuite)?;
    let mef_dst = courante
        .mef
        .as_deref()
        .and_then(mef_suivant)
        .or_else(|| mef_depuis_niveau(&niveau_dst));

    let scolarite_n1 = ScolariteAnnee {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        annee_scolaire: annee_n1.clone(),
        niveau: Some(niveau_dst),
        mef: mef_dst,
        // Classe fine plus tard — dictée métier.
        classe: None,
        regime: courante.regime.clone(),
        statut: ScolariteAnneeStatut::Projetee,
        source: ScolariteSource::MonteeRapide,
        passage_juridique: false,
        redoublement_possible: true,
        date_montee: Some(jour),
    };

    let mut options_proposees = Vec::new();
    for (nature, code, libelle) in options_annee_n {
        if *nature != OptionNature::Academique {
            continue;
        }
        options_proposees.push(OptionAnnee {
            id: Uuid::new_v4(),
            etablissement_id: cmd.etablissement_id,
            eleve_id: cmd.eleve_id,
            annee_scolaire: annee_n1.clone(),
            nature: *nature,
            code: code.trim().to_string(),
            libelle: libelle.trim().to_string(),
            statut: OptionAnneeStatut::Proposee,
        });
    }

    Ok(PlannedMontee {
        scolarite_n1,
        options_proposees,
    })
}

#[derive(Debug, Clone)]
pub struct ConfirmerReinscription {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub annee_scolaire: String,
    pub actor_user_id: Option<String>,
}

/// Parent confirme la réinscription N+1 (projection déjà posée par montée rapide).
pub fn plan_confirmer_reinscription_parent(
    projection: &ScolariteAnnee,
    cmd: ConfirmerReinscription,
) -> Result<ScolariteAnnee, DomainError> {
    if projection.etablissement_id != cmd.etablissement_id || projection.eleve_id != cmd.eleve_id {
        return Err(DomainError::NotFound);
    }
    if projection.annee_scolaire != cmd.annee_scolaire.trim() {
        return Err(DomainError::AnneeScolaireInvalide);
    }
    if !matches!(
        projection.statut,
        ScolariteAnneeStatut::Projetee | ScolariteAnneeStatut::Reinscription
    ) {
        return Err(DomainError::ReinscriptionNonPrete);
    }
    let mut out = projection.clone();
    out.statut = ScolariteAnneeStatut::Reinscription;
    out.source = ScolariteSource::ReinscriptionParent;
    // Toujours pas juridique : l'admin tranche plus tard.
    out.passage_juridique = false;
    out.redoublement_possible = true;
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExigenceDocReinscription {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub annee_cible: String,
    pub code: String,
    pub libelle: String,
    pub tiroir: DocTiroir,
    pub obligatoire: bool,
}

#[derive(Debug, Clone)]
pub struct PoserExigenceDoc {
    pub etablissement_id: Uuid,
    pub annee_cible: String,
    pub code: String,
    pub libelle: String,
    pub tiroir: DocTiroir,
    pub obligatoire: bool,
    pub actor_user_id: Option<String>,
}

pub fn plan_poser_exigence_doc(cmd: PoserExigenceDoc) -> Result<ExigenceDocReinscription, DomainError> {
    let code = cmd.code.trim().to_ascii_uppercase();
    let libelle = cmd.libelle.trim().to_string();
    let annee = cmd.annee_cible.trim().to_string();
    if code.is_empty() || libelle.is_empty() {
        return Err(DomainError::ExigenceDocInvalide);
    }
    annee_relative(&annee, 0)?;
    Ok(ExigenceDocReinscription {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        annee_cible: annee,
        code,
        libelle,
        tiroir: cmd.tiroir,
        obligatoire: cmd.obligatoire,
    })
}

/// Vanne : chaque exigence obligatoire doit avoir un document élève au même code.
pub fn codes_documents_manquants(
    exigences: &[ExigenceDocReinscription],
    documents: &[EleveDocument],
) -> Vec<String> {
    let present: Vec<String> = documents
        .iter()
        .filter_map(|d| d.code.as_ref())
        .map(|c| c.trim().to_ascii_uppercase())
        .filter(|c| !c.is_empty())
        .collect();
    exigences
        .iter()
        .filter(|e| e.obligatoire)
        .filter(|e| !present.iter().any(|c| c == &e.code))
        .map(|e| e.code.clone())
        .collect()
}

pub fn assert_vannes_documents(
    exigences: &[ExigenceDocReinscription],
    documents: &[EleveDocument],
) -> Result<(), DomainError> {
    let manquants = codes_documents_manquants(exigences, documents);
    if manquants.is_empty() {
        Ok(())
    } else {
        Err(DomainError::DocumentsReinscriptionManquants(manquants.join(",")))
    }
}

pub fn plan_choisir_option_n1(cmd: ChoisirOptionN1) -> Result<OptionAnnee, DomainError> {
    let code = cmd.code.trim().to_string();
    let libelle = cmd.libelle.trim().to_string();
    if code.is_empty() || libelle.is_empty() {
        return Err(DomainError::EmptyAttributValeur);
    }
    annee_relative(&cmd.annee_scolaire, 0)?;
    Ok(OptionAnnee {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        annee_scolaire: cmd.annee_scolaire.trim().to_string(),
        nature: cmd.nature,
        code,
        libelle,
        statut: if cmd.retenir {
            OptionAnneeStatut::Retenue
        } else {
            OptionAnneeStatut::Proposee
        },
    })
}

fn mef_depuis_niveau(niveau: &str) -> Option<String> {
    match normalize_niveau(niveau).as_str() {
        "6e" => Some("MEF6".into()),
        "5e" => Some("MEF5".into()),
        "4e" => Some("MEF4".into()),
        "3e" => Some("MEF3".into()),
        "2nde" => Some("MEF2".into()),
        "1re" => Some("MEF1".into()),
        "Tle" => Some("MEFT".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuum_triplet() {
        let jour = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
        let (nm1, n, np1) = annees_continuum(jour);
        assert_eq!(nm1, "2025-2026");
        assert_eq!(n, "2026-2027");
        assert_eq!(np1, "2027-2028");
    }

    #[test]
    fn montee_4e_vers_3e_sans_passage_juridique() {
        let etab = Uuid::new_v4();
        let eleve = Uuid::new_v4();
        let courante = ScolariteAnnee {
            id: Uuid::new_v4(),
            etablissement_id: etab,
            eleve_id: eleve,
            annee_scolaire: "2026-2027".into(),
            niveau: Some("4e".into()),
            mef: Some("MEF4".into()),
            classe: Some("4B".into()),
            regime: Some("DP".into()),
            statut: ScolariteAnneeStatut::Courante,
            source: ScolariteSource::SyncCourante,
            passage_juridique: true,
            redoublement_possible: false,
            date_montee: None,
        };
        let planned = plan_montee_rapide(
            &courante,
            &[(OptionNature::Academique, "LV1_ANG".into(), "Anglais LV1".into())],
            MonteeRapide {
                etablissement_id: etab,
                eleve_id: eleve,
                annee_courante: "2026-2027".into(),
                actor_user_id: None,
            },
            NaiveDate::from_ymd_opt(2027, 2, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(planned.scolarite_n1.annee_scolaire, "2027-2028");
        assert_eq!(planned.scolarite_n1.niveau.as_deref(), Some("3e"));
        assert_eq!(planned.scolarite_n1.mef.as_deref(), Some("MEF3"));
        assert!(planned.scolarite_n1.classe.is_none());
        assert!(!planned.scolarite_n1.passage_juridique);
        assert!(planned.scolarite_n1.redoublement_possible);
        assert_eq!(planned.options_proposees.len(), 1);
        assert_eq!(
            planned.options_proposees[0].statut,
            OptionAnneeStatut::Proposee
        );
    }

    #[test]
    fn niveau_depuis_classe() {
        assert_eq!(
            niveau_depuis_classe_ou_mef(Some("4B"), None).as_deref(),
            Some("4e")
        );
        assert_eq!(
            niveau_depuis_classe_ou_mef(None, Some("MEF5")).as_deref(),
            Some("5e")
        );
    }

    #[test]
    fn vanne_acompte_bloque_si_insuffisant() {
        assert!(assert_vanne_acompte(0, 0).is_ok());
        assert!(assert_vanne_acompte(5000, 5000).is_ok());
        assert!(assert_vanne_acompte(5000, 4999).is_err());
    }

    #[test]
    fn vanne_docs_bloque_si_code_manquant() {
        let etab = Uuid::new_v4();
        let exigences = vec![ExigenceDocReinscription {
            id: Uuid::new_v4(),
            etablissement_id: etab,
            annee_cible: "2027-2028".into(),
            code: "ASSURANCE_RC".into(),
            libelle: "Assurance RC".into(),
            tiroir: DocTiroir::Famille,
            obligatoire: true,
        }];
        assert!(assert_vannes_documents(&exigences, &[]).is_err());
        let ok_doc = EleveDocument {
            id: Uuid::new_v4(),
            etablissement_id: etab,
            eleve_id: Uuid::new_v4(),
            tiroir: DocTiroir::Famille,
            titre: "Attestation".into(),
            code: Some("ASSURANCE_RC".into()),
            mime_type: None,
            storage_key: None,
            confidentialite: crate::DocConfidentialite::Standard,
            source: "upload".into(),
            created_by_user_id: None,
        };
        assert!(assert_vannes_documents(&exigences, &[ok_doc]).is_ok());
    }

    #[test]
    fn candidats_lot_filtre_6e() {
        let etab = Uuid::new_v4();
        let e6 = Uuid::new_v4();
        let e5 = Uuid::new_v4();
        let courantes = vec![
            ScolariteAnnee {
                id: Uuid::new_v4(),
                etablissement_id: etab,
                eleve_id: e6,
                annee_scolaire: "2026-2027".into(),
                niveau: Some("6e".into()),
                mef: Some("MEF6".into()),
                classe: Some("6A".into()),
                regime: None,
                statut: ScolariteAnneeStatut::Courante,
                source: ScolariteSource::SyncCourante,
                passage_juridique: true,
                redoublement_possible: false,
                date_montee: None,
            },
            ScolariteAnnee {
                id: Uuid::new_v4(),
                etablissement_id: etab,
                eleve_id: e5,
                annee_scolaire: "2026-2027".into(),
                niveau: Some("5e".into()),
                mef: Some("MEF5".into()),
                classe: Some("5A".into()),
                regime: None,
                statut: ScolariteAnneeStatut::Courante,
                source: ScolariteSource::SyncCourante,
                passage_juridique: true,
                redoublement_possible: false,
                date_montee: None,
            },
        ];
        let (src, dst, cands) = candidats_lot_montee(&courantes, "6ème").unwrap();
        assert_eq!(src, "6e");
        assert_eq!(dst, "5e");
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].eleve_id, e6);
    }
}
