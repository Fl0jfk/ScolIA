//! Matrice « qui peut écrire à qui » (socle 16, §8, `docs/socles/16_MESSAGERIE.md`) — logique pure,
//! sans base : testable en CI sans la feature `labo`.
//!
//! Principe (§8.1) : une personne **peut ouvrir** un fil vers une cible si une **raison** existe à
//! la date, si **aucune interdiction** ne s'y oppose et si la **règle de l'établissement** (sept
//! réglages bornés, §8.3) ne la ferme pas. **Répondre** dans un fil où l'on participe est permis,
//! sauf fil clos, gelé, restriction, interdiction ou sortie du fil. Le serveur recalcule la
//! matrice à chaque envoi : l'écran n'est qu'une aide.
//!
//! La matrice n'est **jamais saisie** (D-023) : elle se calcule à partir des faits ([`Faits`],
//! lus en base par `msg_m2`) et d'un tableau fixe ([`cellule`], le §8.2). L'établissement ne
//! règle que [`ReglagesMatrice`].
//!
//! Les refus ne disent **jamais pourquoi** quand le motif est sensible : une interdiction de
//! communication, un « ne pas solliciter » ou une suspension donnent le même refus neutre
//! [`Refus::DestinataireExclu`] (« destinataire exclu (décision) », CM-05).

use serde::Serialize;

/// Les sept réglages bornés de l'établissement (§8.3) ; valeurs par défaut = §8.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ReglagesMatrice {
    /// Familles → professeurs de **leur** enfant (défaut : oui).
    pub familles_vers_professeurs: bool,
    /// Familles → direction (défaut : oui).
    pub familles_vers_direction: bool,
    /// Élèves → **leurs** professeurs (défaut : oui).
    pub eleves_vers_professeurs: bool,
    /// Élèves → élèves : `Non`, `Delegues` ou `Classe` (défaut : non, Q-M6).
    pub eleves_vers_eleves: EleveVersEleve,
    /// AESH → familles : `Non`, `Repondre` ou `Ouvrir` (défaut : répondre).
    pub aesh_vers_familles: AeshVersFamille,
    /// Secrétariat → élèves **majeurs** (défaut : oui).
    pub secretariat_vers_majeurs: bool,
    /// Pièces jointes des familles et des élèves (défaut : oui).
    pub pieces_familles: bool,
}

impl Default for ReglagesMatrice {
    fn default() -> Self {
        Self {
            familles_vers_professeurs: true,
            familles_vers_direction: true,
            eleves_vers_professeurs: true,
            eleves_vers_eleves: EleveVersEleve::Non,
            aesh_vers_familles: AeshVersFamille::Repondre,
            secretariat_vers_majeurs: true,
            pieces_familles: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EleveVersEleve {
    Non,
    Delegues,
    Classe,
}

impl EleveVersEleve {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "non" => Some(Self::Non),
            "delegues" => Some(Self::Delegues),
            "classe" => Some(Self::Classe),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Non => "non",
            Self::Delegues => "delegues",
            Self::Classe => "classe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AeshVersFamille {
    Non,
    Repondre,
    Ouvrir,
}

impl AeshVersFamille {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "non" => Some(Self::Non),
            "repondre" => Some(Self::Repondre),
            "ouvrir" => Some(Self::Ouvrir),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Non => "non",
            Self::Repondre => "repondre",
            Self::Ouvrir => "ouvrir",
        }
    }
}

/// Qui écrit (lignes du §8.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Acteur {
    Famille,
    EleveMineur,
    EleveMajeur,
    Professeur,
    ProfPrincipal,
    CpeVieScolaire,
    Secretariat,
    Direction,
    Sante,
    Aesh,
    Accueil,
}

/// À qui (colonnes du §8.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Cible {
    Famille,
    Eleve,
    Professeur,
    ProfPrincipal,
    BoiteVieScolaire,
    BoiteSecretariat,
    Direction,
    Sante,
    Aesh,
    Equipe,
    /// Une autre famille : fermé, non paramétrable (jamais famille ↔ famille).
    AutreFamille,
    /// Un autre élève.
    AutreEleve,
}

/// Une case du tableau §8.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cellule {
    /// « ✓ » : ouvert par défaut.
    Oui,
    /// « — » : fermé, non paramétrable.
    Non,
    /// « P » : décidé par un des sept réglages.
    Reglage(Reglage),
}

/// Un des sept réglages (nom stable des cases « P »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Reglage {
    FamillesVersProfesseurs,
    FamillesVersDirection,
    ElevesVersProfesseurs,
    ElevesVersEleves,
    AeshVersFamilles,
    SecretariatVersMajeurs,
}

/// Le tableau §8.2 : une case par couple (acteur, cible).
pub fn cellule(a: Acteur, c: Cible) -> Cellule {
    use Acteur as A;
    use Cellule::{Non, Oui, Reglage as P};
    use Cible as C;
    // Bornes : jamais famille ↔ famille, ni un élève vers un autre élève hors réglage.
    if c == C::AutreFamille {
        return Non;
    }
    if c == C::AutreEleve {
        return match a {
            A::EleveMineur | A::EleveMajeur => P(Reglage::ElevesVersEleves),
            _ => Non,
        };
    }
    match (a, c) {
        // Famille
        (A::Famille, C::Famille | C::Eleve | C::Equipe | C::Aesh) => Non,
        (A::Famille, C::Professeur) => P(Reglage::FamillesVersProfesseurs),
        (A::Famille, C::Direction) => P(Reglage::FamillesVersDirection),
        (A::Famille, _) => Oui,
        // Élève mineur
        (A::EleveMineur, C::Famille | C::Direction | C::Equipe) => Non,
        (A::EleveMineur, C::Eleve) => P(Reglage::ElevesVersEleves),
        (A::EleveMineur, C::Professeur) => P(Reglage::ElevesVersProfesseurs),
        (A::EleveMineur, _) => Oui,
        // Élève majeur
        (A::EleveMajeur, C::Famille | C::Equipe) => Non,
        (A::EleveMajeur, C::Eleve) => P(Reglage::ElevesVersEleves),
        (A::EleveMajeur, C::Professeur) => P(Reglage::ElevesVersProfesseurs),
        (A::EleveMajeur, _) => Oui,
        // Professeur, direction, CPE : tout, sauf ce que la raison réserve (voir `decider`).
        (A::Professeur | A::CpeVieScolaire | A::Direction, _) => Oui,
        // Professeur principal : pas vers un autre PP.
        (A::ProfPrincipal, C::ProfPrincipal) => Non,
        (A::ProfPrincipal, _) => Oui,
        // Secrétariat, intendance
        (A::Secretariat, C::Eleve) => P(Reglage::SecretariatVersMajeurs),
        (A::Secretariat, C::Sante | C::Aesh | C::Equipe) => Non,
        (A::Secretariat, _) => Oui,
        // Santé : tout sauf l'équipe
        (A::Sante, C::Equipe) => Non,
        (A::Sante, _) => Oui,
        // AESH
        (A::Aesh, C::Famille) => P(Reglage::AeshVersFamilles),
        (A::Aesh, C::BoiteSecretariat | C::Direction | C::Sante) => Non,
        (A::Aesh, _) => Oui,
        // Accueil : message de liaison vers le personnel seulement.
        (A::Accueil, C::Famille | C::Eleve | C::Aesh | C::Equipe) => Non,
        (A::Accueil, _) => Oui,
    }
}

/// Faits de la date, lus en base (jamais fournis par le client).
#[derive(Debug, Clone, Copy, Default)]
pub struct Faits {
    /// `true` : ouvrir un fil ; `false` : répondre dans un fil où l'on participe.
    pub ouverture: bool,
    /// Interdiction de communication, lien suspendu ou clos, non confirmé : même refus neutre.
    pub destinataire_exclu: bool,
    /// « Ne pas solliciter » (R-M7) : aucune **ouverture** vers la personne ; réponse permise.
    pub ne_pas_solliciter: bool,
    /// La raison propre à l'acteur existe à la date : lien d'enseignement (D-022, cours ou groupe),
    /// classe du PP, mission, ou, pour une famille, lien familial avec cet élève.
    pub raison: bool,
    /// Réponse dans un fil existant : jours écoulés depuis la fin de la raison (CM-35 : 15 jours).
    pub jours_depuis_fin_raison: Option<i64>,
    /// L'élève visé est majeur à la date.
    pub eleve_majeur: bool,
    /// L'élève visé relève d'un autre établissement.
    pub autre_etablissement: bool,
}

/// Délai de grâce d'un professeur dont le cours a pris fin (CM-35, paramètre borné S-140).
pub const DELAI_REPONSE_APRES_FIN_JOURS: i64 = 15;

/// Pourquoi on peut écrire (affiché : « vous pouvez écrire à Mme B. : … »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Raison {
    ProfesseurDuCours,
    ProfPrincipalDeLaClasse,
    LienFamilial,
    EleveDeLaClasse,
    ServiceDeLEtablissement,
    ReponseDansUnFil,
    ReponseApresFinDeCours,
    MessageDeLiaison,
}

impl Raison {
    pub fn code(self) -> &'static str {
        match self {
            Self::ProfesseurDuCours => "professeur_cours",
            Self::ProfPrincipalDeLaClasse => "pp",
            Self::LienFamilial => "lien_familial",
            Self::EleveDeLaClasse => "eleve_de_la_classe",
            Self::ServiceDeLEtablissement => "service",
            Self::ReponseDansUnFil => "reponse",
            Self::ReponseApresFinDeCours => "reponse_apres_fin_de_cours",
            Self::MessageDeLiaison => "liaison",
        }
    }

    /// Libellé court, sans nom ni coordonnée.
    pub fn libelle(self) -> &'static str {
        match self {
            Self::ProfesseurDuCours => "professeur de cet élève",
            Self::ProfPrincipalDeLaClasse => "professeur principal de la classe",
            Self::LienFamilial => "lien familial avec cet élève",
            Self::EleveDeLaClasse => "élève de la classe",
            Self::ServiceDeLEtablissement => "service de l'établissement",
            Self::ReponseDansUnFil => "réponse dans une conversation où vous participez",
            Self::ReponseApresFinDeCours => "réponse dans les 15 jours qui suivent la fin du cours",
            Self::MessageDeLiaison => "message de liaison entre personnels",
        }
    }
}

/// Pourquoi on ne peut pas écrire. **Jamais le motif d'une décision** : une interdiction, un
/// « ne pas solliciter » et une suspension se confondent dans [`Refus::DestinataireExclu`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Refus {
    /// La case du tableau est fermée (« — »).
    Matrice,
    /// L'établissement a fermé ce réglage.
    Reglage(Reglage),
    /// « Destinataire exclu (décision) » : message neutre, sans motif (CM-05, R-M6, R-M7, R-M8).
    DestinataireExclu,
    /// Hors des cours, de la classe ou des élèves à la date (D-022, R-M10).
    HorsPerimetre,
    /// Jamais famille ↔ famille.
    FamilleVersFamille,
    /// Un élève mineur n'écrit pas à un élève d'un autre établissement.
    AutreEtablissement,
    /// Le secrétariat n'écrit qu'aux élèves **majeurs**.
    EleveMineurSeulementParLesResponsables,
}

impl Refus {
    /// Message neutre affichable (aucune cause sensible).
    pub fn message(self) -> &'static str {
        match self {
            Self::Matrice => "Vous ne pouvez pas écrire à cette personne.",
            Self::Reglage(_) => "Cet échange est fermé par le réglage de l'établissement.",
            Self::DestinataireExclu => "Destinataire exclu (décision).",
            Self::HorsPerimetre => {
                "Cette personne n'est pas dans vos classes ou vos élèves à cette date."
            }
            Self::FamilleVersFamille => "Les familles n'écrivent pas aux autres familles.",
            Self::AutreEtablissement => "Cette personne relève d'un autre établissement.",
            Self::EleveMineurSeulementParLesResponsables => {
                "Un élève mineur est joint par ses responsables."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Decision {
    Permis(Raison),
    Refuse(Refus),
}

impl Decision {
    pub fn est_permis(&self) -> bool {
        matches!(self, Self::Permis(_))
    }
}

/// Décide si `acteur` peut écrire à `cible` (ouvrir ou répondre, `faits.ouverture`).
///
/// Ordre des contrôles : les **bornes non paramétrables** d'abord (famille ↔ famille, autre
/// établissement, décision d'exclusion), puis la case du tableau, puis le réglage, puis la raison.
pub fn decider(
    acteur: Acteur,
    cible: Cible,
    reglages: &ReglagesMatrice,
    faits: &Faits,
) -> Decision {
    use Decision::{Permis, Refuse};
    // Bornes non paramétrables (§8.2).
    if acteur == Acteur::Famille && cible == Cible::AutreFamille {
        return Refuse(Refus::FamilleVersFamille);
    }
    if cible == Cible::AutreEleve
        && matches!(acteur, Acteur::EleveMineur | Acteur::EleveMajeur)
        && faits.autre_etablissement
    {
        return Refuse(Refus::AutreEtablissement);
    }
    // Interdiction, suspension, lien clos ou non confirmé : jamais d'écriture, même en réponse.
    if faits.destinataire_exclu {
        return Refuse(Refus::DestinataireExclu);
    }
    // « Ne pas solliciter » : aucune ouverture vers la personne ; on lui répond (R-M7, CM-11).
    if faits.ne_pas_solliciter
        && faits.ouverture
        && matches!(cible, Cible::Famille | Cible::Eleve)
        && !matches!(
            acteur,
            Acteur::Famille | Acteur::EleveMineur | Acteur::EleveMajeur
        )
    {
        return Refuse(Refus::DestinataireExclu);
    }
    // La case du tableau.
    match cellule(acteur, cible) {
        Cellule::Non => return Refuse(Refus::Matrice),
        Cellule::Oui => {}
        Cellule::Reglage(r) => {
            let ouvert = match r {
                Reglage::FamillesVersProfesseurs => reglages.familles_vers_professeurs,
                Reglage::FamillesVersDirection => reglages.familles_vers_direction,
                Reglage::ElevesVersProfesseurs => reglages.eleves_vers_professeurs,
                Reglage::ElevesVersEleves => reglages.eleves_vers_eleves != EleveVersEleve::Non,
                Reglage::SecretariatVersMajeurs => reglages.secretariat_vers_majeurs,
                // Répondre est permis dès que le réglage n'est pas « non » ; ouvrir demande « ouvrir ».
                Reglage::AeshVersFamilles => match reglages.aesh_vers_familles {
                    AeshVersFamille::Non => false,
                    AeshVersFamille::Repondre => !faits.ouverture,
                    AeshVersFamille::Ouvrir => true,
                },
            };
            if !ouvert {
                return Refuse(Refus::Reglage(r));
            }
        }
    }
    // Le secrétariat n'écrit qu'à un élève **majeur** (le mineur est joint par ses responsables).
    if acteur == Acteur::Secretariat && cible == Cible::Eleve && !faits.eleve_majeur {
        return Refuse(Refus::EleveMineurSeulementParLesResponsables);
    }
    // La raison : lien d'enseignement, classe, mission, ou lien familial.
    let raison_requise = match (acteur, cible) {
        (Acteur::Professeur, Cible::Famille | Cible::Eleve | Cible::Aesh) => true,
        (Acteur::ProfPrincipal, Cible::Famille | Cible::Eleve) => true,
        (Acteur::Aesh, Cible::Famille | Cible::Eleve) => true,
        (Acteur::Sante, Cible::Famille | Cible::Eleve) => true,
        (Acteur::Famille, Cible::Professeur) => true,
        (Acteur::EleveMineur | Acteur::EleveMajeur, Cible::Professeur) => true,
        _ => false,
    };
    if raison_requise && !faits.raison {
        // CM-35 : un ancien professeur ne **rouvre** pas, mais **répond** pendant 15 jours.
        let dans_le_delai = faits
            .jours_depuis_fin_raison
            .is_some_and(|j| (0..=DELAI_REPONSE_APRES_FIN_JOURS).contains(&j));
        if !faits.ouverture && dans_le_delai {
            return Permis(Raison::ReponseApresFinDeCours);
        }
        return Refuse(Refus::HorsPerimetre);
    }
    Permis(raison_affichee(acteur, cible, faits))
}

fn raison_affichee(acteur: Acteur, cible: Cible, faits: &Faits) -> Raison {
    if !faits.ouverture {
        return Raison::ReponseDansUnFil;
    }
    match (acteur, cible) {
        (Acteur::Professeur, _) => Raison::ProfesseurDuCours,
        (Acteur::ProfPrincipal, _) => Raison::ProfPrincipalDeLaClasse,
        (Acteur::Famille, _) => Raison::LienFamilial,
        (Acteur::EleveMineur | Acteur::EleveMajeur, Cible::Professeur) => Raison::ProfesseurDuCours,
        (Acteur::Accueil, _) => Raison::MessageDeLiaison,
        _ => Raison::ServiceDeLEtablissement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ouvrir() -> Faits {
        Faits {
            ouverture: true,
            raison: true,
            ..Faits::default()
        }
    }

    fn repondre() -> Faits {
        Faits {
            ouverture: false,
            raison: true,
            ..Faits::default()
        }
    }

    fn d(a: Acteur, c: Cible, r: &ReglagesMatrice, f: &Faits) -> Decision {
        decider(a, c, r, f)
    }

    // ─── CM-01 à CM-18 (familles) : ce que la matrice tranche ────────────────────────────────

    /// CM-05 : interdiction de communication → même la direction ne peut pas écrire à la personne,
    /// et le refus ne dit pas pourquoi.
    #[test]
    fn cm_05_interdiction_refus_neutre_pour_tout_le_monde() {
        let r = ReglagesMatrice::default();
        let f = Faits {
            destinataire_exclu: true,
            ..ouvrir()
        };
        for a in [
            Acteur::Direction,
            Acteur::CpeVieScolaire,
            Acteur::Professeur,
            Acteur::Secretariat,
        ] {
            assert_eq!(
                d(a, Cible::Famille, &r, &f),
                Decision::Refuse(Refus::DestinataireExclu),
                "{a:?}"
            );
        }
        // Même en réponse dans un fil existant (R-M6 : relue à chaque envoi).
        let f = Faits {
            destinataire_exclu: true,
            ..repondre()
        };
        assert_eq!(
            d(Acteur::Direction, Cible::Famille, &r, &f),
            Decision::Refuse(Refus::DestinataireExclu)
        );
        assert_eq!(
            Refus::DestinataireExclu.message(),
            "Destinataire exclu (décision)."
        );
    }

    /// CM-11 : « ne pas solliciter » — aucune ouverture, mais on répond dans son fil.
    #[test]
    fn cm_11_ne_pas_solliciter_pas_d_ouverture_mais_reponse() {
        let r = ReglagesMatrice::default();
        let ouv = Faits {
            ne_pas_solliciter: true,
            ..ouvrir()
        };
        let rep = Faits {
            ne_pas_solliciter: true,
            ..repondre()
        };
        assert_eq!(
            d(Acteur::Direction, Cible::Famille, &r, &ouv),
            Decision::Refuse(Refus::DestinataireExclu)
        );
        assert_eq!(
            d(Acteur::Professeur, Cible::Famille, &r, &ouv),
            Decision::Refuse(Refus::DestinataireExclu)
        );
        assert!(
            d(Acteur::Direction, Cible::Famille, &r, &rep).est_permis(),
            "témoin : la réponse reste permise"
        );
        // La personne « à ne pas solliciter » écrit elle-même à l'établissement.
        let elle = Faits {
            ne_pas_solliciter: true,
            ..ouvrir()
        };
        assert!(d(Acteur::Famille, Cible::BoiteVieScolaire, &r, &elle).est_permis());
    }

    /// Jamais famille ↔ famille : borne non paramétrable (§8.2).
    #[test]
    fn jamais_famille_vers_famille() {
        let r = ReglagesMatrice::default();
        assert_eq!(
            d(Acteur::Famille, Cible::AutreFamille, &r, &ouvrir()),
            Decision::Refuse(Refus::FamilleVersFamille)
        );
        for a in [
            Acteur::Professeur,
            Acteur::Direction,
            Acteur::CpeVieScolaire,
        ] {
            assert_eq!(
                d(a, Cible::AutreFamille, &r, &ouvrir()),
                Decision::Refuse(Refus::Matrice),
                "{a:?}"
            );
        }
        // Pas d'écriture parent → élève (autre ou le sien) : « — ».
        assert_eq!(
            d(Acteur::Famille, Cible::Eleve, &r, &ouvrir()),
            Decision::Refuse(Refus::Matrice)
        );
    }

    /// §8.2, ligne « Famille » : vers professeurs et direction, réglages de l'établissement.
    #[test]
    fn famille_vers_professeurs_et_direction_selon_les_reglages() {
        let defaut = ReglagesMatrice::default();
        assert!(
            d(Acteur::Famille, Cible::Professeur, &defaut, &ouvrir()).est_permis(),
            "témoin"
        );
        assert!(
            d(Acteur::Famille, Cible::Direction, &defaut, &ouvrir()).est_permis(),
            "témoin"
        );
        let ferme = ReglagesMatrice {
            familles_vers_professeurs: false,
            familles_vers_direction: false,
            ..defaut
        };
        assert_eq!(
            d(Acteur::Famille, Cible::Professeur, &ferme, &ouvrir()),
            Decision::Refuse(Refus::Reglage(Reglage::FamillesVersProfesseurs))
        );
        assert_eq!(
            d(Acteur::Famille, Cible::Direction, &ferme, &ouvrir()),
            Decision::Refuse(Refus::Reglage(Reglage::FamillesVersDirection))
        );
        // La vie scolaire, le secrétariat et le professeur principal restent ouverts (✓).
        for c in [
            Cible::BoiteVieScolaire,
            Cible::BoiteSecretariat,
            Cible::ProfPrincipal,
            Cible::Sante,
        ] {
            assert!(
                d(Acteur::Famille, c, &ferme, &ouvrir()).est_permis(),
                "{c:?}"
            );
        }
        // Une famille n'écrit qu'aux professeurs de **son** enfant (pas de raison → hors périmètre).
        let sans = Faits {
            raison: false,
            ..ouvrir()
        };
        assert_eq!(
            d(Acteur::Famille, Cible::Professeur, &defaut, &sans),
            Decision::Refuse(Refus::HorsPerimetre)
        );
    }

    /// CM-35 : un ancien professeur ne rouvre plus mais répond pendant 15 jours.
    #[test]
    fn cm_35_ancien_professeur_repond_quinze_jours() {
        let r = ReglagesMatrice::default();
        let fini = |j: i64, ouverture: bool| Faits {
            ouverture,
            raison: false,
            jours_depuis_fin_raison: Some(j),
            ..Faits::default()
        };
        // Ouvrir : refusé dès que le cours a pris fin.
        assert_eq!(
            d(Acteur::Professeur, Cible::Famille, &r, &fini(1, true)),
            Decision::Refuse(Refus::HorsPerimetre)
        );
        // Répondre : permis jusqu'à 15 jours, refusé au 16e.
        assert_eq!(
            d(Acteur::Professeur, Cible::Famille, &r, &fini(15, false)),
            Decision::Permis(Raison::ReponseApresFinDeCours)
        );
        assert_eq!(
            d(Acteur::Professeur, Cible::Famille, &r, &fini(16, false)),
            Decision::Refuse(Refus::HorsPerimetre)
        );
        // Témoin : un professeur dont le cours est en cours ouvre.
        assert_eq!(
            d(Acteur::Professeur, Cible::Famille, &r, &ouvrir()),
            Decision::Permis(Raison::ProfesseurDuCours)
        );
    }

    /// CM-38 : professeur de LV2 d'une classe mixte — il n'écrit qu'aux familles des élèves de son
    /// groupe (la raison est calculée à la date par D-022) ; hors groupe, hors périmètre.
    #[test]
    fn cm_38_professeur_de_lv2_seulement_les_eleves_de_son_groupe() {
        let r = ReglagesMatrice::default();
        let dans_mon_groupe = Faits {
            raison: true,
            ..ouvrir()
        };
        let hors_de_mon_groupe = Faits {
            raison: false,
            ..ouvrir()
        };
        assert!(
            d(Acteur::Professeur, Cible::Famille, &r, &dans_mon_groupe).est_permis(),
            "témoin"
        );
        assert_eq!(
            d(Acteur::Professeur, Cible::Famille, &r, &hors_de_mon_groupe),
            Decision::Refuse(Refus::HorsPerimetre)
        );
        assert_eq!(
            d(Acteur::Professeur, Cible::Eleve, &r, &hors_de_mon_groupe),
            Decision::Refuse(Refus::HorsPerimetre)
        );
    }

    /// CM-10 : un parent sans autorité parentale écrit à la direction et au secrétariat (✓).
    #[test]
    fn cm_10_un_parent_ecrit_a_la_direction_et_au_secretariat() {
        let r = ReglagesMatrice::default();
        for c in [
            Cible::Direction,
            Cible::BoiteSecretariat,
            Cible::BoiteVieScolaire,
        ] {
            assert!(d(Acteur::Famille, c, &r, &ouvrir()).est_permis(), "{c:?}");
        }
    }

    // ─── Élèves (§8.2, §10) ─────────────────────────────────────────────────────────────────

    /// Entre élèves : fermé par défaut (Q-M6) ; jamais vers un autre établissement.
    #[test]
    fn eleve_vers_eleve_ferme_par_defaut_et_jamais_hors_etablissement() {
        let defaut = ReglagesMatrice::default();
        assert_eq!(
            d(Acteur::EleveMineur, Cible::AutreEleve, &defaut, &ouvrir()),
            Decision::Refuse(Refus::Reglage(Reglage::ElevesVersEleves))
        );
        let ouvert = ReglagesMatrice {
            eleves_vers_eleves: EleveVersEleve::Delegues,
            ..defaut
        };
        assert!(
            d(Acteur::EleveMineur, Cible::AutreEleve, &ouvert, &ouvrir()).est_permis(),
            "témoin"
        );
        let ailleurs = Faits {
            autre_etablissement: true,
            ..ouvrir()
        };
        assert_eq!(
            d(Acteur::EleveMineur, Cible::AutreEleve, &ouvert, &ailleurs),
            Decision::Refuse(Refus::AutreEtablissement)
        );
        // Un adulte n'ouvre pas non plus un fil entre élèves.
        assert_eq!(
            d(Acteur::Professeur, Cible::AutreEleve, &ouvert, &ouvrir()),
            Decision::Refuse(Refus::Matrice)
        );
    }

    /// Élève mineur : vers ses professeurs (réglage), vers la vie scolaire, jamais vers la direction
    /// ni les familles.
    #[test]
    fn eleve_mineur_vers_ses_professeurs_la_vie_scolaire_et_la_sante() {
        let defaut = ReglagesMatrice::default();
        assert!(d(Acteur::EleveMineur, Cible::Professeur, &defaut, &ouvrir()).est_permis());
        assert!(d(
            Acteur::EleveMineur,
            Cible::BoiteVieScolaire,
            &defaut,
            &ouvrir()
        )
        .est_permis());
        assert!(d(Acteur::EleveMineur, Cible::Sante, &defaut, &ouvrir()).est_permis());
        assert_eq!(
            d(Acteur::EleveMineur, Cible::Direction, &defaut, &ouvrir()),
            Decision::Refuse(Refus::Matrice)
        );
        assert_eq!(
            d(Acteur::EleveMineur, Cible::Famille, &defaut, &ouvrir()),
            Decision::Refuse(Refus::Matrice)
        );
        let ferme = ReglagesMatrice {
            eleves_vers_professeurs: false,
            ..defaut
        };
        assert_eq!(
            d(Acteur::EleveMineur, Cible::Professeur, &ferme, &ouvrir()),
            Decision::Refuse(Refus::Reglage(Reglage::ElevesVersProfesseurs))
        );
    }

    /// Secrétariat → élèves : seulement les majeurs (réglage oui par défaut).
    #[test]
    fn secretariat_vers_eleves_majeurs_seulement() {
        let defaut = ReglagesMatrice::default();
        let majeur = Faits {
            eleve_majeur: true,
            ..ouvrir()
        };
        let mineur = Faits {
            eleve_majeur: false,
            ..ouvrir()
        };
        assert!(
            d(Acteur::Secretariat, Cible::Eleve, &defaut, &majeur).est_permis(),
            "témoin"
        );
        assert_eq!(
            d(Acteur::Secretariat, Cible::Eleve, &defaut, &mineur),
            Decision::Refuse(Refus::EleveMineurSeulementParLesResponsables)
        );
        let ferme = ReglagesMatrice {
            secretariat_vers_majeurs: false,
            ..defaut
        };
        assert_eq!(
            d(Acteur::Secretariat, Cible::Eleve, &ferme, &majeur),
            Decision::Refuse(Refus::Reglage(Reglage::SecretariatVersMajeurs))
        );
    }

    // ─── Personnels ─────────────────────────────────────────────────────────────────────────

    /// AESH → familles : « non », « répondre » (défaut) ou « ouvrir ».
    #[test]
    fn aesh_vers_familles_non_repondre_ou_ouvrir() {
        let mk = |v| ReglagesMatrice {
            aesh_vers_familles: v,
            ..ReglagesMatrice::default()
        };
        assert!(
            d(
                Acteur::Aesh,
                Cible::Famille,
                &mk(AeshVersFamille::Repondre),
                &repondre()
            )
            .est_permis(),
            "témoin"
        );
        assert_eq!(
            d(
                Acteur::Aesh,
                Cible::Famille,
                &mk(AeshVersFamille::Repondre),
                &ouvrir()
            ),
            Decision::Refuse(Refus::Reglage(Reglage::AeshVersFamilles))
        );
        assert!(d(
            Acteur::Aesh,
            Cible::Famille,
            &mk(AeshVersFamille::Ouvrir),
            &ouvrir()
        )
        .est_permis());
        assert_eq!(
            d(
                Acteur::Aesh,
                Cible::Famille,
                &mk(AeshVersFamille::Non),
                &repondre()
            ),
            Decision::Refuse(Refus::Reglage(Reglage::AeshVersFamilles))
        );
    }

    /// Accueil : jamais vers une famille ni un élève (le message téléphonique crée un fil interne).
    #[test]
    fn accueil_n_ecrit_ni_aux_familles_ni_aux_eleves() {
        let r = ReglagesMatrice::default();
        assert_eq!(
            d(Acteur::Accueil, Cible::Famille, &r, &ouvrir()),
            Decision::Refuse(Refus::Matrice)
        );
        assert_eq!(
            d(Acteur::Accueil, Cible::Eleve, &r, &ouvrir()),
            Decision::Refuse(Refus::Matrice)
        );
        assert_eq!(
            d(Acteur::Accueil, Cible::Professeur, &r, &ouvrir()),
            Decision::Permis(Raison::MessageDeLiaison)
        );
    }

    /// La direction et la CPE écrivent partout (sauf les bornes) ; la santé n'écrit pas à une équipe.
    #[test]
    fn direction_et_cpe_ecrivent_partout_sauf_aux_bornes() {
        let r = ReglagesMatrice::default();
        for c in [
            Cible::Famille,
            Cible::Eleve,
            Cible::Professeur,
            Cible::Direction,
            Cible::Sante,
            Cible::Equipe,
        ] {
            assert!(
                d(Acteur::Direction, c, &r, &ouvrir()).est_permis(),
                "direction → {c:?}"
            );
            assert!(
                d(Acteur::CpeVieScolaire, c, &r, &ouvrir()).est_permis(),
                "CPE → {c:?}"
            );
        }
        assert_eq!(
            d(Acteur::Sante, Cible::Equipe, &r, &ouvrir()),
            Decision::Refuse(Refus::Matrice)
        );
    }

    /// Le tableau est total : aucune case ne panique, et les cases fermées le sont pour tous les
    /// réglages (une case « — » n'est jamais ouverte par un réglage).
    #[test]
    fn le_tableau_est_total_et_les_cases_fermees_restent_fermees() {
        let acteurs = [
            Acteur::Famille,
            Acteur::EleveMineur,
            Acteur::EleveMajeur,
            Acteur::Professeur,
            Acteur::ProfPrincipal,
            Acteur::CpeVieScolaire,
            Acteur::Secretariat,
            Acteur::Direction,
            Acteur::Sante,
            Acteur::Aesh,
            Acteur::Accueil,
        ];
        let cibles = [
            Cible::Famille,
            Cible::Eleve,
            Cible::Professeur,
            Cible::ProfPrincipal,
            Cible::BoiteVieScolaire,
            Cible::BoiteSecretariat,
            Cible::Direction,
            Cible::Sante,
            Cible::Aesh,
            Cible::Equipe,
            Cible::AutreFamille,
            Cible::AutreEleve,
        ];
        let tout_ouvert = ReglagesMatrice {
            familles_vers_professeurs: true,
            familles_vers_direction: true,
            eleves_vers_professeurs: true,
            eleves_vers_eleves: EleveVersEleve::Classe,
            aesh_vers_familles: AeshVersFamille::Ouvrir,
            secretariat_vers_majeurs: true,
            pieces_familles: true,
        };
        let faits = Faits {
            ouverture: true,
            raison: true,
            eleve_majeur: true,
            ..Faits::default()
        };
        for a in acteurs {
            for c in cibles {
                let case = cellule(a, c);
                let dec = decider(a, c, &tout_ouvert, &faits);
                if case == Cellule::Non {
                    assert!(
                        matches!(
                            dec,
                            Decision::Refuse(Refus::Matrice | Refus::FamilleVersFamille)
                        ),
                        "{a:?} → {c:?} doit rester fermé : {dec:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn reglages_texte_aller_retour() {
        for v in [
            EleveVersEleve::Non,
            EleveVersEleve::Delegues,
            EleveVersEleve::Classe,
        ] {
            assert_eq!(EleveVersEleve::parse(v.as_str()), Some(v));
        }
        for v in [
            AeshVersFamille::Non,
            AeshVersFamille::Repondre,
            AeshVersFamille::Ouvrir,
        ] {
            assert_eq!(AeshVersFamille::parse(v.as_str()), Some(v));
        }
        assert_eq!(EleveVersEleve::parse("tous"), None);
        assert_eq!(AeshVersFamille::parse(""), None);
    }
}
