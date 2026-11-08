//! Socle 3, lot I2 — modèle des campagnes, de l'offre et des places, des démarches (fratrie
//! native), des déclarations de responsables, et règles des personas (« ne pas solliciter »,
//! annulation d'une validation erronée, prolongation d'échéance, fermetures du secrétariat).
//! Conception : `docs/socles/03_INSCRIPTIONS.md` §6 à §8, §11 (INS-18, 25, 39, 57, 68, 79, 80),
//! §17 bis (R-I6, R-I8, R-I11, R-I15).
//!
//! * **Les états et leurs transitions** sont dans [`crate::inscriptions_etats`] (une seule table) ;
//!   l'écriture de l'état passe par `inscriptions::transitionner_tx`.
//! * **Les écritures de personnes, de foyers et d'élèves** ne se font jamais ici : l'annulation
//!   d'une validation appelle les commandes inverses des socles 2 et 4 (`eleve_ports`,
//!   `dossier_personnes_ports`), et la facturation retire ses propres lignes brouillon.
//! * **Les envois** aux familles passent par la file `ent_inscription_envoi` : ils partent après
//!   la validation de la transaction, jamais vers une personne marquée « à ne pas solliciter ».

use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::{types, AggregateKind};
use ent_types::inscriptions::*;
use serde_json::{json, Value};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::champs_chiffres::ChampsSensibles;
use crate::inscriptions::{
    self, bornee, conflit, db, introuvable, invalide, opt, publier, suivi_tx, transitionner_tx,
    CampagneRow, DossierRow, COLS_CAMPAGNE,
};
use crate::inscriptions_etats::{self, Cause};
use crate::service::{EntDbError, PgEnt};
use crate::{dossier_personnes, eleve_ports, finances};

pub const REGIMES: [&str; 3] = ["externe", "demi_pension", "interne"];
/// Jours de fermeture au plus d'une même déclaration (garde-fou contre une erreur de saisie).
pub const FERMETURE_JOURS_MAX: i64 = 120;
/// Responsables au plus d'une démarche (rangs 1 à 6).
pub const RESPONSABLES_MAX: usize = 6;
/// Enfants au plus d'une démarche.
pub const ENFANTS_MAX: usize = 6;
/// Types d'événements produits par la validation elle-même (ou par ses réactions) : ils ne
/// comptent pas comme des « données produites depuis » par un autre socle (INS-68).
const EVENEMENTS_DE_VALIDATION: [&str; 3] = [
    types::ELEVE_CREATED,
    types::ELEVE_LIEN_UPDATED,
    types::ELEVE_COMPTE_INVITE,
];

// ---------------------------------------------------------------------------
// Fermetures du secrétariat (R-I6) et échéances
// ---------------------------------------------------------------------------

/// Une échéance qui tombe pendant une fermeture est reportée au jour de réouverture + N jours.
pub(crate) async fn echeance_effective_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
) -> Result<NaiveDate, EntDbError> {
    let mut j = jour;
    // Fermetures qui s'enchaînent (juillet puis août) : quelques tours suffisent.
    for _ in 0..8 {
        let f: Option<(NaiveDate, i16)> = sqlx::query_as(
            "SELECT date_fin, report_jours FROM ent_inscription_fermeture
              WHERE etablissement_id = $1 AND annulee_le IS NULL AND $2 BETWEEN date_debut AND date_fin
              ORDER BY date_fin DESC LIMIT 1",
        )
        .bind(etab)
        .bind(j)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        match f {
            Some((fin, report)) => j = fin + Duration::days(1 + i64::from(report)),
            None => break,
        }
    }
    Ok(j)
}

/// Fermeture en cours aujourd'hui (jour métier de l'établissement), pour le message de la famille.
pub(crate) async fn fermeture_en_cours(
    pool: &PgPool,
    etab: Uuid,
) -> Result<Option<FermetureDto>, EntDbError> {
    let r: Option<(Uuid, NaiveDate, NaiveDate, i16, String, Option<String>, i32)> = sqlx::query_as(
        "SELECT id, date_debut, date_fin, report_jours, motif_public, contact_urgence, echeances_reportees
           FROM ent_inscription_fermeture
          WHERE etablissement_id = $1 AND annulee_le IS NULL
            AND ent_aujourdhui_etab($1) BETWEEN date_debut AND date_fin
          ORDER BY date_fin DESC LIMIT 1",
    )
    .bind(etab)
    .fetch_optional(pool)
    .await
    .map_err(db)?;
    Ok(r.map(|r| FermetureDto {
        id: r.0,
        date_debut: r.1,
        date_fin: r.2,
        report_jours: r.3,
        motif_public: r.4,
        contact_urgence: r.5,
        echeances_reportees: i64::from(r.6),
    }))
}

// ---------------------------------------------------------------------------
// Année, versions de campagne
// ---------------------------------------------------------------------------

/// Année du référentiel pour un libellé `2026-2027`. Une année absente est assurée par la
/// commande du référentiel (`ent_assurer_annee`, 1ᵉʳ septembre – 31 août) : le socle 3 n'écrit
/// pas lui-même dans `ent_annee_scolaire`.
pub(crate) async fn resoudre_annee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    libelle: &str,
) -> Result<Option<Uuid>, EntDbError> {
    let existe: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND libelle = $2",
    )
    .bind(etab)
    .bind(libelle)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if existe.is_some() {
        return Ok(existe);
    }
    let debut: i32 = libelle.get(..4).and_then(|a| a.parse().ok()).unwrap_or(0);
    if !(2000..=2999).contains(&debut) {
        return Ok(None);
    }
    let id: Option<Uuid> = sqlx::query_scalar("SELECT ent_assurer_annee($1, make_date($2, 9, 15))")
        .bind(etab)
        .bind(debut)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    // L'année assurée doit être celle demandée (l'établissement peut avoir des dates propres).
    let ok: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2 AND libelle = $3",
    )
    .bind(etab)
    .bind(id)
    .bind(libelle)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(ok)
}

/// Écrit la version courante d'une campagne dans le journal (instantané des règles et des offres).
pub(crate) async fn ecrire_version_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    campagne_id: Uuid,
    motif: Option<&str>,
    auteur: Uuid,
) -> Result<i32, EntDbError> {
    let c = sqlx::query_as::<_, CampagneRow>(&format!(
        "SELECT {COLS_CAMPAGNE} FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2"
    ))
    .bind(etab)
    .bind(campagne_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let offres: Vec<(Uuid, String, String, Option<String>, Option<i32>)> = sqlx::query_as(
        "SELECT id, niveau_code, parcours_code, regime, capacite FROM ent_inscription_offre
          WHERE etablissement_id = $1 AND campagne_id = $2 AND archivee_le IS NULL ORDER BY niveau_code, parcours_code",
    )
    .bind(etab)
    .bind(campagne_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let instantane = json!({
        "libelle": c.libelle, "nature": c.nature, "annee_cible": c.annee_cible, "statut": c.statut,
        "date_ouverture": c.date_ouverture, "date_cloture": c.date_cloture,
        "date_cloture_depot": c.date_cloture_depot, "date_limite_famille": c.date_limite_famille,
        "regles": {
            "accord_responsables": c.accord_responsables, "decision_requise": c.decision_requise,
            "motif_communique": c.motif_communique, "mode_reinscription": c.mode_reinscription,
            "delai_reponse_jours": c.delai_reponse_jours,
            "delai_annulation_validation_h": c.delai_annulation_validation_h,
            "page_publique": c.page_publique, "au_fil_de_l_eau": c.au_fil_de_l_eau,
        },
        "offres": offres.iter().map(|o| json!({
            "id": o.0, "niveau": o.1, "parcours": o.2, "regime": o.3, "capacite": o.4,
        })).collect::<Vec<_>>(),
    });
    sqlx::query(
        "INSERT INTO ent_inscription_campagne_version
            (id, etablissement_id, campagne_id, version, instantane, motif, cree_par)
         VALUES ($1,$2,$3,$4,$5,$6,$7)
         ON CONFLICT (campagne_id, version) DO UPDATE SET instantane = EXCLUDED.instantane",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(campagne_id)
    .bind(c.version)
    .bind(instantane)
    .bind(motif)
    .bind(auteur)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(c.version)
}

/// Passe la campagne à la version suivante et l'écrit au journal.
pub(crate) async fn nouvelle_version_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    campagne_id: Uuid,
    motif: Option<&str>,
    auteur: Uuid,
) -> Result<i32, EntDbError> {
    sqlx::query(
        "UPDATE ent_inscription_campagne SET version = version + 1, updated_at = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(campagne_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    ecrire_version_tx(conn, etab, campagne_id, motif, auteur).await
}

/// États d'une campagne et leurs sorties (§8.1, INS-17).
pub fn transition_campagne_admise(de: &str, vers: &str) -> bool {
    matches!(
        (de, vers),
        ("brouillon", "planifiee")
            | ("brouillon", "ouverte")
            | ("planifiee", "brouillon")
            | ("planifiee", "ouverte")
            | ("ouverte", "suspendue")
            | ("ouverte", "fermee")
            | ("suspendue", "ouverte")
            | ("suspendue", "fermee")
            | ("fermee", "archivee")
    )
}

/// Règles d'une campagne : valeurs et bornes du logiciel.
pub fn valider_regles(r: &ReglesCampagneDto, nature: &str) -> Result<(), EntDbError> {
    if !["un_seul", "tous_les_titulaires"].contains(&r.accord_responsables.as_str()) {
        return Err(invalide("accord des responsables : un_seul ou tous_les_titulaires"));
    }
    if !["jamais", "facultatif", "toujours"].contains(&r.motif_communique.as_str()) {
        return Err(invalide("motif communiqué : jamais, facultatif ou toujours"));
    }
    if let Some(m) = &r.mode_reinscription {
        if nature != "reinscription" {
            return Err(invalide("le mode de réinscription ne vaut que pour une réinscription"));
        }
        if !["tacite", "explicite", "explicite_avec_acompte"].contains(&m.as_str()) {
            return Err(invalide("mode de réinscription inconnu"));
        }
    }
    if r.delai_reponse_jours.is_some_and(|d| !(2..=30).contains(&d)) {
        return Err(invalide("délai de réponse : 2 à 30 jours"));
    }
    if !(0..=168).contains(&r.delai_annulation_validation_h) {
        return Err(invalide("délai d'annulation d'une validation : 0 à 168 heures"));
    }
    if r.decision_requise && matches!(nature, "affectes" | "reinscription" | "en_cours_annee") {
        return Err(invalide("cette nature de campagne ne comporte pas de décision d'admission"));
    }
    if r.au_fil_de_l_eau && nature != "admission" {
        return Err(invalide("l'admission au fil de l'eau ne vaut que pour une campagne d'admission"));
    }
    Ok(())
}

/// Règles par défaut d'une nature de campagne.
pub fn regles_par_defaut(nature: &str) -> ReglesCampagneDto {
    ReglesCampagneDto {
        decision_requise: nature == "admission",
        mode_reinscription: (nature == "reinscription").then(|| "explicite".to_string()),
        ..ReglesCampagneDto::default()
    }
}

// ---------------------------------------------------------------------------
// Offre, vœux, places
// ---------------------------------------------------------------------------

/// Ligne du compteur de places (vue `ent_inscription_offre_occupation`).
#[derive(sqlx::FromRow)]
struct OffreRow {
    id: Uuid,
    campagne_id: Uuid,
    niveau_code: String,
    niveau_id: Option<Uuid>,
    parcours_code: String,
    regime: Option<String>,
    libelle_public: String,
    capacite: Option<i32>,
    places_reservees_reinscription: bool,
    ouverte_candidature: bool,
    conditions_acces: Option<String>,
    demandes: i64,
    proposees: i64,
    confirmees: i64,
    validees: i64,
    liste_attente: i64,
    reserve_reinscription: i64,
}

const SELECT_OFFRE: &str = "SELECT o.id, o.campagne_id, o.niveau_code, o.niveau_id, o.parcours_code, o.regime,
        o.libelle_public, o.capacite, o.places_reservees_reinscription, o.ouverte_candidature,
        o.conditions_acces, v.demandes, v.proposees, v.confirmees, v.validees, v.liste_attente,
        v.reserve_reinscription
   FROM ent_inscription_offre o
   JOIN ent_inscription_offre_occupation v
     ON v.offre_id = o.id AND v.etablissement_id = o.etablissement_id";

fn offre_dto(r: OffreRow, unites: Vec<UniteHebergementDto>) -> OffreOccupationDto {
    let disponibles = r.capacite.map(|c| {
        i64::from(c) - r.reserve_reinscription - r.proposees - r.confirmees - r.validees
    });
    OffreOccupationDto {
        id: r.id,
        campagne_id: r.campagne_id,
        niveau_code: r.niveau_code,
        niveau_id: r.niveau_id,
        parcours_code: (!r.parcours_code.is_empty()).then_some(r.parcours_code),
        regime: r.regime,
        libelle_public: r.libelle_public,
        capacite: r.capacite,
        places_reservees_reinscription: r.places_reservees_reinscription,
        ouverte_candidature: r.ouverte_candidature,
        conditions_acces: r.conditions_acces,
        demandes: r.demandes,
        proposees: r.proposees,
        confirmees: r.confirmees,
        validees: r.validees,
        liste_attente: r.liste_attente,
        reserve_reinscription: r.reserve_reinscription,
        disponibles,
        unites,
    }
}

/// Places disponibles d'une offre : `None` = capacité non limitée.
pub(crate) async fn disponibles_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    offre_id: Uuid,
) -> Result<Option<i64>, EntDbError> {
    let r: Option<OffreRow> = sqlx::query_as(&format!(
        "{SELECT_OFFRE} WHERE o.etablissement_id = $1 AND o.id = $2"
    ))
    .bind(etab)
    .bind(offre_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.and_then(|r| offre_dto(r, Vec::new()).disponibles))
}

pub(crate) struct PlaceRetenue {
    pub offre_id: Uuid,
    pub depassement: bool,
}

/// Retient le vœu d'une décision (admis ou liste d'attente) : trouve l'offre, vérifie la capacité
/// sous verrou (un dépassement n'est jamais silencieux : il se confirme et se trace), crée ou
/// retient le vœu, fixe le rang d'attente. `None` : la campagne n'a pas d'offre pour ce dossier
/// (aucune capacité à tenir).
pub(crate) async fn retenir_voeu_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    offre_voulue: Option<Uuid>,
    admis: bool,
    confirmer_depassement: bool,
) -> Result<Option<PlaceRetenue>, EntDbError> {
    let offre: Option<Uuid> = match offre_voulue {
        Some(o) => {
            let ok: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM ent_inscription_offre
                  WHERE etablissement_id = $1 AND id = $2 AND campagne_id = $3 AND archivee_le IS NULL",
            )
            .bind(etab)
            .bind(o)
            .bind(d.campagne_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
            Some(ok.ok_or_else(|| invalide("offre inconnue dans cette campagne"))?)
        }
        None => {
            let premier: Option<Uuid> = sqlx::query_scalar(
                "SELECT v.offre_id FROM ent_inscription_voeu v
                   JOIN ent_inscription_offre o ON o.etablissement_id = v.etablissement_id AND o.id = v.offre_id
                  WHERE v.etablissement_id = $1 AND v.dossier_id = $2 AND v.statut <> 'retire'
                    AND o.archivee_le IS NULL
                  ORDER BY v.rang LIMIT 1",
            )
            .bind(etab)
            .bind(d.id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
            match (premier, &d.niveau_demande) {
                (Some(o), _) => Some(o),
                (None, Some(niveau)) => sqlx::query_scalar(
                    "SELECT id FROM ent_inscription_offre
                      WHERE etablissement_id = $1 AND campagne_id = $2 AND upper(niveau_code) = upper(btrim($3))
                        AND parcours_code = '' AND regime IS NULL AND archivee_le IS NULL",
                )
                .bind(etab)
                .bind(d.campagne_id)
                .bind(niveau)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?,
                (None, None) => None,
            }
        }
    };
    let Some(offre) = offre else {
        return Ok(None);
    };
    // Verrou : deux admissions concurrentes sur la même offre ne dépassent pas la capacité.
    sqlx::query("SELECT id FROM ent_inscription_offre WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
        .bind(etab)
        .bind(offre)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    let mut depassement = false;
    if admis {
        if let Some(dispo) = disponibles_tx(conn, etab, offre).await? {
            if dispo < 1 {
                if !confirmer_depassement {
                    return Err(conflit(
                        "capacité de l'offre atteinte : liste d'attente possible, ou dépassement à confirmer",
                    ));
                }
                depassement = true;
            }
        }
    }
    let voeu: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_inscription_voeu WHERE etablissement_id = $1 AND dossier_id = $2 AND offre_id = $3",
    )
    .bind(etab)
    .bind(d.id)
    .bind(offre)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let voeu = match voeu {
        Some(v) => v,
        None => {
            let rang: i16 = sqlx::query_scalar(
                "SELECT COALESCE(max(rang), 0)::smallint + 1 FROM ent_inscription_voeu
                  WHERE etablissement_id = $1 AND dossier_id = $2",
            )
            .bind(etab)
            .bind(d.id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if rang > 5 {
                return Err(invalide("cinq vœux au plus par dossier"));
            }
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_inscription_voeu (id, etablissement_id, dossier_id, rang, offre_id)
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(id)
            .bind(etab)
            .bind(d.id)
            .bind(rang)
            .bind(offre)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            id
        }
    };
    sqlx::query(
        "UPDATE ent_inscription_voeu SET statut = CASE WHEN id = $3 THEN 'retenu'
                                                       WHEN statut = 'retenu' THEN 'non_retenu' ELSE statut END
          WHERE etablissement_id = $1 AND dossier_id = $2",
    )
    .bind(etab)
    .bind(d.id)
    .bind(voeu)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let rang_attente: Option<i32> = if admis {
        None
    } else {
        Some(
            sqlx::query_scalar(
                "SELECT COALESCE(max(d.rang_attente), 0)::int + 1 FROM ent_inscription_dossier d
                   JOIN ent_inscription_voeu v ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
                  WHERE d.etablissement_id = $1 AND v.offre_id = $2 AND d.etat = 'liste_attente' AND d.id <> $3",
            )
            .bind(etab)
            .bind(offre)
            .bind(d.id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?,
        )
    };
    sqlx::query(
        "UPDATE ent_inscription_dossier SET voeu_retenu_id = $3, rang_attente = $4, updated_at = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(d.id)
    .bind(voeu)
    .bind(rang_attente)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(Some(PlaceRetenue {
        offre_id: offre,
        depassement,
    }))
}

/// Les niveaux d'une campagne (écran historique) deviennent des offres sans parcours : la
/// capacité est tenue par l'offre, quel que soit le chemin de saisie.
pub(crate) async fn synchroniser_offres_niveaux_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    campagne_id: Uuid,
    niveaux: &[NiveauCampagneDto],
    auteur: Uuid,
) -> Result<(), EntDbError> {
    for n in niveaux {
        let code = n.code.trim();
        let niveau_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_niveau WHERE etablissement_id = $1 AND upper(code) = upper($2)",
        )
        .bind(etab)
        .bind(code)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let existante: Option<(Uuid, Option<i32>)> = sqlx::query_as(
            "SELECT id, capacite FROM ent_inscription_offre
              WHERE etablissement_id = $1 AND campagne_id = $2 AND upper(niveau_code) = upper($3)
                AND parcours_code = '' AND regime IS NULL AND archivee_le IS NULL",
        )
        .bind(etab)
        .bind(campagne_id)
        .bind(code)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        match existante {
            Some((id, avant)) => {
                if avant != n.capacite {
                    sqlx::query(
                        "UPDATE ent_inscription_offre SET capacite = $3, modifie_le = now()
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(id)
                    .bind(n.capacite)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                    historiser_capacite_tx(conn, etab, id, avant, n.capacite, Some("Niveaux de la campagne"), auteur)
                        .await?;
                }
            }
            None => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_inscription_offre
                        (id, etablissement_id, campagne_id, niveau_code, niveau_id, parcours_code,
                         libelle_public, capacite, cree_par)
                     VALUES ($1,$2,$3,$4,$5,'',$4,$6,$7)",
                )
                .bind(id)
                .bind(etab)
                .bind(campagne_id)
                .bind(code)
                .bind(niveau_id)
                .bind(n.capacite)
                .bind(auteur)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
                historiser_capacite_tx(conn, etab, id, None, n.capacite, Some("Création"), auteur).await?;
            }
        }
    }
    // Offres sans parcours qui ne sont plus dans la liste : retirées de l'offre si aucun vœu
    // ne s'y rattache (jamais de perte d'historique).
    let codes: Vec<String> = niveaux.iter().map(|n| n.code.trim().to_uppercase()).collect();
    sqlx::query(
        "UPDATE ent_inscription_offre o SET archivee_le = now(), modifie_le = now()
          WHERE o.etablissement_id = $1 AND o.campagne_id = $2 AND o.parcours_code = '' AND o.regime IS NULL
            AND o.archivee_le IS NULL AND NOT (upper(o.niveau_code) = ANY($3))
            AND NOT EXISTS (SELECT 1 FROM ent_inscription_voeu v
                             WHERE v.etablissement_id = o.etablissement_id AND v.offre_id = o.id)",
    )
    .bind(etab)
    .bind(campagne_id)
    .bind(&codes)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

async fn historiser_capacite_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    offre_id: Uuid,
    avant: Option<i32>,
    apres: Option<i32>,
    motif: Option<&str>,
    auteur: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_inscription_offre_historique
            (id, etablissement_id, offre_id, capacite_avant, capacite_apres, motif, modifie_par)
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(offre_id)
    .bind(avant)
    .bind(apres)
    .bind(motif)
    .bind(auteur)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

fn valider_offre(b: &OffreBody) -> Result<(), EntDbError> {
    let code = b.niveau_code.trim();
    if code.is_empty() || code.chars().count() > 40 {
        return Err(invalide("niveau : code obligatoire (40 caractères au plus)"));
    }
    let l = b.libelle_public.trim();
    if l.is_empty() || l.chars().count() > 160 {
        return Err(invalide("libellé public obligatoire (160 caractères au plus)"));
    }
    if b.parcours_code.as_deref().is_some_and(|p| {
        p.chars().count() > 40
            || !p
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    }) {
        return Err(invalide("parcours : code en minuscules (a-z, 0-9, _), 40 caractères au plus"));
    }
    if b.regime.as_deref().is_some_and(|r| !REGIMES.contains(&r)) {
        return Err(invalide("régime : externe, demi_pension ou interne"));
    }
    if b.capacite.is_some_and(|c| c < 0) {
        return Err(invalide("la capacité est positive ou nulle"));
    }
    if b.conditions_acces.as_deref().is_some_and(|c| c.chars().count() > 300) {
        return Err(invalide("conditions d'accès : 300 caractères au plus"));
    }
    if !b.unites.is_empty() {
        if b.regime.as_deref() != Some("interne") {
            return Err(invalide("les unités d'hébergement ne valent que pour un régime interne"));
        }
        if b.unites.iter().any(|u| u.libelle.trim().is_empty() || u.capacite < 0) {
            return Err(invalide("unité d'hébergement : libellé obligatoire, capacité positive"));
        }
        let somme: i64 = b.unites.iter().map(|u| i64::from(u.capacite)).sum();
        if b.capacite.is_some_and(|c| somme > i64::from(c)) {
            return Err(invalide(
                "la somme des capacités des unités dépasse la capacité de l'offre",
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

impl PgEnt {
    /// Offres d'une campagne avec leur compteur de places (INS-47).
    pub async fn offres_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        campagne_id: Uuid,
    ) -> Result<Vec<OffreOccupationDto>, EntDbError> {
        self.ins_personnel(etab, actor, role).await?;
        self.campagne_row(etab, campagne_id).await?;
        self.offres_dto(etab, campagne_id).await
    }

    pub(crate) async fn offres_dto(
        &self,
        etab: Uuid,
        campagne_id: Uuid,
    ) -> Result<Vec<OffreOccupationDto>, EntDbError> {
        let rows: Vec<OffreRow> = sqlx::query_as(&format!(
            "{SELECT_OFFRE} WHERE o.etablissement_id = $1 AND o.campagne_id = $2
              ORDER BY o.ordre, o.niveau_code, o.parcours_code, o.regime NULLS FIRST"
        ))
        .bind(etab)
        .bind(campagne_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let unites = self.unites_dto(etab, r.id).await?;
            out.push(offre_dto(r, unites));
        }
        Ok(out)
    }

    async fn unites_dto(&self, etab: Uuid, offre_id: Uuid) -> Result<Vec<UniteHebergementDto>, EntDbError> {
        let rows: Vec<(Uuid, String, i32, Option<String>, i64)> = sqlx::query_as(
            "SELECT u.id, u.libelle, u.capacite, u.conditions_acces,
                    (SELECT count(*) FROM ent_inscription_voeu v
                       JOIN ent_inscription_dossier d
                         ON d.etablissement_id = v.etablissement_id AND d.voeu_retenu_id = v.id
                      WHERE v.etablissement_id = u.etablissement_id AND v.unite_id = u.id
                        AND ent_inscription_etat_effectif(d.etat, d.etat_avant_blocage)
                            IN ('admis', 'confirme', 'confirmee', 'en_attente_passage', 'validee'))
               FROM ent_inscription_offre_unite u
              WHERE u.etablissement_id = $1 AND u.offre_id = $2 ORDER BY u.libelle",
        )
        .bind(etab)
        .bind(offre_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| UniteHebergementDto {
                id: Some(r.0),
                libelle: r.1,
                capacite: r.2,
                conditions_acces: r.3,
                occupees: r.4,
            })
            .collect())
    }

    async fn offre_dto(&self, etab: Uuid, offre_id: Uuid) -> Result<OffreOccupationDto, EntDbError> {
        let r: OffreRow = sqlx::query_as(&format!(
            "{SELECT_OFFRE} WHERE o.etablissement_id = $1 AND o.id = $2"
        ))
        .bind(etab)
        .bind(offre_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let unites = self.unites_dto(etab, offre_id).await?;
        Ok(offre_dto(r, unites))
    }

    /// INS-18 : définir une offre (niveau × parcours × régime, capacité).
    pub async fn creer_offre_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        campagne_id: Uuid,
        b: &OffreBody,
    ) -> Result<OffreOccupationDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        valider_offre(b)?;
        let c = self.campagne_row(etab, campagne_id).await?;
        if matches!(c.statut.as_str(), "archivee") {
            return Err(conflit("campagne archivée"));
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        let niveau_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_niveau WHERE etablissement_id = $1 AND upper(code) = upper($2)",
        )
        .bind(etab)
        .bind(b.niveau_code.trim())
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let r = sqlx::query(
            "INSERT INTO ent_inscription_offre
                (id, etablissement_id, campagne_id, niveau_code, niveau_id, parcours_code, regime,
                 libelle_public, capacite, places_reservees_reinscription, ouverte_candidature,
                 conditions_acces, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(id)
        .bind(etab)
        .bind(campagne_id)
        .bind(b.niveau_code.trim())
        .bind(niveau_id)
        .bind(b.parcours_code.clone().unwrap_or_default())
        .bind(&b.regime)
        .bind(b.libelle_public.trim())
        .bind(b.capacite)
        .bind(b.places_reservees_reinscription.unwrap_or(true))
        .bind(b.ouverte_candidature.unwrap_or(true))
        .bind(opt(&b.conditions_acces))
        .bind(actor)
        .execute(&mut *tx)
        .await;
        if let Err(e) = r {
            return Err(match crate::pg_err::map_sqlx("inscriptions", e) {
                domain_eleve::DomainError::Conflit => {
                    conflit("une offre existe déjà pour ce niveau, ce parcours et ce régime")
                }
                d => EntDbError::Domain(d),
            });
        }
        historiser_capacite_tx(&mut tx, etab, id, None, b.capacite, Some("Création"), actor).await?;
        Self::ecrire_unites_tx(&mut tx, etab, id, &b.unites).await?;
        if !matches!(c.statut.as_str(), "brouillon") {
            nouvelle_version_tx(&mut tx, etab, campagne_id, Some("Offre ajoutée"), actor).await?;
        }
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_OFFRE_MODIFIEE,
            AggregateKind::Admission,
            id,
            None,
            json!({ "offre_id": id, "campagne_id": campagne_id, "capacite": b.capacite }),
            Some(actor),
        )
        .await?;
        inscriptions::commit(tx).await?;
        self.offre_dto(etab, id).await
    }

    /// INS-18 : modifier une offre. Le niveau, le parcours et le régime (la clé de l'offre) ne
    /// changent pas : une autre offre se crée. Un changement de capacité est tracé avec son motif ;
    /// une capacité inférieure aux places déjà prises est possible (dépassement assumé, visible).
    pub async fn modifier_offre_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        offre_id: Uuid,
        b: &OffreBody,
    ) -> Result<OffreOccupationDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        valider_offre(b)?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let (campagne_id, niveau, parcours, regime, avant): (Uuid, String, String, Option<String>, Option<i32>) =
            sqlx::query_as(
                "SELECT campagne_id, niveau_code, parcours_code, regime, capacite FROM ent_inscription_offre
                  WHERE etablissement_id = $1 AND id = $2 AND archivee_le IS NULL FOR UPDATE",
            )
            .bind(etab)
            .bind(offre_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        if niveau.to_uppercase() != b.niveau_code.trim().to_uppercase()
            || parcours != b.parcours_code.clone().unwrap_or_default()
            || regime != b.regime
        {
            return Err(conflit(
                "le niveau, le parcours et le régime d'une offre ne changent pas : créez une autre offre",
            ));
        }
        sqlx::query(
            "UPDATE ent_inscription_offre SET libelle_public = $3, capacite = $4,
                    places_reservees_reinscription = COALESCE($5, places_reservees_reinscription),
                    ouverte_candidature = COALESCE($6, ouverte_candidature),
                    conditions_acces = $7, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(offre_id)
        .bind(b.libelle_public.trim())
        .bind(b.capacite)
        .bind(b.places_reservees_reinscription)
        .bind(b.ouverte_candidature)
        .bind(opt(&b.conditions_acces))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if avant != b.capacite {
            historiser_capacite_tx(&mut tx, etab, offre_id, avant, b.capacite, b.motif.as_deref(), actor)
                .await?;
        }
        Self::ecrire_unites_tx(&mut tx, etab, offre_id, &b.unites).await?;
        let statut: String = sqlx::query_scalar(
            "SELECT statut FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(campagne_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if statut != "brouillon" {
            nouvelle_version_tx(&mut tx, etab, campagne_id, b.motif.as_deref(), actor).await?;
        }
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_OFFRE_MODIFIEE,
            AggregateKind::Admission,
            offre_id,
            None,
            json!({ "offre_id": offre_id, "campagne_id": campagne_id, "capacite": b.capacite }),
            Some(actor),
        )
        .await?;
        inscriptions::commit(tx).await?;
        self.offre_dto(etab, offre_id).await
    }

    /// Retire une offre de la campagne (archivage) : refusé si un vœu s'y rattache.
    pub async fn archiver_offre_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        offre_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let (utilises,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_inscription_voeu WHERE etablissement_id = $1 AND offre_id = $2",
        )
        .bind(etab)
        .bind(offre_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if utilises > 0 {
            return Err(conflit("des vœux se rattachent à cette offre : elle ne peut pas être retirée"));
        }
        let r = sqlx::query(
            "UPDATE ent_inscription_offre SET archivee_le = now(), modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND archivee_le IS NULL",
        )
        .bind(etab)
        .bind(offre_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        inscriptions::commit(tx).await
    }

    async fn ecrire_unites_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        offre_id: Uuid,
        unites: &[UniteHebergementDto],
    ) -> Result<(), EntDbError> {
        let gardees: Vec<Uuid> = unites.iter().filter_map(|u| u.id).collect();
        // Une unité retirée de la liste disparaît seulement si aucun vœu n'y est rattaché.
        sqlx::query(
            "DELETE FROM ent_inscription_offre_unite u
              WHERE u.etablissement_id = $1 AND u.offre_id = $2 AND NOT (u.id = ANY($3))
                AND NOT EXISTS (SELECT 1 FROM ent_inscription_voeu v
                                 WHERE v.etablissement_id = u.etablissement_id AND v.unite_id = u.id)",
        )
        .bind(etab)
        .bind(offre_id)
        .bind(&gardees)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        for u in unites {
            match u.id {
                Some(id) => {
                    sqlx::query(
                        "UPDATE ent_inscription_offre_unite SET libelle = $4, capacite = $5, conditions_acces = $6
                          WHERE etablissement_id = $1 AND offre_id = $2 AND id = $3",
                    )
                    .bind(etab)
                    .bind(offre_id)
                    .bind(id)
                    .bind(u.libelle.trim())
                    .bind(u.capacite)
                    .bind(opt(&u.conditions_acces))
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                }
                None => {
                    sqlx::query(
                        "INSERT INTO ent_inscription_offre_unite
                            (id, etablissement_id, offre_id, libelle, capacite, conditions_acces)
                         VALUES ($1,$2,$3,$4,$5,$6)",
                    )
                    .bind(Uuid::new_v4())
                    .bind(etab)
                    .bind(offre_id)
                    .bind(u.libelle.trim())
                    .bind(u.capacite)
                    .bind(opt(&u.conditions_acces))
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                }
            }
        }
        Ok(())
    }

    // -- Statut de campagne (INS-17) ---------------------------------------

    /// Suspendre, fermer, archiver, replanifier : transitions de §8.1 (une campagne fermée ne
    /// se rouvre pas ; la fermeture des dépôts n'est pas la fin des décisions).
    pub async fn changer_statut_campagne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        vers: &str,
    ) -> Result<CampagneDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let (de,): (String,) = sqlx::query_as(
            "SELECT statut FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if !transition_campagne_admise(&de, vers) {
            return Err(conflit("changement d'état de campagne impossible depuis l'état actuel"));
        }
        let r = sqlx::query(
            "UPDATE ent_inscription_campagne SET statut = $3, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(vers)
        .execute(&mut *tx)
        .await;
        if let Err(e) = r {
            return Err(match crate::pg_err::map_sqlx("inscriptions", e) {
                domain_eleve::DomainError::Conflit => {
                    conflit("une campagne de même nature est déjà ouverte pour cette année")
                }
                d => EntDbError::Domain(d),
            });
        }
        nouvelle_version_tx(&mut tx, etab, id, Some(vers), actor).await?;
        if vers == "fermee" {
            publier(
                &mut tx,
                etab,
                types::ADMISSION_CAMPAGNE_FERMEE,
                AggregateKind::Admission,
                id,
                None,
                json!({ "campagne_id": id }),
                Some(actor),
            )
            .await?;
        }
        inscriptions::commit(tx).await?;
        let r = self.campagne_row(etab, id).await?;
        self.campagne_dto(etab, r, true).await
    }

    // -- Transitions (§7) ----------------------------------------------------

    /// Changements d'état hors instruction et hors décision : classement sans suite, annulation
    /// pour erreur, désistement constaté, non-présentation, réaffectation, non-réinscription
    /// décidée par l'établissement, sans réponse, réouverture. Chaque transition est celle de la
    /// table de [`inscriptions_etats`] ; une autre est refusée.
    pub async fn transition_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &TransitionBody,
    ) -> Result<DossierDto, EntDbError> {
        let (action, decideur) = match b.action.as_str() {
            "annuler" | "non_reinscription_etablissement" => (b.action.as_str(), true),
            "sans_suite" | "desister" | "reprendre" | "non_presente" | "reaffecte" | "sans_reponse"
            | "reouvrir" | "en_attente_passage" => (b.action.as_str(), false),
            _ => return Err(invalide("action de transition inconnue")),
        };
        self.ins_exiger(
            etab,
            actor,
            role,
            if decideur { Action::InscriptionsDecider } else { Action::InscriptionsGerer },
        )
        .await?;
        let motif = b.motif.trim();
        if !(3..=1000).contains(&motif.chars().count()) {
            return Err(invalide("motif obligatoire (3 à 1000 caractères)"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = inscriptions::dossier_tx(&mut tx, etab, id).await?;
        let vers = match (action, d.nature.as_str()) {
            ("sans_suite", _) => "sans_suite",
            ("annuler", _) => "annule",
            ("desister", "reinscription") => "non_reinscription_famille",
            ("desister", _) => "desiste",
            ("reprendre", _) => "en_instruction",
            ("non_presente", _) => "non_presente",
            ("reaffecte", _) => "reaffecte",
            ("non_reinscription_etablissement", _) => "non_reinscription_etablissement",
            ("sans_reponse", _) => "sans_reponse",
            ("reouvrir", _) => "a_confirmer",
            ("en_attente_passage", _) => "en_attente_passage",
            _ => return Err(invalide("action de transition inconnue")),
        };
        // Désistement après validation : l'inscription annuelle prévue est retirée (la fiche
        // élève, elle, reste : l'enfant a existé dans l'établissement).
        if d.etat == "validee" && matches!(vers, "desiste" | "non_reinscription_famille" | "non_presente") {
            if let Some(e) = d.eleve_id {
                eleve_ports::retirer_inscription_annuelle_prevue_tx(&mut tx, etab, e, d.id).await?;
            }
        }
        let n = transitionner_tx(&mut tx, etab, &d, vers, Cause::Etape, Some(motif), Some(actor)).await?;
        publier(
            &mut tx,
            etab,
            if matches!(vers, "desiste") {
                types::INSCRIPTION_DESISTEMENT
            } else {
                types::ADMISSION_ETAPE_CHANGEE
            },
            AggregateKind::Admission,
            id,
            d.eleve_id,
            json!({ "dossier_id": id, "de": d.etat, "vers": n.etat }),
            Some(actor),
        )
        .await?;
        inscriptions::commit(tx).await?;
        self.dossier_for_actor(etab, actor, role, id).await
    }

    /// Désaccord des responsables (§4.4) : poser le blocage (secrétariat), le lever avec pièce
    /// (direction). Aucune décision ni validation n'est possible tant que le dossier est bloqué ;
    /// la levée ramène à l'état d'avant.
    pub async fn blocage_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &BlocageBody,
    ) -> Result<DossierDto, EntDbError> {
        self.ins_exiger(
            etab,
            actor,
            role,
            if b.bloquer { Action::InscriptionsGerer } else { Action::InscriptionsDecider },
        )
        .await?;
        let motif = b.motif.trim();
        if !(3..=1000).contains(&motif.chars().count()) {
            return Err(invalide("motif obligatoire (3 à 1000 caractères)"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = inscriptions::dossier_tx(&mut tx, etab, id).await?;
        let vers = if b.bloquer {
            "bloque_desaccord".to_string()
        } else {
            d.etat_avant_blocage
                .clone()
                .filter(|_| d.etat == "bloque_desaccord")
                .ok_or_else(|| conflit("le dossier n'est pas bloqué"))?
        };
        transitionner_tx(&mut tx, etab, &d, &vers, Cause::Blocage, Some(motif), Some(actor)).await?;
        publier(
            &mut tx,
            etab,
            if b.bloquer {
                types::INSCRIPTION_BLOQUE_DESACCORD
            } else {
                types::INSCRIPTION_BLOCAGE_LEVE
            },
            AggregateKind::Admission,
            id,
            d.eleve_id,
            json!({ "dossier_id": id, "etat": vers }),
            Some(actor),
        )
        .await?;
        inscriptions::commit(tx).await?;
        self.dossier_for_actor(etab, actor, role, id).await
    }

    // -- Famille : retirer, se désister ---------------------------------------

    /// INS-25 : le déposant retire sa candidature, avant toute admission (sans motif obligatoire).
    pub async fn retirer_candidature_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
    ) -> Result<DossierDto, EntDbError> {
        self.sortie_famille(etab, jeton, id, "retire", &["brouillon", "deposee", "en_instruction", "attente_famille", "liste_attente"], "Candidature retirée par la famille")
            .await
    }

    /// INS-39 : le déposant se désiste après une admission (avant la validation).
    pub async fn desister_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
    ) -> Result<DossierDto, EntDbError> {
        self.sortie_famille(etab, jeton, id, "desiste", &["admis", "confirme"], "Désistement de la famille")
            .await
    }

    async fn sortie_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
        vers: &str,
        depuis: &[&str],
        texte: &str,
    ) -> Result<DossierDto, EntDbError> {
        let (v, d0) = self.dossier_du_deposant(etab, jeton, id).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = inscriptions::dossier_tx(&mut tx, etab, id).await?;
        if d.demarche_id != d0.demarche_id || !matches!(d.nature.as_str(), "admission" | "en_cours_annee") {
            return Err(introuvable());
        }
        let etat_actif = inscriptions_etats::etat_effectif(&d.etat, d.etat_avant_blocage.as_deref());
        if !depuis.contains(&etat_actif) || d.etat == "bloque_desaccord" {
            return Err(conflit(if vers == "retire" {
                "la candidature ne peut plus être retirée à ce stade : l'établissement vous répond, ou vous pouvez vous désister"
            } else {
                "le désistement ne s'applique qu'à une admission non encore validée : contactez l'établissement"
            }));
        }
        transitionner_tx(&mut tx, etab, &d, vers, Cause::Etape, Some(texte), None).await?;
        publier(
            &mut tx,
            etab,
            if vers == "retire" { types::ADMISSION_RETIREE } else { types::INSCRIPTION_DESISTEMENT },
            AggregateKind::Admission,
            id,
            d.eleve_id,
            json!({ "dossier_id": id, "campagne_id": d.campagne_id }),
            v.user_id,
        )
        .await?;
        inscriptions::commit(tx).await?;
        self.dossier_famille(etab, jeton, id).await
    }

    // -- « Ne pas solliciter » (INS-57, R-I8) ---------------------------------

    /// INS-57 ★ : la direction marque un responsable « à ne pas solliciter » (danger, ordonnance
    /// de protection, décision de justice). Effets **immédiats** : plus de demande d'accord,
    /// d'invitation, de relance ni de copie vers cette personne, y compris ce qui était déjà
    /// programmé ; motif scellé, jamais renvoyé ; événement sans motif ni personne. Le déposant
    /// (interlocuteur du dossier) ne peut pas être marqué.
    pub async fn ne_pas_solliciter_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &NePasSolliciterBody,
    ) -> Result<NePasSolliciterDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider).await?;
        let motif = b.motif.trim();
        if !(3..=1000).contains(&motif.chars().count()) {
            return Err(invalide("motif obligatoire (3 à 1000 caractères)"));
        }
        if !(1..=RESPONSABLES_MAX as i16).contains(&b.rang) {
            return Err(invalide("rang de responsable : 1 à 6"));
        }
        if b.rang == 1 {
            return Err(invalide(
                "le déposant est l'interlocuteur du dossier : il ne peut pas être marqué « à ne pas solliciter »",
            ));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = inscriptions::dossier_tx(&mut tx, etab, id).await?;
        let lignes: Vec<(Uuid, Option<String>)> = sqlx::query_as(
            "SELECT r.id, lower(r.email) FROM ent_inscription_responsable r
               JOIN ent_inscription_dossier x ON x.etablissement_id = r.etablissement_id AND x.id = r.dossier_id
              WHERE r.etablissement_id = $1 AND x.demarche_id = $2 AND r.rang = $3",
        )
        .bind(etab)
        .bind(d.demarche_id)
        .bind(b.rang)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if lignes.is_empty() {
            return Err(introuvable());
        }
        let coffre = ChampsSensibles::depuis_env();
        let mut envois = 0_i64;
        let mut invitations = 0_i64;
        for (rid, email) in &lignes {
            let scelle = coffre
                .sceller_opt("ent_inscription_responsable", "ne_pas_solliciter_ref", *rid, Some(motif))
                .map_err(EntDbError::Domain)?;
            sqlx::query(
                "UPDATE ent_inscription_responsable SET ne_pas_solliciter = true, ne_pas_solliciter_ref = $3
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(rid)
            .bind(scelle)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            invitations += i64::try_from(
                sqlx::query(
                    "UPDATE ent_inscription_invitation SET statut = 'annulee'
                      WHERE etablissement_id = $1 AND responsable_id = $2
                        AND statut IN ('a_envoyer', 'envoyee', 'non_envoyee')",
                )
                .bind(etab)
                .bind(rid)
                .execute(&mut *tx)
                .await
                .map_err(db)?
                .rows_affected(),
            )
            .unwrap_or(0);
            sqlx::query(
                "UPDATE ent_inscription_lien_attente SET statut = 'annule', decide_le = now()
                  WHERE etablissement_id = $1 AND responsable_id = $2 AND statut = 'en_attente'",
            )
            .bind(etab)
            .bind(rid)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if let Some(e) = email {
                envois += i64::try_from(
                    sqlx::query(
                        "UPDATE ent_inscription_envoi SET statut = 'annule', derniere_erreur = 'annulé : à ne pas solliciter'
                          WHERE etablissement_id = $1 AND statut = 'a_envoyer' AND lower(destinataire) = $2
                            AND dossier_id IN (SELECT id FROM ent_inscription_dossier
                                                WHERE etablissement_id = $1 AND demarche_id = $3)",
                    )
                    .bind(etab)
                    .bind(e)
                    .bind(d.demarche_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?
                    .rows_affected(),
                )
                .unwrap_or(0);
            }
        }
        suivi_tx(
            &mut tx,
            etab,
            id,
            "commentaire",
            None,
            Some("Une règle de confidentialité s'applique à ce dossier."),
            false,
            Some(actor),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_SOLLICITATION_SUSPENDUE,
            AggregateKind::Admission,
            id,
            d.eleve_id,
            json!({ "dossier_id": id, "demarche_id": d.demarche_id }),
            Some(actor),
        )
        .await?;
        inscriptions::commit(tx).await?;
        Ok(NePasSolliciterDto {
            envois_annules: envois,
            invitations_annulees: invitations,
        })
    }

    // -- Annulation d'une validation erronée (INS-68, R-I11) -----------------

    /// INS-68 ★ : annule une validation faite par erreur (mauvais enfant, mauvaise année, doublon),
    /// dans le délai de la campagne (72 h par défaut) et tant qu'**aucun autre socle** n'a produit de
    /// données depuis (la liste est montrée : il faut alors le désistement). Une transaction :
    /// fiche créée par la validation archivée (socle 2), inscription annuelle prévue retirée
    /// (socle 2), liens créés par la validation suspendus (socle 4), invitations révoquées,
    /// lignes de facturation brouillon retirées (facturation) ; le dossier revient à `confirme`
    /// (admission), `confirmee` (réinscription) ou `complete` (élèves affectés).
    pub async fn annuler_validation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &AnnulationValidationBody,
    ) -> Result<AnnulationValidationDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider).await?;
        let motif = b.motif.trim();
        if !(3..=1000).contains(&motif.chars().count()) {
            return Err(invalide("motif obligatoire (3 à 1000 caractères)"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let d = inscriptions::dossier_tx(&mut tx, etab, id).await?;
        if d.etat != "validee" {
            return Err(conflit("seule une inscription validée peut être annulée"));
        }
        let (Some(eleve_id), Some(valide_le)) = (d.eleve_id, d.valide_le) else {
            return Err(conflit("dossier validé sans élève : incohérence, contactez le support"));
        };
        let (delai_h, annee): (i16, String) = sqlx::query_as(
            "SELECT delai_annulation_validation_h, annee_cible FROM ent_inscription_campagne
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(d.campagne_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let ecoule: bool = sqlx::query_scalar("SELECT now() > $1::timestamptz + make_interval(hours => $2::int)")
            .bind(valide_le)
            .bind(i32::from(delai_h))
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if ecoule {
            return Err(conflit(
                "délai d'annulation dépassé : utilisez le désistement (ou la procédure du socle élève)",
            ));
        }
        // Données produites depuis par un autre socle.
        let depuis: Vec<(String, i64)> = sqlx::query_as(
            "SELECT type, count(*) FROM ent_metier_event
              WHERE etablissement_id = $1 AND eleve_id = $2 AND created_at > $3
                AND type <> ALL($4) AND type NOT LIKE 'inscription.%' AND type NOT LIKE 'admission.%'
              GROUP BY type ORDER BY type",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(valide_le)
        .bind(EVENEMENTS_DE_VALIDATION.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if !depuis.is_empty() {
            return Err(EntDbError::ConflitDossierDetail(format!(
                "des données ont été produites depuis la validation ({}) : utilisez le désistement",
                depuis
                    .iter()
                    .map(|(t, n)| format!("{t} ×{n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        // Facturation : seules des lignes brouillon peuvent être retirées.
        if let Err(raison) = finances::annuler_lignes_annuelles_brouillon_tx(&mut tx, etab, &annee, eleve_id).await? {
            return Err(EntDbError::ConflitDossierDetail(raison));
        }
        // Fiche créée par la validation (même horodatage) : archivée ; fiche déjà connue : intacte.
        let fiche_creee: bool = sqlx::query_scalar(
            "SELECT created_at = $3 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(valide_le)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let eleve_archive = if fiche_creee {
            let ok = eleve_ports::annuler_creation_eleve_tx(&mut tx, etab, eleve_id, id, actor).await?;
            if !ok {
                return Err(conflit(
                    "l'élève a déjà une classe ou un statut : l'annulation n'est plus possible, utilisez le désistement",
                ));
            }
            true
        } else {
            false
        };
        let inscription_retiree =
            eleve_ports::retirer_inscription_annuelle_prevue_tx(&mut tx, etab, eleve_id, id).await?;
        let liens = if fiche_creee {
            dossier_personnes::suspendre_liens_crees_le_tx(
                &mut tx,
                etab,
                eleve_id,
                valide_le,
                "Validation annulée",
                actor,
            )
            .await?
        } else {
            0
        };
        let invitations = i64::try_from(
            sqlx::query(
                "UPDATE ent_inscription_invitation SET statut = 'annulee'
                  WHERE etablissement_id = $1 AND dossier_id = $2
                    AND statut IN ('a_envoyer', 'envoyee', 'non_envoyee')",
            )
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected(),
        )
        .unwrap_or(0);
        sqlx::query(
            "UPDATE ent_inscription_envoi SET statut = 'annule', derniere_erreur = 'annulé : validation annulée'
              WHERE etablissement_id = $1 AND dossier_id = $2 AND statut = 'a_envoyer'",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_inscription_lien_attente SET statut = 'annule', decide_le = now()
              WHERE etablissement_id = $1 AND dossier_id = $2 AND statut = 'en_attente'",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let vers = inscriptions_etats::etat_apres_annulation_de_validation(&d.nature);
        verifier_retour(&d, vers)?;
        sqlx::query(
            "UPDATE ent_inscription_dossier
                SET validation_annulee_le = now(), validation_annulee_par = $3, validation_annulee_motif = $4,
                    valide_le = NULL, valide_par = NULL, inscription_annuelle_id = NULL
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(actor)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // `eleve_id` reste au dossier si la fiche préexistait (réinscription) ; sinon il est retiré
        // pour qu'une nouvelle validation crée ou rapproche la bonne fiche.
        if fiche_creee {
            sqlx::query(
                "UPDATE ent_inscription_dossier SET eleve_id = NULL
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let n = transitionner_tx(&mut tx, etab, &d, vers, Cause::AnnulationValidation, Some(motif), Some(actor))
            .await?;
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_VALIDATION_ANNULEE,
            AggregateKind::Eleve,
            eleve_id,
            Some(eleve_id),
            json!({ "dossier_id": id, "eleve_id": eleve_id, "eleve_archive": eleve_archive }),
            Some(actor),
        )
        .await?;
        inscriptions::commit(tx).await?;
        Ok(AnnulationValidationDto {
            dossier_id: id,
            etat: n.etat,
            eleve_archive,
            inscription_annuelle_retiree: inscription_retiree,
            liens_suspendus: liens,
            invitations_revoquees: invitations,
        })
    }

    // -- Prolongation et fermetures (INS-79, INS-80) --------------------------

    /// INS-79 ★ : prolonge une échéance pour toute une campagne ou une sélection de dossiers
    /// (panne, intempérie, grève postale). Une échéance ne recule jamais ; les familles concernées
    /// sont prévenues par la file d'envoi (jamais une personne « à ne pas solliciter ») ; les
    /// relances repartent d'aujourd'hui.
    pub async fn prolonger_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        campagne_id: Uuid,
        b: &ProlongationBody,
    ) -> Result<ProlongationDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        let motif = b.motif_public.trim();
        if !(3..=200).contains(&motif.chars().count()) {
            return Err(invalide("motif public court obligatoire (3 à 200 caractères)"));
        }
        if !["date_limite_famille", "cloture", "echeance_dossiers"].contains(&b.objet.as_str()) {
            return Err(invalide("objet : date_limite_famille, cloture ou echeance_dossiers"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let c = sqlx::query_as::<_, CampagneRow>(&format!(
            "SELECT {COLS_CAMPAGNE} FROM ent_inscription_campagne
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(etab)
        .bind(campagne_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if matches!(c.statut.as_str(), "archivee") {
            return Err(conflit("campagne archivée"));
        }
        let nouvelle = echeance_effective_tx(&mut tx, etab, b.nouvelle_date).await?;
        let mut ancienne: Option<NaiveDate> = None;
        let mut concernes: Vec<Uuid> = Vec::new();
        match b.objet.as_str() {
            "date_limite_famille" => {
                let a = c.date_limite_famille.ok_or_else(|| conflit("cette campagne n'a pas de date limite famille"))?;
                if nouvelle <= a {
                    return Err(conflit("une échéance ne recule jamais : la nouvelle date doit être postérieure"));
                }
                ancienne = Some(a);
                sqlx::query(
                    "UPDATE ent_inscription_campagne SET date_limite_famille = $3 WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(campagne_id)
                .bind(nouvelle)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                concernes = Self::prolonger_dossiers_tx(&mut tx, etab, campagne_id, &c.nature, None, nouvelle).await?;
            }
            "cloture" => {
                let a = c.date_cloture_depot.or(c.date_cloture).ok_or_else(|| conflit("cette campagne n'a pas de date de clôture"))?;
                if nouvelle <= a {
                    return Err(conflit("une échéance ne recule jamais : la nouvelle date doit être postérieure"));
                }
                ancienne = Some(a);
                sqlx::query(
                    "UPDATE ent_inscription_campagne
                        SET date_cloture_depot = CASE WHEN date_cloture_depot IS NOT NULL THEN $3 END,
                            date_cloture = GREATEST(date_cloture, $3)
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(campagne_id)
                .bind(nouvelle)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            _ => {
                concernes = Self::prolonger_dossiers_tx(
                    &mut tx,
                    etab,
                    campagne_id,
                    &c.nature,
                    (!b.dossiers.is_empty()).then_some(&b.dossiers[..]),
                    nouvelle,
                )
                .await?;
                if !b.dossiers.is_empty() && concernes.len() != b.dossiers.len() {
                    return Err(conflit("la sélection contient un dossier inconnu ou clos"));
                }
            }
        }
        let familles = Self::prevenir_familles_tx(
            &mut tx,
            etab,
            &concernes,
            "Échéance prolongée",
            &format!("L'échéance de votre démarche est prolongée jusqu'au {nouvelle}. {motif}"),
            &format!("prolongation:{campagne_id}:{nouvelle}:{}", Uuid::new_v4()),
        )
        .await?;
        let pid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_inscription_prolongation
                (id, etablissement_id, campagne_id, objet, ancienne_date, nouvelle_date, motif_public,
                 nb_dossiers, familles_prevenues, decide_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(pid)
        .bind(etab)
        .bind(campagne_id)
        .bind(&b.objet)
        .bind(ancienne)
        .bind(nouvelle)
        .bind(motif)
        .bind(i32::try_from(concernes.len()).unwrap_or(i32::MAX))
        .bind(i32::try_from(familles).unwrap_or(i32::MAX))
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        nouvelle_version_tx(&mut tx, etab, campagne_id, Some("Échéance prolongée"), actor).await?;
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_ECHEANCE_PROLONGEE,
            AggregateKind::Admission,
            campagne_id,
            None,
            json!({ "campagne_id": campagne_id, "objet": b.objet, "nouvelle_date": nouvelle, "nb_dossiers": concernes.len() }),
            Some(actor),
        )
        .await?;
        inscriptions::commit(tx).await?;
        Ok(ProlongationDto {
            id: pid,
            objet: b.objet.clone(),
            ancienne_date: ancienne,
            nouvelle_date: nouvelle,
            nb_dossiers: i64::try_from(concernes.len()).unwrap_or(i64::MAX),
            familles_prevenues: i64::try_from(familles).unwrap_or(i64::MAX),
        })
    }

    /// Décale `echeance_famille` des dossiers ouverts (jamais en arrière) et repart les relances.
    async fn prolonger_dossiers_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        campagne_id: Uuid,
        nature: &str,
        selection: Option<&[Uuid]>,
        nouvelle: NaiveDate,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let ouverts: Vec<String> = inscriptions_etats::etats(nature)
            .iter()
            .filter(|e| inscriptions_etats::est_ouvert(nature, e))
            .map(|e| e.to_string())
            .collect();
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "UPDATE ent_inscription_dossier
                SET echeance_famille = $3, derniere_relance_le = now(), updated_at = now()
              WHERE etablissement_id = $1 AND campagne_id = $2 AND etat = ANY($4)
                AND (echeance_famille IS NULL OR echeance_famille < $3)
                AND ($5::uuid[] IS NULL OR id = ANY($5))
             RETURNING id",
        )
        .bind(etab)
        .bind(campagne_id)
        .bind(nouvelle)
        .bind(&ouverts)
        .bind(selection)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        Ok(ids)
    }

    /// Met un message d'information en file, une fois par démarche et par adresse, vers les seuls
    /// responsables qui peuvent être sollicités. Renvoie le nombre de familles prévenues.
    async fn prevenir_familles_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        dossiers: &[Uuid],
        sujet: &str,
        corps: &str,
        prefixe_cle: &str,
    ) -> Result<usize, EntDbError> {
        if dossiers.is_empty() {
            return Ok(0);
        }
        let cibles: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT DISTINCT ON (d.demarche_id, lower(r.email)) d.demarche_id, d.id, lower(r.email)
               FROM ent_inscription_dossier d
               JOIN ent_inscription_responsable r
                 ON r.etablissement_id = d.etablissement_id AND r.dossier_id = d.id
              WHERE d.etablissement_id = $1 AND d.id = ANY($2) AND r.email IS NOT NULL AND r.rang = 1
                AND NOT EXISTS (
                    SELECT 1 FROM ent_inscription_responsable x
                      JOIN ent_inscription_dossier y ON y.etablissement_id = x.etablissement_id AND y.id = x.dossier_id
                     WHERE x.etablissement_id = d.etablissement_id AND y.demarche_id = d.demarche_id
                       AND x.ne_pas_solliciter AND lower(x.email) = lower(r.email))
              ORDER BY d.demarche_id, lower(r.email), d.id",
        )
        .bind(etab)
        .bind(dossiers)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut n = 0;
        for (demarche, dossier, email) in cibles {
            sqlx::query(
                "INSERT INTO ent_inscription_envoi
                    (id, etablissement_id, dossier_id, cle, kind, destinataire, sujet, corps)
                 VALUES ($1,$2,$3,$4,'information',$5,$6,$7)
                 ON CONFLICT (etablissement_id, cle) DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(dossier)
            .bind(format!("{prefixe_cle}:{demarche}"))
            .bind(email)
            .bind(sujet)
            .bind(format!("Bonjour,\n\n{corps}\n"))
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            n += 1;
        }
        Ok(n)
    }

    /// INS-80 : déclare une fermeture du secrétariat. Les délais des familles qui tombent pendant
    /// la fermeture sont reportés au jour de réouverture + N jours ; les relances sont suspendues
    /// pendant la fermeture ; un message s'affiche sur la démarche de la famille.
    pub async fn declarer_fermeture_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &FermetureBody,
    ) -> Result<FermetureDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        let motif = b.motif_public.trim();
        if !(3..=200).contains(&motif.chars().count()) {
            return Err(invalide("motif public court obligatoire (3 à 200 caractères)"));
        }
        if b.date_fin < b.date_debut || (b.date_fin - b.date_debut).num_days() > FERMETURE_JOURS_MAX {
            return Err(invalide("période de fermeture invalide (120 jours au plus)"));
        }
        let report = b.report_jours.unwrap_or(3);
        if !(0..=30).contains(&report) {
            return Err(invalide("report : 0 à 30 jours"));
        }
        let contact = bornee(opt(&b.contact_urgence), 200, "contact d'urgence : 200 caractères au plus")?;
        let reouverture = b.date_fin + Duration::days(1 + i64::from(report));
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        // Dossiers ouverts dont l'échéance tombe pendant la fermeture.
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "UPDATE ent_inscription_dossier SET echeance_famille = $4, updated_at = now()
              WHERE etablissement_id = $1 AND echeance_famille BETWEEN $2 AND $3
                AND etat NOT IN ('validee','refuse','sans_suite','desiste','retire','annule','non_presente',
                                 'reaffecte','non_reinscription_famille','non_reinscription_etablissement')
             RETURNING id",
        )
        .bind(etab)
        .bind(b.date_debut)
        .bind(b.date_fin)
        .bind(reouverture)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // Dates limites de famille des campagnes non closes qui tombent pendant la fermeture.
        let campagnes: Vec<Uuid> = sqlx::query_scalar(
            "UPDATE ent_inscription_campagne SET date_limite_famille = $4, version = version + 1, updated_at = now()
              WHERE etablissement_id = $1 AND date_limite_famille BETWEEN $2 AND $3
                AND statut IN ('brouillon','planifiee','ouverte','suspendue')
             RETURNING id",
        )
        .bind(etab)
        .bind(b.date_debut)
        .bind(b.date_fin)
        .bind(reouverture)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for c in &campagnes {
            ecrire_version_tx(&mut tx, etab, *c, Some("Fermeture du secrétariat"), actor).await?;
        }
        let total = ids.len() + campagnes.len();
        sqlx::query(
            "INSERT INTO ent_inscription_fermeture
                (id, etablissement_id, date_debut, date_fin, report_jours, motif_public, contact_urgence,
                 echeances_reportees, declaree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.date_debut)
        .bind(b.date_fin)
        .bind(report)
        .bind(motif)
        .bind(&contact)
        .bind(i32::try_from(total).unwrap_or(i32::MAX))
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        Self::prevenir_familles_tx(
            &mut tx,
            etab,
            &ids,
            "Fermeture du secrétariat",
            &format!(
                "Le secrétariat est fermé du {} au {}. {motif} Votre échéance est reportée au {reouverture}.",
                b.date_debut, b.date_fin
            ),
            &format!("fermeture:{id}"),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_FERMETURE_DECLAREE,
            AggregateKind::Admission,
            id,
            None,
            json!({ "fermeture_id": id, "date_debut": b.date_debut, "date_fin": b.date_fin, "echeances_reportees": total }),
            Some(actor),
        )
        .await?;
        inscriptions::commit(tx).await?;
        Ok(FermetureDto {
            id,
            date_debut: b.date_debut,
            date_fin: b.date_fin,
            report_jours: report,
            motif_public: motif.to_string(),
            contact_urgence: contact,
            echeances_reportees: i64::try_from(total).unwrap_or(i64::MAX),
        })
    }

    /// Fermetures déclarées (à venir, en cours, passées récentes).
    pub async fn fermetures_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<FermetureDto>, EntDbError> {
        self.ins_personnel(etab, actor, role).await?;
        let rows: Vec<(Uuid, NaiveDate, NaiveDate, i16, String, Option<String>, i32)> = sqlx::query_as(
            "SELECT id, date_debut, date_fin, report_jours, motif_public, contact_urgence, echeances_reportees
               FROM ent_inscription_fermeture
              WHERE etablissement_id = $1 AND annulee_le IS NULL
              ORDER BY date_debut DESC LIMIT 100",
        )
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| FermetureDto {
                id: r.0,
                date_debut: r.1,
                date_fin: r.2,
                report_jours: r.3,
                motif_public: r.4,
                contact_urgence: r.5,
                echeances_reportees: i64::from(r.6),
            })
            .collect())
    }

    /// Annule une fermeture (les échéances déjà reportées ne reculent pas : jamais au détriment
    /// d'une famille).
    pub async fn annuler_fermeture_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        let r = sqlx::query(
            "UPDATE ent_inscription_fermeture SET annulee_le = now(), annulee_par = $3
              WHERE etablissement_id = $1 AND id = $2 AND annulee_le IS NULL",
        )
        .bind(etab)
        .bind(id)
        .bind(actor)
        .execute(self.pool())
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        Ok(())
    }

    /// Les relances sont suspendues pendant une fermeture (R-I6).
    pub(crate) async fn exiger_secretariat_ouvert(&self, etab: Uuid) -> Result<(), EntDbError> {
        if fermeture_en_cours(self.pool(), etab).await?.is_some() {
            return Err(conflit("secrétariat fermé : les relances sont suspendues pendant la fermeture"));
        }
        Ok(())
    }

    // -- Démarche familiale (fratrie native) ----------------------------------

    /// Une démarche pour un ou plusieurs enfants : une seule déclaration des responsables, un
    /// dossier par enfant, une référence par dossier. Remplace le dépôt d'un seul enfant (qui
    /// passe désormais par ici).
    pub async fn deposer_demarche(
        &self,
        etab: Uuid,
        jeton: &str,
        b: &DemarcheBody,
    ) -> Result<DemarcheDto, EntDbError> {
        let v = self.resoudre_jeton_preinscription(etab, jeton).await?;
        let n = b.enfants.len();
        if n == 0 || n > ENFANTS_MAX {
            return Err(invalide("une démarche porte sur un à six enfants"));
        }
        if b.autres_responsables.len() + 2 > RESPONSABLES_MAX {
            return Err(invalide("quatre responsables supplémentaires au plus"));
        }
        let mut eleves = Vec::with_capacity(n);
        for e in &b.enfants {
            eleves.push(inscriptions::normaliser_eleve(&e.eleve)?);
        }
        let mut r1 = b.responsable.clone();
        r1.email = Some(v.email.clone());
        r1.meme_foyer = true;
        let r1 = inscriptions::normaliser_responsable(&r1)?;
        let mut autres = Vec::new();
        let mut vus = vec![v.email.clone()];
        let mut plus: Vec<(&ResponsableFormulaireDto, Option<&Vec<usize>>)> = Vec::new();
        if let Some(s) = &b.second_parent {
            plus.push((s, None));
        }
        for a in &b.autres_responsables {
            plus.push((&a.responsable, Some(&a.enfants)));
        }
        for (r, enfants) in plus {
            let r = inscriptions::normaliser_responsable(r)?;
            if let Some(e) = r.email.as_deref() {
                if vus.iter().any(|x| x == e) {
                    return Err(invalide("chaque responsable a sa propre adresse e-mail"));
                }
                vus.push(e.to_string());
            }
            if let Some(es) = enfants {
                if es.iter().any(|i| *i >= n) {
                    return Err(invalide("enfant concerné inconnu dans la démarche"));
                }
            }
            autres.push((r, enfants.cloned().unwrap_or_default()));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let c: Option<(String, String, Option<NaiveDate>, Option<NaiveDate>, Option<NaiveDate>, bool)> =
            sqlx::query_as(
                "SELECT nature, statut, date_ouverture, date_cloture, date_cloture_depot, au_fil_de_l_eau
                   FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2 FOR SHARE",
            )
            .bind(etab)
            .bind(b.campagne_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        let Some((nature, statut, ouv, clo, depot, fil)) = c else {
            return Err(introuvable());
        };
        let today = Utc::now().date_naive();
        let limite = if fil { None } else { depot.or(clo) };
        if !matches!(nature.as_str(), "admission" | "en_cours_annee")
            || statut != "ouverte"
            || ouv.is_some_and(|o| o > today)
            || limite.is_some_and(|c| c < today)
        {
            return Err(conflit("campagne d'admission non ouverte"));
        }
        let niveaux: Vec<(String,)> = sqlx::query_as(
            "SELECT code FROM ent_inscription_campagne_niveau WHERE etablissement_id = $1 AND campagne_id = $2",
        )
        .bind(etab)
        .bind(b.campagne_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let (deja,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_inscription_dossier
              WHERE etablissement_id = $1 AND campagne_id = $2 AND deposant_email = $3",
        )
        .bind(etab)
        .bind(b.campagne_id)
        .bind(&v.email)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja + i64::try_from(n).unwrap_or(i64::MAX) > inscriptions::DOSSIERS_MAX_PAR_DEPOSANT {
            return Err(conflit("nombre maximal de dossiers atteint pour cette campagne"));
        }
        let reutilise = b.utiliser_mes_infos && v.user_id.is_some();
        let demarche_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_inscription_demarche
                (id, etablissement_id, deposant_email, deposant_user_id, deposant_role, reutilise_identite, canal)
             VALUES ($1,$2,$3,$4,'responsable',$5,'en_ligne')",
        )
        .bind(demarche_id)
        .bind(etab)
        .bind(&v.email)
        .bind(v.user_id)
        .bind(reutilise)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let coffre = ChampsSensibles::depuis_env();
        let mut dossiers = Vec::with_capacity(n);
        for (i, (enfant, eleve)) in b.enfants.iter().zip(eleves.iter()).enumerate() {
            // Vœux : offres de cette campagne, ouvertes aux candidatures, sans doublon.
            let mut voeux: Vec<(Uuid, String, String, Option<String>)> = Vec::new();
            if enfant.voeux.len() > 5 {
                return Err(invalide("cinq vœux au plus par enfant"));
            }
            for o in &enfant.voeux {
                if voeux.iter().any(|x| x.0 == *o) {
                    return Err(invalide("un vœu en double"));
                }
                let r: Option<(Uuid, String, String, Option<String>)> = sqlx::query_as(
                    "SELECT id, niveau_code, parcours_code, regime FROM ent_inscription_offre
                      WHERE etablissement_id = $1 AND campagne_id = $2 AND id = $3
                        AND archivee_le IS NULL AND ouverte_candidature",
                )
                .bind(etab)
                .bind(b.campagne_id)
                .bind(o)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                voeux.push(r.ok_or_else(|| invalide("vœu : offre inconnue ou fermée aux candidatures"))?);
            }
            let (niveau, parcours, regime) = match voeux.first() {
                Some((_, niveau, parcours, regime)) => {
                    if eleve
                        .niveau_demande
                        .as_deref()
                        .is_some_and(|x| !x.eq_ignore_ascii_case(niveau))
                    {
                        return Err(invalide("le niveau demandé ne correspond pas au premier vœu"));
                    }
                    (
                        Some(niveau.clone()),
                        (!parcours.is_empty()).then(|| parcours.clone()),
                        regime.clone().or_else(|| eleve.regime_demande.clone()),
                    )
                }
                None => {
                    if !niveaux.is_empty()
                        && !eleve
                            .niveau_demande
                            .as_ref()
                            .is_some_and(|nv| niveaux.iter().any(|(c,)| c == nv))
                    {
                        return Err(invalide("niveau demandé non proposé par cette campagne"));
                    }
                    (eleve.niveau_demande.clone(), None, eleve.regime_demande.clone())
                }
            };
            let dossier_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_inscription_dossier
                    (id, etablissement_id, campagne_id, nature, etat, eleve_nom, eleve_prenom,
                     eleve_date_naissance, eleve_sexe, eleve_ine, niveau_demande, regime_demande,
                     parcours_demande, etablissement_origine, deposant_email, deposant_user_id,
                     reutilise_identite, depose_le, demarche_id)
                 VALUES ($1,$2,$3,$4,'deposee',$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16, now(), $17)",
            )
            .bind(dossier_id)
            .bind(etab)
            .bind(b.campagne_id)
            .bind(&nature)
            .bind(&eleve.nom)
            .bind(&eleve.prenom)
            .bind(eleve.date_naissance)
            .bind(&eleve.sexe)
            .bind(&eleve.ine)
            .bind(&niveau)
            .bind(&regime)
            .bind(&parcours)
            .bind(&eleve.etablissement_origine)
            .bind(&v.email)
            .bind(v.user_id)
            .bind(reutilise)
            .bind(demarche_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if voeux.is_empty() && niveau.is_some() {
                // Sans vœu explicite : le niveau demandé vaut vœu n°1 quand la campagne propose une
                // offre sans parcours pour ce niveau.
                let o: Option<Uuid> = sqlx::query_scalar(
                    "SELECT id FROM ent_inscription_offre
                      WHERE etablissement_id = $1 AND campagne_id = $2 AND upper(niveau_code) = upper($3)
                        AND parcours_code = '' AND regime IS NULL AND archivee_le IS NULL AND ouverte_candidature",
                )
                .bind(etab)
                .bind(b.campagne_id)
                .bind(niveau.as_deref())
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                if let Some(o) = o {
                    voeux.push((o, String::new(), String::new(), None));
                }
            }
            for (rang, (offre, ..)) in voeux.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO ent_inscription_voeu (id, etablissement_id, dossier_id, rang, offre_id)
                     VALUES ($1,$2,$3,$4,$5)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(dossier_id)
                .bind(i16::try_from(rang + 1).unwrap_or(1))
                .bind(offre)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            // Responsables du dossier (projection lue par la validation) ; la déclaration de la
            // démarche est tenue par le déclencheur de la migration 305.
            let mut rangs: Vec<(i16, &ResponsableFormulaireDto)> = vec![(1, &r1)];
            for (k, (r, enfants)) in autres.iter().enumerate() {
                if enfants.is_empty() || enfants.contains(&i) {
                    rangs.push((i16::try_from(k + 2).unwrap_or(2), r));
                }
            }
            for (rang, r) in rangs {
                Self::inserer_responsable_tx(&mut tx, etab, dossier_id, rang, r, &coffre).await?;
            }
            Self::copier_pieces_tx(&mut tx, etab, b.campagne_id, dossier_id).await?;
            suivi_tx(
                &mut tx,
                etab,
                dossier_id,
                "statut",
                Some(("brouillon", "deposee")),
                Some("Préinscription déposée"),
                true,
                None,
            )
            .await?;
            publier(
                &mut tx,
                etab,
                types::ADMISSION_DEPOSEE,
                AggregateKind::Admission,
                dossier_id,
                None,
                json!({
                    "dossier_id": dossier_id, "campagne_id": b.campagne_id, "demarche_id": demarche_id,
                    "second_parent": b.second_parent.is_some(), "nombre_enfants": n,
                }),
                v.user_id,
            )
            .await?;
            dossiers.push(dossier_id);
        }
        if let Some(u) = v.user_id {
            if reutilise {
                Self::journal_identite_tx(&mut tx, u, "informations_reutilisees", etab).await?;
            }
            if b.enregistrer_profil {
                Self::enregistrer_profil_tx(&mut tx, u, &r1).await?;
                Self::journal_identite_tx(&mut tx, u, "profil_enregistre", etab).await?;
            }
        }
        inscriptions::commit(tx).await?;
        self.demarche_dto(etab, demarche_id).await
    }

    /// La démarche d'une famille (dossiers, déclarations) : seul le déposant vérifié la lit.
    pub async fn demarche_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
    ) -> Result<DemarcheDto, EntDbError> {
        let v = self.resoudre_jeton_preinscription(etab, jeton).await?;
        let ok: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_inscription_demarche WHERE etablissement_id = $1 AND id = $2 AND deposant_email = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(&v.email)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        ok.ok_or_else(introuvable)?;
        self.demarche_dto(etab, id).await
    }

    pub(crate) async fn demarche_dto(&self, etab: Uuid, id: Uuid) -> Result<DemarcheDto, EntDbError> {
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_inscription_dossier WHERE etablissement_id = $1 AND demarche_id = $2
              ORDER BY created_at, id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        if ids.is_empty() {
            return Err(introuvable());
        }
        let dossiers = self.resumes_par_ids(etab, &ids).await?;
        let declarations: Vec<(i16, bool, String, Option<String>, String, String, String, Vec<Uuid>, bool, Option<Uuid>)> =
            sqlx::query_as(
                "SELECT ordre, est_deposant, nom, prenom, lien_code, autorite_parentale, foyer_cle,
                        pour_enfants, ne_pas_solliciter, personne_id
                   FROM ent_inscription_declaration_responsable
                  WHERE etablissement_id = $1 AND demarche_id = $2 ORDER BY ordre",
            )
            .bind(etab)
            .bind(id)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        Ok(DemarcheDto {
            id,
            dossiers,
            declarations: declarations
                .into_iter()
                .map(|r| DeclarationResponsableDto {
                    ordre: r.0,
                    est_deposant: r.1,
                    nom: r.2,
                    prenom: r.3,
                    lien_code: r.4,
                    autorite_parentale: r.5,
                    foyer_cle: r.6,
                    pour_enfants: r.7,
                    ne_pas_solliciter: r.8,
                    personne_id: r.9,
                })
                .collect(),
        })
    }

    async fn resumes_par_ids(&self, etab: Uuid, ids: &[Uuid]) -> Result<Vec<DossierResumeDto>, EntDbError> {
        type Ligne = (Uuid, Uuid, String, String, String, String, Option<String>, Option<Uuid>, Option<DateTime<Utc>>, String, String, i64);
        let rows: Vec<Ligne> = sqlx::query_as(
            "SELECT d.id, d.campagne_id, d.nature, d.statut, d.eleve_nom, d.eleve_prenom, d.niveau_demande,
                    d.eleve_id, d.depose_le, d.etat, d.reference,
                    (SELECT count(*) FROM ent_inscription_piece p
                      WHERE p.dossier_id = d.id AND p.obligatoire AND p.statut <> 'acceptee')
               FROM ent_inscription_dossier d
              WHERE d.etablissement_id = $1 AND d.id = ANY($2)
              ORDER BY d.created_at, d.id",
        )
        .bind(etab)
        .bind(ids)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| DossierResumeDto {
                id: r.0,
                campagne_id: r.1,
                nature: r.2,
                statut: r.3,
                eleve_nom: r.4,
                eleve_prenom: r.5,
                niveau_demande: r.6,
                eleve_id: r.7,
                depose_le: r.8,
                etat: r.9,
                reference: r.10,
                pieces_manquantes: r.11,
            })
            .collect())
    }

    async fn enregistrer_profil_tx(
        conn: &mut PgConnection,
        user: Uuid,
        r1: &ResponsableFormulaireDto,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "INSERT INTO ent_profil_personnel (user_id, nom, prenom, civilite, telephone, adresse, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6, now())
             ON CONFLICT (user_id) DO UPDATE SET nom = EXCLUDED.nom, prenom = EXCLUDED.prenom,
                civilite = EXCLUDED.civilite, telephone = EXCLUDED.telephone,
                adresse = EXCLUDED.adresse, updated_at = now()",
        )
        .bind(user)
        .bind(&r1.nom)
        .bind(&r1.prenom)
        .bind(&r1.civilite)
        .bind(&r1.telephone)
        .bind(&r1.adresse)
        .execute(conn)
        .await
        .map_err(db)?;
        Ok(())
    }

    /// Ligne de responsable d'un dossier. Une adresse à protéger n'est **jamais** en clair (colonne
    /// `adresse` vide, valeur scellée) ; la référence de « ne pas solliciter » est scellée.
    async fn inserer_responsable_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        dossier_id: Uuid,
        rang: i16,
        r: &ResponsableFormulaireDto,
        coffre: &ChampsSensibles,
    ) -> Result<(), EntDbError> {
        let resp_id = Uuid::new_v4();
        let reserve = r.reserve.as_ref().map(inscriptions::reserve_depuis_dto).unwrap_or_default();
        let (adresse, adresse_protegee) = if reserve.adresse_a_proteger {
            (
                None,
                coffre
                    .sceller_opt("ent_inscription_responsable", "adresse_protegee", resp_id, r.adresse.as_deref())
                    .map_err(EntDbError::Domain)?,
            )
        } else {
            (r.adresse.clone(), None)
        };
        let reference = coffre
            .sceller_opt(
                "ent_inscription_responsable",
                "ne_pas_solliciter_ref",
                resp_id,
                reserve.ne_pas_solliciter_ref.as_deref(),
            )
            .map_err(EntDbError::Domain)?;
        sqlx::query(
            "INSERT INTO ent_inscription_responsable
                (id, etablissement_id, dossier_id, rang, nom, prenom, civilite, email, telephone,
                 adresse, lien, meme_foyer, autorite_parentale, ne_pas_solliciter,
                 ne_pas_solliciter_ref, adresse_a_proteger, adresse_protegee, langue,
                 besoin_interprete, organisme, nature_code)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21)",
        )
        .bind(resp_id)
        .bind(etab)
        .bind(dossier_id)
        .bind(rang)
        .bind(&r.nom)
        .bind(&r.prenom)
        .bind(&r.civilite)
        .bind(&r.email)
        .bind(&r.telephone)
        .bind(adresse)
        .bind(r.lien.as_deref().unwrap_or("autre"))
        .bind(r.meme_foyer)
        .bind(r.autorite_parentale)
        .bind(reserve.ne_pas_solliciter)
        .bind(reference)
        .bind(reserve.adresse_a_proteger)
        .bind(adresse_protegee)
        .bind(&reserve.langue)
        .bind(reserve.besoin_interprete)
        .bind(&reserve.organisme)
        .bind(&reserve.nature_code)
        .execute(conn)
        .await
        .map_err(db)?;
        Ok(())
    }
}

fn verifier_retour(d: &DossierRow, vers: &str) -> Result<(), EntDbError> {
    inscriptions_etats::verifier_transition(&d.nature, &d.etat, vers, Cause::AnnulationValidation)
        .map_err(conflit)
}

/// Valeur JSON des références de lecture (utilisée par les tests).
#[allow(dead_code)]
pub(crate) fn _json(_: Value) {}
