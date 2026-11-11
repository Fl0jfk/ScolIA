//! Stages / entreprises / conventions (socle 39, lot ST0).
//!
//! Ce module ne porte que des **règles pures** (sans base ni réseau) : validation des saisies,
//! clé du SIRET, statuts. Les contrôles qui exigent la base (chevauchement, année scolaire,
//! doublon de SIRET) sont faits dans la transaction par `ent-db` et remontent la même
//! [`StageErreur`], pour que l'interface dise précisément ce qui ne va pas (RS-10).

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Motif précis d'un refus du module des stages (RS-10). Les messages sont destinés à
/// l'utilisateur et ne contiennent aucune donnée personnelle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageErreur {
    RaisonSocialeInvalide,
    AdresseTropLongue,
    SiretInvalide,
    SiretDoublon,
    DatesInvalides,
    DatesHorsAnnee,
    Chevauchement,
    TuteurManquant,
    TuteurTropLong,
    StatutInvalide,
    ConflitDeVersion,
    // ───── Lot ST1 (a) : paramètres, dispositifs, organismes, périodes ─────
    ParametreInvalide,
    AssuranceIncomplete,
    DispositifInconnu,
    DispositifInactif,
    EtatOrganismeInvalide,
    MotifEtatRequis,
    InterditReserveDirection,
    SiteInvalide,
    ContactInvalide,
    SignataireSansCourriel,
    PeriodeInvalide,
    PeriodeHorsAnnee,
    PeriodeMotifRequis,
    PeriodePublicInvalide,
    PeriodeSansPublic,
    EtatPeriodeInvalide,
    PeriodeDansLeTemps,
}

impl StageErreur {
    /// Code stable lisible par une machine (journaux, tests, interface).
    pub const fn code(self) -> &'static str {
        match self {
            Self::RaisonSocialeInvalide => "raison_sociale_invalide",
            Self::AdresseTropLongue => "adresse_trop_longue",
            Self::SiretInvalide => "siret_invalide",
            Self::SiretDoublon => "siret_doublon",
            Self::DatesInvalides => "dates_invalides",
            Self::DatesHorsAnnee => "dates_hors_annee",
            Self::Chevauchement => "chevauchement",
            Self::TuteurManquant => "tuteur_manquant",
            Self::TuteurTropLong => "tuteur_trop_long",
            Self::StatutInvalide => "statut_invalide",
            Self::ConflitDeVersion => "conflit_de_version",
            Self::ParametreInvalide => "parametre_invalide",
            Self::AssuranceIncomplete => "assurance_incomplete",
            Self::DispositifInconnu => "dispositif_inconnu",
            Self::DispositifInactif => "dispositif_inactif",
            Self::EtatOrganismeInvalide => "etat_organisme_invalide",
            Self::MotifEtatRequis => "motif_etat_requis",
            Self::InterditReserveDirection => "interdit_reserve_direction",
            Self::SiteInvalide => "site_invalide",
            Self::ContactInvalide => "contact_invalide",
            Self::SignataireSansCourriel => "signataire_sans_courriel",
            Self::PeriodeInvalide => "periode_invalide",
            Self::PeriodeHorsAnnee => "periode_hors_annee",
            Self::PeriodeMotifRequis => "periode_motif_requis",
            Self::PeriodePublicInvalide => "periode_public_invalide",
            Self::PeriodeSansPublic => "periode_sans_public",
            Self::EtatPeriodeInvalide => "etat_periode_invalide",
            Self::PeriodeDansLeTemps => "periode_dans_le_temps",
        }
    }

    /// Message à afficher (français, sans donnée personnelle).
    pub const fn message(self) -> &'static str {
        match self {
            Self::RaisonSocialeInvalide => {
                "La raison sociale de l'organisme est obligatoire (200 caractères au plus)."
            }
            Self::AdresseTropLongue => "L'adresse de l'organisme est trop longue (400 caractères au plus).",
            Self::SiretInvalide => {
                "Le SIRET est invalide : 14 chiffres dont la clé de contrôle doit être correcte."
            }
            Self::SiretDoublon => "Un organisme porte déjà ce SIRET dans l'établissement.",
            Self::DatesInvalides => "La date de fin du stage précède sa date de début.",
            Self::DatesHorsAnnee => "Le stage doit tenir dans une même année scolaire de l'établissement.",
            Self::Chevauchement => "Cet élève a déjà un stage sur tout ou partie de ces dates.",
            Self::TuteurManquant => "Le nom du tuteur est obligatoire.",
            Self::TuteurTropLong => "Le nom du tuteur est trop long (120 caractères au plus).",
            Self::StatutInvalide => "Ce changement de statut n'est pas permis.",
            Self::ConflitDeVersion => {
                "La fiche a été modifiée par quelqu'un d'autre : rechargez-la puis recommencez."
            }
            Self::ParametreInvalide => "Un paramètre des stages est hors de ses bornes.",
            Self::AssuranceIncomplete => "L'assureur et le numéro de contrat vont ensemble : renseignez les deux ou aucun.",
            Self::DispositifInconnu => "Ce dispositif de stage n'existe pas.",
            Self::DispositifInactif => "Ce dispositif n'est pas activé pour l'établissement.",
            Self::EtatOrganismeInvalide => "Cet état d'organisme n'existe pas.",
            Self::MotifEtatRequis => "Un organisme déconseillé exige un motif codé.",
            Self::InterditReserveDirection => "Seule la direction peut déclarer un organisme interdit.",
            Self::SiteInvalide => "L'adresse du site est incomplète ou trop longue.",
            Self::ContactInvalide => "Le contact est incomplet (nom, prénom) ou une coordonnée est invalide.",
            Self::SignataireSansCourriel => "Un signataire doit avoir une adresse électronique.",
            Self::PeriodeInvalide => "La période est incomplète (libellé) ou sa fin précède son début.",
            Self::PeriodeHorsAnnee => "La période doit tenir dans l'année scolaire choisie.",
            Self::PeriodeMotifRequis => "Une période individuelle exige un motif.",
            Self::PeriodePublicInvalide => "Le public de la période est invalide (une classe, un groupe ou un élève).",
            Self::PeriodeSansPublic => "Une période doit viser au moins une classe, un groupe ou un élève avant d'être publiée.",
            Self::EtatPeriodeInvalide => "Ce changement d'état de la période n'est pas permis.",
            Self::PeriodeDansLeTemps => "Une période close ne se modifie plus.",
        }
    }

    /// Le refus est un conflit d'état (409) plutôt qu'une saisie invalide (422).
    pub const fn est_conflit(self) -> bool {
        matches!(
            self,
            Self::SiretDoublon
                | Self::Chevauchement
                | Self::ConflitDeVersion
                | Self::EtatPeriodeInvalide
                | Self::PeriodeDansLeTemps
                | Self::DispositifInactif
        )
    }
}

impl std::fmt::Display for StageErreur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for StageErreur {}

/// Contrôle de clé d'un SIRET : 14 chiffres, algorithme de Luhn, avec l'exception connue des
/// établissements de La Poste (SIREN 356 000 000 : somme des chiffres multiple de 5).
/// Doit rester identique à la fonction SQL `ent_siret_cle_valide` (migration 2780).
pub fn siret_cle_valide(siret: &str) -> bool {
    if siret.len() != 14 || !siret.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let chiffres: Vec<u32> = siret.bytes().map(|b| u32::from(b - b'0')).collect();
    if siret.starts_with("356000000") {
        return chiffres.iter().sum::<u32>() % 5 == 0;
    }
    let somme: u32 = chiffres
        .iter()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 0 {
                let x = d * 2;
                if x > 9 {
                    x - 9
                } else {
                    x
                }
            } else {
                d
            }
        })
        .sum();
    somme % 10 == 0
}

/// Statuts d'un stage déclaré (RS-01). `declaree` : stage connu de l'établissement **sans
/// convention signée dans ScolIA** ; ce statut ne vaut jamais « en stage ».
pub mod statut {
    pub const DECLAREE: &str = "declaree";
    pub const CLOTUREE: &str = "cloturee";
    pub const ANNULEE: &str = "annulee";
    pub const TOUS: [&str; 3] = [DECLAREE, CLOTUREE, ANNULEE];

    /// Texte affiché avec le statut `declaree` : le mot ne doit pas laisser croire à une convention.
    pub const MENTION_DECLAREE: &str = "convention non signée dans ScolIA";

    /// Transitions permises depuis un statut (jamais de retour depuis annulée ou clôturée).
    pub fn transition_permise(de: &str, vers: &str) -> bool {
        matches!((de, vers), (DECLAREE, CLOTUREE) | (DECLAREE, ANNULEE))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entreprise {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub raison_sociale: String,
    pub siret: Option<String>,
    pub adresse: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub modifie_le: DateTime<Utc>,
    pub version: i32,
}

#[derive(Debug, Clone)]
pub struct CreerEntreprise {
    pub etablissement_id: Uuid,
    pub raison_sociale: String,
    pub siret: Option<String>,
    pub adresse: String,
    pub actor_user_id: Uuid,
}

/// Valide et normalise une entreprise (SIRET réduit à ses chiffres, clé contrôlée).
pub fn plan_creer_entreprise(cmd: CreerEntreprise) -> Result<Entreprise, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let raison = cmd.raison_sociale.trim().to_string();
    if raison.is_empty() || raison.chars().count() > 200 {
        return Err(DomainError::Stage(StageErreur::RaisonSocialeInvalide));
    }
    let adresse = cmd.adresse.trim().to_string();
    if adresse.chars().count() > 400 {
        return Err(DomainError::Stage(StageErreur::AdresseTropLongue));
    }
    let siret = cmd
        .siret
        .map(|s| s.chars().filter(|c| c.is_ascii_digit()).collect::<String>())
        .filter(|s| !s.is_empty());
    if siret.as_deref().is_some_and(|s| !siret_cle_valide(s)) {
        return Err(DomainError::Stage(StageErreur::SiretInvalide));
    }
    let now = Utc::now();
    Ok(Entreprise {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        raison_sociale: raison,
        siret,
        adresse,
        created_by_user_id: cmd.actor_user_id,
        created_at: now,
        modifie_le: now,
        version: 1,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConventionStage {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub entreprise_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub tuteur_nom: String,
    pub statut: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub modifie_le: DateTime<Utc>,
    pub version: i32,
}

impl ConventionStage {
    /// Une ligne `declaree` n'est **pas** une convention signée : jamais une situation « en stage ».
    pub fn convention_signee(&self) -> bool {
        false
    }

    /// Mention à afficher à côté du statut (RS-01).
    pub fn mention(&self) -> Option<&'static str> {
        (self.statut == statut::DECLAREE).then_some(statut::MENTION_DECLAREE)
    }
}

#[derive(Debug, Clone)]
pub struct CreerConventionStage {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub entreprise_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub tuteur_nom: String,
    pub actor_user_id: Uuid,
}

pub fn plan_creer_convention_stage(
    cmd: CreerConventionStage,
) -> Result<ConventionStage, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() || cmd.entreprise_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.fin < cmd.debut {
        return Err(DomainError::Stage(StageErreur::DatesInvalides));
    }
    let tuteur = cmd.tuteur_nom.trim().to_string();
    if tuteur.is_empty() {
        return Err(DomainError::Stage(StageErreur::TuteurManquant));
    }
    if tuteur.chars().count() > 120 {
        return Err(DomainError::Stage(StageErreur::TuteurTropLong));
    }
    let now = Utc::now();
    Ok(ConventionStage {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        entreprise_id: cmd.entreprise_id,
        debut: cmd.debut,
        fin: cmd.fin,
        tuteur_nom: tuteur,
        statut: statut::DECLAREE.into(),
        created_by_user_id: cmd.actor_user_id,
        created_at: now,
        modifie_le: now,
        version: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entreprise(siret: Option<&str>) -> CreerEntreprise {
        CreerEntreprise {
            etablissement_id: Uuid::new_v4(),
            raison_sociale: "ACME".into(),
            siret: siret.map(str::to_string),
            adresse: "1 rue".into(),
            actor_user_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn siret_longueur_refusee() {
        let err = plan_creer_entreprise(entreprise(Some("123"))).unwrap_err();
        assert_eq!(err, DomainError::Stage(StageErreur::SiretInvalide));
    }

    #[test]
    fn siret_cle_controlee() {
        // RS-08 : 14 chiffres mais clé fausse → refusé ; clé juste → accepté.
        assert!(siret_cle_valide("73282932000074"));
        assert!(!siret_cle_valide("73282932000075"));
        assert!(!siret_cle_valide("12345678901234"));
        assert!(!siret_cle_valide("7328293200007A"));
        let e = plan_creer_entreprise(entreprise(Some("732 829 320 00074"))).unwrap();
        assert_eq!(e.siret.as_deref(), Some("73282932000074"));
        assert_eq!(
            plan_creer_entreprise(entreprise(Some("12345678901234"))).unwrap_err(),
            DomainError::Stage(StageErreur::SiretInvalide)
        );
    }

    #[test]
    fn siret_la_poste_exception() {
        // 356 000 000 : somme des chiffres multiple de 5.
        assert!(siret_cle_valide("35600000000079"));
        assert!(!siret_cle_valide("35600000000078"));
    }

    #[test]
    fn siret_absent_accepte() {
        assert!(plan_creer_entreprise(entreprise(None)).is_ok());
        assert!(plan_creer_entreprise(entreprise(Some("  "))).is_ok());
    }

    #[test]
    fn motifs_distincts_et_stables() {
        let mut e = entreprise(None);
        e.raison_sociale = " ".into();
        assert_eq!(
            plan_creer_entreprise(e).unwrap_err(),
            DomainError::Stage(StageErreur::RaisonSocialeInvalide)
        );
        let mut e = entreprise(None);
        e.adresse = "x".repeat(401);
        assert_eq!(
            plan_creer_entreprise(e).unwrap_err(),
            DomainError::Stage(StageErreur::AdresseTropLongue)
        );
        // Chaque motif a son code et son message (RS-10).
        let tous = [
            StageErreur::RaisonSocialeInvalide,
            StageErreur::AdresseTropLongue,
            StageErreur::SiretInvalide,
            StageErreur::SiretDoublon,
            StageErreur::DatesInvalides,
            StageErreur::DatesHorsAnnee,
            StageErreur::Chevauchement,
            StageErreur::TuteurManquant,
            StageErreur::TuteurTropLong,
            StageErreur::StatutInvalide,
            StageErreur::ConflitDeVersion,
        ];
        let codes: std::collections::HashSet<_> = tous.iter().map(|e| e.code()).collect();
        assert_eq!(codes.len(), tous.len());
        assert!(tous.iter().all(|e| !e.message().is_empty()));
    }

    fn convention(debut: (i32, u32, u32), fin: (i32, u32, u32), tuteur: &str) -> CreerConventionStage {
        CreerConventionStage {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            entreprise_id: Uuid::new_v4(),
            debut: NaiveDate::from_ymd_opt(debut.0, debut.1, debut.2).unwrap(),
            fin: NaiveDate::from_ymd_opt(fin.0, fin.1, fin.2).unwrap(),
            tuteur_nom: tuteur.into(),
            actor_user_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn convention_dates() {
        let err = plan_creer_convention_stage(convention((2026, 11, 1), (2026, 10, 1), "Dupont"))
            .unwrap_err();
        assert_eq!(err, DomainError::Stage(StageErreur::DatesInvalides));
    }

    #[test]
    fn convention_tuteur() {
        assert_eq!(
            plan_creer_convention_stage(convention((2026, 11, 1), (2026, 11, 5), "  ")).unwrap_err(),
            DomainError::Stage(StageErreur::TuteurManquant)
        );
        assert_eq!(
            plan_creer_convention_stage(convention((2026, 11, 1), (2026, 11, 5), &"x".repeat(121)))
                .unwrap_err(),
            DomainError::Stage(StageErreur::TuteurTropLong)
        );
    }

    /// RS-01 : le statut créé est `declaree`, jamais `active`, et ne vaut pas convention signée.
    #[test]
    fn convention_creee_declaree_pas_signee() {
        let c = plan_creer_convention_stage(convention((2026, 11, 2), (2026, 11, 6), "Dupont")).unwrap();
        assert_eq!(c.statut, "declaree");
        assert!(!c.convention_signee());
        assert_eq!(c.mention(), Some("convention non signée dans ScolIA"));
        assert_eq!(c.version, 1);
    }

    #[test]
    fn transitions_de_statut() {
        assert!(statut::transition_permise("declaree", "cloturee"));
        assert!(statut::transition_permise("declaree", "annulee"));
        assert!(!statut::transition_permise("annulee", "declaree"));
        assert!(!statut::transition_permise("cloturee", "declaree"));
        assert!(!statut::transition_permise("declaree", "active"));
    }
}
