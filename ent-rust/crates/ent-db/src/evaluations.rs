//! Évaluations (socle 2, lot S2-7) : grille de saisie par service et période,
//! publication aux familles, moyennes (règles de `domain_eleve::evaluation`),
//! verrouillage des périodes, corrections après fermeture, vue de classe, relevé.
//!
//! Droits (moteur D-017) :
//! - lire une grille : professeur du service, ou `notes.lire` (lignes limitées aux
//!   élèves de sa portée) ;
//! - créer un devoir, saisir, publier : professeur **du service** ou `notes.saisir` sur
//!   tout l'établissement ; chaque élève saisi doit être dans la portée de `notes.saisir`
//!   **et** dans l'effectif du service sur la période ;
//! - statut des périodes et règles de calcul : `periodes.verrouiller` ; coefficients de
//!   matière : `matieres.gerer` ; correction après fermeture : `notes.corriger` ;
//! - familles et élèves : `actor_can_read_notes` (lien actif + `peut_voir_notes`, ou soi),
//!   **évaluations publiées seulement**.
//!
//! Tous les faits sont publiés dans la transaction de l'écriture (outbox S2-0).

use std::collections::{HashMap, HashSet};

use chrono::{NaiveDate, Utc};
use domain_eleve::evaluation::{
    moyenne_generale, moyenne_matiere, statistiques, valider_devoir, valider_resultat, Arrondi,
    ErreurSaisie, Moyenne, ReglesCalcul, ResultatCalcul, Statistiques, StatutResultat,
};
use domain_eleve::{today_utc, DomainError, NotePeriodeStatut};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::evaluations::{
    CoefMatiereBody, CorrigerNoteBody, CreerDevoirGrilleBody, DevoirGrilleDto, GrilleDto,
    LigneClasseDto, LigneGrilleDto, MatiereColonneDto, PeriodeNotesDto, RapportSaisieDto,
    RefusSaisieDto, ReglesCalculDto, ReleveEleveDto, ReleveEvaluationDto, ReleveMatiereDto,
    ResultatDto, SaisieGrilleBody, ServiceEvaluationDto, StatsDto, VueClasseDto,
};
use serde_json::json;
use uuid::Uuid;

use crate::outbox::Publier;
use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};

/// Types d'événements du lot S2-7 (les noms existants d'`ent_events::types` sont réutilisés).
pub mod types_eva {
    pub const EVALUATION_PUBLIEE: &str = "evaluation.publiee";
    pub const RESULTATS_SAISIS: &str = "evaluation.resultats_saisis";
    pub const NOTE_CORRIGEE: &str = "note.corrigee";
    pub const APPRECIATION_GENERALE: &str = "evaluation.appreciation_generale";
    pub const REGLES_MODIFIEES: &str = "evaluation.regles_modifiees";
    pub const CONSEIL_DECISION: &str = "conseil.decision_prise";
    pub const ORIENTATION_AVIS: &str = "orientation.avis_donne";
    pub const ORIENTATION_REPONSE: &str = "orientation.reponse_famille";
    pub const BILAN_CYCLE_SAISI: &str = "evaluation.bilan_cycle_saisi";
    pub const BILAN_CYCLE_VALIDE: &str = "evaluation.bilan_cycle_valide";
    pub const BULLETIN_GENERE: &str = "evaluation.bulletin_genere";
    pub const EXPORT_REALISE: &str = "export.realise";
}

pub(crate) fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("evaluations", err))
}

pub(crate) fn introuvable() -> EntDbError {
    EntDbError::Domain(DomainError::NotFound)
}

pub(crate) fn refus(msg: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(msg)
}

pub(crate) fn conflit(msg: &'static str) -> EntDbError {
    EntDbError::ConflitDossier(msg)
}

fn erreur_saisie(e: ErreurSaisie) -> &'static str {
    match e {
        ErreurSaisie::Format => "note : au plus deux décimales, nombre positif",
        ErreurSaisie::HorsBareme => "note supérieure au barème",
        ErreurSaisie::Bareme => "barème invalide (0 < barème ≤ 1000)",
        ErreurSaisie::Coefficient => "coefficient invalide (0 < coefficient ≤ 100)",
        ErreurSaisie::Incoherent => "statut et valeur incohérents",
    }
}

/// Texte libre borné, sans caractère de contrôle (hors saut de ligne).
pub(crate) fn texte_borne(v: &str, max: usize) -> Result<String, EntDbError> {
    let v = v.trim();
    if v.chars().count() > max || v.chars().any(|c| c.is_control() && c != '\n') {
        return Err(refus("texte trop long ou caractères interdits"));
    }
    Ok(v.to_string())
}

const MAX_SAISIES: usize = 500;

/// Relit le statut de la période **dans la transaction** avec un verrou partagé : un
/// changement de statut concurrent (UPDATE) attend la fin de l'écriture, ou l'inverse.
pub(crate) async fn statut_periode_tx(
    tx: &mut sqlx::PgConnection,
    etab: Uuid,
    periode_id: Uuid,
) -> Result<NotePeriodeStatut, EntDbError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT statut FROM ent_note_periode WHERE etablissement_id = $1 AND id = $2 FOR SHARE",
    )
    .bind(etab)
    .bind(periode_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?;
    let (s,) = row.ok_or_else(introuvable)?;
    Ok(NotePeriodeStatut::parse(&s).unwrap_or(NotePeriodeStatut::Fermee))
}

// ───────────────────────────── Lectures internes ─────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct PeriodeRow {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub statut: NotePeriodeStatut,
}

impl PeriodeRow {
    pub(crate) fn dto(&self) -> PeriodeNotesDto {
        PeriodeNotesDto {
            id: self.id,
            code: self.code.clone(),
            libelle: self.libelle.clone(),
            date_debut: self.date_debut,
            date_fin: self.date_fin,
            statut: self.statut.as_str().to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ServiceRow {
    pub id: Uuid,
    /// Année du service (lue pour les contrôles futurs de bascule d'année).
    pub _annee_id: Uuid,
    pub prof_user_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    /// Classe de rattachement d'un demi-groupe (sinon `classe_id`).
    pub classe_parente: Option<Uuid>,
    pub cible_code: String,
    pub professeur_principal: bool,
}

impl ServiceRow {
    fn dto(&self) -> Option<ServiceEvaluationDto> {
        Some(ServiceEvaluationDto {
            service_id: self.id,
            classe_id: self.classe_id,
            groupe_id: self.groupe_id,
            cible_code: self.cible_code.clone(),
            matiere_id: self.matiere_id?,
            matiere_libelle: self.matiere_libelle.clone(),
            prof_user_id: self.prof_user_id,
            professeur_principal: self.professeur_principal,
        })
    }
}

type ServiceTuple = (
    Uuid,
    Uuid,
    Uuid,
    Option<Uuid>,
    String,
    Option<Uuid>,
    Option<Uuid>,
    Option<Uuid>,
    String,
    bool,
);

const SELECT_SERVICE: &str = "SELECT s.id, s.annee_id, s.prof_user_id, s.matiere_id,
        COALESCE(m.libelle, NULLIF(s.matiere_libelle, ''), '—'),
        s.classe_id, s.groupe_id, COALESCE(s.classe_id, g.classe_id),
        COALESCE(c.code, g.code, ''), s.professeur_principal
   FROM ent_service_enseignement s
   LEFT JOIN ent_matiere m ON m.id = s.matiere_id AND m.etablissement_id = s.etablissement_id
   LEFT JOIN ent_classe c ON c.id = s.classe_id AND c.etablissement_id = s.etablissement_id
   LEFT JOIN ent_groupe g ON g.id = s.groupe_id AND g.etablissement_id = s.etablissement_id
  WHERE s.etablissement_id = $1";

fn service_de(t: ServiceTuple) -> ServiceRow {
    ServiceRow {
        id: t.0,
        _annee_id: t.1,
        prof_user_id: t.2,
        matiere_id: t.3,
        matiere_libelle: t.4,
        classe_id: t.5,
        groupe_id: t.6,
        classe_parente: t.7,
        cible_code: t.8,
        professeur_principal: t.9,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DevoirRow {
    pub id: Uuid,
    pub service_id: Option<Uuid>,
    pub periode_id: Uuid,
    pub matiere_id: Uuid,
    pub titre: String,
    pub date_devoir: NaiveDate,
    pub coef: f64,
    pub sur: f64,
    pub publie_le: Option<chrono::DateTime<Utc>>,
    pub publication_explicite: bool,
}

type DevoirTuple = (
    Uuid,
    Option<Uuid>,
    Uuid,
    Uuid,
    String,
    NaiveDate,
    f64,
    f64,
    Option<chrono::DateTime<Utc>>,
    bool,
);

const COLONNES_DEVOIR: &str = "id, service_id, periode_id, matiere_id, titre, date_devoir, coef, sur, \
    publie_le, publication_explicite";

fn devoir_de(t: DevoirTuple) -> DevoirRow {
    DevoirRow {
        id: t.0,
        service_id: t.1,
        periode_id: t.2,
        matiere_id: t.3,
        titre: t.4,
        date_devoir: t.5,
        coef: t.6,
        sur: t.7,
        publie_le: t.8,
        publication_explicite: t.9,
    }
}

impl DevoirRow {
    /// Visible des familles : devoir historique (sans publication explicite) ou publié.
    fn visible_famille(&self) -> bool {
        !self.publication_explicite || self.publie_le.is_some()
    }
}

/// Résultat lu : le plus récent par (devoir, élève) fait foi.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResultatLu {
    pub statut: StatutResultat,
    pub valeur: f64,
}

fn arrondir(m: Option<Moyenne>, a: Arrondi) -> Option<f64> {
    m.map(|x| x.arrondie(a))
}

fn stats_dto(s: Option<Statistiques>, a: Arrondi) -> StatsDto {
    match s {
        Some(s) => StatsDto {
            moyenne: Some(s.moyenne.arrondie(a)),
            min: Some(s.min.arrondie(a)),
            max: Some(s.max.arrondie(a)),
            effectif: s.effectif as u32,
        },
        None => StatsDto::default(),
    }
}

/// Résultat de calcul d'une classe sur une période (moteur commun : vue de classe,
/// conseil, bulletin, livret, relevé).
#[derive(Debug, Clone)]
pub(crate) struct CalculClasse {
    pub classe_id: Uuid,
    pub classe_code: String,
    pub periode: PeriodeRow,
    pub regles: ReglesCalcul,
    /// (id, nom, prénom), triés.
    pub eleves: Vec<(Uuid, String, String)>,
    /// (matière, libellé, code, coefficient de matière), triées par libellé.
    pub matieres: Vec<(Uuid, String, String, f64)>,
    pub moyennes: HashMap<(Uuid, Uuid), Moyenne>,
    pub generales: HashMap<Uuid, Moyenne>,
    pub stats_matiere: HashMap<Uuid, Statistiques>,
    pub stats_generale: Option<Statistiques>,
    /// Devoirs pris en compte et résultats (pour le relevé).
    pub devoirs: Vec<DevoirRow>,
    pub resultats: HashMap<(Uuid, Uuid), ResultatLu>,
}

impl PgEnt {
    pub(crate) async fn eva_periode(
        &self,
        etab: Uuid,
        periode_id: Uuid,
    ) -> Result<PeriodeRow, EntDbError> {
        let row: Option<(Uuid, String, String, NaiveDate, NaiveDate, String)> = sqlx::query_as(
            "SELECT id, code, libelle, date_debut, date_fin, statut
               FROM ent_note_periode WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(periode_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let (id, code, libelle, date_debut, date_fin, statut) = row.ok_or_else(introuvable)?;
        Ok(PeriodeRow {
            id,
            code,
            libelle,
            date_debut,
            date_fin,
            statut: NotePeriodeStatut::parse(&statut).unwrap_or(NotePeriodeStatut::Fermee),
        })
    }

    pub(crate) async fn eva_service(
        &self,
        etab: Uuid,
        service_id: Uuid,
    ) -> Result<ServiceRow, EntDbError> {
        let row: Option<ServiceTuple> = sqlx::query_as(&format!("{SELECT_SERVICE} AND s.id = $2"))
            .bind(etab)
            .bind(service_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?;
        row.map(service_de).ok_or_else(introuvable)
    }

    pub(crate) async fn eva_regles(&self, etab: Uuid) -> Result<ReglesCalcul, EntDbError> {
        let row: Option<(String, bool)> = sqlx::query_as(
            "SELECT arrondi, absent_compte_zero FROM ent_eva_regle WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(match row {
            Some((a, z)) => ReglesCalcul {
                arrondi: Arrondi::parse(&a).unwrap_or_default(),
                absent_compte_zero: z,
            },
            None => ReglesCalcul::default(),
        })
    }

    /// Effectif d'une classe ou d'un groupe sur la période (affectations du
    /// référentiel qui la chevauchent), trié par nom.
    pub(crate) async fn eva_effectif(
        &self,
        etab: Uuid,
        classe_id: Option<Uuid>,
        groupe_id: Option<Uuid>,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<(Uuid, String, String)>, EntDbError> {
        sqlx::query_as(
            "SELECT DISTINCT e.id, e.nom, e.prenom
               FROM ent_affectation a
               JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1
                AND (($2::uuid IS NOT NULL AND a.classe_id = $2) OR ($3::uuid IS NOT NULL AND a.groupe_id = $3))
                AND a.date_debut <= $5 AND (a.date_fin IS NULL OR a.date_fin >= $4)
                AND (e.date_sortie IS NULL OR e.date_sortie >= $4)
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(classe_id)
        .bind(groupe_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    async fn eva_devoirs_service(
        &self,
        etab: Uuid,
        service_id: Uuid,
        periode_id: Uuid,
    ) -> Result<Vec<DevoirRow>, EntDbError> {
        let rows: Vec<DevoirTuple> = sqlx::query_as(&format!(
            "SELECT {COLONNES_DEVOIR} FROM ent_devoir
              WHERE etablissement_id = $1 AND service_id = $2 AND periode_id = $3
              ORDER BY date_devoir, created_at, id"
        ))
        .bind(etab)
        .bind(service_id)
        .bind(periode_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(devoir_de).collect())
    }

    pub(crate) async fn eva_devoir(&self, etab: Uuid, id: Uuid) -> Result<DevoirRow, EntDbError> {
        let row: Option<DevoirTuple> = sqlx::query_as(&format!(
            "SELECT {COLONNES_DEVOIR} FROM ent_devoir WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        row.map(devoir_de).ok_or_else(introuvable)
    }

    /// Résultats (le plus récent par devoir et élève) des devoirs donnés.
    async fn eva_resultats(
        &self,
        etab: Uuid,
        devoirs: &[Uuid],
    ) -> Result<HashMap<(Uuid, Uuid), ResultatLu>, EntDbError> {
        if devoirs.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<(Uuid, Uuid, String, f64)> = sqlx::query_as(
            "SELECT DISTINCT ON (devoir_id, eleve_id) devoir_id, eleve_id, statut, valeur
               FROM ent_note
              WHERE etablissement_id = $1 AND devoir_id = ANY($2)
              ORDER BY devoir_id, eleve_id, created_at DESC, id DESC",
        )
        .bind(etab)
        .bind(devoirs)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(d, e, s, v)| {
                (
                    (d, e),
                    ResultatLu {
                        statut: StatutResultat::parse(&s).unwrap_or(StatutResultat::NonNote),
                        valeur: v,
                    },
                )
            })
            .collect())
    }

    /// Le professeur est-il celui du service, ou a-t-il `action` sur tout l'établissement ?
    async fn eva_titulaire_ou_etab(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        service: &ServiceRow,
        action: Action,
    ) -> Result<bool, EntDbError> {
        if Role::parse(role).is_none_or(|r| !r.is_staff()) {
            return Ok(false);
        }
        if service.prof_user_id == acteur {
            return Ok(true);
        }
        self.a_action_etablissement(etab, acteur, role, action).await
    }

    /// Professeur principal de la classe (année du service ou courante) ?
    pub(crate) async fn eva_est_pp(
        &self,
        etab: Uuid,
        acteur: Uuid,
        classe_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement
                             WHERE etablissement_id = $1 AND prof_user_id = $2
                               AND classe_id = $3 AND professeur_principal)",
        )
        .bind(etab)
        .bind(acteur)
        .bind(classe_id)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// L'acteur enseigne-t-il dans la classe (service sur la classe ou un de ses groupes) ?
    pub(crate) async fn eva_enseigne_classe(
        &self,
        etab: Uuid,
        acteur: Uuid,
        classe_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement s
                              LEFT JOIN ent_groupe g ON g.id = s.groupe_id AND g.etablissement_id = s.etablissement_id
                             WHERE s.etablissement_id = $1 AND s.prof_user_id = $2
                               AND (s.classe_id = $3 OR g.classe_id = $3))",
        )
        .bind(etab)
        .bind(acteur)
        .bind(classe_id)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// Classe de l'élève sur la période (affectation du référentiel), si une.
    pub(crate) async fn eva_classe_eleve(
        &self,
        etab: Uuid,
        eleve_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Option<Uuid>, EntDbError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT a.classe_id FROM ent_affectation a
              WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND a.classe_id IS NOT NULL
                AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $3)
              ORDER BY a.date_debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(debut)
        .bind(fin)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(row.map(|(c,)| c))
    }

    // ───────────────────────────── Référentiels ─────────────────────────────

    /// Services évaluables : ceux du professeur, ou tous ceux de l'année courante pour
    /// qui a `notes.saisir` / `notes.lire` sur l'établissement.
    pub async fn services_evaluables_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<ServiceEvaluationDto>, EntDbError> {
        if Role::parse(role).is_none_or(|r| !r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        let tous = self
            .a_action_etablissement(etab, acteur, role, Action::NotesSaisir)
            .await?
            || self
                .a_action_etablissement(etab, acteur, role, Action::NotesLire)
                .await?;
        let rows: Vec<ServiceTuple> = sqlx::query_as(&format!(
            "{SELECT_SERVICE} AND s.matiere_id IS NOT NULL AND ($2 OR s.prof_user_id = $3)
               AND EXISTS (SELECT 1 FROM ent_annee_scolaire y
                            WHERE y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
                              AND y.date_debut <= $4 AND y.date_fin >= $4)
             ORDER BY 9, 5"
        ))
        .bind(etab)
        .bind(tous)
        .bind(acteur)
        .bind(today_utc())
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows.into_iter().filter_map(|t| service_de(t).dto()).collect())
    }

    /// Périodes de notes de l'établissement (tout membre connecté).
    pub async fn periodes_notes(&self, etab: Uuid) -> Result<Vec<PeriodeNotesDto>, EntDbError> {
        let rows: Vec<(Uuid, String, String, NaiveDate, NaiveDate, String)> = sqlx::query_as(
            "SELECT id, code, libelle, date_debut, date_fin, statut
               FROM ent_note_periode WHERE etablissement_id = $1 ORDER BY date_debut, code",
        )
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, code, libelle, date_debut, date_fin, statut)| PeriodeNotesDto {
                id,
                code,
                libelle,
                date_debut,
                date_fin,
                statut,
            })
            .collect())
    }

    /// EVA-07 : ouvrir, passer en « appréciations seulement » ou fermer une période.
    pub async fn changer_statut_periode_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        periode_id: Uuid,
        statut: &str,
    ) -> Result<PeriodeNotesDto, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let statut = NotePeriodeStatut::parse(statut).ok_or(refus("statut de période inconnu"))?;
        let avant = self.eva_periode(etab, periode_id).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query("UPDATE ent_note_periode SET statut = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(periode_id)
            .bind(statut.as_str())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::NOTE_PERIODE_STATUT.to_string(),
            aggregate: AggregateKind::NotePeriode,
            aggregate_id: periode_id,
            eleve_id: None,
            payload: json!({ "statut": statut.as_str(), "avant": avant.statut.as_str() }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(PeriodeRow { statut, ..avant }.dto())
    }

    pub(crate) fn exiger_staff(role: &str) -> Result<(), EntDbError> {
        if Role::parse(role).is_some_and(|r| r.is_staff()) {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    pub async fn regles_calcul_for_actor(
        &self,
        etab: Uuid,
        role: &str,
    ) -> Result<ReglesCalculDto, EntDbError> {
        Self::exiger_staff(role)?;
        let r = self.eva_regles(etab).await?;
        Ok(ReglesCalculDto {
            arrondi: r.arrondi.as_str().to_string(),
            absent_compte_zero: r.absent_compte_zero,
        })
    }

    pub async fn definir_regles_calcul_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &ReglesCalculDto,
    ) -> Result<ReglesCalculDto, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let arrondi = Arrondi::parse(&body.arrondi).ok_or(refus("arrondi inconnu"))?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_eva_regle (etablissement_id, arrondi, absent_compte_zero, updated_by_user_id, updated_at)
             VALUES ($1, $2, $3, $4, now())
             ON CONFLICT (etablissement_id) DO UPDATE
                SET arrondi = EXCLUDED.arrondi, absent_compte_zero = EXCLUDED.absent_compte_zero,
                    updated_by_user_id = EXCLUDED.updated_by_user_id, updated_at = now()",
        )
        .bind(etab)
        .bind(arrondi.as_str())
        .bind(body.absent_compte_zero)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_eva::REGLES_MODIFIEES.to_string(),
            aggregate: AggregateKind::NotePeriode,
            aggregate_id: etab,
            eleve_id: None,
            payload: json!({ "arrondi": arrondi.as_str(), "absent_compte_zero": body.absent_compte_zero }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        self.regles_calcul_for_actor(etab, role).await
    }

    /// Coefficient d'une matière dans la moyenne générale d'une classe (`matieres.gerer`).
    pub async fn definir_coef_matiere_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &CoefMatiereBody,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::MatieresGerer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if domain_eleve::evaluation::centiemes(body.coef).is_none_or(|c| c <= 0 || c > 10_000) {
            return Err(refus(erreur_saisie(ErreurSaisie::Coefficient)));
        }
        let ok: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM ent_classe c JOIN ent_matiere m ON m.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.id = $2 AND m.id = $3",
        )
        .bind(etab)
        .bind(body.classe_id)
        .bind(body.matiere_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        if ok.is_none() {
            return Err(introuvable());
        }
        sqlx::query(
            "INSERT INTO ent_eva_coef_matiere (id, etablissement_id, classe_id, matiere_id, coef, updated_by_user_id)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (etablissement_id, classe_id, matiere_id) DO UPDATE
                SET coef = EXCLUDED.coef, updated_by_user_id = EXCLUDED.updated_by_user_id, updated_at = now()",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(body.classe_id)
        .bind(body.matiere_id)
        .bind(body.coef)
        .bind(acteur)
        .execute(self.pool())
        .await
        .map_err(db)?;
        Ok(())
    }

    // ───────────────────────────── Grille ─────────────────────────────

    /// EVA-02 : grille d'un service sur une période.
    pub async fn grille_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        service_id: Uuid,
        periode_id: Uuid,
    ) -> Result<GrilleDto, EntDbError> {
        Self::exiger_staff(role)?;
        let service = self.eva_service(etab, service_id).await?;
        let Some(service_dto) = service.dto() else {
            return Err(refus("service sans matière du référentiel"));
        };
        let periode = self.eva_periode(etab, periode_id).await?;
        let effectif = self
            .eva_effectif(etab, service.classe_id, service.groupe_id, periode.date_debut, periode.date_fin)
            .await?;
        let titulaire = self
            .eva_titulaire_ou_etab(etab, acteur, role, &service, Action::NotesLire)
            .await?;
        let effectif: Vec<_> = if titulaire {
            effectif
        } else {
            let autorises = self
                .eleves_autorises(etab, acteur, role, Action::NotesLire)
                .await?
                .unwrap_or_default();
            let visibles: Vec<_> = effectif
                .into_iter()
                .filter(|(id, _, _)| autorises.contains(id))
                .collect();
            if visibles.is_empty() {
                return Err(EntDbError::Forbidden);
            }
            visibles
        };
        let regles = self.eva_regles(etab).await?;
        let devoirs = self.eva_devoirs_service(etab, service_id, periode_id).await?;
        let ids: Vec<Uuid> = devoirs.iter().map(|d| d.id).collect();
        let resultats = self.eva_resultats(etab, &ids).await?;
        let matiere = service_dto.matiere_id;
        let apprs: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT eleve_id, texte FROM ent_note_appreciation
              WHERE etablissement_id = $1 AND periode_id = $2 AND matiere_id = $3",
        )
        .bind(etab)
        .bind(periode_id)
        .bind(matiere)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let apprs: HashMap<Uuid, String> = apprs.into_iter().collect();
        let mut moyennes = Vec::new();
        let mut lignes = Vec::with_capacity(effectif.len());
        for (eleve_id, nom, prenom) in effectif {
            let mut res = Vec::new();
            let mut calc = Vec::new();
            for d in &devoirs {
                if let Some(r) = resultats.get(&(d.id, eleve_id)) {
                    res.push(ResultatDto {
                        devoir_id: d.id,
                        statut: r.statut.as_str().to_string(),
                        valeur: (r.statut == StatutResultat::Note).then_some(r.valeur),
                    });
                    calc.push(ResultatCalcul {
                        statut: r.statut,
                        valeur: r.valeur,
                        sur: d.sur,
                        coef: d.coef,
                    });
                }
            }
            let m = moyenne_matiere(&calc, &regles);
            if let Some(x) = m {
                moyennes.push(x);
            }
            lignes.push(LigneGrilleDto {
                eleve_id,
                nom,
                prenom,
                resultats: res,
                moyenne: arrondir(m, regles.arrondi),
                appreciation: apprs.get(&eleve_id).cloned(),
            });
        }
        let visibles: HashSet<Uuid> = lignes.iter().map(|l| l.eleve_id).collect();
        let devoirs_dto = devoirs
            .iter()
            .map(|d| {
                // Statistiques du devoir dans son barème, sur les élèves affichés.
                let notes: Vec<Moyenne> = resultats
                    .iter()
                    .filter(|((dv, el), r)| {
                        *dv == d.id && visibles.contains(el) && r.statut == StatutResultat::Note
                    })
                    .filter_map(|(_, r)| domain_eleve::evaluation::centiemes(r.valeur))
                    .map(Moyenne::depuis_centiemes)
                    .collect();
                DevoirGrilleDto {
                    id: d.id,
                    titre: d.titre.clone(),
                    date_devoir: d.date_devoir,
                    coef: d.coef,
                    sur: d.sur,
                    publie_le: d.publie_le,
                    stats: stats_dto(statistiques(&notes), Arrondi::Centieme),
                }
            })
            .collect();
        Ok(GrilleDto {
            service: service_dto,
            periode: periode.dto(),
            saisie_ouverte: periode.statut.autorise_saisie_notes(),
            appreciations_ouvertes: periode.statut.autorise_appreciations(),
            arrondi: regles.arrondi.as_str().to_string(),
            devoirs: devoirs_dto,
            lignes,
            stats: stats_dto(statistiques(&moyennes), regles.arrondi),
        })
    }

    /// EVA-01 (grille) : crée une évaluation du service, à publication explicite.
    pub async fn creer_devoir_grille_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &CreerDevoirGrilleBody,
    ) -> Result<DevoirGrilleDto, EntDbError> {
        Self::exiger_staff(role)?;
        let service = self.eva_service(etab, body.service_id).await?;
        let matiere_id = service.matiere_id.ok_or(refus("service sans matière du référentiel"))?;
        if !self
            .eva_titulaire_ou_etab(etab, acteur, role, &service, Action::NotesSaisir)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let periode = self.eva_periode(etab, body.periode_id).await?;
        if !periode.statut.autorise_saisie_notes() {
            return Err(conflit("période verrouillée : saisie des notes fermée"));
        }
        if body.date_devoir < periode.date_debut || body.date_devoir > periode.date_fin {
            return Err(refus("date du devoir hors de la période"));
        }
        valider_devoir(body.sur, body.coef).map_err(|e| refus(erreur_saisie(e)))?;
        let titre = texte_borne(&body.titre, 200)?;
        if titre.is_empty() {
            return Err(refus("titre du devoir obligatoire"));
        }
        let id = Uuid::new_v4();
        // Classe de rattachement : la classe, ou celle du demi-groupe ; '' pour un
        // groupe transverse (le déclencheur `classe_id` n'invente alors aucune classe).
        let classe_code: String = match service.classe_parente {
            Some(c) => {
                let (code,): (String,) = sqlx::query_as(
                    "SELECT code FROM ent_classe WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(c)
                .fetch_one(self.pool())
                .await
                .map_err(db)?;
                code
            }
            None => String::new(),
        };
        let mut tx = self.pool().begin().await.map_err(db)?;
        if !statut_periode_tx(&mut tx, etab, periode.id).await?.autorise_saisie_notes() {
            return Err(conflit("période verrouillée : saisie des notes fermée"));
        }
        sqlx::query(
            "INSERT INTO ent_devoir (id, etablissement_id, periode_id, matiere_id, classe, classe_id,
                                     groupe_id, service_id, titre, date_devoir, coef, sur,
                                     created_by_user_id, publication_explicite)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,true)",
        )
        .bind(id)
        .bind(etab)
        .bind(periode.id)
        .bind(matiere_id)
        .bind(&classe_code)
        .bind(service.classe_parente)
        .bind(service.groupe_id)
        .bind(service.id)
        .bind(&titre)
        .bind(body.date_devoir)
        .bind(body.coef)
        .bind(body.sur)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::DEVOIR_CREATED.to_string(),
            aggregate: AggregateKind::Devoir,
            aggregate_id: id,
            eleve_id: None,
            payload: json!({
                "periode_id": periode.id,
                "matiere_id": matiere_id,
                "service_id": service.id,
                "coef": body.coef,
                "sur": body.sur,
            }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(DevoirGrilleDto {
            id,
            titre,
            date_devoir: body.date_devoir,
            coef: body.coef,
            sur: body.sur,
            publie_le: None,
            stats: StatsDto::default(),
        })
    }

    /// EVA-02 : saisie groupée des résultats d'un devoir (seulement les cellules
    /// modifiées). Statut `vide` : efface la saisie (période ouverte seulement).
    pub async fn saisir_grille_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        devoir_id: Uuid,
        body: &SaisieGrilleBody,
    ) -> Result<RapportSaisieDto, EntDbError> {
        Self::exiger_staff(role)?;
        if body.resultats.len() > MAX_SAISIES {
            return Err(refus("trop de résultats dans une seule saisie"));
        }
        let devoir = self.eva_devoir(etab, devoir_id).await?;
        let service_id = devoir.service_id.ok_or(refus("devoir hors grille (sans service)"))?;
        let service = self.eva_service(etab, service_id).await?;
        if !self
            .eva_titulaire_ou_etab(etab, acteur, role, &service, Action::NotesSaisir)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let periode = self.eva_periode(etab, devoir.periode_id).await?;
        if !periode.statut.autorise_saisie_notes() {
            return Err(conflit("période verrouillée : saisie des notes fermée"));
        }
        let effectif: HashSet<Uuid> = self
            .eva_effectif(etab, service.classe_id, service.groupe_id, periode.date_debut, periode.date_fin)
            .await?
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();
        let autorises = self
            .eleves_autorises(etab, acteur, role, Action::NotesSaisir)
            .await?;
        let mut rapport = RapportSaisieDto {
            enregistres: 0,
            inchanges: 0,
            refuses: Vec::new(),
        };
        let mut vus = HashSet::new();
        let mut a_ecrire: Vec<(Uuid, Option<(StatutResultat, f64)>)> = Vec::new();
        for r in &body.resultats {
            let refuser = |motif: &str| RefusSaisieDto {
                eleve_id: r.eleve_id,
                motif: motif.to_string(),
            };
            if !vus.insert(r.eleve_id) {
                rapport.refuses.push(refuser("élève en double dans la saisie"));
                continue;
            }
            if !effectif.contains(&r.eleve_id)
                || autorises.as_ref().is_some_and(|a| !a.contains(&r.eleve_id))
            {
                rapport.refuses.push(refuser("élève hors de l'effectif ou de votre portée"));
                continue;
            }
            if r.statut == "vide" {
                a_ecrire.push((r.eleve_id, None));
                continue;
            }
            let Some(statut) = StatutResultat::parse(&r.statut) else {
                rapport.refuses.push(refuser("statut inconnu"));
                continue;
            };
            match valider_resultat(statut, r.valeur, devoir.sur) {
                Ok(v) => a_ecrire.push((r.eleve_id, Some((statut, v)))),
                Err(e) => rapport.refuses.push(refuser(erreur_saisie(e))),
            }
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        if !statut_periode_tx(&mut tx, etab, devoir.periode_id).await?.autorise_saisie_notes() {
            return Err(conflit("période verrouillée : saisie des notes fermée"));
        }
        // Verrou du devoir : deux saisies concurrentes du même devoir sont sérialisées.
        sqlx::query("SELECT 1 FROM ent_devoir WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
            .bind(etab)
            .bind(devoir_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        for (eleve_id, saisie) in a_ecrire {
            let existant: Option<(Uuid, String, f64)> = sqlx::query_as(
                "SELECT id, statut, valeur FROM ent_note
                  WHERE etablissement_id = $1 AND devoir_id = $2 AND eleve_id = $3
                  ORDER BY created_at DESC, id DESC LIMIT 1",
            )
            .bind(etab)
            .bind(devoir_id)
            .bind(eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            match (saisie, existant) {
                (None, None) => rapport.inchanges += 1,
                (None, Some(_)) => {
                    let (corrigee,): (bool,) = sqlx::query_as(
                        "SELECT EXISTS (SELECT 1 FROM ent_note_correction c JOIN ent_note n
                                          ON n.id = c.note_id AND n.etablissement_id = c.etablissement_id
                                         WHERE c.etablissement_id = $1 AND n.devoir_id = $2 AND n.eleve_id = $3)",
                    )
                    .bind(etab)
                    .bind(devoir_id)
                    .bind(eleve_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                    if corrigee {
                        rapport.refuses.push(RefusSaisieDto {
                            eleve_id,
                            motif: "résultat corrigé après fermeture : effacement impossible".into(),
                        });
                        continue;
                    }
                    sqlx::query(
                        "DELETE FROM ent_note WHERE etablissement_id = $1 AND devoir_id = $2 AND eleve_id = $3",
                    )
                    .bind(etab)
                    .bind(devoir_id)
                    .bind(eleve_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    rapport.enregistres += 1;
                }
                (Some((s, v)), Some((_, s0, v0))) if s0 == s.as_str() && (v0 - v).abs() < 1e-9 => {
                    rapport.inchanges += 1;
                }
                (Some((s, v)), Some((id, _, _))) => {
                    sqlx::query(
                        "UPDATE ent_note SET statut = $3, valeur = $4, updated_at = now(), updated_by_user_id = $5
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(id)
                    .bind(s.as_str())
                    .bind(v)
                    .bind(acteur)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    rapport.enregistres += 1;
                }
                (Some((s, v)), None) => {
                    sqlx::query(
                        "INSERT INTO ent_note (id, etablissement_id, eleve_id, matiere_id, valeur, sur, date_note,
                                               auteur_user_id, libelle, devoir_id, periode_id, statut)
                         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
                    )
                    .bind(Uuid::new_v4())
                    .bind(etab)
                    .bind(eleve_id)
                    .bind(devoir.matiere_id)
                    .bind(v)
                    .bind(devoir.sur)
                    .bind(devoir.date_devoir)
                    .bind(acteur)
                    .bind(&devoir.titre)
                    .bind(devoir_id)
                    .bind(devoir.periode_id)
                    .bind(s.as_str())
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    rapport.enregistres += 1;
                }
            }
        }
        if rapport.enregistres > 0 {
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_eva::RESULTATS_SAISIS.to_string(),
                aggregate: AggregateKind::Devoir,
                aggregate_id: devoir_id,
                eleve_id: None,
                // Charge utile minimale : aucune note recopiée.
                payload: json!({ "periode_id": devoir.periode_id, "nombre": rapport.enregistres }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(rapport)
    }

    /// EVA-03 : publie les notes d'une évaluation aux familles (idempotent).
    pub async fn publier_devoir_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        devoir_id: Uuid,
    ) -> Result<DevoirGrilleDto, EntDbError> {
        Self::exiger_staff(role)?;
        let devoir = self.eva_devoir(etab, devoir_id).await?;
        let service_id = devoir.service_id.ok_or(refus("devoir hors grille (sans service)"))?;
        let service = self.eva_service(etab, service_id).await?;
        if !self
            .eva_titulaire_ou_etab(etab, acteur, role, &service, Action::NotesSaisir)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut publie_le = devoir.publie_le;
        if publie_le.is_none() {
            let mut tx = self.pool().begin().await.map_err(db)?;
            let maj: Option<(chrono::DateTime<Utc>,)> = sqlx::query_as(
                "UPDATE ent_devoir SET publie_le = now(), publie_par_user_id = $3
                  WHERE etablissement_id = $1 AND id = $2 AND publie_le IS NULL RETURNING publie_le",
            )
            .bind(etab)
            .bind(devoir_id)
            .bind(acteur)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let Some((le,)) = maj else {
                // Publication concurrente : déjà faite, aucun second événement.
                tx.rollback().await.map_err(db)?;
                return Ok(DevoirGrilleDto {
                    publie_le: self.eva_devoir(etab, devoir_id).await?.publie_le,
                    ..self.eva_devoir_dto(etab, devoir_id).await?
                });
            };
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_eva::EVALUATION_PUBLIEE.to_string(),
                aggregate: AggregateKind::Devoir,
                aggregate_id: devoir_id,
                eleve_id: None,
                payload: json!({
                    "periode_id": devoir.periode_id,
                    "service_id": service.id,
                    "classe_id": service.classe_id,
                    "groupe_id": service.groupe_id,
                }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
            tx.commit().await.map_err(db)?;
            publie_le = Some(le);
        }
        Ok(DevoirGrilleDto {
            id: devoir.id,
            titre: devoir.titre,
            date_devoir: devoir.date_devoir,
            coef: devoir.coef,
            sur: devoir.sur,
            publie_le,
            stats: StatsDto::default(),
        })
    }

    async fn eva_devoir_dto(&self, etab: Uuid, id: Uuid) -> Result<DevoirGrilleDto, EntDbError> {
        let d = self.eva_devoir(etab, id).await?;
        Ok(DevoirGrilleDto {
            id: d.id,
            titre: d.titre,
            date_devoir: d.date_devoir,
            coef: d.coef,
            sur: d.sur,
            publie_le: d.publie_le,
            stats: StatsDto::default(),
        })
    }

    /// EVA-14 : correction d'un résultat après fermeture (ou en phase « appréciations »),
    /// motif obligatoire, journalisée ; le bulletin devra être régénéré (nouvelle version).
    pub async fn corriger_note_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        devoir_id: Uuid,
        body: &CorrigerNoteBody,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let devoir = self.eva_devoir(etab, devoir_id).await?;
        let periode = self.eva_periode(etab, devoir.periode_id).await?;
        if periode.statut.autorise_saisie_notes() {
            return Err(conflit("période ouverte : corriger depuis la grille"));
        }
        let motif = texte_borne(&body.motif, 500)?;
        if motif.chars().count() < 3 {
            return Err(refus("motif de correction obligatoire"));
        }
        let statut = StatutResultat::parse(&body.statut).ok_or(refus("statut inconnu"))?;
        let valeur = valider_resultat(statut, body.valeur, devoir.sur).map_err(|e| refus(erreur_saisie(e)))?;
        self.exiger_eleve_existe(etab, body.eleve_id).await?;
        // L'élève doit appartenir à l'effectif du service (ou de la classe) du devoir.
        let (classe_id, groupe_id) = match devoir.service_id {
            Some(sid) => {
                let sv = self.eva_service(etab, sid).await?;
                (sv.classe_id, sv.groupe_id)
            }
            None => {
                let (c,): (Option<Uuid>,) = sqlx::query_as(
                    "SELECT classe_id FROM ent_devoir WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(devoir_id)
                .fetch_one(self.pool())
                .await
                .map_err(db)?;
                (c, None)
            }
        };
        let effectif = self
            .eva_effectif(etab, classe_id, groupe_id, periode.date_debut, periode.date_fin)
            .await?;
        if !effectif.iter().any(|(id, _, _)| *id == body.eleve_id) {
            return Err(refus("élève hors de l'effectif de l'évaluation"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        if statut_periode_tx(&mut tx, etab, devoir.periode_id).await?.autorise_saisie_notes() {
            return Err(conflit("période ouverte : corriger depuis la grille"));
        }
        sqlx::query("SELECT 1 FROM ent_devoir WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
            .bind(etab)
            .bind(devoir_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let existant: Option<(Uuid, String, f64)> = sqlx::query_as(
            "SELECT id, statut, valeur FROM ent_note
              WHERE etablissement_id = $1 AND devoir_id = $2 AND eleve_id = $3
              ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(etab)
        .bind(devoir_id)
        .bind(body.eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (note_id, ancien_statut, ancienne_valeur) = match existant {
            Some((id, s, v)) => {
                sqlx::query(
                    "UPDATE ent_note SET statut = $3, valeur = $4, updated_at = now(), updated_by_user_id = $5
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .bind(statut.as_str())
                .bind(valeur)
                .bind(acteur)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                (id, s, v)
            }
            None => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_note (id, etablissement_id, eleve_id, matiere_id, valeur, sur, date_note,
                                           auteur_user_id, libelle, devoir_id, periode_id, statut)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
                )
                .bind(id)
                .bind(etab)
                .bind(body.eleve_id)
                .bind(devoir.matiere_id)
                .bind(valeur)
                .bind(devoir.sur)
                .bind(devoir.date_devoir)
                .bind(acteur)
                .bind(&devoir.titre)
                .bind(devoir_id)
                .bind(devoir.periode_id)
                .bind(statut.as_str())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                (id, "vide".to_string(), 0.0)
            }
        };
        sqlx::query(
            "INSERT INTO ent_note_correction (id, etablissement_id, note_id, eleve_id, periode_id, ancien_statut,
                                              ancienne_valeur, nouveau_statut, nouvelle_valeur, motif, corrige_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(note_id)
        .bind(body.eleve_id)
        .bind(devoir.periode_id)
        .bind(&ancien_statut)
        .bind(ancienne_valeur)
        .bind(statut.as_str())
        .bind(valeur)
        .bind(&motif)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_eva::NOTE_CORRIGEE.to_string(),
            aggregate: AggregateKind::Note,
            aggregate_id: note_id,
            eleve_id: Some(body.eleve_id),
            payload: json!({ "periode_id": devoir.periode_id, "devoir_id": devoir_id }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────────── Calcul de classe ─────────────────────────────

    /// Moteur commun : moyennes de tous les élèves d'une classe sur une période.
    /// `publiees_seulement` : ne compter que les évaluations visibles des familles.
    pub(crate) async fn eva_calculer_classe(
        &self,
        etab: Uuid,
        classe_id: Uuid,
        periode_id: Uuid,
        publiees_seulement: bool,
    ) -> Result<CalculClasse, EntDbError> {
        let periode = self.eva_periode(etab, periode_id).await?;
        let row: Option<(String,)> =
            sqlx::query_as("SELECT code FROM ent_classe WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(classe_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?;
        let (classe_code,) = row.ok_or_else(introuvable)?;
        let regles = self.eva_regles(etab).await?;
        let eleves = self
            .eva_effectif(etab, Some(classe_id), None, periode.date_debut, periode.date_fin)
            .await?;
        let ids_eleves: Vec<Uuid> = eleves.iter().map(|e| e.0).collect();
        // Devoirs : ceux des services de la classe et de ses groupes, et les devoirs
        // historiques rattachés à la classe (sans service).
        let rows: Vec<DevoirTuple> = sqlx::query_as(&format!(
            "SELECT {COLONNES_DEVOIR} FROM ent_devoir d
              WHERE d.etablissement_id = $1 AND d.periode_id = $2
                AND (d.classe_id = $3
                     OR d.service_id IN (SELECT s.id FROM ent_service_enseignement s
                                           LEFT JOIN ent_groupe g ON g.id = s.groupe_id AND g.etablissement_id = s.etablissement_id
                                          WHERE s.etablissement_id = $1 AND (s.classe_id = $3 OR g.classe_id = $3)))
              ORDER BY d.date_devoir, d.created_at, d.id"
        ))
        .bind(etab)
        .bind(periode_id)
        .bind(classe_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let devoirs: Vec<DevoirRow> = rows
            .into_iter()
            .map(devoir_de)
            .filter(|d| !publiees_seulement || d.visible_famille())
            .collect();
        let ids_devoirs: Vec<Uuid> = devoirs.iter().map(|d| d.id).collect();
        let resultats = self.eva_resultats(etab, &ids_devoirs).await?;
        // Matières : celles des services de la classe (même sans note) et des devoirs.
        let mut ids_matieres: HashSet<Uuid> = devoirs.iter().map(|d| d.matiere_id).collect();
        let services: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT s.matiere_id FROM ent_service_enseignement s
               LEFT JOIN ent_groupe g ON g.id = s.groupe_id AND g.etablissement_id = s.etablissement_id
              WHERE s.etablissement_id = $1 AND s.matiere_id IS NOT NULL
                AND (s.classe_id = $2 OR g.classe_id = $2)",
        )
        .bind(etab)
        .bind(classe_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        ids_matieres.extend(services.into_iter().map(|(m,)| m));
        let ids_matieres: Vec<Uuid> = ids_matieres.into_iter().collect();
        let matieres_rows: Vec<(Uuid, String, String, Option<f64>)> = sqlx::query_as(
            "SELECT m.id, m.libelle, m.code, k.coef
               FROM ent_matiere m
               LEFT JOIN ent_eva_coef_matiere k
                 ON k.matiere_id = m.id AND k.etablissement_id = m.etablissement_id AND k.classe_id = $3
              WHERE m.etablissement_id = $1 AND m.id = ANY($2)
              ORDER BY m.libelle, m.code",
        )
        .bind(etab)
        .bind(&ids_matieres)
        .bind(classe_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let matieres: Vec<(Uuid, String, String, f64)> = matieres_rows
            .into_iter()
            .map(|(id, lib, code, coef)| (id, lib, code, coef.unwrap_or(1.0)))
            .collect();
        let mut moyennes = HashMap::new();
        let mut generales = HashMap::new();
        let mut par_matiere: HashMap<Uuid, Vec<Moyenne>> = HashMap::new();
        for eleve in &ids_eleves {
            let mut pour_generale = Vec::new();
            for (mid, _, _, coef_matiere) in &matieres {
                let calc: Vec<ResultatCalcul> = devoirs
                    .iter()
                    .filter(|d| d.matiere_id == *mid)
                    .filter_map(|d| {
                        resultats.get(&(d.id, *eleve)).map(|r| ResultatCalcul {
                            statut: r.statut,
                            valeur: r.valeur,
                            sur: d.sur,
                            coef: d.coef,
                        })
                    })
                    .collect();
                if let Some(m) = moyenne_matiere(&calc, &regles) {
                    moyennes.insert((*eleve, *mid), m);
                    par_matiere.entry(*mid).or_default().push(m);
                    pour_generale.push((m, *coef_matiere));
                }
            }
            if let Some(g) = moyenne_generale(&pour_generale) {
                generales.insert(*eleve, g);
            }
        }
        let stats_matiere = par_matiere
            .into_iter()
            .filter_map(|(m, v)| statistiques(&v).map(|s| (m, s)))
            .collect();
        let gen: Vec<Moyenne> = ids_eleves.iter().filter_map(|e| generales.get(e).copied()).collect();
        Ok(CalculClasse {
            classe_id,
            classe_code,
            periode,
            regles,
            eleves,
            matieres,
            moyennes,
            generales,
            stats_matiere,
            stats_generale: statistiques(&gen),
            devoirs,
            resultats,
        })
    }

    /// Vue de classe : moyennes par matière et générales, statistiques (`notes.lire`).
    pub async fn vue_classe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        classe_id: Uuid,
        periode_id: Uuid,
    ) -> Result<VueClasseDto, EntDbError> {
        Self::exiger_staff(role)?;
        let calc = self.eva_calculer_classe(etab, classe_id, periode_id, false).await?;
        let autorises = self
            .eleves_autorises(etab, acteur, role, Action::NotesLire)
            .await?;
        let a = calc.regles.arrondi;
        let lignes: Vec<LigneClasseDto> = calc
            .eleves
            .iter()
            .filter(|(id, _, _)| autorises.as_ref().is_none_or(|s| s.contains(id)))
            .map(|(id, nom, prenom)| LigneClasseDto {
                eleve_id: *id,
                nom: nom.clone(),
                prenom: prenom.clone(),
                moyennes: calc
                    .matieres
                    .iter()
                    .map(|(m, _, _, _)| arrondir(calc.moyennes.get(&(*id, *m)).copied(), a))
                    .collect(),
                moyenne_generale: arrondir(calc.generales.get(id).copied(), a),
            })
            .collect();
        if lignes.is_empty() && !calc.eleves.is_empty() {
            return Err(EntDbError::Forbidden);
        }
        if autorises.is_some() && calc.eleves.is_empty() {
            // Classe vide : seul qui a la lecture sur l'établissement la consulte.
            return Err(EntDbError::Forbidden);
        }
        Ok(VueClasseDto {
            classe_id,
            classe_code: calc.classe_code.clone(),
            periode: calc.periode.dto(),
            arrondi: a.as_str().to_string(),
            matieres: calc
                .matieres
                .iter()
                .map(|(m, lib, _, coef)| MatiereColonneDto {
                    matiere_id: *m,
                    libelle: lib.clone(),
                    coef: *coef,
                    stats: stats_dto(calc.stats_matiere.get(m).copied(), a),
                })
                .collect(),
            lignes,
            stats_generale: stats_dto(calc.stats_generale, a),
        })
    }

    /// EVA-13 : relevé d'un élève. Famille et élève : évaluations **publiées** seulement
    /// (moyennes de classe calculées sur les mêmes évaluations).
    pub async fn releve_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        periode_id: Uuid,
    ) -> Result<ReleveEleveDto, EntDbError> {
        if !self.actor_can_read_notes(etab, acteur, role, eleve_id).await? {
            return Err(EntDbError::Forbidden);
        }
        let staff = Role::parse(role).is_some_and(|r| r.is_staff());
        let periode = self.eva_periode(etab, periode_id).await?;
        let regles = self.eva_regles(etab).await?;
        let Some(classe_id) = self
            .eva_classe_eleve(etab, eleve_id, periode.date_debut, periode.date_fin)
            .await?
        else {
            return Ok(ReleveEleveDto {
                eleve_id,
                periode: periode.dto(),
                arrondi: regles.arrondi.as_str().to_string(),
                matieres: Vec::new(),
                moyenne_generale: None,
            });
        };
        let calc = self.eva_calculer_classe(etab, classe_id, periode_id, !staff).await?;
        let a = calc.regles.arrondi;
        let matieres = calc
            .matieres
            .iter()
            .map(|(m, lib, _, _)| ReleveMatiereDto {
                matiere_id: *m,
                matiere_libelle: lib.clone(),
                moyenne: arrondir(calc.moyennes.get(&(eleve_id, *m)).copied(), a),
                moyenne_classe: calc.stats_matiere.get(m).map(|s| s.moyenne.arrondie(a)),
                evaluations: calc
                    .devoirs
                    .iter()
                    .filter(|d| d.matiere_id == *m)
                    .filter_map(|d| {
                        calc.resultats.get(&(d.id, eleve_id)).map(|r| ReleveEvaluationDto {
                            titre: d.titre.clone(),
                            date_devoir: d.date_devoir,
                            coef: d.coef,
                            sur: d.sur,
                            statut: r.statut.as_str().to_string(),
                            valeur: (r.statut == StatutResultat::Note).then_some(r.valeur),
                        })
                    })
                    .collect(),
            })
            .collect();
        Ok(ReleveEleveDto {
            eleve_id,
            periode: calc.periode.dto(),
            arrondi: a.as_str().to_string(),
            matieres,
            moyenne_generale: arrondir(calc.generales.get(&eleve_id).copied(), a),
        })
    }
}
