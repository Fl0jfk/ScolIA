//! Socle N1, lot N1-1 : **files de demandes**, membres **calculés**, paliers, file de repli RH, réponses types.
//!
//! | Défaut | Garde-fou |
//! |---|---|
//! | NJ-85 | la file de tri a des membres **désignés** ; aucune source « tout le personnel » n'existe (CHECK de 3042) ; une file sans membre actif n'est jamais ouverte à tous : ses demandes passent à la file de repli |
//! | NJ-94 | les membres sont **calculés** à chaque lecture depuis les fonctions, missions et désignations datées du socle 6 ; aucune adresse ni liste d'identifiants ; un départ rend les demandes à la file de repli |
//! | NJ-95 | réponses types à variables **typées** et rendu **échappé** ; aucun balisage dans un modèle |
//!
//! Le jour métier est celui de l'établissement (`ent_aujourdhui_etab`), jamais la date du serveur ni le jour UTC.

use crate::demandes::{personne_du_user, RefusDemande};
use crate::demandes_catalogue::{self as cat, SourceDefaut};
use crate::service::PgEnt;
use crate::structure_calendrier::SERVICES;
use crate::tenant_tx::TenantTx;
use chrono::NaiveDate;
use ent_auth::{Action, Role};
use sqlx::PgConnection;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

// ───────────────────────── Vocabulaire ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeFile {
    Service,
    Tri,
    Direction,
    Confidentielle,
    Repli,
}

impl TypeFile {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::Tri => "tri",
            Self::Direction => "direction",
            Self::Confidentielle => "confidentielle",
            Self::Repli => "repli",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "service" => Self::Service,
            "tri" => Self::Tri,
            "direction" => Self::Direction,
            "confidentielle" => Self::Confidentielle,
            "repli" => Self::Repli,
            _ => return None,
        })
    }
}

/// Création d'une file.
#[derive(Debug, Clone)]
pub struct NouvelleFile {
    pub code: String,
    pub libelle: String,
    /// « le secrétariat des élèves » : ce que le demandeur lit sur son accusé. Sans valeur : le libellé.
    pub role_traitant: Option<String>,
    pub type_: TypeFile,
    pub site_id: Option<Uuid>,
    /// Un service de [`SERVICES`] ; `secretariat` par défaut.
    pub service_calendrier: Option<String>,
    pub delai_vise_jours: Option<i16>,
    pub message_accueil: Option<String>,
    /// Obligatoire sauf pour le repli de dernier recours (R-DM6).
    pub repli_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLigne {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub role_traitant: Option<String>,
    pub type_: String,
    pub site_id: Option<Uuid>,
    pub service_calendrier: String,
    pub delai_vise_jours: Option<i16>,
    pub message_accueil: Option<String>,
    pub repli_id: Option<Uuid>,
}

impl FileLigne {
    /// Ce que le demandeur lit : le rôle de la file, à défaut son libellé.
    pub fn role_lisible(&self) -> &str {
        self.role_traitant.as_deref().unwrap_or(&self.libelle)
    }
}

/// D'où viennent les membres d'une file. **Jamais** « tout le personnel », jamais une adresse (R-DM7, R-DM36).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceMembre {
    /// Agents ayant une mission de ce type (socle 6), éventuellement limitée à un site.
    Mission { type_code: String, site_id: Option<Uuid> },
    /// Agents ayant cette fonction (socle 6).
    Fonction { code: String },
    /// Agent désigné, daté.
    Agent { agent_id: Uuid },
    /// Personne désignée, datée : réservé aux files de repli (président de l'OGEC, personne extérieure).
    Personne { personne_id: Uuid },
}

#[derive(Debug, Clone)]
pub struct RegleMembreSaisie {
    pub source: SourceMembre,
    pub responsable: bool,
    pub rang: i16,
    /// Début de la désignation ; défaut : le jour métier.
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
}

/// Un membre **calculé** à une date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Membre {
    pub personne_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub responsable: bool,
    pub rang: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Avertissement {
    /// La personne désignée est extérieure à la vie interne de l'établissement (aucun engagement actif).
    PersonneExterieure { personne_id: Uuid },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultatRegle {
    /// `false` : rien n'a été écrit, l'avertissement attend une confirmation.
    pub applique: bool,
    pub regle_id: Option<Uuid>,
    pub avertissements: Vec<Avertissement>,
}

/// Le réglage de la file de repli RH, expliqué en clair (Q-DM14).
pub const EXPLICATION_REPLI_RH: &str = "Quand une demande confidentielle concerne une personne qui fait partie de la file RH, \
elle ne lui est jamais envoyée. Elle part à la file de repli choisie ici. Par défaut : l'attaché de gestion et la direction. \
Si un destinataire du repli est lui-même concerné, la demande passe au suivant, puis à la direction. \
Choisir une personne extérieure à la vie interne de l'établissement (par exemple le président de l'OGEC) est possible : \
un avertissement vous le rappelle. Chaque changement est journalisé et le demandeur voit toujours à qui part sa demande.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepliRhVue {
    pub file_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub membres: Vec<Membre>,
    pub explication: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultatRepli {
    pub applique: bool,
    pub avertissements: Vec<Avertissement>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RapportInit {
    pub files_creees: u32,
    pub regles_creees: u32,
    pub reponses_creees: u32,
    pub repli_rh_pose: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReponseTypeLue {
    pub id: Uuid,
    pub libelle: String,
    pub texte: String,
}

// ───────────────────────── Graphe des files et membres calculés ─────────────────────────

/// Les files actives d'un établissement et leurs membres **à une date** : calculé à chaque lecture (R-DM8).
#[derive(Debug, Clone, Default)]
pub struct Graphe {
    pub files: BTreeMap<Uuid, FileLigne>,
    pub membres: BTreeMap<Uuid, Vec<Membre>>,
}

impl Graphe {
    pub fn file_par_code(&self, code: &str) -> Option<&FileLigne> {
        self.files.values().find(|f| f.code == code)
    }

    /// Membres d'une file, hors des personnes visées (R-DM47 : la personne visée n'est jamais destinataire).
    pub fn membres_hors(&self, file: Uuid, vises: &[Uuid]) -> Vec<&Membre> {
        let mut v: Vec<&Membre> =
            self.membres.get(&file).into_iter().flatten().filter(|m| !vises.contains(&m.personne_id)).collect();
        v.sort_by_key(|m| (m.rang, m.personne_id));
        v
    }

    /// Files dont la personne est membre **ou héritière** : une file sans aucun membre actif (départ, fin de
    /// mission) rend ses demandes aux membres de sa file de repli (NJ-94 : jamais de demande orpheline).
    pub fn files_de(&self, personne: Uuid) -> Vec<Uuid> {
        let mut out = BTreeSet::new();
        for (fid, ms) in &self.membres {
            if ms.iter().any(|m| m.personne_id == personne) {
                out.insert(*fid);
            }
        }
        for (fid, f) in &self.files {
            if self.membres.get(fid).is_some_and(|m| !m.is_empty()) {
                continue;
            }
            // Remonte la chaîne de repli jusqu'à la première file qui a des membres (4 sauts au plus).
            let mut cible = f.repli_id;
            for _ in 0..4 {
                let Some(c) = cible else { break };
                match self.membres.get(&c).filter(|m| !m.is_empty()) {
                    Some(ms) => {
                        if ms.iter().any(|m| m.personne_id == personne) {
                            out.insert(*fid);
                        }
                        break;
                    }
                    None => cible = self.files.get(&c).and_then(|x| x.repli_id),
                }
            }
        }
        out.into_iter().collect()
    }
}

pub(crate) async fn jour_metier(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, RefusDemande> {
    let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())").bind(etab).fetch_one(conn).await?;
    Ok(jour)
}

type LigneFile = (Uuid, String, String, Option<String>, String, Option<Uuid>, String, Option<i16>, Option<String>, Option<Uuid>);

fn ligne_file(l: LigneFile) -> FileLigne {
    FileLigne {
        id: l.0,
        code: l.1,
        libelle: l.2,
        role_traitant: l.3,
        type_: l.4,
        site_id: l.5,
        service_calendrier: l.6,
        delai_vise_jours: l.7,
        message_accueil: l.8,
        repli_id: l.9,
    }
}

const SELECT_FILE: &str = "SELECT id, code, libelle, role_traitant, type, site_id, service_calendrier, delai_vise_jours,
                                  message_accueil, file_repli_id FROM ent_dem_file
                            WHERE etablissement_id = $1 AND actif ORDER BY code";

/// Charge les files actives et leurs membres à `jour`.
pub(crate) async fn charger_graphe(conn: &mut PgConnection, etab: Uuid, jour: NaiveDate) -> Result<Graphe, RefusDemande> {
    let files: Vec<LigneFile> =
        sqlx::query_as(SELECT_FILE).bind(etab).fetch_all(&mut *conn).await?;
    let mut g = Graphe::default();
    for l in files {
        let f = ligne_file(l);
        g.membres.insert(f.id, Vec::new());
        g.files.insert(f.id, f);
    }
    let membres: Vec<(Uuid, Uuid, Option<Uuid>, bool, i16)> = sqlx::query_as(MEMBRES_SQL).bind(etab).bind(jour).fetch_all(conn).await?;
    for (file, personne, agent, responsable, rang) in membres {
        if let Some(v) = g.membres.get_mut(&file) {
            v.push(Membre { personne_id: personne, agent_id: agent, responsable, rang });
        }
    }
    Ok(g)
}

/// Les membres calculés de toutes les files actives à la date `$2`. Un agent n'est membre que s'il est **actif**
/// (engagement en cours, non fusionné) ; une personne désignée (repli) n'a pas besoin d'engagement.
const MEMBRES_SQL: &str = "
WITH r AS (
    SELECT r.* FROM ent_dem_file_membre_regle r
     WHERE r.etablissement_id = $1 AND r.annule_le IS NULL AND r.du <= $2 AND (r.au IS NULL OR r.au >= $2)
       AND EXISTS (SELECT 1 FROM ent_dem_file f WHERE f.etablissement_id = r.etablissement_id AND f.id = r.file_id AND f.actif)
), cand AS (
    SELECT r.file_id, m.agent_id, NULL::uuid AS personne_id, r.responsable, r.rang
      FROM r
      JOIN ent_mission_type t ON t.code = r.mission_type_code AND t.actif
           AND (t.etablissement_id IS NULL OR t.etablissement_id = r.etablissement_id)
      JOIN ent_mission m ON m.etablissement_id = r.etablissement_id AND m.type_id = t.id AND m.annule_le IS NULL
           AND m.du <= $2 AND (m.au IS NULL OR m.au >= $2)
     WHERE r.source = 'mission'
       AND (r.site_id IS NULL OR EXISTS (
            SELECT 1 FROM ent_mission_perimetre p
             WHERE p.etablissement_id = m.etablissement_id AND p.mission_id = m.id
               AND (p.nature = 'etablissement' OR (p.nature = 'site' AND p.cible_id = r.site_id))))
    UNION ALL
    SELECT r.file_id, af.agent_id, NULL::uuid, r.responsable, r.rang
      FROM r
      JOIN ent_fonction fo ON fo.code = r.fonction_code AND fo.archivee_le IS NULL
           AND (fo.etablissement_id IS NULL OR fo.etablissement_id = r.etablissement_id)
      JOIN ent_agent_fonction af ON af.etablissement_id = r.etablissement_id AND af.fonction_id = fo.id
           AND af.annule_le IS NULL AND af.du <= $2 AND (af.au IS NULL OR af.au >= $2)
     WHERE r.source = 'fonction'
    UNION ALL
    SELECT r.file_id, r.agent_id, NULL::uuid, r.responsable, r.rang FROM r WHERE r.source = 'agent'
    UNION ALL
    SELECT r.file_id, NULL::uuid, r.personne_id, r.responsable, r.rang FROM r WHERE r.source = 'personne'
)
SELECT c.file_id, COALESCE(a.personne_id, c.personne_id) AS personne_id, a.id AS agent_id,
       COALESCE(bool_or(c.responsable), false) AS responsable, COALESCE(min(c.rang), 1)::smallint AS rang
  FROM cand c
  LEFT JOIN ent_agent a ON a.etablissement_id = $1 AND a.id = c.agent_id
 WHERE (c.agent_id IS NULL OR (
            a.fusionne_dans_agent_id IS NULL
            AND EXISTS (SELECT 1 FROM ent_agent_engagement e
                         WHERE e.etablissement_id = $1 AND e.agent_id = a.id AND e.annule_le IS NULL
                           AND e.du <= $2 AND (e.au IS NULL OR e.au >= $2))))
   AND EXISTS (SELECT 1 FROM ent_personne p WHERE p.etablissement_id = $1
                  AND p.id = COALESCE(a.personne_id, c.personne_id) AND p.fusionnee_dans IS NULL)
 GROUP BY c.file_id, COALESCE(a.personne_id, c.personne_id), a.id";

// ───────────────────────── Réponses types : variables typées, rendu échappé (NJ-95) ─────────────────────────

/// Variables permises dans un modèle. Aucune variable libre : rien d'autre ne peut y entrer.
pub const VARIABLES_REPONSE: &[&str] = &["prenom", "numero", "date", "service"];

/// Valeurs substituées dans un modèle.
#[derive(Debug, Clone, Default)]
pub struct VariablesReponse<'a> {
    pub prenom: &'a str,
    pub numero: &'a str,
    pub date: &'a str,
    pub service: &'a str,
}

/// Découpe un modèle en morceaux : texte brut ou variable. Refuse une variable inconnue, une accolade mal
/// fermée et tout balisage.
fn decouper(modele: &str) -> Result<Vec<(bool, String)>, &'static str> {
    if modele.contains(['<', '>']) {
        return Err("Un modèle de réponse ne contient jamais de balisage.");
    }
    let mut morceaux = Vec::new();
    let mut reste = modele;
    while let Some(i) = reste.find("{{") {
        if i > 0 {
            morceaux.push((false, reste[..i].to_string()));
        }
        let apres = &reste[i + 2..];
        let Some(j) = apres.find("}}") else {
            return Err("Une variable du modèle est mal fermée.");
        };
        let nom = apres[..j].trim();
        if !VARIABLES_REPONSE.contains(&nom) {
            return Err("Ce modèle utilise une variable inconnue.");
        }
        morceaux.push((true, nom.to_string()));
        reste = &apres[j + 2..];
    }
    if reste.contains("}}") {
        return Err("Une variable du modèle est mal ouverte.");
    }
    if !reste.is_empty() {
        morceaux.push((false, reste.to_string()));
    }
    Ok(morceaux)
}

/// Contrôle d'un modèle à l'écriture.
pub fn valider_modele_reponse(texte: &str) -> Result<(), &'static str> {
    if texte.trim().is_empty() || texte.chars().count() > 4000 || texte.contains('\0') {
        return Err("Le texte du modèle est obligatoire (4000 caractères au plus).");
    }
    decouper(texte).map(|_| ())
}

fn valeur<'a>(v: &VariablesReponse<'a>, nom: &str) -> &'a str {
    match nom {
        "prenom" => v.prenom,
        "numero" => v.numero,
        "date" => v.date,
        _ => v.service,
    }
}

/// Échappe pour insertion dans du HTML : toute valeur, sans exception.
pub fn echapper_html(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            c if c.is_control() && c != '\n' => {}
            c => o.push(c),
        }
    }
    o
}

/// Rendu **texte brut** : les variables sont insérées sans caractère de commande.
pub fn rendre_reponse_texte(modele: &str, v: &VariablesReponse<'_>) -> Result<String, &'static str> {
    let mut o = String::new();
    for (est_var, m) in decouper(modele)? {
        if est_var {
            o.extend(valeur(v, &m).chars().filter(|c| !c.is_control() || *c == '\n'));
        } else {
            o.push_str(&m);
        }
    }
    Ok(o)
}

/// Rendu **HTML** : tout est échappé, modèle comme variables (NJ-95).
pub fn rendre_reponse_html(modele: &str, v: &VariablesReponse<'_>) -> Result<String, &'static str> {
    let mut o = String::new();
    for (est_var, m) in decouper(modele)? {
        o.push_str(&echapper_html(if est_var { valeur(v, &m) } else { &m }));
    }
    Ok(o)
}

// ───────────────────────── Aides d'accès et de journal ─────────────────────────

pub(crate) fn code_ok(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub(crate) async fn journaliser(
    conn: &mut PgConnection,
    etab: Uuid,
    sujet: &str,
    cible: &str,
    avant: Option<&str>,
    apres: Option<&str>,
    user: Uuid,
) -> Result<(), RefusDemande> {
    sqlx::query(
        "INSERT INTO ent_dem_reglage_journal (id, etablissement_id, sujet, cible, avant, apres, par_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(sujet)
    .bind(cible)
    .bind(avant)
    .bind(apres)
    .bind(user)
    .execute(conn)
    .await?;
    Ok(())
}

impl PgEnt {
    /// Le droit de paramétrer le guichet (`demandes.parametrer`) : direction et administrateur par défaut.
    pub(crate) async fn exiger_parametrer_demandes(&self, etab: Uuid, user: Uuid, role: &str) -> Result<Role, RefusDemande> {
        let r = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !r.is_staff() || !self.a_action_etablissement(etab, user, r.as_str(), Action::DemandesParametrer).await? {
            return Err(RefusDemande::Interdit);
        }
        Ok(r)
    }

    // ───────────── Files ─────────────

    /// Crée une file. Toute file a un repli (R-DM6) ; celui d'une file confidentielle ou direction est **hors
    /// service** (type `repli`, R-DM47).
    pub async fn creer_file_for_actor(&self, etab: Uuid, user: Uuid, role: &str, n: NouvelleFile) -> Result<Uuid, RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        let libelle = n.libelle.trim();
        if !code_ok(&n.code, 60) || libelle.is_empty() || libelle.chars().count() > 120 {
            return Err(RefusDemande::Invalide("Le code (minuscules, chiffres, _) et le libellé de la file sont obligatoires."));
        }
        if n.role_traitant.as_deref().is_some_and(|r| r.trim().is_empty() || r.chars().count() > 120) {
            return Err(RefusDemande::Invalide("Le rôle de la file est trop long."));
        }
        let service = n.service_calendrier.as_deref().unwrap_or("secretariat");
        if !SERVICES.contains(&service) {
            return Err(RefusDemande::Invalide("Ce calendrier de service n'existe pas."));
        }
        if n.delai_vise_jours.is_some_and(|d| !(1..=90).contains(&d))
            || n.message_accueil.as_deref().is_some_and(|m| m.chars().count() > 500)
        {
            return Err(RefusDemande::Invalide("Le délai visé ou le message d'accueil est hors limites."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let deja: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_dem_file WHERE etablissement_id = $1 AND code = $2").bind(etab).bind(&n.code).fetch_optional(&mut *tx).await?;
        if deja.is_some() {
            return Err(RefusDemande::Conflit("Une file porte déjà ce code."));
        }
        match (n.type_, n.repli_id) {
            (TypeFile::Repli, None) => {}
            (TypeFile::Repli, Some(_)) => return Err(RefusDemande::Invalide("Le repli de dernier recours n'a pas de repli.")),
            (_, None) => return Err(RefusDemande::Invalide("Toute file a une file de repli.")),
            (t, Some(repli)) => {
                let cible: Option<(String,)> =
                    sqlx::query_as("SELECT type FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2 AND actif")
                        .bind(etab)
                        .bind(repli)
                        .fetch_optional(&mut *tx)
                        .await?;
                let Some((type_repli,)) = cible else {
                    return Err(RefusDemande::Invalide("La file de repli est introuvable."));
                };
                if matches!(t, TypeFile::Confidentielle | TypeFile::Direction) && type_repli != "repli" {
                    return Err(RefusDemande::Invalide("Le repli d'une file confidentielle ou de la direction est hors service."));
                }
            }
        }
        if let Some(site) = n.site_id {
            let ok: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_site WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(site).fetch_optional(&mut *tx).await?;
            if ok.is_none() {
                return Err(RefusDemande::Invalide("Ce site n'existe pas."));
            }
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_dem_file (id, etablissement_id, code, libelle, role_traitant, type, site_id, service_calendrier,
                    delai_vise_jours, message_accueil, file_repli_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(etab)
        .bind(&n.code)
        .bind(libelle)
        .bind(n.role_traitant.as_deref().map(str::trim))
        .bind(n.type_.code())
        .bind(n.site_id)
        .bind(service)
        .bind(n.delai_vise_jours)
        .bind(n.message_accueil.as_deref().map(str::trim))
        .bind(n.repli_id)
        .execute(&mut *tx)
        .await?;
        journaliser(&mut tx, etab, "file", &n.code, None, Some(n.type_.code()), user).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Les files actives de l'établissement (paramétrage).
    pub async fn files_for_actor(&self, etab: Uuid, user: Uuid, role: &str) -> Result<Vec<FileLigne>, RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let g = { let j = jour_metier(&mut tx, etab).await?; charger_graphe(&mut tx, etab, j) }.await?;
        tx.commit().await?;
        Ok(g.files.into_values().collect())
    }

    // ───────────── Membres calculés ─────────────

    /// Ajoute une règle de membres. Une personne **extérieure à la vie interne** n'est désignée qu'après
    /// avertissement confirmé (Q-DM14) ; une personne désignée directement n'est admise que dans une file de repli.
    pub async fn ajouter_regle_membre_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        file_id: Uuid,
        saisie: RegleMembreSaisie,
        confirme_exterieur: bool,
    ) -> Result<ResultatRegle, RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        if !(1..=99).contains(&saisie.rang) {
            return Err(RefusDemande::Invalide("Le rang est compris entre 1 et 99."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let du = saisie.du.unwrap_or(jour);
        if saisie.au.is_some_and(|a| a < du) {
            return Err(RefusDemande::Invalide("La date de fin précède la date de début."));
        }
        let f: Option<(String, String)> =
            sqlx::query_as("SELECT type, code FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2 AND actif")
                .bind(etab)
                .bind(file_id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some((type_file, code_file)) = f else { return Err(RefusDemande::Introuvable) };

        let (mut mission, mut fonction, mut agent, mut personne, mut site) = (None, None, None, None, None);
        let source = match &saisie.source {
            SourceMembre::Mission { type_code, site_id } => {
                let ok: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM ent_mission_type WHERE code = $2 AND actif AND (etablissement_id IS NULL OR etablissement_id = $1) LIMIT 1",
                )
                .bind(etab)
                .bind(type_code)
                .fetch_optional(&mut *tx)
                .await?;
                if ok.is_none() {
                    return Err(RefusDemande::Invalide("Ce type de mission n'existe pas."));
                }
                mission = Some(type_code.clone());
                site = *site_id;
                "mission"
            }
            SourceMembre::Fonction { code } => {
                let ok: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM ent_fonction WHERE code = $2 AND archivee_le IS NULL AND (etablissement_id IS NULL OR etablissement_id = $1) LIMIT 1",
                )
                .bind(etab)
                .bind(code)
                .fetch_optional(&mut *tx)
                .await?;
                if ok.is_none() {
                    return Err(RefusDemande::Invalide("Cette fonction n'existe pas."));
                }
                fonction = Some(code.clone());
                "fonction"
            }
            SourceMembre::Agent { agent_id } => {
                let actif: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT a.id FROM ent_agent a
                      WHERE a.etablissement_id = $1 AND a.id = $2 AND a.fusionne_dans_agent_id IS NULL
                        AND EXISTS (SELECT 1 FROM ent_agent_engagement e WHERE e.etablissement_id = a.etablissement_id
                                     AND e.agent_id = a.id AND e.annule_le IS NULL
                                     AND e.du <= $3 AND (e.au IS NULL OR e.au >= $3))",
                )
                .bind(etab)
                .bind(agent_id)
                .bind(du)
                .fetch_optional(&mut *tx)
                .await?;
                if actif.is_none() {
                    return Err(RefusDemande::Invalide("Cet agent n'est pas en poste à cette date."));
                }
                agent = Some(*agent_id);
                "agent"
            }
            SourceMembre::Personne { personne_id } => {
                if type_file != "repli" {
                    return Err(RefusDemande::Invalide("Une personne n'est désignée directement que dans une file de repli."));
                }
                let existe: Option<(Uuid,)> =
                    sqlx::query_as("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND id = $2 AND fusionnee_dans IS NULL")
                        .bind(etab)
                        .bind(personne_id)
                        .fetch_optional(&mut *tx)
                        .await?;
                if existe.is_none() {
                    return Err(RefusDemande::Invalide("La personne désignée est introuvable."));
                }
                personne = Some(*personne_id);
                "personne"
            }
        };
        // Avertissement : une personne désignée sans engagement actif est extérieure à la vie interne.
        let mut avertissements = Vec::new();
        if let Some(p) = personne {
            if personne_exterieure(&mut tx, etab, p, du).await? {
                avertissements.push(Avertissement::PersonneExterieure { personne_id: p });
            }
        }
        if !avertissements.is_empty() && !confirme_exterieur {
            return Ok(ResultatRegle { applique: false, regle_id: None, avertissements });
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_dem_file_membre_regle (id, etablissement_id, file_id, source, mission_type_code, fonction_code,
                    agent_id, personne_id, site_id, responsable, rang, du, au, exterieur_confirme_le, cree_par_user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                     CASE WHEN $14 THEN now() END, $15)",
        )
        .bind(id)
        .bind(etab)
        .bind(file_id)
        .bind(source)
        .bind(mission)
        .bind(fonction)
        .bind(agent)
        .bind(personne)
        .bind(site)
        .bind(saisie.responsable)
        .bind(saisie.rang)
        .bind(du)
        .bind(saisie.au)
        .bind(!avertissements.is_empty())
        .bind(user)
        .execute(&mut *tx)
        .await?;
        journaliser(&mut tx, etab, "membres", &code_file, None, Some(source), user).await?;
        tx.commit().await?;
        Ok(ResultatRegle { applique: true, regle_id: Some(id), avertissements })
    }

    /// Retire une règle de membres (annulation datée, jamais une suppression). Un repli ne reste jamais sans
    /// destinataire : le retrait qui viderait le repli RH est refusé.
    pub async fn retirer_regle_membre_for_actor(&self, etab: Uuid, user: Uuid, role: &str, regle_id: Uuid) -> Result<(), RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT r.file_id, f.code FROM ent_dem_file_membre_regle r
               JOIN ent_dem_file f ON f.etablissement_id = r.etablissement_id AND f.id = r.file_id
              WHERE r.etablissement_id = $1 AND r.id = $2 AND r.annule_le IS NULL",
        )
        .bind(etab)
        .bind(regle_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((file_id, code)) = r else { return Err(RefusDemande::Introuvable) };
        sqlx::query("UPDATE ent_dem_file_membre_regle SET annule_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(regle_id)
            .execute(&mut *tx)
            .await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let est_repli_rh: Option<(Uuid,)> =
            sqlx::query_as("SELECT file_id FROM ent_dem_repli_rh WHERE etablissement_id = $1 AND file_id = $2").bind(etab).bind(file_id).fetch_optional(&mut *tx).await?;
        if est_repli_rh.is_some() {
            let g = charger_graphe(&mut tx, etab, jour).await?;
            if g.membres_hors(file_id, &[]).is_empty() {
                // La transaction n'est pas validée : l'annulation est défaite.
                return Err(RefusDemande::Conflit("Le repli RH doit garder au moins un destinataire."));
            }
        }
        journaliser(&mut tx, etab, "membres", &code, Some("regle"), None, user).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Aperçu des membres d'une file **à une date** (défaut : aujourd'hui), pour qui paramètre le guichet ou
    /// supervise les files (jamais le contenu des demandes).
    pub async fn membres_de_file_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        file_id: Uuid,
        date: Option<NaiveDate>,
    ) -> Result<Vec<Membre>, RefusDemande> {
        let r = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        let autorise = r.is_staff()
            && (self.a_action_etablissement(etab, user, r.as_str(), Action::DemandesParametrer).await?
                || self.a_action_etablissement(etab, user, r.as_str(), Action::DemandesFileSuperviser).await?);
        if !autorise {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = match date {
            Some(d) => d,
            None => jour_metier(&mut tx, etab).await?,
        };
        let g = charger_graphe(&mut tx, etab, jour).await?;
        tx.commit().await?;
        if !g.files.contains_key(&file_id) {
            return Err(RefusDemande::Introuvable);
        }
        Ok(g.membres_hors(file_id, &[]).into_iter().cloned().collect())
    }

    // ───────────── File de repli RH (Q-DM14) ─────────────

    /// Le réglage courant, avec les destinataires d'aujourd'hui et l'explication en clair.
    pub async fn repli_rh_for_actor(&self, etab: Uuid, user: Uuid, role: &str) -> Result<Option<RepliRhVue>, RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let g = charger_graphe(&mut tx, etab, jour).await?;
        let r: Option<(Uuid,)> = sqlx::query_as("SELECT file_id FROM ent_dem_repli_rh WHERE etablissement_id = $1").bind(etab).fetch_optional(&mut *tx).await?;
        tx.commit().await?;
        Ok(r.and_then(|(fid,)| {
            g.files.get(&fid).map(|f| RepliRhVue {
                file_id: fid,
                code: f.code.clone(),
                libelle: f.libelle.clone(),
                membres: g.membres_hors(fid, &[]).into_iter().cloned().collect(),
                explication: EXPLICATION_REPLI_RH,
            })
        }))
    }

    /// Choisit la file de repli RH. Non réglable : la personne visée n'est jamais destinataire (le routage
    /// l'exclut), le repli a **au moins un destinataire** aujourd'hui, et le changement est journalisé.
    pub async fn definir_repli_rh_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        file_id: Uuid,
        confirme_exterieurs: bool,
    ) -> Result<ResultatRepli, RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let g = charger_graphe(&mut tx, etab, jour).await?;
        let Some(f) = g.files.get(&file_id) else { return Err(RefusDemande::Introuvable) };
        if f.type_ != "repli" {
            return Err(RefusDemande::Invalide("Le repli RH est une file de repli, hors du service concerné."));
        }
        let membres = g.membres_hors(file_id, &[]);
        if membres.is_empty() {
            return Err(RefusDemande::Invalide("Le repli doit avoir au moins un destinataire."));
        }
        let mut avertissements = Vec::new();
        for m in &membres {
            if m.agent_id.is_none() && personne_exterieure(&mut tx, etab, m.personne_id, jour).await? {
                avertissements.push(Avertissement::PersonneExterieure { personne_id: m.personne_id });
            }
        }
        if !avertissements.is_empty() && !confirme_exterieurs {
            return Ok(ResultatRepli { applique: false, avertissements });
        }
        let avant: Option<(String,)> = sqlx::query_as(
            "SELECT f.code FROM ent_dem_repli_rh r JOIN ent_dem_file f ON f.etablissement_id = r.etablissement_id AND f.id = r.file_id
              WHERE r.etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO ent_dem_repli_rh (etablissement_id, file_id, maj_par_user_id) VALUES ($1, $2, $3)
             ON CONFLICT (etablissement_id) DO UPDATE SET file_id = EXCLUDED.file_id, maj_le = now(), maj_par_user_id = EXCLUDED.maj_par_user_id",
        )
        .bind(etab)
        .bind(file_id)
        .bind(user)
        .execute(&mut *tx)
        .await?;
        journaliser(&mut tx, etab, "repli_rh", "repli_rh", avant.as_ref().map(|(c,)| c.as_str()), Some(&f.code), user).await?;
        tx.commit().await?;
        Ok(ResultatRepli { applique: true, avertissements })
    }

    // ───────────── Paliers d'escalade ─────────────

    /// Définit (ou remplace) un palier d'escalade d'une file (§13.3).
    #[allow(clippy::too_many_arguments)]
    pub async fn definir_palier_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        file_id: Uuid,
        rang: i16,
        apres_jours: i16,
        vers: &str,
        vers_file_id: Option<Uuid>,
    ) -> Result<(), RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        if !(1..=5).contains(&rang) || !(0..=60).contains(&apres_jours) || !matches!(vers, "responsable_file" | "direction" | "file") || (vers == "file") != vers_file_id.is_some() {
            return Err(RefusDemande::Invalide("Palier d'escalade incohérent."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let f: Option<(String,)> = sqlx::query_as("SELECT code FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2 AND actif").bind(etab).bind(file_id).fetch_optional(&mut *tx).await?;
        let Some((code,)) = f else { return Err(RefusDemande::Introuvable) };
        sqlx::query(
            "INSERT INTO ent_dem_file_palier (id, etablissement_id, file_id, rang, apres_jours, vers, vers_file_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (etablissement_id, file_id, rang)
             DO UPDATE SET apres_jours = EXCLUDED.apres_jours, vers = EXCLUDED.vers, vers_file_id = EXCLUDED.vers_file_id",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(file_id)
        .bind(rang)
        .bind(apres_jours)
        .bind(vers)
        .bind(vers_file_id)
        .execute(&mut *tx)
        .await?;
        journaliser(&mut tx, etab, "palier", &code, None, Some(vers), user).await?;
        tx.commit().await?;
        Ok(())
    }

    // ───────────── Réponses types ─────────────

    pub async fn creer_reponse_type_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        file_id: Uuid,
        libelle: &str,
        texte: &str,
    ) -> Result<Uuid, RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        if libelle.trim().is_empty() || libelle.chars().count() > 120 {
            return Err(RefusDemande::Invalide("Le libellé du modèle est obligatoire (120 caractères au plus)."));
        }
        valider_modele_reponse(texte).map_err(RefusDemande::Invalide)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let f: Option<(String,)> = sqlx::query_as("SELECT code FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2 AND actif").bind(etab).bind(file_id).fetch_optional(&mut *tx).await?;
        let Some((code,)) = f else { return Err(RefusDemande::Introuvable) };
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_dem_reponse_type (id, etablissement_id, file_id, libelle, texte) VALUES ($1, $2, $3, $4, $5)")
            .bind(id)
            .bind(etab)
            .bind(file_id)
            .bind(libelle.trim())
            .bind(texte.trim())
            .execute(&mut *tx)
            .await?;
        journaliser(&mut tx, etab, "reponse_type", &code, None, Some(libelle.trim()), user).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Les réponses types d'une file, pour ses **membres** (ou qui paramètre le guichet).
    pub async fn reponses_types_for_actor(&self, etab: Uuid, user: Uuid, role: &str, file_id: Uuid) -> Result<Vec<ReponseTypeLue>, RefusDemande> {
        let r = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !r.is_staff() {
            return Err(RefusDemande::Interdit);
        }
        let parametre = self.a_action_etablissement(etab, user, r.as_str(), Action::DemandesParametrer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if !parametre {
            let moi = personne_du_user(&mut tx, etab, user).await?;
            let g = { let j = jour_metier(&mut tx, etab).await?; charger_graphe(&mut tx, etab, j) }.await?;
            if !g.files_de(moi).contains(&file_id) {
                return Err(RefusDemande::Introuvable);
            }
        }
        let l: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, libelle, texte FROM ent_dem_reponse_type WHERE etablissement_id = $1 AND file_id = $2 AND actif ORDER BY libelle",
        )
        .bind(etab)
        .bind(file_id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(l.into_iter().map(|(id, libelle, texte)| ReponseTypeLue { id, libelle, texte }).collect())
    }

    // ───────────── Préréglages ─────────────

    /// Crée les files par défaut, leurs règles de membres (fonctions et missions du socle 6), leurs paliers, le
    /// repli RH et quelques réponses types. **Idempotent** : une file qui existe déjà n'est pas touchée.
    pub async fn initialiser_files_defaut_for_actor(&self, etab: Uuid, user: Uuid, role: &str) -> Result<RapportInit, RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let mut rapport = RapportInit::default();
        let mut ids: BTreeMap<&str, Uuid> = BTreeMap::new();
        for d in cat::FILES_DEFAUT {
            let existante: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM ent_dem_file WHERE etablissement_id = $1 AND code = $2").bind(etab).bind(d.code).fetch_optional(&mut *tx).await?;
            if let Some((id,)) = existante {
                ids.insert(d.code, id);
                continue;
            }
            let id = Uuid::new_v4();
            let repli = d.repli.and_then(|r| ids.get(r).copied());
            sqlx::query(
                "INSERT INTO ent_dem_file (id, etablissement_id, code, libelle, role_traitant, type, service_calendrier,
                        message_accueil, file_repli_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(id)
            .bind(etab)
            .bind(d.code)
            .bind(d.libelle)
            .bind(d.role_traitant)
            .bind(d.type_)
            .bind(d.service_calendrier)
            .bind(d.message_accueil)
            .bind(repli)
            .execute(&mut *tx)
            .await?;
            rapport.files_creees += 1;
            for m in d.membres {
                let (source, mission, fonction) = match m.source {
                    SourceDefaut::Fonction(c) => ("fonction", None, Some(c)),
                    SourceDefaut::Mission(c) => ("mission", Some(c), None),
                };
                sqlx::query(
                    "INSERT INTO ent_dem_file_membre_regle (id, etablissement_id, file_id, source, mission_type_code, fonction_code,
                            responsable, rang, du, cree_par_user_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(id)
                .bind(source)
                .bind(mission)
                .bind(fonction)
                .bind(m.responsable)
                .bind(m.rang)
                .bind(jour)
                .bind(user)
                .execute(&mut *tx)
                .await?;
                rapport.regles_creees += 1;
            }
            // Escalade par défaut (§13.3) : responsable à l'échéance, direction à +5 jours ouvrés.
            if matches!(d.type_, "service" | "tri" | "confidentielle") {
                for (rang, jours, vers) in [(1_i16, 0_i16, "responsable_file"), (2, 5, "direction")] {
                    sqlx::query(
                        "INSERT INTO ent_dem_file_palier (id, etablissement_id, file_id, rang, apres_jours, vers) VALUES ($1, $2, $3, $4, $5, $6)",
                    )
                    .bind(Uuid::new_v4())
                    .bind(etab)
                    .bind(id)
                    .bind(rang)
                    .bind(jours)
                    .bind(vers)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            journaliser(&mut tx, etab, "file", d.code, None, Some(d.type_), user).await?;
            ids.insert(d.code, id);
        }
        for (file, libelle, texte) in cat::REPONSES_TYPES_DEFAUT {
            let Some(fid) = ids.get(file) else { continue };
            let existe: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_dem_reponse_type WHERE etablissement_id = $1 AND file_id = $2 AND libelle = $3",
            )
            .bind(etab)
            .bind(fid)
            .bind(libelle)
            .fetch_optional(&mut *tx)
            .await?;
            if existe.is_none() {
                sqlx::query("INSERT INTO ent_dem_reponse_type (id, etablissement_id, file_id, libelle, texte) VALUES ($1, $2, $3, $4, $5)")
                    .bind(Uuid::new_v4())
                    .bind(etab)
                    .bind(fid)
                    .bind(libelle)
                    .bind(texte)
                    .execute(&mut *tx)
                    .await?;
                rapport.reponses_creees += 1;
            }
        }
        if let Some(repli) = ids.get("repli_rh") {
            let r = sqlx::query(
                "INSERT INTO ent_dem_repli_rh (etablissement_id, file_id, maj_par_user_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            )
            .bind(etab)
            .bind(repli)
            .bind(user)
            .execute(&mut *tx)
            .await?;
            if r.rows_affected() > 0 {
                rapport.repli_rh_pose = true;
                journaliser(&mut tx, etab, "repli_rh", "repli_rh", None, Some("repli_rh"), user).await?;
            }
        }
        tx.commit().await?;
        Ok(rapport)
    }
}

/// La personne n'a **aucun engagement actif** à la date : elle est extérieure à la vie interne.
pub(crate) async fn personne_exterieure(conn: &mut PgConnection, etab: Uuid, personne: Uuid, jour: NaiveDate) -> Result<bool, RefusDemande> {
    let (interne,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_agent a
                          JOIN ent_agent_engagement e ON e.etablissement_id = a.etablissement_id AND e.agent_id = a.id
                         WHERE a.etablissement_id = $1 AND a.personne_id = $2 AND a.fusionne_dans_agent_id IS NULL
                           AND e.annule_le IS NULL AND e.du <= $3 AND (e.au IS NULL OR e.au >= $3))",
    )
    .bind(etab)
    .bind(personne)
    .bind(jour)
    .fetch_one(conn)
    .await?;
    Ok(!interne)
}
