//! Messagerie, lot M3 (`docs/socles/16_MESSAGERIE.md` §11.1, §19.3, R-M11, R-M26, R-M41, R-M45 ; CM-31, CM-44,
//! CM-49) : boîtes de service, contre la base locale de test (feature `labo`). Même discipline que M0a à M2 :
//! un **témoin** précède chaque attaque (la même action par un acteur légitime réussit), chaque scénario sème son
//! propre établissement.

use super::tests_messagerie_m1::{
    compte_eleve, labo, lier, lit, nouveau_foyer, nouvel_utilisateur, role,
};
use super::*;
use chrono::{Duration, NaiveTime, TimeZone, Utc};
use domain_eleve::RepondreFil;
use ent_auth::Role;
use uuid::Uuid;

fn jour() -> chrono::NaiveDate {
    domain_eleve::horloge::jour_paris()
}

async fn personne_de(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    let (p,): (Option<Uuid>,) = sqlx::query_as("SELECT ent_msg_personne_du_compte($1, $2)")
        .bind(etab)
        .bind(user)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    p.expect("fiche personne")
}

/// L'agent (fiche socle 6) d'un compte de personnel, engagé depuis longtemps.
async fn agent_de(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    let personne = personne_de(ent, etab, user).await;
    sqlx::query(
        "INSERT INTO ent_agent (id, etablissement_id, personne_id, source) VALUES ($1,$2,$3,'saisie')
         ON CONFLICT (etablissement_id, personne_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(personne)
    .execute(ent.pool())
    .await
    .unwrap();
    let (agent,): (Uuid,) =
        sqlx::query_as("SELECT id FROM ent_agent WHERE etablissement_id = $1 AND personne_id = $2")
            .bind(etab)
            .bind(personne)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    sqlx::query(
        "INSERT INTO ent_agent_engagement (id, etablissement_id, agent_id, du, nature, employeur_type)
         VALUES (gen_random_uuid(), $1, $2, $3, 'titulaire', 'etat') ON CONFLICT DO NOTHING",
    )
    .bind(etab)
    .bind(agent)
    .bind(jour() - Duration::days(400))
    .execute(ent.pool())
    .await
    .unwrap();
    agent
}

async fn type_mission(ent: &PgEnt, etab: Uuid, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_mission_type (id, etablissement_id, code, libelle, perimetres_admis)
         VALUES ($1,$2,$3,$3, ARRAY['etablissement'])",
    )
    .bind(id)
    .bind(etab)
    .bind(code)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

async fn mission(
    ent: &PgEnt,
    etab: Uuid,
    agent: Uuid,
    type_id: Uuid,
    du: chrono::NaiveDate,
    au: Option<chrono::NaiveDate>,
) {
    sqlx::query(
        "INSERT INTO ent_mission (id, etablissement_id, type_id, agent_id, du, au) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(type_id)
    .bind(agent)
    .bind(du)
    .bind(au)
    .execute(ent.pool())
    .await
    .unwrap();
}

/// Une boîte du catalogue, avec un type de mission rattaché ; rend `(boîte, type de mission)`. Les membres
/// reçoivent une mission depuis 30 jours.
async fn boite(ent: &PgEnt, etab: Uuid, dir: Uuid, code: &str, membres: &[Uuid]) -> (Uuid, Uuid) {
    let b = ent
        .creer_boite_for_actor(
            etab,
            dir,
            role(Role::Direction),
            SaisieBoite { code: code.into(), catalogue: true, ..Default::default() },
        )
        .await
        .expect("création de la boîte");
    let t = type_mission(ent, etab, &format!("m_{code}")).await;
    ent.rattacher_mission_boite_for_actor(etab, dir, role(Role::Direction), b.id, t, true)
        .await
        .expect("rattachement");
    for m in membres {
        let a = agent_de(ent, etab, *m).await;
        mission(ent, etab, a, t, jour() - Duration::days(30), None).await;
    }
    (b.id, t)
}

async fn ouvrir_boite(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    r: Role,
    b: Uuid,
    eleve: Uuid,
    sujet: &str,
) -> OuvertureBoite {
    ent.ouvrir_fil_boite_for_actor(etab, user, role(r), b, eleve, sujet, &format!("Contenu confidentiel — {sujet}"))
        .await
        .expect("ouverture vers la boîte")
}

fn rep(etab: Uuid, user: Uuid, fil: Uuid, corps: &str) -> RepondreFil {
    RepondreFil { etablissement_id: etab, fil_id: fil, corps: corps.into(), actor_user_id: user }
}

async fn nb(ent: &PgEnt, sql: &str, binds: &[Uuid]) -> i64 {
    super::tests_messagerie_m1::compter(ent, sql, binds).await
}

// ─── Tri déterministe, catalogue (sans base) ──────────────────────────────────────────────────

#[test]
fn m3_tri_deterministe_mots_d_alerte_sans_accent_ni_casse() {
    assert!(urgence_deterministe("Mon fils parle de SUICIDE ce soir"));
    assert!(urgence_deterministe("Il se fait frapper à la sortie"));
    assert!(urgence_deterministe("harcèlement en cours"));
    assert!(urgence_deterministe("C'est URGENT"));
    assert!(!urgence_deterministe("Bonjour, une question sur le planning de la semaine"));
    assert!(!urgence_deterministe("Merci pour votre retour"));
}

#[test]
fn m3_catalogue_les_boites_de_sante_sont_c3() {
    for (code, _, sens) in CATALOGUE_BOITES {
        let attendu = matches!(*code, "infirmerie" | "psychologue" | "assistante_sociale");
        assert_eq!(*sens == "C3", attendu, "{code}");
    }
    assert_eq!(cible_de_boite("infirmerie") as u8, crate::msg_matrice::Cible::Sante as u8);
    assert!(BANDEAU_URGENCE.contains("3018") && BANDEAU_URGENCE.contains("119"));
}

// ─── Gestion et membres par mission ────────────────────────────────────────────────────────

#[tokio::test]
async fn m3_creation_reservee_a_la_direction_et_c3_non_declassable() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, prof) = (cast.direction.user_id, cast.prof.user_id);
    let saisie = |code: &str| SaisieBoite { code: code.into(), catalogue: true, ..Default::default() };
    // Témoin : la direction crée.
    let vs = ent.creer_boite_for_actor(etab, dir, role(Role::Direction), saisie("vie_scolaire")).await.unwrap();
    assert_eq!(vs.sensibilite, "C2");
    // Un professeur ne crée rien, un doublon est refusé, une boîte hors catalogue sans libellé aussi.
    assert!(matches!(
        ent.creer_boite_for_actor(etab, prof, role(Role::Prof), saisie("secretariat")).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.creer_boite_for_actor(etab, dir, role(Role::Direction), saisie("vie_scolaire")).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(ent.creer_boite_for_actor(etab, dir, role(Role::Direction), saisie("inconnue")).await.is_err());
    // L'infirmerie est C3 quoi qu'on demande ; la base refuse tout déclassement.
    let inf = ent.creer_boite_for_actor(etab, dir, role(Role::Direction), saisie("infirmerie")).await.unwrap();
    assert_eq!(inf.sensibilite, "C3");
    let r = sqlx::query("UPDATE ent_msg_boite SET sensibilite = 'C2' WHERE id = $1").bind(inf.id).execute(ent.pool()).await;
    assert!(r.is_err(), "une boîte de santé ne se déclasse pas");
    // Boîte personnalisée : code libre, mais jamais un code du catalogue.
    let perso = SaisieBoite {
        code: "club_theatre".into(),
        catalogue: false,
        libelle: Some("Club théâtre".into()),
        ..Default::default()
    };
    assert!(ent.creer_boite_for_actor(etab, dir, role(Role::Direction), perso).await.is_ok());
    let faux = SaisieBoite { code: "cdi".into(), catalogue: false, libelle: Some("Faux CDI".into()), ..Default::default() };
    assert!(ent.creer_boite_for_actor(etab, dir, role(Role::Direction), faux).await.is_err());
}

#[tokio::test]
async fn m3_membres_par_mission_et_a_la_date_jamais_une_liste_figee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, prof, adm) =
        (cast.direction.user_id, cast.cpe.user_id, cast.prof.user_id, cast.administratif.user_id);
    let (b, t) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    // Témoin : le porteur de la mission est membre.
    assert!(ent.est_membre_boite(etab, b, cpe).await.unwrap());
    // Mission terminée hier, mission future, personne sans mission : jamais membres.
    let a_prof = agent_de(&ent, etab, prof).await;
    mission(&ent, etab, a_prof, t, jour() - Duration::days(60), Some(jour() - Duration::days(1))).await;
    let a_adm = agent_de(&ent, etab, adm).await;
    mission(&ent, etab, a_adm, t, jour() + Duration::days(3), None).await;
    assert!(!ent.est_membre_boite(etab, b, prof).await.unwrap(), "mission échue");
    assert!(!ent.est_membre_boite(etab, b, adm).await.unwrap(), "mission pas encore commencée");
    assert!(!ent.est_membre_boite(etab, b, dir).await.unwrap(), "la direction n'est pas membre d'office");
    // La liste des membres : direction et membres seulement ; jamais une famille ni un non-membre.
    let liste = ent.membres_boite_for_actor(etab, dir, role(Role::Direction), b).await.unwrap();
    assert_eq!(liste.len(), 1, "le seul porteur d'une mission en cours : le CPE");
    assert!(ent.membres_boite_for_actor(etab, cpe, role(Role::Cpe), b).await.is_ok());
    assert!(matches!(
        ent.membres_boite_for_actor(etab, cast.parent_lea.user_id, role(Role::Parent), b).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.membres_boite_for_actor(etab, adm, role(Role::Administratif), b).await,
        Err(EntDbError::Forbidden)
    ));
    // Une boîte désactivée n'a plus de membres.
    ent.parametrer_boite_for_actor(etab, dir, role(Role::Direction), b, ParamBoite { actif: Some(false), ..Default::default() })
        .await
        .unwrap();
    assert!(!ent.est_membre_boite(etab, b, cpe).await.unwrap());
}

// ─── Fil de boîte : lire = être membre ─────────────────────────────────────────────────────────

#[tokio::test]
async fn m3_une_famille_ecrit_a_la_vie_scolaire_les_membres_lisent_et_repondent_au_nom_de_la_boite() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, prof, adm) =
        (cast.direction.user_id, cast.cpe.user_id, cast.prof.user_id, cast.administratif.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (b, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, b, harry, "Retard de Harry").await;
    // Témoins : le parent relit son fil, le membre le lit.
    assert!(lit(&ent, etab, lily, Role::Parent, o.fil_id).await);
    assert!(lit(&ent, etab, cpe, Role::Cpe, o.fil_id).await, "le membre de la boîte lit");
    // Attaques : ni un professeur, ni le secrétariat, ni la direction (pont des rôles réservés), ni l'autre parent.
    assert!(!lit(&ent, etab, prof, Role::Prof, o.fil_id).await, "un professeur n'est pas membre");
    assert!(!lit(&ent, etab, adm, Role::Administratif, o.fil_id).await, "le secrétariat n'est pas membre");
    assert!(
        !lit(&ent, etab, dir, Role::Direction, o.fil_id).await,
        "la direction, même avec le droit de lire les fils réservés, ne lit pas une boîte dont elle n'est pas membre"
    );
    assert!(!lit(&ent, etab, cast.parent_tom.user_id, Role::Parent, o.fil_id).await);
    // Un membre répond au nom de la boîte ; la famille lit « Vie scolaire — Prénom N. », jamais la liste des membres.
    ent.repondre_au_nom_de_boite_for_actor(role(Role::Cpe), rep(etab, cpe, o.fil_id, "Nous prenons note, merci."), b)
        .await
        .expect("réponse de la boîte");
    let (_, messages) = ent.list_messages_for_actor(etab, lily, role(Role::Parent), o.fil_id).await.unwrap();
    assert_eq!(messages.len(), 2);
    let libelles = ent.expediteurs_boite_for_actor(etab, lily, role(Role::Parent), o.fil_id).await.unwrap();
    assert_eq!(libelles.len(), 1);
    assert!(libelles[0].1.starts_with("Vie scolaire — "), "{}", libelles[0].1);
    // Le membre qui répond n'est pas devenu participant nominatif (il quitterait la lecture en quittant la mission).
    let (nominatifs,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_msg_participant p JOIN ent_personne pe ON pe.id = p.personne_id
          WHERE p.fil_id = $1 AND p.au IS NULL AND pe.user_id = $2",
    )
    .bind(o.fil_id)
    .bind(cpe)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(nominatifs, 0);
    // Un non-membre ne répond pas au nom de la boîte.
    assert!(matches!(
        ent.repondre_au_nom_de_boite_for_actor(role(Role::Prof), rep(etab, prof, o.fil_id, "Intrus"), b).await,
        Err(EntDbError::Forbidden)
    ));
    // Le départ du membre le sort du fil à l'instant, sans ligne à fermer à la main.
    sqlx::query("UPDATE ent_mission SET au = ent_aujourdhui_etab($1) - 1 WHERE etablissement_id = $1").bind(etab).execute(ent.pool()).await.unwrap();
    assert!(!lit(&ent, etab, cpe, Role::Cpe, o.fil_id).await, "fin de mission : plus de lecture");
    assert!(lit(&ent, etab, lily, Role::Parent, o.fil_id).await, "la famille garde son fil");
}

#[tokio::test]
async fn m3_note_interne_invisible_de_la_famille_et_des_non_membres() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, prof) = (cast.direction.user_id, cast.cpe.user_id, cast.prof.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (b, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, b, harry, "Absence").await;
    ent.ecrire_note_interne_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, b, "Famille déjà prévenue par téléphone, à surveiller.")
        .await
        .expect("note interne");
    assert_eq!(ent.notes_internes_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, b).await.unwrap().len(), 1, "témoin");
    // La famille ne voit ni le texte de la note, ni sa trace dans les messages.
    let (_, messages) = ent.list_messages_for_actor(etab, lily, role(Role::Parent), o.fil_id).await.unwrap();
    assert!(messages.iter().all(|m| !m.message.corps.contains("à surveiller")));
    assert_eq!(messages.len(), 1, "la note n'est pas un message");
    assert!(matches!(
        ent.notes_internes_for_actor(etab, lily, role(Role::Parent), o.fil_id, b).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.notes_internes_for_actor(etab, prof, role(Role::Prof), o.fil_id, b).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.ecrire_note_interne_for_actor(etab, prof, role(Role::Prof), o.fil_id, b, "Intrus").await,
        Err(EntDbError::Forbidden)
    ));
    // Une note est en ajout seul.
    assert!(sqlx::query("DELETE FROM ent_msg_note_interne WHERE fil_id = $1").bind(o.fil_id).execute(ent.pool()).await.is_err());
}

// ─── Boîtes C3 (R-M11, CM-31) ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m3_cm31_un_eleve_ecrit_a_l_infirmiere_seule_la_membre_lit() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let (inf, cpe, prof) = (cast.infirmieres[0].user_id, cast.cpe.user_id, cast.prof.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let eleve = compte_eleve(&ent, etab, harry).await;
    let (b, _) = boite(&ent, etab, dir, "infirmerie", &[inf]).await;
    let o = ouvrir_boite(&ent, etab, eleve, Role::Eleve, b, harry, "Je ne me sens pas bien").await;
    // Témoins : l'élève relit, l'infirmière lit.
    assert!(lit(&ent, etab, eleve, Role::Eleve, o.fil_id).await);
    assert!(lit(&ent, etab, inf, Role::Infirmerie, o.fil_id).await);
    // Attaques : parents, vie scolaire, professeur, direction (droit de lire les fils réservés compris).
    for (u, r) in [(lily, Role::Parent), (cpe, Role::Cpe), (prof, Role::Prof), (dir, Role::Direction)] {
        assert!(!lit(&ent, etab, u, r, o.fil_id).await, "{r:?} ne lit pas une boîte C3");
    }
    // Ni liste de fils, ni file « non traités » : aucune trace du fil pour la direction.
    let page = ent
        .list_fils_for_actor(etab, dir, role(Role::Direction), None, false, None)
        .await
        .map(|v| v.iter().any(|f| f.id == o.fil_id))
        .unwrap_or(false);
    assert!(!page, "le fil C3 ne figure dans aucune liste de la direction");
    // Aucun avis aux parents (le fil est celui de l'élève) ; l'événement n'expose ni sujet ni élève.
    let (notifs_parent,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1")
        .bind(lily)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(notifs_parent, 0);
    let (avec_eleve,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE aggregate_id = $1 AND eleve_id IS NOT NULL",
    )
    .bind(o.fil_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(avec_eleve, 0, "aucun événement C3 ne porte l'identité de l'élève");
    // L'avis à l'infirmière est neutre (ni sujet, ni élève, ni boîte).
    let (titre, corps, eleve_n): (String, String, Option<Uuid>) = sqlx::query_as(
        "SELECT titre, corps, eleve_id FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'messagerie_boite'",
    )
    .bind(inf)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(eleve_n.is_none());
    assert!(!format!("{titre}{corps}").to_lowercase().contains("malade") && !format!("{titre}{corps}").contains("bien"));
}

#[tokio::test]
async fn m3_c3_remplacante_datee_et_aucun_transfert_vers_une_boite_ordinaire() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, inf, cpe, prof) = (cast.direction.user_id, cast.infirmieres[0].user_id, cast.cpe.user_id, cast.prof.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (b, _) = boite(&ent, etab, dir, "infirmerie", &[inf]).await;
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, b, harry, "Allergie").await;
    // Un non-membre ne désigne pas de remplaçante d'une boîte C3 ; l'infirmière (membre) le peut.
    let p_prof = personne_de(&ent, etab, prof).await;
    assert!(matches!(
        ent.designer_membre_boite_for_actor(etab, cpe, role(Role::Cpe), b, p_prof, true, jour(), None).await,
        Err(EntDbError::Forbidden)
    ));
    // Jamais un parent ni un élève comme membre.
    let p_parent = personne_de(&ent, etab, lily).await;
    assert!(ent.designer_membre_boite_for_actor(etab, inf, role(Role::Infirmerie), b, p_parent, true, jour(), None).await.is_err());
    // Témoin : désignation datée à partir de demain → pas encore membre ; à partir d'aujourd'hui → membre.
    ent.designer_membre_boite_for_actor(etab, inf, role(Role::Infirmerie), b, p_prof, true, jour() + Duration::days(1), None)
        .await
        .expect("désignation");
    assert!(!ent.est_membre_boite(etab, b, prof).await.unwrap(), "pas avant la date");
    assert!(!lit(&ent, etab, prof, Role::Prof, o.fil_id).await);
    ent.designer_membre_boite_for_actor(etab, inf, role(Role::Infirmerie), b, p_prof, true, jour(), Some(jour()))
        .await
        .unwrap();
    assert!(lit(&ent, etab, prof, Role::Prof, o.fil_id).await, "remplaçante désignée pour aujourd'hui");
    sqlx::query("UPDATE ent_msg_boite_designation SET au = ent_aujourdhui_etab(etablissement_id) - 1, du = ent_aujourdhui_etab(etablissement_id) - 2")
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(!lit(&ent, etab, prof, Role::Prof, o.fil_id).await, "désignation échue");
    // Transfert C3 → boîte ordinaire refusé, et inversement (le fil ne change pas de sensibilité).
    assert!(matches!(
        ent.transferer_fil_boite_for_actor(etab, inf, role(Role::Infirmerie), o.fil_id, b, vs, Some("à la VS")).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    let o2 = ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "Question").await;
    assert!(ent.transferer_fil_boite_for_actor(etab, cpe, role(Role::Cpe), o2.fil_id, vs, b, None).await.is_err());
}

// ─── Traitement ────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn m3_traitement_prise_en_charge_attente_et_traite_apres_reponse_ecrite() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, prof) = (cast.direction.user_id, cast.cpe.user_id, cast.prof.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (b, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, b, harry, "Demande d'entretien").await;
    let file = |statut| ent.file_boite_for_actor(etab, cpe, role(Role::Cpe), b, statut, 50);
    assert_eq!(file(Some(Traitement::Nouveau)).await.unwrap().len(), 1, "témoin : un fil nouveau");
    // Un non-membre ne voit pas la file ; un non-membre ne prend rien en charge.
    assert!(matches!(ent.file_boite_for_actor(etab, prof, role(Role::Prof), b, None, 50).await, Err(EntDbError::Forbidden)));
    assert!(matches!(
        ent.prendre_en_charge_for_actor(etab, prof, role(Role::Prof), o.fil_id, b).await,
        Err(EntDbError::Forbidden)
    ));
    ent.prendre_en_charge_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, b).await.unwrap();
    let l = file(Some(Traitement::Pris)).await.unwrap();
    assert_eq!(l.len(), 1);
    assert!(l[0].pris_par.is_some());
    // R-M45 : une demande de la famille ne passe pas « traité » sans réponse écrite.
    assert!(matches!(
        ent.traiter_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, b).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.repondre_au_nom_de_boite_for_actor(role(Role::Cpe), rep(etab, cpe, o.fil_id, "Rendez-vous jeudi 17 h."), b)
        .await
        .unwrap();
    ent.attendre_famille_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, b).await.unwrap();
    assert_eq!(file(Some(Traitement::AttenteFamille)).await.unwrap().len(), 1);
    ent.traiter_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, b).await.expect("traité après réponse");
    assert_eq!(file(Some(Traitement::Traite)).await.unwrap().len(), 1);
    // La famille répond : réouverture automatique.
    assert!(super::tests_messagerie_m1::ecrit(&ent, etab, lily, Role::Parent, o.fil_id, "Merci, je confirme.").await);
    assert_eq!(file(Some(Traitement::Nouveau)).await.unwrap().len(), 1, "réponse de la famille : rouvert");
    // Échéance : jamais dans le passé ; étiquette de la boîte.
    assert!(ent.poser_echeance_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, b, Some(jour() - Duration::days(1))).await.is_err());
    ent.poser_echeance_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, b, Some(jour() + Duration::days(2))).await.unwrap();
    ent.creer_etiquette_boite_for_actor(etab, dir, role(Role::Direction), b, "rendez_vous", "Rendez-vous").await.unwrap();
    ent.etiqueter_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, b, "rendez_vous", true).await.unwrap();
    let l = file(None).await.unwrap();
    assert_eq!(l[0].etiquettes, "Rendez-vous");
    assert!(l[0].echeance.is_some());
    assert!(ent.etiqueter_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, b, "inconnue", true).await.is_err());
}

#[tokio::test]
async fn m3_transfert_vers_une_autre_boite_la_famille_garde_son_fil() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, adm) = (cast.direction.user_id, cast.cpe.user_id, cast.administratif.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let (sec, _) = boite(&ent, etab, dir, "secretariat", &[adm]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "Certificat de scolarité").await;
    assert!(!lit(&ent, etab, adm, Role::Administratif, o.fil_id).await, "témoin : le secrétariat n'a pas encore le fil");
    ent.transferer_fil_boite_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, vs, sec, Some("C'est pour le secrétariat."))
        .await
        .expect("transfert");
    assert!(lit(&ent, etab, adm, Role::Administratif, o.fil_id).await, "la nouvelle boîte lit");
    assert!(!lit(&ent, etab, cpe, Role::Cpe, o.fil_id).await, "l'ancienne boîte n'a plus le fil");
    assert!(lit(&ent, etab, lily, Role::Parent, o.fil_id).await, "la famille garde son fil");
    let notes = ent.notes_internes_for_actor(etab, adm, role(Role::Administratif), o.fil_id, sec).await.unwrap();
    assert!(notes.iter().any(|n| n.transfert && n.corps.contains("secrétariat")));
    // La file de l'ancienne boîte ne l'affiche plus, celle de la nouvelle oui.
    assert!(ent.file_boite_for_actor(etab, cpe, role(Role::Cpe), vs, None, 50).await.unwrap().is_empty());
    assert_eq!(ent.file_boite_for_actor(etab, adm, role(Role::Administratif), sec, None, 50).await.unwrap().len(), 1);
    // Le transfert est tracé en ajout seul.
    assert!(sqlx::query("DELETE FROM ent_msg_transfert WHERE fil_id = $1").bind(o.fil_id).execute(ent.pool()).await.is_err());
    // Le membre sorti ne reprend plus rien.
    assert!(ent.prendre_en_charge_for_actor(etab, cpe, role(Role::Cpe), o.fil_id, vs).await.is_err());
}

// ─── Horaires, réponse automatique, astreinte (CM-44, R-M26) ──────────────────────────────────────

#[tokio::test]
async fn m3_cm44_boite_fermee_reponse_automatique_bandeau_et_astreinte_pour_l_urgent() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, adm, astreinte) =
        (cast.direction.user_id, cast.cpe.user_id, cast.administratif.user_id, cast.cpes.get(1).map(|c| c.user_id));
    let astreinte = astreinte.unwrap_or(cast.compta.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (b, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe, adm]).await;
    ent.parametrer_boite_for_actor(
        etab,
        dir,
        role(Role::Direction),
        b,
        ParamBoite {
            reponse_auto: Some(Some("La vie scolaire répond du lundi au vendredi, de 8 h à 17 h.".into())),
            delai_vise_jours: Some(2),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let h = |j: i16| HoraireSaisie {
        jour_iso: j,
        debut: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        fin: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
    };
    ent.definir_horaires_boite_for_actor(etab, dir, role(Role::Direction), b, (1..=5).map(h).collect()).await.unwrap();
    // Horaires qui se chevauchent : refusés.
    assert!(ent
        .definir_horaires_boite_for_actor(
            etab,
            dir,
            role(Role::Direction),
            b,
            vec![h(1), HoraireSaisie { jour_iso: 1, debut: NaiveTime::from_hms_opt(9, 0, 0).unwrap(), fin: NaiveTime::from_hms_opt(10, 0, 0).unwrap() }]
        )
        .await
        .is_err());
    ent.definir_horaires_boite_for_actor(etab, dir, role(Role::Direction), b, (1..=5).map(h).collect()).await.unwrap();
    // Vendredi 13 novembre 2026 : 11 h (ouvert) et 18 h (fermé), heure de Paris (UTC+1).
    let ouvert = Utc.with_ymd_and_hms(2026, 11, 13, 10, 0, 0).unwrap();
    let ferme = Utc.with_ymd_and_hms(2026, 11, 13, 17, 0, 0).unwrap();
    assert!(ent.boite_ouverte_a(etab, b, ouvert).await.unwrap());
    assert!(!ent.boite_ouverte_a(etab, b, ferme).await.unwrap());
    assert!(!ent.boite_ouverte_a(etab, b, Utc.with_ymd_and_hms(2026, 11, 14, 10, 0, 0).unwrap()).await.unwrap(), "samedi");
    // Astreinte désignée pour la soirée.
    let p_ast = personne_de(&ent, etab, astreinte).await;
    ent.definir_astreinte_boite_for_actor(etab, dir, role(Role::Direction), b, p_ast, ferme - Duration::hours(1), ferme + Duration::hours(10))
        .await
        .unwrap();
    // Message ordinaire, boîte ouverte : pas de réponse automatique, les deux membres sont avertis.
    let o1 = ent
        .ouvrir_fil_boite_a(etab, lily, role(Role::Parent), b, harry, "Question", "Un renseignement sur le planning.", ouvert)
        .await
        .unwrap();
    assert!(o1.reponse_auto.is_none() && o1.accueil.ouverte_maintenant && !o1.urgence);
    // Message ordinaire, boîte fermée : réponse automatique, bandeau, délai affiché ; l'astreinte n'est pas dérangée.
    let o2 = ent
        .ouvrir_fil_boite_a(etab, lily, role(Role::Parent), b, harry, "Autre question", "Je voulais savoir pour demain.", ferme)
        .await
        .unwrap();
    assert!(o2.reponse_auto.as_deref().unwrap().contains("8 h à 17 h"));
    assert!(!o2.accueil.ouverte_maintenant && o2.accueil.delai_vise_jours == 2 && o2.accueil.bandeau_urgence.contains("3018"));
    let notifs = |u: Uuid| {
        let ent = &ent;
        async move {
            let (n,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'messagerie_boite'",
            )
            .bind(u)
            .fetch_one(ent.pool())
            .await
            .unwrap();
            n
        }
    };
    let (avant_ast, avant_adm) = (notifs(astreinte).await, notifs(adm).await);
    // Message urgent, boîte fermée : seule l'astreinte est prévenue.
    let o3 = ent
        .ouvrir_fil_boite_a(etab, lily, role(Role::Parent), b, harry, "Urgent", "Il parle de suicide ce soir.", ferme)
        .await
        .unwrap();
    assert!(o3.urgence);
    assert_eq!(notifs(astreinte).await, avant_ast + 1, "l'astreinte est prévenue");
    assert_eq!(notifs(adm).await, avant_adm, "les autres membres ne sont pas dérangés hors horaires");
    // L'urgent passe en tête de la file.
    let file = ent.file_boite_for_actor(etab, cpe, role(Role::Cpe), b, None, 50).await.unwrap();
    assert_eq!(file[0].fil_id, o3.fil_id);
    assert!(file[0].urgence);
}

// ─── Matrice : aucun destinataire hors des droits ─────────────────────────────────────────────────

#[tokio::test]
async fn m3_aucun_destinataire_hors_des_droits_de_l_expediteur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, inf) = (cast.direction.user_id, cast.cpe.user_id, cast.infirmieres[0].user_id);
    let (lily, harry, tom) = (cast.parent_lea.user_id, cast.eleve_lea_id, cast.eleve_tom_id);
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let (bd, _) = boite(&ent, etab, dir, "direction", &[dir]).await;
    let (bi, _) = boite(&ent, etab, dir, "infirmerie", &[inf]).await;
    // Témoin : le parent écrit à la vie scolaire pour son enfant.
    ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "Témoin").await;
    // Un parent n'écrit jamais à propos d'un enfant sans lien avec lui.
    let _ = tom;
    let inconnu = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, date_naissance, created_at, updated_at)
         VALUES ($1,$2,'Sans','Lien','inscrit', ent_aujourdhui_etab($2) - 3000, now(), now())",
    )
    .bind(inconnu)
    .bind(etab)
    .execute(ent.pool())
    .await
    .unwrap();
    assert!(matches!(
        ent.ouvrir_fil_boite_for_actor(etab, lily, role(Role::Parent), vs, inconnu, "Autre enfant", "texte").await,
        Err(EntDbError::Forbidden)
    ));
    // Le réglage « familles → direction » ferme la boîte direction, jamais la santé.
    let ecrivables = |u: Uuid, r: Role, e: Uuid| {
        let ent = &ent;
        async move {
            ent.boites_ecrivables_for_actor(etab, u, role(r), e).await.unwrap().into_iter().map(|b| b.libelle).collect::<Vec<_>>()
        }
    };
    let avant = ecrivables(lily, Role::Parent, harry).await;
    assert!(avant.contains(&"Direction".to_string()) && avant.contains(&"Infirmerie".to_string()));
    let ferme = crate::msg_matrice::ReglagesMatrice { familles_vers_direction: false, ..Default::default() };
    ent.definir_reglages_matrice_for_actor(etab, dir, role(Role::Direction), ferme).await.unwrap();
    let apres = ecrivables(lily, Role::Parent, harry).await;
    assert!(!apres.contains(&"Direction".to_string()), "boîte direction fermée par le réglage");
    assert!(apres.contains(&"Infirmerie".to_string()) && apres.contains(&"Vie scolaire".to_string()));
    assert!(matches!(
        ent.ouvrir_fil_boite_for_actor(etab, lily, role(Role::Parent), bd, harry, "Directeur", "texte").await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent.ouvrir_fil_boite_for_actor(etab, lily, role(Role::Parent), bi, harry, "Infirmerie", "texte").await.is_ok());
    // Un parent sous interdiction de communication n'écrit plus à aucune boîte, et ne les voit plus.
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, du, motif)
         SELECT gen_random_uuid(), l.etablissement_id, l.eleve_id, l.personne_id, ARRAY['communication'], ent_aujourdhui_etab(l.etablissement_id) - 1, 'decision'
           FROM ent_eleve_lien l WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.user_id = $3",
    )
    .bind(etab)
    .bind(harry)
    .bind(lily)
    .execute(ent.pool())
    .await
    .map(|_| ())
    .unwrap_or_else(|e| eprintln!("interdiction (schéma du labo) : {e}"));
    let (interdit,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_eleve_lien_communicant WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3",
    )
    .bind(etab)
    .bind(lily)
    .bind(harry)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    if interdit == 0 {
        assert!(matches!(
            ent.ouvrir_fil_boite_for_actor(etab, lily, role(Role::Parent), vs, harry, "Interdit", "texte").await,
            Err(EntDbError::Forbidden)
        ));
        assert!(ent.boites_ecrivables_for_actor(etab, lily, role(Role::Parent), harry).await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn m3_un_eleve_majeur_ecrit_a_une_boite_pour_lui_et_ses_parents_n_y_ont_aucun_acces() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, inf) = (cast.direction.user_id, cast.infirmieres[0].user_id);
    let (b, _) = boite(&ent, etab, dir, "infirmerie", &[inf]).await;
    // Élève majeur, avec compte et un parent lié.
    let majeur = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, date_naissance, created_at, updated_at)
         VALUES ($1,$2,'Majeur','Test','inscrit',$3, now(), now())",
    )
    .bind(majeur)
    .bind(etab)
    .bind(jour() - Duration::days(365 * 19 + 10))
    .execute(ent.pool())
    .await
    .unwrap();
    let compte = compte_eleve(&ent, etab, majeur).await;
    let parent = nouvel_utilisateur(&ent, etab, "parent", "ParentMajeur").await;
    let foyer = nouveau_foyer(&ent, etab).await;
    lier(&ent, etab, majeur, parent, Some(foyer)).await;
    let o = ouvrir_boite(&ent, etab, compte, Role::Eleve, b, majeur, "Rendez-vous").await;
    assert!(lit(&ent, etab, compte, Role::Eleve, o.fil_id).await, "témoin : l'élève majeur lit");
    assert!(lit(&ent, etab, inf, Role::Infirmerie, o.fil_id).await);
    assert!(!lit(&ent, etab, parent, Role::Parent, o.fil_id).await, "ses parents n'ont aucun accès (C3, majeur)");
}

// ─── Téléphone (CM-49) et messages entrants (R-M41) ────────────────────────────────────────────────

#[tokio::test]
async fn m3_cm49_message_telephonique_l_accueil_ne_lit_pas_la_suite() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, prof) = (cast.direction.user_id, cast.cpe.user_id, cast.prof.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let accueil = nouvel_utilisateur(&ent, etab, "accueil", "Accueil").await;
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let saisie = |motif: &str| SaisieTelephone {
        boite_id: vs,
        eleve_id: harry,
        appelant_personne_id: None,
        appelant_libelle: Some("Une maman, au téléphone".into()),
        motif_code: motif.into(),
        rappel_demande: true,
        texte: "Léo est malade, il ne viendra pas aujourd'hui.".into(),
    };
    // Un professeur n'a pas ce droit ; un motif inconnu est refusé.
    assert!(matches!(
        ent.saisir_message_telephonique_for_actor(etab, prof, role(Role::Prof), saisie("absence_signalee")).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent.saisir_message_telephonique_for_actor(etab, accueil, role(Role::Accueil), saisie("inconnu")).await.is_err());
    let fil = ent
        .saisir_message_telephonique_for_actor(etab, accueil, role(Role::Accueil), saisie("absence_signalee"))
        .await
        .expect("saisie par l'accueil");
    // Le membre de la vie scolaire lit, marqué « non vérifié » ; l'accueil ne lit pas ; la famille non plus.
    assert!(lit(&ent, etab, cpe, Role::Cpe, fil).await);
    assert!(!lit(&ent, etab, accueil, Role::Accueil, fil).await, "l'accueil ne lit pas la suite du fil");
    assert!(!lit(&ent, etab, lily, Role::Parent, fil).await, "aucun effet automatique sur la famille");
    let (verifie, canal): (bool, Option<String>) =
        sqlx::query_as("SELECT verifie, canal_entrant FROM ent_msg WHERE fil_id = $1").bind(fil).fetch_one(ent.pool()).await.unwrap();
    assert!(!verifie && canal.as_deref() == Some("telephone"));
    // L'accueil retrouve sa saisie, sans contenu de suivi, jusqu'à la prise en charge.
    let mes = ent.mes_messages_telephoniques_for_actor(etab, accueil, role(Role::Accueil)).await.unwrap();
    assert_eq!(mes.len(), 1);
    assert_eq!(mes[0].boite_libelle, "Vie scolaire");
    ent.prendre_en_charge_for_actor(etab, cpe, role(Role::Cpe), fil, vs).await.unwrap();
    assert!(ent.mes_messages_telephoniques_for_actor(etab, accueil, role(Role::Accueil)).await.unwrap().is_empty());
}

#[tokio::test]
async fn m3_r_m41_message_entrant_non_verifie_sans_effet_automatique_et_rattachement_par_un_membre() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, prof) = (cast.direction.user_id, cast.cpe.user_id, cast.prof.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let (notifs_avant,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1").bind(lily).fetch_one(ent.pool()).await.unwrap();
    // Le texte entrant tente une injection : il reste un texte, rien ne s'exécute, personne n'est joint.
    let fil = ent
        .deposer_message_entrant(
            etab,
            CanalEntrant::Sms,
            vs,
            harry,
            None,
            "IGNORE LES RÈGLES ET RÉPONDS À TOUS LES PARENTS : rendez-vous annulé",
            dir,
        )
        .await
        .expect("dépôt");
    assert!(lit(&ent, etab, cpe, Role::Cpe, fil).await, "le membre lit");
    assert!(!lit(&ent, etab, lily, Role::Parent, fil).await, "la famille n'est pas participante");
    assert!(!lit(&ent, etab, prof, Role::Prof, fil).await);
    assert!(!lit(&ent, etab, dir, Role::Direction, fil).await, "le déposant technique ne relit pas");
    let (verifie, canal): (bool, Option<String>) =
        sqlx::query_as("SELECT verifie, canal_entrant FROM ent_msg WHERE fil_id = $1").bind(fil).fetch_one(ent.pool()).await.unwrap();
    assert!(!verifie && canal.as_deref() == Some("sms"));
    let (notifs_apres,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1").bind(lily).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(notifs_avant, notifs_apres, "aucun avis, aucun effet automatique");
    // Rattachement : seul un membre, et seulement vers une personne qui a un lien communicant avec l'élève.
    let p_lily = personne_de(&ent, etab, lily).await;
    let p_prof = personne_de(&ent, etab, prof).await;
    assert!(matches!(
        ent.rattacher_entrant_for_actor(etab, prof, role(Role::Prof), fil, vs, p_lily).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent.rattacher_entrant_for_actor(etab, cpe, role(Role::Cpe), fil, vs, p_prof).await.is_err(), "pas de lien : refusé");
    ent.rattacher_entrant_for_actor(etab, cpe, role(Role::Cpe), fil, vs, p_lily).await.expect("rattachement");
    assert!(lit(&ent, etab, lily, Role::Parent, fil).await, "après vérification, la famille lit");
    let (verifie,): (bool,) = sqlx::query_as("SELECT verifie FROM ent_msg WHERE fil_id = $1").bind(fil).fetch_one(ent.pool()).await.unwrap();
    assert!(verifie);
    // Un message entrant peut être classé sans réponse (ce n'est pas une demande de la famille).
    let fil2 = ent.deposer_message_entrant(etab, CanalEntrant::Courriel, vs, harry, None, "Publicité", dir).await.unwrap();
    ent.traiter_for_actor(etab, cpe, role(Role::Cpe), fil2, vs).await.expect("classé sans réponse");
}

// ─── Port vers N1 (conversion message ↔ demande, I-008) ──────────────────────────────────────────

#[tokio::test]
async fn m3_port_n1_seul_un_membre_convertit_jamais_un_fil_c3_ni_une_famille() {
    use crate::demandes_echanges::PortMessagerie;
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, prof, inf) = (cast.direction.user_id, cast.cpe.user_id, cast.prof.user_id, cast.infirmieres[0].user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let (bi, _) = boite(&ent, etab, dir, "infirmerie", &[inf]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "À transformer en demande").await;
    let c3 = ouvrir_boite(&ent, etab, lily, Role::Parent, bi, harry, "Confidentiel").await;
    let p_cpe = personne_de(&ent, etab, cpe).await;
    let p_prof = personne_de(&ent, etab, prof).await;
    let port = ent.resoudre_port_messagerie(etab, cpe, role(Role::Cpe), o.fil_id).await.unwrap();
    assert!(port.peut_convertir_fil(etab, p_cpe, o.fil_id), "témoin : le membre convertit");
    assert!(!port.peut_convertir_fil(etab, p_prof, o.fil_id), "pas la personne d'un autre");
    assert!(!port.peut_convertir_fil(etab, p_cpe, c3.fil_id), "pas un autre fil");
    for (u, r, f) in [
        (prof, Role::Prof, o.fil_id),
        (lily, Role::Parent, o.fil_id),
        (inf, Role::Infirmerie, c3.fil_id),
        (cpe, Role::Cpe, c3.fil_id),
    ] {
        let p = ent.resoudre_port_messagerie(etab, u, role(r), f).await.unwrap();
        assert!(!p.peut_convertir_fil(etab, personne_de(&ent, etab, u).await, f), "{r:?} sur {f}");
    }
    // Fail-closed : un port vide ne convertit rien et ne rouvre aucune conversation.
    let vide = MessagerieResolue::default();
    assert!(!vide.peut_convertir_fil(etab, p_cpe, o.fil_id));
    assert!(vide.conversation_depuis_demande(etab, p_cpe, Uuid::new_v4()).is_none());
}

// ─── Séparation entre établissements, rôle NOBYPASSRLS en mode strict ─────────────────────────────

#[tokio::test]
async fn m3_separation_entre_etablissements_rls_stricte_sans_contournement() {
    let Some((ent, cast_a)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let cast_b = ent.seed_labo_cast().await.expect("second établissement");
    let (a, b) = (cast_a.etablissement_id, cast_b.etablissement_id);
    assert_ne!(a, b);
    let (boite_a, _) = boite(&ent, a, cast_a.direction.user_id, "vie_scolaire", &[cast_a.cpe.user_id]).await;
    let (boite_b, _) = boite(&ent, b, cast_b.direction.user_id, "vie_scolaire", &[cast_b.cpe.user_id]).await;
    let oa = ouvrir_boite(&ent, a, cast_a.parent_lea.user_id, Role::Parent, boite_a, cast_a.eleve_lea_id, "A").await;
    ouvrir_boite(&ent, b, cast_b.parent_lea.user_id, Role::Parent, boite_b, cast_b.eleve_lea_id, "B").await;
    // Côté service : le membre de B ne lit rien de A, ne traite rien de A, ne voit pas la boîte de A.
    let lecture_croisee = ent.list_messages_for_actor(b, cast_b.cpe.user_id, role(Role::Cpe), oa.fil_id).await;
    assert!(
        matches!(lecture_croisee, Err(EntDbError::Forbidden) | Err(EntDbError::Domain(_))),
        "aucune lecture d'un fil d'un autre établissement"
    );
    assert!(ent.file_boite_for_actor(b, cast_b.cpe.user_id, role(Role::Cpe), boite_a, None, 50).await.is_err());
    assert!(ent.prendre_en_charge_for_actor(b, cast_b.cpe.user_id, role(Role::Cpe), oa.fil_id, boite_a).await.is_err());
    assert!(ent.ouvrir_fil_boite_for_actor(b, cast_b.parent_lea.user_id, role(Role::Parent), boite_a, cast_b.eleve_lea_id, "X", "y").await.is_err());
    // Côté base : rôle applicatif ni propriétaire, ni BYPASSRLS, en mode strict.
    let url = crate::url_guard::url_labo_ou_echec().unwrap();
    const ROLE: &str = "ent_rls_m3";
    sqlx::query(&format!(
        "DO $$ BEGIN CREATE ROLE {ROLE} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; \
         EXCEPTION WHEN duplicate_object THEN NULL; END $$"
    ))
    .execute(ent.pool())
    .await
    .unwrap();
    for sql in [
        format!("GRANT USAGE ON SCHEMA public TO {ROLE}"),
        format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ent_msg_boite, ent_msg_boite_mission, ent_msg_boite_horaire, \
             ent_msg_boite_designation, ent_msg_astreinte, ent_msg_traitement, ent_msg_etiquette, \
             ent_msg_traitement_etiquette, ent_msg_note_interne, ent_msg_transfert, ent_msg_telephone, \
             ent_msg_lot, ent_msg_lot_eleve, ent_msg_brouillon, ent_msg_participant, ent_msg_fil, ent_msg, \
             ent_mission, ent_agent, ent_personne TO {ROLE}"
        ),
        format!("GRANT SELECT ON ent_etablissement_parametre TO {ROLE}"),
        format!("ALTER ROLE {ROLE} SET app.rls_strict = 'on'"),
    ] {
        sqlx::query(&sql).execute(ent.pool()).await.unwrap();
    }
    let (bypass, sup): (bool, bool) = sqlx::query_as("SELECT rolbypassrls, rolsuper FROM pg_roles WHERE rolname = $1")
        .bind(ROLE)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(!bypass && !sup, "le rôle de test ne contourne pas la RLS");
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(ROLE).unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(u.as_str()).await.unwrap();
    for table in [
        "ent_msg_boite", "ent_msg_boite_mission", "ent_msg_traitement", "ent_msg_participant", "ent_msg_fil",
    ] {
        let mut t = crate::tenant_tx::TenantTx::begin_mode(&app, a, crate::tenant_tx::RlsMode::Strict).await.unwrap();
        let (chez_a, hors_a): (i64, i64) = sqlx::query_as(&format!(
            "SELECT count(*) FILTER (WHERE etablissement_id = $1), count(*) FILTER (WHERE etablissement_id <> $1) FROM {table}"
        ))
        .bind(a)
        .fetch_one(&mut *t)
        .await
        .unwrap();
        assert!(chez_a >= 1, "{table} : l'établissement courant voit ses lignes");
        assert_eq!(hors_a, 0, "{table} : aucune ligne d'un autre établissement en mode strict");
        t.commit().await.unwrap();
        let (sans,): (i64,) = sqlx::query_as(&format!("SELECT count(*) FROM {table}")).fetch_one(&app).await.unwrap();
        assert_eq!(sans, 0, "{table} : sans contexte d'établissement, rien en mode strict");
    }
    // L'oracle d'appartenance ne traverse pas : les membres de la boîte de B, relus depuis A, sont vides.
    let mut t = crate::tenant_tx::TenantTx::begin_mode(&app, a, crate::tenant_tx::RlsMode::Strict).await.unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_msg_boite_membres($1, $2)")
        .bind(b)
        .bind(boite_b)
        .fetch_one(&mut *t)
        .await
        .unwrap();
    assert_eq!(n, 0, "la fonction de membres ne révèle rien d'un autre établissement");
    // Une écriture croisée est refusée par la politique.
    let r = sqlx::query(
        "INSERT INTO ent_msg_note_interne (id, etablissement_id, fil_id, boite_id, auteur_personne_id, corps)
         SELECT gen_random_uuid(), $1, f.id, $2, pe.id, 'x'
           FROM ent_msg_fil f, ent_personne pe WHERE f.etablissement_id = $1 AND pe.etablissement_id = $1 LIMIT 1",
    )
    .bind(b)
    .bind(boite_b)
    .execute(&mut *t)
    .await;
    assert!(r.is_err() || r.unwrap().rows_affected() == 0, "écriture dans l'établissement B depuis le contexte A");
}
