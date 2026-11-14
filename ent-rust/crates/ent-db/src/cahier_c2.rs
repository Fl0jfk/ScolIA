//! Cahier de textes, lot C2 : saisie et publication (entrées par cours et par séance).
//!
//! Source de vérité : `ent_cdt_entree` et ses versions (migrations 2402 et 2413). Jour et heure **de
//! l'établissement** partout : la plage calme (R-C5) et la programmation passent par `ent_cdt_visible_a`
//! et `ent_cdt_instant_local` (SQL), jamais par l'heure UTC ni par l'heure du serveur.
//!
//! **Projection transitoire** : les lecteurs du cahier (agenda, élève, famille, remises, charge) lisent encore
//! les tables de 150 (`ent_seance_contenu`, `ent_travail`) jusqu'à C3/C5. Une entrée ou un travail
//! **publié et visible** (publication échue, plage calme passée) est donc projeté dans l'ancienne table sous
//! le même identifiant (`ancien_id` = identifiant projeté) ; avant, rien n'y figure : brouillons, programmations
//! et publications différées sont invisibles. La projection est paresseuse (`cdt_projeter_echus`), appelée avant
//! chaque lecture des anciens lecteurs. Les avis (événements de l'outbox) partent à la projection, donc jamais
//! pendant la plage calme.
//!
//! **Écritures de l'ancien modèle** : les anciennes routes écrivent encore l'ancienne table ; avant toute
//! modification d'une ligne projetée, C2 relit l'ancienne ligne si elle est plus récente (`rafraichir_*`), pour
//! ne jamais écraser une correction faite par une ancienne route.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ent_auth::Action;
use ent_events::types;
use ent_types::cahier_c2::*;
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::cahier::publier_cdt;
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

pub(crate) fn refus(nature: RefusMetier, message: &'static str) -> EntDbError {
    EntDbError::Refus(nature, message)
}
pub(crate) fn invalide(message: &'static str) -> EntDbError {
    refus(RefusMetier::Invalide, message)
}
pub(crate) fn conflit(message: &'static str) -> EntDbError {
    refus(RefusMetier::Conflit, message)
}

/// Texte de remplacement servi aux anciens lecteurs quand une entrée est retirée (l'élève qui l'avait vue lit ceci).
const TEXTE_RETIRE: &str = "Contenu retiré par son auteur.";

/// R-C22 : balisage maison, aucun HTML stocké. Refuse une balise HTML (`<b>`, `</p>`, `<!--`) ; `a < b` reste permis.
pub(crate) fn contient_html(texte: &str) -> bool {
    let c: Vec<char> = texte.chars().collect();
    for (i, ch) in c.iter().enumerate() {
        if *ch == '<' {
            if let Some(n) = c.get(i + 1) {
                if (n.is_ascii_alphabetic() || *n == '/' || *n == '!') && c[i + 1..].contains(&'>') {
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn texte_borne(
    texte: &str,
    max: usize,
    obligatoire: bool,
    message: &'static str,
) -> Result<String, EntDbError> {
    let t = texte.trim();
    if (obligatoire && t.is_empty()) || t.chars().count() > max {
        return Err(invalide(message));
    }
    if contient_html(t) {
        return Err(invalide("le balisage HTML n'est pas accepté : utilisez le texte simple (titres, listes, liens https)"));
    }
    Ok(t.to_string())
}

#[derive(Debug, FromRow)]
pub(crate) struct CoursRow {
    pub id: Uuid,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub annee_id: Uuid,
}

#[derive(Debug, FromRow)]
pub(crate) struct SeanceInfo {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub statut: String,
    pub annee_id: Uuid,
    pub cours_id: Option<Uuid>,
    pub debut_le: DateTime<Utc>,
    pub fin_le: DateTime<Utc>,
    pub remplacant_user_id: Option<Uuid>,
}

pub(crate) const SELECT_SEANCE_INFO: &str = "SELECT s.id, s.jour, s.heure_debut, s.heure_fin, s.statut, s.annee_id,
       ent_cdt_cours_de_seance(s.etablissement_id, s.id) AS cours_id,
       ent_cdt_instant_local(s.etablissement_id, s.jour, s.heure_debut) AS debut_le,
       ent_cdt_instant_local(s.etablissement_id, s.jour, s.heure_fin) AS fin_le,
       s.remplacant_user_id
  FROM ent_edt_seance s
 WHERE s.etablissement_id = $1";

#[derive(Debug, FromRow)]
struct EntreeRow {
    id: Uuid,
    cours_id: Uuid,
    seance_id: Uuid,
    titre: Option<String>,
    contenu: String,
    etat: String,
    publie_a: Option<DateTime<Utc>>,
    visible_a: Option<DateTime<Utc>>,
    retire_motif: Option<String>,
    auteur_user_id: Option<Uuid>,
    version: i32,
    renseignee_le: Option<DateTime<Utc>>,
    ancien_id: Option<Uuid>,
    copie_de_id: Option<Uuid>,
    modifie_le: DateTime<Utc>,
    auteur: String,
    visible_jour: Option<NaiveDate>,
    visible_heure: Option<String>,
}

const SELECT_ENTREE: &str = "SELECT e.id, e.cours_id, e.seance_id, e.titre, e.contenu, e.etat, e.publie_a, e.visible_a,
       e.retire_motif, e.auteur_user_id, e.version, e.renseignee_le, e.ancien_id, e.copie_de_id, e.modifie_le,
       ent_cdt_nom_affichage(e.etablissement_id, e.auteur_user_id) AS auteur,
       CASE WHEN e.visible_a IS NOT NULL THEN ent_aujourdhui_etab(e.etablissement_id, e.visible_a) END AS visible_jour,
       CASE WHEN e.visible_a IS NOT NULL
            THEN to_char(ent_cdt_heure_locale(e.etablissement_id, e.visible_a), 'HH24:MI') END AS visible_heure
  FROM ent_cdt_entree e
 WHERE e.etablissement_id = $1";

#[derive(Debug, FromRow)]
struct ContributionRow {
    id: Uuid,
    auteur: String,
    contenu: String,
    version: i32,
    cree_le: DateTime<Utc>,
}

const SELECT_CONTRIBUTION: &str = "SELECT c.id,
       COALESCE(c.libelle_externe,
                (SELECT NULLIF(btrim(coalesce(pe.prenom, '') || ' ' || coalesce(pe.nom, '')), '')
                   FROM ent_agent ag
                   JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
                  WHERE ag.etablissement_id = c.etablissement_id AND ag.id = c.auteur_agent_id),
                'Intervenant') AS auteur,
       c.contenu, c.version, c.cree_le
  FROM ent_cdt_contribution c
 WHERE c.etablissement_id = $1 AND c.retiree_le IS NULL";

impl EntreeRow {
    fn dto(self, notions: Vec<String>, contributions: Vec<ContributionDto>) -> EntreeDto {
        let differee = matches!(self.etat.as_str(), "publiee" | "programmee")
            && matches!((self.visible_a, self.publie_a), (Some(v), Some(p)) if v > p);
        EntreeDto {
            id: self.id,
            cours_id: self.cours_id,
            seance_id: self.seance_id,
            titre: self.titre,
            contenu: self.contenu,
            notions,
            etat: self.etat,
            publie_a: self.publie_a,
            visible_a: self.visible_a,
            visible_jour: self.visible_jour,
            visible_heure: self.visible_heure,
            differee_par_plage_calme: differee,
            auteur: self.auteur,
            version: self.version,
            modifie_le: self.modifie_le,
            renseignee_le: self.renseignee_le,
            retire_motif: self.retire_motif,
            copie_de_id: self.copie_de_id,
            contributions,
        }
    }
}

/// Texte servi aux anciens lecteurs : titre, contenu, notions et contributions signées.
fn texte_projete(e: &EntreeRow, notions: &[String], contributions: &[ContributionDto]) -> String {
    if e.etat == "retiree" {
        return TEXTE_RETIRE.to_string();
    }
    let mut t = String::new();
    if let Some(titre) = e.titre.as_deref().filter(|x| !x.is_empty()) {
        t.push_str(titre);
        t.push_str("\n\n");
    }
    t.push_str(&e.contenu);
    if !notions.is_empty() {
        t.push_str("\n\nNotions : ");
        t.push_str(&notions.join(", "));
    }
    for c in contributions {
        t.push_str(&format!("\n\n— {} : {}", c.auteur, c.contenu));
    }
    t
}

async fn notions_de(tx: &mut PgConnection, etab: Uuid, entree: Uuid) -> Result<Vec<String>, EntDbError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT libelle FROM ent_cdt_entree_notion WHERE etablissement_id = $1 AND entree_id = $2 ORDER BY ordre",
    )
    .bind(etab)
    .bind(entree)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(|(l,)| l).collect())
}

async fn contributions_de(tx: &mut PgConnection, etab: Uuid, entree: Uuid) -> Result<Vec<ContributionDto>, EntDbError> {
    let rows: Vec<ContributionRow> =
        sqlx::query_as(&format!("{SELECT_CONTRIBUTION} AND c.entree_id = $2 ORDER BY c.cree_le"))
            .bind(etab)
            .bind(entree)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| ContributionDto { id: r.id, auteur: r.auteur, contenu: r.contenu, version: r.version, cree_le: r.cree_le })
        .collect())
}

async fn entree_dto(tx: &mut PgConnection, etab: Uuid, row: EntreeRow) -> Result<EntreeDto, EntDbError> {
    let notions = notions_de(tx, etab, row.id).await?;
    let contributions = contributions_de(tx, etab, row.id).await?;
    Ok(row.dto(notions, contributions))
}

/// Projette l'entrée dans `ent_seance_contenu` (anciens lecteurs). Renvoie vrai si elle y figure (sinon la séance
/// porte déjà le contenu d'un autre cours : limite documentée, l'entrée reste lisible par le client C2).
async fn miroir_entree(tx: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<bool, EntDbError> {
    let Some(e): Option<EntreeRow> = sqlx::query_as(&format!("{SELECT_ENTREE} AND e.id = $2"))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
    else {
        return Err(introuvable());
    };
    let notions = notions_de(tx, etab, id).await?;
    let contributions = contributions_de(tx, etab, id).await?;
    let texte = texte_projete(&e, &notions, &contributions);
    let mid = e.ancien_id.unwrap_or(e.id);
    let auteur = e.auteur_user_id.unwrap_or_default();
    let maj = sqlx::query(
        "UPDATE ent_seance_contenu SET contenu = $3, ressources = '', version = $4, updated_at = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(mid)
    .bind(&texte)
    .bind(e.version)
    .execute(&mut *tx)
    .await
    .map_err(db)?
    .rows_affected();
    if maj == 0 {
        let ins = sqlx::query(
            "INSERT INTO ent_seance_contenu (id, etablissement_id, seance_id, auteur_user_id, contenu, ressources, version)
             VALUES ($1, $2, $3, $4, $5, '', $6)
             ON CONFLICT (etablissement_id, seance_id) DO NOTHING",
        )
        .bind(mid)
        .bind(etab)
        .bind(e.seance_id)
        .bind(auteur)
        .bind(&texte)
        .bind(e.version)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if ins == 0 {
            return Ok(false);
        }
    }
    if e.ancien_id.is_none() {
        sqlx::query("UPDATE ent_cdt_entree SET ancien_id = id WHERE etablissement_id = $1 AND id = $2 AND ancien_id IS NULL")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
    }
    Ok(true)
}

/// Relit l'ancienne ligne si une ancienne route l'a modifiée depuis la dernière écriture de C2.
async fn rafraichir_entree(tx: &mut PgConnection, a: &Acteur<'_>, e: &EntreeRow) -> Result<(), EntDbError> {
    let Some(mid) = e.ancien_id else { return Ok(()) };
    let ancienne: Option<(String, i32, DateTime<Utc>)> = sqlx::query_as(
        "SELECT contenu, version, updated_at FROM ent_seance_contenu WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(a.etablissement_id)
    .bind(mid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?;
    let Some((contenu, _v, maj)) = ancienne else { return Ok(()) };
    if maj <= e.modifie_le || contenu == e.contenu {
        return Ok(());
    }
    // Une ancienne route a écrit : on garde l'état précédent en version, puis on adopte le texte de l'ancienne ligne.
    sqlx::query(
        "INSERT INTO ent_cdt_entree_version (id, etablissement_id, entree_id, version, titre, contenu, etat,
                auteur_user_id, auteur_agent_id, motif)
         SELECT gen_random_uuid(), e.etablissement_id, e.id, e.version, e.titre, e.contenu, e.etat,
                e.auteur_user_id, e.auteur_agent_id, 'reprise d''une modification faite par l''ancienne route'
           FROM ent_cdt_entree e WHERE e.etablissement_id = $1 AND e.id = $2",
    )
    .bind(a.etablissement_id)
    .bind(e.id)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_cdt_entree SET contenu = $3, titre = NULL, version = version + 1, modifie_le = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(a.etablissement_id)
    .bind(e.id)
    .bind(contenu)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Instant de programmation demandé : instant explicite, ou jour et heure **de l'établissement** (7 h par défaut).
pub(crate) async fn instant_programme(
    tx: &mut PgConnection,
    etab: Uuid,
    publie_a: Option<DateTime<Utc>>,
    jour: Option<NaiveDate>,
    heure: Option<NaiveTime>,
) -> Result<Option<DateTime<Utc>>, EntDbError> {
    match (publie_a, jour) {
        (Some(_), Some(_)) => Err(invalide("indiquez soit un instant, soit un jour et une heure de programmation")),
        (Some(p), None) => Ok(Some(p)),
        (None, Some(j)) => {
            let h = heure.unwrap_or_else(|| NaiveTime::from_hms_opt(7, 0, 0).unwrap_or_default());
            let (i,): (DateTime<Utc>,) = sqlx::query_as("SELECT ent_cdt_instant_local($1, $2, $3)")
                .bind(etab)
                .bind(j)
                .bind(h)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            Ok(Some(i))
        }
        (None, None) => Ok(None),
    }
}

fn acteur_auteur(etab: Uuid, user: Option<Uuid>) -> Acteur<'static> {
    Acteur { etablissement_id: etab, user_id: user.unwrap_or_default(), role: "prof" }
}

impl PgEnt {
    /// Projette les publications échues de l'établissement (entrées et travaux) dans les anciennes tables et émet
    /// les événements d'avis, une seule fois, à l'heure où elles deviennent visibles (plage calme passée).
    pub(crate) async fn cdt_projeter_echus(&self, etab: Uuid) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let entrees: Vec<(Uuid, Option<Uuid>, Uuid, String)> = sqlx::query_as(
            "SELECT id, auteur_user_id, seance_id, etat FROM ent_cdt_entree
              WHERE etablissement_id = $1 AND ancien_id IS NULL AND etat IN ('programmee', 'publiee')
                AND visible_a <= now()
              ORDER BY visible_a LIMIT 200 FOR UPDATE SKIP LOCKED",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (id, auteur, seance, _etat) in entrees {
            sqlx::query("UPDATE ent_cdt_entree SET etat = 'publiee' WHERE etablissement_id = $1 AND id = $2 AND etat = 'programmee'")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            if miroir_entree(&mut tx, etab, id).await? {
                publier_cdt(
                    &mut tx,
                    &acteur_auteur(etab, auteur),
                    types::CAHIER_SEANCE_RENSEIGNEE,
                    id,
                    None,
                    json!({ "entree_id": id, "seance_id": seance, "publication": "echue" }),
                )
                .await?;
            }
        }
        self.cdt_projeter_travaux_echus(&mut tx, etab).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Droit de saisie dans un cours **à la date de l'action** (R-C2) : intervention en saisie (socle 6) avec
    /// `cahier.ecrire`, ou remplaçant désigné **de cette séance** seulement.
    pub(crate) async fn cdt_exiger_saisie(
        &self,
        a: &Acteur<'_>,
        cours_id: Uuid,
        seance_remplacee: Option<&SeanceInfo>,
    ) -> Result<(), EntDbError> {
        if !self.cahier_droit_ecrire_global(a).await? {
            return Err(EntDbError::Forbidden);
        }
        if let Some(s) = seance_remplacee {
            if s.remplacant_user_id == Some(a.user_id) && s.statut != "annulee" && s.cours_id == Some(cours_id) {
                return Ok(());
            }
        }
        let jour = self.jour_etab(a.etablissement_id).await?;
        let (ok,): (bool,) = sqlx::query_as("SELECT ent_cdt_saisie_cours($1, $2, $3, $4)")
            .bind(a.etablissement_id)
            .bind(cours_id)
            .bind(a.user_id)
            .bind(jour)
            .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
            .await
            .map_err(db)?;
        if ok {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Le compte porte-t-il `action` à portée d'établissement (direction) ?
    async fn cdt_droit_etab(&self, a: &Acteur<'_>, action: Action) -> Result<bool, EntDbError> {
        Ok(self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?
            .iter()
            .any(|d| d.action == action && d.portee == ent_auth::Portee::Etablissement))
    }

    /// Le compte porte-t-il `action`, à n'importe quelle portée ? (la portée réelle se contrôle à part)
    async fn cdt_droit_quelconque(&self, a: &Acteur<'_>, action: Action) -> Result<bool, EntDbError> {
        Ok(self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?
            .iter()
            .any(|d| d.action == action))
    }

    pub(crate) async fn cdt_cours(&self, etab: Uuid, cours_id: Uuid) -> Result<CoursRow, EntDbError> {
        sqlx::query_as::<_, CoursRow>(
            "SELECT c.id, c.classe_id, c.groupe_id, c.annee_id
               FROM ent_cours c
              WHERE c.etablissement_id = $1 AND c.id = $2",
        )
        .bind(etab)
        .bind(cours_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    pub(crate) async fn cdt_seance(&self, etab: Uuid, seance_id: Uuid) -> Result<SeanceInfo, EntDbError> {
        sqlx::query_as::<_, SeanceInfo>(&format!("{SELECT_SEANCE_INFO} AND s.id = $2"))
            .bind(etab)
            .bind(seance_id)
            .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)
    }

    /// Une année close ne se modifie plus (R-C28) : lecture seule, archive.
    pub(crate) async fn cdt_exiger_annee_ouverte(&self, etab: Uuid, annee_id: Uuid) -> Result<(), EntDbError> {
        let (close,): (bool,) = sqlx::query_as(
            "SELECT cloturee_le IS NOT NULL FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(annee_id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        if close {
            return Err(conflit("l'année scolaire est close : le cahier n'est plus modifiable"));
        }
        Ok(())
    }

    /// Cours dans lesquels l'acteur écrit aujourd'hui (jour de l'établissement).
    pub async fn cdt_cours_saisie(&self, a: &Acteur<'_>) -> Result<Vec<CoursSaisieDto>, EntDbError> {
        if !self.cahier_droit_ecrire_global(a).await? {
            return Err(EntDbError::Forbidden);
        }
        let jour = self.jour_etab(a.etablissement_id).await?;
        let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT c.id, c.libelle, COALESCE(cl.code, g.code, '')
               FROM ent_cours c
               LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
               LEFT JOIN ent_groupe g ON g.etablissement_id = c.etablissement_id AND g.id = c.groupe_id
              WHERE c.etablissement_id = $1 AND c.ouvert_le <= $3 AND (c.ferme_le IS NULL OR c.ferme_le >= $3)
                AND ent_cdt_saisie_cours($1, c.id, $2, $3)
              ORDER BY c.libelle, 3",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .bind(jour)
        .fetch_all(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(cours_id, libelle, cible)| CoursSaisieDto { cours_id, libelle, cible })
            .collect())
    }

    /// Natures de travail proposées (préréglage de l'établissement, créé à la demande).
    pub async fn cdt_natures(&self, a: &Acteur<'_>) -> Result<Vec<NatureTravailDto>, EntDbError> {
        if !self.cahier_droit_ecrire_global(a).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        sqlx::query("SELECT ent_cdt_assurer_natures($1)")
            .bind(a.etablissement_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let rows: Vec<(String, String, Option<i16>)> = sqlx::query_as(
            "SELECT code, libelle, duree_defaut_min FROM ent_cdt_nature
              WHERE etablissement_id = $1 AND archivee_le IS NULL ORDER BY ordre, libelle",
        )
        .bind(a.etablissement_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(code, libelle, duree_defaut_min)| NatureTravailDto { code, libelle, duree_defaut_min })
            .collect())
    }

    /// Écran 1 : séances d'un cours sur une période, avec entrée et travaux. Brouillons et programmations ne sont
    /// vus que des intervenants en saisie ; la direction lit le publié et le retiré (jamais le brouillon).
    pub async fn cdt_seances_du_cours(
        &self,
        a: &Acteur<'_>,
        cours_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceDuCoursDto>, EntDbError> {
        if au < du || (au - du).num_days() > 100 {
            return Err(invalide("période invalide (100 jours au plus)"));
        }
        let cours = self.cdt_cours(a.etablissement_id, cours_id).await?;
        let saisie = self.cdt_exiger_saisie(a, cours_id, None).await.is_ok();
        if !saisie {
            let cible = crate::cahier::Cible { classe_id: cours.classe_id, groupe_id: cours.groupe_id };
            if !self.cahier_peut(a, Action::CahierLire, cible).await? {
                return Err(EntDbError::Forbidden);
            }
        }
        self.cdt_projeter_echus(a.etablissement_id).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let seances: Vec<SeanceInfo> = sqlx::query_as(&format!(
            "{SELECT_SEANCE_INFO} AND ent_cdt_cours_de_seance(s.etablissement_id, s.id) = $2
                AND s.jour BETWEEN $3 AND $4 ORDER BY s.jour, s.heure_debut"
        ))
        .bind(a.etablissement_id)
        .bind(cours_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = seances.iter().map(|s| s.id).collect();
        let filtre = if saisie {
            ""
        } else {
            " AND (e.etat = 'retiree' OR (e.etat = 'publiee' AND e.visible_a <= now()))"
        };
        let entrees: Vec<EntreeRow> = sqlx::query_as(&format!(
            "{SELECT_ENTREE} AND e.cours_id = $2 AND e.seance_id = ANY($3){filtre}"
        ))
        .bind(a.etablissement_id)
        .bind(cours_id)
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut entrees_dto: Vec<EntreeDto> = Vec::new();
        for e in entrees {
            entrees_dto.push(entree_dto(&mut tx, a.etablissement_id, e).await?);
        }
        let filtre_t = if saisie { "" } else { " AND t.etat IN ('publie', 'retire') AND t.visible_a <= now()" };
        let travaux: Vec<(Uuid, Option<Uuid>, Option<Uuid>, String, String, String, String, NaiveDate, String, String)> =
            sqlx::query_as(&format!(
                "SELECT t.id, t.seance_donnee_id, t.echeance_seance_id, t.titre, n.libelle, t.etat,
                        t.public, ent_aujourdhui_etab(t.etablissement_id, t.echeance_le),
                        to_char(ent_cdt_heure_locale(t.etablissement_id, t.echeance_le), 'HH24:MI'), t.etat
                   FROM ent_cdt_travail t
                   JOIN ent_cdt_nature n ON n.etablissement_id = t.etablissement_id AND n.id = t.nature_id
                  WHERE t.etablissement_id = $1 AND t.cours_id = $2
                    AND (t.seance_donnee_id = ANY($3) OR t.echeance_seance_id = ANY($3)){filtre_t}
                  ORDER BY t.echeance_le"
            ))
            .bind(a.etablissement_id)
            .bind(cours_id)
            .bind(&ids)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut out = Vec::new();
        for s in seances {
            let entree = entrees_dto.iter().find(|e| e.seance_id == s.id).cloned();
            let resume = |t: &(Uuid, Option<Uuid>, Option<Uuid>, String, String, String, String, NaiveDate, String, String)| {
                TravailResumeDto {
                    id: t.0,
                    titre: t.3.clone(),
                    nature: t.4.clone(),
                    etat: t.5.clone(),
                    public: t.6.clone(),
                    echeance_jour: t.7,
                    echeance_heure: t.8.clone(),
                }
            };
            out.push(SeanceDuCoursDto {
                seance_id: s.id,
                jour: s.jour,
                heure_debut: s.heure_debut.format("%H:%M").to_string(),
                heure_fin: s.heure_fin.format("%H:%M").to_string(),
                statut: s.statut,
                entree,
                travaux_donnes: travaux.iter().filter(|t| t.1 == Some(s.id)).map(resume).collect(),
                travaux_attendus: travaux.iter().filter(|t| t.2 == Some(s.id)).map(resume).collect(),
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// CDT-121, 117, 126, 130 : crée ou modifie l'entrée d'un cours pour une séance (brouillon, publication
    /// immédiate ou programmée), avec versions, notions, plage calme et saisie rétroactive.
    pub async fn cdt_entree_enregistrer(
        &self,
        a: &Acteur<'_>,
        cours_id: Uuid,
        seance_id: Uuid,
        body: EntreeBody,
    ) -> Result<EntreeDto, EntDbError> {
        let contenu = texte_borne(&body.contenu, 20_000, true, "contenu vide ou trop long (20 000 caractères au plus)")?;
        let titre = {
            let t = texte_borne(body.titre.as_deref().unwrap_or(""), 200, false, "titre trop long (200 caractères au plus)")?;
            if t.is_empty() { None } else { Some(t) }
        };
        let motif = {
            let m = texte_borne(body.motif.as_deref().unwrap_or(""), 500, false, "motif trop long")?;
            if m.is_empty() { None } else { Some(m) }
        };
        if body.notions.len() > 20 {
            return Err(invalide("vingt notions au plus"));
        }
        let mut notions = Vec::new();
        for n in &body.notions {
            let n = texte_borne(n, 200, false, "notion trop longue (200 caractères au plus)")?;
            if !n.is_empty() {
                notions.push(n);
            }
        }
        if !matches!(body.publication.as_str(), "brouillon" | "maintenant" | "programmer") {
            return Err(invalide("publication : brouillon, maintenant ou programmer"));
        }
        let cours = self.cdt_cours(a.etablissement_id, cours_id).await?;
        let seance = self.cdt_seance(a.etablissement_id, seance_id).await?;
        if seance.cours_id != Some(cours_id) {
            return Err(invalide("cette séance n'appartient pas à ce cours"));
        }
        if seance.statut == "annulee" {
            return Err(conflit("la séance est annulée"));
        }
        self.cdt_exiger_saisie(a, cours_id, Some(&seance)).await?;
        self.cdt_exiger_annee_ouverte(a.etablissement_id, seance.annee_id).await?;
        let reglage = self.cdt_publication_defaut(a.etablissement_id, seance.annee_id).await?;
        self.cdt_projeter_echus(a.etablissement_id).await?;

        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let (maintenant,): (DateTime<Utc>,) = sqlx::query_as("SELECT now()").fetch_one(&mut *tx).await.map_err(db)?;
        let voulue_explicite =
            instant_programme(&mut tx, a.etablissement_id, body.publie_a, body.publie_jour, body.publie_heure).await?;
        let existante: Option<EntreeRow> = sqlx::query_as(&format!(
            "{SELECT_ENTREE} AND e.cours_id = $2 AND e.seance_id = $3 FOR UPDATE OF e"
        ))
        .bind(a.etablissement_id)
        .bind(cours_id)
        .bind(seance_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(e) = &existante {
            rafraichir_entree(&mut tx, a, e).await?;
        }
        let existante: Option<EntreeRow> = match existante {
            Some(e) => sqlx::query_as(&format!("{SELECT_ENTREE} AND e.id = $2"))
                .bind(a.etablissement_id)
                .bind(e.id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?,
            None => None,
        };
        let deja_publiee = existante.as_ref().is_some_and(|e| e.etat == "publiee");
        if deja_publiee && body.publication == "brouillon" {
            return Err(conflit("une entrée publiée ne redevient pas brouillon : retirez-la de la diffusion"));
        }
        // Publication demandée : instant de publication, puis début de visibilité (plage calme). Une entrée déjà
        // publiée garde sa publication et sa visibilité : modifier ne la masque jamais, même en plage calme.
        let (etat, publie_a, visible_a): (&str, Option<DateTime<Utc>>, Option<DateTime<Utc>>) = if let Some(e) =
            existante.as_ref().filter(|e| e.etat == "publiee")
        {
            ("publiee", e.publie_a, e.visible_a)
        } else {
            let (etat, publie_a): (&str, Option<DateTime<Utc>>) = match body.publication.as_str() {
                "brouillon" => ("brouillon", None),
                "maintenant" => ("publiee", Some(maintenant)),
                _ => {
                    let voulue = voulue_explicite.unwrap_or(match reglage.as_str() {
                        "fin_seance" => seance.fin_le,
                        "immediate" => maintenant,
                        _ => seance.debut_le,
                    });
                    if voulue <= maintenant {
                        ("publiee", Some(maintenant))
                    } else {
                        ("programmee", Some(voulue))
                    }
                }
            };
            let visible_a: Option<DateTime<Utc>> = match publie_a {
                Some(p) => {
                    let (v,): (DateTime<Utc>,) = sqlx::query_as("SELECT ent_cdt_visible_a($1, $2)")
                        .bind(a.etablissement_id)
                        .bind(p)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(db)?;
                    Some(v)
                }
                None => None,
            };
            (etat, publie_a, visible_a)
        };
        let (jour_seance,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, $2)")
            .bind(a.etablissement_id)
            .bind(maintenant)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let retro = seance.jour < jour_seance;
        let agent: Option<Uuid> = sqlx::query_scalar("SELECT ent_cdt_agent_du_compte($1, $2)")
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let (id, premiere_visibilite, modif_visible) = match existante {
            None => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_cdt_entree (id, etablissement_id, cours_id, seance_id, annee_id, titre, contenu, etat,
                            publie_a, visible_a, auteur_user_id, auteur_agent_id, renseignee_le)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                )
                .bind(id)
                .bind(a.etablissement_id)
                .bind(cours_id)
                .bind(seance_id)
                .bind(seance.annee_id)
                .bind(&titre)
                .bind(&contenu)
                .bind(etat)
                .bind(publie_a)
                .bind(visible_a)
                .bind(a.user_id)
                .bind(agent)
                .bind(if retro { Some(maintenant) } else { None })
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                (id, etat == "publiee", false)
            }
            Some(e) => {
                let etait_visible = e.etat == "publiee" && e.visible_a.is_some_and(|v| v <= maintenant);
                if e.etat != "brouillon" {
                    sqlx::query(
                        "INSERT INTO ent_cdt_entree_version (id, etablissement_id, entree_id, version, titre, contenu, etat,
                                auteur_user_id, auteur_agent_id, motif, sans_avis)
                         SELECT gen_random_uuid(), e.etablissement_id, e.id, e.version, e.titre, e.contenu, e.etat,
                                e.auteur_user_id, e.auteur_agent_id, $3, $4
                           FROM ent_cdt_entree e WHERE e.etablissement_id = $1 AND e.id = $2",
                    )
                    .bind(a.etablissement_id)
                    .bind(e.id)
                    .bind(&motif)
                    .bind(body.sans_avis)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                }
                sqlx::query(
                    "UPDATE ent_cdt_entree SET titre = $3, contenu = $4, etat = $5, publie_a = $6, visible_a = $7,
                            retire_motif = NULL, retire_par_user_id = NULL,
                            auteur_user_id = $8, auteur_agent_id = COALESCE($9, auteur_agent_id),
                            version = version + 1, modifie_le = now()
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(e.id)
                .bind(&titre)
                .bind(&contenu)
                .bind(etat)
                .bind(publie_a)
                .bind(visible_a)
                .bind(a.user_id)
                .bind(agent)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                (e.id, !etait_visible && etat == "publiee", etait_visible)
            }
        };
        sqlx::query("DELETE FROM ent_cdt_entree_notion WHERE etablissement_id = $1 AND entree_id = $2")
            .bind(a.etablissement_id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        for (i, n) in notions.iter().enumerate() {
            sqlx::query(
                "INSERT INTO ent_cdt_entree_notion (etablissement_id, entree_id, ordre, libelle) VALUES ($1, $2, $3, $4)",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind((i + 1) as i16)
            .bind(n)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        // Projection et avis : seulement quand les lecteurs peuvent voir (plage calme passée).
        let visible_maintenant = etat == "publiee" && visible_a.is_some_and(|v| v <= maintenant);
        if visible_maintenant && miroir_entree(&mut tx, a.etablissement_id, id).await? {
            if premiere_visibilite {
                publier_cdt(
                    &mut tx,
                    a,
                    types::CAHIER_SEANCE_RENSEIGNEE,
                    id,
                    None,
                    json!({ "entree_id": id, "seance_id": seance_id, "cours_id": cours.id }),
                )
                .await?;
            } else if modif_visible {
                publier_cdt(
                    &mut tx,
                    a,
                    types::CAHIER_SEANCE_MODIFIEE,
                    id,
                    None,
                    json!({ "entree_id": id, "seance_id": seance_id, "avis": !body.sans_avis }),
                )
                .await?;
            }
        } else if etat == "programmee" {
            publier_cdt(
                &mut tx,
                a,
                types::CAHIER_PROGRAMME,
                id,
                None,
                json!({ "entree_id": id, "seance_id": seance_id, "publie_a": publie_a, "visible_a": visible_a }),
            )
            .await?;
        }
        let row: EntreeRow = sqlx::query_as(&format!("{SELECT_ENTREE} AND e.id = $2"))
            .bind(a.etablissement_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let dto = entree_dto(&mut tx, a.etablissement_id, row).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Réglage de publication par défaut de l'année (`debut_seance`, `fin_seance`, `immediate`).
    pub(crate) async fn cdt_publication_defaut(&self, etab: Uuid, annee_id: Uuid) -> Result<String, EntDbError> {
        let r: Option<(String,)> = sqlx::query_as(
            "SELECT publication_defaut FROM ent_cdt_reglage WHERE etablissement_id = $1 AND annee_id = $2",
        )
        .bind(etab)
        .bind(annee_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(r.map(|(p,)| p).unwrap_or_else(|| "debut_seance".to_string()))
    }

    async fn cdt_charger_entree(&self, a: &Acteur<'_>, entree_id: Uuid) -> Result<EntreeRow, EntDbError> {
        sqlx::query_as::<_, EntreeRow>(&format!("{SELECT_ENTREE} AND e.id = $2"))
            .bind(a.etablissement_id)
            .bind(entree_id)
            .fetch_optional(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)
    }

    /// CDT-128 : une entrée (intervenants en saisie du cours ; la direction lit le publié et le retiré).
    pub async fn cdt_entree_lire(&self, a: &Acteur<'_>, entree_id: Uuid) -> Result<EntreeDto, EntDbError> {
        let e = self.cdt_charger_entree(a, entree_id).await?;
        let saisie = self.cdt_exiger_saisie(a, e.cours_id, None).await.is_ok();
        if !saisie {
            let cours = self.cdt_cours(a.etablissement_id, e.cours_id).await?;
            let cible = crate::cahier::Cible { classe_id: cours.classe_id, groupe_id: cours.groupe_id };
            if !self.cahier_peut(a, Action::CahierLire, cible).await? || !matches!(e.etat.as_str(), "publiee" | "retiree") {
                return Err(introuvable());
            }
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        entree_dto(&mut tx, a.etablissement_id, e).await
    }

    /// R-C29 : versions précédentes d'une entrée, lisibles de ses lecteurs (ici : intervenants et direction).
    pub async fn cdt_entree_versions(&self, a: &Acteur<'_>, entree_id: Uuid) -> Result<Vec<VersionDto>, EntDbError> {
        let e = self.cdt_charger_entree(a, entree_id).await?;
        self.cdt_entree_lire(a, e.id).await?;
        let rows: Vec<(i32, Option<String>, String, String, Option<Uuid>, Option<String>, bool, DateTime<Utc>)> = sqlx::query_as(
            "SELECT v.version, v.titre, v.contenu, v.etat, v.auteur_user_id, v.motif, v.sans_avis, v.cree_le
               FROM ent_cdt_entree_version v
              WHERE v.etablissement_id = $1 AND v.entree_id = $2 ORDER BY v.version DESC",
        )
        .bind(a.etablissement_id)
        .bind(entree_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (version, titre, contenu, etat, auteur_id, motif, sans_avis, cree_le) in rows {
            let (auteur,): (String,) = sqlx::query_as("SELECT ent_cdt_nom_affichage($1, $2)")
                .bind(a.etablissement_id)
                .bind(auteur_id)
                .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
                .await
                .map_err(db)?;
            out.push(VersionDto { version, titre, contenu, etat, auteur, motif, sans_avis, cree_le });
        }
        Ok(out)
    }

    /// CDT-127 : retire l'entrée de la diffusion (motif obligatoire, versions gardées). L'auteur ou un intervenant en
    /// saisie (`cahier.retirer`) ; la direction (`cahier.retirer` d'établissement) avec avis à l'auteur.
    pub async fn cdt_entree_retirer(&self, a: &Acteur<'_>, entree_id: Uuid, motif: &str) -> Result<EntreeDto, EntDbError> {
        let motif = texte_borne(motif, 500, true, "indiquez le motif du retrait (3 à 500 caractères)")?;
        if motif.chars().count() < 3 {
            return Err(invalide("indiquez le motif du retrait (3 à 500 caractères)"));
        }
        if !self.cdt_droit_quelconque(a, Action::CahierRetirer).await? {
            return Err(EntDbError::Forbidden);
        }
        let e = self.cdt_charger_entree(a, entree_id).await?;
        let direction = self.cdt_droit_etab(a, Action::CahierRetirer).await?;
        let auteur = e.auteur_user_id == Some(a.user_id);
        if !direction && !auteur && self.cdt_exiger_saisie(a, e.cours_id, None).await.is_err() {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let e: EntreeRow = sqlx::query_as(&format!("{SELECT_ENTREE} AND e.id = $2 FOR UPDATE OF e"))
            .bind(a.etablissement_id)
            .bind(entree_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        if e.etat == "retiree" {
            return Err(conflit("l'entrée est déjà retirée"));
        }
        rafraichir_entree(&mut tx, a, &e).await?;
        let e: EntreeRow = sqlx::query_as(&format!("{SELECT_ENTREE} AND e.id = $2"))
            .bind(a.etablissement_id)
            .bind(entree_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_cdt_entree_version (id, etablissement_id, entree_id, version, titre, contenu, etat,
                    auteur_user_id, auteur_agent_id, motif, sans_avis)
             SELECT gen_random_uuid(), e.etablissement_id, e.id, e.version, e.titre, e.contenu, e.etat,
                    e.auteur_user_id, e.auteur_agent_id, $3, false
               FROM ent_cdt_entree e WHERE e.etablissement_id = $1 AND e.id = $2",
        )
        .bind(a.etablissement_id)
        .bind(entree_id)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_cdt_entree SET etat = 'retiree', retire_motif = $3, retire_par_user_id = $4,
                    version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(entree_id)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if e.ancien_id.is_some() {
            miroir_entree(&mut tx, a.etablissement_id, entree_id).await?;
        }
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_RETIREE,
            entree_id,
            None,
            json!({ "entree_id": entree_id, "seance_id": e.seance_id, "retire_par_direction": direction && !auteur }),
        )
        .await?;
        let row: EntreeRow = sqlx::query_as(&format!("{SELECT_ENTREE} AND e.id = $2"))
            .bind(a.etablissement_id)
            .bind(entree_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let dto = entree_dto(&mut tx, a.etablissement_id, row).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// CDT-118 : copie une entrée vers d'autres cours (une entrée par cours), en brouillon ajustable. Les documents
    /// ne sont pas recopiés par ce lot.
    pub async fn cdt_entree_copier(
        &self,
        a: &Acteur<'_>,
        entree_id: Uuid,
        vers: &[CopieVersBody],
    ) -> Result<Vec<EntreeDto>, EntDbError> {
        if vers.is_empty() || vers.len() > 10 {
            return Err(invalide("indiquez de 1 à 10 destinations"));
        }
        let source = self.cdt_charger_entree(a, entree_id).await?;
        self.cdt_exiger_saisie(a, source.cours_id, None).await?;
        let notions = {
            let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
            notions_de(&mut tx, a.etablissement_id, entree_id).await?
        };
        let mut out = Vec::new();
        for cible in vers {
            if cible.seance_id == source.seance_id {
                return Err(invalide("la destination doit être une autre séance"));
            }
            let seance = self.cdt_seance(a.etablissement_id, cible.seance_id).await?;
            if seance.cours_id != Some(cible.cours_id) {
                return Err(invalide("une destination ne correspond pas au cours indiqué"));
            }
            if seance.statut == "annulee" {
                return Err(conflit("une séance de destination est annulée"));
            }
            self.cdt_exiger_saisie(a, cible.cours_id, Some(&seance)).await?;
            self.cdt_exiger_annee_ouverte(a.etablissement_id, seance.annee_id).await?;
            let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
            let (existe,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_cdt_entree WHERE etablissement_id = $1 AND cours_id = $2 AND seance_id = $3)",
            )
            .bind(a.etablissement_id)
            .bind(cible.cours_id)
            .bind(cible.seance_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if existe {
                return Err(conflit("une destination porte déjà une entrée : modifiez-la plutôt"));
            }
            let id = Uuid::new_v4();
            let agent: Option<Uuid> = sqlx::query_scalar("SELECT ent_cdt_agent_du_compte($1, $2)")
                .bind(a.etablissement_id)
                .bind(a.user_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            sqlx::query(
                "INSERT INTO ent_cdt_entree (id, etablissement_id, cours_id, seance_id, annee_id, titre, contenu, etat,
                        auteur_user_id, auteur_agent_id, copie_de_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 'brouillon', $8, $9, $10)",
            )
            .bind(id)
            .bind(a.etablissement_id)
            .bind(cible.cours_id)
            .bind(cible.seance_id)
            .bind(seance.annee_id)
            .bind(&source.titre)
            .bind(&source.contenu)
            .bind(a.user_id)
            .bind(agent)
            .bind(source.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            for (i, n) in notions.iter().enumerate() {
                sqlx::query("INSERT INTO ent_cdt_entree_notion (etablissement_id, entree_id, ordre, libelle) VALUES ($1, $2, $3, $4)")
                    .bind(a.etablissement_id)
                    .bind(id)
                    .bind((i + 1) as i16)
                    .bind(n)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
            }
            let row: EntreeRow = sqlx::query_as(&format!("{SELECT_ENTREE} AND e.id = $2"))
                .bind(a.etablissement_id)
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            out.push(entree_dto(&mut tx, a.etablissement_id, row).await?);
            tx.commit().await.map_err(db)?;
        }
        Ok(out)
    }

    /// CDT-125 : contribution signée d'un intervenant du cours (assistant, co-intervenant) à une entrée.
    pub async fn cdt_contribuer(&self, a: &Acteur<'_>, entree_id: Uuid, contenu: &str) -> Result<ContributionDto, EntDbError> {
        let contenu = texte_borne(contenu, 8000, true, "contribution vide ou trop longue (8 000 caractères au plus)")?;
        if !self.cdt_droit_quelconque(a, Action::CahierContribuer).await? {
            return Err(EntDbError::Forbidden);
        }
        let e = self.cdt_charger_entree(a, entree_id).await?;
        let jour = self.jour_etab(a.etablissement_id).await?;
        let (intervenant,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM ent_service_enseignement s
                 WHERE s.etablissement_id = $1 AND s.cours_id = $2 AND s.etat IN ('ouvert', 'clos')
                   AND EXISTS (
                        SELECT 1 FROM ent_service_intervenants_a($1, s.id, $4) x
                          JOIN ent_agent ag ON ag.etablissement_id = $1 AND ag.id = x.agent_id
                          JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
                         WHERE pe.user_id = $3 AND (x.droits IS NULL OR 'cahier' = ANY (x.droits))))",
        )
        .bind(a.etablissement_id)
        .bind(e.cours_id)
        .bind(a.user_id)
        .bind(jour)
        .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        if !intervenant {
            return Err(EntDbError::Forbidden);
        }
        if e.etat == "retiree" {
            return Err(conflit("l'entrée est retirée"));
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let agent: Option<Uuid> = sqlx::query_scalar("SELECT ent_cdt_agent_du_compte($1, $2)")
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let Some(agent) = agent else {
            return Err(conflit("aucune fiche d'agent pour ce compte : la contribution ne peut pas être signée"));
        };
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_cdt_contribution (id, etablissement_id, entree_id, auteur_agent_id, contenu) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(entree_id)
        .bind(agent)
        .bind(&contenu)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_cdt_contribution_version (id, etablissement_id, contribution_id, version, contenu) VALUES ($1, $2, $3, 1, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&contenu)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if e.ancien_id.is_some() {
            miroir_entree(&mut tx, a.etablissement_id, entree_id).await?;
        }
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_CONTRIBUTION_AJOUTEE,
            entree_id,
            None,
            json!({ "entree_id": entree_id, "contribution_id": id }),
        )
        .await?;
        let c: ContributionRow = sqlx::query_as(&format!("{SELECT_CONTRIBUTION} AND c.id = $2"))
            .bind(a.etablissement_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(ContributionDto { id: c.id, auteur: c.auteur, contenu: c.contenu, version: c.version, cree_le: c.cree_le })
    }
}
