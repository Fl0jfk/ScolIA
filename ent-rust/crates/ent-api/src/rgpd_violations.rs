//! Routes `/rgpd/violations/*` — **registre des violations de données** (socle 27, palier G5 ;
//! `27_RGPD_ARCHIVES.md` §8.7, §14.10).
//!
//! * **Tout personnel** (`violations.declarer`) : `POST /rgpd/violations/declarations` (déclarer en trente secondes),
//!   `GET /rgpd/violations/declarations` (ses déclarations : référence et avancement seulement, R-RG32).
//! * **Gestionnaires** (`violations.gerer`) : `GET /rgpd/violations` (registre), `GET /rgpd/violations/export.csv`,
//!   `POST /rgpd/violations` (saisie, prise de connaissance antérieure possible), `GET /rgpd/violations/{id}` (fiche),
//!   `GET …/{id}/brouillon-cnil`, `POST …/{id}/qualification`, `…/mesures`, `…/avis-dpo`, `…/notification-cnil`,
//!   `…/notification-complementaire`, `…/personnes` (calcul), `GET …/information` (aperçu), `…/cloture` ; fiches PDF
//!   (complément G5, NX27-18) : `GET /rgpd/violations/registre.pdf` (même filtre que le registre) et
//!   `GET /rgpd/violations/{id}/fiche.pdf`.
//! * **Direction** (`violations.decider` ★) : `POST …/{id}/risque`, `…/non-notification`, `POST …/information`
//!   (confirmée par le nombre de destinataires), `…/non-information`.
//!
//! L'établissement vient **de la session**, jamais du corps de la requête. Refus neutres : 403 sans le droit, 404 pour
//! une violation qui n'est pas de cet établissement.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_db::violations::{Declaration, FiltreRegistre, MaDeclaration, Qualification, ViolationVue};
use ent_db::violations_information::OriginePersonnes;
use ent_types::rgpd_violations::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/rgpd/violations/declarations",
            get(mes_declarations).post(declarer),
        )
        .route("/rgpd/violations", get(registre).post(saisir))
        .route("/rgpd/violations/export.csv", get(export_csv))
        .route("/rgpd/violations/registre.pdf", get(registre_pdf))
        .route("/rgpd/violations/{id}/fiche.pdf", get(fiche_pdf))
        .route("/rgpd/violations/{id}", get(fiche))
        .route("/rgpd/violations/{id}/brouillon-cnil", get(brouillon_cnil))
        .route("/rgpd/violations/{id}/qualification", post(qualifier))
        .route("/rgpd/violations/{id}/mesures", post(ajouter_mesure))
        .route("/rgpd/violations/{id}/avis-dpo", post(avis_dpo))
        .route("/rgpd/violations/{id}/risque", post(decider_risque))
        .route(
            "/rgpd/violations/{id}/notification-cnil",
            post(notifier_cnil),
        )
        .route(
            "/rgpd/violations/{id}/notification-complementaire",
            post(complement_cnil),
        )
        .route(
            "/rgpd/violations/{id}/non-notification",
            post(non_notification),
        )
        .route("/rgpd/violations/{id}/personnes", post(calculer_personnes))
        .route(
            "/rgpd/violations/{id}/information",
            get(apercu_information).post(informer_personnes),
        )
        .route(
            "/rgpd/violations/{id}/non-information",
            post(non_information),
        )
        .route("/rgpd/violations/{id}/cloture", post(clore))
}

fn declaration(b: &DeclarationBody) -> Declaration<'_> {
    Declaration {
        intitule: &b.intitule,
        description: b.description.as_deref(),
        constatee_le: b.constatee_le,
        incident_securite: b.incident_securite,
    }
}

fn ma(m: MaDeclaration) -> MaDeclarationDto {
    MaDeclarationDto {
        id: m.id,
        reference: m.reference,
        intitule: m.intitule,
        connue_le: m.connue_le,
        avancement: m.avancement.to_string(),
        message: m.message.to_string(),
    }
}

pub(crate) fn vue(v: ViolationVue) -> ViolationDto {
    ViolationDto {
        id: v.id,
        reference: v.reference,
        nature_incident: v.nature_incident,
        origine: v.origine,
        plateforme: v.plateforme,
        source_code: v.source_code,
        declaree_par_compte_id: v.declaree_par_compte_id,
        intitule: v.intitule,
        description: v.description,
        constatee_le: v.constatee_le,
        connue_le: v.connue_le,
        echeance_cnil_le: v.echeance_cnil_le,
        heures_restantes: v.heures_restantes,
        en_retard: v.en_retard,
        confidentialite: v.confidentialite,
        integrite: v.integrite,
        disponibilite: v.disponibilite,
        categories: v.categories,
        personnes_nombre: v.personnes_nombre,
        personnes_estime: v.personnes_estime,
        personnes_mineurs: v.personnes_mineurs,
        consequences: v.consequences,
        qualifiee_le: v.qualifiee_le,
        risque_propose: v.risque_propose,
        raisons_proposition: v
            .raisons_proposition
            .iter()
            .map(|r| r.to_string())
            .collect(),
        avis_dpo: v.avis_dpo,
        avis_dpo_le: v.avis_dpo_le,
        risque: v.risque,
        risque_motif: v.risque_motif,
        risque_decide_le: v.risque_decide_le,
        notifiee_cnil_le: v.notifiee_cnil_le,
        reference_cnil: v.reference_cnil,
        retard_motif: v.retard_motif,
        non_notification_motif: v.non_notification_motif,
        personnes_informees_le: v.personnes_informees_le,
        personnes_informees_nombre: v.personnes_informees_nombre,
        personnes_non_information_motif: v.personnes_non_information_motif,
        etat: v.etat,
        close_le: v.close_le,
        retour_experience: v.retour_experience,
        revue_le: v.revue_le,
        mesures: v
            .mesures
            .into_iter()
            .map(|m| MesureDto {
                nature: m.nature,
                texte: m.texte,
                le: m.le,
            })
            .collect(),
        notifications: v
            .notifications
            .into_iter()
            .map(|n| NotificationCnilDto {
                rang: n.rang,
                nature: n.nature,
                notifiee_le: n.notifiee_le,
                reference_cnil: n.reference_cnil,
                resume: n.resume,
            })
            .collect(),
        chronologie: v
            .chronologie
            .into_iter()
            .map(|e| EtapeViolationDto {
                etape: e.etape,
                texte: e.texte,
                par_role: e.par_role,
                le: e.le,
            })
            .collect(),
        manque_pour_clore: v.manque_pour_clore.iter().map(|m| m.to_string()).collect(),
        plateforme_element: v.plateforme_element.map(|e| ElementPlateformeDto {
            reference: e.reference,
            intitule: e.intitule,
            description: e.description,
            connue_editeur_le: e.connue_editeur_le,
            informe_le: e.informe_le,
            elements: e.elements,
            personnes_nombre: e.personnes_nombre,
            couche_compte: e.couche_compte,
            notifiee_cnil_par_editeur_le: e.notifiee_cnil_par_editeur_le,
        }),
    }
}

/// RGP-240 — déclarer (tout personnel).
async fn declarer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DeclarationBody>,
) -> Res<(StatusCode, Json<MaDeclarationDto>)> {
    let m = s
        .ent
        .violations_declarer(
            session.etablissement_id,
            session.user_id,
            &session.role,
            declaration(&b),
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(ma(m))))
}

async fn mes_declarations(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<MaDeclarationDto>>> {
    let l = s
        .ent
        .violations_mes_declarations(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(l.into_iter().map(ma).collect()))
}

/// Saisie par un gestionnaire.
async fn saisir(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<SaisieViolationBody>,
) -> Res<(StatusCode, Json<ViolationDto>)> {
    let v = s
        .ent
        .violations_saisir(
            session.etablissement_id,
            session.user_id,
            &session.role,
            declaration(&b.declaration),
            b.connue_le,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(vue(v))))
}

fn filtre(q: &FiltreRegistreQuery) -> FiltreRegistre<'_> {
    FiltreRegistre {
        annee: q.annee,
        etat: q.etat.as_deref(),
        nature_incident: q.nature_incident.as_deref(),
        a_notifier: q.a_notifier.unwrap_or(false),
    }
}

/// RGP-258 — registre.
async fn registre(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<FiltreRegistreQuery>,
) -> Res<Json<Vec<ViolationLigneDto>>> {
    let l = s
        .ent
        .violations_registre(
            session.etablissement_id,
            session.user_id,
            &session.role,
            filtre(&q),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        l.into_iter()
            .map(|l| ViolationLigneDto {
                id: l.id,
                reference: l.reference,
                nature_incident: l.nature_incident,
                origine: l.origine,
                intitule: l.intitule,
                connue_le: l.connue_le,
                echeance_cnil_le: l.echeance_cnil_le,
                heures_restantes: l.heures_restantes,
                en_retard: l.en_retard,
                risque: l.risque,
                notifiee_cnil_le: l.notifiee_cnil_le,
                non_notifiee: l.non_notifiee,
                personnes_nombre: l.personnes_nombre,
                etat: l.etat,
                close_le: l.close_le,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
struct AnneeQuery {
    #[serde(default)]
    annee: Option<i32>,
}

async fn export_csv(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeQuery>,
) -> Res<impl IntoResponse> {
    let corps = s
        .ent
        .violations_registre_csv(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.annee,
        )
        .await
        .map_err(map_db)?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"registre-violations.csv\"",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        corps,
    ))
}

/// PDF : jamais en cache (registre interne), téléchargé sous un nom sans donnée personnelle.
fn reponse_pdf(nom: String, corps: Vec<u8>) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{nom}\""),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        corps,
    )
}

/// NX27-18 — registre en PDF (même filtre que `GET /rgpd/violations`).
async fn registre_pdf(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<FiltreRegistreQuery>,
) -> Res<impl IntoResponse> {
    let corps = s
        .ent
        .violations_registre_pdf(
            session.etablissement_id,
            session.user_id,
            &session.role,
            filtre(&q),
        )
        .await
        .map_err(map_db)?;
    Ok(reponse_pdf("registre-violations.pdf".into(), corps))
}

/// NX27-18 — fiche PDF d'une violation.
async fn fiche_pdf(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<impl IntoResponse> {
    let (reference, corps) = s
        .ent
        .violations_fiche_pdf(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    // La référence (« V-2027-003 ») n'a que des lettres, chiffres et tirets ; filtrée quand même.
    let nom: String = reference
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    Ok(reponse_pdf(format!("violation-{nom}.pdf"), corps))
}

async fn fiche(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ViolationDto>> {
    let v = s
        .ent
        .violations_fiche(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

/// RGP-247 — brouillon de notification (art. 33.3), à recopier sur le téléservice de la CNIL.
async fn brouillon_cnil(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<BrouillonCnilDto>> {
    let b = s
        .ent
        .violations_brouillon_cnil(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(BrouillonCnilDto {
        reference: b.reference,
        rubriques: b
            .rubriques
            .into_iter()
            .map(|r| RubriqueDto {
                titre: r.titre.to_string(),
                texte: r.texte,
            })
            .collect(),
        a_completer: b.a_completer.iter().map(|a| a.to_string()).collect(),
    }))
}

async fn qualifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<QualificationViolationBody>,
) -> Res<Json<ViolationDto>> {
    let cats: Vec<&str> = b.categories.iter().map(String::as_str).collect();
    let v = s
        .ent
        .violations_qualifier(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            Qualification {
                nature_incident: &b.nature_incident,
                confidentialite: b.confidentialite,
                integrite: b.integrite,
                disponibilite: b.disponibilite,
                categories: &cats,
                personnes_nombre: b.personnes_nombre,
                personnes_estime: b.personnes_estime,
                personnes_mineurs: b.personnes_mineurs,
                consequences: b.consequences.as_deref(),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn ajouter_mesure(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<MesureBody>,
) -> Res<Json<ViolationDto>> {
    let v = s
        .ent
        .violations_ajouter_mesure(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.nature,
            &b.texte,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn avis_dpo(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TexteBody>,
) -> Res<Json<ViolationDto>> {
    let v = s
        .ent
        .violations_avis_dpo(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.texte,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

/// RGP-245 ★ — décision du risque (direction).
async fn decider_risque(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RisqueBody>,
) -> Res<Json<ViolationDto>> {
    let v = s
        .ent
        .violations_decider_risque(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.risque,
            &b.motif,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn notifier_cnil(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<NotificationCnilBody>,
) -> Res<Json<ViolationDto>> {
    let v = s
        .ent
        .violations_notifier_cnil(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            b.notifiee_le,
            &b.reference_cnil,
            b.retard_motif.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

async fn complement_cnil(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ComplementCnilBody>,
) -> Res<Json<ViolationDto>> {
    let v = s
        .ent
        .violations_notification_complementaire(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            b.notifiee_le,
            &b.reference_cnil,
            &b.resume,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

/// RGP-253 ★ — non-notification justifiée (risque faible décidé).
async fn non_notification(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TexteBody>,
) -> Res<Json<ViolationDto>> {
    let v = s
        .ent
        .violations_justifier_non_notification(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.texte,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

/// RGP-244 — calcul des personnes concernées depuis une classe, des élèves ou des comptes.
async fn calculer_personnes(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PersonnesBody>,
) -> Res<Json<ViolationDto>> {
    let origine = match b {
        PersonnesBody::Classe { classe_id } => OriginePersonnes::Classe(classe_id),
        PersonnesBody::Eleves { eleve_ids } => OriginePersonnes::Eleves(eleve_ids),
        PersonnesBody::Comptes { compte_ids } => OriginePersonnes::Comptes(compte_ids),
    };
    let v = s
        .ent
        .violations_calculer_personnes(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            origine,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

/// Aperçu relu avant d'informer : combien de personnes, de destinataires, de personnes sans destinataire joignable.
async fn apercu_information(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ApercuInformationDto>> {
    let a = s
        .ent
        .violations_apercu_information(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(ApercuInformationDto {
        personnes: a.personnes,
        destinataires: a.destinataires,
        personnes_sans_destinataire: a.personnes_sans_destinataire,
        mineurs: a.mineurs,
        deja_informees: a.deja_informees,
    }))
}

/// RGP-252 ★ — information des personnes (art. 34), confirmée par le nombre de destinataires de l'aperçu.
async fn informer_personnes(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<InformationBody>,
) -> Res<Json<ViolationDto>> {
    let v = s
        .ent
        .violations_informer_personnes(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.message,
            b.confirmation,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

/// Exception motivée à l'information des personnes (art. 34.3).
async fn non_information(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TexteBody>,
) -> Res<Json<ViolationDto>> {
    let v = s
        .ent
        .violations_motiver_non_information(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.texte,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}

/// RGP-256 — clôture avec retour d'expérience.
async fn clore(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TexteBody>,
) -> Res<Json<ViolationDto>> {
    let v = s
        .ent
        .violations_clore(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.texte,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(vue(v)))
}
