//! Garde statique des espaces de travail (socle 32, lots ET0 et ET1) : sans base, sans feature `labo` ; tourne en CI avec et
//! sans labo. Elle protège les propriétés qui font la sécurité du domaine et qu'un test fonctionnel ne verrait pas si un futur lot
//! contournait le chemin unique :
//!
//! 1. tout SQL qui touche une table `ent_espace*` vit dans les fichiers du domaine (ou dans la liste déclarée) ;
//! 2. toute commande publique `espaces_*` du service passe par `espaces_debut` (droit D-017 vérifié avant la transaction + personne du compte) ; les tâches de
//!    fond sont déclarées et motivées ; une exception inutilisée fait échouer le test (la liste ne ment pas) ;
//! 3. **aucun octet de fichier ne transite par l'API** : l'écriture dans le magasin (`objet_put`, `.put(`) n'existe que dans la
//!    copie d'une version (retour arrière) — l'envoi est direct, à taille signée (ND32-14, R-ET21) ;
//! 4. un lien (`presigner()`) n'est émis qu'après `acces_element` (contrôle des droits, R-ET9) ou, pour un envoi, après la
//!    réservation de quota ;
//! 5. la logique n'appelle jamais l'horloge (`Utc::now`, `now()` SQL) : l'instant est un paramètre (règle 33) ;
//! 6. les migrations 2640 à 2643 n'ont ni JSON, ni tableau, ni BYTEA (D-036), et chaque table porte la RLS d'établissement.

use std::path::{Path, PathBuf};

/// Fichiers autorisés à contenir du SQL sur `ent_espace*`.
const FICHIERS_SQL_ESPACES: &[(&str, &str)] = &[
    ("espaces_service.rs", "espaces, membres, invitations, quota, journal"),
    ("espaces_contenu.rs", "dossiers, éléments, dépôt direct, versions, corbeille"),
    ("espaces_partages.rs", "partages nominatifs"),
    ("espaces_quota.rs", "compteurs de quota et réservations (ET0)"),
    ("espaces_taches.rs", "analyse, expiration, purge, élagage, réconciliation"),
    ("registre_donnees.rs", "catalogue statique des tables"),
    ("migrate.rs", "migrations (rôle propriétaire)"),
    ("pra.rs", "sauvegarde / restauration (rôle propriétaire)"),
    ("garde_espaces.rs", "ce fichier"),
];

/// Commandes publiques qui n'appellent pas `espaces_moi`, avec la raison.
const SANS_ESPACES_MOI: &[(&str, &str)] = &[
    ("espaces_analyser_versions", "tâche de fond (système) : aucun acteur humain ; analyse les versions en attente de l'établissement"),
    ("espaces_liberer_reservations_expirees", "tâche de fond (système) : libère les réservations échues de l'établissement"),
    ("espaces_purger_corbeille", "tâche de fond (système) : efface les éléments échus (30 jours)"),
    ("espaces_elaguer_versions", "tâche de fond (système) : élagage S-3210"),
    ("espaces_reconcilier_occupation", "tâche de fond (système) : recalcul des compteurs"),
    ("espaces_tache_tous_etablissements", "ordonnanceur : parcourt les établissements et appelle les tâches ci-dessus"),
    ("espaces_recevoir_envoi_local", "récepteur de l'envoi local : la session et le jeton signé sont vérifiés par la route ; la réservation du demandeur l'est ici"),
    ("espaces_lien_telechargement", "délègue à `lien_fichier`, qui appelle `espaces_debut` puis `acces_element` avant d'émettre le lien"),
    ("espaces_lien_apercu", "délègue à `lien_fichier`, qui appelle `espaces_debut` puis `acces_element` avant d'émettre le lien"),
    ("espaces_quota", "délègue à `espaces_lister` (qui appelle `espaces_debut`) avant de lire quoi que ce soit"),
];

/// Écritures dans le magasin d'objets autorisées dans le domaine : la copie d'une version (retour arrière).
const ECRITURES_MAGASIN: &[(&str, &str)] = &[
    ("espaces_version_restaurer", "copie d'un objet déjà analysé sous une clé neuve : le retour arrière crée une version"),
    (
        "espaces_recevoir_envoi_local",
        "récepteur de l'envoi du mode local (labo) : vérifie la réservation, le demandeur et la taille signée avant d'écrire ; absent en production (S3)",
    ),
];

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn lire(nom: &str) -> String {
    std::fs::read_to_string(src_dir().join(nom)).unwrap_or_else(|e| panic!("lecture {nom} : {e}"))
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

/// Code de production d'un fichier : sans le module de tests (`#[cfg(test)]`) ni les lignes de commentaire.
fn production(src: &str) -> String {
    let coupe = src.find("#[cfg(test)]").unwrap_or(src.len());
    src[..coupe]
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Découpe en fonctions `(nom, corps)` sur les en-têtes `fn` en tête de ligne (indentation 0 ou 4).
fn fonctions(src: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut courant: Option<(String, String)> = None;
    for ligne in src.lines() {
        let sans = ligne.trim_start();
        let indent = ligne.len() - sans.len();
        let debut = ["pub async fn ", "pub(crate) async fn ", "async fn ", "pub fn ", "pub(crate) fn ", "fn "]
            .iter()
            .find_map(|p| sans.strip_prefix(p));
        if let (Some(reste), true) = (debut, indent <= 4) {
            if let Some(f) = courant.take() {
                out.push(f);
            }
            let nom: String = reste.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            courant = Some((nom, String::new()));
        }
        if let Some((_, corps)) = courant.as_mut() {
            corps.push_str(ligne);
            corps.push('\n');
        }
    }
    if let Some(f) = courant {
        out.push(f);
    }
    out
}

const FICHIERS_DOMAINE: &[&str] =
    &["espaces_service.rs", "espaces_contenu.rs", "espaces_partages.rs", "espaces_quota.rs", "espaces_taches.rs"];

/// 1. Le SQL des tables `ent_espace*` ne vit que dans les fichiers du domaine.
#[test]
fn sql_des_espaces_uniquement_dans_les_fichiers_du_domaine() {
    let mut fichiers = Vec::new();
    fichiers_rs(&src_dir(), &mut fichiers);
    let mut fautes = Vec::new();
    for f in fichiers {
        let nom = f.file_name().unwrap().to_string_lossy().to_string();
        if FICHIERS_SQL_ESPACES.iter().any(|(n, _)| *n == nom) || nom.starts_with("tests_") {
            continue;
        }
        let src = std::fs::read_to_string(&f).unwrap();
        if production(&src).contains("ent_espace") {
            fautes.push(nom);
        }
    }
    assert!(fautes.is_empty(), "SQL sur `ent_espace*` hors des fichiers du domaine : {fautes:?}");
}

/// 2. Toute commande publique du service passe par `espaces_moi` ; les exceptions sont motivées et utilisées.
#[test]
fn chaque_commande_publique_verifie_le_droit_par_espaces_moi() {
    let mut utilisees = std::collections::BTreeSet::new();
    let mut fautes = Vec::new();
    for f in FICHIERS_DOMAINE {
        for (nom, corps) in fonctions(&production(&lire(f))) {
            let en_tete = corps.lines().next().unwrap_or("");
            if !nom.starts_with("espaces_") || !en_tete.contains("pub async fn ") || corps.contains("espaces_debut(") {
                continue;
            }
            match SANS_ESPACES_MOI.iter().find(|(n, _)| *n == nom) {
                Some(_) => {
                    utilisees.insert(nom);
                }
                None => fautes.push(format!("{f}::{nom}")),
            }
        }
    }
    assert!(fautes.is_empty(), "commande sans `espaces_debut` (droit D-017) et sans exception motivée : {fautes:?}");
    for (n, _) in SANS_ESPACES_MOI {
        assert!(utilisees.contains(*n), "exception inutilisée (la fonction appelle désormais `espaces_debut`, ou n'existe plus) : {n}");
    }
}

/// 3. Aucun octet de fichier n'est écrit par l'API hors la copie d'une version.
#[test]
fn aucun_octet_de_fichier_ne_transite_par_l_api() {
    let mut utilisees = std::collections::BTreeSet::new();
    let mut fautes = Vec::new();
    for f in FICHIERS_DOMAINE {
        for (nom, corps) in fonctions(&production(&lire(f))) {
            let ecrit = corps.contains("objet_put(") || corps.contains(".put(") || corps.contains("stocker_objet(");
            if !ecrit {
                continue;
            }
            match ECRITURES_MAGASIN.iter().find(|(n, _)| *n == nom) {
                Some(_) => {
                    utilisees.insert(nom);
                }
                None => fautes.push(format!("{f}::{nom}")),
            }
        }
    }
    assert!(fautes.is_empty(), "écriture d'octets dans le magasin hors exception : {fautes:?}");
    for (n, _) in ECRITURES_MAGASIN {
        assert!(utilisees.contains(*n), "exception inutilisée : {n}");
    }
}

/// 4. Un lien d'accès n'est émis qu'après contrôle des droits ; un envoi qu'après réservation du quota.
#[test]
fn aucun_lien_sans_controle_des_droits() {
    let mut fautes = Vec::new();
    let mut telechargement = false;
    let mut envoi = false;
    for f in FICHIERS_DOMAINE {
        for (nom, corps) in fonctions(&production(&lire(f))) {
            if corps.contains(".issue(") {
                telechargement = true;
                if !corps.contains("acces_element(") {
                    fautes.push(format!("{f}::{nom} émet un lien sans `acces_element`"));
                }
            }
            if corps.contains(".issue_upload(") {
                envoi = true;
                if !corps.contains("quota_reserver(") {
                    fautes.push(format!("{f}::{nom} émet une URL d'envoi sans réserver le quota"));
                }
            }
        }
    }
    assert!(fautes.is_empty(), "{fautes:?}");
    assert!(telechargement && envoi, "la garde ne voit plus d'émission de lien : à mettre à jour");
}

/// 5. La logique ne lit jamais l'horloge : l'instant est un paramètre.
#[test]
fn aucune_horloge_dans_la_logique_des_espaces() {
    let mut fautes = Vec::new();
    for f in FICHIERS_DOMAINE.iter().chain(&["espaces.rs"]) {
        let prod = production(&lire(f));
        for (i, l) in prod.lines().enumerate() {
            let bas = l.to_ascii_lowercase();
            if l.contains("Utc::now") || l.contains("SystemTime") || bas.contains("now()") || bas.contains("current_date") || bas.contains("current_timestamp") {
                fautes.push(format!("{f}:{} {}", i + 1, l.trim()));
            }
        }
    }
    assert!(fautes.is_empty(), "horloge dans la logique des espaces (règle 33) : {fautes:?}");
}

/// 6. Migrations 2640 à 2643 : ni JSON, ni tableau, ni BYTEA ; RLS posée sur chaque table créée.
#[test]
fn migrations_des_espaces_sans_json_ni_tableau_ni_bytea_et_avec_rls() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    for n in ["2640_esp_et0_quotas", "2641_esp_espaces_membres", "2642_esp_dossiers_elements_versions", "2643_esp_partages_journal"] {
        let sql = std::fs::read_to_string(dir.join(format!("{n}.sql"))).unwrap_or_else(|e| panic!("{n} : {e}"));
        let code: String = sql.lines().map(|l| l.split("--").next().unwrap_or("")).collect::<Vec<_>>().join("\n").to_ascii_lowercase();
        for interdit in ["jsonb", " json", "bytea", "[]", "array["] {
            // `ARRAY[` n'est admis que dans l'appel `ent_rls_poser(ARRAY[…])`.
            let occurrences = code.matches(interdit).count();
            let tolere = if interdit == "array[" { code.matches("ent_rls_poser(array[").count() } else { 0 };
            assert_eq!(occurrences, tolere, "{n} : `{interdit}` interdit (D-036)");
        }
        let tables: Vec<String> = code
            .split("create table if not exists ")
            .skip(1)
            .map(|s| s.split(|c: char| c.is_whitespace() || c == '(').next().unwrap().to_string())
            .collect();
        assert!(!tables.is_empty(), "{n} : aucune table");
        for t in tables {
            assert!(code.contains(&format!("'{t}'")), "{n} : la table {t} n'est pas dans l'appel `ent_rls_poser` (RLS d'établissement)");
        }
    }
}
