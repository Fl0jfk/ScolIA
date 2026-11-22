//! Socle 20, lot C8 (1/2) : **tableau de tenue, visa figé, observations, chronologie du cours**
//! (`20_CAHIER_DE_TEXTES.md` §14.1, R-C17, CC-58).
//!
//! * Le **visa** enregistre la référence figée (objet, version) de chaque entrée et travail publiés de la période ; il ne se
//!   réécrit jamais (déclencheur `ent_cdt_visa_fige`). Une modification postérieure est **tracée et visible** : « modifié après
//!   visa », avec le texte vu et le texte actuel, jamais silencieuse.
//! * L'**observation** est adressée à un cours (ses intervenants à la date) ou à un professeur ; le professeur y répond. Un visa
//!   de classe n'a pas d'observation (elle n'aurait pas de destinataire précis). Le visa lui-même est visible des intervenants ;
//!   jamais des familles ni des élèves.
//! * Aucune donnée **structurée** d'élève ne passe par ce module (ni identifiant, ni remise, ni retour, ni variante) ; un nom
//!   d'élève écrit par un professeur dans un texte libre (contenu, consigne) y serait lu tel quel : la saisie l'en avertit.
//! * Ordre des contrôles : personnel (droit du rôle), puis droit de portée, puis existence.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::types;
use ent_types::cahier_c8::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::cahier::publier_cdt;
use crate::cahier_c2::{conflit, invalide, texte_borne};
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Plus longue période d'un tableau, d'un visa ou d'un export (jours).
pub(crate) const PERIODE_MAX_JOURS: i64 = 400;
/// Visas rendus au plus (après le filtre de visibilité : F5, la limite ne s'applique jamais avant le filtre).
const MAX_VISAS: usize = 200;
/// Visas examinés au plus avant filtrage (garde-fou de coût).
const MAX_VISAS_ANALYSES: i64 = 5000;
const MAX_ELEMENTS_DETAIL: i64 = 2000;

pub(crate) fn periode_valide(du: NaiveDate, au: NaiveDate) -> Result<(), EntDbError> {
    if au < du {
        return Err(invalide("la fin de la période précède son début"));
    }
    if (au - du).num_days() > PERIODE_MAX_JOURS {
        return Err(invalide("période trop longue (400 jours au plus)"));
    }
    Ok(())
}

/// Élément le plus récemment figé par un visa, pour chaque objet : base de « modifié après visa ».
const DERNIER_FIGE: &str =
    "SELECT DISTINCT ON (ve.objet, ve.objet_id) ve.objet, ve.objet_id, ve.version AS version_visee, v.vise_le
       FROM ent_cdt_visa_element ve
       JOIN ent_cdt_visa v ON v.etablissement_id = ve.etablissement_id AND v.id = ve.visa_id
      WHERE ve.etablissement_id = $1
      ORDER BY ve.objet, ve.objet_id, v.vise_le DESC, v.id DESC";

/// Jour de l'établissement dans la transaction de l'appelant (jamais le jour UTC) : M6, pas de seconde transaction sur le pool.
pub(crate) async fn jour_tx(tx: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_aujourdhui_etab($1)")
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)
}

/// M7 : fragment SQL « objet `o` (objet, id, cours_id, jour) absent du dernier visa qui couvrait son cours et sa date ». Le dernier
/// visa (date de visa) du cours qui contient la date de l'objet fait foi ; un objet publié après ce visa n'y figure pas.
const AJOUTE_DEPUIS_VISA: &str = "EXISTS (
        SELECT 1 FROM ent_cdt_visa lv
         WHERE lv.id = (
             SELECT v.id FROM ent_cdt_visa v
              WHERE v.etablissement_id = $1 AND v.du <= o.jour AND v.au >= o.jour
                AND (v.cours_id = o.cours_id
                     OR (v.cible = 'classe' AND v.classe_id = (SELECT c.classe_id FROM ent_cours c WHERE c.etablissement_id = $1 AND c.id = o.cours_id))
                     OR (v.cible = 'agent' AND ent_prof_sur_cours($1, o.cours_id, v.prof_user_id, o.jour, false)))
              ORDER BY v.vise_le DESC, v.id DESC LIMIT 1)
           AND lv.etablissement_id = $1
           AND NOT EXISTS (SELECT 1 FROM ent_cdt_visa_element ve
                            WHERE ve.etablissement_id = $1 AND ve.visa_id = lv.id AND ve.objet = o.objet AND ve.objet_id = o.id))";

impl PgEnt {
    /// Direction : le compte porte `action` à portée d'établissement. Sinon refus (ordre : personnel, droit).
    pub(crate) async fn c8_exiger_etab(
        &self,
        a: &Acteur<'_>,
        action: Action,
    ) -> Result<(), EntDbError> {
        if self.cdt_droit_etab(a, action).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Le compte est-il intervenant (toute nature, délai de grâce de lecture compris) du cours à la date de l'action ?
    ///
    /// M6 : s'exécute dans la transaction de l'appelant (`tx`), jamais dans une seconde transaction prise sur le pool.
    pub(crate) async fn c8_intervenant(
        &self,
        a: &Acteur<'_>,
        tx: &mut PgConnection,
        cours: Uuid,
    ) -> Result<bool, EntDbError> {
        let jour = jour_tx(tx, a.etablissement_id).await?;
        let (ok,): (bool,) = sqlx::query_as("SELECT ent_prof_sur_cours($1, $2, $3, $4, false)")
            .bind(a.etablissement_id)
            .bind(cours)
            .bind(a.user_id)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        Ok(ok)
    }

    /// Agent du compte (professeur), s'il en a un.
    pub(crate) async fn c8_agent_du_compte(
        &self,
        a: &Acteur<'_>,
        tx: &mut PgConnection,
    ) -> Result<Option<Uuid>, EntDbError> {
        sqlx::query_scalar("SELECT ent_cdt_agent_du_compte($1, $2)")
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)
    }

    // ───────────────────────────── CDT-210 : tableau de tenue ─────────────────────────────

    /// CDT-210 : tableau de tenue des cahiers (direction, `cahier.viser`). Agrégats par cours, sans classement public ni nom
    /// d'élève ; les intervenants sont des noms d'affichage.
    pub async fn cdt_c8_tenue(
        &self,
        a: &Acteur<'_>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<TenueDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierViser).await?;
        periode_valide(du, au)?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        #[allow(clippy::type_complexity)]
        let lignes: Vec<(Uuid, String, String, i64, i64, Option<NaiveDate>, i64)> = sqlx::query_as(
            "WITH s AS (
                SELECT ent_cdt_cours_de_seance(x.etablissement_id, x.id) AS cours_id, x.id AS seance_id, x.jour
                  FROM ent_edt_seance x
                 WHERE x.etablissement_id = $1 AND x.jour BETWEEN $2 AND $3
                   AND x.jour <= ent_aujourdhui_etab($1, now()) AND x.statut <> 'annulee'),
             pass AS (SELECT cours_id, count(*) AS n FROM s WHERE cours_id IS NOT NULL GROUP BY cours_id),
             ren AS (SELECT s.cours_id, count(DISTINCT s.seance_id) AS n, max(s.jour) AS dernier
                       FROM s JOIN ent_cdt_entree e ON e.etablissement_id = $1 AND e.seance_id = s.seance_id
                                                   AND e.cours_id = s.cours_id AND e.etat = 'publiee'
                      GROUP BY s.cours_id),
             tr AS (SELECT cours_id, count(*) AS n FROM ent_cdt_travail
                     WHERE etablissement_id = $1 AND etat = 'publie' AND donne_le BETWEEN $2 AND $3 GROUP BY cours_id)
             SELECT c.id, c.libelle, COALESCE(cl.code, g.code, ''),
                    COALESCE(pass.n, 0), COALESCE(ren.n, 0), ren.dernier, COALESCE(tr.n, 0)
               FROM ent_cours c
               LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
               LEFT JOIN ent_groupe g ON g.etablissement_id = c.etablissement_id AND g.id = c.groupe_id
               LEFT JOIN pass ON pass.cours_id = c.id
               LEFT JOIN ren ON ren.cours_id = c.id
               LEFT JOIN tr ON tr.cours_id = c.id
              WHERE c.etablissement_id = $1 AND (COALESCE(pass.n, 0) > 0 OR COALESCE(tr.n, 0) > 0 OR COALESCE(ren.n, 0) > 0)
              ORDER BY c.libelle, c.id",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let visas: Vec<(Uuid, DateTime<Utc>)> = sqlx::query_as(
            "SELECT c.id, max(v.vise_le)
               FROM ent_cours c
               JOIN ent_cdt_visa v ON v.etablissement_id = c.etablissement_id
                AND (v.cours_id = c.id OR (v.cible = 'classe' AND v.classe_id = c.classe_id)
                     OR EXISTS (SELECT 1 FROM ent_cdt_visa_element ve
                                  LEFT JOIN ent_cdt_entree e ON ve.objet = 'entree' AND e.id = ve.objet_id
                                  LEFT JOIN ent_cdt_travail t ON ve.objet = 'travail' AND t.id = ve.objet_id
                                 WHERE ve.etablissement_id = v.etablissement_id AND ve.visa_id = v.id
                                   AND COALESCE(e.cours_id, t.cours_id) = c.id))
              WHERE c.etablissement_id = $1 GROUP BY c.id",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let modifies: Vec<(Uuid, i64)> = sqlx::query_as(&format!(
            "WITH d AS ({DERNIER_FIGE})
             SELECT cours_id, count(*) FROM (
                SELECT e.cours_id FROM d JOIN ent_cdt_entree e ON e.etablissement_id = $1 AND d.objet = 'entree' AND e.id = d.objet_id AND e.version > d.version_visee
                UNION ALL
                SELECT t.cours_id FROM d JOIN ent_cdt_travail t ON t.etablissement_id = $1 AND d.objet = 'travail' AND t.id = d.objet_id AND t.version > d.version_visee
             ) x GROUP BY cours_id"
        ))
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let ajoutes: Vec<(Uuid, i64)> = sqlx::query_as(&format!(
            "SELECT o.cours_id, count(*) FROM (
                SELECT 'entree'::text AS objet, e.id, e.cours_id, s.jour
                  FROM ent_cdt_entree e JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
                 WHERE e.etablissement_id = $1 AND e.etat = 'publiee' AND s.jour BETWEEN $2 AND $3
                UNION ALL
                SELECT 'travail', t.id, t.cours_id, t.donne_le
                  FROM ent_cdt_travail t
                 WHERE t.etablissement_id = $1 AND t.etat = 'publie' AND t.donne_le BETWEEN $2 AND $3) o
              WHERE {AJOUTE_DEPUIS_VISA}
              GROUP BY o.cours_id"
        ))
        .bind(etab)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let interv: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT DISTINCT s.cours_id, pe.user_id
               FROM ent_service_enseignement s
               JOIN ent_service_intervenant i ON i.etablissement_id = s.etablissement_id AND i.service_id = s.id
               JOIN ent_agent ag ON ag.etablissement_id = i.etablissement_id AND ag.id = i.agent_id
               JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
              WHERE s.etablissement_id = $1 AND s.cours_id IS NOT NULL AND i.annule_le IS NULL AND pe.user_id IS NOT NULL
                AND i.nature IN ('titulaire', 'co_enseignant', 'suppleant')
                AND i.du <= $3 AND (i.au IS NULL OR i.au >= $2)",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut noms = std::collections::BTreeMap::new();
        for (_, u) in &interv {
            if !noms.contains_key(u) {
                let (n,): (String,) = sqlx::query_as("SELECT ent_cdt_nom_affichage($1, $2)")
                    .bind(etab)
                    .bind(u)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                noms.insert(*u, n);
            }
        }
        let maintenant = Utc::now();
        let seuil = Duration::weeks(SEUIL_SEMAINES_VISA as i64);
        let lignes = lignes
            .into_iter()
            .map(
                |(cours_id, libelle, public, passees, renseignees, derniere, travaux)| {
                    let dernier_visa_le =
                        visas.iter().find(|(c, _)| *c == cours_id).map(|(_, d)| *d);
                    let mut intervenants: Vec<String> = interv
                        .iter()
                        .filter(|(c, _)| *c == cours_id)
                        .filter_map(|(_, u)| noms.get(u).cloned())
                        .collect();
                    intervenants.sort();
                    intervenants.dedup();
                    TenueLigneDto {
                        cours_id,
                        libelle,
                        public,
                        intervenants,
                        seances_passees: passees,
                        seances_renseignees: renseignees,
                        taux_pct: (passees > 0)
                            .then(|| ((renseignees * 100) / passees).min(100) as i32),
                        travaux_donnes: travaux,
                        derniere_entree: derniere,
                        dernier_visa_le,
                        modifies_apres_visa: modifies
                            .iter()
                            .find(|(c, _)| *c == cours_id)
                            .map_or(0, |(_, n)| *n),
                        ajoutes_apres_visa: ajoutes
                            .iter()
                            .find(|(c, _)| *c == cours_id)
                            .map_or(0, |(_, n)| *n),
                        a_viser: dernier_visa_le.is_none_or(|d| maintenant - d > seuil),
                    }
                },
            )
            .collect();
        Ok(TenueDto {
            du,
            au,
            seuil_semaines: SEUIL_SEMAINES_VISA,
            lignes,
        })
    }

    // ───────────────────────────── CDT-211 : viser ─────────────────────────────

    /// CDT-10 / CDT-211 : vise une période d'un cours, d'une classe ou d'un professeur et **fige** ce qui est vu.
    pub async fn cdt_c8_viser(
        &self,
        a: &Acteur<'_>,
        b: VisaCahierBody,
    ) -> Result<VisaCahierDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierViser).await?;
        periode_valide(b.du, b.au)?;
        let observation = match b.observation.as_deref() {
            Some(t) if !t.trim().is_empty() => Some(texte_borne(
                t,
                4000,
                true,
                "observation vide ou trop longue (4 000 caractères au plus)",
            )?),
            _ => None,
        };
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Cible et liste des cours visés.
        let (cours_ids, prof_user): (Vec<Uuid>, Option<Uuid>) = match b.cible.as_str() {
            VISA_COURS => {
                let c = b
                    .cours_id
                    .ok_or_else(|| invalide("cours à viser manquant"))?;
                let (ok,): (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM ent_cours WHERE etablissement_id = $1 AND id = $2)")
                    .bind(etab)
                    .bind(c)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                if !ok {
                    return Err(introuvable());
                }
                (vec![c], None)
            }
            VISA_CLASSE => {
                if observation.is_some() {
                    return Err(invalide("une observation s'adresse à un cours ou à un professeur, pas à une classe entière"));
                }
                let k = b
                    .classe_id
                    .ok_or_else(|| invalide("classe à viser manquante"))?;
                let (ok,): (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM ent_classe WHERE etablissement_id = $1 AND id = $2)")
                    .bind(etab)
                    .bind(k)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                if !ok {
                    return Err(introuvable());
                }
                let l: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_cours WHERE etablissement_id = $1 AND classe_id = $2 ORDER BY id")
                    .bind(etab)
                    .bind(k)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(db)?;
                (l.into_iter().map(|(i,)| i).collect(), None)
            }
            VISA_AGENT => {
                let ag = b
                    .agent_id
                    .ok_or_else(|| invalide("professeur à viser manquant"))?;
                let user: Option<(Option<Uuid>,)> = sqlx::query_as(
                    "SELECT pe.user_id FROM ent_agent ag JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
                      WHERE ag.etablissement_id = $1 AND ag.id = $2",
                )
                .bind(etab)
                .bind(ag)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                let Some((Some(user),)) = user else {
                    return Err(introuvable());
                };
                let l = cours_de_agent(&mut tx, etab, ag, user, b.du, b.au).await?;
                (l, Some(user))
            }
            _ => return Err(invalide("cible du visa : cours, classe ou agent")),
        };
        let id = Uuid::new_v4();
        let (nb_e,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_cdt_entree e JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
              WHERE e.etablissement_id = $1 AND e.cours_id = ANY($2) AND e.etat = 'publiee' AND s.jour BETWEEN $3 AND $4",
        )
        .bind(etab)
        .bind(&cours_ids)
        .bind(b.du)
        .bind(b.au)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let (nb_t,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_cdt_travail WHERE etablissement_id = $1 AND cours_id = ANY($2) AND etat = 'publie' AND donne_le BETWEEN $3 AND $4",
        )
        .bind(etab)
        .bind(&cours_ids)
        .bind(b.du)
        .bind(b.au)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let nb = (nb_e + nb_t) as i32;
        if nb == 0 && !b.confirmer_vide {
            return Err(conflit("aucune entrée ni aucun travail publié sur la période : confirmez le visa d'un cahier vide"));
        }
        let (classe, cours, agent) = (
            b.classe_id.filter(|_| b.cible == VISA_CLASSE),
            b.cours_id.filter(|_| b.cible == VISA_COURS),
            b.agent_id.filter(|_| b.cible == VISA_AGENT),
        );
        sqlx::query(
            "INSERT INTO ent_cdt_visa (id, etablissement_id, cible, classe_id, cours_id, agent_id, prof_user_id, du, au, commentaire,
                    vise_par_user_id, avec_observation, nb_elements)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, '', $10, $11, $12)",
        )
        .bind(id)
        .bind(etab)
        .bind(&b.cible)
        .bind(classe)
        .bind(cours)
        .bind(agent)
        .bind(prof_user)
        .bind(b.du)
        .bind(b.au)
        .bind(a.user_id)
        .bind(observation.is_some())
        .bind(nb)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_cdt_visa_element (id, etablissement_id, visa_id, objet, objet_id, version)
             SELECT gen_random_uuid(), $1, $2, 'entree', e.id, e.version
               FROM ent_cdt_entree e JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
              WHERE e.etablissement_id = $1 AND e.cours_id = ANY($3) AND e.etat = 'publiee' AND s.jour BETWEEN $4 AND $5
             UNION ALL
             SELECT gen_random_uuid(), $1, $2, 'travail', t.id, t.version
               FROM ent_cdt_travail t
              WHERE t.etablissement_id = $1 AND t.cours_id = ANY($3) AND t.etat = 'publie' AND t.donne_le BETWEEN $4 AND $5",
        )
        .bind(etab)
        .bind(id)
        .bind(&cours_ids)
        .bind(b.du)
        .bind(b.au)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(t) = &observation {
            inserer_observation(&mut tx, etab, id, t, a.user_id).await?;
        }
        publier_cdt(&mut tx, a, types::CAHIER_VISE, id, None,
            json!({ "cible": b.cible, "du": b.du.to_string(), "au": b.au.to_string(), "elements": nb, "avec_observation": observation.is_some() })).await?;
        let dto = visa_dto(&mut tx, etab, id, true, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// CDT-212 : ajoute une observation à un visa de cours ou de professeur (direction).
    pub async fn cdt_c8_observer(
        &self,
        a: &Acteur<'_>,
        visa_id: Uuid,
        b: ObservationBody,
    ) -> Result<VisaCahierDto, EntDbError> {
        self.c8_exiger_etab(a, Action::CahierViser).await?;
        let texte = texte_borne(
            &b.texte,
            4000,
            true,
            "observation vide ou trop longue (4 000 caractères au plus)",
        )?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let cible: Option<(String,)> = sqlx::query_as(
            "SELECT cible FROM ent_cdt_visa WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(visa_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((cible,)) = cible else {
            return Err(introuvable());
        };
        if cible == VISA_CLASSE {
            return Err(invalide(
                "une observation s'adresse à un cours ou à un professeur, pas à une classe entière",
            ));
        }
        inserer_observation(&mut tx, etab, visa_id, &texte, a.user_id).await?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_OBSERVATION,
            visa_id,
            None,
            json!({ "visa_id": visa_id }),
        )
        .await?;
        let dto = visa_dto(&mut tx, etab, visa_id, true, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// CDT-213 : le professeur (ou un intervenant du cours) répond à une observation.
    pub async fn cdt_c8_repondre(
        &self,
        a: &Acteur<'_>,
        observation_id: Uuid,
        b: ObservationBody,
    ) -> Result<VisaCahierDto, EntDbError> {
        if !self.cdt_droit_quelconque(a, Action::CahierEcrire).await? {
            return Err(EntDbError::Forbidden);
        }
        let texte = texte_borne(
            &b.texte,
            4000,
            true,
            "réponse vide ou trop longue (4 000 caractères au plus)",
        )?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let o: Option<(Uuid,)> = sqlx::query_as(
            "SELECT visa_id FROM ent_cdt_observation WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(observation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        // F3 : personnel, droit, puis existence : une observation inconnue et une observation d'un autre cours donnent la même
        // réponse (403), pour ne rien révéler de ce qui existe.
        let Some((visa_id,)) = o else {
            return Err(EntDbError::Forbidden);
        };
        if !self.c8_destinataire(a, &mut tx, visa_id).await? {
            return Err(EntDbError::Forbidden);
        }
        sqlx::query("INSERT INTO ent_cdt_observation_reponse (id, etablissement_id, observation_id, texte, par_user_id) VALUES ($1, $2, $3, $4, $5)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(observation_id)
            .bind(&texte)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_OBSERVATION_REPONSE,
            observation_id,
            None,
            json!({ "visa_id": visa_id }),
        )
        .await?;
        let dto = visa_dto(&mut tx, etab, visa_id, true, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Le compte est-il destinataire des observations de ce visa ? (visa de cours : intervenant du cours ; visa de professeur :
    /// ce professeur ; visa de classe : personne.)
    async fn c8_destinataire(
        &self,
        a: &Acteur<'_>,
        tx: &mut PgConnection,
        visa_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let v: Option<(String, Option<Uuid>, Option<Uuid>)> =
            sqlx::query_as("SELECT cible, cours_id, prof_user_id FROM ent_cdt_visa WHERE etablissement_id = $1 AND id = $2")
                .bind(a.etablissement_id)
                .bind(visa_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        Ok(match v {
            Some((c, Some(cours), _)) if c == VISA_COURS => {
                self.c8_intervenant(a, tx, cours).await?
            }
            Some((c, _, Some(u))) if c == VISA_AGENT => u == a.user_id,
            _ => false,
        })
    }

    /// CDT-211, 214 : visas d'un cours (ou tous, pour la direction). Direction : tous, avec observations. Professeur : les visas
    /// de ses cours et de lui-même avec observations ; « visa seul » (sans observation) pour une classe où il enseigne.
    pub async fn cdt_c8_visas(
        &self,
        a: &Acteur<'_>,
        cours_id: Option<Uuid>,
    ) -> Result<Vec<VisaCahierDto>, EntDbError> {
        let direction = self.cdt_droit_etab(a, Action::CahierViser).await?;
        if !direction && !self.cdt_droit_quelconque(a, Action::CahierEcrire).await? {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let candidats: Vec<(Uuid, String, Option<Uuid>, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT v.id, v.cible, v.cours_id, v.classe_id, v.prof_user_id
               FROM ent_cdt_visa v
              WHERE v.etablissement_id = $1
                AND ($2::uuid IS NULL OR v.cours_id = $2
                     OR (v.cible = 'classe' AND v.classe_id = (SELECT classe_id FROM ent_cours WHERE etablissement_id = $1 AND id = $2))
                     OR EXISTS (SELECT 1 FROM ent_cdt_visa_element ve
                                  LEFT JOIN ent_cdt_entree e ON ve.objet = 'entree' AND e.id = ve.objet_id
                                  LEFT JOIN ent_cdt_travail t ON ve.objet = 'travail' AND t.id = ve.objet_id
                                 WHERE ve.etablissement_id = v.etablissement_id AND ve.visa_id = v.id
                                   AND COALESCE(e.cours_id, t.cours_id) = $2))
              ORDER BY v.vise_le DESC, v.id DESC LIMIT $3",
        )
        .bind(etab)
        .bind(cours_id)
        .bind(MAX_VISAS_ANALYSES)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (id, cible, cours, classe, prof) in candidats {
            let (visible, avec_obs, visibles) = if direction {
                (true, true, None)
            } else {
                match (cible.as_str(), cours, classe, prof) {
                    (VISA_COURS, Some(c), _, _) => {
                        let i = self.c8_intervenant(a, &mut tx, c).await?;
                        (i, i, None)
                    }
                    (VISA_AGENT, _, _, Some(u)) => (u == a.user_id, u == a.user_id, None),
                    (VISA_CLASSE, _, Some(k), _) => {
                        let l = self.c8_cours_visibles_classe(a, &mut tx, k).await?;
                        (!l.is_empty(), false, Some(l))
                    }
                    _ => (false, false, None),
                }
            };
            if visible {
                out.push(visa_dto(&mut tx, etab, id, avec_obs, visibles.as_deref()).await?);
                if out.len() >= MAX_VISAS {
                    break;
                }
            }
        }
        Ok(out)
    }

    /// Cours de la classe où le compte intervient aujourd'hui (F4 : base des comptages d'un visa de classe pour un professeur).
    async fn c8_cours_visibles_classe(
        &self,
        a: &Acteur<'_>,
        tx: &mut PgConnection,
        classe: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let jour = jour_tx(tx, a.etablissement_id).await?;
        let l: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT c.id FROM ent_cours c WHERE c.etablissement_id = $1 AND c.classe_id = $2
                AND ent_prof_sur_cours($1, c.id, $3, $4, false)
              ORDER BY c.id",
        )
        .bind(a.etablissement_id)
        .bind(classe)
        .bind(a.user_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(l.into_iter().map(|(i,)| i).collect())
    }

    /// CDT-214 : détail d'un visa avec la référence figée et les modifications postérieures (texte vu, texte actuel, auteur,
    /// date, motif). Direction : tout. Intervenant : les éléments de ses cours (ou tous, si le visa est le sien).
    pub async fn cdt_c8_visa_detail(
        &self,
        a: &Acteur<'_>,
        visa_id: Uuid,
    ) -> Result<VisaDetailDto, EntDbError> {
        let direction = self.cdt_droit_etab(a, Action::CahierViser).await?;
        if !direction && !self.cdt_droit_quelconque(a, Action::CahierEcrire).await? {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let v: Option<(String, Option<Uuid>, Option<Uuid>, Option<Uuid>)> =
            sqlx::query_as("SELECT cible, cours_id, classe_id, prof_user_id FROM ent_cdt_visa WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(visa_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let Some((cible, cours, classe, prof)) = v else {
            // F3 : hors direction, un visa inconnu ou non lisible donne la même réponse (403) ; l'existence ne se déduit pas.
            return Err(if direction {
                introuvable()
            } else {
                EntDbError::Forbidden
            });
        };
        let mut tous = direction;
        let mut avec_obs = direction;
        let mut visibles: Option<Vec<Uuid>> = None;
        if !direction {
            let (ok, obs) = match (cible.as_str(), cours, classe, prof) {
                (VISA_COURS, Some(c), _, _) => {
                    let i = self.c8_intervenant(a, &mut tx, c).await?;
                    (i, i)
                }
                (VISA_AGENT, _, _, Some(u)) => (u == a.user_id, u == a.user_id),
                (VISA_CLASSE, _, Some(k), _) => {
                    let l = self.c8_cours_visibles_classe(a, &mut tx, k).await?;
                    visibles = Some(l.clone());
                    (!l.is_empty(), false)
                }
                _ => (false, false),
            };
            if !ok {
                return Err(EntDbError::Forbidden);
            }
            avec_obs = obs;
            tous = cible == VISA_AGENT || cible == VISA_COURS;
        }
        let visa = visa_dto(&mut tx, etab, visa_id, avec_obs, visibles.as_deref()).await?;
        #[allow(clippy::type_complexity)]
        let lignes: Vec<(String, Uuid, i32, Uuid, String, Option<NaiveDate>, Option<String>, i32, String)> = sqlx::query_as(
            "SELECT x.objet, x.objet_id, x.version, x.cours_id, x.cours_libelle, x.jour, x.titre, x.version_actuelle, x.etat
               FROM (
                 SELECT ve.objet, ve.objet_id, ve.version, e.cours_id, c.libelle AS cours_libelle, s.jour, e.titre,
                        e.version AS version_actuelle, e.etat
                   FROM ent_cdt_visa_element ve
                   JOIN ent_cdt_entree e ON e.etablissement_id = ve.etablissement_id AND e.id = ve.objet_id
                   JOIN ent_cours c ON c.etablissement_id = e.etablissement_id AND c.id = e.cours_id
                   JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
                  WHERE ve.etablissement_id = $1 AND ve.visa_id = $2 AND ve.objet = 'entree'
                 UNION ALL
                 SELECT ve.objet, ve.objet_id, ve.version, t.cours_id, c.libelle, t.donne_le, t.titre, t.version, t.etat
                   FROM ent_cdt_visa_element ve
                   JOIN ent_cdt_travail t ON t.etablissement_id = ve.etablissement_id AND t.id = ve.objet_id
                   JOIN ent_cours c ON c.etablissement_id = t.etablissement_id AND c.id = t.cours_id
                  WHERE ve.etablissement_id = $1 AND ve.visa_id = $2 AND ve.objet = 'travail') x
              ORDER BY x.cours_libelle, x.jour, x.objet, x.objet_id LIMIT $3",
        )
        .bind(etab)
        .bind(visa_id)
        .bind(MAX_ELEMENTS_DETAIL)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut elements = Vec::new();
        for (objet, objet_id, v_visee, cours_id, cours_libelle, jour_e, titre, v_actuelle, etat) in
            lignes
        {
            if !tous && !self.c8_intervenant(a, &mut tx, cours_id).await? {
                continue;
            }
            let modifie = v_actuelle > v_visee;
            let (mut vise, mut actuel, mut modifications) = (None, None, Vec::new());
            if modifie {
                // L'historique garde les états REMPLACÉS : la ligne de version k est l'état d'avant la modification qui a produit
                // la version k + 1 (date et motif de cette modification) ; la version courante est la ligne vivante.
                let (table, col_obj, texte, vivante) = if objet == "entree" {
                    (
                        "ent_cdt_entree_version",
                        "entree_id",
                        "contenu",
                        "ent_cdt_entree",
                    )
                } else {
                    (
                        "ent_cdt_travail_version",
                        "travail_id",
                        "consigne",
                        "ent_cdt_travail",
                    )
                };
                let historique: Vec<(i32, String, String, Option<Uuid>, DateTime<Utc>, Option<String>)> = sqlx::query_as(&format!(
                    "SELECT version, {texte}, etat, auteur_user_id, cree_le, motif FROM {table}
                      WHERE etablissement_id = $1 AND {col_obj} = $2 AND version >= $3 ORDER BY version"
                ))
                .bind(etab)
                .bind(objet_id)
                .bind(v_visee)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
                let live: (String, String, Option<Uuid>) = sqlx::query_as(&format!(
                    "SELECT {texte}, etat, auteur_user_id FROM {vivante} WHERE etablissement_id = $1 AND id = $2"
                ))
                .bind(etab)
                .bind(objet_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                vise = historique
                    .iter()
                    .find(|h| h.0 == v_visee)
                    .map(|h| h.1.clone());
                actuel = Some(live.0.clone());
                for h in historique
                    .iter()
                    .filter(|h| h.0 >= v_visee && h.0 < v_actuelle)
                {
                    let suivante = h.0 + 1;
                    let (etat_s, auteur_s) = match historique.iter().find(|x| x.0 == suivante) {
                        Some(x) => (x.2.clone(), x.3),
                        None => (live.1.clone(), live.2),
                    };
                    modifications.push(ModificationDto {
                        version: suivante,
                        etat: etat_s,
                        par: nom(&mut tx, etab, auteur_s).await?,
                        le: h.4,
                        motif: h.5.clone(),
                    });
                }
            }
            elements.push(VisaElementDto {
                objet,
                objet_id,
                cours_id,
                cours_libelle,
                jour: jour_e,
                titre,
                version_visee: v_visee,
                version_actuelle: v_actuelle,
                etat_actuel: etat,
                modifie_apres_visa: modifie,
                contenu_vise: vise,
                contenu_actuel: actuel,
                modifications,
            });
        }
        // M7 : publiés de la période du visa, absents de ses éléments (ajoutés depuis). Le périmètre est celui du visa.
        let (cible_v, cours_v, classe_v, prof_v, agent_v, du_v, au_v): (String, Option<Uuid>, Option<Uuid>, Option<Uuid>, Option<Uuid>, NaiveDate, NaiveDate) =
            sqlx::query_as("SELECT cible, cours_id, classe_id, prof_user_id, agent_id, du, au FROM ent_cdt_visa WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(visa_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        let perimetre: Vec<Uuid> = match (cible_v.as_str(), cours_v, classe_v, prof_v, agent_v) {
            (VISA_COURS, Some(c), _, _, _) => vec![c],
            (VISA_CLASSE, _, Some(k), _, _) => {
                let l: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_cours WHERE etablissement_id = $1 AND classe_id = $2 ORDER BY id")
                    .bind(etab)
                    .bind(k)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(db)?;
                l.into_iter().map(|(i,)| i).collect()
            }
            (VISA_AGENT, _, _, Some(u), Some(ag)) => {
                cours_de_agent(&mut tx, etab, ag, u, du_v, au_v).await?
            }
            _ => Vec::new(),
        };
        #[allow(clippy::type_complexity)]
        let candidats: Vec<(String, Uuid, Uuid, String, NaiveDate, Option<String>, i32)> = sqlx::query_as(
            "SELECT o.objet, o.id, o.cours_id, c.libelle, o.jour, o.titre, o.version FROM (
                SELECT 'entree'::text AS objet, e.id, e.cours_id, s.jour, e.titre, e.version
                  FROM ent_cdt_entree e JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
                 WHERE e.etablissement_id = $1 AND e.cours_id = ANY($2) AND e.etat = 'publiee' AND s.jour BETWEEN $3 AND $4
                UNION ALL
                SELECT 'travail', t.id, t.cours_id, t.donne_le, t.titre, t.version
                  FROM ent_cdt_travail t
                 WHERE t.etablissement_id = $1 AND t.cours_id = ANY($2) AND t.etat = 'publie' AND t.donne_le BETWEEN $3 AND $4) o
               JOIN ent_cours c ON c.etablissement_id = $1 AND c.id = o.cours_id
              WHERE NOT EXISTS (SELECT 1 FROM ent_cdt_visa_element ve
                                 WHERE ve.etablissement_id = $1 AND ve.visa_id = $5 AND ve.objet = o.objet AND ve.objet_id = o.id)
              ORDER BY c.libelle, o.jour, o.objet, o.id LIMIT $6",
        )
        .bind(etab)
        .bind(&perimetre)
        .bind(du_v)
        .bind(au_v)
        .bind(visa_id)
        .bind(MAX_ELEMENTS_DETAIL)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut ajoutes_depuis_visa = Vec::new();
        for (objet, objet_id, cours_id, cours_libelle, jour, titre, version) in candidats {
            if !tous && !self.c8_intervenant(a, &mut tx, cours_id).await? {
                continue;
            }
            ajoutes_depuis_visa.push(VisaAjouteDto {
                objet,
                objet_id,
                cours_id,
                cours_libelle,
                jour,
                titre,
                version,
            });
        }
        Ok(VisaDetailDto {
            visa,
            elements,
            ajoutes_depuis_visa,
        })
    }

    // ───────────────────────────── écran 3 : chronologie du cours ─────────────────────────────

    /// CDT-214, écran 3 : chronologie des entrées et travaux **publiés** d'un cours (direction, intervenants), avec les marques
    /// « modifié après visa » ; les observations des visas ne sont servies qu'à leurs destinataires.
    pub async fn cdt_c8_chronologie(
        &self,
        a: &Acteur<'_>,
        cours_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<ChronologieDto, EntDbError> {
        let direction = self.cdt_droit_etab(a, Action::CahierViser).await?;
        if !direction && !self.cdt_droit_quelconque(a, Action::CahierEcrire).await? {
            return Err(EntDbError::Forbidden);
        }
        periode_valide(du, au)?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Ordre des contrôles : personnel, droit, puis existence (F3) : un non-intervenant reçoit 403, qu'il vise un cours
        // existant ou non.
        let intervenant = self.c8_intervenant(a, &mut tx, cours_id).await?;
        if !direction && !intervenant {
            return Err(EntDbError::Forbidden);
        }
        let cours = self
            .c8_cours_ref(&mut tx, etab, cours_id)
            .await?
            .ok_or_else(introuvable)?;
        #[allow(clippy::type_complexity)]
        let lignes: Vec<(String, Uuid, NaiveDate, Option<String>, String, String, i32, Option<Uuid>, DateTime<Utc>, Option<i32>)> = sqlx::query_as(&format!(
            "WITH d AS ({DERNIER_FIGE})
             SELECT x.* , d.version_visee FROM (
                SELECT 'entree'::text AS objet, e.id, s.jour, e.titre, e.contenu, e.etat, e.version, e.auteur_user_id, e.modifie_le
                  FROM ent_cdt_entree e JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
                 WHERE e.etablissement_id = $1 AND e.cours_id = $2 AND e.etat = 'publiee' AND s.jour BETWEEN $3 AND $4
                UNION ALL
                SELECT 'travail', t.id, t.donne_le, t.titre, t.consigne, t.etat, t.version, t.auteur_user_id, t.modifie_le
                  FROM ent_cdt_travail t
                 WHERE t.etablissement_id = $1 AND t.cours_id = $2 AND t.etat = 'publie' AND t.donne_le BETWEEN $3 AND $4) x
             LEFT JOIN d ON d.objet = x.objet AND d.objet_id = x.id
             ORDER BY x.jour, x.objet, x.id"
        ))
        .bind(etab)
        .bind(cours_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // M7 : objets publiés absents du dernier visa qui couvrait leur cours et leur date (ajoutés depuis ce visa).
        let ajoutes: Vec<(String, Uuid)> = sqlx::query_as(&format!(
            "SELECT o.objet, o.id FROM (
                SELECT 'entree'::text AS objet, e.id, e.cours_id, s.jour
                  FROM ent_cdt_entree e JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
                 WHERE e.etablissement_id = $1 AND e.cours_id = $2 AND e.etat = 'publiee' AND s.jour BETWEEN $3 AND $4
                UNION ALL
                SELECT 'travail', t.id, t.cours_id, t.donne_le
                  FROM ent_cdt_travail t
                 WHERE t.etablissement_id = $1 AND t.cours_id = $2 AND t.etat = 'publie' AND t.donne_le BETWEEN $3 AND $4) o
              WHERE {AJOUTE_DEPUIS_VISA}"
        ))
        .bind(etab)
        .bind(cours_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut items = Vec::new();
        for (objet, objet_id, jour, titre, contenu, etat, version, auteur, modifie_le, visee) in
            lignes
        {
            let ajoute = ajoutes.contains(&(objet.clone(), objet_id));
            let nom = match auteur {
                Some(u) => sqlx::query_scalar::<_, String>("SELECT ent_cdt_nom_affichage($1, $2)")
                    .bind(etab)
                    .bind(u)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?,
                None => "Enseignant".to_string(),
            };
            items.push(ChronoItemDto {
                objet,
                objet_id,
                jour,
                titre,
                contenu,
                etat,
                version,
                auteur: nom,
                modifie_le,
                modifie_apres_visa: visee.is_some_and(|v| version > v),
                ajoute_depuis_visa: ajoute,
            });
        }
        drop(tx);
        let visas = self.cdt_c8_visas(a, Some(cours_id)).await?;
        Ok(ChronologieDto {
            cours,
            du,
            au,
            items,
            visas,
        })
    }

    pub(crate) async fn c8_cours_ref(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        cours: Uuid,
    ) -> Result<Option<CoursRefDto>, EntDbError> {
        let r: Option<(Uuid, String, String)> = sqlx::query_as(
            "SELECT c.id, c.libelle, COALESCE(cl.code, g.code, '')
               FROM ent_cours c
               LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
               LEFT JOIN ent_groupe g ON g.etablissement_id = c.etablissement_id AND g.id = c.groupe_id
              WHERE c.etablissement_id = $1 AND c.id = $2",
        )
        .bind(etab)
        .bind(cours)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        Ok(r.map(|(cours_id, libelle, public)| CoursRefDto {
            cours_id,
            libelle,
            public,
        }))
    }
}

/// Cours d'un professeur sur une période : interventions de nature d'enseignement non annulées qui la recoupent, et service sans
/// aucune intervention rattaché à son compte (repli de `ent_prof_sur_cours`).
pub(crate) async fn cours_de_agent(
    tx: &mut PgConnection,
    etab: Uuid,
    agent: Uuid,
    user: Uuid,
    du: NaiveDate,
    au: NaiveDate,
) -> Result<Vec<Uuid>, EntDbError> {
    let l: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT s.cours_id FROM ent_service_enseignement s
          WHERE s.etablissement_id = $1 AND s.cours_id IS NOT NULL
            AND (EXISTS (SELECT 1 FROM ent_service_intervenant i
                          WHERE i.etablissement_id = s.etablissement_id AND i.service_id = s.id AND i.agent_id = $2
                            AND i.annule_le IS NULL AND i.nature IN ('titulaire', 'co_enseignant', 'suppleant', 'intervenant', 'assistant')
                            AND i.du <= $5 AND (i.au IS NULL OR i.au >= $4))
                 OR (s.prof_user_id = $3 AND NOT EXISTS (SELECT 1 FROM ent_service_intervenant z
                                                           WHERE z.etablissement_id = s.etablissement_id AND z.service_id = s.id)))
          ORDER BY s.cours_id",
    )
    .bind(etab)
    .bind(agent)
    .bind(user)
    .bind(du)
    .bind(au)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    Ok(l.into_iter().map(|(i,)| i).collect())
}

async fn inserer_observation(
    tx: &mut PgConnection,
    etab: Uuid,
    visa: Uuid,
    texte: &str,
    par: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query("INSERT INTO ent_cdt_observation (id, etablissement_id, visa_id, texte, par_user_id) VALUES ($1, $2, $3, $4, $5)")
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(visa)
        .bind(texte)
        .bind(par)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    Ok(())
}

/// DTO d'un visa ; `avec_observations` faux pour qui n'est pas destinataire (visa seul).
///
/// F4 : `visibles` limite les comptages (éléments figés, modifiés après visa) aux cours que le lecteur peut voir : un
/// professeur qui lit un visa de classe ne déduit pas de nombres l'activité des cours des collègues.
pub(crate) async fn visa_dto(
    tx: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    avec_observations: bool,
    visibles: Option<&[Uuid]>,
) -> Result<VisaCahierDto, EntDbError> {
    #[allow(clippy::type_complexity)]
    let v: (String, NaiveDate, NaiveDate, DateTime<Utc>, Uuid, i32, Option<Uuid>, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
        "SELECT cible, du, au, vise_le, vise_par_user_id, COALESCE(nb_elements, 0), cours_id, classe_id, prof_user_id
           FROM ent_cdt_visa WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    let (cible, du, au, vise_le, par, nb, cours, classe, prof) = v;
    let libelle_cible: String = match (cible.as_str(), cours, classe, prof) {
        (VISA_COURS, Some(c), _, _) => sqlx::query_scalar(
            "SELECT libelle FROM ent_cours WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(c)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?,
        (VISA_AGENT, _, _, Some(u)) => sqlx::query_scalar("SELECT ent_cdt_nom_affichage($1, $2)")
            .bind(etab)
            .bind(u)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?,
        (_, _, Some(k), _) => {
            let code: String = sqlx::query_scalar(
                "SELECT code FROM ent_classe WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(k)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            format!("Classe {code}")
        }
        _ => "Cahier".to_string(),
    };
    let vise_par: String = sqlx::query_scalar("SELECT ent_cdt_nom_affichage($1, $2)")
        .bind(etab)
        .bind(par)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
    let (nb_visibles, modifies): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE COALESCE(e.version, t.version) > ve.version)
           FROM ent_cdt_visa_element ve
           LEFT JOIN ent_cdt_entree e ON ve.objet = 'entree' AND e.etablissement_id = ve.etablissement_id AND e.id = ve.objet_id
           LEFT JOIN ent_cdt_travail t ON ve.objet = 'travail' AND t.etablissement_id = ve.etablissement_id AND t.id = ve.objet_id
          WHERE ve.etablissement_id = $1 AND ve.visa_id = $2
            AND ($3::uuid[] IS NULL OR COALESCE(e.cours_id, t.cours_id) = ANY($3))",
    )
    .bind(etab)
    .bind(id)
    .bind(visibles)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    let nb = if visibles.is_some() {
        nb_visibles as i32
    } else {
        nb
    };
    let mut observations = Vec::new();
    if avec_observations {
        let obs: Vec<(Uuid, String, Option<Uuid>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, texte, par_user_id, cree_le FROM ent_cdt_observation WHERE etablissement_id = $1 AND visa_id = $2 ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (oid, texte, par, le) in obs {
            let par = nom(tx, etab, par).await?;
            let reps: Vec<(Uuid, String, Option<Uuid>, DateTime<Utc>)> = sqlx::query_as(
                "SELECT id, texte, par_user_id, cree_le FROM ent_cdt_observation_reponse WHERE etablissement_id = $1 AND observation_id = $2 ORDER BY cree_le, id",
            )
            .bind(etab)
            .bind(oid)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let mut reponses = Vec::new();
            for (rid, rtexte, rpar, rle) in reps {
                reponses.push(ReponseDto {
                    id: rid,
                    texte: rtexte,
                    par: nom(tx, etab, rpar).await?,
                    le: rle,
                });
            }
            observations.push(ObservationDto {
                id: oid,
                texte,
                par,
                le,
                reponses,
            });
        }
    }
    Ok(VisaCahierDto {
        id,
        cible,
        libelle_cible,
        du,
        au,
        vise_le,
        vise_par,
        nb_elements: nb,
        vide: nb == 0,
        modifies_apres_visa: modifies,
        observations,
    })
}

pub(crate) async fn nom(
    tx: &mut PgConnection,
    etab: Uuid,
    user: Option<Uuid>,
) -> Result<String, EntDbError> {
    match user {
        Some(u) => sqlx::query_scalar("SELECT ent_cdt_nom_affichage($1, $2)")
            .bind(etab)
            .bind(u)
            .fetch_one(&mut *tx)
            .await
            .map_err(db),
        None => Ok("Enseignant".to_string()),
    }
}
