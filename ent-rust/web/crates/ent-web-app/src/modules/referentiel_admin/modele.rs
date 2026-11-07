//! Logique d'écran de l'administration du référentiel : **pure**, sans Leptos ni réseau,
//! testée en natif. Le serveur reste l'autorité (couverture des périodes, droits, bascule) :
//! ces fonctions servent à guider la saisie, à composer les corps de requête et à résumer
//! les rapports renvoyés.

use chrono::{Duration, NaiveDate, NaiveTime};
use ent_types::referentiel_admin::{
    AnneeStatutDto, CorrespondanceDto, CreneauGrilleDto, CreneauGrilleSaisieDto, JourCalendrierDto,
    MefImportDto, PeriodeSaisieDto, RapportImportMefDto, RapportLotDto, RapportOuvertureDto,
    RapportReconductionDto, RapportSeancesDto,
};
use ent_web_api::ClasseApi;
use ent_web_ui::TonEtat;
use uuid::Uuid;

/// Erreurs de saisie : (identifiant du champ, message), pour `ResumeErreurs`.
pub type Erreurs = Vec<(String, String)>;

fn erreur(champ: &str, message: &str) -> (String, String) {
    (champ.to_string(), message.to_string())
}

pub fn lire_date(texte: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texte.trim(), "%Y-%m-%d").ok()
}

pub fn lire_heure(texte: &str) -> Option<NaiveTime> {
    let t = texte.trim();
    NaiveTime::parse_from_str(t, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(t, "%H:%M:%S"))
        .ok()
}

/// « 08:00 » pour l'affichage et les champs de saisie.
pub fn heure_texte(h: NaiveTime) -> String {
    h.format("%H:%M").to_string()
}

// ───────────────────────────── Années et bascule ─────────────────────────────

pub fn libelle_statut_annee(statut: &str) -> &'static str {
    match statut {
        "preparation" => "En préparation",
        "courante" => "Courante",
        "cloturee" => "Clôturée",
        _ => "Statut inconnu",
    }
}

pub fn ton_statut_annee(statut: &str) -> TonEtat {
    match statut {
        "courante" => TonEtat::Ok,
        "preparation" => TonEtat::Avertissement,
        _ => TonEtat::Neutre,
    }
}

/// Étapes proposées pour une année selon son statut (le serveur revérifie chaque transition).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionsAnnee {
    pub preparer_suivante: bool,
    pub ouvrir: bool,
    pub cloturer: bool,
}

pub fn actions_annee(a: &AnneeStatutDto, annees: &[AnneeStatutDto]) -> ActionsAnnee {
    ActionsAnnee {
        preparer_suivante: a.statut == "courante" && annee_suivante(a, annees).is_none(),
        ouvrir: a.statut == "preparation",
        cloturer: a.statut == "courante",
    }
}

/// Année qui suit `a` : celle dont le début est le plus proche après la fin de `a`.
pub fn annee_suivante<'a>(
    a: &AnneeStatutDto,
    annees: &'a [AnneeStatutDto],
) -> Option<&'a AnneeStatutDto> {
    annees
        .iter()
        .filter(|x| x.id != a.id && x.date_debut > a.date_fin)
        .min_by_key(|x| x.date_debut)
}

/// Année en préparation qui suit la courante (cible de la bascule) : une année en préparation
/// antérieure à la courante n'est pas une cible de montée.
pub fn annee_cible(annees: &[AnneeStatutDto]) -> Option<&AnneeStatutDto> {
    let courante = annee_courante(annees)?;
    annees
        .iter()
        .filter(|a| a.statut == "preparation" && a.date_debut > courante.date_fin)
        .min_by_key(|a| a.date_debut)
}

/// Année dont l'état est `courante`, si elle existe.
pub fn annee_courante(annees: &[AnneeStatutDto]) -> Option<&AnneeStatutDto> {
    annees.iter().find(|a| a.statut == "courante")
}

/// Années triées de la plus récente à la plus ancienne.
pub fn trier_annees(mut annees: Vec<AnneeStatutDto>) -> Vec<AnneeStatutDto> {
    annees.sort_by_key(|a| std::cmp::Reverse(a.date_debut));
    annees
}

/// « 2026-2027 » → « 2027-2028 ».
pub fn libelle_annee_suivante(libelle: &str) -> Option<String> {
    let (a, b) = libelle.trim().split_once('-')?;
    let (a, b): (i32, i32) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
    (b == a + 1).then(|| format!("{}-{}", a + 1, b + 1))
}

/// État d'une étape de la bascule d'année.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatEtape {
    Fait,
    AFaire,
    Attente,
}

impl EtatEtape {
    pub fn libelle(self) -> &'static str {
        match self {
            EtatEtape::Fait => "Fait",
            EtatEtape::AFaire => "À faire",
            EtatEtape::Attente => "En attente",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EtapeBascule {
    pub libelle: &'static str,
    pub etat: EtatEtape,
}

/// Les trois étapes de la bascule, d'après les années connues. Le serveur reste l'autorité : l'état
/// « Fait » ne sert que pour la préparation (on voit l'année suivante) ; les élèves sont montés
/// et l'année ouverte à la demande, avec aperçu avant toute écriture.
pub fn etapes_bascule(annees: &[AnneeStatutDto]) -> Vec<EtapeBascule> {
    let courante = annee_courante(annees);
    let preparation = annee_cible(annees);
    let (prepare, ouvrable) = (preparation.is_some(), preparation.is_some());
    vec![
        EtapeBascule {
            libelle: "Préparer l'année suivante (classes, groupes et périodes reconduits)",
            etat: if prepare {
                EtatEtape::Fait
            } else if courante.is_some() {
                EtatEtape::AFaire
            } else {
                EtatEtape::Attente
            },
        },
        EtapeBascule {
            libelle: "Monter les élèves dans leurs nouvelles classes (aperçu puis application)",
            etat: if ouvrable {
                EtatEtape::AFaire
            } else {
                EtatEtape::Attente
            },
        },
        EtapeBascule {
            libelle: "Ouvrir la nouvelle année (l'ancienne est clôturée)",
            etat: if ouvrable {
                EtatEtape::AFaire
            } else {
                EtatEtape::Attente
            },
        },
    ]
}

/// Phrase d'aperçu avant l'ouverture d'une année (aucun aperçu serveur n'existe pour cette étape :
/// on décrit les effets documentés, sans les chiffrer).
pub fn apercu_ouverture(courante: Option<&AnneeStatutDto>, cible: &AnneeStatutDto) -> String {
    match courante {
        Some(c) => format!(
            "L'année {} sera clôturée (lecture seule), l'année {} deviendra l'année courante et \
             la classe courante des élèves basculera sur leur affectation de {}.",
            c.libelle, cible.libelle, cible.libelle
        ),
        None => format!(
            "L'année {} deviendra l'année courante ; aucune année n'est aujourd'hui courante.",
            cible.libelle
        ),
    }
}

pub fn resume_reconduction(r: &RapportReconductionDto) -> String {
    format!(
        "{} classe(s), {} groupe(s), {} période(s) et {} service(s) créés.",
        r.classes_creees, r.groupes_crees, r.periodes_creees, r.services_crees
    )
}

pub fn resume_ouverture(r: &RapportOuvertureDto) -> String {
    format!(
        "Année ouverte ; {} élève(s) basculé(s) sur leur classe de la nouvelle année.",
        r.eleves_synchronises
    )
}

/// Lignes de l'aperçu d'une montée ou d'une affectation en lot.
pub fn resume_lot(r: &RapportLotDto) -> Vec<String> {
    let mut v = vec![
        format!("{} affectation(s) à créer", r.a_creer),
        format!("{} déjà conforme(s)", r.deja_conformes),
    ];
    if !r.conflits.is_empty() {
        v.push(format!(
            "{} élève(s) déjà affecté(s) à une autre classe : non modifié(s)",
            r.conflits.len()
        ));
    }
    if !r.ignores.is_empty() {
        v.push(format!(
            "{} élève(s) ignoré(s) (inconnus ou sortis avant l'année)",
            r.ignores.len()
        ));
    }
    v
}

/// L'application n'est proposée qu'après un aperçu qui crée quelque chose.
pub fn lot_applicable(apercu: Option<&RapportLotDto>) -> bool {
    apercu.is_some_and(|r| r.apercu && r.a_creer > 0)
}

/// Code de la classe du niveau supérieur, proposé par défaut pour la montée (« 6A » → « 5A »).
/// Collège : 6e → 5e → 4e → 3e ; lycée : 2de → 1re → terminale. La 3e (sortie du collège) et
/// la terminale n'ont pas de suite : l'utilisateur choisit. Simple préremplissage, à vérifier.
pub fn code_classe_suivant(code: &str) -> Option<String> {
    let mut cars = code.trim().chars();
    let premier = cars.next()?;
    let reste: String = cars.collect();
    let suivant = match premier {
        '6' => '5',
        '5' => '4',
        '4' => '3',
        '2' => '1',
        '1' => 'T',
        _ => return None,
    };
    Some(format!("{suivant}{reste}"))
}

/// Correspondances proposées (classe source → classe cible de code « suivant »), à confirmer.
pub fn proposer_correspondances(
    sources: &[ClasseApi],
    cibles: &[ClasseApi],
) -> Vec<(Uuid, Option<Uuid>)> {
    sources
        .iter()
        .map(|s| {
            let cible = code_classe_suivant(&s.code)
                .and_then(|c| cibles.iter().find(|x| x.code.eq_ignore_ascii_case(&c)))
                .map(|x| x.id);
            (s.id, cible)
        })
        .collect()
}

/// Corps de la montée : seules les classes avec une cible choisie.
pub fn correspondances_choisies(choix: &[(Uuid, Option<Uuid>)]) -> Vec<CorrespondanceDto> {
    choix
        .iter()
        .filter_map(|(s, c)| c.map(|cible| CorrespondanceDto { source: *s, cible }))
        .collect()
}

// ───────────────────────────── Périodes ─────────────────────────────

/// Ligne éditable d'une période (textes tels que saisis).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LignePeriode {
    pub code: String,
    pub libelle: String,
    pub kind: String,
    pub debut: String,
    pub fin: String,
}

pub const KINDS_PERIODE: [(&str, &str); 3] = [
    ("trimestre", "Trimestre"),
    ("semestre", "Semestre"),
    ("periode", "Période"),
];

pub fn libelle_kind_periode(kind: &str) -> &'static str {
    KINDS_PERIODE
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, l)| *l)
        .unwrap_or("Période")
}

/// Découpage d'une année en `n` périodes contiguës de durée voisine (point de départ à ajuster).
pub fn periodes_modele(annee: &AnneeStatutDto, kind: &str, n: u32) -> Vec<LignePeriode> {
    let total = (annee.date_fin - annee.date_debut).num_days() + 1;
    if n == 0 || total < i64::from(n) {
        return Vec::new();
    }
    let n = i64::from(n);
    let mut debut = annee.date_debut;
    (0..n)
        .map(|i| {
            let fin = if i == n - 1 {
                annee.date_fin
            } else {
                annee.date_debut + Duration::days(total * (i + 1) / n - 1)
            };
            let ligne = LignePeriode {
                code: format!("{}{}", if kind == "semestre" { "S" } else { "T" }, i + 1),
                libelle: format!("{} {}", libelle_kind_periode(kind), i + 1),
                kind: kind.to_string(),
                debut: debut.to_string(),
                fin: fin.to_string(),
            };
            debut = fin + Duration::days(1);
            ligne
        })
        .collect()
}

pub fn lignes_depuis_periodes(
    periodes: &[ent_types::referentiel_admin::PeriodeDto],
) -> Vec<LignePeriode> {
    let mut v: Vec<_> = periodes.iter().collect();
    v.sort_by_key(|p| (p.ordre, p.date_debut));
    v.into_iter()
        .map(|p| LignePeriode {
            code: p.code.clone(),
            libelle: p.libelle.clone(),
            kind: p.kind.clone(),
            debut: p.date_debut.to_string(),
            fin: p.date_fin.to_string(),
        })
        .collect()
}

/// Compose le corps de `PUT …/periodes` ; refuse ce que le serveur refuserait (couverture de
/// l'année sans trou ni chevauchement, codes uniques) pour guider la saisie. Les champs fautifs
/// sont `periode-{i}-{code|libelle|debut|fin}`.
pub fn composer_periodes(
    lignes: &[LignePeriode],
    annee: &AnneeStatutDto,
) -> Result<Vec<PeriodeSaisieDto>, Erreurs> {
    let mut erreurs = Erreurs::new();
    if lignes.is_empty() {
        return Err(vec![erreur("periodes", "Ajoutez au moins une période.")]);
    }
    let mut saisies = Vec::new();
    for (i, l) in lignes.iter().enumerate() {
        let champ = |c: &str| format!("periode-{i}-{c}");
        let (code, libelle) = (l.code.trim(), l.libelle.trim());
        if code.is_empty() {
            erreurs.push((
                champ("code"),
                format!("Période {} : le code est obligatoire.", i + 1),
            ));
        }
        if libelle.is_empty() {
            erreurs.push((
                champ("libelle"),
                format!("Période {} : le libellé est obligatoire.", i + 1),
            ));
        }
        let (d, f) = (lire_date(&l.debut), lire_date(&l.fin));
        if d.is_none() {
            erreurs.push((
                champ("debut"),
                format!("Période {} : date de début invalide.", i + 1),
            ));
        }
        if f.is_none() {
            erreurs.push((
                champ("fin"),
                format!("Période {} : date de fin invalide.", i + 1),
            ));
        }
        if let (Some(d), Some(f)) = (d, f) {
            if f < d {
                erreurs.push((
                    champ("fin"),
                    format!("Période {} : la fin précède le début.", i + 1),
                ));
            } else if !code.is_empty() && !libelle.is_empty() {
                saisies.push(PeriodeSaisieDto {
                    code: code.to_string(),
                    libelle: libelle.to_string(),
                    kind: l.kind.clone(),
                    date_debut: d,
                    date_fin: f,
                });
            }
        }
    }
    if !erreurs.is_empty() {
        return Err(erreurs);
    }
    saisies.sort_by_key(|p| p.date_debut);
    if saisies[0].date_debut != annee.date_debut {
        erreurs.push(erreur(
            "periode-0-debut",
            "La première période doit commencer au début de l'année.",
        ));
    }
    for w in saisies.windows(2) {
        if w[1].date_debut <= w[0].date_fin {
            erreurs.push(erreur(
                "periodes",
                &format!(
                    "Les périodes « {} » et « {} » se chevauchent.",
                    w[0].code, w[1].code
                ),
            ));
        } else if w[1].date_debut != w[0].date_fin + Duration::days(1) {
            erreurs.push(erreur(
                "periodes",
                &format!(
                    "Il y a un trou entre « {} » et « {} ».",
                    w[0].code, w[1].code
                ),
            ));
        }
    }
    if saisies[saisies.len() - 1].date_fin != annee.date_fin {
        erreurs.push(erreur(
            "periodes",
            "La dernière période doit finir à la fin de l'année.",
        ));
    }
    let mut codes = std::collections::HashSet::new();
    if !saisies.iter().all(|p| codes.insert(p.code.clone())) {
        erreurs.push(erreur("periodes", "Deux périodes ont le même code."));
    }
    if erreurs.is_empty() {
        Ok(saisies)
    } else {
        Err(erreurs)
    }
}

// ───────────────────────────── Calendrier ─────────────────────────────

/// Types de jours neutralisés (`ent_jour_calendrier`).
pub const KINDS_JOUR: [(&str, &str); 4] = [
    ("ferme", "Fermeture de l'établissement"),
    ("banalise", "Journée banalisée"),
    ("ferie", "Jour férié"),
    ("vacances", "Vacances"),
];

/// Types de périodes de vacances (`ent_periode_vacances`).
pub const KINDS_VACANCES: [(&str, &str); 3] = [
    ("vacances", "Vacances"),
    ("ferie", "Jour férié"),
    ("fermeture", "Fermeture"),
];

pub fn libelle_kind_jour(kind: &str) -> &'static str {
    match kind {
        "ferme" | "fermeture" => "Fermeture",
        "banalise" => "Journée banalisée",
        "ferie" => "Jour férié",
        "vacances" => "Vacances",
        _ => "Jour neutralisé",
    }
}

/// Plus longue plage qu'on laisse saisir d'un coup (garde-fou de saisie, pas une règle métier).
pub const PLAGE_MAX_JOURS: i64 = 400;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisiePlage {
    pub du: String,
    pub au: String,
    pub kind: String,
    pub libelle: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlageValidee {
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub kind: String,
    pub libelle: String,
}

/// Valide une plage (jours fermés ou vacances) avant envoi. Champs : `cal-du`, `cal-au`, `cal-libelle`.
pub fn composer_plage(s: &SaisiePlage, kinds: &[(&str, &str)]) -> Result<PlageValidee, Erreurs> {
    let mut e = Erreurs::new();
    let (du, au) = (lire_date(&s.du), lire_date(&s.au));
    if du.is_none() {
        e.push(erreur("cal-du", "Date de début invalide."));
    }
    if au.is_none() {
        e.push(erreur("cal-au", "Date de fin invalide."));
    }
    let libelle = s.libelle.trim();
    if libelle.is_empty() {
        e.push(erreur("cal-libelle", "Le libellé est obligatoire."));
    } else if libelle.chars().count() > 120 {
        e.push(erreur(
            "cal-libelle",
            "Libellé trop long (120 caractères au plus).",
        ));
    }
    if !kinds.iter().any(|(k, _)| *k == s.kind) {
        e.push(erreur("cal-kind", "Choisissez un type."));
    }
    if let (Some(d), Some(a)) = (du, au) {
        if a < d {
            e.push(erreur("cal-au", "La fin précède le début."));
        } else if (a - d).num_days() + 1 > PLAGE_MAX_JOURS {
            e.push(erreur("cal-au", "Plage trop longue (400 jours au plus)."));
        }
    }
    match (du, au, e.is_empty()) {
        (Some(du), Some(au), true) => Ok(PlageValidee {
            du,
            au,
            kind: s.kind.clone(),
            libelle: libelle.to_string(),
        }),
        _ => Err(e),
    }
}

/// Plage de jours neutralisés consécutifs de même nature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlageCalendrier {
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub kind: String,
    pub libelle: String,
    pub origine: String,
    pub site_id: Option<Uuid>,
}

impl PlageCalendrier {
    pub fn nb_jours(&self) -> i64 {
        (self.fin - self.debut).num_days() + 1
    }
}

/// Regroupe les jours consécutifs de même type, libellé, origine et site (lecture du calendrier).
pub fn regrouper_calendrier(jours: &[JourCalendrierDto]) -> Vec<PlageCalendrier> {
    let mut tri: Vec<&JourCalendrierDto> = jours.iter().collect();
    tri.sort_by_key(|j| {
        (
            j.kind.clone(),
            j.libelle.clone(),
            j.origine.clone(),
            j.site_id,
            j.jour,
        )
    });
    let mut plages: Vec<PlageCalendrier> = Vec::new();
    for j in tri {
        match plages.last_mut() {
            Some(p)
                if p.kind == j.kind
                    && p.libelle == j.libelle
                    && p.origine == j.origine
                    && p.site_id == j.site_id
                    && j.jour <= p.fin + Duration::days(1) =>
            {
                p.fin = p.fin.max(j.jour);
            }
            _ => plages.push(PlageCalendrier {
                debut: j.jour,
                fin: j.jour,
                kind: j.kind.clone(),
                libelle: j.libelle.clone(),
                origine: j.origine.clone(),
                site_id: j.site_id,
            }),
        }
    }
    plages.sort_by_key(|p| (p.debut, p.fin));
    plages
}

// ───────────────────────────── Grille horaire ─────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LigneCreneau {
    pub code: String,
    pub libelle: String,
    pub demi_journee: String,
    pub debut: String,
    pub fin: String,
}

pub const DEMI_JOURNEES: [(&str, &str); 2] = [("matin", "Matin"), ("apres_midi", "Après-midi")];

pub fn libelle_demi_journee(d: &str) -> &'static str {
    match d {
        "matin" => "Matin",
        "apres_midi" => "Après-midi",
        _ => "—",
    }
}

pub fn lignes_depuis_grille(grille: &[CreneauGrilleDto]) -> Vec<LigneCreneau> {
    let mut v: Vec<&CreneauGrilleDto> = grille.iter().collect();
    v.sort_by_key(|c| (c.heure_debut, c.ordre));
    v.into_iter()
        .map(|c| LigneCreneau {
            code: c.code.clone(),
            libelle: c.libelle.clone(),
            demi_journee: c.demi_journee.clone(),
            debut: heure_texte(c.heure_debut),
            fin: heure_texte(c.heure_fin),
        })
        .collect()
}

/// Compose le corps de `PUT /referentiel/grille` : heures valides, début avant fin, créneaux qui
/// ne se chevauchent pas, codes uniques. Champs : `creneau-{i}-{code|libelle|debut|fin}`.
pub fn composer_grille(lignes: &[LigneCreneau]) -> Result<Vec<CreneauGrilleSaisieDto>, Erreurs> {
    let mut e = Erreurs::new();
    if lignes.is_empty() {
        return Err(vec![erreur("grille", "Ajoutez au moins un créneau.")]);
    }
    let mut v = Vec::new();
    for (i, l) in lignes.iter().enumerate() {
        let champ = |c: &str| format!("creneau-{i}-{c}");
        let n = i + 1;
        if l.code.trim().is_empty() {
            e.push((
                champ("code"),
                format!("Créneau {n} : le code est obligatoire."),
            ));
        }
        if l.libelle.trim().is_empty() {
            e.push((
                champ("libelle"),
                format!("Créneau {n} : le libellé est obligatoire."),
            ));
        }
        let (d, f) = (lire_heure(&l.debut), lire_heure(&l.fin));
        if d.is_none() {
            e.push((
                champ("debut"),
                format!("Créneau {n} : heure de début invalide (HH:MM)."),
            ));
        }
        if f.is_none() {
            e.push((
                champ("fin"),
                format!("Créneau {n} : heure de fin invalide (HH:MM)."),
            ));
        }
        if !DEMI_JOURNEES.iter().any(|(k, _)| *k == l.demi_journee) {
            e.push((
                champ("demi"),
                format!("Créneau {n} : choisissez matin ou après-midi."),
            ));
        }
        if let (Some(d), Some(f)) = (d, f) {
            if f <= d {
                e.push((
                    champ("fin"),
                    format!("Créneau {n} : la fin doit suivre le début."),
                ));
            } else {
                v.push(CreneauGrilleSaisieDto {
                    code: l.code.trim().to_string(),
                    libelle: l.libelle.trim().to_string(),
                    demi_journee: l.demi_journee.clone(),
                    heure_debut: d,
                    heure_fin: f,
                });
            }
        }
    }
    if !e.is_empty() {
        return Err(e);
    }
    v.sort_by_key(|c| c.heure_debut);
    for w in v.windows(2) {
        if w[1].heure_debut < w[0].heure_fin {
            e.push(erreur(
                "grille",
                &format!(
                    "Les créneaux « {} » et « {} » se chevauchent.",
                    w[0].code, w[1].code
                ),
            ));
        }
    }
    let mut codes = std::collections::HashSet::new();
    if !v.iter().all(|c| codes.insert(c.code.clone())) {
        e.push(erreur("grille", "Deux créneaux ont le même code."));
    }
    if e.is_empty() {
        Ok(v)
    } else {
        Err(e)
    }
}

pub fn resume_seances(r: &RapportSeancesDto) -> String {
    let mut t = format!(
        "{} séance(s) créée(s), {} mise(s) à jour, {} supprimée(s) ; {} jour(s) neutralisé(s) par le calendrier.",
        r.creees, r.mises_a_jour, r.supprimees, r.jours_neutralises
    );
    if r.creneaux_ab_ignores > 0 {
        t.push_str(&format!(
            " {} créneau(x) en alternance A/B ignoré(s) faute de calendrier d'alternance.",
            r.creneaux_ab_ignores
        ));
    }
    t
}

/// Plage de génération facultative : les deux dates ou aucune.
pub fn composer_plage_generation(
    du: &str,
    au: &str,
) -> Result<(Option<NaiveDate>, Option<NaiveDate>), Erreurs> {
    match (du.trim().is_empty(), au.trim().is_empty()) {
        (true, true) => Ok((None, None)),
        (false, false) => match (lire_date(du), lire_date(au)) {
            (Some(d), Some(a)) if d <= a => Ok((Some(d), Some(a))),
            (Some(_), Some(_)) => Err(vec![erreur("gen-au", "La fin précède le début.")]),
            _ => Err(vec![erreur("gen-du", "Dates invalides.")]),
        },
        _ => Err(vec![erreur(
            "gen-au",
            "Renseignez les deux dates, ou aucune pour toute l'année.",
        )]),
    }
}

// ───────────────────────────── Sites, niveaux, MEF ─────────────────────────────

/// Site : code (40 car.) et libellé (120 car.) obligatoires. Champs `site-code`, `site-libelle`.
pub fn composer_site(code: &str, libelle: &str) -> Result<(String, String), Erreurs> {
    let (code, libelle) = (code.trim(), libelle.trim());
    let mut e = Erreurs::new();
    if code.is_empty() || code.chars().count() > 40 {
        e.push(erreur(
            "site-code",
            "Code obligatoire (40 caractères au plus).",
        ));
    }
    if libelle.is_empty() || libelle.chars().count() > 120 {
        e.push(erreur(
            "site-libelle",
            "Libellé obligatoire (120 caractères au plus).",
        ));
    }
    if e.is_empty() {
        Ok((code.to_string(), libelle.to_string()))
    } else {
        Err(e)
    }
}

/// Lit un fichier d'import MEF collé : une ligne par MEF, `code_mef;libellé[;niveau][;mefstat11]`
/// (séparateur « ; » ou tabulation). Retourne les lignes lues et les lignes illisibles.
pub fn lire_import_mef(texte: &str) -> (Vec<MefImportDto>, Vec<String>) {
    let mut lues = Vec::new();
    let mut rejetees = Vec::new();
    for (i, ligne) in texte.lines().enumerate() {
        let ligne = ligne.trim();
        if ligne.is_empty() || ligne.starts_with('#') {
            continue;
        }
        let champs: Vec<&str> = ligne.split([';', '\t']).map(str::trim).collect();
        let code = champs.first().copied().unwrap_or("");
        let libelle = champs.get(1).copied().unwrap_or("");
        let optionnel = |k: usize| {
            champs
                .get(k)
                .copied()
                .filter(|v| !v.is_empty())
                .map(str::to_string)
        };
        if code.is_empty() || libelle.is_empty() {
            rejetees.push(format!(
                "Ligne {} : code et libellé sont obligatoires.",
                i + 1
            ));
            continue;
        }
        lues.push(MefImportDto {
            code_mef: code.to_string(),
            libelle_long: libelle.to_string(),
            niveau_code: optionnel(2),
            mefstat11: optionnel(3),
        });
    }
    (lues, rejetees)
}

pub fn resume_import_mef(r: &RapportImportMefDto) -> String {
    let mut t = format!("{} MEF créé(s), {} mis à jour.", r.crees, r.mis_a_jour);
    if !r.rejets.is_empty() {
        t.push_str(&format!(" {} ligne(s) rejetée(s).", r.rejets.len()));
    }
    t
}

// ───────────────────────────── Classes et groupes ─────────────────────────────

/// Classes triées par libellé (ordre naturel : « 6A » avant « 6B »).
pub fn trier_classes_admin(mut classes: Vec<ClasseApi>) -> Vec<ClasseApi> {
    classes.sort_by_key(|a| a.libelle.to_lowercase());
    classes
}

/// Valeur d'un `<select>` « inchangé » (chaîne vide) → `None`, sinon l'identifiant.
pub fn choix_option(valeur: &str) -> Option<Uuid> {
    valeur.trim().parse().ok()
}

/// Libellé d'un groupe (le libellé s'il est renseigné, sinon le code).
pub fn libelle_groupe(g: &ent_types::referentiel_admin::GroupeAdminDto) -> String {
    if g.libelle.trim().is_empty() {
        g.code.clone()
    } else {
        g.libelle.clone()
    }
}

/// Responsable en cours de choix dans le dialogue d'un groupe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Responsable {
    pub user_id: Uuid,
    pub label: String,
}

/// Ajoute un responsable sans doublon.
pub fn ajouter_responsable(liste: &mut Vec<Responsable>, user_id: Uuid, label: &str) {
    if !liste.iter().any(|r| r.user_id == user_id) {
        liste.push(Responsable {
            user_id,
            label: label.to_string(),
        });
    }
}

pub fn retirer_responsable(liste: &mut Vec<Responsable>, user_id: Uuid) {
    liste.retain(|r| r.user_id != user_id);
}

/// Personnel proposé : hors des personnes déjà choisies.
pub fn filtrer_personnel(
    personnel: Vec<ent_types::referentiel_admin::PersonnelDto>,
    exclus: &[Uuid],
) -> Vec<ent_types::referentiel_admin::PersonnelDto> {
    personnel
        .into_iter()
        .filter(|p| !exclus.contains(&p.user_id))
        .collect()
}

/// Niveau correspondant à un identifiant, pour afficher son libellé.
pub fn libelle_niveau(
    niveaux: &[ent_types::referentiel_admin::NiveauDto],
    id: Option<Uuid>,
) -> String {
    id.and_then(|id| niveaux.iter().find(|n| n.id == id))
        .map(|n| n.libelle.clone())
        .unwrap_or_else(|| "—".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        lire_date(s).unwrap()
    }

    fn annee(statut: &str, debut: &str, fin: &str) -> AnneeStatutDto {
        AnneeStatutDto {
            id: Uuid::from_u128(debut.len() as u128 + d(debut).num_days_from_ce() as u128),
            libelle: format!("{}-{}", &debut[..4], &fin[..4]),
            date_debut: d(debut),
            date_fin: d(fin),
            courante: statut == "courante",
            statut: statut.into(),
        }
    }

    use chrono::Datelike;

    #[test]
    fn actions_selon_le_statut_de_l_annee() {
        let courante = annee("courante", "2026-09-01", "2027-07-04");
        let prepa = annee("preparation", "2027-09-01", "2028-07-04");
        let close = annee("cloturee", "2025-09-01", "2026-07-04");
        let toutes = vec![close.clone(), courante.clone(), prepa.clone()];
        let c = actions_annee(&courante, &toutes);
        assert!(
            c.cloturer && !c.ouvrir && !c.preparer_suivante,
            "déjà préparée"
        );
        assert!(actions_annee(&prepa, &toutes).ouvrir);
        let rien = actions_annee(&close, &toutes);
        assert!(!rien.ouvrir && !rien.cloturer && !rien.preparer_suivante);
        // Sans année suivante, on propose de la préparer.
        assert!(actions_annee(&courante, std::slice::from_ref(&courante)).preparer_suivante);
        assert_eq!(annee_suivante(&courante, &toutes).unwrap().id, prepa.id);
        assert_eq!(annee_courante(&toutes).unwrap().id, courante.id);
    }

    #[test]
    fn la_cible_suit_la_courante() {
        let courante = annee("courante", "2026-09-01", "2027-07-04");
        let ancienne = annee("preparation", "2025-09-01", "2026-07-04");
        let suivante = annee("preparation", "2027-09-01", "2028-07-04");
        assert!(annee_cible(&[courante.clone(), ancienne.clone()]).is_none());
        assert_eq!(
            annee_cible(&[ancienne, courante, suivante.clone()])
                .unwrap()
                .id,
            suivante.id
        );
        assert!(annee_cible(&[]).is_none());
    }

    #[test]
    fn etapes_de_la_bascule() {
        let courante = annee("courante", "2026-09-01", "2027-07-04");
        let prepa = annee("preparation", "2027-09-01", "2028-07-04");
        let e = etapes_bascule(std::slice::from_ref(&courante));
        assert_eq!(e[0].etat, EtatEtape::AFaire);
        assert_eq!(
            (e[1].etat, e[2].etat),
            (EtatEtape::Attente, EtatEtape::Attente)
        );
        let e = etapes_bascule(&[courante, prepa]);
        assert_eq!(e[0].etat, EtatEtape::Fait);
        assert_eq!(
            (e[1].etat, e[2].etat),
            (EtatEtape::AFaire, EtatEtape::AFaire)
        );
        assert_eq!(etapes_bascule(&[])[0].etat, EtatEtape::Attente);
        assert_eq!(EtatEtape::Fait.libelle(), "Fait");
    }

    #[test]
    fn annees_triees_et_libelle_suivant() {
        let v = trier_annees(vec![
            annee("cloturee", "2025-09-01", "2026-07-04"),
            annee("preparation", "2027-09-01", "2028-07-04"),
        ]);
        assert_eq!(v[0].libelle, "2027-2028");
        assert_eq!(
            libelle_annee_suivante("2026-2027").as_deref(),
            Some("2027-2028")
        );
        assert_eq!(libelle_annee_suivante("2026-2028"), None);
        assert_eq!(libelle_annee_suivante("abc"), None);
        assert_eq!(libelle_statut_annee("preparation"), "En préparation");
        assert_eq!(ton_statut_annee("courante"), TonEtat::Ok);
    }

    #[test]
    fn apercu_d_ouverture_decrit_les_effets() {
        let c = annee("courante", "2026-09-01", "2027-07-04");
        let p = annee("preparation", "2027-09-01", "2028-07-04");
        let t = apercu_ouverture(Some(&c), &p);
        assert!(t.contains("2026-2027") && t.contains("clôturée") && t.contains("2027-2028"));
        assert!(apercu_ouverture(None, &p).contains("aucune année"));
    }

    #[test]
    fn apercu_de_montee_et_conditions_d_application() {
        let r = RapportLotDto {
            apercu: true,
            a_creer: 12,
            deja_conformes: 3,
            conflits: vec![Uuid::nil()],
            ignores: vec![],
        };
        let lignes = resume_lot(&r);
        assert_eq!(lignes.len(), 3);
        assert!(lignes[2].contains("non modifié"));
        assert!(lot_applicable(Some(&r)));
        // Rien à créer, ou résultat déjà appliqué : pas de second bouton.
        assert!(!lot_applicable(Some(&RapportLotDto {
            a_creer: 0,
            ..r.clone()
        })));
        assert!(!lot_applicable(Some(&RapportLotDto { apercu: false, ..r })));
        assert!(!lot_applicable(None));
    }

    #[test]
    fn correspondances_proposees_pour_la_montee() {
        assert_eq!(code_classe_suivant("6A").as_deref(), Some("5A"));
        assert_eq!(code_classe_suivant("4B").as_deref(), Some("3B"));
        assert_eq!(code_classe_suivant("3A"), None, "sortie du collège");
        assert_eq!(code_classe_suivant("2NDE1").as_deref(), Some("1NDE1"));
        assert_eq!(code_classe_suivant("TERM"), None);
        let classe = |n: u128, code: &str| ClasseApi {
            id: Uuid::from_u128(n),
            annee_id: Uuid::nil(),
            code: code.into(),
            libelle: code.into(),
            niveau: None,
            effectif: 0,
            professeur_principal_id: None,
            professeur_principal_label: None,
        };
        let sources = [classe(1, "6A"), classe(2, "3A")];
        let cibles = [classe(10, "5a")];
        let p = proposer_correspondances(&sources, &cibles);
        assert_eq!(p[0], (Uuid::from_u128(1), Some(Uuid::from_u128(10))));
        assert_eq!(p[1], (Uuid::from_u128(2), None));
        let corps = correspondances_choisies(&p);
        assert_eq!(corps.len(), 1, "seules les classes avec une cible");
        assert_eq!(corps[0].cible, Uuid::from_u128(10));
    }

    #[test]
    fn periodes_modele_couvrent_l_annee_sans_trou() {
        let a = annee("preparation", "2026-09-01", "2027-07-04");
        for (kind, n) in [("trimestre", 3), ("semestre", 2)] {
            let lignes = periodes_modele(&a, kind, n);
            assert_eq!(lignes.len(), n as usize);
            let corps = composer_periodes(&lignes, &a).expect("le modèle est valide");
            assert_eq!(corps[0].date_debut, a.date_debut);
            assert_eq!(corps.last().unwrap().date_fin, a.date_fin);
        }
        assert!(periodes_modele(&a, "trimestre", 0).is_empty());
    }

    #[test]
    fn periodes_invalides_signalees_par_champ() {
        let a = annee("preparation", "2026-09-01", "2027-07-04");
        let mut l = periodes_modele(&a, "trimestre", 3);
        // Trou entre la 1re et la 2e.
        l[1].debut = "2026-12-20".into();
        let e = composer_periodes(&l, &a).unwrap_err();
        assert!(
            e.iter().any(|(c, m)| c == "periodes" && m.contains("trou")),
            "{e:?}"
        );
        // Chevauchement.
        let mut l = periodes_modele(&a, "trimestre", 3);
        l[1].debut = "2026-10-01".into();
        let e = composer_periodes(&l, &a).unwrap_err();
        assert!(e.iter().any(|(_, m)| m.contains("chevauchent")), "{e:?}");
        // Fin de l'année non couverte.
        let mut l = periodes_modele(&a, "trimestre", 3);
        l[2].fin = "2027-06-30".into();
        let e = composer_periodes(&l, &a).unwrap_err();
        assert!(e.iter().any(|(_, m)| m.contains("dernière période")));
        // Début non couvert, code en double, champs vides.
        let mut l = periodes_modele(&a, "trimestre", 3);
        l[0].debut = "2026-09-05".into();
        l[1].code = l[2].code.clone();
        assert!(composer_periodes(&l, &a).is_err());
        let mut l = periodes_modele(&a, "trimestre", 3);
        l[0].code = " ".into();
        l[1].fin = "n'importe quoi".into();
        let e = composer_periodes(&l, &a).unwrap_err();
        assert!(e.iter().any(|(c, _)| c == "periode-0-code"));
        assert!(e.iter().any(|(c, _)| c == "periode-1-fin"));
        assert!(composer_periodes(&[], &a).is_err());
    }

    #[test]
    fn lignes_de_periodes_triees_par_ordre() {
        let a = annee("courante", "2026-09-01", "2027-07-04");
        let mk = |ordre: i16, code: &str, debut: &str, fin: &str| {
            ent_types::referentiel_admin::PeriodeDto {
                id: Uuid::nil(),
                annee_id: a.id,
                niveau_id: None,
                code: code.into(),
                libelle: code.into(),
                kind: "trimestre".into(),
                ordre,
                date_debut: d(debut),
                date_fin: d(fin),
            }
        };
        let l = lignes_depuis_periodes(&[
            mk(2, "T2", "2026-12-01", "2027-03-01"),
            mk(1, "T1", "2026-09-01", "2026-11-30"),
        ]);
        assert_eq!(l[0].code, "T1");
        assert_eq!(libelle_kind_periode("semestre"), "Semestre");
    }

    #[test]
    fn plage_du_calendrier_validee() {
        let ok = SaisiePlage {
            du: "2026-12-19".into(),
            au: "2027-01-03".into(),
            kind: "vacances".into(),
            libelle: " Noël ".into(),
        };
        let p = composer_plage(&ok, &KINDS_VACANCES).unwrap();
        assert_eq!(p.libelle, "Noël");
        let inverse = SaisiePlage {
            au: "2026-12-01".into(),
            ..ok.clone()
        };
        assert!(composer_plage(&inverse, &KINDS_VACANCES).is_err());
        let sans_libelle = SaisiePlage {
            libelle: "  ".into(),
            ..ok.clone()
        };
        assert_eq!(
            composer_plage(&sans_libelle, &KINDS_VACANCES).unwrap_err()[0].0,
            "cal-libelle"
        );
        let mauvais_type = SaisiePlage {
            kind: "autre".into(),
            ..ok.clone()
        };
        assert!(composer_plage(&mauvais_type, &KINDS_JOUR).is_err());
        let trop_long = SaisiePlage {
            au: "2029-01-01".into(),
            ..ok
        };
        assert!(composer_plage(&trop_long, &KINDS_VACANCES).unwrap_err()[0]
            .1
            .contains("trop longue"));
    }

    #[test]
    fn calendrier_regroupe_les_jours_consecutifs() {
        let j = |jour: &str, kind: &str, lib: &str| JourCalendrierDto {
            jour: d(jour),
            kind: kind.into(),
            libelle: lib.into(),
            site_id: None,
            origine: "jour".into(),
        };
        let plages = regrouper_calendrier(&[
            j("2026-12-21", "vacances", "Noël"),
            j("2026-12-19", "vacances", "Noël"),
            j("2026-12-20", "vacances", "Noël"),
            j("2026-11-11", "ferie", "Armistice"),
            j("2026-12-25", "ferie", "Noël"),
        ]);
        assert_eq!(plages.len(), 3);
        assert_eq!(
            (plages[0].debut, plages[0].nb_jours()),
            (d("2026-11-11"), 1)
        );
        assert_eq!(
            (plages[1].debut, plages[1].fin),
            (d("2026-12-19"), d("2026-12-21"))
        );
        assert_eq!(libelle_kind_jour("ferme"), "Fermeture");
        assert_eq!(libelle_kind_jour("?"), "Jour neutralisé");
    }

    #[test]
    fn grille_validee_et_relue() {
        let l = |c: &str, dj: &str, a: &str, b: &str| LigneCreneau {
            code: c.into(),
            libelle: c.into(),
            demi_journee: dj.into(),
            debut: a.into(),
            fin: b.into(),
        };
        let ok = composer_grille(&[
            l("M2", "matin", "09:00", "10:00"),
            l("M1", "matin", "08:00", "09:00"),
        ])
        .unwrap();
        assert_eq!(ok[0].code, "M1", "triée par heure");
        // Chevauchement, fin avant début, heure invalide, demi-journée absente, doublon.
        assert!(composer_grille(&[
            l("A", "matin", "08:00", "09:00"),
            l("B", "matin", "08:30", "09:30")
        ])
        .unwrap_err()
        .iter()
        .any(|(_, m)| m.contains("chevauchent")));
        assert!(composer_grille(&[l("A", "matin", "09:00", "08:00")]).is_err());
        assert_eq!(
            composer_grille(&[l("A", "matin", "8h", "09:00")]).unwrap_err()[0].0,
            "creneau-0-debut"
        );
        assert!(composer_grille(&[l("A", "", "08:00", "09:00")]).is_err());
        assert!(composer_grille(&[
            l("A", "matin", "08:00", "09:00"),
            l("A", "matin", "09:00", "10:00")
        ])
        .unwrap_err()
        .iter()
        .any(|(_, m)| m.contains("même code")));
        assert!(composer_grille(&[]).is_err());
        let depuis = lignes_depuis_grille(&[CreneauGrilleDto {
            id: Uuid::nil(),
            site_id: None,
            code: "M1".into(),
            libelle: "M1".into(),
            demi_journee: "matin".into(),
            ordre: 1,
            heure_debut: lire_heure("08:00:00").unwrap(),
            heure_fin: lire_heure("09:00").unwrap(),
        }]);
        assert_eq!(
            (depuis[0].debut.as_str(), depuis[0].fin.as_str()),
            ("08:00", "09:00")
        );
    }

    #[test]
    fn generation_de_seances() {
        assert_eq!(composer_plage_generation("", " ").unwrap(), (None, None));
        let (a, b) = composer_plage_generation("2026-09-01", "2026-09-30").unwrap();
        assert_eq!((a, b), (Some(d("2026-09-01")), Some(d("2026-09-30"))));
        assert!(composer_plage_generation("2026-09-01", "").is_err());
        assert!(composer_plage_generation("2026-10-01", "2026-09-30").is_err());
        assert!(composer_plage_generation("x", "y").is_err());
        let t = resume_seances(&RapportSeancesDto {
            creees: 10,
            mises_a_jour: 2,
            supprimees: 1,
            jours_neutralises: 5,
            creneaux_ab_ignores: 3,
            ..Default::default()
        });
        assert!(t.contains("10 séance") && t.contains("alternance A/B"));
    }

    #[test]
    fn site_et_import_mef() {
        assert!(composer_site("", "x").is_err());
        assert_eq!(
            composer_site(" PRINC ", " Site principal ").unwrap().0,
            "PRINC"
        );
        assert!(composer_site(&"a".repeat(41), "x").is_err());
        let (lues, rejets) = lire_import_mef(
            "# MEF\n10110001110;6EME;6EME;1011\n10210002110;5EME\n;sans code\nseul\n\n",
        );
        assert_eq!(lues.len(), 2);
        assert_eq!(lues[0].niveau_code.as_deref(), Some("6EME"));
        assert_eq!(lues[0].mefstat11.as_deref(), Some("1011"));
        assert_eq!(lues[1].niveau_code, None);
        assert_eq!(rejets.len(), 2);
        let r = RapportImportMefDto {
            crees: 2,
            mis_a_jour: 1,
            rejets: vec![("X".into(), "doublon".into())],
        };
        assert!(resume_import_mef(&r).contains("1 ligne(s) rejetée(s)"));
        assert_eq!(libelle_niveau(&[], None), "—");
    }

    #[test]
    fn classes_et_groupes() {
        let classe = |lib: &str| ClasseApi {
            id: Uuid::nil(),
            annee_id: Uuid::nil(),
            code: lib.into(),
            libelle: lib.into(),
            niveau: None,
            effectif: 0,
            professeur_principal_id: None,
            professeur_principal_label: None,
        };
        let t = trier_classes_admin(vec![classe("6b"), classe("6A"), classe("5A")]);
        assert_eq!(
            t.iter().map(|c| c.libelle.as_str()).collect::<Vec<_>>(),
            ["5A", "6A", "6b"]
        );
        assert_eq!(choix_option(""), None);
        assert_eq!(choix_option("pas un uuid"), None);
        let id = Uuid::from_u128(7);
        assert_eq!(choix_option(&id.to_string()), Some(id));
        let g = ent_types::referentiel_admin::GroupeAdminDto {
            id,
            annee_id: id,
            code: "THEA".into(),
            libelle: " ".into(),
            classe_id: None,
            responsables: vec![],
        };
        assert_eq!(libelle_groupe(&g), "THEA");
        let mut r = Vec::new();
        ajouter_responsable(&mut r, id, "Rogue");
        ajouter_responsable(&mut r, id, "Rogue (doublon)");
        assert_eq!(r.len(), 1);
        retirer_responsable(&mut r, id);
        assert!(r.is_empty());
        let p = |n: u128| ent_types::referentiel_admin::PersonnelDto {
            user_id: Uuid::from_u128(n),
            email: "x@y.z".into(),
            role: "prof".into(),
            label: "X".into(),
        };
        let l = filtrer_personnel(vec![p(1), p(2)], &[Uuid::from_u128(1)]);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].user_id, Uuid::from_u128(2));
    }

    #[test]
    fn rapports_resumes() {
        let r = RapportReconductionDto {
            annee_cible_id: Uuid::nil(),
            classes_creees: 10,
            groupes_crees: 4,
            periodes_creees: 3,
            services_crees: 0,
        };
        assert!(resume_reconduction(&r).starts_with("10 classe(s)"));
        let o = RapportOuvertureDto {
            annee_id: Uuid::nil(),
            annee_cloturee_id: None,
            eleves_synchronises: 220,
        };
        assert!(resume_ouverture(&o).contains("220"));
    }
}
