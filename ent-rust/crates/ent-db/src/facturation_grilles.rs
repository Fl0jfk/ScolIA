//! Socle 24, lot F3 — **catalogue d'articles qualifiés, grilles tarifaires votées, simulateur, état des
//! contributions** (R-F9, R-F10, R-F23, FD-17, FD-19, FAC-103 à FAC-129, FAC-196, MIG-6).
//!
//! * **Article qualifié** : nature légale (R. 442-48), régime de TVA et unité posés ou validés par la
//!   comptabilité ; les articles existants ne sont jamais modifiés d'office (une **proposition** par règle
//!   leur est présentée) ; une fois qualifiée, la nature d'un article qui porte des lignes ne change plus.
//! * **Grille versionnée** : brouillon → proposée → votée → en vigueur → close (+ abandonnée). Les
//!   transitions, le gel d'une version non brouillon et l'immuabilité de ses tarifs et tranches sont aussi
//!   tenus **par la base** (migration 523). Une correction est une nouvelle version datée (FAC-128).
//! * **Séparation des tâches (R-F23)** : celui qui a **proposé** la grille au conseil ne l'enregistre pas
//!   comme votée, sauf si l'établissement désactive le « second regard » (paramètre, activé par défaut) ;
//!   voter et corriger exigent le droit `facturation.voter`, que la comptabilité n'a pas.
//! * **Simulateur** : scénarios de hausse (pourcentage ou montant, arrondi à l'euro ou à 5 €) sur les
//!   effectifs prévus ou actuels ; **aucun nom** : des nombres par niveau et régime ; familles types fictives.
//!
//! Chaque commande : droit fin (`ent_auth::Action::Facturation*`) → transaction d'établissement (RLS) →
//! écriture → **journal** (même transaction) → événement `facturation.*` sans montant. Toute cible absente ou
//! d'un autre établissement répond « introuvable ». Aucune horloge dans les calculs (règle 33) : les dates
//! (vote, mise en vigueur) sont fournies par l'appelant.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::{types, AggregateKind};
use ent_types::finances_grilles::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::facturation_service::{journaliser_tx, Acte, Chemin};
use crate::finances::{
    annee_valide, conflit, db, est_doublon, introuvable, invalide, opt_texte, publier_finance,
    texte, CATEGORIES, REGIMES,
};
use crate::finances_calcul::{self as calcul, TarifCandidat};
use crate::outbox::Causalite;
use crate::service::{EntDbError, PgEnt, RefusMetier};

const MONTANT_MAX: i64 = 100_000_000;
/// Plafonds par établissement (F5) : familles types et simulations ; au-delà, supprimer d'abord.
const MAX_FAMILLES_TYPES: i64 = 200;
const MAX_SIMULATIONS: i64 = 200;

// ---------------------------------------------------------------------------
// Fonctions pures : catalogue
// ---------------------------------------------------------------------------

/// Proposition de qualification pour un article **existant** (MIG-6) d'après sa catégorie : une règle,
/// jamais une décision. La comptabilité la valide, la corrige ou la refuse.
pub fn proposition_qualification(categorie: &str) -> PropositionQualificationDto {
    let (nature, regime, unite, red, explication) = match categorie {
        "scolarite" => (
            "contribution_r442_48_2",
            "exonere_261_4_4",
            "annee",
            true,
            "contribution des familles : 1° ou 2° de l'article R. 442-48 selon le contrat, à confirmer",
        ),
        "restauration" => (
            "demi_pension",
            "exonere_261_4_4",
            "annee",
            true,
            "nourriture des demi-pensionnaires et internes : prestation étroitement liée, exonérée",
        ),
        "internat" => (
            "internat",
            "exonere_261_4_4",
            "annee",
            true,
            "logement des internes : prestation étroitement liée, exonérée",
        ),
        "activite" => (
            "prestation_facultative",
            "exonere_261_4_4",
            "unite",
            true,
            "prestation facultative : exonération à confirmer par l'expert-comptable",
        ),
        "frais" => (
            "frais_dossier",
            "exonere_261_4_4",
            "unite",
            true,
            "frais annexes : nature à préciser (dossier, examen, fournitures)",
        ),
        // bourse et réduction : écritures de contrepartie, hors champ de la TVA, jamais réduites elles-mêmes.
        _ => (
            "autre_motivee",
            "hors_champ",
            "unite",
            false,
            "écriture de contrepartie (bourse, réduction) : hors champ, nature à motiver",
        ),
    };
    PropositionQualificationDto {
        nature_legale: nature.into(),
        regime_tva: regime.into(),
        unite: unite.into(),
        mode: "forfait".into(),
        soumis_reductions: red,
        soumis_remise_ordre: matches!(categorie, "restauration" | "internat"),
        explication: explication.into(),
    }
}

#[allow(clippy::too_many_arguments)]
fn modele(
    code: &str,
    libelle: &str,
    categorie: &str,
    nature: &str,
    regime: &str,
    taux: Option<i32>,
    unite: &str,
    mode: &str,
    compte: &str,
    flags: (bool, bool, bool),
    remarque: Option<&str>,
) -> ModeleArticleDto {
    ModeleArticleDto {
        code: code.into(),
        libelle: libelle.into(),
        categorie: categorie.into(),
        nature_legale: nature.into(),
        regime_tva: regime.into(),
        taux_tva_bps: taux,
        unite: unite.into(),
        mode: mode.into(),
        remboursable: flags.0,
        soumis_reductions: flags.1,
        soumis_remise_ordre: flags.2,
        compte_produit: compte.into(),
        remarque: remarque.map(Into::into),
    }
}

/// Modèles d'articles fournis par ScolIA (D-023 : prêts à l'emploi) ; l'établissement les copie et les
/// renomme (D-022). Aucune donnée n'est écrite tant qu'un modèle n'est pas copié.
pub fn modeles_articles() -> Vec<ModeleArticleDto> {
    let exo = "exonere_261_4_4";
    let n = (false, true, false);
    let tiers = (false, false, false);
    vec![
        modele(
            "CONTRIBUTION",
            "Contribution des familles",
            "scolarite",
            "contribution_r442_48_2",
            exo,
            None,
            "annee",
            "forfait",
            "706000",
            n,
            Some("1° ou 2° de l'article R. 442-48 : à confirmer avec le contrat d'association"),
        ),
        modele(
            "EXTERNAT_SURVEILLE",
            "Externat surveillé",
            "scolarite",
            "externat_surveille",
            exo,
            None,
            "annee",
            "forfait",
            "706000",
            n,
            None,
        ),
        modele(
            "DEMI_PENSION",
            "Demi-pension (forfait)",
            "restauration",
            "demi_pension",
            exo,
            None,
            "annee",
            "forfait",
            "706000",
            (false, true, true),
            None,
        ),
        modele(
            "DEMI_PENSION_REPAS",
            "Repas (au réel)",
            "restauration",
            "demi_pension",
            exo,
            None,
            "repas",
            "reel",
            "706000",
            (false, true, true),
            None,
        ),
        modele(
            "INTERNAT",
            "Internat (forfait)",
            "internat",
            "internat",
            exo,
            None,
            "annee",
            "forfait",
            "706000",
            (false, true, true),
            None,
        ),
        modele(
            "INTERNAT_NUIT",
            "Internat (à la nuitée)",
            "internat",
            "internat",
            exo,
            None,
            "nuit",
            "reel",
            "706000",
            (false, true, true),
            None,
        ),
        modele(
            "ETUDES",
            "Études surveillées",
            "activite",
            "prestation_facultative",
            exo,
            None,
            "mois",
            "forfait",
            "706000",
            n,
            Some("exonération à confirmer par l'expert-comptable"),
        ),
        modele(
            "GARDERIE",
            "Garderie",
            "activite",
            "prestation_facultative",
            exo,
            None,
            "mois",
            "forfait",
            "706000",
            n,
            Some("garde des moins de 6 ans : attestation à part (socle 25)"),
        ),
        modele(
            "ACTIVITE",
            "Activité",
            "activite",
            "prestation_facultative",
            exo,
            None,
            "seance",
            "forfait",
            "706000",
            n,
            None,
        ),
        modele(
            "SORTIE",
            "Sortie scolaire",
            "activite",
            "sortie_voyage",
            exo,
            None,
            "unite",
            "reel",
            "706000",
            tiers,
            Some("une sortie obligatoire n'est jamais facturée aux familles (R-F41)"),
        ),
        modele(
            "VOYAGE",
            "Voyage scolaire",
            "activite",
            "sortie_voyage",
            exo,
            None,
            "unite",
            "reel",
            "706000",
            tiers,
            Some("une sortie obligatoire n'est jamais facturée aux familles (R-F41)"),
        ),
        modele(
            "FOURNITURES",
            "Fournitures",
            "frais",
            "fournitures",
            exo,
            None,
            "unite",
            "forfait",
            "706000",
            tiers,
            None,
        ),
        modele(
            "MANUELS",
            "Manuels",
            "frais",
            "fournitures",
            exo,
            None,
            "unite",
            "forfait",
            "706000",
            tiers,
            None,
        ),
        modele(
            "FRAIS_DOSSIER",
            "Frais de dossier",
            "frais",
            "frais_dossier",
            exo,
            None,
            "unite",
            "forfait",
            "706000",
            tiers,
            None,
        ),
        modele(
            "FRAIS_EXAMEN",
            "Frais d'examen",
            "frais",
            "autre_motivee",
            exo,
            None,
            "unite",
            "forfait",
            "706000",
            tiers,
            Some("nature à motiver"),
        ),
        modele(
            "CARTE_CANTINE",
            "Carte de cantine perdue",
            "frais",
            "autre_motivee",
            exo,
            None,
            "unite",
            "forfait",
            "706000",
            tiers,
            Some("nature à motiver"),
        ),
        modele(
            "COTISATION_APEL",
            "Cotisation APEL",
            "frais",
            "autre_motivee",
            "pour_compte_tiers",
            None,
            "annee",
            "forfait",
            "467000",
            tiers,
            Some("encaissement pour compte de tiers : hors chiffre d'affaires"),
        ),
        modele(
            "ASSURANCE",
            "Assurance scolaire",
            "frais",
            "autre_motivee",
            "pour_compte_tiers",
            None,
            "annee",
            "forfait",
            "467000",
            tiers,
            Some("encaissement pour compte de tiers : hors chiffre d'affaires"),
        ),
        modele(
            "CAUTION",
            "Caution (badge, casier, matériel)",
            "frais",
            "autre_motivee",
            "hors_champ",
            None,
            "unite",
            "forfait",
            "275000",
            (true, false, false),
            Some("dépôt remboursable : ce n'est pas une vente"),
        ),
        modele(
            "LOCATION_SALLE",
            "Location de salle",
            "frais",
            "autre_motivee",
            "taxable",
            Some(2000),
            "unite",
            "forfait",
            "706000",
            tiers,
            Some("prestation taxable : avis de l'expert-comptable requis (Q-F18)"),
        ),
    ]
}

fn valider_qualification(categorie: &str, b: &QualifierArticleBody) -> Result<(), EntDbError> {
    if !NATURES_LEGALES.contains(&b.nature_legale.as_str()) {
        return Err(invalide("nature légale inconnue"));
    }
    if !REGIMES_TVA.contains(&b.regime_tva.as_str()) {
        return Err(invalide("régime de TVA inconnu"));
    }
    match (b.regime_tva.as_str(), b.taux_tva_bps) {
        ("taxable", Some(t)) if (1..=2500).contains(&t) => {}
        ("taxable", _) => {
            return Err(invalide(
                "un article taxable porte un taux entre 0,01 % et 25 %",
            ))
        }
        (_, Some(_)) => return Err(invalide("le taux de TVA ne concerne que le régime taxable")),
        _ => {}
    }
    if !UNITES.contains(&b.unite.as_str()) {
        return Err(invalide("unité inconnue"));
    }
    if b.mode
        .as_deref()
        .is_some_and(|m| m != "forfait" && m != "reel")
    {
        return Err(invalide("mode : forfait ou reel"));
    }
    if !CATEGORIES.contains(&categorie) {
        return Err(invalide("catégorie inconnue"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Fonctions pures : hausse, arrondi, tranches, contrôles
// ---------------------------------------------------------------------------

/// Arrondi au centime supérieur à partir de la demi-unité (« half up »), en centimes.
pub fn arrondir(montant: i64, arrondi: Arrondi) -> i64 {
    let pas = match arrondi {
        Arrondi::Aucun => return montant,
        Arrondi::Euro => 100,
        Arrondi::CinqEuros => 500,
    };
    ((montant + pas / 2).div_euclid(pas)) * pas
}

/// Montant après hausse puis arrondi. `valeur` : points de base (pourcentage) ou centimes (montant) ;
/// négative = baisse, plancher à 0. Erreur au-delà de 1 000 000 €.
pub fn montant_apres_hausse(
    montant: i64,
    mode: ModeHausse,
    valeur: i64,
    arrondi: Arrondi,
) -> Result<i64, EntDbError> {
    let brut: i128 = match mode {
        ModeHausse::Montant => i128::from(montant) + i128::from(valeur),
        ModeHausse::Pourcentage => {
            let delta = i128::from(montant) * i128::from(valeur);
            // arrondi symétrique (demi éloigné de zéro) : une baisse de 0,5 centime s'arrondit comme une hausse
            let abs = (delta.abs() + 5_000) / 10_000;
            i128::from(montant) + if delta < 0 { -abs } else { abs }
        }
    };
    let brut = brut.max(0);
    if brut > i128::from(MONTANT_MAX) {
        return Err(invalide("montant au-delà de 1 000 000 €"));
    }
    let m = arrondir(i64::try_from(brut).unwrap_or(MONTANT_MAX), arrondi);
    if m > MONTANT_MAX {
        return Err(invalide("montant au-delà de 1 000 000 €"));
    }
    Ok(m)
}

/// Tranches de quotient d'une version : sans trou ni chevauchement, de 0 à l'infini (FAC-118). Renvoie le
/// motif du refus.
pub fn valider_tranches(tranches: &[TrancheQfBody]) -> Result<(), &'static str> {
    if tranches.is_empty() {
        return Ok(());
    }
    if tranches.len() > 12 {
        return Err("douze tranches au plus");
    }
    let mut codes = BTreeSet::new();
    for t in tranches {
        let c = t.code.trim();
        if c.is_empty()
            || c.len() > 8
            || !c
                .chars()
                .all(|x| x.is_ascii_uppercase() || x.is_ascii_digit() || x == '_')
        {
            return Err("code de tranche : 1 à 8 majuscules, chiffres ou _");
        }
        if t.libelle.trim().is_empty() || t.libelle.trim().chars().count() > 60 {
            return Err("libellé de tranche obligatoire (60 caractères)");
        }
        if !codes.insert(c.to_string()) {
            return Err("deux tranches portent le même code");
        }
        if t.qf_min_centimes < 0 || t.qf_max_centimes.is_some_and(|m| m <= t.qf_min_centimes) {
            return Err("tranche : minimum ≥ 0 et maximum > minimum");
        }
    }
    let mut tri: Vec<&TrancheQfBody> = tranches.iter().collect();
    tri.sort_by_key(|t| t.qf_min_centimes);
    if tri[0].qf_min_centimes != 0 {
        return Err("la première tranche commence à 0 (aucun quotient sans tranche)");
    }
    for w in tri.windows(2) {
        match w[0].qf_max_centimes {
            None => return Err("seule la dernière tranche est ouverte"),
            Some(m) if m < w[1].qf_min_centimes => return Err("trou entre deux tranches"),
            Some(m) if m > w[1].qf_min_centimes => return Err("deux tranches se chevauchent"),
            _ => {}
        }
    }
    if tri[tri.len() - 1].qf_max_centimes.is_some() {
        return Err("la dernière tranche est ouverte (aucun quotient sans tranche)");
    }
    Ok(())
}

/// Contrôles d'une version avant proposition (§10.2, R-F9, FAC-119). `non_qualifies` : articles de la grille
/// dont la nature légale et la TVA ne sont pas validées.
pub fn controler_grille(
    tarifs: &[TarifGrilleDto],
    tranches: &[TrancheQfDto],
    non_qualifies: &BTreeSet<Uuid>,
) -> Vec<AnomalieGrilleDto> {
    let mut v = Vec::new();
    let mut ajoute = |code: &str, message: String, article: Option<Uuid>, bloquante: bool| {
        v.push(AnomalieGrilleDto {
            code: code.into(),
            message,
            article_id: article,
            bloquante,
        })
    };
    if tarifs.is_empty() {
        ajoute(
            "grille_vide",
            "la grille ne contient aucun tarif".into(),
            None,
            true,
        );
        return v;
    }
    let mut vus = BTreeSet::new();
    let mut par_article: BTreeMap<Uuid, Vec<&TarifGrilleDto>> = BTreeMap::new();
    for t in tarifs {
        par_article.entry(t.article_id).or_default().push(t);
        let cle = (
            t.article_id,
            t.niveau.clone(),
            t.regime.clone(),
            t.qf_min_centimes,
            t.qf_max_centimes,
        );
        if !vus.insert(cle) {
            ajoute(
                "tarif_en_double",
                format!(
                    "{} : deux tarifs pour les mêmes niveau, régime et tranche",
                    t.article_code
                ),
                Some(t.article_id),
                true,
            );
        }
        if t.montant_annuel_centimes == 0 && t.categorie != "bourse" && t.categorie != "reduction" {
            ajoute(
                "montant_nul",
                format!("{} : montant à 0 €", t.article_code),
                Some(t.article_id),
                false,
            );
        }
    }
    fn a_des_bornes(t: &TarifGrilleDto) -> bool {
        t.qf_min_centimes.is_some() || t.qf_max_centimes.is_some()
    }
    for (art, ts) in &par_article {
        let code = &ts[0].article_code;
        if non_qualifies.contains(art) {
            ajoute(
                "article_non_qualifie",
                format!("{code} : nature légale et TVA à valider (R-F9)"),
                Some(*art),
                true,
            );
        }
        if ts.iter().any(|t| a_des_bornes(t)) {
            if tranches.is_empty() {
                ajoute(
                    "tranches_manquantes",
                    format!(
                        "{code} : des tarifs par tranche de quotient, mais aucune tranche définie"
                    ),
                    Some(*art),
                    true,
                );
            }
            if !ts.iter().any(|t| t.tarif_par_defaut) {
                ajoute(
                    "tarif_defaut_manquant",
                    format!("{code} : désigner le tarif par défaut (sans quotient, FAC-119)"),
                    Some(*art),
                    true,
                );
            }
            for t in ts.iter().filter(|t| a_des_bornes(t)) {
                let connue = tranches.iter().any(|q| {
                    Some(q.qf_min_centimes) == t.qf_min_centimes.or(Some(0))
                        && q.qf_max_centimes == t.qf_max_centimes
                });
                if !tranches.is_empty() && !connue {
                    ajoute(
                        "tarif_hors_tranche",
                        format!(
                            "{code} : bornes de quotient qui ne correspondent à aucune tranche"
                        ),
                        Some(*art),
                        true,
                    );
                }
            }
        }
        for t in ts.iter().filter(|t| t.tarif_par_defaut) {
            if t.qf_max_centimes.is_some() {
                ajoute(
                    "defaut_incoherent",
                    format!("{code} : le tarif par défaut n'a pas de borne haute (plein tarif)"),
                    Some(*art),
                    true,
                );
            }
        }
    }
    v
}

/// Tarif de la version ou de l'historique précédent (identité seulement), pour la continuité (règle 4).
#[derive(Clone, Debug)]
pub struct TarifPrecedent {
    pub article_id: Uuid,
    pub article_code: String,
    pub niveau: Option<String>,
    pub regime: Option<String>,
}

/// Rien ne disparaît en silence (D-069, règle 4) : un article tarifé par la version (ou l'historique) précédente
/// et absent de la nouvelle, ou un niveau / régime qui n'a plus de tarif, **bloque** ; un article déjà facturé
/// sur l'année sans aucun tarif dans la nouvelle version est **signalé**.
pub fn controler_continuite(
    precedents: &[TarifPrecedent],
    tarifs: &[TarifGrilleDto],
    facturees_sans_tarif: &[(Uuid, String)],
) -> Vec<AnomalieGrilleDto> {
    let mut v = Vec::new();
    let mut articles_oublies = BTreeSet::new();
    let mut signales = BTreeSet::new();
    for p in precedents {
        let tarifs_article: Vec<&TarifGrilleDto> = tarifs
            .iter()
            .filter(|t| t.article_id == p.article_id)
            .collect();
        if tarifs_article.is_empty() {
            if articles_oublies.insert(p.article_id) {
                v.push(AnomalieGrilleDto {
                    code: "article_oublie".into(),
                    message: format!(
                        "{} : tarifé par la version précédente, absent de cette grille (rien ne disparaît en silence)",
                        p.article_code
                    ),
                    article_id: Some(p.article_id),
                    bloquante: true,
                });
            }
            continue;
        }
        let couvert = tarifs_article.iter().any(|t| {
            (t.niveau.is_none() || t.niveau == p.niveau)
                && (t.regime.is_none() || t.regime == p.regime)
        });
        if !couvert && signales.insert((p.article_id, p.niveau.clone(), p.regime.clone())) {
            v.push(AnomalieGrilleDto {
                code: "tarif_perdu".into(),
                message: format!(
                    "{} : plus aucun tarif pour niveau {} / régime {} (tarifé par la version précédente)",
                    p.article_code,
                    p.niveau.as_deref().unwrap_or("tous"),
                    p.regime.as_deref().unwrap_or("tous")
                ),
                article_id: Some(p.article_id),
                bloquante: true,
            });
        }
    }
    for (id, code) in facturees_sans_tarif {
        if !articles_oublies.contains(id) {
            v.push(AnomalieGrilleDto {
                code: "article_facture_sans_tarif".into(),
                message: format!(
                    "{code} : facturé sur cette année, mais sans tarif dans cette grille"
                ),
                article_id: Some(*id),
                bloquante: false,
            });
        }
    }
    v
}

// ---------------------------------------------------------------------------
// Fonctions pures : simulation
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ArticleSim {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub categorie: String,
    pub soumis_reductions: bool,
}

impl ArticleSim {
    fn categorie_protegee(&self) -> bool {
        categorie_bourse_ou_reduction(&self.categorie)
    }
}

/// Bourse et réduction : une hausse générale en montant ne les touche jamais (F7).
fn categorie_bourse_ou_reduction(categorie: &str) -> bool {
    matches!(categorie, "bourse" | "reduction")
}

#[derive(Clone, Debug)]
pub struct TarifSim {
    pub id: Uuid,
    pub article_id: Uuid,
    pub niveau: Option<String>,
    pub regime: Option<String>,
    pub qf_min: Option<i64>,
    pub qf_max: Option<i64>,
    pub montant: i64,
}

#[derive(Clone, Debug)]
pub struct RegleSim {
    pub article_id: Option<Uuid>,
    pub mode: ModeHausse,
    pub valeur: i64,
    pub arrondi: Arrondi,
}

#[derive(Clone, Debug)]
pub struct FratrieSim {
    pub rang_min: i64,
    pub taux_bps: i32,
    pub categories: Vec<String>,
}

/// La règle d'un article l'emporte sur la règle générale.
fn regle_de(regles: &[RegleSim], article: Uuid) -> Option<&RegleSim> {
    regles
        .iter()
        .find(|r| r.article_id == Some(article))
        .or_else(|| regles.iter().find(|r| r.article_id.is_none()))
}

pub fn appliquer_regles(
    tarifs: &[TarifSim],
    regles: &[RegleSim],
    articles: &[ArticleSim],
) -> Result<Vec<TarifSim>, EntDbError> {
    tarifs
        .iter()
        .map(|t| {
            let mut n = t.clone();
            if let Some(r) = regle_de(regles, t.article_id) {
                // une hausse en montant générale n'atteint jamais une bourse ni une réduction (F7)
                let epargne = r.article_id.is_none()
                    && r.mode == ModeHausse::Montant
                    && articles
                        .iter()
                        .any(|a| a.id == t.article_id && a.categorie_protegee());
                if !epargne {
                    n.montant = montant_apres_hausse(t.montant, r.mode, r.valeur, r.arrondi)?;
                }
            }
            Ok(n)
        })
        .collect()
}

fn candidats(tarifs: &[TarifSim]) -> Vec<TarifCandidat> {
    tarifs
        .iter()
        .map(|t| TarifCandidat {
            id: t.id,
            article_id: t.article_id,
            niveau: t.niveau.clone(),
            regime: t.regime.clone(),
            qf_min: t.qf_min,
            qf_max: t.qf_max,
            montant_annuel: t.montant,
        })
        .collect()
}

/// Produit annuel attendu : pour chaque (niveau, régime) et chaque article dû par ce régime, effectif × tarif
/// plein (sans quotient). Renvoie le total et le détail par article.
pub fn produit_attendu(
    articles: &[ArticleSim],
    tarifs: &[TarifSim],
    effectifs: &[(String, String, i32)],
) -> (i64, BTreeMap<Uuid, i64>) {
    let c = candidats(tarifs);
    let mut par_article: BTreeMap<Uuid, i64> = BTreeMap::new();
    for (niveau, regime, n) in effectifs {
        let regime = calcul::regime_normalise(Some(regime));
        for a in articles {
            if !calcul::categorie_due(&a.categorie, &regime) {
                continue;
            }
            if let Some(t) = calcul::choisir_tarif(&c, a.id, Some(niveau), &regime, None) {
                *par_article.entry(a.id).or_default() += t.montant_annuel * i64::from(*n);
            }
        }
    }
    (par_article.values().sum(), par_article)
}

/// Total annuel d'une famille type : un total par enfant (tarif choisi avec le quotient de la tranche), moins la
/// meilleure réduction de fratrie atteinte par le rang de l'enfant (comme la génération réelle).
pub fn total_famille(
    articles: &[ArticleSim],
    tarifs: &[TarifSim],
    enfants: &[(String, String)],
    qf: Option<i64>,
    fratries: &[FratrieSim],
) -> i64 {
    let c = candidats(tarifs);
    let mut total = 0;
    for (i, (niveau, regime)) in enfants.iter().enumerate() {
        let rang = (i + 1) as i64;
        let regime = calcul::regime_normalise(Some(regime));
        for a in articles {
            if !calcul::categorie_due(&a.categorie, &regime) {
                continue;
            }
            let Some(t) = calcul::choisir_tarif(&c, a.id, Some(niveau), &regime, qf) else {
                continue;
            };
            let mut montant = t.montant_annuel;
            if a.soumis_reductions {
                let meilleure = fratries
                    .iter()
                    .filter(|f| rang >= f.rang_min && f.categories.contains(&a.categorie))
                    .map(|f| f.taux_bps)
                    .max();
                if let Some(bps) = meilleure {
                    montant -= calcul::taux(montant, bps);
                }
            }
            total += montant;
        }
    }
    total
}

fn euros(centimes: i64) -> String {
    let signe = if centimes < 0 { "-" } else { "" };
    let a = centimes.abs();
    format!("{signe}{},{:02}", a / 100, a % 100)
}

/// Protège une cellule CSV : séparateur, guillemets, retours, et formules (`=`, `+`, `-`, `@`).
fn cellule_csv(s: &str) -> String {
    // le contrôle porte sur le texte d'origine : tabulation et retour chariot en tête sont aussi neutralisés
    let formule = s.starts_with(['=', '+', '-', '@', '\t', '\r']);
    let mut t = s.replace(['\r', '\n', '\t'], " ");
    if formule {
        t.insert(0, '\'');
    }
    if t.contains([';', '"']) {
        t = format!("\"{}\"", t.replace('"', "\"\""));
    }
    t
}

// ---------------------------------------------------------------------------
// Lignes SQL
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow, Clone)]
struct VersionRow {
    id: Uuid,
    annee_scolaire: String,
    numero: i32,
    libelle: String,
    statut: String,
    remplace_id: Option<Uuid>,
    motif: Option<String>,
    effet_retroactif: bool,
    cree_par: Option<Uuid>,
    propose_le: Option<DateTime<Utc>>,
    propose_par: Option<Uuid>,
    vote_le: Option<NaiveDate>,
    vote_reference: Option<String>,
    vote_document_id: Option<Uuid>,
    en_vigueur_du: Option<NaiveDate>,
    close_le: Option<DateTime<Utc>>,
    publiee_le: Option<DateTime<Utc>>,
    nb_tarifs: i64,
    nb_tranches: i64,
}

impl VersionRow {
    fn dto(&self) -> GrilleVersionDto {
        GrilleVersionDto {
            id: self.id,
            annee_scolaire: self.annee_scolaire.clone(),
            numero: self.numero,
            libelle: self.libelle.clone(),
            statut: self.statut.clone(),
            remplace_id: self.remplace_id,
            motif: self.motif.clone(),
            effet_retroactif: self.effet_retroactif,
            propose_le: self.propose_le,
            vote_le: self.vote_le,
            vote_reference: self.vote_reference.clone(),
            vote_document_id: self.vote_document_id,
            en_vigueur_du: self.en_vigueur_du,
            close_le: self.close_le,
            publiee_le: self.publiee_le,
            nb_tarifs: self.nb_tarifs,
            nb_tranches: self.nb_tranches,
        }
    }
}

const SELECT_VERSION: &str =
    "SELECT g.id, g.annee_scolaire, g.numero, g.libelle, g.statut, g.remplace_id, g.motif,
        g.effet_retroactif, g.cree_par, g.propose_le, g.propose_par, g.vote_le, g.vote_reference,
        g.vote_document_id, g.en_vigueur_du, g.close_le, g.publiee_le,
        (SELECT count(*) FROM ent_fin_tarif t WHERE t.etablissement_id = g.etablissement_id
            AND t.grille_version_id = g.id AND t.actif) AS nb_tarifs,
        (SELECT count(*) FROM ent_fin_tranche_qf q WHERE q.etablissement_id = g.etablissement_id
            AND q.grille_version_id = g.id) AS nb_tranches
   FROM ent_fin_grille_version g";

async fn version_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<VersionRow, EntDbError> {
    let sql = format!(
        "{SELECT_VERSION} WHERE g.etablissement_id = $1 AND g.id = $2{}",
        if verrou { " FOR UPDATE OF g" } else { "" }
    );
    sqlx::query_as::<_, VersionRow>(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

#[derive(sqlx::FromRow)]
struct TarifGrilleRow {
    id: Uuid,
    grille_version_id: Uuid,
    annee_scolaire: String,
    article_id: Uuid,
    code: String,
    libelle: String,
    categorie: String,
    niveau: Option<String>,
    regime: Option<String>,
    qf_min_centimes: Option<i64>,
    qf_max_centimes: Option<i64>,
    montant_annuel_centimes: i64,
    tarif_par_defaut: bool,
}

async fn tarifs_version_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    version: Uuid,
    tarif: Option<Uuid>,
) -> Result<Vec<TarifGrilleDto>, EntDbError> {
    let rows: Vec<TarifGrilleRow> = sqlx::query_as(
        "SELECT t.id, t.grille_version_id, t.annee_scolaire, t.article_id, a.code, a.libelle, a.categorie,
                t.niveau, t.regime, t.qf_min_centimes, t.qf_max_centimes, t.montant_annuel_centimes,
                t.tarif_par_defaut
           FROM ent_fin_tarif t
           JOIN ent_fin_article a ON a.id = t.article_id AND a.etablissement_id = t.etablissement_id
          WHERE t.etablissement_id = $1 AND t.grille_version_id = $2 AND t.actif
            AND ($3::UUID IS NULL OR t.id = $3)
          ORDER BY a.categorie, a.code, t.niveau NULLS FIRST, t.regime NULLS FIRST,
                   t.qf_min_centimes NULLS FIRST, t.id",
    )
    .bind(etab)
    .bind(version)
    .bind(tarif)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| TarifGrilleDto {
            id: r.id,
            grille_version_id: r.grille_version_id,
            annee_scolaire: r.annee_scolaire,
            article_id: r.article_id,
            article_code: r.code,
            article_libelle: r.libelle,
            categorie: r.categorie,
            niveau: r.niveau,
            regime: r.regime,
            qf_min_centimes: r.qf_min_centimes,
            qf_max_centimes: r.qf_max_centimes,
            montant_annuel_centimes: r.montant_annuel_centimes,
            tarif_par_defaut: r.tarif_par_defaut,
        })
        .collect())
}

async fn tranches_version_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    version: Uuid,
) -> Result<Vec<TrancheQfDto>, EntDbError> {
    let rows: Vec<(Uuid, String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, code, libelle, qf_min_centimes, qf_max_centimes FROM ent_fin_tranche_qf
          WHERE etablissement_id = $1 AND grille_version_id = $2 ORDER BY qf_min_centimes",
    )
    .bind(etab)
    .bind(version)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| TrancheQfDto {
            id: r.0,
            code: r.1,
            libelle: r.2,
            qf_min_centimes: r.3,
            qf_max_centimes: r.4,
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct ArticleRow {
    id: Uuid,
    code: String,
    libelle: String,
    categorie: String,
    compte_produit: String,
    actif: bool,
    nature_legale: Option<String>,
    regime_tva: Option<String>,
    taux_tva_bps: Option<i32>,
    unite: Option<String>,
    mode: String,
    remboursable: bool,
    soumis_reductions: bool,
    soumis_remise_ordre: bool,
    modele_code: Option<String>,
    qualifie_le: Option<DateTime<Utc>>,
    nb_lignes: i64,
}

impl ArticleRow {
    fn dto(self) -> ArticleQualifieDto {
        let qualifie = self.qualifie_le.is_some();
        let a_valider = !qualifie && self.modele_code.is_some() && self.nature_legale.is_some();
        ArticleQualifieDto {
            proposition: (!qualifie).then(|| proposition_qualification(&self.categorie)),
            id: self.id,
            code: self.code,
            libelle: self.libelle,
            categorie: self.categorie,
            compte_produit: self.compte_produit,
            actif: self.actif,
            nature_legale: self.nature_legale,
            regime_tva: self.regime_tva,
            taux_tva_bps: self.taux_tva_bps,
            unite: self.unite,
            mode: self.mode,
            remboursable: self.remboursable,
            soumis_reductions: self.soumis_reductions,
            soumis_remise_ordre: self.soumis_remise_ordre,
            qualifie,
            a_valider,
            modele_code: self.modele_code,
            qualifie_le: self.qualifie_le,
            nb_lignes: self.nb_lignes,
        }
    }
}

const SELECT_ARTICLE: &str = "SELECT a.id, a.code, a.libelle, a.categorie, a.compte_produit, a.actif, a.nature_legale,
        a.regime_tva, a.taux_tva_bps, a.unite, a.mode, a.remboursable, a.soumis_reductions, a.soumis_remise_ordre,
        a.modele_code, a.qualifie_le,
        (SELECT count(*) FROM ent_fac_ligne l WHERE l.etablissement_id = a.etablissement_id
            AND l.article_id = a.id) AS nb_lignes
   FROM ent_fin_article a";

async fn articles_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Option<Uuid>,
) -> Result<Vec<ArticleQualifieDto>, EntDbError> {
    let sql = format!(
        "{SELECT_ARTICLE} WHERE a.etablissement_id = $1 AND ($2::UUID IS NULL OR a.id = $2)
          ORDER BY a.categorie, a.code"
    );
    let rows: Vec<ArticleRow> = sqlx::query_as(&sql)
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
    Ok(rows.into_iter().map(ArticleRow::dto).collect())
}

async fn nb_non_qualifies_tx(conn: &mut PgConnection, etab: Uuid) -> Result<i64, EntDbError> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_fin_article WHERE etablissement_id = $1 AND actif AND qualifie_le IS NULL",
    )
    .bind(etab)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(n)
}

/// `(exiger la qualification à l'émission, second regard sur le vote)` — défauts prudents si le paramétrage
/// n'existe pas encore.
async fn reglages_tx(conn: &mut PgConnection, etab: Uuid) -> Result<(bool, bool), EntDbError> {
    let r: Option<(bool, bool)> = sqlx::query_as(
        "SELECT exiger_qualification_articles, grille_second_regard FROM ent_fin_parametres
          WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    // ligne absente : établissement créé après la 523, qualification exigée (M7)
    Ok(r.unwrap_or((true, true)))
}

/// Alertes de réglage visibles de la direction : qualification non exigée (établissement antérieur à la 523,
/// à activer avant la rentrée 2027) ; second regard désactivé (auteur et date).
async fn alertes_reglages_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    exiger: bool,
    second: bool,
) -> Result<Vec<String>, EntDbError> {
    let mut v = Vec::new();
    if !exiger {
        v.push(
            "qualification des articles non exigée à l'émission : à activer avant la rentrée 2027"
                .to_string(),
        );
    }
    if !second {
        let r: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
            "SELECT second_regard_desactive_le FROM ent_fin_parametres WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let quand = r.and_then(|r| r.0).map_or_else(String::new, |d| {
            format!(" depuis le {}", d.format("%d/%m/%Y"))
        });
        v.push(format!(
            "second regard sur le vote des grilles désactivé{quand} : la direction vote seule"
        ));
    }
    Ok(v)
}

// -- Points d'entrée utilisés par `finances.rs` (lecture seule) ---------------------------------------

/// Faut-il refuser l'émission d'une pièce dont un article n'est pas qualifié (MIG-6) ?
pub(crate) async fn exiger_qualification_tx(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<bool, EntDbError> {
    Ok(reglages_tx(conn, etab).await?.0)
}

/// Nombre de lignes d'une pièce dont l'article n'est pas qualifié ; une ligne sans article compte comme
/// non qualifiée (M7).
pub(crate) async fn articles_non_qualifies_de_piece_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    facture: Uuid,
) -> Result<i64, EntDbError> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_fac_ligne l
           LEFT JOIN ent_fin_article a ON a.id = l.article_id AND a.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.facture_id = $2 AND (l.article_id IS NULL OR a.qualifie_le IS NULL)",
    )
    .bind(etab)
    .bind(facture)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(n)
}

/// L'année a-t-elle une grille votée ou en vigueur ? (alors on ne saisit plus de tarif historique, R-F10)
pub(crate) async fn annee_a_grille_votee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
) -> Result<bool, EntDbError> {
    let (b,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_fin_grille_version WHERE etablissement_id = $1
                          AND annee_scolaire = $2 AND statut IN ('votee', 'en_vigueur'))",
    )
    .bind(etab)
    .bind(annee)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(b)
}

fn refus(m: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Interdit, m)
}

// ---------------------------------------------------------------------------
// Commandes du service
// ---------------------------------------------------------------------------

impl PgEnt {
    // -- Catalogue ------------------------------------------------------------------------------

    /// FAC-103 : les modèles d'articles fournis (lecture seule, aucune écriture).
    pub async fn modeles_articles_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<ModeleArticleDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        Ok(modeles_articles())
    }

    /// Catalogue avec l'état de qualification et la proposition pour les articles non qualifiés (MIG-6).
    pub async fn articles_qualifies_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<ArticleQualifieDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let v = articles_tx(&mut tx, etab, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    /// FAC-103 : crée un article depuis un modèle (nature, TVA, unité, mode obligatoires) : **qualifié** d'emblée,
    /// sauf un modèle taxable, « à valider » par l'expert-comptable (M7).
    pub async fn article_depuis_modele_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ArticleDepuisModeleBody,
    ) -> Result<ArticleQualifieDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let m = modeles_articles()
            .into_iter()
            .find(|m| m.code == b.modele_code.trim())
            .ok_or(invalide("modèle d'article inconnu"))?;
        let code = opt_texte(&b.code, 30)?.unwrap_or_else(|| m.code.clone());
        if !code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(invalide("code d'article : lettres, chiffres, _ et -"));
        }
        let libelle = match opt_texte(&b.libelle, 120)? {
            Some(l) => l,
            None => m.libelle.clone(),
        };
        let compte = opt_texte(&b.compte_produit, 20)?.unwrap_or_else(|| m.compte_produit.clone());
        if !compte.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(invalide("compte de produit : lettres et chiffres"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fin_article (id, etablissement_id, code, libelle, categorie, compte_produit, actif,
                    created_by, nature_legale, regime_tva, taux_tva_bps, unite, mode, remboursable,
                    soumis_reductions, soumis_remise_ordre, modele_code, qualifie_le, qualifie_par)
             VALUES ($1, $2, $3, $4, $5, $6, true, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                    CASE WHEN $17 THEN NULL ELSE now() END, CASE WHEN $17 THEN NULL ELSE $7 END)",
        )
        .bind(id)
        .bind(etab)
        .bind(&code)
        .bind(&libelle)
        .bind(&m.categorie)
        .bind(&compte)
        .bind(actor)
        .bind(&m.nature_legale)
        .bind(&m.regime_tva)
        .bind(m.taux_tva_bps)
        .bind(&m.unite)
        .bind(&m.mode)
        .bind(m.remboursable)
        .bind(m.soumis_reductions)
        .bind(m.soumis_remise_ordre)
        .bind(&m.code)
        // un modèle taxable reste « à valider » tant que l'expert-comptable ne l'a pas validé (M7)
        .bind(m.regime_tva == "taxable")
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if est_doublon(&e) {
                conflit("un article porte déjà ce code")
            } else {
                db(e)
            }
        })?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "article.creer_depuis_modele",
                "article",
            )
            .objet(id)
            .detail(&format!(
                "article {code} depuis le modèle {} ({}, TVA {}){}",
                m.code,
                m.nature_legale,
                m.regime_tva,
                if m.regime_tva == "taxable" {
                    " : à valider par l'expert-comptable"
                } else {
                    ""
                }
            )),
        )
        .await?;
        publier_finance(
            &mut tx,
            etab,
            types::FACTURATION_ARTICLE_ENREGISTRE,
            AggregateKind::Facture,
            id,
            None,
            json!({ "article_id": id }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let dto = articles_tx(&mut tx, etab, Some(id))
            .await?
            .pop()
            .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// MIG-6 : valide (ou corrige) la nature légale et la TVA d'un article. Un article qualifié qui porte des
    /// lignes ne change plus (déclencheur de la 523) : on en crée un nouveau.
    pub async fn qualifier_article_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &QualifierArticleBody,
    ) -> Result<ArticleQualifieDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let (categorie, deja, nb_lignes): (String, Option<DateTime<Utc>>, i64) = sqlx::query_as(
            "SELECT a.categorie, a.qualifie_le,
                    (SELECT count(*) FROM ent_fac_ligne l WHERE l.etablissement_id = a.etablissement_id
                        AND l.article_id = a.id)
               FROM ent_fin_article a WHERE a.etablissement_id = $1 AND a.id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        valider_qualification(&categorie, b)?;
        if deja.is_some() && nb_lignes > 0 {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "article déjà utilisé sur des pièces : nature et TVA figées, créer un nouvel article",
            ));
        }
        sqlx::query(
            "UPDATE ent_fin_article SET nature_legale = $3, regime_tva = $4, taux_tva_bps = $5, unite = $6,
                    mode = COALESCE($7, mode), remboursable = COALESCE($8, remboursable),
                    soumis_reductions = COALESCE($9, soumis_reductions),
                    soumis_remise_ordre = COALESCE($10, soumis_remise_ordre),
                    qualifie_le = now(), qualifie_par = $11
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&b.nature_legale)
        .bind(&b.regime_tva)
        .bind(b.taux_tva_bps)
        .bind(&b.unite)
        .bind(&b.mode)
        .bind(b.remboursable)
        .bind(b.soumis_reductions)
        .bind(b.soumis_remise_ordre)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "article.qualifier",
                "article",
            )
            .objet(id)
            .detail(&format!(
                "nature {}, TVA {}{}",
                b.nature_legale,
                b.regime_tva,
                if b.regime_tva == "taxable" {
                    " : à valider par l'expert-comptable"
                } else {
                    ""
                }
            )),
        )
        .await?;
        publier_finance(
            &mut tx,
            etab,
            types::FACTURATION_ARTICLE_QUALIFIE,
            AggregateKind::Facture,
            id,
            None,
            json!({ "article_id": id }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let dto = articles_tx(&mut tx, etab, Some(id))
            .await?
            .pop()
            .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// FAC-106 : usage d'un article (lignes et montants nets par année, aucune personne).
    pub async fn usage_article_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ArticleUsageDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_fin_article WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let rows: Vec<(Option<String>, i64, i64)> = sqlx::query_as(
            "SELECT f.annee_scolaire, count(*)::BIGINT, COALESCE(sum(l.quantite::BIGINT * l.montant_unitaire_centimes), 0)::BIGINT
               FROM ent_fac_ligne l
               JOIN ent_fac_facture f ON f.id = l.facture_id AND f.etablissement_id = l.etablissement_id
              WHERE l.etablissement_id = $1 AND l.article_id = $2 AND f.statut IN ('emise', 'soldee')
              GROUP BY f.annee_scolaire ORDER BY f.annee_scolaire",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let par_annee: Vec<UsageAnneeDto> = rows
            .into_iter()
            .map(|r| UsageAnneeDto {
                annee_scolaire: r.0.unwrap_or_else(|| "sans année".into()),
                nb_lignes: r.1,
                montant_net_centimes: r.2,
            })
            .collect();
        Ok(ArticleUsageDto {
            article_id: id,
            nb_lignes: par_annee.iter().map(|a| a.nb_lignes).sum(),
            montant_net_centimes: par_annee.iter().map(|a| a.montant_net_centimes).sum(),
            par_annee,
        })
    }

    pub async fn reglages_grilles_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<ReglagesGrillesDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let (exiger, second) = reglages_tx(&mut tx, etab).await?;
        let n = nb_non_qualifies_tx(&mut tx, etab).await?;
        let alertes = alertes_reglages_tx(&mut tx, etab, exiger, second).await?;
        tx.commit().await.map_err(db)?;
        Ok(ReglagesGrillesDto {
            exiger_qualification_articles: exiger,
            grille_second_regard: second,
            articles_non_qualifies: n,
            alertes,
        })
    }

    /// Règle l'exigence de qualification à l'émission et le second regard sur le vote (droit `parametrer`).
    /// Désactiver le second regard exige **aussi** `voter`, est refusé tant qu'une grille est proposée, est
    /// journalisé et reste signalé à la direction (D-069, règle 3).
    pub async fn definir_reglages_grilles_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ReglagesGrillesBody,
    ) -> Result<ReglagesGrillesDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationParametrer)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let (exiger, second) = reglages_tx(&mut tx, etab).await?;
        let exiger2 = b.exiger_qualification_articles.unwrap_or(exiger);
        let second2 = b.grille_second_regard.unwrap_or(second);
        let desactive = second && !second2;
        if desactive {
            self.fin_exiger(etab, actor, role, Action::FacturationVoter)
                .await?;
            let (proposee,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_fin_grille_version
                                 WHERE etablissement_id = $1 AND statut = 'proposee')",
            )
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if proposee {
                return Err(EntDbError::Refus(
                    RefusMetier::Conflit,
                    "une grille est proposée : le second regard ne se désactive pas pendant le vote",
                ));
            }
        }
        sqlx::query(
            "INSERT INTO ent_fin_parametres (etablissement_id, exiger_qualification_articles, grille_second_regard,
                    second_regard_desactive_le, second_regard_desactive_par, updated_by)
             VALUES ($1, $2, $3, CASE WHEN $3 THEN NULL ELSE now() END, CASE WHEN $3 THEN NULL ELSE $4 END, $4)
             ON CONFLICT (etablissement_id) DO UPDATE SET
                    exiger_qualification_articles = EXCLUDED.exiger_qualification_articles,
                    grille_second_regard = EXCLUDED.grille_second_regard,
                    second_regard_desactive_le = CASE WHEN EXCLUDED.grille_second_regard THEN NULL
                         WHEN $5 THEN now() ELSE ent_fin_parametres.second_regard_desactive_le END,
                    second_regard_desactive_par = CASE WHEN EXCLUDED.grille_second_regard THEN NULL
                         WHEN $5 THEN $4 ELSE ent_fin_parametres.second_regard_desactive_par END,
                    updated_by = EXCLUDED.updated_by, updated_at = now()",
        )
        .bind(etab)
        .bind(exiger2)
        .bind(second2)
        .bind(actor)
        .bind(desactive)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                if desactive {
                    "grille.second_regard_desactive"
                } else {
                    "grille.reglages"
                },
                "parametres",
            )
            .detail(&format!(
                "qualification exigée : {exiger2} ; second regard sur le vote : {second2}"
            )),
        )
        .await?;
        let n = nb_non_qualifies_tx(&mut tx, etab).await?;
        let alertes = alertes_reglages_tx(&mut tx, etab, exiger2, second2).await?;
        tx.commit().await.map_err(db)?;
        Ok(ReglagesGrillesDto {
            exiger_qualification_articles: exiger2,
            grille_second_regard: second2,
            articles_non_qualifies: n,
            alertes,
        })
    }

    // -- Versions de grille ---------------------------------------------------------------------

    pub async fn grilles_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        annee: Option<String>,
    ) -> Result<Vec<GrilleVersionDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let sql = format!(
            "{SELECT_VERSION} WHERE g.etablissement_id = $1 AND ($2::TEXT IS NULL OR g.annee_scolaire = $2)
              ORDER BY g.annee_scolaire DESC, g.numero DESC"
        );
        let rows: Vec<VersionRow> = sqlx::query_as(&sql)
            .bind(etab)
            .bind(annee)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows.iter().map(VersionRow::dto).collect())
    }

    pub async fn grille_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<GrilleDetailDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, id, false).await?;
        let tarifs = tarifs_version_tx(&mut tx, etab, id, None).await?;
        let tranches = tranches_version_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(GrilleDetailDto {
            version: v.dto(),
            tarifs,
            tranches,
        })
    }

    /// FAC-116 : crée une version brouillon, vide, copiée d'une autre version, ou reprenant les tarifs
    /// historiques de l'année (avant les versions, MIG-6).
    pub async fn creer_grille_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &CreerGrilleBody,
    ) -> Result<GrilleVersionDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let annee = annee_valide(&b.annee_scolaire)?;
        if b.copier_de.is_some() && b.reprendre_historique == Some(true) {
            return Err(invalide(
                "copier une version ou reprendre l'historique, pas les deux",
            ));
        }
        let mut tx = self.fin_tx(etab).await?;
        // numéro : verrou d'année pour sérialiser deux créations simultanées
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("fin:grille:{etab}:{annee}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if let Some(src) = b.copier_de {
            version_tx(&mut tx, etab, src, false).await?;
        }
        let (numero,): (i32,) = sqlx::query_as(
            "SELECT COALESCE(max(numero), 0)::INT + 1 FROM ent_fin_grille_version
              WHERE etablissement_id = $1 AND annee_scolaire = $2",
        )
        .bind(etab)
        .bind(&annee)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let libelle =
            opt_texte(&b.libelle, 120)?.unwrap_or_else(|| format!("Grille {annee} v{numero}"));
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fin_grille_version (id, etablissement_id, annee_scolaire, numero, libelle, cree_par)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(etab)
        .bind(&annee)
        .bind(numero)
        .bind(&libelle)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let mut copies = 0;
        if let Some(src) = b.copier_de {
            copies = copier_contenu_tx(&mut tx, etab, src, id, &annee, actor).await?;
        } else if b.reprendre_historique == Some(true) {
            let r = sqlx::query(
                "INSERT INTO ent_fin_tarif (id, etablissement_id, annee_scolaire, article_id, niveau, regime,
                        qf_min_centimes, qf_max_centimes, montant_annuel_centimes, actif, created_by,
                        grille_version_id, tarif_par_defaut)
                 SELECT gen_random_uuid(), etablissement_id, annee_scolaire, article_id, niveau, regime,
                        qf_min_centimes, qf_max_centimes, montant_annuel_centimes, true, $3, $4, false
                   FROM ent_fin_tarif
                  WHERE etablissement_id = $1 AND annee_scolaire = $2 AND actif AND grille_version_id IS NULL",
            )
            .bind(etab)
            .bind(&annee)
            .bind(actor)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            copies = r.rows_affected() as i64;
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "grille.creer",
                "grille",
            )
            .objet(id)
            .detail(&format!(
                "grille {annee} v{numero} ({copies} tarifs repris)"
            )),
        )
        .await?;
        publier_finance(
            &mut tx,
            etab,
            types::FACTURATION_GRILLE_CREEE,
            AggregateKind::Facture,
            id,
            None,
            json!({ "grille_id": id, "annee_scolaire": annee, "numero": numero }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let v = version_tx(&mut tx, etab, id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(v.dto())
    }

    /// FAC-117 : ajoute un tarif à un brouillon.
    pub async fn ajouter_tarif_grille_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        b: &TarifGrilleBody,
    ) -> Result<TarifGrilleDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let (niveau, regime) = valider_tarif(b)?;
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, true).await?;
        exiger_brouillon(&v)?;
        article_actif_tx(&mut tx, etab, b.article_id).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fin_tarif (id, etablissement_id, annee_scolaire, article_id, niveau, regime,
                    qf_min_centimes, qf_max_centimes, montant_annuel_centimes, created_by, grille_version_id,
                    tarif_par_defaut)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(id)
        .bind(etab)
        .bind(&v.annee_scolaire)
        .bind(b.article_id)
        .bind(&niveau)
        .bind(&regime)
        .bind(b.qf_min_centimes)
        .bind(b.qf_max_centimes)
        .bind(b.montant_annuel_centimes)
        .bind(actor)
        .bind(version)
        .bind(b.tarif_par_defaut.unwrap_or(false))
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if est_doublon(&e) {
                conflit("cet article a déjà un tarif par défaut dans cette grille")
            } else {
                db(e)
            }
        })?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "grille.tarif_ajouter",
                "tarif",
            )
            .objet(id)
            .montant(b.montant_annuel_centimes)
            .detail(&format!("grille {} v{}", v.annee_scolaire, v.numero)),
        )
        .await?;
        let dto = tarifs_version_tx(&mut tx, etab, version, Some(id))
            .await?
            .pop()
            .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// FAC-117 : modifie un tarif d'un brouillon.
    pub async fn modifier_tarif_grille_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        tarif: Uuid,
        b: &TarifGrilleBody,
    ) -> Result<TarifGrilleDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let (niveau, regime) = valider_tarif(b)?;
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, true).await?;
        exiger_brouillon(&v)?;
        article_actif_tx(&mut tx, etab, b.article_id).await?;
        let r = sqlx::query(
            "UPDATE ent_fin_tarif SET article_id = $4, niveau = $5, regime = $6, qf_min_centimes = $7,
                    qf_max_centimes = $8, montant_annuel_centimes = $9, tarif_par_defaut = $10, modifie_par = $11
              WHERE etablissement_id = $1 AND grille_version_id = $2 AND id = $3",
        )
        .bind(etab)
        .bind(version)
        .bind(tarif)
        .bind(b.article_id)
        .bind(&niveau)
        .bind(&regime)
        .bind(b.qf_min_centimes)
        .bind(b.qf_max_centimes)
        .bind(b.montant_annuel_centimes)
        .bind(b.tarif_par_defaut.unwrap_or(false))
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if est_doublon(&e) {
                conflit("cet article a déjà un tarif par défaut dans cette grille")
            } else {
                db(e)
            }
        })?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "grille.tarif_modifier",
                "tarif",
            )
            .objet(tarif)
            .montant(b.montant_annuel_centimes)
            .detail(&format!("grille {} v{}", v.annee_scolaire, v.numero)),
        )
        .await?;
        let dto = tarifs_version_tx(&mut tx, etab, version, Some(tarif))
            .await?
            .pop()
            .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Retire un tarif d'un brouillon (une grille figée ne perd jamais un tarif : déclencheur).
    pub async fn retirer_tarif_grille_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        tarif: Uuid,
    ) -> Result<(), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, true).await?;
        exiger_brouillon(&v)?;
        let r = sqlx::query(
            "DELETE FROM ent_fin_tarif WHERE etablissement_id = $1 AND grille_version_id = $2 AND id = $3",
        )
        .bind(etab)
        .bind(version)
        .bind(tarif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "grille.tarif_retirer",
                "tarif",
            )
            .objet(tarif)
            .detail(&format!("grille {} v{}", v.annee_scolaire, v.numero)),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// FAC-118 : remplace les tranches de quotient d'un brouillon (contrôlées d'un bloc).
    pub async fn definir_tranches_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        b: &TranchesBody,
    ) -> Result<Vec<TrancheQfDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        valider_tranches(&b.tranches).map_err(invalide)?;
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, true).await?;
        exiger_brouillon(&v)?;
        sqlx::query(
            "DELETE FROM ent_fin_tranche_qf WHERE etablissement_id = $1 AND grille_version_id = $2",
        )
        .bind(etab)
        .bind(version)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for t in &b.tranches {
            sqlx::query(
                "INSERT INTO ent_fin_tranche_qf (id, etablissement_id, grille_version_id, code, libelle,
                        qf_min_centimes, qf_max_centimes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(version)
            .bind(t.code.trim())
            .bind(t.libelle.trim())
            .bind(t.qf_min_centimes)
            .bind(t.qf_max_centimes)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "grille.tranches",
                "grille",
            )
            .objet(version)
            .detail(&format!(
                "{} tranches pour la grille {} v{}",
                b.tranches.len(),
                v.annee_scolaire,
                v.numero
            )),
        )
        .await?;
        let t = tranches_version_tx(&mut tx, etab, version).await?;
        tx.commit().await.map_err(db)?;
        Ok(t)
    }

    /// FAC-120 : hausse en pourcentage ou en montant avec arrondi, sur un brouillon ; l'aperçu n'écrit rien.
    pub async fn appliquer_hausse_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        b: &HausseBody,
    ) -> Result<ApercuHausseDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        if !(-10_000_000..=10_000_000).contains(&b.valeur) {
            return Err(invalide("hausse hors bornes"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, true).await?;
        exiger_brouillon(&v)?;
        let tarifs = tarifs_version_tx(&mut tx, etab, version, None).await?;
        let mut lignes = Vec::new();
        for t in tarifs
            .iter()
            .filter(|t| b.article_id.is_none_or(|a| a == t.article_id))
            .filter(|t| {
                b.article_id.is_some()
                    || b.mode != ModeHausse::Montant
                    || !categorie_bourse_ou_reduction(&t.categorie)
            })
        {
            let apres =
                montant_apres_hausse(t.montant_annuel_centimes, b.mode, b.valeur, b.arrondi)?;
            lignes.push((
                t.id,
                t.article_code.clone(),
                t.niveau.clone(),
                t.regime.clone(),
                t.montant_annuel_centimes,
                apres,
            ));
        }
        if b.article_id.is_some() && lignes.is_empty() {
            return Err(introuvable());
        }
        if !b.apercu {
            for l in &lignes {
                if l.4 != l.5 {
                    sqlx::query(
                        "UPDATE ent_fin_tarif SET montant_annuel_centimes = $4, modifie_par = $5
                          WHERE etablissement_id = $1 AND grille_version_id = $2 AND id = $3",
                    )
                    .bind(etab)
                    .bind(version)
                    .bind(l.0)
                    .bind(l.5)
                    .bind(actor)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                }
            }
            journaliser_tx(
                &mut tx,
                &Acte::new(
                    etab,
                    Some(actor),
                    role,
                    Chemin::Finances,
                    "grille.hausse",
                    "grille",
                )
                .objet(version)
                .detail(&format!(
                    "hausse {} {} sur {} tarifs de la grille {} v{}",
                    match b.mode {
                        ModeHausse::Pourcentage => "pourcentage (pdb)",
                        ModeHausse::Montant => "montant (centimes)",
                    },
                    b.valeur,
                    lignes.len(),
                    v.annee_scolaire,
                    v.numero
                )),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(ApercuHausseDto {
            apercu: b.apercu,
            lignes: lignes
                .into_iter()
                .map(|l| LigneHausseDto {
                    tarif_id: l.0,
                    article_code: l.1,
                    niveau: l.2,
                    regime: l.3,
                    avant_centimes: l.4,
                    apres_centimes: l.5,
                })
                .collect(),
        })
    }

    /// Contrôles d'une version (lecture) : anomalies bloquantes et avertissements.
    pub async fn controles_grille_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
    ) -> Result<ControlesGrilleDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        version_tx(&mut tx, etab, version, false).await?;
        let c = controles_tx(&mut tx, etab, version).await?;
        tx.commit().await.map_err(db)?;
        Ok(c)
    }

    /// FAC-125 : propose la grille au conseil d'administration (gel du brouillon).
    pub async fn proposer_grille_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
    ) -> Result<GrilleVersionDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, true).await?;
        exiger_brouillon(&v)?;
        let c = controles_tx(&mut tx, etab, version).await?;
        if !c.proposable {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "la grille a des anomalies bloquantes : voir les contrôles",
            ));
        }
        sqlx::query(
            "UPDATE ent_fin_grille_version SET statut = 'proposee', propose_le = now(), propose_par = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(version)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "grille.proposer",
                "grille",
            )
            .objet(version)
            .detail(&format!("grille {} v{}", v.annee_scolaire, v.numero)),
        )
        .await?;
        publier_finance(
            &mut tx,
            etab,
            types::FACTURATION_GRILLE_PROPOSEE,
            AggregateKind::Facture,
            version,
            None,
            json!({ "grille_id": version, "annee_scolaire": v.annee_scolaire, "numero": v.numero }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let v = version_tx(&mut tx, etab, version, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(v.dto())
    }

    /// Retour motivé d'une grille proposée à l'état de brouillon (le conseil demande des changements).
    pub async fn retour_brouillon_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        b: &RetourBrouillonBody,
    ) -> Result<GrilleVersionDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let motif = texte(&b.motif, 500, "motif obligatoire (500 caractères)")?;
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, true).await?;
        if v.statut != "proposee" {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "seule une grille proposée revient en brouillon (une grille votée ne se modifie plus)",
            ));
        }
        sqlx::query(
            "UPDATE ent_fin_grille_version SET statut = 'brouillon', propose_le = NULL, propose_par = NULL
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(version)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "grille.retour_brouillon",
                "grille",
            )
            .objet(version)
            .motif(&motif),
        )
        .await?;
        let v = version_tx(&mut tx, etab, version, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(v.dto())
    }

    /// Abandonne un brouillon ou une proposition (jamais supprimée : l'historique reste).
    pub async fn abandonner_grille_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        b: &RetourBrouillonBody,
    ) -> Result<GrilleVersionDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let motif = texte(&b.motif, 500, "motif obligatoire (500 caractères)")?;
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, true).await?;
        if !matches!(v.statut.as_str(), "brouillon" | "proposee") {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "seul un brouillon ou une proposition peut être abandonné",
            ));
        }
        sqlx::query(
            "UPDATE ent_fin_grille_version SET statut = 'abandonnee' WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(version)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "grille.abandonner",
                "grille",
            )
            .objet(version)
            .motif(&motif),
        )
        .await?;
        let v = version_tx(&mut tx, etab, version, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(v.dto())
    }

    /// FAC-126 ★ : enregistre le vote (date, référence, pièce de la délibération). Version figée.
    /// R-F23 : avec le second regard (défaut), celui qui a proposé la grille ne l'enregistre pas comme votée.
    pub async fn voter_grille_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        b: &VoteGrilleBody,
    ) -> Result<GrilleVersionDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationVoter)
            .await?;
        let jour = self.jour_etablissement(etab).await?;
        self.voter_grille_au_jour(etab, actor, role, version, b, jour)
            .await
    }

    /// Vote à un jour de référence donné (les tests fixent le jour ; `voter_grille_for_actor` lit celui de
    /// l'établissement). Règles 2, 3 et 5 de D-069 : droit avant existence, vote jamais daté dans le futur,
    /// second regard = une **autre personne** que l'auteur réel (créateur, proposant, auteurs de tarifs).
    pub(crate) async fn voter_grille_au_jour(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        b: &VoteGrilleBody,
        jour: NaiveDate,
    ) -> Result<GrilleVersionDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationVoter)
            .await?;
        let reference = texte(
            &b.reference,
            200,
            "référence de la délibération obligatoire (200 caractères)",
        )?;
        if !(2000..=2100).contains(&b.vote_le.year()) {
            return Err(invalide("date de vote invraisemblable"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, true).await?;
        if v.statut != "proposee" {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "seule une grille proposée peut être votée",
            ));
        }
        // le futur se contrôle après l'existence de la grille (ordre : personnel, droit, existence)
        if b.vote_le > jour {
            return Err(invalide(
                "date de vote dans le futur : la délibération a eu lieu au plus tard aujourd'hui",
            ));
        }
        let (_, second) = reglages_tx(&mut tx, etab).await?;
        if second && auteurs_grille_tx(&mut tx, etab, &v).await?.contains(&actor) {
            return Err(refus(
                "second regard : l'auteur de la grille (créateur, proposant ou auteur d'un tarif) ne l'enregistre pas comme votée",
            ));
        }
        if let Some(doc) = b.document_id {
            let (existe,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(doc)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !existe {
                return Err(introuvable());
            }
        }
        sqlx::query(
            "UPDATE ent_fin_grille_version SET statut = 'votee', vote_le = $3, vote_reference = $4,
                    vote_document_id = $5, vote_par = $6
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(version)
        .bind(b.vote_le)
        .bind(&reference)
        .bind(b.document_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "grille.voter",
                "grille",
            )
            .objet(version)
            .detail(&format!(
                "grille {} v{} votée le {}",
                v.annee_scolaire, v.numero, b.vote_le
            )),
        )
        .await?;
        publier_finance(
            &mut tx,
            etab,
            types::FACTURATION_GRILLE_VOTEE,
            AggregateKind::Facture,
            version,
            None,
            json!({ "grille_id": version, "annee_scolaire": v.annee_scolaire, "numero": v.numero,
                    "vote_le": b.vote_le }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let v = version_tx(&mut tx, etab, version, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(v.dto())
    }

    /// FAC-127 : met une grille votée en vigueur à une date. **Une grille ne tarifie qu'à partir de sa date**
    /// (règle 2) : la version appliquée jusque-là reste applicable jusqu'à la veille, puis est close ; toute autre
    /// version votée de l'année est « supplantée » et ne peut plus être mise en vigueur (M5). Date : vote ≤ date,
    /// dans l'année scolaire visée, jamais avant la date d'une version déjà appliquée. Les contrôles de
    /// continuité (règle 4) sont rejoués avant la mise en vigueur.
    pub async fn mettre_en_vigueur_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        b: &MiseEnVigueurBody,
    ) -> Result<GrilleVersionDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, true).await?;
        if v.statut != "votee" {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "seule une grille votée peut être mise en vigueur",
            ));
        }
        let (premier, dernier) = bornes_annee(&v.annee_scolaire)?;
        if b.du < premier || b.du > dernier {
            return Err(invalide(
                "entrée en vigueur hors de l'année scolaire visée par la grille",
            ));
        }
        if v.vote_le.is_some_and(|vote| b.du < vote) {
            return Err(invalide(
                "entrée en vigueur antérieure au vote de la grille",
            ));
        }
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("fin:grille:{etab}:{}", v.annee_scolaire))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let c = controles_tx(&mut tx, etab, version).await?;
        if !c.proposable {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "la grille perdrait des tarifs de la version précédente : voir les contrôles",
            ));
        }
        // versions déjà appliquées (en vigueur ou closes) : la plus récente borne la date
        let appliquees: Vec<(Uuid, String, NaiveDate)> = sqlx::query_as(
            "SELECT id, statut, en_vigueur_du FROM ent_fin_grille_version
              WHERE etablissement_id = $1 AND annee_scolaire = $2 AND id <> $3
                AND statut IN ('en_vigueur', 'close') AND en_vigueur_du IS NOT NULL
              ORDER BY en_vigueur_du DESC, numero DESC FOR UPDATE",
        )
        .bind(etab)
        .bind(&v.annee_scolaire)
        .bind(version)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if appliquees.first().is_some_and(|a| b.du < a.2) {
            return Err(invalide(
                "entrée en vigueur antérieure à celle d'une version déjà appliquée",
            ));
        }
        for (id, statut, _) in &appliquees {
            if statut == "en_vigueur" {
                sqlx::query(
                    "UPDATE ent_fin_grille_version SET statut = 'close', close_le = now()
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        let supplantees: Vec<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_fin_grille_version SET statut = 'supplantee', close_le = now()
              WHERE etablissement_id = $1 AND annee_scolaire = $2 AND id <> $3 AND statut = 'votee'
             RETURNING id",
        )
        .bind(etab)
        .bind(&v.annee_scolaire)
        .bind(version)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_fin_grille_version SET statut = 'en_vigueur', en_vigueur_du = $3, en_vigueur_par = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(version)
        .bind(b.du)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "grille.en_vigueur",
                "grille",
            )
            .objet(version)
            .detail(&format!(
                "grille {} v{} en vigueur au {}{}",
                v.annee_scolaire,
                v.numero,
                b.du,
                if supplantees.is_empty() {
                    String::new()
                } else {
                    format!(
                        " ({} autre(s) version(s) votée(s) supplantée(s))",
                        supplantees.len()
                    )
                }
            )),
        )
        .await?;
        publier_finance(
            &mut tx,
            etab,
            types::FACTURATION_GRILLE_EN_VIGUEUR,
            AggregateKind::Facture,
            version,
            None,
            json!({ "grille_id": version, "annee_scolaire": v.annee_scolaire, "numero": v.numero,
                    "en_vigueur_du": b.du, "effet_retroactif": v.effet_retroactif }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let v = version_tx(&mut tx, etab, version, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(v.dto())
    }

    /// FAC-128 ★ : corrige une grille votée ou en vigueur **par une nouvelle version** (copie en brouillon,
    /// motif obligatoire). L'effet rétroactif est seulement **marqué** : les compléments et avoirs se proposent
    /// au lot F4 ; rien n'est modifié sur les pièces émises.
    pub async fn corriger_grille_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
        b: &CorrectionGrilleBody,
    ) -> Result<GrilleVersionDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationVoter)
            .await?;
        let motif = texte(&b.motif, 500, "motif obligatoire (500 caractères)")?;
        let mut tx = self.fin_tx(etab).await?;
        let src = version_tx(&mut tx, etab, version, true).await?;
        if !matches!(src.statut.as_str(), "votee" | "en_vigueur") {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "seule une grille votée ou en vigueur se corrige par une nouvelle version",
            ));
        }
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("fin:grille:{etab}:{}", src.annee_scolaire))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let (en_cours,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_fin_grille_version WHERE etablissement_id = $1 AND remplace_id = $2
                              AND statut IN ('brouillon', 'proposee', 'votee'))",
        )
        .bind(etab)
        .bind(version)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if en_cours {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "une correction de cette grille est déjà en cours",
            ));
        }
        let (numero,): (i32,) = sqlx::query_as(
            "SELECT COALESCE(max(numero), 0)::INT + 1 FROM ent_fin_grille_version
              WHERE etablissement_id = $1 AND annee_scolaire = $2",
        )
        .bind(etab)
        .bind(&src.annee_scolaire)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fin_grille_version (id, etablissement_id, annee_scolaire, numero, libelle, remplace_id,
                    motif, effet_retroactif, cree_par)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(etab)
        .bind(&src.annee_scolaire)
        .bind(numero)
        .bind(format!("Correction de la grille v{}", src.numero))
        .bind(version)
        .bind(&motif)
        .bind(b.effet_retroactif)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let n = copier_contenu_tx(&mut tx, etab, version, id, &src.annee_scolaire, actor).await?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "grille.corriger",
                "grille",
            )
            .objet(id)
            .motif(&motif)
            .detail(&format!(
                "correction de la grille {} v{} → v{numero} ({n} tarifs repris){}",
                src.annee_scolaire,
                src.numero,
                if b.effet_retroactif {
                    ", effet rétroactif proposé"
                } else {
                    ""
                }
            )),
        )
        .await?;
        publier_finance(
            &mut tx,
            etab,
            types::FACTURATION_GRILLE_CREEE,
            AggregateKind::Facture,
            id,
            None,
            json!({ "grille_id": id, "annee_scolaire": src.annee_scolaire, "numero": numero,
                    "remplace_id": version }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let v = version_tx(&mut tx, etab, id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(v.dto())
    }

    /// FAC-129 : publie la grille (votée ou en vigueur) : libellés et montants seulement, **aucune donnée
    /// personnelle**. La date de publication est posée une fois ; l'appel suivant rend la même publication.
    pub async fn publier_grille_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
    ) -> Result<PublicationGrilleDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationPublier)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, true).await?;
        if !matches!(v.statut.as_str(), "votee" | "en_vigueur") {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "seule une grille votée ou en vigueur se publie",
            ));
        }
        let premiere = v.publiee_le.is_none();
        if premiere {
            sqlx::query(
                "UPDATE ent_fin_grille_version SET publiee_le = now() WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(version)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            journaliser_tx(
                &mut tx,
                &Acte::new(
                    etab,
                    Some(actor),
                    role,
                    Chemin::Finances,
                    "grille.publier",
                    "grille",
                )
                .objet(version)
                .detail(&format!("grille {} v{}", v.annee_scolaire, v.numero)),
            )
            .await?;
            publier_finance(
                &mut tx,
                etab,
                types::FACTURATION_GRILLE_PUBLIEE,
                AggregateKind::Facture,
                version,
                None,
                json!({ "grille_id": version, "annee_scolaire": v.annee_scolaire, "numero": v.numero }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        let v = version_tx(&mut tx, etab, version, false).await?;
        let tarifs = tarifs_version_tx(&mut tx, etab, version, None).await?;
        let tranches = tranches_version_tx(&mut tx, etab, version).await?;
        tx.commit().await.map_err(db)?;
        let lignes = tarifs
            .into_iter()
            .map(|t| {
                let tranche = tranches
                    .iter()
                    .find(|q| {
                        Some(q.qf_min_centimes) == t.qf_min_centimes
                            && q.qf_max_centimes == t.qf_max_centimes
                    })
                    .map(|q| q.libelle.clone());
                PublicationLigneDto {
                    article_libelle: t.article_libelle,
                    categorie: t.categorie,
                    niveau: t.niveau,
                    regime: t.regime,
                    tranche,
                    montant_annuel_centimes: t.montant_annuel_centimes,
                    tarif_par_defaut: t.tarif_par_defaut,
                }
            })
            .collect();
        Ok(PublicationGrilleDto {
            annee_scolaire: v.annee_scolaire,
            numero: v.numero,
            libelle: v.libelle,
            vote_le: v.vote_le,
            publiee_le: v.publiee_le.ok_or_else(introuvable)?,
            lignes,
        })
    }

    // -- Familles types et simulations ------------------------------------------------------------

    pub async fn familles_types_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<FamilleTypeDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationSimuler)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let v = familles_tx(&mut tx, etab, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    /// FAC-124 : déclare une famille type **fictive** (enfants par niveau et régime, tranche de quotient).
    pub async fn creer_famille_type_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &FamilleTypeBody,
    ) -> Result<FamilleTypeDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationSimuler)
            .await?;
        let libelle = texte(&b.libelle, 120, "libellé obligatoire (120 caractères)")?;
        if b.enfants.is_empty() || b.enfants.len() > 10 {
            return Err(invalide("une famille type a de 1 à 10 enfants"));
        }
        let tranche = opt_texte(&b.tranche_code, 8)?;
        for e in &b.enfants {
            if !REGIMES.contains(&e.regime.as_str()) {
                return Err(invalide("régime inconnu (externe, demi_pension, interne)"));
            }
            if e.niveau.trim().is_empty() || e.niveau.trim().chars().count() > 20 {
                return Err(invalide("niveau obligatoire (20 caractères)"));
            }
        }
        let mut tx = self.fin_tx(etab).await?;
        let (nb,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM ent_fin_famille_type WHERE etablissement_id = $1")
                .bind(etab)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        if nb >= MAX_FAMILLES_TYPES {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "plafond de familles types atteint : en supprimer avant d'en créer",
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fin_famille_type (id, etablissement_id, libelle, tranche_code, created_by)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(etab)
        .bind(&libelle)
        .bind(&tranche)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if est_doublon(&e) {
                conflit("une famille type porte déjà ce libellé")
            } else {
                db(e)
            }
        })?;
        for (i, e) in b.enfants.iter().enumerate() {
            sqlx::query(
                "INSERT INTO ent_fin_famille_type_enfant (id, etablissement_id, famille_type_id, rang, niveau, regime)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(i as i16 + 1)
            .bind(e.niveau.trim())
            .bind(&e.regime)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "simulation.famille_type",
                "famille_type",
            )
            .objet(id)
            .detail(&format!(
                "famille type « {libelle} » ({} enfants)",
                b.enfants.len()
            )),
        )
        .await?;
        let dto = familles_tx(&mut tx, etab, true)
            .await?
            .into_iter()
            .find(|f| f.id == id)
            .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    pub async fn desactiver_famille_type_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationSimuler)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let r = sqlx::query(
            "UPDATE ent_fin_famille_type SET actif = false WHERE etablissement_id = $1 AND id = $2 AND actif",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "simulation.famille_type_desactiver",
                "famille_type",
            )
            .objet(id),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// F5 : supprime une famille type (fictive, sans donnée personnelle) et ses enfants ; journalisé.
    pub async fn supprimer_famille_type_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationSimuler)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        sqlx::query("DELETE FROM ent_fin_famille_type_enfant WHERE etablissement_id = $1 AND famille_type_id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let r =
            sqlx::query("DELETE FROM ent_fin_famille_type WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "simulation.famille_type_supprimer",
                "famille_type",
            )
            .objet(id),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// F5 : supprime une simulation (règles, effectifs et historique de ses calculs compris) ; journalisé.
    pub async fn supprimer_simulation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationSimuler)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_fin_simulation WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        for t in [
            "ent_fin_simulation_calcul",
            "ent_fin_simulation_regle",
            "ent_fin_simulation_effectif",
        ] {
            sqlx::query(&format!(
                "DELETE FROM {t} WHERE etablissement_id = $1 AND simulation_id = $2"
            ))
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        sqlx::query("DELETE FROM ent_fin_simulation WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "simulation.supprimer",
                "simulation",
            )
            .objet(id),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// FAC-121 : crée un scénario (règles de hausse et effectifs prévus), le calcule et fige le résultat.
    pub async fn creer_simulation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &SimulationBody,
    ) -> Result<ResultatSimulationDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationSimuler)
            .await?;
        let annee = annee_valide(&b.annee_scolaire)?;
        let libelle = texte(&b.libelle, 120, "libellé obligatoire (120 caractères)")?;
        if b.regles.len() > 50 || b.effectifs.len() > 200 {
            return Err(invalide("trop de règles ou d'effectifs"));
        }
        for r in &b.regles {
            if !(-10_000_000..=10_000_000).contains(&r.valeur) {
                return Err(invalide("hausse hors bornes"));
            }
        }
        let mut vus = BTreeSet::new();
        for e in &b.effectifs {
            if !REGIMES.contains(&e.regime.as_str()) {
                return Err(invalide("régime inconnu (externe, demi_pension, interne)"));
            }
            if e.niveau.trim().is_empty() || e.niveau.trim().chars().count() > 20 {
                return Err(invalide("niveau obligatoire (20 caractères)"));
            }
            if !(0..=100_000).contains(&e.effectif) {
                return Err(invalide("effectif entre 0 et 100 000"));
            }
            if !vus.insert((e.niveau.trim().to_string(), e.regime.clone())) {
                return Err(invalide(
                    "deux effectifs pour le même niveau et le même régime",
                ));
            }
        }
        let mut tx = self.fin_tx(etab).await?;
        let (nb,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM ent_fin_simulation WHERE etablissement_id = $1")
                .bind(etab)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        if nb >= MAX_SIMULATIONS {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "plafond de simulations atteint : en supprimer avant d'en créer",
            ));
        }
        let v = version_tx(&mut tx, etab, b.base_version_id, false).await?;
        if v.statut == "abandonnee" || v.annee_scolaire != annee {
            return Err(invalide(
                "version de base : année différente ou grille abandonnée",
            ));
        }
        for r in &b.regles {
            if let Some(a) = r.article_id {
                article_actif_tx(&mut tx, etab, a).await?;
            }
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fin_simulation (id, etablissement_id, annee_scolaire, libelle, base_version_id, cree_par)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(etab)
        .bind(&annee)
        .bind(&libelle)
        .bind(b.base_version_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for r in &b.regles {
            sqlx::query(
                "INSERT INTO ent_fin_simulation_regle (id, etablissement_id, simulation_id, article_id, hausse_mode,
                        hausse_valeur, arrondi)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(r.article_id)
            .bind(match r.mode {
                ModeHausse::Pourcentage => "pourcentage",
                ModeHausse::Montant => "montant",
            })
            .bind(r.valeur)
            .bind(match r.arrondi {
                Arrondi::Aucun => "aucun",
                Arrondi::Euro => "euro",
                Arrondi::CinqEuros => "cinq_euros",
            })
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        for e in &b.effectifs {
            sqlx::query(
                "INSERT INTO ent_fin_simulation_effectif (id, etablissement_id, simulation_id, niveau, regime, effectif)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(e.niveau.trim())
            .bind(&e.regime)
            .bind(e.effectif)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let res = calculer_simulation_tx(&mut tx, etab, id, Some(actor)).await?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "simulation.creer",
                "simulation",
            )
            .objet(id)
            .detail(&format!(
                "scénario « {libelle} » sur la grille {} v{}",
                v.annee_scolaire, v.numero
            )),
        )
        .await?;
        publier_finance(
            &mut tx,
            etab,
            types::FACTURATION_SIMULATION_CALCULEE,
            AggregateKind::Facture,
            id,
            None,
            json!({ "simulation_id": id, "annee_scolaire": annee }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(res)
    }

    pub async fn simulations_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<SimulationResumeDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationSimuler)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, String, String, Uuid, Option<i64>, Option<i64>, Option<DateTime<Utc>>)> =
            sqlx::query_as(
                "SELECT id, annee_scolaire, libelle, base_version_id, produit_base_centimes,
                        produit_simule_centimes, calcule_le
                   FROM ent_fin_simulation WHERE etablissement_id = $1 ORDER BY created_at DESC, id LIMIT 200",
            )
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| SimulationResumeDto {
                id: r.0,
                annee_scolaire: r.1,
                libelle: r.2,
                base_version_id: r.3,
                produit_base_centimes: r.4,
                produit_simule_centimes: r.5,
                calcule_le: r.6,
            })
            .collect())
    }

    /// Recalcule et fige le résultat d'un scénario (effectifs actuels si aucun effectif prévu).
    pub async fn calculer_simulation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ResultatSimulationDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationSimuler)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let res = calculer_simulation_tx(&mut tx, etab, id, Some(actor)).await?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "simulation.calculer",
                "simulation",
            )
            .objet(id),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(res)
    }

    /// FAC-122 : compare deux scénarios (calcul à la demande, rien n'est écrit).
    pub async fn comparer_simulations_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        a: Uuid,
        b: Uuid,
    ) -> Result<ComparaisonSimulationsDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationSimuler)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let ra = calculer_simulation_tx(&mut tx, etab, a, None).await?;
        let rb = calculer_simulation_tx(&mut tx, etab, b, None).await?;
        tx.commit().await.map_err(db)?;
        let mut familles = Vec::new();
        for fa in &ra.familles {
            if let Some(fb) = rb
                .familles
                .iter()
                .find(|f| f.famille_type_id == fa.famille_type_id)
            {
                familles.push(EcartFamilleDto {
                    famille_type_id: fa.famille_type_id,
                    libelle: fa.libelle.clone(),
                    total_a_centimes: fa.total_simule_centimes,
                    total_b_centimes: fb.total_simule_centimes,
                    ecart_centimes: fb.total_simule_centimes - fa.total_simule_centimes,
                });
            }
        }
        Ok(ComparaisonSimulationsDto {
            ecart_produit_centimes: rb.produit_simule_centimes - ra.produit_simule_centimes,
            a: ra,
            b: rb,
            familles,
        })
    }

    /// FAC-123 : tableau pour le conseil d'administration (aucune donnée personnelle), journalisé.
    pub async fn exporter_conseil_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ExportConseilDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationSimuler)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let res = calculer_simulation_tx(&mut tx, etab, id, None).await?;
        journaliser_tx(
            &mut tx,
            &Acte::new(
                etab,
                Some(actor),
                role,
                Chemin::Finances,
                "simulation.exporter",
                "simulation",
            )
            .objet(id),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        let mut csv =
            String::from("rubrique;libelle;produit_base_eur;produit_simule_eur;ecart_eur\n");
        for p in &res.par_article {
            csv.push_str(&format!(
                "article;{};{};{};{}\n",
                cellule_csv(&p.article_libelle),
                euros(p.produit_base_centimes),
                euros(p.produit_simule_centimes),
                euros(p.produit_simule_centimes - p.produit_base_centimes)
            ));
        }
        for f in &res.familles {
            csv.push_str(&format!(
                "famille_type;{};{};{};{}\n",
                cellule_csv(&f.libelle),
                euros(f.total_base_centimes),
                euros(f.total_simule_centimes),
                euros(f.ecart_centimes)
            ));
        }
        csv.push_str(&format!(
            "total;;{};{};{}\n",
            euros(res.produit_base_centimes),
            euros(res.produit_simule_centimes),
            euros(res.ecart_centimes)
        ));
        Ok(ExportConseilDto {
            titre: format!("Grille {} : {}", res.annee_scolaire, res.libelle),
            resultat: res,
            csv,
        })
    }

    /// FAC-124 : effet d'une grille sur les familles types, sans hausse.
    pub async fn effet_familles_types_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        version: Uuid,
    ) -> Result<Vec<EffetFamilleTypeDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationSimuler)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let v = version_tx(&mut tx, etab, version, false).await?;
        let (articles, tarifs, tranches) = donnees_grille_tx(&mut tx, etab, version).await?;
        let familles = familles_tx(&mut tx, etab, false).await?;
        let fratries = fratries_tx(&mut tx, etab, &v.annee_scolaire).await?;
        tx.commit().await.map_err(db)?;
        Ok(effets_familles(
            &articles, &tarifs, &tarifs, &tranches, &familles, &fratries,
        ))
    }

    // -- État des contributions -----------------------------------------------------------------

    /// FAC-196 : état agrégé des contributions par nature légale pour une année (inspection), sans nom.
    pub async fn etat_contributions_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        annee: &str,
    ) -> Result<EtatContributionsDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let annee = annee_valide(annee)?;
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(
            "SELECT COALESCE(a.nature_legale, 'non_qualifie'),
                    COALESCE(sum(l.quantite::BIGINT * l.montant_unitaire_centimes) FILTER (WHERE f.type_piece <> 'avoir'), 0)::BIGINT,
                    COALESCE(sum(l.quantite::BIGINT * l.montant_unitaire_centimes) FILTER (WHERE f.type_piece = 'avoir'), 0)::BIGINT,
                    count(*)::BIGINT
               FROM ent_fac_ligne l
               JOIN ent_fac_facture f ON f.id = l.facture_id AND f.etablissement_id = l.etablissement_id
               LEFT JOIN ent_fin_article a ON a.id = l.article_id AND a.etablissement_id = l.etablissement_id
              WHERE l.etablissement_id = $1 AND f.annee_scolaire = $2 AND f.statut IN ('emise', 'soldee')
              GROUP BY 1 ORDER BY 1",
        )
        .bind(etab)
        .bind(&annee)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let non_q = nb_non_qualifies_tx(&mut tx, etab).await?;
        tx.commit().await.map_err(db)?;
        let lignes: Vec<ContributionNatureDto> = rows
            .into_iter()
            .map(|r| ContributionNatureDto {
                nature_legale: r.0,
                facture_centimes: r.1,
                avoirs_centimes: r.2,
                net_centimes: r.1 + r.2,
                nb_lignes: r.3,
            })
            .collect();
        let mut csv = String::from("nature_legale;facture_eur;avoirs_eur;net_eur;lignes\n");
        for l in &lignes {
            csv.push_str(&format!(
                "{};{};{};{};{}\n",
                cellule_csv(&l.nature_legale),
                euros(l.facture_centimes),
                euros(l.avoirs_centimes),
                euros(l.net_centimes),
                l.nb_lignes
            ));
        }
        Ok(EtatContributionsDto {
            annee_scolaire: annee,
            total_net_centimes: lignes.iter().map(|l| l.net_centimes).sum(),
            lignes,
            articles_non_qualifies: non_q,
            csv,
        })
    }
}

// ---------------------------------------------------------------------------
// Aides aux commandes
// ---------------------------------------------------------------------------

/// Premier et dernier jour de l'année scolaire « AAAA-AAAA » (1er septembre → 31 août).
fn bornes_annee(annee: &str) -> Result<(NaiveDate, NaiveDate), EntDbError> {
    let debut: i32 = annee
        .get(..4)
        .and_then(|a| a.parse().ok())
        .ok_or(invalide("année scolaire invalide"))?;
    match (
        NaiveDate::from_ymd_opt(debut, 9, 1),
        NaiveDate::from_ymd_opt(debut + 1, 8, 31),
    ) {
        (Some(a), Some(b)) => Ok((a, b)),
        _ => Err(invalide("année scolaire invalide")),
    }
}

/// Auteurs réels d'une grille : créateur, proposant, auteurs et modificateurs des tarifs (règle 3).
async fn auteurs_grille_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    v: &VersionRow,
) -> Result<BTreeSet<Uuid>, EntDbError> {
    let rows: Vec<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT created_by FROM ent_fin_tarif WHERE etablissement_id = $1 AND grille_version_id = $2
         UNION SELECT modifie_par FROM ent_fin_tarif WHERE etablissement_id = $1 AND grille_version_id = $2",
    )
    .bind(etab)
    .bind(v.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut set: BTreeSet<Uuid> = rows.into_iter().filter_map(|r| r.0).collect();
    set.extend(v.cree_par);
    set.extend(v.propose_par);
    Ok(set)
}

fn exiger_brouillon(v: &VersionRow) -> Result<(), EntDbError> {
    if v.statut == "brouillon" {
        Ok(())
    } else {
        Err(EntDbError::Refus(
            RefusMetier::Conflit,
            "grille figée : seule une version en brouillon se modifie (R-F10), créer une correction",
        ))
    }
}

fn valider_tarif(b: &TarifGrilleBody) -> Result<(Option<String>, Option<String>), EntDbError> {
    let regime = opt_texte(&b.regime, 20)?;
    if regime.as_deref().is_some_and(|r| !REGIMES.contains(&r)) {
        return Err(invalide("régime inconnu (externe, demi_pension, interne)"));
    }
    if !(0..=MONTANT_MAX).contains(&b.montant_annuel_centimes) {
        return Err(invalide("montant annuel entre 0 et 1 000 000 €"));
    }
    if b.qf_min_centimes.is_some_and(|m| m < 0)
        || matches!((b.qf_min_centimes, b.qf_max_centimes), (Some(a), Some(z)) if z <= a)
    {
        return Err(invalide("tranche de quotient : minimum < maximum"));
    }
    if b.tarif_par_defaut == Some(true) && b.qf_max_centimes.is_some() {
        return Err(invalide(
            "le tarif par défaut n'a pas de borne haute (plein tarif)",
        ));
    }
    Ok((opt_texte(&b.niveau, 20)?, regime))
}

async fn article_actif_tx(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<(), EntDbError> {
    let r: Option<(bool,)> =
        sqlx::query_as("SELECT actif FROM ent_fin_article WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
    match r {
        None => Err(introuvable()),
        Some((false,)) => Err(EntDbError::Refus(RefusMetier::Conflit, "article désactivé")),
        Some(_) => Ok(()),
    }
}

/// Copie tarifs et tranches d'une version vers une autre (brouillon), pour une année donnée.
async fn copier_contenu_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    source: Uuid,
    cible: Uuid,
    annee: &str,
    actor: Uuid,
) -> Result<i64, EntDbError> {
    let r = sqlx::query(
        "INSERT INTO ent_fin_tarif (id, etablissement_id, annee_scolaire, article_id, niveau, regime,
                qf_min_centimes, qf_max_centimes, montant_annuel_centimes, actif, created_by, grille_version_id,
                tarif_par_defaut)
         SELECT gen_random_uuid(), etablissement_id, $4, article_id, niveau, regime, qf_min_centimes,
                qf_max_centimes, montant_annuel_centimes, true, $5, $3, tarif_par_defaut
           FROM ent_fin_tarif
          WHERE etablissement_id = $1 AND grille_version_id = $2 AND actif",
    )
    .bind(etab)
    .bind(source)
    .bind(cible)
    .bind(annee)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO ent_fin_tranche_qf (id, etablissement_id, grille_version_id, code, libelle, qf_min_centimes,
                qf_max_centimes)
         SELECT gen_random_uuid(), etablissement_id, $3, code, libelle, qf_min_centimes, qf_max_centimes
           FROM ent_fin_tranche_qf WHERE etablissement_id = $1 AND grille_version_id = $2",
    )
    .bind(etab)
    .bind(source)
    .bind(cible)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.rows_affected() as i64)
}

async fn controles_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    version: Uuid,
) -> Result<ControlesGrilleDto, EntDbError> {
    let tarifs = tarifs_version_tx(conn, etab, version, None).await?;
    let tranches = tranches_version_tx(conn, etab, version).await?;
    let nq: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT a.id FROM ent_fin_tarif t
           JOIN ent_fin_article a ON a.id = t.article_id AND a.etablissement_id = t.etablissement_id
          WHERE t.etablissement_id = $1 AND t.grille_version_id = $2 AND a.qualifie_le IS NULL",
    )
    .bind(etab)
    .bind(version)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let non_qualifies: BTreeSet<Uuid> = nq.into_iter().map(|r| r.0).collect();
    let mut anomalies = controler_grille(&tarifs, &tranches, &non_qualifies);
    // continuité avec la version précédente de l'année, ou à défaut avec l'historique (règle 4)
    let (annee,): (String,) = sqlx::query_as(
        "SELECT annee_scolaire FROM ent_fin_grille_version WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(version)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let precedente: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_fin_grille_version
          WHERE etablissement_id = $1 AND annee_scolaire = $2 AND id <> $3
            AND statut IN ('en_vigueur', 'close') AND en_vigueur_du IS NOT NULL
          ORDER BY en_vigueur_du DESC, numero DESC LIMIT 1",
    )
    .bind(etab)
    .bind(&annee)
    .bind(version)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let rows: Vec<(Uuid, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT t.article_id, a.code, t.niveau, t.regime FROM ent_fin_tarif t
           JOIN ent_fin_article a ON a.id = t.article_id AND a.etablissement_id = t.etablissement_id
          WHERE t.etablissement_id = $1 AND t.actif AND a.actif
            AND CASE WHEN $3::UUID IS NULL THEN t.annee_scolaire = $2 AND t.grille_version_id IS NULL
                     ELSE t.grille_version_id = $3 END",
    )
    .bind(etab)
    .bind(&annee)
    .bind(precedente.map(|p| p.0))
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let precedents: Vec<TarifPrecedent> = rows
        .into_iter()
        .map(|r| TarifPrecedent {
            article_id: r.0,
            article_code: r.1,
            niveau: r.2,
            regime: r.3,
        })
        .collect();
    let facturees: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT DISTINCT a.id, a.code FROM ent_fac_ligne l
           JOIN ent_fin_article a ON a.id = l.article_id AND a.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.source_cle LIKE $2 AND a.actif
            AND NOT EXISTS (SELECT 1 FROM ent_fin_tarif t WHERE t.etablissement_id = l.etablissement_id
                              AND t.grille_version_id = $3 AND t.article_id = a.id AND t.actif)",
    )
    .bind(etab)
    .bind(format!("annuel:{annee}%"))
    .bind(version)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    if !tarifs.is_empty() {
        anomalies.extend(controler_continuite(&precedents, &tarifs, &facturees));
    }
    let proposable = !anomalies.iter().any(|a| a.bloquante);
    Ok(ControlesGrilleDto {
        anomalies,
        proposable,
    })
}

async fn familles_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    avec_inactives: bool,
) -> Result<Vec<FamilleTypeDto>, EntDbError> {
    let rows: Vec<(Uuid, String, Option<String>, bool)> = sqlx::query_as(
        "SELECT id, libelle, tranche_code, actif FROM ent_fin_famille_type
          WHERE etablissement_id = $1 AND (actif OR $2) ORDER BY libelle",
    )
    .bind(etab)
    .bind(avec_inactives)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let enfants: Vec<(Uuid, i16, String, String)> = sqlx::query_as(
        "SELECT famille_type_id, rang, niveau, regime FROM ent_fin_famille_type_enfant
          WHERE etablissement_id = $1 ORDER BY famille_type_id, rang",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| FamilleTypeDto {
            enfants: enfants
                .iter()
                .filter(|e| e.0 == r.0)
                .map(|e| EnfantTypeDto {
                    rang: e.1,
                    niveau: e.2.clone(),
                    regime: e.3.clone(),
                })
                .collect(),
            id: r.0,
            libelle: r.1,
            tranche_code: r.2,
            actif: r.3,
        })
        .collect())
}

async fn donnees_grille_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    version: Uuid,
) -> Result<(Vec<ArticleSim>, Vec<TarifSim>, Vec<TrancheQfDto>), EntDbError> {
    let tarifs_dto = tarifs_version_tx(conn, etab, version, None).await?;
    let ids: Vec<Uuid> = tarifs_dto
        .iter()
        .map(|t| t.article_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let arts: Vec<(Uuid, String, String, String, bool)> = sqlx::query_as(
        "SELECT id, code, libelle, categorie, soumis_reductions FROM ent_fin_article
          WHERE etablissement_id = $1 AND id = ANY($2) ORDER BY code",
    )
    .bind(etab)
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let articles = arts
        .into_iter()
        .map(|a| ArticleSim {
            id: a.0,
            code: a.1,
            libelle: a.2,
            categorie: a.3,
            soumis_reductions: a.4,
        })
        .collect();
    let tarifs = tarifs_dto
        .into_iter()
        .map(|t| TarifSim {
            id: t.id,
            article_id: t.article_id,
            niveau: t.niveau,
            regime: t.regime,
            qf_min: t.qf_min_centimes,
            qf_max: t.qf_max_centimes,
            montant: t.montant_annuel_centimes,
        })
        .collect();
    let tranches = tranches_version_tx(conn, etab, version).await?;
    Ok((articles, tarifs, tranches))
}

async fn fratries_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
) -> Result<Vec<FratrieSim>, EntDbError> {
    let rows: Vec<(i16, i32, Vec<String>)> = sqlx::query_as(
        "SELECT rang_min, taux_bps, categories FROM ent_fin_reduction
          WHERE etablissement_id = $1 AND annee_scolaire = $2 AND actif AND nature = 'fratrie'",
    )
    .bind(etab)
    .bind(annee)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| FratrieSim {
            rang_min: i64::from(r.0),
            taux_bps: r.1,
            categories: r.2,
        })
        .collect())
}

fn effets_familles(
    articles: &[ArticleSim],
    base: &[TarifSim],
    simule: &[TarifSim],
    tranches: &[TrancheQfDto],
    familles: &[FamilleTypeDto],
    fratries: &[FratrieSim],
) -> Vec<EffetFamilleTypeDto> {
    familles
        .iter()
        .filter(|f| f.actif)
        .map(|f| {
            let qf = f
                .tranche_code
                .as_deref()
                .and_then(|c| tranches.iter().find(|t| t.code == c))
                .map(|t| t.qf_min_centimes);
            let enfants: Vec<(String, String)> = f
                .enfants
                .iter()
                .map(|e| (e.niveau.clone(), e.regime.clone()))
                .collect();
            let b = total_famille(articles, base, &enfants, qf, fratries);
            let s = total_famille(articles, simule, &enfants, qf, fratries);
            EffetFamilleTypeDto {
                famille_type_id: f.id,
                libelle: f.libelle.clone(),
                total_base_centimes: b,
                total_simule_centimes: s,
                ecart_centimes: s - b,
            }
        })
        .collect()
}

/// Calcule un scénario ; `figer` : enregistre les produits et la date du calcul.
async fn calculer_simulation_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    figer_par: Option<Uuid>,
) -> Result<ResultatSimulationDto, EntDbError> {
    let (annee, libelle, base_version): (String, String, Uuid) = sqlx::query_as(
        "SELECT annee_scolaire, libelle, base_version_id FROM ent_fin_simulation
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    let regles_rows: Vec<(Option<Uuid>, String, i64, String)> = sqlx::query_as(
        "SELECT article_id, hausse_mode, hausse_valeur, arrondi FROM ent_fin_simulation_regle
          WHERE etablissement_id = $1 AND simulation_id = $2 ORDER BY created_at, id",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let regles: Vec<RegleSim> = regles_rows
        .into_iter()
        .map(|r| RegleSim {
            article_id: r.0,
            mode: if r.1 == "montant" {
                ModeHausse::Montant
            } else {
                ModeHausse::Pourcentage
            },
            valeur: r.2,
            arrondi: match r.3.as_str() {
                "euro" => Arrondi::Euro,
                "cinq_euros" => Arrondi::CinqEuros,
                _ => Arrondi::Aucun,
            },
        })
        .collect();
    let prevus: Vec<(String, String, i32)> = sqlx::query_as(
        "SELECT niveau, regime, effectif FROM ent_fin_simulation_effectif
          WHERE etablissement_id = $1 AND simulation_id = $2 ORDER BY niveau, regime",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let (effectifs, origine) = if prevus.is_empty() {
        let actuels: Vec<(Option<String>, Option<String>, i64)> = sqlx::query_as(
            "SELECT c.niveau, e.regime, count(*)::BIGINT FROM ent_eleve e
               LEFT JOIN ent_classe c ON c.id = e.classe_id AND c.etablissement_id = e.etablissement_id
              WHERE e.etablissement_id = $1 AND e.status = 'actif'
              GROUP BY c.niveau, e.regime ORDER BY 1, 2",
        )
        .bind(etab)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut agrege: BTreeMap<(String, String), i32> = BTreeMap::new();
        for (n, r, c) in actuels {
            let regime = calcul::regime_normalise(r.as_deref());
            *agrege
                .entry((n.unwrap_or_else(|| "inconnu".into()), regime))
                .or_default() += i32::try_from(c).unwrap_or(i32::MAX);
        }
        (
            agrege
                .into_iter()
                .map(|((n, r), c)| (n, r, c))
                .collect::<Vec<_>>(),
            "actuel",
        )
    } else {
        (prevus, "prevu")
    };
    let (articles, base, tranches) = donnees_grille_tx(conn, etab, base_version).await?;
    let simule = appliquer_regles(&base, &regles, &articles)?;
    let (produit_base, par_base) = produit_attendu(&articles, &base, &effectifs);
    let (produit_simule, par_simule) = produit_attendu(&articles, &simule, &effectifs);
    let familles = familles_tx(conn, etab, false).await?;
    let fratries = fratries_tx(conn, etab, &annee).await?;
    let effets = effets_familles(&articles, &base, &simule, &tranches, &familles, &fratries);
    let par_article = articles
        .iter()
        .filter(|a| par_base.contains_key(&a.id) || par_simule.contains_key(&a.id))
        .map(|a| ProduitArticleDto {
            article_code: a.code.clone(),
            article_libelle: a.libelle.clone(),
            produit_base_centimes: par_base.get(&a.id).copied().unwrap_or(0),
            produit_simule_centimes: par_simule.get(&a.id).copied().unwrap_or(0),
        })
        .collect();
    let (calcule_le,): (DateTime<Utc>,) = if let Some(acteur) = figer_par {
        // F4 : le résultat précédent n'est jamais écrasé en silence : chaque calcul s'ajoute à l'historique
        sqlx::query(
            "INSERT INTO ent_fin_simulation_calcul (id, etablissement_id, simulation_id, produit_base_centimes,
                    produit_simule_centimes, calcule_par)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(produit_base)
        .bind(produit_simule)
        .bind(acteur)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        sqlx::query_as(
            "UPDATE ent_fin_simulation SET produit_base_centimes = $3, produit_simule_centimes = $4, calcule_le = now()
              WHERE etablissement_id = $1 AND id = $2 RETURNING calcule_le",
        )
        .bind(etab)
        .bind(id)
        .bind(produit_base)
        .bind(produit_simule)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?
    } else {
        sqlx::query_as("SELECT COALESCE(calcule_le, now()) FROM ent_fin_simulation WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?
    };
    Ok(ResultatSimulationDto {
        simulation_id: id,
        annee_scolaire: annee,
        libelle,
        base_version_id: base_version,
        produit_base_centimes: produit_base,
        produit_simule_centimes: produit_simule,
        ecart_centimes: produit_simule - produit_base,
        effectifs: effectifs
            .into_iter()
            .map(|(niveau, regime, effectif)| EffectifUtiliseDto {
                niveau,
                regime,
                effectif,
                origine: origine.into(),
            })
            .collect(),
        par_article,
        familles: effets,
        calcule_le,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tranche(code: &str, min: i64, max: Option<i64>) -> TrancheQfBody {
        TrancheQfBody {
            code: code.into(),
            libelle: format!("Tranche {code}"),
            qf_min_centimes: min,
            qf_max_centimes: max,
        }
    }

    #[test]
    fn arrondi_a_l_euro_et_a_cinq_euros() {
        assert_eq!(arrondir(12_349, Arrondi::Euro), 12_300);
        assert_eq!(arrondir(12_350, Arrondi::Euro), 12_400);
        assert_eq!(arrondir(12_499, Arrondi::CinqEuros), 12_500);
        assert_eq!(arrondir(12_249, Arrondi::CinqEuros), 12_000);
        assert_eq!(arrondir(12_250, Arrondi::CinqEuros), 12_500);
        assert_eq!(arrondir(777, Arrondi::Aucun), 777);
    }

    #[test]
    fn hausse_en_pourcentage_et_en_montant() {
        // +3 % de 1 234,56 € = +37,0368 → 1 271,60 € (centime le plus proche)
        assert_eq!(
            montant_apres_hausse(123_456, ModeHausse::Pourcentage, 300, Arrondi::Aucun).unwrap(),
            127_160
        );
        assert_eq!(
            montant_apres_hausse(123_456, ModeHausse::Pourcentage, 300, Arrondi::Euro).unwrap(),
            127_200
        );
        assert_eq!(
            montant_apres_hausse(100_000, ModeHausse::Montant, 2_500, Arrondi::Aucun).unwrap(),
            102_500
        );
        // baisse : plancher à 0
        assert_eq!(
            montant_apres_hausse(1_000, ModeHausse::Montant, -5_000, Arrondi::Aucun).unwrap(),
            0
        );
        assert_eq!(
            montant_apres_hausse(100_000, ModeHausse::Pourcentage, -1_000, Arrondi::Aucun).unwrap(),
            90_000
        );
        // au-delà de 1 000 000 €
        assert!(montant_apres_hausse(99_999_999, ModeHausse::Montant, 10, Arrondi::Aucun).is_err());
    }

    #[test]
    fn tranches_sans_trou_ni_chevauchement() {
        let ok = [
            tranche("A", 0, Some(50_000)),
            tranche("B", 50_000, Some(90_000)),
            tranche("C", 90_000, None),
        ];
        assert!(valider_tranches(&ok).is_ok());
        // dans le désordre : accepté (triées)
        let desordre = [
            tranche("C", 90_000, None),
            tranche("A", 0, Some(50_000)),
            tranche("B", 50_000, Some(90_000)),
        ];
        assert!(valider_tranches(&desordre).is_ok());
        let trou = [tranche("A", 0, Some(50_000)), tranche("B", 60_000, None)];
        assert_eq!(valider_tranches(&trou), Err("trou entre deux tranches"));
        let chevauche = [tranche("A", 0, Some(60_000)), tranche("B", 50_000, None)];
        assert_eq!(
            valider_tranches(&chevauche),
            Err("deux tranches se chevauchent")
        );
        let pas_zero = [tranche("A", 100, None)];
        assert!(valider_tranches(&pas_zero).is_err());
        let fermee = [tranche("A", 0, Some(100))];
        assert!(valider_tranches(&fermee).is_err());
        let doublon = [tranche("A", 0, Some(100)), tranche("A", 100, None)];
        assert_eq!(
            valider_tranches(&doublon),
            Err("deux tranches portent le même code")
        );
        assert!(valider_tranches(&[]).is_ok());
    }

    fn tarif(
        art: Uuid,
        code: &str,
        min: Option<i64>,
        max: Option<i64>,
        montant: i64,
        defaut: bool,
    ) -> TarifGrilleDto {
        TarifGrilleDto {
            id: Uuid::new_v4(),
            grille_version_id: Uuid::nil(),
            annee_scolaire: "2030-2031".into(),
            article_id: art,
            article_code: code.into(),
            article_libelle: code.into(),
            categorie: "scolarite".into(),
            niveau: None,
            regime: None,
            qf_min_centimes: min,
            qf_max_centimes: max,
            montant_annuel_centimes: montant,
            tarif_par_defaut: defaut,
        }
    }

    #[test]
    fn controles_grille_bloquants() {
        let a = Uuid::new_v4();
        let tranches = vec![
            TrancheQfDto {
                id: Uuid::new_v4(),
                code: "A".into(),
                libelle: "A".into(),
                qf_min_centimes: 0,
                qf_max_centimes: Some(50_000),
            },
            TrancheQfDto {
                id: Uuid::new_v4(),
                code: "B".into(),
                libelle: "B".into(),
                qf_min_centimes: 50_000,
                qf_max_centimes: None,
            },
        ];
        let nq = BTreeSet::new();
        // grille vide
        assert!(controler_grille(&[], &tranches, &nq)
            .iter()
            .any(|x| x.code == "grille_vide" && x.bloquante));
        // complète et cohérente
        let bonne = [
            tarif(a, "CONTRIB", Some(0), Some(50_000), 40_000, false),
            tarif(a, "CONTRIB", Some(50_000), None, 60_000, true),
        ];
        assert!(controler_grille(&bonne, &tranches, &nq)
            .iter()
            .all(|x| !x.bloquante));
        // tarif par défaut manquant
        let sans_defaut = [
            tarif(a, "CONTRIB", Some(0), Some(50_000), 40_000, false),
            tarif(a, "CONTRIB", Some(50_000), None, 60_000, false),
        ];
        assert!(controler_grille(&sans_defaut, &tranches, &nq)
            .iter()
            .any(|x| x.code == "tarif_defaut_manquant"));
        // tranches absentes
        assert!(controler_grille(&bonne, &[], &nq)
            .iter()
            .any(|x| x.code == "tranches_manquantes"));
        // bornes qui ne collent à aucune tranche
        let hors = [
            tarif(a, "CONTRIB", Some(0), Some(70_000), 40_000, false),
            tarif(a, "CONTRIB", Some(70_000), None, 6, true),
        ];
        assert!(controler_grille(&hors, &tranches, &nq)
            .iter()
            .any(|x| x.code == "tarif_hors_tranche"));
        // doublon, article non qualifié, défaut avec borne haute
        let doublon = [
            tarif(a, "C", None, None, 100, true),
            tarif(a, "C", None, None, 200, false),
        ];
        assert!(controler_grille(&doublon, &[], &nq)
            .iter()
            .any(|x| x.code == "tarif_en_double"));
        let nq2: BTreeSet<Uuid> = [a].into();
        assert!(controler_grille(&doublon, &[], &nq2)
            .iter()
            .any(|x| x.code == "article_non_qualifie"));
        let incoh = [
            tarif(a, "C", Some(0), Some(50_000), 100, true),
            tarif(a, "C", Some(50_000), None, 90, false),
        ];
        assert!(controler_grille(&incoh, &tranches, &nq)
            .iter()
            .any(|x| x.code == "defaut_incoherent"));
    }

    #[test]
    fn proposition_par_categorie_et_modeles_coherents() {
        assert_eq!(
            proposition_qualification("restauration").nature_legale,
            "demi_pension"
        );
        assert_eq!(
            proposition_qualification("internat").nature_legale,
            "internat"
        );
        assert_eq!(
            proposition_qualification("reduction").regime_tva,
            "hors_champ"
        );
        assert!(!proposition_qualification("bourse").soumis_reductions);
        // chaque modèle respecte les listes fermées et la règle de la TVA
        let mut codes = BTreeSet::new();
        for m in modeles_articles() {
            assert!(
                codes.insert(m.code.clone()),
                "modèle en double : {}",
                m.code
            );
            assert!(
                NATURES_LEGALES.contains(&m.nature_legale.as_str()),
                "{}",
                m.code
            );
            assert!(REGIMES_TVA.contains(&m.regime_tva.as_str()), "{}", m.code);
            assert!(UNITES.contains(&m.unite.as_str()), "{}", m.code);
            assert!(CATEGORIES.contains(&m.categorie.as_str()), "{}", m.code);
            assert_eq!(
                m.regime_tva == "taxable",
                m.taux_tva_bps.is_some(),
                "{}",
                m.code
            );
        }
        // qualification invalide
        let mauvais = QualifierArticleBody {
            nature_legale: "demi_pension".into(),
            regime_tva: "taxable".into(),
            taux_tva_bps: None,
            unite: "annee".into(),
            mode: None,
            remboursable: None,
            soumis_reductions: None,
            soumis_remise_ordre: None,
        };
        assert!(valider_qualification("restauration", &mauvais).is_err());
        let exonere_avec_taux = QualifierArticleBody {
            regime_tva: "exonere_261_4_4".into(),
            taux_tva_bps: Some(550),
            ..mauvais.clone()
        };
        assert!(valider_qualification("restauration", &exonere_avec_taux).is_err());
        let bon = QualifierArticleBody {
            regime_tva: "taxable".into(),
            taux_tva_bps: Some(2000),
            ..mauvais
        };
        assert!(valider_qualification("frais", &bon).is_ok());
    }

    #[test]
    fn simulation_produit_et_famille_type() {
        let contrib = ArticleSim {
            id: Uuid::new_v4(),
            code: "C".into(),
            libelle: "Contribution".into(),
            categorie: "scolarite".into(),
            soumis_reductions: true,
        };
        let dp = ArticleSim {
            id: Uuid::new_v4(),
            code: "DP".into(),
            libelle: "Demi-pension".into(),
            categorie: "restauration".into(),
            soumis_reductions: true,
        };
        let t = |a: &ArticleSim, niveau: Option<&str>, regime: Option<&str>, m: i64| TarifSim {
            id: Uuid::new_v4(),
            article_id: a.id,
            niveau: niveau.map(Into::into),
            regime: regime.map(Into::into),
            qf_min: None,
            qf_max: None,
            montant: m,
        };
        let tarifs = vec![
            t(&contrib, None, None, 100_000),
            t(&contrib, Some("6EME"), None, 120_000),
            t(&dp, None, Some("demi_pension"), 50_000),
        ];
        let articles = vec![contrib.clone(), dp.clone()];
        // 10 élèves de 6EME externes + 4 de 5EME demi-pensionnaires
        let eff = vec![
            ("6EME".to_string(), "externe".to_string(), 10),
            ("5EME".to_string(), "demi_pension".to_string(), 4),
        ];
        let (total, par) = produit_attendu(&articles, &tarifs, &eff);
        assert_eq!(par[&contrib.id], 10 * 120_000 + 4 * 100_000);
        assert_eq!(par[&dp.id], 4 * 50_000);
        assert_eq!(total, 1_200_000 + 400_000 + 200_000);
        // +3 % arrondi à l'euro sur la contribution seulement
        let regles = vec![RegleSim {
            article_id: Some(contrib.id),
            mode: ModeHausse::Pourcentage,
            valeur: 300,
            arrondi: Arrondi::Euro,
        }];
        let hausse = appliquer_regles(&tarifs, &regles, &articles).unwrap();
        let (apres, _) = produit_attendu(&articles, &hausse, &eff);
        assert_eq!(apres, 10 * 123_600 + 4 * 103_000 + 200_000);
        // famille type : 2 enfants, fratrie -10 % à partir du 2e sur la scolarité
        let fratries = vec![FratrieSim {
            rang_min: 2,
            taux_bps: 1_000,
            categories: vec!["scolarite".into()],
        }];
        let enfants = vec![
            ("6EME".to_string(), "externe".to_string()),
            ("5EME".to_string(), "demi_pension".to_string()),
        ];
        let f = total_famille(&articles, &tarifs, &enfants, None, &fratries);
        // enfant 1 : 120 000 ; enfant 2 : 100 000 − 10 000 + 50 000 DP
        assert_eq!(f, 120_000 + 90_000 + 50_000);
    }

    #[test]
    fn cellules_csv_protegees() {
        assert_eq!(cellule_csv("=SOMME(A1)"), "'=SOMME(A1)");
        assert_eq!(cellule_csv("a;b"), "\"a;b\"");
        assert_eq!(cellule_csv("ligne\nsuivante"), "ligne suivante");
        assert_eq!(euros(123_456), "1234,56");
        assert_eq!(euros(-5), "-0,05");
    }

    #[test]
    fn f6_csv_neutralise_tabulation_et_retour_chariot_en_tete() {
        assert_eq!(cellule_csv("=1+1"), "'=1+1");
        assert!(cellule_csv("\t=1+1").starts_with('\''));
        assert!(cellule_csv("\r=1+1").starts_with('\''));
        assert!(cellule_csv("@SUM(A1)").starts_with('\''));
        assert!(!cellule_csv("a\tb").contains('\t'));
        assert_eq!(cellule_csv("simple"), "simple");
    }

    #[test]
    fn f7_arrondi_symetrique_des_baisses() {
        let p = |m, v| montant_apres_hausse(m, ModeHausse::Pourcentage, v, Arrondi::Aucun).unwrap();
        // demi éloigné de zéro des deux côtés : 0,5 centime → ±1
        assert_eq!(p(100, 50), 101);
        assert_eq!(p(100, -50), 99);
        assert_eq!(p(1_000, 50), 1_005);
        assert_eq!(p(1_000, -50), 995);
        // sous la demi-unité : aucun changement, des deux côtés
        assert_eq!(p(150, 33), 150);
        assert_eq!(p(150, -33), 150);
    }

    #[test]
    fn f7_hausse_generale_en_montant_epargne_bourses_et_reductions() {
        let art = |code: &str, cat: &str| ArticleSim {
            id: Uuid::new_v4(),
            code: code.into(),
            libelle: code.into(),
            categorie: cat.into(),
            soumis_reductions: false,
        };
        let (a, b) = (art("CONTRIB", "scolarite"), art("BOURSE", "bourse"));
        let t = |art: &ArticleSim, m| TarifSim {
            id: Uuid::new_v4(),
            article_id: art.id,
            niveau: None,
            regime: None,
            qf_min: None,
            qf_max: None,
            montant: m,
        };
        let tarifs = vec![t(&a, 100_000), t(&b, 20_000)];
        let articles = vec![a.clone(), b.clone()];
        let generale = RegleSim {
            article_id: None,
            mode: ModeHausse::Montant,
            valeur: 5_000,
            arrondi: Arrondi::Aucun,
        };
        let r = appliquer_regles(&tarifs, std::slice::from_ref(&generale), &articles).unwrap();
        assert_eq!((r[0].montant, r[1].montant), (105_000, 20_000));
        // avec un article explicite, la règle s'applique
        let explicite = RegleSim {
            article_id: Some(b.id),
            ..generale
        };
        let r = appliquer_regles(&tarifs, &[explicite], &articles).unwrap();
        assert_eq!(r[1].montant, 25_000);
    }

    #[test]
    fn m6_continuite_article_oublie_et_tarif_perdu() {
        let art = Uuid::new_v4();
        let autre = Uuid::new_v4();
        let prec = |a: Uuid, n: Option<&str>| TarifPrecedent {
            article_id: a,
            article_code: "X".into(),
            niveau: n.map(Into::into),
            regime: None,
        };
        let tarif = |a: Uuid, n: Option<&str>| TarifGrilleDto {
            id: Uuid::new_v4(),
            grille_version_id: Uuid::new_v4(),
            annee_scolaire: "2030-2031".into(),
            article_id: a,
            article_code: "X".into(),
            article_libelle: "X".into(),
            categorie: "scolarite".into(),
            niveau: n.map(Into::into),
            regime: None,
            qf_min_centimes: None,
            qf_max_centimes: None,
            montant_annuel_centimes: 1,
            tarif_par_defaut: false,
        };
        // article oublié : bloquant
        let v = controler_continuite(&[prec(art, None)], &[tarif(autre, None)], &[]);
        assert!(v.iter().any(|a| a.code == "article_oublie" && a.bloquante));
        // niveau qui n'a plus de tarif : bloquant
        let v = controler_continuite(&[prec(art, Some("6EME"))], &[tarif(art, Some("5EME"))], &[]);
        assert!(v.iter().any(|a| a.code == "tarif_perdu" && a.bloquante));
        // un tarif général couvre
        let v = controler_continuite(&[prec(art, Some("6EME"))], &[tarif(art, None)], &[]);
        assert!(v.is_empty());
        // article facturé sans tarif : signalé sans bloquer
        let v = controler_continuite(&[], &[tarif(art, None)], &[(autre, "Z".into())]);
        assert!(v
            .iter()
            .any(|a| a.code == "article_facture_sans_tarif" && !a.bloquante));
    }
}
