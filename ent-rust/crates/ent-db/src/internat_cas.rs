//! La nuit de l'internat (socle 41, lot I2), suite : cas et procédure (§9.3), transmission du matin,
//! effectif pour la restauration, rappels d'appel, PDF de l'appel, journal des lectures sensibles.
//!
//! * **R-IT18** un cas ne se ferme que par une issue ; chaque étape est un fait horodaté et signé ;
//!   le lendemain il est **transmis** à la vie scolaire (socle 12) ;
//! * **R-IT26** le surveillant ne voit que les numéros autorisés (titulaires joignables du socle 4) ;
//!   jamais une adresse, jamais un numéro de personne dont le lien est suspendu ou hors dates ;
//! * **R-IT34** avis gradués ; **R-IT40** les rappels suivent la nuit réelle et l'attendu calculé ;
//! * **R-IT6** aucune donnée de santé dans les cas, l'appel, le PDF ni les événements.
//!
//! Adaptateurs en attendant d'autres socles : la **restauration** (40) reçoit des événements neutres
//! (`internat.effectif_repas`) ; le **CPE d'astreinte** (6) n'est pas encore branché
//! (`astreinte_branchee = false`) ; l'**absence de nuit** vers la vie scolaire (12) passe par l'événement
//! `internat.absence_nuit` que le 12 rattache au dossier d'assiduité.

use crate::finances::publier;
use crate::internat_hebergement::parametres_tx;
use crate::internat_nuit::{
    attendu_fige_tx, conflit, db, journaliser_lecture_tx, marques_courantes_tx, nuit_a_tx, refus,
};
use crate::outbox::Causalite;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Timelike, Utc};
use domain_eleve::DomainError;
use ent_auth::Action;
use ent_events::{types, AggregateKind};
use ent_types::internat_nuit::*;
use serde_json::json;
use sqlx::PgConnection;
use std::collections::HashSet;
use uuid::Uuid;

const LIEUX: [&str; 6] = [
    "chambre",
    "sanitaires",
    "etude",
    "infirmerie",
    "autre_zone",
    "exterieur",
];
const ISSUES: [&str; 4] = ["retrouve", "rentre_famille", "autorite_saisie", "annule_erreur"];
const APPELLE: [&str; 5] = ["astreinte", "titulaires", "direction", "secours", "aucun"];
/// Délai entre l'avis au surveillant et l'avis au CPE d'astreinte (NI-22).
pub const DELAI_RAPPEL_CPE_MINUTES: i64 = 15;

/// Procédure par défaut (§9.3, à valider par la direction et le CPE, Q-IT2).
pub fn procedure_par_defaut() -> Vec<ProcedureEtapeSaisieDto> {
    let e = |code: &str, libelle: &str, delai: i16, appelle: &str| ProcedureEtapeSaisieDto {
        code: code.into(),
        libelle: libelle.into(),
        delai_minutes: delai,
        appelle: appelle.into(),
    };
    vec![
        e(
            "verifications",
            "Vérifications sur place : chambre, sanitaires, étude, infirmerie, autres zones",
            0,
            "aucun",
        ),
        e(
            "cpe_astreinte",
            "Appeler le CPE d'astreinte ; message au fil d'équipe",
            10,
            "astreinte",
        ),
        e(
            "titulaires",
            "Appeler les titulaires (personnes autorisées seulement)",
            15,
            "titulaires",
        ),
        e(
            "direction",
            "Prévenir la direction ; décider d'appeler le 17",
            30,
            "direction",
        ),
        e(
            "secours",
            "Si la direction le décide : appeler le 17 et noter le numéro d'intervention",
            30,
            "secours",
        ),
    ]
}

async fn procedure_active_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<Uuid, EntDbError> {
    let existant: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_internat_procedure_version \
          WHERE etablissement_id = $1 AND nature = 'non_retrouve' ORDER BY version DESC LIMIT 1",
    )
    .bind(etab)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if let Some(id) = existant {
        return Ok(id);
    }
    inserer_procedure_tx(conn, etab, 1, &procedure_par_defaut(), None, maintenant).await
}

async fn inserer_procedure_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    version: i32,
    etapes: &[ProcedureEtapeSaisieDto],
    par: Option<Uuid>,
    maintenant: DateTime<Utc>,
) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_internat_procedure_version (id, etablissement_id, nature, version, publiee_le, publiee_par_user_id) \
         VALUES ($1,$2,'non_retrouve',$3,$4,$5) ON CONFLICT (etablissement_id, nature, version) DO NOTHING",
    )
    .bind(id)
    .bind(etab)
    .bind(version)
    .bind(maintenant)
    .bind(par)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    // Deux sessions peuvent semer la version 1 ensemble : la perdante relit celle qui existe.
    let reel: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_internat_procedure_version WHERE etablissement_id = $1 AND nature = 'non_retrouve' AND version = $2",
    )
    .bind(etab)
    .bind(version)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if reel == id {
        for (i, e) in etapes.iter().enumerate() {
            sqlx::query(
                "INSERT INTO ent_internat_procedure_etape \
                 (id, etablissement_id, procedure_version_id, rang, code, libelle, delai_minutes, appelle) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind((i + 1) as i16)
            .bind(&e.code)
            .bind(e.libelle.trim())
            .bind(e.delai_minutes)
            .bind(&e.appelle)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
    }
    Ok(reel)
}

fn valider_procedure(etapes: &[ProcedureEtapeSaisieDto]) -> Result<(), EntDbError> {
    if etapes.is_empty() || etapes.len() > 12 {
        return Err(refus("une procédure compte de 1 à 12 étapes"));
    }
    let mut dernier = 0;
    let mut codes = HashSet::new();
    for e in etapes {
        let code_ok = (2..=40).contains(&e.code.len())
            && e.code.chars().all(|c| c.is_ascii_lowercase() || c == '_');
        if !code_ok || e.libelle.trim().is_empty() || e.libelle.chars().count() > 200 {
            return Err(refus("étape de procédure invalide (code ou libellé)"));
        }
        if !(0..=720).contains(&e.delai_minutes) || e.delai_minutes < dernier {
            return Err(refus("les délais des étapes doivent être croissants, de 0 à 720 minutes"));
        }
        if !APPELLE.contains(&e.appelle.as_str()) {
            return Err(refus("personne à appeler inconnue"));
        }
        if !codes.insert(e.code.clone()) {
            return Err(refus("deux étapes portent le même code"));
        }
        dernier = e.delai_minutes;
    }
    Ok(())
}

impl PgEnt {
    // ───────────────────────── Procédure ─────────────────────────

    pub async fn lire_procedure_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<ProcedureDto, EntDbError> {
        self.nuit_portee(etab, actor, role, &[Action::InternatLire, Action::InternatAppel, Action::InternatGerer])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let id = procedure_active_tx(&mut tx, etab, maintenant).await?;
        let r = procedure_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// INT-253 : nouvelle version de la procédure (direction et administration). Les cas déjà ouverts
    /// gardent la version qu'ils ont reçue.
    pub async fn publier_procedure_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ProcedureSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<ProcedureDto, EntDbError> {
        if !matches!(
            ent_auth::Role::parse(role),
            Some(ent_auth::Role::Direction | ent_auth::Role::Admin)
        ) {
            return Err(EntDbError::Forbidden);
        }
        valider_procedure(&b.etapes)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        procedure_active_tx(&mut tx, etab, maintenant).await?;
        let version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(max(version), 0) + 1 FROM ent_internat_procedure_version \
              WHERE etablissement_id = $1 AND nature = 'non_retrouve'",
        )
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let id = inserer_procedure_tx(&mut tx, etab, version, &b.etapes, Some(actor), maintenant).await?;
        let r = procedure_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    // ───────────────────────── Cas ─────────────────────────

    /// Ouvre un cas pour l'élève et la nuit (un seul cas ouvert par élève et par nuit). `None` si un cas
    /// est déjà ouvert.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn ouvrir_cas_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        appel: Option<Uuid>,
        eleve: Uuid,
        nuit: NaiveDate,
        nature: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<Option<Uuid>, EntDbError> {
        let version = procedure_active_tx(conn, etab, maintenant).await?;
        let id = Uuid::new_v4();
        let n = sqlx::query(
            "INSERT INTO ent_internat_cas \
             (id, etablissement_id, eleve_id, appel_id, nuit, nature, ouvert_le, ouvert_par_user_id, procedure_version_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve)
        .bind(appel)
        .bind(nuit)
        .bind(nature)
        .bind(maintenant)
        .bind(actor)
        .bind(version)
        .execute(&mut *conn)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Ok(None);
        }
        publier(
            conn,
            etab,
            types::INTERNAT_CAS_OUVERT,
            AggregateKind::Internat,
            id,
            Some(eleve),
            json!({ "nuit": nuit.to_string(), "nature": nature }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        Ok(Some(id))
    }

    /// Un élève pointé présent ou vu alors qu'un cas est ouvert pour la nuit : le cas se ferme
    /// (« rentré à 23 h 10 »). L'heure est celle du geste.
    pub(crate) async fn clore_cas_retour_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        eleve: Uuid,
        nuit: NaiveDate,
        a: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let r: Option<Uuid> = sqlx::query_scalar(
            "UPDATE ent_internat_cas SET issue = 'retrouve', issue_le = $4, issue_par_user_id = $5, \
                    issue_lieu_code = 'chambre', modifie_le = now() \
              WHERE etablissement_id = $1 AND eleve_id = $2 AND nuit = $3 AND issue IS NULL RETURNING id",
        )
        .bind(etab)
        .bind(eleve)
        .bind(nuit)
        .bind(a)
        .bind(actor)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if let Some(cas) = r {
            publier(
                conn,
                etab,
                types::INTERNAT_CAS_CLOS,
                AggregateKind::Internat,
                cas,
                Some(eleve),
                json!({ "nuit": nuit.to_string(), "issue": "retrouve" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        Ok(())
    }

    /// Cas d'une nuit (en cours d'abord). Élèves protégés sans nom ; lecture journalisée.
    pub async fn lister_cas_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        nuit: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<CasDto>, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatLire, Action::InternatAppel])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut cas = cas_nuit_tx(&mut tx, etab, nuit).await?;
        if let Some(set) = &portee {
            cas.retain(|c| set.contains(&c.0));
        }
        journaliser_lecture_tx(&mut tx, etab, actor, "cas", None, None, maintenant).await?;
        tx.commit().await.map_err(db)?;
        self.cas_vers_dto(etab, actor, role, cas).await
    }

    async fn cas_vers_dto(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        cas: Vec<CasLigne>,
    ) -> Result<Vec<CasDto>, EntDbError> {
        let ids: Vec<Uuid> = cas.iter().map(|c| c.0).collect();
        let (proteges, caches) = self
            .masquer_proteges(etab, actor, role, &ids, "cas de nuit")
            .await?;
        Ok(cas
            .into_iter()
            .map(|c| CasDto {
                id: c.1,
                eleve_id: c.0,
                nom: (!caches.contains(&c.0)).then_some(c.2),
                prenom: (!caches.contains(&c.0)).then_some(c.3),
                protege: proteges.contains(&c.0),
                nuit: c.4,
                nature: c.5,
                ouvert_le: c.6,
                issue: c.7,
                issue_le: c.8,
                transmis_le: c.9,
            })
            .collect())
    }

    /// Vue pas à pas d'un cas : procédure, minuteur (échéance de chaque étape), lieux vérifiés,
    /// numéros des titulaires joignables (R-IT26).
    pub async fn cas_vue_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        cas_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<CasVueDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatLire, Action::InternatAppel])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ligne = cas_tx(&mut tx, etab, cas_id).await?;
        if portee.as_ref().is_some_and(|s| !s.contains(&ligne.0)) {
            return Err(EntDbError::Forbidden);
        }
        let version: Uuid = sqlx::query_scalar(
            "SELECT procedure_version_id FROM ent_internat_cas WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(cas_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let proc = procedure_tx(&mut tx, etab, version).await?;
        #[allow(clippy::type_complexity)]
        let etapes: Vec<(Uuid, i16, String, String, i16, String)> = sqlx::query_as(
            "SELECT id, rang, code, libelle, delai_minutes, appelle FROM ent_internat_procedure_etape \
              WHERE etablissement_id = $1 AND procedure_version_id = $2 ORDER BY rang",
        )
        .bind(etab)
        .bind(version)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let faits: Vec<(Option<Uuid>, Option<String>, DateTime<Utc>, Uuid)> = sqlx::query_as(
            "SELECT etape_id, lieu_code, fait_le, par_user_id FROM ent_internat_cas_etape \
              WHERE etablissement_id = $1 AND cas_id = $2 ORDER BY fait_le, id",
        )
        .bind(etab)
        .bind(cas_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let numeros = numeros_titulaires_tx(&mut tx, etab, ligne.0, ligne.4).await?;
        journaliser_lecture_tx(&mut tx, etab, actor, "cas", Some(ligne.0), None, maintenant).await?;
        tx.commit().await.map_err(db)?;
        let cas = self.cas_vers_dto(etab, actor, role, vec![ligne]).await?.remove(0);
        Ok(CasVueDto {
            procedure_version: proc.version,
            etapes: etapes
                .into_iter()
                .map(|(id, rang, code, libelle, delai, appelle)| {
                    let fait = faits.iter().find(|f| f.0 == Some(id));
                    EtapeProcedureDto {
                        id,
                        rang,
                        code,
                        libelle,
                        delai_minutes: delai,
                        appelle,
                        echeance: cas.ouvert_le + Duration::minutes(i64::from(delai)),
                        faite_le: fait.map(|f| f.2),
                        faite_par_user_id: fait.map(|f| f.3),
                    }
                })
                .collect(),
            lieux_verifies: faits.iter().filter_map(|f| f.1.clone()).collect(),
            numeros_titulaires: numeros,
            astreinte_branchee: false,
            cas,
        })
    }

    /// Fait une étape de la procédure ou vérifie un lieu : horodaté, signé, en ajout seul.
    pub async fn cas_etape_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        cas_id: Uuid,
        b: &CasEtapeSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatAppel, Action::InternatAppelValider])
            .await?;
        if b.etape_id.is_some() == b.lieu_code.is_some() {
            return Err(refus("indiquer une étape OU un lieu vérifié"));
        }
        if let Some(l) = &b.lieu_code {
            if !LIEUX.contains(&l.as_str()) {
                return Err(refus("lieu de vérification inconnu"));
            }
        }
        let note = b.note.as_deref().map(str::trim).filter(|n| !n.is_empty());
        if note.is_some_and(|n| n.chars().count() > 300) {
            return Err(refus("note trop longue (300 caractères au plus)"));
        }
        if note.is_some_and(domain_eleve::mots_de_sante_detectes) {
            return Err(refus("pas de donnée de santé dans un cas : renvoyer vers l'infirmerie"));
        }
        let fait_le = b.fait_le.unwrap_or(maintenant);
        if fait_le > maintenant + Duration::minutes(5) {
            return Err(refus("l'heure du fait est dans le futur"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ligne = cas_verrou_tx(&mut tx, etab, cas_id).await?;
        if portee.as_ref().is_some_and(|s| !s.contains(&ligne.0)) {
            return Err(EntDbError::Forbidden);
        }
        if ligne.7.is_some() {
            return Err(conflit("le cas est clos"));
        }
        if let Some(etape) = b.etape_id {
            let dans_version: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_internat_procedure_etape e \
                   JOIN ent_internat_cas c ON c.etablissement_id = e.etablissement_id \
                    AND c.procedure_version_id = e.procedure_version_id \
                  WHERE e.etablissement_id = $1 AND e.id = $2 AND c.id = $3)",
            )
            .bind(etab)
            .bind(etape)
            .bind(cas_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !dans_version {
                return Err(refus("cette étape n'appartient pas à la procédure du cas"));
            }
        }
        let deja: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_internat_cas_etape WHERE etablissement_id = $1 AND cas_id = $2 \
               AND (($3::uuid IS NOT NULL AND etape_id = $3) OR ($4::text IS NOT NULL AND lieu_code = $4)))",
        )
        .bind(etab)
        .bind(cas_id)
        .bind(b.etape_id)
        .bind(&b.lieu_code)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit("cette étape ou ce lieu est déjà enregistré"));
        }
        sqlx::query(
            "INSERT INTO ent_internat_cas_etape (id, etablissement_id, cas_id, etape_id, lieu_code, fait_le, par_user_id, note, cree_le) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(cas_id)
        .bind(b.etape_id)
        .bind(&b.lieu_code)
        .bind(fait_le)
        .bind(actor)
        .bind(note)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_CAS_ETAPE,
            AggregateKind::Internat,
            cas_id,
            Some(ligne.0),
            json!({ "nuit": ligne.4.to_string(), "etape": b.etape_id.map(|e| e.to_string()), "lieu": b.lieu_code }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Issue d'un cas : la seule manière de le fermer (R-IT18).
    pub async fn cas_issue_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        cas_id: Uuid,
        b: &CasIssueSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<CasDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatAppel, Action::InternatAppelValider])
            .await?;
        if !ISSUES.contains(&b.issue.as_str()) {
            return Err(refus("issue inconnue"));
        }
        if b.issue == "annule_erreur" {
            // Seule la validation peut déclarer une erreur d'ouverture.
            self.nuit_portee(etab, actor, role, &[Action::InternatAppelValider]).await?;
        }
        match b.issue.as_str() {
            "retrouve" => match b.lieu_code.as_deref() {
                Some(l) if LIEUX.contains(&l) => {}
                _ => return Err(refus("« retrouvé » exige le lieu où l'élève a été retrouvé")),
            },
            "rentre_famille" if b.confirme_par_personne_id.is_none() => {
                return Err(refus("« rentré en famille » exige la personne qui l'a confirmé"));
            }
            "autorite_saisie"
                if b.numero_intervention.as_deref().map(str::trim).unwrap_or("").is_empty() =>
            {
                return Err(refus("« autorité saisie » exige le numéro d'intervention"));
            }
            _ => {}
        }
        if b.numero_intervention.as_deref().is_some_and(|n| n.chars().count() > 40) {
            return Err(refus("numéro d'intervention trop long (40 caractères au plus)"));
        }
        let note = b.note.as_deref().map(str::trim).filter(|n| !n.is_empty());
        if note.is_some_and(|n| n.chars().count() > 300 || domain_eleve::mots_de_sante_detectes(n)) {
            return Err(refus("note invalide (300 caractères, aucune donnée de santé)"));
        }
        let a = b.a.unwrap_or(maintenant);
        if a > maintenant + Duration::minutes(5) {
            return Err(refus("l'heure de l'issue est dans le futur"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ligne = cas_verrou_tx(&mut tx, etab, cas_id).await?;
        if portee.as_ref().is_some_and(|s| !s.contains(&ligne.0)) {
            return Err(EntDbError::Forbidden);
        }
        if ligne.7.is_some() {
            return Err(conflit("le cas est déjà clos"));
        }
        if let Some(p) = b.confirme_par_personne_id {
            // La personne qui confirme doit être liée à l'élève (socle 4), lien non suspendu.
            let lien: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien l WHERE l.etablissement_id = $1 AND l.eleve_id = $2 \
                   AND l.personne_id = $3 AND l.suspendu_le IS NULL)",
            )
            .bind(etab)
            .bind(ligne.0)
            .bind(p)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !lien {
                return Err(refus("la personne indiquée n'est pas rattachée à l'élève"));
            }
        }
        sqlx::query(
            "UPDATE ent_internat_cas SET issue = $3, issue_le = $4, issue_par_user_id = $5, issue_lieu_code = $6, \
                    issue_confirmee_par_personne_id = $7, numero_intervention = $8, modifie_le = now() \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(cas_id)
        .bind(&b.issue)
        .bind(a)
        .bind(actor)
        .bind(&b.lieu_code)
        .bind(b.confirme_par_personne_id)
        .bind(b.numero_intervention.as_deref().map(str::trim))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if note.is_some() || b.lieu_code.is_some() {
            sqlx::query(
                "INSERT INTO ent_internat_cas_etape (id, etablissement_id, cas_id, lieu_code, fait_le, par_user_id, note, cree_le) \
                 SELECT $1,$2,$3,$4,$5,$6,$7,$8 WHERE $4::text IS NOT NULL",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(cas_id)
            .bind(&b.lieu_code)
            .bind(a)
            .bind(actor)
            .bind(note)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        publier(
            &mut tx,
            etab,
            types::INTERNAT_CAS_CLOS,
            AggregateKind::Internat,
            cas_id,
            Some(ligne.0),
            json!({ "nuit": ligne.4.to_string(), "issue": b.issue }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let cas = cas_tx(&mut tx, etab, cas_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(self.cas_vers_dto(etab, actor, role, vec![cas]).await?.remove(0))
    }

    // ───────────────────────── Transmission du matin, effectif ─────────────────────────

    /// Chiffres de la nuit à partir des appels clos ou validés : attendus, présents, absents.
    async fn chiffres_nuit_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        nuit: NaiveDate,
    ) -> Result<ChiffresNuit, EntDbError> {
        let appels: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_internat_appel WHERE etablissement_id = $1 AND nuit = $2 AND etat IN ('clos', 'valide') \
              ORDER BY ouvert_le, id",
        )
        .bind(etab)
        .bind(nuit)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut attendus: HashSet<Uuid> = HashSet::new();
        let mut presents: HashSet<Uuid> = HashSet::new();
        let mut absents: HashSet<Uuid> = HashSet::new();
        let mut partis: HashSet<Uuid> = HashSet::new();
        for a in &appels {
            let figes = attendu_fige_tx(conn, etab, *a).await?;
            let marques = marques_courantes_tx(conn, etab, *a).await?;
            for l in &figes {
                if l.statut == "attendu" || l.statut == "ajoute" {
                    attendus.insert(l.eleve_id);
                }
                match marques.get(&l.eleve_id).map(|m| m.marque.as_str()) {
                    Some("present" | "vu" | "infirmerie" | "non_attendu_present") => {
                        presents.insert(l.eleve_id);
                        absents.remove(&l.eleve_id);
                    }
                    Some("absent") => {
                        if !presents.contains(&l.eleve_id) {
                            absents.insert(l.eleve_id);
                        }
                    }
                    Some("parti") => {
                        partis.insert(l.eleve_id);
                    }
                    _ => {}
                }
            }
        }
        let nb_cas: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_internat_cas WHERE etablissement_id = $1 AND nuit = $2",
        )
        .bind(etab)
        .bind(nuit)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        Ok(ChiffresNuit {
            nb_appels: appels.len(),
            attendus: attendus.len() as i32,
            presents: presents.len() as i32,
            absents: absents.into_iter().collect(),
            partis: partis.len() as i32,
            nb_cas: nb_cas as i32,
        })
    }

    /// INT-?? : transmission du matin à la vie scolaire (socle 12) et effectif à la restauration (socle 40).
    /// Une fois par nuit (idempotent). Les événements ne portent que des identifiants, des dates, des
    /// codes et des compteurs.
    pub async fn transmettre_matin_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        nuit: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<TransmissionDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatAppelValider])
            .await?;
        if portee.is_some() {
            // La transmission couvre tout l'internat : une portée restreinte ne la déclenche pas (R-IT30).
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let courante = nuit_a_tx(&mut tx, etab, maintenant).await?;
        if nuit > courante {
            return Err(DomainError::InternatNuitFuture.into());
        }
        let c = self.chiffres_nuit_tx(&mut tx, etab, nuit).await?;
        if c.nb_appels == 0 {
            return Err(conflit("aucun appel clos ou validé pour cette nuit : rien à transmettre"));
        }
        let nouvelle = sqlx::query(
            "INSERT INTO ent_internat_transmission \
             (id, etablissement_id, nuit, nature, transmis_le, transmis_par_user_id, nb_attendus, nb_presents, nb_absents, nb_cas) \
             VALUES ($1,$2,$3,'matin_vie_scolaire',$4,$5,$6,$7,$8,$9) ON CONFLICT DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(nuit)
        .bind(maintenant)
        .bind(actor)
        .bind(c.attendus)
        .bind(c.presents)
        .bind(c.absents.len() as i32)
        .bind(c.nb_cas)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected()
            == 1;
        let (transmis_le, nb_att, nb_pre, nb_abs, nb_cas): (DateTime<Utc>, i32, i32, i32, i32) =
            sqlx::query_as(
                "SELECT transmis_le, nb_attendus, nb_presents, nb_absents, nb_cas FROM ent_internat_transmission \
                  WHERE etablissement_id = $1 AND nuit = $2 AND nature = 'matin_vie_scolaire'",
            )
            .bind(etab)
            .bind(nuit)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if nouvelle {
            // Vers la vie scolaire (12) : une absence de nuit par élève absent, sans motif.
            for e in &c.absents {
                publier(
                    &mut tx,
                    etab,
                    types::INTERNAT_ABSENCE_NUIT,
                    AggregateKind::Internat,
                    *e,
                    Some(*e),
                    json!({ "nuit": nuit.to_string(), "couverte": false }),
                    Some(actor),
                    Causalite::default(),
                )
                .await?;
            }
            sqlx::query(
                "UPDATE ent_internat_cas SET transmis_le = $3 WHERE etablissement_id = $1 AND nuit = $2 AND transmis_le IS NULL",
            )
            .bind(etab)
            .bind(nuit)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            publier(
                &mut tx,
                etab,
                types::INTERNAT_TRANSMISSION,
                AggregateKind::Internat,
                Uuid::new_v4(),
                None,
                json!({ "nuit": nuit.to_string(), "nb_attendus": nb_att, "nb_presents": nb_pre, "nb_absents": nb_abs, "nb_cas": nb_cas }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
            // Vers la restauration (40) : l'effectif, sans nom.
            let repas = EffectifRepasDto {
                nuit,
                diner: c.attendus,
                petit_dejeuner: (c.attendus - nb_abs - c.partis).max(0),
            };
            sqlx::query(
                "INSERT INTO ent_internat_transmission \
                 (id, etablissement_id, nuit, nature, transmis_le, transmis_par_user_id, nb_attendus, nb_presents, nb_absents, nb_cas) \
                 VALUES ($1,$2,$3,'effectif_restauration',$4,$5,$6,$7,$8,$9) ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(nuit)
            .bind(maintenant)
            .bind(actor)
            .bind(repas.diner)
            .bind(repas.petit_dejeuner)
            .bind(nb_abs)
            .bind(nb_cas)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            publier(
                &mut tx,
                etab,
                types::INTERNAT_EFFECTIF_REPAS,
                AggregateKind::Internat,
                Uuid::new_v4(),
                None,
                json!({ "nuit": nuit.to_string(), "diner": repas.diner, "petit_dejeuner": repas.petit_dejeuner }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(TransmissionDto {
            nuit,
            transmis_le,
            nb_attendus: nb_att,
            nb_presents: nb_pre,
            nb_absents: nb_abs,
            nb_cas,
            nouvelle,
        })
    }

    /// Effectif de nuit pour la restauration (port `internat::attendu` agrégé, socle 40) : des
    /// compteurs, aucun nom. Le dîner suit l'attendu du soir ; le petit-déjeuner en retranche les
    /// retraits (déjà dans l'attendu) — il ne dépend donc d'aucun appel.
    pub async fn effectif_repas_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        nuit: NaiveDate,
    ) -> Result<EffectifRepasDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatLire])
            .await?;
        if portee.is_some() {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_internat_attendu_calcule($1, $2) WHERE statut IN ('attendu', 'ajoute')",
        )
        .bind(etab)
        .bind(nuit)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok(EffectifRepasDto {
            nuit,
            diner: n as i32,
            petit_dejeuner: n as i32,
        })
    }

    // ───────────────────────── Rappels (NI-22) ─────────────────────────

    /// Rappel d'un appel du soir non validé à l'heure limite (S-4102) : avis au surveillant puis, 15 minutes
    /// plus tard, au CPE d'astreinte ; **une seule fois par niveau**. Suit la nuit et l'attendu réels :
    /// aucun rappel si personne n'est attendu (internat fermé, vacances), jamais de jour codé en dur (R-IT40).
    /// Tâche de fond : pas d'acteur.
    pub async fn emettre_rappels_appels_a(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<RappelsDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut sortie = RappelsDto {
            surveillant: vec![],
            cpe: vec![],
        };
        let nuit = nuit_a_tx(&mut tx, etab, maintenant).await?;
        let actif: Option<bool> = sqlx::query_scalar(
            "SELECT actif FROM ent_internat_type_appel_actif WHERE etablissement_id = $1 AND type = 'soir'",
        )
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if actif == Some(false) {
            return Ok(sortie);
        }
        let limite = parametres_tx(&mut tx, etab).await?.heure_limite_appel;
        // Instant limite : date de la nuit + heure limite, heure murale de l'établissement.
        let instant_limite: DateTime<Utc> = sqlx::query_scalar(
            "SELECT (($2::date + $3::time) AT TIME ZONE ent_internat_fuseau($1))",
        )
        .bind(etab)
        .bind(nuit)
        .bind(limite)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if maintenant < instant_limite {
            return Ok(sortie);
        }
        // Reste-t-il un élève attendu non couvert par un appel du soir validé ?
        let manquants: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_internat_attendu_calcule($1, $2) a \
              WHERE a.statut IN ('attendu', 'ajoute') AND NOT EXISTS ( \
                    SELECT 1 FROM ent_internat_appel_attendu x \
                      JOIN ent_internat_appel p ON p.etablissement_id = x.etablissement_id AND p.id = x.appel_id \
                     WHERE x.etablissement_id = $1 AND p.nuit = $2 AND p.type = 'soir' AND p.etat = 'valide' \
                       AND x.eleve_id = a.eleve_id)",
        )
        .bind(etab)
        .bind(nuit)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if manquants == 0 {
            return Ok(sortie);
        }
        for (niveau, delai) in [("surveillant", 0), ("cpe", DELAI_RAPPEL_CPE_MINUTES)] {
            if maintenant < instant_limite + Duration::minutes(delai) {
                continue;
            }
            let n = sqlx::query(
                "INSERT INTO ent_internat_appel_rappel (id, etablissement_id, nuit, type, niveau, emis_le) \
                 VALUES ($1,$2,$3,'soir',$4,$5) ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(nuit)
            .bind(niveau)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
            if n == 1 {
                let dto = AppelRappelDto {
                    nuit,
                    type_appel: "soir".into(),
                };
                publier(
                    &mut tx,
                    etab,
                    types::INTERNAT_APPEL_EN_RETARD,
                    AggregateKind::Internat,
                    Uuid::new_v4(),
                    None,
                    json!({ "nuit": nuit.to_string(), "type": "soir", "niveau": niveau, "manquants": manquants }),
                    None,
                    Causalite::default(),
                )
                .await?;
                if niveau == "surveillant" {
                    sortie.surveillant.push(dto);
                } else {
                    sortie.cpe.push(dto);
                }
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(sortie)
    }

    /// Tâche planifiée (chaque minute) : rappels d'appel de nuit de chaque établissement, à l'heure
    /// murale de son fuseau. Idempotente : la base garantit une seule émission par niveau.
    pub async fn internat_rappels_tache(&self) -> Result<usize, EntDbError> {
        let mut total = 0;
        for etab in crate::tenant_tx::etablissements_systeme(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::JobPlanifie("internat_rappels_appel"),
        )
        .await
        .map_err(db)?
        {
            match self.emettre_rappels_appels_a(etab, Utc::now()).await {
                Ok(r) => total += r.surveillant.len() + r.cpe.len(),
                Err(e) => tracing::warn!(error = %e, "rappels d'appel de nuit non émis pour un établissement"),
            }
        }
        Ok(total)
    }

    // ───────────────────────── PDF de l'appel ─────────────────────────

    /// INT-139 : PDF de l'appel, produit côté serveur depuis la liste figée, les marques courantes et
    /// l'historique des corrections. Élèves protégés sans nom (lit seulement) ; aucune donnée de santé.
    pub async fn pdf_appel_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        appel_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<u8>, EntDbError> {
        let vue = self.vue_appel_a(etab, actor, role, appel_id, maintenant).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        journaliser_lecture_tx(&mut tx, etab, actor, "appel_pdf", None, Some(appel_id), maintenant).await?;
        let nb_corrections: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_internat_presence_fait WHERE etablissement_id = $1 AND appel_id = $2 \
               AND motif_correction_code IS NOT NULL",
        )
        .bind(etab)
        .bind(appel_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        // Heure murale de l'établissement (date de création du document : aucune règle n'en dépend).
        let local: NaiveDateTime = sqlx::query_scalar("SELECT $1::timestamptz AT TIME ZONE ent_internat_fuseau($2)")
            .bind(maintenant)
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rendre_pdf_appel(&vue, nb_corrections, local))
    }
}

fn libelle_marque(m: &str) -> &'static str {
    match m {
        "present" => "Présent",
        "absent" => "Absent",
        "activite" => "En activité",
        "infirmerie" => "À l'infirmerie",
        "parti" => "Parti",
        "non_attendu_present" => "Non attendu, présent",
        "vu" => "Vu (ronde)",
        "retard" => "Retard annoncé",
        _ => "—",
    }
}

fn libelle_motif(m: &str) -> &'static str {
    match m {
        "absence_annoncee" => "absence annoncée",
        "sortie" => "en sortie",
        "mobilite" => "mobilité",
        "stage" => "stage",
        "sejour_scolaire" => "séjour scolaire",
        "exclusion_internat" => "non hébergé",
        "fermeture" => "internat fermé",
        "sante" => "absent : santé",
        _ => "",
    }
}

fn libelle_statut(s: &str, motif: Option<&str>) -> String {
    match s {
        "attendu" => "Attendu".into(),
        "ajoute" => "Nuit exceptionnelle".into(),
        "retire" => format!("Excusé ({})", motif.map(libelle_motif).unwrap_or("")),
        _ => "Non attendu".into(),
    }
}

/// PDF d'un appel : en-tête, tableau par chambre, écarts, pied. Déterministe (même entrée, mêmes octets).
pub(crate) fn rendre_pdf_appel(
    vue: &AppelVueDto,
    nb_corrections: i64,
    local: NaiveDateTime,
) -> Vec<u8> {
    use ent_pdf::{Align, Colonne, Document, Ligne, Meta, Tableau};
    let meta = Meta {
        titre: format!("Appel de nuit — {}", vue.appel.nuit),
        auteur: "ScolIA".into(),
        creation: Some(ent_pdf::DateHeure {
            an: local.year() as u16,
            mois: local.month() as u8,
            jour: local.day() as u8,
            heure: local.hour() as u8,
            minute: local.minute() as u8,
            seconde: local.second() as u8,
        }),
    };
    let mut d = Document::new(meta).avec_pied("Document confidentiel — internat");
    d.titre(&format!(
        "Appel {} — nuit du {}",
        vue.appel.type_appel, vue.appel.nuit
    ));
    d.paragraphe(&format!(
        "Périmètre : {} — état : {} — {} attendu(s), {} écart(s).",
        vue.appel.perimetre,
        vue.appel.etat,
        vue.appel.nb_attendus,
        vue.appel.nb_ecarts.unwrap_or(0)
    ));
    if vue.appel.a_posteriori {
        d.paragraphe("Appel saisi après la nuit (saisie a posteriori).");
    }
    let mut t = Tableau::new(vec![
        Colonne::new("Chambre", 1.0, Align::Gauche),
        Colonne::new("Lit", 0.5, Align::Centre),
        Colonne::new("Élève", 3.0, Align::Gauche),
        Colonne::new("Attendu", 2.2, Align::Gauche),
        Colonne::new("Marque", 2.0, Align::Gauche),
    ]);
    for l in &vue.lignes {
        let nom = match (&l.nom, &l.prenom) {
            (Some(n), Some(p)) => format!("{n} {p}"),
            _ => "Élève protégé".to_string(),
        };
        let marque = match (&l.marque, l.statut.as_str()) {
            (Some(m), _) => libelle_marque(m).to_string(),
            (None, "retire") => "Excusé".to_string(),
            _ => "Non pointé".to_string(),
        };
        let mut ligne = vec![
            l.chambre_code.clone().unwrap_or_else(|| "—".into()),
            l.lit_numero.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
            nom,
            libelle_statut(&l.statut, l.motif_code.as_deref()),
            marque,
        ];
        if l.conflit {
            ligne[4].push_str(" (conflit)");
        }
        if l.corrige {
            ligne[4].push_str(" (corrigé)");
        }
        let grasse = matches!(l.marque.as_deref(), Some("absent" | "non_attendu_present")) || l.conflit;
        t.lignes.push(if grasse { Ligne::grasse(ligne) } else { Ligne::new(ligne) });
    }
    d.tableau(&t);
    d.paragraphe(&format!(
        "{} élève(s) pointé(s) sur {}. {} correction(s) après clôture. Liste figée à l'ouverture de l'appel ; \
         aucune donnée de santé n'est portée par ce document.",
        vue.nb_pointes,
        vue.lignes.len(),
        nb_corrections
    ));
    d.octets()
}

// ───────────────────────── Lectures de base ─────────────────────────

pub(crate) struct ChiffresNuit {
    pub nb_appels: usize,
    pub attendus: i32,
    pub presents: i32,
    pub absents: Vec<Uuid>,
    pub partis: i32,
    pub nb_cas: i32,
}

/// (eleve, cas, nom, prenom, nuit, nature, ouvert_le, issue, issue_le, transmis_le)
type CasLigne = (
    Uuid,
    Uuid,
    String,
    String,
    NaiveDate,
    String,
    DateTime<Utc>,
    Option<String>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

const SELECT_CAS: &str = "SELECT c.eleve_id, c.id, e.nom, e.prenom, c.nuit, c.nature, c.ouvert_le, c.issue, c.issue_le, c.transmis_le \
     FROM ent_internat_cas c JOIN ent_eleve e ON e.etablissement_id = c.etablissement_id AND e.id = c.eleve_id";

async fn cas_nuit_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    nuit: NaiveDate,
) -> Result<Vec<CasLigne>, EntDbError> {
    sqlx::query_as(&format!(
        "{SELECT_CAS} WHERE c.etablissement_id = $1 AND c.nuit = $2 \
          ORDER BY (c.issue IS NOT NULL), c.ouvert_le, c.id"
    ))
    .bind(etab)
    .bind(nuit)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)
}

async fn cas_tx(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<CasLigne, EntDbError> {
    let r: Option<CasLigne> = sqlx::query_as(&format!(
        "{SELECT_CAS} WHERE c.etablissement_id = $1 AND c.id = $2"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    r.ok_or(DomainError::NotFound.into())
}

/// Cas verrouillé ; renvoie la ligne puis (index 7) l'issue.
async fn cas_verrou_tx(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<CasLigne, EntDbError> {
    let r: Option<CasLigne> = sqlx::query_as(&format!(
        "{SELECT_CAS} WHERE c.etablissement_id = $1 AND c.id = $2 FOR UPDATE OF c"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    r.ok_or(DomainError::NotFound.into())
}

async fn procedure_tx(conn: &mut PgConnection, etab: Uuid, version_id: Uuid) -> Result<ProcedureDto, EntDbError> {
    let version: i32 = sqlx::query_scalar(
        "SELECT version FROM ent_internat_procedure_version WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(version_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let etapes: Vec<(String, String, i16, String)> = sqlx::query_as(
        "SELECT code, libelle, delai_minutes, appelle FROM ent_internat_procedure_etape \
          WHERE etablissement_id = $1 AND procedure_version_id = $2 ORDER BY rang",
    )
    .bind(etab)
    .bind(version_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(ProcedureDto {
        version,
        etapes: etapes
            .into_iter()
            .map(|(code, libelle, delai_minutes, appelle)| ProcedureEtapeSaisieDto {
                code,
                libelle,
                delai_minutes,
                appelle,
            })
            .collect(),
    })
}

/// Numéros des titulaires joignables (socle 4) : liens non suspendus et dans leurs dates à la nuit,
/// autorité parentale, garde non « aucune », coordonnées de type téléphone ouvertes. **Jamais** une
/// adresse : seul le champ `valeur` d'un mobile ou d'un fixe sort (R-IT26).
async fn numeros_titulaires_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    nuit: NaiveDate,
) -> Result<Vec<NumeroDto>, EntDbError> {
    let rows: Vec<(Uuid, String, String, String)> = sqlx::query_as(
        "SELECT DISTINCT ON (p.id, c.valeur) p.id, COALESCE(NULLIF(btrim(l.lien), ''), 'titulaire'), c.type, c.valeur \
           FROM ent_eleve_lien l \
           JOIN ent_personne p ON p.etablissement_id = l.etablissement_id AND p.id = l.personne_id \
           JOIN ent_personne_coordonnee c ON c.etablissement_id = p.etablissement_id AND c.personne_id = p.id \
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.suspendu_le IS NULL AND l.autorite_parentale \
            AND COALESCE(l.garde, '') <> 'aucune' \
            AND (l.date_debut IS NULL OR l.date_debut <= $3) AND (l.date_fin IS NULL OR l.date_fin >= $3) \
            AND p.fusionnee_dans IS NULL \
            AND c.type IN ('mobile', 'fixe') AND c.au IS NULL \
          ORDER BY p.id, c.valeur",
    )
    .bind(etab)
    .bind(eleve)
    .bind(nuit)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|(personne_id, lien, _type, numero)| NumeroDto {
            personne_id,
            libelle: lien,
            numero,
        })
        .collect())
}

impl PgEnt {
    /// Journal des lectures sensibles (R-IT38), lisible par l'audit (administration, direction).
    pub async fn lectures_sensibles_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        limite: i64,
    ) -> Result<Vec<LectureSensibleDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::AuditLire).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows: Vec<(Uuid, String, Option<Uuid>, Option<Uuid>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT user_id, objet, eleve_id, appel_id, a FROM ent_internat_lecture_sensible \
              WHERE etablissement_id = $1 ORDER BY a DESC, id LIMIT $2",
        )
        .bind(etab)
        .bind(limite.clamp(1, 500))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(user_id, objet, eleve_id, appel_id, a)| LectureSensibleDto {
                user_id,
                objet,
                eleve_id,
                appel_id,
                a,
            })
            .collect())
    }
}
