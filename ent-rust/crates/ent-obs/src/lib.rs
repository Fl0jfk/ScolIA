//! Observabilité ENT — registre de métriques minimal au format Prometheus.
//!
//! Pourquoi pas `axum-prometheus` / `metrics-rs` : on veut **zéro dépendance
//! supplémentaire** dans le binaire de production, un rendu **déterministe**
//! (séries triées → les tests peuvent asserter sur le texte exact) et un coût
//! borné (atomics, pas de lock sur le chemin des requêtes chaudes).
//!
//! Trois types de séries :
//! - **compteur** (`AtomicU64`) : monotone, suffixe `_total` ajouté si absent ;
//! - **jauge** (`AtomicI64`) : valeur entière courante (add/set) ;
//! - **histogramme** : seaux cumulés + `_sum` (microsecondes) + `_count`.
//!
//! Les étiquettes sont **triées par nom** pour former la clé de série : deux
//! appels avec les mêmes étiquettes dans un ordre différent partagent la même
//! série (pas de doublon silencieux). Un nom déclaré avec deux genres
//! différents est refusé (retour `false`) au lieu de produire un export
//! incohérent.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Instant;

/// Bornes par défaut d'un histogramme de durées (secondes) : 1 ms → 60 s.
pub const BUCKETS_DEFAUT: &[f64] = &[
    0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0,
];

/// Genre d'une famille de métriques.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genre {
    Compteur,
    Jauge,
    Histogramme,
}

impl Genre {
    fn as_str(self) -> &'static str {
        match self {
            Genre::Compteur => "counter",
            Genre::Jauge => "gauge",
            Genre::Histogramme => "histogram",
        }
    }
}

struct Famille {
    genre: Genre,
    aide: String,
}

struct Histogramme {
    bornes: Vec<f64>,
    /// Seaux **cumulés** (`le` inclus) — même nombre d'entrées que `bornes`.
    seaux: Vec<AtomicU64>,
    somme_micros: AtomicU64,
    compte: AtomicU64,
}

impl Histogramme {
    fn nouveau(bornes: Vec<f64>) -> Self {
        let seaux = (0..bornes.len()).map(|_| AtomicU64::new(0)).collect();
        Self {
            bornes,
            seaux,
            somme_micros: AtomicU64::new(0),
            compte: AtomicU64::new(0),
        }
    }

    fn observer(&self, secondes: f64) {
        let micros = if secondes.is_finite() && secondes > 0.0 {
            (secondes * 1_000_000.0).round() as u64
        } else {
            0
        };
        for (i, borne) in self.bornes.iter().enumerate() {
            if secondes <= *borne {
                self.seaux[i].fetch_add(1, Ordering::Relaxed);
            }
        }
        self.somme_micros.fetch_add(micros, Ordering::Relaxed);
        self.compte.fetch_add(1, Ordering::Relaxed);
    }
}

enum Serie {
    Compteur(AtomicU64),
    Jauge(AtomicI64),
    Histogramme(Box<Histogramme>),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Cle {
    nom: String,
    etiquettes: Vec<(String, String)>,
}

#[derive(Default)]
struct Registre {
    familles: BTreeMap<String, Famille>,
    series: BTreeMap<Cle, Serie>,
}

/// Registre de métriques d'un processus. Clonable via `Arc`.
#[derive(Default)]
pub struct Metrics {
    registre: RwLock<Registre>,
}

impl std::fmt::Debug for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let registre = self.registre.read().unwrap_or_else(|e| e.into_inner());
        f.debug_struct("Metrics")
            .field("familles", &registre.familles.len())
            .field("series", &registre.series.len())
            .finish()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Déclare une famille. Idempotent si (nom, genre) sont identiques.
    /// Retourne `false` si le nom est déjà pris par un **autre** genre.
    pub fn declarer(&self, nom: &str, genre: Genre, aide: &str) -> bool {
        let mut registre = self.registre.write().unwrap_or_else(|e| e.into_inner());
        match registre.familles.get(nom) {
            Some(f) => f.genre == genre,
            None => {
                registre.familles.insert(
                    nom.to_string(),
                    Famille {
                        genre,
                        aide: aide.to_string(),
                    },
                );
                true
            }
        }
    }

    fn cle(nom: &str, etiquettes: &[(&str, &str)]) -> Cle {
        let mut paires: Vec<(String, String)> = etiquettes
            .iter()
            .filter(|(k, _)| etiquette_valide(k))
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        paires.sort();
        paires.dedup();
        Cle {
            nom: nom.to_string(),
            etiquettes: paires,
        }
    }

    /// Incrémente (ou crée) un compteur. Le suffixe `_total` est ajouté si le
    /// nom ne le porte pas déjà (convention Prometheus).
    pub fn compteur_ajouter(&self, nom: &str, etiquettes: &[(&str, &str)], delta: u64) {
        let nom = nom_compteur(nom);
        self.declarer(&nom, Genre::Compteur, "");
        let cle = Self::cle(&nom, etiquettes);
        let mut registre = self.registre.write().unwrap_or_else(|e| e.into_inner());
        match registre.series.entry(cle) {
            std::collections::btree_map::Entry::Occupied(o) => {
                if let Serie::Compteur(c) = o.get() {
                    c.fetch_add(delta, Ordering::Relaxed);
                }
            }
            std::collections::btree_map::Entry::Vacant(v) => {
                v.insert(Serie::Compteur(AtomicU64::new(delta)));
            }
        }
    }

    pub fn compteur_incr(&self, nom: &str, etiquettes: &[(&str, &str)]) {
        self.compteur_ajouter(nom, etiquettes, 1);
    }

    /// Fixe la valeur d'une jauge (entière).
    pub fn jauge_definir(&self, nom: &str, etiquettes: &[(&str, &str)], valeur: i64) {
        self.declarer(nom, Genre::Jauge, "");
        let cle = Self::cle(nom, etiquettes);
        let mut registre = self.registre.write().unwrap_or_else(|e| e.into_inner());
        match registre.series.entry(cle) {
            std::collections::btree_map::Entry::Occupied(o) => {
                if let Serie::Jauge(j) = o.get() {
                    j.store(valeur, Ordering::Relaxed);
                }
            }
            std::collections::btree_map::Entry::Vacant(v) => {
                v.insert(Serie::Jauge(AtomicI64::new(valeur)));
            }
        }
    }

    /// Ajoute (positif ou négatif) à une jauge — typiquement « en cours ».
    pub fn jauge_ajouter(&self, nom: &str, etiquettes: &[(&str, &str)], delta: i64) {
        self.declarer(nom, Genre::Jauge, "");
        let cle = Self::cle(nom, etiquettes);
        let mut registre = self.registre.write().unwrap_or_else(|e| e.into_inner());
        match registre.series.entry(cle) {
            std::collections::btree_map::Entry::Occupied(o) => {
                if let Serie::Jauge(j) = o.get() {
                    j.fetch_add(delta, Ordering::Relaxed);
                }
            }
            std::collections::btree_map::Entry::Vacant(v) => {
                v.insert(Serie::Jauge(AtomicI64::new(delta)));
            }
        }
    }

    /// Observe une durée (secondes) dans un histogramme (seaux par défaut).
    pub fn histogramme_observer(&self, nom: &str, etiquettes: &[(&str, &str)], secondes: f64) {
        self.histogramme_observer_bornes(nom, etiquettes, secondes, BUCKETS_DEFAUT)
    }

    /// Variante avec bornes explicites (histogrammes métier, ex. tailles).
    pub fn histogramme_observer_bornes(
        &self,
        nom: &str,
        etiquettes: &[(&str, &str)],
        secondes: f64,
        bornes: &[f64],
    ) {
        self.declarer(nom, Genre::Histogramme, "");
        let cle = Self::cle(nom, etiquettes);
        let mut registre = self.registre.write().unwrap_or_else(|e| e.into_inner());
        match registre.series.entry(cle) {
            std::collections::btree_map::Entry::Occupied(o) => {
                if let Serie::Histogramme(h) = o.get() {
                    h.observer(secondes);
                }
            }
            std::collections::btree_map::Entry::Vacant(v) => {
                let h = Histogramme::nouveau(bornes.to_vec());
                h.observer(secondes);
                v.insert(Serie::Histogramme(Box::new(h)));
            }
        }
    }

    /// Renseigne l'aide (`# HELP`) d'une famille déjà déclarée.
    pub fn aider(&self, nom: &str, aide: &str) {
        let mut registre = self.registre.write().unwrap_or_else(|e| e.into_inner());
        if let Some(f) = registre.familles.get_mut(nom) {
            if f.aide.is_empty() {
                f.aide = aide.to_string();
            }
        }
    }

    /// Valeur courante d'un compteur (tests / diagnostics).
    pub fn compteur_valeur(&self, nom: &str, etiquettes: &[(&str, &str)]) -> Option<u64> {
        let nom = nom_compteur(nom);
        let registre = self.registre.read().unwrap_or_else(|e| e.into_inner());
        match registre.series.get(&Self::cle(&nom, etiquettes)) {
            Some(Serie::Compteur(c)) => Some(c.load(Ordering::Relaxed)),
            _ => None,
        }
    }

    /// Valeur courante d'une jauge (tests / diagnostics).
    pub fn jauge_valeur(&self, nom: &str, etiquettes: &[(&str, &str)]) -> Option<i64> {
        let registre = self.registre.read().unwrap_or_else(|e| e.into_inner());
        match registre.series.get(&Self::cle(nom, etiquettes)) {
            Some(Serie::Jauge(j)) => Some(j.load(Ordering::Relaxed)),
            _ => None,
        }
    }

    /// Nombre de séries enregistrées (diagnostics).
    pub fn nombre_series(&self) -> usize {
        let registre = self.registre.read().unwrap_or_else(|e| e.into_inner());
        registre.series.len()
    }

    /// Export au format texte Prometheus (`text/plain; version=0.0.4`).
    ///
    /// Déterministe : familles et séries triées par nom puis étiquettes.
    pub fn rendre(&self) -> String {
        let registre = self.registre.read().unwrap_or_else(|e| e.into_inner());
        let mut out = String::with_capacity(4096);
        let mut dernier: Option<&str> = None;

        for (cle, serie) in &registre.series {
            if dernier != Some(cle.nom.as_str()) {
                if let Some(famille) = registre.familles.get(&cle.nom) {
                    let _ = writeln!(out, "# HELP {} {}", cle.nom, echapper_aide(&famille.aide));
                    let _ = writeln!(out, "# TYPE {} {}", cle.nom, famille.genre.as_str());
                }
                dernier = Some(cle.nom.as_str());
            }
            let libelles = rendre_etiquettes(&cle.etiquettes, None);
            match serie {
                Serie::Compteur(c) => {
                    let _ = writeln!(out, "{}{} {}", cle.nom, libelles, c.load(Ordering::Relaxed));
                }
                Serie::Jauge(j) => {
                    let _ = writeln!(out, "{}{} {}", cle.nom, libelles, j.load(Ordering::Relaxed));
                }
                Serie::Histogramme(h) => {
                    for (i, borne) in h.bornes.iter().enumerate() {
                        let le = format_borne(*borne).to_string();
                        let libelles =
                            rendre_etiquettes(&cle.etiquettes, Some(("le", le.as_str())));
                        let _ = writeln!(
                            out,
                            "{}_bucket{} {}",
                            cle.nom,
                            libelles,
                            h.seaux[i].load(Ordering::Relaxed)
                        );
                    }
                    let inf = rendre_etiquettes(&cle.etiquettes, Some(("le", "+Inf")));
                    let _ = writeln!(
                        out,
                        "{}_bucket{} {}",
                        cle.nom,
                        inf,
                        h.compte.load(Ordering::Relaxed)
                    );
                    let _ = writeln!(
                        out,
                        "{}_sum{} {}",
                        cle.nom,
                        rendre_etiquettes(&cle.etiquettes, None),
                        format_secondes(h.somme_micros.load(Ordering::Relaxed))
                    );
                    let _ = writeln!(
                        out,
                        "{}_count{} {}",
                        cle.nom,
                        rendre_etiquettes(&cle.etiquettes, None),
                        h.compte.load(Ordering::Relaxed)
                    );
                }
            }
        }
        out
    }
}

/// Chronomètre léger pour mesurer une opération et l'observer.
pub struct Mesure {
    debut: Instant,
}

impl Mesure {
    pub fn demarrer() -> Self {
        Self {
            debut: Instant::now(),
        }
    }

    pub fn ecoule_secondes(&self) -> f64 {
        self.debut.elapsed().as_secs_f64()
    }

    /// Observe la durée écoulée dans l'histogramme et la retourne.
    pub fn observer(&self, metrics: &Metrics, nom: &str, etiquettes: &[(&str, &str)]) -> f64 {
        let secondes = self.ecoule_secondes();
        metrics.histogramme_observer(nom, etiquettes, secondes);
        secondes
    }
}

impl Default for Mesure {
    fn default() -> Self {
        Self::demarrer()
    }
}

fn nom_compteur(nom: &str) -> String {
    if nom.ends_with("_total") {
        nom.to_string()
    } else {
        format!("{nom}_total")
    }
}

/// Un nom d'étiquette doit être un identifiant Prometheus
/// (`[a-zA-Z_][a-zA-Z0-9_]*`) ; sinon l'étiquette est ignorée (défense en
/// profondeur : jamais d'injection dans l'export).
fn etiquette_valide(nom: &str) -> bool {
    let mut chars = nom.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn echapper_valeur(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out
}

fn echapper_aide(v: &str) -> String {
    v.replace('\\', "\\\\").replace('\n', "\\n")
}

fn rendre_etiquettes(paires: &[(String, String)], extra: Option<(&str, &str)>) -> String {
    if paires.is_empty() && extra.is_none() {
        return String::new();
    }
    let mut morceaux: Vec<String> = paires
        .iter()
        .map(|(k, v)| format!("{k}=\"{}\"", echapper_valeur(v)))
        .collect();
    if let Some((k, v)) = extra {
        // `le` se place en fin de liste (convention d'affichage Prometheus).
        let insertion = morceaux.iter().position(|m| m.starts_with("le=\""));
        let element = format!("{k}=\"{}\"", echapper_valeur(v));
        match insertion {
            Some(i) => morceaux[i] = element,
            None => morceaux.push(element),
        }
    }
    format!("{{{}}}", morceaux.join(","))
}

fn format_borne(borne: f64) -> String {
    if borne == borne.trunc() && borne.abs() < 1e15 {
        format!("{}", borne as i64)
    } else {
        format!("{borne}")
    }
}

fn format_secondes(micros: u64) -> String {
    format!("{:.6}", micros as f64 / 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compteur_suffixe_total_et_somme() {
        let m = Metrics::new();
        m.compteur_incr("ent_demo_requetes", &[("route", "/a")]);
        m.compteur_ajouter("ent_demo_requetes", &[("route", "/a")], 4);
        let texte = m.rendre();
        assert!(
            texte.contains("# TYPE ent_demo_requetes_total counter"),
            "{texte}"
        );
        assert!(
            texte.contains("ent_demo_requetes_total{route=\"/a\"} 5"),
            "{texte}"
        );
        assert_eq!(
            m.compteur_valeur("ent_demo_requetes_total", &[("route", "/a")]),
            Some(5)
        );
    }

    #[test]
    fn etiquettes_triees_et_ordre_indifferent() {
        let m = Metrics::new();
        m.compteur_incr("c", &[("statut", "200"), ("methode", "GET")]);
        m.compteur_incr("c", &[("methode", "GET"), ("statut", "200")]);
        let texte = m.rendre();
        assert_eq!(m.nombre_series(), 1, "une seule série attendue: {texte}");
        assert!(
            texte.contains("c_total{methode=\"GET\",statut=\"200\"} 2"),
            "{texte}"
        );
    }

    #[test]
    fn etiquette_invalide_ignoree_et_valeur_echappee() {
        let m = Metrics::new();
        m.compteur_incr("c", &[("pas-de-tiret", "x"), ("ok", "a\"b\\c\nd")]);
        let texte = m.rendre();
        assert!(!texte.contains("pas-de-tiret"), "{texte}");
        assert!(texte.contains("ok=\"a\\\"b\\\\c\\nd\""), "{texte}");
    }

    #[test]
    fn jauge_set_et_add() {
        let m = Metrics::new();
        m.jauge_definir("ent_en_cours", &[], 3);
        m.jauge_ajouter("ent_en_cours", &[], 2);
        m.jauge_ajouter("ent_en_cours", &[], -4);
        assert_eq!(m.jauge_valeur("ent_en_cours", &[]), Some(1));
        let texte = m.rendre();
        assert!(texte.contains("# TYPE ent_en_cours gauge"), "{texte}");
        assert!(texte.contains("ent_en_cours 1"), "{texte}");
    }

    #[test]
    fn histogramme_seaux_cumules_somme_et_count() {
        let m = Metrics::new();
        // Deux observations : 0.003 s et 0.4 s.
        m.histogramme_observer("h", &[("route", "/x")], 0.003);
        m.histogramme_observer("h", &[("route", "/x")], 0.4);
        let texte = m.rendre();

        // 0.001 => 0, 0.0025 => 0, 0.005 => 1, 0.01 => 1 … 0.5 => 2, 60 => 2
        assert!(
            texte.contains("h_bucket{route=\"/x\",le=\"0.001\"} 0"),
            "{texte}"
        );
        assert!(
            texte.contains("h_bucket{route=\"/x\",le=\"0.005\"} 1"),
            "{texte}"
        );
        assert!(
            texte.contains("h_bucket{route=\"/x\",le=\"0.1\"} 1"),
            "{texte}"
        );
        assert!(
            texte.contains("h_bucket{route=\"/x\",le=\"0.5\"} 2"),
            "{texte}"
        );
        assert!(
            texte.contains("h_bucket{route=\"/x\",le=\"+Inf\"} 2"),
            "{texte}"
        );
        assert!(texte.contains("h_sum{route=\"/x\"} 0.403000"), "{texte}");
        assert!(texte.contains("h_count{route=\"/x\"} 2"), "{texte}");
        assert!(texte.contains("# TYPE h histogram"), "{texte}");
    }

    #[test]
    fn histogramme_bornes_explicites_inf_coherent() {
        let m = Metrics::new();
        m.histogramme_observer_bornes("taille", &[], 7.0, &[1.0, 10.0]);
        let texte = m.rendre();
        assert!(texte.contains("taille_bucket{le=\"1\"} 0"), "{texte}");
        assert!(texte.contains("taille_bucket{le=\"10\"} 1"), "{texte}");
        assert!(texte.contains("taille_bucket{le=\"+Inf\"} 1"), "{texte}");
    }

    #[test]
    fn helpers_et_help_rendus_une_fois() {
        let m = Metrics::new();
        // L'aide se déclare sur le nom **normalisé** (`_total`).
        m.declarer("ent_x_total", Genre::Compteur, "aide x");
        m.compteur_incr("ent_x", &[("a", "1")]);
        m.compteur_incr("ent_x", &[("a", "2")]);
        let texte = m.rendre();
        assert_eq!(
            texte.matches("# TYPE ent_x_total counter").count(),
            1,
            "{texte}"
        );
        assert_eq!(
            texte.matches("# HELP ent_x_total aide x").count(),
            1,
            "{texte}"
        );
    }

    #[test]
    fn genre_conflictuel_refuse() {
        let m = Metrics::new();
        assert!(m.declarer("z", Genre::Jauge, ""));
        assert!(!m.declarer("z", Genre::Compteur, ""));
        m.jauge_definir("z", &[], 1);
        // Un compteur du même nom ne doit pas écraser la jauge.
        m.compteur_ajouter("z_total", &[], 1);
        let texte = m.rendre();
        assert!(texte.contains("# TYPE z gauge"), "{texte}");
        assert!(texte.contains("# TYPE z_total counter"), "{texte}");
    }

    #[test]
    fn concurrence_sans_perte() {
        let m = std::sync::Arc::new(Metrics::new());
        let mut poignees = Vec::new();
        for _ in 0..8 {
            let m = m.clone();
            poignees.push(std::thread::spawn(move || {
                for _ in 0..1000 {
                    m.compteur_incr("concurrence", &[("t", "1")]);
                }
            }));
        }
        for p in poignees {
            p.join().unwrap();
        }
        assert_eq!(m.compteur_valeur("concurrence", &[("t", "1")]), Some(8000));
    }

    #[test]
    fn mesure_observe() {
        let m = Metrics::new();
        let mesure = Mesure::demarrer();
        let s = mesure.observer(&m, "duree", &[]);
        assert!(s >= 0.0);
        assert!(m.rendre().contains("duree_count 1"));
    }

    #[test]
    fn texte_vide_si_aucune_serie() {
        assert_eq!(Metrics::new().rendre(), "");
    }
}
