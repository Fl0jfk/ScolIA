//! Socle 7 (documents), lot D2 : fondations de la brique, contre la base locale de test (feature `labo`) ; sans
//! `ENT_DATABASE_URL`, les tests sont ignorés en local (échec explicite en CI). Chaque test crée son propre
//! établissement : il ne vérifie que ses propres lignes. Aucun test ne dépend de l'heure ni du jour réels.
//!
//! Critères de fin du lot (§23, ligne D2) et cas limites du §4.2 :
//!
//! | Exigence | Test(s) |
//! |---|---|
//! | version immuable (trigger) | `t_version_emise_immuable`, `t_document_identite_immuable` |
//! | numéros sans trou sous concurrence | `t_numerotation_concurrente_sans_trou`, `t_echec_n_use_pas_de_numero` |
//! | RLS stricte, séparation entre établissements | `t_separation_entre_etablissements`, `tests_rls` (compte des tables) |
//! | D7 adresse protégée (jamais celle d'un autre) | `t_d7_adresse_jamais_celle_d_un_autre` |
//! | D8 deux titulaires, deux exemplaires | `t_d8_deux_titulaires_deux_exemplaires` |
//! | D9 réémission | `t_d9_reemission` |
//! | D10 annulation | `t_d10_annulation` |
//! | droits (matrice, titre neutre, C4) | `t_droits_emission`, `t_registre_titre_neutre`, `t_c4_servi_a_personne` |
//! | paramétrage (contraste AA, textes encadrés) | `t_parametrage_identite_et_natures` |

use super::*;
use crate::documents::*;
use crate::documents_repo::*;
use crate::ref_commun::Acteur;
use crate::service::{EntDbError, RefusMetier};
use crate::tests_demandes::{fiche, labo, monde, Acteur as ActeurTest, Monde};
use sqlx::Row;
use std::sync::Mutex;
use uuid::Uuid;

// ───────────────────────── Doublures des ports ─────────────────────────

/// Moteur de rendu de test : octets déterministes qui contiennent tout ce que le moteur a reçu, pour que les tests
/// vérifient ce qui est imprimé (numéro, destinataire, adresse) sans PDF réel (D1 n'est pas sur ENT).
/// (destinataire, adresse, numéro de version) reçus par le moteur.
type AppelRendu = (Option<String>, Option<String>, i32);

#[derive(Default)]
struct RenduTest {
    appels: Mutex<Vec<AppelRendu>>,
    echec: bool,
}

impl RenduDocument for RenduTest {
    fn rendre(&self, d: &DemandeRendu<'_>) -> Result<DocumentRendu, ErreurRendu> {
        if self.echec {
            return Err(ErreurRendu::Impossible("panne simulée".into()));
        }
        self.appels.lock().unwrap().push((
            d.destinataire_nom.map(str::to_string),
            d.destinataire_adresse.map(str::to_string),
            d.numero_version,
        ));
        let texte = format!(
            "%PDF-1.4 TEST|{}|v{}|{}|{}|{}|{}",
            d.numero_affiche,
            d.numero_version,
            d.code_verification.unwrap_or("-"),
            d.destinataire_nom.unwrap_or("-"),
            d.destinataire_adresse.unwrap_or("-"),
            d.motif.unwrap_or("-")
        );
        Ok(DocumentRendu {
            octets: texte.into_bytes(),
            nb_pages: 1,
        })
    }
}

struct CachetTest;

impl CachetEtablissement for CachetTest {
    fn sceller(&self, _: Uuid, pdf: &[u8]) -> Result<Scelle, ErreurCachet> {
        let mut o = pdf.to_vec();
        o.extend_from_slice(b"|SCELLE");
        Ok(Scelle {
            octets: o,
            cle_ref: "cle-test-1".into(),
        })
    }
}

fn moteur<'a>(r: &'a dyn RenduDocument, c: &'a dyn CachetEtablissement) -> Moteur<'a> {
    Moteur {
        rendu: r,
        cachet: c,
        url_verification: "https://test.scolia.fr/verifier",
    }
}

// ───────────────────────── Le monde de test ─────────────────────────

struct W {
    ent: PgEnt,
    cast: LaboCast,
    m: Monde,
    parent_lea: ActeurTest,
    parent_tom: ActeurTest,
}

impl W {
    async fn nouveau() -> Option<W> {
        let (ent, cast) = labo().await?;
        let m = monde(&ent, &cast).await;
        let parent_lea = fiche(&ent, m.etab, cast.parent_lea.user_id, "parent", false).await;
        let parent_tom = fiche(&ent, m.etab, cast.parent_tom.user_id, "parent", false).await;
        Some(W {
            ent,
            cast,
            m,
            parent_lea,
            parent_tom,
        })
    }

    fn dir(&self) -> Acteur<'static> {
        Acteur {
            etablissement_id: self.m.etab,
            user_id: self.cast.direction.user_id,
            role: "direction",
        }
    }
    fn adm(&self) -> Acteur<'static> {
        Acteur {
            etablissement_id: self.m.etab,
            user_id: self.cast.administratif.user_id,
            role: "administratif",
        }
    }
    fn cpe(&self) -> Acteur<'static> {
        Acteur {
            etablissement_id: self.m.etab,
            user_id: self.cast.cpe.user_id,
            role: "cpe",
        }
    }
    fn prof(&self) -> Acteur<'static> {
        Acteur {
            etablissement_id: self.m.etab,
            user_id: self.cast.prof.user_id,
            role: "prof",
        }
    }
    fn compta(&self) -> Acteur<'static> {
        Acteur {
            etablissement_id: self.m.etab,
            user_id: self.cast.compta.user_id,
            role: "compta",
        }
    }
    fn parent(&self, p: &ActeurTest) -> Acteur<'static> {
        Acteur {
            etablissement_id: self.m.etab,
            user_id: p.user,
            role: "parent",
        }
    }

    fn demande(&self, destinataires: Vec<DestinataireDoc>) -> DemandeEmission {
        DemandeEmission {
            nature_code: "eleve.certificat_scolarite".into(),
            sujet: SujetDoc::Eleve(self.m.eleve),
            titre: "Certificat de scolarité — Harry Potter".into(),
            annee_scolaire_id: None,
            donnees: vec![
                DonneeRendu {
                    cle: "eleve.nom".into(),
                    valeur: "POTTER".into(),
                },
                DonneeRendu {
                    cle: "eleve.prenom".into(),
                    valeur: "Harry".into(),
                },
            ],
            destinataires,
            source: None,
            au_nom_de_agent_id: None,
            langue: "fr".into(),
        }
    }

    fn dest(&self, p: &ActeurTest, nom: &str, adresse: Option<&str>) -> DestinataireDoc {
        DestinataireDoc {
            personne_id: Some(p.personne),
            agent_id: None,
            nom: Some(nom.into()),
            adresse: adresse.map(str::to_string),
            au_coffre: true,
        }
    }

    async fn emettre(
        &self,
        qui: &Acteur<'_>,
        d: DemandeEmission,
    ) -> Result<DocumentEmis, EntDbError> {
        let (r, c) = (RenduTest::default(), CachetTest);
        self.ent
            .emettre_document_for_actor(qui, d, &moteur(&r, &c))
            .await
    }

    async fn un_certificat(&self) -> DocumentEmis {
        self.emettre(
            &self.dir(),
            self.demande(vec![self.dest(&self.parent_lea, "Mme Potter", None)]),
        )
        .await
        .expect("émission")
    }

    async fn compte(&self, sql: &str, id: Uuid) -> i64 {
        let (n,): (i64,) = sqlx::query_as(sql)
            .bind(self.m.etab)
            .bind(id)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        n
    }
}

fn est_refus(e: &EntDbError, attendu: RefusMetier) -> bool {
    matches!(e, EntDbError::Refus(r, _) if *r == attendu)
}

// ───────────────────────── Émission, numéro, code ─────────────────────────

#[tokio::test]
async fn t_emission_numero_code_exemplaire_et_journal() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let e = w.un_certificat().await;
    assert_eq!(e.numero_version, 1);
    assert!(
        e.numero_affiche.ends_with("-SCO-000001"),
        "{}",
        e.numero_affiche
    );
    let code = e.code_verification.clone().expect("code de vérification");
    assert_eq!(code.len(), 12);
    assert_eq!(normaliser_code(&code).as_deref(), Some(code.as_str()));
    assert_eq!(e.exemplaires.len(), 1);
    // Un document émis : numéro complet, version 1, cachet consigné, empreinte de l'objet = celle de la version.
    let r = sqlx::query(
        "SELECT d.etat, d.version_courante, d.verification_publique, v.cachet_cle_ref, v.empreinte_sha256, o.sha256
           FROM ent_doc_document d
           JOIN ent_doc_version v ON v.document_id = d.id AND v.etablissement_id = d.etablissement_id
           JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.cle_objet = v.cle_objet
          WHERE d.etablissement_id = $1 AND d.id = $2",
    )
    .bind(w.m.etab)
    .bind(e.document_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(r.get::<String, _>("etat"), "emis");
    assert_eq!(r.get::<i32, _>("version_courante"), 1);
    assert_eq!(r.get::<String, _>("verification_publique"), "complete");
    assert_eq!(r.get::<String, _>("cachet_cle_ref"), "cle-test-1");
    assert_eq!(
        r.get::<String, _>("empreinte_sha256"),
        r.get::<String, _>("sha256")
    );
    // Les données figées sont scellées (jamais en clair en base).
    let (clair,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_version_donnee WHERE etablissement_id = $1 AND version_id = $2 AND valeur LIKE '%POTTER%'",
    )
    .bind(w.m.etab)
    .bind(e.version_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(clair, 0, "les données personnelles figées sont chiffrées");
    // Journal et événement : dans la même transaction.
    assert_eq!(w.compte("SELECT count(*) FROM ent_doc_journal WHERE etablissement_id = $1 AND document_id = $2 AND action = 'emis'", e.document_id).await, 1);
    let (evt,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'documents.emis'",
    )
    .bind(w.m.etab)
    .bind(e.document_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(evt, 1);
    // L'événement ne porte ni titre ni contenu.
    let (charge,): (String,) = sqlx::query_as(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'documents.emis'",
    )
    .bind(w.m.etab)
    .bind(e.document_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert!(
        !charge.contains("Potter") && !charge.contains("Certificat"),
        "{charge}"
    );
    // Le titulaire lit son exemplaire (ouverture : accusé posé, événement publié une seule fois).
    let lu = w
        .ent
        .lire_document_for_actor(&w.parent(&w.parent_lea), e.document_id, None)
        .await
        .unwrap();
    assert_eq!(lu.exemplaire_rang, Some(1));
    assert!(String::from_utf8_lossy(&lu.octets).contains(&e.numero_affiche));
    w.ent
        .lire_document_for_actor(&w.parent(&w.parent_lea), e.document_id, None)
        .await
        .unwrap();
    assert_eq!(w.compte("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'documents.ouvert'", e.document_id).await, 1);
    assert_eq!(w.compte("SELECT count(*) FROM ent_doc_mise_a_disposition WHERE etablissement_id = $1 AND document_id = $2 AND premiere_ouverture_le IS NOT NULL", e.document_id).await, 1);
}

#[tokio::test]
async fn t_numerotation_concurrente_sans_trou() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let dir = w.dir();
    // 8 émissions réellement simultanées de la même série : chacune sa connexion.
    let un = || {
        w.emettre(
            &dir,
            w.demande(vec![w.dest(&w.parent_lea, "Mme Potter", None)]),
        )
    };
    let (r1, r2, r3, r4, r5, r6, r7, r8) =
        tokio::join!(un(), un(), un(), un(), un(), un(), un(), un());
    let res = vec![r1, r2, r3, r4, r5, r6, r7, r8];
    let mut numeros: Vec<i32> = res
        .into_iter()
        .map(|r| r.expect("émission concurrente").numero_affiche)
        .map(|n| n.rsplit('-').next().unwrap().parse().unwrap())
        .collect();
    numeros.sort_unstable();
    assert_eq!(
        numeros,
        (1..=8).collect::<Vec<_>>(),
        "numéros contigus, sans trou ni doublon"
    );
    // Le compteur de la base dit la même chose.
    let (dernier,): (i32,) = sqlx::query_as(
        "SELECT max(dernier) FROM ent_doc_compteur WHERE etablissement_id = $1 AND serie_code = 'SCO'",
    )
    .bind(w.m.etab)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(dernier, 8);
    // Les codes de vérification sont tous distincts.
    let (distincts,): (i64,) = sqlx::query_as(
        "SELECT count(DISTINCT code_verification) FROM ent_doc_version WHERE etablissement_id = $1",
    )
    .bind(w.m.etab)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(distincts, 8);
}

#[tokio::test]
async fn t_echec_n_use_pas_de_numero() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let dir = w.dir();
    let premier = w.un_certificat().await;
    // Rendu en panne, cachet indisponible, moteur absent : l'émission échoue et ne consomme AUCUN numéro (§9.5).
    let panne = RenduTest {
        echec: true,
        ..Default::default()
    };
    let e = w
        .ent
        .emettre_document_for_actor(
            &dir,
            w.demande(vec![w.dest(&w.parent_lea, "Mme Potter", None)]),
            &moteur(&panne, &CachetTest),
        )
        .await
        .unwrap_err();
    assert!(est_refus(&e, RefusMetier::Conflit), "{e:?}");
    let e = w
        .ent
        .emettre_document_for_actor(
            &dir,
            w.demande(vec![w.dest(&w.parent_lea, "Mme Potter", None)]),
            &moteur(&RenduTest::default(), &CachetIndisponible),
        )
        .await
        .unwrap_err();
    assert!(est_refus(&e, RefusMetier::Conflit), "{e:?}");
    let e = w
        .ent
        .emettre_document_for_actor(
            &dir,
            w.demande(vec![w.dest(&w.parent_lea, "Mme Potter", None)]),
            &moteur(&RenduIndisponible, &CachetTest),
        )
        .await
        .unwrap_err();
    assert!(est_refus(&e, RefusMetier::Conflit), "{e:?}");
    let second = w.un_certificat().await;
    let n = |x: &DocumentEmis| {
        x.numero_affiche
            .rsplit('-')
            .next()
            .unwrap()
            .parse::<i32>()
            .unwrap()
    };
    assert_eq!(
        n(&second),
        n(&premier) + 1,
        "aucun numéro perdu par les trois échecs"
    );
    // Rien n'est resté des tentatives : un seul document de plus.
    let (nb,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM ent_doc_document WHERE etablissement_id = $1")
            .bind(w.m.etab)
            .fetch_one(w.ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(nb, 2);
}

// ───────────────────────── Immuabilité ─────────────────────────

#[tokio::test]
async fn t_version_emise_immuable() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let e = w.un_certificat().await;
    let pool = w.ent.pool_verif();
    for sql in [
        "UPDATE ent_doc_version SET motif = 'modifié' WHERE id = $1",
        "UPDATE ent_doc_version SET empreinte_sha256 = repeat('a', 64) WHERE id = $1",
        "DELETE FROM ent_doc_version WHERE id = $1",
        "UPDATE ent_doc_version_donnee SET valeur = 'x' WHERE version_id = $1",
        "DELETE FROM ent_doc_version_donnee WHERE version_id = $1",
        "UPDATE ent_doc_exemplaire SET rang = 9 WHERE version_id = $1",
        "DELETE FROM ent_doc_exemplaire WHERE version_id = $1",
    ] {
        let r = sqlx::query(sql).bind(e.version_id).execute(pool).await;
        assert!(r.is_err(), "doit être refusé : {sql}");
    }
    // Même sous le rôle propriétaire : un écart d'empreinte entre la version et l'objet est refusé à l'insertion.
    let r = sqlx::query(
        "INSERT INTO ent_doc_version (id, etablissement_id, document_id, numero_version, cle_objet, empreinte_sha256, nb_pages, motif, emise_par)
         SELECT $1, etablissement_id, document_id, 2, cle_objet, repeat('b', 64), 1, 'faux', emise_par
           FROM ent_doc_version WHERE id = $2",
    )
    .bind(Uuid::new_v4())
    .bind(e.version_id)
    .execute(pool)
    .await;
    assert!(
        r.is_err(),
        "l'empreinte de la version doit être celle de l'objet"
    );
    // Les données et le contenu n'ont pas bougé.
    assert_eq!(
        w.compte(
            "SELECT count(*) FROM ent_doc_version WHERE etablissement_id = $1 AND id = $2",
            e.version_id
        )
        .await,
        1
    );
}

#[tokio::test]
async fn t_document_identite_immuable() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let e = w.un_certificat().await;
    let pool = w.ent.pool_verif();
    for sql in [
        "UPDATE ent_doc_document SET numero = numero + 1 WHERE id = $1",
        "UPDATE ent_doc_version SET code_verification = 'ABCDEFGHJKLM' WHERE document_id = $1",
        "UPDATE ent_doc_document SET titre = 'autre titre' WHERE id = $1",
        "UPDATE ent_doc_document SET nature_code = 'eleve.autre' WHERE id = $1",
        "UPDATE ent_doc_document SET sensibilite = 'C0' WHERE id = $1",
        "UPDATE ent_doc_document SET version_courante = 3 WHERE id = $1",
        "UPDATE ent_doc_document SET version_courante = 0 WHERE id = $1",
        "UPDATE ent_doc_document SET etat = 'brouillon' WHERE id = $1",
        "UPDATE ent_doc_document SET etat = 'elimine' WHERE id = $1",
        "DELETE FROM ent_doc_document WHERE id = $1",
    ] {
        let r = sqlx::query(sql).bind(e.document_id).execute(pool).await;
        assert!(r.is_err(), "doit être refusé : {sql}");
    }
    // Journal, consultations et compteurs : ajout seul.
    assert!(
        sqlx::query("DELETE FROM ent_doc_journal WHERE document_id = $1")
            .bind(e.document_id)
            .execute(pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE ent_doc_journal SET motif = 'x' WHERE document_id = $1")
            .bind(e.document_id)
            .execute(pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE ent_doc_compteur SET dernier = 0 WHERE etablissement_id = $1")
            .bind(w.m.etab)
            .execute(pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM ent_doc_compteur WHERE etablissement_id = $1")
            .bind(w.m.etab)
            .execute(pool)
            .await
            .is_err()
    );
    // Une mise à disposition ne se supprime pas : elle se retire (une seule fois).
    assert!(
        sqlx::query("DELETE FROM ent_doc_mise_a_disposition WHERE document_id = $1")
            .bind(e.document_id)
            .execute(pool)
            .await
            .is_err()
    );
    assert!(sqlx::query(
        "UPDATE ent_doc_mise_a_disposition SET canal = 'papier' WHERE document_id = $1"
    )
    .bind(e.document_id)
    .execute(pool)
    .await
    .is_err());
}

// ───────────────────────── D7 : adresse ─────────────────────────

#[tokio::test]
async fn t_d7_adresse_jamais_celle_d_un_autre() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let r = RenduTest::default();
    // Deux titulaires séparés, deux adresses : chaque exemplaire ne reçoit QUE la sienne ; la version de base, aucune.
    let e = w
        .ent
        .emettre_document_for_actor(
            &w.dir(),
            w.demande(vec![
                w.dest(&w.parent_lea, "Mme Potter", Some("4 Privet Drive")),
                w.dest(&w.parent_tom, "M. Potter", Some("12 Grimmauld Place")),
            ]),
            &moteur(&r, &CachetTest),
        )
        .await
        .unwrap();
    let appels = r.appels.lock().unwrap().clone();
    assert_eq!(
        appels.len(),
        3,
        "version de base + un rendu par destinataire"
    );
    assert_eq!(
        appels[0],
        (None, None, 1),
        "la version de base ne porte aucune adresse"
    );
    assert_eq!(
        appels[1],
        (Some("Mme Potter".into()), Some("4 Privet Drive".into()), 1)
    );
    assert_eq!(
        appels[2],
        (
            Some("M. Potter".into()),
            Some("12 Grimmauld Place".into()),
            1
        )
    );
    // La base garantit la même règle : l'adresse imprimée est celle du destinataire, jamais d'une autre personne.
    let lignes: Vec<(i32, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
        "SELECT rang, destinataire_personne_id, adresse_de_personne_id FROM ent_doc_exemplaire WHERE etablissement_id = $1 AND version_id = $2 ORDER BY rang",
    )
    .bind(w.m.etab)
    .bind(e.version_id)
    .fetch_all(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(lignes.len(), 2);
    for (_, dest, adresse) in &lignes {
        assert_eq!(dest, adresse);
    }
    // Insertion directe d'un exemplaire portant l'adresse d'un AUTRE : refusée par la contrainte (D7, ADM-25).
    let violation = sqlx::query(
        "INSERT INTO ent_doc_exemplaire (id, etablissement_id, version_id, rang, destinataire_personne_id, adresse_de_personne_id, cle_objet, empreinte_sha256)
         SELECT $1, etablissement_id, version_id, 9, destinataire_personne_id, $3, cle_objet, empreinte_sha256
           FROM ent_doc_exemplaire WHERE etablissement_id = $2 AND version_id = $4 AND rang = 1",
    )
    .bind(Uuid::new_v4())
    .bind(w.m.etab)
    .bind(w.parent_tom.personne)
    .bind(e.version_id)
    .execute(w.ent.pool_verif())
    .await
    .unwrap_err();
    assert!(
        violation
            .to_string()
            .contains("ent_doc_exemplaire_adresse_chk"),
        "{violation}"
    );
}

// ───────────────────────── D8 : parents séparés ─────────────────────────

#[tokio::test]
async fn t_d8_deux_titulaires_deux_exemplaires() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Lot D4 : le coffre relit le droit du jour. Les deux destinataires de ce test sont titulaires de l'autorité
    // parentale sur l'élève (le jeu de labo ne lie M. Potter qu'à l'autre élève).
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale)
         SELECT $1, $2, $3, $4, $5, 'parent', true
          WHERE NOT EXISTS (SELECT 1 FROM ent_eleve_lien WHERE etablissement_id = $2 AND eleve_id = $3 AND personne_id = $5)",
    )
    .bind(Uuid::new_v4())
    .bind(w.m.etab)
    .bind(w.m.eleve)
    .bind(w.parent_tom.user)
    .bind(w.parent_tom.personne)
    .execute(w.ent.pool_verif())
    .await
    .unwrap();
    let e = w
        .emettre(
            &w.dir(),
            w.demande(vec![
                w.dest(&w.parent_lea, "Mme Potter", None),
                w.dest(&w.parent_tom, "M. Potter", None),
            ]),
        )
        .await
        .unwrap();
    assert_eq!(e.exemplaires.len(), 2);
    // Même document, même numéro ; deux exemplaires de rangs distincts, deux coffres.
    assert_eq!(
        w.compte(
            "SELECT count(*) FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2",
            e.document_id
        )
        .await,
        1
    );
    assert_eq!(w.compte("SELECT count(*) FROM ent_doc_mise_a_disposition WHERE etablissement_id = $1 AND document_id = $2", e.document_id).await, 2);
    let a = w
        .ent
        .lire_document_for_actor(&w.parent(&w.parent_lea), e.document_id, None)
        .await
        .unwrap();
    let b = w
        .ent
        .lire_document_for_actor(&w.parent(&w.parent_tom), e.document_id, None)
        .await
        .unwrap();
    assert_eq!((a.exemplaire_rang, b.exemplaire_rang), (Some(1), Some(2)));
    assert_ne!(a.octets, b.octets, "chacun lit son exemplaire nominatif");
    assert!(String::from_utf8_lossy(&a.octets).contains("Mme Potter"));
    assert!(!String::from_utf8_lossy(&a.octets).contains("M. Potter"));
    assert!(String::from_utf8_lossy(&b.octets).contains("M. Potter"));
    // Un parent sans exemplaire (ni droit documentaire) ne lit rien.
    let autre = fiche(
        &w.ent,
        w.m.etab,
        w.cast
            .parent_weasley
            .as_ref()
            .map(|p| p.user_id)
            .unwrap_or_else(Uuid::new_v4),
        "parent",
        false,
    )
    .await;
    let refus = w
        .ent
        .lire_document_for_actor(&w.parent(&autre), e.document_id, None)
        .await
        .unwrap_err();
    assert!(matches!(refus, EntDbError::Forbidden), "{refus:?}");
}

// ───────────────────────── D9 : réémission ─────────────────────────

#[tokio::test]
async fn t_d9_reemission() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let e = w.un_certificat().await;
    let (r, c) = (RenduTest::default(), CachetTest);
    let m = moteur(&r, &c);
    let demande = |motif: &str, w: &W| DemandeReemission {
        document_id: e.document_id,
        motif: motif.into(),
        donnees: vec![DonneeRendu {
            cle: "eleve.classe".into(),
            valeur: "4B".into(),
        }],
        destinataires: vec![w.dest(&w.parent_lea, "Mme Potter", None)],
        au_nom_de_agent_id: None,
    };
    // Motif obligatoire ; droit exigé (un professeur n'émet rien).
    assert!(w
        .ent
        .reemettre_document_for_actor(&w.dir(), demande("  ", &w), &m)
        .await
        .is_err());
    assert!(matches!(
        w.ent
            .reemettre_document_for_actor(&w.prof(), demande("Classe corrigée", &w), &m)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    let v2 = w
        .ent
        .reemettre_document_for_actor(&w.dir(), demande("Classe corrigée : 4B et non 5A", &w), &m)
        .await
        .unwrap();
    // Même numéro ; un NOUVEAU code de vérification (une version, un code : celui de la version 1 répondra « remplacé »).
    assert_eq!(v2.numero_version, 2);
    assert_eq!(v2.numero_affiche, e.numero_affiche);
    assert!(v2.code_verification.is_some() && v2.code_verification != e.code_verification);
    let (courante, etat): (i32, String) = sqlx::query_as("SELECT version_courante, etat FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2")
        .bind(w.m.etab)
        .bind(e.document_id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!((courante, etat.as_str()), (2, "emis"));
    // Le coffre montre la nouvelle version ; l'ancienne mise à disposition est retirée avec son motif.
    let lu = w
        .ent
        .lire_document_for_actor(&w.parent(&w.parent_lea), e.document_id, None)
        .await
        .unwrap();
    assert_eq!(lu.numero_version, 2);
    assert!(String::from_utf8_lossy(&lu.octets).contains("Classe corrigée"));
    let (retirees,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_mise_a_disposition WHERE etablissement_id = $1 AND document_id = $2 AND retiree_le IS NOT NULL AND retrait_motif LIKE 'remplacé par la version 2'",
    )
    .bind(w.m.etab)
    .bind(e.document_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(retirees, 1);
    // La version 1 reste lisible par le personnel habilité (registre, preuve) ; elle n'est jamais réécrite.
    let v1 = w
        .ent
        .lire_document_for_actor(&w.dir(), e.document_id, Some(1))
        .await
        .unwrap();
    assert_eq!(v1.numero_version, 1);
    assert_eq!(v1.version_courante, 2);
    assert_ne!(v1.octets, lu.octets);
    assert_eq!(
        w.compte(
            "SELECT count(*) FROM ent_doc_version WHERE etablissement_id = $1 AND document_id = $2",
            e.document_id
        )
        .await,
        2
    );
    assert_eq!(w.compte("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'documents.remplace'", e.document_id).await, 1);
    // Une réémission d'un document annulé est refusée.
    w.ent
        .annuler_document_for_actor(&w.dir(), e.document_id, "Mauvais élève")
        .await
        .unwrap();
    let r = w
        .ent
        .reemettre_document_for_actor(&w.dir(), demande("Encore une correction", &w), &m)
        .await
        .unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
}

// ───────────────────────── D10 : annulation ─────────────────────────

#[tokio::test]
async fn t_d10_annulation() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let e = w.un_certificat().await;
    // Droit : ni le professeur ni la comptabilité n'annulent ; motif obligatoire.
    for qui in [w.prof(), w.compta(), w.cpe()] {
        assert!(matches!(
            w.ent
                .annuler_document_for_actor(&qui, e.document_id, "Erreur")
                .await
                .unwrap_err(),
            EntDbError::Forbidden
        ));
    }
    assert!(w
        .ent
        .annuler_document_for_actor(&w.dir(), e.document_id, "")
        .await
        .is_err());
    w.ent
        .annuler_document_for_actor(&w.adm(), e.document_id, "Émis pour le mauvais élève")
        .await
        .unwrap();
    // État, motif, auteur ; mise à disposition retirée ; journal ; événement ; AUCUN message automatique.
    let (etat, motif, par): (String, Option<String>, Option<Uuid>) =
        sqlx::query_as("SELECT etat, annule_motif, annule_par FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2")
            .bind(w.m.etab)
            .bind(e.document_id)
            .fetch_one(w.ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(etat, "annule");
    assert_eq!(motif.as_deref(), Some("Émis pour le mauvais élève"));
    assert_eq!(par, Some(w.cast.administratif.user_id));
    assert_eq!(w.compte("SELECT count(*) FROM ent_doc_mise_a_disposition WHERE etablissement_id = $1 AND document_id = $2 AND retiree_le IS NULL", e.document_id).await, 0);
    assert_eq!(w.compte("SELECT count(*) FROM ent_doc_journal WHERE etablissement_id = $1 AND document_id = $2 AND action = 'annule'", e.document_id).await, 1);
    assert_eq!(w.compte("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'documents.annule'", e.document_id).await, 1);
    // Le socle 7 n'envoie rien : seuls des faits `documents.*` sont publiés pour ce document (le socle 8 propose un
    // message humain à partir de `documents.annule`).
    assert_eq!(
        w.compte("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2 AND type NOT LIKE 'documents.%'", e.document_id).await,
        0
    );
    // Le destinataire ne le voit plus ; le personnel le voit annulé ; une seconde annulation est un conflit.
    let r = w
        .ent
        .lire_document_for_actor(&w.parent(&w.parent_lea), e.document_id, None)
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    let vu = w
        .ent
        .lire_document_for_actor(&w.dir(), e.document_id, None)
        .await
        .unwrap();
    assert_eq!(vu.etat, "annule");
    let r = w
        .ent
        .annuler_document_for_actor(&w.dir(), e.document_id, "Encore")
        .await
        .unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
}

// ───────────────────────── Droits ─────────────────────────

#[tokio::test]
async fn t_droits_emission() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let d = || w.demande(vec![w.dest(&w.parent_lea, "Mme Potter", None)]);
    // Personnel autorisé : direction, secrétariat, CPE, comptabilité.
    for qui in [w.dir(), w.adm(), w.cpe(), w.compta()] {
        w.emettre(&qui, d())
            .await
            .unwrap_or_else(|e| panic!("{} doit pouvoir émettre : {e:?}", qui.role));
    }
    // Ni les professeurs, ni les familles : aucun document officiel.
    for qui in [w.prof(), w.parent(&w.parent_lea)] {
        assert!(
            matches!(
                w.ent
                    .emettre_document_for_actor(
                        &qui,
                        d(),
                        &moteur(&RenduTest::default(), &CachetTest)
                    )
                    .await
                    .unwrap_err(),
                EntDbError::Forbidden
            ),
            "{}",
            qui.role
        );
    }
    // Nature inconnue, sujet d'un autre établissement, aucun destinataire pour une nature nominative.
    let mut inconnue = d();
    inconnue.nature_code = "eleve.inconnue".into();
    assert!(est_refus(
        &w.emettre(&w.dir(), inconnue).await.unwrap_err(),
        RefusMetier::Invalide
    ));
    let mut autre = d();
    autre.sujet = SujetDoc::Eleve(Uuid::new_v4());
    assert!(matches!(
        w.emettre(&w.dir(), autre).await.unwrap_err(),
        EntDbError::Domain(_)
    ));
    assert!(est_refus(
        &w.emettre(&w.dir(), w.demande(vec![])).await.unwrap_err(),
        RefusMetier::Conflit
    ));
    // Nature désactivée par l'établissement : refusée.
    w.ent
        .parametrer_nature_for_actor(&w.dir(), "eleve.certificat_scolarite", false, None, &[])
        .await
        .unwrap();
    assert!(est_refus(
        &w.emettre(&w.dir(), d()).await.unwrap_err(),
        RefusMetier::Conflit
    ));
}

#[tokio::test]
async fn t_registre_titre_neutre() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let e = w.un_certificat().await;
    // Un document du domaine « justice » (C3) déposé au dossier : l'administratif lit le registre mais n'a pas
    // `documents.restreints.lire` ; la direction l'a.
    let secret = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_doc_document (id, etablissement_id, nature_code, entite_id, sujet_type, sujet_id, eleve_id, origine, sensibilite, domaine,
                                       titre, titre_neutre, etat, verification_publique, categorie_conservation)
         SELECT $1, etablissement_id, 'eleve.jugement', entite_id, 'eleve', $3, $3, 'depose', 'C3', 'justice',
                'Jugement de garde — famille Potter', 'Document au dossier', 'depose', 'aucune', 'dossier_justice'
           FROM ent_doc_document WHERE etablissement_id = $2 AND id = $4",
    )
    .bind(secret)
    .bind(w.m.etab)
    .bind(w.m.eleve)
    .bind(e.document_id)
    .execute(w.ent.pool_verif())
    .await
    .unwrap();
    let vue_adm = w
        .ent
        .registre_for_actor(&w.adm(), FiltreRegistre::default())
        .await
        .unwrap();
    let ligne = vue_adm
        .iter()
        .find(|l| l.document_id == secret)
        .expect("présent au registre");
    assert!(ligne.titre_neutre_servi);
    assert_eq!(ligne.titre, "Document au dossier");
    assert!(!ligne.titre.contains("Jugement"));
    assert!(
        !vue_adm
            .iter()
            .find(|l| l.document_id == e.document_id)
            .unwrap()
            .titre_neutre_servi
    );
    let vue_dir = w
        .ent
        .registre_for_actor(&w.dir(), FiltreRegistre::default())
        .await
        .unwrap();
    let ligne = vue_dir.iter().find(|l| l.document_id == secret).unwrap();
    assert!(!ligne.titre_neutre_servi);
    assert!(ligne.titre.contains("Jugement"));
    // Sans `documents.registre.lire` : refus (CPE, professeur, parent).
    for qui in [w.cpe(), w.prof(), w.parent(&w.parent_lea)] {
        assert!(
            matches!(
                w.ent
                    .registre_for_actor(&qui, FiltreRegistre::default())
                    .await
                    .unwrap_err(),
                EntDbError::Forbidden
            ),
            "{}",
            qui.role
        );
    }
    // La lecture du contenu suit le domaine : l'administratif ne lit pas le jugement.
    assert!(matches!(
        w.ent
            .lire_document_for_actor(&w.adm(), secret, None)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    // Filtres.
    let filtre = FiltreRegistre {
        nature_code: Some("eleve.certificat_scolarite".into()),
        ..Default::default()
    };
    assert_eq!(
        w.ent
            .registre_for_actor(&w.dir(), filtre)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn t_c4_servi_a_personne() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let e = w.un_certificat().await;
    // Un document devenu C4 (ex. : pièce d'un changement de prénom) n'est servi par aucun écran ordinaire.
    let c4 = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_doc_document (id, etablissement_id, nature_code, entite_id, sujet_type, sujet_id, eleve_id, origine, sensibilite, domaine,
                                       titre, titre_neutre, etat, verification_publique, categorie_conservation)
         SELECT $1, etablissement_id, 'eleve.piece_prenom', entite_id, 'eleve', $3, $3, 'depose', 'C4', 'scolarite',
                'Pièce du changement de prénom', 'Document au dossier', 'depose', 'aucune', 'dossier_identite'
           FROM ent_doc_document WHERE etablissement_id = $2 AND id = $4",
    )
    .bind(c4)
    .bind(w.m.etab)
    .bind(w.m.eleve)
    .bind(e.document_id)
    .execute(w.ent.pool_verif())
    .await
    .unwrap();
    assert!(matches!(
        w.ent
            .lire_document_for_actor(&w.dir(), c4, None)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    let ligne = w
        .ent
        .registre_for_actor(&w.dir(), FiltreRegistre::default())
        .await
        .unwrap()
        .into_iter()
        .find(|l| l.document_id == c4)
        .unwrap();
    assert!(
        ligne.titre_neutre_servi,
        "C4 : titre neutre même pour la direction"
    );
}

#[tokio::test]
async fn t_consultations_c2_tracees() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let e = w.un_certificat().await;
    // C1 : la lecture n'est pas tracée (volume) ; elle l'est dès C2.
    w.ent
        .lire_document_for_actor(&w.dir(), e.document_id, None)
        .await
        .unwrap();
    assert_eq!(w.compte("SELECT count(*) FROM ent_doc_consultation WHERE etablissement_id = $1 AND document_id = $2", e.document_id).await, 0);
    sqlx::query("ALTER TABLE ent_doc_document DISABLE TRIGGER ent_doc_document_garde_trg")
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_doc_document SET sensibilite = 'C2' WHERE id = $1")
        .bind(e.document_id)
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("ALTER TABLE ent_doc_document ENABLE TRIGGER ent_doc_document_garde_trg")
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    w.ent
        .lire_document_for_actor(&w.dir(), e.document_id, None)
        .await
        .unwrap();
    assert_eq!(w.compte("SELECT count(*) FROM ent_doc_consultation WHERE etablissement_id = $1 AND document_id = $2 AND motif = 'lecture'", e.document_id).await, 1);
}

// ───────────────────────── Séparation entre établissements ─────────────────────────

#[tokio::test]
async fn t_separation_entre_etablissements() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let cast_b = crate::tests_rls_stricte::seed_supplementaire(&w.ent).await;
    let mb = monde(&w.ent, &cast_b).await;
    let e = w.un_certificat().await;
    let dir_b = Acteur {
        etablissement_id: mb.etab,
        user_id: cast_b.direction.user_id,
        role: "direction",
    };
    // Le compte de B ne lit, n'annule ni ne réémet rien de A : 404 neutre (même identifiant, autre établissement).
    assert!(matches!(
        w.ent
            .lire_document_for_actor(&dir_b, e.document_id, None)
            .await
            .unwrap_err(),
        EntDbError::Domain(_)
    ));
    assert!(matches!(
        w.ent
            .annuler_document_for_actor(&dir_b, e.document_id, "Tentative")
            .await
            .unwrap_err(),
        EntDbError::Domain(_)
    ));
    // Le compte de A (même rôle) n'a rien chez B non plus : on lui oppose le périmètre de B.
    let dir_a_chez_b = Acteur {
        etablissement_id: mb.etab,
        ..w.dir()
    };
    assert!(w
        .ent
        .lire_document_for_actor(&dir_a_chez_b, e.document_id, None)
        .await
        .is_err());
    // Le registre de B est vide ; celui de A contient le document.
    assert!(w
        .ent
        .registre_for_actor(&dir_b, FiltreRegistre::default())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        w.ent
            .registre_for_actor(&w.dir(), FiltreRegistre::default())
            .await
            .unwrap()
            .len(),
        1
    );
    // Chaque établissement a ses compteurs, entités et numéros : B démarre à 1 sans avoir vu les numéros de A.
    let (r, c) = (RenduTest::default(), CachetTest);
    let sujet_b = SujetDoc::Eleve(cast_b.eleve_lea_id);
    let parent_b = fiche(&w.ent, mb.etab, cast_b.parent_lea.user_id, "parent", false).await;
    let eb = w
        .ent
        .emettre_document_for_actor(
            &dir_b,
            DemandeEmission {
                sujet: sujet_b,
                destinataires: vec![w.dest(&parent_b, "Parent B", None)],
                ..w.demande(vec![])
            },
            &moteur(&r, &c),
        )
        .await
        .unwrap();
    assert!(
        eb.numero_affiche.ends_with("-SCO-000001"),
        "{}",
        eb.numero_affiche
    );
    // Un sujet de B dans une demande de A : refus (clé composite, jamais de fuite).
    let croise = DemandeEmission {
        sujet: sujet_b,
        ..w.demande(vec![w.dest(&w.parent_lea, "Mme Potter", None)])
    };
    assert!(w.emettre(&w.dir(), croise).await.is_err());
    // RLS stricte, PROUVÉE avec un rôle qui ne contourne pas la RLS (la connexion de test habituelle est propriétaire,
    // voire superutilisatrice : elle voit tout, quel que soit le mode — ce serait une fausse alerte, ou un faux vert).
    const TABLES: [&str; 14] = [
        "ent_doc_document",
        "ent_doc_version",
        "ent_doc_version_donnee",
        "ent_doc_exemplaire",
        "ent_doc_mise_a_disposition",
        "ent_doc_consultation",
        "ent_doc_journal",
        "ent_doc_compteur",
        "ent_doc_entite",
        "ent_doc_entite_identifiant",
        "ent_doc_entite_famille",
        "ent_doc_identite",
        "ent_doc_nature_param",
        "ent_doc_nature_texte",
    ];
    let admin = w.ent.pool_verif();
    sqlx::query(
        "DO $$ BEGIN CREATE ROLE ent_rls_doc LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; \
         EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL; END $$",
    )
    .execute(admin)
    .await
    .unwrap();
    sqlx::query("GRANT USAGE ON SCHEMA public TO ent_rls_doc")
        .execute(admin)
        .await
        .unwrap();
    for table in TABLES {
        sqlx::query(&format!("GRANT SELECT ON {table} TO ent_rls_doc"))
            .execute(admin)
            .await
            .unwrap();
    }
    let url = crate::url_guard::url_labo_ou_echec().expect("URL de labo");
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username("ent_rls_doc").unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(u.as_str())
        .await
        .unwrap();
    let (bypass, sup): (bool, bool) =
        sqlx::query_as("SELECT rolbypassrls, rolsuper FROM pg_roles WHERE rolname = 'ent_rls_doc'")
            .fetch_one(admin)
            .await
            .unwrap();
    assert!(!bypass && !sup, "le rôle de preuve ne contourne pas la RLS");
    // Depuis B : aucune ligne de A, sur CHAQUE table ; depuis A : ses lignes sont bien là (la politique ne cache pas tout).
    let compter = |etab_session: Uuid, etab_cible: Uuid, table: &'static str| {
        let app = app.clone();
        async move {
            let mut tx =
                TenantTx::begin_mode(&app, etab_session, crate::tenant_tx::RlsMode::Strict)
                    .await
                    .unwrap();
            let (n,): (i64,) = sqlx::query_as(&format!(
                "SELECT count(*) FROM {table} WHERE etablissement_id = $1"
            ))
            .bind(etab_cible)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
            n
        }
    };
    for table in TABLES {
        assert_eq!(
            compter(mb.etab, w.m.etab, table).await,
            0,
            "{table} : lignes de A visibles depuis B"
        );
    }
    for table in [
        "ent_doc_document",
        "ent_doc_version",
        "ent_doc_exemplaire",
        "ent_doc_journal",
        "ent_doc_compteur",
        "ent_doc_entite",
    ] {
        assert!(
            compter(w.m.etab, w.m.etab, table).await >= 1,
            "{table} : A ne voit plus ses propres lignes"
        );
    }
    // Sans contexte d'établissement en mode strict : rien du tout (échec fermé).
    let mut tx = TenantTx::begin_mode(&app, Uuid::nil(), crate::tenant_tx::RlsMode::Strict)
        .await
        .unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_doc_document")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(n, 0, "établissement nul : aucune ligne");
}

// ───────────────────────── Paramétrage ─────────────────────────

#[tokio::test]
async fn t_parametrage_identite_et_natures() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Droit : direction et administrateur ; pas le secrétariat ni le CPE.
    for qui in [w.adm(), w.cpe(), w.prof()] {
        assert!(
            matches!(
                w.ent
                    .parametrer_identite_for_actor(&qui, "epuree", None, None, "gauche")
                    .await
                    .unwrap_err(),
                EntDbError::Forbidden
            ),
            "{}",
            qui.role
        );
    }
    // Contraste AA : le jaune sur blanc est refusé, le bleu foncé accepté ; style inconnu refusé.
    assert!(est_refus(
        &w.ent
            .parametrer_identite_for_actor(&w.dir(), "epuree", Some("#FFFF00"), None, "gauche")
            .await
            .unwrap_err(),
        RefusMetier::Invalide
    ));
    assert!(est_refus(
        &w.ent
            .parametrer_identite_for_actor(&w.dir(), "baroque", None, None, "gauche")
            .await
            .unwrap_err(),
        RefusMetier::Invalide
    ));
    let i = w
        .ent
        .parametrer_identite_for_actor(
            &w.dir(),
            "contemporaine",
            Some("#1f3a93"),
            Some("UAI 0750001A"),
            "droite",
        )
        .await
        .unwrap();
    assert_eq!(i.couleur_principale.as_deref(), Some("#1F3A93"));
    let lue = w.ent.identite_documents(w.m.etab).await.unwrap();
    assert_eq!(
        (lue.style_code.as_str(), lue.fenetre_enveloppe.as_str()),
        ("contemporaine", "droite")
    );
    let (version,): (i32,) =
        sqlx::query_as("SELECT version FROM ent_doc_identite WHERE etablissement_id = $1")
            .bind(w.m.etab)
            .fetch_one(w.ent.pool_verif())
            .await
            .unwrap();
    w.ent
        .parametrer_identite_for_actor(&w.dir(), "epuree", None, None, "gauche")
        .await
        .unwrap();
    let (version2,): (i32,) =
        sqlx::query_as("SELECT version FROM ent_doc_identite WHERE etablissement_id = $1")
            .bind(w.m.etab)
            .fetch_one(w.ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(version2, version + 1);
    // Textes encadrés : deux emplacements au plus, 400 caractères au plus.
    let nature = "eleve.certificat_scolarite";
    w.ent
        .parametrer_nature_for_actor(
            &w.dir(),
            nature,
            true,
            Some("classique"),
            &[("mention", "Valable pour l'année en cours")],
        )
        .await
        .unwrap();
    assert_eq!(
        w.ent
            .param_nature(w.m.etab, nature)
            .await
            .unwrap()
            .textes_encadres,
        vec![(
            "mention".to_string(),
            "Valable pour l'année en cours".to_string()
        )]
    );
    assert!(w
        .ent
        .parametrer_nature_for_actor(
            &w.dir(),
            nature,
            true,
            None,
            &[("a", "x"), ("b", "y"), ("c", "z")]
        )
        .await
        .is_err());
    assert!(w
        .ent
        .parametrer_nature_for_actor(
            &w.dir(),
            nature,
            true,
            None,
            &[("mention", &"x".repeat(401))]
        )
        .await
        .is_err());
    assert!(w
        .ent
        .parametrer_nature_for_actor(&w.dir(), "eleve.inexistante", true, None, &[])
        .await
        .is_err());
    // Le style et les textes choisis sont ceux que reçoit le moteur ; le style de la nature déroge à celui de l'identité.
    type Vue = (String, Vec<(String, String)>);
    struct Espion(Mutex<Vec<Vue>>);
    impl RenduDocument for Espion {
        fn rendre(&self, d: &DemandeRendu<'_>) -> Result<DocumentRendu, ErreurRendu> {
            self.0
                .lock()
                .unwrap()
                .push((d.style.to_string(), d.textes_encadres.to_vec()));
            Ok(DocumentRendu {
                octets: b"%PDF espion".to_vec(),
                nb_pages: 1,
            })
        }
    }
    let espion = Espion(Mutex::new(Vec::new()));
    w.ent
        .emettre_document_for_actor(
            &w.dir(),
            w.demande(vec![w.dest(&w.parent_lea, "Mme Potter", None)]),
            &moteur(&espion, &CachetTest),
        )
        .await
        .unwrap();
    let vus = espion.0.lock().unwrap().clone();
    assert_eq!(vus[0].0, "classique", "dérogation de la nature");
    assert_eq!(
        vus[0].1,
        vec![(
            "mention".to_string(),
            "Valable pour l'année en cours".to_string()
        )]
    );
    // Entités émettrices : une série par entité, désignée par famille (R-D13).
    let org = w
        .ent
        .creer_entite_for_actor(
            &w.dir(),
            NouvelleEntite {
                type_code: "organisme_gestion".into(),
                raison_sociale: "OGEC Saint-Test".into(),
                adresse: None,
                mentions_pied: None,
                identifiants: vec![("siret".into(), "12345678900012".into())],
            },
        )
        .await
        .unwrap();
    assert!(w
        .ent
        .creer_entite_for_actor(
            &w.dir(),
            NouvelleEntite {
                type_code: "etablissement".into(),
                raison_sociale: "Doublon".into(),
                adresse: None,
                mentions_pied: None,
                identifiants: vec![]
            }
        )
        .await
        .is_err());
    w.ent
        .affecter_famille_entite_for_actor(&w.dir(), "scolarite", org)
        .await
        .unwrap();
    assert!(w
        .ent
        .affecter_famille_entite_for_actor(&w.dir(), "famille_inconnue", org)
        .await
        .is_err());
    let e = w.un_certificat().await;
    assert!(
        e.numero_affiche.ends_with("-SCO-000001"),
        "la série de l'entité désignée démarre à 1 : {}",
        e.numero_affiche
    );
    let (entite,): (Uuid,) = sqlx::query_as("SELECT entite_id FROM ent_doc_document WHERE id = $1")
        .bind(e.document_id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(entite, org);
}
