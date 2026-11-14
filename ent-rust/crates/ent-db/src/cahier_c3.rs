//! Socle 20, lot C3 : agenda, cahier de l'élève et de la famille, « ce que j'ai manqué », coche « fait », résumé
//! hebdomadaire, abonnement au calendrier (`20_CAHIER_DE_TEXTES.md` §9.2, §9.3, §20.7, §20.8).
//!
//! **Lecteurs** : l'élève lui-même, ou un responsable *habilité* : lien actif (ni suspendu ni hors dates), exercice de
//! l'autorité parentale, aucune interdiction de communication visant ce responsable (parents séparés), élève mineur
//! (le lien d'un élève majeur est fermé tant que l'élève ne l'a pas rouvert), élève encore présent dans l'établissement
//! (C-D5). Tout autre rôle est refusé : le personnel lit le cahier par les routes du lot C2.
//!
//! **Lecture seule sur le modèle versionné** (`ent_cdt_*`) : seuls les travaux publiés et **visibles** (plage calme
//! passée) sont lus, après projection des publications échues. Ni copie, ni dépôt, ni motif d'aménagement ne sortent
//! d'ici ; la coche « fait » n'est lue que par l'élève (R-C14) ; aucune trace de lecture nominative n'est écrite (R-C42).
//!
//! **Jour et heure de l'établissement** partout (`ent_aujourdhui_etab`, `ent_cdt_heure_locale`), jamais le jour UTC.

use std::collections::HashSet;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use ent_auth::Role;
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::cahier_c3::*;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::outbox::Publier;
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme, TenantTx};

/// Plus longue période d'agenda demandée en une fois (jours).
const MAX_PERIODE_JOURS: i64 = 100;
/// Abonnements actifs au plus par compte et par élève.
const MAX_ABONNEMENTS: i64 = 5;

/// Avis **déclarés** par le cahier de textes (contrat d'avis du socle 8 : nature, destinataires, catégorie, gabarit).
/// Le consommateur `AvisSurEvenement` du socle 8 (N0/N11) n'est pas sur `ENT` : le cahier déclare ses avis sans en
/// dépendre ; les événements partent déjà par l'outbox (`ent_outbox`). Gabarits minimaux : jamais une consigne, une note, un
/// motif ni un nom d'élève dans l'avis ; les destinataires sont ceux que le socle 4 habilite (autorité parentale, aucune
/// interdiction de communication, élève mineur) ; l'élève majeur reçoit les siens.
pub const AVIS_CAHIER: &[AvisDeclare] = &[
    AvisDeclare {
        evenement: types::CAHIER_TRAVAIL_DONNE,
        categorie: "Notes et devoirs",
        destinataires: "eleves_du_cours, responsables_habilites",
        gabarit: "Nouveau travail en {matiere}, à rendre pour le {jour}.",
        desactivable: true,
    },
    AvisDeclare {
        evenement: types::CAHIER_SEANCE_MODIFIEE,
        categorie: "Notes et devoirs",
        destinataires: "eleves_du_cours, responsables_habilites",
        gabarit: "Le contenu ou le travail de {matiere} a été modifié.",
        desactivable: true,
    },
    AvisDeclare {
        evenement: types::CAHIER_RETIREE,
        categorie: "Notes et devoirs",
        destinataires: "eleves_du_cours, responsables_habilites",
        gabarit: "Un travail en {matiere} a été retiré.",
        desactivable: true,
    },
    AvisDeclare {
        evenement: types::CAHIER_RESUME_HEBDOMADAIRE,
        categorie: "Notes et devoirs",
        destinataires: "responsables_habilites, eleve_majeur",
        gabarit: "La semaine du {lundi} : {nb_travaux} travaux, {nb_evaluations} évaluations annoncées.",
        desactivable: true,
    },
];

fn invalide(message: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Invalide, message)
}

/// Vue du lecteur : détermine ce qui est servi (la coche n'est jamais servie à une famille).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LecteurC3 {
    Eleve,
    Famille,
}

impl LecteurC3 {
    pub fn code(self) -> &'static str {
        match self {
            Self::Eleve => "eleve",
            Self::Famille => "famille",
        }
    }
}

fn borner(du: NaiveDate, au: NaiveDate) -> Result<(), EntDbError> {
    if au < du {
        return Err(invalide("la période est à l'envers"));
    }
    if (au - du).num_days() > MAX_PERIODE_JOURS {
        return Err(invalide("période trop longue"));
    }
    Ok(())
}

/// Lundi de la semaine d'un jour.
pub fn lundi_de(jour: NaiveDate) -> NaiveDate {
    jour - Duration::days(i64::from(jour.weekday().num_days_from_monday()))
}

#[derive(Debug, FromRow)]
struct LigneTravailRow {
    id: Uuid,
    cours_id: Uuid,
    matiere: String,
    titre: String,
    nature: String,
    consigne: String,
    jour: NaiveDate,
    heure: String,
    duree: Option<i16>,
    rendu: String,
    auteur: String,
    statut: Option<String>,
    fait: bool,
    prolongee: bool,
    est_evaluation: bool,
    evaluation_id: Option<Uuid>,
    modifie_le: DateTime<Utc>,
    version: i32,
}

/// Travaux publiés et visibles d'un élève, par échéance effective, dans le public du cours **au jour de l'établissement**
/// (`$5`) ; un travail à public restreint n'est lu que par ses élèves ciblés.
const SQL_TRAVAUX_ELEVE: &str = "
SELECT t.id, t.cours_id, c.libelle AS matiere, t.titre, n.libelle AS nature, t.consigne,
       ent_aujourdhui_etab(t.etablissement_id, eff.le) AS jour,
       to_char(ent_cdt_heure_locale(t.etablissement_id, eff.le), 'HH24:MI') AS heure,
       t.duree_estimee_min AS duree, t.rendu,
       ent_cdt_nom_affichage(t.etablissement_id, t.auteur_user_id) AS auteur,
       ent_cdt_statut_eleve(t.etablissement_id, t.id, $2, eff.le, t.rendu) AS statut,
       EXISTS (SELECT 1 FROM ent_cdt_coche k
                WHERE k.etablissement_id = t.etablissement_id AND k.travail_id = t.id AND k.eleve_id = $2) AS fait,
       (eff.le <> t.echeance_le) AS prolongee,
       (t.evaluation_id IS NOT NULL) AS est_evaluation, t.evaluation_id,
       t.modifie_le, t.version
  FROM ent_cdt_travail t
  JOIN ent_cdt_nature n ON n.etablissement_id = t.etablissement_id AND n.id = t.nature_id
  JOIN ent_cours c ON c.etablissement_id = t.etablissement_id AND c.id = t.cours_id
 CROSS JOIN LATERAL (SELECT ent_cdt_echeance_effective(t.etablissement_id, t.id, $2, t.echeance_le) AS le) eff
 WHERE t.etablissement_id = $1 AND t.etat = 'publie' AND t.visible_a IS NOT NULL AND t.visible_a <= now()
   AND ent_aujourdhui_etab(t.etablissement_id, eff.le) BETWEEN $3 AND $4
   AND (t.public = 'cours'
        OR EXISTS (SELECT 1 FROM ent_cdt_travail_cible k
                    WHERE k.etablissement_id = t.etablissement_id AND k.travail_id = t.id AND k.eleve_id = $2))
   AND EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $5, t.cours_id, NULL, $2, false))
 ORDER BY eff.le, t.titre, t.id";

#[derive(Debug, FromRow)]
struct LigneSortieRow {
    id: Uuid,
    titre: String,
    jour: NaiveDate,
    heure: String,
}

/// Sorties qui concernent l'élève (participant inscrit ou prêt d'une sortie ouverte) : lecture minimale du socle 42,
/// qui reste propriétaire des états et de la visibilité (titre, jour, heure seulement).
const SQL_SORTIES_ELEVE: &str = "
SELECT s.id, s.titre, ent_aujourdhui_etab(s.etablissement_id, s.debut) AS jour,
       to_char(ent_cdt_heure_locale(s.etablissement_id, s.debut), 'HH24:MI') AS heure
  FROM ent_sortie s
  JOIN ent_sortie_participant p ON p.etablissement_id = s.etablissement_id AND p.sortie_id = s.id
 WHERE s.etablissement_id = $1 AND p.eleve_id = $2
   AND s.etat IN ('ouverte', 'prete', 'en_cours', 'rentree')
   AND p.etat IN ('inscrit', 'pret', 'parti', 'rentre', 'remis')
   AND ent_aujourdhui_etab(s.etablissement_id, s.debut) BETWEEN $3 AND $4
 ORDER BY s.debut, s.id";

/// Lignes de l'agenda d'un élève, triées par jour, heure, genre. `avec_coche` : élève seulement.
pub(crate) async fn lignes_agenda(
    tx: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    du: NaiveDate,
    au: NaiveDate,
    aujourdhui: NaiveDate,
    avec_coche: bool,
) -> Result<Vec<AgendaLigneDto>, EntDbError> {
    let travaux: Vec<LigneTravailRow> = sqlx::query_as(SQL_TRAVAUX_ELEVE)
        .bind(etab)
        .bind(eleve)
        .bind(du)
        .bind(au)
        .bind(aujourdhui)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
    // Une évaluation annoncée au socle 21 et le travail qui l'annonce ne sont comptés qu'une fois (CC-55).
    let annoncees: HashSet<Uuid> = travaux.iter().filter_map(|t| t.evaluation_id).collect();
    let mut lignes: Vec<AgendaLigneDto> = travaux
        .into_iter()
        .map(|t| AgendaLigneDto {
            genre: if t.est_evaluation { GENRE_EVALUATION } else { GENRE_TRAVAIL }.to_string(),
            id: t.id,
            cours_id: Some(t.cours_id),
            matiere: t.matiere,
            titre: t.titre,
            nature: Some(t.nature),
            consigne: Some(t.consigne),
            jour: t.jour,
            heure: Some(t.heure),
            duree_estimee_min: t.duree,
            rendu: Some(t.rendu),
            auteur: Some(t.auteur),
            statut: t.statut,
            fait: avec_coche.then_some(t.fait),
            echeance_prolongee: t.prolongee,
            modifie_le: (t.version > 1).then_some(t.modifie_le),
            version: t.version,
        })
        .collect();
    for e in PgEnt::eva_calendrier(&mut *tx, etab, &[eleve], du, au, aujourdhui).await? {
        if annoncees.contains(&e.evaluation.id) {
            continue;
        }
        lignes.push(AgendaLigneDto {
            genre: GENRE_EVALUATION.to_string(),
            id: e.evaluation.id,
            cours_id: e.evaluation.cours_id,
            matiere: e.evaluation.matiere,
            titre: e.evaluation.titre,
            nature: None,
            consigne: None,
            jour: e.evaluation.jour,
            heure: None,
            duree_estimee_min: None,
            rendu: None,
            auteur: None,
            statut: None,
            fait: None,
            echeance_prolongee: false,
            modifie_le: None,
            version: 1,
        });
    }
    let sorties: Vec<LigneSortieRow> = sqlx::query_as(SQL_SORTIES_ELEVE)
        .bind(etab)
        .bind(eleve)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
    for s in sorties {
        lignes.push(AgendaLigneDto {
            genre: GENRE_SORTIE.to_string(),
            id: s.id,
            cours_id: None,
            matiere: "Sortie".to_string(),
            titre: s.titre,
            nature: None,
            consigne: None,
            jour: s.jour,
            heure: Some(s.heure),
            duree_estimee_min: None,
            rendu: None,
            auteur: None,
            statut: None,
            fait: None,
            echeance_prolongee: false,
            modifie_le: None,
            version: 1,
        });
    }
    let rang = |g: &str| match g {
        GENRE_EVALUATION => 0,
        GENRE_EPREUVE => 1,
        GENRE_SORTIE => 2,
        _ => 3,
    };
    lignes.sort_by(|a, b| {
        (a.jour, a.heure.as_deref().unwrap_or(""), rang(&a.genre), &a.titre, a.id)
            .cmp(&(b.jour, b.heure.as_deref().unwrap_or(""), rang(&b.genre), &b.titre, b.id))
    });
    Ok(lignes)
}

#[derive(Debug, FromRow)]
struct SeanceManqueeRow {
    seance_id: Uuid,
    jour: NaiveDate,
    debut: String,
    fin: String,
    cours_id: Uuid,
    matiere: String,
    entree_id: Option<Uuid>,
    titre: Option<String>,
    contenu: Option<String>,
    auteur: Option<String>,
}

#[derive(Debug, FromRow)]
struct TravailManqueRow {
    id: Uuid,
    titre: String,
    consigne: String,
    jour: NaiveDate,
    heure: String,
}

impl PgEnt {
    /// Élève dont l'acteur lit l'agenda et le cahier : lui-même (élève), ou son enfant (responsable habilité).
    /// Les conditions sont relues **à chaque lecture** (un lien suspendu, un élève devenu majeur ou parti ferme l'accès).
    pub(crate) async fn cdt_lecteur(
        &self,
        a: &Acteur<'_>,
        eleve_id: Option<Uuid>,
    ) -> Result<(Uuid, LecteurC3), EntDbError> {
        let etab = a.etablissement_id;
        let role = Role::parse(a.role);
        let (eleve, vue) = match role {
            Some(Role::Eleve) => {
                let propre = self.eleve_du_compte(a).await?.ok_or(EntDbError::Forbidden)?;
                if eleve_id.is_some_and(|e| e != propre) {
                    return Err(EntDbError::Forbidden);
                }
                (propre, LecteurC3::Eleve)
            }
            Some(Role::Parent) => {
                let eleve = eleve_id.ok_or_else(|| invalide("indiquez l'élève"))?;
                let (ok,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien_communicant l
                        JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
                       WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.eleve_id = $3 AND m.role = 'parent'
                         AND l.autorite_parentale)
                     AND NOT ent_cdt_eleve_majeur($1, $3)",
                )
                .bind(etab)
                .bind(a.user_id)
                .bind(eleve)
                .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
                .await
                .map_err(db)?;
                if !ok {
                    return Err(EntDbError::Forbidden);
                }
                (eleve, LecteurC3::Famille)
            }
            _ => return Err(EntDbError::Forbidden),
        };
        // C-D5 : un élève parti n'a plus de lecture du cours (ni lui, ni sa famille).
        let (present,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve e WHERE e.etablissement_id = $1 AND e.id = $2
                             AND (e.date_sortie IS NULL OR e.date_sortie > ent_aujourdhui_etab($1)))",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        if !present {
            return Err(EntDbError::Forbidden);
        }
        Ok((eleve, vue))
    }

    /// CDT-08, 160, 177, 178 : l'agenda de l'élève (travaux, évaluations annoncées, sorties) sur une période.
    pub async fn cdt_agenda(
        &self,
        a: &Acteur<'_>,
        eleve_id: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<AgendaDto, EntDbError> {
        borner(du, au)?;
        let (eleve, vue) = self.cdt_lecteur(a, eleve_id).await?;
        self.cdt_projeter_echus(a.etablissement_id).await?;
        let aujourdhui = self.jour_etab(a.etablissement_id).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let lignes = lignes_agenda(
            &mut tx,
            a.etablissement_id,
            eleve,
            du,
            au,
            aujourdhui,
            vue == LecteurC3::Eleve,
        )
        .await?;
        Ok(AgendaDto {
            eleve_id: eleve,
            du,
            au,
            vue: vue.code().to_string(),
            lignes,
        })
    }

    /// CDT-162 : ce que l'élève a manqué : le contenu publié des séances où il était absent (absence constatée par le
    /// socle 12, ligne non retirée) et les travaux donnés pendant ces séances.
    pub async fn cdt_manque(
        &self,
        a: &Acteur<'_>,
        eleve_id: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<ManqueDto, EntDbError> {
        borner(du, au)?;
        let (eleve, _vue) = self.cdt_lecteur(a, eleve_id).await?;
        let etab = a.etablissement_id;
        self.cdt_projeter_echus(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Absence à la séance (créneau ou séance datée) ou à la demi-journée (matin avant 12 h 30). Le cours doit avoir
        // compté l'élève dans son public ce jour-là ; une absence retirée ne compte pas.
        let seances: Vec<SeanceManqueeRow> = sqlx::query_as(
            "SELECT DISTINCT ON (s.id) s.id AS seance_id, s.jour,
                    to_char(s.heure_debut, 'HH24:MI') AS debut, to_char(s.heure_fin, 'HH24:MI') AS fin,
                    cc.cours_id, c.libelle AS matiere, e.id AS entree_id, e.titre, e.contenu,
                    CASE WHEN e.id IS NULL THEN NULL
                         ELSE ent_cdt_nom_affichage(e.etablissement_id, e.auteur_user_id) END AS auteur
               FROM ent_asd_absence_seance a
               JOIN ent_edt_seance s ON s.etablissement_id = a.etablissement_id AND s.jour = a.jour
                AND ((a.kind = 'seance' AND (s.creneau_id = a.seance_ref OR s.id = a.seance_ref))
                  OR (a.kind = 'demi_journee'
                      AND ((a.periode = 'matin' AND s.heure_debut < TIME '12:30')
                        OR (a.periode = 'apres_midi' AND s.heure_debut >= TIME '12:30'))))
              CROSS JOIN LATERAL (SELECT ent_cdt_cours_de_seance(s.etablissement_id, s.id) AS cours_id) cc
               JOIN ent_cours c ON c.etablissement_id = s.etablissement_id AND c.id = cc.cours_id
               LEFT JOIN ent_cdt_entree e ON e.etablissement_id = s.etablissement_id AND e.seance_id = s.id
                AND e.cours_id = cc.cours_id AND e.etat = 'publiee' AND e.visible_a IS NOT NULL AND e.visible_a <= now()
              WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND a.retiree_le IS NULL
                AND a.jour BETWEEN $3 AND $4 AND s.statut <> 'annulee'
                AND EXISTS (SELECT 1 FROM ent_public_cours_jour($1, a.jour, cc.cours_id, NULL, $2, false))
              ORDER BY s.id",
        )
        .bind(etab)
        .bind(eleve)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for s in seances {
            let notions: Vec<(String,)> = match s.entree_id {
                Some(id) => sqlx::query_as(
                    "SELECT libelle FROM ent_cdt_entree_notion WHERE etablissement_id = $1 AND entree_id = $2 ORDER BY ordre",
                )
                .bind(etab)
                .bind(id)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?,
                None => Vec::new(),
            };
            let travaux: Vec<TravailManqueRow> = sqlx::query_as(
                "SELECT t.id, t.titre, t.consigne,
                        ent_aujourdhui_etab(t.etablissement_id, eff.le) AS jour,
                        to_char(ent_cdt_heure_locale(t.etablissement_id, eff.le), 'HH24:MI') AS heure
                   FROM ent_cdt_travail t
                  CROSS JOIN LATERAL (SELECT ent_cdt_echeance_effective(t.etablissement_id, t.id, $2, t.echeance_le) AS le) eff
                  WHERE t.etablissement_id = $1 AND t.seance_donnee_id = $3 AND t.cours_id = $4
                    AND t.etat = 'publie' AND t.visible_a IS NOT NULL AND t.visible_a <= now()
                    AND (t.public = 'cours'
                         OR EXISTS (SELECT 1 FROM ent_cdt_travail_cible k
                                     WHERE k.etablissement_id = t.etablissement_id AND k.travail_id = t.id AND k.eleve_id = $2))
                  ORDER BY eff.le, t.titre, t.id",
            )
            .bind(etab)
            .bind(eleve)
            .bind(s.seance_id)
            .bind(s.cours_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            if s.entree_id.is_none() && travaux.is_empty() {
                continue; // rien n'a été publié pour cette séance : rien à rattraper
            }
            out.push(ManqueSeanceDto {
                seance_id: s.seance_id,
                jour: s.jour,
                heure_debut: s.debut,
                heure_fin: s.fin,
                matiere: s.matiere,
                auteur: s.auteur,
                titre: s.titre,
                contenu: s.contenu,
                notions: notions.into_iter().map(|(n,)| n).collect(),
                travaux: travaux
                    .into_iter()
                    .map(|t| ManqueTravailDto {
                        id: t.id,
                        titre: t.titre,
                        consigne: t.consigne,
                        echeance_jour: t.jour,
                        echeance_heure: Some(t.heure),
                    })
                    .collect(),
            });
        }
        out.sort_by(|x, y| (x.jour, &x.heure_debut, x.seance_id).cmp(&(y.jour, &y.heure_debut, y.seance_id)));
        Ok(ManqueDto {
            eleve_id: eleve,
            du,
            au,
            seances: out,
        })
    }

    /// CDT-09, 163 : l'élève coche (ou décoche) « fait ». Outil **personnel** : jamais servi à un adulte (R-C14), sans
    /// aucun effet sur le statut constaté. Réservé à l'élève lui-même, pour un travail qu'il peut lire.
    pub async fn cdt_cocher(&self, a: &Acteur<'_>, travail_id: Uuid, fait: bool) -> Result<(), EntDbError> {
        if Role::parse(a.role) != Some(Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        let (eleve, _vue) = self.cdt_lecteur(a, None).await?;
        let etab = a.etablissement_id;
        self.cdt_projeter_echus(etab).await?;
        let aujourdhui = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Le travail doit être publié, visible et concerner l'élève : sinon il n'existe pas pour lui (pas de sondage).
        let (concerne,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_cdt_travail t
                             WHERE t.etablissement_id = $1 AND t.id = $2 AND t.etat = 'publie'
                               AND t.visible_a IS NOT NULL AND t.visible_a <= now()
                               AND (t.public = 'cours'
                                    OR EXISTS (SELECT 1 FROM ent_cdt_travail_cible k
                                                WHERE k.etablissement_id = t.etablissement_id AND k.travail_id = t.id
                                                  AND k.eleve_id = $3))
                               AND EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $4, t.cours_id, NULL, $3, false)))",
        )
        .bind(etab)
        .bind(travail_id)
        .bind(eleve)
        .bind(aujourdhui)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !concerne {
            return Err(introuvable());
        }
        if fait {
            sqlx::query(
                "INSERT INTO ent_cdt_coche (etablissement_id, travail_id, eleve_id, fait_le)
                 VALUES ($1, $2, $3, now()) ON CONFLICT (travail_id, eleve_id) DO NOTHING",
            )
            .bind(etab)
            .bind(travail_id)
            .bind(eleve)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        } else {
            // Décocher : la coche est un outil de l'élève, sans valeur de preuve ; on la retire.
            sqlx::query("DELETE FROM ent_cdt_coche WHERE etablissement_id = $1 AND travail_id = $2 AND eleve_id = $3")
                .bind(etab)
                .bind(travail_id)
                .bind(eleve)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// CDT-180, 183 : résumé de la semaine (lundi → dimanche) : décomptes et lignes. Semaine courante par défaut.
    pub async fn cdt_resume_semaine(
        &self,
        a: &Acteur<'_>,
        eleve_id: Option<Uuid>,
        semaine: Option<NaiveDate>,
    ) -> Result<ResumeSemaineDto, EntDbError> {
        let (eleve, vue) = self.cdt_lecteur(a, eleve_id).await?;
        let etab = a.etablissement_id;
        self.cdt_projeter_echus(etab).await?;
        let aujourdhui = self.jour_etab(etab).await?;
        let lundi = lundi_de(semaine.unwrap_or(aujourdhui));
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        resume_de(&mut tx, etab, eleve, lundi, aujourdhui, vue == LecteurC3::Eleve).await
    }

    // ───────────────────────────── Abonnement au calendrier ─────────────────────────────

    /// CDT-176 : crée un abonnement (jeton révocable). L'adresse n'est donnée qu'une fois. Un responsable s'abonne au
    /// calendrier d'un enfant qu'il a le droit de lire ; un élève, au sien.
    pub async fn cdt_abonnement_creer(
        &self,
        a: &Acteur<'_>,
        eleve_id: Option<Uuid>,
    ) -> Result<AbonnementCreeDto, EntDbError> {
        let (eleve, _) = self.cdt_lecteur(a, eleve_id).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (actifs,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_cdt_abonnement
              WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3 AND revoque_le IS NULL",
        )
        .bind(etab)
        .bind(a.user_id)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if actifs >= MAX_ABONNEMENTS {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "trop d'abonnements actifs : révoquez-en un avant d'en créer un autre",
            ));
        }
        // 244 bits tirés du générateur du système (deux UUID v4), jamais relus : seule l'empreinte est gardée.
        let secret = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let id = Uuid::new_v4();
        let (cree_le, cree_jour): (DateTime<Utc>, NaiveDate) = sqlx::query_as(
            "INSERT INTO ent_cdt_abonnement (id, etablissement_id, user_id, eleve_id, jeton_hash)
             VALUES ($1, $2, $3, $4, $5) RETURNING cree_le, ent_aujourdhui_etab(etablissement_id, cree_le)",
        )
        .bind(id)
        .bind(etab)
        .bind(a.user_id)
        .bind(eleve)
        .bind(empreinte(&secret))
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(AbonnementCreeDto {
            abonnement: AbonnementDto {
                id,
                eleve_id: eleve,
                cree_le,
                cree_jour,
            },
            chemin: format!("/cahier/c3/calendrier/{}-{}.ics", etab.simple(), secret),
        })
    }

    /// Abonnements actifs de l'acteur (jamais le jeton).
    pub async fn cdt_abonnements(&self, a: &Acteur<'_>) -> Result<Vec<AbonnementDto>, EntDbError> {
        // Seuls l'élève et ses responsables ont des abonnements : le personnel n'en a pas (refus, pas une liste vide).
        if !matches!(Role::parse(a.role), Some(Role::Eleve | Role::Parent)) {
            return Err(EntDbError::Forbidden);
        }
        let rows: Vec<(Uuid, Uuid, DateTime<Utc>, NaiveDate)> = sqlx::query_as(
            "SELECT id, eleve_id, cree_le, ent_aujourdhui_etab(etablissement_id, cree_le) FROM ent_cdt_abonnement
              WHERE etablissement_id = $1 AND user_id = $2 AND revoque_le IS NULL ORDER BY cree_le, id",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, eleve_id, cree_le, cree_jour)| AbonnementDto {
                id,
                eleve_id,
                cree_le,
                cree_jour,
            })
            .collect())
    }

    /// Révoque un abonnement de l'acteur (celui d'un autre compte n'existe pas pour lui).
    pub async fn cdt_abonnement_revoquer(&self, a: &Acteur<'_>, id: Uuid) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE ent_cdt_abonnement SET revoque_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND user_id = $3 AND revoque_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Flux du calendrier pour un jeton (route publique, sans session). Les droits du **compte abonné** sont relus à chaque
    /// lecture : un abonnement révoqué, un lien suspendu, un élève devenu majeur ou parti ne donne plus rien (404).
    /// Renvoie le nom du calendrier et les lignes **sans consigne** (socle 9), de la veille à +120 jours.
    pub async fn cdt_calendrier_par_jeton(&self, jeton: &str) -> Result<(String, Vec<AgendaLigneDto>), EntDbError> {
        let (etab, secret) = decouper_jeton(jeton).ok_or_else(introuvable)?;
        let (user, eleve): (Uuid, Uuid) = sqlx::query_as(
            "SELECT user_id, eleve_id FROM ent_cdt_abonnement
              WHERE etablissement_id = $1 AND jeton_hash = $2 AND revoque_le IS NULL",
        )
        .bind(etab)
        .bind(empreinte(secret))
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let (role,): (String,) = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2
              ORDER BY (role = 'parent') DESC, role LIMIT 1",
        )
        .bind(etab)
        .bind(user)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let acteur = Acteur {
            etablissement_id: etab,
            user_id: user,
            role: role.as_str(),
        };
        // Les droits du compte sont relus : tout refus devient « introuvable » (le jeton ne dit rien de plus).
        let (lu, _) = self.cdt_lecteur(&acteur, Some(eleve)).await.map_err(|_| introuvable())?;
        self.cdt_projeter_echus(etab).await?;
        let aujourdhui = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes = lignes_agenda(
            &mut tx,
            etab,
            lu,
            aujourdhui - Duration::days(1),
            aujourdhui + Duration::days(MAX_PERIODE_JOURS),
            aujourdhui,
            false,
        )
        .await?;
        Ok((
            "ScolIA — travail à faire".to_string(),
            lignes
                .into_iter()
                .map(|mut l| {
                    l.consigne = None;
                    l.auteur = None;
                    l
                })
                .collect(),
        ))
    }

    // ───────────────────────────── Résumé hebdomadaire (avis) ─────────────────────────────

    /// Prépare les résumés hebdomadaires d'une semaine : un seul envoi par élève et par semaine (même sous passages
    /// concurrents), et seulement si la semaine contient au moins une échéance. L'événement ne porte que des décomptes ;
    /// le consommateur d'avis (socle 8) choisit les destinataires habilités et les canaux. Renvoie le nombre d'élèves.
    pub async fn cdt_resumes_preparer(&self, etab: Uuid, lundi: NaiveDate) -> Result<usize, EntDbError> {
        if lundi.weekday().num_days_from_monday() != 0 {
            return Err(invalide("la semaine est identifiée par son lundi"));
        }
        self.cdt_projeter_echus(etab).await?;
        let aujourdhui = self.jour_etab(etab).await?;
        let eleves: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT e.id FROM ent_eleve e
              WHERE e.etablissement_id = $1 AND (e.date_sortie IS NULL OR e.date_sortie > $2)
                AND EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $2, NULL, NULL, e.id, false))
              ORDER BY e.id",
        )
        .bind(etab)
        .bind(aujourdhui)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?;
        let mut envoyes = 0;
        for (eleve,) in eleves {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let r = resume_de(&mut tx, etab, eleve, lundi, aujourdhui, false).await?;
            if r.nb_travaux + r.nb_evaluations + r.nb_autres == 0 {
                continue;
            }
            let neuf = sqlx::query(
                "INSERT INTO ent_cdt_resume_envoi (etablissement_id, eleve_id, semaine_debut, nb_travaux, nb_evaluations)
                 VALUES ($1, $2, $3, $4, $5) ON CONFLICT (eleve_id, semaine_debut) DO NOTHING",
            )
            .bind(etab)
            .bind(eleve)
            .bind(lundi)
            .bind(r.nb_travaux as i16)
            .bind(r.nb_evaluations as i16)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
            if neuf == 1 {
                let conn: &mut PgConnection = &mut tx;
                conn.publier(MetierEventDraft {
                    etablissement_id: etab,
                    type_name: types::CAHIER_RESUME_HEBDOMADAIRE.to_string(),
                    aggregate: AggregateKind::CahierTexte,
                    aggregate_id: eleve,
                    eleve_id: Some(eleve),
                    payload: serde_json::json!({
                        "semaine_debut": lundi,
                        "nb_travaux": r.nb_travaux,
                        "nb_evaluations": r.nb_evaluations,
                    }),
                    actor_user_id: None,
                })
                .await?;
                envoyes += 1;
            }
            tx.commit().await.map_err(db)?;
        }
        Ok(envoyes)
    }

    /// Tâche de fond : chaque dimanche à partir de 17 h (jour et heure de l'établissement), prépare le résumé de la semaine
    /// qui commence le lendemain. Idempotente : rejouable à chaque passage sans doublon d'avis.
    pub async fn cdt_resumes_tache(&self) -> Result<usize, EntDbError> {
        let etabs = etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("cahier_resume_hebdomadaire"))
            .await
            .map_err(db)?;
        let mut n = 0;
        for etab in etabs {
            let (jour, heure): (NaiveDate, String) = match sqlx::query_as(
                "SELECT ent_aujourdhui_etab($1), to_char(ent_cdt_heure_locale($1, now()), 'HH24:MI')",
            )
            .bind(etab)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
            .await
            {
                Ok(x) => x,
                Err(e) => {
                    tracing::warn!(error = %e, "résumé hebdomadaire : jour de l'établissement illisible");
                    continue;
                }
            };
            if jour.weekday().num_days_from_monday() != 6 || heure.as_str() < "17:00" {
                continue;
            }
            match self.cdt_resumes_preparer(etab, jour + Duration::days(1)).await {
                Ok(k) => n += k,
                Err(e) => tracing::warn!(error = %e, "résumés hebdomadaires non préparés"),
            }
        }
        Ok(n)
    }
}

async fn resume_de(
    tx: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    lundi: NaiveDate,
    aujourdhui: NaiveDate,
    avec_coche: bool,
) -> Result<ResumeSemaineDto, EntDbError> {
    let dimanche = lundi + Duration::days(6);
    let lignes = lignes_agenda(tx, etab, eleve, lundi, dimanche, aujourdhui, avec_coche).await?;
    let compte = |g: &str| lignes.iter().filter(|l| l.genre == g).count() as i32;
    Ok(ResumeSemaineDto {
        eleve_id: eleve,
        semaine_debut: lundi,
        semaine_fin: dimanche,
        nb_travaux: compte(GENRE_TRAVAIL),
        nb_evaluations: compte(GENRE_EVALUATION),
        nb_autres: compte(GENRE_EPREUVE) + compte(GENRE_SORTIE),
        duree_estimee_min: lignes
            .iter()
            .filter(|l| l.genre == GENRE_TRAVAIL)
            .map(|l| i32::from(l.duree_estimee_min.unwrap_or(0)))
            .sum(),
        lignes,
    })
}

/// Empreinte SHA-256 (hexadécimale) du secret d'un jeton.
pub(crate) fn empreinte(secret: &str) -> String {
    let d = Sha256::digest(secret.as_bytes());
    d.iter().map(|o| format!("{o:02x}")).collect()
}

/// `{établissement en 32 hexadécimaux}-{secret en 64 hexadécimaux}[.ics]`.
pub(crate) fn decouper_jeton(jeton: &str) -> Option<(Uuid, &str)> {
    let jeton = jeton.strip_suffix(".ics").unwrap_or(jeton);
    let (etab, secret) = jeton.split_once('-')?;
    if etab.len() != 32 || secret.len() != 64 || !secret.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some((Uuid::parse_str(etab).ok()?, secret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lundi_de_chaque_jour_de_la_semaine() {
        let lundi = NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
        for k in 0..7 {
            assert_eq!(lundi_de(lundi + Duration::days(k)), lundi);
        }
        assert_eq!(lundi_de(lundi + Duration::days(7)), lundi + Duration::days(7));
    }

    #[test]
    fn jeton_bien_forme_seulement() {
        let etab = Uuid::new_v4();
        let secret = "a".repeat(64);
        let j = format!("{}-{}.ics", etab.simple(), secret);
        assert_eq!(decouper_jeton(&j), Some((etab, secret.as_str())));
        assert!(decouper_jeton(&format!("{}-{}", etab.simple(), "a".repeat(63))).is_none());
        assert!(decouper_jeton(&format!("{}-{}", etab.simple(), "g".repeat(64))).is_none());
        assert!(decouper_jeton("sans-tiret-valide").is_none());
        assert!(decouper_jeton("").is_none());
    }

    #[test]
    fn empreinte_stable_et_hexadecimale() {
        let e = empreinte("secret");
        assert_eq!(e.len(), 64);
        assert_eq!(e, empreinte("secret"));
        assert_ne!(e, empreinte("secret2"));
    }

    #[test]
    fn avis_minimaux_sans_contenu_ni_note_ni_motif() {
        assert_eq!(AVIS_CAHIER.len(), 4);
        let mut vus = HashSet::new();
        for a in AVIS_CAHIER {
            assert!(vus.insert(a.evenement), "un seul avis par événement : {}", a.evenement);
            assert!(a.evenement.starts_with("cahier."));
            assert_eq!(a.categorie, "Notes et devoirs");
            for interdit in ["{consigne}", "{note}", "{motif}", "{nom}", "{prenom}", "{titre}", "{contenu}", "{adresse}"] {
                assert!(!a.gabarit.contains(interdit), "gabarit « {} » : {interdit} interdit", a.gabarit);
            }
            assert!(a.gabarit.len() <= 120, "avis court");
            // Un responsable n'est jamais destinataire sans passer par l'habilitation du lien.
            assert!(!a.destinataires.contains("responsables,") && !a.destinataires.ends_with("responsables"));
        }
    }

    #[test]
    fn periode_bornee() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
        assert!(borner(d, d).is_ok());
        assert!(borner(d, d - Duration::days(1)).is_err());
        assert!(borner(d, d + Duration::days(101)).is_err());
    }
}
