//! Routes du dossier élève et des personnes (socle 2, lot S2-1).
//!
//! L'autorité est `ent-db` (`dossier_personnes.rs`) : droits (`personnes.gerer`,
//! `payeurs.definir`, `liens.suspendre`, `personnes.fusionner`), liens actifs,
//! minimisation. Une référence d'un autre établissement répond 404.
//! Types JSON partagés avec le frontend : `ent_types::dossier`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use ent_db::{
    ConflitInteretVu, ContactSaisie, ContactUrgence, EntreeChronologie, FicheEleve, FusionPersonne,
    InterdictionSaisie, InterdictionVue, LienResultat, LienSaisie, Personne, PersonneSaisie,
    PersonneTrouvee, Responsable,
};
use ent_types::dossier::{
    ConflitInteretBody, ConflitInteretDto, InterdictionBody, InterdictionDto, LeverInterdictionBody,
    MasquageDto,
    ActionFicheDto, AdresseParqueeDto, AdresseProtegeeDto, AjoutResponsableBody, AlertesProtectionDto,
    ConfirmationGuichetBody, ConfirmationGuichetDto, LienEnAttenteEleveDto, NePasSolliciterBody, ProtectionADeclarerDto, LienEnAttenteDto, MonLienEnAttenteDto, ReponseLienBody, CompteBody, ContactBody, ContactUrgenceDto, EntreeChronologieDto,
    FicheEleveDto, FusionBody, FusionDto, LienBody, PayeursBody, PersonneBody, PersonneDto,
    PersonneTrouveeDto, ResponsableDto, ResumeSectionsDto, SectionsDto, SuspensionBody,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dossier/eleves/{id}/fiche", get(fiche))
        .route("/dossier/eleves/{id}/chronologie", get(chronologie))
        .route(
            "/dossier/eleves/{id}/responsables",
            get(responsables).post(lier),
        )
        .route("/dossier/eleves/{id}/responsables/ajout", post(ajouter_responsable))
        .route("/dossier/mes-liens-en-attente", get(mes_liens_en_attente))
        .route(
            "/dossier/mes-liens-en-attente/{id}/reponse",
            post(repondre_lien_en_attente),
        )
        // Lot A0 bis : confirmation au guichet, protection des personnes (direction)
        .route(
            "/dossier/liens-en-attente/{id}/confirmer-guichet",
            post(confirmer_guichet),
        )
        .route(
            "/dossier/eleves/{id}/liens-en-attente",
            get(liens_en_attente_eleve),
        )
        .route("/dossier/alertes-protection", get(alertes_protection))
        .route("/dossier/personnes/{id}/ne-pas-solliciter", post(ne_pas_solliciter))
        .route(
            "/dossier/adresses-protegees/{id}/ouvrir",
            post(ouvrir_adresse_protegee),
        )
        // Lot A1 : interdictions (groupe R) et conflits d'intérêts (R-A18)
        .route(
            "/dossier/eleves/{id}/interdictions",
            get(interdictions).post(poser_interdiction),
        )
        .route("/dossier/interdictions/{id}/lever", post(lever_interdiction))
        .route("/dossier/eleves/{id}/conflit-interet", post(declarer_conflit_interet))
        .route("/dossier/conflits-interet", get(conflits_interet))
        .route("/dossier/conflits-interet/{id}/lever", post(lever_conflit_interet))
        .route("/dossier/eleves/{id}/payeurs", put(payeurs))
        .route("/dossier/eleves/{id}/contacts", get(contacts).post(ajouter_contact))
        .route("/dossier/eleves/{id}/personne-eleve", post(personne_eleve))
        .route("/dossier/liens/{id}", put(modifier_lien))
        .route("/dossier/liens/{id}/suspendre", post(suspendre))
        .route("/dossier/liens/{id}/retablir", post(retablir))
        .route(
            "/dossier/contacts/{id}",
            axum::routing::delete(retirer_contact),
        )
        .route("/dossier/contacts/{id}/valider", post(valider_contact))
        .route("/dossier/personnes", get(rechercher).post(creer_personne))
        .route(
            "/dossier/personnes/{id}",
            get(get_personne).put(modifier_personne),
        )
        .route("/dossier/personnes/{id}/compte", put(rattacher_compte))
        .route("/dossier/fusions", get(fusions).post(fusionner))
        .route("/dossier/fusions/{id}/annuler", post(defusionner))
}

// -- Conversions (règle d'orphelin : fonctions locales) ---------------------

pub(crate) fn personne_dto(p: Personne) -> PersonneDto {
    PersonneDto {
        id: p.id,
        nom: p.nom,
        prenom: p.prenom,
        civilite: p.civilite,
        email: p.email,
        telephone: p.telephone,
        telephone_2: p.telephone_2,
        adresse: p.adresse,
        a_compte: p.user_id.is_some(),
        represente_eleve: p.eleve_id.is_some(),
        fusionnee_dans: p.fusionnee_dans,
    }
}

fn responsable_dto(r: Responsable) -> ResponsableDto {
    ResponsableDto {
        lien_id: r.lien_id,
        eleve_id: r.eleve_id,
        personne: personne_dto(r.personne),
        foyer_id: r.foyer_id,
        foyer_libelle: r.foyer_libelle,
        lien: r.lien,
        autorite_parentale: r.autorite_parentale,
        garde: r.garde,
        date_debut: r.date_debut,
        date_fin: r.date_fin,
        suspendu_le: r.suspendu_le,
        suspension_motif: r.suspension_motif,
        suspension_reference: r.suspension_reference,
        actif: r.actif,
        peut_voir_notes: r.peut_voir_notes,
        peut_voir_vs: r.peut_voir_vs,
        peut_justifier_absence: r.peut_justifier_absence,
        peut_autoriser_sorties: r.peut_autoriser_sorties,
        est_payeur: r.est_payeur,
        part_paiement: r.part_paiement,
        masque: r.masque.into_iter().map(masquage_dto).collect(),
    }
}

/// Groupe fermé et raison (A1) : le libellé et le message viennent du catalogue `ent_auth`.
pub(crate) fn masquage_dto(m: ent_auth::Masquage) -> MasquageDto {
    MasquageDto {
        groupe: m.groupe.code().into(),
        libelle: m.groupe.libelle().into(),
        raison: m.raison.code().into(),
        message: m.raison.message().into(),
        action_a_demander: m.raison.action_a_demander().map(|a| a.code().into()),
    }
}

fn interdiction_dto(i: InterdictionVue) -> InterdictionDto {
    InterdictionDto {
        id: i.id,
        personne_id: i.personne_id,
        personne_libelle: i.personne_libelle,
        objet: i.objet,
        bloquante: i.bloquante,
        consigne: i.consigne,
        du: i.du,
        au: i.au,
        fondement: i.fondement,
        motif: i.motif,
    }
}

/// Réponse portant un motif ou un fondement : jamais mise en cache.
fn sans_cache<T: serde::Serialize>(v: T) -> Response {
    let mut rep = Json(v).into_response();
    rep.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    rep
}

fn contact_dto(c: ContactUrgence) -> ContactUrgenceDto {
    ContactUrgenceDto {
        id: c.id,
        personne: personne_dto(c.personne),
        relation: c.relation,
        ordre: c.ordre,
        peut_recuperer: c.peut_recuperer,
        statut: c.statut,
        commentaire: c.commentaire,
    }
}

fn compte(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn fiche_dto(f: FicheEleve) -> FicheEleveDto {
    let s = f.sections;
    let masques = f
        .masques
        .iter()
        .map(|m| ent_types::dossier::MasqueChampsDto {
            groupe: m.groupe.to_string(),
            raison: m.raison.to_string(),
            action: m.action.map(str::to_string),
            message: m.message.clone(),
        })
        .collect();
    FicheEleveDto {
        eleve_id: f.eleve.id,
        nom: f.eleve.nom,
        prenom: f.eleve.prenom,
        classe: f.eleve.classe,
        regime: f.eleve.regime,
        statut: f.eleve.status.as_str().to_string(),
        ine: f.eleve.ine,
        date_naissance: f.eleve.date_naissance,
        age_ans: f.age_ans,
        majeur: f.majeur,
        date_entree: f.eleve.date_entree,
        date_sortie: f.eleve.date_sortie,
        sections: SectionsDto {
            identite: s.identite,
            scolarite: s.scolarite,
            famille: s.famille,
            options: s.options,
            notes: s.notes,
            vs: s.vs,
            documents: s.documents,
            dispositifs: s.dispositifs,
            repas: s.repas,
            bourses: s.bourses,
            continuum: s.continuum,
        },
        resume: ResumeSectionsDto {
            notes: compte(f.resume.notes),
            sanctions: compte(f.resume.sanctions),
            incidents: compte(f.resume.incidents),
            carnet: compte(f.resume.carnet),
            documents: compte(f.resume.documents),
            options: compte(f.resume.options),
            dispositifs: compte(f.resume.dispositifs),
            bourses: compte(f.resume.bourses),
        },
        responsables: f.responsables.into_iter().map(responsable_dto).collect(),
        contacts: f.contacts.into_iter().map(contact_dto).collect(),
        actions: f
            .actions
            .into_iter()
            .map(|a| ActionFicheDto {
                code: a.code.into(),
                libelle: a.libelle.into(),
            })
            .collect(),
        alertes: f.alertes,
        masques,
        masquages: f.masquages.into_iter().map(masquage_dto).collect(),
        interdictions: f.interdictions.into_iter().map(interdiction_dto).collect(),
        protections: f
            .protections
            .into_iter()
            .map(|p| ent_types::dossier::ProtectionFicheDto {
                personne_id: p.personne_id,
                designation: p.designation,
                confirmee: p.confirmee,
            })
            .collect(),
    }
}

fn chrono_dto(e: EntreeChronologie) -> EntreeChronologieDto {
    EntreeChronologieDto {
        quand: e.quand,
        source: e.source.into(),
        domaine: e.domaine.into(),
        titre: e.titre,
    }
}

fn fusion_dto(f: FusionPersonne) -> FusionDto {
    FusionDto {
        id: f.id,
        source_id: f.source_id,
        cible_id: f.cible_id,
        motif: f.motif,
        fusionne_le: f.fusionne_le,
        annule_le: f.annule_le,
    }
}

fn trouvee_dto(t: PersonneTrouvee) -> PersonneTrouveeDto {
    PersonneTrouveeDto {
        personne: personne_dto(t.personne),
        nb_liens: t.nb_liens,
    }
}

fn saisie_personne(b: PersonneBody) -> PersonneSaisie {
    PersonneSaisie {
        nom: b.nom,
        prenom: b.prenom,
        civilite: b.civilite,
        email: b.email,
        telephone: b.telephone,
        telephone_2: b.telephone_2,
        adresse: b.adresse,
    }
}

fn saisie_lien(b: LienBody) -> LienSaisie {
    LienSaisie {
        personne_id: b.personne_id,
        foyer_id: b.foyer_id,
        lien: b.lien,
        autorite_parentale: b.autorite_parentale,
        garde: b.garde,
        date_debut: b.date_debut,
        date_fin: b.date_fin,
        peut_voir_notes: b.peut_voir_notes,
        peut_voir_vs: b.peut_voir_vs,
        peut_justifier_absence: b.peut_justifier_absence,
        peut_autoriser_sorties: b.peut_autoriser_sorties,
    }
}

// -- Lecture ---------------------------------------------------------------

async fn fiche(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<FicheEleveDto>> {
    let f = state
        .ent
        .fiche_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "dossier inaccessible".to_string()))?;
    Ok(Json(fiche_dto(f)))
}

#[derive(Deserialize)]
struct ChronoQuery {
    #[serde(default)]
    limite: Option<usize>,
}

async fn chronologie(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<ChronoQuery>,
) -> Res<Json<Vec<EntreeChronologieDto>>> {
    let v = state
        .ent
        .chronologie_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            q.limite.unwrap_or(100),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(chrono_dto).collect()))
}

async fn responsables(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ResponsableDto>>> {
    let v = state
        .ent
        .responsables_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(responsable_dto).collect()))
}

async fn contacts(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ContactUrgenceDto>>> {
    let v = state
        .ent
        .contacts_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(contact_dto).collect()))
}

#[derive(Deserialize)]
struct RechercheQuery {
    #[serde(default)]
    q: String,
}

async fn rechercher(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<RechercheQuery>,
) -> Res<Json<Vec<PersonneTrouveeDto>>> {
    let v = state
        .ent
        .rechercher_personnes_for_actor(session.etablissement_id, session.user_id, &session.role, &q.q)
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(trouvee_dto).collect()))
}

async fn get_personne(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<PersonneDto>> {
    let p = state
        .ent
        .get_personne_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(personne_dto(p)))
}

async fn fusions(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<FusionDto>>> {
    let v = state
        .ent
        .list_fusions_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(fusion_dto).collect()))
}

// -- Écriture --------------------------------------------------------------

async fn creer_personne(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<PersonneBody>,
) -> Res<(StatusCode, Json<PersonneDto>)> {
    let p = state
        .ent
        .creer_personne_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &saisie_personne(b),
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(personne_dto(p))))
}

async fn modifier_personne(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PersonneBody>,
) -> Res<Json<PersonneDto>> {
    let p = state
        .ent
        .modifier_personne_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &saisie_personne(b),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(personne_dto(p)))
}

async fn rattacher_compte(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CompteBody>,
) -> Res<Json<PersonneDto>> {
    let p = state
        .ent
        .rattacher_compte_confirme(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            b.user_id,
            b.confirmation_direction,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(personne_dto(p)))
}

async fn personne_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<PersonneDto>> {
    let p = state
        .ent
        .personne_eleve_majeur_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(personne_dto(p)))
}

async fn lier(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<LienBody>,
) -> Res<Response> {
    let r = state
        .ent
        .lier_personne_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &saisie_lien(b),
        )
        .await
        .map_err(map_db)?;
    Ok(reponse_lien(r))
}

/// Commande unique : personne (nouvelle ou existante) + lien, une seule transaction.
async fn ajouter_responsable(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AjoutResponsableBody>,
) -> Res<Response> {
    let r = state
        .ent
        .ajouter_responsable_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &ent_db::AjoutResponsable {
                nouvelle_personne: b.nouvelle_personne.map(saisie_personne),
                lien: saisie_lien(b.lien),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(reponse_lien(r))
}

/// 201 + responsable (lien actif) ; 202 + `LienEnAttenteDto` (personne avec compte : invitation).
fn reponse_lien(r: LienResultat) -> Response {
    match r {
        LienResultat::Actif(r) => (StatusCode::CREATED, Json(responsable_dto(*r))).into_response(),
        LienResultat::EnAttente { attente_id, .. } => (
            StatusCode::ACCEPTED,
            Json(LienEnAttenteDto {
                en_attente: true,
                attente_id,
            }),
        )
            .into_response(),
    }
}

/// Liens que le personnel a demandés à la personne connectée (elle seule peut les confirmer).
async fn mes_liens_en_attente(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<MonLienEnAttenteDto>>> {
    let v = state
        .ent
        .mes_liens_en_attente(session.etablissement_id, session.user_id)
        .await
        .map_err(map_db)?;
    Ok(Json(
        v.into_iter()
            .map(|l| MonLienEnAttenteDto {
                id: l.id,
                eleve_prenom: l.eleve_prenom,
                eleve_initiale: l.eleve_initiale,
                lien: l.lien,
                demande_le: l.demande_le,
            })
            .collect(),
    ))
}

async fn repondre_lien_en_attente(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ReponseLienBody>,
) -> Res<StatusCode> {
    state
        .ent
        .repondre_lien_en_attente(session.etablissement_id, session.user_id, id, b.accepter)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

/// ADM-82 : le secrétariat confirme un lien en attente au guichet (type de pièce et auteur tracés).
async fn confirmer_guichet(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ConfirmationGuichetBody>,
) -> Res<Json<ConfirmationGuichetDto>> {
    let piece = ent_db::PieceIdentite::parse(b.piece.trim())
        .ok_or((StatusCode::BAD_REQUEST, "type de pièce inconnu".to_string()))?;
    let lien_id = state
        .ent
        .confirmer_lien_guichet_confirme(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            piece,
            b.confirmation_direction,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(ConfirmationGuichetDto { lien_id }))
}

/// Liens en attente de confirmation d'un élève (personnel : proposer le guichet).
async fn liens_en_attente_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<LienEnAttenteEleveDto>>> {
    let v = state
        .ent
        .liens_en_attente_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        v.into_iter()
            .map(|l| LienEnAttenteEleveDto {
                attente_id: l.attente_id,
                personne_id: l.personne_id,
                nom: l.nom,
                prenom: l.prenom,
                a_un_compte: l.a_un_compte,
                lien: l.lien,
                source: l.source,
                demande_le: l.demande_le,
            })
            .collect(),
    ))
}

/// Alertes de protection (adresses parquées, « ne pas solliciter » à confirmer) — direction.
async fn alertes_protection(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<AlertesProtectionDto>> {
    let a = state
        .ent
        .alertes_protection_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(AlertesProtectionDto {
        adresses_a_traiter: a
            .adresses_a_traiter
            .into_iter()
            .map(|x| AdresseParqueeDto {
                id: x.id,
                personne_id: x.personne_id,
                nom: x.nom,
                prenom: x.prenom,
                origine: x.origine,
                created_at: x.created_at,
            })
            .collect(),
        ne_pas_solliciter_a_confirmer: a
            .ne_pas_solliciter_a_confirmer
            .into_iter()
            .map(|x| ProtectionADeclarerDto {
                personne_id: x.personne_id,
                nom: x.nom,
                prenom: x.prenom,
                declaree_le: x.declaree_le,
            })
            .collect(),
    }))
}

/// La direction pose, confirme ou lève « ne pas solliciter » pour une personne.
async fn ne_pas_solliciter(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<NePasSolliciterBody>,
) -> Res<StatusCode> {
    state
        .ent
        .definir_ne_pas_solliciter_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            b.actif,
            b.reference,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

/// La direction ouvre une adresse parquée (POST : jamais préchargée ni mise en cache ; journalisé).
async fn ouvrir_adresse_protegee(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Response> {
    let adresse = state
        .ent
        .ouvrir_adresse_protegee_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    let mut rep = Json(AdresseProtegeeDto { adresse }).into_response();
    rep.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    Ok(rep)
}

async fn modifier_lien(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<LienBody>,
) -> Res<Json<ResponsableDto>> {
    let r = state
        .ent
        .modifier_lien_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &saisie_lien(b),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(responsable_dto(r)))
}

async fn suspendre(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SuspensionBody>,
) -> Res<Json<ResponsableDto>> {
    let r = state
        .ent
        .suspendre_lien_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.motif,
            b.reference.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(responsable_dto(r)))
}

async fn retablir(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ResponsableDto>> {
    let r = state
        .ent
        .retablir_lien_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(responsable_dto(r)))
}

async fn payeurs(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PayeursBody>,
) -> Res<Json<Vec<ResponsableDto>>> {
    let parts: Vec<(Uuid, i16)> = b.payeurs.iter().map(|p| (p.lien_id, p.part)).collect();
    let v = state
        .ent
        .definir_payeurs_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &parts,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(responsable_dto).collect()))
}

async fn ajouter_contact(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ContactBody>,
) -> Res<(StatusCode, Json<ContactUrgenceDto>)> {
    let saisie = ContactSaisie {
        personne_id: b.personne_id,
        nouvelle_personne: b.nouvelle_personne.map(saisie_personne),
        relation: b.relation,
        ordre: b.ordre,
        peut_recuperer: b.peut_recuperer,
        commentaire: b.commentaire,
    };
    let c = state
        .ent
        .ajouter_contact_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &saisie,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(contact_dto(c))))
}

async fn valider_contact(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .valider_contact_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn retirer_contact(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .retirer_contact_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fusionner(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<FusionBody>,
) -> Res<(StatusCode, Json<FusionDto>)> {
    let f = state
        .ent
        .fusionner_personnes_confirme(
            session.etablissement_id,
            session.user_id,
            &session.role,
            b.source_id,
            b.cible_id,
            &b.motif,
            b.confirmation_direction,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(fusion_dto(f))))
}

async fn defusionner(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<FusionDto>> {
    let f = state
        .ent
        .defusionner_personnes_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(fusion_dto(f)))
}

// -- Lot A1 : interdictions et conflits d'intérêts ----------------------------

/// Interdictions actives de l'élève (consigne avec `remise.lire`, fondement avec F3, motif avec F4).
async fn interdictions(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Response> {
    let v = state
        .ent
        .interdictions_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(sans_cache(v.into_iter().map(interdiction_dto).collect::<Vec<_>>()))
}

async fn poser_interdiction(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<InterdictionBody>,
) -> Res<(StatusCode, Json<Uuid>)> {
    let nouvelle = state
        .ent
        .poser_interdiction_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &InterdictionSaisie {
                personne_id: b.personne_id,
                personne_libelle: b.personne_libelle,
                objet: b.objet,
                fondement: b.fondement,
                motif: b.motif,
                consigne: b.consigne,
                du: b.du,
                au: b.au,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(nouvelle)))
}

async fn lever_interdiction(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<LeverInterdictionBody>,
) -> Res<StatusCode> {
    state
        .ent
        .lever_interdiction_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.motif,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Déclaration d'un conflit d'intérêts (l'agent pour lui-même, ou la direction pour un agent).
async fn declarer_conflit_interet(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ConflitInteretBody>,
) -> Res<(StatusCode, Json<Uuid>)> {
    let nouvelle = state
        .ent
        .declarer_conflit_interet_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            b.agent_user_id,
            id,
            &b.nature,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(nouvelle)))
}

async fn lever_conflit_interet(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .lever_conflit_interet_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

fn conflit_dto(c: ConflitInteretVu) -> ConflitInteretDto {
    ConflitInteretDto {
        declaration_id: c.declaration_id,
        user_id: c.user_id,
        eleve_id: c.eleve_id,
        origine: c.origine,
        nature: c.nature,
        acces_30j: c.acces_30j,
        dernier_acces: c.dernier_acces,
    }
}

/// Tableau de la direction : conflits déclarés et accès tracés.
async fn conflits_interet(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Response> {
    let v = state
        .ent
        .conflits_interet_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(sans_cache(v.into_iter().map(conflit_dto).collect::<Vec<_>>()))
}
