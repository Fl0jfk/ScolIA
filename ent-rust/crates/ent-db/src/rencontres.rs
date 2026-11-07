//! Rendez-vous et rencontres parents-professeurs (socle 2, lot S2-8, D21 :
//! AGD-02, AGD-03).
//!
//! Une **rencontre** (`brouillon` → `ouverte` → `close`) concerne une classe.
//! L'organisateur (`rencontres.gerer`) propose des **créneaux** par enseignant
//! de cette classe. Une famille réserve un créneau **pour un de ses enfants**
//! de la classe, auprès d'un enseignant qui l'a en cours. Règles :
//! - un seul rendez-vous par enseignant et par enfant (index unique) ;
//! - un responsable ne réserve pas deux créneaux qui se chevauchent ;
//! - un créneau ne se réserve qu'une fois (mise à jour conditionnelle : deux
//!   familles simultanées → l'une reçoit un conflit) ;
//! - seul un responsable avec **lien actif et autorité parentale** réserve ;
//! - l'enseignant d'un créneau doit avoir la classe en service.
//!
//! Les rappels sont émis par [`PgEnt::emettre_rappels_rdv`] (idempotents), via le
//! domaine Notifications (préférences de chacun). La visio (COM-09) n'est pas
//! livrée : service externe non décidé.

use crate::campagnes::{eleves_de_audience, types_com};
use crate::notifications::{db, notifier_personne, EnvoiAPlanifier};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use chrono::{DateTime, Duration, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

fn introuvable() -> EntDbError {
    EntDbError::Domain(domain_eleve::DomainError::NotFound)
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Rencontre {
    pub id: Uuid,
    pub titre: String,
    pub classe_id: Uuid,
    pub classe_libelle: String,
    pub statut: String,
    pub nb_creneaux: i64,
    pub nb_reserves: i64,
    pub created_at: DateTime<Utc>,
}

const SELECT_RENCONTRE: &str = "SELECT r.id, r.titre, r.classe_id, c.libelle AS classe_libelle,
        r.statut,
        (SELECT count(*) FROM ent_creneau_rdv k
          WHERE k.etablissement_id = r.etablissement_id AND k.rencontre_id = r.id) AS nb_creneaux,
        (SELECT count(*) FROM ent_creneau_rdv k
          WHERE k.etablissement_id = r.etablissement_id AND k.rencontre_id = r.id
            AND k.statut = 'reserve') AS nb_reserves,
        r.created_at
   FROM ent_rencontre r
   JOIN ent_classe c ON c.id = r.classe_id AND c.etablissement_id = r.etablissement_id";

#[derive(Debug, Clone, Deserialize)]
pub struct NouvelleRencontre {
    pub titre: String,
    pub classe_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NouveauxCreneaux {
    pub enseignant_user_id: Uuid,
    pub debut: DateTime<Utc>,
    pub duree_minutes: i32,
    pub nombre: i32,
}

/// Créneau tel que vu par une famille : ni le nom ni l'enfant d'une autre famille.
#[derive(Debug, Clone, Serialize)]
pub struct CreneauVu {
    pub id: Uuid,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    /// `libre`, `reserve_par_moi` ou `indisponible` (pris par une autre famille).
    pub etat: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnseignantCreneaux {
    pub enseignant_user_id: Uuid,
    pub nom: String,
    pub matieres: String,
    pub creneaux: Vec<CreneauVu>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LignePlanning {
    pub creneau_id: Uuid,
    pub rencontre_id: Uuid,
    pub enseignant_user_id: Uuid,
    pub enseignant_nom: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub statut: String,
    pub eleve_prenom: Option<String>,
    pub eleve_nom: Option<String>,
    pub responsable_nom: Option<String>,
}

const SELECT_PLANNING: &str = "SELECT k.id AS creneau_id, k.rencontre_id, k.enseignant_user_id,
        u.display_name AS enseignant_nom, k.debut, k.fin, k.statut,
        e.prenom AS eleve_prenom, e.nom AS eleve_nom, p.display_name AS responsable_nom
   FROM ent_creneau_rdv k
   JOIN ent_user u ON u.id = k.enseignant_user_id
   LEFT JOIN ent_eleve e ON e.id = k.eleve_id AND e.etablissement_id = k.etablissement_id
   LEFT JOIN ent_user p ON p.id = k.reserve_par_user_id";

/// Enfant d'une famille concerné par une rencontre ouverte.
#[derive(Debug, Clone, Serialize)]
pub struct RencontreFamille {
    pub rencontre_id: Uuid,
    pub titre: String,
    pub classe_libelle: String,
    pub eleve_id: Uuid,
    pub eleve_prenom: String,
}

impl PgEnt {
    async fn exiger_rencontres(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::RencontresGerer).await
    }

    pub async fn creer_rencontre(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        n: NouvelleRencontre,
    ) -> Result<Rencontre, EntDbError> {
        self.exiger_rencontres(etab, acteur, role).await?;
        let titre = n.titre.trim().to_string();
        if titre.is_empty() || titre.chars().count() > 200 {
            return Err(EntDbError::SaisieInvalide("titre invalide"));
        }
        let ok: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM ent_classe WHERE etablissement_id = $1 AND id = $2 AND archivee_le IS NULL",
        )
        .bind(etab)
        .bind(n.classe_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        if ok.is_none() {
            return Err(EntDbError::SaisieInvalide("classe inconnue dans cet établissement"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_rencontre (id, etablissement_id, titre, classe_id, cree_par)
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id)
        .bind(etab)
        .bind(&titre)
        .bind(n.classe_id)
        .bind(acteur)
        .execute(self.pool())
        .await
        .map_err(db)?;
        self.rencontre(etab, id).await
    }

    async fn rencontre(&self, etab: Uuid, id: Uuid) -> Result<Rencontre, EntDbError> {
        sqlx::query_as(&format!("{SELECT_RENCONTRE} WHERE r.etablissement_id = $1 AND r.id = $2"))
            .bind(etab)
            .bind(id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?
            .ok_or_else(introuvable)
    }

    /// Rencontres de l'établissement (organisateur).
    pub async fn rencontres_gestion(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<Rencontre>, EntDbError> {
        self.exiger_rencontres(etab, acteur, role).await?;
        sqlx::query_as(&format!(
            "{SELECT_RENCONTRE} WHERE r.etablissement_id = $1 ORDER BY r.created_at DESC LIMIT 200"
        ))
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Enseignants ayant la classe en service (classe entière ou demi-groupe).
    pub async fn enseignants_de_classe(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        classe_id: Uuid,
    ) -> Result<Vec<(Uuid, String, String)>, EntDbError> {
        self.exiger_rencontres(etab, acteur, role).await?;
        enseignants_de(self.pool(), etab, classe_id).await
    }

    /// Ajoute des créneaux consécutifs à un enseignant de la classe.
    pub async fn ajouter_creneaux(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        rencontre_id: Uuid,
        n: NouveauxCreneaux,
    ) -> Result<usize, EntDbError> {
        self.exiger_rencontres(etab, acteur, role).await?;
        if !(5..=120).contains(&n.duree_minutes) || !(1..=40).contains(&n.nombre) {
            return Err(EntDbError::SaisieInvalide("durée ou nombre de créneaux invalide"));
        }
        let r = self.rencontre(etab, rencontre_id).await?;
        if r.statut == "close" {
            return Err(EntDbError::ConflitDossier("la rencontre est close"));
        }
        let profs = enseignants_de(self.pool(), etab, r.classe_id).await?;
        if !profs.iter().any(|(id, _, _)| *id == n.enseignant_user_id) {
            return Err(EntDbError::SaisieInvalide("cet enseignant n'a pas la classe en service"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        // Sérialise les ajouts concurrents pour un même enseignant (chevauchements).
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 9))")
            .bind(n.enseignant_user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let pas = Duration::minutes(i64::from(n.duree_minutes));
        let mut crees = 0usize;
        for i in 0..n.nombre {
            let debut = n.debut + pas * i;
            let fin = debut + pas;
            let (chevauche,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_creneau_rdv
                                 WHERE etablissement_id = $1 AND enseignant_user_id = $2
                                   AND debut < $4 AND fin > $3)",
            )
            .bind(etab)
            .bind(n.enseignant_user_id)
            .bind(debut)
            .bind(fin)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if chevauche {
                return Err(EntDbError::ConflitDossier(
                    "un créneau de cet enseignant en chevauche un autre",
                ));
            }
            sqlx::query(
                "INSERT INTO ent_creneau_rdv
                    (id, etablissement_id, rencontre_id, enseignant_user_id, debut, fin)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(rencontre_id)
            .bind(n.enseignant_user_id)
            .bind(debut)
            .bind(fin)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            crees += 1;
        }
        tx.commit().await.map_err(db)?;
        Ok(crees)
    }

    /// Ouvre la prise de rendez-vous et prévient les familles de la classe.
    pub async fn ouvrir_rencontre(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        rencontre_id: Uuid,
    ) -> Result<Rencontre, EntDbError> {
        self.exiger_rencontres(etab, acteur, role).await?;
        let r = self.rencontre(etab, rencontre_id).await?;
        if r.nb_creneaux == 0 {
            return Err(EntDbError::SaisieInvalide("aucun créneau proposé"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let ouverte = sqlx::query(
            "UPDATE ent_rencontre SET statut = 'ouverte', ouverte_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'brouillon'",
        )
        .bind(etab)
        .bind(rencontre_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if ouverte.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier("la rencontre est déjà ouverte ou close"));
        }
        let evt = tx
            .publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_com::RENCONTRE_OUVERTE.to_string(),
                aggregate: AggregateKind::Annonce,
                aggregate_id: rencontre_id,
                eleve_id: None,
                payload: json!({ "rencontre_id": rencontre_id, "classe_id": r.classe_id }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        let eleves: Vec<Uuid> = eleves_de_audience(
            &mut tx,
            etab,
            "classe",
            &[r.classe_id],
            Utc::now().date_naive(),
        )
        .await?
        .into_iter()
        .collect();
        let responsables = crate::campagnes::responsables_de(&mut tx, etab, &eleves).await?;
        let titre = format!("Rencontres parents-professeurs : {}", r.titre);
        let corps = format!(
            "La prise de rendez-vous est ouverte pour la classe {}. \
             Choisissez vos créneaux depuis l'ENT.",
            r.classe_libelle
        );
        let maintenant = Utc::now();
        for resp in responsables {
            notifier_personne(
                &mut tx,
                EnvoiAPlanifier {
                    etablissement_id: etab,
                    user_id: resp.user_id,
                    kind: "rencontre_ouverte",
                    titre: &titre,
                    corps: &corps,
                    eleve_id: Some(resp.eleve_id),
                    notification_id: None,
                    source_type: "rencontre",
                    source_id: rencontre_id,
                    cle_dedup: format!("renc:{rencontre_id}:ouverte"),
                    droit_requis: Some("lien"),
                    regroupable: true,
                    canaux_voulus: &["email", "push"],
                },
                evt.id,
                maintenant,
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        self.rencontre(etab, rencontre_id).await
    }

    pub async fn cloturer_rencontre(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        rencontre_id: Uuid,
    ) -> Result<Rencontre, EntDbError> {
        self.exiger_rencontres(etab, acteur, role).await?;
        let r = sqlx::query(
            "UPDATE ent_rencontre SET statut = 'close', close_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut <> 'close'",
        )
        .bind(etab)
        .bind(rencontre_id)
        .execute(self.pool())
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            // Inexistante ou déjà close : distingue les deux.
            self.rencontre(etab, rencontre_id).await?;
            return Err(EntDbError::ConflitDossier("la rencontre est déjà close"));
        }
        self.rencontre(etab, rencontre_id).await
    }

    /// Planning complet (organisateur).
    pub async fn planning_rencontre(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        rencontre_id: Uuid,
    ) -> Result<Vec<LignePlanning>, EntDbError> {
        self.exiger_rencontres(etab, acteur, role).await?;
        self.rencontre(etab, rencontre_id).await?;
        sqlx::query_as(&format!(
            "{SELECT_PLANNING} WHERE k.etablissement_id = $1 AND k.rencontre_id = $2
              ORDER BY u.display_name NULLS LAST, k.debut"
        ))
        .bind(etab)
        .bind(rencontre_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Mes créneaux d'enseignant (toutes rencontres confondues).
    pub async fn mes_creneaux_enseignant(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<LignePlanning>, EntDbError> {
        sqlx::query_as(&format!(
            "{SELECT_PLANNING} WHERE k.etablissement_id = $1 AND k.enseignant_user_id = $2
               AND k.fin >= now() - interval '1 day'
              ORDER BY k.debut"
        ))
        .bind(etab)
        .bind(user_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    // ----- Famille ---------------------------------------------------------

    /// Rencontres ouvertes concernant les enfants du responsable connecté.
    pub async fn rencontres_famille(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<RencontreFamille>, EntDbError> {
        let rows: Vec<(Uuid, String, String, Uuid, String)> = sqlx::query_as(
            "SELECT DISTINCT r.id, r.titre, c.libelle, e.id, e.prenom
               FROM ent_eleve_lien_actif l
               JOIN ent_eleve e ON e.id = l.eleve_id AND e.etablissement_id = l.etablissement_id
               JOIN ent_affectation a
                 ON a.eleve_id = l.eleve_id AND a.etablissement_id = l.etablissement_id
                AND a.date_debut <= current_date AND (a.date_fin IS NULL OR a.date_fin >= current_date)
               JOIN ent_rencontre r
                 ON r.etablissement_id = a.etablissement_id AND r.classe_id = a.classe_id
                AND r.statut = 'ouverte'
               JOIN ent_classe c ON c.id = r.classe_id AND c.etablissement_id = r.etablissement_id
              WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.autorite_parentale
              ORDER BY r.titre, e.prenom",
        )
        .bind(etab)
        .bind(user_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(rencontre_id, titre, classe_libelle, eleve_id, eleve_prenom)| RencontreFamille {
                rencontre_id,
                titre,
                classe_libelle,
                eleve_id,
                eleve_prenom,
            })
            .collect())
    }

    /// Le responsable réserve-t-il légitimement pour cet enfant dans cette rencontre ?
    async fn verifier_famille(
        &self,
        etab: Uuid,
        user_id: Uuid,
        rencontre_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Rencontre, EntDbError> {
        let r = self.rencontre(etab, rencontre_id).await?;
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM ent_eleve_lien_actif l
                  JOIN ent_affectation a
                    ON a.eleve_id = l.eleve_id AND a.etablissement_id = l.etablissement_id
                   AND a.classe_id = $4
                   AND a.date_debut <= current_date AND (a.date_fin IS NULL OR a.date_fin >= current_date)
                 WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.eleve_id = $3
                   AND l.autorite_parentale)",
        )
        .bind(etab)
        .bind(user_id)
        .bind(eleve_id)
        .bind(r.classe_id)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        if !ok {
            // Pas de lien actif avec cet enfant dans cette classe : indiscernable d'un inconnu.
            return Err(introuvable());
        }
        Ok(r)
    }

    /// Créneaux proposés pour un enfant, par enseignant.
    pub async fn creneaux_famille(
        &self,
        etab: Uuid,
        user_id: Uuid,
        rencontre_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<EnseignantCreneaux>, EntDbError> {
        let r = self.verifier_famille(etab, user_id, rencontre_id, eleve_id).await?;
        if r.statut != "ouverte" {
            return Err(EntDbError::ConflitDossier("la prise de rendez-vous n'est pas ouverte"));
        }
        // RP1-27 : seulement les enseignants qui ont CET enfant (classe entière ou son groupe).
        let profs = enseignants_de_eleve(self.pool(), etab, r.classe_id, eleve_id).await?;
        let rows: Vec<(Uuid, Uuid, DateTime<Utc>, DateTime<Utc>, String, Option<Uuid>, Option<Uuid>)> =
            sqlx::query_as(
                "SELECT id, enseignant_user_id, debut, fin, statut, reserve_par_user_id, eleve_id
                   FROM ent_creneau_rdv
                  WHERE etablissement_id = $1 AND rencontre_id = $2
                  ORDER BY debut",
            )
            .bind(etab)
            .bind(rencontre_id)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        Ok(profs
            .into_iter()
            .map(|(id, nom, matieres)| EnseignantCreneaux {
                enseignant_user_id: id,
                nom,
                matieres,
                creneaux: rows
                    .iter()
                    .filter(|k| k.1 == id)
                    .map(|k| CreneauVu {
                        id: k.0,
                        debut: k.2,
                        fin: k.3,
                        etat: if k.4 == "libre" {
                            "libre"
                        } else if k.5 == Some(user_id) && k.6 == Some(eleve_id) {
                            "reserve_par_moi"
                        } else {
                            "indisponible"
                        }
                        .to_string(),
                    })
                    .collect(),
            })
            .collect())
    }

    /// Réserve un créneau pour un enfant (AGD-03).
    pub async fn reserver_creneau(
        &self,
        etab: Uuid,
        user_id: Uuid,
        creneau_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<LignePlanning, EntDbError> {
        let k: Option<(Uuid, Uuid, DateTime<Utc>, DateTime<Utc>, String)> = sqlx::query_as(
            "SELECT rencontre_id, enseignant_user_id, debut, fin, statut
               FROM ent_creneau_rdv WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(creneau_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let (rencontre_id, enseignant, debut, fin, _) = k.ok_or_else(introuvable)?;
        let r = self.verifier_famille(etab, user_id, rencontre_id, eleve_id).await?;
        if r.statut != "ouverte" {
            return Err(EntDbError::ConflitDossier("la prise de rendez-vous n'est pas ouverte"));
        }
        let profs = enseignants_de_eleve(self.pool(), etab, r.classe_id, eleve_id).await?;
        if !profs.iter().any(|(id, _, _)| *id == enseignant) {
            return Err(EntDbError::SaisieInvalide("cet enseignant n'a pas votre enfant en classe"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        // Un responsable ne peut être à deux endroits à la fois.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 10))")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let (chevauche,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_creneau_rdv
                             WHERE etablissement_id = $1 AND reserve_par_user_id = $2
                               AND statut = 'reserve' AND debut < $4 AND fin > $3)",
        )
        .bind(etab)
        .bind(user_id)
        .bind(debut)
        .bind(fin)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if chevauche {
            return Err(EntDbError::ConflitDossier("vous avez déjà un rendez-vous à cette heure"));
        }
        let (deja,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_creneau_rdv
                             WHERE etablissement_id = $1 AND rencontre_id = $2
                               AND enseignant_user_id = $3 AND eleve_id = $4 AND statut = 'reserve')",
        )
        .bind(etab)
        .bind(rencontre_id)
        .bind(enseignant)
        .bind(eleve_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja {
            return Err(EntDbError::ConflitDossier(
                "un rendez-vous est déjà pris avec cet enseignant pour cet enfant",
            ));
        }
        // Mise à jour conditionnelle : une seule famille l'emporte.
        let pris = sqlx::query(
            "UPDATE ent_creneau_rdv
                SET statut = 'reserve', reserve_par_user_id = $3, eleve_id = $4, reserve_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'libre'",
        )
        .bind(etab)
        .bind(creneau_id)
        .bind(user_id)
        .bind(eleve_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if pris.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier("ce créneau vient d'être pris"));
        }
        let evt = tx
            .publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_com::RDV_PRIS.to_string(),
                aggregate: AggregateKind::Annonce,
                aggregate_id: creneau_id,
                eleve_id: Some(eleve_id),
                payload: json!({ "creneau_id": creneau_id, "rencontre_id": rencontre_id }),
                actor_user_id: Some(user_id.to_string()),
            })
            .await?;
        notifier_rdv(&mut tx, etab, creneau_id, evt.id, "rdv_pris", "Rendez-vous confirmé").await?;
        tx.commit().await.map_err(db)?;
        ligne_planning(self, etab, creneau_id).await
    }

    /// Annule un rendez-vous : le responsable qui l'a pris, ou l'organisateur.
    pub async fn annuler_reservation(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        creneau_id: Uuid,
    ) -> Result<LignePlanning, EntDbError> {
        let organisateur = self
            .a_action_etablissement(etab, user_id, role, Action::RencontresGerer)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let k: Option<(Option<Uuid>, Option<Uuid>, String)> = sqlx::query_as(
            "SELECT reserve_par_user_id, eleve_id, statut FROM ent_creneau_rdv
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(creneau_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (par, eleve, statut) = k.ok_or_else(introuvable)?;
        if statut != "reserve" {
            return Err(EntDbError::ConflitDossier("ce créneau n'est pas réservé"));
        }
        if par != Some(user_id) && !organisateur {
            // Pas le titulaire : indiscernable d'un créneau inconnu.
            return Err(introuvable());
        }
        let evt = tx
            .publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_com::RDV_ANNULE.to_string(),
                aggregate: AggregateKind::Annonce,
                aggregate_id: creneau_id,
                eleve_id: eleve,
                payload: json!({ "creneau_id": creneau_id }),
                actor_user_id: Some(user_id.to_string()),
            })
            .await?;
        // Les notifications relisent le créneau AVANT sa libération.
        notifier_rdv(&mut tx, etab, creneau_id, evt.id, "rdv_annule", "Rendez-vous annulé").await?;
        sqlx::query(
            "UPDATE ent_creneau_rdv
                SET statut = 'libre', reserve_par_user_id = NULL, eleve_id = NULL,
                    reserve_le = NULL, rappel_envoye_le = NULL
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(creneau_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        ligne_planning(self, etab, creneau_id).await
    }

    /// Rappels des rendez-vous dans les prochaines `horizon` (24 h par défaut).
    /// Idempotent : un rendez-vous n'est rappelé qu'une fois. Retourne le nombre
    /// de rendez-vous rappelés.
    pub async fn emettre_rappels_rdv(
        &self,
        maintenant: DateTime<Utc>,
        horizon: Duration,
        etablissement: Option<Uuid>,
    ) -> Result<usize, EntDbError> {
        let mut tx = self.pool().begin().await.map_err(db)?;
        let dus: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, etablissement_id FROM ent_creneau_rdv
              WHERE statut = 'reserve' AND rappel_envoye_le IS NULL
                AND debut > $1 AND debut <= $2
                AND ($3::uuid IS NULL OR etablissement_id = $3)
              ORDER BY debut LIMIT 500
              FOR UPDATE SKIP LOCKED",
        )
        .bind(maintenant)
        .bind(maintenant + horizon)
        .bind(etablissement)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (id, etab) in &dus {
            // Un rappel par réservation : l'identifiant d'événement est propre à ce rappel.
            notifier_rdv(&mut tx, *etab, *id, Uuid::new_v4(), "rdv_rappel", "Rappel de rendez-vous")
                .await?;
            sqlx::query(
                "UPDATE ent_creneau_rdv SET rappel_envoye_le = $3
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(dus.len())
    }
}

async fn ligne_planning(ent: &PgEnt, etab: Uuid, creneau_id: Uuid) -> Result<LignePlanning, EntDbError> {
    sqlx::query_as(&format!("{SELECT_PLANNING} WHERE k.etablissement_id = $1 AND k.id = $2"))
        .bind(etab)
        .bind(creneau_id)
        .fetch_optional(ent.pool())
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

/// Enseignants (id, nom, matières) ayant la classe en service.
async fn enseignants_de(
    pool: &sqlx::PgPool,
    etab: Uuid,
    classe_id: Uuid,
) -> Result<Vec<(Uuid, String, String)>, EntDbError> {
    let rows: Vec<(Uuid, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT s.prof_user_id, max(u.display_name),
                string_agg(DISTINCT NULLIF(s.matiere_libelle, ''), ', ')
           FROM ent_service_enseignement s
           JOIN ent_user u ON u.id = s.prof_user_id
           LEFT JOIN ent_groupe g ON g.id = s.groupe_id AND g.etablissement_id = s.etablissement_id
          WHERE s.etablissement_id = $1 AND (s.classe_id = $2 OR g.classe_id = $2)
          GROUP BY s.prof_user_id
          ORDER BY max(u.display_name) NULLS LAST, s.prof_user_id",
    )
    .bind(etab)
    .bind(classe_id)
    .fetch_all(pool)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|(id, nom, m)| (id, nom.unwrap_or_else(|| "(enseignant)".into()), m.unwrap_or_default()))
        .collect())
}

/// Enseignants (id, nom, matières) qui ont **cet élève** : service de la classe entière, ou
/// service d'un groupe de la classe auquel l'élève est affecté aujourd'hui (D-022, RP1-27).
async fn enseignants_de_eleve(
    pool: &sqlx::PgPool,
    etab: Uuid,
    classe_id: Uuid,
    eleve_id: Uuid,
) -> Result<Vec<(Uuid, String, String)>, EntDbError> {
    let rows: Vec<(Uuid, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT s.prof_user_id, max(u.display_name),
                string_agg(DISTINCT NULLIF(s.matiere_libelle, ''), ', ')
           FROM ent_service_enseignement s
           JOIN ent_user u ON u.id = s.prof_user_id
           LEFT JOIN ent_groupe g ON g.id = s.groupe_id AND g.etablissement_id = s.etablissement_id
          WHERE s.etablissement_id = $1
            AND (s.classe_id = $2
                 OR (g.classe_id = $2
                     AND EXISTS (SELECT 1 FROM ent_affectation a
                                  WHERE a.etablissement_id = s.etablissement_id
                                    AND a.eleve_id = $3 AND a.groupe_id = s.groupe_id
                                    AND a.date_debut <= CURRENT_DATE
                                    AND (a.date_fin IS NULL OR a.date_fin >= CURRENT_DATE))))
          GROUP BY s.prof_user_id
          ORDER BY max(u.display_name) NULLS LAST, s.prof_user_id",
    )
    .bind(etab)
    .bind(classe_id)
    .bind(eleve_id)
    .fetch_all(pool)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|(id, nom, m)| (id, nom.unwrap_or_else(|| "(enseignant)".into()), m.unwrap_or_default()))
        .collect())
}

/// Prévient le responsable et l'enseignant d'un créneau (dans l'ENT + canaux).
async fn notifier_rdv(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    creneau_id: Uuid,
    source_event_id: Uuid,
    kind: &'static str,
    titre: &str,
) -> Result<(), EntDbError> {
    let k: Option<(Uuid, DateTime<Utc>, Uuid, Option<Uuid>, Option<Uuid>, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT k.enseignant_user_id, k.debut, k.rencontre_id, k.reserve_par_user_id, k.eleve_id,
                    u.display_name, e.prenom
               FROM ent_creneau_rdv k
               JOIN ent_user u ON u.id = k.enseignant_user_id
               LEFT JOIN ent_eleve e ON e.id = k.eleve_id AND e.etablissement_id = k.etablissement_id
              WHERE k.etablissement_id = $1 AND k.id = $2",
        )
        .bind(etab)
        .bind(creneau_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
    let Some((prof, debut, _renc, parent, eleve, prof_nom, prenom)) = k else {
        return Ok(());
    };
    let quand = crate::assiduite::heure_paris(debut).format("%d/%m/%Y à %H:%M").to_string();
    let prof_nom = prof_nom.unwrap_or_else(|| "l'enseignant".into());
    let prenom = prenom.unwrap_or_else(|| "votre enfant".into());
    let cle = format!("rdv:{creneau_id}:{kind}:{source_event_id}");
    let maintenant = Utc::now();
    if let Some(parent) = parent {
        let corps = format!("Rendez-vous avec {prof_nom} pour {prenom}, le {quand}.");
        notifier_personne(
            conn,
            EnvoiAPlanifier {
                etablissement_id: etab,
                user_id: parent,
                kind,
                titre,
                corps: &corps,
                eleve_id: eleve,
                notification_id: None,
                source_type: "rencontre",
                source_id: creneau_id,
                cle_dedup: cle.clone(),
                droit_requis: Some("lien"),
                regroupable: kind == "rdv_pris",
                canaux_voulus: &["email", "sms", "push"],
            },
            source_event_id,
            maintenant,
        )
        .await?;
    }
    let corps = format!("Rendez-vous parent pour {prenom}, le {quand}.");
    notifier_personne(
        conn,
        EnvoiAPlanifier {
            etablissement_id: etab,
            user_id: prof,
            kind,
            titre,
            corps: &corps,
            eleve_id: eleve,
            notification_id: None,
            source_type: "rencontre",
            source_id: creneau_id,
            cle_dedup: cle,
            droit_requis: None,
            regroupable: kind == "rdv_pris",
            canaux_voulus: &["email", "push"],
        },
        source_event_id,
        maintenant,
    )
    .await?;
    Ok(())
}
