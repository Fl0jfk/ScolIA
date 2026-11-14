//! **Catalogue de conservation** (socle 27, §8.2 et §8.2 bis ; lots G0 et G1).
//!
//! Constantes du code : chaque catégorie porte son code, son fait générateur, ses durées
//! **proposées**, son sort final et sa **source** avec la marque de vérification. Ce n'est
//! qu'une proposition de ScolIA : tant que l'établissement (et son DPO, s'il en a un) n'a pas
//! validé une catégorie, **aucun sort final n'est appliqué** (R-RG1). Les durées de ce fichier
//! ne déclenchent rien : le moteur de conservation (G6) n'existe pas encore.
//!
//! Garde-fous (tests de ce fichier, sans base) :
//! - R-RG12 : toute table « données personnelles » du registre a une catégorie connue ;
//! - R-RG13 : toute catégorie a une source, ou la mention « aucune source, choix de minimisation ».
//!
//! Écart à la conception : le catalogue vit dans `ent-db` (module), pas dans une crate
//! `ent-conformite` ; la séparation en crate se fera avec G2 si elle devient utile.

use crate::registre_donnees::ClasseTable;

/// Fiabilité d'une durée ou d'une source (marques de la conception : [V], [NV], [R]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fiabilite {
    /// Texte vérifié [V].
    Verifie,
    /// Texte non vérifié pour l'ENT [NV] : à faire confirmer.
    NonVerifie,
    /// Recommandation de ScolIA [R] (minimisation), sans texte.
    Recommande,
}

/// Sort final proposé. Jamais appliqué automatiquement (§8.3 point 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortFinal {
    Destruction,
    /// Tri par le service d'archives (instruction de 2005).
    Tri,
    /// Conservation définitive ou versement (ScolIA n'est pas un dépositaire agréé).
    ConservationDefinitive,
    /// Effacement du dossier (règle du socle 13, R511-13 IV).
    EffacementDossier,
    /// Suit le sort de l'objet auquel la ligne se rattache.
    SuitObjet,
    AnonymisationCompte,
    /// Suit la fin du contrat avec ScolIA.
    SuitContrat,
    /// Fixé par un autre socle (santé : socle 34).
    AutreSocle,
}

impl SortFinal {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Destruction => "destruction",
            Self::Tri => "tri",
            Self::ConservationDefinitive => "conservation_definitive",
            Self::EffacementDossier => "effacement_dossier",
            Self::SuitObjet => "suit_objet",
            Self::AnonymisationCompte => "anonymisation_compte",
            Self::SuitContrat => "suit_contrat",
            Self::AutreSocle => "autre_socle",
        }
    }
}

/// Une catégorie du catalogue.
#[derive(Clone, Copy, Debug)]
pub struct Categorie {
    pub code: &'static str,
    pub libelle: &'static str,
    /// Événement qui fait partir les durées.
    pub fait_generateur: &'static str,
    /// Utilité courante, en mois (`None` : sans durée propre, ex. « scolarité »).
    pub utilite_mois: Option<u16>,
    /// Conservation intermédiaire, en mois (`None` : non fixée ou suit un autre objet).
    pub intermediaire_mois: Option<u16>,
    pub sort: SortFinal,
    /// Texte de référence, ou « aucune source, choix de minimisation ».
    pub source: &'static str,
    pub fiabilite: Fiabilite,
    /// Un visa de l'administration des archives est exigé (public ; privé sous contrat : Q-RG2).
    pub visa: bool,
    /// Socle qui exécutera le sort.
    pub socle: &'static str,
}

/// Mention obligatoire quand aucune source n'existe (R-RG13).
pub const SANS_SOURCE: &str = "aucune source, choix de minimisation";

/// Version du catalogue : une nouvelle version est proposée à l'établissement comme
/// changement, jamais appliquée seule (§8.2 point 4).
pub const VERSION_CATALOGUE: &str = "2026-11-14-g1";

#[allow(clippy::too_many_arguments)]
const fn c(
    code: &'static str,
    libelle: &'static str,
    fait: &'static str,
    utilite: Option<u16>,
    inter: Option<u16>,
    sort: SortFinal,
    source: &'static str,
    fiabilite: Fiabilite,
    visa: bool,
    socle: &'static str,
) -> Categorie {
    Categorie {
        code,
        libelle,
        fait_generateur: fait,
        utilite_mois: utilite,
        intermediaire_mois: inter,
        sort,
        source,
        fiabilite,
        visa,
        socle,
    }
}

use Fiabilite::{NonVerifie as NV, Recommande as R, Verifie as V};
use SortFinal as S;

const INSTR_2005: &str = "instruction de tri des EPLE de 2005";

/// Le catalogue initial (§8.2 bis, complété des catégories que la carte des tables exige).
/// **À valider par le DPO et le service d'archives.**
#[rustfmt::skip]
pub const CATALOGUE: &[Categorie] = &[
    c("scol.registre", "registre des élèves, listes d'élèves", "fin d'année", Some(12), Some(600), S::ConservationDefinitive, INSTR_2005, V, true, "2"),
    c("scol.dossier", "dossier scolaire (identité, scolarité, orientation, décisions)", "sortie de l'élève", Some(12), Some(120), S::Tri, INSTR_2005, V, true, "2"),
    c("scol.bulletin", "bulletins, relevés, conseils de classe", "fin d'année", None, None, S::Tri, INSTR_2005, NV, true, "2"),
    c("vs.appel", "registres d'appel, présences, retards", "fin d'année", Some(12), Some(120), S::Destruction, "instruction de 2005 ; R131-5 du code de l'éducation", V, false, "12"),
    c("vs.justificatif", "mots d'excuse, justificatifs, certificats d'absence", "fin d'année", None, Some(12), S::Destruction, INSTR_2005, V, false, "12"),
    c("disc.sanction", "sanctions, punitions, incidents, procédures", "prononcé", None, None, S::EffacementDossier, "R511-13 IV du code de l'éducation", V, false, "13"),
    c("disc.conseil", "dossier du conseil de discipline, commissions", "décision", Some(12), Some(120), S::Tri, INSTR_2005, V, true, "13"),
    c("disc.registre", "registre des sanctions", "fin d'année", Some(12), Some(120), S::ConservationDefinitive, INSTR_2005, V, true, "13"),
    c("ins.demarche_non_aboutie", "dossiers d'admission refusés ou abandonnés", "fin de campagne", None, None, S::Destruction, SANS_SOURCE, NV, false, "3"),
    c("ins.piece", "pièces et dossiers d'une inscription aboutie", "validation", None, None, S::Destruction, SANS_SOURCE, NV, false, "3"),
    c("adm.decision_justice", "décisions sur l'autorité parentale, protection", "fin d'effet", None, None, S::Destruction, SANS_SOURCE, NV, false, "4"),
    c("adm.personne", "responsables, foyers, coordonnées, consentements", "fin de scolarité de l'élève", None, None, S::Destruction, SANS_SOURCE, R, false, "4"),
    c("adm.coordonnee_retiree", "coordonnées remplacées", "remplacement", None, Some(12), S::Destruction, SANS_SOURCE, R, false, "4"),
    c("fin.piece", "factures, quittances, pièces comptables", "clôture de l'exercice", Some(12), Some(120), S::Destruction, "code de commerce L123-22 (privé) ; instruction de 2005 (public)", V, false, "24"),
    c("fin.bourse", "dossiers individuels de bourse", "fin d'année", None, Some(24), S::Destruction, INSTR_2005, V, false, "24"),
    c("fin.iban", "coordonnées bancaires, mandats, prélèvements", "fin du mandat", None, Some(13), S::Destruction, "règles SEPA (13 mois après le dernier prélèvement)", NV, false, "25"),
    c("msg.conversation", "messagerie", "dernier message", Some(12), Some(12), S::Destruction, SANS_SOURCE, NV, false, "16"),
    c("cdt.contenu", "cahier de textes", "fin d'année", Some(24), Some(60), S::Destruction, "SDET 2024, annexe opérationnelle", V, false, "14"),
    c("eva.note", "notes hors bulletin, devoirs, évaluations", "fin d'année", Some(12), Some(12), S::Destruction, SANS_SOURCE, NV, false, "2"),
    c("acc.trace", "traces d'accès (consultations)", "consultation", Some(12), None, S::Destruction, "délibération CNIL 2021-122", V, false, "27"),
    c("acc.trace_controle", "traces gardées pour un contrôle interne motivé", "ouverture du contrôle", None, Some(36), S::Destruction, "CNIL", V, false, "27"),
    c("jrn.modification", "journal des modifications", "suit l'objet modifié", None, None, S::SuitObjet, "socle 1, exigence E7", V, false, "27"),
    c("cpt.compte", "comptes, rattachements, rôles, délégations", "dernière fin de rattachement", None, Some(3), S::AnonymisationCompte, SANS_SOURCE, R, false, "1"),
    c("cpt.securite", "tentatives de connexion, sessions, verrous", "événement", Some(6), None, S::Destruction, "CNIL", V, false, "1"),
    c("rh.dossier", "dossier du personnel", "départ", None, None, S::AutreSocle, "référentiel CNIL de la gestion du personnel", V, false, "6"),
    c("rh.absence", "absences, arrêts, remplacements des personnels", "fin d'année", None, None, S::AutreSocle, "référentiel CNIL de la gestion du personnel", NV, false, "15"),
    c("rh.candidat", "candidatures non retenues", "dernier contact", None, Some(24), S::Destruction, "CNIL", V, false, "6"),
    c("sante.dossier", "santé, PAI, infirmerie, psychologue (module désactivé)", "règles du socle 34", None, None, S::AutreSocle, "socle 34 (désactivé tant que l'hébergement HDS n'est pas obtenu)", NV, false, "34"),
    c("rgpd.gouvernance", "gouvernance de la conformité : DPO, règles de conservation, journal des décisions", "remplacement", None, Some(60), S::Destruction, SANS_SOURCE, R, false, "27"),
    c("rgpd.demande", "demandes de droits et réponses", "clôture", None, Some(60), S::Destruction, "prescription", NV, false, "27"),
    c("rgpd.violation", "registre des violations", "clôture", None, Some(60), S::Destruction, "article 33.5 du RGPD (durée non fixée par le texte)", NV, false, "27"),
    c("dem.demande", "demandes internes (matériel, HSE, reprographie)", "clôture", None, Some(12), S::Destruction, SANS_SOURCE, R, false, "43"),
    c("sor.sortie", "sorties, voyages, participants, prestataires", "fin de la sortie", None, Some(60), S::Destruction, SANS_SOURCE, NV, false, "42"),
    c("sta.stage", "stages, conventions, entreprises", "fin du stage", None, Some(60), S::Destruction, SANS_SOURCE, NV, false, "37"),
    c("vie.pratique", "restauration, internat, présences", "fin d'année", Some(12), Some(12), S::Destruction, SANS_SOURCE, R, false, "41"),
    c("edt.planning", "emploi du temps, agenda, rendez-vous", "fin d'année", Some(12), Some(12), S::Destruction, SANS_SOURCE, R, false, "10"),
    c("com.envoi", "files d'envoi, notifications, appareils", "envoi", Some(1), Some(12), S::Destruction, SANS_SOURCE, R, false, "8"),
    c("export.archive", "archives d'export", "génération", Some(0), None, S::Destruction, "PORTABILITE_EXPORTS.md §8 : 7 jours", V, false, "28"),
    c("tech.parametrage", "paramétrage et référentiels sans donnée personnelle", "vie de l'établissement", None, None, S::SuitContrat, SANS_SOURCE, R, false, "—"),
];

/// Profil de l'établissement : fixe la proposition de visa (R-RG5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profil {
    Public,
    PriveSousContrat,
    PriveHorsContrat,
}

impl Profil {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::PriveSousContrat => "prive_sous_contrat",
            Self::PriveHorsContrat => "prive_hors_contrat",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "public" => Some(Self::Public),
            "prive_sous_contrat" => Some(Self::PriveSousContrat),
            "prive_hors_contrat" => Some(Self::PriveHorsContrat),
            _ => None,
        }
    }
}

/// Visa exigé pour une catégorie selon le profil : public, ou privé sous contrat tant que le juriste n'a
/// pas tranché (R-RG5, Q-RG2) ; jamais pour le privé hors contrat.
pub fn visa_exige(c: &Categorie, profil: Profil) -> bool {
    c.visa && profil != Profil::PriveHorsContrat
}

/// Bornes dans lesquelles un établissement peut ajuster une règle (§8.2 point 2) : jamais en dessous
/// d'un minimum légal connu, jamais au-dessus d'un maximum légal connu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bornes {
    pub utilite_min: Option<u16>,
    pub utilite_max: Option<u16>,
    pub intermediaire_min: Option<u16>,
    pub intermediaire_max: Option<u16>,
}

/// Valeur la plus haute acceptée pour toute durée (100 ans) : garde contre une saisie absurde.
pub const DUREE_MAX_MOIS: u16 = 1200;

/// Bornes d'une catégorie. Seules les bornes **vérifiées** [V] sont posées ; les autres durées restent
/// libres dans [0 ; 1200] mois, sous la responsabilité de l'établissement et de son DPO.
pub fn bornes(code: &str, profil: Profil) -> Bornes {
    match code {
        // Traces d'accès : durée bornée à 6–36 mois (S-541, CNIL 2021-122).
        "acc.trace" => Bornes {
            utilite_min: Some(6),
            utilite_max: Some(36),
            ..Bornes::default()
        },
        "acc.trace_controle" => Bornes {
            intermediaire_max: Some(36),
            ..Bornes::default()
        },
        // Pièces comptables du privé : 10 ans (code de commerce L123-22), utilité courante incluse.
        "fin.piece" if profil != Profil::Public => Bornes {
            intermediaire_min: Some(108),
            ..Bornes::default()
        },
        _ => Bornes::default(),
    }
}

// Le sort final d'une catégorie ne s'ajuste pas (registre à conserver, sanctions effacées selon
// R511-13 IV…) : seules les durées s'ajustent, dans leurs bornes.

/// Catégorie du catalogue par code.
pub fn categorie(code: &str) -> Option<&'static Categorie> {
    CATALOGUE.iter().find(|c| c.code == code)
}

/// Catégorie de conservation **proposée** pour une table (R-RG12). Carte à relire par le DPO ;
/// elle ne décide aucune durée appliquée.
pub fn categorie_de_table(t: &ClasseTable) -> &'static str {
    let n = t.nom;
    let has = |s: &str| n.contains(s);
    if n == "ent_journal_modif" {
        return "jrn.modification";
    }
    if t.sante && !matches!(t.domaine, "personnels_absences" | "assiduite") {
        return "sante.dossier";
    }
    match t.domaine {
        "assiduite" => {
            if has("justificatif") || has("declaration") || n == "ent_dispense_vs" {
                "vs.justificatif"
            } else if has("sortie") {
                "vie.pratique"
            } else {
                "vs.appel"
            }
        }
        "cahier_textes" => "cdt.contenu",
        "demandes" => "dem.demande",
        "finances" => {
            if has("sepa") || has("iban") || has("prenotification") {
                "fin.iban"
            } else if has("bourse") {
                "fin.bourse"
            } else {
                "fin.piece"
            }
        }
        "evaluations_pedagogie" => {
            if has("bulletin") || has("conseil") || has("orientation") || has("bilan") {
                "scol.bulletin"
            } else if has("cahier_texte") || has("cdt") || has("seance_contenu") || has("travail") {
                "cdt.contenu"
            } else {
                "eva.note"
            }
        }
        "inscriptions" => {
            if has("campagne") && !t.donnees_personnelles {
                "tech.parametrage"
            } else {
                "ins.piece"
            }
        }
        "dossier_personnes" => {
            if has("histo") || has("version") {
                "adm.coordonnee_retiree"
            } else {
                "adm.personne"
            }
        }
        "dossier_eleve" => {
            if has("convention_stage") || n == "ent_entreprise" {
                "sta.stage"
            } else if n == "ent_annonce" || has("formulaire") || has("doc") || n == "ent_objet" {
                "msg.conversation"
            } else if has("regle_effacement") {
                "tech.parametrage"
            } else if has("import") || has("journal") {
                "jrn.modification"
            } else if has("cours")
                || has("rencontre")
                || has("rdv")
                || has("lieu")
                || has("salle")
                || has("calendrier")
                || has("affectation")
                || has("service")
                || has("disponibilite")
                || has("equipe")
                || has("fermeture")
                || has("contrainte")
                || has("creneau")
            {
                "edt.planning"
            } else {
                "scol.dossier"
            }
        }
        "conformite" => "rgpd.gouvernance",
        "vie_scolaire" => "disc.sanction",
        "messagerie" => "msg.conversation",
        "communication_technique" => "com.envoi",
        "sorties_voyages" => "sor.sortie",
        "stages" => "sta.stage",
        "vie_pratique" => "vie.pratique",
        "emploi_du_temps_structure" => "edt.planning",
        "personnels" => "rh.dossier",
        "personnels_absences" => "rh.absence",
        "portails" => "edt.planning",
        "identite_acces" => {
            if has("session")
                || has("login_guard")
                || has("rate_limit")
                || has("challenge")
                || has("state")
                || has("password_reset")
                || has("email_change")
                || has("recuperation")
            {
                "cpt.securite"
            } else {
                "cpt.compte"
            }
        }
        "structure_scolaire" if t.donnees_personnelles => "scol.dossier",
        _ => "tech.parametrage",
    }
}

/// Catégorie d'une table du registre, par nom.
pub fn categorie_de_nom(nom: &str) -> Option<&'static str> {
    crate::registre_donnees::classer(nom).map(categorie_de_table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registre_donnees::REGISTRE;
    use std::collections::BTreeSet;

    #[test]
    fn codes_du_catalogue_uniques() {
        let codes: BTreeSet<_> = CATALOGUE.iter().map(|c| c.code).collect();
        assert_eq!(codes.len(), CATALOGUE.len(), "code de catégorie en double");
    }

    /// R-RG13 : chaque catégorie a une source, ou la mention explicite.
    #[test]
    fn chaque_categorie_a_une_source() {
        for c in CATALOGUE {
            assert!(
                !c.source.trim().is_empty(),
                "{} : source vide (R-RG13)",
                c.code
            );
            assert!(
                !c.libelle.trim().is_empty() && !c.fait_generateur.trim().is_empty(),
                "{}",
                c.code
            );
            // Une durée sans texte ne peut pas être présentée comme vérifiée.
            if c.source == SANS_SOURCE {
                assert_ne!(
                    c.fiabilite,
                    Fiabilite::Verifie,
                    "{} : « vérifié » sans source",
                    c.code
                );
            }
        }
    }

    /// R-RG12 : toute table « données personnelles » (et toute table classée) a une catégorie
    /// connue du catalogue.
    #[test]
    fn chaque_table_de_donnees_personnelles_a_une_categorie() {
        let mut inconnues = Vec::new();
        for t in REGISTRE {
            let code = categorie_de_table(t);
            if categorie(code).is_none() {
                inconnues.push(format!("{} -> {code}", t.nom));
            }
            if t.donnees_personnelles {
                assert_ne!(
                    code, "tech.parametrage",
                    "{} : donnée personnelle classée « sans donnée personnelle »",
                    t.nom
                );
            }
        }
        assert!(
            inconnues.is_empty(),
            "catégorie inconnue du catalogue : {inconnues:?}"
        );
    }

    /// Une table de santé relève toujours de `sante.dossier` (sauf absences de personnel, rh.absence).
    #[test]
    fn la_sante_suit_le_socle_34() {
        for t in REGISTRE
            .iter()
            .filter(|t| t.sante && !matches!(t.domaine, "personnels_absences" | "assiduite"))
        {
            assert_eq!(categorie_de_table(t), "sante.dossier", "{}", t.nom);
        }
    }
}
