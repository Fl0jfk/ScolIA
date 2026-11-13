//! Socle N1 (demandes), lot N1-0 : garde-fous du cœur, contre la base locale de test (feature `labo`) ; sans
//! `ENT_DATABASE_URL`, les tests sont ignorés en local (échec explicite en CI).
//!
//! Chaque défaut du registre `ALERTES_NEXTJS_PROD.md` (§5 ter) fermé par ce lot a un test nommé `t_njNN_…`
//! (règle 14 de `PILOTAGE.md`, D-051) :
//!
//! | Défaut | Test(s) |
//! |---|---|
//! | NJ-72 (aucune limitation de débit) | `t_nj72_debit_depot`, `t_nj72_debit_gestes` |
//! | NJ-80 (tout compte écrit dans toute demande) | `t_nj80_ecrire_sans_droit_refuse` |
//! | NJ-82 (le demandeur lit les notes internes) | `t_nj82_projection_demandeur` |
//! | NJ-84 (identité saisie par le client) | `t_nj84_demandeur_de_session`, `t_nj84_pour_le_compte_trace` |
//! | NJ-86 (contenu intégral hors de l'ENT) | `t_nj86_enveloppe_neutre` |
//! | NJ-88 (dépôt public, rattachement par e-mail) | `t_nj88_aucun_depot_public` (+ `ent-api` : aucune route publique) |
//! | NJ-89 (preuve détruite, purge par lecture) | `t_nj89_faits_ajout_seul`, `t_nj89_purge_par_ordonnanceur` |
//! | NJ-90 (écritures perdues, attributs texte) | `t_nj90_version_concurrente`, `t_nj90_valeurs_typees` |
//! | NJ-93 (messages d'exception au client) | `t_nj93_erreurs_neutres` |
//!
//! Chaque test crée son propre établissement (`seed_labo_cast`) : il ne vérifie que ses propres lignes.
//! Matrice de séparation §24.3 : pièges 1 (`t_nj80…`), 3 (`t_nj82…`), 5 (`t_nj80…`), 6 (`t_nj84…`),
//! 8 (`separation_entre_etablissements`), 15 (`t_nj90_version_concurrente`), 16 (`t_nj89_purge…`).

use super::*;
use crate::demandes::*;
use crate::rate_limit::Quota;
use chrono::NaiveDate;
use ent_auth::Role;
use serde_json::json;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |_| {}).await;
    Some((ent, cast))
}

/// Un acteur de test : compte, fiche de personne et, pour un traitant, fiche d'agent.
#[derive(Clone, Copy)]
struct Acteur {
    user: Uuid,
    personne: Uuid,
    agent: Option<Uuid>,
    role: &'static str,
}

/// Sources de test : une file, des membres désignés, un droit de famille réglable.
#[derive(Clone)]
struct Sources {
    file: Uuid,
    membres: Vec<Uuid>,
    famille: bool,
}

impl SourcesDemandes for Sources {
    fn file_de_depot(&self, _: Uuid, _: &str) -> Option<Uuid> {
        Some(self.file)
    }
    fn files_de(&self, _: Uuid, personne: Uuid) -> Vec<Uuid> {
        if self.membres.contains(&personne) {
            vec![self.file]
        } else {
            Vec::new()
        }
    }
    fn peut_deposer_famille(&self, _: Uuid, _: Uuid, _: Role, _: Option<Uuid>) -> bool {
        self.famille
    }
}

struct Monde {
    etab: Uuid,
    prof: Acteur,
    cpe: Acteur,
    direction: Acteur,
    compta: Acteur,
    parent: Acteur,
    eleve: Uuid,
    src: Sources,
}

async fn fiche(ent: &PgEnt, etab: Uuid, user: Uuid, role: &'static str, agent: bool) -> Acteur {
    let pool = ent.pool_verif();
    let personne = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, user_id, source) VALUES ($1, $2, $3, $4, 'compte')")
        .bind(personne)
        .bind(etab)
        .bind(format!("Test{}", &Uuid::new_v4().simple().to_string()[..8]))
        .bind(user)
        .execute(pool)
        .await
        .unwrap();
    let agent = if agent {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_agent (id, etablissement_id, personne_id, source) VALUES ($1, $2, $3, 'saisie')")
            .bind(id)
            .bind(etab)
            .bind(personne)
            .execute(pool)
            .await
            .unwrap();
        Some(id)
    } else {
        None
    };
    Acteur { user, personne, agent, role }
}

/// Établissement de test prêt : année courante, fiches, une file dont le prof et le CPE sont membres.
/// Le prof est membre de la file ; **le CPE ne l'est pas** par défaut (hors file).
async fn monde(ent: &PgEnt, cast: &LaboCast) -> Monde {
    let etab = cast.etablissement_id;
    let pool = ent.pool_verif();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante")
        .bind(etab)
        .fetch_one(pool)
        .await
        .unwrap();
    if n == 0 {
        let jour = domain_eleve::horloge::jour_paris();
        sqlx::query("INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante) VALUES ($1, $2, $3, $4, $5, true)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(format!("T{}", &Uuid::new_v4().simple().to_string()[..12]))
            .bind(jour - chrono::Duration::days(100))
            .bind(jour + chrono::Duration::days(200))
            .execute(pool)
            .await
            .unwrap();
    }
    let prof = fiche(ent, etab, cast.prof.user_id, "prof", true).await;
    let cpe = fiche(ent, etab, cast.cpe.user_id, "cpe", true).await;
    let direction = fiche(ent, etab, cast.direction.user_id, "direction", true).await;
    let compta = fiche(ent, etab, cast.compta.user_id, "compta", true).await;
    let parent = fiche(ent, etab, cast.parent_lea.user_id, "parent", false).await;
    let src = Sources { file: Uuid::new_v4(), membres: vec![prof.personne], famille: false };
    Monde { etab, prof, cpe, direction, compta, parent, eleve: cast.eleve_lea_id, src }
}

fn depot(nature: &str) -> DepotDemande {
    DepotDemande {
        nature_code: nature.into(),
        objet: "Une ampoule grillée dans le couloir".into(),
        texte: Some("Texte libre de la demande.".into()),
        valeurs: vec![],
        urgence: None,
        eleve_id: None,
        pour_le_compte_de: None,
        canal: None,
    }
}

fn depot_maintenance() -> DepotDemande {
    let mut d = depot("maintenance.intervention");
    d.valeurs = vec![ValeurSaisie { champ: "gravite".into(), rang: 0, valeur: Valeur::Code("gene".into()) }];
    d
}

async fn deposer(ent: &PgEnt, m: &Monde, qui: &Acteur, d: DepotDemande) -> Result<EtatDemande, RefusDemande> {
    ent.deposer_demande_for_actor(m.etab, qui.user, qui.role, &m.src, d).await
}

/// Dépose une demande de maintenance par `qui` et la renvoie.
async fn une_demande(ent: &PgEnt, m: &Monde, qui: &Acteur) -> EtatDemande {
    deposer(ent, m, qui, depot_maintenance()).await.expect("dépôt")
}

async fn nb_faits(ent: &PgEnt, etab: Uuid, id: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(etab)
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    n
}

async fn nb_evenements(ent: &PgEnt, etab: Uuid, id: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2")
        .bind(etab)
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    n
}

// ───────────────────────── Kit ─────────────────────────

#[test]
fn t_kit_nature_complete() {
    // Toutes les natures intégrées respectent les règles de l'assemblage (§8.5).
    for n in NATURES {
        verifier_nature(n).unwrap_or_else(|e| panic!("{} : {e}", n.code));
    }
    let codes: std::collections::BTreeSet<_> = NATURES.iter().map(|n| n.code).collect();
    assert_eq!(codes.len(), NATURES.len(), "codes de nature uniques");
    // Une nature C2+ ne s'ouvre jamais aux familles ni aux élèves.
    let mauvais = NatureDecl { confidentialite: Confidentialite::C2, publics: &[PublicNature::Responsable], ..NATURES[3] };
    assert!(verifier_nature(&mauvais).unwrap_err().contains("C2+"));
    // Code sans préfixe de domaine, délais absents, champ en double, liste vide : refusés.
    assert!(verifier_nature(&NatureDecl { code: "sansdomaine", ..NATURES[0] }).is_err());
    assert!(verifier_nature(&NatureDecl { delai_prise_jours: 0, ..NATURES[0] }).is_err());
    assert!(verifier_nature(&NatureDecl { mots_cles: &[], ..NATURES[0] }).is_err());
    const DOUBLE: &[ChampDecl] = &[
        ChampDecl { code: "a", type_: ChampType::Bool, obligatoire: false, rangs_max: 1 },
        ChampDecl { code: "a", type_: ChampType::Date, obligatoire: false, rangs_max: 1 },
    ];
    assert!(verifier_nature(&NatureDecl { champs: DOUBLE, ..NATURES[0] }).is_err());
    const LISTE_VIDE: &[ChampDecl] = &[ChampDecl { code: "l", type_: ChampType::Liste(&[]), obligatoire: false, rangs_max: 1 }];
    assert!(verifier_nature(&NatureDecl { champs: LISTE_VIDE, ..NATURES[0] }).is_err());
    // Une nature qui demande un élève mais n'a que le public « personnel » est incohérente.
    assert!(verifier_nature(&NatureDecl { eleve_requis: true, ..NATURES[0] }).is_err());
}

#[test]
fn conservation_par_categorie() {
    assert_eq!(Conservation::TechniqueCourte.duree_mois(), Some(12));
    assert_eq!(Conservation::InterneCourante.duree_mois(), Some(24));
    assert_eq!(Conservation::Comptable.duree_mois(), Some(120));
    // RH : jamais de purge automatique (archive, durée de la relation de travail + 5 ans).
    assert_eq!(Conservation::Rh.duree_mois(), None);
}

#[test]
fn enveloppe_neutre_refuse_tout_contenu() {
    let id = Uuid::new_v4();
    let ok = EnveloppeNeutre::nouvelle(id, "D2026-00001", "maintenance.intervention", "deposee").vers_valeur();
    assert!(controler_enveloppe(&ok).is_ok());
    // Clé en trop (objet, texte, nom, e-mail, pièce) : refus.
    for cle in ["objet", "texte", "nom", "email", "piece", "demandeur"] {
        let mut v = ok.clone();
        v[cle] = json!("x");
        assert!(controler_enveloppe(&v).is_err(), "clé « {cle} » acceptée");
    }
    // Valeur qui ressemble à un texte libre ou à une adresse : refus.
    let mut v = ok.clone();
    v["numero"] = json!("Mme Dupont a besoin d'un justificatif");
    assert!(controler_enveloppe(&v).is_err());
    let mut v = ok.clone();
    v["numero"] = json!("a@b.fr");
    assert!(controler_enveloppe(&v).is_err());
    let mut v = ok.clone();
    v["lien"] = json!("https://exemple.test/x");
    assert!(controler_enveloppe(&v).is_err());
    assert!(controler_enveloppe(&json!({"demande_id": id})).is_err());
    assert!(controler_enveloppe(&json!("texte")).is_err());
}

// ───────────────────────── NJ-84 : le demandeur est la session ─────────────────────────

#[tokio::test]
async fn t_nj84_demandeur_de_session() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    // Un client malveillant envoie une identité (piège 6 de la matrice) : ces champs n'existent pas dans le
    // corps accepté, ils sont ignorés à la désérialisation et n'ont aucun chemin vers la base.
    let corps: DepotDemande = serde_json::from_value(json!({
        "nature_code": "secretariat.demande_interne",
        "objet": "Demande ordinaire",
        "email": "victime@exemple.test",
        "nom": "Dupont", "prenom": "Jeanne", "telephone": "0600000000",
        "demandeur_personne_id": m.direction.personne,
        "deposant_personne_id": m.direction.personne,
        "demandeur": { "email": "victime@exemple.test" }
    }))
    .unwrap();
    let etat = deposer(&ent, &m, &m.prof, corps).await.unwrap();
    let (demandeur, deposant): (Option<Uuid>, Option<Uuid>) = sqlx::query_as(
        "SELECT demandeur_personne_id, deposant_personne_id FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(m.etab)
    .bind(etat.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(demandeur, Some(m.prof.personne), "le demandeur est la personne de la session");
    assert_eq!(deposant, None, "pas de déposant distinct");
    // Aucune adresse ni téléphone n'est stocké dans N1 (§24.4) : la colonne n'existe pas.
    let (colonnes,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM information_schema.columns WHERE table_name LIKE 'ent_dem_%'
            AND (column_name ~ '(mail|courriel|telephone|adresse)')",
    )
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(colonnes, 0, "aucune colonne d'adresse ou de téléphone dans les tables de demandes");
    // Un compte sans fiche de personne dans l'établissement ne dépose rien.
    let sans_fiche = deposer(&ent, &m, &Acteur { user: cast.administratif.user_id, personne: Uuid::nil(), agent: None, role: "administratif" }, depot("secretariat.demande_interne")).await;
    assert_eq!(sans_fiche.unwrap_err(), RefusDemande::Interdit);
}

#[tokio::test]
async fn t_nj84_pour_le_compte_trace() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let mut d = depot("secretariat.demande_interne");
    d.pour_le_compte_de = Some(m.prof.personne);
    // Un professeur ne dépose pas pour le compte d'un collègue : il n'a pas l'action.
    assert_eq!(deposer(&ent, &m, &m.cpe, d.clone()).await.unwrap_err(), RefusDemande::Interdit);
    // La direction le peut : le demandeur est le collègue, le déposant est tracé (R-DM5).
    let etat = deposer(&ent, &m, &m.direction, d.clone()).await.unwrap();
    let (demandeur, deposant): (Option<Uuid>, Option<Uuid>) = sqlx::query_as(
        "SELECT demandeur_personne_id, deposant_personne_id FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(m.etab)
    .bind(etat.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((demandeur, deposant), (Some(m.prof.personne), Some(m.direction.personne)));
    let (auteur, motif): (Option<Uuid>, Option<String>) = sqlx::query_as(
        "SELECT auteur_personne_id, motif_code FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'depose'",
    )
    .bind(m.etab)
    .bind(etat.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((auteur, motif.as_deref()), (Some(m.direction.personne), Some("pour_le_compte")));
    // Le collègue voit la demande ; le déposant la voit aussi (jusqu'à 30 jours après clôture).
    assert_eq!(ent.demande_demandeur_for_actor(m.etab, m.prof.user, etat.id).await.unwrap().numero, etat.numero);
    assert_eq!(ent.demande_demandeur_for_actor(m.etab, m.direction.user, etat.id).await.unwrap().numero, etat.numero);
    // Une personne d'un autre établissement ne peut pas être désignée : réponse neutre.
    let mut d2 = depot("secretariat.demande_interne");
    d2.pour_le_compte_de = Some(Uuid::new_v4());
    assert!(matches!(deposer(&ent, &m, &m.direction, d2).await, Err(RefusDemande::Invalide(_))));
}

// ───────────────────────── NJ-88 : aucun dépôt public ─────────────────────────

#[tokio::test]
async fn t_nj88_aucun_depot_public() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    // Un rôle inconnu (aucune session reconnue) ne dépose pas.
    let r = ent.deposer_demande_for_actor(m.etab, Uuid::new_v4(), "anonyme", &m.src, depot("secretariat.demande_interne")).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
    // Un compte inconnu de l'établissement, même avec un rôle valide, n'a pas de fiche : refusé.
    let r = ent.deposer_demande_for_actor(m.etab, Uuid::new_v4(), "prof", &m.src, depot("secretariat.demande_interne")).await;
    assert!(matches!(r, Err(RefusDemande::Interdit | RefusDemande::Introuvable)), "{r:?}");
    // Les canaux « courriel reçu », « courrier », « téléphone », « comptoir » ne sont pas des dépôts par session.
    for canal in [Canal::CourrielRecu, Canal::Courrier, Canal::Telephone, Canal::Comptoir, Canal::Reprise] {
        let mut d = depot("secretariat.demande_interne");
        d.canal = Some(canal);
        assert!(matches!(deposer(&ent, &m, &m.prof, d).await, Err(RefusDemande::Invalide(_))), "{canal:?}");
    }
    // Un parent sans droit de lien (source absente) ne dépose pas, même une nature de famille.
    let mut d = depot("scolarite.attestation");
    d.eleve_id = Some(m.eleve);
    d.valeurs = vec![ValeurSaisie { champ: "type_attestation".into(), rang: 0, valeur: Valeur::Code("assiduite".into()) }];
    let r = ent.deposer_demande_for_actor(m.etab, m.parent.user, "parent", &SourcesAbsentes, d.clone()).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
    // Source de lien présente : le parent dépose pour son enfant ; jamais « pour le compte » d'autrui.
    let src = Sources { famille: true, ..m.src.clone() };
    let ok = ent.deposer_demande_for_actor(m.etab, m.parent.user, "parent", &src, d.clone()).await.unwrap();
    assert_eq!(ok.statut, "deposee");
    let mut pc = d.clone();
    pc.pour_le_compte_de = Some(m.prof.personne);
    assert_eq!(ent.deposer_demande_for_actor(m.etab, m.parent.user, "parent", &src, pc).await.unwrap_err(), RefusDemande::Interdit);
    // Un parent ne dépose pas une nature réservée au personnel.
    let r = ent.deposer_demande_for_actor(m.etab, m.parent.user, "parent", &src, depot("secretariat.demande_interne")).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
    // Sans file (source absente), aucun dépôt n'aboutit : jamais « tout le personnel » (R-DM6, R-DM7).
    let r = ent.deposer_demande_for_actor(m.etab, m.prof.user, "prof", &SourcesAbsentes, depot("secretariat.demande_interne")).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Indisponible);
}

// ───────────────────────── NJ-82 : projections ─────────────────────────

#[tokio::test]
async fn t_nj82_projection_demandeur() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let etat = une_demande(&ent, &m, &m.cpe).await;
    // Le traitant prend, puis un fait interne (note) est écrit : le demandeur ne verra ni l'un ni l'autre en détail.
    let pris = ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, etat.id, 1, Geste::Prendre).await.unwrap();
    assert_eq!(pris.statut, "prise");
    sqlx::query("INSERT INTO ent_dem_fait (id, etablissement_id, demande_id, type, auteur_personne_id, motif_code) VALUES ($1, $2, $3, 'note_interne', $4, 'hypothese_ia')")
        .bind(Uuid::new_v4())
        .bind(m.etab)
        .bind(etat.id)
        .bind(m.prof.personne)
        .execute(ent.pool_verif())
        .await
        .unwrap();

    let vue = ent.demande_demandeur_for_actor(m.etab, m.cpe.user, etat.id).await.unwrap();
    let json = serde_json::to_value(&vue).unwrap();
    let mut cles: Vec<&str> = json.as_object().unwrap().keys().map(String::as_str).collect();
    cles.sort_unstable();
    assert_eq!(
        cles,
        ["clos_le", "decision", "depose_le", "id", "issue", "nature_code", "numero", "objet", "statut", "texte", "urgence", "version"],
        "la projection du demandeur est une liste fermée de champs"
    );
    assert_eq!(vue.statut, "prise");
    let serialise = json.to_string();
    for interdit in [m.src.file.to_string(), m.prof.agent.unwrap().to_string(), m.prof.personne.to_string(), "note_interne".into(), "hypothese".into()] {
        assert!(!serialise.contains(&interdit), "fuite dans la projection du demandeur : {interdit}");
    }
    // Garantie dans la base : la vue du demandeur n'a physiquement aucune colonne d'exploitation.
    let colonnes: Vec<(String,)> = sqlx::query_as("SELECT column_name FROM information_schema.columns WHERE table_name = 'ent_dem_vue_demandeur'")
        .fetch_all(ent.pool_verif())
        .await
        .unwrap();
    let colonnes: Vec<String> = colonnes.into_iter().map(|(c,)| c).collect();
    for interdit in ["file_id", "attribuee_a_agent_id", "attribuee_le", "gel", "conservation_jusqu_au", "beneficiaire_agent_id", "canal_depot", "confidentialite", "purgee_le"] {
        assert!(!colonnes.iter().any(|c| c == interdit), "la vue du demandeur expose « {interdit} »");
    }
    // La liste « mes demandes » : mêmes champs.
    let liste = ent.mes_demandes_for_actor(m.etab, m.cpe.user).await.unwrap();
    assert_eq!(liste.len(), 1);
    assert_eq!(serde_json::to_value(&liste[0]).unwrap(), json);
    // L'historique visible du demandeur : dépôt et prise, sans note interne ni agent.
    let h = ent.historique_demandeur_for_actor(m.etab, m.cpe.user, etat.id).await.unwrap();
    let etapes: Vec<&str> = h.iter().map(|e| e.etape.as_str()).collect();
    assert_eq!(etapes, ["depose", "prise"]);
    let h_json = serde_json::to_string(&h).unwrap();
    assert!(!h_json.contains("note_interne") && !h_json.contains(&m.prof.agent.unwrap().to_string()));
    // Le traitant, lui, a la vue d'exploitation (file, prise).
    let t = ent.demande_traitant_for_actor(m.etab, m.prof.user, "prof", &m.src, RefDemande::Id(etat.id)).await.unwrap();
    assert_eq!((t.file_id, t.attribuee_a_agent_id), (m.src.file, m.prof.agent));
}

// ───────────────────────── NJ-80 : écrire sans droit ─────────────────────────

#[tokio::test]
async fn t_nj80_ecrire_sans_droit_refuse() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let etat = une_demande(&ent, &m, &m.compta).await;
    let faits_avant = nb_faits(&ent, m.etab, etat.id).await;
    let events_avant = nb_evenements(&ent, m.etab, etat.id).await;
    // Le CPE (hors file), la direction (hors file), le parent : ni demandeur, ni déposant, ni membre de la file.
    for (qui, role) in [(&m.cpe, "cpe"), (&m.direction, "direction"), (&m.parent, "parent")] {
        for geste in [Geste::Prendre, Geste::Relacher, Geste::Annuler, Geste::Cloturer { issue: Issue::Traitee }] {
            let r = ent.agir_sur_demande_for_actor(m.etab, qui.user, role, &m.src, etat.id, 1, geste).await;
            // 404 et non 403 : on ne révèle pas que la demande existe (R-DM10).
            assert_eq!(r, Err(RefusDemande::Introuvable), "{role} {geste:?}");
        }
        // Aucune lecture non plus, par identifiant comme par numéro (piège 1 de la matrice).
        assert_eq!(
            ent.demande_traitant_for_actor(m.etab, qui.user, role, &m.src, RefDemande::Id(etat.id)).await.unwrap_err(),
            RefusDemande::Introuvable
        );
        assert_eq!(
            ent.demande_traitant_for_actor(m.etab, qui.user, role, &m.src, RefDemande::Numero(etat.numero.clone())).await.unwrap_err(),
            RefusDemande::Introuvable
        );
        assert_eq!(ent.demande_demandeur_for_actor(m.etab, qui.user, etat.id).await.unwrap_err(), RefusDemande::Introuvable);
        assert!(ent.ma_file_for_actor(m.etab, qui.user, role, &m.src).await.map(|l| l.is_empty()).unwrap_or(true));
    }
    // Rien n'a bougé : ni fait, ni événement, ni changement d'état ; le message d'erreur ne dit rien de la demande.
    assert_eq!(nb_faits(&ent, m.etab, etat.id).await, faits_avant);
    assert_eq!(nb_evenements(&ent, m.etab, etat.id).await, events_avant);
    let (statut, version): (String, i32) = sqlx::query_as("SELECT statut, version FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
        .bind(m.etab)
        .bind(etat.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!((statut.as_str(), version), ("deposee", 1));
    let refus = RefusDemande::Introuvable;
    assert!(!refus.message_public().contains(&etat.numero));
    // Une demande qui n'existe pas donne EXACTEMENT la même réponse qu'une demande interdite.
    let inexistante = ent.agir_sur_demande_for_actor(m.etab, m.cpe.user, "cpe", &m.src, Uuid::new_v4(), 1, Geste::Prendre).await;
    assert_eq!(inexistante, Err(RefusDemande::Introuvable));
    // Le demandeur lui-même peut agir sur la sienne (annuler), mais ne prend pas en charge (droit du geste : 403, il la connaît).
    assert_eq!(
        ent.agir_sur_demande_for_actor(m.etab, m.compta.user, "compta", &m.src, etat.id, 1, Geste::Prendre).await,
        Err(RefusDemande::Interdit)
    );
    // Un membre de la file prend (piège 5 inverse : le droit passe par la file).
    assert!(ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, etat.id, 1, Geste::Prendre).await.is_ok());
    // Une demande déjà prise ne se reprend pas (NJ-81, garde de base).
    let membre2 = Sources { membres: vec![m.prof.personne, m.cpe.personne], ..m.src.clone() };
    assert!(matches!(
        ent.agir_sur_demande_for_actor(m.etab, m.cpe.user, "cpe", &membre2, etat.id, 2, Geste::Prendre).await,
        Err(RefusDemande::Conflit(_))
    ));
    // Un membre qui n'est pas le porteur ne relâche ni ne clôt la prise d'un autre.
    assert_eq!(
        ent.agir_sur_demande_for_actor(m.etab, m.cpe.user, "cpe", &membre2, etat.id, 2, Geste::Relacher).await,
        Err(RefusDemande::Interdit)
    );
    assert_eq!(
        ent.agir_sur_demande_for_actor(m.etab, m.cpe.user, "cpe", &membre2, etat.id, 2, Geste::Cloturer { issue: Issue::Traitee }).await,
        Err(RefusDemande::Interdit)
    );
}

// ───────────────────────── NJ-90 : version et valeurs ─────────────────────────

#[tokio::test]
async fn t_nj90_version_concurrente() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let etat = une_demande(&ent, &m, &m.compta).await;
    // Deux membres de la file prennent EN MÊME TEMPS la même version : une seule prise réussit (piège 15).
    let src = Sources { membres: vec![m.prof.personne, m.cpe.personne], ..m.src.clone() };
    let ent = std::sync::Arc::new(ent);
    let mut taches = Vec::new();
    for qui in [m.prof, m.cpe] {
        let (ent, src) = (ent.clone(), src.clone());
        let (etab, id) = (m.etab, etat.id);
        taches.push(tokio::spawn(async move {
            ent.agir_sur_demande_for_actor(etab, qui.user, qui.role, &src, id, 1, Geste::Prendre).await
        }));
    }
    let mut resultats = Vec::new();
    for t in taches {
        resultats.push(t.await.unwrap());
    }
    let gagnants = resultats.iter().filter(|r| r.is_ok()).count();
    let conflits = resultats.iter().filter(|r| matches!(r, Err(RefusDemande::Conflit(_)))).count();
    assert_eq!((gagnants, conflits), (1, 1), "{resultats:?}");
    assert_eq!(RefusDemande::Conflit("x").statut_http(), 409);
    let (version, porteur): (i32, Option<Uuid>) = sqlx::query_as("SELECT version, attribuee_a_agent_id FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
        .bind(m.etab)
        .bind(etat.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(version, 2);
    assert!(porteur == m.prof.agent || porteur == m.cpe.agent);
    assert_eq!(nb_faits(&ent, m.etab, etat.id).await, 2, "dépôt + une seule prise");
    // Une version périmée est refusée (409) même sans concurrence.
    let r = ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &src, etat.id, 1, Geste::Cloturer { issue: Issue::Traitee }).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))), "{r:?}");
    // La base garde aussi la version : un UPDATE direct qui n'incrémente pas est refusé.
    let r = sqlx::query("UPDATE ent_dem_demande SET urgence = 'haute' WHERE etablissement_id = $1 AND id = $2")
        .bind(m.etab)
        .bind(etat.id)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "écriture sans nouvelle version acceptée par la base");
    let r = sqlx::query("UPDATE ent_dem_demande SET urgence = 'haute', version = version + 1 WHERE etablissement_id = $1 AND id = $2")
        .bind(m.etab)
        .bind(etat.id)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_ok());
}

#[tokio::test]
async fn t_nj90_valeurs_typees() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let lieu = Uuid::new_v4();
    let mut d = depot_maintenance();
    d.valeurs = vec![
        ValeurSaisie { champ: "gravite".into(), rang: 0, valeur: Valeur::Code("bloquante".into()) },
        ValeurSaisie { champ: "nb_personnes".into(), rang: 0, valeur: Valeur::Entier(30) },
        ValeurSaisie { champ: "constate_le".into(), rang: 0, valeur: Valeur::Date(NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()) },
        ValeurSaisie { champ: "heure_constat".into(), rang: 0, valeur: Valeur::Heure(chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap()) },
        ValeurSaisie { champ: "appareil_arrete".into(), rang: 0, valeur: Valeur::Bool(true) },
        ValeurSaisie { champ: "lieu_id".into(), rang: 0, valeur: Valeur::Ref(lieu) },
        ValeurSaisie { champ: "cout_estime".into(), rang: 0, valeur: Valeur::Decimal("125.50".into()) },
        ValeurSaisie { champ: "precision_lieu".into(), rang: 0, valeur: Valeur::Texte("Salle 12, près de la fenêtre".into()) },
    ];
    let etat = deposer(&ent, &m, &m.cpe, d.clone()).await.unwrap();
    // Relecture : chaque valeur retrouve son type, sans conversion de texte.
    let lues = ent.valeurs_demande_for_actor(m.etab, m.cpe.user, "cpe", &m.src, etat.id).await.unwrap();
    let cherche = |c: &str| lues.iter().find(|v| v.champ == c).map(|v| v.valeur.clone()).unwrap();
    assert_eq!(lues.len(), 8);
    assert_eq!(cherche("nb_personnes"), Valeur::Entier(30));
    assert_eq!(cherche("cout_estime"), Valeur::Decimal("125.50".into()));
    assert_eq!(cherche("lieu_id"), Valeur::Ref(lieu));
    assert_eq!(cherche("appareil_arrete"), Valeur::Bool(true));
    assert_eq!(cherche("gravite"), Valeur::Code("bloquante".into()));
    // Une personne extérieure n'y accède pas (404 neutre).
    assert_eq!(ent.valeurs_demande_for_actor(m.etab, m.direction.user, "direction", &m.src, etat.id).await.unwrap_err(), RefusDemande::Introuvable);
    // Refus : champ non déclaré, mauvais type, hors liste, hors bornes, manquant, doublon, trop de rangs, décimal mal formé.
    let mauvais = |champ: &str, rang: u16, v: Valeur| {
        let mut x = depot_maintenance();
        x.valeurs.push(ValeurSaisie { champ: champ.into(), rang, valeur: v });
        x
    };
    for x in [
        mauvais("champ_inconnu", 0, Valeur::Texte("x".into())),
        mauvais("nb_personnes", 0, Valeur::Texte("30".into())),
        mauvais("nb_personnes", 0, Valeur::Entier(0)),
        mauvais("nb_personnes", 1, Valeur::Entier(3)),
        mauvais("cout_estime", 0, Valeur::Decimal("12,5".into())),
        mauvais("cout_estime", 0, Valeur::Decimal("1.234".into())),
        mauvais("gravite", 0, Valeur::Code("catastrophique".into())),
        mauvais("precision_lieu", 0, Valeur::Texte("a".repeat(121))),
        mauvais("lieu_id", 0, Valeur::Ref(Uuid::nil())),
    ] {
        assert!(matches!(deposer(&ent, &m, &m.cpe, x).await, Err(RefusDemande::Invalide(_))));
    }
    let mut sans_obligatoire = depot_maintenance();
    sans_obligatoire.valeurs.clear();
    assert!(matches!(deposer(&ent, &m, &m.cpe, sans_obligatoire).await, Err(RefusDemande::Invalide(_))));
    // Objet vide ou trop long, texte trop long : refus.
    for obj in [String::new(), "   ".into(), "x".repeat(141)] {
        let mut x = depot_maintenance();
        x.objet = obj;
        assert!(matches!(deposer(&ent, &m, &m.cpe, x).await, Err(RefusDemande::Invalide(_))));
    }
    let mut long = depot_maintenance();
    long.texte = Some("x".repeat(2001));
    assert!(matches!(deposer(&ent, &m, &m.cpe, long).await, Err(RefusDemande::Invalide(_))));
    // La base refuse une ligne qui porte deux valeurs, ou aucune (une colonne par type, exactement une).
    let deux = sqlx::query("INSERT INTO ent_dem_valeur (id, etablissement_id, demande_id, champ_code, rang, valeur_texte, valeur_entier) VALUES ($1, $2, $3, 'zz', 9, 'a', 1)")
        .bind(Uuid::new_v4()).bind(m.etab).bind(etat.id).execute(ent.pool_verif()).await;
    assert!(deux.is_err(), "deux valeurs dans une ligne");
    let aucune = sqlx::query("INSERT INTO ent_dem_valeur (id, etablissement_id, demande_id, champ_code, rang) VALUES ($1, $2, $3, 'zz', 9)")
        .bind(Uuid::new_v4()).bind(m.etab).bind(etat.id).execute(ent.pool_verif()).await;
    assert!(aucune.is_err(), "aucune valeur dans une ligne");
    // Valeurs figées : ni réécriture ni suppression hors purge.
    let maj = sqlx::query("UPDATE ent_dem_valeur SET valeur_entier = 31 WHERE etablissement_id = $1 AND demande_id = $2 AND champ_code = 'nb_personnes'")
        .bind(m.etab).bind(etat.id).execute(ent.pool_verif()).await;
    assert!(maj.is_err());
    let sup = sqlx::query("DELETE FROM ent_dem_valeur WHERE etablissement_id = $1 AND demande_id = $2").bind(m.etab).bind(etat.id).execute(ent.pool_verif()).await;
    assert!(sup.is_err());
    assert_eq!(ent.valeurs_demande_for_actor(m.etab, m.cpe.user, "cpe", &m.src, etat.id).await.unwrap().len(), 8);
}

// ───────────────────────── NJ-89 : faits et purge ─────────────────────────

#[tokio::test]
async fn t_nj89_faits_ajout_seul() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let etat = une_demande(&ent, &m, &m.cpe).await;
    ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, etat.id, 1, Geste::Prendre).await.unwrap();
    ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, etat.id, 2, Geste::Relacher).await.unwrap();
    ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, etat.id, 3, Geste::Cloturer { issue: Issue::Traitee }).await.unwrap();
    // Chaîne : rangs consécutifs, empreintes liées, vérifiée par la base.
    let faits: Vec<(i32, String, String, String)> = sqlx::query_as(
        "SELECT rang, type, empreinte, empreinte_precedente FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY rang",
    )
    .bind(m.etab)
    .bind(etat.id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let types: Vec<&str> = faits.iter().map(|f| f.1.as_str()).collect();
    assert_eq!(types, ["depose", "prise", "relache", "close"]);
    assert_eq!(faits.iter().map(|f| f.0).collect::<Vec<_>>(), [1, 2, 3, 4]);
    assert_eq!(faits[0].3, "0".repeat(64));
    for w in faits.windows(2) {
        assert_eq!(w[1].3, w[0].2, "chaque fait pointe l'empreinte du précédent");
    }
    assert!(faits.iter().all(|f| f.2.len() == 64 && f.2.chars().all(|c| c.is_ascii_hexdigit())));
    assert_eq!(ent.verifier_chaine_demande(m.etab, etat.id).await.unwrap(), None);
    // Ni modification, ni suppression, ni vidage d'un fait — pour personne, propriétaire compris.
    let pool = ent.pool_verif();
    assert!(sqlx::query("UPDATE ent_dem_fait SET motif_code = 'x' WHERE etablissement_id = $1 AND demande_id = $2").bind(m.etab).bind(etat.id).execute(pool).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2").bind(m.etab).bind(etat.id).execute(pool).await.is_err());
    assert!(sqlx::query("TRUNCATE ent_dem_fait").execute(pool).await.is_err());
    // Ni suppression de la demande (aucune cascade), ni changement de son identité.
    assert!(sqlx::query("DELETE FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2").bind(m.etab).bind(etat.id).execute(pool).await.is_err());
    assert!(sqlx::query("UPDATE ent_dem_demande SET numero = 'D9999-99999', version = version + 1 WHERE etablissement_id = $1 AND id = $2").bind(m.etab).bind(etat.id).execute(pool).await.is_err());
    assert!(sqlx::query("UPDATE ent_dem_demande SET demandeur_personne_id = $3, version = version + 1 WHERE etablissement_id = $1 AND id = $2").bind(m.etab).bind(etat.id).bind(m.prof.personne).execute(pool).await.is_err());
    // Aucune suppression en cascade : la personne qui a déposé ne s'efface pas tant que sa demande existe.
    assert!(sqlx::query("DELETE FROM ent_personne WHERE etablissement_id = $1 AND id = $2").bind(m.etab).bind(m.cpe.personne).execute(pool).await.is_err());
    // Une altération se voit : on désactive les déclencheurs DANS une transaction annulée, on falsifie, on vérifie.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_dem_fait DISABLE TRIGGER USER").execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE ent_dem_fait SET motif_code = 'falsifie' WHERE etablissement_id = $1 AND demande_id = $2 AND rang = 3").bind(m.etab).bind(etat.id).execute(&mut *tx).await.unwrap();
    let (rang,): (Option<i32>,) = sqlx::query_as("SELECT ent_dem_verifier_chaine($1, $2)").bind(m.etab).bind(etat.id).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(rang, Some(3), "le premier maillon altéré est signalé");
    tx.rollback().await.unwrap();
    assert_eq!(ent.verifier_chaine_demande(m.etab, etat.id).await.unwrap(), None, "retour à l'état intact");
    // Une demande terminée refuse tout nouveau geste.
    let r = ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, etat.id, 4, Geste::Prendre).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))), "{r:?}");
}

#[tokio::test]
async fn t_nj89_purge_par_ordonnanceur() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let pool = ent.pool_verif();
    // Trois demandes closes : technique (12 mois), gelée, et une RH (jamais purgée automatiquement).
    let technique = deposer(&ent, &m, &m.cpe, depot_maintenance()).await.unwrap();
    let gelee = deposer(&ent, &m, &m.cpe, depot_maintenance()).await.unwrap();
    let mut rh = depot("rh.demande_agent");
    rh.valeurs = vec![ValeurSaisie { champ: "type_demande".into(), rang: 0, valeur: Valeur::Code("formation".into()) }];
    let rh = deposer(&ent, &m, &m.cpe, rh).await.unwrap();
    for id in [technique.id, gelee.id, rh.id] {
        ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, id, 1, Geste::Cloturer { issue: Issue::Traitee }).await.unwrap();
    }
    // Durée par nature, calculée à la clôture depuis le jour métier de l'établissement (§15.3).
    let jour = domain_eleve::horloge::jour_paris();
    let conservations: Vec<(Uuid, Option<NaiveDate>)> = sqlx::query_as("SELECT id, conservation_jusqu_au FROM ent_dem_demande WHERE etablissement_id = $1")
        .bind(m.etab).fetch_all(pool).await.unwrap();
    let conservation = |id: Uuid| conservations.iter().find(|c| c.0 == id).unwrap().1;
    let attendue = jour.checked_add_months(chrono::Months::new(12)).unwrap();
    assert!(conservation(technique.id).is_some_and(|d| (d - attendue).num_days().abs() <= 1), "12 mois pour une demande technique");
    assert_eq!(conservation(rh.id), None, "RH : pas de purge automatique");

    // LECTURE ≠ PURGE (piège 16) : même échue, la demande n'est jamais purgée par une lecture.
    for id in [technique.id, gelee.id, rh.id] {
        sqlx::query("UPDATE ent_dem_demande SET conservation_jusqu_au = $3, gel = ($2 = $4), version = version + 1 WHERE etablissement_id = $1 AND id = $2")
            .bind(m.etab).bind(id).bind(jour - chrono::Duration::days(1)).bind(gelee.id).execute(pool).await.unwrap();
    }
    // (la demande RH garde une date échue artificielle : elle prouve que seule la catégorie décide à la clôture ;
    //  ici on la remet sans échéance pour le test de purge)
    sqlx::query("UPDATE ent_dem_demande SET conservation_jusqu_au = NULL, version = version + 1 WHERE etablissement_id = $1 AND id = $2").bind(m.etab).bind(rh.id).execute(pool).await.unwrap();
    let _ = ent.mes_demandes_for_actor(m.etab, m.cpe.user).await.unwrap();
    let _ = ent.ma_file_for_actor(m.etab, m.prof.user, "prof", &m.src).await.unwrap();
    let _ = ent.demande_traitant_for_actor(m.etab, m.prof.user, "prof", &m.src, RefDemande::Id(technique.id)).await.unwrap();
    let (purgees,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_demande WHERE etablissement_id = $1 AND purgee_le IS NOT NULL").bind(m.etab).fetch_one(pool).await.unwrap();
    assert_eq!(purgees, 0, "aucune lecture ne purge");
    // Le contenu n'est pas effaçable à la main : la base n'accepte la purge que sous le réglage de l'ordonnanceur.
    let direct = sqlx::query("UPDATE ent_dem_demande SET objet = NULL, purgee_le = now(), version = version + 1 WHERE etablissement_id = $1 AND id = $2")
        .bind(m.etab).bind(technique.id).execute(pool).await;
    assert!(direct.is_err(), "purge hors ordonnanceur acceptée");

    // L'ordonnanceur passe : seule la demande échue, close et non gelée est purgée.
    let rapport = ent.purger_demandes_echues(m.etab).await.unwrap();
    assert_eq!(rapport.purgees, 1);
    assert_eq!(rapport.par_nature.get("maintenance.intervention"), Some(&1));
    assert_eq!(rapport.gelees_ignorees, 1, "la demande gelée est comptée et laissée");
    let ligne = |id: Uuid| {
        sqlx::query_as::<_, (Option<String>, Option<Uuid>, Option<chrono::DateTime<chrono::Utc>>)>(
            "SELECT objet, demandeur_personne_id, purgee_le FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(m.etab)
        .bind(id)
        .fetch_one(pool)
    };
    let p = ligne(technique.id).await.unwrap();
    assert!(p.0.is_none() && p.1.is_none() && p.2.is_some(), "contenu et identité effacés");
    let g = ligne(gelee.id).await.unwrap();
    assert!(g.0.is_some() && g.1.is_some() && g.2.is_none(), "gel respecté");
    let r = ligne(rh.id).await.unwrap();
    assert!(r.0.is_some() && r.2.is_none(), "RH jamais purgée automatiquement");
    // Valeurs effacées pour la purgée, gardées pour les autres.
    let (v,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_valeur WHERE etablissement_id = $1 AND demande_id = $2").bind(m.etab).bind(technique.id).fetch_one(pool).await.unwrap();
    assert_eq!(v, 0);
    let (v,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_valeur WHERE etablissement_id = $1 AND demande_id = $2").bind(m.etab).bind(gelee.id).fetch_one(pool).await.unwrap();
    assert_eq!(v, 1);
    // La preuve reste : faits conservés, chaîne intacte, fait de purge signé par la tâche (jamais une personne).
    assert_eq!(ent.verifier_chaine_demande(m.etab, technique.id).await.unwrap(), None);
    let (derniere, tache): (String, Option<String>) = sqlx::query_as("SELECT type, auteur_tache FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY rang DESC LIMIT 1")
        .bind(m.etab).bind(technique.id).fetch_one(pool).await.unwrap();
    assert_eq!((derniere.as_str(), tache.as_deref()), ("purgee", Some("purge_conservation")));
    // Une demande purgée disparaît des projections et n'accepte plus d'écriture.
    assert_eq!(ent.demande_demandeur_for_actor(m.etab, m.cpe.user, technique.id).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(
        ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, technique.id, 3, Geste::Prendre).await,
        Err(RefusDemande::Introuvable)
    );
    // Idempotence : un second passage ne trouve plus rien.
    assert_eq!(ent.purger_demandes_echues(m.etab).await.unwrap().purgees, 0);
}

// ───────────────────────── NJ-86 : enveloppe neutre ─────────────────────────

#[tokio::test]
async fn t_nj86_enveloppe_neutre() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let mut d = depot("rh.demande_agent");
    d.objet = "Question de santé pour Mme Dupont".into();
    d.texte = Some("Mon numéro est 06 12 34 56 78, écrivez à jeanne.dupont@exemple.test".into());
    d.valeurs = vec![ValeurSaisie { champ: "type_demande".into(), rang: 0, valeur: Valeur::Code("temps_partiel".into()) }];
    let etat = deposer(&ent, &m, &m.cpe, d).await.unwrap();
    ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, etat.id, 1, Geste::Prendre).await.unwrap();
    ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, etat.id, 2, Geste::Cloturer { issue: Issue::Accordee }).await.unwrap();
    let evenements: Vec<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT type, payload FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2 ORDER BY created_at, id",
    )
    .bind(m.etab)
    .bind(etat.id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let types: Vec<&str> = evenements.iter().map(|e| e.0.as_str()).collect();
    assert_eq!(types, ["demande.deposee", "demande.prise", "demande.close"]);
    for (type_, charge) in &evenements {
        assert!(controler_enveloppe(charge).is_ok(), "{type_} : {charge}");
        let texte = charge.to_string();
        for interdit in ["Dupont", "santé", "06 12", "@", "temps_partiel", "Question", "numéro", "texte", "objet"] {
            assert!(!texte.contains(interdit), "{type_} : « {interdit} » dans l'enveloppe {texte}");
        }
        assert_eq!(charge["lien"], format!("/demandes/{}", etat.id), "lien interne seulement");
    }
    // Les faits non plus ne portent aucun texte : seulement identifiants et codes.
    let (colonnes_texte,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM information_schema.columns WHERE table_name = 'ent_dem_fait' AND column_name IN ('texte', 'objet', 'message', 'commentaire')",
    )
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(colonnes_texte, 0);
}

// ───────────────────────── Confidentialité C2 : texte scellé et lecture tracée ─────────────────────────

#[tokio::test]
async fn texte_c2_scelle_et_lecture_tracee() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let mut d = depot("rh.demande_agent");
    d.texte = Some("Je souhaite un temps partiel pour raison familiale".into());
    d.valeurs = vec![ValeurSaisie { champ: "type_demande".into(), rang: 0, valeur: Valeur::Code("temps_partiel".into()) }];
    let etat = deposer(&ent, &m, &m.cpe, d).await.unwrap();
    // En base : le texte n'est jamais en clair (C2).
    let (stocke, scelle): (Option<String>, bool) = sqlx::query_as("SELECT texte, texte_scelle FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
        .bind(m.etab).bind(etat.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(scelle && stocke.as_deref().is_some_and(|s| s.starts_with("enc:v1:") && !s.contains("temps partiel")));
    // Un texte en clair en C2 est refusé par la base elle-même.
    let clair = sqlx::query("UPDATE ent_dem_demande SET texte = 'en clair', texte_scelle = false, version = version + 1 WHERE etablissement_id = $1 AND id = $2")
        .bind(m.etab).bind(etat.id).execute(ent.pool_verif()).await;
    assert!(clair.is_err(), "texte C2 en clair accepté");
    // Le demandeur relit son texte ; le membre de la file aussi, et sa lecture est tracée (R-DM16).
    let vue = ent.demande_demandeur_for_actor(m.etab, m.cpe.user, etat.id).await.unwrap();
    assert_eq!(vue.texte.as_deref(), Some("Je souhaite un temps partiel pour raison familiale"));
    let avant = nb_faits(&ent, m.etab, etat.id).await;
    let t = ent.demande_traitant_for_actor(m.etab, m.prof.user, "prof", &m.src, RefDemande::Id(etat.id)).await.unwrap();
    assert_eq!(t.texte.as_deref(), vue.texte.as_deref());
    assert_eq!(t.confidentialite, "C2");
    assert_eq!(nb_faits(&ent, m.etab, etat.id).await, avant + 1, "lecture C2 tracée par un fait");
    // « Ma file » ne porte jamais de texte, C0 comme C2.
    let file = ent.ma_file_for_actor(m.etab, m.prof.user, "prof", &m.src).await.unwrap();
    assert!(file.iter().all(|d| d.texte.is_none()));
    // Un texte C1 reste lisible tel quel et la lecture n'est pas tracée.
    let c1 = une_demande(&ent, &m, &m.cpe).await;
    let avant = nb_faits(&ent, m.etab, c1.id).await;
    ent.demande_traitant_for_actor(m.etab, m.prof.user, "prof", &m.src, RefDemande::Id(c1.id)).await.unwrap();
    assert_eq!(nb_faits(&ent, m.etab, c1.id).await, avant);
}

// ───────────────────────── NJ-72 : limitation de débit ─────────────────────────

#[tokio::test]
async fn t_nj72_debit_depot() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let quotas = QuotasDemandes { depot: Quota::new(3, 3600), geste: Quota::new(1000, 3600) };
    for i in 0..3 {
        ent.deposer_demande_avec_quotas(m.etab, m.cpe.user, "cpe", &m.src, depot_maintenance(), quotas)
            .await
            .unwrap_or_else(|e| panic!("dépôt {i} : {e:?}"));
    }
    // Le quatrième dépôt de la même personne est refusé (429) : aucune demande créée.
    let r = ent.deposer_demande_avec_quotas(m.etab, m.cpe.user, "cpe", &m.src, depot_maintenance(), quotas).await;
    let Err(RefusDemande::TropDeDemandes { retry_after_secs }) = r else { panic!("{r:?}") };
    assert!(retry_after_secs >= 1);
    assert_eq!(RefusDemande::TropDeDemandes { retry_after_secs }.statut_http(), 429);
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_demande WHERE etablissement_id = $1").bind(m.etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(n, 3);
    // La limite est PAR PERSONNE (clé = compte authentifié) : un autre compte n'est pas pénalisé,
    // et il ne suffit pas de changer d'en-tête ou d'adresse pour l'éviter (aucune adresse n'entre dans la clé).
    ent.deposer_demande_avec_quotas(m.etab, m.prof.user, "prof", &m.src, depot_maintenance(), quotas).await.unwrap();
    // Le refus pour débit passe avant tout travail : même une nature inexistante est limitée.
    let mut inconnue = depot("secretariat.demande_interne");
    inconnue.nature_code = "inconnue.nature".into();
    assert!(matches!(
        ent.deposer_demande_avec_quotas(m.etab, m.cpe.user, "cpe", &m.src, inconnue, quotas).await,
        Err(RefusDemande::TropDeDemandes { .. })
    ));
    // Quota par défaut raisonnable et réglable.
    let defaut = QuotasDemandes::depuis_env();
    assert!(defaut.depot.max >= 5 && defaut.geste.max >= defaut.depot.max);
}

#[tokio::test]
async fn t_nj72_debit_gestes() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let etat = une_demande(&ent, &m, &m.cpe).await;
    let quotas = QuotasDemandes { depot: Quota::new(1000, 3600), geste: Quota::new(2, 3600) };
    for _ in 0..2 {
        // Hors file : 404, mais le débit est décompté (on ne sonde pas les identifiants en rafale).
        let r = ent.agir_avec_quotas(m.etab, m.direction.user, "direction", &m.src, etat.id, 1, Geste::Prendre, quotas).await;
        assert_eq!(r, Err(RefusDemande::Introuvable));
    }
    let r = ent.agir_avec_quotas(m.etab, m.direction.user, "direction", &m.src, etat.id, 1, Geste::Prendre, quotas).await;
    assert!(matches!(r, Err(RefusDemande::TropDeDemandes { .. })), "{r:?}");
}

// ───────────────────────── NJ-93 : erreurs neutres ─────────────────────────

#[tokio::test]
async fn t_nj93_erreurs_neutres() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let interdits = ["sqlx", "postgres", "ent_dem", "constraint", "violates", "pool", "closed", "relation", "column", "cfk", "SELECT", "INSERT"];
    let neutre = |r: &RefusDemande| {
        let msg = r.message_public().to_string();
        assert_eq!(r.to_string(), msg, "Display = message public");
        for i in interdits {
            assert!(!msg.to_lowercase().contains(&i.to_lowercase()), "« {i} » dans « {msg} »");
        }
        assert!(!msg.is_empty());
    };
    // 1. Violation de clé étrangère réelle (élève inexistant) : message fixe, aucun nom de contrainte.
    let src = Sources { famille: true, ..m.src.clone() };
    let mut d = depot("scolarite.attestation");
    d.eleve_id = Some(Uuid::new_v4());
    d.valeurs = vec![ValeurSaisie { champ: "type_attestation".into(), rang: 0, valeur: Valeur::Code("assiduite".into()) }];
    let r = ent.deposer_demande_for_actor(m.etab, m.parent.user, "parent", &src, d).await.unwrap_err();
    assert_eq!(r, RefusDemande::Indisponible);
    neutre(&r);
    // 2. Panne de la base : même réponse fixe.
    let url = crate::url_guard::url_labo_ou_echec().unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let en_panne = PgEnt::from_pool(pool.clone());
    pool.close().await;
    let r = en_panne.deposer_demande_for_actor(m.etab, m.cpe.user, "cpe", &m.src, depot_maintenance()).await.unwrap_err();
    assert_eq!(r, RefusDemande::Indisponible);
    neutre(&r);
    // 3. Toutes les variantes ont une phrase fixe sans donnée.
    for r in [
        RefusDemande::Introuvable,
        RefusDemande::Interdit,
        RefusDemande::Invalide("Une des informations saisies n'est pas valide."),
        RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."),
        RefusDemande::TropDeDemandes { retry_after_secs: 5 },
        RefusDemande::Indisponible,
    ] {
        neutre(&r);
    }
    // 4. Une erreur de base brute n'a aucune voie vers le client : la conversion perd le détail.
    let brute = sqlx::Error::Protocol("secret: mot de passe=hunter2 table ent_dem_demande".into());
    let r: RefusDemande = brute.into();
    assert_eq!(r, RefusDemande::Indisponible);
    assert!(!format!("{r:?}").contains("hunter2") && !r.message_public().contains("hunter2"));
}

// ───────────────────────── Annulation et clôture ─────────────────────────

#[tokio::test]
async fn annulation_par_le_demandeur_avant_la_prise() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let a = une_demande(&ent, &m, &m.cpe).await;
    // Seul le demandeur (ou le déposant) annule ; un membre de la file ne l'annule pas à sa place.
    assert_eq!(
        ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, a.id, 1, Geste::Annuler).await,
        Err(RefusDemande::Interdit)
    );
    let fin = ent.agir_sur_demande_for_actor(m.etab, m.cpe.user, "cpe", &m.src, a.id, 1, Geste::Annuler).await.unwrap();
    assert_eq!(fin.statut, "annulee");
    let vue = ent.demande_demandeur_for_actor(m.etab, m.cpe.user, a.id).await.unwrap();
    assert_eq!((vue.statut.as_str(), vue.issue.as_deref()), ("annulee", Some("annulee_par_demandeur")));
    assert!(vue.clos_le.is_some());
    // Échéances closes avec la demande (rien n'est supprimé).
    let (ouvertes,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2 AND traitee_le IS NULL")
        .bind(m.etab).bind(a.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(ouvertes, 0);
    // Une demande déjà prise ne s'annule plus : le demandeur écrit au service (§8.2).
    let b = une_demande(&ent, &m, &m.cpe).await;
    ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, b.id, 1, Geste::Prendre).await.unwrap();
    let r = ent.agir_sur_demande_for_actor(m.etab, m.cpe.user, "cpe", &m.src, b.id, 2, Geste::Annuler).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))), "{r:?}");
    // Issue réservée au demandeur refusée au traitant ; l'issue « accordée » porte sa décision.
    let r = ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, b.id, 2, Geste::Cloturer { issue: Issue::AnnuleeParDemandeur }).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
    ent.agir_sur_demande_for_actor(m.etab, m.prof.user, "prof", &m.src, b.id, 2, Geste::Cloturer { issue: Issue::Refusee }).await.unwrap();
    let (decision, issue): (Option<String>, Option<String>) = sqlx::query_as("SELECT decision, issue FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
        .bind(m.etab).bind(b.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!((decision.as_deref(), issue.as_deref()), (Some("refusee"), Some("refusee")));
}

// ───────────────────────── Numérotation ─────────────────────────

#[tokio::test]
async fn numerotation_lisible_unique_sous_concurrence() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let ent = std::sync::Arc::new(ent);
    let quotas = QuotasDemandes { depot: Quota::new(1000, 3600), geste: Quota::new(1000, 3600) };
    let mut taches = Vec::new();
    for qui in [m.prof, m.cpe, m.direction, m.compta, m.prof, m.cpe] {
        let (ent, src, etab) = (ent.clone(), m.src.clone(), m.etab);
        taches.push(tokio::spawn(async move {
            ent.deposer_demande_avec_quotas(etab, qui.user, qui.role, &src, depot("secretariat.demande_interne"), quotas).await
        }));
    }
    let mut numeros = Vec::new();
    for t in taches {
        numeros.push(t.await.unwrap().expect("dépôt").numero);
    }
    numeros.sort();
    numeros.dedup();
    assert_eq!(numeros.len(), 6, "six numéros distincts : {numeros:?}");
    assert!(numeros.iter().all(|n| n.starts_with('D') && n.len() == 11), "{numeros:?}");
    let suffixes: Vec<i32> = numeros.iter().map(|n| n[6..].parse().unwrap()).collect();
    assert_eq!(suffixes, [1, 2, 3, 4, 5, 6], "numérotation sans trou");
}

// ───────────────────────── Séparation entre établissements (piège 8) ─────────────────────────

#[tokio::test]
async fn separation_entre_etablissements() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let a = monde(&ent, &cast).await;
    let cast_b = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let mut b = monde(&ent, &cast_b).await;
    // Même file (même identifiant) des deux côtés : seul le cloisonnement d'établissement protège B.
    b.src.file = a.src.file;
    let da = une_demande(&ent, &a, &a.cpe).await;
    let db = une_demande(&ent, &b, &b.cpe).await;
    assert_ne!(da.id, db.id);
    // Chaque établissement a sa numérotation propre.
    assert_eq!(da.numero[6..], db.numero[6..], "premier numéro de chaque établissement");
    // Les identifiants de B ne se lisent ni ne s'écrivent avec la session de A (404 neutre, même file).
    let src_commune = Sources { file: a.src.file, membres: vec![a.prof.personne, b.prof.personne], famille: false };
    assert_eq!(ent.demande_demandeur_for_actor(a.etab, a.cpe.user, db.id).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(
        ent.demande_traitant_for_actor(a.etab, a.prof.user, "prof", &src_commune, RefDemande::Id(db.id)).await.unwrap_err(),
        RefusDemande::Introuvable
    );
    assert_eq!(
        ent.agir_sur_demande_for_actor(a.etab, a.prof.user, "prof", &src_commune, db.id, 1, Geste::Prendre).await,
        Err(RefusDemande::Introuvable)
    );
    // Le compte de A n'a aucune fiche chez B : ses requêtes sur B sont neutres.
    assert_eq!(
        ent.agir_sur_demande_for_actor(b.etab, a.prof.user, "prof", &src_commune, db.id, 1, Geste::Prendre).await,
        Err(RefusDemande::Introuvable)
    );
    // Les compteurs ne fuient pas : « ma file » de A ne contient que du A.
    let src_a = Sources { membres: vec![a.prof.personne], ..a.src.clone() };
    let ma_file = ent.ma_file_for_actor(a.etab, a.prof.user, "prof", &src_a).await.unwrap();
    assert_eq!(ma_file.len(), 1);
    assert_eq!(ma_file[0].id, da.id);
}

// ───────────────────────── Restauration : échéances suspendues ─────────────────────────

#[tokio::test]
async fn restauration_suspend_les_echeances() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let a = une_demande(&ent, &m, &m.cpe).await;
    let ouvertes = |ent: &PgEnt| {
        sqlx::query_as::<_, (i64, i64)>(
            "SELECT count(*) FILTER (WHERE traitee_le IS NULL AND suspendue_depuis IS NULL),
                    count(*) FILTER (WHERE suspendue_depuis IS NOT NULL)
               FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2",
        )
        .bind(m.etab)
        .bind(a.id)
        .fetch_one(ent.pool_verif())
    };
    assert_eq!(ouvertes(&ent).await.unwrap(), (2, 0));
    let mut conn = ent.pool_verif().acquire().await.unwrap();
    let rapport = crate::registre_donnees::neutraliser_files_restauration(
        &mut conn,
        crate::registre_donnees::PerimetreNeutralisation::Etablissement(m.etab),
    )
    .await
    .unwrap();
    assert!(rapport.tables.iter().any(|t| t.table == "ent_dem_echeance" && t.lignes == 2), "{:?}", rapport.tables);
    drop(conn);
    // Suspendues, jamais supprimées ; un second passage ne trouve plus rien.
    assert_eq!(ouvertes(&ent).await.unwrap(), (0, 2));
}
