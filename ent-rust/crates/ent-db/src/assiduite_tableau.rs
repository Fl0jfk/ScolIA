//! Socle 12, lot AS14b-bis : tableau de bord de la vie scolaire en panneaux (CPE, AED).
//!
//! Ce module n'ajoute que ce qui manquait à la file du matin (AS6, `file_du_matin_for_actor`) et aux écrans AS14b :
//! - **les exclus de cours du jour** pas encore arrivés à la vie scolaire (la file du matin ne montre un exclu qu'après
//!   [`MINUTES_NON_ARRIVEE`](super::assiduite_as6) minutes, comme alerte « urgente ») ;
//! - **l'heure de bascule** (réglage d'établissement, 11 h par défaut, heure locale) : avant elle le panneau des
//!   absents passe en premier, après ce sont les exclus et les appels non faits ;
//! - **la disposition personnelle** de chaque utilisateur (ordre épinglé, panneaux repliés ou masqués), mémorisée côté
//!   serveur. Aucune ligne = disposition automatique.
//!
//! Le regroupement par panneau (qui range quelle ligne de la file dans quel panneau) est **pur** et vit dans le
//! frontend (`ent-web-app`, module `tableau_vs`) : il ne décide aucun droit, il présente. Aucune donnée d'élève ici
//! hors identité minimale (nom, prénom, classe) ; ni motif, ni note interne, ni texte libre (R-AS22).

use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::vie_scolaire::{db, invalide};
use chrono::{DateTime, NaiveTime, Utc};
use ent_auth::Action;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Les panneaux connus (liste fermée, identique à la contrainte de `ent_asd_tableau_pref`).
pub const PANNEAUX_TABLEAU: [&str; 5] = ["absents", "exclus", "appels", "incoherences", "familles"];

/// Heure de bascule par défaut (heure locale de l'établissement).
pub const HEURE_BASCULE_DEFAUT: (u32, u32) = (11, 0);

/// Minutes sans arrivée au-delà desquelles l'exclu est déjà dans le bandeau « Urgent » (R-AS37) : même valeur que la
/// file du matin (`assiduite_as6`), répétée pour marquer la ligne dans le panneau.
const MINUTES_URGENT: i64 = 10;

/// Un élève exclu de cours ce jour, dont l'arrivée à la vie scolaire n'est pas confirmée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExcluDuJour {
    pub appel_id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub mineur: bool,
    /// Heure locale de l'exclusion.
    pub depuis: NaiveTime,
    pub minutes_depuis: i64,
    /// Déjà dans le bandeau « Urgent » (au moins 10 minutes sans arrivée).
    pub urgent: bool,
    pub creneau_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExclusDuJour {
    pub heure_locale: NaiveTime,
    pub exclus: Vec<ExcluDuJour>,
}

/// Disposition d'un panneau choisie par l'utilisateur : la position dans la liste donne le rang.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreferencePanneau {
    pub panneau: String,
    pub replie: bool,
    pub masque: bool,
}

/// Réglage de l'établissement et disposition de l'utilisateur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigTableau {
    pub heure_bascule: NaiveTime,
    /// `None` = mode automatique ; `Some` = ordre épinglé (rang = position), tous les panneaux listés.
    pub preference: Option<Vec<PreferencePanneau>>,
}

#[derive(sqlx::FromRow)]
struct LigneExclu {
    appel_id: Uuid,
    eleve_id: Uuid,
    nom: String,
    prenom: String,
    classe: Option<String>,
    mineur: bool,
    depuis: NaiveTime,
    minutes: i64,
    creneau_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct LignePref {
    panneau: String,
    replie: bool,
    masque: bool,
}

/// Heure de bascule par défaut du code (11 h).
pub fn heure_bascule_defaut() -> NaiveTime {
    NaiveTime::from_hms_opt(HEURE_BASCULE_DEFAUT.0, HEURE_BASCULE_DEFAUT.1, 0).unwrap_or_default()
}

/// L'heure de bascule doit tenir dans la journée scolaire (6 h à 18 h, comme la contrainte de la base).
pub fn heure_bascule_valide(h: NaiveTime) -> bool {
    let min = NaiveTime::from_hms_opt(6, 0, 0).unwrap_or_default();
    let max = NaiveTime::from_hms_opt(18, 0, 0).unwrap_or_default();
    h >= min && h <= max
}

/// Une disposition valide liste **chacun** des cinq panneaux **une seule fois** (le rang est la position).
pub fn preference_valide(p: &[PreferencePanneau]) -> bool {
    p.len() == PANNEAUX_TABLEAU.len()
        && PANNEAUX_TABLEAU.iter().all(|n| p.iter().filter(|x| x.panneau == *n).count() == 1)
}

impl PgEnt {
    /// Exclus de cours du jour dont l'arrivée n'est pas confirmée (`absences.lire` et `appel.verifier`, comme la file
    /// du matin). Plus ancien d'abord. Aucun motif, aucune punition : l'exclusion est un fait d'appel.
    pub async fn exclus_du_jour_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<ExclusDuJour, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AbsencesLire).await?;
        self.exiger_action_etab(etab, acteur, role, Action::AppelVerifier).await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let heure: NaiveTime = sqlx::query_scalar("SELECT ent_cdt_heure_locale($1, $2)")
            .bind(etab)
            .bind(maintenant)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let lignes: Vec<LigneExclu> = sqlx::query_as(
            "SELECT l.appel_id, l.eleve_id, e.nom, e.prenom, e.classe,
                    COALESCE(e.date_naissance + INTERVAL '18 years' > $2, true) AS mineur,
                    ent_cdt_heure_locale(l.etablissement_id, l.exclusion_cours_le) AS depuis,
                    GREATEST(0, EXTRACT(EPOCH FROM ($3 - l.exclusion_cours_le)) / 60)::bigint AS minutes,
                    a.creneau_id
               FROM ent_appel_ligne l
               JOIN ent_appel a ON a.etablissement_id = l.etablissement_id AND a.id = l.appel_id
               JOIN ent_eleve e ON e.etablissement_id = l.etablissement_id AND e.id = l.eleve_id
              WHERE l.etablissement_id = $1 AND a.jour = $2 AND a.etat <> 'neutralise'
                AND l.exclusion_cours_le IS NOT NULL AND l.arrivee_vs_confirmee_le IS NULL
              ORDER BY l.exclusion_cours_le, l.id",
        )
        .bind(etab)
        .bind(jour)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let exclus = lignes
            .into_iter()
            .map(|l| ExcluDuJour {
                appel_id: l.appel_id,
                eleve_id: l.eleve_id,
                nom: l.nom,
                prenom: l.prenom,
                classe: l.classe,
                mineur: l.mineur,
                depuis: l.depuis,
                minutes_depuis: l.minutes,
                urgent: l.minutes >= MINUTES_URGENT,
                creneau_id: l.creneau_id,
            })
            .collect();
        Ok(ExclusDuJour {
            heure_locale: heure,
            exclus,
        })
    }

    /// Heure de bascule de l'établissement et disposition de l'utilisateur (mêmes droits que la file du matin).
    pub async fn config_tableau_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<ConfigTableau, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AbsencesLire).await?;
        self.exiger_action_etab(etab, acteur, role, Action::AppelVerifier).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let heure: Option<NaiveTime> =
            sqlx::query_scalar("SELECT heure_bascule FROM ent_asd_tableau_reglage WHERE etablissement_id = $1")
                .bind(etab)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let lignes: Vec<LignePref> = sqlx::query_as(
            "SELECT panneau, replie, masque FROM ent_asd_tableau_pref
              WHERE etablissement_id = $1 AND user_id = $2 ORDER BY rang",
        )
        .bind(etab)
        .bind(acteur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let preference = if lignes.is_empty() {
            None
        } else {
            Some(
                lignes
                    .into_iter()
                    .map(|l| PreferencePanneau {
                        panneau: l.panneau,
                        replie: l.replie,
                        masque: l.masque,
                    })
                    .collect(),
            )
        };
        Ok(ConfigTableau {
            heure_bascule: heure.unwrap_or_else(heure_bascule_defaut),
            preference,
        })
    }

    /// Règle l'heure de bascule de l'établissement (`assiduite.regle_gerer`, comme la règle d'assiduité).
    pub async fn regler_heure_bascule_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        heure: NaiveTime,
    ) -> Result<NaiveTime, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteRegleGerer).await?;
        if !heure_bascule_valide(heure) {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_asd_tableau_reglage (id, etablissement_id, heure_bascule, modifie_par)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (etablissement_id)
             DO UPDATE SET heure_bascule = EXCLUDED.heure_bascule, modifie_par = EXCLUDED.modifie_par,
                           modifie_le = now()",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(heure)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(heure)
    }

    /// Épingle la disposition de l'utilisateur connecté (la sienne, jamais celle d'un autre). Droits d'accès au
    /// tableau : un utilisateur qui ne voit pas la file n'a rien à ranger.
    pub async fn poser_preference_tableau_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        panneaux: Vec<PreferencePanneau>,
    ) -> Result<Vec<PreferencePanneau>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AbsencesLire).await?;
        self.exiger_action_etab(etab, acteur, role, Action::AppelVerifier).await?;
        if !preference_valide(&panneaux) {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query("DELETE FROM ent_asd_tableau_pref WHERE etablissement_id = $1 AND user_id = $2")
            .bind(etab)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        for (i, p) in panneaux.iter().enumerate() {
            sqlx::query(
                "INSERT INTO ent_asd_tableau_pref (id, etablissement_id, user_id, panneau, rang, replie, masque)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(acteur)
            .bind(&p.panneau)
            .bind(i16::try_from(i + 1).unwrap_or(1))
            .bind(p.replie)
            .bind(p.masque)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(panneaux)
    }

    /// Revient au mode automatique : supprime les lignes de l'utilisateur connecté. Idempotent.
    pub async fn effacer_preference_tableau_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AbsencesLire).await?;
        self.exiger_action_etab(etab, acteur, role, Action::AppelVerifier).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query("DELETE FROM ent_asd_tableau_pref WHERE etablissement_id = $1 AND user_id = $2")
            .bind(etab)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pref(n: &str) -> PreferencePanneau {
        PreferencePanneau {
            panneau: n.into(),
            replie: false,
            masque: false,
        }
    }

    #[test]
    fn heure_bascule_bornee() {
        let h = |a, b| NaiveTime::from_hms_opt(a, b, 0).unwrap_or_default();
        assert_eq!(heure_bascule_defaut(), h(11, 0));
        assert!(heure_bascule_valide(h(6, 0)) && heure_bascule_valide(h(18, 0)) && heure_bascule_valide(h(11, 30)));
        assert!(!heure_bascule_valide(h(5, 59)) && !heure_bascule_valide(h(18, 1)));
    }

    #[test]
    fn preference_exige_les_cinq_panneaux_une_fois() {
        let tous: Vec<_> = PANNEAUX_TABLEAU.iter().map(|n| pref(n)).collect();
        assert!(preference_valide(&tous));
        assert!(!preference_valide(&tous[..4]));
        let mut doublon = tous.clone();
        doublon[4] = pref("absents");
        assert!(!preference_valide(&doublon));
        let mut inconnu = tous;
        inconnu[0] = pref("urgent");
        assert!(!preference_valide(&inconnu), "le bandeau Urgent n'est pas un panneau et ne se range pas");
    }
}
