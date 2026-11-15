//! Socle 10, lot T3 : contrôles du §9.3 et effets des modifications qui placent quelque chose (déplacement,
//! ajout, fusion, remplacement), levée, correction du passé et lecture du journal.
//! Voir `edt_modif.rs` pour le modèle et le pont avec les écrivains historiques.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use chrono::{NaiveDate, NaiveTime};
use ent_auth::{Action, Role};
use ent_events::AggregateKind;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::edt_modif::{
    archiver_exceptions_de, charge_evenement, etat_occupant, inserer_modification,
    lever_projection, poser_annulations, recalculer_etat, resynchroniser_lieux, valider_motifs,
    ChangementSeance, ConflitModif, CtxModif, LigneModif, ResultatModif, SeanceMod,
    EVT_SEANCE_AJOUTEE, EVT_SEANCE_ANNULEE, EVT_SEANCE_MODIFIEE, EVT_SEANCE_RETABLIE,
    SELECT_SEANCE_MOD,
};
use crate::ref_commun::{conflit, db, introuvable, invalide, publier_agg, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Ce qu'on place : un jour, des heures murales, un lieu, et les contrôles à faire.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Placement {
    pub jour: NaiveDate,
    pub debut: NaiveTime,
    pub fin: NaiveTime,
    pub lieu: Option<Uuid>,
    /// Contrôler le public (aucun élève dans une autre séance active).
    pub public: bool,
    /// Contrôler les intervenants (libres, disponibles, présents).
    pub intervenants: bool,
}

/// Ce qui est placé : une séance existante (exclue des comparaisons) ou une séance à créer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CiblePlacement {
    pub exclu: Option<Uuid>,
    pub cours_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub prof_user_id: Uuid,
    pub remplacant_user_id: Option<Uuid>,
}

impl CiblePlacement {
    pub(crate) fn de(s: &SeanceMod) -> Self {
        Self {
            exclu: Some(s.id),
            cours_id: s.cours_id,
            classe_id: s.classe_id,
            prof_user_id: s.prof_user_id,
            remplacant_user_id: s.remplacant_user_id,
        }
    }
}

const OCCUPANTS: [&str; 3] = ["prevue", "modifiee", "ajoutee"];

impl PgEnt {
    /// Contrôles du §9.3, **à la date** : jour ouvré, public libre (inclusions comprises), intervenants libres,
    /// disponibles et présents, lieu libre (séances, réservations, indisponibilités), capacité, déplacement entre
    /// sites. Les conflits durs bloquent (sauf forçage), les avertissements s'affichent. Aucun nom n'est
    /// renvoyé : un code, un décompte, la séance en face.
    pub(crate) async fn controler_placement(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        cible: &CiblePlacement,
        p: &Placement,
    ) -> Result<Vec<ConflitModif>, EntDbError> {
        let mut out = Vec::new();
        let occ: Vec<String> = OCCUPANTS.iter().map(|s| (*s).to_string()).collect();
        let exclu = cible.exclu.unwrap_or_else(Uuid::nil);
        // 1. Jour ouvré (calendrier, vacances, fermetures par site).
        let site: Option<Uuid> = match cible.classe_id {
            Some(c) => sqlx::query_scalar(
                "SELECT site_id FROM ent_classe WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(c)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?
            .flatten(),
            None => None,
        };
        let ferme = self
            .calendrier_plage(etab, p.jour, p.jour)
            .await?
            .iter()
            .any(|j| j.site_id.is_none() || (site.is_some() && j.site_id == site));
        if ferme {
            out.push(ConflitModif::dur("jour_ferme", 1, None));
        }
        // 2. Public libre.
        let mut effectif: i64 = 0;
        if let Some(cours) = cible.cours_id {
            let (n, autre): (i64, Option<Uuid>) = sqlx::query_as(
                "SELECT count(DISTINCT p1.eleve_id), (array_agg(o.id ORDER BY o.heure_debut))[1]
                   FROM ent_public_cours_jour($1, $2, $3) p1
                   JOIN ent_edt_seance o ON o.etablissement_id = $1 AND o.jour = $2 AND o.id <> $4
                    AND o.etat = ANY($7) AND o.cours_id IS NOT NULL AND o.heure_debut < $6 AND $5 < o.heure_fin
                   JOIN LATERAL ent_public_cours_jour($1, $2, o.cours_id) p2 ON p2.eleve_id = p1.eleve_id",
            )
            .bind(etab)
            .bind(p.jour)
            .bind(cours)
            .bind(exclu)
            .bind(p.debut)
            .bind(p.fin)
            .bind(&occ)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            effectif = sqlx::query_scalar(
                "SELECT count(DISTINCT eleve_id) FROM ent_public_cours_jour($1, $2, $3)",
            )
            .bind(etab)
            .bind(p.jour)
            .bind(cours)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if p.public && n > 0 {
                out.push(ConflitModif::dur("public_occupe", n, autre));
            }
        }
        // 3. Intervenants : en saisie (remplaçant sinon titulaire) et co-intervenants du cours.
        let en_saisie = cible.remplacant_user_id.unwrap_or(cible.prof_user_id);
        let mut users: Vec<Uuid> = vec![en_saisie];
        if let Some(cours) = cible.cours_id {
            let co: Vec<Uuid> = sqlx::query_scalar(
                "SELECT DISTINCT prof_user_id FROM ent_service_enseignement
                  WHERE etablissement_id = $1 AND cours_id = $2",
            )
            .bind(etab)
            .bind(cours)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
            users.extend(co);
        }
        users.sort();
        users.dedup();
        if p.intervenants {
            let (n, autre): (i64, Option<Uuid>) = sqlx::query_as(
                "SELECT count(*), (array_agg(o.id ORDER BY o.heure_debut))[1] FROM ent_edt_seance o
                  WHERE o.etablissement_id = $1 AND o.jour = $2 AND o.id <> $3 AND o.etat = ANY($6)
                    AND o.heure_debut < $5 AND $4 < o.heure_fin
                    AND coalesce(o.remplacant_user_id, o.prof_user_id) = ANY($7)",
            )
            .bind(etab)
            .bind(p.jour)
            .bind(exclu)
            .bind(p.debut)
            .bind(p.fin)
            .bind(&occ)
            .bind(&users)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if n > 0 {
                out.push(ConflitModif::dur("intervenant_occupe", n, autre));
            }
            if self
                .prof_indisponible(etab, en_saisie, p.jour, p.debut, p.fin)
                .await?
            {
                out.push(ConflitModif::dur("intervenant_indisponible", 1, None));
            }
            if self.absent_le(etab, en_saisie, p.jour).await? {
                out.push(ConflitModif::dur("intervenant_absent", 1, None));
            }
        }
        // 4. Lieu : séances, réservations (socle 19), indisponibilités (salle fermée), capacité.
        if let Some(lieu) = p.lieu {
            let (n, autre): (i64, Option<Uuid>) = sqlx::query_as(
                "SELECT count(*), (array_agg(o.id ORDER BY o.heure_debut))[1]
                   FROM ent_edt_seance_lieu l
                   JOIN ent_edt_seance o ON o.id = l.seance_id AND o.etablissement_id = l.etablissement_id
                  WHERE l.etablissement_id = $1 AND l.lieu_id = $2 AND o.jour = $3 AND o.id <> $4
                    AND o.etat = ANY($7) AND o.heure_debut < $6 AND $5 < o.heure_fin",
            )
            .bind(etab)
            .bind(lieu)
            .bind(p.jour)
            .bind(exclu)
            .bind(p.debut)
            .bind(p.fin)
            .bind(&occ)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if n > 0 {
                out.push(ConflitModif::dur("lieu_occupe", n, autre));
            }
            let (reserv, indispo): (i64, i64) = sqlx::query_as(
                "WITH fz AS (SELECT coalesce((SELECT fuseau FROM ent_etablissement_parametre
                                               WHERE etablissement_id = $1), 'Europe/Paris') AS z),
                      t AS (SELECT (($3::date + $4::time) AT TIME ZONE fz.z) AS d,
                                   (($3::date + $5::time) AT TIME ZONE fz.z) AS f FROM fz)
                 SELECT (SELECT count(*) FROM ent_agenda_reservation r, t
                          WHERE r.etablissement_id = $1 AND r.salle_id = $2 AND r.statut = 'confirmee'
                            AND r.debut < t.f AND t.d < r.fin),
                        (SELECT count(*) FROM ent_salle_indisponibilite i, t
                          WHERE i.etablissement_id = $1 AND i.salle_id = $2 AND i.annulee_le IS NULL
                            AND i.du < t.f AND t.d < i.au)",
            )
            .bind(etab)
            .bind(lieu)
            .bind(p.jour)
            .bind(p.debut)
            .bind(p.fin)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if reserv > 0 {
                out.push(ConflitModif::dur("lieu_reserve", reserv, None));
            }
            if indispo > 0 {
                out.push(ConflitModif::dur("lieu_indisponible", indispo, None));
            }
            let capacite: Option<i32> = sqlx::query_scalar(
                "SELECT capacite FROM ent_salle WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(lieu)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?
            .flatten();
            if let Some(cap) = capacite {
                if effectif > i64::from(cap) {
                    out.push(ConflitModif::avertissement(
                        "capacite_depassee",
                        effectif - i64::from(cap),
                        None,
                    ));
                }
            }
            // Déplacement entre sites : le même intervenant enchaîne, à moins de dix minutes, deux sites.
            let (entre_sites,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_edt_seance o
                   JOIN ent_edt_seance_lieu ol ON ol.seance_id = o.id AND ol.etablissement_id = o.etablissement_id
                   JOIN ent_salle so ON so.id = ol.lieu_id AND so.etablissement_id = o.etablissement_id
                   JOIN ent_salle sn ON sn.etablissement_id = o.etablissement_id AND sn.id = $4
                  WHERE o.etablissement_id = $1 AND o.jour = $2 AND o.id <> $3 AND o.etat = ANY($7)
                    AND coalesce(o.remplacant_user_id, o.prof_user_id) = $8
                    AND so.site_id IS NOT NULL AND sn.site_id IS NOT NULL AND so.site_id <> sn.site_id
                    AND o.heure_debut < ($6::time + interval '10 minutes')
                    AND ($5::time - interval '10 minutes') < o.heure_fin",
            )
            .bind(etab)
            .bind(p.jour)
            .bind(exclu)
            .bind(lieu)
            .bind(p.debut)
            .bind(p.fin)
            .bind(&occ)
            .bind(en_saisie)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if p.intervenants && entre_sites > 0 {
                out.push(ConflitModif::avertissement(
                    "deplacement_entre_sites",
                    entre_sites,
                    None,
                ));
            }
        }
        Ok(out)
    }

    /// Remplacement ponctuel : la commande de remplacement historique reste la seule qui crée la délégation
    /// d'appel et de cahier du remplaçant et tient les verrous de concurrence ; elle journalise par le pont.
    pub(crate) async fn modifier_intervenant(
        &self,
        a: &Acteur<'_>,
        cmd: &crate::edt_modif::CmdModifierSeance,
        remplacant_user_id: Uuid,
    ) -> Result<ResultatModif, EntDbError> {
        let etab = a.etablissement_id;
        let seance_id = cmd.seance_id.ok_or_else(invalide)?;
        let s: SeanceMod = sqlx::query_as(&format!(
            "{SELECT_SEANCE_MOD} WHERE s.etablissement_id = $1 AND s.id = $2"
        ))
        .bind(etab)
        .bind(seance_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let mut res = ResultatModif {
            seance_id: Some(s.id),
            apercu: cmd.apercu,
            ..Default::default()
        };
        if s.passee {
            res.conflits
                .push(ConflitModif::dur("seance_passee", 1, None));
            return Ok(res);
        }
        if !etat_occupant(&s.etat) {
            return Err(conflit());
        }
        if cmd.apercu {
            return Ok(res);
        }
        let absence = (cmd.origine.as_deref() == Some("absence"))
            .then_some(cmd.origine_ref)
            .flatten();
        let r = self
            .edt_organiser_remplacement(
                a,
                &crate::edt_remplacement::CmdRemplacement {
                    creneau_id: s.creneau_id,
                    jour: s.jour,
                    remplacant_user_id,
                    absence_rh_id: absence,
                    nouvelle_salle: None,
                },
            )
            .await;
        let r = match r {
            Ok(r) => r,
            Err(EntDbError::Domain(domain_eleve::DomainError::Conflit)) => {
                res.conflits
                    .push(ConflitModif::dur("intervenant_non_disponible", 1, None));
                return Ok(res);
            }
            Err(e) => return Err(e),
        };
        let mid: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_edt_modification
              WHERE etablissement_id = $1 AND ancien_exception_id = $2 AND levee_le IS NULL
              ORDER BY cree_le DESC LIMIT 1",
        )
        .bind(etab)
        .bind(r.exception_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        res.applique = true;
        res.deja_fait = r.deja_fait;
        res.modification_id = mid;
        Ok(res)
    }

    /// Ajout d'une séance hors plan (rattrapage, soutien, devoir surveillé), EDT-164.
    pub(crate) async fn appliquer_ajout(
        &self,
        tx: &mut TenantTx<'_>,
        ctx: &CtxModif<'_>,
        changement: &ChangementSeance,
        exclu: Option<Uuid>,
        avec_cle: bool,
    ) -> Result<ResultatModif, EntDbError> {
        let ChangementSeance::Ajout {
            cours_id,
            jour,
            heure_debut,
            heure_fin,
            lieu_id,
        } = changement
        else {
            return Err(invalide());
        };
        let a = &ctx.a;
        let etab = a.etablissement_id;
        if heure_fin <= heure_debut {
            return Err(invalide());
        }
        let mut res = ResultatModif {
            apercu: ctx.apercu,
            ..Default::default()
        };
        // Jour d'établissement (heure de Paris) : un ajout dans le passé est une correction.
        if *jour < self.aujourdhui_etablissement(etab).await? {
            res.conflits
                .push(ConflitModif::dur("seance_passee", 1, None));
            return Ok(res);
        }
        let cours: Option<(Uuid, Option<Uuid>, NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT annee_id, classe_id, ouvert_le, ferme_le FROM ent_cours WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(cours_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        let Some((annee_id, classe_id, ouvert_le, ferme_le)) = cours else {
            return Err(introuvable());
        };
        if *jour < ouvert_le || ferme_le.is_some_and(|f| *jour > f) {
            return Err(conflit());
        }
        let annee_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2
                             AND date_debut <= $3 AND date_fin >= $3)",
        )
        .bind(etab)
        .bind(annee_id)
        .bind(jour)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        if !annee_ok {
            return Err(invalide());
        }
        // Le créneau du cours qui porte la séance : un créneau sans séance ce jour-là (clé historique).
        #[derive(FromRow)]
        struct Creneau {
            id: Uuid,
            classe: String,
            service_id: Option<Uuid>,
            prof_user_id: Uuid,
            matiere_id: Option<Uuid>,
            matiere_libelle: String,
            salle: Option<String>,
        }
        let creneau: Option<Creneau> = sqlx::query_as(
            "SELECT c.id, btrim(c.classe) AS classe, c.service_id, c.prof_user_id, c.matiere_id, c.matiere_libelle, c.salle
               FROM ent_edt_creneau c
              WHERE c.etablissement_id = $1 AND c.cours_id = $2
                AND NOT EXISTS (SELECT 1 FROM ent_edt_seance s
                                 WHERE s.etablissement_id = c.etablissement_id AND s.creneau_id = c.id AND s.jour = $3)
              ORDER BY c.id LIMIT 1",
        )
        .bind(etab)
        .bind(cours_id)
        .bind(jour)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        let Some(creneau) = creneau else {
            res.conflits
                .push(ConflitModif::dur("creneau_deja_ce_jour", 1, None));
            return Ok(res);
        };
        let lieu = match lieu_id {
            Some(l) => {
                let salle: Option<(String, bool)> = sqlx::query_as(
                    "SELECT code, actif FROM ent_salle WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(l)
                .fetch_optional(&mut **tx)
                .await
                .map_err(db)?;
                match salle {
                    Some((code, true)) => Some((*l, code)),
                    Some((_, false)) => return Err(conflit()),
                    None => return Err(introuvable()),
                }
            }
            None => None,
        };
        let cible = CiblePlacement {
            exclu,
            cours_id: Some(*cours_id),
            classe_id,
            prof_user_id: creneau.prof_user_id,
            remplacant_user_id: None,
        };
        let conflits = self
            .controler_placement(
                tx,
                etab,
                &cible,
                &Placement {
                    jour: *jour,
                    debut: *heure_debut,
                    fin: *heure_fin,
                    lieu: lieu.as_ref().map(|l| l.0),
                    public: true,
                    intervenants: true,
                },
            )
            .await?;
        let (affiches, forces) = Self::trancher_conflits(ctx, conflits);
        res.forces = forces.clone();
        res.conflits = affiches;
        if res.conflits.iter().any(|c| c.dur) {
            return Ok(res);
        }
        res.applique = true;
        if ctx.apercu {
            return Ok(res);
        }
        let seance_id = Uuid::new_v4();
        let salle_texte = lieu.as_ref().map(|l| l.1.clone()).or(creneau.salle.clone());
        sqlx::query(
            "INSERT INTO ent_edt_seance
               (id, etablissement_id, annee_id, creneau_id, jour, classe_id, classe, service_id, prof_user_id,
                matiere_id, matiere_libelle, salle, heure_debut, heure_fin, demi_journee, statut, source, cours_id,
                motif_public_code, motif_public)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,
                     CASE WHEN $13 < time '12:30' THEN 'matin' ELSE 'apres_midi' END,'prevue','manuel',$15,$16,$17)",
        )
        .bind(seance_id)
        .bind(etab)
        .bind(annee_id)
        .bind(creneau.id)
        .bind(jour)
        .bind(classe_id)
        .bind(&creneau.classe)
        .bind(creneau.service_id)
        .bind(creneau.prof_user_id)
        .bind(creneau.matiere_id)
        .bind(&creneau.matiere_libelle)
        .bind(&salle_texte)
        .bind(heure_debut)
        .bind(heure_fin)
        .bind(cours_id)
        .bind(&ctx.motif_code)
        .bind(&ctx.motif_public)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        resynchroniser_lieux(tx, etab, &[seance_id]).await?;
        let l = ctx.habiller(
            LigneModif {
                seance_id: Some(seance_id),
                creneau_id: Some(creneau.id),
                jour: Some(*jour),
                type_code: "ajout",
                apres_etat: Some("ajoutee".to_string()),
                apres_jour: Some(*jour),
                apres: Some((*heure_debut, *heure_fin)),
                apres_lieu: lieu.as_ref().map(|l| l.0),
                apres_salle_texte: salle_texte.clone(),
                ..Default::default()
            },
            &forces,
        );
        let mut l = l;
        if !avec_cle {
            l.cle = None; // la clé d'idempotence est portée par l'autre ligne (remplacement par un autre cours)
        }
        let mid = inserer_modification(tx, etab, a.user_id, &l).await?;
        res.seance_id = Some(seance_id);
        res.modification_id = Some(mid);
        if ctx.lot_id.is_some() {
            return Ok(res);
        }
        let ev = publier_agg(
            tx,
            a,
            AggregateKind::Edt,
            EVT_SEANCE_AJOUTEE,
            seance_id,
            charge_evenement(
                mid,
                Some(seance_id),
                Some(creneau.id),
                *jour,
                "ajout",
                &ctx.motif_code,
                ctx.motif_public.as_deref(),
                &ctx.origine,
            ),
        )
        .await?;
        if let Ok(mut v) = ctx.evenements.lock() {
            v.push(ev);
        }
        Ok(res)
    }

    /// Déplacement d'une séance à un autre jour (EDT-161) : l'ancienne séance est `deplacee`, une séance
    /// d'arrivée `ajoutee` est créée, la projection historique (exception, déplacement) suit.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn appliquer_deplacement(
        &self,
        tx: &mut TenantTx<'_>,
        ctx: &CtxModif<'_>,
        s: &SeanceMod,
        nouveau_jour: NaiveDate,
        heure_debut: NaiveTime,
        heure_fin: NaiveTime,
        lieu_id: Option<Uuid>,
        mut res: ResultatModif,
    ) -> Result<ResultatModif, EntDbError> {
        let a = &ctx.a;
        let etab = a.etablissement_id;
        if nouveau_jour < self.aujourdhui_etablissement(etab).await? {
            res.conflits
                .push(ConflitModif::dur("seance_passee", 1, None));
            return Ok(res);
        }
        let annee_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2
                             AND date_debut <= $3 AND date_fin >= $3)",
        )
        .bind(etab)
        .bind(s.annee_id)
        .bind(nouveau_jour)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        if !annee_ok {
            return Err(invalide());
        }
        // Clé historique : une séance par créneau et par jour.
        let occupe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_edt_seance WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3)",
        )
        .bind(etab)
        .bind(s.creneau_id)
        .bind(nouveau_jour)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        if occupe {
            res.conflits
                .push(ConflitModif::dur("creneau_deja_ce_jour", 1, None));
            return Ok(res);
        }
        let lieu = match lieu_id.or(s.lieu_id) {
            Some(l) => {
                let salle: Option<(String, bool)> = sqlx::query_as(
                    "SELECT code, actif FROM ent_salle WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(l)
                .fetch_optional(&mut **tx)
                .await
                .map_err(db)?;
                match salle {
                    Some((code, true)) => Some((l, code)),
                    Some((_, false)) => return Err(conflit()),
                    None => return Err(introuvable()),
                }
            }
            None => None,
        };
        let conflits = self
            .controler_placement(
                tx,
                etab,
                &CiblePlacement::de(s),
                &Placement {
                    jour: nouveau_jour,
                    debut: heure_debut,
                    fin: heure_fin,
                    lieu: lieu.as_ref().map(|l| l.0),
                    public: true,
                    intervenants: true,
                },
            )
            .await?;
        let (affiches, forces) = Self::trancher_conflits(ctx, conflits);
        res.forces = forces.clone();
        res.conflits = affiches;
        if res.conflits.iter().any(|c| c.dur) {
            return Ok(res);
        }
        res.applique = true;
        if ctx.apercu {
            return Ok(res);
        }
        let arrivee = Uuid::new_v4();
        let salle_texte = lieu.as_ref().map(|l| l.1.clone()).or(s.salle.clone());
        sqlx::query(
            "INSERT INTO ent_edt_seance
               (id, etablissement_id, annee_id, creneau_id, jour, classe_id, classe, service_id, prof_user_id,
                matiere_id, matiere_libelle, salle, heure_debut, heure_fin, demi_journee, statut, source, cours_id)
             SELECT $3, o.etablissement_id, o.annee_id, o.creneau_id, $4, o.classe_id, o.classe, o.service_id,
                    o.prof_user_id, o.matiere_id, o.matiere_libelle, $5, $6, $7,
                    CASE WHEN $6 < time '12:30' THEN 'matin' ELSE 'apres_midi' END, 'prevue', 'deplacement', o.cours_id
               FROM ent_edt_seance o WHERE o.etablissement_id = $1 AND o.id = $2",
        )
        .bind(etab)
        .bind(s.id)
        .bind(arrivee)
        .bind(nouveau_jour)
        .bind(&salle_texte)
        .bind(heure_debut)
        .bind(heure_fin)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        resynchroniser_lieux(tx, etab, &[arrivee]).await?;
        self.lever_vivantes(
            tx,
            etab,
            s.id,
            &[
                "annulation",
                "deplacement",
                "neutralisation",
                "changement_lieu",
            ],
            a.user_id,
        )
        .await?;
        let l = ctx.habiller(
            LigneModif {
                seance_id: Some(s.id),
                arrivee: Some(arrivee),
                creneau_id: Some(s.creneau_id),
                jour: Some(s.jour),
                type_code: "deplacement",
                avant_etat: Some(s.etat.clone()),
                apres_etat: Some("deplacee".to_string()),
                avant_jour: Some(s.jour),
                apres_jour: Some(nouveau_jour),
                avant: Some((s.heure_debut, s.heure_fin)),
                apres: Some((heure_debut, heure_fin)),
                avant_lieu: s.lieu_id,
                apres_lieu: lieu.as_ref().map(|l| l.0),
                apres_salle_texte: salle_texte.clone(),
                ..Default::default()
            },
            &forces,
        );
        let mid = inserer_modification(tx, etab, a.user_id, &l).await?;
        poser_annulations(
            tx,
            etab,
            &[s.id],
            a.user_id,
            None,
            &ctx.motif_code,
            ctx.motif_public.as_deref(),
            "deplacement",
        )
        .await?;
        sqlx::query(
            "INSERT INTO ent_edt_deplacement
               (id, etablissement_id, creneau_id, jour_origine, nouveau_jour, nouvelle_heure_debut,
                nouvelle_heure_fin, nouvelle_salle, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             ON CONFLICT (etablissement_id, creneau_id, jour_origine) DO UPDATE SET
                nouveau_jour = EXCLUDED.nouveau_jour, nouvelle_heure_debut = EXCLUDED.nouvelle_heure_debut,
                nouvelle_heure_fin = EXCLUDED.nouvelle_heure_fin, nouvelle_salle = EXCLUDED.nouvelle_salle,
                cree_par_user_id = EXCLUDED.cree_par_user_id",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(s.creneau_id)
        .bind(s.jour)
        .bind(nouveau_jour)
        .bind(heure_debut)
        .bind(heure_fin)
        .bind(lieu.as_ref().map(|l| l.1.clone()))
        .bind(a.user_id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        sqlx::query("UPDATE ent_edt_seance SET remplacee_par_id = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(s.id)
            .bind(arrivee)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        res.modification_id = Some(mid);
        res.seance_arrivee_id = Some(arrivee);
        self.evenement_modif_edt(tx, a, s, mid, "deplacement", ctx, EVT_SEANCE_MODIFIEE)
            .await?;
        Ok(res)
    }

    /// Fusion de deux séances (EDT-165) : `s` est conservée, `avec` y est absorbée (neutralisée), et son cours
    /// rejoint le public de la séance conservée (`ent_edt_seance_public_fusion`).
    pub(crate) async fn appliquer_fusion(
        &self,
        tx: &mut TenantTx<'_>,
        ctx: &CtxModif<'_>,
        s: &SeanceMod,
        avec: Uuid,
        mut res: ResultatModif,
    ) -> Result<ResultatModif, EntDbError> {
        let a = &ctx.a;
        let etab = a.etablissement_id;
        let b: SeanceMod = sqlx::query_as(&format!(
            "{SELECT_SEANCE_MOD} WHERE s.etablissement_id = $1 AND s.id = $2 FOR UPDATE OF s"
        ))
        .bind(etab)
        .bind(avec)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if b.passee {
            res.conflits
                .push(ConflitModif::dur("seance_passee", 1, None));
            return Ok(res);
        }
        if !etat_occupant(&b.etat)
            || b.jour != s.jour
            || s.cours_id.is_none()
            || b.cours_id.is_none()
            || b.cours_id == s.cours_id
            || !(b.heure_debut < s.heure_fin && s.heure_debut < b.heure_fin)
        {
            return Err(conflit());
        }
        // Capacité du lieu conservé : public des deux cours.
        if let Some(lieu) = s.lieu_id {
            let capacite: Option<i32> = sqlx::query_scalar(
                "SELECT capacite FROM ent_salle WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(lieu)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db)?
            .flatten();
            let total: i64 = sqlx::query_scalar(
                "SELECT count(DISTINCT eleve_id) FROM (
                     SELECT eleve_id FROM ent_public_cours_jour($1, $2, $3)
                     UNION SELECT eleve_id FROM ent_public_cours_jour($1, $2, $4)) x",
            )
            .bind(etab)
            .bind(s.jour)
            .bind(s.cours_id)
            .bind(b.cours_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(db)?;
            if let Some(cap) = capacite {
                if total > i64::from(cap) {
                    res.conflits.push(ConflitModif::avertissement(
                        "capacite_depassee",
                        total - i64::from(cap),
                        None,
                    ));
                }
            }
        }
        res.applique = true;
        if ctx.apercu {
            return Ok(res);
        }
        self.lever_vivantes(
            tx,
            etab,
            b.id,
            &["annulation", "deplacement", "neutralisation"],
            a.user_id,
        )
        .await?;
        let ligne_a = ctx.habiller(
            LigneModif {
                seance_id: Some(s.id),
                arrivee: Some(b.id),
                creneau_id: Some(s.creneau_id),
                jour: Some(s.jour),
                type_code: "fusion",
                avant_etat: Some(s.etat.clone()),
                apres_etat: Some("modifiee".to_string()),
                avant: Some((s.heure_debut, s.heure_fin)),
                ..Default::default()
            },
            &[],
        );
        let mid = inserer_modification(tx, etab, a.user_id, &ligne_a).await?;
        // La séance absorbée : neutralisée, sans cle (la clé d'idempotence est portée par la ligne de la séance conservée).
        let mut ligne_b = ctx.habiller(
            LigneModif {
                seance_id: Some(b.id),
                arrivee: Some(s.id),
                creneau_id: Some(b.creneau_id),
                jour: Some(b.jour),
                type_code: "neutralisation",
                avant_etat: Some(b.etat.clone()),
                apres_etat: Some("neutralisee".to_string()),
                avant: Some((b.heure_debut, b.heure_fin)),
                ..Default::default()
            },
            &[],
        );
        ligne_b.cle = None;
        ligne_b.motif_interne = None;
        inserer_modification(tx, etab, a.user_id, &ligne_b).await?;
        poser_annulations(
            tx,
            etab,
            &[b.id],
            a.user_id,
            None,
            &ctx.motif_code,
            ctx.motif_public.as_deref(),
            "annulation",
        )
        .await?;
        sqlx::query(
            "INSERT INTO ent_edt_seance_public_fusion (seance_id, etablissement_id, cours_id) VALUES ($1,$2,$3)
             ON CONFLICT DO NOTHING",
        )
        .bind(s.id)
        .bind(etab)
        .bind(b.cours_id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_edt_seance SET origine = 'fusion', updated_at = now() WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(s.id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        sqlx::query("UPDATE ent_edt_seance SET remplacee_par_id = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(b.id)
            .bind(s.id)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        res.modification_id = Some(mid);
        res.seance_arrivee_id = Some(b.id);
        self.evenement_modif_edt(tx, a, s, mid, "fusion", ctx, EVT_SEANCE_MODIFIEE)
            .await?;
        self.evenement_modif_edt(tx, a, &b, mid, "neutralisation", ctx, EVT_SEANCE_ANNULEE)
            .await?;
        Ok(res)
    }
}

// ───────────────────────────── Levée ─────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultatLevee {
    pub deja_fait: bool,
    pub modification_id: Uuid,
    pub seance_id: Option<Uuid>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub(crate) struct ModifLue {
    id: Uuid,
    seance_id: Option<Uuid>,
    seance_arrivee_id: Option<Uuid>,
    lot_id: Option<Uuid>,
    creneau_id: Option<Uuid>,
    jour: NaiveDate,
    type_: String,
    avant_debut: Option<NaiveTime>,
    avant_fin: Option<NaiveTime>,
    motif_public_code: String,
    motif_public: Option<String>,
    origine: String,
    levee: bool,
}

impl PgEnt {
    /// Lève une modification (EDT-168) : la séance revient à son état calculé, une modification **de la même
    /// séance** liée (fusion) est levée avec elle ; la levée d'une modification déjà annoncée déclenche le
    /// message de rétablissement (événement `edt.seance_retablie`). Idempotente.
    pub async fn edt_lever_modification(
        &self,
        a: &Acteur<'_>,
        modification_id: Uuid,
    ) -> Result<ResultatLevee, EntDbError> {
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let m: ModifLue = sqlx::query_as(
            "SELECT id, seance_id, seance_arrivee_id, lot_id, creneau_id, jour, type AS type_, avant_debut, avant_fin,
                    motif_public_code, motif_public, origine, (levee_le IS NOT NULL) AS levee
               FROM ent_edt_modification WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(modification_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let action = if m.type_ == "changement_lieu" {
            Action::EdtModifierLieu
        } else {
            Action::EdtModifier
        };
        self.exiger_edt(a, action).await?;
        if m.levee {
            return Ok(ResultatLevee {
                deja_fait: true,
                modification_id,
                seance_id: m.seance_id,
            });
        }
        if m.type_ == "correction" {
            return Err(conflit()); // une correction se corrige par une autre correction
        }
        let Some(seance_id) = m.seance_id else {
            return Err(conflit());
        };
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 6))")
            .bind(format!("{etab}:seance:{seance_id}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let s: SeanceMod = sqlx::query_as(&format!(
            "{SELECT_SEANCE_MOD} WHERE s.etablissement_id = $1 AND s.id = $2 FOR UPDATE OF s"
        ))
        .bind(etab)
        .bind(seance_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if s.passee {
            return Err(conflit()); // §9.6 : le passé ne se lève pas, il se corrige
        }
        self.lever_une(&mut tx, a, &m, &s).await?;
        let ctx = CtxModif {
            a: *a,
            motif_code: m.motif_public_code.clone(),
            motif_public: m.motif_public.clone(),
            motif_interne: None,
            origine: m.origine.clone(),
            origine_ref: None,
            cle: None,
            forcer: None,
            apercu: false,
            lot_id: None,
            evenements: std::sync::Mutex::new(Vec::new()),
        };
        self.evenement_modif_edt(&mut tx, a, &s, m.id, &m.type_, &ctx, EVT_SEANCE_RETABLIE)
            .await?;
        tx.commit().await.map_err(db)?;
        if let Ok(evs) = ctx.evenements.lock() {
            self.diffuser_apres_commit(&evs);
        }
        Ok(ResultatLevee {
            deja_fait: false,
            modification_id,
            seance_id: Some(seance_id),
        })
    }

    /// Levée d'une modification dans la transaction de l'appelant (séance déjà verrouillée).
    pub(crate) async fn lever_une(
        &self,
        tx: &mut TenantTx<'_>,
        a: &Acteur<'_>,
        m: &ModifLue,
        s: &SeanceMod,
    ) -> Result<(), EntDbError> {
        let etab = a.etablissement_id;
        // Le journal d'abord : le déclencheur d'état dérive ensuite de ce qui reste vivant.
        sqlx::query(
            "UPDATE ent_edt_modification SET levee_le = now(), levee_par = $3
              WHERE etablissement_id = $1 AND id = $2 AND levee_le IS NULL",
        )
        .bind(etab)
        .bind(m.id)
        .bind(a.user_id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        match m.type_.as_str() {
            "annulation" | "neutralisation" => {
                lever_projection(tx, etab, &[s.id], a.user_id).await?;
            }
            "changement_lieu" => {
                let kind: Option<String> = sqlx::query_scalar(
                    "SELECT kind FROM ent_edt_exception WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3",
                )
                .bind(etab)
                .bind(s.creneau_id)
                .bind(s.jour)
                .fetch_optional(&mut **tx)
                .await
                .map_err(db)?;
                if kind.as_deref() == Some("remplacement") {
                    // Le remplaçant reste ; seule la salle revient au plan.
                    sqlx::query(
                        "UPDATE ent_edt_exception SET nouvelle_salle = NULL, nature = 'remplacement'
                          WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3",
                    )
                    .bind(etab)
                    .bind(s.creneau_id)
                    .bind(s.jour)
                    .execute(&mut **tx)
                    .await
                    .map_err(db)?;
                    sqlx::query(
                        "UPDATE ent_edt_seance x SET salle = c.salle, updated_at = now() FROM ent_edt_creneau c
                          WHERE x.etablissement_id = $1 AND x.id = $2 AND c.id = x.creneau_id
                            AND c.etablissement_id = x.etablissement_id",
                    )
                    .bind(etab)
                    .bind(s.id)
                    .execute(&mut **tx)
                    .await
                    .map_err(db)?;
                    resynchroniser_lieux(tx, etab, &[s.id]).await?;
                } else {
                    lever_projection(tx, etab, &[s.id], a.user_id).await?;
                }
            }
            "deplacement" | "intervenant" => {
                // Projection historique complète : déplacement archivé, séance d'arrivée retirée, délégation
                // révoquée (la commande historique, dans cette transaction).
                Self::lever_exception_tx(tx, etab, s.creneau_id, s.jour, a.user_id).await?;
            }
            "changement_horaire" => {
                if let (Some(d), Some(f)) = (m.avant_debut, m.avant_fin) {
                    sqlx::query(
                        "UPDATE ent_edt_seance SET heure_debut = $3, heure_fin = $4, updated_at = now(),
                                demi_journee = CASE WHEN $3 < time '12:30' THEN 'matin' ELSE 'apres_midi' END
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(s.id)
                    .bind(d)
                    .bind(f)
                    .execute(&mut **tx)
                    .await
                    .map_err(db)?;
                }
                recalculer_etat(tx, etab, &[s.id]).await?;
            }
            "ajout" => {
                crate::referentiel_seances::retirer_ou_supprimer_seances(
                    tx,
                    etab,
                    &[s.id],
                    "ajout levé",
                )
                .await?;
            }
            "fusion" => {
                if let Some(b) = m.seance_arrivee_id {
                    // La séance absorbée revient (sa ligne de neutralisation est levée avec la fusion).
                    sqlx::query(
                        "UPDATE ent_edt_modification SET levee_le = now(), levee_par = $4
                          WHERE etablissement_id = $1 AND seance_id = $2 AND seance_arrivee_id = $3
                            AND type = 'neutralisation' AND levee_le IS NULL",
                    )
                    .bind(etab)
                    .bind(b)
                    .bind(s.id)
                    .bind(a.user_id)
                    .execute(&mut **tx)
                    .await
                    .map_err(db)?;
                    lever_projection(tx, etab, &[b], a.user_id).await?;
                    sqlx::query("UPDATE ent_edt_seance SET remplacee_par_id = NULL WHERE etablissement_id = $1 AND id = $2")
                        .bind(etab)
                        .bind(b)
                        .execute(&mut **tx)
                        .await
                        .map_err(db)?;
                    sqlx::query(
                        "DELETE FROM ent_edt_seance_public_fusion f USING ent_edt_seance x
                          WHERE f.etablissement_id = $1 AND f.seance_id = $2 AND x.id = $3
                            AND x.etablissement_id = f.etablissement_id AND f.cours_id = x.cours_id",
                    )
                    .bind(etab)
                    .bind(s.id)
                    .bind(b)
                    .execute(&mut **tx)
                    .await
                    .map_err(db)?;
                }
                sqlx::query("UPDATE ent_edt_seance SET origine = 'plan', updated_at = now() WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(s.id)
                    .execute(&mut **tx)
                    .await
                    .map_err(db)?;
                recalculer_etat(tx, etab, &[s.id]).await?;
            }
            _ => return Err(conflit()),
        }
        let _ = m.lot_id;
        let _ = m.creneau_id;
        let _ = m.jour;
        Ok(())
    }
}

// ───────────────────────────── Correction du passé ─────────────────────────────

/// Correction d'une séance passée (§9.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChangementPasse {
    /// Le cours a bien eu lieu (il était annulé ou non assuré).
    Tenue,
    /// Le cours n'a pas eu lieu (il était prévu).
    NonTenue,
    Lieu {
        lieu_id: Uuid,
    },
    Horaire {
        heure_debut: NaiveTime,
        heure_fin: NaiveTime,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct CmdCorrectionPasse {
    pub seance_id: Uuid,
    pub changement: ChangementPasse,
    pub motif_public_code: String,
    #[serde(default)]
    pub motif_public: Option<String>,
    /// OBLIGATOIRE : une correction du passé se justifie (lue par la direction et le secrétariat seulement).
    pub motif_interne: String,
    #[serde(default)]
    pub cle: Option<String>,
}

/// Ce que la correction touche en aval : signalé, jamais défait en silence (§9.6).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffetsAval {
    pub appel_deja_fait: bool,
    pub contenus_cahier: i64,
    pub travaux: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultatCorrection {
    pub modification_id: Uuid,
    pub deja_fait: bool,
    pub effets_aval: EffetsAval,
}

impl PgEnt {
    /// EDT-173 : corrige une séance passée. Droit spécial (`edt.corriger_passe`), motif interne obligatoire,
    /// journalisée (`passe = true`), effets en aval signalés.
    pub async fn edt_corriger_seance_passee(
        &self,
        a: &Acteur<'_>,
        cmd: &CmdCorrectionPasse,
    ) -> Result<ResultatCorrection, EntDbError> {
        self.exiger_edt(a, Action::EdtCorrigerPasse).await?;
        let etab = a.etablissement_id;
        let (motif_public, interne) = valider_motifs(
            &cmd.motif_public_code,
            cmd.motif_public.as_deref(),
            Some(&cmd.motif_interne),
            cmd.cle.as_deref(),
            None,
        )?;
        let Some(interne) = interne else {
            return Err(invalide());
        }; // motif interne obligatoire
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if let Some(cle) = cmd.cle.as_deref() {
            let deja: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_edt_modification WHERE etablissement_id = $1 AND cle = $2 AND type = 'correction'
                    AND seance_id = $3",
            )
            .bind(etab)
            .bind(cle)
            .bind(cmd.seance_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((id,)) = deja {
                return Ok(ResultatCorrection {
                    modification_id: id,
                    deja_fait: true,
                    effets_aval: EffetsAval::default(),
                });
            }
        }
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 6))")
            .bind(format!("{etab}:seance:{}", cmd.seance_id))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let s: SeanceMod = sqlx::query_as(&format!(
            "{SELECT_SEANCE_MOD} WHERE s.etablissement_id = $1 AND s.id = $2 FOR UPDATE OF s"
        ))
        .bind(etab)
        .bind(cmd.seance_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if !s.passee || s.etat == "retiree" {
            return Err(conflit()); // une séance à venir se modifie par la commande ordinaire
        }
        // Effets en aval, relevés AVANT la correction.
        let appel: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_appel WHERE etablissement_id = $1 AND seance_id = $2)",
        )
        .bind(etab)
        .bind(s.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let contenus: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_seance_contenu WHERE etablissement_id = $1 AND seance_id = $2",
        )
        .bind(etab)
        .bind(s.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let travaux: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_travail WHERE etablissement_id = $1 AND seance_id = $2",
        )
        .bind(etab)
        .bind(s.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let ctx = CtxModif {
            a: *a,
            motif_code: cmd.motif_public_code.clone(),
            motif_public,
            motif_interne: Some(interne),
            origine: "saisie".to_string(),
            origine_ref: None,
            cle: cmd.cle.clone(),
            forcer: None,
            apercu: false,
            lot_id: None,
            evenements: std::sync::Mutex::new(Vec::new()),
        };
        let mut ligne = LigneModif {
            seance_id: Some(s.id),
            creneau_id: Some(s.creneau_id),
            jour: Some(s.jour),
            type_code: "correction",
            avant_etat: Some(s.etat.clone()),
            avant: Some((s.heure_debut, s.heure_fin)),
            avant_lieu: s.lieu_id,
            passe: true,
            ..Default::default()
        };
        match &cmd.changement {
            ChangementPasse::Tenue => {
                if etat_occupant(&s.etat) {
                    return Err(conflit());
                }
                ligne.apres_etat = Some("prevue".to_string());
            }
            ChangementPasse::NonTenue => {
                if !etat_occupant(&s.etat) {
                    return Err(conflit());
                }
                ligne.apres_etat = Some("annulee".to_string());
            }
            ChangementPasse::Lieu { lieu_id } => {
                let salle: Option<String> = sqlx::query_scalar(
                    "SELECT code FROM ent_salle WHERE etablissement_id = $1 AND id = $2 AND actif",
                )
                .bind(etab)
                .bind(lieu_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                let Some(code) = salle else {
                    return Err(introuvable());
                };
                ligne.apres_etat = Some(s.etat.clone());
                ligne.apres_lieu = Some(*lieu_id);
                ligne.apres_salle_texte = Some(code);
            }
            ChangementPasse::Horaire {
                heure_debut,
                heure_fin,
            } => {
                if heure_fin <= heure_debut {
                    return Err(invalide());
                }
                ligne.apres_etat = Some(s.etat.clone());
                ligne.apres = Some((*heure_debut, *heure_fin));
            }
        }
        let ligne = ctx.habiller(ligne, &[]);
        let mid = inserer_modification(&mut tx, etab, a.user_id, &ligne).await?;
        match &cmd.changement {
            ChangementPasse::Tenue => {
                // Les modifications vivantes qui annulaient la séance sont levées par la correction.
                self.lever_vivantes(
                    &mut tx,
                    etab,
                    s.id,
                    &["annulation", "neutralisation", "deplacement"],
                    a.user_id,
                )
                .await?;
                lever_projection(&mut tx, etab, &[s.id], a.user_id).await?;
            }
            ChangementPasse::NonTenue => {
                poser_annulations(
                    &mut tx,
                    etab,
                    &[s.id],
                    a.user_id,
                    None,
                    &ctx.motif_code,
                    ctx.motif_public.as_deref(),
                    "annulation",
                )
                .await?;
            }
            ChangementPasse::Lieu { .. } => {
                if let Some(code) = &ligne.apres_salle_texte {
                    self.ecrire_exception_lieu_publique(&mut tx, etab, &s, code, a.user_id)
                        .await?;
                }
            }
            ChangementPasse::Horaire {
                heure_debut,
                heure_fin,
            } => {
                sqlx::query(
                    "UPDATE ent_edt_seance SET heure_debut = $3, heure_fin = $4, updated_at = now(),
                            demi_journee = CASE WHEN $3 < time '12:30' THEN 'matin' ELSE 'apres_midi' END
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(s.id)
                .bind(heure_debut)
                .bind(heure_fin)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        let charge = json!({
            "modification_id": mid, "seance_id": s.id, "creneau_id": s.creneau_id, "jour": s.jour,
            "type": "correction", "passe": true, "motif_public_code": ctx.motif_code,
            "motif_public": ctx.motif_public, "origine": "saisie",
        });
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            EVT_SEANCE_MODIFIEE,
            s.id,
            charge,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ResultatCorrection {
            modification_id: mid,
            deja_fait: false,
            effets_aval: EffetsAval {
                appel_deja_fait: appel,
                contenus_cahier: contenus,
                travaux,
            },
        })
    }

    /// Projection du lieu d'une séance passée corrigée (même écriture que la modification de lieu).
    async fn ecrire_exception_lieu_publique(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        s: &SeanceMod,
        code: &str,
        par: Uuid,
    ) -> Result<(), EntDbError> {
        archiver_exceptions_de(tx, etab, &[s.id], "remplacee", par).await?;
        sqlx::query(
            "INSERT INTO ent_edt_exception
               (id, etablissement_id, creneau_id, jour, kind, nouvelle_salle, motif, created_by_user_id,
                created_at, nature)
             VALUES ($1,$2,$3,$4,'changement_salle',$5,'',$6,now(),'changement_salle')
             ON CONFLICT (etablissement_id, creneau_id, jour) DO UPDATE SET
                nouvelle_salle = EXCLUDED.nouvelle_salle, created_by_user_id = EXCLUDED.created_by_user_id,
                created_at = now(),
                kind = CASE WHEN ent_edt_exception.kind = 'remplacement' THEN 'remplacement' ELSE 'changement_salle' END,
                nature = 'changement_salle'",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(s.creneau_id)
        .bind(s.jour)
        .bind(code)
        .bind(par)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        sqlx::query("UPDATE ent_edt_seance SET salle = $3, updated_at = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(s.id)
            .bind(code)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        resynchroniser_lieux(tx, etab, &[s.id]).await?;
        Ok(())
    }
}

// ───────────────────────────── Lecture du journal ─────────────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FiltreModifications {
    #[serde(default)]
    pub seance_id: Option<Uuid>,
    #[serde(default)]
    pub lot_id: Option<Uuid>,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    #[serde(default)]
    pub vivantes_seulement: bool,
}

/// Une modification telle que la voit un lecteur : motif PUBLIC seulement ; le motif interne n'est rempli que
/// pour la direction et le secrétariat.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ModificationVue {
    pub id: Uuid,
    pub seance_id: Option<Uuid>,
    pub seance_arrivee_id: Option<Uuid>,
    pub lot_id: Option<Uuid>,
    pub jour: NaiveDate,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub type_code: String,
    pub avant_etat: Option<String>,
    pub apres_etat: Option<String>,
    pub avant_jour: Option<NaiveDate>,
    pub apres_jour: Option<NaiveDate>,
    pub avant_debut: Option<NaiveTime>,
    pub avant_fin: Option<NaiveTime>,
    pub apres_debut: Option<NaiveTime>,
    pub apres_fin: Option<NaiveTime>,
    pub avant_lieu_id: Option<Uuid>,
    pub apres_lieu_id: Option<Uuid>,
    pub motif_public_code: String,
    pub motif_public: Option<String>,
    pub origine: String,
    pub forcee: bool,
    pub passe: bool,
    pub levee: bool,
    pub cree_le: chrono::DateTime<chrono::Utc>,
    #[sqlx(default)]
    pub motif_interne: Option<String>,
}

impl PgEnt {
    /// Journal des modifications. Le personnel d'encadrement (`edt.modifications_lire`) lit l'établissement ;
    /// un professeur ne lit que les modifications des séances qu'il assure (titulaire ou remplaçant), sur une
    /// séance précise : hors de ce périmètre, refus. Le motif interne n'est joint que pour `edt.motif_interne_lire`.
    pub async fn edt_modifications(
        &self,
        a: &Acteur<'_>,
        f: &FiltreModifications,
    ) -> Result<Vec<ModificationVue>, EntDbError> {
        let etab = a.etablissement_id;
        let role = Role::parse(a.role).ok_or(EntDbError::Forbidden)?;
        if !role.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        let encadrement = self.ref_peut(a, Action::EdtModificationsLire).await?;
        let interne = encadrement && self.ref_peut(a, Action::EdtMotifInterneLire).await?;
        if f.du
            .zip(f.au)
            .is_some_and(|(d, e)| e < d || (e - d).num_days() > 400)
        {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if !encadrement {
            // Périmètre du professeur : une séance précise, qu'il assure. Sinon refus (pas d'oracle).
            let Some(sid) = f.seance_id else {
                return Err(EntDbError::Forbidden);
            };
            let concerne: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_edt_seance WHERE etablissement_id = $1 AND id = $2
                                 AND (prof_user_id = $3 OR remplacant_user_id = $3))",
            )
            .bind(etab)
            .bind(sid)
            .bind(a.user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !concerne || role != Role::Prof {
                return Err(EntDbError::Forbidden);
            }
        }
        let sql = "SELECT m.id, m.seance_id, m.seance_arrivee_id, m.lot_id, m.jour, m.type, m.avant_etat, m.apres_etat,
                          m.avant_jour, m.apres_jour, m.avant_debut, m.avant_fin, m.apres_debut, m.apres_fin,
                          m.avant_lieu_id, m.apres_lieu_id, m.motif_public_code, m.motif_public, m.origine,
                          (m.force_motif IS NOT NULL) AS forcee, m.passe, (m.levee_le IS NOT NULL) AS levee,
                          m.cree_le,
                          CASE WHEN $6 THEN i.motif_interne END AS motif_interne
                     FROM ent_edt_modification m
                     LEFT JOIN ent_edt_modification_interne i
                       ON i.modification_id = m.id AND i.etablissement_id = m.etablissement_id
                    WHERE m.etablissement_id = $1
                      AND ($2::uuid IS NULL OR m.seance_id = $2) AND ($3::uuid IS NULL OR m.lot_id = $3)
                      AND ($4::date IS NULL OR m.jour >= $4) AND ($5::date IS NULL OR m.jour <= $5)
                      AND (NOT $7 OR m.levee_le IS NULL)
                    ORDER BY m.jour, m.cree_le, m.id LIMIT 1000";
        sqlx::query_as::<_, ModificationVue>(sql)
            .bind(etab)
            .bind(f.seance_id)
            .bind(f.lot_id)
            .bind(f.du)
            .bind(f.au)
            .bind(interne)
            .bind(f.vivantes_seulement)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)
    }
}

// ───────────────────────────── Rattrapage, séances non assurées ─────────────────────────────

/// Une plage où le public, l'intervenant et le lieu d'une séance annulée sont libres (EDT-175).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlageRattrapage {
    pub jour: NaiveDate,
    pub debut: NaiveTime,
    pub fin: NaiveTime,
}

/// Une séance à venir sans intervenant en saisie (EDT-176) : cours vacant ou état `non_assuree`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
pub struct SeanceNonAssuree {
    pub seance_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub cours_id: Option<Uuid>,
    pub classe: String,
    /// `cours_vacant` (poste vacant) ou `non_assuree` (état posé par une absence non remplacée).
    pub raison: String,
}

impl PgEnt {
    /// EDT-175 : propose des plages de rattrapage pour une séance (annulée ou non) : celles où la classe,
    /// l'intervenant en saisie et le lieu de la séance sont libres, sur la plage demandée.
    pub async fn edt_rattrapage_propositions(
        &self,
        a: &Acteur<'_>,
        seance_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<PlageRattrapage>, EntDbError> {
        self.exiger_edt(a, Action::EdtModifier).await?;
        let etab = a.etablissement_id;
        let s: SeanceMod = sqlx::query_as(&format!(
            "{SELECT_SEANCE_MOD} WHERE s.etablissement_id = $1 AND s.id = $2"
        ))
        .bind(etab)
        .bind(seance_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let duree = (s.heure_fin - s.heure_debut).num_minutes();
        let en_saisie = s.remplacant_user_id.unwrap_or(s.prof_user_id);
        let classe = s.classe_id.ok_or_else(invalide)?;
        let libres_classe = self
            .edt_plages_libres(
                a,
                crate::edt_ports::RessourcePlan::Classe(classe),
                du,
                au,
                duree,
            )
            .await?;
        let libres_prof = self
            .edt_plages_libres(
                a,
                crate::edt_ports::RessourcePlan::Intervenant(en_saisie),
                du,
                au,
                duree,
            )
            .await?;
        let libres_lieu = match s.lieu_id {
            Some(l) => Some(
                self.edt_plages_libres(a, crate::edt_ports::RessourcePlan::Lieu(l), du, au, duree)
                    .await?,
            ),
            None => None,
        };
        // Clé historique (créneau, jour) : seuls les jours où le cours a encore un créneau sans séance peuvent
        // recevoir la séance de rattrapage (levée au lot T11, avec les projections historiques).
        let jours_possibles: std::collections::BTreeSet<NaiveDate> = match s.cours_id {
            Some(cours) => sqlx::query_scalar::<_, NaiveDate>(
                "SELECT d::date FROM generate_series($3::date, $4::date, interval '1 day') d
                  WHERE EXISTS (SELECT 1 FROM ent_edt_creneau c
                                 WHERE c.etablissement_id = $1 AND c.cours_id = $2
                                   AND NOT EXISTS (SELECT 1 FROM ent_edt_seance x
                                                    WHERE x.etablissement_id = c.etablissement_id
                                                      AND x.creneau_id = c.id AND x.jour = d::date))",
            )
            .bind(etab)
            .bind(cours)
            .bind(du)
            .bind(au)
            .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?
            .into_iter()
            .collect(),
            None => Default::default(),
        };
        let mut out = Vec::new();
        for pc in libres_classe
            .iter()
            .filter(|p| jours_possibles.contains(&p.jour))
        {
            for pp in libres_prof.iter().filter(|p| p.jour == pc.jour) {
                let (mut d, mut f) = (pc.debut.max(pp.debut), pc.fin.min(pp.fin));
                if let Some(ll) = &libres_lieu {
                    // Le lieu doit couvrir la plage ; sinon la plus longue intersection avec ses plages libres.
                    let Some((d2, f2)) = ll
                        .iter()
                        .filter(|pl| pl.jour == pc.jour)
                        .map(|pl| (d.max(pl.debut), f.min(pl.fin)))
                        .find(|(d2, f2)| *f2 > *d2 && (*f2 - *d2).num_minutes() >= duree)
                    else {
                        continue;
                    };
                    d = d2;
                    f = f2;
                }
                if f > d && (f - d).num_minutes() >= duree {
                    out.push(PlageRattrapage {
                        jour: pc.jour,
                        debut: d,
                        fin: f,
                    });
                }
            }
        }
        out.sort_by_key(|p| (p.jour, p.debut));
        out.dedup();
        out.truncate(30);
        Ok(out)
    }

    /// EDT-176 : les séances à venir qui n'ont personne en saisie, pour les traiter (remplacement, annulation).
    pub async fn edt_seances_non_assurees(
        &self,
        a: &Acteur<'_>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceNonAssuree>, EntDbError> {
        self.exiger_edt(a, Action::EdtModificationsLire).await?;
        if au < du || (au - du).num_days() > 62 {
            return Err(invalide());
        }
        let etab = a.etablissement_id;
        sqlx::query_as::<_, SeanceNonAssuree>(
            "SELECT s.id AS seance_id, s.jour, s.heure_debut, s.heure_fin, s.cours_id, s.classe,
                    CASE WHEN s.etat = 'non_assuree' THEN 'non_assuree' ELSE 'cours_vacant' END AS raison
               FROM ent_edt_seance s
              WHERE s.etablissement_id = $1 AND s.jour BETWEEN $2 AND $3
                AND (s.fin_utc IS NULL OR s.fin_utc > now())
                AND (s.etat = 'non_assuree'
                     OR (s.etat IN ('prevue', 'modifiee', 'ajoutee') AND s.remplacant_user_id IS NULL
                         AND s.cours_id IN (SELECT v.cours_id FROM ent_cours_vacants(s.etablissement_id, s.jour) v)))
              ORDER BY s.jour, s.heure_debut, s.id",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)
    }
}
