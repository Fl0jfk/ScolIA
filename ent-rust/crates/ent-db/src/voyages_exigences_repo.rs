//! Socle 42, lot V1 : exigences par participant, campagne d'autorisations, guichet, attestations de reprise,
//! désaccords, dérogations, « qui manque quoi » et relances (`docs/socles/42_SORTIES_VOYAGES.md` §9.3, §11.2,
//! §19.2, §19.5 ; VOY-139, VOY-149, VOY-171 à VOY-173, VOY-260, VOY-277 ; CV-04 à CV-09, CV-25, CV-26).
//!
//! * **port vers le socle 18** (codé en parallèle, aucune dépendance de code) : la campagne se **demande** par
//!   un fait sortant `voyages.campagne_ouverte` ; les décisions **reviennent** par la commande d'entrée
//!   [`PgEnt::voyages_autorisation_decision_recue`] (service, jamais exposée en HTTP) ; guichet et
//!   attestation écrivent les mêmes faits, avec leur source ;
//! * **qui signe** : un titulaire avec lien actif **et** autorité parentale (l'autre est présumé d'accord), ou
//!   l'élève majeur lui-même ; recalculé à chaque lecture : un titulaire qui perd l'autorité rend sa signature
//!   « à revoir » (CV-06) ;
//! * **faits en ajout seul** ; l'état de chaque exigence et celui du participant (`inscrit`, `pret`, `bloque`)
//!   sont **recalculés par le serveur** à chaque fait ; seuls ces trois états sont gérés, jamais `parti` ni
//!   `desiste` ;
//! * **désaccord** (CV-04) : déclaré par la gestion, il bloque jusqu'à la décision **motivée** de la direction ;
//!   il n'est jamais montré à l'organisateur ni notifié à un titulaire ;
//! * **dérogation** : direction seule, motivée, sur les exigences **non légales** (soins, image) ; jamais sur un
//!   refus, jamais sur la participation ni le désaccord ; elle ne vaut **jamais** consentement à l'image ;
//! * **« qui manque quoi »** : l'organisateur voit des **états**, jamais un motif de dérogation ni une cause
//!   de désaccord (R-SV18, §21.4) ;
//! * **relances** J-21, J-10, J-5 (une par seuil, rattrapage sans rafale) et relance groupée manuelle : les
//!   destinataires sont **calculés par le serveur** (titulaires sollicitables : une interdiction de
//!   communication exclut), jamais envoyés à un titulaire qui a refusé ; textes fixes, aucun nom d'élève.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use ent_object::upload::UsageFichier;
use ent_object::ObjectStore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use crate::voyages_exigences::{
    avec_derogation, date_signature_plausible, est_derogeable, est_signable, etat_participant, etat_signature,
    requises, seuils_dus, Decision, DecisionSignataire, AUTORISATION, DESACCORD,
};
use crate::voyages_prestataires::ligne_propre;
use crate::voyages_repo::{
    charger_avec_acces, interdit, journaliser, publier, refus, texte, tx, Acces, Fait, SortieLigne,
};

const ETATS_CAMPAGNE: [&str; 3] = ["ouverte", "prete", "en_cours"];
const GUICHET_MASSE_MAX: usize = 200;

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CampagneSaisie {
    #[serde(default)]
    pub avec_image: bool,
    #[serde(default)]
    pub avec_soins: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CampagneVue {
    pub avec_image: bool,
    pub avec_soins: bool,
    pub statut: String,
    pub ouverte_le: DateTime<Utc>,
    pub reference_externe: Option<String>,
}

/// Décision reçue du socle 18 (port d'entrée). Le service revérifie tout : sortie, participant, campagne,
/// qualité du signataire à la date de la signature.
#[derive(Debug, Clone)]
pub struct DecisionRecue {
    pub sortie_id: Uuid,
    pub participant_id: Uuid,
    pub code: String,
    pub decision: String,
    pub signataire_personne_id: Uuid,
    pub signee_le: NaiveDate,
    pub reference_externe: Option<String>,
}

/// Scan d'un coupon papier : passe par la politique de dépôt commune (type détecté sur les octets).
#[derive(Debug, Clone)]
pub struct ScanSaisie {
    pub nom: String,
    pub mime: Option<String>,
    pub contenu: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct GuichetSaisie {
    pub participant_id: Uuid,
    pub code: String,
    /// `accepte`, `refuse` ou `retire`.
    pub decision: String,
    pub signataire_personne_id: Uuid,
    /// Date portée par le coupon papier (jamais la date de saisie).
    pub signee_le: NaiveDate,
    pub scan: Option<ScanSaisie>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttestationLigne {
    pub participant_id: Uuid,
    pub signataire_personne_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttestationSaisie {
    pub lignes: Vec<AttestationLigne>,
    pub signee_le: NaiveDate,
    #[serde(default)]
    pub confirme: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CelluleVue {
    pub code: String,
    pub etat: String,
    pub bloquante: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LigneMatrice {
    pub participant_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub etat: String,
    pub cellules: Vec<CelluleVue>,
    /// Au moins une exigence bloquante n'est pas satisfaite.
    pub manque: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Matrice {
    pub campagne: Option<CampagneVue>,
    pub codes: Vec<String>,
    pub lignes: Vec<LigneMatrice>,
    pub manquants: i64,
    pub bloques: i64,
    pub prets: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApercuRelance {
    /// Comptes à notifier (titulaires sollicitables avec un compte, ou élèves majeurs).
    pub destinataires_comptes: i64,
    /// Titulaires sans compte : à relancer par courrier (le courrier réel attend le socle 8, lot N6).
    pub destinataires_sans_compte: i64,
    pub participants_concernes: i64,
}

// ───────────────────────── Aides ─────────────────────────

#[derive(FromRow)]
struct CampagneLigne {
    avec_image: bool,
    avec_soins: bool,
    statut: String,
}

async fn campagne_de(conn: &mut PgConnection, etab: Uuid, sortie: Uuid) -> Result<Option<CampagneLigne>, EntDbError> {
    sqlx::query_as("SELECT avec_image, avec_soins, statut FROM ent_sortie_campagne WHERE etablissement_id = $1 AND sortie_id = $2")
        .bind(etab)
        .bind(sortie)
        .fetch_optional(conn)
        .await
        .map_err(db)
}

fn garde_participants(acces: &Acces) -> Result<(), EntDbError> {
    if acces.gere(Action::VoyagesParticipants) {
        Ok(())
    } else if acces.detail() {
        Err(interdit())
    } else {
        Err(introuvable())
    }
}

async fn aujourdhui(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_aujourdhui_etab($1)").bind(etab).fetch_one(conn).await.map_err(db)
}

/// Élève d'un participant et majorité à la date de l'établissement.
async fn eleve_du_participant(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    participant: Uuid,
) -> Result<(Uuid, bool), EntDbError> {
    sqlx::query_as(
        "SELECT sp.eleve_id, COALESCE(e.date_naissance + INTERVAL '18 years' <= ent_aujourdhui_etab($1), false)
           FROM ent_sortie_participant sp JOIN ent_eleve e ON e.etablissement_id = sp.etablissement_id AND e.id = sp.eleve_id
          WHERE sp.etablissement_id = $1 AND sp.sortie_id = $2 AND sp.id = $3",
    )
    .bind(etab)
    .bind(sortie)
    .bind(participant)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

/// Personnes qui peuvent signer pour l'élève aujourd'hui : l'élève lui-même s'il est majeur, sinon les titulaires
/// avec lien actif et autorité parentale.
async fn signataires_qualifies(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    majeur: bool,
) -> Result<HashSet<Uuid>, EntDbError> {
    let v: Vec<Uuid> = if majeur {
        sqlx::query_scalar("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND eleve_id = $2 AND fusionnee_dans IS NULL")
            .bind(etab)
            .bind(eleve)
            .fetch_all(conn)
            .await
            .map_err(db)?
    } else {
        sqlx::query_scalar(
            "SELECT personne_id FROM ent_eleve_lien_actif WHERE etablissement_id = $1 AND eleve_id = $2 AND autorite_parentale",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_all(conn)
        .await
        .map_err(db)?
    };
    Ok(v.into_iter().collect())
}

/// Crée les exigences manquantes des participants qui partent encore (participants ajoutés après l'ouverture).
async fn synchroniser(conn: &mut PgConnection, etab: Uuid, s: &SortieLigne, c: &CampagneLigne) -> Result<(), EntDbError> {
    for (code, bloquante) in requises(&s.nature, c.avec_image, c.avec_soins) {
        sqlx::query(
            "INSERT INTO ent_sortie_exigence (id, etablissement_id, sortie_id, participant_id, code, bloquante)
             SELECT gen_random_uuid(), $1, $2, sp.id, $3, $4
               FROM ent_sortie_participant sp
              WHERE sp.etablissement_id = $1 AND sp.sortie_id = $2
                AND sp.etat NOT IN ('propose', 'liste_attente', 'desiste', 'retire', 'exclu_disciplinaire')
             ON CONFLICT (participant_id, code) DO NOTHING",
        )
        .bind(etab)
        .bind(s.id)
        .bind(code)
        .bind(bloquante)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

#[derive(FromRow)]
struct ExigenceLigne {
    id: Uuid,
    code: String,
    bloquante: bool,
}

/// Recalcule les exigences d'un participant puis son état (`inscrit`, `pret`, `bloque`). Idempotent.
async fn recalculer(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Option<Uuid>,
    s: &SortieLigne,
    participant: Uuid,
) -> Result<(), EntDbError> {
    let (eleve, majeur) = eleve_du_participant(conn, etab, s.id, participant).await?;
    let qualifies = signataires_qualifies(conn, etab, eleve, majeur).await?;
    let regle_tous = s.destination == "etranger" && !majeur;
    let exigences: Vec<ExigenceLigne> = sqlx::query_as(
        "SELECT id, code, bloquante FROM ent_sortie_exigence WHERE etablissement_id = $1 AND participant_id = $2",
    )
    .bind(etab)
    .bind(participant)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut etats: Vec<(String, bool, String)> = Vec::new();
    for x in exigences {
        let etat: String = if x.code == DESACCORD {
            // Déclaré puis levé par une décision POSTÉRIEURE de la direction ; sinon il bloque.
            let (declare, decide): (Option<DateTime<Utc>>, Option<DateTime<Utc>>) = sqlx::query_as(
                "SELECT max(cree_le) FILTER (WHERE type = 'declaration_desaccord'),
                        max(cree_le) FILTER (WHERE type = 'decision_desaccord')
                   FROM ent_sortie_exigence_fait WHERE etablissement_id = $1 AND exigence_id = $2",
            )
            .bind(etab)
            .bind(x.id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            match (declare, decide) {
                (Some(d), Some(l)) if l >= d => "levee".into(),
                (Some(_), _) => "attendue".into(),
                _ => "levee".into(),
            }
        } else {
            let dernieres: Vec<(Uuid, String)> = sqlx::query_as(
                "SELECT DISTINCT ON (signataire_personne_id) signataire_personne_id, decision
                   FROM ent_sortie_exigence_fait
                  WHERE etablissement_id = $1 AND exigence_id = $2 AND type IN ('formulaire', 'guichet', 'attestation_reprise')
                  ORDER BY signataire_personne_id, cree_le DESC, id DESC",
            )
            .bind(etab)
            .bind(x.id)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
            let decisions: Vec<DecisionSignataire> = dernieres
                .iter()
                .filter_map(|(p, d)| {
                    Decision::parse(d).map(|decision| DecisionSignataire { signataire: *p, qualifie: qualifies.contains(p), decision })
                })
                .collect();
            let signature = etat_signature(&decisions, qualifies.len(), regle_tous);
            let derogation: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_sortie_exigence_fait WHERE etablissement_id = $1 AND exigence_id = $2 AND type = 'derogation')",
            )
            .bind(etab)
            .bind(x.id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            avec_derogation(signature, &x.code, derogation).to_string()
        };
        // Dernière acceptation encore valable : source, signataire et date (affichage de la gestion).
        let derniere: Option<(String, Uuid, NaiveDate)> = sqlx::query_as(
            "SELECT type, signataire_personne_id, signee_le FROM ent_sortie_exigence_fait
              WHERE etablissement_id = $1 AND exigence_id = $2 AND decision = 'accepte' ORDER BY cree_le DESC, id DESC LIMIT 1",
        )
        .bind(etab)
        .bind(x.id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let source = match etat.as_str() {
            "derogee" => Some("derogation".to_string()),
            "levee" => Some("decision_direction".to_string()),
            "remplie" | "a_revoir" => derniere.as_ref().map(|d| d.0.clone()),
            _ => None,
        };
        sqlx::query(
            "UPDATE ent_sortie_exigence
                SET etat = $3, source = $4, signataire_personne_id = $5, signee_le = $6, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(x.id)
        .bind(&etat)
        .bind(&source)
        .bind(derniere.as_ref().map(|d| d.1))
        .bind(derniere.as_ref().map(|d| d.2))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        etats.push((x.code, x.bloquante, etat));
    }
    let actuel: String = sqlx::query_scalar("SELECT etat FROM ent_sortie_participant WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(participant)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    let vues: Vec<(&str, bool, &str)> = etats.iter().map(|(c, b, e)| (c.as_str(), *b, e.as_str())).collect();
    if let Some(nouveau) = etat_participant(&actuel, &vues) {
        if nouveau != actuel {
            sqlx::query("UPDATE ent_sortie_participant SET etat = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(participant)
                .bind(nouveau)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
            sqlx::query(
                "INSERT INTO ent_sortie_participant_fait (id, etablissement_id, participant_id, acteur_user_id, type, etat_avant, etat_apres)
                 VALUES ($1,$2,$3,$4,'etat',$5,$6)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(participant)
            .bind(user)
            .bind(&actuel)
            .bind(nouveau)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
    }
    Ok(())
}

async fn recalculer_tous(conn: &mut PgConnection, etab: Uuid, user: Option<Uuid>, s: &SortieLigne) -> Result<(), EntDbError> {
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT participant_id FROM ent_sortie_exigence WHERE etablissement_id = $1 AND sortie_id = $2",
    )
    .bind(etab)
    .bind(s.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for p in ids {
        recalculer(conn, etab, user, s, p).await?;
    }
    Ok(())
}

async fn exigence_de(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    participant: Uuid,
    code: &str,
) -> Result<(Uuid, bool), EntDbError> {
    sqlx::query_as(
        "SELECT id, bloquante FROM ent_sortie_exigence
          WHERE etablissement_id = $1 AND sortie_id = $2 AND participant_id = $3 AND code = $4 FOR UPDATE",
    )
    .bind(etab)
    .bind(sortie)
    .bind(participant)
    .bind(code)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

/// Écrit un fait de signature après avoir revérifié le signataire (qualifié **aujourd'hui**).
#[allow(clippy::too_many_arguments)] // un fait de signature porte ses douze faits : les regrouper n'apporterait rien de lisible
async fn ecrire_signature(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Option<Uuid>,
    s: &SortieLigne,
    participant: Uuid,
    code: &str,
    decision: &str,
    signataire: Uuid,
    signee_le: NaiveDate,
    type_: &str,
    reference: Option<&str>,
    scan: Option<(String, String, String, i32, String)>,
) -> Result<(), EntDbError> {
    if !est_signable(code) || Decision::parse(decision).is_none() {
        return Err(invalide());
    }
    let (eleve, majeur) = eleve_du_participant(conn, etab, s.id, participant).await?;
    let (exigence, _) = exigence_de(conn, etab, s.id, participant, code).await?;
    if !signataires_qualifies(conn, etab, eleve, majeur).await?.contains(&signataire) {
        return Err(refus(
            RefusMetier::Invalide,
            "Ce signataire n'est pas titulaire de l'autorité parentale (ou n'est pas l'élève majeur) : sa signature n'est pas recevable.",
        ));
    }
    let (cle, nom, mime, taille, sha) = match scan {
        Some((c, n, m, t, h)) => (Some(c), Some(n), Some(m), Some(t), Some(h)),
        None => (None, None, None, None, None),
    };
    sqlx::query(
        "INSERT INTO ent_sortie_exigence_fait
             (id, etablissement_id, exigence_id, type, decision, signataire_personne_id, signee_le, acteur_user_id,
              scan_cle_objet, scan_nom, scan_mime, scan_taille, scan_sha256, reference_externe)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(exigence)
    .bind(type_)
    .bind(decision)
    .bind(signataire)
    .bind(signee_le)
    .bind(user)
    .bind(cle)
    .bind(nom)
    .bind(mime)
    .bind(taille)
    .bind(sha)
    .bind(reference)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    recalculer(conn, etab, user, s, participant).await
}

// ───────────────────────── Campagne ─────────────────────────

impl PgEnt {
    /// VOY-171 : ouvre la campagne d'autorisations (participation, image, soins). Un seul formulaire, des
    /// consentements distincts ; la demande part au socle 18 par un fait, les décisions reviennent par le port.
    pub async fn voyage_campagne_ouvrir(&self, a: &Acteur<'_>, id: Uuid, saisie: CampagneSaisie) -> Result<Matrice, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_participants(&acces)?;
        if !ETATS_CAMPAGNE.contains(&s.etat.as_str()) {
            return Err(refus(RefusMetier::Conflit, "La campagne s'ouvre une fois la sortie autorisée."));
        }
        if requises(&s.nature, saisie.avec_image, saisie.avec_soins).is_empty() {
            return Err(refus(
                RefusMetier::Invalide,
                "Une sortie obligatoire est gratuite et ne demande aucune autorisation : rien à recueillir.",
            ));
        }
        if campagne_de(&mut t, a.etablissement_id, id).await?.is_some() {
            return Err(conflit());
        }
        sqlx::query(
            "INSERT INTO ent_sortie_campagne (id, etablissement_id, sortie_id, avec_image, avec_soins, ouverte_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(id)
        .bind(saisie.avec_image)
        .bind(saisie.avec_soins)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let c = campagne_de(&mut t, a.etablissement_id, id).await?.ok_or_else(introuvable)?;
        synchroniser(&mut t, a.etablissement_id, &s, &c).await?;
        recalculer_tous(&mut t, a.etablissement_id, Some(a.user_id), &s).await?;
        journaliser(&mut t, a, id, Fait::simple("campagne_ouverte")).await?;
        let ev = publier(
            &mut t,
            a,
            "voyages.campagne_ouverte",
            id,
            json!({ "image": saisie.avec_image, "soins": saisie.avec_soins, "participation": s.nature == "facultative" }),
        )
        .await?;
        let m = matrice_de(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(m)
    }

    /// Ferme la campagne (les faits restent ; plus aucun guichet ni décision entrante).
    pub async fn voyage_campagne_clore(&self, a: &Acteur<'_>, id: Uuid) -> Result<(), EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_participants(&acces)?;
        let n = sqlx::query(
            "UPDATE ent_sortie_campagne SET statut = 'close', close_le = now()
              WHERE etablissement_id = $1 AND sortie_id = $2 AND statut = 'ouverte'",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit());
        }
        journaliser(&mut t, a, id, Fait::simple("campagne_close")).await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// **Port d'entrée du socle 18** : une décision signée en ligne (S1). Service seulement : l'appelant est
    /// l'autre socle, dans son propre contrôle ; tout est revérifié ici (campagne ouverte, participant de la
    /// sortie, signataire qualifié, date plausible). Retourne l'état de l'exigence après recalcul.
    pub async fn voyages_autorisation_decision_recue(&self, etab: Uuid, d: DecisionRecue) -> Result<String, EntDbError> {
        let mut t = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s: SortieLigne = sqlx::query_as("SELECT * FROM ent_sortie WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
            .bind(etab)
            .bind(d.sortie_id)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        let c = campagne_de(&mut t, etab, s.id).await?.ok_or_else(introuvable)?;
        if c.statut != "ouverte" {
            return Err(refus(RefusMetier::Conflit, "La campagne est close."));
        }
        let jour = aujourdhui(&mut t, etab).await?;
        if !date_signature_plausible(d.signee_le, jour) {
            return Err(invalide());
        }
        synchroniser(&mut t, etab, &s, &c).await?;
        ecrire_signature(
            &mut t,
            etab,
            None,
            &s,
            d.participant_id,
            &d.code,
            &d.decision,
            d.signataire_personne_id,
            d.signee_le,
            "formulaire",
            d.reference_externe.as_deref(),
            None,
        )
        .await?;
        let etat: String = sqlx::query_scalar(
            "SELECT etat FROM ent_sortie_exigence WHERE etablissement_id = $1 AND participant_id = $2 AND code = $3",
        )
        .bind(etab)
        .bind(d.participant_id)
        .bind(&d.code)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(etat)
    }

    // ───────────────── Guichet, attestations ─────────────────

    /// VOY-173 : enregistre une signature **papier** au guichet : le signataire est un titulaire qualifié (ou
    /// l'élève majeur), la date est celle du coupon, le scan est facultatif (politique de dépôt commune).
    pub async fn voyage_autorisation_guichet(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        g: GuichetSaisie,
    ) -> Result<Matrice, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_participants(&acces)?;
        let c = campagne_de(&mut t, a.etablissement_id, id).await?.ok_or_else(|| {
            refus(RefusMetier::Conflit, "Aucune campagne d'autorisations n'est ouverte pour cette sortie.")
        })?;
        if c.statut != "ouverte" {
            return Err(refus(RefusMetier::Conflit, "La campagne est close."));
        }
        let jour = aujourdhui(&mut t, a.etablissement_id).await?;
        if !date_signature_plausible(g.signee_le, jour) {
            return Err(refus(RefusMetier::Invalide, "La date de la signature papier est invalide (future ou trop ancienne)."));
        }
        let scan = match &g.scan {
            None => None,
            Some(sc) => {
                let v = self
                    .politique_fichiers()
                    .valider(UsageFichier::Document, &sc.nom, sc.mime.as_deref(), &sc.contenu)
                    .map_err(|_| refus(RefusMetier::Invalide, "Scan refusé (type, taille ou contenu)."))?;
                let pid = Uuid::new_v4();
                let cle = ObjectStore::key_for(a.etablissement_id, "autorisation_papier", pid, v.extension);
                let info = self.objet_put(&cle, &sc.contenu).await?;
                crate::objets::enregistrer_objet(&mut *t, a.etablissement_id, &info, Some(v.mime)).await?;
                Some((cle, ligne_propre(&v.nom, 200), v.mime.to_string(), sc.contenu.len() as i32, info.sha256))
            }
        };
        synchroniser(&mut t, a.etablissement_id, &s, &c).await?;
        ecrire_signature(
            &mut t,
            a.etablissement_id,
            Some(a.user_id),
            &s,
            g.participant_id,
            &g.code,
            &g.decision,
            g.signataire_personne_id,
            g.signee_le,
            "guichet",
            None,
            scan,
        )
        .await?;
        journaliser(&mut t, a, id, Fait { champ: Some(&g.code), ..Fait::simple("autorisation_guichet") }).await?;
        let m = matrice_de(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        Ok(m)
    }

    /// VOY-260 (R-SV39) : attestation **en masse**, avec confirmation, des autorisations papier d'un séjour
    /// repris de Next.js (qui n'a jamais recueilli d'autorisation). Source `attestation_reprise` ; réservée aux
    /// séjours issus de la reprise ; tout ou rien ; jamais présumée.
    pub async fn voyage_attestations_masse(&self, a: &Acteur<'_>, id: Uuid, att: AttestationSaisie) -> Result<Matrice, EntDbError> {
        if !att.confirme {
            return Err(refus(RefusMetier::Invalide, "Confirmez l'attestation après relecture de la liste."));
        }
        if att.lignes.is_empty() || att.lignes.len() > GUICHET_MASSE_MAX {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_participants(&acces)?;
        let reprise: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_reprise_sejour WHERE etablissement_id = $1 AND sortie_id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if !reprise {
            return Err(refus(RefusMetier::Conflit, "L'attestation en masse ne vaut que pour un séjour repris de Next.js."));
        }
        let c = campagne_de(&mut t, a.etablissement_id, id).await?.ok_or_else(|| {
            refus(RefusMetier::Conflit, "Ouvrez d'abord la campagne d'autorisations de cette sortie.")
        })?;
        let jour = aujourdhui(&mut t, a.etablissement_id).await?;
        if !date_signature_plausible(att.signee_le, jour) {
            return Err(refus(RefusMetier::Invalide, "La date de la signature papier est invalide."));
        }
        synchroniser(&mut t, a.etablissement_id, &s, &c).await?;
        for l in &att.lignes {
            ecrire_signature(
                &mut t,
                a.etablissement_id,
                Some(a.user_id),
                &s,
                l.participant_id,
                AUTORISATION,
                "accepte",
                l.signataire_personne_id,
                att.signee_le,
                "attestation_reprise",
                None,
                None,
            )
            .await?;
        }
        journaliser(&mut t, a, id, Fait { champ: Some("attestation_reprise"), apres: Some(att.lignes.len().to_string()), ..Fait::simple("autorisations_attestees") })
            .await?;
        let m = matrice_de(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        Ok(m)
    }

    // ───────────────── Désaccord et dérogation (direction) ─────────────────

    /// CV-04 : un désaccord est déclaré (porté à la connaissance de l'établissement) : le participant est
    /// **bloqué** jusqu'à la décision motivée de la direction. Aucun titulaire n'est informé ; l'organisateur
    /// ne voit pas la cause.
    pub async fn voyage_desaccord_declarer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        participant: Uuid,
        titulaire_personne_id: Uuid,
    ) -> Result<(), EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_participants(&acces)?;
        let c = campagne_de(&mut t, a.etablissement_id, id).await?.ok_or_else(|| {
            refus(RefusMetier::Conflit, "Ouvrez d'abord la campagne d'autorisations de cette sortie.")
        })?;
        synchroniser(&mut t, a.etablissement_id, &s, &c).await?;
        let (eleve, majeur) = eleve_du_participant(&mut t, a.etablissement_id, id, participant).await?;
        if !signataires_qualifies(&mut t, a.etablissement_id, eleve, majeur).await?.contains(&titulaire_personne_id) {
            return Err(refus(RefusMetier::Invalide, "Le désaccord doit venir d'un titulaire de l'autorité parentale."));
        }
        sqlx::query(
            "INSERT INTO ent_sortie_exigence (id, etablissement_id, sortie_id, participant_id, code, bloquante)
             VALUES ($1,$2,$3,$4,$5,true) ON CONFLICT (participant_id, code) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(id)
        .bind(participant)
        .bind(DESACCORD)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let (exigence, _) = exigence_de(&mut t, a.etablissement_id, id, participant, DESACCORD).await?;
        sqlx::query(
            "INSERT INTO ent_sortie_exigence_fait (id, etablissement_id, exigence_id, type, signataire_personne_id, acteur_user_id)
             VALUES ($1,$2,$3,'declaration_desaccord',$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(exigence)
        .bind(titulaire_personne_id)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        recalculer(&mut t, a.etablissement_id, Some(a.user_id), &s, participant).await?;
        // Le journal de la sortie ne dit PAS qui est en désaccord, ni de qui il vient.
        journaliser(&mut t, a, id, Fait::simple("desaccord_declare")).await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// CV-04 : décision **motivée** de la direction (« accord présumé levé ») : seule la direction
    /// (`voyages.autoriser`). Elle ne vaut pas autorisation : l'autorisation du titulaire reste exigée.
    pub async fn voyage_desaccord_decider(&self, a: &Acteur<'_>, id: Uuid, participant: Uuid, motif: &str) -> Result<(), EntDbError> {
        let motif = texte(motif, 1000)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.a(Action::VoyagesAutoriser) {
            return if acces.detail() { Err(interdit()) } else { Err(introuvable()) };
        }
        let (exigence, _) = exigence_de(&mut t, a.etablissement_id, id, participant, DESACCORD).await?;
        let etat: String = sqlx::query_scalar("SELECT etat FROM ent_sortie_exigence WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(exigence)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        if etat != "attendue" {
            return Err(conflit());
        }
        sqlx::query(
            "INSERT INTO ent_sortie_exigence_fait (id, etablissement_id, exigence_id, type, acteur_user_id, motif)
             VALUES ($1,$2,$3,'decision_desaccord',$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(exigence)
        .bind(a.user_id)
        .bind(&motif)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        recalculer(&mut t, a.etablissement_id, Some(a.user_id), &s, participant).await?;
        journaliser(&mut t, a, id, Fait::simple("desaccord_decide")).await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// VOY-139 : dérogation **motivée** à une exigence **non légale** (soins, image) : direction seule. Jamais
    /// sur un refus, ni sur la participation d'une sortie facultative, ni sur un désaccord. Elle n'est **pas**
    /// un consentement : les publications restent filtrées par le droit à l'image du socle 2.
    pub async fn voyage_exigence_deroger(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        participant: Uuid,
        code: &str,
        motif: &str,
    ) -> Result<(), EntDbError> {
        let motif = texte(motif, 1000)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.a(Action::VoyagesAutoriser) {
            return if acces.detail() { Err(interdit()) } else { Err(introuvable()) };
        }
        if !est_derogeable(code) {
            return Err(refus(
                RefusMetier::Interdit,
                "Cette exigence est légale : la direction ne peut ni la lever ni y déroger.",
            ));
        }
        let (exigence, _) = exigence_de(&mut t, a.etablissement_id, id, participant, code).await?;
        let etat: String = sqlx::query_scalar("SELECT etat FROM ent_sortie_exigence WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(exigence)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        if etat == "refusee" {
            return Err(refus(RefusMetier::Interdit, "Le refus d'un titulaire ne se déroge jamais."));
        }
        sqlx::query(
            "INSERT INTO ent_sortie_exigence_fait (id, etablissement_id, exigence_id, type, acteur_user_id, motif)
             VALUES ($1,$2,$3,'derogation',$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(exigence)
        .bind(a.user_id)
        .bind(&motif)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        recalculer(&mut t, a.etablissement_id, Some(a.user_id), &s, participant).await?;
        journaliser(&mut t, a, id, Fait { champ: Some(code), ..Fait::simple("exigence_derogee") }).await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────── « Qui manque quoi » et relances ─────────────────

    /// VOY-149 : la matrice « qui manque quoi » (participants en lignes, exigences en colonnes). L'organisateur
    /// voit des **états** ; la colonne « désaccord » est réservée à la direction ; aucun motif n'est montré.
    pub async fn voyage_exigences_matrice(&self, a: &Acteur<'_>, id: Uuid) -> Result<Matrice, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_participants(&acces)?;
        if let Some(c) = campagne_de(&mut t, a.etablissement_id, id).await? {
            synchroniser(&mut t, a.etablissement_id, &s, &c).await?;
            recalculer_tous(&mut t, a.etablissement_id, Some(a.user_id), &s).await?;
        }
        let m = matrice_de(&mut t, a, &s, &acces).await?;
        t.commit().await.map_err(db)?;
        Ok(m)
    }

    /// Aperçu de la relance groupée : combien de comptes recevraient l'avis (calcul serveur).
    pub async fn voyage_relance_apercu(&self, a: &Acteur<'_>, id: Uuid) -> Result<ApercuRelance, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        garde_participants(&acces)?;
        let (comptes, sans, concernes) = destinataires(&mut t, a.etablissement_id, id).await?;
        t.commit().await.map_err(db)?;
        Ok(ApercuRelance {
            destinataires_comptes: comptes.len() as i64,
            destinataires_sans_compte: sans.len() as i64,
            participants_concernes: concernes,
        })
    }

    /// VOY-149 : relance groupée des manquants, après aperçu et confirmation (N3). Au plus une relance manuelle
    /// par 12 heures et par sortie. Les destinataires sont calculés ici, jamais fournis.
    pub async fn voyage_relancer_manquants(&self, a: &Acteur<'_>, id: Uuid, confirme: bool) -> Result<ApercuRelance, EntDbError> {
        if !confirme {
            return Err(refus(RefusMetier::Invalide, "Confirmez la relance après avoir lu l'aperçu."));
        }
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        garde_participants(&acces)?;
        let c = campagne_de(&mut t, a.etablissement_id, id).await?.ok_or_else(|| {
            refus(RefusMetier::Conflit, "Aucune campagne d'autorisations n'est ouverte pour cette sortie.")
        })?;
        if c.statut != "ouverte" {
            return Err(refus(RefusMetier::Conflit, "La campagne est close."));
        }
        let recente: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_relance
                             WHERE etablissement_id = $1 AND sortie_id = $2 AND seuil = 'manuelle' AND cree_le > now() - interval '12 hours')",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if recente {
            return Err(refus(RefusMetier::Conflit, "Une relance a déjà été envoyée il y a moins de douze heures."));
        }
        synchroniser(&mut t, a.etablissement_id, &s, &c).await?;
        recalculer_tous(&mut t, a.etablissement_id, Some(a.user_id), &s).await?;
        let r = relancer(&mut t, a.etablissement_id, &s, "manuelle", false, Some(a.user_id)).await?;
        journaliser(&mut t, a, id, Fait { champ: Some("relance"), apres: Some(r.destinataires_comptes.to_string()), ..Fait::simple("relance_autorisations") })
            .await?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }

    /// VOY-172 : relances automatiques J-21, J-10, J-5 pour toutes les sorties de l'établissement dont la
    /// campagne est ouverte. Une relance par seuil et par sortie (idempotent), rattrapage sans rafale. À appeler
    /// une fois par jour (aucun ordonnanceur branché ; la direction peut la déclencher). Retourne le nombre de
    /// comptes notifiés.
    pub async fn voyages_relances_traiter(&self, etab: Uuid) -> Result<usize, EntDbError> {
        let mut t = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let sorties: Vec<(SortieLigne, i64)> = {
            let ids: Vec<(Uuid, i64)> = sqlx::query_as(
                "SELECT s.id, ((s.debut AT TIME ZONE 'Europe/Paris')::date - ent_aujourdhui_etab($1))::bigint
                   FROM ent_sortie s JOIN ent_sortie_campagne c ON c.etablissement_id = s.etablissement_id AND c.sortie_id = s.id
                  WHERE s.etablissement_id = $1 AND c.statut = 'ouverte' AND s.etat IN ('ouverte', 'prete')
                  ORDER BY s.debut, s.id FOR UPDATE OF s",
            )
            .bind(etab)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
            let mut v = Vec::new();
            for (sid, jours) in ids {
                let s: SortieLigne = sqlx::query_as("SELECT * FROM ent_sortie WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(sid)
                    .fetch_one(&mut *t)
                    .await
                    .map_err(db)?;
                v.push((s, jours));
            }
            v
        };
        let mut notifies = 0usize;
        for (s, jours) in sorties {
            let deja: Vec<String> = sqlx::query_scalar(
                "SELECT seuil FROM ent_sortie_relance WHERE etablissement_id = $1 AND sortie_id = $2 AND seuil <> 'manuelle'",
            )
            .bind(etab)
            .bind(s.id)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
            let deja: Vec<&str> = deja.iter().map(String::as_str).collect();
            let dus = seuils_dus(jours, &deja);
            if dus.is_empty() {
                continue;
            }
            let c = campagne_de(&mut t, etab, s.id).await?.ok_or_else(introuvable)?;
            synchroniser(&mut t, etab, &s, &c).await?;
            recalculer_tous(&mut t, etab, None, &s).await?;
            for (seuil, omise) in dus {
                let r = relancer(&mut t, etab, &s, seuil, omise, None).await?;
                if !omise {
                    notifies += r.destinataires_comptes as usize;
                }
            }
        }
        t.commit().await.map_err(db)?;
        Ok(notifies)
    }
}

// ───────────────────────── Matrice, destinataires, relance ─────────────────────────

async fn matrice_de(conn: &mut PgConnection, a: &Acteur<'_>, s: &SortieLigne, acces: &Acces) -> Result<Matrice, EntDbError> {
    let direction = acces.a(Action::VoyagesAutoriser);
    let campagne: Option<CampagneVue> = sqlx::query_as(
        "SELECT avec_image, avec_soins, statut, ouverte_le, reference_externe FROM ent_sortie_campagne
          WHERE etablissement_id = $1 AND sortie_id = $2",
    )
    .bind(a.etablissement_id)
    .bind(s.id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let participants = crate::voyages_repo::liste_participants(conn, a, s, &["inscrit", "pret", "bloque", "parti"]).await?;
    let exigences: Vec<(Uuid, String, String, bool)> = sqlx::query_as(
        "SELECT participant_id, code, etat, bloquante FROM ent_sortie_exigence WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY code",
    )
    .bind(a.etablissement_id)
    .bind(s.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut par_participant: HashMap<Uuid, Vec<CelluleVue>> = HashMap::new();
    for (p, code, etat, bloquante) in exigences {
        // Le désaccord n'est jamais montré à l'organisateur (§21.4) : il voit seulement « bloqué ».
        if code == DESACCORD && !direction {
            continue;
        }
        par_participant.entry(p).or_default().push(CelluleVue { code, etat, bloquante });
    }
    let mut codes: Vec<String> = par_participant.values().flatten().map(|c| c.code.clone()).collect();
    codes.sort();
    codes.dedup();
    let (mut manquants, mut bloques, mut prets) = (0, 0, 0);
    let mut lignes = Vec::new();
    for p in participants {
        let cellules = par_participant.remove(&p.id).unwrap_or_default();
        let manque = cellules.iter().any(|c| c.bloquante && !crate::voyages_exigences::est_satisfaite(&c.etat)) || p.etat == "bloque";
        match p.etat.as_str() {
            "bloque" => bloques += 1,
            "pret" => prets += 1,
            _ => {}
        }
        if manque {
            manquants += 1;
        }
        lignes.push(LigneMatrice { participant_id: p.id, nom: p.nom, prenom: p.prenom, classe: p.classe, etat: p.etat, cellules, manque });
    }
    Ok(Matrice { campagne, codes, lignes, manquants, bloques, prets })
}

/// Destinataires d'une relance : `(comptes, personnes sans compte, participants concernés)`.
async fn destinataires(conn: &mut PgConnection, etab: Uuid, sortie: Uuid) -> Result<(Vec<Uuid>, Vec<Uuid>, i64), EntDbError> {
    // Participants dont l'autorisation de participation manque ou est à revoir (jamais un refus : le titulaire a
    // répondu), mineurs ou majeurs.
    let concernes: Vec<(Uuid, Uuid, bool)> = sqlx::query_as(
        "SELECT sp.id, sp.eleve_id, COALESCE(e.date_naissance + INTERVAL '18 years' <= ent_aujourdhui_etab($1), false)
           FROM ent_sortie_exigence x
           JOIN ent_sortie_participant sp ON sp.etablissement_id = x.etablissement_id AND sp.id = x.participant_id
           JOIN ent_eleve e ON e.etablissement_id = sp.etablissement_id AND e.id = sp.eleve_id
          WHERE x.etablissement_id = $1 AND x.sortie_id = $2 AND x.code = $3 AND x.etat IN ('attendue', 'a_revoir')
            AND sp.etat IN ('inscrit', 'pret', 'bloque')",
    )
    .bind(etab)
    .bind(sortie)
    .bind(AUTORISATION)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut comptes: HashSet<Uuid> = HashSet::new();
    let mut sans: HashSet<Uuid> = HashSet::new();
    for (_, eleve, majeur) in &concernes {
        if *majeur {
            // Un élève majeur reçoit lui-même l'avis ; ses parents ne sont pas relancés à sa place (CV-07).
            let u: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM ent_eleve_compte WHERE etablissement_id = $1 AND eleve_id = $2")
                .bind(etab)
                .bind(eleve)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
            if let Some(u) = u {
                comptes.insert(u);
            }
            continue;
        }
        let titulaires: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT personne_id, user_id FROM ent_eleve_lien_sollicitable
              WHERE etablissement_id = $1 AND eleve_id = $2 AND autorite_parentale",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for (personne, user) in titulaires {
            match user {
                Some(u) => {
                    comptes.insert(u);
                }
                None => {
                    sans.insert(personne);
                }
            }
        }
    }
    let mut comptes: Vec<Uuid> = comptes.into_iter().collect();
    comptes.sort();
    let mut sans: Vec<Uuid> = sans.into_iter().collect();
    sans.sort();
    Ok((comptes, sans, concernes.len() as i64))
}

/// Enregistre la relance (ajout seul) puis notifie les comptes (sauf si `omise`).
async fn relancer(
    conn: &mut PgConnection,
    etab: Uuid,
    s: &SortieLigne,
    seuil: &str,
    omise: bool,
    acteur: Option<Uuid>,
) -> Result<ApercuRelance, EntDbError> {
    let (comptes, sans, concernes) = destinataires(conn, etab, s.id).await?;
    let (nc, ns) = if omise { (0, 0) } else { (comptes.len() as i32, sans.len() as i32) };
    let relance = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_sortie_relance (id, etablissement_id, sortie_id, seuil, omise, destinataires_comptes, destinataires_sans_compte, acteur_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(relance)
    .bind(etab)
    .bind(s.id)
    .bind(seuil)
    .bind(omise)
    .bind(nc)
    .bind(ns)
    .bind(acteur)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if !omise {
        // Texte FIXE : ni nom d'élève, ni motif ; la plage calme et les canaux suivent les préférences de la
        // personne (socle 8).
        let titre = "Autorisation à signer";
        let corps = format!("L'autorisation de participation à « {} » n'a pas encore été signée. Ouvrez-la dans votre espace pour la signer.", s.titre);
        for u in &comptes {
            notifier_personne(
                conn,
                EnvoiAPlanifier {
                    etablissement_id: etab,
                    user_id: *u,
                    kind: "voyages_autorisation_relance",
                    titre,
                    corps: &corps,
                    eleve_id: None,
                    notification_id: None,
                    source_type: "systeme",
                    source_id: relance,
                    cle_dedup: format!("relance:{relance}:{u}"),
                    droit_requis: None,
                    regroupable: false,
                    canaux_voulus: &["email", "push"],
                },
                relance,
                Utc::now(),
            )
            .await?;
        }
    }
    Ok(ApercuRelance {
        destinataires_comptes: i64::from(nc),
        destinataires_sans_compte: i64::from(ns),
        participants_concernes: concernes,
    })
}
