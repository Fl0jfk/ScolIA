//! Personnels, lot P5 — responsables de groupes datés, équipes calculées, équipes constituées
//! (`06_PERSONNELS.md` §8.7, §8.8, §10.5, §10.6, PRS-66, PRS-70 à 75 ; migrations 356 et 358).
//!
//! | Commande / port                                                | Droit                              |
//! |----------------------------------------------------------------|------------------------------------|
//! | `responsables_groupe_for_actor`                                | `personnels.services.lire`         |
//! | `ajouter_`, `terminer_responsable_groupe_for_actor` (PRS-66)   | `missions.gerer`                   |
//! | `equipe_calculee_for_actor` (PRS-70)                           | personnel : `personnels.services.lire` ; famille et élève : leur propre élève |
//! | `creer_equipe_for_actor` (PRS-71, 73)                          | `equipes.gerer` (équipe éducative : ou professeur principal de l'élève) |
//! | `ajouter_membre_`, `terminer_membre_equipe_for_actor` (PRS-72) | `equipes.gerer` ou coordinateur de l'équipe |
//! | `clore_equipe_for_actor` (PRS-75)                              | `equipes.gerer` ou coordinateur    |
//! | `equipe_destinataires` (PRS-74, port du socle 16)              | port : le socle appelant a contrôlé le droit d'écrire |
//!
//! **Une équipe ne donne aucun droit** (Q-P9) : ni l'équipe calculée ni l'équipe constituée n'est lue par
//! `droits_effectifs`, `perimetre` ou `ent_public_cours_jour` ; seuls les services, missions, responsabilités de
//! groupe et rôles en donnent. Une équipe CONFIDENTIELLE (cellule de veille, équipe autour d'un élève) n'est
//! lisible que de ses membres et de la direction : pour les autres, elle n'existe pas (404).
//!
//! **Équipe pédagogique calculée** (aucune table de membres), à une date : intervenants EN SAISIE ou TITULAIRES des
//! services ouverts dont la cible est la classe ou un groupe qui contient au moins un élève de la classe ;
//! missions actives sur la classe, son niveau ou ses élèves ; pour un groupe, ses responsables datés.
//!
//! Chaque commande est une transaction `TenantTx` (RLS) = fait + journal + événement. Jour métier : celui de
//! l'établissement (`ent_aujourdhui_etab`).

use std::collections::BTreeMap;

use chrono::NaiveDate;
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::personnels_equipes::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::dossier_personnes::aujourdhui_tx;
use crate::outbox::Publier;
use crate::personnels::{
    agent_du_user_tx, agent_tx, conflit, db, introuvable, invalide, journal_tx,
};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

const NATURES_RESPONSABLE: &[&str] = &["responsable", "animateur", "co_animateur"];
const MAX_MEMBRES: usize = 200;

fn erreur_equipe(err: sqlx::Error) -> EntDbError {
    if let sqlx::Error::Database(d) = &err {
        let code = d.code().map(|c| c.to_string()).unwrap_or_default();
        match d.constraint().unwrap_or_default() {
            "ent_groupe_resp_sans_chevauchement" => {
                return conflit("Cet agent a déjà une responsabilité sur ce groupe à ces dates.");
            }
            "ent_equipe_membre_sans_chevauchement"
            | "ent_equipe_membre_personne_sans_chevauchement" => {
                return conflit("Cette personne est déjà membre de l'équipe à ces dates.");
            }
            _ => {}
        }
        if code == "23503" {
            return introuvable();
        }
        if code == "23514" {
            return invalide("Dates ou données d'équipe incohérentes.");
        }
    }
    db(err)
}

async fn publier_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregate_id: Uuid,
    payload: serde_json::Value,
    actor: Uuid,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Referentiel,
        aggregate_id,
        eleve_id: None,
        payload,
        actor_user_id: Some(actor.to_string()),
    })
    .await?;
    Ok(())
}

/// L'agent a-t-il un engagement courant à `jour` ? (un responsable ou un membre d'équipe est en poste.)
async fn en_poste_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_agent_engagement e
                         WHERE e.etablissement_id = $1 AND e.agent_id = $2 AND e.annule_le IS NULL
                           AND e.du <= $3 AND (e.au IS NULL OR e.au >= $3))",
    )
    .bind(etab)
    .bind(agent)
    .bind(jour)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(ok)
}

// ---------------------------------------------------------------------------------------------
// PRS-66 : responsables de groupes
// ---------------------------------------------------------------------------------------------

type LigneResponsable = (
    Uuid,
    Uuid,
    Uuid,
    Option<String>,
    String,
    NaiveDate,
    Option<NaiveDate>,
);

const SELECT_RESPONSABLE: &str = "SELECT r.id, r.groupe_id, r.agent_id,
        nullif(btrim(coalesce(p.prenom, '') || ' ' || p.nom), '') AS nom, r.nature, r.du, r.au
   FROM ent_groupe_responsable r
   JOIN ent_agent a ON a.etablissement_id = r.etablissement_id AND a.id = r.agent_id
   JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id";

fn dto_responsable(l: LigneResponsable, jour: NaiveDate) -> ResponsableGroupeDto {
    ResponsableGroupeDto {
        id: l.0,
        groupe_id: l.1,
        agent_id: l.2,
        agent_nom: l.3,
        nature: l.4,
        du: l.5,
        au: l.6,
        active: l.5 <= jour && l.6.is_none_or(|a| a >= jour),
    }
}

async fn groupe_existe_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    groupe: Uuid,
) -> Result<(), EntDbError> {
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_groupe WHERE etablissement_id = $1 AND id = $2)",
    )
    .bind(etab)
    .bind(groupe)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    if ok {
        Ok(())
    } else {
        Err(introuvable())
    }
}

impl PgEnt {
    /// Responsables d'un groupe, historique compris (les terminées ne sont pas `active`).
    pub async fn responsables_groupe_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        groupe_id: Uuid,
    ) -> Result<Vec<ResponsableGroupeDto>, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsServicesLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        groupe_existe_tx(&mut tx, etab, groupe_id).await?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let sql = format!(
            "{SELECT_RESPONSABLE} WHERE r.etablissement_id = $1 AND r.groupe_id = $2 ORDER BY r.du, p.nom, r.id"
        );
        let lignes: Vec<LigneResponsable> = sqlx::query_as(&sql)
            .bind(etab)
            .bind(groupe_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(|l| dto_responsable(l, jour))
            .collect())
    }

    /// PRS-66 — désigne un responsable, animateur ou co-animateur datés. Début futur permis, jamais passé
    /// (sauf aujourd'hui) ; l'agent doit être en poste à la date de début.
    pub async fn ajouter_responsable_groupe_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        groupe_id: Uuid,
        b: &AjouterResponsableBody,
    ) -> Result<ResponsableGroupeDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::MissionsGerer)
            .await?;
        if !NATURES_RESPONSABLE.contains(&b.nature.as_str()) {
            return Err(invalide("Nature de responsabilité inconnue."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        groupe_existe_tx(&mut tx, etab, groupe_id).await?;
        agent_tx(&mut tx, etab, b.agent_id, true).await?;
        let du = b.du.unwrap_or(jour);
        if du < jour {
            return Err(invalide(
                "Une responsabilité ne commence pas dans le passé.",
            ));
        }
        if b.au.is_some_and(|a| a < du) {
            return Err(invalide("La fin précède le début."));
        }
        if !en_poste_tx(&mut tx, etab, b.agent_id, du).await? {
            return Err(invalide("L'agent n'a aucun engagement à cette date."));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_groupe_responsable (id, groupe_id, etablissement_id, agent_id, nature, du, au)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(groupe_id)
        .bind(etab)
        .bind(b.agent_id)
        .bind(&b.nature)
        .bind(du)
        .bind(b.au)
        .execute(&mut *tx)
        .await
        .map_err(erreur_equipe)?;
        let payload = json!({ "responsabilite_id": id, "groupe_id": groupe_id, "agent_id": b.agent_id,
                              "nature": b.nature, "du": du, "au": b.au });
        journal_tx(
            &mut tx,
            etab,
            "groupe_responsable_ajoute",
            "Responsable de groupe désigné",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::GROUPE_RESPONSABLE_AJOUTE,
            groupe_id,
            payload,
            actor,
        )
        .await?;
        let sql = format!("{SELECT_RESPONSABLE} WHERE r.etablissement_id = $1 AND r.id = $2");
        let l: LigneResponsable = sqlx::query_as(&sql)
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(dto_responsable(l, jour))
    }

    /// PRS-66 — termine une responsabilité (dernier jour `au`, défaut aujourd'hui). Une responsabilité qui
    /// n'a pas encore commencé est retirée ; le droit de `mes_groupes` s'éteint le soir de `au`.
    pub async fn terminer_responsable_groupe_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        responsabilite_id: Uuid,
        b: &TerminerResponsableBody,
    ) -> Result<ResponsableGroupeDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::MissionsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let (groupe_id, agent_id, du, au): (Uuid, Uuid, NaiveDate, Option<NaiveDate>) =
            sqlx::query_as(
                "SELECT groupe_id, agent_id, du, au FROM ent_groupe_responsable
              WHERE etablissement_id = $1 AND id = $2 AND agent_id IS NOT NULL FOR UPDATE",
            )
            .bind(etab)
            .bind(responsabilite_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        let fin = b.au.unwrap_or(jour);
        if au.is_some_and(|a| a < jour) {
            return Err(conflit("Cette responsabilité est déjà terminée."));
        }
        if fin < du {
            return Err(invalide("La fin précède le début."));
        }
        sqlx::query(
            "UPDATE ent_groupe_responsable SET au = $3, termine_par_user_id = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(responsabilite_id)
        .bind(fin)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(erreur_equipe)?;
        let payload = json!({ "responsabilite_id": responsabilite_id, "groupe_id": groupe_id,
                              "agent_id": agent_id, "au": fin });
        journal_tx(
            &mut tx,
            etab,
            "groupe_responsable_termine",
            "Responsable de groupe terminé",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::GROUPE_RESPONSABLE_TERMINE,
            groupe_id,
            payload,
            actor,
        )
        .await?;
        let sql = format!("{SELECT_RESPONSABLE} WHERE r.etablissement_id = $1 AND r.id = $2");
        let l: LigneResponsable = sqlx::query_as(&sql)
            .bind(etab)
            .bind(responsabilite_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(dto_responsable(l, jour))
    }
}

// ---------------------------------------------------------------------------------------------
// PRS-70 : équipes calculées
// ---------------------------------------------------------------------------------------------

/// Cibles d'un calcul d'équipe : les ensembles lus par les requêtes.
#[derive(Default, Debug)]
struct Contexte {
    classes: Vec<Uuid>,
    groupes: Vec<Uuid>,
    eleves: Vec<Uuid>,
    niveaux: Vec<Uuid>,
    /// Groupes dont on prend les responsables.
    groupes_resp: Vec<Uuid>,
}

async fn eleves_de_classes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    classes: &[Uuid],
    jour: NaiveDate,
) -> Result<Vec<Uuid>, EntDbError> {
    let l: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT a.eleve_id FROM ent_affectation a
          WHERE a.etablissement_id = $1 AND a.classe_id = ANY($2)
            AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
            AND NOT EXISTS (SELECT 1 FROM ent_eleve e WHERE e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
                             AND e.date_sortie IS NOT NULL AND e.date_sortie <= $3)",
    )
    .bind(etab)
    .bind(classes)
    .bind(jour)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(l.into_iter().map(|(e,)| e).collect())
}

async fn groupes_de_eleves_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleves: &[Uuid],
    jour: NaiveDate,
) -> Result<Vec<Uuid>, EntDbError> {
    let l: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT a.groupe_id FROM ent_affectation a
          WHERE a.etablissement_id = $1 AND a.eleve_id = ANY($2) AND a.groupe_id IS NOT NULL
            AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)",
    )
    .bind(etab)
    .bind(eleves)
    .bind(jour)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(l.into_iter().map(|(g,)| g).collect())
}

async fn niveaux_de_classes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    classes: &[Uuid],
) -> Result<Vec<Uuid>, EntDbError> {
    let l: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT niveau_id FROM ent_classe
          WHERE etablissement_id = $1 AND id = ANY($2) AND niveau_id IS NOT NULL",
    )
    .bind(etab)
    .bind(classes)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(l.into_iter().map(|(n,)| n).collect())
}

/// Contexte d'une classe : ses élèves, les groupes qui en contiennent au moins un, son niveau.
async fn contexte_classes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    classes: Vec<Uuid>,
    jour: NaiveDate,
) -> Result<Contexte, EntDbError> {
    let eleves = eleves_de_classes_tx(conn, etab, &classes, jour).await?;
    let groupes = groupes_de_eleves_tx(conn, etab, &eleves, jour).await?;
    let niveaux = niveaux_de_classes_tx(conn, etab, &classes).await?;
    Ok(Contexte {
        classes,
        groupes,
        eleves,
        niveaux,
        groupes_resp: Vec::new(),
    })
}

/// (agent, code de la raison, libellé, référence).
type Raison = (Uuid, String, Option<String>, Option<Uuid>);

async fn raisons_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    c: &Contexte,
) -> Result<Vec<Raison>, EntDbError> {
    let mut out: Vec<Raison> = Vec::new();
    // 1. Intervenants en saisie ou titulaires des services ouverts (ou clos mais dans leur période).
    let interv: Vec<Raison> = sqlx::query_as(
        "SELECT i.agent_id, i.nature, s.matiere_libelle, s.id
           FROM ent_service_enseignement s
           JOIN ent_annee_scolaire y ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
           CROSS JOIN LATERAL ent_service_intervenants_a(s.etablissement_id, s.id, $2) i
          WHERE s.etablissement_id = $1 AND s.etat IN ('ouvert', 'clos')
            AND coalesce(s.du, y.date_debut) <= $2 AND coalesce(s.au, y.date_fin) >= $2
            AND (s.classe_id = ANY($3) OR s.groupe_id = ANY($4))
            AND (i.saisie_effective OR i.nature = 'titulaire')",
    )
    .bind(etab)
    .bind(jour)
    .bind(&c.classes)
    .bind(&c.groupes)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    out.extend(interv);
    // 2. Missions actives sur la classe, le groupe, un élève ou le niveau (PP, CPE référent, AESH, coordinateurs).
    let missions: Vec<Raison> = sqlx::query_as(
        "SELECT DISTINCT m.agent_id, 'mission', t.libelle, m.id
           FROM ent_mission m
           JOIN ent_mission_type t ON t.id = m.type_id
           JOIN ent_mission_perimetre p ON p.etablissement_id = m.etablissement_id AND p.mission_id = m.id
          WHERE m.etablissement_id = $1 AND m.annule_le IS NULL AND m.du <= $2 AND (m.au IS NULL OR m.au >= $2)
            AND ((p.nature = 'classe' AND p.cible_id = ANY($3))
              OR (p.nature = 'groupe' AND p.cible_id = ANY($4))
              OR (p.nature = 'eleves' AND p.cible_id = ANY($5))
              OR (p.nature = 'niveau' AND p.cible_id = ANY($6)))",
    )
    .bind(etab)
    .bind(jour)
    .bind(&c.classes)
    .bind(&c.groupes)
    .bind(&c.eleves)
    .bind(&c.niveaux)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    out.extend(missions);
    // 3. Responsables des groupes (équipe d'un groupe).
    if !c.groupes_resp.is_empty() {
        let resp: Vec<Raison> = sqlx::query_as(
            "SELECT r.agent_id, 'responsable_groupe', r.nature, r.id
               FROM ent_groupe_responsable r
              WHERE r.etablissement_id = $1 AND r.groupe_id = ANY($2) AND r.agent_id IS NOT NULL
                AND r.du <= $3 AND (r.au IS NULL OR r.au >= $3)",
        )
        .bind(etab)
        .bind(&c.groupes_resp)
        .bind(jour)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        out.extend(resp);
    }
    Ok(out)
}

/// Équipe d'une discipline : agents de la discipline de poste, services de la matière, coordinateurs.
async fn raisons_discipline_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    matiere: Option<Uuid>,
    code: Option<&str>,
) -> Result<Vec<Raison>, EntDbError> {
    let mut out: Vec<Raison> = sqlx::query_as(
        "SELECT d.agent_id, 'discipline_poste', coalesce(d.code_discipline_poste, 'matière'), d.id
           FROM ent_agent_discipline d
          WHERE d.etablissement_id = $1 AND d.annule_le IS NULL AND d.du <= $2 AND (d.au IS NULL OR d.au >= $2)
            AND (($3::uuid IS NOT NULL AND d.matiere_id = $3) OR ($4::text IS NOT NULL AND d.code_discipline_poste = $4))",
    )
    .bind(etab)
    .bind(jour)
    .bind(matiere)
    .bind(code)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    if matiere.is_some() {
        let serv: Vec<Raison> = sqlx::query_as(
            "SELECT i.agent_id, i.nature, s.matiere_libelle, s.id
               FROM ent_service_enseignement s
               JOIN ent_annee_scolaire y ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
               CROSS JOIN LATERAL ent_service_intervenants_a(s.etablissement_id, s.id, $2) i
              WHERE s.etablissement_id = $1 AND s.etat IN ('ouvert', 'clos') AND s.matiere_id = $3
                AND coalesce(s.du, y.date_debut) <= $2 AND coalesce(s.au, y.date_fin) >= $2
                AND (i.saisie_effective OR i.nature = 'titulaire')",
        )
        .bind(etab)
        .bind(jour)
        .bind(matiere)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        out.extend(serv);
    }
    if let Some(code) = code {
        let coord: Vec<Raison> = sqlx::query_as(
            "SELECT m.agent_id, 'mission', t.libelle, m.id
               FROM ent_mission m
               JOIN ent_mission_type t ON t.id = m.type_id
               JOIN ent_mission_perimetre p ON p.etablissement_id = m.etablissement_id AND p.mission_id = m.id
              WHERE m.etablissement_id = $1 AND m.annule_le IS NULL AND m.du <= $2 AND (m.au IS NULL OR m.au >= $2)
                AND p.nature = 'discipline' AND p.discipline_code = $3",
        )
        .bind(etab)
        .bind(jour)
        .bind(code)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        out.extend(coord);
    }
    Ok(out)
}

/// Noms, fonctions et regroupement par agent. `vue_famille` : désignation prévue pour les familles et les élèves
/// (identité restreinte : civilité et initiale ; nom d'usage choisi par l'agent sinon).
async fn membres_calcules_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    raisons: Vec<Raison>,
    vue_famille: bool,
) -> Result<Vec<MembreCalculeDto>, EntDbError> {
    let mut par_agent: BTreeMap<Uuid, Vec<RaisonEquipeDto>> = BTreeMap::new();
    for (agent, code, libelle, ref_id) in raisons {
        let r = RaisonEquipeDto {
            code,
            libelle,
            ref_id,
        };
        let v = par_agent.entry(agent).or_default();
        if !v.contains(&r) {
            v.push(r);
        }
    }
    if par_agent.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Uuid> = par_agent.keys().copied().collect();
    let lignes: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
        "SELECT a.id,
                CASE WHEN $4 THEN
                       CASE WHEN a.identite_restreinte
                            THEN btrim(coalesce(p.civilite, '') || ' ' || left(p.nom, 1) || '.')
                            ELSE coalesce(nullif(btrim(a.nom_affiche_familles), ''),
                                          btrim(coalesce(p.civilite, '') || ' ' || p.nom)) END
                     ELSE btrim(coalesce(p.prenom, '') || ' ' || p.nom) END,
                (SELECT f.code FROM ent_agent_fonction af JOIN ent_fonction f ON f.id = af.fonction_id
                  WHERE af.etablissement_id = a.etablissement_id AND af.agent_id = a.id AND af.principale
                    AND af.annule_le IS NULL AND af.du <= $3 AND (af.au IS NULL OR af.au >= $3) LIMIT 1)
           FROM ent_agent a
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND a.id = ANY($2)",
    )
    .bind(etab)
    .bind(&ids)
    .bind(jour)
    .bind(vue_famille)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    let mut membres: Vec<MembreCalculeDto> = lignes
        .into_iter()
        .map(|(agent_id, nom, fonction_code)| MembreCalculeDto {
            agent_id,
            nom: Some(nom),
            fonction_code,
            raisons: par_agent.remove(&agent_id).unwrap_or_default(),
        })
        .collect();
    membres.sort_by(|a, b| a.nom.cmp(&b.nom).then(a.agent_id.cmp(&b.agent_id)));
    Ok(membres)
}

/// Calcul d'une équipe pédagogique à `jour` (sans contrôle de droit : l'appelant l'a fait).
async fn equipe_calculee_tx(
    tx: &mut PgConnection,
    etab: Uuid,
    f: &FiltreEquipeCalculee,
    jour: NaiveDate,
    vue_famille: bool,
) -> Result<EquipeCalculeeDto, EntDbError> {
    let cible = f.cible_id;
    let raisons = match f.nature.as_str() {
        "classe" => {
            let id = cible.ok_or_else(|| invalide("Classe requise."))?;
            existe_tx(tx, etab, "ent_classe", id).await?;
            let c = contexte_classes_tx(tx, etab, vec![id], jour).await?;
            raisons_tx(tx, etab, jour, &c).await?
        }
        "groupe" => {
            let id = cible.ok_or_else(|| invalide("Groupe requis."))?;
            groupe_existe_tx(tx, etab, id).await?;
            let c = Contexte {
                groupes: vec![id],
                groupes_resp: vec![id],
                ..Contexte::default()
            };
            raisons_tx(tx, etab, jour, &c).await?
        }
        "niveau" => {
            let id = cible.ok_or_else(|| invalide("Niveau requis."))?;
            existe_tx(tx, etab, "ent_niveau", id).await?;
            let classes: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT c.id FROM ent_classe c
                   JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
                  WHERE c.etablissement_id = $1 AND c.niveau_id = $2 AND y.date_debut <= $3 AND y.date_fin >= $3",
            )
            .bind(etab)
            .bind(id)
            .bind(jour)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let mut c =
                contexte_classes_tx(tx, etab, classes.into_iter().map(|(c,)| c).collect(), jour)
                    .await?;
            c.niveaux = vec![id];
            raisons_tx(tx, etab, jour, &c).await?
        }
        "discipline" => {
            if cible.is_none() && f.discipline_code.is_none() {
                return Err(invalide("Matière ou code de discipline requis."));
            }
            if let Some(m) = cible {
                existe_tx(tx, etab, "ent_matiere", m).await?;
            }
            raisons_discipline_tx(tx, etab, jour, cible, f.discipline_code.as_deref()).await?
        }
        "eleve" => {
            let id = cible.ok_or_else(|| invalide("Élève requis."))?;
            existe_tx(tx, etab, "ent_eleve", id).await?;
            let classes: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT DISTINCT classe_id FROM ent_affectation
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND classe_id IS NOT NULL
                    AND date_debut <= $3 AND (date_fin IS NULL OR date_fin >= $3)",
            )
            .bind(etab)
            .bind(id)
            .bind(jour)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let classes: Vec<Uuid> = classes.into_iter().map(|(c,)| c).collect();
            let groupes = groupes_de_eleves_tx(tx, etab, &[id], jour).await?;
            let niveaux = niveaux_de_classes_tx(tx, etab, &classes).await?;
            let c = Contexte {
                classes,
                groupes,
                eleves: vec![id],
                niveaux,
                groupes_resp: Vec::new(),
            };
            raisons_tx(tx, etab, jour, &c).await?
        }
        _ => {
            return Err(invalide(
                "Nature d'équipe inconnue (classe, groupe, niveau, discipline, eleve).",
            ))
        }
    };
    let membres = membres_calcules_tx(tx, etab, jour, raisons, vue_famille).await?;
    Ok(EquipeCalculeeDto {
        nature: f.nature.clone(),
        cible_id: f.cible_id,
        discipline_code: f.discipline_code.clone(),
        jour,
        membres,
    })
}

/// 404 uniforme : la ligne n'existe pas dans CET établissement.
async fn existe_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    table: &str,
    id: Uuid,
) -> Result<(), EntDbError> {
    let sql = match table {
        "ent_classe" => {
            "SELECT EXISTS (SELECT 1 FROM ent_classe WHERE etablissement_id = $1 AND id = $2)"
        }
        "ent_niveau" => {
            "SELECT EXISTS (SELECT 1 FROM ent_niveau WHERE etablissement_id = $1 AND id = $2)"
        }
        "ent_matiere" => {
            "SELECT EXISTS (SELECT 1 FROM ent_matiere WHERE etablissement_id = $1 AND id = $2)"
        }
        "ent_eleve" => {
            "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)"
        }
        _ => return Err(introuvable()),
    };
    let (ok,): (bool,) = sqlx::query_as(sql)
        .bind(etab)
        .bind(id)
        .fetch_one(conn)
        .await
        .map_err(db)?;
    if ok {
        Ok(())
    } else {
        Err(introuvable())
    }
}

impl PgEnt {
    /// PRS-70 — équipe pédagogique calculée à une date. Le personnel lit les équipes de classe, de groupe, de
    /// niveau et de discipline ; l'équipe d'un ÉLÈVE exige un droit sur cet élève (le professeur : un élève de
    /// son périmètre). Une famille ne lit que l'équipe de SON enfant, un élève que la sienne, désignations
    /// adaptées (identité restreinte). Aucune autre nature n'est offerte aux familles et aux élèves.
    pub async fn equipe_calculee_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        f: &FiltreEquipeCalculee,
    ) -> Result<EquipeCalculeeDto, EntDbError> {
        let r = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let famille = matches!(r, Role::Parent | Role::Eleve);
        if famille {
            if f.nature != "eleve" {
                return Err(EntDbError::Forbidden);
            }
            let eleve = f.cible_id.ok_or_else(|| invalide("Élève requis."))?;
            let lie = if r == Role::Parent {
                self.lien_parent_eleve(etab, actor, eleve).await?.is_some()
            } else {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                let (ok,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_eleve_compte WHERE etablissement_id = $1
                                       AND user_id = $2 AND eleve_id = $3)",
                )
                .bind(etab)
                .bind(actor)
                .bind(eleve)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                ok
            };
            if !lie {
                return Err(introuvable());
            }
        } else {
            self.exiger_personnels(etab, actor, role, Action::PersonnelsServicesLire)
                .await?;
            if f.nature == "eleve" {
                let eleve = f.cible_id.ok_or_else(|| invalide("Élève requis."))?;
                if !self
                    .autoriser_eleve(etab, actor, role, Action::ElevesAnnuaire, eleve)
                    .await?
                {
                    return Err(introuvable());
                }
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = match f.date {
            Some(d) => d,
            None => aujourdhui_tx(&mut tx, etab).await?,
        };
        equipe_calculee_tx(&mut tx, etab, f, jour, famille).await
    }

    /// Port pour les autres socles (conseils de classe, messagerie, agenda) : l'équipe pédagogique d'une classe à
    /// une date, en agents. Aucun contrôle de droit : le socle appelant a déjà contrôlé le sien ; le résultat ne
    /// donne aucun droit.
    pub async fn equipe_pedagogique_classe(
        &self,
        etab: Uuid,
        classe_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = FiltreEquipeCalculee {
            nature: "classe".into(),
            cible_id: Some(classe_id),
            discipline_code: None,
            date: Some(jour),
        };
        let e = equipe_calculee_tx(&mut tx, etab, &f, jour, false).await?;
        Ok(e.membres.into_iter().map(|m| m.agent_id).collect())
    }
}

// ---------------------------------------------------------------------------------------------
// PRS-71 à 75 : équipes constituées
// ---------------------------------------------------------------------------------------------

struct EquipeLigne {
    id: Uuid,
    type_code: String,
    type_libelle: String,
    libelle: String,
    annee_id: Option<Uuid>,
    du: NaiveDate,
    au: Option<NaiveDate>,
    objet_eleve_id: Option<Uuid>,
    liste_diffusion: bool,
    visible_annuaire: bool,
    confidentielle: bool,
    clos: bool,
}

const SELECT_EQUIPE: &str = "SELECT e.id, t.code, t.libelle, t.objet_eleve, e.libelle, e.annee_id, e.du, e.au,
        e.objet_eleve_id, e.liste_diffusion, e.visible_annuaire, e.confidentielle, (e.clos_le IS NOT NULL)
   FROM ent_equipe e JOIN ent_equipe_type t ON t.id = e.type_id
  WHERE e.etablissement_id = $1";

type TupleEquipe = (
    Uuid,
    String,
    String,
    bool,
    String,
    Option<Uuid>,
    NaiveDate,
    Option<NaiveDate>,
    Option<Uuid>,
    bool,
    bool,
    bool,
    bool,
);

impl From<TupleEquipe> for EquipeLigne {
    fn from(t: TupleEquipe) -> Self {
        EquipeLigne {
            id: t.0,
            type_code: t.1,
            type_libelle: t.2,
            libelle: t.4,
            annee_id: t.5,
            du: t.6,
            au: t.7,
            objet_eleve_id: t.8,
            liste_diffusion: t.9,
            visible_annuaire: t.10,
            confidentielle: t.11,
            clos: t.12,
        }
    }
}

async fn equipe_ligne_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<EquipeLigne, EntDbError> {
    let sql = format!(
        "{SELECT_EQUIPE} AND e.id = $2 {}",
        if verrou { "FOR UPDATE OF e" } else { "" }
    );
    sqlx::query_as::<_, TupleEquipe>(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .map(EquipeLigne::from)
        .ok_or_else(introuvable)
}

/// Contexte de l'acteur pour une équipe : direction, agent propre.
struct Qui {
    direction: bool,
    agent: Option<Uuid>,
}

async fn qui_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    role: &str,
) -> Result<Qui, EntDbError> {
    Ok(Qui {
        direction: matches!(Role::parse(role), Some(Role::Admin | Role::Direction)),
        agent: agent_du_user_tx(conn, etab, actor).await?,
    })
}

async fn membre_actif_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    equipe: Uuid,
    agent: Option<Uuid>,
    jour: NaiveDate,
    coordinateur: bool,
) -> Result<bool, EntDbError> {
    let Some(agent) = agent else { return Ok(false) };
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_equipe_membre m
                         WHERE m.etablissement_id = $1 AND m.equipe_id = $2 AND m.agent_id = $3
                           AND m.du <= $4 AND (m.au IS NULL OR m.au >= $4)
                           AND ($5 = false OR m.role_dans_equipe = 'coordinateur'))",
    )
    .bind(etab)
    .bind(equipe)
    .bind(agent)
    .bind(jour)
    .bind(coordinateur)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(ok)
}

/// Une équipe confidentielle n'existe que pour ses membres et la direction (404 pour les autres).
async fn peut_lire_equipe_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    q: &Qui,
    e: &EquipeLigne,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    if !e.confidentielle || q.direction {
        return Ok(true);
    }
    membre_actif_tx(conn, etab, e.id, q.agent, jour, false).await
}

fn etat_equipe(e: &EquipeLigne, jour: NaiveDate) -> &'static str {
    if e.clos {
        "close"
    } else if e.du > jour {
        "a_venir"
    } else if e.au.is_some_and(|a| a < jour) {
        "terminee"
    } else {
        "active"
    }
}

type LigneMembre = (
    Uuid,
    Option<Uuid>,
    Option<Uuid>,
    Option<String>,
    String,
    NaiveDate,
    Option<NaiveDate>,
);

async fn membres_equipe_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    equipe: Uuid,
    jour: NaiveDate,
) -> Result<Vec<MembreEquipeDto>, EntDbError> {
    let l: Vec<LigneMembre> =
        sqlx::query_as(
            "SELECT m.id, m.agent_id, m.personne_id,
                    nullif(btrim(coalesce(p.prenom, '') || ' ' || p.nom), ''), m.role_dans_equipe, m.du, m.au
               FROM ent_equipe_membre m
               LEFT JOIN ent_agent a ON a.etablissement_id = m.etablissement_id AND a.id = m.agent_id
               JOIN ent_personne p ON p.etablissement_id = m.etablissement_id
                                  AND p.id = coalesce(m.personne_id, a.personne_id)
              WHERE m.etablissement_id = $1 AND m.equipe_id = $2
              ORDER BY m.du, p.nom, m.id",
        )
        .bind(etab)
        .bind(equipe)
        .fetch_all(conn)
        .await
        .map_err(db)?;
    Ok(l.into_iter()
        .map(
            |(id, agent_id, personne_id, nom, role_dans_equipe, du, au)| MembreEquipeDto {
                id,
                agent_id,
                personne_id,
                nom,
                role_dans_equipe,
                du,
                au,
                actif: du <= jour && au.is_none_or(|a| a >= jour),
            },
        )
        .collect())
}

async fn equipe_dto_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    e: EquipeLigne,
    jour: NaiveDate,
) -> Result<EquipeDto, EntDbError> {
    let membres = membres_equipe_tx(conn, etab, e.id, jour).await?;
    let etat = etat_equipe(&e, jour).to_string();
    Ok(EquipeDto {
        id: e.id,
        type_code: e.type_code,
        type_libelle: e.type_libelle,
        libelle: e.libelle,
        annee_id: e.annee_id,
        du: e.du,
        au: e.au,
        objet_eleve_id: e.objet_eleve_id,
        liste_diffusion: e.liste_diffusion,
        visible_annuaire: e.visible_annuaire,
        confidentielle: e.confidentielle,
        etat,
        membres,
        ouvre_des_droits: false,
    })
}

/// Ajoute un membre (agent ou personne invitée) ; contrôles de poste et de lien familial.
async fn ajouter_membre_tx(
    tx: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    e: &EquipeLigne,
    m: &NouveauMembreBody,
    jour: NaiveDate,
) -> Result<Uuid, EntDbError> {
    if m.agent_id.is_some() == m.personne_id.is_some() {
        return Err(invalide("Un membre est un agent OU une personne invitée."));
    }
    let du = m.du.unwrap_or_else(|| jour.max(e.du));
    if du < e.du {
        return Err(invalide("Un membre ne commence pas avant l'équipe."));
    }
    let au = m.au.or(e.au);
    if au.is_some_and(|a| a < du) {
        return Err(invalide("La fin précède le début."));
    }
    let role = match (m.agent_id, m.personne_id) {
        (Some(agent), None) => {
            agent_tx(tx, etab, agent, false).await?;
            if !en_poste_tx(tx, etab, agent, du).await? {
                return Err(invalide("L'agent n'a aucun engagement à cette date."));
            }
            match m.role_dans_equipe.as_deref() {
                None | Some("membre") => "membre",
                Some("coordinateur") => "coordinateur",
                _ => return Err(invalide("Rôle d'équipe inconnu (coordinateur, membre).")),
            }
        }
        (None, Some(personne)) => {
            // Une famille n'est invitée que dans une équipe autour d'UN élève dont elle est responsable.
            let Some(eleve) = e.objet_eleve_id else {
                return Err(invalide(
                    "On n'invite une famille que dans une équipe autour d'un élève.",
                ));
            };
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien l
                                 WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.personne_id = $3)",
            )
            .bind(etab)
            .bind(eleve)
            .bind(personne)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
            "invite"
        }
        _ => unreachable!(),
    };
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_equipe_membre (id, etablissement_id, equipe_id, agent_id, personne_id,
                                        role_dans_equipe, du, au, cree_par_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(id)
    .bind(etab)
    .bind(e.id)
    .bind(m.agent_id)
    .bind(m.personne_id)
    .bind(role)
    .bind(du)
    .bind(au)
    .bind(actor)
    .execute(&mut *tx)
    .await
    .map_err(erreur_equipe)?;
    Ok(id)
}

impl PgEnt {
    pub async fn types_equipe_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<TypeEquipeDto>, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsServicesLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l: Vec<(Uuid, String, String, bool)> = sqlx::query_as(
            "SELECT id, code, libelle, objet_eleve FROM ent_equipe_type
              WHERE (etablissement_id IS NULL OR etablissement_id = $1) AND actif ORDER BY libelle",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(l.into_iter()
            .map(|(id, code, libelle, objet_eleve)| TypeEquipeDto {
                id,
                code,
                libelle,
                objet_eleve,
            })
            .collect())
    }

    /// Équipes constituées lisibles par l'acteur (les confidentielles : seulement les siennes, ou la direction).
    pub async fn equipes_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        f: &FiltreEquipes,
    ) -> Result<Vec<EquipeDto>, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsServicesLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let q = qui_tx(&mut tx, etab, actor, role).await?;
        let sql = format!(
            "{SELECT_EQUIPE}
                AND ($2 OR (e.clos_le IS NULL AND (e.au IS NULL OR e.au >= $3)))
              ORDER BY e.du DESC, e.libelle, e.id"
        );
        let lignes: Vec<TupleEquipe> = sqlx::query_as(&sql)
            .bind(etab)
            .bind(f.avec_closes)
            .bind(jour)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut out = Vec::new();
        for t in lignes {
            let e = EquipeLigne::from(t);
            if peut_lire_equipe_tx(&mut tx, etab, &q, &e, jour).await? {
                out.push(equipe_dto_tx(&mut tx, etab, e, jour).await?);
            }
        }
        Ok(out)
    }

    pub async fn equipe_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        equipe_id: Uuid,
    ) -> Result<EquipeDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsServicesLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let q = qui_tx(&mut tx, etab, actor, role).await?;
        let e = equipe_ligne_tx(&mut tx, etab, equipe_id, false).await?;
        if !peut_lire_equipe_tx(&mut tx, etab, &q, &e, jour).await? {
            return Err(introuvable());
        }
        equipe_dto_tx(&mut tx, etab, e, jour).await
    }

    /// PRS-71 et PRS-73 — crée une équipe constituée (et ses premiers membres, dans la même transaction).
    /// Une équipe autour d'un élève est confidentielle d'office, jamais dans l'annuaire, et peut être
    /// constituée par le professeur principal de l'élève (mission active sur sa classe).
    pub async fn creer_equipe_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &CreerEquipeBody,
    ) -> Result<EquipeDto, EntDbError> {
        let libelle = b.libelle.trim();
        if libelle.is_empty() || libelle.chars().count() > 160 {
            return Err(invalide(
                "Libellé d'équipe requis (160 caractères au plus).",
            ));
        }
        if b.membres.len() > MAX_MEMBRES {
            return Err(invalide("Trop de membres."));
        }
        let a_le_droit = self
            .a_action_etablissement(etab, actor, role, Action::EquipesGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let q = qui_tx(&mut tx, etab, actor, role).await?;
        let (type_id, objet_eleve): (Uuid, bool) = sqlx::query_as(
            "SELECT id, objet_eleve FROM ent_equipe_type
              WHERE code = $2 AND actif AND (etablissement_id = $1 OR etablissement_id IS NULL)
              ORDER BY (etablissement_id IS NULL) LIMIT 1",
        )
        .bind(etab)
        .bind(&b.type_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if objet_eleve != b.objet_eleve_id.is_some() {
            return Err(invalide(if objet_eleve {
                "Une équipe éducative a un élève pour objet."
            } else {
                "Seule une équipe éducative a un élève pour objet."
            }));
        }
        if !a_le_droit {
            // Seul le professeur principal de l'élève constitue son équipe éducative (PRS-73).
            let eleve = b.objet_eleve_id.ok_or(EntDbError::Forbidden)?;
            if !self
                .est_pp_de_eleve_tx(&mut tx, etab, q.agent, eleve, jour)
                .await?
            {
                return Err(EntDbError::Forbidden);
            }
        }
        let du = b.du.unwrap_or(jour);
        if du < jour {
            return Err(invalide("Une équipe ne commence pas dans le passé."));
        }
        if b.au.is_some_and(|a| a < du) {
            return Err(invalide("La fin précède le début."));
        }
        let confidentielle = b.confidentielle || b.objet_eleve_id.is_some();
        let visible_annuaire = b.visible_annuaire && b.objet_eleve_id.is_none() && !confidentielle;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_equipe (id, etablissement_id, type_id, libelle, annee_id, du, au, objet_eleve_id,
                                     liste_diffusion, visible_annuaire, confidentielle, cree_par_user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(id)
        .bind(etab)
        .bind(type_id)
        .bind(libelle)
        .bind(b.annee_id)
        .bind(du)
        .bind(b.au)
        .bind(b.objet_eleve_id)
        .bind(b.liste_diffusion)
        .bind(visible_annuaire)
        .bind(confidentielle)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(erreur_equipe)?;
        let e = equipe_ligne_tx(&mut tx, etab, id, false).await?;
        for m in &b.membres {
            ajouter_membre_tx(&mut tx, etab, actor, &e, m, jour).await?;
        }
        // Journal et événement : identifiants, codes, dates, jamais de nom.
        let payload = json!({ "equipe_id": id, "type": b.type_code, "du": du, "au": b.au,
                              "objet_eleve_id": b.objet_eleve_id, "confidentielle": confidentielle,
                              "nb_membres": b.membres.len() });
        journal_tx(
            &mut tx,
            etab,
            "equipe_creee",
            "Équipe constituée",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(&mut tx, etab, types::EQUIPE_CREEE, id, payload, actor).await?;
        let dto = equipe_dto_tx(&mut tx, etab, e, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    async fn est_pp_de_eleve_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        agent: Option<Uuid>,
        eleve: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let Some(agent) = agent else { return Ok(false) };
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
               SELECT 1 FROM ent_mission m
                 JOIN ent_mission_type t ON t.id = m.type_id AND t.code = 'professeur_principal'
                 JOIN ent_mission_perimetre p ON p.etablissement_id = m.etablissement_id AND p.mission_id = m.id
                 JOIN ent_affectation a ON a.etablissement_id = m.etablissement_id AND a.classe_id = p.cible_id
                WHERE m.etablissement_id = $1 AND m.agent_id = $2 AND m.annule_le IS NULL
                  AND m.du <= $4 AND (m.au IS NULL OR m.au >= $4)
                  AND p.nature = 'classe' AND a.eleve_id = $3
                  AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $4))",
        )
        .bind(etab)
        .bind(agent)
        .bind(eleve)
        .bind(jour)
        .fetch_one(conn)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// Contrôle commun à PRS-72 et PRS-75 : l'équipe est lisible (404 sinon) et l'acteur a `equipes.gerer` ou
    /// coordonne l'équipe aujourd'hui.
    async fn exiger_modifier_equipe_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        e: &EquipeLigne,
        jour: NaiveDate,
    ) -> Result<(), EntDbError> {
        let q = qui_tx(conn, etab, actor, role).await?;
        if !peut_lire_equipe_tx(conn, etab, &q, e, jour).await? {
            return Err(introuvable());
        }
        let droit = self
            .a_action_etablissement(etab, actor, role, Action::EquipesGerer)
            .await?;
        if droit || membre_actif_tx(conn, etab, e.id, q.agent, jour, true).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// PRS-72 — ajoute un membre daté.
    pub async fn ajouter_membre_equipe_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        equipe_id: Uuid,
        b: &NouveauMembreBody,
    ) -> Result<EquipeDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let e = equipe_ligne_tx(&mut tx, etab, equipe_id, true).await?;
        self.exiger_modifier_equipe_tx(&mut tx, etab, actor, role, &e, jour)
            .await?;
        if e.clos {
            return Err(conflit("Cette équipe est close."));
        }
        let membre = ajouter_membre_tx(&mut tx, etab, actor, &e, b, jour).await?;
        let payload = json!({ "equipe_id": equipe_id, "membre_id": membre, "operation": "ajout",
                              "agent_id": b.agent_id, "personne_id": b.personne_id });
        journal_tx(
            &mut tx,
            etab,
            "equipe_membres_modifies",
            "Membres d'équipe modifiés",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::EQUIPE_MEMBRES_MODIFIES,
            equipe_id,
            payload,
            actor,
        )
        .await?;
        let dto = equipe_dto_tx(&mut tx, etab, e, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// PRS-72 — termine un membre (dernier jour `au`, défaut aujourd'hui). Un membre qui n'a pas commencé est
    /// retiré. Le dernier coordinateur peut partir : la direction reste compétente (`equipes.gerer`).
    pub async fn terminer_membre_equipe_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        equipe_id: Uuid,
        membre_id: Uuid,
        b: &TerminerMembreBody,
    ) -> Result<EquipeDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let e = equipe_ligne_tx(&mut tx, etab, equipe_id, true).await?;
        self.exiger_modifier_equipe_tx(&mut tx, etab, actor, role, &e, jour)
            .await?;
        let (du, au): (NaiveDate, Option<NaiveDate>) = sqlx::query_as(
            "SELECT du, au FROM ent_equipe_membre
              WHERE etablissement_id = $1 AND equipe_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(equipe_id)
        .bind(membre_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let fin = b.au.unwrap_or(jour);
        if au.is_some_and(|a| a < jour) {
            return Err(conflit("Ce membre est déjà parti."));
        }
        if fin < du {
            if du > jour {
                // N'a pas commencé : retiré.
                sqlx::query(
                    "DELETE FROM ent_equipe_membre WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(membre_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            } else {
                return Err(invalide("La fin précède le début."));
            }
        } else {
            sqlx::query(
                "UPDATE ent_equipe_membre SET au = $3 WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(membre_id)
            .bind(fin)
            .execute(&mut *tx)
            .await
            .map_err(erreur_equipe)?;
        }
        let payload = json!({ "equipe_id": equipe_id, "membre_id": membre_id, "operation": "fin", "au": fin });
        journal_tx(
            &mut tx,
            etab,
            "equipe_membres_modifies",
            "Membres d'équipe modifiés",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::EQUIPE_MEMBRES_MODIFIES,
            equipe_id,
            payload,
            actor,
        )
        .await?;
        let dto = equipe_dto_tx(&mut tx, etab, e, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// PRS-75 — clôt l'équipe : dernier jour `au` (défaut aujourd'hui) ; les membres encore présents partent le
    /// même jour, ceux qui n'ont pas commencé sont retirés. Une équipe close ne se rouvre pas.
    pub async fn clore_equipe_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        equipe_id: Uuid,
        b: &ClorePeriodeBody,
    ) -> Result<EquipeDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let e = equipe_ligne_tx(&mut tx, etab, equipe_id, true).await?;
        self.exiger_modifier_equipe_tx(&mut tx, etab, actor, role, &e, jour)
            .await?;
        if e.clos {
            return Err(conflit("Cette équipe est déjà close."));
        }
        let fin = b.au.unwrap_or(jour);
        if fin < e.du {
            return Err(invalide("La fin précède le début de l'équipe."));
        }
        sqlx::query("DELETE FROM ent_equipe_membre WHERE etablissement_id = $1 AND equipe_id = $2 AND du > $3")
            .bind(etab)
            .bind(equipe_id)
            .bind(fin)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "UPDATE ent_equipe_membre SET au = $3
              WHERE etablissement_id = $1 AND equipe_id = $2 AND (au IS NULL OR au > $3)",
        )
        .bind(etab)
        .bind(equipe_id)
        .bind(fin)
        .execute(&mut *tx)
        .await
        .map_err(erreur_equipe)?;
        sqlx::query(
            "UPDATE ent_equipe SET au = $3, clos_le = now(), clos_par_user_id = $4, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(equipe_id)
        .bind(fin)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(erreur_equipe)?;
        let payload = json!({ "equipe_id": equipe_id, "au": fin });
        journal_tx(
            &mut tx,
            etab,
            "equipe_close",
            "Équipe close",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::EQUIPE_CLOSE,
            equipe_id,
            payload,
            actor,
        )
        .await?;
        let e = equipe_ligne_tx(&mut tx, etab, equipe_id, false).await?;
        let dto = equipe_dto_tx(&mut tx, etab, e, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// PRS-74 — port pour le socle 16 (messagerie) : destinataires d'une équipe CONSTITUÉE à une date, calculés
    /// au moment de l'envoi. Sans les invités (familles) sauf `avec_invites`. Refus si l'équipe n'est pas une
    /// liste de diffusion. Aucun contrôle de droit d'écrire ici : le socle 16 l'a fait ; le résultat ne donne
    /// aucun droit sur les élèves. Équipe d'un autre établissement : 404.
    pub async fn equipe_destinataires(
        &self,
        etab: Uuid,
        equipe_id: Uuid,
        jour: NaiveDate,
        avec_invites: bool,
    ) -> Result<DestinatairesEquipeDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e = equipe_ligne_tx(&mut tx, etab, equipe_id, false).await?;
        if !e.liste_diffusion {
            return Err(invalide("Cette équipe n'est pas une liste de diffusion."));
        }
        let l: Vec<(Option<Uuid>, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT m.agent_id, m.personne_id, pe.user_id
               FROM ent_equipe_membre m
               LEFT JOIN ent_agent a ON a.etablissement_id = m.etablissement_id AND a.id = m.agent_id
               LEFT JOIN ent_personne pe ON pe.etablissement_id = m.etablissement_id AND pe.id = a.personne_id
              WHERE m.etablissement_id = $1 AND m.equipe_id = $2 AND m.du <= $3 AND (m.au IS NULL OR m.au >= $3)
              ORDER BY m.id",
        )
        .bind(etab)
        .bind(equipe_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut d = DestinatairesEquipeDto {
            equipe_id,
            jour,
            agents: Vec::new(),
            comptes: Vec::new(),
            personnes_invitees: Vec::new(),
        };
        for (agent, personne, compte) in l {
            if let Some(a) = agent {
                d.agents.push(a);
                if let Some(c) = compte {
                    d.comptes.push(c);
                }
            } else if let (true, Some(p)) = (avec_invites, personne) {
                d.personnes_invitees.push(p);
            }
        }
        Ok(d)
    }
}
