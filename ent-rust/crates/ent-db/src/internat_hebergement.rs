//! Hébergement de l'internat (socle 41, lots I0 et I1) : affectation sous contraintes, séjours issus
//! du régime, rythmes versionnés, zones, lits, plan, dérogations, paramètres.
//!
//! Règles appliquées ici (voir `domain_eleve::internat` pour la partie pure) :
//! * **R-IT1** le séjour naît du régime daté du socle 2 (`ent_eleve_periode`), jamais d'une saisie ;
//! * **R-IT3** pas de suppression : fin datée ;
//! * **R-IT4** rythme versionné (ajout seul) ;
//! * **R-IT10** une commande = une transaction ; la chambre est verrouillée (`FOR UPDATE`) ; la base
//!   garde en plus la capacité (migration 2820) et l'exclusion lit × dates (migration 2823) ;
//! * **R-IT11** public de la chambre contre le sexe d'état civil ; dérogation de la direction ;
//! * **R-IT12** interdiction de contact ; dérogation de la direction ;
//! * **R-IT32** le plan ne porte aucun nom.
//!
//! Droits : `internat.gerer` (avec délégation D-017) ; dérogations, interdictions et paramètres de
//! la direction (administrateur compris).

use crate::finances::publier;
use crate::internat_repo::{
    get_affectation_ouverte_tx, get_chambre_tx, insert_affectation_tx, insert_chambre_tx,
    map_sqlx, periodes_chambre_tx, update_affectation_fin_tx, update_chambre_tx,
};
use crate::outbox::Causalite;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{Duration, NaiveDate, NaiveTime};
use domain_eleve::{
    pic_occupation, plan_affecter_eleve_internat, plan_creer_chambre_internat,
    plan_modifier_chambre_internat, AffectationInternat, AffecterEleveInternat, ChambreInternat,
    ContexteAffectation, CreerChambreInternat, DomainError, EtatChambre, ModifierChambreInternat,
    PublicChambre, RegimeCode, Rythme, MOTIFS_AFFECTATION,
};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind};
use ent_types::internat::*;
use serde_json::json;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx(e))
}

fn invalide(msg: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(msg)
}

fn rythme_vers_dto(r: &Rythme) -> JoursRythmeDto {
    JoursRythmeDto {
        dimanche: r.dimanche,
        lundi: r.lundi,
        mardi: r.mardi,
        mercredi: r.mercredi,
        jeudi: r.jeudi,
        vendredi: r.vendredi,
        samedi: r.samedi,
    }
}

fn dto_vers_rythme(j: &JoursRythmeDto) -> Rythme {
    Rythme {
        dimanche: j.dimanche,
        lundi: j.lundi,
        mardi: j.mardi,
        mercredi: j.mercredi,
        jeudi: j.jeudi,
        vendredi: j.vendredi,
        samedi: j.samedi,
    }
}

/// Demande d'affectation (toutes les entrées d'INT-116 / INT-117).
#[derive(Debug, Clone)]
pub struct DemandeAffectation {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub chambre_id: Uuid,
    pub lit_id: Option<Uuid>,
    pub lit_libelle: Option<String>,
    pub debut: NaiveDate,
    pub actor_user_id: Uuid,
    pub motif_code: Option<String>,
    pub besoin_accessibilite: bool,
}

impl DemandeAffectation {
    pub fn depuis_cmd(c: AffecterEleveInternat) -> Self {
        Self {
            etablissement_id: c.etablissement_id,
            eleve_id: c.eleve_id,
            chambre_id: c.chambre_id,
            lit_id: None,
            lit_libelle: c.lit,
            debut: c.debut,
            actor_user_id: c.actor_user_id,
            motif_code: None,
            besoin_accessibilite: false,
        }
    }
}

fn est_direction(role: &str) -> bool {
    matches!(Role::parse(role), Some(Role::Direction | Role::Admin))
}

// ───────────────────────── Lectures sur transaction ─────────────────────────

/// Régime de l'élève à une date, d'après les périodes datées du socle 2 ; à défaut de toute période,
/// le régime courant de la fiche. `None` : inconnu ou non interprétable (R-IT2, jamais de devinette).
pub(crate) async fn regime_a_la_date_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    jour: NaiveDate,
) -> Result<Option<RegimeCode>, EntDbError> {
    let periode: Option<String> = sqlx::query_scalar(
        "SELECT valeur FROM ent_eleve_periode \
          WHERE etablissement_id = $1 AND eleve_id = $2 AND kind = 'regime' \
            AND date_debut <= $3 AND (date_fin IS NULL OR date_fin >= $3) \
          ORDER BY date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .bind(eleve)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if let Some(v) = periode {
        return Ok(RegimeCode::depuis_texte(&v));
    }
    let nb: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM ent_eleve_periode \
          WHERE etablissement_id = $1 AND eleve_id = $2 AND kind = 'regime'",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if nb > 0 {
        return Ok(None);
    }
    let regime: Option<Option<String>> = sqlx::query_scalar(
        "SELECT regime FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(regime.flatten().and_then(|r| RegimeCode::depuis_texte(&r)))
}

pub(crate) async fn parametres_tx(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<ParametresInternatDto, EntDbError> {
    let row = sqlx::query(
        "SELECT heure_limite_annonce, heure_limite_appel, seuil_retard_minutes, \
                heure_recuperation_vendredi, jour_bilan, voisins_visibles_familles, depot_telephones, \
                garde_honorabilite, impression_presents, majeur_info_parents, rythme_dimanche, \
                rythme_lundi, rythme_mardi, rythme_mercredi, rythme_jeudi, rythme_vendredi, rythme_samedi \
           FROM ent_internat_parametres WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some(r) = row else {
        return Ok(parametres_defaut());
    };
    Ok(ParametresInternatDto {
        heure_limite_annonce: r.get("heure_limite_annonce"),
        heure_limite_appel: r.get("heure_limite_appel"),
        seuil_retard_minutes: r.get("seuil_retard_minutes"),
        heure_recuperation_vendredi: r.get("heure_recuperation_vendredi"),
        jour_bilan: r.get("jour_bilan"),
        voisins_visibles_familles: r.get("voisins_visibles_familles"),
        depot_telephones: r.get("depot_telephones"),
        garde_honorabilite: r.get("garde_honorabilite"),
        impression_presents: r.get("impression_presents"),
        majeur_info_parents: r.get("majeur_info_parents"),
        rythme_defaut: JoursRythmeDto {
            dimanche: r.get("rythme_dimanche"),
            lundi: r.get("rythme_lundi"),
            mardi: r.get("rythme_mardi"),
            mercredi: r.get("rythme_mercredi"),
            jeudi: r.get("rythme_jeudi"),
            vendredi: r.get("rythme_vendredi"),
            samedi: r.get("rythme_samedi"),
        },
    })
}

/// Défauts de §28 (S-4100 à S-4117).
pub fn parametres_defaut() -> ParametresInternatDto {
    let t = |h, m| NaiveTime::from_hms_opt(h, m, 0).expect("heure valide");
    ParametresInternatDto {
        heure_limite_annonce: t(16, 0),
        heure_limite_appel: t(21, 30),
        seuil_retard_minutes: 30,
        heure_recuperation_vendredi: t(18, 0),
        jour_bilan: 4,
        voisins_visibles_familles: false,
        depot_telephones: false,
        garde_honorabilite: false,
        impression_presents: true,
        majeur_info_parents: "si_a_charge".into(),
        rythme_defaut: rythme_vers_dto(&Rythme::DEFAUT),
    }
}

async fn aujourdhui_tx(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, now())")
        .bind(etab)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

// ───────────────────────── Séjours (R-IT1) ─────────────────────────

/// Resynchronise les séjours d'internat d'un élève sur son régime daté (INT-110, INT-112) et renvoie
/// le séjour qui couvre `jour_ref`. Idempotent. Un séjour est ouvert pour chaque période où le
/// régime est « interne » (périodes contiguës fusionnées) et clos quand le régime change.
pub(crate) async fn synchroniser_sejours_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    jour_ref: NaiveDate,
    actor: Option<Uuid>,
) -> Result<Option<Uuid>, EntDbError> {
    let periodes: Vec<(String, NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT valeur, date_debut, date_fin FROM ent_eleve_periode \
          WHERE etablissement_id = $1 AND eleve_id = $2 AND kind = 'regime' ORDER BY date_debut",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut cibles: Vec<(NaiveDate, Option<NaiveDate>)> = Vec::new();
    if periodes.is_empty() {
        let regime: Option<Option<String>> = sqlx::query_scalar(
            "SELECT regime FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if regime
            .flatten()
            .and_then(|r| RegimeCode::depuis_texte(&r))
            .is_some_and(RegimeCode::est_heberge)
        {
            cibles.push((jour_ref, None));
        }
    }
    for (valeur, debut, fin) in &periodes {
        if !RegimeCode::depuis_texte(valeur).is_some_and(RegimeCode::est_heberge) {
            continue;
        }
        match cibles.last_mut() {
            Some((_, dernier_fin))
                if dernier_fin.is_some_and(|f| *debut <= f + Duration::days(1)) =>
            {
                *dernier_fin = *fin;
            }
            _ => cibles.push((*debut, *fin)),
        }
    }

    let existants: Vec<(Uuid, NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT id, du, au FROM ent_internat_sejour \
          WHERE etablissement_id = $1 AND eleve_id = $2 AND origine = 'regime' ORDER BY du",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let chevauche = |a: (NaiveDate, Option<NaiveDate>), b: (NaiveDate, Option<NaiveDate>)| {
        a.0 <= b.1.unwrap_or(NaiveDate::MAX) && b.0 <= a.1.unwrap_or(NaiveDate::MAX)
    };
    let mut apparies: Vec<Uuid> = Vec::new();
    let mut courant: Option<Uuid> = None;
    for (du, au) in &cibles {
        let existant = existants
            .iter()
            .find(|(id, d, a)| !apparies.contains(id) && chevauche((*d, *a), (*du, *au)));
        let id = match existant {
            Some((id, d, a)) => {
                apparies.push(*id);
                if d != du || a != au {
                    sqlx::query(
                        "UPDATE ent_internat_sejour SET du = $3, au = $4, modifie_le = now(), \
                                ferme_motif_code = CASE WHEN $4::date IS NULL THEN NULL \
                                                        ELSE COALESCE(ferme_motif_code, 'changement_regime') END \
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(id)
                    .bind(du)
                    .bind(au)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                    publier_sejour(conn, etab, eleve, *id, *du, *au, actor, "mis_a_jour").await?;
                }
                *id
            }
            None => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_internat_sejour (id, etablissement_id, eleve_id, du, au, origine) \
                     VALUES ($1,$2,$3,$4,$5,'regime')",
                )
                .bind(id)
                .bind(etab)
                .bind(eleve)
                .bind(du)
                .bind(au)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
                // Rythme initial = rythme par défaut de l'établissement (S-4100), version à la date du séjour.
                let defaut = parametres_tx(conn, etab).await?.rythme_defaut;
                inserer_rythme_tx(conn, etab, id, *du, &dto_vers_rythme(&defaut), actor).await?;
                publier_sejour(conn, etab, eleve, id, *du, *au, actor, "ouvert").await?;
                id
            }
        };
        if *du <= jour_ref && au.map(|a| jour_ref <= a).unwrap_or(true) {
            courant = Some(id);
        }
    }
    // Séjour ouvert dont la période de régime a disparu (régime corrigé) : clos, jamais supprimé.
    for (id, du, au) in &existants {
        if !apparies.contains(id) && au.is_none() {
            sqlx::query(
                "UPDATE ent_internat_sejour SET au = du, ferme_motif_code = 'correction_regime', \
                        modifie_le = now() WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            publier_sejour(conn, etab, eleve, *id, *du, Some(*du), actor, "clos").await?;
        }
    }
    Ok(courant)
}

#[allow(clippy::too_many_arguments)]
async fn publier_sejour(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    sejour: Uuid,
    du: NaiveDate,
    au: Option<NaiveDate>,
    actor: Option<Uuid>,
    etat: &str,
) -> Result<(), EntDbError> {
    publier(
        conn,
        etab,
        types::INTERNAT_SEJOUR,
        AggregateKind::Internat,
        sejour,
        Some(eleve),
        json!({ "du": du.to_string(), "au": au.map(|a| a.to_string()), "etat": etat }),
        actor,
        Causalite::default(),
    )
    .await?;
    Ok(())
}

async fn inserer_rythme_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    sejour: Uuid,
    du: NaiveDate,
    r: &Rythme,
    actor: Option<Uuid>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_internat_rythme (id, etablissement_id, sejour_id, du, dimanche, lundi, mardi, \
                mercredi, jeudi, vendredi, samedi, cree_par_user_id) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(sejour)
    .bind(du)
    .bind(r.dimanche)
    .bind(r.lundi)
    .bind(r.mardi)
    .bind(r.mercredi)
    .bind(r.jeudi)
    .bind(r.vendredi)
    .bind(r.samedi)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Ferme les séjours d'un élève à son départ (sortie de l'établissement) : fin datée, jamais de suppression.
pub(crate) async fn clore_sejours_depart_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    au: NaiveDate,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_internat_sejour SET au = GREATEST(du, $3), ferme_motif_code = 'depart', modifie_le = now() \
          WHERE etablissement_id = $1 AND eleve_id = $2 AND (au IS NULL OR au > $3)",
    )
    .bind(etab)
    .bind(eleve)
    .bind(au)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

// ───────────────────────── Affectation (INT-116, INT-117) ─────────────────────────

/// Cœur de l'affectation, sur une transaction déjà ouverte : verrou de la chambre, contexte, règles
/// pures, choix du lit, insertion. À appeler **dans la même transaction** que la fin éventuelle de
/// l'affectation précédente (changement de chambre).
async fn affecter_tx(
    conn: &mut PgConnection,
    d: &DemandeAffectation,
) -> Result<AffectationInternat, EntDbError> {
    let etab = d.etablissement_id;
    // Verrou de ligne : sérialise les affectations concurrentes de la même chambre (R-IT10, NJ-54).
    let chambre = get_chambre_tx(conn, etab, d.chambre_id, true)
        .await?
        .ok_or(DomainError::InternatChambreInvalide)?;
    if let Some(m) = &d.motif_code {
        if !MOTIFS_AFFECTATION.contains(&m.as_str()) {
            return Err(invalide("motif d'affectation inconnu"));
        }
    }
    let sexe: Option<Option<String>> = sqlx::query_scalar(
        "SELECT sexe FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(d.eleve_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let sexe = sexe.ok_or(DomainError::NotFound)?;

    // Zone fermée (travaux) : refus comme pour une chambre fermée.
    if let Some(zone_id) = chambre.zone_id {
        let z: Option<(Option<NaiveDate>, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT ferme_du, ferme_au FROM ent_internat_zone WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(zone_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if let Some((Some(du), au)) = z {
            if du <= d.debut && au.map(|a| d.debut <= a).unwrap_or(true) {
                return Err(DomainError::InternatChambreFermee.into());
            }
        }
    }

    let regime = regime_a_la_date_tx(conn, etab, d.eleve_id, d.debut).await?;
    let sejour_id = if regime.is_some_and(RegimeCode::est_heberge) {
        synchroniser_sejours_tx(conn, etab, d.eleve_id, d.debut, Some(d.actor_user_id)).await?
    } else {
        None
    };
    let deja_affecte = get_affectation_ouverte_tx(conn, etab, d.eleve_id).await?.is_some();
    let periodes = periodes_chambre_tx(conn, etab, d.chambre_id, d.debut).await?;
    let pic = pic_occupation(&periodes, d.debut);

    let derogation_public: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_internat_derogation \
          WHERE etablissement_id = $1 AND nature = 'public_chambre' AND eleve_id = $2 AND chambre_id = $3 \
            AND du <= $4 AND (au IS NULL OR au >= $4))",
    )
    .bind(etab)
    .bind(d.eleve_id)
    .bind(d.chambre_id)
    .bind(d.debut)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;

    let interdits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM (\
            SELECT DISTINCT a.eleve_id FROM ent_internat_affectation a \
             WHERE a.etablissement_id = $1 AND a.chambre_id = $2 AND a.eleve_id <> $3 \
               AND (a.fin IS NULL OR a.fin >= $4)) occ \
          WHERE EXISTS (SELECT 1 FROM ent_internat_interdiction_contact i \
                         WHERE i.etablissement_id = $1 \
                           AND ((i.eleve_a_id = $3 AND i.eleve_b_id = occ.eleve_id) \
                             OR (i.eleve_a_id = occ.eleve_id AND i.eleve_b_id = $3)) \
                           AND (i.au IS NULL OR i.au >= $4)) \
            AND NOT EXISTS (SELECT 1 FROM ent_internat_derogation g \
                             WHERE g.etablissement_id = $1 AND g.nature = 'interdiction_contact' \
                               AND ((g.eleve_id = $3 AND g.avec_eleve_id = occ.eleve_id) \
                                 OR (g.eleve_id = occ.eleve_id AND g.avec_eleve_id = $3)) \
                               AND g.du <= $4 AND (g.au IS NULL OR g.au >= $4))",
    )
    .bind(etab)
    .bind(d.chambre_id)
    .bind(d.eleve_id)
    .bind(d.debut)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;

    let ctx = ContexteAffectation {
        regime,
        sejour_id,
        pic_occupants: pic,
        deja_affecte,
        sexe,
        derogation_public,
        interdits_sans_derogation: interdits.clamp(0, i64::from(u16::MAX)) as u16,
        besoin_accessibilite: d.besoin_accessibilite,
    };
    let mut a = plan_affecter_eleve_internat(
        AffecterEleveInternat {
            etablissement_id: etab,
            eleve_id: d.eleve_id,
            chambre_id: d.chambre_id,
            lit: d.lit_libelle.clone(),
            debut: d.debut,
            actor_user_id: d.actor_user_id,
        },
        &chambre,
        &ctx,
    )?;
    a.motif_code = d.motif_code.clone();

    // Lit : celui demandé, sinon le premier libre ; jamais neutralisé, jamais occupé à partir de la date.
    let lit: Option<(Uuid, i16)> = sqlx::query_as(
        "SELECT l.id, l.numero FROM ent_internat_lit l \
          WHERE l.etablissement_id = $1 AND l.chambre_id = $2 AND ($4::uuid IS NULL OR l.id = $4) \
            AND (l.neutralise_du IS NULL OR (l.neutralise_au IS NOT NULL AND l.neutralise_au < $3)) \
            AND NOT EXISTS (SELECT 1 FROM ent_internat_affectation x \
                             WHERE x.etablissement_id = l.etablissement_id AND x.lit_id = l.id \
                               AND (x.fin IS NULL OR x.fin >= $3)) \
          ORDER BY l.numero LIMIT 1",
    )
    .bind(etab)
    .bind(d.chambre_id)
    .bind(d.debut)
    .bind(d.lit_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    match lit {
        Some((id, numero)) => {
            a.lit_id = Some(id);
            if a.lit.is_none() {
                a.lit = Some(numero.to_string());
            }
        }
        None => {
            return Err(if d.lit_id.is_some() {
                DomainError::InternatLitIndisponible.into()
            } else {
                DomainError::InternatChambrePleine.into()
            })
        }
    }

    insert_affectation_tx(conn, &a).await?;
    publier(
        conn,
        etab,
        types::INTERNAT_AFFECTATION,
        AggregateKind::Internat,
        a.id,
        Some(a.eleve_id),
        json!({
            "chambre_id": a.chambre_id,
            "lit_id": a.lit_id,
            "debut": a.debut.to_string(),
            "lit": a.lit,
            "motif": a.motif_code,
        }),
        Some(a.created_by_user_id),
        Causalite::default(),
    )
    .await?;
    Ok(a)
}

impl PgEnt {
    /// Journalise une lecture sensible de l'internat (R-IT38, E7) dans le journal en ajout seul.
    pub(crate) async fn journaliser_lecture_internat(
        &self,
        etab: Uuid,
        actor: Uuid,
        eleve: Option<Uuid>,
        action: &str,
        libelle: &str,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle, \
                    visible_famille, actor_user_id) VALUES ($1,$2,$3,'internat',$4,$5,false,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(eleve)
        .bind(action)
        .bind(libelle)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Nuit courante de l'établissement (date du soir, fuseau local : R-IT5).
    pub async fn nuit_courante_internat(&self, etab: Uuid) -> Result<NaiveDate, EntDbError> {
        Ok(self.internat.nuit_courante(etab).await?)
    }

    /// Affecte un interne (une seule transaction : R-IT10).
    pub(crate) async fn affecter_internat(
        &self,
        d: DemandeAffectation,
    ) -> Result<AffectationInternat, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), d.etablissement_id).await.map_err(db)?;
        let a = affecter_tx(&mut tx, &d).await?;
        tx.commit().await.map_err(db)?;
        Ok(a)
    }

    /// INT-116 : affectation à un lit (API d'hébergement).
    pub async fn affecter_lit_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &AffectationSaisieDto,
    ) -> Result<AffectationHebergementDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let a = self
            .affecter_internat(DemandeAffectation {
                etablissement_id: etab,
                eleve_id: b.eleve_id,
                chambre_id: b.chambre_id,
                lit_id: b.lit_id,
                lit_libelle: None,
                debut: b.debut,
                actor_user_id: actor,
                motif_code: b.motif_code.clone(),
                besoin_accessibilite: b.besoin_accessibilite,
            })
            .await?;
        self.affectation_hebergement_dto(etab, &a).await
    }

    /// INT-117 : changement de chambre. L'ancienne affectation prend fin la veille de la date d'effet,
    /// la nouvelle commence à cette date — dans la même transaction (atomique : jamais sans lit).
    pub async fn changer_chambre_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        b: &ChangementChambreDto,
    ) -> Result<AffectationHebergementDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        if !MOTIFS_AFFECTATION.contains(&b.motif_code.as_str()) {
            return Err(invalide("motif de changement de chambre obligatoire (codé)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ancienne = get_affectation_ouverte_tx(&mut tx, etab, eleve)
            .await?
            .ok_or(DomainError::InternatAffectationInvalide)?;
        if b.date_effet <= ancienne.debut {
            return Err(invalide(
                "date d'effet postérieure au début de l'affectation actuelle requise",
            ));
        }
        let mut close = ancienne.clone();
        close.fin = Some(b.date_effet - Duration::days(1));
        update_affectation_fin_tx(&mut tx, &close).await?;
        let a = affecter_tx(
            &mut tx,
            &DemandeAffectation {
                etablissement_id: etab,
                eleve_id: eleve,
                chambre_id: b.chambre_id,
                lit_id: b.lit_id,
                lit_libelle: None,
                debut: b.date_effet,
                actor_user_id: actor,
                motif_code: Some(b.motif_code.clone()),
                besoin_accessibilite: b.besoin_accessibilite,
            },
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_AFFECTATION,
            AggregateKind::Internat,
            close.id,
            Some(eleve),
            json!({ "fin": close.fin.map(|f| f.to_string()), "cloture": true, "motif": b.motif_code }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.affectation_hebergement_dto(etab, &a).await
    }

    async fn affectation_hebergement_dto(
        &self,
        etab: Uuid,
        a: &AffectationInternat,
    ) -> Result<AffectationHebergementDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let numero: Option<i16> = match a.lit_id {
            Some(l) => sqlx::query_scalar(
                "SELECT numero FROM ent_internat_lit WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(l)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?,
            None => None,
        };
        Ok(AffectationHebergementDto {
            id: a.id,
            eleve_id: a.eleve_id,
            chambre_id: a.chambre_id,
            lit_id: a.lit_id,
            lit_numero: numero,
            sejour_id: a.sejour_id,
            debut: a.debut,
            fin: a.fin,
            motif_code: a.motif_code.clone(),
        })
    }

    // ───────────────────────── Chambres ─────────────────────────

    fn chambre_dto(c: &ChambreInternat) -> ChambreEtendueDto {
        ChambreEtendueDto {
            id: c.id,
            batiment: c.batiment.clone(),
            etage: c.etage,
            code: c.code.clone(),
            capacite: c.capacite,
            public: c.public.map(|p| p.as_str().to_string()),
            zone_id: c.zone_id,
            accessible_pmr: c.accessible_pmr,
            douche: c.douche,
            lavabo: c.lavabo,
            bureau: c.bureau,
            etat: c.etat.as_str().to_string(),
            ferme_du: c.ferme_du,
            ferme_au: c.ferme_au,
            notes_entretien: c.notes_entretien.clone(),
            actif: c.actif,
        }
    }

    /// Cohérence chambre / zone : pas de chambre en partie commune, public identique (R-IT11).
    async fn verifier_zone_chambre(
        conn: &mut PgConnection,
        etab: Uuid,
        zone_id: Uuid,
        public: Option<PublicChambre>,
    ) -> Result<(), EntDbError> {
        let zone: Option<String> = sqlx::query_scalar(
            "SELECT public FROM ent_internat_zone WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(zone_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let zone = zone.ok_or(DomainError::InternatZoneInvalide)?;
        if zone == "commun" {
            return Err(DomainError::InternatZoneInvalide.into());
        }
        if public.map(|p| p.as_str()) != Some(zone.as_str()) {
            return Err(DomainError::InternatPublicIncompatible.into());
        }
        Ok(())
    }

    /// INT-102 : créer une chambre (zone, public, lits, accessibilité, équipements).
    pub async fn creer_chambre_etendue_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ChambreSaisieDto,
    ) -> Result<ChambreEtendueDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let mut c = plan_creer_chambre_internat(CreerChambreInternat {
            etablissement_id: etab,
            batiment: b.batiment.clone(),
            etage: b.etage,
            code: b.code.clone(),
            capacite: b.capacite,
            genre: b.public.clone(),
            actor_user_id: actor,
        })?;
        c.zone_id = b.zone_id;
        c.accessible_pmr = b.accessible_pmr;
        c.douche = b.douche;
        c.lavabo = b.lavabo;
        c.bureau = b.bureau;
        c.notes_entretien = b
            .notes_entretien
            .as_ref()
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty());
        if c.notes_entretien.as_ref().is_some_and(|n| n.chars().count() > 300) {
            return Err(DomainError::InternatChambreInvalide.into());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if let Some(z) = c.zone_id {
            Self::verifier_zone_chambre(&mut tx, etab, z, c.public).await?;
        }
        insert_chambre_tx(&mut tx, &c).await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_CHAMBRE,
            AggregateKind::Internat,
            c.id,
            None,
            json!({ "code": c.code, "capacite": c.capacite, "public": c.public.map(|p| p.as_str()), "etat": "creee" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(Self::chambre_dto(&c))
    }

    /// INT-104 / INT-105 : modifier, fermer ou retirer une chambre. Refus sous l'occupation (RI-07).
    pub async fn modifier_chambre_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        chambre_id: Uuid,
        b: &ChambreModifDto,
    ) -> Result<ChambreEtendueDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let mut cmd = ModifierChambreInternat {
            code: b.code.clone(),
            capacite: b.capacite,
            accessible_pmr: b.accessible_pmr,
            douche: b.douche,
            lavabo: b.lavabo,
            bureau: b.bureau,
            ..Default::default()
        };
        if let Some(p) = &b.public {
            cmd.public = Some(PublicChambre::parse(p).ok_or(DomainError::InternatChambreInvalide)?);
        }
        if b.retirer_zone {
            cmd.zone_id = Some(None);
        } else if let Some(z) = b.zone_id {
            cmd.zone_id = Some(Some(z));
        }
        if let Some(e) = &b.etat {
            cmd.etat = Some(EtatChambre::parse(e).ok_or(DomainError::InternatChambreInvalide)?);
        }
        if b.effacer_fermeture {
            cmd.ferme_du = Some(None);
            cmd.ferme_au = Some(None);
        } else {
            if b.ferme_du.is_some() {
                cmd.ferme_du = Some(b.ferme_du);
            }
            if b.ferme_au.is_some() {
                cmd.ferme_au = Some(b.ferme_au);
            }
        }
        if let Some(n) = &b.notes_entretien {
            cmd.notes_entretien = Some(Some(n.clone()));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let actuelle = get_chambre_tx(&mut tx, etab, chambre_id, true)
            .await?
            .ok_or(DomainError::NotFound)?;
        let aujourdhui = aujourdhui_tx(&mut tx, etab).await?;
        let periodes = periodes_chambre_tx(&mut tx, etab, chambre_id, aujourdhui).await?;
        let pic = pic_occupation(&periodes, aujourdhui);
        let nouvelle = plan_modifier_chambre_internat(&actuelle, &cmd, pic, !periodes.is_empty())?;
        if let Some(z) = nouvelle.zone_id {
            Self::verifier_zone_chambre(&mut tx, etab, z, nouvelle.public).await?;
        }
        update_chambre_tx(&mut tx, &nouvelle).await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_CHAMBRE,
            AggregateKind::Internat,
            nouvelle.id,
            None,
            json!({ "code": nouvelle.code, "capacite": nouvelle.capacite, "etat": nouvelle.etat.as_str(), "modifiee": true }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(Self::chambre_dto(&nouvelle))
    }

    pub async fn lister_chambres_etendues_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<ChambreEtendueDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let l = self.internat.list_chambres(etab, false).await?;
        Ok(l.iter().map(Self::chambre_dto).collect())
    }

    // ───────────────────────── Lits ─────────────────────────

    pub async fn lister_lits_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        chambre_id: Uuid,
    ) -> Result<Vec<LitDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(
            "SELECT id, chambre_id, numero, neutralise_du, neutralise_au, motif_code \
               FROM ent_internat_lit WHERE etablissement_id = $1 AND chambre_id = $2 ORDER BY numero",
        )
        .bind(etab)
        .bind(chambre_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .iter()
            .map(|r| LitDto {
                id: r.get("id"),
                chambre_id: r.get("chambre_id"),
                numero: r.get("numero"),
                neutralise_du: r.get("neutralise_du"),
                neutralise_au: r.get("neutralise_au"),
                motif_code: r.get("motif_code"),
            })
            .collect())
    }

    /// INT-106 : neutraliser un lit (cassé, accueil d'urgence) ; refusé s'il est occupé pendant la période.
    pub async fn neutraliser_lit_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        lit_id: Uuid,
        b: &NeutraliserLitDto,
    ) -> Result<(), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        if b.au.is_some_and(|a| a < b.du) {
            return Err(invalide("fin de neutralisation antérieure au début"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let occupe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_internat_affectation \
              WHERE etablissement_id = $1 AND lit_id = $2 AND (fin IS NULL OR fin >= $3) \
                AND ($4::date IS NULL OR debut <= $4))",
        )
        .bind(etab)
        .bind(lit_id)
        .bind(b.du)
        .bind(b.au)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if occupe {
            return Err(DomainError::InternatLitIndisponible.into());
        }
        let n = sqlx::query(
            "UPDATE ent_internat_lit SET neutralise_du = $3, neutralise_au = $4, motif_code = $5, \
                    modifie_le = now() WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(lit_id)
        .bind(b.du)
        .bind(b.au)
        .bind(&b.motif_code)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound.into());
        }
        publier(
            &mut tx,
            etab,
            types::INTERNAT_LIT,
            AggregateKind::Internat,
            lit_id,
            None,
            json!({ "neutralise": true, "du": b.du.to_string(), "motif": b.motif_code }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    pub async fn retablir_lit_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        lit_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE ent_internat_lit SET neutralise_du = NULL, neutralise_au = NULL, motif_code = NULL, \
                    modifie_le = now() WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(lit_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound.into());
        }
        publier(
            &mut tx,
            etab,
            types::INTERNAT_LIT,
            AggregateKind::Internat,
            lit_id,
            None,
            json!({ "neutralise": false }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────── Zones ─────────────────────────

    fn zone_depuis_ligne(r: &sqlx::postgres::PgRow) -> ZoneDto {
        ZoneDto {
            id: r.get("id"),
            batiment_id: r.get("batiment_id"),
            code: r.get("code"),
            libelle: r.get("libelle"),
            public: r.get("public"),
            ordre: r.get("ordre"),
            ferme_du: r.get("ferme_du"),
            ferme_au: r.get("ferme_au"),
        }
    }

    /// INT-100 : déclarer une zone d'un bâtiment du socle 5.
    pub async fn creer_zone_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ZoneSaisieDto,
    ) -> Result<ZoneDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        if !["filles", "garcons", "commun"].contains(&b.public.as_str()) {
            return Err(DomainError::InternatZoneInvalide.into());
        }
        let (code, libelle) = (b.code.trim(), b.libelle.trim());
        if code.is_empty() || libelle.is_empty() {
            return Err(DomainError::InternatZoneInvalide.into());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let id = Uuid::new_v4();
        let r = sqlx::query(
            "INSERT INTO ent_internat_zone (id, etablissement_id, batiment_id, code, libelle, public, \
                    ordre, cree_par_user_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
             RETURNING id, batiment_id, code, libelle, public, ordre, ferme_du, ferme_au",
        )
        .bind(id)
        .bind(etab)
        .bind(b.batiment_id)
        .bind(code)
        .bind(libelle)
        .bind(&b.public)
        .bind(b.ordre.unwrap_or(0))
        .bind(actor)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_ZONE,
            AggregateKind::Internat,
            id,
            None,
            json!({ "code": code, "public": b.public, "etat": "creee" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let z = Self::zone_depuis_ligne(&r);
        tx.commit().await.map_err(db)?;
        Ok(z)
    }

    /// INT-101 : modifier, fermer ou rouvrir une zone ; les occupants à reloger sont signalés.
    pub async fn modifier_zone_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        zone_id: Uuid,
        b: &ZoneModifDto,
    ) -> Result<ZoneModifieeDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        if b.public
            .as_deref()
            .is_some_and(|p| !["filles", "garcons", "commun"].contains(&p))
        {
            return Err(DomainError::InternatZoneInvalide.into());
        }
        if b.ferme_au.is_some() && b.ferme_du.is_none() {
            return Err(DomainError::InternatZoneInvalide.into());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let actuelle = sqlx::query(
            "SELECT id, batiment_id, code, libelle, public, ordre, ferme_du, ferme_au \
               FROM ent_internat_zone WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(zone_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(DomainError::NotFound)?;
        let z0 = Self::zone_depuis_ligne(&actuelle);
        let (du, au) = if b.rouvrir {
            (None, None)
        } else if b.ferme_du.is_some() {
            (b.ferme_du, b.ferme_au)
        } else {
            (z0.ferme_du, z0.ferme_au)
        };
        let public = b.public.clone().unwrap_or_else(|| z0.public.clone());
        // Changer le public d'une zone qui contient déjà des chambres d'un autre public est refusé (R-IT11).
        if public != z0.public {
            let incoherentes: i64 = sqlx::query_scalar(
                "SELECT COUNT(*)::bigint FROM ent_internat_chambre \
                  WHERE etablissement_id = $1 AND zone_id = $2 AND (public IS DISTINCT FROM $3 OR $3 = 'commun')",
            )
            .bind(etab)
            .bind(zone_id)
            .bind(&public)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if incoherentes > 0 {
                return Err(DomainError::InternatPublicIncompatible.into());
            }
        }
        let r = sqlx::query(
            "UPDATE ent_internat_zone SET libelle = COALESCE($3, libelle), public = $4, \
                    ordre = COALESCE($5, ordre), ferme_du = $6, ferme_au = $7, modifie_le = now() \
              WHERE etablissement_id = $1 AND id = $2 \
              RETURNING id, batiment_id, code, libelle, public, ordre, ferme_du, ferme_au",
        )
        .bind(etab)
        .bind(zone_id)
        .bind(b.libelle.as_ref().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()))
        .bind(&public)
        .bind(b.ordre)
        .bind(du)
        .bind(au)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let zone = Self::zone_depuis_ligne(&r);
        let occupants: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM ent_internat_affectation a \
               JOIN ent_internat_chambre c ON c.etablissement_id = a.etablissement_id AND c.id = a.chambre_id \
              WHERE a.etablissement_id = $1 AND c.zone_id = $2 AND a.fin IS NULL",
        )
        .bind(etab)
        .bind(zone_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_ZONE,
            AggregateKind::Internat,
            zone_id,
            None,
            json!({ "code": zone.code, "public": zone.public, "ferme": zone.ferme_du.is_some() }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ZoneModifieeDto {
            zone,
            occupants_a_reloger: if du.is_some() { occupants } else { 0 },
        })
    }

    pub async fn lister_zones_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<ZoneDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(
            "SELECT id, batiment_id, code, libelle, public, ordre, ferme_du, ferme_au \
               FROM ent_internat_zone WHERE etablissement_id = $1 ORDER BY batiment_id, ordre, code",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows.iter().map(Self::zone_depuis_ligne).collect())
    }

    // ───────────────────────── Séjours et rythmes ─────────────────────────

    /// INT-110 : (re)synchronise les séjours d'un élève sur son régime daté. Explicite : la lecture
    /// d'un séjour n'écrit jamais (DI-12 : pas de déduplicateur qui écrit à chaque lecture).
    pub async fn synchroniser_sejours_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<Vec<SejourDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(DomainError::NotFound.into());
        }
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        synchroniser_sejours_tx(&mut tx, etab, eleve, jour, Some(actor)).await?;
        let l = sejours_tx(&mut tx, etab, eleve).await?;
        tx.commit().await.map_err(db)?;
        Ok(l)
    }

    pub async fn lister_sejours_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<Vec<SejourDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sejours_tx(&mut tx, etab, eleve).await
    }

    /// INT-111 : nouvelle version du rythme d'un séjour (R-IT4). Une version ne se modifie pas.
    pub async fn definir_rythme_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        sejour_id: Uuid,
        b: &RythmeSaisieDto,
    ) -> Result<SejourDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let r = dto_vers_rythme(&b.jours);
        if !r.au_moins_un_soir() {
            return Err(DomainError::InternatRythmeInvalide.into());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s: Option<(Uuid, NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT eleve_id, du, au FROM ent_internat_sejour WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(sejour_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (eleve, du, au) = s.ok_or(DomainError::NotFound)?;
        if b.du < du || au.is_some_and(|a| b.du > a) {
            return Err(DomainError::InternatRythmeInvalide.into());
        }
        // Même date que la version existante : refusé (ajout seul, une version par date).
        inserer_rythme_tx(&mut tx, etab, sejour_id, b.du, &r, Some(actor))
            .await
            .map_err(|e| match e {
                EntDbError::Domain(DomainError::Conflit) => {
                    EntDbError::Domain(DomainError::InternatRythmeInvalide)
                }
                autre => autre,
            })?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_RYTHME,
            AggregateKind::Internat,
            sejour_id,
            Some(eleve),
            json!({ "du": b.du.to_string() }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let l = sejours_tx(&mut tx, etab, eleve).await?;
        tx.commit().await.map_err(db)?;
        l.into_iter()
            .find(|s| s.id == sejour_id)
            .ok_or_else(|| DomainError::NotFound.into())
    }

    // ───────────────────────── Dérogations et interdictions ─────────────────────────

    /// R-IT11 / R-IT12 : dérogation de la direction (jamais de motif en clair).
    pub async fn accorder_derogation_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &DerogationSaisieDto,
    ) -> Result<DerogationDto, EntDbError> {
        if !est_direction(role) {
            return Err(EntDbError::Forbidden);
        }
        match b.nature.as_str() {
            "public_chambre" if b.chambre_id.is_some() && b.avec_eleve_id.is_none() => {}
            "interdiction_contact"
                if b.avec_eleve_id.is_some_and(|a| a != b.eleve_id) && b.chambre_id.is_none() => {}
            _ => return Err(invalide("dérogation : nature et cible incohérentes")),
        }
        if b.au.is_some_and(|a| a < b.du) {
            return Err(invalide("fin de dérogation antérieure au début"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let id = Uuid::new_v4();
        let cree_le: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
            "INSERT INTO ent_internat_derogation (id, etablissement_id, eleve_id, nature, chambre_id, \
                    avec_eleve_id, du, au, decide_par_user_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) \
             RETURNING cree_le",
        )
        .bind(id)
        .bind(etab)
        .bind(b.eleve_id)
        .bind(&b.nature)
        .bind(b.chambre_id)
        .bind(b.avec_eleve_id)
        .bind(b.du)
        .bind(b.au)
        .bind(actor)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_DEROGATION,
            AggregateKind::Internat,
            id,
            Some(b.eleve_id),
            json!({ "nature": b.nature, "du": b.du.to_string() }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(DerogationDto {
            id,
            eleve_id: b.eleve_id,
            nature: b.nature.clone(),
            chambre_id: b.chambre_id,
            avec_eleve_id: b.avec_eleve_id,
            du: b.du,
            au: b.au,
            decide_par_user_id: actor,
            cree_le,
        })
    }

    /// Port provisoire de la mesure du socle 13 : paire d'élèves sous interdiction de contact.
    pub async fn poser_interdiction_contact_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &InterdictionContactSaisieDto,
    ) -> Result<InterdictionContactDto, EntDbError> {
        if !est_direction(role) {
            return Err(EntDbError::Forbidden);
        }
        if b.eleve_a_id == b.eleve_b_id || b.au.is_some_and(|a| a < b.du) {
            return Err(invalide("interdiction de contact : paire ou dates invalides"));
        }
        let (a, c) = if b.eleve_a_id < b.eleve_b_id {
            (b.eleve_a_id, b.eleve_b_id)
        } else {
            (b.eleve_b_id, b.eleve_a_id)
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_interdiction_contact (id, etablissement_id, eleve_a_id, eleve_b_id, \
                    du, au, cree_par_user_id) VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(etab)
        .bind(a)
        .bind(c)
        .bind(b.du)
        .bind(b.au)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(InterdictionContactDto { id, eleve_a_id: a, eleve_b_id: c, du: b.du, au: b.au })
    }

    // ───────────────────────── Plan (INT-107) ─────────────────────────

    /// Plan des lits par zone pour un jour : occupation, neutralisations, surcapacités — **sans nom**.
    pub async fn plan_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<PlanDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let zones: Vec<ZoneDto> = sqlx::query(
            "SELECT id, batiment_id, code, libelle, public, ordre, ferme_du, ferme_au \
               FROM ent_internat_zone WHERE etablissement_id = $1 ORDER BY batiment_id, ordre, code",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .iter()
        .map(Self::zone_depuis_ligne)
        .collect();
        let chambres = self.internat_chambres_tx(&mut tx, etab).await?;
        let lits = sqlx::query(
            "SELECT id, chambre_id, numero, neutralise_du, neutralise_au FROM ent_internat_lit \
              WHERE etablissement_id = $1 ORDER BY chambre_id, numero",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let occupations: Vec<(Uuid, Option<Uuid>, Uuid)> = sqlx::query_as(
            "SELECT chambre_id, lit_id, eleve_id FROM ent_internat_affectation \
              WHERE etablissement_id = $1 AND debut <= $2 AND (fin IS NULL OR fin >= $2)",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let (mut libres, mut occupes, mut neutralises) = (0i64, 0i64, 0i64);
        let mut construire = |c: &ChambreInternat| -> PlanChambreDto {
            let mut plan_lits = Vec::new();
            for l in lits.iter().filter(|l| l.get::<Uuid, _>("chambre_id") == c.id) {
                let id: Uuid = l.get("id");
                let du: Option<NaiveDate> = l.get("neutralise_du");
                let au: Option<NaiveDate> = l.get("neutralise_au");
                let neutre = du.is_some_and(|d| d <= jour && au.map(|a| jour <= a).unwrap_or(true));
                let occupant = occupations
                    .iter()
                    .find(|(ch, lit, _)| *ch == c.id && *lit == Some(id))
                    .map(|(_, _, e)| *e);
                let etat = if neutre {
                    neutralises += 1;
                    "neutralise"
                } else if occupant.is_some() {
                    occupes += 1;
                    "occupe"
                } else {
                    libres += 1;
                    "libre"
                };
                plan_lits.push(PlanLitDto {
                    lit_id: id,
                    numero: l.get("numero"),
                    etat: etat.into(),
                    eleve_id: occupant,
                });
            }
            let sans_lit: Vec<Uuid> = occupations
                .iter()
                .filter(|(ch, lit, _)| *ch == c.id && lit.is_none())
                .map(|(_, _, e)| *e)
                .collect();
            let nb_occupants = occupations.iter().filter(|(ch, _, _)| *ch == c.id).count();
            PlanChambreDto {
                id: c.id,
                batiment: c.batiment.clone(),
                etage: c.etage,
                code: c.code.clone(),
                public: c.public.map(|p| p.as_str().to_string()),
                accessible_pmr: c.accessible_pmr,
                etat: c.etat.as_str().to_string(),
                capacite: c.capacite,
                surcapacite: nb_occupants as i64 > i64::from(c.capacite),
                lits: plan_lits,
                occupants_sans_lit: sans_lit,
            }
        };
        let mut zones_plan: Vec<PlanZoneDto> = Vec::new();
        for z in zones {
            let chs: Vec<PlanChambreDto> = chambres
                .iter()
                .filter(|c| c.zone_id == Some(z.id) && c.etat != EtatChambre::Retiree)
                .map(&mut construire)
                .collect();
            zones_plan.push(PlanZoneDto { zone: z, chambres: chs });
        }
        let sans_zone: Vec<PlanChambreDto> = chambres
            .iter()
            .filter(|c| c.zone_id.is_none() && c.etat != EtatChambre::Retiree)
            .map(&mut construire)
            .collect();
        Ok(PlanDto {
            jour,
            zones: zones_plan,
            chambres_sans_zone: sans_zone,
            lits_libres: libres,
            lits_occupes: occupes,
            lits_neutralises: neutralises,
        })
    }

    async fn internat_chambres_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
    ) -> Result<Vec<ChambreInternat>, EntDbError> {
        // Réutilise la lecture du dépôt dans la même transaction (RLS posée par l'appelant).
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_internat_chambre WHERE etablissement_id = $1 ORDER BY batiment, etage, code",
        )
        .bind(etab)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for id in ids {
            if let Some(c) = get_chambre_tx(conn, etab, id, false).await? {
                out.push(c);
            }
        }
        Ok(out)
    }

    // ───────────────────────── Paramètres ─────────────────────────

    pub async fn parametres_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<ParametresInternatDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        parametres_tx(&mut tx, etab).await
    }

    /// Modification des paramètres : direction (administrateur compris).
    pub async fn modifier_parametres_internat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        m: &ParametresInternatModifDto,
    ) -> Result<ParametresInternatDto, EntDbError> {
        if !est_direction(role) {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut p = parametres_tx(&mut tx, etab).await?;
        if let Some(v) = m.heure_limite_annonce {
            p.heure_limite_annonce = v;
        }
        if let Some(v) = m.heure_limite_appel {
            p.heure_limite_appel = v;
        }
        if let Some(v) = m.seuil_retard_minutes {
            if !(0..=720).contains(&v) {
                return Err(invalide("seuil de retard hors limites (0 à 720 minutes)"));
            }
            p.seuil_retard_minutes = v;
        }
        if let Some(v) = m.heure_recuperation_vendredi {
            p.heure_recuperation_vendredi = v;
        }
        if let Some(v) = m.jour_bilan {
            if !(1..=7).contains(&v) {
                return Err(invalide("jour du bilan : 1 (lundi) à 7 (dimanche)"));
            }
            p.jour_bilan = v;
        }
        if let Some(v) = m.voisins_visibles_familles {
            p.voisins_visibles_familles = v;
        }
        if let Some(v) = m.depot_telephones {
            p.depot_telephones = v;
        }
        if let Some(v) = m.garde_honorabilite {
            p.garde_honorabilite = v;
        }
        if let Some(v) = m.impression_presents {
            p.impression_presents = v;
        }
        if let Some(v) = &m.majeur_info_parents {
            if !["toujours", "si_a_charge", "jamais"].contains(&v.as_str()) {
                return Err(invalide("information des parents d'un majeur : valeur inconnue"));
            }
            p.majeur_info_parents = v.clone();
        }
        if let Some(v) = &m.rythme_defaut {
            if !dto_vers_rythme(v).au_moins_un_soir() {
                return Err(DomainError::InternatRythmeInvalide.into());
            }
            p.rythme_defaut = *v;
        }
        let r = &p.rythme_defaut;
        sqlx::query(
            "INSERT INTO ent_internat_parametres (etablissement_id, heure_limite_annonce, heure_limite_appel, \
                    seuil_retard_minutes, heure_recuperation_vendredi, jour_bilan, voisins_visibles_familles, \
                    depot_telephones, garde_honorabilite, impression_presents, majeur_info_parents, \
                    rythme_dimanche, rythme_lundi, rythme_mardi, rythme_mercredi, rythme_jeudi, \
                    rythme_vendredi, rythme_samedi, modifie_par) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19) \
             ON CONFLICT (etablissement_id) DO UPDATE SET \
                    heure_limite_annonce = EXCLUDED.heure_limite_annonce, \
                    heure_limite_appel = EXCLUDED.heure_limite_appel, \
                    seuil_retard_minutes = EXCLUDED.seuil_retard_minutes, \
                    heure_recuperation_vendredi = EXCLUDED.heure_recuperation_vendredi, \
                    jour_bilan = EXCLUDED.jour_bilan, \
                    voisins_visibles_familles = EXCLUDED.voisins_visibles_familles, \
                    depot_telephones = EXCLUDED.depot_telephones, \
                    garde_honorabilite = EXCLUDED.garde_honorabilite, \
                    impression_presents = EXCLUDED.impression_presents, \
                    majeur_info_parents = EXCLUDED.majeur_info_parents, \
                    rythme_dimanche = EXCLUDED.rythme_dimanche, rythme_lundi = EXCLUDED.rythme_lundi, \
                    rythme_mardi = EXCLUDED.rythme_mardi, rythme_mercredi = EXCLUDED.rythme_mercredi, \
                    rythme_jeudi = EXCLUDED.rythme_jeudi, rythme_vendredi = EXCLUDED.rythme_vendredi, \
                    rythme_samedi = EXCLUDED.rythme_samedi, modifie_par = EXCLUDED.modifie_par, \
                    modifie_le = now()",
        )
        .bind(etab)
        .bind(p.heure_limite_annonce)
        .bind(p.heure_limite_appel)
        .bind(p.seuil_retard_minutes)
        .bind(p.heure_recuperation_vendredi)
        .bind(p.jour_bilan)
        .bind(p.voisins_visibles_familles)
        .bind(p.depot_telephones)
        .bind(p.garde_honorabilite)
        .bind(p.impression_presents)
        .bind(&p.majeur_info_parents)
        .bind(r.dimanche)
        .bind(r.lundi)
        .bind(r.mardi)
        .bind(r.mercredi)
        .bind(r.jeudi)
        .bind(r.vendredi)
        .bind(r.samedi)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_PARAMETRES,
            AggregateKind::Internat,
            etab,
            None,
            json!({ "modifie": true }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(p)
    }
}

async fn sejours_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
) -> Result<Vec<SejourDto>, EntDbError> {
    let rows = sqlx::query(
        "SELECT id, eleve_id, du, au, origine, ferme_motif_code FROM ent_internat_sejour \
          WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY du",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut out = Vec::new();
    for r in rows {
        let id: Uuid = r.get("id");
        let rythmes = sqlx::query(
            "SELECT du, dimanche, lundi, mardi, mercredi, jeudi, vendredi, samedi \
               FROM ent_internat_rythme WHERE etablissement_id = $1 AND sejour_id = $2 ORDER BY du",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
        .iter()
        .map(|x| RythmeDto {
            du: x.get("du"),
            jours: JoursRythmeDto {
                dimanche: x.get("dimanche"),
                lundi: x.get("lundi"),
                mardi: x.get("mardi"),
                mercredi: x.get("mercredi"),
                jeudi: x.get("jeudi"),
                vendredi: x.get("vendredi"),
                samedi: x.get("samedi"),
            },
        })
        .collect();
        out.push(SejourDto {
            id,
            eleve_id: r.get("eleve_id"),
            du: r.get("du"),
            au: r.get("au"),
            origine: r.get("origine"),
            ferme_motif_code: r.get("ferme_motif_code"),
            rythmes,
        });
    }
    Ok(out)
}
