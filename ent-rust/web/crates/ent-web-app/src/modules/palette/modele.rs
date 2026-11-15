//! Logique d'écran pure de la palette de commandes (lot P6 du socle 29 ; sans Leptos ni réseau).
//! Aucune règle métier : les droits viennent du moteur D-017 déjà chargé par la coquille (affichage), le
//! serveur reste juge de chaque écran ; les capacités viennent du registre `ent_types::palette`.

use ent_types::palette::Capacite;
use ent_types::recherche::ResultatRecherche;

/// Résultats par famille (la liste reste lisible ; on affine plutôt que de défiler).
pub const MAX_PAR_FAMILLE: usize = 8;
/// Entrées « récentes » retenues sur l'appareil, **en mémoire** (jamais côté serveur, POR-201).
pub const MAX_RECENTS: usize = 5;
/// Longueur minimale d'une recherche « Trouver » (même règle que le serveur, 29 §12.1).
pub const MIN_RECHERCHE: usize = 2;

/// Famille de résultats (29 §12.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Famille {
    Recent,
    AllerA,
    Faire,
    Trouver,
}

impl Famille {
    pub fn titre(self) -> &'static str {
        match self {
            Self::Recent => "Récents",
            Self::AllerA => "Aller à",
            Self::Faire => "Faire",
            Self::Trouver => "Trouver",
        }
    }
}

/// Une ligne de la palette : un libellé et où aller. Jamais d'exécution directe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntreePalette {
    pub famille: Famille,
    pub libelle: String,
    pub lien: String,
}

/// Minuscules sans accents (la saisie « presence » trouve « Présence »).
pub fn normaliser(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ÿ' | 'ý' => 'y',
            c => c,
        })
        .collect()
}

/// Rang de correspondance (plus petit = plus haut), ou `None`. Chaque mot de la saisie doit se retrouver :
/// libellé qui commence par la saisie (0), début d'un mot du libellé (1), alias ou fragment (2).
pub fn rang(requete: &str, libelle: &str, alias: &[&str]) -> Option<u8> {
    let q = normaliser(requete);
    let q = q.trim();
    if q.is_empty() {
        return Some(3);
    }
    let lib = normaliser(libelle);
    if lib.starts_with(q) {
        return Some(0);
    }
    let alias: Vec<String> = alias.iter().map(|a| normaliser(a)).collect();
    let mots: Vec<&str> = q.split_whitespace().collect();
    let tous_mots_debut = mots.iter().all(|m| {
        lib.split(|c: char| !c.is_alphanumeric())
            .any(|w| w.starts_with(m))
    });
    if tous_mots_debut {
        return Some(1);
    }
    let dans = |m: &str| lib.contains(m) || alias.iter().any(|a| a.contains(m));
    mots.iter().all(|m| dans(m)).then_some(2)
}

fn borner(mut v: Vec<(u8, EntreePalette)>) -> Vec<EntreePalette> {
    v.sort_by_key(|(r, _)| *r);
    v.into_iter()
        .take(MAX_PAR_FAMILLE)
        .map(|(_, e)| e)
        .collect()
}

/// Les familles « Aller à » et « Faire » pour une saisie. `pages` = (libellé, lien) de la **même** liste que
/// la navigation de l'écran ; `capacites` = résultat de `capacites_permises` sous les droits de la personne.
/// Saisie vide : les pages, puis les actions ; jamais de « Trouver » sans saisie.
pub fn construire(
    requete: &str,
    pages: &[(String, String)],
    capacites: &[&Capacite],
) -> Vec<EntreePalette> {
    let aller: Vec<(u8, EntreePalette)> = pages
        .iter()
        .filter_map(|(l, h)| {
            rang(requete, l, &[]).map(|r| {
                (
                    r,
                    EntreePalette {
                        famille: Famille::AllerA,
                        libelle: l.clone(),
                        lien: h.clone(),
                    },
                )
            })
        })
        .collect();
    let faire: Vec<(u8, EntreePalette)> = capacites
        .iter()
        .filter_map(|c| {
            rang(requete, c.libelle, c.alias).map(|r| {
                (
                    r,
                    EntreePalette {
                        famille: Famille::Faire,
                        libelle: c.libelle.to_string(),
                        lien: c.ecran.to_string(),
                    },
                )
            })
        })
        .collect();
    // Saisie vide : « Aller à » est toute la navigation (c'est elle, la liste de l'écran) ; en tapant, on borne.
    let aller = if requete.trim().is_empty() {
        let mut a = aller;
        a.sort_by_key(|(r, _)| *r);
        a.into_iter().map(|(_, e)| e).collect()
    } else {
        borner(aller)
    };
    let mut v = borner(faire);
    v.extend(aller);
    // « Faire » avant « Aller à » quand on tape : l'action demandée prime sur la page qui la porte.
    v.sort_by_key(|e| match e.famille {
        Famille::Faire => 0,
        _ => 1,
    });
    v
}

/// Résultats de la recherche globale (P5) rendus comme entrées « Trouver » ; seuls ceux qui ont un écran.
pub fn trouver(resultats: &[ResultatRecherche]) -> Vec<EntreePalette> {
    resultats
        .iter()
        .filter_map(|r| {
            r.lien.as_ref().map(|l| EntreePalette {
                famille: Famille::Trouver,
                libelle: r.libelle.clone(),
                lien: l.clone(),
            })
        })
        .take(MAX_PAR_FAMILLE)
        .collect()
}

/// La saisie justifie-t-elle une recherche côté serveur ? (compte pour le plafond du jour : pas avant.)
pub fn recherche_utile(saisie: &str) -> Option<String> {
    let t = saisie.trim();
    (t.chars().count() >= MIN_RECHERCHE).then(|| t.chars().take(100).collect())
}

/// Ajoute une entrée aux récents (la plus récente en tête, sans doublon, bornés). Un résultat « Trouver »
/// n'y entre jamais : il désigne une personne, et les récents ne doivent contenir aucune donnée d'élève.
pub fn memoriser(recents: &mut Vec<EntreePalette>, e: &EntreePalette) {
    if e.famille == Famille::Trouver {
        return;
    }
    recents.retain(|r| r.lien != e.lien);
    recents.insert(
        0,
        EntreePalette {
            famille: Famille::Recent,
            ..e.clone()
        },
    );
    recents.truncate(MAX_RECENTS);
}

/// Annonce `aria-live` du nombre de résultats.
pub fn annonce(n: usize) -> String {
    match n {
        0 => "Aucun résultat.".to_string(),
        1 => "1 résultat.".to_string(),
        n => format!("{n} résultats."),
    }
}

/// Ctrl + K ou ⌘ + K ouvre la palette (aucun autre modificateur) ; sans souris, même pendant une saisie.
pub fn ouvre_la_palette(touche: &str, ctrl: bool, meta: bool, alt: bool, maj: bool) -> bool {
    (ctrl || meta) && !alt && !maj && touche.eq_ignore_ascii_case("k")
}

/// Index suivant dans la liste (circulaire) ; `None` quand la liste est vide.
pub fn deplacer(courant: Option<usize>, delta: i32, n: usize) -> Option<usize> {
    if n == 0 {
        return None;
    }
    let n = n as i32;
    let base = courant.map_or(if delta > 0 { -1 } else { 0 }, |c| c as i32);
    Some((base + delta).rem_euclid(n) as usize)
}

/// Raccourcis documentés (POR-202) : (touches, effet).
pub const RACCOURCIS: &[(&str, &str)] = &[
    ("Ctrl K  /  ⌘ K", "Ouvrir la palette"),
    ("↑  ↓", "Choisir un résultat"),
    ("Entrée", "Ouvrir le résultat choisi"),
    ("Échap", "Fermer la palette"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use ent_types::palette::{capacites_permises, CAPACITES};
    use ent_types::recherche::SourceRecherche;
    use uuid::Uuid;

    fn pages() -> Vec<(String, String)> {
        vec![
            ("Aujourd'hui".into(), "/web".into()),
            ("Journée".into(), "/web/journee".into()),
            ("Appel".into(), "/web/appel".into()),
            ("Messages".into(), "/web/messages".into()),
        ]
    }

    #[test]
    fn les_accents_et_la_casse_ne_comptent_pas() {
        assert_eq!(normaliser("Présence À l'ÉCOLE"), "presence a l'ecole");
        assert_eq!(rang("PRESENCE", "Faire l'appel", &["présence"]), Some(2));
        assert_eq!(rang("faire", "Faire l'appel", &[]), Some(0));
        assert_eq!(rang("appel", "Faire l'appel", &[]), Some(1));
        assert_eq!(rang("zzz", "Faire l'appel", &["appel"]), None);
    }

    /// POR-200 : « abs » trouve « Déclarer une absence » par son alias.
    #[test]
    fn un_alias_trouve_l_action() {
        let tout = capacites_permises(|_| true);
        let v = construire("abs", &pages(), &tout);
        assert!(v
            .iter()
            .any(|e| e.famille == Famille::Faire && e.libelle == "Déclarer une absence"));
    }

    /// POR-198 / POR-199 : sans droit, aucune action ; les pages restent celles de la navigation.
    #[test]
    fn sans_droit_aucune_action_et_les_pages_de_l_ecran() {
        let v = construire("", &pages(), &[]);
        assert!(v.iter().all(|e| e.famille == Famille::AllerA));
        assert_eq!(v.len(), 4);
        let v = construire("jour", &pages(), &[]);
        let liens: Vec<_> = v.iter().map(|e| e.lien.as_str()).collect();
        assert_eq!(
            liens,
            ["/web/journee", "/web"],
            "début de libellé avant fragment"
        );
    }

    #[test]
    fn faire_passe_avant_aller_a_et_chaque_famille_est_bornee() {
        let tout = capacites_permises(|_| true);
        let v = construire("a", &pages(), &tout);
        let premiere_page = v.iter().position(|e| e.famille == Famille::AllerA);
        let derniere_action = v.iter().rposition(|e| e.famille == Famille::Faire);
        if let (Some(p), Some(a)) = (premiere_page, derniere_action) {
            assert!(a < p);
        }
        assert!(v.iter().filter(|e| e.famille == Famille::Faire).count() <= MAX_PAR_FAMILLE);
        assert!(CAPACITES.len() > MAX_PAR_FAMILLE, "la borne doit servir");
    }

    #[test]
    fn la_recherche_serveur_attend_deux_caracteres() {
        assert_eq!(recherche_utile(" a "), None);
        assert_eq!(recherche_utile(" du "), Some("du".into()));
        assert_eq!(
            recherche_utile(&"x".repeat(300)).unwrap().chars().count(),
            100
        );
    }

    #[test]
    fn trouver_garde_ce_qui_a_un_ecran() {
        let r = |lien: Option<&str>| ResultatRecherche {
            source: SourceRecherche::Eleves,
            id: Uuid::nil(),
            libelle: "DUPONT Léa — 4B".into(),
            lien: lien.map(str::to_string),
        };
        let v = trouver(&[r(Some("/web/eleves/x")), r(None)]);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].famille, Famille::Trouver);
    }

    /// POR-201 : récents sur l'appareil, sans doublon, bornés, jamais une personne trouvée.
    #[test]
    fn les_recents_sont_bornes_sans_doublon_et_sans_personne() {
        let mut r = vec![];
        let e = |l: &str, f| EntreePalette {
            famille: f,
            libelle: l.into(),
            lien: format!("/web/{l}"),
        };
        for i in 0..8 {
            memoriser(&mut r, &e(&format!("p{i}"), Famille::AllerA));
        }
        memoriser(&mut r, &e("p7", Famille::Faire));
        memoriser(&mut r, &e("eleve", Famille::Trouver));
        assert_eq!(r.len(), MAX_RECENTS);
        assert_eq!(r[0].lien, "/web/p7");
        assert_eq!(r.iter().filter(|x| x.lien == "/web/p7").count(), 1);
        assert!(r
            .iter()
            .all(|x| x.famille == Famille::Recent && x.lien != "/web/eleve"));
    }

    #[test]
    fn le_raccourci_d_ouverture() {
        assert!(ouvre_la_palette("k", true, false, false, false));
        assert!(ouvre_la_palette("K", false, true, false, false));
        assert!(!ouvre_la_palette("k", false, false, false, false));
        assert!(!ouvre_la_palette("k", true, false, true, false));
        assert!(!ouvre_la_palette("j", true, false, false, false));
    }

    #[test]
    fn le_deplacement_est_circulaire() {
        assert_eq!(deplacer(None, 1, 0), None);
        assert_eq!(deplacer(None, 1, 3), Some(0));
        assert_eq!(deplacer(None, -1, 3), Some(2));
        assert_eq!(deplacer(Some(2), 1, 3), Some(0));
        assert_eq!(deplacer(Some(0), -1, 3), Some(2));
    }

    /// POR-244 (palette / écran) : toute capacité « Faire » ouvre une route réellement déclarée par l'application.
    #[test]
    fn chaque_capacite_ouvre_une_route_existante() {
        let routes = include_str!("../../lib.rs");
        for c in CAPACITES {
            let reste = c.ecran.strip_prefix("/web/").expect(c.id);
            assert!(
                routes.contains(&format!("path!(\"{reste}\")")),
                "{} : route {} absente de l'application",
                c.id,
                c.ecran
            );
        }
    }

    /// POR-244 (palette / écran) : pour un profil et ses droits, une action dont l'écran figure dans la
    /// navigation n'est proposée que si la navigation le propose aussi (jamais un écran que le menu cache).
    #[test]
    fn palette_et_navigation_s_accordent() {
        use crate::noyau::droits::droits_du_role;
        use crate::noyau::droits_moteur::DroitsMoteur;
        use crate::noyau::navigation::entrees_avec_moteur;
        use ent_web_api::DroitEffectif;
        let moteur = |codes: &[&str]| {
            DroitsMoteur::new(
                codes
                    .iter()
                    .map(|c| DroitEffectif {
                        action: (*c).into(),
                        portee: "etablissement".into(),
                        cibles: Vec::new(),
                    })
                    .collect(),
            )
        };
        let cas: [(&str, &[&str]); 4] = [
            (
                "prof",
                &[
                    "appel.faire",
                    "cahier.ecrire",
                    "notes.saisir",
                    "recherche.globale",
                ],
            ),
            (
                "cpe",
                &[
                    "absences.declarer",
                    "punitions.donner",
                    "assiduite.signaler",
                    "edt.gerer",
                    "appel.faire",
                ],
            ),
            (
                "administratif",
                &[
                    "inscriptions.gerer",
                    "campagnes.lancer",
                    "formulaires.gerer",
                    "rencontres.gerer",
                    "afaire.file.prendre",
                ],
            ),
            (
                "admin",
                &["roles.gerer", "delegations.gerer", "inscriptions.gerer"],
            ),
        ];
        for (role, codes) in cas {
            let m = moteur(codes);
            let nav: Vec<String> = entrees_avec_moteur(droits_du_role(role), &m)
                .into_iter()
                .map(|e| e.href)
                .collect();
            let offertes = capacites_permises(|d| m.a(d));
            assert!(!offertes.is_empty(), "{role} : aucune action");
            for c in offertes {
                let est_une_page = crate::noyau::navigation::entrees_avec_moteur(
                    droits_du_role("admin"),
                    &moteur(&[
                        "roles.gerer",
                        "inscriptions.gerer",
                        "campagnes.lancer",
                        "formulaires.gerer",
                        "rencontres.gerer",
                        "afaire.file.prendre",
                        "recherche.globale",
                        "absences.declarer",
                        "punitions.donner",
                        "assiduite.signaler",
                        "edt.gerer",
                        "appel.faire",
                        "notes.saisir",
                        "cahier.ecrire",
                        "delegations.gerer",
                    ]),
                )
                .iter()
                .any(|e| e.href == c.ecran);
                if est_une_page {
                    assert!(
                        nav.contains(&c.ecran.to_string()),
                        "{role} : « {} » proposée par la palette mais {} absent de la navigation {nav:?}",
                        c.libelle,
                        c.ecran
                    );
                }
            }
            // « Aller à » = la navigation, mot pour mot, saisie vide.
            let pages: Vec<(String, String)> = entrees_avec_moteur(droits_du_role(role), &m)
                .into_iter()
                .map(|e| (e.libelle, e.href))
                .collect();
            let aller: Vec<String> = construire("", &pages, &[])
                .into_iter()
                .map(|e| e.lien)
                .collect();
            assert_eq!(aller.len(), nav.len(), "{role}");
        }
    }

    #[test]
    fn annonces() {
        assert_eq!(annonce(0), "Aucun résultat.");
        assert_eq!(annonce(1), "1 résultat.");
        assert_eq!(annonce(7), "7 résultats.");
        assert!(!RACCOURCIS.is_empty());
    }
}
