//! Socle 12, lot AS2 (appel v2, situations, corrections). Réf. : `docs/socles/12_ASSIDUITE.md` §8, §9,
//! §17.1, §17.4, §24. Critères de fin : séance annulée après l'appel → absences neutralisées et
//! démenti ; élève inscrit après l'ouverture → ajouté ; prévenu à 8 h 05 → visible dans l'appel
//! ouvert ; toute correction historisée ; deux ouvertures simultanées → même appel.
//! Sans `ENT_DATABASE_URL` : ignorés par absence de base (le test échoue en CI, qui fournit Postgres).

use super::*;
use crate::droits_repo::{Droit, RoleSaisie};
use chrono::{NaiveDate, NaiveTime};
use domain_eleve::{CreateEleve, DeclarerRetard, PoserEdtException};
use ent_auth::{Action, Portee, Role};
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| { e.install_brain_p0(); }).await;
    Some((ent, cast))
}

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

fn interdit<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

async fn aujourdhui(ent: &PgEnt, cast: &LaboCast) -> NaiveDate {
    ent.aujourdhui_etablissement(cast.etablissement_id).await.unwrap()
}

async fn creneau(ent: &PgEnt, cast: &LaboCast, jour: NaiveDate, debut: (u32, u32), fin: (u32, u32)) -> Uuid {
    ent.seed_creneau_du_jour(cast.etablissement_id, "4B", "AS2", cast.prof.user_id, jour, t(debut.0, debut.1), t(fin.0, fin.1), None)
        .await
        .expect("créneau")
        .id
}

/// Appel ouvert de 4B (Rogue) aujourd'hui entre 8 h et 9 h.
async fn appel_ouvert(ent: &PgEnt, cast: &LaboCast, jour: NaiveDate) -> Uuid {
    let c = creneau(ent, cast, jour, (8, 0), (9, 0)).await;
    ent.ouvrir_appel(cast.etablissement_id, c, jour, cast.prof.user_id, "prof").await.expect("ouvrir").0.id
}

fn saisie(etat: &str) -> SaisieLigne {
    SaisieLigne { etat: etat.into(), minutes: None, lieu_code: None }
}

async fn pointer(ent: &PgEnt, cast: &LaboCast, appel: Uuid, eleve: Uuid, etat: &str) -> LigneAppelV2 {
    ent.pointer_ligne_v2(cast.etablissement_id, appel, eleve, cast.prof.user_id, "prof", saisie(etat), "web")
        .await
        .expect("pointer")
}

async fn valider(ent: &PgEnt, cast: &LaboCast, appel: Uuid) -> AppelV2 {
    ent.valider_appel_v2(cast.etablissement_id, appel, cast.prof.user_id, "prof",
        ValidationAppel { confirme_non_pointes: false, canal: None }).await.expect("valider")
}

async fn n(ent: &PgEnt, sql: &str, a: Uuid, b: Uuid) -> i64 {
    let (c,): (i64,) = sqlx::query_as(sql).bind(a).bind(b).fetch_one(ent.pool_verif()).await.unwrap();
    c
}

async fn ligne(ent: &PgEnt, cast: &LaboCast, appel: Uuid, eleve: Uuid) -> LigneAppelV2 {
    ent.appel_v2_lu(cast.etablissement_id, appel, cast.prof.user_id, "prof").await.unwrap()
        .lignes.into_iter().find(|l| l.eleve_id == eleve).expect("ligne")
}

async fn declarer(ent: &PgEnt, cast: &LaboCast, jour: NaiveDate, debut: Option<NaiveTime>, fin: Option<NaiveTime>) -> Uuid {
    ent.declarer_absence_accueil(cast.cpe.user_id, "cpe", DeclarerAbsence {
        etablissement_id: cast.etablissement_id,
        eleve_id: cast.eleve_lea_id,
        debut_jour: jour,
        debut_heure: debut,
        fin_jour: jour,
        fin_heure: fin,
        motif: "maladie".into(),
        commentaire: Some("Mon fils a une gastro-entérite".into()),
        declarant_nom: Some("Lily Potter".into()),
        declarant_qualite: Some("mère".into()),
        piece_document_id: None,
    }).await.expect("déclaration").id
}

// ─── Ouverture : atomique et idempotente (DA-08) ────────────────────────────

#[tokio::test]
async fn as2_deux_ouvertures_simultanees_donnent_le_meme_appel() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let jour = aujourdhui(&ent, &cast).await;
    let c = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    let (a, b, d, e) = tokio::join!(
        ent.ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof"),
        ent.ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof"),
        ent.ouvrir_appel(etab, c, jour, cast.cpe.user_id, "cpe"),
        ent.ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof"),
    );
    let (a, b, d, e) = (a.unwrap(), b.unwrap(), d.unwrap(), e.unwrap());
    assert!(a.0.id == b.0.id && b.0.id == d.0.id && d.0.id == e.0.id, "un seul appel pour la séance");
    assert!(a.1.len() >= 2, "effectif complet dès la première lecture");
    for x in [&b, &d, &e] {
        assert_eq!(x.1.len(), a.1.len(), "jamais de lignes partielles");
    }
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_appel WHERE etablissement_id = $1 AND creneau_id = $2", etab, c).await, 1);
    // Un seul événement d'ouverture.
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'appel.opened'", etab, a.0.id).await, 1);
}

// ─── Effectif et pré-remplissage actualisés (DA-19, §9.3) ───────────────────

#[tokio::test]
async fn as2_eleve_arrive_apres_l_ouverture_est_ajoute() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let jour = aujourdhui(&ent, &cast).await;
    let appel = appel_ouvert(&ent, &cast, jour).await;
    let avant = ent.appel_v2_lu(etab, appel, cast.prof.user_id, "prof").await.unwrap();
    let nouveau = ent.create_eleve(CreateEleve {
        etablissement_id: etab, nom: "Arrivant".into(), prenom: "Tardif".into(), ine: None, sexe: None,
        date_naissance: None, numero_badge: None, redoublant: false, classe: Some("4B".into()), regime: None,
        mef: None, site: None, date_entree: None, date_sortie: None, motif_sortie: None, actor_user_id: None,
    }).await.expect("inscription après l'ouverture");
    let apres = ent.appel_v2_lu(etab, appel, cast.prof.user_id, "prof").await.unwrap();
    assert_eq!(apres.lignes.len(), avant.lignes.len() + 1, "DA-19 : l'élève arrivé est ajouté");
    let l = apres.lignes.iter().find(|l| l.eleve_id == nouveau.id).expect("ligne du nouvel élève");
    assert_eq!(l.etat, "present");
    // L'ajout est historisé (système), et rien n'est retiré.
    let h = ent.historique_ligne_for_actor(etab, appel, nouveau.id, cast.prof.user_id, "prof").await.unwrap();
    assert_eq!((h.len(), h[0].canal.as_str()), (1, "systeme"));
    // Une fois validé, l'effectif est figé.
    valider(&ent, &cast, appel).await;
    ent.create_eleve(CreateEleve {
        etablissement_id: etab, nom: "Trop".into(), prenom: "Tard".into(), ine: None, sexe: None,
        date_naissance: None, numero_badge: None, redoublant: false, classe: Some("4B".into()), regime: None,
        mef: None, site: None, date_entree: None, date_sortie: None, motif_sortie: None, actor_user_id: None,
    }).await.unwrap();
    let fige = ent.appel_v2_lu(etab, appel, cast.prof.user_id, "prof").await.unwrap();
    assert_eq!(fige.lignes.len(), apres.lignes.len(), "effectif figé à la validation");
}

#[tokio::test]
async fn as2_declaration_de_8h05_visible_dans_l_appel_ouvert_sans_motif() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let (jour, lea) = (aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let appel = appel_ouvert(&ent, &cast, jour).await;
    assert_eq!(ligne(&ent, &cast, appel, lea).await.etat, "present");
    // 8 h 05 : l'accueil enregistre l'appel d'une maman.
    let decl = declarer(&ent, &cast, jour, None, None).await;
    let l = ligne(&ent, &cast, appel, lea).await;
    assert_eq!((l.etat.as_str(), l.pre_remplie, l.libelle_court.as_deref()), ("absent", true, Some("prévenu")));
    // R-AS19 / R-AS21 / R-AS22 : le DTO de l'appel ne porte NI motif NI texte de la déclaration.
    let json = serde_json::to_string(&ent.appel_v2_lu(etab, appel, cast.prof.user_id, "prof").await.unwrap()).unwrap().to_lowercase();
    for interdit in ["gastro", "maladie", "motif", "commentaire", "potter", "lily", "declarant"] {
        assert!(!json.contains(interdit), "le professeur ne doit pas voir « {interdit} » : {json}");
    }
    // Le texte libre est séparé : le professeur ne le lit pas, la vie scolaire si (NJ-35).
    let (sit,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_asd_situation WHERE etablissement_id = $1 AND origine_ref_id = $2")
        .bind(etab).bind(decl).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(matches!(ent.texte_situation_for_actor(etab, cast.prof.user_id, "prof", sit).await, Err(EntDbError::Domain(_))));
    assert!(matches!(ent.texte_situation_for_actor(etab, cast.prof.user_id, "prof", Uuid::new_v4()).await, Err(EntDbError::Domain(_))),
        "même réponse pour « interdit » et « inexistant » (R-AS35)");
    let texte = ent.texte_situation_for_actor(etab, cast.cpe.user_id, "cpe", sit).await.unwrap();
    assert_eq!(texte, vec!["Mon fils a une gastro-entérite".to_string()]);
    // La déclaration annulée : la ligne pré-remplie (non touchée) redevient « présent ».
    ent.annuler_declaration(etab, decl, cast.cpe.user_id, "cpe").await.expect("annuler");
    let l = ligne(&ent, &cast, appel, lea).await;
    assert_eq!((l.etat.as_str(), l.pre_remplie), ("present", false));
}

#[tokio::test]
async fn as2_une_ligne_saisie_a_la_main_n_est_jamais_ecrasee_par_une_situation() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (jour, lea) = (aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let appel = appel_ouvert(&ent, &cast, jour).await;
    pointer(&ent, &cast, appel, lea, "present").await; // le professeur la voit en classe
    declarer(&ent, &cast, jour, None, None).await;
    let l = ligne(&ent, &cast, appel, lea).await;
    assert_eq!((l.etat.as_str(), l.pre_remplie), ("present", false), "le geste du professeur prime (§8.2)");
    // Présent malgré une déclaration : incohérence signalée à la vie scolaire, rien n'est modifié.
    let inc = ent.incoherences_appel_for_actor(cast.etablissement_id, appel, cast.cpe.user_id, "cpe").await.unwrap();
    assert!(inc.is_empty(), "la ligne n'est pas liée à la déclaration (posée avant) : {inc:?}");
}

#[tokio::test]
async fn as2_present_malgre_une_declaration_est_signale_et_non_corrige() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (jour, lea) = (aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let appel = appel_ouvert(&ent, &cast, jour).await;
    declarer(&ent, &cast, jour, None, None).await;
    assert_eq!(ligne(&ent, &cast, appel, lea).await.etat, "absent");
    pointer(&ent, &cast, appel, lea, "present").await; // « l'enfant est finalement venu »
    let l = ligne(&ent, &cast, appel, lea).await;
    assert_eq!(l.etat, "present", "le professeur a raison sur ce qu'il voit");
    let inc = ent.incoherences_appel_for_actor(cast.etablissement_id, appel, cast.cpe.user_id, "cpe").await.unwrap();
    assert_eq!(inc, vec![IncoherenceV2 { eleve_id: lea, code: "present_malgre_declaration".into() }]);
    // Un professeur étranger à l'appel ne lit pas les incohérences.
    assert!(interdit(ent.incoherences_appel_for_actor(cast.etablissement_id, appel, cast.profs[1].user_id, "prof").await));
}

#[tokio::test]
async fn as2_localisation_infirmerie_prerempli_et_n_est_jamais_une_absence() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let appel = appel_ouvert(&ent, &cast, jour).await;
    let inf = cast.infirmieres[0].user_id;
    // Droit `localisation.poser` : pas pour un professeur (R-AS20 : la localisation est posée par l'infirmerie, la vie scolaire).
    let p = || PoserLocalisation { eleve_id: lea, lieu_code: "infirmerie".into(), jour: None, heure_debut: None, heure_fin: None };
    assert!(interdit(ent.poser_localisation_for_actor(etab, cast.prof.user_id, "prof", p()).await));
    let mauvais = PoserLocalisation { lieu_code: "cantine".into(), ..p() };
    assert!(matches!(ent.poser_localisation_for_actor(etab, inf, "infirmerie", mauvais).await, Err(EntDbError::SaisieInvalide(_))));
    ent.poser_localisation_for_actor(etab, inf, "infirmerie", p()).await.expect("localiser");
    // L'appel déjà ouvert l'a vue (rafraîchissement) : localisé, jamais absent.
    let l = ligne(&ent, &cast, appel, lea).await;
    assert_eq!((l.etat.as_str(), l.lieu_code.as_deref(), l.libelle_court.as_deref()), ("localise", Some("infirmerie"), Some("à l'infirmerie")));
    let v = valider(&ent, &cast, appel).await;
    assert_eq!(v.recapitulatif.localises, 1);
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2", etab, lea).await, 0, "R-AS4 : localisé n'est pas absent");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence_seance WHERE etablissement_id = $1 AND eleve_id = $2", etab, lea).await, 0);
}

#[tokio::test]
async fn as2_en_sortie_est_attendu_ailleurs_et_un_present_est_signale() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    sqlx::query(
        "INSERT INTO ent_asd_situation (id, etablissement_id, eleve_id, nature, debut_jour, fin_jour, source, canal)
         VALUES ($1,$2,$3,'sortie',$4,$4,'socle','systeme')",
    )
    .bind(Uuid::new_v4()).bind(etab).bind(lea).bind(jour).execute(ent.pool_verif()).await.unwrap();
    let appel = appel_ouvert(&ent, &cast, jour).await;
    let l = ligne(&ent, &cast, appel, lea).await;
    assert_eq!((l.etat.as_str(), l.lieu_code.as_deref(), l.libelle_court.as_deref()), ("attendu_ailleurs", Some("sortie"), Some("en sortie")));
    // « attendu ailleurs » est posé par le système, jamais à la main.
    assert!(matches!(
        ent.pointer_ligne_v2(etab, appel, lea, cast.prof.user_id, "prof", saisie("attendu_ailleurs"), "web").await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // NJ-24 (côté appel) : l'élève en sortie n'est plus invisible ; si le professeur le marque présent
    // (sortie mal tenue, désistement non annoncé), c'est tracé ET signalé à la vie scolaire.
    pointer(&ent, &cast, appel, lea, "present").await;
    let inc = ent.incoherences_appel_for_actor(etab, appel, cast.cpe.user_id, "cpe").await.unwrap();
    assert_eq!(inc, vec![IncoherenceV2 { eleve_id: lea, code: "present_alors_quen_sortie".into() }]);
    let h = ent.historique_ligne_for_actor(etab, appel, lea, cast.prof.user_id, "prof").await.unwrap();
    assert_eq!(h.iter().map(|x| x.etat_apres.as_str()).collect::<Vec<_>>(), vec!["present", "attendu_ailleurs", "present"]);
}

#[tokio::test]
async fn as2_retard_annonce_prevenu_jusqu_a_l_heure_dite() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    // Le professeur ne déclare rien (NJ-34) ; l'accueil / la vie scolaire, si.
    let a = || AnnoncerRetard { eleve_id: lea, jour: None, heure_arrivee_prevue: t(10, 0), declarant_nom: Some("Lily Potter".into()) };
    assert!(interdit(ent.annoncer_retard_for_actor(etab, cast.prof.user_id, "prof", a()).await));
    assert!(interdit(ent.annoncer_retard_for_actor(etab, cast.parent_lea.user_id, "parent", a()).await));
    ent.annoncer_retard_for_actor(etab, cast.cpe.user_id, "cpe", a()).await.expect("annoncer");
    let tot = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    let tard = creneau(&ent, &cast, jour, (10, 0), (11, 0)).await;
    let (a1, _) = ent.ouvrir_appel(etab, tot, jour, cast.prof.user_id, "prof").await.unwrap();
    let (a2, _) = ent.ouvrir_appel(etab, tard, jour, cast.prof.user_id, "prof").await.unwrap();
    let l1 = ligne(&ent, &cast, a1.id, lea).await;
    assert_eq!((l1.etat.as_str(), l1.libelle_court.as_deref()), ("absent", Some("prévenu")));
    let l2 = ligne(&ent, &cast, a2.id, lea).await;
    assert_eq!((l2.etat.as_str(), l2.pre_remplie), ("present", false), "présence attendue ensuite");
}

// ─── Pointage, modes, validation ────────────────────────────────────────────

#[tokio::test]
async fn as2_pointage_historise_retard_calcule_et_recapitulatif() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea, tom) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id, cast.eleve_tom_id);
    // Séance démarrée il y a longtemps : le retard sans minutes reçoit les minutes écoulées, bornées.
    let appel = appel_ouvert(&ent, &cast, jour).await;
    let l = pointer(&ent, &cast, appel, lea, "retard").await;
    assert!(l.minutes_retard.is_some_and(|m| (1..=600).contains(&m)), "{l:?}");
    // Précisions invalides refusées ; un état qui n'est pas à saisir aussi.
    for (etat, minutes, lieu) in [("present", Some(5), None), ("retard", Some(0), None), ("localise", None, None),
                                  ("absent", None, Some("cdi")), ("non_pointe", None, None), ("nimporte", None, None)] {
        let r = ent.pointer_ligne_v2(etab, appel, lea, cast.prof.user_id, "prof",
            SaisieLigne { etat: etat.into(), minutes, lieu_code: lieu.map(str::to_string) }, "web").await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{etat} {minutes:?} {lieu:?} : {r:?}");
    }
    assert!(matches!(ent.pointer_ligne_v2(etab, appel, lea, cast.prof.user_id, "prof", saisie("present"), "satellite").await, Err(EntDbError::SaisieInvalide(_))));
    // Un élève d'une autre classe n'est pas dans cet appel.
    let r = ent.pointer_ligne_v2(etab, appel, tom, cast.prof.user_id, "prof", saisie("absent"), "web").await;
    assert!(matches!(r, Err(EntDbError::AppelInvalide)), "{r:?}");
    pointer(&ent, &cast, appel, lea, "absent").await;
    // Historique : saisie, saisie (retard puis absent), version croissante, auteur et canal tracés.
    let h = ent.historique_ligne_for_actor(etab, appel, lea, cast.prof.user_id, "prof").await.unwrap();
    assert_eq!(h.iter().map(|x| x.version).collect::<Vec<_>>(), vec![2, 3]);
    assert!(h.iter().all(|x| x.auteur_id == Some(cast.prof.user_id) && x.canal == "web"));
    assert_eq!((h[0].etat_avant.as_deref(), h[0].etat_apres.as_str()), (Some("present"), "retard"));
    // Ajout seul, même pour le propriétaire de la base.
    assert!(sqlx::query("UPDATE ent_appel_ligne_histo SET motif_texte = 'x' WHERE etablissement_id = $1").bind(etab).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_appel_ligne_histo WHERE etablissement_id = $1").bind(etab).execute(ent.pool_verif()).await.is_err());
    let v = valider(&ent, &cast, appel).await;
    assert_eq!(v.etat, "valide");
    assert_eq!((v.recapitulatif.absents, v.recapitulatif.retards), (1, 0));
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'a_traiter'", etab, lea).await, 1);
    // Validé : plus de pointage (c'est une correction).
    assert!(matches!(ent.pointer_ligne_v2(etab, appel, lea, cast.prof.user_id, "prof", saisie("present"), "web").await, Err(EntDbError::AppelInvalide)));
    assert!(matches!(ent.valider_appel_v2(etab, appel, cast.prof.user_id, "prof", ValidationAppel { confirme_non_pointes: false, canal: None }).await, Err(EntDbError::AppelInvalide)));
}

#[tokio::test]
async fn as2_retard_valide_garde_ses_minutes() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let appel = appel_ouvert(&ent, &cast, jour).await;
    ent.pointer_ligne_v2(etab, appel, lea, cast.prof.user_id, "prof", SaisieLigne { etat: "retard".into(), minutes: Some(7), lieu_code: None }, "mobile").await.unwrap();
    valider(&ent, &cast, appel).await;
    let (m,): (Option<i32>,) = sqlx::query_as("SELECT minutes FROM ent_retard WHERE etablissement_id = $1 AND eleve_id = $2 AND source_appel_id = $3")
        .bind(etab).bind(lea).bind(appel).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(m, Some(7), "les minutes pointées vont au retard canonique");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2", etab, lea).await, 0, "un retard n'est pas une absence");
}

#[tokio::test]
async fn as2_activite_presents_seulement_exige_confirmation_des_non_pointes() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour) = (cast.etablissement_id, aujourdhui(&ent, &cast).await);
    let (dir, cpe, lea, tom) = (cast.direction.user_id, cast.cpe.user_id, cast.eleve_lea_id, cast.eleve_tom_id);
    ent.definir_activite_appelee_for_actor(etab, dir, "direction", "etude", true).await.unwrap();
    let act = |mode: &str, cle: &str| OuvrirActivite {
        type_activite: "etude".into(), jour, heure_debut: t(17, 0), heure_fin: t(18, 0),
        libelle: "Étude du soir".into(), cle: Some(cle.into()), eleve_ids: vec![lea, tom], mode_pointage: Some(mode.into()),
    };
    let a = ent.ouvrir_appel_activite(etab, cpe, "cpe", act("presents_seulement", "etude-1")).await.expect("ouvrir étude");
    assert!(a.lignes.iter().all(|l| l.etat == "non_pointe"));
    ent.pointer_ligne_v2(etab, a.id, lea, cpe, "cpe", saisie("present"), "web").await.unwrap();
    let r = ent.valider_appel_v2(etab, a.id, cpe, "cpe", ValidationAppel { confirme_non_pointes: false, canal: None }).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "confirmation obligatoire : {r:?}");
    assert_eq!(ent.appel_v2_lu(etab, a.id, cpe, "cpe").await.unwrap().etat, "en_cours", "rien n'a bougé");
    let v = ent.valider_appel_v2(etab, a.id, cpe, "cpe", ValidationAppel { confirme_non_pointes: true, canal: Some("mobile".into()) }).await.unwrap();
    assert_eq!((v.recapitulatif.presents, v.recapitulatif.absents), (1, 1));
    // L'absence à une activité porte l'horaire de l'activité (17 h – 18 h), pas une journée entière.
    let (d, f): (NaiveTime, NaiveTime) = sqlx::query_as("SELECT heure_debut, heure_fin FROM ent_absence_seance WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab).bind(tom).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!((d, f), (t(17, 0), t(18, 0)));
    // Mode « tous présents » : les non pointés sont présents, sans confirmation.
    let b = ent.ouvrir_appel_activite(etab, cpe, "cpe", act("tous_presents", "etude-2")).await.unwrap();
    let v = ent.valider_appel_v2(etab, b.id, cpe, "cpe", ValidationAppel { confirme_non_pointes: false, canal: None }).await.unwrap();
    assert_eq!(v.recapitulatif.presents, 2);
    // Idempotent par clé.
    let c = ent.ouvrir_appel_activite(etab, cpe, "cpe", act("tous_presents", "etude-2")).await.unwrap();
    assert_eq!(c.id, b.id);
}

#[tokio::test]
async fn as2_role_libre_aed_fait_l_appel_d_une_permanence_critere_as1() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, dir) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.direction.user_id);
    let (aed, base) = (cast.comptas[0].user_id, Role::Personnel.as_str());
    let (lea, tom) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let perm = || OuvrirActivite {
        type_activite: "permanence".into(), jour, heure_debut: t(14, 0), heure_fin: t(15, 0),
        libelle: "Permanence salle 12".into(), cle: None, eleve_ids: vec![lea, tom], mode_pointage: None,
    };
    // Sans rôle : refus (NJ-31 : aucun accès « par défaut »).
    assert!(interdit(ent.ouvrir_appel_activite(etab, aed, base, perm()).await));
    let droit = |action| Droit { action, portee: Portee::Etablissement, cibles: vec![] };
    let role = ent.creer_role_for_actor(etab, dir, "direction", RoleSaisie {
        nom: format!("AED {}", &Uuid::new_v4().simple().to_string()[..6]), description: None, modele: None,
        droits: vec![droit(Action::AppelFaire), droit(Action::AbsencesProposerQualification)],
    }).await.unwrap();
    ent.attribuer_role_for_actor(etab, dir, "direction", role.id, aed, None).await.unwrap();
    ent.definir_activite_appelee_for_actor(etab, dir, "direction", "permanence", true).await.unwrap();
    let a = ent.ouvrir_appel_activite(etab, aed, base, perm()).await.expect("AED ouvre la permanence");
    assert_eq!(a.type_activite, "permanence");
    ent.pointer_ligne_v2(etab, a.id, tom, aed, base, saisie("absent"), "mobile").await.unwrap();
    ent.valider_appel_v2(etab, a.id, aed, base, ValidationAppel { confirme_non_pointes: false, canal: Some("mobile".into()) }).await.unwrap();
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2", etab, tom).await, 1);
    // … mais il ne corrige pas un appel validé (pas `appel.corriger`) et ne lit pas l'appel d'un autre.
    let r = ent.corriger_ligne_for_actor(etab, a.id, tom, aed, base,
        CorrectionLigne { etat: "present".into(), minutes: None, lieu_code: None, motif_code: None, motif_texte: None, version_base: None }, "web").await;
    assert!(r.is_ok(), "l'auteur corrige le jour même : {r:?}");
    let autre = ent.ouvrir_appel_activite(etab, cast.cpe.user_id, "cpe", OuvrirActivite { cle: Some("autre".into()), ..perm() }).await.unwrap();
    assert!(interdit(ent.appel_v2_lu(etab, autre.id, aed, base).await), "l'appel d'un autre n'est pas le sien");
    // Activité non appelée : refus métier.
    ent.definir_activite_appelee_for_actor(etab, dir, "direction", "permanence", false).await.unwrap();
    assert!(matches!(ent.ouvrir_appel_activite(etab, aed, base, OuvrirActivite { cle: Some("x".into()), ..perm() }).await, Err(EntDbError::Refus(..))));
}

// ─── Corrections (AS-131), démenti (AS-136) ─────────────────────────────────

fn correction(etat: &str, motif: Option<&str>) -> CorrectionLigne {
    CorrectionLigne { etat: etat.into(), minutes: None, lieu_code: None, motif_code: None, motif_texte: motif.map(str::to_string), version_base: None }
}

async fn appel_valide_lea_absent(ent: &PgEnt, cast: &LaboCast, jour: NaiveDate) -> Uuid {
    let appel = appel_ouvert(ent, cast, jour).await;
    pointer(ent, cast, appel, cast.eleve_lea_id, "absent").await;
    valider(ent, cast, appel).await;
    appel
}

#[tokio::test]
async fn as2_correction_de_l_intervenant_le_jour_meme_retire_l_absence_et_dement() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let appel = appel_valide_lea_absent(&ent, &cast, jour).await;
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence_seance WHERE etablissement_id = $1 AND appel_id = $2", etab, appel).await, 1);
    let avant_ev = n(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'assiduite.absence_dementie'", etab, appel).await;
    let l = ent.corriger_ligne_for_actor(etab, appel, lea, cast.prof.user_id, "prof", correction("present", Some("erreur de saisie")), "web").await.unwrap();
    assert_eq!(l.etat, "present");
    // Effets recalculés : plus de séance absente, l'absence du jour est annulée, démenti publié.
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence_seance WHERE etablissement_id = $1 AND appel_id = $2", etab, appel).await, 0);
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'annulee_present'", etab, lea).await, 1);
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'assiduite.absence_dementie'", etab, appel).await, avant_ev + 1, "démenti à la famille (AS-136)");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND eleve_id = $2 AND type = 'appel.ligne_corrigee'", etab, lea).await, 1);
    // Historique : jamais réécrit en silence (R-AS8).
    let h = ent.historique_ligne_for_actor(etab, appel, lea, cast.prof.user_id, "prof").await.unwrap();
    let derniere = h.last().unwrap();
    assert_eq!((derniere.etat_avant.as_deref(), derniere.etat_apres.as_str(), derniere.motif_code.as_deref()), (Some("absent"), "present", Some("erreur_saisie")));
    assert_eq!(derniere.motif_texte.as_deref(), Some("erreur de saisie"));
    // Et de nouveau absent (oubli) : l'absence et l'avis sont recréés.
    ent.corriger_ligne_for_actor(etab, appel, lea, cast.prof.user_id, "prof", correction("absent", None), "web").await.unwrap();
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence_seance WHERE etablissement_id = $1 AND appel_id = $2", etab, appel).await, 1);
}

#[tokio::test]
async fn as2_la_correction_a_des_gardes_intervenant_jour_motif_version() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let appel = appel_valide_lea_absent(&ent, &cast, jour).await;
    // Le professeur d'une autre classe ne corrige pas l'appel de son collègue.
    assert!(interdit(ent.corriger_ligne_for_actor(etab, appel, lea, cast.profs[1].user_id, "prof", correction("present", None), "web").await));
    // Famille, élève, comptabilité : aucun accès (NJ-31).
    for (u, r) in [(cast.parent_lea.user_id, "parent"), (cast.compta.user_id, "compta")] {
        assert!(interdit(ent.corriger_ligne_for_actor(etab, appel, lea, u, r, correction("present", None), "web").await), "{r}");
    }
    // Hors du jour même, le professeur ne corrige plus : il demande à la vie scolaire.
    sqlx::query("UPDATE ent_appel SET jour = jour - 1 WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(appel).execute(ent.pool_verif()).await.unwrap();
    assert!(interdit(ent.corriger_ligne_for_actor(etab, appel, lea, cast.prof.user_id, "prof", correction("present", Some("trop tard")), "web").await));
    // La vie scolaire corrige à tout moment, MAIS avec un motif.
    let r = ent.corriger_ligne_for_actor(etab, appel, lea, cast.cpe.user_id, "cpe", correction("present", None), "web").await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "motif obligatoire : {r:?}");
    // Version périmée : conflit, rien n'est écrit.
    let mut c = correction("present", Some("billet d'entrée"));
    c.version_base = Some(1);
    let r = ent.corriger_ligne_for_actor(etab, appel, lea, cast.cpe.user_id, "cpe", c, "web").await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    let v = ligne(&ent, &cast, appel, lea).await.version;
    let mut c = correction("retard", Some("arrivé avec un billet"));
    c.minutes = Some(12);
    c.version_base = Some(v);
    let l = ent.corriger_ligne_for_actor(etab, appel, lea, cast.cpe.user_id, "cpe", c, "web").await.unwrap();
    assert_eq!((l.etat.as_str(), l.minutes_retard), ("retard", Some(12)));
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_retard WHERE etablissement_id = $1 AND source_appel_id = $2", etab, appel).await, 1, "devenu retard : retard canonique créé");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence_seance WHERE etablissement_id = $1 AND appel_id = $2", etab, appel).await, 0, "plus absent");
    // Motif inconnu et motif trop long refusés.
    let mut c = correction("present", Some("x"));
    c.motif_code = Some("nimporte".into());
    assert!(matches!(ent.corriger_ligne_for_actor(etab, appel, lea, cast.cpe.user_id, "cpe", c, "web").await, Err(EntDbError::SaisieInvalide(_))));
    let long = "m".repeat(201);
    assert!(matches!(ent.corriger_ligne_for_actor(etab, appel, lea, cast.cpe.user_id, "cpe", correction("present", Some(&long)), "web").await, Err(EntDbError::SaisieInvalide(_))));
    // Retard → présent : le retard canonique disparaît aussi.
    ent.corriger_ligne_for_actor(etab, appel, lea, cast.cpe.user_id, "cpe", correction("present", Some("erreur")), "web").await.unwrap();
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_retard WHERE etablissement_id = $1 AND source_appel_id = $2", etab, appel).await, 0);
}

// ─── Neutralisation d'une séance annulée après l'appel (DA-09, AS-135) ──────

#[tokio::test]
async fn as2_seance_annulee_apres_l_appel_neutralise_les_absences_et_dement() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea, hermione) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id, cast.eleves[2].id);
    let c = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    let (a, _) = ent.ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof").await.unwrap();
    pointer(&ent, &cast, a.id, lea, "absent").await;
    pointer(&ent, &cast, a.id, hermione, "retard").await;
    valider(&ent, &cast, a.id).await;
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence_seance WHERE etablissement_id = $1 AND appel_id = $2", etab, a.id).await, 1);
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_retard WHERE etablissement_id = $1 AND source_appel_id = $2", etab, a.id).await, 1);
    // La direction annule la séance APRÈS l'appel (professeur absent, fermeture…).
    ent.poser_edt_exception_for_actor("direction", PoserEdtException {
        etablissement_id: etab, creneau_id: c, jour, kind: EdtExceptionKind::Annulation, remplacant_user_id: None,
        nouvelle_salle: None, motif: "professeur absent".into(), actor_user_id: cast.direction.user_id,
    }).await.expect("annulation");
    let v = ent.appel_v2_lu(etab, a.id, cast.cpe.user_id, "cpe").await.unwrap();
    assert_eq!(v.etat, "neutralise");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence_seance WHERE etablissement_id = $1 AND appel_id = $2", etab, a.id).await, 0, "les absences cessent de compter");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_retard WHERE etablissement_id = $1 AND source_appel_id = $2", etab, a.id).await, 0);
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'a_traiter'", etab, lea).await, 0);
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'assiduite.absence_dementie'", etab, a.id).await, 1, "un démenti pour l'absent");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'appel.neutralise'", etab, a.id).await, 1);
    // Les lignes gardent leur historique (R-AS8) ; plus aucune correction ni pointage possible.
    assert!(ent.historique_ligne_for_actor(etab, a.id, lea, cast.cpe.user_id, "cpe").await.unwrap().iter().any(|h| h.motif_code.as_deref() == Some("neutralisation")));
    assert!(matches!(ent.corriger_ligne_for_actor(etab, a.id, lea, cast.cpe.user_id, "cpe", correction("absent", Some("x")), "web").await, Err(EntDbError::AppelInvalide)));
    // Idempotent : une seconde annulation ne change rien.
    assert!(!ent.neutraliser_appel_seance(etab, c, jour, "annulation_seance", None).await.unwrap());
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'assiduite.absence_dementie'", etab, a.id).await, 1);
    // Séance sans appel : rien à neutraliser ; cause inconnue refusée.
    assert!(!ent.neutraliser_appel_seance(etab, Uuid::new_v4(), jour, "annulation_seance", None).await.unwrap());
    assert!(matches!(ent.neutraliser_appel_seance(etab, c, jour, "caprice", None).await, Err(EntDbError::SaisieInvalide(_))));
    // La neutralisation d'un appel jamais validé ne crée aucun démenti.
    let c2 = creneau(&ent, &cast, jour, (10, 0), (11, 0)).await;
    let (a2, _) = ent.ouvrir_appel(etab, c2, jour, cast.prof.user_id, "prof").await.unwrap();
    assert!(ent.neutraliser_appel_seance(etab, c2, jour, "fermeture", None).await.unwrap());
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'assiduite.absence_dementie'", etab, a2.id).await, 0);
}

// ─── Délégation (AS-105) ────────────────────────────────────────────────────

#[tokio::test]
async fn as2_delegation_datee_a_un_collegue() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let appel = appel_ouvert(&ent, &cast, jour).await;
    let (rogue, mcg) = (cast.profs[0].user_id, cast.profs[1].user_id);
    // Avant : McGonagall n'a aucun accès à l'appel de Rogue.
    assert!(interdit(ent.appel_v2_lu(etab, appel, mcg, "prof").await));
    // Seul l'intervenant délègue ; la famille et la compta n'ont pas l'action.
    assert!(interdit(ent.deleguer_appel_for_actor(etab, appel, mcg, "prof", mcg, None).await));
    assert!(interdit(ent.deleguer_appel_for_actor(etab, appel, cast.parent_lea.user_id, "parent", mcg, None).await));
    // On ne délègue pas à soi-même, ni à une personne d'un autre établissement.
    assert!(matches!(ent.deleguer_appel_for_actor(etab, appel, rogue, "prof", rogue, None).await, Err(EntDbError::AppelInvalide)));
    assert!(ent.deleguer_appel_for_actor(etab, appel, rogue, "prof", Uuid::new_v4(), None).await.is_err());
    ent.deleguer_appel_for_actor(etab, appel, rogue, "prof", mcg, Some("sortie au CDI")).await.unwrap();
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_appel_delegation WHERE etablissement_id = $1 AND appel_id = $2 AND revoquee_le IS NULL", etab, appel).await, 1);
    // Le délégué lit, pointe et valide cet appel (et seulement celui-là).
    assert_eq!(ent.appel_v2_lu(etab, appel, mcg, "prof").await.unwrap().delegue_a, Some(mcg));
    ent.pointer_ligne_v2(etab, appel, lea, mcg, "prof", saisie("absent"), "web").await.unwrap();
    let h = ent.historique_ligne_for_actor(etab, appel, lea, mcg, "prof").await.unwrap();
    assert_eq!(h.last().unwrap().auteur_id, Some(mcg), "l'auteur du geste est tracé");
    let autre = {
        let c2 = creneau(&ent, &cast, jour, (10, 0), (11, 0)).await;
        ent.ouvrir_appel(etab, c2, jour, rogue, "prof").await.unwrap().0.id
    };
    assert!(interdit(ent.appel_v2_lu(etab, autre, mcg, "prof").await), "la délégation vaut pour une séance, pas pour les autres");
    // Une nouvelle délégation remplace la précédente ; la révocation rend l'accès.
    let troisieme = cast.profs[2].user_id;
    ent.deleguer_appel_for_actor(etab, appel, rogue, "prof", troisieme, None).await.unwrap();
    assert!(interdit(ent.appel_v2_lu(etab, appel, mcg, "prof").await));
    assert!(ent.appel_v2_lu(etab, appel, troisieme, "prof").await.is_ok());
    assert!(interdit(ent.revoquer_delegation_for_actor(etab, appel, troisieme, "prof").await), "le délégué ne révoque pas");
    ent.revoquer_delegation_for_actor(etab, appel, rogue, "prof").await.unwrap();
    assert!(interdit(ent.appel_v2_lu(etab, appel, troisieme, "prof").await));
    // Visible de la vie scolaire (supervision).
    assert!(ent.appel_v2_lu(etab, appel, cast.cpe.user_id, "cpe").await.is_ok());
}

// ─── Exclusion ponctuelle de cours (R-AS37, AS-208) ─────────────────────────

#[tokio::test]
async fn as2_exclusion_de_cours_suivie_jusqu_a_l_arrivee_a_la_vie_scolaire() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let appel = appel_ouvert(&ent, &cast, jour).await;
    let l = ent.exclure_du_cours_for_actor(etab, appel, lea, cast.prof.user_id, "prof", "web").await.unwrap();
    assert_eq!((l.etat.as_str(), l.lieu_code.as_deref(), l.arrivee_vs_attendue), ("localise", Some("vie_scolaire"), true));
    let inc = ent.incoherences_appel_for_actor(etab, appel, cast.cpe.user_id, "cpe").await.unwrap();
    assert_eq!(inc, vec![IncoherenceV2 { eleve_id: lea, code: "exclusion_sans_arrivee".into() }]);
    // Jamais « absent » : la validation ne crée aucune absence.
    valider(&ent, &cast, appel).await;
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2", etab, lea).await, 0);
    // L'arrivée est confirmée par la vie scolaire (pas par le professeur).
    assert!(interdit(ent.confirmer_arrivee_vs_for_actor(etab, appel, lea, cast.prof.user_id, "prof").await));
    ent.confirmer_arrivee_vs_for_actor(etab, appel, lea, cast.cpe.user_id, "cpe").await.unwrap();
    assert!(ent.incoherences_appel_for_actor(etab, appel, cast.cpe.user_id, "cpe").await.unwrap().is_empty());
    assert!(matches!(ent.confirmer_arrivee_vs_for_actor(etab, appel, lea, cast.cpe.user_id, "cpe").await, Err(EntDbError::Domain(_))), "déjà confirmée");
    // Un professeur étranger à l'appel n'exclut personne de ce cours.
    assert!(interdit(ent.exclure_du_cours_for_actor(etab, appel, lea, cast.profs[1].user_id, "prof", "web").await));
}

// ─── Séparation (NJ-30, NJ-31, NJ-34, NJ-35) ────────────────────────────────

#[tokio::test]
async fn as2_matrice_de_separation_aucun_acces_hors_des_droits() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let appel = appel_ouvert(&ent, &cast, jour).await;
    declarer(&ent, &cast, jour, None, None).await;
    let c = creneau(&ent, &cast, jour, (11, 0), (12, 0)).await;
    // NJ-31 : aucun rôle sans `appel.faire` (famille, élève, comptabilité, psychologue…) n'ouvre,
    // ne lit, ne pointe, ne valide ni ne corrige un appel — sur chaque route du service.
    let intrus: Vec<(Uuid, &str)> = vec![
        (cast.parent_lea.user_id, "parent"), (cast.parent_tom.user_id, "parent"), (cast.compta.user_id, "compta"),
        (cast.eleve_lea_id, "eleve"), (cast.psys[0].user_id, "psy"), (cast.comptas[0].user_id, "personnel"),
    ];
    for (u, r) in &intrus {
        assert!(interdit(ent.ouvrir_appel(etab, c, jour, *u, r).await), "ouvrir {r}");
        assert!(interdit(ent.appel_v2_lu(etab, appel, *u, r).await), "lire {r}");
        assert!(interdit(ent.pointer_ligne_v2(etab, appel, lea, *u, r, saisie("absent"), "web").await), "pointer {r}");
        assert!(interdit(ent.valider_appel_v2(etab, appel, *u, r, ValidationAppel { confirme_non_pointes: false, canal: None }).await), "valider {r}");
        assert!(interdit(ent.corriger_ligne_for_actor(etab, appel, lea, *u, r, correction("present", Some("x")), "web").await), "corriger {r}");
        assert!(interdit(ent.historique_ligne_for_actor(etab, appel, lea, *u, r).await), "historique {r}");
        assert!(interdit(ent.exclure_du_cours_for_actor(etab, appel, lea, *u, r, "web").await), "exclure {r}");
        assert!(interdit(ent.incoherences_appel_for_actor(etab, appel, *u, r).await), "incohérences {r}");
    }
    // NJ-30 / NJ-34 : le professeur ne lit ni les situations de l'établissement ni les motifs, et ne
    // déclare ni ne localise (même pour son propre élève).
    assert!(interdit(ent.situations_du_jour_for_actor(etab, cast.prof.user_id, "prof", jour).await));
    assert!(interdit(ent.poser_localisation_for_actor(etab, cast.prof.user_id, "prof",
        PoserLocalisation { eleve_id: lea, lieu_code: "cdi".into(), jour: None, heure_debut: None, heure_fin: None }).await));
    assert!(interdit(ent.motifs_for_actor(etab, cast.prof.user_id, "prof").await));
    assert!(interdit(ent.lever_situation_for_actor(etab, cast.prof.user_id, "prof", Uuid::new_v4(), false).await) ||
            matches!(ent.lever_situation_for_actor(etab, cast.prof.user_id, "prof", Uuid::new_v4(), false).await, Err(EntDbError::Domain(_))));
    // La vie scolaire, elle, voit les situations (sans le texte libre dans la liste).
    let sit = ent.situations_du_jour_for_actor(etab, cast.cpe.user_id, "cpe", jour).await.unwrap();
    assert!(sit.iter().any(|s| s.eleve_id == lea && s.nature == "declaration"));
    assert!(!serde_json::to_string(&sit).unwrap().contains("gastro"), "NJ-35 : le texte libre n'est jamais dans la liste");
    // Autre établissement : référence inconnue (aucune fuite).
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    assert!(ent.appel_v2_lu(autre.etablissement_id, appel, autre.cpe.user_id, "cpe").await.is_err(), "appel d'un autre établissement");
    assert!(ent.historique_ligne_for_actor(autre.etablissement_id, appel, lea, autre.cpe.user_id, "cpe").await.is_err());
    // Et la ligne de Lea n'a pas bougé pendant ces tentatives.
    assert_eq!(ligne(&ent, &cast, appel, lea).await.etat, "absent");
    let _ = DeclarerRetard { etablissement_id: etab, eleve_id: lea, jour, minutes: None, motif: None, source_appel_id: None, actor_user_id: None };
}

// ─── Compatibilité : l'ancien parcours (ouvrir, marquer, clore) fonctionne et est historisé ─────

#[tokio::test]
async fn as2_ancien_parcours_compatible_et_historise() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, jour, lea) = (cast.etablissement_id, aujourdhui(&ent, &cast).await, cast.eleve_lea_id);
    let c = creneau(&ent, &cast, jour, (8, 0), (9, 0)).await;
    let (a, lignes) = ent.ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof").await.unwrap();
    assert!(lignes.iter().all(|l| l.etat == LigneEtat::Present && l.version == 1));
    ent.marquer_appel_ligne(etab, a.id, cast.prof.user_id, "prof", lea, LigneEtat::Absent).await.unwrap();
    let clos = ent.clore_appel(etab, a.id, cast.prof.user_id, "prof", None).await.unwrap();
    assert_eq!(clos.statut, AppelStatut::Clos);
    let (statut, etat, canal, par): (String, String, Option<String>, Option<Uuid>) = sqlx::query_as(
        "SELECT statut, etat, canal_validation, valide_par FROM ent_appel WHERE etablissement_id = $1 AND id = $2")
        .bind(etab).bind(a.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!((statut.as_str(), etat.as_str(), canal.as_deref(), par), ("clos", "valide", Some("web"), Some(cast.prof.user_id)));
    let h = ent.historique_ligne_for_actor(etab, a.id, lea, cast.prof.user_id, "prof").await.unwrap();
    assert_eq!(h.len(), 1, "le marquage de l'ancienne route est historisé aussi");
}
