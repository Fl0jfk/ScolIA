//! Trace des events vus par le Brain (audit), sans dupliquer l'état élève.

use ent_events::MetierEvent;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenEvent {
    pub type_name: String,
    pub etablissement_id: Uuid,
    pub eleve_id: Option<Uuid>,
}

#[derive(Debug, Default)]
pub struct BrainJournal {
    entries: Mutex<Vec<SeenEvent>>,
}

impl BrainJournal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_event(&self, event: &MetierEvent) {
        self.entries.lock().expect("brain journal").push(SeenEvent {
            type_name: event.type_name.clone(),
            etablissement_id: event.etablissement_id,
            eleve_id: event.eleve_id,
        });
    }

    pub fn all(&self) -> Vec<SeenEvent> {
        self.entries.lock().expect("brain journal").clone()
    }

    pub fn types(&self) -> Vec<String> {
        self.all().into_iter().map(|e| e.type_name).collect()
    }

    pub fn len(&self) -> usize {
        self.entries.lock().expect("brain journal").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
