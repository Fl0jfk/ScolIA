//! Socle 2 (l'élève), lot E4 — scolarité : scolarisations datées, statut DÉRIVÉ, capacité
//! juridique, majorité, mobilité, modalités, dates de changement de régime. Logique
//! **pure**, sans accès base (les fonctions SQL `ent_eleve_statut_a` et
//! `ent_eleve_majorite_le` en sont le miroir, vérifié par les tests de base).
//!
//! Convention de date (celle de l'existant) : `date_sortie` est le PREMIER jour où l'élève
//! n'est plus dans l'établissement ; la scolarisation couvre `[date_entree, date_sortie[`.

use chrono::{Datelike, Duration, NaiveDate};

/// Erreur de règle de scolarité (messages sans donnée personnelle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErreurScolarite {
    #[error("{0}")]
    Invalide(&'static str),
    #[error("{0}")]
    Conflit(&'static str),
}

macro_rules! codes {
    ($(#[$m:meta])* $v:vis enum $nom:ident { $($var:ident => $code:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $v enum $nom { $($var),+ }
        impl $nom {
            pub const TOUS: &'static [Self] = &[$(Self::$var),+];
            pub fn code(self) -> &'static str { match self { $(Self::$var => $code),+ } }
            pub fn parse(s: &str) -> Option<Self> { Self::TOUS.iter().copied().find(|x| x.code() == s) }
        }
    };
}

codes! {
    /// Motif d'entrée (codes du logiciel).
    pub enum MotifEntree {
        PremiereInscription => "premiere_inscription",
        ChangementEtablissement => "changement_etablissement",
        Retour => "retour",
        PassageInterne => "passage_interne",
        Reprise => "reprise",
    }
}

codes! {
    /// Motif de sortie (codes du logiciel ; un motif local se rattache à l'un d'eux).
    pub enum MotifSortie {
        FinDeCycle => "fin_de_cycle",
        ChangementEtablissement => "changement_etablissement",
        Demenagement => "demenagement",
        ExclusionDefinitive => "exclusion_definitive",
        Abandon => "abandon",
        FinObligation => "fin_obligation",
        InstructionFamille => "instruction_famille",
        Etranger => "etranger",
        Deces => "deces",
        Autre => "autre",
    }
}

codes! {
    /// Nature de l'établissement d'origine ou de destination.
    pub enum TypeEtablissement {
        Public => "public",
        PriveSousContrat => "prive_sous_contrat",
        PriveHorsContrat => "prive_hors_contrat",
        InstructionFamille => "instruction_famille",
        Etranger => "etranger",
        Inconnu => "inconnu",
    }
}

codes! {
    /// Statut de l'élève, DÉDUIT des scolarisations (jamais saisi). `Archive` est réservé :
    /// les durées de conservation relèvent du DPO et ne sont pas inventées.
    pub enum StatutEleve {
        AVenir => "a_venir",
        Scolarise => "scolarise",
        Sorti => "sorti",
        Archive => "archive",
    }
}

codes! {
    pub enum EtatInscription {
        Prevue => "prevue",
        Confirmee => "confirmee",
        EnCours => "en_cours",
        Terminee => "terminee",
        Annulee => "annulee",
    }
}

codes! {
    pub enum StatutApprenant {
        Scolaire => "scolaire",
        Apprenti => "apprenti",
        Autre => "autre",
    }
}

codes! {
    pub enum NatureCapacite {
        Emancipation => "emancipation",
        Tutelle => "tutelle",
        Curatelle => "curatelle",
        SauvegardeJustice => "sauvegarde_justice",
        HabilitationFamiliale => "habilitation_familiale",
    }
}

codes! {
    pub enum TypeMobilite {
        Etranger => "etranger",
        Soins => "soins",
        Cned => "cned",
        Autre => "autre",
    }
}

codes! {
    /// Modalité de scolarisation [NV : intitulés officiels à vérifier].
    pub enum CodeModalite {
        TempsPartiel => "temps_partiel",
        DoubleInscriptionCned => "double_inscription_cned",
        ScolarisationPartagee => "scolarisation_partagee",
        Upe2a => "upe2a",
        Ulis => "ulis",
        Autre => "autre",
    }
}

codes! {
    /// Motif codé d'une dérogation de changement de régime hors des dates autorisées (R-E5).
    pub enum MotifDerogationRegime {
        Demenagement => "demenagement",
        RaisonMedicale => "raison_medicale",
        SituationFamiliale => "situation_familiale",
        DecisionEtablissement => "decision_etablissement",
    }
}

/// Une scolarisation, vue par la logique pure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Periode {
    pub date_entree: NaiveDate,
    pub date_sortie: Option<NaiveDate>,
    pub motif_sortie: Option<MotifSortie>,
}

impl Periode {
    /// L'élève est-il dans l'établissement ce jour-là ? (`[entrée, sortie[`)
    pub fn couvre(&self, jour: NaiveDate) -> bool {
        self.date_entree <= jour && self.date_sortie.is_none_or(|s| s > jour)
    }
}

/// Statut dérivé à un jour. Miroir exact de la fonction SQL `ent_eleve_statut_a` :
/// `scolarise` si une scolarisation couvre le jour ; sinon `a_venir` si l'une commence plus
/// tard (ou si l'élève n'en a aucune : fiche créée avant l'entrée) ; sinon `sorti`.
pub fn statut_derive(periodes: &[Periode], jour: NaiveDate) -> StatutEleve {
    if periodes.iter().any(|p| p.couvre(jour)) {
        StatutEleve::Scolarise
    } else if periodes.is_empty() || periodes.iter().any(|p| p.date_entree > jour) {
        StatutEleve::AVenir
    } else {
        StatutEleve::Sorti
    }
}

/// Deux périodes `[entrée, sortie[` se chevauchent-elles ? (une période vide n'en chevauche aucune)
pub fn se_chevauchent(a: &Periode, b: &Periode) -> bool {
    let fin_a = a.date_sortie;
    let fin_b = b.date_sortie;
    let vide = |p: &Periode| p.date_sortie.is_some_and(|s| s <= p.date_entree);
    if vide(a) || vide(b) {
        return false;
    }
    fin_a.is_none_or(|f| f > b.date_entree) && fin_b.is_none_or(|f| f > a.date_entree)
}

/// Longueur maximale d'un libellé libre (origine, destination).
const LIBELLE_MAX: usize = 200;

fn uai_valide(uai: &str) -> bool {
    !uai.is_empty() && uai.len() <= 20 && uai.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Origine ou destination : UAI facultatif (alphanumérique ≤ 20 caractères : le format national
/// exact et l'annuaire des établissements relèvent du socle 9, R-E23), libellé libre de secours.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Etablissement {
    pub uai: Option<String>,
    pub libelle: Option<String>,
    pub type_: Option<TypeEtablissement>,
}

fn valider_etablissement(e: &Etablissement) -> Result<Etablissement, ErreurScolarite> {
    let uai = e.uai.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if let Some(u) = uai {
        if !uai_valide(u) {
            return Err(ErreurScolarite::Invalide("UAI invalide (lettres et chiffres, 20 au plus)"));
        }
    }
    let libelle = e.libelle.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if libelle.is_some_and(|l| l.chars().count() > LIBELLE_MAX) {
        return Err(ErreurScolarite::Invalide("libellé d'établissement trop long (200 caractères)"));
    }
    Ok(Etablissement {
        uai: uai.map(|u| u.to_ascii_uppercase()),
        libelle: libelle.map(String::from),
        type_: e.type_,
    })
}

/// Demande d'ouverture d'une scolarisation (ELV-08) ou de réintégration (ELV-12).
#[derive(Debug, Clone)]
pub struct DemandeOuverture {
    pub date_entree: NaiveDate,
    pub motif: MotifEntree,
    pub origine: Etablissement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OuverturePlanifiee {
    pub date_entree: NaiveDate,
    pub motif: MotifEntree,
    pub origine: Etablissement,
}

/// ELV-08 / ELV-12 : planifie l'ouverture. Refus : une scolarisation encore ouverte ; une
/// entrée antérieure ou égale à une sortie… au contraire, une entrée AVANT la dernière sortie
/// (chevauchement) ; une entrée plus de 2 ans dans le futur (erreur de frappe probable).
/// `reintegration` : exige un élève déjà sorti, motif `retour`, interdit après un décès.
pub fn planifier_ouverture(
    existantes: &[Periode],
    demande: DemandeOuverture,
    reintegration: bool,
    jour: NaiveDate,
) -> Result<OuverturePlanifiee, ErreurScolarite> {
    if existantes.iter().any(|p| p.date_sortie.is_none()) {
        return Err(ErreurScolarite::Conflit(
            "une scolarisation est déjà ouverte : la clôturer avant d'en ouvrir une autre",
        ));
    }
    if demande.date_entree > jour + Duration::days(730) {
        return Err(ErreurScolarite::Invalide("date d'entrée trop éloignée dans le futur"));
    }
    let nouvelle = Periode {
        date_entree: demande.date_entree,
        date_sortie: None,
        motif_sortie: None,
    };
    if existantes.iter().any(|p| se_chevauchent(p, &nouvelle)) {
        return Err(ErreurScolarite::Conflit(
            "la scolarisation chevauche une scolarisation existante de cet élève",
        ));
    }
    // Un élève décédé ne rouvre jamais, quelle que soit la voie (R-E12).
    if existantes
        .iter()
        .max_by_key(|p| p.date_entree)
        .is_some_and(|p| p.motif_sortie == Some(MotifSortie::Deces))
    {
        return Err(ErreurScolarite::Conflit("un élève décédé ne peut pas être réintégré"));
    }
    if reintegration {
        let derniere = existantes
            .iter()
            .max_by_key(|p| p.date_entree)
            .ok_or(ErreurScolarite::Invalide(
                "aucune scolarisation passée : ouvrir une scolarisation (première inscription)",
            ))?;
        if demande.motif != MotifEntree::Retour {
            return Err(ErreurScolarite::Invalide("une réintégration a pour motif « retour »"));
        }
    } else if demande.motif == MotifEntree::Retour && existantes.is_empty() {
        return Err(ErreurScolarite::Invalide(
            "motif « retour » sans scolarisation passée : utiliser un autre motif",
        ));
    }
    Ok(OuverturePlanifiee {
        date_entree: demande.date_entree,
        motif: demande.motif,
        origine: valider_etablissement(&demande.origine)?,
    })
}

/// Demande de clôture (ELV-11) : sortie, radiation.
#[derive(Debug, Clone)]
pub struct DemandeCloture {
    pub date_sortie: NaiveDate,
    pub motif: MotifSortie,
    /// Précision libre, seulement pour `autre` (obligatoire alors).
    pub motif_libre: Option<String>,
    pub destination: Etablissement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloturePlanifiee {
    pub date_sortie: NaiveDate,
    pub motif: MotifSortie,
    pub motif_libre: Option<String>,
    pub destination: Etablissement,
}

/// ELV-11 : planifie la clôture de la scolarisation `courante` (ouverte).
pub fn planifier_cloture(
    courante: &Periode,
    demande: DemandeCloture,
    jour: NaiveDate,
) -> Result<CloturePlanifiee, ErreurScolarite> {
    if courante.date_sortie.is_some() {
        return Err(ErreurScolarite::Conflit("la scolarisation est déjà clôturée"));
    }
    if demande.date_sortie < courante.date_entree {
        return Err(ErreurScolarite::Invalide("la sortie ne peut pas précéder l'entrée"));
    }
    if demande.date_sortie > jour + Duration::days(366) {
        return Err(ErreurScolarite::Invalide("date de sortie trop éloignée dans le futur"));
    }
    let libre = demande.motif_libre.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if demande.motif == MotifSortie::Autre && libre.is_none() {
        return Err(ErreurScolarite::Invalide("le motif « autre » demande une précision"));
    }
    if libre.is_some_and(|l| l.chars().count() > 500) {
        return Err(ErreurScolarite::Invalide("précision trop longue (500 caractères)"));
    }
    Ok(CloturePlanifiee {
        date_sortie: demande.date_sortie,
        motif: demande.motif,
        motif_libre: libre.map(String::from),
        destination: valider_etablissement(&demande.destination)?,
    })
}

// ─────────────────────────────── Majorité ───────────────────────────────

/// Jour des 18 ans, d'après la date de naissance RETENUE. `None` si la précision n'est pas
/// « jour » ou si la date est contestée (R-E1 : jamais de majorité calculée sur un doute).
/// Un 29 février donne le 28 février des années non bissextiles [H : à valider
/// juridiquement] (comportement de la fonction SQL `ent_eleve_majorite_le`).
pub fn date_majorite(naissance: Option<NaiveDate>, precision: &str, statut: &str) -> Option<NaiveDate> {
    let n = naissance?;
    if precision != "jour" || statut == "contestee" {
        return None;
    }
    NaiveDate::from_ymd_opt(n.year() + 18, n.month(), n.day())
        .or_else(|| NaiveDate::from_ymd_opt(n.year() + 18, n.month(), n.day() - 1))
}

/// Majeur au jour donné (âge) ; `None` si la majorité ne peut pas être calculée.
pub fn est_majeur_le(naissance: Option<NaiveDate>, precision: &str, statut: &str, jour: NaiveDate) -> Option<bool> {
    date_majorite(naissance, precision, statut).map(|m| m <= jour)
}

// ───────────────────── Dates de changement de régime (R-E5) ─────────────────────

/// Réglage de l'établissement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatesChangement {
    Libre,
    /// Le changement prend effet le lendemain de la fin d'une période du référentiel.
    FinDePeriode(Vec<NaiveDate>),
    /// Idem, trimestres seulement.
    FinDeTrimestre(Vec<NaiveDate>),
    /// Dates explicites.
    Liste(Vec<NaiveDate>),
}

impl DatesChangement {
    /// Dates d'effet autorisées (hors `Libre`), triées.
    pub fn autorisees(&self) -> Vec<NaiveDate> {
        let mut d: Vec<NaiveDate> = match self {
            Self::Libre => Vec::new(),
            Self::FinDePeriode(fins) | Self::FinDeTrimestre(fins) => {
                fins.iter().map(|f| *f + Duration::days(1)).collect()
            }
            Self::Liste(l) => l.clone(),
        };
        d.sort();
        d.dedup();
        d
    }

    pub fn est_libre(&self) -> bool {
        matches!(self, Self::Libre)
    }
}

/// La date d'effet est-elle autorisée sans dérogation ?
pub fn date_changement_autorisee(regle: &DatesChangement, date: NaiveDate) -> bool {
    regle.est_libre() || regle.autorisees().contains(&date)
}

/// Prochaine date autorisée à partir de `depuis` (incluse) : proposée par ELV-10.
pub fn prochaine_date_autorisee(regle: &DatesChangement, depuis: NaiveDate) -> Option<NaiveDate> {
    if regle.est_libre() {
        return Some(depuis);
    }
    regle.autorisees().into_iter().find(|d| *d >= depuis)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, j).unwrap()
    }

    fn periode(entree: NaiveDate, sortie: Option<NaiveDate>) -> Periode {
        Periode { date_entree: entree, date_sortie: sortie, motif_sortie: None }
    }

    #[test]
    fn statut_derive_aux_bornes() {
        let p = [periode(d(2025, 9, 1), Some(d(2026, 1, 15)))];
        assert_eq!(statut_derive(&p, d(2025, 8, 31)), StatutEleve::AVenir);
        assert_eq!(statut_derive(&p, d(2025, 9, 1)), StatutEleve::Scolarise);
        assert_eq!(statut_derive(&p, d(2026, 1, 14)), StatutEleve::Scolarise);
        // La date de sortie est le premier jour d'absence.
        assert_eq!(statut_derive(&p, d(2026, 1, 15)), StatutEleve::Sorti);
        assert_eq!(statut_derive(&[], d(2026, 1, 15)), StatutEleve::AVenir);
        // Sortie puis retour annoncé : « à venir » tant que le retour n'a pas eu lieu.
        let deux = [p[0].clone(), periode(d(2026, 9, 1), None)];
        assert_eq!(statut_derive(&deux, d(2026, 3, 1)), StatutEleve::AVenir);
        assert_eq!(statut_derive(&deux, d(2026, 9, 1)), StatutEleve::Scolarise);
        // Période vide (entrée annulée le jour même) : jamais scolarisé.
        let vide = [periode(d(2025, 9, 1), Some(d(2025, 9, 1)))];
        assert_eq!(statut_derive(&vide, d(2025, 9, 1)), StatutEleve::Sorti);
    }

    #[test]
    fn chevauchement_demi_ouvert() {
        let a = periode(d(2024, 9, 1), Some(d(2025, 6, 30)));
        // Entrée le jour de sortie de la précédente : pas de chevauchement.
        assert!(!se_chevauchent(&a, &periode(d(2025, 6, 30), None)));
        assert!(se_chevauchent(&a, &periode(d(2025, 6, 29), None)));
        assert!(se_chevauchent(&periode(d(2024, 1, 1), None), &a));
        assert!(!se_chevauchent(&a, &periode(d(2026, 1, 1), Some(d(2026, 1, 1)))));
    }

    fn ouvrir(date: NaiveDate, motif: MotifEntree) -> DemandeOuverture {
        DemandeOuverture { date_entree: date, motif, origine: Etablissement::default() }
    }

    #[test]
    fn ouverture_refuse_chevauchement_et_double_ouverture() {
        let jour = d(2026, 1, 10);
        let close = [periode(d(2024, 9, 1), Some(d(2025, 6, 30)))];
        assert!(planifier_ouverture(&close, ouvrir(d(2025, 9, 1), MotifEntree::Retour), true, jour).is_ok());
        assert!(matches!(
            planifier_ouverture(&close, ouvrir(d(2025, 3, 1), MotifEntree::Retour), true, jour),
            Err(ErreurScolarite::Conflit(_))
        ));
        let ouverte = [periode(d(2025, 9, 1), None)];
        assert!(matches!(
            planifier_ouverture(&ouverte, ouvrir(d(2026, 2, 1), MotifEntree::Retour), true, jour),
            Err(ErreurScolarite::Conflit(_))
        ));
        assert!(matches!(
            planifier_ouverture(&[], ouvrir(d(2029, 1, 1), MotifEntree::PremiereInscription), false, jour),
            Err(ErreurScolarite::Invalide(_))
        ));
    }

    #[test]
    fn reintegration_exige_un_passe_un_motif_retour_et_pas_de_deces() {
        let jour = d(2026, 1, 10);
        assert!(planifier_ouverture(&[], ouvrir(jour, MotifEntree::Retour), true, jour).is_err());
        let sorti = [Periode {
            date_entree: d(2024, 9, 1),
            date_sortie: Some(d(2025, 6, 30)),
            motif_sortie: Some(MotifSortie::Demenagement),
        }];
        assert!(planifier_ouverture(&sorti, ouvrir(jour, MotifEntree::PassageInterne), true, jour).is_err());
        let decede = [Periode { motif_sortie: Some(MotifSortie::Deces), ..sorti[0].clone() }];
        // Même par une entrée ordinaire (pas seulement la réintégration).
        assert!(planifier_ouverture(&decede, ouvrir(jour, MotifEntree::ChangementEtablissement), false, jour).is_err());
        assert!(matches!(
            planifier_ouverture(&decede, ouvrir(jour, MotifEntree::Retour), true, jour),
            Err(ErreurScolarite::Conflit(_))
        ));
        // Un « retour » sans passé n'est pas une première inscription.
        assert!(planifier_ouverture(&[], ouvrir(jour, MotifEntree::Retour), false, jour).is_err());
    }

    #[test]
    fn uai_et_libelles_controles() {
        let jour = d(2026, 1, 10);
        let mut dem = ouvrir(jour, MotifEntree::ChangementEtablissement);
        dem.origine = Etablissement { uai: Some(" 0751234a ".into()), libelle: Some("Collège X".into()), type_: Some(TypeEtablissement::Public) };
        let ok = planifier_ouverture(&[], dem.clone(), false, jour).unwrap();
        assert_eq!(ok.origine.uai.as_deref(), Some("0751234A"));
        dem.origine.uai = Some("07 51;".into());
        assert!(planifier_ouverture(&[], dem.clone(), false, jour).is_err());
        dem.origine = Etablissement { libelle: Some("x".repeat(201)), ..Default::default() };
        assert!(planifier_ouverture(&[], dem, false, jour).is_err());
    }

    #[test]
    fn cloture_regles() {
        let jour = d(2026, 1, 10);
        let c = periode(d(2025, 9, 1), None);
        let dem = |date, motif, libre: Option<&str>| DemandeCloture {
            date_sortie: date, motif, motif_libre: libre.map(String::from), destination: Etablissement::default(),
        };
        assert!(planifier_cloture(&c, dem(jour, MotifSortie::Demenagement, None), jour).is_ok());
        assert!(planifier_cloture(&c, dem(d(2025, 8, 1), MotifSortie::Demenagement, None), jour).is_err());
        assert!(planifier_cloture(&c, dem(jour, MotifSortie::Autre, None), jour).is_err());
        assert!(planifier_cloture(&c, dem(jour, MotifSortie::Autre, Some("précisé")), jour).is_ok());
        assert!(planifier_cloture(&c, dem(d(2028, 1, 1), MotifSortie::FinDeCycle, None), jour).is_err());
        let close = Periode { date_sortie: Some(jour), ..c };
        assert!(matches!(
            planifier_cloture(&close, dem(jour, MotifSortie::FinDeCycle, None), jour),
            Err(ErreurScolarite::Conflit(_))
        ));
    }

    #[test]
    fn majorite_le_jour_des_18_ans() {
        assert_eq!(date_majorite(Some(d(2008, 3, 14)), "jour", "declaree"), Some(d(2026, 3, 14)));
        assert_eq!(est_majeur_le(Some(d(2008, 3, 14)), "jour", "declaree", d(2026, 3, 13)), Some(false));
        assert_eq!(est_majeur_le(Some(d(2008, 3, 14)), "jour", "declaree", d(2026, 3, 14)), Some(true));
        // 29 février : 28 février des années non bissextiles.
        assert_eq!(date_majorite(Some(d(2008, 2, 29)), "jour", "declaree"), Some(d(2026, 2, 28)));
        // R-E1 : jamais de majorité sur une date contestée ou imprécise.
        assert_eq!(date_majorite(Some(d(2008, 3, 14)), "jour", "contestee"), None);
        assert_eq!(date_majorite(Some(d(2008, 1, 1)), "annee", "presumee"), None);
        assert_eq!(date_majorite(None, "jour", "declaree"), None);
        // Présumée au jour : calculée (avertissement côté affichage).
        assert!(date_majorite(Some(d(2008, 3, 14)), "jour", "presumee").is_some());
    }

    #[test]
    fn dates_de_changement_de_regime() {
        let libre = DatesChangement::Libre;
        assert!(date_changement_autorisee(&libre, d(2026, 2, 3)));
        assert_eq!(prochaine_date_autorisee(&libre, d(2026, 2, 3)), Some(d(2026, 2, 3)));
        let fins = DatesChangement::FinDeTrimestre(vec![d(2025, 12, 19), d(2026, 3, 20)]);
        assert!(date_changement_autorisee(&fins, d(2025, 12, 20)));
        assert!(!date_changement_autorisee(&fins, d(2026, 1, 10)));
        assert_eq!(prochaine_date_autorisee(&fins, d(2026, 1, 10)), Some(d(2026, 3, 21)));
        assert_eq!(prochaine_date_autorisee(&fins, d(2026, 6, 1)), None);
        let liste = DatesChangement::Liste(vec![d(2026, 4, 1), d(2026, 1, 5)]);
        assert_eq!(liste.autorisees(), vec![d(2026, 1, 5), d(2026, 4, 1)]);
    }

    #[test]
    fn codes_aller_retour() {
        for m in MotifSortie::TOUS {
            assert_eq!(MotifSortie::parse(m.code()), Some(*m));
        }
        for m in MotifEntree::TOUS {
            assert_eq!(MotifEntree::parse(m.code()), Some(*m));
        }
        assert_eq!(StatutEleve::parse("scolarise"), Some(StatutEleve::Scolarise));
        assert_eq!(NatureCapacite::parse("tutelle"), Some(NatureCapacite::Tutelle));
        assert!(MotifSortie::parse("inconnu").is_none());
        // Les codes Rust suivent exactement les CHECK SQL (motifs de sortie).
        assert!(MotifSortie::TOUS.iter().any(|m| m.code() == "deces"));
    }
}
