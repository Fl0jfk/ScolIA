//! Commande `inspecter` (lot R0) : lit un export, vérifie le schéma attendu, compte les lignes par
//! table et par collection, vérifie les empreintes. Le rapport ne contient **aucune valeur** de
//! donnée : seulement des noms de tables, de colonnes, de collections « sûres » et des nombres.

use crate::dump::{self, CollecteTables, ColonneDump, Evt};
use crate::empreintes::{self, Verdict, Verification};
use crate::s3::{self, ObjetS3};
use crate::schema::{
    self, EcartsSchema, SchemaAttendu, TABLES_DEPUIS_T0, TABLES_INVENTAIRE_T0, TABLES_SANTE,
};
use crate::{err, garde, Res};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Tables déclarées dans le code du Next.js mais jamais lues ni écrites (T0 §5.3) : si elles ne sont
/// pas vides, c'est signalé (une ancienne version du code a pu les remplir).
pub const TABLES_MORTES: &[&str] = &[
    "annee_bascule",
    "bulletin_ligne",
    "certificate_program",
    "certificate_program_attr",
    "certificate_award",
    "certificate_award_attr",
    "conseil_seance",
    "conseil_avis",
    "domain_planning_domain",
    "domain_planning_booking",
    "domain_planning_session",
    "domain_planning_signup",
    "export_academique",
    "personnel_document",
    "personnel_formation",
    "personnel_habilitation",
    "personnel_entretien",
];

#[derive(Debug, Clone)]
pub struct Options {
    pub dump: PathBuf,
    pub objets_s3: Option<PathBuf>,
    /// Fichier `SHA256SUMS` à vérifier.
    pub sommes: Option<PathBuf>,
    /// Empreinte SHA-256 attendue du dump (fichier ou dossier).
    pub empreinte_dump: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct TableRapport {
    pub lignes: u64,
    pub dans_le_schema: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CollectionRapport {
    pub enregistrements: u64,
    pub attributs: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RapportS3 {
    pub objets: u64,
    pub octets: u64,
    pub par_prefixe: BTreeMap<String, (u64, u64)>,
    pub objets_secrets_a_ne_pas_reprendre: u64,
    pub objets_sans_empreinte: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Rapport {
    pub type_de_dump: String,
    pub tables_attendues: usize,
    pub tables_inventaire_t0: usize,
    pub tables_depuis_t0: Vec<String>,
    pub tables_vues_avec_donnees: usize,
    pub lignes_total: u64,
    pub tables: BTreeMap<String, TableRapport>,
    pub collections: BTreeMap<String, CollectionRapport>,
    pub ecarts_schema: EcartsSchema,
    pub tables_mortes_non_vides: Vec<String>,
    pub tables_sante_non_vides: Vec<String>,
    pub s3: Option<RapportS3>,
    pub empreintes: Vec<Verification>,
    pub empreinte_dump_calculee: Option<String>,
    pub empreinte_dump_attendue_ok: Option<bool>,
}

impl Rapport {
    /// Vrai si le schéma est conforme, les empreintes vérifiées et aucun objet secret n'est présent.
    pub fn conforme(&self) -> bool {
        self.ecarts_schema.est_vide()
            && self
                .empreintes
                .iter()
                .all(|v| v.verdict == Verdict::Conforme)
            && self.empreinte_dump_attendue_ok != Some(false)
    }
}

/// Un nom de collection n'est cité dans le rapport que s'il ressemble à un identifiant de structure
/// (lettres minuscules, chiffres, `_`, `-`, `.`), sans longue suite de chiffres ni forme d'UUID.
pub fn nom_collection_sur(nom: &str) -> String {
    let ok = !nom.is_empty()
        && nom.len() <= 60
        && nom
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && nom
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-' | '.'))
        && !a_une_suite_de_chiffres(nom, 6)
        && !ressemble_a_un_uuid(nom);
    if ok {
        nom.to_string()
    } else {
        "(autre)".to_string()
    }
}

fn a_une_suite_de_chiffres(s: &str, n: usize) -> bool {
    let mut run = 0;
    for c in s.chars() {
        if c.is_ascii_digit() {
            run += 1;
            if run >= n {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

fn ressemble_a_un_uuid(s: &str) -> bool {
    s.len() >= 32
        && s.chars()
            .filter(|c| c.is_ascii_hexdigit() || *c == '-')
            .count()
            >= 32
}

/// Lance l'inspection.
pub fn inspecter(o: &Options) -> Res<Rapport> {
    let source = dump::ouvrir(&o.dump)?;
    let attendu = SchemaAttendu::embarque();
    let mut collecte = CollecteTables::default();
    let mut tables: BTreeMap<String, TableRapport> = BTreeMap::new();
    let mut collections: BTreeMap<String, CollectionRapport> = BTreeMap::new();
    let mut copy_colonnes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut courante: Option<(String, Option<usize>)> = None;
    let mut total = 0u64;

    dump::lire(&source, &mut |e| {
        match e {
            Evt::Ddl(l) => collecte.ligne(&l),
            Evt::CopyDebut {
                schema: sch,
                table,
                colonnes,
            } => {
                let table = dump::cle_table(&sch, &table);
                let idx_coll = colonnes.iter().position(|c| c == "collection");
                tables.entry(table.clone()).or_default();
                copy_colonnes.insert(table.clone(), colonnes);
                courante = Some((table, idx_coll));
            }
            Evt::Ligne(cells) => {
                if let Some((t, idx)) = &courante {
                    tables.entry(t.clone()).or_default().lignes += 1;
                    total += 1;
                    if t == "ent_collection_record" || t == "ent_collection_attr" {
                        if let Some(i) = idx {
                            let nom = cells
                                .get(*i)
                                .and_then(|c| c.as_deref())
                                .map(nom_collection_sur)
                                .unwrap_or_else(|| "(autre)".into());
                            let c = collections.entry(nom).or_default();
                            if t == "ent_collection_record" {
                                c.enregistrements += 1;
                            } else {
                                c.attributs += 1;
                            }
                        }
                    }
                }
            }
            Evt::CopyFin => courante = None,
        }
        Ok(())
    })?;

    // Tables vues : celles du CREATE TABLE ; à défaut, celles des COPY (dump « données seules »).
    let mut vues = collecte.tables.clone();
    for (t, cols) in &copy_colonnes {
        if !vues.iter().any(|(n, _)| n == t) {
            vues.push((
                t.clone(),
                cols.iter()
                    .map(|c| ColonneDump {
                        nom: c.clone(),
                        type_sql: "?".into(),
                    })
                    .collect(),
            ));
        }
    }
    let ecarts_schema = schema::comparer(&attendu, &vues);
    for (t, r) in tables.iter_mut() {
        r.dans_le_schema = attendu.tables.contains_key(t);
    }
    // Tables du schéma présentes mais sans bloc COPY : 0 ligne.
    for (t, _) in &vues {
        tables.entry(t.clone()).or_insert_with(|| TableRapport {
            lignes: 0,
            dans_le_schema: attendu.tables.contains_key(t),
        });
    }

    let tables_mortes_non_vides = TABLES_MORTES
        .iter()
        .filter(|t| tables.get(**t).is_some_and(|r| r.lignes > 0))
        .map(|t| (*t).to_string())
        .collect();
    let tables_sante_non_vides = TABLES_SANTE
        .iter()
        .filter(|t| tables.get(**t).is_some_and(|r| r.lignes > 0))
        .map(|t| (*t).to_string())
        .collect();

    // Objets S3.
    let s3 = match &o.objets_s3 {
        None => None,
        Some(p) => {
            garde::exiger_pas_de_connexion("--objets-s3", &p.to_string_lossy())?;
            let objets: Vec<ObjetS3> = s3::lire_liste(p)?;
            Some(RapportS3 {
                objets: objets.len() as u64,
                octets: objets.iter().map(|x| x.taille).sum(),
                par_prefixe: s3::compter(&objets),
                objets_secrets_a_ne_pas_reprendre: objets
                    .iter()
                    .filter(|x| s3::est_secret(&x.cle))
                    .count() as u64,
                objets_sans_empreinte: objets.iter().filter(|x| x.sha256.is_none()).count() as u64,
            })
        }
    };

    // Empreintes.
    let empreintes = match &o.sommes {
        None => Vec::new(),
        Some(p) => empreintes::verifier_sommes(p)?,
    };
    let (empreinte_dump_calculee, empreinte_dump_attendue_ok) = match &o.empreinte_dump {
        None => (None, None),
        Some(attendue) => {
            if attendue.len() != 64 || !attendue.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(err!(
                    "--empreinte : 64 caractères hexadécimaux attendus (SHA-256)"
                ));
            }
            let h = if o.dump.is_dir() {
                empreintes::sha256_dossier(&o.dump)?
            } else {
                empreintes::sha256_fichier(&o.dump)?
            };
            let ok = h.eq_ignore_ascii_case(attendue);
            (Some(h), Some(ok))
        }
    };

    Ok(Rapport {
        type_de_dump: match source {
            dump::Source::Texte(_) => "texte".into(),
            dump::Source::Repertoire(_) => "répertoire".into(),
        },
        tables_attendues: attendu.tables_requises().count(),
        tables_inventaire_t0: TABLES_INVENTAIRE_T0,
        tables_depuis_t0: TABLES_DEPUIS_T0.iter().map(|s| (*s).to_string()).collect(),
        tables_vues_avec_donnees: tables.values().filter(|t| t.lignes > 0).count(),
        lignes_total: total,
        tables,
        collections,
        ecarts_schema,
        tables_mortes_non_vides,
        tables_sante_non_vides,
        s3,
        empreintes,
        empreinte_dump_calculee,
        empreinte_dump_attendue_ok,
    })
}

/// Rapport lisible (français), sans aucune valeur de donnée.
pub fn texte(r: &Rapport) -> String {
    let mut s = String::new();
    s.push_str("=== Inspection de l'export ===\n");
    s.push_str(&format!("Type de dump : {}\n", r.type_de_dump));
    s.push_str(&format!(
        "Tables attendues : {} ({} de l'inventaire T0 + {} ajoutées depuis : {})\n",
        r.tables_attendues,
        r.tables_inventaire_t0,
        r.tables_depuis_t0.len(),
        r.tables_depuis_t0.join(", ")
    ));
    s.push_str(&format!(
        "Tables contenant des données : {} — lignes au total : {}\n\n",
        r.tables_vues_avec_donnees, r.lignes_total
    ));

    s.push_str("--- Schéma ---\n");
    let e = &r.ecarts_schema;
    if e.est_vide() {
        s.push_str("CONFORME : toutes les tables et colonnes attendues sont là, rien d'inconnu.\n");
    } else {
        s.push_str("ÉCART DE SCHÉMA (l'export n'est pas celui que l'outil attend) :\n");
        for (titre, l) in [
            ("tables manquantes", &e.tables_manquantes),
            ("tables inconnues", &e.tables_inconnues),
            ("colonnes manquantes", &e.colonnes_manquantes),
            ("colonnes inconnues", &e.colonnes_inconnues),
            ("types différents", &e.types_differents),
        ] {
            if !l.is_empty() {
                s.push_str(&format!("  {titre} ({}) :\n", l.len()));
                for x in l.iter().take(50) {
                    s.push_str(&format!("    - {x}\n"));
                }
                if l.len() > 50 {
                    s.push_str(&format!("    … et {} autres\n", l.len() - 50));
                }
            }
        }
    }
    s.push('\n');
    s.push_str("--- Lignes par table (tables non vides) ---\n");
    for (t, x) in &r.tables {
        if x.lignes > 0 {
            s.push_str(&format!(
                "{:>10}  {}{}\n",
                x.lignes,
                t,
                if x.dans_le_schema {
                    ""
                } else {
                    "  (hors schéma attendu)"
                }
            ));
        }
    }
    let vides = r.tables.values().filter(|t| t.lignes == 0).count();
    s.push_str(&format!("{vides:>10}  table(s) vide(s)\n\n"));

    s.push_str("--- Collections (ent_collection_*) ---\n");
    if r.collections.is_empty() {
        s.push_str("(aucune)\n");
    }
    for (c, x) in &r.collections {
        s.push_str(&format!(
            "{:>8} enregistrement(s), {:>8} attribut(s)  {}\n",
            x.enregistrements, x.attributs, c
        ));
    }
    s.push('\n');
    if !r.tables_mortes_non_vides.is_empty() {
        s.push_str(&format!(
            "ATTENTION : tables jamais utilisées par le code Next.js mais non vides : {}\n",
            r.tables_mortes_non_vides.join(", ")
        ));
    }
    if !r.tables_sante_non_vides.is_empty() {
        s.push_str(&format!(
            "Santé : tables non vides ({}). Elles seront EXCLUES de l'export anonymisé.\n",
            r.tables_sante_non_vides.join(", ")
        ));
    }
    if let Some(x) = &r.s3 {
        s.push_str("\n--- Objets S3 (liste fournie) ---\n");
        s.push_str(&format!("{} objet(s), {} octet(s)\n", x.objets, x.octets));
        for (p, (n, o)) in &x.par_prefixe {
            s.push_str(&format!("{n:>8} objet(s) {o:>14} octet(s)  {p}/\n"));
        }
        if x.objets_secrets_a_ne_pas_reprendre > 0 {
            s.push_str(&format!(
                "ALERTE : {} objet(s) de type secret ou registre (préfixe tenants/, secrets, clés) : à ne jamais reprendre.\n",
                x.objets_secrets_a_ne_pas_reprendre
            ));
        }
        if x.objets_sans_empreinte > 0 {
            s.push_str(&format!(
                "{} objet(s) sans empreinte SHA-256 dans la liste.\n",
                x.objets_sans_empreinte
            ));
        }
    }
    if !r.empreintes.is_empty() || r.empreinte_dump_attendue_ok.is_some() {
        s.push_str("\n--- Empreintes SHA-256 ---\n");
        for v in &r.empreintes {
            let m = match v.verdict {
                Verdict::Conforme => "conforme",
                Verdict::Different => "DIFFÉRENTE (fichier modifié ou corrompu)",
                Verdict::Manquant => "FICHIER MANQUANT",
            };
            s.push_str(&format!("  {} : {m}\n", v.fichier));
        }
        match r.empreinte_dump_attendue_ok {
            Some(true) => s.push_str("  empreinte du dump : conforme à celle fournie\n"),
            Some(false) => s.push_str("  empreinte du dump : DIFFÉRENTE de celle fournie\n"),
            None => {}
        }
    }
    s.push_str(&format!(
        "\nRésultat : {}\n",
        if r.conforme() {
            "CONFORME"
        } else {
            "NON CONFORME"
        }
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_de_collection_sures() {
        assert_eq!(nom_collection_sur("internat__rooms"), "internat__rooms");
        assert_eq!(
            nom_collection_sur("rh__onboarding__tokens"),
            "rh__onboarding__tokens"
        );
        assert_eq!(nom_collection_sur("documents__12345678"), "(autre)");
        assert_eq!(nom_collection_sur("Dupont Jean"), "(autre)");
        assert_eq!(nom_collection_sur("a@b.c"), "(autre)");
        assert_eq!(
            nom_collection_sur("123e4567-e89b-12d3-a456-426614174000"),
            "(autre)"
        );
        assert_eq!(nom_collection_sur(""), "(autre)");
    }
}
