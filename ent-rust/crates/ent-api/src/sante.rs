//! Routes de la santé et de l'accompagnement (socle 2, lot S2-11).
//!
//! L'autorité est `ent-db` (moteur de droits D-017 : `sante.*`, `accompagnement.gerer`,
//! `amenagements.lire`, `contraintes.lire`, `sante.urgence_lire`) : ces routes ne font
//! que traduire HTTP ↔ service. L'établissement vient de la session, jamais du client.
//! Une référence d'un autre établissement répond 404.
//!
//! Minimisation : les listes destinées aux professeurs, à la restauration et aux
//! sorties ne portent que des codes du catalogue fermé et leur libellé neutre.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::{NaiveDate, NaiveTime};
use domain_eleve::UpsertPai;
use ent_db::accompagnement::{
    AeshAffectation, AffecterAesh, Amenagement, AmenagementsEleve, CATALOGUE_AMENAGEMENTS,
    DefinirAmenagement, DispositifARevoir, DispositifVue, DossierAccompagnement, OuvrirDispositif,
    PlanifierReunion, ReunionEquipe,
};
use ent_db::sante_acc::{
    Audience, ClorePassage, ContraintePubliee, ContraintesEleve, EnregistrerProtocole, ExtrasPai,
    FicheUrgence, NouveauPassage, PaiARevoir, PaiComplet, PassageSante, ProtocoleSante,
    PublierContrainte, CATALOGUE_CONTRAINTES, ISSUES,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sante/catalogue", get(catalogue))
        // Infirmerie
        .route("/sante/passages", get(passages).post(creer_passage))
        .route("/sante/passages/{id}/cloturer", post(cloturer_passage))
        .route("/sante/passages/{id}/informer-famille", post(informer_famille))
        .route("/sante/pai-a-reviser", get(pai_a_reviser))
        .route("/eleves/{id}/pai-complet", get(pai_complet).put(enregistrer_pai))
        .route("/eleves/{id}/pai/signatures", post(signer_pai))
        .route("/eleves/{id}/fiche-urgence", get(fiche_urgence))
        .route("/sante/protocoles", get(protocoles).put(enregistrer_protocole))
        // Contraintes publiées
        .route("/sante/contraintes", get(contraintes_liste))
        .route("/sante/contraintes/{id}", delete(retirer_contrainte))
        .route(
            "/eleves/{id}/contraintes",
            get(contraintes_eleve).post(publier_contrainte),
        )
        // Accompagnement
        .route("/accompagnement/mes-amenagements", get(mes_amenagements))
        .route("/accompagnement/a-reviser", get(dispositifs_a_reviser))
        .route("/accompagnement/dispositifs/{id}/reviser", post(reviser_dispositif))
        .route("/accompagnement/amenagements/{id}", delete(retirer_amenagement))
        .route("/accompagnement/reunions/{id}/compte-rendu", post(compte_rendu))
        .route("/eleves/{id}/accompagnement", get(dossier_accompagnement))
        .route("/eleves/{id}/dispositifs", post(ouvrir_dispositif))
        .route(
            "/eleves/{id}/amenagements",
            get(amenagements_eleve).post(definir_amenagement),
        )
        .route("/eleves/{id}/aesh", post(affecter_aesh))
        .route("/eleves/{id}/reunions", post(planifier_reunion))
}

// ---------------------------------------------------------------------------
// Catalogues (données statiques, sans information d'élève)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct EntreeCat {
    code: &'static str,
    libelle: &'static str,
    audiences: Vec<&'static str>,
}

#[derive(Serialize)]
struct Catalogue {
    contraintes: Vec<EntreeCat>,
    amenagements: Vec<EntreeCat>,
    issues: Vec<&'static str>,
    parties_pai: Vec<&'static str>,
}

async fn catalogue(_session: EntSession) -> Json<Catalogue> {
    Json(Catalogue {
        contraintes: CATALOGUE_CONTRAINTES
            .iter()
            .map(|e| EntreeCat {
                code: e.code,
                libelle: e.libelle,
                audiences: e.audiences.iter().map(|a| a.as_str()).collect(),
            })
            .collect(),
        amenagements: CATALOGUE_AMENAGEMENTS
            .iter()
            .map(|e| EntreeCat {
                code: e.code,
                libelle: e.libelle,
                audiences: Vec::new(),
            })
            .collect(),
        issues: ISSUES.to_vec(),
        parties_pai: ent_db::sante_acc::PARTIES_PAI.to_vec(),
    })
}

fn audience(valeur: &str) -> Res<Audience> {
    Audience::parse(valeur)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "audience inconnue".to_string()))
}

// ---------------------------------------------------------------------------
// Passages
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct PassagesQuery {
    eleve_id: Option<Uuid>,
    jour: Option<NaiveDate>,
}

async fn passages(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<PassagesQuery>,
) -> Res<Json<Vec<PassageSante>>> {
    state
        .ent
        .passages_sante_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.eleve_id,
            q.jour,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct PassageBody {
    eleve_id: Uuid,
    jour: NaiveDate,
    heure_arrivee: Option<NaiveTime>,
    motif: String,
    #[serde(default)]
    note_soins: String,
    #[serde(default)]
    soins: String,
    #[serde(default)]
    visible_famille: bool,
    #[serde(default = "issue_defaut")]
    issue: String,
    #[serde(default)]
    informer_famille: bool,
}

fn issue_defaut() -> String {
    "non_renseignee".into()
}

async fn creer_passage(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<PassageBody>,
) -> Res<(StatusCode, Json<PassageSante>)> {
    state
        .ent
        .enregistrer_passage_for_actor(
            session.user_id,
            &session.role,
            NouveauPassage {
                etablissement_id: session.etablissement_id,
                eleve_id: b.eleve_id,
                jour: b.jour,
                heure_arrivee: b.heure_arrivee,
                motif: b.motif,
                note_soins: b.note_soins,
                soins: b.soins,
                visible_famille: b.visible_famille,
                issue: b.issue,
                informer_famille: b.informer_famille,
            },
        )
        .await
        .map(|p| (StatusCode::CREATED, Json(p)))
        .map_err(map_db)
}

#[derive(Deserialize)]
struct ClotureBody {
    heure_sortie: Option<NaiveTime>,
    issue: String,
    soins: Option<String>,
    recupere_par_personne_id: Option<Uuid>,
}

async fn cloturer_passage(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ClotureBody>,
) -> Res<Json<PassageSante>> {
    state
        .ent
        .cloturer_passage_for_actor(
            session.user_id,
            &session.role,
            ClorePassage {
                etablissement_id: session.etablissement_id,
                passage_id: id,
                heure_sortie: b.heure_sortie,
                issue: b.issue,
                soins: b.soins,
                recupere_par_personne_id: b.recupere_par_personne_id,
            },
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn informer_famille(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<PassageSante>> {
    state
        .ent
        .informer_famille_passage_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

// ---------------------------------------------------------------------------
// PAI
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct HorizonQuery {
    horizon: Option<i32>,
}

async fn pai_a_reviser(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<HorizonQuery>,
) -> Res<Json<Vec<PaiARevoir>>> {
    state
        .ent
        .pai_a_reviser_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.horizon.unwrap_or(30),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn pai_complet(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Option<PaiComplet>>> {
    state
        .ent
        .pai_complet_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct PaiBody {
    motif: String,
    mesures_accueil: String,
    #[serde(default)]
    protocole_urgence: String,
    #[serde(default)]
    note_interne: String,
    date_debut: NaiveDate,
    date_fin: Option<NaiveDate>,
    #[serde(default = "vrai")]
    actif: bool,
    date_revision: Option<NaiveDate>,
    #[serde(default)]
    trousse_urgence: bool,
    #[serde(default)]
    trousse_lieu: String,
    trousse_peremption: Option<NaiveDate>,
}

fn vrai() -> bool {
    true
}

async fn enregistrer_pai(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PaiBody>,
) -> Res<Json<Option<PaiComplet>>> {
    let (etab, user) = (session.etablissement_id, session.user_id);
    state
        .ent
        .enregistrer_pai_for_actor(
            &session.role,
            UpsertPai {
                etablissement_id: etab,
                eleve_id: id,
                motif: b.motif,
                mesures_accueil: b.mesures_accueil,
                protocole_urgence: b.protocole_urgence,
                note_interne: b.note_interne,
                date_debut: b.date_debut,
                date_fin: b.date_fin,
                actif: b.actif,
                actor_user_id: user,
            },
            Some(ExtrasPai {
                date_revision: b.date_revision,
                trousse_urgence: b.trousse_urgence,
                trousse_lieu: b.trousse_lieu.trim().to_string(),
                trousse_peremption: b.trousse_peremption,
            }),
        )
        .await
        .map_err(map_db)?;
    state
        .ent
        .pai_complet_for_actor(etab, user, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct SignatureBody {
    partie: String,
    signataire_nom: Option<String>,
}

async fn signer_pai(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SignatureBody>,
) -> Res<Json<PaiComplet>> {
    state
        .ent
        .signer_pai_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.partie,
            b.signataire_nom.as_deref(),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn fiche_urgence(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<FicheUrgence>> {
    state
        .ent
        .fiche_urgence_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn protocoles(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<ProtocoleSante>>> {
    state
        .ent
        .protocoles_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct ProtocoleBody {
    id: Option<Uuid>,
    titre: String,
    categorie: String,
    contenu: String,
    #[serde(default = "vrai")]
    actif: bool,
}

async fn enregistrer_protocole(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<ProtocoleBody>,
) -> Res<Json<ProtocoleSante>> {
    state
        .ent
        .enregistrer_protocole_for_actor(
            session.user_id,
            &session.role,
            EnregistrerProtocole {
                etablissement_id: session.etablissement_id,
                id: b.id,
                titre: b.titre,
                categorie: b.categorie,
                contenu: b.contenu,
                actif: b.actif,
            },
        )
        .await
        .map(Json)
        .map_err(map_db)
}

// ---------------------------------------------------------------------------
// Contraintes publiées
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct AudienceQuery {
    audience: Option<String>,
}

async fn contraintes_liste(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<AudienceQuery>,
) -> Res<Json<Vec<ContraintesEleve>>> {
    let a = audience(q.audience.as_deref().unwrap_or(""))?;
    state
        .ent
        .contraintes_liste_for_actor(session.etablissement_id, session.user_id, &session.role, a)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn contraintes_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<AudienceQuery>,
) -> Res<Json<Vec<ContraintePubliee>>> {
    let a = q.audience.as_deref().map(audience).transpose()?;
    state
        .ent
        .contraintes_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id, a)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct ContrainteBody {
    code: String,
    audiences: Vec<String>,
    date_fin: Option<NaiveDate>,
}

async fn publier_contrainte(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ContrainteBody>,
) -> Res<(StatusCode, Json<ContraintePubliee>)> {
    let audiences = b
        .audiences
        .iter()
        .map(|a| audience(a))
        .collect::<Res<Vec<_>>>()?;
    state
        .ent
        .publier_contrainte_for_actor(
            session.user_id,
            &session.role,
            PublierContrainte {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                code: b.code,
                audiences,
                date_fin: b.date_fin,
            },
        )
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(map_db)
}

async fn retirer_contrainte(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .retirer_contrainte_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

// ---------------------------------------------------------------------------
// Accompagnement
// ---------------------------------------------------------------------------

async fn mes_amenagements(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<AmenagementsEleve>>> {
    state
        .ent
        .mes_amenagements_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn dispositifs_a_reviser(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<HorizonQuery>,
) -> Res<Json<Vec<DispositifARevoir>>> {
    state
        .ent
        .dispositifs_a_reviser_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.horizon.unwrap_or(30),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn dossier_accompagnement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<DossierAccompagnement>> {
    state
        .ent
        .dossier_accompagnement_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct DispositifBody {
    kind: String,
    libelle: Option<String>,
    document_id: Option<Uuid>,
    date_debut: Option<NaiveDate>,
    date_revision: Option<NaiveDate>,
}

async fn ouvrir_dispositif(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DispositifBody>,
) -> Res<(StatusCode, Json<DispositifVue>)> {
    state
        .ent
        .ouvrir_dispositif_for_actor(
            session.user_id,
            &session.role,
            OuvrirDispositif {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                kind: b.kind,
                libelle: b.libelle,
                document_id: b.document_id,
                date_debut: b.date_debut,
                date_revision: b.date_revision,
            },
        )
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

#[derive(Deserialize)]
struct RevisionBody {
    date_revision: Option<NaiveDate>,
    #[serde(default)]
    clore: bool,
}

async fn reviser_dispositif(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RevisionBody>,
) -> Res<Json<DispositifVue>> {
    state
        .ent
        .reviser_dispositif_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            b.date_revision,
            b.clore,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn amenagements_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<Amenagement>>> {
    state
        .ent
        .amenagements_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct AmenagementBody {
    code: String,
    portee: String,
    dispositif_id: Option<Uuid>,
    date_fin: Option<NaiveDate>,
}

async fn definir_amenagement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AmenagementBody>,
) -> Res<(StatusCode, Json<Amenagement>)> {
    state
        .ent
        .definir_amenagement_for_actor(
            session.user_id,
            &session.role,
            DefinirAmenagement {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                dispositif_id: b.dispositif_id,
                code: b.code,
                portee: b.portee,
                date_fin: b.date_fin,
            },
        )
        .await
        .map(|a| (StatusCode::CREATED, Json(a)))
        .map_err(map_db)
}

async fn retirer_amenagement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .retirer_amenagement_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct AeshBody {
    aesh_nom: String,
    heures_hebdo: Option<i16>,
    dispositif_id: Option<Uuid>,
    date_debut: NaiveDate,
    date_fin: Option<NaiveDate>,
}

async fn affecter_aesh(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AeshBody>,
) -> Res<(StatusCode, Json<AeshAffectation>)> {
    state
        .ent
        .affecter_aesh_for_actor(
            session.user_id,
            &session.role,
            AffecterAesh {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                dispositif_id: b.dispositif_id,
                aesh_nom: b.aesh_nom,
                heures_hebdo: b.heures_hebdo,
                date_debut: b.date_debut,
                date_fin: b.date_fin,
            },
        )
        .await
        .map(|a| (StatusCode::CREATED, Json(a)))
        .map_err(map_db)
}

#[derive(Deserialize)]
struct ReunionBody {
    kind: String,
    date_reunion: NaiveDate,
    dispositif_id: Option<Uuid>,
}

async fn planifier_reunion(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ReunionBody>,
) -> Res<(StatusCode, Json<ReunionEquipe>)> {
    state
        .ent
        .planifier_reunion_for_actor(
            session.user_id,
            &session.role,
            PlanifierReunion {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                dispositif_id: b.dispositif_id,
                kind: b.kind,
                date_reunion: b.date_reunion,
            },
        )
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

#[derive(Deserialize)]
struct CompteRenduBody {
    compte_rendu: String,
}

async fn compte_rendu(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CompteRenduBody>,
) -> Res<StatusCode> {
    state
        .ent
        .consigner_reunion_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.compte_rendu,
        )
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}
