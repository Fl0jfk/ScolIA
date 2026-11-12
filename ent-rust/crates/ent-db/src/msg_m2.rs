//! Messagerie, lot M2 (`docs/socles/16_MESSAGERIE.md` §8, §9, R-M3, R-M5 à R-M8, R-M12) : matrice
//! « qui peut écrire à qui » branchée sur les faits de la base, aperçu des destinataires, sept
//! réglages bornés, dédoublement d'un fil commun (séparation, MSG-118) et majorité de l'élève.
//!
//! La logique de décision est **pure** et vit dans [`crate::msg_matrice`] ; ce module lit les
//! faits (lien d'enseignement D-022, lien familial, interdiction, « ne pas solliciter », majorité)
//! et applique la décision. Le serveur recalcule la matrice à chaque envoi : l'aperçu n'est qu'une
//! aide, jamais une autorisation.
//!
//! Port du socle 4 « destinataires » : **adaptateur provisoire** sur les liens actuels
//! (`ent_eleve_lien_sollicitable`, `ent_eleve_lien_communicant`), comme le prévoit le §27.1.
//! Les ports des socles 5 (cours, fin de cours pour CM-35) et 6 (missions, équipes) sont à brancher
//! quand ils publieront leurs services ; l'adaptateur lit aujourd'hui `autoriser_eleve` (D-022).

use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::msg_matrice::{
    decider, Acteur, AeshVersFamille, Cible, Decision, EleveVersEleve, Faits, ReglagesMatrice,
};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Une personne destinataire d'un aperçu : un nom d'affichage, le lien, la raison. **Jamais** de
/// coordonnée (adresse, téléphone, e-mail : R-M4).
#[derive(Debug, Clone, Serialize)]
pub struct DestinataireApercu {
    pub nom_affiche: String,
    pub lien: String,
    pub raison: &'static str,
}

/// Une conversation qui sera créée : un fil par foyer (R-M3), foyers numérotés (« foyer 1 »).
#[derive(Debug, Clone, Serialize)]
pub struct ConversationApercu {
    pub numero: usize,
    pub destinataires: Vec<DestinataireApercu>,
}

/// Aperçu des destinataires d'un message du personnel « aux responsables » d'un élève (R-M5 : le
/// personnel choisit des élèves, le logiciel calcule les destinataires).
#[derive(Debug, Clone, Serialize)]
pub struct ApercuDestinataires {
    pub eleve_id: Uuid,
    pub decision: Decision,
    /// Élève majeur : le message s'adresse à l'élève, pas à ses parents (D23, R-M12).
    pub destinataire_est_l_eleve: bool,
    pub conversations: Vec<ConversationApercu>,
    /// Nombre de responsables **exclus** (décision, « ne pas solliciter »), sans nom ni cause :
    /// l'exclusion est agrégée pour le personnel non habilité (R-M6, socle 8 R-N51).
    pub exclus: usize,
}

impl PgEnt {
    // ───────────────────────── Les sept réglages bornés (§8.3) ─────────────────────────

    /// Réglages en vigueur (défaut §8.2 si l'établissement n'en a jamais enregistré).
    pub async fn reglages_matrice(
        &self,
        etablissement_id: Uuid,
    ) -> Result<ReglagesMatrice, EntDbError> {
        #[allow(clippy::type_complexity)]
        let l: Option<(bool, bool, bool, String, String, bool, bool)> = sqlx::query_as(
            "SELECT familles_vers_professeurs, familles_vers_direction, eleves_vers_professeurs,
                    eleves_vers_eleves, aesh_vers_familles, secretariat_vers_majeurs, pieces_familles
               FROM ent_msg_reglage WHERE etablissement_id = $1",
        )
        .bind(etablissement_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await?;
        let Some((fp, fd, ep, ee, ak, sm, pf)) = l else {
            return Ok(ReglagesMatrice::default());
        };
        Ok(ReglagesMatrice {
            familles_vers_professeurs: fp,
            familles_vers_direction: fd,
            eleves_vers_professeurs: ep,
            // La contrainte de la base interdit toute autre valeur ; en cas de doute, le plus fermé.
            eleves_vers_eleves: EleveVersEleve::parse(&ee).unwrap_or(EleveVersEleve::Non),
            aesh_vers_familles: AeshVersFamille::parse(&ak).unwrap_or(AeshVersFamille::Non),
            secretariat_vers_majeurs: sm,
            pieces_familles: pf,
        })
    }

    /// Règle les sept réglages (MSG-262) : `notifications.gerer` à l'échelle de l'établissement,
    /// comme la plage calme. Les valeurs sont bornées par le type (`EleveVersEleve`,
    /// `AeshVersFamille`) : aucune liste de rôles libre. L'ancienne valeur est conservée par la base
    /// (`ent_msg_reglage_version`, ajout seul).
    pub async fn definir_reglages_matrice_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        saisie: ReglagesMatrice,
    ) -> Result<ReglagesMatrice, EntDbError> {
        if !self
            .a_action_etablissement(
                etablissement_id,
                actor_user_id,
                role,
                Action::NotificationsGerer,
            )
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        sqlx::query(
            "INSERT INTO ent_msg_reglage
                (etablissement_id, familles_vers_professeurs, familles_vers_direction,
                 eleves_vers_professeurs, eleves_vers_eleves, aesh_vers_familles,
                 secretariat_vers_majeurs, pieces_familles, modifie_par, modifie_le)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,now())
             ON CONFLICT (etablissement_id) DO UPDATE
                SET familles_vers_professeurs = EXCLUDED.familles_vers_professeurs,
                    familles_vers_direction = EXCLUDED.familles_vers_direction,
                    eleves_vers_professeurs = EXCLUDED.eleves_vers_professeurs,
                    eleves_vers_eleves = EXCLUDED.eleves_vers_eleves,
                    aesh_vers_familles = EXCLUDED.aesh_vers_familles,
                    secretariat_vers_majeurs = EXCLUDED.secretariat_vers_majeurs,
                    pieces_familles = EXCLUDED.pieces_familles,
                    modifie_par = EXCLUDED.modifie_par, modifie_le = now()",
        )
        .bind(etablissement_id)
        .bind(saisie.familles_vers_professeurs)
        .bind(saisie.familles_vers_direction)
        .bind(saisie.eleves_vers_professeurs)
        .bind(saisie.eleves_vers_eleves.as_str())
        .bind(saisie.aesh_vers_familles.as_str())
        .bind(saisie.secretariat_vers_majeurs)
        .bind(saisie.pieces_familles)
        .bind(actor_user_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::MSG_REGLAGE.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: etablissement_id,
                eleve_id: None,
                // Valeurs de réglage : aucune donnée personnelle.
                payload: json!({
                    "familles_vers_professeurs": saisie.familles_vers_professeurs,
                    "familles_vers_direction": saisie.familles_vers_direction,
                    "eleves_vers_professeurs": saisie.eleves_vers_professeurs,
                    "eleves_vers_eleves": saisie.eleves_vers_eleves.as_str(),
                    "aesh_vers_familles": saisie.aesh_vers_familles.as_str(),
                    "secretariat_vers_majeurs": saisie.secretariat_vers_majeurs,
                    "pieces_familles": saisie.pieces_familles,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        self.reglages_matrice(etablissement_id).await
    }

    // ───────────────────────── Matrice (§8) ─────────────────────────

    /// Ligne du §8.2 qui correspond à un rôle de base (D-017 : le rôle de base ne donne pas de
    /// droit, il classe seulement l'acteur dans la matrice).
    pub(crate) fn acteur_matrice(role: &str, eleve_majeur: bool) -> Option<Acteur> {
        Some(match Role::parse(role)? {
            Role::Parent => Acteur::Famille,
            Role::Eleve if eleve_majeur => Acteur::EleveMajeur,
            Role::Eleve => Acteur::EleveMineur,
            Role::Prof => Acteur::Professeur,
            Role::Cpe => Acteur::CpeVieScolaire,
            Role::Admin | Role::Direction => Acteur::Direction,
            Role::Administratif | Role::Compta | Role::Personnel => Acteur::Secretariat,
            Role::Infirmerie | Role::Psy => Acteur::Sante,
            Role::Accueil => Acteur::Accueil,
        })
    }

    /// `peut_ecrire` (R-M2, §8.1) : l'acteur peut-il ouvrir (ou répondre à) une conversation vers
    /// `cible`, à propos de l'élève `eleve_id` ? Recalculé à chaque appel, à la date du jour de
    /// l'établissement. `lien_user` désigne un responsable précis (sinon : les responsables de
    /// l'élève pris ensemble).
    pub async fn peut_ecrire_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        cible: Cible,
        eleve_id: Uuid,
        lien_user: Option<Uuid>,
        ouverture: bool,
    ) -> Result<Decision, EntDbError> {
        let jour = self.aujourdhui_etablissement(etablissement_id).await?;
        let majeur = self
            .eleve_est_majeur(etablissement_id, eleve_id, jour)
            .await?;
        let Some(acteur) = Self::acteur_matrice(role, majeur) else {
            return Ok(Decision::Refuse(crate::msg_matrice::Refus::Matrice));
        };
        let reglages = self.reglages_matrice(etablissement_id).await?;
        let faits = self
            .faits_ecriture(
                etablissement_id,
                actor_user_id,
                role,
                acteur,
                cible,
                eleve_id,
                lien_user,
                ouverture,
                majeur,
            )
            .await?;
        Ok(decider(acteur, cible, &reglages, &faits))
    }

    #[allow(clippy::too_many_arguments)]
    async fn faits_ecriture(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        acteur: Acteur,
        cible: Cible,
        eleve_id: Uuid,
        lien_user: Option<Uuid>,
        ouverture: bool,
        eleve_majeur: bool,
    ) -> Result<Faits, EntDbError> {
        let mut faits = Faits {
            ouverture,
            eleve_majeur,
            ..Faits::default()
        };

        // La raison propre à l'acteur, à la date.
        faits.raison = match acteur {
            Acteur::Professeur | Acteur::ProfPrincipal | Acteur::Aesh | Acteur::Sante => {
                self.autoriser_eleve(
                    etablissement_id,
                    actor_user_id,
                    role,
                    Action::MessagerieEleve,
                    eleve_id,
                )
                .await?
            }
            Acteur::Famille => self
                .foyers
                .lien_communicant(etablissement_id, actor_user_id, eleve_id)
                .await?
                .is_some(),
            Acteur::EleveMineur | Acteur::EleveMajeur => {
                self.eleve_comptes
                    .eleve_id_for_user(etablissement_id, actor_user_id)
                    .await?
                    == Some(eleve_id)
            }
            // Direction, CPE, secrétariat, accueil : un service de l'établissement, pas de lien.
            _ => true,
        };

        // Exclusions : relues à chaque appel (R-M6, R-M7, R-M8).
        if acteur == Acteur::Famille {
            // L'acteur lui-même : un lien interdit, suspendu ou échu l'exclut, même en réponse.
            let (actif, communicant): (i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM ent_eleve_lien_actif
                          WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3),
                        (SELECT count(*) FROM ent_eleve_lien_communicant
                          WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3)",
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .bind(actor_user_id)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
            .await?;
            faits.destinataire_exclu = actif > 0 && communicant == 0;
        } else if matches!(cible, Cible::Famille) {
            // (liens actifs, communicants, sollicitables) du responsable visé, ou de tous.
            let (actifs, communicants, sollicitables): (i64, i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM ent_eleve_lien_actif
                          WHERE etablissement_id = $1 AND eleve_id = $2 AND ($3::uuid IS NULL OR user_id = $3)),
                        (SELECT count(*) FROM ent_eleve_lien_communicant
                          WHERE etablissement_id = $1 AND eleve_id = $2 AND ($3::uuid IS NULL OR user_id = $3)),
                        (SELECT count(*) FROM ent_eleve_lien_sollicitable
                          WHERE etablissement_id = $1 AND eleve_id = $2 AND ($3::uuid IS NULL OR user_id = $3))",
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .bind(lien_user)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
            .await?;
            // Tous exclus par décision (interdiction, suspension) : rien à qui écrire.
            faits.destinataire_exclu = actifs > 0 && communicants == 0;
            // Reste joignable, mais personne n'est « sollicitable » (ne pas solliciter).
            faits.ne_pas_solliciter = communicants > 0 && sollicitables == 0;
        }
        Ok(faits)
    }

    // ───────────────────────── Aperçu des destinataires (R-M5) ─────────────────────────

    /// Aperçu des destinataires d'un message du personnel « aux responsables » de l'élève : les
    /// conversations qui seront créées (un fil par foyer, R-M3), avec nom d'affichage, lien et
    /// raison, **sans coordonnée**, et le nombre de responsables exclus, **sans nom ni cause**.
    pub async fn apercu_destinataires_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<ApercuDestinataires, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        if !self
            .actor_can_access_msg_fil(etablissement_id, actor_user_id, role, eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let jour = self.aujourdhui_etablissement(etablissement_id).await?;
        let majeur = self
            .eleve_est_majeur(etablissement_id, eleve_id, jour)
            .await?;
        let cible = if majeur { Cible::Eleve } else { Cible::Famille };
        let decision = self
            .peut_ecrire_for_actor(
                etablissement_id,
                actor_user_id,
                role,
                cible,
                eleve_id,
                None,
                true,
            )
            .await?;
        if majeur {
            // D23 : adressé à l'élève majeur ; ses parents n'y entrent que selon les droits
            // conservés du socle 4 (aucun n'existe encore : jamais).
            return Ok(ApercuDestinataires {
                eleve_id,
                decision,
                destinataire_est_l_eleve: true,
                conversations: Vec::new(),
                exclus: 0,
            });
        }
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            Option<Uuid>,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        )> = sqlx::query_as(
            "SELECT l.foyer_id, p.civilite, p.nom, p.prenom, l.lien
                   FROM ent_eleve_lien_sollicitable l
                   LEFT JOIN ent_personne p
                     ON p.etablissement_id = l.etablissement_id AND p.id = l.personne_id
                  WHERE l.etablissement_id = $1 AND l.eleve_id = $2
                  ORDER BY l.foyer_id NULLS LAST, l.created_at, l.id",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await?;
        let (exclus,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_eleve_lien_actif a
              WHERE a.etablissement_id = $1 AND a.eleve_id = $2
                AND NOT EXISTS (SELECT 1 FROM ent_eleve_lien_sollicitable s
                                 WHERE s.etablissement_id = a.etablissement_id AND s.id = a.id)",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await?;
        // Un foyer = une conversation ; un responsable sans foyer a la sienne (comme à l'ouverture).
        let mut conversations: Vec<(Option<Uuid>, Vec<DestinataireApercu>)> = Vec::new();
        for (foyer, civilite, nom, prenom, lien) in rows {
            let d = DestinataireApercu {
                nom_affiche: nom_affiche(civilite.as_deref(), prenom.as_deref(), nom.as_deref()),
                lien,
                raison: "titulaire_foyer",
            };
            match (foyer, conversations.last_mut()) {
                (Some(f), Some((Some(g), liste))) if f == *g => liste.push(d),
                _ => conversations.push((foyer, vec![d])),
            }
        }
        Ok(ApercuDestinataires {
            eleve_id,
            decision,
            destinataire_est_l_eleve: false,
            conversations: conversations
                .into_iter()
                .enumerate()
                .map(|(i, (_, destinataires))| ConversationApercu {
                    numero: i + 1,
                    destinataires,
                })
                .collect(),
            exclus: exclus as usize,
        })
    }

    // ───────────────────────── Séparation d'un fil commun (MSG-118, CM-03, CM-04) ─────────────────────────

    /// Sépare un fil commun à plusieurs responsables : les comptes `comptes` quittent le fil
    /// d'origine pour un **fil jumeau** qui garde tout le passé (copié) ; les nouveaux messages ne
    /// sont plus communs. Un responsable demande son propre fil (`comptes` ignoré : lui seul) ; la
    /// direction (`messagerie.fils_reserves.lire`) peut séparer des responsables. Rend l'identifiant
    /// du fil jumeau.
    pub async fn separer_fil_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        fil_id: Uuid,
        comptes: Vec<Uuid>,
    ) -> Result<Uuid, EntDbError> {
        let fil = self
            .messages
            .get_fil(etablissement_id, fil_id)
            .await?
            .ok_or(domain_eleve::DomainError::MsgFilIntrouvable)?;
        if !self
            .peut_lire_fil(etablissement_id, actor_user_id, role, &fil)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let comptes = match Role::parse(role) {
            Some(Role::Parent) => vec![actor_user_id],
            Some(r) if r.is_staff() => {
                if !self
                    .a_action_etablissement(
                        etablissement_id,
                        actor_user_id,
                        role,
                        Action::MessagerieFilsReserves,
                    )
                    .await?
                {
                    return Err(EntDbError::Forbidden);
                }
                comptes
            }
            _ => return Err(EntDbError::Forbidden),
        };
        if comptes.is_empty() {
            return Err(EntDbError::SaisieInvalide("aucun responsable à séparer"));
        }
        // Chaque compte doit participer au fil côté famille, et un autre responsable doit y rester.
        let (parmi, autres): (i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE pe.user_id = ANY($3)),
                    count(*) FILTER (WHERE NOT (pe.user_id = ANY($3)))
               FROM ent_msg_participant p
               JOIN ent_personne pe ON pe.etablissement_id = p.etablissement_id AND pe.id = p.personne_id
              WHERE p.etablissement_id = $1 AND p.fil_id = $2 AND p.au IS NULL AND p.cote = 'famille'",
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(&comptes)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await?;
        if parmi as usize != comptes.len() {
            return Err(EntDbError::Forbidden);
        }
        if autres == 0 {
            return Err(EntDbError::SaisieInvalide(
                "le fil n'a pas d'autre responsable : rien à séparer",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let (jumeau,): (Option<Uuid>,) = sqlx::query_as(
            "SELECT ent_msg_dedoubler($1, $2, 'SEPARATION', 'separation_demandee', 'separation')",
        )
        .bind(fil_id)
        .bind(&comptes)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        let jumeau = jumeau.ok_or(EntDbError::Forbidden)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::MESSAGERIE_FIL_DEDOUBLE.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: fil_id,
                eleve_id: Some(fil.eleve_id),
                payload: json!({ "fil_jumeau_id": jumeau, "comptes": comptes.len(), "motif": "separation" }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        for c in &comptes {
            self.events
                .publish(MetierEventDraft {
                    etablissement_id,
                    type_name: types::MESSAGERIE_PARTICIPANT_RETIRE.to_string(),
                    aggregate: AggregateKind::MsgFil,
                    aggregate_id: fil_id,
                    eleve_id: Some(fil.eleve_id),
                    payload: json!({ "compte": c, "motif": "separation" }),
                    actor_user_id: Some(actor_user_id.to_string()),
                })
                .await?;
        }
        Ok(jumeau)
    }

    // ───────────────────────── Majorité (R-M12, raccord socle 4) ─────────────────────────

    /// À la majorité de l'élève : ses responsables sortent des fils que l'établissement leur avait
    /// ouverts à son propos (`ent_msg_majorite_eleve`, motif codé `majorite`). Rend le nombre de
    /// participations closes. **Provisoire** : le socle 4 ne publie pas encore les droits conservés
    /// après la majorité (Q-A7) ; dès qu'il les publiera, ils seront lus ici.
    pub async fn appliquer_majorite_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<u64, EntDbError> {
        let jour = self.aujourdhui_etablissement(etablissement_id).await?;
        if !self
            .eleve_est_majeur(etablissement_id, eleve_id, jour)
            .await?
        {
            return Ok(0);
        }
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let (n,): (i32,) = sqlx::query_as("SELECT ent_msg_majorite_eleve($1, $2)")
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        if n > 0 {
            self.events
                .publish(MetierEventDraft {
                    etablissement_id,
                    type_name: types::MESSAGERIE_PARTICIPANT_RETIRE.to_string(),
                    aggregate: AggregateKind::Eleve,
                    aggregate_id: eleve_id,
                    eleve_id: Some(eleve_id),
                    payload: json!({ "motif": "majorite", "participations": n }),
                    actor_user_id: None,
                })
                .await?;
        }
        Ok(n as u64)
    }
}

/// « Mme Prénom NOM » : nom d'affichage seulement.
fn nom_affiche(civilite: Option<&str>, prenom: Option<&str>, nom: Option<&str>) -> String {
    let civ = match civilite {
        Some("mme") => "Mme",
        Some("m") => "M.",
        _ => "",
    };
    [civ, prenom.unwrap_or(""), nom.unwrap_or("")]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nom_affiche_sans_coordonnee() {
        assert_eq!(
            nom_affiche(Some("mme"), Some("Lily"), Some("Potter")),
            "Mme Lily Potter"
        );
        assert_eq!(nom_affiche(None, None, Some("Potter")), "Potter");
        assert_eq!(
            nom_affiche(Some("autre"), Some("Alex"), Some("B")),
            "Alex B"
        );
        assert_eq!(nom_affiche(None, None, None), "");
    }

    #[test]
    fn acteur_de_chaque_role_de_base() {
        use PgEnt as P;
        assert_eq!(P::acteur_matrice("parent", false), Some(Acteur::Famille));
        assert_eq!(P::acteur_matrice("eleve", false), Some(Acteur::EleveMineur));
        assert_eq!(P::acteur_matrice("eleve", true), Some(Acteur::EleveMajeur));
        assert_eq!(P::acteur_matrice("prof", false), Some(Acteur::Professeur));
        assert_eq!(
            P::acteur_matrice("cpe", false),
            Some(Acteur::CpeVieScolaire)
        );
        assert_eq!(
            P::acteur_matrice("administratif", false),
            Some(Acteur::Secretariat)
        );
        assert_eq!(
            P::acteur_matrice("direction", false),
            Some(Acteur::Direction)
        );
        assert_eq!(P::acteur_matrice("infirmerie", false), Some(Acteur::Sante));
        assert_eq!(P::acteur_matrice("accueil", false), Some(Acteur::Accueil));
        assert_eq!(P::acteur_matrice("inconnu", false), None);
    }
}
