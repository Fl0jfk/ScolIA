//! Logique d'écran de la messagerie : pure, sans Leptos ni réseau, testée en natif
//! (tri, filtres, non lus, validation des saisies et des pièces jointes, libellés).
//! Les droits, la visibilité des fils et l'accusé de lecture restent côté serveur.

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Timelike};
use ent_web_api::{ApiError, FilBoite, MessageFil};
use uuid::Uuid;

/// Mois abrégés à la française.
const MOIS_COURTS: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];

/// Longueur maximale d'un objet / d'un message (confort : le serveur reste l'autorité).
pub const SUJET_MAX: usize = 200;
pub const MESSAGE_MAX: usize = 10_000;
/// Taille maximale d'une pièce jointe acceptée par le serveur (`MSG_PIECE_MAX_OCTETS`).
pub const PIECE_MAX_OCTETS: usize = 256 * 1024;
/// Types de pièces jointes proposés (sous-ensemble prudent de ceux acceptés par le serveur).
pub const MIME_PIECES: [&str; 4] = ["application/pdf", "image/jpeg", "image/png", "text/plain"];
/// Longueur maximale d'un libellé de dossier (règle du serveur).
pub const DOSSIER_MAX: usize = 40;

// ---------------------------------------------------------------------------
// Boîte de réception
// ---------------------------------------------------------------------------

/// Vue de la boîte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Vue {
    /// Conversations non archivées.
    #[default]
    Actifs,
    /// Conversations non archivées ayant au moins un message non lu.
    NonLus,
    /// Conversations archivées (personnel).
    Archives,
}

impl Vue {
    pub fn depuis(cle: &str) -> Vue {
        match cle {
            "non_lus" => Vue::NonLus,
            "archives" => Vue::Archives,
            _ => Vue::Actifs,
        }
    }

    pub fn cle(self) -> &'static str {
        match self {
            Vue::Actifs => "actifs",
            Vue::NonLus => "non_lus",
            Vue::Archives => "archives",
        }
    }
}

/// Filtres de la boîte.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filtres {
    pub vue: Vue,
    /// Recherche libre (objet, élève, dossier) ; insensible à la casse et aux accents.
    pub recherche: String,
    /// Dossier exact ; vide = tous.
    pub dossier: String,
    /// La vue « Toutes » inclut les conversations archivées (famille : pas de vue dédiée).
    pub avec_archives: bool,
}

/// Minuscules, sans accents courants, espaces compactés (recherche tolérante).
pub fn normaliser(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for c in texte.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => sortie.push('a'),
            'é' | 'è' | 'ê' | 'ë' => sortie.push('e'),
            'î' | 'ï' | 'í' => sortie.push('i'),
            'ô' | 'ö' | 'ó' | 'õ' => sortie.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => sortie.push('u'),
            'ç' => sortie.push('c'),
            'ñ' => sortie.push('n'),
            'œ' => sortie.push_str("oe"),
            'æ' => sortie.push_str("ae"),
            c => sortie.push(c),
        }
    }
    sortie.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn est_non_lu(fil: &FilBoite) -> bool {
    !fil.archive && fil.nb_non_lus > 0
}

/// Instant d'un horodatage RFC 3339 du serveur.
fn instant(iso: &str) -> Option<DateTime<chrono::FixedOffset>> {
    DateTime::parse_from_rfc3339(iso).ok()
}

/// Date et heure locales (navigateur) d'un horodatage du serveur.
pub fn en_local(iso: &str) -> Option<NaiveDateTime> {
    instant(iso).map(|d| d.with_timezone(&chrono::Local).naive_local())
}

/// Plus récents d'abord ; à égalité (ou date illisible), ordre stable par objet.
pub fn trier(fils: &mut [FilBoite]) {
    fils.sort_by(|a, b| {
        let (ia, ib) = (instant(&a.last_message_at), instant(&b.last_message_at));
        ib.cmp(&ia).then_with(|| a.sujet.cmp(&b.sujet))
    });
}

fn correspond(fil: &FilBoite, requete: &str) -> bool {
    if requete.is_empty() {
        return true;
    }
    let botte = normaliser(&format!(
        "{} {} {}",
        fil.sujet,
        fil.eleve_label.as_deref().unwrap_or(""),
        fil.dossier.as_deref().unwrap_or("")
    ));
    requete.split(' ').all(|mot| botte.contains(mot))
}

/// Fils à afficher : vue, dossier et recherche appliqués, triés.
pub fn filtrer(fils: &[FilBoite], filtres: &Filtres) -> Vec<FilBoite> {
    let requete = normaliser(&filtres.recherche);
    let dossier = filtres.dossier.trim();
    let mut sortie: Vec<FilBoite> = fils
        .iter()
        .filter(|f| match filtres.vue {
            Vue::Actifs => filtres.avec_archives || !f.archive,
            Vue::NonLus => est_non_lu(f),
            Vue::Archives => f.archive,
        })
        .filter(|f| dossier.is_empty() || f.dossier.as_deref() == Some(dossier))
        .filter(|f| correspond(f, &requete))
        .cloned()
        .collect();
    trier(&mut sortie);
    sortie
}

/// Compteurs des vues : (actifs, fils non lus, archivés).
pub fn compteurs(fils: &[FilBoite]) -> (usize, usize, usize) {
    let actifs = fils.iter().filter(|f| !f.archive).count();
    let non_lus = fils.iter().filter(|f| est_non_lu(f)).count();
    let archives = fils.iter().filter(|f| f.archive).count();
    (actifs, non_lus, archives)
}

/// Dossiers présents (sans doublon, ordre alphabétique).
pub fn dossiers(fils: &[FilBoite]) -> Vec<String> {
    let mut v: Vec<String> = fils
        .iter()
        .filter_map(|f| f.dossier.clone())
        .filter(|d| !d.trim().is_empty())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// « 1 conversation », « 3 conversations », « Aucune conversation ».
pub fn libelle_nombre(n: usize) -> String {
    match n {
        0 => "Aucune conversation".into(),
        1 => "1 conversation".into(),
        n => format!("{n} conversations"),
    }
}

/// « 14:05 ».
fn heure(d: NaiveDateTime) -> String {
    format!("{:02}:{:02}", d.hour(), d.minute())
}

/// « 3 nov. », « 3 nov. 2025 ».
fn jour_mois(d: NaiveDate, aujourdhui: NaiveDate) -> String {
    let mois = MOIS_COURTS[(d.month0() as usize).min(11)];
    let jour = if d.day() == 1 {
        "1er".to_string()
    } else {
        d.day().to_string()
    };
    if d.year() == aujourdhui.year() {
        format!("{jour} {mois}")
    } else {
        format!("{jour} {mois} {}", d.year())
    }
}

/// Date courte d'une ligne de la boîte : heure aujourd'hui, « Hier », sinon jour et mois.
pub fn libelle_date_court(date: NaiveDateTime, aujourdhui: NaiveDate) -> String {
    let jour = date.date();
    if jour == aujourdhui {
        heure(date)
    } else if aujourdhui.pred_opt() == Some(jour) {
        "Hier".into()
    } else {
        jour_mois(jour, aujourdhui)
    }
}

/// Date d'un message : « Aujourd'hui à 14:05 », « Hier à 14:05 », « 3 nov. à 14:05 ».
pub fn libelle_date_heure(date: NaiveDateTime, aujourdhui: NaiveDate) -> String {
    let jour = date.date();
    if jour == aujourdhui {
        format!("Aujourd'hui à {}", heure(date))
    } else if aujourdhui.pred_opt() == Some(jour) {
        format!("Hier à {}", heure(date))
    } else {
        format!("{} à {}", jour_mois(jour, aujourdhui), heure(date))
    }
}

/// « 3 messages non lus », « 1 message non lu ».
pub fn libelle_non_lus_fil(n: i64) -> String {
    if n == 1 {
        "1 message non lu".into()
    } else {
        format!("{n} messages non lus")
    }
}

// ---------------------------------------------------------------------------
// Fil
// ---------------------------------------------------------------------------

/// Messages dans l'ordre chronologique (le plus ancien d'abord).
pub fn ordonner_messages(mut messages: Vec<MessageFil>) -> Vec<MessageFil> {
    messages.sort_by(|a, b| {
        instant(&a.created_at)
            .cmp(&instant(&b.created_at))
            .then_with(|| a.id.cmp(&b.id))
    });
    messages
}

/// Nom de l'auteur affiché : « Vous » pour soi, sinon le libellé du serveur.
pub fn auteur_affiche(message: &MessageFil, moi: Option<Uuid>) -> String {
    if moi == Some(message.auteur_user_id) {
        return "Vous".into();
    }
    message
        .auteur_label
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .unwrap_or("Utilisateur de l'établissement")
        .to_string()
}

/// Accusé de lecture d'un de ses propres messages (« Lu par 2 personnes »).
pub fn libelle_accuses(n: i64) -> Option<String> {
    match n {
        n if n <= 0 => None,
        1 => Some("Lu par 1 personne".into()),
        n => Some(format!("Lu par {n} personnes")),
    }
}

/// « 240 o », « 1,5 Ko », « 2,0 Mo ».
pub fn taille_lisible(octets: i64) -> String {
    let o = octets.max(0) as f64;
    if o < 1024.0 {
        format!("{} o", o as i64)
    } else if o < 1024.0 * 1024.0 {
        format!("{:.1} Ko", o / 1024.0).replace('.', ",")
    } else {
        format!("{:.1} Mo", o / (1024.0 * 1024.0)).replace('.', ",")
    }
}

/// Nom de fichier sûr pour l'enregistrement : sans séparateur de chemin ni caractère de contrôle.
pub fn nom_fichier_sur(nom: &str) -> String {
    let net: String = nom
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let net = net.trim().trim_start_matches('.').trim();
    if net.is_empty() {
        "piece-jointe".into()
    } else {
        net.chars().take(120).collect()
    }
}

// ---------------------------------------------------------------------------
// Saisies
// ---------------------------------------------------------------------------

pub fn valider_sujet(sujet: &str) -> Result<String, String> {
    let s = sujet.trim();
    if s.is_empty() {
        Err("Indiquez l'objet du message.".into())
    } else if s.chars().count() > SUJET_MAX {
        Err(format!("L'objet est limité à {SUJET_MAX} caractères."))
    } else {
        Ok(s.to_string())
    }
}

pub fn valider_message(message: &str) -> Result<String, String> {
    let m = message.trim();
    if m.is_empty() {
        Err("Écrivez votre message.".into())
    } else if m.chars().count() > MESSAGE_MAX {
        Err(format!("Le message est limité à {MESSAGE_MAX} caractères."))
    } else {
        Ok(m.to_string())
    }
}

/// Libellé de dossier : minuscules, lettres, chiffres, `_` et `-`, 40 caractères au plus ;
/// vide = aucun dossier (même règle que le serveur).
pub fn valider_dossier(dossier: &str) -> Result<Option<String>, String> {
    let d = dossier.trim().to_ascii_lowercase();
    if d.is_empty() {
        return Ok(None);
    }
    if d.chars().count() > DOSSIER_MAX
        || !d
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "Le dossier ne peut contenir que des lettres, chiffres, tirets et tirets bas ({DOSSIER_MAX} caractères au plus)."
        ));
    }
    Ok(Some(d))
}

/// Priorité acceptée par le serveur (`normale` ou `haute`).
pub fn valider_priorite(priorite: &str) -> &'static str {
    if priorite == "haute" {
        "haute"
    } else {
        "normale"
    }
}

/// Pièce jointe acceptée côté interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PieceValide {
    pub nom: String,
    pub mime: String,
}

/// Vérifie nom, type et taille avant de lire ou d'envoyer le fichier.
pub fn valider_piece(nom: &str, mime: &str, taille: usize) -> Result<PieceValide, String> {
    if taille == 0 {
        return Err("Ce fichier est vide.".into());
    }
    if taille > PIECE_MAX_OCTETS {
        return Err(format!(
            "Ce fichier est trop volumineux ({} ; maximum {}).",
            taille_lisible(taille as i64),
            taille_lisible(PIECE_MAX_OCTETS as i64)
        ));
    }
    let mime = mime.trim().to_ascii_lowercase();
    if !MIME_PIECES.contains(&mime.as_str()) {
        return Err("Type de fichier non accepté : PDF, image JPEG ou PNG, texte brut.".into());
    }
    Ok(PieceValide {
        nom: nom_fichier_sur(nom),
        mime,
    })
}

/// « NOM Prénom (classe) ».
pub fn libelle_eleve(nom: &str, prenom: &str, classe: Option<&str>) -> String {
    match classe.map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) => format!("{nom} {prenom} ({c})"),
        None => format!("{nom} {prenom}"),
    }
}

/// Une erreur après laquelle il ne faut rien enchaîner : session perdue, droits refusés,
/// trop de requêtes (règle des écritures, FRONTEND_LEPTOS.md § 11).
pub fn arret_definitif(erreur: &ApiError) -> bool {
    matches!(
        erreur,
        ApiError::NonConnecte | ApiError::Interdit(_) | ApiError::TropDeRequetes
    )
}

/// Déroulé d'un envoi avec pièce jointe : le message est créé d'abord, la pièce ensuite.
/// Résultat de l'étape « pièce » après un message déjà envoyé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IssuePiece {
    SansPiece,
    Jointe,
    /// Le message est parti, pas la pièce (à signaler sans renvoyer le message).
    Echec(String),
}

/// Phrase de confirmation affichée après l'envoi.
pub fn message_apres_envoi(piece: &IssuePiece) -> String {
    match piece {
        IssuePiece::SansPiece => "Message envoyé.".into(),
        IssuePiece::Jointe => "Message envoyé avec sa pièce jointe.".into(),
        IssuePiece::Echec(raison) => {
            format!("Message envoyé, mais la pièce jointe n'a pas pu l'être : {raison}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fil(sujet: &str, date: &str, non_lus: i64, archive: bool) -> FilBoite {
        FilBoite {
            id: Uuid::from_u128(sujet.bytes().fold(7u128, |a, b| a * 31 + u128::from(b))),
            eleve_id: Uuid::nil(),
            eleve_label: Some("POTTER Harry".into()),
            sujet: sujet.into(),
            created_by_user_id: None,
            created_at: date.into(),
            last_message_at: date.into(),
            archive,
            priorite: "normale".into(),
            dossier: None,
            nb_non_lus: non_lus,
        }
    }

    fn message(auteur: u128, date: &str, corps: &str) -> MessageFil {
        MessageFil {
            id: Uuid::from_u128(date.len() as u128 + corps.len() as u128),
            fil_id: Uuid::nil(),
            auteur_user_id: Uuid::from_u128(auteur),
            auteur_label: Some("Rogue Severus".into()),
            corps: corps.into(),
            created_at: date.into(),
            lu_par_moi: false,
            nb_accuses: 0,
            pieces: vec![],
        }
    }

    fn jeu() -> Vec<FilBoite> {
        let mut dossier = fil("Retard répété", "2026-11-03T08:00:00+00:00", 0, false);
        dossier.dossier = Some("vs".into());
        dossier.eleve_label = Some("WEASLEY Ron".into());
        vec![
            fil("Sortie au musée", "2026-11-01T08:00:00+00:00", 2, false),
            fil("Réunion parents", "2026-11-02T08:00:00+00:00", 0, false),
            dossier,
            fil("Ancienne affaire", "2026-10-01T08:00:00+00:00", 3, true),
        ]
    }

    #[test]
    fn normalisation_sans_accents_ni_casse() {
        assert_eq!(normaliser("  Réunion   PARENTS  "), "reunion parents");
        assert_eq!(normaliser("Élève Chloé Œuf"), "eleve chloe oeuf");
    }

    #[test]
    fn vues_actifs_non_lus_archives() {
        let jeu = jeu();
        let titres = |f: Filtres| -> Vec<String> {
            filtrer(&jeu, &f).into_iter().map(|x| x.sujet).collect()
        };
        assert_eq!(
            titres(Filtres::default()),
            ["Retard répété", "Réunion parents", "Sortie au musée"],
            "actifs, du plus récent au plus ancien, sans archivés"
        );
        assert_eq!(
            titres(Filtres {
                vue: Vue::NonLus,
                ..Default::default()
            }),
            ["Sortie au musée"],
            "un fil archivé n'est jamais « non lu »"
        );
        assert_eq!(
            titres(Filtres {
                vue: Vue::Archives,
                ..Default::default()
            }),
            ["Ancienne affaire"]
        );
        assert_eq!(compteurs(&jeu), (3, 1, 1));
    }

    #[test]
    fn la_famille_garde_acces_aux_archives() {
        let jeu = jeu();
        let toutes = filtrer(
            &jeu,
            &Filtres {
                avec_archives: true,
                ..Default::default()
            },
        );
        assert_eq!(toutes.len(), 4);
        assert_eq!(filtrer(&jeu, &Filtres::default()).len(), 3);
    }

    #[test]
    fn recherche_et_dossier() {
        let jeu = jeu();
        let filtre = |recherche: &str, dossier: &str| Filtres {
            recherche: recherche.into(),
            dossier: dossier.into(),
            ..Default::default()
        };
        assert_eq!(
            filtrer(&jeu, &filtre("REUNION", "")).len(),
            1,
            "accents et casse"
        );
        assert_eq!(
            filtrer(&jeu, &filtre("weasley", "")).len(),
            1,
            "libellé de l'élève"
        );
        assert_eq!(
            filtrer(&jeu, &filtre("potter musee", "")).len(),
            1,
            "tous les mots"
        );
        assert_eq!(filtrer(&jeu, &filtre("inconnu", "")).len(), 0);
        assert_eq!(filtrer(&jeu, &filtre("", "vs")).len(), 1);
        assert_eq!(filtrer(&jeu, &filtre("", "scolarite")).len(), 0);
        assert_eq!(dossiers(&jeu), ["vs"]);
        assert_eq!(
            filtrer(&jeu, &filtre("retard", "vs"))[0].sujet,
            "Retard répété"
        );
    }

    #[test]
    fn tri_par_date_reelle_et_non_par_texte() {
        // Fuseaux différents : l'instant compte, pas la chaîne.
        let mut v = vec![
            fil("a", "2026-11-03T10:00:00+02:00", 0, false), // 08:00 UTC
            fil("b", "2026-11-03T09:00:00+00:00", 0, false), // 09:00 UTC
        ];
        trier(&mut v);
        assert_eq!(v[0].sujet, "b");
        // Date illisible : en dernier, sans panique.
        let mut v = vec![
            fil("x", "pas une date", 0, false),
            fil("y", "2026-11-03T09:00:00+00:00", 0, false),
        ];
        trier(&mut v);
        assert_eq!(v[0].sujet, "y");
    }

    #[test]
    fn libelles_de_dates() {
        let auj = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
        let d = |j: u32, an: i32| {
            NaiveDate::from_ymd_opt(an, 11, j)
                .unwrap()
                .and_hms_opt(14, 5, 0)
                .unwrap()
        };
        assert_eq!(libelle_date_court(d(4, 2026), auj), "14:05");
        assert_eq!(libelle_date_court(d(3, 2026), auj), "Hier");
        assert_eq!(libelle_date_court(d(1, 2026), auj), "1er nov.");
        assert_eq!(libelle_date_court(d(2, 2025), auj), "2 nov. 2025");
        assert_eq!(libelle_date_heure(d(4, 2026), auj), "Aujourd'hui à 14:05");
        assert_eq!(libelle_date_heure(d(3, 2026), auj), "Hier à 14:05");
        assert_eq!(libelle_date_heure(d(2, 2026), auj), "2 nov. à 14:05");
        // Mois courts : pas d'abréviation inutile.
        let mai = NaiveDate::from_ymd_opt(2026, 5, 12)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap();
        assert_eq!(libelle_date_court(mai, auj), "12 mai");
        assert_eq!(libelle_nombre(0), "Aucune conversation");
        assert_eq!(libelle_nombre(1), "1 conversation");
        assert_eq!(libelle_nombre(5), "5 conversations");
        assert_eq!(libelle_non_lus_fil(1), "1 message non lu");
        assert_eq!(libelle_non_lus_fil(3), "3 messages non lus");
    }

    #[test]
    fn messages_dans_l_ordre_chronologique() {
        let m = ordonner_messages(vec![
            message(1, "2026-11-03T09:00:00+00:00", "deuxième"),
            message(2, "2026-11-03T08:00:00+00:00", "premier"),
        ]);
        assert_eq!(m[0].corps, "premier");
    }

    #[test]
    fn auteur_et_accuses() {
        let m = message(7, "2026-11-03T09:00:00+00:00", "x");
        assert_eq!(auteur_affiche(&m, Some(Uuid::from_u128(7))), "Vous");
        assert_eq!(
            auteur_affiche(&m, Some(Uuid::from_u128(8))),
            "Rogue Severus"
        );
        let mut sans = m.clone();
        sans.auteur_label = Some("  ".into());
        assert_eq!(
            auteur_affiche(&sans, None),
            "Utilisateur de l'établissement"
        );
        assert_eq!(libelle_accuses(0), None);
        assert_eq!(libelle_accuses(1).as_deref(), Some("Lu par 1 personne"));
        assert_eq!(libelle_accuses(3).as_deref(), Some("Lu par 3 personnes"));
    }

    #[test]
    fn validation_des_saisies() {
        assert!(valider_sujet("   ").is_err());
        assert_eq!(valider_sujet("  Sortie  ").unwrap(), "Sortie");
        assert!(valider_sujet(&"x".repeat(SUJET_MAX + 1)).is_err());
        assert!(valider_message("\n \t").is_err());
        assert_eq!(valider_message(" Bonjour \n").unwrap(), "Bonjour");
        assert!(valider_message(&"é".repeat(MESSAGE_MAX + 1)).is_err());
        assert!(
            valider_message(&"é".repeat(MESSAGE_MAX)).is_ok(),
            "caractères, pas octets"
        );
    }

    #[test]
    fn validation_du_dossier_comme_le_serveur() {
        assert_eq!(valider_dossier(" VS ").unwrap().as_deref(), Some("vs"));
        assert_eq!(valider_dossier("  ").unwrap(), None);
        assert!(valider_dossier("vie scolaire").is_err());
        assert!(valider_dossier("é").is_err());
        assert!(valider_dossier(&"a".repeat(41)).is_err());
        assert!(valider_dossier(&"a".repeat(40)).is_ok());
        assert_eq!(valider_priorite("haute"), "haute");
        assert_eq!(valider_priorite("urgent"), "normale");
    }

    #[test]
    fn validation_des_pieces_jointes() {
        let ok = valider_piece("devis.pdf", "application/pdf", 1000).unwrap();
        assert_eq!(ok.mime, "application/pdf");
        assert!(valider_piece("a.pdf", "application/pdf", 0).is_err());
        assert!(valider_piece("a.pdf", "application/pdf", PIECE_MAX_OCTETS).is_ok());
        let trop = valider_piece("a.pdf", "application/pdf", PIECE_MAX_OCTETS + 1).unwrap_err();
        assert!(
            trop.contains("trop volumineux") && trop.contains("256,0 Ko"),
            "{trop}"
        );
        assert!(valider_piece("a.exe", "application/x-msdownload", 10).is_err());
        assert!(valider_piece("a", "", 10).is_err(), "type inconnu refusé");
        assert!(
            valider_piece("a.PNG", " IMAGE/PNG ", 10).is_ok(),
            "casse et espaces"
        );
        assert_eq!(taille_lisible(240), "240 o");
        assert_eq!(taille_lisible(1536), "1,5 Ko");
        assert_eq!(taille_lisible(2 * 1024 * 1024), "2,0 Mo");
    }

    #[test]
    fn nom_de_fichier_sans_chemin() {
        assert_eq!(nom_fichier_sur("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(nom_fichier_sur("C:\\dossier\\a.pdf"), "C__dossier_a.pdf");
        assert_eq!(nom_fichier_sur(""), "piece-jointe");
        assert_eq!(nom_fichier_sur("..."), "piece-jointe");
        assert_eq!(nom_fichier_sur("a\u{0}b\n.txt"), "a_b_.txt");
        assert_eq!(nom_fichier_sur(&"x".repeat(300)).chars().count(), 120);
    }

    #[test]
    fn arret_sur_401_403_429() {
        assert!(arret_definitif(&ApiError::NonConnecte));
        assert!(arret_definitif(&ApiError::Interdit("x".into())));
        assert!(arret_definitif(&ApiError::TropDeRequetes));
        assert!(!arret_definitif(&ApiError::Invalide("x".into())));
        assert!(!arret_definitif(&ApiError::Reseau("x".into())));
        assert!(!arret_definitif(&ApiError::Serveur(500, "x".into())));
    }

    #[test]
    fn phrases_apres_envoi() {
        assert_eq!(
            message_apres_envoi(&IssuePiece::SansPiece),
            "Message envoyé."
        );
        assert!(message_apres_envoi(&IssuePiece::Jointe).contains("pièce jointe"));
        let m = message_apres_envoi(&IssuePiece::Echec("trop gros".into()));
        assert!(m.starts_with("Message envoyé, mais") && m.ends_with("trop gros"));
        assert_eq!(
            libelle_eleve("POTTER", "Harry", Some("4B")),
            "POTTER Harry (4B)"
        );
        assert_eq!(libelle_eleve("POTTER", "Harry", Some(" ")), "POTTER Harry");
    }
}
