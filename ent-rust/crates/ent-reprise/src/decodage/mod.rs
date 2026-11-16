//! Décodage **strict** de l'export du Next.js (lot R2) : tables du dump, collections clé-valeur, JSON des
//! colonnes `jsonb`, JSON stockés dans S3.
//!
//! Règle : tout champ inconnu, tout type inattendu, toute valeur hors domaine est une **erreur rapportée**
//! (par table et par collection : lues, comprises, refusées avec la raison, ignorées avec la raison), jamais
//! ignorée en silence. Le rapport ne contient aucune valeur de donnée : codes, noms de colonnes et de
//! tables, numéros de ligne, identifiants techniques sûrs.
//!
//! Ce module n'a ni connexion ni écriture : il produit un [`Decodage`] en mémoire. Le chargement en zone
//! de reprise est dans `ent-db`, la simulation dans [`crate::simulation`].

pub mod attr;
pub mod cellule;
pub mod forme;
pub mod s3json;

use crate::dump::{self, CollecteTables, Evt};
use crate::inspection::nom_collection_sur;
use crate::perimetre::{disposition_collection, Disposition, Perimetre};
use crate::s3::{self, ObjetS3};
use crate::schema::{ColSchema, ListeBlanche, Regle, SchemaAttendu};
use crate::{garde, Res};
use cellule::{decoder, Refus, Valeur};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

/// Détail de refus conservé par objet (les comptes par code restent exacts au-delà).
pub const REFUS_DETAILLES_PAR_OBJET: usize = 50;

#[derive(Debug, Clone)]
pub struct OptionsDecodage {
    pub dump: PathBuf,
    pub objets_s3: Option<PathBuf>,
    /// Dossier des **contenus** S3 (clé = chemin relatif) ; sans lui, les JSON S3 sont comptés « non lus ».
    pub contenus_s3: Option<PathBuf>,
    /// Instant de référence (expiration des dépôts en attente) : fourni, jamais lu de l'horloge.
    pub maintenant: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ObjetRapport {
    pub disposition: String,
    pub lues: u64,
    pub comprises: u64,
    pub refusees: u64,
    pub ignorees: u64,
    pub refus_par_code: BTreeMap<String, u64>,
    pub ignorees_par_raison: BTreeMap<String, u64>,
}

impl ObjetRapport {
    /// `lues = comprises + refusées + ignorées`.
    pub fn invariant(&self) -> bool {
        self.lues == self.comprises + self.refusees + self.ignorees
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RefusDetail {
    pub objet: String,
    pub source_ligne: u64,
    pub source_id: Option<String>,
    pub code: String,
    pub colonne: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RapportDecodage {
    pub tables: BTreeMap<String, ObjetRapport>,
    pub collections: BTreeMap<String, ObjetRapport>,
    pub s3: BTreeMap<String, ObjetRapport>,
    /// Écarts de schéma : table, colonne inconnue ou manquante (une entrée par écart).
    pub ecarts_schema: Vec<String>,
    /// Colonnes JSON dont le contenu non vide a été refusé faute de catalogue.
    pub colonnes_a_cataloguer: Vec<String>,
    pub documents_attr: u64,
    pub secrets_ecartes: BTreeMap<String, u64>,
    pub refus: Vec<RefusDetail>,
    pub refus_non_detailles: u64,
}

impl RapportDecodage {
    pub fn total_refusees(&self) -> u64 {
        self.tables.values().map(|o| o.refusees).sum::<u64>()
            + self.s3.values().map(|o| o.refusees).sum::<u64>()
    }

    /// Rien de refusé, aucun écart de schéma.
    pub fn conforme(&self) -> bool {
        self.total_refusees() == 0 && self.ecarts_schema.is_empty()
    }

    pub fn invariants_ok(&self) -> bool {
        self.tables
            .values()
            .chain(self.collections.values())
            .chain(self.s3.values())
            .all(ObjetRapport::invariant)
    }
}

/// Ligne comprise, prête pour la zone.
#[derive(Debug, Clone)]
pub struct LigneAcceptee {
    /// Table source, ou `collection:<nom>` pour une ligne de collection.
    pub objet: String,
    pub disposition: Disposition,
    pub domaine: String,
    pub source_ligne: u64,
    pub cle: String,
    /// SHA-256 (hex) du contenu de la ligne : détecte « mis à jour » à la simulation suivante.
    pub empreinte: String,
    /// Une cellule par colonne de zone de la table, dans l'ordre de [`TableChargee::colonnes`].
    pub cellules: Vec<Option<String>>,
    /// Type de la feuille d'un `*_attr`.
    pub type_feuille: Option<attr::TypeFeuille>,
}

#[derive(Debug, Clone, Default)]
pub struct TableChargee {
    pub colonnes: Vec<String>,
    pub lignes: Vec<LigneAcceptee>,
}

/// Objet S3 classé.
#[derive(Debug, Clone)]
pub struct ObjetS3Classe {
    pub objet: ObjetS3,
    pub nature: s3json::NatureS3,
}

/// Document S3 décodé (validé) : le JSON reste en mémoire pour la zone typée.
#[derive(Debug, Clone)]
pub struct DocumentS3 {
    pub cle: String,
    pub nature: s3json::NatureS3,
    pub valeur: serde_json::Value,
    pub expire: bool,
}

#[derive(Debug, Default)]
pub struct Decodage {
    pub tables: BTreeMap<String, TableChargee>,
    pub objets_s3: Vec<ObjetS3Classe>,
    pub documents_s3: Vec<DocumentS3>,
    pub rapport: RapportDecodage,
}

struct ColPlan {
    nom: String,
    type_sql: String,
    nullable: bool,
    secret: bool,
    forme: Option<forme::Forme>,
}

struct Plan {
    table: String,
    disposition: Disposition,
    domaine: String,
    cols: Vec<ColPlan>,
    cle_idx: Vec<usize>,
    attr: Option<(usize, usize, Vec<usize>)>,
    coll_idx: Option<usize>,
    /// Écart de schéma : les lignes sont refusées sans être décodées.
    ecart: bool,
    vus: HashSet<String>,
    docs: attr::Documents,
}

/// Identifiant source citable : forme sûre seulement.
fn id_sur(s: &str) -> Option<String> {
    let ok = !s.is_empty()
        && s.len() <= 200
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.' | ':'));
    ok.then(|| s.to_string())
}

fn empreinte(cells: &[Option<String>]) -> String {
    let mut h = Sha256::new();
    for c in cells {
        match c {
            Some(v) => {
                h.update([1u8]);
                h.update((v.len() as u64).to_le_bytes());
                h.update(v.as_bytes());
            }
            None => h.update([0u8]),
        }
    }
    hex::encode(h.finalize())
}

struct Moteur<'a> {
    schema: &'a SchemaAttendu,
    blanche: &'a ListeBlanche,
    perimetre: &'a Perimetre,
    sortie: Decodage,
}

impl Moteur<'_> {
    fn plan(&mut self, table: &str, colonnes: &[String]) -> Plan {
        let attendu = self.schema.tables.get(table);
        let entree = self.perimetre.table(table);
        let (disposition, domaine, cle) = match entree {
            Some(e) => (e.disposition.clone(), e.domaine.clone(), e.cle.clone()),
            None => (
                Disposition::Ignore("table_inconnue".into()),
                String::new(),
                Vec::new(),
            ),
        };
        let mut ecart = attendu.is_none();
        if attendu.is_none() {
            self.sortie
                .rapport
                .ecarts_schema
                .push(format!("table inconnue : {table}"));
        }
        let mut cols = Vec::new();
        if let Some(attendues) = attendu {
            for c in colonnes {
                if !attendues.iter().any(|a| &a.nom == c) {
                    ecart = true;
                    self.sortie
                        .rapport
                        .ecarts_schema
                        .push(format!("colonne inconnue : {table}.{c}"));
                }
            }
            for a in attendues {
                if !colonnes.contains(&a.nom) {
                    ecart = true;
                    self.sortie
                        .rapport
                        .ecarts_schema
                        .push(format!("colonne manquante : {table}.{}", a.nom));
                }
            }
            for c in colonnes {
                if let Some(a) = attendues.iter().find(|a| &a.nom == c) {
                    cols.push(plan_colonne(table, a, self.blanche));
                }
            }
        }
        let idx = |n: &str| colonnes.iter().position(|c| c == n);
        let cle_idx: Vec<usize> = cle.iter().filter_map(|c| idx(c)).collect();
        let attr = match (idx("path"), idx("value")) {
            (Some(p), Some(v)) if table.ends_with("_attr") => {
                let owner = cle_idx.iter().copied().filter(|i| *i != p).collect();
                Some((p, v, owner))
            }
            _ => None,
        };
        Plan {
            table: table.to_string(),
            disposition,
            domaine,
            cols,
            cle_idx,
            attr,
            coll_idx: idx("collection"),
            ecart,
            vus: HashSet::new(),
            docs: attr::Documents::default(),
        }
    }

    fn compter<'s>(&'s mut self, objet: &str, collection: bool) -> &'s mut ObjetRapport {
        let m = if collection {
            &mut self.sortie.rapport.collections
        } else {
            &mut self.sortie.rapport.tables
        };
        m.entry(objet.to_string()).or_default()
    }

    /// Une ligne refusée : compte (table, et collection le cas échéant) + détail.
    fn ligne_refusee(
        &mut self,
        plan: &Plan,
        objet: &str,
        ligne: u64,
        sid: Option<String>,
        refus: Vec<(Option<String>, Refus)>,
    ) {
        let code_principal = refus[0].1.code;
        let collection = objet.starts_with("collection:");
        {
            let o = self.compter(&plan.table, false);
            o.refusees += 1;
            *o.refus_par_code
                .entry(code_principal.to_string())
                .or_default() += 1;
        }
        if collection {
            let o = self.compter(objet.trim_start_matches("collection:"), true);
            o.refusees += 1;
            *o.refus_par_code
                .entry(code_principal.to_string())
                .or_default() += 1;
        }
        for (col, r) in &refus {
            let rap = &mut self.sortie.rapport;
            let deja = rap.refus.iter().filter(|d| d.objet == objet).count();
            if deja < REFUS_DETAILLES_PAR_OBJET {
                rap.refus.push(RefusDetail {
                    objet: objet.to_string(),
                    source_ligne: ligne,
                    source_id: sid.clone(),
                    code: r.code.to_string(),
                    colonne: col.clone(),
                    message: r.message.clone(),
                });
            } else {
                rap.refus_non_detailles += 1;
            }
        }
    }

    fn ligne_ignoree(&mut self, plan: &Plan, objet: &str, raison: &str) {
        let collection = objet.starts_with("collection:");
        let o = self.compter(&plan.table, false);
        o.ignorees += 1;
        *o.ignorees_par_raison.entry(raison.to_string()).or_default() += 1;
        if collection {
            let o = self.compter(objet.trim_start_matches("collection:"), true);
            o.ignorees += 1;
            *o.ignorees_par_raison.entry(raison.to_string()).or_default() += 1;
        }
    }

    fn traiter(&mut self, plan: &mut Plan, cells: Vec<Option<String>>, ligne: u64) {
        {
            let o = self.compter(&plan.table, false);
            o.lues += 1;
            if o.disposition.is_empty() {
                o.disposition = disposition_texte(&plan.disposition);
            }
        }
        // Objet et disposition de la ligne (les collections se ventilent par famille).
        let mut objet = plan.table.clone();
        let mut disposition = plan.disposition.clone();
        if let (Some(ci), true) = (plan.coll_idx, plan.disposition == Disposition::Collections) {
            let Some(Some(nom)) = cells.get(ci) else {
                let r = cellule::refus("collection_absente", "nom de collection absent");
                self.ligne_refusee(
                    plan,
                    &objet,
                    ligne,
                    None,
                    vec![(Some("collection".into()), r)],
                );
                return;
            };
            let sur = nom_collection_sur(nom);
            objet = format!("collection:{sur}");
            let o = self.compter(&sur, true);
            o.lues += 1;
            match disposition_collection(nom) {
                Some(d) => {
                    if o.disposition.is_empty() {
                        o.disposition = disposition_texte(&d);
                    }
                    disposition = d;
                }
                None => {
                    o.disposition = "inconnue".into();
                    let r = cellule::refus(
                        "collection_inconnue",
                        "famille de collection inconnue (R-01 : bloque la reprise)",
                    );
                    self.ligne_refusee(
                        plan,
                        &objet,
                        ligne,
                        None,
                        vec![(Some("collection".into()), r)],
                    );
                    return;
                }
            }
        }
        if plan.ecart {
            let r = cellule::refus(
                "schema_ecart",
                "table en écart de schéma : lignes non décodées",
            );
            self.ligne_refusee(plan, &objet, ligne, None, vec![(None, r)]);
            return;
        }
        match &disposition {
            Disposition::Sante => {
                let r = cellule::refus(
                    "sante_hds",
                    "donnée de santé ou sensible : reprise interdite sans hébergeur HDS (R-MSE5)",
                );
                self.ligne_refusee(plan, &objet, ligne, None, vec![(None, r)]);
                return;
            }
            Disposition::Ignore(raison) => {
                let raison = raison.clone();
                self.ligne_ignoree(plan, &objet, &raison);
                return;
            }
            _ => {}
        }
        if cells.len() != plan.cols.len() {
            let r = cellule::refus(
                "ligne_mal_formee",
                "nombre de cellules différent du nombre de colonnes",
            );
            self.ligne_refusee(plan, &objet, ligne, None, vec![(None, r)]);
            return;
        }
        let mut refus: Vec<(Option<String>, Refus)> = Vec::new();
        for (i, c) in plan.cols.iter().enumerate() {
            if c.secret {
                continue;
            }
            match decoder(&c.type_sql, c.nullable, cells[i].as_deref()) {
                Err(r) => refus.push((Some(c.nom.clone()), r)),
                Ok(Valeur::Json(v)) => match &c.forme {
                    None => refus.push((
                        Some(c.nom.clone()),
                        cellule::refus("json_non_catalogue", "colonne JSON sans forme déclarée"),
                    )),
                    Some(f) => {
                        let e = forme::valider(f, &v);
                        if !e.is_empty() && forme::est_a_cataloguer(&plan.table, &c.nom) {
                            let k = format!("{}.{}", plan.table, c.nom);
                            if !self.sortie.rapport.colonnes_a_cataloguer.contains(&k) {
                                self.sortie.rapport.colonnes_a_cataloguer.push(k);
                            }
                        }
                        refus.extend(e.into_iter().map(|r| (Some(c.nom.clone()), r)));
                    }
                },
                Ok(_) => {}
            }
        }
        // Identifiant source.
        let mut morceaux = Vec::new();
        for &i in &plan.cle_idx {
            match cells.get(i).and_then(|c| c.as_deref()) {
                Some(v) if !v.is_empty() => morceaux.push(v.to_string()),
                _ => {
                    refus.push((
                        plan.cols.get(i).map(|c| c.nom.clone()),
                        cellule::refus("cle_absente", "identifiant source absent"),
                    ));
                }
            }
        }
        // Pour une collection, `cle_idx` contient `collection` : l'identifiant est unique par collection.
        let mut cle = morceaux.join("/");
        let mut type_feuille = None;
        if let Some((pi, vi, owner)) = &plan.attr {
            let chemin = cells[*pi].as_deref().unwrap_or("");
            if let Err(r) = attr::valider_chemin(chemin) {
                refus.push((Some("path".into()), r));
            }
            let valeur = cells[*vi].as_deref().unwrap_or("");
            match attr::decoder_feuille(valeur) {
                Ok(t) => {
                    if let Err(r) = attr::valider_semantique(chemin, t, valeur) {
                        refus.push((Some("value".into()), r));
                    }
                    type_feuille = Some(t);
                }
                Err(r) => refus.push((Some("value".into()), r)),
            }
            let prop: Vec<&str> = owner.iter().filter_map(|i| cells[*i].as_deref()).collect();
            plan.docs.ajouter(&prop.join("/"), chemin);
        }
        let sid = id_sur(&cle);
        if !refus.is_empty() {
            self.ligne_refusee(plan, &objet, ligne, sid, refus);
            return;
        }
        if !plan.vus.insert(cle.clone()) {
            let r = cellule::refus(
                "cle_dupliquee",
                "identifiant source déjà vu dans cette table",
            );
            self.ligne_refusee(plan, &objet, ligne, sid, vec![(None, r)]);
            return;
        }
        if cle.is_empty() {
            cle = format!("ligne-{ligne}");
        }
        let zone: Vec<Option<String>> = plan
            .cols
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.secret)
            .map(|(i, _)| cells[i].clone())
            .collect();
        for (i, c) in plan.cols.iter().enumerate() {
            if c.secret && cells[i].is_some() {
                *self
                    .sortie
                    .rapport
                    .secrets_ecartes
                    .entry(format!("{}.{}", plan.table, c.nom))
                    .or_default() += 1;
            }
        }
        let l = LigneAcceptee {
            objet: objet.clone(),
            disposition: disposition.clone(),
            domaine: plan.domaine.clone(),
            source_ligne: ligne,
            cle,
            empreinte: empreinte(&zone),
            cellules: zone,
            type_feuille,
        };
        self.compter(&plan.table, false).comprises += 1;
        if objet.starts_with("collection:") {
            self.compter(objet.trim_start_matches("collection:"), true)
                .comprises += 1;
        }
        let t = self.sortie.tables.entry(plan.table.clone()).or_default();
        if t.colonnes.is_empty() {
            t.colonnes = plan
                .cols
                .iter()
                .filter(|c| !c.secret)
                .map(|c| c.nom.clone())
                .collect();
        }
        t.lignes.push(l);
    }

    fn fin_table(&mut self, plan: Plan) {
        for (prop, code) in plan.docs.conflits() {
            let r = cellule::refus(code, "document aplati incohérent");
            let sid = id_sur(&prop);
            // Le document entier est refusé : ses lignes comprises sont retirées de la zone.
            let mut retirees: Vec<String> = Vec::new();
            if let Some(t) = self.sortie.tables.get_mut(&plan.table) {
                let debut = format!("{prop}/");
                let (gardees, sorties): (Vec<_>, Vec<_>) = std::mem::take(&mut t.lignes)
                    .into_iter()
                    .partition(|l| !(prop.is_empty() || l.cle.starts_with(&debut)));
                t.lignes = gardees;
                retirees = sorties.into_iter().map(|l| l.objet).collect();
            }
            for objet in &retirees {
                let mut cibles = vec![(plan.table.clone(), false)];
                if let Some(c) = objet.strip_prefix("collection:") {
                    cibles.push((c.to_string(), true));
                }
                for (nom, coll) in cibles {
                    let o = self.compter(&nom, coll);
                    o.comprises = o.comprises.saturating_sub(1);
                    o.refusees += 1;
                    *o.refus_par_code.entry(code.to_string()).or_default() += 1;
                }
            }
            let rap = &mut self.sortie.rapport;
            rap.refus.push(RefusDetail {
                objet: plan.table.clone(),
                source_ligne: 0,
                source_id: sid,
                code: r.code.to_string(),
                colonne: None,
                message: r.message,
            });
        }
        self.sortie.rapport.documents_attr += plan.docs.nombre() as u64;
    }
}

fn disposition_texte(d: &Disposition) -> String {
    match d {
        Disposition::Reprise => "reprise".into(),
        Disposition::Rapprochement => "rapprochement".into(),
        Disposition::Archive => "archive".into(),
        Disposition::Ignore(r) => format!("ignore:{r}"),
        Disposition::Sante => "sante".into(),
        Disposition::Collections => "collections".into(),
    }
}

fn plan_colonne(table: &str, a: &ColSchema, blanche: &ListeBlanche) -> ColPlan {
    let secret = blanche.regle(table, &a.nom) == Some(Regle::Effacer);
    let forme = if a.type_sql == "jsonb" {
        forme::forme_colonne(table, &a.nom, a.nullable)
    } else {
        None
    };
    ColPlan {
        nom: a.nom.clone(),
        type_sql: a.type_sql.clone(),
        nullable: a.nullable,
        secret,
        forme,
    }
}

/// Décode l'export. Retourne une erreur seulement si l'export est illisible ; tout le reste est rapporté.
pub fn decoder_export(o: &OptionsDecodage) -> Res<Decodage> {
    garde::exiger_pas_de_connexion("--dump", &o.dump.to_string_lossy())?;
    let source = dump::ouvrir(&o.dump)?;
    let schema = SchemaAttendu::embarque();
    let blanche = ListeBlanche::embarquee();
    let perimetre = Perimetre::embarque();
    let mut m = Moteur {
        schema: &schema,
        blanche: &blanche,
        perimetre: &perimetre,
        sortie: Decodage::default(),
    };
    let mut collecte = CollecteTables::default();
    let mut courant: Option<(Plan, u64)> = None;
    let mut vues: Vec<String> = Vec::new();
    dump::lire(&source, &mut |e| {
        match e {
            Evt::Ddl(l) => collecte.ligne(&l),
            Evt::CopyDebut {
                schema: s,
                table,
                colonnes,
            } => {
                let t = dump::cle_table(&s, &table);
                let plan = m.plan(&t, &colonnes);
                m.compter(&t, false).disposition = disposition_texte(&plan.disposition);
                vues.push(t);
                courant = Some((plan, 0));
            }
            Evt::Ligne(cells) => {
                if let Some((plan, n)) = courant.as_mut() {
                    *n += 1;
                    let ligne = *n;
                    m.traiter(plan, cells, ligne);
                }
            }
            Evt::CopyFin => {
                if let Some((plan, _)) = courant.take() {
                    m.fin_table(plan);
                }
            }
        }
        Ok(())
    })?;
    // Tables requises absentes de l'export (ni DDL ni COPY).
    for t in schema.tables_requises() {
        let dans_ddl = collecte.tables.iter().any(|(n, _)| n == t);
        if !vues.contains(t) && !dans_ddl {
            m.sortie
                .rapport
                .ecarts_schema
                .push(format!("table manquante : {t}"));
        }
    }
    let mut sortie = m.sortie;
    decoder_s3(o, &mut sortie)?;
    Ok(sortie)
}

fn decoder_s3(o: &OptionsDecodage, sortie: &mut Decodage) -> Res<()> {
    let Some(p) = &o.objets_s3 else { return Ok(()) };
    garde::exiger_pas_de_connexion("--objets-s3", &p.to_string_lossy())?;
    let objets = s3::lire_liste(p)?;
    let racine = match &o.contenus_s3 {
        Some(d) => Some(garde::chemin_local_existant(
            "--contenus-s3",
            &d.to_string_lossy(),
        )?),
        None => None,
    };
    for ob in objets {
        let nat = s3json::nature(&ob.cle);
        let nom = format!("{nat:?}");
        let rap = sortie.rapport.s3.entry(nom.clone()).or_default();
        rap.lues += 1;
        rap.disposition = match nat {
            s3json::NatureS3::RentreeEnAttente | s3json::NatureS3::DemandeEnAttente => {
                "reprise".into()
            }
            s3json::NatureS3::Fichier => "fichier".into(),
            s3json::NatureS3::Secret => "refuse".into(),
            _ => "ignore".into(),
        };
        match nat {
            s3json::NatureS3::Secret => {
                rap.refusees += 1;
                *rap.refus_par_code.entry("s3_secret".into()).or_default() += 1;
            }
            s3json::NatureS3::Connaissance => {
                rap.ignorees += 1;
                *rap.ignorees_par_raison
                    .entry("perdable".into())
                    .or_default() += 1;
            }
            s3json::NatureS3::JsonInventorie => {
                rap.ignorees += 1;
                *rap.ignorees_par_raison
                    .entry("inventorie_non_repris".into())
                    .or_default() += 1;
            }
            s3json::NatureS3::Fichier => {
                rap.comprises += 1;
            }
            s3json::NatureS3::RentreeEnAttente | s3json::NatureS3::DemandeEnAttente => {
                let Some(racine) = &racine else {
                    rap.ignorees += 1;
                    *rap.ignorees_par_raison
                        .entry("contenu_non_fourni".into())
                        .or_default() += 1;
                    sortie.objets_s3.push(ObjetS3Classe {
                        objet: ob,
                        nature: nat,
                    });
                    continue;
                };
                let lecture = lire_contenu(racine, &ob.cle);
                match lecture.and_then(|c| {
                    s3json::decoder(nat, &c).map_err(|v| {
                        v.into_iter()
                            .next()
                            .unwrap_or_else(|| cellule::refus("s3_nature", "illisible"))
                    })
                }) {
                    Ok(v) => {
                        let exp = v
                            .get("expiresAt")
                            .and_then(|x| x.as_str())
                            .and_then(|x| DateTime::parse_from_rfc3339(x).ok());
                        let expire = exp.is_none_or(|e| e.with_timezone(&Utc) < o.maintenant);
                        if expire {
                            rap.ignorees += 1;
                            *rap.ignorees_par_raison
                                .entry("jeton_expire".into())
                                .or_default() += 1;
                        } else {
                            rap.comprises += 1;
                        }
                        sortie.documents_s3.push(DocumentS3 {
                            cle: ob.cle.clone(),
                            nature: nat,
                            valeur: v,
                            expire,
                        });
                    }
                    Err(r) => {
                        rap.refusees += 1;
                        *rap.refus_par_code.entry(r.code.to_string()).or_default() += 1;
                        sortie.rapport.refus.push(RefusDetail {
                            objet: nom,
                            source_ligne: 0,
                            source_id: None,
                            code: r.code.to_string(),
                            colonne: None,
                            message: r.message,
                        });
                    }
                }
            }
        }
        sortie.objets_s3.push(ObjetS3Classe {
            objet: ob,
            nature: nat,
        });
    }
    Ok(())
}

fn lire_contenu(racine: &std::path::Path, cle: &str) -> Result<String, Refus> {
    if cle.split('/').any(|s| s == ".." || s.is_empty()) || cle.starts_with('/') {
        return Err(cellule::refus("s3_cle_invalide", "clé d'objet non sûre"));
    }
    let chemin = racine.join(cle);
    let meta = std::fs::metadata(&chemin)
        .map_err(|_| cellule::refus("s3_contenu_absent", "contenu de l'objet non fourni"))?;
    if meta.len() > 1 << 20 {
        return Err(cellule::refus("s3_taille", "objet JSON au-delà de 1 Mio"));
    }
    std::fs::read_to_string(&chemin)
        .map_err(|_| cellule::refus("s3_contenu_illisible", "contenu non lisible en UTF-8"))
}

/// Rapport lisible (français), sans valeur de donnée.
pub fn texte(r: &RapportDecodage) -> String {
    let mut s = String::from("=== Décodage strict de l'export ===\n");
    let ligne = |o: &ObjetRapport| {
        format!(
            "lues {} · comprises {} · refusées {} · ignorées {}",
            o.lues, o.comprises, o.refusees, o.ignorees
        )
    };
    s.push_str("\n-- Tables avec des lignes --\n");
    for (t, o) in r.tables.iter().filter(|(_, o)| o.lues > 0) {
        s.push_str(&format!("{t} [{}] : {}\n", o.disposition, ligne(o)));
        for (c, n) in &o.refus_par_code {
            s.push_str(&format!("    refus {c} : {n}\n"));
        }
        for (c, n) in &o.ignorees_par_raison {
            s.push_str(&format!("    ignorées ({c}) : {n}\n"));
        }
    }
    if !r.collections.is_empty() {
        s.push_str("\n-- Collections --\n");
        for (t, o) in &r.collections {
            s.push_str(&format!("{t} [{}] : {}\n", o.disposition, ligne(o)));
        }
    }
    if !r.s3.is_empty() {
        s.push_str("\n-- Objets S3 --\n");
        for (t, o) in &r.s3 {
            s.push_str(&format!("{t} : {}\n", ligne(o)));
        }
    }
    if !r.ecarts_schema.is_empty() {
        s.push_str("\n-- Écarts de schéma --\n");
        for e in &r.ecarts_schema {
            s.push_str(&format!("{e}\n"));
        }
    }
    if !r.colonnes_a_cataloguer.is_empty() {
        s.push_str(&format!(
            "\nColonnes JSON à cataloguer (contenu refusé faute de catalogue) : {}\n",
            r.colonnes_a_cataloguer.join(", ")
        ));
    }
    s.push_str(&format!(
        "\nDocuments aplatis : {} · refus détaillés : {} (+ {} non détaillés)\n",
        r.documents_attr,
        r.refus.len(),
        r.refus_non_detailles
    ));
    s.push_str(if r.conforme() {
        "Verdict : CONFORME\n"
    } else {
        "Verdict : NON CONFORME (voir les refus)\n"
    });
    s
}
