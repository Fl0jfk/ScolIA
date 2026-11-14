//! Socle 29, lot P4 : files des personnels (`docs/socles/29_PORTAILS.md` §8.4, §18.3, §29.4). Base
//! locale de test (feature `labo`).
//!
//! Critères de fin : **deux CPE ne traitent jamais la même tâche** et **aucune tâche orpheline**.
//!
//! Chaque test ne vérifie que ses propres lignes (tâches retrouvées par l'objet qu'il a créé) et ne
//! lit jamais l'horloge réelle : les instants sont fixes (`t0`), les gestes les reçoivent en paramètre.
//! Un témoin précède chaque mesure de protection : un refus n'est une preuve que si le témoin passe.

use super::*;
use crate::afaire::{
    self, Destinataire, OuvertureTache, FILE_DECISIONS, FILE_JUSTIFICATIFS, FILE_PROTECTIONS,
};
use crate::afaire_files::{file_declaree, FILES};
use crate::delegation::{DelegationSaisie, DroitDelegue};
use crate::droits_repo::{Droit, RoleSaisie};
use crate::ref_commun::Acteur;
use chrono::{DateTime, Duration, TimeZone, Utc};
use ent_auth::{Action, Portee};
use ent_types::afaire::{EtatAfaire, NiveauPrise, MESSAGE_DEJA_PRISE, MESSAGE_DEJA_TRAITEE};
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
    })
    .await;
    Some((ent, cast))
}

/// Instant fixe (mercredi, 10 h UTC) : aucun test ne dépend de l'heure de la CI. Assez lointain pour
/// que les lignes posées avec l'horloge de la base (`visible_a_partir_de`) soient déjà visibles.
fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2031, 3, 12, 10, 0, 0).unwrap()
}

fn eleve<'a>(c: &'a LaboCast, prenom: &str) -> &'a LaboEleve {
    c.eleves
        .iter()
        .find(|e| e.prenom == prenom)
        .expect("élève du cast")
}

fn deja_prise<T: std::fmt::Debug>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::ConflitDossier(m)) if *m == MESSAGE_DEJA_PRISE)
}

fn introuvable<T: std::fmt::Debug>(r: &Result<T, EntDbError>) -> bool {
    matches!(
        r,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    )
}

fn interdit<T: std::fmt::Debug>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

/// Un justificatif à trancher qui n'appartient qu'à ce test : absence propre, justificatif soumis,
/// puis rapprochement ; retourne (absence, tâche de la file des justificatifs).
async fn tache_justificatif(ent: &PgEnt, c: &LaboCast) -> (Uuid, Uuid) {
    let etab = c.etablissement_id;
    let absence = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_absence (id, etablissement_id, eleve_id, debut, fin, motif, statut, motif_justif)
         VALUES ($1, $2, $3, '2031-03-10', '2031-03-10', (SELECT code FROM ent_absence_motif LIMIT 1), 'a_traiter', 'rdv')",
    )
    .bind(absence)
    .bind(etab)
    .bind(eleve(c, "Harry").id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    ent.rapprocher_afaire(etab, t0()).await.unwrap();
    let (tache,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_afaire WHERE etablissement_id = $1 AND objet_id = $2
            AND nature_code = 'vs.trancher_justificatif'",
    )
    .bind(etab)
    .bind(absence)
    .fetch_one(ent.pool_verif())
    .await
    .expect("tâche du justificatif");
    (absence, tache)
}

async fn etat_en_base(ent: &PgEnt, etab: Uuid, tache: Uuid) -> String {
    let (e,): (String,) =
        sqlx::query_as("SELECT etat FROM ent_afaire WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(tache)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    e
}

async fn prises_actives_en_base(ent: &PgEnt, etab: Uuid, tache: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_afaire_prise WHERE etablissement_id = $1 AND afaire_id = $2 AND relachee_le IS NULL",
    )
    .bind(etab)
    .bind(tache)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    n
}

async fn dans_la_file(
    ent: &PgEnt,
    etab: Uuid,
    p: &LaboPersona,
    code: &str,
    tache: Uuid,
) -> Option<ent_types::afaire::TacheFile> {
    ent.file_afaire(etab, p.user_id, p.role.as_str(), code, t0())
        .await
        .unwrap_or_else(|e| panic!("file de {:?} : {e:?}", p.role))
        .taches
        .into_iter()
        .find(|t| t.id == tache)
}

// ─────────────────────────── Déclarations (sans base) ───────────────────────────

#[test]
fn p4_les_files_declarees_couvrent_les_natures_et_ne_sont_pas_ouvertes_a_tous() {
    crate::afaire_files::verifier_files().expect("files déclarées");
    for f in FILES {
        // Aucune file ne s'ouvre à « tout le personnel » : un droit précis, ou le profil de la vie scolaire.
        assert!(
            matches!(
                f.decider,
                crate::afaire_files::DroitFile::Justificatifs
                    | crate::afaire_files::DroitFile::Action(_)
            ),
            "{}",
            f.code
        );
    }
    assert!(file_declaree(FILE_PROTECTIONS).unwrap().deux_niveaux());
    assert!(!file_declaree(FILE_JUSTIFICATIFS).unwrap().deux_niveaux());
}

// ─────────────────────────── Critère 1 : deux CPE, une seule tâche ───────────────────────────

#[tokio::test]
async fn p4_deux_cpe_ne_traitent_jamais_la_meme_tache() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (cpe1, cpe2) = (&c.cpes[0], &c.cpes[1]);
    let (_, tache) = tache_justificatif(&ent, &c).await;

    // Témoin : les deux CPE voient la tâche dans la file, et dans « Mes files ».
    assert!(
        dans_la_file(&ent, etab, cpe1, FILE_JUSTIFICATIFS, tache)
            .await
            .is_some(),
        "témoin cpe1"
    );
    assert!(
        dans_la_file(&ent, etab, cpe2, FILE_JUSTIFICATIFS, tache)
            .await
            .is_some(),
        "témoin cpe2"
    );
    let tuile = |m: &ent_types::afaire::MesFiles| {
        m.tuiles
            .iter()
            .find(|t| t.code == FILE_JUSTIFICATIFS)
            .cloned()
            .expect("tuile des justificatifs")
    };
    let avant1 = tuile(
        &ent.mes_files(etab, cpe1.user_id, "cpe", t0())
            .await
            .unwrap(),
    );
    let avant2 = tuile(
        &ent.mes_files(etab, cpe2.user_id, "cpe", t0())
            .await
            .unwrap(),
    );
    assert_eq!((avant1.prises_par_moi, avant2.prises_par_autres), (0, 0));

    // La première prise gagne.
    ent.afaire_prendre(etab, cpe1.user_id, "cpe", tache, NiveauPrise::Decider, t0())
        .await
        .unwrap();
    // La seconde reçoit « déjà prise », jamais le nom du collègue dans l'erreur.
    let r = ent
        .afaire_prendre(etab, cpe2.user_id, "cpe", tache, NiveauPrise::Decider, t0())
        .await;
    assert!(deja_prise(&r), "seconde prise refusée");
    assert_eq!(
        prises_actives_en_base(&ent, etab, tache).await,
        1,
        "une seule prise en base"
    );
    assert_eq!(etat_en_base(&ent, etab, tache).await, "prise");
    // Reprendre sa propre prise (double clic) est sans effet.
    ent.afaire_prendre(etab, cpe1.user_id, "cpe", tache, NiveauPrise::Decider, t0())
        .await
        .unwrap();
    assert_eq!(prises_actives_en_base(&ent, etab, tache).await, 1);

    // Qui a pris quoi (POR-155) : « prénom + initiale », jamais l'adresse.
    let vue1 = dans_la_file(&ent, etab, cpe1, FILE_JUSTIFICATIFS, tache)
        .await
        .unwrap();
    let vue2 = dans_la_file(&ent, etab, cpe2, FILE_JUSTIFICATIFS, tache)
        .await
        .unwrap();
    assert_eq!(vue1.etat, EtatAfaire::Prise);
    assert!(vue1.prises[0].par_moi && !vue2.prises[0].par_moi);
    assert_eq!(vue2.prises[0].par, "Remus L.");
    assert!(!format!("{vue2:?}").contains('@'), "aucune adresse");

    // Mon à-faire : ce qu'un collègue traite n'y est plus ; la mienne y reste.
    let a1 = ent
        .a_faire_de(etab, cpe1.user_id, "cpe", t0())
        .await
        .unwrap();
    let a2 = ent
        .a_faire_de(etab, cpe2.user_id, "cpe", t0())
        .await
        .unwrap();
    assert!(
        a1.taches.iter().any(|t| t.id == tache),
        "témoin : la mienne reste"
    );
    assert!(
        a2.taches.iter().all(|t| t.id != tache),
        "la tâche d'un collègue n'est plus dans mon à-faire"
    );
    // Les tuiles suivent : une prise de moi chez l'un, une prise d'un collègue chez l'autre.
    let apres1 = tuile(
        &ent.mes_files(etab, cpe1.user_id, "cpe", t0())
            .await
            .unwrap(),
    );
    let apres2 = tuile(
        &ent.mes_files(etab, cpe2.user_id, "cpe", t0())
            .await
            .unwrap(),
    );
    assert_eq!(apres1.prises_par_moi, 1);
    assert_eq!(apres2.prises_par_autres, avant2.prises_par_autres + 1);
    // Le second ne peut pas agir sur la tâche (aperçu) : « déjà prise », pas un 404.
    let r = ent
        .afaire_apercu(etab, cpe2.user_id, "cpe", tache, t0())
        .await;
    assert!(deja_prise(&r), "{r:?}");

    // Le second ne relâche pas la prise du premier.
    let r = ent
        .afaire_relacher(etab, cpe2.user_id, "cpe", tache, t0())
        .await;
    assert!(interdit(&r), "{r:?}");
    assert_eq!(prises_actives_en_base(&ent, etab, tache).await, 1);
    // Le premier relâche : la tâche est de nouveau libre, le second la prend.
    ent.afaire_relacher(etab, cpe1.user_id, "cpe", tache, t0())
        .await
        .unwrap();
    assert_eq!(etat_en_base(&ent, etab, tache).await, "ouverte");
    ent.afaire_prendre(etab, cpe2.user_id, "cpe", tache, NiveauPrise::Decider, t0())
        .await
        .unwrap();
    assert!(deja_prise(
        &ent.afaire_prendre(etab, cpe1.user_id, "cpe", tache, NiveauPrise::Decider, t0())
            .await
    ));
}

#[tokio::test]
async fn p4_deux_prises_simultanees_une_seule_gagne() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (cpe1, cpe2) = (&c.cpes[0], &c.cpes[1]);
    let (_, tache) = tache_justificatif(&ent, &c).await;
    // CPO-36 : « deux CPE prennent la même tâche en même temps » — la contrainte en base tranche.
    let (r1, r2) = tokio::join!(
        ent.afaire_prendre(etab, cpe1.user_id, "cpe", tache, NiveauPrise::Decider, t0()),
        ent.afaire_prendre(etab, cpe2.user_id, "cpe", tache, NiveauPrise::Decider, t0()),
    );
    let (gagnants, perdants): (Vec<_>, Vec<_>) = [r1, r2].into_iter().partition(|r| r.is_ok());
    assert_eq!(gagnants.len(), 1, "exactement une prise");
    assert!(deja_prise(&Err::<(), _>(
        perdants.into_iter().next().unwrap().unwrap_err()
    )));
    assert_eq!(prises_actives_en_base(&ent, etab, tache).await, 1);
}

#[tokio::test]
async fn p4_membres_calcules_jamais_tout_le_personnel() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (_, tache) = tache_justificatif(&ent, &c).await;
    // Témoin : un membre prend.
    ent.afaire_prendre(
        etab,
        c.cpe.user_id,
        "cpe",
        tache,
        NiveauPrise::Decider,
        t0(),
    )
    .await
    .unwrap();
    // Un professeur, une infirmière, une famille : pas membres de la file → même réponse « introuvable ».
    let infirmiere = &c.infirmieres[0];
    for (p, role) in [
        (&c.prof, "prof"),
        (infirmiere, "infirmerie"),
        (&c.compta, "compta"),
    ] {
        assert!(
            introuvable(
                &ent.file_afaire(etab, p.user_id, role, FILE_JUSTIFICATIFS, t0())
                    .await
            ),
            "{role}"
        );
        assert!(
            introuvable(
                &ent.afaire_prendre(etab, p.user_id, role, tache, NiveauPrise::Decider, t0())
                    .await
            ),
            "{role}"
        );
    }
    // Une famille n'accède à aucune file (« Mes files » est un écran du personnel).
    let r = ent
        .mes_files(etab, c.parent_lea.user_id, "parent", t0())
        .await;
    assert!(interdit(&r), "{r:?}");
    // Une file qui n'existe pas se comporte comme une file interdite (pas d'énumération).
    assert!(introuvable(
        &ent.file_afaire(etab, c.cpe.user_id, "cpe", "n.importe.quoi", t0())
            .await
    ));
    // La file de la direction n'est pas celle de la vie scolaire.
    assert!(introuvable(
        &ent.file_afaire(etab, c.cpe.user_id, "cpe", FILE_DECISIONS, t0())
            .await
    ));
    // Une tâche inconnue : introuvable, jamais « déjà prise ».
    assert!(introuvable(
        &ent.afaire_prendre(
            etab,
            c.cpe.user_id,
            "cpe",
            Uuid::new_v4(),
            NiveauPrise::Decider,
            t0()
        )
        .await
    ));
}

// ─────────────────────────── Expiration et clôture ───────────────────────────

#[tokio::test]
async fn p4_une_prise_expire_et_le_rapprochement_la_libere() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (cpe1, cpe2) = (&c.cpes[0], &c.cpes[1]);
    let (_, tache) = tache_justificatif(&ent, &c).await;
    ent.afaire_prendre(etab, cpe1.user_id, "cpe", tache, NiveauPrise::Decider, t0())
        .await
        .unwrap();
    // Un jour plus tard (S-2901 : deux jours ouvrés) la prise tient encore.
    let r = ent
        .afaire_prendre(
            etab,
            cpe2.user_id,
            "cpe",
            tache,
            NiveauPrise::Decider,
            t0() + Duration::days(1),
        )
        .await;
    assert!(deja_prise(&r), "prise encore active");
    // Cinq jours plus tard elle est échue : la lecture ne la montre plus…
    let tard = t0() + Duration::days(5);
    let vue = ent
        .file_afaire(etab, cpe2.user_id, "cpe", FILE_JUSTIFICATIFS, tard)
        .await
        .unwrap();
    let t = vue
        .taches
        .iter()
        .find(|t| t.id == tache)
        .expect("la tâche est de nouveau libre");
    assert!(t.prises.is_empty() && t.etat == EtatAfaire::Ouverte);
    // … et le rapprochement la libère pour de bon (CPO-37 : un CPE en congé avec des tâches prises).
    let rap = ent.rapprocher_afaire(etab, tard).await.unwrap();
    assert!(rap.prises_liberees >= 1, "{rap:?}");
    assert_eq!(prises_actives_en_base(&ent, etab, tache).await, 0);
    assert_eq!(etat_en_base(&ent, etab, tache).await, "ouverte");
    ent.afaire_prendre(etab, cpe2.user_id, "cpe", tache, NiveauPrise::Decider, tard)
        .await
        .unwrap();
    // Le journal garde la trace sans contenu : codes seulement.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_afaire_journal WHERE etablissement_id = $1 AND afaire_id = $2
            AND motif_code IN ('prise', 'prise_expiree')",
    )
    .bind(etab)
    .bind(tache)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(n >= 3, "prise, expiration, nouvelle prise : {n}");
}

#[tokio::test]
async fn p4_la_cloture_par_le_proprietaire_libere_la_prise() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (absence, tache) = tache_justificatif(&ent, &c).await;
    ent.afaire_prendre(
        etab,
        c.cpe.user_id,
        "cpe",
        tache,
        NiveauPrise::Decider,
        t0(),
    )
    .await
    .unwrap();
    // La commande du propriétaire (décision de la vie scolaire) retire le justificatif de sa file.
    ent.accepter_justificatif_for_actor(etab, c.cpe.user_id, "cpe", absence, None)
        .await
        .unwrap();
    // Le propriétaire ne la dit plus ouverte : le rapprochement la ferme (instants fixes, aucun délai d'outbox).
    ent.rapprocher_afaire(etab, t0() + Duration::hours(1))
        .await
        .unwrap();
    assert_eq!(etat_en_base(&ent, etab, tache).await, "sans_objet");
    assert_eq!(
        prises_actives_en_base(&ent, etab, tache).await,
        0,
        "plus de prise active sur une tâche close"
    );
    // Une tâche close n'est plus prenable : « déjà traitée ».
    let r = ent
        .afaire_prendre(
            etab,
            c.cpes[1].user_id,
            "cpe",
            tache,
            NiveauPrise::Decider,
            t0(),
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::ConflitDossier(m)) if m == MESSAGE_DEJA_TRAITEE),
        "{r:?}"
    );
}

// ─────────────────────────── Réattribution ───────────────────────────

#[tokio::test]
async fn p4_reattribution_par_le_responsable_vers_un_collegue_de_la_file() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (cpe1, cpe2) = (&c.cpes[0], &c.cpes[1]);
    let (_, tache) = tache_justificatif(&ent, &c).await;
    ent.afaire_prendre(etab, cpe1.user_id, "cpe", tache, NiveauPrise::Decider, t0())
        .await
        .unwrap();

    // Un CPE n'est pas responsable de service : refus (le droit `afaire.file.reattribuer` manque).
    let r = ent
        .afaire_reattribuer(
            etab,
            cpe2.user_id,
            "cpe",
            tache,
            cpe2.user_id,
            NiveauPrise::Decider,
            t0(),
        )
        .await;
    assert!(interdit(&r), "{r:?}");
    assert!(
        dans_la_file(&ent, etab, cpe1, FILE_JUSTIFICATIFS, tache)
            .await
            .unwrap()
            .prises[0]
            .par_moi,
        "témoin : rien n'a bougé"
    );

    // La direction réattribue : à un collègue de la file…
    let dir = &c.direction;
    let vue = ent
        .file_afaire(etab, dir.user_id, "direction", FILE_JUSTIFICATIFS, t0())
        .await
        .unwrap();
    assert!(vue.peut_reattribuer);
    assert!(
        vue.collegues.iter().any(|x| x.compte_id == cpe2.user_id),
        "le CPE est proposé"
    );
    assert!(
        vue.collegues.iter().all(|x| x.compte_id != c.prof.user_id),
        "jamais un non-membre"
    );
    ent.afaire_reattribuer(
        etab,
        dir.user_id,
        "direction",
        tache,
        cpe2.user_id,
        NiveauPrise::Decider,
        t0(),
    )
    .await
    .unwrap();
    let v2 = dans_la_file(&ent, etab, cpe2, FILE_JUSTIFICATIFS, tache)
        .await
        .unwrap();
    assert!(
        v2.prises[0].par_moi,
        "la tâche est désormais celle du collègue"
    );
    let v1 = dans_la_file(&ent, etab, cpe1, FILE_JUSTIFICATIFS, tache)
        .await
        .unwrap();
    assert!(!v1.prises[0].par_moi);
    assert_eq!(
        prises_actives_en_base(&ent, etab, tache).await,
        1,
        "toujours une seule prise"
    );
    // … jamais à quelqu'un qui n'est pas de la file, ni à un compte inconnu.
    let r = ent
        .afaire_reattribuer(
            etab,
            dir.user_id,
            "direction",
            tache,
            c.prof.user_id,
            NiveauPrise::Decider,
            t0(),
        )
        .await;
    assert!(introuvable(&r), "{r:?}");
    let r = ent
        .afaire_reattribuer(
            etab,
            dir.user_id,
            "direction",
            tache,
            Uuid::new_v4(),
            NiveauPrise::Decider,
            t0(),
        )
        .await;
    assert!(introuvable(&r), "{r:?}");
    // Journalisé : qui a réattribué, jamais le nom du collègue.
    let (par, motif): (Option<Uuid>, String) = sqlx::query_as(
        "SELECT par_compte_id, motif_code FROM ent_afaire_journal
          WHERE etablissement_id = $1 AND afaire_id = $2 AND motif_code = 'reattribuee'",
    )
    .bind(etab)
    .bind(tache)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((par, motif.as_str()), (Some(dir.user_id), "reattribuee"));
}

// ─────────────────────────── Files à deux niveaux ───────────────────────────

#[tokio::test]
async fn p4_file_a_deux_niveaux_preparer_puis_decider() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let dir = &c.direction;
    let secretaire = &c.administratif;
    // Une protection à confirmer (adresse protégée à traiter) : objet propre à ce test.
    let (personne,): (Uuid,) =
        sqlx::query_as("SELECT id FROM ent_personne WHERE etablissement_id = $1 LIMIT 1")
            .bind(etab)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    let adresse = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine)
         VALUES ($1, $2, $3, 'enc:v1:x', 'saisie')",
    )
    .bind(adresse)
    .bind(etab)
    .bind(personne)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    ent.rapprocher_afaire(etab, t0()).await.unwrap();
    let (tache,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_afaire WHERE etablissement_id = $1 AND objet_id = $2 AND nature_code = 'dos.protection_a_confirmer'",
    )
    .bind(etab)
    .bind(adresse)
    .fetch_one(ent.pool_verif())
    .await
    .expect("tâche de la protection");

    // Avant son rôle de préparation, le secrétariat n'a pas accès à la file (membres calculés).
    assert!(introuvable(
        &ent.file_afaire(
            etab,
            secretaire.user_id,
            "administratif",
            FILE_PROTECTIONS,
            t0()
        )
        .await
    ));
    // Témoin : la direction la voit.
    assert!(
        dans_la_file(&ent, etab, dir, FILE_PROTECTIONS, tache)
            .await
            .is_some(),
        "témoin direction"
    );
    // Un rôle paramétré donne au secrétariat le seul droit de préparer (gérer les données protégées).
    let role = ent
        .creer_role_for_actor(
            etab,
            dir.user_id,
            "direction",
            RoleSaisie {
                nom: format!(
                    "Préparation protections {}",
                    &Uuid::new_v4().simple().to_string()[..8]
                ),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::ResponsablesProtegeGerer,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(
        etab,
        dir.user_id,
        "direction",
        role.id,
        secretaire.user_id,
        None,
    )
    .await
    .unwrap();

    let vue = ent
        .file_afaire(
            etab,
            secretaire.user_id,
            "administratif",
            FILE_PROTECTIONS,
            t0(),
        )
        .await
        .unwrap();
    assert!(vue.deux_niveaux && vue.taches.iter().any(|t| t.id == tache));
    // Le secrétariat prépare, il ne décide pas : une prise « décider » lui est refusée comme à un non-membre.
    let r = ent
        .afaire_prendre(
            etab,
            secretaire.user_id,
            "administratif",
            tache,
            NiveauPrise::Decider,
            t0(),
        )
        .await;
    assert!(introuvable(&r), "{r:?}");
    // La file des justificatifs, elle, n'a qu'un niveau : « préparer » n'y existe pas.
    let (_, tj) = tache_justificatif(&ent, &c).await;
    let r = ent
        .afaire_prendre(etab, c.cpe.user_id, "cpe", tj, NiveauPrise::Preparer, t0())
        .await;
    assert!(introuvable(&r), "{r:?}");

    ent.afaire_prendre(
        etab,
        secretaire.user_id,
        "administratif",
        tache,
        NiveauPrise::Preparer,
        t0(),
    )
    .await
    .unwrap();
    assert_eq!(etat_en_base(&ent, etab, tache).await, "prise");
    // Seul celui qui prépare remet la tâche : la direction n'a pas de prise « préparer ».
    let r = ent
        .afaire_preparee(etab, dir.user_id, "direction", tache, t0())
        .await;
    assert!(introuvable(&r), "{r:?}");
    ent.afaire_preparee(etab, secretaire.user_id, "administratif", tache, t0())
        .await
        .unwrap();
    // La tâche attend sa décision : « préparée par X ».
    assert_eq!(etat_en_base(&ent, etab, tache).await, "en_attente");
    let v = dans_la_file(&ent, etab, dir, FILE_PROTECTIONS, tache)
        .await
        .unwrap();
    assert_eq!(v.etat, EtatAfaire::EnAttente);
    assert!(
        v.prepare_par
            .as_deref()
            .is_some_and(|n| !n.is_empty() && !n.contains('@')),
        "{v:?}"
    );
    // La direction décide : sa prise passe la tâche à « prise », puis relâchée elle redevient « en attente ».
    ent.afaire_prendre(
        etab,
        dir.user_id,
        "direction",
        tache,
        NiveauPrise::Decider,
        t0(),
    )
    .await
    .unwrap();
    assert_eq!(etat_en_base(&ent, etab, tache).await, "prise");
    ent.afaire_relacher(etab, dir.user_id, "direction", tache, t0())
        .await
        .unwrap();
    assert_eq!(
        etat_en_base(&ent, etab, tache).await,
        "en_attente",
        "préparée : elle attend toujours sa décision"
    );
}

// ─────────────────────────── Délégations (socle 6) ───────────────────────────

#[tokio::test]
async fn p4_un_delegataire_rejoint_la_file_le_temps_de_la_delegation() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (_, tache) = tache_justificatif(&ent, &c).await;
    let delegue = &c.profs[3];
    // Sans délégation : le professeur n'est pas de la file (témoin négatif).
    assert!(introuvable(
        &ent.file_afaire(etab, delegue.user_id, "prof", FILE_JUSTIFICATIFS, t0())
            .await
    ));
    let d = Acteur {
        etablissement_id: etab,
        user_id: c.direction.user_id,
        role: "direction",
    };
    let fin = domain_eleve::horloge::jour_paris() + Duration::days(3);
    let deleg = ent
        .delegation_accorder(
            &d,
            &DelegationSaisie {
                delegue_user_id: delegue.user_id,
                droits: vec![DroitDelegue {
                    action: Action::AbsencesJustifier.code().into(),
                    portee: Portee::Etablissement.code().into(),
                    cibles: vec![],
                }],
                debut: None,
                fin,
                motif: "Absence du CPE".into(),
            },
        )
        .await
        .expect("délégation accordée");
    let t = dans_la_file(&ent, etab, delegue, FILE_JUSTIFICATIFS, tache).await;
    assert!(
        t.is_some(),
        "le délégataire voit la file pendant la délégation"
    );
    ent.afaire_prendre(
        etab,
        delegue.user_id,
        "prof",
        tache,
        NiveauPrise::Decider,
        t0(),
    )
    .await
    .unwrap();
    // La délégation prend fin : il ne peut plus agir ni voir la file ; sa prise expirera ou sera réattribuée.
    ent.delegation_revoquer(&d, deleg.id).await.unwrap();
    assert!(introuvable(
        &ent.file_afaire(etab, delegue.user_id, "prof", FILE_JUSTIFICATIFS, t0())
            .await
    ));
    assert!(introuvable(
        &ent.afaire_prendre(
            etab,
            delegue.user_id,
            "prof",
            tache,
            NiveauPrise::Decider,
            t0()
        )
        .await
    ));
}

// ─────────────────────────── Supervision par nombres ───────────────────────────

#[tokio::test]
async fn p4_supervision_des_nombres_sans_nom_ni_classement() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let dir = &c.direction;
    let (_, tache) = tache_justificatif(&ent, &c).await;
    // Ferme la tâche à t0 + 1 h : le délai médian existe (POR-162).
    let (objet,): (Uuid,) = sqlx::query_as("SELECT objet_id FROM ent_afaire WHERE id = $1")
        .bind(tache)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let mut tx = crate::tenant_tx::TenantTx::begin(ent.pool(), etab)
        .await
        .unwrap();
    afaire::fermer_tache(
        &mut tx,
        &afaire::FermetureTache {
            etablissement_id: etab,
            nature: "vs.trancher_justificatif",
            objet_id: objet,
            destinataire: None,
            motif: afaire::MotifClotureAfaire::Faite,
            par_compte_id: Some(c.cpe.user_id),
            version_source: afaire::version_de(t0() + Duration::hours(1)),
            maintenant: t0() + Duration::hours(1),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let (_, autre) = tache_justificatif(&ent, &c).await;

    // Témoin : la direction supervise ; les autres profils sont refusés.
    let s = ent
        .afaire_supervision(etab, dir.user_id, "direction", t0() + Duration::hours(2))
        .await
        .unwrap();
    let ligne = s
        .lignes
        .iter()
        .find(|l| l.file == FILE_JUSTIFICATIFS && l.nature == "vs.trancher_justificatif")
        .expect("ligne");
    assert!(
        ligne.ouvertes >= 1 && ligne.delai_median_minutes.is_some(),
        "{ligne:?}"
    );
    for (p, role) in [
        (&c.cpe, "cpe"),
        (&c.prof, "prof"),
        (&c.administratif, "administratif"),
    ] {
        let r = ent.afaire_supervision(etab, p.user_id, role, t0()).await;
        assert!(interdit(&r), "{role} : {r:?}");
    }
    // Jamais un nom ni une adresse ; jamais de classement (ordre alphabétique des files puis des natures).
    let dump = format!("{s:?}");
    for interdit in ["Remus", "Severus", "Harry", "@ent.lab"] {
        assert!(
            !dump.contains(interdit),
            "la supervision ne dit rien de {interdit}"
        );
    }
    let cles: Vec<(&str, &str)> = s
        .lignes
        .iter()
        .map(|l| (l.file.as_str(), l.nature.as_str()))
        .collect();
    let mut triees = cles.clone();
    triees.sort();
    assert_eq!(cles, triees, "ordre stable, indépendant des volumes");
    let _ = autre;
}

// ─────────────────────────── Critère 2 : aucune tâche orpheline ───────────────────────────

/// Un compte qui n'appartient plus à l'établissement (départ, CPO-39).
async fn compte_parti(ent: &PgEnt) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_user (id, email, password_hash, display_name) VALUES ($1, $2, 'x', 'Agent Parti')")
        .bind(id)
        .bind(format!("parti-{}@ent.lab", id.simple()))
        .execute(ent.pool_verif())
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn p4_aucune_tache_orpheline_elles_retournent_a_la_file_de_repli() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let dir = &c.direction;
    let parti = compte_parti(&ent).await;
    // Une décision nominative d'un agent parti : seul destinataire, un compte sans appartenance.
    let objet = Uuid::new_v4();
    let mut tx = crate::tenant_tx::TenantTx::begin(ent.pool(), etab)
        .await
        .unwrap();
    afaire::ouvrir_tache(
        &mut tx,
        &OuvertureTache {
            etablissement_id: etab,
            nature: "afaire.decision_en_attente",
            objet_id: objet,
            eleve_id: None,
            valeurs: vec![(afaire::Var::Nombre, "3".into())],
            destinataires: vec![Destinataire::Compte(parti)],
            echeance_le: None,
            version_source: afaire::version_de(t0()),
            par_compte_id: None,
            maintenant: t0(),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let (tache,): (Uuid,) =
        sqlx::query_as("SELECT id FROM ent_afaire WHERE etablissement_id = $1 AND objet_id = $2")
            .bind(etab)
            .bind(objet)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();

    // Témoin : la direction n'est pas encore destinataire ; la tâche est détectée comme orpheline.
    assert!(dans_la_file(&ent, etab, dir, FILE_DECISIONS, tache)
        .await
        .is_none());
    let orph = ent
        .afaire_orphelines(etab, dir.user_id, "direction")
        .await
        .unwrap();
    assert!(orph.iter().any(|o| o.id == tache), "orpheline détectée");
    assert!(
        orph.iter().all(|o| !o.titre.contains("Parti")),
        "titre neutre seulement"
    );
    let s = ent
        .afaire_supervision(etab, dir.user_id, "direction", t0())
        .await
        .unwrap();
    assert!(s.orphelines >= 1);
    // Les autres profils ne voient pas les orphelines.
    assert!(interdit(
        &ent.afaire_orphelines(etab, c.cpe.user_id, "cpe").await
    ));

    // Le rapprochement rend la tâche à la file de repli de sa nature (R-PO9) : plus d'orpheline.
    let rap = ent.rapprocher_afaire(etab, t0()).await.unwrap();
    assert!(rap.reaffectees_repli >= 1, "{rap:?}");
    assert!(
        dans_la_file(&ent, etab, dir, FILE_DECISIONS, tache)
            .await
            .is_some(),
        "la direction la reçoit"
    );
    let orph = ent
        .afaire_orphelines(etab, dir.user_id, "direction")
        .await
        .unwrap();
    assert!(orph.iter().all(|o| o.id != tache), "plus orpheline");
    // Idempotent : un second passage ne ré-ajoute rien.
    let rap2 = ent.rapprocher_afaire(etab, t0()).await.unwrap();
    assert_eq!(rap2.reaffectees_repli, 0, "{rap2:?}");
}

#[tokio::test]
async fn p4_clore_une_orpheline_sans_objet_apres_verification_aupres_du_proprietaire() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let dir = &c.direction;
    let parti = compte_parti(&ent).await;
    // 1. Une orpheline dont le propriétaire ne dit rien (nature sans source) : close sans objet.
    let objet = Uuid::new_v4();
    let mut tx = crate::tenant_tx::TenantTx::begin(ent.pool(), etab)
        .await
        .unwrap();
    for (nature, obj, valeurs) in [(
        "afaire.decision_en_attente",
        objet,
        vec![(afaire::Var::Nombre, "9".into())],
    )] {
        afaire::ouvrir_tache(
            &mut tx,
            &OuvertureTache {
                etablissement_id: etab,
                nature,
                objet_id: obj,
                eleve_id: None,
                valeurs,
                destinataires: vec![Destinataire::Compte(parti)],
                echeance_le: None,
                version_source: afaire::version_de(t0()),
                par_compte_id: None,
                maintenant: t0(),
            },
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
    let (tache,): (Uuid,) =
        sqlx::query_as("SELECT id FROM ent_afaire WHERE etablissement_id = $1 AND objet_id = $2")
            .bind(etab)
            .bind(objet)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    // Seuls les superviseurs closent.
    assert!(interdit(
        &ent.afaire_clore_orpheline(etab, c.cpe.user_id, "cpe", tache, t0())
            .await
    ));
    ent.afaire_clore_orpheline(etab, dir.user_id, "direction", tache, t0())
        .await
        .unwrap();
    assert_eq!(etat_en_base(&ent, etab, tache).await, "sans_objet");
    // Ce qui n'est pas orphelin ne se clôt pas par là (pas d'énumération, pas de clôture arbitraire).
    let (_, justificatif) = tache_justificatif(&ent, &c).await;
    assert!(introuvable(
        &ent.afaire_clore_orpheline(etab, dir.user_id, "direction", justificatif, t0())
            .await
    ));
    assert_eq!(etat_en_base(&ent, etab, justificatif).await, "ouverte");

    // 2. Une orpheline que le propriétaire dit ENCORE ouverte reste : justificatif réel, destinataire parti.
    let (absence, t2) = tache_justificatif(&ent, &c).await;
    sqlx::query(
        "UPDATE ent_afaire_destinataire SET retire_le = $3 WHERE etablissement_id = $1 AND afaire_id = $2 AND type = 'file'",
    )
    .bind(etab)
    .bind(t2)
    .bind(t0())
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let mut tx = crate::tenant_tx::TenantTx::begin(ent.pool(), etab)
        .await
        .unwrap();
    afaire::ouvrir_tache(
        &mut tx,
        &OuvertureTache {
            etablissement_id: etab,
            nature: "vs.trancher_justificatif",
            objet_id: absence,
            eleve_id: Some(eleve(&c, "Harry").id),
            valeurs: vec![
                (afaire::Var::Prenom, "Harry".into()),
                (afaire::Var::Date, "10/03".into()),
            ],
            destinataires: vec![Destinataire::Compte(parti)],
            echeance_le: None,
            version_source: afaire::version_de(t0() + Duration::minutes(1)),
            par_compte_id: None,
            maintenant: t0() + Duration::minutes(1),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let orph = ent
        .afaire_orphelines(etab, dir.user_id, "direction")
        .await
        .unwrap();
    assert!(orph.iter().any(|o| o.id == t2), "témoin : orpheline");
    let r = ent
        .afaire_clore_orpheline(etab, dir.user_id, "direction", t2, t0())
        .await;
    assert!(
        matches!(r, Err(EntDbError::ConflitDossier(_))),
        "le propriétaire la dit encore ouverte : {r:?}"
    );
    assert_eq!(etat_en_base(&ent, etab, t2).await, "ouverte");
}
