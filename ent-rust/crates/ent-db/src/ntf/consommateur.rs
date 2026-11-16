//! Socle 8, lot N1 — consommateur `notifications.avis` : un seul consommateur pour tous les
//! [`AvisSurEvenement`] déclarés (`08_NOTIFICATIONS.md` §6.2).
//!
//! Il crée l'avis, ses destinataires et ses remises planifiées **dans la transaction de distribution**
//! (rejouable : aucun effet externe, l'expédition remet plus tard) et annule, tant que le délai de
//! confirmation court, les avis dont une nature déclare l'événement dans `annulee_par`.
//! Bascule progressive : une nature n'est servie ici, pour un établissement, que si
//! `ent_ntf_nature_etab.via_brique` est vrai ; sinon l'ancien chemin prévient seul.

use super::assemblage::verifier_assemblage;
use super::catalogue::{AVIS, NATURES};
use super::contrat::*;
use super::service::{annuler_avis, aviser, reglage_nature, DemandeAvis};
use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};
use sqlx::PgConnection;
use std::sync::OnceLock;
use uuid::Uuid;

pub const CONSOMMATEUR_AVIS: &str = "notifications.avis";

pub struct ConsommateurAvis {
    types: &'static [&'static str],
}

/// Types d'événements écoutés : ceux des émissions et ceux d'annulation, sans doublon.
fn types_ecoutes() -> &'static [&'static str] {
    static T: OnceLock<Vec<&'static str>> = OnceLock::new();
    T.get_or_init(|| {
        let mut v: Vec<&'static str> = AVIS.iter().map(|a| a.evenement).collect();
        for n in NATURES {
            v.extend(n.annulee_par.iter().copied());
        }
        v.sort_unstable();
        v.dedup();
        v
    })
}

impl ConsommateurAvis {
    /// Vérifie l'assemblage du catalogue : une déclaration invalide **empêche le démarrage**.
    pub fn nouveau() -> Self {
        let violations = verifier_assemblage(NATURES, AVIS);
        assert!(violations.is_empty(), "catalogue des avis invalide : {violations:#?}");
        Self { types: types_ecoutes() }
    }
}

async fn traiter(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let vue = EvenementVu(evt.event());
    let err = |e: &dyn std::fmt::Display| e.to_string();
    // Émissions.
    for a in AVIS.iter().filter(|a| a.evenement == vue.type_name()) {
        if !(a.filtre)(&vue) {
            continue;
        }
        let r = reglage_nature(tx, etab, a.nature.code).await.map_err(|e| err(&e))?;
        if !r.via_brique || r.desactivee {
            continue;
        }
        let Some(eleve_id) = vue.eleve_id() else {
            continue;
        };
        // Relecture dans l'établissement de l'événement : un élève d'un autre établissement est introuvable.
        let existe: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| err(&e))?;
        if existe.is_none() {
            continue;
        }
        aviser(
            tx,
            DemandeAvis {
                etablissement_id: etab,
                nature: a.nature,
                eleve_id: Some(eleve_id),
                cle: (a.cle)(&vue),
                parametres: (a.parametres)(&vue),
                emetteur_brique: None,
                source_type: "evenement".into(),
                source_id: Some(vue.id()),
                cause_event_id: Some(vue.id()),
                effet_le: evt.event().created_at,
                urgence_plus: None,
                delai_min: None,
            },
        )
        .await
        .map_err(|e| err(&e))?;
    }
    // Annulations : un événement d'annulation retire les avis encore dans leur délai de confirmation.
    let natures_annulees: Vec<&'static NatureAvisDecl> =
        NATURES.iter().copied().filter(|n| n.annulee_par.contains(&vue.type_name())).collect();
    if let (false, Some(eleve_id)) = (natures_annulees.is_empty(), vue.eleve_id()) {
        let jour = vue.payload_jour("jour").or_else(|| vue.payload_jour("debut"));
        for n in natures_annulees {
            let ids: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT a.id FROM ent_ntf_avis a
                  WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND a.nature = $3
                    AND a.statut = 'planifie' AND a.confirme_apres > $4
                    AND ($5::date IS NULL OR EXISTS (
                          SELECT 1 FROM ent_ntf_avis_param p
                           WHERE p.avis_id = a.id AND p.cle = 'jour' AND p.valeur_jour = $5))
                  FOR UPDATE",
            )
            .bind(etab)
            .bind(eleve_id)
            .bind(n.code)
            .bind(evt.event().created_at)
            .bind(jour)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| err(&e))?;
            for (id,) in ids {
                annuler_avis(tx, etab, id, "annulee_par_evenement", evt.event().created_at)
                    .await
                    .map_err(|e| err(&e))?;
            }
        }
    }
    Ok(())
}

impl Consommateur for ConsommateurAvis {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_AVIS
    }

    fn types(&self) -> &'static [&'static str] {
        self.types
    }

    fn traiter<'a>(&'a self, tx: &'a mut PgConnection, evt: &'a EvenementDistribue) -> FutureTraitement<'a> {
        Box::pin(traiter(tx, evt))
    }
}
