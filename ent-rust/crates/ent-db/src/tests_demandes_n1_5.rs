//! Socle N1 (demandes), lot N1-5 : **reprise R9** des données du Next.js, contre la base locale de test (feature `labo`) ;
//! sans `ENT_DATABASE_URL`, les tests sont ignorés en local (échec explicite en CI). Chaque test crée son propre
//! établissement et ne vérifie que ses propres lignes. Aucun test ne dépend de l'heure réelle : le jour est celui de
//! l'établissement. **Aucune donnée réelle** : l'export est un jeu fictif au format décrit dans
//! `docs/socles/N1_5_LIVRAISON.md`, construit ici avec des identifiants aléatoires de 12 caractères au moins.
//!
//! | Critère (§29.2, §32 point 6) | Test(s) |
//! |---|---|
//! | 100 % des demandes ouvertes reprises et attribuées à une file | `t_reprise_demandes_ouvertes_et_closes` |
//! | Rejouer deux fois = même résultat | `t_reprise_rejouable` |
//! | CDM-50 : 0 heure HSE créée au 15, aucune séance possible | `t_reprise_hse_cdm50` |
//! | Photocopies : ouvertes seulement | `t_reprise_photocopies_ouvertes` |
//! | Organisation : proposition, jamais appliquée | `t_reprise_organisation_en_proposition` |
//! | NJ-84, NJ-88 : aucun rattachement par e-mail, adresse masquée, rapprochement par le secrétariat | `t_reprise_demandeur_sans_email` |
//! | DEM-349 : guichet en lecture seule | `t_guichet_lecture_seule` |
//! | R-DM49 : aucun avis par la reprise | `t_reprise_demandes_ouvertes_et_closes` |
//! | Droit `demandes.reprise` (administrateur seulement), export illisible | `t_reprise_droit_et_export_refuses` |
//! | D-036 : pas de JSON, de tableau ni d'adresse en clair | `t_tables_n1_5_sans_json_ni_adresse` |

use super::*;
use crate::demandes::*;
use crate::demandes_reprise::*;
use crate::tests_demandes::{fiche, labo, monde, Acteur, Monde};
use chrono::{Duration, NaiveDate};
use ent_auth::Role;
use serde_json::{json, Value};
use uuid::Uuid;

fn code(prefixe: &str) -> String {
    format!("{prefixe}{}", &Uuid::new_v4().simple().to_string()[..12])
}

fn instant(jour: NaiveDate) -> String {
    format!("{jour}T09:00:00Z")
}

struct Pieces;

impl PortPiecesReprise for Pieces {
    fn copier(&self, _: Uuid, _: &PieceSource<'_>) -> Option<DocumentRepris> {
        Some(DocumentRepris {
            document_id: Uuid::new_v4(),
            pages: Some(3),
        })
    }
}

#[derive(Clone)]
struct SourcesFixes(Uuid);

impl SourcesDemandes for SourcesFixes {
    fn file_de_depot(&self, _: Uuid, _: &str) -> Option<Uuid> {
        Some(self.0)
    }
    fn files_de(&self, _: Uuid, _: Uuid) -> Vec<Uuid> {
        vec![self.0]
    }
    fn peut_deposer_famille(&self, _: Uuid, _: Uuid, _: Role, _: Option<Uuid>) -> bool {
        false
    }
}

struct R {
    ent: PgEnt,
    m: Monde,
    admin: Acteur,
    jour: NaiveDate,
    debut_annee: NaiveDate,
    email_prof: String,
}

impl R {
    async fn nouveau() -> Option<R> {
        let (ent, cast) = labo().await?;
        let m = monde(&ent, &cast).await;
        let admin = fiche(&ent, m.etab, cast.admin.user_id, "admin", true).await;
        let pool = ent.pool_verif();
        let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())")
            .bind(m.etab)
            .fetch_one(pool)
            .await
            .unwrap();
        let (debut_annee,): (NaiveDate,) = sqlx::query_as(
            "SELECT date_debut FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND (courante OR (date_debut <= $2 AND date_fin >= $2))
              ORDER BY courante DESC, date_debut DESC LIMIT 1",
        )
        .bind(m.etab)
        .bind(jour)
        .fetch_one(pool)
        .await
        .unwrap();
        // Les adresses des personnels sont celles de leurs fiches : de quoi retrouver un agent (jamais un demandeur).
        let email_prof = format!("{}@exemple.invalid", code("prof"));
        sqlx::query("UPDATE ent_personne SET email = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(m.etab)
            .bind(m.prof.personne)
            .bind(&email_prof)
            .execute(pool)
            .await
            .unwrap();
        let r = R {
            ent,
            m,
            admin,
            jour,
            debut_annee,
            email_prof,
        };
        // Les files par défaut (dont la file de tri) : sans elles, rien n'est routé.
        r.ent
            .initialiser_files_defaut_for_actor(r.m.etab, r.m.direction.user, "direction")
            .await
            .expect("préréglages des files");
        Some(r)
    }

    fn etab(&self) -> Uuid {
        self.m.etab
    }

    fn comptes(&self) -> Value {
        json!([
            {"user_ref": "u_prof", "user_id": self.m.prof.user},
            {"user_ref": "u_dir", "user_id": self.m.direction.user},
            {"user_ref": "u_inconnu", "user_id": null}
        ])
    }

    fn export(&self, requests: Value, hse: Value, photocopies: Value, org: Value) -> String {
        json!({
            "format": FORMAT_EXPORT,
            "version": VERSION_EXPORT,
            "comptes": self.comptes(),
            "requests": requests,
            "hse": hse,
            "photocopies": photocopies,
            "routing": {"tasks": [{"id": "maintenance", "label": "Maintenance"}, {"id": "cpe_3e4e", "label": "CPE 3e-4e"}]},
            "org": org,
        })
        .to_string()
    }

    async fn reprendre(&self, texte: &str, pieces: &dyn PortPiecesReprise) -> RapportReprise {
        self.ent
            .reprendre_for_actor(
                self.etab(),
                self.admin.user,
                "admin",
                ModeReprise::Repetition,
                texte,
                pieces,
            )
            .await
            .expect("reprise")
    }

    async fn nb(&self, sql: &str, bind: Option<Uuid>) -> i64 {
        let mut q = sqlx::query_as::<_, (i64,)>(sql).bind(self.etab());
        if let Some(b) = bind {
            q = q.bind(b);
        }
        q.fetch_one(self.ent.pool_verif()).await.unwrap().0
    }

    /// La demande reprise d'un objet source (cible de sa correspondance).
    async fn cible(&self, source: &str, source_id: &str) -> Option<Uuid> {
        sqlx::query_as::<_, (Option<Uuid>,)>(
            "SELECT cible_id FROM ent_dem_reprise_correspondance WHERE etablissement_id = $1 AND source = $2 AND source_id = $3",
        )
        .bind(self.etab())
        .bind(source)
        .bind(source_id)
        .fetch_optional(self.ent.pool_verif())
        .await
        .unwrap()
        .and_then(|(c,)| c)
    }

    async fn decision(&self, source: &str, source_id: &str) -> Option<(String, Option<String>)> {
        sqlx::query_as(
            "SELECT decision, raison_code FROM ent_dem_reprise_correspondance
              WHERE etablissement_id = $1 AND source = $2 AND source_id = $3",
        )
        .bind(self.etab())
        .bind(source)
        .bind(source_id)
        .fetch_optional(self.ent.pool_verif())
        .await
        .unwrap()
    }

    async fn ligne(
        &self,
        id: Uuid,
    ) -> (
        String,
        Option<String>,
        String,
        Option<Uuid>,
        Option<NaiveDate>,
        String,
    ) {
        sqlx::query_as(
            "SELECT statut, issue, nature_code, attribuee_a_agent_id, conservation_jusqu_au, canal_depot
               FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(self.etab())
        .bind(id)
        .fetch_one(self.ent.pool_verif())
        .await
        .unwrap()
    }
}

fn requete(id: &str, statut: &str, route: &str, jour: NaiveDate) -> Value {
    json!({
        "id": id,
        "created_at": instant(jour - Duration::days(7)),
        "updated_at": instant(jour - Duration::days(1)),
        "status": statut,
        "category": "",
        "subject": format!("Sujet {id}"),
        "description": "Description fictive de la demande.",
        "requester_user_id": "u_prof",
        "requester_email": "",
        "assigned_unit": route,
        "assigned_email": "",
        "assigned_route_id": route,
        "attrs": [], "comments": [], "history": [], "attachments": []
    })
}

// ───────────────────────── Fonctions pures ─────────────────────────

#[test]
fn t_reprise_fonctions_pures() {
    assert_eq!(
        masquer_adresse("jean.dupont@exemple.fr").as_deref(),
        Some("j***@e***.fr")
    );
    assert_eq!(masquer_adresse("pas une adresse"), None);
    assert_eq!(
        nature_depuis_route("maintenance"),
        ("maintenance.intervention", false)
    );
    assert_eq!(
        nature_depuis_route("comptabilite"),
        ("comptabilite.demande_interne", false)
    );
    assert_eq!(
        nature_depuis_route("admin_secretariat"),
        ("secretariat.demande_interne", false)
    );
    assert_eq!(
        nature_depuis_route("accueil"),
        ("secretariat.demande_interne", false)
    );
    assert_eq!(
        nature_depuis_route("rh_personnel"),
        ("rh.demande_agent", false)
    );
    assert_eq!(
        nature_depuis_route("cpe_3e4e"),
        ("vie_scolaire.demande_interne", false)
    );
    assert_eq!(
        nature_depuis_route("vie_scolaire_lycee"),
        ("vie_scolaire.demande_interne", false)
    );
    // La corbeille et toute route inconnue vont à la file de tri : jamais perdues.
    assert_eq!(nature_depuis_route("corbeille").1, true);
    assert_eq!(nature_depuis_route("route_jamais_vue").1, true);
    assert!(horodatage("2026-11-02T09:00:00Z").is_some());
    assert!(horodatage("2026-11-02").is_some());
    assert!(horodatage("n'importe quoi").is_none());
    assert!(horodatage("").is_none());
}

// ───────────────────────── Demandes (request) ─────────────────────────

#[tokio::test]
async fn t_reprise_demandes_ouvertes_et_closes() {
    let Some(r) = R::nouveau().await else { return };
    let (
        i_nouvelle,
        i_cours,
        i_attente,
        i_corbeille,
        i_inconnue,
        i_close,
        i_echue,
        i_sans,
        i_statut,
    ) = (
        code("rq"),
        code("rq"),
        code("rq"),
        code("rq"),
        code("rq"),
        code("rq"),
        code("rq"),
        code("rq"),
        code("rq"),
    );
    let mut nouvelle = requete(&i_nouvelle, "NOUVELLE", "maintenance", r.jour);
    nouvelle["attrs"] =
        json!([{"path": "lieu", "value": "Salle 12"}, {"path": "gravite", "value": "gene"}]);
    nouvelle["comments"] = json!([
        {"id": code("c"), "at": instant(r.jour - Duration::days(6)), "by_name": "Agent A", "by_user_id": "u_dir",
         "body": "Commentaire interne fictif.", "visible_demandeur": false},
        {"id": code("c"), "at": instant(r.jour - Duration::days(5)), "by_name": "Agent B", "by_user_id": null,
         "body": "Réponse au demandeur.", "visible_demandeur": true}
    ]);
    nouvelle["history"] = json!([
        {"at": instant(r.jour - Duration::days(6)), "action": "Créée"},
        {"at": instant(r.jour - Duration::days(5)), "action": "Assignée à l'unité"}
    ]);
    nouvelle["attachments"] = json!([{"id": code("pj"), "file_name": "photo.jpg", "s3_key": "requests/x/photo.jpg", "content_type": "image/jpeg"}]);
    let mut en_cours = requete(&i_cours, "EN_COURS", "maintenance", r.jour);
    en_cours["assigned_email"] = json!(r.email_prof.to_uppercase());
    let attente = requete(&i_attente, "EN_ATTENTE", "comptabilite", r.jour);
    let corbeille = requete(&i_corbeille, "NOUVELLE", "corbeille", r.jour);
    let inconnue = requete(&i_inconnue, "EN_COURS", "route_jamais_vue", r.jour);
    let mut close = requete(&i_close, "TERMINEE", "admin_secretariat", r.jour);
    close["purge_at"] = json!(instant(r.jour + Duration::days(60)));
    let mut echue = requete(&i_echue, "TERMINEE", "maintenance", r.jour);
    echue["purge_at"] = json!(instant(r.jour - Duration::days(3)));
    let sans = requete(&i_sans, "TERMINEE", "maintenance", r.jour);
    let statut = requete(&i_statut, "BIZARRE", "maintenance", r.jour);
    let export = r.export(
        json!([nouvelle, en_cours, attente, corbeille, inconnue, close, echue, sans, statut]),
        json!([]),
        json!([]),
        json!(null),
    );

    let rapport = r.reprendre(&export, &PiecesAbsentes).await;

    // Critère go/no-go : 100 % des demandes ouvertes reprises et attribuées à une file.
    assert_eq!(
        rapport.requetes_ouvertes, 6,
        "5 ouvertes + 1 statut inconnu à vérifier comptés : {rapport:#?}"
    );
    // Le statut inconnu est « à vérifier » : il n'est ni repris ni perdu en silence.
    assert_eq!(
        r.decision("request", &i_statut).await,
        Some(("a_verifier".into(), Some("statut_inconnu".into())))
    );
    for id in [&i_nouvelle, &i_cours, &i_attente, &i_corbeille, &i_inconnue] {
        let cible = r
            .cible("request", id)
            .await
            .unwrap_or_else(|| panic!("{id} non reprise"));
        let (statut, _, _, _, _, canal) = r.ligne(cible).await;
        assert_eq!(canal, "reprise");
        assert!(
            ["deposee", "prise", "en_attente_tiers"].contains(&statut.as_str()),
            "{statut}"
        );
        // Chaque demande a une file (R-DM6) et une chaîne de faits intacte.
        assert_eq!(
            r.ent
                .verifier_chaine_demande(r.etab(), cible)
                .await
                .unwrap(),
            None
        );
    }
    // Natures déduites des routes (§29.1) ; statuts ; prise reprise si l'agent est retrouvé.
    let c_nouvelle = r.cible("request", &i_nouvelle).await.unwrap();
    assert_eq!(r.ligne(c_nouvelle).await.2, "maintenance.intervention");
    assert_eq!(r.ligne(c_nouvelle).await.0, "deposee");
    let c_cours = r.cible("request", &i_cours).await.unwrap();
    let (st, _, _, agent, _, _) = r.ligne(c_cours).await;
    assert_eq!(
        (st.as_str(), agent),
        ("prise", r.m.prof.agent),
        "agent retrouvé par l'adresse de sa fiche, sans casse"
    );
    let c_attente = r.cible("request", &i_attente).await.unwrap();
    assert_eq!(r.ligne(c_attente).await.2, "comptabilite.demande_interne");
    assert_eq!(r.ligne(c_attente).await.0, "en_attente_tiers");
    // La corbeille et la route inconnue vont à la file de tri ; l'agent inconnu → remise en file, jamais perdue.
    for id in [&i_corbeille, &i_inconnue] {
        let cible = r.cible("request", id).await.unwrap();
        let (type_,): (String,) = sqlx::query_as(
            "SELECT f.type FROM ent_dem_demande d JOIN ent_dem_file f ON f.etablissement_id = d.etablissement_id AND f.id = d.file_id
              WHERE d.etablissement_id = $1 AND d.id = $2",
        )
        .bind(r.etab())
        .bind(cible)
        .fetch_one(r.ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(type_, "tri");
    }
    let c_inconnue = r.cible("request", &i_inconnue).await.unwrap();
    assert_eq!(
        r.ligne(c_inconnue).await.0,
        "deposee",
        "agent non retrouvé : remise en file"
    );
    // Les closes : archive lecture seule jusqu'à `purge_at`, issue « traitée », pas de prolongation.
    let c_close = r.cible("request", &i_close).await.unwrap();
    let (st, issue, _, _, conservation, _) = r.ligne(c_close).await;
    assert_eq!((st.as_str(), issue.as_deref()), ("close", Some("traitee")));
    assert_eq!(
        conservation,
        Some(r.jour + Duration::days(60)),
        "conservation = jour d'établissement de purge_at"
    );
    assert_eq!(
        r.decision("request", &i_echue).await,
        Some(("ignoree".into(), Some("purge_deja_echue".into())))
    );
    assert_eq!(
        r.decision("request", &i_sans).await,
        Some(("a_verifier".into(), Some("purge_at_absente".into())))
    );
    // Pas d'échéance sur une archive ; des échéances sur une demande ouverte.
    assert_eq!(
        r.nb(
            "SELECT count(*) FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2",
            Some(c_close)
        )
        .await,
        0
    );
    assert!(
        r.nb(
            "SELECT count(*) FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2",
            Some(c_nouvelle)
        )
        .await
            >= 1
    );
    // Valeurs typées quand le champ est déclaré (gravité), le reste en note interne ; commentaires internes par défaut.
    assert_eq!(
        r.nb("SELECT count(*) FROM ent_dem_valeur WHERE etablissement_id = $1 AND demande_id = $2 AND champ_code = 'gravite' AND valeur_code = 'gene'", Some(c_nouvelle)).await,
        1
    );
    assert_eq!(
        r.nb("SELECT count(*) FROM ent_dem_message WHERE etablissement_id = $1 AND demande_id = $2 AND visibilite = 'demandeur'", Some(c_nouvelle)).await,
        1,
        "seul le commentaire que l'export dit visible l'est"
    );
    assert!(r.nb("SELECT count(*) FROM ent_dem_message WHERE etablissement_id = $1 AND demande_id = $2 AND visibilite = 'interne'", Some(c_nouvelle)).await >= 2);
    // Historique : un fait par ligne source.
    assert_eq!(r.nb("SELECT count(*) FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'reprise_historique'", Some(c_nouvelle)).await, 2);
    // Socle 7 absent : la pièce n'est PAS reprise, et le rapport le dit (critère « 0 pièce illisible » non atteint).
    assert_eq!(rapport.pieces_non_reprises, 1);
    assert!(!rapport.go_no_go(), "une pièce non reprise empêche le go");
    // R-DM49 : la reprise n'envoie aucun avis (un seul avis de reprise par personne, le jour d'ouverture).
    assert_eq!(r.nb("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = ANY(SELECT id FROM ent_dem_demande WHERE etablissement_id = $1 AND canal_depot = 'reprise')", None).await, 0);
    assert_eq!(rapport.heures_creees_au_15, 0);
    assert!(!rapport.echantillon.is_empty() && rapport.echantillon.len() <= 20);
    assert!(rapport.duree_ms >= 0);

    // Avec le socle 7 branché (port fictif), les pièces sont reprises : le critère pièces passe.
    let r2 = R::nouveau().await.unwrap();
    let mut une = requete(&code("rq"), "NOUVELLE", "maintenance", r2.jour);
    une["attachments"] = json!([{"id": code("pj"), "file_name": "a.pdf", "s3_key": "k/a.pdf"}]);
    let rapport2 = r2
        .reprendre(
            &r2.export(json!([une]), json!([]), json!([]), json!(null)),
            &Pieces,
        )
        .await;
    assert_eq!(rapport2.pieces_non_reprises, 0);
    assert!(rapport2.go_no_go(), "{rapport2:#?}");
    assert_eq!(
        r2.nb(
            "SELECT count(*) FROM ent_dem_piece WHERE etablissement_id = $1",
            None
        )
        .await,
        1
    );
}

#[tokio::test]
async fn t_reprise_rejouable() {
    let Some(r) = R::nouveau().await else { return };
    let ids: Vec<String> = (0..4).map(|_| code("rq")).collect();
    let mut close = requete(&ids[2], "TERMINEE", "maintenance", r.jour);
    close["purge_at"] = json!(instant(r.jour + Duration::days(30)));
    let export = r.export(
        json!([
            requete(&ids[0], "NOUVELLE", "maintenance", r.jour),
            requete(&ids[1], "EN_ATTENTE", "comptabilite", r.jour),
            close,
            requete(&ids[3], "BIZARRE", "maintenance", r.jour)
        ]),
        json!([]),
        json!([]),
        json!(null),
    );
    let premier = r.reprendre(&export, &PiecesAbsentes).await;
    assert!(!premier.rejoue);
    let avant = r
        .nb(
            "SELECT count(*) FROM ent_dem_demande WHERE etablissement_id = $1",
            None,
        )
        .await;
    let faits_avant = r
        .nb(
            "SELECT count(*) FROM ent_dem_fait WHERE etablissement_id = $1",
            None,
        )
        .await;
    let personnes_avant = r
        .nb(
            "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1",
            None,
        )
        .await;
    let cibles: Vec<Option<Uuid>> = {
        let mut v = Vec::new();
        for i in &ids {
            v.push(r.cible("request", i).await);
        }
        v
    };

    // Deuxième passe sur le MÊME export : même résultat, rien de créé (§32 point 6).
    let second = r.reprendre(&export, &PiecesAbsentes).await;
    assert!(second.rejoue, "l'empreinte de l'export est reconnue");
    assert_eq!(second.empreinte_export, premier.empreinte_export);
    assert_ne!(second.lot_id, premier.lot_id);
    assert_eq!(
        r.nb(
            "SELECT count(*) FROM ent_dem_demande WHERE etablissement_id = $1",
            None
        )
        .await,
        avant
    );
    assert_eq!(
        r.nb(
            "SELECT count(*) FROM ent_dem_fait WHERE etablissement_id = $1",
            None
        )
        .await,
        faits_avant
    );
    assert_eq!(
        r.nb(
            "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1",
            None
        )
        .await,
        personnes_avant
    );
    for (i, c) in ids.iter().zip(&cibles) {
        assert_eq!(
            &r.cible("request", i).await,
            c,
            "même cible à la deuxième passe"
        );
    }
    // Les comptages d'une passe rejouée sont ceux de la première (les « à vérifier » sont retentés, sans création).
    assert_eq!(second.requetes_ouvertes, premier.requetes_ouvertes);
    assert_eq!(
        second.requetes_ouvertes_reprises,
        premier.requetes_ouvertes_reprises
    );
    let (passages,): (i32,) = sqlx::query_as(
        "SELECT passages FROM ent_dem_reprise_correspondance WHERE etablissement_id = $1 AND source = 'request' AND source_id = $2",
    )
    .bind(r.etab())
    .bind(&ids[0])
    .fetch_one(r.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(passages, 2);
    // Le numéro n'est pas consommé par une passe rejouée : la numérotation reste continue.
    let (dernier,): (i32,) = sqlx::query_as("SELECT dernier FROM ent_dem_numerotation WHERE etablissement_id = $1 ORDER BY dernier DESC LIMIT 1")
        .bind(r.etab())
        .fetch_one(r.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(i64::from(dernier), avant);
}

// ───────────────────────── HSE (CDM-50) ─────────────────────────

fn hse(
    id: &str,
    statut: &str,
    jour: NaiveDate,
    heures: Option<f64>,
    decideur: Option<&str>,
) -> Value {
    let mut v = json!({
        "id": id,
        "createdAt": instant(jour - Duration::days(5)),
        "updatedAt": instant(jour - Duration::days(2)),
        "status": statut,
        "createdBy": {"userId": "u_prof", "name": "Professeur fictif", "email": ""},
        "resumeDemande": format!("Atelier {id}"),
        "motif": "Soutien",
        "classe": "4B",
        "details": "Détails fictifs.",
    });
    if let Some(h) = heures {
        v["nombreHeures"] = json!(h);
    }
    if let Some(d) = decideur {
        v["decidedBy"] = json!({"userId": d, "name": "Direction fictive"});
        v["decidedAt"] = json!(instant(jour - Duration::days(3)));
    }
    v
}

#[tokio::test]
async fn t_reprise_hse_cdm50() {
    let Some(r) = R::nouveau().await else { return };
    let (attente, acceptee, refusee, annulee, ancienne, sans_volume) = (
        code("hs"),
        code("hs"),
        code("hs"),
        code("hs"),
        code("hs"),
        code("hs"),
    );
    let mut vieille = hse(&ancienne, "ACCEPTEE", r.jour, Some(2.0), Some("u_dir"));
    vieille["createdAt"] = json!(instant(r.debut_annee - Duration::days(40)));
    let mut a_pdf = hse(&acceptee, "ACCEPTEE", r.jour, Some(6.5), Some("u_dir"));
    a_pdf["acceptancePdfPath"] = json!("demandes-hse/x/acceptation.pdf");
    let export = r.export(
        json!([]),
        json!([
            hse(&attente, "EN_ATTENTE", r.jour, Some(4.0), None),
            a_pdf,
            hse(&refusee, "REFUSEE", r.jour, Some(1.0), Some("u_dir")),
            hse(&annulee, "ANNULEE", r.jour, Some(1.0), None),
            vieille,
            hse(&sans_volume, "EN_ATTENTE", r.jour, None, None)
        ]),
        json!([]),
        json!(null),
    );
    let rapport = r.reprendre(&export, &PiecesAbsentes).await;

    // EN_ATTENTE → en_validation : la décision est à refaire dans ScolIA.
    let c = r
        .cible("demandes_hse", &attente)
        .await
        .expect("HSE en attente reprise");
    assert_eq!(r.ligne(c).await.0, "en_validation");
    let (volume, decision): (i32, Option<String>) = sqlx::query_as(
        "SELECT volume_demande_quarts, decision FROM ent_dem_hse WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(r.etab())
    .bind(c)
    .fetch_one(r.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (volume, decision),
        (16, None),
        "4 h = 16 quarts d'heure, aucune décision reprise"
    );
    // ACCEPTEE → close, issue accordée, volume accordé, « transmise hors ScolIA ».
    let a = r
        .cible("demandes_hse", &acceptee)
        .await
        .expect("HSE acceptée reprise");
    let (st, issue, _, _, conservation, _) = r.ligne(a).await;
    assert_eq!(
        (st.as_str(), issue.as_deref(), conservation),
        ("close", Some("accordee"), None),
        "RH : jamais de purge automatique"
    );
    assert_eq!(
        r.decision("demandes_hse", &acceptee).await,
        Some(("archive".into(), Some("hse_transmise_hors_scolia".into())))
    );
    let (accorde, dec, par): (Option<i32>, Option<String>, Option<Uuid>) = sqlx::query_as(
        "SELECT volume_accorde_quarts, decision, decide_par_personne_id FROM ent_dem_hse WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(r.etab())
    .bind(a)
    .fetch_one(r.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (accorde, dec.as_deref(), par),
        (Some(26), Some("accordee"), Some(r.m.direction.personne))
    );
    assert_eq!(
        r.ent.verifier_chaine_demande(r.etab(), a).await.unwrap(),
        None
    );
    // Refusée, annulée → archive ; année précédente et volume absent → rien de repris, dit pourquoi.
    assert_eq!(
        r.ligne(r.cible("demandes_hse", &refusee).await.unwrap())
            .await
            .1
            .as_deref(),
        Some("refusee")
    );
    assert_eq!(
        r.ligne(r.cible("demandes_hse", &annulee).await.unwrap())
            .await
            .0,
        "annulee"
    );
    assert_eq!(
        r.decision("demandes_hse", &ancienne).await,
        Some(("ignoree".into(), Some("annee_precedente".into())))
    );
    assert_eq!(
        r.decision("demandes_hse", &sans_volume).await,
        Some(("a_verifier".into(), Some("volume_absent".into())))
    );

    // CDM-50 : AUCUNE heure au socle 15, et aucune séance possible sur une HSE reprise comme déjà transmise.
    assert_eq!(rapport.heures_creees_au_15, 0);
    assert_eq!(
        r.nb(
            "SELECT count(*) FROM ent_dem_hse_seance WHERE etablissement_id = $1",
            None
        )
        .await,
        0
    );
    let essai = sqlx::query(
        "INSERT INTO ent_dem_hse_seance (id, etablissement_id, demande_id, jour, duree_quarts, etat)
         VALUES ($1, $2, $3, $4, 4, 'prevue')",
    )
    .bind(Uuid::new_v4())
    .bind(r.etab())
    .bind(a)
    .bind(r.jour + Duration::days(1))
    .execute(r.ent.pool_verif())
    .await;
    assert!(
        essai.is_err(),
        "le déclencheur interdit toute séance sur une HSE transmise hors ScolIA"
    );
    // Et par le service : l'agent ne peut pas déclarer de séance, donc jamais d'heure à constater.
    struct Heures(std::sync::Mutex<u32>);
    impl crate::demandes_hse::PortHeures for Heures {
        fn constater(&self, _: Uuid, _: &crate::demandes_hse::HeureAConstater) -> Option<Uuid> {
            *self.0.lock().unwrap() += 1;
            Some(Uuid::new_v4())
        }
    }
    let heures = Heures(std::sync::Mutex::new(0));
    let sources = SourcesFixes(Uuid::new_v4());
    let refus = r
        .ent
        .declarer_seance_for_actor(
            r.etab(),
            r.m.prof.user,
            "prof",
            &sources,
            &heures,
            a,
            crate::demandes_hse::SeanceSaisie {
                jour: r.jour + Duration::days(1),
                debut: None,
                duree_quarts: 4,
                fait: false,
                lieu: None,
            },
        )
        .await;
    assert!(
        refus.is_err(),
        "aucune séance sur une HSE reprise comme transmise"
    );
    assert_eq!(
        *heures.0.lock().unwrap(),
        0,
        "le port du socle 15 n'est jamais appelé"
    );
    // Le PDF d'acceptation : socle 7 absent → signalé, jamais perdu en silence.
    assert!(rapport.pieces_non_reprises >= 1);
    // Le compteur du lot ne peut pas être modifié (contrainte en base).
    let tricher = sqlx::query(
        "UPDATE ent_dem_reprise_lot SET heures_creees_au_15 = 3 WHERE etablissement_id = $1",
    )
    .bind(r.etab())
    .execute(r.ent.pool_verif())
    .await;
    assert!(tricher.is_err(), "heures_creees_au_15 est contraint à 0");
}

// ───────────────────────── Photocopies ─────────────────────────

fn photocopie(id: &str, statut: &str, type_: Option<&str>, jour: NaiveDate, n: i64) -> Value {
    let mut v = json!({
        "id": id,
        "createdAt": instant(jour - Duration::days(3)),
        "status": statut,
        "createdBy": {"userId": "u_prof", "name": "Professeur fictif", "email": ""},
        "motif": format!("Contrôle {id}"),
        "classesOuMatiere": "4B",
        "nombrePhotocopies": n,
        "documents": [{"key": format!("photocopies-couleur/{id}.pdf"), "fileName": "controle.pdf", "contentType": "application/pdf"}],
    });
    if let Some(t) = type_ {
        v["typeImpression"] = json!(t);
    }
    v
}

#[tokio::test]
async fn t_reprise_photocopies_ouvertes() {
    let Some(r) = R::nouveau().await else { return };
    // La file d'impression est celle d'un site : la demande reprise en hérite.
    let site = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_site (id, etablissement_id, code, libelle) VALUES ($1, $2, $3, $3)",
    )
    .bind(site)
    .bind(r.etab())
    .bind(code("s"))
    .execute(r.ent.pool_verif())
    .await
    .unwrap();
    let (attente, acceptee, prete, refusee, mauvaise) =
        (code("ph"), code("ph"), code("ph"), code("ph"), code("ph"));
    let export = r.export(
        json!([]),
        json!([]),
        json!([
            photocopie(&attente, "EN_ATTENTE", None, r.jour, 120),
            photocopie(&acceptee, "ACCEPTEE", Some("NOIR_BLANC"), r.jour, 60),
            photocopie(&prete, "PRETE", None, r.jour, 10),
            photocopie(&refusee, "REFUSEE", None, r.jour, 10),
            photocopie(&mauvaise, "EN_ATTENTE", None, r.jour, 0)
        ]),
        json!(null),
    );
    // Le site vient de la file de reprographie : on la rattache à notre site (sinon, plusieurs sites → « à vérifier »).
    sqlx::query("UPDATE ent_dem_file SET site_id = $2 WHERE etablissement_id = $1 AND code = $3")
        .bind(r.etab())
        .bind(site)
        .bind(nature("reprographie.tirage").expect("nature").file_defaut)
        .execute(r.ent.pool_verif())
        .await
        .unwrap();
    let rapport = r.reprendre(&export, &Pieces).await;

    // Seules les demandes ouvertes sont reprises (T0 : le reste est perdable, sauf décision de Florian, Q-DM8).
    assert_eq!(
        r.decision("photocopies", &prete).await,
        Some(("ignoree".into(), Some("non_ouverte".into())))
    );
    assert_eq!(
        r.decision("photocopies", &refusee).await,
        Some(("ignoree".into(), Some("non_ouverte".into())))
    );
    assert_eq!(
        r.decision("photocopies", &mauvaise).await,
        Some(("a_verifier".into(), Some("quantite_invalide".into())))
    );
    let a = r
        .cible("photocopies", &attente)
        .await
        .expect("EN_ATTENTE reprise");
    let (st, _, nat, _, _, _) = r.ligne(a).await;
    assert_eq!(
        (st.as_str(), nat.as_str()),
        ("en_validation", "reprographie.tirage")
    );
    let (etat, couleur, copies, validation): (String, bool, i32, bool) = sqlx::query_as(
        "SELECT etat, couleur, copies_total, validation_requise FROM ent_dem_repro WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(r.etab())
    .bind(a)
    .fetch_one(r.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (etat.as_str(), couleur, copies, validation),
        ("a_valider", true, 120, true)
    );
    let b = r
        .cible("photocopies", &acceptee)
        .await
        .expect("ACCEPTEE reprise");
    let (etat_b, couleur_b): (String, bool) = sqlx::query_as(
        "SELECT etat, couleur FROM ent_dem_repro WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(r.etab())
    .bind(b)
    .fetch_one(r.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (etat_b.as_str(), couleur_b),
        ("a_imprimer", false),
        "noir et blanc accepté : directement à imprimer"
    );
    // PDF repris par le socle 7 (port fictif) : un document et une pièce « fichier à imprimer » par demande ouverte.
    assert_eq!(
        r.nb(
            "SELECT count(*) FROM ent_dem_repro_document WHERE etablissement_id = $1",
            None
        )
        .await,
        2
    );
    assert_eq!(rapport.pieces_non_reprises, 0);
}

// ───────────────────────── Organisation (proposition) ─────────────────────────

#[tokio::test]
async fn t_reprise_organisation_en_proposition() {
    let Some(r) = R::nouveau().await else { return };
    let inconnu = format!("{}@exemple.invalid", code("inconnu"));
    let unite = code("unite_");
    let org = json!({"units": [
        {"id": unite, "label": "Maintenance", "managerEmails": [r.email_prof], "memberEmails": [inconnu, r.email_prof.to_uppercase()],
         "taskIds": ["maintenance"], "active": true},
        {"id": code("off_"), "label": "Désactivée", "managerEmails": [], "memberEmails": [], "taskIds": [], "active": false}
    ]});
    let files_avant = r
        .nb(
            "SELECT count(*) FROM ent_dem_file WHERE etablissement_id = $1",
            None,
        )
        .await;
    let regles_avant = r
        .nb(
            "SELECT count(*) FROM ent_dem_file_membre_regle WHERE etablissement_id = $1",
            None,
        )
        .await;
    let rapport = r
        .reprendre(
            &r.export(json!([]), json!([]), json!([]), org),
            &PiecesAbsentes,
        )
        .await;

    // Une proposition : files, natures, désignations datées, adresses non retrouvées.
    let lignes: Vec<(String, Option<Uuid>, bool, Option<NaiveDate>, Option<String>, String)> = sqlx::query_as(
        "SELECT genre, agent_id, responsable, du, repere_masque, statut FROM ent_dem_reprise_proposition
          WHERE etablissement_id = $1 ORDER BY genre",
    )
    .bind(r.etab())
    .fetch_all(r.ent.pool_verif())
    .await
    .unwrap();
    let genres: Vec<&str> = lignes.iter().map(|l| l.0.as_str()).collect();
    assert_eq!(
        genres,
        ["adresse_non_retrouvee", "designation", "file", "nature"],
        "l'unité désactivée n'est pas proposée"
    );
    assert_eq!(rapport.propositions, 4);
    let desig = lignes.iter().find(|l| l.0 == "designation").unwrap();
    assert_eq!(
        desig.1, r.m.prof.agent,
        "responsable retrouvé par adresse (sans casse), une seule désignation"
    );
    assert!(desig.2, "responsable");
    assert_eq!(
        desig.3,
        Some(r.jour),
        "désignation DATÉE du jour d'établissement, à convertir en mission"
    );
    let adr = lignes
        .iter()
        .find(|l| l.0 == "adresse_non_retrouvee")
        .unwrap();
    assert!(
        adr.4.as_deref().is_some_and(|m| m.contains("***@")),
        "adresse masquée : {adr:?}"
    );
    assert!(
        lignes.iter().all(|l| l.5 == "proposee"),
        "rien n'est retenu seul"
    );
    // Jamais appliquée : aucune file, aucune règle de membre créée par la reprise.
    assert_eq!(
        r.nb(
            "SELECT count(*) FROM ent_dem_file WHERE etablissement_id = $1",
            None
        )
        .await,
        files_avant
    );
    assert_eq!(
        r.nb(
            "SELECT count(*) FROM ent_dem_file_membre_regle WHERE etablissement_id = $1",
            None
        )
        .await,
        regles_avant
    );
    // Rejouer ne double pas la proposition.
    let _ = r.reprendre(&r.export(json!([]), json!([]), json!([]), json!({"units": [
        {"id": unite, "label": "Maintenance", "managerEmails": [r.email_prof], "memberEmails": [inconnu], "taskIds": ["maintenance"], "active": true}]})), &PiecesAbsentes).await;
    assert!(
        r.nb(
            "SELECT count(*) FROM ent_dem_reprise_proposition WHERE etablissement_id = $1",
            None
        )
        .await
            <= 4
    );
}

// ───────────────────────── Demandeurs (NJ-84, NJ-88) ─────────────────────────

#[tokio::test]
async fn t_reprise_demandeur_sans_email() {
    let Some(r) = R::nouveau().await else { return };
    // Une adresse qui est EXACTEMENT celle d'une personne de ScolIA : elle ne rattache RIEN (NJ-84, NJ-88).
    let (parent_email,): (Option<String>,) =
        sqlx::query_as("SELECT email FROM ent_personne WHERE etablissement_id = $1 AND id = $2")
            .bind(r.etab())
            .bind(r.m.parent.personne)
            .fetch_one(r.ent.pool_verif())
            .await
            .unwrap();
    let adresse = parent_email.unwrap_or_else(|| format!("{}@exemple.invalid", code("parent")));
    sqlx::query("UPDATE ent_personne SET email = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(r.etab())
        .bind(r.m.parent.personne)
        .bind(&adresse)
        .execute(r.ent.pool_verif())
        .await
        .unwrap();
    let (a, b, c) = (code("rq"), code("rq"), code("rq"));
    let mut externe = requete(&a, "NOUVELLE", "maintenance", r.jour);
    externe["requester_user_id"] = json!(null);
    externe["requester_email"] = json!(adresse);
    let mut externe_bis = requete(&b, "NOUVELLE", "maintenance", r.jour);
    externe_bis["requester_user_id"] = json!(null);
    externe_bis["requester_email"] = json!(adresse.to_uppercase());
    let mut sans_compte = requete(&c, "NOUVELLE", "maintenance", r.jour);
    sans_compte["requester_user_id"] = json!("u_inconnu");
    let rapport = r
        .reprendre(
            &r.export(
                json!([externe, externe_bis, sans_compte]),
                json!([]),
                json!([]),
                json!(null),
            ),
            &PiecesAbsentes,
        )
        .await;
    assert_eq!(
        rapport.requetes_ouvertes_reprises, 3,
        "reprises quand même : jamais perdues"
    );

    let ca = r.cible("request", &a).await.unwrap();
    let (demandeur,): (Option<Uuid>,) = sqlx::query_as(
        "SELECT demandeur_personne_id FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(r.etab())
    .bind(ca)
    .fetch_one(r.ent.pool_verif())
    .await
    .unwrap();
    assert_ne!(
        demandeur,
        Some(r.m.parent.personne),
        "aucun rattachement automatique par e-mail"
    );
    // Aucun avis ni accès pour la personne dont l'adresse « correspond » : elle ne voit pas la demande.
    let vue = r
        .ent
        .mes_demandes_for_actor(r.etab(), r.m.parent.user)
        .await
        .unwrap();
    assert!(vue.iter().all(|d| d.id != ca));
    // L'adresse n'est stockée NULLE PART en clair (ni dans la liste à rapprocher, ni dans les correspondances).
    for table in [
        "ent_dem_reprise_demandeur",
        "ent_dem_reprise_correspondance",
        "ent_dem_reprise_proposition",
    ] {
        let (n,): (i64,) = sqlx::query_as(&format!("SELECT count(*) FROM {table} t WHERE t.etablissement_id = $1 AND t::text ILIKE '%' || $2 || '%'"))
            .bind(r.etab())
            .bind(&adresse)
            .fetch_one(r.ent.pool_verif())
            .await
            .unwrap();
        assert_eq!(n, 0, "{table} ne contient jamais l'adresse en clair");
    }
    // La même adresse (casse différente) ne donne qu'UNE ligne « à rapprocher », adresse masquée ; l'autre vient de « u_inconnu ».
    let liste = r
        .ent
        .demandeurs_a_rapprocher_for_actor(r.etab(), r.admin.user, "admin")
        .await
        .unwrap();
    assert_eq!(liste.len(), 2, "{liste:#?}");
    let ext = liste
        .iter()
        .find(|l| l.source_ref.starts_with("ext:"))
        .expect("externe");
    assert_eq!(ext.nb_demandes, 2);
    assert!(ext
        .repere_masque
        .as_deref()
        .is_some_and(|m| m.contains("***@")));
    assert!(
        r.ent
            .demandeurs_a_rapprocher_for_actor(r.etab(), r.m.prof.user, "prof")
            .await
            .is_err(),
        "le secrétariat seul (droit de reprise)"
    );

    // Le secrétariat rapproche : les demandes sont alors à la personne retrouvée, tracé, chaîne intacte.
    let n = r
        .ent
        .rapprocher_demandeur_for_actor(
            r.etab(),
            r.admin.user,
            "admin",
            &ext.source_ref,
            r.m.parent.personne,
        )
        .await
        .unwrap();
    assert_eq!(n, 2);
    let vue = r
        .ent
        .mes_demandes_for_actor(r.etab(), r.m.parent.user)
        .await
        .unwrap();
    assert!(
        vue.iter().any(|d| d.id == ca),
        "la demande rapprochée est lisible de son demandeur"
    );
    assert_eq!(r.nb("SELECT count(*) FROM ent_dem_fait WHERE etablissement_id = $1 AND type = 'rapprochement'", None).await, 2);
    assert_eq!(
        r.ent.verifier_chaine_demande(r.etab(), ca).await.unwrap(),
        None
    );
    // Un second rapprochement est refusé ; le demandeur d'une demande ordinaire ne change toujours JAMAIS (R-DM4).
    assert!(r
        .ent
        .rapprocher_demandeur_for_actor(
            r.etab(),
            r.admin.user,
            "admin",
            &ext.source_ref,
            r.m.prof.personne
        )
        .await
        .is_err());
    let direct = sqlx::query("UPDATE ent_dem_demande SET demandeur_personne_id = $3, version = version + 1 WHERE etablissement_id = $1 AND id = $2")
        .bind(r.etab())
        .bind(ca)
        .bind(r.m.prof.personne)
        .execute(r.ent.pool_verif())
        .await;
    assert!(
        direct.is_err(),
        "hors du chemin de rapprochement, le demandeur ne change jamais"
    );
}

// ───────────────────────── Guichet en lecture seule (DEM-349) ─────────────────────────

#[tokio::test]
async fn t_guichet_lecture_seule() {
    let Some(r) = R::nouveau().await else { return };
    let (file,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_dem_file WHERE etablissement_id = $1 ORDER BY code LIMIT 1",
    )
    .bind(r.etab())
    .fetch_one(r.ent.pool_verif())
    .await
    .unwrap();
    let sources = SourcesFixes(file);
    let depot = crate::tests_demandes::depot_maintenance();
    // Guichet ouvert : le dépôt passe.
    let etat = r
        .ent
        .deposer_demande_for_actor(r.etab(), r.m.prof.user, "prof", &sources, depot.clone())
        .await
        .expect("dépôt ouvert");
    assert!(!r.ent.etat_guichet(r.etab()).await.unwrap().lecture_seule);

    // Seul le droit de reprise (administrateur) règle le guichet.
    assert!(r
        .ent
        .regler_guichet_for_actor(r.etab(), r.m.prof.user, "prof", true, "bascule")
        .await
        .is_err());
    assert!(r
        .ent
        .regler_guichet_for_actor(r.etab(), r.admin.user, "admin", true, "n_importe_quoi")
        .await
        .is_err());
    let g = r
        .ent
        .regler_guichet_for_actor(r.etab(), r.admin.user, "admin", true, "bascule")
        .await
        .unwrap();
    assert_eq!(
        (g.lecture_seule, g.raison.as_deref()),
        (true, Some("bascule"))
    );
    let avant = r
        .nb(
            "SELECT count(*) FROM ent_dem_demande WHERE etablissement_id = $1",
            None,
        )
        .await;
    // Dépôt et gestes : refusés PROPREMENT (409, phrase claire), rien de créé ni de modifié.
    let refus = r
        .ent
        .deposer_demande_for_actor(r.etab(), r.m.prof.user, "prof", &sources, depot)
        .await
        .unwrap_err();
    assert!(matches!(refus, RefusDemande::Conflit(_)), "{refus:?}");
    assert!(
        refus.message_public().contains("lecture seule")
            && refus.message_public().contains("accueil")
    );
    assert_eq!(
        r.nb(
            "SELECT count(*) FROM ent_dem_demande WHERE etablissement_id = $1",
            None
        )
        .await,
        avant
    );
    let geste = r
        .ent
        .agir_sur_demande_for_actor(
            r.etab(),
            r.m.direction.user,
            "direction",
            &sources,
            etat.id,
            etat.version,
            Geste::Prendre,
        )
        .await
        .unwrap_err();
    assert!(matches!(geste, RefusDemande::Conflit(_)), "{geste:?}");
    // La lecture reste libre.
    assert!(r
        .ent
        .mes_demandes_for_actor(r.etab(), r.m.prof.user)
        .await
        .unwrap()
        .iter()
        .any(|d| d.id == etat.id));
    // La reprise de BASCULE exige le guichet en lecture seule ; la répétition non.
    let vide = r.export(json!([]), json!([]), json!([]), json!(null));
    let bascule = r
        .ent
        .reprendre_for_actor(
            r.etab(),
            r.admin.user,
            "admin",
            ModeReprise::Bascule,
            &vide,
            &PiecesAbsentes,
        )
        .await;
    assert!(
        bascule.is_ok(),
        "guichet en lecture seule : la bascule peut reprendre"
    );
    // Réouverture : tout redevient possible, et chaque changement est journalisé.
    r.ent
        .regler_guichet_for_actor(r.etab(), r.admin.user, "admin", false, "bascule")
        .await
        .unwrap();
    let pris = r
        .ent
        .agir_sur_demande_for_actor(
            r.etab(),
            r.m.direction.user,
            "direction",
            &sources,
            etat.id,
            etat.version,
            Geste::Prendre,
        )
        .await;
    assert!(pris.is_ok(), "guichet rouvert : le geste passe ({pris:?})");
    let refus = r
        .ent
        .reprendre_for_actor(
            r.etab(),
            r.admin.user,
            "admin",
            ModeReprise::Bascule,
            &vide,
            &PiecesAbsentes,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(refus, RefusDemande::Conflit(_)),
        "bascule refusée tant que le guichet est ouvert"
    );
    assert_eq!(r.nb("SELECT count(*) FROM ent_dem_reglage_journal WHERE etablissement_id = $1 AND sujet = 'guichet'", None).await, 2);
}

// ───────────────────────── Droits et export illisible ─────────────────────────

#[tokio::test]
async fn t_reprise_droit_et_export_refuses() {
    let Some(r) = R::nouveau().await else { return };
    let vide = r.export(json!([]), json!([]), json!([]), json!(null));
    // `demandes.reprise` : administrateur seulement (la direction n'a pas ce droit : D-017).
    for (qui, role) in [
        (r.m.direction.user, "direction"),
        (r.m.prof.user, "prof"),
        (r.m.parent.user, "parent"),
    ] {
        let e = r
            .ent
            .reprendre_for_actor(
                r.etab(),
                qui,
                role,
                ModeReprise::Repetition,
                &vide,
                &PiecesAbsentes,
            )
            .await
            .unwrap_err();
        assert_eq!(e, RefusDemande::Interdit, "{role}");
        assert!(
            r.ent
                .rapport_reprise_for_actor(r.etab(), qui, role)
                .await
                .is_err(),
            "{role}"
        );
    }
    // Export illisible, mauvais format ou mauvaise version : refusé avant tout traitement, rien d'écrit.
    for mauvais in [
        "pas du json",
        "{}",
        &json!({"format": "autre", "version": 1}).to_string(),
        &json!({"format": FORMAT_EXPORT, "version": 99}).to_string(),
    ] {
        let e = r
            .ent
            .reprendre_for_actor(
                r.etab(),
                r.admin.user,
                "admin",
                ModeReprise::Repetition,
                mauvais,
                &PiecesAbsentes,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(e, RefusDemande::Invalide(_)),
            "{mauvais:?} : {e:?}"
        );
    }
    assert_eq!(
        r.nb(
            "SELECT count(*) FROM ent_dem_reprise_lot WHERE etablissement_id = $1",
            None
        )
        .await,
        0
    );
    // Rapport du dernier lot, lisible de l'administrateur.
    let _ = r.reprendre(&vide, &PiecesAbsentes).await;
    let rapport = r
        .ent
        .rapport_reprise_for_actor(r.etab(), r.admin.user, "admin")
        .await
        .unwrap();
    assert_eq!(rapport.heures_creees_au_15, 0);
    // Cloisonnement : un autre établissement ne voit rien de ce lot.
    let autre = Uuid::new_v4();
    assert!(r.ent.rapport_du_lot(autre, rapport.lot_id).await.is_err());
}

// ───────────────────────── D-036 ─────────────────────────

#[test]
fn t_tables_n1_5_sans_json_ni_adresse() {
    let dossier = concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations");
    for f in [
        "3048_demandes_n1_5_reprise.sql",
        "3049_demandes_n1_5_bascule.sql",
    ] {
        let sql = std::fs::read_to_string(format!("{dossier}/{f}"))
            .unwrap()
            .to_lowercase();
        let code: String = sql
            .lines()
            .filter(|l| !l.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        for interdit in ["json", "bytea", "[]", "email", "telephone", "adresse_mail"] {
            assert!(
                !code.contains(interdit),
                "{f} contient « {interdit} » (D-036 : pas de JSON, de tableau, d'adresse en clair)"
            );
        }
        assert!(
            code.contains("ent_rls_poser"),
            "{f} : RLS par ent_rls_poser"
        );
    }
}
