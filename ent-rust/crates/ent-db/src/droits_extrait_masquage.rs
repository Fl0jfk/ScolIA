//! Socle 27, **lot G3b-2** : marquage **déterministe** des éléments d'un extrait qui ne doivent pas partir tels
//! quels, et relecture humaine (RGP-150, RGP-151).
//!
//! Le marquage n'est **jamais** le fait d'une IA (R-RG15) : il découle de règles écrites ici, appliquées au contenu
//! assemblé. Il porte sur des **éléments référencés** — une table, une ligne (ou son rang quand elle n'a pas
//! d'identifiant), une colonne — jamais sur une copie des données : le contenu d'un extrait n'est pas stocké.
//!
//! Raisons (`RAISONS`) :
//!
//! | raison | ce qu'elle protège | qui peut lever |
//! |---|---|---|
//! | `autre_parent` | données propres de l'autre parent (coordonnées, échanges, situation) | `droits.arbitrer` |
//! | `adresse_protegee` | adresse mise de côté et tout indice de localisation | `droits.arbitrer` |
//! | `parent_donnees_propres` | pour un élève demandeur, ce qui n'est pas l'identité de ses responsables (R-RG48) | `droits.arbitrer` |
//! | `autre_eleve` | autre élève dans un fait collectif | `droits.traiter` |
//! | `redacteur` | rédacteur d'une note interne | `droits.traiter` |
//! | `signalement` | signalement en cours (socle 36) | `droits.arbitrer` |
//! | `piece_tiers` | pièce du coffre qui concerne aussi un tiers (jamais remise ni caviardée, R-RG45) | `droits.arbitrer` |
//! | `manuel` | ajout d'un agent | `droits.traiter` |
//!
//! Aucune table de protection n'est **rendue** ; leur lecture se limite à savoir **si** une protection existe (un
//! identifiant de personne, jamais l'adresse), pour poser des marquages. Le code `adresse_protegee` n'est jamais
//! montré à un porteur de `droits.traiter` sans `droits.arbitrer` (R-RG46) : la vue dit « à arbitrer ».

use crate::droits_demandes::{db, etat_courant, exiger_ouverte, invalide, journal, texte};
use crate::droits_extrait::{ContenuExtrait, SectionContenu};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Utc};
use domain_eleve::DomainError;
use ent_auth::Action;
use serde::Serialize;
use sqlx::{PgConnection, Row};
use std::collections::BTreeSet;
use uuid::Uuid;

/// Raisons de marquage (vocabulaire fermé, contrôlé par la base).
pub const RAISONS: &[&str] = &[
    "autre_parent",
    "adresse_protegee",
    "parent_donnees_propres",
    "autre_eleve",
    "redacteur",
    "signalement",
    "piece_tiers",
    "manuel",
];

/// Raisons dont la **levée** exige `droits.arbitrer`.
pub fn levee_reservee_direction(raison: &str) -> bool {
    matches!(
        raison,
        "autre_parent"
            | "adresse_protegee"
            | "parent_donnees_propres"
            | "signalement"
            | "piece_tiers"
    )
}

/// Texte substitué à la valeur masquée dans ce qui est remis.
pub fn libelle_masque(raison: &str) -> &'static str {
    match raison {
        "autre_parent" | "parent_donnees_propres" => "[masqué : autre personne]",
        "adresse_protegee" => "[masqué : information protégée]",
        "autre_eleve" => "[masqué : autre élève]",
        "redacteur" => "[masqué : rédacteur]",
        "signalement" => "[masqué : procédure en cours]",
        "piece_tiers" => "[pièce non remise : concerne aussi une autre personne]",
        _ => "[masqué]",
    }
}

/// Colonnes qui désignent une **personne ou un compte** dans une ligne.
const COLONNES_PERSONNE: &[&str] = &[
    "personne_id",
    "responsable_personne_id",
    "parent_personne_id",
    "auteur_personne_id",
    "user_id",
    "auteur_user_id",
    "responsable_user_id",
    "destinataire_user_id",
    "contact_personne_id",
];

/// Colonnes qui désignent le **rédacteur** d'un texte interne.
const COLONNES_REDACTEUR: &[&str] = &[
    "auteur_user_id",
    "auteur_personne_id",
    "cree_par",
    "cree_par_user_id",
    "redige_par",
    "redacteur_user_id",
    "saisi_par",
];

/// Ce qui reste visible d'un responsable pour un élève demandeur (R-RG48) : son identité de responsable.
const IDENTITE_RESPONSABLE: &[&str] = &[
    "id",
    "nom",
    "prenom",
    "civilite",
    "lien",
    "qualite",
    "autorite_parentale",
    "dossier_id",
    "eleve_id",
];

fn colonne_localisation(nom: &str) -> bool {
    [
        "adresse",
        "ville",
        "code_postal",
        "commune",
        "latitude",
        "longitude",
        "transport",
        "arret",
    ]
    .iter()
    .any(|f| nom.contains(f))
}

/// Faits, tirés de la base, sur lesquels s'appuie le marquage (aucun contenu n'y figure).
#[derive(Clone, Debug, Default)]
pub struct Faits {
    pub sujet: Option<Uuid>,
    pub dem_compte: Option<Uuid>,
    /// Personne du demandeur (fiche rapprochée ou issue de son compte).
    pub dem_personne: Option<Uuid>,
    pub dem_est_sujet: bool,
    /// Comptes et personnes **liés à l'élève** autres que le demandeur (l'autre parent, les tiers).
    pub autres: BTreeSet<Uuid>,
    /// Parmi eux, ceux qui bénéficient d'une protection au dossier (identifiants seulement).
    pub proteges: BTreeSet<Uuid>,
    /// Personnes de la famille du demandeur et de l'élève dont une pièce est « pour lui ».
    pub siens: BTreeSet<Uuid>,
    /// Sujet « personne » ou « agent » (M6) : les élèves dont la personne est responsable (lien actif ou non).
    /// Tout autre élève cité dans ses données est un « autre élève ».
    pub eleves_lies: BTreeSet<Uuid>,
    /// F7 : noms (`prénom nom`, puis `nom`) des autres élèves cités dans le contenu, tirés de la base. Une cellule de
    /// texte qui les contient est marquée `autre_eleve`, comme l'identifiant.
    pub noms_autres_eleves: Vec<String>,
}

/// Un élément marqué (calculé, pas encore décidé).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MarquageCalcule {
    pub fournisseur: String,
    pub table: String,
    pub ligne_id: Option<Uuid>,
    /// Rang de la ligne dans sa table (0, 1…) : identifie un élément sans identifiant.
    pub rang: i32,
    /// `None` : la ligne entière.
    pub colonne: Option<String>,
    pub raison: &'static str,
    /// Forme canonique de l'élément marqué (la cellule, ou la ligne entière), **en mémoire seulement** : sert à
    /// calculer l'empreinte à clé qui conditionne le report d'une décision (M2). Jamais stockée.
    pub element: String,
}

/// Forme canonique d'une ligne (colonnes triées à l'assemblage) ou d'une cellule.
fn element_de(l: &crate::droits_extrait::LigneContenu, colonne: Option<&str>) -> String {
    match colonne {
        Some(c) => l
            .valeurs
            .iter()
            .find(|(n, _)| n == c)
            .map(|(_, v)| v.clone())
            .unwrap_or_default(),
        None => l
            .valeurs
            .iter()
            .map(|(n, v)| format!("{n}={v}"))
            .collect::<Vec<_>>()
            .join("\u{1f}"),
    }
}

/// F7 : la cellule cite-t-elle un autre élève par son nom ? Comparaison sans casse, sur un mot entier.
fn cite_un_nom(valeur: &str, noms: &[String]) -> bool {
    if noms.is_empty() || valeur.len() < 3 {
        return false;
    }
    let v = valeur.to_lowercase();
    noms.iter().any(|n| {
        let n = n.to_lowercase();
        let mut depart = 0;
        while let Some(i) = v[depart..].find(&n) {
            let a = depart + i;
            let b = a + n.len();
            let avant = v[..a]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric());
            let apres = v[b..].chars().next().is_none_or(|c| !c.is_alphanumeric());
            if avant && apres {
                return true;
            }
            depart = b;
        }
        false
    })
}

/// Types de pièce qui ne concernent que leur porteur (personne, agent) : toute autre pièce, au nom d'une personne,
/// concerne aussi un tiers (pièce du foyer : livret de famille, justificatif de domicile ; décision de justice ; type
/// inconnu) et n'est jamais remise sans arbitrage (R-RG45, M6).
fn piece_propre_au_porteur(type_code: &str) -> bool {
    use crate::documents_pieces::{type_piece, PortePiece};
    matches!(
        type_piece(type_code).map(|t| &t.portee),
        Some(PortePiece::Personne | PortePiece::Agent)
    )
}

fn uuid_de(v: &str) -> Option<Uuid> {
    Uuid::parse_str(v.trim()).ok()
}

/// Marque le contenu, **de façon déterministe** : mêmes données, mêmes marquages, même ordre.
pub fn marquer(contenu: &ContenuExtrait, f: &Faits) -> Vec<MarquageCalcule> {
    let mut sortie: BTreeSet<MarquageCalcule> = BTreeSet::new();
    let eleve = contenu.sujet_type == "eleve";
    for s in &contenu.sections {
        marquer_section(s, f, eleve, contenu.sujet_id, &mut sortie);
    }
    sortie.into_iter().collect()
}

fn marquer_section(
    s: &SectionContenu,
    f: &Faits,
    eleve: bool,
    sujet: Uuid,
    sortie: &mut BTreeSet<MarquageCalcule>,
) {
    for t in &s.tables {
        let messagerie = t.nom.starts_with("ent_msg");
        let note = t.nom.contains("note") || t.nom.contains("observation");
        let signalement = t.nom.contains("signalement");
        for (rang, l) in t.lignes.iter().enumerate() {
            let rang = rang as i32;
            let mut marque = |colonne: Option<&str>, raison: &'static str| {
                sortie.insert(MarquageCalcule {
                    fournisseur: s.fournisseur.clone(),
                    table: t.nom.clone(),
                    ligne_id: l.id,
                    rang,
                    colonne: colonne.map(str::to_string),
                    raison,
                    element: element_de(l, colonne),
                });
            };
            if signalement {
                marque(None, "signalement");
                continue;
            }
            if t.nom == "ent_doc_piece" && !eleve {
                // M6 : sujet « personne » ou « agent » ; la pièce est à son nom (règle de lecture), seul son type dit
                // si elle concerne aussi un tiers.
                let type_code = l
                    .valeurs
                    .iter()
                    .find(|(c, _)| c == "type_code")
                    .map(|(_, v)| v.as_str())
                    .unwrap_or("");
                if !piece_propre_au_porteur(type_code) {
                    marque(None, "piece_tiers");
                    continue;
                }
            } else if t.nom == "ent_doc_piece" {
                let deposant = l
                    .valeurs
                    .iter()
                    .find(|(c, _)| c == "deposant_user_id")
                    .and_then(|(_, v)| uuid_de(v));
                let personne = l
                    .valeurs
                    .iter()
                    .find(|(c, _)| c == "personne_id")
                    .and_then(|(_, v)| uuid_de(v));
                let a_lui = (deposant.is_some() && deposant == f.dem_compte)
                    || personne.is_some_and(|p| f.siens.contains(&p));
                if !a_lui {
                    marque(None, "piece_tiers");
                    continue;
                }
            }
            let mut ligne_autre_parent = false;
            // Un responsable saisi au dossier d'inscription : sa ligne est marquée dès qu'elle n'est pas celle du
            // demandeur, **y compris sans fiche rapprochée** (E1).
            if eleve && t.nom == "ent_inscription_responsable" {
                let fiche = l
                    .valeurs
                    .iter()
                    .find(|(c, _)| c == "personne_id")
                    .and_then(|(_, v)| uuid_de(v));
                if fiche.is_none() || fiche != f.dem_personne {
                    ligne_autre_parent = true;
                }
            }
            for (c, v) in &l.valeurs {
                let Some(u) = uuid_de(v) else {
                    // F7 : le nom d'un autre élève dans un texte (fait collectif, observation).
                    if cite_un_nom(v, &f.noms_autres_eleves) {
                        marque(Some(c), "autre_eleve");
                    }
                    continue;
                };
                let autre_eleve = if eleve {
                    u != sujet
                } else {
                    !f.eleves_lies.contains(&u)
                };
                if c.ends_with("eleve_id") && autre_eleve {
                    marque(Some(c), "autre_eleve");
                }
                if note && COLONNES_REDACTEUR.contains(&c.as_str()) {
                    marque(Some(c), "redacteur");
                }
                if eleve && COLONNES_PERSONNE.contains(&c.as_str()) && f.autres.contains(&u) {
                    if f.proteges.contains(&u) {
                        marque(None, "adresse_protegee");
                        ligne_autre_parent = true;
                    } else if messagerie {
                        // Un message d'un autre parent dans un fil que le demandeur partage : l'auteur seul est masqué.
                        marque(Some(c), "autre_parent");
                    } else {
                        ligne_autre_parent = true;
                    }
                }
            }
            if ligne_autre_parent {
                if f.dem_est_sujet {
                    // R-RG48 : l'élève demandeur garde l'identité de ses responsables, rien de plus.
                    for (c, _) in &l.valeurs {
                        if !IDENTITE_RESPONSABLE.contains(&c.as_str()) {
                            marque(Some(c), "parent_donnees_propres");
                        }
                    }
                } else {
                    marque(None, "autre_parent");
                }
            }
            // Indice de localisation : tant qu'un autre parent est protégé, rien de ce qui situe la famille ne sort.
            if eleve && !f.proteges.is_empty() {
                for (c, _) in &l.valeurs {
                    if colonne_localisation(c) {
                        marque(Some(c), "adresse_protegee");
                    }
                }
            }
        }
    }
}

/// Remplace, dans une copie du contenu, les éléments **non levés** par leur libellé de masquage.
pub fn appliquer_masques(contenu: &ContenuExtrait, marques: &[MarquageVue]) -> ContenuExtrait {
    remplacer(contenu, marques, |m| {
        (m.decision != "leve").then(|| libelle_masque(&m.raison_interne).to_string())
    })
}

/// Ce qu'un lecteur sans `droits.arbitrer` ne doit ni voir ni situer (R-RG46, M4), calculé **depuis les seuls
/// marquages** : les marquages calculés portent toujours le rang d'origine de leur ligne.
#[derive(Debug, Default)]
pub(crate) struct Reserve {
    /// Lignes retirées entières : (fournisseur, table) → (identifiant, rang d'origine).
    lignes: std::collections::BTreeMap<(String, String), Vec<(Option<Uuid>, i32)>>,
    /// Cellules retirées : (fournisseur, table, identifiant, rang d'origine, colonne).
    cellules: Vec<(String, String, Option<Uuid>, i32, String)>,
}

impl Reserve {
    pub(crate) fn depuis(marques: &[MarquageVue]) -> Self {
        let mut r = Reserve::default();
        for m in marques
            .iter()
            .filter(|m| m.raison_interne == "adresse_protegee")
        {
            let Some(rang) = m.rang else { continue };
            match &m.colonne {
                None => r
                    .lignes
                    .entry((m.fournisseur.clone(), m.table.clone()))
                    .or_default()
                    .push((m.ligne_id, rang)),
                Some(c) => r.cellules.push((
                    m.fournisseur.clone(),
                    m.table.clone(),
                    m.ligne_id,
                    rang,
                    c.clone(),
                )),
            }
        }
        r
    }

    fn meme_ligne(a: (Option<Uuid>, i32), b: (Option<Uuid>, Option<i32>)) -> bool {
        match (a.0, b.0) {
            (Some(x), Some(y)) => x == y,
            (None, None) => b.1 == Some(a.1),
            _ => false,
        }
    }

    /// La ligne (fournisseur, table, identifiant, rang d'origine) est-elle retirée entière ?
    fn ligne_retiree(&self, f: &str, t: &str, lid: Option<Uuid>, rang: Option<i32>) -> bool {
        self.lignes
            .get(&(f.to_string(), t.to_string()))
            .is_some_and(|v| v.iter().any(|x| Self::meme_ligne(*x, (lid, rang))))
    }

    fn cellule_retiree(
        &self,
        f: &str,
        t: &str,
        lid: Option<Uuid>,
        rang: Option<i32>,
        c: &str,
    ) -> bool {
        self.cellules.iter().any(|(cf, ct, cl, cr, cc)| {
            cf == f && ct == t && cc == c && Self::meme_ligne((*cl, *cr), (lid, rang))
        })
    }

    /// Rangs d'origine retirés d'une table (lignes sans identifiant comme avec : toutes décalent l'affichage).
    fn rangs_retires(&self, f: &str, t: &str) -> BTreeSet<i32> {
        self.lignes
            .get(&(f.to_string(), t.to_string()))
            .map(|v| v.iter().map(|x| x.1).collect())
            .unwrap_or_default()
    }

    /// Rang **affiché** (lignes retirées ôtées) d'un rang d'origine.
    fn rang_affiche(&self, f: &str, t: &str, origine: i32) -> i32 {
        origine
            - self
                .rangs_retires(f, t)
                .iter()
                .filter(|r| **r < origine)
                .count() as i32
    }

    /// E2 : rang **d'origine** d'un rang affiché au lecteur (ce qu'il désigne à l'écran).
    pub(crate) fn rang_d_origine(&self, f: &str, t: &str, affiche: i32) -> i32 {
        let retires = self.rangs_retires(f, t);
        let (mut o, mut k) = (0i32, 0i32);
        loop {
            if !retires.contains(&o) {
                if k == affiche {
                    return o;
                }
                k += 1;
            }
            o += 1;
        }
    }

    /// Un marquage qui situerait une protection : la protection elle-même, ou tout marquage dont la cible (ligne
    /// entière ou cellule) est retirée. E1 : sur la ligne d'un responsable protégé, le marquage `autre_parent` qui la
    /// double désignerait une ligne absente du contenu, signal distinctif d'une protection.
    fn cache(&self, m: &MarquageVue) -> bool {
        if m.raison_interne == "adresse_protegee" {
            return true;
        }
        if m.ligne_id.is_none() && m.rang.is_none() {
            return false; // toute la table : ne situe rien
        }
        if self.ligne_retiree(&m.fournisseur, &m.table, m.ligne_id, m.rang) {
            return true;
        }
        m.colonne
            .as_deref()
            .is_some_and(|c| self.cellule_retiree(&m.fournisseur, &m.table, m.ligne_id, m.rang, c))
    }

    /// Marquages visibles du lecteur, rangs **affichés** ; et le nombre de ceux qu'on lui cache.
    pub(crate) fn filtrer(&self, marques: Vec<MarquageVue>) -> (Vec<MarquageVue>, usize) {
        let mut caches = 0;
        let mut v = Vec::with_capacity(marques.len());
        for mut m in marques {
            if self.cache(&m) {
                caches += 1;
                continue;
            }
            if let Some(r) = m.rang {
                m.rang = Some(self.rang_affiche(&m.fournisseur, &m.table, r));
            }
            v.push(m);
        }
        (v, caches)
    }
}

/// Pour un lecteur sans `droits.arbitrer` (R-RG46, M4) : les éléments protégés sont **absents** de ce qu'il voit
/// (la cellule, ou la ligne entière, est retirée du contenu) ; aucun marquage visible ne les situe (ni le marquage de
/// protection, ni un autre marquage sur la même cible, E1) ; les rangs montrés sont ceux du contenu filtré (E2). Seul
/// un compteur global `elements_reserves_direction` dit combien de marquages lui sont cachés, sans dire où. Les autres
/// raisons réservées à la direction gardent l'alias neutre « à arbitrer ».
pub fn cacher_protections(contenu: &ContenuExtrait, marques: &[MarquageVue]) -> ContenuExtrait {
    let r = Reserve::depuis(marques);
    let mut c = contenu.clone();
    for s in &mut c.sections {
        for t in &mut s.tables {
            let mut rang = 0i32;
            t.lignes.retain_mut(|l| {
                let o = rang;
                rang += 1;
                let lid = l.id;
                if r.ligne_retiree(&s.fournisseur, &t.nom, lid, Some(o)) {
                    return false;
                }
                l.valeurs.retain(|(col, _)| {
                    !r.cellule_retiree(&s.fournisseur, &t.nom, lid, Some(o), col)
                });
                true
            });
        }
        let n: usize = s.tables.iter().map(|t| t.lignes.len()).sum();
        s.lignes = n;
    }
    let (visibles, caches) = r.filtrer(marques.to_vec());
    c.elements_reserves_direction = caches;
    c.marquages = visibles;
    c
}

/// Le marquage vise-t-il la ligne `lid` de rang `rang` ? Identifiant d'abord ; sans identifiant de marquage, le rang
/// (`None` : toutes les lignes de la table, ajout manuel M5).
fn vise(m: &MarquageVue, lid: Option<Uuid>, rang: i32) -> bool {
    match (m.ligne_id, lid) {
        (Some(a), Some(b)) => a == b,
        (Some(_), None) => false,
        (None, _) => m.rang.is_none_or(|x| x == rang),
    }
}

fn remplacer(
    contenu: &ContenuExtrait,
    marques: &[MarquageVue],
    texte: impl Fn(&MarquageVue) -> Option<String>,
) -> ContenuExtrait {
    let mut c = contenu.clone();
    for s in &mut c.sections {
        for t in &mut s.tables {
            for (rang, l) in t.lignes.iter_mut().enumerate() {
                let lid = l.id;
                for m in marques.iter().filter(|m| {
                    m.fournisseur == s.fournisseur && m.table == t.nom && vise(m, lid, rang as i32)
                }) {
                    let Some(masque) = texte(m) else { continue };
                    for (col, val) in &mut l.valeurs {
                        if m.colonne.as_deref().is_none_or(|x| x == col) {
                            *val = masque.clone();
                        }
                    }
                }
            }
        }
    }
    c.marquages = marques.to_vec();
    c
}

/// Contenu inchangé, accompagné des marquages (relecture par la direction).
pub fn remplacer_vide(mut contenu: ContenuExtrait, marques: &[MarquageVue]) -> ContenuExtrait {
    contenu.marquages = marques.to_vec();
    contenu
}

/// Un marquage tel que l'écran le voit.
#[derive(Clone, Debug, Serialize)]
pub struct MarquageVue {
    pub id: Uuid,
    pub fournisseur: String,
    pub table: String,
    pub ligne_id: Option<Uuid>,
    /// `None` : toutes les lignes de la table (ajout manuel sans ligne désignée, M5).
    pub rang: Option<i32>,
    pub colonne: Option<String>,
    /// Raison **telle que le lecteur peut la voir** (opaque `a_arbitrer` pour toute raison réservée à la direction
    /// quand le lecteur n'a pas `droits.arbitrer`, R-RG46, M4).
    pub raison: String,
    #[serde(skip)]
    pub raison_interne: String,
    pub propose_par: String,
    /// `a_decider` | `masque` | `leve`.
    pub decision: String,
    pub motif: Option<String>,
    pub decide_le: Option<DateTime<Utc>>,
    /// Sa levée demande `droits.arbitrer`.
    pub levee_reservee_direction: bool,
}

fn raison_visible(raison: &str, peut_arbitrer: bool) -> String {
    // M4 : l'alias couvre toutes les raisons réservées, sinon il équivaudrait au code `adresse_protegee`.
    if levee_reservee_direction(raison) && !peut_arbitrer {
        "a_arbitrer".to_string()
    } else {
        raison.to_string()
    }
}

/// Faits de marquage d'un élève demandeur ou tiers, tirés de la base.
pub(crate) async fn faits_eleve(
    conn: &mut PgConnection,
    etab: Uuid,
    sujet: Uuid,
    dem_compte: Option<Uuid>,
    dem_personne: Option<Uuid>,
    dem_est_sujet: bool,
) -> Result<Faits, EntDbError> {
    let mut f = Faits {
        sujet: Some(sujet),
        dem_compte,
        dem_est_sujet,
        ..Faits::default()
    };
    // Autres titulaires : le rattachement est la **personne** (`personne_id`, obligatoire) ; le compte est facultatif.
    // Un responsable sans compte est donc un « autre » comme les autres (E1).
    let liens: Vec<(Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
        "SELECT l.user_id, COALESCE(l.personne_id, p.id)
           FROM public.ent_eleve_lien l
           LEFT JOIN public.ent_personne p
             ON p.etablissement_id = l.etablissement_id AND p.user_id = l.user_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2
            AND NOT (l.user_id IS NOT NULL AND l.user_id = $3)
            AND NOT (COALESCE(l.personne_id, p.id) IS NOT NULL AND COALESCE(l.personne_id, p.id) = $4)",
    )
    .bind(etab)
    .bind(sujet)
    .bind(dem_compte)
    .bind(dem_personne)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (u, p) in &liens {
        if let Some(u) = u {
            f.autres.insert(*u);
        }
        if let Some(p) = p {
            f.autres.insert(*p);
        }
    }
    // Les responsables des dossiers d'inscription et les participants « famille » des fils de l'élève, quand ils
    // portent une fiche personne : même s'ils n'ont (plus) de lien actif, ils ne sont pas le demandeur.
    let autres_fiches: Vec<Uuid> = sqlx::query_scalar(
        "SELECT r.personne_id FROM public.ent_inscription_responsable r
           JOIN public.ent_inscription_dossier d ON d.etablissement_id = r.etablissement_id AND d.id = r.dossier_id
          WHERE r.etablissement_id = $1 AND d.eleve_id = $2 AND r.personne_id IS NOT NULL
         UNION
         SELECT pa.personne_id FROM public.ent_msg_participant pa
           JOIN public.ent_msg_fil_eleve fe ON fe.etablissement_id = pa.etablissement_id AND fe.fil_id = pa.fil_id
          WHERE pa.etablissement_id = $1 AND fe.eleve_id = $2 AND pa.cote = 'famille' AND pa.personne_id IS NOT NULL",
    )
    .bind(etab)
    .bind(sujet)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for p in autres_fiches {
        if Some(p) != dem_personne {
            f.autres.insert(p);
        }
    }
    f.dem_personne = dem_personne;
    if !f.autres.is_empty() {
        let ids: Vec<Uuid> = f.autres.iter().copied().collect();
        // Protection : on ne lit que l'identifiant de la personne protégée, jamais l'adresse ni la pièce.
        let proteges: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT p.id, p.user_id FROM public.ent_personne p
              WHERE p.etablissement_id = $1 AND (p.id = ANY($2) OR p.user_id = ANY($2))
                AND (EXISTS (SELECT 1 FROM public.ent_adresse_protegee a
                              WHERE a.etablissement_id = p.etablissement_id AND a.personne_id = p.id)
                     OR EXISTS (SELECT 1 FROM public.ent_personne_reserve r
                                 WHERE r.etablissement_id = p.etablissement_id AND r.personne_id = p.id
                                   AND r.ne_pas_solliciter))",
        )
        .bind(etab)
        .bind(&ids)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for (p, u) in proteges {
            f.proteges.insert(p);
            if let Some(u) = u {
                f.proteges.insert(u);
            }
        }
    }
    // Personnes « siennes » : la fiche de l'élève et celle du demandeur.
    let siens: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM public.ent_personne
          WHERE etablissement_id = $1 AND (eleve_id = $2 OR id = $3)",
    )
    .bind(etab)
    .bind(sujet)
    .bind(dem_personne)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    f.siens.extend(siens);
    Ok(f)
}

/// M6 : faits de marquage d'un sujet **personne** ou **agent** (sa fiche personne). Les élèves dont il est
/// responsable ne sont pas des « autres élèves » ; ses pièces sont à son nom.
pub(crate) async fn faits_personne(
    conn: &mut PgConnection,
    etab: Uuid,
    personne: Uuid,
    dem_compte: Option<Uuid>,
    dem_personne: Option<Uuid>,
) -> Result<Faits, EntDbError> {
    let eleves: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT l.eleve_id FROM public.ent_eleve_lien l
          WHERE l.etablissement_id = $1 AND l.personne_id = $2",
    )
    .bind(etab)
    .bind(personne)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(Faits {
        sujet: None,
        dem_compte,
        dem_personne,
        dem_est_sujet: false,
        siens: [personne].into_iter().collect(),
        eleves_lies: eleves.into_iter().collect(),
        ..Faits::default()
    })
}

/// F7 : les noms des **autres** élèves que le contenu cite par leur identifiant (`…eleve_id`), pour marquer aussi
/// leur nom dans les textes. Lecture d'identité seule, jamais rendue.
pub(crate) async fn noms_des_autres_eleves(
    conn: &mut PgConnection,
    etab: Uuid,
    contenu: &ContenuExtrait,
    f: &Faits,
) -> Result<Vec<String>, EntDbError> {
    let eleve = contenu.sujet_type == "eleve";
    let mut ids: BTreeSet<Uuid> = BTreeSet::new();
    for s in &contenu.sections {
        for t in &s.tables {
            for l in &t.lignes {
                for (c, v) in &l.valeurs {
                    if !c.ends_with("eleve_id") {
                        continue;
                    }
                    if let Some(u) = uuid_de(v) {
                        let autre = if eleve {
                            u != contenu.sujet_id
                        } else {
                            !f.eleves_lies.contains(&u)
                        };
                        if autre {
                            ids.insert(u);
                        }
                    }
                }
            }
        }
    }
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Uuid> = ids.into_iter().collect();
    let rows: Vec<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT e.prenom, e.nom FROM public.ent_eleve e WHERE e.etablissement_id = $1 AND e.id = ANY($2)",
    )
    .bind(etab)
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut noms: BTreeSet<String> = BTreeSet::new();
    for (prenom, nom) in rows {
        let nom = nom
            .map(|n| n.trim().to_string())
            .filter(|n| n.chars().count() >= 3);
        let prenom = prenom
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty());
        if let (Some(p), Some(n)) = (&prenom, &nom) {
            noms.insert(format!("{p} {n}"));
        }
        if let Some(n) = nom {
            noms.insert(n);
        }
    }
    // Les plus longs d'abord : déterministe, et « Léa Martin » se trouve avant « Martin ».
    let mut v: Vec<String> = noms.into_iter().collect();
    v.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
    Ok(v)
}

/// Usage de la clé dérivée qui calcule l'empreinte d'un élément marqué (M2).
pub const USAGE_EMPREINTE_ELEMENT: &str = "marquage-rgpd:v1";

/// Empreinte **à clé** d'un élément marqué, liée à la demande et à sa place (M2).
pub(crate) fn empreinte_element(
    secrets: &ent_auth::SecretBox,
    demande: Uuid,
    m: &MarquageCalcule,
) -> Result<String, EntDbError> {
    let donnees = format!(
        "{demande}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
        m.fournisseur,
        m.table,
        m.colonne.as_deref().unwrap_or("*"),
        m.element
    );
    secrets
        .hmac_hex(USAGE_EMPREINTE_ELEMENT, donnees.as_bytes())
        .map_err(EntDbError::Secrets)
}

/// Écrit les marquages d'un extrait ; une décision déjà prise sur le **même élément** d'une version précédente
/// du même extrait est reportée (la relecture n'est pas à refaire quand rien n'a changé).
///
/// M2 : « même élément » veut dire même place **et** même contenu (empreinte à clé identique). Une ligne avec
/// identifiant se retrouve par son identifiant, quel que soit son rang ; une ligne sans identifiant par son rang,
/// et une **levée** n'est jamais reportée sur elle (une insertion décale les rangs : on ne lève pas à l'aveugle).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn persister_marquages(
    conn: &mut PgConnection,
    secrets: &ent_auth::SecretBox,
    etab: Uuid,
    demande: Uuid,
    extrait: Uuid,
    sujet_type: &str,
    sujet_id: Uuid,
    marques: &[MarquageCalcule],
) -> Result<(), EntDbError> {
    for m in marques {
        let empreinte = empreinte_element(secrets, demande, m)?;
        sqlx::query(
            "INSERT INTO rgp_extrait_masquage
                 (id, etablissement_id, extrait_id, fournisseur_code, table_nom, ligne_id, rang, colonne,
                  raison, propose_par, decision, motif, decide_par_compte_id, decide_le, element_empreinte)
             SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, 'sys',
                    COALESCE(prev.decision, 'a_decider'), prev.motif, prev.decide_par_compte_id, prev.decide_le, $13
               FROM (SELECT 1) AS x
               LEFT JOIN LATERAL (
                    SELECT mm.decision, mm.motif, mm.decide_par_compte_id, mm.decide_le
                      FROM public.rgp_extrait_masquage mm
                      JOIN public.rgp_extrait e ON e.etablissement_id = mm.etablissement_id AND e.id = mm.extrait_id
                     WHERE e.etablissement_id = $2 AND e.demande_id = $10 AND e.sujet_type = $11
                       AND e.sujet_id = $12 AND e.id <> $3
                       AND mm.fournisseur_code = $4 AND mm.table_nom = $5
                       AND (($6::uuid IS NOT NULL AND mm.ligne_id = $6)
                            OR ($6::uuid IS NULL AND mm.ligne_id IS NULL AND mm.rang = $7 AND mm.decision <> 'leve'))
                       AND mm.colonne IS NOT DISTINCT FROM $8 AND mm.raison = $9
                       AND mm.propose_par = 'sys' AND mm.decision <> 'a_decider'
                       AND mm.element_empreinte = $13
                     ORDER BY e.version DESC LIMIT 1) prev ON true",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(extrait)
        .bind(&m.fournisseur)
        .bind(&m.table)
        .bind(m.ligne_id)
        .bind(m.rang)
        .bind(&m.colonne)
        .bind(m.raison)
        .bind(demande)
        .bind(sujet_type)
        .bind(sujet_id)
        .bind(&empreinte)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    // M1 : un masquage ajouté **à la main** (RGP-151) survit au réassemblage : il est recopié, avec sa décision,
    // sur la nouvelle version. Sinon l'élément retiré par l'agent reviendrait dans l'extrait suivant.
    sqlx::query(
        "INSERT INTO rgp_extrait_masquage
             (id, etablissement_id, extrait_id, fournisseur_code, table_nom, ligne_id, rang, colonne, raison,
              propose_par, decision, motif, decide_par_compte_id, decide_le)
         SELECT gen_random_uuid(), mm.etablissement_id, $3, mm.fournisseur_code, mm.table_nom, mm.ligne_id,
                mm.rang, mm.colonne, mm.raison, mm.propose_par, mm.decision, mm.motif,
                mm.decide_par_compte_id, mm.decide_le
           FROM public.rgp_extrait_masquage mm
          WHERE mm.etablissement_id = $2 AND mm.propose_par = 'agent'
            AND mm.extrait_id = (SELECT e.id FROM public.rgp_extrait e
                                  WHERE e.etablissement_id = $2 AND e.demande_id = $1 AND e.sujet_type = $4
                                    AND e.sujet_id = $5 AND e.id <> $3
                                  ORDER BY e.version DESC LIMIT 1)",
    )
    .bind(demande)
    .bind(etab)
    .bind(extrait)
    .bind(sujet_type)
    .bind(sujet_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Empreinte des marquages **calculés** : si les faits (protection, lien, participation) changent après
/// l'assemblage, elle change, et l'extrait est périmé (M3).
pub fn empreinte_marques(marques: &[MarquageCalcule]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for m in marques {
        h.update(format!(
            "{}|{}|{:?}|{}|{:?}|{}\n",
            m.fournisseur, m.table, m.ligne_id, m.rang, m.colonne, m.raison
        ));
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Les marquages d'un extrait, vus par un lecteur selon ses droits.
pub(crate) async fn marquages_de(
    conn: &mut PgConnection,
    etab: Uuid,
    extrait: Uuid,
    peut_arbitrer: bool,
) -> Result<Vec<MarquageVue>, EntDbError> {
    let rows = sqlx::query(
        "SELECT id, fournisseur_code, table_nom, ligne_id, rang, colonne, raison, propose_par, decision, motif,
                decide_le
           FROM rgp_extrait_masquage WHERE etablissement_id = $1 AND extrait_id = $2
          ORDER BY fournisseur_code, table_nom, rang NULLS FIRST, colonne NULLS FIRST, id",
    )
    .bind(etab)
    .bind(extrait)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .iter()
        .map(|r| {
            let raison: String = r.get("raison");
            MarquageVue {
                id: r.get("id"),
                fournisseur: r.get("fournisseur_code"),
                table: r.get("table_nom"),
                ligne_id: r.get("ligne_id"),
                rang: r.get::<Option<i32>, _>("rang"),
                colonne: r.get("colonne"),
                raison: raison_visible(&raison, peut_arbitrer),
                levee_reservee_direction: levee_reservee_direction(&raison),
                raison_interne: raison,
                propose_par: r.get("propose_par"),
                decision: r.get("decision"),
                motif: r.get("motif"),
                decide_le: r.get("decide_le"),
            }
        })
        .collect())
}

/// M4 : pour un lecteur sans `droits.arbitrer`, les marquages de protection ne sont pas listés.
/// Même filtre que [`cacher_protections`] (E1, E2), sans le contenu.
pub(crate) fn sans_protections_si(v: Vec<MarquageVue>, peut_arbitrer: bool) -> Vec<MarquageVue> {
    if peut_arbitrer {
        v
    } else {
        Reserve::depuis(&v).filtrer(v).0
    }
}

/// F5 : toute décision de marquage prise après l'avis du DPO l'efface (l'avis portait sur une autre relecture) ; la
/// soumission reste, le DPO redonne son avis.
async fn effacer_avis_dpo(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    extrait: Uuid,
    acteur: Uuid,
    role: &str,
) -> Result<(), EntDbError> {
    let n = sqlx::query(
        "UPDATE rgp_extrait
            SET dpo_avis_le = NULL, dpo_avis_par_compte_id = NULL, dpo_avis_code = NULL, dpo_avis_texte = NULL,
                modifie_le = now()
          WHERE etablissement_id = $1 AND id = $2 AND dpo_avis_le IS NOT NULL",
    )
    .bind(etab)
    .bind(extrait)
    .execute(&mut *conn)
    .await
    .map_err(db)?
    .rows_affected();
    if n > 0 {
        journal(
            conn,
            etab,
            demande,
            "extrait_avis_dpo",
            Some("avis du DPO retiré : la relecture a changé depuis, nouvel avis à donner"),
            Some(acteur),
            Some(role),
        )
        .await?;
    }
    Ok(())
}

/// Nombre de marquages restant à décider : aucune remise tant qu'il n'est pas nul.
pub(crate) async fn restant_a_decider(
    conn: &mut PgConnection,
    etab: Uuid,
    extrait: Uuid,
) -> Result<i64, EntDbError> {
    sqlx::query_scalar(
        "SELECT count(*) FROM rgp_extrait_masquage
          WHERE etablissement_id = $1 AND extrait_id = $2 AND decision = 'a_decider'",
    )
    .bind(etab)
    .bind(extrait)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// Le DPO ou le gestionnaire peut-il agir sur les marquages de cet extrait ?
async fn extrait_actif(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    extrait: Uuid,
) -> Result<(), EntDbError> {
    let perime: Option<Option<DateTime<Utc>>> = sqlx::query_scalar(
        "SELECT perime_le FROM rgp_extrait WHERE etablissement_id = $1 AND demande_id = $2 AND id = $3",
    )
    .bind(etab)
    .bind(demande)
    .bind(extrait)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    match perime {
        None => Err(EntDbError::Domain(DomainError::NotFound)),
        Some(Some(_)) => Err(EntDbError::ConflitDossier(
            "extrait périmé : assembler une nouvelle version",
        )),
        Some(None) => Ok(()),
    }
}

impl PgEnt {
    /// Les marquages d'un extrait (`droits.traiter`, ou `droits.arbitrer` si l'extrait est soumis au DPO).
    pub async fn droits_masquages(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        extrait: Uuid,
    ) -> Result<Vec<MarquageVue>, EntDbError> {
        let traite = self
            .a_action_etablissement(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let arbitre = self
            .a_action_etablissement(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        if !traite && !arbitre {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let soumis: Option<Option<DateTime<Utc>>> = sqlx::query_scalar(
            "SELECT soumis_dpo_le FROM rgp_extrait WHERE etablissement_id = $1 AND demande_id = $2 AND id = $3",
        )
        .bind(etab)
        .bind(demande)
        .bind(extrait)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some(soumis) = soumis else {
            return Err(EntDbError::Domain(DomainError::NotFound));
        };
        if !traite && soumis.is_none() {
            // F6 : même réponse qu'un extrait inexistant (pas d'oracle d'existence pour le DPO).
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        let v = marquages_de(&mut tx, etab, extrait, arbitre).await?;
        Ok(sans_protections_si(v, arbitre))
    }

    /// **RGP-150** — décide un marquage : `masque` (accepté, la valeur est masquée dans ce qui est remis) ou
    /// `leve` (avec motif). Lever une raison réservée à la direction exige `droits.arbitrer` ; un porteur de
    /// `droits.traiter` seul reçoit le même refus qu'un marquage inexistant.
    pub async fn droits_masquage_decider(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        masquage: Uuid,
        decision: &str,
        motif: Option<&str>,
    ) -> Result<Vec<MarquageVue>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        if !matches!(decision, "masque" | "leve") {
            return Err(invalide("décision inconnue"));
        }
        let motif_n = match decision {
            "leve" => Some(
                motif
                    .and_then(|m| texte(m, 3, 1000))
                    .ok_or(invalide("motif obligatoire pour lever un marquage"))?,
            ),
            _ => motif.and_then(|m| texte(m, 3, 1000)),
        };
        let peut_arbitrer = self
            .a_action_etablissement(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        let r = sqlx::query(
            "SELECT m.extrait_id, m.raison FROM rgp_extrait_masquage m
               JOIN rgp_extrait e ON e.etablissement_id = m.etablissement_id AND e.id = m.extrait_id
              WHERE m.etablissement_id = $1 AND m.id = $2 AND e.demande_id = $3 FOR UPDATE OF m",
        )
        .bind(etab)
        .bind(masquage)
        .bind(demande)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let extrait: Uuid = r.get("extrait_id");
        let raison: String = r.get("raison");
        extrait_actif(&mut tx, etab, demande, extrait).await?;
        if decision == "leve" && levee_reservee_direction(&raison) && !peut_arbitrer {
            // Même réponse qu'un élément introuvable : le code de la raison ne se devine pas par le refus.
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        sqlx::query(
            "UPDATE rgp_extrait_masquage
                SET decision = $3, motif = $4, decide_par_compte_id = $5, decide_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(masquage)
        .bind(decision)
        .bind(&motif_n)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "masquage_decide",
            Some(if decision == "leve" {
                "un marquage a été levé (motif au dossier)"
            } else {
                "un marquage a été accepté"
            }),
            Some(acteur),
            Some(role),
        )
        .await?;
        effacer_avis_dpo(&mut tx, etab, demande, extrait, acteur, role).await?;
        let v = marquages_de(&mut tx, etab, extrait, peut_arbitrer).await?;
        tx.commit().await?;
        Ok(sans_protections_si(v, peut_arbitrer))
    }

    /// **RGP-151** — ajoute à la main un masquage, avec motif. Il est décidé « masqué » d'emblée. `droits.traiter`.
    ///
    /// M5 : l'élément désigné doit **exister** dans le contenu de l'extrait (reconstruit, jamais servi ici) :
    /// * une table entière (ni ligne ni rang) : toutes ses lignes, ou toute une colonne si `colonne` est donnée ;
    /// * une ligne par son identifiant (`ligne_id`), ou par son rang (`rang`, 0 pour la première) si elle n'en a pas ;
    /// * une colonne, qui doit figurer dans la ou les lignes désignées.
    ///
    /// Un élément introuvable est refusé (une faute de frappe ne crée plus un masquage sans effet affiché « masqué ») ;
    /// le même élément masqué deux fois à la main est refusé (409).
    #[allow(clippy::too_many_arguments)]
    pub async fn droits_masquage_ajouter(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        extrait: Uuid,
        fournisseur: &str,
        table: &str,
        ligne_id: Option<Uuid>,
        rang: Option<i32>,
        colonne: Option<&str>,
        motif: &str,
    ) -> Result<Vec<MarquageVue>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let motif = texte(motif, 3, 1000).ok_or(invalide("motif obligatoire"))?;
        let ident = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        };
        if !ident(fournisseur) || !ident(table) || colonne.is_some_and(|c| !ident(c)) {
            return Err(invalide("élément à masquer incohérent"));
        }
        if ligne_id.is_some() && rang.is_some() {
            return Err(invalide(
                "désigner la ligne par son identifiant ou par son rang, pas les deux",
            ));
        }
        if rang.is_some_and(|r| r < 0) {
            return Err(invalide("rang de ligne incohérent"));
        }
        let peut_arbitrer = self
            .a_action_etablissement(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat = etat_courant(&mut tx, etab, demande).await?;
        exiger_ouverte(&etat)?;
        extrait_actif(&mut tx, etab, demande, extrait).await?;
        let section: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM rgp_extrait_section
                             WHERE etablissement_id = $1 AND extrait_id = $2 AND fournisseur_code = $3)",
        )
        .bind(etab)
        .bind(extrait)
        .bind(fournisseur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !section {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        let contenu = self
            .droits_contenu_reconstruit(&mut tx, etab, demande, extrait)
            .await?;
        // Pour un lecteur sans `droits.arbitrer`, ce qui est protégé n'existe pas : il ne peut pas le désigner.
        let contenu = if peut_arbitrer {
            contenu
        } else {
            let m = marquages_de(&mut tx, etab, extrait, false).await?;
            cacher_protections(&contenu, &m)
        };
        element_existe(&contenu, fournisseur, table, ligne_id, rang, colonne)?;
        // E2 : le rang désigné est celui que le lecteur voit ; on stocke le rang d'origine, que `vise` compare.
        let rang = match (rang, peut_arbitrer) {
            (Some(r), false) => {
                let m = marquages_de(&mut tx, etab, extrait, false).await?;
                Some(Reserve::depuis(&m).rang_d_origine(fournisseur, table, r))
            }
            (r, _) => r,
        };
        let deja: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM rgp_extrait_masquage
                             WHERE etablissement_id = $1 AND extrait_id = $2 AND fournisseur_code = $3
                               AND table_nom = $4 AND ligne_id IS NOT DISTINCT FROM $5 AND rang IS NOT DISTINCT FROM $6
                               AND colonne IS NOT DISTINCT FROM $7 AND raison = 'manuel')",
        )
        .bind(etab)
        .bind(extrait)
        .bind(fournisseur)
        .bind(table)
        .bind(ligne_id)
        .bind(rang)
        .bind(colonne)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja {
            return Err(EntDbError::ConflitDossier(
                "cet élément est déjà masqué à la main",
            ));
        }
        sqlx::query(
            "INSERT INTO rgp_extrait_masquage
                 (id, etablissement_id, extrait_id, fournisseur_code, table_nom, ligne_id, rang, colonne,
                  raison, propose_par, decision, motif, decide_par_compte_id, decide_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'manuel', 'agent', 'masque', $9, $10, now())",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(extrait)
        .bind(fournisseur)
        .bind(table)
        .bind(ligne_id)
        .bind(rang)
        .bind(colonne)
        .bind(&motif)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "masquage_ajoute",
            Some("un masquage a été ajouté à la main"),
            Some(acteur),
            Some(role),
        )
        .await?;
        effacer_avis_dpo(&mut tx, etab, demande, extrait, acteur, role).await?;
        let v = marquages_de(&mut tx, etab, extrait, peut_arbitrer).await?;
        tx.commit().await?;
        Ok(sans_protections_si(v, peut_arbitrer))
    }
}

/// M5 : l'élément désigné par un ajout manuel existe-t-il dans le contenu ?
fn element_existe(
    contenu: &ContenuExtrait,
    fournisseur: &str,
    table: &str,
    ligne_id: Option<Uuid>,
    rang: Option<i32>,
    colonne: Option<&str>,
) -> Result<(), EntDbError> {
    let absent = || invalide("élément à masquer absent de l'extrait");
    let t = contenu
        .sections
        .iter()
        .filter(|s| s.fournisseur == fournisseur)
        .flat_map(|s| s.tables.iter())
        .find(|t| t.nom == table)
        .ok_or_else(absent)?;
    let lignes: Vec<&crate::droits_extrait::LigneContenu> = match (ligne_id, rang) {
        (Some(id), _) => t.lignes.iter().filter(|l| l.id == Some(id)).collect(),
        (None, Some(r)) => t.lignes.get(r as usize).into_iter().collect(),
        (None, None) => t.lignes.iter().collect(),
    };
    if lignes.is_empty() {
        return Err(absent());
    }
    if let Some(c) = colonne {
        if !lignes.iter().any(|l| l.valeurs.iter().any(|(n, _)| n == c)) {
            return Err(absent());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::droits_extrait::{LigneContenu, StatutSection, TableContenu};

    fn l(id: Option<Uuid>, v: &[(&str, &str)]) -> LigneContenu {
        LigneContenu {
            id,
            valeurs: v
                .iter()
                .map(|(c, x)| (c.to_string(), x.to_string()))
                .collect(),
        }
    }

    fn contenu(tables: Vec<TableContenu>) -> ContenuExtrait {
        ContenuExtrait {
            sujet_type: "eleve".into(),
            sujet_id: Uuid::from_u128(1),
            periode_du: None,
            periode_au: None,
            sections: vec![SectionContenu {
                fournisseur: "administratif".into(),
                socle: "4".into(),
                statut: StatutSection::Ok,
                motif_code: None,
                tables,
                exclusions: vec![],
                tables_sans_rattachement: 0,
                lignes: 0,
                empreinte: None,
            }],
            complet: true,
            empreinte: String::new(),
            nb_lignes: 0,
            destinataires_internes: vec![],
            destinataires_externes: vec![],
            soustraitants: vec![],
            durees: vec![],
            sources: vec![],
            marquages: vec![],
            elements_reserves_direction: 0,
            instant: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
        }
    }

    #[test]
    fn marquage_deterministe_et_stable() {
        let mere = Uuid::from_u128(7);
        let t = TableContenu {
            nom: "ent_inscription_responsable".into(),
            lignes: vec![l(
                Some(Uuid::from_u128(9)),
                &[
                    ("personne_id", &mere.to_string()),
                    ("telephone", "0600000000"),
                    ("nom", "Granger"),
                ],
            )],
        };
        let f = Faits {
            autres: [mere].into_iter().collect(),
            ..Faits::default()
        };
        let a = marquer(&contenu(vec![t.clone()]), &f);
        let b = marquer(&contenu(vec![t]), &f);
        assert_eq!(a, b);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].raison, "autre_parent");
        assert!(a[0].colonne.is_none());
    }

    #[test]
    fn eleve_demandeur_garde_l_identite_de_ses_responsables() {
        let mere = Uuid::from_u128(7);
        let t = TableContenu {
            nom: "ent_inscription_responsable".into(),
            lignes: vec![l(
                Some(Uuid::from_u128(9)),
                &[
                    ("personne_id", &mere.to_string()),
                    ("telephone", "0600000000"),
                    ("nom", "Granger"),
                ],
            )],
        };
        let f = Faits {
            autres: [mere].into_iter().collect(),
            dem_est_sujet: true,
            ..Faits::default()
        };
        let a = marquer(&contenu(vec![t]), &f);
        assert!(a.iter().all(|m| m.raison == "parent_donnees_propres"));
        assert!(a.iter().any(|m| m.colonne.as_deref() == Some("telephone")));
        assert!(!a.iter().any(|m| m.colonne.as_deref() == Some("nom")));
    }

    #[test]
    fn indices_de_localisation_masques_tant_qu_un_parent_est_protege() {
        let mere = Uuid::from_u128(7);
        let t = TableContenu {
            nom: "ent_eleve".into(),
            lignes: vec![l(
                Some(Uuid::from_u128(1)),
                &[("ville", "Lyon"), ("nom", "Lea")],
            )],
        };
        let f = Faits {
            autres: [mere].into_iter().collect(),
            proteges: [mere].into_iter().collect(),
            ..Faits::default()
        };
        let a = marquer(&contenu(vec![t]), &f);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].raison, "adresse_protegee");
        assert_eq!(a[0].colonne.as_deref(), Some("ville"));
    }

    #[test]
    fn masque_applique_remplace_la_valeur_et_respecte_la_levee() {
        let c = contenu(vec![TableContenu {
            nom: "ent_eleve".into(),
            lignes: vec![l(
                Some(Uuid::from_u128(1)),
                &[("ville", "Lyon"), ("nom", "Lea")],
            )],
        }]);
        let vue = |decision: &str| MarquageVue {
            id: Uuid::nil(),
            fournisseur: "administratif".into(),
            table: "ent_eleve".into(),
            ligne_id: Some(Uuid::from_u128(1)),
            rang: Some(0),
            colonne: Some("ville".into()),
            raison: "adresse_protegee".into(),
            raison_interne: "adresse_protegee".into(),
            propose_par: "sys".into(),
            decision: decision.into(),
            motif: None,
            decide_le: None,
            levee_reservee_direction: true,
        };
        let m = appliquer_masques(&c, &[vue("a_decider")]);
        let v = &m.sections[0].tables[0].lignes[0].valeurs;
        assert_eq!(v[0].1, "[masqué : information protégée]");
        assert_eq!(v[1].1, "Lea");
        let m = appliquer_masques(&c, &[vue("leve")]);
        assert_eq!(m.sections[0].tables[0].lignes[0].valeurs[0].1, "Lyon");
    }

    #[test]
    fn la_levee_d_une_protection_est_reservee_a_la_direction() {
        for r in [
            "adresse_protegee",
            "autre_parent",
            "parent_donnees_propres",
            "piece_tiers",
        ] {
            assert!(levee_reservee_direction(r), "{r}");
        }
        for r in ["autre_eleve", "redacteur", "manuel"] {
            assert!(!levee_reservee_direction(r), "{r}");
        }
        assert_eq!(raison_visible("adresse_protegee", false), "a_arbitrer");
        assert_eq!(raison_visible("adresse_protegee", true), "adresse_protegee");
    }

    fn vue_de(m: &MarquageCalcule, id: u128) -> MarquageVue {
        MarquageVue {
            id: Uuid::from_u128(id),
            fournisseur: m.fournisseur.clone(),
            table: m.table.clone(),
            ligne_id: m.ligne_id,
            rang: Some(m.rang),
            colonne: m.colonne.clone(),
            raison: m.raison.into(),
            raison_interne: m.raison.into(),
            propose_par: "sys".into(),
            decision: "a_decider".into(),
            motif: None,
            decide_le: None,
            levee_reservee_direction: levee_reservee_direction(m.raison),
        }
    }

    /// F7 : le nom d'un autre élève dans un texte est marqué, sur un mot entier et sans casse.
    #[test]
    fn le_nom_d_un_autre_eleve_dans_un_texte_est_marque() {
        let t = TableContenu {
            nom: "ent_incident".into(),
            lignes: vec![l(
                Some(Uuid::from_u128(5)),
                &[
                    ("eleve_id", &Uuid::from_u128(1).to_string()),
                    ("recit", "Bousculade avec Tom MARTIN dans la cour"),
                    ("lieu", "Martinique"),
                ],
            )],
        };
        let f = Faits {
            noms_autres_eleves: vec!["Tom Martin".into(), "Martin".into()],
            ..Faits::default()
        };
        let a = marquer(&contenu(vec![t]), &f);
        assert_eq!(a.len(), 1, "{a:?}");
        assert_eq!(a[0].colonne.as_deref(), Some("recit"));
        assert_eq!(a[0].raison, "autre_eleve");
        assert!(!cite_un_nom("Martinique", &["Martin".into()]));
        assert!(cite_un_nom("vu par martin.", &["Martin".into()]));
    }

    /// M6 : pour un sujet « personne », une pièce du foyer est marquée ; une pièce propre au porteur ne l'est pas ;
    /// un élève qui n'est pas le sien est un « autre élève ».
    #[test]
    fn sujet_personne_pieces_de_tiers_et_autres_eleves() {
        let enfant = Uuid::from_u128(21);
        let autre = Uuid::from_u128(22);
        let mut c = contenu(vec![
            TableContenu {
                nom: "ent_doc_piece".into(),
                lignes: vec![
                    l(
                        Some(Uuid::from_u128(31)),
                        &[("type_code", "livret_famille")],
                    ),
                    l(Some(Uuid::from_u128(32)), &[("type_code", "type_disparu")]),
                ],
            },
            TableContenu {
                nom: "ent_rdv".into(),
                lignes: vec![
                    l(
                        Some(Uuid::from_u128(41)),
                        &[("eleve_id", &enfant.to_string())],
                    ),
                    l(
                        Some(Uuid::from_u128(42)),
                        &[("eleve_id", &autre.to_string())],
                    ),
                ],
            },
        ]);
        c.sujet_type = "personne".into();
        let f = Faits {
            eleves_lies: [enfant].into_iter().collect(),
            ..Faits::default()
        };
        let a = marquer(&c, &f);
        let pieces: Vec<_> = a.iter().filter(|m| m.raison == "piece_tiers").collect();
        assert_eq!(pieces.len(), 2, "pièce du foyer et type inconnu : {a:?}");
        let eleves: Vec<_> = a.iter().filter(|m| m.raison == "autre_eleve").collect();
        assert_eq!(eleves.len(), 1);
        assert_eq!(eleves[0].ligne_id, Some(Uuid::from_u128(42)));
    }

    /// M2 : l'élément porté par un marquage suit le contenu (sa forme canonique change avec la valeur).
    #[test]
    fn l_element_marque_suit_le_contenu() {
        let mere = Uuid::from_u128(7);
        let t = |tel: &str| TableContenu {
            nom: "ent_inscription_responsable".into(),
            lignes: vec![l(
                Some(Uuid::from_u128(9)),
                &[("personne_id", &mere.to_string()), ("telephone", tel)],
            )],
        };
        let f = Faits {
            autres: [mere].into_iter().collect(),
            ..Faits::default()
        };
        let a = marquer(&contenu(vec![t("0600000000")]), &f);
        let b = marquer(&contenu(vec![t("0611111111")]), &f);
        assert_eq!(a.len(), 1);
        assert_ne!(a[0].element, b[0].element);
        assert_eq!(
            empreinte_marques(&a),
            empreinte_marques(&b),
            "les faits n'ont pas changé : l'extrait n'est pas périmé pour autant"
        );
        let k = ent_auth::SecretBox::dev();
        let d = Uuid::from_u128(3);
        assert_ne!(
            empreinte_element(&k, d, &a[0]).unwrap(),
            empreinte_element(&k, d, &b[0]).unwrap()
        );
        assert_eq!(
            empreinte_element(&k, d, &a[0]).unwrap(),
            empreinte_element(&k, d, &a[0].clone()).unwrap()
        );
    }

    /// M4 : sans `droits.arbitrer`, la cellule protégée disparaît, la ligne protégée aussi ; seul le compteur reste.
    #[test]
    fn une_protection_est_absente_de_la_vue_du_secretariat() {
        let mere = Uuid::from_u128(7);
        let c = contenu(vec![
            TableContenu {
                nom: "ent_eleve".into(),
                lignes: vec![l(
                    Some(Uuid::from_u128(1)),
                    &[("nom", "Lea"), ("ville", "Lyon")],
                )],
            },
            TableContenu {
                nom: "ent_inscription_responsable".into(),
                lignes: vec![
                    l(
                        Some(Uuid::from_u128(9)),
                        &[("nom", "Dupont"), ("personne_id", &mere.to_string())],
                    ),
                    l(Some(Uuid::from_u128(10)), &[("nom", "Granger")]),
                ],
            },
        ]);
        let f = Faits {
            autres: [mere].into_iter().collect(),
            proteges: [mere].into_iter().collect(),
            dem_personne: Some(Uuid::from_u128(99)),
            ..Faits::default()
        };
        let marques: Vec<MarquageVue> = marquer(&c, &f)
            .iter()
            .enumerate()
            .map(|(i, m)| vue_de(m, i as u128 + 1))
            .collect();
        let vu = cacher_protections(&c, &marques);
        let t = format!("{:?}", vu.sections[0].tables);
        assert!(
            !t.contains("Lyon") && !t.contains("ville") && !t.contains("Dupont"),
            "{t}"
        );
        assert!(t.contains("Lea"));
        assert!(vu.elements_reserves_direction > 0);
        assert!(vu
            .marquages
            .iter()
            .all(|m| m.raison_interne != "adresse_protegee"));
        // Toutes les raisons réservées portent le même alias.
        for r in [
            "autre_parent",
            "adresse_protegee",
            "piece_tiers",
            "signalement",
        ] {
            assert_eq!(raison_visible(r, false), "a_arbitrer");
        }
        assert_eq!(raison_visible("redacteur", false), "redacteur");
    }

    /// M5 : un masquage sans ligne ni rang vise toutes les lignes ; avec un rang, la seule ligne de ce rang.
    #[test]
    fn un_masquage_sans_ligne_vise_toute_la_table() {
        let c = contenu(vec![TableContenu {
            nom: "ent_note".into(),
            lignes: vec![l(None, &[("texte", "a")]), l(None, &[("texte", "b")])],
        }]);
        let mut m = vue_de(
            &MarquageCalcule {
                fournisseur: "administratif".into(),
                table: "ent_note".into(),
                ligne_id: None,
                rang: 0,
                colonne: Some("texte".into()),
                raison: "manuel",
                element: String::new(),
            },
            1,
        );
        m.decision = "masque".into();
        m.rang = None;
        let r = appliquer_masques(&c, std::slice::from_ref(&m));
        assert!(r.sections[0].tables[0]
            .lignes
            .iter()
            .all(|l| l.valeurs[0].1 == "[masqué]"));
        m.rang = Some(1);
        let r = appliquer_masques(&c, &[m]);
        assert_eq!(r.sections[0].tables[0].lignes[0].valeurs[0].1, "a");
        assert_eq!(r.sections[0].tables[0].lignes[1].valeurs[0].1, "[masqué]");
        assert!(element_existe(
            &c,
            "administratif",
            "ent_note",
            None,
            Some(1),
            Some("texte")
        )
        .is_ok());
        assert!(element_existe(&c, "administratif", "ent_note", None, Some(2), None).is_err());
        assert!(element_existe(&c, "administratif", "ent_absente", None, None, None).is_err());
        assert!(element_existe(&c, "autre", "ent_note", None, None, None).is_err());
    }

    /// E1, E2 (contre-revue des suites) : aucun marquage visible ne désigne une ligne protégée retirée ; les rangs
    /// montrés suivent le contenu filtré, et le rang que désigne le lecteur se retraduit en rang d'origine.
    #[test]
    fn aucun_marquage_visible_ne_situe_une_ligne_protegee() {
        let mere = Uuid::from_u128(7);
        let c = contenu(vec![TableContenu {
            nom: "ent_inscription_responsable".into(),
            lignes: vec![
                l(
                    None,
                    &[("nom", "Dupont"), ("personne_id", &mere.to_string())],
                ),
                l(None, &[("nom", "Martin")]),
                l(
                    None,
                    &[
                        ("nom", "Granger"),
                        ("personne_id", &Uuid::from_u128(99).to_string()),
                    ],
                ),
            ],
        }]);
        let f = Faits {
            autres: [mere].into_iter().collect(),
            proteges: [mere].into_iter().collect(),
            dem_personne: Some(Uuid::from_u128(99)),
            ..Faits::default()
        };
        let marques: Vec<MarquageVue> = marquer(&c, &f)
            .iter()
            .enumerate()
            .map(|(i, m)| vue_de(m, i as u128 + 1))
            .collect();
        assert!(
            marques
                .iter()
                .any(|m| m.raison_interne == "autre_parent" && m.rang == Some(0)),
            "la ligne protégée porte aussi un marquage « autre parent » : {marques:?}"
        );
        let vu = cacher_protections(&c, &marques);
        let lignes = &vu.sections[0].tables[0].lignes;
        assert_eq!(lignes.len(), 2);
        assert!(
            vu.marquages
                .iter()
                .all(|m| m.rang != Some(0) || lignes[0].valeurs.iter().any(|(_, v)| v == "Martin")),
            "{:?}",
            vu.marquages
        );
        // La ligne « Martin » (rang d'origine 1) est montrée au rang 0, et son marquage aussi.
        let martin = vu
            .marquages
            .iter()
            .find(|m| m.raison_interne == "autre_parent")
            .expect("marquage de la ligne sans fiche");
        assert_eq!(martin.rang, Some(0));
        assert_eq!(
            vu.elements_reserves_direction,
            marques.len() - vu.marquages.len()
        );
        assert!(
            vu.elements_reserves_direction >= 2,
            "protection et marquage frère cachés"
        );
        let r = Reserve::depuis(&marques);
        assert_eq!(
            r.rang_d_origine("administratif", "ent_inscription_responsable", 0),
            1
        );
        assert_eq!(
            r.rang_d_origine("administratif", "ent_inscription_responsable", 1),
            2
        );
        // Même filtre sans le contenu.
        let liste = sans_protections_si(marques.clone(), false);
        assert_eq!(liste.len(), vu.marquages.len());
    }
}
