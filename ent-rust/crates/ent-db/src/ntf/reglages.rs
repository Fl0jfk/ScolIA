//! Socle 8, lot N3 — réglages de notification : préréglages, préférences par catégorie, heures calmes de la
//! personne et de l'établissement, pause, fratrie ; reprise de l'ancien moteur (`08_NOTIFICATIONS.md` §9.2–9.4).
//!
//! Les préférences sont par **personne** (elle survit au compte) et par **catégorie**. Règle d'application
//! (§9.2) : pour U0 et U1 la personne peut retirer des canaux externes ; pour U2 et U3 elle ne peut qu'en
//! ajouter (l'absence ou l'alerte ne se désactive pas) ; le plancher de la nature rétablit au besoin un canal.
//! L'ancien moteur (`ent_preference_notification`) reste lu par les natures non encore basculées (lot N11) :
//! chaque écriture de l'ancien réglage est **reflétée** ici, pour qu'aucune des deux sources ne diverge.

use super::contrat::Categorie;
use super::temps::{Calme, ModeResume, Plage};
use crate::notifications::Preferences;
use chrono::{DateTime, NaiveDateTime, NaiveTime, Timelike, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

/// Préférence d'une personne pour une catégorie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefCategorie {
    pub push: bool,
    pub email: bool,
    pub sms: bool,
    pub resume: bool,
}

impl Default for PrefCategorie {
    /// Défauts : push et e-mail actifs, **SMS désactivé** (consentement explicite).
    fn default() -> Self {
        Self { push: true, email: true, sms: false, resume: false }
    }
}

/// Préréglages proposés à la personne (§9.2) : un choix, pas dix réglages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// Seulement ce qui compte (U2, U3) en direct ; le reste en résumé quotidien.
    Essentiel,
    /// Défaut : push et e-mail, au fil de l'eau.
    Equilibre,
    /// Tout en direct, SMS compris.
    Tout,
    /// Peu de bruit : e-mail seulement, résumé quotidien.
    Discret,
}

impl Preset {
    pub fn code(self) -> &'static str {
        match self {
            Preset::Essentiel => "essentiel",
            Preset::Equilibre => "equilibre",
            Preset::Tout => "tout",
            Preset::Discret => "discret",
        }
    }

    pub fn depuis_code(c: &str) -> Option<Self> {
        Some(match c {
            "essentiel" => Preset::Essentiel,
            "equilibre" => Preset::Equilibre,
            "tout" => Preset::Tout,
            "discret" => Preset::Discret,
            _ => return None,
        })
    }

    /// Préférences qu'impose le préréglage pour une catégorie.
    pub fn prefs(self, _categorie: Categorie) -> PrefCategorie {
        match self {
            Preset::Essentiel => PrefCategorie { push: true, email: false, sms: false, resume: true },
            Preset::Equilibre => PrefCategorie::default(),
            Preset::Tout => PrefCategorie { push: true, email: true, sms: true, resume: false },
            Preset::Discret => PrefCategorie { push: false, email: true, sms: false, resume: true },
        }
    }

    pub fn mode_resume(self) -> ModeResume {
        match self {
            Preset::Essentiel | Preset::Discret => ModeResume::Quotidien,
            Preset::Equilibre | Preset::Tout => ModeResume::Immediat,
        }
    }
}

/// Réglage d'une personne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReglagePersonne {
    pub preset: String,
    pub calme: Calme,
    pub mode_resume: ModeResume,
    /// Heure du résumé, en minutes depuis minuit.
    pub heure_resume: u16,
    pub regrouper_fratrie: bool,
    pub apercus_verrouille: bool,
    pub pause_jusqua: Option<DateTime<Utc>>,
}

impl Default for ReglagePersonne {
    fn default() -> Self {
        Self {
            preset: "equilibre".into(),
            calme: Calme::default(),
            mode_resume: ModeResume::Immediat,
            heure_resume: 18 * 60,
            regrouper_fratrie: false,
            apercus_verrouille: false,
            pause_jusqua: None,
        }
    }
}

fn minutes(t: NaiveTime) -> u16 {
    (t.hour() * 60 + t.minute()) as u16
}

fn heure(m: u16) -> NaiveTime {
    NaiveTime::from_hms_opt(u32::from(m / 60), u32::from(m % 60), 0).expect("minute valide")
}

/// Deux heures égales = aucune plage (pas de « toute la journée » silencieuse par accident).
pub fn plage_depuis(debut: Option<NaiveTime>, fin: Option<NaiveTime>) -> Option<Plage> {
    match (debut, fin) {
        (Some(d), Some(f)) if minutes(d) != minutes(f) => Some(Plage { debut: minutes(d), fin: minutes(f) }),
        _ => None,
    }
}

pub async fn lire_reglage(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    personne_id: Uuid,
) -> Result<ReglagePersonne, sqlx::Error> {
    type Ligne = (
        String,
        Option<NaiveTime>,
        Option<NaiveTime>,
        Option<i16>,
        String,
        NaiveTime,
        bool,
        bool,
        Option<DateTime<Utc>>,
    );
    let l: Option<Ligne> = sqlx::query_as(
        "SELECT preset, calme_debut, calme_fin, jours_calmes_masque, resume_mode, resume_heure,
                regrouper_fratrie, apercus_verrouille, pause_jusqua
           FROM ent_ntf_reglage_personne WHERE etablissement_id = $1 AND personne_id = $2",
    )
    .bind(etablissement_id)
    .bind(personne_id)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(match l {
        None => ReglagePersonne::default(),
        Some((preset, cd, cf, jours, mode, rh, fratrie, apercus, pause)) => ReglagePersonne {
            preset,
            calme: Calme { plage: plage_depuis(cd, cf), jours: jours.unwrap_or(0) as u8 },
            mode_resume: ModeResume::depuis_code(&mode),
            heure_resume: minutes(rh),
            regrouper_fratrie: fratrie,
            apercus_verrouille: apercus,
            pause_jusqua: pause,
        },
    })
}

pub async fn lire_pref_categorie(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    personne_id: Uuid,
    categorie: Categorie,
) -> Result<PrefCategorie, sqlx::Error> {
    let l: Option<(bool, bool, bool, bool)> = sqlx::query_as(
        "SELECT push, email, sms, resume FROM ent_ntf_preference
          WHERE etablissement_id = $1 AND personne_id = $2 AND categorie = $3",
    )
    .bind(etablissement_id)
    .bind(personne_id)
    .bind(categorie.code())
    .fetch_optional(&mut *conn)
    .await?;
    Ok(l.map(|(push, email, sms, resume)| PrefCategorie { push, email, sms, resume }).unwrap_or_default())
}

/// Écrit le réglage de la personne (heures calmes, résumé, fratrie, pause).
pub async fn definir_reglage(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    personne_id: Uuid,
    r: &ReglagePersonne,
    maintenant: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let mode = match r.mode_resume {
        ModeResume::Immediat => "immediat",
        ModeResume::Quotidien => "quotidien",
        ModeResume::Hebdo => "hebdo",
    };
    sqlx::query(
        "INSERT INTO ent_ntf_reglage_personne
            (etablissement_id, personne_id, preset, calme_debut, calme_fin, jours_calmes_masque, resume_mode,
             resume_heure, regrouper_fratrie, apercus_verrouille, pause_jusqua, maj_le)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
         ON CONFLICT (etablissement_id, personne_id) DO UPDATE SET
            preset = EXCLUDED.preset, calme_debut = EXCLUDED.calme_debut, calme_fin = EXCLUDED.calme_fin,
            jours_calmes_masque = EXCLUDED.jours_calmes_masque, resume_mode = EXCLUDED.resume_mode,
            resume_heure = EXCLUDED.resume_heure, regrouper_fratrie = EXCLUDED.regrouper_fratrie,
            apercus_verrouille = EXCLUDED.apercus_verrouille, pause_jusqua = EXCLUDED.pause_jusqua,
            maj_le = EXCLUDED.maj_le",
    )
    .bind(etablissement_id)
    .bind(personne_id)
    .bind(&r.preset)
    .bind(r.calme.plage.map(|p| heure(p.debut)))
    .bind(r.calme.plage.map(|p| heure(p.fin)))
    .bind(if r.calme.jours == 0 { None } else { Some(i16::from(r.calme.jours)) })
    .bind(mode)
    .bind(heure(r.heure_resume))
    .bind(r.regrouper_fratrie)
    .bind(r.apercus_verrouille)
    .bind(r.pause_jusqua)
    .bind(maintenant)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Applique un préréglage : réglage (mode du résumé) et préférences des dix catégories.
pub async fn appliquer_preset(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    personne_id: Uuid,
    preset: Preset,
    maintenant: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let mut r = lire_reglage(conn, etablissement_id, personne_id).await?;
    r.preset = preset.code().into();
    r.mode_resume = preset.mode_resume();
    definir_reglage(conn, etablissement_id, personne_id, &r, maintenant).await?;
    for c in Categorie::TOUTES {
        definir_pref_categorie(conn, etablissement_id, personne_id, c, preset.prefs(c), maintenant).await?;
    }
    Ok(())
}

pub async fn definir_pref_categorie(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    personne_id: Uuid,
    categorie: Categorie,
    p: PrefCategorie,
    maintenant: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ent_ntf_preference (etablissement_id, personne_id, categorie, push, email, sms, resume, maj_le)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
         ON CONFLICT (etablissement_id, personne_id, categorie) DO UPDATE SET
            push = EXCLUDED.push, email = EXCLUDED.email, sms = EXCLUDED.sms, resume = EXCLUDED.resume,
            maj_le = EXCLUDED.maj_le",
    )
    .bind(etablissement_id)
    .bind(personne_id)
    .bind(categorie.code())
    .bind(p.push)
    .bind(p.email)
    .bind(p.sms)
    .bind(p.resume)
    .bind(maintenant)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Reflète une écriture de l'ancien réglage (`definir_preferences_notification`) dans les nouvelles tables.
/// Sans personne rattachée au compte dans l'établissement, il n'y a rien à refléter.
pub async fn refleter_ancien_reglage(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    user_id: Uuid,
    pref: &Preferences,
    maintenant: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let personne: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
    )
    .bind(etablissement_id)
    .bind(user_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some(personne) = personne else {
        return Ok(());
    };
    let mut r = lire_reglage(conn, etablissement_id, personne).await?;
    r.calme = Calme {
        plage: match (pref.calme_debut, pref.calme_fin) {
            (Some(d), Some(f)) if d != f => Some(Plage { debut: d, fin: f }),
            _ => None,
        },
        jours: r.calme.jours,
    };
    r.mode_resume = if pref.resume == "quotidien" { ModeResume::Quotidien } else { ModeResume::Immediat };
    definir_reglage(conn, etablissement_id, personne, &r, maintenant).await?;
    let p = PrefCategorie { push: pref.push, email: pref.email, sms: pref.sms, resume: pref.resume == "quotidien" };
    for c in Categorie::TOUTES {
        definir_pref_categorie(conn, etablissement_id, personne, c, p, maintenant).await?;
    }
    Ok(())
}

/// Réglage de l'établissement lu pour décider du temps.
#[derive(Debug, Clone)]
pub struct ParametreEtab {
    pub calme: Calme,
    pub vacances_hebdo: bool,
    pub budget_u0: u32,
    pub fuseau: String,
}

pub async fn lire_parametre(conn: &mut PgConnection, etablissement_id: Uuid) -> Result<ParametreEtab, sqlx::Error> {
    let l: Option<(NaiveTime, NaiveTime, i16, String, i16)> = sqlx::query_as(
        "SELECT calme_debut, calme_fin, jours_calmes_masque, vacances_mode, budget_bruit_u0
           FROM ent_ntf_parametre WHERE etablissement_id = $1",
    )
    .bind(etablissement_id)
    .fetch_optional(&mut *conn)
    .await?;
    // Fuseau de l'établissement (Europe/Paris par défaut), jamais l'heure de Paris en dur.
    let fuseau: Option<String> =
        sqlx::query_scalar("SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1")
            .bind(etablissement_id)
            .fetch_optional(&mut *conn)
            .await?;
    let fuseau = fuseau.unwrap_or_else(|| "Europe/Paris".into());
    Ok(match l {
        // Défauts du §9.3 : 20 h 30–7 h 00, dimanche.
        None => ParametreEtab {
            calme: Calme { plage: Some(Plage { debut: 20 * 60 + 30, fin: 7 * 60 }), jours: 1 << 6 },
            vacances_hebdo: false,
            budget_u0: 5,
            fuseau,
        },
        Some((d, f, jours, vacances, budget)) => ParametreEtab {
            calme: Calme { plage: plage_depuis(Some(d), Some(f)), jours: jours as u8 },
            vacances_hebdo: vacances == "hebdo",
            budget_u0: budget as u32,
            fuseau,
        },
    })
}

/// Instant UTC → heure locale de l'établissement (changements d'heure inclus : c'est la base qui les connaît).
pub async fn local_de(conn: &mut PgConnection, fuseau: &str, instant: DateTime<Utc>) -> Result<NaiveDateTime, sqlx::Error> {
    sqlx::query_scalar("SELECT ($2::timestamptz AT TIME ZONE $1)")
        .bind(fuseau)
        .bind(instant)
        .fetch_one(&mut *conn)
        .await
}

/// Heure locale de l'établissement → instant UTC.
pub async fn vers_utc(conn: &mut PgConnection, fuseau: &str, local: NaiveDateTime) -> Result<DateTime<Utc>, sqlx::Error> {
    sqlx::query_scalar("SELECT ($2::timestamp AT TIME ZONE $1)")
        .bind(fuseau)
        .bind(local)
        .fetch_one(&mut *conn)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deux_heures_egales_ne_font_pas_une_plage() {
        let t = |h| NaiveTime::from_hms_opt(h, 0, 0);
        assert_eq!(plage_depuis(t(0), t(0)), None);
        assert_eq!(plage_depuis(t(20), None), None);
        assert_eq!(plage_depuis(t(21), t(7)), Some(Plage { debut: 21 * 60, fin: 7 * 60 }));
    }

    #[test]
    fn les_preregalages_ne_donnent_jamais_de_sms_sauf_tout() {
        for c in Categorie::TOUTES {
            assert!(!Preset::Equilibre.prefs(c).sms);
            assert!(!Preset::Essentiel.prefs(c).sms && !Preset::Discret.prefs(c).sms);
            assert!(Preset::Tout.prefs(c).sms);
        }
        assert_eq!(Preset::depuis_code("tout"), Some(Preset::Tout));
        assert_eq!(Preset::depuis_code("inconnu"), None);
        assert_eq!(Preset::Essentiel.mode_resume(), ModeResume::Quotidien);
    }
}
