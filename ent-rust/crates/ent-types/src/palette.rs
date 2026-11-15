//! Socle 29, lot P6 : registre des capacités consommé par la palette de commandes
//! (`docs/socles/29_PORTAILS.md` §12.2, POR-197 à POR-202, POR-244 ; `docs/socles/IA_COEUR.md` §4.3, §5.2).
//!
//! Le registre est **statique et compilé** : aucun établissement, aucune donnée ne peut y ajouter une
//! entrée. Il ne contient **aucune logique métier** : chaque capacité nomme les droits D-017 requis
//! (tous), l'écran qui porte l'action et son niveau de confirmation. La palette et l'assistant (socle 30,
//! quand le cœur IA existera) lisent **la même fonction** `capacites_permises`, donc le même ensemble
//! pour les mêmes droits : c'est l'équivalence palette / écran / IA (POR-244).
//!
//! Une capacité « Faire » ne s'exécute jamais depuis la palette : elle ouvre l'écran, qui reste maître et
//! revérifie les droits côté serveur. L'aperçu, la confirmation et l'idempotence du cycle IA (§6.1) seront
//! branchés sur ce même registre par le lot IA-L0 ; en attendant, `ouverte_ia` n'est qu'une déclaration.

/// Genre d'une capacité (IA_COEUR §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenreCapacite {
    /// Sans effet de bord.
    Lecture,
    /// Effet métier : se prépare, puis s'exécute sur confirmation humaine.
    Commande,
}

/// Niveau de confirmation d'une commande (IA_COEUR §6.2). Un établissement peut le monter, jamais le descendre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confirmation {
    /// Pas d'écriture.
    N0,
    /// Écriture interne, réversible, un objet.
    N1,
    /// Écriture touchant des élèves ou une donnée suivie : détail élément par élément.
    N2,
    /// Sensible, externe, difficilement réversible : ré-authentification récente.
    N3,
    /// Réservée à l'écran : l'IA ne la prépare jamais.
    FermeeIa,
}

/// Une capacité déclarée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacite {
    /// Identifiant stable (`absence.declarer`).
    pub id: &'static str,
    pub genre: GenreCapacite,
    /// Socle propriétaire.
    pub socle: u8,
    /// Libellé court, à l'infinitif (« Déclarer une absence »).
    pub libelle: &'static str,
    /// Mots-clés et abréviations (« abs ») ; jamais de langage naturel (c'est l'assistant).
    pub alias: &'static [&'static str],
    /// Droits D-017 **tous** requis. Vide interdit (test de garde).
    pub droits: &'static [&'static str],
    /// Écran qui porte l'action (chemin web).
    pub ecran: &'static str,
    pub confirmation: Confirmation,
    /// L'assistant peut-il la préparer (en plus de l'écran) ? Jamais pour `FermeeIa`.
    pub ouverte_ia: bool,
}

#[allow(clippy::too_many_arguments)]
const fn cap(
    id: &'static str,
    genre: GenreCapacite,
    socle: u8,
    libelle: &'static str,
    alias: &'static [&'static str],
    droits: &'static [&'static str],
    ecran: &'static str,
    confirmation: Confirmation,
    ouverte_ia: bool,
) -> Capacite {
    Capacite {
        id,
        genre,
        socle,
        libelle,
        alias,
        droits,
        ecran,
        confirmation,
        ouverte_ia,
    }
}

use Confirmation::*;
use GenreCapacite::*;

/// Le registre. Les droits sont les codes du catalogue D-017 (`ent_auth::Action`), vérifiés par un test.
#[rustfmt::skip]
pub const CAPACITES: &[Capacite] = &[
    cap("appel.faire", Commande, 12, "Faire l'appel", &["appel", "presence", "présence"], &["appel.faire"], "/web/appel", N2, true),
    cap("absence.declarer", Commande, 12, "Déclarer une absence", &["abs", "absent", "retard"], &["absences.declarer"], "/web/journee", N2, true),
    cap("assiduite.signaler", Commande, 12, "Signaler un problème d'assiduité", &["signalement", "assiduite"], &["assiduite.signaler"], "/web/signalements", N2, true),
    cap("cahier.ecrire", Commande, 2, "Écrire au cahier de textes", &["cahier", "devoirs", "travail"], &["cahier.ecrire"], "/web/cahier", N1, true),
    cap("notes.saisir", Commande, 21, "Saisir des notes", &["notes", "evaluation", "devoir"], &["notes.saisir"], "/web/evaluations", N2, true),
    cap("salle.reserver", Commande, 19, "Réserver une salle", &["salle", "reservation", "réservation"], &["agenda.lieux.reserver"], "/web/salles", N1, true),
    cap("remplacement.organiser", Commande, 10, "Organiser un remplacement", &["remplacement", "absent prof", "edt"], &["edt.gerer"], "/web/edt/remplacements", N2, true),
    cap("punition.donner", Commande, 13, "Donner une punition", &["punition", "sanction", "retenue"], &["punitions.donner"], "/web/discipline", N3, false),
    cap("inscription.traiter", Commande, 3, "Traiter une inscription", &["inscription", "dossier", "admission"], &["inscriptions.gerer"], "/web/inscriptions", N2, false),
    cap("voyage.preparer", Commande, 42, "Préparer une sortie ou un voyage", &["sortie", "voyage"], &["voyages.creer"], "/web/voyages/nouveau", N2, true),
    cap("campagne.lancer", Commande, 8, "Lancer une campagne d'envoi", &["campagne", "envoi", "diffusion"], &["campagnes.lancer"], "/web/campagnes", N3, false),
    cap("formulaire.gerer", Commande, 8, "Gérer les formulaires à signer", &["formulaire", "signature", "autorisation"], &["formulaires.gerer"], "/web/formulaires", N2, false),
    cap("rencontre.organiser", Commande, 19, "Organiser une rencontre parents-professeurs", &["rencontre", "rdv", "rendez-vous"], &["rencontres.gerer"], "/web/rencontres", N3, false),
    cap("tache.prendre", Commande, 29, "Prendre une tâche en charge", &["tache", "tâche", "file", "a faire"], &["afaire.file.prendre"], "/web/mes-files", N1, true),
    cap("recherche.lancer", Lecture, 29, "Rechercher dans l'ENT", &["chercher", "trouver", "recherche"], &["recherche.globale"], "/web/recherche", N0, true),
    cap("delegation.gerer", Commande, 6, "Déléguer mes droits", &["delegation", "délégation", "remplaçant"], &["delegations.gerer"], "/web/delegations", FermeeIa, false),
    cap("roles.gerer", Commande, 1, "Gérer les rôles et les droits", &["roles", "rôles", "droits"], &["roles.gerer"], "/web/direction/roles", FermeeIa, false),
];

/// Port `capacites(acteur)` (29 §6.3) : les capacités dont **tous** les droits sont détenus. `a_droit` est
/// l'unique prédicat de droits (moteur D-017 côté écran, droits effectifs côté serveur). La palette et
/// l'assistant passent ici : même acteur, même ensemble.
pub fn capacites_permises(a_droit: impl Fn(&str) -> bool) -> Vec<&'static Capacite> {
    CAPACITES
        .iter()
        .filter(|c| !c.droits.is_empty() && c.droits.iter().all(|d| a_droit(d)))
        .collect()
}

/// Sous-ensemble que l'assistant peut préparer : toujours inclus dans `capacites_permises`.
pub fn capacites_ouvertes_ia(a_droit: impl Fn(&str) -> bool) -> Vec<&'static Capacite> {
    capacites_permises(a_droit)
        .into_iter()
        .filter(|c| c.ouverte_ia && c.confirmation != FermeeIa)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn aucune_capacite_sans_droit_ni_doublon() {
        let mut ids = HashSet::new();
        for c in CAPACITES {
            assert!(!c.droits.is_empty(), "{} sans droit", c.id);
            assert!(ids.insert(c.id), "{} en double", c.id);
            assert!(c.ecran.starts_with("/web/"), "{}", c.id);
            assert!(!c.libelle.is_empty(), "{}", c.id);
        }
    }

    #[test]
    fn une_commande_a_un_niveau_et_une_lecture_est_n0() {
        for c in CAPACITES {
            match c.genre {
                Lecture => assert_eq!(c.confirmation, N0, "{}", c.id),
                Commande => assert_ne!(c.confirmation, N0, "{} : une commande se confirme", c.id),
            }
            if c.confirmation == FermeeIa {
                assert!(!c.ouverte_ia, "{} fermée à l'IA mais ouverte", c.id);
            }
        }
    }

    #[test]
    fn sans_droit_rien_et_tous_les_droits_sont_requis() {
        assert!(capacites_permises(|_| false).is_empty());
        let tout = capacites_permises(|_| true);
        assert_eq!(tout.len(), CAPACITES.len());
        let seul = capacites_permises(|d| d == "appel.faire");
        assert_eq!(seul.len(), 1);
        assert_eq!(seul[0].id, "appel.faire");
    }

    /// POR-244 : l'IA ne voit jamais plus que la palette, ni une capacité fermée.
    #[test]
    fn l_ia_est_incluse_dans_la_palette() {
        for droits in [
            vec![],
            vec!["appel.faire", "punitions.donner", "roles.gerer"],
        ] {
            let a = |d: &str| droits.contains(&d);
            let palette: HashSet<_> = capacites_permises(a).iter().map(|c| c.id).collect();
            for c in capacites_ouvertes_ia(a) {
                assert!(palette.contains(c.id));
                assert_ne!(c.confirmation, FermeeIa);
            }
        }
        let ia: Vec<_> = capacites_ouvertes_ia(|_| true)
            .iter()
            .map(|c| c.id)
            .collect();
        assert!(!ia.contains(&"roles.gerer") && !ia.contains(&"delegation.gerer"));
        assert!(
            !ia.contains(&"punition.donner"),
            "une sanction reste à l'écran"
        );
    }
}
