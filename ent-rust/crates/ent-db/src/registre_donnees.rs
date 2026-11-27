//! **Registre de classement des tables** (D-037, exigences E1, E9, E12 de
//! `docs/PRA_SAUVEGARDES.md` §18.4).
//!
//! Source unique, versionnée avec le code, qui classe **chaque** table de la
//! base : portée, colonne d'établissement, données personnelles, santé,
//! fichiers associés, export de portabilité, neutralisation à la restauration,
//! rétention. Il sert trois usages :
//!
//! 1. **garde-fou CI** : un test échoue si une table du schéma (après toutes les
//!    migrations) n'est pas classée, si une entrée ne correspond plus à une
//!    table, ou si une table « par établissement » n'a pas
//!    `etablissement_id NOT NULL` ni de RLS — sauf **exception explicite et
//!    motivée** (jamais masquée) ;
//! 2. **extraction d'un établissement / portabilité** (PRA §11, à venir) ;
//! 3. **neutralisation des files d'envoi à la restauration** (E9, F-014) :
//!    [`neutraliser_files_restauration`].
//!
//! Ajouter une table = ajouter une ligne ici (le test le rappelle). Ce n'est
//! pas de la mémoire de l'application : aucune donnée métier, que du catalogue.
//!
//! État des exceptions : chaque exception encore présente est une **dette
//! déclarée** ; le test échoue aussi si une exception devient inutile (table
//! corrigée), pour que la liste ne mente jamais.

use sqlx::{PgConnection, Row};
use uuid::Uuid;

/// Portée d'une table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Portee {
    /// Donnée d'un établissement : `etablissement_id NOT NULL` + RLS attendus.
    Etablissement,
    /// Référentiel commun à tous les établissements (jamais réécrit par une
    /// restauration d'établissement).
    ReferentielGlobal,
    /// Identité partagée entre établissements (D-013) : jamais de donnée métier.
    CompteMultiEtablissement,
    /// État technique ou de plateforme (sessions, verrous, racine des
    /// établissements…).
    Technique,
}

/// Fichiers associés à la table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fichiers {
    Aucun,
    /// Référence un document (objet) stocké ailleurs (`piece_document_id`, …).
    Reference,
    /// Porte une clé d'objet (`storage_key`) : fichier hors base.
    Objet,
    /// Contenu binaire **en base** (`BYTEA`) : à sortir (E5).
    Bytea,
    /// Fichier **hors base** (`storage_key` + empreinte) ; la colonne `BYTEA` historique existe
    /// encore mais n'est plus écrite par l'application : elle ne garde d'octets que pour les
    /// pièces non encore reprises (`reprise_bytea`). Disparaît avec l'étape 4 (suppression de la
    /// colonne) : la table passe alors en `Objet`.
    ObjetAvecResidu,
}

/// Rétention.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retention {
    /// À définir (D-037 : durées de conservation à décider avec la direction / DPO).
    ADefinir,
    /// État éphémère : jamais restauré, jamais conservé.
    Ephemere,
}

/// Nature d'une exception déclarée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exception {
    /// `etablissement_id` absent ou nullable sur une table « par établissement ».
    EtabNullable = 0,
    /// Pas de RLS sur une table « par établissement ».
    SansRls = 1,
    /// Aucune colonne d'horodatage (E6).
    SansHorodatage = 2,
}

/// Neutralisation à la restauration (E9) : ce qu'il faut faire d'une file ou
/// d'un effet de bord en attente sur une base restaurée. Le SQL est **fixe**
/// (littéraux de ce fichier) ; seul l'établissement éventuel est lié en paramètre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Neutralisation {
    /// Rien à faire.
    Aucune,
    /// Vider la table (état de sécurité éphémère : sessions, défis, codes).
    Purge,
    /// `UPDATE table SET <set> WHERE <ou>` : suspend les éléments en attente,
    /// sans les supprimer (rejouables par un humain).
    Maj { set: &'static str, ou: &'static str },
    /// Classée « effet de bord » mais sans état à suspendre : la raison dit pourquoi.
    JournalSeul(&'static str),
    /// Ne modifie rien mais **compte** les lignes à revoir à la main (`ou`).
    Verification { ou: &'static str, pourquoi: &'static str },
}

/// Classement d'une table.
#[derive(Clone, Copy, Debug)]
pub struct ClasseTable {
    pub nom: &'static str,
    pub domaine: &'static str,
    pub portee: Portee,
    /// La table porte une colonne `etablissement_id`.
    pub colonne_etablissement: bool,
    pub donnees_personnelles: bool,
    pub sante: bool,
    pub fichiers: Fichiers,
    /// Incluse dans l'export de portabilité (RGPD art. 20) de la personne.
    pub export_portabilite: bool,
    /// Colonnes que l'export de la personne ne rend JAMAIS (motif écrit, demandeur… : elles révèlent un tiers ou une
    /// décision interne). Vide pour la plupart des tables ; l'assemblage de l'extrait les retire (lot G3).
    pub colonnes_hors_export: &'static [&'static str],
    /// Condition SQL (alias `x`) que doit remplir une ligne pour entrer dans l'export de la personne : ce qui n'est pas
    /// encore **publié** à la famille n'est jamais exporté (décision prise mais non publiée, suivi qui la révèle).
    /// L'assemblage de l'extrait (lot G3) l'applique ; vide pour la plupart des tables.
    pub export_si: Option<&'static str>,
    /// Raison écrite pour laquelle la table est exportable **sans** `export_si` ni `hors_export` (« tout exportable » :
    /// la famille ou l'élève a déjà reçu ou rédigé chaque ligne et chaque colonne). Exigée pour toute table `eva_*`
    /// exportée sans autre déclaration (test `toute_table_eva_exportee_est_declaree`).
    pub export_integral: Option<&'static str>,
    /// Table exportable pour la personne (art. 20) mais **absente de l'extrait par élève** : elle ne se rattache à aucun
    /// élève (registre de l'établissement, journal du personnel). Raison écrite exigée ; la base doit confirmer qu'elle
    /// n'a pas de colonne `eleve_id` (test `hors_export_par_eleve_est_justifie_et_sans_eleve`). L'assemblage de l'extrait
    /// (lot G3) saute ces tables ; aucune autre table exportée ne peut être sautée.
    pub hors_export_par_eleve: Option<&'static str>,
    pub neutralisation: Neutralisation,
    pub retention: Retention,
    exceptions: [Option<&'static str>; 3],
}

impl ClasseTable {
    /// Raison de l'exception de ce type, si elle est déclarée.
    pub fn exception(&self, e: Exception) -> Option<&'static str> {
        self.exceptions[e as usize]
    }

    /// Exceptions déclarées (type, raison).
    pub fn exceptions(&self) -> impl Iterator<Item = (Exception, &'static str)> + '_ {
        [Exception::EtabNullable, Exception::SansRls, Exception::SansHorodatage]
            .into_iter()
            .filter_map(|e| self.exception(e).map(|r| (e, r)))
    }

    /// La table est une file / un effet de bord à traiter à la restauration.
    pub fn est_file_envoi(&self) -> bool {
        !matches!(self.neutralisation, Neutralisation::Aucune)
    }
}

const fn t(nom: &'static str, domaine: &'static str, portee: Portee) -> ClasseTable {
    ClasseTable {
        nom,
        domaine,
        portee,
        colonne_etablissement: matches!(portee, Portee::Etablissement),
        donnees_personnelles: false,
        sante: false,
        fichiers: Fichiers::Aucun,
        export_portabilite: false,
        colonnes_hors_export: &[],
        export_si: None,
        export_integral: None,
        hors_export_par_eleve: None,
        neutralisation: Neutralisation::Aucune,
        retention: Retention::ADefinir,
        exceptions: [None; 3],
    }
}

impl ClasseTable {
    const fn perso(mut self) -> Self {
        self.donnees_personnelles = true;
        self
    }
    const fn sante(mut self) -> Self {
        self.sante = true;
        self.donnees_personnelles = true;
        self
    }
    const fn fichiers(mut self, f: Fichiers) -> Self {
        self.fichiers = f;
        self
    }
    const fn export(mut self) -> Self {
        self.export_portabilite = true;
        self
    }
    /// Colonnes retirées de l'export de portabilité de la personne (voir [`ClasseTable::colonnes_hors_export`]).
    const fn hors_export(mut self, colonnes: &'static [&'static str]) -> Self {
        self.colonnes_hors_export = colonnes;
        self
    }
    /// Condition de ligne de l'export de portabilité (voir [`ClasseTable::export_si`]).
    const fn export_si(mut self, condition: &'static str) -> Self {
        self.export_si = Some(condition);
        self
    }
    /// « Tout exportable », avec la raison écrite (voir [`ClasseTable::export_integral`]).
    const fn tout_exportable(mut self, raison: &'static str) -> Self {
        self.export_integral = Some(raison);
        self
    }
    /// Hors de l'extrait par élève, avec la raison écrite (voir [`ClasseTable::hors_export_par_eleve`]).
    const fn hors_export_par_eleve(mut self, raison: &'static str) -> Self {
        self.hors_export_par_eleve = Some(raison);
        self
    }
    /// Table hors « par établissement » qui porte tout de même `etablissement_id`.
    const fn avec_etab(mut self) -> Self {
        self.colonne_etablissement = true;
        self
    }
    const fn neutralise(mut self, n: Neutralisation) -> Self {
        if matches!(n, Neutralisation::Purge) {
            self.retention = Retention::Ephemere;
        }
        self.neutralisation = n;
        self
    }
    const fn exc(mut self, e: Exception, raison: &'static str) -> Self {
        self.exceptions[e as usize] = Some(raison);
        self
    }
}

const N_PURGE: Neutralisation = Neutralisation::Purge;
/// Échéances de demandes (N1) : une base restaurée ne doit déclencher ni relance ni escalade ; elles sont
/// suspendues (jamais supprimées) et un humain les reprend (R-DM20 : une échéance suspendue ne court plus).
const N_DEM_ECHEANCE: Neutralisation = Neutralisation::Maj {
    set: "suspendue_depuis = now()",
    ou: "traitee_le IS NULL AND suspendue_depuis IS NULL",
};
/// Réservations de quota des espaces (socle 32, ET0) : les envois annoncés mais ni confirmés ni libérés sont libérés à la
/// restauration (le compteur d'occupation est ensuite recalculé depuis les versions et les réservations ouvertes).
const N_ESPACE_RESERVATION: Neutralisation = Neutralisation::Maj {
    set: "liberee_le = now(), liberee_motif = 'restauration'",
    ou: "confirmee_le IS NULL AND liberee_le IS NULL",
};
const N_PUSH_DEVICE: Neutralisation = Neutralisation::Maj {
    set: "revoked_at = now(), revoque_motif = 'restauration'",
    ou: "revoked_at IS NULL",
};
const N_OUTBOX: Neutralisation = Neutralisation::Maj {
    set: "statut = 'echec', derniere_erreur = 'neutralise_restauration'",
    ou: "statut = 'a_distribuer'",
};
const N_NOTIF: Neutralisation = Neutralisation::Maj {
    set: "statut = 'ignore', motif = 'neutralise_restauration'",
    ou: "statut IN ('en_attente', 'envoi')",
};
const N_NTF_AVIS: Neutralisation = Neutralisation::Maj {
    set: "statut = 'annule', annule_le = now(), annule_motif = 'neutralise_restauration'",
    ou: "statut IN ('prepare', 'planifie')",
};
const N_NTF_REMISE: Neutralisation = Neutralisation::Maj {
    set: "statut = 'annulee', motif = 'neutralise_restauration'",
    ou: "statut IN ('planifiee', 'reservee')",
};
/// Envois aux prestataires (demandes, relances, bons de commande, avenants) encore en file : à ne
/// jamais laisser partir après une restauration (un bon de commande ne doit pas partir deux fois).
const N_ENVOI_PRESTATAIRE: Neutralisation = Neutralisation::Maj {
    set: "statut = 'annule', derniere_erreur = 'neutralise_restauration'",
    ou: "statut = 'a_envoyer'",
};
/// Avis de restauration (commande, avenants) encore en file : à ne jamais laisser partir après une restauration.
const N_REPAS_AVIS: Neutralisation = Neutralisation::Maj {
    set: "statut = 'echec'",
    ou: "statut IN ('a_envoyer', 'sans_destinataire')",
};
/// Adresses de réponse à usage unique (D-065) : une base restaurée ne doit pas laisser vivre des clés émises
/// avant la restauration (elles pourraient viser un dossier qui n'est plus le même).
const N_REPONSE_CLE: Neutralisation = Neutralisation::Maj {
    set: "revoquee_le = now()",
    ou: "revoquee_le IS NULL",
};
const N_INSENVOI: Neutralisation = Neutralisation::Maj {
    set: "statut = 'annule', derniere_erreur = 'neutralise_restauration'",
    ou: "statut = 'a_envoyer'",
};
const N_INSINV: Neutralisation = Neutralisation::Maj {
    set: "statut = 'annulee'",
    ou: "statut IN ('a_envoyer', 'non_envoyee', 'envoyee')",
};
const N_AGINV: Neutralisation = Neutralisation::Maj {
    set: "annulee_le = now()",
    ou: "utilisee_le IS NULL AND annulee_le IS NULL",
};
const N_ELINV: Neutralisation = Neutralisation::Maj {
    set: "statut = 'revoquee'",
    ou: "statut = 'provisionne' AND activated_user_id IS NULL",
};
const N_COORD: Neutralisation = Neutralisation::Maj {
    set: "statut = 'annulee'",
    ou: "statut = 'envoye'",
};
/// Rappels de rendez-vous : `rappel_envoye_le IS NULL` déclenche un envoi ; une
/// base restaurée « oublierait » les rappels partis après la sauvegarde.
const N_RDV: Neutralisation = Neutralisation::Maj {
    set: "rappel_envoye_le = now()",
    ou: "statut = 'reserve' AND rappel_envoye_le IS NULL",
};
const N_PRENOT: Neutralisation = Neutralisation::Verification {
    ou: "date_execution >= current_date",
    pourquoi: "pré-notifications SEPA à échéance future : `envoyee_le` est posée à la génération, sans file d'envoi ; vérifier ce que les familles ont réellement reçu avant toute nouvelle génération",
};
const N_SEPA: Neutralisation = Neutralisation::Verification {
    ou: "statut IN ('prevu', 'exporte')",
    pourquoi: "prélèvements non exécutés : ne jamais exporter vers la banque sans revue humaine après une restauration (double prélèvement)",
};

/// Index des tâches du socle 29 : les tâches ouvertes sont closes « restauration » ; le rapprochement
/// les rouvre si la source les dit encore ouvertes (rien n'est jamais rejoué en silence).
const N_AFAIRE: Neutralisation = Neutralisation::Maj {
    set: "etat = 'sans_objet', issue = 'restauration', close_le = now()",
    ou: "etat IN ('ouverte', 'prise', 'en_attente')",
};

/// Le registre. Ordre alphabétique (aligné sur le catalogue).
pub const REGISTRE: &[ClasseTable] = &[
    // Socle 27, G1 : règles de conservation décidées par l'établissement ; le catalogue est dans le code.
    t("cons_regle", "conformite", Portee::Etablissement).perso(),
    t("cons_regle_journal", "conformite", Portee::Etablissement).perso(),
    t("ent_absence", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_absence_declaration", "assiduite", Portee::Etablissement).perso().sante().fichiers(Fichiers::Reference).export(),
    t("ent_absence_motif", "assiduite", Portee::ReferentielGlobal).exc(Exception::SansHorodatage, "référentiel statique (code, libellé), jamais modifié par un établissement"),
    t("ent_absence_motif_etab", "assiduite", Portee::Etablissement),
    t("ent_absence_rapprochement", "assiduite", Portee::Etablissement).perso().export(),
    // AR-03 (socle 15) : le motif d'une absence de personnel peut être médical ; la table sort du domaine
    // « assiduité » des élèves et porte la marque santé.
    t("ent_absence_rh", "personnels_absences", Portee::Etablissement).sante().export(),
    t("ent_absence_seance", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_absence_signal", "assiduite", Portee::Etablissement).perso().export(),
    // Socle 15 (absences des personnels, lots A0–A1) : la nature d'une absence est une donnée de santé
    // ou d'opinion (art. 9) ; seules ces tables la portent.
    t("ent_absp_absence", "personnels_absences", Portee::Etablissement).sante().export(),
    t("ent_absp_arret", "personnels_absences", Portee::Etablissement).sante().export(),
    t("ent_absp_circuit", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_circuit_regle", "personnels_absences", Portee::Etablissement),
    t("ent_absp_decision", "personnels_absences", Portee::Etablissement).sante().export(),
    t("ent_absp_decision_externe", "personnels_absences", Portee::Etablissement).sante().export(),
    t("ent_absp_demande_suppleance", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_dsn_suivi", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_element_paie", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_etat", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_etat_ligne", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_gel", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_heure", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_heure_fait", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_impact", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_journal", "personnels_absences", Portee::Etablissement).sante().export(),
    t("ent_absp_message", "personnels_absences", Portee::Etablissement).sante().export(),
    t("ent_absp_nature", "personnels_absences", Portee::Etablissement).exc(Exception::EtabNullable, "natures fournies par ScolIA (etablissement_id NULL autorisé, aucune donnée de personne), propres à l'établissement sinon"),
    t("ent_absp_nature_etab", "personnels_absences", Portee::Etablissement),
    t("ent_absp_param", "personnels_absences", Portee::Etablissement),
    t("ent_absp_piece", "personnels_absences", Portee::Etablissement).sante().export(),
    t("ent_absp_piece_lecture", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_plage", "personnels_absences", Portee::Etablissement).sante().export(),
    t("ent_absp_rattrapage", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_rattrapage_creneau", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_rcd_compte_rendu", "personnels_absences", Portee::Etablissement).export(),
    t("ent_absp_rcd_engagement", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_rcd_plan", "personnels_absences", Portee::Etablissement).export(),
    t("ent_absp_rcd_remontee", "personnels_absences", Portee::Etablissement).export(),
    t("ent_absp_rcd_remontee_ligne", "personnels_absences", Portee::Etablissement).export(),
    t("ent_absp_remplacement", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_reponse", "personnels_absences", Portee::Etablissement).perso().export(),
    t("ent_absp_sollicitation", "personnels_absences", Portee::Etablissement).perso().export(),
    // Socle 18 (accords, autorisations, consentements) : catalogue de référence, objets, versions, décisions en ajout seul.
    t("ent_acc_catalogue", "dossier_eleve", Portee::ReferentielGlobal).exc(Exception::SansHorodatage, "catalogue de référence fourni par ScolIA (R-FC6), évolue par migration, jamais modifié par un établissement"),
    t("ent_acc_catalogue_usage", "dossier_eleve", Portee::ReferentielGlobal).exc(Exception::SansHorodatage, "usages du catalogue de référence, statiques"),
    t("ent_acc_decision", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_acc_decision_usage", "dossier_eleve", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille de la décision (clé composite) : datée par la décision, en ajout seul"),
    t("ent_acc_desaccord", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_acc_document_lien", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_acc_journal", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_acc_objet", "dossier_eleve", Portee::Etablissement),
    t("ent_acc_objet_source", "dossier_eleve", Portee::Etablissement),
    t("ent_acc_preuve", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_acc_usage", "dossier_eleve", Portee::Etablissement),
    t("ent_acc_version", "dossier_eleve", Portee::Etablissement),
    t("ent_acc_version_usage", "dossier_eleve", Portee::Etablissement).exc(Exception::SansHorodatage, "table de liaison (version ↔ usage), datée par la version, figée à la publication"),
    t("ent_adresse_protegee", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_aesh_affectation", "sante", Portee::Etablissement).perso().sante().export(),
    // Socle 29 (lot P2) : l'index des tâches est une PROJECTION reconstructible (§8.10, R-PO24) :
    // neutralisé à la restauration (les tâches ouvertes passent « sans objet / restauration »), puis
    // rebâti par le rapprochement, qui rouvre celles que leur propriétaire dit encore ouvertes.
    t("ent_afaire", "portails", Portee::Etablissement).perso().neutralise(N_AFAIRE),
    t("ent_afaire_cle", "portails", Portee::Etablissement),
    t("ent_afaire_destinataire", "portails", Portee::Etablissement).perso(),
    t("ent_afaire_etat_personnel", "portails", Portee::Etablissement).perso(),
    t("ent_afaire_journal", "portails", Portee::Etablissement).perso(),
    t("ent_afaire_nature_reglage", "portails", Portee::Etablissement),
    t("ent_afaire_prise", "portails", Portee::Etablissement).perso(),
    t("ent_affectation", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_affectation_supprimee", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_agenda_audience", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_agenda_coupon", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_agenda_coupon_reponse", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_agenda_evenement", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_agenda_journal", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_agenda_lieu_regle", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_agenda_participant", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_agenda_reservation", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_agenda_reservation_fait", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_agenda_serie", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_agenda_serie_jour", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_agenda_type_local", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_agent", "personnels", Portee::Etablissement).perso().export(),
    t("ent_agent_contact_urgence", "personnels", Portee::Etablissement).perso().export(),
    t("ent_agent_discipline", "personnels", Portee::Etablissement).perso().export(),
    t("ent_agent_engagement", "personnels", Portee::Etablissement).perso().export(),
    t("ent_agent_engagement_suspension", "personnels", Portee::Etablissement).perso().export(),
    t("ent_agent_fonction", "personnels", Portee::Etablissement).perso().export(),
    t("ent_agent_identifiant", "personnels", Portee::Etablissement).perso().export(),
    t("ent_agent_invitation", "personnels", Portee::Etablissement).perso().neutralise(N_AGINV),
    t("ent_agent_qualification", "personnels", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_alerte_cpe", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_amenagement", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_annee_scolaire", "etablissement", Portee::Etablissement),
    t("ent_annonce", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_appel", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_appel_delegation", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_appel_ligne", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_appel_ligne_histo", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_appel_relance", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_absence", "assiduite", Portee::Etablissement).perso().sante().export(),
    t("ent_asd_absence_declaration", "assiduite", Portee::Etablissement).perso(),
    t("ent_asd_absence_etat", "assiduite", Portee::Etablissement).perso().sante(),
    t("ent_asd_absence_seance", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_activite_appelee", "assiduite", Portee::Etablissement),
    t("ent_asd_alerte", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_appel_substitution", "assiduite", Portee::Etablissement).perso(),
    t("ent_asd_avis_demi_journee", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_contact", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_courrier", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_demande", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_demi_journee", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_justificatif", "assiduite", Portee::Etablissement).perso().sante().fichiers(Fichiers::Objet).export(),
    t("ent_asd_justificatif_lecture", "assiduite", Portee::Etablissement).perso(),
    t("ent_asd_mois_arrete", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_mois_cloture", "assiduite", Portee::Etablissement),
    t("ent_asd_motif", "assiduite", Portee::Etablissement),
    t("ent_asd_motif_version", "assiduite", Portee::Etablissement),
    t("ent_asd_prevenir", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_proposition", "assiduite", Portee::Etablissement).perso(),
    t("ent_asd_rectification", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_regle", "assiduite", Portee::Etablissement),
    t("ent_asd_relance", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_asd_situation", "assiduite", Portee::Etablissement).perso().sante().export(),
    t("ent_asd_situation_texte", "assiduite", Portee::Etablissement).perso().sante().export(),
    t("ent_asd_situation_version", "assiduite", Portee::Etablissement).perso(),
    t("ent_asd_tableau_pref", "assiduite", Portee::Etablissement).perso(),
    t("ent_asd_tableau_reglage", "assiduite", Portee::Etablissement),
    t("ent_asd_vigilance", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_attribution_cible", "personnels", Portee::Etablissement).perso().export(),
    t("ent_bank_rapprochement", "finances", Portee::Etablissement).perso().export(),
    t("ent_batiment", "etablissement", Portee::Etablissement),
    t("ent_bilan_cycle", "evaluations_pedagogie", Portee::Etablissement),
    t("ent_bilan_cycle_composante", "evaluations_pedagogie", Portee::Etablissement).exc(Exception::SansHorodatage, "ligne fille : datée par ent_bilan_cycle"),
    t("ent_bourse_type", "finances", Portee::Etablissement),
    t("ent_bulletin", "evaluations_pedagogie", Portee::Etablissement).perso().fichiers(Fichiers::Objet).export(),
    t("ent_cahier_texte", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_calendrier_cible", "emploi_du_temps_structure", Portee::Etablissement).exc(Exception::SansHorodatage, "table de liaison (calendrier ↔ cible), datée par le calendrier"),
    t("ent_calendrier_evenement", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_campagne", "communication_technique", Portee::Etablissement).neutralise(Neutralisation::JournalSeul("une campagne lancée a déjà créé ses envois dans ent_notification_envoi (neutralisés)")),
    t("ent_campagne_destinataire", "communication_technique", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par la campagne"),
    t("ent_carnet_entree", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_catalogue_indicateur", "dossier_eleve", Portee::ReferentielGlobal).exc(Exception::SansHorodatage, "catalogue du logiciel (pastilles de l'élève), égal à `domain_eleve::CATALOGUE_INDICATEURS`, évolue par migration, jamais modifié par un établissement"),
    t("ent_cdt_abonnement", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_acces_inspection", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_acces_inspection_cours", "cahier_textes", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par l'accès d'inspection"),
    t("ent_cdt_acces_inspection_historique", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_affichage", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_alerte_charge", "cahier_textes", Portee::Etablissement),
    t("ent_cdt_amenagement_simule", "cahier_textes", Portee::Etablissement).perso(),
    t("ent_cdt_archive", "cahier_textes", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_cdt_cloture", "cahier_textes", Portee::Etablissement).perso(),
    t("ent_cdt_coche", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_consentement_exemple", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_consultation", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_continuite", "cahier_textes", Portee::Etablissement).perso(),
    t("ent_cdt_continuite_cours", "cahier_textes", Portee::Etablissement).perso(),
    t("ent_cdt_contribution", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_contribution_version", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_copies_decision", "cahier_textes", Portee::Etablissement),
    t("ent_cdt_delai_demande", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_depot", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_depot_signalement", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_echeance_eleve", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_entree", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_entree_notion", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_entree_version", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_export", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_groupe", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_groupe_membre", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_historique", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_cdt_lien", "cahier_textes", Portee::Etablissement),
    t("ent_cdt_nature", "cahier_textes", Portee::Etablissement),
    t("ent_cdt_note_accompagnement", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_observation", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_observation_reponse", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_panne", "cahier_textes", Portee::Etablissement),
    t("ent_cdt_piece", "evaluations_pedagogie", Portee::Etablissement).perso().fichiers(Fichiers::Objet).export(),
    t("ent_cdt_piece_jointe", "cahier_textes", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_cdt_progression", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_progression_etape", "cahier_textes", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par la progression"),
    t("ent_cdt_progression_partage", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_quarantaine", "cahier_textes", Portee::Etablissement),
    t("ent_cdt_reglage", "cahier_textes", Portee::Etablissement),
    t("ent_cdt_reglage_format", "cahier_textes", Portee::Etablissement),
    t("ent_cdt_relance", "cahier_textes", Portee::Etablissement).perso(),
    t("ent_cdt_remise", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_report", "cahier_textes", Portee::Etablissement),
    t("ent_cdt_resume_envoi", "cahier_textes", Portee::Etablissement).perso().neutralise(Neutralisation::JournalSeul("trace des résumés hebdomadaires déjà envoyés (déduplication) ; l'envoi passe par ent_outbox / ent_notification_envoi (neutralisés)")),
    t("ent_cdt_retour", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_retour_critere", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_retour_version", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_signalement_copie", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_statut", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_travail", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_travail_cible", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_travail_format", "cahier_textes", Portee::Etablissement),
    t("ent_cdt_travail_version", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_variante", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_variante_eleve", "cahier_textes", Portee::Etablissement).perso().export(),
    t("ent_cdt_visa", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_cdt_visa_element", "cahier_textes", Portee::Etablissement),
    t("ent_classe", "structure_scolaire", Portee::Etablissement),
    t("ent_competence_evaluation", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_competence_item", "evaluations_pedagogie", Portee::Etablissement),
    t("ent_conflit_interet", "identite_acces", Portee::Etablissement).perso().export(),
    t("ent_conseil_classe", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_conseil_classe_dossier", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_contrainte_publiee", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_convention_stage", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_coord_reprise_marque", "dossier_personnes", Portee::Technique),
    t("ent_coordonnee_verification", "dossier_personnes", Portee::Etablissement).perso().neutralise(N_COORD),
    t("ent_courrier_entrant", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_courrier_piece", "sorties_voyages", Portee::Etablissement).perso().fichiers(Fichiers::Objet),
    t("ent_cours", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_cours_a_traiter", "emploi_du_temps_structure", Portee::Etablissement).perso(),
    t("ent_cours_membre", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_creneau_rdv", "emploi_du_temps_structure", Portee::Etablissement).perso().neutralise(N_RDV),
    t("ent_delegation", "identite_acces", Portee::Etablissement).perso().export(),
    t("ent_delegation_droit", "identite_acces", Portee::Etablissement).perso().export(),
    t("ent_delegation_droit_cible", "identite_acces", Portee::Etablissement).perso().export(),
    t("ent_dem_abonne", "demandes", Portee::Etablissement).perso().export(),
    t("ent_dem_consultation", "demandes", Portee::Etablissement).perso(),
    t("ent_dem_consultation_partage", "demandes", Portee::Etablissement),
    t("ent_dem_demande", "demandes", Portee::Etablissement).perso().export(),
    t("ent_dem_echeance", "demandes", Portee::Etablissement).neutralise(N_DEM_ECHEANCE),
    t("ent_dem_fait", "demandes", Portee::Etablissement).perso().export(),
    t("ent_dem_file", "demandes", Portee::Etablissement),
    t("ent_dem_file_membre_regle", "demandes", Portee::Etablissement).perso().export(),
    t("ent_dem_file_palier", "demandes", Portee::Etablissement),
    t("ent_dem_guichet", "demandes", Portee::Etablissement),
    t("ent_dem_guichet_journal", "demandes", Portee::Etablissement).perso(),
    t("ent_dem_hse", "demandes", Portee::Etablissement).perso().export(),
    t("ent_dem_hse_decision", "demandes", Portee::Etablissement).perso().export(),
    t("ent_dem_hse_enveloppe", "demandes", Portee::Etablissement),
    t("ent_dem_hse_enveloppe_mvt", "demandes", Portee::Etablissement).perso(),
    t("ent_dem_hse_export", "demandes", Portee::Etablissement).perso(),
    t("ent_dem_hse_public", "demandes", Portee::Etablissement),
    t("ent_dem_hse_reglage", "demandes", Portee::Etablissement),
    t("ent_dem_hse_seance", "demandes", Portee::Etablissement).perso().export(),
    t("ent_dem_lien", "demandes", Portee::Etablissement),
    t("ent_dem_message", "demandes", Portee::Etablissement).perso().export(),
    t("ent_dem_nature_etab", "demandes", Portee::Etablissement),
    t("ent_dem_nature_public", "demandes", Portee::Etablissement),
    t("ent_dem_numerotation", "demandes", Portee::Etablissement),
    t("ent_dem_personne_visee", "demandes", Portee::Etablissement).perso(),
    t("ent_dem_piece", "demandes", Portee::Etablissement).perso().export().fichiers(Fichiers::Reference),
    t("ent_dem_reglage_journal", "demandes", Portee::Etablissement).perso().export(),
    t("ent_dem_repli_rh", "demandes", Portee::Etablissement),
    t("ent_dem_reponse_type", "demandes", Portee::Etablissement),
    // Reprise du Next.js (N1-5) : classée « reprise », purgeable après la décision de Florian (E8, migration 3059).
    t("ent_dem_reprise_correspondance", "demandes", Portee::Etablissement),
    t("ent_dem_reprise_demandeur", "demandes", Portee::Etablissement).perso(),
    t("ent_dem_reprise_lot", "demandes", Portee::Etablissement),
    t("ent_dem_reprise_proposition", "demandes", Portee::Etablissement).perso(),
    t("ent_dem_repro", "demandes", Portee::Etablissement).perso().export(),
    t("ent_dem_repro_cible", "demandes", Portee::Etablissement),
    t("ent_dem_repro_document", "demandes", Portee::Etablissement).perso().export().fichiers(Fichiers::Reference),
    t("ent_dem_repro_lieu_retrait", "demandes", Portee::Etablissement),
    t("ent_dem_repro_oeuvre", "demandes", Portee::Etablissement),
    t("ent_dem_repro_receptionnaire", "demandes", Portee::Etablissement).perso(),
    t("ent_dem_repro_recurrence", "demandes", Portee::Etablissement).perso(),
    t("ent_dem_repro_site_reglage", "demandes", Portee::Etablissement),
    t("ent_dem_routage_regle", "demandes", Portee::Etablissement),
    t("ent_dem_valeur", "demandes", Portee::Etablissement).perso().export(),
    // Contre-revue du pilote (passe 6, M-B) : archive des devoirs de l'ancienne grille (classe, titre, date, auteur) : SANS
    // `eleve_id`, elle couvre tous les devoirs de l'établissement, publiés ou non. L'exporter livrerait à un élève les devoirs
    // des autres classes et des devoirs jamais publiés : hors export. Le devoir d'une note publiée est déjà rendu par
    // `ent_note_archive` (son libellé) et par l'évaluation reprise (`eva_evaluation`).
    t("ent_devoir_archive", "evaluations_pedagogie", Portee::Etablissement).perso(),
    t("ent_dispense_vs", "assiduite", Portee::Etablissement).perso().sante().export(),
    t("ent_disponibilite", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_doc_a_signer", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_doc_circuit", "documents", Portee::Etablissement).perso().export().fichiers(Fichiers::Objet),
    t("ent_doc_compteur", "documents", Portee::Etablissement),
    t("ent_doc_consultation", "documents", Portee::Etablissement).perso().export(),
    t("ent_doc_defi", "documents", Portee::Etablissement).perso(),
    t("ent_doc_document", "documents", Portee::Etablissement).perso().export(),
    t("ent_doc_entite", "documents", Portee::Etablissement),
    t("ent_doc_entite_famille", "documents", Portee::Etablissement),
    t("ent_doc_entite_identifiant", "documents", Portee::Etablissement),
    t("ent_doc_exemplaire", "documents", Portee::Etablissement).perso().export().fichiers(Fichiers::Objet),
    t("ent_doc_horodatage", "documents", Portee::Etablissement),
    t("ent_doc_horodatage_nature", "documents", Portee::Etablissement),
    t("ent_doc_horodatage_service", "documents", Portee::Etablissement),
    t("ent_doc_identite", "documents", Portee::Etablissement),
    t("ent_doc_journal", "documents", Portee::Etablissement).perso().export(),
    t("ent_doc_mise_a_disposition", "documents", Portee::Etablissement).perso().export(),
    t("ent_doc_nature_param", "documents", Portee::Etablissement),
    t("ent_doc_nature_texte", "documents", Portee::Etablissement),
    t("ent_doc_piece", "documents", Portee::Etablissement).perso().export().fichiers(Fichiers::Objet),
    t("ent_doc_piece_decision", "documents", Portee::Etablissement).perso().export(),
    t("ent_doc_piece_demande", "documents", Portee::Etablissement).perso().export(),
    t("ent_doc_signataire", "documents", Portee::Etablissement).perso().export().fichiers(Fichiers::Objet),
    t("ent_doc_signataire_mode", "documents", Portee::Etablissement),
    t("ent_doc_signature", "documents", Portee::Etablissement).perso().export().fichiers(Fichiers::Objet),
    t("ent_doc_signature_menu", "documents", Portee::Etablissement),
    t("ent_doc_version", "documents", Portee::Etablissement).perso().export().fichiers(Fichiers::Objet),
    t("ent_doc_version_donnee", "documents", Portee::Etablissement).perso().export(),
    t("ent_document_grant", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_edition_vs", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_edt_annulation_masse", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_besoin", "emploi_du_temps_structure", Portee::Etablissement).export(),
    t("ent_edt_calendrier_ab", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_edt_contrainte", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_edt_creneau", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_deplacement", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    // Archive datée des déplacements levés (A0, AR-15) : rien ne s'efface.
    t("ent_edt_deplacement_leve", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_exception", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    // Archive datée des exceptions levées ou remplacées (A0, AR-15) : rien ne s'efface.
    t("ent_edt_exception_levee", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_import", "emploi_du_temps_structure", Portee::Etablissement),
    // Socle 10, lot T5 : version individuelle d'un élève (l'effet est porté par `ent_cours_membre`).
    t("ent_edt_individuelle", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_individuelle_ligne", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    // Socle 10, lot T3 : journal du réel, lots et motifs internes (jamais exportés hors direction).
    // Socle 10, lot T4 : lectures de localisation d'un élève (qui, quand, pourquoi ; jamais le lieu).
    t("ent_edt_localisation_journal", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_lot", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_lot_cible", "emploi_du_temps_structure", Portee::Etablissement).export(),
    t("ent_edt_modification", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_modification_interne", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_piece", "emploi_du_temps_structure", Portee::Etablissement).export(),
    t("ent_edt_placement", "emploi_du_temps_structure", Portee::Etablissement).export(),
    t("ent_edt_placement_lieu", "emploi_du_temps_structure", Portee::Etablissement).export(),
    t("ent_edt_seance", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_seance_intervenant", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_seance_lieu", "emploi_du_temps_structure", Portee::Etablissement).export(),
    t("ent_edt_seance_public_fusion", "emploi_du_temps_structure", Portee::Etablissement).export(),
    t("ent_edt_version", "emploi_du_temps_structure", Portee::Etablissement).export(),
    // Socle 10, lot T5 : historique des faits de publication.
    t("ent_edt_version_journal", "emploi_du_temps_structure", Portee::Etablissement).export(),
    t("ent_edt_version_perimetre", "emploi_du_temps_structure", Portee::Etablissement).export(),
    t("ent_eleve", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_bourse", "finances", Portee::Etablissement).perso().export(),
    t("ent_eleve_capacite", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_eleve_compte", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_confidentialite", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_contact_urgence", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_coordonnee", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_coordonnee_verification", "dossier_eleve", Portee::Etablissement).perso().neutralise(N_COORD),
    t("ent_eleve_dispense_repas", "vie_pratique", Portee::Etablissement).perso().sante().export(),
    t("ent_eleve_dispositif", "dossier_eleve", Portee::Etablissement).perso().sante().export(),
    t("ent_eleve_document", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Objet).export(),
    t("ent_eleve_doublon", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_doublon_passage", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_enseignement", "structure_scolaire", Portee::Etablissement).perso().export(),
    t("ent_eleve_enseignement_a_traiter", "structure_scolaire", Portee::Etablissement).perso(),
    t("ent_eleve_enseignement_histo", "structure_scolaire", Portee::Etablissement).perso(),
    t("ent_eleve_enseignement_reprise", "structure_scolaire", Portee::Etablissement).perso(),
    t("ent_eleve_fusion", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_fusion_conflit", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_fusion_conflit_cle", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_fusion_doublon", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_fusion_ligne", "dossier_eleve", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par ent_eleve_fusion"),
    t("ent_eleve_fusion_ligne_cle", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_grille_repas", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_eleve_identifiant", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_identite", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_eleve_identite_a_traiter", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_eleve_identite_scellee", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_eleve_indicateur", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_eleve_inscription", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_inscription_a_traiter", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_eleve_interdiction", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_eleve_invitation", "dossier_eleve", Portee::Etablissement).perso().neutralise(N_ELINV),
    t("ent_eleve_lien", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_majorite_evenement", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_mobilite", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_modalite", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_eleve_non_doublon", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_numero_compteur", "dossier_eleve", Portee::Etablissement).exc(Exception::SansHorodatage, "compteur technique (numéro d'élève), valeur seule"),
    t("ent_eleve_option", "structure_scolaire", Portee::Etablissement).perso().export(),
    t("ent_eleve_option_a_traiter", "structure_scolaire", Portee::Etablissement).perso(),
    t("ent_eleve_option_annee", "structure_scolaire", Portee::Etablissement),
    t("ent_eleve_option_groupe", "structure_scolaire", Portee::Etablissement).perso().export(),
    t("ent_eleve_orphelin", "dossier_eleve", Portee::Etablissement).perso().exc(Exception::EtabNullable, "orphelins détectés dont l'établissement peut être inconnu"),
    t("ent_eleve_periode", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_photo", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Objet).export(),
    t("ent_eleve_photo_refus", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_proposition", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_eleve_proposition_champ", "dossier_eleve", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par la proposition"),
    t("ent_eleve_scolarisation", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_scolarite_annee", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_scolarite_ecart", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_email_change", "identite_acces", Portee::Technique).perso().neutralise(N_PURGE).avec_etab(),
    t("ent_entreprise", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_entreprise_contact", "stages", Portee::Etablissement).perso().export(),
    t("ent_entreprise_site", "stages", Portee::Etablissement),
    t("ent_equipe", "personnels", Portee::Etablissement).perso().export(),
    t("ent_equipe_membre", "personnels", Portee::Etablissement).perso().export(),
    t("ent_equipe_reunion", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_equipe_type", "personnels", Portee::Etablissement).exc(Exception::EtabNullable, "types d'équipes fournis par ScolIA (etablissement_id NULL autorisé), types propres par établissement"),
    t("ent_espace", "espaces", Portee::Etablissement).perso().export(),
    t("ent_espace_ancrage", "espaces", Portee::Etablissement).perso().export(),
    // Dérivés (aperçus, vignettes, texte extrait) : RECONSTRUCTIBLES (E12), régénérés après restauration ; jamais exportés.
    t("ent_espace_derive", "espaces", Portee::Etablissement).perso().fichiers(Fichiers::Reference),
    t("ent_espace_dossier", "espaces", Portee::Etablissement).perso().export(),
    t("ent_espace_element", "espaces", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    // Intention d'un envoi direct (nom, dossier, élément visé) attachée à sa réservation de quota : l'effet de bord (l'envoi) est neutralisé
    // par la table de réservation ; ici il n'y a aucun état à suspendre.
    t("ent_espace_envoi", "espaces", Portee::Etablissement).perso().neutralise(Neutralisation::JournalSeul("intention d'envoi : l'état à suspendre est celui de ent_espace_reservation (neutralisée à la restauration)")),
    t("ent_espace_journal", "espaces", Portee::Etablissement).perso().export(),
    t("ent_espace_membre_ajoute", "espaces", Portee::Etablissement).perso().export(),
    t("ent_espace_membre_exclu", "espaces", Portee::Etablissement).perso().export(),
    t("ent_espace_occupation", "espaces", Portee::Etablissement),
    t("ent_espace_partage", "espaces", Portee::Etablissement).perso().export(),
    t("ent_espace_quota_profil", "espaces", Portee::Etablissement),
    // Réservations de quota : état VOLATIL (E9) — une base restaurée ne doit garder aucun envoi « annoncé » ; elles sont libérées, jamais
    // supprimées, et les compteurs d'occupation sont recalculés par `espaces_reconcilier_occupation`.
    t("ent_espace_reservation", "espaces", Portee::Etablissement).perso().fichiers(Fichiers::Reference).neutralise(N_ESPACE_RESERVATION),
    t("ent_espace_role_calcule", "espaces", Portee::Etablissement),
    t("ent_espace_version", "espaces", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_espace_version_figee", "espaces", Portee::Etablissement),
    t("ent_etablissement", "etablissement", Portee::Technique),
    t("ent_etablissement_domaine", "etablissement", Portee::Etablissement).exc(Exception::SansRls, "résolution hôte → établissement (D-012) : lue avant de connaître l'établissement"),
    t("ent_etablissement_jour_ouvre", "etablissement", Portee::Etablissement),
    t("ent_etablissement_parametre", "etablissement", Portee::Etablissement),
    t("ent_etablissement_profil", "etablissement", Portee::Etablissement),
    t("ent_eva_coef_matiere", "evaluations_pedagogie", Portee::Etablissement),
    // Journal des exports du personnel (format, classe, période, effectif, empreinte, auteur de l'export) : aucune note ni
    // donnée d'élève ; l'auteur de l'export est la personne exportée elle-même.
    // Suites V5 passe 6 (Faible 8) : exportable pour le membre du personnel qui a exporté, mais hors de l'extrait PAR ÉLÈVE.
    t("ent_eva_export", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .tout_exportable("journal des exports faits par la personne elle-même : aucune note, aucun élève nommé")
        .hors_export_par_eleve("journal du personnel (format, classe, effectif, empreinte, auteur de l'export) : aucun élève n'y est rattaché, l'extrait d'un élève ne le contient jamais"),
    t("ent_eva_regle", "evaluations_pedagogie", Portee::Etablissement),
    t("ent_event_traitement", "communication_technique", Portee::Etablissement).neutralise(Neutralisation::JournalSeul("marqueurs « déjà traité » des consommateurs d'événements : à conserver pour ne pas rejouer")).exc(Exception::SansRls, "marqueurs d'idempotence relus par `rejouer_consommateur` en dehors d'un contexte d'établissement, avec ent_outbox"),
    t("ent_fac_echeance", "finances", Portee::Etablissement).perso().export(),
    t("ent_fac_encaissement", "finances", Portee::Etablissement).perso().export(),
    t("ent_fac_facture", "finances", Portee::Etablissement).perso().export(),
    t("ent_fac_ligne", "finances", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne de facture immuable : datée par la facture (période en dates)"),
    t("ent_fac_quittance", "finances", Portee::Etablissement).perso().export(),
    t("ent_fac_rappel", "finances", Portee::Etablissement).perso().neutralise(Neutralisation::JournalSeul("trace des rappels déjà émis ; l'émission passe par ent_outbox / ent_notification_envoi (neutralisés)")),
    t("ent_fac_sepa_mandat", "finances", Portee::Etablissement).perso().export(),
    t("ent_fac_sepa_prelevement", "finances", Portee::Etablissement).perso().neutralise(N_SEPA),
    t("ent_fermeture_historique", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_fiche_sante", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_filiere", "structure_scolaire", Portee::Etablissement),
    t("ent_fin_article", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_bourse_bareme", "finances", Portee::Etablissement),
    t("ent_fin_compte", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_compte_payeur", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_compteur", "finances", Portee::Etablissement).exc(Exception::SansHorodatage, "compteur technique (numérotation), valeur seule"),
    t("ent_fin_consequence", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_designation", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_echeancier_type", "finances", Portee::Etablissement).exc(Exception::SansHorodatage, "paramétrage financier sans historique"),
    t("ent_fin_entite_facturation", "finances", Portee::Etablissement),
    t("ent_fin_export", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_export_piece", "finances", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par ent_fin_export"),
    t("ent_fin_famille_type", "finances", Portee::Etablissement),
    t("ent_fin_famille_type_enfant", "finances", Portee::Etablissement),
    t("ent_fin_grille_version", "finances", Portee::Etablissement),
    t("ent_fin_journal", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_lettrage", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_palier_relance", "finances", Portee::Etablissement).exc(Exception::SansHorodatage, "paramétrage financier sans historique"),
    t("ent_fin_parametres", "finances", Portee::Etablissement),
    t("ent_fin_prenotification", "finances", Portee::Etablissement).perso().neutralise(N_PRENOT),
    t("ent_fin_prise_en_charge", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_reduction", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_reduction_eleve", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_reglement", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_rejet", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_relance", "finances", Portee::Etablissement).perso().neutralise(Neutralisation::JournalSeul("les relances sont proposées puis décidées par un humain : aucun envoi automatique")),
    t("ent_fin_reprise_rapport", "finances", Portee::Etablissement),
    t("ent_fin_simulation", "finances", Portee::Etablissement),
    t("ent_fin_simulation_calcul", "finances", Portee::Etablissement),
    t("ent_fin_simulation_effectif", "finances", Portee::Etablissement),
    t("ent_fin_simulation_regle", "finances", Portee::Etablissement),
    t("ent_fin_tache", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_tarif", "finances", Portee::Etablissement),
    t("ent_fin_tranche_qf", "finances", Portee::Etablissement),
    t("ent_fonction", "personnels", Portee::Etablissement).exc(Exception::EtabNullable, "fonctions nationales/communes sans établissement (etablissement_id NULL autorisé)"),
    t("ent_formulaire", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_formulaire_reponse", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_foyer", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_foyer_iban_histo", "finances", Portee::Etablissement).perso(),
    t("ent_foyer_quotient", "finances", Portee::Etablissement).perso().export(),
    t("ent_frm_audience_critere", "dossier_eleve", Portee::Etablissement).exc(Exception::SansHorodatage, "critère d'audience (ligne fille du formulaire), daté par le formulaire"),
    t("ent_frm_auteur", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_frm_cible", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_frm_condition", "dossier_eleve", Portee::Etablissement).exc(Exception::SansHorodatage, "condition d'affichage (ligne fille de la question), figée avec la version publiée"),
    t("ent_frm_correction", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_frm_formulaire", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_frm_journal", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_frm_ligne", "dossier_eleve", Portee::Etablissement).exc(Exception::SansHorodatage, "ligne de grille (fille de la question), figée avec la version publiée"),
    t("ent_frm_option", "dossier_eleve", Portee::Etablissement).exc(Exception::SansHorodatage, "option de réponse (fille de la question), figée avec la version publiée"),
    t("ent_frm_question", "dossier_eleve", Portee::Etablissement).exc(Exception::SansHorodatage, "question (fille de la version), figée avec la version publiée"),
    t("ent_frm_section", "dossier_eleve", Portee::Etablissement).exc(Exception::SansHorodatage, "section (fille de la version), figée avec la version publiée"),
    t("ent_frm_soumission", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_frm_valeur", "dossier_eleve", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "valeur de réponse (ligne fille de la soumission), datée par la soumission, en ajout seul"),
    t("ent_frm_version", "dossier_eleve", Portee::Etablissement),
    t("ent_grille", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_grille_cible", "emploi_du_temps_structure", Portee::Etablissement).exc(Exception::SansHorodatage, "table de liaison (grille ↔ cible), datée par la grille"),
    t("ent_grille_creneau", "emploi_du_temps_structure", Portee::Etablissement).exc(Exception::SansHorodatage, "ligne fille : datée par la grille horaire"),
    t("ent_grille_horaire", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_groupe", "structure_scolaire", Portee::Etablissement),
    t("ent_groupe_classe", "structure_scolaire", Portee::Etablissement),
    t("ent_groupe_responsable", "structure_scolaire", Portee::Etablissement).perso().export(),
    t("ent_habilitation_signature", "personnels", Portee::Etablissement).perso().export(),
    // MB1 (2227) : registre d'idempotence des écritures (réponses rejouées 24 h) ; jamais exporté (réponses d'API,
    // pas des données de la personne), vidé à la restauration (une écriture non rejouée est réémise par l'appareil).
    t("ent_idempotence_cle", "identite_acces", Portee::Etablissement).perso().neutralise(N_PURGE),
    t("ent_identite_journal", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_import_ligne", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_import_lot", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_incident", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_incident_type", "vie_scolaire", Portee::Etablissement),
    t("ent_infirmerie_passage", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_info_comp_champ", "dossier_personnes", Portee::Etablissement),
    t("ent_info_comp_valeur", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_inscription_accord", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_accord_demande", "inscriptions", Portee::Etablissement).perso().neutralise(N_PURGE),
    t("ent_inscription_accord_trace", "inscriptions", Portee::Etablissement).perso().export(),
    // F8 : un avis interne n'est jamais rendu à la personne concernée (ni famille, ni candidat majeur).
    t("ent_inscription_avis", "inscriptions", Portee::Etablissement).perso(),
    t("ent_inscription_campagne", "inscriptions", Portee::Etablissement),
    t("ent_inscription_campagne_niveau", "inscriptions", Portee::Etablissement).exc(Exception::SansHorodatage, "ligne fille : datée par la campagne d'inscription"),
    t("ent_inscription_campagne_piece", "inscriptions", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par la campagne d'inscription"),
    t("ent_inscription_campagne_version", "inscriptions", Portee::Etablissement),
    t("ent_inscription_commission", "inscriptions", Portee::Etablissement).perso(),
    t("ent_inscription_commission_dossier", "inscriptions", Portee::Etablissement).perso(),
    t("ent_inscription_condition", "inscriptions", Portee::Etablissement).perso().export().hors_export(&["motif", "remplie_par", "source_ref"])
        // Passe 3 (M-D) : une condition naît d'une décision d'admission ; elle n'est exportée qu'une fois **cette
        // décision publiée** et tant qu'aucune décision plus récente n'attend sa publication (le `libelle` d'une
        // réserve ne quitte jamais une décision que la famille n'a pas lue). Passe 4 (M4) : seul le **bout de chaîne**
        // compte (décision sans successeur) ; une décision remplacée avant publication ne bloque plus rien.
        .export_si("EXISTS (SELECT 1 FROM ent_inscription_decision d WHERE d.etablissement_id = x.etablissement_id AND d.dossier_id = x.dossier_id AND d.publiee_le IS NOT NULL AND d.sens IN ('admis', 'admis_sous_reserve')) AND NOT EXISTS (SELECT 1 FROM ent_inscription_decision p WHERE p.etablissement_id = x.etablissement_id AND p.dossier_id = x.dossier_id AND p.publiee_le IS NULL AND p.caduque_le IS NULL AND NOT EXISTS (SELECT 1 FROM ent_inscription_decision s WHERE s.etablissement_id = p.etablissement_id AND s.remplace_decision_id = p.id))"),
    // Journal des levées (motif, auteur) : interne, jamais exporté.
    t("ent_inscription_condition_levee", "inscriptions", Portee::Etablissement).perso(),
    t("ent_inscription_condition_param", "inscriptions", Portee::Etablissement),
    // F8 : le motif interne ne quitte jamais le personnel ; le déport (identité d'un agent) et le procès-verbal de
    // commission (document collectif de l'établissement) ne sont pas exportés non plus.
    t("ent_inscription_decision", "inscriptions", Portee::Etablissement).perso().export()
        .hors_export(&["motif_interne", "decide_par", "publiee_par", "commission_id", "publication_prevue_le", "origine",
                       "remplace_decision_id", "depassement_capacite", "alternative", "caduque_le", "caduque_motif"])
        .export_si("x.publiee_le IS NOT NULL"),
    t("ent_inscription_declaration_responsable", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_demarche", "inscriptions", Portee::Etablissement),
    t("ent_inscription_deport", "inscriptions", Portee::Etablissement).perso(),
    t("ent_inscription_dossier", "inscriptions", Portee::Etablissement).perso().export()
        // État interne, rang brut, motif et auteur de la décision : ce que la famille lit est l'état **visible**
        // (`visibilite_famille_tx`, rang `rang_famille_tx`), assemblé par l'extrait, jamais ces colonnes (D-069 M-D).
        .hors_export(&["decision_motif", "decide_par", "etat", "etat_avant_blocage", "statut", "rang_attente", "ordre_attente", "offre_attente_id", "echeance_famille",
                       // Passe 3 : posés par les transitions d'une décision (motif communiqué, vœu retenu, date) avant
                       // sa publication ; aucune règle de ligne ne sépare le cas, la colonne sort en entier.
                       "voeu_retenu_id", "clos_le", "motif_cloture", "decide_le"]),
    t("ent_inscription_envoi", "inscriptions", Portee::Etablissement).perso().neutralise(N_INSENVOI).exc(Exception::SansRls, "worker d'envoi des e-mails d'inscription : relève les envois dus de tous les établissements en un lot (`FOR UPDATE SKIP LOCKED`)"),
    t("ent_inscription_fermeture", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_invitation", "inscriptions", Portee::Etablissement).perso().neutralise(N_INSINV).exc(Exception::SansRls, "jeton d'invitation résolu avant de connaître l'établissement (lien reçu par e-mail) ; relue aussi par le worker d'envoi transverse"),
    t("ent_inscription_lien_attente", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_offre", "inscriptions", Portee::Etablissement),
    t("ent_inscription_offre_historique", "inscriptions", Portee::Etablissement),
    t("ent_inscription_offre_unite", "inscriptions", Portee::Etablissement).exc(Exception::SansHorodatage, "ligne fille : datée par l'offre"),
    t("ent_inscription_parametre_lien", "inscriptions", Portee::Etablissement),
    t("ent_inscription_piece", "inscriptions", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_inscription_piece_regle", "inscriptions", Portee::Etablissement).export(),
    t("ent_inscription_piece_regle_condition", "inscriptions", Portee::Etablissement).export().exc(Exception::SansHorodatage, "ligne fille : datée par la règle de pièce"),
    t("ent_inscription_prolongation", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_proposition_place", "inscriptions", Portee::Etablissement).perso().export().hors_export(&["motif", "envoyee_par", "rang_au_moment", "place_no", "origine", "designee_le"])
        // Passe 3 (M-D) : la famille ne lit que les propositions **envoyées** et leur suite ; une place désignée, écartée
        // ou annulée est interne (elle porte une décision parfois non publiée).
        .export_si("x.statut IN ('envoyee', 'acceptee', 'declinee', 'expiree')"),
    t("ent_inscription_reference_compteur", "inscriptions", Portee::Etablissement).exc(Exception::SansHorodatage, "compteur technique (références de dossier), valeur seule"),
    t("ent_inscription_reprise_rapport", "inscriptions", Portee::Etablissement).perso(),
    t("ent_inscription_resolution", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_responsable", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_suivi", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_suivi_dossier", "inscriptions", Portee::Etablissement).perso().export().hors_export(&["texte", "auteur_user_id", "decision_id", "visible_famille"])
        // Une ligne rattachée à une décision n'est exportée qu'une fois cette décision publiée : ni `statut_avant` ni
        // `statut_apres` d'un changement d'état encore caché à la famille (D-069 M-D).
        .export_si("x.visible_famille AND (x.decision_id IS NULL OR EXISTS (SELECT 1 FROM ent_inscription_decision d WHERE d.etablissement_id = x.etablissement_id AND d.id = x.decision_id AND d.publiee_le IS NOT NULL))"),
    t("ent_inscription_voeu", "inscriptions", Portee::Etablissement).perso().export()
        // Passe 3 (M-D) : `retenu` / `non_retenu` est posé par la décision, avant sa publication ; le statut sort en entier.
        .hors_export(&["statut"])
        // Passe 4 (M3) : un vœu **créé par une décision** (`origine = 'decision'`) porte l'offre et le rang que la famille n'a
        // pas encore lus ; il n'est exporté qu'une fois **une décision publiée** du dossier retient cette offre.
        .export_si("x.origine = 'famille' OR EXISTS (SELECT 1 FROM ent_inscription_decision d WHERE d.etablissement_id = x.etablissement_id AND d.dossier_id = x.dossier_id AND d.offre_id = x.offre_id AND d.publiee_le IS NOT NULL)"),
    t("ent_internat_absence", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_affectation", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_appel", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_appel_attendu", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_appel_rappel", "vie_pratique", Portee::Etablissement),
    // Passe 2 (M5) : la nature « interdiction_auteur » et la note de clôture révèlent la cause d'un blocage (donnée de protection).
    t("ent_internat_cas", "vie_pratique", Portee::Etablissement).perso().export().hors_export(&["nature", "issue_note"]),
    t("ent_internat_cas_etape", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_chambre", "vie_pratique", Portee::Etablissement),
    t("ent_internat_correspondant", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_derogation", "vie_pratique", Portee::Etablissement).perso().export(),
    // M10 (règle 3) : l'export de portabilité d'une personne ne contient ni texte libre ni autre élève. Un événement de nuit
    // (nature, niveau, mesures : circonstances impliquant d'autres élèves) et ses compléments (texte libre) n'en font pas
    // partie ; la participation propre de l'élève (`ent_internat_evenement_participant`) reste exportée.
    t("ent_internat_evenement", "vie_pratique", Portee::Etablissement).perso(),
    t("ent_internat_evenement_complement", "vie_pratique", Portee::Etablissement).perso(),
    t("ent_internat_evenement_participant", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_i3_fait", "vie_pratique", Portee::Etablissement).perso(),
    t("ent_internat_interdiction_contact", "vie_pratique", Portee::Etablissement).perso(),
    t("ent_internat_lecture_sensible", "vie_pratique", Portee::Etablissement).perso(),
    // M10 : le fait de liaison (code, nuit) est exporté, jamais son texte libre.
    t("ent_internat_liaison", "vie_pratique", Portee::Etablissement).perso().export().hors_export(&["texte"]),
    t("ent_internat_liaison_code", "vie_pratique", Portee::Etablissement),
    t("ent_internat_liaison_fait", "vie_pratique", Portee::Etablissement).perso(),
    t("ent_internat_liste", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_liste_chambre", "vie_pratique", Portee::Etablissement),
    t("ent_internat_lit", "vie_pratique", Portee::Etablissement),
    t("ent_internat_nuit_exceptionnelle", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_parametres", "vie_pratique", Portee::Etablissement),
    t("ent_internat_parametres_vie", "vie_pratique", Portee::Etablissement),
    t("ent_internat_perte_droit_fait", "vie_pratique", Portee::Etablissement).perso(),
    t("ent_internat_preoccupation", "vie_pratique", Portee::Etablissement).perso(),
    t("ent_internat_presence", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_presence_fait", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_procedure_etape", "vie_pratique", Portee::Etablissement),
    t("ent_internat_procedure_version", "vie_pratique", Portee::Etablissement),
    t("ent_internat_retrait", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_rythme", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_sejour", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_sortie", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_sortie_groupe", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_sortie_groupe_accord_hors_ligne", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_sortie_groupe_encadrant", "vie_pratique", Portee::Etablissement),
    t("ent_internat_sortie_groupe_fait", "vie_pratique", Portee::Etablissement).perso(),
    // Passe 2 (M5) : `motif_code` (interdiction, désaccord connu, majorité) est la cause d'un blocage, `reponse_user_id` le
    // compte qui a répondu : ni l'un ni l'autre ne sortent dans l'export de portabilité.
    t("ent_internat_sortie_groupe_participant", "vie_pratique", Portee::Etablissement)
        .perso()
        .export()
        .hors_export(&["motif_code", "reponse_user_id"]),
    t("ent_internat_sortie_type", "vie_pratique", Portee::Etablissement),
    t("ent_internat_suivi", "vie_pratique", Portee::Etablissement).perso().export(),
    // F5 : le tiers reçoit sa propre fiche (personne, dates, état), jamais le motif écrit ni le titulaire qui l'a demandé.
    t("ent_internat_temps", "vie_pratique", Portee::Etablissement),
    t("ent_internat_temps_participant", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_tiers_autorise", "vie_pratique", Portee::Etablissement)
        .perso()
        .export()
        .hors_export(&["motif", "demande_par_personne_id"]),
    t("ent_internat_transmission", "vie_pratique", Portee::Etablissement),
    t("ent_internat_type_appel_actif", "vie_pratique", Portee::Etablissement),
    t("ent_internat_vigilance", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_zone", "vie_pratique", Portee::Etablissement),
    t("ent_jour_calendrier", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_journal_modif", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_lien_attente", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_lieu", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_login_guard", "identite_acces", Portee::Technique).perso().neutralise(N_PURGE),
    t("ent_mail_outbox", "communication_technique", Portee::Etablissement).perso().neutralise(Neutralisation::JournalSeul("journal des envois déjà tentés (dry_run/sent/failed) : aucun état « à envoyer », rien à suspendre")),
    t("ent_mandat_sepa_demande", "finances", Portee::Etablissement).perso().export(),
    t("ent_matiere", "evaluations_pedagogie", Portee::Etablissement),
    t("ent_mef", "structure_scolaire", Portee::Etablissement),
    t("ent_membership", "identite_acces", Portee::Etablissement).perso().export().exc(Exception::SansRls, "lu à l'authentification, avant le choix d'établissement (un compte peut appartenir à plusieurs établissements, D-013) : ses adhésions sont listées toutes ensemble"),
    t("ent_membership_casquette", "identite_acces", Portee::Etablissement).perso().export(),
    t("ent_menu_cantine", "vie_pratique", Portee::Etablissement),
    t("ent_metier_event", "communication_technique", Portee::Etablissement).perso().neutralise(Neutralisation::JournalSeul("journal des faits métier ; la distribution est pilotée par ent_outbox (neutralisé)")).exc(Exception::SansRls, "journal d'événements lu par le distributeur transverse (jointure avec ent_outbox, tous établissements) ; écrit par les services dans leur transaction"),
    t("ent_migration_appliquee", "plateforme", Portee::Technique),
    t("ent_mission", "personnels", Portee::Etablissement).perso().export(),
    t("ent_mission_perimetre", "personnels", Portee::Etablissement).perso().export(),
    t("ent_mission_type", "personnels", Portee::Etablissement).exc(Exception::EtabNullable, "types de missions fournis par ScolIA (etablissement_id NULL autorisé), copiés ou désactivés par établissement"),
    t("ent_module_etablissement", "etablissement", Portee::Etablissement),
    t("ent_msg", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_accuse", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_annulation_garde", "messagerie", Portee::Etablissement),
    t("ent_msg_astreinte", "messagerie", Portee::Etablissement).perso(),
    t("ent_msg_boite", "messagerie", Portee::Etablissement),
    t("ent_msg_boite_designation", "messagerie", Portee::Etablissement).perso(),
    t("ent_msg_boite_horaire", "messagerie", Portee::Etablissement).exc(Exception::SansHorodatage, "plage horaire d'une boîte : valeur de paramétrage remplacée en bloc, sans historique propre"),
    t("ent_msg_boite_mission", "messagerie", Portee::Etablissement).exc(Exception::SansHorodatage, "liaison boîte ↔ type de mission : la date utile est celle de la mission (socle 6)"),
    t("ent_msg_brouillon", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_carte_document", "messagerie", Portee::Etablissement).perso(),
    t("ent_msg_etiquette", "messagerie", Portee::Etablissement).exc(Exception::SansHorodatage, "liste courte d'étiquettes d'une boîte : paramétrage sans date"),
    t("ent_msg_fil", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_fil_eleve", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_lecture", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_lien_decision", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_lot", "messagerie", Portee::Etablissement).perso(),
    t("ent_msg_lot_eleve", "messagerie", Portee::Etablissement).perso().exc(Exception::SansHorodatage, "liaison lot ↔ élève : la date est celle du lot (`ent_msg_lot.cree_le`)"),
    t("ent_msg_modele", "messagerie", Portee::Etablissement),
    t("ent_msg_note_interne", "messagerie", Portee::Etablissement).perso(),
    t("ent_msg_participant", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_piece", "messagerie", Portee::Etablissement).perso().fichiers(Fichiers::ObjetAvecResidu).export(),
    t("ent_msg_piece_acces", "messagerie", Portee::Etablissement).perso(),
    t("ent_msg_reglage", "messagerie", Portee::Etablissement),
    t("ent_msg_reglage_version", "messagerie", Portee::Etablissement),
    t("ent_msg_reprise_rapport", "messagerie", Portee::Etablissement),
    t("ent_msg_telephone", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_traitement", "messagerie", Portee::Etablissement).perso(),
    t("ent_msg_traitement_etiquette", "messagerie", Portee::Etablissement).exc(Exception::SansHorodatage, "liaison traitement ↔ étiquette : la date est celle du traitement (`maj_le`)"),
    t("ent_msg_transfert", "messagerie", Portee::Etablissement).perso(),
    t("ent_msg_version", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_niveau", "structure_scolaire", Portee::Etablissement),
    // Suites V5 passe 5 (F4) : table de REPRISE des appréciations (écrite par le conseil de classe et l'ancien module de notes),
    // lue par le bulletin ; elle n'a ni état, ni date de clôture : exportée telle quelle, elle contournait la règle E1 de
    // `eva_appreciation` (texte pas encore remis à la famille). Retirée de l'export jusqu'à G3 (qui exportera le bulletin figé,
    // c'est-à-dire ce que la famille a reçu) ; `eva_appreciation`, conditionnée, porte l'appréciation remise.
    t("ent_note_appreciation", "evaluations_pedagogie", Portee::Etablissement).perso(),
    // Suites V5 passe 6 (Moyen 5, D-069 : on n'exporte que ce que la famille voit, rien de visible ne disparaît). Archive des
    // notes de l'ancienne grille, reprise dans `eva_resultat` : une note n'entre dans l'extrait que si son devoir historique
    // est PUBLIÉ (même règle que la reprise : sans publication explicite, visible dès sa création ; en publication explicite,
    // visible quand `publie_le` est posé). Une note sans devoir n'a jamais été reprise ni montrée : exclue. Ni l'auteur de la
    // saisie, ni le dernier éditeur.
    t("ent_note_archive", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .hors_export(&["auteur_user_id", "updated_by_user_id"])
    // Contre-revue du pilote (passe 6, M-D) : l'archive est figée, la publication vit ensuite dans l'évaluation reprise (même
    // identifiant que le devoir, 472). Si l'évaluation reprise existe et n'est plus publiée (retrait), la note ne sort plus,
    // comme `eva_resultat`. Si la reprise a refusé le devoir (aucune évaluation), la note vue par la famille dans l'ancienne
    // grille reste exportée (D-069 : rien de visible ne disparaît).
        .export_si("x.devoir_id IS NOT NULL AND EXISTS (SELECT 1 FROM ent_devoir_archive d WHERE d.etablissement_id = x.etablissement_id AND d.id = x.devoir_id AND (NOT d.publication_explicite OR d.publie_le IS NOT NULL)) AND NOT EXISTS (SELECT 1 FROM eva_evaluation e WHERE e.etablissement_id = x.etablissement_id AND e.id = x.devoir_id AND NOT (e.publiee_le IS NOT NULL AND e.etat IN ('ouverte', 'annulee')))"),
    // Journal des corrections d'une note après clôture (motifs, auteurs, valeurs avant/après, y compris sur des notes jamais
    // publiées) : interne, comme `eva_resultat_historique`. La note corrigée visible est dans `ent_note_archive`.
    t("ent_note_correction", "evaluations_pedagogie", Portee::Etablissement).perso(),
    // Copies de notes écartées lors du dédoublonnage (D8) : rebuts techniques, jamais montrés à la famille : hors export.
    t("ent_note_doublon_archive", "evaluations_pedagogie", Portee::Etablissement).perso(),
    // Registre des périodes de notes de l'établissement (code, libellé, dates) : sans `eleve_id`, aucune donnée d'élève.
    t("ent_note_periode", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .hors_export_par_eleve("registre des périodes de notes de l'établissement (code, libellé, dates) : sans eleve_id, jamais dans l'extrait d'un élève"),
    t("ent_note_periode_journal", "evaluations_pedagogie", Portee::Etablissement).perso(),
    t("ent_notification", "communication_technique", Portee::Etablissement).perso().export(),
    t("ent_notification_envoi", "communication_technique", Portee::Etablissement).perso().neutralise(N_NOTIF),
    t("ent_notification_plafond", "communication_technique", Portee::Etablissement).perso(),
    t("ent_ntf_a_prevenir", "communication_technique", Portee::Etablissement).perso(),
    t("ent_ntf_avis", "communication_technique", Portee::Etablissement).perso().neutralise(N_NTF_AVIS),
    t("ent_ntf_avis_param", "communication_technique", Portee::Etablissement).exc(Exception::SansHorodatage, "paramètres typés d'un avis (identifiants), datés par l'avis"),
    t("ent_ntf_destinataire", "communication_technique", Portee::Etablissement).perso(),
    t("ent_ntf_gel", "communication_technique", Portee::Etablissement).perso(),
    t("ent_ntf_lien", "communication_technique", Portee::Etablissement).perso(),
    t("ent_ntf_nature_etab", "communication_technique", Portee::Etablissement),
    t("ent_ntf_parametre", "communication_technique", Portee::Etablissement),
    t("ent_ntf_preference", "communication_technique", Portee::Etablissement).perso().export(),
    t("ent_ntf_reglage_personne", "communication_technique", Portee::Etablissement).perso().export(),
    t("ent_ntf_remise", "communication_technique", Portee::Etablissement).perso().neutralise(N_NTF_REMISE),
    t("ent_ntf_resume", "communication_technique", Portee::Etablissement).perso(),
    t("ent_objet", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Objet).export(),
    t("ent_offre_cible", "structure_scolaire", Portee::Etablissement).exc(Exception::SansHorodatage, "table de liaison (offre ↔ cible), datée par l'offre"),
    t("ent_offre_enseignement", "structure_scolaire", Portee::Etablissement),
    t("ent_offre_groupe", "structure_scolaire", Portee::Etablissement),
    t("ent_offre_incompatibilite", "structure_scolaire", Portee::Etablissement),
    t("ent_offre_prerequis", "structure_scolaire", Portee::Etablissement),
    t("ent_offre_regle_choix", "structure_scolaire", Portee::Etablissement),
    t("ent_offre_regle_choix_offre", "structure_scolaire", Portee::Etablissement).exc(Exception::SansHorodatage, "table de liaison (règle ↔ offre), datée par la règle"),
    t("ent_oidc_identity", "identite_acces", Portee::CompteMultiEtablissement).perso().export(),
    t("ent_oidc_provider", "identite_acces", Portee::Etablissement).exc(Exception::SansRls, "résolu par fournisseur avant toute authentification (SSO OIDC) : l'établissement n'est pas encore connu"),
    t("ent_oidc_state", "identite_acces", Portee::Technique).perso().neutralise(N_PURGE).avec_etab(),
    t("ent_orientation_avis", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_outbox", "communication_technique", Portee::Etablissement).perso().neutralise(N_OUTBOX).exc(Exception::SansRls, "distributeur d'événements transverse : un lot lit les événements de tous les établissements (`distribuer_un_lot`), le contexte est posé par événement dans `traiter_par`"),
    t("ent_outbox_agregat", "communication_technique", Portee::Etablissement).neutralise(Neutralisation::JournalSeul("compteur de séquence par agrégat : à conserver, sinon l'ordre des événements se rompt")).exc(Exception::SansRls, "compteur de séquence tenu par le distributeur transverse d'événements (mêmes accès que ent_outbox)").exc(Exception::SansHorodatage, "compteur technique (séquence par agrégat), valeur seule"),
    t("ent_pai", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_pai_piece", "sante", Portee::Etablissement).perso().sante().fichiers(Fichiers::ObjetAvecResidu).export(),
    t("ent_pai_signature", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_param_eleve_champ", "structure_scolaire", Portee::Etablissement),
    t("ent_param_eleve_fuseau", "structure_scolaire", Portee::Etablissement),
    t("ent_param_eleve_numero", "structure_scolaire", Portee::Etablissement),
    t("ent_param_eleve_profil", "structure_scolaire", Portee::Etablissement),
    t("ent_param_eleve_regime", "structure_scolaire", Portee::Etablissement),
    t("ent_param_eleve_regle_auto", "structure_scolaire", Portee::Etablissement),
    t("ent_param_etab", "etablissement", Portee::Etablissement),
    t("ent_parametre", "etablissement", Portee::Etablissement),
    t("ent_passkey_challenge", "identite_acces", Portee::Technique).perso().neutralise(N_PURGE).avec_etab(),
    t("ent_passkey_credential", "identite_acces", Portee::CompteMultiEtablissement).perso().export().avec_etab(),
    t("ent_password_reset", "identite_acces", Portee::Technique).perso().neutralise(N_PURGE),
    t("ent_periode", "evaluations_pedagogie", Portee::Etablissement),
    t("ent_periode_vacances", "evaluations_pedagogie", Portee::Etablissement),
    t("ent_periodisation", "evaluations_pedagogie", Portee::Etablissement),
    t("ent_periodisation_cible", "evaluations_pedagogie", Portee::Etablissement),
    t("ent_personne", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_consentement", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_coordonnee", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_coordonnee_histo", "dossier_personnes", Portee::Etablissement).perso(),
    t("ent_personne_divergence_decision", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_fusion", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_fusion_contact_supprime", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_fusion_lien_supprime", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_fusion_objet", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_plage_joignable", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_preference", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_preference_version", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_preference_version_plage", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_reserve", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_version", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_politique_securite", "identite_acces", Portee::Etablissement),
    t("ent_preference_notification", "communication_technique", Portee::Etablissement).perso().export(),
    t("ent_preinscription_verification", "inscriptions", Portee::Etablissement).perso().neutralise(N_PURGE).exc(Exception::SansRls, "jeton de vérification d'e-mail résolu avant de connaître l'établissement (lien reçu par e-mail)"),
    t("ent_prestataire", "sorties_voyages", Portee::Etablissement),
    t("ent_prestataire_adresse", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_prestataire_categorie", "sorties_voyages", Portee::Etablissement),
    t("ent_procedure_disciplinaire", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_profil_personnel", "identite_acces", Portee::CompteMultiEtablissement).perso().export(),
    t("ent_psy_dossier", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_psy_entretien", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_psy_rdv", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_punition", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_punition_type", "vie_scolaire", Portee::Etablissement),
    // MB0 (DS5, F5) : la personne reçoit les métadonnées de ses appareils (plateforme, dates, état), jamais le
    // jeton ni son empreinte ; à la restauration, tous les appareils actifs sont révoqués (plus aucun envoi).
    t("ent_push_device", "communication_technique", Portee::Etablissement)
        .perso()
        .export()
        .hors_export(&["token", "token_empreinte"])
        .neutralise(N_PUSH_DEVICE),
    t("ent_push_outbox", "communication_technique", Portee::Etablissement).perso().neutralise(Neutralisation::JournalSeul("journal des envois déjà tentés (dry_run/sent/failed) : aucun état « à envoyer », rien à suspendre")),
    t("ent_rate_limit", "identite_acces", Portee::Technique).neutralise(N_PURGE),
    t("ent_rdv_fait", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    // Socle 29 (lot P5) : compteur volatil de recherches (aucun texte de requête), vidé à la restauration.
    t("ent_recherche_compteur", "portails", Portee::Etablissement).perso().neutralise(N_PURGE),
    t("ent_recuperation_passkey", "identite_acces", Portee::Technique).perso().neutralise(N_PURGE).avec_etab(),
    t("ent_regime_code", "vie_pratique", Portee::ReferentielGlobal).exc(Exception::SansHorodatage, "référentiel statique des régimes (R-IT2), jamais modifié par un établissement"),
    t("ent_regime_correspondance", "vie_pratique", Portee::ReferentielGlobal).exc(Exception::SansHorodatage, "table de correspondance des vocabulaires de régime (R-IT2), statique"),
    t("ent_regle_effacement", "autre", Portee::Etablissement),
    t("ent_reinscription_campagne", "inscriptions", Portee::Etablissement),
    t("ent_reinscription_exigence_doc", "inscriptions", Portee::Etablissement),
    t("ent_rencontre", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_repas_activite", "vie_pratique", Portee::Etablissement),
    t("ent_repas_passage", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_retard", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_role", "identite_acces", Portee::Etablissement),
    t("ent_role_attribution", "identite_acces", Portee::Etablissement).perso().export(),
    t("ent_role_droit", "identite_acces", Portee::Etablissement).exc(Exception::SansHorodatage, "table de liaison (rôle ↔ droit), datée par le rôle"),
    t("ent_salle", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_salle_indisponibilite", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_sanction", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_sanction_motif", "vie_scolaire", Portee::Etablissement),
    t("ent_sanction_type", "vie_scolaire", Portee::Etablissement),
    t("ent_sante_protocole", "sante", Portee::Etablissement),
    t("ent_seance_contenu", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_semaine", "evaluations_pedagogie", Portee::Etablissement),
    t("ent_semaine_type_regle", "evaluations_pedagogie", Portee::Etablissement),
    t("ent_service_enseignement", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_service_intervenant", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_session", "identite_acces", Portee::Technique).perso().neutralise(N_PURGE).avec_etab(),
    t("ent_signalement_assiduite", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_signalement_assiduite_detail", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_site", "etablissement", Portee::Etablissement),
    t("ent_sms_outbox", "communication_technique", Portee::Etablissement).perso().neutralise(Neutralisation::JournalSeul("journal des envois déjà tentés (dry_run/sent/failed) : aucun état « à envoyer », rien à suspendre")),
    t("ent_sortie", "sorties_voyages", Portee::Etablissement).perso().export(),
    t("ent_sortie_acompte_fournisseur", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_aide", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_argent_acces", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_argent_contournement", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_argent_contournement_evt", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_argent_demande", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_argent_export", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_argent_issue", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_argent_refus", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_argent_reglage", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_autorisation", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_sortie_avenant", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_avoir_propose", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_bordereau", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_bordereau_ligne", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_bordereau_reception", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_bordereau_remise", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_budget_ligne", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_budget_version", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_campagne", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_commande", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_controle", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_sortie_demande", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_sortie_devis", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_document", "sorties_voyages", Portee::Etablissement).fichiers(Fichiers::Objet),
    t("ent_sortie_encadrant", "sorties_voyages", Portee::Etablissement).perso().export(),
    t("ent_sortie_envoi", "sorties_voyages", Portee::Etablissement).perso().neutralise(N_ENVOI_PRESTATAIRE),
    t("ent_sortie_etape", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_exigence", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_exigence_fait", "sorties_voyages", Portee::Etablissement).perso().fichiers(Fichiers::Objet),
    t("ent_sortie_frais_reel", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_internat_retrait", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_internat_retrait_fin", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_journal", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_journal_delegue", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_journal_lien", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_journal_voyage", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_nominatif_ouverture", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_param", "assiduite", Portee::Etablissement),
    t("ent_sortie_part", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_participant", "sorties_voyages", Portee::Etablissement).perso().export(),
    t("ent_sortie_participant_fait", "sorties_voyages", Portee::Etablissement).perso().export(),
    t("ent_sortie_plan", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_prix_fige", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_public", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_publication", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_publication_photo", "sorties_voyages", Portee::Etablissement).perso().fichiers(Fichiers::Objet),
    t("ent_sortie_publication_photo_eleve", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_rappel", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_reglage", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_regle_niveau", "assiduite", Portee::Etablissement),
    t("ent_sortie_regle_regime", "assiduite", Portee::Etablissement),
    t("ent_sortie_relance", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_repas_attribution", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_repas_avis", "sorties_voyages", Portee::Etablissement).perso().neutralise(N_REPAS_AVIS),
    t("ent_sortie_repas_commande", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_repas_externe", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_repas_ligne", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_reponse_cle", "sorties_voyages", Portee::Etablissement).neutralise(N_REPONSE_CLE),
    t("ent_sortie_report", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_reprise_ecart", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_reprise_participant", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_reprise_sejour", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_serie", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_serie_exclusion", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_signature", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_transmission", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_sortie_transmission_champ", "sorties_voyages", Portee::Etablissement),
    t("ent_sortie_versement", "sorties_voyages", Portee::Etablissement).perso(),
    t("ent_stage_dispositif_etab", "stages", Portee::Etablissement),
    t("ent_stage_parametre", "stages", Portee::Etablissement),
    t("ent_stage_periode", "stages", Portee::Etablissement),
    t("ent_stage_periode_public", "stages", Portee::Etablissement).perso(),
    t("ent_structure_journal", "emploi_du_temps_structure", Portee::Etablissement).perso(),
    t("ent_systeme_identifiant", "dossier_personnes", Portee::ReferentielGlobal).exc(Exception::SansHorodatage, "référentiel global statique (codes de systèmes d'identifiants)"),
    t("ent_tarif_conso", "finances", Portee::Etablissement),
    t("ent_taux_tva", "finances", Portee::Etablissement),
    t("ent_travail", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_travail_eleve", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_travail_remise", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_user", "identite_acces", Portee::CompteMultiEtablissement).perso().export(),
    t("ent_voeu_inscription", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_vs_commission", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_vs_commission_dossier", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_vs_regle_assiduite", "assiduite", Portee::Etablissement),
    t("eva_aesh_indisponibilite_port", "evaluations_pedagogie", Portee::Etablissement).perso(),
    // Suites V5 passe 4 (M2, RGPD art. 15) : n'est exportée que l'appréciation **que la famille voit**, même règle que la
    // vue `ent_note_appreciation_v` (468, M5) : validée, de type cours ou générale, et appréciations closes pour la classe de
    // l'élève (rang d'état de période ≥ 2). Validée par le professeur ne veut pas dire publiée (contre-revue, E1). Ni auteur,
    // ni dernier éditeur, ni origine (interne). `aide_ia` reste : la famille a le droit de savoir qu'un texte a été assisté.
    t("eva_appreciation", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .hors_export(&["auteur", "modifie_par", "origine_appreciation_id"])
        .export_si("x.etat = 'validee' AND x.type IN ('cours', 'generale') AND EXISTS (SELECT 1 FROM ent_periode p WHERE p.id = x.periode_id AND p.etablissement_id = x.etablissement_id AND EXISTS (SELECT 1 FROM eva_periode_eleve_jour(x.etablissement_id, x.eleve_id, p.date_fin) pe WHERE pe.periode_id = x.periode_id AND eva_etat_rang(pe.etat) >= 2))"),
    // Journal en ajout seul : versions successives (brouillons, textes écartés) et éditeur de chacune : interne.
    t("eva_appreciation_historique", "evaluations_pedagogie", Portee::Etablissement).perso(),
    t("eva_banque_phrase", "evaluations_pedagogie", Portee::Etablissement).perso(),
    t("eva_bareme_item", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_coefficient_matiere", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_coefficient_sous_matiere", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_consultation", "evaluations_pedagogie", Portee::Etablissement).perso(),
    // La demande (motif, état, date de décision) est celle de la famille ; la personne qui a décidé est interne.
    t("eva_correction_demande", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .hors_export(&["decide_par"])
        .tout_exportable("demande de la famille sur une note publiée : motif, état et date de décision lui ont été communiqués"),
    // Suites V5 passe 5 (F3, D-069 : on n'exporte que ce qui est communiqué à la famille). Aucune colonne ne dit qu'une
    // dispense a été notifiée à la famille (ni lecture familiale de la table dans l'ENT) : valeur par défaut PRUDENTE =
    // retirée de l'export. À rouvrir quand l'ENT notifiera la décision (colonne `communiquee_le` + `export_si`).
    t("eva_dispense_port", "evaluations_pedagogie", Portee::Etablissement).perso(),
    // Enseignement suivi ailleurs. Contre-revue du pilote (passe 5, Moyen 1) : sans cette table, une moyenne externe publiée
    // exportée n'a ni matière ni organisme (la famille la voit pourtant avec sa matière au bulletin). Exporté seulement
    // quand il porte une moyenne PUBLIÉE de statut « moyenne » et compte dans la moyenne générale (même lecture que le
    // bulletin, passe 6 Moyen 2) ; référent tiers, auteur et la colonne « compte dans la moyenne générale » restent hors export.
    t("eva_enseignement_externe", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .hors_export(&["referent", "declare_par", "compte_moyenne_generale"])
        .export_si("EXISTS (SELECT 1 FROM eva_moyenne_externe m WHERE m.etablissement_id = x.etablissement_id AND m.enseignement_externe_id = x.id AND m.publiee_le IS NOT NULL AND m.statut = 'moyenne' AND x.compte_moyenne_generale)"),
    t("eva_evaluation", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_evaluation_commune", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_evaluation_historique", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_evaluation_publication_journal", "evaluations_pedagogie", Portee::Etablissement),
    // Public restreint d'une évaluation : ne sort que si l'évaluation est publiée (sinon elle révèle une évaluation non annoncée).
    t("eva_evaluation_restriction", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .export_si("EXISTS (SELECT 1 FROM eva_evaluation e WHERE e.etablissement_id = x.etablissement_id AND e.id = x.evaluation_id AND e.publiee_le IS NOT NULL AND e.etat IN ('ouverte', 'annulee'))"),
    t("eva_import_journal", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_instantane", "evaluations_pedagogie", Portee::Etablissement),
    // Instantané d'arrêt (dérivé : moyennes figées, statistiques de classe) pris AVANT la publication du bulletin et calculé
    // sur des notes pas toutes publiées : retiré de l'export. Le bulletin figé et les notes publiées portent ce que la
    // famille a reçu ; l'assemblage de l'extrait (G3) reprendra le bulletin.
    t("eva_instantane_ligne", "evaluations_pedagogie", Portee::Etablissement).perso(),
    // Passe 3 (D-069) : rien de non publié n'est jamais exporté. Une moyenne externe ne sort que publiée ; ni auteur, ni motif.
    // Passe 6 (Moyen 2) : ALIGNÉE sur la lecture du bulletin (`evaluations_v1.rs`, moyennes externes) : la famille ne voit une
    // moyenne externe que si elle est publiée, de statut « moyenne » (valeur numérique) et rattachée à un enseignement qui
    // COMPTE dans la moyenne générale. Une ligne « en attente » ou « dispense », ou celle d'un enseignement qui ne compte pas,
    // n'apparaît pas au bulletin : elle n'a pas été remise à la famille.
    t("eva_moyenne_externe", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .hors_export(&["saisi_par", "publiee_par", "retrait_par", "dernier_retrait_par", "motif_correction", "dernier_motif_correction"])
        .export_si("x.publiee_le IS NOT NULL AND x.statut = 'moyenne' AND EXISTS (SELECT 1 FROM eva_enseignement_externe ens WHERE ens.etablissement_id = x.etablissement_id AND ens.id = x.enseignement_externe_id AND ens.compte_moyenne_generale)"),
    // Journal en ajout seul (valeurs avant correction, motifs, auteurs, lignes non publiées) : interne, jamais exporté.
    t("eva_moyenne_externe_historique", "evaluations_pedagogie", Portee::Etablissement).perso(),
    // Statut « en attente » / « non significative » d'une moyenne : donnée personnelle de l'élève (exportée au titre du DROIT
    // D'ACCÈS, RGPD art. 15, décision D-069 du pilote), PAS au titre d'une lecture familiale : aucune route ne le montre à la
    // famille en dehors de la moyenne. On ne l'exporte donc qu'à la condition qu'il ne révèle rien de non publié : quand les
    // appréciations de la période sont closes pour la classe de l'élève (même règle que `eva_appreciation` et la vue
    // `ent_note_periode_v`) ; avant, il révèle qu'une moyenne est en suspens. Motif interne et auteur du statut jamais exportés.
    t("eva_moyenne_statut", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .hors_export(&["motif_interne", "par"])
        .export_si("EXISTS (SELECT 1 FROM ent_periode p WHERE p.id = x.periode_id AND p.etablissement_id = x.etablissement_id AND EXISTS (SELECT 1 FROM eva_periode_eleve_jour(x.etablissement_id, x.eleve_id, p.date_fin) pe WHERE pe.periode_id = x.periode_id AND eva_etat_rang(pe.etat) >= 2))"),
    t("eva_nature", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_periode_etat", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_periode_transition", "evaluations_pedagogie", Portee::Etablissement),
    // Convocation au rattrapage (date, lieu, modalité, échéance, état). Donnée personnelle de l'élève exportée au titre du DROIT
    // D'ACCÈS (RGPD art. 15, décision D-069 du pilote) : aucune lecture familiale de cette table n'est établie dans l'ENT, on ne
    // prétend donc pas qu'elle a été « communiquée ». Condition (passe 5, M5) : l'export ne révèle rien de non publié : seule
    // une convocation maintenue (état ≠ « annulée ») d'une évaluation PUBLIÉE sort (une évaluation non publiée n'est pas annoncée ;
    // le motif d'une convocation annulée est interne). Jamais : auteurs, motifs internes, ni le zéro attribué (une note non publiée).
    t("eva_rattrapage", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .hors_export(&["cree_par", "modifie_par", "motif_annulation", "dernier_motif_prolongation", "zero_attribue_le", "zero_attribue_par"])
        .export_si("x.etat <> 'annule' AND EXISTS (SELECT 1 FROM eva_evaluation e WHERE e.etablissement_id = x.etablissement_id AND e.id = x.evaluation_id AND e.publiee_le IS NOT NULL AND e.etat IN ('ouverte', 'annulee'))"),
    // Autorisation de rattrapage donnée par la direction (versions successives, échéance, motif, auteur) : décision interne,
    // la famille reçoit la convocation (`eva_rattrapage`), pas l'autorisation (M5) : retirée de l'export.
    t("eva_rattrapage_autorise", "evaluations_pedagogie", Portee::Etablissement).perso(),
    // Journal en ajout seul des convocations (états successifs, dates avant/après, auteurs) : comme l'historique des
    // appréciations, interne ; la convocation en vigueur est dans `eva_rattrapage` (M4) : retiré de l'export.
    t("eva_rattrapage_historique", "evaluations_pedagogie", Portee::Etablissement).perso(),
    t("eva_reglage", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_reglage_historique", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_reprise_compte", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_reprise_execution", "evaluations_pedagogie", Portee::Etablissement),
    t("eva_reprise_ligne", "evaluations_pedagogie", Portee::Etablissement),
    // Une note ne sort que si son évaluation est publiée (même condition que la vue `eva_resultat_publie`).
    // Résultat d'une évaluation publiée, sans les champs internes (note privée du professeur, marque « aménagé » jamais
    // imprimée à un tiers, auteur, motifs de correction) — comme la vue `eva_resultat_publie` que lit la famille.
    t("eva_resultat", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .hors_export(&["note_privee", "amenage", "saisi_par", "motif_correction", "dernier_motif_correction", "derniere_ecriture_correction", "source", "version"])
        .export_si("EXISTS (SELECT 1 FROM eva_evaluation e WHERE e.etablissement_id = x.etablissement_id AND e.id = x.evaluation_id AND e.publiee_le IS NOT NULL AND e.etat IN ('ouverte', 'annulee'))"),
    // Résultat externe : publié, compté et non retiré (ce que lit la famille), sans auteur ni motif.
    t("eva_resultat_externe", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .hors_export(&["saisi_par", "compte_decide_par", "compte_decide_le", "compte_motif", "publiee_par", "retiree_par", "motif_retrait"])
        .export_si("x.publiee_le IS NOT NULL AND x.compte_dans_cours_id IS NOT NULL AND x.retiree_le IS NULL"),
    // Journal en ajout seul des résultats (valeurs successives, motifs, auteurs, y compris avant publication) : interne.
    t("eva_resultat_historique", "evaluations_pedagogie", Portee::Etablissement).perso(),
    t("eva_resultat_item", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .export_si("EXISTS (SELECT 1 FROM eva_resultat r JOIN eva_evaluation e ON e.etablissement_id = r.etablissement_id AND e.id = r.evaluation_id WHERE r.etablissement_id = x.etablissement_id AND r.id = x.resultat_id AND e.publiee_le IS NOT NULL AND e.etat IN ('ouverte', 'annulee'))"),
    // Demande de revue de la famille et réponse qui lui a été faite (`reponse` n'existe que si l'état est « repondue »).
    t("eva_revision_demande", "evaluations_pedagogie", Portee::Etablissement).perso().export()
        .tout_exportable("message de la famille et réponse qui lui a été adressée : aucun champ interne dans la table"),
    t("ref_asd_nature_motif", "assiduite", Portee::ReferentielGlobal),
    t("ref_asd_profil_academique", "assiduite", Portee::ReferentielGlobal),
    t("ref_asd_profil_niveau", "assiduite", Portee::ReferentielGlobal),
    t("ref_calendrier_officiel", "referentiel", Portee::ReferentielGlobal),
    t("ref_stage_dispositif", "stages", Portee::ReferentielGlobal),
    // Socle 28, lots R2 et R3 : pilotage de la reprise et file de rapprochement (preuve : conservation « mes.reprise », durée à décider).
    // Les résumés minimisés de la file sont mis à NULL à la purge de la zone (MSE-179).
    t("rep_correspondance", "mise_en_service", Portee::Etablissement),
    t("rep_decision", "mise_en_service", Portee::Etablissement).perso(),
    t("rep_decision_raison", "mise_en_service", Portee::Etablissement).exc(Exception::SansHorodatage, "ligne fille de `rep_decision` (la date de la décision y est portée)"),
    t("rep_execution", "mise_en_service", Portee::Etablissement),
    t("rep_execution_a_decider", "mise_en_service", Portee::Etablissement),
    t("rep_execution_compte", "mise_en_service", Portee::Etablissement),
    t("rep_proposition", "mise_en_service", Portee::Etablissement).perso(),
    t("rep_proposition_candidat", "mise_en_service", Portee::Etablissement).perso().exc(Exception::SansHorodatage, "ligne fille de `rep_proposition` (la date de la proposition y est portée)"),
    t("rep_proposition_raison", "mise_en_service", Portee::Etablissement).exc(Exception::SansHorodatage, "ligne fille de `rep_proposition` (la date de la proposition y est portée)"),
    t("rep_refus", "mise_en_service", Portee::Etablissement),
    t("rep_synonyme", "mise_en_service", Portee::Etablissement),
    // Socle 27, lot G3a (migration 2544) : guichet des demandes de droits. Catégorie de conservation
    // `rgpd.demande` (clôture + 60 mois, `conservation.rs`). Neutralisation à la restauration : AUCUNE —
    // une demande restaurée est vraie (elle a bien été reçue) et son délai légal court depuis sa réception ;
    // rien n'est à « re-envoyer » (l'accusé de réception part par la file du socle 8, qui a sa propre
    // neutralisation). La pièce d'identité n'est pas en base : seule sa référence au coffre (socle 7) l'est.
    t("rgp_demande", "conformite", Portee::Etablissement).perso(),
    // Compteur de numéros : aucune donnée personnelle, un entier par année.
    t("rgp_demande_compteur", "conformite", Portee::Etablissement),
    t("rgp_demande_droit", "conformite", Portee::Etablissement).perso(),
    t("rgp_demande_identite", "conformite", Portee::Etablissement)
        .perso()
        .fichiers(Fichiers::Reference),
    t("rgp_demande_journal", "conformite", Portee::Etablissement).perso(),
    t("rgp_destinataire_externe", "conformite", Portee::Etablissement),
    t("rgp_document", "conformite", Portee::Etablissement),
    t("rgp_ecart", "conformite", Portee::Etablissement),
    t("rgp_gouvernance", "conformite", Portee::Etablissement).perso(),
    t("rgp_mention", "conformite", Portee::Etablissement),
    t("rgp_soustraitant", "conformite", Portee::Technique),
    t("rgp_soustraitant_annonce", "conformite", Portee::Technique),
    t("rgp_soustraitant_donnee", "conformite", Portee::Technique).exc(Exception::SansHorodatage, "table de liaison de plateforme (clé composite) : l'horodatage est porté par `rgp_soustraitant`"),
    t("rgp_soustraitant_local", "conformite", Portee::Etablissement),
    t("rgp_soustraitant_objection", "conformite", Portee::Etablissement).perso(),
    t("rgp_traitement", "conformite", Portee::Etablissement),
    t("rgp_traitement_version", "conformite", Portee::Etablissement).perso(),
    // Socle 27, palier G5 (migration 2547) : registre des violations. Catégorie `rgpd.violation` (clôture + 60 mois).
    // Neutralisation à la restauration : AUCUNE — une violation restaurée est vraie (elle a bien eu lieu) et son horloge
    // court depuis la prise de connaissance ; les alertes partent par la file du socle 8, qui a sa propre neutralisation.
    t("rgp_violation", "conformite", Portee::Etablissement).perso(),
    t("rgp_violation_categorie", "conformite", Portee::Etablissement),
    // Compteur de numéros : aucune donnée personnelle, un entier par année.
    t("rgp_violation_compteur", "conformite", Portee::Etablissement),
    t("rgp_violation_etape", "conformite", Portee::Etablissement).perso(),
    t("rgp_violation_mesure", "conformite", Portee::Etablissement).perso(),
    t("rgp_violation_notification", "conformite", Portee::Etablissement).perso(),
    t("rgp_violation_personne", "conformite", Portee::Etablissement).perso(),
    // Violation de plateforme : déclarée par l'éditeur, sans établissement ni donnée d'un établissement (P).
    t("rgp_violation_plateforme", "conformite", Portee::Technique),
    t("rgp_violation_plateforme_etab", "conformite", Portee::Etablissement),
];

/// Classement d'une table par nom.
pub fn classer(nom: &str) -> Option<&'static ClasseTable> {
    REGISTRE.iter().find(|c| c.nom == nom)
}

/// Périmètre d'une neutralisation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerimetreNeutralisation {
    /// Toute la base (restauration globale).
    Tout,
    /// Un seul établissement (restauration d'un établissement dans un bac).
    /// Les tables sans `etablissement_id` sont ignorées.
    Etablissement(Uuid),
}

/// Résultat pour une table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LigneNeutralisation {
    pub table: &'static str,
    /// `purge`, `suspendu`, `journal_seul`, `a_verifier`, `ignore_hors_perimetre`.
    pub action: &'static str,
    /// Lignes modifiées / supprimées (`purge`, `suspendu`).
    pub lignes: u64,
    /// Lignes à revoir à la main (`a_verifier`).
    pub a_verifier: i64,
    pub note: &'static str,
}

/// Rapport complet, une ligne par table classée comme file / effet de bord.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RapportNeutralisation {
    pub tables: Vec<LigneNeutralisation>,
}

impl RapportNeutralisation {
    /// Total des lignes suspendues ou supprimées.
    pub fn total_neutralise(&self) -> u64 {
        self.tables.iter().map(|l| l.lignes).sum()
    }
    /// Lignes à revoir à la main (SEPA…).
    pub fn total_a_verifier(&self) -> i64 {
        self.tables.iter().map(|l| l.a_verifier).sum()
    }
}

fn citer(id: &str) -> String {
    format!("\"{}\"", id.replace('"', "\"\""))
}

/// **Suspend toutes les files d'envoi classées** sur une base restaurée (E9,
/// erreur F-014 : une restauration naïve renverrait des messages aux familles).
///
/// À appeler par la procédure de restauration **avant** de démarrer le
/// distributeur, les passerelles courriel / SMS / push et tout planificateur.
/// À exécuter dans une transaction de l'appelant (`conn`) ; idempotente : une
/// seconde exécution ne trouve plus rien à suspendre. Rien n'est supprimé sauf
/// les états éphémères (sessions, défis, codes) : les envois suspendus restent
/// en base, en statut explicite, et un humain peut les rejouer
/// (`rejouer_echec`, nouvelle invitation…).
///
/// Les écarts de schéma (une table du registre absente de la base) sont des
/// erreurs : on ne neutralise pas « à moitié » sans le dire.
pub async fn neutraliser_files_restauration(
    conn: &mut PgConnection,
    perimetre: PerimetreNeutralisation,
) -> Result<RapportNeutralisation, sqlx::Error> {
    let mut rapport = RapportNeutralisation::default();
    for c in REGISTRE.iter().filter(|c| c.est_file_envoi()) {
        let nom = citer(c.nom);
        let filtre = match perimetre {
            PerimetreNeutralisation::Tout => String::new(),
            PerimetreNeutralisation::Etablissement(_) if c.colonne_etablissement => {
                " AND etablissement_id = $1".to_string()
            }
            PerimetreNeutralisation::Etablissement(_) => {
                rapport.tables.push(LigneNeutralisation {
                    table: c.nom,
                    action: "ignore_hors_perimetre",
                    lignes: 0,
                    a_verifier: 0,
                    note: "table sans etablissement_id : hors d'une restauration d'établissement",
                });
                continue;
            }
        };
        let id = match perimetre {
            PerimetreNeutralisation::Etablissement(id) => Some(id),
            PerimetreNeutralisation::Tout => None,
        };
        match c.neutralisation {
            Neutralisation::Aucune => {}
            Neutralisation::JournalSeul(raison) => rapport.tables.push(LigneNeutralisation {
                table: c.nom,
                action: "journal_seul",
                lignes: 0,
                a_verifier: 0,
                note: raison,
            }),
            Neutralisation::Purge => {
                let sql = format!("DELETE FROM {nom} WHERE true{filtre}");
                let mut q = sqlx::query(&sql);
                if let Some(id) = id {
                    q = q.bind(id);
                }
                let n = q.execute(&mut *conn).await?.rows_affected();
                rapport.tables.push(LigneNeutralisation {
                    table: c.nom,
                    action: "purge",
                    lignes: n,
                    a_verifier: 0,
                    note: "état éphémère : jamais réintégré",
                });
            }
            Neutralisation::Maj { set, ou } => {
                let sql = format!("UPDATE {nom} SET {set} WHERE ({ou}){filtre}");
                let mut q = sqlx::query(&sql);
                if let Some(id) = id {
                    q = q.bind(id);
                }
                let n = q.execute(&mut *conn).await?.rows_affected();
                rapport.tables.push(LigneNeutralisation {
                    table: c.nom,
                    action: "suspendu",
                    lignes: n,
                    a_verifier: 0,
                    note: "éléments en attente suspendus, non supprimés",
                });
            }
            Neutralisation::Verification { ou, pourquoi } => {
                let sql = format!("SELECT count(*) AS n FROM {nom} WHERE ({ou}){filtre}");
                let mut q = sqlx::query(&sql);
                if let Some(id) = id {
                    q = q.bind(id);
                }
                let n: i64 = q.fetch_one(&mut *conn).await?.get("n");
                rapport.tables.push(LigneNeutralisation {
                    table: c.nom,
                    action: "a_verifier",
                    lignes: 0,
                    a_verifier: n,
                    note: pourquoi,
                });
            }
        }
    }
    Ok(rapport)
}

/// Tables du domaine « évaluations » : `eva_*`, `ent_eva_*` et, depuis la passe 6, les anciennes tables `ent_note_*` et
/// `ent_devoir_*` (contre-revue du pilote, M-B).
#[cfg(test)]
pub(crate) fn est_table_evaluation(nom: &str) -> bool {
    nom.starts_with("eva_") || nom.starts_with("ent_eva_") || nom.starts_with("ent_note_") || nom.starts_with("ent_devoir_")
}

/// Une justification écrite n'est pas une simple longueur : elle doit dire QUI a reçu ou rédigé ce que la table contient
/// (famille, élève, personne…) et compter au moins quatre mots (suites V5 passe 6, Faible 7).
#[cfg(test)]
pub(crate) fn justification_valable(raison: &str) -> bool {
    let r = raison.trim().to_lowercase();
    let qui = [
        "famille",
        "élève",
        "eleve",
        "personne",
        "demandeur",
        "parent",
    ];
    r.len() >= 20 && r.split_whitespace().count() >= 4 && qui.iter().any(|m| r.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn registre_trie_sans_doublon() {
        let noms: Vec<&str> = REGISTRE.iter().map(|c| c.nom).collect();
        let mut tries = noms.clone();
        tries.sort_unstable();
        tries.dedup();
        assert_eq!(noms, tries, "registre : ordre alphabétique et noms uniques");
    }

    /// Tables d'évaluation exportées sans `export_si`, sans `hors_export`, sans mention « tout exportable » justifiée ni
    /// déclaration « hors extrait par élève » (suites V5 passe 4, M2) : leur export serait un oubli, jamais un choix.
    fn eva_exportees_sans_declaration(registre: &[ClasseTable]) -> Vec<&'static str> {
        registre
            .iter()
            .filter(|c| est_table_evaluation(c.nom))
            .filter(|c| c.export_portabilite)
            .filter(|c| {
                c.export_si.is_none()
                    && c.colonnes_hors_export.is_empty()
                    && c.hors_export_par_eleve
                        .is_none_or(|r| !justification_valable(r))
                    && c.export_integral.is_none_or(|r| !justification_valable(r))
            })
            .map(|c| c.nom)
            .collect()
    }

    #[test]
    fn toute_table_eva_exportee_est_declaree() {
        assert_eq!(
            eva_exportees_sans_declaration(REGISTRE),
            Vec::<&str>::new(),
            "table d'évaluation exportée sans export_si, hors_export ni « tout exportable » justifié"
        );
        // Une mention vide ne vaut pas justification.
        for c in REGISTRE.iter().filter(|c| c.export_integral.is_some()) {
            assert!(c.export_portabilite, "{} : « tout exportable » sur une table non exportée", c.nom);
        }
    }

    /// Test négatif : une table `eva_*` ajoutée à l'export sans déclaration est vue (et une raison vide ne suffit pas).
    #[test]
    fn une_table_eva_exportee_sans_declaration_est_vue() {
        let nue = t("eva_nouvelle_table", "evaluations_pedagogie", Portee::Etablissement).perso().export();
        let vide = t("eva_autre_table", "evaluations_pedagogie", Portee::Etablissement).perso().export().tout_exportable(" ");
        let ok = t("eva_table_ok", "evaluations_pedagogie", Portee::Etablissement)
            .perso()
            .export()
            .tout_exportable("chaque ligne et chaque colonne a été remise à la famille");
        let non_exportee = t("eva_interne", "evaluations_pedagogie", Portee::Etablissement).perso();
        let hors_domaine = t("ent_autre", "evaluations_pedagogie", Portee::Etablissement).perso().export();
        assert_eq!(
            eva_exportees_sans_declaration(&[nue, vide, ok, non_exportee, hors_domaine]),
            vec!["eva_nouvelle_table", "eva_autre_table"]
        );
    }

    /// Une colonne qui porte un ÉTAT (cycle de vie, publication, retrait, décision) : une ligne n'a de sens pour la famille que
    /// dans certains états (suites V5 passe 5, M1).
    fn est_colonne_d_etat(colonne: &str) -> bool {
        colonne.starts_with("etat")
            || colonne.starts_with("statut")
            || [
                "publi",
                "retir",
                "retrait",
                "annul",
                "clotur",
                "supprim",
                "archiv",
                "decide_le",
                "valide_le",
            ]
            .iter()
            .any(|m| colonne.contains(m))
    }

    /// Tables d'évaluation exportées, **à état**, sans `export_si` ni « tout exportable » justifié : `hors_export` (qui ne
    /// retire que des colonnes) ne dit rien des lignes qui ne doivent pas sortir (brouillon, non publié, annulé).
    fn eva_a_etat_sans_decision(registre: &[ClasseTable], colonnes: &BTreeMap<String, Vec<String>>) -> Vec<&'static str> {
        registre
            .iter()
            .filter(|c| est_table_evaluation(c.nom))
            .filter(|c| c.export_portabilite)
            .filter(|c| c.hors_export_par_eleve.is_none())
            .filter(|c| {
                colonnes
                    .get(c.nom)
                    .is_some_and(|cols| cols.iter().any(|x| est_colonne_d_etat(x)))
            })
            .filter(|c| {
                c.export_si.is_none() && c.export_integral.is_none_or(|r| !justification_valable(r))
            })
            .map(|c| c.nom)
            .collect()
    }

    /// Colonnes de chaque table `eva_*` / `ent_eva_*` / `ent_note_*` de la base.
    async fn colonnes_des_tables(pool: &sqlx::PgPool) -> BTreeMap<String, Vec<String>> {
        let lignes: Vec<(String, String)> = sqlx::query_as(
            r"SELECT table_name::text, column_name::text FROM information_schema.columns
              WHERE table_schema = 'public' AND (table_name LIKE 'eva\_%' OR table_name LIKE 'ent\_eva\_%' OR table_name LIKE 'ent\_note\_%')",
        )
        .fetch_all(pool)
        .await
        .expect("colonnes");
        let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (t, c) in lignes {
            m.entry(t).or_default().push(c);
        }
        m
    }

    /// M1 : `hors_export` seul n'est pas une décision pour une table à état. Échec franc, avec la liste.
    #[tokio::test]
    async fn toute_table_eva_a_etat_exportee_a_une_condition_ou_une_justification() {
        let Some(pool) = base().await else {
            return eprintln!("SAUT : ENT_DATABASE_URL absente, la garde M1 (tables à état exportées) n'a PAS été exécutée");
        };
        let colonnes = colonnes_des_tables(&pool).await;
        // La garde ne doit pas être aveugle : les tables d'état connues sont bien vues comme telles.
        for connue in [
            "eva_rattrapage",
            "eva_moyenne_statut",
            "eva_appreciation",
            "eva_moyenne_externe",
            "ent_note_archive",
        ] {
            assert!(
                colonnes.get(connue).is_some_and(|cols| cols.iter().any(|x| est_colonne_d_etat(x))),
                "{connue} doit être reconnue comme table à état"
            );
        }
        assert_eq!(
            eva_a_etat_sans_decision(REGISTRE, &colonnes),
            Vec::<&str>::new(),
            "table d'évaluation à état exportée sans export_si ni « tout exportable » justifié (hors_export seul ne suffit pas)"
        );
    }

    /// Négatif de M1 : `hors_export` seul sur une table à état est vu ; `export_si` ou une raison écrite VALABLE suffit (une
    /// simple longueur ne suffit pas : passe 6, Faible 7) ; une table sans colonne d'état n'est pas concernée ; une table
    /// `ent_note_*` est suivie comme une table `eva_*` ; une table hors extrait par élève n'est pas concernée.
    #[test]
    fn hors_export_seul_sur_une_table_a_etat_est_vu() {
        let seul = t("eva_a", "evaluations_pedagogie", Portee::Etablissement).perso().export().hors_export(&["par"]);
        let cond = t("eva_b", "evaluations_pedagogie", Portee::Etablissement).perso().export().hors_export(&["par"]).export_si("x.etat = 'ok'");
        let raison = t("eva_c", "evaluations_pedagogie", Portee::Etablissement)
            .perso()
            .export()
            .tout_exportable("demande de la famille : tous les états lui ont été communiqués");
        let vide = t("eva_d", "evaluations_pedagogie", Portee::Etablissement).perso().export().tout_exportable("  ");
        let sans_etat = t("eva_e", "evaluations_pedagogie", Portee::Etablissement).perso().export().hors_export(&["par"]);
        let longue_creuse = t("eva_f", "evaluations_pedagogie", Portee::Etablissement)
            .perso()
            .export()
            .tout_exportable("rien à signaler dans cette table, tout va bien");
        let ancienne = t("ent_note_x", "evaluations_pedagogie", Portee::Etablissement)
            .perso()
            .export()
            .hors_export(&["par"]);
        let hors_eleve = t("ent_note_y", "evaluations_pedagogie", Portee::Etablissement)
            .perso()
            .export()
            .hors_export_par_eleve(
                "registre de l'établissement : aucun élève, jamais dans l'extrait d'un élève",
            );
        let mut cols = BTreeMap::new();
        for n in [
            "eva_a",
            "eva_b",
            "eva_c",
            "eva_d",
            "eva_f",
            "ent_note_x",
            "ent_note_y",
        ] {
            cols.insert(n.to_string(), vec!["id".to_string(), "etat".to_string()]);
        }
        cols.insert("eva_e".to_string(), vec!["id".to_string(), "par".to_string()]);
        assert_eq!(
            eva_a_etat_sans_decision(
                &[
                    seul,
                    cond,
                    raison,
                    vide,
                    sans_etat,
                    longue_creuse,
                    ancienne,
                    hors_eleve
                ],
                &cols
            ),
            vec!["eva_a", "eva_d", "eva_f", "ent_note_x"]
        );
        // Contenu de la justification : qui a reçu ou rédigé ?
        assert!(justification_valable(
            "message de la famille et réponse qui lui a été adressée"
        ));
        assert!(!justification_valable("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(!justification_valable(
            "une raison assez longue mais sans destinataire"
        ));
        assert!(!justification_valable("famille"));
        // Détection d'état par le nom : au-delà de `etat` / `statut` / `publiee_le`.
        for col in [
            "etat",
            "statut_ligne",
            "publiee_le",
            "retrait_le",
            "annule_le",
            "cloture_le",
            "archivee_le",
            "supprime_le",
        ] {
            assert!(est_colonne_d_etat(col), "{col}");
        }
        assert!(!est_colonne_d_etat("cree_le") && !est_colonne_d_etat("valeur"));
    }

    /// Faible 9 (passe 6) : la table de reprise des appréciations n'est pas exportée ; et si quelqu'un l'exporte un jour sans
    /// condition, la garde de déclaration (étendue aux `ent_note_*`) le voit — le test éprouve la garde, pas seulement le registre.
    #[test]
    fn la_table_de_reprise_des_appreciations_n_est_pas_exportee_sans_condition() {
        let c = *classer("ent_note_appreciation").expect("classée");
        assert!(
            !c.export_portabilite,
            "ent_note_appreciation : retirée de l'export jusqu'à G3 (F4)"
        );
        assert!(
            c.export_si.is_none(),
            "ent_note_appreciation : une condition d'export suppose un export décidé"
        );
        // Mutation : exportée sans rien d'autre → la garde la signale.
        let mut exportee = c;
        exportee.export_portabilite = true;
        assert_eq!(
            eva_exportees_sans_declaration(&[exportee]),
            vec!["ent_note_appreciation"]
        );
        // Exportée avec une condition → la garde ne la signale plus (la condition devient l'affaire de la revue).
        exportee.export_si = Some("x.etat = 'validee'");
        assert!(eva_exportees_sans_declaration(&[exportee]).is_empty());
    }

    /// Passe 6 (Moyen 5) : les tables `ent_note_*` du journal et des rebuts ne sont pas exportées ; l'archive des notes l'est
    /// sous condition de publication du devoir historique, sans l'auteur ; le registre des périodes et le journal d'export du
    /// personnel sont déclarés hors extrait par élève (avec une raison écrite valable).
    #[test]
    fn les_tables_ent_note_et_le_journal_du_personnel_sont_classes() {
        for nom in [
            "ent_note_correction",
            "ent_note_doublon_archive",
            "ent_note_periode_journal",
            "ent_note_appreciation",
        ] {
            assert!(
                !classer(nom).expect("classée").export_portabilite,
                "{nom} : interne, jamais exportée"
            );
        }
        let archive = classer("ent_note_archive").expect("classée");
        assert!(
            archive.export_portabilite && archive.export_si.is_some(),
            "ent_note_archive : export conditionné"
        );
        for col in ["auteur_user_id", "updated_by_user_id"] {
            assert!(
                archive.colonnes_hors_export.contains(&col),
                "ent_note_archive : {col} hors export"
            );
        }
        for nom in ["ent_note_periode", "ent_eva_export"] {
            let raison = classer(nom).expect("classée").hors_export_par_eleve;
            assert!(
                raison.is_some_and(justification_valable_hors_eleve),
                "{nom} : hors extrait par élève, raison écrite"
            );
        }
    }

    /// Colonnes qui rattachent une ligne à une personne (élève, famille, compte) : une table qui en porte une ne peut pas être
    /// déclarée « hors extrait par élève » (contre-revue du pilote, passe 6, M-A).
    const COLONNES_DE_RATTACHEMENT_A_UNE_PERSONNE: &[&str] = &[
        "resultat_id",
        "enseignement_externe_id",
        "rattrapage_id",
        "user_id",
        "personne_id",
        "beneficiaire_id",
        "enfant_id",
        "responsable_id",
    ];

    /// Raison d'une table hors extrait par élève : au moins quatre mots et elle dit pourquoi aucun élève n'y est rattaché.
    pub(crate) fn justification_valable_hors_eleve(raison: &str) -> bool {
        let r = raison.trim().to_lowercase();
        r.len() >= 20
            && r.split_whitespace().count() >= 4
            && (r.contains("élève") || r.contains("eleve"))
    }

    /// Une table « hors extrait par élève » est exportable pour la personne, sa raison est écrite et la BASE confirme qu'elle
    /// n'a aucune colonne `eleve_id` (sinon la déclaration cacherait des lignes d'élève).
    #[tokio::test]
    async fn hors_export_par_eleve_est_justifie_et_sans_eleve() {
        for c in REGISTRE
            .iter()
            .filter(|c| c.hors_export_par_eleve.is_some())
        {
            assert!(
                c.export_portabilite,
                "{} : « hors extrait par élève » suppose une table exportable",
                c.nom
            );
            assert!(
                c.hors_export_par_eleve
                    .is_some_and(justification_valable_hors_eleve),
                "{} : raison « hors extrait par élève » insuffisante",
                c.nom
            );
        }
        let Some(pool) = base().await else {
            // M-A : en CI (GitHub Actions) la base existe toujours : la vérification ne doit jamais y sauter en silence.
            assert!(
                std::env::var("GITHUB_ACTIONS").is_err(),
                "CI sans base : la vérification « sans eleve_id » n'a pas été exécutée"
            );
            return eprintln!("SAUT : ENT_DATABASE_URL absente, la vérification « sans eleve_id » n'a PAS été exécutée");
        };
        let colonnes = colonnes_des_tables(&pool).await;
        for c in REGISTRE
            .iter()
            .filter(|c| c.hors_export_par_eleve.is_some())
        {
            let cols = colonnes
                .get(c.nom)
                .unwrap_or_else(|| panic!("{} : table introuvable", c.nom));
            // Contre-revue du pilote (passe 6, M-A) : toute clé qui rattache une ligne à une personne compte, pas seulement
            // `eleve_id` (l'assembleur de l'extrait rattache aussi par résultat, enseignement externe ou rattrapage).
            assert!(
                !cols.iter().any(|x| x.starts_with("eleve")
                    || COLONNES_DE_RATTACHEMENT_A_UNE_PERSONNE.contains(&x.as_str())),
                "{} : déclarée hors extrait par élève mais porte une colonne d'élève ({cols:?})",
                c.nom
            );
        }
    }

    /// M4 / M5 / F3 : ce qui n'est pas communiqué à la famille n'est pas exporté.
    #[test]
    fn les_tables_internes_de_rattrapage_et_de_declaration_ne_sont_pas_exportees() {
        for nom in ["eva_rattrapage_historique", "eva_rattrapage_autorise", "eva_dispense_port"] {
            let c = classer(nom).expect("classée");
            assert!(!c.export_portabilite, "{nom} ne doit pas être exportée (D-069)");
        }
        // Contre-revue du pilote (Moyen 1) : l'enseignement externe sort avec une moyenne publiée (sa matière et son
        // organisme donnent son sens à la moyenne), jamais sans condition ni avec le référent, l'auteur ou la décision interne.
        let ens = classer("eva_enseignement_externe").expect("classée");
        assert!(ens.export_portabilite && ens.export_si.is_some(), "eva_enseignement_externe : export conditionné");
        for col in ["referent", "declare_par", "compte_moyenne_generale"] {
            assert!(ens.colonnes_hors_export.contains(&col), "eva_enseignement_externe : {col} hors export");
        }
        for nom in ["eva_rattrapage", "eva_moyenne_statut"] {
            assert!(classer(nom).expect("classée").export_si.is_some(), "{nom} : condition d'état attendue");
        }
    }

    #[test]
    fn regles_de_coherence_internes() {
        for c in REGISTRE {
            if c.portee == Portee::Etablissement {
                assert!(c.colonne_etablissement, "{} : portée établissement => colonne", c.nom);
            }
            if c.sante {
                assert!(c.donnees_personnelles, "{} : santé => données personnelles", c.nom);
            }
            if matches!(c.fichiers, Fichiers::Bytea | Fichiers::ObjetAvecResidu) {
                assert!(c.donnees_personnelles || c.nom.ends_with("_piece"), "{}", c.nom);
            }
            // E9 : tout ce qui ressemble à une file d'envoi / jeton doit être déclaré.
            let suspect = ["outbox", "envoi", "invitation", "verification"]
                .iter()
                .any(|m| c.nom.contains(m));
            if suspect {
                assert!(
                    c.est_file_envoi(),
                    "{} ressemble à une file d'envoi : la déclarer « à neutraliser » (E9)",
                    c.nom
                );
            }
            // Un état éphémère n'est pas exportable.
            if c.retention == Retention::Ephemere {
                assert!(!c.export_portabilite, "{} : éphémère => pas d'export", c.nom);
            }
        }
    }

    struct Colonnes {
        etab_present: bool,
        etab_not_null: bool,
        rls: bool,
        horodatage: bool,
        bytea: bool,
        storage_key: bool,
    }

    async fn catalogue(conn: &mut PgConnection) -> Vec<(String, Colonnes)> {
        let lignes = sqlx::query(
            r#"
            SELECT c.relname AS nom,
                   EXISTS (SELECT 1 FROM pg_attribute a WHERE a.attrelid = c.oid AND a.attnum > 0
                           AND NOT a.attisdropped AND a.attname = 'etablissement_id') AS etab,
                   EXISTS (SELECT 1 FROM pg_attribute a WHERE a.attrelid = c.oid AND a.attnum > 0
                           AND NOT a.attisdropped AND a.attname = 'etablissement_id'
                           AND a.attnotnull) AS etab_nn,
                   c.relrowsecurity AS rls,
                   EXISTS (SELECT 1 FROM pg_attribute a WHERE a.attrelid = c.oid AND a.attnum > 0
                           AND NOT a.attisdropped
                           AND format_type(a.atttypid, a.atttypmod) LIKE 'timestamp%') AS horo,
                   EXISTS (SELECT 1 FROM pg_attribute a WHERE a.attrelid = c.oid AND a.attnum > 0
                           AND NOT a.attisdropped AND a.atttypid = 'bytea'::regtype) AS bytea,
                   EXISTS (SELECT 1 FROM pg_attribute a WHERE a.attrelid = c.oid AND a.attnum > 0
                           AND NOT a.attisdropped AND a.attname = 'storage_key') AS skey
            FROM pg_class c
            WHERE c.relnamespace = 'public'::regnamespace AND c.relkind IN ('r', 'p')
            ORDER BY c.relname
            "#,
        )
        .fetch_all(&mut *conn)
        .await
        .expect("catalogue");
        lignes
            .iter()
            .map(|r| {
                (
                    r.get::<String, _>("nom"),
                    Colonnes {
                        etab_present: r.get("etab"),
                        etab_not_null: r.get("etab_nn"),
                        rls: r.get("rls"),
                        horodatage: r.get("horo"),
                        bytea: r.get("bytea"),
                        storage_key: r.get("skey"),
                    },
                )
            })
            .collect()
    }

    /// Base locale de test migrée (sans `PgEnt` : pas de configuration passkey requise).
    async fn base() -> Option<sqlx::PgPool> {
        let url = crate::url_guard::url_labo_ou_echec()?;
        crate::url_guard::assert_local_postgres_url(&url).expect("base locale");
        let pool = sqlx::PgPool::connect(&url).await.expect("connect");
        crate::migrate::apply_socle_migrations(&pool).await.expect("migrate");
        Some(pool)
    }

    #[tokio::test]
    async fn chaque_table_de_la_base_est_classee_et_conforme() {
        let Some(pool) = base().await else { return };
        let mut conn = pool.acquire().await.expect("conn");
        let cat = catalogue(&mut conn).await;

        let en_base: BTreeSet<&str> = cat.iter().map(|(n, _)| n.as_str()).collect();
        let classees: BTreeSet<&str> = REGISTRE.iter().map(|c| c.nom).collect();
        let non_classees: Vec<_> = en_base.difference(&classees).collect();
        let obsoletes: Vec<_> = classees.difference(&en_base).collect();
        assert!(
            non_classees.is_empty(),
            "tables NON classées dans registre_donnees.rs (D-037, E12) : {non_classees:?}"
        );
        assert!(obsoletes.is_empty(), "entrées du registre sans table : {obsoletes:?}");

        let mut erreurs: Vec<String> = Vec::new();
        for (nom, col) in &cat {
            let c = classer(nom).expect("classée");
            if c.colonne_etablissement != col.etab_present {
                erreurs.push(format!(
                    "{nom} : colonne etablissement_id {} en base, {} au registre",
                    col.etab_present, c.colonne_etablissement
                ));
            }
            let nullable = col.etab_present && !col.etab_not_null;
            let etab_ok = col.etab_present && !nullable;
            let sans_etab_declare = c.exception(Exception::EtabNullable);
            if c.portee == Portee::Etablissement {
                if !etab_ok && sans_etab_declare.is_none() {
                    erreurs.push(format!("{nom} : « par établissement » sans etablissement_id NOT NULL"));
                }
                if etab_ok && sans_etab_declare.is_some() {
                    erreurs.push(format!("{nom} : exception EtabNullable inutile (retirer)"));
                }
                if !col.rls && c.exception(Exception::SansRls).is_none() {
                    erreurs.push(format!("{nom} : « par établissement » sans RLS"));
                }
                if col.rls && c.exception(Exception::SansRls).is_some() {
                    erreurs.push(format!("{nom} : exception SansRls inutile (RLS posée, retirer)"));
                }
            } else {
                if sans_etab_declare.is_some() || c.exception(Exception::SansRls).is_some() {
                    erreurs.push(format!("{nom} : exceptions établissement sans objet hors portée établissement"));
                }
            }
            if !col.horodatage && c.exception(Exception::SansHorodatage).is_none() {
                erreurs.push(format!("{nom} : aucun horodatage et pas d'exception déclarée (E6)"));
            }
            if col.horodatage && c.exception(Exception::SansHorodatage).is_some() {
                erreurs.push(format!("{nom} : exception SansHorodatage inutile (retirer)"));
            }
            if col.bytea && !matches!(c.fichiers, Fichiers::Bytea | Fichiers::ObjetAvecResidu) {
                erreurs.push(format!("{nom} : colonne BYTEA non déclarée (Fichiers::Bytea, E5)"));
            }
            if !col.bytea && c.fichiers == Fichiers::Bytea {
                erreurs.push(format!("{nom} : déclarée BYTEA mais plus de colonne BYTEA (mettre à jour)"));
            }
            if c.fichiers == Fichiers::ObjetAvecResidu && !(col.bytea && col.storage_key) {
                erreurs.push(format!("{nom} : « objet avec résidu » sans colonne BYTEA ou sans storage_key (passer en Objet)"));
            }
            if col.storage_key
                && !matches!(c.fichiers, Fichiers::Objet | Fichiers::Bytea | Fichiers::ObjetAvecResidu)
            {
                erreurs.push(format!("{nom} : colonne storage_key non déclarée (Fichiers::Objet)"));
            }
        }
        assert!(erreurs.is_empty(), "registre non conforme :\n{}", erreurs.join("\n"));
    }

    /// Liste lisible des tables non conformes (dette déclarée) : exécutée pour
    /// que le décompte reste vérifié par la CI et visible avec `--nocapture`.
    #[test]
    fn dette_declaree_comptee() {
        let sans_rls = REGISTRE.iter().filter(|c| c.exception(Exception::SansRls).is_some()).count();
        let sans_horo = REGISTRE.iter().filter(|c| c.exception(Exception::SansHorodatage).is_some()).count();
        let etab_nullable = REGISTRE.iter().filter(|c| c.exception(Exception::EtabNullable).is_some()).count();
        let hors_etab = REGISTRE
            .iter()
            .filter(|c| !c.colonne_etablissement)
            .count();
        println!(
            "dette déclarée : {sans_rls} sans RLS, {sans_horo} sans horodatage, {etab_nullable} établissement nullable, {hors_etab} sans etablissement_id sur {} tables",
            REGISTRE.len()
        );
        assert!(REGISTRE.len() > 300);
    }

    /// Toutes les actions de neutralisation s'exécutent sur le schéma réel
    /// (colonnes et statuts valides), dans une transaction annulée.
    #[tokio::test]
    async fn neutralisation_s_execute_sur_toute_la_base_sans_effet_persistant() {
        let Some(pool) = base().await else { return };
        let mut tx = pool.begin().await.expect("tx");
        let r = neutraliser_files_restauration(&mut tx, PerimetreNeutralisation::Tout)
            .await
            .expect("neutralisation globale");
        let attendues = REGISTRE.iter().filter(|c| c.est_file_envoi()).count();
        assert_eq!(r.tables.len(), attendues, "une ligne de rapport par table classée");
        tx.rollback().await.expect("rollback");

        let mut tx = pool.begin().await.expect("tx");
        let id = Uuid::new_v4();
        let r = neutraliser_files_restauration(&mut tx, PerimetreNeutralisation::Etablissement(id))
            .await
            .expect("neutralisation d'un établissement");
        assert_eq!(r.total_neutralise(), 0);
        tx.rollback().await.expect("rollback");
    }

    /// Cas réel : files remplies sur un établissement ; après neutralisation, plus
    /// rien d'envoyable, les autres établissements sont intacts, et une seconde
    /// exécution ne trouve plus rien (idempotence).
    #[tokio::test]
    async fn neutralisation_suspend_les_files_d_un_etablissement() {
        let Some(pool) = base().await else { return };
        let mut tx = pool.begin().await.expect("tx");
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let user = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_user (id, email, password_hash) VALUES ($1, $2, 'x')")
            .bind(user)
            .bind(format!("{user}@exemple.test"))
            .execute(&mut *tx)
            .await
            .expect("user");

        for etab in [a, b] {
            // courriel / notification en attente
            sqlx::query(
                "INSERT INTO ent_notification_envoi
                   (id, etablissement_id, destinataire_user_id, canal, kind, titre, corps,
                    source_type, source_id, cle_dedup, statut)
                 VALUES ($1, $2, $3, 'email', 'test', 't', 'c', 'systeme', $2, $4, 'en_attente'),
                        ($5, $2, $3, 'sms', 'test', 't', 'c', 'systeme', $2, $6, 'envoi')",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(user)
            .bind(format!("k1-{}", Uuid::new_v4()))
            .bind(Uuid::new_v4())
            .bind(format!("k2-{}", Uuid::new_v4()))
            .execute(&mut *tx)
            .await
            .expect("notification_envoi");
            // courriel d'inscription à envoyer
            sqlx::query(
                "INSERT INTO ent_inscription_envoi
                   (id, etablissement_id, cle, kind, destinataire, sujet, corps, statut)
                 VALUES ($1, $2, $3, 'information', 'a@exemple.test', 's', 'c', 'a_envoyer')",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(format!("cle-{}", Uuid::new_v4()))
            .execute(&mut *tx)
            .await
            .expect("inscription_envoi");
            // session : état éphémère
            let _ = etab;
        }

        let r = neutraliser_files_restauration(&mut tx, PerimetreNeutralisation::Etablissement(a))
            .await
            .expect("neutralisation");
        let ligne = |t: &str| r.tables.iter().find(|l| l.table == t).expect("ligne").clone();
        assert_eq!(ligne("ent_notification_envoi").lignes, 2);
        assert_eq!(ligne("ent_notification_envoi").action, "suspendu");
        assert_eq!(ligne("ent_inscription_envoi").lignes, 1);
        assert_eq!(ligne("ent_mail_outbox").action, "journal_seul");

        let restant = |etab: Uuid| {
            let q = sqlx::query_scalar::<_, i64>(
                "SELECT (SELECT count(*) FROM ent_notification_envoi
                          WHERE etablissement_id = $1 AND statut IN ('en_attente', 'envoi'))
                      + (SELECT count(*) FROM ent_inscription_envoi
                          WHERE etablissement_id = $1 AND statut = 'a_envoyer')",
            )
            .bind(etab);
            q
        };
        assert_eq!(restant(a).fetch_one(&mut *tx).await.unwrap(), 0, "A neutralisé");
        assert_eq!(restant(b).fetch_one(&mut *tx).await.unwrap(), 3, "B intact");

        // Idempotence.
        let r2 = neutraliser_files_restauration(&mut tx, PerimetreNeutralisation::Etablissement(a))
            .await
            .expect("2e passage");
        assert_eq!(r2.total_neutralise(), 0);

        // Les éléments suspendus sont conservés avec un statut explicite.
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_notification_envoi
              WHERE etablissement_id = $1 AND statut = 'ignore'
                AND motif = 'neutralise_restauration'",
        )
        .bind(a)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(n, 2);
        tx.rollback().await.expect("rollback");
    }
}
