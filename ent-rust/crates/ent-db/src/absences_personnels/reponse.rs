//! Socle 15, lot A4 — répondre à un impact : proposer, solliciter, désigner, accepter, refuser, lever
//! (`docs/socles/15_ABSENCES_PERSONNELS.md` §11, R-AP19, R-AP24, R-AP37, R-AP41 ; ABP-158, 167, 169, 175 à 178,
//! 184 à 186, 247, 299).
//!
//! Niveau « organisation » : rien ici ne lit ni n'écrit la nature de l'absence ; les messages et les événements
//! ne portent que des identifiants et la description neutre de la séance.
//!
//! **Garanties.**
//! * Un agent n'a **jamais deux séances acceptées ou désignées qui se chevauchent** : contrainte d'exclusion en
//!   base (migration 2305), donc valable quel que soit le chemin ; deux acceptations simultanées, une seule passe
//!   (CAP-31, AR-09). Une séance n'a qu'une demande ouverte (hors appel à volontaires) et une seule retenue.
//! * Une réponse est **réversible** (`lever_reponse`) : l'exception d'emploi du temps est levée (délégation
//!   révoquée, AR-18), la demande retenue passe `annulee` (cause `levee`) et la séance redevient à traiter.
//! * Une acceptation périmée (demande expirée, séance déjà pourvue) est refusée **proprement** (ABP-299).
//! * **Pas de sortie anticipée sans information préalable des familles** (R-13, R-AP24) : tant que le canal de
//!   prévenance des familles (socles 8 et 10) n'existe pas, aucune sortie anticipée n'est acceptée.
//! * Les propositions suivent des critères **déterministes et affichés** ; jamais l'âge, l'ancienneté, l'historique
//!   d'absences, la santé, la situation familiale (R-AP37) ; aucune contrainte d'un collègue n'est nommée à un autre.
//!
//! **Application à l'emploi du temps.** Le socle 10 n'expose pas encore de port d'écriture (`modifier_seance`) : la
//! réponse passe par les commandes existantes de l'emploi du temps (`edt_organiser_remplacement`,
//! `edt_annuler_seance`, `edt_lever_exception`), au nom de la personne qui a sollicité (ses droits, sa délégation).
//! L'état de l'impact et la réponse sont ensuite repris par le recalcul du lot A3.
//!
//! **Non fait (lots suivants ou ports absents)** : autre cours (179), déplacer (182), continuité encadrée (183),
//! échanges (276), avancer (277), SMS (278), capacité de salle (280), créneau RCD rendu (281), surveillants
//! d'examen (296), contestation d'une désignation (187), correction d'une séance passée (188), plan en lot (168),
//! refus motivé d'un volontaire RCD (177 : plan RCD au lot A6), notification effective des familles (236 à 247 :
//! socles 8 et 10).

use super::*;
use crate::edt_remplacement::CmdRemplacement;
use crate::ref_commun::Acteur;
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme};

/// Fenêtre de service d'une sollicitation immédiate (§11.3, Q-AP14) : de 6 h 30 à 20 h, heure de l'établissement.
pub const SERVICE_DEBUT: (u32, u32) = (6, 30);
pub const SERVICE_FIN: (u32, u32) = (20, 0);
const REFUS_CODES: &[&str] = &["absent", "autre_service", "delai", "autre"];

/// Une sollicitation est-elle envoyable à cette heure locale ? Hors fenêtre, elle attend 6 h 30.
pub fn envoi_autorise(heure_locale: NaiveTime) -> bool {
    let (d, f) = (
        NaiveTime::from_hms_opt(SERVICE_DEBUT.0, SERVICE_DEBUT.1, 0).expect("heure"),
        NaiveTime::from_hms_opt(SERVICE_FIN.0, SERVICE_FIN.1, 0).expect("heure"),
    );
    heure_locale >= d && heure_locale < f
}

/// Délai d'expiration (S-1505) : court pour une séance du jour même, long sinon.
pub fn delai_expiration(jour_meme: bool, minutes_jour_meme: i64, heures_autre: i64) -> Duration {
    if jour_meme {
        Duration::minutes(minutes_jour_meme)
    } else {
        Duration::hours(heures_autre)
    }
}

/// Message neutre envoyé au remplaçant : la séance, jamais la nature de l'absence (R-AP41). Un mot exclu dans
/// la matière ou la classe (texte saisi par ailleurs) fait retomber sur une formule générique.
pub fn message_sollicitation(
    matiere: &str,
    classe: &str,
    jour: NaiveDate,
    debut: NaiveTime,
    fin: NaiveTime,
    salle: Option<&str>,
) -> String {
    let salle = salle.map(str::trim).filter(|s| !s.is_empty());
    let detail = format!("{matiere} {classe} {salle}", salle = salle.unwrap_or(""));
    if mot_exclu(&detail).is_some() {
        return "Une séance a besoin d'être couverte. Pouvez-vous la prendre en charge ?".to_string();
    }
    let lieu = salle.map(|s| format!(", salle {s}")).unwrap_or_default();
    format!(
        "Pouvez-vous assurer le cours de {matiere} avec la classe {classe}, le {} de {} à {}{lieu} ?",
        jour.format("%d/%m/%Y"),
        debut.format("%H h %M"),
        fin.format("%H h %M"),
    )
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ImpactSeance {
    id: Uuid,
    absence_id: Uuid,
    etat: String,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    seance_id: Option<Uuid>,
    creneau_id: Option<Uuid>,
    jour: Option<NaiveDate>,
    heure_debut: Option<NaiveTime>,
    heure_fin: Option<NaiveTime>,
    classe: Option<String>,
    matiere_libelle: Option<String>,
    salle: Option<String>,
    absent_agent_id: Uuid,
}

const IMPACT_SEANCE: &str = "SELECT i.id, i.absence_id, i.etat, i.debut, i.fin, i.seance_id,
        s.creneau_id, s.jour, s.heure_debut, s.heure_fin, s.classe, s.matiere_libelle, s.salle,
        a.agent_id AS absent_agent_id
   FROM ent_absp_impact i
   JOIN ent_absp_absence a ON a.etablissement_id = i.etablissement_id AND a.id = i.absence_id
   LEFT JOIN ent_edt_seance s ON s.etablissement_id = i.etablissement_id AND s.id = i.seance_id";

impl ImpactSeance {
    fn seance(&self) -> Option<SeanceImpactDto> {
        Some(SeanceImpactDto {
            seance_id: self.seance_id?,
            creneau_id: self.creneau_id?,
            jour: self.jour?,
            heure_debut: self.heure_debut?,
            heure_fin: self.heure_fin?,
            classe: self.classe.clone()?,
            matiere_libelle: self.matiere_libelle.clone()?,
            salle: self.salle.clone(),
        })
    }
    fn message(&self) -> String {
        match self.seance() {
            Some(s) => message_sollicitation(&s.matiere_libelle, &s.classe, s.jour, s.heure_debut, s.heure_fin, s.salle.as_deref()),
            None => "Une séance a besoin d'être couverte.".to_string(),
        }
    }
    /// L'impact accepte-t-il encore une réponse ?
    fn ouvert(&self) -> bool {
        matches!(self.etat.as_str(), "a_traiter" | "non_assure" | "propose" | "sollicite")
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SollicitationLigne {
    id: Uuid,
    impact_id: Uuid,
    agent_id: Uuid,
    mode: String,
    groupe_id: Option<Uuid>,
    sollicite_par_user_id: Uuid,
    sollicite_par_role: String,
    envoyee_le: DateTime<Utc>,
    envoi_prevu_le: Option<DateTime<Utc>>,
    expire_le: Option<DateTime<Utc>>,
    etat: String,
    repondue_le: Option<DateTime<Utc>>,
    accuse_le: Option<DateTime<Utc>>,
}

const SOLLICITATION_COLONNES: &str = "id, impact_id, agent_id, mode, groupe_id, sollicite_par_user_id,
    sollicite_par_role, envoyee_le, envoi_prevu_le, expire_le, etat, repondue_le, accuse_le";

fn sollicitation_dto(s: SollicitationLigne, i: &ImpactSeance) -> SollicitationDto {
    SollicitationDto {
        id: s.id,
        impact_id: s.impact_id,
        agent_id: s.agent_id,
        mode: s.mode,
        etat: s.etat,
        envoyee_le: s.envoyee_le,
        expire_le: s.expire_le,
        repondue_le: s.repondue_le,
        accuse_le: s.accuse_le,
        seance: i.seance(),
        message: i.message(),
    }
}

/// Conflit d'exclusion (`23P01`) ou d'unicité (`23505`) : « déjà pris » plutôt qu'une panne.
fn est_conflit_base(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if matches!(d.code().as_deref(), Some("23P01") | Some("23505")))
}

impl PgEnt {
    /// Droit d'organiser les remplacements : `edt.gerer` ET lecture d'organisation (l'administrateur technique
    /// n'a pas accès). Renvoie le rôle.
    async fn exiger_organiser(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<Role, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !(self.a_droit_abs(etab, actor, role, Action::EdtGerer).await?
            && self.peut_lire_organisation(etab, actor, role).await?)
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(r)
    }

    async fn impact_seance(
        conn: &mut PgConnection,
        etab: Uuid,
        impact_id: Uuid,
        verrou: bool,
    ) -> Result<ImpactSeance, EntDbError> {
        sqlx::query_as::<_, ImpactSeance>(&format!(
            "{IMPACT_SEANCE} WHERE i.etablissement_id = $1 AND i.id = $2 {}",
            if verrou { "FOR UPDATE OF i" } else { "" }
        ))
        .bind(etab)
        .bind(impact_id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    /// Une absence en confidentialité renforcée n'est répondable que par la direction et les valideurs.
    async fn controle_renforcee(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<(), EntDbError> {
        let (renforcee,): (bool,) = sqlx::query_as(
            "SELECT confidentialite_renforcee FROM ent_absp_absence WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_one(conn)
        .await
        .map_err(db)?;
        if renforcee && !self.voit_renforcees(etab, actor, role).await? {
            return Err(introuvable());
        }
        Ok(())
    }

    // ───────────────────────────── ABP-167 : propositions ─────────────────────────────

    /// Remplaçants possibles pour un impact, du mieux placé au moins bien placé, **avec leurs raisons** :
    /// même discipline d'abord, puis le moins sollicité sur l'année (remplacements faits, puis demandes sans suite),
    /// puis la charge de la journée, puis le nom. Candidats : libres sur la plage et sans conflit (règles de
    /// l'emploi du temps), hors l'absent et hors un agent déjà retenu sur une plage qui chevauche.
    pub async fn propositions_impact_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        impact_id: Uuid,
    ) -> Result<Vec<PropositionDto>, EntDbError> {
        self.exiger_organiser(etab, actor, role).await?;
        let (imp, absence_agent) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let imp = Self::impact_seance(&mut tx, etab, impact_id, false).await?;
            self.controle_renforcee(&mut tx, etab, actor, role, imp.absence_id).await?;
            let a = imp.absent_agent_id;
            (imp, a)
        };
        let (Some(creneau), Some(jour)) = (imp.creneau_id, imp.jour) else {
            return Err(conflit("la séance de cet impact n'existe plus"));
        };
        let acteur = Acteur { etablissement_id: etab, user_id: actor, role };
        let candidats = self.edt_candidats_remplacants(&acteur, creneau, jour, true).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut sortie: Vec<PropositionDto> = Vec::new();
        for c in candidats {
            let Some(agent) = agent_du_user_tx(&mut tx, etab, c.user_id).await? else { continue };
            if agent == absence_agent {
                continue;
            }
            // Déjà retenu (accepté ou désigné) sur une plage qui chevauche : jamais proposé.
            let (pris,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_absp_sollicitation WHERE etablissement_id = $1 AND agent_id = $2
                                 AND etat IN ('acceptee', 'designee') AND plage && tstzrange($3, $4))",
            )
            .bind(etab)
            .bind(agent)
            .bind(imp.debut)
            .bind(imp.fin)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if pris {
                continue;
            }
            let (faits, sans_suite): (i64, i64) = sqlx::query_as(
                "SELECT count(*) FILTER (WHERE etat IN ('acceptee', 'designee')),
                        count(*) FILTER (WHERE etat IN ('refusee', 'expiree'))
                   FROM ent_absp_sollicitation
                  WHERE etablissement_id = $1 AND agent_id = $2 AND envoyee_le >= now() - interval '1 year'",
            )
            .bind(etab)
            .bind(agent)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            sortie.push(PropositionDto {
                agent_id: agent,
                nom: c.nom,
                meme_discipline: c.competent,
                seances_ce_jour: c.seances_ce_jour as u32,
                remplacements_faits: faits as u32,
                demandes_sans_suite: sans_suite as u32,
            });
        }
        sortie.sort_by(|a, b| {
            (!a.meme_discipline, a.remplacements_faits, a.demandes_sans_suite, a.seances_ce_jour, &a.nom)
                .cmp(&(!b.meme_discipline, b.remplacements_faits, b.demandes_sans_suite, b.seances_ce_jour, &b.nom))
        });
        Ok(sortie)
    }

    // ───────────────────────────── ABP-175, 185 : solliciter ─────────────────────────────

    /// Sollicite un remplaçant pour un impact (§11.3). Une seule demande ouverte par séance ; expiration selon
    /// S-1505 ; hors de la fenêtre de service (6 h 30 – 20 h) l'envoi attend 6 h 30. Le message est neutre.
    pub async fn solliciter_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        impact_id: Uuid,
        agent_id: Uuid,
    ) -> Result<SollicitationDto, EntDbError> {
        let mut v = self.creer_sollicitations(etab, actor, role, impact_id, &[agent_id], "sollicitation").await?;
        Ok(v.remove(0))
    }

    /// ABP-185 : appel à volontaires — plusieurs destinataires pour la même séance ; le premier qui accepte est
    /// retenu, les autres sont annulés (« prise »).
    pub async fn appel_volontaires_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        impact_id: Uuid,
        agents: &[Uuid],
    ) -> Result<Vec<SollicitationDto>, EntDbError> {
        if agents.len() < 2 || agents.len() > 30 {
            return Err(invalide("un appel à volontaires vise entre 2 et 30 personnes"));
        }
        self.creer_sollicitations(etab, actor, role, impact_id, agents, "appel_volontaires").await
    }

    async fn creer_sollicitations(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        impact_id: Uuid,
        agents: &[Uuid],
        mode: &str,
    ) -> Result<Vec<SollicitationDto>, EntDbError> {
        self.exiger_organiser(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let imp = Self::impact_seance(&mut tx, etab, impact_id, true).await?;
        self.controle_renforcee(&mut tx, etab, actor, role, imp.absence_id).await?;
        if !imp.ouvert() {
            return Err(conflit("cette séance a déjà une réponse"));
        }
        let (maintenant, jour_local, heure_locale): (DateTime<Utc>, NaiveDate, NaiveTime) = sqlx::query_as(
            "SELECT now(), ent_aujourdhui_etab($1, now()),
                    (now() AT TIME ZONE COALESCE((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1),
                                                  'Europe/Paris'))::time",
        )
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if imp.fin <= maintenant {
            return Err(conflit("la séance est passée"));
        }
        let (minutes, heures): (i16, i16) = sqlx::query_as(
            "SELECT COALESCE((SELECT sollicitation_expire_jour_meme_min FROM ent_absp_param WHERE etablissement_id = $1), 20)::smallint,
                    COALESCE((SELECT sollicitation_expire_autre_h FROM ent_absp_param WHERE etablissement_id = $1), 12)::smallint",
        )
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let jour_meme = imp.jour == Some(jour_local);
        // Plage calme : une demande du jour même ou du lendemain matin part de 6 h 30 à 20 h ; sinon elle attend.
        let envoi_prevu: Option<DateTime<Utc>> = if envoi_autorise(heure_locale) {
            None
        } else {
            let (p,): (DateTime<Utc>,) = sqlx::query_as(
                "SELECT (((CASE WHEN (now() AT TIME ZONE f.fus)::time >= time '20:00'
                                 THEN ((now() AT TIME ZONE f.fus)::date + 1) ELSE (now() AT TIME ZONE f.fus)::date END)
                          + time '06:30') AT TIME ZONE f.fus)
                   FROM (SELECT COALESCE((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1),
                                          'Europe/Paris') AS fus) f",
            )
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            Some(p)
        };
        let depart = envoi_prevu.unwrap_or(maintenant);
        let expire = depart + delai_expiration(jour_meme, minutes as i64, heures as i64);
        let groupe = (mode == "appel_volontaires").then(Uuid::new_v4);
        let mut sortie = Vec::new();
        let mut evenements = Vec::new();
        for &agent in agents {
            if agent == imp.absent_agent_id {
                return Err(invalide("l'absent ne peut pas être sollicité pour sa propre séance"));
            }
            let (existe, a_compte): (bool, bool) = sqlx::query_as(
                "SELECT true, p.user_id IS NOT NULL FROM ent_agent ag
                   JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
                  WHERE ag.etablissement_id = $1 AND ag.id = $2",
            )
            .bind(etab)
            .bind(agent)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .unwrap_or((false, false));
            if !existe || !a_compte {
                return Err(introuvable());
            }
            let id = Uuid::new_v4();
            let res = sqlx::query(
                "INSERT INTO ent_absp_sollicitation (id, etablissement_id, impact_id, agent_id, mode, groupe_id,
                        sollicite_par_user_id, sollicite_par_role, envoyee_le, envoi_prevu_le, expire_le, plage)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11, tstzrange($12, $13))",
            )
            .bind(id)
            .bind(etab)
            .bind(impact_id)
            .bind(agent)
            .bind(mode)
            .bind(groupe)
            .bind(actor)
            .bind(role)
            .bind(maintenant)
            .bind(envoi_prevu)
            .bind(expire)
            .bind(imp.debut)
            .bind(imp.fin)
            .execute(&mut *tx)
            .await;
            if let Err(e) = res {
                return Err(if est_conflit_base(&e) { conflit("une demande est déjà en cours pour cette séance") } else { db(e) });
            }
            let ligne = Self::sollicitation(&mut tx, etab, id).await?;
            if envoi_prevu.is_none() {
                evenements.push(
                    evenement(
                        &mut tx,
                        etab,
                        "absence_personnel.sollicitation_envoyee",
                        imp.absence_id,
                        agent,
                        json!({ "sollicitation_id": id, "impact_id": impact_id, "mode": mode,
                                "seance_id": imp.seance_id, "expire_le": expire }),
                        actor,
                    )
                    .await?,
                );
            }
            sortie.push(sollicitation_dto(ligne, &imp));
        }
        sqlx::query("UPDATE ent_absp_impact SET etat = 'sollicite', calcule_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(impact_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        journal(&mut tx, etab, imp.absence_id, "sollicitation_envoyee", Some("mode"), None, Some(mode), actor).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evenements);
        Ok(sortie)
    }

    async fn sollicitation(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<SollicitationLigne, EntDbError> {
        sqlx::query_as::<_, SollicitationLigne>(&format!(
            "SELECT {SOLLICITATION_COLONNES} FROM ent_absp_sollicitation WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    // ───────────────────────────── ABP-169 : désigner ─────────────────────────────

    /// Désignation directe d'urgence, sans sollicitation : la plage est réservée en base (exclusion), le
    /// remplacement est appliqué à l'emploi du temps (disponibilité contrôlée en transaction, R-AP19), le
    /// remplaçant reçoit l'information et **accuse réception** (ABP-178).
    pub async fn designer_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        impact_id: Uuid,
        agent_id: Uuid,
    ) -> Result<SollicitationDto, EntDbError> {
        self.exiger_organiser(etab, actor, role).await?;
        let (sollicitation_id, imp, remplacant_user) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let imp = Self::impact_seance(&mut tx, etab, impact_id, true).await?;
            self.controle_renforcee(&mut tx, etab, actor, role, imp.absence_id).await?;
            if !imp.ouvert() {
                return Err(conflit("cette séance a déjà une réponse"));
            }
            if agent_id == imp.absent_agent_id {
                return Err(invalide("l'absent ne peut pas être désigné pour sa propre séance"));
            }
            let user: Uuid = sqlx::query_scalar(
                "SELECT p.user_id FROM ent_agent ag
                   JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
                  WHERE ag.etablissement_id = $1 AND ag.id = $2 AND p.user_id IS NOT NULL",
            )
            .bind(etab)
            .bind(agent_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
            // Une demande ouverte sur la séance est annulée : la désignation la remplace.
            sqlx::query(
                "UPDATE ent_absp_sollicitation SET etat = 'annulee', annulee_cause = 'prise', repondue_le = now()
                  WHERE etablissement_id = $1 AND impact_id = $2 AND etat = 'envoyee'",
            )
            .bind(etab)
            .bind(impact_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let id = Uuid::new_v4();
            let res = sqlx::query(
                "INSERT INTO ent_absp_sollicitation (id, etablissement_id, impact_id, agent_id, mode,
                        sollicite_par_user_id, sollicite_par_role, etat, plage)
                 VALUES ($1,$2,$3,$4,'designation',$5,$6,'designee', tstzrange($7, $8))",
            )
            .bind(id)
            .bind(etab)
            .bind(impact_id)
            .bind(agent_id)
            .bind(actor)
            .bind(role)
            .bind(imp.debut)
            .bind(imp.fin)
            .execute(&mut *tx)
            .await;
            if let Err(e) = res {
                return Err(if est_conflit_base(&e) {
                    conflit("cet agent est déjà retenu sur une séance qui chevauche celle-ci")
                } else {
                    db(e)
                });
            }
            journal(&mut tx, etab, imp.absence_id, "designation", Some("mode"), None, Some("designation"), actor).await?;
            tx.commit().await.map_err(db)?;
            (id, imp, user)
        };
        self.appliquer_remplacement(etab, actor, role, &imp, remplacant_user, sollicitation_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = evenement(
            &mut tx,
            etab,
            "absence_personnel.designation",
            imp.absence_id,
            agent_id,
            json!({ "sollicitation_id": sollicitation_id, "impact_id": impact_id, "seance_id": imp.seance_id }),
            actor,
        )
        .await?;
        let ligne = Self::sollicitation(&mut tx, etab, sollicitation_id).await?;
        let imp = Self::impact_seance(&mut tx, etab, impact_id, false).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(sollicitation_dto(ligne, &imp))
    }

    /// Applique le remplacement à l'emploi du temps au nom de `actor` (ses droits), puis fait reprendre l'impact
    /// par le recalcul. En cas d'échec, la demande retenue est annulée (cause `echec`) et l'impact redevient à
    /// traiter : jamais une séance « acceptée » que l'emploi du temps ne connaît pas.
    async fn appliquer_remplacement(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        imp: &ImpactSeance,
        remplacant_user: Uuid,
        sollicitation_id: Uuid,
    ) -> Result<(), EntDbError> {
        let (Some(creneau), Some(jour)) = (imp.creneau_id, imp.jour) else {
            return self.defaire(etab, imp, sollicitation_id, conflit("la séance n'existe plus")).await;
        };
        let acteur = Acteur { etablissement_id: etab, user_id: actor, role };
        let r = self
            .edt_organiser_remplacement(
                &acteur,
                &CmdRemplacement {
                    creneau_id: creneau,
                    jour,
                    remplacant_user_id: remplacant_user,
                    absence_rh_id: Some(imp.absence_id),
                    nouvelle_salle: None,
                },
            )
            .await;
        if let Err(e) = r {
            return self.defaire(etab, imp, sollicitation_id, e).await;
        }
        self.recalculer_impact_absence(etab, actor, imp.absence_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_absp_sollicitation s SET reponse_id = i.reponse_active_id
               FROM ent_absp_impact i
              WHERE s.etablissement_id = $1 AND s.id = $2 AND i.etablissement_id = s.etablissement_id AND i.id = s.impact_id",
        )
        .bind(etab)
        .bind(sollicitation_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    async fn defaire(&self, etab: Uuid, imp: &ImpactSeance, sollicitation_id: Uuid, cause: EntDbError) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_absp_sollicitation SET etat = 'annulee', annulee_cause = 'echec', repondue_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(sollicitation_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("UPDATE ent_absp_impact SET etat = 'a_traiter', calcule_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(imp.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Err(cause)
    }

    // ───────────────────────────── ABP-176, 177, 178, 299 : le remplaçant répond ─────────────────────────────

    async fn ma_sollicitation(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        id: Uuid,
        verrou: bool,
    ) -> Result<SollicitationLigne, EntDbError> {
        let mon_agent = agent_du_user_tx(conn, etab, actor).await?.ok_or_else(introuvable)?;
        let s = sqlx::query_as::<_, SollicitationLigne>(&format!(
            "SELECT {SOLLICITATION_COLONNES} FROM ent_absp_sollicitation WHERE etablissement_id = $1 AND id = $2 {}",
            if verrou { "FOR UPDATE" } else { "" }
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        // Les demandes des autres sont introuvables (pas d'oracle) ; une envoi différé n'est pas encore visible.
        if s.agent_id != mon_agent {
            return Err(introuvable());
        }
        Ok(s)
    }

    /// Accepte une sollicitation (d'un geste). Refus propre si elle a expiré, si la séance est déjà pourvue
    /// (ABP-299) ou si l'agent a déjà accepté une séance qui chevauche (CAP-31). L'acceptation est appliquée à
    /// l'emploi du temps au nom de la personne qui a sollicité.
    pub async fn accepter_sollicitation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        sollicitation_id: Uuid,
    ) -> Result<SollicitationDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let (s, imp, remplacant_user) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let s = self.ma_sollicitation(&mut tx, etab, actor, sollicitation_id, true).await?;
            let (maintenant,): (DateTime<Utc>,) =
                sqlx::query_as("SELECT now()").fetch_one(&mut *tx).await.map_err(db)?;
            if s.envoi_prevu_le.is_some_and(|p| p > maintenant) {
                return Err(introuvable());
            }
            let imp = Self::impact_seance(&mut tx, etab, s.impact_id, true).await?;
            if s.etat != "envoyee" {
                return Err(conflit(match s.etat.as_str() {
                    "acceptee" | "designee" => "déjà acceptée",
                    "annulee" => "cette demande a été reprise : la séance est déjà pourvue",
                    "expiree" => "cette demande a expiré",
                    _ => "cette demande n'est plus ouverte",
                }));
            }
            if s.expire_le.is_some_and(|e| e <= maintenant) {
                sqlx::query("UPDATE ent_absp_sollicitation SET etat = 'expiree', repondue_le = now() WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(s.id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                tx.commit().await.map_err(db)?;
                return Err(conflit("cette demande a expiré"));
            }
            if !imp.ouvert() || imp.fin <= maintenant {
                sqlx::query(
                    "UPDATE ent_absp_sollicitation SET etat = 'annulee', annulee_cause = 'prise', repondue_le = now()
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(s.id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                tx.commit().await.map_err(db)?;
                return Err(conflit("la séance est déjà pourvue ou passée"));
            }
            let res = sqlx::query(
                "UPDATE ent_absp_sollicitation SET etat = 'acceptee', repondue_le = now(), accuse_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(s.id)
            .execute(&mut *tx)
            .await;
            if let Err(e) = res {
                return Err(if est_conflit_base(&e) {
                    conflit("vous avez déjà accepté une séance qui chevauche celle-ci")
                } else {
                    db(e)
                });
            }
            // Appel à volontaires : le premier qui accepte gagne, les autres sont prévenus.
            if let Some(g) = s.groupe_id {
                sqlx::query(
                    "UPDATE ent_absp_sollicitation SET etat = 'annulee', annulee_cause = 'prise', repondue_le = now()
                      WHERE etablissement_id = $1 AND groupe_id = $2 AND id <> $3 AND etat = 'envoyee'",
                )
                .bind(etab)
                .bind(g)
                .bind(s.id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            journal(&mut tx, etab, imp.absence_id, "sollicitation_acceptee", Some("mode"), None, Some(&s.mode), actor).await?;
            tx.commit().await.map_err(db)?;
            (s.clone(), imp, actor)
        };
        self.appliquer_remplacement(etab, s.sollicite_par_user_id, &s.sollicite_par_role, &imp, remplacant_user, s.id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let agent = Self::sollicitation(&mut tx, etab, s.id).await?;
        let ev = evenement(
            &mut tx,
            etab,
            "absence_personnel.sollicitation_acceptee",
            imp.absence_id,
            agent.agent_id,
            json!({ "sollicitation_id": s.id, "impact_id": imp.id, "seance_id": imp.seance_id }),
            actor,
        )
        .await?;
        let imp = Self::impact_seance(&mut tx, etab, imp.id, false).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(sollicitation_dto(agent, &imp))
    }

    /// Refuse une sollicitation. Libre et **sans motif** pour un collègue (aucune obligation) ; un motif légitime
    /// codé est accepté (volontaire RCD, lot A6). La séance redevient à traiter s'il n'y a plus de demande ouverte.
    pub async fn refuser_sollicitation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        sollicitation_id: Uuid,
        refus_code: Option<&str>,
    ) -> Result<SollicitationDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if let Some(c) = refus_code {
            if !REFUS_CODES.contains(&c) {
                return Err(invalide("motif de refus inconnu"));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = self.ma_sollicitation(&mut tx, etab, actor, sollicitation_id, true).await?;
        if s.etat != "envoyee" {
            return Err(conflit("cette demande n'est plus ouverte"));
        }
        sqlx::query(
            "UPDATE ent_absp_sollicitation SET etat = 'refusee', repondue_le = now(), refus_code = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(s.id)
        .bind(refus_code)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let imp = Self::impact_seance(&mut tx, etab, s.impact_id, true).await?;
        self.rouvrir_impact_sans_demande(&mut tx, etab, &imp).await?;
        journal(&mut tx, etab, imp.absence_id, "sollicitation_refusee", Some("mode"), None, Some(&s.mode), actor).await?;
        let ligne = Self::sollicitation(&mut tx, etab, s.id).await?;
        let imp = Self::impact_seance(&mut tx, etab, s.impact_id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(sollicitation_dto(ligne, &imp))
    }

    /// L'impact redevient « à traiter » quand il n'a plus aucune demande ouverte ni retenue.
    async fn rouvrir_impact_sans_demande(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        imp: &ImpactSeance,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "UPDATE ent_absp_impact i SET etat = 'a_traiter', calcule_le = now()
              WHERE i.etablissement_id = $1 AND i.id = $2 AND i.etat = 'sollicite'
                AND NOT EXISTS (SELECT 1 FROM ent_absp_sollicitation s WHERE s.etablissement_id = i.etablissement_id
                                  AND s.impact_id = i.id AND s.etat IN ('envoyee', 'acceptee', 'designee'))",
        )
        .bind(etab)
        .bind(imp.id)
        .execute(conn)
        .await
        .map_err(db)?;
        Ok(())
    }

    /// ABP-178 : le remplaçant accuse réception d'une désignation (ou d'une acceptation). Idempotent.
    pub async fn accuser_reception_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        sollicitation_id: Uuid,
    ) -> Result<SollicitationDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = self.ma_sollicitation(&mut tx, etab, actor, sollicitation_id, true).await?;
        if !matches!(s.etat.as_str(), "designee" | "acceptee") {
            return Err(conflit("rien à accuser : cette demande n'est pas retenue"));
        }
        sqlx::query("UPDATE ent_absp_sollicitation SET accuse_le = COALESCE(accuse_le, now()) WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(s.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let ligne = Self::sollicitation(&mut tx, etab, s.id).await?;
        let imp = Self::impact_seance(&mut tx, etab, s.impact_id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(sollicitation_dto(ligne, &imp))
    }

    /// ABP-158 : « ma séance de remplacement » — les demandes ouvertes et les séances retenues de l'agent
    /// connecté, avec la séance (classe, matière, heure, salle) et le message neutre ; jamais la nature.
    pub async fn mes_sollicitations_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<SollicitationDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some(mon_agent) = agent_du_user_tx(&mut tx, etab, actor).await? else {
            return Ok(Vec::new());
        };
        let lignes: Vec<SollicitationLigne> = sqlx::query_as(&format!(
            "SELECT {SOLLICITATION_COLONNES} FROM ent_absp_sollicitation
              WHERE etablissement_id = $1 AND agent_id = $2
                AND ((etat = 'envoyee' AND (expire_le IS NULL OR expire_le > now())
                      AND (envoi_prevu_le IS NULL OR envoi_prevu_le <= now()))
                     OR (etat IN ('acceptee', 'designee') AND upper(plage) > now()))
              ORDER BY lower(plage), id"
        ))
        .bind(etab)
        .bind(mon_agent)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut sortie = Vec::with_capacity(lignes.len());
        for l in lignes {
            let imp = Self::impact_seance(&mut tx, etab, l.impact_id, false).await?;
            sortie.push(sollicitation_dto(l, &imp));
        }
        Ok(sortie)
    }

    // ───────────────────────────── ABP-186 : lever ; ABP-184 : annuler ─────────────────────────────

    /// Lève la réponse d'un impact (rétablir) : l'exception d'emploi du temps est levée — la délégation du
    /// remplaçant est révoquée (AR-18) —, la demande retenue passe `annulee` (cause `levee`), la séance redevient à
    /// traiter. Fait daté et journalisé.
    pub async fn lever_reponse_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        impact_id: Uuid,
    ) -> Result<ImpactDto, EntDbError> {
        self.exiger_organiser(etab, actor, role).await?;
        let imp = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let imp = Self::impact_seance(&mut tx, etab, impact_id, false).await?;
            self.controle_renforcee(&mut tx, etab, actor, role, imp.absence_id).await?;
            imp
        };
        if !matches!(imp.etat.as_str(), "remplace" | "annule") {
            return Err(conflit("aucune réponse à lever sur cette séance"));
        }
        let (Some(creneau), Some(jour)) = (imp.creneau_id, imp.jour) else {
            return Err(conflit("la séance n'existe plus"));
        };
        let acteur = Acteur { etablissement_id: etab, user_id: actor, role };
        self.edt_lever_exception(&acteur, creneau, jour).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_absp_sollicitation SET etat = 'annulee', annulee_cause = 'levee', repondue_le = now()
              WHERE etablissement_id = $1 AND impact_id = $2 AND etat IN ('acceptee', 'designee')",
        )
        .bind(etab)
        .bind(impact_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(&mut tx, etab, imp.absence_id, "reponse_levee", Some("impact"), Some(&imp.etat), Some("a_traiter"), actor).await?;
        tx.commit().await.map_err(db)?;
        self.recalculer_impact_absence(etab, actor, imp.absence_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = Self::impact_seance(&mut tx, etab, impact_id, false).await?;
        Ok(ImpactDto {
            id: r.id,
            absence_id: r.absence_id,
            type_impact: "seance".into(),
            seance: r.seance(),
            debut: r.debut,
            fin: r.fin,
            etat: r.etat,
            a_revoir: false,
        })
    }

    /// Annule la séance (les élèves sont pris en charge). **Aucune sortie anticipée sans information préalable
    /// des familles** (R-13, R-AP24, R-AP56) : tant que le canal de prévenance n'existe pas, `sortie_anticipee`
    /// est refusée ; la prise en charge se fait dans les horaires habituels.
    pub async fn annuler_seance_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        impact_id: Uuid,
        sortie_anticipee: bool,
    ) -> Result<ImpactDto, EntDbError> {
        self.exiger_organiser(etab, actor, role).await?;
        if sortie_anticipee {
            return Err(conflit(
                "pas de sortie anticipée sans information préalable des familles : surveillance dans les horaires habituels (R-13)",
            ));
        }
        let imp = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let imp = Self::impact_seance(&mut tx, etab, impact_id, false).await?;
            self.controle_renforcee(&mut tx, etab, actor, role, imp.absence_id).await?;
            imp
        };
        if !imp.ouvert() {
            return Err(conflit("cette séance a déjà une réponse"));
        }
        let (Some(creneau), Some(jour)) = (imp.creneau_id, imp.jour) else {
            return Err(conflit("la séance n'existe plus"));
        };
        let acteur = Acteur { etablissement_id: etab, user_id: actor, role };
        self.edt_annuler_seance(&acteur, creneau, jour, Some(imp.absence_id)).await?;
        // Les demandes ouvertes n'ont plus d'objet.
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_absp_sollicitation SET etat = 'annulee', annulee_cause = 'levee', repondue_le = now()
              WHERE etablissement_id = $1 AND impact_id = $2 AND etat = 'envoyee'",
        )
        .bind(etab)
        .bind(impact_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(&mut tx, etab, imp.absence_id, "seance_annulee", Some("impact"), Some(&imp.etat), Some("annule"), actor).await?;
        tx.commit().await.map_err(db)?;
        self.recalculer_impact_absence(etab, actor, imp.absence_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = Self::impact_seance(&mut tx, etab, impact_id, false).await?;
        Ok(ImpactDto {
            id: r.id,
            absence_id: r.absence_id,
            type_impact: "seance".into(),
            seance: r.seance(),
            debut: r.debut,
            fin: r.fin,
            etat: r.etat,
            a_revoir: false,
        })
    }

    // ───────────────────────────── Tâches planifiées ─────────────────────────────

    /// Tâche (chaque minute, idempotente) : expire les sollicitations échues (la séance redevient à traiter,
    /// la suivante peut être proposée), envoie celles que la plage calme avait différées, et alerte la vie
    /// scolaire quand un remplaçant désigné n'a pas accusé réception une heure avant la séance (ABP-247).
    pub async fn traiter_sollicitations(&self) -> Result<BilanSollicitationsDto, EntDbError> {
        let mut bilan = BilanSollicitationsDto::default();
        for etab in etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("absences_personnels_sollicitations"))
            .await
            .map_err(db)?
        {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let echues: Vec<(Uuid, Uuid, Uuid, Uuid)> = sqlx::query_as(
                "UPDATE ent_absp_sollicitation SET etat = 'expiree', repondue_le = now()
                  WHERE etablissement_id = $1 AND etat = 'envoyee' AND expire_le IS NOT NULL AND expire_le <= now()
                  RETURNING impact_id, id, agent_id, sollicite_par_user_id",
            )
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let mut evenements = Vec::new();
            for (impact_id, sid, agent, par) in &echues {
                let imp = Self::impact_seance(&mut tx, etab, *impact_id, true).await?;
                self.rouvrir_impact_sans_demande(&mut tx, etab, &imp).await?;
                evenements.push(
                    evenement(
                        &mut tx,
                        etab,
                        "absence_personnel.sollicitation_expiree",
                        imp.absence_id,
                        *agent,
                        json!({ "sollicitation_id": sid, "impact_id": impact_id }),
                        *par,
                    )
                    .await?,
                );
                bilan.expirees += 1;
            }
            // Accusés manquants : une alerte par désignation, une seule fois.
            let manquants: Vec<(Uuid, Uuid, Uuid, Uuid, Uuid)> = sqlx::query_as(
                "SELECT s.id, s.impact_id, s.agent_id, s.sollicite_par_user_id, i.absence_id
                   FROM ent_absp_sollicitation s
                   JOIN ent_absp_impact i ON i.etablissement_id = s.etablissement_id AND i.id = s.impact_id
                  WHERE s.etablissement_id = $1 AND s.etat IN ('designee', 'acceptee') AND s.accuse_le IS NULL
                    AND lower(s.plage) - interval '1 hour' <= now() AND lower(s.plage) > now()
                    AND NOT EXISTS (SELECT 1 FROM ent_absp_journal j WHERE j.etablissement_id = s.etablissement_id
                                      AND j.objet = 'absence' AND j.objet_id = i.absence_id
                                      AND j.action = 'accuse_manquant' AND j.valeur_apres = s.id::text)",
            )
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            for (sid, impact_id, agent, par, absence_id) in manquants {
                journal(&mut tx, etab, absence_id, "accuse_manquant", Some("sollicitation"), None, Some(&sid.to_string()), par).await?;
                evenements.push(
                    evenement(
                        &mut tx,
                        etab,
                        "absence_personnel.accuse_manquant",
                        absence_id,
                        agent,
                        json!({ "sollicitation_id": sid, "impact_id": impact_id }),
                        par,
                    )
                    .await?,
                );
                bilan.accuses_manquants += 1;
            }
            tx.commit().await.map_err(db)?;
            self.diffuser_apres_commit(&evenements);
        }
        Ok(bilan)
    }
}

#[cfg(test)]
mod tests_purs {
    use super::*;

    fn h(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn plage_calme_de_20_h_a_6_h_30() {
        assert!(envoi_autorise(h(6, 30)));
        assert!(envoi_autorise(h(12, 0)));
        assert!(envoi_autorise(h(19, 59)));
        assert!(!envoi_autorise(h(20, 0)));
        assert!(!envoi_autorise(h(23, 15)));
        assert!(!envoi_autorise(h(6, 29)));
    }

    #[test]
    fn delai_d_expiration_court_le_jour_meme_long_sinon() {
        assert_eq!(delai_expiration(true, 20, 12), Duration::minutes(20));
        assert_eq!(delai_expiration(false, 20, 12), Duration::hours(12));
    }

    #[test]
    fn message_neutre_sans_nature_ni_mot_exclu() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
        let m = message_sollicitation("Mathématiques", "4eB", d, h(10, 0), h(11, 0), Some("B12"));
        assert!(m.contains("Mathématiques") && m.contains("4eB") && m.contains("salle B12"), "{m}");
        for interdit in ["maladie", "malade", "absent", "arrêt", "congé"] {
            assert!(!m.to_lowercase().contains(interdit), "« {interdit} » dans le message : {m}");
        }
        // Un mot exclu dans le texte saisi fait retomber sur la formule générique.
        let m = message_sollicitation("Mathématiques", "4eB malade", d, h(10, 0), h(11, 0), None);
        assert!(!m.contains("malade"), "{m}");
        assert!(m.contains("séance"), "{m}");
    }
}
