use ent_events::{
    AggregateKind, EventBusError, EventHandler, MetierEvent, MetierEventDraft,
};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct PgEventStore {
    pool: PgPool,
    handlers: Mutex<Vec<Arc<dyn EventHandler>>>,
}

impl PgEventStore {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            handlers: Mutex::new(Vec::new()),
        }
    }

    pub fn register_handler(&self, handler: Arc<dyn EventHandler>) {
        self.handlers
            .lock()
            .expect("handlers lock")
            .push(handler);
    }

    pub async fn publish(&self, draft: MetierEventDraft) -> Result<MetierEvent, EventBusError> {
        if draft.etablissement_id.is_nil() {
            return Err(EventBusError::MissingEtablissement);
        }
        if draft.type_name.trim().is_empty() {
            return Err(EventBusError::EmptyType);
        }
        let event = draft.into_event();
        let aggregate = aggregate_str(event.aggregate);

        sqlx::query(
            r#"
            INSERT INTO ent_metier_event (
                id, etablissement_id, type, aggregate, aggregate_id,
                eleve_id, payload, actor_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(event.id)
        .bind(event.etablissement_id)
        .bind(&event.type_name)
        .bind(aggregate)
        .bind(event.aggregate_id)
        .bind(event.eleve_id)
        .bind(&event.payload)
        .bind(&event.actor_user_id)
        .bind(event.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("[ent-db] event insert failed: {e}");
            EventBusError::EmptyType
        })?;

        let handlers = self.handlers.lock().expect("handlers lock").clone();
        for handler in handlers {
            handler.on_event(&event)?;
        }
        Ok(event)
    }

    pub async fn events_for_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<MetierEvent>, EventBusError> {
        let rows = sqlx::query_as::<_, EventRow>(
            r#"
            SELECT id, etablissement_id, type, aggregate, aggregate_id,
                   eleve_id, payload, actor_user_id, created_at
            FROM ent_metier_event
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("[ent-db] event select failed: {e}");
            EventBusError::EmptyType
        })?;

        Ok(rows.into_iter().map(EventRow::into_event).collect())
    }
}

fn aggregate_str(a: AggregateKind) -> &'static str {
    match a {
        AggregateKind::Eleve => "eleve",
        AggregateKind::Scolarite => "scolarite",
        AggregateKind::Foyer => "foyer",
        AggregateKind::Regime => "regime",
        AggregateKind::Appel => "appel",
        AggregateKind::Absence => "absence",
        AggregateKind::Document => "document",
        AggregateKind::Repas => "repas",
        AggregateKind::Bourse => "bourse",
        AggregateKind::Continuum => "continuum",
        AggregateKind::Incident => "incident",
        AggregateKind::Sanction => "sanction",
        AggregateKind::Carnet => "carnet",
        AggregateKind::Edt => "edt",
        AggregateKind::CahierTexte => "cahier_texte",
        AggregateKind::NotePeriode => "note_periode",
        AggregateKind::Devoir => "devoir",
        AggregateKind::Matiere => "matiere",
        AggregateKind::Note => "note",
        AggregateKind::MsgFil => "msg_fil",
        AggregateKind::Facture => "facture",
        AggregateKind::Infirmerie => "infirmerie",
        AggregateKind::Psy => "psy",
        AggregateKind::Commission => "commission",
        AggregateKind::Mail => "mail",
        AggregateKind::Sms => "sms",
        AggregateKind::Push => "push",
        AggregateKind::Bank => "bank",
        AggregateKind::Object => "object",
        AggregateKind::Sortie => "sortie",
        AggregateKind::Internat => "internat",
        AggregateKind::DispenseVs => "dispense_vs",
        AggregateKind::Journal => "journal",
        AggregateKind::AlerteCpe => "alerte_cpe",
        AggregateKind::DocASigner => "doc_a_signer",
        AggregateKind::ConseilClasse => "conseil_classe",
        AggregateKind::InfoComp => "info_comp",
        AggregateKind::Vacances => "vacances",
        AggregateKind::MenuCantine => "menu_cantine",
        AggregateKind::Quotient => "quotient",
        AggregateKind::Tva => "tva",
        AggregateKind::Trombi => "trombi",
        AggregateKind::ParamEtab => "param_etab",
        AggregateKind::Salle => "salle",
        AggregateKind::Filiere => "filiere",
        AggregateKind::Voeu => "voeu",
        AggregateKind::InscriptionSuivi => "inscription_suivi",
        AggregateKind::IbanHisto => "iban_histo",
        AggregateKind::TarifConso => "tarif_conso",
        AggregateKind::AbsenceRh => "absence_rh",
        AggregateKind::EditionVs => "edition_vs",
        AggregateKind::Entreprise => "entreprise",
        AggregateKind::ConventionStage => "convention_stage",
        AggregateKind::EdtContrainte => "edt_contrainte",
        AggregateKind::Annonce => "annonce",
        AggregateKind::Retard => "retard",
        AggregateKind::Competence => "competence",
    }
}

fn parse_aggregate(s: &str) -> AggregateKind {
    match s {
        "scolarite" => AggregateKind::Scolarite,
        "foyer" => AggregateKind::Foyer,
        "regime" => AggregateKind::Regime,
        "appel" => AggregateKind::Appel,
        "absence" => AggregateKind::Absence,
        "document" => AggregateKind::Document,
        "repas" => AggregateKind::Repas,
        "bourse" => AggregateKind::Bourse,
        "continuum" => AggregateKind::Continuum,
        "incident" => AggregateKind::Incident,
        "sanction" => AggregateKind::Sanction,
        "carnet" => AggregateKind::Carnet,
        "edt" => AggregateKind::Edt,
        "cahier_texte" => AggregateKind::CahierTexte,
        "note_periode" => AggregateKind::NotePeriode,
        "devoir" => AggregateKind::Devoir,
        "matiere" => AggregateKind::Matiere,
        "note" => AggregateKind::Note,
        "msg_fil" => AggregateKind::MsgFil,
        "facture" => AggregateKind::Facture,
        "infirmerie" => AggregateKind::Infirmerie,
        "psy" => AggregateKind::Psy,
        "commission" => AggregateKind::Commission,
        "mail" => AggregateKind::Mail,
        "sms" => AggregateKind::Sms,
        "push" => AggregateKind::Push,
        "bank" => AggregateKind::Bank,
        "object" => AggregateKind::Object,
        "sortie" => AggregateKind::Sortie,
        "internat" => AggregateKind::Internat,
        "dispense_vs" => AggregateKind::DispenseVs,
        "journal" => AggregateKind::Journal,
        "alerte_cpe" => AggregateKind::AlerteCpe,
        "doc_a_signer" => AggregateKind::DocASigner,
        "conseil_classe" => AggregateKind::ConseilClasse,
        "info_comp" => AggregateKind::InfoComp,
        "vacances" => AggregateKind::Vacances,
        "menu_cantine" => AggregateKind::MenuCantine,
        "quotient" => AggregateKind::Quotient,
        "tva" => AggregateKind::Tva,
        "trombi" => AggregateKind::Trombi,
        "param_etab" => AggregateKind::ParamEtab,
        "salle" => AggregateKind::Salle,
        "filiere" => AggregateKind::Filiere,
        "voeu" => AggregateKind::Voeu,
        "inscription_suivi" => AggregateKind::InscriptionSuivi,
        "iban_histo" => AggregateKind::IbanHisto,
        "tarif_conso" => AggregateKind::TarifConso,
        "absence_rh" => AggregateKind::AbsenceRh,
        "edition_vs" => AggregateKind::EditionVs,
        "entreprise" => AggregateKind::Entreprise,
        "convention_stage" => AggregateKind::ConventionStage,
        "edt_contrainte" => AggregateKind::EdtContrainte,
        "annonce" => AggregateKind::Annonce,
        "retard" => AggregateKind::Retard,
        "competence" => AggregateKind::Competence,
        _ => AggregateKind::Eleve,
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    etablissement_id: Uuid,
    #[sqlx(rename = "type")]
    type_name: String,
    aggregate: String,
    aggregate_id: Uuid,
    eleve_id: Option<Uuid>,
    payload: serde_json::Value,
    actor_user_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl EventRow {
    fn into_event(self) -> MetierEvent {
        MetierEvent {
            id: self.id,
            etablissement_id: self.etablissement_id,
            type_name: self.type_name,
            aggregate: parse_aggregate(&self.aggregate),
            aggregate_id: self.aggregate_id,
            eleve_id: self.eleve_id,
            payload: self.payload,
            actor_user_id: self.actor_user_id,
            created_at: self.created_at,
        }
    }
}
