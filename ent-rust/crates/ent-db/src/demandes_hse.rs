//! Socle N1, lot N1-3 : **HSE** (heures supplémentaires effectives), nature `rh.hse` (§17 de
//! `docs/socles/N1_DEMANDES.md`, **§38 et D-061 qui l'emportent**).
//!
//! * **En heures, jamais en euros** (D-061) : tout se compte en quarts d'heure ; aucun montant, aucun taux (R-DM44).
//!   L'application académique calcule les euros ; ScolIA lui remet un **export en heures par type d'heure**.
//! * **Circuit** : le professeur demande (heures **faites ou prévues**) → la **direction valide**, rien d'autre. Pas
//!   d'attestation séparée du service fait : l'acceptation vaut confiance. L'attestation reste un **réglage
//!   facultatif, désactivé par défaut** (R-DM50 quand il est activé).
//! * **Heures transmises** : à la validation, pour les séances faites, puis à la déclaration ou à la confirmation de
//!   chaque séance ensuite — par le port [`PortHeures`] (socle 15) s'il existe, sinon par l'**export de secours**
//!   (CSV mensuel, tracé). Jamais deux fois (R-DM42).
//!
//! Défauts du registre fermés ici (NJ-91, règle 14 de `PILOTAGE.md`) :
//!
//! | Défaut | Garde-fou |
//! |---|---|
//! | adresse nominative codée en dur comme destinataire | aucune adresse dans ce module (`t_nj91_aucun_destinataire_code`) : la décision publie une enveloppe neutre |
//! | image de signature apposée automatiquement | aucune image ni signature : la décision est un fait daté et nominatif de la direction |
//! | acceptation = heures à payer sans contrôle | décider exige un décideur autre que le bénéficiaire et que le déposant (R-DM40), l'enveloppe dépassée exige un motif ; l'attestation est un réglage (`t_nj91_attestation_facultative_desactivee_par_defaut`) |
//! | déposant sans trace | déposant stocké et visible (`t_nj91_deposant_trace`) ; bénéficiaire prévenu et confirmant (R-DM43) |
//!
//! Le jour métier est celui de l'établissement (`ent_aujourdhui_etab`), jamais la date du serveur.

use crate::demandes::{
    ajouter_fait, charger_pour_acces, publier_enveloppe, ContexteVolet, DepotDemande, EnveloppeNeutre, EtatDemande, FaitNouveau,
    Lien, QuotasDemandes, RefusDemande, SourcesDemandes, Valeur, ValeurSaisie, Volet,
};
use crate::demandes_files::jour_metier;
use crate::service::PgEnt;
use crate::tenant_tx::TenantTx;
use chrono::{Datelike, NaiveDate, NaiveTime};
use ent_auth::{Action, Role};
use ent_events::types;
use serde::Serialize;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

/// Nature déclarée.
pub const NATURE_HSE: &str = "rh.hse";

/// Motifs de la demande (liste fermée, Q-DM5 : à confirmer avec Florian).
pub const MOTIFS: &[&str] =
    &["soutien", "atelier", "accompagnement_sortie", "stage_reussite", "preparation_concours", "tutorat", "autre"];

/// Motifs d'une décision partielle ou d'un refus (codes, jamais de texte libre dans les faits).
pub const MOTIFS_DECISION: &[&str] = &["budget", "pas_besoin", "hors_perimetre", "volume_excessif", "autre"];

/// Type d'heure de l'export (l'application académique en déduit le tarif ; ScolIA ne calcule aucun montant).
pub fn type_heure_du_motif(motif: &str) -> &'static str {
    match motif {
        "soutien" | "atelier" | "stage_reussite" | "tutorat" => "hse_enseignement",
        "accompagnement_sortie" => "hse_accompagnement",
        _ => "hse_autre",
    }
}

/// « 6 h 15 » en décimal d'heures à deux chiffres (`6.25`) : calcul entier, jamais un flottant.
pub fn heures_decimales(quarts: i64) -> String {
    let signe = if quarts < 0 { "-" } else { "" };
    let q = quarts.abs();
    format!("{signe}{}.{:02}", q / 4, (q % 4) * 25)
}

// ───────────────────────── Port vers le socle 15 ─────────────────────────

/// Une heure à constater au socle 15 (§17.6). L'origine `(demande, séance)` rend l'appel **idempotent** côté 15.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeureAConstater {
    pub agent_id: Uuid,
    pub jour: NaiveDate,
    pub quarts: i32,
    pub type_heure: &'static str,
    pub demande_id: Uuid,
    pub seance_id: Uuid,
}

/// Ce que N1 demande au **socle 15** : constater une heure. Instantané sans accès base, comme les autres ports.
/// `None` = le socle 15 ne peut pas la recevoir : N1 garde l'heure pour l'**export de secours** (DEM-282).
pub trait PortHeures: Send + Sync {
    fn constater(&self, etab: Uuid, h: &HeureAConstater) -> Option<Uuid>;
}

/// Le socle 15 n'a pas livré `ent_absp_heure` : aucune heure n'est poussée, tout passe par l'export de secours.
pub struct HeuresAbsentes;

impl PortHeures for HeuresAbsentes {
    fn constater(&self, _: Uuid, _: &HeureAConstater) -> Option<Uuid> {
        None
    }
}

// ───────────────────────── Saisies ─────────────────────────

/// Une séance saisie : faite (déjà réalisée) ou prévue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeanceSaisie {
    pub jour: NaiveDate,
    pub debut: Option<NaiveTime>,
    /// Durée en quarts d'heure (1 à 48 : une séance dure au plus 12 h).
    pub duree_quarts: i32,
    pub fait: bool,
    pub lieu: Option<String>,
}

/// Le volet HSE écrit avec la demande.
#[derive(Debug, Clone)]
pub struct VoletHse {
    motif: String,
    volume_quarts: i32,
    periode_du: NaiveDate,
    periode_au: NaiveDate,
    lieu: Option<String>,
    classes: Vec<Uuid>,
    groupes: Vec<Uuid>,
    seances: Vec<SeanceSaisie>,
}

/// Dépôt d'une demande d'HSE. **Aucun champ d'identité** : le bénéficiaire est la personne de la session, ou la
/// personne désignée par un déposant habilité (R-DM43 : elle confirmera ou retirera).
#[derive(Debug, Clone)]
pub struct DepotHse {
    pub motif: String,
    pub objet: String,
    pub precisions: Option<String>,
    pub volume_quarts: i32,
    pub periode_du: NaiveDate,
    pub periode_au: NaiveDate,
    pub lieu: Option<String>,
    pub classes: Vec<Uuid>,
    pub groupes: Vec<Uuid>,
    pub seances: Vec<SeanceSaisie>,
    pub pour_le_compte_de: Option<Uuid>,
}

impl DepotHse {
    /// Contrôle la saisie (sans base) et la traduit en demande générique, volet compris. Sert aussi à calculer le
    /// routage (`sources_demandes_for_actor`) avant le dépôt.
    pub fn vers_depot_demande(&self) -> Result<DepotDemande, RefusDemande> {
        let d = self.clone();
        if !MOTIFS.contains(&d.motif.as_str()) {
            return Err(RefusDemande::Invalide("Le motif choisi n'existe pas."));
        }
        if !(1..=1600).contains(&d.volume_quarts) {
            return Err(RefusDemande::Invalide("Le volume demandé doit être de 15 minutes à 400 heures."));
        }
        if d.periode_au < d.periode_du || (d.periode_au - d.periode_du).num_days() > 366 {
            return Err(RefusDemande::Invalide("La période est incohérente."));
        }
        if d.seances.len() > 60 || d.classes.len() > 20 || d.groupes.len() > 20 {
            return Err(RefusDemande::Invalide("Trop de séances ou de classes pour une seule demande."));
        }
        let mut total = 0_i64;
        for s in &d.seances {
            valider_seance(s)?;
            total += i64::from(s.duree_quarts);
        }
        if total > i64::from(d.volume_quarts) {
            return Err(RefusDemande::Invalide("Les séances dépassent le volume demandé."));
        }
        let lieu = texte_court(&d.lieu, 120)?;
        let mut valeurs = vec![
            ValeurSaisie { champ: "motif".into(), rang: 0, valeur: Valeur::Code(d.motif.clone()) },
            ValeurSaisie { champ: "volume_quarts".into(), rang: 0, valeur: Valeur::Entier(i64::from(d.volume_quarts)) },
            ValeurSaisie { champ: "periode_du".into(), rang: 0, valeur: Valeur::Date(d.periode_du) },
            ValeurSaisie { champ: "periode_au".into(), rang: 0, valeur: Valeur::Date(d.periode_au) },
        ];
        if let Some(l) = &lieu {
            valeurs.push(ValeurSaisie { champ: "lieu".into(), rang: 0, valeur: Valeur::Texte(l.clone()) });
        }
        let volet = VoletHse {
            motif: d.motif,
            volume_quarts: d.volume_quarts,
            periode_du: d.periode_du,
            periode_au: d.periode_au,
            lieu,
            classes: d.classes,
            groupes: d.groupes,
            seances: d.seances,
        };
        let depot = DepotDemande {
            nature_code: NATURE_HSE.into(),
            objet: d.objet,
            texte: d.precisions,
            valeurs,
            urgence: None,
            eleve_id: None,
            pour_le_compte_de: d.pour_le_compte_de,
            canal: None,
            volet: Some(Volet::Hse(volet)),
        };
        Ok(depot)
    }
}

/// Décision de la direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionHse {
    /// Accorde le volume demandé (ou celui de l'avenant).
    Accorder,
    /// Accorde moins que demandé ; motif codé obligatoire.
    AccorderEnPartie { volume_quarts: i32, motif: String },
    /// Refuse ; motif codé obligatoire.
    Refuser { motif: String },
}

/// Ce qui est demandé en plus d'une décision : confirmer un dépassement d'enveloppe, avec son motif (conf. 2).
#[derive(Debug, Clone, Default)]
pub struct OptionsDecision {
    pub confirmer_depassement: bool,
    pub motif_depassement: Option<String>,
}

// ───────────────────────── Sorties ─────────────────────────

/// Résultat d'une décision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResultatDecisionHse {
    pub etat: EtatDemande,
    pub decision: String,
    pub volume_accorde_quarts: i32,
    pub seances_transmises: u32,
    /// `port_15` ou `export_secours` quand des heures ont été transmises à cette occasion.
    pub transmission: Option<&'static str>,
    pub depassement_enveloppe: bool,
}

/// Résultat d'une déclaration ou d'une confirmation de séance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResultatSeance {
    pub seance_id: Uuid,
    pub transmise: bool,
    /// Phrases fixes : l'enseignant était déclaré absent ce jour-là (CDM-43). Jamais un blocage, jamais un motif.
    pub avertissements: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SeanceHse {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub debut: Option<NaiveTime>,
    pub duree_quarts: i32,
    pub retenu_quarts: Option<i32>,
    pub forfait: bool,
    pub etat: String,
    pub transmission: String,
    pub fondement: Option<String>,
    pub echantillon: bool,
}

/// Le détail d'une demande d'HSE, **sans** coordonnée ni montant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DetailHse {
    pub demande_id: Uuid,
    pub numero: String,
    pub statut: String,
    pub motif: String,
    pub type_heure: String,
    pub volume_demande_quarts: i32,
    pub volume_accorde_quarts: Option<i32>,
    pub periode_du: NaiveDate,
    pub periode_au: NaiveDate,
    pub decision: Option<String>,
    pub confirmation_requise: bool,
    pub confirmee: bool,
    pub avenants: i16,
    pub avenant_en_attente: bool,
    pub deposee_pour_compte: bool,
    pub version: i32,
    pub seances: Vec<SeanceHse>,
}

/// Une ligne de « mes HSE » (DEM-279) ou de la file à décider (DEM-262).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LigneHse {
    pub demande_id: Uuid,
    pub numero: String,
    pub statut: String,
    pub motif: String,
    pub volume_demande_quarts: i32,
    pub volume_accorde_quarts: Option<i32>,
    pub fait_quarts: i64,
    pub transmis_quarts: i64,
    pub decision: Option<String>,
    pub periode_au: NaiveDate,
}

/// Répartition des HSE accordées par agent sur l'année (DEM-280, DEM-377) : **jamais visible des collègues**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepartitionHse {
    pub agent_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub demandes: i64,
    pub accorde_quarts: i64,
    pub fait_quarts: i64,
}

/// L'enveloppe d'un site (ou de l'établissement) pour l'année, en quarts d'heure (DEM-263, DEM-278).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnveloppeHse {
    pub annee_id: Uuid,
    pub site_id: Option<Uuid>,
    /// `false` : aucune enveloppe saisie, donc aucune alerte (l'établissement n'est pas obligé d'en tenir une).
    pub saisie: bool,
    pub dotation_quarts: i64,
    pub abondement_quarts: i64,
    pub retrait_quarts: i64,
    pub prevision_quarts: i64,
    /// Dotation + abondements − retraits ; à défaut de dotation, la prévision (DEM-376).
    pub total_quarts: i64,
    /// Accordé, sous déduction du non réalisé des périodes closes (rendu à l'enveloppe, DEM-276).
    pub engage_quarts: i64,
    pub fait_quarts: i64,
    pub restant_quarts: i64,
    pub par_motif: Vec<(String, i64)>,
}

/// Réglage de l'établissement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ReglageHse {
    pub attestation_service_fait: bool,
    pub echantillon_pct: i16,
}

impl Default for ReglageHse {
    fn default() -> Self {
        Self { attestation_service_fait: false, echantillon_pct: 10 }
    }
}

/// Un export de secours (DEM-282) : le CSV et le lot qui le trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportHse {
    pub lot_id: Uuid,
    pub mois: NaiveDate,
    pub nb_seances: i32,
    pub total_quarts: i32,
    pub csv: String,
}

/// Rapport de l'alerte de fin de période : un nombre, aucun nom.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RapportAlertesHse {
    pub signalees: u32,
}

// ───────────────────────── Aides ─────────────────────────

pub(crate) fn texte_court(t: &Option<String>, max: usize) -> Result<Option<String>, RefusDemande> {
    match t.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) if s.chars().count() > max || s.contains('\0') => {
            Err(RefusDemande::Invalide("Un des textes saisis est trop long."))
        }
        Some(s) => Ok(Some(s.to_string())),
    }
}

fn valider_seance(s: &SeanceSaisie) -> Result<(), RefusDemande> {
    if !(1..=48).contains(&s.duree_quarts) {
        return Err(RefusDemande::Invalide("La durée d'une séance est de 15 minutes à 12 heures."));
    }
    texte_court(&s.lieu, 120)?;
    Ok(())
}

/// Part déterministe de l'échantillon de contrôle (R-DM50, SN1-15) : dérivée de l'identifiant de la séance, donc
/// rejouable et non manipulable par la personne qui attestée.
pub fn dans_echantillon(seance: Uuid, pct: i16) -> bool {
    let b = seance.as_bytes();
    let n = u32::from(b[0]) | (u32::from(b[1]) << 8) | (u32::from(b[2]) << 16);
    (n % 100) < u32::try_from(pct.max(0)).unwrap_or(0)
}

async fn reglage(conn: &mut PgConnection, etab: Uuid) -> Result<ReglageHse, RefusDemande> {
    let r: Option<(bool, i16)> = sqlx::query_as(
        "SELECT attestation_service_fait, echantillon_pct FROM ent_dem_hse_reglage WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(conn)
    .await?;
    Ok(r.map(|(a, e)| ReglageHse { attestation_service_fait: a, echantillon_pct: e }).unwrap_or_default())
}

#[derive(Debug, FromRow)]
struct LigneHseDb {
    annee_id: Uuid,
    site_id: Option<Uuid>,
    beneficiaire_agent_id: Uuid,
    beneficiaire_personne_id: Uuid,
    motif: String,
    type_heure: String,
    volume_demande_quarts: i32,
    volume_accorde_quarts: Option<i32>,
    periode_du: NaiveDate,
    periode_au: NaiveDate,
    confirmation_requise: bool,
    confirmee: bool,
    decision: Option<String>,
    avenants: i16,
    avenant_volume_quarts: Option<i32>,
    avenant_periode_au: Option<NaiveDate>,
}

async fn charger_hse(conn: &mut PgConnection, etab: Uuid, demande: Uuid, verrou: bool) -> Result<LigneHseDb, RefusDemande> {
    let sql = format!(
        "SELECT h.annee_id, h.site_id, h.beneficiaire_agent_id, a.personne_id AS beneficiaire_personne_id, h.motif, h.type_heure,
                h.volume_demande_quarts, h.volume_accorde_quarts, h.periode_du, h.periode_au, h.confirmation_requise,
                (h.confirmee_le IS NOT NULL) AS confirmee, h.decision, h.avenants, h.avenant_volume_quarts,
                h.avenant_periode_au
           FROM ent_dem_hse h
           JOIN ent_agent a ON a.etablissement_id = h.etablissement_id AND a.id = h.beneficiaire_agent_id
          WHERE h.etablissement_id = $1 AND h.demande_id = $2{}",
        if verrou { " FOR UPDATE OF h" } else { "" }
    );
    let l: Option<LigneHseDb> = sqlx::query_as(&sql).bind(etab).bind(demande).fetch_optional(conn).await?;
    l.ok_or(RefusDemande::Introuvable)
}

fn refus_geste_hse(a: &crate::demandes::Acces) -> Result<(), RefusDemande> {
    if a.ligne.nature_code != NATURE_HSE {
        // Une demande d'une autre nature n'existe pas pour ce geste.
        return Err(RefusDemande::Introuvable);
    }
    Ok(())
}

#[derive(Debug, FromRow)]
struct SeanceDb {
    id: Uuid,
    jour: NaiveDate,
    duree_quarts: i32,
    retenu_quarts: Option<i32>,
    etat: String,
    transmission: String,
}

async fn seances_de(conn: &mut PgConnection, etab: Uuid, demande: Uuid, verrou: bool) -> Result<Vec<SeanceDb>, RefusDemande> {
    let sql = format!(
        "SELECT id, jour, duree_quarts, retenu_quarts, etat, transmission FROM ent_dem_hse_seance
          WHERE etablissement_id = $1 AND demande_id = $2 AND etat <> 'annulee'
          ORDER BY jour, debut NULLS LAST, cree_le, id{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    Ok(sqlx::query_as(&sql).bind(etab).bind(demande).fetch_all(conn).await?)
}

/// Fin de l'année scolaire de la demande (CDM-46 : aucune séance après le dernier jour de l'année d'origine).
async fn fin_annee(conn: &mut PgConnection, etab: Uuid, annee: Uuid) -> Result<(NaiveDate, NaiveDate), RefusDemande> {
    let r: Option<(NaiveDate, NaiveDate)> =
        sqlx::query_as("SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(annee)
            .fetch_optional(conn)
            .await?;
    r.ok_or(RefusDemande::Indisponible)
}

fn verifier_periode(jour_seance: NaiveDate, du: NaiveDate, au: NaiveDate) -> Result<(), RefusDemande> {
    if jour_seance < du || jour_seance > au {
        return Err(RefusDemande::Invalide("La séance est hors de la période demandée."));
    }
    Ok(())
}

// ───────────────────────── Dépôt (volet écrit avec la demande) ─────────────────────────

/// Écrit le volet dans la transaction du dépôt (appelé par `deposer_demande_avec_quotas`). Tout ce qui dépend de la
/// base — bénéficiaire, année, classes, jour — est contrôlé ici ; tout ce qui n'en dépend pas l'a été avant.
pub(crate) async fn ecrire_volet(conn: &mut PgConnection, ctx: &ContexteVolet, h: &VoletHse) -> Result<(), RefusDemande> {
    // Le bénéficiaire est un AGENT (socle 6) : jamais saisi comme texte (NJ-91).
    let agent: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM ent_agent WHERE etablissement_id = $1 AND personne_id = $2 ORDER BY cree_le LIMIT 1")
            .bind(ctx.etab)
            .bind(ctx.demandeur)
            .fetch_optional(&mut *conn)
            .await?;
    let Some((agent_id,)) = agent else {
        return Err(RefusDemande::Invalide("Seul un personnel peut recevoir des heures supplémentaires."));
    };
    let (debut_annee, fin_annee) = fin_annee(&mut *conn, ctx.etab, ctx.annee_id).await?;
    if h.periode_du < debut_annee || h.periode_au > fin_annee {
        return Err(RefusDemande::Invalide("La période doit tenir dans l'année scolaire en cours."));
    }
    for s in &h.seances {
        verifier_periode(s.jour, h.periode_du, h.periode_au)?;
        if s.fait && s.jour > ctx.jour {
            return Err(RefusDemande::Invalide("Une séance à venir ne peut pas être déclarée faite."));
        }
        if !s.fait && s.jour < ctx.jour {
            return Err(RefusDemande::Invalide("Une séance passée se déclare faite."));
        }
    }
    if !h.classes.is_empty() {
        let (n,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM ent_classe WHERE etablissement_id = $1 AND id = ANY($2)")
                .bind(ctx.etab)
                .bind(&h.classes)
                .fetch_one(&mut *conn)
                .await?;
        if usize::try_from(n).ok() != Some(h.classes.len()) {
            return Err(RefusDemande::Invalide("Une des classes choisies est introuvable."));
        }
    }
    if !h.groupes.is_empty() {
        let (n,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM ent_groupe WHERE etablissement_id = $1 AND id = ANY($2)")
                .bind(ctx.etab)
                .bind(&h.groupes)
                .fetch_one(&mut *conn)
                .await?;
        if usize::try_from(n).ok() != Some(h.groupes.len()) {
            return Err(RefusDemande::Invalide("Un des groupes choisis est introuvable."));
        }
    }
    let pour_compte = ctx.deposant != ctx.demandeur;
    sqlx::query(
        "INSERT INTO ent_dem_hse (etablissement_id, demande_id, annee_id, site_id, beneficiaire_agent_id, motif, type_heure,
                volume_demande_quarts, periode_du, periode_au, lieu_libelle, confirmation_requise)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(ctx.etab)
    .bind(ctx.demande_id)
    .bind(ctx.annee_id)
    .bind(ctx.site_id)
    .bind(agent_id)
    .bind(&h.motif)
    .bind(type_heure_du_motif(&h.motif))
    .bind(h.volume_quarts)
    .bind(h.periode_du)
    .bind(h.periode_au)
    .bind(h.lieu.as_deref())
    .bind(pour_compte)
    .execute(&mut *conn)
    .await?;
    for c in &h.classes {
        sqlx::query("INSERT INTO ent_dem_hse_public (id, etablissement_id, demande_id, classe_id) VALUES ($1, $2, $3, $4)")
            .bind(Uuid::new_v4())
            .bind(ctx.etab)
            .bind(ctx.demande_id)
            .bind(c)
            .execute(&mut *conn)
            .await?;
    }
    for g in &h.groupes {
        sqlx::query("INSERT INTO ent_dem_hse_public (id, etablissement_id, demande_id, groupe_id) VALUES ($1, $2, $3, $4)")
            .bind(Uuid::new_v4())
            .bind(ctx.etab)
            .bind(ctx.demande_id)
            .bind(g)
            .execute(&mut *conn)
            .await?;
    }
    for s in &h.seances {
        sqlx::query(
            "INSERT INTO ent_dem_hse_seance (id, etablissement_id, demande_id, jour, debut, duree_quarts, etat, lieu_libelle,
                    declaree_le, declaree_par_personne_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CASE WHEN $9 THEN now() END, CASE WHEN $9 THEN $10::uuid END)",
        )
        .bind(Uuid::new_v4())
        .bind(ctx.etab)
        .bind(ctx.demande_id)
        .bind(s.jour)
        .bind(s.debut)
        .bind(s.duree_quarts)
        .bind(if s.fait { "faite" } else { "prevue" })
        .bind(s.lieu.as_deref())
        .bind(s.fait)
        .bind(ctx.deposant)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

impl PgEnt {
    /// Dépose une demande d'HSE au nom de la personne de la session (ou, pour un déposant habilité, d'un collègue qui
    /// confirmera : R-DM43). La demande, son volet, ses séances et son public s'écrivent **dans une seule
    /// transaction** : jamais une demande sans volet. `sources` se construit avec
    /// [`DepotHse::vers_depot_demande`] (le routage dépend du motif).
    pub async fn deposer_hse_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        d: DepotHse,
    ) -> Result<EtatDemande, RefusDemande> {
        let depot = d.vers_depot_demande()?;
        self.deposer_demande_for_actor(etab, user_id, role, sources, depot).await
    }

    /// Le bénéficiaire confirme une demande déposée en son nom (R-DM43, DEM-261). Sans effet s'il l'a déjà fait.
    pub async fn confirmer_hse_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
    ) -> Result<(), RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        refus_geste_hse(&a)?;
        let h = charger_hse(&mut tx, etab, id, true).await?;
        if a.moi != h.beneficiaire_personne_id {
            return Err(RefusDemande::Interdit);
        }
        if h.confirmation_requise && !h.confirmee {
            sqlx::query("UPDATE ent_dem_hse SET confirmee_le = now() WHERE etablissement_id = $1 AND demande_id = $2")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            let mut f = FaitNouveau::nouveau(id, "hse_confirmee_par_beneficiaire");
            f.auteur_personne = Some(a.moi);
            ajouter_fait(&mut tx, etab, f).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Décision de la direction : accorder, accorder en partie, refuser (DEM-264 à DEM-266). **Personne ne décide sa
    /// propre demande** (R-DM40) ni celle qu'il a déposée. Au-delà de l'enveloppe : avertissement, confirmé et motivé.
    #[allow(clippy::too_many_arguments)]
    pub async fn decider_hse_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        heures: &dyn PortHeures,
        id: Uuid,
        version_attendue: i32,
        decision: DecisionHse,
        options: OptionsDecision,
    ) -> Result<ResultatDecisionHse, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        refus_geste_hse(&a)?;
        // Droit : membre de la file de direction ET action de décision.
        if a.lien != Lien::Traitant
            || !self.a_action_etablissement(etab, user_id, role.as_str(), Action::DemandesHseDecider).await?
        {
            return Err(RefusDemande::Interdit);
        }
        let h = charger_hse(&mut tx, etab, id, true).await?;
        // R-DM40 : ni le bénéficiaire, ni le demandeur, ni le déposant ne décident.
        if a.moi == h.beneficiaire_personne_id
            || a.ligne.demandeur_personne_id == Some(a.moi)
            || a.ligne.deposant_personne_id == Some(a.moi)
        {
            return Err(RefusDemande::Interdit);
        }
        if a.ligne.version != version_attendue {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        let avenant = h.avenant_volume_quarts.is_some();
        if h.decision.is_some() && !avenant {
            return Err(RefusDemande::Conflit("Cette demande a déjà reçu une décision."));
        }
        if a.ligne.statut == "annulee" {
            return Err(RefusDemande::Conflit("Cette demande est annulée."));
        }
        if h.confirmation_requise && !h.confirmee {
            return Err(RefusDemande::Conflit("Le collègue doit d'abord confirmer cette demande, ou la retirer."));
        }
        let demande_quarts = h.avenant_volume_quarts.unwrap_or(h.volume_demande_quarts);
        let (code, volume, motif): (&'static str, i32, Option<String>) = match &decision {
            DecisionHse::Accorder => ("accordee", demande_quarts, None),
            DecisionHse::AccorderEnPartie { volume_quarts, motif } => {
                if *volume_quarts < 1 || *volume_quarts >= demande_quarts {
                    return Err(RefusDemande::Invalide("Un accord partiel est inférieur au volume demandé."));
                }
                if !MOTIFS_DECISION.contains(&motif.as_str()) {
                    return Err(RefusDemande::Invalide("Le motif de la décision est obligatoire."));
                }
                ("accordee_partiellement", *volume_quarts, Some(motif.clone()))
            }
            DecisionHse::Refuser { motif } => {
                if !MOTIFS_DECISION.contains(&motif.as_str()) {
                    return Err(RefusDemande::Invalide("Le motif du refus est obligatoire."));
                }
                ("refusee", 0, Some(motif.clone()))
            }
        };
        // Enveloppe : avertissement, jamais un blocage ; un dépassement se confirme avec un motif.
        let jour = jour_metier(&mut tx, etab).await?;
        let mut depassement = false;
        if volume > 0 {
            let env = calculer_enveloppe(&mut tx, etab, h.annee_id, h.site_id, jour).await?;
            if env.saisie {
                // Ce que cette demande compte déjà dans l'engagé est retiré (avenant) avant de comparer.
                let deja = i64::from(h.volume_accorde_quarts.unwrap_or(0)).min(env.engage_quarts);
                if i64::from(volume) > env.restant_quarts + deja {
                    depassement = true;
                    let motif_depassement = options.motif_depassement.as_deref().map(str::trim).filter(|m| !m.is_empty());
                    if !options.confirmer_depassement || motif_depassement.is_none() {
                        return Err(RefusDemande::Conflit(
                            "L'enveloppe du site est dépassée : confirmez la décision en indiquant un motif.",
                        ));
                    }
                }
            }
        }
        let motif_fait = if depassement { Some("depassement_enveloppe".to_string()) } else { motif.clone() };

        // Allocation des séances (ordre chronologique) : la quantité retenue est celle qui sera transmise.
        let seances = seances_de(&mut tx, etab, id, true).await?;
        let deja_transmis: i64 =
            seances.iter().filter(|s| s.transmission != "aucune").map(|s| i64::from(s.retenu_quarts.unwrap_or(0))).sum();
        let mut restant = (i64::from(volume) - deja_transmis).max(0);
        if seances.is_empty() && volume > 0 {
            // Aucune séance détaillée : une séance « forfait » à la fin de la période porte tout le volume.
            let faite = h.periode_au <= jour;
            sqlx::query(
                "INSERT INTO ent_dem_hse_seance (id, etablissement_id, demande_id, jour, duree_quarts, retenu_quarts, forfait,
                        etat, declaree_le, declaree_par_personne_id)
                 VALUES ($1, $2, $3, $4, $5, $5, true, $6, CASE WHEN $7 THEN now() END, CASE WHEN $7 THEN $8::uuid END)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(h.periode_au)
            .bind(volume)
            .bind(if faite { "faite" } else { "prevue" })
            .bind(faite)
            .bind(h.beneficiaire_personne_id)
            .execute(&mut *tx)
            .await?;
        } else {
            for s in seances.iter().filter(|s| s.transmission == "aucune") {
                let retenu = if code == "refusee" { 0 } else { i64::from(s.duree_quarts).min(restant) };
                restant -= retenu;
                sqlx::query(
                    "UPDATE ent_dem_hse_seance SET retenu_quarts = $4 WHERE etablissement_id = $1 AND demande_id = $2 AND id = $3",
                )
                .bind(etab)
                .bind(id)
                .bind(s.id)
                .bind(i32::try_from(retenu).unwrap_or(0))
                .execute(&mut *tx)
                .await?;
            }
        }

        let (rang,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_hse_decision WHERE etablissement_id = $1 AND demande_id = $2")
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        let decision_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_dem_hse_decision (id, etablissement_id, demande_id, rang, decision, volume_quarts, motif_code, avenant,
                    depassement_enveloppe, decide_par_personne_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(decision_id)
        .bind(etab)
        .bind(id)
        .bind(i16::try_from(rang).unwrap_or(i16::MAX))
        .bind(code)
        .bind(volume)
        .bind(motif_fait.as_deref())
        .bind(avenant)
        .bind(depassement)
        .bind(a.moi)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE ent_dem_hse
                SET decision = $3, volume_accorde_quarts = $4, decide_par_personne_id = $5, decide_le = now(),
                    avenants = avenants + CASE WHEN $6 THEN 1 ELSE 0 END,
                    periode_au = COALESCE(avenant_periode_au, periode_au),
                    avenant_volume_quarts = NULL, avenant_periode_au = NULL
              WHERE etablissement_id = $1 AND demande_id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(code)
        .bind(volume)
        .bind(a.moi)
        .bind(avenant)
        .execute(&mut *tx)
        .await?;

        // La demande se clôt sur la décision (D-061 : la validation de la direction, rien d'autre).
        let issue = code;
        let n = sqlx::query(
            "UPDATE ent_dem_demande
                SET statut = 'close', issue = $4, decision = $4, clos_le = now(), conservation_jusqu_au = NULL,
                    attribuee_a_agent_id = NULL, attribuee_le = NULL, version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(version_attendue)
        .bind(issue)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        sqlx::query("UPDATE ent_dem_echeance SET traitee_le = now() WHERE etablissement_id = $1 AND demande_id = $2 AND traitee_le IS NULL")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let mut f = FaitNouveau::nouveau(id, "hse_decidee");
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some(&a.ligne.statut);
        f.statut_apres = Some("close");
        f.motif = motif_fait.as_deref().or(Some(code));
        f.ref_id = Some(decision_id);
        ajouter_fait(&mut tx, etab, f).await?;
        let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, NATURE_HSE, "hse_decidee");
        publier_enveloppe(&mut tx, etab, types::DEMANDE_HSE_DECIDEE, &env, Some(user_id)).await?;

        // Heures transmises à la validation (séances faites), sauf si l'établissement attend une attestation.
        let mut transmises = 0_u32;
        let mut canal = None;
        if code != "refusee" && !reglage(&mut tx, etab).await?.attestation_service_fait {
            let (n, c) = transmettre_faites(&mut tx, etab, id, h.beneficiaire_agent_id, &h.type_heure, heures, a.moi).await?;
            transmises = n;
            canal = c;
        }
        tx.commit().await?;
        Ok(ResultatDecisionHse {
            etat: EtatDemande { id, numero: a.ligne.numero, statut: "close".into(), version: version_attendue + 1 },
            decision: code.to_string(),
            volume_accorde_quarts: volume,
            seances_transmises: transmises,
            transmission: canal,
            depassement_enveloppe: depassement,
        })
    }
}

// ───────────────────────── Transmission des heures (R-DM42) ─────────────────────────

/// Transmet les séances **faites** non encore transmises : au socle 15 par le port, sinon pour l'export de secours.
/// Idempotent : une séance transmise ne l'est jamais deux fois (la base l'interdit aussi).
async fn transmettre_faites(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    agent: Uuid,
    type_heure: &str,
    heures: &dyn PortHeures,
    auteur: Uuid,
) -> Result<(u32, Option<&'static str>), RefusDemande> {
    let a_transmettre: Vec<(Uuid, NaiveDate, i32)> = sqlx::query_as(
        "SELECT id, jour, retenu_quarts FROM ent_dem_hse_seance
          WHERE etablissement_id = $1 AND demande_id = $2 AND etat IN ('faite', 'attestee') AND transmission = 'aucune'
            AND COALESCE(retenu_quarts, 0) > 0
          ORDER BY jour, debut NULLS LAST, cree_le, id FOR UPDATE",
    )
    .bind(etab)
    .bind(demande)
    .fetch_all(&mut *conn)
    .await?;
    let type_heure_fixe: &'static str = match type_heure {
        "hse_enseignement" => "hse_enseignement",
        "hse_accompagnement" => "hse_accompagnement",
        _ => "hse_autre",
    };
    let (mut nombre, mut canal) = (0_u32, None);
    for (seance_id, jour, quarts) in a_transmettre {
        let hid = heures.constater(
            etab,
            &HeureAConstater { agent_id: agent, jour, quarts, type_heure: type_heure_fixe, demande_id: demande, seance_id },
        );
        let mode = if hid.is_some() { "port_15" } else { "export_secours" };
        sqlx::query(
            "UPDATE ent_dem_hse_seance SET transmission = $3, transmise_le = now(), heure_constatee_id = $4
              WHERE etablissement_id = $1 AND id = $2 AND transmission = 'aucune'",
        )
        .bind(etab)
        .bind(seance_id)
        .bind(mode)
        .bind(hid)
        .execute(&mut *conn)
        .await?;
        nombre += 1;
        canal = Some(mode);
    }
    if nombre > 0 {
        let mut f = FaitNouveau::nouveau(demande, "hse_heures_transmises");
        f.auteur_personne = Some(auteur);
        f.motif = canal;
        ajouter_fait(&mut *conn, etab, f).await?;
        let (numero, nature): (String, String) =
            sqlx::query_as("SELECT numero, nature_code FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(demande)
                .fetch_one(&mut *conn)
                .await?;
        let env = EnveloppeNeutre::nouvelle(demande, &numero, &nature, "hse_transmise");
        publier_enveloppe(&mut *conn, etab, types::DEMANDE_HSE_TRANSMISE, &env, None).await?;
    }
    Ok((nombre, canal))
}

// ───────────────────────── Séances ─────────────────────────

async fn avertissements_seance(
    conn: &mut PgConnection,
    etab: Uuid,
    agent: Uuid,
    jour: NaiveDate,
) -> Result<Vec<&'static str>, RefusDemande> {
    let absents = crate::demandes_prise::agents_absents(conn, etab, &[agent], jour).await?;
    let mut v = Vec::new();
    if absents.contains_key(&agent) {
        v.push("Vous étiez déclaré absent ce jour-là : la direction verra cette incohérence.");
    }
    Ok(v)
}

impl PgEnt {
    /// Déclare une séance réalisée (DEM-269) sur une demande accordée : la quantité ne dépasse jamais le volume accordé.
    #[allow(clippy::too_many_arguments)]
    pub async fn declarer_seance_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        heures: &dyn PortHeures,
        id: Uuid,
        seance: SeanceSaisie,
    ) -> Result<ResultatSeance, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        valider_seance(&seance)?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        refus_geste_hse(&a)?;
        let h = charger_hse(&mut tx, etab, id, true).await?;
        if a.moi != h.beneficiaire_personne_id {
            return Err(RefusDemande::Interdit);
        }
        let accorde = match (h.decision.as_deref(), h.volume_accorde_quarts) {
            (Some("accordee" | "accordee_partiellement"), Some(v)) => v,
            _ => return Err(RefusDemande::Conflit("Cette demande n'est pas encore accordée.")),
        };
        if a.ligne.statut == "annulee" {
            return Err(RefusDemande::Conflit("Cette demande est annulée."));
        }
        let jour = jour_metier(&mut tx, etab).await?;
        let (_, fin) = fin_annee(&mut tx, etab, h.annee_id).await?;
        if seance.jour > fin {
            // CDM-46 : une séance après la fin de l'année d'origine relève d'une nouvelle demande.
            return Err(RefusDemande::Invalide("Cette séance est après la fin de l'année scolaire : faites une nouvelle demande."));
        }
        verifier_periode(seance.jour, h.periode_du, h.periode_au)?;
        if seance.fait && seance.jour > jour {
            return Err(RefusDemande::Invalide("Une séance à venir ne peut pas être déclarée faite."));
        }
        if !seance.fait && seance.jour < jour {
            return Err(RefusDemande::Invalide("Une séance passée se déclare faite."));
        }
        let (deja,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(sum(COALESCE(retenu_quarts, duree_quarts)), 0)::bigint FROM ent_dem_hse_seance
              WHERE etablissement_id = $1 AND demande_id = $2 AND etat <> 'annulee'",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if deja + i64::from(seance.duree_quarts) > i64::from(accorde) {
            return Err(RefusDemande::Conflit("Le volume accordé est atteint : demandez un avenant."));
        }
        let seance_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_dem_hse_seance (id, etablissement_id, demande_id, jour, debut, duree_quarts, retenu_quarts, etat,
                    lieu_libelle, declaree_le, declaree_par_personne_id)
             VALUES ($1, $2, $3, $4, $5, $6, $6, $7, $8, CASE WHEN $9 THEN now() END, CASE WHEN $9 THEN $10::uuid END)",
        )
        .bind(seance_id)
        .bind(etab)
        .bind(id)
        .bind(seance.jour)
        .bind(seance.debut)
        .bind(seance.duree_quarts)
        .bind(if seance.fait { "faite" } else { "prevue" })
        .bind(seance.lieu.as_deref())
        .bind(seance.fait)
        .bind(a.moi)
        .execute(&mut *tx)
        .await?;
        let avertissements = if seance.fait { avertissements_seance(&mut tx, etab, h.beneficiaire_agent_id, seance.jour).await? } else { Vec::new() };
        if !avertissements.is_empty() {
            let mut f = FaitNouveau::nouveau(id, "hse_incoherence_absence");
            f.auteur_tache = Some("ordonnanceur_echeances");
            f.ref_id = Some(seance_id);
            ajouter_fait(&mut tx, etab, f).await?;
        }
        let mut transmise = false;
        if seance.fait && !reglage(&mut tx, etab).await?.attestation_service_fait {
            transmise = transmettre_faites(&mut tx, etab, id, h.beneficiaire_agent_id, &h.type_heure, heures, a.moi).await?.0 > 0;
        }
        tx.commit().await?;
        Ok(ResultatSeance { seance_id, transmise, avertissements })
    }

    /// Confirme d'un geste qu'une séance prévue a eu lieu (DEM-270) : elle devient « faite » puis est transmise.
    #[allow(clippy::too_many_arguments)]
    pub async fn confirmer_seance_faite_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        heures: &dyn PortHeures,
        id: Uuid,
        seance_id: Uuid,
    ) -> Result<ResultatSeance, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        refus_geste_hse(&a)?;
        let h = charger_hse(&mut tx, etab, id, true).await?;
        if a.moi != h.beneficiaire_personne_id {
            return Err(RefusDemande::Interdit);
        }
        if h.decision.is_none() || h.decision.as_deref() == Some("refusee") {
            return Err(RefusDemande::Conflit("Cette demande n'est pas accordée."));
        }
        let jour = jour_metier(&mut tx, etab).await?;
        let s: Option<(NaiveDate, String)> = sqlx::query_as(
            "SELECT jour, etat FROM ent_dem_hse_seance WHERE etablissement_id = $1 AND demande_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .bind(seance_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (jour_seance, etat) = s.ok_or(RefusDemande::Introuvable)?;
        if etat != "prevue" {
            return Err(RefusDemande::Conflit("Cette séance n'est pas une séance prévue."));
        }
        if jour_seance > jour {
            return Err(RefusDemande::Invalide("Une séance à venir ne peut pas être déclarée faite."));
        }
        sqlx::query(
            "UPDATE ent_dem_hse_seance SET etat = 'faite', declaree_le = now(), declaree_par_personne_id = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(seance_id)
        .bind(a.moi)
        .execute(&mut *tx)
        .await?;
        let avertissements = avertissements_seance(&mut tx, etab, h.beneficiaire_agent_id, jour_seance).await?;
        let mut transmise = false;
        if !reglage(&mut tx, etab).await?.attestation_service_fait {
            transmise = transmettre_faites(&mut tx, etab, id, h.beneficiaire_agent_id, &h.type_heure, heures, a.moi).await?.0 > 0;
        }
        tx.commit().await?;
        Ok(ResultatSeance { seance_id, transmise, avertissements })
    }

    /// Réglage facultatif de l'attestation du service fait (D-061 : désactivée par défaut) et de l'échantillon de
    /// contrôle. Tout changement est journalisé.
    pub async fn regler_hse_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        attestation_service_fait: bool,
        echantillon_pct: i16,
    ) -> Result<ReglageHse, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        if !(0..=100).contains(&echantillon_pct) {
            return Err(RefusDemande::Invalide("Le pourcentage de contrôle doit être compris entre 0 et 100."));
        }
        if !self.a_action_etablissement(etab, user_id, role, Action::DemandesParametrer).await? {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let avant = reglage(&mut tx, etab).await?;
        sqlx::query(
            "INSERT INTO ent_dem_hse_reglage (etablissement_id, attestation_service_fait, echantillon_pct, maj_par_user_id)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (etablissement_id) DO UPDATE
                SET attestation_service_fait = EXCLUDED.attestation_service_fait, echantillon_pct = EXCLUDED.echantillon_pct,
                    maj_le = now(), maj_par_user_id = EXCLUDED.maj_par_user_id",
        )
        .bind(etab)
        .bind(attestation_service_fait)
        .bind(echantillon_pct)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        let libelle = |r: &ReglageHse| format!("attestation={} echantillon={}", if r.attestation_service_fait { "oui" } else { "non" }, r.echantillon_pct);
        let apres = ReglageHse { attestation_service_fait, echantillon_pct };
        sqlx::query(
            "INSERT INTO ent_dem_reglage_journal (id, etablissement_id, sujet, cible, avant, apres, par_user_id)
             VALUES ($1, $2, 'hse', 'service_fait', $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(libelle(&avant))
        .bind(libelle(&apres))
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(apres)
    }

    /// Le réglage en vigueur (lecture ouverte à qui décide, attesté ou paramètre).
    pub async fn reglage_hse_for_actor(&self, etab: Uuid, user_id: Uuid, role: &str) -> Result<ReglageHse, RefusDemande> {
        let ok = self.a_action_etablissement(etab, user_id, role, Action::DemandesHseDecider).await?
            || self.a_action_etablissement(etab, user_id, role, Action::DemandesParametrer).await?
            || self.a_action_etablissement(etab, user_id, role, Action::DemandesHseAttester).await?;
        if !ok {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = reglage(&mut tx, etab).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Attestation du service fait (DEM-271) — **réglage facultatif, désactivé par défaut** (D-061). Fondée sur un
    /// indice (appel, tiers) ou sur la déclaration sur l'honneur de l'enseignant, celle-ci contrôlée par échantillon
    /// (R-DM50). Une attestation en masse sans indice exige une confirmation explicite.
    #[allow(clippy::too_many_arguments)]
    pub async fn attester_hse_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        heures: &dyn PortHeures,
        id: Uuid,
        seances: &[Uuid],
        fondement: &str,
        confirmer_masse: bool,
    ) -> Result<u32, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        if !role.is_staff() || !self.a_action_etablissement(etab, user_id, role.as_str(), Action::DemandesHseAttester).await? {
            return Err(RefusDemande::Introuvable);
        }
        if !matches!(fondement, "appel" | "tiers" | "declaration_honneur") {
            return Err(RefusDemande::Invalide("Le fondement de l'attestation est obligatoire."));
        }
        if seances.is_empty() || seances.len() > 200 {
            return Err(RefusDemande::Invalide("Choisissez les séances à attester."));
        }
        if fondement == "declaration_honneur" && seances.len() > 10 && !confirmer_masse {
            return Err(RefusDemande::Conflit(
                "Attester en masse sur simple déclaration exige une confirmation explicite.",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = crate::demandes::personne_du_user(&mut tx, etab, user_id).await.map_err(|_| RefusDemande::Introuvable)?;
        let rg = reglage(&mut tx, etab).await?;
        // Une demande d'une autre nature, ou d'un autre établissement, n'existe pas pour l'attestant.
        let h = charger_hse(&mut tx, etab, id, true).await?;
        if !rg.attestation_service_fait {
            return Err(RefusDemande::Conflit("L'attestation du service fait n'est pas activée dans cet établissement."));
        }
        // R-DM40 : on n'atteste pas ses propres heures.
        if moi == h.beneficiaire_personne_id {
            return Err(RefusDemande::Interdit);
        }
        let mut n = 0_u32;
        for s in seances {
            let echantillon = fondement == "declaration_honneur" && dans_echantillon(*s, rg.echantillon_pct);
            let r = sqlx::query(
                "UPDATE ent_dem_hse_seance
                    SET etat = 'attestee', attestee_par_personne_id = $4, attestee_le = now(), fondement = $5, echantillon = $6
                  WHERE etablissement_id = $1 AND demande_id = $2 AND id = $3 AND etat = 'faite' AND transmission = 'aucune'",
            )
            .bind(etab)
            .bind(id)
            .bind(s)
            .bind(moi)
            .bind(fondement)
            .bind(echantillon)
            .execute(&mut *tx)
            .await?;
            n += u32::try_from(r.rows_affected()).unwrap_or(0);
        }
        if n == 0 {
            return Err(RefusDemande::Conflit("Aucune de ces séances n'est à attester."));
        }
        let mut f = FaitNouveau::nouveau(id, "hse_attestee");
        f.auteur_personne = Some(moi);
        f.motif = Some(fondement);
        ajouter_fait(&mut tx, etab, f).await?;
        transmettre_faites(&mut tx, etab, id, h.beneficiaire_agent_id, &h.type_heure, heures, moi).await?;
        tx.commit().await?;
        Ok(n)
    }

    /// L'attestant conteste une séance déclarée : elle revient à l'enseignant avec un motif (DEM-272).
    pub async fn contester_seance_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        id: Uuid,
        seance_id: Uuid,
        motif: &str,
    ) -> Result<(), RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let motif = motif.trim();
        if motif.is_empty() || motif.chars().count() > 300 || motif.contains('\0') {
            return Err(RefusDemande::Invalide("Le motif de la contestation est obligatoire (300 caractères au plus)."));
        }
        if !self.a_action_etablissement(etab, user_id, role, Action::DemandesHseAttester).await? {
            return Err(RefusDemande::Introuvable);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = crate::demandes::personne_du_user(&mut tx, etab, user_id).await.map_err(|_| RefusDemande::Introuvable)?;
        let h = charger_hse(&mut tx, etab, id, true).await?;
        if moi == h.beneficiaire_personne_id {
            return Err(RefusDemande::Interdit);
        }
        let r = sqlx::query(
            "UPDATE ent_dem_hse_seance SET etat = 'contestee', contestee_motif = $4
              WHERE etablissement_id = $1 AND demande_id = $2 AND id = $3 AND etat = 'faite' AND transmission = 'aucune'",
        )
        .bind(etab)
        .bind(id)
        .bind(seance_id)
        .bind(motif)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() != 1 {
            return Err(RefusDemande::Conflit("Cette séance ne peut pas être contestée."));
        }
        let mut f = FaitNouveau::nouveau(id, "hse_seance_contestee");
        f.auteur_personne = Some(moi);
        f.ref_id = Some(seance_id);
        ajouter_fait(&mut tx, etab, f).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Demande d'avenant (DEM-274) : plus d'heures ou une période prolongée ; **même demande, nouvelle décision**. La
    /// décision précédente reste au registre, jamais réécrite.
    #[allow(clippy::too_many_arguments)]
    pub async fn demander_avenant_hse_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
        nouveau_volume_quarts: i32,
        nouvelle_fin: Option<NaiveDate>,
    ) -> Result<EtatDemande, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        refus_geste_hse(&a)?;
        let h = charger_hse(&mut tx, etab, id, true).await?;
        if a.moi != h.beneficiaire_personne_id {
            return Err(RefusDemande::Interdit);
        }
        if a.ligne.version != version_attendue {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        let accorde = match (h.decision.as_deref(), h.volume_accorde_quarts) {
            (Some("accordee" | "accordee_partiellement"), Some(v)) => v,
            _ => return Err(RefusDemande::Conflit("Un avenant suppose une demande accordée.")),
        };
        if h.avenant_volume_quarts.is_some() {
            return Err(RefusDemande::Conflit("Un avenant est déjà en attente de décision."));
        }
        if !(1..=1600).contains(&nouveau_volume_quarts) || nouveau_volume_quarts <= accorde {
            return Err(RefusDemande::Invalide("Un avenant demande plus d'heures que le volume accordé."));
        }
        let (_, fin) = fin_annee(&mut tx, etab, h.annee_id).await?;
        let nouvelle_fin = nouvelle_fin.unwrap_or(h.periode_au);
        if nouvelle_fin < h.periode_au || nouvelle_fin > fin {
            return Err(RefusDemande::Invalide("La nouvelle fin doit rester dans l'année scolaire."));
        }
        sqlx::query(
            "UPDATE ent_dem_hse SET avenant_volume_quarts = $3, avenant_periode_au = $4
              WHERE etablissement_id = $1 AND demande_id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(nouveau_volume_quarts)
        .bind(nouvelle_fin)
        .execute(&mut *tx)
        .await?;
        let n = sqlx::query(
            "UPDATE ent_dem_demande
                SET statut = 'rouverte', clos_le = NULL, issue = NULL, decision = NULL, conservation_jusqu_au = NULL,
                    attribuee_a_agent_id = NULL, attribuee_le = NULL, version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(version_attendue)
        .execute(&mut *tx)
        .await?;
        if n.rows_affected() != 1 {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        let mut f = FaitNouveau::nouveau(id, "hse_avenant_demande");
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some(&a.ligne.statut);
        f.statut_apres = Some("rouverte");
        ajouter_fait(&mut tx, etab, f).await?;
        let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, NATURE_HSE, "rouverte");
        publier_enveloppe(&mut tx, etab, types::DEMANDE_ROUVERTE, &env, Some(user_id)).await?;
        tx.commit().await?;
        Ok(EtatDemande { id, numero: a.ligne.numero, statut: "rouverte".into(), version: version_attendue + 1 })
    }

    /// Annule une demande d'HSE (DEM-275) : par le demandeur ou le déposant, par un décideur avec motif ; **tant
    /// qu'aucune heure n'a été transmise** (les heures déjà transmises restent dues).
    #[allow(clippy::too_many_arguments)]
    pub async fn annuler_hse_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
        motif: Option<&str>,
    ) -> Result<EtatDemande, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        refus_geste_hse(&a)?;
        let h = charger_hse(&mut tx, etab, id, true).await?;
        let est_decideur = a.lien == Lien::Traitant
            && a.moi != h.beneficiaire_personne_id
            && self.a_action_etablissement(etab, user_id, role.as_str(), Action::DemandesHseDecider).await?;
        let est_demandeur = matches!(a.lien, Lien::Demandeur | Lien::Deposant);
        if !est_demandeur && !est_decideur {
            return Err(RefusDemande::Interdit);
        }
        if est_decideur && !est_demandeur && motif.map(str::trim).is_none_or(str::is_empty) {
            return Err(RefusDemande::Invalide("Le motif de l'annulation est obligatoire."));
        }
        if a.ligne.version != version_attendue {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        if a.ligne.statut == "annulee" {
            return Err(RefusDemande::Conflit("Cette demande est déjà annulée."));
        }
        let (transmises,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_dem_hse_seance WHERE etablissement_id = $1 AND demande_id = $2 AND transmission <> 'aucune'",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if transmises > 0 {
            return Err(RefusDemande::Conflit("Des heures ont déjà été transmises : elles restent dues."));
        }
        sqlx::query("UPDATE ent_dem_hse_seance SET etat = 'annulee' WHERE etablissement_id = $1 AND demande_id = $2 AND etat <> 'annulee'")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let n = sqlx::query(
            "UPDATE ent_dem_demande
                SET statut = 'annulee', issue = 'annulee_par_demandeur', decision = NULL, clos_le = now(),
                    attribuee_a_agent_id = NULL, attribuee_le = NULL, version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(version_attendue)
        .execute(&mut *tx)
        .await?;
        if n.rows_affected() != 1 {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        sqlx::query("UPDATE ent_dem_echeance SET traitee_le = now() WHERE etablissement_id = $1 AND demande_id = $2 AND traitee_le IS NULL")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let mut f = FaitNouveau::nouveau(id, "annulee");
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some(&a.ligne.statut);
        f.statut_apres = Some("annulee");
        f.motif = Some(if est_demandeur { "annulee_par_demandeur" } else { "annulee_par_decideur" });
        ajouter_fait(&mut tx, etab, f).await?;
        let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, NATURE_HSE, "close");
        publier_enveloppe(&mut tx, etab, types::DEMANDE_CLOSE, &env, Some(user_id)).await?;
        tx.commit().await?;
        Ok(EtatDemande { id, numero: a.ligne.numero, statut: "annulee".into(), version: version_attendue + 1 })
    }
}

// ───────────────────────── Enveloppe (en quarts d'heure) ─────────────────────────

async fn annee_courante(conn: &mut PgConnection, etab: Uuid, jour: NaiveDate) -> Result<Uuid, RefusDemande> {
    let r: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_annee_scolaire
          WHERE etablissement_id = $1 AND (courante OR (date_debut <= $2 AND date_fin >= $2))
          ORDER BY courante DESC, date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .bind(jour)
    .fetch_optional(conn)
    .await?;
    r.map(|(i,)| i).ok_or(RefusDemande::Indisponible)
}

/// Calcul de l'enveloppe : mouvements, accordé (net du non réalisé des périodes closes) et fait.
async fn calculer_enveloppe(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: Uuid,
    site: Option<Uuid>,
    jour: NaiveDate,
) -> Result<EnveloppeHse, RefusDemande> {
    let mvts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT m.type, COALESCE(sum(m.quarts), 0)::bigint
           FROM ent_dem_hse_enveloppe e
           JOIN ent_dem_hse_enveloppe_mvt m ON m.etablissement_id = e.etablissement_id AND m.enveloppe_id = e.id
          WHERE e.etablissement_id = $1 AND e.annee_id = $2 AND e.site_id IS NOT DISTINCT FROM $3
          GROUP BY m.type",
    )
    .bind(etab)
    .bind(annee)
    .bind(site)
    .fetch_all(&mut *conn)
    .await?;
    let get = |t: &str| mvts.iter().find(|(k, _)| k == t).map_or(0, |(_, v)| *v);
    let (dotation, abondement, retrait, prevision) = (get("dotation"), get("abondement"), get("retrait"), get("prevision"));
    let saisie = !mvts.is_empty();
    let base = if dotation > 0 { dotation } else { prevision };
    let total = base + abondement - retrait;
    // Accordé et fait, par demande ; le non réalisé d'une période close est rendu à l'enveloppe (DEM-276).
    let lignes: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT h.motif,
                CASE WHEN h.periode_au < $4 THEN LEAST(COALESCE(h.volume_accorde_quarts, 0)::bigint, COALESCE(f.fait, 0))
                     ELSE COALESCE(h.volume_accorde_quarts, 0)::bigint END AS engage,
                COALESCE(f.fait, 0) AS fait
           FROM ent_dem_hse h
           JOIN ent_dem_demande d ON d.etablissement_id = h.etablissement_id AND d.id = h.demande_id
           LEFT JOIN LATERAL (SELECT sum(COALESCE(s.retenu_quarts, 0))::bigint AS fait FROM ent_dem_hse_seance s
                               WHERE s.etablissement_id = h.etablissement_id AND s.demande_id = h.demande_id
                                 AND s.etat IN ('faite', 'attestee')) f ON true
          WHERE h.etablissement_id = $1 AND h.annee_id = $2 AND h.site_id IS NOT DISTINCT FROM $3
            AND h.decision IN ('accordee', 'accordee_partiellement') AND d.statut <> 'annulee'",
    )
    .bind(etab)
    .bind(annee)
    .bind(site)
    .bind(jour)
    .fetch_all(&mut *conn)
    .await?;
    let engage: i64 = lignes.iter().map(|(_, e, _)| *e).sum();
    let fait: i64 = lignes.iter().map(|(_, _, f)| *f).sum();
    let mut par_motif: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for (m, e, _) in &lignes {
        *par_motif.entry(m.clone()).or_insert(0) += *e;
    }
    Ok(EnveloppeHse {
        annee_id: annee,
        site_id: site,
        saisie,
        dotation_quarts: dotation,
        abondement_quarts: abondement,
        retrait_quarts: retrait,
        prevision_quarts: prevision,
        total_quarts: total,
        engage_quarts: engage,
        fait_quarts: fait,
        restant_quarts: total - engage,
        par_motif: par_motif.into_iter().collect(),
    })
}

impl PgEnt {
    /// Ajoute un mouvement d'enveloppe (DEM-277, DEM-376) : dotation, abondement, retrait ou prévision, **en quarts
    /// d'heure**. Les mouvements ne se modifient jamais : on corrige par un mouvement contraire.
    #[allow(clippy::too_many_arguments)]
    pub async fn mouvement_enveloppe_hse_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        site_id: Option<Uuid>,
        type_: &str,
        quarts: i32,
        note: Option<&str>,
    ) -> Result<(), RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        if !matches!(type_, "dotation" | "abondement" | "retrait" | "prevision") {
            return Err(RefusDemande::Invalide("Ce type de mouvement n'existe pas."));
        }
        if !(1..=400_000).contains(&quarts) {
            return Err(RefusDemande::Invalide("Le volume du mouvement est invalide."));
        }
        let note = texte_court(&note.map(str::to_string), 300)?;
        if !self.a_action_etablissement(etab, user_id, role, Action::DemandesHseEnveloppeGerer).await? {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = crate::demandes::personne_du_user(&mut tx, etab, user_id).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let annee = annee_courante(&mut tx, etab, jour).await?;
        let existante: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_dem_hse_enveloppe WHERE etablissement_id = $1 AND annee_id = $2 AND site_id IS NOT DISTINCT FROM $3",
        )
        .bind(etab)
        .bind(annee)
        .bind(site_id)
        .fetch_optional(&mut *tx)
        .await?;
        let enveloppe = match existante {
            Some((i,)) => i,
            None => {
                let i = Uuid::new_v4();
                sqlx::query("INSERT INTO ent_dem_hse_enveloppe (id, etablissement_id, annee_id, site_id) VALUES ($1, $2, $3, $4)")
                    .bind(i)
                    .bind(etab)
                    .bind(annee)
                    .bind(site_id)
                    .execute(&mut *tx)
                    .await?;
                i
            }
        };
        sqlx::query(
            "INSERT INTO ent_dem_hse_enveloppe_mvt (id, etablissement_id, enveloppe_id, type, quarts, note, par_personne_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(enveloppe)
        .bind(type_)
        .bind(quarts)
        .bind(note)
        .bind(moi)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// L'enveloppe d'un site pour l'année en cours (DEM-263, DEM-278), pour qui décide ou gère l'enveloppe.
    pub async fn enveloppe_hse_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        site_id: Option<Uuid>,
    ) -> Result<EnveloppeHse, RefusDemande> {
        let ok = self.a_action_etablissement(etab, user_id, role, Action::DemandesHseDecider).await?
            || self.a_action_etablissement(etab, user_id, role, Action::DemandesHseEnveloppeGerer).await?;
        if !ok {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let annee = annee_courante(&mut tx, etab, jour).await?;
        let e = calculer_enveloppe(&mut tx, etab, annee, site_id, jour).await?;
        tx.commit().await?;
        Ok(e)
    }

    /// Répartition des HSE accordées par agent sur l'année en cours (DEM-280, DEM-377), pour arbitrer équitablement.
    /// Réservée aux décideurs : **jamais visible des collègues**.
    pub async fn repartition_hse_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<Vec<RepartitionHse>, RefusDemande> {
        if !self.a_action_etablissement(etab, user_id, role, Action::DemandesHseDecider).await? {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let annee = annee_courante(&mut tx, etab, jour).await?;
        let l: Vec<(Uuid, String, Option<String>, i64, i64, i64)> = sqlx::query_as(
            "SELECT h.beneficiaire_agent_id, p.nom, p.prenom, count(*)::bigint,
                    COALESCE(sum(h.volume_accorde_quarts), 0)::bigint,
                    COALESCE(sum((SELECT COALESCE(sum(s.retenu_quarts), 0) FROM ent_dem_hse_seance s
                                   WHERE s.etablissement_id = h.etablissement_id AND s.demande_id = h.demande_id
                                     AND s.etat IN ('faite', 'attestee'))), 0)::bigint
               FROM ent_dem_hse h
               JOIN ent_agent a ON a.etablissement_id = h.etablissement_id AND a.id = h.beneficiaire_agent_id
               JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
               JOIN ent_dem_demande d ON d.etablissement_id = h.etablissement_id AND d.id = h.demande_id
              WHERE h.etablissement_id = $1 AND h.annee_id = $2 AND h.decision IN ('accordee', 'accordee_partiellement')
                AND d.statut <> 'annulee'
              GROUP BY h.beneficiaire_agent_id, p.nom, p.prenom
              ORDER BY 5 DESC, p.nom",
        )
        .bind(etab)
        .bind(annee)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(l.into_iter()
            .map(|(agent_id, nom, prenom, demandes, accorde_quarts, fait_quarts)| RepartitionHse {
                agent_id,
                nom,
                prenom,
                demandes,
                accorde_quarts,
                fait_quarts,
            })
            .collect())
    }
}

// ───────────────────────── Lectures ─────────────────────────

const SELECT_LIGNE: &str = "SELECT d.id, d.numero, d.statut, h.motif, h.volume_demande_quarts, h.volume_accorde_quarts,
        COALESCE((SELECT sum(COALESCE(s.retenu_quarts, 0)) FROM ent_dem_hse_seance s
                   WHERE s.etablissement_id = h.etablissement_id AND s.demande_id = h.demande_id
                     AND s.etat IN ('faite', 'attestee')), 0)::bigint AS fait,
        COALESCE((SELECT sum(COALESCE(s.retenu_quarts, 0)) FROM ent_dem_hse_seance s
                   WHERE s.etablissement_id = h.etablissement_id AND s.demande_id = h.demande_id
                     AND s.transmission <> 'aucune'), 0)::bigint AS transmis,
        h.decision, h.periode_au
   FROM ent_dem_hse h
   JOIN ent_dem_demande d ON d.etablissement_id = h.etablissement_id AND d.id = h.demande_id";

type LigneHseSql = (Uuid, String, String, String, i32, Option<i32>, i64, i64, Option<String>, NaiveDate);

fn ligne_hse(l: LigneHseSql) -> LigneHse {
    LigneHse {
        demande_id: l.0,
        numero: l.1,
        statut: l.2,
        motif: l.3,
        volume_demande_quarts: l.4,
        volume_accorde_quarts: l.5,
        fait_quarts: l.6,
        transmis_quarts: l.7,
        decision: l.8,
        periode_au: l.9,
    }
}

impl PgEnt {
    /// « Mes HSE » (DEM-279) : celles dont je suis le bénéficiaire ou le demandeur. Rien des collègues.
    pub async fn mes_hse_for_actor(&self, etab: Uuid, user_id: Uuid) -> Result<Vec<LigneHse>, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = crate::demandes::personne_du_user(&mut tx, etab, user_id).await?;
        let sql = format!(
            "{SELECT_LIGNE}
              WHERE h.etablissement_id = $1
                AND (d.demandeur_personne_id = $2
                     OR EXISTS (SELECT 1 FROM ent_agent a WHERE a.etablissement_id = h.etablissement_id
                                  AND a.id = h.beneficiaire_agent_id AND a.personne_id = $2))
              ORDER BY d.cree_le DESC, d.id LIMIT 200"
        );
        let l: Vec<LigneHseSql> = sqlx::query_as(&sql).bind(etab).bind(moi).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(l.into_iter().map(ligne_hse).collect())
    }

    /// La file des HSE **à décider** (DEM-262) : celles des files dont je suis membre, jamais la mienne ni celle que
    /// j'ai déposée (R-DM40), jamais une demande qui me vise (R-DM47).
    pub async fn hse_a_decider_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
    ) -> Result<Vec<LigneHse>, RefusDemande> {
        let r = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !r.is_staff() || !self.a_action_etablissement(etab, user_id, role, Action::DemandesHseDecider).await? {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = crate::demandes::personne_du_user(&mut tx, etab, user_id).await?;
        let files = sources.files_de(etab, moi);
        let sql = format!(
            "{SELECT_LIGNE}
              WHERE h.etablissement_id = $1 AND d.file_id = ANY($3)
                AND d.statut NOT IN ('annulee', 'brouillon')
                AND (h.decision IS NULL OR h.avenant_volume_quarts IS NOT NULL)
                AND NOT (h.confirmation_requise AND h.confirmee_le IS NULL)
                AND d.demandeur_personne_id IS DISTINCT FROM $2
                AND d.deposant_personne_id IS DISTINCT FROM $2
                AND NOT EXISTS (SELECT 1 FROM ent_agent a WHERE a.etablissement_id = h.etablissement_id
                                  AND a.id = h.beneficiaire_agent_id AND a.personne_id = $2)
                AND NOT EXISTS (SELECT 1 FROM ent_dem_personne_visee v WHERE v.etablissement_id = h.etablissement_id
                                  AND v.demande_id = h.demande_id AND v.personne_id = $2)
              ORDER BY d.depose_le, d.id LIMIT 200"
        );
        let l: Vec<LigneHseSql> = sqlx::query_as(&sql).bind(etab).bind(moi).bind(&files).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(l.into_iter().map(ligne_hse).collect())
    }

    /// Le détail d'une demande d'HSE pour qui y a accès (demandeur, déposant, bénéficiaire, décideur membre de la file).
    pub async fn detail_hse_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
    ) -> Result<DetailHse, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, false).await?;
        refus_geste_hse(&a)?;
        if !matches!(a.lien, Lien::Demandeur | Lien::Deposant | Lien::Traitant) {
            return Err(RefusDemande::Introuvable);
        }
        let h = charger_hse(&mut tx, etab, id, false).await?;
        let ss: Vec<(Uuid, NaiveDate, Option<NaiveTime>, i32, Option<i32>, bool, String, String, Option<String>, bool)> = sqlx::query_as(
            "SELECT id, jour, debut, duree_quarts, retenu_quarts, forfait, etat, transmission, fondement, echantillon
               FROM ent_dem_hse_seance WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY jour, debut NULLS LAST, cree_le, id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(DetailHse {
            demande_id: id,
            numero: a.ligne.numero,
            statut: a.ligne.statut,
            motif: h.motif,
            type_heure: h.type_heure,
            volume_demande_quarts: h.volume_demande_quarts,
            volume_accorde_quarts: h.volume_accorde_quarts,
            periode_du: h.periode_du,
            periode_au: h.periode_au,
            decision: h.decision,
            confirmation_requise: h.confirmation_requise,
            confirmee: h.confirmee,
            avenants: h.avenants,
            avenant_en_attente: h.avenant_volume_quarts.is_some(),
            deposee_pour_compte: a.ligne.deposant_personne_id.is_some(),
            version: a.ligne.version,
            seances: ss
                .into_iter()
                .map(|s| SeanceHse {
                    id: s.0,
                    jour: s.1,
                    debut: s.2,
                    duree_quarts: s.3,
                    retenu_quarts: s.4,
                    forfait: s.5,
                    etat: s.6,
                    transmission: s.7,
                    fondement: s.8,
                    echantillon: s.9,
                })
                .collect(),
        })
    }
}

// ───────────────────────── Export de secours (DEM-282) ─────────────────────────

/// Une cellule CSV : séparateur « ; », guillemets doublés, et neutralisation d'une formule de tableur (`=`, `+`, `-`,
/// arobase) : un nom ne s'exécute jamais dans le tableur de la comptabilité.
pub(crate) fn cellule_csv(s: &str) -> String {
    let mut t = s.replace(['\r', '\n'], " ");
    if t.starts_with(['=', '+', '-', '@', '\t']) {
        t.insert(0, '\'');
    }
    if t.contains([';', '"']) {
        format!("\"{}\"", t.replace('"', "\"\""))
    } else {
        t
    }
}

impl PgEnt {
    /// L'export mensuel de secours : les heures retenues, **en heures par agent et par type d'heure**, pour
    /// l'application académique. Tracé (lot, auteur, date). Rejouable : les mêmes heures reviennent tant que le socle 15
    /// n'est pas prêt, mais le lot ne se pose qu'une fois par séance (`export_id`).
    pub async fn exporter_hse_mensuel_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        mois: NaiveDate,
    ) -> Result<ExportHse, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        if !self.a_action_etablissement(etab, user_id, role, Action::DemandesHseEnveloppeGerer).await? {
            return Err(RefusDemande::Interdit);
        }
        let debut = mois.with_day(1).ok_or(RefusDemande::Invalide("Le mois est invalide."))?;
        let fin = if debut.month() == 12 {
            NaiveDate::from_ymd_opt(debut.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(debut.year(), debut.month() + 1, 1)
        }
        .ok_or(RefusDemande::Invalide("Le mois est invalide."))?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = crate::demandes::personne_du_user(&mut tx, etab, user_id).await?;
        let l: Vec<(String, Option<String>, Option<String>, String, i64, i64)> = sqlx::query_as(
            "SELECT p.nom, p.prenom, a.matricule_interne, h.type_heure, sum(s.retenu_quarts)::bigint, count(*)::bigint
               FROM ent_dem_hse_seance s
               JOIN ent_dem_hse h ON h.etablissement_id = s.etablissement_id AND h.demande_id = s.demande_id
               JOIN ent_agent a ON a.etablissement_id = h.etablissement_id AND a.id = h.beneficiaire_agent_id
               JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
              WHERE s.etablissement_id = $1 AND s.transmission = 'export_secours' AND s.jour >= $2 AND s.jour < $3
                AND COALESCE(s.retenu_quarts, 0) > 0
              GROUP BY p.nom, p.prenom, a.matricule_interne, h.type_heure, a.id
              ORDER BY p.nom, p.prenom NULLS FIRST, h.type_heure",
        )
        .bind(etab)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *tx)
        .await?;
        let (nb, total): (i64, i64) = (l.iter().map(|x| x.5).sum(), l.iter().map(|x| x.4).sum());
        let lot = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_dem_hse_export (id, etablissement_id, mois, nb_seances, total_quarts, exporte_par_personne_id)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(lot)
        .bind(etab)
        .bind(debut)
        .bind(i32::try_from(nb).unwrap_or(i32::MAX))
        .bind(i32::try_from(total).unwrap_or(i32::MAX))
        .bind(moi)
        .execute(&mut *tx)
        .await?;
        // Le lot ne se pose qu'une fois par séance : un second export reprend les mêmes heures sans les remarquer.
        sqlx::query(
            "UPDATE ent_dem_hse_seance SET export_id = $4
              WHERE etablissement_id = $1 AND transmission = 'export_secours' AND export_id IS NULL
                AND jour >= $2 AND jour < $3 AND COALESCE(retenu_quarts, 0) > 0",
        )
        .bind(etab)
        .bind(debut)
        .bind(fin)
        .bind(lot)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        let mut csv = String::from("nom;prenom;matricule;type_heure;heures;quarts_heure;seances\n");
        for (nom, prenom, matricule, type_heure, quarts, seances) in l {
            csv.push_str(&format!(
                "{};{};{};{};{};{};{}\n",
                cellule_csv(&nom),
                cellule_csv(prenom.as_deref().unwrap_or("")),
                cellule_csv(matricule.as_deref().unwrap_or("")),
                type_heure,
                heures_decimales(quarts),
                quarts,
                seances
            ));
        }
        Ok(ExportHse {
            lot_id: lot,
            mois: debut,
            nb_seances: i32::try_from(nb).unwrap_or(i32::MAX),
            total_quarts: i32::try_from(total).unwrap_or(i32::MAX),
            csv,
        })
    }

    /// Alerte de fin de période (DEM-284, CDM-42) : HSE accordées, période terminée, **aucune séance faite**. Tâche
    /// de l'ordonnanceur, **rejouable** (un seul fait par demande) ; rapport sans nom.
    pub async fn alerter_hse_non_declarees(&self, etab: Uuid) -> Result<RapportAlertesHse, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let l: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT h.demande_id, d.numero
               FROM ent_dem_hse h
               JOIN ent_dem_demande d ON d.etablissement_id = h.etablissement_id AND d.id = h.demande_id
              WHERE h.etablissement_id = $1 AND h.decision IN ('accordee', 'accordee_partiellement')
                AND h.periode_au < $2 AND d.statut <> 'annulee'
                AND NOT EXISTS (SELECT 1 FROM ent_dem_hse_seance s WHERE s.etablissement_id = h.etablissement_id
                                  AND s.demande_id = h.demande_id AND s.etat IN ('faite', 'attestee'))
                AND NOT EXISTS (SELECT 1 FROM ent_dem_fait f WHERE f.etablissement_id = h.etablissement_id
                                  AND f.demande_id = h.demande_id AND f.type = 'hse_rien_declare')
              ORDER BY h.periode_au, h.demande_id LIMIT 500",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await?;
        let mut r = RapportAlertesHse::default();
        for (id, numero) in l {
            let mut f = FaitNouveau::nouveau(id, "hse_rien_declare");
            f.auteur_tache = Some("ordonnanceur_echeances");
            ajouter_fait(&mut tx, etab, f).await?;
            let env = EnveloppeNeutre::nouvelle(id, &numero, NATURE_HSE, "hse_rien_declare");
            publier_enveloppe(&mut tx, etab, types::DEMANDE_HSE_RIEN_DECLARE, &env, None).await?;
            r.signalees += 1;
        }
        tx.commit().await?;
        Ok(r)
    }
}
