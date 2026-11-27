//! Transverse « la deuxième personne fondée sur la personne physique », lot 1 (migration 279) : l'individu.
//! Contre la base locale de test (feature `labo`). Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! Un établissement neuf par test. Deux fiches reliées sont le même individu (dans les deux sens, par chaîne, par fusion),
//! seulement dans leur établissement ; la levée retire le lien, jamais par l'individu concerné ; la déclaration se fait
//! depuis un compte et se confirme depuis l'autre ; les tables sont en ajout seul ; les propositions ne relient rien.

use super::*;
use crate::individu::{comptes_de, meme_individu, personnes_du_compte, MSG_DEJA_RELIEES, MSG_LEVEE_PAR_SOI};
use crate::service::RefusMetier;
use domain_eleve::DomainError;
use uuid::Uuid;

const MDP: &str = "MotDePasse-Ind1-Aa9!";

pub(crate) async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

/// Un compte de l'établissement (rôle donné), son adresse.
pub(crate) async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> (Uuid, String) {
    let email = format!("ind-{role}-{}@ent.local", Uuid::new_v4().simple());
    let id = ent
        .register_user(&email, MDP, Some(&format!("Nom {role}")), etab, role)
        .await
        .expect("compte")
        .id;
    (id, email)
}

/// La fiche du compte (créée si le compte n'en a pas : compte parent sans lien).
pub(crate) async fn fiche_du_compte(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let existante: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2")
            .bind(etab)
            .bind(user)
            .fetch_optional(&mut *tx)
            .await
            .unwrap();
    let id = match existante {
        Some(id) => id,
        None => {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom, user_id) VALUES ($1, $2, 'Parent', 'Compte', $3)")
                .bind(id)
                .bind(etab)
                .bind(user)
                .execute(&mut *tx)
                .await
                .unwrap();
            id
        }
    };
    tx.commit().await.unwrap();
    id
}

/// Une fiche sans compte (parent saisi par le secrétariat).
pub(crate) async fn fiche_sans_compte(ent: &PgEnt, etab: Uuid, nom: &str, telephone: Option<&str>) -> Uuid {
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom, telephone) VALUES ($1, $2, $3, 'Sans compte', $4)")
        .bind(id)
        .bind(etab)
        .bind(nom)
        .bind(telephone)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    id
}

async fn meme(ent: &PgEnt, etab: Uuid, a: Uuid, b: Uuid) -> bool {
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    meme_individu(&mut tx, etab, a, b).await.unwrap()
}

async fn fiches(ent: &PgEnt, etab: Uuid, u: Uuid) -> Vec<Uuid> {
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    personnes_du_compte(&mut tx, etab, u).await.unwrap()
}

fn est_refus(e: &EntDbError, nature: RefusMetier) -> bool {
    matches!(e, EntDbError::Refus(n, _) if *n == nature)
}

/// Relier, dans les deux sens, par chaîne, lever : le lien agit tout de suite et cesse à la levée.
#[tokio::test]
async fn individu_relier_chaine_et_levee() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let (dir, _) = compte(&ent, etab, "direction").await;
    let (sec, _) = compte(&ent, etab, "administratif").await;
    let (prof, _) = compte(&ent, etab, "prof").await;
    let (parent, _) = compte(&ent, etab, "parent").await;
    let p_prof = fiche_du_compte(&ent, etab, prof).await;
    let p_parent = fiche_du_compte(&ent, etab, parent).await;
    let p_sans = fiche_sans_compte(&ent, etab, "Martin", None).await;

    assert!(meme(&ent, etab, prof, prof).await, "un compte est lui-même");
    assert!(!meme(&ent, etab, prof, parent).await, "deux comptes non reliés : deux individus");

    // Un professeur ne relie pas ; le secrétariat relie.
    let e = ent
        .individu_relier_for_actor(etab, prof, "prof", p_prof, p_parent, "deux_comptes", false)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    let e = ent
        .individu_relier_for_actor(etab, sec, "administratif", p_prof, p_parent, "inconnu", false)
        .await
        .unwrap_err();
    assert!(est_refus(&e, RefusMetier::Invalide), "motif hors liste : {e:?}");
    let l1 = ent
        .individu_relier_for_actor(etab, sec, "administratif", p_parent, p_prof, "deux_comptes", false)
        .await
        .unwrap();
    assert!(meme(&ent, etab, prof, parent).await && meme(&ent, etab, parent, prof).await, "dans les deux sens");
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let c = comptes_de(&mut tx, etab, prof).await.unwrap();
    drop(tx);
    assert!(c.contains(&prof) && c.contains(&parent) && c.len() == 2, "{c:?}");

    // Une seconde fois, dans un sens ou dans l'autre : déjà reliées.
    let e = ent
        .individu_relier_for_actor(etab, dir, "direction", p_prof, p_parent, "autre", false)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Refus(RefusMetier::Conflit, m) if m == MSG_DEJA_RELIEES), "{e:?}");

    // Chaîne : la fiche sans compte reliée au parent est aussi le professeur.
    let l2 = ent
        .individu_relier_for_actor(etab, dir, "direction", p_parent, p_sans, "fiche_sans_compte", false)
        .await
        .unwrap();
    assert!(fiches(&ent, etab, prof).await.contains(&p_sans), "chaîne prof – parent – fiche sans compte");
    // Fermer la boucle ne dit rien de plus : refusé.
    let e = ent
        .individu_relier_for_actor(etab, dir, "direction", p_prof, p_sans, "autre", false)
        .await
        .unwrap_err();
    assert!(est_refus(&e, RefusMetier::Conflit), "{e:?}");

    let liens = ent.individu_liens_for_actor(etab, sec, "administratif").await.unwrap();
    assert_eq!(liens.len(), 2);
    assert!(ent.individu_liens_for_actor(etab, prof, "prof").await.is_err());

    // Le secrétariat ne lève pas ; la direction lève, avec un motif codé ; une seule fois.
    assert!(matches!(
        ent.individu_lever_for_actor(etab, sec, "administratif", l1, "erreur").await.unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(est_refus(
        &ent.individu_lever_for_actor(etab, dir, "direction", l1, "texte libre").await.unwrap_err(),
        RefusMetier::Invalide
    ));
    ent.individu_lever_for_actor(etab, dir, "direction", l1, "erreur").await.unwrap();
    assert!(est_refus(
        &ent.individu_lever_for_actor(etab, dir, "direction", l1, "erreur").await.unwrap_err(),
        RefusMetier::Conflit
    ));
    assert!(!meme(&ent, etab, prof, parent).await, "levé : deux individus");
    assert!(fiches(&ent, etab, parent).await.contains(&p_sans), "l'autre lien reste");
    assert!(!fiches(&ent, etab, prof).await.contains(&p_sans));
    let _ = l2;

    // Après la levée, la paire peut être reliée de nouveau.
    ent.individu_relier_for_actor(etab, dir, "direction", p_prof, p_parent, "deux_comptes", false)
        .await
        .unwrap();
    assert!(meme(&ent, etab, prof, parent).await);
}

/// Lever retire des refus : jamais par l'individu concerné, même s'il est de la direction.
#[tokio::test]
async fn individu_la_direction_ne_leve_pas_son_propre_lien() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let (dir, _) = compte(&ent, etab, "direction").await;
    let (dir2, _) = compte(&ent, etab, "direction").await;
    let (parent, _) = compte(&ent, etab, "parent").await;
    let p_dir = fiche_du_compte(&ent, etab, dir).await;
    let p_parent = fiche_du_compte(&ent, etab, parent).await;
    let l = ent
        .individu_relier_for_actor(etab, dir, "direction", p_dir, p_parent, "deux_comptes", false)
        .await
        .unwrap();
    let e = ent.individu_lever_for_actor(etab, dir, "direction", l, "erreur").await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(RefusMetier::Interdit, m) if m == MSG_LEVEE_PAR_SOI), "{e:?}");
    assert!(meme(&ent, etab, dir, parent).await, "le refus ne lève rien");
    ent.individu_lever_for_actor(etab, dir2, "direction", l, "homonyme").await.unwrap();
    assert!(!meme(&ent, etab, dir, parent).await);
}

/// Déclaration depuis un compte, confirmée depuis l'autre ; refus ; adresse inconnue sans trace ; expiration ; quota.
#[tokio::test]
async fn individu_declaration_en_deux_temps() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let autre_etab = Uuid::new_v4();
    let (prof, _) = compte(&ent, etab, "prof").await;
    let (parent, email_parent) = compte(&ent, etab, "parent").await;
    let (_, email_ailleurs) = compte(&ent, autre_etab, "parent").await;
    let p_prof = fiche_du_compte(&ent, etab, prof).await;
    let p_parent = fiche_du_compte(&ent, etab, parent).await;

    // Adresse inconnue ou d'un autre établissement : même réponse, rien n'est écrit.
    ent.individu_declarer_for_actor(etab, prof, "personne@nulle-part.local").await.unwrap();
    ent.individu_declarer_for_actor(etab, prof, &email_ailleurs).await.unwrap();
    assert!(est_refus(
        &ent.individu_declarer_for_actor(etab, prof, "pas une adresse").await.unwrap_err(),
        RefusMetier::Invalide
    ));
    let n: i64 = {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        sqlx::query_scalar("SELECT count(*) FROM ent_individu_declaration WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .unwrap()
    };
    assert_eq!(n, 0);

    // Déclaration (deux fois : une seule en attente), vue du seul compte cible.
    ent.individu_declarer_for_actor(etab, prof, &email_parent.to_uppercase()).await.unwrap();
    ent.individu_declarer_for_actor(etab, prof, &email_parent).await.unwrap();
    let moi = ent.individu_moi_for_actor(etab, parent).await.unwrap();
    assert_eq!(moi.declarations_recues.len(), 1);
    assert_eq!(moi.declarations_recues[0].declarant, "Nom prof");
    assert!(ent.individu_moi_for_actor(etab, prof).await.unwrap().declarations_recues.is_empty());
    let d = moi.declarations_recues[0].id;

    // Le déclarant ne confirme pas lui-même : la déclaration n'existe pas pour lui.
    let e = ent.individu_repondre_for_actor(etab, prof, d, true).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    assert!(!meme(&ent, etab, prof, parent).await);

    // Le compte cible confirme : les deux fiches sont reliées.
    let lien = ent.individu_repondre_for_actor(etab, parent, d, true).await.unwrap();
    assert!(lien.is_some());
    assert!(meme(&ent, etab, prof, parent).await);
    assert!(fiches(&ent, etab, prof).await.contains(&p_parent));
    assert!(est_refus(&ent.individu_repondre_for_actor(etab, parent, d, true).await.unwrap_err(), RefusMetier::Conflit));
    let moi = ent.individu_moi_for_actor(etab, prof).await.unwrap();
    assert_eq!((moi.autres_comptes, moi.fiches_reliees), (1, 2));
    let _ = p_prof;

    // Refus : rien n'est relié.
    let (prof2, email_prof2) = compte(&ent, etab, "prof").await;
    let (cpe, _) = compte(&ent, etab, "cpe").await;
    ent.individu_declarer_for_actor(etab, cpe, &email_prof2).await.unwrap();
    let d2 = ent.individu_moi_for_actor(etab, prof2).await.unwrap().declarations_recues[0].id;
    assert_eq!(ent.individu_repondre_for_actor(etab, prof2, d2, false).await.unwrap(), None);
    assert!(!meme(&ent, etab, cpe, prof2).await);

    // Expiration : une déclaration de plus de 24 h ne se confirme plus (vieillie par la purge volontaire seulement).
    ent.individu_declarer_for_actor(etab, cpe, &email_prof2).await.unwrap();
    let d3 = ent.individu_moi_for_actor(etab, prof2).await.unwrap().declarations_recues[0].id;
    {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        sqlx::query("SET LOCAL ent.purge_etablissement = 'on'").execute(&mut *tx).await.unwrap();
        sqlx::query("UPDATE ent_individu_declaration SET declare_le = now() - interval '2 days', expire_le = now() - interval '1 day' WHERE id = $1")
            .bind(d3)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    assert!(ent.individu_moi_for_actor(etab, prof2).await.unwrap().declarations_recues.is_empty(), "expirée : plus listée");
    assert!(est_refus(&ent.individu_repondre_for_actor(etab, prof2, d3, true).await.unwrap_err(), RefusMetier::Conflit));
    assert!(!meme(&ent, etab, cpe, prof2).await);
    // La base refuse aussi la confirmation d'une déclaration expirée en SQL direct.
    {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        let e = sqlx::query("UPDATE ent_individu_declaration SET issue = 'confirmee' WHERE id = $1")
            .bind(d3)
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert!(format!("{e}").contains("expirée"), "{e}");
    }

    // Quota : au-delà de 5 par jour, refus (même pour des adresses inconnues).
    let (sec, _) = compte(&ent, etab, "administratif").await;
    let mut refuse = false;
    for i in 0..6 {
        if ent.individu_declarer_for_actor(etab, sec, &format!("x{i}@inconnu.local")).await.is_err() {
            refuse = true;
        }
    }
    assert!(refuse, "la sixième déclaration de la journée est refusée");
}

/// Ajout seul : rien ne change hors la levée (une fois) et l'issue (une fois) ; la suppression d'une fiche emporte ses
/// liens ; la naissance est contrôlée.
#[tokio::test]
async fn individu_tables_en_ajout_seul() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let (dir, _) = compte(&ent, etab, "direction").await;
    let (parent, _) = compte(&ent, etab, "parent").await;
    let p_dir = fiche_du_compte(&ent, etab, dir).await;
    let p_parent = fiche_du_compte(&ent, etab, parent).await;
    let p_sans = fiche_sans_compte(&ent, etab, "Durand", None).await;
    let l = ent
        .individu_relier_for_actor(etab, dir, "direction", p_dir, p_parent, "deux_comptes", false)
        .await
        .unwrap();
    let l2 = ent
        .individu_relier_for_actor(etab, dir, "direction", p_parent, p_sans, "fiche_sans_compte", false)
        .await
        .unwrap();

    async fn sql(ent: &PgEnt, etab: Uuid, q: &str, id: Uuid) -> Result<u64, sqlx::Error> {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        let r = sqlx::query(q).bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(r.rows_affected())
    }
    for q in [
        "UPDATE ent_individu_lien SET pose_par_user_id = gen_random_uuid() WHERE id = $1",
        "UPDATE ent_individu_lien SET motif_code = 'autre' WHERE id = $1",
        "UPDATE ent_individu_lien SET leve_le = now(), leve_par_user_id = gen_random_uuid(), leve_motif_code = 'autre', origine = 'declaration_confirmee' WHERE id = $1",
    ] {
        let r = sql(&ent, etab, q, l).await;
        assert!(r.is_err(), "{q}");
    }
    let e = sql(&ent, etab, "DELETE FROM ent_individu_lien WHERE id = $1", l).await.unwrap_err();
    assert!(format!("{e}").contains("ajout seul"), "{e}");
    // Une levée en SQL direct passe une fois, puis plus rien ne change.
    sql(&ent, etab, "UPDATE ent_individu_lien SET leve_le = now(), leve_par_user_id = gen_random_uuid(), leve_motif_code = 'autre' WHERE id = $1", l)
        .await
        .unwrap();
    assert!(sql(&ent, etab, "UPDATE ent_individu_lien SET leve_motif_code = 'erreur' WHERE id = $1", l).await.is_err());
    // Un lien ne naît pas levé.
    let e = {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        sqlx::query(
            "INSERT INTO ent_individu_lien (id, etablissement_id, personne_a, personne_b, origine, motif_code, pose_par_user_id, leve_le, leve_par_user_id, leve_motif_code)
             VALUES (gen_random_uuid(), $1, LEAST($2, $3), GREATEST($2, $3), 'declaration_direction', 'autre', $4, now(), $4, 'autre')",
        )
        .bind(etab)
        .bind(p_dir)
        .bind(p_parent)
        .bind(dir)
        .execute(&mut *tx)
        .await
        .unwrap_err()
    };
    assert!(format!("{e}").contains("naît actif"), "{e}");
    // TRUNCATE refusé.
    {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        assert!(sqlx::query("TRUNCATE ent_individu_lien").execute(&mut *tx).await.is_err());
    }
    // La fiche sans compte supprimée emporte son lien (cascade), sans erreur.
    sql(&ent, etab, "DELETE FROM ent_personne WHERE id = $1", p_sans).await.unwrap();
    let reste: i64 = {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        sqlx::query_scalar("SELECT count(*) FROM ent_individu_lien WHERE id = $1")
            .bind(l2)
            .fetch_one(&mut *tx)
            .await
            .unwrap()
    };
    assert_eq!(reste, 0);
}

/// Un lien ne vaut que dans son établissement ; une fiche d'un autre établissement est introuvable.
#[tokio::test]
async fn individu_cloisonne_par_etablissement() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let (dir_a, _) = compte(&ent, a, "direction").await;
    let (dir_b, _) = compte(&ent, b, "direction").await;
    let (prof, _) = compte(&ent, a, "prof").await;
    let (parent, _) = compte(&ent, a, "parent").await;
    let p_prof = fiche_du_compte(&ent, a, prof).await;
    let p_parent = fiche_du_compte(&ent, a, parent).await;
    let p_b = fiche_sans_compte(&ent, b, "Ailleurs", None).await;
    ent.individu_relier_for_actor(a, dir_a, "direction", p_prof, p_parent, "deux_comptes", false)
        .await
        .unwrap();
    assert!(meme(&ent, a, prof, parent).await);
    assert!(!meme(&ent, b, prof, parent).await, "le lien de A ne vaut pas dans B");
    let e = ent
        .individu_relier_for_actor(a, dir_a, "direction", p_prof, p_b, "autre", false)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    assert!(ent.individu_liens_for_actor(b, dir_b, "direction").await.unwrap().is_empty());
}

/// Fusion : la fiche absorbée suit sa cible ; défusionnée, elle ne la suit plus.
#[tokio::test]
async fn individu_suit_la_fusion_de_fiches() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let (dir, _) = compte(&ent, etab, "direction").await;
    let (prof, _) = compte(&ent, etab, "prof").await;
    let (parent, _) = compte(&ent, etab, "parent").await;
    let p_prof = fiche_du_compte(&ent, etab, prof).await;
    let p_parent = fiche_du_compte(&ent, etab, parent).await;
    let p_doublon = fiche_sans_compte(&ent, etab, "Doublon", None).await;
    ent.individu_relier_for_actor(etab, dir, "direction", p_prof, p_doublon, "fiche_sans_compte", false)
        .await
        .unwrap();
    // Le doublon est absorbé par la fiche du parent : prof et parent deviennent le même individu.
    let poser = |cible: Option<Uuid>| {
        let ent = &ent;
        async move {
            let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
            sqlx::query("UPDATE ent_personne SET fusionnee_dans = $2 WHERE etablissement_id = $1 AND id = $3")
                .bind(etab)
                .bind(cible)
                .bind(p_doublon)
                .execute(&mut *tx)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }
    };
    poser(Some(p_parent)).await;
    assert!(meme(&ent, etab, prof, parent).await, "la fiche absorbée suit sa cible");
    poser(None).await;
    assert!(!meme(&ent, etab, prof, parent).await, "défusion : retour");
}

/// Propositions : une coordonnée partagée entre un agent et une autre fiche, jamais la coordonnée ; écartée, plus
/// proposée ; reliée, plus proposée ; jamais pour deux fiches sans agent.
#[tokio::test]
async fn individu_propositions_sans_rapprochement_automatique() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let (dir, _) = compte(&ent, etab, "direction").await;
    let (prof, _) = compte(&ent, etab, "prof").await;
    let (cpe, _) = compte(&ent, etab, "cpe").await;
    let p_prof = fiche_du_compte(&ent, etab, prof).await;
    let p_cpe = fiche_du_compte(&ent, etab, cpe).await;
    {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        for p in [p_prof, p_cpe] {
            sqlx::query("UPDATE ent_personne SET telephone = $3 WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(p)
                .bind(if p == p_prof { "06 12 34 56 78" } else { "06 99 99 99 01" })
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        tx.commit().await.unwrap();
    }
    let parent_tel = fiche_sans_compte(&ent, etab, "Parent tel", Some("+33 6 12 34 56 78")).await;
    let parent_cpe = fiche_sans_compte(&ent, etab, "Parent cpe", Some("0699999901")).await;
    // Deux fiches sans agent qui partagent un téléphone : jamais proposées.
    let _x = fiche_sans_compte(&ent, etab, "X", Some("0711111111")).await;
    let _y = fiche_sans_compte(&ent, etab, "Y", Some("0711111111")).await;

    let props = ent.individu_propositions_for_actor(etab, dir, "direction").await.unwrap();
    let paire = |a: Uuid, b: Uuid| if a < b { (a, b) } else { (b, a) };
    let vues: Vec<(Uuid, Uuid)> = props.iter().map(|p| (p.personne_a, p.personne_b)).collect();
    assert!(vues.contains(&paire(p_prof, parent_tel)), "{props:?}");
    assert!(vues.contains(&paire(p_cpe, parent_cpe)), "{props:?}");
    assert!(!vues.contains(&paire(_x, _y)), "sans agent : pas de proposition");
    let json = serde_json::to_string(&props).unwrap();
    assert!(!json.contains("56 78") && !json.contains("5678"), "jamais la coordonnée : {json}");
    assert!(props.iter().all(|p| p.raisons == vec!["telephone".to_string()]));
    assert!(fiches(&ent, etab, prof).await == vec![p_prof], "une proposition ne relie rien");
    assert!(ent.individu_propositions_for_actor(etab, prof, "prof").await.is_err());

    // Écartée : plus proposée.
    ent.individu_ecarter_for_actor(etab, dir, "direction", parent_cpe, p_cpe, "homonyme").await.unwrap();
    ent.individu_ecarter_for_actor(etab, dir, "direction", parent_cpe, p_cpe, "homonyme").await.unwrap();
    // Acceptée : reliée, plus proposée.
    ent.individu_relier_for_actor(etab, dir, "direction", parent_tel, p_prof, "fiche_sans_compte", true)
        .await
        .unwrap();
    let vues: Vec<(Uuid, Uuid)> = ent
        .individu_propositions_for_actor(etab, dir, "direction")
        .await
        .unwrap()
        .iter()
        .map(|p| (p.personne_a, p.personne_b))
        .collect();
    assert!(!vues.contains(&paire(p_prof, parent_tel)));
    assert!(!vues.contains(&paire(p_cpe, parent_cpe)));
    let liens = ent.individu_liens_for_actor(etab, dir, "direction").await.unwrap();
    assert_eq!(liens.len(), 1);
    assert_eq!(liens[0].origine, "proposition_acceptee");
}
