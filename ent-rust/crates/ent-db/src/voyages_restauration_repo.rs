//! Socle 42, lot V0d : restauration d'une sortie — commandes et lectures
//! (`docs/socles/42_SORTIES_VOYAGES.md` §14 ; NX-38 à NX-40, NX-86 ; VOY-256).
//!
//! * **droits** : le porteur ou `voyages.modifier` (secrétariat, direction) ; un encadrant non porteur,
//!   l'accueil, la comptabilité et les familles ne voient rien de la commande (variantes alimentaires et
//!   noms) — « introuvable » ;
//! * **destinataire** : la boîte de restauration de l'**établissement** (réglage saisi par la direction),
//!   jamais une adresse venue du navigateur (corrige VD-06) ; sans boîte, l'avis reste « sans destinataire » ;
//! * **qui mange** : paniers attribués à des participants ou à des adultes de la sortie, **plafonnés** (jamais
//!   plus que commandé) et **exacts** pour passer la commande ou émettre un avenant (NX-39) ; l'attribution
//!   d'un désisté est ignorée du décompte et un avenant est proposé ;
//! * **sans INE** ni motif ni coordonnée de famille dans l'avis (corrige VD-16) ;
//! * **avenant** « annule et remplace », numéroté, motivé (NX-40) ; avis en ajout seul, contenu figé par la base ;
//! * **concurrence** : chaque écriture porte la révision lue (`rev`).

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ent_auth::Action;
use ent_mail::OutgoingMail;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use crate::voyages::est_terminal;
use crate::voyages_prestataires::adresse_valide;
use crate::voyages_prestataires_repo::{journal_annuaire, nom_etablissement};
use crate::voyages_repo::{
    charger_avec_acces, journaliser, liste_participants, publier, refus, texte_opt, tx, Fait, SortieLigne,
};
use crate::voyages_restauration::{
    categorie_valide, corps_avis, differences, ecarts_paniers, exacte, objet_avis, plafond_respecte, variante_valide,
    Convive, EcartPanier, FaitsAvis, LigneTotal,
};

const TENTATIVES_MAX: i32 = 3;
const LIGNES_MAX: usize = 400;
const EXTERNES_MAX: usize = 100;

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct LigneSaisie {
    pub jour: NaiveDate,
    pub categorie: String,
    #[serde(default)]
    pub variante: Option<String>,
    pub quantite: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExterneSaisie {
    pub jour: NaiveDate,
    pub libelle: String,
    pub nombre: i32,
    #[serde(default)]
    pub variante: Option<String>,
}

/// Remplacement complet des lignes et des convives externes (le reste — attributions — est conservé).
#[derive(Debug, Clone, Deserialize)]
pub struct RepasSaisie {
    /// Révision lue ; absente seulement à la création de la commande.
    #[serde(default)]
    pub rev: Option<i32>,
    #[serde(default)]
    pub livraison_heure: Option<NaiveTime>,
    #[serde(default)]
    pub livraison_lieu: Option<String>,
    pub lignes: Vec<LigneSaisie>,
    #[serde(default)]
    pub externes: Vec<ExterneSaisie>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttributionSaisie {
    pub rev: i32,
    pub jour: NaiveDate,
    #[serde(default)]
    pub participant_id: Option<Uuid>,
    #[serde(default)]
    pub encadrant_id: Option<Uuid>,
    #[serde(default)]
    pub variante: Option<String>,
    #[serde(default)]
    pub etiquette_nominative: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CommandeRepasVue {
    pub id: Uuid,
    pub livraison_heure: Option<NaiveTime>,
    pub livraison_lieu: Option<String>,
    pub numero_avis: i32,
    pub rev: i32,
    pub rev_transmise: i32,
    pub statut: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct LigneRepasVue {
    pub jour: NaiveDate,
    pub categorie: String,
    pub variante: String,
    pub quantite: i32,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ExterneVue {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub libelle: String,
    pub nombre: i32,
    pub variante: String,
}

/// Une attribution : le nom n'est donné qu'à qui gère la commande (jamais d'INE).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AttributionVue {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub participant_id: Option<Uuid>,
    pub encadrant_id: Option<Uuid>,
    pub nom: String,
    pub prenom: String,
    pub variante: String,
    pub etiquette_nominative: bool,
    /// Le participant ne part plus : son panier ne compte pas et un avenant est proposé.
    pub ignoree: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EcartVue {
    pub jour: NaiveDate,
    pub variante: String,
    pub commandes: i32,
    pub attribues: i32,
}

impl From<EcartPanier> for EcartVue {
    fn from(e: EcartPanier) -> Self {
        Self { jour: e.jour, variante: e.variante, commandes: e.commandes, attribues: e.attribues }
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AvisVue {
    pub id: Uuid,
    pub numero: i32,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub type_: String,
    pub objet: String,
    pub motif: Option<String>,
    pub statut: String,
    pub tentatives: i32,
    pub envoye_le: Option<DateTime<Utc>>,
    pub cree_le: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepasVue {
    pub commande: Option<CommandeRepasVue>,
    pub lignes: Vec<LigneRepasVue>,
    pub externes: Vec<ExterneVue>,
    pub attributions: Vec<AttributionVue>,
    pub ecarts: Vec<EcartVue>,
    /// Des changements restent à transmettre à la cuisine (avenant à émettre).
    pub a_transmettre: bool,
    /// Propositions d'avenant, sans nom (un participant qui ne part plus a encore un panier).
    pub propositions: Vec<String>,
    pub avis: Vec<AvisVue>,
    pub boite_configuree: bool,
}

/// Aperçu : exactement l'objet et le corps qui partiraient (R-SV12).
#[derive(Debug, Clone, Serialize)]
pub struct AvisApercu {
    pub objet: String,
    pub corps: String,
    pub numero: i32,
    pub avenant: bool,
    pub boite_configuree: bool,
}

// ───────────────────────── Aides ─────────────────────────

fn sans_acces() -> EntDbError {
    // « Introuvable » : un profil sans droit de gestion ne sait pas que la commande existe (§21.4).
    introuvable()
}

async fn jours_de_la_sortie(conn: &mut PgConnection, s: &SortieLigne) -> Result<(NaiveDate, NaiveDate), EntDbError> {
    sqlx::query_as(
        "SELECT ($1::timestamptz AT TIME ZONE 'Europe/Paris')::date,
                (($2::timestamptz - interval '1 microsecond') AT TIME ZONE 'Europe/Paris')::date",
    )
    .bind(s.debut)
    .bind(s.fin)
    .fetch_one(conn)
    .await
    .map_err(db)
}

async fn commande_de(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    verrou: bool,
) -> Result<Option<CommandeRepasVue>, EntDbError> {
    let sql = if verrou {
        "SELECT id, livraison_heure, livraison_lieu, numero_avis, rev, rev_transmise, statut
           FROM ent_sortie_repas_commande WHERE etablissement_id = $1 AND sortie_id = $2 FOR UPDATE"
    } else {
        "SELECT id, livraison_heure, livraison_lieu, numero_avis, rev, rev_transmise, statut
           FROM ent_sortie_repas_commande WHERE etablissement_id = $1 AND sortie_id = $2"
    };
    sqlx::query_as(sql).bind(etab).bind(sortie).fetch_optional(conn).await.map_err(db)
}

async fn lignes_de(conn: &mut PgConnection, etab: Uuid, commande: Uuid) -> Result<Vec<LigneRepasVue>, EntDbError> {
    sqlx::query_as(
        "SELECT jour, categorie, variante, quantite FROM ent_sortie_repas_ligne
          WHERE etablissement_id = $1 AND commande_id = $2 ORDER BY jour, categorie, variante",
    )
    .bind(etab)
    .bind(commande)
    .fetch_all(conn)
    .await
    .map_err(db)
}

async fn externes_de(conn: &mut PgConnection, etab: Uuid, commande: Uuid) -> Result<Vec<ExterneVue>, EntDbError> {
    sqlx::query_as(
        "SELECT id, jour, libelle, nombre, variante FROM ent_sortie_repas_externe
          WHERE etablissement_id = $1 AND commande_id = $2 ORDER BY jour, libelle",
    )
    .bind(etab)
    .bind(commande)
    .fetch_all(conn)
    .await
    .map_err(db)
}

async fn attributions_de(
    conn: &mut PgConnection,
    etab: Uuid,
    commande: Uuid,
) -> Result<Vec<AttributionVue>, EntDbError> {
    sqlx::query_as(
        "SELECT a.id, a.jour, a.participant_id, a.encadrant_id,
                COALESCE(e.nom, p.nom, '') AS nom, COALESCE(e.prenom, p.prenom, '') AS prenom,
                a.variante, a.etiquette_nominative,
                (sp.id IS NOT NULL AND sp.etat IN ('desiste', 'retire', 'exclu_disciplinaire')) AS ignoree
           FROM ent_sortie_repas_attribution a
           LEFT JOIN ent_sortie_participant sp ON sp.etablissement_id = a.etablissement_id AND sp.id = a.participant_id
           LEFT JOIN ent_eleve e ON e.etablissement_id = sp.etablissement_id AND e.id = sp.eleve_id
           LEFT JOIN ent_sortie_encadrant se ON se.etablissement_id = a.etablissement_id AND se.id = a.encadrant_id
           LEFT JOIN ent_personne p ON p.etablissement_id = se.etablissement_id AND p.id = se.personne_id
          WHERE a.etablissement_id = $1 AND a.commande_id = $2
          ORDER BY a.jour, nom, prenom",
    )
    .bind(etab)
    .bind(commande)
    .fetch_all(conn)
    .await
    .map_err(db)
}

fn totaux(l: &[LigneRepasVue]) -> Vec<LigneTotal> {
    l.iter()
        .map(|x| LigneTotal {
            jour: x.jour,
            categorie: x.categorie.clone(),
            variante: x.variante.clone(),
            quantite: x.quantite,
        })
        .collect()
}

/// Paniers attribués qui comptent : participants qui partent encore, adultes, convives externes.
fn comptes_attribues(att: &[AttributionVue], ext: &[ExterneVue]) -> Vec<(NaiveDate, String, i32)> {
    let mut v: Vec<(NaiveDate, String, i32)> =
        att.iter().filter(|a| !a.ignoree).map(|a| (a.jour, a.variante.clone(), 1)).collect();
    v.extend(ext.iter().map(|e| (e.jour, e.variante.clone(), e.nombre)));
    v
}

async fn ecarts_de(conn: &mut PgConnection, etab: Uuid, commande: Uuid) -> Result<Vec<EcartPanier>, EntDbError> {
    let l = lignes_de(conn, etab, commande).await?;
    let a = attributions_de(conn, etab, commande).await?;
    let e = externes_de(conn, etab, commande).await?;
    Ok(ecarts_paniers(&totaux(&l), &comptes_attribues(&a, &e)))
}

async fn boite_restauration(conn: &mut PgConnection, etab: Uuid) -> Result<Option<String>, EntDbError> {
    let r: Option<Option<String>> = sqlx::query_scalar(
        "SELECT boite_restauration_email FROM ent_sortie_reglage WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(r.flatten())
}

fn garde_gestion(acces: &crate::voyages_repo::Acces) -> Result<(), EntDbError> {
    if acces.gere(Action::VoyagesModifier) {
        Ok(())
    } else if acces.detail() {
        // Il connaît la sortie mais pas la restauration : même réponse qu'une commande inexistante.
        Err(sans_acces())
    } else {
        Err(introuvable())
    }
}

async fn vue_de(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
) -> Result<RepasVue, EntDbError> {
    let boite_configuree = boite_restauration(conn, a.etablissement_id).await?.is_some();
    let Some(cmd) = commande_de(conn, a.etablissement_id, s.id, false).await? else {
        return Ok(RepasVue {
            commande: None,
            lignes: vec![],
            externes: vec![],
            attributions: vec![],
            ecarts: vec![],
            a_transmettre: false,
            propositions: vec![],
            avis: vec![],
            boite_configuree,
        });
    };
    let lignes = lignes_de(conn, a.etablissement_id, cmd.id).await?;
    let externes = externes_de(conn, a.etablissement_id, cmd.id).await?;
    let attributions = attributions_de(conn, a.etablissement_id, cmd.id).await?;
    let ecarts: Vec<EcartVue> =
        ecarts_paniers(&totaux(&lignes), &comptes_attribues(&attributions, &externes)).into_iter().map(Into::into).collect();
    let mut propositions = Vec::new();
    let ignorees = attributions.iter().filter(|x| x.ignoree).count();
    if ignorees > 0 && cmd.statut == "passee" {
        propositions.push(format!(
            "{ignorees} panier(s) restent attribués à des élèves qui ne partent plus : émettez un avenant pour les retirer."
        ));
    }
    let avis = sqlx::query_as::<_, AvisVue>(
        "SELECT id, numero, type, objet, motif, statut, tentatives, envoye_le, cree_le
           FROM ent_sortie_repas_avis WHERE etablissement_id = $1 AND commande_id = $2 ORDER BY numero",
    )
    .bind(a.etablissement_id)
    .bind(cmd.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let a_transmettre = cmd.statut == "passee" && (cmd.rev > cmd.rev_transmise || ignorees > 0);
    Ok(RepasVue {
        commande: Some(cmd),
        lignes,
        externes,
        attributions,
        ecarts,
        a_transmettre,
        propositions,
        avis,
        boite_configuree,
    })
}

/// Compose l'avis (aperçu et envoi partagent ce même rendu : R-SV12).
async fn composer_avis(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    cmd: &CommandeRepasVue,
    avenant: bool,
    motif: Option<&str>,
) -> Result<(String, String, i32), EntDbError> {
    let lignes = totaux(&lignes_de(conn, a.etablissement_id, cmd.id).await?);
    let externes: Vec<(NaiveDate, String, i32, String)> = externes_de(conn, a.etablissement_id, cmd.id)
        .await?
        .into_iter()
        .map(|e| (e.jour, e.variante, e.nombre, e.libelle))
        .collect();
    // Convives nominatifs : participants qui partent encore (classe du jour de la sortie) et adultes.
    let participants = liste_participants(conn, a, s, &[]).await?;
    let mut convives: Vec<Convive> = Vec::new();
    for at in attributions_de(conn, a.etablissement_id, cmd.id).await? {
        if at.ignoree {
            continue;
        }
        let classe = at
            .participant_id
            .and_then(|pid| participants.iter().find(|p| p.id == pid))
            .and_then(|p| p.classe.clone());
        convives.push(Convive {
            jour: at.jour,
            nom: at.nom,
            prenom: at.prenom,
            classe,
            variante: at.variante,
            etiquette: at.etiquette_nominative,
        });
    }
    let numero = if avenant { cmd.numero_avis + 1 } else { 0 };
    let etab = nom_etablissement(conn, a.etablissement_id).await?;
    let corps = corps_avis(&FaitsAvis {
        etablissement: &etab,
        reference: &s.reference,
        titre: &s.titre,
        numero,
        avenant,
        motif,
        livraison_heure: cmd.livraison_heure,
        livraison_lieu: cmd.livraison_lieu.as_deref(),
        lignes: &lignes,
        convives: &convives,
        externes: &externes,
    });
    Ok((objet_avis(&s.reference, numero, avenant), corps, numero))
}

fn valider_saisie(s: &RepasSaisie, du: NaiveDate, au: NaiveDate) -> Result<(), EntDbError> {
    if s.lignes.len() > LIGNES_MAX || s.externes.len() > EXTERNES_MAX {
        return Err(invalide());
    }
    let mut vus = std::collections::HashSet::new();
    for l in &s.lignes {
        let v = l.variante.as_deref().unwrap_or("standard");
        if !categorie_valide(&l.categorie) || !variante_valide(v) || !(0..=2000).contains(&l.quantite) {
            return Err(invalide());
        }
        if l.jour < du || l.jour > au {
            return Err(refus(RefusMetier::Invalide, "Un jour de la commande est hors des dates de la sortie."));
        }
        if !vus.insert((l.jour, l.categorie.clone(), v.to_string())) {
            return Err(refus(RefusMetier::Invalide, "Une ligne de commande est en double."));
        }
    }
    for e in &s.externes {
        let v = e.variante.as_deref().unwrap_or("standard");
        if !variante_valide(v) || !(1..=500).contains(&e.nombre) || e.libelle.trim().is_empty() || e.libelle.chars().count() > 100
        {
            return Err(invalide());
        }
        if e.jour < du || e.jour > au {
            return Err(refus(RefusMetier::Invalide, "Un jour de la commande est hors des dates de la sortie."));
        }
    }
    Ok(())
}

// ───────────────────────── Commandes ─────────────────────────

impl PgEnt {
    /// Lecture de la commande de restauration d'une sortie (porteur ou `voyages.modifier`).
    pub async fn voyage_repas_lire(&self, a: &Acteur<'_>, id: Uuid) -> Result<RepasVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        garde_gestion(&acces)?;
        let v = vue_de(&mut t, a, &s).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// NX-38, NX-86 : écrit (crée ou remplace) les lignes de commande et les convives externes.
    pub async fn voyage_repas_ecrire(&self, a: &Acteur<'_>, id: Uuid, saisie: RepasSaisie) -> Result<RepasVue, EntDbError> {
        let lieu = texte_opt(saisie.livraison_lieu.as_deref(), 300)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gestion(&acces)?;
        if est_terminal(&s.etat) {
            return Err(refus(RefusMetier::Conflit, "Ce dossier n'est plus modifiable."));
        }
        let (du, au) = jours_de_la_sortie(&mut t, &s).await?;
        valider_saisie(&saisie, du, au)?;
        let cmd_id = match commande_de(&mut t, a.etablissement_id, id, true).await? {
            None => {
                if saisie.rev.is_some() {
                    return Err(conflit());
                }
                let cid = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_sortie_repas_commande (id, etablissement_id, sortie_id, cree_par_user_id)
                     VALUES ($1,$2,$3,$4)",
                )
                .bind(cid)
                .bind(a.etablissement_id)
                .bind(id)
                .bind(a.user_id)
                .execute(&mut *t)
                .await
                .map_err(db)?;
                cid
            }
            Some(c) => {
                if saisie.rev != Some(c.rev) || c.statut == "annulee" {
                    return Err(conflit());
                }
                c.id
            }
        };
        sqlx::query("DELETE FROM ent_sortie_repas_ligne WHERE etablissement_id = $1 AND commande_id = $2")
            .bind(a.etablissement_id)
            .bind(cmd_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        sqlx::query("DELETE FROM ent_sortie_repas_externe WHERE etablissement_id = $1 AND commande_id = $2")
            .bind(a.etablissement_id)
            .bind(cmd_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        for l in &saisie.lignes {
            sqlx::query(
                "INSERT INTO ent_sortie_repas_ligne (id, etablissement_id, commande_id, jour, categorie, variante, quantite)
                 VALUES ($1,$2,$3,$4,$5,$6,$7)",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(cmd_id)
            .bind(l.jour)
            .bind(&l.categorie)
            .bind(l.variante.as_deref().unwrap_or("standard"))
            .bind(l.quantite)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        for e in &saisie.externes {
            sqlx::query(
                "INSERT INTO ent_sortie_repas_externe (id, etablissement_id, commande_id, jour, libelle, nombre, variante)
                 VALUES ($1,$2,$3,$4,$5,$6,$7)",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(cmd_id)
            .bind(e.jour)
            .bind(e.libelle.trim())
            .bind(e.nombre)
            .bind(e.variante.as_deref().unwrap_or("standard"))
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        sqlx::query(
            "UPDATE ent_sortie_repas_commande SET rev = rev + 1, livraison_heure = $3, livraison_lieu = $4, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(cmd_id)
        .bind(saisie.livraison_heure)
        .bind(&lieu)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        // Plafond : aucune attribution (nominative ou externe) au-delà de ce qui est commandé.
        if !plafond_respecte(&ecarts_de(&mut t, a.etablissement_id, cmd_id).await?) {
            return Err(refus(
                RefusMetier::Conflit,
                "Des paniers attribués dépassent la quantité commandée : retirez d'abord des attributions.",
            ));
        }
        journaliser(&mut t, a, id, Fait::simple("repas_modifies")).await?;
        let ev = publier(&mut t, a, "voyages.repas_modifie", id, json!({})).await?;
        let v = vue_de(&mut t, a, &s).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// NX-39 : attribue un panier à un participant ou à un adulte de la sortie, pour un jour.
    pub async fn voyage_repas_attribuer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        s: AttributionSaisie,
    ) -> Result<RepasVue, EntDbError> {
        if s.participant_id.is_some() == s.encadrant_id.is_some() {
            return Err(refus(RefusMetier::Invalide, "Un panier est attribué à un participant OU à un adulte."));
        }
        let variante = s.variante.as_deref().unwrap_or("standard");
        if !variante_valide(variante) {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let (so, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gestion(&acces)?;
        if est_terminal(&so.etat) {
            return Err(refus(RefusMetier::Conflit, "Ce dossier n'est plus modifiable."));
        }
        let (du, au) = jours_de_la_sortie(&mut t, &so).await?;
        if s.jour < du || s.jour > au {
            return Err(refus(RefusMetier::Invalide, "Ce jour est hors des dates de la sortie."));
        }
        let cmd = commande_de(&mut t, a.etablissement_id, id, true).await?.ok_or_else(introuvable)?;
        if cmd.rev != s.rev || cmd.statut == "annulee" {
            return Err(conflit());
        }
        if let Some(p) = s.participant_id {
            let ok: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_sortie_participant
                                 WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3
                                   AND etat IN ('inscrit', 'pret', 'bloque', 'parti'))",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind(p)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            if !ok {
                return Err(refus(RefusMetier::Invalide, "Ce participant n'est pas inscrit à la sortie."));
            }
        }
        if let Some(e) = s.encadrant_id {
            let ok: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_sortie_encadrant WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3)",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind(e)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            if !ok {
                return Err(refus(RefusMetier::Invalide, "Cet adulte n'encadre pas la sortie."));
            }
        }
        let r = sqlx::query(
            "INSERT INTO ent_sortie_repas_attribution
                 (id, etablissement_id, commande_id, jour, participant_id, encadrant_id, variante, etiquette_nominative, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(cmd.id)
        .bind(s.jour)
        .bind(s.participant_id)
        .bind(s.encadrant_id)
        .bind(variante)
        .bind(s.etiquette_nominative)
        .bind(a.user_id)
        .execute(&mut *t)
        .await;
        if let Err(e) = r {
            // Déjà attribué ce jour : conflit plutôt qu'erreur technique.
            return Err(match &e {
                sqlx::Error::Database(d) if d.code().as_deref() == Some("23505") => conflit(),
                _ => db(e),
            });
        }
        sqlx::query("UPDATE ent_sortie_repas_commande SET rev = rev + 1, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(cmd.id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        if !plafond_respecte(&ecarts_de(&mut t, a.etablissement_id, cmd.id).await?) {
            return Err(refus(RefusMetier::Conflit, "Tous les paniers commandés ce jour sont déjà attribués."));
        }
        journaliser(&mut t, a, id, Fait::simple("repas_attribue")).await?;
        let v = vue_de(&mut t, a, &so).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Retire une attribution de panier.
    pub async fn voyage_repas_attribution_retirer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        attribution_id: Uuid,
        rev: i32,
    ) -> Result<RepasVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (so, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gestion(&acces)?;
        if est_terminal(&so.etat) {
            return Err(refus(RefusMetier::Conflit, "Ce dossier n'est plus modifiable."));
        }
        let cmd = commande_de(&mut t, a.etablissement_id, id, true).await?.ok_or_else(introuvable)?;
        if cmd.rev != rev {
            return Err(conflit());
        }
        let n = sqlx::query("DELETE FROM ent_sortie_repas_attribution WHERE etablissement_id = $1 AND commande_id = $2 AND id = $3")
            .bind(a.etablissement_id)
            .bind(cmd.id)
            .bind(attribution_id)
            .execute(&mut *t)
            .await
            .map_err(db)?
            .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        sqlx::query("UPDATE ent_sortie_repas_commande SET rev = rev + 1, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(cmd.id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        journaliser(&mut t, a, id, Fait::simple("repas_attribution_retiree")).await?;
        let v = vue_de(&mut t, a, &so).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Aperçu de l'avis qui partirait : même rendu que l'envoi (R-SV12).
    pub async fn voyage_repas_apercu(&self, a: &Acteur<'_>, id: Uuid) -> Result<AvisApercu, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        garde_gestion(&acces)?;
        let cmd = commande_de(&mut t, a.etablissement_id, id, false).await?.ok_or_else(introuvable)?;
        let avenant = cmd.statut == "passee";
        let (objet, corps, numero) = composer_avis(&mut t, a, &s, &cmd, avenant, Some("(à préciser)")).await?;
        let boite_configuree = boite_restauration(&mut t, a.etablissement_id).await?.is_some();
        t.commit().await.map_err(db)?;
        Ok(AvisApercu { objet, corps, numero, avenant, boite_configuree })
    }

    /// NX-38 : passe la commande (premier avis à la cuisine). Les paniers doivent être **exactement**
    /// attribués ; confirmation expresse après aperçu (N3).
    pub async fn voyage_repas_passer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        rev: i32,
        confirme: bool,
    ) -> Result<RepasVue, EntDbError> {
        if !confirme {
            return Err(refus(RefusMetier::Invalide, "Confirmez l'envoi après avoir lu l'aperçu."));
        }
        self.repas_emettre(a, id, rev, false, None).await
    }

    /// NX-40 : avenant « annule et remplace », numéroté et motivé.
    pub async fn voyage_repas_avenant_emettre(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        rev: i32,
        motif: &str,
        confirme: bool,
    ) -> Result<RepasVue, EntDbError> {
        if !confirme {
            return Err(refus(RefusMetier::Invalide, "Confirmez l'envoi après avoir lu l'aperçu."));
        }
        let motif = crate::voyages_repo::texte(motif, 500)?;
        self.repas_emettre(a, id, rev, true, Some(motif)).await
    }

    async fn repas_emettre(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        rev: i32,
        avenant: bool,
        motif: Option<String>,
    ) -> Result<RepasVue, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_gestion(&acces)?;
        if est_terminal(&s.etat) {
            return Err(refus(RefusMetier::Conflit, "Ce dossier n'est plus modifiable."));
        }
        let cmd = commande_de(&mut t, a.etablissement_id, id, true).await?.ok_or_else(introuvable)?;
        if cmd.rev != rev {
            return Err(conflit());
        }
        match (avenant, cmd.statut.as_str()) {
            (false, "brouillon") => {}
            (true, "passee") => {
                let ignorees: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM ent_sortie_repas_attribution a
                       JOIN ent_sortie_participant sp ON sp.etablissement_id = a.etablissement_id AND sp.id = a.participant_id
                      WHERE a.etablissement_id = $1 AND a.commande_id = $2 AND sp.etat IN ('desiste', 'retire', 'exclu_disciplinaire')",
                )
                .bind(a.etablissement_id)
                .bind(cmd.id)
                .fetch_one(&mut *t)
                .await
                .map_err(db)?;
                if cmd.rev <= cmd.rev_transmise && ignorees == 0 {
                    return Err(refus(RefusMetier::Conflit, "Aucun changement à transmettre à la cuisine."));
                }
            }
            _ => return Err(refus(RefusMetier::Conflit, "La commande n'est pas dans l'état attendu pour cet envoi.")),
        }
        let lignes = lignes_de(&mut t, a.etablissement_id, cmd.id).await?;
        if !lignes.iter().any(|l| l.quantite > 0) {
            return Err(refus(RefusMetier::Invalide, "La commande est vide."));
        }
        if !exacte(&ecarts_de(&mut t, a.etablissement_id, cmd.id).await?) {
            return Err(refus(
                RefusMetier::Conflit,
                "Les paniers attribués ne correspondent pas exactement aux paniers commandés.",
            ));
        }
        let (objet, corps, numero) = composer_avis(&mut t, a, &s, &cmd, avenant, motif.as_deref()).await?;
        sqlx::query(
            "INSERT INTO ent_sortie_repas_avis (id, etablissement_id, commande_id, numero, type, objet, corps, motif, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(cmd.id)
        .bind(numero)
        .bind(if avenant { "avenant" } else { "commande" })
        .bind(&objet)
        .bind(&corps)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_sortie_repas_commande SET statut = 'passee', numero_avis = $3, rev_transmise = rev, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(cmd.id)
        .bind(numero)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            id,
            Fait { motif: motif.as_deref(), ..Fait::simple(if avenant { "repas_avenant_emis" } else { "repas_commande_passee" }) },
        )
        .await?;
        let ev = publier(&mut t, a, if avenant { "voyages.repas_avenant" } else { "voyages.repas_commande" }, id, json!({ "numero": numero }))
            .await?;
        let v = vue_de(&mut t, a, &s).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        if let Err(e) = self.voyages_repas_avis_traiter(a.etablissement_id, 20).await {
            tracing::warn!(erreur = %e, "voyages : envoi des avis de restauration reporté");
        }
        Ok(v)
    }

    /// Envoie les avis en file de l'établissement à **sa** boîte de restauration. Un échec n'est jamais
    /// fatal ; après [`TENTATIVES_MAX`] essais, l'avis passe en `echec` (visible). Sans boîte : l'avis
    /// reste « sans destinataire » et repart dès que la direction a réglé la boîte. Retourne le nombre
    /// d'avis partis.
    pub async fn voyages_repas_avis_traiter(&self, etab: Uuid, limite: i64) -> Result<usize, EntDbError> {
        let mut t = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let boite = boite_restauration(&mut t, etab).await?;
        let lignes: Vec<(Uuid, String, String, i32)> = sqlx::query_as(
            "SELECT id, objet, corps, tentatives FROM ent_sortie_repas_avis
              WHERE etablissement_id = $1 AND statut IN ('a_envoyer', 'sans_destinataire')
              ORDER BY cree_le, id LIMIT $2 FOR UPDATE SKIP LOCKED",
        )
        .bind(etab)
        .bind(limite.clamp(1, 100))
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut partis = 0;
        for (avis_id, objet, corps, tentatives) in lignes {
            let statut = match &boite {
                None => "sans_destinataire",
                Some(adr) => {
                    let mut ok = false;
                    if self.mailer().is_ready() {
                        let r = self
                            .mailer()
                            .send(OutgoingMail { to: adr.clone(), subject: objet, body_text: corps, attachments: vec![] })
                            .await;
                        ok = matches!(&r, Ok(rc) if self.envoi_reussi(rc));
                    }
                    if ok {
                        "envoye"
                    } else if tentatives + 1 >= TENTATIVES_MAX {
                        "echec"
                    } else {
                        "a_envoyer"
                    }
                }
            };
            sqlx::query(
                "UPDATE ent_sortie_repas_avis SET statut = $3, tentatives = tentatives + CASE WHEN $4 THEN 1 ELSE 0 END,
                        envoye_le = CASE WHEN $3 = 'envoye' THEN now() ELSE NULL END, modifie_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(avis_id)
            .bind(statut)
            .bind(boite.is_some())
            .execute(&mut *t)
            .await
            .map_err(db)?;
            if statut == "envoye" {
                partis += 1;
            }
        }
        t.commit().await.map_err(db)?;
        Ok(partis)
    }

    /// Réglage : boîte de restauration de l'établissement. **Direction seule** (`voyages.autoriser`) :
    /// c'est la destination des listes nominatives, elle ne se redirige pas à la légère (VD-06).
    pub async fn voyages_reglage_restauration_ecrire(
        &self,
        a: &Acteur<'_>,
        boite: Option<&str>,
    ) -> Result<(), EntDbError> {
        self.voyages_exiger(a, Action::VoyagesAutoriser).await?;
        let boite = match boite.map(str::trim).filter(|x| !x.is_empty()) {
            None => None,
            Some(b) => Some(
                adresse_valide(b)
                    .ok_or_else(|| refus(RefusMetier::Invalide, "Adresse de la boîte de restauration invalide."))?
                    .0,
            ),
        };
        let mut t = tx(self, a).await?;
        sqlx::query(
            "INSERT INTO ent_sortie_reglage (id, etablissement_id, boite_restauration_email, modifie_par_user_id)
             VALUES ($1,$2,$3,$4)
             ON CONFLICT (etablissement_id) DO UPDATE
                SET boite_restauration_email = EXCLUDED.boite_restauration_email, modifie_le = now(),
                    modifie_par_user_id = EXCLUDED.modifie_par_user_id",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(&boite)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journal_annuaire(&mut t, a, "reglage_restauration_modifie", "Boîte de restauration des sorties modifiée").await?;
        t.commit().await.map_err(db)?;
        let _ = self.voyages_repas_avis_traiter(a.etablissement_id, 20).await;
        Ok(())
    }

    /// Droit de direction (`voyages.autoriser`) : déclenchements manuels (envoi des avis, purge) tant qu'aucun
    /// ordonnanceur n'est branché.
    pub async fn voyages_exiger_direction(&self, a: &Acteur<'_>) -> Result<(), EntDbError> {
        self.voyages_exiger(a, Action::VoyagesAutoriser).await
    }

    /// Différences entre deux lectures de la commande (aide à rédiger le motif d'un avenant).
    pub fn repas_differences(avant: &[LigneRepasVue], apres: &[LigneRepasVue]) -> Vec<String> {
        differences(&totaux(avant), &totaux(apres))
    }
}
