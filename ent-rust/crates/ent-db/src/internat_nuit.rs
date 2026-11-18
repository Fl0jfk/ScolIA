//! La nuit de l'internat (socle 41, lot I2) : attendu calculé, appels (types, périmètres, listes),
//! pointage en ajout seul, clôture, validation, correction tracée.
//!
//! Règles appliquées ici :
//! * **R-IT5 / R-IT40** la nuit est la date du soir dans le fuseau de l'établissement ; le calcul de
//!   l'attendu part des séjours, rythmes et du calendrier réel (fonction SQL `ent_internat_attendu_calcule`) ;
//! * **R-IT13** faits en ajout seul ; la valeur courante est le fait le plus récent ; deux surveillants
//!   qui se contredisent produisent un **conflit signalé** ;
//! * **R-IT14** on ne pointe que l'attendu du périmètre de l'appel (et « non attendu présent ») ;
//! * **R-IT16** clôture impossible avec un attendu non pointé ou une activité en attente ; chaque absent
//!   non couvert ouvre un **cas** ;
//! * **R-IT17** correction après validation : fait motivé, avis de correction ;
//! * **R-IT30** périmètre vérifié **au serveur** (portée des droits `internat.*`) : un acteur qui ne
//!   couvre pas tous les élèves d'un appel ne l'ouvre, ne le clôt ni ne le valide pas ;
//! * **R-IT31** un seul appel par type, nuit et périmètre ;
//! * **R-IT34** avis gradués : U1 sans écart, U2 écart sans cas, U3 cas ouvert ;
//! * **R-IT38** chaque lecture sensible est journalisée ; un élève protégé (socle 2, E7) n'est jamais
//!   nommé à un tiers (ligne sans nom, retrouvée par son lit) et sa consultation est tracée ;
//! * **R-IT6** aucune donnée de santé : la marque « à l'infirmerie » renvoie vers le socle 34.
//!
//! Chaque fonction `*_a` reçoit l'instant (`maintenant`) : aucun test ne dépend de l'heure réelle.

use crate::droits_ctx::accorde;
use crate::finances::publier;
use crate::internat_hebergement::parametres_tx;
use crate::outbox::Causalite;
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::Action;
use ent_events::{types, AggregateKind};
use ent_types::internat_nuit::*;
use serde_json::json;
use sqlx::PgConnection;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub(crate) fn db(e: sqlx::Error) -> EntDbError {
    if let sqlx::Error::Database(d) = &e {
        match d.code().as_deref() {
            Some("22007") => return EntDbError::Domain(DomainError::InternatNuitFuture),
            Some("23514") if d.message().contains("correction motivée") => {
                return EntDbError::Domain(DomainError::InternatMotifRequis)
            }
            Some("23514") if d.message().contains("périmètre de l'appel") => {
                return refus("élève hors du périmètre de l'appel")
            }
            Some("42501") => return EntDbError::Refus(RefusMetier::Conflit, "opération refusée par la règle d'ajout seul"),
            _ => {}
        }
    }
    EntDbError::Domain(crate::internat_repo::map_sqlx(e))
}

pub(crate) fn refus(msg: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Invalide, msg)
}

pub(crate) fn conflit(msg: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Conflit, msg)
}

fn est_unique(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23505"))
}

/// Types d'appel (S-4101) et périmètres connus.
pub const TYPES_APPEL: [&str; 7] = [
    "arrivee",
    "etude",
    "soir",
    "extinction",
    "ronde",
    "matin",
    "rassemblement",
];
const PERIMETRES: [&str; 4] = ["internat", "batiment", "zone", "liste"];
/// Marques d'un pointage. `excuse` n'en fait pas partie : une absence couverte vient de l'attendu.
const MARQUES: [&str; 8] = [
    "present",
    "absent",
    "activite",
    "infirmerie",
    "parti",
    "non_attendu_present",
    "vu",
    "retard",
];
const MOTIFS_CORRECTION: [&str; 5] = [
    "oubli",
    "erreur_saisie",
    "information_tardive",
    "rattrapage",
    "autre",
];
pub(crate) const MOTIFS_RETRAIT: [&str; 8] = [
    "absence_annoncee",
    "sortie",
    "mobilite",
    "stage",
    "sejour_scolaire",
    "exclusion_internat",
    "fermeture",
    "sante",
];
const MOTIFS_NUIT_EXCEPTIONNELLE: [&str; 6] = [
    "examen",
    "accueil_weekend",
    "intemperies",
    "delai_accorde",
    "decision_direction",
    "autre",
];

/// Marque autorisée pour un statut de la liste figée (R-IT14).
pub fn marque_permise(statut: &str, marque: &str) -> bool {
    match statut {
        "attendu" | "ajoute" => MARQUES.contains(&marque) && marque != "non_attendu_present",
        "retire" | "non_attendu" => marque == "non_attendu_present",
        _ => false,
    }
}

/// Une marque qui laisse l'élève « hors de l'internat » en attente de son retour (R-IT16).
pub fn marque_en_attente(marque: &str) -> bool {
    matches!(marque, "activite" | "retard")
}

/// Niveau d'avis d'une validation (R-IT34).
pub fn niveau_avis(nb_ecarts: i32, cas_ouverts: i32) -> &'static str {
    if cas_ouverts > 0 {
        "U3"
    } else if nb_ecarts > 0 {
        "U2"
    } else {
        "U1"
    }
}

// ───────────────────────── Lectures communes ─────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct AppelRow {
    pub id: Uuid,
    pub nuit: NaiveDate,
    pub type_appel: String,
    pub perimetre: String,
    pub batiment_id: Option<Uuid>,
    pub zone_id: Option<Uuid>,
    pub liste_id: Option<Uuid>,
    pub etat: String,
    pub a_posteriori: bool,
    pub ouvert_le: DateTime<Utc>,
    pub clos_le: Option<DateTime<Utc>>,
    pub valide_le: Option<DateTime<Utc>>,
    pub nb_attendus: i32,
    pub nb_ecarts: Option<i32>,
}

impl AppelRow {
    pub(crate) fn dto(&self) -> AppelDto {
        AppelDto {
            id: self.id,
            nuit: self.nuit,
            type_appel: self.type_appel.clone(),
            perimetre: self.perimetre.clone(),
            batiment_id: self.batiment_id,
            zone_id: self.zone_id,
            liste_id: self.liste_id,
            etat: self.etat.clone(),
            a_posteriori: self.a_posteriori,
            ouvert_le: self.ouvert_le,
            clos_le: self.clos_le,
            valide_le: self.valide_le,
            nb_attendus: self.nb_attendus,
            nb_ecarts: self.nb_ecarts,
        }
    }
}

type AppelTuple = (
    Uuid,
    NaiveDate,
    String,
    String,
    Option<Uuid>,
    Option<Uuid>,
    Option<Uuid>,
    String,
    bool,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    i32,
    Option<i32>,
);

const COLONNES_APPEL: &str = "id, nuit, type, perimetre, batiment_id, zone_id, liste_id, etat, \
     a_posteriori, ouvert_le, clos_le, valide_le, nb_attendus, nb_ecarts";

fn depuis_tuple(t: AppelTuple) -> AppelRow {
    AppelRow {
        id: t.0,
        nuit: t.1,
        type_appel: t.2,
        perimetre: t.3,
        batiment_id: t.4,
        zone_id: t.5,
        liste_id: t.6,
        etat: t.7,
        a_posteriori: t.8,
        ouvert_le: t.9,
        clos_le: t.10,
        valide_le: t.11,
        nb_attendus: t.12,
        nb_ecarts: t.13,
    }
}

/// Appel de l'établissement, verrouillé (`FOR UPDATE`) ou partagé (`FOR SHARE`). `NotFound` sinon.
pub(crate) async fn appel_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: &str,
) -> Result<AppelRow, EntDbError> {
    let sql = format!(
        "SELECT {COLONNES_APPEL} FROM ent_internat_appel WHERE etablissement_id = $1 AND id = $2 {verrou}"
    );
    let t: Option<AppelTuple> = sqlx::query_as(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
    t.map(depuis_tuple).ok_or(DomainError::NotFound.into())
}

pub(crate) async fn nuit_a_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    instant: DateTime<Utc>,
) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_nuit_etab($1, $2)")
        .bind(etab)
        .bind(instant)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

/// Ligne de l'attendu d'une nuit, avec le lit lisible et l'identité (le masque est posé après).
#[derive(Debug, Clone)]
pub(crate) struct LigneAttendu {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub chambre_id: Option<Uuid>,
    pub chambre_code: Option<String>,
    pub lit_numero: Option<i16>,
    pub statut: String,
    pub motif_code: Option<String>,
}

type LigneTuple = (
    Uuid,
    String,
    String,
    Option<Uuid>,
    Option<String>,
    Option<i16>,
    String,
    Option<String>,
);

fn ligne_de(t: LigneTuple) -> LigneAttendu {
    LigneAttendu {
        eleve_id: t.0,
        nom: t.1,
        prenom: t.2,
        chambre_id: t.3,
        chambre_code: t.4,
        lit_numero: t.5,
        statut: t.6,
        motif_code: t.7,
    }
}

/// Attendu calculé d'une nuit pour un périmètre (`internat`, `batiment`, `zone`, `liste`).
pub(crate) async fn attendu_perimetre_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    nuit: NaiveDate,
    perimetre: &str,
    batiment_id: Option<Uuid>,
    zone_id: Option<Uuid>,
    liste_id: Option<Uuid>,
) -> Result<Vec<LigneAttendu>, EntDbError> {
    let rows: Vec<LigneTuple> = sqlx::query_as(
        "SELECT a.eleve_id, e.nom, e.prenom, a.chambre_id, c.code, l.numero, a.statut, a.motif_code \
           FROM ent_internat_attendu_calcule($1, $2) a \
           JOIN ent_eleve e ON e.etablissement_id = $1 AND e.id = a.eleve_id \
           LEFT JOIN ent_internat_chambre c ON c.etablissement_id = $1 AND c.id = a.chambre_id \
           LEFT JOIN ent_internat_zone z ON z.etablissement_id = $1 AND z.id = c.zone_id \
           LEFT JOIN ent_internat_lit l ON l.etablissement_id = $1 AND l.id = a.lit_id \
          WHERE $3 = 'internat' \
             OR ($3 = 'batiment' AND z.batiment_id = $4) \
             OR ($3 = 'zone' AND c.zone_id = $5) \
             OR ($3 = 'liste' AND a.chambre_id IN ( \
                    SELECT lc.chambre_id FROM ent_internat_liste_chambre lc \
                     WHERE lc.etablissement_id = $1 AND lc.liste_id = $6)) \
          ORDER BY c.code NULLS LAST, l.numero NULLS LAST, e.nom, e.prenom, a.eleve_id",
    )
    .bind(etab)
    .bind(nuit)
    .bind(perimetre)
    .bind(batiment_id)
    .bind(zone_id)
    .bind(liste_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(ligne_de).collect())
}

/// Liste figée d'un appel.
pub(crate) async fn attendu_fige_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    appel: Uuid,
) -> Result<Vec<LigneAttendu>, EntDbError> {
    let rows: Vec<LigneTuple> = sqlx::query_as(
        "SELECT x.eleve_id, e.nom, e.prenom, x.chambre_id, c.code, l.numero, x.statut, x.motif_code \
           FROM ent_internat_appel_attendu x \
           JOIN ent_eleve e ON e.etablissement_id = x.etablissement_id AND e.id = x.eleve_id \
           LEFT JOIN ent_internat_chambre c ON c.etablissement_id = x.etablissement_id AND c.id = x.chambre_id \
           LEFT JOIN ent_internat_lit l ON l.etablissement_id = x.etablissement_id AND l.id = x.lit_id \
          WHERE x.etablissement_id = $1 AND x.appel_id = $2 \
          ORDER BY c.code NULLS LAST, l.numero NULLS LAST, e.nom, e.prenom, x.eleve_id",
    )
    .bind(etab)
    .bind(appel)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(ligne_de).collect())
}

/// Dernier fait d'un élève dans un appel, et conflit éventuel.
#[derive(Debug, Clone)]
pub(crate) struct MarqueCourante {
    pub marque: String,
    pub a: DateTime<Utc>,
    pub par_user_id: Uuid,
    pub retour_attendu: Option<DateTime<Utc>>,
    pub conflit: bool,
    pub corrige: bool,
}

pub(crate) async fn marques_courantes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    appel: Uuid,
) -> Result<HashMap<Uuid, MarqueCourante>, EntDbError> {
    // Deux derniers faits de chaque élève : le plus récent est la valeur courante ; s'ils viennent
    // de deux personnes, disent deux choses et sont proches (≤ 120 s), c'est un conflit (CIT-03).
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        Uuid,
        String,
        DateTime<Utc>,
        Uuid,
        Option<DateTime<Utc>>,
        Option<String>,
        i64,
        Option<String>,
        Option<Uuid>,
        Option<DateTime<Utc>>,
    )> = sqlx::query_as(
        "SELECT eleve_id, marque, a, par_user_id, retour_attendu, motif_correction_code, rang, \
                marque_prec, par_prec, a_prec \
           FROM ( \
             SELECT f.eleve_id, f.marque, f.a, f.par_user_id, f.retour_attendu, f.motif_correction_code, \
                    row_number() OVER w AS rang, \
                    lead(f.marque) OVER w AS marque_prec, \
                    lead(f.par_user_id) OVER w AS par_prec, \
                    lead(f.a) OVER w AS a_prec \
               FROM ent_internat_presence_fait f \
              WHERE f.etablissement_id = $1 AND f.appel_id = $2 \
             WINDOW w AS (PARTITION BY f.eleve_id ORDER BY f.recu_le DESC, f.cree_le DESC, f.id DESC) \
           ) t WHERE rang = 1",
    )
    .bind(etab)
    .bind(appel)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(
            |(eleve, marque, a, par, retour, motif, _rang, marque_prec, par_prec, a_prec)| {
                let conflit = match (marque_prec, par_prec, a_prec) {
                    (Some(m), Some(p), Some(ap)) => {
                        m != marque && p != par && (a - ap).num_seconds().abs() <= 120
                    }
                    _ => false,
                };
                (
                    eleve,
                    MarqueCourante {
                        marque,
                        a,
                        par_user_id: par,
                        retour_attendu: retour,
                        conflit,
                        corrige: motif.is_some(),
                    },
                )
            },
        )
        .collect())
}

pub(crate) async fn journaliser_lecture_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
    objet: &str,
    eleve: Option<Uuid>,
    appel: Option<Uuid>,
    instant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_internat_lecture_sensible (id, etablissement_id, user_id, objet, eleve_id, appel_id, a) \
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(user)
    .bind(objet)
    .bind(eleve)
    .bind(appel)
    .bind(instant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

impl PgEnt {
    /// Droit + portée sur les élèves. `Forbidden` sans aucun droit ; `None` = tout l'établissement ;
    /// `Some(ensemble)` = seulement ces élèves (R-IT30). Une des actions suffit.
    pub(crate) async fn nuit_portee(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        actions: &[Action],
    ) -> Result<Option<HashSet<Uuid>>, EntDbError> {
        let droits = self.droits_effectifs(etab, actor, role).await?;
        let mut retenue: Option<Action> = None;
        for a in actions {
            if droits.iter().any(|d| accorde(d.action, *a)) {
                retenue = Some(*a);
                // Une action portée établissement l'emporte sur les portées restreintes.
                if droits
                    .iter()
                    .any(|d| accorde(d.action, *a) && d.portee == ent_auth::Portee::Etablissement)
                {
                    return Ok(None);
                }
            }
        }
        let Some(a) = retenue else {
            return Err(EntDbError::Forbidden);
        };
        self.eleves_autorises(etab, actor, role, a).await
    }

    /// Noms masqués des élèves protégés (E7) et trace de leur consultation.
    pub(crate) async fn masquer_proteges(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        ids: &[Uuid],
        libelle: &str,
    ) -> Result<(HashSet<Uuid>, HashSet<Uuid>), EntDbError> {
        if ids.is_empty() {
            return Ok((HashSet::new(), HashSet::new()));
        }
        let profil = self.profil_champs(etab, actor, role).await?;
        let caches = self.proteges_caches(etab, role, &profil, ids).await?;
        let proteges = self.eleves_proteges_parmi(etab, ids).await?;
        for e in &proteges {
            self.tracer_consultation(etab, actor, role, Some(*e), "internat.nuit", libelle, false)
                .await?;
        }
        Ok((proteges, caches))
    }

    // ───────────────────────── Attendu ─────────────────────────

    /// INT-129 : attendu calculé d'une nuit (lecture, journalisée).
    pub async fn attendu_nuit_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        nuit: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<AttenduDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatLire])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut lignes =
            attendu_perimetre_tx(&mut tx, etab, nuit, "internat", None, None, None).await?;
        if let Some(set) = &portee {
            lignes.retain(|l| set.contains(&l.eleve_id));
        }
        journaliser_lecture_tx(&mut tx, etab, actor, "attendu", None, None, maintenant).await?;
        tx.commit().await.map_err(db)?;
        let ids: Vec<Uuid> = lignes.iter().map(|l| l.eleve_id).collect();
        let (proteges, caches) = self
            .masquer_proteges(etab, actor, role, &ids, "attendu de la nuit")
            .await?;
        Ok(AttenduDto {
            nuit,
            calcule_le: maintenant,
            lignes: lignes
                .into_iter()
                .map(|l| AttenduLigneDto {
                    eleve_id: l.eleve_id,
                    nom: (!caches.contains(&l.eleve_id)).then_some(l.nom),
                    prenom: (!caches.contains(&l.eleve_id)).then_some(l.prenom),
                    protege: proteges.contains(&l.eleve_id),
                    chambre_code: l.chambre_code,
                    lit_numero: l.lit_numero,
                    statut: l.statut,
                    motif_code: l.motif_code,
                })
                .collect(),
        })
    }

    async fn eleve_existe_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<(), EntDbError> {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if ok {
            Ok(())
        } else {
            Err(DomainError::NotFound.into())
        }
    }

    /// INT-121 : nuit exceptionnelle (élève accueilli une nuit de plus). Jamais dans le passé.
    pub async fn poser_nuit_exceptionnelle_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &NuitExceptionnelleSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<NuitExceptionnelleDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        if !MOTIFS_NUIT_EXCEPTIONNELLE.contains(&b.motif_code.as_str()) {
            return Err(refus("motif de nuit exceptionnelle inconnu"));
        }
        // Un élève protégé reste invisible d'un tiers : réponse « absent » (E7).
        self.exiger_eleve_visible(etab, actor, role, b.eleve_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        Self::eleve_existe_tx(&mut tx, etab, b.eleve_id).await?;
        let courante = nuit_a_tx(&mut tx, etab, maintenant).await?;
        if b.nuit < courante {
            return Err(refus("une nuit exceptionnelle ne se pose pas dans le passé"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_nuit_exceptionnelle \
             (id, etablissement_id, eleve_id, nuit, motif_code, par_user_id, cree_le) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.eleve_id)
        .bind(b.nuit)
        .bind(&b.motif_code)
        .bind(actor)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if est_unique(&e) {
                conflit("cette nuit exceptionnelle existe déjà pour cet élève")
            } else {
                db(e)
            }
        })?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_NUIT_EXCEPTIONNELLE,
            AggregateKind::Internat,
            id,
            Some(b.eleve_id),
            json!({ "nuit": b.nuit.to_string(), "motif_code": b.motif_code }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_ATTENDU_MODIFIE,
            AggregateKind::Internat,
            id,
            Some(b.eleve_id),
            json!({ "nuit": b.nuit.to_string(), "cause": "nuit_exceptionnelle" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(NuitExceptionnelleDto {
            id,
            eleve_id: b.eleve_id,
            nuit: b.nuit,
            motif_code: b.motif_code.clone(),
            annulee: false,
        })
    }

    pub async fn annuler_nuit_exceptionnelle_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: Option<(Uuid, NaiveDate)> = sqlx::query_as(
            "UPDATE ent_internat_nuit_exceptionnelle SET annulee_le = $3, annulee_par_user_id = $4 \
              WHERE etablissement_id = $1 AND id = $2 AND annulee_le IS NULL RETURNING eleve_id, nuit",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .bind(actor)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (eleve, nuit) = r.ok_or(DomainError::NotFound)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_ATTENDU_MODIFIE,
            AggregateKind::Internat,
            id,
            Some(eleve),
            json!({ "nuit": nuit.to_string(), "cause": "nuit_exceptionnelle_annulee" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Retrait de l'attendu (adaptateur des socles 12, 13, 34, 39 et saisie du CPE). L'infirmerie ne
    /// peut poser que « santé », sans détail (hospitalisation) ; tout autre retrait exige `internat.gerer`.
    pub async fn poser_retrait_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &RetraitSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<RetraitDto, EntDbError> {
        if !MOTIFS_RETRAIT.contains(&b.motif_code.as_str()) {
            return Err(refus("motif de retrait inconnu"));
        }
        let parsed = ent_auth::Role::parse(role);
        let source = if parsed == Some(ent_auth::Role::Infirmerie) {
            // Port du socle 34 : l'infirmière dit seulement « absent : santé ».
            if b.motif_code != "sante" {
                return Err(EntDbError::Forbidden);
            }
            "infirmerie"
        } else {
            self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
            if matches!(parsed, Some(ent_auth::Role::Direction | ent_auth::Role::Admin)) {
                "direction"
            } else {
                "cpe"
            }
        };
        if b.au_nuit < b.du_nuit || (b.au_nuit - b.du_nuit).num_days() > 400 {
            return Err(refus("période de retrait invalide"));
        }
        self.exiger_eleve_visible(etab, actor, role, b.eleve_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        Self::eleve_existe_tx(&mut tx, etab, b.eleve_id).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_retrait \
             (id, etablissement_id, eleve_id, du_nuit, au_nuit, motif_code, source, par_user_id, cree_le) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.eleve_id)
        .bind(b.du_nuit)
        .bind(b.au_nuit)
        .bind(&b.motif_code)
        .bind(source)
        .bind(actor)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_ATTENDU_MODIFIE,
            AggregateKind::Internat,
            id,
            Some(b.eleve_id),
            json!({ "du": b.du_nuit.to_string(), "au": b.au_nuit.to_string(), "cause": "retrait", "motif_code": b.motif_code }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(RetraitDto {
            id,
            eleve_id: b.eleve_id,
            du_nuit: b.du_nuit,
            au_nuit: b.au_nuit,
            motif_code: b.motif_code.clone(),
            source: source.into(),
            annule: false,
        })
    }

    pub async fn annuler_retrait_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let infirmerie = ent_auth::Role::parse(role) == Some(ent_auth::Role::Infirmerie);
        if !infirmerie {
            self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: Option<(Uuid, NaiveDate, NaiveDate, String)> = sqlx::query_as(
            "UPDATE ent_internat_retrait SET annule_le = $3, annule_par_user_id = $4 \
              WHERE etablissement_id = $1 AND id = $2 AND annule_le IS NULL \
                AND ($5 = false OR motif_code = 'sante') \
          RETURNING eleve_id, du_nuit, au_nuit, motif_code",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .bind(actor)
        .bind(infirmerie)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (eleve, du, au, motif) = r.ok_or(DomainError::NotFound)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_ATTENDU_MODIFIE,
            AggregateKind::Internat,
            id,
            Some(eleve),
            json!({ "du": du.to_string(), "au": au.to_string(), "cause": "retrait_annule", "motif_code": motif }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────── Listes ─────────────────────────

    pub async fn creer_liste_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ListeSaisieDto,
    ) -> Result<ListeDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let libelle = b.libelle.trim();
        if libelle.is_empty() || libelle.chars().count() > 80 {
            return Err(refus("libellé de liste invalide"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_liste (id, etablissement_id, libelle, confie_a_user_id) VALUES ($1,$2,$3,$4)",
        )
        .bind(id)
        .bind(etab)
        .bind(libelle)
        .bind(b.confie_a_user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if est_unique(&e) {
                conflit("une liste porte déjà ce libellé")
            } else {
                db(e)
            }
        })?;
        Self::poser_chambres_liste_tx(&mut tx, etab, id, &b.chambre_ids).await?;
        tx.commit().await.map_err(db)?;
        Ok(ListeDto {
            id,
            libelle: libelle.into(),
            chambre_ids: b.chambre_ids.clone(),
            confie_a_user_id: b.confie_a_user_id,
            actif: true,
        })
    }

    async fn poser_chambres_liste_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        liste: Uuid,
        chambres: &[Uuid],
    ) -> Result<(), EntDbError> {
        sqlx::query("DELETE FROM ent_internat_liste_chambre WHERE etablissement_id = $1 AND liste_id = $2")
            .bind(etab)
            .bind(liste)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        for c in chambres {
            sqlx::query(
                "INSERT INTO ent_internat_liste_chambre (etablissement_id, liste_id, chambre_id) \
                 VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
            )
            .bind(etab)
            .bind(liste)
            .bind(c)
            .execute(&mut *conn)
            .await
            .map_err(|e| match e {
                sqlx::Error::Database(ref d) if d.code().as_deref() == Some("23503") => {
                    EntDbError::Domain(DomainError::NotFound)
                }
                e => db(e),
            })?;
        }
        Ok(())
    }

    /// Remplace les chambres d'une liste et son surveillant ; `actif = false` la retire sans la supprimer.
    pub async fn modifier_liste_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &ListeSaisieDto,
        actif: bool,
    ) -> Result<ListeDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let libelle = b.libelle.trim();
        if libelle.is_empty() || libelle.chars().count() > 80 {
            return Err(refus("libellé de liste invalide"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE ent_internat_liste SET libelle = $3, confie_a_user_id = $4, actif = $5, modifie_le = now() \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(libelle)
        .bind(b.confie_a_user_id)
        .bind(actif)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound.into());
        }
        Self::poser_chambres_liste_tx(&mut tx, etab, id, &b.chambre_ids).await?;
        tx.commit().await.map_err(db)?;
        Ok(ListeDto {
            id,
            libelle: libelle.into(),
            chambre_ids: b.chambre_ids.clone(),
            confie_a_user_id: b.confie_a_user_id,
            actif,
        })
    }

    pub async fn lister_listes_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<ListeDto>, EntDbError> {
        self.nuit_portee(etab, actor, role, &[Action::InternatLire, Action::InternatAppel, Action::InternatGerer])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let listes: Vec<(Uuid, String, Option<Uuid>, bool)> = sqlx::query_as(
            "SELECT id, libelle, confie_a_user_id, actif FROM ent_internat_liste \
              WHERE etablissement_id = $1 ORDER BY libelle",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let liens: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT liste_id, chambre_id FROM ent_internat_liste_chambre WHERE etablissement_id = $1 ORDER BY chambre_id",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(listes
            .into_iter()
            .map(|(id, libelle, confie, actif)| ListeDto {
                id,
                libelle,
                chambre_ids: liens.iter().filter(|(l, _)| *l == id).map(|(_, c)| *c).collect(),
                confie_a_user_id: confie,
                actif,
            })
            .collect())
    }

    // ───────────────────────── Appels ─────────────────────────

    /// INT-130 : ouvre un appel. La liste de ce qu'on croit à cet instant est **figée** (ce qu'on croyait
    /// à l'heure de l'appel). Un seul appel par type, nuit et périmètre (R-IT31).
    pub async fn ouvrir_appel_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &AppelOuvertureDto,
        maintenant: DateTime<Utc>,
    ) -> Result<AppelVueDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatAppel])
            .await?;
        if !TYPES_APPEL.contains(&b.type_appel.as_str()) {
            return Err(refus("type d'appel inconnu"));
        }
        if !PERIMETRES.contains(&b.perimetre.as_str()) {
            return Err(refus("périmètre d'appel inconnu"));
        }
        let coherent = match b.perimetre.as_str() {
            "internat" => b.batiment_id.is_none() && b.zone_id.is_none() && b.liste_id.is_none(),
            "batiment" => b.batiment_id.is_some() && b.zone_id.is_none() && b.liste_id.is_none(),
            "zone" => b.zone_id.is_some() && b.batiment_id.is_none() && b.liste_id.is_none(),
            _ => b.liste_id.is_some() && b.batiment_id.is_none() && b.zone_id.is_none(),
        };
        if !coherent {
            return Err(refus("périmètre incohérent avec l'identifiant fourni"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Type d'appel désactivé par l'établissement (S-4101).
        let actif: Option<bool> = sqlx::query_scalar(
            "SELECT actif FROM ent_internat_type_appel_actif WHERE etablissement_id = $1 AND type = $2",
        )
        .bind(etab)
        .bind(&b.type_appel)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if actif == Some(false) {
            return Err(refus("ce type d'appel est désactivé dans l'établissement"));
        }
        let courante = nuit_a_tx(&mut tx, etab, maintenant).await?;
        let nuit = b.nuit.unwrap_or(courante);
        if nuit > courante {
            return Err(DomainError::InternatNuitFuture.into());
        }
        // Le périmètre désigné doit exister dans cet établissement.
        let existe: bool = match b.perimetre.as_str() {
            "internat" => true,
            "batiment" => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_batiment WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(b.batiment_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?,
            "zone" => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_internat_zone WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(b.zone_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?,
            _ => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_internat_liste WHERE etablissement_id = $1 AND id = $2 AND actif)",
            )
            .bind(etab)
            .bind(b.liste_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?,
        };
        if !existe {
            return Err(DomainError::NotFound.into());
        }
        let lignes = attendu_perimetre_tx(
            &mut tx,
            etab,
            nuit,
            &b.perimetre,
            b.batiment_id,
            b.zone_id,
            b.liste_id,
        )
        .await?;
        // R-IT30 : l'acteur doit couvrir TOUS les élèves du périmètre.
        if let Some(set) = &portee {
            if lignes.iter().any(|l| !set.contains(&l.eleve_id)) {
                return Err(EntDbError::Forbidden);
            }
        }
        let id = Uuid::new_v4();
        let nb_attendus = lignes
            .iter()
            .filter(|l| l.statut == "attendu" || l.statut == "ajoute")
            .count() as i32;
        sqlx::query(
            "INSERT INTO ent_internat_appel \
             (id, etablissement_id, nuit, type, perimetre, batiment_id, zone_id, liste_id, a_posteriori, \
              ouvert_par_user_id, ouvert_le, nb_attendus) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(id)
        .bind(etab)
        .bind(nuit)
        .bind(&b.type_appel)
        .bind(&b.perimetre)
        .bind(b.batiment_id)
        .bind(b.zone_id)
        .bind(b.liste_id)
        .bind(nuit < courante)
        .bind(actor)
        .bind(maintenant)
        .bind(nb_attendus)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if est_unique(&e) {
                conflit("un appel existe déjà pour ce type, cette nuit et ce périmètre")
            } else {
                db(e)
            }
        })?;
        for l in &lignes {
            sqlx::query(
                "INSERT INTO ent_internat_appel_attendu \
                 (etablissement_id, appel_id, eleve_id, chambre_id, lit_id, statut, motif_code, calcule_le) \
                 SELECT $1, $2, $3, a.chambre_id, a.lit_id, $4, $5, $6 \
                   FROM ent_internat_attendu_calcule($1, $7) a WHERE a.eleve_id = $3",
            )
            .bind(etab)
            .bind(id)
            .bind(l.eleve_id)
            .bind(&l.statut)
            .bind(&l.motif_code)
            .bind(maintenant)
            .bind(nuit)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        publier(
            &mut tx,
            etab,
            types::INTERNAT_APPEL_OUVERT,
            AggregateKind::Internat,
            id,
            None,
            json!({
                "nuit": nuit.to_string(), "type": b.type_appel, "perimetre": b.perimetre,
                "a_posteriori": nuit < courante, "nb_attendus": nb_attendus
            }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.vue_appel_a(etab, actor, role, id, maintenant).await
    }

    /// Appels d'une nuit (liste, sans noms) : lecture soumise à la portée.
    pub async fn lister_appels_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        nuit: NaiveDate,
    ) -> Result<Vec<AppelDto>, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatLire, Action::InternatAppel])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let sql = format!(
            "SELECT {COLONNES_APPEL} FROM ent_internat_appel WHERE etablissement_id = $1 AND nuit = $2 \
              ORDER BY ouvert_le, id"
        );
        let rows: Vec<AppelTuple> = sqlx::query_as(&sql)
            .bind(etab)
            .bind(nuit)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut sortie = Vec::new();
        for r in rows {
            let appel = depuis_tuple(r);
            if let Some(set) = &portee {
                // Un appel n'est listé que s'il ne contient aucun élève hors de la portée de l'acteur.
                let hors: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_internat_appel_attendu \
                      WHERE etablissement_id = $1 AND appel_id = $2 AND NOT (eleve_id = ANY($3)))",
                )
                .bind(etab)
                .bind(appel.id)
                .bind(set.iter().copied().collect::<Vec<_>>())
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if hors {
                    continue;
                }
            }
            sortie.push(appel.dto());
        }
        Ok(sortie)
    }

    /// Vue d'un appel : liste figée, valeur courante de chaque élève, conflits, écarts avec l'attendu
    /// actuel. Lecture journalisée ; élèves protégés sans nom (E7).
    pub async fn vue_appel_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        appel_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<AppelVueDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatLire, Action::InternatAppel])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let appel = appel_tx(&mut tx, etab, appel_id, "").await?;
        let mut lignes = attendu_fige_tx(&mut tx, etab, appel_id).await?;
        if let Some(set) = &portee {
            // R-IT30 : un appel qui déborde de la portée n'est pas lisible (même sans le filtrer ligne à ligne).
            if lignes.iter().any(|l| !set.contains(&l.eleve_id)) {
                return Err(EntDbError::Forbidden);
            }
        }
        let marques = marques_courantes_tx(&mut tx, etab, appel_id).await?;
        let actuel: HashMap<Uuid, (String, Option<String>)> = attendu_perimetre_tx(
            &mut tx,
            etab,
            appel.nuit,
            &appel.perimetre,
            appel.batiment_id,
            appel.zone_id,
            appel.liste_id,
        )
        .await?
        .into_iter()
        .map(|l| (l.eleve_id, (l.statut, l.motif_code)))
        .collect();
        let mut ecarts: Vec<Uuid> = lignes
            .iter()
            .filter(|l| actuel.get(&l.eleve_id).is_some_and(|(s, m)| *s != l.statut || *m != l.motif_code))
            .map(|l| l.eleve_id)
            .collect();
        let figes: HashSet<Uuid> = lignes.iter().map(|l| l.eleve_id).collect();
        ecarts.extend(actuel.keys().filter(|e| !figes.contains(e)).copied());
        ecarts.sort();
        journaliser_lecture_tx(&mut tx, etab, actor, "appel", None, Some(appel_id), maintenant).await?;
        tx.commit().await.map_err(db)?;
        let ids: Vec<Uuid> = lignes.iter().map(|l| l.eleve_id).collect();
        let (proteges, caches) = self
            .masquer_proteges(etab, actor, role, &ids, "appel de nuit")
            .await?;
        lignes.sort_by(|a, b| {
            (&a.chambre_code, a.lit_numero, &a.nom, &a.prenom).cmp(&(&b.chambre_code, b.lit_numero, &b.nom, &b.prenom))
        });
        let mut nb_pointes = 0;
        let lignes: Vec<AppelLigneDto> = lignes
            .into_iter()
            .map(|l| {
                let m = marques.get(&l.eleve_id);
                if m.is_some() || l.statut == "retire" {
                    nb_pointes += 1;
                }
                AppelLigneDto {
                    eleve_id: l.eleve_id,
                    nom: (!caches.contains(&l.eleve_id)).then_some(l.nom),
                    prenom: (!caches.contains(&l.eleve_id)).then_some(l.prenom),
                    protege: proteges.contains(&l.eleve_id),
                    chambre_code: l.chambre_code,
                    lit_numero: l.lit_numero,
                    statut: l.statut,
                    motif_code: l.motif_code,
                    marque: m.map(|m| m.marque.clone()),
                    marque_a: m.map(|m| m.a),
                    marque_par_user_id: m.map(|m| m.par_user_id),
                    retour_attendu: m.and_then(|m| m.retour_attendu),
                    conflit: m.is_some_and(|m| m.conflit),
                    renvoi_infirmerie: m.is_some_and(|m| m.marque == "infirmerie"),
                    corrige: m.is_some_and(|m| m.corrige),
                }
            })
            .collect();
        Ok(AppelVueDto {
            appel: appel.dto(),
            calcule_le: maintenant,
            lignes,
            nb_pointes,
            ecarts_avec_attendu_actuel: ecarts,
        })
    }

    // ───────────────────────── Pointage ─────────────────────────

    /// INT-131 à INT-134 : pointe un élève. Ajout seul : rien n'écrase le fait d'un autre surveillant.
    pub async fn pointer_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        appel_id: Uuid,
        b: &PointageSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<PointageResultatDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatAppel])
            .await?;
        if let Some(set) = &portee {
            if !set.contains(&b.eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let appel = appel_tx(&mut tx, etab, appel_id, "FOR SHARE").await?;
        let correction = appel.etat != "ouvert";
        if correction {
            // Une correction après clôture ou validation exige le droit de valider (R-IT17).
            self.nuit_portee(etab, actor, role, &[Action::InternatAppelValider])
                .await?;
        }
        let resultat = self
            .pointer_tx(&mut tx, etab, actor, &appel, b, maintenant)
            .await?;
        tx.commit().await.map_err(db)?;
        if self.est_protege(etab, b.eleve_id).await? {
            self.tracer_consultation(etab, actor, role, Some(b.eleve_id), "internat.nuit", "pointage d'un élève protégé", false)
                .await?;
        }
        Ok(resultat)
    }

    async fn est_protege(&self, etab: Uuid, eleve: Uuid) -> Result<bool, EntDbError> {
        Ok(self.eleves_proteges_parmi(etab, &[eleve]).await?.contains(&eleve))
    }

    pub(crate) async fn pointer_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        appel: &AppelRow,
        b: &PointageSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<PointageResultatDto, EntDbError> {
        if !MARQUES.contains(&b.marque.as_str()) {
            return Err(refus("marque de pointage inconnue"));
        }
        let note = b.note.as_deref().map(str::trim).unwrap_or("");
        if note.chars().count() > 500 {
            return Err(refus("note trop longue (500 caractères au plus)"));
        }
        if !note.is_empty() && domain_eleve::mots_de_sante_detectes(note) {
            // R-IT6 : aucune donnée de santé dans l'appel ; renvoi vers l'infirmerie.
            return Err(refus(
                "pas de donnée de santé dans l'appel : utiliser la marque « à l'infirmerie »",
            ));
        }
        let correction = appel.etat != "ouvert";
        let motif_code = b.motif_correction_code.as_deref();
        let motif_texte = b.motif_correction_texte.as_deref().map(str::trim).filter(|t| !t.is_empty());
        if correction {
            match motif_code {
                Some(c) if MOTIFS_CORRECTION.contains(&c) => {}
                _ => return Err(DomainError::InternatMotifRequis.into()),
            }
        } else if motif_code.is_some() {
            return Err(refus("un motif de correction n'a de sens que sur un appel clos ou validé"));
        }
        if motif_texte.is_some_and(|t| t.chars().count() > 300) {
            return Err(refus("motif de correction trop long (300 caractères au plus)"));
        }
        // Heure du geste : jamais dans le futur ; et dans la nuit de l'appel (sauf saisie a posteriori).
        let a = b.a.unwrap_or(maintenant);
        if a > maintenant + Duration::minutes(5) {
            return Err(refus("l'heure du geste est dans le futur"));
        }
        if !appel.a_posteriori && !correction && nuit_a_tx(conn, etab, a).await? != appel.nuit {
            return Err(refus("l'heure du geste n'est pas dans la nuit de l'appel"));
        }
        if marque_en_attente(&b.marque) && b.marque == "retard" && b.retour_attendu.is_none() {
            return Err(refus("une arrivée tardive annoncée porte l'heure de retour attendue"));
        }
        // R-IT14 : l'élève doit figurer dans la liste figée, avec une marque compatible avec son statut.
        let statut: Option<String> = sqlx::query_scalar(
            "SELECT statut FROM ent_internat_appel_attendu WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3",
        )
        .bind(etab)
        .bind(appel.id)
        .bind(b.eleve_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some(statut) = statut else {
            return Err(refus("élève hors du périmètre de l'appel ou hors séjour (R-IT14)"));
        };
        if !marque_permise(&statut, &b.marque) {
            return Err(refus(
                "marque incompatible avec l'attendu : un élève non attendu ne se pointe que « non attendu présent »",
            ));
        }
        let affectation: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_internat_affectation \
              WHERE etablissement_id = $1 AND eleve_id = $2 AND debut <= $3 AND (fin IS NULL OR fin >= $3) \
              ORDER BY debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(b.eleve_id)
        .bind(appel.nuit)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let precedent: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_internat_presence_fait WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3 \
              ORDER BY recu_le DESC, cree_le DESC, id DESC LIMIT 1",
        )
        .bind(etab)
        .bind(appel.id)
        .bind(b.eleve_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let fait = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_presence_fait \
             (id, etablissement_id, eleve_id, affectation_id, appel_id, nuit, marque, note, retour_attendu, a, recu_le, \
              par_user_id, appareil_id, correction_de, motif_correction_code, motif_correction_texte, cree_le) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$11)",
        )
        .bind(fait)
        .bind(etab)
        .bind(b.eleve_id)
        .bind(affectation)
        .bind(appel.id)
        .bind(appel.nuit)
        .bind(&b.marque)
        .bind(note)
        .bind(b.retour_attendu)
        .bind(a)
        .bind(maintenant)
        .bind(actor)
        .bind(b.appareil_id)
        .bind(if correction { precedent } else { None })
        .bind(motif_code)
        .bind(motif_texte)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        // Retour d'un élève absent : le cas ouvert se ferme (« rentré à 23 h 10 »).
        if matches!(b.marque.as_str(), "present" | "vu") {
            self.clore_cas_retour_tx(conn, etab, actor, b.eleve_id, appel.nuit, a)
                .await?;
        }
        let marques = marques_courantes_tx(conn, etab, appel.id).await?;
        let conflit = marques.get(&b.eleve_id).is_some_and(|m| m.conflit);
        let evenement = if correction && appel.etat == "valide" {
            types::INTERNAT_APPEL_CORRIGE
        } else {
            types::INTERNAT_PRESENCE_POINTEE
        };
        publier(
            conn,
            etab,
            evenement,
            AggregateKind::Internat,
            fait,
            Some(b.eleve_id),
            json!({
                "appel_id": appel.id.to_string(), "nuit": appel.nuit.to_string(),
                "marque": b.marque, "correction": correction, "conflit": conflit
            }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        Ok(PointageResultatDto {
            fait_id: fait,
            eleve_id: b.eleve_id,
            marque: b.marque.clone(),
            conflit,
            renvoi_infirmerie: b.marque == "infirmerie",
        })
    }

    /// « Tous présents » pour une chambre : un fait par élève attendu de la chambre qui n'a encore
    /// aucune marque (un fait existant n'est jamais écrasé : R-IT13).
    pub async fn tous_presents_chambre_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        appel_id: Uuid,
        chambre_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<PointageResultatDto>, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatAppel])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let appel = appel_tx(&mut tx, etab, appel_id, "FOR SHARE").await?;
        if appel.etat != "ouvert" {
            return Err(conflit("l'appel est clos : corriger élève par élève, avec un motif"));
        }
        let figes = attendu_fige_tx(&mut tx, etab, appel_id).await?;
        let marques = marques_courantes_tx(&mut tx, etab, appel_id).await?;
        let mut sortie = Vec::new();
        for l in figes.iter().filter(|l| {
            l.chambre_id == Some(chambre_id)
                && (l.statut == "attendu" || l.statut == "ajoute")
                && !marques.contains_key(&l.eleve_id)
        }) {
            if portee.as_ref().is_some_and(|s| !s.contains(&l.eleve_id)) {
                return Err(EntDbError::Forbidden);
            }
            let r = self
                .pointer_tx(
                    &mut tx,
                    etab,
                    actor,
                    &appel,
                    &PointageSaisieDto {
                        eleve_id: l.eleve_id,
                        marque: "present".into(),
                        note: None,
                        retour_attendu: None,
                        a: None,
                        appareil_id: None,
                        motif_correction_code: None,
                        motif_correction_texte: None,
                    },
                    maintenant,
                )
                .await?;
            sortie.push(r);
        }
        tx.commit().await.map_err(db)?;
        Ok(sortie)
    }

    /// Historique des faits d'un appel (le plus ancien d'abord) : rien n'est effacé.
    pub async fn historique_appel_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        appel_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<HistoriqueFaitDto>, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatLire])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        appel_tx(&mut tx, etab, appel_id, "").await?;
        if let Some(set) = &portee {
            // R-IT30 : l'historique d'un appel qui déborde de la portée n'est pas lisible.
            let figes = attendu_fige_tx(&mut tx, etab, appel_id).await?;
            if figes.iter().any(|l| !set.contains(&l.eleve_id)) {
                return Err(EntDbError::Forbidden);
            }
        }
        #[allow(clippy::type_complexity)]
        let rows: Vec<(Uuid, Uuid, String, String, DateTime<Utc>, DateTime<Utc>, Uuid, Option<String>, Option<String>)> =
            sqlx::query_as(
                "SELECT id, eleve_id, marque, note, a, recu_le, par_user_id, motif_correction_code, motif_correction_texte \
                   FROM ent_internat_presence_fait WHERE etablissement_id = $1 AND appel_id = $2 \
                  ORDER BY recu_le, cree_le, id",
            )
            .bind(etab)
            .bind(appel_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        if let Some(set) = &portee {
            if rows.iter().any(|r| !set.contains(&r.1)) {
                return Err(EntDbError::Forbidden);
            }
        }
        journaliser_lecture_tx(&mut tx, etab, actor, "appel", None, Some(appel_id), maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| HistoriqueFaitDto {
                id: r.0,
                eleve_id: r.1,
                marque: r.2,
                note: r.3,
                a: r.4,
                recu_le: r.5,
                par_user_id: r.6,
                motif_correction_code: r.7,
                motif_correction_texte: r.8,
            })
            .collect())
    }

    // ───────────────────────── Clôture et validation ─────────────────────────

    /// INT-136 : clôture. Impossible avec un attendu non pointé ou une activité en attente de retour ;
    /// chaque absent non couvert ouvre un cas (R-IT16). Le retard annoncé dépassé ouvre un cas de retour.
    pub async fn cloturer_appel_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        appel_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<ClotureResultatDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatAppel, Action::InternatAppelValider])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let appel = appel_tx(&mut tx, etab, appel_id, "FOR UPDATE").await?;
        if appel.etat != "ouvert" {
            return Err(conflit("l'appel n'est plus ouvert"));
        }
        let figes = attendu_fige_tx(&mut tx, etab, appel_id).await?;
        if let Some(set) = &portee {
            if figes.iter().any(|l| !set.contains(&l.eleve_id)) {
                return Err(EntDbError::Forbidden);
            }
        }
        let marques = marques_courantes_tx(&mut tx, etab, appel_id).await?;
        let seuil = Duration::minutes(i64::from(parametres_tx(&mut tx, etab).await?.seuil_retard_minutes));
        let (mut non_pointes, mut en_attente) = (0, 0);
        let mut ecarts = 0;
        let mut a_ouvrir: Vec<(Uuid, &'static str)> = Vec::new();
        for l in figes.iter().filter(|l| l.statut == "attendu" || l.statut == "ajoute") {
            match marques.get(&l.eleve_id) {
                None => non_pointes += 1,
                Some(m) => match m.marque.as_str() {
                    "absent" => {
                        ecarts += 1;
                        a_ouvrir.push((l.eleve_id, "non_retrouve"));
                    }
                    "activite" | "retard" => match m.retour_attendu {
                        Some(r) if r + seuil <= maintenant => {
                            ecarts += 1;
                            a_ouvrir.push((l.eleve_id, "retard_retour"));
                        }
                        _ => en_attente += 1,
                    },
                    _ => {}
                },
            }
        }
        for l in figes.iter().filter(|l| l.statut != "attendu" && l.statut != "ajoute") {
            if marques.get(&l.eleve_id).is_some_and(|m| m.marque == "non_attendu_present") {
                ecarts += 1;
            }
        }
        if non_pointes > 0 {
            return Err(EntDbError::ConflitDossierDetail(format!(
                "{non_pointes} élève(s) attendu(s) non pointé(s) : la clôture est impossible (R-IT16)"
            )));
        }
        if en_attente > 0 {
            return Err(EntDbError::ConflitDossierDetail(format!(
                "{en_attente} élève(s) en activité ou en retard annoncé : la clôture attend leur retour (R-IT16)"
            )));
        }
        let mut cas_ouverts = Vec::new();
        for (eleve, nature) in a_ouvrir {
            if let Some(cas) = self
                .ouvrir_cas_tx(&mut tx, etab, actor, Some(appel.id), eleve, appel.nuit, nature, maintenant)
                .await?
            {
                cas_ouverts.push(cas);
            }
        }
        sqlx::query(
            "UPDATE ent_internat_appel SET etat = 'clos', clos_le = $3, clos_par_user_id = $4, nb_ecarts = $5, \
                    modifie_le = now() WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(appel_id)
        .bind(maintenant)
        .bind(actor)
        .bind(ecarts)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_APPEL_CLOS,
            AggregateKind::Internat,
            appel_id,
            None,
            json!({ "nuit": appel.nuit.to_string(), "type": appel.type_appel, "nb_ecarts": ecarts, "cas_ouverts": cas_ouverts.len() }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let appel = appel_tx(&mut tx, etab, appel_id, "").await?;
        tx.commit().await.map_err(db)?;
        Ok(ClotureResultatDto {
            appel: appel.dto(),
            cas_ouverts,
        })
    }

    /// INT-137 : validation (avis gradués R-IT34). Exige un appel clos.
    pub async fn valider_appel_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        appel_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<ValidationResultatDto, EntDbError> {
        let portee = self
            .nuit_portee(etab, actor, role, &[Action::InternatAppelValider])
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let appel = appel_tx(&mut tx, etab, appel_id, "FOR UPDATE").await?;
        if appel.etat == "valide" {
            return Err(conflit("l'appel est déjà validé"));
        }
        if appel.etat != "clos" {
            return Err(conflit("clore l'appel avant de le valider"));
        }
        if let Some(set) = &portee {
            let figes = attendu_fige_tx(&mut tx, etab, appel_id).await?;
            if figes.iter().any(|l| !set.contains(&l.eleve_id)) {
                return Err(EntDbError::Forbidden);
            }
        }
        sqlx::query(
            "UPDATE ent_internat_appel SET etat = 'valide', valide_le = $3, valide_par_user_id = $4, modifie_le = now() \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(appel_id)
        .bind(maintenant)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let nb_ecarts = appel.nb_ecarts.unwrap_or(0);
        let cas_ouverts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_internat_cas WHERE etablissement_id = $1 AND appel_id = $2 AND issue IS NULL",
        )
        .bind(etab)
        .bind(appel_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let urgence = niveau_avis(nb_ecarts, cas_ouverts as i32);
        publier(
            &mut tx,
            etab,
            types::INTERNAT_APPEL_VALIDE,
            AggregateKind::Internat,
            appel_id,
            None,
            json!({
                "nuit": appel.nuit.to_string(), "type": appel.type_appel, "urgence": urgence,
                "nb_ecarts": nb_ecarts, "cas_ouverts": cas_ouverts
            }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let appel = appel_tx(&mut tx, etab, appel_id, "").await?;
        tx.commit().await.map_err(db)?;
        Ok(ValidationResultatDto {
            appel: appel.dto(),
            urgence: urgence.into(),
            nb_ecarts,
            cas_ouverts: cas_ouverts as i32,
        })
    }
}
