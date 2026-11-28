//! Socle 27, palier G5 : **personnes concernées et information des personnes** (RGP-244, RGP-252 ; RGPD art. 34).
//!
//! * **Calcul** depuis l'origine : une classe (élèves affectés au jour de l'établissement), une liste d'élèves, une liste
//!   de comptes (personnels dont les identifiants ont fuité). Le nombre posé sur la violation devient **exact**.
//! * **Destinataires** (Q-RG3, valeur prudente D-069) : pour un élève **mineur**, les titulaires de l'autorité parentale
//!   dont le lien est actif et non suspendu, et l'élève lui-même s'il a 15 ans ou plus et un compte (âge du consentement
//!   numérique en France, loi du 20 juin 2018) ; pour un élève **majeur**, lui seul ; pour un compte, son titulaire.
//! * **Information** ★ : la direction relit l'aperçu (nombre de destinataires, personnes sans destinataire joignable),
//!   écrit le message, puis **confirme le nombre** affiché (confirmation humaine N3) ; l'avis part par le socle 8, selon
//!   les préférences de chacun (ce n'est pas une alerte : la plage calme est respectée). Aucune donnée touchée n'est
//!   écrite dans l'avis au-delà du message relu (E14). L'information est faite une fois ; elle est figée par la base.

use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::violations::{etape, ViolationVue};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::Action;
use serde::Serialize;
use sqlx::PgConnection;
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

/// Nature de notification déclarée au socle 8 (§18.1).
pub const KIND_PERSONNES: &str = "rgpd.personnes_violation";
/// Âge à partir duquel un élève mineur est informé lui-même, en plus de ses titulaires (D-069, Q-RG3).
pub const AGE_INFORMATION_ELEVE: u32 = 15;
/// Plafond d'une liste explicite (élèves ou comptes) : au-delà, passer par une classe ou une estimation.
pub const PLAFOND_LISTE: usize = 5000;

/// Origine du calcul des personnes concernées.
#[derive(Clone, Debug)]
pub enum OriginePersonnes {
    /// Élèves affectés à la classe au jour de l'établissement (une liste de classe divulguée, CL27-41).
    Classe(Uuid),
    /// Élèves désignés un par un.
    Eleves(Vec<Uuid>),
    /// Comptes désignés un par un (identifiants de personnels, CL27-42).
    Comptes(Vec<Uuid>),
}

/// Aperçu avant l'information (lu par la direction avant de confirmer).
#[derive(Clone, Debug, Serialize)]
pub struct ApercuInformation {
    pub personnes: i64,
    pub destinataires: i64,
    /// Personnes concernées sans aucun destinataire joignable (aucun titulaire avec compte, élève sans compte) :
    /// à informer par un autre moyen (courrier), ou communication publique (art. 34.3 c).
    pub personnes_sans_destinataire: i64,
    pub mineurs: i64,
    pub deja_informees: bool,
}

/// Âge révolu à `jour` (date de naissance exacte seulement).
fn age_le(naissance: NaiveDate, jour: NaiveDate) -> u32 {
    let mut a = jour.year() - naissance.year();
    if (jour.month(), jour.day()) < (naissance.month(), naissance.day()) {
        a -= 1;
    }
    a.max(0) as u32
}

/// Destinataires d'un élève (règle de l'en-tête). Rend `(mineur, comptes)`.
async fn destinataires_eleve(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    jour: NaiveDate,
) -> Result<(bool, Vec<Uuid>), EntDbError> {
    let (naissance, precision, majorite): (Option<NaiveDate>, Option<String>, Option<NaiveDate>) = sqlx::query_as(
        "SELECT date_naissance, date_naissance_precision::text,
                ent_eleve_majorite_le(date_naissance, date_naissance_precision, date_naissance_statut)
           FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or(invalide("élève inconnu dans cet établissement"))?;
    // Majorité inconnue (date imprécise) : traité comme mineur, la lecture la plus protectrice (titulaires informés).
    let majeur = majorite.is_some_and(|m| m <= jour);
    let compte_eleve: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM ent_eleve_compte WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY user_id LIMIT 1",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if majeur {
        return Ok((false, compte_eleve.into_iter().collect()));
    }
    let mut out: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT p.user_id
           FROM ent_eleve_lien l
           JOIN ent_personne p ON p.etablissement_id = l.etablissement_id AND p.id = l.personne_id
           JOIN ent_membership m ON m.etablissement_id = l.etablissement_id AND m.user_id = p.user_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2
            AND l.autorite_parentale AND l.suspendu_le IS NULL
            AND (l.date_debut IS NULL OR l.date_debut <= $3)
            AND (l.date_fin IS NULL OR l.date_fin >= $3)
            AND p.fusionnee_dans IS NULL AND p.user_id IS NOT NULL
          ORDER BY p.user_id",
    )
    .bind(etab)
    .bind(eleve)
    .bind(jour)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let exacte = precision.as_deref().is_none_or(|p| p == "jour");
    if let (Some(n), Some(c), true) = (naissance, compte_eleve, exacte) {
        if age_le(n, jour) >= AGE_INFORMATION_ELEVE && !out.contains(&c) {
            out.push(c);
        }
    }
    Ok((true, out))
}

async fn eleves_de_classe(
    conn: &mut PgConnection,
    etab: Uuid,
    classe: Uuid,
    jour: NaiveDate,
) -> Result<Vec<Uuid>, EntDbError> {
    let existe: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_classe WHERE etablissement_id = $1 AND id = $2)",
    )
    .bind(etab)
    .bind(classe)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !existe {
        return Err(invalide("classe inconnue dans cet établissement"));
    }
    sqlx::query_scalar(
        "SELECT DISTINCT a.eleve_id
           FROM ent_affectation a
           JOIN ent_eleve e ON e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
          WHERE a.etablissement_id = $1 AND a.classe_id = $2 AND a.date_debut <= $3
            AND (a.date_fin IS NULL OR a.date_fin >= $3)
            AND (e.date_sortie IS NULL OR e.date_sortie > $3)
          ORDER BY a.eleve_id",
    )
    .bind(etab)
    .bind(classe)
    .bind(jour)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)
}

async fn apercu(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<ApercuInformation, EntDbError> {
    let (personnes, destinataires, sans, deja): (i64, i64, i64, bool) = sqlx::query_as(
        "SELECT count(DISTINCT (sujet_type, sujet_id)),
                count(DISTINCT destinataire_user_id),
                (SELECT count(*) FROM (
                    SELECT sujet_type, sujet_id FROM rgp_violation_personne
                     WHERE etablissement_id = $1 AND violation_id = $2
                     GROUP BY sujet_type, sujet_id HAVING count(destinataire_user_id) = 0) s),
                (SELECT personnes_informees_le IS NOT NULL FROM rgp_violation WHERE etablissement_id = $1 AND id = $2)
           FROM rgp_violation_personne WHERE etablissement_id = $1 AND violation_id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let mineurs: i64 = sqlx::query_scalar("SELECT COALESCE(personnes_mineurs::int, 0)::bigint FROM rgp_violation WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    Ok(ApercuInformation {
        personnes,
        destinataires,
        personnes_sans_destinataire: sans,
        mineurs,
        deja_informees: deja,
    })
}

struct EtatViolation {
    risque: Option<String>,
    informees: bool,
}

async fn etat_verrouille(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<EtatViolation, EntDbError> {
    let r: Option<(String, String, Option<String>, bool)> = sqlx::query_as(
        "SELECT etat, nature_incident, risque, personnes_informees_le IS NOT NULL
           FROM rgp_violation WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let (etat, nature, risque, informees) = r.ok_or(EntDbError::Domain(DomainError::NotFound))?;
    if etat == "close" {
        return Err(EntDbError::ConflitDossier(
            "la violation est close : le registre ne se modifie plus",
        ));
    }
    if nature != "violation" {
        return Err(EntDbError::ConflitDossier(
            "un incident de sécurité sans atteinte aux données n'a pas de personnes à informer",
        ));
    }
    Ok(EtatViolation { risque, informees })
}

impl PgEnt {
    /// **RGP-244** — calcule les personnes concernées depuis l'origine et rend la fiche. Remplace le calcul précédent
    /// tant que les personnes n'ont pas été informées ; le nombre de la violation devient exact (non estimé).
    pub async fn violations_calculer_personnes(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        origine: OriginePersonnes,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        match &origine {
            OriginePersonnes::Eleves(l) | OriginePersonnes::Comptes(l)
                if l.is_empty() || l.len() > PLAFOND_LISTE =>
            {
                return Err(invalide("liste vide ou trop longue (5 000 au plus) : passez par une classe ou une estimation"));
            }
            _ => {}
        }
        let jour = self.aujourdhui_etablissement(etab).await?;
        let maintenant = Utc::now();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let e = etat_verrouille(&mut tx, etab, id).await?;
        if e.informees {
            return Err(EntDbError::ConflitDossier(
                "les personnes ont déjà été informées : le calcul est figé",
            ));
        }
        sqlx::query(
            "DELETE FROM rgp_violation_personne WHERE etablissement_id = $1 AND violation_id = $2",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let (type_origine, id_origine, lignes, mineurs): (
            &str,
            Option<Uuid>,
            Vec<(&str, Uuid, Option<Uuid>)>,
            bool,
        ) = match origine {
            OriginePersonnes::Classe(c) => {
                let eleves = eleves_de_classe(&mut tx, etab, c, jour).await?;
                let (l, m) = lignes_eleves(&mut tx, etab, &eleves, jour).await?;
                ("classe", Some(c), l, m)
            }
            OriginePersonnes::Eleves(mut l) => {
                l.sort_unstable();
                l.dedup();
                let (l, m) = lignes_eleves(&mut tx, etab, &l, jour).await?;
                ("eleves", None, l, m)
            }
            OriginePersonnes::Comptes(mut l) => {
                l.sort_unstable();
                l.dedup();
                let membres: Vec<Uuid> = sqlx::query_scalar(
                        "SELECT DISTINCT user_id FROM ent_membership WHERE etablissement_id = $1 AND user_id = ANY($2)",
                    )
                    .bind(etab)
                    .bind(&l)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(db)?;
                if membres.len() != l.len() {
                    return Err(invalide(
                        "un compte désigné n'est pas rattaché à cet établissement",
                    ));
                }
                (
                    "comptes",
                    None,
                    membres
                        .into_iter()
                        .map(|u| ("compte", u, Some(u)))
                        .collect(),
                    false,
                )
            }
        };
        for (sujet_type, sujet, dest) in &lignes {
            sqlx::query(
                "INSERT INTO rgp_violation_personne (id, etablissement_id, violation_id, sujet_type, sujet_id, destinataire_user_id)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(*sujet_type)
            .bind(*sujet)
            .bind(*dest)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let a = apercu(&mut tx, etab, id).await?;
        sqlx::query(
            "UPDATE rgp_violation
                SET personnes_nombre = $3, personnes_estime = false, personnes_mineurs = personnes_mineurs OR $4,
                    personnes_origine_type = $5, personnes_origine_id = $6, modifie_le = $7
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(a.personnes as i32)
        .bind(mineurs)
        .bind(type_origine)
        .bind(id_origine)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        etape(
            &mut tx,
            etab,
            id,
            "personnes_calculees",
            Some(&format!(
                "{} personne(s), {} destinataire(s), origine : {type_origine}",
                a.personnes, a.destinataires
            )),
            Some(acteur),
            Some(role),
        )
        .await?;
        tx.commit().await?;
        self.violations_fiche(etab, acteur, role, id).await
    }

    /// Aperçu avant l'information des personnes (relu par la direction, puis confirmé par le nombre).
    pub async fn violations_apercu_information(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ApercuInformation, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM rgp_violation WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        let a = apercu(&mut tx, etab, id).await?;
        tx.commit().await?;
        Ok(a)
    }

    /// **RGP-252 ★** — informe les personnes concernées (art. 34), après **confirmation humaine** : `confirmation` doit
    /// être le nombre de destinataires de l'aperçu (si le calcul a changé entre-temps, rien ne part). Exige un risque
    /// décidé ; obligatoire pour « élevé », possible pour « avéré » par choix de la direction ; jamais pour « faible »
    /// (on n'inquiète pas sans raison). Une seule fois.
    #[allow(clippy::too_many_arguments)]
    pub async fn violations_informer_personnes(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        message: &str,
        confirmation: i64,
    ) -> Result<ViolationVue, EntDbError> {
        self.violations_informer_personnes_a(
            etab,
            acteur,
            role,
            id,
            message,
            confirmation,
            Utc::now(),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn violations_informer_personnes_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        message: &str,
        confirmation: i64,
        maintenant: DateTime<Utc>,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsDecider)
            .await?;
        let message = message.trim();
        let n = message.chars().count();
        if !(20..=4000).contains(&n) || message.chars().any(|c| c.is_control() && c != '\n') {
            return Err(invalide(
                "message aux personnes : 20 à 4 000 caractères, en langage clair",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let e = etat_verrouille(&mut tx, etab, id).await?;
        if e.informees {
            return Err(EntDbError::ConflitDossier(
                "les personnes ont déjà été informées",
            ));
        }
        match e.risque.as_deref() {
            Some("eleve") | Some("avere") => {}
            Some(_) => return Err(EntDbError::ConflitDossier(
                "risque décidé faible : l'information des personnes n'est pas exigée (art. 34.1)",
            )),
            None => {
                return Err(EntDbError::ConflitDossier(
                    "décider le risque avant d'informer les personnes",
                ))
            }
        }
        let a = apercu(&mut tx, etab, id).await?;
        if a.personnes == 0 {
            return Err(EntDbError::ConflitDossier(
                "calculer d'abord les personnes concernées",
            ));
        }
        if a.destinataires == 0 {
            return Err(EntDbError::ConflitDossier(
                "aucun destinataire joignable : informez par un autre moyen (courrier) puis motivez l'exception (art. 34.3)",
            ));
        }
        if confirmation != a.destinataires {
            return Err(EntDbError::ConflitDossier(
                "le nombre confirmé ne correspond plus aux destinataires : relisez l'aperçu",
            ));
        }
        let reference: String = sqlx::query_scalar(
            "SELECT 'V-' || annee || '-' || lpad(numero::text, 3, '0') FROM rgp_violation WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let dpo: Option<String> = sqlx::query_scalar(
            "SELECT dpo_contact FROM rgp_gouvernance WHERE etablissement_id = $1 AND dpo_contact IS NOT NULL",
        )
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let corps = format!(
            "{message}{}",
            dpo.map(|c| format!("\n\nContact du délégué à la protection des données : {c}."))
                .unwrap_or_default()
        );
        let destinataires: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT destinataire_user_id FROM rgp_violation_personne
              WHERE etablissement_id = $1 AND violation_id = $2 AND destinataire_user_id IS NOT NULL
              ORDER BY destinataire_user_id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for u in &destinataires {
            notifier_personne(
                &mut tx,
                EnvoiAPlanifier {
                    etablissement_id: etab,
                    user_id: *u,
                    kind: KIND_PERSONNES,
                    titre: "Information sur un incident de données",
                    corps: &corps,
                    eleve_id: None,
                    notification_id: None,
                    source_type: "systeme",
                    source_id: id,
                    cle_dedup: format!("rgpd:violation:{id}:personnes"),
                    droit_requis: None,
                    regroupable: false,
                    canaux_voulus: &["email", "push"],
                },
                id,
                maintenant,
            )
            .await?;
        }
        sqlx::query(
            "UPDATE rgp_violation_personne SET informee_le = $3
              WHERE etablissement_id = $1 AND violation_id = $2 AND destinataire_user_id IS NOT NULL",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE rgp_violation SET personnes_informees_le = $3, personnes_informees_nombre = $4,
                    personnes_non_information_motif = NULL, modifie_le = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .bind(destinataires.len() as i32)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        etape(
            &mut tx,
            etab,
            id,
            "personnes_informees",
            Some(&format!(
                "{} destinataire(s) informé(s) pour {} personne(s) ({reference}) ; {} sans destinataire joignable",
                destinataires.len(),
                a.personnes,
                a.personnes_sans_destinataire
            )),
            Some(acteur),
            Some(role),
        )
        .await?;
        tx.commit().await?;
        self.violations_fiche(etab, acteur, role, id).await
    }
}

type Ligne = (&'static str, Uuid, Option<Uuid>);

async fn lignes_eleves(
    conn: &mut PgConnection,
    etab: Uuid,
    eleves: &[Uuid],
    jour: NaiveDate,
) -> Result<(Vec<Ligne>, bool), EntDbError> {
    let mut out = Vec::new();
    let mut mineurs = false;
    for e in eleves {
        let (mineur, dests) = destinataires_eleve(conn, etab, *e, jour).await?;
        mineurs |= mineur;
        if dests.is_empty() {
            out.push(("eleve", *e, None));
        }
        for d in dests {
            out.push(("eleve", *e, Some(d)));
        }
    }
    Ok((out, mineurs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_revolu() {
        let d = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        assert_eq!(age_le(d("2012-03-10"), d("2027-03-09")), 14);
        assert_eq!(age_le(d("2012-03-10"), d("2027-03-10")), 15);
        assert_eq!(age_le(d("2012-02-29"), d("2027-02-28")), 14);
        assert_eq!(age_le(d("2030-01-01"), d("2027-01-01")), 0);
    }
}
