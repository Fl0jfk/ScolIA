//! Socle 2 (l'élève), lot E7 : confidentialité renforcée (ELV-15), journal de consultation,
//! scellement des versions sensibles, photos (ELV-16, ELV-42), coordonnées propres (ELV-17).
//! Scénarios contre la base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, ils sont
//! ignorés. Aucune dépendance à l'heure réelle : les instants sont passés aux commandes
//! (`*_a`), les mesures commencent à une date passée fixe.

use super::*;
use crate::eleve_confidentialite::PoserConfidentialite;
use crate::eleve_coordonnee::AjouterCoordonneeEleve;
use crate::eleve_photo::ProposerPhoto;
use crate::identite::{CorrectionIdentite, UsageIdentite};
use crate::seed::LaboCast;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use domain_eleve::{CreateEleve, ModifIdentite};
use ent_auth::Role;
use uuid::Uuid;

/// Instant de référence : bien avant toute exécution (les mesures sont donc « en vigueur »
/// pour la base, qui juge à l'horloge réelle), et jamais l'horloge réelle pour les commandes.
fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 15, 10, 0, 0).unwrap()
}

fn plus(min: i64) -> DateTime<Utc> {
    t0() + chrono::Duration::minutes(min)
}

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (mut ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |_e| {}).await;
    ent.install_mail_dry_run();
    Some((ent, cast))
}

fn lettres(n: usize) -> String {
    Uuid::new_v4().as_bytes().iter().take(n).map(|b| (b'a' + b % 26) as char).collect()
}

async fn eleve(ent: &PgEnt, etab: Uuid, prenom: &str, classe: &str) -> domain_eleve::Eleve {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: format!("Prot{}", lettres(6)),
        prenom: prenom.into(),
        ine: None,
        sexe: Some("F".into()),
        date_naissance: NaiveDate::from_ymd_opt(2012, 3, 14),
        numero_badge: None,
        redoublant: false,
        classe: Some(classe.into()),
        regime: None,
        mef: None,
        site: None,
        date_entree: None,
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
}

fn user(cast: &LaboCast, r: Role) -> (Uuid, &'static str) {
    let p = match r {
        Role::Admin => &cast.admin,
        Role::Direction => &cast.direction,
        Role::Administratif => &cast.administratif,
        Role::Cpe => &cast.cpe,
        Role::Prof => &cast.prof,
        Role::Compta => &cast.compta,
        _ => unreachable!(),
    };
    (p.user_id, r.as_str())
}

fn pose(etab: Uuid, eleve: Uuid, acteur: Uuid, motif: &str, mesures: &[&str]) -> PoserConfidentialite {
    PoserConfidentialite {
        etablissement_id: etab,
        eleve_id: eleve,
        motif: motif.into(),
        mesures: mesures.iter().map(|m| (*m).to_string()).collect(),
        du: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        au: None,
        acteur,
    }
}

async fn poser(ent: &PgEnt, cast: &LaboCast, eleve: Uuid, motif: &str, mesures: &[&str]) {
    let (d, role) = user(cast, Role::Direction);
    ent.poser_confidentialite_a(role, pose(cast.etablissement_id, eleve, d, motif, mesures), t0())
        .await
        .unwrap();
}

fn forbidden<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

fn absent<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound)))
}

fn conflit<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::ConflitDossier(_)))
}

fn invalide<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::SaisieInvalide(_)))
}

// ───────────────────────────── ELV-15 : confidentialité renforcée ─────────────────────────────

/// Cycle complet : pose (motif scellé, événement et journal sans motif), double pose refusée,
/// modification (lever + poser), levée, double levée refusée ; seule la direction (G6) agit.
#[tokio::test]
async fn poser_modifier_lever_la_confidentialite() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Camille", "6A").await;
    let (dir, rdir) = user(&cast, Role::Direction);
    let (adm, radm) = user(&cast, Role::Administratif);
    let motif = format!("Jugement du 3 mai {}", lettres(8));

    // Le secrétariat (ni G6 ni droit de gestion) ne peut ni poser ni lever.
    let r = ent.poser_confidentialite_a(radm, pose(etab, e.id, adm, &motif, &[]), t0()).await;
    assert!(forbidden(&r), "{r:?}");
    let r = ent.lever_confidentialite_a(radm, etab, adm, e.id, "fin_de_mesure", t0()).await;
    assert!(forbidden(&r), "{r:?}");
    // Rien à lever : l'élève n'a pas de mesure.
    let r = ent.lever_confidentialite_a(rdir, etab, dir, e.id, "fin_de_mesure", t0()).await;
    assert!(absent(&r), "{r:?}");

    let m = ent
        .poser_confidentialite_a(rdir, pose(etab, e.id, dir, &motif, &["alerte_consultation"]), t0())
        .await
        .unwrap();
    // Effets minimaux toujours posés, mesure demandée en plus.
    assert_eq!(m.mesures, ["masquer_photo", "hors_annuaire", "alerte_consultation"]);
    // Le motif est scellé en base : jamais en clair.
    let (scelle,): (String,) =
        sqlx::query_as("SELECT motif_scelle FROM ent_eleve_confidentialite WHERE id = $1")
            .bind(m.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(scelle.starts_with("enc:v") && !scelle.contains(&motif), "motif en clair en base");
    // Ni l'événement ni le journal ne portent le motif ni les mesures.
    let payload: String = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.confidentialite_changee'",
    )
    .bind(e.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(payload.contains("pose") && !payload.contains(&motif) && !payload.contains("alerte"));
    let journal: Vec<String> = sqlx::query_scalar(
        "SELECT libelle || ' ' || payload::text FROM ent_journal_modif WHERE eleve_id = $1 AND domaine = 'eleve_sensible'",
    )
    .bind(e.id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(journal.len(), 1);
    assert!(!journal[0].contains(&motif) && !journal[0].contains("alerte"));

    // Une seule mesure en vigueur à la fois.
    let r = ent.poser_confidentialite_a(rdir, pose(etab, e.id, dir, &motif, &[]), t0()).await;
    assert!(conflit(&r), "{r:?}");

    // Lecture par G6 : le motif est descellé ; le secrétariat n'en sait rien (404, pas 403).
    let l = ent.mesures_confidentialite_pour_acteur_a(rdir, etab, dir, e.id, plus(1)).await.unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].motif.as_deref(), Some(motif.as_str()));
    let r = ent.mesures_confidentialite_pour_acteur_a(radm, etab, adm, e.id, plus(1)).await;
    assert!(absent(&r), "{r:?}");

    // Modification : la précédente est levée (changement_de_mesure), la nouvelle posée ensemble.
    let m2 = ent
        .modifier_confidentialite_a(rdir, pose(etab, e.id, dir, "Nouvelle décision", &["adresse_protegee"]), plus(2))
        .await
        .unwrap();
    assert_eq!(m2.mesures, ["masquer_photo", "hors_annuaire", "adresse_protegee"]);
    let (leve, code): (Option<DateTime<Utc>>, Option<String>) =
        sqlx::query_as("SELECT leve_le, leve_motif_code FROM ent_eleve_confidentialite WHERE id = $1")
            .bind(m.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(leve, Some(plus(2)));
    assert_eq!(code.as_deref(), Some("changement_de_mesure"));
    let (ouvertes,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_eleve_confidentialite WHERE eleve_id = $1 AND leve_le IS NULL",
    )
    .bind(e.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(ouvertes, 1);

    // Levée : motif codé obligatoire et connu ; puis plus rien à lever.
    let r = ent.lever_confidentialite_a(rdir, etab, dir, e.id, "parce_que", plus(3)).await;
    assert!(invalide(&r), "{r:?}");
    assert_eq!(ent.lever_confidentialite_a(rdir, etab, dir, e.id, "fin_de_mesure", plus(3)).await.unwrap(), 1);
    let r = ent.lever_confidentialite_a(rdir, etab, dir, e.id, "fin_de_mesure", plus(4)).await;
    assert!(absent(&r), "{r:?}");
    // Rien ne se supprime : trois lignes d'historique au total.
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_confidentialite WHERE eleve_id = $1")
        .bind(e.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap_or((0,));
    assert!(n == 2, "deux mesures (la première levée, la seconde levée) : {n}");
    let r = sqlx::query("DELETE FROM ent_eleve_confidentialite WHERE eleve_id = $1")
        .bind(e.id)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "une mesure ne se supprime pas");
}

/// Saisies invalides : motif vide, mesure inconnue, période incohérente — rien n'est écrit.
#[tokio::test]
async fn saisies_invalides_et_cloisonnement() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Noa", "6A").await;
    let (dir, rdir) = user(&cast, Role::Direction);
    let r = ent.poser_confidentialite_a(rdir, pose(etab, e.id, dir, "   ", &[]), t0()).await;
    assert!(invalide(&r), "{r:?}");
    let r = ent.poser_confidentialite_a(rdir, pose(etab, e.id, dir, "motif", &["invisible"]), t0()).await;
    assert!(invalide(&r), "{r:?}");
    let mut p = pose(etab, e.id, dir, "motif", &[]);
    p.au = Some(NaiveDate::from_ymd_opt(2025, 12, 1).unwrap());
    let r = ent.poser_confidentialite_a(rdir, p, t0()).await;
    assert!(invalide(&r), "{r:?}");
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_confidentialite WHERE eleve_id = $1")
        .bind(e.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 0);
    // Élève absent, ou d'un autre établissement : 404, jamais un succès.
    let r = ent.poser_confidentialite_a(rdir, pose(etab, Uuid::new_v4(), dir, "motif", &[]), t0()).await;
    assert!(absent(&r), "{r:?}");
    let autre = Uuid::new_v4();
    let ailleurs = eleve(&ent, autre, "Ailleurs", "6A").await;
    let r = ent.poser_confidentialite_a(rdir, pose(etab, ailleurs.id, dir, "motif", &[]), t0()).await;
    assert!(absent(&r), "{r:?}");
    let r = ent.lever_confidentialite_a(rdir, etab, dir, ailleurs.id, "fin_de_mesure", t0()).await;
    assert!(absent(&r), "{r:?}");
    let r = ent.mesures_confidentialite_pour_acteur_a(rdir, etab, dir, ailleurs.id, t0()).await;
    assert!(absent(&r), "{r:?}");
}

/// R-E10 : l'agent en conflit d'intérêts avec l'élève ne décide pas à son sujet.
#[tokio::test]
async fn conflit_d_interet_interdit_la_decision() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Lou", "6A").await;
    let (dir, rdir) = user(&cast, Role::Direction);
    sqlx::query(
        "INSERT INTO ent_conflit_interet (id, etablissement_id, user_id, eleve_id, nature, origine, declare_par)
         VALUES (gen_random_uuid(), $1, $2, $3, 'proche', 'direction', $2)",
    )
    .bind(etab)
    .bind(dir)
    .bind(e.id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let r = ent.poser_confidentialite_a(rdir, pose(etab, e.id, dir, "motif", &[]), t0()).await;
    assert!(conflit(&r), "{r:?}");
    let r = ent.lever_confidentialite_a(rdir, etab, dir, e.id, "fin_de_mesure", t0()).await;
    assert!(conflit(&r), "{r:?}");
}

/// Un élève protégé est absent de toutes les lectures d'un tiers (ni G6 ni lien direct) : fiche,
/// identifiants, scolarité, historique, listes, trombinoscope ; la direction (G6) le voit ; la
/// mesure levée, il redevient visible.
#[tokio::test]
async fn l_eleve_protege_est_invisible_d_un_tiers() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Maé", "6A").await;
    let (dir, rdir) = user(&cast, Role::Direction);
    let tiers = [user(&cast, Role::Administratif), user(&cast, Role::Cpe)];
    // Contrôle positif : avant la mesure, tout le monde le voit.
    for (u, r) in tiers {
        assert!(ent.get_eleve_for_actor(etab, u, r, e.id).await.unwrap().is_some(), "{r} avant");
    }
    poser(&ent, &cast, e.id, "Enfant placé", &[]).await;
    for (u, r) in tiers {
        assert!(ent.get_eleve_for_actor(etab, u, r, e.id).await.unwrap().is_none(), "{r} : fiche");
        assert!(ent.get_eleve_vue_for_actor(etab, u, r, e.id).await.unwrap().is_none(), "{r} : vue");
        assert!(ent.search_eleves_pour_acteur(etab, u, r, &e.nom, 20).await.unwrap().is_empty(), "{r} : recherche");
        let l = ent.identifiants_pour_acteur(etab, u, r, e.id).await;
        assert!(forbidden(&l) || absent(&l), "{r} : identifiants {l:?}");
        let l = ent.scolarite_pour_acteur(etab, u, r, e.id).await;
        assert!(forbidden(&l) || absent(&l), "{r} : scolarité {l:?}");
        let l = ent.historique_identite_pour_acteur(etab, u, r, e.id).await;
        assert!(forbidden(&l) || absent(&l), "{r} : historique {l:?}");
        let liste = ent.vues_eleves_pour_acteur(etab, u, r, vec![e.clone()]).await.unwrap();
        assert!(liste.is_empty(), "{r} : liste");
        let trombi = ent.list_trombi_classe_for_actor(etab, u, r, "6A", NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()).await;
        if let Ok(t) = trombi {
            assert!(t.iter().all(|x| x.eleve_id != e.id), "{r} : trombinoscope");
        }
    }
    // La direction (G6) le voit partout.
    assert!(ent.get_eleve_for_actor(etab, dir, rdir, e.id).await.unwrap().is_some());
    assert!(ent.get_eleve_vue_for_actor(etab, dir, rdir, e.id).await.unwrap().is_some());
    assert!(ent.scolarite_pour_acteur(etab, dir, rdir, e.id).await.is_ok());
    assert_eq!(ent.eleves_proteges_pour_acteur(rdir, etab, dir, 100).await.unwrap().iter().filter(|m| m.eleve_id == e.id).count(), 1);
    // Le secrétariat ne peut pas lister les élèves protégés (réservé à G6).
    let (adm, radm) = user(&cast, Role::Administratif);
    assert!(forbidden(&ent.eleves_proteges_pour_acteur(radm, etab, adm, 100).await));
    // Levée : l'élève redevient visible pour tous.
    ent.lever_confidentialite_a(rdir, etab, dir, e.id, "fin_de_mesure", plus(5)).await.unwrap();
    for (u, r) in tiers {
        assert!(ent.get_eleve_for_actor(etab, u, r, e.id).await.unwrap().is_some(), "{r} après");
    }
}

/// Lien direct : un professeur dont l'élève protégé est dans « mes classes » le voit encore ;
/// un autre professeur ne le voit pas.
#[tokio::test]
async fn le_lien_direct_garde_l_acces() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (prof, rprof) = user(&cast, Role::Prof);
    let harry = cast.eleve_lea_id;
    let avant = ent.get_eleve_for_actor(etab, prof, rprof, harry).await.unwrap().is_some();
    assert!(avant, "contrôle positif : le professeur du labo suit Harry");
    poser(&ent, &cast, harry, "Mesure de protection", &[]).await;
    assert!(ent.get_eleve_for_actor(etab, prof, rprof, harry).await.unwrap().is_some(), "lien direct conservé");
    // Un tiers de la même équipe (secrétariat, établissement entier) ne le voit plus.
    let (adm, radm) = user(&cast, Role::Administratif);
    assert!(ent.get_eleve_for_actor(etab, adm, radm, harry).await.unwrap().is_none());
}

/// Journal de consultation : ouverture d'une fiche protégée, lecture de l'état civil, lecture
/// du motif ; lisible par `audit.lire` seulement ; alerte si la mesure l'exige.
#[tokio::test]
async fn consultation_journalisee_et_alerte() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (dir, rdir) = user(&cast, Role::Direction);
    let (adm, radm) = user(&cast, Role::Administratif);
    let (prof, rprof) = user(&cast, Role::Prof);
    let harry = cast.eleve_lea_id;
    // Lecture de l'état civil d'un élève ordinaire par le secrétariat : tracée.
    let autre = eleve(&ent, etab, "Rémi", "6A").await;
    ent.get_eleve_vue_for_actor(etab, adm, radm, autre.id).await.unwrap().unwrap();
    let c = ent.consultations_pour_acteur(rdir, etab, dir, autre.id, 50).await.unwrap();
    assert!(c.iter().any(|x| x.acteur == Some(adm) && x.action == "etat_civil"), "{c:?}");
    // Réservé à `audit.lire` ; élève d'un autre établissement : 404.
    assert!(forbidden(&ent.consultations_pour_acteur(rprof, etab, prof, autre.id, 50).await));
    assert!(absent(&ent.consultations_pour_acteur(rdir, etab, dir, Uuid::new_v4(), 50).await));

    // Mesure avec alerte : la consultation du professeur (lien direct) est tracée et alertée ;
    // celle de la direction (G6) est tracée sans alerte.
    poser(&ent, &cast, harry, "Protection", &["alerte_consultation"]).await;
    ent.get_eleve_vue_for_actor(etab, prof, rprof, harry).await.unwrap().unwrap();
    ent.get_eleve_vue_for_actor(etab, dir, rdir, harry).await.unwrap().unwrap();
    let c = ent.consultations_pour_acteur(rdir, etab, dir, harry, 50).await.unwrap();
    assert!(c.iter().any(|x| x.acteur == Some(prof) && x.action == "fiche_protegee"), "{c:?}");
    assert!(c.iter().any(|x| x.acteur == Some(dir) && x.action == "fiche_protegee"), "{c:?}");
    let alertes: Vec<String> = sqlx::query_scalar(
        "SELECT actor_user_id FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.consultation_protegee'",
    )
    .bind(harry)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(!alertes.is_empty() && alertes.iter().all(|a| *a == prof.to_string()), "alerte pour le non-G6 seulement : {alertes:?}");
    // La lecture du motif est tracée aussi.
    ent.mesures_confidentialite_pour_acteur_a(rdir, etab, dir, harry, plus(7)).await.unwrap();
    let c = ent.consultations_pour_acteur(rdir, etab, dir, harry, 50).await.unwrap();
    assert!(c.iter().any(|x| x.action == "sensible"), "{c:?}");
    // Rien de personnel dans le libellé ; une ligne du journal ne se modifie pas.
    assert!(c.iter().all(|x| !x.libelle.contains(&harry.to_string())));
    let r = sqlx::query("UPDATE ent_journal_modif SET libelle = 'x' WHERE id = $1")
        .bind(c[0].id)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "journal en ajout seul");
}

// ───────────────────────────── Scellement des versions sensibles ─────────────────────────────

/// Une version sensible devenue non courante est scellée : valeurs en clair vidées, une ligne
/// chiffrée par champ ; la direction (G6) les relit, le secrétariat ne voit pas la version ; la
/// base refuse toute autre modification.
#[tokio::test]
async fn les_versions_sensibles_non_courantes_sont_scellees() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Samantha", "6A").await;
    let (adm, radm) = user(&cast, Role::Administratif);
    let (dir, rdir) = user(&cast, Role::Direction);
    let usage = |p: &str| UsageIdentite {
        etablissement_id: etab,
        eleve_id: e.id,
        nom_usage: None,
        prenom_usage: Some(Some(p.into())),
        effet_du: None,
        motif_code: None,
        motif: None,
        piece_document_id: None,
        source: "saisie".into(),
        acteur: adm,
    };
    let v2 = ent.definir_usage(radm, usage("Sam")).await.unwrap().version;
    assert!(v2.sensible);
    // Courante : en clair (la fiche l'affiche).
    let (clair,): (Option<String>,) = sqlx::query_as("SELECT prenom_usage FROM ent_eleve_identite WHERE id = $1")
        .bind(v2.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(clair.as_deref(), Some("Sam"));
    // Une nouvelle version la rend non courante : elle est scellée dans la même transaction.
    let v3 = ent.definir_usage(radm, usage("Sammy")).await.unwrap().version;
    let (nom, prenom_u, prenoms, sexe): (String, Option<String>, Vec<String>, Option<String>) = sqlx::query_as(
        "SELECT nom_naissance, prenom_usage, prenoms, sexe FROM ent_eleve_identite WHERE id = $1",
    )
    .bind(v2.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(nom, "[scellé]");
    assert!(prenom_u.is_none() && sexe.is_none());
    assert_eq!(prenoms, ["[scellé]"]);
    let scellees: Vec<(String, String)> = sqlx::query_as(
        "SELECT champ, valeur_scellee FROM ent_eleve_identite_scellee WHERE version_id = $1 ORDER BY champ",
    )
    .bind(v2.id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let champs: Vec<&str> = scellees.iter().map(|(c, _)| c.as_str()).collect();
    assert!(champs.contains(&"prenom_usage") && champs.contains(&"nom_naissance") && champs.contains(&"prenoms"), "{champs:?}");
    for (_, v) in &scellees {
        assert!(v.starts_with("enc:v") && !v.contains("Sam") && !v.contains(&e.nom), "valeur en clair : {v}");
    }
    // La version courante reste en clair (la fiche et la recherche la lisent).
    let (cv,): (Option<String>,) = sqlx::query_as("SELECT prenom_usage FROM ent_eleve_identite WHERE id = $1")
        .bind(v3.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(cv.as_deref(), Some("Sammy"));
    // G6 relit l'ancienne valeur ; G5 sans G6 ne voit pas la version sensible.
    let h = ent.historique_identite_pour_acteur(etab, dir, rdir, e.id).await.unwrap();
    let v = h.iter().find(|x| x.id == v2.id).expect("version scellée dans l'historique G6");
    assert_eq!(v.prenom_usage.as_deref(), Some("Sam"));
    assert_eq!(v.nom_naissance, e.nom);
    let h = ent.historique_identite_pour_acteur(etab, adm, radm, e.id).await.unwrap();
    assert!(h.iter().all(|x| !x.sensible), "version sensible visible sans G6");
    // La base refuse toute modification d'une version, y compris une fausse levée de scellé.
    for (sql, cible) in [
        ("UPDATE ent_eleve_identite SET nom_naissance = 'X' WHERE id = $1", v3.id),
        ("UPDATE ent_eleve_identite SET nom_naissance = '[scellé]', prenoms = ARRAY['[scellé]'], prenom_usage = NULL WHERE id = $1", v3.id),
        ("UPDATE ent_eleve_identite SET prenom_usage = 'Sam' WHERE id = $1", v2.id),
        ("DELETE FROM ent_eleve_identite WHERE id = $1", v2.id),
        ("DELETE FROM ent_eleve_identite_scellee WHERE version_id = $1", v2.id),
        ("UPDATE ent_eleve_identite_scellee SET champ = 'sexe' WHERE version_id = $1", v2.id),
    ] {
        let r = sqlx::query(sql).bind(cible).execute(ent.pool_verif()).await;
        assert!(r.is_err(), "{sql}");
    }
    // La tâche quotidienne est idempotente : rien de plus à sceller pour cet élève.
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_eleve_identite WHERE eleve_id = $1 AND sensible AND nom_naissance <> '[scellé]'",
    )
    .bind(e.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1, "seule la version courante reste en clair");
    ent.sceller_identites_sensibles_anciennes().await.unwrap();
    let apres: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_eleve_identite_scellee WHERE version_id = $1")
        .bind(v2.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(apres, scellees.len() as i64);
}

/// Corriger une version dont la base est scellée reste possible : l'identité manipule les
/// valeurs descellées (aucun « [scellé] » ne remonte dans la fiche).
#[tokio::test]
async fn la_fiche_ne_voit_jamais_le_marqueur() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Aline", "6A").await;
    let (adm, radm) = user(&cast, Role::Administratif);
    let usage = |p: Option<&str>| UsageIdentite {
        etablissement_id: etab,
        eleve_id: e.id,
        nom_usage: None,
        prenom_usage: Some(p.map(String::from)),
        effet_du: None,
        motif_code: None,
        motif: None,
        piece_document_id: None,
        source: "saisie".into(),
        acteur: adm,
    };
    ent.definir_usage(radm, usage(Some("Aly"))).await.unwrap();
    ent.definir_usage(radm, usage(None)).await.unwrap();
    // Correction de la version courante : la base est la version courante (en clair), pas la scellée.
    let r = ent
        .corriger_identite(
            radm,
            CorrectionIdentite {
                etablissement_id: etab,
                eleve_id: e.id,
                version_id: None,
                modif: ModifIdentite { nom_usage: Some(Some("Dupuis".into())), ..Default::default() },
                motif_code: "faute_de_frappe".into(),
                motif: None,
                piece_document_id: None,
                source: "saisie".into(),
                acteur: adm,
            },
        )
        .await
        .unwrap();
    assert_eq!(r.version.nom_naissance, e.nom);
    assert_eq!(r.version.prenoms, ["Aline"]);
    let f = ent.get_eleve_for_actor(etab, adm, radm, e.id).await.unwrap().unwrap();
    assert!(!f.nom.contains("scellé") && !f.prenom.contains("scellé"));
}

// ───────────────────────────── ELV-16 : photos ─────────────────────────────

/// JPEG minimal mais COMPLET (gris 1×1) : la photo est nettoyée de ses métadonnées avant stockage (E9, F2),
/// un simple en-tête n'est plus une image.
pub(crate) const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x12, 0x34, 0xFF, 0xD9];

fn depot(etab: Uuid, eleve: Uuid, acteur: Uuid, valider: bool) -> ProposerPhoto {
    ProposerPhoto {
        etablissement_id: etab,
        eleve_id: eleve,
        acteur,
        content_type: "image/jpeg".into(),
        octets: JPEG.to_vec(),
        prise_le: None,
        valider_aussitot: valider,
        photographe: false,
    }
}

/// Photo : le personnel dépose et valide, la famille propose seulement ; une seule photo validée
/// (l'ancienne est retirée) ; la fiche suit ; retrait, rejet, transitions interdites ; l'objet
/// et le statut ne se modifient que par les commandes.
#[tokio::test]
async fn cycle_de_vie_d_une_photo() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let (prof, rprof) = user(&cast, Role::Prof);
    let harry = cast.eleve_lea_id;
    let (parent, rparent) = (cast.parent_lea.user_id, Role::Parent.as_str());

    // Le professeur n'a pas `eleves.photo.gerer`.
    let r = ent.proposer_photo_a(rprof, depot(etab, harry, prof, true), t0()).await;
    assert!(forbidden(&r), "{r:?}");
    // Fichier qui n'est pas une image : refusé, rien d'écrit.
    let mut faux = depot(etab, harry, adm, true);
    faux.octets = b"MZ\x90\x00 pas une image".to_vec();
    assert!(ent.proposer_photo_a(radm, faux, t0()).await.is_err());

    let p1 = ent.proposer_photo_a(radm, depot(etab, harry, adm, true), t0()).await.unwrap();
    assert_eq!(p1.statut, "validee");
    assert_eq!(p1.source, "etablissement");
    let (cle, pid): (Option<String>, Option<Uuid>) =
        sqlx::query_as("SELECT photo_key, photo_id FROM ent_eleve WHERE id = $1")
            .bind(harry)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(cle.is_some_and(|c| c.starts_with(&format!("{etab}/trombi/{harry}/"))));
    assert_eq!(pid, Some(p1.id));

    // Famille : proposée seulement (la fiche ne change pas), source déduite de l'acteur.
    let p2 = ent.proposer_photo_a(rparent, depot(etab, harry, parent, true), plus(1)).await.unwrap();
    assert_eq!((p2.statut.as_str(), p2.source.as_str()), ("proposee", "famille"));
    let (pid,): (Option<Uuid>,) = sqlx::query_as("SELECT photo_id FROM ent_eleve WHERE id = $1")
        .bind(harry)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(pid, Some(p1.id));
    // Un autre parent (lié à un autre élève) ne peut rien proposer : 404.
    let autre_parent = cast.parent_tom.user_id;
    let r = ent.proposer_photo_a(rparent, depot(etab, harry, autre_parent, false), plus(1)).await;
    assert!(absent(&r), "{r:?}");
    // File à valider.
    let file = ent.photos_a_valider_pour_acteur(radm, etab, adm, 50).await.unwrap();
    assert!(file.iter().any(|p| p.id == p2.id));
    // Validation : l'ancienne est retirée (remplacée), la fiche suit.
    let v = ent.valider_photo_a(radm, etab, adm, p2.id, plus(2)).await.unwrap();
    assert_eq!(v.statut, "validee");
    let h = ent.photos_eleve_pour_acteur(radm, etab, adm, harry).await.unwrap();
    let ancienne = h.iter().find(|p| p.id == p1.id).unwrap();
    assert_eq!((ancienne.statut.as_str(), ancienne.motif_code.as_deref()), ("retiree", Some("remplacee")));
    let (pid,): (Option<Uuid>,) = sqlx::query_as("SELECT photo_id FROM ent_eleve WHERE id = $1")
        .bind(harry)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(pid, Some(p2.id));
    // Revalider une photo déjà décidée : refusé.
    assert!(conflit(&ent.valider_photo_a(radm, etab, adm, p2.id, plus(3)).await));
    // Rejet d'une proposition, motif codé obligatoire ; puis retrait de la photo validée.
    let p3 = ent.proposer_photo_a(rparent, depot(etab, harry, parent, false), plus(4)).await.unwrap();
    assert!(invalide(&ent.rejeter_photo(radm, etab, adm, p3.id, "nimporte").await));
    assert_eq!(ent.rejeter_photo(radm, etab, adm, p3.id, "illisible").await.unwrap().statut, "rejetee");
    assert!(conflit(&ent.retirer_photo(radm, etab, adm, p3.id, "illisible").await));
    assert_eq!(ent.retirer_photo(radm, etab, adm, p2.id, "demande_famille").await.unwrap().statut, "retiree");
    let (cle,): (Option<String>,) = sqlx::query_as("SELECT photo_key FROM ent_eleve WHERE id = $1")
        .bind(harry)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(cle.is_none(), "la fiche n'a plus de photo");
    // Cible absente ou d'un autre établissement : 404.
    assert!(absent(&ent.valider_photo_a(radm, etab, adm, Uuid::new_v4(), t0()).await));
    let autre = Uuid::new_v4();
    let ailleurs = eleve(&ent, autre, "Ailleurs", "6A").await;
    assert!(absent(&ent.proposer_photo_a(radm, depot(etab, ailleurs.id, adm, true), t0()).await));
    // Les lignes ne se modifient pas hors transitions, ne se suppriment pas.
    for sql in [
        "UPDATE ent_eleve_photo SET object_key = 'x' WHERE id = $1",
        "UPDATE ent_eleve_photo SET statut = 'proposee' WHERE id = $1",
        "DELETE FROM ent_eleve_photo WHERE id = $1",
    ] {
        assert!(sqlx::query(sql).bind(p2.id).execute(ent.pool_verif()).await.is_err(), "{sql}");
    }
}

/// ELV-42 : refus de la photo administrative (famille) : photo retirée, propositions rejetées,
/// aucune nouvelle proposition, silhouette dans les listes ; révocable.
#[tokio::test]
async fn refus_de_la_photo_administrative() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let (parent, rparent) = (cast.parent_lea.user_id, Role::Parent.as_str());
    let harry = cast.eleve_lea_id;
    ent.proposer_photo_a(radm, depot(etab, harry, adm, true), t0()).await.unwrap();
    ent.enregistrer_refus_photo_a(rparent, etab, parent, harry, plus(1)).await.unwrap();
    let (cle,): (Option<String>,) = sqlx::query_as("SELECT photo_key FROM ent_eleve WHERE id = $1")
        .bind(harry)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(cle.is_none());
    assert!(conflit(&ent.proposer_photo_a(radm, depot(etab, harry, adm, true), plus(2)).await));
    assert!(conflit(&ent.enregistrer_refus_photo_a(rparent, etab, parent, harry, plus(2)).await));
    assert_eq!(ent.eleves_refus_photo(etab, &[harry]).await.unwrap(), [harry]);
    // L'élève sort de « photos manquantes » (aucune relance).
    assert!(!ent.eleves_sans_photo_pour_acteur(radm, etab, adm, 5000).await.unwrap().contains(&harry));
    // Révocable : la photo se repropose.
    ent.lever_refus_photo_a(rparent, etab, parent, harry, plus(3)).await.unwrap();
    assert!(absent(&ent.lever_refus_photo_a(rparent, etab, parent, harry, plus(4)).await));
    ent.proposer_photo_a(radm, depot(etab, harry, adm, true), plus(5)).await.unwrap();
}

/// Photo d'un élève protégé : invisible d'un tiers, y compris pour le déposer ou la valider.
#[tokio::test]
async fn photo_d_un_eleve_protege() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let e = eleve(&ent, etab, "Zoé", "6A").await;
    let p = ent.proposer_photo_a(radm, depot(etab, e.id, adm, false), t0()).await.unwrap();
    poser(&ent, &cast, e.id, "Protection", &[]).await;
    assert!(absent(&ent.proposer_photo_a(radm, depot(etab, e.id, adm, true), plus(1)).await));
    assert!(absent(&ent.valider_photo_a(radm, etab, adm, p.id, plus(1)).await));
    assert!(absent(&ent.photos_eleve_pour_acteur(radm, etab, adm, e.id).await));
    assert!(ent.photos_a_valider_pour_acteur(radm, etab, adm, 500).await.unwrap().iter().all(|x| x.id != p.id));
    // La direction (G6) décide.
    let (dir, rdir) = user(&cast, Role::Direction);
    assert_eq!(ent.valider_photo_a(rdir, etab, dir, p.id, plus(2)).await.unwrap().statut, "validee");
}

// ───────────────────────────── ELV-17 : coordonnées propres ─────────────────────────────

fn coord(etab: Uuid, eleve: Uuid, acteur: Uuid, type_: &str, valeur: &str, usages: &[&str]) -> AjouterCoordonneeEleve {
    AjouterCoordonneeEleve {
        etablissement_id: etab,
        eleve_id: eleve,
        acteur,
        type_: type_.into(),
        valeur: valeur.into(),
        usages: usages.iter().map(|u| (*u).to_string()).collect(),
    }
}

fn code_email(ent: &PgEnt, to: &str) -> String {
    let (_, _, corps) = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .rev()
        .find(|(a, sujet, _)| a == to && sujet.contains("code de vérification"))
        .expect("code envoyé par e-mail");
    let i = corps.find("est : ").expect("code") + "est : ".len();
    corps[i..i + 6].to_string()
}

fn code_sms(ent: &PgEnt, to: &str) -> String {
    let r = ent.sms().receipts().into_iter().rev().find(|r| r.to == to).expect("code envoyé par SMS");
    let i = r.body.find(": ").expect("code") + 2;
    r.body[i..i + 6].to_string()
}

/// Coordonnées : ajout par le secrétariat, vérification par code (empreinte seule, cinq essais,
/// expiration), usages réservés aux coordonnées vérifiées, retrait ; plafond par type ; doublons.
#[tokio::test]
async fn coordonnees_de_l_eleve_et_verification_par_code() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let e = eleve(&ent, etab, "Inès", "6A").await;
    let mail = format!("ines.{}@exemple.test", lettres(6));

    let c = ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, "email", &format!("  {} ", mail.to_uppercase()), &["notifications"]), t0()).await.unwrap();
    assert_eq!((c.statut.as_str(), c.source.as_str()), ("active", "etablissement"));
    assert_eq!(c.valeur, mail, "normalisée en minuscules");
    assert!(c.verifiee_le.is_none());
    // Doublon, valeur mal formée, type inconnu, usage incompatible.
    assert!(conflit(&ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, "email", &mail, &[]), t0()).await));
    assert!(invalide(&ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, "email", "pas-une-adresse", &[]), t0()).await));
    assert!(invalide(&ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, "fixe", "0102030405", &[]), t0()).await));
    assert!(invalide(&ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, "mobile", "0102030405", &[]), t0()).await), "un fixe n'est pas un mobile");
    assert!(invalide(&ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, "mobile", "0612345678", &["compte"]), t0()).await));
    // Non vérifiée : inutilisable pour un envoi.
    assert!(ent.coordonnees_eleve_pour_usage(etab, e.id, "notifications").await.unwrap().is_empty());
    // Usages « compte » et « urgence » : seulement après vérification.
    assert!(conflit(&ent.modifier_usages_coordonnee_eleve(radm, etab, adm, c.id, &["urgence".into()]).await));

    // Vérification : mauvais code (erreur unique), bon code, usage unique.
    ent.demander_verification_coordonnee_eleve_a(radm, etab, adm, c.id, plus(1)).await.unwrap();
    let code = code_email(&ent, &mail);
    let (hash,): (String,) = sqlx::query_as("SELECT code_hash FROM ent_eleve_coordonnee_verification WHERE coordonnee_id = $1 AND statut = 'envoye'")
        .bind(c.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(hash != code && !hash.contains(&code), "l'empreinte seule est en base");
    let faux = if code == "000000" { "111111" } else { "000000" };
    assert!(invalide(&ent.verifier_coordonnee_eleve_a(radm, etab, adm, c.id, faux, plus(2)).await));
    assert!(invalide(&ent.verifier_coordonnee_eleve_a(radm, etab, adm, c.id, "12", plus(2)).await));
    let ok = ent.verifier_coordonnee_eleve_a(radm, etab, adm, c.id, &code, plus(3)).await.unwrap();
    assert_eq!(ok.verifiee_le, Some(plus(3)));
    assert!(invalide(&ent.verifier_coordonnee_eleve_a(radm, etab, adm, c.id, &code, plus(4)).await), "usage unique");
    assert_eq!(ent.coordonnees_eleve_pour_usage(etab, e.id, "notifications").await.unwrap(), [("email".to_string(), mail.clone())]);
    // M2 : le personnel ne peut pas poser l'usage « compte » à la place de l'élève.
    assert!(forbidden(&ent.modifier_usages_coordonnee_eleve(radm, etab, adm, c.id, &["urgence".into(), "compte".into()]).await));
    let u = ent.modifier_usages_coordonnee_eleve(radm, etab, adm, c.id, &["urgence".into()]).await.unwrap();
    assert_eq!(u.usages, ["urgence"]);
    assert!(ent.coordonnees_eleve_pour_usage(etab, e.id, "notifications").await.unwrap().is_empty(), "usage retiré");

    // Expiration : un code reçu n'est plus bon vingt minutes plus tard.
    let m = format!("lea.{}@exemple.test", lettres(6));
    let c2 = ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, "email", &m, &[]), t0()).await.unwrap();
    ent.demander_verification_coordonnee_eleve_a(radm, etab, adm, c2.id, plus(10)).await.unwrap();
    let code2 = code_email(&ent, &m);
    assert!(invalide(&ent.verifier_coordonnee_eleve_a(radm, etab, adm, c2.id, &code2, plus(10 + 20)).await), "expiré");
    // Cinq essais manqués épuisent le code ; trois demandes par heure au plus.
    ent.demander_verification_coordonnee_eleve_a(radm, etab, adm, c2.id, plus(40)).await.unwrap();
    let code3 = code_email(&ent, &m);
    let faux = if code3 == "000000" { "111111" } else { "000000" };
    for i in 0..5 {
        assert!(invalide(&ent.verifier_coordonnee_eleve_a(radm, etab, adm, c2.id, faux, plus(41 + i)).await));
    }
    assert!(invalide(&ent.verifier_coordonnee_eleve_a(radm, etab, adm, c2.id, &code3, plus(47)).await), "code épuisé");
    assert!(ent.demander_verification_coordonnee_eleve_a(radm, etab, adm, c2.id, plus(48)).await.is_ok());
    assert!(conflit(&ent.demander_verification_coordonnee_eleve_a(radm, etab, adm, c2.id, plus(49)).await), "trois codes par heure");
    // Plafond par type (deux vivantes) ; retrait puis ajout.
    assert!(conflit(&ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, "email", &format!("x.{}@exemple.test", lettres(6)), &[]), t0()).await));
    let r = ent.retirer_coordonnee_eleve_a(radm, etab, adm, c2.id, "inconnu", plus(60)).await;
    assert!(invalide(&r), "{r:?}");
    assert_eq!(ent.retirer_coordonnee_eleve_a(radm, etab, adm, c2.id, "erreur_de_saisie", plus(60)).await.unwrap().statut, "retiree");
    assert!(conflit(&ent.retirer_coordonnee_eleve_a(radm, etab, adm, c2.id, "erreur_de_saisie", plus(61)).await));
    ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, "email", &format!("y.{}@exemple.test", lettres(6)), &[]), t0()).await.unwrap();
    // Un mobile, vérifié par SMS.
    let c3 = ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, "mobile", "06 12 34 56 78", &["urgence"]), t0()).await.unwrap();
    assert_eq!(c3.valeur, "+33612345678");
    ent.demander_verification_coordonnee_eleve_a(radm, etab, adm, c3.id, plus(70)).await.unwrap();
    let code4 = code_sms(&ent, "+33612345678");
    ent.verifier_coordonnee_eleve_a(radm, etab, adm, c3.id, &code4, plus(71)).await.unwrap();
    // L'événement et le journal ne contiennent jamais la valeur.
    let evt: Vec<String> = sqlx::query_scalar("SELECT payload::text FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.coordonnees_modifiees'")
        .bind(e.id)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap();
    assert!(!evt.is_empty() && evt.iter().all(|p| !p.contains("@exemple") && !p.contains("+336")));
    let j: Vec<String> = sqlx::query_scalar("SELECT libelle || payload::text FROM ent_journal_modif WHERE eleve_id = $1 AND domaine = 'eleve_coordonnee'")
        .bind(e.id)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap();
    assert!(!j.is_empty() && j.iter().all(|p| !p.contains("@exemple") && !p.contains("+336")));
    // La lecture par le secrétariat est tracée.
    let l = ent.coordonnees_eleve_pour_acteur_a(radm, etab, adm, e.id, plus(80)).await.unwrap();
    assert!(l.len() >= 4);
    let (dir, rdir) = user(&cast, Role::Direction);
    let c = ent.consultations_pour_acteur(rdir, etab, dir, e.id, 50).await.unwrap();
    assert!(c.iter().any(|x| x.action == "coordonnees"));
}

/// Famille : elle propose (le secrétariat décide) tant que l'élève est mineur ; l'élève gère les
/// siennes ; un élève majeur est hors de portée de la famille ; cloisonnement.
#[tokio::test]
async fn la_famille_propose_l_eleve_gere_le_majeur_echappe_aux_parents() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let (parent, rparent) = (cast.parent_lea.user_id, Role::Parent.as_str());
    let harry = cast.eleve_lea_id;
    let mail = format!("harry.{}@exemple.test", lettres(6));
    // Le parent propose : proposée, source famille, à décider.
    let p = ent.ajouter_coordonnee_eleve_a(rparent, coord(etab, harry, parent, "email", &mail, &["notifications"]), t0()).await.unwrap();
    assert_eq!((p.statut.as_str(), p.source.as_str()), ("proposee", "famille"));
    assert!(ent.coordonnees_eleve_pour_usage(etab, harry, "notifications").await.unwrap().is_empty());
    // Un autre parent (autre élève) : 404. La famille ne décide pas, ne vérifie pas.
    assert!(absent(&ent.ajouter_coordonnee_eleve_a(rparent, coord(etab, harry, cast.parent_tom.user_id, "email", "a@b.test", &[]), t0()).await));
    assert!(forbidden(&ent.decider_coordonnee_eleve_a(rparent, etab, parent, p.id, true, None, plus(1)).await));
    // Le secrétariat refuse sans motif : invalide ; accepte : active, à vérifier.
    assert!(invalide(&ent.decider_coordonnee_eleve_a(radm, etab, adm, p.id, false, None, plus(1)).await));
    let a = ent.decider_coordonnee_eleve_a(radm, etab, adm, p.id, true, None, plus(2)).await.unwrap();
    assert_eq!(a.statut, "active");
    assert!(conflit(&ent.decider_coordonnee_eleve_a(radm, etab, adm, p.id, true, None, plus(3)).await));
    assert!(forbidden(&ent.demander_verification_coordonnee_eleve_a(rparent, etab, parent, p.id, plus(3)).await));
    // Le parent voit les coordonnées de son enfant mineur (proposées ou actives seulement).
    let l = ent.coordonnees_eleve_pour_acteur_a(rparent, etab, parent, harry, plus(4)).await.unwrap();
    assert!(l.iter().all(|c| c.statut == "proposee" || c.statut == "active"));
    // Une proposition retirée par la famille devient refusée.
    let m2 = format!("h2.{}@exemple.test", lettres(6));
    let p2 = ent.ajouter_coordonnee_eleve_a(rparent, coord(etab, harry, parent, "email", &m2, &[]), t0()).await.unwrap();
    assert_eq!(ent.retirer_coordonnee_eleve_a(rparent, etab, parent, p2.id, "erreur_de_saisie", plus(5)).await.unwrap().statut, "refusee");

    // Majorité : une fois majeur (date de naissance corrigée), la famille n'a plus aucune prise
    // (404) ; le secrétariat garde la main.
    let majeur = eleve(&ent, etab, "Maëlle", "Tle").await;
    ent.corriger_identite(
        radm,
        CorrectionIdentite {
            etablissement_id: etab,
            eleve_id: harry,
            version_id: None,
            modif: ModifIdentite { date_naissance: Some(Some(NaiveDate::from_ymd_opt(2000, 5, 17).unwrap())), ..Default::default() },
            motif_code: "d_apres_piece".into(),
            motif: None,
            piece_document_id: None,
            source: "saisie".into(),
            acteur: adm,
        },
    )
    .await
    .unwrap();
    assert!(absent(&ent.ajouter_coordonnee_eleve_a(rparent, coord(etab, harry, parent, "email", "m@b.test", &[]), t0()).await));
    assert!(absent(&ent.coordonnees_eleve_pour_acteur_a(rparent, etab, parent, harry, plus(6)).await));
    assert!(absent(&ent.retirer_coordonnee_eleve_a(rparent, etab, parent, a.id, "demande_famille", plus(6)).await));
    assert!(absent(&ent.proposer_photo_a(rparent, depot(etab, harry, parent, false), plus(6)).await));
    assert!(ent.ajouter_coordonnee_eleve_a(radm, coord(etab, harry, adm, "email", &format!("g.{}@exemple.test", lettres(6)), &[]), plus(7)).await.is_ok());
    // Élève lié à son compte : il ajoute la sienne (active, à vérifier) ; un autre élève ne le peut pas.
    let (compte, role_eleve) = (crate::tests_messagerie_m1::nouvel_utilisateur(&ent, etab, "eleve", "Elève").await, Role::Eleve.as_str());
    sqlx::query("INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id) VALUES ($1, $2, $3)")
        .bind(etab)
        .bind(majeur.id)
        .bind(compte)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let m3 = format!("maelle.{}@exemple.test", lettres(6));
    let c = ent.ajouter_coordonnee_eleve_a(role_eleve, coord(etab, majeur.id, compte, "email", &m3, &[]), t0()).await.unwrap();
    assert_eq!((c.statut.as_str(), c.source.as_str()), ("active", "eleve"));
    assert!(absent(&ent.ajouter_coordonnee_eleve_a(role_eleve, coord(etab, harry, compte, "email", "z@b.test", &[]), t0()).await));
    // Cible d'un autre établissement : 404.
    let autre = eleve(&ent, Uuid::new_v4(), "Ailleurs", "6A").await;
    assert!(absent(&ent.ajouter_coordonnee_eleve_a(radm, coord(etab, autre.id, adm, "email", "q@b.test", &[]), t0()).await));
    assert!(absent(&ent.retirer_coordonnee_eleve_a(radm, etab, adm, Uuid::new_v4(), "autre", t0()).await));
}
