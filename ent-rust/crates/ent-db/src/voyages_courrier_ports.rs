//! Socle 42 : **ports du courrier sortant** posés par D-065 (`docs/DECISIONS.md`) pour les e-mails aux
//! prestataires et à la restauration. L'**envoi réel** attend le socle 8, lot N6 ; ici, les règles et les
//! tables qui n'ont pas à changer quand l'outil d'envoi et de réception sera branché.
//!
//! * **Expédition** : chaque établissement écrit depuis le sous-domaine de son **code UAI**, en minuscules
//!   (`ne-pas-repondre@0761713z.scolia.fr`). L'adresse est **toujours déduite de l'établissement du dossier**
//!   (profil de l'établissement), jamais fournie par un utilisateur : aucun établissement n'envoie depuis le
//!   domaine d'un autre ;
//! * **Réponse** : adresse **à usage unique** sur le même sous-domaine, `r-<clé aléatoire>@<uai>.scolia.fr`.
//!   La clé ne porte aucune donnée en clair : seule son empreinte est conservée et renvoie, côté serveur, vers
//!   l'établissement, le module, le dossier et le destinataire. **Double contrôle** : la clé doit appartenir à
//!   l'établissement du sous-domaine, sinon rejet (file de tri humaine) ; elle expire, se révoque, et ne sert
//!   qu'une fois ;
//! * remplace la boîte `boite+<établissement>@` de Next.js (VD-45 : un message n'est jamais « traité » sans
//!   rattachement confirmé).
//!
//! Ces règles sont **sans effet de bord** sur Next.js (aucune boîte existante n'est lue ni modifiée).

use chrono::{DateTime, Duration, Utc};
use ent_auth::Action;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::structure_etab::uai_valide;
use crate::voyages_repo::{charger_avec_acces, interdit, refus, tx};

/// Zone DNS du service : les sous-domaines d'établissement y sont créés par ScolIA (D-065, lot N6).
pub const ZONE_COURRIER: &str = "scolia.fr";
/// Durée de vie d'une adresse de réponse (une réponse de prestataire arrive en quelques jours à semaines).
pub const DUREE_REPONSE_JOURS: i64 = 60;

/// Sous-domaine d'envoi d'un établissement : `<uai en minuscules>.scolia.fr`. `None` si l'UAI est absent ou
/// de format invalide (aucun envoi n'est alors possible depuis cet établissement).
pub fn sous_domaine(uai: &str) -> Option<String> {
    let u = uai.trim().to_ascii_uppercase();
    uai_valide(&u).then(|| format!("{}.{ZONE_COURRIER}", u.to_ascii_lowercase()))
}

/// Adresse d'expédition d'un établissement.
pub fn adresse_expedition(uai: &str) -> Option<String> {
    sous_domaine(uai).map(|d| format!("ne-pas-repondre@{d}"))
}

/// Adresse de réponse à usage unique pour une clé donnée.
pub fn adresse_reponse(uai: &str, cle: &str) -> Option<String> {
    sous_domaine(uai).map(|d| format!("r-{cle}@{d}"))
}

/// Découpe une adresse de réponse : `(clé, sous-domaine)`. Refuse tout ce qui n'a pas exactement la forme
/// `r-<64 hexadécimaux>@<uai>.scolia.fr` (casse ignorée).
pub fn decouper_reponse(adresse: &str) -> Option<(String, String)> {
    let a = adresse.trim().to_ascii_lowercase();
    let (local, domaine) = a.split_once('@')?;
    let cle = local.strip_prefix("r-")?;
    if cle.len() != 64 || !cle.chars().all(|c| c.is_ascii_hexdigit()) || domaine.contains('@') {
        return None;
    }
    let uai = domaine.strip_suffix(&format!(".{ZONE_COURRIER}"))?;
    if uai.contains('.') || !uai_valide(&uai.to_ascii_uppercase()) {
        return None;
    }
    Some((cle.to_string(), domaine.to_string()))
}

fn nouvelle_cle() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn empreinte(cle: &str) -> String {
    ent_object::sha256_hex(cle.as_bytes())
}

/// À quoi renvoie une adresse de réponse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CibleReponse {
    /// Un prestataire de l'annuaire, à l'adresse vérifiée qui a reçu le message.
    Prestataire { prestataire_id: Uuid, adresse_id: Uuid },
    /// La cuisine de l'établissement.
    Restauration,
}

/// Résultat de la résolution d'une adresse de réponse reçue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reponse {
    /// Clé valide : le dossier visé (usage unique : la clé est consommée).
    Rattachee { sortie_id: Uuid, cible: CibleReponse },
    /// Tout le reste : clé inconnue, d'un autre établissement, expirée, révoquée ou déjà utilisée, adresse mal
    /// formée. Le message va en **file de tri humaine** ; la raison n'est donnée qu'au journal technique.
    ATrier(&'static str),
}

#[derive(sqlx::FromRow)]
struct CleLigne {
    id: Uuid,
    sortie_id: Uuid,
    objet: String,
    prestataire_id: Option<Uuid>,
    adresse_id: Option<Uuid>,
    expire_le: DateTime<Utc>,
    utilisee_le: Option<DateTime<Utc>>,
    revoquee_le: Option<DateTime<Utc>>,
}

async fn uai_de(conn: &mut PgConnection, etab: Uuid) -> Result<Option<String>, EntDbError> {
    let u: Option<Option<String>> =
        sqlx::query_scalar("SELECT uai FROM ent_etablissement_profil WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_optional(conn)
            .await
            .map_err(db)?;
    Ok(u.flatten())
}

impl PgEnt {
    /// Adresse d'expédition de l'établissement, déduite de son profil (jamais fournie par l'appelant).
    pub async fn voyages_courrier_expediteur(&self, etab: Uuid) -> Result<Option<String>, EntDbError> {
        let mut t = crate::tenant_tx::TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let uai = uai_de(&mut t, etab).await?;
        t.commit().await.map_err(db)?;
        Ok(uai.as_deref().and_then(adresse_expedition))
    }

    /// Émet une adresse de réponse à usage unique pour un message sortant de la sortie. Droit : porteur ou
    /// `voyages.prestataires` pour un prestataire, porteur ou `voyages.modifier` pour la restauration.
    /// `None` si l'établissement n'a pas d'UAI valide. L'adresse complète n'est retournée **qu'ici**.
    pub async fn voyage_courrier_reponse_emettre(
        &self,
        a: &Acteur<'_>,
        sortie: Uuid,
        cible: CibleReponse,
    ) -> Result<Option<String>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, sortie, false).await?;
        let (droit, presta, adr) = match &cible {
            CibleReponse::Prestataire { prestataire_id, adresse_id } => {
                (Action::VoyagesPrestataires, Some(*prestataire_id), Some(*adresse_id))
            }
            CibleReponse::Restauration => (Action::VoyagesModifier, None, None),
        };
        if !acces.gere(droit) {
            return Err(interdit());
        }
        if let (Some(p), Some(ad)) = (presta, adr) {
            // L'adresse doit être une adresse ACTIVE de CE prestataire, dans CET établissement.
            let ok: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_prestataire_adresse
                                 WHERE etablissement_id = $1 AND prestataire_id = $2 AND id = $3 AND desactivee_le IS NULL)",
            )
            .bind(a.etablissement_id)
            .bind(p)
            .bind(ad)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
        }
        let Some(uai) = uai_de(&mut t, a.etablissement_id).await? else {
            return Ok(None);
        };
        let cle = nouvelle_cle();
        let Some(adresse) = adresse_reponse(&uai, &cle) else {
            return Ok(None);
        };
        sqlx::query(
            "INSERT INTO ent_sortie_reponse_cle
                 (id, etablissement_id, cle_hash, sortie_id, objet, prestataire_id, adresse_id, expire_le)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(empreinte(&cle))
        .bind(sortie)
        .bind(if presta.is_some() { "prestataire" } else { "restauration" })
        .bind(presta)
        .bind(adr)
        .bind(Utc::now() + Duration::days(DUREE_REPONSE_JOURS))
        .execute(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(Some(adresse))
    }

    /// Résout une adresse de réponse reçue sur le sous-domaine de l'établissement `etab_du_sous_domaine`
    /// (l'outil de réception associe le sous-domaine à l'établissement avant d'appeler). **Usage unique** : une
    /// clé valide est consommée. Tout doute (clé d'un autre établissement, expirée, révoquée, déjà utilisée)
    /// renvoie [`Reponse::ATrier`] : jamais de rattachement deviné.
    pub async fn voyage_courrier_reponse_resoudre(
        &self,
        etab_du_sous_domaine: Uuid,
        adresse: &str,
    ) -> Result<Reponse, EntDbError> {
        let Some((cle, domaine)) = decouper_reponse(adresse) else {
            return Ok(Reponse::ATrier("adresse_mal_formee"));
        };
        let mut t = crate::tenant_tx::TenantTx::begin(self.pool(), etab_du_sous_domaine).await.map_err(db)?;
        // Contrôle 1 : le sous-domaine est bien celui de l'établissement qui reçoit.
        let attendu = uai_de(&mut t, etab_du_sous_domaine).await?.as_deref().and_then(sous_domaine);
        if attendu.as_deref() != Some(domaine.as_str()) {
            t.commit().await.map_err(db)?;
            return Ok(Reponse::ATrier("sous_domaine_autre_etablissement"));
        }
        // Contrôle 2 : la clé appartient à cet établissement (la lecture est cloisonnée par la RLS).
        let ligne: Option<CleLigne> = sqlx::query_as(
            "SELECT id, sortie_id, objet, prestataire_id, adresse_id, expire_le, utilisee_le, revoquee_le
               FROM ent_sortie_reponse_cle WHERE etablissement_id = $1 AND cle_hash = $2 FOR UPDATE",
        )
        .bind(etab_du_sous_domaine)
        .bind(empreinte(&cle))
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let Some(CleLigne { id, sortie_id, objet, prestataire_id: presta, adresse_id: adr, expire_le: expire, utilisee_le: utilisee, revoquee_le: revoquee }) = ligne else {
            t.commit().await.map_err(db)?;
            return Ok(Reponse::ATrier("cle_inconnue"));
        };
        let motif = if revoquee.is_some() {
            Some("cle_revoquee")
        } else if utilisee.is_some() {
            Some("cle_deja_utilisee")
        } else if expire <= Utc::now() {
            Some("cle_expiree")
        } else {
            None
        };
        if let Some(m) = motif {
            t.commit().await.map_err(db)?;
            return Ok(Reponse::ATrier(m));
        }
        sqlx::query("UPDATE ent_sortie_reponse_cle SET utilisee_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab_du_sous_domaine)
            .bind(id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        t.commit().await.map_err(db)?;
        let cible = match (objet.as_str(), presta, adr) {
            ("prestataire", Some(p), Some(ad)) => CibleReponse::Prestataire { prestataire_id: p, adresse_id: ad },
            _ => CibleReponse::Restauration,
        };
        Ok(Reponse::Rattachee { sortie_id, cible })
    }

    /// Révoque toutes les adresses de réponse encore **vivantes** (ni utilisées, ni expirées) d'une sortie (clôture, annulation).
    pub async fn voyage_courrier_reponses_revoquer(&self, a: &Acteur<'_>, sortie: Uuid) -> Result<u64, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, sortie, false).await?;
        if !(acces.gere(Action::VoyagesPrestataires) || acces.gere(Action::VoyagesModifier)) {
            return Err(refus(RefusMetier::Interdit, "Droit insuffisant."));
        }
        let n = sqlx::query(
            "UPDATE ent_sortie_reponse_cle SET revoquee_le = now()
              WHERE etablissement_id = $1 AND sortie_id = $2 AND revoquee_le IS NULL
                AND utilisee_le IS NULL AND expire_le > now()",
        )
        .bind(a.etablissement_id)
        .bind(sortie)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        t.commit().await.map_err(db)?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sous_domaine_et_adresses_derives_de_l_uai() {
        assert_eq!(sous_domaine("0761713Z").as_deref(), Some("0761713z.scolia.fr"));
        assert_eq!(adresse_expedition("0761713z").as_deref(), Some("ne-pas-repondre@0761713z.scolia.fr"));
        assert_eq!(sous_domaine("pas-un-uai"), None);
        assert_eq!(sous_domaine(""), None);
        // Un UAI qui tente d'injecter un autre domaine n'est jamais valide.
        assert_eq!(sous_domaine("0761713Z.evil.fr"), None);
        assert_eq!(sous_domaine("0761713Z@evil.fr"), None);
    }

    #[test]
    fn decoupage_strict_de_l_adresse_de_reponse() {
        let cle = "a".repeat(64);
        let adr = adresse_reponse("0761713Z", &cle).unwrap();
        assert_eq!(adr, format!("r-{cle}@0761713z.scolia.fr"));
        assert_eq!(decouper_reponse(&adr), Some((cle.clone(), "0761713z.scolia.fr".into())));
        assert_eq!(decouper_reponse(&adr.to_uppercase()), Some((cle.clone(), "0761713z.scolia.fr".into())));
        for mauvais in [
            format!("{cle}@0761713z.scolia.fr"),
            format!("r-{}@0761713z.scolia.fr", "g".repeat(64)),
            format!("r-{}@0761713z.scolia.fr", "a".repeat(63)),
            format!("r-{cle}@evil.fr"),
            format!("r-{cle}@0761713z.scolia.fr.evil.fr"),
            format!("r-{cle}@sous.0761713z.scolia.fr"),
            format!("r-{cle}@zzz.scolia.fr"),
            format!("r-{cle}@0761713z.scolia.fr, x@y.fr"),
            String::new(),
        ] {
            assert_eq!(decouper_reponse(&mauvais), None, "{mauvais}");
        }
    }

    #[test]
    fn la_cle_n_est_jamais_conservee_en_clair() {
        let c = nouvelle_cle();
        assert_eq!(c.len(), 64);
        assert_ne!(empreinte(&c), c);
        assert_ne!(nouvelle_cle(), c);
    }
}
