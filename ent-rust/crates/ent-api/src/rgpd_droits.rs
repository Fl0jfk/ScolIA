//! Routes `/rgpd/demandes/*` — **guichet des demandes de droits** (socle 27, lot G3a ;
//! `27_RGPD_ARCHIVES.md` §9, §14.3).
//!
//! * **Guichet** (`droits.recevoir`, `droits.traiter`, `droits.arbitrer`) : `GET /rgpd/demandes` (file),
//!   `POST /rgpd/demandes` (saisie d'un courrier), `GET /rgpd/demandes/{id}` (fiche),
//!   `POST …/{id}/qualification`, `…/identite`, `…/identite/precision`, `…/affectation`,
//!   `…/prolongation`, `…/droits`, `…/decision`, `…/titulaires`, `…/cloture`, `…/refus`,
//!   `…/reouverture`, et `GET /rgpd/guichet/referentiel`.
//! * **Demandeur connecté** : `POST /rgpd/mes-demandes` (dépôt), `GET /rgpd/mes-demandes` (suivi).
//! * **Public, sans session** : `POST /rgpd/demandes/sans-compte`. Réponse **invariable** : elle ne dit
//!   jamais si l'établissement détient des données sur la personne (R-RG42). Débit limité par
//!   `anti_abus::regle`.
//!
//! L'établissement vient **de la session** (ou de l'hôte pour la route publique), jamais du corps de la
//! requête. Les refus sont neutres : 403 quand le rôle n'a pas le droit, 404 quand l'objet n'existe pas
//! dans cet établissement — rien ne dit qu'il existe ailleurs.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_db::droits_demandes::{
    DepotConnecte, DepotSansCompte, SaisieDemande, ALERTE_ECHEANCE_JOURS, DROITS,
    IDENTITE_SUSPEND_DELAI, QUALITES,
};
use ent_types::rgpd_droits::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::tenant::TenantHote;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Public (sans session) : dépôt d'une demande par une personne sans compte.
        .route("/rgpd/demandes/sans-compte", post(deposer_sans_compte))
        // Demandeur connecté.
        .route("/rgpd/mes-demandes", get(mes_demandes).post(deposer))
        // Guichet.
        .route("/rgpd/guichet/referentiel", get(referentiel))
        .route("/rgpd/demandes", get(file).post(saisir))
        .route("/rgpd/demandes/{id}", get(fiche))
        .route("/rgpd/demandes/{id}/qualification", post(qualifier))
        .route("/rgpd/demandes/{id}/identite", post(verifier_identite))
        .route(
            "/rgpd/demandes/{id}/identite/precision",
            post(demander_identite),
        )
        .route(
            "/rgpd/demandes/{id}/identite/arbitrage",
            post(arbitrer_identite),
        )
        .route("/rgpd/demandes/{id}/rapprochement", post(rapprocher))
        .route("/rgpd/demandes/{id}/affectation", post(affecter))
        .route("/rgpd/demandes/{id}/prolongation", post(prolonger))
        .route("/rgpd/demandes/{id}/droits", post(ajouter_droit))
        .route("/rgpd/demandes/{id}/decision", post(decider))
        .route("/rgpd/demandes/{id}/titulaires", post(titulaires))
        .route("/rgpd/demandes/{id}/cloture", post(clore))
        .route("/rgpd/demandes/{id}/refus", post(refuser))
        .route("/rgpd/demandes/{id}/reouverture", post(rouvrir))
}

fn opt(v: &Option<String>) -> Option<&str> {
    v.as_deref()
}

/// Les droits demandés, en `&str` empruntés (le service valide la liste).
fn droits_demandes(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

fn vue(v: ent_db::droits_demandes::DemandeVue) -> DemandeDto {
    DemandeDto {
        id: v.id,
        reference: v.reference,
        canal: v.canal,
        recue_le: v.recue_le,
        saisie_le: v.saisie_le,
        saisie_tardive: v.saisie_tardive,
        echeance_le: v.echeance_le,
        prolongee_jusqu_au: v.prolongee_jusqu_au,
        prolongation_motif: v.prolongation_motif,
        echeance_effective: v.echeance_effective,
        jours_restants: v.jours_restants,
        en_retard: v.en_retard,
        qualite: v.qualite,
        demandeur_nom: v.demandeur_nom,
        demandeur_a_un_compte: v.demandeur_a_un_compte,
        sujet_type: v.sujet_type,
        sujet_id: v.sujet_id,
        personne_concernee: v.sujet_nom_saisi,
        qualification: v.qualification,
        qualification_motif: v.qualification_motif,
        perimetre_texte: v.perimetre_texte,
        perimetre_du: v.perimetre_du,
        perimetre_au: v.perimetre_au,
        etat: v.etat,
        gestionnaire_personne_id: v.gestionnaire_personne_id,
        identite_demandee_le: v.identite_demandee_le,
        identite_fournie_le: v.identite_fournie_le,
        identite_attente_jours: v.identite_attente_jours,
        reouvertures: v.reouvertures,
        close_le: v.close_le,
        reponse_resume: v.reponse_resume,
        refus_motif: v.refus_motif,
        titulaires_informes_le: v.titulaires_informes_le,
        titulaires_non_informes_motif: v.titulaires_non_informes_motif,
        droits: v
            .droits
            .into_iter()
            .map(|d| DroitDecisionDto {
                droit: d.droit,
                decision: d.decision,
                motif_code: d.motif_code,
                motif_texte: d.motif_texte,
                decide_le: d.decide_le,
            })
            .collect(),
        identites: v
            .identites
            .into_iter()
            .map(|i| IdentiteDto {
                mode: i.mode,
                resultat: i.resultat,
                qualite_constatee: i.qualite_constatee,
                qualite_note: i.qualite_note,
                verifie_le: i.verifie_le,
                piece_detenue: i.piece_detenue,
                destruction_en_attente: i.destruction_en_attente,
                piece_detruite_le: i.piece_detruite_le,
            })
            .collect(),
        journal: v
            .journal
            .into_iter()
            .map(|l| JournalDto {
                evenement: l.evenement,
                detail: l.detail,
                par_role: l.par_role,
                le: l.le,
            })
            .collect(),
        piece_a_detruire: v.piece_a_detruire,
        destruction_en_attente: v.destruction_en_attente,
        identite_arbitrage_code: v.identite_arbitrage_code,
        titulaires_a_decider: v.titulaires_a_decider,
    }
}

// ── Public ──────────────────────────────────────────────────────────────────────────────────

/// RGP-142 — dépôt sans compte. **Aucune information** sur l'existence d'un dossier : la réponse est
/// toujours la même phrase (R-RG42). L'établissement vient de l'hôte de la requête.
async fn deposer_sans_compte(
    State(s): State<AppState>,
    tenant: TenantHote,
    Json(b): Json<DepotPublicBody>,
) -> Res<Json<DepotPublicDto>> {
    let etab = tenant
        .etablissement_id
        .ok_or((StatusCode::NOT_FOUND, "établissement inconnu".to_string()))?;
    let droits = droits_demandes(&b.droits);
    let message = s
        .ent
        .droits_deposer_sans_compte(
            etab,
            DepotSansCompte {
                nom_saisi: &b.nom,
                contact: &b.contact,
                qualite: &b.qualite,
                sujet_nom_saisi: &b.personne_concernee,
                droits: &droits,
                perimetre_texte: opt(&b.perimetre_texte),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(DepotPublicDto {
        message: message.to_string(),
    }))
}

// ── Demandeur connecté ──────────────────────────────────────────────────────────────────────

/// RGP-140 — dépôt par un demandeur connecté.
async fn deposer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DepotBody>,
) -> Res<(StatusCode, Json<DemandeDto>)> {
    let droits = droits_demandes(&b.droits);
    let v = s
        .ent
        .droits_deposer_connecte(
            session.etablissement_id,
            session.user_id,
            &session.role,
            DepotConnecte {
                qualite: &b.qualite,
                sujet_type: &b.sujet_type,
                sujet_id: b.sujet_id,
                droits: &droits,
                perimetre_texte: opt(&b.perimetre_texte),
                perimetre_du: b.perimetre_du,
                perimetre_au: b.perimetre_au,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(vue(v))))
}

/// RGP-155 — suivi de **ses** demandes.
async fn mes_demandes(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<SuiviDemandeDto>>> {
    let l = s
        .ent
        .droits_mes_demandes(session.etablissement_id, session.user_id)
        .await
        .map_err(map_db)?;
    Ok(Json(
        l.into_iter()
            .map(|d| SuiviDemandeDto {
                id: d.id,
                reference: d.reference,
                recue_le: d.recue_le,
                echeance_effective: d.echeance_effective,
                etat: d.etat,
                avancement: d.avancement,
                droits: d.droits,
                prolongee_jusqu_au: d.prolongee_jusqu_au,
                prolongation_motif: d.prolongation_motif,
                close_le: d.close_le,
                reponse_resume: d.reponse_resume,
            })
            .collect(),
    ))
}

// ── Guichet ─────────────────────────────────────────────────────────────────────────────────

/// Référentiel du guichet : vocabulaire, grille d'identité (RGP-307), modèles de réponse (RGP-304).
async fn referentiel(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<ReferentielGuichetDto>> {
    let (grille, modeles) = s
        .ent
        .droits_referentiel(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(ReferentielGuichetDto {
        droits: DROITS.iter().map(|d| (*d).to_string()).collect(),
        qualites: QUALITES.iter().map(|q| (*q).to_string()).collect(),
        canaux_guichet: ["courrier", "courriel", "guichet", "dpo", "cnil"]
            .iter()
            .map(|c| (*c).to_string())
            .collect(),
        grille_identite: grille
            .iter()
            .map(|e| EtapeVerificationDto {
                code: e.code.to_string(),
                libelle: e.libelle.to_string(),
                geste: e.geste.to_string(),
                obligatoire: e.obligatoire,
            })
            .collect(),
        modeles_reponse: modeles
            .iter()
            .map(|m| ModeleReponseDto {
                code: m.code.to_string(),
                libelle: m.libelle.to_string(),
                droit: m.droit.map(str::to_string),
                motif_code: m.motif_code.to_string(),
                texte: m.texte.to_string(),
            })
            .collect(),
        alerte_echeance_jours: ALERTE_ECHEANCE_JOURS.to_vec(),
        identite_suspend_delai: IDENTITE_SUSPEND_DELAI,
    }))
}

#[derive(Debug, Deserialize)]
struct FiltreFile {
    /// Inclure les demandes closes des douze derniers mois.
    #[serde(default)]
    closes: bool,
}

/// RGP-169 — file du guichet (métadonnées seulement, R-RG32).
async fn file(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<FiltreFile>,
) -> Res<Json<TableauDemandesDto>> {
    let t = s
        .ent
        .droits_tableau(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.closes,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(TableauDemandesDto {
        jour: t.jour,
        lignes: t
            .lignes
            .into_iter()
            .map(|l| DemandeLigneDto {
                id: l.id,
                reference: l.reference,
                canal: l.canal,
                recue_le: l.recue_le,
                echeance_effective: l.echeance_effective,
                jours_restants: l.jours_restants,
                en_retard: l.en_retard,
                etat: l.etat,
                qualification: l.qualification,
                qualite: l.qualite,
                sujet_type: l.sujet_type,
                gestionnaire_personne_id: l.gestionnaire_personne_id,
                droits: l.droits,
                saisie_tardive: l.saisie_tardive,
            })
            .collect(),
        ouvertes: t.ouvertes,
        en_retard: t.en_retard,
        echeance_proche: t.echeance_proche,
        identite_a_verifier: t.identite_a_verifier,
        identite_a_arbitrer: t.identite_a_arbitrer,
        a_qualifier: t.a_qualifier,
        titulaires_a_decider: t.titulaires_a_decider,
        closes: t.closes,
    }))
}

/// RGP-143 — saisie d'une demande reçue hors ligne.
async fn saisir(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<SaisieBody>,
) -> Res<(StatusCode, Json<DemandeDto>)> {
    let droits = droits_demandes(&b.droits);
    let v = s
        .ent
        .droits_saisir_au_guichet(
            session.etablissement_id,
            session.user_id,
            &session.role,
            SaisieDemande {
                canal: &b.canal,
                recue_le: b.recue_le,
                demandeur_compte_id: b.demandeur_compte_id,
                demandeur_personne_id: b.demandeur_personne_id,
                demandeur_nom_saisi: opt(&b.demandeur_nom),
                demandeur_contact: opt(&b.demandeur_contact),
                qualite: &b.qualite,
                sujet_type: &b.sujet_type,
                sujet_id: b.sujet_id,
                sujet_nom_saisi: opt(&b.personne_concernee),
                droits: &droits,
                perimetre_texte: opt(&b.perimetre_texte),
                perimetre_du: b.perimetre_du,
                perimetre_au: b.perimetre_au,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(vue(v))))
}

async fn fiche(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_demande(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn qualifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<QualificationBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_qualifier(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.qualification,
            &b.motif,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn verifier_identite(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<IdentiteBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_verifier_identite(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.mode,
            b.reussie,
            opt(&b.note),
            b.piece_document_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

/// E2 — la direction tranche une qualité non compatible (`droits.arbitrer`).
async fn arbitrer_identite(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ArbitrageIdentiteBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_arbitrer_identite(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            b.accepte,
            &b.motif,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

/// M2 — rapprochement du sujet d'un dépôt sans compte et de la fiche du demandeur.
async fn rapprocher(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RapprochementBody>,
) -> Res<Json<DemandeDto>> {
    let sujet = match (opt(&b.sujet_type), b.sujet_id) {
        (Some(t), Some(i)) => Some((t, i)),
        (None, None) => None,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "type et identifiant du sujet vont ensemble".to_string(),
            ))
        }
    };
    let v = s
        .ent
        .droits_rapprocher(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            sujet,
            b.demandeur_personne_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn demander_identite(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PrecisionIdentiteBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_demander_identite(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.demande,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn affecter(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AffectationBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_affecter(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            b.gestionnaire_personne_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn prolonger(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ProlongationBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_prolonger(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            b.jusqu_au,
            &b.motif,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn ajouter_droit(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DroitBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_ajouter_droit(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.droit,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn decider(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DecisionBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_decider(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.droit,
            &b.decision,
            opt(&b.motif_code),
            opt(&b.motif_texte),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn titulaires(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TitulairesBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_titulaires_information(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            b.informer,
            opt(&b.motif),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn clore(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ClotureBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_clore(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.reponse_resume,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn refuser(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<MotifBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_refuser(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.motif,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn rouvrir(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<MotifBody>,
) -> Res<Json<DemandeDto>> {
    let v = s
        .ent
        .droits_rouvrir(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.motif,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}
