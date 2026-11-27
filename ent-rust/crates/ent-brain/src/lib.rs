//! Brain ENT — all-in-one, **zéro doublon d'état**.
//!
//! - Classe / statut / dates d'effet : une seule source = `ent_eleve` + `ent_eleve_periode`.
//! - Répercussions futures (ex. séjour → absence) = INSERT direct sur la
//!   table concernée, liée à `eleve_id` — pas de silo « domaine VS ».
//! - Les modules UI lisent ces tables ; ils ne recopient pas.

mod actions;
mod journal;
mod react;
mod runtime;

pub use actions::BrainAction;
pub use journal::{BrainJournal, SeenEvent};
pub use react::react;
pub use runtime::{BrainP0, BrainRuntime};

// Ré-export pour que ent-db applique les actions sans dépendre du chemin interne.

#[cfg(test)]
mod tests {
    use super::*;
    use domain_eleve::{
        today_utc, ChangeClasse, ChangeStatus, CreateEleve, EleveService, EleveStatus,
        InMemoryAbsenceRepo, InMemoryEleveRepo, InMemoryPeriodeRepo,
    };
    use ent_events::{types, AggregateKind, InMemoryEventBus, MetierEvent};
    use serde_json::json;
    use uuid::Uuid;

    fn wired() -> (
        EleveService<InMemoryEleveRepo, InMemoryAbsenceRepo, InMemoryPeriodeRepo, InMemoryEventBus>,
        BrainP0,
        Uuid,
    ) {
        let bus = InMemoryEventBus::new();
        let brain = BrainP0::install();
        bus.register_handler(brain.runtime.clone());
        let etab = Uuid::new_v4();
        (
            EleveService::new(
                InMemoryEleveRepo::new(),
                InMemoryAbsenceRepo::new(),
                InMemoryPeriodeRepo::new(),
                bus,
            ),
            brain,
            etab,
        )
    }

    #[test]
    fn classe_vit_sur_eleve_brain_ne_fait_que_voir_levent() {
        let (svc, brain, etab) = wired();
        let eleve = svc
            .create(CreateEleve {
                etablissement_id: etab,
                nom: "Dupont".into(),
                prenom: "Léa".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some("4B".into()),
                regime: None,
                mef: None,
                site: None,
                date_entree: None,
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .expect("create");

        assert_eq!(eleve.classe.as_deref(), Some("4B"));
        assert!(brain.journal.types().contains(&"eleve.created".to_string()));
    }

    #[test]
    fn change_classe_met_a_jour_eleve_sans_action_miroir() {
        let (svc, brain, etab) = wired();
        let eleve = svc
            .create(CreateEleve {
                etablissement_id: etab,
                nom: "Martin".into(),
                prenom: "Tom".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some("5A".into()),
                regime: None,
                mef: None,
                site: None,
                date_entree: None,
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .expect("create");

        let updated = svc
            .change_classe(ChangeClasse {
                etablissement_id: etab,
                eleve_id: eleve.id,
                nouvelle_classe: "4B".into(),
                date_effet: today_utc(),
                actor_user_id: None,
            })
            .expect("change");

        assert_eq!(updated.classe.as_deref(), Some("4B"));
        assert!(brain
            .journal
            .types()
            .contains(&"scolarite.classe_changed".to_string()));
    }

    #[test]
    fn sortie_sur_status_eleve_sans_copie() {
        let (svc, brain, etab) = wired();
        let eleve = svc
            .create(CreateEleve {
                etablissement_id: etab,
                nom: "Sorti".into(),
                prenom: "X".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some("3C".into()),
                regime: None,
                mef: None,
                site: None,
                date_entree: None,
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .expect("create");

        let updated = svc
            .change_status(ChangeStatus {
                etablissement_id: etab,
                eleve_id: eleve.id,
                nouveau_status: EleveStatus::Sorti,
                actor_user_id: None,
            })
            .expect("sortie");

        assert_eq!(updated.status, EleveStatus::Sorti);
        assert!(brain
            .journal
            .types()
            .contains(&"eleve.status_changed".to_string()));
    }

    #[test]
    fn react_socle_ne_duplique_pas_classe_ni_statut() {
        let event = MetierEvent {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            type_name: types::SCOLARITE_CLASSE_CHANGED.into(),
            aggregate: AggregateKind::Scolarite,
            aggregate_id: Uuid::new_v4(),
            eleve_id: Some(Uuid::new_v4()),
            payload: json!({"nouvelle_valeur": "4B", "date_effet": "2026-10-08"}),
            actor_user_id: None,
            created_at: chrono::Utc::now(),
        };
        assert!(react(&event).is_empty());
    }
}
