//! Anonymisation des **JSON** : colonnes `jsonb`, valeurs des tables clé-valeur (`*_attr`,
//! `ent_collection_attr`) et feuilles `j:{…}`.
//!
//! Le JSON n'a pas de schéma déclaré : chaque **clé** est classée par son nom ([`classer_cle`]).
//! Règle **fermée** : une clé inconnue ne garde jamais un texte — sa valeur chaîne est remplacée
//! par un texte fictif de même longueur, et le nom de la clé (structure, jamais une valeur) est
//! listé au rapport pour compléter le dictionnaire. Les nombres, booléens, UUID et dates ISO
//! sont conservés (structure, montants, quantités).

use super::cle::Cle;
use super::cle_s3::reecrire_cle;
use super::controle::Sensibles;
use super::pseudo::{est_uuid, pseudonymiser, texte};
use crate::schema::Genre;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClasseCle {
    Genre(Genre),
    CleS3,
    Effacer,
    Garder,
    Inconnue,
}

/// Noms de clés (normalisés) conservés : structure, statuts, identifiants, dates, quantités.
const GARDER_EXACT: &[&str] = &[
    "id",
    "ids",
    "status",
    "statut",
    "type",
    "kind",
    "code",
    "count",
    "nb",
    "nombre",
    "index",
    "order",
    "ordre",
    "sort",
    "version",
    "enabled",
    "active",
    "actif",
    "visible",
    "color",
    "couleur",
    "theme",
    "slug",
    "mode",
    "state",
    "niveau",
    "niveaux",
    "classe",
    "classes",
    "matiere",
    "annee",
    "periode",
    "regime",
    "site",
    "sens",
    "scope",
    "category",
    "categorie",
    "mef",
    "lv1",
    "lv2",
    "option",
    "options",
    "secteur",
    "canal",
    "source",
    "locale",
    "lang",
    "timezone",
    "currency",
    "unit",
    "unite",
    "role",
    "roles",
    "cycle",
    "groupe",
    "group",
    "len",
    "length",
    "size",
    "taille",
    "date",
    "dates",
    "debut",
    "fin",
    "start",
    "end",
    "jour",
    "heure",
    "semaine",
    "at",
    "kinds",
    "types",
    "statuses",
    "cycles",
    "diplome",
    "diploma",
    "response",
    "reponse_",
    "rank",
    "rang",
    "position",
    "priority",
    "priorite",
    "montant",
    "total",
    "prix",
    "quantite",
    "coefficient",
    "coef",
    "note_",
    "valeur_",
    "oui",
    "non",
    "checked",
    "selected",
    "default",
    "required",
    "obligatoire",
    "optionnel",
    "published",
    "publie",
    "locked",
    "gele",
];
const GARDER_SUFFIXES: &[&str] = &[
    "id", "ids", "at", "date", "dates", "count", "status", "statut",
];
const NON_NOMS: &[&str] = &[
    "filename",
    "classname",
    "rolename",
    "sitename",
    "typename",
    "bucketname",
    "pathname",
    "hostname",
    "domainname",
    "keyname",
    "fieldname",
    "tablename",
    "columnname",
    "labelname",
    "schemaname",
    "eventname",
    "statusname",
];

/// Classe une clé JSON par son nom.
pub fn classer_cle(nom: &str) -> ClasseCle {
    let k: String = nom
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    if k.is_empty() {
        return ClasseCle::Inconnue;
    }
    let c = |s: &str| k.contains(s);
    if [
        "password",
        "motdepasse",
        "secret",
        "token",
        "apikey",
        "refresh",
        "passkey",
        "otp",
        "base64",
        "credential",
    ]
    .iter()
    .any(|s| c(s))
    {
        return ClasseCle::Effacer;
    }
    if (k.ends_with("key") || k.ends_with("keys") || k.ends_with("url"))
        && [
            "s3",
            "photo",
            "pdf",
            "file",
            "doc",
            "fichier",
            "signature",
            "logo",
            "image",
            "justificatif",
            "piece",
            "attachment",
            "pap",
        ]
        .iter()
        .any(|s| c(s))
    {
        return ClasseCle::CleS3;
    }
    if c("externaluserid") || c("externalid") {
        return ClasseCle::Genre(Genre::Identifiant);
    }
    if c("email") || c("mail") {
        return ClasseCle::Genre(Genre::Email);
    }
    if k == "tel"
        || ["phone", "telephone", "mobile", "portable", "gsm"]
            .iter()
            .any(|s| c(s))
    {
        return ClasseCle::Genre(Genre::Telephone);
    }
    if c("prenom") || c("firstname") || c("givenname") {
        return ClasseCle::Genre(Genre::Prenom);
    }
    if c("lastname")
        || c("familyname")
        || c("surname")
        || c("nomdefamille")
        || c("nomfamille")
        || k == "nom"
    {
        return ClasseCle::Genre(Genre::Nom);
    }
    if matches!(k.as_str(), "ine" | "eleveine" | "numeroine" | "ineeleve") {
        return ClasseCle::Genre(Genre::Ine);
    }
    if c("iban") {
        return ClasseCle::Genre(Genre::Iban);
    }
    if k == "bic" {
        return ClasseCle::Genre(Genre::Bic);
    }
    if k == "rum" {
        return ClasseCle::Genre(Genre::Rum);
    }
    if c("naissance") || c("birth") || k == "dob" {
        return if c("lieu") || c("place") {
            ClasseCle::Genre(Genre::Ville)
        } else {
            ClasseCle::Genre(Genre::DateNaissance)
        };
    }
    if c("codepostal") || c("zip") || c("postal") {
        return ClasseCle::Genre(Genre::CodePostal);
    }
    if c("adresse") || c("address") || c("street") {
        return ClasseCle::Genre(Genre::Adresse);
    }
    if k == "ville" || k == "city" || k == "commune" {
        return ClasseCle::Genre(Genre::Ville);
    }
    if k == "ip" || c("ipaddress") {
        return ClasseCle::Genre(Genre::Ip);
    }
    if c("useragent") {
        return ClasseCle::Genre(Genre::Agent);
    }
    if k == "filename" || c("nomfichier") || k == "file" || k == "fichier" {
        return ClasseCle::Genre(Genre::NomFichier);
    }
    if !NON_NOMS.contains(&k.as_str())
        && (k.ends_with("name") || k.ends_with("nom") || c("fullname") || c("displayname"))
    {
        return ClasseCle::Genre(Genre::NomComplet);
    }
    if [
        "comment",
        "commentaire",
        "note",
        "motif",
        "reason",
        "description",
        "observation",
        "message",
        "body",
        "text",
        "texte",
        "contenu",
        "corps",
        "detail",
        "appreciation",
        "remarque",
        "justification",
        "subject",
        "sujet",
        "titre",
        "title",
        "label",
        "libelle",
        "intro",
        "content",
        "preview",
    ]
    .iter()
    .any(|s| c(s))
    {
        return ClasseCle::Genre(Genre::Texte);
    }
    if GARDER_EXACT.contains(&k.as_str()) || GARDER_SUFFIXES.iter().any(|s| k.ends_with(s)) {
        return ClasseCle::Garder;
    }
    ClasseCle::Inconnue
}

/// Statistiques de traitement (noms de clés seulement, jamais de valeurs).
#[derive(Debug, Default, Clone)]
pub struct StatsJson {
    pub cles_inconnues: BTreeMap<String, u64>,
    pub feuilles_traitees: u64,
}

fn est_nom_de_cle_sur(k: &str) -> bool {
    !k.is_empty()
        && k.len() <= 48
        && k.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn forme_ine(s: &str) -> bool {
    let n = s.len();
    (10..=12).contains(&n)
        && s.chars().all(|c| c.is_ascii_alphanumeric())
        && s.chars().filter(|c| c.is_ascii_digit()).count() >= 8
}

fn forme_date_iso(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
}

pub struct Contexte<'a> {
    pub cle: &'a Cle,
    pub stats: &'a mut StatsJson,
    pub sensibles: &'a mut Sensibles,
}

impl Contexte<'_> {
    /// Réécrit le nom d'une clé d'objet qui ressemble à une donnée (e-mail, INE, nom…) : les noms de
    /// structure sont conservés.
    fn nom_de_cle(&mut self, k: &str) -> String {
        if est_uuid(k) || (est_nom_de_cle_sur(k) && !forme_ine(k)) {
            return k.to_string();
        }
        if k.contains('@') {
            self.sensibles.noter(Genre::Email, k);
            return pseudonymiser(self.cle, Genre::Email, k);
        }
        if forme_ine(k) {
            self.sensibles.noter(Genre::Ine, k);
            return pseudonymiser(self.cle, Genre::Ine, k);
        }
        pseudonymiser(self.cle, Genre::Identifiant, k)
    }

    fn feuille_chaine(&mut self, s: &str, nom: Option<&str>) -> String {
        self.stats.feuilles_traitees += 1;
        if s.is_empty() {
            return String::new();
        }
        let classe = nom.map(classer_cle).unwrap_or(ClasseCle::Inconnue);
        match classe {
            ClasseCle::Genre(g) => {
                self.sensibles.noter(g, s);
                pseudonymiser(self.cle, g, s)
            }
            ClasseCle::CleS3 => reecrire_cle(self.cle, s),
            ClasseCle::Effacer => String::new(),
            ClasseCle::Garder => s.to_string(),
            ClasseCle::Inconnue => {
                // Forme sans risque : UUID, date ISO, booléen textuel, entier court.
                if est_uuid(s)
                    || forme_date_iso(s)
                    || matches!(s, "true" | "false")
                    || (s.len() <= 6 && s.chars().all(|c| c.is_ascii_digit()))
                {
                    return s.to_string();
                }
                if forme_ine(s) {
                    self.sensibles.noter(Genre::Ine, s);
                    return pseudonymiser(self.cle, Genre::Ine, s);
                }
                let cle_stat = match nom {
                    Some(n) if est_nom_de_cle_sur(n) => n.to_string(),
                    Some(_) => "(clé non structurelle)".to_string(),
                    None => "(sans clé)".to_string(),
                };
                *self.stats.cles_inconnues.entry(cle_stat).or_default() += 1;
                texte(self.cle, s)
            }
        }
    }

    /// Anonymise une valeur JSON ; `nom` = nom de la clé qui la porte (hérité par les éléments d'un tableau).
    pub fn valeur(&mut self, v: &Value, nom: Option<&str>) -> Value {
        match v {
            Value::String(s) => Value::String(self.feuille_chaine(s, nom)),
            Value::Array(a) => Value::Array(a.iter().map(|x| self.valeur(x, nom)).collect()),
            Value::Object(o) => {
                let mut m = Map::new();
                for (k, x) in o {
                    let nk = self.nom_de_cle(k);
                    // Le nom de clé d'origine sert à classer la valeur ; s'il a été réécrit (donnée),
                    // la valeur est traitée comme inconnue.
                    let nv = self.valeur(x, if nk == *k { Some(k.as_str()) } else { None });
                    m.insert(nk, nv);
                }
                Value::Object(m)
            }
            autre => autre.clone(),
        }
    }
}

/// Anonymise un texte JSON ; `None` si ce n'est pas du JSON valide.
pub fn anonymiser_texte_json(ctx: &mut Contexte, brut: &str) -> Option<String> {
    let v: Value = serde_json::from_str(brut).ok()?;
    serde_json::to_string(&ctx.valeur(&v, None)).ok()
}

/// Dernier segment **non numérique** d'un chemin `a.b.0.nom` → `nom` (nom de la clé de la feuille).
pub fn cle_de_chemin(chemin: &str) -> Option<&str> {
    chemin
        .rsplit('.')
        .find(|s| !s.is_empty() && !s.chars().all(|c| c.is_ascii_digit()))
}

/// Réécrit un chemin de table clé-valeur : segments numériques, `__len`, `__root`, UUID et noms de
/// clé de structure conservés ; tout segment qui ressemble à une donnée (INE, e-mail, mot avec
/// espace ou trait d'union…) est remplacé. Un chemin contenant `@` est remplacé en entier (un
/// e-mail contient des points).
pub fn reecrire_chemin(cle: &Cle, chemin: &str) -> String {
    if chemin.contains('@') {
        return pseudonymiser(cle, Genre::Identifiant, chemin);
    }
    chemin
        .split('.')
        .map(|seg| {
            if seg.is_empty()
                || seg == "__len"
                || seg == "__root"
                || (seg.len() <= 6 && seg.chars().all(|c| c.is_ascii_digit()))
                || est_uuid(seg)
                || (est_nom_de_cle_sur(seg) && !forme_ine(seg))
            {
                seg.to_string()
            } else {
                pseudonymiser(cle, Genre::Identifiant, seg)
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Valeur d'une table clé-valeur : `s:`, `n:`, `b:`, `null:null`, `undef:`, `j:`.
pub fn anonymiser_attr(ctx: &mut Contexte, chemin: &str, valeur: &str) -> String {
    let nom = cle_de_chemin(chemin);
    if let Some(s) = valeur.strip_prefix("s:") {
        return format!("s:{}", ctx.feuille_chaine(s, nom));
    }
    if let Some(j) = valeur.strip_prefix("j:") {
        return match anonymiser_texte_json(ctx, j) {
            Some(t) => format!("j:{t}"),
            // Pas du JSON valide : jamais gardé en clair.
            None => format!("j:{}", texte(ctx.cle, j)),
        };
    }
    if valeur.starts_with("n:")
        || valeur.starts_with("b:")
        || valeur == "null:null"
        || valeur == "undef:"
    {
        return valeur.to_string();
    }
    // Préfixe inconnu : fermé par défaut.
    ctx.stats.feuilles_traitees += 1;
    *ctx.stats
        .cles_inconnues
        .entry("(valeur sans préfixe connu)".to_string())
        .or_default() += 1;
    texte(ctx.cle, valeur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cle() -> Cle {
        Cle::depuis_texte(&"ef".repeat(32)).unwrap()
    }

    #[test]
    fn classement_des_cles() {
        assert_eq!(classer_cle("email"), ClasseCle::Genre(Genre::Email));
        assert_eq!(classer_cle("parent_emails"), ClasseCle::Genre(Genre::Email));
        assert_eq!(classer_cle("Prenom"), ClasseCle::Genre(Genre::Prenom));
        assert_eq!(classer_cle("nom"), ClasseCle::Genre(Genre::Nom));
        assert_eq!(classer_cle("nombre"), ClasseCle::Garder);
        assert_eq!(classer_cle("telephone"), ClasseCle::Genre(Genre::Telephone));
        assert_eq!(classer_cle("photo_key"), ClasseCle::CleS3);
        assert_eq!(classer_cle("access_token"), ClasseCle::Effacer);
        assert_eq!(classer_cle("created_at"), ClasseCle::Garder);
        assert_eq!(classer_cle("userId"), ClasseCle::Garder);
        assert_eq!(classer_cle("commentaire"), ClasseCle::Genre(Genre::Texte));
        assert_eq!(
            classer_cle("cle_inconnue_du_dictionnaire"),
            ClasseCle::Inconnue
        );
        assert_eq!(
            classer_cle("auteurNom"),
            ClasseCle::Genre(Genre::NomComplet)
        );
        assert_eq!(
            classer_cle("file_name"),
            ClasseCle::Genre(Genre::NomFichier)
        );
    }

    #[test]
    fn objet_complet_aucune_valeur_reelle_ne_reste() {
        let c = cle();
        let mut st = StatsJson::default();
        let mut sens = Sensibles::default();
        let mut ctx = Contexte {
            cle: &c,
            stats: &mut st,
            sensibles: &mut sens,
        };
        let v = json!({
            "nom": "Zorglub", "prenom": "Aldric", "email": "aldric@example.test", "statut": "actif", "nombre": 3,
            "actif": true, "classes": ["6A", "5B"],
            "contact": {"telephone": "06 12 34 56 78", "adresse": "12 rue des Lilas-Fictifs"},
            "commentaire": "Texte libre sur Zorglub", "cle_inconnue": "valeur libre Zorglub", "photo_key": "eleves/dupont/p.jpg",
            "id": "123e4567-e89b-42d3-a456-426614174000", "cree": "2026-09-01T08:00:00Z", "n": null
        });
        let r = ctx.valeur(&v, None);
        let s = r.to_string();
        for interdit in [
            "Zorglub",
            "Aldric",
            "aldric@example.test",
            "06 12",
            "Lilas",
            "dupont",
        ] {
            assert!(!s.contains(interdit), "{interdit} dans {s}");
        }
        assert_eq!(r["nombre"], 3);
        assert_eq!(r["actif"], true);
        assert_eq!(r["classes"], json!(["6A", "5B"]));
        assert_eq!(r["statut"], "actif");
        assert_eq!(r["id"], "123e4567-e89b-42d3-a456-426614174000");
        assert_eq!(r["cree"], "2026-09-01T08:00:00Z");
        assert!(r["n"].is_null());
        assert_eq!(st.cles_inconnues.get("cle_inconnue"), Some(&1));
        assert!(sens.fuite("zorglub"));
    }

    #[test]
    fn cles_d_objet_nominatives_reecrites() {
        let c = cle();
        let mut st = StatsJson::default();
        let mut sens = Sensibles::default();
        let mut ctx = Contexte {
            cle: &c,
            stats: &mut st,
            sensibles: &mut sens,
        };
        let r = ctx.valeur(&json!({"jean.dupont@example.test": 1, "12345678901": 2, "dupont-jean": 3, "statut_x": 4}), None);
        let s = r.to_string();
        assert!(!s.contains("dupont") && !s.contains("12345678901"), "{s}");
        assert!(s.contains("statut_x"));
    }

    #[test]
    fn chemins_reecrits_seulement_si_ils_ressemblent_a_une_donnee() {
        let c = cle();
        assert_eq!(reecrire_chemin(&c, "notes.0.texte"), "notes.0.texte");
        assert_eq!(reecrire_chemin(&c, "__root"), "__root");
        assert_eq!(reecrire_chemin(&c, "liste.__len"), "liste.__len");
        let u = "123e4567-e89b-42d3-a456-426614174000";
        assert_eq!(reecrire_chemin(&c, &format!("{u}.nom")), format!("{u}.nom"));
        let r = reecrire_chemin(&c, "jean.dupont@example.test.classe");
        assert!(!r.contains("dupont") && !r.contains("jean"), "{r}");
        let r = reecrire_chemin(&c, "12345678901.classe");
        assert!(!r.contains("12345678901") && r.ends_with(".classe"), "{r}");
        let r = reecrire_chemin(&c, "Dupont Jean.classe");
        assert!(!r.contains("Dupont"), "{r}");
        assert_eq!(reecrire_chemin(&c, "x"), reecrire_chemin(&c, "x"));
    }

    #[test]
    fn attr_prefixes() {
        let c = cle();
        let mut st = StatsJson::default();
        let mut sens = Sensibles::default();
        let mut ctx = Contexte {
            cle: &c,
            stats: &mut st,
            sensibles: &mut sens,
        };
        let a = anonymiser_attr(&mut ctx, "contact.nom", "s:Zorglub");
        assert!(a.starts_with("s:") && !a.contains("Zorglub"));
        assert_eq!(anonymiser_attr(&mut ctx, "age", "n:12"), "n:12");
        assert_eq!(anonymiser_attr(&mut ctx, "actif", "b:1"), "b:1");
        assert_eq!(anonymiser_attr(&mut ctx, "x", "null:null"), "null:null");
        let j = anonymiser_attr(
            &mut ctx,
            "__root",
            "j:{\"123e4567-e89b-42d3-a456-426614174000\":\"0123456789A\"}",
        );
        assert!(j.starts_with("j:") && !j.contains("0123456789A"), "{j}");
        assert!(j.contains("123e4567-e89b-42d3-a456-426614174000"));
        let inconnu = anonymiser_attr(&mut ctx, "x", "Zorglub-sans-prefixe");
        assert!(!inconnu.contains("Zorglub"));
        let json_casse = anonymiser_attr(&mut ctx, "x", "j:{pas du json Zorglub");
        assert!(!json_casse.contains("Zorglub"));
        assert_eq!(cle_de_chemin("notes.0.texte"), Some("texte"));
        assert_eq!(cle_de_chemin("liste.3"), Some("liste"));
    }
}
