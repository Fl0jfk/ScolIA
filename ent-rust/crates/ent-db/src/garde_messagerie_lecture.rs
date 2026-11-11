//! Garde statique de la messagerie (socle 16, R-M1 et R-M2) : **aucune lecture d'un fil, d'un
//! message, d'une pièce ou d'un accusé hors du contrôle d'accès unique**. Sans base, sans feature
//! `labo` : tourne en CI avec et sans labo.
//!
//! Trois contrôles :
//! 1. tout SQL qui touche une table `ent_msg*` vit dans `msg_repo.rs` (ou dans les fichiers
//!    déclarés ci-dessous) : un nouveau chemin d'accès ailleurs fait échouer le test ;
//! 2. chaque fonction de lecture de `msg_repo.rs` est **classée** (filtre intégré, lecture par
//!    identifiant, pont des rôles réservés) : une fonction non classée, ou une fonction
//!    classée « filtre intégré » qui n'appelle plus la vue `ent_msg_fil_lisible`, fait échouer le test ;
//! 3. toute fonction du service qui appelle une lecture **par identifiant** passe d'abord par
//!    `actor_can_access_fil` / `peut_lire_fil` (ou par le contrôle des pièces), sauf exception
//!    déclarée et motivée. Une exception inutilisée fait échouer le test (la liste ne ment pas).

use std::path::{Path, PathBuf};

/// Fichiers autorisés à contenir du SQL sur `ent_msg*` (le dépôt de la brique, la matrice M2 qui
/// ne lit que des participants, le registre, les migrations, la sauvegarde, la reprise des pièces).
const FICHIERS_SQL_MESSAGERIE: &[(&str, &str)] = &[
    (
        "msg_repo.rs",
        "dépôt de la brique : seul chemin SQL vers les fils, messages, pièces et accusés",
    ),
    (
        "msg_m2.rs",
        "matrice M2 : lit les participants d'un fil pour la séparation, jamais le contenu",
    ),
    ("registre_donnees.rs", "catalogue statique des tables"),
    ("migrate.rs", "migrations (rôle propriétaire)"),
    ("pra.rs", "sauvegarde / restauration (rôle propriétaire)"),
    (
        "reprise_bytea.rs",
        "reprise des pièces historiques vers le stockage objet (opération propriétaire)",
    ),
    ("garde_messagerie_lecture.rs", "ce fichier"),
];

/// Fonctions de lecture de `msg_repo.rs`, par catégorie.
/// A : le filtre de lecture est **dans** la requête (vue `ent_msg_fil_lisible` ou prédicat).
const FILTRE_INTEGRE: &[&str] = &[
    "fils_lisibles_famille",
    "fils_lisibles_personnel",
    "page_fils_personnel",
    "destinataires_famille_fil",
];
/// B : lecture par identifiant ; l'appelant a déjà appelé `peut_lire_fil` (contrôle 3).
const PAR_IDENTIFIANT: &[&str] = &[
    "get_fil",
    "list_messages",
    "list_accuses_fil",
    "list_pieces_meta_fil",
    "get_piece",
    "get_message",
    "message_remis_pour",
    "count_pieces_message",
    "count_non_lus_par_fils",
    "list_fils_eleve",
    "list_fils_eleves",
    "annuler_message_differe",
    "peut_ecrire_fil",
    "compter_participants",
    "personnels_a_prevenir",
];
/// C : pont des rôles réservés (D26), borné par `lit_fils_reserves` dans le service.
const PONT_ROLES_RESERVES: &[&str] = &["fils_non_traites"];

/// Fonctions du service qui appellent une lecture par identifiant **sans** appeler le contrôle de
/// lecture, avec la raison.
const EXCEPTIONS_SERVICE: &[(&str, &str)] = &[(
    "count_msg_non_lus_par_fils",
    "compteur sur des identifiants issus d'une liste déjà filtrée (`enrich_msg_fils`)",
)];

const CONTROLES_LECTURE: &[&str] = &[
    "actor_can_access_fil",
    "peut_lire_fil",
    "piece_message_autorisee",
    "filtrer_fils_famille",
];

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn fichiers_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("lecture {}: {e}", dir.display())) {
        let p = e.unwrap().path();
        if p.is_dir() {
            fichiers_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn nom(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().to_string()
}

/// Découpe un fichier en fonctions (`(nom, corps)`), sur les en-têtes `fn` en tête de ligne.
fn fonctions(src: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut courant: Option<(String, String)> = None;
    for ligne in src.lines() {
        let t = ligne.trim_start();
        let debut = [
            "pub async fn ",
            "pub(crate) async fn ",
            "async fn ",
            "pub fn ",
            "pub(crate) fn ",
            "fn ",
        ]
        .iter()
        .find_map(|p| t.strip_prefix(p));
        if let Some(reste) = debut {
            // Une fonction « de premier niveau » d'un bloc `impl` : indentation de 0 ou 4 espaces.
            let indent = ligne.len() - t.len();
            if indent <= 4 {
                if let Some(c) = courant.take() {
                    out.push(c);
                }
                let n: String = reste
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                courant = Some((n, String::new()));
            }
        }
        if let Some((_, corps)) = courant.as_mut() {
            corps.push_str(ligne);
            corps.push('\n');
        }
    }
    if let Some(c) = courant {
        out.push(c);
    }
    out
}

/// Tables qui portent du **contenu** de conversation (fils, messages, pièces, accusés, participants).
/// Les réglages, modèles et décisions sur les liens n'en sont pas : ils ne contiennent aucun échange.
const TABLES_CONTENU: &[&str] = &[
    "ent_msg",
    "ent_msg_fil",
    "ent_msg_piece",
    "ent_msg_accuse",
    "ent_msg_participant",
    "ent_msg_fil_eleve",
    "ent_msg_version",
];

fn mots(texte: &str) -> Vec<&str> {
    texte
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|m| !m.is_empty())
        .collect()
}

/// Vrai si `texte` contient `FROM|JOIN|UPDATE|INTO` suivi d'une table de contenu de messagerie.
fn touche_table_msg(texte: &str) -> bool {
    mots(texte).windows(2).any(|w| {
        matches!(
            w[0].to_ascii_uppercase().as_str(),
            "FROM" | "JOIN" | "UPDATE" | "INTO"
        ) && TABLES_CONTENU.contains(&w[1])
    })
}

/// Vrai si `texte` **lit** (`FROM` ou `JOIN`) une table de contenu ou la vue `ent_msg_fil_lisible`.
fn lit_une_table_msg(texte: &str) -> bool {
    mots(texte).windows(2).any(|w| {
        matches!(w[0].to_ascii_uppercase().as_str(), "FROM" | "JOIN")
            && (TABLES_CONTENU.contains(&w[1]) || w[1] == "ent_msg_fil_lisible")
    })
}

/// 1. Aucun autre chemin SQL vers les tables de messagerie (dans tout le dépôt Rust du backend).
#[test]
fn le_sql_de_messagerie_n_existe_que_dans_les_fichiers_declares() {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut tous = Vec::new();
    for crate_dir in std::fs::read_dir(&racine).unwrap().flatten() {
        let src = crate_dir.path().join("src");
        if src.is_dir() {
            fichiers_rs(&src, &mut tous);
        }
    }
    let mut fautifs = Vec::new();
    let mut vus = std::collections::BTreeSet::new();
    for f in tous {
        let n = nom(&f);
        if n.starts_with("tests_") || n == "tests.rs" {
            continue;
        }
        let src = std::fs::read_to_string(&f).unwrap();
        // Les modules `#[cfg(test)]` de fin de fichier ne comptent pas.
        let src = src.split("\n#[cfg(test)]").next().unwrap_or("");
        if touche_table_msg(src) {
            if FICHIERS_SQL_MESSAGERIE.iter().any(|(a, _)| *a == n) {
                vus.insert(n);
            } else {
                fautifs.push(f.display().to_string());
            }
        }
    }
    assert!(
        fautifs.is_empty(),
        "SQL sur les tables ent_msg* hors des fichiers déclarés (R-M2 : un seul chemin de lecture) : {fautifs:?}"
    );
    assert!(
        vus.contains("msg_repo.rs"),
        "le dépôt de la brique doit être scanné"
    );
}

/// 2. Chaque lecture du dépôt est classée ; un « filtre intégré » appelle bien la vue.
#[test]
fn chaque_lecture_du_depot_de_messagerie_est_classee() {
    let src = std::fs::read_to_string(src_dir().join("msg_repo.rs")).unwrap();
    let src = src.split("\n#[cfg(test)]").next().unwrap();
    let mut non_classees = Vec::new();
    let mut lues = Vec::new();
    for (n, corps) in fonctions(src) {
        if !corps.contains("pub async fn") || !lit_une_table_msg(&corps) {
            continue;
        }
        lues.push(n.clone());
        let classee = FILTRE_INTEGRE
            .iter()
            .chain(PAR_IDENTIFIANT)
            .chain(PONT_ROLES_RESERVES)
            .any(|c| *c == n);
        if !classee {
            non_classees.push(n.clone());
        }
        if FILTRE_INTEGRE.contains(&n.as_str())
            && !(corps.contains("ent_msg_fil_lisible")
                || corps.contains("PREDICAT_LISIBLE_PERSONNEL")
                || corps.contains("ent_msg_participant"))
        {
            panic!("`{n}` est classée « filtre intégré » mais n'appelle plus la vue ent_msg_fil_lisible");
        }
    }
    assert!(
        non_classees.is_empty(),
        "fonctions de lecture de msg_repo.rs non classées (ajoutez-les à la garde en justifiant) : {non_classees:?}"
    );
    // Une entrée de la garde qui ne correspond à aucune lecture est périmée.
    for c in FILTRE_INTEGRE
        .iter()
        .chain(PAR_IDENTIFIANT)
        .chain(PONT_ROLES_RESERVES)
    {
        assert!(
            lues.iter().any(|l| l == c),
            "entrée périmée dans la garde : `{c}`"
        );
    }
}

/// 3. Une lecture par identifiant est précédée du contrôle de lecture.
#[test]
fn toute_lecture_par_identifiant_passe_par_le_controle_de_lecture() {
    let mut fichiers = Vec::new();
    fichiers_rs(&src_dir(), &mut fichiers);
    let mut fautives = Vec::new();
    let mut exceptions_utilisees = std::collections::BTreeSet::new();
    for f in fichiers {
        let n = nom(&f);
        if n.starts_with("tests_") || n == "msg_repo.rs" || n == "garde_messagerie_lecture.rs" {
            continue;
        }
        let src = std::fs::read_to_string(&f).unwrap();
        let src = src.split("\n#[cfg(test)]").next().unwrap_or("");
        for (nom_fn, corps) in fonctions(src) {
            let compact: String = corps.chars().filter(|c| !c.is_whitespace()).collect();
            let lit_par_id = PAR_IDENTIFIANT
                .iter()
                // « peut_ecrire_fil » et « personnels_a_prevenir » sont eux-mêmes des contrôles ou des
                // avis aux seuls participants : ils ne lisent pas de contenu.
                .filter(|m| {
                    !matches!(
                        **m,
                        "peut_ecrire_fil" | "compter_participants" | "personnels_a_prevenir"
                    )
                })
                .any(|m| compact.contains(&format!("messages.{m}(")));
            if !lit_par_id {
                continue;
            }
            if CONTROLES_LECTURE.iter().any(|c| corps.contains(c)) {
                continue;
            }
            if EXCEPTIONS_SERVICE.iter().any(|(e, _)| *e == nom_fn) {
                exceptions_utilisees.insert(nom_fn);
                continue;
            }
            fautives.push(format!("{n}::{nom_fn}"));
        }
    }
    assert!(
        fautives.is_empty(),
        "fonctions qui lisent un fil par identifiant sans contrôle de lecture (R-M2) : {fautives:?}"
    );
    for (e, _) in EXCEPTIONS_SERVICE {
        assert!(
            exceptions_utilisees.contains(*e),
            "exception périmée dans la garde : `{e}`"
        );
    }
}

#[cfg(test)]
mod outils {
    use super::*;

    #[test]
    fn le_detecteur_de_tables_distingue_lecture_et_fonctions() {
        assert!(touche_table_msg("SELECT 1 FROM ent_msg m WHERE x"));
        assert!(touche_table_msg("INSERT INTO ent_msg_participant (a)"));
        assert!(touche_table_msg("UPDATE ent_msg_fil SET a = 1"));
        assert!(!touche_table_msg("SELECT ent_msg_ajouter_participant($1)"));
        assert!(!touche_table_msg("SELECT ent_msg_remise_calme($1, $2)"));
        assert!(!touche_table_msg("SELECT 1 FROM ent_message_autre"));
        assert!(lit_une_table_msg("FROM ent_msg_fil_lisible($1, $3) l"));
    }

    #[test]
    fn le_decoupeur_de_fonctions_trouve_les_noms() {
        let f = fonctions(
            "impl X {\n    pub async fn a(&self) {\n        1\n    }\n    fn b() {}\n}\n",
        );
        assert_eq!(
            f.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }
}
