//! Campagnes d'information (socle 2, lot S2-8, COM-05).
//!
//! Une campagne passe par trois temps, avec **confirmation humaine** avant tout
//! envoi :
//!
//! 1. `brouillon` : titre, texte, canaux, audience (établissement, niveaux,
//!    classes ou groupes) ;
//! 2. `apercu` : les destinataires sont **calculés** à partir des droits de
//!    l'auteur et des liens actifs des responsables ; l'aperçu en donne le
//!    nombre, la répartition par canal, l'estimation du coût SMS et une
//!    **empreinte** des destinataires ;
//! 3. `lancee` : l'auteur confirme en renvoyant l'empreinte (et, si des SMS
//!    partent, une seconde confirmation explicite). Le serveur **recalcule** les
//!    destinataires : si quelque chose a changé depuis l'aperçu (lien suspendu,
//!    élève réaffecté, droits retirés), le lancement est refusé et l'aperçu
//!    doit être refait. Un second lancement est refusé (idempotence).
//!
//! **Ciblage selon les droits.** L'action `campagnes.lancer` porte une portée :
//! un rôle limité à des classes désignées ne peut atteindre que les familles de
//! ces classes ; l'audience « établissement » exige la portée établissement.
//! **Seuls** les responsables ayant un lien actif avec autorité parentale, compte
//! rattaché à l'établissement avec le rôle « parent », sont destinataires.
//!
//! **Anti-abus.** Plafonds par établissement (campagnes par jour, SMS par
//! campagne, SMS par jour) vérifiés au lancement ; plafonds SMS par personne et
//! par jour vérifiés à l'envoi (voir `notifications`).

use crate::notifications::{
    canaux_disponibles, db, notifier_personne, plafonds_de, EnvoiAPlanifier, Preferences,
};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use uuid::Uuid;

pub mod types_com {
    pub const CAMPAGNE_LANCEE: &str = "campagne.lancee";
    pub const RENCONTRE_OUVERTE: &str = "agenda.rencontre_ouverte";
    pub const RDV_PRIS: &str = "agenda.rdv_pris";
    pub const RDV_ANNULE: &str = "agenda.rdv_annule";
    pub const FORMULAIRE_PUBLIE: &str = "formulaire.publie";
    pub const FORMULAIRE_REPONDU: &str = "formulaire.repondu";
    pub const CONSENTEMENT_DONNE: &str = "consentement.donne";
    pub const CONSENTEMENT_REFUSE: &str = "consentement.refuse";
    pub const CONSENTEMENT_RETIRE: &str = "consentement.retire";
}

/// Audiences possibles d'une campagne ou d'un formulaire.
pub const AUDIENCES: [&str; 4] = ["etablissement", "niveau", "classe", "groupe"];

pub(crate) fn hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

pub(crate) fn sha256_hex(texte: &str) -> String {
    hex(&Sha256::digest(texte.as_bytes()))
}

fn nettoyer(v: &str, max: usize, msg: &'static str) -> Result<String, EntDbError> {
    let t = v.trim();
    if t.is_empty() || t.chars().count() > max {
        return Err(EntDbError::SaisieInvalide(msg));
    }
    Ok(t.to_string())
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Campagne {
    pub id: Uuid,
    pub titre: String,
    pub corps: String,
    pub canaux: Vec<String>,
    pub audience: String,
    pub cible_ids: Vec<Uuid>,
    pub statut: String,
    pub cree_par: Uuid,
    pub apercu_empreinte: Option<String>,
    pub apercu_nb: Option<i32>,
    pub apercu_sms: Option<i32>,
    pub apercu_le: Option<DateTime<Utc>>,
    pub lancee_par: Option<Uuid>,
    pub lancee_le: Option<DateTime<Utc>>,
    pub nb_destinataires: Option<i32>,
    pub created_at: DateTime<Utc>,
}

const COLONNES_CAMPAGNE: &str = "id, titre, corps, canaux, audience, cible_ids, statut, cree_par, \
    apercu_empreinte, apercu_nb, apercu_sms, apercu_le, lancee_par, lancee_le, \
    nb_destinataires, created_at";

#[derive(Debug, Clone, Deserialize)]
pub struct NouvelleCampagne {
    pub titre: String,
    pub corps: String,
    pub canaux: Vec<String>,
    pub audience: String,
    #[serde(default)]
    pub cible_ids: Vec<Uuid>,
}

/// Un destinataire tel que montré dans l'aperçu : jamais d'adresse ni de numéro.
#[derive(Debug, Clone, Serialize)]
pub struct DestinataireApercu {
    pub nom: String,
    pub nb_eleves: i32,
    pub canaux: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EstimationSms {
    pub destinataires: usize,
    /// Segments par message (160 caractères, puis 153 par segment).
    pub segments_par_message: usize,
    pub credits_estimes: usize,
    pub plafond_campagne: i32,
    pub plafond_jour_restant: i64,
    pub depasse: bool,
    pub tronque: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApercuCampagne {
    pub campagne_id: Uuid,
    pub nb_destinataires: usize,
    pub nb_eleves: usize,
    pub par_canal: BTreeMap<String, usize>,
    /// Destinataires qu'aucun des canaux choisis ne peut joindre (hors ENT).
    pub sans_canal_externe: usize,
    /// Élèves de l'audience que les droits de l'auteur ne couvrent pas.
    pub exclus_hors_droits: usize,
    pub sms: Option<EstimationSms>,
    pub empreinte: String,
    pub destinataires: Vec<DestinataireApercu>,
    pub tronque: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfirmationLancement {
    pub empreinte: String,
    pub nb_destinataires: usize,
    /// Seconde confirmation, obligatoire si des SMS partent.
    #[serde(default)]
    pub confirme_sms: bool,
}

// ---------------------------------------------------------------------------
// Ciblage (partagé avec les formulaires et les rencontres)
// ---------------------------------------------------------------------------

/// Élèves actuellement affectés à l'audience (sans filtre de droits).
pub(crate) async fn eleves_de_audience(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    audience: &str,
    cible_ids: &[Uuid],
    jour: NaiveDate,
) -> Result<BTreeSet<Uuid>, EntDbError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT a.eleve_id
           FROM ent_affectation a
           JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
          WHERE a.etablissement_id = $1
            AND a.date_debut <= $2 AND (a.date_fin IS NULL OR a.date_fin >= $2)
            AND (e.date_sortie IS NULL OR e.date_sortie > $2)
            AND ($3 = 'etablissement'
                 OR ($3 = 'classe' AND a.classe_id = ANY($4))
                 OR ($3 = 'groupe' AND a.groupe_id = ANY($4))
                 OR ($3 = 'niveau' AND a.classe_id IN (
                        SELECT c.id FROM ent_classe c
                         WHERE c.etablissement_id = $1 AND c.niveau_id = ANY($4))))",
    )
    .bind(etab)
    .bind(jour)
    .bind(audience)
    .bind(cible_ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Vérifie que l'audience désigne des objets de l'établissement.
pub(crate) async fn valider_audience(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    audience: &str,
    cible_ids: &[Uuid],
) -> Result<(), EntDbError> {
    if !AUDIENCES.contains(&audience) {
        return Err(EntDbError::SaisieInvalide("audience inconnue"));
    }
    if (audience == "etablissement") != cible_ids.is_empty() {
        return Err(EntDbError::SaisieInvalide(
            "l'audience « établissement » n'a pas de cible, les autres en exigent",
        ));
    }
    if cible_ids.len() > 200 {
        return Err(EntDbError::SaisieInvalide("trop de cibles"));
    }
    if cible_ids.is_empty() {
        return Ok(());
    }
    let table = match audience {
        "niveau" => "ent_niveau",
        "classe" => "ent_classe",
        _ => "ent_groupe",
    };
    let distincts: Vec<Uuid> = cible_ids.iter().copied().collect::<BTreeSet<_>>().into_iter().collect();
    let (n,): (i64,) = sqlx::query_as(&format!(
        "SELECT count(*) FROM {table} WHERE etablissement_id = $1 AND id = ANY($2)"
    ))
    .bind(etab)
    .bind(&distincts)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if n as usize != distincts.len() {
        return Err(EntDbError::SaisieInvalide("cible inconnue dans cet établissement"));
    }
    Ok(())
}

/// Un responsable à joindre : lien actif avec autorité, compte « parent » de l'établissement.
#[derive(Debug, Clone)]
pub(crate) struct Responsable {
    pub user_id: Uuid,
    pub nom: String,
    pub eleve_id: Uuid,
    pub nb_eleves: i32,
    pub pref: Preferences,
    pub a_email: bool,
    pub a_telephone: bool,
    pub a_push: bool,
}

pub(crate) async fn responsables_de(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    eleves: &[Uuid],
) -> Result<Vec<Responsable>, EntDbError> {
    type Ligne = (Uuid, Option<String>, Uuid, i64, String, Option<String>, bool, bool, bool, String, Option<i16>, Option<i16>, bool);
    let rows: Vec<Ligne> = sqlx::query_as(
        "SELECT r.user_id, u.display_name, r.eleve_id, r.nb, u.email, u.telephone,
                COALESCE(p.canal_email, true), COALESCE(p.canal_sms, false),
                COALESCE(p.canal_push, true), COALESCE(p.resume, 'immediat'),
                p.calme_debut, p.calme_fin,
                EXISTS (SELECT 1 FROM ent_push_device d
                         WHERE d.etablissement_id = $1 AND d.user_id = r.user_id
                           AND d.revoked_at IS NULL)
           FROM (
                SELECT l.user_id, (array_agg(l.eleve_id ORDER BY l.eleve_id))[1] AS eleve_id,
                       count(DISTINCT l.eleve_id) AS nb
                  FROM ent_eleve_lien_actif l
                  JOIN ent_membership m
                    ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
                 WHERE l.etablissement_id = $1 AND l.eleve_id = ANY($2)
                   AND l.user_id IS NOT NULL AND l.autorite_parentale AND m.role = 'parent'
                 GROUP BY l.user_id) r
           JOIN ent_user u ON u.id = r.user_id
           LEFT JOIN ent_preference_notification p
             ON p.etablissement_id = $1 AND p.user_id = r.user_id
          ORDER BY u.display_name NULLS LAST, r.user_id",
    )
    .bind(etab)
    .bind(eleves)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|l| Responsable {
            user_id: l.0,
            nom: l.1.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "(sans nom)".into()),
            eleve_id: l.2,
            nb_eleves: l.3 as i32,
            a_email: !l.4.trim().is_empty(),
            a_telephone: l.5.as_deref().is_some_and(|t| !ent_sms::normalize_phone(t).is_empty()),
            pref: Preferences {
                email: l.6,
                sms: l.7,
                push: l.8,
                resume: l.9,
                calme_debut: l.10.map(|v| v as u16),
                calme_fin: l.11.map(|v| v as u16),
            },
            a_push: l.12,
        })
        .collect())
}

struct Calcul {
    responsables: Vec<Responsable>,
    nb_eleves: usize,
    exclus_hors_droits: usize,
    empreinte: String,
}

fn empreinte_de(c: &Campagne, user_ids: &BTreeSet<Uuid>) -> String {
    let mut canaux = c.canaux.clone();
    canaux.sort();
    let mut cibles = c.cible_ids.clone();
    cibles.sort();
    sha256_hex(&format!(
        "{}|{:?}|{:?}|{}|{:?}",
        c.audience,
        cibles,
        canaux,
        sha256_hex(&format!("{}\n{}", c.titre, c.corps)),
        user_ids
    ))
}

impl PgEnt {
    /// Droit d'agir en campagne + élèves que l'auteur peut atteindre
    /// (`None` : tout l'établissement).
    async fn perimetre_campagne(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Option<HashSet<Uuid>>, EntDbError> {
        self.exiger_action_quelconque(etab, acteur, role, Action::CampagnesLancer)
            .await?;
        self.eleves_autorises(etab, acteur, role, Action::CampagnesLancer)
            .await
    }

    async fn calculer_destinataires(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        c: &Campagne,
    ) -> Result<Calcul, EntDbError> {
        let perimetre = self.perimetre_campagne(etab, acteur, role).await?;
        if c.audience == "etablissement" && perimetre.is_some() {
            // Un rôle limité à des classes ne peut pas écrire à tout l'établissement.
            return Err(EntDbError::Forbidden);
        }
        let mut conn = self.pool().acquire().await.map_err(db)?;
        let jour = Utc::now().date_naive();
        let cibles = eleves_de_audience(&mut conn, etab, &c.audience, &c.cible_ids, jour).await?;
        let total = cibles.len();
        let autorises: Vec<Uuid> = match &perimetre {
            None => cibles.into_iter().collect(),
            Some(p) => cibles.into_iter().filter(|e| p.contains(e)).collect(),
        };
        let exclus = total - autorises.len();
        let responsables = responsables_de(&mut conn, etab, &autorises).await?;
        let ids: BTreeSet<Uuid> = responsables.iter().map(|r| r.user_id).collect();
        let empreinte = empreinte_de(c, &ids);
        Ok(Calcul {
            responsables,
            nb_eleves: autorises.len(),
            exclus_hors_droits: exclus,
            empreinte,
        })
    }

    async fn campagne_visible(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<Campagne, EntDbError> {
        let c: Option<Campagne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_CAMPAGNE} FROM ent_campagne WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let c = c.ok_or(EntDbError::Domain(domain_eleve::DomainError::NotFound))?;
        if c.cree_par != acteur
            && !self
                .a_action_etablissement(etab, acteur, role, Action::NotificationsGerer)
                .await?
        {
            // Même réponse qu'une campagne inexistante : pas de fuite d'existence.
            return Err(EntDbError::Domain(domain_eleve::DomainError::NotFound));
        }
        Ok(c)
    }

    /// Crée un brouillon de campagne.
    pub async fn creer_campagne(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        n: NouvelleCampagne,
    ) -> Result<Campagne, EntDbError> {
        let perimetre = self.perimetre_campagne(etab, acteur, role).await?;
        if n.audience == "etablissement" && perimetre.is_some() {
            return Err(EntDbError::Forbidden);
        }
        let titre = nettoyer(&n.titre, 200, "titre invalide")?;
        let corps = nettoyer(&n.corps, 5000, "texte invalide")?;
        let canaux: BTreeSet<String> = n.canaux.iter().map(|c| c.trim().to_string()).collect();
        if canaux.is_empty() || canaux.iter().any(|c| !["email", "sms", "push"].contains(&c.as_str())) {
            return Err(EntDbError::SaisieInvalide("canaux invalides (email, sms, push)"));
        }
        let mut cibles: Vec<Uuid> = n.cible_ids.clone();
        cibles.sort();
        cibles.dedup();
        let mut conn = self.pool().acquire().await.map_err(db)?;
        valider_audience(&mut conn, etab, &n.audience, &cibles).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_campagne
                (id, etablissement_id, titre, corps, canaux, audience, cible_ids, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(etab)
        .bind(&titre)
        .bind(&corps)
        .bind(canaux.into_iter().collect::<Vec<_>>())
        .bind(&n.audience)
        .bind(&cibles)
        .bind(acteur)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        drop(conn);
        self.campagne_visible(etab, acteur, role, id).await
    }

    /// Campagnes de l'auteur ; toutes celles de l'établissement pour `notifications.gerer`.
    pub async fn campagnes_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<Campagne>, EntDbError> {
        self.exiger_action_quelconque(etab, acteur, role, Action::CampagnesLancer)
            .await?;
        let toutes = self
            .a_action_etablissement(etab, acteur, role, Action::NotificationsGerer)
            .await?;
        sqlx::query_as(&format!(
            "SELECT {COLONNES_CAMPAGNE} FROM ent_campagne
              WHERE etablissement_id = $1 AND ($3 OR cree_par = $2)
              ORDER BY created_at DESC LIMIT 200"
        ))
        .bind(etab)
        .bind(acteur)
        .bind(toutes)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    pub async fn campagne_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<Campagne, EntDbError> {
        self.exiger_action_quelconque(etab, acteur, role, Action::CampagnesLancer)
            .await?;
        self.campagne_visible(etab, acteur, role, id).await
    }

    /// Abandonne une campagne non lancée.
    pub async fn annuler_campagne(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<Campagne, EntDbError> {
        self.exiger_action_quelconque(etab, acteur, role, Action::CampagnesLancer)
            .await?;
        let c = self.campagne_visible(etab, acteur, role, id).await?;
        if c.cree_par != acteur {
            return Err(EntDbError::Forbidden);
        }
        let r = sqlx::query(
            "UPDATE ent_campagne SET statut = 'annulee', updated_at = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut IN ('brouillon', 'apercu')",
        )
        .bind(etab)
        .bind(id)
        .execute(self.pool())
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier("la campagne n'est plus annulable"));
        }
        self.campagne_visible(etab, acteur, role, id).await
    }

    /// Aperçu des destinataires (COM-05) : calcule, mémorise l'empreinte, ne
    /// bloque ni n'envoie rien.
    pub async fn apercu_campagne(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ApercuCampagne, EntDbError> {
        self.exiger_action_quelconque(etab, acteur, role, Action::CampagnesLancer)
            .await?;
        let c = self.campagne_visible(etab, acteur, role, id).await?;
        if c.cree_par != acteur {
            return Err(EntDbError::Forbidden);
        }
        if !matches!(c.statut.as_str(), "brouillon" | "apercu") {
            return Err(EntDbError::ConflitDossier("la campagne n'est plus modifiable"));
        }
        let calcul = self.calculer_destinataires(etab, acteur, role, &c).await?;
        let voulus: Vec<&str> = c.canaux.iter().map(String::as_str).collect();
        let mut par_canal: BTreeMap<String, usize> = BTreeMap::new();
        let mut sans_canal = 0usize;
        let mut vus = Vec::new();
        for r in &calcul.responsables {
            let canaux = canaux_disponibles(&r.pref, &voulus, r.a_email, r.a_telephone, r.a_push);
            if canaux.is_empty() {
                sans_canal += 1;
            }
            for k in &canaux {
                *par_canal.entry((*k).to_string()).or_default() += 1;
            }
            vus.push(DestinataireApercu {
                nom: r.nom.clone(),
                nb_eleves: r.nb_eleves,
                canaux: canaux.iter().map(|k| k.to_string()).collect(),
            });
        }
        let nb_sms = par_canal.get("sms").copied().unwrap_or(0);
        let sms = if c.canaux.iter().any(|k| k == "sms") {
            let mut conn = self.pool().acquire().await.map_err(db)?;
            let plafonds = plafonds_de(&mut conn, etab).await?;
            let deja = sms_du_jour(&mut conn, etab).await?;
            let longueur = format!("{} : {}", c.titre, c.corps).chars().count();
            let segments = if longueur <= 160 { 1 } else { longueur.div_ceil(153) };
            let restant = (i64::from(plafonds.sms_par_jour) - deja).max(0);
            Some(EstimationSms {
                destinataires: nb_sms,
                segments_par_message: segments.min(2),
                credits_estimes: nb_sms * segments.min(2),
                plafond_campagne: plafonds.sms_par_campagne,
                plafond_jour_restant: restant,
                depasse: nb_sms as i64 > i64::from(plafonds.sms_par_campagne)
                    || nb_sms as i64 > restant,
                tronque: longueur > 306,
            })
        } else {
            None
        };
        sqlx::query(
            "UPDATE ent_campagne
                SET statut = 'apercu', apercu_empreinte = $3, apercu_nb = $4, apercu_sms = $5,
                    apercu_le = now(), updated_at = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut IN ('brouillon', 'apercu')",
        )
        .bind(etab)
        .bind(id)
        .bind(&calcul.empreinte)
        .bind(calcul.responsables.len() as i32)
        .bind(nb_sms as i32)
        .execute(self.pool())
        .await
        .map_err(db)?;
        let tronque = vus.len() > 200;
        vus.truncate(200);
        Ok(ApercuCampagne {
            campagne_id: id,
            nb_destinataires: calcul.responsables.len(),
            nb_eleves: calcul.nb_eleves,
            par_canal,
            sans_canal_externe: sans_canal,
            exclus_hors_droits: calcul.exclus_hors_droits,
            sms,
            empreinte: calcul.empreinte,
            destinataires: vus,
            tronque,
        })
    }

    /// Lance la campagne **après confirmation humaine** de l'aperçu.
    pub async fn lancer_campagne(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        conf: ConfirmationLancement,
    ) -> Result<Campagne, EntDbError> {
        self.exiger_action_quelconque(etab, acteur, role, Action::CampagnesLancer)
            .await?;
        let c = self.campagne_visible(etab, acteur, role, id).await?;
        if c.cree_par != acteur {
            return Err(EntDbError::Forbidden);
        }
        match c.statut.as_str() {
            "apercu" => {}
            "lancee" => return Err(EntDbError::ConflitDossier("campagne déjà lancée")),
            "brouillon" => {
                return Err(EntDbError::SaisieInvalide("aperçu des destinataires requis avant le lancement"))
            }
            _ => return Err(EntDbError::ConflitDossier("la campagne n'est plus lançable")),
        }
        // Recalcul : les droits et les liens sont relus MAINTENANT.
        let calcul = self.calculer_destinataires(etab, acteur, role, &c).await?;
        if conf.empreinte != calcul.empreinte
            || c.apercu_empreinte.as_deref() != Some(calcul.empreinte.as_str())
            || conf.nb_destinataires != calcul.responsables.len()
        {
            return Err(EntDbError::ConflitDossier(
                "les destinataires ont changé depuis l'aperçu : refaire l'aperçu",
            ));
        }
        if calcul.responsables.is_empty() {
            return Err(EntDbError::SaisieInvalide("aucun destinataire"));
        }
        let voulus: Vec<&str> = c.canaux.iter().map(String::as_str).collect();
        let nb_sms = calcul
            .responsables
            .iter()
            .filter(|r| {
                canaux_disponibles(&r.pref, &voulus, r.a_email, r.a_telephone, r.a_push)
                    .contains(&"sms")
            })
            .count();

        let mut tx = self.pool().begin().await.map_err(db)?;
        // Verrou : deux lancements simultanés de la même campagne se sérialisent.
        let statut: Option<(String,)> = sqlx::query_as(
            "SELECT statut FROM ent_campagne WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        match statut.as_ref().map(|s| s.0.as_str()) {
            Some("apercu") => {}
            Some("lancee") => return Err(EntDbError::ConflitDossier("campagne déjà lancée")),
            _ => return Err(EntDbError::ConflitDossier("la campagne n'est plus lançable")),
        }
        let plafonds = plafonds_de(&mut tx, etab).await?;
        let (lancees_jour,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_campagne
              WHERE etablissement_id = $1 AND statut = 'lancee'
                AND lancee_le >= date_trunc('day', now() AT TIME ZONE 'Europe/Paris')
                                 AT TIME ZONE 'Europe/Paris'",
        )
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if lancees_jour >= i64::from(plafonds.campagnes_par_jour) {
            return Err(EntDbError::ConflitDossier("plafond de campagnes du jour atteint"));
        }
        if nb_sms > 0 {
            if !conf.confirme_sms {
                return Err(EntDbError::SaisieInvalide(
                    "double confirmation requise : des SMS vont partir",
                ));
            }
            if nb_sms as i64 > i64::from(plafonds.sms_par_campagne) {
                return Err(EntDbError::ConflitDossier("plafond de SMS par campagne dépassé"));
            }
            let deja = sms_du_jour(&mut tx, etab).await?;
            if deja + nb_sms as i64 > i64::from(plafonds.sms_par_jour) {
                return Err(EntDbError::ConflitDossier("plafond de SMS du jour dépassé"));
            }
        }
        let evt = tx
            .publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_com::CAMPAGNE_LANCEE.to_string(),
                // Agrégat « Annonce » réutilisé (pas de variante dédiée tant que
                // l'énumération est partagée entre lots en parallèle).
                aggregate: AggregateKind::Annonce,
                aggregate_id: id,
                eleve_id: None,
                payload: json!({
                    "campagne_id": id,
                    "audience": c.audience,
                    "canaux": c.canaux,
                    "nb_destinataires": calcul.responsables.len(),
                }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        let maintenant = Utc::now();
        for r in &calcul.responsables {
            sqlx::query(
                "INSERT INTO ent_campagne_destinataire
                    (campagne_id, etablissement_id, user_id, eleve_id, nb_eleves)
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(id)
            .bind(etab)
            .bind(r.user_id)
            .bind(r.eleve_id)
            .bind(r.nb_eleves)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            // Dans l'ENT (toujours) puis canaux externes selon les préférences.
            notifier_personne(
                &mut tx,
                EnvoiAPlanifier {
                    etablissement_id: etab,
                    user_id: r.user_id,
                    kind: "campagne",
                    titre: &c.titre,
                    corps: &c.corps,
                    eleve_id: Some(r.eleve_id),
                    notification_id: None,
                    source_type: "campagne",
                    source_id: id,
                    cle_dedup: format!("camp:{id}"),
                    droit_requis: Some("lien"),
                    regroupable: true,
                    canaux_voulus: &voulus,
                },
                evt.id,
                maintenant,
            )
            .await?;
        }
        sqlx::query(
            "UPDATE ent_campagne
                SET statut = 'lancee', lancee_par = $3, lancee_le = now(), nb_destinataires = $4,
                    updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(acteur)
        .bind(calcul.responsables.len() as i32)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        self.campagne_visible(etab, acteur, role, id).await
    }
}

/// SMS déjà planifiés ou envoyés aujourd'hui (heure de Paris) dans l'établissement.
async fn sms_du_jour(conn: &mut sqlx::PgConnection, etab: Uuid) -> Result<i64, EntDbError> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_notification_envoi
          WHERE etablissement_id = $1 AND canal = 'sms'
            AND ((statut = 'en_attente' AND created_at >= date_trunc('day', now() AT TIME ZONE 'Europe/Paris')
                                                         AT TIME ZONE 'Europe/Paris')
                 OR (plafond_reserve AND tentative_le >= date_trunc('day', now() AT TIME ZONE 'Europe/Paris')
                                                         AT TIME ZONE 'Europe/Paris'))",
    )
    .bind(etab)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camp() -> Campagne {
        Campagne {
            id: Uuid::nil(),
            titre: "Réunion".into(),
            corps: "Jeudi 18 h".into(),
            canaux: vec!["email".into(), "sms".into()],
            audience: "classe".into(),
            cible_ids: vec![Uuid::from_u128(2), Uuid::from_u128(1)],
            statut: "brouillon".into(),
            cree_par: Uuid::nil(),
            apercu_empreinte: None,
            apercu_nb: None,
            apercu_sms: None,
            apercu_le: None,
            lancee_par: None,
            lancee_le: None,
            nb_destinataires: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn empreinte_stable_et_sensible() {
        let a: BTreeSet<Uuid> = [Uuid::from_u128(10), Uuid::from_u128(11)].into();
        let b: BTreeSet<Uuid> = [Uuid::from_u128(10)].into();
        let c = camp();
        assert_eq!(empreinte_de(&c, &a), empreinte_de(&c, &a));
        assert_ne!(empreinte_de(&c, &a), empreinte_de(&c, &b), "un destinataire en moins");
        let mut d = camp();
        d.corps = "Jeudi 19 h".into();
        assert_ne!(empreinte_de(&c, &a), empreinte_de(&d, &a), "texte modifié");
        let mut e = camp();
        e.canaux = vec!["sms".into(), "email".into()];
        assert_eq!(empreinte_de(&c, &a), empreinte_de(&e, &a), "ordre des canaux indifférent");
    }

    #[test]
    fn sha256_connu() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
