//! Messagerie, lot M1 (`docs/socles/16_MESSAGERIE.md` §27.1, R-M1, R-M2, R-M14) : tests contre la
//! base locale de test (feature `labo`). Chaque scénario sème son propre établissement de labo
//! (jamais de donnée réelle) ; sans `ENT_DATABASE_URL`, les scénarios sont ignorés (en CI, leur
//! absence fait échouer le test).
//!
//! Un test d'attaque commence par un **témoin** : la même action par un acteur légitime réussit,
//! sinon le test ne prouverait rien. Chaque défaut `NJ-xx` de `ALERTES_NEXTJS_PROD.md` fermé par M1
//! a son test, nommé `m1_nj_xx_…` (PILOTAGE, règle 14).

use super::*;
use domain_eleve::{AttacherPiece, DomainError, OuvrirFil, RepondreFil};
use ent_auth::Role;
use std::collections::BTreeSet;
use uuid::Uuid;

pub(crate) async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    ent.install_mail_dry_run();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

pub(crate) fn role(r: Role) -> &'static str {
    r.as_str()
}

// ─── Mise en scène ──────────────────────────────────────────────────────────────────────────

pub(crate) async fn nouvel_utilisateur(ent: &PgEnt, etab: Uuid, base: &str, nom: &str) -> Uuid {
    let user = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
         VALUES ($1,$2,'x',$3, now(), now())",
    )
    .bind(user)
    .bind(format!("{}-{}@ent.lab", nom.to_lowercase(), user.simple()))
    .bind(nom)
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_membership (user_id, etablissement_id, role, created_at)
         VALUES ($1,$2,$3, now())",
    )
    .bind(user)
    .bind(etab)
    .bind(base)
    .execute(ent.pool())
    .await
    .unwrap();
    user
}

pub(crate) async fn nouveau_foyer(ent: &PgEnt, etab: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_foyer (id, etablissement_id, libelle) VALUES ($1,$2,$3)")
        .bind(id)
        .bind(etab)
        .bind("Foyer de test")
        .execute(ent.pool())
        .await
        .unwrap();
    id
}

pub(crate) async fn lier(
    ent: &PgEnt,
    etab: Uuid,
    eleve: Uuid,
    user: Uuid,
    foyer: Option<Uuid>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, foyer_id, lien)
         VALUES ($1,$2,$3,$4,$5,'pere')",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve)
    .bind(user)
    .bind(foyer)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

pub(crate) async fn compte_eleve(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Uuid {
    let existant: Option<(Uuid,)> = sqlx::query_as(
        "SELECT user_id FROM ent_eleve_compte WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_optional(ent.pool())
    .await
    .unwrap();
    if let Some((u,)) = existant {
        return u;
    }
    let user = nouvel_utilisateur(ent, etab, "eleve", "Eleve").await;
    sqlx::query(
        "INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id) VALUES ($1,$2,$3)",
    )
    .bind(etab)
    .bind(eleve)
    .bind(user)
    .execute(ent.pool())
    .await
    .unwrap();
    user
}

pub(crate) async fn ouvrir(
    ent: &PgEnt,
    etab: Uuid,
    r: Role,
    user: Uuid,
    eleve: Uuid,
    sujet: &str,
) -> Vec<crate::messagerie_m0b::FilOuvert> {
    ent.ouvrir_fils_for_actor(
        role(r),
        OuvrirFil {
            etablissement_id: etab,
            eleve_id: eleve,
            sujet: sujet.into(),
            premier_message: format!("Contenu confidentiel — {sujet}"),
            actor_user_id: user,
        },
        OptionsEnvoiMessage::default(),
    )
    .await
    .expect("ouverture")
}

pub(crate) async fn ouvrir_un(
    ent: &PgEnt,
    etab: Uuid,
    r: Role,
    user: Uuid,
    eleve: Uuid,
    sujet: &str,
) -> crate::messagerie_m0b::FilOuvert {
    let mut v = ouvrir(ent, etab, r, user, eleve, sujet).await;
    assert_eq!(v.len(), 1, "un seul fil attendu pour « {sujet} »");
    v.remove(0)
}

pub(crate) async fn lit(ent: &PgEnt, etab: Uuid, user: Uuid, r: Role, fil: Uuid) -> bool {
    match ent.list_messages_for_actor(etab, user, role(r), fil).await {
        Ok(_) => true,
        Err(EntDbError::Forbidden) => false,
        Err(e) => panic!("lecture du fil {fil} : {e:?}"),
    }
}

pub(crate) async fn ecrit(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    r: Role,
    fil: Uuid,
    corps: &str,
) -> bool {
    match ent
        .repondre_fil_for_actor(
            role(r),
            RepondreFil {
                etablissement_id: etab,
                fil_id: fil,
                corps: corps.into(),
                actor_user_id: user,
            },
        )
        .await
    {
        Ok(_) => true,
        Err(EntDbError::Forbidden) => false,
        Err(e) => panic!("réponse au fil {fil} : {e:?}"),
    }
}

pub(crate) async fn compter(ent: &PgEnt, sql: &str, binds: &[Uuid]) -> i64 {
    let mut q = sqlx::query_as::<_, (i64,)>(sql);
    for b in binds {
        q = q.bind(*b);
    }
    q.fetch_one(ent.pool()).await.unwrap().0
}

/// Participants actifs d'un fil : `(côté, raison)` triés.
pub(crate) async fn participants(ent: &PgEnt, fil: Uuid) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = sqlx::query_as(
        "SELECT cote, raison FROM ent_msg_participant WHERE fil_id = $1 AND au IS NULL",
    )
    .bind(fil)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    v.sort();
    v
}

// ─── Création : les participants sont posés avec le fil (R-M1) ──────────────────────────────

#[tokio::test]
async fn m1_participants_poses_a_la_creation_et_a_la_premiere_ecriture() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    let (cpe, dir, prof) = (cast.cpe.user_id, cast.direction.user_id, cast.prof.user_id);
    let lily = cast.parent_lea.user_id;

    let ouvert = ouvrir_un(
        &ent,
        etab,
        Role::Direction,
        dir,
        harry,
        "Rendez-vous de Harry",
    )
    .await;
    let fil = ouvert.fil.id;
    // Un parent (titulaire du foyer) et l'ouvreur ; ni le professeur, ni la CPE.
    let p = participants(&ent, fil).await;
    assert!(
        p.contains(&("famille".into(), "titulaire_foyer".into())),
        "{p:?}"
    );
    assert!(
        p.contains(&("etablissement".into(), "auteur".into())),
        "{p:?}"
    );
    assert_eq!(p.len(), 2, "{p:?}");
    // La nature, l'origine et l'élève concerné sont posés.
    let (nature, origine, sens, etat): (String, String, String, String) =
        sqlx::query_as("SELECT nature, origine, sensibilite, etat FROM ent_msg_fil WHERE id = $1")
            .bind(fil)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(
        (
            nature.as_str(),
            origine.as_str(),
            sens.as_str(),
            etat.as_str()
        ),
        ("famille", "personnel", "C2", "ouvert")
    );
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_msg_fil_eleve WHERE fil_id = $1 AND eleve_id = $2",
            &[fil, harry]
        )
        .await,
        1
    );
    // L'empreinte du premier message est posée.
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_msg WHERE fil_id = $1 AND length(empreinte) = 64",
            &[fil]
        )
        .await,
        1
    );
    // Témoin : la CPE (rôle réservé) lit par le pont ; elle n'est pas encore participante.
    assert!(
        lit(&ent, etab, cpe, Role::Cpe, fil).await,
        "témoin : la CPE (pont)"
    );
    assert_eq!(participants(&ent, fil).await.len(), 2);
    // Dès qu'elle écrit, elle devient participante (raison codée « a_ecrit »).
    assert!(ecrit(&ent, etab, cpe, Role::Cpe, fil, "Je prends le dossier").await);
    let p = participants(&ent, fil).await;
    assert!(
        p.contains(&("etablissement".into(), "a_ecrit".into())),
        "{p:?}"
    );
    // Le professeur n'y participe pas et ne le lit pas (D21/D22 conservés).
    assert!(!lit(&ent, etab, prof, Role::Prof, fil).await);
    // Événements `messagerie.*` : sans sujet ni corps (R-M14).
    let charges: Vec<(String, String)> = sqlx::query_as(
        "SELECT type, payload::text FROM ent_metier_event
          WHERE etablissement_id = $1 AND aggregate_id = $2 AND type LIKE 'messagerie.%'",
    )
    .bind(etab)
    .bind(fil)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    let noms: BTreeSet<&str> = charges.iter().map(|c| c.0.as_str()).collect();
    assert!(noms.contains("messagerie.fil_ouvert"), "{noms:?}");
    assert!(noms.contains("messagerie.message_envoye"), "{noms:?}");
    assert!(noms.contains("messagerie.participant_ajoute"), "{noms:?}");
    for (_, payload) in &charges {
        assert!(
            !payload.contains("Rendez-vous") && !payload.contains("confidentiel"),
            "{payload}"
        );
    }
    let _ = lily;
}

// ─── R-M1 : lire = participer ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn m1_lire_c_est_participer_un_lien_ne_suffit_pas() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    let dir = cast.direction.user_id;
    let lily = cast.parent_lea.user_id;
    // James : second responsable, autre foyer (parents séparés).
    let james = nouvel_utilisateur(&ent, etab, "parent", "James").await;
    let foyer_james = nouveau_foyer(&ent, etab).await;
    lier(&ent, etab, harry, james, Some(foyer_james)).await;

    let fils = ouvrir(
        &ent,
        etab,
        Role::Direction,
        dir,
        harry,
        "Orientation de Harry",
    )
    .await;
    assert_eq!(fils.len(), 2, "un fil par foyer (R-M3)");
    // Fils jumeaux : le second pointe le premier.
    let (jumeaux,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_msg_fil WHERE id = ANY($1) AND jumeau_de = ANY($1)",
    )
    .bind(fils.iter().map(|f| f.fil.id).collect::<Vec<_>>())
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(jumeaux, 1, "le second fil est le jumeau du premier");

    let mut de = std::collections::HashMap::new();
    for f in &fils {
        let (foyer,): (Option<Uuid>,) =
            sqlx::query_as("SELECT foyer_lecteur_id FROM ent_msg_fil WHERE id = $1")
                .bind(f.fil.id)
                .fetch_one(ent.pool())
                .await
                .unwrap();
        de.insert(foyer.unwrap(), f.fil.id);
    }
    let (fil_lily, fil_james) = (de[&cast.foyer_lea_id], de[&foyer_james]);
    // Témoins : chacun lit son fil.
    assert!(
        lit(&ent, etab, lily, Role::Parent, fil_lily).await,
        "témoin : Lily"
    );
    assert!(
        lit(&ent, etab, james, Role::Parent, fil_james).await,
        "témoin : James"
    );
    // Attaque : le lien de James avec Harry ne lui donne pas le fil de l'autre foyer.
    assert!(!lit(&ent, etab, james, Role::Parent, fil_lily).await);
    assert!(!lit(&ent, etab, lily, Role::Parent, fil_james).await);
    assert!(!ecrit(&ent, etab, james, Role::Parent, fil_lily, "Je me permets").await);

    // Retirée du fil, Lily ne le lit plus alors que son lien est intact : lire = participer.
    sqlx::query(
        "UPDATE ent_msg_participant SET au = now(), motif_sortie = 'retire_par_direction'
          WHERE fil_id = $1 AND cote = 'famille'",
    )
    .bind(fil_lily)
    .execute(ent.pool())
    .await
    .unwrap();
    assert!(
        !lit(&ent, etab, lily, Role::Parent, fil_lily).await,
        "participation close"
    );
    // Suspendue (CM-16) : ni lecture ni écriture, sans effacement.
    sqlx::query(
        "UPDATE ent_msg_participant SET au = NULL, motif_sortie = NULL, suspendu_le = now()
          WHERE fil_id = $1 AND cote = 'famille'",
    )
    .bind(fil_lily)
    .execute(ent.pool())
    .await
    .unwrap();
    assert!(
        !lit(&ent, etab, lily, Role::Parent, fil_lily).await,
        "participation suspendue"
    );
    // Reprise de la suspension : la lecture revient.
    sqlx::query(
        "UPDATE ent_msg_participant SET suspendu_le = NULL WHERE fil_id = $1 AND cote = 'famille'",
    )
    .bind(fil_lily)
    .execute(ent.pool())
    .await
    .unwrap();
    assert!(
        lit(&ent, etab, lily, Role::Parent, fil_lily).await,
        "levée de la suspension"
    );
}

// ─── R-M2 : peut_ecrire_fil ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m1_fil_clos_ou_participation_sans_ecriture_refuse_l_ecriture() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    let dir = cast.direction.user_id;
    let lily = cast.parent_lea.user_id;
    let fil = ouvrir_un(&ent, etab, Role::Direction, dir, harry, "Absence de Harry")
        .await
        .fil
        .id;

    assert!(
        ecrit(&ent, etab, lily, Role::Parent, fil, "Merci, nous arrivons.").await,
        "témoin"
    );
    // Participation sans écriture (lecture seule) : lit, n'écrit pas.
    sqlx::query(
        "UPDATE ent_msg_participant SET peut_ecrire = false WHERE fil_id = $1 AND cote = 'famille'",
    )
    .bind(fil)
    .execute(ent.pool())
    .await
    .unwrap();
    assert!(
        lit(&ent, etab, lily, Role::Parent, fil).await,
        "elle lit encore"
    );
    assert!(!ecrit(&ent, etab, lily, Role::Parent, fil, "Encore un message").await);
    sqlx::query("UPDATE ent_msg_participant SET peut_ecrire = true WHERE fil_id = $1")
        .bind(fil)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(
        ecrit(&ent, etab, lily, Role::Parent, fil, "À nouveau").await,
        "témoin : écriture rendue"
    );
    // Fil clos ou gelé : plus personne n'écrit, la lecture reste.
    for etat in ["clos", "gele"] {
        sqlx::query("UPDATE ent_msg_fil SET etat = $2 WHERE id = $1")
            .bind(fil)
            .bind(etat)
            .execute(ent.pool())
            .await
            .unwrap();
        assert!(
            !ecrit(&ent, etab, lily, Role::Parent, fil, "Trop tard").await,
            "{etat} : parent"
        );
        assert!(
            !ecrit(&ent, etab, dir, Role::Direction, fil, "Trop tard").await,
            "{etat} : direction"
        );
        assert!(
            lit(&ent, etab, lily, Role::Parent, fil).await,
            "{etat} : lecture conservée"
        );
    }
}

// ─── Test d'or de la reprise (MIG-1 à MIG-4) ────────────────────────────────────────────────

/// Prédicat de lecture **famille** de M0b (avant M1), conservé ici comme oracle : `$1` établissement,
/// `$3` compte, `$4` vrai pour un parent. Ne sert qu'à prouver que la reprise ne change aucun lecteur.
const ORACLE_M0B_FAMILLE: &str = r#"
SELECT f.id FROM ent_msg_fil f
 WHERE f.etablissement_id = $1 AND f.id = ANY($2)
   AND (f.prive_user_id IS NULL OR f.prive_user_id = $3)
   AND (f.foyer_lecteur_id IS NULL OR NOT $4
        OR EXISTS (SELECT 1 FROM ent_eleve_lien_communicant l
                    WHERE l.etablissement_id = f.etablissement_id
                      AND l.eleve_id = f.eleve_id AND l.user_id = $3
                      AND l.foyer_id = f.foyer_lecteur_id))
   AND (f.foyer_id IS NULL
        OR EXISTS (SELECT 1 FROM ent_eleve_lien_communicant l
                    WHERE l.etablissement_id = f.etablissement_id
                      AND l.eleve_id = f.eleve_id AND l.user_id = $3
                      AND l.foyer_id = f.foyer_id AND l.est_payeur))
   AND CASE WHEN $4 THEN
            f.audience IN ('famille', 'responsables')
            AND EXISTS (SELECT 1 FROM ent_eleve_lien_communicant l
                         WHERE l.etablissement_id = f.etablissement_id
                           AND l.eleve_id = f.eleve_id AND l.user_id = $3
                           AND (NOT f.vie_scolaire
                                OR (l.peut_voir_vs AND l.autorite_parentale)))
        ELSE
            f.audience IN ('famille', 'eleve') AND f.foyer_id IS NULL
        END
"#;

async fn lisibles_avant(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    parent: bool,
    fils: &[Uuid],
) -> BTreeSet<Uuid> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(ORACLE_M0B_FAMILLE)
        .bind(etab)
        .bind(fils)
        .bind(user)
        .bind(parent)
        .fetch_all(ent.pool())
        .await
        .unwrap();
    rows.into_iter().map(|r| r.0).collect()
}

async fn lisibles_apres(ent: &PgEnt, etab: Uuid, user: Uuid, fils: &[Uuid]) -> BTreeSet<Uuid> {
    let rows: Vec<(Uuid,)> =
        sqlx::query_as("SELECT fil_id FROM ent_msg_fil_lisible($1, $3) WHERE fil_id = ANY($2)")
            .bind(etab)
            .bind(fils)
            .bind(user)
            .fetch_all(ent.pool())
            .await
            .unwrap();
    rows.into_iter().map(|r| r.0).collect()
}

/// Fil « hérité » : posé directement en base, sans participants (comme avant M1).
#[allow(clippy::too_many_arguments)]
async fn fil_herite(
    ent: &PgEnt,
    etab: Uuid,
    eleve: Uuid,
    createur: Uuid,
    sujet: &str,
    audience: &str,
    prive: Option<Uuid>,
    foyer: Option<Uuid>,
    foyer_lecteur: Option<Uuid>,
    vie_scolaire: bool,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_msg_fil (id, etablissement_id, eleve_id, sujet, created_by_user_id, audience,
                                  prive_user_id, foyer_id, foyer_lecteur_id, vie_scolaire)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve)
    .bind(sujet)
    .bind(createur)
    .bind(audience)
    .bind(prive)
    .bind(foyer)
    .bind(foyer_lecteur)
    .bind(vie_scolaire)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

async fn message_herite(
    ent: &PgEnt,
    etab: Uuid,
    fil: Uuid,
    auteur: Uuid,
    corps: &str,
    il_y_a_min: i32,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_msg (id, etablissement_id, fil_id, auteur_user_id, corps, created_at)
         VALUES ($1,$2,$3,$4,$5, now() - make_interval(mins => $6))",
    )
    .bind(id)
    .bind(etab)
    .bind(fil)
    .bind(auteur)
    .bind(corps)
    .bind(il_y_a_min)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

async fn accuse_herite(
    ent: &PgEnt,
    etab: Uuid,
    fil: Uuid,
    msg: Uuid,
    lecteur: Uuid,
    il_y_a_min: i32,
) {
    sqlx::query(
        "INSERT INTO ent_msg_accuse (id, etablissement_id, fil_id, message_id, lecteur_user_id, lu_at)
         VALUES ($1,$2,$3,$4,$5, now() - make_interval(mins => $6))",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(fil)
    .bind(msg)
    .bind(lecteur)
    .bind(il_y_a_min)
    .execute(ent.pool())
    .await
    .unwrap();
}

pub(crate) async fn corps_du_fil(ent: &PgEnt, fil: Uuid) -> Vec<String> {
    let v: Vec<(String,)> =
        sqlx::query_as("SELECT corps FROM ent_msg WHERE fil_id = $1 ORDER BY created_at, id")
            .bind(fil)
            .fetch_all(ent.pool())
            .await
            .unwrap();
    v.into_iter().map(|x| x.0).collect()
}

#[tokio::test]
async fn m1_test_d_or_de_la_reprise_aucun_lecteur_ne_gagne_ni_ne_perd_hors_mig_3() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    let (dir, cpe) = (cast.direction.user_id, cast.cpe.user_id);
    let lily = cast.parent_lea.user_id;
    let james = nouvel_utilisateur(&ent, etab, "parent", "James").await;
    let foyer_james = nouveau_foyer(&ent, etab).await;
    lier(&ent, etab, harry, james, Some(foyer_james)).await;
    let chloe = compte_eleve(&ent, etab, harry).await;
    let foyer_lily = cast.foyer_lea_id;

    // A — MIG-1 : fil ouvert par Lily, la direction y répond.
    let a = fil_herite(
        &ent,
        etab,
        harry,
        lily,
        "A privé",
        "responsables",
        Some(lily),
        None,
        None,
        false,
    )
    .await;
    message_herite(&ent, etab, a, lily, "A1 Lily", 50).await;
    message_herite(&ent, etab, a, dir, "A2 direction", 40).await;
    // B — MIG-2 : relance de facture du foyer de Lily.
    let b = fil_herite(
        &ent,
        etab,
        harry,
        dir,
        "B relance",
        "responsables",
        None,
        Some(foyer_lily),
        None,
        false,
    )
    .await;
    message_herite(&ent, etab, b, dir, "B1 relance", 50).await;
    // C — MIG-4 : fil ouvert par l'élève.
    let c = fil_herite(
        &ent,
        etab,
        harry,
        chloe,
        "C élève",
        "eleve",
        Some(chloe),
        None,
        None,
        false,
    )
    .await;
    message_herite(&ent, etab, c, chloe, "C1 élève", 50).await;
    message_herite(&ent, etab, c, cpe, "C2 CPE", 40).await;
    // D — fil du personnel à un foyer (M0b D22).
    let d = fil_herite(
        &ent,
        etab,
        harry,
        dir,
        "D foyer",
        "responsables",
        None,
        None,
        Some(foyer_lily),
        false,
    )
    .await;
    message_herite(&ent, etab, d, dir, "D1 direction", 50).await;
    // G — vie scolaire (alerte d'absences).
    let g = fil_herite(
        &ent,
        etab,
        harry,
        cpe,
        "G vie scolaire",
        "responsables",
        None,
        None,
        None,
        true,
    )
    .await;
    message_herite(&ent, etab, g, cpe, "G1 alerte", 50).await;
    // E — MIG-3, deux foyers ont lu et répondu.
    let e = fil_herite(
        &ent,
        etab,
        harry,
        dir,
        "E deux foyers",
        "responsables",
        None,
        None,
        None,
        false,
    )
    .await;
    let e1 = message_herite(&ent, etab, e, dir, "E1 direction", 90).await;
    accuse_herite(&ent, etab, e, e1, lily, 80).await;
    accuse_herite(&ent, etab, e, e1, james, 70).await;
    message_herite(&ent, etab, e, lily, "E2 réponse de Lily", 60).await;
    message_herite(&ent, etab, e, james, "E3 réponse de James", 50).await;
    // F — MIG-3, personne n'a lu.
    let f = fil_herite(
        &ent,
        etab,
        harry,
        dir,
        "F jamais lu",
        "responsables",
        None,
        None,
        None,
        false,
    )
    .await;
    message_herite(&ent, etab, f, dir, "F1 direction", 90).await;
    // H — MIG-3, un seul foyer a lu.
    let h = fil_herite(
        &ent,
        etab,
        harry,
        dir,
        "H un lecteur",
        "responsables",
        None,
        None,
        None,
        false,
    )
    .await;
    let h1 = message_herite(&ent, etab, h, dir, "H1 direction", 90).await;
    accuse_herite(&ent, etab, h, h1, lily, 80).await;

    let tous = [a, b, c, d, e, f, g, h];
    let comptes = [(lily, true), (james, true), (chloe, false)];
    let mut avant = std::collections::HashMap::new();
    for (u, parent) in comptes {
        avant.insert(u, lisibles_avant(&ent, etab, u, parent, &tous).await);
    }
    // Témoins : l'oracle voit bien des lecteurs (sinon la comparaison ne prouverait rien).
    assert!(avant[&lily].contains(&a) && avant[&chloe].contains(&c) && avant[&james].contains(&e));
    assert!(!avant[&james].contains(&a) && !avant[&lily].contains(&c));

    let n: (i32,) = sqlx::query_as("SELECT ent_msg_m1_reprise()")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(n.0 >= 8, "au moins nos huit fils sont repris");
    // Idempotence : un second passage ne reprend rien et ne duplique rien.
    let nb_fils: i64 = compter(
        &ent,
        "SELECT count(*) FROM ent_msg_fil WHERE etablissement_id = $1",
        &[etab],
    )
    .await;
    let n2: (i32,) = sqlx::query_as("SELECT ent_msg_m1_reprise()")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n2.0, 0);
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_msg_fil WHERE etablissement_id = $1",
            &[etab]
        )
        .await,
        nb_fils
    );

    // Fils jumeaux créés par le dédoublement (E et F).
    let jumeaux: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, jumeau_de FROM ent_msg_fil WHERE etablissement_id = $1 AND jumeau_de IS NOT NULL",
    )
    .bind(etab)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    let jumeau_de = |origine: Uuid| jumeaux.iter().find(|j| j.1 == origine).map(|j| j.0);
    let (e_bis, f_bis) = (
        jumeau_de(e).expect("E dédoublé"),
        jumeau_de(f).expect("F : un fil par foyer"),
    );
    assert!(
        jumeau_de(h).is_none(),
        "un seul foyer a lu : pas de dédoublement"
    );

    // 1. Hors MIG-3 (A, B, C, D, G) : exactement les mêmes lecteurs, ni plus ni moins.
    for (u, _) in comptes {
        let apres = lisibles_apres(&ent, etab, u, &tous).await;
        for fil in [a, b, c, d, g] {
            assert_eq!(
                avant[&u].contains(&fil),
                apres.contains(&fil),
                "le fil {fil} change de lecteur pour {u} hors MIG-3"
            );
        }
    }
    // 2. MIG-3 : chaque responsable qui lisait E (resp. F) lit **un seul** des deux fils jumeaux.
    for fil in [(e, e_bis), (f, f_bis)] {
        for u in [lily, james] {
            let apres = lisibles_apres(&ent, etab, u, &[fil.0, fil.1]).await;
            assert_eq!(
                apres.len(),
                1,
                "{u} lit exactement un des deux fils jumeaux {fil:?}"
            );
        }
        let l = lisibles_apres(&ent, etab, lily, &[fil.0, fil.1]).await;
        let j = lisibles_apres(&ent, etab, james, &[fil.0, fil.1]).await;
        assert!(
            l.is_disjoint(&j),
            "jamais deux foyers dans un même fil (R-M3)"
        );
    }
    // Le premier lecteur (Lily) garde le fil d'origine de E ; James passe au jumeau.
    assert!(lisibles_apres(&ent, etab, lily, &[e]).await.contains(&e));
    assert!(lisibles_apres(&ent, etab, james, &[e_bis])
        .await
        .contains(&e_bis));
    // H : le seul foyer qui a lu garde le fil ; l'autre le perd (MIG-3 documenté).
    assert!(lisibles_apres(&ent, etab, lily, &[h]).await.contains(&h));
    assert!(!lisibles_apres(&ent, etab, james, &[h]).await.contains(&h));
    // Aucun lecteur n'a gagné un fil hérité qu'il ne lisait pas (hors jumeaux de MIG-3).
    for (u, _) in comptes {
        let apres = lisibles_apres(&ent, etab, u, &tous).await;
        for fil in &apres {
            assert!(avant[&u].contains(fil), "{u} gagne le fil {fil}");
        }
    }

    // 3. Les messages : le personnel est copié (même texte, `copie_de`), la réponse de chaque foyer
    //    suit son foyer, aucune réponse d'un foyer n'est lisible de l'autre (R-M4).
    let c_e = corps_du_fil(&ent, e).await;
    let c_e_bis = corps_du_fil(&ent, e_bis).await;
    assert_eq!(c_e, vec!["E1 direction", "E2 réponse de Lily"]);
    assert_eq!(c_e_bis, vec!["E1 direction", "E3 réponse de James"]);
    assert_eq!(corps_du_fil(&ent, f_bis).await, vec!["F1 direction"]);
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_msg WHERE fil_id = $1 AND copie_de IS NOT NULL",
            &[e_bis]
        )
        .await,
        1
    );
    // L'accusé de lecture de James sur le message du personnel a suivi la copie ; celui de Lily reste.
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_msg_accuse a JOIN ent_msg m ON m.id = a.message_id WHERE m.fil_id = $1 AND a.lecteur_user_id = $2",
            &[e_bis, james]
        )
        .await,
        1
    );
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_msg_accuse WHERE fil_id = $1 AND lecteur_user_id = $2",
            &[e, james]
        )
        .await,
        0
    );
    // 4. Le rapport à la direction trace chaque dédoublement (sans contenu).
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_msg_reprise_rapport WHERE etablissement_id = $1 AND regle = 'MIG-3' AND fil_jumeau_id IS NOT NULL",
            &[etab]
        )
        .await,
        2
    );
    // Les participants du fil de l'élève : lui, la CPE, aucun parent (MIG-4).
    let p = participants(&ent, c).await;
    assert!(
        p.iter().all(|(cote, _)| cote != "famille"),
        "MIG-4 : aucun parent {p:?}"
    );
    assert!(p.iter().any(|(cote, _)| cote == "eleve"));
    // Fils de vie scolaire à deux foyers : signalés, non dédoublés (MIG-5 / M7).
    let (vs,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_msg_reprise_rapport WHERE fil_id = $1 AND effet = 'foyers_dans_un_fil'",
    )
    .bind(g)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        vs, 1,
        "deux foyers dans un même fil de vie scolaire sont signalés"
    );
}

// ─── Défauts Next.js fermés (NJ-xx, PILOTAGE règle 14) ──────────────────────────────────────

/// NJ-37 : en Next.js, la règle « un professeur limité à ses classes » se contourne avec un
/// `foyerId` ou un `threadId` quelconque. Ici, un fil se lit et s'écrit par la participation et le
/// périmètre, jamais sur l'identifiant fourni.
#[tokio::test]
async fn m1_nj_37_un_professeur_ne_lit_ni_n_ecrit_un_fil_hors_de_ses_eleves_en_changeant_d_identifiant(
) {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let (prof, dir) = (cast.prof.user_id, cast.direction.user_id);
    let (lily, drago_parent) = (cast.parent_lea.user_id, cast.parent_tom.user_id);

    // Témoin : le professeur (classe de Harry) ouvre et lit son propre fil.
    let propre = ouvrir_un(&ent, etab, Role::Prof, prof, harry, "Devoir de Harry")
        .await
        .fil
        .id;
    assert!(
        lit(&ent, etab, prof, Role::Prof, propre).await,
        "témoin : son fil"
    );
    // Fils d'élèves hors de son périmètre (Draco) et fil privé d'un parent de son élève.
    let draco_fil = ouvrir_un(&ent, etab, Role::Direction, dir, draco, "Dossier de Draco")
        .await
        .fil
        .id;
    let prive_lily = ouvrir_un(&ent, etab, Role::Parent, lily, harry, "Message de Lily")
        .await
        .fil
        .id;
    let prive_drago = ouvrir_un(
        &ent,
        etab,
        Role::Parent,
        drago_parent,
        draco,
        "Message du père de Draco",
    )
    .await
    .fil
    .id;
    for fil in [draco_fil, prive_lily, prive_drago] {
        assert!(
            !lit(&ent, etab, prof, Role::Prof, fil).await,
            "NJ-37 : lecture {fil}"
        );
        assert!(
            !ecrit(&ent, etab, prof, Role::Prof, fil, "Je me permets").await,
            "NJ-37 : écriture {fil}"
        );
    }
    // Sa boîte ne rend que les fils où il participe : jamais « les 100 derniers fils de l'établissement ».
    let liste = ent
        .list_fils_for_actor(etab, prof, role(Role::Prof), None, true, None)
        .await
        .expect("boîte");
    let ids: BTreeSet<Uuid> = liste.iter().map(|f| f.id).collect();
    assert!(ids.contains(&propre), "témoin : son fil est dans sa boîte");
    for fil in [draco_fil, prive_lily, prive_drago] {
        assert!(
            !ids.contains(&fil),
            "NJ-37 : le fil {fil} fuit dans la boîte du professeur"
        );
    }
    // Un identifiant inventé ne donne rien (ni lecture, ni écriture).
    let faux = Uuid::new_v4();
    assert!(ent
        .list_messages_for_actor(etab, prof, role(Role::Prof), faux)
        .await
        .is_err());
}

/// NJ-36 et NJ-29 : en Next.js, la route de pièce jointe ne vérifie pas que le fil appartient au
/// foyer du parent, et la clé de la pièce est fournie par le navigateur. Ici, la pièce suit le
/// contrôle du fil (le client ne fournit ni clé ni chemin) et l'accès d'un autre foyer est refusé.
#[tokio::test]
async fn m1_nj_36_nj_29_la_piece_d_un_fil_non_lisible_est_refusee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    let dir = cast.direction.user_id;
    let lily = cast.parent_lea.user_id;
    let james = nouvel_utilisateur(&ent, etab, "parent", "James").await;
    let foyer_james = nouveau_foyer(&ent, etab).await;
    lier(&ent, etab, harry, james, Some(foyer_james)).await;

    let fils = ouvrir(
        &ent,
        etab,
        Role::Direction,
        dir,
        harry,
        "Pièce confidentielle",
    )
    .await;
    let mut par_foyer = std::collections::HashMap::new();
    for f in &fils {
        let (foyer,): (Option<Uuid>,) =
            sqlx::query_as("SELECT foyer_lecteur_id FROM ent_msg_fil WHERE id = $1")
                .bind(f.fil.id)
                .fetch_one(ent.pool())
                .await
                .unwrap();
        par_foyer.insert(foyer.unwrap(), f);
    }
    let fil_lily = par_foyer[&cast.foyer_lea_id];
    let meta = ent
        .attacher_piece_for_actor(
            role(Role::Direction),
            AttacherPiece {
                etablissement_id: etab,
                message_id: fil_lily.message.id,
                nom: "ordonnance.pdf".into(),
                mime_type: "application/pdf".into(),
                contenu: b"%PDF-1.4 secret".to_vec(),
                actor_user_id: dir,
            },
        )
        .await
        .expect("pièce jointe");
    // Témoin : Lily (participante) télécharge sa pièce.
    assert!(
        ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), meta.id)
            .await
            .is_ok(),
        "témoin : Lily"
    );
    // Attaque : James (autre foyer, même élève) ne télécharge ni ne demande de lien court.
    assert!(
        matches!(
            ent.telecharger_piece_for_actor(etab, james, role(Role::Parent), meta.id)
                .await,
            Err(EntDbError::Forbidden)
        ),
        "NJ-36 : la pièce d'un autre foyer"
    );
    // Il ne peut pas non plus y attacher une pièce (la clé n'est jamais fournie par le client).
    let r = ent
        .attacher_piece_for_actor(
            role(Role::Parent),
            AttacherPiece {
                etablissement_id: etab,
                message_id: fil_lily.message.id,
                nom: "x.pdf".into(),
                mime_type: "application/pdf".into(),
                contenu: b"%PDF-1.4".to_vec(),
                actor_user_id: james,
            },
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::Forbidden)),
        "NJ-29 : attachement par un non-participant"
    );
    // La clé d'une pièce est dérivée du serveur : `{établissement}/messages/{pièce}.…`, jamais fournie.
    assert!(crate::service::msg_storage_key_is_owned(
        &piece_factice(meta.id, etab, fil_lily.fil.id, fil_lily.message.id),
        &format!("{etab}/messages/{}.pdf", meta.id)
    ));
    assert!(!crate::service::msg_storage_key_is_owned(
        &piece_factice(meta.id, etab, fil_lily.fil.id, fil_lily.message.id),
        &format!("{etab}/messages-autre/{}.pdf", meta.id)
    ));
}

/// NJ-59 : en Next.js, la présence de tout membre est lisible de tous et le « vu par » expose
/// l'e-mail. Ici, un participant ne voit que les fils où il participe ; ni la liste des participants
/// d'un autre foyer, ni leur existence, n'est exposée (aucune adresse composée, R-M4).
#[tokio::test]
async fn m1_nj_59_un_parent_ne_voit_pas_les_participants_ni_l_existence_d_un_autre_fil() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    let dir = cast.direction.user_id;
    let lily = cast.parent_lea.user_id;
    let james = nouvel_utilisateur(&ent, etab, "parent", "James").await;
    let foyer_james = nouveau_foyer(&ent, etab).await;
    lier(&ent, etab, harry, james, Some(foyer_james)).await;
    let fils = ouvrir(
        &ent,
        etab,
        Role::Direction,
        dir,
        harry,
        "Sujet réservé au foyer",
    )
    .await;
    let mut fil_james = None;
    for f in &fils {
        let (foyer,): (Option<Uuid>,) =
            sqlx::query_as("SELECT foyer_lecteur_id FROM ent_msg_fil WHERE id = $1")
                .bind(f.fil.id)
                .fetch_one(ent.pool())
                .await
                .unwrap();
        if foyer == Some(foyer_james) {
            fil_james = Some(f.fil.id);
        }
    }
    let fil_james = fil_james.unwrap();
    // Témoin : James voit son fil dans sa liste.
    let liste_james = ent
        .list_fils_for_actor(etab, james, role(Role::Parent), None, true, None)
        .await
        .unwrap();
    assert!(liste_james.iter().any(|f| f.id == fil_james), "témoin");
    // Attaque : Lily ne voit ni le fil, ni lors d'une recherche de ses fils.
    let liste_lily = ent
        .list_fils_for_actor(etab, lily, role(Role::Parent), None, true, None)
        .await
        .unwrap();
    assert!(
        !liste_lily.iter().any(|f| f.id == fil_james),
        "NJ-59 : existence du fil d'un autre foyer"
    );
    assert!(!lit(&ent, etab, lily, Role::Parent, fil_james).await);
    // Le « vu par » ne rend que des comptes de leur propre fil, sans e-mail : les accusés ne portent
    // que des identifiants (aucune coordonnée n'est composée dans un fil).
    let (_, msgs) = ent
        .list_messages_for_actor(etab, james, role(Role::Parent), fil_james)
        .await
        .unwrap();
    let rendu = format!("{msgs:?}");
    assert!(
        !rendu.contains("@ent.lab"),
        "aucun e-mail dans la lecture d'un fil : {rendu}"
    );
    let _ = DomainError::NotFound;
}

fn piece_factice(id: Uuid, etab: Uuid, fil: Uuid, message: Uuid) -> domain_eleve::MsgPiece {
    domain_eleve::MsgPiece {
        id,
        etablissement_id: etab,
        fil_id: fil,
        message_id: message,
        nom: "o.pdf".into(),
        mime_type: "application/pdf".into(),
        taille_octets: 0,
        contenu: Vec::new(),
        storage_key: None,
        sha256: None,
        created_by_user_id: Uuid::nil(),
        created_at: chrono::Utc::now(),
    }
}
