//! Socle 15, lot A9 — absences collectives : grève, tableau de crise, fermeture
//! (`docs/socles/15_ABSENCES_PERSONNELS.md` §16 ; R-11, R-AP2, R-AP3, R-AP46 ; ABP-253 à 258).
//!
//! **Grève (R-11).** Aucune pré-déclaration dans le second degré : la grève se saisit le jour même ou après. Le matin,
//! l'adjoint **coche les grévistes d'une liste** ; le 15 crée **une absence `greve` par agent**, signalée, **en une
//! seule transaction** (une liste, un lot) ; un agent peut se déclarer gréviste après coup. La nature est sensible
//! (opinion, art. 9) : **aucune liste de grévistes hors du cercle** — direction et secrétariat (`lire_nature`), et
//! l'agent pour lui-même ; ni un collègue, ni la vie scolaire, ni l'accueil, ni la comptabilité, ni une famille.
//!
//! | Commande                                    | Droit (D-017)                                               | ABP   |
//! |---------------------------------------------|-------------------------------------------------------------|-------|
//! | `cocher_grevistes_for_actor`                | `personnels.greve.gerer` (direction)                        | 253   |
//! | `grevistes_du_jour_for_actor`               | `personnels.absences.lire_nature` (direction, secrétariat)  | 253   |
//! | `declarer_greve_for_actor`                  | l'agent, pour lui-même (jamais d'avance)                    | 254   |
//! | `annuler_seances_greve_for_actor`           | `personnels.greve.gerer` + `edt.lot_appliquer`                | 255   |
//! | `tableau_de_crise_for_actor`                | `personnels.greve.gerer`                                    | 256   |
//! | `export_retenue_greve_csv_for_actor`        | `lire_nature` **et** `instruire` (secrétariat, direction)   | 257   |
//! | tâche `neutraliser_absences_fermees`        | système (séances neutralisées par une fermeture)            | 258   |
//!
//! **Garanties.**
//! * Une seule transaction pour le lot : douze grévistes cochés, douze absences ou aucune (un agent déjà absent ou
//!   refusé ne bloque pas les autres : savepoint par agent, signalé dans la réponse).
//! * Le tableau de crise ne porte **que des nombres et des classes** : ni nom, ni nature, ni donnée de santé.
//! * **Retenue (État, 1/30ᵉ par jour, même pour moins d'une journée)** : ScolIA **signale** les jours de grève à l'export,
//!   la décision et le calcul restent hors ScolIA (R-11) ; l'OGEC (retenue proportionnelle, Q-AP10) n'est pas exporté ; un
//!   jour fermé n'est pas compté ; une absence non confirmée (signalée) n'a aucun effet extérieur (R-AP46).
//! * Une fermeture de l'établissement neutralise les absences du jour : leurs impacts passent « sans objet » (recalcul),
//!   elles ne sont plus comptées ce jour-là.
//!
//! **Non fait** : le motif public « mouvement social » d'un lot d'annulations (le port du socle 10 ne l'expose pas : les
//! séances sont annulées sans motif nominatif) ; la bascule en continuité pédagogique à distance (socle 33) ;
//! la notification des familles (socles 8 et 10).

use std::collections::{BTreeMap, HashSet};

use super::rcd::{cellule_csv, evenement_rcd};
use super::suppleance::jour_etab;
use super::*;
use crate::ref_commun::Acteur;
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme};

pub const NATURE_GREVE: &str = "greve";
pub const EVT_FERMETURE_NEUTRALISEE: &str = "absence_personnel.fermeture_neutralisee";
/// Agents cochés au plus dans un lot.
pub const LOT_MAX: usize = 300;
/// Une grève se saisit au plus tard après ce nombre de jours (le jour même ou après, pas des mois plus tard).
pub const RETARD_MAX_JOURS: i64 = 31;

// ─────────────────────────────── logique pure (testée sans base) ───────────────────────────────

/// Classe d'une séance dans le tableau de crise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClasseSeance {
    /// Aucun intervenant absent.
    Normale,
    /// Remplacée, déplacée, ou en continuité encadrée : les élèves ont cours ou sont pris en charge.
    Assuree,
    /// Annulée ou non assurée : des élèves sans cours.
    NonAssuree,
    /// L'intervenant est absent, aucune réponse encore.
    EnAttente,
}

/// Classe une séance d'après son état au socle 10 et les états des impacts qui la touchent.
pub fn classer_seance(etat_seance: &str, impacts: &[String]) -> (bool, ClasseSeance) {
    let actifs: Vec<&str> = impacts
        .iter()
        .map(String::as_str)
        .filter(|e| *e != "sans_objet")
        .collect();
    let touchee = !actifs.is_empty();
    let annulee = matches!(etat_seance, "annulee" | "non_assuree");
    let classe = if actifs.iter().any(|e| {
        matches!(
            *e,
            "remplace" | "autre_cours" | "deplace" | "supplee" | "continuite"
        )
    }) {
        ClasseSeance::Assuree
    } else if annulee || actifs.iter().any(|e| matches!(*e, "annule" | "non_assure")) {
        ClasseSeance::NonAssuree
    } else if touchee {
        ClasseSeance::EnAttente
    } else {
        ClasseSeance::Normale
    };
    (touchee, classe)
}

/// Taux entier d'impact, en pourcentage.
pub fn taux_pct(touchees: u32, seances: u32) -> u32 {
    (touchees * 100).checked_div(seances).unwrap_or(0)
}

/// Une grève se saisit le jour même ou avant, jamais d'avance (R-11), et pas au-delà de `RETARD_MAX_JOURS`.
pub fn jour_de_greve_valide(jour: NaiveDate, aujourdhui: NaiveDate) -> Result<(), &'static str> {
    if jour > aujourdhui {
        return Err(
            "Une grève ne se déclare pas d'avance : saisissez-la le jour même ou après (R-11).",
        );
    }
    if (aujourdhui - jour).num_days() > RETARD_MAX_JOURS {
        return Err("Cette grève est trop ancienne pour être saisie ici.");
    }
    Ok(())
}

/// Les heures sont données ensemble ou pas du tout, dans l'ordre.
pub fn heures_valides(d: Option<NaiveTime>, f: Option<NaiveTime>) -> bool {
    match (d, f) {
        (None, None) => true,
        (Some(d), Some(f)) => f > d,
        _ => false,
    }
}

/// Tableau de retenue : (agent, jour) uniques, triés, hors jours fermés, dans le mois.
pub fn jours_de_retenue(
    plages: &[(Uuid, NaiveDate, NaiveDate)],
    mois: NaiveDate,
    fermes: &HashSet<NaiveDate>,
) -> BTreeMap<Uuid, Vec<NaiveDate>> {
    let fin_mois = super::heures::dernier_du_mois(mois);
    let mut out: BTreeMap<Uuid, std::collections::BTreeSet<NaiveDate>> = BTreeMap::new();
    for (agent, d, f) in plages {
        let mut j = (*d).max(mois);
        let borne = (*f).min(fin_mois);
        while j <= borne {
            if !fermes.contains(&j) {
                out.entry(*agent).or_default().insert(j);
            }
            j += Duration::days(1);
        }
    }
    out.into_iter()
        .map(|(a, v)| (a, v.into_iter().collect()))
        .collect()
}

// ─────────────────────────────── base ───────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
struct GreveLigne {
    absence_id: Uuid,
    agent_id: Uuid,
    nom: String,
    prenom: Option<String>,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    etat: String,
}

fn nom_complet(nom: &str, prenom: &Option<String>) -> String {
    match prenom.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => format!("{p} {nom}"),
        None => nom.to_string(),
    }
}

impl From<GreveLigne> for GrevisteDto {
    fn from(g: GreveLigne) -> Self {
        GrevisteDto {
            absence_id: g.absence_id,
            agent_id: g.agent_id,
            agent: nom_complet(&g.nom, &g.prenom),
            debut: g.debut,
            fin: g.fin,
            etat: g.etat,
        }
    }
}

const GREVISTE_SQL: &str = "SELECT a.id AS absence_id, a.agent_id, p.nom, p.prenom, a.debut, a.fin, a.etat
       FROM ent_absp_absence a
       JOIN ent_absp_nature n ON n.id = a.nature_id AND n.code = 'greve'
       JOIN ent_agent ag ON ag.etablissement_id = a.etablissement_id AND ag.id = a.agent_id
       JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
      WHERE a.etablissement_id = $1 AND a.etat NOT IN ('annulee', 'refusee') AND a.debut < $3 AND a.fin > $2";

async fn grevistes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
) -> Result<Vec<GrevisteDto>, EntDbError> {
    let d0 = instant(&mut *conn, etab, jour, None).await?;
    let d1 = instant_fin(&mut *conn, etab, jour, None).await?;
    let v: Vec<GreveLigne> = sqlx::query_as(&format!(
        "{GREVISTE_SQL} ORDER BY p.nom, p.prenom, a.debut, a.id"
    ))
    .bind(etab)
    .bind(d0)
    .bind(d1)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(v.into_iter().map(Into::into).collect())
}

impl PgEnt {
    async fn exiger_greve_gerer(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Role, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !self
            .a_droit_abs(etab, actor, role, Action::PersonnelsGreveGerer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(r)
    }

    // ───────────────────────────── Cocher, lire, se déclarer (ABP-253, 254) ─────────────────────────────

    /// ABP-253 : coche les grévistes d'une demi-journée. Une absence `greve` par agent, signalée, en **une** transaction.
    pub async fn cocher_grevistes_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &CocherGrevistesBody,
    ) -> Result<CocheGrevistesDto, EntDbError> {
        self.exiger_greve_gerer(etab, actor, role).await?;
        let mut vus = HashSet::new();
        let agents: Vec<Uuid> = body
            .agents
            .iter()
            .copied()
            .filter(|a| vus.insert(*a))
            .collect();
        if agents.is_empty() || agents.len() > LOT_MAX {
            return Err(invalide("Cochez de 1 à 300 agents."));
        }
        if !heures_valides(body.heure_debut, body.heure_fin) {
            return Err(invalide(
                "Donnez les deux heures (début et fin), la fin après le début.",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let aujourdhui = jour_etab(&mut tx, etab).await?;
        jour_de_greve_valide(body.jour, aujourdhui).map_err(conflit)?;
        let nature = nature_par_code(&mut tx, etab, NATURE_GREVE)
            .await?
            .ok_or_else(|| conflit("La nature « grève » est désactivée pour cet établissement."))?;
        let mut crees: Vec<Uuid> = Vec::new();
        let mut deja: Vec<Uuid> = Vec::new();
        let mut refuses: Vec<Uuid> = Vec::new();
        let mut evenements = Vec::new();
        for agent in agents {
            // Un agent déjà absent sur la plage (hors absence annulée ou refusée) n'est pas doublé.
            let plages = self
                .plages_depuis(
                    &mut tx,
                    etab,
                    body.jour,
                    body.jour,
                    body.heure_debut,
                    body.heure_fin,
                    &[],
                )
                .await?;
            let (d, f) = (plages[0].debut, plages[0].fin);
            let existe: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2 AND fusionne_dans_agent_id IS NULL)")
                .bind(etab)
                .bind(agent)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            if !existe {
                refuses.push(agent);
                continue;
            }
            let occupe: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_absp_absence a
                                 JOIN ent_absp_plage p ON p.etablissement_id = a.etablissement_id AND p.absence_id = a.id AND p.retiree_le IS NULL
                                WHERE a.etablissement_id = $1 AND a.agent_id = $2 AND a.etat NOT IN ('annulee', 'refusee')
                                  AND p.debut < $4 AND p.fin > $3)",
            )
            .bind(etab)
            .bind(agent)
            .bind(d)
            .bind(f)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if occupe {
                deja.push(agent);
                continue;
            }
            // Un agent qui échoue (engagement, contrainte) ne fait pas tomber la liste : savepoint par agent.
            sqlx::query("SAVEPOINT greve_agent")
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            let r = self
                .creer_absences_tx(
                    &mut tx,
                    etab,
                    actor,
                    Creation {
                        agent_id: agent,
                        nature: Some(&nature),
                        texte_libre: None,
                        plages,
                        fin_provisoire: false,
                        preference_traitement: None,
                        canal: "collegue",
                        confirmee: false,
                        engagement_filtre: None,
                        zone_qualification: None,
                    },
                )
                .await;
            match r {
                Ok((ids, evs, _)) => {
                    sqlx::query("RELEASE SAVEPOINT greve_agent")
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    crees.extend(ids);
                    evenements.extend(evs);
                }
                Err(_) => {
                    sqlx::query("ROLLBACK TO SAVEPOINT greve_agent")
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    sqlx::query("RELEASE SAVEPOINT greve_agent")
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    refuses.push(agent);
                }
            }
        }
        for id in &crees {
            self.recalculer_impact_tolerant(&mut tx, etab, *id, actor)
                .await;
        }
        let liste = grevistes_tx(&mut tx, etab, body.jour).await?;
        let ids: HashSet<Uuid> = crees.iter().copied().collect();
        let grevistes: Vec<GrevisteDto> = liste
            .into_iter()
            .filter(|g| ids.contains(&g.absence_id))
            .collect();
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evenements);
        Ok(CocheGrevistesDto {
            jour: body.jour,
            grevistes,
            deja_absents: deja,
            refuses,
        })
    }

    /// La liste des grévistes d'un jour : **réservée au cercle** (direction, secrétariat). Un autre profil n'obtient pas
    /// une liste vide : il est refusé (pas d'oracle sur l'existence d'une grève).
    pub async fn grevistes_du_jour_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<GrevisteDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        grevistes_tx(&mut tx, etab, jour).await
    }

    /// ABP-254 : l'agent se déclare gréviste après coup. Jamais d'avance (R-11). Si la direction l'avait déjà coché
    /// (absence signalée), il la **confirme** ; sinon l'absence est créée, déclarée. Jamais deux absences pour la même plage.
    pub async fn declarer_greve_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &DeclarerGreveBody,
    ) -> Result<GrevisteDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !heures_valides(body.heure_debut, body.heure_fin) {
            return Err(invalide(
                "Donnez les deux heures (début et fin), la fin après le début.",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some(moi) = agent_du_user_tx(&mut tx, etab, actor).await? else {
            return Err(EntDbError::Forbidden);
        };
        let aujourdhui = jour_etab(&mut tx, etab).await?;
        jour_de_greve_valide(body.jour, aujourdhui).map_err(conflit)?;
        let plages = self
            .plages_depuis(
                &mut tx,
                etab,
                body.jour,
                body.jour,
                body.heure_debut,
                body.heure_fin,
                &[],
            )
            .await?;
        let (d, f) = (plages[0].debut, plages[0].fin);
        // Déjà cochée par la direction ?
        let existante: Option<(Uuid, String, bool)> = sqlx::query_as(
            "SELECT a.id, a.etat, n.code = 'greve'
               FROM ent_absp_absence a LEFT JOIN ent_absp_nature n ON n.id = a.nature_id
              WHERE a.etablissement_id = $1 AND a.agent_id = $2 AND a.etat NOT IN ('annulee', 'refusee') AND a.debut < $4 AND a.fin > $3
              ORDER BY a.cree_le LIMIT 1",
        )
        .bind(etab)
        .bind(moi)
        .bind(d)
        .bind(f)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let id = match existante {
            Some((id, etat, true)) => {
                if etat == "signalee" {
                    tx.commit().await.map_err(db)?;
                    self.confirmer_absence_for_actor(
                        etab,
                        actor,
                        role,
                        id,
                        &ConfirmerBody {
                            nature_code: Some(NATURE_GREVE.into()),
                        },
                        false,
                    )
                    .await?;
                    let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                    return self.greviste_de(&mut tx, etab, id).await;
                }
                id
            }
            Some((_, _, false)) => {
                return Err(conflit(
                    "Vous êtes déjà absent sur cette plage : la direction qualifie l'absence.",
                ))
            }
            None => {
                let nature = nature_par_code(&mut tx, etab, NATURE_GREVE)
                    .await?
                    .ok_or_else(|| {
                        conflit("La nature « grève » est désactivée pour cet établissement.")
                    })?;
                let (ids, evs, _) = self
                    .creer_absences_tx(
                        &mut tx,
                        etab,
                        actor,
                        Creation {
                            agent_id: moi,
                            nature: Some(&nature),
                            texte_libre: None,
                            plages,
                            fin_provisoire: false,
                            preference_traitement: None,
                            canal: "agent",
                            confirmee: true,
                            engagement_filtre: None,
                            zone_qualification: None,
                        },
                    )
                    .await?;
                let id = ids[0];
                for i in &ids {
                    self.recalculer_impact_tolerant(&mut tx, etab, *i, actor)
                        .await;
                }
                let g = self.greviste_de(&mut tx, etab, id).await?;
                tx.commit().await.map_err(db)?;
                self.diffuser_apres_commit(&evs);
                return Ok(g);
            }
        };
        self.greviste_de(&mut tx, etab, id).await
    }

    async fn greviste_de(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        absence: Uuid,
    ) -> Result<GrevisteDto, EntDbError> {
        let l: GreveLigne = sqlx::query_as(
            "SELECT a.id AS absence_id, a.agent_id, p.nom, p.prenom, a.debut, a.fin, a.etat
               FROM ent_absp_absence a
               JOIN ent_agent ag ON ag.etablissement_id = a.etablissement_id AND ag.id = a.agent_id
               JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
              WHERE a.etablissement_id = $1 AND a.id = $2",
        )
        .bind(etab)
        .bind(absence)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        Ok(l.into())
    }

    // ───────────────────────────── Annulation en lot (ABP-255) ─────────────────────────────

    /// ABP-255 : annule en lot les séances des grévistes d'un jour. Chaque absence cochée est d'abord **confirmée**
    /// (par la direction, au nom de la liste) puis **décidée** (jamais la sienne), puis ses séances à traiter sont annulées
    /// par le port du socle 10 `edt_annuler_seances` (un lot par absence, clé d'idempotence, un seul message par personne). Un échec n'arrête pas les autres : il est compté.
    pub async fn annuler_seances_greve_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<AnnulationGreveDto, EntDbError> {
        self.exiger_greve_gerer(etab, actor, role).await?;
        if !self
            .a_droit_abs(etab, actor, role, Action::EdtLotAppliquer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let liste: Vec<(Uuid, String)> = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let d0 = instant(&mut tx, etab, jour, None).await?;
            let d1 = instant_fin(&mut tx, etab, jour, None).await?;
            sqlx::query_as(
                "SELECT a.id, a.etat FROM ent_absp_absence a JOIN ent_absp_nature n ON n.id = a.nature_id AND n.code = 'greve'
                  WHERE a.etablissement_id = $1 AND a.etat NOT IN ('annulee', 'refusee') AND a.debut < $3 AND a.fin > $2
                  ORDER BY a.cree_le, a.id",
            )
            .bind(etab)
            .bind(d0)
            .bind(d1)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
        };
        let mut bilan = AnnulationGreveDto::default();
        for (id, etat) in liste {
            let mut etat = etat;
            if etat == "signalee" {
                match self
                    .confirmer_absence_for_actor(
                        etab,
                        actor,
                        role,
                        id,
                        &ConfirmerBody {
                            nature_code: Some(NATURE_GREVE.into()),
                        },
                        true,
                    )
                    .await
                {
                    Ok(_) => {
                        bilan.absences_confirmees += 1;
                        etat = "declaree".into();
                    }
                    Err(_) => {
                        bilan.a_revoir += 1;
                        continue;
                    }
                }
            }
            if matches!(etat.as_str(), "declaree" | "en_instruction") {
                match self
                    .valider_absence_for_actor(etab, actor, role, id, &DecisionBody::default())
                    .await
                {
                    Ok(_) => bilan.absences_validees += 1,
                    Err(_) => {
                        bilan.a_revoir += 1;
                        continue;
                    }
                }
            }
            // Séances « à traiter » de cette absence, ce jour-là : un seul lot par absence, par le port du socle 10
            // (`edt_annuler_seances`, clé d'idempotence de la décision) — le 15 n'écrit jamais dans l'emploi du temps.
            let impacts: Vec<(Uuid, Uuid, String)> = {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                sqlx::query_as(
                    "SELECT i.id, i.seance_id, i.etat FROM ent_absp_impact i
                       JOIN ent_edt_seance s ON s.etablissement_id = i.etablissement_id AND s.id = i.seance_id
                      WHERE i.etablissement_id = $1 AND i.absence_id = $2 AND i.type = 'seance' AND i.etat = 'a_traiter' AND s.jour = $3
                      ORDER BY i.debut, i.id",
                )
                .bind(etab)
                .bind(id)
                .bind(jour)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?
            };
            if impacts.is_empty() {
                continue;
            }
            let seances: Vec<Uuid> = impacts.iter().map(|i| i.1).collect();
            let acteur = Acteur {
                etablissement_id: etab,
                user_id: actor,
                role,
            };
            match self
                .edt_annuler_seances(
                    &acteur,
                    &seances,
                    Some(id),
                    Some(format!("greve:{id}:{jour}")),
                )
                .await
            {
                Ok(_) => {
                    let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                    // Les demandes ouvertes n'ont plus d'objet ; chaque séance annulée est consignée (comme A4).
                    for (impact, _, etat) in &impacts {
                        sqlx::query(
                            "UPDATE ent_absp_sollicitation SET etat = 'annulee', annulee_cause = 'levee', repondue_le = now()
                              WHERE etablissement_id = $1 AND impact_id = $2 AND etat = 'envoyee'",
                        )
                        .bind(etab)
                        .bind(impact)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                        journal(
                            &mut tx,
                            etab,
                            id,
                            "seance_annulee",
                            Some("impact"),
                            Some(etat),
                            Some("annule"),
                            actor,
                        )
                        .await?;
                    }
                    tx.commit().await.map_err(db)?;
                    self.recalculer_impact_absence(etab, actor, id).await?;
                    bilan.seances_annulees += impacts.len() as u32;
                }
                Err(_) => bilan.a_revoir += impacts.len() as u32,
            }
        }
        Ok(bilan)
    }

    // ───────────────────────────── Tableau de crise (ABP-256) ─────────────────────────────

    /// ABP-256 : le tableau de crise d'un jour — taux d'impact par heure, classes sans aucun cours sur une demi-journée.
    /// Nombres et classes seulement : ni nom d'absent, ni nature, ni information de santé (jamais diffusée en collectif).
    pub async fn tableau_de_crise_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<TableauCriseDto, EntDbError> {
        self.exiger_greve_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<(String, NaiveTime, NaiveTime, Option<String>, String, Vec<String>)> = sqlx::query_as(
            "SELECT s.classe, s.heure_debut, s.heure_fin, s.demi_journee, s.etat,
                    COALESCE((SELECT array_agg(i.etat) FROM ent_absp_impact i
                               WHERE i.etablissement_id = s.etablissement_id AND i.seance_id = s.id AND i.type = 'seance'),
                             ARRAY[]::text[])
               FROM ent_edt_seance s
              WHERE s.etablissement_id = $1 AND s.jour = $2 AND s.etat NOT IN ('retiree', 'neutralisee')
              ORDER BY s.heure_debut, s.heure_fin, s.classe",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut par_heure: BTreeMap<(NaiveTime, NaiveTime), CriseCreneauDto> = BTreeMap::new();
        let mut par_classe: BTreeMap<(String, String), (u32, u32)> = BTreeMap::new(); // (séances, perdues)
        let (mut total, mut touchees) = (0u32, 0u32);
        for (classe, d, f, demi, etat, impacts) in lignes {
            let (touchee, c) = classer_seance(&etat, &impacts);
            total += 1;
            touchees += u32::from(touchee);
            let e = par_heure.entry((d, f)).or_insert(CriseCreneauDto {
                heure_debut: d,
                heure_fin: f,
                seances: 0,
                touchees: 0,
                remplacees: 0,
                non_assurees: 0,
                en_attente: 0,
                taux_impact_pct: 0,
            });
            e.seances += 1;
            e.touchees += u32::from(touchee);
            match c {
                ClasseSeance::Assuree => e.remplacees += 1,
                ClasseSeance::NonAssuree => e.non_assurees += 1,
                ClasseSeance::EnAttente => e.en_attente += 1,
                ClasseSeance::Normale => {}
            }
            let k = par_classe
                .entry((classe, demi.unwrap_or_else(|| "journee".into())))
                .or_default();
            k.0 += 1;
            k.1 += u32::from(matches!(
                c,
                ClasseSeance::NonAssuree | ClasseSeance::EnAttente
            ));
        }
        let creneaux = par_heure
            .into_values()
            .map(|mut c| {
                c.taux_impact_pct = taux_pct(c.touchees, c.seances);
                c
            })
            .collect();
        let classes_sans_cours = par_classe
            .into_iter()
            .filter(|(_, (n, p))| *n > 0 && n == p)
            .map(|((classe, demi_journee), _)| CriseClasseDto {
                classe,
                demi_journee,
            })
            .collect();
        Ok(TableauCriseDto {
            jour,
            seances: total,
            touchees,
            creneaux,
            classes_sans_cours,
        })
    }

    // ───────────────────────────── Retenue signalée à l'export (ABP-257) ─────────────────────────────

    /// ABP-257 : les jours de grève du mois des agents de l'État, à signaler à l'export (1/30ᵉ par jour, même pour moins
    /// d'une journée : R-11). **ScolIA ne décide ni ne calcule la retenue.** Secrétariat et direction ; l'OGEC n'est pas
    /// exporté (Q-AP10) ; une absence non confirmée n'est pas exportée (R-AP46) ; un jour fermé n'est pas compté.
    pub async fn export_retenue_greve_csv_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        mois: NaiveDate,
    ) -> Result<String, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !(self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
            .await?
            && self
                .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesInstruire)
                .await?)
        {
            return Err(EntDbError::Forbidden);
        }
        use chrono::Datelike;
        if mois.day() != 1 {
            return Err(invalide("Indiquez le premier jour du mois."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d0 = instant(&mut tx, etab, mois, None).await?;
        let d1 = instant(&mut tx, etab, super::heures::mois_suivant(mois), None).await?;
        let rows: Vec<(Uuid, Uuid, String, Option<String>, NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT a.id, a.agent_id, p.nom, p.prenom,
                    ent_aujourdhui_etab($1, pl.debut), ent_aujourdhui_etab($1, pl.fin - interval '1 microsecond')
               FROM ent_absp_absence a
               JOIN ent_absp_nature n ON n.id = a.nature_id AND n.code = 'greve'
               JOIN ent_absp_plage pl ON pl.etablissement_id = a.etablissement_id AND pl.absence_id = a.id AND pl.retiree_le IS NULL
               JOIN ent_agent ag ON ag.etablissement_id = a.etablissement_id AND ag.id = a.agent_id
               JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
              WHERE a.etablissement_id = $1 AND a.employeur = 'etat' AND a.etat NOT IN ('signalee', 'annulee', 'refusee')
                AND pl.debut < $3 AND pl.fin > $2
              ORDER BY p.nom, p.prenom, pl.debut",
        )
        .bind(etab)
        .bind(d0)
        .bind(d1)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let fermes: HashSet<NaiveDate> = sqlx::query_scalar::<_, NaiveDate>(
            "SELECT jour FROM ent_jour_calendrier WHERE etablissement_id = $1 AND jour >= $2 AND jour < $3 AND kind IN ('ferme', 'ferie', 'vacances')",
        )
        .bind(etab)
        .bind(mois)
        .bind(super::heures::mois_suivant(mois))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let mut noms: BTreeMap<Uuid, String> = BTreeMap::new();
        let mut absences: HashSet<Uuid> = HashSet::new();
        let plages: Vec<(Uuid, NaiveDate, NaiveDate)> = rows
            .iter()
            .map(|(abs, agent, nom, prenom, d, f)| {
                noms.insert(*agent, nom_complet(nom, prenom));
                absences.insert(*abs);
                (*agent, *d, *f)
            })
            .collect();
        let jours = jours_de_retenue(&plages, mois, &fermes);
        let mut csv = String::from("mois,agent_id,agent,jour,signal\n");
        for (agent, js) in &jours {
            for j in js {
                csv.push_str(&format!(
                    "{},{},{},{},retenue_a_decider_hors_scolia\n",
                    mois,
                    agent,
                    cellule_csv(noms.get(agent).map_or("", String::as_str)),
                    j
                ));
            }
        }
        // Trace : chaque export consigne les absences qu'il a signalées (ajout seul).
        let mut tous: Vec<Uuid> = absences.into_iter().collect();
        tous.sort();
        for a in tous {
            journal(
                &mut tx,
                etab,
                a,
                "retenue_signalee_export",
                Some("mois"),
                None,
                Some(&mois.to_string()),
                actor,
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(csv)
    }

    // ───────────────────────────── Fermeture (ABP-258) ─────────────────────────────

    /// ABP-258 : une fermeture de l'établissement (socle 5) neutralise les séances du jour (socle 10) ; les absences de
    /// ce jour ne sont plus comptées : leurs impacts sont recalculés et passent « sans objet ». Idempotent. Renvoie le
    /// nombre d'absences recalculées.
    pub(crate) async fn neutraliser_absences_fermees_etab(
        &self,
        etab: Uuid,
    ) -> Result<u32, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let absences: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT DISTINCT i.absence_id, a.saisie_par_user_id
               FROM ent_absp_impact i
               JOIN ent_edt_seance s ON s.etablissement_id = i.etablissement_id AND s.id = i.seance_id AND s.etat = 'neutralisee'
               JOIN ent_jour_calendrier j ON j.etablissement_id = s.etablissement_id AND j.jour = s.jour AND j.kind = 'ferme'
               JOIN ent_absp_absence a ON a.etablissement_id = i.etablissement_id AND a.id = i.absence_id
              WHERE i.etablissement_id = $1 AND i.type = 'seance' AND i.etat <> 'sans_objet'
              LIMIT 500",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut n = 0;
        for (absence, auteur) in absences {
            let (_, _) = self
                .recalculer_impact_tx(&mut tx, etab, absence, auteur)
                .await?;
            evenement_rcd(
                &mut tx,
                etab,
                EVT_FERMETURE_NEUTRALISEE,
                absence,
                json!({ "absence_id": absence }),
                None,
            )
            .await?;
            n += 1;
        }
        tx.commit().await.map_err(db)?;
        Ok(n)
    }

    /// Tâche planifiée : la fermeture de chaque établissement.
    pub async fn neutraliser_absences_fermees(&self) -> Result<u32, EntDbError> {
        let mut total = 0;
        for etab in etablissements_systeme(
            self.pool(),
            ContexteSysteme::JobPlanifie("absences_personnels_fermeture"),
        )
        .await
        .map_err(db)?
        {
            match self.neutraliser_absences_fermees_etab(etab).await {
                Ok(n) => total += n,
                Err(e) => tracing::warn!(%etab, "fermeture : {e}"),
            }
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }
    fn v(x: &[&str]) -> Vec<String> {
        x.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn la_greve_ne_se_declare_jamais_d_avance() {
        let j = d(2027, 3, 10);
        assert!(jour_de_greve_valide(j, j).is_ok());
        assert!(jour_de_greve_valide(j - Duration::days(1), j).is_ok());
        assert!(jour_de_greve_valide(j + Duration::days(1), j).is_err());
        assert!(jour_de_greve_valide(j - Duration::days(32), j).is_err());
    }

    #[test]
    fn les_heures_vont_ensemble() {
        let t = |h| NaiveTime::from_hms_opt(h, 0, 0).unwrap();
        assert!(heures_valides(None, None) && heures_valides(Some(t(8)), Some(t(12))));
        assert!(!heures_valides(Some(t(8)), None) && !heures_valides(Some(t(12)), Some(t(8))));
    }

    #[test]
    fn une_seance_se_classe_selon_ses_impacts() {
        assert_eq!(
            classer_seance("prevue", &[]),
            (false, ClasseSeance::Normale)
        );
        assert_eq!(
            classer_seance("prevue", &v(&["sans_objet"])),
            (false, ClasseSeance::Normale)
        );
        assert_eq!(
            classer_seance("prevue", &v(&["a_traiter"])),
            (true, ClasseSeance::EnAttente)
        );
        assert_eq!(
            classer_seance("prevue", &v(&["sollicite"])),
            (true, ClasseSeance::EnAttente)
        );
        assert_eq!(
            classer_seance("modifiee", &v(&["remplace"])),
            (true, ClasseSeance::Assuree)
        );
        assert_eq!(
            classer_seance("annulee", &v(&["annule"])),
            (true, ClasseSeance::NonAssuree)
        );
        assert_eq!(
            classer_seance("prevue", &v(&["non_assure"])),
            (true, ClasseSeance::NonAssuree)
        );
        assert_eq!(
            classer_seance("annulee", &[]),
            (false, ClasseSeance::NonAssuree)
        );
        assert_eq!(
            classer_seance("prevue", &v(&["continuite"])),
            (true, ClasseSeance::Assuree)
        );
    }

    #[test]
    fn le_taux_est_un_pourcentage_entier() {
        assert_eq!(taux_pct(0, 0), 0);
        assert_eq!(taux_pct(1, 3), 33);
        assert_eq!(taux_pct(3, 3), 100);
    }

    #[test]
    fn la_retenue_compte_un_jour_par_jour_de_greve_hors_jours_fermes() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let mois = d(2027, 3, 1);
        let fermes: HashSet<NaiveDate> = [d(2027, 3, 12)].into_iter().collect();
        let plages = vec![
            (a, d(2027, 3, 10), d(2027, 3, 10)), // moins d'une journée : un jour
            (a, d(2027, 3, 10), d(2027, 3, 10)), // doublon : un seul jour
            (a, d(2027, 3, 11), d(2027, 3, 13)), // le 12 est fermé
            (b, d(2027, 2, 27), d(2027, 3, 2)),  // à cheval sur février : seuls mars 1 et 2
            (b, d(2027, 3, 31), d(2027, 4, 2)),  // à cheval sur avril : seul le 31
        ];
        let j = jours_de_retenue(&plages, mois, &fermes);
        assert_eq!(j[&a], vec![d(2027, 3, 10), d(2027, 3, 11), d(2027, 3, 13)]);
        assert_eq!(j[&b], vec![d(2027, 3, 1), d(2027, 3, 2), d(2027, 3, 31)]);
    }

    #[test]
    fn le_nom_complet_gere_l_absence_de_prenom() {
        assert_eq!(nom_complet("Durand", &Some("Alice".into())), "Alice Durand");
        assert_eq!(nom_complet("Durand", &None), "Durand");
        assert_eq!(nom_complet("Durand", &Some("  ".into())), "Durand");
    }
}
