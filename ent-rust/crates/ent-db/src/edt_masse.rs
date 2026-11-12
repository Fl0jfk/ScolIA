//! S2-5 — annulation en masse (EDT-08 ★) et changements visibles des élèves et familles.
//!
//! Une grève, une fermeture ou une sortie de niveau annule d'un coup toutes les séances
//! concernées de la plage (vacances et jours fermés exclus d'office, semaines A/B respectées).
//! La direction **prévisualise** d'abord les classes touchées ; l'application est un seul lot
//! (`ent_edt_annulation_masse`) et un seul événement `edt.annulation_masse` ; une clé
//! d'idempotence fournie par le client empêche un double envoi de créer deux lots.

use std::collections::{BTreeMap, HashSet};

use chrono::{Duration, NaiveDate};
use ent_auth::Action;
use ent_events::AggregateKind;
use serde_json::json;
use uuid::Uuid;

use crate::edt_admin::PlanEdt;
use crate::edt_remplacement::{ChangementEdt, ClasseTouchee, CmdAnnulationMasse, RapportMasse};
use crate::ref_commun::{Acteur, conflit, db, introuvable, invalide, publier_agg};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

impl PgEnt {
    /// Séances que le lot annulerait, avec le nombre de séances déjà annulées ou déplacées.
    async fn masse_selection(
        &self,
        etab: Uuid,
        cmd: &CmdAnnulationMasse,
    ) -> Result<(Vec<PlanEdt>, usize), EntDbError> {
        if cmd.au < cmd.du
            || (cmd.au - cmd.du).num_days() > 62
            || cmd.motif_public.trim().is_empty()
            || cmd.motif_public.chars().count() > 200
            || cmd
                .cle
                .as_deref()
                .is_some_and(|c| c.is_empty() || c.len() > 80)
            || matches!((cmd.heure_debut, cmd.heure_fin), (Some(d), Some(f)) if f <= d)
        {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Classes visées.
        let classes: Option<HashSet<Uuid>> = match cmd.portee.as_str() {
            "etablissement" => {
                if !cmd.cibles.is_empty() {
                    return Err(invalide());
                }
                None
            }
            "classes" | "niveau" => {
                if cmd.cibles.is_empty() || cmd.cibles.len() > 200 {
                    return Err(invalide());
                }
                let col = if cmd.portee == "niveau" {
                    "niveau_id"
                } else {
                    "id"
                };
                let sql = format!(
                    "SELECT id FROM ent_classe WHERE etablissement_id = $1 AND {col} = ANY($2)"
                );
                let ids: Vec<Uuid> = sqlx::query_scalar(&sql)
                    .bind(etab)
                    .bind(&cmd.cibles)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(db)?;
                // Une cible d'un autre établissement n'existe pas ici : 404, jamais ignorée.
                if cmd.portee == "classes"
                    && ids.len() != cmd.cibles.iter().collect::<HashSet<_>>().len()
                {
                    return Err(introuvable());
                }
                Some(ids.into_iter().collect())
            }
            _ => return Err(invalide()),
        };
        let plans = self.edt_plans(etab, cmd.du, cmd.au).await?;
        let deplaces: HashSet<(Uuid, NaiveDate)> = sqlx::query_as::<_, (Uuid, NaiveDate)>(
            "SELECT creneau_id, jour_origine FROM ent_edt_deplacement
              WHERE etablissement_id = $1 AND jour_origine BETWEEN $2 AND $3",
        )
        .bind(etab)
        .bind(cmd.du)
        .bind(cmd.au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let mut deja = 0;
        let mut out = Vec::new();
        for p in plans {
            let pl = &p.plan;
            if classes.as_ref().is_some_and(|c| !c.contains(&pl.classe_id)) {
                continue;
            }
            if let (Some(d), Some(f)) = (cmd.heure_debut, cmd.heure_fin) {
                if !(pl.heure_debut < f && d < pl.heure_fin) {
                    continue;
                }
            }
            if pl.statut == "annulee" || deplaces.contains(&(pl.creneau_id, pl.jour)) {
                deja += 1;
                continue;
            }
            out.push(p);
        }
        Ok((out, deja))
    }

    fn rapport_masse(
        selection: &[PlanEdt],
        deja: usize,
        apercu: bool,
        lot: Option<Uuid>,
        deja_applique: bool,
    ) -> RapportMasse {
        let mut par_classe: BTreeMap<(String, Uuid), usize> = BTreeMap::new();
        for p in selection {
            *par_classe
                .entry((p.plan.classe.clone(), p.plan.classe_id))
                .or_default() += 1;
        }
        RapportMasse {
            apercu,
            deja_applique,
            lot_id: lot,
            nb_seances: selection.len(),
            nb_classes: par_classe.len(),
            classes: par_classe
                .into_iter()
                .map(|((classe, classe_id), nb)| ClasseTouchee {
                    classe_id,
                    classe,
                    nb_seances: nb,
                })
                .collect(),
            deja_annulees: deja,
        }
    }

    /// EDT-08 ★ : aperçu puis application d'une annulation en masse (`edt.annuler_masse`).
    pub async fn edt_annulation_masse(
        &self,
        a: &Acteur<'_>,
        cmd: &CmdAnnulationMasse,
    ) -> Result<RapportMasse, EntDbError> {
        self.exiger_edt(a, Action::EdtAnnulerMasse).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        // Même clé déjà appliquée : rien de nouveau (double envoi, rejeu réseau).
        if let Some(cle) = cmd.cle.as_deref().filter(|_| !cmd.apercu) {
            let existant: Option<(Uuid, NaiveDate, NaiveDate, String, String, i32, i32)> =
                sqlx::query_as(
                    "SELECT id, du, au, portee, motif_public, nb_seances, nb_classes
                   FROM ent_edt_annulation_masse WHERE etablissement_id = $1 AND cle = $2",
                )
                .bind(etab)
                .bind(cle)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
            if let Some((id, du, au, portee, motif, nb, nbc)) = existant {
                if du != cmd.du
                    || au != cmd.au
                    || portee != cmd.portee
                    || motif != cmd.motif_public.trim()
                {
                    return Err(conflit()); // même clé, autre demande
                }
                return Ok(RapportMasse {
                    apercu: false,
                    deja_applique: true,
                    lot_id: Some(id),
                    nb_seances: nb as usize,
                    nb_classes: nbc as usize,
                    classes: Vec::new(),
                    deja_annulees: 0,
                });
            }
        }
        let (selection, deja) = self.masse_selection(etab, cmd).await?;
        if cmd.apercu {
            return Ok(Self::rapport_masse(&selection, deja, true, None, false));
        }
        if selection.is_empty() {
            return Err(conflit()); // rien à annuler : pas de lot vide
        }
        let rapport = Self::rapport_masse(&selection, deja, false, None, false);
        let lot_id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 7))")
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let insere = sqlx::query(
            "INSERT INTO ent_edt_annulation_masse
               (id, etablissement_id, cle, du, au, portee, cibles, heure_debut, heure_fin,
                motif_public, nb_seances, nb_classes, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(lot_id)
        .bind(etab)
        .bind(cmd.cle.as_deref())
        .bind(cmd.du)
        .bind(cmd.au)
        .bind(&cmd.portee)
        .bind(&cmd.cibles)
        .bind(cmd.heure_debut)
        .bind(cmd.heure_fin)
        .bind(cmd.motif_public.trim())
        .bind(rapport.nb_seances as i32)
        .bind(rapport.nb_classes as i32)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await;
        match insere {
            Ok(_) => {}
            // Deux envois simultanés de la même clé : le second voit la contrainte d'unicité.
            Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => {
                return Err(conflit());
            }
            Err(e) => return Err(db(e)),
        }
        for p in &selection {
            let pl = &p.plan;
            let exc = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO ent_edt_exception
                   (id, etablissement_id, creneau_id, jour, kind, motif, created_by_user_id,
                    created_at, lot_id, nature)
                 VALUES ($1,$2,$3,$4,'annulation','',$5,now(),$6,'annulation')
                 ON CONFLICT (etablissement_id, creneau_id, jour) DO UPDATE SET
                    kind = 'annulation', remplacant_user_id = NULL, nouvelle_salle = NULL,
                    created_by_user_id = EXCLUDED.created_by_user_id, created_at = now(),
                    lot_id = EXCLUDED.lot_id, nature = 'annulation'
                 RETURNING id",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(pl.creneau_id)
            .bind(pl.jour)
            .bind(a.user_id)
            .bind(lot_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            // Les délégations de remplacement de ces séances n'ont plus d'objet.
            sqlx::query(
                "UPDATE ent_delegation SET statut = 'revoquee', revoquee_le = now(), revoquee_par_user_id = $3
                  WHERE etablissement_id = $1 AND reference_id = $2 AND source = 'remplacement'
                    AND statut = 'accordee'",
            )
            .bind(etab)
            .bind(exc)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE ent_edt_seance SET statut = 'annulee', remplacant_user_id = NULL, updated_at = now()
                  WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3 AND source = 'genere'
                AND statut <> 'retiree'",
            )
            .bind(etab)
            .bind(pl.creneau_id)
            .bind(pl.jour)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            "edt.annulation_masse",
            etab,
            json!({ "lot_id": lot_id, "du": cmd.du, "au": cmd.au,
                    "nb_seances": rapport.nb_seances, "nb_classes": rapport.nb_classes }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(RapportMasse {
            lot_id: Some(lot_id),
            ..rapport
        })
    }

    /// Changements (annulations, remplacements, salles, déplacements) visibles d'un élève
    /// entre deux dates : élève lui-même, responsable au lien actif, ou personnel.
    pub async fn edt_changements(
        &self,
        a: &Acteur<'_>,
        eleve_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<ChangementEdt>, EntDbError> {
        self.edt_acces_eleve(a, eleve_id).await?;
        let etab = a.etablissement_id;
        if au < du || (au - du).num_days() > 92 {
            return Err(invalide());
        }
        let (classes, groupes) = self.edt_cibles_eleve(etab, eleve_id, du).await?;
        let classes: Vec<Uuid> = classes.into_iter().collect();
        let groupes: Vec<Uuid> = groupes.into_iter().collect();
        if classes.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<ChangementEdt> = sqlx::query_as::<_, LigneChangement>(
            "SELECT x.creneau_id, x.jour, coalesce(x.nature, x.kind) AS nature, c.matiere_libelle,
                    btrim(c.classe) AS classe, c.heure_debut, c.heure_fin,
                    x.nouvelle_salle,
                    CASE WHEN x.kind = 'remplacement'
                         THEN coalesce(nullif(btrim(u.display_name), ''), u.email) END AS remplacant_nom,
                    d.nouveau_jour, d.nouvelle_heure_debut, d.nouvelle_heure_fin,
                    l.motif_public, x.created_at AS enregistre_le
               FROM ent_edt_exception x
               JOIN ent_edt_creneau c ON c.id = x.creneau_id AND c.etablissement_id = x.etablissement_id
               JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.code = btrim(c.classe)
                                 AND cl.id = ANY($4)
               LEFT JOIN ent_user u ON u.id = x.remplacant_user_id
               LEFT JOIN ent_edt_deplacement d ON d.etablissement_id = x.etablissement_id
                      AND d.creneau_id = x.creneau_id AND d.jour_origine = x.jour
               LEFT JOIN ent_edt_annulation_masse l ON l.id = x.lot_id AND l.etablissement_id = x.etablissement_id
              WHERE x.etablissement_id = $1 AND x.jour BETWEEN $2 AND $3
                AND (c.groupe_id IS NULL OR c.groupe_id = ANY($5))
              ORDER BY x.jour, c.heure_debut",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .bind(&classes)
        .bind(&groupes)
        .fetch_all(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?
        .into_iter()
        .map(LigneChangement::vers)
        .collect();
        let _ = Duration::days(0);
        Ok(rows)
    }
}

#[derive(sqlx::FromRow)]
struct LigneChangement {
    creneau_id: Uuid,
    jour: NaiveDate,
    nature: String,
    matiere_libelle: String,
    classe: String,
    heure_debut: chrono::NaiveTime,
    heure_fin: chrono::NaiveTime,
    nouvelle_salle: Option<String>,
    remplacant_nom: Option<String>,
    nouveau_jour: Option<NaiveDate>,
    nouvelle_heure_debut: Option<chrono::NaiveTime>,
    nouvelle_heure_fin: Option<chrono::NaiveTime>,
    motif_public: Option<String>,
    enregistre_le: chrono::DateTime<chrono::Utc>,
}

impl LigneChangement {
    fn vers(self) -> ChangementEdt {
        ChangementEdt {
            creneau_id: self.creneau_id,
            jour: self.jour,
            nature: self.nature,
            matiere_libelle: self.matiere_libelle,
            classe: self.classe,
            heure_debut: self.heure_debut,
            heure_fin: self.heure_fin,
            nouvelle_salle: self.nouvelle_salle,
            remplacant_nom: self.remplacant_nom,
            nouveau_jour: self.nouveau_jour,
            nouvelle_heure_debut: self.nouvelle_heure_debut,
            nouvelle_heure_fin: self.nouvelle_heure_fin,
            motif_public: self.motif_public,
            enregistre_le: self.enregistre_le,
        }
    }
}
