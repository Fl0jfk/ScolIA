//! Socle 29, lot P5 : recherche globale (`docs/socles/29_PORTAILS.md` §12.1, §18.6). Base locale de test
//! (feature `labo`). Instants fixes, aucune heure réelle ; chaque test ne lit que ses propres lignes (compteurs
//! de ses propres comptes, personnes à nom aléatoire). Un témoin précède chaque mesure de protection.

use super::*;
use crate::recherche_globale::{PLAFOND_LETTRES, PLAFOND_PERSONNEL};
use chrono::{DateTime, Duration, TimeZone, Utc};
use ent_types::recherche::{ReponseRecherche, SourceRecherche, MESSAGE_RECHERCHE_BLOQUEE};
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
    })
    .await;
    Some((ent, cast))
}

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2031, 3, 12, 10, 0, 0).unwrap()
}

fn bloquee<T: std::fmt::Debug>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::ConflitDossier(m)) if *m == MESSAGE_RECHERCHE_BLOQUEE)
}

async fn chercher(
    ent: &PgEnt,
    c: &LaboCast,
    p: &LaboPersona,
    q: &str,
) -> Result<ReponseRecherche, EntDbError> {
    ent.rechercher_global(c.etablissement_id, p.user_id, p.role.as_str(), q, t0())
        .await
}

async fn remettre_a_zero(ent: &PgEnt, compte: Uuid) {
    sqlx::query("DELETE FROM ent_recherche_compteur WHERE compte_id = $1")
        .bind(compte)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

async fn preparer_compteur(ent: &PgEnt, etab: Uuid, compte: Uuid, nb: i32, courtes: i32) {
    remettre_a_zero(ent, compte).await;
    sqlx::query(
        "INSERT INTO ent_recherche_compteur (etablissement_id, compte_id, jour, nb, nb_courtes) VALUES ($1, $2, '2031-03-12', $3, $4)",
    )
    .bind(etab)
    .bind(compte)
    .bind(nb)
    .bind(courtes)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

async fn compteur(ent: &PgEnt, compte: Uuid) -> (i32, i32) {
    sqlx::query_as("SELECT nb, nb_courtes FROM ent_recherche_compteur WHERE compte_id = $1 AND jour = '2031-03-12'")
        .bind(compte)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap_or((0, 0))
}

async fn taches_blocage(ent: &PgEnt, etab: Uuid, compte: Uuid) -> Vec<String> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT etat FROM ent_afaire WHERE etablissement_id = $1 AND nature_code = 'recherche.bloquee' AND objet_id = $2",
    )
    .bind(etab)
    .bind(compte)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    rows.into_iter().map(|(e,)| e).collect()
}

fn nom_unique() -> String {
    // Lettres seules (12 caractères) : la clé de recherche ignore les chiffres isolés.
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .map(|c| {
            if c.is_ascii_digit() {
                (b'g' + c.to_digit(10).unwrap() as u8) as char
            } else {
                c
            }
        })
        .take(12)
        .collect()
}

#[tokio::test]
async fn p5_sources_sous_les_droits_et_libelles_minimaux() {
    let Some((ent, c)) = labo().await else { return };
    for p in [&c.direction, &c.cpe] {
        remettre_a_zero(&ent, p.user_id).await;
    }
    // Élèves : la direction retrouve Harry, sans accent ni casse, avec un libellé minimal et un lien.
    let r = chercher(&ent, &c, &c.direction, "potter").await.unwrap();
    let harry = r
        .resultats
        .iter()
        .find(|x| x.source == SourceRecherche::Eleves && x.libelle.starts_with("POTTER Harry"))
        .expect("témoin : Harry est trouvé");
    assert_eq!(harry.libelle, "POTTER Harry — 4B");
    assert!(harry
        .lien
        .as_deref()
        .is_some_and(|l| l.starts_with("/web/eleves/")));
    // Classes : « 4B » est trouvée dans le référentiel.
    let r = chercher(&ent, &c, &c.direction, "4b").await.unwrap();
    assert!(
        r.resultats
            .iter()
            .any(|x| x.source == SourceRecherche::Classes && x.libelle.contains("4B")),
        "{r:?}"
    );
    // Personnels : le nom seulement, jamais l'adresse électronique.
    let r = chercher(&ent, &c, &c.direction, "lupin").await.unwrap();
    assert!(
        r.resultats
            .iter()
            .any(|x| x.source == SourceRecherche::Personnels && x.libelle == "Remus Lupin"),
        "{r:?}"
    );
    let dump = format!("{r:?}");
    assert!(
        !dump.contains('@') && !dump.contains("ent.lab"),
        "aucune adresse : {dump}"
    );
    // Limites : 10 par source, 40 au total.
    let r = chercher(&ent, &c, &c.direction, "e").await; // trop court : rien
    assert!(r.unwrap().resultats.is_empty());
    for source in [
        SourceRecherche::Eleves,
        SourceRecherche::Personnes,
        SourceRecherche::Classes,
        SourceRecherche::Personnels,
    ] {
        let r = chercher(&ent, &c, &c.direction, "ro").await.unwrap();
        assert!(r.resultats.iter().filter(|x| x.source == source).count() <= 10);
        assert!(r.resultats.len() <= 40);
    }
    // Les familles n'ont pas cet outil.
    for p in [&c.parent_lea, &c.parent_tom] {
        assert!(matches!(
            chercher(&ent, &c, p, "potter").await,
            Err(EntDbError::Forbidden)
        ));
    }
    // Aucune injection : les jokers ne dégénèrent pas en annuaire complet.
    let r = chercher(&ent, &c, &c.direction, "%%").await.unwrap();
    assert!(r.resultats.is_empty(), "{r:?}");
}

#[tokio::test]
async fn p5_adresse_protegee_et_ne_pas_solliciter_invisibles_sans_le_droit_sans_indice() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (dir, secretaire) = (&c.direction, &c.administratif);
    for p in [dir, secretaire] {
        remettre_a_zero(&ent, p.user_id).await;
    }
    let (visible, protegee, reservee) = (nom_unique(), nom_unique(), nom_unique());
    for (nom, _) in [(&visible, 0), (&protegee, 1), (&reservee, 2)] {
        sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom, source) VALUES ($1, $2, $3, 'Test', 'saisie')")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(nom)
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
    for (nom, adresse) in [(&protegee, true), (&reservee, false)] {
        let (p,): (Uuid,) =
            sqlx::query_as("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND nom = $2")
                .bind(etab)
                .bind(nom)
                .fetch_one(ent.pool_verif())
                .await
                .unwrap();
        if adresse {
            sqlx::query("INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine) VALUES ($1, $2, $3, 'enc:v1:x', 'saisie')")
                .bind(Uuid::new_v4()).bind(etab).bind(p).execute(ent.pool_verif()).await.unwrap();
        } else {
            sqlx::query("INSERT INTO ent_personne_reserve (id, etablissement_id, personne_id, ne_pas_solliciter) VALUES (gen_random_uuid(), $1, $2, true)")
                .bind(etab).bind(p).execute(ent.pool_verif()).await.unwrap();
        }
    }
    let trouve = |r: &ReponseRecherche, nom: &str| {
        r.resultats.iter().any(|x| {
            x.source == SourceRecherche::Personnes && x.libelle.to_lowercase().contains(nom)
        })
    };
    // Témoin : la direction (qui lit les protections) retrouve les trois.
    let r = chercher(&ent, &c, dir, &visible).await.unwrap();
    assert!(
        trouve(&r, &visible),
        "témoin : personne ordinaire, direction"
    );
    assert!(
        trouve(
            &chercher(&ent, &c, dir, &protegee).await.unwrap(),
            &protegee
        ),
        "porteur du droit"
    );
    assert!(
        trouve(
            &chercher(&ent, &c, dir, &reservee).await.unwrap(),
            &reservee
        ),
        "porteur du droit"
    );
    // Le secrétariat (personnes.gerer, mais pas les protections) retrouve l'ordinaire, et ni résultat ni indice pour les autres.
    let rs = chercher(&ent, &c, secretaire, &visible).await.unwrap();
    assert!(
        trouve(&rs, &visible),
        "témoin : le secrétariat cherche bien les personnes"
    );
    for nom in [&protegee, &reservee] {
        let r = chercher(&ent, &c, secretaire, nom).await.unwrap();
        assert!(
            r.resultats.is_empty(),
            "aucun résultat ni indice pour {nom} : {r:?}"
        );
        assert_eq!(
            format!("{r:?}"),
            format!(
                "{:?}",
                chercher(&ent, &c, secretaire, &nom_unique()).await.unwrap()
            ),
            "même réponse qu'une recherche sans résultat (pas d'oracle)"
        );
    }
}

#[tokio::test]
async fn p5_anti_enumeration_ralentissement_blocage_et_deblocages() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (prof, admin) = (&c.profs[2], &c.admin);
    let base = prof.role.as_str().to_string();
    assert_eq!(base, "prof");
    remettre_a_zero(&ent, admin.user_id).await;

    // Libre au début ; ralenti à 75 % du plafond ; bloqué au-delà.
    preparer_compteur(&ent, etab, prof.user_id, 10, 0).await;
    assert!(
        !chercher(&ent, &c, prof, "potter").await.unwrap().ralenti,
        "libre"
    );
    preparer_compteur(&ent, etab, prof.user_id, PLAFOND_PERSONNEL * 3 / 4 - 1, 0).await;
    assert!(
        chercher(&ent, &c, prof, "potter").await.unwrap().ralenti,
        "ralenti à 75 %"
    );
    preparer_compteur(&ent, etab, prof.user_id, PLAFOND_PERSONNEL, 0).await;
    let r = chercher(&ent, &c, prof, "potter").await;
    assert!(bloquee(&r), "plafond dépassé : {r:?}");
    // Toujours bloqué ensuite, même texte ; une tâche de sécurité est ouverte, une seule.
    assert!(bloquee(&chercher(&ent, &c, prof, "granger").await));
    assert_eq!(
        taches_blocage(&ent, etab, prof.user_id).await,
        vec!["ouverte".to_string()]
    );
    // L'administrateur voit le blocage (nom court, nombres : aucun texte) ; le professeur ne le lève pas.
    let liste = ent
        .rechercher_blocages(etab, admin.user_id, "admin", t0())
        .await
        .unwrap();
    assert!(
        liste.iter().any(|b| b.compte_id == prof.user_id),
        "{liste:?}"
    );
    assert!(matches!(
        ent.rechercher_blocages(etab, prof.user_id, "prof", t0())
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.rechercher_debloquer(etab, prof.user_id, "prof", prof.user_id, t0())
            .await,
        Err(EntDbError::Forbidden)
    ));
    // L'administrateur lève : la recherche reprend, la tâche est close.
    ent.rechercher_debloquer(etab, admin.user_id, "admin", prof.user_id, t0())
        .await
        .unwrap();
    assert!(chercher(&ent, &c, prof, "potter").await.is_ok());
    assert_eq!(
        taches_blocage(&ent, etab, prof.user_id).await,
        vec!["faite".to_string()]
    );
    // Lever un compte qui n'est pas bloqué : introuvable (pas d'oracle).
    assert!(matches!(
        ent.rechercher_debloquer(etab, admin.user_id, "admin", prof.user_id, t0())
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));

    // Recherche systématique de lettres : bloquée bien avant le plafond.
    preparer_compteur(&ent, etab, prof.user_id, PLAFOND_LETTRES, PLAFOND_LETTRES).await;
    let r = chercher(&ent, &c, prof, "ab").await;
    assert!(bloquee(&r), "lettres : {r:?}");

    // Déblocage personnel (POR-259) : motif de la liste fermée, une seule fois par jour.
    assert!(matches!(
        ent.rechercher_debloquer_soi(etab, prof.user_id, "prof", "n.importe.quoi", t0())
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    ent.rechercher_debloquer_soi(etab, prof.user_id, "prof", "rentree", t0())
        .await
        .unwrap();
    assert!(
        chercher(&ent, &c, prof, "potter").await.is_ok(),
        "débloqué par la personne elle-même"
    );
    sqlx::query("UPDATE ent_recherche_compteur SET nb = $2, nb_courtes = 0 WHERE compte_id = $1 AND jour = '2031-03-12'")
        .bind(prof.user_id).bind(PLAFOND_PERSONNEL).execute(ent.pool_verif()).await.unwrap();
    assert!(
        bloquee(&chercher(&ent, &c, prof, "potter").await),
        "bloquée de nouveau"
    );
    assert!(
        matches!(
            ent.rechercher_debloquer_soi(etab, prof.user_id, "prof", "autre", t0())
                .await,
            Err(EntDbError::Forbidden)
        ),
        "une seule fois par jour : ensuite l'administrateur seul"
    );
    // Une famille n'a ni recherche ni déblocage.
    assert!(matches!(
        ent.rechercher_debloquer_soi(etab, c.parent_lea.user_id, "parent", "autre", t0())
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Le lendemain, un nouveau compteur : plus de blocage.
    let demain = t0() + Duration::days(1);
    assert!(ent
        .rechercher_global(etab, prof.user_id, "prof", "potter", demain)
        .await
        .is_ok());
    remettre_a_zero(&ent, prof.user_id).await;
}

#[tokio::test]
async fn p5_trop_court_ne_compte_pas_et_aucun_texte_n_est_conserve() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let p = &c.cpes[0];
    remettre_a_zero(&ent, p.user_id).await;
    assert!(chercher(&ent, &c, p, "a")
        .await
        .unwrap()
        .resultats
        .is_empty());
    assert!(chercher(&ent, &c, p, "  ")
        .await
        .unwrap()
        .resultats
        .is_empty());
    assert_eq!(
        compteur(&ent, p.user_id).await,
        (0, 0),
        "rien n'est compté sous deux caractères"
    );
    let secret = nom_unique();
    chercher(&ent, &c, p, &secret).await.unwrap();
    assert_eq!(compteur(&ent, p.user_id).await, (1, 0));
    chercher(&ent, &c, p, "po").await.unwrap();
    assert_eq!(
        compteur(&ent, p.user_id).await,
        (2, 1),
        "une recherche de deux caractères est une « lettre »"
    );
    // Le schéma de la table ne contient aucune colonne de texte libre : seul un motif de liste fermée.
    let cols: Vec<(String, String)> = sqlx::query_as(
        "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = 'ent_recherche_compteur' ORDER BY 1",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let textes: Vec<&str> = cols
        .iter()
        .filter(|(_, t)| t == "text")
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(textes, ["motif_deblocage"], "{cols:?}");
    // Et le texte cherché n'existe nulle part dans la ligne.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_recherche_compteur c WHERE c::text LIKE '%' || $1 || '%'",
    )
    .bind(&secret)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 0);
    // Purge à 30 jours : un compteur ancien est retiré, le récent reste.
    sqlx::query("INSERT INTO ent_recherche_compteur (etablissement_id, compte_id, jour, nb) VALUES ($1, $2, '2031-01-01', 5)")
        .bind(etab).bind(p.user_id).execute(ent.pool_verif()).await.unwrap();
    let n = ent.purger_recherche_compteurs(etab, t0()).await.unwrap();
    assert!(n >= 1);
    let (anciens,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_recherche_compteur WHERE compte_id = $1 AND jour = '2031-01-01'",
    )
    .bind(p.user_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(anciens, 0);
    assert_eq!(
        compteur(&ent, p.user_id).await.0,
        2,
        "le compteur du jour reste"
    );
    remettre_a_zero(&ent, p.user_id).await;
}
