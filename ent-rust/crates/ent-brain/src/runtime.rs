//! Runtime Brain : voit les events, appliquera plus tard les écritures directes.
//! Ne maintient **aucune** copie de classe / statut (déjà sur `ent_eleve`).

use crate::journal::BrainJournal;
use crate::react::react;
use ent_events::{EventBusError, EventHandler, MetierEvent};
use std::sync::Arc;

pub struct BrainRuntime {
    journal: Arc<BrainJournal>,
}

impl BrainRuntime {
    pub fn new(journal: Arc<BrainJournal>) -> Self {
        Self { journal }
    }

    pub fn journal(&self) -> &BrainJournal {
        &self.journal
    }
}

impl EventHandler for BrainRuntime {
    fn name(&self) -> &str {
        "ent_brain"
    }

    fn on_event(&self, event: &MetierEvent) -> Result<(), EventBusError> {
        self.journal.record_event(event);
        // Écritures additionnelles (absences, etc.) quand react() en produira.
        let _actions = react(event);
        Ok(())
    }
}

pub struct BrainP0 {
    pub runtime: Arc<BrainRuntime>,
    pub journal: Arc<BrainJournal>,
}

impl BrainP0 {
    pub fn install() -> Self {
        let journal = Arc::new(BrainJournal::new());
        let runtime = Arc::new(BrainRuntime::new(journal.clone()));
        Self { runtime, journal }
    }
}
