//! Modèle de l'écran d'appel : logique pure, sans Leptos ni réseau (testable en natif).
//!
//! Aucune règle métier n'est dupliquée ici : qui peut faire l'appel, quels élèves
//! sont inscrits, ce que déclenche la clôture… restent décidés par `ent-api`.
//! Ce module ne gère que l'état d'écran : saisie en cours, filtres, sélection,
//! liste des changements à envoyer.

use ent_web_api::{ApiError, AppelApi, Capacites, EtatApi};
use uuid::Uuid;

/// Statut affiché pour un élève.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Statut {
    Present,
    Absent,
    Retard,
    Dispense,
}

impl Statut {
    pub const TOUS: [Statut; 4] = [
        Statut::Present,
        Statut::Absent,
        Statut::Retard,
        Statut::Dispense,
    ];

    pub fn libelle(self) -> &'static str {
        match self {
            Statut::Present => "Présent",
            Statut::Absent => "Absent",
            Statut::Retard => "Retard",
            Statut::Dispense => "Dispensé",
        }
    }

    pub fn libelle_pluriel(self) -> &'static str {
        match self {
            Statut::Present => "Présents",
            Statut::Absent => "Absents",
            Statut::Retard => "Retards",
            Statut::Dispense => "Dispensés",
        }
    }

    /// Identifiant court (classes CSS, attributs `value`).
    pub fn code(self) -> &'static str {
        match self {
            Statut::Present => "present",
            Statut::Absent => "absent",
            Statut::Retard => "retard",
            Statut::Dispense => "dispense",
        }
    }

    /// Raccourci clavier sur une ligne.
    pub fn touche(self) -> char {
        match self {
            Statut::Present => 'p',
            Statut::Absent => 'a',
            Statut::Retard => 'r',
            Statut::Dispense => 'd',
        }
    }

    pub fn depuis_touche(c: char) -> Option<Statut> {
        Statut::TOUS
            .into_iter()
            .find(|s| s.touche() == c.to_ascii_lowercase())
    }

    pub fn depuis_api(etat: &str) -> Option<Statut> {
        match etat {
            "present" => Some(Statut::Present),
            "absent" => Some(Statut::Absent),
            "retard" => Some(Statut::Retard),
            "dispense" => Some(Statut::Dispense),
            _ => None,
        }
    }

    /// Correspondance vers l'API. `None` si l'API ne sait pas l'enregistrer.
    pub fn vers_api(self) -> Option<EtatApi> {
        match self {
            Statut::Present => Some(EtatApi::Present),
            Statut::Absent => Some(EtatApi::Absent),
            Statut::Retard => Some(EtatApi::Retard),
            Statut::Dispense => None,
        }
    }

    /// Le statut est-il saisissable avec ces capacités d'API ?
    pub fn disponible(self, capacites: Capacites) -> bool {
        match self {
            Statut::Dispense => capacites.dispense,
            _ => true,
        }
    }
}

/// Minuscules sans accents, pour une recherche tolérante (« helene » trouve « Hélène »).
pub fn normaliser(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for c in texte.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => sortie.push('a'),
            'ç' => sortie.push('c'),
            'é' | 'è' | 'ê' | 'ë' => sortie.push('e'),
            'î' | 'ï' | 'í' | 'ì' => sortie.push('i'),
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => sortie.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => sortie.push('u'),
            'ÿ' | 'ý' => sortie.push('y'),
            'ñ' => sortie.push('n'),
            'œ' => sortie.push_str("oe"),
            'æ' => sortie.push_str("ae"),
            '-' | '\'' | '’' => sortie.push(' '),
            c => sortie.push(c),
        }
    }
    sortie
}

/// Vrai si tous les mots de la recherche apparaissent dans le nom (ordre libre).
pub fn correspond(cle_normalisee: &str, recherche: &str) -> bool {
    let r = normaliser(recherche);
    r.split_whitespace().all(|mot| cle_normalisee.contains(mot))
}

/// Données fixes d'un élève dans la feuille d'appel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EleveAppel {
    pub eleve_id: Uuid,
    pub nom_affiche: String,
    pub cle_recherche: String,
    /// Statut enregistré côté serveur (référence pour calculer les changements).
    pub statut_enregistre: Statut,
}

/// Feuille d'appel telle que reçue du serveur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Feuille {
    pub appel_id: Uuid,
    pub classe: String,
    pub matiere: Option<String>,
    pub jour: chrono::NaiveDate,
    pub close: bool,
    pub eleves: Vec<EleveAppel>,
}

impl Feuille {
    /// Construit la feuille, triée par nom (insensible aux accents et à la casse).
    pub fn depuis_api(appel: &AppelApi) -> Feuille {
        let mut eleves: Vec<EleveAppel> = appel
            .lignes
            .iter()
            .map(|l| {
                let nom = l
                    .eleve_label
                    .clone()
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| "Élève sans nom".to_string());
                EleveAppel {
                    eleve_id: l.eleve_id,
                    cle_recherche: normaliser(&nom),
                    nom_affiche: nom,
                    statut_enregistre: Statut::depuis_api(&l.etat).unwrap_or(Statut::Present),
                }
            })
            .collect();
        eleves.sort_by(|a, b| a.cle_recherche.cmp(&b.cle_recherche));
        Feuille {
            appel_id: appel.id,
            classe: appel.classe.clone(),
            matiere: appel.matiere_libelle.clone(),
            jour: appel.jour,
            close: appel.est_clos(),
            eleves,
        }
    }
}

/// Saisie en cours pour un élève.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saisie {
    pub eleve_id: Uuid,
    pub statut: Statut,
    pub commentaire: String,
}

/// Compteurs par statut.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Compteurs {
    pub present: usize,
    pub absent: usize,
    pub retard: usize,
    pub dispense: usize,
}

impl Compteurs {
    pub fn de(statuts: impl IntoIterator<Item = Statut>) -> Compteurs {
        let mut c = Compteurs::default();
        for s in statuts {
            match s {
                Statut::Present => c.present += 1,
                Statut::Absent => c.absent += 1,
                Statut::Retard => c.retard += 1,
                Statut::Dispense => c.dispense += 1,
            }
        }
        c
    }

    pub fn pour(&self, s: Statut) -> usize {
        match s {
            Statut::Present => self.present,
            Statut::Absent => self.absent,
            Statut::Retard => self.retard,
            Statut::Dispense => self.dispense,
        }
    }

    pub fn total(&self) -> usize {
        self.present + self.absent + self.retard + self.dispense
    }
}

/// Changement à envoyer au serveur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changement {
    pub eleve_id: Uuid,
    pub nom: String,
    pub etat: EtatApi,
}

/// Problème bloquant avant envoi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocage {
    /// Statut choisi que l'API ne sait pas enregistrer (ex. « dispensé »).
    StatutNonPrisEnCharge { nom: String, statut: Statut },
    /// Commentaire saisi alors que l'API ne les enregistre pas.
    CommentaireNonPrisEnCharge { nom: String },
}

/// Liste des changements par rapport à l'état enregistré, ou les blocages.
pub fn changements(
    eleves: &[EleveAppel],
    saisies: &[Saisie],
    capacites: Capacites,
) -> Result<Vec<Changement>, Vec<Blocage>> {
    let mut sortie = Vec::new();
    let mut blocages = Vec::new();
    for (eleve, saisie) in eleves.iter().zip(saisies) {
        debug_assert_eq!(eleve.eleve_id, saisie.eleve_id);
        if !saisie.commentaire.trim().is_empty() && !capacites.commentaire {
            blocages.push(Blocage::CommentaireNonPrisEnCharge {
                nom: eleve.nom_affiche.clone(),
            });
        }
        if saisie.statut == eleve.statut_enregistre {
            continue;
        }
        match saisie.statut.vers_api() {
            Some(etat) if saisie.statut.disponible(capacites) => sortie.push(Changement {
                eleve_id: eleve.eleve_id,
                nom: eleve.nom_affiche.clone(),
                etat,
            }),
            _ => blocages.push(Blocage::StatutNonPrisEnCharge {
                nom: eleve.nom_affiche.clone(),
                statut: saisie.statut,
            }),
        }
    }
    if blocages.is_empty() {
        Ok(sortie)
    } else {
        Err(blocages)
    }
}

/// Filtre d'affichage.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filtre {
    pub recherche: String,
    pub statut: Option<Statut>,
}

/// Indices des élèves visibles avec ce filtre.
pub fn visibles(eleves: &[EleveAppel], statuts: &[Statut], filtre: &Filtre) -> Vec<usize> {
    eleves
        .iter()
        .zip(statuts)
        .enumerate()
        .filter(|(_, (e, s))| {
            filtre.statut.is_none_or(|f| f == **s)
                && (filtre.recherche.trim().is_empty()
                    || correspond(&e.cle_recherche, &filtre.recherche))
        })
        .map(|(i, _)| i)
        .collect()
}

/// Lignes concernées par une action groupée : sélectionnées **et** affichées.
/// Une ligne sélectionnée puis masquée par la recherche ou un filtre n'est jamais
/// modifiée à l'insu de l'utilisateur (elle reste sélectionnée).
pub fn cibles_action_groupee(selection: &[bool], visibles: &[usize]) -> Vec<usize> {
    visibles
        .iter()
        .copied()
        .filter(|&i| selection.get(i).copied().unwrap_or(false))
        .collect()
}

/// Nombre de lignes sélectionnées mais masquées par le filtre courant.
pub fn selection_masquee(selection: &[bool], visibles: &[usize]) -> usize {
    let total = selection.iter().filter(|s| **s).count();
    total - cibles_action_groupee(selection, visibles).len()
}

/// Résultat d'un enregistrement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bilan {
    pub envoyes: usize,
    pub echecs: Vec<(String, String)>,
    pub clos: bool,
}

/// Erreur qui rend inutile (ou dangereux) de poursuivre l'envoi des lignes suivantes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arret {
    /// 401 : session expirée ou révoquée.
    SessionExpiree,
    /// 403 : droits insuffisants (les lignes suivantes seraient refusées aussi).
    Interdit,
    /// 429 : limitation de débit (insister aggraverait le blocage).
    TropDeRequetes,
}

/// Décision pure : faut-il interrompre l'envoi après cette erreur ? Les autres erreurs
/// (refus de validation d'une ligne, erreur serveur ponctuelle…) n'arrêtent pas la boucle :
/// elles sont listées ligne par ligne.
pub fn arret_apres(erreur: &ApiError) -> Option<Arret> {
    match erreur {
        ApiError::NonConnecte => Some(Arret::SessionExpiree),
        ApiError::Interdit(_) => Some(Arret::Interdit),
        ApiError::TropDeRequetes => Some(Arret::TropDeRequetes),
        _ => None,
    }
}

/// Message unique affiché quand l'envoi est interrompu ; `non_envoyees` = lignes jamais tentées.
pub fn message_arret(arret: Arret, non_envoyees: usize) -> String {
    let cause = match arret {
        Arret::SessionExpiree => "votre session a expiré",
        Arret::Interdit => "vous n'avez pas (ou plus) les droits pour modifier cet appel",
        Arret::TropDeRequetes => "trop de requêtes en peu de temps, patientez avant de réessayer",
    };
    let reste = match non_envoyees {
        0 => String::new(),
        n => format!(" {n} ligne(s) n'ont pas été tentées."),
    };
    format!("Envoi interrompu : {cause}.{reste} L'appel n'a pas été clos.")
}

/// Jour demandé par `?jour=` : `(jour, invalide)`. Absent → aujourd'hui sans avertissement ;
/// illisible ou hors 2000–2100 → aujourd'hui **avec** `invalide = true` (à signaler à l'écran).
pub fn jour_demande(
    parametre: Option<&str>,
    aujourdhui: chrono::NaiveDate,
) -> (chrono::NaiveDate, bool) {
    use chrono::Datelike;
    match parametre {
        None => (aujourdhui, false),
        Some(p) => match p.parse::<chrono::NaiveDate>() {
            Ok(d) if (2000..=2100).contains(&d.year()) => (d, false),
            _ => (aujourdhui, true),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use ent_web_api::{LigneAppelApi, CAPACITES_API_ACTUELLE};

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn appel(lignes: &[(&str, &str)]) -> AppelApi {
        AppelApi {
            id: id(999),
            classe: "4B".into(),
            jour: NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(),
            statut: "ouvert".into(),
            creneau_id: Some(id(1000)),
            matiere_libelle: Some("Potions".into()),
            lignes: lignes
                .iter()
                .enumerate()
                .map(|(i, (nom, etat))| LigneAppelApi {
                    eleve_id: id(i as u128 + 1),
                    eleve_label: Some(nom.to_string()),
                    etat: etat.to_string(),
                })
                .collect(),
        }
    }

    const TOUT: Capacites = Capacites {
        dispense: true,
        commentaire: true,
    };

    #[test]
    fn normalisation_sans_accents() {
        assert_eq!(normaliser("Hélène D'ARCY-Œuvre"), "helene d arcy oeuvre");
        assert!(correspond(&normaliser("GRANGER Hermione"), "herm gran"));
        assert!(!correspond(&normaliser("GRANGER Hermione"), "potter"));
    }

    #[test]
    fn feuille_triee_et_statuts_relus() {
        let f = Feuille::depuis_api(&appel(&[
            ("WEASLEY Ron", "retard"),
            ("ÉLÉONORE Zoé", "absent"),
            ("BLACK Sirius", "present"),
        ]));
        let noms: Vec<_> = f.eleves.iter().map(|e| e.nom_affiche.as_str()).collect();
        assert_eq!(noms, ["BLACK Sirius", "ÉLÉONORE Zoé", "WEASLEY Ron"]);
        assert_eq!(f.eleves[2].statut_enregistre, Statut::Retard);
        assert!(!f.close);
    }

    #[test]
    fn filtre_recherche_et_statut() {
        let f = Feuille::depuis_api(&appel(&[
            ("POTTER Harry", "present"),
            ("POTTER Lily", "absent"),
            ("MALFOY Drago", "absent"),
        ]));
        let statuts: Vec<_> = f.eleves.iter().map(|e| e.statut_enregistre).collect();
        let v = visibles(
            &f.eleves,
            &statuts,
            &Filtre {
                recherche: "potter".into(),
                statut: None,
            },
        );
        assert_eq!(v.len(), 2);
        let v = visibles(
            &f.eleves,
            &statuts,
            &Filtre {
                recherche: "".into(),
                statut: Some(Statut::Absent),
            },
        );
        assert_eq!(v.len(), 2);
        let v = visibles(
            &f.eleves,
            &statuts,
            &Filtre {
                recherche: "potter".into(),
                statut: Some(Statut::Absent),
            },
        );
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn changements_seulement_les_lignes_modifiees() {
        let f = Feuille::depuis_api(&appel(&[
            ("A a", "present"),
            ("B b", "present"),
            ("C c", "absent"),
        ]));
        let mut saisies: Vec<Saisie> = f
            .eleves
            .iter()
            .map(|e| Saisie {
                eleve_id: e.eleve_id,
                statut: e.statut_enregistre,
                commentaire: String::new(),
            })
            .collect();
        assert_eq!(
            changements(&f.eleves, &saisies, CAPACITES_API_ACTUELLE).unwrap(),
            vec![]
        );
        saisies[1].statut = Statut::Retard;
        saisies[2].statut = Statut::Present;
        let c = changements(&f.eleves, &saisies, CAPACITES_API_ACTUELLE).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].etat, EtatApi::Retard);
        assert_eq!(c[1].etat, EtatApi::Present);
    }

    #[test]
    fn dispense_et_commentaire_bloques_si_api_ne_les_gere_pas() {
        let f = Feuille::depuis_api(&appel(&[("A a", "present"), ("B b", "present")]));
        let mut saisies: Vec<Saisie> = f
            .eleves
            .iter()
            .map(|e| Saisie {
                eleve_id: e.eleve_id,
                statut: e.statut_enregistre,
                commentaire: String::new(),
            })
            .collect();
        saisies[0].statut = Statut::Dispense;
        saisies[1].commentaire = "infirmerie".into();
        let b = changements(&f.eleves, &saisies, CAPACITES_API_ACTUELLE).unwrap_err();
        assert_eq!(b.len(), 2);
        // Avec une API qui les gérerait, le commentaire ne bloque plus ; « dispensé »
        // n'a pas encore d'équivalent API côté client, il reste signalé.
        let b = changements(&f.eleves, &saisies, TOUT).unwrap_err();
        assert_eq!(
            b,
            vec![Blocage::StatutNonPrisEnCharge {
                nom: "A a".into(),
                statut: Statut::Dispense
            }]
        );
    }

    #[test]
    fn compteurs_et_touches() {
        let c = Compteurs::de([
            Statut::Present,
            Statut::Absent,
            Statut::Absent,
            Statut::Retard,
        ]);
        assert_eq!(
            (c.present, c.absent, c.retard, c.dispense, c.total()),
            (1, 2, 1, 0, 4)
        );
        assert_eq!(Statut::depuis_touche('A'), Some(Statut::Absent));
        assert_eq!(Statut::depuis_touche('x'), None);
        assert!(!Statut::Dispense.disponible(CAPACITES_API_ACTUELLE));
    }

    /// Non-régression (constaté dans le navigateur le 2026-11-02) : une action groupée
    /// modifiait aussi un élève sélectionné puis masqué par la recherche.
    #[test]
    fn action_groupee_ignore_les_eleves_masques_par_la_recherche() {
        let f = Feuille::depuis_api(&appel(&[
            ("ROUX Adèle", "present"),
            ("BERTRAND Anaïs", "present"),
            ("MICHEL Anaïs", "present"),
        ]));
        let statuts: Vec<_> = f.eleves.iter().map(|e| e.statut_enregistre).collect();
        // Tri : BERTRAND (0), MICHEL (1), ROUX (2). ROUX est sélectionné, puis on cherche « anais ».
        let mut selection = vec![false, false, true];
        let v = visibles(
            &f.eleves,
            &statuts,
            &Filtre {
                recherche: "anais".into(),
                statut: None,
            },
        );
        assert_eq!(v, vec![0, 1]);
        // « Tout sélectionner » ne coche que les lignes affichées.
        for &i in &v {
            selection[i] = true;
        }
        assert_eq!(
            cibles_action_groupee(&selection, &v),
            vec![0, 1],
            "ROUX, masqué, n'est pas concerné"
        );
        assert_eq!(selection_masquee(&selection, &v), 1);
        // Sans filtre, la même sélection concerne bien les trois.
        let tous = visibles(&f.eleves, &statuts, &Filtre::default());
        assert_eq!(cibles_action_groupee(&selection, &tous), vec![0, 1, 2]);
        assert_eq!(selection_masquee(&selection, &tous), 0);
    }

    #[test]
    fn classe_de_400_eleves_filtree_rapidement() {
        let noms: Vec<String> = (0..400).map(|i| format!("NOM{i:03} Prénom{i}")).collect();
        let lignes: Vec<(&str, &str)> = noms.iter().map(|n| (n.as_str(), "present")).collect();
        let f = Feuille::depuis_api(&appel(&lignes));
        let statuts: Vec<_> = f.eleves.iter().map(|e| e.statut_enregistre).collect();
        let debut = std::time::Instant::now();
        for _ in 0..100 {
            let v = visibles(
                &f.eleves,
                &statuts,
                &Filtre {
                    recherche: "prenom12".into(),
                    statut: None,
                },
            );
            assert_eq!(v.len(), 11);
        }
        // 100 filtrages de 400 lignes : très en dessous d'une image (16 ms) en natif.
        assert!(debut.elapsed().as_millis() < 500, "{:?}", debut.elapsed());
    }

    #[test]
    fn arret_de_l_envoi_selon_l_erreur() {
        assert_eq!(
            arret_apres(&ApiError::NonConnecte),
            Some(Arret::SessionExpiree)
        );
        assert_eq!(
            arret_apres(&ApiError::Interdit("x".into())),
            Some(Arret::Interdit)
        );
        assert_eq!(
            arret_apres(&ApiError::TropDeRequetes),
            Some(Arret::TropDeRequetes)
        );
        assert_eq!(arret_apres(&ApiError::Invalide("x".into())), None);
        assert_eq!(arret_apres(&ApiError::Conflit("x".into())), None);
        assert_eq!(arret_apres(&ApiError::Serveur(500, "x".into())), None);
        assert_eq!(arret_apres(&ApiError::Reseau("x".into())), None);
        let m = message_arret(Arret::Interdit, 3);
        assert!(m.contains("droits") && m.contains("3 ligne(s)") && m.contains("pas été clos"));
        assert!(!message_arret(Arret::TropDeRequetes, 0).contains("ligne(s)"));
    }

    #[test]
    fn jour_demande_signale_les_valeurs_invalides() {
        let auj = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
        let j = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        assert_eq!(jour_demande(None, auj), (auj, false));
        assert_eq!(jour_demande(Some("2026-11-05"), auj), (j, false));
        assert_eq!(jour_demande(Some("demain"), auj), (auj, true));
        assert_eq!(jour_demande(Some("2026-13-45"), auj), (auj, true));
        assert_eq!(jour_demande(Some(""), auj), (auj, true));
        assert_eq!(jour_demande(Some("9999-01-01"), auj), (auj, true));
    }
}
