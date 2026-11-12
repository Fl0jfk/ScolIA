//! Socle 29, lots P2 et P3 : index des tâches, kit, rapprochement, à-faire famille, cartes d'action
//! (`docs/socles/29_PORTAILS.md` §8, §15.4, §24, §25). Base locale de test (feature `labo`).
//!
//! Chaque scénario commence par un **témoin** (l'acteur légitime voit sa tâche) avant la mesure de
//! protection : un refus n'est une preuve que si le témoin passe.

use super::*;
use crate::afaire::{self, Destinataire, FermetureTache, MotifClotureAfaire, OuvertureTache, Var};
use crate::appel_repo::LigneEtat;
use crate::outbox::ConfigDistribution;
use chrono::{DateTime, Duration, Utc};
use domain_eleve::today_utc;
use ent_auth::Role;
use ent_types::afaire::{AFaire, EtatAfaire, MENTION_SANS_AUTORITE, MESSAGE_DEJA_TRAITEE};
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
    })
    .await;
    Some((ent, cast))
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

/// Les scénarios hors R-PO31 ne dépendent pas de l'heure à laquelle la CI tourne : une tâche ouverte
/// pendant la plage calme (20 h – 7 h) n'est visible qu'à la fin de la plage, et ce comportement est
/// testé à part avec une horloge injectée (`p3_plage_calme_visibilite_et_pastille_gelee`). Ici on
/// lève simplement le délai de visibilité des tâches de l'établissement de test.
async fn sans_plage(ent: &PgEnt, etab: Uuid) {
    sqlx::query("UPDATE ent_afaire SET visible_a_partir_de = LEAST(visible_a_partir_de, now()) WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

async fn distribuer(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(config(etab))
        .distribuer_tout()
        .await
        .unwrap();
    sans_plage(ent, etab).await;
}

async fn rapprocher(ent: &PgEnt, etab: Uuid) -> ent_types::afaire::RapportRapprochement {
    let r = ent.rapprocher_afaire(etab, Utc::now()).await.unwrap();
    sans_plage(ent, etab).await;
    r
}

fn eleve<'a>(c: &'a LaboCast, prenom: &str) -> &'a LaboEleve {
    c.eleves
        .iter()
        .find(|e| e.prenom == prenom)
        .expect("élève du cast")
}

/// Nombre de tâches « justifier une absence » (le jeu de labo porte aussi des mots de carnet et des
/// documents à signer, qui ont leurs propres scénarios).
fn just(a: &AFaire) -> i64 {
    a.taches
        .iter()
        .filter(|t| t.nature == "vs.justifier_absence" && t.etat.est_ouvert())
        .count() as i64
}

fn titres(a: &AFaire) -> Vec<String> {
    a.taches.iter().map(|t| t.titre.clone()).collect()
}

async fn liste(ent: &PgEnt, etab: Uuid, p: &LaboPersona) -> AFaire {
    ent.a_faire_de(etab, p.user_id, p.role.as_str(), Utc::now())
        .await
        .unwrap_or_else(|e| panic!("à-faire de {:?} : {e:?}", p.role))
}

/// Absents à l'appel (Harry, Hermione, Ginny) : produit les événements `eleve.absence_declared`.
async fn appel_avec_absents(ent: &PgEnt, c: &LaboCast, absents: &[Uuid]) {
    // Le jeu de labo contient déjà des absences à traiter (dont une pour Harry aujourd'hui, déjà
    // justifiée) : l'appel s'y rattacherait. On les écarte pour que le scénario parte d'un état connu.
    sqlx::query("UPDATE ent_absence SET statut = 'annulee_present' WHERE etablissement_id = $1")
        .bind(c.etablissement_id)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let (appel, _) = ent
        .ouvrir_appel(
            c.etablissement_id,
            c.creneau_rogue_4b,
            today_utc(),
            c.prof.user_id,
            Role::Prof.as_str(),
        )
        .await
        .expect("ouvrir l'appel");
    for e in absents {
        ent.marquer_appel_ligne(
            c.etablissement_id,
            appel.id,
            c.prof.user_id,
            Role::Prof.as_str(),
            *e,
            LigneEtat::Absent,
        )
        .await
        .expect("marquer");
    }
    ent.clore_appel(
        c.etablissement_id,
        appel.id,
        c.prof.user_id,
        Role::Prof.as_str(),
        Some(c.prof.user_id.to_string()),
    )
    .await
    .expect("clore l'appel");
}

async fn absence_de(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Uuid {
    let (id,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_one(ent.pool_verif())
    .await
    .expect("absence de l'élève");
    id
}

async fn nombre(ent: &PgEnt, etab: Uuid, sql: &'static str) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql)
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    n
}

/// Parent supplémentaire d'un élève (compte + lien), dans le foyer indiqué (créé si absent).
async fn parent_en_plus(
    ent: &PgEnt,
    etab: Uuid,
    eleve: Uuid,
    foyer: Option<Uuid>,
    nom: &str,
) -> LaboPersona {
    let user = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
         VALUES ($1, $2, 'x', $3, now(), now())",
    )
    .bind(user)
    .bind(format!("{}-{}@ent.lab", nom.to_lowercase(), user.simple()))
    .bind(nom)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1, $2, 'parent', now())")
        .bind(user)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let foyer = match foyer {
        Some(f) => f,
        None => {
            let f = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_foyer (id, etablissement_id, libelle) VALUES ($1, $2, 'Autre foyer')")
                .bind(f)
                .bind(etab)
                .execute(ent.pool_verif())
                .await
                .unwrap();
            f
        }
    };
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, foyer_id, lien)
         VALUES ($1, $2, $3, $4, $5, 'pere')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve)
    .bind(user)
    .bind(foyer)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    LaboPersona {
        user_id: user,
        email: String::new(),
        role: Role::Parent,
        token: String::new(),
    }
}

async fn foyer_du_lien(ent: &PgEnt, etab: Uuid, eleve: Uuid, user: Uuid) -> Uuid {
    let (f,): (Uuid,) = sqlx::query_as(
        "SELECT foyer_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
    )
    .bind(etab)
    .bind(eleve)
    .bind(user)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    f
}

fn erreur_est_deja_traitee<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::ConflitDossier(m)) if m == MESSAGE_DEJA_TRAITEE)
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// P2 — le kit, les réactions, le rapprochement, la reconstruction
// ─────────────────────────────────────────────────────────────────────────────────────────────

/// Les tâches s'ouvrent dans la réaction au fait (outbox), l'index ne copie ni motif ni identité
/// inutile, le rapprochement n'a rien à réparer, une perte est réparée, une restauration neutralise
/// puis le rapprochement rebâtit à l'identique, et toute fermeture est journalisée en ajout seul.
#[tokio::test]
async fn p2_reactions_rapprochement_perte_et_restauration() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let harry = eleve(&c, "Harry").clone();
    let hermione = eleve(&c, "Hermione").clone();

    appel_avec_absents(&ent, &c, &[harry.id, hermione.id]).await;
    // Avant la distribution : le fait est là, la tâche pas encore (elle naît de la réaction).
    assert_eq!(just(&liste(&ent, etab, &harry.parent).await), 0);
    distribuer(&ent, etab).await;

    // Témoin : chaque famille voit SA tâche, titre neutre.
    let a_harry = liste(&ent, etab, &harry.parent).await;
    assert_eq!(just(&a_harry), 1, "{:?}", titres(&a_harry));
    let t = a_harry
        .taches
        .iter()
        .find(|t| t.nature == "vs.justifier_absence")
        .unwrap();
    assert!(
        t.titre.starts_with("Absence à justifier — Harry"),
        "{}",
        t.titre
    );
    assert_eq!(t.nature, "vs.justifier_absence");
    assert!(matches!(
        t.action,
        Some(ent_types::afaire::ActionTache::Carte { .. })
    ));
    let a_herm = liste(&ent, etab, &hermione.parent).await;
    assert!(a_herm
        .taches
        .iter()
        .any(|t| t.titre.contains("Absence à justifier — Hermione")));
    // Aucune fuite de motif ni de nom de famille dans l'index (R-PO5).
    let (fuite,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_afaire WHERE etablissement_id = $1 AND (titre_neutre ILIKE '%potter%' OR titre_neutre ILIKE '%motif%')",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(fuite, 0);

    // Idempotence : la réaction rejouée et le rapprochement n'ajoutent ni ne ferment rien.
    // Première passe = reprise initiale : le jeu de labo contient des faits sans événement (carnet,
    // documents, appels) que le rapprochement indexe, sans copier aucune donnée.
    let reprise = rapprocher(&ent, etab).await;
    assert!(
        reprise.ajoutees > 0,
        "la reprise indexe l'existant : {reprise:?}"
    );
    let avant = nombre(
        &ent,
        etab,
        "SELECT count(*) FROM ent_afaire WHERE etablissement_id = $1",
    )
    .await;
    distribuer(&ent, etab).await;
    let r1 = rapprocher(&ent, etab).await;
    assert_eq!((r1.ajoutees, r1.fermees), (0, 0), "aucun écart : {r1:?}");
    let r2 = rapprocher(&ent, etab).await;
    assert_eq!((r2.ajoutees, r2.fermees), (0, 0));
    assert_eq!(
        nombre(
            &ent,
            etab,
            "SELECT count(*) FROM ent_afaire WHERE etablissement_id = $1"
        )
        .await,
        avant
    );

    // Perte d'événements simulée : l'index est vidé (tables enfants d'abord) ; le rapprochement le rebâtit.
    for sql in [
        "DELETE FROM ent_afaire_etat_personnel WHERE etablissement_id = $1",
        "DELETE FROM ent_afaire_prise WHERE etablissement_id = $1",
        "DELETE FROM ent_afaire_cle WHERE etablissement_id = $1",
        "DELETE FROM ent_afaire_destinataire WHERE etablissement_id = $1",
    ] {
        sqlx::query(sql)
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
    // Le journal est en ajout seul (déclencheur) : on ne le purge pas ; on ne touche donc pas à ent_afaire.
    sqlx::query("UPDATE ent_afaire SET etat = 'annulee', close_le = now(), issue = 'annulee' WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(
        just(&liste(&ent, etab, &harry.parent).await),
        0,
        "index perdu : rien n'est montré"
    );
    let r = rapprocher(&ent, etab).await;
    assert!(
        r.ajoutees >= 2,
        "les tâches manquantes sont rouvertes : {r:?}"
    );
    assert_eq!(just(&liste(&ent, etab, &harry.parent).await), 1);
    assert_eq!(just(&liste(&ent, etab, &hermione.parent).await), 1);

    // Restauration : neutralisation puis reconstruction (E9, R-PO24).
    let mut conn = ent.pool_verif().acquire().await.unwrap();
    let rap = crate::registre_donnees::neutraliser_files_restauration(
        &mut conn,
        crate::registre_donnees::PerimetreNeutralisation::Etablissement(etab),
    )
    .await
    .unwrap();
    drop(conn);
    assert!(
        rap.tables.iter().any(|l| l.table == "ent_afaire"),
        "l'index est neutralisé"
    );
    assert_eq!(
        just(&liste(&ent, etab, &harry.parent).await),
        0,
        "neutralisé : plus rien n'est ouvert"
    );
    let (neutres,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_afaire WHERE etablissement_id = $1 AND issue = 'restauration'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(neutres >= 2);
    let r = rapprocher(&ent, etab).await;
    assert!(
        r.rouvertes >= 2,
        "reconstruit depuis les propriétaires : {r:?}"
    );
    assert_eq!(just(&liste(&ent, etab, &harry.parent).await), 1);
    assert!(liste(&ent, etab, &harry.parent)
        .await
        .taches
        .iter()
        .any(|t| t.titre.starts_with("Absence à justifier — Harry")));

    // Le journal des transitions est en ajout seul.
    let modif =
        sqlx::query("UPDATE ent_afaire_journal SET motif_code = 'x' WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await;
    assert!(modif.is_err(), "ent_afaire_journal est en ajout seul");
    let suppr = sqlx::query("DELETE FROM ent_afaire_journal WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await;
    assert!(suppr.is_err());
}

/// Une tâche fantôme (index ouvert, source fermée) n'est jamais montrée (R-PO3) ; une source qui
/// perd le droit du destinataire la retire immédiatement, sans attendre le rapprochement ni un cache
/// (R-PO25) ; un lien suspendu fait disparaître les tâches à la requête suivante.
#[tokio::test]
async fn p2_revalidation_a_l_affichage_fantome_et_lien_suspendu() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let hermione = eleve(&c, "Hermione").clone();
    appel_avec_absents(&ent, &c, &[hermione.id]).await;
    distribuer(&ent, etab).await;
    assert_eq!(
        just(&liste(&ent, etab, &hermione.parent).await),
        1,
        "témoin"
    );

    // La source dit « justifié » sans que l'index le sache (événement perdu) : pas de tâche fantôme.
    let absence = absence_de(&ent, etab, hermione.id).await;
    sqlx::query("UPDATE ent_absence SET motif_justif = 'rendez-vous' WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(absence)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let a = liste(&ent, etab, &hermione.parent).await;
    assert_eq!(just(&a), 0, "fantôme masqué : {:?}", titres(&a));
    // La tâche reste ouverte dans l'index jusqu'au rapprochement, qui la ferme.
    let r = rapprocher(&ent, etab).await;
    assert!(r.fermees >= 1, "{r:?}");
    sqlx::query(
        "UPDATE ent_absence SET motif_justif = NULL WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(absence)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    // La source la dit de nouveau ouverte : le rapprochement la rouvre (fait plus récent).
    rapprocher(&ent, etab).await;
    assert_eq!(
        just(&liste(&ent, etab, &hermione.parent).await),
        1,
        "rouverte"
    );

    // Lien suspendu (décision de justice) : la tâche disparaît à la requête suivante, sans trace.
    sqlx::query(
        "UPDATE ent_eleve_lien SET suspendu_le = now(), suspension_motif = 'decision'
          WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
    )
    .bind(etab)
    .bind(hermione.id)
    .bind(hermione.parent.user_id)
    .execute(ent.pool_verif())
    .await
    .unwrap_or_else(|e| panic!("suspension : {e}"));
    let a = liste(&ent, etab, &hermione.parent).await;
    assert_eq!(
        a.compteur,
        0,
        "lien suspendu : aucune tâche ({:?})",
        titres(&a)
    );
    assert!(a.taches.is_empty() && a.groupes.is_empty());
    assert!(
        a.mention.is_none(),
        "aucune mention qui révèle quoi que ce soit sur le lien suspendu"
    );
}

/// Matrice de séparation (§15.4) : personne d'autre que le responsable habilité ne voit le titre ni
/// n'obtient d'aperçu ; un responsable sans exercice de l'autorité parentale n'a aucune tâche de
/// décision, seulement une mention sobre (R-PO8) ; le personnel ne voit que ses files.
#[tokio::test]
async fn p2_separation_parent_sans_autorite_personnel_autre_etablissement() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let harry = eleve(&c, "Harry").clone();
    let draco = eleve(&c, "Draco").clone();
    assert!(
        !draco.peut_justifier,
        "Draco : lien sans droit de justifier"
    );
    appel_avec_absents(&ent, &c, &[harry.id]).await;
    // Draco absent (source directe : l'appel de sa classe n'est pas dans ce scénario).
    sqlx::query(
        "INSERT INTO ent_absence (id, etablissement_id, eleve_id, debut, fin, motif, statut)
         VALUES ($1, $2, $3, $4, $4, (SELECT code FROM ent_absence_motif LIMIT 1), 'a_traiter')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(draco.id)
    .bind(today_utc())
    .execute(ent.pool_verif())
    .await
    .unwrap();
    distribuer(&ent, etab).await;
    rapprocher(&ent, etab).await;

    // Témoin : le responsable de Harry voit sa tâche.
    let a = liste(&ent, etab, &harry.parent).await;
    assert_eq!(just(&a), 1, "témoin");
    let tache_harry = a.taches[0].id;

    // Parent d'un autre foyer et sans exercice : aucune tâche, mention sobre ; jamais le titre de Harry.
    let tom = liste(&ent, etab, &draco.parent).await;
    assert_eq!(just(&tom), 0, "{:?}", titres(&tom));
    assert_eq!(tom.mention.as_deref(), Some(MENTION_SANS_AUTORITE));
    assert!(!format!("{tom:?}").contains("Harry"));
    // Aperçu et exécution d'une tâche d'autrui : introuvable (404 neutre), jamais « déjà traitée ».
    let r = ent
        .afaire_apercu(
            etab,
            draco.parent.user_id,
            "parent",
            tache_harry,
            Utc::now(),
        )
        .await;
    assert!(
        matches!(
            r,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        ),
        "{r:?}"
    );
    let r = ent
        .afaire_executer(
            etab,
            draco.parent.user_id,
            "parent",
            tache_harry,
            true,
            Some("x".into()),
            Utc::now(),
        )
        .await;
    assert!(
        matches!(
            r,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        ),
        "{r:?}"
    );

    // Personnel : professeur, direction, comptabilité et administratif ne reçoivent pas les tâches
    // nominatives des familles (la direction supervise des nombres, pas des contenus).
    for p in [&c.prof, &c.direction, &c.compta, &c.administratif, &c.admin] {
        let a = liste(&ent, etab, p).await;
        assert!(
            a.taches.iter().all(|t| t.id != tache_harry),
            "{:?} ne doit pas voir une tâche de famille : {:?}",
            p.role,
            titres(&a)
        );
        let r = ent
            .afaire_apercu(etab, p.user_id, p.role.as_str(), tache_harry, Utc::now())
            .await;
        assert!(r.is_err(), "{:?} ne reçoit aucun aperçu", p.role);
    }
    // Un élève mineur n'a aucune tâche de décision.
    if let Some(compte) = &harry.compte {
        assert_eq!(
            just(&liste(&ent, etab, compte).await),
            0,
            "élève mineur : rien à décider"
        );
    }

    // Autre établissement : rien, ni la tâche ni son existence.
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let a = liste(&ent, autre.etablissement_id, &autre.parent_lea).await;
    assert_eq!(a.compteur, 0);
    let r = ent
        .afaire_apercu(
            autre.etablissement_id,
            autre.parent_lea.user_id,
            "parent",
            tache_harry,
            Utc::now(),
        )
        .await;
    assert!(r.is_err());
    // Rôle inconnu : refusé.
    assert!(ent
        .a_faire_de(etab, harry.parent.user_id, "inconnu", Utc::now())
        .await
        .is_err());
}

/// La file de la vie scolaire (« justificatifs à trancher ») : seuls les porteurs du droit la voient ;
/// la tâche naît de la justification de la famille et se ferme à la décision ; le rapprochement du
/// propriétaire n'écrit aucun nom dans le rapport (E14).
#[tokio::test]
async fn p2_file_vie_scolaire_et_rapport_sans_nom() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let harry = eleve(&c, "Harry").clone();
    appel_avec_absents(&ent, &c, &[harry.id]).await;
    distribuer(&ent, etab).await;
    let a = liste(&ent, etab, &harry.parent).await;
    let tache = a
        .taches
        .iter()
        .find(|t| t.nature == "vs.justifier_absence")
        .unwrap()
        .id;

    // Carte d'action : aperçu puis exécution avec confirmation (R-PO1, §8.7).
    let ap = ent
        .afaire_apercu(etab, harry.parent.user_id, "parent", tache, Utc::now())
        .await
        .unwrap();
    assert!(
        ap.texte.contains("Harry") && ap.texte.contains("la vie scolaire décidera"),
        "{}",
        ap.texte
    );
    assert_eq!(ap.champs.len(), 1);
    // Sans confirmation explicite : refus ; sans motif : refus. Rien n'a été écrit.
    let r = ent
        .afaire_executer(
            etab,
            harry.parent.user_id,
            "parent",
            tache,
            false,
            Some("rdv".into()),
            Utc::now(),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    let r = ent
        .afaire_executer(
            etab,
            harry.parent.user_id,
            "parent",
            tache,
            true,
            None,
            Utc::now(),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    assert_eq!(just(&liste(&ent, etab, &harry.parent).await), 1);

    // Avant la justification : la file de la vie scolaire est vide (témoin négatif).
    assert!(liste(&ent, etab, &c.cpe)
        .await
        .taches
        .iter()
        .all(|t| t.nature != "vs.trancher_justificatif"));

    let r = ent
        .afaire_executer(
            etab,
            harry.parent.user_id,
            "parent",
            tache,
            true,
            Some("Rendez-vous médical".into()),
            Utc::now(),
        )
        .await
        .unwrap();
    assert!(r.fait);
    assert_eq!(
        just(&liste(&ent, etab, &harry.parent).await),
        0,
        "la tâche de la famille est close"
    );
    // La commande du propriétaire a bien été exécutée : l'absence porte le motif.
    let absence = absence_de(&ent, etab, harry.id).await;
    let (motif,): (Option<String>,) = sqlx::query_as(
        "SELECT motif_justif FROM ent_absence WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(absence)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(motif.as_deref(), Some("Rendez-vous médical"));
    // Une seconde exécution (double clic, rejeu) : « déjà traitée », jamais deux fois.
    let r = ent
        .afaire_executer(
            etab,
            harry.parent.user_id,
            "parent",
            tache,
            true,
            Some("Rendez-vous médical".into()),
            Utc::now(),
        )
        .await;
    assert!(erreur_est_deja_traitee(r));

    // Vie scolaire : la tâche à trancher est apparue, dans la file seulement, sans le motif.
    distribuer(&ent, etab).await;
    let cpe = liste(&ent, etab, &c.cpe).await;
    let a_trancher: Vec<_> = cpe
        .taches
        .iter()
        .filter(|t| t.nature == "vs.trancher_justificatif")
        .collect();
    assert_eq!(a_trancher.len(), 1, "{:?}", titres(&cpe));
    assert!(a_trancher[0]
        .titre
        .starts_with("Justificatif à trancher — Harry"));
    assert!(
        !format!("{cpe:?}").contains("médical"),
        "aucun motif dans l'à-faire"
    );
    // Le professeur n'est pas membre de la file.
    assert!(liste(&ent, etab, &c.prof)
        .await
        .taches
        .iter()
        .all(|t| t.nature != "vs.trancher_justificatif"));

    // La décision de la vie scolaire ferme la tâche (réaction au fait).
    ent.accepter_justificatif_for_actor(etab, c.cpe.user_id, "cpe", absence, None)
        .await
        .unwrap();
    distribuer(&ent, etab).await;
    assert!(liste(&ent, etab, &c.cpe)
        .await
        .taches
        .iter()
        .all(|t| t.nature != "vs.trancher_justificatif"));

    // Rapport de rapprochement : des comptes, jamais de nom (E14).
    let rap = rapprocher(&ent, etab).await;
    let json = serde_json::to_string(&rap).unwrap();
    for interdit in ["Harry", "Potter", "médical"] {
        assert!(
            !json.contains(interdit),
            "le rapport ne doit pas contenir {interdit}"
        );
    }
    // Droit d'usage : seul l'administrateur lance un rapprochement.
    assert!(ent
        .rapprocher_afaire_for_actor(etab, c.admin.user_id, "admin", Utc::now())
        .await
        .is_ok());
    assert!(matches!(
        ent.rapprocher_afaire_for_actor(etab, c.prof.user_id, "prof", Utc::now())
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.rapprocher_afaire_for_actor(etab, harry.parent.user_id, "parent", Utc::now())
            .await,
        Err(EntDbError::Forbidden)
    ));
}

/// Signer le carnet et un document : cardinalité « par autorité » (un seul titulaire suffit, la tâche
/// disparaît pour tous), décès (R-PO10 : sans objet, aucun message), départ du destinataire.
#[tokio::test]
async fn p2_carnet_document_autorite_et_deces() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let harry = eleve(&c, "Harry").clone();
    // Document à signer publié par l'administratif ; mot au carnet par le CPE.
    let doc = ent
        .publier_doc_a_signer_for_actor(
            "administratif",
            domain_eleve::PublierDocASigner {
                etablissement_id: etab,
                eleve_id: harry.id,
                code: "reglement".into(),
                titre: "Règlement intérieur".into(),
                corps: "Texte".into(),
                annee_scolaire: "2026-2027".into(),
                echeance: None,
                actor_user_id: c.administratif.user_id,
            },
        )
        .await
        .expect("publier");
    ent.creer_carnet_entree_for_actor(
        "cpe",
        domain_eleve::CreerCarnetEntree {
            etablissement_id: etab,
            eleve_id: harry.id,
            date_entree: today_utc(),
            categorie: domain_eleve::CarnetCategorie::Information,
            titre: "Information".into(),
            corps: "Texte du mot".into(),
            visible_famille: true,
            actor_user_id: c.cpe.user_id,
        },
    )
    .await
    .expect("carnet");
    distribuer(&ent, etab).await;

    // Deux titulaires de l'autorité dans le même foyer : la tâche « par autorité » est indexée chez chacun.
    let foyer = foyer_du_lien(&ent, etab, harry.id, harry.parent.user_id).await;
    let second = parent_en_plus(&ent, etab, harry.id, Some(foyer), "Second").await;
    rapprocher(&ent, etab).await;
    let a1 = liste(&ent, etab, &harry.parent).await;
    let a2 = liste(&ent, etab, &second).await;
    for (qui, a) in [("titulaire", &a1), ("second", &a2)] {
        let natures: Vec<_> = a.taches.iter().map(|t| t.nature.as_str()).collect();
        eprintln!("DEBUG {qui} : {natures:?}");
        assert!(
            natures.contains(&"doc.signer") && natures.contains(&"vs.signer_carnet"),
            "{natures:?}"
        );
    }
    let (nb_taches,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_afaire WHERE etablissement_id = $1 AND nature_code = 'doc.signer' AND etat = 'ouverte' AND objet_id = $2",
    )
    .bind(etab)
    .bind(doc.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        nb_taches, 1,
        "une seule tâche partagée pour deux titulaires"
    );

    // L'un signe le document : disparition pour tous, même foyer = immédiate (CPO-75).
    let (t,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_afaire WHERE etablissement_id = $1 AND nature_code = 'doc.signer' AND objet_id = $2",
    )
    .bind(etab)
    .bind(doc.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let ap = ent
        .afaire_apercu(etab, harry.parent.user_id, "parent", t, Utc::now())
        .await
        .unwrap();
    assert!(ap.texte.contains("Règlement intérieur"));
    ent.afaire_executer(
        etab,
        harry.parent.user_id,
        "parent",
        t,
        true,
        None,
        Utc::now(),
    )
    .await
    .unwrap();
    for p in [&harry.parent, &second] {
        let a = liste(&ent, etab, p).await;
        assert!(
            a.taches.iter().all(|x| x.id != t),
            "même foyer : disparue tout de suite"
        );
    }

    // Décès de l'élève : tout devient « sans objet », sans message, sans compteur (R-PO10).
    let notifs_avant = nombre(
        &ent,
        etab,
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1",
    )
    .await;
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let n = crate::afaire_service::fermer_taches_eleve(
        &mut tx,
        etab,
        harry.id,
        MotifClotureAfaire::Deces,
        Utc::now(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(n >= 1);
    assert_eq!(just(&liste(&ent, etab, &harry.parent).await), 0);
    assert_eq!(just(&liste(&ent, etab, &second).await), 0);
    assert_eq!(
        nombre(
            &ent,
            etab,
            "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1"
        )
        .await,
        notifs_avant,
        "aucun message sur un décès"
    );
    let (issue,): (String,) = sqlx::query_as(
        "SELECT issue FROM ent_afaire WHERE etablissement_id = $1 AND eleve_id = $2 AND issue = 'deces' LIMIT 1",
    )
    .bind(etab)
    .bind(harry.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(issue, "deces");
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// P3 — neutralité entre parents, plage calme, majorité, escalade
// ─────────────────────────────────────────────────────────────────────────────────────────────

/// R-PO7, R-PO38 : une tâche faite par le responsable d'un AUTRE foyer reste affichée chez l'autre
/// jusqu'au point de rafraîchissement neutre (7 h), et toute action répond « Cette démarche est déjà
/// traitée », avec le même type d'erreur qu'une clôture par l'établissement.
#[tokio::test]
async fn p3_neutralite_entre_parents_de_foyers_differents() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let harry = eleve(&c, "Harry").clone();
    let autre_foyer = parent_en_plus(&ent, etab, harry.id, None, "Separe").await;
    appel_avec_absents(&ent, &c, &[harry.id]).await;
    distribuer(&ent, etab).await;
    rapprocher(&ent, etab).await;

    // Témoin : chacun a SA tâche (une réponse par élève, indexée chez chaque titulaire).
    let a = liste(&ent, etab, &harry.parent).await;
    let b = liste(&ent, etab, &autre_foyer).await;
    assert_eq!((just(&a), just(&b)), (1, 1));
    let pick = |l: &AFaire| {
        l.taches
            .iter()
            .find(|t| t.nature == "vs.justifier_absence")
            .unwrap()
            .id
    };
    let (ta, tb) = (pick(&a), pick(&b));
    assert_eq!(ta, tb, "la même tâche, deux portes");

    // A justifie. B ne voit plus de tâche « à faire » mais la carte est encore affichée (neutralité temporelle).
    ent.afaire_executer(
        etab,
        harry.parent.user_id,
        "parent",
        ta,
        true,
        Some("Grippe".into()),
        Utc::now(),
    )
    .await
    .unwrap();
    let b_apres = liste(&ent, etab, &autre_foyer).await;
    assert_eq!(just(&b_apres), 0, "plus rien à faire pour B");
    let carte: Vec<_> = b_apres.taches.iter().filter(|t| t.id == tb).collect();
    assert_eq!(
        carte.len(),
        1,
        "mais la carte reste affichée jusqu'au point neutre"
    );
    assert_eq!(carte[0].etat, EtatAfaire::Faite);
    assert!(!carte[0].nouveau);
    // Aucun nom, aucune heure de l'autre dans ce que voit B.
    let vu_par_b = format!("{b_apres:?}");
    assert!(!vu_par_b.contains(&harry.parent.user_id.to_string()));
    // L'action de B : « déjà traitée ». Même erreur pour un inconnu qui connaîtrait l'identifiant : 404, pas 409.
    let r = ent
        .afaire_executer(
            etab,
            autre_foyer.user_id,
            "parent",
            tb,
            true,
            Some("Autre".into()),
            Utc::now(),
        )
        .await;
    assert!(
        erreur_est_deja_traitee(r),
        "même réponse qu'une clôture par l'établissement"
    );
    let r = ent
        .afaire_apercu(etab, autre_foyer.user_id, "parent", tb, Utc::now())
        .await;
    assert!(erreur_est_deja_traitee(r));
    // Après le prochain point neutre (≤ 24 h), la carte disparaît.
    let lendemain = Utc::now() + Duration::hours(25);
    let b_plus_tard = ent
        .a_faire_de(etab, autre_foyer.user_id, "parent", lendemain)
        .await
        .unwrap();
    assert!(
        b_plus_tard.taches.iter().all(|t| t.id != tb),
        "disparue au point de rafraîchissement neutre"
    );

    // Une clôture par l'établissement (vie scolaire) produit exactement la même réponse côté famille.
    let harry_absence = absence_de(&ent, etab, harry.id).await;
    ent.accepter_justificatif_for_actor(etab, c.cpe.user_id, "cpe", harry_absence, None)
        .await
        .unwrap();
    distribuer(&ent, etab).await;
    let r = ent
        .afaire_executer(
            etab,
            harry.parent.user_id,
            "parent",
            ta,
            true,
            Some("x".into()),
            Utc::now(),
        )
        .await;
    assert!(erreur_est_deja_traitee(r));
}

/// R-PO31 : une tâche ouverte pendant la plage calme (jeudi 21 h 30) n'est visible d'une famille
/// qu'à 7 h ; la pastille (compteur) est gelée pendant la plage ; une tâche d'urgence déclarée ou un
/// membre du personnel ne sont pas concernés. Aucune horloge cachée : l'heure est passée en paramètre.
#[tokio::test]
async fn p3_plage_calme_visibilite_et_pastille_gelee() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let harry = eleve(&c, "Harry").clone();
    // Document à signer en base (source de vérité), ouvert dans l'index à « jeudi 12/11/2026, 21 h 30 » (heure de Paris).
    let doc = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_doc_a_signer (id, etablissement_id, eleve_id, code, titre, corps, annee_scolaire, publie_par_user_id)
         VALUES ($1, $2, $3, 'charte', 'Charte', 'x', '2026-2027', $4)",
    )
    .bind(doc)
    .bind(etab)
    .bind(harry.id)
    .bind(c.administratif.user_id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let jeudi_21h30: DateTime<Utc> = "2026-11-12T20:30:00Z".parse().unwrap();
    let sources = ent
        .taches_source(etab, "doc.signer", Some(&[doc]), jeudi_21h30)
        .await
        .unwrap();
    assert_eq!(sources.len(), 1, "la source dit la tâche ouverte");
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    for s in &sources {
        afaire::ouvrir_tache(
            &mut tx,
            &OuvertureTache {
                etablissement_id: etab,
                nature: "doc.signer",
                objet_id: s.objet_id,
                eleve_id: s.eleve_id,
                valeurs: s.valeurs.clone(),
                destinataires: s.destinataires.clone(),
                echeance_le: s.echeance_le,
                version_source: afaire::version_de(jeudi_21h30),
                par_compte_id: None,
                maintenant: jeudi_21h30,
            },
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();

    let a = |h: &str| -> DateTime<Utc> { h.parse().unwrap() };
    // 22 h (Paris 23 h) : rien, pastille à zéro.
    let pendant = ent
        .a_faire_de(
            etab,
            harry.parent.user_id,
            "parent",
            a("2026-11-12T22:00:00Z"),
        )
        .await
        .unwrap();
    assert!(
        pendant.taches.is_empty() && pendant.compteur == 0,
        "{:?}",
        titres(&pendant)
    );
    assert_eq!(
        ent.compteur_a_faire(
            etab,
            harry.parent.user_id,
            "parent",
            a("2026-11-13T05:59:00Z")
        )
        .await
        .unwrap(),
        0,
        "6 h 59 à Paris : encore la plage calme"
    );
    // 7 h 05 à Paris (06:05 UTC) : visible.
    let apres = ent
        .a_faire_de(
            etab,
            harry.parent.user_id,
            "parent",
            a("2026-11-13T06:05:00Z"),
        )
        .await
        .unwrap();
    assert_eq!(apres.compteur, 1);
    assert!(apres.taches[0].titre.starts_with("Document à signer"));
}

/// §8.8 : à la majorité, la décision bascule vers l'élève, les parents ne reçoivent plus la tâche ;
/// un élève majeur sans compte envoie la tâche à la file de repli (R-PO9), jamais nulle part.
#[tokio::test]
async fn p3_majorite_bascule_vers_l_eleve_et_repli() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let hermione = eleve(&c, "Hermione").clone();
    let Some(compte) = hermione.compte.clone() else {
        eprintln!("skip : pas de compte élève dans le cast");
        return;
    };
    appel_avec_absents(&ent, &c, &[hermione.id]).await;
    distribuer(&ent, etab).await;
    assert_eq!(
        just(&liste(&ent, etab, &hermione.parent).await),
        1,
        "témoin : parent avant majorité"
    );
    assert_eq!(just(&liste(&ent, etab, &compte).await), 0);

    {
        let mut tx = ent.pool_verif().begin().await.unwrap();
        sqlx::query("SELECT set_config('ent.identite_projection', 'on', true)")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE ent_eleve SET date_naissance = (ent_aujourdhui_etab(etablissement_id) - INTERVAL '19 years')::date WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(hermione.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    rapprocher(&ent, etab).await;
    let parent = liste(&ent, etab, &hermione.parent).await;
    assert_eq!(
        parent.compteur,
        0,
        "majeur : les parents ne décident plus ({:?})",
        titres(&parent)
    );
    let eleve_liste = liste(&ent, etab, &compte).await;
    assert_eq!(just(&eleve_liste), 1, "la tâche est celle de l'élève");

    // Majeur sans compte : la file de repli (vie scolaire) reçoit la tâche.
    sqlx::query("DELETE FROM ent_eleve_compte WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(hermione.id)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    rapprocher(&ent, etab).await;
    let cpe = liste(&ent, etab, &c.cpe).await;
    assert!(
        cpe.taches
            .iter()
            .any(|t| t.nature == "vs.justifier_absence" && t.titre.contains("Hermione")),
        "repli : {:?}",
        titres(&cpe)
    );
}

/// Escalade d'une tâche bloquante (R-PO11, §8.5) : elle ne se reporte pas, et au bout du délai la file
/// d'escalade la reçoit **en plus** (jamais en moins) ; la déclaration est refusée si elle manque d'escalade.
#[tokio::test]
async fn p2_escalade_d_une_tache_bloquante() {
    let Some((ent, c)) = labo().await else { return };
    let etab = c.etablissement_id;
    let objet = Uuid::new_v4();
    let t0 = Utc::now();
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let res = afaire::ouvrir_tache(
        &mut tx,
        &OuvertureTache {
            etablissement_id: etab,
            nature: "afaire.decision_en_attente",
            objet_id: objet,
            eleve_id: None,
            valeurs: vec![(Var::Nombre, "7".into())],
            destinataires: vec![Destinataire::File(afaire::FILE_DECISIONS.into())],
            echeance_le: None,
            version_source: afaire::version_de(t0),
            par_compte_id: None,
            maintenant: t0,
        },
    )
    .await
    .unwrap();
    assert_eq!(res.taches_creees, 1);
    // Idempotent : deux ouvertures identiques = une tâche (l'outbox est rejouée).
    let again = afaire::ouvrir_tache(
        &mut tx,
        &OuvertureTache {
            etablissement_id: etab,
            nature: "afaire.decision_en_attente",
            objet_id: objet,
            eleve_id: None,
            valeurs: vec![(Var::Nombre, "7".into())],
            destinataires: vec![Destinataire::File(afaire::FILE_DECISIONS.into())],
            echeance_le: None,
            version_source: afaire::version_de(t0),
            par_compte_id: None,
            maintenant: t0,
        },
    )
    .await
    .unwrap();
    assert_eq!((again.taches_creees, again.destinataires_ajoutes), (0, 0));
    // Un fait plus ancien n'écrase pas un plus récent (CPO-51).
    let ancien = afaire::ouvrir_tache(
        &mut tx,
        &OuvertureTache {
            etablissement_id: etab,
            nature: "afaire.decision_en_attente",
            objet_id: objet,
            eleve_id: None,
            valeurs: vec![(Var::Nombre, "99".into())],
            destinataires: vec![Destinataire::File(afaire::FILE_DECISIONS.into())],
            echeance_le: None,
            version_source: afaire::version_de(t0) - 1_000_000,
            par_compte_id: None,
            maintenant: t0,
        },
    )
    .await
    .unwrap();
    assert!(ancien.ignoree);
    // Avant le délai : pas d'escalade ; après : la file d'administration reçoit la tâche en plus.
    assert_eq!(
        afaire::escalader_echues(&mut tx, etab, t0 + Duration::hours(10))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        afaire::escalader_echues(&mut tx, etab, t0 + Duration::hours(169))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        afaire::escalader_echues(&mut tx, etab, t0 + Duration::hours(170))
            .await
            .unwrap(),
        0,
        "idempotent"
    );
    let (actifs,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_afaire_destinataire WHERE etablissement_id = $1 AND retire_le IS NULL
            AND role_dans_tache IN ('titulaire', 'escalade')",
    )
    .bind(etab)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        actifs, 2,
        "le destinataire initial reste, l'escalade s'ajoute"
    );
    // Fermeture : état final, « nouveau » effacé, journal complet (ouverte → faite).
    let n = afaire::fermer_tache(
        &mut tx,
        &FermetureTache {
            etablissement_id: etab,
            nature: "afaire.decision_en_attente",
            objet_id: objet,
            destinataire: None,
            motif: MotifClotureAfaire::Faite,
            par_compte_id: Some(c.direction.user_id),
            version_source: afaire::version_de(t0) + 1,
            maintenant: t0 + Duration::hours(171),
        },
    )
    .await
    .unwrap();
    assert_eq!(n, 1);
    assert!(
        !afaire::tache_ouverte_sur(&mut tx, etab, "afaire.decision_en_attente", objet)
            .await
            .unwrap()
    );
    let (j,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_afaire_journal j JOIN ent_afaire a ON a.id = j.afaire_id AND a.etablissement_id = j.etablissement_id
          WHERE a.etablissement_id = $1 AND a.objet_id = $2",
    )
    .bind(etab)
    .bind(objet)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(j, 2, "ouverture et clôture journalisées");
    tx.commit().await.unwrap();
}
