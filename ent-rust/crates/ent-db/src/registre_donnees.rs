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
const N_OUTBOX: Neutralisation = Neutralisation::Maj {
    set: "statut = 'echec', derniere_erreur = 'neutralise_restauration'",
    ou: "statut = 'a_distribuer'",
};
const N_NOTIF: Neutralisation = Neutralisation::Maj {
    set: "statut = 'ignore', motif = 'neutralise_restauration'",
    ou: "statut IN ('en_attente', 'envoi')",
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

/// Le registre. Ordre alphabétique (aligné sur le catalogue).
pub const REGISTRE: &[ClasseTable] = &[
    t("ent_absence", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_absence_declaration", "assiduite", Portee::Etablissement).perso().sante().fichiers(Fichiers::Reference).export(),
    t("ent_absence_motif", "assiduite", Portee::ReferentielGlobal).exc(Exception::SansHorodatage, "référentiel statique (code, libellé), jamais modifié par un établissement"),
    t("ent_absence_motif_etab", "assiduite", Portee::Etablissement),
    t("ent_absence_rapprochement", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_absence_rh", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_absence_seance", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_absence_signal", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_adresse_protegee", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_aesh_affectation", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_affectation", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_affectation_supprimee", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
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
    t("ent_appel_ligne", "assiduite", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : daté par l'appel parent (ent_appel)"),
    t("ent_appel_relance", "assiduite", Portee::Etablissement).perso().export(),
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
    t("ent_cdt_historique", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_cdt_piece", "evaluations_pedagogie", Portee::Etablissement).perso().fichiers(Fichiers::Objet).export(),
    t("ent_cdt_visa", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
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
    t("ent_cours", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_cours_a_traiter", "emploi_du_temps_structure", Portee::Etablissement).perso(),
    t("ent_cours_membre", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_creneau_rdv", "emploi_du_temps_structure", Portee::Etablissement).perso().neutralise(N_RDV),
    t("ent_delegation", "identite_acces", Portee::Etablissement).perso().export(),
    t("ent_delegation_droit", "identite_acces", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par ent_delegation (D-036 : droits normalisés)"),
    t("ent_delegation_droit_cible", "identite_acces", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par ent_delegation (D-036 : cibles des droits)"),
    t("ent_devoir", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_dispense_vs", "assiduite", Portee::Etablissement).perso().sante().export(),
    t("ent_disponibilite", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_doc_a_signer", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_document_grant", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_edition_vs", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_edt_annulation_masse", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_calendrier_ab", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_edt_contrainte", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_edt_creneau", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_deplacement", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_exception", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_edt_import", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_edt_seance", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_eleve", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_bourse", "finances", Portee::Etablissement).perso().export(),
    t("ent_eleve_capacite", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_eleve_compte", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_confidentialite", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_contact_urgence", "dossier_eleve", Portee::Etablissement).perso().export(),
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
    t("ent_eleve_fusion_ligne", "dossier_eleve", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par ent_eleve_fusion"),
    t("ent_eleve_grille_repas", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_eleve_identifiant", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_identite", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_eleve_identite_a_traiter", "dossier_eleve", Portee::Etablissement).perso(),
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
    t("ent_eleve_proposition", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_eleve_proposition_champ", "dossier_eleve", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par la proposition"),
    t("ent_eleve_scolarisation", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_scolarite_annee", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_eleve_scolarite_ecart", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_email_change", "identite_acces", Portee::Technique).perso().neutralise(N_PURGE).avec_etab(),
    t("ent_entreprise", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_equipe_reunion", "emploi_du_temps_structure", Portee::Etablissement).perso().export(),
    t("ent_etablissement", "etablissement", Portee::Technique),
    t("ent_etablissement_domaine", "etablissement", Portee::Etablissement).exc(Exception::SansRls, "résolution hôte → établissement (D-012) : lue avant de connaître l'établissement"),
    t("ent_etablissement_parametre", "etablissement", Portee::Etablissement),
    t("ent_etablissement_profil", "etablissement", Portee::Etablissement),
    t("ent_eva_coef_matiere", "evaluations_pedagogie", Portee::Etablissement),
    t("ent_eva_export", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
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
    t("ent_fin_compte_payeur", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_compteur", "finances", Portee::Etablissement).exc(Exception::SansHorodatage, "compteur technique (numérotation), valeur seule"),
    t("ent_fin_echeancier_type", "finances", Portee::Etablissement).exc(Exception::SansHorodatage, "paramétrage financier sans historique"),
    t("ent_fin_export", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_export_piece", "finances", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par ent_fin_export"),
    t("ent_fin_lettrage", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_palier_relance", "finances", Portee::Etablissement).exc(Exception::SansHorodatage, "paramétrage financier sans historique"),
    t("ent_fin_parametres", "finances", Portee::Etablissement),
    t("ent_fin_prenotification", "finances", Portee::Etablissement).perso().neutralise(N_PRENOT),
    t("ent_fin_reduction", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_reduction_eleve", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_reglement", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_rejet", "finances", Portee::Etablissement).perso().export(),
    t("ent_fin_relance", "finances", Portee::Etablissement).perso().neutralise(Neutralisation::JournalSeul("les relances sont proposées puis décidées par un humain : aucun envoi automatique")),
    t("ent_fin_tarif", "finances", Portee::Etablissement),
    t("ent_fonction", "personnels", Portee::Etablissement).exc(Exception::EtabNullable, "fonctions nationales/communes sans établissement (etablissement_id NULL autorisé)"),
    t("ent_formulaire", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_formulaire_reponse", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_foyer", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_foyer_iban_histo", "finances", Portee::Etablissement).perso(),
    t("ent_foyer_quotient", "finances", Portee::Etablissement).perso().export(),
    t("ent_grille", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_grille_cible", "emploi_du_temps_structure", Portee::Etablissement).exc(Exception::SansHorodatage, "table de liaison (grille ↔ cible), datée par la grille"),
    t("ent_grille_creneau", "emploi_du_temps_structure", Portee::Etablissement).exc(Exception::SansHorodatage, "ligne fille : datée par la grille horaire"),
    t("ent_grille_horaire", "emploi_du_temps_structure", Portee::Etablissement),
    t("ent_groupe", "structure_scolaire", Portee::Etablissement),
    t("ent_groupe_responsable", "structure_scolaire", Portee::Etablissement).perso().export(),
    t("ent_identite_journal", "dossier_eleve", Portee::Etablissement).perso(),
    t("ent_import_ligne", "dossier_eleve", Portee::Etablissement).perso().export(),
    t("ent_import_lot", "dossier_eleve", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_incident", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_incident_type", "vie_scolaire", Portee::Etablissement),
    t("ent_infirmerie_passage", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_info_comp_champ", "dossier_personnes", Portee::Etablissement),
    t("ent_info_comp_valeur", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_inscription_campagne", "inscriptions", Portee::Etablissement),
    t("ent_inscription_campagne_niveau", "inscriptions", Portee::Etablissement).exc(Exception::SansHorodatage, "ligne fille : datée par la campagne d'inscription"),
    t("ent_inscription_campagne_piece", "inscriptions", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par la campagne d'inscription"),
    t("ent_inscription_campagne_version", "inscriptions", Portee::Etablissement),
    t("ent_inscription_decision", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_declaration_responsable", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_demarche", "inscriptions", Portee::Etablissement),
    t("ent_inscription_dossier", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_envoi", "inscriptions", Portee::Etablissement).perso().neutralise(N_INSENVOI).exc(Exception::SansRls, "worker d'envoi des e-mails d'inscription : relève les envois dus de tous les établissements en un lot (`FOR UPDATE SKIP LOCKED`)"),
    t("ent_inscription_fermeture", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_invitation", "inscriptions", Portee::Etablissement).perso().neutralise(N_INSINV).exc(Exception::SansRls, "jeton d'invitation résolu avant de connaître l'établissement (lien reçu par e-mail) ; relue aussi par le worker d'envoi transverse"),
    t("ent_inscription_lien_attente", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_offre", "inscriptions", Portee::Etablissement),
    t("ent_inscription_offre_historique", "inscriptions", Portee::Etablissement),
    t("ent_inscription_offre_unite", "inscriptions", Portee::Etablissement).exc(Exception::SansHorodatage, "ligne fille : datée par l'offre"),
    t("ent_inscription_parametre_lien", "inscriptions", Portee::Etablissement),
    t("ent_inscription_piece", "inscriptions", Portee::Etablissement).perso().fichiers(Fichiers::Reference).export(),
    t("ent_inscription_prolongation", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_reference_compteur", "inscriptions", Portee::Etablissement).exc(Exception::SansHorodatage, "compteur technique (références de dossier), valeur seule"),
    t("ent_inscription_reprise_rapport", "inscriptions", Portee::Etablissement).perso(),
    t("ent_inscription_resolution", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_responsable", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_suivi", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_suivi_dossier", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_inscription_voeu", "inscriptions", Portee::Etablissement).perso().export(),
    t("ent_internat_affectation", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_chambre", "vie_pratique", Portee::Etablissement),
    t("ent_internat_presence", "vie_pratique", Portee::Etablissement).perso().export(),
    t("ent_internat_suivi", "vie_pratique", Portee::Etablissement).perso().export(),
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
    t("ent_mission", "personnels", Portee::Etablissement).perso().export(),
    t("ent_mission_perimetre", "personnels", Portee::Etablissement).perso().export(),
    t("ent_mission_type", "personnels", Portee::Etablissement).exc(Exception::EtabNullable, "types de missions fournis par ScolIA (etablissement_id NULL autorisé), copiés ou désactivés par établissement"),
    t("ent_module_etablissement", "etablissement", Portee::Etablissement),
    t("ent_msg", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_accuse", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_fil", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_lien_decision", "messagerie", Portee::Etablissement).perso().export(),
    t("ent_msg_modele", "messagerie", Portee::Etablissement),
    t("ent_msg_piece", "messagerie", Portee::Etablissement).perso().fichiers(Fichiers::ObjetAvecResidu).export(),
    t("ent_msg_reglage", "messagerie", Portee::Etablissement),
    t("ent_niveau", "structure_scolaire", Portee::Etablissement),
    t("ent_note", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_note_appreciation", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_note_correction", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_note_doublon_archive", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_note_periode", "evaluations_pedagogie", Portee::Etablissement).perso().export(),
    t("ent_note_periode_journal", "evaluations_pedagogie", Portee::Etablissement).perso(),
    t("ent_notification", "communication_technique", Portee::Etablissement).perso().export(),
    t("ent_notification_envoi", "communication_technique", Portee::Etablissement).perso().neutralise(N_NOTIF),
    t("ent_notification_plafond", "communication_technique", Portee::Etablissement).perso(),
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
    t("ent_personne_preference", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_preference_version", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_reserve", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_personne_version", "dossier_personnes", Portee::Etablissement).perso().export(),
    t("ent_politique_securite", "identite_acces", Portee::Etablissement),
    t("ent_preference_notification", "communication_technique", Portee::Etablissement).perso().export(),
    t("ent_preinscription_verification", "inscriptions", Portee::Etablissement).perso().neutralise(N_PURGE).exc(Exception::SansRls, "jeton de vérification d'e-mail résolu avant de connaître l'établissement (lien reçu par e-mail)"),
    t("ent_procedure_disciplinaire", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_profil_personnel", "identite_acces", Portee::CompteMultiEtablissement).perso().export(),
    t("ent_psy_dossier", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_psy_entretien", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_psy_rdv", "sante", Portee::Etablissement).perso().sante().export(),
    t("ent_punition", "vie_scolaire", Portee::Etablissement).perso().export(),
    t("ent_punition_type", "vie_scolaire", Portee::Etablissement),
    t("ent_push_device", "communication_technique", Portee::Etablissement).perso().export(),
    t("ent_push_outbox", "communication_technique", Portee::Etablissement).perso().neutralise(Neutralisation::JournalSeul("journal des envois déjà tentés (dry_run/sent/failed) : aucun état « à envoyer », rien à suspendre")),
    t("ent_rate_limit", "identite_acces", Portee::Technique).neutralise(N_PURGE),
    t("ent_recuperation_passkey", "identite_acces", Portee::Technique).perso().neutralise(N_PURGE).avec_etab(),
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
    t("ent_signalement_assiduite_detail", "assiduite", Portee::Etablissement).perso().export().exc(Exception::SansHorodatage, "ligne fille : datée par ent_signalement_assiduite (D-036 : détail des demi-journées)"),
    t("ent_site", "etablissement", Portee::Etablissement),
    t("ent_sms_outbox", "communication_technique", Portee::Etablissement).perso().neutralise(Neutralisation::JournalSeul("journal des envois déjà tentés (dry_run/sent/failed) : aucun état « à envoyer », rien à suspendre")),
    t("ent_sortie_autorisation", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_sortie_controle", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_sortie_demande", "assiduite", Portee::Etablissement).perso().export(),
    t("ent_sortie_param", "assiduite", Portee::Etablissement),
    t("ent_sortie_regle_niveau", "assiduite", Portee::Etablissement),
    t("ent_sortie_regle_regime", "assiduite", Portee::Etablissement),
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
    t("ref_calendrier_officiel", "referentiel", Portee::ReferentielGlobal),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn registre_trie_sans_doublon() {
        let noms: Vec<&str> = REGISTRE.iter().map(|c| c.nom).collect();
        let mut tries = noms.clone();
        tries.sort_unstable();
        tries.dedup();
        assert_eq!(noms, tries, "registre : ordre alphabétique et noms uniques");
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
