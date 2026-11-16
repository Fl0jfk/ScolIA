//! Socle 29, lot P7 : modes de l'accueil et bandeaux (`docs/socles/29_PORTAILS.md` §10, POR-226 à POR-235).
//!
//! * **Le mode est calculé** à partir de faits des autres socles (calendrier du socle 5 : rentrée, vacances,
//!   examens, fermetures et journées modifiées ; alerte du socle 14), jamais choisi à la main (D-023).
//!   Priorité : `alerte` > `journee_modifiee` > `examens` > `rentree` > `vacances` > `ordinaire`.
//! * **Une décision prise la veille est visible le matin** : le calcul relit le calendrier à chaque ouverture
//!   de l'accueil, sans file ni cache ; une fermeture de demain est aussi annoncée (bandeau d'information).
//! * **Publication** : une famille ne voit que ce que l'établissement lui a publié (`publie_familles`) ; un
//!   personnel voit ce qui est publié pour lui ou pour les familles ; la direction voit tout.
//! * **Alerte (socle 14)** : le port `PortAlerte` n'existe pas encore côté ENT ; l'accueil le lit derrière un
//!   trait (`AucuneAlerte` en production, `AlerteSimulee` pour les tests et le labo). La page d'alerte est
//!   alors **la page entière** : aucune autre tuile, bandeau de sécurité non masquable, familles : consigne
//!   officielle seulement. Le bouton « Je suis en sécurité avec … » appartient au 14 et reste absent.
//! * Le jour vient de l'appelant (heure de l'établissement) : aucune horloge ici.
//!
//! Non calculés faute de source (voir `docs/PROJECT_STATUS.md`) : `veille_conseil` (23), `bascule` (5),
//! `cohabitation` (D-032) ; bandeaux des socles 8 (canal SMS), 16 (urgence), 21 (guichet), 6 (remplacement).

use std::future::Future;
use std::pin::Pin;

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use ent_auth::Role;
use ent_types::portail::{
    ActionPortail, Bandeau, EleveEnCharge, EtatTuile, GraviteBandeau, LignePortail, ModePortail,
    PageAlerte, TonLigne, TuilePortail,
};
use uuid::Uuid;

use crate::service::EntDbError;
use crate::structure_calendrier::{EvenementJour, Jour};

/// Jours relus avant la date : la rentrée se repère par son premier jour (J-15 à J+15).
pub const AVANT: i64 = 16;
/// Jours relus après la date : la fenêtre de rentrée (J+15) et « reprise le … » après les vacances.
pub const APRES: i64 = 60;
/// Fenêtre de rentrée (§10.1).
pub const FENETRE_RENTREE: i64 = 15;

/// Public d'un accueil : décide ce qui a été publié pour la personne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Public {
    Famille,
    Personnel,
    /// Direction et administrateur : voient aussi ce qui n'est pas (encore) publié.
    Direction,
}

impl Public {
    pub fn du_role(r: Role) -> Self {
        match r {
            Role::Parent | Role::Eleve => Self::Famille,
            Role::Admin | Role::Direction => Self::Direction,
            _ => Self::Personnel,
        }
    }

    fn voit(self, e: &EvenementJour) -> bool {
        match self {
            Self::Famille => e.publie_familles,
            Self::Personnel => e.publie_personnels || e.publie_familles,
            Self::Direction => true,
        }
    }
}

/// Types d'événements qui **modifient** la journée (hors vacances, fériés, ponts, fermetures annuelles,
/// examens et rentrée, qui ont leur propre mode ou aucun effet sur l'accueil).
const MODIFIANTS: [&str; 4] = [
    "fermeture_exceptionnelle",
    "journee_banalisee",
    "journee_pedagogique",
    "rattrapage",
];

fn evenements_visibles(j: &Jour, p: Public) -> impl Iterator<Item = &EvenementJour> {
    j.evenements.iter().filter(move |e| p.voit(e))
}

fn modifie(e: &EvenementJour) -> bool {
    MODIFIANTS.contains(&e.type_evenement.as_str()) || e.effet == "cours_modifies"
}

fn jour_de_cours_normal(date: NaiveDate) -> bool {
    !matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

/// Premier jour de la rentrée la plus proche si `date` est dans la fenêtre J-15 à J+15 (§10.1).
/// `jours` couvre `[date - AVANT, date + APRES]`, dans l'ordre.
pub fn rentree_proche(jours: &[Jour], date: NaiveDate, p: Public) -> Option<NaiveDate> {
    let present = |j: &Jour| evenements_visibles(j, p).any(|e| e.type_evenement == "rentree");
    jours
        .iter()
        .enumerate()
        .filter(|(i, j)| {
            let d = j.date;
            let est_debut = present(j) && (*i == 0 || !present(&jours[*i - 1]));
            est_debut && (d - date).num_days().abs() <= FENETRE_RENTREE
        })
        .map(|(_, j)| j.date)
        .min_by_key(|d| (*d - date).num_days().abs())
}

/// Prochain jour avec cours strictement après `date` (reprise après les vacances).
pub fn reprise(jours: &[Jour], date: NaiveDate) -> Option<NaiveDate> {
    jours
        .iter()
        .find(|j| j.date > date && j.ouvre)
        .map(|j| j.date)
}

/// Le mode du moment, sans l'alerte (qui prime sur tout et vient d'un autre port).
pub fn calculer_mode(jours: &[Jour], date: NaiveDate, p: Public) -> ModePortail {
    let Some(auj) = jours.iter().find(|j| j.date == date) else {
        return ModePortail::Ordinaire;
    };
    if auj.type_jour != "vacances"
        && jour_de_cours_normal(date)
        && evenements_visibles(auj, p).any(modifie)
    {
        return ModePortail::JourneeModifiee;
    }
    if evenements_visibles(auj, p).any(|e| e.type_evenement == "examen") {
        return ModePortail::Examens;
    }
    if rentree_proche(jours, date, p).is_some() {
        return ModePortail::Rentree;
    }
    if auj.type_jour == "vacances" {
        return ModePortail::Vacances;
    }
    ModePortail::Ordinaire
}

fn nom_service(s: &str) -> &str {
    match s {
        "cours" => "cours",
        "accueil_eleves" => "accueil des élèves",
        "restauration" => "restauration",
        "internat" => "internat",
        "transports" => "transports",
        "administration" => "administration",
        autre => autre,
    }
}

fn liste_services(j: &Jour) -> String {
    j.services_fermes
        .iter()
        .map(|s| nom_service(s))
        .collect::<Vec<_>>()
        .join(", ")
}

const LIEN_CALENDRIER: &str = "/web/mon-agenda";

/// Bandeaux nés du calendrier (socle 5) : fermeture exceptionnelle du jour (attention) ou de demain
/// (information), autre journée modifiée du jour (information). Texte = libellé que l'établissement a publié.
pub fn bandeaux_calendrier(auj: &Jour, demain: Option<&Jour>, p: Public) -> Vec<Bandeau> {
    let mut v = Vec::new();
    let mut deja: Vec<Uuid> = Vec::new();
    for e in evenements_visibles(auj, p).filter(|e| modifie(e)) {
        if deja.contains(&e.id) {
            continue;
        }
        deja.push(e.id);
        if e.type_evenement == "fermeture_exceptionnelle" {
            let services = liste_services(auj);
            let texte = if services.is_empty() {
                format!("Fermeture exceptionnelle : {}.", e.libelle)
            } else {
                format!(
                    "Fermeture exceptionnelle : {}. Services touchés : {services}.",
                    e.libelle
                )
            };
            v.push(Bandeau::nouveau(
                "structure.fermeture",
                GraviteBandeau::Attention,
                &texte,
                true,
                Some(LIEN_CALENDRIER),
            ));
        } else {
            v.push(Bandeau::nouveau(
                "structure.journee_modifiee",
                GraviteBandeau::Info,
                &format!("Journée modifiée : {}.", e.libelle),
                true,
                Some(LIEN_CALENDRIER),
            ));
        }
    }
    if let Some(d) = demain {
        if let Some(e) = evenements_visibles(d, p)
            .find(|e| e.type_evenement == "fermeture_exceptionnelle" && !deja.contains(&e.id))
        {
            v.push(Bandeau::nouveau(
                "structure.fermeture_demain",
                GraviteBandeau::Info,
                &format!("Demain : fermeture exceptionnelle, {}.", e.libelle),
                true,
                Some(LIEN_CALENDRIER),
            ));
        }
    }
    v
}

const JOURS: [&str; 7] = [
    "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
];
const MOIS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

/// « lundi 4 novembre » (le serveur écrit la date : l'écran n'a pas de fuseau à deviner).
pub fn date_longue(d: NaiveDate) -> String {
    format!(
        "{} {} {}",
        JOURS[d.weekday().num_days_from_monday() as usize],
        if d.day() == 1 {
            "1er".to_string()
        } else {
            d.day().to_string()
        },
        MOIS[d.month0() as usize]
    )
}

fn tuile_ok(code: &str, titre: &str, principal: String) -> TuilePortail {
    let mut t = TuilePortail::nouvelle(code, titre, 12);
    t.etat = EtatTuile::Ok;
    t.principal = Some(principal);
    t
}

/// La tuile « Aujourd'hui modifié » : ce qui change aujourd'hui (cours, services), sans motif.
fn tuile_journee_modifiee(auj: &Jour, p: Public) -> TuilePortail {
    let mut t = tuile_ok(
        "journee.modifiee",
        "Aujourd'hui modifié",
        "La journée n'est pas ordinaire.".to_string(),
    );
    for e in evenements_visibles(auj, p).filter(|e| modifie(e)) {
        let effet = match e.effet.as_str() {
            "sans_cours" => "Pas de cours",
            "sans_eleves" => "Élèves non accueillis",
            "accueil_seulement" => "Accueil des élèves seulement, sans cours",
            "cours_modifies" => "Cours modifiés",
            _ => "Journée modifiée",
        };
        t.lignes.push(LignePortail {
            libelle: e.libelle.clone(),
            detail: Some(effet.to_string()),
            ton: TonLigne::Attention,
            lien: Some(LIEN_CALENDRIER.to_string()),
        });
    }
    for s in &auj.services_fermes {
        t.lignes.push(LignePortail {
            libelle: format!("Service fermé : {}", nom_service(s)),
            detail: None,
            ton: TonLigne::Attention,
            lien: None,
        });
    }
    t.action = Some(ActionPortail {
        libelle: "Voir mon agenda".into(),
        lien: LIEN_CALENDRIER.into(),
    });
    t
}

/// Tuiles ajoutées par le mode, **en tête** de page ; avec les codes de tuiles que le mode retire
/// (vacances : aucune tuile « maintenant » pour les personnels, plus d'emploi du temps pour les familles).
pub fn effets_du_mode(
    mode: ModePortail,
    jours: &[Jour],
    date: NaiveDate,
    p: Public,
) -> (Vec<TuilePortail>, &'static [&'static str]) {
    let Some(auj) = jours.iter().find(|j| j.date == date) else {
        return (vec![], &[]);
    };
    match mode {
        ModePortail::JourneeModifiee => (vec![tuile_journee_modifiee(auj, p)], &[]),
        ModePortail::Vacances => {
            let tuile = match reprise(jours, date) {
                Some(d) => tuile_ok(
                    "vacances.reprise",
                    "Vacances",
                    format!("Reprise des cours le {}.", date_longue(d)),
                ),
                None => tuile_ok(
                    "vacances.reprise",
                    "Vacances",
                    "Les vacances sont en cours.".to_string(),
                ),
            };
            (vec![tuile], &["edt.journee", "edt.enfants", "vs.jour"])
        }
        ModePortail::Rentree => match (p, rentree_proche(jours, date, p)) {
            (Public::Famille, Some(r)) => (
                vec![tuile_ok(
                    "rentree.carte",
                    "Rentrée",
                    format!("Rentrée des élèves le {}.", date_longue(r)),
                )],
                &[],
            ),
            _ => (vec![], &[]),
        },
        ModePortail::Examens => {
            let libelle = evenements_visibles(auj, p)
                .find(|e| e.type_evenement == "examen")
                .map(|e| e.libelle.clone())
                .unwrap_or_default();
            (
                vec![tuile_ok(
                    "examens.session",
                    "Examens",
                    format!("Session d'examens en cours : {libelle}."),
                )],
                &[],
            )
        }
        _ => (vec![], &[]),
    }
}

// ───────────────────────── Alerte (socle 14), derrière un trait ─────────────────────────

pub type Futur<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Alerte déclenchée : seule la consigne **officielle** est connue de l'accueil.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlerteEnCours {
    pub consigne: String,
}

/// Port du socle 14 (alerte PPMS), tel que l'accueil le consomme. **Absent sur ENT** : voir la module-doc.
pub trait PortAlerte: Send + Sync {
    fn alerte_en_cours(&self, etab: Uuid) -> Futur<'_, Result<Option<AlerteEnCours>, EntDbError>>;
    /// « Les élèves dont vous êtes responsable maintenant » (liste de présence), pour les personnels.
    fn eleves_en_charge(
        &self,
        etab: Uuid,
        acteur: Uuid,
    ) -> Futur<'_, Result<Vec<EleveEnCharge>, EntDbError>>;
}

/// Production tant que le socle 14 n'existe pas : jamais d'alerte.
pub struct AucuneAlerte;

impl PortAlerte for AucuneAlerte {
    fn alerte_en_cours(&self, _: Uuid) -> Futur<'_, Result<Option<AlerteEnCours>, EntDbError>> {
        Box::pin(async { Ok(None) })
    }
    fn eleves_en_charge(
        &self,
        _: Uuid,
        _: Uuid,
    ) -> Futur<'_, Result<Vec<EleveEnCharge>, EntDbError>> {
        Box::pin(async { Ok(vec![]) })
    }
}

/// Page d'alerte : la consigne ; la liste des élèves en charge seulement pour un personnel.
pub async fn page_alerte(
    port: &dyn PortAlerte,
    etab: Uuid,
    acteur: Uuid,
    p: Public,
    a: &AlerteEnCours,
) -> PageAlerte {
    let eleves = if p == Public::Famille {
        vec![]
    } else {
        port.eleves_en_charge(etab, acteur)
            .await
            .unwrap_or_default()
    };
    PageAlerte {
        consigne: a.consigne.clone(),
        eleves_en_charge: eleves,
        action_securite: None,
    }
}

pub fn bandeau_alerte(a: &AlerteEnCours) -> Bandeau {
    Bandeau::nouveau(
        "alerte.ppms",
        GraviteBandeau::Securite,
        &format!("Alerte en cours. {}", a.consigne),
        false,
        None,
    )
}

pub fn bandeau_alerte_indisponible() -> Bandeau {
    Bandeau::nouveau(
        "alerte.indisponible",
        GraviteBandeau::Attention,
        "L'état d'alerte n'a pas pu être vérifié. En cas de doute, suivez les consignes affichées dans l'établissement.",
        true,
        None,
    )
}

/// Alerte simulée (tests et labo) : tient lieu du socle 14 jusqu'à son arrivée.
#[cfg(any(test, feature = "labo"))]
#[derive(Default)]
pub struct AlerteSimulee {
    etat: std::sync::Mutex<Vec<(Uuid, AlerteEnCours)>>,
    charges: std::sync::Mutex<Vec<(Uuid, Uuid, EleveEnCharge)>>,
}

#[cfg(any(test, feature = "labo"))]
impl AlerteSimulee {
    pub fn declencher(&self, etab: Uuid, consigne: &str) {
        let mut e = self.etat.lock().unwrap();
        e.retain(|(x, _)| *x != etab);
        e.push((
            etab,
            AlerteEnCours {
                consigne: consigne.to_string(),
            },
        ));
    }
    pub fn cloturer(&self, etab: Uuid) {
        self.etat.lock().unwrap().retain(|(x, _)| *x != etab);
    }
    pub fn confier(&self, etab: Uuid, acteur: Uuid, libelle: &str, presence: &str) {
        self.charges.lock().unwrap().push((
            etab,
            acteur,
            EleveEnCharge {
                libelle: libelle.to_string(),
                presence: presence.to_string(),
            },
        ));
    }
}

#[cfg(any(test, feature = "labo"))]
impl PortAlerte for AlerteSimulee {
    fn alerte_en_cours(&self, etab: Uuid) -> Futur<'_, Result<Option<AlerteEnCours>, EntDbError>> {
        let r = self
            .etat
            .lock()
            .unwrap()
            .iter()
            .find(|(x, _)| *x == etab)
            .map(|(_, a)| a.clone());
        Box::pin(async move { Ok(r) })
    }
    fn eleves_en_charge(
        &self,
        etab: Uuid,
        acteur: Uuid,
    ) -> Futur<'_, Result<Vec<EleveEnCharge>, EntDbError>> {
        let r = self
            .charges
            .lock()
            .unwrap()
            .iter()
            .filter(|(e, a, _)| *e == etab && *a == acteur)
            .map(|(_, _, c)| c.clone())
            .collect();
        Box::pin(async move { Ok(r) })
    }
}

/// Fenêtre à relire pour une date.
pub fn fenetre(date: NaiveDate) -> (NaiveDate, NaiveDate) {
    (date - Duration::days(AVANT), date + Duration::days(APRES))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure_calendrier::DemiJournee;

    fn d(m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, j).unwrap()
    }

    fn ev(t: &str, effet: &str, fam: bool, pers: bool) -> EvenementJour {
        EvenementJour {
            id: Uuid::new_v4(),
            type_evenement: t.into(),
            libelle: format!("{t} du jour"),
            effet: effet.into(),
            jour_reference: None,
            publie_familles: fam,
            publie_personnels: pers,
        }
    }

    fn jour(date: NaiveDate, type_jour: &str, ouvre: bool, evs: Vec<EvenementJour>) -> Jour {
        Jour {
            date,
            annee_id: None,
            ouvre,
            type_jour: type_jour.into(),
            matin: DemiJournee::default(),
            apres_midi: DemiJournee::default(),
            semaine_type: None,
            jour_reference: None,
            services_fermes: vec![],
            evenements: evs,
        }
    }

    /// Fenêtre ordinaire autour de `date` (jeudi 12 novembre 2026) : aucun événement.
    fn fenetre_vide(date: NaiveDate) -> Vec<Jour> {
        let (du, au) = fenetre(date);
        du.iter_days()
            .take_while(|x| *x <= au)
            .map(|x| jour(x, "cours", jour_de_cours_normal(x), vec![]))
            .collect()
    }

    fn poser(jours: &mut [Jour], date: NaiveDate, e: EvenementJour) {
        jours
            .iter_mut()
            .find(|j| j.date == date)
            .unwrap()
            .evenements
            .push(e);
    }

    #[test]
    fn ordinaire_par_defaut() {
        let date = d(11, 12);
        let j = fenetre_vide(date);
        for p in [Public::Famille, Public::Personnel, Public::Direction] {
            assert_eq!(calculer_mode(&j, date, p), ModePortail::Ordinaire);
        }
    }

    /// « Une fermeture décidée la veille est visible le matin » : le calcul ne dépend que du calendrier relu.
    #[test]
    fn une_fermeture_publiee_donne_la_journee_modifiee_et_son_bandeau() {
        let date = d(11, 12);
        let mut j = fenetre_vide(date);
        let mut e = ev("fermeture_exceptionnelle", "sans_cours", true, false);
        e.libelle = "Intempéries".into();
        poser(&mut j, date, e);
        j.iter_mut()
            .find(|x| x.date == date)
            .unwrap()
            .services_fermes = vec!["cours".into(), "restauration".into()];
        assert_eq!(
            calculer_mode(&j, date, Public::Famille),
            ModePortail::JourneeModifiee
        );
        let auj = j.iter().find(|x| x.date == date).unwrap();
        let b = bandeaux_calendrier(auj, None, Public::Famille);
        assert_eq!(b.len(), 1);
        assert!(b[0].texte.contains("Intempéries") && b[0].texte.contains("restauration"));
        assert_eq!(b[0].gravite, GraviteBandeau::Attention);
        let t = tuile_journee_modifiee(auj, Public::Famille);
        assert!(t.lignes.iter().any(|l| l.libelle == "Intempéries"));
        assert!(t.lignes.iter().any(|l| l.libelle.contains("restauration")));
    }

    #[test]
    fn publication_famille_personnel_direction() {
        let date = d(11, 12);
        let mut j = fenetre_vide(date);
        poser(
            &mut j,
            date,
            ev("fermeture_exceptionnelle", "sans_cours", false, false),
        );
        assert_eq!(
            calculer_mode(&j, date, Public::Famille),
            ModePortail::Ordinaire
        );
        assert_eq!(
            calculer_mode(&j, date, Public::Personnel),
            ModePortail::Ordinaire
        );
        assert_eq!(
            calculer_mode(&j, date, Public::Direction),
            ModePortail::JourneeModifiee
        );
        let mut j = fenetre_vide(date);
        poser(
            &mut j,
            date,
            ev("fermeture_exceptionnelle", "sans_cours", false, true),
        );
        assert_eq!(
            calculer_mode(&j, date, Public::Famille),
            ModePortail::Ordinaire
        );
        assert_eq!(
            calculer_mode(&j, date, Public::Personnel),
            ModePortail::JourneeModifiee
        );
    }

    #[test]
    fn la_fermeture_de_demain_est_annoncee_aujourd_hui() {
        let date = d(11, 12);
        let mut j = fenetre_vide(date);
        poser(
            &mut j,
            d(11, 13),
            ev("fermeture_exceptionnelle", "sans_cours", true, false),
        );
        assert_eq!(
            calculer_mode(&j, date, Public::Famille),
            ModePortail::Ordinaire,
            "aujourd'hui reste ordinaire"
        );
        let auj = j.iter().find(|x| x.date == date).unwrap();
        let dem = j.iter().find(|x| x.date == d(11, 13));
        let b = bandeaux_calendrier(auj, dem, Public::Famille);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].code, "structure.fermeture_demain");
        assert_eq!(b[0].gravite, GraviteBandeau::Info);
        // Pas de doublon le jour même.
        let b2 = bandeaux_calendrier(dem.unwrap(), None, Public::Famille);
        assert_eq!(b2[0].code, "structure.fermeture");
    }

    #[test]
    fn le_week_end_n_est_pas_une_journee_modifiee() {
        let samedi = d(11, 14);
        let mut j = fenetre_vide(samedi);
        poser(
            &mut j,
            samedi,
            ev("fermeture_exceptionnelle", "sans_cours", true, false),
        );
        assert_eq!(
            calculer_mode(&j, samedi, Public::Famille),
            ModePortail::Ordinaire
        );
    }

    #[test]
    fn rentree_de_j_moins_15_a_j_plus_15() {
        let r = d(9, 1); // rentrée : mardi 1er septembre 2026
        let mut ok = vec![];
        for date in [d(8, 17), d(8, 31), d(9, 1), d(9, 16)] {
            let mut j = fenetre_vide(date);
            poser(&mut j, r, ev("rentree", "informatif", true, false));
            poser(&mut j, d(9, 2), ev("rentree", "informatif", true, false)); // la rentrée dure deux jours
            ok.push(calculer_mode(&j, date, Public::Famille));
        }
        assert_eq!(ok, [ModePortail::Rentree; 4]);
        for date in [d(8, 16), d(9, 17)] {
            let mut j = fenetre_vide(date);
            poser(&mut j, r, ev("rentree", "informatif", true, false));
            poser(&mut j, d(9, 2), ev("rentree", "informatif", true, false));
            assert_eq!(
                calculer_mode(&j, date, Public::Famille),
                ModePortail::Ordinaire,
                "{date}"
            );
        }
        // Non publiée aux familles : pas de mode pour elles.
        let date = d(9, 1);
        let mut j = fenetre_vide(date);
        poser(&mut j, r, ev("rentree", "informatif", false, true));
        assert_eq!(
            calculer_mode(&j, date, Public::Famille),
            ModePortail::Ordinaire
        );
        assert_eq!(
            calculer_mode(&j, date, Public::Personnel),
            ModePortail::Rentree
        );
    }

    #[test]
    fn la_carte_de_rentree_ne_va_qu_aux_familles() {
        let date = d(8, 28);
        let mut j = fenetre_vide(date);
        poser(&mut j, d(9, 1), ev("rentree", "informatif", true, true));
        let (t, _) = effets_du_mode(ModePortail::Rentree, &j, date, Public::Famille);
        assert_eq!(t.len(), 1);
        assert_eq!(
            t[0].principal.as_deref(),
            Some("Rentrée des élèves le mardi 1er septembre.")
        );
        let (t, _) = effets_du_mode(ModePortail::Rentree, &j, date, Public::Personnel);
        assert!(t.is_empty());
    }

    #[test]
    fn vacances_reprise_et_aucune_tuile_maintenant() {
        let date = d(10, 21); // mercredi, vacances de la Toussaint jusqu'au dimanche 1er novembre
        let mut j = fenetre_vide(date);
        for x in j.iter_mut() {
            if x.date >= d(10, 17) && x.date <= d(11, 1) {
                x.type_jour = "vacances".into();
                x.ouvre = false;
            }
        }
        assert_eq!(
            calculer_mode(&j, date, Public::Personnel),
            ModePortail::Vacances
        );
        let (t, retire) = effets_du_mode(ModePortail::Vacances, &j, date, Public::Personnel);
        assert_eq!(
            t[0].principal.as_deref(),
            Some("Reprise des cours le lundi 2 novembre.")
        );
        assert!(retire.contains(&"vs.jour") && retire.contains(&"edt.journee"));
    }

    #[test]
    fn examens_et_priorites() {
        let date = d(6, 16);
        let mut j = fenetre_vide(date);
        poser(&mut j, date, ev("examen", "informatif", true, true));
        assert_eq!(
            calculer_mode(&j, date, Public::Famille),
            ModePortail::Examens
        );
        // Une fermeture le jour d'un examen prime.
        poser(
            &mut j,
            date,
            ev("fermeture_exceptionnelle", "sans_cours", true, true),
        );
        assert_eq!(
            calculer_mode(&j, date, Public::Famille),
            ModePortail::JourneeModifiee
        );
    }

    #[test]
    fn date_en_francais() {
        assert_eq!(date_longue(d(11, 14)), "samedi 14 novembre");
        assert_eq!(date_longue(d(12, 1)), "mardi 1er décembre");
    }

    // ── Alerte ──

    #[tokio::test]
    async fn la_page_d_alerte_famille_n_a_que_la_consigne() {
        let (etab, prof) = (Uuid::new_v4(), Uuid::new_v4());
        let port = AlerteSimulee::default();
        port.declencher(etab, "Mise à l'abri dans les salles.");
        port.confier(etab, prof, "DUPONT Léa — 4B", "présent");
        let a = port.alerte_en_cours(etab).await.unwrap().expect("alerte");
        let fam = page_alerte(&port, etab, prof, Public::Famille, &a).await;
        assert_eq!(fam.consigne, "Mise à l'abri dans les salles.");
        assert!(
            fam.eleves_en_charge.is_empty(),
            "aucun détail opérationnel pour les familles"
        );
        let pers = page_alerte(&port, etab, prof, Public::Personnel, &a).await;
        assert_eq!(pers.eleves_en_charge.len(), 1);
        assert!(
            pers.action_securite.is_none(),
            "« en sécurité » appartient au socle 14"
        );
        let b = bandeau_alerte(&a);
        assert_eq!(b.gravite, GraviteBandeau::Securite);
        assert!(!b.masquable);
        // Cloisonnement : une autre structure ne voit rien.
        assert!(port
            .alerte_en_cours(Uuid::new_v4())
            .await
            .unwrap()
            .is_none());
        port.cloturer(etab);
        assert!(port.alerte_en_cours(etab).await.unwrap().is_none());
        assert!(AucuneAlerte.alerte_en_cours(etab).await.unwrap().is_none());
    }
}
