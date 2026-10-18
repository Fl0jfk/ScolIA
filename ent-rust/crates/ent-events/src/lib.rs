//! Événements métier transverses — socle du Brain.
//!
//! Première implémentation : bus **in-memory** pour tests L0.
//! La persistance Postgres (`metier_event`) viendra en L1 sans changer le contrat public.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

/// Types d'événements connus (alignés sur l'amorce ScolIA `eleve-core/events.ts`).
pub mod types {
    pub const ELEVE_CREATED: &str = "eleve.created";
    pub const ELEVE_STATUS_CHANGED: &str = "eleve.status_changed";
    pub const ELEVE_IDENTITY_CHANGED: &str = "eleve.identity_changed";
    /// Réservé : aucun émetteur tant que « scolarité ouverte/fermée » (période
    /// vs année) n'est pas tranché côté produit — ne pas inventer la sémantique.
    pub const SCOLARITE_OPENED: &str = "scolarite.opened";
    pub const SCOLARITE_CLASSE_CHANGED: &str = "scolarite.classe_changed";
    pub const SCOLARITE_MEF_CHANGED: &str = "scolarite.mef_changed";
    pub const SCOLARITE_SITE_CHANGED: &str = "scolarite.site_changed";
    pub const SCOLARITE_CLOSED: &str = "scolarite.closed";
    pub const ELEVE_REGIME_CHANGED: &str = "eleve.regime_changed";
    pub const ELEVE_SORTIE: &str = "eleve.sortie";
    /// Réservé : changement de foyer — émission non branchée (règle produit non validée).
    pub const FOYER_CHANGED: &str = "foyer.changed";
    pub const ELEVE_LIEN_UPDATED: &str = "eleve.lien_updated";
    /// Fait d'absence sur l'élève (une vérité pour absences, cantine, appel…).
    pub const ELEVE_ABSENCE_DECLARED: &str = "eleve.absence_declared";
    pub const ELEVE_ABSENCE_JUSTIF_SOUMIS: &str = "eleve.absence_justif_soumis";
    pub const ELEVE_ABSENCE_JUSTIF_RETRAIT: &str = "eleve.absence_justif_retrait";
    pub const ELEVE_ABSENCE_JUSTIFIED: &str = "eleve.absence_justified";
    pub const ELEVE_ABSENCE_REFUSED: &str = "eleve.absence_refused";
    pub const ELEVE_ABSENCE_ANNULEE_PRESENT: &str = "eleve.absence_annulee_present";
    /// Compteur d'absences actives a atteint le seuil établissement (file VS).
    pub const ELEVE_ABSENCE_SEUIL_ATTEINT: &str = "eleve.absence_seuil_atteint";
    pub const ELEVE_ABSENCE_SIGNAL_EN_COURS: &str = "eleve.absence_signal_en_cours";
    pub const ELEVE_ABSENCE_SIGNAL_CLOS: &str = "eleve.absence_signal_clos";
    pub const ABSENCE_MOTIF_ETAB_CREATED: &str = "absence.motif_etab_created";
    pub const ELEVE_COMPTE_LIE: &str = "eleve.compte_lie";
    pub const ELEVE_COMPTE_REVOQUE: &str = "eleve.compte_revoque";
    pub const ELEVE_COMPTE_INVITE: &str = "eleve.compte_invite";
    pub const ELEVE_OPTION_ADDED: &str = "eleve.option_added";
    pub const ELEVE_DOCUMENT_UPLOADED: &str = "eleve.document_uploaded";
    pub const ELEVE_DOCUMENT_GRANT_REQUESTED: &str = "eleve.document_grant_requested";
    pub const ELEVE_DOCUMENT_GRANT_DECIDED: &str = "eleve.document_grant_decided";
    pub const ELEVE_DISPOSITIF_AJOUTE: &str = "eleve.dispositif_ajoute";
    pub const REPAS_ACTIVITE_CREATED: &str = "repas.activite_created";
    pub const ELEVE_GRILLE_REPAS_POSED: &str = "eleve.grille_repas_posed";
    pub const ELEVE_GRILLE_REPAS_CLOSE: &str = "eleve.grille_repas_close";
    pub const ELEVE_DISPENSE_REPAS_POSED: &str = "eleve.dispense_repas_posed";
    pub const ELEVE_DISPENSE_REPAS_ANNULEE: &str = "eleve.dispense_repas_annulee";
    pub const ELEVE_REPAS_PASSAGE: &str = "eleve.repas_passage";
    pub const BOURSE_TYPE_CREATED: &str = "bourse.type_created";
    pub const ELEVE_BOURSE_AFFECTEE: &str = "eleve.bourse_affectee";
    pub const ELEVE_BOURSE_DECIDEE: &str = "eleve.bourse_decidee";
    pub const REINSCRIPTION_CAMPAGNE_OUVERTE: &str = "reinscription.campagne_ouverte";
    pub const ELEVE_SCOLARITE_ANNEE_ENREGISTREE: &str = "eleve.scolarite_annee_enregistree";
    pub const ELEVE_MONTEE_RAPIDE: &str = "eleve.montee_rapide";
    pub const ELEVE_MONTEE_LOT: &str = "eleve.montee_lot";
    pub const ELEVE_OPTION_N1_CHOISIE: &str = "eleve.option_n1_choisie";
    pub const ELEVE_REINSCRIPTION_CONFIRMEE: &str = "eleve.reinscription_confirmee";
    pub const REINSCRIPTION_EXIGENCE_DOC: &str = "reinscription.exigence_doc";
    pub const REINSCRIPTION_ACOMPTE_CONFIG: &str = "reinscription.acompte_config";
    pub const VS_INCIDENT_TYPE_CREATED: &str = "vs.incident_type_created";
    pub const VS_SANCTION_TYPE_CREATED: &str = "vs.sanction_type_created";
    pub const VS_SANCTION_MOTIF_CREATED: &str = "vs.sanction_motif_created";
    pub const ELEVE_INCIDENT_DECLARED: &str = "eleve.incident_declared";
    pub const ELEVE_SANCTION_PRONONCEE: &str = "eleve.sanction_prononcee";
    pub const ELEVE_SANCTION_ANNULEE: &str = "eleve.sanction_annulee";
    pub const MATIERE_CREATED: &str = "matiere.created";
    pub const NOTE_SAISIE: &str = "note.saisie";
    pub const NOTE_APPRECIATION: &str = "note.appreciation";
    pub const APPEL_OPENED: &str = "appel.opened";
    pub const APPEL_CLOSED: &str = "appel.closed";
    pub const ELEVE_CARNET_ENTREE: &str = "eleve.carnet_entree";
    pub const ELEVE_CARNET_SIGNEE: &str = "eleve.carnet_signee";
    pub const EDT_CRENEAU_CREATED: &str = "edt.creneau_created";
    pub const EDT_EXCEPTION: &str = "edt.exception";
    pub const EDT_CALENDRIER_AB: &str = "edt.calendrier_ab";
    pub const CAHIER_TEXTE: &str = "cahier.texte";
    pub const NOTE_PERIODE_CREATED: &str = "note.periode_created";
    pub const NOTE_PERIODE_STATUT: &str = "note.periode_statut";
    pub const NOTE_BULLETIN_PUBLIE: &str = "note.bulletin_publie";
    pub const DEVOIR_CREATED: &str = "devoir.created";
    pub const DEVOIR_DELETED: &str = "devoir.deleted";
    pub const MSG_FIL_OUVERT: &str = "msg.fil_ouvert";
    pub const MSG_MODELE: &str = "msg.modele";
    pub const MSG_FIL_ARCHIVE: &str = "msg.fil_archive";
    pub const MSG_FIL_CLASSER: &str = "msg.fil_classer";
    pub const MSG_ENVOYE: &str = "msg.envoye";
    pub const MSG_ACCUSE: &str = "msg.accuse";
    pub const MSG_PIECE: &str = "msg.piece";
    pub const FAC_FACTURE_CREEE: &str = "fac.facture_creee";
    pub const FAC_FACTURE_EMISE: &str = "fac.facture_emise";
    pub const FAC_ENCAISSEMENT: &str = "fac.encaissement";
    pub const FAC_FACTURE_SOLDEE: &str = "fac.facture_soldee";
    pub const FAC_ECHEANCIER_POSE: &str = "fac.echeancier_pose";
    pub const FAC_QUITTANCE_EMISE: &str = "fac.quittance_emise";
    pub const FAC_SEPA_MANDAT: &str = "fac.sepa_mandat";
    pub const FAC_SEPA_PRELEVEMENT: &str = "fac.sepa_prelevement";
    pub const FAC_SEPA_EXPORT: &str = "fac.sepa_export";
    pub const FAC_SEPA_EXECUTE: &str = "fac.sepa_execute";
    pub const FAC_SEPA_REJET: &str = "fac.sepa_rejet";
    pub const FAC_RAPPEL_ENVOYE: &str = "fac.rappel_envoye";
    pub const INFIRMERIE_PASSAGE: &str = "infirmerie.passage";
    pub const INFIRMERIE_FICHE_SANTE: &str = "infirmerie.fiche_sante";
    pub const INFIRMERIE_FICHE_SANTE_PDF: &str = "infirmerie.fiche_sante_pdf";
    pub const INFIRMERIE_PAI: &str = "infirmerie.pai";
    pub const INFIRMERIE_PAI_SIGNE: &str = "infirmerie.pai_signe";
    pub const INFIRMERIE_PAI_PIECE: &str = "infirmerie.pai_piece";
    pub const PSY_ENTRETIEN: &str = "psy.entretien";
    pub const PSY_RDV: &str = "psy.rdv";
    pub const PSY_DOSSIER: &str = "psy.dossier";
    pub const PSY_COURRIER: &str = "psy.courrier";
    pub const VS_COMMISSION: &str = "vs.commission";
    pub const VS_COMMISSION_CONVOCATION: &str = "vs.commission_convocation";
    pub const VS_COMMISSION_DECISION: &str = "vs.commission_decision";
    pub const VS_SIGNAL_COMMISSION: &str = "vs.signal_commission";
    /// Mail SMTP sortant (carnet, seuil, courrier psy…).
    pub const MAIL_ENVOYE: &str = "mail.envoye";
    /// SMS sortant (sanctions collège, carnet…).
    pub const SMS_ENVOYE: &str = "sms.envoye";
    /// Notification push (apps natives / web).
    pub const PUSH_ENVOYE: &str = "push.envoye";
    pub const PUSH_DEVICE_REGISTERED: &str = "push.device_registered";
    pub const BANK_RAPPROCHEMENT: &str = "bank.rapprochement";
    pub const OBJECT_STORED: &str = "object.stored";
    pub const EDT_IMPORT: &str = "edt.import";
    pub const SORTIE_AUTORISATION: &str = "sortie.autorisation";
    pub const SORTIE_DEMANDE: &str = "sortie.demande";
    pub const SORTIE_DECISION: &str = "sortie.decision";
    pub const SORTIE_CONTROLE: &str = "sortie.controle";
    pub const INTERNAT_CHAMBRE: &str = "internat.chambre";
    pub const INTERNAT_AFFECTATION: &str = "internat.affectation";
    pub const INTERNAT_PRESENCE: &str = "internat.presence";
    pub const INTERNAT_SUIVI: &str = "internat.suivi";
    pub const DISPENSE_VS: &str = "vs.dispense";
    pub const DISPENSE_VS_CLOTURE: &str = "vs.dispense_cloture";
    pub const JOURNAL_ENTREE: &str = "journal.entree";
    pub const ALERTE_CPE: &str = "vs.alerte_cpe";
    pub const ALERTE_CPE_CLOTURE: &str = "vs.alerte_cpe_cloture";
    pub const DOC_A_SIGNER: &str = "doc.a_signer";
    pub const DOC_SIGNE: &str = "doc.signe";
    pub const CONSEIL_CLASSE: &str = "notes.conseil_classe";
    pub const CONSEIL_DOSSIER: &str = "notes.conseil_dossier";
    pub const INFO_COMP_CHAMP: &str = "admin.info_comp_champ";
    pub const INFO_COMP_VALEUR: &str = "admin.info_comp_valeur";
    pub const VACANCES: &str = "etab.vacances";
    /// Réservé (audit) : recherche de doublons d'homonymes — émission non branchée.
    pub const HOMONYME_RECHERCHE: &str = "eleve.homonyme_recherche";
    pub const MENU_CANTINE: &str = "repas.menu";
    pub const QUOTIENT_FOYER: &str = "fac.quotient";
    pub const TAUX_TVA: &str = "fac.taux_tva";
    pub const PHOTO_ELEVE: &str = "eleve.photo";
    pub const PARAM_ETAB: &str = "etab.param";
    pub const SALLE: &str = "etab.salle";
    pub const FILIERE: &str = "ins.filiere";
    pub const VOEU: &str = "ins.voeu";
    pub const INSCRIPTION_SUIVI: &str = "ins.suivi";
    pub const IBAN_HISTO: &str = "fac.iban_histo";
    pub const TARIF_CONSO: &str = "repas.tarif_conso";
    pub const ABSENCE_RH: &str = "rh.absence";
    pub const ABSENCE_RH_VALIDEE: &str = "rh.absence_validee";
    pub const EDT_AFFECTATION: &str = "edt.affectation";
    pub const EDITION_VS: &str = "vs.edition";
    pub const ENTREPRISE: &str = "stage.entreprise";
    pub const CONVENTION_STAGE: &str = "stage.convention";
    pub const EDT_CONTRAINTE: &str = "edt.contrainte";
    pub const ANNONCE_PUBLIEE: &str = "annonce.publiee";
    pub const RETARD_DECLARE: &str = "vs.retard";
    pub const COMPETENCE_EVALUEE: &str = "note.competence";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregateKind {
    Eleve,
    Scolarite,
    Foyer,
    Regime,
    Appel,
    Absence,
    Document,
    Repas,
    Bourse,
    Continuum,
    Incident,
    Sanction,
    Carnet,
    Edt,
    CahierTexte,
    NotePeriode,
    Devoir,
    Matiere,
    Note,
    MsgFil,
    Facture,
    Infirmerie,
    Psy,
    Commission,
    Mail,
    Sms,
    Push,
    Bank,
    Object,
    Sortie,
    Internat,
    DispenseVs,
    Journal,
    AlerteCpe,
    DocASigner,
    ConseilClasse,
    InfoComp,
    Vacances,
    MenuCantine,
    Quotient,
    Tva,
    Trombi,
    ParamEtab,
    Salle,
    Filiere,
    Voeu,
    InscriptionSuivi,
    IbanHisto,
    TarifConso,
    AbsenceRh,
    EditionVs,
    Entreprise,
    ConventionStage,
    EdtContrainte,
    Annonce,
    Retard,
    Competence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetierEvent {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub type_name: String,
    pub aggregate: AggregateKind,
    pub aggregate_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub payload: Value,
    pub actor_user_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MetierEventDraft {
    pub etablissement_id: Uuid,
    pub type_name: String,
    pub aggregate: AggregateKind,
    pub aggregate_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub payload: Value,
    pub actor_user_id: Option<String>,
}

impl MetierEventDraft {
    pub fn into_event(self) -> MetierEvent {
        MetierEvent {
            id: Uuid::new_v4(),
            etablissement_id: self.etablissement_id,
            type_name: self.type_name,
            aggregate: self.aggregate,
            aggregate_id: self.aggregate_id,
            eleve_id: self.eleve_id,
            payload: self.payload,
            actor_user_id: self.actor_user_id,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EventBusError {
    #[error("etablissement_id manquant ou nil")]
    MissingEtablissement,
    #[error("type d'événement vide")]
    EmptyType,
}

/// Handler Brain : réagit à un événement (répercussions métier).
pub trait EventHandler: Send + Sync {
    fn name(&self) -> &str;
    fn on_event(&self, event: &MetierEvent) -> Result<(), EventBusError>;
}

pub trait EventBus: Send + Sync {
    fn publish(&self, draft: MetierEventDraft) -> Result<MetierEvent, EventBusError>;
    fn events_for_eleve(&self, etablissement_id: Uuid, eleve_id: Uuid) -> Vec<MetierEvent>;
    fn all_events(&self) -> Vec<MetierEvent>;
}

/// Bus mémoire + fan-out vers handlers enregistrés.
#[derive(Default)]
pub struct InMemoryEventBus {
    inner: Mutex<Vec<MetierEvent>>,
    handlers: Mutex<Vec<Arc<dyn EventHandler>>>,
}

impl InMemoryEventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_handler(&self, handler: Arc<dyn EventHandler>) {
        self.handlers
            .lock()
            .expect("event bus handlers lock")
            .push(handler);
    }
}

impl EventBus for InMemoryEventBus {
    fn publish(&self, draft: MetierEventDraft) -> Result<MetierEvent, EventBusError> {
        if draft.etablissement_id.is_nil() {
            return Err(EventBusError::MissingEtablissement);
        }
        if draft.type_name.trim().is_empty() {
            return Err(EventBusError::EmptyType);
        }
        let event = draft.into_event();
        {
            let mut store = self.inner.lock().expect("event bus store lock");
            store.push(event.clone());
        }
        let handlers = self.handlers.lock().expect("event bus handlers lock").clone();
        for handler in handlers {
            handler.on_event(&event)?;
        }
        Ok(event)
    }

    fn events_for_eleve(&self, etablissement_id: Uuid, eleve_id: Uuid) -> Vec<MetierEvent> {
        self.inner
            .lock()
            .expect("event bus store lock")
            .iter()
            .filter(|e| e.etablissement_id == etablissement_id && e.eleve_id == Some(eleve_id))
            .cloned()
            .collect()
    }

    fn all_events(&self) -> Vec<MetierEvent> {
        self.inner.lock().expect("event bus store lock").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingHandler {
        hits: Arc<AtomicUsize>,
    }

    impl EventHandler for CountingHandler {
        fn name(&self) -> &str {
            "counting"
        }

        fn on_event(&self, _event: &MetierEvent) -> Result<(), EventBusError> {
            self.hits.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn publish_stores_and_notifies_handlers() {
        let bus = InMemoryEventBus::new();
        let hits = Arc::new(AtomicUsize::new(0));
        bus.register_handler(Arc::new(CountingHandler {
            hits: Arc::clone(&hits),
        }));

        let etab = Uuid::new_v4();
        let eleve = Uuid::new_v4();
        let event = bus
            .publish(MetierEventDraft {
                etablissement_id: etab,
                type_name: types::ELEVE_CREATED.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: eleve,
                eleve_id: Some(eleve),
                payload: json!({ "nom": "DUPONT" }),
                actor_user_id: Some("user-1".into()),
            })
            .expect("publish");

        assert_eq!(event.type_name, types::ELEVE_CREATED);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert_eq!(bus.events_for_eleve(etab, eleve).len(), 1);
    }

    #[test]
    fn reject_nil_etablissement() {
        let bus = InMemoryEventBus::new();
        let err = bus
            .publish(MetierEventDraft {
                etablissement_id: Uuid::nil(),
                type_name: types::ELEVE_CREATED.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: Uuid::new_v4(),
                eleve_id: None,
                payload: Value::Null,
                actor_user_id: None,
            })
            .expect_err("must fail");
        assert_eq!(err, EventBusError::MissingEtablissement);
    }
}
