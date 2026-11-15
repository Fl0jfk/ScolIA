//! Socle 8, lot N3 — le temps d'une remise : heures calmes à trois étages, urgences, résumé, budget de bruit,
//! pause (`08_NOTIFICATIONS.md` §9.3, §9.4, CN-49).
//!
//! Moteur **pur** : il travaille en heure locale de l'établissement (`NaiveDateTime`) ; la conversion en instant
//! UTC se fait en base (`AT TIME ZONE fuseau`), seule à connaître les changements d'heure. Aucun accès à
//! l'horloge : `local` est fourni (jamais `Local::now()` ni `Utc::now().date_naive()`).
//!
//! Règles :
//! - l'ENT (in-app) est toujours immédiat ; seuls les canaux externes sont décalés ;
//! - **U3** part tout de suite : ni heures calmes, ni pause, ni résumé, ni budget ;
//! - **U2** attend la fin des heures calmes de **nuit de l'établissement** seulement (une absence de 10 h ne
//!   peut pas attendre 18 h : ni heures calmes de jour choisies par la personne, ni jours calmes, ni pause) ;
//! - **U0 et U1** attendent la fin de toutes les fenêtres : heures calmes de l'établissement, jours calmes
//!   (dimanche par défaut), heures calmes de la personne, pause, indisponibilité (personnels, lot N12) ;
//! - **U0** (et les natures regroupables au résumé) passent au **résumé** à l'heure choisie (quotidien) ou le
//!   lundi (hebdomadaire, ou vacances « hebdo » de l'établissement), et au-delà du **budget de bruit** ;
//! - le résumé lui-même ne tombe jamais dans les heures calmes de l'établissement.

use super::contrat::Urgence;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

/// Plage en minutes depuis minuit ; `fin <= debut` : passe minuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plage {
    pub debut: u16,
    pub fin: u16,
}

impl Plage {
    pub fn contient(&self, minute: u16) -> bool {
        if self.debut < self.fin {
            self.debut <= minute && minute < self.fin
        } else {
            minute >= self.debut || minute < self.fin
        }
    }
}

/// Fenêtre de silence : une plage horaire et des jours entiers (masque : bit 0 = lundi … bit 6 = dimanche).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Calme {
    pub plage: Option<Plage>,
    pub jours: u8,
}

impl Calme {
    pub fn jour_calme(&self, d: NaiveDate) -> bool {
        self.jours & (1 << d.weekday().num_days_from_monday()) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeResume {
    Immediat,
    Quotidien,
    Hebdo,
}

impl ModeResume {
    pub fn depuis_code(c: &str) -> Self {
        match c {
            "quotidien" => ModeResume::Quotidien,
            "hebdo" => ModeResume::Hebdo,
            _ => ModeResume::Immediat,
        }
    }
}

/// Ce que le moteur sait de la personne et de l'établissement à l'instant `local`.
#[derive(Debug, Clone)]
pub struct ContexteTemps {
    pub local: NaiveDateTime,
    /// Premier étage : heures calmes de l'établissement.
    pub etab: Calme,
    /// Deuxième étage : heures calmes de la personne.
    pub personne: Calme,
    /// Troisième étage : indisponibilité d'un personnel (congé, hors service) ; lot N12.
    pub indispo_jusqua: Option<NaiveDateTime>,
    pub pause_jusqua: Option<NaiveDateTime>,
    pub mode_resume: ModeResume,
    /// Heure du résumé, en minutes depuis minuit.
    pub heure_resume: u16,
    /// Vacances de l'établissement en mode « hebdo » : U0 au résumé hebdomadaire (CN-45).
    pub vacances_hebdo: bool,
    /// Avis U0 déjà prévus aujourd'hui pour la personne, et budget de l'établissement.
    pub u0_du_jour: u32,
    pub budget_u0: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moment {
    Immediat,
    Differe(NaiveDateTime),
    Resume(NaiveDateTime, MotifResume),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotifResume {
    Quotidien,
    Hebdo,
    BudgetBruit,
}

impl MotifResume {
    pub fn code(self) -> &'static str {
        match self {
            MotifResume::Quotidien => "resume_quotidien",
            MotifResume::Hebdo => "resume_hebdo",
            MotifResume::BudgetBruit => "budget_bruit",
        }
    }
}

fn minute(t: NaiveDateTime) -> u16 {
    (t.hour() * 60 + t.minute()) as u16
}

fn a_minute(d: NaiveDate, m: u16) -> NaiveDateTime {
    d.and_time(NaiveTime::from_hms_opt(u32::from(m / 60), u32::from(m % 60), 0).expect("minute valide"))
}

/// Fin de la plage qui contient `t` (au plus proche), sinon `t`.
fn sortir_de_plage(t: NaiveDateTime, p: Option<Plage>) -> NaiveDateTime {
    let Some(p) = p else { return t };
    if !p.contient(minute(t)) {
        return t;
    }
    let fin = a_minute(t.date(), p.fin);
    if fin > t {
        fin
    } else {
        a_minute(t.date() + Duration::days(1), p.fin)
    }
}

/// Premier instant ≥ `t` hors de toutes les fenêtres retenues (point fixe, borné).
fn attendre_fin_des_fenetres(mut t: NaiveDateTime, c: &ContexteTemps, jours_calmes: bool, jour_et_personne: bool) -> NaiveDateTime {
    for _ in 0..64 {
        let avant = t;
        if jour_et_personne {
            for fin in [c.pause_jusqua, c.indispo_jusqua].into_iter().flatten() {
                if fin > t {
                    t = fin;
                }
            }
        }
        t = sortir_de_plage(t, c.etab.plage);
        if jours_calmes && c.etab.jour_calme(t.date()) {
            t = a_minute(t.date() + Duration::days(1), 0);
        }
        if jour_et_personne {
            t = sortir_de_plage(t, c.personne.plage);
            if c.personne.jour_calme(t.date()) {
                t = a_minute(t.date() + Duration::days(1), 0);
            }
        }
        if t == avant {
            break;
        }
    }
    t
}

/// Prochain instant du résumé à `heure`, au plus tôt à `t` : tous les jours, ou le lundi si `hebdo`.
fn prochain_resume(t: NaiveDateTime, heure: u16, hebdo: bool) -> NaiveDateTime {
    let mut d = t.date();
    for _ in 0..8 {
        let cible = a_minute(d, heure);
        let jour_ok = !hebdo || d.weekday().num_days_from_monday() == 0;
        if cible >= t && jour_ok {
            return cible;
        }
        d += Duration::days(1);
    }
    a_minute(d, heure)
}

/// Décide du moment d'une remise. `regroupable` : la nature accepte le résumé (U0 l'est toujours).
pub fn decider(urgence: Urgence, externe: bool, regroupable: bool, c: &ContexteTemps) -> Moment {
    if !externe || urgence == Urgence::U3 {
        return Moment::Immediat;
    }
    if urgence == Urgence::U2 {
        // Nuit de l'établissement seulement ; jamais le résumé, jamais les fenêtres de jour de la personne.
        let t = sortir_de_plage(c.local, c.etab.plage);
        return if t == c.local { Moment::Immediat } else { Moment::Differe(t) };
    }
    let calme = attendre_fin_des_fenetres(c.local, c, true, true);
    let u0 = urgence == Urgence::U0;
    let hebdo = c.vacances_hebdo && u0 || c.mode_resume == ModeResume::Hebdo && (u0 || regroupable);
    let quotidien = c.mode_resume == ModeResume::Quotidien && (u0 || regroupable);
    let bruit = u0 && c.u0_du_jour >= c.budget_u0;
    if hebdo || quotidien || bruit {
        let (motif, hebdo_effectif) = if hebdo {
            (MotifResume::Hebdo, true)
        } else if quotidien {
            (MotifResume::Quotidien, false)
        } else {
            (MotifResume::BudgetBruit, false)
        };
        let heure = c.heure_resume;
        let cible = prochain_resume(calme, heure, hebdo_effectif);
        // Le résumé ne tombe pas dans la nuit de l'établissement.
        let cible = attendre_fin_des_fenetres(cible, c, false, false);
        return Moment::Resume(cible, motif);
    }
    if calme == c.local {
        Moment::Immediat
    } else {
        Moment::Differe(calme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(j: u32, h: u32, m: u32) -> NaiveDateTime {
        // Novembre 2026 : le 2 est un lundi, le 8 un dimanche.
        NaiveDate::from_ymd_opt(2026, 11, j).unwrap().and_hms_opt(h, m, 0).unwrap()
    }

    fn ctx(local: NaiveDateTime) -> ContexteTemps {
        ContexteTemps {
            local,
            etab: Calme { plage: Some(Plage { debut: 20 * 60 + 30, fin: 7 * 60 }), jours: 1 << 6 },
            personne: Calme::default(),
            indispo_jusqua: None,
            pause_jusqua: None,
            mode_resume: ModeResume::Immediat,
            heure_resume: 18 * 60,
            vacances_hebdo: false,
            u0_du_jour: 0,
            budget_u0: 5,
        }
    }

    #[test]
    fn l_ent_est_toujours_immediat_et_u3_part_tout_de_suite() {
        let nuit = ctx(d(3, 23, 0));
        assert_eq!(decider(Urgence::U0, false, true, &nuit), Moment::Immediat);
        assert_eq!(decider(Urgence::U3, true, false, &nuit), Moment::Immediat);
        let mut pause = ctx(d(3, 23, 0));
        pause.pause_jusqua = Some(d(20, 0, 0));
        assert_eq!(decider(Urgence::U3, true, false, &pause), Moment::Immediat, "U3 ignore la pause");
    }

    #[test]
    fn u2_attend_la_nuit_de_l_etablissement_seulement() {
        // 10 h, dans les heures calmes de JOUR choisies par la personne : une absence ne peut pas attendre.
        let mut c = ctx(d(3, 10, 0));
        c.personne = Calme { plage: Some(Plage { debut: 9 * 60, fin: 12 * 60 }), jours: 0 };
        assert_eq!(decider(Urgence::U2, true, false, &c), Moment::Immediat);
        // 21 h : nuit de l'établissement, partira à 7 h.
        assert_eq!(decider(Urgence::U2, true, false, &ctx(d(3, 21, 0))), Moment::Differe(d(4, 7, 0)));
        // Dimanche à 10 h : le jour calme du dimanche ne vaut pas pour U2.
        assert_eq!(decider(Urgence::U2, true, false, &ctx(d(8, 10, 0))), Moment::Immediat);
        // Pause : jamais pour U2.
        let mut p = ctx(d(3, 10, 0));
        p.pause_jusqua = Some(d(20, 0, 0));
        assert_eq!(decider(Urgence::U2, true, false, &p), Moment::Immediat);
    }

    #[test]
    fn u1_attend_toutes_les_fenetres() {
        // Samedi 21 h → dimanche (jour calme) → lundi 7 h.
        assert_eq!(decider(Urgence::U1, true, false, &ctx(d(7, 21, 0))), Moment::Differe(d(9, 7, 0)));
        // Mardi 14 h, heures calmes de la personne 13 h–15 h : 15 h.
        let mut c = ctx(d(3, 14, 0));
        c.personne = Calme { plage: Some(Plage { debut: 13 * 60, fin: 15 * 60 }), jours: 0 };
        assert_eq!(decider(Urgence::U1, true, false, &c), Moment::Differe(d(3, 15, 0)));
        // Fenêtres qui s'enchaînent : calme de la personne jusqu'à 21 h, puis nuit de l'établissement : 7 h.
        let mut c = ctx(d(3, 14, 0));
        c.personne = Calme { plage: Some(Plage { debut: 13 * 60, fin: 21 * 60 }), jours: 0 };
        assert_eq!(decider(Urgence::U1, true, false, &c), Moment::Differe(d(4, 7, 0)));
        // Pause jusqu'au 10 à midi.
        let mut c = ctx(d(3, 14, 0));
        c.pause_jusqua = Some(d(10, 12, 0));
        assert_eq!(decider(Urgence::U1, true, false, &c), Moment::Differe(d(10, 12, 0)));
        // Indisponibilité d'un personnel (étage 3).
        let mut c = ctx(d(3, 14, 0));
        c.indispo_jusqua = Some(d(3, 17, 30));
        assert_eq!(decider(Urgence::U1, true, false, &c), Moment::Differe(d(3, 17, 30)));
        // Jours calmes de la personne (mardi = bit 1).
        let mut c = ctx(d(3, 10, 0));
        c.personne = Calme { plage: None, jours: 1 << 1 };
        assert_eq!(decider(Urgence::U1, true, false, &c), Moment::Differe(d(4, 7, 0)));
    }

    #[test]
    fn plage_qui_passe_minuit_et_bornes() {
        let p = Plage { debut: 20 * 60 + 30, fin: 7 * 60 };
        assert!(p.contient(20 * 60 + 30) && p.contient(0) && p.contient(6 * 60 + 59));
        assert!(!p.contient(7 * 60) && !p.contient(20 * 60 + 29));
        // À 7 h 00 pile : hors plage, immédiat.
        assert_eq!(decider(Urgence::U1, true, false, &ctx(d(3, 7, 0))), Moment::Immediat);
        // À 20 h 30 pile : dans la plage, 7 h le lendemain.
        assert_eq!(decider(Urgence::U1, true, false, &ctx(d(3, 20, 30))), Moment::Differe(d(4, 7, 0)));
    }

    #[test]
    fn u0_au_resume_a_l_heure_choisie() {
        let mut c = ctx(d(3, 10, 0));
        c.mode_resume = ModeResume::Quotidien;
        assert_eq!(decider(Urgence::U0, true, false, &c), Moment::Resume(d(3, 18, 0), MotifResume::Quotidien));
        // Après l'heure du résumé : le lendemain.
        c.local = d(3, 19, 0);
        // 19 h : hors nuit, mais après 18 h → lendemain 18 h.
        assert_eq!(decider(Urgence::U0, true, false, &c), Moment::Resume(d(4, 18, 0), MotifResume::Quotidien));
        // Mode immédiat : U0 part tout de suite (hors fenêtres).
        assert_eq!(decider(Urgence::U0, true, false, &ctx(d(3, 10, 0))), Moment::Immediat);
        // Hebdomadaire : le lundi suivant.
        let mut h = ctx(d(3, 10, 0));
        h.mode_resume = ModeResume::Hebdo;
        assert_eq!(decider(Urgence::U0, true, false, &h), Moment::Resume(d(9, 18, 0), MotifResume::Hebdo));
        // Vacances « hebdo » de l'établissement (CN-45) sans choix de la personne.
        let mut v = ctx(d(3, 10, 0));
        v.vacances_hebdo = true;
        assert_eq!(decider(Urgence::U0, true, false, &v), Moment::Resume(d(9, 18, 0), MotifResume::Hebdo));
        // U1 non regroupable ne passe jamais au résumé ; regroupable (retards) si la personne le choisit.
        let mut q = ctx(d(3, 10, 0));
        q.mode_resume = ModeResume::Quotidien;
        assert_eq!(decider(Urgence::U1, true, false, &q), Moment::Immediat);
        assert_eq!(decider(Urgence::U1, true, true, &q), Moment::Resume(d(3, 18, 0), MotifResume::Quotidien));
        // Résumé réglé dans la nuit de l'établissement : repoussé à 7 h.
        let mut n = ctx(d(3, 10, 0));
        n.mode_resume = ModeResume::Quotidien;
        n.heure_resume = 22 * 60;
        assert_eq!(decider(Urgence::U0, true, false, &n), Moment::Resume(d(4, 7, 0), MotifResume::Quotidien));
    }

    #[test]
    fn budget_de_bruit_cn49() {
        let mut c = ctx(d(3, 10, 0));
        c.u0_du_jour = 4;
        assert_eq!(decider(Urgence::U0, true, false, &c), Moment::Immediat, "le 5e passe encore");
        c.u0_du_jour = 5;
        assert_eq!(decider(Urgence::U0, true, false, &c), Moment::Resume(d(3, 18, 0), MotifResume::BudgetBruit));
        // Le budget ne s'applique ni à U1 ni à U2.
        assert_eq!(decider(Urgence::U1, true, false, &c), Moment::Immediat);
        assert_eq!(decider(Urgence::U2, true, false, &c), Moment::Immediat);
    }
}
