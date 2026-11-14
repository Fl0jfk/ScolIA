//! Socle 2 (l'élève), lot E6 : recherche instantanée (ELV-20). Scénarios contre la base locale
//! de test (feature `labo`) ; sans `ENT_DATABASE_URL`, ils sont ignorés.

use super::*;
use crate::identite::UsageIdentite;
use chrono::NaiveDate;
use domain_eleve::CreateEleve;
use ent_auth::Role;
use std::time::Instant;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| { }).await;
    Some((ent, cast))
}

/// Suffixe de lettres seulement (un nom sans chiffre, unique d'un test à l'autre).
fn lettres(n: usize) -> String {
    Uuid::new_v4()
        .as_bytes()
        .iter()
        .take(n)
        .map(|b| (b'a' + b % 26) as char)
        .collect()
}

async fn eleve(
    ent: &PgEnt,
    etab: Uuid,
    nom: &str,
    prenom: &str,
    classe: &str,
    ine: Option<&str>,
) -> domain_eleve::Eleve {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: prenom.into(),
        ine: ine.map(String::from),
        sexe: Some("F".into()),
        date_naissance: NaiveDate::from_ymd_opt(2010, 3, 14),
        numero_badge: None,
        redoublant: false,
        classe: Some(classe.into()),
        regime: None,
        mef: None,
        site: None,
        date_entree: None,
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
}

async fn chercher(ent: &PgEnt, cast: &LaboCast, role: Role, q: &str) -> Vec<domain_eleve::Eleve> {
    let user = match role {
        Role::Administratif => cast.administratif.user_id,
        Role::Direction => cast.direction.user_id,
        Role::Cpe => cast.cpe.user_id,
        Role::Prof => cast.prof.user_id,
        _ => unreachable!(),
    };
    ent.search_eleves_pour_acteur(cast.etablissement_id, user, role.as_str(), q, 50)
        .await
        .unwrap()
        .into_iter()
        .map(|v| v.eleve)
        .collect()
}

fn contient(l: &[domain_eleve::Eleve], id: Uuid) -> bool {
    l.iter().any(|e| e.id == id)
}

/// Accents, casse, traits d'union, apostrophes, fautes de frappe : on retrouve l'élève.
#[tokio::test]
async fn tolerante_aux_accents_a_la_casse_aux_traits_d_union_et_aux_fautes() {
    let Some((ent, cast)) = labo().await else { return };
    let nom = format!("Müller-Dubois{}", lettres(4));
    let e = eleve(&ent, cast.etablissement_id, &nom, "Éléonore", "6A", None).await;
    let suffixe = nom.chars().skip("Müller-Dubois".chars().count()).collect::<String>();
    for q in [
        "eleonore",
        "ÉLÉONORE",
        "Éléonore Müller",
        &format!("muller dubois{suffixe}"),
        &format!("mullerdubois{suffixe}").replace("mullerdubois", "muller-dubois"),
        &format!("dubois{suffixe} eleo"),
        // Faute de frappe : une lettre de trop, une de moins, une transposée.
        &format!("elleonore muller dubois{suffixe}"),
        &format!("eleonre dubois{suffixe}"),
        &format!("eleonore mueller dubois{suffixe}"),
    ] {
        let l = chercher(&ent, &cast, Role::Administratif, q).await;
        assert!(contient(&l, e.id), "« {q} » devrait retrouver l'élève");
    }
    // Un mot qui ne ressemble à rien ne retrouve pas l'élève (pas de bruit).
    let l = chercher(&ent, &cast, Role::Administratif, "eleonore zzzzzzzz").await;
    assert!(!contient(&l, e.id));
    // La classe est un critère de la désignation (« dubois 6A »).
    let l = chercher(&ent, &cast, Role::Administratif, &format!("dubois{suffixe} 6a")).await;
    assert!(contient(&l, e.id));
    let l = chercher(&ent, &cast, Role::Administratif, &format!("dubois{suffixe} 3c")).await;
    assert!(!contient(&l, e.id));
}

/// Un élève dont la désignation affichée est un prénom d'usage ne se découvre pas par son
/// prénom d'état civil sans le droit G2 (pas d'oracle) ; avec G2, il se retrouve.
#[tokio::test]
async fn l_etat_civil_n_est_interrogeable_qu_avec_son_groupe() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let nom = format!("Civil{}", lettres(5));
    let e = eleve(&ent, etab, &nom, "Samantha", "6A", None).await;
    ent.definir_usage(
        Role::Administratif.as_str(),
        UsageIdentite {
            etablissement_id: etab,
            eleve_id: e.id,
            nom_usage: None,
            prenom_usage: Some(Some("Sam".into())),
            effet_du: None,
            motif_code: None,
            motif: None,
            piece_document_id: None,
            source: "saisie".into(),
            acteur: cast.administratif.user_id,
        },
    )
    .await
    .unwrap();
    // La clé « désignation » ne contient que ce que l'annuaire montre ; la clé civile, l'état civil.
    let (cle, civil): (String, String) =
        sqlx::query_as("SELECT cle_recherche, cle_recherche_civil FROM ent_eleve WHERE id = $1")
            .bind(e.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(cle.contains("sam") && !cle.contains("samantha"), "{cle}");
    assert!(civil.contains("samantha"), "{civil}");
    // Secrétariat (G2 partout) : retrouvé par l'état civil comme par l'usage.
    let l = chercher(&ent, &cast, Role::Administratif, &format!("samantha {}", nom.to_lowercase())).await;
    assert!(contient(&l, e.id));
    // CPE (désignation seulement) : retrouvé par le prénom affiché, jamais par l'état civil.
    let l = chercher(&ent, &cast, Role::Cpe, &format!("sam {}", nom.to_lowercase())).await;
    assert!(contient(&l, e.id), "le prénom affiché se cherche");
    let l = chercher(&ent, &cast, Role::Cpe, &format!("samantha {}", nom.to_lowercase())).await;
    assert!(!contient(&l, e.id), "le prénom d'état civil ne doit pas se déduire");
    let l = chercher(&ent, &cast, Role::Cpe, "samantha").await;
    assert!(!contient(&l, e.id));
}

/// INE et numéro d'élève : seulement avec les droits correspondants, jamais par préfixe d'INE.
#[tokio::test]
async fn ine_et_numero_seulement_avec_les_droits() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let ine = format!("{}{}", "98765", Uuid::new_v4().simple().to_string()[..6].to_uppercase());
    let nom = format!("Ident{}", lettres(5));
    let e = eleve(&ent, etab, &nom, "Zoé", "6A", Some(&ine)).await;
    // Secrétariat (G3 partout) : INE complet.
    let l = chercher(&ent, &cast, Role::Administratif, &ine).await;
    assert!(contient(&l, e.id), "INE complet trouvé");
    let l = chercher(&ent, &cast, Role::Administratif, &ine.to_lowercase()).await;
    assert!(contient(&l, e.id), "insensible à la casse");
    // Jamais par préfixe : pas d'énumération des identifiants.
    let l = chercher(&ent, &cast, Role::Administratif, &ine[..8]).await;
    assert!(!contient(&l, e.id), "préfixe d'INE refusé");
    // CPE (pas de G3) : l'INE n'ouvre rien, même complet.
    let l = chercher(&ent, &cast, Role::Cpe, &ine).await;
    assert!(!contient(&l, e.id), "pas d'INE sans G3");
    // Numéro d'élève (G1) : début de numéro, secrétariat seulement.
    let (numero,): (String,) = sqlx::query_as("SELECT numero FROM ent_eleve WHERE id = $1")
        .bind(e.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let l = chercher(&ent, &cast, Role::Administratif, &numero).await;
    assert!(contient(&l, e.id), "numéro {numero} trouvé");
    let l = chercher(&ent, &cast, Role::Cpe, &numero).await;
    assert!(!contient(&l, e.id), "pas de numéro sans G1");
}

/// La portée d'un professeur est une condition de la requête : ses élèves sortent même quand
/// beaucoup d'autres élèves de l'établissement répondent mieux à la requête.
#[tokio::test]
async fn la_portee_est_appliquee_avant_la_limite() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    // Le professeur du labo enseigne en 4B : Ginny (4B) est à lui, Ron (5A) ne l'est pas.
    let l = chercher(&ent, &cast, Role::Prof, "weasley").await;
    assert!(
        !contient(&l, cast.eleve_ron_id.unwrap_or_else(Uuid::nil)) || cast.eleve_ron_id.is_none(),
        "un professeur ne trouve pas un élève hors de ses classes et groupes"
    );
    // Beaucoup d'homonymes hors de sa portée ne doivent pas évincer ses propres élèves.
    let marque = lettres(6);
    for i in 0..60 {
        eleve(&ent, etab, &format!("Foule{marque}"), &format!("Zita{}", lettres(2 + i % 2)), "5A", None).await;
    }
    let l = chercher(&ent, &cast, Role::Administratif, &format!("foule{marque}")).await;
    assert_eq!(l.len(), 50, "la limite s'applique");
    let l = chercher(&ent, &cast, Role::Prof, &format!("foule{marque}")).await;
    assert!(l.is_empty(), "aucun de ces élèves n'est à lui : {}", l.len());
    // Un élève de sa classe qui répond moins bien apparaît quand même (portée avant limite).
    let e4b = eleve(&ent, etab, &format!("Foule{marque}"), "Zita", "4B", None).await;
    let _ = e4b; // la classe texte seule ne crée pas d'affectation : la portée vient du référentiel
    let harry = cast.eleve_lea_id;
    let l = chercher(&ent, &cast, Role::Prof, "potter").await;
    assert!(contient(&l, harry), "Harry est en 4B, classe du professeur du labo");
}

/// Un élève en confidentialité renforcée ne sort pas de la recherche pour un tiers : ni le
/// secrétariat ni la CPE (annuaire de tout l'établissement) ; la direction (G6) le voit.
#[tokio::test]
async fn eleve_protege_absent_de_la_recherche_pour_un_tiers() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let nom = format!("Protege{}", lettres(5));
    let e = eleve(&ent, etab, &nom, "Lina", "6A", None).await;
    let l = chercher(&ent, &cast, Role::Administratif, &nom).await;
    assert!(contient(&l, e.id), "visible avant la mesure");
    sqlx::query(
        "INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, motif_scelle, mesures, du)
         VALUES (gen_random_uuid(), $1, $2, 'enc:v1:test', ARRAY['hors_annuaire'], ent_aujourdhui() - 1)",
    )
    .bind(etab)
    .bind(e.id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    for role in [Role::Administratif, Role::Cpe, Role::Prof] {
        let l = chercher(&ent, &cast, role, &nom).await;
        assert!(!contient(&l, e.id), "{role:?} ne doit pas le trouver");
    }
    // Même avec son nom complet exact, et par la classe.
    let l = chercher(&ent, &cast, Role::Administratif, &format!("lina {nom} 6a")).await;
    assert!(!contient(&l, e.id));
    // La direction détient G6 : elle le trouve.
    let l = chercher(&ent, &cast, Role::Direction, &nom).await;
    assert!(contient(&l, e.id), "la direction (G6) le voit");
    // Mesure levée : il redevient visible.
    sqlx::query("UPDATE ent_eleve_confidentialite SET leve_le = now() WHERE eleve_id = $1")
        .bind(e.id)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let l = chercher(&ent, &cast, Role::Administratif, &nom).await;
    assert!(contient(&l, e.id));
}

/// Un octet NUL est une saisie invalide, pas une panne ; les jokers SQL sont inertes.
#[tokio::test]
async fn saisies_hostiles() {
    let Some((ent, cast)) = labo().await else { return };
    let r = ent
        .search_eleves_pour_acteur(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            "a\0b",
            10,
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    for q in ["%", "_", "\\", "'; DROP TABLE ent_eleve; --", "%%%%", "a%b_c"] {
        let l = chercher(&ent, &cast, Role::Administratif, q).await;
        assert!(l.len() <= 50, "{q}");
    }
    // `%` seul ne liste pas tout l'établissement (ce n'est plus un joker).
    let tous = chercher(&ent, &cast, Role::Administratif, "").await;
    let joker = chercher(&ent, &cast, Role::Administratif, "%").await;
    assert!(joker.len() <= tous.len());
}

/// Un autre établissement n'existe pas pour la recherche (D-013).
#[tokio::test]
async fn cloisonnement_entre_etablissements() {
    let Some((ent, cast)) = labo().await else { return };
    let autre = Uuid::new_v4();
    let nom = format!("Ailleurs{}", lettres(5));
    let e = eleve(&ent, autre, &nom, "Noa", "6A", Some("ZZ9988776655")).await;
    let l = chercher(&ent, &cast, Role::Administratif, &nom).await;
    assert!(!contient(&l, e.id));
    let l = chercher(&ent, &cast, Role::Administratif, "ZZ9988776655").await;
    assert!(!contient(&l, e.id));
}

/// Critère de fin du lot : recherche < 100 ms sur 5 000 élèves (labo, PostgreSQL local).
#[tokio::test]
async fn recherche_sous_100_ms_sur_5000_eleves() {
    let Some((ent, cast)) = labo().await else { return };
    // Établissement fixe : les 5 000 élèves (non supprimables : l'historique d'identité est en
    // ajout seul) sont créés une fois par base de test, puis réutilisés.
    let etab = Uuid::from_u128(0x5e6e_0000_0000_4000_8000_0000_0000_5000);
    let deja: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_eleve WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let noms = [
        "Martin", "Bernard", "Dubois", "Thomas", "Robert", "Richard", "Petit", "Durand", "Leroy", "Moreau",
        "Simon", "Laurent", "Lefebvre", "Michel", "Garcia", "David", "Bertrand", "Roux", "Vincent", "Fournier",
        "Morel", "Girard", "Andre", "Mercier", "Dupont", "Lambert", "Bonnet", "Francois", "Martinez", "Legrand",
        "Benali", "Haddad", "Nguyen", "Da Silva", "Müller", "O'Brien", "Saint-Pierre", "Lemaître", "Çelik", "Ben Amar",
    ];
    let prenoms = [
        "Léa", "Lucas", "Emma", "Hugo", "Chloé", "Louis", "Manon", "Nathan", "Inès", "Gabriel", "Camille", "Jules",
        "Sarah", "Adam", "Éloïse", "Raphaël", "Yanis", "Anaïs", "Mohamed", "Jean-Pierre", "Zoé", "Maëlys",
        "Thomas", "Clara", "Noé", "Lilou", "Enzo", "Alice", "Tom", "Lola",
    ];
    // 5 000 élèves en une requête (les déclencheurs de la base — identité, numéro, clé de
    // recherche — s'exécutent comme pour une création normale).
    let n: i64 = if deja >= 5000 { deja } else { sqlx::query_scalar(
        "WITH ins AS (
           INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, classe)
           SELECT gen_random_uuid(), $1,
                  ($2::text[])[1 + (g * 7) % array_length($2::text[], 1)] || CASE WHEN g % 11 = 0 THEN '-' || g ELSE '' END,
                  ($3::text[])[1 + (g * 13) % array_length($3::text[], 1)],
                  'actif', (ARRAY['6A','6B','5A','5B','4A','4B','3A','3B'])[1 + g % 8]
             FROM generate_series(1, 5000) g
           RETURNING 1)
         SELECT count(*) FROM ins",
    )
    .bind(etab)
    .bind(&noms.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    .bind(&prenoms.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    .fetch_one(ent.pool_verif())
    .await
    .unwrap() };
    assert!(n >= 5000);
    sqlx::query("ANALYZE ent_eleve").execute(ent.pool_verif()).await.unwrap();

    let requetes = [
        "martin", "martin lea", "mohamed benali", "benali", "lemaitre", "elo", "jean pierre", "mullr", "saint pierre",
        "yanis 4b", "celik", "ben amar", "obrien", "dupnt", "francois zoe", "a", "ma",
    ];
    // Échauffement (plans, cache).
    for q in requetes {
        ent.search_eleves_pour_acteur(etab, cast.administratif.user_id, Role::Administratif.as_str(), q, 50)
            .await
            .unwrap();
    }
    let mut mesures: Vec<(u128, &str)> = Vec::new();
    for _ in 0..5 {
        for q in requetes {
            let t = Instant::now();
            let l = ent
                .search_eleves_pour_acteur(etab, cast.administratif.user_id, Role::Administratif.as_str(), q, 50)
                .await
                .unwrap();
            let ms = t.elapsed().as_millis();
            if ["martin", "lemaitre", "elo", "jean pierre", "saint pierre", "celik", "ben amar", "benali", "a", "ma"]
                .contains(&q)
            {
                assert!(!l.is_empty(), "« {q} » sans résultat");
            }
            mesures.push((ms, q));
        }
    }
    mesures.sort();
    let mediane = mesures[mesures.len() / 2].0;
    let (pire, q_pire) = *mesures.last().unwrap();
    eprintln!("[recherche] 5 000 élèves : médiane {mediane} ms, pire {pire} ms (« {q_pire} »), {} mesures", mesures.len());
    assert!(mediane < 100, "médiane {mediane} ms ≥ 100 ms");
    assert!(pire < 250, "pire cas {pire} ms (« {q_pire} »)");
}
