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
                });
            };
            if signalement {
                marque(None, "signalement");
                continue;
            }
            if t.nom == "ent_doc_piece" {
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
                let Some(u) = uuid_de(v) else { continue };
                if eleve && c.ends_with("eleve_id") && u != sujet {
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

/// Pour un lecteur sans `droits.arbitrer` : les éléments protégés ne sont pas montrés, même pendant la relecture.
pub fn cacher_protections(contenu: &ContenuExtrait, marques: &[MarquageVue]) -> ContenuExtrait {
    remplacer(contenu, marques, |m| {
        (m.raison_interne == "adresse_protegee").then(|| "[réservé à la direction]".to_string())
    })
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
                    m.fournisseur == s.fournisseur
                        && m.table == t.nom
                        && match (m.ligne_id, lid) {
                            (Some(a), Some(b)) => a == b,
                            _ => m.rang == rang as i32,
                        }
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
    pub rang: i32,
    pub colonne: Option<String>,
    /// Raison **telle que le lecteur peut la voir** (opaque `a_arbitrer` pour une protection, R-RG46).
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
    if raison == "adresse_protegee" && !peut_arbitrer {
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

/// Écrit les marquages d'un extrait ; une décision déjà prise sur le **même élément** d'une version précédente
/// du même extrait est reportée (la relecture n'est pas à refaire quand rien n'a changé).
pub(crate) async fn persister_marquages(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    extrait: Uuid,
    sujet_type: &str,
    sujet_id: Uuid,
    marques: &[MarquageCalcule],
) -> Result<(), EntDbError> {
    for m in marques {
        sqlx::query(
            "INSERT INTO rgp_extrait_masquage
                 (id, etablissement_id, extrait_id, fournisseur_code, table_nom, ligne_id, rang, colonne,
                  raison, propose_par, decision, motif, decide_par_compte_id, decide_le)
             SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, 'sys',
                    COALESCE(prev.decision, 'a_decider'), prev.motif, prev.decide_par_compte_id, prev.decide_le
               FROM (SELECT 1) AS x
               LEFT JOIN LATERAL (
                    SELECT mm.decision, mm.motif, mm.decide_par_compte_id, mm.decide_le
                      FROM public.rgp_extrait_masquage mm
                      JOIN public.rgp_extrait e ON e.etablissement_id = mm.etablissement_id AND e.id = mm.extrait_id
                     WHERE e.etablissement_id = $2 AND e.demande_id = $10 AND e.sujet_type = $11
                       AND e.sujet_id = $12 AND e.id <> $3
                       AND mm.fournisseur_code = $4 AND mm.table_nom = $5
                       AND mm.ligne_id IS NOT DISTINCT FROM $6 AND mm.rang = $7
                       AND mm.colonne IS NOT DISTINCT FROM $8 AND mm.raison = $9
                       AND mm.propose_par = 'sys' AND mm.decision <> 'a_decider'
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
          ORDER BY fournisseur_code, table_nom, rang, colonne NULLS FIRST, id",
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
                rang: r.get("rang"),
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
            return Err(EntDbError::Forbidden);
        }
        marquages_de(&mut tx, etab, extrait, arbitre).await
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
        let v = marquages_de(&mut tx, etab, extrait, peut_arbitrer).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// **RGP-151** — ajoute à la main un masquage (une table, une ligne ou une colonne de l'extrait), avec motif.
    /// Il est décidé « masqué » d'emblée. `droits.traiter`.
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
        sqlx::query(
            "INSERT INTO rgp_extrait_masquage
                 (id, etablissement_id, extrait_id, fournisseur_code, table_nom, ligne_id, rang, colonne,
                  raison, propose_par, decision, motif, decide_par_compte_id, decide_le)
             VALUES ($1, $2, $3, $4, $5, $6, 0, $7, 'manuel', 'agent', 'masque', $8, $9, now())",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(extrait)
        .bind(fournisseur)
        .bind(table)
        .bind(ligne_id)
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
        let v = marquages_de(&mut tx, etab, extrait, peut_arbitrer).await?;
        tx.commit().await?;
        Ok(v)
    }
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
            rang: 0,
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
}
