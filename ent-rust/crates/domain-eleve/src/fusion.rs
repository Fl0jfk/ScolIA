//! Socle 2 (l'élève), lot E6 — registre `TABLES_ELEVE` : TOUTE table qui porte `eleve_id`
//! et ce que devient chacune de ses lignes quand deux fiches d'élèves sont fusionnées (§7.10
//! de `docs/socles/02_ELEVE.md`).
//!
//! Même esprit que `TABLES_CLASSE` (référentiel) et que la garde SQL statique : un **test de
//! garde** (`ent-db`, `tests_eleve_fusion`) lit `information_schema` et **échoue** si une
//! table qui porte `eleve_id` n'est pas déclarée ici (ou si une table déclarée n'existe plus,
//! ou si la clé primaire déclarée n'est pas la vraie). Aucune table nouvelle n'échappe donc à
//! la fusion : l'auteur d'une migration doit choisir une stratégie.
//!
//! Deux stratégies seulement, volontairement :
//! * [`Strategie::Deplacer`] — toutes les lignes passent à l'élève conservé. Si une ligne de
//!   l'absorbé entre en conflit d'unicité avec une ligne du conservé (même année d'inscription,
//!   même INE, scolarisations qui se chevauchent…), la ligne **du conservé l'emporte** et celle
//!   de l'absorbé reste sur l'absorbé, listée dans l'aperçu et dans le registre de la fusion.
//! * [`Strategie::Reste`] — la table est un **historique** ou un **état propre à la fiche** :
//!   ses lignes restent attachées à l'élève absorbé (lisibles via la fusion, jamais perdues).
//!
//! Ce module est pur (aucune base de données) : le déplacement est dans `ent-db::eleve_fusion`.

/// Délai pendant lequel une fusion s'annule (Q-E16 : 30 jours, comme les personnes).
pub const DELAI_DEFUSION_JOURS: i64 = 30;

/// Ce que devient une table lors d'une fusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategie {
    /// Les lignes passent à l'élève conservé ; en cas de conflit d'unicité, le conservé l'emporte.
    Deplacer,
    /// Les lignes restent sur l'élève absorbé (historique ou état propre à la fiche).
    Reste,
}

/// Une table qui porte `eleve_id`.
#[derive(Debug, Clone, Copy)]
pub struct TableEleve {
    pub table: &'static str,
    /// Colonnes de la clé primaire réelle (vérifiée contre la base par le test de garde).
    pub cle: &'static [&'static str],
    pub strategie: Strategie,
    /// Pourquoi la table reste sur l'absorbé (vide pour `Deplacer`).
    pub raison: &'static str,
}

impl TableEleve {
    /// Colonnes qui identifient une ligne **indépendamment de l'élève** (la clé sans `eleve_id`).
    pub fn cle_hors_eleve(&self) -> Vec<&'static str> {
        self.cle.iter().copied().filter(|c| *c != "eleve_id").collect()
    }
}

const fn d(table: &'static str, cle: &'static [&'static str]) -> TableEleve {
    TableEleve { table, cle, strategie: Strategie::Deplacer, raison: "" }
}

const fn r(table: &'static str, cle: &'static [&'static str], raison: &'static str) -> TableEleve {
    TableEleve { table, cle, strategie: Strategie::Reste, raison }
}

/// Registre de toutes les tables qui portent `eleve_id` (ordre alphabétique).
pub const TABLES_ELEVE: &[TableEleve] = &[
    d("ent_absence", &["id"]),
    d("ent_absence_declaration", &["id"]),
    d("ent_absence_seance", &["id"]),
    d("ent_absence_signal", &["id"]),
    d("ent_aesh_affectation", &["id"]),
    d("ent_affectation", &["id"]),
    d("ent_alerte_cpe", &["id"]),
    d("ent_amenagement", &["id"]),
    d("ent_appel_ligne", &["id"]),
    d("ent_bilan_cycle", &["id"]),
    r("ent_bulletin", &["id"], "bulletin émis : document figé (contenu et élève immuables)"),
    d("ent_campagne_destinataire", &["campagne_id", "user_id"]),
    d("ent_carnet_entree", &["id"]),
    d("ent_competence_evaluation", &["id"]),
    d("ent_conflit_interet", &["id"]),
    d("ent_conseil_classe_dossier", &["id"]),
    d("ent_contrainte_publiee", &["id"]),
    d("ent_convention_stage", &["id"]),
    d("ent_creneau_rdv", &["id"]),
    d("ent_dispense_vs", &["id"]),
    d("ent_doc_a_signer", &["id"]),
    d("ent_eleve_bourse", &["id"]),
    d("ent_eleve_capacite", &["id"]),
    d("ent_eleve_compte", &["etablissement_id", "eleve_id"]),
    d("ent_eleve_contact_urgence", &["id"]),
    d("ent_eleve_dispense_repas", &["id"]),
    d("ent_eleve_dispositif", &["id"]),
    d("ent_eleve_document", &["id"]),
    d("ent_eleve_grille_repas", &["id"]),
    d("ent_eleve_identifiant", &["id"]),
    r("ent_eleve_identite", &["id"], "versions d'identité immuables : l'historique d'état civil reste sur la fiche absorbée"),
    r("ent_eleve_identite_a_traiter", &["id"], "diagnostic de la reprise des identités"),
    d("ent_eleve_inscription", &["id"]),
    r("ent_eleve_inscription_a_traiter", &["id"], "diagnostic de la reprise des inscriptions"),
    d("ent_eleve_interdiction", &["id"]),
    d("ent_eleve_invitation", &["id"]),
    d("ent_eleve_lien", &["id"]),
    r("ent_eleve_majorite_evenement", &["eleve_id", "sorte"], "marqueurs de la tâche nocturne, propres à chaque fiche"),
    d("ent_eleve_mobilite", &["id"]),
    d("ent_eleve_modalite", &["id"]),
    d("ent_eleve_option", &["id"]),
    r("ent_eleve_option_a_traiter", &["id"], "diagnostic de la reprise des options"),
    d("ent_eleve_option_annee", &["id"]),
    d("ent_eleve_option_groupe", &["source", "option_id"]),
    r("ent_eleve_orphelin", &["id"], "diagnostic des clés composées"),
    d("ent_eleve_periode", &["id"]),
    r("ent_eleve_proposition", &["id"], "propositions déposées sur la fiche absorbée (immuables une fois décidées)"),
    d("ent_eleve_scolarisation", &["id"]),
    d("ent_eleve_scolarite_annee", &["id"]),
    r("ent_eleve_scolarite_ecart", &["id"], "diagnostic du statut dérivé"),
    d("ent_equipe_reunion", &["id"]),
    d("ent_fac_ligne", &["id"]),
    d("ent_fiche_sante", &["id"]),
    d("ent_fin_reduction_eleve", &["id"]),
    d("ent_formulaire_reponse", &["id"]),
    d("ent_import_ligne", &["id"]),
    d("ent_incident", &["id"]),
    d("ent_infirmerie_passage", &["id"]),
    d("ent_info_comp_valeur", &["id"]),
    d("ent_inscription_dossier", &["id"]),
    d("ent_inscription_lien_attente", &["id"]),
    d("ent_inscription_suivi", &["id"]),
    d("ent_internat_affectation", &["id"]),
    d("ent_internat_presence", &["id"]),
    d("ent_internat_suivi", &["id"]),
    r("ent_journal_modif", &["id"], "journal d'audit en ajout seul : l'historique reste sur la fiche où il a été écrit"),
    d("ent_lien_attente", &["id"]),
    r("ent_mail_outbox", &["id"], "file d'envoi : messages déjà adressés"),
    r("ent_metier_event", &["id"], "événements déjà publiés : historique immuable"),
    d("ent_msg_fil", &["id"]),
    d("ent_note", &["id"]),
    d("ent_note_appreciation", &["id"]),
    d("ent_note_correction", &["id"]),
    d("ent_notification", &["id"]),
    d("ent_notification_envoi", &["id"]),
    d("ent_orientation_avis", &["id"]),
    d("ent_pai", &["id"]),
    d("ent_pai_piece", &["id"]),
    d("ent_pai_signature", &["id"]),
    d("ent_personne", &["id"]),
    d("ent_procedure_disciplinaire", &["id"]),
    d("ent_psy_dossier", &["id"]),
    d("ent_psy_entretien", &["id"]),
    d("ent_psy_rdv", &["id"]),
    d("ent_punition", &["id"]),
    r("ent_push_outbox", &["id"], "file d'envoi : notifications déjà adressées"),
    d("ent_repas_passage", &["id"]),
    d("ent_retard", &["id"]),
    d("ent_sanction", &["id"]),
    d("ent_signalement_assiduite", &["id"]),
    r("ent_sms_outbox", &["id"], "file d'envoi : SMS déjà adressés"),
    d("ent_sortie_autorisation", &["id"]),
    d("ent_sortie_controle", &["id"]),
    d("ent_sortie_demande", &["id"]),
    d("ent_travail_eleve", &["travail_id", "eleve_id"]),
    d("ent_travail_remise", &["id"]),
    d("ent_voeu_inscription", &["id"]),
    d("ent_vs_commission_dossier", &["id"]),
];

/// La déclaration d'une table du registre.
pub fn table_eleve(nom: &str) -> Option<&'static TableEleve> {
    TABLES_ELEVE.iter().find(|t| t.table == nom)
}

/// Tables dont les lignes passent à l'élève conservé, dans l'ordre du registre.
pub fn tables_a_deplacer() -> impl Iterator<Item = &'static TableEleve> {
    TABLES_ELEVE.iter().filter(|t| t.strategie == Strategie::Deplacer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn registre_sans_doublon_et_trie() {
        let noms: Vec<_> = TABLES_ELEVE.iter().map(|t| t.table).collect();
        let uniques: HashSet<_> = noms.iter().collect();
        assert_eq!(noms.len(), uniques.len(), "une table déclarée deux fois");
        let mut tri = noms.clone();
        tri.sort_unstable();
        assert_eq!(noms, tri, "registre à garder en ordre alphabétique");
    }

    #[test]
    fn chaque_table_a_une_cle_et_les_reste_une_raison() {
        for t in TABLES_ELEVE {
            assert!(!t.cle.is_empty(), "{} : clé primaire vide", t.table);
            assert_eq!(
                t.strategie == Strategie::Reste,
                !t.raison.is_empty(),
                "{} : une table qui reste sur l'absorbé doit dire pourquoi (et seulement celles-là)",
                t.table
            );
        }
    }

    #[test]
    fn historiques_immuables_restent_sur_l_absorbe() {
        for t in ["ent_journal_modif", "ent_metier_event", "ent_eleve_identite", "ent_bulletin"] {
            assert_eq!(table_eleve(t).unwrap().strategie, Strategie::Reste, "{t}");
        }
        for t in ["ent_note", "ent_absence", "ent_eleve_inscription", "ent_eleve_identifiant"] {
            assert_eq!(table_eleve(t).unwrap().strategie, Strategie::Deplacer, "{t}");
        }
    }

    #[test]
    fn cle_hors_eleve() {
        assert_eq!(table_eleve("ent_travail_eleve").unwrap().cle_hors_eleve(), vec!["travail_id"]);
        assert_eq!(table_eleve("ent_note").unwrap().cle_hors_eleve(), vec!["id"]);
        assert_eq!(
            table_eleve("ent_eleve_compte").unwrap().cle_hors_eleve(),
            vec!["etablissement_id"]
        );
    }
}
