//! Socle 42, lot V0c : règles **pures** des prestataires, devis, commandes et du courrier entrant
//! (`docs/socles/42_SORTIES_VOYAGES.md` §13 ; NX-41 à NX-50, NX-70 à NX-74).
//!
//! Rien ici ne touche la base : adresses et SIREN, domaines voisins (hameçonnage), référence de
//! correspondance non devinable, décision de rattachement d'un courrier, lecture **prudente** du
//! montant d'un devis (proposée, jamais « le plus grand nombre trouvé »), contenus des PDF et des
//! messages. Les messages sont du **texte brut** : aucun HTML, donc rien à injecter (corrige NJ-11).

use uuid::Uuid;

use crate::voyages_pdf::Bloc;

pub const CATEGORIES: &[&str] = &["transporteur", "hebergeur", "agence", "lieu", "autre"];

/// Adresse de courrier : minuscules, sans espace, une seule arobase, un point dans le domaine, aucun
/// séparateur de liste ni chevron (pas de destinataires multiples glissés dans un champ).
pub fn adresse_valide(brut: &str) -> Option<(String, String)> {
    let e = brut.trim().to_lowercase();
    if e.len() < 6 || e.len() > 254 {
        return None;
    }
    if e.chars()
        .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | ',' | ';' | '"' | '\\'))
    {
        return None;
    }
    let (local, domaine) = e.split_once('@')?;
    if local.is_empty() || domaine.contains('@') || !domaine.contains('.') {
        return None;
    }
    if domaine.starts_with('.') || domaine.ends_with('.') || domaine.contains("..") {
        return None;
    }
    if !e.is_ascii() {
        return None;
    }
    Some((e.clone(), domaine.to_string()))
}

/// SIREN : 9 chiffres, clé de Luhn.
pub fn siren_valide(s: &str) -> bool {
    let c: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if c.len() != 9 || s.chars().count() != 9 {
        return false;
    }
    let somme: u32 = c
        .iter()
        .rev()
        .enumerate()
        .map(|(i, d)| {
            if i % 2 == 1 {
                let x = d * 2;
                if x > 9 {
                    x - 9
                } else {
                    x
                }
            } else {
                *d
            }
        })
        .sum();
    somme % 10 == 0
}

fn sans_confusion(d: &str) -> String {
    d.replace("rn", "m")
        .chars()
        .map(|c| match c {
            '0' => 'o',
            '1' | 'i' => 'l',
            '5' => 's',
            c => c,
        })
        .collect()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i];
        for j in 1..=b.len() {
            let cout = usize::from(a[i - 1] != b[j - 1]);
            cur.push((prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cout));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Un domaine **différent mais voisin** d'un domaine connu (une lettre changée, ajoutée, retirée,
/// ou une confusion `rn`/`m`, `0`/`o`, `1`/`l`) : alerte d'hameçonnage à l'écran de tri (§13.3).
pub fn domaine_voisin(inconnu: &str, connu: &str) -> bool {
    let (a, b) = (inconnu.to_lowercase(), connu.to_lowercase());
    if a == b || a.len() < 5 || b.len() < 5 {
        return false;
    }
    sans_confusion(&a) == sans_confusion(&b) || levenshtein(&a, &b) <= 2
}

/// Référence de correspondance d'une sortie, placée dans l'objet des messages : la référence lisible
/// de la sortie et une **somme de contrôle** liée à l'établissement et à l'identifiant interne. Ce
/// n'est pas un secret : c'est un identifiant de correspondance qu'on ne peut pas deviner d'une
/// référence voisine.
pub fn reference_courrier(reference_sortie: &str, etab: Uuid, sortie: Uuid) -> String {
    let somme = ent_object::sha256_hex(format!("{etab}|{sortie}|{reference_sortie}").as_bytes());
    format!("{reference_sortie}-{}", somme[..4].to_uppercase())
}

/// Vrai si le texte contient la référence **exacte** (somme de contrôle comprise).
pub fn contient_reference(texte: &str, reference: &str) -> bool {
    texte.to_uppercase().contains(&reference.to_uppercase())
}

/// Références `VOY-AAAA-XXXXXXXX[-SSSS]` trouvées dans un texte : (base lisible, somme de contrôle).
/// La somme n'est jamais déduite : un texte sans elle ne rattache pas (§13.3).
pub fn references_candidates(texte: &str) -> Vec<(String, Option<String>)> {
    let t = texte.to_uppercase();
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    let mut reste = t.as_str();
    while let Some(p) = reste.find("VOY-") {
        let suite = &reste[p..];
        let jeton: String = suite
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '-')
            .take(40)
            .collect();
        let parts: Vec<&str> = jeton.split('-').collect();
        if parts.len() >= 3
            && parts[1].len() == 4
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].len() == 8
            && parts[2].chars().all(|c| c.is_ascii_hexdigit())
        {
            let base = format!("VOY-{}-{}", parts[1], parts[2]);
            let somme = parts
                .get(3)
                .filter(|x| x.len() == 4 && x.chars().all(|c| c.is_ascii_hexdigit()))
                .map(|x| x.to_string());
            if !out.iter().any(|(b, s)| *b == base && *s == somme) {
                out.push((base, somme));
            }
        }
        reste = &suite[4..];
    }
    out
}

/// Résultat du rattachement d'un courrier entrant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rattachement {
    /// Référence exacte **et** expéditeur déjà sollicité pour cette sortie.
    Auto { sortie: Uuid, prestataire: Uuid },
    ATrier { raison: &'static str },
}

/// §13.3 : seules (1) la référence exacte et (2) l'expéditeur du prestataire à qui une demande a été
/// envoyée pour **cette** sortie, **réunies**, donnent un rattachement automatique. Tout le reste va
/// à la file de tri ; jamais de devinette.
pub fn decider_rattachement(
    sortie_par_reference: Option<Uuid>,
    prestataire_expediteur: Option<Uuid>,
    demande_envoyee_pour_cette_sortie: bool,
) -> Rattachement {
    match (sortie_par_reference, prestataire_expediteur) {
        (Some(s), Some(p)) if demande_envoyee_pour_cette_sortie => {
            Rattachement::Auto { sortie: s, prestataire: p }
        }
        (Some(_), Some(_)) => Rattachement::ATrier { raison: "reference_seule" },
        (Some(_), None) => Rattachement::ATrier { raison: "expediteur_inconnu" },
        (None, Some(_)) => Rattachement::ATrier { raison: "expediteur_seul" },
        (None, None) => Rattachement::ATrier { raison: "ni_reference_ni_expediteur" },
    }
}

/// Jetons numériques d'une ligne : chiffres, espaces (y compris insécables), points et virgules.
fn jetons_numeriques(ligne: &str) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    let oct: Vec<(usize, char)> = ligne.char_indices().collect();
    let mut i = 0;
    while i < oct.len() {
        if oct[i].1.is_ascii_digit() {
            let debut = oct[i].0;
            let mut j = i;
            while j < oct.len()
                && (oct[j].1.is_ascii_digit()
                    || matches!(oct[j].1, ' ' | '\u{a0}' | '\u{202f}' | '.' | ','))
            {
                j += 1;
            }
            // Un séparateur final n'appartient pas au nombre.
            let mut fin_idx = j;
            while fin_idx > i && !oct[fin_idx - 1].1.is_ascii_digit() {
                fin_idx -= 1;
            }
            let fin = if fin_idx < oct.len() { oct[fin_idx].0 } else { ligne.len() };
            out.push((debut, fin, ligne[debut..fin].to_string()));
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

fn centimes_du_jeton(j: &str) -> Option<i64> {
    let net: String = j.chars().filter(|c| !matches!(c, ' ' | '\u{a0}' | '\u{202f}')).collect();
    let (ent, dec) = if let Some((e, d)) = net.rsplit_once(',') {
        (e.replace('.', ""), d.to_string())
    } else if let Some((e, d)) = net.rsplit_once('.') {
        if d.len() == 3 {
            (net.replace('.', ""), String::new())
        } else {
            (e.replace('.', ""), d.to_string())
        }
    } else {
        (net.clone(), String::new())
    };
    if ent.is_empty() || !ent.chars().all(|c| c.is_ascii_digit()) || dec.len() > 2 {
        return None;
    }
    if !dec.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let euros: i64 = ent.parse().ok()?;
    let cts: i64 = match dec.len() {
        0 => 0,
        1 => dec.parse::<i64>().ok()? * 10,
        _ => dec.parse().ok()?,
    };
    let total = euros.checked_mul(100)?.checked_add(cts)?;
    (total <= 100_000_000_000).then_some(total)
}

/// Montant TTC **proposé** par la lecture du texte d'un devis (NX-70). Ne retient que le nombre qui
/// suit un libellé « TTC » ; si le texte en donne plusieurs **différents**, ou aucun, rien n'est
/// proposé : un humain saisit et confirme. Jamais « le plus grand nombre trouvé ».
pub fn montant_ttc_propose(texte: &str) -> Option<i64> {
    let mut trouves: Vec<i64> = Vec::new();
    for ligne in texte.lines() {
        let bas = ligne.to_lowercase();
        let mut depart = 0;
        while let Some(p) = bas[depart..].find("ttc") {
            let pos = depart + p;
            let apres = pos + 3;
            let jetons = jetons_numeriques(&ligne[apres.min(ligne.len())..]);
            let jeton = jetons.first().map(|(_, _, j)| j.clone()).or_else(|| {
                jetons_numeriques(&ligne[..pos.min(ligne.len())])
                    .last()
                    .map(|(_, _, j)| j.clone())
            });
            if let Some(c) = jeton.and_then(|j| centimes_du_jeton(&j)) {
                trouves.push(c);
            }
            depart = apres;
        }
    }
    trouves.sort_unstable();
    trouves.dedup();
    if trouves.len() == 1 {
        trouves.pop()
    } else {
        None
    }
}

/// Texte d'une ligne sans saut ni caractère de contrôle (objets de message, noms de pièce).
pub fn ligne_propre(s: &str, max: usize) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

/// Poste du budget d'après la catégorie du prestataire (NX-51).
pub fn poste_budget(categorie: &str) -> &'static str {
    match categorie {
        "transporteur" => "transport",
        "hebergeur" => "hebergement",
        "lieu" => "activites",
        _ => "divers",
    }
}

fn euros(c: i64) -> String {
    let e = (c / 100).to_string();
    let mut g = String::new();
    for (i, ch) in e.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            g.push('\u{202f}');
        }
        g.push(ch);
    }
    format!("{} ,{:02} €", g.chars().rev().collect::<String>(), c % 100).replace(" ,", ",")
}

/// Dates déjà mises en forme (heure de Paris) par la base : ce module reste pur.
/// Faits d'une demande de devis (§13.2) : classes, dates, point de rendez-vous, **effectifs**, car
/// sur place, consignes **saisies par l'organisateur** (jamais un texte venu d'un e-mail entrant).
pub struct FaitsDemande<'a> {
    pub etablissement: &'a str,
    pub reference_courrier: &'a str,
    pub titre: &'a str,
    pub destination: &'a str,
    pub debut: String,
    pub fin: String,
    pub point_rendez_vous: Option<&'a str>,
    pub effectif_eleves: Option<i32>,
    pub effectif_adultes: Option<i32>,
    pub car_sur_place: bool,
    pub consignes: Option<&'a str>,
    pub pied_de_page: Option<&'a str>,
}

fn nb(n: Option<i32>) -> String {
    n.map_or("à préciser".to_string(), |v| v.to_string())
}

pub fn blocs_demande_devis(f: &FaitsDemande<'_>) -> Vec<Bloc> {
    let mut b = vec![
        Bloc::Titre("Demande de devis".into()),
        Bloc::Paragraphe(format!("{} — référence {}", f.etablissement, f.reference_courrier)),
        Bloc::Espace,
        Bloc::Ligne { gauche: "Sortie".into(), droite: f.titre.into(), gras: true },
        Bloc::Ligne { gauche: "Destination".into(), droite: f.destination.into(), gras: false },
        Bloc::Ligne { gauche: "Départ".into(), droite: f.debut.clone(), gras: false },
        Bloc::Ligne { gauche: "Retour".into(), droite: f.fin.clone(), gras: false },
        Bloc::Ligne {
            gauche: "Point de rendez-vous".into(),
            droite: f.point_rendez_vous.unwrap_or("à préciser").into(),
            gras: false,
        },
        Bloc::Ligne { gauche: "Élèves".into(), droite: nb(f.effectif_eleves), gras: false },
        Bloc::Ligne { gauche: "Adultes".into(), droite: nb(f.effectif_adultes), gras: false },
        Bloc::Ligne {
            gauche: "Car sur place pendant le séjour".into(),
            droite: if f.car_sur_place { "oui" } else { "non" }.into(),
            gras: false,
        },
    ];
    if let Some(c) = f.consignes.filter(|c| !c.trim().is_empty()) {
        b.push(Bloc::Espace);
        b.push(Bloc::SousTitre("Consignes de l'organisateur".into()));
        b.push(Bloc::Paragraphe(c.to_string()));
    }
    b.push(Bloc::Espace);
    b.push(Bloc::Paragraphe(
        "Merci de répondre à ce message en conservant la référence ci-dessus dans l'objet. Ce document ne contient ni nom d'élève ni coordonnées de familles.".into(),
    ));
    if let Some(p) = f.pied_de_page.filter(|p| !p.trim().is_empty()) {
        b.push(Bloc::Paragraphe(p.to_string()));
    }
    b
}

/// Faits d'un bon de commande ou d'un avenant.
pub struct FaitsCommande<'a> {
    pub etablissement: &'a str,
    pub reference_courrier: &'a str,
    pub numero: i32,
    /// `Some(n)` pour l'avenant n° n (« annule et remplace »).
    pub avenant: Option<i32>,
    pub motif_avenant: Option<&'a str>,
    pub prestataire: &'a str,
    pub reference_devis: Option<&'a str>,
    pub montant_centimes: i64,
    pub effectif_eleves: i32,
    pub effectif_adultes: i32,
    pub debut: String,
    pub fin: String,
    pub empreinte_devis: &'a str,
    pub signe_le: String,
    pub pied_de_page: Option<&'a str>,
}

pub fn blocs_commande(f: &FaitsCommande<'_>) -> Vec<Bloc> {
    let titre = match f.avenant {
        Some(n) => format!("Avenant n° {n} au bon de commande n° {} — annule et remplace", f.numero),
        None => format!("Bon de commande n° {}", f.numero),
    };
    let mut b = vec![
        Bloc::Titre(titre),
        Bloc::Paragraphe(format!("{} — référence {}", f.etablissement, f.reference_courrier)),
        Bloc::Espace,
        Bloc::Ligne { gauche: "Prestataire".into(), droite: f.prestataire.into(), gras: true },
        Bloc::Ligne {
            gauche: "Devis".into(),
            droite: f.reference_devis.unwrap_or("sans référence").into(),
            gras: false,
        },
        Bloc::Ligne { gauche: "Montant TTC".into(), droite: euros(f.montant_centimes), gras: true },
        Bloc::Ligne { gauche: "Élèves".into(), droite: f.effectif_eleves.to_string(), gras: false },
        Bloc::Ligne { gauche: "Adultes".into(), droite: f.effectif_adultes.to_string(), gras: false },
        Bloc::Ligne { gauche: "Du".into(), droite: f.debut.clone(), gras: false },
        Bloc::Ligne { gauche: "Au".into(), droite: f.fin.clone(), gras: false },
    ];
    if let Some(m) = f.motif_avenant {
        b.push(Bloc::Espace);
        b.push(Bloc::SousTitre("Motif de l'avenant".into()));
        b.push(Bloc::Paragraphe(m.to_string()));
    }
    b.push(Bloc::Espace);
    b.push(Bloc::Paragraphe(format!(
        "Devis accepté et signé par le chef d'établissement le {} (empreinte du devis {}).",
        f.signe_le,
        &f.empreinte_devis[..16.min(f.empreinte_devis.len())]
    )));
    if f.avenant.is_some() {
        b.push(Bloc::Paragraphe(
            "La commande est à confirmer de nouveau par le prestataire. La version précédente reste au dossier.".into(),
        ));
    }
    if let Some(p) = f.pied_de_page.filter(|p| !p.trim().is_empty()) {
        b.push(Bloc::Paragraphe(p.to_string()));
    }
    b
}

/// Objet et corps (texte brut) d'un envoi : texte **fixe** par type, seules la référence et le nom de
/// l'établissement sont insérés.
pub fn message_envoi(type_: &str, etablissement: &str, reference: &str, numero: Option<i32>) -> (String, String) {
    let etab = ligne_propre(etablissement, 120);
    match type_ {
        "relance" => (
            format!("[{reference}] Relance — demande de devis"),
            format!("Bonjour,\n\nNous n'avons pas encore reçu votre devis pour la demande jointe de {etab}. Merci de nous le faire parvenir en répondant à ce message, en conservant la référence dans l'objet.\n\nCordialement,\n{etab}"),
        ),
        "bon_commande" => (
            format!("[{reference}] Bon de commande n° {}", numero.unwrap_or(1)),
            format!("Bonjour,\n\nVeuillez trouver ci-joint le bon de commande de {etab}, établi sur la base de votre devis. Merci d'en accuser réception en répondant à ce message, en indiquant le nom et le numéro de téléphone du conducteur.\n\nCordialement,\n{etab}"),
        ),
        "avenant" => (
            format!("[{reference}] Avenant n° {} — annule et remplace", numero.unwrap_or(1)),
            format!("Bonjour,\n\nVeuillez trouver ci-joint un avenant à la commande de {etab}. Il annule et remplace la version précédente. Merci de le confirmer en répondant à ce message.\n\nCordialement,\n{etab}"),
        ),
        _ => (
            format!("[{reference}] Demande de devis"),
            format!("Bonjour,\n\n{etab} sollicite un devis pour la sortie décrite dans le document joint. Merci de répondre à ce message en conservant la référence dans l'objet.\n\nCordialement,\n{etab}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresse_refuse_les_destinataires_multiples_et_les_injections() {
        assert!(adresse_valide(" Contact@Cars-Dupont.FR ").is_some());
        assert_eq!(adresse_valide("a@b.fr").unwrap().1, "b.fr");
        for mauvais in [
            "a@b.fr, c@d.fr",
            "a@b.fr;c@d.fr",
            "Nom <a@b.fr>",
            "a@b.fr\nBcc: x@y.fr",
            "a@@b.fr",
            "a@b",
            "a b@c.fr",
            "a@b..fr",
            "é@b.fr",
            "",
        ] {
            assert!(adresse_valide(mauvais).is_none(), "{mauvais:?}");
        }
    }

    #[test]
    fn siren_cle_de_luhn() {
        assert!(siren_valide("732829320"));
        assert!(!siren_valide("732829321"));
        assert!(!siren_valide("73282932"));
        assert!(!siren_valide("73282932a"));
    }

    #[test]
    fn domaines_voisins() {
        assert!(domaine_voisin("cars-dupond.fr", "cars-dupont.fr"));
        assert!(domaine_voisin("cars-dupnt.fr", "cars-dupont.fr"));
        assert!(domaine_voisin("rnonbus.fr", "monbus.fr"));
        assert!(!domaine_voisin("cars-dupont.fr", "cars-dupont.fr"));
        assert!(!domaine_voisin("autre-societe.com", "cars-dupont.fr"));
    }

    #[test]
    fn reference_non_devinable_et_exacte() {
        let (e, s) = (Uuid::new_v4(), Uuid::new_v4());
        let r = reference_courrier("VOY-2027-ABCDEF12", e, s);
        assert!(r.starts_with("VOY-2027-ABCDEF12-"));
        assert_eq!(r, reference_courrier("VOY-2027-ABCDEF12", e, s));
        assert_ne!(r, reference_courrier("VOY-2027-ABCDEF12", Uuid::new_v4(), s));
        assert!(contient_reference(&format!("Re: [{}] devis", r.to_lowercase()), &r));
        // La référence lisible seule, sans la somme de contrôle, ne suffit pas.
        assert!(!contient_reference("Re: VOY-2027-ABCDEF12 devis", &r));
    }

    #[test]
    fn references_extraites_du_texte() {
        let r = references_candidates("Re: [voy-2027-abcdef12-9f3a] devis ; ancien VOY-2026-00112233");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], ("VOY-2027-ABCDEF12".to_string(), Some("9F3A".to_string())));
        assert_eq!(r[1], ("VOY-2026-00112233".to_string(), None));
        assert!(references_candidates("VOY-27-123 rien").is_empty());
    }

    #[test]
    fn rattachement_automatique_seulement_si_les_deux_sont_reunis() {
        let (s, p) = (Uuid::new_v4(), Uuid::new_v4());
        assert_eq!(decider_rattachement(Some(s), Some(p), true), Rattachement::Auto { sortie: s, prestataire: p });
        assert_eq!(decider_rattachement(Some(s), Some(p), false), Rattachement::ATrier { raison: "reference_seule" });
        assert_eq!(decider_rattachement(Some(s), None, false), Rattachement::ATrier { raison: "expediteur_inconnu" });
        assert_eq!(decider_rattachement(None, Some(p), true), Rattachement::ATrier { raison: "expediteur_seul" });
        assert_eq!(decider_rattachement(None, None, false), Rattachement::ATrier { raison: "ni_reference_ni_expediteur" });
    }

    #[test]
    fn montant_propose_prudent() {
        assert_eq!(montant_ttc_propose("Total HT : 1 000,00 €\nTotal TTC : 1 200,00 €"), Some(120_000));
        assert_eq!(montant_ttc_propose("Prix TTC 1.350,50 EUR"), Some(135_050));
        assert_eq!(montant_ttc_propose("Montant 1350.50 TTC"), Some(135_050));
        // Deux montants TTC différents : rien n'est proposé.
        assert_eq!(montant_ttc_propose("Option A TTC 900\nOption B TTC 1 100"), None);
        // Le même montant répété reste univoque.
        assert_eq!(montant_ttc_propose("TTC 900,00\nNet à payer TTC 900,00"), Some(90_000));
        // Sans libellé TTC : jamais « le plus grand nombre ».
        assert_eq!(montant_ttc_propose("Km 4 500\nPrix 1 200"), None);
        assert_eq!(montant_ttc_propose(""), None);
    }

    #[test]
    fn messages_en_texte_brut_sans_injection() {
        let (o, c) = message_envoi("bon_commande", "Lycée\r\nBcc: x@y.fr", "VOY-1-AAAA", Some(2));
        assert!(!o.contains('\n') && !o.contains('\r'));
        assert!(!c.contains("Bcc: x@y.fr\n"), "{c}");
        assert!(o.contains("VOY-1-AAAA"));
    }

    #[test]
    fn pdf_de_demande_sans_donnee_de_famille() {
        let f = FaitsDemande {
            etablissement: "Lycée Test",
            reference_courrier: "VOY-2027-AAAAAAAA-1234",
            titre: "Sortie",
            destination: "Paris",
            debut: "01/01/2027 à 08:00".into(),
            fin: "01/01/2027 à 18:00".into(),
            point_rendez_vous: None,
            effectif_eleves: Some(30),
            effectif_adultes: Some(3),
            car_sur_place: true,
            consignes: Some("Arrêt devant l'entrée B"),
            pied_de_page: None,
        };
        let b = blocs_demande_devis(&f);
        assert!(b.iter().any(|x| matches!(x, Bloc::Ligne { gauche, droite, .. } if gauche == "Élèves" && droite == "30")));
        assert!(b.iter().any(|x| matches!(x, Bloc::Paragraphe(t) if t.contains("Arrêt devant"))));
    }
}
