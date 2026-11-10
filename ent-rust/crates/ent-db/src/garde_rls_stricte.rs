//! Garde statique de la RLS stricte (docs/securite/RLS_STRICTE.md §6) : sans base, sans feature `labo`.

// ---------------------------------------------------------------------------------------------
// Garde statique : aucun accès direct au pool hors contexte, sauf exception déclarée et motivée
// ---------------------------------------------------------------------------------------------

/// Fichiers dont **tous** les accès se font hors contexte d'établissement, car ils ne touchent que des
/// tables sans RLS (identité, sessions, passkeys, OIDC, limitation de débit, racine des établissements,
/// distributeur d'événements) ou la mécanique elle-même. Voir `docs/securite/RLS_STRICTE.md` §4.
const FICHIERS_SANS_CONTEXTE: &[(&str, &str)] = &[
    ("auth_repo", "identité, sessions, mots de passe : lus avant le choix d'établissement (ent_user, ent_session, ent_membership)"),
    ("oidc_repo", "SSO : fournisseur et état OIDC résolus avant toute authentification"),
    ("passkey_repo", "passkeys et défis : par compte, avant le choix d'établissement"),
    ("recuperation_repo", "récupération de compte par passkey : par compte"),
    ("email_change_repo", "changement d'adresse : par compte"),
    ("rate_limit", "limitation de débit : table sans établissement"),
    ("etablissement_repo", "racine des établissements et domaines (D-012) : lue avant de connaître l'établissement"),
    ("outbox", "distributeur d'événements : ent_outbox et ent_metier_event sans RLS ; contexte posé par événement dans `traiter_par`"),
    ("event_store", "journal d'événements ent_metier_event : sans RLS (lu par le distributeur transverse)"),
    ("tenant_tx", "la mécanique de contexte elle-même"),
    ("registre_donnees", "catalogue statique + neutralisation à la restauration (opération propriétaire hors application)"),
    ("migrate", "migrations : rôle propriétaire"),
    ("pra", "sauvegarde / restauration : rôle propriétaire, hors chemin applicatif"),
    ("seed", "jeu de labo : passe par les services"),
    ("scenarios", "scénarios de labo"),
    ("lib", "déclarations"),
    ("pg_err", "traduction d'erreurs"),
    ("url_guard", "garde d'URL"),
    ("garde_rls_stricte", "ce fichier (motifs recherchés)"),
    ("service", "façade : construction des dépôts, aucun SQL direct hors contexte"),
];

/// (fichier, extrait de la ligne, justification) : accès directs au pool **autorisés**. Une exception inutilisée
/// fait échouer le test (la liste ne doit jamais mentir).
const EXCEPTIONS_ACCES_DIRECTS: &[(&str, &str, &str)] = &[
    ("acces", ".fetch_optional(self.pool())", "ent_membership : table sans RLS (lue à l'authentification)"),
    ("inscriptions", ".fetch_all(self.pool())", "ent_inscription_invitation : table sans RLS (jeton résolu avant l'établissement)"),
    ("inscriptions", ".fetch_optional(self.pool())", "ent_preinscription_verification / ent_inscription_invitation : tables sans RLS (jetons)"),
    ("inscriptions", "fermeture_en_cours(self.pool(), etab)", "l'assistant ouvre son propre contexte d'établissement"),
    ("inscriptions_modele", "fermeture_en_cours(self.pool(), etab)", "l'assistant ouvre son propre contexte d'établissement"),
    ("rencontres", "enseignants_de(self.pool()", "l'assistant ouvre son propre contexte d'établissement"),
    ("rencontres", "enseignants_de_eleve(self.pool()", "l'assistant ouvre son propre contexte d'établissement"),
    ("inscriptions_reactions", "pool.begin()", "worker d'envoi des e-mails d'inscription : ent_inscription_envoi et ent_inscription_invitation, tables sans RLS (relève les envois de tous les établissements)"),
    ("champs_chiffres", "Conn::Pool(pool.acquire()", "tables sans établissement (branche `None`, aucune aujourd'hui) ; les tables par établissement passent par TenantTx"),
    ("delegation", "charger_droits(self.pool(), etab", "l'assistant ouvre son propre contexte d'établissement"),
    ("sante_acc", ".fetch_optional(self.pool())", "ent_user : table d'identité sans RLS"),
    ("securite_repo", ".execute(self.pool())", "ent_session / ent_membership : tables sans RLS"),
    ("structure_calendrier", "self.pool().begin()", "ref_calendrier_officiel : référentiel global sans établissement (fonction de plateforme)"),
];

/// Lignes de code où le pool est employé comme exécuteur sans passer par `TenantTx`, `lire_par_etablissement`
/// ou `etablissements_systeme`. Les commentaires, signatures, constructeurs et clones sont ignorés.
fn acces_directs_au_pool(src: &str) -> Vec<(usize, String)> {
    let motifs = ["&self.pool", "self.pool()", "self.pool.", "&pool", "(pool)", "(pool,", "(&*pool", "pool.begin(", "pool.acquire("];
    let ok = [
        "TenantTx::begin(", "tenant(", "lire_par_etablissement(", "etablissements_systeme(", "::new(",
        "pool.clone()", "pool().clone()", "pool: ", "{ pool }", "fn ",
    ];
    let appelants = ["lire_par_etablissement(", "etablissements_systeme(", "TenantTx::begin(", "tenant("];
    let lignes: Vec<&str> = src.split("#[cfg(test)]").next().unwrap_or("").lines().collect();
    let mut out = Vec::new();
    for (i, l) in lignes.iter().enumerate() {
        let code = l.split("//").next().unwrap_or("");
        let t = code.trim();
        let prec = if i > 0 { lignes[i - 1].trim_end() } else { "" };
        let drapeau = if t == "pool," || t == "pool);" || t == "pool)" || t == "self.pool()," {
            // argument nu d'un appel : valide si l'appel ouvre un contexte (ligne précédente).
            prec.ends_with('(') && !appelants.iter().any(|m| prec.ends_with(&format!("{m}")) )
        } else {
            motifs.iter().any(|m| code.contains(m)) && !ok.iter().any(|m| code.contains(m))
        };
        if drapeau {
            out.push((i + 1, t.to_string()));
        }
    }
    out
}

/// **Aucun accès direct au pool** dans le code d'`ent-db` hors exceptions motivées : tout accès à une table
/// « par établissement » passe par `TenantTx` (ou le contexte système nommé). Garde contre la
/// régression : un nouveau dépôt qui oublie le contexte échoue ici, avant la CI strict de domaine.
#[test]
fn aucun_acces_direct_au_pool_hors_exception() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut utilisees = std::collections::BTreeSet::new();
    let mut violations = Vec::new();
    for entree in std::fs::read_dir(&dir).unwrap() {
        let chemin = entree.unwrap().path();
        let Some(nom) = chemin.file_stem().and_then(|n| n.to_str()).map(str::to_string) else { continue };
        if chemin.extension().and_then(|e| e.to_str()) != Some("rs")
            || nom.starts_with("tests_")
            || FICHIERS_SANS_CONTEXTE.iter().any(|(f, _)| *f == nom)
        {
            continue;
        }
        let src = std::fs::read_to_string(&chemin).unwrap();
        for (ligne, texte) in acces_directs_au_pool(&src) {
            match EXCEPTIONS_ACCES_DIRECTS
                .iter()
                .position(|(f, extrait, _)| *f == nom && texte.contains(extrait))
            {
                Some(k) => {
                    utilisees.insert(k);
                }
                None => violations.push(format!("{nom}.rs:{ligne} : {texte}")),
            }
        }
    }
    assert!(
        violations.is_empty(),
        "accès direct au pool hors contexte d'établissement (utiliser TenantTx, ou déclarer l'exception) :\n{}",
        violations.join("\n")
    );
    let inutiles: Vec<_> = EXCEPTIONS_ACCES_DIRECTS
        .iter()
        .enumerate()
        .filter(|(k, _)| !utilisees.contains(k))
        .map(|(_, (f, e, _))| format!("{f} : {e}"))
        .collect();
    assert!(inutiles.is_empty(), "exceptions devenues inutiles (à retirer) :\n{}", inutiles.join("\n"));
}
