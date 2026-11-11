//! Import d'élèves en lot avec prévisualisation et rejets ligne à ligne
//! (socle 2, lot S2-3 ; IOP-01 au format documenté, IOP-04 ; A18).
//!
//! Les formats nationaux (AAF pour le public, exports BEE) ne sont pas encore
//! disponibles (`INTEROPERABILITE_NATIONALE.md` : spécifications sur demande) :
//! ce moteur lit le format **`scolia_eleves_v1`** documenté dans
//! `docs/SOCLE2_DOMAINES.md` §6.3 ; un lecteur AAF/BEE n'aura qu'à produire les
//! mêmes lignes.
//!
//! Règles : prévisualiser n'écrit aucun élève ; appliquer crée seulement les
//! lignes « à créer » ; rapprochement par INE puis par identité (nom, prénom,
//! date de naissance) ; un élève déjà présent n'est **jamais modifié** (ligne
//! « existant ») ; chaque rejet porte ses motifs et se traite à part (ignorer
//! ou corriger puis créer).

// Lignes SQL lues en tuples (requêtes ponctuelles) : types longs assumés.
#![allow(clippy::type_complexity)]

use chrono::{NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::inscriptions::{ImportBody, LigneImportDto, LotImportDto, TraiterLigneBody};
use serde_json::{json, Map, Value};
use sqlx::PgConnection;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

use crate::dossier_personnes;
use crate::eleve_ports;
use crate::outbox::Publier;
use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};

pub const FORMAT_ELEVES_V1: &str = "scolia_eleves_v1";
/// Colonnes reconnues (en-tête obligatoire, ordre libre, `;` comme séparateur).
pub const COLONNES: [&str; 17] = [
    "nom",
    "prenom",
    "date_naissance",
    "sexe",
    "ine",
    "classe",
    "regime",
    "resp1_nom",
    "resp1_prenom",
    "resp1_email",
    "resp1_telephone",
    "resp1_lien",
    "resp2_nom",
    "resp2_prenom",
    "resp2_email",
    "resp2_telephone",
    "resp2_lien",
];
pub const LIGNES_MAX: usize = 5000;
/// Longueur maximale d'une valeur de colonne (contrainte `ent_import_ligne_valeurs_chk`, D-036).
pub const VALEUR_MAX: usize = 4000;

/// SQL d'écriture d'une ligne dans les colonnes typées de `ent_import_ligne` (D-036) : la ligne
/// (clés = [`COLONNES`], valeurs textuelles) transite en JSON de paramètre et est éclatée par
/// `jsonb_to_record` ; rien n'est stocké en JSON.
fn definition_colonnes() -> String {
    COLONNES.iter().map(|c| format!("{c} text")).collect::<Vec<_>>().join(", ")
}

fn sql_insertion_ligne() -> String {
    format!(
        "INSERT INTO ent_import_ligne (id, etablissement_id, lot_id, numero, {cols}, statut, motifs, eleve_id)
         SELECT $1,$2,$3,$4, {dcols}, $6,$7,$8 FROM jsonb_to_record($5::jsonb) AS d({def})",
        cols = COLONNES.join(", "),
        dcols = COLONNES.iter().map(|c| format!("d.{c}")).collect::<Vec<_>>().join(", "),
        def = definition_colonnes()
    )
}

fn sql_maj_ligne() -> String {
    format!(
        "UPDATE ent_import_ligne AS l SET statut = $2, motifs = $3, eleve_id = $4, {sets},
                traite_par = $6, traite_le = now()
           FROM jsonb_to_record($5::jsonb) AS d({def}) WHERE l.id = $1 AND l.etablissement_id = $7",
        sets = COLONNES.iter().map(|c| format!("{c} = d.{c}")).collect::<Vec<_>>().join(", "),
        def = definition_colonnes()
    )
}
pub const TAILLE_MAX: usize = 2 * 1024 * 1024;

fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("imports", err))
}

fn invalide(msg: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(msg)
}

/// Découpe une ligne CSV (`;`), guillemets doubles et `""` gérés.
pub fn decouper_ligne(l: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut guillemets = false;
    let mut it = l.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if guillemets && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' => guillemets = !guillemets,
            ';' if !guillemets => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).collect()
}

/// Lignes brutes : (numéro de ligne du fichier, colonnes → valeurs non vides).
pub fn lire_csv(contenu: &str) -> Result<Vec<(i32, BTreeMap<String, String>)>, EntDbError> {
    if contenu.len() > TAILLE_MAX {
        return Err(invalide("fichier trop volumineux (2 Mo au plus)"));
    }
    let contenu = contenu.trim_start_matches('\u{feff}');
    let mut lignes = contenu
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty());
    let Some((_, entete)) = lignes.next() else {
        return Err(invalide("fichier vide"));
    };
    let entete: Vec<String> = decouper_ligne(entete)
        .into_iter()
        .map(|c| c.to_lowercase())
        .collect();
    for c in &entete {
        if !COLONNES.contains(&c.as_str()) {
            return Err(invalide(
                "colonne inconnue dans l'en-tête (voir le format scolia_eleves_v1)",
            ));
        }
    }
    for obligatoire in ["nom", "prenom", "date_naissance"] {
        if !entete.iter().any(|c| c == obligatoire) {
            return Err(invalide(
                "en-tête incomplet : nom, prenom et date_naissance sont obligatoires",
            ));
        }
    }
    let mut out = Vec::new();
    for (i, l) in lignes {
        if out.len() >= LIGNES_MAX {
            return Err(invalide("trop de lignes (5000 au plus)"));
        }
        let valeurs = decouper_ligne(l);
        let mut m = BTreeMap::new();
        for (c, v) in entete.iter().zip(valeurs) {
            if v.chars().count() > VALEUR_MAX {
                return Err(invalide("valeur trop longue (4000 caractères au plus)"));
            }
            if !v.is_empty() {
                m.insert(c.clone(), v);
            }
        }
        out.push((i32::try_from(i + 1).unwrap_or(i32::MAX), m));
    }
    Ok(out)
}

/// Date `JJ/MM/AAAA` ou `AAAA-MM-JJ`.
pub fn lire_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%d/%m/%Y")
        .or_else(|_| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .ok()
}

/// Ligne validée (sans base) : motifs de rejet propres à la ligne.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LigneValide {
    pub nom: String,
    pub prenom: String,
    pub date_naissance: Option<NaiveDate>,
    pub sexe: Option<String>,
    pub ine: Option<String>,
    pub classe: Option<String>,
    pub regime: Option<String>,
    pub responsables: Vec<Resp>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resp {
    pub nom: String,
    pub prenom: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub lien: String,
}

pub fn valider_ligne(m: &BTreeMap<String, String>) -> (LigneValide, Vec<String>) {
    let mut motifs = Vec::new();
    let g = |k: &str| m.get(k).cloned();
    let nom = g("nom").unwrap_or_default();
    let prenom = g("prenom").unwrap_or_default();
    if nom.is_empty() || nom.chars().count() > 120 {
        motifs.push("nom manquant ou trop long".to_string());
    }
    if prenom.is_empty() || prenom.chars().count() > 120 {
        motifs.push("prénom manquant ou trop long".to_string());
    }
    let date_naissance = match g("date_naissance") {
        None => {
            motifs.push("date de naissance manquante".to_string());
            None
        }
        Some(d) => {
            let v = lire_date(&d);
            if v.is_none() {
                motifs.push("date de naissance illisible (JJ/MM/AAAA)".to_string());
            }
            v
        }
    };
    let sexe = match g("sexe").map(|s| s.to_uppercase()) {
        None => None,
        Some(s) if s == "F" || s == "M" => Some(s),
        Some(_) => {
            motifs.push("sexe : F ou M".to_string());
            None
        }
    };
    let ine = match g("ine") {
        None => None,
        Some(i) => match domain_eleve::normalize_ine(&i) {
            Ok(i) => Some(i),
            Err(_) => {
                motifs.push("INE invalide".to_string());
                None
            }
        },
    };
    let regime = match g("regime").map(|r| r.to_lowercase()) {
        None => None,
        Some(r) if ["externe", "demi_pension", "interne"].contains(&r.as_str()) => Some(r),
        Some(_) => {
            motifs.push("régime inconnu (externe, demi_pension, interne)".to_string());
            None
        }
    };
    let mut responsables = Vec::new();
    for n in ["resp1", "resp2"] {
        let Some(rnom) = g(&format!("{n}_nom")) else {
            if [format!("{n}_email"), format!("{n}_prenom")]
                .iter()
                .any(|k| m.contains_key(k))
            {
                motifs.push(format!("{n} : nom manquant"));
            }
            continue;
        };
        let email = g(&format!("{n}_email")).map(|e| e.to_lowercase());
        if let Some(e) = &email {
            if crate::inscriptions::normaliser_email(e).is_err() {
                motifs.push(format!("{n} : e-mail mal formé"));
            }
        }
        let lien = g(&format!("{n}_lien"))
            .map(|l| l.to_lowercase())
            .unwrap_or_else(|| "autre".into());
        if !["mere", "pere", "tuteur", "autre"].contains(&lien.as_str()) {
            motifs.push(format!("{n} : lien inconnu (mere, pere, tuteur, autre)"));
        }
        responsables.push(Resp {
            nom: rnom,
            prenom: g(&format!("{n}_prenom")),
            email,
            telephone: g(&format!("{n}_telephone")),
            lien,
        });
    }
    (
        LigneValide {
            nom,
            prenom,
            date_naissance,
            sexe,
            ine,
            classe: g("classe"),
            regime,
            responsables,
        },
        motifs,
    )
}

fn cle_identite(l: &LigneValide) -> String {
    format!(
        "{}|{}|{}",
        l.nom.to_lowercase(),
        l.prenom.to_lowercase(),
        l.date_naissance.map(|d| d.to_string()).unwrap_or_default()
    )
}

#[derive(sqlx::FromRow)]
struct LigneRow {
    id: Uuid,
    numero: i32,
    donnees: Value,
    statut: String,
    motifs: Vec<String>,
    eleve_id: Option<Uuid>,
}

fn donnees_map(v: &Value) -> BTreeMap<String, String> {
    v.as_object()
        .map(|o| {
            o.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

impl PgEnt {
    /// Classes de l'année courante (code → id).
    async fn classes_courantes(&self, etab: Uuid) -> Result<HashMap<String, Uuid>, EntDbError> {
        let rows: Vec<(String, Uuid)> = sqlx::query_as(
            "SELECT lower(c.code), c.id FROM ent_classe c
               JOIN ent_annee_scolaire a ON a.id = c.annee_id AND a.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND a.courante AND c.archivee_le IS NULL",
        )
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows.into_iter().collect())
    }

    /// Statut d'une ligne contre la base : rejet, existant ou à créer.
    async fn classer_ligne(
        &self,
        etab: Uuid,
        l: &LigneValide,
        mut motifs: Vec<String>,
        classes: &HashMap<String, Uuid>,
    ) -> Result<(String, Vec<String>, Option<Uuid>), EntDbError> {
        if let Some(c) = &l.classe {
            if !classes.contains_key(&c.to_lowercase()) {
                motifs.push("classe inconnue dans l'année courante".to_string());
            }
        }
        if !motifs.is_empty() {
            return Ok(("rejet".into(), motifs, None));
        }
        if let Some(ine) = &l.ine {
            let r: Option<(Uuid, String, String, Option<NaiveDate>)> = sqlx::query_as(
                "SELECT id, nom, prenom, date_naissance FROM ent_eleve WHERE etablissement_id = $1 AND ine = $2",
            )
            .bind(etab)
            .bind(ine)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?;
            if let Some((id, nom, prenom, naissance)) = r {
                let meme = nom.eq_ignore_ascii_case(&l.nom)
                    && prenom.eq_ignore_ascii_case(&l.prenom)
                    && (naissance.is_none() || naissance == l.date_naissance);
                return Ok(if meme {
                    ("existant".into(), Vec::new(), Some(id))
                } else {
                    (
                        "rejet".into(),
                        vec!["INE déjà attribué à un autre élève de l'établissement".to_string()],
                        None,
                    )
                });
            }
        }
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND lower(nom) = lower($2)
                AND lower(prenom) = lower($3) AND date_naissance IS NOT DISTINCT FROM $4 LIMIT 1",
        )
        .bind(etab)
        .bind(&l.nom)
        .bind(&l.prenom)
        .bind(l.date_naissance)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(match r {
            Some((id,)) => ("existant".into(), Vec::new(), Some(id)),
            None => ("a_creer".into(), Vec::new(), None),
        })
    }

    /// IOP-01 : prévisualiser un fichier (aucun élève écrit).
    pub async fn previsualiser_import_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ImportBody,
    ) -> Result<LotImportDto, EntDbError> {
        self.exiger_import(etab, actor, role).await?;
        let nom = b.nom_fichier.trim();
        if nom.is_empty() || nom.chars().count() > 200 {
            return Err(invalide(
                "nom de fichier obligatoire (200 caractères au plus)",
            ));
        }
        let lignes = lire_csv(&b.contenu)?;
        let classes = self.classes_courantes(etab).await?;
        let lot_id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_import_lot (id, etablissement_id, format, nom_fichier, cree_par)
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(lot_id)
        .bind(etab)
        .bind(FORMAT_ELEVES_V1)
        .bind(nom)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let mut vus: HashMap<String, i32> = HashMap::new();
        for (numero, m) in lignes {
            let (l, mut motifs) = valider_ligne(&m);
            if motifs.is_empty() {
                let cle = l.ine.clone().unwrap_or_else(|| cle_identite(&l));
                if let Some(n) = vus.get(&cle) {
                    motifs.push(format!("doublon dans le fichier (ligne {n})"));
                } else {
                    vus.insert(cle, numero);
                }
            }
            let (statut, motifs, eleve_id) = self.classer_ligne(etab, &l, motifs, &classes).await?;
            let donnees: Map<String, Value> =
                m.into_iter().map(|(k, v)| (k, Value::String(v))).collect();
            sqlx::query(&sql_insertion_ligne())
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(lot_id)
            .bind(numero)
            .bind(Value::Object(donnees))
            .bind(&statut)
            .bind(&motifs)
            .bind(eleve_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        self.lot_import(etab, lot_id).await
    }

    async fn exiger_import(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<(), EntDbError> {
        if self
            .a_action_etablissement(etab, actor, role, Action::ImportsAppliquer)
            .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    async fn lot_import(&self, etab: Uuid, lot_id: Uuid) -> Result<LotImportDto, EntDbError> {
        let lot: Option<(String, String, String)> = sqlx::query_as(
            "SELECT format, nom_fichier, statut FROM ent_import_lot WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(lot_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let (format, nom_fichier, statut) = lot.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let lignes = sqlx::query_as::<_, LigneRow>(
            "SELECT id, numero, ent_import_ligne_donnees(l) AS donnees, statut, motifs, eleve_id
               FROM ent_import_ligne l
              WHERE etablissement_id = $1 AND lot_id = $2 ORDER BY numero",
        )
        .bind(etab)
        .bind(lot_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let compte = |s: &str| {
            i32::try_from(lignes.iter().filter(|l| l.statut == s).count()).unwrap_or(i32::MAX)
        };
        Ok(LotImportDto {
            id: lot_id,
            format,
            nom_fichier,
            statut,
            nb_lignes: i32::try_from(lignes.len()).unwrap_or(i32::MAX),
            nb_a_creer: compte("a_creer"),
            nb_existantes: compte("existant"),
            nb_rejets: compte("rejet"),
            nb_creees: compte("creee"),
            lignes: lignes
                .into_iter()
                .map(|l| {
                    let m = donnees_map(&l.donnees);
                    LigneImportDto {
                        id: l.id,
                        numero: l.numero,
                        statut: l.statut,
                        motifs: l.motifs,
                        eleve_nom: m.get("nom").cloned().unwrap_or_default(),
                        eleve_prenom: m.get("prenom").cloned().unwrap_or_default(),
                        eleve_id: l.eleve_id,
                    }
                })
                .collect(),
        })
    }

    pub async fn lot_import_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        lot_id: Uuid,
    ) -> Result<LotImportDto, EntDbError> {
        self.exiger_import(etab, actor, role).await?;
        self.lot_import(etab, lot_id).await
    }

    /// Crée l'élève d'une ligne valide (personnes, foyer, liens, affectation) **par les commandes
    /// des autres socles** : `eleve_ports::creer_eleve_tx` (socle 2), `integrer_declaration_tx`
    /// (socle 4) et l'affectation du référentiel. Ce module n'écrit lui-même dans aucune de leurs
    /// tables (test de garde `tests_garde_inscriptions`).
    ///
    /// Rapprochement : une fiche déjà créée **par un import** pour le même e-mail, et qui n'a pas
    /// de compte, est reprise (fratrie dans un même fichier) avec son foyer ; toute autre
    /// correspondance (compte existant, fiche saisie ailleurs, même nom seulement) donne une
    /// **nouvelle** fiche — jamais un lien implicite vers une personne existante (§2.2 n° 1).
    async fn creer_depuis_ligne(
        conn: &mut PgConnection,
        etab: Uuid,
        l: &LigneValide,
        classe_id: Option<Uuid>,
        lot_id: Uuid,
        actor: Uuid,
    ) -> Result<Uuid, EntDbError> {
        let eleve_id = eleve_ports::creer_eleve_tx(
            conn,
            etab,
            &eleve_ports::NouvelEleve {
                nom: &l.nom,
                prenom: &l.prenom,
                ine: l.ine.as_deref(),
                sexe: l.sexe.as_deref(),
                date_naissance: l.date_naissance,
                regime: l.regime.as_deref(),
                statut: eleve_ports::StatutInitial::Actif,
            },
            eleve_ports::OrigineFait {
                source: eleve_ports::SourceEleve::Import,
                source_ref: Some(lot_id),
            },
            actor,
        )
        .await?;
        if let Some(c) = classe_id {
            eleve_ports::affecter_arrivee_en_cours_d_annee_tx(conn, etab, eleve_id, c, "lot")
                .await?;
        }
        let mut declarations = Vec::new();
        let mut resolutions = Vec::new();
        for (i, r) in l.responsables.iter().enumerate() {
            let cle = Uuid::new_v4();
            let d = dossier_personnes::DeclarationResponsable {
                cle,
                rang: (i + 1) as i16,
                nom: r.nom.clone(),
                prenom: r.prenom.clone(),
                civilite: None,
                email: r.email.clone(),
                telephone: r.telephone.clone().filter(|t| t.len() <= 40),
                adresse: None,
                lien: r.lien.clone(),
                // Un seul foyer par ligne : le premier responsable le crée, les autres le partagent.
                meme_foyer: true,
                autorite_parentale: true,
                droits: dossier_personnes::DroitsDeclares {
                    notes: true,
                    vs: true,
                    justifier: true,
                    sorties: false,
                },
                payeur: i == 0,
                part_paiement: if i == 0 { Some(100) } else { None },
                reserve: Default::default(),
            };
            let mut reprise: Option<(Uuid, Option<Uuid>)> = None;
            for c in dossier_personnes::correspondances_tx(conn, etab, &d).await? {
                if c.raison != "email" || c.a_un_compte {
                    continue;
                }
                let (de_import,): (bool,) = sqlx::query_as(
                    "SELECT source = 'reprise' FROM ent_personne WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(c.personne_id)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?;
                if de_import {
                    let foyer = match c.foyers.as_slice() {
                        [f] => Some(f.foyer_id),
                        _ => None,
                    };
                    reprise = Some((c.personne_id, foyer));
                    break;
                }
            }
            resolutions.push(match reprise {
                Some((p, Some(f))) => dossier_personnes::Resolution {
                    cle,
                    personne: Some(dossier_personnes::ResolutionPersonne::MemePersonne(p)),
                    foyer: Some(dossier_personnes::ResolutionFoyer::MemeFoyer(f)),
                },
                Some((p, None)) => dossier_personnes::Resolution {
                    cle,
                    personne: Some(dossier_personnes::ResolutionPersonne::MemePersonne(p)),
                    // Fiche reprise sans foyer unique : un nouveau foyer, explicite.
                    foyer: Some(dossier_personnes::ResolutionFoyer::NouveauFoyer),
                },
                None => dossier_personnes::Resolution {
                    cle,
                    personne: Some(dossier_personnes::ResolutionPersonne::NouvellePersonne),
                    foyer: None,
                },
            });
            declarations.push(d);
        }
        if !declarations.is_empty() {
            dossier_personnes::integrer_declaration_tx(
                conn,
                etab,
                eleve_id,
                &declarations,
                &resolutions,
                dossier_personnes::SourceLien::Import,
                actor,
            )
            .await?;
        }
        Ok(eleve_id)
    }

    /// Appliquer un lot prévisualisé : crée les lignes « à créer » (revérifiées),
    /// dans une seule transaction ; les rejets restent à traiter.
    pub async fn appliquer_import_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        lot_id: Uuid,
    ) -> Result<LotImportDto, EntDbError> {
        self.exiger_import(etab, actor, role).await?;
        let classes = self.classes_courantes(etab).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let statut: Option<(String,)> = sqlx::query_as(
            "SELECT statut FROM ent_import_lot WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(lot_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        match statut {
            None => return Err(EntDbError::Domain(DomainError::NotFound)),
            Some((s,)) if s != "previsualise" => {
                return Err(EntDbError::ConflitDossier("lot déjà appliqué ou annulé"))
            }
            _ => {}
        }
        let lignes = sqlx::query_as::<_, LigneRow>(
            "SELECT id, numero, ent_import_ligne_donnees(l) AS donnees, statut, motifs, eleve_id
               FROM ent_import_ligne l
              WHERE etablissement_id = $1 AND lot_id = $2 AND statut = 'a_creer' ORDER BY numero",
        )
        .bind(etab)
        .bind(lot_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut creees = 0_i64;
        for lr in lignes {
            let (l, motifs) = valider_ligne(&donnees_map(&lr.donnees));
            // Revérification au moment d'appliquer (un élève a pu être créé entre-temps).
            let (statut, motifs, existant) = self.classer_ligne(etab, &l, motifs, &classes).await?;
            if statut != "a_creer" {
                sqlx::query("UPDATE ent_import_ligne SET statut = $2, motifs = $3, eleve_id = $4 WHERE id = $1 AND etablissement_id = $5")
                    .bind(lr.id)
                    .bind(&statut)
                    .bind(&motifs)
                    .bind(existant)
                    .bind(etab)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                continue;
            }
            let classe = l
                .classe
                .as_ref()
                .and_then(|c| classes.get(&c.to_lowercase()).copied());
            let eleve_id =
                Self::creer_depuis_ligne(&mut tx, etab, &l, classe, lot_id, actor).await?;
            sqlx::query(
                "UPDATE ent_import_ligne SET statut = 'creee', eleve_id = $2, traite_par = $3, traite_le = now()
                  WHERE id = $1 AND etablissement_id = $4",
            )
            .bind(lr.id)
            .bind(eleve_id)
            .bind(actor)
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            creees += 1;
        }
        sqlx::query(
            "UPDATE ent_import_lot SET statut = 'applique', applique_par = $3, applique_le = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(lot_id)
        .bind(actor)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::IMPORT_LOT_APPLIQUE.into(),
            aggregate: AggregateKind::Import,
            aggregate_id: lot_id,
            eleve_id: None,
            payload: json!({ "lot_id": lot_id, "format": FORMAT_ELEVES_V1, "creees": creees }),
            actor_user_id: Some(actor.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        self.lot_import(etab, lot_id).await
    }

    /// IOP-04 : traiter un rejet. `ignorer` : la ligne est écartée ; `corriger` :
    /// les valeurs données remplacent celles du fichier, la ligne est revalidée
    /// et, si le lot est déjà appliqué, l'élève est créé aussitôt.
    pub async fn traiter_ligne_import_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        ligne_id: Uuid,
        b: &TraiterLigneBody,
    ) -> Result<LigneImportDto, EntDbError> {
        self.exiger_import(etab, actor, role).await?;
        if b.corrections.values().any(|v| v.chars().count() > VALEUR_MAX) {
            return Err(invalide("valeur trop longue (4000 caractères au plus)"));
        }
        for k in b.corrections.keys() {
            if !COLONNES.contains(&k.as_str()) {
                return Err(invalide("colonne inconnue dans les corrections"));
            }
        }
        let classes = self.classes_courantes(etab).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let r: Option<(Uuid, Value, String, String)> = sqlx::query_as(
            "SELECT l.lot_id, ent_import_ligne_donnees(l), l.statut, t.statut FROM ent_import_ligne l
               JOIN ent_import_lot t ON t.id = l.lot_id AND t.etablissement_id = l.etablissement_id
              WHERE l.etablissement_id = $1 AND l.id = $2 FOR UPDATE OF l",
        )
        .bind(etab)
        .bind(ligne_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (lot_id, donnees, statut, statut_lot) =
            r.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if statut != "rejet" {
            return Err(EntDbError::ConflitDossier(
                "seule une ligne rejetée se traite",
            ));
        }
        let (nouveau, motifs, eleve_id, m) = match b.action.as_str() {
            "ignorer" => (
                "ignoree".to_string(),
                Vec::new(),
                None,
                donnees_map(&donnees),
            ),
            "corriger" => {
                let mut m = donnees_map(&donnees);
                for (k, v) in &b.corrections {
                    let v = v.trim();
                    if v.is_empty() {
                        m.remove(k);
                    } else {
                        m.insert(k.clone(), v.to_string());
                    }
                }
                let (l, motifs) = valider_ligne(&m);
                let (statut, motifs, existant) =
                    self.classer_ligne(etab, &l, motifs, &classes).await?;
                if statut == "a_creer" && statut_lot == "applique" {
                    let classe = l
                        .classe
                        .as_ref()
                        .and_then(|c| classes.get(&c.to_lowercase()).copied());
                    let e =
                        Self::creer_depuis_ligne(&mut tx, etab, &l, classe, lot_id, actor).await?;
                    ("creee".to_string(), Vec::new(), Some(e), m)
                } else {
                    (statut, motifs, existant, m)
                }
            }
            _ => return Err(invalide("action : ignorer ou corriger")),
        };
        let donnees: Map<String, Value> =
            m.into_iter().map(|(k, v)| (k, Value::String(v))).collect();
        sqlx::query(&sql_maj_ligne())
        .bind(ligne_id)
        .bind(&nouveau)
        .bind(&motifs)
        .bind(eleve_id)
        .bind(Value::Object(donnees.clone()))
        .bind(actor)
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::IMPORT_REJET_TRAITE.into(),
            aggregate: AggregateKind::Import,
            aggregate_id: lot_id,
            eleve_id,
            payload: json!({ "lot_id": lot_id, "ligne_id": ligne_id, "action": b.action, "statut": nouveau }),
            actor_user_id: Some(actor.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        let numero: (i32,) = sqlx::query_as("SELECT numero FROM ent_import_ligne WHERE id = $1 AND etablissement_id = $2")
            .bind(ligne_id)
            .bind(etab)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
        let g = |k: &str| {
            donnees
                .get(k)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        };
        Ok(LigneImportDto {
            id: ligne_id,
            numero: numero.0,
            statut: nouveau,
            motifs,
            eleve_nom: g("nom"),
            eleve_prenom: g("prenom"),
            eleve_id,
        })
    }
}

#[cfg(test)]
mod tests_unitaires {
    use super::*;

    #[test]
    fn decoupe_guillemets() {
        assert_eq!(
            decouper_ligne(r#"a;"b;c";"d""e""#),
            vec!["a", "b;c", "d\"e"]
        );
        assert_eq!(decouper_ligne(" x ; ;y"), vec!["x", "", "y"]);
    }

    #[test]
    fn entete_et_lignes() {
        let csv = "\u{feff}nom;prenom;date_naissance;classe\nDupont;Léa;03/02/2014;6A\n\nMartin;Tom;2013-05-01;\n";
        let l = lire_csv(csv).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].0, 2);
        assert_eq!(l[1].0, 4);
        assert_eq!(l[0].1.get("classe").map(String::as_str), Some("6A"));
        assert!(!l[1].1.contains_key("classe"));
        assert!(
            lire_csv("nom;prenom\nA;B").is_err(),
            "date_naissance obligatoire"
        );
        assert!(lire_csv("nom;prenom;date_naissance;inconnue\n").is_err());
        assert!(lire_csv("").is_err());
    }

    #[test]
    fn validation_ligne() {
        let mut m = BTreeMap::new();
        m.insert("nom".into(), "Dupont".into());
        m.insert("prenom".into(), "Léa".into());
        m.insert("date_naissance".into(), "31/02/2014".into());
        m.insert("sexe".into(), "x".into());
        m.insert("resp1_email".into(), "a@b.fr".into());
        let (_, motifs) = valider_ligne(&m);
        assert_eq!(motifs.len(), 3, "{motifs:?}");
        m.insert("date_naissance".into(), "28/02/2014".into());
        m.insert("sexe".into(), "f".into());
        m.insert("resp1_nom".into(), "Dupont".into());
        let (l, motifs) = valider_ligne(&m);
        assert!(motifs.is_empty(), "{motifs:?}");
        assert_eq!(l.sexe.as_deref(), Some("F"));
        assert_eq!(l.responsables.len(), 1);
        assert_eq!(l.responsables[0].lien, "autre");
    }
}
