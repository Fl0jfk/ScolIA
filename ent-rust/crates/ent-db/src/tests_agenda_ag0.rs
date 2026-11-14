//! Socle 19, lot AG0 : correctifs urgents des rencontres parents-professeurs (RD-01 à RD-11,
//! `19_AGENDA_RDV.md` §27). Un test par défaut, contre la base locale de labo (feature `labo`).
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés. Aucun appel réseau (envois simulés).

use super::*;
use crate::rencontres::{NouveauxCreneaux, NouvelleRencontre};
use chrono::{DateTime, Duration, Utc};
use ent_auth::Role;
use uuid::Uuid;

/// Vide le fil des faits d'un établissement (tests de labo seulement : la table est en ajout seul).
pub(crate) async fn vider_faits_rdv_pour_tests(pool: &sqlx::PgPool, etab: Uuid) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_rdv_fait DISABLE TRIGGER ent_rdv_fait_immuable")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("DELETE FROM ent_rdv_fait WHERE etablissement_id = $1")
        .bind(etab)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE ent_rdv_fait ENABLE TRIGGER ent_rdv_fait_immuable")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

pub(crate) async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    // Selon `ENT_RLS_MODE`, le code testé tourne sur le rôle applicatif strict (sans BYPASSRLS) :
    // un accès qui oublie le contexte d'établissement voit 0 ligne (RD-06, `ligne_planning`).
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
        e.install_mail_dry_run();
    })
    .await;
    reset(&ent, cast.etablissement_id).await;
    Some((ent, cast))
}

pub(crate) async fn reset(ent: &PgEnt, etab: Uuid) {
    vider_faits_rdv_pour_tests(ent.pool_verif(), etab).await;
    for t in [
        "ent_notification_envoi",
        "ent_notification",
        "ent_creneau_rdv",
        "ent_rencontre",
    ] {
        sqlx::query(&format!("DELETE FROM {t} WHERE etablissement_id = $1"))
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
    sqlx::query(
        "DELETE FROM ent_eleve_interdiction WHERE etablissement_id = $1 AND consigne = 'ag0-test'",
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("DELETE FROM ent_etablissement_parametre WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_eleve_lien SET suspendu_le = NULL, suspendu_par = NULL, suspension_motif = NULL, suspension_reference = NULL WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

pub(crate) fn dir(c: &LaboCast) -> (Uuid, &'static str) {
    (c.direction.user_id, Role::Direction.as_str())
}

fn dans_trois_jours_9h() -> DateTime<Utc> {
    (Utc::now() + Duration::days(3))
        .date_naive()
        .and_hms_opt(9, 0, 0)
        .unwrap()
        .and_utc()
}

pub(crate) async fn classe_4b(ent: &PgEnt, etab: Uuid) -> Uuid {
    let (id,): (Uuid,) = sqlx::query_as(
        "SELECT c.id FROM ent_classe c
           JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.code = '4B' AND c.archivee_le IS NULL
          ORDER BY y.courante DESC, y.date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .expect("classe 4B amorcée par le seed");
    id
}

pub(crate) async fn donner_service(ent: &PgEnt, etab: Uuid, prof: Uuid, classe: Uuid) {
    sqlx::query(
        "INSERT INTO ent_service_enseignement
            (id, etablissement_id, annee_id, prof_user_id, matiere_libelle, classe_id)
         SELECT $1, c.etablissement_id, c.annee_id, $3, 'Potions', c.id
           FROM ent_classe c
          WHERE c.etablissement_id = $2 AND c.id = $4
            AND NOT EXISTS (SELECT 1 FROM ent_service_enseignement s
                             WHERE s.etablissement_id = c.etablissement_id AND s.prof_user_id = $3
                               AND s.classe_id = c.id)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(prof)
    .bind(classe)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

/// Rencontre ouverte avec `nombre` créneaux de 10 min pour le professeur du cast, dès `debut`.
async fn rencontre_ouverte(
    ent: &PgEnt,
    cast: &LaboCast,
    debut: DateTime<Utc>,
    nombre: i32,
) -> (Uuid, Vec<Uuid>) {
    let etab = cast.etablissement_id;
    let (adm, role) = dir(cast);
    let c4b = classe_4b(ent, etab).await;
    donner_service(ent, etab, cast.prof.user_id, c4b).await;
    let r = ent
        .creer_rencontre(
            etab,
            adm,
            role,
            NouvelleRencontre {
                titre: "AG0".into(),
                classe_id: c4b,
            },
        )
        .await
        .unwrap();
    ent.ajouter_creneaux(
        etab,
        adm,
        role,
        r.id,
        NouveauxCreneaux {
            enseignant_user_id: cast.prof.user_id,
            debut,
            duree_minutes: 10,
            nombre,
        },
    )
    .await
    .unwrap();
    ent.ouvrir_rencontre(etab, adm, role, r.id).await.unwrap();
    let ids: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_creneau_rdv WHERE etablissement_id = $1 AND rencontre_id = $2 ORDER BY debut",
    )
    .bind(etab)
    .bind(r.id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    (r.id, ids.into_iter().map(|(i,)| i).collect())
}

async fn faits(ent: &PgEnt, cast: &LaboCast, creneau: Uuid) -> Vec<crate::rencontres::FaitCreneau> {
    let (adm, role) = dir(cast);
    ent.fil_creneau(cast.etablissement_id, adm, role, creneau)
        .await
        .unwrap()
}

/// Second responsable (même foyer, autorité parentale) de l'enfant de `parent_lea`.
async fn second_parent_de_lea(ent: &PgEnt, cast: &LaboCast) -> Uuid {
    let etab = cast.etablissement_id;
    let user = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
         VALUES ($1,$2,'x','Second parent', now(), now())",
    )
    .bind(user)
    .bind(format!("second-{}@ent.lab", user.simple()))
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1,$2,'parent', now())")
        .bind(user)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE ent_eleve_lien SET foyer_id = $4, autorite_parentale = true
          WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
    )
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(cast.parent_lea.user_id)
    .bind(cast.foyer_lea_id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, foyer_id, lien, autorite_parentale)
         VALUES ($1,$2,$3,$4,$5,'mere', true)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(user)
    .bind(cast.foyer_lea_id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    user
}

pub(crate) async fn corps_notifications(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    kind: &str,
) -> Vec<String> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT corps FROM ent_notification
          WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = $3",
    )
    .bind(etab)
    .bind(user)
    .bind(kind)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    rows.into_iter().map(|(c,)| c).collect()
}

// ─── RD-01 ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rd01_annuler_puis_relire_le_fil_donne_deux_faits() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (_, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 2).await;
    let lea = cast.parent_lea.user_id;
    ent.reserver_creneau(etab, lea, creneaux[0], cast.eleve_lea_id)
        .await
        .unwrap();
    ent.annuler_reservation(etab, lea, "parent", creneaux[0], None)
        .await
        .unwrap();
    let f = faits(&ent, &cast, creneaux[0]).await;
    let noms: Vec<&str> = f.iter().map(|x| x.fait.as_str()).collect();
    assert_eq!(noms, ["reserve", "annule_famille"]);
    // Le créneau est libre, mais le fil garde qui avait réservé et pour quel enfant.
    assert_eq!(f[1].reserve_par_user_id, Some(lea));
    assert_eq!(f[1].eleve_id, Some(cast.eleve_lea_id));
    assert_eq!(f[1].par_user_id, Some(lea));
    // Le fil est en ajout seul.
    let maj = sqlx::query("UPDATE ent_rdv_fait SET fait = 'honore' WHERE creneau_id = $1")
        .bind(creneaux[0])
        .execute(ent.pool_verif())
        .await;
    assert!(maj.is_err(), "UPDATE du fil refusé");
    let sup = sqlx::query("DELETE FROM ent_rdv_fait WHERE creneau_id = $1")
        .bind(creneaux[0])
        .execute(ent.pool_verif())
        .await;
    assert!(sup.is_err(), "DELETE du fil refusé");
    // Le fil n'est lisible que de l'organisateur et de l'enseignant du créneau.
    assert!(ent
        .fil_creneau(etab, lea, "parent", creneaux[0])
        .await
        .is_err());
    reset(&ent, etab).await;
}

// ─── RD-02 ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rd02_creneau_passe_ou_proche_non_reservable_et_annulation_tardive_refusee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    let (_, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 3).await;
    // Créneau passé (hier) et créneau dans 3 h : refusés avec un conflit.
    sqlx::query("UPDATE ent_creneau_rdv SET debut = now() - interval '1 day', fin = now() - interval '1 day' + interval '10 minutes' WHERE id = $1")
        .bind(creneaux[0])
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_creneau_rdv SET debut = now() + interval '3 hours', fin = now() + interval '3 hours 10 minutes' WHERE id = $1")
        .bind(creneaux[1])
        .execute(ent.pool_verif())
        .await
        .unwrap();
    for c in [creneaux[0], creneaux[1]] {
        let r = ent.reserver_creneau(etab, lea, c, cast.eleve_lea_id).await;
        assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    }
    // Annulation à H-1 : refusée pour la famille, permise à l'organisateur (avec motif).
    ent.reserver_creneau(etab, lea, creneaux[2], cast.eleve_lea_id)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_creneau_rdv SET debut = now() + interval '1 hour', fin = now() + interval '70 minutes' WHERE id = $1")
        .bind(creneaux[2])
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let r = ent
        .annuler_reservation(etab, lea, "parent", creneaux[2], None)
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    let (adm, role) = dir(&cast);
    ent.annuler_reservation(
        etab,
        adm,
        role,
        creneaux[2],
        Some("absence imprévue de l'enseignant"),
    )
    .await
    .unwrap();
    // Rencontre close : la famille n'annule plus (l'organisateur garde la main).
    let (renc, creneaux) =
        rencontre_ouverte(&ent, &cast, dans_trois_jours_9h() + Duration::days(1), 1).await;
    ent.reserver_creneau(etab, lea, creneaux[0], cast.eleve_lea_id)
        .await
        .unwrap();
    ent.cloturer_rencontre(etab, adm, role, renc).await.unwrap();
    let r = ent
        .annuler_reservation(etab, lea, "parent", creneaux[0], None)
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    reset(&ent, etab).await;
}

// ─── RD-03 ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rd03_second_parent_du_meme_foyer_voit_le_rendez_vous_sans_le_nom() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    let second = second_parent_de_lea(&ent, &cast).await;
    let (renc, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 2).await;
    ent.reserver_creneau(etab, lea, creneaux[0], cast.eleve_lea_id)
        .await
        .unwrap();
    let vue = ent
        .creneaux_famille(etab, second, renc, cast.eleve_lea_id)
        .await
        .unwrap();
    let etat = |id: Uuid| {
        vue.iter()
            .flat_map(|p| p.creneaux.iter())
            .find(|c| c.id == id)
            .unwrap()
            .etat
            .clone()
    };
    assert_eq!(
        etat(creneaux[0]),
        "reserve_pour_cet_enfant",
        "déjà pris pour Léa"
    );
    assert_eq!(etat(creneaux[1]), "libre");
    // Il est prévenu, sans le nom du responsable qui a réservé.
    let avis = corps_notifications(&ent, etab, second, "rdv_pris").await;
    assert_eq!(avis.len(), 1, "le second parent est prévenu");
    let (nom_lea,): (String,) = sqlx::query_as("SELECT display_name FROM ent_user WHERE id = $1")
        .bind(lea)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(!avis[0].contains(&nom_lea), "{}", avis[0]);
    // Et il ne peut pas prendre un autre rendez-vous avec le même enseignant pour cet enfant.
    let r = ent
        .reserver_creneau(etab, second, creneaux[1], cast.eleve_lea_id)
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    // Le réservant, lui, voit toujours « réservé par moi ».
    let moi = ent
        .creneaux_famille(etab, lea, renc, cast.eleve_lea_id)
        .await
        .unwrap();
    assert!(moi
        .iter()
        .flat_map(|p| &p.creneaux)
        .any(|c| c.id == creneaux[0] && c.etat == "reserve_par_moi"));
    reset(&ent, etab).await;
}

#[tokio::test]
async fn rd03_interdiction_ou_adresse_protegee_ne_revele_rien_au_second_parent() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    let second = second_parent_de_lea(&ent, &cast).await;
    let (renc, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 2).await;
    // Interdiction de visite visant le premier parent (il reste sollicitable : seule la
    // « communication » le retire).
    let (personne,): (Uuid,) = sqlx::query_as(
        "SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
    )
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(lea)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1,$2,$3,$4,ARRAY['visite'],'decision_justice','ag0-test', ent_aujourdhui() - 1, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(personne)
    .bind(cast.direction.user_id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    ent.reserver_creneau(etab, lea, creneaux[0], cast.eleve_lea_id)
        .await
        .unwrap();
    let vue = ent
        .creneaux_famille(etab, second, renc, cast.eleve_lea_id)
        .await
        .unwrap();
    let etat = vue
        .iter()
        .flat_map(|p| &p.creneaux)
        .find(|c| c.id == creneaux[0])
        .unwrap()
        .etat
        .clone();
    assert_eq!(
        etat, "indisponible",
        "sous interdiction : aucune information"
    );
    assert!(
        corps_notifications(&ent, etab, second, "rdv_pris")
            .await
            .is_empty(),
        "et aucun avis"
    );
    reset(&ent, etab).await;
    // Adresse protégée du premier parent : même silence.
    let (renc, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 2).await;
    sqlx::query(
        "INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine)
         VALUES ($1,$2,$3,'enc:v1:test','saisie')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(personne)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    ent.reserver_creneau(etab, lea, creneaux[0], cast.eleve_lea_id)
        .await
        .unwrap();
    let vue = ent
        .creneaux_famille(etab, second, renc, cast.eleve_lea_id)
        .await
        .unwrap();
    let etat = vue
        .iter()
        .flat_map(|p| &p.creneaux)
        .find(|c| c.id == creneaux[0])
        .unwrap()
        .etat
        .clone();
    assert_eq!(
        etat, "indisponible",
        "adresse protégée : aucune information"
    );
    sqlx::query(
        "DELETE FROM ent_adresse_protegee WHERE etablissement_id = $1 AND personne_id = $2",
    )
    .bind(etab)
    .bind(personne)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    reset(&ent, etab).await;
}

// ─── RD-04 ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rd04_deux_responsables_du_meme_enfant_en_parallele_un_seul_reussit() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    let second = second_parent_de_lea(&ent, &cast).await;
    let (_, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 2).await;
    let (a, b) = tokio::join!(
        ent.reserver_creneau(etab, lea, creneaux[0], cast.eleve_lea_id),
        ent.reserver_creneau(etab, second, creneaux[1], cast.eleve_lea_id),
    );
    let (ok, ko) = (
        a.is_ok() as u8 + b.is_ok() as u8,
        [&a, &b].iter().filter(|r| r.is_err()).count(),
    );
    assert_eq!((ok, ko), (1, 1), "{a:?} / {b:?}");
    for r in [a, b] {
        if let Err(e) = r {
            assert!(
                matches!(e, EntDbError::ConflitDossier(_)),
                "conflit lisible, pas une erreur technique : {e:?}"
            );
        }
    }
    reset(&ent, etab).await;
}

// ─── RD-05 ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rd05_lien_rompu_annulation_refusee_et_rendez_vous_a_revoir() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    let (_, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 1).await;
    ent.reserver_creneau(etab, lea, creneaux[0], cast.eleve_lea_id)
        .await
        .unwrap();
    // Le lien est suspendu (fin d'autorité, départ, décision) : le responsable n'annule plus.
    sqlx::query(
        "UPDATE ent_eleve_lien SET suspendu_le = now(), suspendu_par = $4, suspension_motif = 'test'
          WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
    )
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(lea)
    .bind(cast.direction.user_id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let r = ent
        .annuler_reservation(etab, lea, "parent", creneaux[0], None)
        .await;
    assert!(
        matches!(
            r,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        ),
        "404 : {r:?}"
    );
    // Réaction : rendez-vous « à revoir », une seule fois, enseignant prévenu.
    assert_eq!(
        ent.reviser_rdv_liens_rompus(Utc::now(), Some(etab))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        ent.reviser_rdv_liens_rompus(Utc::now(), Some(etab))
            .await
            .unwrap(),
        0,
        "idempotent"
    );
    let f = faits(&ent, &cast, creneaux[0]).await;
    assert_eq!(f.iter().filter(|x| x.fait == "a_revoir").count(), 1);
    assert!(
        !corps_notifications(&ent, etab, cast.prof.user_id, "rdv_a_revoir")
            .await
            .is_empty()
    );
    // Le rendez-vous reste en place (c'est à l'organisateur de le revoir).
    let (statut,): (String,) = sqlx::query_as("SELECT statut FROM ent_creneau_rdv WHERE id = $1")
        .bind(creneaux[0])
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(statut, "reserve");
    reset(&ent, etab).await;
}

// ─── RD-06 ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rd06_rappels_emis_sans_contexte_d_etablissement_et_traces_au_fil() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    let (_, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 1).await;
    ent.reserver_creneau(etab, lea, creneaux[0], cast.eleve_lea_id)
        .await
        .unwrap();
    // Le job planifié (main.rs) passe `None` : un passage par établissement, chacun dans son
    // contexte (en RLS stricte, une lecture sans contexte ne voit aucune ligne).
    let t = dans_trois_jours_9h() - Duration::hours(2);
    let n = ent
        .emettre_rappels_rdv(t, Duration::hours(24), None)
        .await
        .unwrap();
    assert!(n >= 1, "rappel émis : {n}");
    assert!(!corps_notifications(&ent, etab, lea, "rdv_rappel")
        .await
        .is_empty());
    let f = faits(&ent, &cast, creneaux[0]).await;
    assert!(
        f.iter().any(|x| x.fait == "rappel"),
        "le rappel est inscrit au fil"
    );
    assert_eq!(
        ent.emettre_rappels_rdv(t, Duration::hours(24), None)
            .await
            .unwrap(),
        0,
        "un seul rappel"
    );
    reset(&ent, etab).await;
}

// ─── RD-07 ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rd07_heure_de_l_avis_dans_le_fuseau_de_l_etablissement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    sqlx::query(
        "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, 'Indian/Reunion')
         ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = 'Indian/Reunion'",
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    // 09:00 UTC = 13:00 à La Réunion (UTC+4, sans heure d'été) ; Paris aurait affiché 10:00 ou 11:00.
    let (_, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 1).await;
    ent.reserver_creneau(etab, lea, creneaux[0], cast.eleve_lea_id)
        .await
        .unwrap();
    let avis = corps_notifications(&ent, etab, lea, "rdv_pris").await;
    assert!(avis.iter().any(|c| c.contains("à 13:00")), "{avis:?}");
    reset(&ent, etab).await;
}

// ─── RD-08 ───────────────────────────────────────────────────────────────────────────────────

#[test]
fn rd08_sms_seulement_pour_une_annulation_par_l_etablissement_le_jour_meme() {
    use crate::rencontres::sms_autorise;
    assert!(!sms_autorise("rdv_pris", false, Duration::days(3)));
    assert!(!sms_autorise("rdv_pris", true, Duration::hours(2)));
    assert!(!sms_autorise("rdv_rappel", false, Duration::hours(2)));
    assert!(
        !sms_autorise("rdv_annule", false, Duration::hours(2)),
        "annulation par la famille"
    );
    assert!(
        !sms_autorise("rdv_annule", true, Duration::days(3)),
        "annulation par l'établissement, loin"
    );
    assert!(sms_autorise("rdv_annule", true, Duration::hours(5)));
}

// ─── RD-09 ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rd09_annulation_par_l_organisateur_exige_un_motif_repris_dans_l_avis() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let lea = cast.parent_lea.user_id;
    let (_, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 1).await;
    ent.reserver_creneau(etab, lea, creneaux[0], cast.eleve_lea_id)
        .await
        .unwrap();
    for vide in [None, Some(""), Some("   ")] {
        let r = ent
            .annuler_reservation(etab, adm, role, creneaux[0], vide)
            .await;
        assert!(
            matches!(r, Err(EntDbError::SaisieInvalide(_))),
            "{vide:?} : {r:?}"
        );
    }
    ent.annuler_reservation(
        etab,
        adm,
        role,
        creneaux[0],
        Some("réunion pédagogique déplacée"),
    )
    .await
    .unwrap();
    let avis = corps_notifications(&ent, etab, lea, "rdv_annule").await;
    assert!(
        avis.iter()
            .any(|c| c.contains("réunion pédagogique déplacée")),
        "{avis:?}"
    );
    let f = faits(&ent, &cast, creneaux[0]).await;
    assert_eq!(f.last().unwrap().fait, "annule_organisateur");
    assert_eq!(
        f.last().unwrap().motif_public.as_deref(),
        Some("réunion pédagogique déplacée")
    );
    reset(&ent, etab).await;
}

// ─── RD-10 ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rd10_enseignant_bloque_son_creneau_et_saisit_la_presence() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    let prof = (cast.prof.user_id, Role::Prof.as_str());
    let (renc, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 2).await;
    // Un autre enseignant (ou la famille) ne touche pas au créneau : 404.
    let autre = cast.cpe.user_id;
    assert!(ent
        .bloquer_creneau(etab, autre, Role::Cpe.as_str(), creneaux[0], true)
        .await
        .is_err());
    assert!(ent
        .bloquer_creneau(etab, lea, "parent", creneaux[0], true)
        .await
        .is_err());
    // L'enseignant bloque : le créneau n'est plus réservable et la famille le voit « indisponible ».
    let l = ent
        .bloquer_creneau(etab, prof.0, prof.1, creneaux[0], true)
        .await
        .unwrap();
    assert_eq!(l.statut, "bloque");
    let r = ent
        .reserver_creneau(etab, lea, creneaux[0], cast.eleve_lea_id)
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    let vue = ent
        .creneaux_famille(etab, lea, renc, cast.eleve_lea_id)
        .await
        .unwrap();
    assert!(vue
        .iter()
        .flat_map(|p| &p.creneaux)
        .any(|c| c.id == creneaux[0] && c.etat == "indisponible"));
    // Débloquer puis réserver l'autre créneau.
    let l = ent
        .bloquer_creneau(etab, prof.0, prof.1, creneaux[0], false)
        .await
        .unwrap();
    assert_eq!(l.statut, "libre");
    ent.reserver_creneau(etab, lea, creneaux[1], cast.eleve_lea_id)
        .await
        .unwrap();
    // On ne bloque pas un créneau réservé.
    assert!(matches!(
        ent.bloquer_creneau(etab, prof.0, prof.1, creneaux[1], true)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Présence : pas avant le début, puis « honoré » ; seul l'enseignant du créneau.
    let r = ent
        .marquer_presence_rdv(etab, prof.0, creneaux[1], "honore")
        .await;
    assert!(
        matches!(r, Err(EntDbError::ConflitDossier(_))),
        "pas encore commencé : {r:?}"
    );
    sqlx::query("UPDATE ent_creneau_rdv SET debut = now() - interval '20 minutes', fin = now() - interval '10 minutes' WHERE id = $1")
        .bind(creneaux[1])
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert!(ent
        .marquer_presence_rdv(etab, autre, creneaux[1], "honore")
        .await
        .is_err());
    assert!(ent
        .marquer_presence_rdv(etab, prof.0, creneaux[1], "peut-etre")
        .await
        .is_err());
    let l = ent
        .marquer_presence_rdv(etab, prof.0, creneaux[1], "non_venu")
        .await
        .unwrap();
    assert_eq!(l.presence.as_deref(), Some("non_venu"));
    let f = faits(&ent, &cast, creneaux[1]).await;
    assert_eq!(f.last().unwrap().fait, "non_venu");
    reset(&ent, etab).await;
}

// ─── RD-11 ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rd11_cles_composites_et_aucune_suppression_en_cascade() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (renc, creneaux) = rencontre_ouverte(&ent, &cast, dans_trois_jours_9h(), 1).await;
    for (table, nom) in [
        ("ent_creneau_rdv", "ent_creneau_rdv_rencontre_fk"),
        ("ent_creneau_rdv", "ent_creneau_rdv_eleve_fk"),
        ("ent_rencontre", "ent_rencontre_classe_fk"),
        ("ent_rdv_fait", "ent_rdv_fait_creneau_fk"),
        ("ent_rdv_fait", "ent_rdv_fait_eleve_fk"),
    ] {
        let (n_colonnes,): (i32,) = sqlx::query_as(
            "SELECT array_length(conkey, 1) FROM pg_constraint WHERE conrelid = $1::regclass AND conname = $2",
        )
        .bind(table)
        .bind(nom)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap_or_else(|e| panic!("{nom} absente : {e}"));
        assert_eq!(n_colonnes, 2, "{nom} : clé composite (établissement, …)");
    }
    let (suppr,): (String,) = sqlx::query_as(
        "SELECT confdeltype::text FROM pg_constraint WHERE conname = 'ent_creneau_rdv_rencontre_fk'",
    )
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(suppr, "r", "ON DELETE RESTRICT");
    // Supprimer une rencontre qui a des créneaux est refusé ; supprimer un créneau qui a un fait aussi.
    let r = sqlx::query("DELETE FROM ent_rencontre WHERE id = $1")
        .bind(renc)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "la rencontre garde ses créneaux");
    ent.reserver_creneau(
        etab,
        cast.parent_lea.user_id,
        creneaux[0],
        cast.eleve_lea_id,
    )
    .await
    .unwrap();
    let r = sqlx::query("DELETE FROM ent_creneau_rdv WHERE id = $1")
        .bind(creneaux[0])
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "le créneau garde son fil");
    // L'élève d'un créneau doit exister dans l'établissement.
    let r = sqlx::query("UPDATE ent_creneau_rdv SET eleve_id = $2 WHERE id = $1")
        .bind(creneaux[0])
        .bind(Uuid::new_v4())
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "élève inconnu refusé");
    reset(&ent, etab).await;
}
