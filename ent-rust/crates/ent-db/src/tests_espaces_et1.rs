//! Socle 32, lot ET1 — espace personnel et espaces libres entre personnels (parité M19) : dossiers, éléments, versions,
//! corbeille, partages nominatifs, quota, téléchargement, avis d'invitation, journal
//! (`32_BUREAUTIQUE.md` §2.2 NX32-01 à NX32-20, §2.3 ND32-01 à ND32-18, §17, §18 ; R-ET1 à R-ET6, R-ET9 à R-ET13, R-ET25).
//!
//! Chaque action reprise de Next.js porte son identifiant NX32 dans le nom ou le commentaire du test ; chaque défaut ND32
//! corrigé a son test de non-régression. Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**. Aucun test ne
//! dépend de l'heure réelle : instants fixes (`t0()` + durées) ; magasin mémoire du labo ; jetons locaux à secret fixe.

use super::*;
use crate::espaces::*;
use crate::tests_espaces_et0::*;
use chrono::{Duration, NaiveDate};
use uuid::Uuid;

type Dom = domain_eleve::DomainError;

fn introuvable<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Domain(Dom::NotFound)))
}

fn uniq(prefixe: &str) -> String {
    format!("{prefixe}-{}", &Uuid::new_v4().simple().to_string()[..10])
}

fn jour(annee: i32, mois: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(annee, mois, j).unwrap()
}

/// Espace libre créé par `user` avec les membres donnés.
async fn libre(ent: &PgEnt, etab: Uuid, user: Uuid, role: &str, libelle: &str, membres: Vec<NouveauMembre>) -> Uuid {
    ent.espaces_creer_libre(
        etab,
        user,
        role,
        NouvelEspace { libelle: libelle.into(), description: None, couleur_code: None, icone_code: None, membres },
        t0(),
    )
    .await
    .unwrap()
    .id
}

async fn nb_lignes(ent: &PgEnt, table: &str, etab: Uuid) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE etablissement_id = $1"))
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

// ───────────────────────── NX32-01, NX32-12, ND32-11 : espaces, lecture sans écriture ─────────────────────────

/// NX32-01, NX32-12 (ETR-110, ETR-111) : mon espace s'ouvre une fois (idempotent) ; la liste ne contient que mes espaces,
/// du plus récent au plus ancien ; **une lecture n'écrit rien** (ND32-11).
#[tokio::test]
async fn et1_mon_espace_idempotent_et_lectures_sans_ecriture() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe) = (cast.etablissement_id, &cast.prof, &cast.cpe);
    let (role, role_c) = (prof.role.as_str(), cpe.role.as_str());
    collegue(&ent, etab, prof.user_id).await;
    let pc = collegue(&ent, etab, cpe.user_id).await;
    let a = ent.espaces_ouvrir_mon_espace(etab, prof.user_id, role, t0()).await.unwrap();
    let b = ent.espaces_ouvrir_mon_espace(etab, prof.user_id, role, t0() + Duration::days(1)).await.unwrap();
    assert_eq!(a.id, b.id, "un seul espace personnel par personne");
    assert_eq!((a.type_espace.as_str(), a.role, a.quota_octets), ("personnel", RoleEspace::Animateur, 2 * 1024 * 1024 * 1024));
    let n_perso: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_espace e JOIN ent_espace_ancrage a ON a.etablissement_id = e.etablissement_id AND a.espace_id = e.id
          WHERE e.etablissement_id = $1 AND a.personne_id = (SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2)",
    )
    .bind(etab)
    .bind(prof.user_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n_perso, 1);
    let libre_a = libre(&ent, etab, prof.user_id, role, &uniq("Équipe"), vec![NouveauMembre { personne_id: pc, role: RoleEspace::Contributeur }]).await;
    let mes = ent.espaces_lister(etab, prof.user_id, role, t0()).await.unwrap();
    assert_eq!(mes[0].type_espace, "personnel", "l'espace personnel d'abord");
    assert!(mes.iter().any(|e| e.id == libre_a));
    // Le collègue voit l'espace libre où il est invité, pas mon espace personnel (R-ET2).
    let siens = ent.espaces_lister(etab, cpe.user_id, role_c, t0()).await.unwrap();
    assert!(siens.iter().any(|e| e.id == libre_a && e.invitation_non_vue));
    assert!(!siens.iter().any(|e| e.id == a.id));
    assert!(introuvable(ent.espaces_detail(etab, cpe.user_id, role_c, a.id, t0()).await));
    // Aucune lecture n'écrit : compteurs et dates de vu inchangés après lister, détail, parcourir, badge, partagés avec moi.
    let avant = (
        nb_lignes(&ent, "ent_espace_journal", etab).await,
        nb_lignes(&ent, "ent_notification", etab).await,
        nb_lignes(&ent, "ent_metier_event", etab).await,
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ent_espace_membre_ajoute WHERE etablissement_id = $1 AND vu_le IS NULL")
            .bind(etab)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap(),
    );
    ent.espaces_lister(etab, cpe.user_id, role_c, t0()).await.unwrap();
    ent.espaces_detail(etab, cpe.user_id, role_c, libre_a, t0()).await.unwrap();
    ent.espaces_parcourir(etab, cpe.user_id, role_c, libre_a, None, t0()).await.unwrap();
    ent.espaces_invitations_non_vues(etab, cpe.user_id, role_c, t0()).await.unwrap();
    ent.espaces_partages_avec_moi(etab, cpe.user_id, role_c, t0()).await.unwrap();
    ent.espaces_quota(etab, cpe.user_id, role_c, t0()).await.unwrap();
    let apres = (
        nb_lignes(&ent, "ent_espace_journal", etab).await,
        nb_lignes(&ent, "ent_notification", etab).await,
        nb_lignes(&ent, "ent_metier_event", etab).await,
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ent_espace_membre_ajoute WHERE etablissement_id = $1 AND vu_le IS NULL")
            .bind(etab)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap(),
    );
    assert_eq!(avant, apres, "ND32-11 : une lecture n'écrit rien");
}

// ───────────────────────── NX32-10, 11, 13, 17, 19, 20 : espaces libres, membres, invitations ─────────────────────────

/// NX32-10, NX32-17, ND32-03 (ETR-106, ETR-150, ETR-151, R-ET12) : les membres sont **résolus par le cœur** : un élève, un
/// parent, un identifiant inconnu, une personne d'un autre établissement ou sans fonction active sont refusés avec le même
/// message ; un collègue de l'annuaire est accepté.
#[tokio::test]
async fn et1_membres_resolus_par_le_coeur_nd32_03() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe) = (cast.etablissement_id, &cast.prof, &cast.cpe);
    let (role, role_c) = (prof.role.as_str(), cpe.role.as_str());
    collegue(&ent, etab, prof.user_id).await;
    let pc = collegue(&ent, etab, cpe.user_id).await;
    let espace = libre(&ent, etab, prof.user_id, role, &uniq("Projet"), vec![]).await;
    // Candidats invalides.
    let eleve: Uuid = sqlx::query_scalar("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(cast.eleve_lea_id)
        .fetch_optional(ent.pool_verif())
        .await
        .unwrap()
        .unwrap_or_else(Uuid::new_v4);
    let parent = {
        // Un parent : fiche personne sans agent.
        let u = cast.parent_lea.user_id;
        let existante: Option<Uuid> = sqlx::query_scalar("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2")
            .bind(etab)
            .bind(u)
            .fetch_optional(ent.pool_verif())
            .await
            .unwrap();
        existante.unwrap_or_else(Uuid::new_v4)
    };
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let etranger = collegue(&ent, autre.etablissement_id, autre.prof.user_id).await;
    let sans_fonction = {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, source) VALUES ($1, $2, 'SansAgent', 'saisie')")
            .bind(id)
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .unwrap();
        id
    };
    let mut messages = std::collections::BTreeSet::new();
    for pid in [eleve, parent, etranger, sans_fonction, Uuid::new_v4()] {
        let e = ent
            .espaces_membre_ajouter(etab, prof.user_id, role, espace, NouveauMembre { personne_id: pid, role: RoleEspace::Lecteur }, t0())
            .await
            .unwrap_err();
        match e {
            EntDbError::Refus(RefusMetier::Invalide, m) => {
                messages.insert(m);
            }
            autre => panic!("refus attendu : {autre:?}"),
        }
    }
    assert_eq!(messages.len(), 1, "un seul message : aucun oracle sur l'existence d'une personne");
    // Création en un geste avec un membre invalide : rien n'est créé.
    let avant = nb_lignes(&ent, "ent_espace", etab).await;
    let e = ent
        .espaces_creer_libre(
            etab,
            prof.user_id,
            role,
            NouvelEspace {
                libelle: uniq("Raté"),
                description: None,
                couleur_code: None,
                icone_code: None,
                membres: vec![NouveauMembre { personne_id: eleve, role: RoleEspace::Lecteur }],
            },
            t0(),
        )
        .await;
    assert!(matches!(e, Err(EntDbError::Refus(RefusMetier::Invalide, _))));
    assert_eq!(nb_lignes(&ent, "ent_espace", etab).await, avant);
    // Un collègue de l'annuaire est accepté ; deux fois : conflit.
    ent.espaces_membre_ajouter(etab, prof.user_id, role, espace, NouveauMembre { personne_id: pc, role: RoleEspace::Contributeur }, t0()).await.unwrap();
    assert!(matches!(
        ent.espaces_membre_ajouter(etab, prof.user_id, role, espace, NouveauMembre { personne_id: pc, role: RoleEspace::Contributeur }, t0()).await,
        Err(EntDbError::Refus(RefusMetier::Conflit, _))
    ));
    // Un contributeur n'ajoute personne ; un non-membre ne sait même pas que l'espace existe.
    assert!(matches!(
        ent.espaces_membre_ajouter(etab, cpe.user_id, role_c, espace, NouveauMembre { personne_id: sans_fonction, role: RoleEspace::Lecteur }, t0()).await,
        Err(EntDbError::Refus(RefusMetier::Interdit, _) | EntDbError::Refus(RefusMetier::Invalide, _))
    ));
    let dir = &cast.direction;
    collegue(&ent, etab, dir.user_id).await;
    assert!(introuvable(
        ent.espaces_membre_ajouter(etab, dir.user_id, dir.role.as_str(), espace, NouveauMembre { personne_id: pc, role: RoleEspace::Lecteur }, t0()).await
    ));
    // Annuaire : le collègue est trouvé (hors moi), une saisie trop courte est refusée.
    let nom: String = sqlx::query_scalar("SELECT nom FROM ent_personne WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(pc)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let trouves = ent.espaces_chercher_collegues(etab, prof.user_id, role, &nom, t0()).await.unwrap();
    assert!(trouves.iter().any(|c| c.personne_id == pc));
    assert!(ent.espaces_chercher_collegues(etab, prof.user_id, role, "a", t0()).await.is_err());
}

/// NX32-19, NX32-20 (ETR-152) : l'invitation prévient le collègue **dans la transaction** (avis du socle 8, enveloppe neutre
/// sans nom d'espace ni de fichier), le badge « non vu » se lève par une commande explicite (ND32-11, ND32-12) ; pas d'avis pour
/// le créateur.
#[tokio::test]
async fn et1_invitation_avis_neutre_et_badge_nx32_19() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe) = (cast.etablissement_id, &cast.prof, &cast.cpe);
    let (role, role_c) = (prof.role.as_str(), cpe.role.as_str());
    collegue(&ent, etab, prof.user_id).await;
    let pc = collegue(&ent, etab, cpe.user_id).await;
    let avant = ent.espaces_invitations_non_vues(etab, cpe.user_id, role_c, t0()).await.unwrap().nombre;
    let espace = libre(&ent, etab, prof.user_id, role, "SecretLibelleEspace", vec![NouveauMembre { personne_id: pc, role: RoleEspace::Lecteur }]).await;
    assert_eq!(ent.espaces_invitations_non_vues(etab, cpe.user_id, role_c, t0()).await.unwrap().nombre, avant + 1);
    assert_eq!(ent.espaces_invitations_non_vues(etab, prof.user_id, role, t0()).await.unwrap().nombre, 0, "le créateur n'a pas d'invitation");
    // Avis neutre : jamais le nom de l'espace.
    let (titre, corps): (String, String) = sqlx::query_as(
        "SELECT titre, corps FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'espaces.invitation'
          ORDER BY created_at DESC LIMIT 1",
    )
    .bind(etab)
    .bind(cpe.user_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(!titre.contains("Secret") && !corps.contains("Secret"), "{titre} / {corps}");
    let evt_payload: String = sqlx::query_scalar("SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'espaces.membre_ajoute' ORDER BY created_at DESC LIMIT 1")
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(!evt_payload.contains("Secret"));
    // Levée explicite à l'ouverture de l'espace.
    let levees = ent.espaces_marquer_vu(etab, cpe.user_id, role_c, Some(espace), None, t0()).await.unwrap();
    assert_eq!(levees, 1);
    assert_eq!(ent.espaces_invitations_non_vues(etab, cpe.user_id, role_c, t0()).await.unwrap().nombre, avant);
}

/// NX32-11, NX32-13 (ETR-153, ETR-154, ETR-155) : retirer, quitter, transférer ; le **dernier animateur** ne part pas sans
/// transfert ; il reste toujours un animateur ; l'accès disparaît immédiatement.
#[tokio::test]
async fn et1_retirer_quitter_transferer_dernier_animateur() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe, dir) = (cast.etablissement_id, &cast.prof, &cast.cpe, &cast.direction);
    let (role, role_c, role_d) = (prof.role.as_str(), cpe.role.as_str(), dir.role.as_str());
    collegue(&ent, etab, prof.user_id).await;
    let pc = collegue(&ent, etab, cpe.user_id).await;
    let pd = collegue(&ent, etab, dir.user_id).await;
    let espace = libre(&ent, etab, prof.user_id, role, &uniq("Équipe"), vec![
        NouveauMembre { personne_id: pc, role: RoleEspace::Contributeur },
        NouveauMembre { personne_id: pd, role: RoleEspace::Lecteur },
    ])
    .await;
    // Le seul animateur ne part pas.
    assert!(matches!(ent.espaces_quitter(etab, prof.user_id, role, espace, t0()).await, Err(EntDbError::Refus(RefusMetier::Conflit, _))));
    // Un contributeur ne retire personne (autre que lui-même).
    assert!(matches!(
        ent.espaces_membre_retirer(etab, cpe.user_id, role_c, espace, pd, t0()).await,
        Err(EntDbError::Refus(RefusMetier::Interdit, _))
    ));
    // Transfert : le collègue devient animateur, moi contributeur.
    ent.espaces_transferer_animation(etab, prof.user_id, role, espace, pc, t0()).await.unwrap();
    let detail = ent.espaces_detail(etab, prof.user_id, role, espace, t0()).await.unwrap();
    let r = |p: Uuid| detail.membres.iter().find(|m| m.personne_id == p).map(|m| m.role);
    assert_eq!(r(pc), Some(RoleEspace::Animateur));
    assert_eq!(detail.espace.role, RoleEspace::Contributeur);
    // L'ancien animateur (contributeur) ne gère plus les membres.
    assert!(matches!(
        ent.espaces_membre_retirer(etab, prof.user_id, role, espace, pd, t0()).await,
        Err(EntDbError::Refus(RefusMetier::Interdit, _))
    ));
    // Le nouvel animateur retire le lecteur : accès coupé tout de suite ; l'historique est conservé.
    ent.espaces_membre_retirer(etab, cpe.user_id, role_c, espace, pd, t0()).await.unwrap();
    assert!(introuvable(ent.espaces_parcourir(etab, dir.user_id, role_d, espace, None, t0()).await));
    // Quitter : le contributeur part librement ; le dernier animateur (le collègue) ne part pas.
    ent.espaces_quitter(etab, prof.user_id, role, espace, t0()).await.unwrap();
    assert!(introuvable(ent.espaces_parcourir(etab, prof.user_id, role, espace, None, t0()).await));
    assert!(matches!(ent.espaces_quitter(etab, cpe.user_id, role_c, espace, t0()).await, Err(EntDbError::Refus(RefusMetier::Conflit, _))));
    let actifs: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_espace_membre_ajoute WHERE etablissement_id = $1 AND espace_id = $2 AND retire_le IS NULL")
        .bind(etab)
        .bind(espace)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(actifs, 1, "il reste un animateur");
    // Retour possible : l'historique garde les anciennes lignes, la contrainte de chevauchement ne gêne pas un retour.
    ent.espaces_membre_ajouter(etab, cpe.user_id, role_c, espace, NouveauMembre { personne_id: pd, role: RoleEspace::Lecteur }, t0()).await.unwrap();
    // Ouverture de l'espace personnel : on n'y est pas « membre », on ne le quitte pas.
    let perso = mon_espace(&ent, etab, dir.user_id, role_d).await;
    assert!(matches!(ent.espaces_quitter(etab, dir.user_id, role_d, perso, t0()).await, Err(EntDbError::Refus(RefusMetier::Conflit, _))));
}

// ───────────────────────── NX32-03, 07 : dossiers ─────────────────────────

/// NX32-03, NX32-07 (ETR-112, ETR-120, R-ET25, ND32-16) : noms propres, doublons refusés, profondeur ≤ 8, déplacement jamais
/// dans soi-même ni au-delà de huit niveaux, aucun objet ne bouge.
#[tokio::test]
async fn et1_dossiers_noms_profondeur_deplacement() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof) = (cast.etablissement_id, &cast.prof);
    let role = prof.role.as_str();
    let espace = libre(&ent, etab, prof.user_id, role, &uniq("Arbo"), vec![]).await;
    let creer = |parent: Option<Uuid>, nom: String| {
        let ent = &ent;
        async move { ent.espaces_dossier_creer(etab, prof.user_id, role, espace, parent, &nom, t0()).await }
    };
    for mauvais in ["", "   ", "a/b", "..", "x\u{202E}y", &"é".repeat(121)] {
        assert!(matches!(creer(None, mauvais.to_string()).await, Err(EntDbError::Refus(RefusMetier::Invalide, _))), "{mauvais:?}");
    }
    let racine = creer(None, "  Cours   de maths ".into()).await.unwrap();
    assert_eq!(racine.nom, "Cours de maths");
    assert!(matches!(creer(None, "cours de MATHS".into()).await, Err(EntDbError::Refus(RefusMetier::Conflit, _))), "doublon insensible à la casse");
    // Huit niveaux : le neuvième est refusé (ND32-16).
    let mut parent = racine.id;
    for n in 2..=8 {
        parent = creer(Some(parent), format!("niveau {n}")).await.unwrap().id;
    }
    assert!(matches!(creer(Some(parent), "niveau 9".into()).await, Err(EntDbError::Refus(RefusMetier::Conflit, _))));
    // Déplacements.
    let a = creer(None, "A".into()).await.unwrap();
    let a1 = creer(Some(a.id), "A1".into()).await.unwrap();
    let deplacer = |id: Uuid, vers: Option<Uuid>| {
        let ent = &ent;
        async move { ent.espaces_dossier_deplacer(etab, prof.user_id, role, id, vers, t0()).await }
    };
    assert!(matches!(deplacer(a.id, Some(a.id)).await, Err(EntDbError::Refus(RefusMetier::Conflit, _))), "dans lui-même");
    assert!(matches!(deplacer(a.id, Some(a1.id)).await, Err(EntDbError::Refus(RefusMetier::Conflit, _))), "dans un descendant");
    // « A » (profondeur 2 avec A1) sous le niveau 7 donnerait 9 niveaux : refusé ; sous le niveau 5, huit : accepté.
    let contenu_racine = ent.espaces_parcourir(etab, prof.user_id, role, espace, Some(racine.id), t0()).await.unwrap();
    assert_eq!(contenu_racine.chemin.len(), 1);
    let niveau_de = |n: usize| {
        let ent = &ent;
        async move {
            // Descend de n-1 niveaux depuis la racine.
            let mut courant = racine.id;
            for _ in 1..n {
                let c = ent.espaces_parcourir(etab, prof.user_id, role, espace, Some(courant), t0()).await.unwrap();
                courant = c.dossiers[0].id;
            }
            courant
        }
    };
    let n7 = niveau_de(7).await;
    assert!(matches!(deplacer(a.id, Some(n7)).await, Err(EntDbError::Refus(RefusMetier::Conflit, _))));
    let n5 = niveau_de(5).await;
    deplacer(a.id, Some(n5)).await.unwrap();
    // Nom pris à destination : refus.
    let b = creer(None, "A".into()).await.unwrap();
    assert!(matches!(deplacer(b.id, Some(n5)).await, Err(EntDbError::Refus(RefusMetier::Conflit, _))));
    // Renommage : nom libre accepté, nom d'un frère refusé.
    ent.espaces_dossier_renommer(etab, prof.user_id, role, b.id, "B", t0()).await.unwrap();
    assert!(matches!(
        ent.espaces_dossier_renommer(etab, prof.user_id, role, b.id, "Cours de maths", t0()).await,
        Err(EntDbError::Refus(RefusMetier::Conflit, _))
    ));
}

// ───────────────────────── NX32-04, 05, 06, ND32-02 : fichiers, versions, téléchargement ─────────────────────────

/// ND32-02, R-ET3 (ETR-114, ETR-160, ETR-161, ETR-162, ETR-164) : **jamais d'écrasement**. Un membre qui dépose sous le nom d'un
/// fichier existant reçoit « nom déjà pris » avec la proposition ; il choisit « nouvelle version » (le propriétaire garde sa
/// version, l'historique est listé) ou « nouveau fichier (2) » ; le retour arrière **crée** une version.
#[tokio::test]
async fn et1_jamais_d_ecrasement_versions_nd32_02() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe) = (cast.etablissement_id, &cast.prof, &cast.cpe);
    let (role, role_c) = (prof.role.as_str(), cpe.role.as_str());
    collegue(&ent, etab, prof.user_id).await;
    let pc = collegue(&ent, etab, cpe.user_id).await;
    let espace = libre(&ent, etab, prof.user_id, role, &uniq("Docs"), vec![NouveauMembre { personne_id: pc, role: RoleEspace::Contributeur }]).await;
    let v1 = pdf_unique(2048);
    let r1 = deposer_et_analyser(&ent, etab, prof.user_id, role, espace, demande("projet.pdf", v1.len()), &v1, t0()).await;
    assert_eq!((r1.numero, r1.etat.as_str()), (1, "analyse"));
    // Le collègue dépose sous le même nom : ni écrasement ni choix silencieux.
    let v2 = pdf_unique(4096);
    let rep = ent.espaces_demander_depot(etab, cpe.user_id, role_c, espace, demande("Projet.PDF", v2.len()), t0()).await.unwrap();
    let ReponseDepot::NomDejaPris { element_id, nom_propose } = rep else { panic!("nom déjà pris attendu") };
    assert_eq!((element_id, nom_propose.as_str()), (r1.element_id, "Projet (2).PDF"));
    assert_eq!(occupation(&ent, etab, espace).await.1, 0, "aucun quota réservé tant que le choix n'est pas fait");
    // Choix « nouvelle version ».
    let mut d = demande("projet.pdf", v2.len());
    d.si_existe = Some(ChoixSiExiste::NouvelleVersion);
    d.commentaire = Some("relecture du collègue".into());
    let r2 = deposer_et_analyser(&ent, etab, cpe.user_id, role_c, espace, d, &v2, t0() + Duration::minutes(5)).await;
    assert_eq!((r2.element_id, r2.numero), (r1.element_id, 2));
    // La version du propriétaire est intacte : téléchargeable, de même taille, de même empreinte.
    let versions = ent.espaces_versions(etab, prof.user_id, role, r1.element_id, t0()).await.unwrap();
    assert_eq!(versions.iter().map(|v| v.numero).collect::<Vec<_>>(), vec![2, 1]);
    assert!(versions[0].courante && !versions[1].courante);
    assert_eq!((versions[0].auteur_nom.is_empty(), versions[0].commentaire.as_deref()), (false, Some("relecture du collègue")));
    assert_eq!(versions[1].taille, v1.len() as i64);
    let cle_v1: String = sqlx::query_scalar(
        "SELECT o.cle_objet FROM ent_espace_version v JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
          WHERE v.etablissement_id = $1 AND v.element_id = $2 AND v.numero = 1",
    )
    .bind(etab)
    .bind(r1.element_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(ent.lire_objet(etab, &cle_v1).await.unwrap(), v1, "le contenu de la version 1 n'a pas bougé");
    ent.espaces_lien_telechargement(etab, prof.user_id, role, r1.element_id, Some(1), t0()).await.unwrap();
    // Choix « nouveau fichier » : un autre élément, nommé « (2) ».
    let v3 = pdf_unique(1024);
    let mut d3 = demande("projet.pdf", v3.len());
    d3.si_existe = Some(ChoixSiExiste::NouveauFichier);
    let r3 = deposer_et_analyser(&ent, etab, cpe.user_id, role_c, espace, d3, &v3, t0() + Duration::minutes(6)).await;
    assert_ne!(r3.element_id, r1.element_id);
    let c = ent.espaces_parcourir(etab, prof.user_id, role, espace, None, t0() + Duration::minutes(7)).await.unwrap();
    let noms: Vec<&str> = c.elements.iter().map(|e| e.nom.as_str()).collect();
    assert_eq!(noms, vec!["projet (2).pdf", "projet.pdf"]);
    // « Nouvelle version » sans élément du même nom : refus.
    let mut inconnu = demande("rien.pdf", 100);
    inconnu.si_existe = Some(ChoixSiExiste::NouvelleVersion);
    assert!(matches!(
        ent.espaces_demander_depot(etab, prof.user_id, role, espace, inconnu, t0()).await,
        Err(EntDbError::Refus(RefusMetier::Conflit, _))
    ));
    // Retour à la version 1 : une troisième version (copie), la 2 reste ; un contributeur non auteur ne le peut pas.
    assert!(matches!(
        ent.espaces_version_restaurer(etab, cpe.user_id, role_c, r1.element_id, 1, t0()).await,
        Err(EntDbError::Refus(RefusMetier::Interdit, _))
    ));
    let rest = ent.espaces_version_restaurer(etab, prof.user_id, role, r1.element_id, 1, t0() + Duration::minutes(8)).await.unwrap();
    assert_eq!((rest.numero, rest.etat.as_str()), (3, "disponible"));
    let versions = ent.espaces_versions(etab, prof.user_id, role, r1.element_id, t0()).await.unwrap();
    assert_eq!(versions.iter().map(|v| v.numero).collect::<Vec<_>>(), vec![3, 2, 1]);
    assert_eq!((versions[0].origine.as_str(), versions[0].restaure_de_numero, versions[0].courante), ("restauration", Some(1), true));
    // L'objet restauré est une copie octet pour octet.
    let cle_v3: String = sqlx::query_scalar(
        "SELECT o.cle_objet FROM ent_espace_version v JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
          WHERE v.etablissement_id = $1 AND v.element_id = $2 AND v.numero = 3",
    )
    .bind(etab)
    .bind(r1.element_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_ne!(cle_v3, cle_v1);
    assert_eq!(ent.lire_objet(etab, &cle_v3).await.unwrap(), v1);
    // Le quota compte toutes les versions (CET-21) : v1 + v2 + v3 + le fichier (2).
    let attendu = (v1.len() * 2 + v2.len() + v3.len()) as i64;
    assert_eq!(occupation(&ent, etab, espace).await, (attendu, 0));
    // Un lecteur ne dépose pas (rôle d'espace fermé, R-ET5).
    let lect = &cast.direction;
    let pl = collegue(&ent, etab, lect.user_id).await;
    ent.espaces_membre_ajouter(etab, prof.user_id, role, espace, NouveauMembre { personne_id: pl, role: RoleEspace::Lecteur }, t0()).await.unwrap();
    assert!(matches!(
        ent.espaces_demander_depot(etab, lect.user_id, lect.role.as_str(), espace, demande("autre.pdf", 100), t0()).await,
        Err(EntDbError::Refus(RefusMetier::Interdit, _))
    ));
}

/// NX32-06, ETR-118, R-ET9, ND32-06, ND32-10 : le téléchargement est un **lien court de 60 s lié au compte**, émis après contrôle
/// des droits ; un non-membre ne reçoit rien ; la clé d'objet ne contient jamais le nom.
#[tokio::test]
async fn et1_telechargement_lien_court_lie_au_compte_nx32_06() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe) = (cast.etablissement_id, &cast.prof, &cast.cpe);
    let (role, role_c) = (prof.role.as_str(), cpe.role.as_str());
    collegue(&ent, etab, prof.user_id).await;
    collegue(&ent, etab, cpe.user_id).await;
    let espace = libre(&ent, etab, prof.user_id, role, &uniq("Docs"), vec![]).await;
    let octets = pdf_unique(3000);
    let r = deposer_et_analyser(&ent, etab, prof.user_id, role, espace, demande("Bulletin Ada.pdf", octets.len()), &octets, t0()).await;
    let lien = ent.espaces_lien_telechargement(etab, prof.user_id, role, r.element_id, None, t0()).await.unwrap();
    let jeton = lien.url.trim_start_matches("/fichiers/");
    let c = ent.presigner().verify_local(jeton, t0().timestamp()).unwrap();
    assert_eq!((c.user_id, c.etablissement_id, c.filename.as_str()), (prof.user_id, etab, "Bulletin Ada.pdf"));
    assert_eq!(c.disposition, ent_object::presign::Disposition::Attachment, "toujours téléchargé, jamais interprété");
    assert!(!c.key.to_lowercase().contains("bulletin"), "clé d'objet opaque : {}", c.key);
    assert_eq!(lien.expire_le, t0() + Duration::seconds(60));
    assert!(ent.presigner().verify_local(jeton, t0().timestamp() + 60).is_err(), "expiré après 60 s");
    // Le contenu servi est celui du registre (empreinte contrôlée).
    assert_eq!(ent.lire_objet(etab, &c.key).await.unwrap(), octets);
    // Aperçu minimal : PDF servi en ligne (type sûr) ; une archive reste en téléchargement ; mêmes droits que le téléchargement.
    let ap = ent.espaces_lien_apercu(etab, prof.user_id, role, r.element_id, None, t0()).await.unwrap();
    let c_ap = ent.presigner().verify_local(ap.url.trim_start_matches("/fichiers/"), t0().timestamp()).unwrap();
    assert_eq!((c_ap.disposition, c_ap.mime.as_str()), (ent_object::presign::Disposition::Inline, "application/pdf"));
    assert!(introuvable(ent.espaces_lien_apercu(etab, cpe.user_id, role_c, r.element_id, None, t0()).await));
    // Un non-membre : introuvable ; une version inexistante : introuvable ; élément inconnu : introuvable.
    assert!(introuvable(ent.espaces_lien_telechargement(etab, cpe.user_id, role_c, r.element_id, None, t0()).await));
    assert!(introuvable(ent.espaces_lien_telechargement(etab, prof.user_id, role, r.element_id, Some(9), t0()).await));
    assert!(introuvable(ent.espaces_lien_telechargement(etab, prof.user_id, role, Uuid::new_v4(), None, t0()).await));
}

/// NX32-05 (ETR-115) : une arborescence déposée = dossiers créés puis **un dépôt par fichier** ; un fichier refusé n'empêche
/// pas les autres (le bilan des refus est celui des résultats).
#[tokio::test]
async fn et1_arborescence_un_depot_par_fichier_bilan_nx32_05() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof) = (cast.etablissement_id, &cast.prof);
    let role = prof.role.as_str();
    let espace = libre(&ent, etab, prof.user_id, role, &uniq("Glisser"), vec![]).await;
    let projet = ent.espaces_dossier_creer(etab, prof.user_id, role, espace, None, "Projet", t0()).await.unwrap();
    let images = ent.espaces_dossier_creer(etab, prof.user_id, role, espace, Some(projet.id), "images", t0()).await.unwrap();
    let mut bilan = Vec::new();
    let lot: Vec<(Option<Uuid>, &str, Vec<u8>)> = vec![
        (Some(projet.id), "plan.pdf", pdf_unique(500)),
        (Some(images.id), "photo.pdf", pdf_unique(700)),
        (Some(images.id), "refuse.pdf", b"<html><script>x</script></html>".to_vec()),
        (Some(projet.id), "notes.txt", b"du texte simple".to_vec()),
    ];
    for (dossier, nom, octets) in &lot {
        let mut d = demande(nom, octets.len());
        d.dossier_id = *dossier;
        bilan.push(deposer(&ent, etab, prof.user_id, role, espace, d, octets, t0()).await.map(|_| ()));
    }
    assert_eq!(bilan.iter().filter(|r| r.is_ok()).count(), 3);
    assert!(matches!(bilan[2], Err(EntDbError::Fichier(_))));
    ent.espaces_analyser_versions(etab, sain(), 50, t0()).await.unwrap();
    let c = ent.espaces_parcourir(etab, prof.user_id, role, espace, Some(images.id), t0()).await.unwrap();
    assert_eq!(c.chemin.iter().map(|d| d.nom.as_str()).collect::<Vec<_>>(), vec!["Projet", "images"]);
    assert_eq!(c.elements.iter().map(|e| e.nom.as_str()).collect::<Vec<_>>(), vec!["photo.pdf"]);
    // Un dossier d'un autre espace n'accepte pas de dépôt (clé composite : même espace).
    let autre = libre(&ent, etab, prof.user_id, role, &uniq("Autre"), vec![]).await;
    let mut d = demande("x.pdf", 100);
    d.dossier_id = Some(projet.id);
    assert!(introuvable(ent.espaces_demander_depot(etab, prof.user_id, role, autre, d, t0()).await));
}

/// ND32-09, R-ET4 (ETR-120) : renommer ou déplacer un fichier ne touche **jamais** l'objet stocké ; refus si le nom est pris à
/// destination (avec « (2) » proposé au dépôt) ; un partage de l'élément suit le déplacement (ND32-08, R-ET13).
#[tokio::test]
async fn et1_deplacer_renommer_sans_toucher_l_objet_et_partage_qui_suit() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe) = (cast.etablissement_id, &cast.prof, &cast.cpe);
    let (role, role_c) = (prof.role.as_str(), cpe.role.as_str());
    collegue(&ent, etab, prof.user_id).await;
    let pc = collegue(&ent, etab, cpe.user_id).await;
    let espace = libre(&ent, etab, prof.user_id, role, &uniq("Rangement"), vec![]).await;
    let octets = pdf_unique(900);
    let r = deposer_et_analyser(&ent, etab, prof.user_id, role, espace, demande("a.pdf", octets.len()), &octets, t0()).await;
    let cle = |ent: &PgEnt| {
        let (etab, el) = (etab, r.element_id);
        let pool = ent.pool_verif().clone();
        async move {
            sqlx::query_scalar::<_, String>(
                "SELECT o.cle_objet FROM ent_espace_version v JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
                  WHERE v.etablissement_id = $1 AND v.element_id = $2 AND v.numero = 1",
            )
            .bind(etab)
            .bind(el)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    let avant = cle(&ent).await;
    let a_part = ent.espaces_partager(etab, prof.user_id, role, DemandePartage {
        element_id: Some(r.element_id), dossier_id: None, destinataires: vec![NouveauPartage { destinataire_personne_id: pc, droit: DroitPartage::Lecture }], fin: None,
    }, t0()).await.unwrap();
    assert_eq!(a_part.len(), 1);
    let dossier = ent.espaces_dossier_creer(etab, prof.user_id, role, espace, None, "Archives", t0()).await.unwrap();
    ent.espaces_element_deplacer(etab, prof.user_id, role, r.element_id, Some(dossier.id), t0()).await.unwrap();
    ent.espaces_element_renommer(etab, prof.user_id, role, r.element_id, "renomme.pdf", t0()).await.unwrap();
    assert_eq!(cle(&ent).await, avant, "R-ET4 : l'objet n'a pas bougé");
    assert!(ent.objects().existe(&avant).unwrap());
    // Le destinataire accède toujours à l'élément (le partage porte l'identifiant), sous son nouveau nom et dans son nouveau dossier.
    let recus = ent.espaces_partages_avec_moi(etab, cpe.user_id, role_c, t0()).await.unwrap();
    assert!(recus.iter().any(|p| p.element_id == Some(r.element_id) && p.nom == "renomme.pdf"));
    ent.espaces_lien_telechargement(etab, cpe.user_id, role_c, r.element_id, None, t0()).await.unwrap();
    // Nom pris à destination : refus (le déplacement ne propose pas d'écraser).
    let b = deposer_et_analyser(&ent, etab, prof.user_id, role, espace, demande("renomme.pdf", 100), &pdf_unique(100), t0()).await;
    assert!(matches!(
        ent.espaces_element_deplacer(etab, prof.user_id, role, b.element_id, Some(dossier.id), t0()).await,
        Err(EntDbError::Refus(RefusMetier::Conflit, _))
    ));
    // À la racine, « RENOMME.pdf » est libre (l'autre fichier est dans « Archives ») : le renommage passe, insensible à la casse
    // seulement entre frères.
    ent.espaces_element_renommer(etab, prof.user_id, role, b.element_id, "RENOMME.pdf", t0()).await.unwrap();
    // Un nom à extension active ou un séparateur de chemin est refusé au renommage.
    assert!(ent.espaces_element_renommer(etab, prof.user_id, role, b.element_id, "../x.pdf", t0()).await.is_err());
    assert!(ent.espaces_element_renommer(etab, prof.user_id, role, b.element_id, "page.html.pdf", t0()).await.is_err());
}

// ───────────────────────── NX32-08, 09 : corbeille, restauration, purge ─────────────────────────

/// NX32-08, NX32-09, ETR-122 à ETR-125, S-3205, R-ET6, CET-20 : corbeille de 30 jours ; un dossier se supprime après **aperçu**
/// et confirmation ; les fichiers d'autrui sont listés (et interdisent la suppression à un non-animateur) ; restauration sans
/// perte (nom libre ou « (2) ») ; effacement à l'échéance (instants fixes) qui rend le quota ; une version **figée** par un autre
/// socle n'est jamais effacée.
#[tokio::test]
async fn et1_corbeille_apercu_restauration_purge_figee() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe) = (cast.etablissement_id, &cast.prof, &cast.cpe);
    let (role, role_c) = (prof.role.as_str(), cpe.role.as_str());
    collegue(&ent, etab, prof.user_id).await;
    let pc = collegue(&ent, etab, cpe.user_id).await;
    let espace = libre(&ent, etab, prof.user_id, role, &uniq("Corbeille"), vec![NouveauMembre { personne_id: pc, role: RoleEspace::Contributeur }]).await;
    let dossier = ent.espaces_dossier_creer(etab, cpe.user_id, role_c, espace, None, "Dossier du collègue", t0()).await.unwrap();
    let mut d = demande("mien.pdf", 600);
    d.dossier_id = Some(dossier.id);
    let mien = deposer_et_analyser(&ent, etab, prof.user_id, role, espace, d, &pdf_unique(600), t0()).await;
    let mut d2 = demande("sien.pdf", 700);
    d2.dossier_id = Some(dossier.id);
    let sien = deposer_et_analyser(&ent, etab, cpe.user_id, role_c, espace, d2, &pdf_unique(700), t0()).await;
    assert_eq!(occupation(&ent, etab, espace).await.0, 1300);
    // Aperçu sans confirmation : rien n'est supprimé ; les fichiers d'autrui sont listés pour le collègue.
    let apercu = ent.espaces_dossier_corbeille(etab, cpe.user_id, role_c, dossier.id, false, t0()).await.unwrap();
    assert_eq!((apercu.nb_dossiers, apercu.nb_elements, apercu.fait), (1, 2, false));
    assert_eq!(apercu.elements_d_autrui, vec!["mien.pdf"]);
    assert_eq!(ent.espaces_parcourir(etab, cpe.user_id, role_c, espace, None, t0()).await.unwrap().dossiers.len(), 1);
    // L'auteur du dossier (contributeur) ne peut pas l'emporter avec le fichier d'un autre.
    assert!(matches!(
        ent.espaces_dossier_corbeille(etab, cpe.user_id, role_c, dossier.id, true, t0()).await,
        Err(EntDbError::Refus(RefusMetier::Interdit, _))
    ));
    // L'animateur confirme : tout part dans la corbeille ; rien d'effacé ; le quota reste compté.
    let fait = ent.espaces_dossier_corbeille(etab, prof.user_id, role, dossier.id, true, t0()).await.unwrap();
    assert!(fait.fait);
    let c = ent.espaces_parcourir(etab, prof.user_id, role, espace, None, t0()).await.unwrap();
    assert!(c.dossiers.is_empty() && c.elements.is_empty());
    assert_eq!(occupation(&ent, etab, espace).await.0, 1300);
    let lignes = ent.espaces_corbeille_lister(etab, prof.user_id, role, espace, t0()).await.unwrap();
    assert_eq!(lignes.len(), 3, "le dossier, et ses deux fichiers");
    assert!(lignes.iter().all(|l| l.purge_le == t0() + Duration::days(30)));
    // Le collègue ne voit dans la corbeille que ce qu'il a supprimé ou créé.
    let siennes = ent.espaces_corbeille_lister(etab, cpe.user_id, role_c, espace, t0()).await.unwrap();
    assert!(siennes.iter().any(|l| l.nom == "sien.pdf"));
    assert!(!siennes.iter().any(|l| l.nom == "mien.pdf"));
    // Un fichier supprimé avec son dossier se restaure par son dossier ; il n'est plus téléchargeable.
    assert!(introuvable(ent.espaces_lien_telechargement(etab, prof.user_id, role, mien.element_id, None, t0()).await));
    assert!(matches!(
        ent.espaces_restaurer(etab, prof.user_id, role, "element", mien.element_id, t0()).await,
        Err(EntDbError::Refus(RefusMetier::Conflit, _))
    ));
    // Entre-temps un nouveau dossier de même nom est créé : la restauration le renomme « (2) » (rien n'est perdu).
    ent.espaces_dossier_creer(etab, prof.user_id, role, espace, None, "Dossier du collègue", t0() + Duration::days(1)).await.unwrap();
    ent.espaces_restaurer(etab, prof.user_id, role, "dossier", dossier.id, t0() + Duration::days(1)).await.unwrap();
    let c = ent.espaces_parcourir(etab, prof.user_id, role, espace, None, t0() + Duration::days(1)).await.unwrap();
    assert_eq!(c.dossiers.iter().map(|d| d.nom.as_str()).collect::<Vec<_>>(), vec!["Dossier du collègue", "Dossier du collègue (2)"]);
    ent.espaces_lien_telechargement(etab, prof.user_id, role, mien.element_id, None, t0() + Duration::days(1)).await.unwrap();
    // Fichier seul à la corbeille (auteur), puis purge à l'échéance : 29 jours, rien ; 30 jours, effacé, quota rendu, objet effacé.
    let cle_sien: String = sqlx::query_scalar(
        "SELECT o.cle_objet FROM ent_espace_version v JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
          WHERE v.etablissement_id = $1 AND v.element_id = $2",
    )
    .bind(etab)
    .bind(sien.element_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let t_supp = t0() + Duration::days(2);
    ent.espaces_element_corbeille(etab, cpe.user_id, role_c, sien.element_id, t_supp).await.unwrap();
    ent.espaces_purger_corbeille(etab, 1000, t_supp + Duration::days(29)).await.unwrap();
    assert!(ent.objects().existe(&cle_sien).unwrap(), "29 jours : rien n'est effacé");
    assert_eq!(occupation(&ent, etab, espace).await.0, 1300);
    let b = ent.espaces_purger_corbeille(etab, 1000, t_supp + Duration::days(30)).await.unwrap();
    assert!(b.effaces >= 1, "{b:?}");
    assert!(!ent.objects().existe(&cle_sien).unwrap(), "objet effacé physiquement après la validation");
    assert_eq!(occupation(&ent, etab, espace).await.0, 600, "quota rendu");
    let (etat, supprime): (String, bool) = sqlx::query_as(
        "SELECT e.etat, o.supprime_le IS NOT NULL FROM ent_espace_element e
           JOIN ent_espace_version v ON v.etablissement_id = e.etablissement_id AND v.element_id = e.id
           JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
          WHERE e.etablissement_id = $1 AND e.id = $2",
    )
    .bind(etab)
    .bind(sien.element_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((etat.as_str(), supprime), ("efface", true));
    // R-ET6 : une version figée par un autre socle n'est jamais effacée (ni par l'échéance, ni en « vider la corbeille »).
    let t_f = t0() + Duration::days(40);
    ent.espaces_element_corbeille(etab, prof.user_id, role, mien.element_id, t_f).await.unwrap();
    sqlx::query("INSERT INTO ent_espace_version_figee (version_id, etablissement_id, socle_code, reference_id) VALUES ($1, $2, 'remise_20', $3)")
        .bind(mien.version_id)
        .bind(etab)
        .bind(Uuid::new_v4())
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let b = ent.espaces_purger_corbeille(etab, 1000, t_f + Duration::days(60)).await.unwrap();
    assert!(b.gardes >= 1, "{b:?}");
    let etat_mien: String = sqlx::query_scalar("SELECT etat FROM ent_espace_element WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(mien.element_id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(etat_mien, "corbeille", "R-ET6 : la version figée n'est pas effacée");
    assert_eq!(ent.espaces_vider_corbeille(etab, prof.user_id, role, espace, t_f + Duration::days(60)).await.unwrap(), (0, 1));
    let lignes = ent.espaces_corbeille_lister(etab, prof.user_id, role, espace, t_f + Duration::days(60)).await.unwrap();
    assert!(lignes.iter().any(|l| l.id == mien.element_id && l.figee));
    // Vider la corbeille : l'animateur seul ; efface les autres éléments à la demande.
    let autre = deposer_et_analyser(&ent, etab, prof.user_id, role, espace, demande("jetable.pdf", 300), &pdf_unique(300), t_f).await;
    ent.espaces_element_corbeille(etab, prof.user_id, role, autre.element_id, t_f).await.unwrap();
    assert!(matches!(
        ent.espaces_vider_corbeille(etab, cpe.user_id, role_c, espace, t_f).await,
        Err(EntDbError::Refus(RefusMetier::Interdit, _))
    ));
    assert_eq!(ent.espaces_vider_corbeille(etab, prof.user_id, role, espace, t_f).await.unwrap(), (1, 1));
}

// ───────────────────────── NX32-14, 15, 16 : partages nominatifs ─────────────────────────

/// NX32-14, NX32-15, NX32-16, NX32-02 (ETR-130 à ETR-134) : partage nominatif d'un fichier — lecture ou écriture, daté ;
/// « Partagés avec moi » ; quitter ; retirer (effet immédiat) ; remplacer (jamais empiler) ; un destinataire ne repartage pas ;
/// la lecture seule ne dépose pas, l'écriture dépose une **version** ; la date de fin coupe l'accès (instants fixes).
#[tokio::test]
async fn et1_partages_nominatifs_nx32_14_a_16() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe, dir) = (cast.etablissement_id, &cast.prof, &cast.cpe, &cast.direction);
    let (role, role_c, role_d) = (prof.role.as_str(), cpe.role.as_str(), dir.role.as_str());
    collegue(&ent, etab, prof.user_id).await;
    let pc = collegue(&ent, etab, cpe.user_id).await;
    let pd = collegue(&ent, etab, dir.user_id).await;
    let perso = mon_espace(&ent, etab, prof.user_id, role).await;
    let nom = format!("{}.pdf", uniq("partage"));
    let octets = pdf_unique(1500);
    let r = deposer_et_analyser(&ent, etab, prof.user_id, role, perso, demande(&nom, octets.len()), &octets, t0()).await;
    let demande_p = |droit, fin| DemandePartage {
        element_id: Some(r.element_id), dossier_id: None, destinataires: vec![NouveauPartage { destinataire_personne_id: pc, droit }], fin,
    };
    // Un collègue sans lien avec l'espace personnel ne le voit pas.
    assert!(introuvable(ent.espaces_lien_telechargement(etab, cpe.user_id, role_c, r.element_id, None, t0()).await));
    // Partage en lecture jusqu'au 12 mars (inclus).
    let p = ent.espaces_partager(etab, prof.user_id, role, demande_p(DroitPartage::Lecture, Some(jour(2027, 3, 12))), t0()).await.unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!((p[0].debut, p[0].fin, p[0].droit), (jour(2027, 3, 10), Some(jour(2027, 3, 12)), DroitPartage::Lecture));
    let recus = ent.espaces_partages_avec_moi(etab, cpe.user_id, role_c, t0()).await.unwrap();
    let mien = recus.iter().find(|x| x.element_id == Some(r.element_id)).expect("partagé avec moi");
    assert!(!mien.vu && mien.nom == nom);
    assert!(ent.espaces_invitations_non_vues(etab, cpe.user_id, role_c, t0()).await.unwrap().nombre >= 1);
    ent.espaces_lien_telechargement(etab, cpe.user_id, role_c, r.element_id, None, t0()).await.unwrap();
    // Le destinataire ne repartage pas, ne renomme pas, ne supprime pas, ne dépose pas (lecture seule).
    let repartage = DemandePartage {
        element_id: Some(r.element_id), dossier_id: None, destinataires: vec![NouveauPartage { destinataire_personne_id: pd, droit: DroitPartage::Lecture }], fin: None,
    };
    assert!(matches!(ent.espaces_partager(etab, cpe.user_id, role_c, repartage, t0()).await, Err(EntDbError::Refus(RefusMetier::Interdit, _))));
    assert!(ent.espaces_element_renommer(etab, cpe.user_id, role_c, r.element_id, "x.pdf", t0()).await.is_err());
    assert!(ent.espaces_element_corbeille(etab, cpe.user_id, role_c, r.element_id, t0()).await.is_err());
    let mut dv = demande(&nom, 200);
    dv.si_existe = Some(ChoixSiExiste::NouvelleVersion);
    assert!(ent.espaces_demander_depot(etab, cpe.user_id, role_c, perso, dv.clone(), t0()).await.is_err());
    assert!(matches!(
        ent.espaces_demander_version(etab, cpe.user_id, role_c, r.element_id, 100, None, None, t0()).await,
        Err(EntDbError::Refus(RefusMetier::Interdit, _))
    ));
    // À qui est-ce partagé ? (auteur) ; le destinataire n'a pas la liste.
    assert_eq!(ent.espaces_partages_de(etab, prof.user_id, role, Some(r.element_id), None, t0()).await.unwrap().len(), 1);
    assert!(introuvable(ent.espaces_partages_de(etab, cpe.user_id, role_c, Some(r.element_id), None, t0()).await));
    // Passage en écriture (remplacement, pas d'empilement) : il peut déposer une version, jamais écraser.
    let p2 = ent.espaces_partager(etab, prof.user_id, role, demande_p(DroitPartage::Ecriture, None), t0() + Duration::hours(1)).await.unwrap();
    assert_eq!(p2[0].id, p[0].id);
    assert_eq!(ent.espaces_partages_de(etab, prof.user_id, role, Some(r.element_id), None, t0()).await.unwrap().len(), 1);
    // Le destinataire n'est pas membre de l'espace : il dépose une version par l'identifiant de l'élément partagé.
    let a = ent
        .espaces_demander_version(etab, cpe.user_id, role_c, r.element_id, 700, None, Some("ma relecture".into()), t0() + Duration::hours(1))
        .await
        .expect("un partage en écriture permet de déposer une nouvelle version");
    let v_part = pdf_unique(700);
    ent.objects().put(&cle_reservee(&ent, etab, a.reservation_id).await, &v_part).unwrap();
    let rv = ent.espaces_finaliser_depot(etab, cpe.user_id, role_c, a.reservation_id, t0() + Duration::hours(1)).await.unwrap();
    assert_eq!((rv.element_id, rv.numero), (r.element_id, 2));
    ent.espaces_analyser_versions(etab, sain(), 200, t0() + Duration::hours(1)).await.unwrap();
    let vs = ent.espaces_versions(etab, prof.user_id, role, r.element_id, t0() + Duration::hours(1)).await.unwrap();
    assert_eq!(vs.iter().map(|v| v.numero).collect::<Vec<_>>(), vec![2, 1], "la version du propriétaire reste");
    // Le destinataire en lecture seule n'obtient pas cette voie (testé plus haut : refusée) ; un tiers non plus.
    assert!(introuvable(ent.espaces_demander_version(etab, dir.user_id, role_d, r.element_id, 100, None, None, t0() + Duration::hours(1)).await));
    // Quitter un partage reçu : accès coupé, l'auteur garde tout.
    ent.espaces_partage_quitter(etab, cpe.user_id, role_c, p[0].id, t0() + Duration::hours(2)).await.unwrap();
    assert!(introuvable(ent.espaces_lien_telechargement(etab, cpe.user_id, role_c, r.element_id, None, t0() + Duration::hours(2)).await));
    assert!(ent.espaces_lien_telechargement(etab, prof.user_id, role, r.element_id, None, t0() + Duration::hours(2)).await.is_ok());
    // Nouveau partage daté : la date de fin coupe l'accès le lendemain de la fin (jour de l'établissement).
    let p3 = ent.espaces_partager(etab, prof.user_id, role, demande_p(DroitPartage::Lecture, Some(jour(2027, 3, 12))), t0() + Duration::days(1)).await.unwrap();
    let t_fin = t0() + Duration::days(2); // 12 mars : dernier jour
    assert!(ent.espaces_lien_telechargement(etab, cpe.user_id, role_c, r.element_id, None, t_fin).await.is_ok());
    let t_apres = t0() + Duration::days(3); // 13 mars
    assert!(introuvable(ent.espaces_lien_telechargement(etab, cpe.user_id, role_c, r.element_id, None, t_apres).await));
    assert!(ent.espaces_partages_avec_moi(etab, cpe.user_id, role_c, t_apres).await.unwrap().iter().all(|x| x.element_id != Some(r.element_id)));
    // Prolonger (auteur) : l'accès revient ; retirer : immédiat.
    ent.espaces_partage_modifier_fin(etab, prof.user_id, role, p3[0].id, None, t_apres).await.unwrap();
    assert!(ent.espaces_lien_telechargement(etab, cpe.user_id, role_c, r.element_id, None, t_apres).await.is_ok());
    assert!(matches!(
        ent.espaces_partage_modifier_fin(etab, prof.user_id, role, p3[0].id, Some(jour(2027, 3, 1)), t_apres).await,
        Err(EntDbError::Refus(RefusMetier::Invalide, _))
    ));
    ent.espaces_partage_retirer(etab, prof.user_id, role, p3[0].id, t_apres).await.unwrap();
    assert!(introuvable(ent.espaces_lien_telechargement(etab, cpe.user_id, role_c, r.element_id, None, t_apres).await));
    // Un tiers ne retire ni ne quitte le partage d'un autre.
    let p4 = ent.espaces_partager(etab, prof.user_id, role, demande_p(DroitPartage::Lecture, None), t_apres).await.unwrap();
    assert!(introuvable(ent.espaces_partage_retirer(etab, dir.user_id, role_d, p4[0].id, t_apres).await));
    assert!(introuvable(ent.espaces_partage_quitter(etab, dir.user_id, role_d, p4[0].id, t_apres).await));
}

/// R-ET12, R-ET13, ND32-03 : on ne partage qu'avec un agent actif de l'établissement (jamais un élève, un parent, un inconnu, un
/// autre établissement) ; un partage de dossier couvre son contenu et ses sous-dossiers ; un fichier non analysé ne se partage
/// pas ; un partage vers soi-même est refusé.
#[tokio::test]
async fn et1_partage_destinataires_resolus_et_dossier_recursif() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe) = (cast.etablissement_id, &cast.prof, &cast.cpe);
    let (role, role_c) = (prof.role.as_str(), cpe.role.as_str());
    let pp = collegue(&ent, etab, prof.user_id).await;
    let pc = collegue(&ent, etab, cpe.user_id).await;
    let perso = mon_espace(&ent, etab, prof.user_id, role).await;
    let dossier = ent.espaces_dossier_creer(etab, prof.user_id, role, perso, None, &uniq("Partagé"), t0()).await.unwrap();
    let sous = ent.espaces_dossier_creer(etab, prof.user_id, role, perso, Some(dossier.id), "sous", t0()).await.unwrap();
    let mut d = demande("dans-sous.pdf", 400);
    d.dossier_id = Some(sous.id);
    let el = deposer_et_analyser(&ent, etab, prof.user_id, role, perso, d, &pdf_unique(400), t0()).await;
    let partage = |dest: Uuid| DemandePartage {
        element_id: None, dossier_id: Some(dossier.id), destinataires: vec![NouveauPartage { destinataire_personne_id: dest, droit: DroitPartage::Lecture }], fin: None,
    };
    // Destinataires refusés.
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let etranger = collegue(&ent, autre.etablissement_id, autre.prof.user_id).await;
    let eleve: Uuid = sqlx::query_scalar("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(cast.eleve_lea_id)
        .fetch_optional(ent.pool_verif())
        .await
        .unwrap()
        .unwrap_or_else(Uuid::new_v4);
    for dest in [pp, etranger, eleve, Uuid::new_v4()] {
        assert!(
            matches!(ent.espaces_partager(etab, prof.user_id, role, partage(dest), t0()).await, Err(EntDbError::Refus(RefusMetier::Invalide, _))),
            "destinataire {dest} refusé"
        );
    }
    // Ni élément ni dossier, ou les deux : refus. Liste vide, ou trop longue : refus.
    let mut deux = partage(pc);
    deux.element_id = Some(el.element_id);
    assert!(ent.espaces_partager(etab, prof.user_id, role, deux, t0()).await.is_err());
    let mut vide = partage(pc);
    vide.destinataires.clear();
    assert!(ent.espaces_partager(etab, prof.user_id, role, vide, t0()).await.is_err());
    // Avant le partage : rien. Après : le collègue parcourt le dossier, le sous-dossier et télécharge l'élément du sous-dossier.
    assert!(introuvable(ent.espaces_parcourir(etab, cpe.user_id, role_c, perso, Some(dossier.id), t0()).await));
    ent.espaces_partager(etab, prof.user_id, role, partage(pc), t0()).await.unwrap();
    let c = ent.espaces_parcourir(etab, cpe.user_id, role_c, perso, Some(dossier.id), t0()).await.unwrap();
    assert_eq!(c.dossiers.len(), 1);
    assert!(!c.peut_ecrire);
    let c2 = ent.espaces_parcourir(etab, cpe.user_id, role_c, perso, Some(sous.id), t0()).await.unwrap();
    assert_eq!(c2.elements.len(), 1);
    ent.espaces_lien_telechargement(etab, cpe.user_id, role_c, el.element_id, None, t0()).await.unwrap();
    // La racine de l'espace personnel reste fermée : un partage ne donne pas l'espace.
    assert!(introuvable(ent.espaces_parcourir(etab, cpe.user_id, role_c, perso, None, t0()).await));
    // Un dossier d'un autre espace de l'auteur ne se lit pas par l'identifiant du dossier partagé.
    let autre_espace = libre(&ent, etab, prof.user_id, role, &uniq("Autre"), vec![]).await;
    assert!(introuvable(ent.espaces_parcourir(etab, cpe.user_id, role_c, autre_espace, Some(dossier.id), t0()).await));
    // Fichier non analysé : pas de partage.
    let brut = deposer(&ent, etab, prof.user_id, role, perso, demande(&format!("{}.pdf", uniq("brut")), 300), &pdf_unique(300), t0()).await.unwrap();
    let e = ent
        .espaces_partager(etab, prof.user_id, role, DemandePartage {
            element_id: Some(brut.element_id), dossier_id: None, destinataires: vec![NouveauPartage { destinataire_personne_id: pc, droit: DroitPartage::Lecture }], fin: None,
        }, t0())
        .await;
    assert!(matches!(e, Err(EntDbError::Refus(RefusMetier::Conflit, _))), "{e:?}");
}

// ───────────────────────── NX32-18 : quota ─────────────────────────

/// NX32-18 (ETR-260, R-ET20) : mon quota et ce qui l'occupe, par espace et par famille de format, versions comprises ; un dépôt
/// qui dépasserait le quota de l'espace est refusé (le volume d'un espace collectif est compté à l'espace, pas à la personne).
#[tokio::test]
async fn et1_quota_vue_et_depassement_nx32_18() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe) = (cast.etablissement_id, &cast.prof, &cast.cpe);
    let (role, role_c) = (prof.role.as_str(), cpe.role.as_str());
    collegue(&ent, etab, prof.user_id).await;
    let pc = collegue(&ent, etab, cpe.user_id).await;
    let espace = libre(&ent, etab, prof.user_id, role, &uniq("Quota"), vec![NouveauMembre { personne_id: pc, role: RoleEspace::Contributeur }]).await;
    fixer_quota(&ent, etab, espace, (3 * MIO) as i64).await;
    deposer_et_analyser(&ent, etab, prof.user_id, role, espace, demande("a.pdf", MIO), &pdf_unique(MIO), t0()).await;
    let texte: &[u8] = b"du texte brut a compter";
    deposer_et_analyser(&ent, etab, cpe.user_id, role_c, espace, demande("b.txt", texte.len()), texte, t0()).await;
    let vue = ent.espaces_quota(etab, prof.user_id, role, t0()).await.unwrap();
    let q = vue.iter().find(|q| q.espace_id == espace).unwrap();
    assert_eq!(q.quota_octets, (3 * MIO) as i64);
    assert!(q.octets_utilises >= MIO as i64);
    assert!(q.par_type.iter().any(|(f, n)| f == "document" && *n >= MIO as i64));
    // Le dépôt du collègue est compté à l'espace : 2 Mio de plus dépassent les 3 Mio.
    let e = ent.espaces_demander_depot(etab, cpe.user_id, role_c, espace, demande("c.pdf", 2 * MIO), t0()).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(RefusMetier::Conflit, m) if m.contains("quota")), "{e:?}");
    // L'occupation exacte d'un collègue n'est pas lisible par un tiers : un non-membre n'a pas la ligne de cet espace.
    let tiers = &cast.direction;
    collegue(&ent, etab, tiers.user_id).await;
    let vue_t = ent.espaces_quota(etab, tiers.user_id, tiers.role.as_str(), t0()).await.unwrap();
    assert!(!vue_t.iter().any(|q| q.espace_id == espace));
}

// ───────────────────────── Journal, cloisonnement, droits ─────────────────────────

/// §15 point 6 : journal en **ajout seul** ; l'animateur lit celui de son espace ; la direction (`espaces.journal.lire`) lit celui
/// d'un espace collectif mais **jamais** celui d'un espace personnel ; un membre simple ou un tiers : introuvable.
#[tokio::test]
async fn et1_journal_ajout_seul_et_lecture_encadree() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, cpe, dir) = (cast.etablissement_id, &cast.prof, &cast.cpe, &cast.direction);
    let (role, role_c, role_d) = (prof.role.as_str(), cpe.role.as_str(), dir.role.as_str());
    collegue(&ent, etab, prof.user_id).await;
    let pc = collegue(&ent, etab, cpe.user_id).await;
    collegue(&ent, etab, dir.user_id).await;
    let espace = libre(&ent, etab, prof.user_id, role, &uniq("Journal"), vec![NouveauMembre { personne_id: pc, role: RoleEspace::Lecteur }]).await;
    ent.espaces_dossier_creer(etab, prof.user_id, role, espace, None, "Dossier", t0()).await.unwrap();
    let j = ent.espaces_journal(etab, prof.user_id, role, espace, 50, t0()).await.unwrap();
    let actions: Vec<&str> = j.iter().map(|l| l.action_code.as_str()).collect();
    assert!(actions.contains(&"espace.cree") && actions.contains(&"membre.ajoute") && actions.contains(&"dossier.cree"), "{actions:?}");
    // Le journal ne contient ni nom de dossier ni texte libre : des codes.
    assert!(j.iter().all(|l| l.detail_code.as_deref().is_none_or(|d| d.len() <= 80)));
    // Direction : oui pour un espace collectif (droit `espaces.journal.lire`), non pour un espace personnel.
    assert!(ent.espaces_journal(etab, dir.user_id, role_d, espace, 10, t0()).await.is_ok());
    let perso = mon_espace(&ent, etab, prof.user_id, role).await;
    assert!(introuvable(ent.espaces_journal(etab, dir.user_id, role_d, perso, 10, t0()).await));
    assert!(ent.espaces_journal(etab, prof.user_id, role, perso, 10, t0()).await.is_ok(), "le propriétaire lit le journal de son espace");
    // Un lecteur simple : introuvable.
    assert!(introuvable(ent.espaces_journal(etab, cpe.user_id, role_c, espace, 10, t0()).await));
    // Ajout seul : ni modification ni suppression, même par le propriétaire de la base (déclencheur).
    let e = sqlx::query("UPDATE ent_espace_journal SET action_code = 'x' WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await;
    assert!(e.is_err());
    let e = sqlx::query("DELETE FROM ent_espace_journal WHERE etablissement_id = $1").bind(etab).execute(ent.pool_verif()).await;
    assert!(e.is_err());
}

/// §15 point 1, R-ET2 (cloisonnement, ND32-18) : un acteur d'un autre établissement ne lit, ne liste, ne télécharge rien par
/// identifiant ; un espace ne se lit pas non plus avec l'identifiant d'un élément d'un autre espace ; chaque ligne porte
/// l'établissement et les clés d'objet sont préfixées par lui.
#[tokio::test]
async fn et1_cloisonnement_entre_etablissements_et_entre_espaces() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof) = (cast.etablissement_id, &cast.prof);
    let role = prof.role.as_str();
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let (etab_b, prof_b) = (autre.etablissement_id, &autre.prof);
    let role_b = prof_b.role.as_str();
    collegue(&ent, etab, prof.user_id).await;
    collegue(&ent, etab_b, prof_b.user_id).await;
    let espace = libre(&ent, etab, prof.user_id, role, &uniq("Fermé"), vec![]).await;
    let octets = pdf_unique(800);
    let r = deposer_et_analyser(&ent, etab, prof.user_id, role, espace, demande("secret.pdf", octets.len()), &octets, t0()).await;
    let dossier = ent.espaces_dossier_creer(etab, prof.user_id, role, espace, None, "D", t0()).await.unwrap();
    // L'acteur de l'établissement B, avec SON établissement et les identifiants de A : introuvable partout.
    assert!(introuvable(ent.espaces_detail(etab_b, prof_b.user_id, role_b, espace, t0()).await));
    assert!(introuvable(ent.espaces_parcourir(etab_b, prof_b.user_id, role_b, espace, None, t0()).await));
    assert!(introuvable(ent.espaces_parcourir(etab_b, prof_b.user_id, role_b, espace, Some(dossier.id), t0()).await));
    assert!(introuvable(ent.espaces_lien_telechargement(etab_b, prof_b.user_id, role_b, r.element_id, None, t0()).await));
    assert!(introuvable(ent.espaces_versions(etab_b, prof_b.user_id, role_b, r.element_id, t0()).await));
    assert!(introuvable(ent.espaces_journal(etab_b, prof_b.user_id, role_b, espace, 10, t0()).await));
    assert!(introuvable(ent.espaces_element_corbeille(etab_b, prof_b.user_id, role_b, r.element_id, t0()).await));
    assert!(introuvable(ent.espaces_membre_retirer(etab_b, prof_b.user_id, role_b, espace, Uuid::new_v4(), t0()).await));
    // Un compte de A qui présenterait l'établissement B n'a pas de fiche dans B : refus.
    assert!(ent.espaces_lister(etab_b, prof.user_id, role, t0()).await.is_err());
    // Les listes de B ne contiennent rien de A.
    let liste_b = ent.espaces_lister(etab_b, prof_b.user_id, role_b, t0()).await.unwrap();
    assert!(liste_b.iter().all(|e| e.id != espace));
    // Toutes les lignes de A portent A ; les clés d'objet commencent par A.
    let hors: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_objet o JOIN ent_espace_version v ON v.etablissement_id = o.etablissement_id AND v.objet_id = o.id
          WHERE v.etablissement_id = $1 AND o.cle_objet NOT LIKE $2",
    )
    .bind(etab)
    .bind(format!("{etab}/espaces/%"))
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(hors, 0);
    // Un identifiant de dossier d'un autre espace du même établissement ne s'utilise pas dans un espace dont on est membre.
    let autre_espace = libre(&ent, etab, prof.user_id, role, &uniq("Autre"), vec![]).await;
    assert!(introuvable(ent.espaces_parcourir(etab, prof.user_id, role, autre_espace, Some(dossier.id), t0()).await));
    assert!(introuvable(ent.espaces_dossier_creer(etab, prof.user_id, role, autre_espace, Some(dossier.id), "x", t0()).await));
}

/// ND32-04, D-017 : le droit est vérifié dans **chaque** commande ; un compte sans `espaces.creer` ne crée pas d'espace ; un
/// compte sans `espaces.partager` ne partage pas ; un parent ou un élève n'a aucun accès aux espaces de personnels.
#[tokio::test]
async fn et1_droits_verifies_dans_chaque_commande_nd32_04() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let etab = cast.etablissement_id;
    let parent = &cast.parent_lea;
    let r = "parent";
    let n = NouvelEspace { libelle: "X".into(), description: None, couleur_code: None, icone_code: None, membres: vec![] };
    assert!(matches!(ent.espaces_creer_libre(etab, parent.user_id, r, n, t0()).await, Err(EntDbError::Forbidden)));
    let dp = DemandePartage { element_id: Some(Uuid::new_v4()), dossier_id: None, destinataires: vec![NouveauPartage { destinataire_personne_id: Uuid::new_v4(), droit: DroitPartage::Lecture }], fin: None };
    assert!(matches!(ent.espaces_partager(etab, parent.user_id, r, dp, t0()).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.espaces_parcourir(etab, parent.user_id, r, Uuid::new_v4(), None, t0()).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.espaces_lien_telechargement(etab, parent.user_id, r, Uuid::new_v4(), None, t0()).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.espaces_marquer_vu(etab, parent.user_id, r, None, None, t0()).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.espaces_quota(etab, parent.user_id, r, t0()).await, Err(EntDbError::Forbidden)));
    // Bornes : nom d'espace invalide, couleur ou icône hors catalogue, description trop longue, trop d'espaces.
    let prof = &cast.prof;
    collegue(&ent, etab, prof.user_id).await;
    let base = || NouvelEspace { libelle: uniq("Bornes"), description: None, couleur_code: None, icone_code: None, membres: vec![] };
    let mut m = base();
    m.libelle = "   ".into();
    assert!(ent.espaces_creer_libre(etab, prof.user_id, prof.role.as_str(), m, t0()).await.is_err());
    let mut m = base();
    m.couleur_code = Some("fluo".into());
    assert!(ent.espaces_creer_libre(etab, prof.user_id, prof.role.as_str(), m, t0()).await.is_err());
    let mut m = base();
    m.icone_code = Some("<script>".into());
    assert!(ent.espaces_creer_libre(etab, prof.user_id, prof.role.as_str(), m, t0()).await.is_err());
    let mut m = base();
    m.description = Some("x".repeat(501));
    assert!(ent.espaces_creer_libre(etab, prof.user_id, prof.role.as_str(), m, t0()).await.is_err());
    let mut ok = base();
    ok.couleur_code = Some("vert".into());
    ok.icone_code = Some("livre".into());
    let e = ent.espaces_creer_libre(etab, prof.user_id, prof.role.as_str(), ok, t0()).await.unwrap();
    assert_eq!((e.couleur_code.as_str(), e.icone_code.as_str()), ("vert", "livre"));
    // Renommage par l'animateur, refusé à un autre ; l'espace personnel ne se renomme pas.
    let m = ModifEspace { libelle: Some("Nouveau nom".into()), description: None, couleur_code: None, icone_code: None };
    let e2 = ent.espaces_modifier(etab, prof.user_id, prof.role.as_str(), e.id, m.clone(), t0()).await.unwrap();
    assert_eq!(e2.libelle, "Nouveau nom");
    let cpe = &cast.cpe;
    collegue(&ent, etab, cpe.user_id).await;
    assert!(introuvable(ent.espaces_modifier(etab, cpe.user_id, cpe.role.as_str(), e.id, m.clone(), t0()).await));
    let perso = mon_espace(&ent, etab, prof.user_id, prof.role.as_str()).await;
    assert!(ent.espaces_modifier(etab, prof.user_id, prof.role.as_str(), perso, m, t0()).await.is_err());
}

/// §24, D-036 : aucune colonne JSON, aucun tableau, aucun BYTEA dans les tables `ent_espace_*` ; chaque table d'établissement a
/// la clé d'établissement, la RLS activée et forcée ; les types fermés sont des contraintes (pas de texte libre).
#[tokio::test]
async fn et1_schema_sans_json_ni_tableau_ni_bytea_et_rls_posee() {
    let Some((ent, _cast)) = labo_espaces().await else { return };
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name::text FROM information_schema.tables
          WHERE table_schema = 'public' AND table_type = 'BASE TABLE' AND table_name LIKE 'ent\\_espace%' ORDER BY 1",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(tables.len() >= 14, "{tables:?}");
    let interdits: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text, data_type::text FROM information_schema.columns
          WHERE table_schema = 'public' AND table_name LIKE 'ent\\_espace%'
            AND (data_type IN ('json', 'jsonb', 'bytea', 'ARRAY') OR udt_name LIKE '\\_%')",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(interdits.is_empty(), "D-036 : {interdits:?}");
    for t in &tables {
        let (etab_col, rls, force): (bool, bool, bool) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = $1
                              AND column_name = 'etablissement_id' AND is_nullable = 'NO'),
                    c.relrowsecurity, c.relforcerowsecurity
               FROM pg_class c WHERE c.relname = $1 AND c.relnamespace = 'public'::regnamespace",
        )
        .bind(t)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert!(etab_col && rls && force, "{t} : etablissement_id NOT NULL = {etab_col}, RLS = {rls}, FORCE = {force}");
    }
}
