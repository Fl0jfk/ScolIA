//! Socle N1, lot N1-1 : **catalogue des natures**, raccourcis vers les socles propriétaires, files par défaut et
//! assemblage (§8.4, §8.5 de `docs/socles/N1_DEMANDES.md`).
//!
//! * [`NATURES_BASCULE`] complète [`crate::demandes::NATURES`] (les quatre natures du cœur) avec les autres natures
//!   intégrées de la bascule, familles et élèves compris (D-061, Q-DM2). Les natures des briques propriétaires
//!   (44, 48, 49…) sont déclarées ici avec leur propriétaire tant que leur crate n'existe pas.
//! * [`RACCOURCIS`] : une démarche qui a un circuit chez une brique est un **raccourci** (lien profond), jamais une
//!   demande N1 (R-DM1, R-DM3, R-DM31).
//! * [`METAS`] : ce que le noyau applique et que [`crate::demandes::NatureDecl`] ne porte pas (propriétaire,
//!   catégorie de routage, natures d'avis et de tâches déclarées).
//! * [`FILES_DEFAUT`] : les files que l'établissement obtient d'un geste (`initialiser_files_defaut`), avec leurs
//!   règles de membres **calculées** depuis le socle 6 (fonctions, missions) : aucune liste d'adresses (NJ-94).
//! * [`verifier_catalogue`] : l'assemblage refuse un catalogue incomplet (`t_kit_nature_complete_catalogue`).

use crate::demandes::{
    champ, ChampDecl, ChampType, Confidentialite, Conservation, NatureDecl, PublicNature, NATURES,
};

/// Natures d'avis déclarées au socle 8 (l'adaptateur de notification n'existe pas encore : la déclaration est la
/// seule moitié livrée). `demandes.prise_en_charge` est **activée par défaut** pour toutes les natures (§37.1).
pub const AVIS_DECLARES: &[&str] = &[
    "demandes.deposee",
    "demandes.vue_par_service",
    "demandes.prise_en_charge",
    "demandes.complement_demande",
    "demandes.terminee",
    "demandes.en_retard",
];

/// Natures de tâche déclarées au socle 29 (« Ma file », rapprochement quand le 29 arrive).
pub const TACHES_DECLAREES: &[&str] = &["demandes.a_traiter", "demandes.complement", "demandes.en_retard"];

/// Ce que le noyau applique à une nature et que sa déclaration de champs ne porte pas.
#[derive(Debug, Clone, Copy)]
pub struct NatureMeta {
    pub code: &'static str,
    /// `n1` (nature intégrée) ou numéro du socle propriétaire du volet (`44`, `48`, `49`, `23`, `24`).
    pub proprietaire: &'static str,
    /// Champ liste de la nature qui sert de critère de **catégorie** au routage.
    pub categorie_champ: Option<&'static str>,
    /// EPLE : accusé de réception à mentions (CRPA, décision implicite) (§6.1).
    pub decision_usager: bool,
    pub avis: &'static [&'static str],
    pub taches: &'static [&'static str],
}

/// Un raccourci : aucune demande N1 n'est créée, aucun champ n'est déclaré.
#[derive(Debug, Clone, Copy)]
pub struct RaccourciDecl {
    pub code: &'static str,
    pub libelle: &'static str,
    pub mots_cles: &'static [&'static str],
    /// Socle qui porte le circuit.
    pub socle: &'static str,
    /// Lien profond vers l'écran du socle propriétaire (chemin interne, jamais une URL externe).
    pub lien: &'static str,
    pub publics: &'static [PublicNature],
}

const PERS: &[PublicNature] = &[PublicNature::Personnel];
const FAMILLE: &[PublicNature] = &[PublicNature::Responsable, PublicNature::EleveMajeur];
const FAMILLE_ELEVE: &[PublicNature] =
    &[PublicNature::Responsable, PublicNature::EleveMajeur, PublicNature::EleveMineur];

/// Les natures intégrées de la bascule qui s'ajoutent aux quatre natures du cœur (`demandes::NATURES`, dont
/// l'ordre est figé : des tests du lot N1-0 les indexent).
pub const NATURES_BASCULE: &[NatureDecl] = &[
    NatureDecl {
        code: "numerique.assistance",
        version: 1,
        libelle: "Assistance numérique",
        mots_cles: &["mot de passe", "ordinateur", "imprimante", "logiciel", "ENT", "informatique", "poste"],
        publics: PERS,
        confidentialite: Confidentialite::C1,
        champs: &[
            champ(
                "zone",
                ChampType::Liste(&["compte", "mot_de_passe", "poste", "imprimante", "logiciel", "ent", "autre"]),
                true,
            ),
            champ("precision_poste", ChampType::Texte { max: 120 }, false),
        ],
        texte_max: Some(2000),
        eleve_requis: false,
        delai_prise_jours: 1,
        delai_traitement_jours: 5,
        conservation: Conservation::TechniqueCourte,
        file_defaut: "numerique",
    },
    NatureDecl {
        code: "numerique.acces_compte",
        version: 1,
        libelle: "Je n'arrive pas à me connecter",
        mots_cles: &["connexion", "identifiant", "mot de passe oublié", "compte bloqué", "se connecter"],
        publics: FAMILLE_ELEVE,
        confidentialite: Confidentialite::C1,
        champs: &[champ(
            "probleme",
            ChampType::Liste(&["mot_de_passe_oublie", "compte_bloque", "pas_de_compte", "autre"]),
            true,
        )],
        texte_max: Some(1000),
        eleve_requis: false,
        delai_prise_jours: 1,
        delai_traitement_jours: 3,
        conservation: Conservation::TechniqueCourte,
        file_defaut: "numerique",
    },
    NatureDecl {
        code: "entretien.demande",
        version: 1,
        libelle: "Demande d'entretien ou de ménage",
        mots_cles: &["ménage", "nettoyage", "poubelles", "salle à préparer", "entretien"],
        publics: PERS,
        confidentialite: Confidentialite::C0,
        champs: &[
            champ(
                "type_entretien",
                ChampType::Liste(&["salle_a_preparer", "nettoyage", "poubelles", "autre"]),
                true,
            ),
            champ("lieu_id", ChampType::Ref, false),
            champ("date_souhaitee", ChampType::Date, false),
        ],
        texte_max: Some(1000),
        eleve_requis: false,
        delai_prise_jours: 2,
        delai_traitement_jours: 5,
        conservation: Conservation::TechniqueCourte,
        file_defaut: "entretien",
    },
    NatureDecl {
        code: "comptabilite.demande_interne",
        version: 1,
        libelle: "Demande à la comptabilité",
        mots_cles: &["facture", "bon de commande", "avance", "remboursement", "frais"],
        publics: PERS,
        confidentialite: Confidentialite::C1,
        champs: &[
            champ(
                "type_demande",
                ChampType::Liste(&["facture", "bon_de_commande", "avance", "remboursement_frais"]),
                true,
            ),
            champ("montant", ChampType::Decimal, false),
        ],
        texte_max: Some(2000),
        eleve_requis: false,
        delai_prise_jours: 2,
        delai_traitement_jours: 10,
        conservation: Conservation::Comptable,
        file_defaut: "comptabilite",
    },
    NatureDecl {
        code: "achat.fournitures",
        version: 1,
        libelle: "Demande de fournitures ou de petit matériel",
        mots_cles: &["fournitures", "matériel", "commande", "papier", "stylos"],
        publics: PERS,
        confidentialite: Confidentialite::C0,
        champs: &[
            champ("quantite", ChampType::Entier { min: 1, max: 10_000 }, true),
            champ("reference", ChampType::Texte { max: 120 }, false),
        ],
        texte_max: Some(1000),
        eleve_requis: false,
        delai_prise_jours: 2,
        delai_traitement_jours: 10,
        conservation: Conservation::TechniqueCourte,
        file_defaut: "intendance",
    },
    NatureDecl {
        code: "direction.question",
        version: 1,
        libelle: "Question ou demande à la direction",
        mots_cles: &["direction", "chef d'établissement", "question", "rendez-vous"],
        publics: PERS,
        confidentialite: Confidentialite::C1,
        champs: &[],
        texte_max: Some(4000),
        eleve_requis: false,
        delai_prise_jours: 2,
        delai_traitement_jours: 10,
        conservation: Conservation::InterneCourante,
        file_defaut: "secretariat_direction",
    },
    NatureDecl {
        code: "vie_scolaire.demande_interne",
        version: 1,
        libelle: "Demande à la vie scolaire",
        mots_cles: &["surveillance", "salle d'étude", "appel", "vie scolaire"],
        publics: PERS,
        confidentialite: Confidentialite::C1,
        champs: &[champ(
            "type_demande",
            ChampType::Liste(&["surveillance", "salle_etude", "appel_a_faire_passer", "autre"]),
            true,
        )],
        texte_max: Some(2000),
        eleve_requis: false,
        delai_prise_jours: 1,
        delai_traitement_jours: 3,
        conservation: Conservation::InterneCourante,
        file_defaut: "vie_scolaire",
    },
    NatureDecl {
        code: "scolarite.duplicata",
        version: 1,
        libelle: "Duplicata de bulletin, relevé de notes ou diplôme d'établissement",
        mots_cles: &["duplicata", "bulletin", "relevé de notes", "diplôme"],
        publics: FAMILLE,
        confidentialite: Confidentialite::C1,
        champs: &[
            champ("document", ChampType::Liste(&["bulletin", "releve_notes", "diplome_etablissement"]), true),
            champ("periode", ChampType::Texte { max: 120 }, false),
        ],
        texte_max: Some(1000),
        eleve_requis: true,
        delai_prise_jours: 2,
        delai_traitement_jours: 7,
        conservation: Conservation::FamilleCourante,
        file_defaut: "secretariat_scolarite",
    },
    NatureDecl {
        code: "scolarite.carte_eleve",
        version: 1,
        libelle: "Carte d'élève, carnet de correspondance ou casier",
        mots_cles: &["carte", "carnet", "casier", "perdu", "cantine"],
        publics: FAMILLE_ELEVE,
        confidentialite: Confidentialite::C0,
        champs: &[champ(
            "objet_perdu",
            ChampType::Liste(&["carte_eleve", "carnet_correspondance", "casier", "carte_cantine"]),
            true,
        )],
        texte_max: Some(500),
        eleve_requis: true,
        delai_prise_jours: 1,
        delai_traitement_jours: 3,
        conservation: Conservation::FamilleCourante,
        file_defaut: "vie_scolaire",
    },
    NatureDecl {
        code: "secretariat.demande_famille",
        version: 1,
        libelle: "Demande au secrétariat",
        mots_cles: &["secrétariat", "renseignement", "courrier", "autre démarche"],
        publics: FAMILLE,
        confidentialite: Confidentialite::C1,
        champs: &[],
        texte_max: Some(2000),
        eleve_requis: true,
        delai_prise_jours: 2,
        delai_traitement_jours: 5,
        conservation: Conservation::FamilleCourante,
        file_defaut: "secretariat_scolarite",
    },
    NatureDecl {
        code: "comptabilite.demande_famille",
        version: 1,
        libelle: "Demande à la comptabilité",
        mots_cles: &["facture", "attestation de paiement", "frais de scolarité", "paiement"],
        publics: FAMILLE,
        confidentialite: Confidentialite::C1,
        champs: &[champ(
            "type_demande",
            ChampType::Liste(&["attestation_paiement", "facture", "question_facture"]),
            true,
        )],
        texte_max: Some(2000),
        eleve_requis: true,
        delai_prise_jours: 2,
        delai_traitement_jours: 7,
        conservation: Conservation::Comptable,
        file_defaut: "comptabilite",
    },
    NatureDecl {
        code: "rh.hse",
        version: 1,
        libelle: "Demande d'heures supplémentaires effectives (HSE)",
        mots_cles: &["HSE", "heures supplémentaires", "soutien", "atelier", "heures faites", "accompagnement"],
        publics: PERS,
        confidentialite: Confidentialite::C1,
        champs: &[
            champ(
                "motif",
                ChampType::Liste(&[
                    "soutien",
                    "atelier",
                    "accompagnement_sortie",
                    "stage_reussite",
                    "preparation_concours",
                    "tutorat",
                    "autre",
                ]),
                true,
            ),
            // Quarts d'heure : D-061, les HSE se comptent en heures, jamais en euros.
            champ("volume_quarts", ChampType::Entier { min: 1, max: 1600 }, true),
            champ("periode_du", ChampType::Date, true),
            champ("periode_au", ChampType::Date, true),
            champ("lieu", ChampType::Texte { max: 120 }, false),
        ],
        texte_max: Some(2000),
        eleve_requis: false,
        delai_prise_jours: 2,
        delai_traitement_jours: 10,
        conservation: Conservation::Rh,
        file_defaut: "direction",
    },
    NatureDecl {
        code: "reprographie.tirage",
        version: 1,
        libelle: "Demande de copies (reprographie)",
        mots_cles: &["photocopies", "copies", "reprographie", "imprimer", "tirage", "polycopiés"],
        publics: PERS,
        confidentialite: Confidentialite::C1,
        champs: &[
            champ("format", ChampType::Liste(&["a4", "a3"]), true),
            champ("couleur", ChampType::Bool, true),
            champ("recto_verso", ChampType::Bool, false),
            champ("agrafage", ChampType::Bool, false),
            champ("exemplaires", ChampType::Entier { min: 1, max: 5000 }, true),
            champ("pour_le", ChampType::Date, true),
            champ("pour_heure", ChampType::Heure, false),
            champ("oeuvres_protegees", ChampType::Bool, false),
        ],
        texte_max: Some(1000),
        eleve_requis: false,
        delai_prise_jours: 1,
        delai_traitement_jours: 3,
        conservation: Conservation::Reprographie,
        file_defaut: "reprographie",
    },
];

/// Raccourcis : la démarche a un circuit chez une brique ; le guichet y mène, il ne crée rien (R-DM1).
pub const RACCOURCIS: &[RaccourciDecl] = &[
    RaccourciDecl {
        code: "absences.declarer",
        libelle: "Déclarer une absence ou demander une autorisation d'absence",
        mots_cles: &["absence", "congé", "autorisation d'absence", "arrêt maladie"],
        socle: "15",
        lien: "/absences-personnels/declarer",
        publics: PERS,
    },
    RaccourciDecl {
        code: "agenda.salle",
        libelle: "Réserver une salle",
        mots_cles: &["réserver", "salle", "réservation"],
        socle: "19",
        lien: "/agenda/reserver-salle",
        publics: PERS,
    },
    RaccourciDecl {
        code: "ressources.materiel",
        libelle: "Emprunter du matériel",
        mots_cles: &["emprunt", "matériel", "vidéoprojecteur", "prêt"],
        socle: "44",
        lien: "/ressources/emprunter",
        publics: PERS,
    },
    RaccourciDecl {
        code: "sorties.projet",
        libelle: "Proposer une sortie ou un voyage",
        mots_cles: &["sortie", "voyage", "séjour", "projet"],
        socle: "42",
        lien: "/sorties/proposer",
        publics: PERS,
    },
    RaccourciDecl {
        code: "scolarite.certificat",
        libelle: "Certificat de scolarité",
        mots_cles: &["certificat de scolarité", "certificat", "scolarité"],
        socle: "7",
        lien: "/documents/certificat-scolarite",
        publics: FAMILLE,
    },
    RaccourciDecl {
        code: "assiduite.justifier",
        libelle: "Justifier une absence de mon enfant",
        mots_cles: &["justifier", "absence", "retard", "mot d'excuse"],
        socle: "12",
        lien: "/assiduite/justifier",
        publics: FAMILLE,
    },
    RaccourciDecl {
        code: "administratif.coordonnees",
        libelle: "Changer mes coordonnées ou ma situation",
        mots_cles: &["adresse", "téléphone", "coordonnées", "déménagement", "situation"],
        socle: "4",
        lien: "/administratif/coordonnees",
        publics: FAMILLE,
    },
    RaccourciDecl {
        code: "restauration.regime",
        libelle: "Changer de régime (demi-pension, externat)",
        mots_cles: &["demi-pension", "externat", "cantine", "régime"],
        socle: "40",
        lien: "/restauration/regime",
        publics: FAMILLE,
    },
    RaccourciDecl {
        code: "inscriptions.depart",
        libelle: "Départ, radiation ou exeat",
        mots_cles: &["départ", "radiation", "exeat", "changer d'établissement"],
        socle: "3",
        lien: "/inscriptions/depart",
        publics: FAMILLE,
    },
    RaccourciDecl {
        code: "examens.amenagement",
        libelle: "Aménagement d'examen",
        mots_cles: &["aménagement", "tiers-temps", "examen"],
        socle: "38",
        lien: "/examens/amenagement",
        publics: FAMILLE,
    },
    RaccourciDecl {
        code: "agenda.rdv",
        libelle: "Demander un rendez-vous",
        mots_cles: &["rendez-vous", "rencontrer", "entretien"],
        socle: "19",
        lien: "/agenda/rendez-vous",
        publics: FAMILLE_ELEVE,
    },
];

/// Canal protégé et numéros d'urgence : jamais une demande N1 (R-DM31). Affiché en tête du catalogue.
pub const SIGNALEMENT_GRAVE: &str = "Je veux signaler une situation grave";
pub const NUMEROS_URGENCE: &[&str] = &["3018", "119", "15", "17"];

macro_rules! meta {
    ($code:literal, $prop:literal, $cat:expr, $dec:expr) => {
        NatureMeta {
            code: $code,
            proprietaire: $prop,
            categorie_champ: $cat,
            decision_usager: $dec,
            avis: AVIS_DECLARES,
            taches: TACHES_DECLAREES,
        }
    };
}

/// Une entrée par nature de [`toutes_les_natures`].
pub const METAS: &[NatureMeta] = &[
    meta!("secretariat.demande_interne", "n1", None, false),
    meta!("maintenance.intervention", "44", Some("gravite"), false),
    meta!("rh.demande_agent", "49", Some("type_demande"), false),
    meta!("scolarite.attestation", "n1", Some("type_attestation"), true),
    meta!("numerique.assistance", "n1", Some("zone"), false),
    meta!("numerique.acces_compte", "n1", Some("probleme"), false),
    meta!("entretien.demande", "44", Some("type_entretien"), false),
    meta!("comptabilite.demande_interne", "48", Some("type_demande"), false),
    meta!("achat.fournitures", "48", None, false),
    meta!("direction.question", "n1", None, false),
    meta!("vie_scolaire.demande_interne", "n1", Some("type_demande"), false),
    meta!("scolarite.duplicata", "23", Some("document"), true),
    meta!("scolarite.carte_eleve", "n1", Some("objet_perdu"), false),
    meta!("secretariat.demande_famille", "n1", None, false),
    meta!("comptabilite.demande_famille", "24", Some("type_demande"), true),
    meta!("rh.hse", "n1", Some("motif"), false),
    // Le socle 44 reprendra la déclaration (même code, mêmes tables, R-DM45).
    meta!("reprographie.tirage", "n1", Some("format"), false),
];

/// Toutes les natures N1 (cœur et bascule), dans un ordre stable.
pub fn toutes_les_natures() -> impl Iterator<Item = &'static NatureDecl> {
    NATURES.iter().chain(NATURES_BASCULE.iter())
}

/// La nature déclarée sous ce code (cœur ou bascule).
pub fn nature_declaree(code: &str) -> Option<&'static NatureDecl> {
    toutes_les_natures().find(|n| n.code == code)
}

pub fn meta(code: &str) -> Option<&'static NatureMeta> {
    METAS.iter().find(|m| m.code == code)
}

pub fn raccourci(code: &str) -> Option<&'static RaccourciDecl> {
    RACCOURCIS.iter().find(|r| r.code == code)
}

// ───────────────────────── Files par défaut ─────────────────────────

/// Une source de membres déclarée par défaut : toujours une fonction ou un type de mission du socle 6.
#[derive(Debug, Clone, Copy)]
pub enum SourceDefaut {
    Fonction(&'static str),
    Mission(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub struct MembreDefaut {
    pub source: SourceDefaut,
    pub responsable: bool,
    pub rang: i16,
}

const fn fonction(code: &'static str, responsable: bool, rang: i16) -> MembreDefaut {
    MembreDefaut { source: SourceDefaut::Fonction(code), responsable, rang }
}
const fn mission(code: &'static str, responsable: bool, rang: i16) -> MembreDefaut {
    MembreDefaut { source: SourceDefaut::Mission(code), responsable, rang }
}

#[derive(Debug, Clone, Copy)]
pub struct FileDecl {
    pub code: &'static str,
    pub libelle: &'static str,
    pub role_traitant: &'static str,
    /// `service` · `tri` · `direction` · `confidentielle` · `repli`.
    pub type_: &'static str,
    pub service_calendrier: &'static str,
    /// Code de la file de repli (obligatoire sauf pour le repli de dernier recours).
    pub repli: Option<&'static str>,
    pub membres: &'static [MembreDefaut],
    pub message_accueil: Option<&'static str>,
}

/// Les files, dans l'ordre de création : chaque file vient après son repli.
pub const FILES_DEFAUT: &[FileDecl] = &[
    FileDecl {
        code: "repli_rh",
        libelle: "Repli RH : attaché de gestion et direction",
        role_traitant: "la direction et l'attaché de gestion",
        type_: "repli",
        service_calendrier: "secretariat",
        repli: None,
        membres: &[
            fonction("gestionnaire", false, 1),
            fonction("chef_etablissement", true, 2),
            fonction("directeur_ecole", true, 2),
        ],
        message_accueil: None,
    },
    FileDecl {
        code: "direction",
        libelle: "Direction",
        role_traitant: "la direction",
        type_: "direction",
        service_calendrier: "secretariat",
        repli: Some("repli_rh"),
        membres: &[
            fonction("chef_etablissement", true, 1),
            fonction("directeur_ecole", true, 1),
            fonction("adjoint", false, 2),
        ],
        message_accueil: None,
    },
    FileDecl {
        code: "tri",
        libelle: "Tri des demandes",
        role_traitant: "le secrétariat de direction",
        type_: "tri",
        service_calendrier: "secretariat",
        repli: Some("direction"),
        membres: &[fonction("secretaire_direction", true, 1)],
        message_accueil: None,
    },
    FileDecl {
        code: "secretariat_direction",
        libelle: "Secrétariat de direction",
        role_traitant: "le secrétariat de direction",
        type_: "service",
        service_calendrier: "secretariat",
        repli: Some("tri"),
        membres: &[fonction("secretaire_direction", true, 1)],
        message_accueil: None,
    },
    FileDecl {
        code: "secretariat",
        libelle: "Secrétariat",
        role_traitant: "le secrétariat",
        type_: "service",
        service_calendrier: "secretariat",
        repli: Some("tri"),
        membres: &[fonction("secretaire_direction", false, 1), fonction("secretaire_scolarite", false, 1)],
        message_accueil: None,
    },
    FileDecl {
        code: "secretariat_scolarite",
        libelle: "Secrétariat des élèves",
        role_traitant: "le secrétariat des élèves",
        type_: "service",
        service_calendrier: "secretariat",
        repli: Some("tri"),
        membres: &[fonction("secretaire_scolarite", true, 1)],
        message_accueil: Some("Les attestations demandées avant midi sont en général prêtes le lendemain."),
    },
    FileDecl {
        code: "maintenance",
        libelle: "Maintenance",
        role_traitant: "le service de maintenance",
        type_: "service",
        service_calendrier: "intendance",
        repli: Some("tri"),
        membres: &[fonction("agent_maintenance", false, 1)],
        message_accueil: Some("En cas de danger (gaz, électricité, fuite importante) : quittez les lieux et appelez le 18."),
    },
    FileDecl {
        code: "entretien",
        libelle: "Entretien",
        role_traitant: "le service d'entretien",
        type_: "service",
        service_calendrier: "intendance",
        repli: Some("tri"),
        membres: &[fonction("agent_entretien", false, 1)],
        message_accueil: None,
    },
    FileDecl {
        code: "numerique",
        libelle: "Assistance numérique",
        role_traitant: "l'assistance numérique",
        type_: "service",
        service_calendrier: "secretariat",
        repli: Some("tri"),
        membres: &[mission("referent_numerique", true, 1), fonction("technicien_informatique", false, 2)],
        message_accueil: None,
    },
    FileDecl {
        code: "comptabilite",
        libelle: "Comptabilité",
        role_traitant: "la comptabilité",
        type_: "service",
        service_calendrier: "intendance",
        repli: Some("tri"),
        membres: &[fonction("comptable", false, 1), fonction("gestionnaire", true, 1)],
        message_accueil: None,
    },
    FileDecl {
        code: "intendance",
        libelle: "Intendance",
        role_traitant: "l'intendance",
        type_: "service",
        service_calendrier: "intendance",
        repli: Some("tri"),
        membres: &[fonction("gestionnaire", true, 1)],
        message_accueil: None,
    },
    FileDecl {
        code: "vie_scolaire",
        libelle: "Vie scolaire",
        role_traitant: "la vie scolaire",
        type_: "service",
        service_calendrier: "vie_scolaire",
        repli: Some("tri"),
        membres: &[fonction("cpe", true, 1), fonction("aed", false, 2), fonction("surveillant", false, 2)],
        message_accueil: None,
    },
    FileDecl {
        code: "reprographie",
        libelle: "Reprographie",
        role_traitant: "la reprographie",
        type_: "service",
        service_calendrier: "secretariat",
        repli: Some("tri"),
        // Passerelle de la demande ; la file d'IMPRESSION est tenue par SITE (désignation du réceptionnaire,
        // `ent_dem_repro_receptionnaire`, NJ-77), jamais par un rôle entier.
        membres: &[fonction("gestionnaire", true, 1), fonction("secretaire_direction", false, 2)],
        message_accueil: Some("La reprographie a besoin d'un délai : indiquez pour quand vous avez besoin des copies."),
    },
    FileDecl {
        code: "rh",
        libelle: "Ressources humaines",
        role_traitant: "la gestion du personnel",
        type_: "confidentielle",
        service_calendrier: "secretariat",
        repli: Some("repli_rh"),
        membres: &[fonction("secretaire_direction", false, 1)],
        message_accueil: Some("Votre demande est confidentielle : seules les personnes habilitées la lisent."),
    },
];

pub fn file_decl(code: &str) -> Option<&'static FileDecl> {
    FILES_DEFAUT.iter().find(|f| f.code == code)
}

/// Réponses types de départ (texte avec variables typées `{{prenom}}`, `{{numero}}`, `{{date}}`, `{{service}}`).
pub const REPONSES_TYPES_DEFAUT: &[(&str, &str, &str)] = &[
    (
        "secretariat_scolarite",
        "Attestation disponible",
        "Bonjour {{prenom}}, votre attestation ({{numero}}) est disponible dans vos documents.",
    ),
    (
        "maintenance",
        "Intervention planifiée",
        "Bonjour {{prenom}}, une intervention est prévue pour votre demande {{numero}} le {{date}}.",
    ),
];

// ───────────────────────── Assemblage (§8.5) ─────────────────────────

/// Ce que vérifie l'assemblage du catalogue complet : renvoie la première règle violée.
pub fn verifier_catalogue() -> Result<(), String> {
    use std::collections::BTreeSet;
    let mut codes = BTreeSet::new();
    for n in toutes_les_natures() {
        crate::demandes::verifier_nature(n).map_err(|e| format!("{} : {e}", n.code))?;
        if !codes.insert(n.code) {
            return Err(format!("code « {} » en double", n.code));
        }
        let m = meta(n.code).ok_or_else(|| format!("{} : aucune méta (propriétaire, avis, tâches)", n.code))?;
        if m.proprietaire.is_empty() {
            return Err(format!("{} : propriétaire absent", n.code));
        }
        // Natures d'avis et de tâches déclarées, et connues.
        if m.avis.is_empty() || m.avis.iter().any(|a| !AVIS_DECLARES.contains(a)) {
            return Err(format!("{} : natures d'avis absentes ou inconnues", n.code));
        }
        if m.taches.is_empty() || m.taches.iter().any(|t| !TACHES_DECLAREES.contains(t)) {
            return Err(format!("{} : natures de tâche absentes ou inconnues", n.code));
        }
        // La catégorie de routage est un champ liste de la nature.
        if let Some(cc) = m.categorie_champ {
            match n.champs.iter().find(|c| c.code == cc) {
                Some(ChampDecl { type_: ChampType::Liste(_), .. }) => {}
                _ => return Err(format!("{} : catégorie de routage « {cc} » n'est pas un champ liste", n.code)),
            }
        }
        // Routage par défaut résolu vers une file ET une file de repli.
        let f = file_decl(n.file_defaut).ok_or_else(|| format!("{} : file « {} » inconnue", n.code, n.file_defaut))?;
        if f.repli.is_none() && f.type_ != "repli" {
            return Err(format!("{} : file « {} » sans repli", n.code, f.code));
        }
        // Une nature C3 ne se route jamais vers une file non confidentielle ; C2+ non plus.
        let confidentielle = matches!(n.confidentialite, Confidentialite::C2 | Confidentialite::C3);
        if confidentielle && !matches!(f.type_, "confidentielle" | "direction") {
            return Err(format!("{} : nature C2+ routée vers une file non confidentielle", n.code));
        }
    }
    // Les raccourcis : lien profond interne, aucun champ, codes distincts des natures (R-DM1).
    for r in RACCOURCIS {
        if !codes.insert(r.code) {
            return Err(format!("raccourci « {} » en double avec une nature", r.code));
        }
        if !r.lien.starts_with('/') || r.lien.starts_with("//") || r.lien.contains("://") {
            return Err(format!("raccourci « {} » : lien profond non interne", r.code));
        }
        if r.libelle.trim().is_empty() || r.mots_cles.is_empty() || r.publics.is_empty() {
            return Err(format!("raccourci « {} » incomplet", r.code));
        }
    }
    // Les files : repli connu, ordre de création (le repli précède), pas de cycle.
    let mut vues: Vec<&str> = Vec::new();
    for f in FILES_DEFAUT {
        if let Some(r) = f.repli {
            if !vues.contains(&r) {
                return Err(format!("file « {} » : repli « {r} » absent ou créé après", f.code));
            }
            let cible = file_decl(r).ok_or("repli inconnu")?;
            if matches!(f.type_, "confidentielle" | "direction") && cible.type_ != "repli" {
                return Err(format!("file « {} » : le repli d'une file confidentielle ou direction est hors service", f.code));
            }
        } else if f.type_ != "repli" {
            return Err(format!("file « {} » sans repli", f.code));
        }
        if f.membres.is_empty() {
            return Err(format!("file « {} » sans règle de membres", f.code));
        }
        vues.push(f.code);
    }
    if !FILES_DEFAUT.iter().any(|f| f.type_ == "tri") {
        return Err("aucune file de tri".into());
    }
    Ok(())
}
