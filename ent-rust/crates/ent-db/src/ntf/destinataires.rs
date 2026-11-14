//! Socle 8, lot N2 — destinataires = **personnes** (`08_NOTIFICATIONS.md` §9.1, §15).
//!
//! Le port `destinataires` du socle 4 (A11) n'existe pas encore : cet adaptateur provisoire lit
//! `ent_eleve_lien` et `ent_personne`. Il retourne **toutes** les personnes liées à l'élève, retenues ou
//! exclues avec leur raison, au lieu de ne montrer que les comptes qui restent :
//!
//! - responsable **sans compte** (SMS, e-mail, tâche manuelle) ;
//! - exclusions tracées : fusion, lien suspendu, hors dates, interdiction de communication, « ne pas
//!   solliciter », droit du lien insuffisant, aucune coordonnée, compte non responsable ;
//! - joignabilité par canal (push, e-mail, SMS) et plancher de la nature ;
//! - dédoublonnage par destination (deux parents, un même numéro, un seul envoi, CN-33).
//!
//! Le jour d'établissement vient de `ent_aujourdhui_etab(établissement, instant)` : jamais `current_date`.
//! L'élève lui-même (`EleveDest`) et les **organisations** (organisme payeur, `organisation_ref`) restent à
//! brancher quand leurs sources existent (socles 4 et 24) ; le schéma les porte déjà.

use super::contrat::*;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use std::collections::HashSet;
use uuid::Uuid;

/// Code d'exclusion (stocké dans `ent_ntf_destinataire.exclusion_code`).
pub mod exclusion {
    pub const PERSONNE_FUSIONNEE: &str = "personne_fusionnee";
    pub const LIEN_SUSPENDU: &str = "lien_suspendu";
    pub const LIEN_HORS_DATES: &str = "lien_hors_dates";
    pub const INTERDICTION: &str = "interdiction";
    pub const NE_PAS_SOLLICITER: &str = "ne_pas_solliciter";
    pub const DROIT_INSUFFISANT: &str = "droit_insuffisant";
    pub const COMPTE_NON_RESPONSABLE: &str = "compte_non_responsable";
    pub const AUCUNE_COORDONNEE: &str = "aucune_coordonnee";
    pub const GEL: &str = "gel";
    /// Exclusions qui relèvent d'une **mesure de protection** : jamais détaillées à qui n'est pas habilité (R-N51).
    pub const SENSIBLES: &[&str] = &[INTERDICTION, NE_PAS_SOLLICITER, GEL];
}

/// Une personne liée à l'élève, avec tout ce qu'il faut pour la classer.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LigneLien {
    pub personne_id: Uuid,
    pub compte_id: Option<Uuid>,
    pub foyer_id: Option<Uuid>,
    pub peut_voir_vs: bool,
    pub peut_voir_notes: bool,
    pub fusionnee: bool,
    pub suspendu: bool,
    pub hors_dates: bool,
    pub interdit: bool,
    pub ne_pas_solliciter: bool,
    /// Le compte est membre « parent » de l'établissement (sans objet pour une personne sans compte).
    pub membre_parent: bool,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub push: bool,
}

/// Classe une ligne : `None` = retenue, sinon le code d'exclusion. Ordre = gravité de la raison.
pub fn classer(l: &LigneLien, droit: DroitRequis) -> Option<&'static str> {
    use exclusion::*;
    if l.fusionnee {
        return Some(PERSONNE_FUSIONNEE);
    }
    if l.suspendu {
        return Some(LIEN_SUSPENDU);
    }
    if l.hors_dates {
        return Some(LIEN_HORS_DATES);
    }
    if l.interdit {
        return Some(INTERDICTION);
    }
    if l.ne_pas_solliciter {
        return Some(NE_PAS_SOLLICITER);
    }
    let droit_ok = match droit {
        DroitRequis::Lien => true,
        DroitRequis::VieScolaire => l.peut_voir_vs,
        DroitRequis::Notes => l.peut_voir_notes,
    };
    if !droit_ok {
        return Some(DROIT_INSUFFISANT);
    }
    if l.compte_id.is_some() && !l.membre_parent {
        return Some(COMPTE_NON_RESPONSABLE);
    }
    if l.compte_id.is_none() && !l.a_une_coordonnee() {
        return Some(AUCUNE_COORDONNEE);
    }
    None
}

impl LigneLien {
    fn email_utile(&self) -> Option<String> {
        self.email.as_deref().map(|e| e.trim().to_lowercase()).filter(|e| e.contains('@'))
    }
    fn telephone_utile(&self) -> Option<String> {
        self.telephone.as_deref().map(ent_sms::normalize_phone).filter(|t| !t.is_empty())
    }
    pub fn a_une_coordonnee(&self) -> bool {
        self.email_utile().is_some() || self.telephone_utile().is_some()
    }
}

/// Un destinataire résolu.
#[derive(Debug, Clone)]
pub struct Resolu {
    pub ligne: LigneLien,
    /// `None` = retenu.
    pub exclusion: Option<&'static str>,
}

/// Lecture : toutes les personnes liées à l'élève, classées à l'instant `maintenant` (jour d'établissement).
pub async fn personnes_de_l_eleve(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    droit: DroitRequis,
    maintenant: DateTime<Utc>,
) -> Result<Vec<Resolu>, sqlx::Error> {
    let lignes: Vec<LigneLien> = sqlx::query_as(
        "SELECT l.personne_id, l.user_id AS compte_id, l.foyer_id, l.peut_voir_vs, l.peut_voir_notes,
                (p.fusionnee_dans IS NOT NULL) AS fusionnee,
                (l.suspendu_le IS NOT NULL) AS suspendu,
                NOT ((l.date_debut IS NULL OR l.date_debut <= ent_aujourdhui_etab(l.etablissement_id, $3))
                 AND (l.date_fin IS NULL OR l.date_fin >= ent_aujourdhui_etab(l.etablissement_id, $3))) AS hors_dates,
                EXISTS (SELECT 1 FROM ent_eleve_interdiction i
                         WHERE i.etablissement_id = l.etablissement_id AND i.eleve_id = l.eleve_id
                           AND i.levee_le IS NULL AND 'communication' = ANY (i.objet)
                           AND i.du <= ent_aujourdhui_etab(l.etablissement_id, $3)
                           AND (i.au IS NULL OR i.au >= ent_aujourdhui_etab(l.etablissement_id, $3))
                           AND i.personne_id IS NOT NULL
                           AND (i.personne_id = l.personne_id
                                OR i.personne_id IN (SELECT s.id FROM ent_personne s
                                                      WHERE s.etablissement_id = l.etablissement_id
                                                        AND s.fusionnee_dans = l.personne_id))) AS interdit,
                EXISTS (SELECT 1 FROM ent_personne_reserve r
                         WHERE r.etablissement_id = l.etablissement_id AND r.personne_id = l.personne_id
                           AND r.ne_pas_solliciter) AS ne_pas_solliciter,
                (m.user_id IS NOT NULL) AS membre_parent,
                COALESCE(NULLIF(btrim(u.email), ''), p.email) AS email,
                COALESCE(NULLIF(btrim(u.telephone), ''), p.telephone) AS telephone,
                EXISTS (SELECT 1 FROM ent_push_device d
                         WHERE d.etablissement_id = l.etablissement_id AND d.user_id = l.user_id
                           AND d.revoked_at IS NULL) AS push
           FROM ent_eleve_lien l
           JOIN ent_personne p ON p.id = l.personne_id AND p.etablissement_id = l.etablissement_id
           LEFT JOIN ent_user u ON u.id = l.user_id
           LEFT JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
                                     AND m.role = 'parent'
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2
          ORDER BY l.created_at, l.id",
    )
    .bind(etablissement_id)
    .bind(eleve_id)
    .bind(maintenant)
    .fetch_all(&mut *conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|ligne| {
            let exclusion = classer(&ligne, droit);
            Resolu { ligne, exclusion }
        })
        .collect())
}

/// Un sujet est-il gelé (décès de l'élève, CN-13) ?
pub async fn sujet_gele(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    sujet_type: &str,
    sujet_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let g: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM ent_ntf_gel WHERE etablissement_id = $1 AND sujet_type = $2 AND sujet_id = $3
            AND leve_le IS NULL LIMIT 1",
    )
    .bind(etablissement_id)
    .bind(sujet_type)
    .bind(sujet_id)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(g.is_some())
}

/// Relecture à la remise (R-N1) : la personne est-elle encore destinataire ? `Ok(None)` = oui, sinon le code.
pub async fn reclasser(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    personne_id: Uuid,
    droit: DroitRequis,
    maintenant: DateTime<Utc>,
) -> Result<Option<&'static str>, sqlx::Error> {
    if sujet_gele(conn, etablissement_id, "eleve", eleve_id).await? {
        return Ok(Some(exclusion::GEL));
    }
    let toutes = personnes_de_l_eleve(conn, etablissement_id, eleve_id, droit, maintenant).await?;
    Ok(toutes
        .into_iter()
        .find(|r| r.ligne.personne_id == personne_id)
        .map(|r| r.exclusion)
        .unwrap_or(Some(exclusion::LIEN_SUSPENDU)))
}

/// Canaux externes à planifier pour une personne (§9.2) et besoin éventuel d'une tâche manuelle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanCanaux {
    pub canaux: Vec<Canal>,
    pub a_prevenir: bool,
}

/// Algorithme des canaux (R-N10) : canaux par défaut ∩ joignables ; si le plancher n'est pas satisfait, on
/// ajoute dans l'ordre push → e-mail → SMS (U2/U3 et autorisé) ; sinon tâche manuelle. U3 : tous les joignables.
/// Une personne sans compte n'a ni ENT ni push ; son SMS attend les gabarits sans compte (lot N4).
pub fn planifier_canaux(n: &NatureAvisDecl, l: &LigneLien, sms_u2_actif: bool) -> PlanCanaux {
    let a_compte = l.compte_id.is_some();
    let push = a_compte && l.push;
    let email = l.email_utile().is_some();
    let sms = a_compte && l.telephone_utile().is_some();
    let joignable = |c: Canal| match c {
        Canal::InApp => a_compte,
        Canal::Push => push,
        Canal::Email => email,
        Canal::Sms => sms,
    };
    let mut canaux: Vec<Canal> = Vec::new();
    if a_compte {
        canaux.push(Canal::InApp);
    }
    for c in n.canaux_defaut.iter().copied().filter(|c| c.externe() && joignable(*c)) {
        canaux.push(c);
    }
    let externe = |v: &[Canal]| v.iter().any(|c| c.externe());
    if n.plancher == Plancher::Tous {
        for c in [Canal::Push, Canal::Email, Canal::Sms] {
            if joignable(c) && !canaux.contains(&c) {
                canaux.push(c);
            }
        }
    }
    if n.plancher != Plancher::InApp && !externe(&canaux) {
        for c in [Canal::Push, Canal::Email, Canal::Sms] {
            let autorise = c != Canal::Sms || (n.urgence >= Urgence::U2 && sms_u2_actif);
            if joignable(c) && autorise {
                canaux.push(c);
                break;
            }
        }
    }
    let a_prevenir = n.plancher != Plancher::InApp && !externe(&canaux);
    PlanCanaux { canaux, a_prevenir }
}

/// Destinations déjà atteintes pour un avis : une seule remise par destination et par avis (R-N22, CN-33).
#[derive(Debug, Default)]
pub struct Destinations {
    emails: HashSet<String>,
    telephones: HashSet<String>,
}

impl Destinations {
    /// `true` si la destination est nouvelle (la remise a lieu), `false` si elle est déjà servie.
    pub fn reserver(&mut self, canal: Canal, l: &LigneLien) -> bool {
        match canal {
            Canal::Email => l.email_utile().map(|e| self.emails.insert(e)).unwrap_or(true),
            Canal::Sms => l.telephone_utile().map(|t| self.telephones.insert(t)).unwrap_or(true),
            // ENT et push sont propres au compte : jamais dédoublonnés entre personnes.
            Canal::InApp | Canal::Push => true,
        }
    }
}

/// Résumé des exclusions pour un lecteur (R-N51) : le détail à qui est habilité, un nombre agrégé sinon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExclusionsVues {
    Detail(Vec<(Uuid, String)>),
    /// Exclus et non joignables fondus en un seul nombre, « moins de 3 » en dessous du seuil.
    Agrege(String),
}

pub fn exclusions_vues(exclus: &[(Uuid, String)], non_joignables: usize, habilite: bool) -> ExclusionsVues {
    if habilite {
        return ExclusionsVues::Detail(exclus.to_vec());
    }
    let n = exclus.len() + non_joignables;
    ExclusionsVues::Agrege(if n == 0 {
        "aucun".to_string()
    } else if n < 3 {
        "moins de 3".to_string()
    } else {
        n.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntf::catalogue::{ABSENCE_CONSTATEE, RETARD_CONSTATE};

    fn ligne() -> LigneLien {
        LigneLien {
            personne_id: Uuid::nil(),
            compte_id: Some(Uuid::nil()),
            foyer_id: None,
            peut_voir_vs: true,
            peut_voir_notes: true,
            fusionnee: false,
            suspendu: false,
            hors_dates: false,
            interdit: false,
            ne_pas_solliciter: false,
            membre_parent: true,
            email: Some("Parent@Example.invalid".into()),
            telephone: Some("0692123456".into()),
            push: true,
        }
    }

    #[test]
    fn classement_par_gravite_et_droit_du_lien() {
        assert_eq!(classer(&ligne(), DroitRequis::VieScolaire), None);
        let mut l = ligne();
        l.peut_voir_vs = false;
        assert_eq!(classer(&l, DroitRequis::VieScolaire), Some("droit_insuffisant"));
        assert_eq!(classer(&l, DroitRequis::Lien), None, "avis d'infirmerie : le lien suffit");
        l.interdit = true;
        assert_eq!(classer(&l, DroitRequis::Lien), Some("interdiction"));
        l.suspendu = true;
        assert_eq!(classer(&l, DroitRequis::Lien), Some("lien_suspendu"), "suspendu passe avant");
        let mut l = ligne();
        l.ne_pas_solliciter = true;
        assert_eq!(classer(&l, DroitRequis::Lien), Some("ne_pas_solliciter"));
        let mut l = ligne();
        l.membre_parent = false;
        assert_eq!(classer(&l, DroitRequis::Lien), Some("compte_non_responsable"));
        let mut l = ligne();
        (l.compte_id, l.email, l.telephone) = (None, None, None);
        assert_eq!(classer(&l, DroitRequis::Lien), Some("aucune_coordonnee"));
        l.email = Some("sans-compte@example.invalid".into());
        assert_eq!(classer(&l, DroitRequis::Lien), None, "personne sans compte mais joignable");
    }

    #[test]
    fn canaux_avec_compte_et_sans_compte() {
        let l = ligne();
        let p = planifier_canaux(&ABSENCE_CONSTATEE, &l, false);
        assert_eq!(p.canaux, vec![Canal::InApp, Canal::Push]);
        assert!(!p.a_prevenir);
        // Sans appareil : le plancher « un externe » ajoute l'e-mail (le SMS exige U2 et le réglage).
        let mut sans_push = ligne();
        sans_push.push = false;
        let p = planifier_canaux(&ABSENCE_CONSTATEE, &sans_push, false);
        assert_eq!(p.canaux, vec![Canal::InApp, Canal::Email]);
        // Ni appareil ni e-mail : SMS si U2 et autorisé, sinon tâche manuelle.
        sans_push.email = None;
        assert!(planifier_canaux(&ABSENCE_CONSTATEE, &sans_push, false).a_prevenir);
        let p = planifier_canaux(&ABSENCE_CONSTATEE, &sans_push, true);
        assert_eq!(p.canaux, vec![Canal::InApp, Canal::Sms]);
        // Plancher « ENT » : jamais de tâche manuelle.
        let p = planifier_canaux(&RETARD_CONSTATE, &sans_push, false);
        assert!(!p.a_prevenir && p.canaux == vec![Canal::InApp]);
        // Sans compte : e-mail seulement, jamais d'ENT ni de push ; sans e-mail, tâche manuelle.
        let mut sans_compte = ligne();
        sans_compte.compte_id = None;
        let p = planifier_canaux(&crate::ntf::catalogue::PASSAGE_INFIRMERIE, &sans_compte, true);
        assert_eq!(p.canaux, vec![Canal::Email]);
        sans_compte.email = None;
        let p = planifier_canaux(&crate::ntf::catalogue::PASSAGE_INFIRMERIE, &sans_compte, true);
        assert!(p.canaux.is_empty() && p.a_prevenir);
    }

    #[test]
    fn une_seule_remise_par_destination_cn33() {
        let mut d = Destinations::default();
        let a = ligne();
        let mut b = ligne();
        b.email = Some(" parent@example.INVALID ".into());
        assert!(d.reserver(Canal::Email, &a));
        assert!(!d.reserver(Canal::Email, &b), "même adresse, casse et espaces ignorés");
        assert!(d.reserver(Canal::InApp, &b) && d.reserver(Canal::InApp, &a), "l'ENT n'est jamais dédoublonné");
        let mut c = ligne();
        c.telephone = Some("06 92 12 34 56".into());
        assert!(d.reserver(Canal::Sms, &a));
        assert!(!d.reserver(Canal::Sms, &c), "même numéro écrit autrement");
    }

    #[test]
    fn exclusions_agregees_pour_qui_n_est_pas_habilite_r_n51() {
        let e = vec![(Uuid::new_v4(), "interdiction".to_string())];
        assert!(matches!(exclusions_vues(&e, 0, true), ExclusionsVues::Detail(d) if d.len() == 1));
        assert_eq!(exclusions_vues(&e, 0, false), ExclusionsVues::Agrege("moins de 3".into()));
        assert_eq!(exclusions_vues(&[], 0, false), ExclusionsVues::Agrege("aucun".into()));
        let trois: Vec<(Uuid, String)> = (0..2).map(|_| (Uuid::new_v4(), "gel".to_string())).collect();
        assert_eq!(exclusions_vues(&trois, 1, false), ExclusionsVues::Agrege("3".into()));
    }
}
