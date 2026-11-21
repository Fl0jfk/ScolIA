//! Socle 2 (l'élève), lot E6 : fusion de deux fiches (ELV-23), défusion à l'identique (ELV-24)
//! et registre `TABLES_ELEVE` avec son test de garde. Scénarios contre la base locale de test
//! (feature `labo`) ; sans `ENT_DATABASE_URL`, ils sont ignorés. Chaque scénario travaille dans
//! son propre établissement (jetable).

use super::*;
use crate::eleve_fusion::{
    AvertissementFusion, DemandeFusion, ResultatDefusion, ResultatFusion,
};
use chrono::{Duration, NaiveDate};
use domain_eleve::{
    CreateEleve, EtatInscription, StatutApprenant, StrategieFusion, TABLES_ELEVE,
};
use ent_auth::Role;
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::scolarite::InscriptionAnnuelle;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| { }).await;
    Some((ent, cast))
}

const DIR: &str = "direction";

fn jour(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn alea(n: usize) -> String {
    Uuid::new_v4().simple().to_string()[..n].to_uppercase()
}

#[allow(clippy::too_many_arguments)]
async fn eleve(
    ent: &PgEnt,
    etab: Uuid,
    nom: &str,
    prenom: &str,
    ine: Option<&str>,
    badge: Option<&str>,
    entree: Option<NaiveDate>,
    sortie: Option<NaiveDate>,
) -> domain_eleve::Eleve {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: prenom.into(),
        ine: ine.map(String::from),
        sexe: Some("M".into()),
        date_naissance: Some(jour(2012, 3, 4)),
        numero_badge: badge.map(String::from),
        redoublant: false,
        classe: Some("6A".into()),
        regime: None,
        mef: None,
        site: None,
        date_entree: entree,
        date_sortie: sortie,
        motif_sortie: sortie.map(|_| "demenagement".into()),
        actor_user_id: None,
    })
    .await
    .unwrap()
}

/// Données d'autres domaines, posées en SQL : absences, retards, carnet, document.
async fn donnees(ent: &PgEnt, etab: Uuid, eleve: Uuid, absences: i64, retards: i64, carnet: i64, documents: i64) {
    for i in 0..absences {
        sqlx::query("INSERT INTO ent_absence (id, etablissement_id, eleve_id, debut, fin, motif) VALUES (gen_random_uuid(), $1, $2, $3, $3, 'maladie')")
            .bind(etab)
            .bind(eleve)
            .bind(jour(2025, 10, 1) + Duration::days(i))
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
    for i in 0..retards {
        sqlx::query("INSERT INTO ent_retard (id, etablissement_id, eleve_id, jour) VALUES (gen_random_uuid(), $1, $2, $3)")
            .bind(etab)
            .bind(eleve)
            .bind(jour(2025, 11, 1) + Duration::days(i))
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
    for i in 0..carnet {
        sqlx::query("INSERT INTO ent_carnet_entree (id, etablissement_id, eleve_id, date_entree, categorie, titre, corps) VALUES (gen_random_uuid(), $1, $2, $3, 'information', 'Mot', 'Texte')")
            .bind(etab)
            .bind(eleve)
            .bind(jour(2025, 9, 1) + Duration::days(i))
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
    for i in 0..documents {
        sqlx::query("INSERT INTO ent_eleve_document (id, etablissement_id, eleve_id, tiroir, titre) VALUES (gen_random_uuid(), $1, $2, 'administratif', $3)")
            .bind(etab)
            .bind(eleve)
            .bind(format!("Pièce {i}"))
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
}

/// Image canonique de tout ce que portent deux élèves : chaque table du registre et la ligne
/// d'`ent_eleve`. Sert à vérifier « à l'identique ».
async fn image(ent: &PgEnt, etab: Uuid, ids: &[Uuid]) -> BTreeMap<String, Value> {
    let mut m = BTreeMap::new();
    for e in ids {
        for t in TABLES_ELEVE.iter().filter(|t| !matches!(t.table, "ent_journal_modif" | "ent_metier_event")) {
            let v: Value = sqlx::query_scalar(&format!(
                "SELECT coalesce(jsonb_agg(to_jsonb(t.*) ORDER BY to_jsonb(t.*)::text), '[]'::jsonb)
                   FROM {} t WHERE t.etablissement_id = $1 AND t.eleve_id = $2",
                t.table
            ))
            .bind(etab)
            .bind(e)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
            m.insert(format!("{e}/{}", t.table), v);
        }
        let v: Value = sqlx::query_scalar("SELECT to_jsonb(e) FROM ent_eleve e WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(e)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
        m.insert(format!("{e}/ent_eleve"), v);
    }
    m
}

fn assert_identique(avant: &BTreeMap<String, Value>, apres: &BTreeMap<String, Value>) {
    let ecarts: Vec<String> = avant
        .iter()
        .filter(|(k, v)| apres.get(*k) != Some(v))
        .map(|(k, v)| format!("{k}\n   avant {}\n   après {}", abrege(v), abrege(&apres[k])))
        .collect();
    assert!(ecarts.is_empty(), "la défusion n'est pas à l'identique :\n{}", ecarts.join("\n"));
    assert_eq!(avant.len(), apres.len());
}

fn abrege(v: &Value) -> String {
    let s = v.to_string();
    if s.len() > 400 {
        format!("{}…", &s[..400])
    } else {
        s
    }
}

async fn nb(ent: &PgEnt, table: &str, etab: Uuid, eleve: Uuid) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE etablissement_id = $1 AND eleve_id = $2"))
        .bind(etab)
        .bind(eleve)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

/// Fusion avec acquittement de tous les conflits de l'aperçu (deux fiches normales ont chacune
/// une période de classe ouverte : conflit d'unicité attendu, la ligne du conservé l'emporte).
async fn fuser(ent: &PgEnt, etab: Uuid, absorbe: Uuid, conserve: Uuid, acteur: Uuid) -> crate::eleve_fusion::FusionEleves {
    let ap = ent.apercu_fusion_pour_acteur(DIR, etab, acteur, absorbe, conserve).await.unwrap();
    assert!(ap.conflits.iter().all(|c| c.table == "ent_eleve_periode"), "{:?}", ap.conflits);
    let mut d = demande(etab, absorbe, conserve, acteur);
    d.conflits_acquittes = ap.conflits;
    match ent.fusionner_eleves_pour_acteur(DIR, d).await.unwrap() {
        ResultatFusion::Faite(f) => f,
        ResultatFusion::ConflitsAResoudre(c) => panic!("conflits inattendus : {c:?}"),
    }
}

fn demande(etab: Uuid, absorbe: Uuid, conserve: Uuid, acteur: Uuid) -> DemandeFusion {
    DemandeFusion {
        etablissement_id: etab,
        absorbe_id: absorbe,
        conserve_id: conserve,
        motif_code: "doublon_saisie".into(),
        motif: None,
        conflits_acquittes: Vec::new(),
        acteur,
    }
}

async fn inscription(ent: &PgEnt, etab: Uuid, eleve: Uuid, date_annee: &str, acteur: Uuid) {
    let (annee,): (Uuid,) = sqlx::query_as("SELECT ent_assurer_annee($1, $2::date)")
        .bind(etab)
        .bind(date_annee)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    ent.enregistrer_inscription_annuelle(
        "administratif",
        InscriptionAnnuelle {
            etablissement_id: etab,
            eleve_id: eleve,
            annee_id: annee,
            niveau_id: None,
            mef_id: None,
            statut_apprenant: StatutApprenant::Scolaire,
            etat: EtatInscription::Confirmee,
            redoublement: Some(false),
            source: "saisie".into(),
            acteur: Some(acteur),
        },
    )
    .await
    .unwrap();
}

/// Deux fiches sans conflit : B (absorbé) a un ancien séjour clos, un badge et une année
/// d'inscription passée ; A (conservé) est scolarisé aujourd'hui avec un INE.
async fn deux_fiches(ent: &PgEnt, cast: &LaboCast, etab: Uuid) -> (domain_eleve::Eleve, domain_eleve::Eleve) {
    let acteur = cast.direction.user_id;
    let sfx = alea(4);
    let a = eleve(ent, etab, &format!("Fusion{sfx}"), "Jean", Some(&format!("AAAA{}", alea(7))), None,
        Some(jour(2025, 9, 1)), None).await;
    let b = eleve(ent, etab, &format!("Fusionn{sfx}"), "Jean", None, Some(&format!("BADGE-{}", alea(6))),
        Some(jour(2022, 9, 1)), Some(jour(2023, 7, 1))).await;
    donnees(ent, etab, a.id, 2, 1, 0, 1).await;
    donnees(ent, etab, b.id, 3, 2, 1, 1).await;
    inscription(ent, etab, a.id, &chrono::Utc::now().format("%Y-%m-%d").to_string(), acteur).await;
    inscription(ent, etab, b.id, "2022-10-01", acteur).await;
    (a, b)
}

/// Critère de fin du lot : fusion puis défusion **à l'identique**, sur des fiches qui portent
/// des données de plusieurs domaines, une scolarité, des identifiants et des inscriptions.
#[tokio::test]
async fn fusion_puis_defusion_a_l_identique() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (Uuid::new_v4(), cast.direction.user_id);
    let (a, b) = deux_fiches(&ent, &cast, etab).await;
    let avant = image(&ent, etab, &[a.id, b.id]).await;
    let a_badge: Option<String> = sqlx::query_scalar("SELECT numero_badge FROM ent_eleve WHERE id = $1").bind(a.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(a_badge.is_none());

    // Aperçu : rien n'est écrit, tout est annoncé.
    let ap = ent.apercu_fusion_pour_acteur(DIR, etab, dir, b.id, a.id).await.unwrap();
    assert!(ap.possible && ap.conflits.iter().all(|c| c.table == "ent_eleve_periode"), "{:?}", ap.conflits);
    let ligne = |dom: &str| ap.lignes.iter().filter(|l| l.domaine == dom).map(|l| l.a_deplacer).sum::<usize>();
    assert_eq!(ligne("Absences et retards"), 3 + 2, "3 absences et 2 retards de B");
    assert_eq!(ligne("Carnet de liaison"), 1);
    assert!(ap.lignes.iter().any(|l| l.table.as_deref() == Some("ent_eleve_identite") && l.restent >= 1),
        "l'historique d'identité reste sur l'absorbé");
    assert!(ap.avertissements.is_empty(), "{:?}", ap.avertissements);
    assert!(ap.differences.contains(&"nom"));
    assert_identique(&avant, &image(&ent, etab, &[a.id, b.id]).await);
    assert!(sqlx::query_scalar::<_, Option<Uuid>>("SELECT fusionne_dans FROM ent_eleve WHERE id = $1")
        .bind(b.id).fetch_one(ent.pool_verif()).await.unwrap().is_none(), "l'aperçu n'écrit rien");

    // Fusion.
    let f = fuser(&ent, etab, b.id, a.id, dir).await;
    assert_eq!((f.absorbe_id, f.conserve_id), (b.id, a.id));
    assert!(f.annulable_jusqu_au > chrono::Utc::now() + Duration::days(29));
    // Les données de B sont chez A ; B est marqué absorbé et ne porte plus que ses historiques.
    assert_eq!(nb(&ent, "ent_absence", etab, a.id).await, 2 + 3);
    assert_eq!(nb(&ent, "ent_retard", etab, a.id).await, 1 + 2);
    assert_eq!(nb(&ent, "ent_carnet_entree", etab, a.id).await, 1);
    assert_eq!(nb(&ent, "ent_eleve_document", etab, a.id).await, 2);
    assert_eq!(nb(&ent, "ent_absence", etab, b.id).await, 0);
    assert_eq!(nb(&ent, "ent_eleve_scolarisation", etab, a.id).await, 2, "ancien séjour de B + séjour de A");
    assert_eq!(nb(&ent, "ent_eleve_inscription", etab, a.id).await, 2);
    assert!(nb(&ent, "ent_eleve_identite", etab, b.id).await >= 1, "l'historique d'état civil reste sur B");
    let absorbe: Option<Uuid> = sqlx::query_scalar("SELECT fusionne_dans FROM ent_eleve WHERE id = $1").bind(b.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(absorbe, Some(a.id));
    // Le badge de B est passé à A (projection recalculée) ; l'INE de A est intact.
    let (badge, ine): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT numero_badge, ine FROM ent_eleve WHERE id = $1").bind(a.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(badge.is_some_and(|x| x.starts_with("BADGE-")));
    assert!(ine.is_some_and(|x| x.starts_with("AAAA")));
    assert_eq!(ent.eleve_effectif(etab, b.id).await.unwrap(), a.id, "toute lecture redirige");
    assert_eq!(ent.eleve_effectif(etab, a.id).await.unwrap(), a.id);
    // L'absorbé ne sort plus de la recherche ; le conservé, si.
    let trouves = ent.search_eleves_pour_acteur(etab, dir, DIR, "fusionn", 50).await.unwrap();
    assert!(trouves.iter().all(|v| v.eleve.id != b.id), "l'absorbé est sorti de la recherche");
    assert!(trouves.iter().any(|v| v.eleve.id == a.id));
    // Événement et journal : jamais une valeur d'identité.
    let (payload, journal): (Value, Value) = sqlx::query_as(
        "SELECT (SELECT payload FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'eleve.fusionne'),
                (SELECT payload FROM ent_journal_modif WHERE etablissement_id = $1 AND action = 'eleve_fusionne' LIMIT 1)",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    for p in [payload.to_string(), journal.to_string()] {
        assert!(!p.to_lowercase().contains("fusion") || p.contains("fusion_id"), "{p}");
        assert!(!p.contains("Jean") && !p.contains("BADGE-") && !p.contains("AAAA"), "{p}");
    }

    // Une fusion déjà faite ne se refait pas ; on ne fusionne pas une fiche avec elle-même.
    assert!(matches!(
        ent.fusionner_eleves_pour_acteur(DIR, demande(etab, b.id, a.id, dir)).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(ent.fusionner_eleves_pour_acteur(DIR, demande(etab, a.id, a.id, dir)).await.is_err());

    // Défusion : tout revient, à l'identique.
    let ResultatDefusion::Faite(d) = ent.defusionner_eleves_pour_acteur(DIR, etab, dir, f.id).await.unwrap() else {
        panic!("aucune ligne modifiée attendue")
    };
    assert!(d.annule_le.is_some());
    assert_identique(&avant, &image(&ent, etab, &[a.id, b.id]).await);
    assert_eq!(ent.eleve_effectif(etab, b.id).await.unwrap(), b.id);
    // Annulée : on ne l'annule pas deux fois ; on peut refusionner.
    assert!(matches!(
        ent.defusionner_eleves_pour_acteur(DIR, etab, dir, f.id).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    let f2 = fuser(&ent, etab, b.id, a.id, dir).await;
    assert_ne!(f2.id, f.id);
    let fusions = ent.fusions_pour_acteur(DIR, etab, dir, 10).await.unwrap();
    assert_eq!(fusions.len(), 2);
    assert!(fusions[0].annule_le.is_none() && fusions[1].annule_le.is_some());
}

/// Conflits : la ligne du conservé l'emporte, celle de l'absorbé reste sur l'absorbé ; chaque
/// conflit doit être acquitté ; la défusion remet tout à l'identique.
#[tokio::test]
async fn conflits_a_acquitter_puis_defusion_a_l_identique() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (Uuid::new_v4(), cast.direction.user_id);
    let ine = format!("CONF{}", alea(7));
    let ine_b = format!("CONG{}", alea(7));
    // Deux INE actifs différents, deux séjours ouverts, la même année d'inscription : trois
    // familles de conflits (un élève n'a qu'un INE actif, un séjour ouvert, une inscription par année).
    let a = eleve(&ent, etab, &format!("Conflit{}", alea(4)), "Lina", Some(&ine), None, Some(jour(2025, 9, 1)), None).await;
    let b = eleve(&ent, etab, &format!("Conflit{}", alea(4)), "Lina", Some(&ine_b), None, Some(jour(2025, 9, 1)), None).await;
    donnees(&ent, etab, a.id, 1, 0, 0, 0).await;
    donnees(&ent, etab, b.id, 2, 0, 0, 0).await;
    let aujourdhui = chrono::Utc::now().format("%Y-%m-%d").to_string();
    inscription(&ent, etab, a.id, &aujourdhui, dir).await;
    inscription(&ent, etab, b.id, &aujourdhui, dir).await;
    let avant = image(&ent, etab, &[a.id, b.id]).await;

    let ap = ent.apercu_fusion_pour_acteur(DIR, etab, dir, b.id, a.id).await.unwrap();
    let tables: Vec<&str> = ap.conflits.iter().map(|c| c.table.as_str()).collect();
    assert!(tables.contains(&"ent_eleve_scolarisation"), "{tables:?}");
    assert!(tables.contains(&"ent_eleve_identifiant"), "{tables:?}");
    assert!(tables.contains(&"ent_eleve_inscription"), "{tables:?}");
    assert!(ap.avertissements.contains(&AvertissementFusion::AbsorbeEncoreScolarise));
    assert!(ap.avertissements.contains(&AvertissementFusion::IneDifferents));
    assert_identique(&avant, &image(&ent, etab, &[a.id, b.id]).await);

    // Sans acquittement : refusée, rien n'est écrit.
    let r = ent.fusionner_eleves_pour_acteur(DIR, demande(etab, b.id, a.id, dir)).await.unwrap();
    let ResultatFusion::ConflitsAResoudre(restants) = r else { panic!("conflits attendus") };
    assert_eq!(restants.len(), ap.conflits.len());
    assert_identique(&avant, &image(&ent, etab, &[a.id, b.id]).await);
    // Acquittement partiel : refusée aussi.
    let mut partielle = demande(etab, b.id, a.id, dir);
    partielle.conflits_acquittes = ap.conflits[..1].to_vec();
    assert!(matches!(
        ent.fusionner_eleves_pour_acteur(DIR, partielle).await.unwrap(),
        ResultatFusion::ConflitsAResoudre(_)
    ));
    // Acquittement complet : faite. Les lignes en conflit restent sur l'absorbé.
    let mut complete = demande(etab, b.id, a.id, dir);
    complete.conflits_acquittes = ap.conflits.clone();
    let ResultatFusion::Faite(f) = ent.fusionner_eleves_pour_acteur(DIR, complete).await.unwrap() else { panic!("acquittés") };
    assert_eq!(f.conflits.len(), ap.conflits.len());
    assert_eq!(nb(&ent, "ent_eleve_scolarisation", etab, a.id).await, 1, "séjour de A inchangé");
    assert_eq!(nb(&ent, "ent_eleve_scolarisation", etab, b.id).await, 1, "séjour en conflit resté sur B");
    assert_eq!(nb(&ent, "ent_absence", etab, a.id).await, 3, "sans conflit : passent à A");
    let ine_a: Option<String> = sqlx::query_scalar("SELECT ine FROM ent_eleve WHERE id = $1").bind(a.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(ine_a.as_deref(), Some(ine.as_str()));
    // Défusion à l'identique.
    let ResultatDefusion::Faite(_) = ent.defusionner_eleves_pour_acteur(DIR, etab, dir, f.id).await.unwrap() else { panic!("défusion") };
    assert_identique(&avant, &image(&ent, etab, &[a.id, b.id]).await);
}

/// Une ligne déplacée modifiée (ou supprimée) depuis la fusion bloque la défusion : la liste est
/// montrée, rien n'est restauré ; une fois la ligne rétablie, la défusion passe.
#[tokio::test]
async fn defusion_refusee_si_une_ligne_deplacee_a_change() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (Uuid::new_v4(), cast.direction.user_id);
    let (a, b) = deux_fiches(&ent, &cast, etab).await;
    let avant = image(&ent, etab, &[a.id, b.id]).await;
    let f = fuser(&ent, etab, b.id, a.id, dir).await;
    // Une absence de B (maintenant sur A) est justifiée depuis la fusion ; un retard est supprimé.
    let (abs_id,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2 AND debut = $3")
        .bind(etab).bind(a.id).bind(jour(2025, 10, 3)).fetch_one(ent.pool_verif()).await.unwrap();
    sqlx::query("UPDATE ent_absence SET motif = 'rdv_medical' WHERE id = $1").bind(abs_id).execute(ent.pool_verif()).await.unwrap();
    let (ret,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_retard WHERE etablissement_id = $1 AND eleve_id = $2 AND jour = $3")
        .bind(etab).bind(a.id).bind(jour(2025, 11, 2)).fetch_one(ent.pool_verif()).await.unwrap();
    let ligne_retard: Value = sqlx::query_scalar("SELECT to_jsonb(r) FROM ent_retard r WHERE id = $1").bind(ret).fetch_one(ent.pool_verif()).await.unwrap();
    sqlx::query("DELETE FROM ent_retard WHERE id = $1").bind(ret).execute(ent.pool_verif()).await.unwrap();
    let r = ent.defusionner_eleves_pour_acteur(DIR, etab, dir, f.id).await.unwrap();
    let ResultatDefusion::LignesModifiees(l) = r else { panic!("défusion refusée attendue") };
    assert_eq!(l.len(), 2, "{l:?}");
    assert!(l.iter().any(|x| x.table == "ent_absence" && x.raison == "modifiee"));
    assert!(l.iter().any(|x| x.table == "ent_retard" && x.raison == "absente"));
    assert_eq!(l[0].domaine, "Absences et retards");
    // Rien n'a bougé : B reste absorbé.
    assert_eq!(nb(&ent, "ent_absence", etab, b.id).await, 0);
    assert_eq!(ent.eleve_effectif(etab, b.id).await.unwrap(), a.id);
    // On rétablit les deux lignes telles qu'elles étaient : la défusion passe, à l'identique.
    sqlx::query("UPDATE ent_absence SET motif = 'maladie' WHERE id = $1").bind(abs_id).execute(ent.pool_verif()).await.unwrap();
    sqlx::query("INSERT INTO ent_retard SELECT * FROM jsonb_populate_record(NULL::ent_retard, $1)").bind(&ligne_retard).execute(ent.pool_verif()).await.unwrap();
    let ResultatDefusion::Faite(_) = ent.defusionner_eleves_pour_acteur(DIR, etab, dir, f.id).await.unwrap() else { panic!("défusion") };
    assert_identique(&avant, &image(&ent, etab, &[a.id, b.id]).await);
}

/// Délai de 30 jours, et ordre inverse : on n'annule pas une fusion dont le conservé a été
/// absorbé depuis.
#[tokio::test]
async fn delai_d_annulation_et_fusions_chainees() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (Uuid::new_v4(), cast.direction.user_id);
    let a = eleve(&ent, etab, &format!("Chaine{}", alea(4)), "Eva", None, None, Some(jour(2025, 9, 1)), None).await;
    let b = eleve(&ent, etab, &format!("Chaine{}", alea(4)), "Eva", None, None, Some(jour(2024, 9, 1)), Some(jour(2025, 7, 1))).await;
    let c = eleve(&ent, etab, &format!("Chaine{}", alea(4)), "Eva", None, None, Some(jour(2023, 9, 1)), Some(jour(2024, 7, 1))).await;
    donnees(&ent, etab, b.id, 1, 0, 0, 0).await;
    donnees(&ent, etab, a.id, 1, 0, 0, 0).await;
    // B → A, puis A → C.
    let f1 = fuser(&ent, etab, b.id, a.id, dir).await;
    let f2 = fuser(&ent, etab, a.id, c.id, dir).await;
    assert_eq!(nb(&ent, "ent_absence", etab, c.id).await, 2);
    assert_eq!(ent.eleve_effectif(etab, b.id).await.unwrap(), c.id, "la chaîne est suivie");
    // Annuler F1 alors que A est absorbé : refusé (ordre inverse).
    assert!(matches!(
        ent.defusionner_eleves_pour_acteur(DIR, etab, dir, f1.id).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    let ResultatDefusion::Faite(_) = ent.defusionner_eleves_pour_acteur(DIR, etab, dir, f2.id).await.unwrap() else { panic!() };
    let ResultatDefusion::Faite(_) = ent.defusionner_eleves_pour_acteur(DIR, etab, dir, f1.id).await.unwrap() else { panic!() };
    assert_eq!(nb(&ent, "ent_absence", etab, b.id).await, 1);
    assert_eq!(nb(&ent, "ent_absence", etab, a.id).await, 1);
    // Au-delà de 30 jours, la défusion est refusée.
    let f3 = fuser(&ent, etab, b.id, a.id, dir).await;
    sqlx::query("ALTER TABLE ent_eleve_fusion DISABLE TRIGGER ent_eleve_fusion_garde").execute(ent.pool_verif()).await.unwrap();
    sqlx::query("UPDATE ent_eleve_fusion SET fusionne_le = now() - interval '31 days' WHERE id = $1").bind(f3.id).execute(ent.pool_verif()).await.unwrap();
    sqlx::query("ALTER TABLE ent_eleve_fusion ENABLE TRIGGER ent_eleve_fusion_garde").execute(ent.pool_verif()).await.unwrap();
    assert!(matches!(
        ent.defusionner_eleves_pour_acteur(DIR, etab, dir, f3.id).await,
        Err(EntDbError::ConflitDossier(m)) if m.contains("délai")
    ));
    // La garde SQL : une fusion ne se réécrit ni ne se supprime ; son registre est en ajout seul.
    assert!(sqlx::query("UPDATE ent_eleve_fusion SET motif = 'x' WHERE id = $1").bind(f3.id).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_eleve_fusion WHERE id = $1").bind(f3.id).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_eleve_fusion_ligne WHERE fusion_id = $1").bind(f3.id).execute(ent.pool_verif()).await.is_err());
}

/// Droits : seule la portée établissement de `eleves.fusionner` ; motif et cloisonnement.
#[tokio::test]
async fn droits_motif_et_cloisonnement() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, autre, dir) = (Uuid::new_v4(), Uuid::new_v4(), cast.direction.user_id);
    let a = eleve(&ent, etab, &format!("Droit{}", alea(4)), "Sam", None, None, Some(jour(2025, 9, 1)), None).await;
    let b = eleve(&ent, etab, &format!("Droit{}", alea(4)), "Sam", None, None, Some(jour(2024, 9, 1)), Some(jour(2025, 7, 1))).await;
    let ailleurs = eleve(&ent, autre, &format!("Droit{}", alea(4)), "Sam", None, None, Some(jour(2025, 9, 1)), None).await;
    // Secrétariat, CPE, comptabilité : pas de fusion (Admin et Direction seulement).
    for (role, user) in [
        (Role::Administratif, cast.administratif.user_id),
        (Role::Cpe, cast.cpe.user_id),
        (Role::Compta, cast.compta.user_id),
        (Role::Prof, cast.prof.user_id),
    ] {
        assert!(matches!(
            ent.fusionner_eleves_pour_acteur(role.as_str(), demande(etab, b.id, a.id, user)).await,
            Err(EntDbError::Forbidden)
        ), "{role:?}");
        assert!(matches!(
            ent.apercu_fusion_pour_acteur(role.as_str(), etab, user, b.id, a.id).await,
            Err(EntDbError::Forbidden)
        ), "{role:?}");
        assert!(matches!(
            ent.fusions_pour_acteur(role.as_str(), etab, user, 10).await,
            Err(EntDbError::Forbidden)
        ), "{role:?}");
    }
    // Motif : code inconnu, « autre » sans texte, texte trop long.
    let mut d = demande(etab, b.id, a.id, dir);
    d.motif_code = "parce que".into();
    assert!(matches!(ent.fusionner_eleves_pour_acteur(DIR, d).await, Err(EntDbError::SaisieInvalide(_))));
    let mut d = demande(etab, b.id, a.id, dir);
    d.motif_code = "autre".into();
    assert!(matches!(ent.fusionner_eleves_pour_acteur(DIR, d).await, Err(EntDbError::SaisieInvalide(_))));
    let mut d = demande(etab, b.id, a.id, dir);
    d.motif = Some("x".repeat(501));
    assert!(matches!(ent.fusionner_eleves_pour_acteur(DIR, d).await, Err(EntDbError::SaisieInvalide(_))));
    // Un autre établissement n'existe pas.
    assert!(matches!(
        ent.fusionner_eleves_pour_acteur(DIR, demande(etab, b.id, ailleurs.id, dir)).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    assert!(matches!(
        ent.apercu_fusion_pour_acteur(DIR, etab, dir, ailleurs.id, a.id).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    assert!(ent.fusions_pour_acteur(DIR, autre, dir, 10).await.unwrap().is_empty());
    assert_eq!(nb(&ent, "ent_eleve_scolarisation", etab, a.id).await, 1, "rien n'a été fait");
}

/// Un élève en confidentialité renforcée : l'aperçu le signale (direction), la fusion fonctionne
/// pour qui détient G6 et la mesure suit l'élève conservé.
#[tokio::test]
async fn eleve_protege_signale_et_mesure_conservee() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, dir) = (Uuid::new_v4(), cast.direction.user_id);
    let a = eleve(&ent, etab, &format!("Prot{}", alea(4)), "Léo", None, None, Some(jour(2025, 9, 1)), None).await;
    let b = eleve(&ent, etab, &format!("Prot{}", alea(4)), "Léo", None, None, Some(jour(2024, 9, 1)), Some(jour(2025, 7, 1))).await;
    sqlx::query(
        "INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, motif_scelle, mesures, du)
         VALUES (gen_random_uuid(), $1, $2, 'enc:v1:test', ARRAY['hors_annuaire'], current_date - 1)",
    )
    .bind(etab)
    .bind(b.id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let ap = ent.apercu_fusion_pour_acteur(DIR, etab, dir, b.id, a.id).await.unwrap();
    assert!(ap.avertissements.contains(&AvertissementFusion::EleveProtege));
    let f = fuser(&ent, etab, b.id, a.id, dir).await;
    let protege: bool = sqlx::query_scalar("SELECT ent_eleve_confidentiel($1, $2)").bind(etab).bind(a.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(protege, "la mesure de B suit la fiche conservée");
    // Les fusions qui touchent un élève protégé n'existent pas pour qui n'a pas G6 : ici la
    // direction l'a, donc la liste la montre.
    assert_eq!(ent.fusions_pour_acteur(DIR, etab, dir, 10).await.unwrap().len(), 1);
    let ResultatDefusion::Faite(_) = ent.defusionner_eleves_pour_acteur(DIR, etab, dir, f.id).await.unwrap() else { panic!() };
    let protege_a: bool = sqlx::query_scalar("SELECT ent_eleve_confidentiel($1, $2)").bind(etab).bind(a.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(!protege_a, "après défusion, la mesure est revenue à B");
}

/// **Test de garde du registre** : toute table qui porte `eleve_id` est déclarée dans
/// `TABLES_ELEVE` avec sa vraie clé primaire ; toute table déclarée existe. Une migration qui
/// ajoute une table avec `eleve_id` sans la déclarer fait échouer ce test.
#[tokio::test]
async fn garde_du_registre_tables_eleve() {
    let Some((ent, _)) = labo().await else { return };
    let reelles: Vec<(String,)> = sqlx::query_as(
        "SELECT c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace AND n.nspname = 'public'
          WHERE c.relkind IN ('r', 'p')
            AND EXISTS (SELECT 1 FROM pg_attribute a WHERE a.attrelid = c.oid AND a.attname = 'eleve_id' AND NOT a.attisdropped)
          ORDER BY 1",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let reelles: Vec<String> = reelles.into_iter().map(|(n,)| n).collect();
    let declarees: Vec<String> = TABLES_ELEVE.iter().map(|t| t.table.to_string()).collect();
    let non_declarees: Vec<&String> = reelles.iter().filter(|t| !declarees.contains(t)).collect();
    assert!(
        non_declarees.is_empty(),
        "tables qui portent `eleve_id` sans être déclarées dans TABLES_ELEVE (domain-eleve/src/fusion.rs) : {non_declarees:?}. \\
         Choisir `Deplacer` (les lignes suivent l'élève conservé) ou `Reste` (historique propre à la fiche, avec la raison)."
    );
    let disparues: Vec<&String> = declarees.iter().filter(|t| !reelles.contains(t)).collect();
    assert!(disparues.is_empty(), "tables déclarées qui n'ont plus de colonne `eleve_id` : {disparues:?}");
    // La clé primaire déclarée est la vraie (elle identifie les lignes déplacées), et chaque table
    // déplacée porte l'établissement (la fusion filtre toujours par établissement).
    for t in TABLES_ELEVE {
        let (pk, etab): (Vec<String>, bool) = sqlx::query_as(
            "SELECT coalesce((SELECT array_agg(a.attname::text ORDER BY array_position(i.indkey::int2[], a.attnum))
                                FROM pg_index i JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
                               WHERE i.indrelid = ($1)::regclass AND i.indisprimary), '{}'::text[]),
                    EXISTS (SELECT 1 FROM pg_attribute a WHERE a.attrelid = ($1)::regclass AND a.attname = 'etablissement_id')",
        )
        .bind(t.table)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(pk, t.cle.iter().map(|s| s.to_string()).collect::<Vec<_>>(), "clé primaire de {}", t.table);
        assert!(etab, "{} sans etablissement_id", t.table);
    }
    // Les tables qui désignent deux élèves sous d'autres noms (`eleve_a`, `eleve_b`) ne sont pas
    // dans le registre : la fusion les met à jour explicitement (doublons, non-doublons).
    assert!(reelles.iter().all(|t| t != "ent_eleve_doublon" && t != "ent_eleve_non_doublon"));
    // Les stratégies `Reste` disent pourquoi.
    assert!(TABLES_ELEVE.iter().filter(|t| t.strategie == StrategieFusion::Reste).all(|t| !t.raison.is_empty()));
}
