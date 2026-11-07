//! Logique d'écran des finances (S2-9), sans Leptos ni réseau : affichage des
//! montants (centimes entiers), saisie, libellés, sélection pour l'émission,
//! lien de téléchargement des fichiers remis par le serveur.

use ent_web_api::types_finances::FactureResumeDto;
use ent_web_ui::TonEtat;
use uuid::Uuid;

/// `123456` → `1 234,56 €` (espace insécable fine pour les milliers).
pub fn euros(centimes: i64) -> String {
    let signe = if centimes < 0 { "-" } else { "" };
    let abs = centimes.unsigned_abs();
    let entiers = (abs / 100).to_string();
    let mut groupes = String::new();
    for (i, c) in entiers.chars().enumerate() {
        if i > 0 && (entiers.len() - i).is_multiple_of(3) {
            groupes.push('\u{202F}');
        }
        groupes.push(c);
    }
    format!("{signe}{groupes},{:02}\u{a0}€", abs % 100)
}

/// Saisie d'un montant en euros (`12`, `12,5`, `1 200,00`, `12.50`) → centimes.
pub fn centimes_depuis_saisie(s: &str) -> Result<i64, String> {
    let nettoye: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '€' && *c != '\u{202F}')
        .map(|c| if c == '.' { ',' } else { c })
        .collect();
    if nettoye.is_empty() {
        return Err("Montant obligatoire.".into());
    }
    let (e, d) = match nettoye.split_once(',') {
        Some((e, d)) => (e, d),
        None => (nettoye.as_str(), ""),
    };
    if e.is_empty()
        || !e.chars().all(|c| c.is_ascii_digit())
        || d.len() > 2
        || !d.chars().all(|c| c.is_ascii_digit())
    {
        return Err("Montant invalide : chiffres et au plus deux décimales.".into());
    }
    if e.len() > 9 {
        return Err("Montant trop élevé.".into());
    }
    let entiers: i64 = e.parse().map_err(|_| "Montant invalide.".to_string())?;
    let dec: i64 = match d.len() {
        0 => 0,
        1 => d.parse::<i64>().unwrap_or(0) * 10,
        _ => d.parse().unwrap_or(0),
    };
    Ok(entiers * 100 + dec)
}

/// Année scolaire « AAAA-AAAA » (deux années consécutives).
pub fn annee_valide(s: &str) -> bool {
    let Some((a, b)) = s.trim().split_once('-') else {
        return false;
    };
    match (a.parse::<i32>(), b.parse::<i32>()) {
        (Ok(a), Ok(b)) => a.to_string().len() == 4 && b == a + 1,
        _ => false,
    }
}

pub fn libelle_statut(statut: &str, type_piece: &str) -> &'static str {
    match (type_piece, statut) {
        ("avoir", "brouillon") => "Avoir en préparation",
        ("avoir", "emise") => "Avoir émis",
        ("avoir", "soldee") => "Avoir imputé",
        (_, "brouillon") => "Brouillon",
        (_, "emise") => "Émise",
        (_, "soldee") => "Soldée",
        _ => "Inconnu",
    }
}

pub fn ton_statut(statut: &str) -> TonEtat {
    match statut {
        "brouillon" => TonEtat::Info,
        "emise" => TonEtat::Avertissement,
        "soldee" => TonEtat::Ok,
        _ => TonEtat::Neutre,
    }
}

pub fn libelle_echeance(etat: &str) -> &'static str {
    match etat {
        "payee" => "Payée",
        "en_retard" => "En retard",
        _ => "À venir",
    }
}

pub fn ton_echeance(etat: &str) -> TonEtat {
    match etat {
        "payee" => TonEtat::Ok,
        "en_retard" => TonEtat::Mal,
        _ => TonEtat::Neutre,
    }
}

pub fn libelle_prelevement(statut: &str) -> &'static str {
    match statut {
        "prevu" => "Prévu (pré-notifié)",
        "exporte" => "Transmis à la banque",
        "execute" => "Encaissé",
        "rejete" => "Rejeté",
        "annule" => "Annulé",
        _ => "Inconnu",
    }
}

pub fn libelle_mode(mode: &str) -> &'static str {
    match mode {
        "manuel" => "Paiement manuel (chèque, virement, espèces)",
        _ => "Prélèvement automatique",
    }
}

pub fn libelle_suite(suite: &str) -> &'static str {
    match suite {
        "manuel" => "Retour au paiement manuel",
        _ => "Re-présentée au prochain prélèvement",
    }
}

pub fn libelle_relance(statut: &str) -> &'static str {
    match statut {
        "proposee" => "À confirmer",
        "envoyee" => "Envoyée",
        "annulee" => "Annulée",
        _ => "Inconnu",
    }
}

/// Codes de rejet proposés à la saisie (le serveur reste juge de la liste).
pub const CODES_REJET: [(&str, &str); 8] = [
    ("AM04", "AM04 — provision insuffisante"),
    ("MS02", "MS02 — refus du débiteur"),
    ("MD06", "MD06 — contestation (remboursement)"),
    ("AC04", "AC04 — compte clôturé"),
    ("AC01", "AC01 — numéro de compte incorrect"),
    ("AC06", "AC06 — compte bloqué"),
    ("MD01", "MD01 — absence de mandat"),
    ("MS03", "MS03 — raison non communiquée"),
];

pub const MOYENS: [(&str, &str); 5] = [
    ("cheque", "Chèque"),
    ("virement", "Virement"),
    ("especes", "Espèces"),
    ("cb", "Carte (sur place)"),
    ("autre", "Autre"),
];

/// Pièces cochées réellement émettables (brouillons seulement).
pub fn emettables(factures: &[FactureResumeDto], cochees: &[Uuid]) -> Vec<Uuid> {
    factures
        .iter()
        .filter(|f| f.statut == "brouillon" && cochees.contains(&f.id))
        .map(|f| f.id)
        .collect()
}

/// Total des pièces sélectionnées (factures positives, avoirs négatifs).
pub fn total_selection(factures: &[FactureResumeDto], ids: &[Uuid]) -> i64 {
    factures
        .iter()
        .filter(|f| ids.contains(&f.id))
        .map(|f| f.total_centimes)
        .sum()
}

/// Adresse `data:` pour enregistrer un fichier reçu (pain.008, CSV, écritures) :
/// encodage pour cent de tout ce qui n'est pas alphanumérique ou `-_.~`.
pub fn lien_telechargement(contenu: &str, type_mime: &str) -> String {
    let mut out = format!("data:{type_mime};charset=utf-8,");
    for b in contenu.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            autre => out.push_str(&format!("%{autre:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facture(statut: &str, total: i64) -> FactureResumeDto {
        FactureResumeDto {
            id: Uuid::from_u128(total as u128 + statut.len() as u128),
            numero: "F".into(),
            type_piece: "facture".into(),
            origine: None,
            statut: statut.into(),
            foyer_id: Uuid::nil(),
            foyer: "Foyer".into(),
            libelle: "L".into(),
            annee_scolaire: None,
            total_centimes: total,
            regle_centimes: 0,
            reste_centimes: 0,
            facture_origine_id: None,
            created_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
            emise_at: None,
        }
    }

    #[test]
    fn affichage_des_montants() {
        assert_eq!(euros(0), "0,00\u{a0}€");
        assert_eq!(euros(5), "0,05\u{a0}€");
        assert_eq!(euros(123_456), "1\u{202F}234,56\u{a0}€");
        assert_eq!(euros(-100_000_001), "-1\u{202F}000\u{202F}000,01\u{a0}€");
    }

    #[test]
    fn saisie_des_montants_en_centimes() {
        assert_eq!(centimes_depuis_saisie("12"), Ok(1_200));
        assert_eq!(centimes_depuis_saisie("12,5"), Ok(1_250));
        assert_eq!(centimes_depuis_saisie("12.05"), Ok(1_205));
        assert_eq!(centimes_depuis_saisie(" 1 200,00 € "), Ok(120_000));
        assert!(centimes_depuis_saisie("").is_err());
        assert!(
            centimes_depuis_saisie("12,345").is_err(),
            "trois décimales refusées"
        );
        assert!(centimes_depuis_saisie("-3").is_err());
        assert!(centimes_depuis_saisie("1e3").is_err());
        assert!(centimes_depuis_saisie("9999999999").is_err());
    }

    #[test]
    fn annees_scolaires() {
        assert!(annee_valide("2030-2031"));
        assert!(!annee_valide("2030-2032"));
        assert!(!annee_valide("30-31"));
        assert!(!annee_valide("2030"));
    }

    #[test]
    fn selection_emettable_et_total() {
        let f = vec![
            facture("brouillon", 100),
            facture("emise", 200),
            facture("brouillon", -30),
        ];
        let toutes: Vec<Uuid> = f.iter().map(|x| x.id).collect();
        let e = emettables(&f, &toutes);
        assert_eq!(e.len(), 2, "les pièces déjà émises sont ignorées");
        assert_eq!(total_selection(&f, &e), 70);
    }

    #[test]
    fn lien_data_encode() {
        let l = lien_telechargement("<a>é &", "application/xml");
        assert_eq!(l, "data:application/xml;charset=utf-8,%3Ca%3E%C3%A9%20%26");
        assert!(!l.contains('<'));
    }

    #[test]
    fn libelles() {
        assert_eq!(libelle_statut("emise", "avoir"), "Avoir émis");
        assert_eq!(libelle_statut("soldee", "facture"), "Soldée");
        assert_eq!(libelle_prelevement("exporte"), "Transmis à la banque");
        assert_eq!(libelle_suite("manuel"), "Retour au paiement manuel");
        assert_eq!(libelle_relance("proposee"), "À confirmer");
        assert_eq!(libelle_echeance("en_retard"), "En retard");
    }
}
