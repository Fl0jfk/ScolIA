//! Garde statique d'isolation (socle 1, C6) : toute requête SQL de
//! `ent-db/src/*.rs` qui lit, modifie ou supprime une table portant
//! `etablissement_id` doit contenir `etablissement_id` dans le même littéral.
//!
//! La liste des tables est déduite des migrations (`CREATE TABLE` dont le
//! corps déclare la colonne, ou `ALTER TABLE … ADD COLUMN etablissement_id`).
//! Les exceptions sont **nominatives et justifiées** (`EXCEPTIONS`) : une
//! nouvelle requête sans filtre fait échouer le test tant qu'elle n'est pas
//! corrigée ou justifiée. Sans base : ne dépend que des fichiers sources.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// (fichier, extrait unique du littéral SQL, justification).
/// Un extrait vide exempte toutes les requêtes du fichier sur ses tables.
const EXCEPTIONS: &[(&str, &str, &str)] = &[
    // ── Tables d'identité : la clé est la personne ou le secret, pas le tenant ──
    // Sessions : adressées par `id` (déjà lu depuis la session authentifiée),
    // `token_hash` (secret 256 bits) ou `user_id` (révocation de SES sessions).
    ("auth_repo.rs", "UPDATE ent_session SET portee = 'complete'", "promotion de la session d'enrôlement par id de session authentifiée"),
    ("auth_repo.rs", "UPDATE ent_session SET revoked_at = now() WHERE id = $1 AND user_id = $2", "révocation d'un appareil : id + user_id de la session appelante"),
    ("auth_repo.rs", "DELETE FROM ent_session WHERE expires_at < now()", "purge globale des sessions expirées (maintenance, sans lecture)"),
    ("auth_repo.rs", "UPDATE ent_session SET last_seen_at", "glissement de l'expiration par id de session déjà authentifiée"),
    ("auth_repo.rs", "UPDATE ent_session SET revoked_at = now() WHERE token_hash = $1", "déconnexion par secret (hash du jeton 256 bits)"),
    ("auth_repo.rs", "UPDATE ent_session SET revoked_at = now() WHERE user_id = $1", "révocation de toutes les sessions d'une PERSONNE (une personne = une identité unique, tous établissements : A1)"),
    // Passkeys : rattachées à la personne (A1), pas à un établissement.
    ("passkey_repo.rs", "FROM ent_passkey_credential WHERE user_id = $1", "passkeys de la personne (A1) : lecture par user_id de la session"),
    ("passkey_repo.rs", "ent_passkey_credential WHERE cred_id = $1", "identifiant de credential WebAuthn, unique mondial, fourni par l'authentificateur"),
    ("passkey_repo.rs", "SET passkey = $2, last_used_at = now() WHERE cred_id = $1", "mise à jour du compteur de signature par cred_id (unique mondial) après vérification de l'assertion"),
    ("passkey_repo.rs", "DELETE FROM ent_passkey_credential WHERE id = $1 AND user_id = $2", "suppression de SA passkey : id + user_id de la session"),
    ("passkey_repo.rs", "DELETE FROM ent_passkey_credential WHERE user_id = $1", "reset des passkeys d'une personne (action de direction contrôlée en amont)"),
    ("passkey_repo.rs", "DELETE FROM ent_passkey_challenge WHERE expires_at", "purge globale des challenges expirés"),
    // Récupération de passkey en libre-service (D-018) : parcours de la PERSONNE,
    // identité unique tous établissements (D-013). `etablissement_id` n'y est que
    // l'établissement d'origine de la demande (journal) ; les lignes sont adressées
    // par user_id (personne verrouillée), id lu dans la même transaction, ou
    // empreinte d'un jeton secret de 256 bits à usage unique.
    ("recuperation_repo.rs", "SELECT count(*) FROM ent_recuperation_passkey WHERE user_id = $1", "une seule demande active par personne (D-018), tous établissements"),
    ("recuperation_repo.rs", "WHERE user_id = $1 AND statut = 'demandee'", "remplacement de la demande non confirmée de la personne (D-018)"),
    ("recuperation_repo.rs", "WHERE id = $1 AND statut IN ('demandee', 'en_attente')", "abandon par id de la demande créée dans le même parcours"),
    ("recuperation_repo.rs", "SET statut = 'en_attente', confirmed_at = now()", "confirmation par empreinte du jeton de confirmation (secret 256 bits, usage unique)"),
    ("recuperation_repo.rs", "WHERE annul_hash = $1", "annulation par empreinte du jeton d'annulation (secret 256 bits)"),
    ("recuperation_repo.rs", "WHERE user_id = $1 AND statut IN ('demandee', 'en_attente')", "connexion passkey réussie : annule les demandes de la personne (D-018)"),
    ("recuperation_repo.rs", "FROM ent_recuperation_passkey WHERE final_hash = $1", "finalisation par empreinte du jeton de finalisation (secret 256 bits, FOR UPDATE)"),
    ("recuperation_repo.rs", "SET statut = 'expiree', closed_at = now() WHERE id = $1", "expiration par id lu juste avant dans la même transaction"),
    ("recuperation_repo.rs", "DELETE FROM ent_passkey_credential WHERE user_id = $1", "finalisation : suppression des passkeys de la personne (A1, D-018)"),
    ("recuperation_repo.rs", "DELETE FROM ent_passkey_challenge WHERE user_id = $1", "finalisation : défis en cours de la personne"),
    ("recuperation_repo.rs", "UPDATE ent_session SET revoked_at = now() WHERE user_id = $1", "finalisation : révocation de toutes les sessions de la personne (A1)"),
    ("recuperation_repo.rs", "SET statut = 'terminee', closed_at = now() WHERE id = $1", "clôture par id lu dans la même transaction"),
    ("recuperation_repo.rs", "DELETE FROM ent_recuperation_passkey WHERE created_at < $1", "purge globale des demandes closes ou périmées (maintenance)"),
    // OIDC : maintenance de démarrage et purge.
    ("oidc_repo.rs", "UPDATE ent_oidc_provider SET client_secret = $2 WHERE id = $1", "rechiffrement du secret au démarrage, par id lu juste avant"),
    ("oidc_repo.rs", "FROM ent_oidc_provider WHERE client_secret NOT LIKE", "migration de démarrage du chiffrement des secrets hérités (aucune donnée exposée)"),
    ("oidc_repo.rs", "DELETE FROM ent_oidc_state WHERE expires_at", "purge globale des états expirés"),
    // Sondes de métriques (/metrics) : compteurs agrégés, aucune donnée nominative.
    ("service.rs", "FROM ent_session WHERE revoked_at IS NULL AND expires_at > now()", "métrique globale (compteur)"),
    ("service.rs", "FROM ent_passkey_credential", "métrique globale (compteur)"),
    ("service.rs", "FROM ent_eleve", "métrique globale (compteur)"),
    ("service.rs", "FROM ent_mail_outbox GROUP BY status", "métrique globale (compteurs par statut)"),
];

fn tables_a_etablissement(migrations: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut fichiers: Vec<_> = fs::read_dir(migrations)
        .expect("dossier migrations")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "sql"))
        .collect();
    fichiers.sort();
    for f in fichiers {
        let sql = fs::read_to_string(&f).expect("migration");
        // Retire les commentaires de ligne.
        let sql: String = sql
            .lines()
            .map(|l| l.split("--").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        let bas = sql.to_ascii_lowercase();
        // CREATE TABLE [IF NOT EXISTS] nom ( corps );
        let mut i = 0;
        while let Some(p) = bas[i..].find("create table") {
            let debut = i + p + "create table".len();
            let reste = &bas[debut..];
            let reste_t = reste.trim_start();
            let reste_t = reste_t
                .strip_prefix("if not exists")
                .unwrap_or(reste_t)
                .trim_start();
            let nom: String = reste_t
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let Some(ouv) = bas[debut..].find('(') else {
                break;
            };
            let corps_debut = debut + ouv + 1;
            let (mut prof, mut j) = (1usize, corps_debut);
            let b = bas.as_bytes();
            while j < b.len() && prof > 0 {
                match b[j] {
                    b'(' => prof += 1,
                    b')' => prof -= 1,
                    _ => {}
                }
                j += 1;
            }
            let corps = &bas[corps_debut..j.saturating_sub(1)];
            if corps.contains("etablissement_id") && !nom.is_empty() {
                out.insert(nom);
            }
            i = j.max(debut);
        }
        // ALTER TABLE nom ADD COLUMN [IF NOT EXISTS] etablissement_id
        let mut i = 0;
        while let Some(p) = bas[i..].find("alter table") {
            let debut = i + p + "alter table".len();
            let fin = bas[debut..].find(';').map_or(bas.len(), |f| debut + f);
            let instr = &bas[debut..fin];
            if instr.contains("add column") && instr.contains("etablissement_id") {
                let t = instr.trim_start();
                let t = t.strip_prefix("if exists").unwrap_or(t).trim_start();
                let t = t.strip_prefix("only").unwrap_or(t).trim_start();
                let nom: String = t
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !nom.is_empty() {
                    out.insert(nom);
                }
            }
            i = fin;
        }
    }
    out
}

/// Littéraux de chaîne d'un source Rust (normaux et bruts), hors commentaires
/// de ligne. Renvoie (numéro de ligne, contenu).
fn litteraux(src: &str) -> Vec<(usize, String)> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let (mut i, mut ligne) = (0usize, 1usize);
    while i < b.len() {
        match b[i] {
            b'\n' => {
                ligne += 1;
                i += 1;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'r' if matches!(b.get(i + 1), Some(b'#') | Some(b'"'))
                && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_')) =>
            {
                let mut k = i + 1;
                let mut dieses = 0;
                while b.get(k) == Some(&b'#') {
                    dieses += 1;
                    k += 1;
                }
                if b.get(k) != Some(&b'"') {
                    i += 1;
                    continue;
                }
                k += 1;
                let mut ferme = String::from("\"");
                ferme.push_str(&"#".repeat(dieses));
                let Some(f) = src[k..].find(&ferme) else {
                    break;
                };
                let contenu = &src[k..k + f];
                out.push((ligne, contenu.to_string()));
                ligne += contenu.matches('\n').count();
                i = k + f + ferme.len();
            }
            b'"' => {
                let mut k = i + 1;
                let mut contenu = String::new();
                while k < b.len() && b[k] != b'"' {
                    if b[k] == b'\\' {
                        k += 1;
                    }
                    if k < b.len() {
                        if b[k] == b'\n' {
                            ligne += 1;
                        }
                        contenu.push(b[k] as char);
                    }
                    k += 1;
                }
                out.push((ligne, contenu));
                i = k + 1;
            }
            b'\'' => {
                // littéral de caractère ('"') ou durée de vie : saute prudemment.
                if b.get(i + 2) == Some(&b'\'') {
                    i += 3;
                } else if b.get(i + 1) == Some(&b'\\') && b.get(i + 3) == Some(&b'\'') {
                    i += 4;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// Tables `ent_*` visées par une instruction SELECT / UPDATE / DELETE.
fn tables_visees(sql: &str) -> Vec<String> {
    let mots: Vec<String> = sql
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|m| !m.is_empty())
        .map(|m| m.to_ascii_lowercase())
        .collect();
    let est_dml = mots
        .iter()
        .any(|m| matches!(m.as_str(), "select" | "update" | "delete"));
    if !est_dml {
        return vec![];
    }
    let mut out = Vec::new();
    for w in mots.windows(2) {
        if matches!(w[0].as_str(), "from" | "update" | "join") && w[1].starts_with("ent_") {
            out.push(w[1].clone());
        }
    }
    out
}

#[test]
fn toute_requete_sur_table_a_etablissement_filtre_par_etablissement() {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tables = tables_a_etablissement(&racine.join("../../migrations"));
    assert!(
        tables.len() > 50 && tables.contains("ent_membership"),
        "déduction des tables à etablissement_id incohérente ({} tables)",
        tables.len()
    );
    let mut fichiers: Vec<_> = fs::read_dir(racine.join("src"))
        .expect("src")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    fichiers.sort();
    let mut manques = Vec::new();
    let mut verifies = 0usize;
    let mut exemptes = BTreeSet::new();
    for f in &fichiers {
        let nom = f.file_name().unwrap().to_string_lossy().to_string();
        // Tests unitaires : requêtes de contrôle sur base de test.
        if nom.starts_with("tests_") {
            continue;
        }
        let src = fs::read_to_string(f).expect("source");
        for (ligne, lit) in litteraux(&src) {
            let visees: Vec<String> = tables_visees(&lit)
                .into_iter()
                .filter(|t| tables.contains(t))
                .collect();
            if visees.is_empty() {
                continue;
            }
            verifies += 1;
            if lit.to_ascii_lowercase().contains("etablissement_id") {
                continue;
            }
            let norm = lit.split_whitespace().collect::<Vec<_>>().join(" ");
            let mut exempte = false;
            for (idx, (fichier, extrait, _)) in EXCEPTIONS.iter().enumerate() {
                if *fichier == nom && (extrait.is_empty() || norm.contains(extrait)) {
                    exemptes.insert(idx);
                    exempte = true;
                }
            }
            if exempte {
                continue;
            }
            let une_ligne = lit.split_whitespace().collect::<Vec<_>>().join(" ");
            manques.push(format!(
                "{nom}:{ligne} tables {visees:?} sans etablissement_id : {}",
                une_ligne.chars().take(160).collect::<String>()
            ));
        }
    }
    eprintln!(
        "[isolation statique] {} tables à etablissement_id, {verifies} requêtes contrôlées, {} exception(s) utilisée(s)",
        tables.len(),
        exemptes.len()
    );
    // Une exception inutilisée est périmée : on la retire (pas de dette silencieuse).
    let perimees: Vec<_> = EXCEPTIONS
        .iter()
        .enumerate()
        .filter(|(i, _)| !exemptes.contains(i))
        .map(|(_, (f, e, _))| format!("{f} « {e} »"))
        .collect();
    assert!(
        manques.is_empty() && perimees.is_empty(),
        "{} requête(s) sans filtre d'établissement :\n{}\nexceptions périmées : {perimees:?}",
        manques.len(),
        manques.join("\n")
    );
}

#[test]
fn le_garde_detecte_une_requete_sans_filtre() {
    // Auto-test du détecteur sur des littéraux synthétiques.
    let src = r##"let a = sqlx::query("SELECT x FROM ent_note WHERE id = $1");
        let b = sqlx::query(r#"UPDATE ent_note SET a=1 WHERE id=$1 AND etablissement_id=$2"#);"##;
    let lits = litteraux(src);
    assert_eq!(lits.len(), 2);
    assert_eq!(tables_visees(&lits[0].1), vec!["ent_note".to_string()]);
    assert!(lits[1].1.contains("etablissement_id"));
}
