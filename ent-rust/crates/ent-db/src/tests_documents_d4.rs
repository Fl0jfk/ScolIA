//! Socle 7 (documents), lot D4 : le coffre, contre la base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, les
//! tests sont ignorés en local (échec explicite en CI ou avec `ENT_EXIGER_BASE`). Chaque test crée son propre
//! établissement. En mode strict (`ENT_RLS_MODE=strict`), le service tourne sous un rôle sans privilège
//! (`NOBYPASSRLS`) : tous ces tests prouvent donc aussi le cloisonnement par la base.
//!
//! Critères de fin du lot (§23, ligne D4) et cas de §4.2 :
//!
//! | Exigence | Test(s) |
//! |---|---|
//! | parents séparés : deux exemplaires, deux coffres | `t_parents_separes_deux_coffres` |
//! | lien suspendu → plus rien de nouveau ; passé retiré ; rendu explicitement | `t_lien_suspendu_retrait_et_rendu`, `t_evenement_lien_suspendu_retire` |
//! | parent sans autorité parentale | `t_parent_sans_autorite` |
//! | élève majeur : reçoit seul ; bascule à la majorité idempotente et notifiée | `t_majeur_recoit_seul_et_bascule`, `t_evenement_majorite_bascule_et_previent` |
//! | aucune pièce jointe, aucun contenu dans la notification | `t_notification_sans_contenu_ni_piece_jointe` |
//! | accusé de première ouverture, une seule fois, visible du personnel seulement | `t_accuse_une_seule_fois` |
//! | export du coffre (archive + index), limité au droit du jour | `t_export_du_coffre` |
//! | mise à disposition d'un document existant ; retrait motivé | `t_mise_a_disposition_et_retrait` |
//! | séparation entre établissements | `t_separation_entre_etablissements` |

use super::*;
use crate::documents::*;
use crate::documents_coffre::*;
use crate::documents_repo::*;
use crate::outbox::ConfigDistribution;
use crate::ref_commun::Acteur;
use crate::service::{EntDbError, RefusMetier};
use crate::tests_demandes::{fiche, labo, monde, Acteur as ActeurTest, Monde};
use chrono::NaiveDate;
use uuid::Uuid;

// ───────────────────────── Doublures ─────────────────────────

struct Rendu;

impl RenduDocument for Rendu {
    fn rendre(&self, d: &DemandeRendu<'_>) -> Result<DocumentRendu, ErreurRendu> {
        let texte = format!(
            "%PDF-1.4 TEST|{}|v{}|{}",
            d.numero_affiche,
            d.numero_version,
            d.destinataire_nom.unwrap_or("-")
        );
        Ok(DocumentRendu {
            octets: texte.into_bytes(),
            nb_pages: 1,
        })
    }
}

struct Cachet;

impl CachetEtablissement for Cachet {
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

fn config(etab: Uuid) -> ConfigDistribution {
    ConfigDistribution {
        taille_lot: 100,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    }
}

// ───────────────────────── Le monde de test ─────────────────────────

pub(crate) struct W {
    pub(crate) ent: PgEnt,
    pub(crate) cast: LaboCast,
    pub(crate) m: Monde,
    pub(crate) lily: ActeurTest,
    pub(crate) tom: ActeurTest,
    pub(crate) jour: NaiveDate,
}

impl W {
    pub(crate) async fn nouveau() -> Option<W> {
        let (ent, cast) = labo().await?;
        let m = monde(&ent, &cast).await;
        let lily = fiche(&ent, m.etab, cast.parent_lea.user_id, "parent", false).await;
        let tom = fiche(&ent, m.etab, cast.parent_tom.user_id, "parent", false).await;
        let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1)")
            .bind(m.etab)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
        Some(W {
            ent,
            cast,
            m,
            lily,
            tom,
            jour,
        })
    }

    pub(crate) fn acteur(&self, user: Uuid, role: &'static str) -> Acteur<'static> {
        Acteur {
            etablissement_id: self.m.etab,
            user_id: user,
            role,
        }
    }
    pub(crate) fn dir(&self) -> Acteur<'static> {
        self.acteur(self.cast.direction.user_id, "direction")
    }
    pub(crate) fn adm(&self) -> Acteur<'static> {
        self.acteur(self.cast.administratif.user_id, "administratif")
    }
    fn prof(&self) -> Acteur<'static> {
        self.acteur(self.cast.prof.user_id, "prof")
    }
    fn parent(&self, p: &ActeurTest) -> Acteur<'static> {
        self.acteur(p.user, "parent")
    }

    /// Un élève mineur (né en 2012), scolarisé depuis 60 jours, niveau « 4e ».
    pub(crate) async fn eleve(&self, nom: &str, prenom: &str) -> Uuid {
        self.eleve_ne_le(nom, prenom, NaiveDate::from_ymd_opt(2012, 3, 10).unwrap())
            .await
    }

    async fn eleve_ne_le(&self, nom: &str, prenom: &str, naissance: NaiveDate) -> Uuid {
        let pool = self.ent.pool_verif();
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, classe, date_naissance, date_entree, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'actif', '4B', $5, $6, now(), now())",
        )
        .bind(id)
        .bind(self.m.etab)
        .bind(nom)
        .bind(prenom)
        .bind(naissance)
        .bind(self.jour - chrono::Duration::days(60))
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO ent_eleve_scolarite_annee (id, etablissement_id, eleve_id, annee_scolaire, niveau, classe, statut, source)
             VALUES ($1, $2, $3, '2026-2027', '4e', '4B', 'courante', 'saisie')",
        )
        .bind(Uuid::new_v4())
        .bind(self.m.etab)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
        id
    }

    pub(crate) async fn lier(&self, eleve: Uuid, p: &ActeurTest, autorite: bool) {
        sqlx::query(
            "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale)
             VALUES ($1, $2, $3, $4, $5, 'parent', $6)",
        )
        .bind(Uuid::new_v4())
        .bind(self.m.etab)
        .bind(eleve)
        .bind(p.user)
        .bind(p.personne)
        .bind(autorite)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
    }

    async fn suspendre(&self, eleve: Uuid, p: &ActeurTest, suspendre: bool) {
        sqlx::query(
            "UPDATE ent_eleve_lien SET suspendu_le = CASE WHEN $4 THEN now() ELSE NULL END,
                    suspension_motif = CASE WHEN $4 THEN 'suspension de test' ELSE NULL END
              WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3",
        )
        .bind(self.m.etab)
        .bind(eleve)
        .bind(p.personne)
        .bind(suspendre)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
    }

    pub(crate) async fn certificat(&self, eleve: Uuid) -> DocumentEmis {
        self.ent
            .emettre_certificat_scolarite_for_actor(&self.adm(), eleve, &moteur(&Rendu, &Cachet))
            .await
            .expect("certificat émis")
    }

    /// Un compte pour l'élève (rôle élève, rattaché par `ent_eleve_compte`).
    async fn compte_eleve(&self, eleve: Uuid) -> Acteur<'static> {
        let user = Uuid::new_v4();
        let pool = self.ent.pool_verif();
        sqlx::query(
            "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at) VALUES ($1, $2, 'x', 'Élève', now(), now())",
        )
        .bind(user)
        .bind(format!("eleve-{}@ent.lab", user.simple()))
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1, $2, 'eleve', now())")
            .bind(user)
            .bind(self.m.etab)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id, created_at) VALUES ($1, $2, $3, now())")
            .bind(self.m.etab)
            .bind(eleve)
            .bind(user)
            .execute(pool)
            .await
            .unwrap();
        self.acteur(user, "eleve")
    }

    /// Fixe la date de naissance (la base interdit l'écriture directe : déclencheurs suspendus dans une transaction du
    /// propriétaire seulement, le temps d'une instruction de préparation).
    async fn fixer_naissance(&self, eleve: Uuid, naissance: NaiveDate) {
        let mut tx = self.ent.pool_verif().begin().await.unwrap();
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE ent_eleve SET date_naissance = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(self.m.etab)
        .bind(eleve)
        .bind(naissance)
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    async fn distribuer(&self) {
        self.ent
            .distributeur(config(self.m.etab))
            .distribuer_tout()
            .await
            .unwrap();
    }

    async fn compte(&self, sql: &str) -> i64 {
        let (n,): (i64,) = sqlx::query_as(sql)
            .bind(self.m.etab)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        n
    }

    pub(crate) async fn coffre(&self, a: &Acteur<'_>) -> Vec<LigneCoffre> {
        self.ent
            .coffre_for_actor(a, &FiltreCoffre::default())
            .await
            .unwrap()
    }
}

fn est_refus(e: &EntDbError, attendu: RefusMetier) -> bool {
    matches!(e, EntDbError::Refus(r, _) if *r == attendu)
}

fn introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound))
}

// ───────────────────────── Coffres ─────────────────────────

#[tokio::test]
async fn t_parents_separes_deux_coffres() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Dupont", "Laura").await;
    w.lier(eleve, &w.lily, true).await;
    w.lier(eleve, &w.tom, true).await;
    let e = w.certificat(eleve).await;
    assert_eq!(e.exemplaires.len(), 2);
    let (lily, tom) = (w.parent(&w.lily), w.parent(&w.tom));
    let (cl, ct) = (w.coffre(&lily).await, w.coffre(&tom).await);
    assert_eq!(
        cl.len(),
        1,
        "un document dans le coffre de chaque titulaire"
    );
    assert_eq!(ct.len(), 1);
    assert_ne!(cl[0].mise_a_disposition_id, ct[0].mise_a_disposition_id);
    assert_eq!(
        cl[0].document_id, ct[0].document_id,
        "même document, même numéro"
    );
    assert_eq!(cl[0].numero_affiche, ct[0].numero_affiche);
    let a = w
        .ent
        .ouvrir_du_coffre_for_actor(&lily, cl[0].mise_a_disposition_id)
        .await
        .unwrap();
    let b = w
        .ent
        .ouvrir_du_coffre_for_actor(&tom, ct[0].mise_a_disposition_id)
        .await
        .unwrap();
    assert_ne!(
        a.exemplaire_rang, b.exemplaire_rang,
        "deux exemplaires nominatifs"
    );
    assert_ne!(a.octets, b.octets, "chacun a le sien");
    // Chacun ne peut ouvrir que SA mise à disposition : l'identifiant de l'autre répond « introuvable ».
    let e1 = w
        .ent
        .ouvrir_du_coffre_for_actor(&lily, ct[0].mise_a_disposition_id)
        .await
        .unwrap_err();
    assert!(introuvable(&e1), "{e1:?}");
    // Le filtre par enfant et par année ; « pour moi » ne contient pas les documents de l'enfant.
    let f = FiltreCoffre {
        eleve_id: Some(eleve),
        ..Default::default()
    };
    assert_eq!(w.ent.coffre_for_actor(&lily, &f).await.unwrap().len(), 1);
    let f = FiltreCoffre {
        pour_moi: true,
        ..Default::default()
    };
    assert!(w.ent.coffre_for_actor(&lily, &f).await.unwrap().is_empty());
    let f = FiltreCoffre {
        famille: Some("inscriptions.".into()),
        ..Default::default()
    };
    assert!(w.ent.coffre_for_actor(&lily, &f).await.unwrap().is_empty());
    let f = FiltreCoffre {
        famille: Some("eleve.".into()),
        ..Default::default()
    };
    assert_eq!(w.ent.coffre_for_actor(&lily, &f).await.unwrap().len(), 1);
    // Un professeur n'a pas de coffre sur l'élève : son coffre est vide.
    assert!(w.coffre(&w.prof()).await.is_empty());
    // La lecture historique (D2) d'un titulaire passe par le même coffre.
    let lu = w
        .ent
        .lire_document_for_actor(&lily, e.document_id, None)
        .await
        .unwrap();
    assert_eq!(lu.octets, a.octets);
}

#[tokio::test]
async fn t_lien_suspendu_retrait_et_rendu() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Martin", "Louis").await;
    w.lier(eleve, &w.lily, true).await;
    w.lier(eleve, &w.tom, true).await;
    w.certificat(eleve).await;
    let (lily, tom) = (w.parent(&w.lily), w.parent(&w.tom));
    let mad = w.coffre(&lily).await[0].mise_a_disposition_id;
    // Suspension du lien de Lily (sans événement, comme deux chemins du socle 4) : le droit est relu à la lecture,
    // le coffre est vide et l'ouverture refusée AVANT tout rangement.
    w.suspendre(eleve, &w.lily, true).await;
    assert!(
        w.coffre(&lily).await.is_empty(),
        "lien suspendu : plus rien dans le coffre"
    );
    assert!(introuvable(
        &w.ent
            .ouvrir_du_coffre_for_actor(&lily, mad)
            .await
            .unwrap_err()
    ));
    assert_eq!(
        w.coffre(&tom).await.len(),
        1,
        "l'autre titulaire n'est pas touché"
    );
    // Rien de nouveau : une nouvelle émission ne va qu'à Tom.
    let e2 = w.certificat(eleve).await;
    assert_eq!(e2.exemplaires.len(), 1);
    assert!(w.coffre(&lily).await.is_empty());
    assert_eq!(w.coffre(&tom).await.len(), 2);
    // La réconciliation range le passé (retrait motivé, journal).
    assert_eq!(
        w.ent.reconcilier_liens_suspendus(w.m.etab).await.unwrap(),
        1
    );
    assert_eq!(
        w.ent.reconcilier_liens_suspendus(w.m.etab).await.unwrap(),
        0,
        "idempotente"
    );
    let (motif,): (String,) = sqlx::query_as(
        "SELECT retrait_motif FROM ent_doc_mise_a_disposition WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.m.etab)
    .bind(mad)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(motif, "lien suspendu");
    // Rendre explicitement (Q-D12) : refusé tant que le lien est suspendu, droit `documents.classer` exigé.
    let e = w
        .ent
        .rendre_mise_a_disposition_for_actor(&w.dir(), mad, "Lien rétabli")
        .await
        .unwrap_err();
    assert!(est_refus(&e, RefusMetier::Conflit), "{e:?}");
    assert!(matches!(
        w.ent
            .rendre_mise_a_disposition_for_actor(&w.prof(), mad, "Essai")
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    w.suspendre(eleve, &w.lily, false).await;
    assert!(
        matches!(
            w.ent
                .rendre_mise_a_disposition_for_actor(&lily, mad, "Moi-même")
                .await
                .unwrap_err(),
            EntDbError::Forbidden
        ),
        "un parent ne se rend pas son propre document"
    );
    let rendue = w
        .ent
        .rendre_mise_a_disposition_for_actor(&w.dir(), mad, "Lien rétabli")
        .await
        .unwrap();
    let c = w.coffre(&lily).await;
    assert_eq!(
        c.len(),
        1,
        "le document rendu est de nouveau dans le coffre"
    );
    assert_eq!(c[0].mise_a_disposition_id, rendue);
    assert!(w
        .ent
        .ouvrir_du_coffre_for_actor(&lily, rendue)
        .await
        .is_ok());
    // Rendre deux fois : refusé (elle n'est plus retirée... la nouvelle est active, l'ancienne reste retirée).
    let e = w
        .ent
        .rendre_mise_a_disposition_for_actor(&w.dir(), rendue, "Encore")
        .await
        .unwrap_err();
    assert!(est_refus(&e, RefusMetier::Conflit));
    // Le journal garde la trace des deux gestes.
    let n = w
        .compte("SELECT count(*) FROM ent_doc_journal WHERE etablissement_id = $1 AND action IN ('mise_a_disposition_retiree', 'mise_a_disposition_rendue')")
        .await;
    assert_eq!(n, 2);
}

#[tokio::test]
async fn t_evenement_lien_suspendu_retire() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Petit", "Jules").await;
    w.lier(eleve, &w.lily, true).await;
    w.lier(eleve, &w.tom, true).await;
    w.certificat(eleve).await;
    w.suspendre(eleve, &w.lily, true).await;
    // L'événement `lien.suspendu` du socle 4 (charge utile : lien et personne) est consommé par le coffre.
    publier_evenement(
        &w,
        ent_events::types::LIEN_SUSPENDU,
        ent_events::AggregateKind::Eleve,
        eleve,
        Some(eleve),
        serde_json::json!({ "lien_id": Uuid::new_v4(), "personne_id": w.lily.personne }),
    )
    .await;
    w.distribuer().await;
    let (n, motif): (i64, Option<String>) = sqlx::query_as(
        "SELECT count(*), max(retrait_motif) FROM ent_doc_mise_a_disposition WHERE etablissement_id = $1 AND personne_id = $2 AND retiree_le IS NOT NULL",
    )
    .bind(w.m.etab)
    .bind(w.lily.personne)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((n, motif.as_deref()), (1, Some("lien suspendu")));
    let (tom_actif,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_mise_a_disposition WHERE etablissement_id = $1 AND personne_id = $2 AND retiree_le IS NULL",
    )
    .bind(w.m.etab)
    .bind(w.tom.personne)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(tom_actif, 1);
}

async fn publier_evenement(
    w: &W,
    type_name: &str,
    aggregate: ent_events::AggregateKind,
    aggregate_id: Uuid,
    eleve: Option<Uuid>,
    payload: serde_json::Value,
) {
    use crate::outbox::Publier;
    let mut tx = crate::tenant_tx::TenantTx::begin(w.ent.pool(), w.m.etab)
        .await
        .unwrap();
    tx.publier(ent_events::MetierEventDraft {
        etablissement_id: w.m.etab,
        type_name: type_name.to_string(),
        aggregate,
        aggregate_id,
        eleve_id: eleve,
        payload,
        actor_user_id: None,
    })
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn t_parent_sans_autorite() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Roux", "Emma").await;
    w.lier(eleve, &w.lily, true).await;
    w.lier(eleve, &w.tom, false).await; // lien sans autorité parentale (beau-parent, tiers de confiance)
    let e = w.certificat(eleve).await;
    assert_eq!(
        e.exemplaires.len(),
        1,
        "seul le titulaire de l'autorité reçoit"
    );
    assert!(w.coffre(&w.parent(&w.tom)).await.is_empty());
    // Même mis à disposition à la main, un parent sans autorité est refusé.
    let err = w
        .ent
        .mettre_a_disposition_for_actor(
            &w.adm(),
            e.document_id,
            DestinataireMad::Personne(w.tom.personne),
        )
        .await
        .unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit), "{err:?}");
    // Et même si une ligne existait (ancienne autorité perdue depuis), le droit relu à la lecture ne sert rien.
    sqlx::query(
        "INSERT INTO ent_doc_mise_a_disposition (id, etablissement_id, document_id, exemplaire_id, personne_id, canal)
         SELECT $1, ex.etablissement_id, v.document_id, ex.id, $3, 'coffre'
           FROM ent_doc_exemplaire ex JOIN ent_doc_version v ON v.etablissement_id = ex.etablissement_id AND v.id = ex.version_id
          WHERE ex.etablissement_id = $2 AND v.document_id = $4 ORDER BY ex.rang LIMIT 1",
    )
    .bind(Uuid::new_v4())
    .bind(w.m.etab)
    .bind(w.tom.personne)
    .bind(e.document_id)
    .execute(w.ent.pool_verif())
    .await
    .unwrap();
    assert!(
        w.coffre(&w.parent(&w.tom)).await.is_empty(),
        "sans autorité parentale, le coffre reste vide"
    );
}

#[tokio::test]
async fn t_majeur_recoit_seul_et_bascule() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Né un jour après le 18e anniversaire moins un : mineur aujourd'hui, majeur demain.
    let eleve = w
        .eleve_ne_le(
            "Lambert",
            "Hugo",
            ajouter_annees(w.jour, -18) + chrono::Duration::days(1),
        )
        .await;
    w.lier(eleve, &w.lily, true).await;
    w.lier(eleve, &w.tom, true).await;
    let compte = w.compte_eleve(eleve).await;
    let e = w.certificat(eleve).await;
    assert_eq!(e.exemplaires.len(), 2, "mineur : les titulaires reçoivent");
    assert!(
        w.coffre(&compte).await.is_empty(),
        "l'élève mineur n'a rien tant que rien ne lui a été adressé"
    );
    // Un document de scolarité C1 peut être adressé à l'élève mineur (nature et sensibilité admises).
    let mad_eleve = w
        .ent
        .mettre_a_disposition_for_actor(&w.adm(), e.document_id, DestinataireMad::Eleve(eleve))
        .await
        .unwrap();
    assert_eq!(w.coffre(&compte).await.len(), 1);
    assert!(w
        .ent
        .ouvrir_du_coffre_for_actor(&compte, mad_eleve)
        .await
        .is_ok());
    // Le jour de ses 18 ans : les parents ne voient plus rien (droit relu), AVANT même la bascule.
    w.fixer_naissance(eleve, ajouter_annees(w.jour, -18)).await;
    assert!(
        w.coffre(&w.parent(&w.lily)).await.is_empty(),
        "majeur : le coffre des parents se ferme"
    );
    assert!(w.coffre(&w.parent(&w.tom)).await.is_empty());
    // Les mises à disposition nouvelles vont à l'élève seul (D3 : l'élève majeur reçoit lui-même) et refusées aux parents.
    let err = w
        .ent
        .mettre_a_disposition_for_actor(
            &w.adm(),
            e.document_id,
            DestinataireMad::Personne(w.lily.personne),
        )
        .await
        .unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit), "{err:?}");
    // La bascule : retire les mises à disposition des parents, ouvre l'élève ; idempotente.
    let b = w
        .ent
        .basculer_coffre_majorite(w.m.etab, eleve)
        .await
        .unwrap();
    assert_eq!((b.retirees, b.sans_exemplaire_neutre), (2, 0), "{b:?}");
    assert_eq!(
        b.ouvertes, 0,
        "l'élève avait déjà reçu le sien avant sa majorité"
    );
    let b2 = w
        .ent
        .basculer_coffre_majorite(w.m.etab, eleve)
        .await
        .unwrap();
    assert_eq!(b2, BasculeFaite::default(), "idempotente");
    assert_eq!(w.coffre(&compte).await.len(), 1);
    let (retraits,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_mise_a_disposition WHERE etablissement_id = $1 AND retrait_motif = 'majorité de l''élève'",
    )
    .bind(w.m.etab)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(retraits, 2);
}

fn ajouter_annees(d: NaiveDate, n: i32) -> NaiveDate {
    use chrono::Datelike;
    NaiveDate::from_ymd_opt(d.year() + n, d.month(), d.day())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(d.year() + n, d.month(), d.day() - 1).unwrap())
}

#[tokio::test]
async fn t_evenement_majorite_bascule_et_previent() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w
        .eleve_ne_le(
            "Garnier",
            "Zoé",
            ajouter_annees(w.jour, -18) + chrono::Duration::days(1),
        )
        .await;
    w.lier(eleve, &w.lily, true).await;
    let compte = w.compte_eleve(eleve).await;
    w.certificat(eleve).await;
    assert_eq!(w.coffre(&w.parent(&w.lily)).await.len(), 1);
    // Majorité atteinte, puis l'événement du socle 2 (`eleve.majeur`) arrive.
    w.fixer_naissance(eleve, ajouter_annees(w.jour, -18)).await;
    publier_evenement(
        &w,
        ent_events::types::ELEVE_MAJEUR,
        ent_events::AggregateKind::Eleve,
        eleve,
        Some(eleve),
        serde_json::json!({}),
    )
    .await;
    w.distribuer().await; // consomme `eleve.majeur`, publie `documents.coffre_bascule`
    w.distribuer().await; // consomme `documents.coffre_bascule` : prévient l'élève
    let c = w.coffre(&compte).await;
    assert_eq!(
        c.len(),
        1,
        "l'élève reçoit son certificat sur un exemplaire sans adresse"
    );
    assert!(w.coffre(&w.parent(&w.lily)).await.is_empty());
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'coffre_majorite'",
    )
    .bind(w.m.etab)
    .bind(compte.user_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1, "l'élève est informé");
    // Rejouer l'événement ne change rien (idempotence du consommateur et de la bascule).
    publier_evenement(
        &w,
        ent_events::types::ELEVE_MAJEUR,
        ent_events::AggregateKind::Eleve,
        eleve,
        Some(eleve),
        serde_json::json!({}),
    )
    .await;
    w.distribuer().await;
    w.distribuer().await;
    assert_eq!(w.coffre(&compte).await.len(), 1);
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'coffre_majorite'",
    )
    .bind(w.m.etab)
    .bind(compte.user_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1, "pas de second message");
}

#[tokio::test]
async fn t_notification_sans_contenu_ni_piece_jointe() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Dubois", "Sacha").await;
    w.lier(eleve, &w.lily, true).await;
    let e = w.certificat(eleve).await;
    w.distribuer().await;
    let notifs: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT kind, titre, corps FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2",
    )
    .bind(w.m.etab)
    .bind(w.lily.user)
    .fetch_all(w.ent.pool_verif())
    .await
    .unwrap();
    let doc: Vec<_> = notifs
        .iter()
        .filter(|n| n.0 == "document_disponible")
        .collect();
    assert_eq!(doc.len(), 1, "une notification par mise à disposition");
    let tout = format!("{} {}", doc[0].1, doc[0].2).to_lowercase();
    for interdit in [
        "certificat",
        "dubois",
        "sacha",
        "scolarité",
        "pdf",
        &e.numero_affiche.to_lowercase(),
        "eleve.",
    ] {
        assert!(
            !tout.contains(interdit),
            "la notification ne doit pas contenir « {interdit} » : {tout}"
        );
    }
    assert!(
        tout.contains("coffre") && tout.contains("connectez"),
        "elle renvoie au coffre : {tout}"
    );
    // Même chose pour tous les envois planifiés (mail, SMS, push) issus de ces notifications.
    let envois: Vec<(String, String)> = sqlx::query_as(
        "SELECT titre, corps FROM ent_notification_envoi WHERE etablissement_id = $1 AND kind = 'document_disponible'",
    )
    .bind(w.m.etab)
    .fetch_all(w.ent.pool_verif())
    .await
    .unwrap();
    for (t, c) in &envois {
        let x = format!("{t} {c}").to_lowercase();
        assert!(
            !x.contains("certificat") && !x.contains("dubois") && !x.contains("pdf"),
            "{x}"
        );
    }
    // Les canaux n'ont aucun moyen de porter une pièce jointe : aucune colonne de ce genre dans leurs tables.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM information_schema.columns
          WHERE table_name IN ('ent_notification', 'ent_notification_envoi', 'ent_mail_outbox', 'ent_sms_outbox', 'ent_push_outbox')
            AND (column_name ILIKE '%attach%' OR column_name ILIKE '%piece%' OR column_name ILIKE '%fichier%'
                 OR column_name ILIKE '%file%' OR column_name ILIKE '%pdf%' OR column_name ILIKE '%objet%')",
    )
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 0, "aucun canal ne sait envoyer une pièce jointe (Q-D13)");
    // Un retrait avant la distribution n'envoie rien.
    let eleve2 = w.eleve("Moreau", "Lina").await;
    w.lier(eleve2, &w.tom, true).await;
    let e2 = w.certificat(eleve2).await;
    let mad = w.coffre(&w.parent(&w.tom)).await[0].mise_a_disposition_id;
    w.ent
        .retirer_mise_a_disposition_for_actor(&w.adm(), mad, "Erreur de destinataire")
        .await
        .unwrap();
    w.distribuer().await;
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'document_disponible'",
    )
    .bind(w.m.etab)
    .bind(w.tom.user)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        n, 0,
        "document retiré avant la distribution : aucun message ({})",
        e2.numero_affiche
    );
}

#[tokio::test]
async fn t_accuse_une_seule_fois() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Blanc", "Théo").await;
    w.lier(eleve, &w.lily, true).await;
    let e = w.certificat(eleve).await;
    let lily = w.parent(&w.lily);
    let mad = w.coffre(&lily).await[0].mise_a_disposition_id;
    // Avant ouverture : l'accusé est vide pour le personnel ; la famille n'a pas accès aux accusés.
    let avant = w
        .ent
        .accuses_for_actor(&w.adm(), e.document_id)
        .await
        .unwrap();
    assert_eq!(avant.len(), 1);
    assert!(avant[0].premiere_ouverture_le.is_none());
    assert_eq!(avant[0].titulaire, "personne");
    assert!(matches!(
        w.ent
            .accuses_for_actor(&lily, e.document_id)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(matches!(
        w.ent
            .accuses_for_actor(&w.prof(), e.document_id)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    let non_ouverts = w
        .ent
        .non_ouverts_for_actor(
            &w.adm(),
            "eleve.",
            chrono::Utc::now() + chrono::Duration::minutes(1),
        )
        .await
        .unwrap();
    assert!(
        non_ouverts.iter().any(|n| n.mise_a_disposition_id == mad),
        "relance proposée"
    );
    // Deux ouvertures simultanées : un seul accusé, un seul événement.
    let (r1, r2) = tokio::join!(
        w.ent.ouvrir_du_coffre_for_actor(&lily, mad),
        w.ent.ouvrir_du_coffre_for_actor(&lily, mad)
    );
    r1.unwrap();
    r2.unwrap();
    let apres = w
        .ent
        .accuses_for_actor(&w.adm(), e.document_id)
        .await
        .unwrap();
    let t1 = apres[0].premiere_ouverture_le.expect("accusé posé");
    w.ent.ouvrir_du_coffre_for_actor(&lily, mad).await.unwrap();
    let encore = w
        .ent
        .accuses_for_actor(&w.adm(), e.document_id)
        .await
        .unwrap();
    assert_eq!(
        encore[0].premiere_ouverture_le,
        Some(t1),
        "l'accusé ne bouge plus"
    );
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'documents.ouvert' AND aggregate_id = $2",
    )
    .bind(w.m.etab)
    .bind(e.document_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1, "un seul `documents.ouvert`");
    assert!(w
        .ent
        .non_ouverts_for_actor(
            &w.adm(),
            "eleve.",
            chrono::Utc::now() + chrono::Duration::minutes(1)
        )
        .await
        .unwrap()
        .iter()
        .all(|n| n.mise_a_disposition_id != mad));
    // Le coffre indique l'ouverture au titulaire.
    assert_eq!(w.coffre(&lily).await[0].premiere_ouverture_le, Some(t1));
    assert!(
        accuse_demande("discipline.convocation") && !accuse_demande("eleve.certificat_scolarite")
    );
}

#[tokio::test]
async fn t_export_du_coffre() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Faure", "Inès").await;
    let eleve2 = w.eleve("Faure", "Noé").await;
    w.lier(eleve, &w.lily, true).await;
    w.lier(eleve2, &w.lily, true).await;
    w.lier(eleve2, &w.tom, true).await;
    let e1 = w.certificat(eleve).await;
    let e2 = w.certificat(eleve2).await;
    let lily = w.parent(&w.lily);
    let x = w
        .ent
        .exporter_coffre_for_actor(&lily, &FiltreCoffre::default())
        .await
        .unwrap();
    assert_eq!(x.nb_documents, 2);
    let lu =
        crate::documents_zip::lire_zip_stored(&x.octets).expect("archive valide (CRC vérifiés)");
    assert_eq!(lu.len(), 3, "deux PDF et l'index");
    assert!(lu
        .iter()
        .filter(|(n, _)| n.ends_with(".pdf"))
        .all(|(_, o)| o.starts_with(b"%PDF")));
    let index =
        String::from_utf8(lu.iter().find(|(n, _)| n == "index.csv").unwrap().1.clone()).unwrap();
    assert!(
        index.contains(&e1.numero_affiche) && index.contains(&e2.numero_affiche),
        "{index}"
    );
    assert!(index.contains("Inès Faure") && index.contains("Noé Faure"));
    assert_eq!(x.sha256, ent_object::sha256_hex(&x.octets));
    // Par enfant : un seul document.
    let f = FiltreCoffre {
        eleve_id: Some(eleve2),
        ..Default::default()
    };
    assert_eq!(
        w.ent
            .exporter_coffre_for_actor(&lily, &f)
            .await
            .unwrap()
            .nb_documents,
        1
    );
    // Tom n'exporte que le sien ; lien suspendu : plus rien d'autre que l'index.
    let tom = w.parent(&w.tom);
    assert_eq!(
        w.ent
            .exporter_coffre_for_actor(&tom, &FiltreCoffre::default())
            .await
            .unwrap()
            .nb_documents,
        1
    );
    w.suspendre(eleve2, &w.tom, true).await;
    assert_eq!(
        w.ent
            .exporter_coffre_for_actor(&tom, &FiltreCoffre::default())
            .await
            .unwrap()
            .nb_documents,
        0
    );
    // Un compte sans aucune mise à disposition n'exporte qu'un index vide.
    let vide = w
        .ent
        .exporter_coffre_for_actor(&w.prof(), &FiltreCoffre::default())
        .await
        .unwrap();
    assert_eq!(vide.nb_documents, 0);
    // L'export est journalisé.
    let n = w.compte("SELECT count(*) FROM ent_doc_journal WHERE etablissement_id = $1 AND action = 'coffre_exporte'").await;
    assert_eq!(n, 5);
}

#[tokio::test]
async fn t_mise_a_disposition_et_retrait() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Henry", "Maël").await;
    w.lier(eleve, &w.lily, true).await;
    let e = w.certificat(eleve).await;
    // Ajouter Tom (titulaire) après coup : droit `documents.emettre` exigé, exemplaire sans adresse choisi.
    w.lier(eleve, &w.tom, true).await;
    assert!(matches!(
        w.ent
            .mettre_a_disposition_for_actor(
                &w.prof(),
                e.document_id,
                DestinataireMad::Personne(w.tom.personne)
            )
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    let mad = w
        .ent
        .mettre_a_disposition_for_actor(
            &w.adm(),
            e.document_id,
            DestinataireMad::Personne(w.tom.personne),
        )
        .await
        .unwrap();
    assert_eq!(w.coffre(&w.parent(&w.tom)).await.len(), 1);
    // Deux fois : refusé (déjà mis à disposition).
    let err = w
        .ent
        .mettre_a_disposition_for_actor(
            &w.adm(),
            e.document_id,
            DestinataireMad::Personne(w.tom.personne),
        )
        .await
        .unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit), "{err:?}");
    // Retrait : droit `documents.classer`, motif obligatoire, ouverture antérieure signalée.
    assert!(matches!(
        w.ent
            .retirer_mise_a_disposition_for_actor(&w.prof(), mad, "Erreur de destinataire")
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(
        w.ent
            .retirer_mise_a_disposition_for_actor(&w.adm(), mad, "  ")
            .await
            .is_err(),
        "motif obligatoire"
    );
    w.ent
        .ouvrir_du_coffre_for_actor(&w.parent(&w.tom), mad)
        .await
        .unwrap();
    let ouverte = w
        .ent
        .retirer_mise_a_disposition_for_actor(&w.adm(), mad, "Erreur de destinataire")
        .await
        .unwrap();
    assert!(ouverte, "l'ouverture antérieure reste visible");
    assert!(w.coffre(&w.parent(&w.tom)).await.is_empty());
    let err = w
        .ent
        .retirer_mise_a_disposition_for_actor(&w.adm(), mad, "Encore")
        .await
        .unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit));
    // L'accusé garde la trace du retrait.
    let acc = w
        .ent
        .accuses_for_actor(&w.dir(), e.document_id)
        .await
        .unwrap();
    let retiree = acc.iter().find(|a| a.mise_a_disposition_id == mad).unwrap();
    assert!(retiree.retiree_le.is_some() && retiree.premiere_ouverture_le.is_some());
    assert_eq!(
        retiree.retrait_motif.as_deref(),
        Some("Erreur de destinataire")
    );
    // Un document annulé sort du coffre de tous.
    w.ent
        .annuler_document_for_actor(&w.dir(), e.document_id, "Émis par erreur")
        .await
        .unwrap();
    assert!(w.coffre(&w.parent(&w.lily)).await.is_empty());
}

#[tokio::test]
async fn t_separation_entre_etablissements() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Chevalier", "Rose").await;
    w.lier(eleve, &w.lily, true).await;
    let e = w.certificat(eleve).await;
    let lily = w.parent(&w.lily);
    let mad = w.coffre(&lily).await[0].mise_a_disposition_id;
    // Un autre établissement, ses comptes : rien de ce qui est à A n'est visible, lisible ni retirable.
    let cast_b = crate::tests_rls_stricte::seed_supplementaire(&w.ent).await;
    let mb = monde(&w.ent, &cast_b).await;
    let parent_b = fiche(&w.ent, mb.etab, cast_b.parent_lea.user_id, "parent", false).await;
    let dir_b = Acteur {
        etablissement_id: mb.etab,
        user_id: cast_b.direction.user_id,
        role: "direction",
    };
    let adm_b = Acteur {
        etablissement_id: mb.etab,
        user_id: cast_b.administratif.user_id,
        role: "administratif",
    };
    let par_b = Acteur {
        etablissement_id: mb.etab,
        user_id: parent_b.user,
        role: "parent",
    };
    assert!(w
        .ent
        .coffre_for_actor(&par_b, &FiltreCoffre::default())
        .await
        .unwrap()
        .is_empty());
    assert!(introuvable(
        &w.ent
            .ouvrir_du_coffre_for_actor(&par_b, mad)
            .await
            .unwrap_err()
    ));
    // Le parent de A, présenté dans le contexte de B (même identité, autre établissement) : rien non plus.
    let lily_chez_b = Acteur {
        etablissement_id: mb.etab,
        ..lily
    };
    assert!(w
        .ent
        .coffre_for_actor(&lily_chez_b, &FiltreCoffre::default())
        .await
        .unwrap()
        .is_empty());
    assert!(introuvable(
        &w.ent
            .ouvrir_du_coffre_for_actor(&lily_chez_b, mad)
            .await
            .unwrap_err()
    ));
    // Le personnel de B ne retire, ne rend, ne met à disposition ni ne voit les accusés d'un document de A.
    assert!(introuvable(
        &w.ent
            .retirer_mise_a_disposition_for_actor(&adm_b, mad, "Tentative")
            .await
            .unwrap_err()
    ));
    assert!(w
        .ent
        .rendre_mise_a_disposition_for_actor(&adm_b, mad, "Tentative")
        .await
        .is_err());
    assert!(w
        .ent
        .mettre_a_disposition_for_actor(
            &adm_b,
            e.document_id,
            DestinataireMad::Personne(parent_b.personne)
        )
        .await
        .is_err());
    assert!(w
        .ent
        .accuses_for_actor(&dir_b, e.document_id)
        .await
        .is_err());
    assert!(
        w.ent
            .exporter_coffre_for_actor(&par_b, &FiltreCoffre::default())
            .await
            .unwrap()
            .nb_documents
            == 0
    );
    // Les comptes de A, eux, voient toujours leur document.
    assert_eq!(w.coffre(&lily).await.len(), 1);
    assert!(w
        .ent
        .retirer_mise_a_disposition_for_actor(&adm_b, mad, "Tentative")
        .await
        .is_err());
    // La réconciliation et la bascule d'un établissement ne touchent pas l'autre.
    assert_eq!(w.ent.reconcilier_liens_suspendus(mb.etab).await.unwrap(), 0);
    assert_eq!(
        w.ent
            .basculer_coffre_majorite(mb.etab, eleve)
            .await
            .unwrap(),
        BasculeFaite::default(),
        "un élève d'un autre établissement est introuvable : rien à basculer"
    );
    assert_eq!(w.coffre(&lily).await.len(), 1);
}
