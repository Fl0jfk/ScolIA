//! Inventaire des routes du routeur, extrait du **source** de `routes.rs`
//! (lecture seule, à la compilation) : la liste suit donc automatiquement les
//! routes ajoutées ; un test échoue si une route nouvelle répond sans session.

#[derive(Debug, Clone)]
pub struct RouteDecl {
    pub chemin: String,
    pub methode: &'static str,
    pub handler: String,
}

const METHODES: [(&str, &str); 5] = [
    ("get(", "GET"),
    ("post(", "POST"),
    ("put(", "PUT"),
    ("delete(", "DELETE"),
    ("patch(", "PATCH"),
];

/// Toutes les sources qui déclarent des routes (routeurs fusionnés par
/// `routes.rs` : cahier, communication_s28, dossier, edt_s2, évaluations, finances, inscriptions, santé, référentiel, référentiel S2-2, rôles,
/// vie scolaire, personnels, services d'enseignement). Une nouvelle source de routes doit être ajoutée ici (le test d'inventaire compare avec les fichiers de `src/`).
pub const SOURCES: [&str; 25] = [
    include_str!("../../src/routes.rs"),
    include_str!("../../src/cahier.rs"),
    include_str!("../../src/dossier.rs"),
    include_str!("../../src/finances.rs"),
    include_str!("../../src/inscriptions.rs"),
    include_str!("../../src/referentiel.rs"),
    include_str!("../../src/referentiel_s2.rs"),
    include_str!("../../src/roles.rs"),
    include_str!("../../src/vie_scolaire.rs"),
    include_str!("../../src/edt_s2.rs"),
    include_str!("../../src/evaluations.rs"),
    include_str!("../../src/communication_s28.rs"),
    include_str!("../../src/sante.rs"),
    include_str!("../../src/eleve_propositions.rs"),
    include_str!("../../src/eleve_fusion.rs"),
    include_str!("../../src/personnels.rs"),
    include_str!("../../src/personnels_missions.rs"),
    include_str!("../../src/personnels_services.rs"),
    include_str!("../../src/structure.rs"),
    include_str!("../../src/structure_offre.rs"),
    include_str!("../../src/structure_calendrier.rs"),
    include_str!("../../src/structure_periodes.rs"),
    include_str!("../../src/structure_cours.rs"),
    include_str!("../../src/voyages.rs"),
    include_str!("../../src/agenda.rs"),
];

/// Fichiers de `src/` contenant `.route(` : doit être couvert par `SOURCES`.
pub const NOMS_SOURCES: [&str; 25] = [
    "routes.rs",
    "cahier.rs",
    "dossier.rs",
    "finances.rs",
    "inscriptions.rs",
    "referentiel.rs",
    "referentiel_s2.rs",
    "roles.rs",
    "vie_scolaire.rs",
    "edt_s2.rs",
    "evaluations.rs",
    "communication_s28.rs",
    "sante.rs",
    "eleve_propositions.rs",
    "eleve_fusion.rs",
    "personnels.rs",
    "personnels_missions.rs",
    "personnels_services.rs",
    "structure.rs",
    "structure_offre.rs",
    "structure_calendrier.rs",
    "structure_periodes.rs",
    "structure_cours.rs",
    "voyages.rs",
    "agenda.rs",
];

pub fn routes() -> Vec<RouteDecl> {
    SOURCES.iter().flat_map(|s| routes_de(s)).collect()
}

fn routes_de(src: &str) -> Vec<RouteDecl> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(p) = src[i..].find(".route(") {
        let debut = i + p + ".route(".len();
        // Fin de l'appel : parenthèse fermante correspondante (hors chaînes).
        let (mut profondeur, mut j, mut en_chaine) = (1usize, debut, false);
        while j < b.len() && profondeur > 0 {
            match b[j] {
                b'\\' if en_chaine => j += 1,
                b'"' => en_chaine = !en_chaine,
                b'(' if !en_chaine => profondeur += 1,
                b')' if !en_chaine => profondeur -= 1,
                _ => {}
            }
            j += 1;
        }
        i = j;
        let interieur = src[debut..j - 1].trim_start();
        let Some(reste) = interieur.strip_prefix('"') else {
            continue;
        };
        let Some(fin_chemin) = reste.find('"') else {
            continue;
        };
        let chemin = reste[..fin_chemin].to_string();
        let expr = &reste[fin_chemin + 1..];
        let eb = expr.as_bytes();
        for (jeton, nom) in METHODES {
            let mut k = 0;
            while let Some(q) = expr[k..].find(jeton) {
                let pos = k + q;
                let avant_ok = pos == 0 || {
                    let c = eb[pos - 1];
                    !(c.is_ascii_alphanumeric() || c == b'_')
                };
                k = pos + jeton.len();
                if !avant_ok {
                    continue;
                }
                let fin = expr[k..].find(')').map(|f| k + f).unwrap_or(expr.len());
                out.push(RouteDecl {
                    chemin: chemin.clone(),
                    methode: nom,
                    handler: expr[k..fin].trim().to_string(),
                });
            }
        }
    }
    out
}

/// Remplace les paramètres `{id}` / `{*rest}` par des valeurs inoffensives.
pub fn instancier(chemin: &str) -> String {
    let neutre = "00000000-0000-4000-8000-000000000001";
    let mut out = String::new();
    let mut reste = chemin;
    while let Some(d) = reste.find('{') {
        out.push_str(&reste[..d]);
        let f = reste[d..].find('}').map(|x| d + x).unwrap_or(reste.len() - 1);
        let nom = &reste[d + 1..f];
        out.push_str(if nom.starts_with('*') { "x" } else { neutre });
        reste = &reste[f + 1..];
    }
    out.push_str(reste);
    out
}
