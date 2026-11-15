//! Socle 10 (emploi du temps), lot T3 : lots de modifications (EDT-169 à 172) —
//! `docs/socles/10_EMPLOI_DU_TEMPS.md` §9.2.
//!
//! Un lot vise **plusieurs séances d'un coup** : un professeur sur une plage, une classe, un niveau, un site,
//! tout l'établissement (grève, intempéries), une salle (travaux), une liste de séances (le socle 15 y passe
//! pour les séances d'une absence). Le serveur calcule l'**aperçu** (séances, classes, élèves, intervenants,
//! notifications qui partiront) avant confirmation ; une clé d'idempotence fait que deux clics = un lot ; le
//! lot se **lève** en bloc. L'écriture est ensembliste (quelques requêtes, pas une par séance) : un lot de
//! 2 000 séances reste sous la seconde. Un seul événement `edt.lot_modifications` par lot : le socle 8 ne
//! prévient une personne qu'une fois, même si le lot touche plusieurs de ses séances.
//!
//! La **grève du jour** (EDT-170) annule les séances des professeurs déclarés absents ce matin, sans motif
//! nominatif public (le motif est « mouvement social », jamais le nom). La **salle fermée** (EDT-171) propose
//! une salle libre, compatible (même type, capacité suffisante, même site si possible) pour chaque séance, et
//! applique les propositions confirmées, chacune avec les contrôles du §9.3 ; ce qui n'a pas de solution est
//! rendu à la direction, jamais deviné.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::collections::BTreeSet;

use chrono::{NaiveDate, NaiveTime};
use ent_auth::Action;
use ent_events::AggregateKind;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::edt_modif::{
    lever_projection, poser_annulations, valider_motifs, ChangementSeance, ConflitModif, CtxModif,
    EVT_LOT_MODIFICATIONS,
};
use crate::ref_commun::{conflit, db, introuvable, invalide, publier_agg, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

pub const PORTEES_LOT: [&str; 8] = [
    "etablissement",
    "site",
    "niveau",
    "classes",
    "groupes",
    "intervenant",
    "lieu",
    "seances",
];
pub const NATURES_LOT: [&str; 4] = ["annulation", "greve", "neutralisation", "salle_fermee"];
/// Au-delà, le lot est refusé (une plage de deux mois suffit à une grève, à une fermeture).
pub const LOT_JOURS_MAX: i64 = 62;
pub const LOT_CIBLES_MAX: usize = 500;

#[derive(Debug, Clone, Deserialize)]
pub struct CmdLot {
    /// `annulation` (intempéries, sortie de niveau…), `greve`, `neutralisation` (événement qui remplace les
    /// cours : examen, sortie de tout le public, journée banalisée ciblée).
    pub nature: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub portee: String,
    /// Identifiants de la portée : sites, niveaux, classes, groupes, comptes d'intervenants, lieux ou séances.
    #[serde(default)]
    pub cibles: Vec<Uuid>,
    #[serde(default)]
    pub heure_debut: Option<NaiveTime>,
    #[serde(default)]
    pub heure_fin: Option<NaiveTime>,
    pub motif_public_code: String,
    #[serde(default)]
    pub motif_public: Option<String>,
    #[serde(default)]
    pub motif_interne: Option<String>,
    #[serde(default)]
    pub origine: Option<String>,
    #[serde(default)]
    pub origine_ref: Option<Uuid>,
    #[serde(default)]
    pub cle: Option<String>,
    #[serde(default)]
    pub apercu: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApercuLot {
    pub nb_seances: i64,
    pub nb_classes: i64,
    pub nb_eleves: i64,
    pub nb_intervenants: i64,
    /// Personnes qui recevront UN message (élèves, responsables au lien actif, intervenants).
    pub nb_notifications: i64,
    /// Séances de la plage déjà annulées, déplacées ou neutralisées (ignorées).
    pub deja_inactives: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultatLot {
    pub apercu: bool,
    pub deja_applique: bool,
    pub lot_id: Option<Uuid>,
    pub chiffres: ApercuLot,
}

/// Séances visées par un lot (identifiants, ordre stable pour les verrous).
async fn selectionner(
    conn: &mut PgConnection,
    etab: Uuid,
    cmd: &CmdLot,
    verrouiller: bool,
) -> Result<Vec<Uuid>, EntDbError> {
    let filtre = match cmd.portee.as_str() {
        "etablissement" => "",
        "site" => "AND s.classe_id IN (SELECT c.id FROM ent_classe c WHERE c.etablissement_id = s.etablissement_id AND c.site_id = ANY($4))",
        "niveau" => "AND s.classe_id IN (SELECT c.id FROM ent_classe c WHERE c.etablissement_id = s.etablissement_id AND c.niveau_id = ANY($4))",
        "classes" => "AND s.classe_id = ANY($4)",
        "groupes" => "AND s.cours_id IN (SELECT k.id FROM ent_cours k WHERE k.etablissement_id = s.etablissement_id AND k.groupe_id = ANY($4))",
        "intervenant" => "AND coalesce(s.remplacant_user_id, s.prof_user_id) = ANY($4)",
        "lieu" => "AND EXISTS (SELECT 1 FROM ent_edt_seance_lieu l WHERE l.seance_id = s.id AND l.etablissement_id = s.etablissement_id AND l.lieu_id = ANY($4))",
        "seances" => "AND s.id = ANY($4)",
        _ => return Err(invalide()),
    };
    let sql = format!(
        "SELECT s.id FROM ent_edt_seance s
          WHERE s.etablissement_id = $1 AND s.jour BETWEEN $2 AND $3
            AND s.etat IN ('prevue', 'modifiee', 'ajoutee')
            AND (s.fin_utc IS NULL OR s.fin_utc > now())
            AND ($5::time IS NULL OR (s.heure_debut < $6 AND $5 < s.heure_fin))
            {filtre}
          ORDER BY s.id {}",
        if verrouiller { "FOR UPDATE OF s" } else { "" }
    );
    sqlx::query_scalar(&sql)
        .bind(etab)
        .bind(cmd.du)
        .bind(cmd.au)
        .bind(&cmd.cibles)
        .bind(cmd.heure_debut)
        .bind(cmd.heure_fin)
        .fetch_all(conn)
        .await
        .map_err(db)
}

/// Les cibles existent toutes dans l'établissement : une cible d'un autre établissement n'existe pas ici (404),
/// jamais ignorée en silence.
async fn verifier_cibles(
    conn: &mut PgConnection,
    etab: Uuid,
    cmd: &CmdLot,
) -> Result<(), EntDbError> {
    let table = match cmd.portee.as_str() {
        "etablissement" => {
            return if cmd.cibles.is_empty() {
                Ok(())
            } else {
                Err(invalide())
            };
        }
        "site" => "SELECT id FROM ent_site WHERE etablissement_id = $1 AND id = ANY($2)",
        "niveau" => "SELECT id FROM ent_niveau WHERE etablissement_id = $1 AND id = ANY($2)",
        "classes" => "SELECT id FROM ent_classe WHERE etablissement_id = $1 AND id = ANY($2)",
        "groupes" => "SELECT id FROM ent_groupe WHERE etablissement_id = $1 AND id = ANY($2)",
        "lieu" => "SELECT id FROM ent_salle WHERE etablissement_id = $1 AND id = ANY($2)",
        "intervenant" => {
            "SELECT user_id FROM ent_membership WHERE etablissement_id = $1 AND user_id = ANY($2)"
        }
        "seances" => "SELECT id FROM ent_edt_seance WHERE etablissement_id = $1 AND id = ANY($2)",
        _ => return Err(invalide()),
    };
    let distinctes: BTreeSet<Uuid> = cmd.cibles.iter().copied().collect();
    if distinctes.is_empty() || distinctes.len() > LOT_CIBLES_MAX {
        return Err(invalide());
    }
    let trouvees: Vec<Uuid> = sqlx::query_scalar(table)
        .bind(etab)
        .bind(&cmd.cibles)
        .fetch_all(conn)
        .await
        .map_err(db)?;
    if trouvees.into_iter().collect::<BTreeSet<_>>().len() != distinctes.len() {
        return Err(introuvable());
    }
    Ok(())
}

fn valider_commande(cmd: &CmdLot) -> Result<(), EntDbError> {
    if !NATURES_LOT.contains(&cmd.nature.as_str())
        || !PORTEES_LOT.contains(&cmd.portee.as_str())
        || cmd.nature == "salle_fermee"
        || cmd.au < cmd.du
        || (cmd.au - cmd.du).num_days() > LOT_JOURS_MAX
        || matches!((cmd.heure_debut, cmd.heure_fin), (Some(d), Some(f)) if f <= d)
        || cmd.heure_debut.is_some() != cmd.heure_fin.is_some()
    {
        return Err(invalide());
    }
    Ok(())
}

async fn chiffrer(
    conn: &mut PgConnection,
    etab: Uuid,
    cmd: &CmdLot,
    ids: &[Uuid],
) -> Result<ApercuLot, EntDbError> {
    let (nb_classes, nb_intervenants): (i64, i64) = sqlx::query_as(
        "SELECT count(DISTINCT classe_id), count(DISTINCT coalesce(remplacant_user_id, prof_user_id))
           FROM ent_edt_seance WHERE etablissement_id = $1 AND id = ANY($2)",
    )
    .bind(etab)
    .bind(ids)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let nb_eleves: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT p.eleve_id) FROM (SELECT DISTINCT cours_id, jour FROM ent_edt_seance
                                                   WHERE etablissement_id = $1 AND id = ANY($2)
                                                     AND cours_id IS NOT NULL) x
           CROSS JOIN LATERAL ent_public_cours_jour($1, x.jour, x.cours_id) p",
    )
    .bind(etab)
    .bind(ids)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    // Personnes qui recevront un message : comptes d'élèves, responsables au lien actif, intervenants.
    let nb_notifications: i64 = sqlx::query_scalar(
        "WITH el AS (SELECT DISTINCT p.eleve_id FROM (SELECT DISTINCT cours_id, jour FROM ent_edt_seance
                                                       WHERE etablissement_id = $1 AND id = ANY($2)
                                                         AND cours_id IS NOT NULL) x
                      CROSS JOIN LATERAL ent_public_cours_jour($1, x.jour, x.cours_id) p),
              u AS (SELECT c.user_id FROM ent_eleve_compte c JOIN el ON el.eleve_id = c.eleve_id
                     WHERE c.etablissement_id = $1
                    UNION SELECT l.user_id FROM ent_eleve_lien_actif l JOIN el ON el.eleve_id = l.eleve_id
                     WHERE l.etablissement_id = $1 AND l.user_id IS NOT NULL
                    UNION SELECT coalesce(remplacant_user_id, prof_user_id) FROM ent_edt_seance
                     WHERE etablissement_id = $1 AND id = ANY($2))
         SELECT count(*) FROM u",
    )
    .bind(etab)
    .bind(ids)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let deja_inactives: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance s
          WHERE s.etablissement_id = $1 AND s.jour BETWEEN $2 AND $3
            AND s.etat IN ('annulee', 'deplacee', 'neutralisee')
            AND ($4::time IS NULL OR (s.heure_debut < $5 AND $4 < s.heure_fin))",
    )
    .bind(etab)
    .bind(cmd.du)
    .bind(cmd.au)
    .bind(cmd.heure_debut)
    .bind(cmd.heure_fin)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(ApercuLot {
        nb_seances: ids.len() as i64,
        nb_classes,
        nb_eleves,
        nb_intervenants,
        nb_notifications,
        deja_inactives,
    })
}

impl PgEnt {
    /// Aperçu serveur d'un lot, sans rien écrire (§9.2).
    pub async fn edt_lot_apercu(
        &self,
        a: &Acteur<'_>,
        cmd: &CmdLot,
    ) -> Result<ApercuLot, EntDbError> {
        self.exiger_edt(a, Action::EdtLotAppliquer).await?;
        valider_commande(cmd)?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        verifier_cibles(&mut tx, etab, cmd).await?;
        let ids = selectionner(&mut tx, etab, cmd, false).await?;
        chiffrer(&mut tx, etab, cmd, &ids).await
    }

    /// Applique un lot d'annulation, de grève ou de neutralisation (EDT-169, 170, 172) : un lot, une ligne du
    /// journal par séance, un événement. Deux envois de la même clé = un lot.
    pub async fn edt_lot_appliquer(
        &self,
        a: &Acteur<'_>,
        cmd: &CmdLot,
    ) -> Result<ResultatLot, EntDbError> {
        self.exiger_edt(a, Action::EdtLotAppliquer).await?;
        valider_commande(cmd)?;
        let etab = a.etablissement_id;
        let (motif_public, motif_interne) = valider_motifs(
            &cmd.motif_public_code,
            cmd.motif_public.as_deref(),
            cmd.motif_interne.as_deref(),
            cmd.cle.as_deref(),
            cmd.origine.as_deref(),
        )?;
        let origine = cmd.origine.clone().unwrap_or_else(|| "saisie".to_string());
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if cmd.apercu {
            verifier_cibles(&mut tx, etab, cmd).await?;
            let ids = selectionner(&mut tx, etab, cmd, false).await?;
            let chiffres = chiffrer(&mut tx, etab, cmd, &ids).await?;
            return Ok(ResultatLot {
                apercu: true,
                chiffres,
                ..Default::default()
            });
        }
        // Un lot à la fois par établissement : deux envois simultanés de la même clé se suivent.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 8))")
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if let Some(cle) = cmd.cle.as_deref() {
            let deja: Option<(Uuid, NaiveDate, NaiveDate, String, String, i32, i32, i32, i32)> = sqlx::query_as(
                "SELECT id, du, au, portee, nature, nb_seances, nb_classes, nb_eleves, nb_intervenants
                   FROM ent_edt_lot WHERE etablissement_id = $1 AND cle = $2",
            )
            .bind(etab)
            .bind(cle)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((id, du, au, portee, nature, ns, nc, ne, ni)) = deja {
                if du != cmd.du || au != cmd.au || portee != cmd.portee || nature != cmd.nature {
                    return Err(conflit()); // même clé, autre demande
                }
                return Ok(ResultatLot {
                    deja_applique: true,
                    lot_id: Some(id),
                    chiffres: ApercuLot {
                        nb_seances: i64::from(ns),
                        nb_classes: i64::from(nc),
                        nb_eleves: i64::from(ne),
                        nb_intervenants: i64::from(ni),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            }
        }
        verifier_cibles(&mut tx, etab, cmd).await?;
        let ids = selectionner(&mut tx, etab, cmd, true).await?;
        if ids.is_empty() {
            return Err(conflit()); // pas de lot vide
        }
        let chiffres = chiffrer(&mut tx, etab, cmd, &ids).await?;
        let lot_id = Uuid::new_v4();
        let type_code = if cmd.nature == "neutralisation" {
            "neutralisation"
        } else {
            "annulation"
        };
        let apres = if cmd.nature == "neutralisation" {
            "neutralisee"
        } else {
            "annulee"
        };
        sqlx::query(
            "INSERT INTO ent_edt_lot
               (id, etablissement_id, cle, nature, du, au, portee, heure_debut, heure_fin, motif_public_code,
                motif_public, origine, origine_ref, nb_seances, nb_classes, nb_eleves, nb_intervenants, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
        )
        .bind(lot_id)
        .bind(etab)
        .bind(cmd.cle.as_deref())
        .bind(&cmd.nature)
        .bind(cmd.du)
        .bind(cmd.au)
        .bind(&cmd.portee)
        .bind(cmd.heure_debut)
        .bind(cmd.heure_fin)
        .bind(&cmd.motif_public_code)
        .bind(&motif_public)
        .bind(match origine.as_str() {
            "absence" | "fermeture" | "sortie" | "examen" | "calendrier" => origine.as_str(),
            _ => "saisie",
        })
        .bind(cmd.origine_ref)
        .bind(chiffres.nb_seances as i32)
        .bind(chiffres.nb_classes as i32)
        .bind(chiffres.nb_eleves as i32)
        .bind(chiffres.nb_intervenants as i32)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if cmd.portee != "etablissement" {
            sqlx::query(
                "INSERT INTO ent_edt_lot_cible (lot_id, etablissement_id, cible_id)
                 SELECT $1, $2, c FROM unnest($3::uuid[]) AS t(c) ON CONFLICT DO NOTHING",
            )
            .bind(lot_id)
            .bind(etab)
            .bind(&cmd.cibles)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        // Les décisions de lieu ou d'intervenant de ces séances sont remplacées par le lot (levées, gardées).
        sqlx::query(
            "UPDATE ent_edt_modification SET levee_le = now(), levee_par = $3
              WHERE etablissement_id = $1 AND seance_id = ANY($2) AND levee_le IS NULL
                AND type IN ('changement_lieu', 'intervenant')",
        )
        .bind(etab)
        .bind(&ids)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Le journal d'abord (le déclencheur d'état en dérive `neutralisee`), puis les séances.
        let origine_modif = if origine == "saisie" {
            "lot"
        } else {
            origine.as_str()
        };
        sqlx::query(
            "INSERT INTO ent_edt_modification
               (id, etablissement_id, seance_id, lot_id, creneau_id, jour, type, avant_etat, apres_etat, avant_debut,
                avant_fin, avant_lieu_id, motif_public_code, motif_public, origine, origine_ref, cree_par)
             SELECT gen_random_uuid(), s.etablissement_id, s.id, $3, s.creneau_id, s.jour, $4, s.etat, $5,
                    s.heure_debut, s.heure_fin,
                    (SELECT l.lieu_id FROM ent_edt_seance_lieu l WHERE l.seance_id = s.id
                      AND l.etablissement_id = s.etablissement_id ORDER BY l.principal DESC, l.lieu_id LIMIT 1),
                    $6, $7, $8, $9, $10
               FROM ent_edt_seance s WHERE s.etablissement_id = $1 AND s.id = ANY($2)",
        )
        .bind(etab)
        .bind(&ids)
        .bind(lot_id)
        .bind(type_code)
        .bind(apres)
        .bind(&cmd.motif_public_code)
        .bind(&motif_public)
        .bind(origine_modif)
        .bind(cmd.origine_ref)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(interne) = &motif_interne {
            sqlx::query(
                "INSERT INTO ent_edt_modification_interne (modification_id, etablissement_id, motif_interne, cree_par)
                 SELECT m.id, m.etablissement_id, $3, $4 FROM ent_edt_modification m
                  WHERE m.etablissement_id = $1 AND m.lot_id = $2",
            )
            .bind(etab)
            .bind(lot_id)
            .bind(interne)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let absence = (origine == "absence").then_some(cmd.origine_ref).flatten();
        poser_annulations(
            &mut tx,
            etab,
            &ids,
            a.user_id,
            absence,
            &cmd.motif_public_code,
            motif_public.as_deref(),
            "annulation",
        )
        .await?;
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            EVT_LOT_MODIFICATIONS,
            lot_id,
            json!({ "lot_id": lot_id, "action": "pose", "nature": cmd.nature, "du": cmd.du, "au": cmd.au,
                    "nb_seances": chiffres.nb_seances, "motif_public_code": cmd.motif_public_code,
                    "motif_public": motif_public, "origine": origine }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ResultatLot {
            apercu: false,
            deja_applique: false,
            lot_id: Some(lot_id),
            chiffres,
        })
    }

    /// Lève un lot en bloc (EDT-168 pour un lot) : toutes ses modifications vivantes dont la séance n'est pas
    /// passée sont levées ; la levée d'un lot déjà annoncé envoie UN message de rétablissement par personne.
    pub async fn edt_lot_lever(
        &self,
        a: &Acteur<'_>,
        lot_id: Uuid,
    ) -> Result<ResultatLeveeLot, EntDbError> {
        self.exiger_edt(a, Action::EdtLotAppliquer).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 8))")
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let lot: Option<(bool, String, NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT (leve_le IS NOT NULL), nature, du, au FROM ent_edt_lot
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(lot_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((leve, nature, du, au)) = lot else {
            return Err(introuvable());
        };
        if leve {
            return Ok(ResultatLeveeLot {
                deja_fait: true,
                nb_levees: 0,
                nb_passees_gardees: 0,
            });
        }
        // Séances encore à venir du lot (les passées gardent leur état : le passé ne se lève pas, §9.6).
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT m.seance_id FROM ent_edt_modification m
               JOIN ent_edt_seance s ON s.id = m.seance_id AND s.etablissement_id = m.etablissement_id
              WHERE m.etablissement_id = $1 AND m.lot_id = $2 AND m.levee_le IS NULL AND m.seance_id IS NOT NULL
                AND (s.fin_utc IS NULL OR s.fin_utc > now())
              ORDER BY m.seance_id FOR UPDATE OF s",
        )
        .bind(etab)
        .bind(lot_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let passees: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_edt_modification m
              WHERE m.etablissement_id = $1 AND m.lot_id = $2 AND m.levee_le IS NULL
                AND NOT (m.seance_id = ANY($3))",
        )
        .bind(etab)
        .bind(lot_id)
        .bind(&ids)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_edt_modification SET levee_le = now(), levee_par = $4
              WHERE etablissement_id = $1 AND lot_id = $2 AND levee_le IS NULL AND seance_id = ANY($3)",
        )
        .bind(etab)
        .bind(lot_id)
        .bind(&ids)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        lever_projection(&mut tx, etab, &ids, a.user_id).await?;
        // Un lot de lieu se lève de même : la salle revient au plan (le plan est la référence, pas un état
        // intermédiaire). Le lot est levé quand plus rien de vivant ne le porte.
        let reste: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_edt_modification WHERE etablissement_id = $1 AND lot_id = $2 AND levee_le IS NULL",
        )
        .bind(etab)
        .bind(lot_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if reste == 0 {
            sqlx::query("UPDATE ent_edt_lot SET leve_le = now(), leve_par = $3 WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(lot_id)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            EVT_LOT_MODIFICATIONS,
            lot_id,
            json!({ "lot_id": lot_id, "action": "levee", "nature": nature, "du": du, "au": au,
                    "nb_seances": ids.len() }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ResultatLeveeLot {
            deja_fait: false,
            nb_levees: ids.len() as i64,
            nb_passees_gardees: passees,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultatLeveeLot {
    pub deja_fait: bool,
    pub nb_levees: i64,
    pub nb_passees_gardees: i64,
}

// ───────────────────────────── Grève du jour (EDT-170) ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DemiJournee {
    Matin,
    ApresMidi,
    Journee,
}

impl DemiJournee {
    fn heures(self) -> Option<(NaiveTime, NaiveTime)> {
        let h = |a, m, s| NaiveTime::from_hms_opt(a, m, s).expect("heure");
        match self {
            Self::Matin => Some((h(0, 0, 0), h(12, 30, 0))),
            Self::ApresMidi => Some((h(12, 30, 0), h(23, 59, 59))),
            Self::Journee => None,
        }
    }
}

/// Un professeur de la liste à cocher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct CandidatGreve {
    pub user_id: Uuid,
    pub nb_seances: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CmdGreve {
    pub jour: NaiveDate,
    pub demi_journee: DemiJournee,
    /// Comptes des professeurs déclarés absents (cochés).
    pub intervenants: Vec<Uuid>,
    #[serde(default)]
    pub motif_interne: Option<String>,
    #[serde(default)]
    pub cle: Option<String>,
    #[serde(default)]
    pub apercu: bool,
}

impl PgEnt {
    /// Liste à cocher de la grève du jour : les intervenants qui ont encore au moins une séance à venir.
    pub async fn edt_greve_candidats(
        &self,
        a: &Acteur<'_>,
        jour: NaiveDate,
        demi_journee: DemiJournee,
    ) -> Result<Vec<CandidatGreve>, EntDbError> {
        self.exiger_edt(a, Action::EdtLotAppliquer).await?;
        let etab = a.etablissement_id;
        let h = demi_journee.heures();
        sqlx::query_as::<_, CandidatGreve>(
            "SELECT coalesce(s.remplacant_user_id, s.prof_user_id) AS user_id, count(*) AS nb_seances
               FROM ent_edt_seance s
              WHERE s.etablissement_id = $1 AND s.jour = $2 AND s.etat IN ('prevue', 'modifiee', 'ajoutee')
                AND (s.fin_utc IS NULL OR s.fin_utc > now())
                AND ($3::time IS NULL OR (s.heure_debut < $4 AND $3 < s.heure_fin))
              GROUP BY 1 ORDER BY 2 DESC, 1",
        )
        .bind(etab)
        .bind(jour)
        .bind(h.map(|x| x.0))
        .bind(h.map(|x| x.1))
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)
    }

    /// Annule les séances des professeurs déclarés absents ce jour ou cette demi-journée. Le motif public est
    /// « mouvement social » : jamais le nom d'un professeur gréviste (le motif interne, s'il y en a, reste
    /// réservé à la direction).
    pub async fn edt_greve_du_jour(
        &self,
        a: &Acteur<'_>,
        cmd: &CmdGreve,
    ) -> Result<ResultatLot, EntDbError> {
        if cmd.intervenants.is_empty() {
            return Err(invalide());
        }
        let h = cmd.demi_journee.heures();
        self.edt_lot_appliquer(
            a,
            &CmdLot {
                nature: "greve".to_string(),
                du: cmd.jour,
                au: cmd.jour,
                portee: "intervenant".to_string(),
                cibles: cmd.intervenants.clone(),
                heure_debut: h.map(|x| x.0),
                heure_fin: h.map(|x| x.1),
                motif_public_code: "mouvement_social".to_string(),
                motif_public: None,
                motif_interne: cmd.motif_interne.clone(),
                origine: None,
                origine_ref: None,
                cle: cmd.cle.clone(),
                apercu: cmd.apercu,
            },
        )
        .await
    }

    /// Port pour le socle 15 : annule une liste de séances (celles d'une absence décidée) en un lot, avec la
    /// clé d'idempotence de la décision et l'absence citée.
    pub async fn edt_annuler_seances(
        &self,
        a: &Acteur<'_>,
        seances: &[Uuid],
        absence_rh_id: Option<Uuid>,
        cle: Option<String>,
    ) -> Result<ResultatLot, EntDbError> {
        if seances.is_empty() || seances.len() > LOT_CIBLES_MAX {
            return Err(invalide());
        }
        let etab = a.etablissement_id;
        let (du, au): (Option<NaiveDate>, Option<NaiveDate>) = sqlx::query_as(
            "SELECT min(jour), max(jour) FROM ent_edt_seance WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etab)
        .bind(seances)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let (Some(du), Some(au)) = (du, au) else {
            return Err(introuvable());
        };
        self.edt_lot_appliquer(
            a,
            &CmdLot {
                nature: "annulation".to_string(),
                du,
                au,
                portee: "seances".to_string(),
                cibles: seances.to_vec(),
                heure_debut: None,
                heure_fin: None,
                motif_public_code: "absence_professeur".to_string(),
                motif_public: None,
                motif_interne: None,
                origine: absence_rh_id.map(|_| "absence".to_string()),
                origine_ref: absence_rh_id,
                cle,
                apercu: false,
            },
        )
        .await
    }
}

// ───────────────────────────── Salle fermée en lot (EDT-171) ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionLieu {
    pub seance_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub effectif: i64,
    /// Salle libre proposée ; `None` : aucune solution automatique (la direction décide).
    pub lieu_propose: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CmdSalleFermee {
    pub salle_id: Uuid,
    pub du: NaiveDate,
    pub au: NaiveDate,
    /// Propositions confirmées : (séance, lieu). Une séance absente de la liste n'est pas changée.
    pub propositions: Vec<(Uuid, Uuid)>,
    #[serde(default = "motif_travaux")]
    pub motif_public_code: String,
    #[serde(default)]
    pub motif_public: Option<String>,
    #[serde(default)]
    pub motif_interne: Option<String>,
    #[serde(default)]
    pub cle: Option<String>,
}

fn motif_travaux() -> String {
    "travaux".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultatSalleFermee {
    pub lot_id: Option<Uuid>,
    pub deja_applique: bool,
    pub appliquees: Vec<Uuid>,
    /// Séances dont la proposition n'a pas passé les contrôles du §9.3 au moment de l'application.
    pub refusees: Vec<(Uuid, Vec<ConflitModif>)>,
}

impl PgEnt {
    /// Propose, pour chaque séance à venir de la salle sur la plage, une salle libre et compatible.
    pub async fn edt_salle_fermee_propositions(
        &self,
        a: &Acteur<'_>,
        salle_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<PropositionLieu>, EntDbError> {
        self.exiger_edt(a, Action::EdtLotAppliquer).await?;
        if au < du || (au - du).num_days() > LOT_JOURS_MAX {
            return Err(invalide());
        }
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (type_salle, site): (String, Option<Uuid>) = sqlx::query_as(
            "SELECT type, site_id FROM ent_salle WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(salle_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        #[derive(FromRow)]
        struct S {
            id: Uuid,
            jour: NaiveDate,
            heure_debut: NaiveTime,
            heure_fin: NaiveTime,
            effectif: i64,
        }
        let seances: Vec<S> = sqlx::query_as(
            "SELECT s.id, s.jour, s.heure_debut, s.heure_fin,
                    coalesce((SELECT count(DISTINCT p.eleve_id) FROM ent_public_cours_jour(s.etablissement_id, s.jour, s.cours_id) p), 0) AS effectif
               FROM ent_edt_seance s
               JOIN ent_edt_seance_lieu l ON l.seance_id = s.id AND l.etablissement_id = s.etablissement_id
              WHERE s.etablissement_id = $1 AND l.lieu_id = $2 AND s.jour BETWEEN $3 AND $4
                AND s.etat IN ('prevue', 'modifiee', 'ajoutee') AND (s.fin_utc IS NULL OR s.fin_utc > now())
              ORDER BY s.jour, s.heure_debut, s.id",
        )
        .bind(etab)
        .bind(salle_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut prises: Vec<(Uuid, NaiveDate, NaiveTime, NaiveTime)> = Vec::new();
        let mut out = Vec::new();
        for s in seances {
            let candidats: Vec<Uuid> = sqlx::query_scalar(
                "WITH fz AS (SELECT coalesce((SELECT fuseau FROM ent_etablissement_parametre
                                               WHERE etablissement_id = $1), 'Europe/Paris') AS z),
                      t AS (SELECT (($4::date + $5::time) AT TIME ZONE fz.z) AS d,
                                   (($4::date + $6::time) AT TIME ZONE fz.z) AS f FROM fz)
                 SELECT sa.id FROM ent_salle sa
                  WHERE sa.etablissement_id = $1 AND sa.actif AND sa.id <> $2 AND sa.type = $3
                    AND (sa.capacite IS NULL OR sa.capacite >= $7)
                    AND NOT EXISTS (SELECT 1 FROM ent_edt_seance_lieu l
                                      JOIN ent_edt_seance o ON o.id = l.seance_id AND o.etablissement_id = l.etablissement_id
                                     WHERE l.etablissement_id = $1 AND l.lieu_id = sa.id AND o.jour = $4
                                       AND o.etat IN ('prevue', 'modifiee', 'ajoutee')
                                       AND o.heure_debut < $6 AND $5 < o.heure_fin)
                    AND NOT EXISTS (SELECT 1 FROM ent_agenda_reservation r, t
                                     WHERE r.etablissement_id = $1 AND r.salle_id = sa.id AND r.statut = 'confirmee'
                                       AND r.debut < t.f AND t.d < r.fin)
                    AND NOT EXISTS (SELECT 1 FROM ent_salle_indisponibilite i, t
                                     WHERE i.etablissement_id = $1 AND i.salle_id = sa.id AND i.annulee_le IS NULL
                                       AND i.du < t.f AND t.d < i.au)
                  ORDER BY (sa.site_id IS NOT DISTINCT FROM $8) DESC, sa.capacite NULLS LAST, sa.code, sa.id
                  LIMIT 20",
            )
            .bind(etab)
            .bind(salle_id)
            .bind(&type_salle)
            .bind(s.jour)
            .bind(s.heure_debut)
            .bind(s.heure_fin)
            .bind(s.effectif as i32)
            .bind(site)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            // Une salle déjà proposée à ce créneau n'est pas proposée deux fois.
            let choix = candidats.into_iter().find(|c| {
                !prises.iter().any(|(l, j, d, f)| {
                    l == c && *j == s.jour && *d < s.heure_fin && s.heure_debut < *f
                })
            });
            if let Some(c) = choix {
                prises.push((c, s.jour, s.heure_debut, s.heure_fin));
            }
            out.push(PropositionLieu {
                seance_id: s.id,
                jour: s.jour,
                heure_debut: s.heure_debut,
                heure_fin: s.heure_fin,
                effectif: s.effectif,
                lieu_propose: choix,
            });
        }
        Ok(out)
    }

    /// Applique les propositions confirmées d'une salle fermée : un lot, un changement de lieu par séance,
    /// chacun avec les contrôles du §9.3 (une proposition qui n'est plus libre est refusée, jamais forcée).
    pub async fn edt_salle_fermee_appliquer(
        &self,
        a: &Acteur<'_>,
        cmd: &CmdSalleFermee,
    ) -> Result<ResultatSalleFermee, EntDbError> {
        self.exiger_edt(a, Action::EdtLotAppliquer).await?;
        self.exiger_edt(a, Action::EdtModifierLieu).await?;
        let etab = a.etablissement_id;
        if cmd.au < cmd.du
            || (cmd.au - cmd.du).num_days() > LOT_JOURS_MAX
            || cmd.propositions.is_empty()
            || cmd.propositions.len() > 2000
        {
            return Err(invalide());
        }
        let (motif_public, motif_interne) = valider_motifs(
            &cmd.motif_public_code,
            cmd.motif_public.as_deref(),
            cmd.motif_interne.as_deref(),
            cmd.cle.as_deref(),
            None,
        )?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 8))")
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if let Some(cle) = cmd.cle.as_deref() {
            let deja: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM ent_edt_lot WHERE etablissement_id = $1 AND cle = $2 AND nature = 'salle_fermee'",
            )
            .bind(etab)
            .bind(cle)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some(id) = deja {
                return Ok(ResultatSalleFermee {
                    lot_id: Some(id),
                    deja_applique: true,
                    ..Default::default()
                });
            }
        }
        let existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_salle WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(cmd.salle_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let lot_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_edt_lot
               (id, etablissement_id, cle, nature, du, au, portee, motif_public_code, motif_public, origine,
                nb_seances, cree_par)
             VALUES ($1,$2,$3,'salle_fermee',$4,$5,'lieu',$6,$7,'fermeture',$8,$9)",
        )
        .bind(lot_id)
        .bind(etab)
        .bind(cmd.cle.as_deref())
        .bind(cmd.du)
        .bind(cmd.au)
        .bind(&cmd.motif_public_code)
        .bind(&motif_public)
        .bind(cmd.propositions.len() as i32)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_edt_lot_cible (lot_id, etablissement_id, cible_id) VALUES ($1,$2,$3)",
        )
        .bind(lot_id)
        .bind(etab)
        .bind(cmd.salle_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ctx = CtxModif {
            a: *a,
            motif_code: cmd.motif_public_code.clone(),
            motif_public,
            motif_interne,
            origine: "fermeture".to_string(),
            origine_ref: Some(cmd.salle_id),
            cle: None,
            forcer: None,
            apercu: false,
            lot_id: Some(lot_id),
            evenements: std::sync::Mutex::new(Vec::new()),
        };
        let mut res = ResultatSalleFermee {
            lot_id: Some(lot_id),
            ..Default::default()
        };
        for (seance_id, lieu_id) in &cmd.propositions {
            // Chaque séance dans son propre point de sauvegarde : un refus n'annule pas les autres.
            sqlx::query("SAVEPOINT sf")
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            let r = self
                .appliquer_sur_seance(
                    &mut tx,
                    &ctx,
                    *seance_id,
                    &ChangementSeance::ChangementLieu { lieu_id: *lieu_id },
                )
                .await;
            match r {
                Ok(r) if r.applique => {
                    sqlx::query("RELEASE SAVEPOINT sf")
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    res.appliquees.push(*seance_id);
                }
                Ok(r) => {
                    sqlx::query("ROLLBACK TO SAVEPOINT sf")
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    res.refusees.push((*seance_id, r.conflits));
                }
                Err(EntDbError::Domain(_)) => {
                    sqlx::query("ROLLBACK TO SAVEPOINT sf")
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    res.refusees.push((
                        *seance_id,
                        vec![ConflitModif::dur("seance_introuvable_ou_inactive", 1, None)],
                    ));
                }
                Err(e) => return Err(e),
            }
        }
        sqlx::query(
            "UPDATE ent_edt_lot SET nb_seances = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(lot_id)
        .bind(res.appliquees.len() as i32)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if res.appliquees.is_empty() {
            return Err(conflit()); // aucun changement possible : pas de lot vide
        }
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            EVT_LOT_MODIFICATIONS,
            lot_id,
            json!({ "lot_id": lot_id, "action": "pose", "nature": "salle_fermee", "du": cmd.du, "au": cmd.au,
                    "nb_seances": res.appliquees.len(), "motif_public_code": cmd.motif_public_code,
                    "motif_public": ctx.motif_public, "origine": "fermeture" }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(res)
    }
}
