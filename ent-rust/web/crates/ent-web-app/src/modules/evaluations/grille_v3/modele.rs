//! Logique d'écran pure de la grille V3 (socle 21, lot V3) : texte d'une case, navigation au clavier, lecture
//! d'un collage ou d'un fichier de tableur, construction des lots à envoyer, application des retours.
//! **Aucun calcul de moyenne** : le serveur les fournit (moteur unique). Aucune dépendance à Leptos ni au réseau.

use ent_types::evaluations_v3::{
    code_statut, formater_milliemes, libelle_statut, CelluleDto, CelluleRetourDto, CelluleSaisieDto,
    ConflitDto, ImportLigneBody,
};
use uuid::Uuid;

// ───────────────────────── Texte d'une case ─────────────────────────

/// Le texte qu'on lit dans une case : « 12,5 », « abs »… (vide : rien de saisi).
pub fn texte_cellule(c: &CelluleDto) -> String {
    match (c.statut.as_deref(), c.valeur_m) {
        (Some("note"), Some(m)) => formater_milliemes(m),
        (Some(s), _) => code_statut(s).to_string(),
        _ => String::new(),
    }
}

/// Texte lu par un lecteur d'écran pour une case : « Léa Martin, DS 1 sur 20, 12,5 ».
pub fn phrase_cellule(eleve: &str, titre: &str, bareme_m: i64, statut: Option<&str>, valeur_m: Option<i64>) -> String {
    let contenu = match (statut, valeur_m) {
        (Some("note"), Some(m)) => format!("{} sur {}", formater_milliemes(m), formater_milliemes(bareme_m)),
        (Some(s), _) => libelle_statut(s).to_string(),
        _ => "pas de résultat".to_string(),
    };
    format!("{eleve}, {titre} sur {}, {contenu}", formater_milliemes(bareme_m))
}

/// Moyenne affichée (millièmes) : « 12,5 » ou un tiret.
pub fn moyenne_affichee(m: Option<i64>) -> String {
    m.map_or_else(|| "–".to_string(), formater_milliemes)
}

// ───────────────────────── Navigation au clavier ─────────────────────────

/// Une touche de navigation dans la grille.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deplacement {
    Haut,
    Bas,
    Gauche,
    Droite,
}

/// Décision d'une touche dans une case : `Some((ligne, colonne))` à focaliser, ou `None` (comportement natif).
/// Les flèches gauche/droite ne quittent la case que si le curseur est au bord du texte (on édite sinon).
pub fn cible_touche(
    touche: &str,
    maj: bool,
    ligne: usize,
    col: usize,
    nb_lignes: usize,
    nb_cols: usize,
    curseur_au_debut: bool,
    curseur_a_la_fin: bool,
) -> Option<(usize, usize)> {
    let d = match touche {
        "ArrowDown" => Deplacement::Bas,
        "ArrowUp" => Deplacement::Haut,
        "Enter" if maj => Deplacement::Haut,
        "Enter" => Deplacement::Bas,
        "ArrowRight" if curseur_a_la_fin => Deplacement::Droite,
        "ArrowLeft" if curseur_au_debut => Deplacement::Gauche,
        _ => return None,
    };
    match d {
        Deplacement::Bas if ligne + 1 < nb_lignes => Some((ligne + 1, col)),
        Deplacement::Haut if ligne > 0 => Some((ligne - 1, col)),
        Deplacement::Droite if col + 1 < nb_cols => Some((ligne, col + 1)),
        Deplacement::Gauche if col > 0 => Some((ligne, col - 1)),
        _ => None,
    }
}

/// Identifiant du champ d'une case (pour le focus).
pub fn id_case(ligne: usize, col: usize) -> String {
    format!("eva3-c-{ligne}-{col}")
}

// ───────────────────────── Lecture d'un collage ou d'un fichier ─────────────────────────

/// Lit un texte de tableur en lignes de cellules. Détecte le séparateur (tabulation > point-virgule > virgule),
/// gère les champs entre guillemets (`""` pour un guillemet), les fins de ligne `\r\n`, ignore la dernière
/// ligne vide. Le séparateur décimal « , » d'une note n'est pris pour séparateur que si ni tabulation ni
/// point-virgule n'apparaissent ET que le texte ressemble à du CSV (au moins deux lignes de même largeur).
pub fn lire_tableur(texte: &str) -> Vec<Vec<String>> {
    let texte = texte.trim_start_matches('\u{feff}');
    let sep = if texte.contains('\t') {
        '\t'
    } else if texte.contains(';') {
        ';'
    } else if ressemble_a_du_csv_virgule(texte) {
        ','
    } else {
        '\u{0}'
    };
    let mut lignes: Vec<Vec<String>> = Vec::new();
    let mut ligne: Vec<String> = Vec::new();
    let mut champ = String::new();
    let mut entre_guillemets = false;
    let mut chars = texte.chars().peekable();
    while let Some(c) = chars.next() {
        if entre_guillemets {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    champ.push('"');
                    chars.next();
                } else {
                    entre_guillemets = false;
                }
            } else {
                champ.push(c);
            }
            continue;
        }
        match c {
            '"' if champ.is_empty() => entre_guillemets = true,
            c if c == sep => {
                ligne.push(std::mem::take(&mut champ));
            }
            '\r' => {}
            '\n' => {
                ligne.push(std::mem::take(&mut champ));
                lignes.push(std::mem::take(&mut ligne));
            }
            c => champ.push(c),
        }
    }
    if !champ.is_empty() || !ligne.is_empty() {
        ligne.push(champ);
        lignes.push(ligne);
    }
    // Ligne finale vide (« a\tb\n ») : ignorée ; une ligne entièrement vide au milieu est conservée (rang).
    while lignes.last().is_some_and(|l| l.iter().all(|c| c.trim().is_empty())) {
        lignes.pop();
    }
    lignes
}

fn ressemble_a_du_csv_virgule(t: &str) -> bool {
    let largeurs: Vec<usize> = t
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.matches(',').count())
        .collect();
    largeurs.len() >= 2 && largeurs.iter().all(|n| *n == largeurs[0]) && largeurs[0] >= 1
        // « 12,5 » seul sur chaque ligne est une colonne de notes, pas un CSV à deux champs.
        && t.lines().any(|l| l.chars().any(|c| c.is_alphabetic() && !"abdnrsp".contains(c.to_ascii_lowercase())))
}

/// Un collage qui tient dans une seule case (« 12,5 ») se colle comme un texte ordinaire.
pub fn est_un_bloc(texte: &str) -> bool {
    let t = lire_tableur(texte);
    t.len() > 1 || t.first().is_some_and(|l| l.len() > 1)
}

/// Colle un bloc à partir de la case (ligne, col) : une ligne de la grille par ligne collée (même ordre), une
/// évaluation par colonne collée. Rend les évaluations visées et les lignes (`eleve_id` + textes). Refuse un
/// bloc qui déborde de la grille (rien n'est deviné).
pub fn plan_collage(
    bloc: &[Vec<String>],
    ligne: usize,
    col: usize,
    eleves: &[Uuid],
    evaluations: &[Uuid],
) -> Result<(Vec<Uuid>, Vec<ImportLigneBody>), String> {
    if bloc.is_empty() {
        return Err("Rien à coller.".into());
    }
    let largeur = bloc.iter().map(Vec::len).max().unwrap_or(0);
    if ligne + bloc.len() > eleves.len() {
        return Err(format!(
            "Le bloc compte {} ligne(s) et dépasse la fin de la liste d'élèves ({} élève(s) à partir de la case choisie).",
            bloc.len(),
            eleves.len().saturating_sub(ligne)
        ));
    }
    if col + largeur > evaluations.len() {
        return Err(format!(
            "Le bloc compte {largeur} colonne(s) et dépasse la dernière évaluation ({} évaluation(s) à partir de la case choisie).",
            evaluations.len().saturating_sub(col)
        ));
    }
    let cibles = evaluations[col..col + largeur].to_vec();
    let lignes = bloc
        .iter()
        .enumerate()
        .map(|(i, l)| ImportLigneBody {
            eleve_id: Some(eleves[ligne + i]),
            nom: None,
            rang: None,
            cellules: (0..largeur).map(|k| l.get(k).cloned().unwrap_or_default()).collect(),
        })
        .collect();
    Ok((cibles, lignes))
}

/// Évaluation d'après un titre d'en-tête (insensible à la casse, aux accents simples et aux espaces).
fn cle_titre(t: &str) -> String {
    t.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'à' | 'â' => 'a',
            'î' | 'ï' => 'i',
            'ô' => 'o',
            'ù' | 'û' => 'u',
            'ç' => 'c',
            c => c,
        })
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Ce que le fichier du professeur contient, une fois lu.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanFichier {
    pub cibles: Vec<Uuid>,
    pub lignes: Vec<ImportLigneBody>,
    /// Colonnes de l'en-tête qui ne correspondent à aucune évaluation (laissées de côté, dites).
    pub colonnes_ignorees: Vec<String>,
    /// La première ligne a été prise pour un en-tête.
    pub entete: bool,
}

/// Plan d'import d'un fichier. Colonne 1 : le nom de l'élève (« NOM Prénom » dans n'importe quel ordre). Les
/// colonnes suivantes : une évaluation chacune, retrouvée par le titre de l'en-tête ; sans en-tête reconnu, et
/// s'il y a une seule colonne de notes, elle va dans `cible_unique`. Sans nom (première colonne vide ou
/// absente), le rang dans la liste sert au rapprochement (CE-55, « puis par ordre »).
pub fn plan_fichier(
    table: &[Vec<String>],
    evaluations: &[(Uuid, String)],
    cible_unique: Option<Uuid>,
) -> Result<PlanFichier, String> {
    if table.is_empty() {
        return Err("Le fichier est vide.".into());
    }
    let largeur = table.iter().map(Vec::len).max().unwrap_or(0);
    if largeur < 2 {
        return Err("Le fichier doit avoir au moins deux colonnes : le nom de l'élève et les notes.".into());
    }
    let entete: Vec<Option<Uuid>> = table[0]
        .iter()
        .skip(1)
        .map(|h| {
            let k = cle_titre(h);
            evaluations.iter().find(|(_, t)| !k.is_empty() && cle_titre(t) == k).map(|(id, _)| *id)
        })
        .collect();
    let a_un_entete = entete.iter().any(Option::is_some);
    let (debut, colonnes): (usize, Vec<Option<Uuid>>) = if a_un_entete {
        (1, entete.clone())
    } else if largeur == 2 {
        (0, vec![cible_unique])
    } else {
        return Err(
            "Les colonnes de notes ne se reconnaissent pas : mettez en en-tête le titre exact de chaque évaluation."
                .into(),
        );
    };
    if colonnes.iter().all(Option::is_none) {
        return Err("Choisissez l'évaluation visée : le fichier n'a qu'une colonne de notes sans titre reconnu.".into());
    }
    let mut cibles: Vec<Uuid> = Vec::new();
    let mut indices: Vec<usize> = Vec::new();
    for (i, c) in colonnes.iter().enumerate() {
        if let Some(id) = c {
            if cibles.contains(id) {
                return Err("Deux colonnes du fichier visent la même évaluation.".into());
            }
            cibles.push(*id);
            indices.push(i + 1);
        }
    }
    let colonnes_ignorees: Vec<String> = if a_un_entete {
        table[0]
            .iter()
            .skip(1)
            .zip(colonnes.iter())
            .filter(|(h, c)| c.is_none() && !h.trim().is_empty())
            .map(|(h, _)| h.trim().to_string())
            .collect()
    } else {
        Vec::new()
    };
    let lignes = table[debut..]
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let nom = l.first().map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
            ImportLigneBody {
                eleve_id: None,
                rang: nom.is_none().then_some(i + 1),
                nom,
                cellules: indices.iter().map(|k| l.get(*k).cloned().unwrap_or_default()).collect(),
            }
        })
        .collect();
    Ok(PlanFichier { cibles, lignes, colonnes_ignorees, entete: a_un_entete })
}

// ───────────────────────── Envoi et retours ─────────────────────────

/// Une case de l'écran, vue par la logique pure.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseVue {
    pub evaluation_id: Uuid,
    pub eleve_id: Uuid,
    /// Texte tapé en ce moment.
    pub texte: String,
    /// Texte du serveur à la dernière lecture ou écriture.
    pub base: String,
    pub version: Option<i32>,
    pub concerne: bool,
    pub saisie_ouverte: bool,
    /// Case en conflit non résolu : n'est plus envoyée tant qu'elle n'est pas tranchée.
    pub en_conflit: bool,
}

impl CaseVue {
    pub fn modifiee(&self) -> bool {
        self.concerne && self.texte.trim() != self.base.trim() && !self.texte.trim().is_empty()
    }
}

/// Les cases à envoyer : modifiées, concernées, hors conflit (`ouverte_a_la_saisie` : la direction en lecture
/// corrige avec un motif, géré à part). Le serveur reste juge de chacune.
pub fn cases_a_envoyer(cases: &[CaseVue], motif: Option<&str>) -> Vec<CelluleSaisieDto> {
    cases
        .iter()
        .filter(|c| c.modifiee() && !c.en_conflit)
        .map(|c| CelluleSaisieDto {
            evaluation_id: c.evaluation_id,
            eleve_id: c.eleve_id,
            texte: c.texte.trim().to_string(),
            version_attendue: c.version,
            motif: if c.saisie_ouverte { None } else { motif.map(str::to_string) },
        })
        .collect()
}

/// Effet d'un retour du serveur sur une case.
#[derive(Debug, Clone, PartialEq)]
pub enum EffetRetour {
    /// Écrit ou inchangé : la case prend cette version et ce texte de base.
    Enregistree { version: i32, base: String },
    /// Rien à écrire.
    Vide,
    /// La case a changé ailleurs : à résoudre explicitement.
    Conflit(ConflitDto),
    /// Refusée, avec la raison.
    Refusee(String),
}

pub fn effet_retour(r: &CelluleRetourDto) -> EffetRetour {
    match r.issue.as_str() {
        "ecrit" | "inchange" => EffetRetour::Enregistree {
            version: r.version.unwrap_or(1),
            base: match (r.statut.as_deref(), r.valeur_m) {
                (Some("note"), Some(m)) => formater_milliemes(m),
                (Some(s), _) => code_statut(s).to_string(),
                _ => String::new(),
            },
        },
        "vide" => EffetRetour::Vide,
        "conflit" => match &r.conflit {
            Some(c) => EffetRetour::Conflit(c.clone()),
            None => EffetRetour::Refusee("La case a changé : rechargez la grille.".to_string()),
        },
        _ => EffetRetour::Refusee(r.message.clone().unwrap_or_else(|| "Saisie refusée.".to_string())),
    }
}

/// Texte « canonique » de la case telle qu'elle est en base (pour « prendre la valeur en base »).
pub fn texte_conflit(c: &ConflitDto) -> String {
    match (c.statut.as_str(), c.valeur_m) {
        ("note", Some(m)) => formater_milliemes(m),
        (s, _) => code_statut(s).to_string(),
    }
}

/// Texte d'un conflit : ce qui est en base, par qui, quand (heure locale posée par l'écran).
pub fn phrase_conflit(c: &ConflitDto) -> String {
    let valeur = match (c.statut.as_str(), c.valeur_m) {
        ("note", Some(m)) => formater_milliemes(m),
        (s, _) => libelle_statut(s).to_string(),
    };
    match &c.par_nom {
        Some(n) => format!("{valeur} (saisi par {n})"),
        None => valeur,
    }
}

/// Bilan d'un lot, en une phrase pour la zone `aria-live`.
pub fn bilan_lot(retours: &[CelluleRetourDto]) -> String {
    let n = |i: &str| retours.iter().filter(|r| r.issue == i).count();
    let ecrites = n("ecrit");
    let mut parts = Vec::new();
    if ecrites > 0 {
        parts.push(format!("{ecrites} case(s) enregistrée(s)"));
    }
    if n("conflit") > 0 {
        parts.push(format!("{} conflit(s) à résoudre", n("conflit")));
    }
    if n("refuse") > 0 {
        parts.push(format!("{} case(s) refusée(s)", n("refuse")));
    }
    if parts.is_empty() {
        "Aucune modification à enregistrer.".to_string()
    } else {
        parts.join(", ") + "."
    }
}

// ───────────────────────── Création d'une évaluation (contrôle local) ─────────────────────────

/// Un nombre saisi dans un formulaire (« 20 », « 1,5 »), en millièmes.
pub fn milliemes_formulaire(t: &str) -> Option<i64> {
    ent_types::evaluations_v3::milliemes_depuis_texte(t)
}

/// Erreurs d'un formulaire d'évaluation : (champ, message). Le serveur revérifie tout.
pub fn controler_evaluation(titre: &str, date: &str, bareme: &str, coef: &str) -> Vec<(&'static str, String)> {
    let mut e = Vec::new();
    if titre.trim().is_empty() {
        e.push(("titre", "Le titre est obligatoire.".to_string()));
    }
    if date.trim().is_empty() {
        e.push(("date", "La date est obligatoire.".to_string()));
    }
    match milliemes_formulaire(bareme) {
        Some(b) if b > 0 && b <= 1_000_000 => {}
        _ => e.push(("bareme", "Barème : un nombre entre 0,001 et 1000.".to_string())),
    }
    match milliemes_formulaire(coef) {
        Some(c) if c <= 100_000 => {}
        _ => e.push(("coef", "Coefficient : un nombre entre 0 et 100.".to_string())),
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn tableur_tabulation_guillemets_et_fin_de_ligne() {
        let t = lire_tableur("Léa Martin\t12,5\t\"a\tb\"\r\nPaul Durand\tabs\t\r\n");
        assert_eq!(t.len(), 2);
        assert_eq!(t[0], vec!["Léa Martin", "12,5", "a\tb"]);
        assert_eq!(t[1], vec!["Paul Durand", "abs", ""]);
    }

    #[test]
    fn tableur_point_virgule_excel_francais_et_csv_virgule() {
        let t = lire_tableur("Nom;DS 1;DS 2\nMartin Léa;12,5;8\n");
        assert_eq!(t[0], vec!["Nom", "DS 1", "DS 2"]);
        assert_eq!(t[1], vec!["Martin Léa", "12,5", "8"]);
        let c = lire_tableur("Nom,DS 1\nMartin Léa,12\nDurand Paul,9\n");
        assert_eq!(c[2], vec!["Durand Paul", "9"]);
    }

    #[test]
    fn une_colonne_de_notes_a_virgule_decimale_n_est_pas_un_csv() {
        let t = lire_tableur("12,5\n8,25\nabs\n");
        assert_eq!(t, vec![vec!["12,5"], vec!["8,25"], vec!["abs"]]);
    }

    #[test]
    fn un_bloc_est_plusieurs_cases() {
        assert!(!est_un_bloc("12,5"));
        assert!(!est_un_bloc("12,5\n"));
        assert!(est_un_bloc("12\n13"));
        assert!(est_un_bloc("12\t13"));
    }

    #[test]
    fn collage_cale_sur_la_case_et_refuse_un_debordement() {
        let eleves = [id(1), id(2), id(3)];
        let evals = [id(10), id(11)];
        let bloc = lire_tableur("12\t8\n13\t9\n");
        let (cibles, lignes) = plan_collage(&bloc, 1, 0, &eleves, &evals).unwrap();
        assert_eq!(cibles, vec![id(10), id(11)]);
        assert_eq!(lignes[0].eleve_id, Some(id(2)));
        assert_eq!(lignes[1].eleve_id, Some(id(3)));
        assert_eq!(lignes[1].cellules, vec!["13", "9"]);
        assert!(plan_collage(&bloc, 2, 0, &eleves, &evals).unwrap_err().contains("dépasse"));
        assert!(plan_collage(&bloc, 0, 1, &eleves, &evals).unwrap_err().contains("colonne"));
    }

    #[test]
    fn fichier_avec_entete_reconnu_et_colonne_ignoree() {
        let evals = vec![(id(10), "DS 1".to_string()), (id(11), "Interro n°2".to_string())];
        let t = lire_tableur("Nom;ds 1;Moyenne;Interro n°2\nMartin Léa;12;11;8\n;9;;7\n");
        let p = plan_fichier(&t, &evals, None).unwrap();
        assert_eq!(p.cibles, vec![id(10), id(11)]);
        assert_eq!(p.colonnes_ignorees, vec!["Moyenne"]);
        assert_eq!(p.lignes[0].nom.as_deref(), Some("Martin Léa"));
        assert_eq!(p.lignes[0].cellules, vec!["12", "8"]);
        assert_eq!((p.lignes[1].nom.clone(), p.lignes[1].rang), (None, Some(2)), "sans nom : par rang");
        assert!(p.entete);
    }

    #[test]
    fn fichier_sans_entete_une_seule_colonne_va_dans_l_evaluation_choisie() {
        let evals = vec![(id(10), "DS 1".to_string())];
        let t = lire_tableur("Martin Léa\t12\nDurand Paul\tabs\n");
        let p = plan_fichier(&t, &evals, Some(id(10))).unwrap();
        assert_eq!(p.cibles, vec![id(10)]);
        assert_eq!(p.lignes.len(), 2);
        assert!(plan_fichier(&t, &evals, None).is_err(), "sans évaluation choisie ni titre : refus");
        let large = lire_tableur("Martin\t12\t13\n");
        assert!(plan_fichier(&large, &evals, Some(id(10))).is_err());
        assert!(plan_fichier(&[], &evals, None).is_err());
    }

    #[test]
    fn fichier_deux_colonnes_pour_la_meme_evaluation_refuse() {
        let evals = vec![(id(10), "DS 1".to_string())];
        let t = lire_tableur("Nom;DS 1;ds1\nMartin;12;13\n");
        assert!(plan_fichier(&t, &evals, None).is_err());
    }

    #[test]
    fn clavier_fleches_entree_et_bords() {
        let c = |k, maj, l, col, g, d| cible_touche(k, maj, l, col, 3, 2, g, d);
        assert_eq!(c("ArrowDown", false, 0, 0, false, false), Some((1, 0)));
        assert_eq!(c("ArrowDown", false, 2, 0, false, false), None, "dernière ligne");
        assert_eq!(c("ArrowUp", false, 1, 1, false, false), Some((0, 1)));
        assert_eq!(c("Enter", false, 0, 1, false, false), Some((1, 1)));
        assert_eq!(c("Enter", true, 1, 1, false, false), Some((0, 1)));
        assert_eq!(c("ArrowRight", false, 0, 0, false, true), Some((0, 1)));
        assert_eq!(c("ArrowRight", false, 0, 0, false, false), None, "curseur dans le texte : on édite");
        assert_eq!(c("ArrowLeft", false, 0, 1, true, false), Some((0, 0)));
        assert_eq!(c("ArrowLeft", false, 0, 0, true, false), None);
        assert_eq!(c("a", false, 0, 0, true, true), None);
    }

    fn case(texte: &str, base: &str, version: Option<i32>) -> CaseVue {
        CaseVue {
            evaluation_id: id(10),
            eleve_id: id(1),
            texte: texte.into(),
            base: base.into(),
            version,
            concerne: true,
            saisie_ouverte: true,
            en_conflit: false,
        }
    }

    #[test]
    fn seules_les_cases_modifiees_partent_avec_leur_version() {
        let mut conflit = case("5", "", None);
        conflit.en_conflit = true;
        let mut non_concerne = case("5", "", None);
        non_concerne.concerne = false;
        let cases = vec![case("12", "", None), case("13", "13", Some(2)), case("14", "13", Some(2)), conflit, non_concerne, case("", "13", Some(2))];
        let lot = cases_a_envoyer(&cases, None);
        assert_eq!(lot.len(), 2);
        assert_eq!((lot[0].texte.as_str(), lot[0].version_attendue), ("12", None));
        assert_eq!((lot[1].texte.as_str(), lot[1].version_attendue), ("14", Some(2)));
    }

    #[test]
    fn le_motif_ne_part_que_pour_les_cases_verrouillees() {
        let mut v = case("15", "11", Some(1));
        v.saisie_ouverte = false;
        let lot = cases_a_envoyer(&[v, case("9", "", None)], Some("erreur de report"));
        assert_eq!(lot[0].motif.as_deref(), Some("erreur de report"));
        assert_eq!(lot[1].motif, None);
    }

    fn retour(issue: &str) -> CelluleRetourDto {
        CelluleRetourDto {
            evaluation_id: id(10),
            eleve_id: id(1),
            issue: issue.into(),
            version: Some(3),
            statut: Some("note".into()),
            valeur_m: Some(12_500),
            message: None,
            conflit: None,
        }
    }

    #[test]
    fn retours_ecrit_conflit_refus() {
        assert_eq!(effet_retour(&retour("ecrit")), EffetRetour::Enregistree { version: 3, base: "12,5".into() });
        let mut c = retour("conflit");
        c.conflit = Some(ConflitDto { statut: "note".into(), valeur_m: Some(9_000), version: 4, par_nom: Some("Ada Lovelace".into()), le: None });
        match effet_retour(&c) {
            EffetRetour::Conflit(x) => assert_eq!(phrase_conflit(&x), "9 (saisi par Ada Lovelace)"),
            autre => panic!("{autre:?}"),
        }
        let mut r = retour("refuse");
        r.message = Some("21 dépasse le barème".into());
        assert_eq!(effet_retour(&r), EffetRetour::Refusee("21 dépasse le barème".into()));
        assert_eq!(effet_retour(&retour("vide")), EffetRetour::Vide);
        let mut sans = retour("conflit");
        sans.conflit = None;
        assert!(matches!(effet_retour(&sans), EffetRetour::Refusee(_)), "un conflit sans état n'écrase jamais");
    }

    #[test]
    fn bilan_en_une_phrase() {
        assert_eq!(bilan_lot(&[retour("ecrit"), retour("ecrit"), retour("conflit"), retour("refuse")]), "2 case(s) enregistrée(s), 1 conflit(s) à résoudre, 1 case(s) refusée(s).");
        assert_eq!(bilan_lot(&[]), "Aucune modification à enregistrer.");
    }

    #[test]
    fn textes_de_case_et_phrase_pour_lecteur_d_ecran() {
        let c = CelluleDto { concerne: true, statut: Some("note".into()), valeur_m: Some(12_500), version: Some(1), resultat_id: None, saisie_ouverte: true, rattrapage_autorise: false, hors_periode: false, corrige: false, commentaire: false };
        assert_eq!(texte_cellule(&c), "12,5");
        let a = CelluleDto { statut: Some("absent".into()), valeur_m: None, ..c.clone() };
        assert_eq!(texte_cellule(&a), "abs");
        assert_eq!(phrase_cellule("Léa Martin", "DS 1", 20_000, Some("note"), Some(12_500)), "Léa Martin, DS 1 sur 20, 12,5 sur 20");
        assert_eq!(phrase_cellule("Léa Martin", "DS 1", 20_000, Some("absent"), None), "Léa Martin, DS 1 sur 20, Absent");
        assert_eq!(phrase_cellule("Léa Martin", "DS 1", 20_000, None, None), "Léa Martin, DS 1 sur 20, pas de résultat");
        assert_eq!(moyenne_affichee(None), "–");
        assert_eq!(moyenne_affichee(Some(14_000)), "14");
    }

    #[test]
    fn controle_du_formulaire_d_evaluation() {
        assert!(controler_evaluation("DS", "2031-10-10", "20", "1,5").is_empty());
        let e = controler_evaluation(" ", "", "0", "101");
        assert_eq!(e.iter().map(|x| x.0).collect::<Vec<_>>(), ["titre", "date", "bareme", "coef"]);
        assert!(!controler_evaluation("DS", "2031-10-10", "20", "0").is_empty() == false, "coefficient 0 admis");
    }
}
