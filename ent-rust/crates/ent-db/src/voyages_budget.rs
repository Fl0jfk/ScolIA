//! Socle 42, lot V0b : budget d'une sortie — **logique pure** (`docs/socles/42_SORTIES_VOYAGES.md`
//! §12.1, R-SV15 à R-SV17, NX-51, NX-53 à NX-55) et rappels de l'organisateur (NX-16).
//!
//! Rien ici ne touche la base : saisie stricte des montants au format français, calcul de la part
//! des familles (au centime près, arrondie à l'euro inférieur), équilibre et causes d'un déficit,
//! comparaison avec le prix annoncé figé, marge de sécurité préréglée, échéances des rappels.
//! Les commandes qui écrivent (droit, état, version, journal, outbox dans **une** transaction)
//! sont dans `voyages_budget_repo.rs`.

use serde::Serialize;

pub const POSTES_DEPENSE: [&str; 8] = [
    "transport",
    "hebergement",
    "restauration",
    "activites",
    "assurance",
    "divers",
    "encadrement",
    "surcout_handicap",
];

pub const POSTES_RECETTE: [&str; 7] = [
    "etablissement",
    "apel",
    "collectivite",
    "subvention_europeenne",
    "don_affecte",
    "ventes_eleves",
    "aides_individuelles",
];

pub const AFFECTATIONS: [&str; 2] = ["encadrement", "general"];

/// Plafond d'un montant saisi : un milliard d'euros, en centimes (aligné sur la contrainte SQL).
pub const MONTANT_MAX_CENTIMES: i64 = 100_000_000_000;

pub fn poste_valide(sens: &str, poste: &str) -> bool {
    match sens {
        "depense" => POSTES_DEPENSE.contains(&poste),
        "recette" => POSTES_RECETTE.contains(&poste),
        _ => false,
    }
}

/// Libellé français d'un poste (écrans et PDF).
pub fn libelle_poste(poste: &str) -> &'static str {
    match poste {
        "transport" => "Transport",
        "hebergement" => "Hébergement",
        "restauration" => "Restauration",
        "activites" => "Activités et entrées",
        "assurance" => "Assurance",
        "divers" => "Frais divers",
        "encadrement" => "Encadrement (transport, hébergement et repas des accompagnateurs)",
        "surcout_handicap" => "Surcoût lié au handicap ou à un besoin médical",
        "etablissement" => "Établissement",
        "apel" => "APEL (collectif)",
        "collectivite" => "Collectivité",
        "subvention_europeenne" => "Subvention européenne (Erasmus+)",
        "don_affecte" => "Don affecté",
        "ventes_eleves" => "Ventes et actions des élèves",
        "aides_individuelles" => "Aides individuelles (total agrégé, sans nom)",
        _ => "Autre",
    }
}

// ───────────────────────── Montants (R-SV16) ─────────────────────────

/// Saisie **stricte** d'un montant au format français : virgule décimale (0 à 2 chiffres), espace
/// des milliers (espace, espace insécable ou fine insécable, par groupes de trois). « 1.350 » est
/// **refusé** (point = ambiguïté entre milliers et décimales), comme le signe, l'exposant et tout
/// symbole : c'est l'interface qui propose le format, le serveur ne devine jamais.
pub fn montant_en_centimes(saisie: &str) -> Result<i64, &'static str> {
    const FORMAT: &str = "Montant invalide : écrivez par exemple 1 350,50 (virgule décimale, deux chiffres au plus).";
    let t = saisie.trim();
    if t.is_empty() {
        return Err("Le montant est obligatoire.");
    }
    let (entier, decimales) = match t.split_once(',') {
        Some((e, d)) => (e, Some(d)),
        None => (t, None),
    };
    if let Some(d) = decimales {
        if d.is_empty() || d.len() > 2 || !d.bytes().all(|b| b.is_ascii_digit()) {
            return Err(FORMAT);
        }
    }
    let est_espace = |c: char| matches!(c, ' ' | '\u{a0}' | '\u{202f}');
    let chiffres: String = if entier.chars().any(est_espace) {
        // Groupes : 1 à 3 chiffres, puis des groupes de exactement 3.
        let groupes: Vec<&str> = entier.split(est_espace).collect();
        let bien_forme = groupes.len() > 1
            && groupes.iter().enumerate().all(|(i, g)| {
                g.bytes().all(|b| b.is_ascii_digit())
                    && if i == 0 {
                        (1..=3).contains(&g.len())
                    } else {
                        g.len() == 3
                    }
            });
        if !bien_forme {
            return Err(FORMAT);
        }
        groupes.concat()
    } else {
        if entier.is_empty() || !entier.bytes().all(|b| b.is_ascii_digit()) {
            return Err(FORMAT);
        }
        entier.to_string()
    };
    let euros: i64 = chiffres.parse().map_err(|_| FORMAT)?;
    let mut centimes = euros.checked_mul(100).ok_or(FORMAT)?;
    if let Some(d) = decimales {
        let v: i64 = d.parse().map_err(|_| FORMAT)?;
        centimes += if d.len() == 1 { v * 10 } else { v };
    }
    if centimes > MONTANT_MAX_CENTIMES {
        return Err("Montant trop élevé.");
    }
    Ok(centimes)
}

/// « 1 350,50 € » (espaces insécables) ; le signe moins précède le montant.
pub fn euros(centimes: i64) -> String {
    let neg = centimes < 0;
    let abs = centimes.unsigned_abs();
    let (e, c) = (abs / 100, abs % 100);
    let brut = e.to_string();
    let mut groupes = String::new();
    for (i, ch) in brut.chars().enumerate() {
        if i > 0 && (brut.len() - i) % 3 == 0 {
            groupes.push('\u{a0}');
        }
        groupes.push(ch);
    }
    format!("{}{groupes},{c:02}\u{a0}€", if neg { "-" } else { "" })
}

// ───────────────────────── Marge de sécurité (NX-53) ─────────────────────────

/// Marge préréglée : 0 % de proximité, 5 % pour une sortie pédagogique, 7 % pour un voyage en France
/// (ou frontalier), 10 % à l'étranger. Modifiable avec un motif ; [H] à confirmer avec la comptabilité.
pub fn marge_prereglee(forme: &str, destination: &str) -> i32 {
    match destination {
        "proximite" => 0,
        "etranger" => 10,
        _ if forme == "voyage" => 7,
        _ => 5,
    }
}

// ───────────────────────── Calcul de la part des familles (R-SV15) ─────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct LigneCalc<'a> {
    pub sens: &'a str,
    pub poste: &'a str,
    pub montant_centimes: i64,
    pub affectation: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Calcul {
    pub nb_payants: i64,
    pub depenses_total: i64,
    /// Dépenses hors encadrement et hors surcoût handicap : la base de la part des familles.
    pub depenses_familles: i64,
    pub encadrement: i64,
    pub surcout_handicap: i64,
    pub recettes_total: i64,
    /// Recettes affectées à l'encadrement (couvrent l'encadrement et le surcoût, pas la part).
    pub recettes_encadrement: i64,
    /// Recettes hors familles qui réduisent la base de la part.
    pub recettes_reduisant_la_part: i64,
    pub marge_pct: i64,
    pub marge: i64,
    /// Coût réel par élève (plafond de la part) ; `None` sans participant payant.
    pub cout_reel_par_eleve: Option<i64>,
    /// Part des familles, **fixe**, arrondie à l'euro inférieur ; `None` tant que le nombre de
    /// participants payants n'est pas renseigné. Toujours 0 pour une sortie obligatoire (R-SV8).
    pub part_familles: Option<i64>,
    /// Ce que paient les familles au total (part × participants payants : somme exacte).
    pub total_familles: i64,
    /// Dépenses − recettes − participation des familles, plancher 0.
    pub deficit: i64,
    /// Part du déficit due à l'arrondi à l'euro inférieur : prise en charge par l'établissement.
    pub arrondi_pris_en_charge: i64,
    pub excedent: i64,
    pub equilibre: bool,
    pub causes: Vec<String>,
}

fn plancher(x: i128) -> i64 {
    x.clamp(0, i64::MAX as i128) as i64
}

/// Part des familles (R-SV15) = (dépenses − encadrement − surcoût handicap − recettes hors familles
/// + marge) ÷ participants payants, **fixe**, arrondie à l'**euro inférieur** dans l'intérêt des
///   familles (Q-V4, aligné sur `24_FINANCES.md` §1.6), au plus égale au coût réel par élève.
///
/// Les recettes **affectées à l'encadrement** couvrent l'encadrement et le surcoût (jamais la part) ;
/// les autres réduisent la base. Une sortie obligatoire a une part de 0, imposée (R-SV8).
/// **Équilibre** : aucun déficit hors arrondi (l'arrondi à l'euro inférieur est pris en charge par
/// l'établissement et affiché). Un budget déséquilibré ne se valide pas ; les causes sont listées.
pub fn calculer(
    lignes: &[LigneCalc<'_>],
    nb_payants: i64,
    marge_pct: i64,
    obligatoire: bool,
) -> Calcul {
    let somme = |f: &dyn Fn(&LigneCalc<'_>) -> bool| -> i128 {
        lignes
            .iter()
            .filter(|l| f(l))
            .map(|l| l.montant_centimes.max(0) as i128)
            .sum()
    };
    let depenses_total = somme(&|l| l.sens == "depense");
    let encadrement = somme(&|l| l.sens == "depense" && l.poste == "encadrement");
    let surcout = somme(&|l| l.sens == "depense" && l.poste == "surcout_handicap");
    let depenses_familles = depenses_total - encadrement - surcout;
    let recettes_total = somme(&|l| l.sens == "recette");
    let rec_enc = somme(&|l| l.sens == "recette" && l.affectation == Some("encadrement"));
    let rec_part = recettes_total - rec_enc;
    let nb = nb_payants.max(0) as i128;
    let marge_pct = marge_pct.clamp(0, 30) as i128;

    let net = (depenses_familles - rec_part).max(0);
    // Marge arrondie au centime supérieur (prudence), uniquement sur ce que les familles financent.
    let marge = (net * marge_pct + 99) / 100;
    let mut causes = Vec::new();

    let (part, cout_reel): (Option<i128>, Option<i128>) = if obligatoire {
        (Some(0), if nb > 0 { Some(depenses_familles / nb) } else { None })
    } else if nb == 0 {
        (None, None)
    } else {
        let cout = depenses_familles / nb;
        let brut = ((net + marge) / nb).min(cout);
        (Some(brut - brut % 100), Some(cout))
    };
    let total_familles = part.map_or(0, |p| p * nb);
    let deficit = (depenses_total - recettes_total - total_familles).max(0);
    let excedent = (recettes_total + total_familles - depenses_total).max(0);
    let arrondi = if obligatoire || part.is_none() {
        0
    } else {
        (net - total_familles).max(0).min(deficit)
    };
    let deficit_reel = deficit - arrondi;

    if part.is_none() && depenses_total > 0 {
        causes.push("Renseignez le nombre de participants payants.".to_string());
    }
    if deficit_reel > 0 {
        if obligatoire {
            causes.push(format!(
                "Sortie obligatoire : aucune participation des familles ; {} restent à couvrir par une recette (établissement, collectivité…).",
                euros(plancher(deficit_reel))
            ));
        } else {
            causes.push(format!(
                "L'encadrement et le surcoût lié au handicap ({}) ne sont couverts qu'à hauteur de {} par des recettes affectées à l'encadrement : il manque {}.",
                euros(plancher(encadrement + surcout)),
                euros(plancher(rec_enc)),
                euros(plancher(deficit_reel))
            ));
        }
    }
    Calcul {
        nb_payants: nb as i64,
        depenses_total: plancher(depenses_total),
        depenses_familles: plancher(depenses_familles),
        encadrement: plancher(encadrement),
        surcout_handicap: plancher(surcout),
        recettes_total: plancher(recettes_total),
        recettes_encadrement: plancher(rec_enc),
        recettes_reduisant_la_part: plancher(rec_part),
        marge_pct: marge_pct as i64,
        marge: plancher(marge),
        cout_reel_par_eleve: cout_reel.map(plancher),
        part_familles: part.map(plancher),
        total_familles: plancher(total_familles),
        deficit: plancher(deficit),
        arrondi_pris_en_charge: plancher(arrondi),
        excedent: plancher(excedent),
        equilibre: part.is_some() && deficit_reel == 0,
        causes,
    }
}

// ───────────────────────── Prix annoncé figé (NX-55, R-SV17) ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcartPrix {
    /// Aucun prix annoncé : le premier est figé à l'ouverture.
    Premier,
    Identique,
    /// Baisse : toujours possible ; produit un avoir au profit de chaque payeur (V2).
    Baisse,
    /// Hausse : décision motivée de la direction et nouvel avis aux familles.
    Hausse,
}

pub fn comparer_prix(valide: i64, fige: Option<i64>) -> EcartPrix {
    match fige {
        None => EcartPrix::Premier,
        Some(f) if valide == f => EcartPrix::Identique,
        Some(f) if valide < f => EcartPrix::Baisse,
        Some(_) => EcartPrix::Hausse,
    }
}

// ───────────────────────── Rappels de l'organisateur (NX-16) ─────────────────────────

/// Un rappel dû : code stable (une ligne par sortie et par code) et message pour le porteur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RappelDu {
    pub code: &'static str,
    pub message: &'static str,
}

pub const CODES_RAPPEL: [&str; 5] = [
    "reunion_parents",
    "dossier_bloque",
    "effectif",
    "liste_car",
    "communication_parents",
];

/// Message du porteur pour un code de rappel (texte fixe : aucun texte libre n'y est recopié).
pub fn message_rappel(code: &str) -> &'static str {
    match code {
        "reunion_parents" => {
            "Planifiez la réunion d'information des parents (obligatoire pour un voyage)."
        }
        "dossier_bloque" => {
            "Le dossier n'est pas encore autorisé à quinze jours du départ : relancez l'étape en cours."
        }
        "effectif" => "La liste des participants n'est pas confirmée à une semaine du départ.",
        "liste_car" => "Transmettez la liste des participants au transporteur (J-4).",
        "communication_parents" => {
            "Jour du départ : vérifiez la dernière information donnée aux familles."
        }
        _ => "Rappel de sortie.",
    }
}

/// États où un rappel a un sens : la sortie est engagée mais pas encore partie.
pub fn rappelable(etat: &str) -> bool {
    matches!(
        etat,
        "soumise"
            | "a_completer"
            | "preparation"
            | "a_signer"
            | "budget_a_valider"
            | "vote"
            | "a_autoriser"
            | "ouverte"
            | "prete"
    )
}

/// Rappels dus à `jours_avant` jours du départ (date de Paris, R-SV29), 0 le jour même. Repris de
/// Next.js (J-14, J-7, J-4, J0 ; J-15 cuisine en V0d) et complétés (réunion des parents à J-30,
/// §11.1). Un rappel manqué un jour (service arrêté) est rattrapé le jour suivant : les seuils
/// sont des « au plus tard ».
pub fn rappels_dus(
    jours_avant: i64,
    etat: &str,
    forme: &str,
    besoin_car: bool,
    liste_confirmee: bool,
) -> Vec<RappelDu> {
    let mut codes: Vec<&'static str> = Vec::new();
    if jours_avant < 0 || !rappelable(etat) {
        return Vec::new();
    }
    if jours_avant <= 30 && forme == "voyage" {
        codes.push("reunion_parents");
    }
    let autorisee = matches!(etat, "ouverte" | "prete");
    if jours_avant <= 14 && !autorisee {
        codes.push("dossier_bloque");
    }
    if jours_avant <= 7 && autorisee && !liste_confirmee {
        codes.push("effectif");
    }
    if jours_avant <= 4 && autorisee && besoin_car {
        codes.push("liste_car");
    }
    if jours_avant == 0 && autorisee {
        codes.push("communication_parents");
    }
    codes
        .into_iter()
        .map(|code| RappelDu { code, message: message_rappel(code) })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(poste: &'static str, c: i64) -> LigneCalc<'static> {
        LigneCalc { sens: "depense", poste, montant_centimes: c, affectation: None }
    }

    fn r(poste: &'static str, c: i64, aff: Option<&'static str>) -> LigneCalc<'static> {
        LigneCalc { sens: "recette", poste, montant_centimes: c, affectation: aff }
    }

    #[test]
    fn montants_au_format_francais_strict() {
        assert_eq!(montant_en_centimes("12"), Ok(1200));
        assert_eq!(montant_en_centimes("12,5"), Ok(1250));
        assert_eq!(montant_en_centimes("12,05"), Ok(1205));
        assert_eq!(montant_en_centimes("0,99"), Ok(99));
        assert_eq!(montant_en_centimes("1 350,50"), Ok(135_050));
        assert_eq!(montant_en_centimes("1\u{a0}350"), Ok(135_000));
        assert_eq!(montant_en_centimes("12\u{202f}345\u{202f}678,9"), Ok(1_234_567_890));
        assert_eq!(montant_en_centimes("  42 "), Ok(4200));
        // « 1.350 » est refusé : milliers ou décimales ? (R-SV16)
        for faux in [
            "1.350", "1,350", "1 35", "1 3500", "12 345 67", " ", "", "-5", "+5", "5 €", "1e3",
            "12,", ",5", "1,2,3", "abc", "1  350", "1 350 ,5", "٣",
        ] {
            assert!(montant_en_centimes(faux).is_err(), "« {faux} » doit être refusé");
        }
        assert!(montant_en_centimes("1 000 000 001").is_err(), "au-delà du plafond");
        assert!(montant_en_centimes("1 000 000 000").is_ok());
        assert!(montant_en_centimes("99999999999999999999").is_err());
    }

    #[test]
    fn affichage_des_euros() {
        assert_eq!(euros(135_050), "1\u{a0}350,50\u{a0}€");
        assert_eq!(euros(0), "0,00\u{a0}€");
        assert_eq!(euros(5), "0,05\u{a0}€");
        assert_eq!(euros(123_456_789), "1\u{a0}234\u{a0}567,89\u{a0}€");
        assert_eq!(euros(-100), "-1,00\u{a0}€");
    }

    #[test]
    fn marges_preregleess_selon_le_contexte() {
        assert_eq!(marge_prereglee("sans_nuitee", "proximite"), 0);
        assert_eq!(marge_prereglee("sans_nuitee", "france"), 5);
        assert_eq!(marge_prereglee("voyage", "france"), 7);
        assert_eq!(marge_prereglee("voyage", "frontalier"), 7);
        assert_eq!(marge_prereglee("voyage", "etranger"), 10);
        assert_eq!(marge_prereglee("sans_nuitee", "etranger"), 10);
    }

    #[test]
    fn part_exacte_sans_arrondi() {
        // 30 payants ; 1 500 € de car + 3 000 € d'hébergement = 150 € par élève, sans recette.
        let c = calculer(&[d("transport", 150_000), d("hebergement", 300_000)], 30, 0, false);
        assert_eq!(c.part_familles, Some(15_000));
        assert_eq!(c.total_familles, 450_000);
        assert_eq!(c.deficit, 0);
        assert!(c.equilibre && c.causes.is_empty());
    }

    #[test]
    fn part_arrondie_a_l_euro_inferieur_et_arrondi_pris_en_charge() {
        // 1 000 € / 30 = 33,33 € → 33 € ; les 10 € d'écart sont à la charge de l'établissement.
        let c = calculer(&[d("transport", 100_000)], 30, 0, false);
        assert_eq!(c.part_familles, Some(3_300));
        assert_eq!(c.total_familles, 99_000);
        assert_eq!(c.deficit, 1_000);
        assert_eq!(c.arrondi_pris_en_charge, 1_000);
        assert!(c.equilibre, "l'arrondi seul ne bloque pas la validation");
        assert_eq!(c.cout_reel_par_eleve, Some(3_333));
    }

    #[test]
    fn l_encadrement_et_le_surcout_sortent_de_la_part() {
        // 4 500 € pour les élèves, 600 € d'encadrement et 300 € de surcoût : la part reste 150 €.
        let l = [
            d("transport", 450_000),
            d("encadrement", 60_000),
            d("surcout_handicap", 30_000),
        ];
        let c = calculer(&l, 30, 0, false);
        assert_eq!(c.depenses_familles, 450_000);
        assert_eq!(c.part_familles, Some(15_000));
        // Non couverts : déséquilibré, cause lisible.
        assert_eq!(c.deficit, 90_000);
        assert!(!c.equilibre);
        assert_eq!(c.causes.len(), 1);
        assert!(c.causes[0].contains("encadrement"));
        // Une recette affectée à l'encadrement équilibre sans toucher la part.
        let mut l2 = l.to_vec();
        l2.push(r("etablissement", 90_000, Some("encadrement")));
        let c = calculer(&l2, 30, 0, false);
        assert_eq!(c.part_familles, Some(15_000));
        assert!(c.equilibre && c.deficit == 0);
        // Un don affecté à l'encadrement qui dépasse le besoin ne baisse pas la part : excédent.
        l2.push(r("don_affecte", 50_000, Some("encadrement")));
        let c = calculer(&l2, 30, 0, false);
        assert_eq!(c.part_familles, Some(15_000));
        assert_eq!(c.excedent, 50_000);
    }

    #[test]
    fn les_recettes_generales_reduisent_la_part() {
        // APEL 900 € : (4 500 − 900) / 30 = 120 €.
        let c = calculer(
            &[d("transport", 450_000), r("apel", 90_000, Some("general"))],
            30,
            0,
            false,
        );
        assert_eq!(c.part_familles, Some(12_000));
        assert!(c.equilibre);
        // Recettes supérieures aux dépenses : part nulle, excédent.
        let c = calculer(
            &[d("transport", 100_000), r("collectivite", 150_000, None)],
            10,
            0,
            false,
        );
        assert_eq!(c.part_familles, Some(0));
        assert_eq!(c.excedent, 50_000);
        assert!(c.equilibre);
    }

    #[test]
    fn marge_de_securite_et_plafond_au_cout_reel() {
        // 4 500 € − 1 500 € de recettes = 3 000 € ; marge 10 % = 300 € ; 3 300 / 30 = 110 €.
        let l = [d("transport", 450_000), r("subvention_europeenne", 150_000, None)];
        let c = calculer(&l, 30, 10, false);
        assert_eq!(c.marge, 30_000);
        assert_eq!(c.part_familles, Some(11_000));
        assert!(c.equilibre);
        // Sans recette, la marge ne peut pas dépasser le coût réel par élève (150 €).
        let c = calculer(&[d("transport", 450_000)], 30, 10, false);
        assert_eq!(c.part_familles, Some(15_000));
        assert_eq!(c.cout_reel_par_eleve, Some(15_000));
    }

    #[test]
    fn sortie_obligatoire_part_nulle_imposee() {
        let c = calculer(&[d("transport", 80_000)], 25, 0, true);
        assert_eq!(c.part_familles, Some(0));
        assert_eq!(c.total_familles, 0);
        assert!(!c.equilibre);
        assert!(c.causes[0].contains("obligatoire"));
        // Un arrondi n'est jamais toléré ici : l'établissement doit couvrir toute la dépense.
        let c = calculer(
            &[d("transport", 80_000), r("etablissement", 80_000, None)],
            25,
            0,
            true,
        );
        assert!(c.equilibre);
        let c = calculer(
            &[d("transport", 80_000), r("etablissement", 79_999, None)],
            25,
            0,
            true,
        );
        assert!(!c.equilibre, "un centime manquant bloque");
    }

    #[test]
    fn sans_participant_payant_pas_de_part() {
        let c = calculer(&[d("transport", 80_000)], 0, 0, false);
        assert_eq!(c.part_familles, None);
        assert!(!c.equilibre);
        assert!(c.causes[0].contains("participants payants"));
        // Un budget vide est calculable : rien à payer, mais « part » inconnue sans effectif.
        let c = calculer(&[], 12, 0, false);
        assert_eq!(c.part_familles, Some(0));
        assert!(c.equilibre);
    }

    #[test]
    fn somme_des_parts_exacte_et_sans_depassement() {
        // La somme des parts est toujours part × payants, jamais un reste réparti au hasard.
        for nb in [1_i64, 3, 7, 29, 31, 2000] {
            let c = calculer(&[d("activites", 1_234_567)], nb, 0, false);
            assert_eq!(c.total_familles, c.part_familles.unwrap() * nb);
            assert_eq!(c.part_familles.unwrap() % 100, 0);
            assert!(c.part_familles.unwrap() <= c.cout_reel_par_eleve.unwrap());
        }
        // Très gros montants : aucun dépassement d'entier.
        let gros = [d("transport", MONTANT_MAX_CENTIMES); 50];
        let c = calculer(&gros, 2000, 30, false);
        assert!(c.depenses_total > 0 && c.part_familles.is_some());
    }

    #[test]
    fn comparaison_avec_le_prix_fige() {
        assert_eq!(comparer_prix(15_000, None), EcartPrix::Premier);
        assert_eq!(comparer_prix(15_000, Some(15_000)), EcartPrix::Identique);
        assert_eq!(comparer_prix(14_000, Some(15_000)), EcartPrix::Baisse);
        assert_eq!(comparer_prix(16_000, Some(15_000)), EcartPrix::Hausse);
    }

    #[test]
    fn rappels_aux_echeances_de_next_js() {
        let codes = |j, etat, forme, car, liste| {
            rappels_dus(j, etat, forme, car, liste)
                .into_iter()
                .map(|r| r.code)
                .collect::<Vec<_>>()
        };
        // Loin du départ, rien (et la réunion des parents ne concerne que les voyages).
        assert!(codes(45, "ouverte", "voyage", true, false).is_empty());
        assert!(codes(30, "ouverte", "sans_nuitee", false, false).is_empty());
        assert_eq!(codes(30, "preparation", "voyage", false, false), ["reunion_parents"]);
        // J-14 : dossier non autorisé.
        assert_eq!(
            codes(14, "budget_a_valider", "sans_nuitee", false, false),
            ["dossier_bloque"]
        );
        assert!(!codes(14, "ouverte", "sans_nuitee", false, false).contains(&"dossier_bloque"));
        // J-7 : effectif non confirmé ; confirmé, plus de rappel.
        assert_eq!(codes(7, "ouverte", "sans_nuitee", false, false), ["effectif"]);
        assert!(codes(7, "prete", "sans_nuitee", false, true).is_empty());
        // J-4 : liste au car ; J0 : communication.
        assert_eq!(codes(4, "prete", "sans_nuitee", true, true), ["liste_car"]);
        assert_eq!(
            codes(0, "prete", "voyage", true, true),
            ["reunion_parents", "liste_car", "communication_parents"]
        );
        // Rattrapage : à J-2 sans rappel antérieur, ceux de J-7 et J-4 sont dus ensemble.
        assert_eq!(
            codes(2, "ouverte", "sans_nuitee", true, false),
            ["effectif", "liste_car"]
        );
        // Parti, passé, brouillon, refusé, annulé : jamais.
        for etat in ["brouillon", "en_cours", "rentree", "close", "refusee", "annulee"] {
            assert!(codes(3, etat, "voyage", true, false).is_empty(), "{etat}");
        }
        assert!(codes(-1, "ouverte", "voyage", true, false).is_empty());
    }

    #[test]
    fn postes_par_sens() {
        assert!(poste_valide("depense", "transport"));
        assert!(!poste_valide("depense", "apel"));
        assert!(poste_valide("recette", "apel"));
        assert!(!poste_valide("recette", "transport"));
        assert!(!poste_valide("autre", "transport"));
        for p in POSTES_DEPENSE.iter().chain(POSTES_RECETTE.iter()) {
            assert_ne!(libelle_poste(p), "Autre", "{p} sans libellé");
        }
    }
}
