//! Socle N1, lot N1-1 : **routage déterministe**, natures activées, catalogue, simulateur, accusé de réception et
//! adaptateur réel des sources du cœur ([`SourcesFiles`], qui remplace `SourcesAbsentes`).
//!
//! Le routage est **déterministe et explicable** (§10.1) : règles de l'établissement, puis file de la nature, puis
//! file de tri du site ; jamais le texte libre, jamais l'IA (NJ-87). Une file sans destinataire (départ, absence du
//! seul membre, personne visée) passe à la file de repli, puis à la direction : **jamais à tout le personnel**
//! (NJ-85, R-DM7). La personne visée n'est jamais destinataire (R-DM47, Q-DM14).

use crate::demandes::{
    nature, DepotDemande, PublicNature, RefusDemande, RoutageResolu, SourcesDemandes, Urgence, Valeur,
};
use crate::demandes_catalogue::{self as cat, NatureMeta};
use crate::demandes_files::{charger_graphe, code_ok, jour_metier, journaliser, FileLigne, Graphe};
use crate::service::PgEnt;
use crate::structure_calendrier::calendrier;
use crate::tenant_tx::TenantTx;
use chrono::NaiveDate;
use ent_auth::Role;
use sqlx::PgConnection;
use std::collections::BTreeMap;
use uuid::Uuid;

// ───────────────────────── Vocabulaire ─────────────────────────

pub fn code_public(p: PublicNature) -> &'static str {
    match p {
        PublicNature::Personnel => "personnel",
        PublicNature::Responsable => "responsable",
        PublicNature::EleveMajeur => "eleve_majeur",
        PublicNature::EleveMineur => "eleve_mineur",
    }
}

pub fn parse_public(s: &str) -> Option<PublicNature> {
    Some(match s {
        "personnel" => PublicNature::Personnel,
        "responsable" => PublicNature::Responsable,
        "eleve_majeur" => PublicNature::EleveMajeur,
        "eleve_mineur" => PublicNature::EleveMineur,
        _ => return None,
    })
}

/// Phrase du refus quand une nature n'est pas proposée (CDM-07).
pub const NATURE_NON_PROPOSEE: &str = "Cette démarche n'est plus proposée. Adressez-vous au secrétariat.";

/// Mentions de l'accusé de réception d'une nature à décision de l'usager (EPLE, CRPA, §6.1) [NV : formulation à
/// valider par la direction].
pub const MENTIONS_ACCUSE_EPLE: &[&str] = &[
    "Votre demande a bien été reçue : cet accusé indique la date de réception, le service qui la traite et un délai indicatif.",
    "Selon la nature de la démarche, le silence gardé pendant deux mois peut valoir décision implicite ; les voies de recours vous sont indiquées avec la réponse.",
];

// ───────────────────────── Natures activées ─────────────────────────

/// Ce que l'établissement règle pour une nature : activer, ouvrir des publics, raccourcir, désigner la file.
#[derive(Debug, Clone, Default)]
pub struct ReglageNature {
    pub actif: bool,
    /// Parmi les publics que la nature autorise (jamais plus).
    pub publics: Vec<PublicNature>,
    pub delai_prise_jours: Option<i16>,
    pub delai_traitement_jours: Option<i16>,
    pub file_defaut_code: Option<String>,
    pub texte_aide: Option<String>,
}

#[derive(Debug, Clone)]
struct NatureEtab {
    actif: bool,
    publics: Vec<String>,
    delai_prise: Option<i16>,
    delai_traitement: Option<i16>,
    file_defaut: Option<Uuid>,
}

async fn charger_natures_etab(conn: &mut PgConnection, etab: Uuid) -> Result<BTreeMap<String, NatureEtab>, RefusDemande> {
    let l: Vec<(String, bool, Option<i16>, Option<i16>, Option<Uuid>)> = sqlx::query_as(
        "SELECT nature_code, actif, delai_prise_jours, delai_traitement_jours, file_defaut_id
           FROM ent_dem_nature_etab WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await?;
    let mut m: BTreeMap<String, NatureEtab> = l
        .into_iter()
        .map(|(c, actif, p, t, f)| (c, NatureEtab { actif, publics: Vec::new(), delai_prise: p, delai_traitement: t, file_defaut: f }))
        .collect();
    let pubs: Vec<(String, String)> =
        sqlx::query_as("SELECT nature_code, public_code FROM ent_dem_nature_public WHERE etablissement_id = $1").bind(etab).fetch_all(conn).await?;
    for (c, p) in pubs {
        if let Some(n) = m.get_mut(&c) {
            n.publics.push(p);
        }
    }
    Ok(m)
}

// ───────────────────────── Règles de routage ─────────────────────────

#[derive(Debug, Clone)]
pub struct NouvelleRegleRoutage {
    pub nature_code: String,
    pub categorie_code: Option<String>,
    pub site_id: Option<Uuid>,
    pub niveau_code: Option<String>,
    pub public: Option<PublicNature>,
    pub urgence: Option<Urgence>,
    pub destination_file_id: Uuid,
    pub ordre: i16,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
struct RegleRoutage {
    id: Uuid,
    nature_code: String,
    categorie: Option<String>,
    site_id: Option<Uuid>,
    niveau: Option<String>,
    public: Option<String>,
    urgence: Option<String>,
    destination: Uuid,
    ordre: i16,
}

async fn charger_regles(conn: &mut PgConnection, etab: Uuid, nature_code: &str, jour: NaiveDate) -> Result<Vec<RegleRoutage>, RefusDemande> {
    type L = (Uuid, String, Option<String>, Option<Uuid>, Option<String>, Option<String>, Option<String>, Uuid, i16);
    let l: Vec<L> = sqlx::query_as(
        "SELECT id, nature_code, categorie_code, site_id, niveau_code, public_code, urgence, destination_file_id, ordre
           FROM ent_dem_routage_regle
          WHERE etablissement_id = $1 AND nature_code = $2 AND actif AND du <= $3 AND (au IS NULL OR au >= $3)
          ORDER BY ordre, id",
    )
    .bind(etab)
    .bind(nature_code)
    .bind(jour)
    .fetch_all(conn)
    .await?;
    Ok(l.into_iter()
        .map(|(id, nature_code, categorie, site_id, niveau, public, urgence, destination, ordre)| RegleRoutage {
            id,
            nature_code,
            categorie,
            site_id,
            niveau,
            public,
            urgence,
            destination,
            ordre,
        })
        .collect())
}

// ───────────────────────── Résolution ─────────────────────────

/// Critères d'une demande, **tous structurés** : jamais le texte libre (§10.5).
#[derive(Debug, Clone)]
pub struct CriteresRoutage {
    pub nature_code: String,
    pub categorie: Option<String>,
    /// Site effectif : celui du lieu, à défaut celui de l'élève ou du demandeur (CDM-04).
    pub site_id: Option<Uuid>,
    pub niveau_code: Option<String>,
    pub public: PublicNature,
    pub urgence: Urgence,
}

#[derive(Debug, Clone)]
pub struct Resolution {
    pub file: FileLigne,
    pub motif: &'static str,
    pub ref_id: Option<Uuid>,
    /// File visée avant le repli, le cas échéant.
    pub initiale: Option<FileLigne>,
    pub raison: String,
    pub delai_prise_jours: u16,
    pub delai_traitement_jours: u16,
}

impl RegleRoutage {
    fn s_applique(&self, c: &CriteresRoutage) -> bool {
        self.categorie.as_ref().is_none_or(|x| c.categorie.as_ref() == Some(x))
            && self.site_id.is_none_or(|x| c.site_id == Some(x))
            && self.niveau.as_ref().is_none_or(|x| c.niveau_code.as_ref() == Some(x))
            && self.public.as_deref().is_none_or(|x| x == code_public(c.public))
            && self.urgence.as_deref().is_none_or(|x| x == c.urgence.code())
    }
}

/// Délais effectifs : la nature déclare, l'établissement **raccourcit** (jamais ne rallonge), la file borne.
pub fn delais_effectifs(decl_prise: u16, decl_traitement: u16, ne: Option<(Option<i16>, Option<i16>)>, file: &FileLigne) -> (u16, u16) {
    let mut prise = decl_prise;
    let mut traitement = decl_traitement;
    if let Some((p, t)) = ne {
        if let Some(p) = p.and_then(|x| u16::try_from(x).ok()) {
            prise = prise.min(p);
        }
        if let Some(t) = t.and_then(|x| u16::try_from(x).ok()) {
            traitement = traitement.min(t);
        }
    }
    if let Some(v) = file.delai_vise_jours.and_then(|x| u16::try_from(x).ok()) {
        traitement = traitement.min(v);
    }
    (prise, traitement.max(prise))
}

fn repli_de(g: &Graphe, repli_rh: Option<Uuid>, depart: &FileLigne, vises: &[Uuid]) -> Option<(FileLigne, &'static str)> {
    if !g.membres_hors(depart.id, vises).is_empty() {
        return Some((depart.clone(), ""));
    }
    // La file avait des membres, mais ils sont tous visés : on le distingue d'une file vide (signal différent).
    let avait = g.membres.get(&depart.id).is_some_and(|m| !m.is_empty());
    let motif = if avait { "repli_personne_visee" } else { "repli_file_vide" };
    // Une file confidentielle se replie sur le repli RH réglé par l'établissement ; les autres sur leur repli.
    let mut cible = if depart.type_ == "confidentielle" { repli_rh.or(depart.repli_id) } else { depart.repli_id };
    let mut vus = vec![depart.id];
    for _ in 0..6 {
        let Some(c) = cible else { break };
        if vus.contains(&c) {
            break;
        }
        vus.push(c);
        let f = g.files.get(&c)?;
        if !g.membres_hors(c, vises).is_empty() {
            return Some((f.clone(), motif));
        }
        cible = f.repli_id;
    }
    // Dernier recours : la direction (R-DM47 : passage au suivant puis à la direction).
    let dir = g.files.values().find(|f| f.type_ == "direction" && !vus.contains(&f.id))?;
    (!g.membres_hors(dir.id, vises).is_empty()).then(|| (dir.clone(), motif))
}

/// Raison lisible des membres d'une file (jamais du demandeur) : « règle n° 10 : maintenance.intervention… ».
fn raison_lisible(motif: &str, c: &CriteresRoutage, regle: Option<&RegleRoutage>, initiale: Option<&FileLigne>, finale: &FileLigne) -> String {
    let base = match (motif, regle) {
        ("regle", Some(r)) => {
            let mut s = format!("règle n° {} : {}", r.ordre, c.nature_code);
            if let Some(x) = &c.categorie {
                s.push_str(&format!(", catégorie {x}"));
            }
            if r.site_id.is_some() {
                s.push_str(", site de la règle");
            }
            if let Some(n) = &c.niveau_code {
                if r.niveau.is_some() {
                    s.push_str(&format!(", niveau {n}"));
                }
            }
            s
        }
        ("file_de_la_nature", _) => format!("file désignée pour la nature {}", c.nature_code),
        ("defaut_nature", _) => format!("file par défaut de la nature {}", c.nature_code),
        ("tri_du_site", _) => "aucune règle ni file par défaut : file de tri".to_string(),
        (m, _) if m.starts_with("repli") => {
            let de = initiale.map_or("?", |f| f.code.as_str());
            let cause = if m == "repli_personne_visee" { "personne visée" } else { "aucun membre actif" };
            format!("file « {de} » : {cause} → repli")
        }
        _ => "routage".to_string(),
    };
    format!("{base} → {}", finale.code)
}

#[allow(clippy::too_many_arguments)]
fn resoudre(
    g: &Graphe,
    repli_rh: Option<Uuid>,
    regles: &[RegleRoutage],
    ne: Option<&NatureEtab>,
    c: &CriteresRoutage,
    vises: &[Uuid],
) -> Option<Resolution> {
    let decl = nature(&c.nature_code)?;
    // 1. première règle qui s'applique ; 2. file de la nature désignée par l'établissement ; 3. file par défaut
    // de la nature ; 4. file de tri du site. On saute une cible inactive ou inconnue.
    let mut choix: Option<(FileLigne, &'static str, Option<Uuid>, Option<&RegleRoutage>)> = None;
    for r in regles.iter().filter(|r| r.s_applique(c)) {
        if let Some(f) = g.files.get(&r.destination) {
            choix = Some((f.clone(), "regle", Some(r.id), Some(r)));
            break;
        }
    }
    if choix.is_none() {
        if let Some(f) = ne.and_then(|n| n.file_defaut).and_then(|id| g.files.get(&id)) {
            choix = Some((f.clone(), "file_de_la_nature", None, None));
        }
    }
    if choix.is_none() {
        if let Some(f) = g.file_par_code(decl.file_defaut) {
            choix = Some((f.clone(), "defaut_nature", None, None));
        }
    }
    if choix.is_none() {
        let tri = g
            .files
            .values()
            .filter(|f| f.type_ == "tri")
            .find(|f| f.site_id.is_some() && f.site_id == c.site_id)
            .or_else(|| g.files.values().find(|f| f.type_ == "tri" && f.site_id.is_none()))
            .or_else(|| g.files.values().find(|f| f.type_ == "tri"));
        if let Some(f) = tri {
            choix = Some((f.clone(), "tri_du_site", None, None));
        }
    }
    let (initiale, mut motif, mut ref_id, regle) = choix?;
    let (finale, repli_motif) = repli_de(g, repli_rh, &initiale, vises)?;
    let mut init = None;
    if !repli_motif.is_empty() {
        init = Some(initiale);
        motif = repli_motif;
        ref_id = init.as_ref().map(|f| f.id);
    }
    let raison = raison_lisible(motif, c, regle, init.as_ref(), &finale);
    let (prise, traitement) = delais_effectifs(
        decl.delai_prise_jours,
        decl.delai_traitement_jours,
        ne.map(|n| (n.delai_prise, n.delai_traitement)),
        &finale,
    );
    Some(Resolution { file: finale, motif, ref_id, initiale: init, raison, delai_prise_jours: prise, delai_traitement_jours: traitement })
}

async fn repli_rh_id(conn: &mut PgConnection, etab: Uuid) -> Result<Option<Uuid>, RefusDemande> {
    let r: Option<(Uuid,)> = sqlx::query_as("SELECT file_id FROM ent_dem_repli_rh WHERE etablissement_id = $1").bind(etab).fetch_optional(conn).await?;
    Ok(r.map(|(i,)| i))
}

// ───────────────────────── Critères d'un dépôt ─────────────────────────

async fn criteres_depot(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    personne: Option<Uuid>,
    public: PublicNature,
    decl_code: &str,
    depot: &DepotDemande,
) -> Result<CriteresRoutage, RefusDemande> {
    let meta: Option<&NatureMeta> = cat::meta(decl_code);
    let categorie = meta.and_then(|m| m.categorie_champ).and_then(|cc| {
        depot.valeurs.iter().find(|v| v.champ == cc).and_then(|v| match &v.valeur {
            Valeur::Code(c) => Some(c.clone()),
            _ => None,
        })
    });
    // CDM-04 : le site du lieu l'emporte sur le site du demandeur.
    let lieu = depot.valeurs.iter().find(|v| v.champ == "lieu_id").and_then(|v| match v.valeur {
        Valeur::Ref(id) => Some(id),
        _ => None,
    });
    let mut site: Option<Uuid> = None;
    if let Some(l) = lieu {
        let r: Option<(Option<Uuid>,)> = sqlx::query_as("SELECT site_id FROM ent_salle WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(l).fetch_optional(&mut *conn).await?;
        site = r.and_then(|(s,)| s);
    }
    if site.is_none() && public == PublicNature::Personnel {
        if let Some(p) = personne {
            // Le demandeur n'a un site que s'il n'en a qu'un (missions de périmètre site en cours).
            let sites: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT DISTINCT p.cible_id FROM ent_mission m
                   JOIN ent_agent a ON a.etablissement_id = m.etablissement_id AND a.id = m.agent_id
                   JOIN ent_mission_perimetre p ON p.etablissement_id = m.etablissement_id AND p.mission_id = m.id
                  WHERE m.etablissement_id = $1 AND a.personne_id = $2 AND m.annule_le IS NULL
                    AND m.du <= $3 AND (m.au IS NULL OR m.au >= $3) AND p.nature = 'site'",
            )
            .bind(etab)
            .bind(p)
            .bind(jour)
            .fetch_all(&mut *conn)
            .await?;
            if sites.len() == 1 {
                site = Some(sites[0].0);
            }
        }
    }
    // CDM-05 : le routage suit l'élève sujet (son niveau), pas le demandeur.
    let niveau_code = match depot.eleve_id {
        Some(e) => {
            let r: Option<(String,)> = sqlx::query_as(
                "SELECT n.code FROM ent_eleve_inscription i
                   JOIN ent_niveau n ON n.id = i.niveau_id
                   JOIN ent_annee_scolaire a ON a.id = i.annee_id
                  WHERE i.etablissement_id = $1 AND i.eleve_id = $2 AND i.etat <> 'annulee'
                  ORDER BY a.courante DESC, i.date_debut DESC NULLS LAST LIMIT 1",
            )
            .bind(etab)
            .bind(e)
            .fetch_optional(&mut *conn)
            .await?;
            r.map(|(c,)| c)
        }
        None => None,
    };
    Ok(CriteresRoutage {
        nature_code: decl_code.to_string(),
        categorie,
        site_id: site,
        niveau_code,
        public,
        urgence: depot.urgence.unwrap_or(Urgence::Normale),
    })
}

// ───────────────────────── Adaptateur réel des sources ─────────────────────────

/// Instantané des sources du cœur pour **une** requête : files de l'acteur (membres calculés au jour métier),
/// routage du dépôt, activation des natures, droits de lien des familles. Remplace `SourcesAbsentes` : sans file,
/// rien ne passe (fail-closed), mais les files existent maintenant.
#[derive(Debug, Clone, Default)]
pub struct SourcesFiles {
    etab: Uuid,
    personne: Option<Uuid>,
    files_membre: Vec<Uuid>,
    routages: BTreeMap<String, RoutageResolu>,
    refus: BTreeMap<String, &'static str>,
    famille: Option<(Uuid, Option<Uuid>, bool)>,
}

impl SourcesDemandes for SourcesFiles {
    fn file_de_depot(&self, etab: Uuid, nature_code: &str) -> Option<Uuid> {
        self.routage_de_depot(etab, nature_code).map(|r| r.file_id)
    }
    fn files_de(&self, etab: Uuid, personne_id: Uuid) -> Vec<Uuid> {
        if etab == self.etab && self.personne == Some(personne_id) {
            self.files_membre.clone()
        } else {
            Vec::new()
        }
    }
    fn peut_deposer_famille(&self, etab: Uuid, user_id: Uuid, _role: Role, eleve_id: Option<Uuid>) -> bool {
        etab == self.etab && self.famille.is_some_and(|(u, e, ok)| ok && u == user_id && e == eleve_id)
    }
    fn routage_de_depot(&self, etab: Uuid, nature_code: &str) -> Option<RoutageResolu> {
        if etab != self.etab {
            return None;
        }
        self.routages.get(nature_code).cloned()
    }
    fn refus_nature(&self, etab: Uuid, nature_code: &str) -> Option<&'static str> {
        if etab != self.etab {
            return Some(NATURE_NON_PROPOSEE);
        }
        self.refus.get(nature_code).copied()
    }
}

/// Public d'une session famille ou élève pour cette nature : le responsable légal, l'élève majeur ou mineur.
async fn public_famille(ent: &PgEnt, etab: Uuid, user: Uuid, role: Role, eleve: Option<Uuid>, jour: NaiveDate) -> Result<(PublicNature, bool), RefusDemande> {
    match role {
        Role::Parent => {
            let ok = match eleve {
                Some(e) => ent.lien_parent_eleve(etab, user, e).await?.is_some(),
                None => !ent.eleves_lies(etab, user).await?.is_empty(),
            };
            Ok((PublicNature::Responsable, ok))
        }
        Role::Eleve => {
            let mut tx = TenantTx::begin(ent.pool(), etab).await?;
            let moi: Option<(Uuid,)> =
                sqlx::query_as("SELECT eleve_id FROM ent_eleve_compte WHERE etablissement_id = $1 AND user_id = $2").bind(etab).bind(user).fetch_optional(&mut *tx).await?;
            tx.commit().await?;
            let Some((mon_eleve,)) = moi else { return Ok((PublicNature::EleveMineur, false)) };
            let majeur = ent.eleve_est_majeur(etab, mon_eleve, jour).await?;
            let public = if majeur { PublicNature::EleveMajeur } else { PublicNature::EleveMineur };
            Ok((public, eleve.is_none_or(|e| e == mon_eleve)))
        }
        _ => Ok((PublicNature::Personnel, true)),
    }
}

impl PgEnt {
    /// Instantané des sources pour l'acteur de la session. Avec `depot`, résout aussi le routage de cette
    /// demande (nature activée ? public ouvert ? quelle file ? quels délais ?).
    pub async fn sources_demandes_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        depot: Option<&DepotDemande>,
    ) -> Result<SourcesFiles, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let g = charger_graphe(&mut tx, etab, jour).await?;
        let personne: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL ORDER BY created_at LIMIT 1",
        )
        .bind(etab)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
        let personne = personne.map(|(p,)| p);
        let mut s = SourcesFiles { etab, personne, ..Default::default() };
        if role.is_staff() {
            s.files_membre = personne.map(|p| g.files_de(p)).unwrap_or_default();
        }
        let Some(depot) = depot else {
            tx.commit().await?;
            return Ok(s);
        };
        let Some(decl) = nature(&depot.nature_code) else {
            tx.commit().await?;
            return Ok(s);
        };
        // Public et droits de lien (socle 4) : sessions famille et élève.
        let (public, famille_ok) = if role.is_staff() {
            (PublicNature::Personnel, true)
        } else {
            public_famille(self, etab, user_id, role, depot.eleve_id, jour).await?
        };
        s.famille = Some((user_id, depot.eleve_id, famille_ok));
        // Nature activée par l'établissement et ouverte à ce public (CDM-07, fail-closed : rien d'activé, rien d'ouvert).
        let natures = charger_natures_etab(&mut tx, etab).await?;
        let ne = natures.get(decl.code);
        let ouverte = ne.is_some_and(|n| n.actif && n.publics.iter().any(|p| p == code_public(public)));
        if !ouverte {
            s.refus.insert(decl.code.to_string(), NATURE_NON_PROPOSEE);
            tx.commit().await?;
            return Ok(s);
        }
        let criteres = criteres_depot(&mut tx, etab, jour, personne, public, decl.code, depot).await?;
        let regles = charger_regles(&mut tx, etab, decl.code, jour).await?;
        let repli = repli_rh_id(&mut tx, etab).await?;
        tx.commit().await?;
        if let Some(r) = resoudre(&g, repli, &regles, ne, &criteres, &[]) {
            s.routages.insert(
                decl.code.to_string(),
                RoutageResolu {
                    file_id: r.file.id,
                    motif: Some(r.motif),
                    ref_id: r.ref_id,
                    service_calendrier: r.file.service_calendrier.clone(),
                    delai_prise_jours: Some(r.delai_prise_jours),
                    delai_traitement_jours: Some(r.delai_traitement_jours),
                    site_id: criteres.site_id,
                },
            );
        }
        Ok(s)
    }

    // ───────────── Natures activées ─────────────

    /// Active (ou désactive) une nature, ouvre ses publics, raccourcit ses délais, désigne sa file. Contrôles :
    /// nature déclarée ; publics parmi ceux de la déclaration ; délais jamais plus longs que la déclaration ;
    /// une nature C2+ ne se route jamais vers une file non confidentielle (§10.5).
    pub async fn regler_nature_for_actor(&self, etab: Uuid, user: Uuid, role: &str, code: &str, r: ReglageNature) -> Result<(), RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        let decl = nature(code).ok_or(RefusDemande::Invalide("Cette démarche n'existe pas."))?;
        if r.actif && r.publics.is_empty() {
            return Err(RefusDemande::Invalide("Choisissez au moins un public pour activer la démarche."));
        }
        if r.publics.iter().any(|p| !decl.publics.contains(p)) {
            return Err(RefusDemande::Invalide("Cette démarche n'est pas ouverte à ce public."));
        }
        let (p, t) = (r.delai_prise_jours, r.delai_traitement_jours);
        if p.is_some_and(|x| x < 1 || x as u16 > decl.delai_prise_jours)
            || t.is_some_and(|x| x < 1 || x as u16 > decl.delai_traitement_jours)
            || matches!((p, t), (Some(a), Some(b)) if b < a)
        {
            return Err(RefusDemande::Invalide("Un délai peut être raccourci, jamais rallongé."));
        }
        if r.texte_aide.as_deref().is_some_and(|a| a.chars().count() > 500) {
            return Err(RefusDemande::Invalide("Le texte d'aide est trop long."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let mut file_id = None;
        if let Some(fc) = r.file_defaut_code.as_deref() {
            let f: Option<(Uuid, String)> = sqlx::query_as("SELECT id, type FROM ent_dem_file WHERE etablissement_id = $1 AND code = $2 AND actif").bind(etab).bind(fc).fetch_optional(&mut *tx).await?;
            let Some((id, type_)) = f else { return Err(RefusDemande::Invalide("Cette file n'existe pas.")) };
            if matches!(decl.confidentialite, crate::demandes::Confidentialite::C2 | crate::demandes::Confidentialite::C3)
                && !matches!(type_.as_str(), "confidentielle" | "direction")
            {
                return Err(RefusDemande::Invalide("Une démarche confidentielle ne peut pas aller vers une file ordinaire."));
            }
            file_id = Some(id);
        }
        let avant: Option<(bool,)> = sqlx::query_as("SELECT actif FROM ent_dem_nature_etab WHERE etablissement_id = $1 AND nature_code = $2").bind(etab).bind(code).fetch_optional(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO ent_dem_nature_etab (id, etablissement_id, nature_code, actif, delai_prise_jours, delai_traitement_jours,
                    file_defaut_id, texte_aide, maj_par_user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (etablissement_id, nature_code) DO UPDATE SET
                 actif = EXCLUDED.actif, delai_prise_jours = EXCLUDED.delai_prise_jours,
                 delai_traitement_jours = EXCLUDED.delai_traitement_jours, file_defaut_id = EXCLUDED.file_defaut_id,
                 texte_aide = EXCLUDED.texte_aide, maj_le = now(), maj_par_user_id = EXCLUDED.maj_par_user_id",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(code)
        .bind(r.actif)
        .bind(p)
        .bind(t)
        .bind(file_id)
        .bind(r.texte_aide.as_deref().map(str::trim))
        .bind(user)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM ent_dem_nature_public WHERE etablissement_id = $1 AND nature_code = $2").bind(etab).bind(code).execute(&mut *tx).await?;
        for pub_ in &r.publics {
            sqlx::query("INSERT INTO ent_dem_nature_public (etablissement_id, nature_code, public_code) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
                .bind(etab)
                .bind(code)
                .bind(code_public(*pub_))
                .execute(&mut *tx)
                .await?;
        }
        let av = avant.map(|(a,)| if a { "active" } else { "inactive" });
        journaliser(&mut tx, etab, "nature", code, av, Some(if r.actif { "active" } else { "inactive" }), user).await?;
        tx.commit().await?;
        Ok(())
    }

    // ───────────── Règles de routage ─────────────

    pub async fn ajouter_regle_routage_for_actor(&self, etab: Uuid, user: Uuid, role: &str, n: NouvelleRegleRoutage) -> Result<Uuid, RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        let decl = nature(&n.nature_code).ok_or(RefusDemande::Invalide("Cette démarche n'existe pas."))?;
        if !(1..=999).contains(&n.ordre) {
            return Err(RefusDemande::Invalide("L'ordre est compris entre 1 et 999."));
        }
        if let Some(c) = &n.categorie_code {
            // La catégorie est une valeur du champ liste que la nature déclare comme critère.
            let valeurs = cat::meta(decl.code)
                .and_then(|m| m.categorie_champ)
                .and_then(|cc| decl.champs.iter().find(|c| c.code == cc))
                .and_then(|c| match c.type_ {
                    crate::demandes::ChampType::Liste(l) => Some(l),
                    _ => None,
                });
            if !valeurs.is_some_and(|l| l.contains(&c.as_str())) {
                return Err(RefusDemande::Invalide("Cette catégorie n'existe pas pour cette démarche."));
            }
        }
        if n.niveau_code.as_deref().is_some_and(|c| !code_ok(&c.to_ascii_lowercase(), 40) && c.trim().is_empty()) {
            return Err(RefusDemande::Invalide("Niveau invalide."));
        }
        if let Some(p) = n.public {
            if !decl.publics.contains(&p) {
                return Err(RefusDemande::Invalide("Cette démarche n'est pas ouverte à ce public."));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let du = n.du.unwrap_or(jour);
        if n.au.is_some_and(|a| a < du) {
            return Err(RefusDemande::Invalide("La date de fin précède la date de début."));
        }
        let f: Option<(String, String)> =
            sqlx::query_as("SELECT type, code FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2 AND actif").bind(etab).bind(n.destination_file_id).fetch_optional(&mut *tx).await?;
        let Some((type_, code_file)) = f else { return Err(RefusDemande::Invalide("La file de destination est introuvable.")) };
        // §10.5 : une nature C3 (et C2) ne se route jamais vers une file non confidentielle.
        if matches!(decl.confidentialite, crate::demandes::Confidentialite::C2 | crate::demandes::Confidentialite::C3)
            && !matches!(type_.as_str(), "confidentielle" | "direction")
        {
            return Err(RefusDemande::Invalide("Une démarche confidentielle ne peut pas aller vers une file ordinaire."));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_dem_routage_regle (id, etablissement_id, nature_code, categorie_code, site_id, niveau_code, public_code,
                    urgence, destination_file_id, ordre, du, au, cree_par_user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(id)
        .bind(etab)
        .bind(&n.nature_code)
        .bind(n.categorie_code.as_deref())
        .bind(n.site_id)
        .bind(n.niveau_code.as_deref())
        .bind(n.public.map(code_public))
        .bind(n.urgence.map(|u| u.code()))
        .bind(n.destination_file_id)
        .bind(n.ordre)
        .bind(du)
        .bind(n.au)
        .bind(user)
        .execute(&mut *tx)
        .await
        .map_err(|e| match e.as_database_error().and_then(|d| d.code()).as_deref() {
            Some("23503") => RefusDemande::Invalide("Le site ou la file indiqués n'existent pas."),
            _ => RefusDemande::from(e),
        })?;
        journaliser(&mut tx, etab, "routage", &n.nature_code, None, Some(&code_file), user).await?;
        tx.commit().await?;
        Ok(id)
    }

    pub async fn desactiver_regle_routage_for_actor(&self, etab: Uuid, user: Uuid, role: &str, regle_id: Uuid) -> Result<(), RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r: Option<(String,)> = sqlx::query_as("UPDATE ent_dem_routage_regle SET actif = false WHERE etablissement_id = $1 AND id = $2 AND actif RETURNING nature_code")
            .bind(etab)
            .bind(regle_id)
            .fetch_optional(&mut *tx)
            .await?;
        let Some((code,)) = r else { return Err(RefusDemande::Introuvable) };
        journaliser(&mut tx, etab, "routage", &code, Some("active"), Some("inactive"), user).await?;
        tx.commit().await?;
        Ok(())
    }

    // ───────────── Simulateur (DEM-334) ─────────────

    /// « Une attestation pour un élève de 2nde du site Lycée irait à… » : le routage **sans rien créer**. Peut
    /// simuler une personne visée (R-DM47), pour vérifier le repli avant qu'il serve.
    pub async fn simuler_routage_for_actor(&self, etab: Uuid, user: Uuid, role: &str, s: SimulationRoutage) -> Result<ResultatSimulation, RefusDemande> {
        self.exiger_parametrer_demandes(etab, user, role).await?;
        let decl = nature(&s.nature_code).ok_or(RefusDemande::Invalide("Cette démarche n'existe pas."))?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let g = charger_graphe(&mut tx, etab, jour).await?;
        let natures = charger_natures_etab(&mut tx, etab).await?;
        let regles = charger_regles(&mut tx, etab, decl.code, jour).await?;
        let repli = repli_rh_id(&mut tx, etab).await?;
        tx.commit().await?;
        let c = CriteresRoutage {
            nature_code: decl.code.to_string(),
            categorie: s.categorie,
            site_id: s.site_id,
            niveau_code: s.niveau_code,
            public: s.public.unwrap_or(PublicNature::Personnel),
            urgence: s.urgence.unwrap_or(Urgence::Normale),
        };
        let Some(r) = resoudre(&g, repli, &regles, natures.get(decl.code), &c, &s.vises) else {
            return Err(RefusDemande::Indisponible);
        };
        Ok(ResultatSimulation {
            file_code: r.file.code.clone(),
            file_libelle: r.file.libelle.clone(),
            role_traitant: r.file.role_lisible().to_string(),
            motif: r.motif,
            raison: r.raison,
            nb_destinataires: g.membres_hors(r.file.id, &s.vises).len(),
            repli_utilise: r.initiale.is_some(),
            file_initiale: r.initiale.map(|f| f.code),
            delai_prise_jours: r.delai_prise_jours,
            delai_traitement_jours: r.delai_traitement_jours,
        })
    }

    // ───────────── Catalogue (guichet) ─────────────

    /// « Faire une demande » : seulement ce que la personne a le droit de faire (R-DM56) — natures **activées**
    /// et ouvertes à son public, raccourcis de son public. Les situations graves ne sont jamais une demande N1
    /// (R-DM31) : l'entrée dédiée mène au canal protégé et aux numéros d'urgence.
    pub async fn catalogue_for_actor(&self, etab: Uuid, user: Uuid, role: &str) -> Result<Catalogue, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_metier(&mut tx, etab).await?;
        let g = charger_graphe(&mut tx, etab, jour).await?;
        let natures = charger_natures_etab(&mut tx, etab).await?;
        tx.commit().await?;
        let public = if role.is_staff() { PublicNature::Personnel } else { public_famille(self, etab, user, role, None, jour).await?.0 };
        let mut entrees = Vec::new();
        for decl in cat::toutes_les_natures() {
            let Some(ne) = natures.get(decl.code).filter(|n| n.actif && n.publics.iter().any(|p| p == code_public(public))) else {
                continue;
            };
            let file = ne.file_defaut.and_then(|id| g.files.get(&id)).or_else(|| g.file_par_code(decl.file_defaut));
            let (prise, traitement) = file.map_or((decl.delai_prise_jours, decl.delai_traitement_jours), |f| {
                delais_effectifs(decl.delai_prise_jours, decl.delai_traitement_jours, Some((ne.delai_prise, ne.delai_traitement)), f)
            });
            entrees.push(EntreeCatalogue {
                code: decl.code,
                libelle: decl.libelle,
                mots_cles: decl.mots_cles,
                eleve_requis: decl.eleve_requis,
                role_traitant: file.map(|f| f.role_lisible().to_string()),
                message_accueil: file.and_then(|f| f.message_accueil.clone()),
                delai_prise_jours: prise,
                delai_traitement_jours: traitement,
            });
        }
        let raccourcis = cat::RACCOURCIS.iter().filter(|r| r.publics.contains(&public)).collect();
        Ok(Catalogue { natures: entrees, raccourcis, signalement_grave: cat::SIGNALEMENT_GRAVE, numeros_urgence: cat::NUMEROS_URGENCE })
    }

    // ───────────── Accusé de réception ─────────────

    /// L'accusé de réception vu du **demandeur ou du déposant** (404 neutre pour tout autre) : le service qui
    /// traite, son rôle, un délai indicatif daté, la mention de fermeture (R-DM22) et, pour une nature à décision
    /// de l'usager, les mentions EPLE. Aucun nom de personne avant la prise en charge (R-DM53).
    pub async fn accuse_reception_for_actor(&self, etab: Uuid, user: Uuid, demande_id: Uuid) -> Result<AccuseReception, RefusDemande> {
        let d = self.demande_demandeur_for_actor(etab, user, demande_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (file_id, depose): (Uuid, Option<NaiveDate>) = sqlx::query_as(
            "SELECT d.file_id, (d.depose_le AT TIME ZONE COALESCE((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1), 'Europe/Paris'))::date
               FROM ent_dem_demande d WHERE d.etablissement_id = $1 AND d.id = $2",
        )
        .bind(etab)
        .bind(demande_id)
        .fetch_one(&mut *tx)
        .await?;
        let f: Option<(String, Option<String>, String, String, Option<String>)> = sqlx::query_as(
            "SELECT libelle, role_traitant, service_calendrier, type, message_accueil FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(file_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (service, role_traitant, calendrier_service, message_accueil) = match f {
            Some((libelle, role, cal, _type, msg)) => {
                let r = role.unwrap_or_else(|| libelle.clone());
                (libelle, r, cal, msg)
            }
            // Une demande du cœur sans file connue (sources de test) : accusé minimal, sans promesse de rôle.
            None => ("l'établissement".to_string(), "l'établissement".to_string(), "secretariat".to_string(), None),
        };
        let (delai,): (Option<NaiveDate>,) = sqlx::query_as(
            "SELECT (e.due_le AT TIME ZONE COALESCE((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1), 'Europe/Paris'))::date
               FROM ent_dem_echeance e WHERE e.etablissement_id = $1 AND e.demande_id = $2 AND e.type = 'traitement'
              ORDER BY e.cree_le LIMIT 1",
        )
        .bind(etab)
        .bind(demande_id)
        .fetch_one(&mut *tx)
        .await?;
        // CDM-06 : déposée pendant une fermeture du service, la demande est accusée tout de suite, avec la mention
        // de la fermeture ; le délai part à la réouverture.
        let mut mention_fermeture = None;
        if let Some(j) = depose {
            if let Ok(js) = calendrier::jour_service(&mut tx, etab, j, &calendrier_service).await {
                if !js.ouvert {
                    mention_fermeture = calendrier::echeance_jours_ouvres(&mut tx, etab, j, 1, &calendrier_service).await.ok();
                }
            }
        }
        tx.commit().await?;
        let decl = nature(&d.nature_code);
        let mentions = if cat::meta(&d.nature_code).is_some_and(|m| m.decision_usager) { MENTIONS_ACCUSE_EPLE.to_vec() } else { Vec::new() };
        Ok(AccuseReception {
            numero: d.numero,
            nature_libelle: decl.map_or("Demande", |n| n.libelle),
            service,
            role_traitant,
            delai_indicatif: delai,
            mention_fermeture,
            message_accueil,
            mentions,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct SimulationRoutage {
    pub nature_code: String,
    pub categorie: Option<String>,
    pub site_id: Option<Uuid>,
    pub niveau_code: Option<String>,
    pub public: Option<PublicNature>,
    pub urgence: Option<Urgence>,
    /// Personnes visées : jamais destinataires (R-DM47).
    pub vises: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultatSimulation {
    pub file_code: String,
    pub file_libelle: String,
    pub role_traitant: String,
    pub motif: &'static str,
    pub raison: String,
    pub nb_destinataires: usize,
    pub repli_utilise: bool,
    pub file_initiale: Option<String>,
    pub delai_prise_jours: u16,
    pub delai_traitement_jours: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntreeCatalogue {
    pub code: &'static str,
    pub libelle: &'static str,
    pub mots_cles: &'static [&'static str],
    pub eleve_requis: bool,
    pub role_traitant: Option<String>,
    pub message_accueil: Option<String>,
    pub delai_prise_jours: u16,
    pub delai_traitement_jours: u16,
}

#[derive(Debug, Clone)]
pub struct Catalogue {
    pub natures: Vec<EntreeCatalogue>,
    pub raccourcis: Vec<&'static cat::RaccourciDecl>,
    pub signalement_grave: &'static str,
    pub numeros_urgence: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccuseReception {
    pub numero: String,
    pub nature_libelle: &'static str,
    /// Le service qui traite, jamais une personne (R-DM53) : le nom vient avec la prise en charge.
    pub service: String,
    /// « le secrétariat des élèves ».
    pub role_traitant: String,
    /// Délai indicatif daté (jour métier de l'établissement).
    pub delai_indicatif: Option<NaiveDate>,
    /// Réouverture du service si la demande a été déposée pendant une fermeture.
    pub mention_fermeture: Option<NaiveDate>,
    pub message_accueil: Option<String>,
    pub mentions: Vec<&'static str>,
}
