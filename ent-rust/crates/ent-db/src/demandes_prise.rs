//! Socle N1 (demandes), lot N1-2 : **prise en charge, délégation, réaffectation, consultation, désignation d'une
//! personne visée, requalification, « Résolu ? », preuves visibles du demandeur** (`N1_DEMANDES.md` §11, §37, §38).
//!
//! La promesse centrale (§37.1) tient en **trois preuves visibles du demandeur** : *envoyée* (le service, son rôle,
//! un délai indicatif), *vue* (première ouverture du détail par un membre de la file, R-DM53), *prise* (prénom et
//! initiale, sauf « service seul »). Un retard est **toujours montré** (R-DM54). Rien ne bouge en silence : toute
//! réaffectation est un geste humain, journalisé, qui prévient (29 §8.9).
//!
//! Défauts du Next.js fermés ici (règle 14 de `PILOTAGE.md`) :
//!
//! * **NJ-81** — une demande d'une autre file ne se détourne pas : toutes les entrées passent par la garde d'accès
//!   unique ([`charger_pour_acces`]) ; sans lien avec la demande (même responsable d'une autre file) : 404 neutre.
//!   Le transfert ne part que de la file qui détient la demande, jamais vers une file qui ne peut pas la lire.
//! * **NJ-80** étendu aux échanges et commentaires : voir `demandes_echanges`.
//!
//! Le jour métier est celui de l'établissement (`ent_aujourdhui_etab`), y compris pour convertir un horodatage en
//! date ; jamais le jour du serveur ni le jour UTC.

use crate::champs_chiffres::ChampsSensibles;
use crate::demandes::{
    ajouter_fait, charger_demande, charger_pour_acces, echeance_ouvree, nature, ouvrir_texte, personne_du_user,
    publier_enveloppe, Acces, EnveloppeNeutre, EtatDemande, FaitNouveau, Lien, QuotasDemandes,
    RefusDemande, SourcesDemandes, Statut, Urgence,
};
use crate::demandes_echanges::{
    ecrire_tx, est_responsable_de, prenom_initiale, service_public, valider_texte_message, DocumentsAbsents, Ecriture,
    Fil, MessageEnvoye, MessageVu, PieceVue,
};
use crate::demandes_files::{charger_graphe, jour_metier};
use crate::demandes_routage::router_demande_existante;
use crate::service::PgEnt;
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::{Action, Role};
use ent_events::types;
use serde::Serialize;
use sqlx::PgConnection;
use std::collections::BTreeMap;
use uuid::Uuid;

/// Délai de réouverture après clôture (SN1-05, 30 jours par défaut ; réglage ADM à venir avec les écrans).
pub const DELAI_REOUVERTURE_JOURS: i64 = 30;
/// Motifs codés d'une réaffectation (jamais de texte libre dans le fait).
pub const MOTIFS_REAFFECTATION: [&str; 5] = ["absence", "depart", "surcharge", "competence", "autre"];
/// Motifs codés d'un transfert (§11.6).
pub const MOTIFS_TRANSFERT: [&str; 4] = ["hors_de_notre_service", "releve_d_un_autre_service", "mauvaise_nature", "autre"];

/// Destination d'une réaffectation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cible {
    /// Un membre de la file (identifiant d'agent).
    Agent(Uuid),
    /// Remettre la demande dans la file : plus personne ne la porte.
    File,
}

// ───────────────────────── Vues rendues ─────────────────────────

/// Les quatre états en mots simples (§37.1) ; les dix états du §11.1 restent internes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatSimple {
    Brouillon,
    Envoyee,
    Vue,
    EnCours,
    /// « On attend votre réponse » : un complément est demandé.
    AttenteReponse,
    Terminee,
    Annulee,
}

impl EtatSimple {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Brouillon => "brouillon",
            Self::Envoyee => "envoyee",
            Self::Vue => "vue",
            Self::EnCours => "en_cours",
            Self::AttenteReponse => "attente_reponse",
            Self::Terminee => "terminee",
            Self::Annulee => "annulee",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreuveEnvoyee {
    pub le: DateTime<Utc>,
    /// Le service qui traite (« l'établissement » si la file ou la nature est confidentielle).
    pub service: String,
    /// Le rôle de la personne qui traite (« le secrétariat des élèves »), jamais un nom avant la prise.
    pub role: Option<String>,
    /// Date indicative de réponse (échéance de traitement).
    pub delai_indicatif_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreuveVue {
    pub le: DateTime<Utc>,
    pub service: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrisePar {
    pub le: DateTime<Utc>,
    /// « Paul D. », ou le service seul (« le secrétariat ») pour une file « service seul » ou confidentielle.
    pub par: String,
    pub nominatif: bool,
}

/// Retard **montré** au demandeur (R-DM54) : jamais caché.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RetardMontre {
    pub jours: i64,
    /// Le responsable du service a été prévenu (fait `retard_signale` posé par l'ordonnanceur).
    pub responsable_prevenu: bool,
}

/// Les trois preuves et l'état simple, **sans aucun identifiant de file, d'agent ni de règle** (NJ-82).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreuvesDemandeur {
    pub etat: EtatSimple,
    pub etat_libelle: String,
    pub envoyee: PreuveEnvoyee,
    pub vue: Option<PreuveVue>,
    pub prise: Option<PrisePar>,
    pub retard: Option<RetardMontre>,
}

/// Suite d'une réponse « Résolu ? » (DEM-394, CDM-86).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuiteResolu {
    /// « Oui » : merci, rien ne bouge.
    Merci,
    /// « Non » dans le délai : la demande est rouverte, même demande.
    Rouverte(EtatDemande),
    /// « Non » hors délai : une nouvelle demande est conseillée, liée à celle-ci.
    HorsDelai { ancienne_demande: Uuid },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Requalification {
    pub etat: EtatDemande,
    /// L'échéance de traitement a changé : le demandeur en est averti (CDM-01) ; sinon, aucun avis.
    pub delai_change: bool,
    /// La demande a quitté sa file (les membres de l'ancienne file ne la voient plus, CDM-11).
    pub file_change: bool,
}

/// Un membre de la file que le demandeur peut désigner comme « concerné » (DEM-360) : un libellé, jamais plus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MembreDesignable {
    pub personne_id: Uuid,
    pub libelle: String,
}

/// Une demande « vue mais non prise » depuis plusieurs jours (DEM-399) : titre neutre, rien de confidentiel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VueNonPrise {
    pub id: Uuid,
    pub numero: String,
    /// `None` pour une nature C2+ : la direction voit l'existence et le retard, pas la nature (R-DM15).
    pub nature_code: Option<String>,
    pub file_id: Uuid,
    pub vue_le: DateTime<Utc>,
}

/// Les demandes que porte un agent absent (sans motif : l'absence se lit au socle 15, jamais ici).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DemandesDAbsent {
    pub agent_id: Uuid,
    pub nb_demandes: i64,
    /// Dernier jour d'absence (jour métier), sans motif ni nature d'absence.
    pub absent_jusqu_au: NaiveDate,
}

/// Une consultation demandée à un tiers.
#[derive(Debug, Clone)]
pub struct ConsultationNouvelle {
    pub agent_id: Uuid,
    pub question: String,
    /// Échéance de réponse en jours ouvrés (1 à 30).
    pub delai_jours: u16,
    /// Le tiers voit l'objet et le texte de la demande (jamais pour une nature C2+).
    pub partage_demande: bool,
    /// Pièces de la demande partagées avec lui (aucune par défaut).
    pub pieces_partagees: Vec<Uuid>,
}

/// Ce que voit le **consulté** : la question, ce qui est partagé, l'échange. Jamais le reste de la demande.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConsultationVue {
    pub id: Uuid,
    pub numero: String,
    pub objet: Option<String>,
    pub texte: Option<String>,
    pub question: String,
    pub echeance: NaiveDate,
    pub repondue: bool,
    pub messages: Vec<MessageVu>,
    pub pieces: Vec<PieceVue>,
}

/// Rapport de l'ordonnanceur des retards : un nombre, aucun nom.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RapportRetards {
    pub signalees: u32,
}

// ───────────────────────── Aides ─────────────────────────

fn statut_de(s: &str) -> Result<Statut, RefusDemande> {
    Statut::parse(s).ok_or(RefusDemande::Indisponible)
}

fn rang_conf(c: &str) -> u8 {
    match c {
        "C0" => 0,
        "C1" => 1,
        "C2" => 2,
        _ => 3,
    }
}

/// Le porteur de la prise, ou un membre quand personne ne la porte, ou le responsable : seuls ceux-là engagent la
/// demande (transfert, requalification, consultation). Un autre membre ne la détourne pas (NJ-81).
fn a_la_main(a: &Acces, responsable: bool) -> bool {
    a.ligne.attribuee_a_agent_id.is_none() || a.ligne.attribuee_a_agent_id == a.agent || responsable
}

/// Les agents absents ce jour (socle 15 : absences **décidées**, de toute cause), avec leur dernier jour d'absence.
/// Sans motif ni nature : seule la période sort de cette fonction (R-AP2).
pub(crate) async fn agents_absents(
    conn: &mut PgConnection,
    etab: Uuid,
    agents: &[Uuid],
    jour: NaiveDate,
) -> Result<BTreeMap<Uuid, NaiveDate>, RefusDemande> {
    if agents.is_empty() {
        return Ok(BTreeMap::new());
    }
    let etats: Vec<String> = crate::absences_personnels::ETATS_DECIDES.iter().map(|s| s.to_string()).collect();
    let l: Vec<(Uuid, NaiveDate)> = sqlx::query_as(
        "SELECT a.agent_id, max(ent_aujourdhui_etab($1, a.fin - interval '1 microsecond'))
           FROM ent_absp_absence a
          WHERE a.etablissement_id = $1 AND a.agent_id = ANY($2) AND a.etat = ANY($4)
            AND ent_aujourdhui_etab($1, a.debut) <= $3
            AND ent_aujourdhui_etab($1, a.fin - interval '1 microsecond') >= $3
          GROUP BY a.agent_id",
    )
    .bind(etab)
    .bind(agents)
    .bind(jour)
    .bind(etats)
    .fetch_all(conn)
    .await?;
    Ok(l.into_iter().collect())
}

async fn est_visee(conn: &mut PgConnection, etab: Uuid, demande: Uuid, personne: Uuid) -> Result<bool, RefusDemande> {
    let (v,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_dem_personne_visee
                         WHERE etablissement_id = $1 AND demande_id = $2 AND personne_id = $3)",
    )
    .bind(etab)
    .bind(demande)
    .bind(personne)
    .fetch_one(conn)
    .await?;
    Ok(v)
}

async fn vises_de(conn: &mut PgConnection, etab: Uuid, demande: Uuid) -> Result<Vec<Uuid>, RefusDemande> {
    let l: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT personne_id FROM ent_dem_personne_visee WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY cree_le",
    )
    .bind(etab)
    .bind(demande)
    .fetch_all(conn)
    .await?;
    Ok(l.into_iter().map(|(p,)| p).collect())
}

/// Un destinataire valide d'une attribution : **membre de la file** de la demande (jamais d'escalade de droits :
/// on ne confie pas une demande à qui n'aurait pas le droit de la lire), non visé, non absent.
async fn destinataire_valide(conn: &mut PgConnection, etab: Uuid, a: &Acces, demande: Uuid, agent: Uuid) -> Result<(), RefusDemande> {
    let jour = jour_metier(conn, etab).await?;
    let g = charger_graphe(conn, etab, jour).await?;
    let membre = g
        .membres
        .get(&a.ligne.file_id)
        .and_then(|ms| ms.iter().find(|m| m.agent_id == Some(agent)))
        .ok_or(RefusDemande::Invalide("Cette personne n'est pas membre de la file de cette demande."))?;
    if est_visee(conn, etab, demande, membre.personne_id).await? {
        return Err(RefusDemande::Invalide("Cette personne ne peut pas recevoir cette demande."));
    }
    if agents_absents(conn, etab, &[agent], jour).await?.contains_key(&agent) {
        return Err(RefusDemande::Conflit("Cette personne est absente : choisissez un autre membre."));
    }
    Ok(())
}

/// Insère une échéance (jour métier converti en fin de journée de l'établissement, comme au dépôt).
async fn inserer_echeance(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    type_: &str,
    jour: NaiveDate,
    suspendue: bool,
) -> Result<(), RefusDemande> {
    sqlx::query(
        "INSERT INTO ent_dem_echeance (id, etablissement_id, demande_id, type, due_le, suspendue_depuis)
         VALUES ($1, $2, $3, $4,
                 ($5::date + time '23:59:59') AT TIME ZONE COALESCE(
                    (SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $2), 'Europe/Paris'),
                 CASE WHEN $6 THEN now() END)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(demande)
    .bind(type_)
    .bind(jour)
    .bind(suspendue)
    .execute(conn)
    .await?;
    Ok(())
}

struct ChangeAttribution<'a> {
    fait: &'a str,
    motif: Option<&'a str>,
    ref_id: Option<Uuid>,
    evenement: &'static str,
}

/// Change le porteur de la prise (ou la remet en file) : **seul** chemin de réaffectation. Version vérifiée, fait
/// chaîné, événement à enveloppe neutre ; jamais silencieux (§37.5).
async fn changer_attribution(
    conn: &mut PgConnection,
    etab: Uuid,
    user_id: Uuid,
    a: &Acces,
    id: Uuid,
    vers: Option<Uuid>,
    c: ChangeAttribution<'_>,
) -> Result<EtatDemande, RefusDemande> {
    let statut = statut_de(&a.ligne.statut)?;
    let apres = match (vers, statut) {
        (Some(_), Statut::Deposee | Statut::Rouverte) => Statut::Prise,
        (None, Statut::Prise) => Statut::Deposee,
        (_, s) => s,
    };
    let n = sqlx::query(
        "UPDATE ent_dem_demande
            SET statut = $4, attribuee_a_agent_id = $5,
                attribuee_le = CASE WHEN $5::uuid IS NULL THEN NULL ELSE now() END, version = version + 1
          WHERE etablissement_id = $1 AND id = $2 AND version = $3",
    )
    .bind(etab)
    .bind(id)
    .bind(a.ligne.version)
    .bind(apres.code())
    .bind(vers)
    .execute(&mut *conn)
    .await?
    .rows_affected();
    if n != 1 {
        return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
    }
    if vers.is_some() {
        sqlx::query(
            "UPDATE ent_dem_echeance SET traitee_le = now()
              WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'prise_en_charge' AND traitee_le IS NULL",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *conn)
        .await?;
    }
    let mut f = FaitNouveau::nouveau(id, c.fait);
    f.auteur_personne = Some(a.moi);
    f.statut_avant = Some(&a.ligne.statut);
    f.statut_apres = Some(apres.code());
    f.agent_avant = a.ligne.attribuee_a_agent_id;
    f.agent_apres = vers;
    f.motif = c.motif;
    f.ref_id = c.ref_id;
    ajouter_fait(conn, etab, f).await?;
    let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, &a.ligne.nature_code, c.evenement);
    publier_enveloppe(conn, etab, types::DEMANDE_REAFFECTEE, &env, Some(user_id)).await?;
    Ok(EtatDemande { id, numero: a.ligne.numero.clone(), statut: apres.code().into(), version: a.ligne.version + 1 })
}

impl PgEnt {
    /// Début d'un geste : débit (NJ-72), garde d'accès unique avec verrou (NJ-80, NJ-81), version (NJ-90).
    async fn debut_geste(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
    ) -> Result<(TenantTx<'static>, Acces, Role), RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        if acces.ligne.version != version_attendue {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        if statut_de(&acces.ligne.statut)?.est_final() {
            return Err(RefusDemande::Conflit("Cette demande est terminée."));
        }
        Ok((tx, acces, role))
    }

    // ───────────── Réaffecter, déléguer, reprendre ─────────────

    /// Réattribue la demande à un membre de la file, ou la remet en file (DEM-173). Réservé au **responsable de la
    /// file qui détient la demande** : le responsable d'une autre file ne la voit pas (NJ-81). Journalisé, avec avis.
    #[allow(clippy::too_many_arguments)]
    pub async fn reaffecter_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
        cible: Cible,
        motif: Option<&str>,
    ) -> Result<EtatDemande, RefusDemande> {
        let (mut tx, a, _) = self.debut_geste(etab, user_id, role, sources, id, version_attendue).await?;
        if a.lien != Lien::Traitant || !est_responsable_de(&mut tx, etab, a.ligne.file_id, a.moi).await? {
            return Err(RefusDemande::Interdit);
        }
        let motif = motif_code(motif, &MOTIFS_REAFFECTATION)?;
        let vers = match cible {
            Cible::Agent(ag) => {
                if a.ligne.attribuee_a_agent_id == Some(ag) {
                    return Err(RefusDemande::Conflit("Cette personne porte déjà la demande."));
                }
                destinataire_valide(&mut tx, etab, &a, id, ag).await?;
                Some(ag)
            }
            Cible::File => {
                if a.ligne.attribuee_a_agent_id.is_none() {
                    return Err(RefusDemande::Conflit("La demande est déjà dans la file."));
                }
                None
            }
        };
        let e = changer_attribution(
            &mut tx,
            etab,
            user_id,
            &a,
            id,
            vers,
            ChangeAttribution { fait: "reaffectee", motif, ref_id: None, evenement: "reaffectee" },
        )
        .await?;
        tx.commit().await?;
        Ok(e)
    }

    /// Délègue ponctuellement la demande à un membre de la file, avec une note interne (DEM-174). Par le **porteur
    /// de la prise** ou le responsable de la file. Anti-escalade : le destinataire est membre de la file, donc
    /// habilité à lire la nature (une demande C3 ne sort jamais vers qui n'a pas le droit de la lire).
    #[allow(clippy::too_many_arguments)]
    pub async fn deleguer_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
        a_agent: Uuid,
        note: Option<&str>,
    ) -> Result<EtatDemande, RefusDemande> {
        let (mut tx, a, _) = self.debut_geste(etab, user_id, role, sources, id, version_attendue).await?;
        if a.lien != Lien::Traitant {
            return Err(RefusDemande::Interdit);
        }
        let porteur = a.ligne.attribuee_a_agent_id.is_some() && a.ligne.attribuee_a_agent_id == a.agent;
        if !porteur && !est_responsable_de(&mut tx, etab, a.ligne.file_id, a.moi).await? {
            return Err(RefusDemande::Interdit);
        }
        if a.ligne.attribuee_a_agent_id == Some(a_agent) {
            return Err(RefusDemande::Conflit("Cette personne porte déjà la demande."));
        }
        destinataire_valide(&mut tx, etab, &a, id, a_agent).await?;
        let note = note.map(valider_texte_message).transpose()?;
        let e = changer_attribution(
            &mut tx,
            etab,
            user_id,
            &a,
            id,
            Some(a_agent),
            ChangeAttribution { fait: "deleguee", motif: Some("delegation"), ref_id: None, evenement: "deleguee" },
        )
        .await?;
        if let Some(t) = note {
            let ec = Ecriture { fil: Fil::Interne, texte: &t, pieces: &[], corrige: None, complement: false };
            ecrire_tx(&mut tx, etab, user_id, &DocumentsAbsents, id, &a, ec).await?;
        }
        tx.commit().await?;
        Ok(e)
    }

    /// Reprend « pour » un collègue absent qui m'a **délégué** (délégation datée du socle 6) : la demande qu'il porte
    /// passe à mon nom, d'un geste (§11.3). Sans délégation active de sa part vers moi : refusé. Je dois être membre
    /// de la file (pas d'escalade de droits).
    pub async fn reprendre_pour_delegant_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
    ) -> Result<EtatDemande, RefusDemande> {
        let (mut tx, a, _) = self.debut_geste(etab, user_id, role, sources, id, version_attendue).await?;
        let moi_agent = a.agent.filter(|_| a.lien == Lien::Traitant).ok_or(RefusDemande::Interdit)?;
        let porteur = a
            .ligne
            .attribuee_a_agent_id
            .ok_or(RefusDemande::Conflit("Cette demande n'est pas prise : prenez-la."))?;
        if porteur == moi_agent {
            return Err(RefusDemande::Conflit("Vous portez déjà cette demande."));
        }
        let deleg: Option<(Uuid,)> = sqlx::query_as(
            "SELECT d.id FROM ent_delegation d
               JOIN ent_agent ag ON ag.etablissement_id = d.etablissement_id AND ag.id = $4
               JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
              WHERE d.etablissement_id = $1 AND d.delegue_user_id = $2 AND d.delegant_user_id = p.user_id
                AND d.statut = 'accordee' AND d.revoquee_le IS NULL
                AND d.debut <= ent_aujourdhui_etab($1, now()) AND d.fin >= ent_aujourdhui_etab($1, now())
              LIMIT 1",
        )
        .bind(etab)
        .bind(user_id)
        .bind(porteur)
        .bind(porteur)
        .fetch_optional(&mut *tx)
        .await?;
        let (deleg_id,) = deleg.ok_or(RefusDemande::Interdit)?;
        let e = changer_attribution(
            &mut tx,
            etab,
            user_id,
            &a,
            id,
            Some(moi_agent),
            ChangeAttribution { fait: "reprise_deleguee", motif: Some("delegation"), ref_id: Some(deleg_id), evenement: "reprise_deleguee" },
        )
        .await?;
        tx.commit().await?;
        Ok(e)
    }

    /// Les demandes que portent des agents **absents** dans les files que je dirige (DEM-235, §11.4) : un compte par
    /// agent et sa date de retour, sans motif. Rien n'est réattribué par cette lecture.
    pub async fn demandes_des_absents_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
    ) -> Result<Vec<DemandesDAbsent>, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !role.is_staff() {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let g = charger_graphe(&mut tx, etab, jour).await?;
        let mes_files = fichiers_dirigees(&g, moi, sources.files_de(etab, moi));
        let l: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT attribuee_a_agent_id, count(*) FROM ent_dem_demande
              WHERE etablissement_id = $1 AND file_id = ANY($2) AND attribuee_a_agent_id IS NOT NULL
                AND statut NOT IN ('brouillon', 'close', 'annulee') AND purgee_le IS NULL
              GROUP BY attribuee_a_agent_id",
        )
        .bind(etab)
        .bind(&mes_files)
        .fetch_all(&mut *tx)
        .await?;
        let agents: Vec<Uuid> = l.iter().map(|(a, _)| *a).collect();
        let absents = agents_absents(&mut tx, etab, &agents, jour).await?;
        tx.commit().await?;
        Ok(l.into_iter()
            .filter_map(|(agent_id, nb)| {
                absents.get(&agent_id).map(|j| DemandesDAbsent { agent_id, nb_demandes: nb, absent_jusqu_au: *j })
            })
            .collect())
    }

    /// **Réaffecte en un geste** toutes les demandes prises par un agent absent (ou parti de la file), vers un membre
    /// ou vers la file (DEM-384, CDM-83). Réservé au responsable de chacune des files concernées ; jamais
    /// silencieux : un fait et un avis par demande. Rend le nombre de demandes réaffectées.
    pub async fn reaffecter_absent_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        agent_absent: Uuid,
        cible: Cible,
    ) -> Result<u32, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !role.is_staff() {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await?;
        let moi_agent = crate::personnels::agent_du_user_tx(&mut tx, etab, user_id).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let g = charger_graphe(&mut tx, etab, jour).await?;
        let dirigees = fichiers_dirigees(&g, moi, sources.files_de(etab, moi));
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_dem_demande
              WHERE etablissement_id = $1 AND file_id = ANY($2) AND attribuee_a_agent_id = $3
                AND statut NOT IN ('brouillon', 'close', 'annulee') AND purgee_le IS NULL
              ORDER BY depose_le, id FOR UPDATE",
        )
        .bind(etab)
        .bind(&dirigees)
        .bind(agent_absent)
        .fetch_all(&mut *tx)
        .await?;
        if rows.is_empty() {
            return Err(RefusDemande::Conflit("Aucune demande à réaffecter."));
        }
        let absent = agents_absents(&mut tx, etab, &[agent_absent], jour).await?.contains_key(&agent_absent);
        let mut n = 0u32;
        for (did,) in rows {
            let ligne = charger_demande(&mut tx, etab, did, true).await?;
            // L'agent n'est « absent » que s'il l'est au 15 ou s'il a quitté la file (départ, fin de mission).
            let toujours_membre = g
                .membres
                .get(&ligne.file_id)
                .is_some_and(|ms| ms.iter().any(|m| m.agent_id == Some(agent_absent)));
            if !absent && toujours_membre {
                return Err(RefusDemande::Conflit("Cet agent n'est pas signalé absent."));
            }
            let a = Acces { ligne, moi, agent: moi_agent, lien: Lien::Traitant };
            let vers = match cible {
                Cible::Agent(ag) => {
                    destinataire_valide(&mut tx, etab, &a, did, ag).await?;
                    Some(ag)
                }
                Cible::File => None,
            };
            changer_attribution(
                &mut tx,
                etab,
                user_id,
                &a,
                did,
                vers,
                ChangeAttribution { fait: "reaffectee", motif: Some("absence"), ref_id: None, evenement: "reaffectee" },
            )
            .await?;
            n += 1;
        }
        tx.commit().await?;
        Ok(n)
    }

    // ───────────── Transfert ─────────────

    /// Transfère la demande à une autre file (DEM-176, R-DM12) : par le porteur de la prise, le responsable, ou un
    /// membre quand personne ne la porte. La demande perd sa prise et repart « reçue » dans la file cible ; la file
    /// d'origine ne la voit plus. **Jamais vers une file qui ne peut pas la lire** (nature C2+ : file confidentielle
    /// seulement ; file sans destinataire possible : refusé) et jamais depuis une file dont on n'est pas membre
    /// (NJ-81 : la garde d'accès répond 404).
    #[allow(clippy::too_many_arguments)]
    pub async fn transferer_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
        vers_file: Uuid,
        motif: &str,
    ) -> Result<EtatDemande, RefusDemande> {
        let (mut tx, a, _) = self.debut_geste(etab, user_id, role, sources, id, version_attendue).await?;
        if a.lien != Lien::Traitant {
            return Err(RefusDemande::Interdit);
        }
        if !a_la_main(&a, est_responsable_de(&mut tx, etab, a.ligne.file_id, a.moi).await?) {
            return Err(RefusDemande::Interdit);
        }
        let statut = statut_de(&a.ligne.statut)?;
        if statut == Statut::EnAttenteDemandeur {
            return Err(RefusDemande::Conflit("La demande attend la réponse du demandeur."));
        }
        let motif = motif_code(Some(motif), &MOTIFS_TRANSFERT)?.ok_or(RefusDemande::Invalide("Le motif du transfert est obligatoire."))?;
        if vers_file == a.ligne.file_id {
            return Err(RefusDemande::Invalide("La demande est déjà dans cette file."));
        }
        let jour = jour_metier(&mut tx, etab).await?;
        let g = charger_graphe(&mut tx, etab, jour).await?;
        let cible = g.files.get(&vers_file).ok_or(RefusDemande::Invalide("Cette file n'existe pas."))?;
        if rang_conf(&a.ligne.confidentialite) >= 2 && cible.type_ != "confidentielle" {
            return Err(RefusDemande::Invalide("Cette demande est confidentielle : elle ne va qu'à une file confidentielle."));
        }
        let vises = vises_de(&mut tx, etab, id).await?;
        if g.membres_hors(vers_file, &vises).is_empty() {
            return Err(RefusDemande::Conflit("Cette file n'a aucun destinataire possible."));
        }
        let decl = nature(&a.ligne.nature_code).ok_or(RefusDemande::Indisponible)?;
        let (prise, _) = crate::demandes_routage::delais_effectifs(decl.delai_prise_jours, decl.delai_traitement_jours, None, cible);
        let due = echeance_ouvree(&mut tx, etab, jour, prise, &cible.service_calendrier).await;
        let n = sqlx::query(
            "UPDATE ent_dem_demande SET file_id = $4, attribuee_a_agent_id = NULL, attribuee_le = NULL,
                    statut = 'deposee', version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(a.ligne.version)
        .bind(vers_file)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        // Le délai de prise en charge repart pour la nouvelle file ; l'échéance de traitement reste celle du dépôt.
        sqlx::query(
            "UPDATE ent_dem_echeance SET traitee_le = now()
              WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'prise_en_charge' AND traitee_le IS NULL",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        inserer_echeance(&mut tx, etab, id, "prise_en_charge", due, false).await?;
        let mut f = FaitNouveau::nouveau(id, "transferee");
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some(&a.ligne.statut);
        f.statut_apres = Some("deposee");
        f.file_avant = Some(a.ligne.file_id);
        f.file_apres = Some(vers_file);
        f.agent_avant = a.ligne.attribuee_a_agent_id;
        f.motif = Some(motif);
        ajouter_fait(&mut tx, etab, f).await?;
        let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, &a.ligne.nature_code, "transferee");
        publier_enveloppe(&mut tx, etab, types::DEMANDE_TRANSFEREE, &env, Some(user_id)).await?;
        tx.commit().await?;
        Ok(EtatDemande { id, numero: a.ligne.numero, statut: "deposee".into(), version: a.ligne.version + 1 })
    }

    // ───────────── Consultation d'un collègue ─────────────

    /// Consulte un collègue (DEM-206, §11.6) : une question, une échéance, ce que le traitant choisit de partager.
    /// La demande **reste chez le traitant**. Réservé à celui qui a la main ; jamais pour une nature C2+ (le tiers
    /// n'a pas le droit de la lire) ; jamais la personne visée, jamais le demandeur lui-même.
    pub async fn consulter_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        c: ConsultationNouvelle,
    ) -> Result<Uuid, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        if a.lien != Lien::Traitant {
            return Err(RefusDemande::Interdit);
        }
        if statut_de(&a.ligne.statut)?.est_final() {
            return Err(RefusDemande::Conflit("Cette demande est terminée."));
        }
        if !a_la_main(&a, est_responsable_de(&mut tx, etab, a.ligne.file_id, a.moi).await?) {
            return Err(RefusDemande::Interdit);
        }
        if rang_conf(&a.ligne.confidentialite) >= 2 {
            return Err(RefusDemande::Invalide("Une demande confidentielle ne se partage pas avec un tiers."));
        }
        let question = c.question.trim();
        if question.is_empty() || question.chars().count() > 1000 || question.contains('\0') {
            return Err(RefusDemande::Invalide("La question est obligatoire (1000 caractères au plus)."));
        }
        if c.delai_jours == 0 || c.delai_jours > 30 {
            return Err(RefusDemande::Invalide("L'échéance de la consultation est de 1 à 30 jours ouvrés."));
        }
        // Le consulté : un agent actif, ni le demandeur, ni la personne visée, ni soi-même.
        let jour = jour_metier(&mut tx, etab).await?;
        let cible: Option<(Uuid,)> = sqlx::query_as(
            "SELECT ag.personne_id FROM ent_agent ag
              WHERE ag.etablissement_id = $1 AND ag.id = $2 AND ag.fusionne_dans_agent_id IS NULL
                AND EXISTS (SELECT 1 FROM ent_agent_engagement e WHERE e.etablissement_id = ag.etablissement_id
                             AND e.agent_id = ag.id AND e.annule_le IS NULL AND e.du <= $3 AND (e.au IS NULL OR e.au >= $3))",
        )
        .bind(etab)
        .bind(c.agent_id)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await?;
        let (cible_personne,) = cible.ok_or(RefusDemande::Invalide("Cette personne ne peut pas être consultée."))?;
        if Some(cible_personne) == a.ligne.demandeur_personne_id
            || Some(cible_personne) == a.ligne.deposant_personne_id
            || Some(c.agent_id) == a.agent
            || est_visee(&mut tx, etab, id, cible_personne).await?
        {
            return Err(RefusDemande::Invalide("Cette personne ne peut pas être consultée."));
        }
        let (cal,): (String,) = sqlx::query_as("SELECT service_calendrier FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(a.ligne.file_id)
            .fetch_one(&mut *tx)
            .await?;
        let echeance = echeance_ouvree(&mut tx, etab, jour, c.delai_jours, &cal).await;
        let cid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_dem_consultation (id, etablissement_id, demande_id, consulte_agent_id, question,
                    partage_demande, echeance, demandee_par_personne_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(cid)
        .bind(etab)
        .bind(id)
        .bind(c.agent_id)
        .bind(question)
        .bind(c.partage_demande)
        .bind(echeance)
        .bind(a.moi)
        .execute(&mut *tx)
        .await?;
        for p in &c.pieces_partagees {
            let ok: (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_dem_piece WHERE etablissement_id = $1 AND demande_id = $2 AND id = $3 AND retire_le IS NULL)",
            )
            .bind(etab)
            .bind(id)
            .bind(p)
            .fetch_one(&mut *tx)
            .await?;
            if !ok.0 {
                return Err(RefusDemande::Invalide("Une des pièces à partager est introuvable."));
            }
            sqlx::query(
                "INSERT INTO ent_dem_consultation_partage (id, etablissement_id, consultation_id, piece_id) VALUES ($1, $2, $3, $4)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(cid)
            .bind(p)
            .execute(&mut *tx)
            .await?;
        }
        let mut f = FaitNouveau::nouveau(id, "consultation_demandee");
        f.auteur_personne = Some(a.moi);
        f.agent_apres = Some(c.agent_id);
        f.ref_id = Some(cid);
        ajouter_fait(&mut tx, etab, f).await?;
        let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, &a.ligne.nature_code, "consultation_demandee");
        publier_enveloppe(&mut tx, etab, types::DEMANDE_CONSULTEE, &env, Some(user_id)).await?;
        tx.commit().await?;
        Ok(cid)
    }

    /// Les consultations ouvertes qui me sont adressées (DEM-213) : identifiant, numéro, échéance. Rien d'autre.
    pub async fn mes_consultations_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<Vec<(Uuid, String, NaiveDate)>, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !role.is_staff() {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await?;
        let l: Vec<(Uuid, String, NaiveDate)> = sqlx::query_as(
            "SELECT c.id, d.numero, c.echeance
               FROM ent_dem_consultation c
               JOIN ent_agent a ON a.etablissement_id = c.etablissement_id AND a.id = c.consulte_agent_id
               JOIN ent_dem_demande d ON d.etablissement_id = c.etablissement_id AND d.id = c.demande_id
              WHERE c.etablissement_id = $1 AND a.personne_id = $2 AND c.repondue_le IS NULL
                AND d.statut NOT IN ('close', 'annulee') AND d.purgee_le IS NULL
              ORDER BY c.echeance, c.id",
        )
        .bind(etab)
        .bind(moi)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(l)
    }

    /// Ce que voit le **consulté** (DEM-207) : la question, l'échéance, l'échange, et — seulement si le traitant l'a
    /// choisi — l'objet et le texte de la demande et les pièces nommées. Une autre personne : 404.
    pub async fn consultation_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        consultation_id: Uuid,
    ) -> Result<ConsultationVue, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        if !role.is_staff() {
            return Err(RefusDemande::Introuvable);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await.map_err(|_| RefusDemande::Introuvable)?;
        #[allow(clippy::type_complexity)]
        let c: Option<(Uuid, String, bool, NaiveDate, Option<DateTime<Utc>>, String, Option<String>, Option<String>, bool, String)> =
            sqlx::query_as(
                "SELECT c.demande_id, c.question, c.partage_demande, c.echeance, c.repondue_le,
                        d.numero, d.objet, d.texte, d.texte_scelle, d.statut
                   FROM ent_dem_consultation c
                   JOIN ent_agent a ON a.etablissement_id = c.etablissement_id AND a.id = c.consulte_agent_id
                   JOIN ent_dem_demande d ON d.etablissement_id = c.etablissement_id AND d.id = c.demande_id
                  WHERE c.etablissement_id = $1 AND c.id = $2 AND a.personne_id = $3 AND d.purgee_le IS NULL",
            )
            .bind(etab)
            .bind(consultation_id)
            .bind(moi)
            .fetch_optional(&mut *tx)
            .await?;
        let (demande_id, question, partage, echeance, repondue, numero, objet, texte, scelle, _) =
            c.ok_or(RefusDemande::Introuvable)?;
        let (objet, texte) = if partage { (objet, ouvrir_texte(demande_id, texte, scelle)?) } else { (None, None) };
        #[allow(clippy::type_complexity)]
        let ms: Vec<(Uuid, DateTime<Utc>, Uuid, String, String, bool, String)> = sqlx::query_as(
            "SELECT id, cree_le, auteur_personne_id, genre, texte, texte_scelle, visibilite
               FROM ent_dem_message WHERE etablissement_id = $1 AND consultation_id = $2 AND masque_le IS NULL
              ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(consultation_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut messages = Vec::new();
        for (mid, le, auteur, genre, t, s, _) in ms {
            let t = if s {
                ChampsSensibles::depuis_env().ouvrir("ent_dem_message", "texte", mid, &t).map_err(|_| RefusDemande::Indisponible)?
            } else {
                t
            };
            let libelle = if auteur == moi { "Vous".to_string() } else { prenom_initiale(&mut tx, etab, auteur).await? };
            messages.push(MessageVu {
                id: mid,
                le,
                auteur: libelle,
                du_service: auteur != moi,
                fil: "consultation",
                genre,
                texte: Some(t),
                masque: false,
                pieces: Vec::new(),
            });
        }
        let ps: Vec<(Uuid, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT p.id, p.role, p.cree_le FROM ent_dem_consultation_partage s
               JOIN ent_dem_piece p ON p.etablissement_id = s.etablissement_id AND p.id = s.piece_id
              WHERE s.etablissement_id = $1 AND s.consultation_id = $2 AND p.retire_le IS NULL
              ORDER BY p.cree_le, p.id",
        )
        .bind(etab)
        .bind(consultation_id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ConsultationVue {
            id: consultation_id,
            numero,
            objet,
            texte,
            question,
            echeance,
            repondue: repondue.is_some(),
            messages,
            pieces: ps
                .into_iter()
                .map(|(id, role, le)| PieceVue { id, role, au_demandeur: false, deposee_par_moi: false, retiree: false, le })
                .collect(),
        })
    }

    /// Répond à une consultation (DEM-207) : un échange de consultation, jamais visible du demandeur. La première
    /// réponse date la consultation. Seul le consulté répond ; une autre personne : 404.
    pub async fn repondre_consultation_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        consultation_id: Uuid,
        texte: &str,
    ) -> Result<MessageEnvoye, RefusDemande> {
        self.limiter("demandes.echange", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (demande_id, consulte): (Uuid, Option<Uuid>) = sqlx::query_as(
            "SELECT demande_id, consulte_agent_id FROM ent_dem_consultation WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(consultation_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(RefusDemande::Introuvable)?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, demande_id, true).await?;
        // Seul le consulté répond : un membre de la file écrit dans l'échange, il ne répond pas à sa place.
        if consulte.is_none() || consulte != a.agent {
            return Err(RefusDemande::Introuvable);
        }
        let e = Ecriture { fil: Fil::Consultation(consultation_id), texte, pieces: &[], corrige: None, complement: false };
        let r = ecrire_tx(&mut tx, etab, user_id, &DocumentsAbsents, demande_id, &a, e).await?;
        let n = sqlx::query(
            "UPDATE ent_dem_consultation SET repondue_le = now() WHERE etablissement_id = $1 AND id = $2 AND repondue_le IS NULL",
        )
        .bind(etab)
        .bind(consultation_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n == 1 {
            let mut f = FaitNouveau::nouveau(demande_id, "consultation_repondue");
            f.auteur_personne = Some(a.moi);
            f.ref_id = Some(consultation_id);
            ajouter_fait(&mut tx, etab, f).await?;
        }
        tx.commit().await?;
        Ok(r)
    }

    // ───────────── Désignation d'une personne visée (DEM-360, R-DM47) ─────────────

    /// Les membres du service que le demandeur peut désigner comme « concernés » (DEM-360) : un libellé, jamais plus.
    /// Réservé au demandeur d'une demande C1 et plus ; sans motif demandé.
    pub async fn membres_designables_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
    ) -> Result<Vec<MembreDesignable>, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, false).await?;
        if a.lien != Lien::Demandeur || a.ligne.confidentialite == "C0" {
            return Err(RefusDemande::Interdit);
        }
        let jour = jour_metier(&mut tx, etab).await?;
        let g = charger_graphe(&mut tx, etab, jour).await?;
        let deja = vises_de(&mut tx, etab, id).await?;
        let mut out = Vec::new();
        for m in g.membres.get(&a.ligne.file_id).into_iter().flatten() {
            if m.personne_id == a.moi || deja.contains(&m.personne_id) {
                continue;
            }
            out.push(MembreDesignable { personne_id: m.personne_id, libelle: prenom_initiale(&mut tx, etab, m.personne_id).await? });
        }
        tx.commit().await?;
        Ok(out)
    }

    /// Le demandeur indique « cette demande concerne une personne de ce service » (DEM-360). Effets immédiats et
    /// définitifs : la personne ne **voit** plus la demande, ne la **prend** plus, ne **reçoit** plus aucun avis
    /// (garde d'accès) ; si elle la portait, la demande retourne dans la file ; si plus personne du service ne peut la
    /// traiter, elle passe à la file de repli, puis à la direction (R-DM47, CDM-57 à CDM-59). Aucun motif demandé,
    /// aucune trace lisible de la personne visée.
    #[allow(clippy::too_many_arguments)]
    pub async fn designer_personne_visee_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
        personne_id: Uuid,
    ) -> Result<EtatDemande, RefusDemande> {
        let (mut tx, a, _) = self.debut_geste(etab, user_id, role, sources, id, version_attendue).await?;
        if a.lien != Lien::Demandeur || a.ligne.confidentialite == "C0" {
            return Err(RefusDemande::Interdit);
        }
        if personne_id == a.moi {
            return Err(RefusDemande::Invalide("Choisissez une personne du service."));
        }
        let jour = jour_metier(&mut tx, etab).await?;
        let g = charger_graphe(&mut tx, etab, jour).await?;
        let membre = g
            .membres
            .get(&a.ligne.file_id)
            .and_then(|ms| ms.iter().find(|m| m.personne_id == personne_id))
            .cloned()
            .ok_or(RefusDemande::Invalide("Cette personne n'est pas dans le service qui traite la demande."))?;
        if est_visee(&mut tx, etab, id, personne_id).await? {
            return Err(RefusDemande::Conflit("Cette personne est déjà désignée."));
        }
        sqlx::query(
            "INSERT INTO ent_dem_personne_visee (id, etablissement_id, demande_id, personne_id, designee_par_personne_id)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(personne_id)
        .bind(a.moi)
        .execute(&mut *tx)
        .await?;
        let vises = vises_de(&mut tx, etab, id).await?;
        // La personne visée ne porte plus la demande ; elle retourne dans la file (statut « reçue »).
        let porteur_vise = a.ligne.attribuee_a_agent_id.is_some() && a.ligne.attribuee_a_agent_id == membre.agent_id;
        let (urgence, site): (String, Option<Uuid>) =
            sqlx::query_as("SELECT urgence, site_id FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        let r = router_demande_existante(
            &mut tx,
            etab,
            jour,
            a.ligne.demandeur_personne_id,
            &a.ligne.nature_code,
            site,
            a.ligne.eleve_id,
            urgence_de(&urgence),
            &vises,
        )
        .await?;
        let res = r.resolution.ok_or(RefusDemande::Conflit("Aucune personne du service ne peut traiter cette demande."))?;
        let file_change = res.file.id != a.ligne.file_id;
        let statut = statut_de(&a.ligne.statut)?;
        let (nouveau_statut, retire_prise) = if file_change || porteur_vise {
            (
                match statut {
                    Statut::EnAttenteDemandeur => Statut::EnAttenteDemandeur,
                    _ => Statut::Deposee,
                },
                true,
            )
        } else {
            (statut, false)
        };
        let n = sqlx::query(
            "UPDATE ent_dem_demande
                SET file_id = $4,
                    attribuee_a_agent_id = CASE WHEN $5 THEN NULL ELSE attribuee_a_agent_id END,
                    attribuee_le = CASE WHEN $5 THEN NULL ELSE attribuee_le END,
                    statut = $6, version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(a.ligne.version)
        .bind(res.file.id)
        .bind(retire_prise)
        .bind(nouveau_statut.code())
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        // Le fait ne nomme pas la personne visée : il dit seulement qu'une désignation a eu lieu (R-DM47).
        let mut f = FaitNouveau::nouveau(id, "personne_visee_designee");
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some(&a.ligne.statut);
        f.statut_apres = Some(nouveau_statut.code());
        f.file_avant = Some(a.ligne.file_id);
        f.file_apres = Some(res.file.id);
        ajouter_fait(&mut tx, etab, f).await?;
        if file_change {
            let mut r2 = FaitNouveau::nouveau(id, "routage");
            r2.auteur_tache = Some("routage_deterministe");
            r2.file_avant = Some(a.ligne.file_id);
            r2.file_apres = Some(res.file.id);
            r2.motif = Some(res.motif);
            ajouter_fait(&mut tx, etab, r2).await?;
            let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, &a.ligne.nature_code, "transferee");
            publier_enveloppe(&mut tx, etab, types::DEMANDE_TRANSFEREE, &env, Some(user_id)).await?;
        } else if retire_prise {
            let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, &a.ligne.nature_code, "reaffectee");
            publier_enveloppe(&mut tx, etab, types::DEMANDE_REAFFECTEE, &env, Some(user_id)).await?;
        }
        tx.commit().await?;
        Ok(EtatDemande { id, numero: a.ligne.numero, statut: nouveau_statut.code().into(), version: a.ligne.version + 1 })
    }

    // ───────────── Requalification ─────────────

    /// Requalifie la nature d'une demande (DEM-153, CDM-01, CDM-11) : le délai, la file et la confidentialité suivent
    /// la nouvelle nature. Par celui qui a la main. Une demande ne devient **jamais moins confidentielle** ; en
    /// devenant C2+, ses textes déjà écrits sont **scellés** et elle quitte la file (l'ancienne file ne la voit plus
    /// ; l'historique de lecture reste). Le demandeur n'est averti que si le délai change.
    #[allow(clippy::too_many_arguments)]
    pub async fn requalifier_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
        nouvelle_nature: &str,
    ) -> Result<Requalification, RefusDemande> {
        let (mut tx, a, _) = self.debut_geste(etab, user_id, role, sources, id, version_attendue).await?;
        if a.lien != Lien::Traitant {
            return Err(RefusDemande::Interdit);
        }
        if !a_la_main(&a, est_responsable_de(&mut tx, etab, a.ligne.file_id, a.moi).await?) {
            return Err(RefusDemande::Interdit);
        }
        let decl = nature(nouvelle_nature).ok_or(RefusDemande::Invalide("Cette démarche n'existe pas."))?;
        if decl.code == a.ligne.nature_code {
            return Err(RefusDemande::Invalide("La demande est déjà de cette nature."));
        }
        let nouvelle_conf = decl.confidentialite.code();
        if rang_conf(nouvelle_conf) < rang_conf(&a.ligne.confidentialite) {
            return Err(RefusDemande::Invalide("Une demande ne devient jamais moins confidentielle."));
        }
        let jour = jour_metier(&mut tx, etab).await?;
        let (urgence, site, texte, texte_scelle, depose_jour): (String, Option<Uuid>, Option<String>, bool, Option<NaiveDate>) =
            sqlx::query_as(
                "SELECT urgence, site_id, texte, texte_scelle,
                        CASE WHEN depose_le IS NULL THEN NULL ELSE ent_aujourdhui_etab($1, depose_le) END
                   FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        let vises = vises_de(&mut tx, etab, id).await?;
        let urgence = urgence_de(&urgence);
        let r = router_demande_existante(
            &mut tx,
            etab,
            jour,
            a.ligne.demandeur_personne_id,
            decl.code,
            site,
            a.ligne.eleve_id,
            urgence,
            &vises,
        )
        .await?;
        if !r.proposee {
            return Err(RefusDemande::Invalide("Cette démarche n'est pas proposée dans l'établissement."));
        }
        let res = r.resolution.ok_or(RefusDemande::Conflit("Aucune file ne peut recevoir cette demande."))?;
        // Une nature C2+ ne va jamais vers une file non confidentielle (§10.5).
        if rang_conf(nouvelle_conf) >= 2 && res.file.type_ != "confidentielle" {
            return Err(RefusDemande::Conflit("Aucune file confidentielle ne peut recevoir cette demande."));
        }
        let file_change = res.file.id != a.ligne.file_id;

        // Délais : la nouvelle nature, comptés depuis le DÉPÔT (le demandeur ne perd pas de temps pour une erreur de
        // qualification). Les échéances ouvertes sont closes, les nouvelles posées ; une attente de demandeur reste
        // suspendue.
        let statut = statut_de(&a.ligne.statut)?;
        let depart = depose_jour.unwrap_or(jour);
        let nouveau_traitement = echeance_ouvree(&mut tx, etab, depart, res.delai_traitement_jours, &res.file.service_calendrier).await;
        let (ancien_traitement,): (Option<NaiveDate>,) = sqlx::query_as(
            "SELECT max(ent_aujourdhui_etab($1, due_le)) FROM ent_dem_echeance
              WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'traitement' AND traitee_le IS NULL",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let delai_change = ancien_traitement != Some(nouveau_traitement);
        let prise_faite = a.ligne.attribuee_a_agent_id.is_some() && !file_change;

        // Scellement : tout texte déjà écrit en clair devient illisible sans clé dès que la demande est C2+.
        sqlx::query("SELECT set_config('ent.dem_requalif', 'on', true)").execute(&mut *tx).await?;
        let scelle_demande = rang_conf(nouvelle_conf) >= 2 && rang_conf(&a.ligne.confidentialite) < 2;
        let texte_apres = match (&texte, scelle_demande, texte_scelle) {
            (Some(t), true, false) => Some(
                ChampsSensibles::depuis_env().sceller("ent_dem_demande", "texte", id, t).map_err(|_| RefusDemande::Indisponible)?,
            ),
            _ => texte.clone(),
        };
        if scelle_demande {
            let msgs: Vec<(Uuid, String)> = sqlx::query_as(
                "SELECT id, texte FROM ent_dem_message WHERE etablissement_id = $1 AND demande_id = $2 AND NOT texte_scelle",
            )
            .bind(etab)
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;
            for (mid, t) in msgs {
                let s = ChampsSensibles::depuis_env().sceller("ent_dem_message", "texte", mid, &t).map_err(|_| RefusDemande::Indisponible)?;
                sqlx::query(
                    "UPDATE ent_dem_message SET texte = $3, texte_scelle = true WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(mid)
                .bind(s)
                .execute(&mut *tx)
                .await?;
            }
        }
        let nouveau_statut = if file_change {
            match statut {
                Statut::EnAttenteDemandeur => Statut::EnAttenteDemandeur,
                _ => Statut::Deposee,
            }
        } else {
            statut
        };
        let n = sqlx::query(
            "UPDATE ent_dem_demande
                SET nature_code = $4, nature_version = $5, confidentialite = $6, file_id = $7,
                    texte = $8, texte_scelle = ($8::text IS NOT NULL AND ($9 OR texte_scelle)),
                    attribuee_a_agent_id = CASE WHEN $10 THEN NULL ELSE attribuee_a_agent_id END,
                    attribuee_le = CASE WHEN $10 THEN NULL ELSE attribuee_le END,
                    statut = $11, version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(a.ligne.version)
        .bind(decl.code)
        .bind(decl.version)
        .bind(nouvelle_conf)
        .bind(res.file.id)
        .bind(texte_apres)
        .bind(scelle_demande)
        .bind(file_change)
        .bind(nouveau_statut.code())
        .execute(&mut *tx)
        .await?
        .rows_affected();
        sqlx::query("SELECT set_config('ent.dem_requalif', 'off', true)").execute(&mut *tx).await?;
        if n != 1 {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        sqlx::query(
            "UPDATE ent_dem_echeance SET traitee_le = now()
              WHERE etablissement_id = $1 AND demande_id = $2 AND type IN ('prise_en_charge', 'traitement') AND traitee_le IS NULL",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if !prise_faite {
            let due = echeance_ouvree(&mut tx, etab, depart, res.delai_prise_jours, &res.file.service_calendrier).await;
            inserer_echeance(&mut tx, etab, id, "prise_en_charge", due, false).await?;
        }
        inserer_echeance(&mut tx, etab, id, "traitement", nouveau_traitement, nouveau_statut == Statut::EnAttenteDemandeur).await?;

        let mut f = FaitNouveau::nouveau(id, "requalifiee");
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some(&a.ligne.statut);
        f.statut_apres = Some(nouveau_statut.code());
        f.file_avant = Some(a.ligne.file_id);
        f.file_apres = Some(res.file.id);
        f.agent_avant = a.ligne.attribuee_a_agent_id;
        let motif = decl.code.replace('.', "_");
        f.motif = Some(&motif);
        ajouter_fait(&mut tx, etab, f).await?;
        if file_change {
            let mut r2 = FaitNouveau::nouveau(id, "routage");
            r2.auteur_tache = Some("routage_deterministe");
            r2.file_avant = Some(a.ligne.file_id);
            r2.file_apres = Some(res.file.id);
            r2.motif = Some(res.motif);
            r2.ref_id = res.ref_id;
            ajouter_fait(&mut tx, etab, r2).await?;
        }
        let ev = if delai_change { "requalifiee_delai_change" } else { "requalifiee" };
        let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, decl.code, ev);
        publier_enveloppe(&mut tx, etab, types::DEMANDE_REQUALIFIEE, &env, Some(user_id)).await?;
        tx.commit().await?;
        Ok(Requalification {
            etat: EtatDemande { id, numero: a.ligne.numero, statut: nouveau_statut.code().into(), version: a.ligne.version + 1 },
            delai_change,
            file_change,
        })
    }

    // ───────────── « Résolu ? » ─────────────

    /// « Résolu ? Oui / Non » à la clôture (DEM-394, DEM-130, CDM-86). **Non** rouvre la **même** demande dans le délai
    /// de réouverture (SN1-05) ; hors délai, une nouvelle demande est conseillée, liée à l'ancienne. La réponse est
    /// un fait de la demande : **jamais une note d'agent**, jamais rattachée à une personne nommée (§37.5).
    #[allow(clippy::too_many_arguments)]
    pub async fn repondre_resolu_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
        resolu: bool,
    ) -> Result<SuiteResolu, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        if a.lien != Lien::Demandeur {
            return Err(RefusDemande::Interdit);
        }
        if a.ligne.statut != "close" {
            return Err(RefusDemande::Conflit("Cette demande n'est pas terminée."));
        }
        if a.ligne.version != version_attendue {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        let (deja,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2
                              AND type IN ('resolu_oui', 'resolu_non', 'resolu_non_hors_delai'))",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if deja {
            return Err(RefusDemande::Conflit("Vous avez déjà répondu."));
        }
        let (issue, jours): (Option<String>, i64) = sqlx::query_as(
            "SELECT issue, (ent_aujourdhui_etab($1, now()) - ent_aujourdhui_etab($1, clos_le))::bigint
               FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let _ = issue;
        if resolu {
            let mut f = FaitNouveau::nouveau(id, "resolu_oui");
            f.auteur_personne = Some(a.moi);
            ajouter_fait(&mut tx, etab, f).await?;
            tx.commit().await?;
            return Ok(SuiteResolu::Merci);
        }
        if jours > DELAI_REOUVERTURE_JOURS {
            let mut f = FaitNouveau::nouveau(id, "resolu_non_hors_delai");
            f.auteur_personne = Some(a.moi);
            ajouter_fait(&mut tx, etab, f).await?;
            tx.commit().await?;
            return Ok(SuiteResolu::HorsDelai { ancienne_demande: id });
        }
        // Réouverture : même demande, plus de prise (n'importe quel membre la reprend), délais repartis.
        let decl = nature(&a.ligne.nature_code).ok_or(RefusDemande::Indisponible)?;
        let jour = jour_metier(&mut tx, etab).await?;
        let (cal, vise): (String, Option<i16>) =
            sqlx::query_as("SELECT service_calendrier, delai_vise_jours FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(a.ligne.file_id)
                .fetch_one(&mut *tx)
                .await?;
        let prise = decl.delai_prise_jours;
        let traitement = vise.and_then(|v| u16::try_from(v).ok()).map_or(decl.delai_traitement_jours, |v| v.min(decl.delai_traitement_jours)).max(prise);
        let n = sqlx::query(
            "UPDATE ent_dem_demande
                SET statut = 'rouverte', clos_le = NULL, issue = NULL, decision = NULL, conservation_jusqu_au = NULL,
                    attribuee_a_agent_id = NULL, attribuee_le = NULL, version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(a.ligne.version)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        for (type_, j) in [("prise_en_charge", prise), ("traitement", traitement)] {
            let due = echeance_ouvree(&mut tx, etab, jour, j, &cal).await;
            inserer_echeance(&mut tx, etab, id, type_, due, false).await?;
        }
        let mut f = FaitNouveau::nouveau(id, "resolu_non");
        f.auteur_personne = Some(a.moi);
        ajouter_fait(&mut tx, etab, f).await?;
        let mut f = FaitNouveau::nouveau(id, "rouverte");
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some("close");
        f.statut_apres = Some("rouverte");
        f.motif = Some("non_resolu");
        ajouter_fait(&mut tx, etab, f).await?;
        let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, &a.ligne.nature_code, "rouverte");
        publier_enveloppe(&mut tx, etab, types::DEMANDE_ROUVERTE, &env, Some(user_id)).await?;
        tx.commit().await?;
        Ok(SuiteResolu::Rouverte(EtatDemande { id, numero: a.ligne.numero, statut: "rouverte".into(), version: a.ligne.version + 1 }))
    }

    // ───────────── Les preuves du demandeur ─────────────

    /// Les **trois preuves** et l'état en mots simples (DEM-126, DEM-393, R-DM53, R-DM54). Réservé au demandeur, à son
    /// déposant et à ses co-demandeurs ; ne rend **aucun identifiant** de file, d'agent, de règle (NJ-82). Le service
    /// est nommé par son rôle ; le prénom et l'initiale n'apparaissent qu'à la **prise**, et jamais pour une file
    /// « service seul » ou confidentielle. Un retard est toujours dit, avec l'information que le responsable est
    /// prévenu quand c'est le cas.
    pub async fn preuves_demandeur_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
    ) -> Result<PreuvesDemandeur, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, false).await?;
        if !matches!(a.lien, Lien::Demandeur | Lien::Deposant | Lien::CoDemandeur) {
            return Err(RefusDemande::Introuvable);
        }
        let (depose_le, attribuee_le): (Option<DateTime<Utc>>, Option<DateTime<Utc>>) = sqlx::query_as(
            "SELECT depose_le, attribuee_le FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let sp = service_public(&mut tx, etab, a.ligne.file_id, &a.ligne.confidentialite).await?;
        let vue: Option<(DateTime<Utc>,)> = sqlx::query_as(
            "SELECT le FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'vue_par_service'
                AND file_apres_id = $3 ORDER BY rang LIMIT 1",
        )
        .bind(etab)
        .bind(id)
        .bind(a.ligne.file_id)
        .fetch_optional(&mut *tx)
        .await?;
        let delai: Option<(DateTime<Utc>,)> = sqlx::query_as(
            "SELECT due_le FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'traitement'
              ORDER BY cree_le DESC LIMIT 1",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let statut = statut_de(&a.ligne.statut)?;
        let en_cours = matches!(
            statut,
            Statut::Prise | Statut::EnAttenteTiers | Statut::EnValidation | Statut::Traitee | Statut::Rouverte
        );
        // Prise : seulement si quelqu'un porte la demande et qu'elle est en traitement.
        let prise = match (a.ligne.attribuee_a_agent_id, attribuee_le) {
            (Some(agent), Some(le)) if en_cours || statut == Statut::EnAttenteDemandeur => {
                let (personne,): (Uuid,) =
                    sqlx::query_as("SELECT personne_id FROM ent_agent WHERE etablissement_id = $1 AND id = $2")
                        .bind(etab)
                        .bind(agent)
                        .fetch_one(&mut *tx)
                        .await?;
                let (par, nominatif) = if sp.nominatif {
                    (prenom_initiale(&mut tx, etab, personne).await?, true)
                } else {
                    (sp.qui(), false)
                };
                Some(PrisePar { le, par, nominatif })
            }
            _ => None,
        };
        let etat = match statut {
            Statut::Brouillon => EtatSimple::Brouillon,
            Statut::Annulee => EtatSimple::Annulee,
            Statut::Close | Statut::Traitee => EtatSimple::Terminee,
            Statut::EnAttenteDemandeur => EtatSimple::AttenteReponse,
            Statut::Deposee if vue.is_none() => EtatSimple::Envoyee,
            Statut::Deposee => EtatSimple::Vue,
            _ => EtatSimple::EnCours,
        };
        let qui = sp.qui();
        let etat_libelle = match etat {
            EtatSimple::Brouillon => "brouillon".to_string(),
            EtatSimple::Envoyee => "envoyée".to_string(),
            EtatSimple::Vue => format!("vue par {qui}"),
            EtatSimple::EnCours => "en cours".to_string(),
            EtatSimple::AttenteReponse => "on attend votre réponse".to_string(),
            EtatSimple::Terminee => "terminée".to_string(),
            EtatSimple::Annulee => "annulée".to_string(),
        };
        // Retard : jamais caché (R-DM54). Suspendu pendant une attente du demandeur ; sans objet une fois terminé.
        let retard = if matches!(etat, EtatSimple::Envoyee | EtatSimple::Vue | EtatSimple::EnCours) {
            let r: Option<(String, i64)> = sqlx::query_as(
                "SELECT type, (ent_aujourdhui_etab($1, now()) - ent_aujourdhui_etab($1, due_le))::bigint
                   FROM ent_dem_echeance
                  WHERE etablissement_id = $1 AND demande_id = $2 AND type IN ('prise_en_charge', 'traitement')
                    AND traitee_le IS NULL AND suspendue_depuis IS NULL AND due_le < now()
                  ORDER BY due_le LIMIT 1",
            )
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
            match r {
                None => None,
                Some((type_, jours)) => {
                    let (prevenu,): (bool,) = sqlx::query_as(
                        "SELECT EXISTS (SELECT 1 FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2
                                          AND type = 'retard_signale' AND motif_code = $3)",
                    )
                    .bind(etab)
                    .bind(id)
                    .bind(type_)
                    .fetch_one(&mut *tx)
                    .await?;
                    Some(RetardMontre { jours: jours.max(0), responsable_prevenu: prevenu })
                }
            }
        } else {
            None
        };
        tx.commit().await?;
        Ok(PreuvesDemandeur {
            etat,
            etat_libelle,
            envoyee: PreuveEnvoyee {
                le: depose_le.unwrap_or_else(Utc::now),
                service: sp.service.clone(),
                role: sp.role.clone(),
                delai_indicatif_le: delai.map(|(d,)| d),
            },
            vue: vue.map(|(le,)| PreuveVue { le, service: sp.qui() }),
            prise,
            retard,
        })
    }

    // ───────────── Retards et « vues mais non prises » ─────────────

    /// **Tâche de l'ordonnanceur** (socle 31) : pose un fait `retard_signale` par demande et par échéance dépassée
    /// (prise en charge ou traitement), une seule fois (index unique : rejouable sans doublon), et publie l'avis à
    /// enveloppe neutre au responsable du service (R-DM54). Ne touche pas aux demandes en attente du demandeur
    /// (décompte suspendu), terminées ou purgées.
    pub async fn signaler_retards(&self, etab: Uuid) -> Result<RapportRetards, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let l: Vec<(Uuid, String, String, String, Uuid)> = sqlx::query_as(
            "SELECT e.demande_id, e.type, d.numero, d.nature_code, d.file_id
               FROM ent_dem_echeance e
               JOIN ent_dem_demande d ON d.etablissement_id = e.etablissement_id AND d.id = e.demande_id
              WHERE e.etablissement_id = $1 AND e.type IN ('prise_en_charge', 'traitement')
                AND e.traitee_le IS NULL AND e.suspendue_depuis IS NULL AND e.due_le < now()
                AND d.purgee_le IS NULL AND d.statut NOT IN ('brouillon', 'close', 'annulee', 'en_attente_demandeur')
                AND NOT EXISTS (SELECT 1 FROM ent_dem_fait f WHERE f.etablissement_id = e.etablissement_id
                                  AND f.demande_id = e.demande_id AND f.type = 'retard_signale' AND f.motif_code = e.type)
              ORDER BY e.due_le, e.id LIMIT 500",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await?;
        let mut rapport = RapportRetards::default();
        for (demande, type_, numero, nature_code, file) in l {
            let mut f = FaitNouveau::nouveau(demande, "retard_signale");
            f.auteur_tache = Some("ordonnanceur_echeances");
            f.file_apres = Some(file);
            f.motif = Some(&type_);
            ajouter_fait(&mut tx, etab, f).await?;
            let env = EnveloppeNeutre::nouvelle(demande, &numero, &nature_code, "en_retard");
            publier_enveloppe(&mut tx, etab, types::DEMANDE_EN_RETARD, &env, None).await?;
            rapport.signalees += 1;
        }
        tx.commit().await?;
        Ok(rapport)
    }

    /// Les demandes **vues mais non prises** depuis au moins `jours` jours (DEM-399, tableau des retards) : pour le
    /// responsable de la file (la sienne) et pour la direction (`demandes.file.superviser`, toutes les files). Titres
    /// neutres : la nature d'une demande C2+ n'est pas rendue (R-DM15).
    pub async fn vues_non_prises_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        jours: i64,
    ) -> Result<Vec<VueNonPrise>, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !role.is_staff() || !(0..=365).contains(&jours) {
            return Err(RefusDemande::Interdit);
        }
        let superviseur = self.a_action_etablissement(etab, user_id, role.as_str(), Action::DemandesFileSuperviser).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let g = charger_graphe(&mut tx, etab, jour).await?;
        let files: Vec<Uuid> =
            if superviseur { g.files.keys().copied().collect() } else { fichiers_dirigees(&g, moi, sources.files_de(etab, moi)) };
        let l: Vec<(Uuid, String, String, String, Uuid, DateTime<Utc>)> = sqlx::query_as(
            "SELECT d.id, d.numero, d.nature_code, d.confidentialite, d.file_id, f.le
               FROM ent_dem_demande d
               JOIN ent_dem_fait f ON f.etablissement_id = d.etablissement_id AND f.demande_id = d.id
                                  AND f.type = 'vue_par_service' AND f.file_apres_id = d.file_id
              WHERE d.etablissement_id = $1 AND d.file_id = ANY($2) AND d.attribuee_a_agent_id IS NULL
                AND d.statut IN ('deposee', 'rouverte') AND d.purgee_le IS NULL
                AND (ent_aujourdhui_etab($1, now()) - ent_aujourdhui_etab($1, f.le)) >= $3
              ORDER BY f.le, d.id",
        )
        .bind(etab)
        .bind(&files)
        .bind(jours as i32)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(l.into_iter()
            .map(|(id, numero, nature, conf, file_id, vue_le)| VueNonPrise {
                id,
                numero,
                nature_code: matches!(conf.as_str(), "C0" | "C1").then_some(nature),
                file_id,
                vue_le,
            })
            .collect())
    }
}

fn urgence_de(s: &str) -> Urgence {
    match s {
        "securite" => Urgence::Securite,
        "haute" => Urgence::Haute,
        _ => Urgence::Normale,
    }
}

/// Les files que cette personne **dirige** (responsable), parmi celles dont elle est membre.
fn fichiers_dirigees(g: &crate::demandes_files::Graphe, moi: Uuid, mes_files: Vec<Uuid>) -> Vec<Uuid> {
    mes_files
        .into_iter()
        .filter(|f| g.membres.get(f).is_some_and(|ms| ms.iter().any(|m| m.personne_id == moi && m.responsable)))
        .collect()
}

/// Un motif codé parmi une liste fermée ; `None` accepté quand le motif est facultatif.
fn motif_code(m: Option<&str>, permis: &[&'static str]) -> Result<Option<&'static str>, RefusDemande> {
    match m {
        None => Ok(None),
        Some(m) => permis.iter().find(|p| **p == m).map(|p| Some(*p)).ok_or(RefusDemande::Invalide("Motif inconnu.")),
    }
}
