//! Logique d'écran pure des écrans « Élèves » (socle 2, lots E9a et E9b) : sans Leptos ni réseau,
//! testée en natif. **Aucune règle métier** : droits, masquage, protégés et cloisonnement sont
//! décidés par le serveur ; ce module ne fait que présenter ce qu'il renvoie (libellés, tons,
//! pagination, destination des tuiles, lecture d'une frise d'identité).

use chrono::NaiveDate;
use ent_types::eleve_synthese::{PastilleDto, TuileDto};
use ent_types::identite::IdentiteVersionDto;
use ent_web_ui::TonEtat;
use uuid::Uuid;

use crate::noyau::temps::date_longue;

/// Nombre d'élèves par page de la liste.
pub const PAGE_TAILLE: u32 = 50;

/// Longueur minimale d'une recherche (en dessous, la liste complète est montrée).
pub const RECHERCHE_MIN: usize = 2;

// ───────────────────────────── Désignation et statut ─────────────────────────────

/// « Léa MARTIN » : prénom puis nom en capitales (le nom se lit d'un coup d'œil).
pub fn designation(prenom: &str, nom: &str) -> String {
    format!("{} {}", prenom.trim(), nom.trim().to_uppercase()).trim().to_string()
}

/// Libellé du statut d'un élève (valeurs de `ent_eleve.status`).
pub fn libelle_statut(statut: &str) -> &'static str {
    match statut {
        "actif" => "Scolarisé",
        "preinscrit" => "Préinscrit",
        "sorti" => "Sorti de l'établissement",
        _ => "Statut inconnu",
    }
}

pub fn ton_statut(statut: &str) -> TonEtat {
    match statut {
        "actif" => TonEtat::Ok,
        "preinscrit" => TonEtat::Info,
        _ => TonEtat::Neutre,
    }
}

/// Choix du filtre de statut : (valeur envoyée au serveur, libellé).
pub const STATUTS: [(&str, &str); 4] =
    [("", "Tous les statuts"), ("actif", "Scolarisés"), ("preinscrit", "Préinscrits"), ("sorti", "Sortis")];

// ───────────────────────────── Liste : pagination et messages ─────────────────────────────

/// « 51 à 100 sur 312 élèves » (annoncé aux lecteurs d'écran à chaque changement).
pub fn resume_page(total: i64, decalage: i64, affiches: usize) -> String {
    let affiches = i64::try_from(affiches).unwrap_or(0);
    match (total, affiches) {
        (0, _) | (_, 0) => "Aucun élève".to_string(),
        (1, _) => "1 élève".to_string(),
        (t, a) if decalage == 0 && a >= t => format!("{t} élèves"),
        (t, a) => format!("{} à {} sur {t} élèves", decalage + 1, decalage + a),
    }
}

/// Décalage de la page précédente, s'il y en a une.
pub fn page_precedente(decalage: u32, taille: u32) -> Option<u32> {
    (decalage > 0).then(|| decalage.saturating_sub(taille.max(1)))
}

/// Décalage de la page suivante, s'il y en a une.
pub fn page_suivante(decalage: u32, taille: u32, total: i64) -> Option<u32> {
    let suivant = i64::from(decalage) + i64::from(taille.max(1));
    (suivant < total).then(|| u32::try_from(suivant).unwrap_or(u32::MAX))
}

/// Texte d'explication de la portée de la liste, quand elle n'est pas l'établissement entier.
pub fn message_portee(portee: &str) -> Option<&'static str> {
    (portee != "etablissement")
        .then_some("Vous voyez les élèves de votre périmètre (par exemple vos classes), pas tout l'établissement.")
}

/// Message de la liste vide, selon que des filtres sont posés ou non.
pub fn message_liste_vide(filtres_actifs: bool) -> (&'static str, &'static str) {
    if filtres_actifs {
        (
            "Aucun élève ne correspond",
            "Essayez sans accent, avec moins de lettres, ou retirez un filtre. La recherche tolère les fautes de frappe.",
        )
    } else {
        ("Aucun élève à afficher", "Aucun élève n'est dans votre périmètre pour le moment.")
    }
}

/// Aide affichée quand la recherche atteint son plafond.
pub const MESSAGE_TRONQUE: &str =
    "La recherche a atteint son plafond : seuls les premiers résultats sont comptés. Précisez le nom ou la classe.";

/// Faut-il envoyer la saisie comme recherche ? Une seule lettre ne cherche rien (liste complète).
pub fn recherche_a_envoyer(saisie: &str) -> String {
    let t = saisie.trim();
    if t.chars().count() < RECHERCHE_MIN {
        String::new()
    } else {
        t.chars().take(100).collect()
    }
}

/// L'élément visé par une touche est-il une zone de saisie ? (balise en capitales, comme `tagName`.)
pub fn est_zone_de_saisie(balise: &str, contenu_editable: bool) -> bool {
    contenu_editable || matches!(balise.to_ascii_uppercase().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
}

/// La touche « / » ouvre la recherche, sauf pendant une saisie et avec une touche de contrôle
/// (jamais de raccourci qui vole le caractère qu'on tape).
pub fn ouvre_la_recherche(touche: &str, ctrl: bool, meta: bool, alt: bool, en_saisie: bool) -> bool {
    touche == "/" && !ctrl && !meta && !alt && !en_saisie
}

/// Teinte du chip d'une pastille (classes `ui-pastille--*` du design system).
pub fn teinte_gravite(gravite: &str) -> &'static str {
    match gravite {
        "alerte" => "mal",
        "attention" => "avert",
        _ => "info",
    }
}

/// Ton d'un badge de pastille. Toujours doublé du libellé : la couleur n'est qu'un renfort.
pub fn ton_gravite(gravite: &str) -> TonEtat {
    match gravite {
        "alerte" => TonEtat::Mal,
        "attention" => TonEtat::Avertissement,
        _ => TonEtat::Info,
    }
}

/// Libellé complet d'une pastille pour le lecteur d'écran, valeur comprise (« Interne, régime interne »).
pub fn phrase_pastille(p: &PastilleDto) -> String {
    let mut t = p.libelle.clone();
    if let Some(v) = p.valeur_code.as_deref().filter(|v| !v.is_empty()) {
        t.push_str(&format!(" ({v})"));
    }
    if let Some(d) = p.valeur_date {
        t.push_str(&format!(", jusqu'au {}", date_longue(d)));
    }
    t
}

// ───────────────────────────── Fiche : tuiles de la synthèse ─────────────────────────────

/// Explication d'une tuile masquée : le **pourquoi**, jamais le contenu (« jamais de masquage muet »).
pub fn raison_masquage(raison: Option<&str>) -> &'static str {
    match raison {
        Some("hors_portee") => "Cet élève n'est pas dans votre périmètre pour cette information.",
        Some("reserve_au_personnel") => "Information réservée au personnel habilité.",
        Some("droit_manquant") | None => {
            "Vos droits ne donnent pas accès à cette information. Si vous en avez besoin, demandez-le à la direction."
        }
        Some(_) => "Information non accessible avec vos droits.",
    }
}

/// Section de la fiche vers laquelle une tuile renvoie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Onglet {
    Synthese,
    Identite,
    Historique,
}

impl Onglet {
    pub const TOUS: [Onglet; 3] = [Onglet::Synthese, Onglet::Identite, Onglet::Historique];

    /// Valeur du paramètre d'adresse `?onglet=`.
    pub fn code(self) -> &'static str {
        match self {
            Onglet::Synthese => "synthese",
            Onglet::Identite => "identite",
            Onglet::Historique => "historique",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Onglet::Synthese => "Synthèse",
            Onglet::Identite => "Identité",
            Onglet::Historique => "Historique",
        }
    }

    /// Un paramètre inconnu retombe sur la synthèse (jamais une page vide).
    pub fn depuis_code(code: &str) -> Onglet {
        Onglet::TOUS.into_iter().find(|o| o.code() == code).unwrap_or(Onglet::Synthese)
    }
}

/// Où mène une tuile visible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    /// Une section de cette même fiche.
    Onglet(Onglet),
    /// Un écran existant ailleurs dans l'application.
    Page(String),
    /// L'écran du domaine n'existe pas encore : la tuile informe, sans lien mort.
    Aucune,
}

/// Destination d'une tuile du contrat d'E8, selon ce que l'application sait afficher aujourd'hui.
/// Les liens du serveur (`/web/eleves/{id}/scolarite`…) pointent des écrans futurs : ils ne sont
/// jamais repris tels quels (pas de page introuvable).
pub fn destination_tuile(domaine: &str, eleve: Uuid) -> Destination {
    match domaine {
        "identite" | "identifiants" => Destination::Onglet(Onglet::Identite),
        "sante" => Destination::Page(format!("/web/infirmerie/eleves/{eleve}")),
        _ => Destination::Aucune,
    }
}

/// Libellé du lien d'une tuile.
pub fn libelle_ouverture(domaine: &str) -> &'static str {
    match domaine {
        "identifiants" => "Voir les identifiants",
        "sante" => "Ouvrir à l'infirmerie",
        _ => "Ouvrir",
    }
}

/// Texte d'une tuile visible qui n'a pas (encore) d'écran : ce qu'elle contient, pas un lien mort.
pub fn corps_tuile_sans_ecran(domaine: &str) -> &'static str {
    match domaine {
        "scolarite" => "Le parcours de scolarité (entrées, sorties, classes, régime) arrive prochainement dans cette fiche.",
        "protection" => "Mesure de confidentialité en vigueur. Sa gestion se fait depuis l'écran de la direction.",
        _ => "Cette section n'a pas encore d'écran dans la fiche.",
    }
}

/// Une tuile visible n'a ni pastille ni destination : on le dit au lieu de laisser un cadre vide.
pub fn tuile_vide(t: &TuileDto) -> bool {
    t.pastilles.is_empty() && matches!(destination_tuile(&t.domaine, Uuid::nil()), Destination::Aucune)
}

// ───────────────────────────── Identité et historique ─────────────────────────────

/// Libellé d'un champ d'identité (code du catalogue d'E5).
pub fn libelle_champ(code: &str) -> &'static str {
    match code {
        "nom_naissance" => "Nom de naissance",
        "nom_usage" => "Nom d'usage",
        "prenoms" => "Prénoms",
        "prenom_usage" => "Prénom d'usage",
        "sexe" => "Sexe à l'état civil",
        "date_naissance" => "Date de naissance",
        "naissance_commune" => "Commune de naissance",
        "naissance_pays_cog" => "Pays de naissance",
        "naissance_lieu_etranger" => "Ville de naissance à l'étranger",
        _ => "Champ d'identité",
    }
}

/// Nature d'une version : une correction n'est pas un changement d'état civil (lisible comme telle).
pub fn libelle_nature(nature: &str) -> &'static str {
    match nature {
        "creation" => "Création de la fiche",
        "reprise" => "Reprise de données",
        "import" => "Import d'une source officielle",
        "correction" => "Correction d'une erreur",
        "changement_etat_civil" => "Changement d'état civil",
        "usage" => "Nom ou prénom d'usage",
        "fusion" => "Fusion de fiches",
        _ => "Modification",
    }
}

pub fn ton_nature(nature: &str) -> TonEtat {
    match nature {
        "correction" => TonEtat::Avertissement,
        "changement_etat_civil" => TonEtat::Info,
        _ => TonEtat::Neutre,
    }
}

/// Libellé de l'origine d'une version.
pub fn libelle_source(source: &str) -> &'static str {
    match source {
        "saisie" => "Saisie par l'établissement",
        "siecle" => "SIECLE",
        "onde" => "ONDE",
        "reprise" => "Reprise",
        "famille" => "Demande d'une famille",
        "eleve" => "Demande de l'élève",
        "import" => "Import",
        "fusion" => "Fusion",
        _ => "Autre source",
    }
}

fn texte_ou_tiret(v: &Option<String>) -> String {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty()).map_or_else(|| "—".to_string(), str::to_string)
}

/// Date de naissance telle qu'on la sait : jamais de « 1ᵉʳ janvier » inventé (R-E1).
pub fn date_naissance_affichee(d: Option<NaiveDate>, precision: &str, statut: &str) -> String {
    let Some(d) = d else {
        return match precision {
            "inconnue" => "Date de naissance inconnue".to_string(),
            _ => "—".to_string(),
        };
    };
    let base = match precision {
        "annee" => format!("{}", chrono::Datelike::year(&d)),
        "mois" => format!(
            "{} {}",
            crate::noyau::temps::nom_mois(chrono::Datelike::month(&d)),
            chrono::Datelike::year(&d)
        ),
        _ => date_longue(d),
    };
    match statut {
        "presumee" => format!("{base} (date présumée)"),
        "contestee" => format!("{base} (date contestée)"),
        _ => base,
    }
}

/// Une ligne « champ : valeur » de la carte d'identité.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LigneIdentite {
    pub libelle: &'static str,
    pub valeur: String,
}

/// Valeurs courantes d'une version, dans l'ordre de lecture. Seul ce que la version contient est
/// montré : le serveur a déjà retiré ce que le lecteur n'a pas le droit de voir.
pub fn lignes_identite(v: &IdentiteVersionDto) -> Vec<LigneIdentite> {
    let l = |libelle: &'static str, valeur: String| LigneIdentite { libelle, valeur };
    let mut out = vec![
        l("Nom de naissance", v.nom_naissance.to_uppercase()),
        l("Prénoms", if v.prenoms.is_empty() { "—".into() } else { v.prenoms.join(", ") }),
    ];
    if v.nom_usage.as_deref().is_some_and(|s| !s.trim().is_empty()) {
        out.push(l("Nom d'usage", texte_ou_tiret(&v.nom_usage).to_uppercase()));
    }
    if v.prenom_usage.as_deref().is_some_and(|s| !s.trim().is_empty()) {
        out.push(l("Prénom d'usage", texte_ou_tiret(&v.prenom_usage)));
    }
    out.push(l(
        "Sexe à l'état civil",
        match v.sexe.as_deref() {
            Some("F") => "Féminin".into(),
            Some("M") => "Masculin".into(),
            _ => "—".into(),
        },
    ));
    out.push(l(
        "Date de naissance",
        date_naissance_affichee(v.date_naissance, &v.date_naissance_precision, &v.date_naissance_statut),
    ));
    let lieu = [v.naissance_commune.as_deref(), v.naissance_lieu_etranger.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    out.push(l("Lieu de naissance", if lieu.is_empty() { "—".into() } else { lieu }));
    out
}

/// La version courante : la plus récente dont l'effet est atteint (un changement daté d'un jour futur
/// n'est pas encore l'état civil), sans version qui l'ait remplacée.
pub fn version_courante<'a>(versions: &'a [IdentiteVersionDto], jour: NaiveDate) -> Option<&'a IdentiteVersionDto> {
    versions
        .iter()
        .filter(|v| v.effet_du <= jour && v.remplacee_par.is_none())
        .max_by_key(|v| (v.effet_du, v.enregistre_le))
}

/// Versions de la frise, la plus récente d'abord (effet puis enregistrement).
pub fn frise(versions: &[IdentiteVersionDto]) -> Vec<&IdentiteVersionDto> {
    let mut v: Vec<&IdentiteVersionDto> = versions.iter().collect();
    v.sort_by(|a, b| (b.effet_du, b.enregistre_le).cmp(&(a.effet_du, a.enregistre_le)));
    v
}

/// Champs qui diffèrent entre une version et celle qu'elle corrige ou remplace (par ses valeurs
/// lisibles). Sans version précédente (création), rien n'est « changé » : la ligne dit la nature seule.
pub fn champs_modifies(avant: &IdentiteVersionDto, apres: &IdentiteVersionDto) -> Vec<&'static str> {
    let mut v = Vec::new();
    if avant.nom_naissance != apres.nom_naissance {
        v.push("Nom de naissance");
    }
    if avant.prenoms != apres.prenoms || avant.rang_prenom_usuel != apres.rang_prenom_usuel {
        v.push("Prénoms");
    }
    if avant.nom_usage != apres.nom_usage {
        v.push("Nom d'usage");
    }
    if avant.prenom_usage != apres.prenom_usage {
        v.push("Prénom d'usage");
    }
    if avant.sexe != apres.sexe {
        v.push("Sexe à l'état civil");
    }
    if avant.date_naissance != apres.date_naissance
        || avant.date_naissance_precision != apres.date_naissance_precision
        || avant.date_naissance_statut != apres.date_naissance_statut
    {
        v.push("Date de naissance");
    }
    if avant.naissance_commune != apres.naissance_commune
        || avant.naissance_pays_cog != apres.naissance_pays_cog
        || avant.naissance_lieu_etranger != apres.naissance_lieu_etranger
    {
        v.push("Lieu de naissance");
    }
    v
}

/// Ligne de la frise : ce qui a changé par rapport à la version précédente de la liste (triée).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EtapeFrise {
    pub id: Uuid,
    pub nature: &'static str,
    pub ton: TonEtat,
    pub effet_du: NaiveDate,
    pub source: &'static str,
    /// Champs changés (vide pour une création).
    pub champs: Vec<&'static str>,
    pub motif: Option<String>,
    pub corrigee: bool,
    pub avec_piece: bool,
    pub sensible: bool,
}

/// Frise complète, la plus récente d'abord : chaque étape se compare à la **précédente dans le temps**.
pub fn etapes_frise(versions: &[IdentiteVersionDto]) -> Vec<EtapeFrise> {
    let mut chrono: Vec<&IdentiteVersionDto> = versions.iter().collect();
    chrono.sort_by(|a, b| (a.effet_du, a.enregistre_le).cmp(&(b.effet_du, b.enregistre_le)));
    let mut etapes: Vec<EtapeFrise> = chrono
        .iter()
        .enumerate()
        .map(|(i, v)| EtapeFrise {
            id: v.id,
            nature: libelle_nature(&v.nature),
            ton: ton_nature(&v.nature),
            effet_du: v.effet_du,
            source: libelle_source(&v.source),
            champs: if i == 0 { Vec::new() } else { champs_modifies(chrono[i - 1], v) },
            motif: v.motif.clone().filter(|m| !m.trim().is_empty()),
            corrigee: v.remplacee_par.is_some(),
            avec_piece: v.piece_document_id.is_some(),
            sensible: v.sensible,
        })
        .collect();
    etapes.reverse();
    etapes
}

// ───────────────────────────── Identifiants ─────────────────────────────

pub fn libelle_systeme(systeme: &str) -> &'static str {
    match systeme {
        "INE" => "INE",
        "BADGE" => "Badge",
        "SIECLE_ELEVE" => "Identifiant SIECLE",
        "ONDE" => "Identifiant ONDE",
        _ => "Identifiant",
    }
}

pub fn libelle_statut_identifiant(statut: &str) -> &'static str {
    match statut {
        "declare" => "Déclaré",
        "a_verifier" => "À vérifier",
        "confirme" => "Confirmé",
        "conteste" => "Contesté",
        "revoque" => "Révoqué",
        _ => "Statut inconnu",
    }
}

pub fn ton_statut_identifiant(statut: &str) -> TonEtat {
    match statut {
        "confirme" => TonEtat::Ok,
        "conteste" => TonEtat::Mal,
        "a_verifier" => TonEtat::Avertissement,
        _ => TonEtat::Neutre,
    }
}

// ───────────────────────────── Consultations ─────────────────────────────

/// Libellé neutre d'une consultation (jamais la valeur lue).
pub fn libelle_consultation(action: &str) -> &'static str {
    match action {
        "fiche_protegee" => "Ouverture de la fiche",
        "etat_civil" => "Lecture de l'état civil",
        "identifiants" => "Lecture des identifiants",
        "coordonnees" => "Lecture des coordonnées",
        "sensible" => "Lecture d'une information sensible",
        "pastilles_sensibles" => "Lecture de pastilles sensibles",
        _ => "Consultation",
    }
}

#[cfg(test)]
mod tests;
