//! Socle 27, palier G5 : **violations de la plateforme** (RGP-254, RGP-255, RGP-311 ; `27_RGPD_ARCHIVES.md` §8.7.6,
//! R-RG36, CL27-44).
//!
//! Une violation chez l'éditeur ou chez un sous-traitant de la plateforme est déclarée **une fois** par l'éditeur
//! (`rgp_violation_plateforme`, table de plateforme sans établissement ni RLS), puis **propagée** à chaque établissement
//! concerné avec ses seuls éléments : une violation d'origine `plateforme` est créée dans le registre de
//! l'établissement (son horloge de 72 h part de l'instant où l'éditeur l'informe, art. 33.2), la direction est
//! alertée comme pour toute déclaration, et décide ensuite de sa propre notification.
//!
//! Ces appels sont des **appels système** de la console de l'éditeur : la commande d'opérateur `ent-api
//! plateforme-violation` (complément G5), comme `ent-api plateforme-domaine`. Aucune route HTTP ne les expose, aucun
//! droit d'établissement ne les permet (même régime que les sous-traitants de la plateforme, G2).

use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme, TenantTx};
use crate::violations::{alerter_declaration, etape, inserer, Insertion};
use chrono::{DateTime, Datelike, Duration, Utc};
use domain_eleve::DomainError;
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

fn texte(v: &str, min: usize, max: usize, m: &'static str) -> Result<String, EntDbError> {
    let t = v.trim();
    let n = t.chars().count();
    if n < min || n > max || t.chars().any(|c| c.is_control() && c != '\n') {
        return Err(invalide(m));
    }
    Ok(t.to_string())
}

/// Ce que l'éditeur déclare (RGP-254).
#[derive(Clone, Debug, Default)]
pub struct DeclarationPlateforme<'a> {
    pub intitule: &'a str,
    pub description: &'a str,
    /// Prise de connaissance par l'éditeur.
    pub connue_le: DateTime<Utc>,
    pub confidentialite: bool,
    pub integrite: bool,
    pub disponibilite: bool,
    /// La couche compte (identifiants, passkeys, rattachements), dont ScolIA est responsable, est touchée (R-RG50).
    pub couche_compte: bool,
}

/// Référence lisible de l'éditeur : « P-2027-002 ».
pub fn reference_plateforme(annee: i32, numero: i64) -> String {
    format!("P-{annee}-{numero:03}")
}

impl PgEnt {
    /// **RGP-254 (appel système)** — l'éditeur déclare une violation de la plateforme. Rend l'identifiant et la
    /// référence « P-AAAA-NNN » (numérotée par année de prise de connaissance, UTC : la plateforme n'a pas de fuseau).
    pub async fn violations_plateforme_declarer(
        &self,
        declaree_par: &str,
        d: DeclarationPlateforme<'_>,
        maintenant: DateTime<Utc>,
    ) -> Result<(Uuid, String), EntDbError> {
        let declaree_par = texte(
            declaree_par,
            2,
            120,
            "déclarant de l'éditeur : 2 à 120 caractères",
        )?;
        let intitule = texte(d.intitule, 3, 200, "intitulé : 3 à 200 caractères")?;
        let description = texte(d.description, 3, 4000, "description : 3 à 4 000 caractères")?;
        if !(d.confidentialite || d.integrite || d.disponibilite) {
            return Err(invalide(
                "dites ce qui est atteint : confidentialité, intégrité ou disponibilité",
            ));
        }
        if d.connue_le > maintenant
            || d.connue_le < maintenant - Duration::days(crate::violations::CONNUE_LE_MAX_JOURS)
        {
            return Err(invalide(
                "prise de connaissance : dans l'année écoulée, jamais dans le futur",
            ));
        }
        let annee = d.connue_le.year();
        let mut tx = self.pool().begin().await.map_err(db)?;
        // Numérotation sans trou ni doublon : un verrou par année sérialise les déclarations de l'éditeur.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("rgp_violation_plateforme:{annee}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let dernier: i64 = sqlx::query_scalar(
            "SELECT COALESCE(max(split_part(reference, '-', 3)::bigint), 0) FROM rgp_violation_plateforme
              WHERE reference LIKE $1",
        )
        .bind(format!("P-{annee}-%"))
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let reference = reference_plateforme(annee, dernier + 1);
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO rgp_violation_plateforme (id, reference, connue_le, declaree_le, declaree_par, intitule,
                    description, confidentialite, integrite, disponibilite, couche_compte, cree_le, modifie_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $4, $4)",
        )
        .bind(id)
        .bind(&reference)
        .bind(d.connue_le)
        .bind(maintenant)
        .bind(&declaree_par)
        .bind(&intitule)
        .bind(&description)
        .bind(d.confidentialite)
        .bind(d.integrite)
        .bind(d.disponibilite)
        .bind(d.couche_compte)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok((id, reference))
    }

    /// **RGP-254 / RGP-255 (appel système)** — propage une violation de la plateforme à UN établissement concerné, avec
    /// les éléments qui le concernent seul (ses comptes, ses données). Crée la violation d'origine `plateforme` dans son
    /// registre et alerte ses gestionnaires. Idempotent : une seconde propagation au même établissement rend la même
    /// violation (`false` : déjà propagée) sans rien réécrire.
    pub async fn violations_plateforme_propager(
        &self,
        violation_plateforme: Uuid,
        etab: Uuid,
        elements: &str,
        personnes_nombre: Option<i32>,
        maintenant: DateTime<Utc>,
    ) -> Result<(Uuid, bool), EntDbError> {
        let elements = texte(
            elements,
            3,
            4000,
            "éléments propres à l'établissement : 3 à 4 000 caractères",
        )?;
        if personnes_nombre.is_some_and(|n| n < 0) {
            return Err(invalide("nombre de personnes négatif"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Deux propagations simultanées au même établissement se suivent : la seconde trouve la première.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("rgp_violation_plateforme:{violation_plateforme}:{etab}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let p: Option<(String, String, bool, bool, bool, String)> = sqlx::query_as(
            "SELECT reference, intitule, confidentialite, integrite, disponibilite, etat
               FROM rgp_violation_plateforme WHERE id = $1 FOR SHARE",
        )
        .bind(violation_plateforme)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (reference, intitule, c, i, d, etat) =
            p.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let deja: Option<Uuid> = sqlx::query_scalar(
            "SELECT violation_id FROM rgp_violation_plateforme_etab
              WHERE etablissement_id = $1 AND violation_plateforme_id = $2",
        )
        .bind(etab)
        .bind(violation_plateforme)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(v) = deja {
            tx.commit().await?;
            return Ok((v, false));
        }
        if etat == "close" {
            return Err(EntDbError::ConflitDossier(
                "violation de la plateforme close : plus de propagation",
            ));
        }
        // L'intitulé porte la référence de l'éditeur ; tronqué au besoin à la longueur du registre.
        let titre: String = format!("{reference} : {intitule}")
            .chars()
            .take(200)
            .collect();
        let (id, _, _) = inserer(
            &mut tx,
            etab,
            Insertion {
                nature_incident: "violation",
                origine: "plateforme",
                plateforme_violation_id: Some(violation_plateforme),
                source_code: None,
                source_ref: None,
                declaree_par: None,
                intitule: &titre,
                description: Some(&elements),
                constatee_le: None,
                // L'horloge de l'établissement part de l'instant où le sous-traitant l'informe (art. 33.2).
                connue_le: maintenant,
                personnes_nombre,
            },
            maintenant,
        )
        .await?;
        // Pré-qualification : la nature de l'atteinte connue de l'éditeur ; la direction qualifie ensuite.
        sqlx::query(
            "UPDATE rgp_violation SET confidentialite = $3, integrite = $4, disponibilite = $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(c)
        .bind(i)
        .bind(d)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO rgp_violation_plateforme_etab (id, etablissement_id, violation_plateforme_id, violation_id,
                    elements, personnes_nombre, informe_le, cree_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(violation_plateforme)
        .bind(id)
        .bind(&elements)
        .bind(personnes_nombre)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        etape(
            &mut tx,
            etab,
            id,
            "propagee",
            Some(&format!(
                "violation de la plateforme {reference}, signalée par l'éditeur"
            )),
            None,
            Some("plateforme"),
        )
        .await?;
        alerter_declaration(self, &mut tx, etab, id, None, maintenant).await?;
        tx.commit().await?;
        Ok((id, true))
    }

    /// **RGP-311 (appel système)** — ScolIA, responsable de la couche compte, saisit sa propre notification à la CNIL.
    /// Réservé aux violations qui touchent la couche compte ; une seule fois.
    pub async fn violations_plateforme_notifier_cnil(
        &self,
        violation_plateforme: Uuid,
        notifiee_le: DateTime<Utc>,
        reference_cnil: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let reference_cnil = texte(
            reference_cnil,
            2,
            120,
            "référence CNIL : 2 à 120 caractères",
        )?;
        if notifiee_le > maintenant {
            return Err(invalide("date de notification dans le futur"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let r: Option<(bool, bool, DateTime<Utc>)> = sqlx::query_as(
            "SELECT couche_compte, notifiee_cnil_le IS NOT NULL, connue_le FROM rgp_violation_plateforme
              WHERE id = $1 FOR UPDATE",
        )
        .bind(violation_plateforme)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (couche_compte, deja, connue_le) =
            r.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if !couche_compte {
            return Err(EntDbError::ConflitDossier(
                "ScolIA n'est que sous-traitant : chaque établissement notifie pour sa part",
            ));
        }
        if deja {
            return Err(EntDbError::ConflitDossier(
                "notification de ScolIA déjà saisie",
            ));
        }
        if notifiee_le < connue_le {
            return Err(invalide(
                "notification antérieure à la prise de connaissance",
            ));
        }
        sqlx::query(
            "UPDATE rgp_violation_plateforme SET notifiee_cnil_le = $2, reference_cnil = $3, modifie_le = $4 WHERE id = $1",
        )
        .bind(violation_plateforme)
        .bind(notifiee_le)
        .bind(&reference_cnil)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// **Appel système** — l'éditeur clôt sa violation : plus de propagation. Les violations propagées restent au
    /// registre de chaque établissement, qui les instruit et les clôt lui-même.
    pub async fn violations_plateforme_clore(
        &self,
        violation_plateforme: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let n = sqlx::query(
            "UPDATE rgp_violation_plateforme SET etat = 'close', close_le = $2, modifie_le = $2
              WHERE id = $1 AND etat = 'ouverte'",
        )
        .bind(violation_plateforme)
        .bind(maintenant)
        .execute(self.pool())
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(EntDbError::ConflitDossier(
                "violation de la plateforme inconnue ou déjà close",
            ));
        }
        Ok(())
    }
}

/// Une violation de la plateforme, vue de la console de l'éditeur.
#[derive(Clone, Debug)]
pub struct ViolationPlateformeVue {
    pub id: Uuid,
    pub reference: String,
    pub intitule: String,
    pub description: String,
    pub connue_le: DateTime<Utc>,
    pub declaree_le: DateTime<Utc>,
    pub declaree_par: String,
    pub confidentialite: bool,
    pub integrite: bool,
    pub disponibilite: bool,
    pub couche_compte: bool,
    pub notifiee_cnil_le: Option<DateTime<Utc>>,
    pub reference_cnil: Option<String>,
    pub etat: String,
    pub close_le: Option<DateTime<Utc>>,
}

/// Ce que l'éditeur sait d'une propagation : l'établissement informé, quand, et la référence de la violation dans son
/// registre (pour le suivi). Jamais ce que l'établissement en a décidé (qualification, risque, notification) : c'est
/// sa décision de responsable de traitement (D-069).
#[derive(Clone, Debug)]
pub struct PropagationVue {
    pub etablissement_id: Uuid,
    pub etablissement_slug: String,
    pub etablissement_nom: String,
    pub informe_le: DateTime<Utc>,
    pub reference_locale: String,
    pub personnes_nombre: Option<i32>,
}

/// Fiche publique d'un établissement (slug, nom), pour la confirmation d'une propagation.
#[derive(Clone, Debug)]
pub struct EtablissementCible {
    pub id: Uuid,
    pub slug: String,
    pub nom: String,
}

const COLONNES_P: &str = "id, reference, intitule, description, connue_le, declaree_le, declaree_par, confidentialite, \
     integrite, disponibilite, couche_compte, notifiee_cnil_le, reference_cnil, etat, close_le";

type LigneP = (
    Uuid,
    String,
    String,
    String,
    DateTime<Utc>,
    DateTime<Utc>,
    String,
    bool,
    bool,
    bool,
    bool,
    Option<DateTime<Utc>>,
    Option<String>,
    String,
    Option<DateTime<Utc>>,
);

fn vue_p(r: LigneP) -> ViolationPlateformeVue {
    ViolationPlateformeVue {
        id: r.0,
        reference: r.1,
        intitule: r.2,
        description: r.3,
        connue_le: r.4,
        declaree_le: r.5,
        declaree_par: r.6,
        confidentialite: r.7,
        integrite: r.8,
        disponibilite: r.9,
        couche_compte: r.10,
        notifiee_cnil_le: r.11,
        reference_cnil: r.12,
        etat: r.13,
        close_le: r.14,
    }
}

impl PgEnt {
    /// **Console de l'éditeur** — les violations de la plateforme, ouvertes d'abord, puis de la plus récente à la plus
    /// ancienne (500 au plus).
    pub async fn violations_plateforme_lister(&self) -> Result<Vec<ViolationPlateformeVue>, EntDbError> {
        let mut tx = self.pool().begin().await.map_err(db)?;
        let rows: Vec<LigneP> = sqlx::query_as(&format!(
            "SELECT {COLONNES_P} FROM rgp_violation_plateforme
              ORDER BY (etat = 'ouverte') DESC, connue_le DESC, reference DESC LIMIT 500"
        ))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows.into_iter().map(vue_p).collect())
    }

    /// **Console de l'éditeur** — une violation de la plateforme, par sa référence (« P-2027-002 ») ou son identifiant.
    pub async fn violations_plateforme_trouver(
        &self,
        reference_ou_id: &str,
    ) -> Result<ViolationPlateformeVue, EntDbError> {
        let cle = reference_ou_id.trim();
        let id = Uuid::parse_str(cle).ok();
        let mut tx = self.pool().begin().await.map_err(db)?;
        let r: Option<LigneP> = sqlx::query_as(&format!(
            "SELECT {COLONNES_P} FROM rgp_violation_plateforme WHERE id = $1 OR reference = upper($2)"
        ))
        .bind(id.unwrap_or_else(Uuid::nil))
        .bind(cle)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        r.map(vue_p).ok_or(EntDbError::Domain(DomainError::NotFound))
    }

    /// **Console de l'éditeur** — l'établissement visé par une propagation (pour la confirmation par le slug).
    pub async fn violations_plateforme_cible(&self, etab: Uuid) -> Result<EtablissementCible, EntDbError> {
        let mut tx = self.pool().begin().await.map_err(db)?;
        let r: Option<(String, String)> =
            sqlx::query_as("SELECT slug, nom FROM ent_etablissement WHERE id = $1")
                .bind(etab)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let (slug, nom) = r.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        Ok(EtablissementCible { id: etab, slug, nom })
    }

    /// **Console de l'éditeur** — à quels établissements une violation de la plateforme a été propagée. La table des
    /// propagations est sous RLS : chaque établissement est lu dans son propre contexte (contexte système nommé).
    pub async fn violations_plateforme_propagations(
        &self,
        violation_plateforme: Uuid,
    ) -> Result<Vec<PropagationVue>, EntDbError> {
        let mut out = Vec::new();
        for etab in etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("console_violations_plateforme"))
            .await
            .map_err(db)?
        {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            let r: Option<(DateTime<Utc>, Option<i32>, i32, i32, String, String)> = sqlx::query_as(
                "SELECT e.informe_le, e.personnes_nombre, v.annee, v.numero,
                        COALESCE(x.slug, e.etablissement_id::text), COALESCE(x.nom, 'établissement sans fiche')
                   FROM rgp_violation_plateforme_etab e
                   JOIN rgp_violation v ON v.etablissement_id = e.etablissement_id AND v.id = e.violation_id
                   LEFT JOIN ent_etablissement x ON x.id = e.etablissement_id
                  WHERE e.etablissement_id = $1 AND e.violation_plateforme_id = $2",
            )
            .bind(etab)
            .bind(violation_plateforme)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await?;
            if let Some((informe_le, personnes_nombre, annee, numero, slug, nom)) = r {
                out.push(PropagationVue {
                    etablissement_id: etab,
                    etablissement_slug: slug,
                    etablissement_nom: nom,
                    informe_le,
                    reference_locale: crate::violations::reference(annee, numero),
                    personnes_nombre,
                });
            }
        }
        out.sort_by(|a, b| a.informe_le.cmp(&b.informe_le).then(a.etablissement_slug.cmp(&b.etablissement_slug)));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_lisible() {
        assert_eq!(reference_plateforme(2027, 2), "P-2027-002");
        assert_eq!(reference_plateforme(2027, 1234), "P-2027-1234");
    }
}
