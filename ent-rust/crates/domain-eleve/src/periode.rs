//! Historique daté (dates d'effet) — cœur du dossier élève.
//!
//! Classe / régime / MEF / site : le jeudi n'écrase pas le mardi.
//! Modules (appel, cantine, facture) lisent la valeur **au jour**, pas un texte
//! « toujours vrai depuis toujours » sur la fiche.

use crate::DomainError;
use crate::Eleve;
use chrono::{Duration, NaiveDate};
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributKind {
    Classe,
    Regime,
    Mef,
    Site,
}

impl AttributKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Classe => "classe",
            Self::Regime => "regime",
            Self::Mef => "mef",
            Self::Site => "site",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "classe" => Some(Self::Classe),
            "regime" => Some(Self::Regime),
            "mef" => Some(Self::Mef),
            "site" => Some(Self::Site),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ElevePeriode {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub kind: AttributKind,
    pub valeur: String,
    pub date_debut: NaiveDate,
    /// `None` = période encore ouverte.
    pub date_fin: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct ChangeAttribut {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub kind: AttributKind,
    pub nouvelle_valeur: String,
    pub date_effet: NaiveDate,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum CutoverOp {
    Noop,
    /// Même `date_debut` que la période ouverte : on corrige la valeur sur place.
    ReplaceOpen {
        period_id: Uuid,
        nouvelle_valeur: String,
    },
    /// Ferme la veille de la date d'effet, ouvre une nouvelle période.
    CloseAndOpen {
        close_id: Uuid,
        close_date_fin: NaiveDate,
        open: ElevePeriode,
    },
    /// Première période pour cet attribut.
    OpenFirst {
        period: ElevePeriode,
    },
}

#[derive(Debug, Clone)]
pub struct PlannedCutover {
    pub op: CutoverOp,
    /// Projection fiche : valeur « courante » seulement si `date_effet <= today`.
    pub eleve: Eleve,
    pub event: Option<MetierEventDraft>,
}

pub fn day_before(date: NaiveDate) -> NaiveDate {
    date - Duration::days(1)
}

/// Planifie un changement daté sans toucher encore le dépôt.
pub fn plan_cutover(
    current: &Eleve,
    open: Option<&ElevePeriode>,
    cmd: ChangeAttribut,
    today: NaiveDate,
) -> Result<PlannedCutover, DomainError> {
    if current.etablissement_id != cmd.etablissement_id {
        return Err(DomainError::TenantMismatch);
    }
    let nouvelle = cmd.nouvelle_valeur.trim().to_string();
    if nouvelle.is_empty() {
        return Err(DomainError::EmptyAttributValeur);
    }

    let mut eleve = current.clone();
    let ancienne = match cmd.kind {
        AttributKind::Classe => current.classe.clone(),
        AttributKind::Regime => current.regime.clone(),
        AttributKind::Mef => current.mef.clone(),
        AttributKind::Site => current.site.clone(),
    };

    let op = match open {
        None => CutoverOp::OpenFirst {
            period: ElevePeriode {
                id: Uuid::new_v4(),
                etablissement_id: cmd.etablissement_id,
                eleve_id: cmd.eleve_id,
                kind: cmd.kind,
                valeur: nouvelle.clone(),
                date_debut: cmd.date_effet,
                date_fin: None,
            },
        },
        Some(p) => {
            if p.etablissement_id != cmd.etablissement_id || p.eleve_id != cmd.eleve_id {
                return Err(DomainError::TenantMismatch);
            }
            if p.kind != cmd.kind {
                return Err(DomainError::PeriodeKindMismatch);
            }
            if p.date_fin.is_some() {
                return Err(DomainError::NoOpenPeriode);
            }
            if cmd.date_effet < p.date_debut {
                return Err(DomainError::DateEffetAvantDebut);
            }
            if p.valeur == nouvelle {
                CutoverOp::Noop
            } else if cmd.date_effet == p.date_debut {
                CutoverOp::ReplaceOpen {
                    period_id: p.id,
                    nouvelle_valeur: nouvelle.clone(),
                }
            } else {
                let close_date_fin = day_before(cmd.date_effet);
                if close_date_fin < p.date_debut {
                    return Err(DomainError::DateEffetAvantDebut);
                }
                CutoverOp::CloseAndOpen {
                    close_id: p.id,
                    close_date_fin,
                    open: ElevePeriode {
                        id: Uuid::new_v4(),
                        etablissement_id: cmd.etablissement_id,
                        eleve_id: cmd.eleve_id,
                        kind: cmd.kind,
                        valeur: nouvelle.clone(),
                        date_debut: cmd.date_effet,
                        date_fin: None,
                    },
                }
            }
        }
    };

    if matches!(op, CutoverOp::Noop) {
        return Ok(PlannedCutover {
            op,
            eleve,
            event: None,
        });
    }

    // Projection fiche : jamais anticiper une date d'effet future
    // (évite le piège ScolIA : fiche mise à jour au clic, période plus tard).
    if cmd.date_effet <= today {
        apply_projection(&mut eleve, cmd.kind, &nouvelle);
    }

    let type_name = match cmd.kind {
        AttributKind::Classe => types::SCOLARITE_CLASSE_CHANGED,
        AttributKind::Regime => types::ELEVE_REGIME_CHANGED,
        AttributKind::Mef => types::SCOLARITE_MEF_CHANGED,
        AttributKind::Site => types::SCOLARITE_SITE_CHANGED,
    };
    let aggregate = match cmd.kind {
        AttributKind::Regime => AggregateKind::Regime,
        _ => AggregateKind::Scolarite,
    };

    let event = MetierEventDraft {
        etablissement_id: eleve.etablissement_id,
        type_name: type_name.to_string(),
        aggregate,
        aggregate_id: eleve.id,
        eleve_id: Some(eleve.id),
        payload: json!({
            "kind": cmd.kind.as_str(),
            "ancienne_valeur": ancienne,
            "nouvelle_valeur": nouvelle,
            "date_effet": cmd.date_effet.to_string(),
        }),
        actor_user_id: cmd.actor_user_id,
    };

    Ok(PlannedCutover {
        op,
        eleve,
        event: Some(event),
    })
}

fn apply_projection(eleve: &mut Eleve, kind: AttributKind, valeur: &str) {
    match kind {
        AttributKind::Classe => eleve.classe = Some(valeur.to_string()),
        AttributKind::Regime => eleve.regime = Some(valeur.to_string()),
        AttributKind::Mef => eleve.mef = Some(valeur.to_string()),
        AttributKind::Site => eleve.site = Some(valeur.to_string()),
    }
}

/// Valeur d'un attribut couvrant `jour` (inclusif sur `[debut, fin]`).
pub fn valeur_au(periodes: &[ElevePeriode], kind: AttributKind, jour: NaiveDate) -> Option<&str> {
    periodes
        .iter()
        .filter(|p| p.kind == kind)
        .find(|p| {
            p.date_debut <= jour && p.date_fin.map(|f| jour <= f).unwrap_or(true)
        })
        .map(|p| p.valeur.as_str())
}

/// Présent dans les effectifs / appel le jour J ?
/// Présent en base avant la rentrée ≠ compté avant `date_entree`.
/// Après `date_sortie` : plus compté ; l'historique reste.
pub fn en_effectif_au_jour(eleve: &Eleve, jour: NaiveDate) -> bool {
    if let Some(entree) = eleve.date_entree {
        if jour < entree {
            return false;
        }
    }
    if let Some(sortie) = eleve.date_sortie {
        // date_sortie = premier jour hors effectif
        if jour >= sortie {
            return false;
        }
    }
    true
}

pub fn plan_open_initial(
    eleve: &Eleve,
    kind: AttributKind,
    valeur: &str,
    date_debut: NaiveDate,
) -> Option<ElevePeriode> {
    let v = valeur.trim();
    if v.is_empty() {
        return None;
    }
    Some(ElevePeriode {
        id: Uuid::new_v4(),
        etablissement_id: eleve.etablissement_id,
        eleve_id: eleve.id,
        kind,
        valeur: v.to_string(),
        date_debut,
        date_fin: None,
    })
}

pub trait PeriodeRepository: Send + Sync {
    fn open(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        kind: AttributKind,
    ) -> Result<Option<ElevePeriode>, DomainError>;

    fn list(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<ElevePeriode>, DomainError>;

    fn apply_cutover(&self, op: &CutoverOp) -> Result<(), DomainError>;

    fn close_open_periodes(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        date_fin: NaiveDate,
    ) -> Result<(), DomainError>;
}

#[derive(Default)]
pub struct InMemoryPeriodeRepo {
    store: std::sync::Mutex<Vec<ElevePeriode>>,
}

impl InMemoryPeriodeRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PeriodeRepository for InMemoryPeriodeRepo {
    fn open(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        kind: AttributKind,
    ) -> Result<Option<ElevePeriode>, DomainError> {
        Ok(self
            .store
            .lock()
            .expect("periode lock")
            .iter()
            .find(|p| {
                p.etablissement_id == etablissement_id
                    && p.eleve_id == eleve_id
                    && p.kind == kind
                    && p.date_fin.is_none()
            })
            .cloned())
    }

    fn list(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<ElevePeriode>, DomainError> {
        let mut rows: Vec<_> = self
            .store
            .lock()
            .expect("periode lock")
            .iter()
            .filter(|p| p.etablissement_id == etablissement_id && p.eleve_id == eleve_id)
            .cloned()
            .collect();
        rows.sort_by_key(|p| (p.kind.as_str().to_string(), p.date_debut));
        Ok(rows)
    }

    fn apply_cutover(&self, op: &CutoverOp) -> Result<(), DomainError> {
        let mut g = self.store.lock().expect("periode lock");
        match op {
            CutoverOp::Noop => {}
            CutoverOp::ReplaceOpen {
                period_id,
                nouvelle_valeur,
            } => {
                let p = g
                    .iter_mut()
                    .find(|p| p.id == *period_id)
                    .ok_or(DomainError::NotFound)?;
                p.valeur = nouvelle_valeur.clone();
            }
            CutoverOp::CloseAndOpen {
                close_id,
                close_date_fin,
                open,
            } => {
                let p = g
                    .iter_mut()
                    .find(|p| p.id == *close_id)
                    .ok_or(DomainError::NotFound)?;
                p.date_fin = Some(*close_date_fin);
                g.push(open.clone());
            }
            CutoverOp::OpenFirst { period } => {
                g.push(period.clone());
            }
        }
        Ok(())
    }

    fn close_open_periodes(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        date_fin: NaiveDate,
    ) -> Result<(), DomainError> {
        for p in self.store.lock().expect("periode lock").iter_mut() {
            if p.etablissement_id == etablissement_id
                && p.eleve_id == eleve_id
                && p.date_fin.is_none()
            {
                if date_fin < p.date_debut {
                    p.date_fin = Some(p.date_debut);
                } else {
                    p.date_fin = Some(date_fin);
                }
            }
        }
        Ok(())
    }
}

/// Face appel / cantine : classe au jour J depuis l'historique.
pub fn module_appel_classe_au<P: PeriodeRepository>(
    repo: &P,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
) -> Result<Option<String>, DomainError> {
    let periodes = repo.list(etablissement_id, eleve_id)?;
    Ok(valeur_au(&periodes, AttributKind::Classe, jour).map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plan_create, today_utc, CreateEleve, EleveStatus};

    fn eleve_fixture() -> Eleve {
        plan_create(CreateEleve {
            etablissement_id: Uuid::new_v4(),
            nom: "A".into(),
            prenom: "B".into(),
            ine: None,
            sexe: None,
            date_naissance: None,
            numero_badge: None,
            redoublant: false,
            classe: Some("6A".into()),
            regime: None,
            mef: None,
            site: None,
            date_entree: Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
            date_sortie: None,
            motif_sortie: None,
            actor_user_id: None,
        })
        .unwrap()
        .eleve
    }

    #[test]
    fn cutover_jeudi_ne_ecrase_pas_mardi() {
        let eleve = eleve_fixture();
        let mardi = NaiveDate::from_ymd_opt(2026, 10, 6).unwrap();
        let jeudi = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let open = ElevePeriode {
            id: Uuid::new_v4(),
            etablissement_id: eleve.etablissement_id,
            eleve_id: eleve.id,
            kind: AttributKind::Classe,
            valeur: "6A".into(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date_fin: None,
        };

        let planned = plan_cutover(
            &eleve,
            Some(&open),
            ChangeAttribut {
                etablissement_id: eleve.etablissement_id,
                eleve_id: eleve.id,
                kind: AttributKind::Classe,
                nouvelle_valeur: "5B".into(),
                date_effet: jeudi,
                actor_user_id: None,
            },
            jeudi,
        )
        .expect("plan");

        match planned.op {
            CutoverOp::CloseAndOpen {
                close_date_fin,
                open: ref new_p,
                ..
            } => {
                assert_eq!(close_date_fin, mardi + Duration::days(1)); // mercredi 7
                assert_eq!(close_date_fin, day_before(jeudi));
                assert_eq!(new_p.date_debut, jeudi);
                assert_eq!(new_p.valeur, "5B");
            }
            other => panic!("expected CloseAndOpen, got {other:?}"),
        }
        assert_eq!(planned.eleve.classe.as_deref(), Some("5B"));

        let mut hist = vec![open.clone()];
        if let CutoverOp::CloseAndOpen {
            close_id,
            close_date_fin,
            open: new_p,
        } = &planned.op
        {
            hist.iter_mut().find(|p| p.id == *close_id).unwrap().date_fin = Some(*close_date_fin);
            hist.push(new_p.clone());
        }
        assert_eq!(valeur_au(&hist, AttributKind::Classe, mardi), Some("6A"));
        assert_eq!(valeur_au(&hist, AttributKind::Classe, jeudi), Some("5B"));
    }

    #[test]
    fn projection_ne_anticipe_pas_date_future() {
        let eleve = eleve_fixture();
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let futur = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let open = ElevePeriode {
            id: Uuid::new_v4(),
            etablissement_id: eleve.etablissement_id,
            eleve_id: eleve.id,
            kind: AttributKind::Classe,
            valeur: "6A".into(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date_fin: None,
        };
        let planned = plan_cutover(
            &eleve,
            Some(&open),
            ChangeAttribut {
                etablissement_id: eleve.etablissement_id,
                eleve_id: eleve.id,
                kind: AttributKind::Classe,
                nouvelle_valeur: "5B".into(),
                date_effet: futur,
                actor_user_id: None,
            },
            today,
        )
        .unwrap();
        assert_eq!(planned.eleve.classe.as_deref(), Some("6A"));
    }

    #[test]
    fn effectif_respecte_entree_et_sortie() {
        let mut e = eleve_fixture();
        e.date_entree = Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap());
        e.date_sortie = Some(NaiveDate::from_ymd_opt(2026, 12, 20).unwrap());
        e.status = EleveStatus::Actif;

        assert!(!en_effectif_au_jour(
            &e,
            NaiveDate::from_ymd_opt(2026, 8, 31).unwrap()
        ));
        assert!(en_effectif_au_jour(
            &e,
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()
        ));
        assert!(en_effectif_au_jour(
            &e,
            NaiveDate::from_ymd_opt(2026, 12, 19).unwrap()
        ));
        assert!(!en_effectif_au_jour(
            &e,
            NaiveDate::from_ymd_opt(2026, 12, 20).unwrap()
        ));
    }

    #[test]
    fn refuse_date_effet_avant_debut_ouvert() {
        let eleve = eleve_fixture();
        let open = ElevePeriode {
            id: Uuid::new_v4(),
            etablissement_id: eleve.etablissement_id,
            eleve_id: eleve.id,
            kind: AttributKind::Regime,
            valeur: "DP".into(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date_fin: None,
        };
        let err = plan_cutover(
            &eleve,
            Some(&open),
            ChangeAttribut {
                etablissement_id: eleve.etablissement_id,
                eleve_id: eleve.id,
                kind: AttributKind::Regime,
                nouvelle_valeur: "EXT".into(),
                date_effet: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
                actor_user_id: None,
            },
            today_utc(),
        )
        .unwrap_err();
        assert_eq!(err, DomainError::DateEffetAvantDebut);
    }
}
