//! Socle N1, lot N1-3 : **reprographie**, nature `reprographie.tirage` (§18 de `docs/socles/N1_DEMANDES.md`,
//! conforme à la conception du socle 44 §15 : NRM-01 à NRM-10, DRM-01 à DRM-13).
//!
//! | Défaut | Garde-fou |
//! |---|---|
//! | NJ-77 : la file de photocopies de **tous les sites** est lisible | la file d'impression d'un réceptionnaire ne montre que les sites qui lui sont **désignés** (table `ent_dem_repro_receptionnaire`, jamais un rôle entier) : `t_nj77_file_par_site` |
//! | NJ-78 : le PDF d'un **autre** peut être rattaché | un fichier ne se rattache que s'il a été déposé **par le déposant** (port du socle 7) : `t_nj78_document_du_deposant` |
//! | NJ-92 : « prête » par un lien public valable 30 jours | « prête » est un **geste authentifié** (session + droit) : aucune colonne de jeton, aucune route sans session : `t_nj92_pret_par_geste_authentifie` |
//!
//! Circuit : **noir et blanc sous le seuil du site** → file d'impression d'emblée ; **couleur** ou **au-delà du
//! seuil** → validation par la direction (une seule décision, DRM-07, R-DM40 : jamais par soi-même). Les
//! avertissements (délai trop court, plafonds CFC) ne bloquent jamais. Les PDF ne sont **jamais** joints à un
//! courriel (DRM-12) et sont effacés 30 jours après le retrait (60 jours après « prête » sans retrait).
//!
//! Le jour métier est celui de l'établissement, jamais la date du serveur.

use crate::demandes::{
    ajouter_fait, charger_demande, echeance_ouvree, personne_du_user, publier_enveloppe,
    Conservation, ContexteVolet, DepotDemande, EnveloppeNeutre, EtatDemande, FaitNouveau,
    LigneDemande, QuotasDemandes, RefusDemande, SourcesDemandes, Valeur, ValeurSaisie, Volet,
};
use crate::demandes_echanges::PortDocuments;
use crate::demandes_files::jour_metier;
use crate::demandes_hse::{cellule_csv, texte_court};
use crate::service::PgEnt;
use crate::tenant_tx::TenantTx;
use chrono::{Datelike, Months, NaiveDate, NaiveTime};
use ent_auth::{Action, Role};
use ent_events::types;
use serde::Serialize;
use sqlx::PgConnection;
use uuid::Uuid;

pub const NATURE_REPRO: &str = "reprographie.tirage";

/// Seuil par défaut de copies en noir et blanc accepté d'emblée (SN1-11).
pub const SEUIL_DEFAUT: i32 = 500;
/// Délai réaliste par défaut de la reprographie, en jours ouvrés (SN1-12).
pub const DELAI_DEFAUT_JOURS: i16 = 1;

/// Phrases fixes envoyées au demandeur quand le réceptionnaire signale un problème (DEM-301) : jamais de texte libre
/// du réceptionnaire dans un message automatique.
pub const PROBLEMES: &[(&str, &str)] = &[
    (
        "pdf_illisible",
        "Le fichier à imprimer est illisible : merci d'en déposer un nouveau.",
    ),
    (
        "quantite_douteuse",
        "La quantité demandée semble inhabituelle : merci de la confirmer.",
    ),
    (
        "format_impossible",
        "Ce format ou cette finition n'est pas possible sur ce site : merci de préciser.",
    ),
];

// ───────────────────────── Saisies ─────────────────────────

/// Une cible : une classe, un groupe ou une demande libre ; exemplaires et lieu de retrait **par cible** (DEM-368).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CibleRepro {
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub exemplaires: i32,
    pub retrait_lieu_id: Option<Uuid>,
}

/// Un extrait d'œuvre protégée (plafonds CFC : 10 % d'un livre, 30 % d'une revue).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OeuvreSaisie {
    pub genre: String,
    pub titre: String,
    pub auteur: Option<String>,
    pub editeur: Option<String>,
    pub pages_copiees: i32,
    pub pages_oeuvre: i32,
}

/// Le volet écrit avec la demande.
#[derive(Debug, Clone)]
pub struct VoletRepro {
    site_id: Uuid,
    couleur: bool,
    recto_verso: bool,
    agrafage: bool,
    format: String,
    exemplaires_total: i32,
    pages_total: i32,
    pour_le: NaiveDate,
    pour_heure: Option<NaiveTime>,
    retrait_lieu_id: Option<Uuid>,
    documents: Vec<(Uuid, i32)>,
    cibles: Vec<CibleRepro>,
    oeuvres: Vec<OeuvreSaisie>,
}

/// Dépôt d'une demande de copies. Aucun champ d'identité : le demandeur est la personne de la session (ou, pour un
/// déposant habilité, l'enseignant désigné, DRM-08).
#[derive(Debug, Clone)]
pub struct DepotRepro {
    pub site_id: Uuid,
    pub objet: String,
    pub motif: Option<String>,
    /// 1 à 5 fichiers déposés **par le déposant** au socle 7.
    pub documents: Vec<Uuid>,
    pub format: String,
    pub couleur: bool,
    pub recto_verso: bool,
    pub agrafage: bool,
    pub cibles: Vec<CibleRepro>,
    /// Utilisé seulement quand aucune cible n'est donnée.
    pub exemplaires: Option<i32>,
    pub pour_le: NaiveDate,
    pub pour_heure: Option<NaiveTime>,
    pub retrait_lieu_id: Option<Uuid>,
    pub oeuvres: Vec<OeuvreSaisie>,
    pub pour_le_compte_de: Option<Uuid>,
}

impl DepotRepro {
    /// La demande générique **sans volet**, pour calculer le routage (`sources_demandes_for_actor`) avant le dépôt.
    /// Ne sert jamais à déposer : une nature à volet se dépose par [`PgEnt::deposer_repro_for_actor`].
    pub fn pour_routage(&self) -> DepotDemande {
        let exemplaires: i64 = if self.cibles.is_empty() {
            i64::from(self.exemplaires.unwrap_or(1))
        } else {
            self.cibles.iter().map(|c| i64::from(c.exemplaires)).sum()
        };
        DepotDemande {
            nature_code: NATURE_REPRO.into(),
            objet: self.objet.clone(),
            texte: None,
            valeurs: vec![
                ValeurSaisie {
                    champ: "format".into(),
                    rang: 0,
                    valeur: Valeur::Code(self.format.clone()),
                },
                ValeurSaisie {
                    champ: "couleur".into(),
                    rang: 0,
                    valeur: Valeur::Bool(self.couleur),
                },
                ValeurSaisie {
                    champ: "exemplaires".into(),
                    rang: 0,
                    valeur: Valeur::Entier(exemplaires),
                },
                ValeurSaisie {
                    champ: "pour_le".into(),
                    rang: 0,
                    valeur: Valeur::Date(self.pour_le),
                },
            ],
            urgence: None,
            eleve_id: None,
            pour_le_compte_de: self.pour_le_compte_de,
            canal: None,
            volet: None,
        }
    }
}

/// Une demande prête à être redéposée (DEM-366) : **jamais les fichiers** d'une autre demande.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModeleRepro {
    pub site_id: Uuid,
    pub objet: Option<String>,
    pub format: String,
    pub couleur: bool,
    pub recto_verso: bool,
    pub agrafage: bool,
    pub exemplaires_total: i32,
    pub retrait_lieu_id: Option<Uuid>,
}

// ───────────────────────── Sorties ─────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResultatRepro {
    pub etat: EtatDemande,
    /// L'état de la demande de copies : `a_valider` ou `a_imprimer`.
    pub etat_repro: String,
    pub validation_requise: bool,
    pub avertissement_delai: bool,
    pub avertissement_cfc: bool,
    pub copies_total: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct LigneImpression {
    pub demande_id: Uuid,
    pub numero: String,
    pub site_id: Uuid,
    pub etat: String,
    pub format: String,
    pub couleur: bool,
    pub recto_verso: bool,
    pub agrafage: bool,
    pub pages_total: i32,
    pub exemplaires_total: i32,
    pub copies_total: i32,
    pub pour_le: NaiveDate,
    pub pour_heure: Option<NaiveTime>,
    pub avertissement_delai: bool,
    pub nb_documents: i64,
    pub demandeur_nom: Option<String>,
    pub retrait_lieu: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DetailRepro {
    pub demande_id: Uuid,
    pub numero: String,
    pub statut: String,
    pub etat: String,
    pub site_id: Uuid,
    pub couleur: bool,
    pub recto_verso: bool,
    pub agrafage: bool,
    pub format: String,
    pub pages_total: i32,
    pub exemplaires_total: i32,
    pub copies_total: i32,
    pub pour_le: NaiveDate,
    pub validation_requise: bool,
    pub avertissement_delai: bool,
    pub avertissement_cfc: bool,
    pub retrait_lieu: Option<String>,
    pub prete_le: Option<chrono::DateTime<chrono::Utc>>,
    /// « Prête, pas encore vue » : vrai tant que le demandeur n'a pas ouvert sa demande prête.
    pub prete_non_vue: bool,
    pub documents: Vec<Uuid>,
    pub cibles: Vec<CibleRepro>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LigneCompteur {
    pub site_id: Uuid,
    pub demandeur_nom: String,
    pub couleur: bool,
    pub demandes: i64,
    pub copies: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RapportRepro {
    pub traitees: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReglageSiteRepro {
    pub site_id: Uuid,
    pub seuil_copies: i32,
    pub delai_jours: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EffectifCible {
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub effectif: i64,
}

// ───────────────────────── Aides ─────────────────────────

/// Plafond CFC dépassé : plus de 10 % d'un livre ou de 30 % d'une revue (calcul entier).
pub fn cfc_depasse(o: &OeuvreSaisie) -> bool {
    let pct: i64 = if o.genre == "revue" { 30 } else { 10 };
    i64::from(o.pages_copiees) * 100 > i64::from(o.pages_oeuvre) * pct
}

fn valider_oeuvre(o: &OeuvreSaisie) -> Result<(), RefusDemande> {
    if !matches!(o.genre.as_str(), "livre" | "revue") {
        return Err(RefusDemande::Invalide(
            "Le genre de l'œuvre est « livre » ou « revue ».",
        ));
    }
    let titre = o.titre.trim();
    if titre.is_empty() || titre.chars().count() > 200 || titre.contains('\0') {
        return Err(RefusDemande::Invalide(
            "Le titre de l'œuvre est obligatoire.",
        ));
    }
    texte_court(&o.auteur, 120)?;
    texte_court(&o.editeur, 120)?;
    if !(1..=5000).contains(&o.pages_copiees)
        || !(1..=20000).contains(&o.pages_oeuvre)
        || o.pages_copiees > o.pages_oeuvre
    {
        return Err(RefusDemande::Invalide(
            "Les pages copiées ne peuvent pas dépasser celles de l'œuvre.",
        ));
    }
    Ok(())
}

/// Le lien d'un acteur avec une demande de copies. **Un seul chemin** les calcule : [`acces_repro`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfilRepro {
    Demandeur,
    Deposant,
    /// Réceptionnaire désigné pour le site de la demande.
    Receptionnaire,
    /// Titulaire de `demandes.repro.valider`.
    Validateur,
}

struct AccesRepro {
    ligne: LigneDemande,
    moi: Uuid,
    agent: Option<Uuid>,
    profils: Vec<ProfilRepro>,
    site_id: Uuid,
    etat: String,
}

impl AccesRepro {
    fn est(&self, p: ProfilRepro) -> bool {
        self.profils.contains(&p)
    }
    fn demandeur_ou_deposant(&self) -> bool {
        self.est(ProfilRepro::Demandeur) || self.est(ProfilRepro::Deposant)
    }
}

/// **La garde d'accès** de la reprographie (NJ-77, NJ-80) : sans lien — ni demandeur, ni déposant, ni réceptionnaire
/// **du site de la demande**, ni titulaire de la validation — la demande **n'existe pas** (404 neutre).
#[allow(clippy::too_many_arguments)]
async fn acces_repro(
    ent: &PgEnt,
    conn: &mut PgConnection,
    etab: Uuid,
    user_id: Uuid,
    role: Role,
    id: Uuid,
    verrou: bool,
) -> Result<AccesRepro, RefusDemande> {
    let moi = personne_du_user(conn, etab, user_id)
        .await
        .map_err(|_| RefusDemande::Introuvable)?;
    let ligne = charger_demande(conn, etab, id, verrou).await?;
    if ligne.nature_code != NATURE_REPRO {
        return Err(RefusDemande::Introuvable);
    }
    let (site_id, etat): (Uuid, String) = sqlx::query_as(
        "SELECT site_id, etat FROM ent_dem_repro WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(RefusDemande::Introuvable)?;
    let agent: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_agent WHERE etablissement_id = $1 AND personne_id = $2 ORDER BY cree_le LIMIT 1")
        .bind(etab)
        .bind(moi)
        .fetch_optional(&mut *conn)
        .await?;
    let agent = agent.map(|(a,)| a);
    let mut profils = Vec::new();
    if ligne.demandeur_personne_id == Some(moi) {
        profils.push(ProfilRepro::Demandeur);
    }
    if ligne.deposant_personne_id == Some(moi) {
        profils.push(ProfilRepro::Deposant);
    }
    if role.is_staff() {
        if let Some(a) = agent {
            if est_receptionnaire(conn, etab, a, site_id).await? {
                profils.push(ProfilRepro::Receptionnaire);
            }
        }
        if ent
            .a_action_etablissement(etab, user_id, role.as_str(), Action::DemandesReproValider)
            .await?
        {
            profils.push(ProfilRepro::Validateur);
        }
    }
    if profils.is_empty() {
        return Err(RefusDemande::Introuvable);
    }
    Ok(AccesRepro {
        ligne,
        moi,
        agent,
        profils,
        site_id,
        etat,
    })
}

/// L'agent est désigné pour ce site **aujourd'hui** (jour de l'établissement).
async fn est_receptionnaire(
    conn: &mut PgConnection,
    etab: Uuid,
    agent: Uuid,
    site: Uuid,
) -> Result<bool, RefusDemande> {
    let (v,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_dem_repro_receptionnaire
                         WHERE etablissement_id = $1 AND agent_id = $2 AND site_id = $3
                           AND du <= ent_aujourdhui_etab($1, now()) AND (au IS NULL OR au > ent_aujourdhui_etab($1, now())))",
    )
    .bind(etab)
    .bind(agent)
    .bind(site)
    .fetch_one(conn)
    .await?;
    Ok(v)
}

async fn reglage_site(
    conn: &mut PgConnection,
    etab: Uuid,
    site: Uuid,
) -> Result<(i32, i16), RefusDemande> {
    let r: Option<(i32, i16)> = sqlx::query_as(
        "SELECT seuil_copies, delai_jours FROM ent_dem_repro_site_reglage WHERE etablissement_id = $1 AND site_id = $2",
    )
    .bind(etab)
    .bind(site)
    .fetch_optional(conn)
    .await?;
    Ok(r.unwrap_or((SEUIL_DEFAUT, DELAI_DEFAUT_JOURS)))
}

async fn lieu_du_site(
    conn: &mut PgConnection,
    etab: Uuid,
    lieu: Uuid,
    site: Uuid,
) -> Result<(), RefusDemande> {
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_dem_repro_lieu_retrait WHERE etablissement_id = $1 AND id = $2 AND site_id = $3 AND actif)",
    )
    .bind(etab)
    .bind(lieu)
    .bind(site)
    .fetch_one(conn)
    .await?;
    if ok {
        Ok(())
    } else {
        Err(RefusDemande::Invalide(
            "Ce lieu de retrait n'existe pas sur ce site.",
        ))
    }
}

// ───────────────────────── Dépôt (volet écrit avec la demande) ─────────────────────────

/// Écrit le volet dans la transaction du dépôt.
pub(crate) async fn ecrire_volet(
    conn: &mut PgConnection,
    ctx: &ContexteVolet,
    v: &VoletRepro,
) -> Result<(), RefusDemande> {
    let (existe,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_site WHERE etablissement_id = $1 AND id = $2)",
    )
    .bind(ctx.etab)
    .bind(v.site_id)
    .fetch_one(&mut *conn)
    .await?;
    if !existe {
        return Err(RefusDemande::Invalide("Ce site n'existe pas."));
    }
    if let Some(l) = v.retrait_lieu_id {
        lieu_du_site(&mut *conn, ctx.etab, l, v.site_id).await?;
    }
    for c in &v.cibles {
        if let Some(l) = c.retrait_lieu_id {
            lieu_du_site(&mut *conn, ctx.etab, l, v.site_id).await?;
        }
    }
    let classes: Vec<Uuid> = v.cibles.iter().filter_map(|c| c.classe_id).collect();
    let groupes: Vec<Uuid> = v.cibles.iter().filter_map(|c| c.groupe_id).collect();
    if !classes.is_empty() {
        let (n,): (i64,) = sqlx::query_as("SELECT count(DISTINCT id) FROM ent_classe WHERE etablissement_id = $1 AND id = ANY($2)")
            .bind(ctx.etab)
            .bind(&classes)
            .fetch_one(&mut *conn)
            .await?;
        let distinctes: std::collections::BTreeSet<_> = classes.iter().collect();
        if usize::try_from(n).ok() != Some(distinctes.len()) {
            return Err(RefusDemande::Invalide(
                "Une des classes choisies est introuvable.",
            ));
        }
    }
    if !groupes.is_empty() {
        let (n,): (i64,) = sqlx::query_as("SELECT count(DISTINCT id) FROM ent_groupe WHERE etablissement_id = $1 AND id = ANY($2)")
            .bind(ctx.etab)
            .bind(&groupes)
            .fetch_one(&mut *conn)
            .await?;
        let distinctes: std::collections::BTreeSet<_> = groupes.iter().collect();
        if usize::try_from(n).ok() != Some(distinctes.len()) {
            return Err(RefusDemande::Invalide(
                "Un des groupes choisis est introuvable.",
            ));
        }
    }
    let copies = i64::from(v.pages_total) * i64::from(v.exemplaires_total);
    if copies > 1_000_000 {
        return Err(RefusDemande::Invalide(
            "Le volume de copies est trop élevé pour une seule demande.",
        ));
    }
    // DRM-07 : couleur, ou noir et blanc au-delà du seuil du site → validation de la direction.
    let (seuil, delai) = reglage_site(&mut *conn, ctx.etab, v.site_id).await?;
    let validation = v.couleur || copies > i64::from(seuil);
    // Avertissements, jamais un refus (DEM-293, DEM-294) : délai plus court que celui du site ; plafonds CFC.
    let au_plus_tot = echeance_ouvree(
        &mut *conn,
        ctx.etab,
        ctx.jour,
        u16::try_from(delai).unwrap_or(1),
        "secretariat",
    )
    .await;
    let avert_delai = v.pour_le < au_plus_tot;
    let avert_cfc = v.oeuvres.iter().any(cfc_depasse);
    sqlx::query(
        "INSERT INTO ent_dem_repro (etablissement_id, demande_id, site_id, couleur, recto_verso, agrafage, format, exemplaires_total,
                pages_total, copies_total, pour_le, pour_heure, retrait_lieu_id, oeuvres_protegees, avertissement_delai,
                avertissement_cfc, etat, validation_requise)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
    )
    .bind(ctx.etab)
    .bind(ctx.demande_id)
    .bind(v.site_id)
    .bind(v.couleur)
    .bind(v.recto_verso)
    .bind(v.agrafage)
    .bind(&v.format)
    .bind(v.exemplaires_total)
    .bind(v.pages_total)
    .bind(i32::try_from(copies).unwrap_or(i32::MAX))
    .bind(v.pour_le)
    .bind(v.pour_heure)
    .bind(v.retrait_lieu_id)
    .bind(!v.oeuvres.is_empty())
    .bind(avert_delai)
    .bind(avert_cfc)
    .bind(if validation { "a_valider" } else { "a_imprimer" })
    .bind(validation)
    .execute(&mut *conn)
    .await?;
    // NJ-78 : les fichiers sont ceux du DÉPOSANT (contrôlé par le port avant la transaction) ; la base garde l'auteur.
    for (doc, pages) in &v.documents {
        sqlx::query(
            "INSERT INTO ent_dem_repro_document (id, etablissement_id, demande_id, document_id, pages, depose_par_personne_id)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(ctx.etab)
        .bind(ctx.demande_id)
        .bind(doc)
        .bind(pages)
        .bind(ctx.deposant)
        .execute(&mut *conn)
        .await?;
    }
    for c in &v.cibles {
        sqlx::query(
            "INSERT INTO ent_dem_repro_cible (id, etablissement_id, demande_id, classe_id, groupe_id, exemplaires, retrait_lieu_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(ctx.etab)
        .bind(ctx.demande_id)
        .bind(c.classe_id)
        .bind(c.groupe_id)
        .bind(c.exemplaires)
        .bind(c.retrait_lieu_id)
        .execute(&mut *conn)
        .await?;
    }
    for o in &v.oeuvres {
        sqlx::query(
            "INSERT INTO ent_dem_repro_oeuvre (id, etablissement_id, demande_id, genre, titre, auteur, editeur, pages_copiees, pages_oeuvre)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(Uuid::new_v4())
        .bind(ctx.etab)
        .bind(ctx.demande_id)
        .bind(&o.genre)
        .bind(o.titre.trim())
        .bind(o.auteur.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .bind(o.editeur.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .bind(o.pages_copiees)
        .bind(o.pages_oeuvre)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

impl PgEnt {
    /// Dépose une demande de copies. Les fichiers doivent avoir été déposés **par le déposant** (NJ-78) ; le nombre de
    /// pages est lu par le socle 7 ; la demande et son volet s'écrivent dans **une seule transaction**.
    pub async fn deposer_repro_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        docs: &dyn PortDocuments,
        d: DepotRepro,
    ) -> Result<ResultatRepro, RefusDemande> {
        if !matches!(d.format.as_str(), "a4" | "a3") {
            return Err(RefusDemande::Invalide("Le format est A4 ou A3."));
        }
        if d.documents.is_empty() || d.documents.len() > 5 {
            return Err(RefusDemande::Invalide("Joignez de 1 à 5 fichiers PDF."));
        }
        let distincts: std::collections::BTreeSet<_> = d.documents.iter().collect();
        if distincts.len() != d.documents.len() {
            return Err(RefusDemande::Invalide("Un fichier est joint deux fois."));
        }
        if d.cibles.len() > 20 || d.oeuvres.len() > 10 {
            return Err(RefusDemande::Invalide(
                "Trop de classes ou d'œuvres pour une seule demande.",
            ));
        }
        let exemplaires_total: i64 = if d.cibles.is_empty() {
            i64::from(d.exemplaires.unwrap_or(0))
        } else {
            for c in &d.cibles {
                if !(1..=5000).contains(&c.exemplaires)
                    || (c.classe_id.is_some() && c.groupe_id.is_some())
                {
                    return Err(RefusDemande::Invalide("Une des cibles est invalide."));
                }
            }
            d.cibles.iter().map(|c| i64::from(c.exemplaires)).sum()
        };
        if !(1..=5000).contains(&exemplaires_total) {
            return Err(RefusDemande::Invalide(
                "Le nombre d'exemplaires est de 1 à 5 000.",
            ));
        }
        for o in &d.oeuvres {
            valider_oeuvre(o)?;
        }
        let motif = texte_court(&d.motif, 1000)?;
        // NJ-78 : qui est le déposant ? La personne de la SESSION, jamais celle qu'on dépose « pour le compte ».
        let moi = {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            let p = personne_du_user(&mut tx, etab, user_id).await?;
            tx.commit().await?;
            p
        };
        let mut documents = Vec::new();
        for doc in &d.documents {
            if !docs.document_depose_par(etab, *doc, moi) {
                return Err(RefusDemande::Invalide(
                    "Un des fichiers n'est pas disponible : déposez-le d'abord vous-même.",
                ));
            }
            let pages = docs
                .pages_pdf(etab, *doc)
                .filter(|p| (1..=2000).contains(p))
                .ok_or(RefusDemande::Invalide(
                    "Le nombre de pages d'un des fichiers est illisible.",
                ))?;
            documents.push((*doc, pages));
        }
        let pages_total: i32 = documents.iter().map(|(_, p)| *p).sum();
        if pages_total > 5000 {
            return Err(RefusDemande::Invalide(
                "Trop de pages pour une seule demande.",
            ));
        }
        let exemplaires_total = i32::try_from(exemplaires_total).unwrap_or(5000);
        let mut valeurs = vec![
            ValeurSaisie {
                champ: "format".into(),
                rang: 0,
                valeur: Valeur::Code(d.format.clone()),
            },
            ValeurSaisie {
                champ: "couleur".into(),
                rang: 0,
                valeur: Valeur::Bool(d.couleur),
            },
            ValeurSaisie {
                champ: "recto_verso".into(),
                rang: 0,
                valeur: Valeur::Bool(d.recto_verso),
            },
            ValeurSaisie {
                champ: "agrafage".into(),
                rang: 0,
                valeur: Valeur::Bool(d.agrafage),
            },
            ValeurSaisie {
                champ: "exemplaires".into(),
                rang: 0,
                valeur: Valeur::Entier(i64::from(exemplaires_total)),
            },
            ValeurSaisie {
                champ: "pour_le".into(),
                rang: 0,
                valeur: Valeur::Date(d.pour_le),
            },
            ValeurSaisie {
                champ: "oeuvres_protegees".into(),
                rang: 0,
                valeur: Valeur::Bool(!d.oeuvres.is_empty()),
            },
        ];
        if let Some(h) = d.pour_heure {
            valeurs.push(ValeurSaisie {
                champ: "pour_heure".into(),
                rang: 0,
                valeur: Valeur::Heure(h),
            });
        }
        let volet = VoletRepro {
            site_id: d.site_id,
            couleur: d.couleur,
            recto_verso: d.recto_verso,
            agrafage: d.agrafage,
            format: d.format,
            exemplaires_total,
            pages_total,
            pour_le: d.pour_le,
            pour_heure: d.pour_heure,
            retrait_lieu_id: d.retrait_lieu_id,
            documents,
            cibles: d.cibles,
            oeuvres: d.oeuvres,
        };
        let depot = DepotDemande {
            nature_code: NATURE_REPRO.into(),
            objet: d.objet,
            texte: motif,
            valeurs,
            urgence: None,
            eleve_id: None,
            pour_le_compte_de: d.pour_le_compte_de,
            canal: None,
            volet: Some(Volet::Repro(volet)),
        };
        let etat = self
            .deposer_demande_for_actor(etab, user_id, role, sources, depot)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (etat_repro, validation_requise, avertissement_delai, avertissement_cfc, copies_total): (String, bool, bool, bool, i32) =
            sqlx::query_as(
                "SELECT etat, validation_requise, avertissement_delai, avertissement_cfc, copies_total
                   FROM ent_dem_repro WHERE etablissement_id = $1 AND demande_id = $2",
            )
            .bind(etab)
            .bind(etat.id)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(ResultatRepro {
            etat,
            etat_repro,
            validation_requise,
            avertissement_delai,
            avertissement_cfc,
            copies_total,
        })
    }

    /// Exemplaires proposés d'après l'effectif des classes ou groupes choisis (DEM-291). Des **nombres**, jamais de
    /// noms ; modifiables par le demandeur.
    pub async fn effectifs_cibles_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        classes: &[Uuid],
        groupes: &[Uuid],
    ) -> Result<Vec<EffectifCible>, RefusDemande> {
        let r = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !r.is_staff()
            || !self
                .a_action_etablissement(etab, user_id, role, Action::DemandesDeposer)
                .await?
        {
            return Err(RefusDemande::Interdit);
        }
        if classes.len() + groupes.len() > 40 {
            return Err(RefusDemande::Invalide("Trop de classes ou de groupes."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let mut sortie = Vec::new();
        for c in classes {
            let (n,): (i64,) = sqlx::query_as(
                "SELECT count(DISTINCT eleve_id) FROM ent_affectation
                  WHERE etablissement_id = $1 AND classe_id = $2 AND date_debut <= $3 AND (date_fin IS NULL OR date_fin >= $3)",
            )
            .bind(etab)
            .bind(c)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await?;
            sortie.push(EffectifCible {
                classe_id: Some(*c),
                groupe_id: None,
                effectif: n,
            });
        }
        for g in groupes {
            let (n,): (i64,) = sqlx::query_as(
                "SELECT count(DISTINCT eleve_id) FROM ent_affectation
                  WHERE etablissement_id = $1 AND groupe_id = $2 AND date_debut <= $3 AND (date_fin IS NULL OR date_fin >= $3)",
            )
            .bind(etab)
            .bind(g)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await?;
            sortie.push(EffectifCible {
                classe_id: None,
                groupe_id: Some(*g),
                effectif: n,
            });
        }
        tx.commit().await?;
        Ok(sortie)
    }
}

// ───────────────────────── Validation (DRM-07, R-DM40) ─────────────────────────

impl PgEnt {
    /// Valide ou refuse une demande de copies en couleur ou au-delà du seuil (DEM-295). **Une seule décision**, et
    /// jamais sur sa propre demande ni sur celle qu'on a déposée (R-DM40).
    pub async fn valider_repro_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        id: Uuid,
        accepter: bool,
        note: Option<&str>,
    ) -> Result<EtatDemande, RefusDemande> {
        self.limiter(
            "demandes.geste",
            user_id,
            QuotasDemandes::depuis_env().geste,
        )
        .await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let note = texte_court(&note.map(str::to_string), 300)?;
        if !accepter && note.is_none() {
            return Err(RefusDemande::Invalide("Un refus s'accompagne d'une note."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = acces_repro(self, &mut tx, etab, user_id, role, id, true).await?;
        if !a.est(ProfilRepro::Validateur) {
            return Err(RefusDemande::Interdit);
        }
        if a.demandeur_ou_deposant() {
            return Err(RefusDemande::Interdit);
        }
        if a.etat != "a_valider" {
            return Err(RefusDemande::Conflit(
                "Cette demande n'attend pas de validation.",
            ));
        }
        let n = sqlx::query(
            "UPDATE ent_dem_repro SET etat = $3, valide_par_personne_id = $4, valide_le = now(), validation_note = $5
              WHERE etablissement_id = $1 AND demande_id = $2 AND etat = 'a_valider'",
        )
        .bind(etab)
        .bind(id)
        .bind(if accepter { "a_imprimer" } else { "refusee" })
        .bind(a.moi)
        .bind(note.as_deref())
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit("Cette demande vient d'être décidée."));
        }
        let mut version = a.ligne.version;
        let mut statut = a.ligne.statut.clone();
        if !accepter {
            let jour = jour_metier(&mut tx, etab).await?;
            let conservation = Conservation::Reprographie
                .duree_mois()
                .and_then(|m| jour.checked_add_months(Months::new(m)));
            let n = sqlx::query(
                "UPDATE ent_dem_demande
                    SET statut = 'close', issue = 'refusee', decision = 'refusee', clos_le = now(), conservation_jusqu_au = $4,
                        attribuee_a_agent_id = NULL, attribuee_le = NULL, version = version + 1
                  WHERE etablissement_id = $1 AND id = $2 AND version = $3",
            )
            .bind(etab)
            .bind(id)
            .bind(a.ligne.version)
            .bind(conservation)
            .execute(&mut *tx)
            .await?
            .rows_affected();
            if n != 1 {
                return Err(RefusDemande::Conflit(
                    "La demande a changé entre-temps : rechargez-la.",
                ));
            }
            sqlx::query("UPDATE ent_dem_echeance SET traitee_le = now() WHERE etablissement_id = $1 AND demande_id = $2 AND traitee_le IS NULL")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            version += 1;
            statut = "close".into();
        }
        let mut f = FaitNouveau::nouveau(
            id,
            if accepter {
                "repro_validee"
            } else {
                "repro_refusee"
            },
        );
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some(&a.ligne.statut);
        f.statut_apres = Some(&statut);
        ajouter_fait(&mut tx, etab, f).await?;
        let env = EnveloppeNeutre::nouvelle(
            id,
            &a.ligne.numero,
            NATURE_REPRO,
            if accepter { "repro_validee" } else { "close" },
        );
        publier_enveloppe(
            &mut tx,
            etab,
            if accepter {
                types::DEMANDE_REPRO_VALIDEE
            } else {
                types::DEMANDE_CLOSE
            },
            &env,
            Some(user_id),
        )
        .await?;
        tx.commit().await?;
        Ok(EtatDemande {
            id,
            numero: a.ligne.numero,
            statut,
            version,
        })
    }

    /// Les demandes de copies à valider (DEM-295), pour le titulaire de `demandes.repro.valider` : jamais les siennes.
    pub async fn repro_a_valider_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<Vec<LigneImpression>, RefusDemande> {
        let r = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !r.is_staff()
            || !self
                .a_action_etablissement(etab, user_id, role, Action::DemandesReproValider)
                .await?
        {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await?;
        let l = lignes_impression(&mut tx, etab, &["a_valider"], None, Some(moi)).await?;
        tx.commit().await?;
        Ok(l)
    }
}

// ───────────────────────── File d'impression par site (NJ-77, R-DM46) ─────────────────────────

/// Lecture de la file : sites **désignés** (si `agent` donné) ou tous (validation), jamais les demandes de `exclure`.
async fn lignes_impression(
    conn: &mut PgConnection,
    etab: Uuid,
    etats: &[&str],
    agent: Option<Uuid>,
    exclure_personne: Option<Uuid>,
) -> Result<Vec<LigneImpression>, RefusDemande> {
    let etats: Vec<String> = etats.iter().map(|s| (*s).to_string()).collect();
    let l: Vec<LigneImpression> = sqlx::query_as(
        "SELECT d.id AS demande_id, d.numero, r.site_id, r.etat, r.format, r.couleur, r.recto_verso, r.agrafage, r.pages_total,
                r.exemplaires_total, r.copies_total, r.pour_le, r.pour_heure, r.avertissement_delai,
                (SELECT count(*) FROM ent_dem_repro_document x WHERE x.etablissement_id = r.etablissement_id
                    AND x.demande_id = r.demande_id AND x.efface_le IS NULL) AS nb_documents,
                NULLIF(btrim(COALESCE(p.prenom, '') || ' ' || p.nom), '') AS demandeur_nom,
                (SELECT lr.libelle FROM ent_dem_repro_lieu_retrait lr WHERE lr.etablissement_id = r.etablissement_id
                    AND lr.id = COALESCE(r.prete_retrait_lieu_id, r.retrait_lieu_id)) AS retrait_lieu
           FROM ent_dem_repro r
           JOIN ent_dem_demande d ON d.etablissement_id = r.etablissement_id AND d.id = r.demande_id
           LEFT JOIN ent_personne p ON p.etablissement_id = d.etablissement_id AND p.id = d.demandeur_personne_id
          WHERE r.etablissement_id = $1 AND r.etat = ANY($2) AND d.purgee_le IS NULL
            AND ($3::uuid IS NULL OR r.site_id IN (
                   SELECT x.site_id FROM ent_dem_repro_receptionnaire x
                    WHERE x.etablissement_id = $1 AND x.agent_id = $3
                      AND x.du <= ent_aujourdhui_etab($1, now()) AND (x.au IS NULL OR x.au > ent_aujourdhui_etab($1, now()))))
            AND ($4::uuid IS NULL OR (d.demandeur_personne_id IS DISTINCT FROM $4 AND d.deposant_personne_id IS DISTINCT FROM $4))
          ORDER BY r.pour_le, r.pour_heure NULLS LAST, d.depose_le, d.id
          LIMIT 300",
    )
    .bind(etab)
    .bind(&etats)
    .bind(agent)
    .bind(exclure_personne)
    .fetch_all(conn)
    .await?;
    Ok(l)
}

impl PgEnt {
    /// La file d'impression **des sites désignés au réceptionnaire** (DEM-296) : rien des autres sites (NJ-77). Sans
    /// désignation, la file est vide : jamais « tout un rôle » par défaut.
    pub async fn file_impression_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<Vec<LigneImpression>, RefusDemande> {
        let r = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !r.is_staff() {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let agent = crate::personnels::agent_du_user_tx(&mut tx, etab, user_id).await?;
        let l = match agent {
            Some(a) => {
                lignes_impression(
                    &mut tx,
                    etab,
                    &["a_imprimer", "en_impression", "prete"],
                    Some(a),
                    None,
                )
                .await?
            }
            None => Vec::new(),
        };
        tx.commit().await?;
        Ok(l)
    }

    /// « Mes copies » : mes demandes de copies (demandeur ou déposant), avec l'état courant.
    pub async fn mes_repro_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<LigneImpression>, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await?;
        let l: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT d.id FROM ent_dem_demande d JOIN ent_dem_repro r ON r.etablissement_id = d.etablissement_id AND r.demande_id = d.id
              WHERE d.etablissement_id = $1 AND (d.demandeur_personne_id = $2 OR d.deposant_personne_id = $2) AND d.purgee_le IS NULL
              ORDER BY d.cree_le DESC LIMIT 100",
        )
        .bind(etab)
        .bind(moi)
        .fetch_all(&mut *tx)
        .await?;
        let ids: Vec<Uuid> = l.into_iter().map(|(i,)| i).collect();
        let toutes = lignes_impression_ids(&mut tx, etab, &ids).await?;
        tx.commit().await?;
        Ok(toutes)
    }
}

async fn lignes_impression_ids(
    conn: &mut PgConnection,
    etab: Uuid,
    ids: &[Uuid],
) -> Result<Vec<LigneImpression>, RefusDemande> {
    let toutes = lignes_impression(
        conn,
        etab,
        &[
            "a_valider",
            "a_imprimer",
            "en_impression",
            "prete",
            "retiree",
            "refusee",
            "annulee",
        ],
        None,
        None,
    )
    .await?;
    let mut sortie: Vec<LigneImpression> = toutes
        .into_iter()
        .filter(|l| ids.contains(&l.demande_id))
        .collect();
    sortie.sort_by_key(|l| ids.iter().position(|i| *i == l.demande_id));
    Ok(sortie)
}

// ───────────────────────── Gestes du réceptionnaire et du demandeur ─────────────────────────

impl PgEnt {
    /// Ouvre un PDF à imprimer (DEM-297) : jeton **court**, émis par le socle 7 **après** le droit de N1, **journalisé**.
    pub async fn ouvrir_pdf_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        docs: &dyn PortDocuments,
        id: Uuid,
        document_id: Uuid,
    ) -> Result<String, RefusDemande> {
        self.limiter(
            "demandes.geste",
            user_id,
            QuotasDemandes::depuis_env().geste,
        )
        .await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = acces_repro(self, &mut tx, etab, user_id, role, id, false).await?;
        // Le fichier se lit par le réceptionnaire du site (une fois la demande acceptée) ou par son demandeur.
        let lisible = (a.est(ProfilRepro::Receptionnaire)
            && matches!(a.etat.as_str(), "a_imprimer" | "en_impression" | "prete"))
            || a.demandeur_ou_deposant();
        if !lisible {
            return Err(RefusDemande::Introuvable);
        }
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_dem_repro_document WHERE etablissement_id = $1 AND demande_id = $2
                              AND document_id = $3 AND efface_le IS NULL)",
        )
        .bind(etab)
        .bind(id)
        .bind(document_id)
        .fetch_one(&mut *tx)
        .await?;
        if !ok {
            return Err(RefusDemande::Introuvable);
        }
        let jeton = docs
            .jeton_lecture(etab, document_id, user_id)
            .ok_or(RefusDemande::Indisponible)?;
        let mut f = FaitNouveau::nouveau(id, "repro_pdf_ouvert");
        f.auteur_personne = Some(a.moi);
        f.ref_id = Some(document_id);
        ajouter_fait(&mut tx, etab, f).await?;
        tx.commit().await?;
        Ok(jeton)
    }

    /// Passe la demande « en impression » (DEM-298) : la demande est prise en charge, donc **plus annulable**.
    pub async fn mettre_en_impression_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<EtatDemande, RefusDemande> {
        self.limiter(
            "demandes.geste",
            user_id,
            QuotasDemandes::depuis_env().geste,
        )
        .await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = acces_repro(self, &mut tx, etab, user_id, role, id, true).await?;
        if !a.est(ProfilRepro::Receptionnaire) {
            return Err(RefusDemande::Interdit);
        }
        let agent = a.agent.ok_or(RefusDemande::Interdit)?;
        let n = sqlx::query(
            "UPDATE ent_dem_repro SET etat = 'en_impression', en_impression_le = now()
              WHERE etablissement_id = $1 AND demande_id = $2 AND etat = 'a_imprimer'",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit("Cette demande n'est pas à imprimer."));
        }
        let n = sqlx::query(
            "UPDATE ent_dem_demande SET statut = 'prise', attribuee_a_agent_id = $4, attribuee_le = now(), version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(a.ligne.version)
        .bind(agent)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit(
                "La demande a changé entre-temps : rechargez-la.",
            ));
        }
        sqlx::query("UPDATE ent_dem_echeance SET traitee_le = now() WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'prise_en_charge' AND traitee_le IS NULL")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let mut f = FaitNouveau::nouveau(id, "prise");
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some(&a.ligne.statut);
        f.statut_apres = Some("prise");
        f.agent_apres = Some(agent);
        f.motif = Some("en_impression");
        ajouter_fait(&mut tx, etab, f).await?;
        let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, NATURE_REPRO, "prise");
        publier_enveloppe(&mut tx, etab, types::DEMANDE_PRISE, &env, Some(user_id)).await?;
        tx.commit().await?;
        Ok(EtatDemande {
            id,
            numero: a.ligne.numero,
            statut: "prise".into(),
            version: a.ligne.version + 1,
        })
    }

    /// Marque la demande **prête** (DEM-299) : un **geste authentifié** du réceptionnaire du site, avec le lieu de
    /// retrait. Aucun lien public, aucun jeton (NJ-92) : sans session, rien n'existe pour agir.
    pub async fn marquer_prete_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        id: Uuid,
        lieu_retrait: Option<Uuid>,
    ) -> Result<EtatDemande, RefusDemande> {
        self.limiter(
            "demandes.geste",
            user_id,
            QuotasDemandes::depuis_env().geste,
        )
        .await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = acces_repro(self, &mut tx, etab, user_id, role, id, true).await?;
        if !a.est(ProfilRepro::Receptionnaire) {
            return Err(RefusDemande::Interdit);
        }
        if let Some(l) = lieu_retrait {
            lieu_du_site(&mut tx, etab, l, a.site_id).await?;
        }
        let n = sqlx::query(
            "UPDATE ent_dem_repro SET etat = 'prete', prete_par_personne_id = $3, prete_le = now(), prete_retrait_lieu_id = $4
              WHERE etablissement_id = $1 AND demande_id = $2 AND etat IN ('a_imprimer', 'en_impression')",
        )
        .bind(etab)
        .bind(id)
        .bind(a.moi)
        .bind(lieu_retrait)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit(
                "Cette demande n'est pas en cours d'impression.",
            ));
        }
        let n = sqlx::query(
            "UPDATE ent_dem_demande SET statut = 'traitee', version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(a.ligne.version)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit(
                "La demande a changé entre-temps : rechargez-la.",
            ));
        }
        sqlx::query("UPDATE ent_dem_echeance SET traitee_le = now() WHERE etablissement_id = $1 AND demande_id = $2 AND traitee_le IS NULL")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let mut f = FaitNouveau::nouveau(id, "repro_prete");
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some(&a.ligne.statut);
        f.statut_apres = Some("traitee");
        ajouter_fait(&mut tx, etab, f).await?;
        let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, NATURE_REPRO, "repro_prete");
        publier_enveloppe(
            &mut tx,
            etab,
            types::DEMANDE_REPRO_PRETE,
            &env,
            Some(user_id),
        )
        .await?;
        tx.commit().await?;
        Ok(EtatDemande {
            id,
            numero: a.ligne.numero,
            statut: "traitee".into(),
            version: a.ligne.version + 1,
        })
    }

    /// Marque les copies **retirées** (DEM-300) : par le réceptionnaire ou par le demandeur. La demande se clôt.
    pub async fn marquer_retiree_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<EtatDemande, RefusDemande> {
        self.limiter(
            "demandes.geste",
            user_id,
            QuotasDemandes::depuis_env().geste,
        )
        .await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = acces_repro(self, &mut tx, etab, user_id, role, id, true).await?;
        if !a.est(ProfilRepro::Receptionnaire) && !a.demandeur_ou_deposant() {
            return Err(RefusDemande::Interdit);
        }
        let n = sqlx::query(
            "UPDATE ent_dem_repro SET etat = 'retiree', retiree_le = now()
              WHERE etablissement_id = $1 AND demande_id = $2 AND etat = 'prete'",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit("Ces copies ne sont pas prêtes."));
        }
        let jour = jour_metier(&mut tx, etab).await?;
        let conservation = Conservation::Reprographie
            .duree_mois()
            .and_then(|m| jour.checked_add_months(Months::new(m)));
        let n = sqlx::query(
            "UPDATE ent_dem_demande
                SET statut = 'close', issue = 'traitee', clos_le = now(), conservation_jusqu_au = $4, version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(a.ligne.version)
        .bind(conservation)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit(
                "La demande a changé entre-temps : rechargez-la.",
            ));
        }
        let mut f = FaitNouveau::nouveau(id, "close");
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some(&a.ligne.statut);
        f.statut_apres = Some("close");
        f.motif = Some("traitee");
        ajouter_fait(&mut tx, etab, f).await?;
        let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, NATURE_REPRO, "close");
        publier_enveloppe(&mut tx, etab, types::DEMANDE_CLOSE, &env, Some(user_id)).await?;
        tx.commit().await?;
        Ok(EtatDemande {
            id,
            numero: a.ligne.numero,
            statut: "close".into(),
            version: a.ligne.version + 1,
        })
    }

    /// Le demandeur annule tant que l'impression n'a pas commencé (DEM-303).
    pub async fn annuler_repro_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<EtatDemande, RefusDemande> {
        self.limiter(
            "demandes.geste",
            user_id,
            QuotasDemandes::depuis_env().geste,
        )
        .await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = acces_repro(self, &mut tx, etab, user_id, role, id, true).await?;
        if !a.demandeur_ou_deposant() {
            return Err(RefusDemande::Interdit);
        }
        let n = sqlx::query(
            "UPDATE ent_dem_repro SET etat = 'annulee' WHERE etablissement_id = $1 AND demande_id = $2 AND etat IN ('a_valider', 'a_imprimer')",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit(
                "L'impression est commencée : écrivez à la reprographie.",
            ));
        }
        let n = sqlx::query(
            "UPDATE ent_dem_demande
                SET statut = 'annulee', issue = 'annulee_par_demandeur', clos_le = now(), version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(a.ligne.version)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit(
                "La demande a changé entre-temps : rechargez-la.",
            ));
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
        f.motif = Some("annulee_par_demandeur");
        ajouter_fait(&mut tx, etab, f).await?;
        let env = EnveloppeNeutre::nouvelle(id, &a.ligne.numero, NATURE_REPRO, "close");
        publier_enveloppe(&mut tx, etab, types::DEMANDE_CLOSE, &env, Some(user_id)).await?;
        tx.commit().await?;
        Ok(EtatDemande {
            id,
            numero: a.ligne.numero,
            statut: "annulee".into(),
            version: a.ligne.version + 1,
        })
    }

    /// Le réceptionnaire signale un problème (DEM-301) : un **message fixe** au demandeur (jamais de texte libre dans
    /// l'automatique) et la demande attend sa réponse.
    pub async fn signaler_probleme_repro_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        id: Uuid,
        probleme: &str,
    ) -> Result<EtatDemande, RefusDemande> {
        self.limiter(
            "demandes.geste",
            user_id,
            QuotasDemandes::depuis_env().geste,
        )
        .await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let phrase = PROBLEMES
            .iter()
            .find(|(c, _)| *c == probleme)
            .map(|(_, p)| *p)
            .ok_or(RefusDemande::Invalide("Ce type de problème n'existe pas."))?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = acces_repro(self, &mut tx, etab, user_id, role, id, true).await?;
        if !a.est(ProfilRepro::Receptionnaire) {
            return Err(RefusDemande::Interdit);
        }
        if !matches!(a.etat.as_str(), "a_imprimer" | "en_impression")
            || a.ligne.statut == "en_attente_demandeur"
        {
            return Err(RefusDemande::Conflit(
                "Cette demande n'est pas en cours de traitement.",
            ));
        }
        let mid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_dem_message (id, etablissement_id, demande_id, auteur_personne_id, visibilite, genre, texte)
             VALUES ($1, $2, $3, $4, 'demandeur', 'complement_demande', $5)",
        )
        .bind(mid)
        .bind(etab)
        .bind(id)
        .bind(a.moi)
        .bind(phrase)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE ent_dem_repro SET probleme_signale_le = now() WHERE etablissement_id = $1 AND demande_id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let n = sqlx::query(
            "UPDATE ent_dem_demande SET statut = 'en_attente_demandeur', version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(a.ligne.version)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit(
                "La demande a changé entre-temps : rechargez-la.",
            ));
        }
        let mut f = FaitNouveau::nouveau(id, "complement_demande");
        f.auteur_personne = Some(a.moi);
        f.statut_avant = Some(&a.ligne.statut);
        f.statut_apres = Some("en_attente_demandeur");
        f.motif = Some(
            PROBLEMES
                .iter()
                .find(|(c, _)| *c == probleme)
                .map_or("autre", |(c, _)| *c),
        );
        f.ref_id = Some(mid);
        ajouter_fait(&mut tx, etab, f).await?;
        let env =
            EnveloppeNeutre::nouvelle(id, &a.ligne.numero, NATURE_REPRO, "complement_demande");
        publier_enveloppe(&mut tx, etab, types::DEMANDE_ECHANGE, &env, Some(user_id)).await?;
        tx.commit().await?;
        Ok(EtatDemande {
            id,
            numero: a.ligne.numero,
            statut: "en_attente_demandeur".into(),
            version: a.ligne.version + 1,
        })
    }

    /// Transfère la demande à la file d'un **autre site** (DEM-302, CDM-54 : copieur en panne). Le site cible doit
    /// avoir un réceptionnaire désigné ; le lieu de retrait sera choisi à « prête ».
    pub async fn transferer_repro_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        id: Uuid,
        site_cible: Uuid,
    ) -> Result<(), RefusDemande> {
        self.limiter(
            "demandes.geste",
            user_id,
            QuotasDemandes::depuis_env().geste,
        )
        .await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = acces_repro(self, &mut tx, etab, user_id, role, id, true).await?;
        if !a.est(ProfilRepro::Receptionnaire) {
            return Err(RefusDemande::Interdit);
        }
        if site_cible == a.site_id {
            return Err(RefusDemande::Invalide("Choisissez un autre site."));
        }
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_dem_repro_receptionnaire
                             WHERE etablissement_id = $1 AND site_id = $2
                               AND du <= ent_aujourdhui_etab($1, now()) AND (au IS NULL OR au > ent_aujourdhui_etab($1, now())))",
        )
        .bind(etab)
        .bind(site_cible)
        .fetch_one(&mut *tx)
        .await?;
        if !ok {
            return Err(RefusDemande::Conflit("Ce site n'a personne pour imprimer."));
        }
        let n = sqlx::query(
            "UPDATE ent_dem_repro SET site_id = $3, retrait_lieu_id = NULL, etat = 'a_imprimer', en_impression_le = NULL
              WHERE etablissement_id = $1 AND demande_id = $2 AND etat IN ('a_imprimer', 'en_impression')",
        )
        .bind(etab)
        .bind(id)
        .bind(site_cible)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit("Cette demande n'est pas à imprimer."));
        }
        // La prise (si elle existait) revient à la file : le nouveau site la reprend.
        let n = sqlx::query(
            "UPDATE ent_dem_demande SET statut = 'deposee', attribuee_a_agent_id = NULL, attribuee_le = NULL, version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(a.ligne.version)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit(
                "La demande a changé entre-temps : rechargez-la.",
            ));
        }
        let mut f = FaitNouveau::nouveau(id, "repro_transferee");
        f.auteur_personne = Some(a.moi);
        f.ref_id = Some(site_cible);
        ajouter_fait(&mut tx, etab, f).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Le détail d'une demande de copies pour son demandeur, son déposant, le réceptionnaire du site ou le validateur.
    /// Quand le demandeur ouvre sa demande **prête**, « prête, pas encore vue » disparaît.
    pub async fn detail_repro_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<DetailRepro, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = acces_repro(self, &mut tx, etab, user_id, role, id, false).await?;
        type Ligne = (
            String,
            Uuid,
            bool,
            bool,
            bool,
            String,
            i32,
            i32,
            i32,
            NaiveDate,
            bool,
            bool,
            bool,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        );
        let r: Ligne = sqlx::query_as(
            "SELECT r.etat, r.site_id, r.couleur, r.recto_verso, r.agrafage, r.format, r.pages_total, r.exemplaires_total,
                    r.copies_total, r.pour_le, r.validation_requise, r.avertissement_delai, r.avertissement_cfc,
                    (SELECT lr.libelle FROM ent_dem_repro_lieu_retrait lr WHERE lr.etablissement_id = r.etablissement_id
                        AND lr.id = COALESCE(r.prete_retrait_lieu_id, r.retrait_lieu_id)),
                    r.prete_le, r.prete_vue_le
               FROM ent_dem_repro r WHERE r.etablissement_id = $1 AND r.demande_id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if a.demandeur_ou_deposant() && r.0 == "prete" && r.15.is_none() {
            sqlx::query("UPDATE ent_dem_repro SET prete_vue_le = now() WHERE etablissement_id = $1 AND demande_id = $2 AND prete_vue_le IS NULL")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        let documents: Vec<(Uuid,)> = if a.demandeur_ou_deposant()
            || a.est(ProfilRepro::Receptionnaire)
            || a.est(ProfilRepro::Validateur)
        {
            sqlx::query_as(
                "SELECT document_id FROM ent_dem_repro_document WHERE etablissement_id = $1 AND demande_id = $2 AND efface_le IS NULL ORDER BY cree_le, id",
            )
            .bind(etab)
            .bind(id)
            .fetch_all(&mut *tx)
            .await?
        } else {
            Vec::new()
        };
        let cibles: Vec<(Option<Uuid>, Option<Uuid>, i32, Option<Uuid>)> = sqlx::query_as(
            "SELECT classe_id, groupe_id, exemplaires, retrait_lieu_id FROM ent_dem_repro_cible WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        let prete_non_vue = a.demandeur_ou_deposant() && r.0 == "prete" && r.15.is_none();
        Ok(DetailRepro {
            demande_id: id,
            numero: a.ligne.numero,
            statut: a.ligne.statut,
            etat: r.0,
            site_id: r.1,
            couleur: r.2,
            recto_verso: r.3,
            agrafage: r.4,
            format: r.5,
            pages_total: r.6,
            exemplaires_total: r.7,
            copies_total: r.8,
            pour_le: r.9,
            validation_requise: r.10,
            avertissement_delai: r.11,
            avertissement_cfc: r.12,
            retrait_lieu: r.13,
            prete_le: r.14,
            prete_non_vue,
            documents: documents.into_iter().map(|(d,)| d).collect(),
            cibles: cibles
                .into_iter()
                .map(|(c, g, e, l)| CibleRepro {
                    classe_id: c,
                    groupe_id: g,
                    exemplaires: e,
                    retrait_lieu_id: l,
                })
                .collect(),
        })
    }
}

// ───────────────────────── Dupliquer et programmer une récurrence (R-DM52) ─────────────────────────

impl PgEnt {
    /// Prépare une nouvelle demande à partir de la mienne (DEM-366) : **options et quantités seulement**, jamais les
    /// fichiers ni la demande d'un autre. Le demandeur dépose ensuite normalement.
    pub async fn dupliquer_repro_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ModeleRepro, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = acces_repro(self, &mut tx, etab, user_id, role, id, false).await?;
        // R-DM52 : « du même auteur » — ni le réceptionnaire ni le validateur ne copient la demande d'un autre.
        if !a.demandeur_ou_deposant() {
            return Err(RefusDemande::Introuvable);
        }
        let m = modele_de(&mut tx, etab, id, &a.ligne).await?;
        tx.commit().await?;
        Ok(m)
    }

    /// Programme un rappel hebdomadaire sur une de mes demandes (DEM-367) : le rappel prépare un brouillon à
    /// redéposer, **jamais un dépôt automatique**.
    pub async fn programmer_recurrence_repro_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        id: Uuid,
        jour_semaine: i16,
    ) -> Result<Uuid, RefusDemande> {
        if !(1..=7).contains(&jour_semaine) {
            return Err(RefusDemande::Invalide(
                "Le jour de la semaine est de 1 (lundi) à 7 (dimanche).",
            ));
        }
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a = acces_repro(self, &mut tx, etab, user_id, role, id, false).await?;
        if !a.demandeur_ou_deposant() {
            return Err(RefusDemande::Introuvable);
        }
        let rid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_dem_repro_recurrence (id, etablissement_id, personne_id, modele_demande_id, jour_semaine)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (etablissement_id, personne_id, modele_demande_id) WHERE actif
             DO UPDATE SET jour_semaine = EXCLUDED.jour_semaine
             RETURNING id",
        )
        .bind(rid)
        .bind(etab)
        .bind(a.moi)
        .bind(id)
        .bind(jour_semaine)
        .execute(&mut *tx)
        .await?;
        let (reel,): (Uuid,) = sqlx::query_as(
            "SELECT id FROM ent_dem_repro_recurrence WHERE etablissement_id = $1 AND personne_id = $2 AND modele_demande_id = $3 AND actif",
        )
        .bind(etab)
        .bind(a.moi)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(reel)
    }

    /// Les brouillons à redéposer **aujourd'hui** (jour de l'établissement) : un seul rappel par jour et par
    /// récurrence. Rien n'est déposé.
    pub async fn recurrences_dues_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<(Uuid, ModeleRepro)>, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let iso = i16::try_from(jour.weekday().number_from_monday()).unwrap_or(1);
        let l: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "UPDATE ent_dem_repro_recurrence SET dernier_rappel_le = $3
              WHERE etablissement_id = $1 AND personne_id = $2 AND actif AND jour_semaine = $4
                AND (dernier_rappel_le IS NULL OR dernier_rappel_le < $3)
             RETURNING id, modele_demande_id",
        )
        .bind(etab)
        .bind(moi)
        .bind(jour)
        .bind(iso)
        .fetch_all(&mut *tx)
        .await?;
        let mut sortie = Vec::new();
        for (rid, modele) in l {
            let ligne = charger_demande(&mut tx, etab, modele, false).await?;
            sortie.push((rid, modele_de(&mut tx, etab, modele, &ligne).await?));
        }
        tx.commit().await?;
        Ok(sortie)
    }
}

async fn modele_de(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    ligne: &LigneDemande,
) -> Result<ModeleRepro, RefusDemande> {
    let r: (Uuid, String, bool, bool, bool, i32, Option<Uuid>, Option<String>) = sqlx::query_as(
        "SELECT r.site_id, r.format, r.couleur, r.recto_verso, r.agrafage, r.exemplaires_total, r.retrait_lieu_id, d.objet
           FROM ent_dem_repro r JOIN ent_dem_demande d ON d.etablissement_id = r.etablissement_id AND d.id = r.demande_id
          WHERE r.etablissement_id = $1 AND r.demande_id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(conn)
    .await?;
    let _ = ligne;
    Ok(ModeleRepro {
        site_id: r.0,
        format: r.1,
        couleur: r.2,
        recto_verso: r.3,
        agrafage: r.4,
        exemplaires_total: r.5,
        retrait_lieu_id: r.6,
        objet: r.7,
    })
}

// ───────────────────────── Réglages du site, désignations, compteurs ─────────────────────────

impl PgEnt {
    /// Règle le seuil noir et blanc et le délai d'un site (DEM-305, SN1-11, SN1-12). Journalisé.
    pub async fn regler_site_repro_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        site: Uuid,
        seuil_copies: i32,
        delai_jours: i16,
    ) -> Result<ReglageSiteRepro, RefusDemande> {
        self.limiter(
            "demandes.geste",
            user_id,
            QuotasDemandes::depuis_env().geste,
        )
        .await?;
        if !(1..=100_000).contains(&seuil_copies) || !(0..=15).contains(&delai_jours) {
            return Err(RefusDemande::Invalide(
                "Le seuil ou le délai est hors limites.",
            ));
        }
        if !self
            .a_action_etablissement(etab, user_id, role, Action::DemandesParametrer)
            .await?
        {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_site WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(site)
        .fetch_one(&mut *tx)
        .await?;
        if !existe {
            return Err(RefusDemande::Invalide("Ce site n'existe pas."));
        }
        let (a_seuil, a_delai) = reglage_site(&mut tx, etab, site).await?;
        sqlx::query(
            "INSERT INTO ent_dem_repro_site_reglage (etablissement_id, site_id, seuil_copies, delai_jours, maj_par_user_id)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (etablissement_id, site_id) DO UPDATE
                SET seuil_copies = EXCLUDED.seuil_copies, delai_jours = EXCLUDED.delai_jours, maj_le = now(),
                    maj_par_user_id = EXCLUDED.maj_par_user_id",
        )
        .bind(etab)
        .bind(site)
        .bind(seuil_copies)
        .bind(delai_jours)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO ent_dem_reglage_journal (id, etablissement_id, sujet, cible, avant, apres, par_user_id)
             VALUES ($1, $2, 'reprographie', $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(format!("site {site}"))
        .bind(format!("seuil={a_seuil} delai={a_delai}"))
        .bind(format!("seuil={seuil_copies} delai={delai_jours}"))
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ReglageSiteRepro {
            site_id: site,
            seuil_copies,
            delai_jours,
        })
    }

    /// Ajoute un lieu de retrait à un site (DEM-305).
    pub async fn ajouter_lieu_retrait_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        site: Uuid,
        libelle: &str,
    ) -> Result<Uuid, RefusDemande> {
        self.limiter(
            "demandes.geste",
            user_id,
            QuotasDemandes::depuis_env().geste,
        )
        .await?;
        let libelle = libelle.trim();
        if libelle.is_empty() || libelle.chars().count() > 80 || libelle.contains('\0') {
            return Err(RefusDemande::Invalide(
                "Le lieu de retrait est obligatoire (80 caractères au plus).",
            ));
        }
        if !self
            .a_action_etablissement(etab, user_id, role, Action::DemandesParametrer)
            .await?
        {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_site WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(site)
        .fetch_one(&mut *tx)
        .await?;
        if !existe {
            return Err(RefusDemande::Invalide("Ce site n'existe pas."));
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_dem_repro_lieu_retrait (id, etablissement_id, site_id, libelle) VALUES ($1, $2, $3, $4)")
            .bind(id)
            .bind(etab)
            .bind(site)
            .bind(libelle)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Désigne un agent réceptionnaire d'un site, ou met fin à sa désignation (DEM-296, R-RM27). Journalisé.
    pub async fn designer_receptionnaire_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        site: Uuid,
        agent: Uuid,
        designer: bool,
    ) -> Result<(), RefusDemande> {
        self.limiter(
            "demandes.geste",
            user_id,
            QuotasDemandes::depuis_env().geste,
        )
        .await?;
        if !self
            .a_action_etablissement(etab, user_id, role, Action::DemandesParametrer)
            .await?
        {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        if designer {
            sqlx::query(
                "INSERT INTO ent_dem_repro_receptionnaire (id, etablissement_id, site_id, agent_id, du, designe_par_user_id)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (etablissement_id, site_id, agent_id) WHERE au IS NULL DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(site)
            .bind(agent)
            .bind(jour)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| RefusDemande::Invalide("Ce site ou cet agent n'existe pas."))?;
        } else {
            sqlx::query(
                "UPDATE ent_dem_repro_receptionnaire SET au = $4
                  WHERE etablissement_id = $1 AND site_id = $2 AND agent_id = $3 AND au IS NULL",
            )
            .bind(etab)
            .bind(site)
            .bind(agent)
            .bind(jour)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO ent_dem_reglage_journal (id, etablissement_id, sujet, cible, avant, apres, par_user_id)
             VALUES ($1, $2, 'reprographie', $3, NULL, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(format!("receptionnaire site {site}"))
        .bind(if designer { "designe" } else { "retire" })
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Compteurs simples (DEM-304) : copies par site, demandeur et couleur, pour un mois. Export CSV pour une
    /// refacturation interne éventuelle. Réservé à qui valide la reprographie ou gère l'enveloppe.
    pub async fn compteurs_repro_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        mois: NaiveDate,
    ) -> Result<(Vec<LigneCompteur>, String), RefusDemande> {
        let ok = self
            .a_action_etablissement(etab, user_id, role, Action::DemandesReproValider)
            .await?
            || self
                .a_action_etablissement(etab, user_id, role, Action::DemandesHseEnveloppeGerer)
                .await?;
        if !ok {
            return Err(RefusDemande::Interdit);
        }
        let debut = mois
            .with_day(1)
            .ok_or(RefusDemande::Invalide("Le mois est invalide."))?;
        let fin = debut
            .checked_add_months(Months::new(1))
            .ok_or(RefusDemande::Invalide("Le mois est invalide."))?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let l: Vec<(Uuid, String, bool, i64, i64)> = sqlx::query_as(
            "SELECT r.site_id, btrim(COALESCE(p.prenom, '') || ' ' || p.nom), r.couleur, count(*)::bigint, sum(r.copies_total)::bigint
               FROM ent_dem_repro r
               JOIN ent_dem_demande d ON d.etablissement_id = r.etablissement_id AND d.id = r.demande_id
               JOIN ent_personne p ON p.etablissement_id = d.etablissement_id AND p.id = d.demandeur_personne_id
              WHERE r.etablissement_id = $1 AND r.etat IN ('prete', 'retiree')
                AND ent_aujourdhui_etab($1, r.prete_le) >= $2 AND ent_aujourdhui_etab($1, r.prete_le) < $3
              GROUP BY r.site_id, p.nom, p.prenom, r.couleur
              ORDER BY r.site_id, 2, r.couleur",
        )
        .bind(etab)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        let mut csv = String::from("site;demandeur;couleur;demandes;copies\n");
        let mut lignes = Vec::new();
        for (site_id, nom, couleur, demandes, copies) in l {
            csv.push_str(&format!(
                "{site_id};{};{};{demandes};{copies}\n",
                cellule_csv(&nom),
                if couleur { "couleur" } else { "noir_et_blanc" }
            ));
            lignes.push(LigneCompteur {
                site_id,
                demandeur_nom: nom,
                couleur,
                demandes,
                copies,
            });
        }
        Ok((lignes, csv))
    }
}

// ───────────────────────── Tâches de l'ordonnanceur ─────────────────────────

impl PgEnt {
    /// Rappelle au demandeur des copies prêtes **non retirées à J+3** (DEM-306, CDM-55) : un seul rappel par demande,
    /// jour d'établissement. Rapport sans nom.
    pub async fn rappeler_copies_non_retirees(
        &self,
        etab: Uuid,
    ) -> Result<RapportRepro, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let l: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT r.demande_id, d.numero FROM ent_dem_repro r
               JOIN ent_dem_demande d ON d.etablissement_id = r.etablissement_id AND d.id = r.demande_id
              WHERE r.etablissement_id = $1 AND r.etat = 'prete' AND r.rappel_retrait_le IS NULL
                AND ent_aujourdhui_etab($1, r.prete_le) + 3 <= $2
              ORDER BY r.prete_le, r.demande_id LIMIT 500 FOR UPDATE OF r",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await?;
        let mut rapport = RapportRepro::default();
        for (id, numero) in l {
            sqlx::query("UPDATE ent_dem_repro SET rappel_retrait_le = now() WHERE etablissement_id = $1 AND demande_id = $2")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            let mut f = FaitNouveau::nouveau(id, "repro_rappel_retrait");
            f.auteur_tache = Some("ordonnanceur_echeances");
            ajouter_fait(&mut tx, etab, f).await?;
            let env = EnveloppeNeutre::nouvelle(id, &numero, NATURE_REPRO, "repro_rappel");
            publier_enveloppe(&mut tx, etab, types::DEMANDE_REPRO_RAPPEL, &env, None).await?;
            rapport.traitees += 1;
        }
        tx.commit().await?;
        Ok(rapport)
    }

    /// Efface les PDF échus (DEM-256, §18.5, CDM-55) : 30 jours après le retrait, 60 jours après « prête » sans
    /// retrait, 30 jours après un refus ou une annulation. **Le socle 7 efface** ; N1 ne marque `efface_le` que s'il
    /// le confirme (sinon rien, et on recommence au passage suivant).
    pub async fn effacer_pdf_repro_echus(
        &self,
        etab: Uuid,
        docs: &dyn PortDocuments,
    ) -> Result<RapportRepro, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let l: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT x.demande_id, x.document_id
               FROM ent_dem_repro_document x
               JOIN ent_dem_repro r ON r.etablissement_id = x.etablissement_id AND r.demande_id = x.demande_id
               JOIN ent_dem_demande d ON d.etablissement_id = r.etablissement_id AND d.id = r.demande_id
              WHERE x.etablissement_id = $1 AND x.efface_le IS NULL
                AND ((r.etat = 'retiree' AND ent_aujourdhui_etab($1, r.retiree_le) + 30 <= $2)
                  OR (r.etat = 'prete' AND ent_aujourdhui_etab($1, r.prete_le) + 60 <= $2)
                  OR (r.etat IN ('refusee', 'annulee') AND d.clos_le IS NOT NULL AND ent_aujourdhui_etab($1, d.clos_le) + 30 <= $2))
              ORDER BY x.demande_id, x.id LIMIT 500 FOR UPDATE OF x",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await?;
        let mut rapport = RapportRepro::default();
        for (demande, document) in l {
            if !docs.effacer(etab, document) {
                continue;
            }
            sqlx::query(
                "UPDATE ent_dem_repro_document SET efface_le = now()
                  WHERE etablissement_id = $1 AND demande_id = $2 AND document_id = $3 AND efface_le IS NULL",
            )
            .bind(etab)
            .bind(demande)
            .bind(document)
            .execute(&mut *tx)
            .await?;
            let mut f = FaitNouveau::nouveau(demande, "repro_pdf_efface");
            f.auteur_tache = Some("purge_conservation");
            f.ref_id = Some(document);
            ajouter_fait(&mut tx, etab, f).await?;
            rapport.traitees += 1;
        }
        tx.commit().await?;
        Ok(rapport)
    }
}
