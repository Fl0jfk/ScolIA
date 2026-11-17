//! Logique d'écran des heures constatées, états mensuels, paie et DSN (socle 15, A11d) : **pure**, sans
//! Leptos ni réseau, testée en natif.
//!
//! Le serveur reste l'autorité (droit par action **et** périmètre d'employeur, §23.3) : ici on présente et on
//! ne propose pas ce qui serait refusé à coup sûr. **Règle R-AP28** : une heure n'est constatée que sur une
//! séance assurée (appel fait, ou attestation du remplaçant) ; l'écran ne propose donc de **saisir** que les
//! natures sans séance (`NATURES_SAISIE`), jamais le remplacement de courte durée ni le rattrapage dû.

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use ent_types::absences_personnels::{BilanValidationDto, EtatLigneDto, EtatMensuelDto, HeureDto};
use ent_types::personnels::AgentResumeDto;
use ent_web_ui::TonEtat;
use uuid::Uuid;

use crate::noyau::temps::nom_mois;

// ───────────────────────────── Libellés ─────────────────────────────

/// Natures d'heure connues du serveur (`NATURES_HEURE`).
pub const NATURES_HEURE: &[&str] = &[
    "rcd",
    "hse",
    "mission_pacte",
    "rattrapage_du",
    "service_non_fait",
    "vacation",
];

/// Natures qu'on peut **saisir** sans séance (`NATURES_SAISIE` du serveur) : jamais `rcd` ni `rattrapage_du`,
/// qui naissent d'une séance assurée (R-AP28).
pub const NATURES_SAISIE: &[&str] = &["hse", "mission_pacte", "vacation", "service_non_fait"];

pub fn libelle_nature_heure(nature: &str) -> &'static str {
    match nature {
        "rcd" => "Remplacement de courte durée",
        "hse" => "Heure supplémentaire effective",
        "mission_pacte" => "Mission Pacte",
        "rattrapage_du" => "Rattrapage dû (solde une dette)",
        "service_non_fait" => "Service non fait (retenue à signaler)",
        "vacation" => "Vacation",
        _ => "Nature inconnue",
    }
}

pub fn libelle_etat_heure(etat: &str) -> &'static str {
    match etat {
        "constatee" => "Constatée",
        "contestee" => "Contestée",
        "validee" => "Validée",
        "figee" => "Figée dans un état",
        "exportee" => "Exportée",
        _ => "État inconnu",
    }
}

pub fn ton_etat_heure(etat: &str) -> TonEtat {
    match etat {
        "contestee" => TonEtat::Avertissement,
        "validee" => TonEtat::Ok,
        "figee" | "exportee" => TonEtat::Info,
        _ => TonEtat::Neutre,
    }
}

pub fn libelle_preuve(preuve: &str) -> &'static str {
    match preuve {
        "appel" => "Appel fait",
        "attestation" => "Attestation du remplaçant",
        "saisie" => "Saisie sans séance",
        _ => "Preuve inconnue",
    }
}

pub fn libelle_employeur(employeur: &str) -> &'static str {
    match employeur {
        "etat" => "Agents de l'État",
        "ogec" => "Salariés de l'OGEC",
        _ => "Employeur inconnu",
    }
}

pub fn libelle_etat_mensuel(etat: &str) -> &'static str {
    match etat {
        "fige" => "Figé, à signer",
        "signe" => "Signé",
        "exporte" => "Exporté",
        _ => "État inconnu",
    }
}

pub fn ton_etat_mensuel(etat: &str) -> TonEtat {
    match etat {
        "fige" => TonEtat::Avertissement,
        "signe" | "exporte" => TonEtat::Ok,
        _ => TonEtat::Neutre,
    }
}

/// Motifs codés d'une contestation, d'une correction ou d'une régularisation (`RAISONS` du serveur).
pub const RAISONS_HEURE: &[(&str, &str)] = &[
    ("duree_erronee", "Durée erronée"),
    ("seance_non_assuree", "Séance non assurée"),
    ("mauvais_agent", "Mauvais agent"),
    ("doublon", "Doublon"),
    ("oubli", "Oubli"),
    ("autre", "Autre"),
];

pub fn libelle_unite(unite: &str) -> &'static str {
    match unite {
        "heure" => "h",
        "jour" => "j",
        "demi_jour" => "demi-j.",
        _ => "",
    }
}

pub fn libelle_etat_element(etat: &str) -> &'static str {
    match etat {
        "prepare" => "Préparé",
        "envoye" => "Envoyé à la paie",
        "regularise" => "Régularisation",
        _ => "État inconnu",
    }
}

pub fn ton_etat_element(etat: &str) -> TonEtat {
    match etat {
        "prepare" => TonEtat::Info,
        "envoye" => TonEtat::Ok,
        "regularise" => TonEtat::Avertissement,
        _ => TonEtat::Neutre,
    }
}

/// Employeurs que le rôle peut parcourir (miroir de `employeurs_permis` du serveur, qui reste juge) : la
/// direction les deux, le secrétariat l'État, la comptabilité l'OGEC.
pub fn employeurs_offerts(role: &str) -> &'static [&'static str] {
    match role {
        "direction" => &["etat", "ogec"],
        "administratif" => &["etat"],
        "compta" => &["ogec"],
        _ => &[],
    }
}

// ───────────────────────────── Durées ─────────────────────────────

/// « 1 h 30 », « 45 min », « 2 h » : durée lisible d'un nombre de minutes (signé : une régularisation à la
/// baisse est négative).
pub fn libelle_duree(minutes: i64) -> String {
    let signe = if minutes < 0 { "−" } else { "" };
    let m = minutes.unsigned_abs();
    let (h, reste) = (m / 60, m % 60);
    match (h, reste) {
        (0, r) => format!("{signe}{r} min"),
        (h, 0) => format!("{signe}{h} h"),
        (h, r) => format!("{signe}{h} h {r:02}"),
    }
}

/// Durée effective d'une heure : la durée figée plus les régularisations (jamais réécrite).
pub fn duree_effective(h: &HeureDto) -> i64 {
    i64::from(h.duree_minutes) + i64::from(h.regularisation_minutes)
}

/// Lit une durée saisie : « 1h30 », « 1 h 30 », « 2h », « 45 », « 45 min », « 1:30 » (un nombre seul = minutes).
/// 1 minute à 24 heures, comme le serveur.
pub fn analyser_duree(texte: &str) -> Result<i32, &'static str> {
    const ERREUR: &str = "Indiquez une durée, par exemple 1 h 30 ou 45 min.";
    let t = texte.trim().to_lowercase().replace(['\u{a0}', ' '], "");
    if t.is_empty() {
        return Err(ERREUR);
    }
    let nombre = |s: &str| -> Result<i64, &'static str> { s.parse::<i64>().map_err(|_| ERREUR) };
    let minutes = if let Some((h, m)) = t.split_once(['h', ':']) {
        let m = m.trim_end_matches("min").trim_end_matches('m');
        let h = if h.is_empty() { 0 } else { nombre(h)? };
        let m = if m.is_empty() { 0 } else { nombre(m)? };
        if m >= 60 {
            return Err("Les minutes vont de 0 à 59.");
        }
        h * 60 + m
    } else {
        nombre(t.trim_end_matches("min").trim_end_matches('m'))?
    };
    if !(1..=1440).contains(&minutes) {
        return Err("La durée va de 1 minute à 24 heures.");
    }
    i32::try_from(minutes).map_err(|_| ERREUR)
}

// ───────────────────────────── Mois ─────────────────────────────

pub fn premier_du_mois(d: NaiveDate) -> NaiveDate {
    d.with_day(1).unwrap_or(d)
}

/// « novembre 2026 ».
pub fn libelle_mois(mois: NaiveDate) -> String {
    format!("{} {}", nom_mois(mois.month()), mois.year())
}

fn mois_avant(mois: NaiveDate) -> NaiveDate {
    let (a, m) = if mois.month() == 1 {
        (mois.year() - 1, 12)
    } else {
        (mois.year(), mois.month() - 1)
    };
    NaiveDate::from_ymd_opt(a, m, 1).unwrap_or(mois)
}

/// Le mois est terminé : l'état ne se fige qu'après son dernier jour (règle du serveur).
pub fn mois_termine(mois: NaiveDate, aujourdhui: NaiveDate) -> bool {
    premier_du_mois(mois) < premier_du_mois(aujourdhui)
}

/// Les `n` derniers mois **terminés**, du plus récent au plus ancien (états à figer).
pub fn derniers_mois_termines(aujourdhui: NaiveDate, n: usize) -> Vec<NaiveDate> {
    let mut v = Vec::with_capacity(n);
    let mut m = mois_avant(premier_du_mois(aujourdhui));
    for _ in 0..n {
        v.push(m);
        m = mois_avant(m);
    }
    v
}

/// Le mois courant puis les `n - 1` précédents, pour parcourir les heures.
pub fn derniers_mois(aujourdhui: NaiveDate, n: usize) -> Vec<NaiveDate> {
    let mut v = vec![premier_du_mois(aujourdhui)];
    v.extend(derniers_mois_termines(aujourdhui, n.saturating_sub(1)));
    v
}

// ───────────────────────────── Noms des agents ─────────────────────────────

/// Annuaire des agents de l'établissement, pour afficher un nom à la place d'un identifiant.
#[derive(Clone, Debug, Default)]
pub struct Noms(HashMap<Uuid, String>);

impl Noms {
    pub fn depuis_annuaire(agents: &[AgentResumeDto]) -> Self {
        Self(
            agents
                .iter()
                .map(|a| (a.agent_id, a.nom_affiche.trim().to_string()))
                .filter(|(_, n)| !n.is_empty())
                .collect(),
        )
    }

    /// Agents à proposer dans une liste (nom, puis identifiant en valeur), par ordre alphabétique.
    pub fn options(&self) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = self
            .0
            .iter()
            .map(|(id, n)| (id.to_string(), n.clone()))
            .collect();
        v.sort_by_key(|a| sans_accent_min(&a.1));
        v
    }

    /// Le nom, ou « Agent non listé » : jamais un identifiant brut à l'écran.
    pub fn nom(&self, agent: Uuid) -> String {
        self.0
            .get(&agent)
            .cloned()
            .unwrap_or_else(|| "Agent non listé".to_string())
    }
}

fn sans_accent_min(t: &str) -> String {
    super::modele::sans_accent(t).to_lowercase()
}

/// L'agent correspond-il à la recherche (nom, sans accent ni casse) ?
pub fn nom_correspond(nom: &str, recherche: &str) -> bool {
    let r = sans_accent_min(recherche.trim());
    r.is_empty() || sans_accent_min(nom).contains(&r)
}

// ───────────────────────────── Regroupements ─────────────────────────────

/// Total par nature, dans l'ordre des natures du serveur, sans les natures absentes.
pub fn totaux_par_nature(heures: &[HeureDto]) -> Vec<(&'static str, i64)> {
    NATURES_HEURE
        .iter()
        .filter_map(|n| {
            let somme: i64 = heures
                .iter()
                .filter(|h| h.nature == *n)
                .map(duree_effective)
                .sum();
            let present = heures.iter().any(|h| h.nature == *n);
            present.then_some((*n, somme))
        })
        .collect()
}

/// Heures regroupées par mois (le plus récent d'abord), chaque groupe trié par date puis nature.
pub fn par_mois(heures: &[HeureDto]) -> Vec<(NaiveDate, Vec<HeureDto>)> {
    let mut mois: Vec<NaiveDate> = heures.iter().map(|h| h.mois).collect();
    mois.sort_unstable_by(|a, b| b.cmp(a));
    mois.dedup();
    mois.into_iter()
        .map(|m| {
            let mut g: Vec<HeureDto> = heures.iter().filter(|h| h.mois == m).cloned().collect();
            g.sort_by(|a, b| b.le.cmp(&a.le).then_with(|| a.nature.cmp(&b.nature)));
            (m, g)
        })
        .collect()
}

/// Une heure peut être contestée tant qu'elle n'est ni figée ni exportée (sinon : régularisation).
pub fn peut_contester(etat: &str) -> bool {
    matches!(etat, "constatee" | "validee")
}

/// Corriger : heure non figée (les contestées se règlent par une correction).
pub fn peut_corriger(etat: &str) -> bool {
    matches!(etat, "constatee" | "contestee" | "validee")
}

/// Régulariser : heure figée ou exportée, sur l'état suivant, jamais une réécriture.
pub fn peut_regulariser(etat: &str) -> bool {
    matches!(etat, "figee" | "exportee")
}

pub fn nb_a_regler(heures: &[HeureDto]) -> usize {
    heures.iter().filter(|h| h.etat == "contestee").count()
}

pub fn nb_a_valider(heures: &[HeureDto]) -> usize {
    heures.iter().filter(|h| h.etat == "constatee").count()
}

/// Phrase du bilan d'une validation (ABP-203) : ce qui est laissé à un autre ou contesté est dit.
pub fn phrase_bilan_validation(b: &BilanValidationDto) -> String {
    let mut p = vec![match b.validees {
        0 => "Aucune heure à valider.".to_string(),
        1 => "1 heure validée.".to_string(),
        n => format!("{n} heures validées."),
    }];
    if b.laissees_a_un_autre > 0 {
        p.push(format!(
            "{} de vos propres heures sont laissées à une autre personne.",
            b.laissees_a_un_autre
        ));
    }
    if b.contestees_restantes > 0 {
        p.push(format!(
            "{} heure(s) contestée(s) restent à régler avant de figer l'état.",
            b.contestees_restantes
        ));
    }
    p.join(" ")
}

/// Total par agent des lignes d'un état (signées : les régularisations à la baisse sont négatives).
pub fn totaux_par_agent(lignes: &[EtatLigneDto]) -> Vec<(Uuid, i64, bool)> {
    let mut ordre: Vec<Uuid> = Vec::new();
    for l in lignes {
        if !ordre.contains(&l.agent_id) {
            ordre.push(l.agent_id);
        }
    }
    ordre
        .into_iter()
        .map(|a| {
            let propres: Vec<&EtatLigneDto> = lignes.iter().filter(|l| l.agent_id == a).collect();
            let somme = propres.iter().map(|l| i64::from(l.duree_minutes)).sum();
            let regul = propres.iter().any(|l| l.regularisation_de_id.is_some());
            (a, somme, regul)
        })
        .collect()
}

/// Le chef d'établissement signe un état figé (jamais déjà signé) ; l'export suit la signature.
pub fn peut_signer(e: &EtatMensuelDto) -> bool {
    e.etat == "fige"
}

/// Identifiants des états rencontrés dans des heures : l'API ne liste pas les états, elle les relie à leurs heures.
pub fn etats_des_heures(heures: &[HeureDto]) -> Vec<Uuid> {
    let mut v: Vec<Uuid> = Vec::new();
    for h in heures {
        if let Some(e) = h.etat_mensuel_id {
            if !v.contains(&e) {
                v.push(e);
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Identifiant distinct à chaque appel (les identifiants aléatoires ne sont pas compilés pour le navigateur).
    fn identifiant() -> Uuid {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(1000);
        Uuid::from_u128(u128::from(N.fetch_add(1, Ordering::Relaxed)))
    }

    fn jour(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn heure(nature: &str, minutes: i32, regul: i32, etat: &str, le: NaiveDate) -> HeureDto {
        HeureDto {
            id: identifiant(),
            agent_id: Uuid::from_u128(1),
            seance_id: None,
            nature: nature.into(),
            duree_minutes: minutes,
            regularisation_minutes: regul,
            le,
            mois: premier_du_mois(le),
            employeur: "etat".into(),
            etat: etat.into(),
            preuve: "appel".into(),
            etat_mensuel_id: None,
        }
    }

    #[test]
    fn on_ne_saisit_jamais_une_heure_de_remplacement() {
        // R-AP28 : le remplacement de courte durée et le rattrapage dû naissent d'une séance assurée.
        assert!(!NATURES_SAISIE.contains(&"rcd"));
        assert!(!NATURES_SAISIE.contains(&"rattrapage_du"));
        for n in NATURES_SAISIE {
            assert!(NATURES_HEURE.contains(n), "{n}");
        }
        for n in NATURES_HEURE {
            assert_ne!(libelle_nature_heure(n), "Nature inconnue", "{n}");
        }
    }

    #[test]
    fn chaque_role_ne_parcourt_que_son_employeur() {
        assert_eq!(employeurs_offerts("direction"), &["etat", "ogec"]);
        assert_eq!(employeurs_offerts("administratif"), &["etat"]);
        assert_eq!(employeurs_offerts("compta"), &["ogec"]);
        assert!(employeurs_offerts("prof").is_empty());
        assert!(employeurs_offerts("parent").is_empty());
    }

    #[test]
    fn les_durees_sont_lisibles() {
        assert_eq!(libelle_duree(0), "0 min");
        assert_eq!(libelle_duree(45), "45 min");
        assert_eq!(libelle_duree(60), "1 h");
        assert_eq!(libelle_duree(90), "1 h 30");
        assert_eq!(libelle_duree(125), "2 h 05");
        assert_eq!(libelle_duree(-30), "−30 min");
    }

    #[test]
    fn la_duree_saisie_suit_les_bornes_du_serveur() {
        assert_eq!(analyser_duree("1h30"), Ok(90));
        assert_eq!(analyser_duree(" 1 h 30 "), Ok(90));
        assert_eq!(analyser_duree("2h"), Ok(120));
        assert_eq!(analyser_duree("1:30"), Ok(90));
        assert_eq!(analyser_duree("45"), Ok(45));
        assert_eq!(analyser_duree("45 min"), Ok(45));
        assert_eq!(analyser_duree("1h05min"), Ok(65));
        assert_eq!(analyser_duree("24h"), Ok(1440));
        assert!(analyser_duree("").is_err());
        assert!(analyser_duree("0").is_err());
        assert!(analyser_duree("25h").is_err());
        assert!(analyser_duree("1h75").is_err());
        assert!(analyser_duree("beaucoup").is_err());
        assert!(analyser_duree("-5").is_err());
    }

    #[test]
    fn la_duree_effective_ajoute_les_regularisations() {
        let h = heure("hse", 60, -15, "exportee", jour(2026, 10, 12));
        assert_eq!(duree_effective(&h), 45);
        let h = heure("hse", 60, 30, "figee", jour(2026, 10, 12));
        assert_eq!(duree_effective(&h), 90);
    }

    #[test]
    fn seuls_les_mois_termines_se_figent() {
        let auj = jour(2026, 11, 16);
        assert!(!mois_termine(jour(2026, 11, 1), auj));
        assert!(mois_termine(jour(2026, 10, 1), auj));
        assert_eq!(
            derniers_mois_termines(auj, 3),
            vec![jour(2026, 10, 1), jour(2026, 9, 1), jour(2026, 8, 1)]
        );
        // Passage d'année.
        assert_eq!(
            derniers_mois_termines(jour(2027, 1, 5), 2),
            vec![jour(2026, 12, 1), jour(2026, 11, 1)]
        );
        assert_eq!(
            derniers_mois(auj, 2),
            vec![jour(2026, 11, 1), jour(2026, 10, 1)]
        );
        assert_eq!(libelle_mois(jour(2026, 11, 1)), "novembre 2026");
    }

    #[test]
    fn le_cycle_d_une_heure_propose_la_bonne_action() {
        for etat in ["constatee", "validee"] {
            assert!(peut_contester(etat), "{etat}");
        }
        for etat in ["contestee", "figee", "exportee"] {
            assert!(!peut_contester(etat), "{etat}");
        }
        for etat in ["constatee", "contestee", "validee"] {
            assert!(peut_corriger(etat) && !peut_regulariser(etat), "{etat}");
        }
        for etat in ["figee", "exportee"] {
            assert!(peut_regulariser(etat) && !peut_corriger(etat), "{etat}");
        }
    }

    #[test]
    fn les_heures_se_regroupent_par_mois_et_par_nature() {
        let v = vec![
            heure("hse", 60, 0, "validee", jour(2026, 10, 3)),
            heure("rcd", 55, 0, "constatee", jour(2026, 11, 2)),
            heure("hse", 30, -10, "exportee", jour(2026, 10, 20)),
        ];
        let g = par_mois(&v);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].0, jour(2026, 11, 1));
        assert_eq!(g[1].1[0].le, jour(2026, 10, 20), "le plus récent d'abord");
        assert_eq!(totaux_par_nature(&v), vec![("rcd", 55), ("hse", 80)]);
        assert_eq!(nb_a_valider(&v), 1);
        assert_eq!(nb_a_regler(&v), 0);
    }

    #[test]
    fn le_bilan_de_validation_dit_ce_qui_reste() {
        let b = BilanValidationDto {
            validees: 3,
            laissees_a_un_autre: 1,
            contestees_restantes: 2,
        };
        let t = phrase_bilan_validation(&b);
        assert!(t.contains("3 heures validées"));
        assert!(t.contains("laissées à une autre personne"));
        assert!(t.contains("2 heure(s) contestée(s)"));
        assert_eq!(
            phrase_bilan_validation(&BilanValidationDto::default()),
            "Aucune heure à valider."
        );
    }

    #[test]
    fn un_etat_fige_se_signe_une_fois() {
        let mut e = EtatMensuelDto {
            id: identifiant(),
            mois: jour(2026, 10, 1),
            employeur: "etat".into(),
            etat: "fige".into(),
            fige_le: chrono::DateTime::UNIX_EPOCH,
            signe_le: None,
            exporte_le: None,
            total_minutes: 0,
            lignes: vec![],
        };
        assert!(peut_signer(&e));
        e.etat = "signe".into();
        assert!(!peut_signer(&e));
    }

    #[test]
    fn les_lignes_d_etat_se_totalisent_par_agent_avec_les_regularisations() {
        let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
        let ligne = |agent, minutes, regul| EtatLigneDto {
            id: identifiant(),
            agent_id: agent,
            nature: "hse".into(),
            duree_minutes: minutes,
            heure_id: None,
            regularisation_de_id: regul,
        };
        let t = totaux_par_agent(&[
            ligne(a, 60, None),
            ligne(b, 30, None),
            ligne(a, -15, Some(Uuid::from_u128(9))),
        ]);
        assert_eq!(t, vec![(a, 45, true), (b, 30, false)]);
    }

    #[test]
    fn la_recherche_par_nom_ignore_accents_et_casse() {
        assert!(nom_correspond("Hélène Martin", "helene"));
        assert!(nom_correspond("Hélène Martin", "  MARTIN "));
        assert!(nom_correspond("Hélène Martin", ""));
        assert!(!nom_correspond("Hélène Martin", "dupont"));
    }

    #[test]
    fn l_annuaire_se_propose_par_ordre_alphabetique() {
        let a = |n: u128, nom: &str| AgentResumeDto {
            agent_id: Uuid::from_u128(n),
            nom_affiche: nom.into(),
            fonction_code: None,
            fonction_libelle: None,
            famille: None,
            etat: "actif".into(),
            email: None,
            user_id: None,
            identite_restreinte: false,
        };
        let noms = Noms::depuis_annuaire(&[a(1, "Zoé Roux"), a(2, "Élodie Blanc"), a(3, "  ")]);
        let o = noms.options();
        assert_eq!(o.len(), 2, "un nom vide n'est pas listé");
        assert_eq!(o[0].1, "Élodie Blanc");
        assert_eq!(noms.nom(Uuid::from_u128(1)), "Zoé Roux");
    }

    #[test]
    fn un_agent_inconnu_n_affiche_jamais_son_identifiant() {
        let noms = Noms::default();
        let id = Uuid::from_u128(42);
        assert_eq!(noms.nom(id), "Agent non listé");
        assert!(!noms.nom(id).contains("0000"));
    }

    #[test]
    fn les_etats_se_retrouvent_par_leurs_heures() {
        let e = Uuid::from_u128(5);
        let mut h1 = heure("hse", 60, 0, "figee", jour(2026, 10, 3));
        let mut h2 = heure("hse", 60, 0, "figee", jour(2026, 10, 4));
        h1.etat_mensuel_id = Some(e);
        h2.etat_mensuel_id = Some(e);
        assert_eq!(etats_des_heures(&[h1, h2]), vec![e]);
    }
}
