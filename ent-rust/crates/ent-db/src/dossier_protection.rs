//! Lot A0 bis du socle 4 (`docs/socles/04_ADMINISTRATIF.md` R-A38 à R-A40, ADM-82) :
//!
//! * **A0-b** contrat `DeclarationResponsables` v1 et ses champs réservés ([`ReserveDeclaree`]) ;
//!   une adresse à protéger n'entre **jamais** dans `ent_personne.adresse` : elle est parquée
//!   scellée (`ent_adresse_protegee`) pour la direction, avec une alerte, jusqu'au lot A3.
//! * **A0-c** « ne pas solliciter » : [`personne_ne_pas_solliciter_tx`] est le point unique lu
//!   par les invitations, demandes d'accord, relances et envois aux familles.
//! * **A0-d** confirmation d'un lien au guichet ([`PieceIdentite`], type de pièce et auteur
//!   tracés, jamais la copie) ou, pour la personne connectée, en un geste.

use super::*;
use crate::champs_chiffres::ChampsSensibles;

/// Version du contrat `DeclarationResponsables` (champs réservés inclus).
pub const CONTRAT_DECLARATION_VERSION: i16 = 1;

const TABLE_ADRESSE: &str = "ent_adresse_protegee";
const TABLE_RESERVE: &str = "ent_personne_reserve";

fn conflit_p(msg: &'static str) -> EntDbError {
    EntDbError::ConflitDossier(msg)
}

/// Champs réservés d'un responsable déclaré (contrat v1). Tous facultatifs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReserveDeclaree {
    /// L'adresse déclarée doit être protégée : elle n'est pas écrite dans la fiche.
    pub adresse_a_proteger: bool,
    /// Aucune invitation, demande d'accord ni relance vers cette personne.
    pub ne_pas_solliciter: bool,
    /// Référence de la pièce de protection (ordonnance…), scellée en base.
    pub ne_pas_solliciter_ref: Option<String>,
    pub langue: Option<String>,
    pub besoin_interprete: bool,
    /// Organisme titulaire (ex. service de l'aide sociale à l'enfance).
    pub organisme: Option<String>,
    /// Code du vocabulaire de nature de lien (socle 4, lot A1).
    pub nature_code: Option<String>,
}

fn motif_code_ok(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl ReserveDeclaree {
    /// Aucun champ réservé renseigné.
    pub fn est_vide(&self) -> bool {
        *self == ReserveDeclaree::default()
    }

    /// Normalise et valide (la base garde les mêmes bornes).
    pub fn normaliser(&self) -> Result<ReserveDeclaree, EntDbError> {
        let langue = opt(&self.langue).map(|l| l.to_lowercase());
        if let Some(l) = &langue {
            let (base, region) = l.split_once('-').unwrap_or((l.as_str(), ""));
            let base_ok = (2..=3).contains(&base.len()) && base.chars().all(|c| c.is_ascii_lowercase());
            let region_ok = region.is_empty()
                || ((2..=8).contains(&region.len()) && region.chars().all(|c| c.is_ascii_alphanumeric()));
            if !base_ok || !region_ok {
                return Err(invalide("langue : code de langue invalide (ex. fr, ar, prs)"));
            }
        }
        let organisme = opt(&self.organisme);
        if organisme.as_ref().is_some_and(|o| o.chars().count() > 200) {
            return Err(invalide("organisme trop long (200 caractères au plus)"));
        }
        let nature_code = opt(&self.nature_code);
        if nature_code.as_ref().is_some_and(|n| !motif_code_ok(n, 40)) {
            return Err(invalide("nature_code invalide"));
        }
        let reference = opt(&self.ne_pas_solliciter_ref);
        if reference.as_ref().is_some_and(|r| r.chars().count() > 200) {
            return Err(invalide("référence de protection trop longue (200 caractères au plus)"));
        }
        if reference.is_some() && !self.ne_pas_solliciter {
            return Err(invalide(
                "une référence de protection n'a de sens qu'avec « ne pas solliciter »",
            ));
        }
        Ok(ReserveDeclaree {
            adresse_a_proteger: self.adresse_a_proteger,
            ne_pas_solliciter: self.ne_pas_solliciter,
            ne_pas_solliciter_ref: reference,
            langue,
            besoin_interprete: self.besoin_interprete,
            organisme,
            nature_code,
        })
    }
}

fn coffre() -> ChampsSensibles {
    ChampsSensibles::depuis_env()
}

/// Point unique de la règle A0-c : cette personne ne doit recevoir **aucune** sollicitation
/// (invitation, demande d'accord, relance, message automatique).
pub async fn personne_ne_pas_solliciter_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
) -> Result<bool, EntDbError> {
    let (v,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_personne_reserve
                         WHERE etablissement_id = $1 AND personne_id = $2 AND ne_pas_solliciter)",
    )
    .bind(etab)
    .bind(personne_id)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(v)
}

/// Adresse mise de côté : scellée, jamais en clair. Renvoie l'identifiant de l'alerte.
pub async fn parquer_adresse_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    adresse: &str,
    origine: &str,
    source_ref: Option<Uuid>,
    actor: Uuid,
) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    let scellee = coffre()
        .sceller(TABLE_ADRESSE, "adresse", id, adresse)
        .map_err(EntDbError::Domain)?;
    if scellee.is_empty() {
        return Err(invalide("adresse à protéger vide"));
    }
    sqlx::query(
        "INSERT INTO ent_adresse_protegee
            (id, etablissement_id, personne_id, adresse, origine, source_ref, cree_par)
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(id)
    .bind(etab)
    .bind(personne_id)
    .bind(scellee)
    .bind(origine)
    .bind(source_ref)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    // Le journal ne contient jamais l'adresse.
    journal_tx(
        conn,
        etab,
        None,
        "adresse_protegee_parquee",
        "Adresse à protéger mise de côté (direction)",
        json!({ "alerte_id": id, "personne_id": personne_id, "origine": origine }),
        false,
        actor,
    )
    .await?;
    Ok(id)
}

/// Enregistre les champs réservés d'une personne (fusionnés avec l'existant ; une déclaration
/// ne lève jamais un « ne pas solliciter » déjà posé).
pub async fn appliquer_reserve_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    r: &ReserveDeclaree,
    actor: Uuid,
) -> Result<(), EntDbError> {
    let existant: Option<(Uuid, bool, Option<String>)> = sqlx::query_as(
        "SELECT id, ne_pas_solliciter, ne_pas_solliciter_ref FROM ent_personne_reserve
          WHERE etablissement_id = $1 AND personne_id = $2 FOR UPDATE",
    )
    .bind(etab)
    .bind(personne_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let (id, deja_nps, ref_existante) = existant.unwrap_or((Uuid::new_v4(), false, None));
    // Référence : celle de la déclaration si elle est fournie, sinon l'existante (déjà scellée).
    let reference = match &r.ne_pas_solliciter_ref {
        Some(v) => Some(
            coffre()
                .sceller(TABLE_RESERVE, "ne_pas_solliciter_ref", id, v)
                .map_err(EntDbError::Domain)?,
        ),
        None => ref_existante,
    };
    let nouvelle_protection = r.ne_pas_solliciter && !deja_nps;
    sqlx::query(
        "INSERT INTO ent_personne_reserve
            (id, etablissement_id, personne_id, ne_pas_solliciter, ne_pas_solliciter_ref,
             ne_pas_solliciter_le, ne_pas_solliciter_par, langue, besoin_interprete, organisme,
             nature_code, contrat_version)
         VALUES ($1,$2,$3,$4,$5, CASE WHEN $4 THEN now() END, CASE WHEN $4 THEN $6 END,
                 $7,$8,$9,$10,$11)
         ON CONFLICT (etablissement_id, personne_id) DO UPDATE SET
            ne_pas_solliciter = ent_personne_reserve.ne_pas_solliciter OR EXCLUDED.ne_pas_solliciter,
            ne_pas_solliciter_ref = COALESCE(EXCLUDED.ne_pas_solliciter_ref,
                                             ent_personne_reserve.ne_pas_solliciter_ref),
            ne_pas_solliciter_le = CASE
                WHEN ent_personne_reserve.ne_pas_solliciter THEN ent_personne_reserve.ne_pas_solliciter_le
                WHEN EXCLUDED.ne_pas_solliciter THEN now()
                ELSE ent_personne_reserve.ne_pas_solliciter_le END,
            ne_pas_solliciter_par = CASE
                WHEN ent_personne_reserve.ne_pas_solliciter THEN ent_personne_reserve.ne_pas_solliciter_par
                WHEN EXCLUDED.ne_pas_solliciter THEN EXCLUDED.ne_pas_solliciter_par
                ELSE ent_personne_reserve.ne_pas_solliciter_par END,
            langue = COALESCE(EXCLUDED.langue, ent_personne_reserve.langue),
            besoin_interprete = ent_personne_reserve.besoin_interprete OR EXCLUDED.besoin_interprete,
            organisme = COALESCE(EXCLUDED.organisme, ent_personne_reserve.organisme),
            nature_code = COALESCE(EXCLUDED.nature_code, ent_personne_reserve.nature_code),
            contrat_version = EXCLUDED.contrat_version,
            updated_at = now()",
    )
    .bind(id)
    .bind(etab)
    .bind(personne_id)
    .bind(r.ne_pas_solliciter)
    .bind(reference)
    .bind(actor)
    .bind(&r.langue)
    .bind(r.besoin_interprete)
    .bind(&r.organisme)
    .bind(&r.nature_code)
    .bind(CONTRAT_DECLARATION_VERSION)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if nouvelle_protection {
        journal_tx(
            conn,
            etab,
            None,
            "ne_pas_solliciter_declare",
            "« Ne pas solliciter » déclaré (à confirmer par la direction)",
            json!({ "personne_id": personne_id }),
            false,
            actor,
        )
        .await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// A0-d : confirmation d'un lien
// ---------------------------------------------------------------------------

/// Pièce d'identité vue au guichet. Seul le **type** est conservé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceIdentite {
    CarteIdentite,
    Passeport,
    TitreSejour,
    PermisConduire,
    LivretFamille,
    Autre,
}

impl PieceIdentite {
    pub fn code(self) -> &'static str {
        match self {
            Self::CarteIdentite => "carte_identite",
            Self::Passeport => "passeport",
            Self::TitreSejour => "titre_sejour",
            Self::PermisConduire => "permis_conduire",
            Self::LivretFamille => "livret_famille",
            Self::Autre => "autre",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "carte_identite" => Self::CarteIdentite,
            "passeport" => Self::Passeport,
            "titre_sejour" => Self::TitreSejour,
            "permis_conduire" => Self::PermisConduire,
            "livret_famille" => Self::LivretFamille,
            "autre" => Self::Autre,
            _ => return None,
        })
    }
}

/// Voie par laquelle un lien en attente est confirmé (tracée sur la demande).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoieConfirmation {
    /// Jeton reçu par e-mail.
    Invitation,
    /// La personne, connectée, confirme en un geste.
    PersonneConnectee,
    /// Le secrétariat, après vérification d'une pièce d'identité.
    Guichet(PieceIdentite),
}

impl VoieConfirmation {
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::Invitation => "invitation",
            Self::PersonneConnectee => "personne_connectee",
            Self::Guichet(_) => "guichet",
        }
    }
    pub(super) fn piece(self) -> Option<&'static str> {
        match self {
            Self::Guichet(p) => Some(p.code()),
            _ => None,
        }
    }
    pub(super) fn source_lien(self) -> &'static str {
        match self {
            Self::Guichet(_) => "confirmation_guichet",
            _ => "confirmation_personne",
        }
    }
}

// ---------------------------------------------------------------------------
// Vues et commandes de la direction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdresseParquee {
    pub id: Uuid,
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub origine: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectionADeclarer {
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub declaree_le: Option<DateTime<Utc>>,
}

/// Alertes d'information de la direction : jamais une adresse, jamais une référence.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlertesProtection {
    pub adresses_a_traiter: Vec<AdresseParquee>,
    pub ne_pas_solliciter_a_confirmer: Vec<ProtectionADeclarer>,
}

impl PgEnt {
    /// Alertes « adresse à protéger parquée » et « ne pas solliciter à confirmer ».
    /// Réservé à la direction (`liens.suspendre`).
    pub async fn alertes_protection_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<AlertesProtection, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::LiensSuspendre)
            .await?;
        let adresses: Vec<(Uuid, Uuid, String, Option<String>, String, DateTime<Utc>)> =
            sqlx::query_as(
                "SELECT a.id, a.personne_id, p.nom, p.prenom, a.origine, a.created_at
                   FROM ent_adresse_protegee a
                   JOIN ent_personne p ON p.id = a.personne_id AND p.etablissement_id = a.etablissement_id
                  WHERE a.etablissement_id = $1 AND a.statut = 'a_traiter'
                  ORDER BY a.created_at",
            )
            .bind(etab)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        let nps: Vec<(Uuid, String, Option<String>, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT p.id, p.nom, p.prenom, r.ne_pas_solliciter_le
               FROM ent_personne_reserve r
               JOIN ent_personne p ON p.id = r.personne_id AND p.etablissement_id = r.etablissement_id
              WHERE r.etablissement_id = $1 AND r.ne_pas_solliciter
                AND r.ne_pas_solliciter_confirme_le IS NULL
              ORDER BY r.ne_pas_solliciter_le",
        )
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(AlertesProtection {
            adresses_a_traiter: adresses
                .into_iter()
                .map(|(id, personne_id, nom, prenom, origine, created_at)| AdresseParquee {
                    id,
                    personne_id,
                    nom,
                    prenom,
                    origine,
                    created_at,
                })
                .collect(),
            ne_pas_solliciter_a_confirmer: nps
                .into_iter()
                .map(|(personne_id, nom, prenom, declaree_le)| ProtectionADeclarer {
                    personne_id,
                    nom,
                    prenom,
                    declaree_le,
                })
                .collect(),
        })
    }

    /// La direction pose, confirme ou lève « ne pas solliciter » pour une personne.
    pub async fn definir_ne_pas_solliciter_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        actif: bool,
        reference: Option<String>,
    ) -> Result<(), EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::LiensSuspendre)
            .await?;
        let reserve = ReserveDeclaree {
            ne_pas_solliciter: actif,
            ne_pas_solliciter_ref: reference,
            ..Default::default()
        };
        let reserve = if actif {
            reserve.normaliser()?
        } else {
            ReserveDeclaree::default()
        };
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p = personne_tx(&mut tx, etab, personne_id, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if p.fusionnee_dans.is_some() {
            return Err(conflit_p("personne fusionnée dans une autre fiche"));
        }
        if actif {
            appliquer_reserve_tx(&mut tx, etab, personne_id, &reserve, actor).await?;
            sqlx::query(
                "UPDATE ent_personne_reserve
                    SET ne_pas_solliciter_confirme_le = now(), ne_pas_solliciter_confirme_par = $3,
                        updated_at = now()
                  WHERE etablissement_id = $1 AND personne_id = $2",
            )
            .bind(etab)
            .bind(personne_id)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        } else {
            sqlx::query(
                "UPDATE ent_personne_reserve
                    SET ne_pas_solliciter = false, ne_pas_solliciter_confirme_le = NULL,
                        ne_pas_solliciter_confirme_par = NULL, updated_at = now()
                  WHERE etablissement_id = $1 AND personne_id = $2",
            )
            .bind(etab)
            .bind(personne_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        journal_tx(
            &mut tx,
            etab,
            None,
            if actif { "ne_pas_solliciter_pose" } else { "ne_pas_solliciter_leve" },
            if actif {
                "« Ne pas solliciter » confirmé par la direction"
            } else {
                "« Ne pas solliciter » levé par la direction"
            },
            json!({ "personne_id": personne_id }),
            false,
            actor,
        )
        .await?;
        commit(tx).await
    }

    /// La direction ouvre une adresse parquée (consultation journalisée) et la marque traitée.
    pub async fn ouvrir_adresse_protegee_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        alerte_id: Uuid,
    ) -> Result<String, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::LiensSuspendre)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let row: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT personne_id, adresse FROM ent_adresse_protegee
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(alerte_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (personne_id, scellee) = row.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let adresse = coffre()
            .ouvrir(TABLE_ADRESSE, "adresse", alerte_id, &scellee)
            .map_err(EntDbError::Domain)?;
        sqlx::query(
            "UPDATE ent_adresse_protegee SET statut = 'traitee', traitee_le = COALESCE(traitee_le, now()),
                    traitee_par = COALESCE(traitee_par, $3)
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(alerte_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            None,
            "adresse_protegee_consultee",
            "Adresse protégée consultée par la direction",
            json!({ "alerte_id": alerte_id, "personne_id": personne_id }),
            false,
            actor,
        )
        .await?;
        commit(tx).await?;
        Ok(adresse)
    }

    /// ADM-82 : le secrétariat confirme un lien en attente **au guichet**, après avoir vu une
    /// pièce d'identité de la personne. Le type de la pièce et l'auteur sont tracés ; la copie
    /// et le numéro de la pièce ne sont jamais enregistrés. Renvoie le lien réel créé (s'il
    /// n'existait pas déjà).
    pub async fn confirmer_lien_guichet_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        attente_id: Uuid,
        piece: PieceIdentite,
    ) -> Result<Option<Uuid>, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let row: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT a.personne_id, p.fusionnee_dans FROM ent_lien_attente a
               JOIN ent_personne p ON p.id = a.personne_id AND p.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND a.id = $2 AND a.statut = 'en_attente'",
        )
        .bind(etab)
        .bind(attente_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (personne_id, fusionnee) = row.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if fusionnee.is_some() {
            return Err(conflit_p("personne fusionnée dans une autre fiche"));
        }
        let filtre = FiltreAttente {
            attente_id: Some(attente_id),
            ..Default::default()
        };
        let (liens, decidees) = confirmer_liens_attente_voie_tx(
            &mut tx,
            etab,
            personne_id,
            &filtre,
            actor,
            VoieConfirmation::Guichet(piece),
        )
        .await?;
        if decidees == 0 {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        commit(tx).await?;
        Ok(liens.first().copied())
    }
}

// ---------------------------------------------------------------------------
// A0-a : fuseau de l'établissement
// ---------------------------------------------------------------------------

/// Fuseau par défaut d'un établissement sans paramètre.
pub const FUSEAU_DEFAUT: &str = "Europe/Paris";

impl PgEnt {
    /// Fuseau horaire de l'établissement (nom IANA), Europe/Paris par défaut.
    pub async fn fuseau_etablissement(&self, etab: Uuid) -> Result<String, EntDbError> {
        let r: Option<(String,)> = sqlx::query_as(
            "SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(r.map_or_else(|| FUSEAU_DEFAUT.to_string(), |(f,)| f))
    }

    /// Définit le fuseau de l'établissement (structure de l'établissement, `referentiel.gerer`).
    /// Un nom que PostgreSQL ne connaît pas est refusé.
    pub async fn definir_fuseau_etablissement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        fuseau: &str,
    ) -> Result<(), EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::ReferentielGerer)
            .await?;
        let fuseau = fuseau.trim();
        let (connu,): (bool,) =
            sqlx::query_as("SELECT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = $1)")
                .bind(fuseau)
                .fetch_one(self.pool())
                .await
                .map_err(db)?;
        if !connu {
            return Err(invalide("fuseau horaire inconnu (nom IANA attendu, ex. Indian/Reunion)"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau, updated_by)
             VALUES ($1,$2,$3)
             ON CONFLICT (etablissement_id) DO UPDATE
                SET fuseau = EXCLUDED.fuseau, updated_by = EXCLUDED.updated_by, updated_at = now()",
        )
        .bind(etab)
        .bind(fuseau)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            None,
            "fuseau_etablissement",
            "Fuseau horaire de l'établissement modifié",
            json!({ "fuseau": fuseau }),
            false,
            actor,
        )
        .await?;
        commit(tx).await
    }
}
