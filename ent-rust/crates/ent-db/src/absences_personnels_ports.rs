//! Socle 15 — ports exposés aux autres socles (`15_ABSENCES_PERSONNELS.md` §6.2, D-058).
//!
//! **Port « absences validées non remplacées »** (socles 10 et 19) : les séances dont l'enseignant est
//! absent — absence **décidée** (validée, prise en compte, décision de l'employeur, traitée, close) —
//! et qui n'ont **pas de remplaçant**. C'est ce qui permet au socle 19 de libérer une salle de classe
//! quand un cours est supprimé (D-058) et au socle 10 de savoir quelles séances « n'ont plus
//! d'enseignant ».
//!
//! * **Jamais de motif, de nature ni de nom d'agent** : seulement la séance (classe, matière, heures,
//!   salle) et l'identifiant de l'absence (pour retrouver la cause côté RH, avec les droits qui vont avec).
//!   L'affichage dit « cours de 4ᵉ B supprimé », jamais pourquoi (D-058, R-AP2).
//! * **Jamais de copie** : le port se calcule à chaque appel sur les séances du socle 10 et les absences
//!   du 15 ; un remplaçant nommé (`edt_organiser_remplacement`) ou une absence annulée la fait disparaître
//!   sans qu'aucun autre socle ait quelque chose à défaire.
//! * **Une absence seulement signalée, refusée ou annulée n'y figure pas** : « une salle de classe libérée
//!   n'est proposée qu'après la décision d'absence » (D-058), jamais sur simple soupçon (R-AP46).
//! * Une absence partielle (de 10 h à 12 h) ne touche que les séances qui la recoupent.

use chrono::{Duration, NaiveDate, NaiveDateTime};
use ent_auth::Action;
use ent_types::absences_personnels::SeanceSansEnseignantDto;
use uuid::Uuid;

use crate::absences_personnels::ETATS_DECIDES;
use crate::personnels::{db, invalide};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

impl PgEnt {
    /// Port interne : séances sans enseignant du fait d'absences décidées, entre deux dates (inclusives,
    /// 92 jours au plus). Aucun contrôle de droit (l'appelant est un autre socle, dans sa propre
    /// autorisation) : la couche HTTP passe par [`Self::absences_validees_non_remplacees_for_actor`].
    pub async fn absences_validees_non_remplacees(
        &self,
        etab: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceSansEnseignantDto>, EntDbError> {
        if au < du || (au - du).num_days() > 92 {
            return Err(invalide("Période invalide (92 jours au plus)."));
        }
        // Plages décidées, en heure locale de l'établissement (comparables aux heures de séance).
        let plages: Vec<(Uuid, Uuid, NaiveDateTime, NaiveDateTime)> = sqlx::query_as(
            "WITH f AS (SELECT COALESCE((SELECT fuseau FROM ent_etablissement_parametre
                                          WHERE etablissement_id = $1), 'Europe/Paris') AS fus)
             SELECT a.id, p.user_id, (pl.debut AT TIME ZONE f.fus), (pl.fin AT TIME ZONE f.fus)
               FROM f, ent_absp_absence a
               JOIN ent_absp_plage pl ON pl.etablissement_id = a.etablissement_id AND pl.absence_id = a.id
                                      AND pl.retiree_le IS NULL
               JOIN ent_agent ag ON ag.etablissement_id = a.etablissement_id AND ag.id = a.agent_id
               JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
              WHERE a.etablissement_id = $1 AND a.cause = 'rh' AND a.etat = ANY($4) AND p.user_id IS NOT NULL
                AND (pl.debut AT TIME ZONE f.fus)::date <= $3
                AND ((pl.fin - interval '1 microsecond') AT TIME ZONE f.fus)::date >= $2
              ORDER BY pl.debut, a.id",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .bind(ETATS_DECIDES.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?;
        if plages.is_empty() {
            return Ok(Vec::new());
        }
        let mut agents: Vec<Uuid> = plages.iter().map(|p| p.1).collect();
        agents.sort();
        agents.dedup();
        let mut out: Vec<SeanceSansEnseignantDto> = Vec::new();
        for user in agents {
            let mine: Vec<&(Uuid, Uuid, NaiveDateTime, NaiveDateTime)> =
                plages.iter().filter(|p| p.1 == user).collect();
            let d = mine.iter().map(|p| p.2.date()).min().unwrap_or(du).max(du);
            let f = mine
                .iter()
                .map(|p| (p.3 - Duration::microseconds(1)).date())
                .max()
                .unwrap_or(au)
                .min(au);
            if f < d {
                continue;
            }
            let seances = self.seances_du_personnel(etab, user, d, f).await?;
            for s in seances {
                // Seul l'enseignant en titre compte ici : un remplaçant absent à son tour est un autre cas.
                if s.role_absent != "titulaire" {
                    continue;
                }
                let statut = match s.statut.as_str() {
                    "prevue" => "non_remplacee",
                    "annulee" => "annulee",
                    _ => continue, // remplacée ou déplacée : la salle et le cours sont tenus
                };
                let debut = s.jour.and_time(s.heure_debut);
                let fin = s.jour.and_time(s.heure_fin);
                // La séance doit recouper une plage de l'absence (absence partielle).
                let Some(abs) = mine.iter().find(|p| p.2 < fin && p.3 > debut) else {
                    continue;
                };
                if out
                    .iter()
                    .any(|x| x.creneau_id == s.creneau_id && x.jour == s.jour)
                {
                    continue;
                }
                out.push(SeanceSansEnseignantDto {
                    absence_id: abs.0,
                    creneau_id: s.creneau_id,
                    jour: s.jour,
                    classe: s.classe,
                    matiere_libelle: s.matiere_libelle,
                    heure_debut: s.heure_debut,
                    heure_fin: s.heure_fin,
                    salle: s.salle,
                    statut: statut.to_string(),
                });
            }
        }
        out.sort_by(|a, b| {
            (a.jour, a.heure_debut, &a.classe).cmp(&(b.jour, b.heure_debut, &b.classe))
        });
        Ok(out)
    }

    /// Lecture HTTP du port : `edt.gerer`, `personnels.absences.lire_organisation` ou `lire_nature`.
    pub async fn absences_validees_non_remplacees_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceSansEnseignantDto>, EntDbError> {
        let ok = self
            .a_action_etablissement(etab, actor, role, Action::EdtGerer)
            .await?
            || self
                .a_action_etablissement(
                    etab,
                    actor,
                    role,
                    Action::PersonnelsAbsencesLireOrganisation,
                )
                .await?
            || self
                .a_action_etablissement(etab, actor, role, Action::PersonnelsAbsencesLireNature)
                .await?;
        if !ok {
            return Err(EntDbError::Forbidden);
        }
        self.absences_validees_non_remplacees(etab, du, au).await
    }
}
