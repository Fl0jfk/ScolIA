//! Socle 2 (l'élève), lot E8 — routes des pastilles et de la synthèse de la fiche (le service
//! d'E9b). `GET /eleves/{id}/synthese`, `GET /eleves/{id}/pastilles`, `GET /eleves/pastilles`
//! (compteurs), `GET /eleves/pastilles/catalogue`, `GET /eleves/pastilles/{code}` (liste de travail).
//!
//! Personnel seulement. L'autorité est `ent-db` : une pastille ne se montre que si l'acteur détient,
//! sur cet élève, l'une des actions du catalogue ; un élève absent, hors de la portée de l'annuaire
//! ou protégé pour ce tiers répond `404`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use domain_eleve::{CATALOGUE_INDICATEURS, CONTRATS_TUILES};
use ent_db::eleve_indicateurs::{EtatTuile, PastilleVue, SyntheseEleve};
use ent_types::eleve_synthese as dto;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eleves/liste", get(liste))
        .route("/eleves/pastilles", get(compteurs))
        .route("/eleves/pastilles/catalogue", get(catalogue))
        .route("/eleves/pastilles/{code}", get(liste_par_pastille))
        .route("/eleves/{id}/synthese", get(synthese))
        .route("/eleves/{id}/pastilles", get(pastilles))
}

#[derive(Debug, Deserialize)]
struct LimiteQuery {
    limite: Option<i64>,
}

fn pastille_dto(p: &PastilleVue) -> dto::PastilleDto {
    dto::PastilleDto {
        code: p.code.to_string(),
        domaine: p.domaine.to_string(),
        libelle: p.libelle.to_string(),
        icone: p.icone.to_string(),
        gravite: p.gravite.code().to_string(),
        valeur_code: p.valeur_code.clone(),
        valeur_date: p.valeur_date,
        du: p.du,
    }
}

fn synthese_dto(s: &SyntheseEleve) -> dto::SyntheseDto {
    dto::SyntheseDto {
        eleve_id: s.eleve_id,
        pastilles: s.pastilles.iter().map(pastille_dto).collect(),
        tuiles: s
            .tuiles
            .iter()
            .map(|t| dto::TuileDto {
                domaine: t.domaine.to_string(),
                libelle: t.libelle.to_string(),
                rang: t.rang,
                lien: t.lien.clone(),
                etat: match t.etat {
                    EtatTuile::Visible => "visible",
                    EtatTuile::Masquee(_) => "masquee",
                }
                .to_string(),
                raison: match t.etat {
                    EtatTuile::Visible => None,
                    EtatTuile::Masquee(r) => Some(r.code().to_string()),
                },
                pastilles: t.pastilles.iter().map(pastille_dto).collect(),
            })
            .collect(),
    }
}

#[derive(Debug, Deserialize)]
struct ListeQuery {
    q: Option<String>,
    classe: Option<String>,
    statut: Option<String>,
    pastille: Option<String>,
    limite: Option<i64>,
    decalage: Option<i64>,
}

/// `GET /eleves/liste` — une page de la liste des élèves (lot E9a). Même annuaire, même portée et mêmes
/// protégés écartés que `GET /eleves/search` ; les filtres ne portent que sur ce que le lecteur voit.
async fn liste(State(state): State<AppState>, session: EntSession, Query(q): Query<ListeQuery>) -> Res<Json<dto::EleveListeDto>> {
    session.require_staff()?;
    let vide = |s: &Option<String>| s.as_deref().map(str::trim).filter(|x| !x.is_empty()).map(str::to_string);
    let (classe, statut, pastille) = (vide(&q.classe), vide(&q.statut), vide(&q.pastille));
    let l = state
        .ent
        .lister_eleves_pour_acteur(
            &session.role,
            session.etablissement_id,
            session.user_id,
            &ent_db::FiltreListeEleves {
                q: q.q.as_deref().unwrap_or("").trim(),
                classe: classe.as_deref(),
                statut: statut.as_deref(),
                pastille: pastille.as_deref(),
                limite: q.limite.unwrap_or(50),
                decalage: q.decalage.unwrap_or(0),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(dto::EleveListeDto {
        eleves: l
            .eleves
            .iter()
            .map(|v| dto::EleveLigneDto {
                id: v.eleve.id,
                prenom: v.eleve.prenom.clone(),
                nom: v.eleve.nom.clone(),
                classe: v.eleve.classe.clone(),
                statut: v.eleve.status.as_str().to_string(),
            })
            .collect(),
        total: l.total,
        decalage: l.decalage,
        limite: l.limite,
        tronque: l.tronque,
        portee: if l.portee_etendue { "etablissement" } else { "perimetre" }.to_string(),
        classes: l.classes,
    }))
}

/// `GET /eleves/{id}/synthese` — pastilles visibles et tuiles par domaine.
async fn synthese(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<dto::SyntheseDto>> {
    session.require_staff()?;
    let s = state
        .ent
        .synthese_eleve_pour_acteur(&session.role, session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(synthese_dto(&s)))
}

/// `GET /eleves/{id}/pastilles` — pastilles visibles seulement (en-tête de la fiche).
async fn pastilles(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<dto::PastilleDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .pastilles_eleve_pour_acteur(&session.role, session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(pastille_dto).collect()))
}

/// `GET /eleves/pastilles` — combien d'élèves par pastille (échelle de l'établissement).
async fn compteurs(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<dto::CompteurPastilleDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .compteurs_pastilles_pour_acteur(&session.role, session.etablissement_id, session.user_id)
        .await
        .map_err(map_db)?;
    Ok(Json(
        l.into_iter()
            .filter_map(|(code, nombre)| {
                let d = domain_eleve::indicateur_catalogue(code)?;
                Some(dto::CompteurPastilleDto {
                    code: code.to_string(),
                    libelle: d.libelle.to_string(),
                    gravite: d.gravite.code().to_string(),
                    nombre,
                })
            })
            .collect(),
    ))
}

/// `GET /eleves/pastilles/catalogue` — le catalogue du logiciel et le contrat des tuiles (aucune
/// donnée d'élève : lisible de tout le personnel).
async fn catalogue(session: EntSession) -> Res<Json<dto::CatalogueDto>> {
    session.require_staff()?;
    Ok(Json(dto::CatalogueDto {
        indicateurs: CATALOGUE_INDICATEURS
            .iter()
            .map(|d| dto::IndicateurCatalogueDto {
                code: d.code.to_string(),
                domaine: d.domaine.to_string(),
                libelle: d.libelle.to_string(),
                icone: d.icone.to_string(),
                gravite: d.gravite.code().to_string(),
            })
            .collect(),
        tuiles: CONTRATS_TUILES
            .iter()
            .map(|c| dto::ContratTuileDto {
                domaine: c.domaine.to_string(),
                libelle: c.libelle.to_string(),
                rang: c.rang,
                personnel_seul: c.personnel_seul,
            })
            .collect(),
    }))
}

/// `GET /eleves/pastilles/{code}` — liste de travail (« sans INE », « PAI en cours »…).
async fn liste_par_pastille(
    State(state): State<AppState>,
    session: EntSession,
    Path(code): Path<String>,
    Query(q): Query<LimiteQuery>,
) -> Res<Json<dto::ElevesPastilleDto>> {
    session.require_staff()?;
    let eleves = state
        .ent
        .eleves_par_pastille_pour_acteur(&session.role, session.etablissement_id, session.user_id, &code, q.limite.unwrap_or(500))
        .await
        .map_err(map_db)?;
    Ok(Json(dto::ElevesPastilleDto { code, eleves }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::EleveDto;
    use chrono::NaiveDate;
    use domain_eleve::MasqueChamps;

    /// Le JSON que sert `GET /eleves/{id}` se relit tel quel dans le type partagé du frontend
    /// (`EleveFicheDto`) : les deux formes ne dérivent pas, y compris les masquages expliqués.
    #[test]
    fn le_json_de_l_eleve_se_relit_dans_le_type_partage_du_frontend() {
        let id = Uuid::new_v4();
        let serveur = EleveDto {
            id,
            etablissement_id: Uuid::new_v4(),
            nom: "Martin".into(),
            prenom: "Léa".into(),
            ine: None,
            sexe: Some("F".into()),
            date_naissance: NaiveDate::from_ymd_opt(2012, 3, 14),
            numero_badge: None,
            redoublant: None,
            status: "actif".into(),
            classe: Some("6A".into()),
            regime: None,
            mef: None,
            site: None,
            date_entree: None,
            date_sortie: None,
            motif_sortie: None,
            masques: vec![
                MasqueChamps {
                    groupe: "identifiants",
                    raison: "droit_manquant",
                    action: Some("eleves.identifiants.lire"),
                    message: "Identifiants non ouverts".into(),
                },
                MasqueChamps { groupe: "sensible", raison: "reserve_au_personnel", action: None, message: "Réservé".into() },
            ],
        };
        let json = serde_json::to_value(&serveur).unwrap();
        let client: ent_types::eleve_synthese::EleveFicheDto = serde_json::from_value(json.clone()).unwrap();
        assert_eq!((client.id, client.nom.as_str(), client.prenom.as_str()), (id, "Martin", "Léa"));
        assert_eq!(client.classe.as_deref(), Some("6A"));
        assert_eq!(client.status, "actif");
        assert_eq!(client.redoublant, None, "donnée non communiquée, pas un faux « non »");
        assert_eq!(client.masques.len(), 2);
        assert_eq!(client.masques[0].action.as_deref(), Some("eleves.identifiants.lire"));
        assert_eq!(client.masques[1].action, None);
        // Aller-retour : rien n'est perdu ni inventé pour les champs que le client reprend.
        let retour = serde_json::to_value(&client).unwrap();
        for cle in ["id", "nom", "prenom", "sexe", "date_naissance", "status", "classe"] {
            assert_eq!(retour.get(cle), json.get(cle), "{cle}");
        }
    }

    #[test]
    fn la_liste_se_relit_et_ne_porte_que_la_designation() {
        let l = dto::EleveListeDto {
            eleves: vec![dto::EleveLigneDto {
                id: Uuid::new_v4(),
                prenom: "Léa".into(),
                nom: "Martin".into(),
                classe: Some("6A".into()),
                statut: "actif".into(),
            }],
            total: 1,
            decalage: 0,
            limite: 50,
            tronque: false,
            portee: "etablissement".into(),
            classes: vec!["6A".into()],
        };
        let v = serde_json::to_value(&l).unwrap();
        let ligne = v["eleves"][0].as_object().unwrap();
        // Ni date, ni identifiant national, ni photo : seulement ce qu'une ligne de liste montre.
        let mut cles: Vec<_> = ligne.keys().cloned().collect();
        cles.sort();
        assert_eq!(cles, ["classe", "id", "nom", "prenom", "statut"]);
        let relu: dto::EleveListeDto = serde_json::from_value(v).unwrap();
        assert_eq!(relu, l);
    }
}
