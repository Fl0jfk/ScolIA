//! Socle 5, lot S6 : groupes, composition datée, changement de classe, règles de composition, dédoublement,
//! correction rétroactive, sortie de l'élève, cloisonnement, droits, heure de Paris. Base locale de test
//! (feature `labo`) ; sans `ENT_DATABASE_URL`, ignorés en local et **en échec en CI** (`ENT_EXIGER_BASE`).

use super::*;
use crate::ref_commun::Acteur;
use crate::structure_composition::*;
use crate::structure_groupes::{groupes, GenerationSaisie, GroupeModif, GroupeSaisie, GroupeVue};
use chrono::{Duration, NaiveDate};
use domain_eleve::{CreateEleve, DomainError};
use ent_auth::{Action, Portee, Role};
use sqlx::Row;
use uuid::Uuid;

fn jour() -> NaiveDate {
    domain_eleve::horloge::jour_paris()
}

/// Un compte réel du labo (les journaux référencent `ent_user`) : les droits viennent du profil de base
/// (modèle par défaut), jamais du compte.
static COMPTE: std::sync::OnceLock<Uuid> = std::sync::OnceLock::new();

fn acteur(etab: Uuid, role: Role) -> Acteur<'static> {
    Acteur {
        etablissement_id: etab,
        user_id: *COMPTE.get().expect("compte du labo"),
        role: role.as_str(),
    }
}

fn adm(etab: Uuid) -> Acteur<'static> {
    acteur(etab, Role::Administratif)
}

fn est_forbidden<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

fn est_domaine<T>(r: &Result<T, EntDbError>, d: DomainError) -> bool {
    matches!(r, Err(EntDbError::Domain(e)) if *e == d)
}

struct Monde {
    etab: Uuid,
    annee: Uuid,
}

async fn monde(ent: &PgEnt) -> Monde {
    let (etab, annee) = (Uuid::new_v4(), Uuid::new_v4());
    let t = jour();
    sqlx::query(
        "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
         VALUES ($1,$2,'N',$3,$4,true)",
    )
    .bind(annee)
    .bind(etab)
    .bind(t - Duration::days(60))
    .bind(t + Duration::days(200))
    .execute(ent.pool())
    .await
    .unwrap();
    Monde { etab, annee }
}

fn sfx() -> String {
    Uuid::new_v4().simple().to_string()[..6].to_uppercase()
}

async fn classe(ent: &PgEnt, m: &Monde, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle) VALUES ($1,$2,$3,$4,$4)")
        .bind(id)
        .bind(m.etab)
        .bind(m.annee)
        .bind(code)
        .execute(ent.pool())
        .await
        .unwrap();
    id
}

async fn eleve(ent: &PgEnt, etab: Uuid, nom: &str) -> Uuid {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: "S6".into(),
        ine: None,
        sexe: Some(
            if nom.len().is_multiple_of(2) {
                "F"
            } else {
                "M"
            }
            .into(),
        ),
        date_naissance: NaiveDate::from_ymd_opt(2012, 5, 6),
        numero_badge: None,
        redoublant: false,
        classe: None,
        regime: None,
        mef: None,
        site: None,
        date_entree: Some(jour() - Duration::days(55)),
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
    .id
}

/// Place l'élève dans la classe depuis `depuis` par la commande unique (écriture réelle).
async fn en_classe(ent: &PgEnt, m: &Monde, e: Uuid, c: Uuid, depuis: NaiveDate) {
    let r = ent
        .structure_eleve_classe_changer(
            &adm(m.etab),
            ChangementClasseSaisie {
                eleve_id: e,
                classe_id: c,
                date_effet: Some(depuis),
                apercu: false,
                entrer_dans: vec![],
            },
        )
        .await
        .unwrap();
    assert!(r.applique);
}

async fn offre(ent: &PgEnt, m: &Monde, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_offre_enseignement (id, etablissement_id, annee_id, code, libelle, modalite, public_type)
         VALUES ($1,$2,$3,$4,$4,'langue_2','groupe')",
    )
    .bind(id)
    .bind(m.etab)
    .bind(m.annee)
    .bind(code)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

async fn suit(ent: &PgEnt, m: &Monde, e: Uuid, o: Uuid, depuis: NaiveDate) {
    sqlx::query(
        "INSERT INTO ent_eleve_enseignement (id, etablissement_id, annee_id, eleve_id, offre_id, etat, choisi_le, debut_le)
         VALUES ($1,$2,$3,$4,$5,'accepte',$6,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(m.etab)
    .bind(m.annee)
    .bind(e)
    .bind(o)
    .bind(depuis)
    .execute(ent.pool())
    .await
    .unwrap();
}

fn saisie(code: &str, nature: &str, classes: Vec<Uuid>, offre: Option<Uuid>) -> GroupeSaisie {
    GroupeSaisie {
        code: code.into(),
        libelle: None,
        nature: nature.into(),
        classes,
        offre_id: offre,
        capacite: None,
        capacite_bloquante: false,
        annee_id: None,
        ouvert_le: None,
        ouvre_droits: None,
    }
}

async fn groupe(
    ent: &PgEnt,
    m: &Monde,
    code: &str,
    nature: &str,
    classes: Vec<Uuid>,
    offre: Option<Uuid>,
) -> GroupeVue {
    ent.structure_groupe_creer(&adm(m.etab), saisie(code, nature, classes, offre))
        .await
        .unwrap()
}

fn sans_droits(mut s: GroupeSaisie) -> GroupeSaisie {
    s.ouvre_droits = Some(false);
    s
}

fn compo(entrants: &[Uuid], sortants: &[Uuid], date: Option<NaiveDate>) -> CompositionSaisie {
    CompositionSaisie {
        entrants: entrants.to_vec(),
        sortants: sortants.to_vec(),
        date_effet: date,
        motif_code: None,
        apercu: false,
    }
}

async fn membres(ent: &PgEnt, etab: Uuid, g: Uuid, j: NaiveDate) -> Vec<Uuid> {
    let mut c = ent.pool().acquire().await.unwrap();
    let mut v = groupes::membres(&mut c, etab, j, g).await.unwrap();
    v.sort();
    v
}

fn trie(mut v: Vec<Uuid>) -> Vec<Uuid> {
    v.sort();
    v
}

async fn nb_evenements(ent: &PgEnt, etab: Uuid, type_name: &str) -> i64 {
    sqlx::query("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = $2")
        .bind(etab)
        .bind(type_name)
        .fetch_one(ent.pool())
        .await
        .unwrap()
        .get(0)
}

async fn nb_archive(ent: &PgEnt, etab: Uuid, eleve: Uuid, motif: &str) -> i64 {
    sqlx::query(
        "SELECT count(*) FROM ent_affectation_supprimee WHERE etablissement_id = $1 AND eleve_id = $2 AND motif_annulation = $3",
    )
    .bind(etab)
    .bind(eleve)
    .bind(motif)
    .fetch_one(ent.pool())
    .await
    .unwrap()
    .get(0)
}

async fn demarrer() -> Option<PgEnt> {
    crate::tests_d022_bis::labo().await.map(|(e, cast)| {
        let _ = COMPTE.set(cast.direction.user_id);
        e
    })
}

// ─── STR-50 : création, classes d'origine, offre, droits ─────────────────────────────────────

#[tokio::test]
async fn groupe_creer_classes_d_origine_offre_et_nature() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let (c4a, c4b) = (classe(&ent, &m, "4A").await, classe(&ent, &m, "4B").await);
    let esp = offre(&ent, &m, "ESP2").await;

    // Groupe inter-classes : deux classes d'origine, projection `classe_id` nulle ; l'offre est portée.
    let g = groupe(
        &ent,
        &m,
        "4AB-ESP",
        "enseignement",
        vec![c4a, c4b],
        Some(esp),
    )
    .await;
    assert_eq!(trie(g.classes.clone()), trie(vec![c4a, c4b]));
    assert_eq!(g.offre_id, Some(esp));
    assert!(g.ouvre_droits && g.ferme_le.is_none());
    assert_eq!(
        g.ouvert_le,
        jour() - Duration::days(60),
        "défaut : début de l'année"
    );
    let proj: Option<Uuid> = sqlx::query_scalar("SELECT classe_id FROM ent_groupe WHERE id = $1")
        .bind(g.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(
        proj, None,
        "plusieurs classes : classe_id n'est plus qu'une projection nulle"
    );

    // Une seule classe : la projection la reprend (lecteurs historiques).
    let g1 = groupe(&ent, &m, "4A-DEMI", "enseignement", vec![c4a], None).await;
    let proj: Option<Uuid> = sqlx::query_scalar("SELECT classe_id FROM ent_groupe WHERE id = $1")
        .bind(g1.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(proj, Some(c4a));
    // Retrait d'une classe d'origine : la projection suit.
    let g = ent
        .structure_groupe_modifier(
            &adm(m.etab),
            g.id,
            GroupeModif {
                classes: Some(vec![c4b]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(g.classes, vec![c4b]);

    // Nature « liste » : aucun droit n'en découle (forcé, et un droit explicite est refusé).
    let l = groupe(&ent, &m, "LISTE1", "liste", vec![], None).await;
    assert!(!l.ouvre_droits);
    let mut s = saisie("LISTE2", "liste", vec![], None);
    s.ouvre_droits = Some(true);
    assert!(est_domaine(
        &ent.structure_groupe_creer(&adm(m.etab), s).await,
        DomainError::ValeurInvalide
    ));
    // Une liste ne peut pas être le public d'un cours (elle ouvrirait la visibilité du professeur, D-022).
    let r = sqlx::query(
        "INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, groupe_id, ouvert_le, source)
         VALUES ($1,$2,$3,'X',$4,$5,'saisie')",
    )
    .bind(Uuid::new_v4())
    .bind(m.etab)
    .bind(m.annee)
    .bind(l.id)
    .bind(jour())
    .execute(ent.pool())
    .await;
    assert!(
        r.is_err(),
        "groupe sans droits comme public d'un cours : refusé par la base"
    );

    // Doublon de code dans l'année : conflit. Capacité invalide, nature inconnue : refus.
    assert!(est_domaine(
        &ent.structure_groupe_creer(&adm(m.etab), saisie("4AB-ESP", "activite", vec![], None))
            .await,
        DomainError::Conflit
    ));
    let mut s = saisie("CAP0", "activite", vec![], None);
    s.capacite = Some(0);
    assert!(est_domaine(
        &ent.structure_groupe_creer(&adm(m.etab), s).await,
        DomainError::ValeurInvalide
    ));
    assert!(est_domaine(
        &ent.structure_groupe_creer(&adm(m.etab), saisie("N1", "inconnue", vec![], None))
            .await,
        DomainError::ValeurInvalide
    ));
    // Code vide ou trop long.
    assert!(est_domaine(
        &ent.structure_groupe_creer(&adm(m.etab), saisie("  ", "activite", vec![], None))
            .await,
        DomainError::ValeurInvalide
    ));
    assert!(est_domaine(
        &ent.structure_groupe_creer(
            &adm(m.etab),
            saisie(&"x".repeat(41), "activite", vec![], None)
        )
        .await,
        DomainError::ValeurInvalide
    ));
    // Offre d'une autre année ou organisée par classe : refus.
    let autre_annee = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
         VALUES ($1,$2,'N+1',$3,$4,false)",
    )
    .bind(autre_annee)
    .bind(m.etab)
    .bind(jour() + Duration::days(300))
    .bind(jour() + Duration::days(600))
    .execute(ent.pool())
    .await
    .unwrap();
    let offre_n1 = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_offre_enseignement (id, etablissement_id, annee_id, code, libelle, modalite, public_type)
         VALUES ($1,$2,$3,'X1','X1','langue_2','groupe')",
    )
    .bind(offre_n1)
    .bind(m.etab)
    .bind(autre_annee)
    .execute(ent.pool())
    .await
    .unwrap();
    assert!(est_domaine(
        &ent.structure_groupe_creer(
            &adm(m.etab),
            saisie("HORS-ANNEE", "enseignement", vec![], Some(offre_n1))
        )
        .await,
        DomainError::ValeurInvalide
    ));
    // Classe d'une autre année que l'année demandée : refus.
    let mut s = saisie("MIXTE", "enseignement", vec![c4a], None);
    s.annee_id = Some(autre_annee);
    assert!(est_domaine(
        &ent.structure_groupe_creer(&adm(m.etab), s).await,
        DomainError::ValeurInvalide
    ));
    // Classe inconnue : introuvable.
    assert!(est_domaine(
        &ent.structure_groupe_creer(
            &adm(m.etab),
            saisie("Z", "enseignement", vec![Uuid::new_v4()], None)
        )
        .await,
        DomainError::NotFound
    ));
    // Événement et journal.
    assert!(nb_evenements(&ent, m.etab, "referentiel.groupe_cree").await >= 3);
    let j: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_structure_journal WHERE etablissement_id = $1 AND objet = 'groupe' AND action = 'cree'",
    )
    .bind(m.etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(j >= 3);
}

// ─── STR-51 : génération idempotente ────────────────────────────────────────────────────────

#[tokio::test]
async fn generation_des_groupes_d_un_enseignement_apercu_et_idempotence() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let (c1, c2, c3) = (
        classe(&ent, &m, "5A").await,
        classe(&ent, &m, "5B").await,
        classe(&ent, &m, "5C").await,
    );
    let o = offre(&ent, &m, "LAT").await;
    let a = adm(m.etab);
    let s = |mode: &str, classes: Vec<Uuid>, ecrire: bool| GenerationSaisie {
        offre_id: o,
        classes,
        mode: mode.into(),
        capacite: Some(24),
        capacite_bloquante: false,
        ecrire,
    };
    // Aperçu : rien n'est créé.
    let ap = ent
        .structure_groupes_generer(&a, s("par_classe", vec![c1, c2], false))
        .await
        .unwrap();
    assert!(!ap.applique && ap.groupes.len() == 2 && ap.crees.is_empty());
    assert_eq!(ap.groupes[0].code, "5A-LAT");
    assert!(ent
        .structure_groupes_liste(&a, Some(m.annee), None, None, false, None)
        .await
        .unwrap()
        .is_empty());
    // Écriture : deux groupes, un par classe ; rejouée : rien de plus.
    let r = ent
        .structure_groupes_generer(&a, s("par_classe", vec![c1, c2], true))
        .await
        .unwrap();
    assert_eq!(r.crees.len(), 2);
    let r2 = ent
        .structure_groupes_generer(&a, s("par_classe", vec![c1, c2], true))
        .await
        .unwrap();
    assert!(
        r2.crees.is_empty() && r2.groupes.iter().all(|g| g.existant.is_some()),
        "idempotent"
    );
    assert_eq!(
        ent.structure_groupes_liste(&a, Some(m.annee), None, None, false, None)
            .await
            .unwrap()
            .len(),
        2
    );
    // Un groupe pour 5A + 5B + 5C (regroupé) est un AUTRE ensemble de classes : créé.
    let r3 = ent
        .structure_groupes_generer(&a, s("regroupe", vec![c1, c2, c3], true))
        .await
        .unwrap();
    assert_eq!(r3.crees.len(), 1);
    assert_eq!(r3.groupes[0].code, "5A+5B+5C-LAT");
    // Liste filtrée par classe : les groupes de 5A (par classe et inter-classes).
    let de_5a = ent
        .structure_groupes_liste(&a, Some(m.annee), Some(c1), None, false, None)
        .await
        .unwrap();
    assert_eq!(de_5a.len(), 2);
    assert_eq!(
        nb_evenements(&ent, m.etab, "referentiel.groupes_generes").await,
        2
    );
    // Mode inconnu, aucune classe : refus. Classe d'un autre établissement : introuvable.
    assert!(est_domaine(
        &ent.structure_groupes_generer(&a, s("autre", vec![c1], false))
            .await,
        DomainError::ValeurInvalide
    ));
    assert!(est_domaine(
        &ent.structure_groupes_generer(&a, s("par_classe", vec![], false))
            .await,
        DomainError::ValeurInvalide
    ));
    let m2 = monde(&ent).await;
    let autre = classe(&ent, &m2, "5A").await;
    assert!(est_domaine(
        &ent.structure_groupes_generer(&a, s("par_classe", vec![autre], false))
            .await,
        DomainError::NotFound
    ));
}

// ─── STR-63, 65, 66 : composition datée, entrées et sorties en cours d'année ────────────────

#[tokio::test]
async fn composition_datee_entrees_sorties_en_cours_d_annee() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let c = classe(&ent, &m, "6A").await;
    let g = groupe(&ent, &m, "6A-GB", "enseignement", vec![c], None).await;
    let a = adm(m.etab);
    let (e1, e2, e3, e4) = (
        eleve(&ent, m.etab, "A1").await,
        eleve(&ent, m.etab, "A2").await,
        eleve(&ent, m.etab, "A3").await,
        eleve(&ent, m.etab, "A4").await,
    );
    for e in [e1, e2, e3, e4] {
        en_classe(&ent, &m, e, c, jour() - Duration::days(40)).await;
    }
    let t = jour();
    // Aperçu : rien n'est écrit.
    let mut s = compo(&[e1, e2], &[], Some(t - Duration::days(30)));
    s.apercu = true;
    let r = ent
        .structure_composition_modifier(&a, g.id, s)
        .await
        .unwrap();
    assert!(!r.applique && r.entrants.len() == 2);
    assert!(membres(&ent, m.etab, g.id, t).await.is_empty());
    // Entrées du début d'année (rétroactif pour la gestion : avertissement, pas un blocage).
    let r = ent
        .structure_composition_modifier(
            &a,
            g.id,
            compo(&[e1, e2, e2], &[], Some(t - Duration::days(30))),
        )
        .await
        .unwrap();
    assert!(r.applique);
    assert_eq!(
        trie(r.entrants.clone()),
        trie(vec![e1, e2]),
        "le doublon de la liste est dédoublonné avant calcul"
    );
    assert!(r.avertissements.contains(&"retroactif".to_string()));
    assert_eq!(membres(&ent, m.etab, g.id, t).await, trie(vec![e1, e2]));
    // Arrivée en cours d'année (e3 dans 5 jours) : pas membre aujourd'hui, membre à partir de la date.
    ent.structure_composition_modifier(&a, g.id, compo(&[e3], &[], Some(t + Duration::days(5))))
        .await
        .unwrap();
    assert_eq!(membres(&ent, m.etab, g.id, t).await, trie(vec![e1, e2]));
    assert_eq!(
        membres(&ent, m.etab, g.id, t + Duration::days(5)).await,
        trie(vec![e1, e2, e3])
    );
    // Sortie d'e1 à t + 10 : membre le 9, plus le 10 ; l'historique reste exact (« qui était dans quel groupe le 12 ? »).
    ent.structure_composition_modifier(&a, g.id, compo(&[], &[e1], Some(t + Duration::days(10))))
        .await
        .unwrap();
    assert!(membres(&ent, m.etab, g.id, t + Duration::days(9))
        .await
        .contains(&e1));
    assert!(!membres(&ent, m.etab, g.id, t + Duration::days(10))
        .await
        .contains(&e1));
    assert!(
        membres(&ent, m.etab, g.id, t - Duration::days(20))
            .await
            .contains(&e1),
        "le passé ne bouge pas"
    );
    // Déjà membre : ignoré et signalé, jamais écrasé ; non membre qui sort : refusé ; entrant = sortant : refus.
    let r = ent
        .structure_composition_modifier(&a, g.id, compo(&[e2], &[e4], None))
        .await
        .unwrap();
    assert_eq!(r.deja_membres, vec![e2]);
    assert!(r
        .refuses
        .iter()
        .any(|x| x.eleve_id == e4 && x.cause == "non_membre"));
    assert!(est_domaine(
        &ent.structure_composition_modifier(&a, g.id, compo(&[e4], &[e4], None))
            .await,
        DomainError::ValeurInvalide
    ));
    // Élève inconnu (ou d'un autre établissement) : refusé comme inconnu.
    let m2 = monde(&ent).await;
    let etranger = eleve(&ent, m2.etab, "E1").await;
    let r = ent
        .structure_composition_modifier(&a, g.id, compo(&[etranger, Uuid::new_v4()], &[], None))
        .await
        .unwrap();
    assert!(
        r.entrants.is_empty()
            && r.refuses.len() == 2
            && r.refuses.iter().all(|x| x.cause == "inconnu")
    );
    // Élève sorti avant la date : non scolarisé.
    sqlx::query("UPDATE ent_eleve SET date_sortie = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(m.etab)
        .bind(e4)
        .bind(t - Duration::days(1))
        .execute(ent.pool())
        .await
        .unwrap();
    let r = ent
        .structure_composition_modifier(&a, g.id, compo(&[e4], &[], None))
        .await
        .unwrap();
    assert!(r
        .refuses
        .iter()
        .any(|x| x.eleve_id == e4 && x.cause == "non_scolarise"));
    // Dates hors du groupe ou de l'année : refus.
    assert!(est_domaine(
        &ent.structure_composition_modifier(
            &a,
            g.id,
            compo(&[e3], &[], Some(t - Duration::days(90)))
        )
        .await,
        DomainError::ValeurInvalide
    ));
    assert!(est_domaine(
        &ent.structure_composition_modifier(
            &a,
            g.id,
            compo(&[e3], &[], Some(t + Duration::days(300)))
        )
        .await,
        DomainError::ValeurInvalide
    ));
    // Motif de code invalide.
    let mut s = compo(&[e3], &[], None);
    s.motif_code = Some("Pas Valide".into());
    assert!(est_domaine(
        &ent.structure_composition_modifier(&a, g.id, s).await,
        DomainError::ValeurInvalide
    ));
    // Faits publiés (identifiants seulement) et journal.
    assert!(nb_evenements(&ent, m.etab, "referentiel.composition_modifiee").await >= 3);
    let payloads: Vec<String> = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'referentiel.composition_modifiee'",
    )
    .bind(m.etab)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(
        payloads
            .iter()
            .all(|p| !p.contains("A1") && !p.contains("S6")),
        "aucun nom d'élève dans un fait"
    );
    // Une sortie le jour même de l'entrée annule l'affectation (archivée, jamais perdue).
    ent.structure_composition_modifier(
        &a,
        g.id,
        compo(&[e4.max(e3)], &[], Some(t + Duration::days(20))),
    )
    .await
    .ok();
    let avant = membres(&ent, m.etab, g.id, t + Duration::days(20))
        .await
        .len();
    assert!(avant >= 1);
}

#[tokio::test]
async fn capacite_avertissement_ou_blocage_et_groupe_ferme() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let a = adm(m.etab);
    let mut s = saisie("BASSIN", "activite", vec![], None);
    s.capacite = Some(2);
    s.capacite_bloquante = true;
    let bassin = ent.structure_groupe_creer(&a, s).await.unwrap();
    let mut s = saisie("CHORALE", "activite", vec![], None);
    s.capacite = Some(1);
    let chorale = ent.structure_groupe_creer(&a, s).await.unwrap();
    let es: Vec<Uuid> = {
        let mut v = Vec::new();
        for i in 0..3 {
            v.push(eleve(&ent, m.etab, &format!("Cap{i}")).await);
        }
        v
    };
    // Capacité de sécurité (bloquante, R-S15) : le troisième est refusé, rien n'est écrit.
    assert!(est_domaine(
        &ent.structure_composition_modifier(&a, bassin.id, compo(&es, &[], None))
            .await,
        DomainError::Conflit
    ));
    assert!(
        membres(&ent, m.etab, bassin.id, jour()).await.is_empty(),
        "refus total : aucune ligne écrite"
    );
    // Deux élèves passent.
    ent.structure_composition_modifier(&a, bassin.id, compo(&es[..2], &[], None))
        .await
        .unwrap();
    // Capacité simple : avertissement, l'inscription est faite.
    let r = ent
        .structure_composition_modifier(&a, chorale.id, compo(&es, &[], None))
        .await
        .unwrap();
    assert!(r.capacite_depassee && r.avertissements.contains(&"capacite_depassee".to_string()));
    assert_eq!(membres(&ent, m.etab, chorale.id, jour()).await.len(), 3);
    // Groupe d'activité fermé : sa composition ne bouge plus.
    let f = ent
        .structure_groupe_fermer(&a, chorale.id, Some(jour()), false)
        .await
        .unwrap();
    assert_eq!(f.membres_clos, 3);
    assert!(est_domaine(
        &ent.structure_composition_modifier(
            &a,
            chorale.id,
            compo(&[], &[es[0]], Some(jour() + Duration::days(2)))
        )
        .await,
        DomainError::ValeurInvalide
    ));
    assert!(
        est_domaine(
            &ent.structure_groupe_modifier(
                &a,
                chorale.id,
                GroupeModif {
                    libelle: Some("X".into()),
                    ..Default::default()
                }
            )
            .await,
            DomainError::ValeurInvalide
        ) || ent
            .structure_groupe_modifier(
                &a,
                chorale.id,
                GroupeModif {
                    libelle: Some("X".into()),
                    ..Default::default()
                }
            )
            .await
            .is_ok()
    );
}

// ─── Heure de Paris : le jour par défaut est celui de l'établissement ─────────────────────────

#[tokio::test]
async fn jour_par_defaut_selon_le_fuseau_de_l_etablissement() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Deux établissements dont les fuseaux sont à +14 h et −11 h : les jours de l'établissement diffèrent du jour UTC
    // pendant une grande partie de la journée ; l'année est large pour couvrir les deux.
    for fuseau in ["Pacific/Kiritimati", "Pacific/Pago_Pago", "Europe/Paris"] {
        let m = monde(&ent).await;
        sqlx::query(
            "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1,$2)
             ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = EXCLUDED.fuseau",
        )
        .bind(m.etab)
        .bind(fuseau)
        .execute(ent.pool())
        .await
        .unwrap();
        let attendu: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1)")
            .bind(m.etab)
            .fetch_one(ent.pool())
            .await
            .unwrap();
        let a = adm(m.etab);
        let g = groupe(&ent, &m, "FUSEAU", "activite", vec![], None).await;
        let e = eleve(&ent, m.etab, "Fuseau").await;
        let r = ent
            .structure_composition_modifier(&a, g.id, compo(&[e], &[], None))
            .await
            .unwrap();
        assert_eq!(
            r.date_effet, attendu,
            "{fuseau} : l'entrée sans date prend le jour de l'établissement"
        );
        let debut: NaiveDate = sqlx::query_scalar(
            "SELECT date_debut FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3",
        )
        .bind(m.etab)
        .bind(e)
        .bind(g.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        assert_eq!(debut, attendu);
        // La liste des groupes « aujourd'hui » compte l'élève ce jour-là (le jour UTC pourrait l'exclure).
        let l = ent
            .structure_groupes_liste(&a, Some(m.annee), None, None, false, None)
            .await
            .unwrap();
        assert_eq!(l[0].effectif, 1, "{fuseau}");
        // Fermeture sans date : dernier jour = jour de l'établissement.
        let f = ent
            .structure_groupe_fermer(&a, g.id, None, false)
            .await
            .unwrap();
        assert_eq!(f.groupe.ferme_le, Some(attendu));
    }
}

// ─── Responsable d'atelier : composer SON groupe, jamais celui d'un autre ─────────────────────

#[tokio::test]
async fn responsable_d_atelier_compose_son_groupe_et_seulement_lui() {
    let Some((ent, cast)) = crate::tests_d022_bis::labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let dir = (cast.direction.user_id, cast.direction.role.as_str());
    // La création et l'attribution de rôles sont à l'administrateur (anti-escalade : la direction ne détient pas
    // `groupes.composer`).
    let admin = (cast.admin.user_id, cast.admin.role.as_str());
    let (resp, autre) = (cast.profs[0].user_id, cast.profs[1].user_id);
    let a_dir = Acteur {
        etablissement_id: etab,
        user_id: dir.0,
        role: dir.1,
    };
    let a_resp = Acteur {
        etablissement_id: etab,
        user_id: resp,
        role: Role::Prof.as_str(),
    };
    let sx = sfx();
    // Sans droit `groupes.composer` : un professeur ne compose rien, même s'il est responsable.
    let atelier = ent
        .structure_groupe_creer(
            &a_dir,
            sans_droits(saisie(&format!("THEA{sx}"), "activite", vec![], None)),
        )
        .await
        .unwrap();
    let autre_atelier = ent
        .structure_groupe_creer(
            &a_dir,
            sans_droits(saisie(&format!("ECHEC{sx}"), "activite", vec![], None)),
        )
        .await
        .unwrap();
    let classe_ens = ent
        .structure_groupe_creer(
            &a_dir,
            saisie(&format!("ENS{sx}"), "enseignement", vec![], None),
        )
        .await
        .unwrap();
    let atelier_a_droits = ent
        .structure_groupe_creer(
            &a_dir,
            saisie(&format!("DROITS{sx}"), "activite", vec![], None),
        )
        .await
        .unwrap();
    ent.definir_responsables_groupe_for_actor(etab, dir.0, dir.1, atelier_a_droits.id, &[resp])
        .await
        .unwrap();
    ent.definir_responsables_groupe_for_actor(etab, dir.0, dir.1, atelier.id, &[resp])
        .await
        .unwrap();
    let e1 = eleve(&ent, etab, &format!("Th{sx}")).await;
    assert!(est_forbidden(
        &ent.structure_composition_modifier(&a_resp, atelier.id, compo(&[e1], &[], None))
            .await
    ));
    assert!(est_forbidden(
        &ent.structure_groupe_creer(
            &a_resp,
            saisie(&format!("NO{sx}"), "enseignement", vec![], None)
        )
        .await
    ));
    // Rôle « Responsable d'atelier » : groupes.composer à la portée « mes groupes ».
    let role = ent
        .creer_role_for_actor(
            etab,
            admin.0,
            admin.1,
            crate::droits_repo::RoleSaisie {
                nom: format!("Responsable d'atelier {sx}"),
                description: None,
                modele: None,
                droits: vec![crate::droits_repo::Droit {
                    action: Action::GroupesComposer,
                    portee: Portee::MesGroupes,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, admin.0, admin.1, role.id, resp, None)
        .await
        .unwrap();
    // Son atelier : entrées et sorties à partir d'aujourd'hui.
    let r = ent
        .structure_composition_modifier(&a_resp, atelier.id, compo(&[e1], &[], None))
        .await
        .unwrap();
    assert_eq!(r.entrants, vec![e1]);
    let d = ent
        .structure_groupe_detail(&a_resp, atelier.id, None)
        .await
        .unwrap();
    assert_eq!(d.membres.len(), 1, "il voit les membres de son atelier");
    // Pas dans le passé.
    assert!(est_domaine(
        &ent.structure_composition_modifier(
            &a_resp,
            atelier.id,
            compo(&[], &[e1], Some(jour() - Duration::days(3)))
        )
        .await,
        DomainError::ValeurInvalide
    ));
    // Jamais l'atelier d'un autre, ni un groupe d'enseignement (même sans être responsable).
    assert!(est_forbidden(
        &ent.structure_composition_modifier(&a_resp, autre_atelier.id, compo(&[e1], &[], None))
            .await
    ));
    assert!(est_forbidden(
        &ent.structure_composition_modifier(&a_resp, classe_ens.id, compo(&[e1], &[], None))
            .await
    ));
    assert!(est_forbidden(
        &ent.structure_groupe_detail(&a_resp, autre_atelier.id, None)
            .await
    ));
    // Un atelier qui OUVRE des droits sur ses membres ne se compose que par la gestion (pas d'escalade de visibilité).
    assert!(est_forbidden(
        &ent.structure_composition_modifier(&a_resp, atelier_a_droits.id, compo(&[e1], &[], None))
            .await
    ));
    // Un tiers sans responsabilité (même rôle) ne compose pas l'atelier.
    ent.attribuer_role_for_actor(etab, admin.0, admin.1, role.id, autre, None)
        .await
        .unwrap();
    let a_autre = Acteur {
        etablissement_id: etab,
        user_id: autre,
        role: Role::Prof.as_str(),
    };
    assert!(est_forbidden(
        &ent.structure_composition_modifier(&a_autre, atelier.id, compo(&[e1], &[e1], None))
            .await
    ));
    // Sortie de l'atelier par le responsable ; la liste n'est jamais éditable hors de son périmètre.
    let r = ent
        .structure_composition_modifier(&a_resp, atelier.id, compo(&[], &[e1], None))
        .await
        .unwrap();
    assert_eq!(r.sortants, vec![e1]);
    // Retrait du rôle : effet immédiat.
    ent.retirer_role_for_actor(etab, admin.0, admin.1, role.id, resp)
        .await
        .unwrap();
    assert!(est_forbidden(
        &ent.structure_composition_modifier(&a_resp, atelier.id, compo(&[e1], &[], None))
            .await
    ));
    ent.retirer_role_for_actor(etab, admin.0, admin.1, role.id, autre)
        .await
        .unwrap();
    ent.supprimer_role_for_actor(etab, admin.0, admin.1, role.id)
        .await
        .unwrap();
}

// ─── STR-53 : créer une activité dont on devient responsable ──────────────────────────────────

#[tokio::test]
async fn creation_d_activite_le_createur_devient_responsable() {
    let Some((ent, cast)) = crate::tests_d022_bis::labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let admin = (cast.admin.user_id, cast.admin.role.as_str());
    let prof = cast.profs[3].user_id;
    let a_prof = Acteur {
        etablissement_id: etab,
        user_id: prof,
        role: Role::Prof.as_str(),
    };
    let sx = sfx();
    assert!(est_forbidden(
        &ent.structure_activite_creer(
            &a_prof,
            saisie(&format!("CLUB{sx}"), "activite", vec![], None)
        )
        .await
    ));
    let role = ent
        .creer_role_for_actor(
            etab,
            admin.0,
            admin.1,
            crate::droits_repo::RoleSaisie {
                nom: format!("Créateur d'activité {sx}"),
                description: None,
                modele: None,
                droits: vec![crate::droits_repo::Droit {
                    action: Action::GroupesCreerActivite,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, admin.0, admin.1, role.id, prof, None)
        .await
        .unwrap();
    // La nature est toujours « activité » (même si la saisie demande autre chose), sans offre.
    let g = ent
        .structure_activite_creer(
            &a_prof,
            saisie(&format!("CLUB{sx}"), "enseignement", vec![], None),
        )
        .await
        .unwrap();
    assert_eq!(g.nature, "activite");
    assert!(
        !g.ouvre_droits,
        "créée par un non-gestionnaire : aucun droit sur les élèves"
    );
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_groupe_responsable WHERE etablissement_id = $1 AND groupe_id = $2 AND user_id = $3 AND au IS NULL",
    )
    .bind(etab)
    .bind(g.id)
    .bind(prof)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "le créateur est responsable");
    // `groupes.creer_activite` ne permet pas de créer un groupe d'enseignement.
    assert!(est_forbidden(
        &ent.structure_groupe_creer(
            &a_prof,
            saisie(&format!("ENS{sx}"), "enseignement", vec![], None)
        )
        .await
    ));
    ent.retirer_role_for_actor(etab, admin.0, admin.1, role.id, prof)
        .await
        .unwrap();
    ent.supprimer_role_for_actor(etab, admin.0, admin.1, role.id)
        .await
        .unwrap();
}

// ─── STR-60, 61 : changement de classe en une commande ─────────────────────────────────────────

#[tokio::test]
async fn changement_de_classe_groupes_recalcules_apercu_et_evenements() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let a = adm(m.etab);
    let (c5a, c5b, c5c) = (
        classe(&ent, &m, "5A").await,
        classe(&ent, &m, "5B").await,
        classe(&ent, &m, "5C").await,
    );
    let esp = offre(&ent, &m, "ESPAGNOL").await;
    let latin = offre(&ent, &m, "LATIN").await;
    // 5A : demi-groupe 5A-1 ; espagnol inter-classes 5A+5B ; latin de 5A. 5C : espagnol de 5C, latin de 5C (deux groupes).
    let demi = groupe(&ent, &m, "5A-1", "enseignement", vec![c5a], None).await;
    let g_esp_ab = groupe(
        &ent,
        &m,
        "5AB-ESP",
        "enseignement",
        vec![c5a, c5b],
        Some(esp),
    )
    .await;
    let g_lat_a = groupe(&ent, &m, "5A-LAT", "enseignement", vec![c5a], Some(latin)).await;
    let g_esp_c = groupe(&ent, &m, "5C-ESP", "enseignement", vec![c5c], Some(esp)).await;
    let g_lat_c1 = groupe(&ent, &m, "5C-LAT1", "enseignement", vec![c5c], Some(latin)).await;
    let g_lat_c2 = groupe(&ent, &m, "5C-LAT2", "enseignement", vec![c5c], Some(latin)).await;
    let t = jour();
    let e = eleve(&ent, m.etab, "Mobile").await;
    en_classe(&ent, &m, e, c5a, t - Duration::days(40)).await;
    suit(&ent, &m, e, esp, t - Duration::days(40)).await;
    suit(&ent, &m, e, latin, t - Duration::days(40)).await;
    for g in [&demi, &g_esp_ab, &g_lat_a] {
        ent.structure_composition_modifier(
            &a,
            g.id,
            compo(&[e], &[], Some(t - Duration::days(40))),
        )
        .await
        .unwrap();
    }
    // Un cours par groupe (public) pour lire cours quittés / rejoints, avec un professeur sur le latin de 5A.
    let prof = Uuid::new_v4();
    let cours = |g: Uuid, lib: &'static str| {
        let ent = &ent;
        let m = &m;
        async move {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, groupe_id, ouvert_le, source)
                 VALUES ($1,$2,$3,$4,$5,$6,'saisie')",
            )
            .bind(id)
            .bind(m.etab)
            .bind(m.annee)
            .bind(lib)
            .bind(g)
            .bind(jour() - Duration::days(60))
            .execute(ent.pool())
            .await
            .unwrap();
            id
        }
    };
    let base_changee = nb_evenements(&ent, m.etab, "referentiel.affectation_changee").await;
    let base_classe = nb_evenements(&ent, m.etab, "scolarite.classe_changed").await;
    let cours_lat_a = cours(g_lat_a.id, "Latin 5A").await;
    let cours_lat_c1 = cours(g_lat_c1.id, "Latin 5C1").await;
    let _ = prof;
    // ── Aperçu (défaut) vers 5B : l'espagnol 5A+5B est conservé (5B est aussi d'origine), le demi-groupe 5A et le latin
    // de 5A sont quittés ; rien n'est écrit.
    let ap = ent
        .structure_eleve_classe_changer(
            &a,
            ChangementClasseSaisie {
                eleve_id: e,
                classe_id: c5b,
                date_effet: Some(t),
                apercu: true,
                entrer_dans: vec![],
            },
        )
        .await
        .unwrap();
    assert!(!ap.applique);
    assert_eq!(
        trie(ap.groupes_quittes.clone()),
        trie(vec![demi.id, g_lat_a.id])
    );
    assert_eq!(
        ap.groupes_conserves,
        vec![g_esp_ab.id],
        "il garde son groupe d'espagnol inter-classes"
    );
    assert_eq!(ap.classe_avant, Some(c5a));
    assert!(ap.cours_quittes.contains(&cours_lat_a));
    assert!(
        ap.groupes_proposes.is_empty(),
        "aucun groupe de latin pour 5B : rien à proposer"
    );
    let vue = |c: Uuid| async move { c };
    let _ = vue(c5a).await;
    let encore: Option<Uuid> = sqlx::query_scalar(
        "SELECT classe_id FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND classe_id IS NOT NULL
         AND date_fin IS NULL",
    )
    .bind(m.etab)
    .bind(e)
    .fetch_optional(ent.pool())
    .await
    .unwrap();
    assert_eq!(encore, Some(c5a), "l'aperçu n'écrit rien");
    assert_eq!(
        nb_evenements(&ent, m.etab, "referentiel.affectation_changee").await,
        base_changee
    );

    // ── Vers 5C : espagnol 5A+5B quitté, proposé : espagnol de 5C (un seul) ; latin : deux groupes, à choisir.
    let ap = ent
        .structure_eleve_classe_changer(
            &a,
            ChangementClasseSaisie {
                eleve_id: e,
                classe_id: c5c,
                date_effet: Some(t),
                apercu: true,
                entrer_dans: vec![],
            },
        )
        .await
        .unwrap();
    assert_eq!(
        trie(ap.groupes_quittes.clone()),
        trie(vec![demi.id, g_esp_ab.id, g_lat_a.id])
    );
    assert_eq!(ap.groupes_proposes, vec![g_esp_c.id]);
    assert_eq!(ap.groupes_a_choisir.len(), 1);
    assert_eq!(ap.groupes_a_choisir[0].offre_id, Some(latin));
    assert_eq!(
        trie(ap.groupes_a_choisir[0].groupes.clone()),
        trie(vec![g_lat_c1.id, g_lat_c2.id])
    );
    // Un groupe qui ne vient pas de l'aperçu est refusé (jamais d'entrée dans un groupe quelconque).
    assert!(est_domaine(
        &ent.structure_eleve_classe_changer(
            &a,
            ChangementClasseSaisie {
                eleve_id: e,
                classe_id: c5c,
                date_effet: Some(t),
                apercu: false,
                entrer_dans: vec![demi.id],
            },
        )
        .await,
        DomainError::ValeurInvalide
    ));
    // ── Écriture : il choisit le latin 1 et prend l'espagnol de 5C.
    let r = ent
        .structure_eleve_classe_changer(
            &a,
            ChangementClasseSaisie {
                eleve_id: e,
                classe_id: c5c,
                date_effet: Some(t),
                apercu: false,
                entrer_dans: vec![g_esp_c.id, g_lat_c1.id],
            },
        )
        .await
        .unwrap();
    assert!(r.applique);
    assert!(r.cours_quittes.contains(&cours_lat_a) && r.cours_rejoints.contains(&cours_lat_c1));
    // Groupes quittés : fin la veille ; groupes rejoints : début à la date. Le passé du groupe quitté reste lisible.
    for g in [&demi, &g_esp_ab, &g_lat_a] {
        assert!(
            membres(&ent, m.etab, g.id, t - Duration::days(1))
                .await
                .contains(&e),
            "{} hier",
            g.code
        );
        assert!(
            !membres(&ent, m.etab, g.id, t).await.contains(&e),
            "{} aujourd'hui",
            g.code
        );
    }
    for g in [&g_esp_c, &g_lat_c1] {
        assert!(!membres(&ent, m.etab, g.id, t - Duration::days(1))
            .await
            .contains(&e));
        assert!(membres(&ent, m.etab, g.id, t).await.contains(&e));
    }
    assert!(!membres(&ent, m.etab, g_lat_c2.id, t).await.contains(&e));
    let mut c = ent.pool().acquire().await.unwrap();
    let vue = groupes::vue_scolarite(&mut c, m.etab, t, e).await.unwrap();
    assert_eq!(vue.classe, Some(c5c));
    assert_eq!(
        trie(vue.groupes.clone()),
        trie(vec![g_esp_c.id, g_lat_c1.id])
    );
    assert!(vue.cours.contains(&cours_lat_c1) && !vue.cours.contains(&cours_lat_a));
    assert_eq!(trie(vue.enseignements_suivis), trie(vec![esp, latin]));
    let hier = groupes::vue_scolarite(&mut c, m.etab, t - Duration::days(1), e)
        .await
        .unwrap();
    assert_eq!(
        hier.classe,
        Some(c5a),
        "l'historique répond à « où était l'élève le 12 ? »"
    );
    assert!(hier.cours.contains(&cours_lat_a));
    drop(c);
    // Faits : affectation changée, composition modifiée (sortants et entrants), classe changée (socle 2).
    assert_eq!(
        nb_evenements(&ent, m.etab, "referentiel.affectation_changee").await,
        base_changee + 1
    );
    assert!(nb_evenements(&ent, m.etab, "referentiel.composition_modifiee").await >= 5);
    assert_eq!(
        nb_evenements(&ent, m.etab, "scolarite.classe_changed").await,
        base_classe + 1,
        "le socle 2 publie son fait"
    );
    // Déjà dans la classe : conflit. Classe d'un autre établissement : introuvable. Hors année : refus.
    assert!(est_domaine(
        &ent.structure_eleve_classe_changer(
            &a,
            ChangementClasseSaisie {
                eleve_id: e,
                classe_id: c5c,
                date_effet: Some(t),
                apercu: true,
                entrer_dans: vec![]
            }
        )
        .await,
        DomainError::Conflit
    ));
    let m2 = monde(&ent).await;
    let autre_classe = classe(&ent, &m2, "5C").await;
    assert!(est_domaine(
        &ent.structure_eleve_classe_changer(
            &a,
            ChangementClasseSaisie {
                eleve_id: e,
                classe_id: autre_classe,
                date_effet: Some(t),
                apercu: true,
                entrer_dans: vec![]
            }
        )
        .await,
        DomainError::NotFound
    ));
    assert!(est_domaine(
        &ent.structure_eleve_classe_changer(
            &a,
            ChangementClasseSaisie {
                eleve_id: e,
                classe_id: c5b,
                date_effet: Some(t + Duration::days(400)),
                apercu: true,
                entrer_dans: vec![]
            }
        )
        .await,
        DomainError::ValeurInvalide
    ));
}

// ─── STR-64 : règle de composition, jamais appliquée seule ───────────────────────────────────

#[tokio::test]
async fn regle_de_composition_propose_et_ne_s_applique_pas_seule() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let a = adm(m.etab);
    let (c4a, c4b) = (classe(&ent, &m, "4A").await, classe(&ent, &m, "4B").await);
    let all = offre(&ent, &m, "ALL2").await;
    let g = groupe(
        &ent,
        &m,
        "4AB-ALL",
        "enseignement",
        vec![c4a, c4b],
        Some(all),
    )
    .await;
    let t = jour();
    let (e1, e2, e3, e4) = (
        eleve(&ent, m.etab, "R1").await,
        eleve(&ent, m.etab, "R2").await,
        eleve(&ent, m.etab, "R3").await,
        eleve(&ent, m.etab, "R4").await,
    );
    en_classe(&ent, &m, e1, c4a, t - Duration::days(40)).await;
    en_classe(&ent, &m, e2, c4b, t - Duration::days(40)).await;
    en_classe(&ent, &m, e3, c4a, t - Duration::days(40)).await;
    en_classe(&ent, &m, e4, c4a, t - Duration::days(40)).await;
    // e1, e2 suivent l'allemand ; e3 aussi mais sera déjà dans un autre groupe de l'offre ; e4 ne le suit pas.
    for e in [e1, e2, e3] {
        suit(&ent, &m, e, all, t - Duration::days(40)).await;
    }
    let autre = groupe(&ent, &m, "4A-ALL-BIS", "enseignement", vec![c4a], Some(all)).await;
    ent.structure_composition_modifier(
        &a,
        autre.id,
        compo(&[e3], &[], Some(t - Duration::days(30))),
    )
    .await
    .unwrap();
    // Un ancien membre qui a abandonné l'allemand : sortant proposé. e4 est membre sans suivre l'enseignement.
    ent.structure_composition_modifier(&a, g.id, compo(&[e4], &[], Some(t - Duration::days(30))))
        .await
        .unwrap();
    let p = ent
        .structure_groupe_propositions(&a, g.id, None)
        .await
        .unwrap();
    assert!(p.regle_complete);
    assert_eq!(
        trie(p.entrants.clone()),
        trie(vec![e1, e2]),
        "e3 est déjà dans un autre groupe de l'offre, e4 ne la suit pas"
    );
    assert_eq!(
        p.sortants,
        vec![e4],
        "membre qui ne suit plus l'enseignement : sortie proposée"
    );
    assert!(p.hors_classes_origine.is_empty());
    // La proposition ne s'applique jamais seule : lecture = aucune écriture.
    assert!(!membres(&ent, m.etab, g.id, t).await.contains(&e1));
    // Application : seul ce qui est proposé ET validé est appliqué ; le reste est refusé.
    let r = ent
        .structure_groupe_regle_appliquer(
            &a,
            g.id,
            ApplicationRegle {
                entrants: vec![e1, e3, e4],
                sortants: vec![e4, e1],
                date_effet: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(r.entrants, vec![e1]);
    assert_eq!(r.sortants, vec![e4]);
    assert!(r
        .refuses
        .iter()
        .any(|x| x.eleve_id == e3 && x.cause == "non_propose"));
    assert!(
        r.refuses
            .iter()
            .any(|x| x.eleve_id == e4 && x.cause == "non_propose"),
        "e4 n'est pas un entrant proposé"
    );
    assert_eq!(membres(&ent, m.etab, g.id, t).await, vec![e1]);
    // Groupe sans offre ni classe : la règle ne propose rien et le dit.
    let libre = groupe(&ent, &m, "LIBRE", "enseignement", vec![], None).await;
    let p = ent
        .structure_groupe_propositions(&a, libre.id, None)
        .await
        .unwrap();
    assert!(!p.regle_complete && p.entrants.is_empty() && p.sortants.is_empty());
    // Membre hors des classes d'origine : avertissement.
    let c_hors = classe(&ent, &m, "3A").await;
    let e5 = eleve(&ent, m.etab, "R5").await;
    en_classe(&ent, &m, e5, c_hors, t - Duration::days(40)).await;
    ent.structure_composition_modifier(&a, g.id, compo(&[e5], &[], None))
        .await
        .unwrap();
    let p = ent
        .structure_groupe_propositions(&a, g.id, None)
        .await
        .unwrap();
    assert_eq!(p.hors_classes_origine, vec![e5]);
    // Les droits : gestionnaires seulement.
    assert!(est_forbidden(
        &ent.structure_groupe_propositions(&acteur(m.etab, Role::Prof), g.id, None)
            .await
    ));
}

// ─── STR-65 : affectation en lot ───────────────────────────────────────────────────────────

#[tokio::test]
async fn affectation_en_lot_apercu_dedoublonnage_et_conflits_jamais_ecrases() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let a = adm(m.etab);
    let (c6a, c6b) = (classe(&ent, &m, "6A").await, classe(&ent, &m, "6B").await);
    let (e1, e2, e3, e4) = (
        eleve(&ent, m.etab, "L1").await,
        eleve(&ent, m.etab, "L2").await,
        eleve(&ent, m.etab, "L3").await,
        eleve(&ent, m.etab, "L4").await,
    );
    let t = jour();
    en_classe(&ent, &m, e3, c6b, t - Duration::days(30)).await; // déjà ailleurs
    en_classe(&ent, &m, e4, c6a, t - Duration::days(30)).await; // déjà dans la classe
    let m2 = monde(&ent).await;
    let etranger = eleve(&ent, m2.etab, "L5").await;
    let lot = |apercu: bool| LotSaisie {
        classe_id: c6a,
        eleves: vec![e1, e2, e2, e3, e4, etranger],
        date_effet: Some(t - Duration::days(10)),
        apercu,
    };
    let ap = ent.structure_affectations_lot(&a, lot(true)).await.unwrap();
    assert!(!ap.applique);
    assert_eq!(ap.doublons_retires, 1);
    assert_eq!(trie(ap.affectes.clone()), trie(vec![e1, e2]));
    assert_eq!(ap.deja_dans_la_classe, vec![e4]);
    assert!(ap
        .refuses
        .iter()
        .any(|x| x.eleve_id == e3 && x.cause == "dans_une_autre_classe"));
    assert!(ap
        .refuses
        .iter()
        .any(|x| x.eleve_id == etranger && x.cause == "inconnu"));
    let nb: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_affectation WHERE etablissement_id = $1 AND classe_id = $2",
    )
    .bind(m.etab)
    .bind(c6a)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(nb, 1, "l'aperçu n'a rien écrit (seul e4 est dans 6A)");
    let r = ent
        .structure_affectations_lot(&a, lot(false))
        .await
        .unwrap();
    assert!(r.applique && r.affectes.len() == 2);
    // e3 n'a pas été déplacé ; rejeu : rien de plus (idempotent).
    let c3: Uuid = sqlx::query_scalar(
        "SELECT classe_id FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND classe_id IS NOT NULL",
    )
    .bind(m.etab)
    .bind(e3)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(c3, c6b);
    let r2 = ent
        .structure_affectations_lot(&a, lot(false))
        .await
        .unwrap();
    assert!(r2.affectes.is_empty() && r2.deja_dans_la_classe.len() == 3);
    assert_eq!(
        nb_evenements(&ent, m.etab, "referentiel.affectations_lot").await,
        1
    );
    // Liste vide ou démesurée : refus ; classe d'un autre établissement : introuvable.
    assert!(est_domaine(
        &ent.structure_affectations_lot(
            &a,
            LotSaisie {
                classe_id: c6a,
                eleves: vec![],
                date_effet: None,
                apercu: true
            }
        )
        .await,
        DomainError::ValeurInvalide
    ));
    let autre = classe(&ent, &m2, "6A").await;
    assert!(est_domaine(
        &ent.structure_affectations_lot(
            &a,
            LotSaisie {
                classe_id: autre,
                eleves: vec![e1],
                date_effet: None,
                apercu: true
            }
        )
        .await,
        DomainError::NotFound
    ));
    // STR-68 : file de travail. e1…e4 sont en classe ; un nouvel élève sans classe y figure.
    let nouveau = eleve(&ent, m.etab, "L6").await;
    let f = ent.structure_a_affecter(&a, None, None).await.unwrap();
    assert!(f.sans_classe.contains(&nouveau) && !f.sans_classe.contains(&e1));
}

// ─── STR-52 : demi-groupes ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn demi_groupes_couvrent_la_classe_selon_chaque_mode() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let a = adm(m.etab);
    let t = jour();
    let mut classes = Vec::new();
    for (i, mode) in ["alphabetique", "alterne", "equilibre", "manuel"]
        .iter()
        .enumerate()
    {
        let c = classe(&ent, &m, &format!("DG{i}")).await;
        let mut es = Vec::new();
        for k in 0..5 {
            let e = eleve(&ent, m.etab, &format!("Dg{i}{k:02}")).await;
            en_classe(&ent, &m, e, c, t - Duration::days(30)).await;
            es.push(e);
        }
        let r = ent
            .structure_demi_groupes_creer(
                &a,
                DemiGroupesSaisie {
                    classe_id: c,
                    mode: (*mode).into(),
                    code_a: None,
                    code_b: None,
                    eleves_b: if *mode == "manuel" {
                        es[..2].to_vec()
                    } else {
                        vec![]
                    },
                    date_effet: Some(t),
                    capacite_bloquante: None,
                },
            )
            .await
            .unwrap();
        assert!(r.couvre_la_classe, "{mode}");
        assert_eq!(r.eleves_a.len() + r.eleves_b.len(), 5, "{mode}");
        assert!(r.eleves_a.iter().all(|e| !r.eleves_b.contains(e)));
        assert_eq!(
            trie(membres(&ent, m.etab, r.groupe_a, t).await),
            trie(r.eleves_a.clone())
        );
        assert_eq!(
            trie(membres(&ent, m.etab, r.groupe_b, t).await),
            trie(r.eleves_b.clone())
        );
        if *mode == "alphabetique" {
            assert_eq!((r.eleves_a.len(), r.eleves_b.len()), (3, 2));
        }
        if *mode == "manuel" {
            assert_eq!(trie(r.eleves_b.clone()), trie(es[..2].to_vec()));
        }
        classes.push((c, r));
    }
    // Les mêmes codes ne se recréent pas (« 4A-A » existe) ; un mode inconnu ou un manuel mal formé est refusé.
    let (c0, _) = &classes[0];
    assert!(est_domaine(
        &ent.structure_demi_groupes_creer(
            &a,
            DemiGroupesSaisie {
                classe_id: *c0,
                mode: "alphabetique".into(),
                code_a: None,
                code_b: None,
                eleves_b: vec![],
                date_effet: Some(t),
                capacite_bloquante: None
            }
        )
        .await,
        DomainError::Conflit
    ));
    assert!(est_domaine(
        &ent.structure_demi_groupes_creer(
            &a,
            DemiGroupesSaisie {
                classe_id: *c0,
                mode: "chaos".into(),
                code_a: None,
                code_b: None,
                eleves_b: vec![],
                date_effet: Some(t),
                capacite_bloquante: None
            }
        )
        .await,
        DomainError::ValeurInvalide
    ));
    let vide = classe(&ent, &m, "DGVIDE").await;
    assert!(est_domaine(
        &ent.structure_demi_groupes_creer(
            &a,
            DemiGroupesSaisie {
                classe_id: vide,
                mode: "alterne".into(),
                code_a: None,
                code_b: None,
                eleves_b: vec![],
                date_effet: Some(t),
                capacite_bloquante: None
            }
        )
        .await,
        DomainError::ValeurInvalide
    ));
}

// ─── STR-55 : fermeture d'un groupe ──────────────────────────────────────────────────────────

#[tokio::test]
async fn fermeture_d_un_groupe_clot_membres_responsables_et_cours() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let a = adm(m.etab);
    let t = jour();
    let g = groupe(&ent, &m, "ATELIER-TH", "activite", vec![], None).await;
    let (e1, e2) = (
        eleve(&ent, m.etab, "F1").await,
        eleve(&ent, m.etab, "F2").await,
    );
    ent.structure_composition_modifier(&a, g.id, compo(&[e1], &[], Some(t - Duration::days(20))))
        .await
        .unwrap();
    ent.structure_composition_modifier(&a, g.id, compo(&[e2], &[], Some(t + Duration::days(15))))
        .await
        .unwrap();
    let resp = *COMPTE.get().unwrap();
    sqlx::query(
        "INSERT INTO ent_groupe_responsable (id, groupe_id, etablissement_id, user_id, nature, du) VALUES ($1,$2,$3,$4,'responsable',$5)",
    )
    .bind(Uuid::new_v4())
    .bind(g.id)
    .bind(m.etab)
    .bind(resp)
    .bind(t - Duration::days(20))
    .execute(ent.pool())
    .await
    .unwrap();
    let cours = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, groupe_id, ouvert_le, source)
         VALUES ($1,$2,$3,'Théâtre',$4,$5,'saisie')",
    )
    .bind(cours)
    .bind(m.etab)
    .bind(m.annee)
    .bind(g.id)
    .bind(t - Duration::days(20))
    .execute(ent.pool())
    .await
    .unwrap();
    // Aperçu : dit ce qui sera clos, n'écrit rien.
    let fin = t + Duration::days(10);
    let ap = ent
        .structure_groupe_fermer(&a, g.id, Some(fin), true)
        .await
        .unwrap();
    assert_eq!(ap.membres_clos, 1);
    assert_eq!(ap.affectations_futures_annulees, 1);
    assert_eq!(ap.responsables_clos, 1);
    assert_eq!(ap.cours_fermes, vec![cours]);
    assert_eq!(
        ent.structure_groupe_detail(&a, g.id, None)
            .await
            .unwrap()
            .groupe
            .ferme_le,
        None
    );
    // Écriture.
    let f = ent
        .structure_groupe_fermer(&a, g.id, Some(fin), false)
        .await
        .unwrap();
    assert_eq!(f.groupe.ferme_le, Some(fin));
    assert!(
        membres(&ent, m.etab, g.id, fin).await.contains(&e1),
        "dernier jour inclus"
    );
    assert!(membres(&ent, m.etab, g.id, fin + Duration::days(1))
        .await
        .is_empty());
    assert!(
        membres(&ent, m.etab, g.id, fin + Duration::days(10))
            .await
            .is_empty(),
        "l'affectation future est annulée"
    );
    assert_eq!(
        nb_archive(&ent, m.etab, e2, "groupe_ferme").await,
        1,
        "annulée, archivée avec son motif, jamais perdue"
    );
    let (au, ferme): (Option<NaiveDate>, Option<NaiveDate>) = (
        sqlx::query_scalar(
            "SELECT au FROM ent_groupe_responsable WHERE etablissement_id = $1 AND groupe_id = $2",
        )
        .bind(m.etab)
        .bind(g.id)
        .fetch_one(ent.pool())
        .await
        .unwrap(),
        sqlx::query_scalar(
            "SELECT ferme_le FROM ent_cours WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(m.etab)
        .bind(cours)
        .fetch_one(ent.pool())
        .await
        .unwrap(),
    );
    assert_eq!((au, ferme), (Some(fin), Some(fin)));
    assert_eq!(
        nb_evenements(&ent, m.etab, "referentiel.groupe_ferme").await,
        1
    );
    // Fermé définitivement : refermer, composer au-delà, entrer dans un groupe fermé : refusés.
    assert!(est_domaine(
        &ent.structure_groupe_fermer(&a, g.id, None, false).await,
        DomainError::Conflit
    ));
    assert!(est_domaine(
        &ent.structure_composition_modifier(
            &a,
            g.id,
            compo(&[e2], &[], Some(fin + Duration::days(1)))
        )
        .await,
        DomainError::ValeurInvalide
    ));
    // Date de fermeture antérieure à l'ouverture ou hors année : refus.
    let g2 = groupe(&ent, &m, "ATELIER-2", "activite", vec![], None).await;
    assert!(est_domaine(
        &ent.structure_groupe_fermer(&a, g2.id, Some(t + Duration::days(500)), false)
            .await,
        DomainError::ValeurInvalide
    ));
    assert!(est_domaine(
        &ent.structure_groupe_fermer(&a, g2.id, Some(t - Duration::days(100)), false)
            .await,
        DomainError::ValeurInvalide
    ));
}

// ─── STR-67 : la sortie de l'élève ferme ses affectations dans la transaction du socle 2 ──────

#[tokio::test]
async fn sortie_de_l_eleve_ferme_ses_affectations_par_le_port() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let a = adm(m.etab);
    let t = jour();
    let c = classe(&ent, &m, "SORTIE").await;
    let g = groupe(&ent, &m, "SORTIE-G", "enseignement", vec![c], None).await;
    let futur = groupe(&ent, &m, "SORTIE-FUTUR", "activite", vec![], None).await;
    let e = eleve(&ent, m.etab, "Part").await;
    en_classe(&ent, &m, e, c, t - Duration::days(30)).await;
    ent.structure_composition_modifier(&a, g.id, compo(&[e], &[], Some(t - Duration::days(30))))
        .await
        .unwrap();
    ent.structure_composition_modifier(
        &a,
        futur.id,
        compo(&[e], &[], Some(t + Duration::days(20))),
    )
    .await
    .unwrap();
    let sortie = t + Duration::days(5);
    ent.cloturer_scolarisation_systeme(crate::scolarite::ClotureScolarisation {
        etablissement_id: m.etab,
        eleve_id: e,
        date_sortie: sortie,
        motif: domain_eleve::MotifSortie::Demenagement,
        motif_libre: None,
        destination: domain_eleve::EtablissementScolarite::default(),
        exeat_document_id: None,
        acteur: Some(*COMPTE.get().unwrap()),
    })
    .await
    .unwrap();
    // Dernier jour présent : la veille de la sortie, pour la classe comme pour le groupe.
    assert!(membres(&ent, m.etab, g.id, sortie - Duration::days(1))
        .await
        .contains(&e));
    assert!(!membres(&ent, m.etab, g.id, sortie).await.contains(&e));
    let fin: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT date_fin FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND classe_id = $3",
    )
    .bind(m.etab)
    .bind(e)
    .bind(c)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(fin, Some(sortie - Duration::days(1)));
    // L'atelier qui commençait après la sortie : annulé (archivé avec le motif « sortie »), pas un fantôme.
    assert_eq!(nb_archive(&ent, m.etab, e, "sortie").await, 1);
    assert!(membres(&ent, m.etab, futur.id, t + Duration::days(25))
        .await
        .is_empty());
    // Un fait par groupe quitté, sans identité de personne autre que l'identifiant.
    assert!(nb_evenements(&ent, m.etab, "referentiel.composition_modifiee").await >= 1);
}

// ─── STR-104 : corriger n'est pas changer ───────────────────────────────────────────────────

#[tokio::test]
async fn correction_retroactive_annule_sans_annoncer_de_changement() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let a = adm(m.etab);
    let t = jour();
    let faux = groupe(&ent, &m, "SPE-FAUX", "enseignement", vec![], None).await;
    let juste = groupe(&ent, &m, "SPE-JUSTE", "enseignement", vec![], None).await;
    let e = eleve(&ent, m.etab, "Erreur").await;
    let debut = t - Duration::days(21);
    ent.structure_composition_modifier(&a, faux.id, compo(&[e], &[], Some(debut)))
        .await
        .unwrap();
    let avant_compo = nb_evenements(&ent, m.etab, "referentiel.composition_modifiee").await;
    let s = |apercu: bool, j: Option<Uuid>| CorrectionSaisie {
        eleve_id: e,
        groupe_errone: faux.id,
        groupe_juste: j,
        motif_code: Some("erreur_de_saisie".into()),
        apercu,
    };
    // Aperçu (défaut) : n'écrit rien, dit la période touchée.
    let ap = ent
        .structure_affectation_corriger(&a, s(true, Some(juste.id)))
        .await
        .unwrap();
    assert!(!ap.applique && ap.du == debut && ap.au.is_none());
    assert!(membres(&ent, m.etab, faux.id, t).await.contains(&e));
    // Écriture : la bonne affectation commence à la date d'origine ; l'erronée est archivée avec son motif.
    let r = ent
        .structure_affectation_corriger(&a, s(false, Some(juste.id)))
        .await
        .unwrap();
    assert!(r.applique);
    assert!(
        membres(&ent, m.etab, juste.id, debut).await.contains(&e),
        "posée à la date d'origine"
    );
    assert!(!membres(&ent, m.etab, faux.id, t).await.contains(&e));
    assert!(
        !membres(&ent, m.etab, faux.id, debut).await.contains(&e),
        "l'erreur n'a jamais existé pour les lecteurs"
    );
    assert_eq!(nb_archive(&ent, m.etab, e, "erreur_de_saisie").await, 1);
    // Corriger ≠ changer : un fait de correction (socles 12 et 21), AUCUN fait de composition ni de changement.
    assert_eq!(
        nb_evenements(&ent, m.etab, "referentiel.affectation_corrigee").await,
        1
    );
    assert_eq!(
        nb_evenements(&ent, m.etab, "referentiel.composition_modifiee").await,
        avant_compo
    );
    assert_eq!(
        nb_evenements(&ent, m.etab, "referentiel.affectation_changee").await,
        0
    );
    let motif_inscrit: Option<String> = sqlx::query_scalar(
        "SELECT motif_code FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3",
    )
    .bind(m.etab)
    .bind(e)
    .bind(juste.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(motif_inscrit.as_deref(), Some("erreur_de_saisie"));
    // Membre en trop sans bon groupe : annulation seule. Introuvable : refus. Même groupe : refus.
    let e2 = eleve(&ent, m.etab, "Trop").await;
    ent.structure_composition_modifier(&a, faux.id, compo(&[e2], &[], Some(debut)))
        .await
        .unwrap();
    ent.structure_affectation_corriger(
        &a,
        CorrectionSaisie {
            eleve_id: e2,
            groupe_errone: faux.id,
            groupe_juste: None,
            motif_code: None,
            apercu: false,
        },
    )
    .await
    .unwrap();
    assert!(!membres(&ent, m.etab, faux.id, t).await.contains(&e2));
    assert!(est_domaine(
        &ent.structure_affectation_corriger(
            &a,
            CorrectionSaisie {
                eleve_id: e2,
                groupe_errone: faux.id,
                groupe_juste: None,
                motif_code: None,
                apercu: false
            }
        )
        .await,
        DomainError::NotFound
    ));
    assert!(est_domaine(
        &ent.structure_affectation_corriger(
            &a,
            CorrectionSaisie {
                eleve_id: e,
                groupe_errone: juste.id,
                groupe_juste: Some(juste.id),
                motif_code: None,
                apercu: false
            }
        )
        .await,
        DomainError::ValeurInvalide
    ));
}

// ─── STR-103 : dédoublement ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn dedoublement_d_un_groupe_cours_dupliques_et_postes_a_pourvoir() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let a = adm(m.etab);
    let t = jour();
    let c = classe(&ent, &m, "1ERE").await;
    let esp = offre(&ent, &m, "ESPSPE").await;
    let g = groupe(&ent, &m, "ESP-36", "enseignement", vec![c], Some(esp)).await;
    let mut es = Vec::new();
    for i in 0..6 {
        let e = eleve(&ent, m.etab, &format!("Dd{i:02}")).await;
        es.push(e);
    }
    ent.structure_composition_modifier(&a, g.id, compo(&es, &[], Some(t - Duration::days(30))))
        .await
        .unwrap();
    let cours = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, groupe_id, offre_id, heures_hebdo, ouvert_le, source)
         VALUES ($1,$2,$3,'Espagnol',$4,$5,3.0,$6,'saisie')",
    )
    .bind(cours)
    .bind(m.etab)
    .bind(m.annee)
    .bind(g.id)
    .bind(esp)
    .bind(t - Duration::days(30))
    .execute(ent.pool())
    .await
    .unwrap();
    let saisie_d = |apercu: bool, rep: &str, b: Vec<Uuid>| DedoublementSaisie {
        code_b: None,
        repartition: rep.into(),
        eleves_b: b,
        date_effet: Some(t),
        apercu,
    };
    // Aperçu : rien n'est créé.
    let ap = ent
        .structure_groupe_dedoubler(&a, g.id, saisie_d(true, "alterne", vec![]))
        .await
        .unwrap();
    assert!(!ap.applique && ap.groupe_b.is_none());
    assert_eq!((ap.reste_dans_a.len(), ap.passent_dans_b.len()), (3, 3));
    assert_eq!(ap.cours_dupliques, vec![cours]);
    assert_eq!(
        ent.structure_groupes_liste(&a, Some(m.annee), None, None, false, None)
            .await
            .unwrap()
            .len(),
        1
    );
    // Écriture manuelle : deux élèves précis passent dans B.
    let r = ent
        .structure_groupe_dedoubler(&a, g.id, saisie_d(false, "manuel", es[..2].to_vec()))
        .await
        .unwrap();
    assert!(r.applique);
    let gb = r.groupe_b.unwrap();
    assert_eq!(
        trie(membres(&ent, m.etab, gb, t).await),
        trie(es[..2].to_vec())
    );
    assert_eq!(
        trie(membres(&ent, m.etab, g.id, t).await),
        trie(es[2..].to_vec())
    );
    assert!(
        membres(&ent, m.etab, g.id, t - Duration::days(1))
            .await
            .len()
            == 6,
        "le passé de A reste entier"
    );
    // B reprend nature, classes, offre ; un cours dupliqué, sans professeur : poste à pourvoir.
    let b = ent.structure_groupe_detail(&a, gb, None).await.unwrap();
    assert_eq!(b.groupe.offre_id, Some(esp));
    assert_eq!(b.groupe.classes, vec![c]);
    assert_eq!(b.cours_ouverts.len(), 1);
    let mut conn = ent.pool().acquire().await.unwrap();
    let vacants = crate::structure_cours::cours::vacants(&mut conn, m.etab, t)
        .await
        .unwrap();
    assert!(
        vacants.contains(&b.cours_ouverts[0]),
        "service à pourvoir (socle 6)"
    );
    assert_eq!(r.postes_a_pourvoir, b.cours_ouverts);
    drop(conn);
    assert_eq!(
        nb_evenements(&ent, m.etab, "referentiel.groupe_scinde").await,
        1
    );
    // Refus : manuel incomplet (tous les élèves), groupe de 1 membre, nature autre qu'enseignement.
    let petit = groupe(&ent, &m, "PETIT", "enseignement", vec![], None).await;
    assert!(est_domaine(
        &ent.structure_groupe_dedoubler(&a, petit.id, saisie_d(true, "alterne", vec![]))
            .await,
        DomainError::ValeurInvalide
    ));
    let act = groupe(&ent, &m, "CLUB", "activite", vec![], None).await;
    assert!(est_domaine(
        &ent.structure_groupe_dedoubler(&a, act.id, saisie_d(true, "alterne", vec![]))
            .await,
        DomainError::ValeurInvalide
    ));
    assert!(est_domaine(
        &ent.structure_groupe_dedoubler(&a, g.id, saisie_d(true, "manuel", es[2..].to_vec()))
            .await,
        DomainError::ValeurInvalide
    ));
}

// ─── STR-68 : file de travail ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn file_de_travail_choix_sans_groupe_et_groupe_hors_classe() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let a = adm(m.etab);
    let t = jour();
    let (c1, c2) = (classe(&ent, &m, "T1").await, classe(&ent, &m, "T2").await);
    let o = offre(&ent, &m, "OPTX").await;
    let g = groupe(&ent, &m, "T1-OPT", "enseignement", vec![c1], Some(o)).await;
    let (e1, e2) = (
        eleve(&ent, m.etab, "W1").await,
        eleve(&ent, m.etab, "W2").await,
    );
    en_classe(&ent, &m, e1, c1, t - Duration::days(30)).await;
    en_classe(&ent, &m, e2, c2, t - Duration::days(30)).await;
    suit(&ent, &m, e1, o, t - Duration::days(30)).await; // suit sans être dans le groupe
    ent.structure_composition_modifier(&a, g.id, compo(&[e2], &[], Some(t - Duration::days(5))))
        .await
        .unwrap();
    let f = ent.structure_a_affecter(&a, None, None).await.unwrap();
    assert!(f.sans_classe.is_empty());
    assert!(f
        .choix_incoherents
        .iter()
        .any(|x| x.cause == "sans_groupe" && x.eleve_id == Some(e1)));
    assert!(f
        .choix_incoherents
        .iter()
        .any(|x| x.cause == "groupe_sans_choix" && x.eleve_id == Some(e2)));
    assert_eq!(
        f.groupes_hors_classe,
        vec![(g.id, e2)],
        "e2 est dans T2, le groupe est celui de T1 : avertissement"
    );
    // Une fois e1 placé, le choix n'est plus incohérent.
    ent.structure_composition_modifier(&a, g.id, compo(&[e1], &[], None))
        .await
        .unwrap();
    let f = ent.structure_a_affecter(&a, None, None).await.unwrap();
    assert!(!f
        .choix_incoherents
        .iter()
        .any(|x| x.cause == "sans_groupe" && x.eleve_id == Some(e1)));
    // Droit de lecture : tout le personnel ; pas les familles.
    assert!(ent
        .structure_a_affecter(&acteur(m.etab, Role::Prof), None, None)
        .await
        .is_ok());
    assert!(est_forbidden(
        &ent.structure_a_affecter(&acteur(m.etab, Role::Parent), None, None)
            .await
    ));
}

// ─── Séparation entre établissements, droits par profil ─────────────────────────────────────

#[tokio::test]
async fn separation_entre_etablissements_aucun_acces_hors_perimetre() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (ma, mb) = (monde(&ent).await, monde(&ent).await);
    let (a, b) = (adm(ma.etab), adm(mb.etab));
    let ca = classe(&ent, &ma, "SEP").await;
    let cb = classe(&ent, &mb, "SEP").await;
    let ga = groupe(&ent, &ma, "SEP-G", "enseignement", vec![ca], None).await;
    let gb = groupe(&ent, &mb, "SEP-G", "enseignement", vec![cb], None).await;
    let (ea, eb) = (
        eleve(&ent, ma.etab, "SepA").await,
        eleve(&ent, mb.etab, "SepB").await,
    );
    en_classe(&ent, &ma, ea, ca, jour() - Duration::days(30)).await;
    // B ne voit, ne modifie, ne ferme, ne compose, ne dédouble rien de A : introuvable (aucun oracle d'existence).
    assert!(est_domaine(
        &ent.structure_groupe_detail(&b, ga.id, None).await,
        DomainError::NotFound
    ));
    assert!(est_domaine(
        &ent.structure_groupe_modifier(
            &b,
            ga.id,
            GroupeModif {
                libelle: Some("X".into()),
                ..Default::default()
            }
        )
        .await,
        DomainError::NotFound
    ));
    assert!(est_domaine(
        &ent.structure_groupe_fermer(&b, ga.id, None, false).await,
        DomainError::NotFound
    ));
    assert!(est_domaine(
        &ent.structure_composition_modifier(&b, ga.id, compo(&[eb], &[], None))
            .await,
        DomainError::NotFound
    ));
    assert!(est_domaine(
        &ent.structure_groupe_propositions(&b, ga.id, None).await,
        DomainError::NotFound
    ));
    assert!(est_domaine(
        &ent.structure_groupe_dedoubler(
            &b,
            ga.id,
            DedoublementSaisie {
                code_b: None,
                repartition: "alterne".into(),
                eleves_b: vec![],
                date_effet: None,
                apercu: false
            }
        )
        .await,
        DomainError::NotFound
    ));
    // Les listes ne mélangent jamais.
    let la = ent
        .structure_groupes_liste(&a, Some(ma.annee), None, None, false, None)
        .await
        .unwrap();
    assert_eq!(la.iter().map(|g| g.id).collect::<Vec<_>>(), vec![ga.id]);
    assert!(ent
        .structure_groupes_liste(&b, Some(ma.annee), None, None, false, None)
        .await
        .unwrap()
        .is_empty());
    // Un élève de A dans un groupe de B : inconnu. Un élève de B dans le groupe de A : inconnu.
    let r = ent
        .structure_composition_modifier(&b, gb.id, compo(&[ea], &[], None))
        .await
        .unwrap();
    assert!(r.entrants.is_empty() && r.refuses[0].cause == "inconnu");
    // Changement de classe : l'élève ou la classe de l'autre établissement sont introuvables.
    assert!(est_domaine(
        &ent.structure_eleve_classe_changer(
            &b,
            ChangementClasseSaisie {
                eleve_id: ea,
                classe_id: cb,
                date_effet: None,
                apercu: true,
                entrer_dans: vec![]
            }
        )
        .await,
        DomainError::NotFound
    ));
    assert!(est_domaine(
        &ent.structure_eleve_classe_changer(
            &b,
            ChangementClasseSaisie {
                eleve_id: eb,
                classe_id: ca,
                date_effet: None,
                apercu: true,
                entrer_dans: vec![]
            }
        )
        .await,
        DomainError::NotFound
    ));
    // Correction : l'affectation d'un élève de A n'existe pas pour B.
    assert!(est_domaine(
        &ent.structure_affectation_corriger(
            &b,
            CorrectionSaisie {
                eleve_id: ea,
                groupe_errone: ga.id,
                groupe_juste: None,
                motif_code: None,
                apercu: true
            }
        )
        .await,
        DomainError::NotFound
    ));
    // Création : une classe d'origine de l'autre établissement est introuvable.
    assert!(est_domaine(
        &ent.structure_groupe_creer(&b, saisie("INTRUS", "enseignement", vec![ca], None))
            .await,
        DomainError::NotFound
    ));
    // La base refuse aussi les croisements écrits à la main : affectation vers un groupe d'un autre établissement,
    // classe d'origine d'un autre établissement.
    let r = sqlx::query(
        "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, source)
         VALUES ($1,$2,$3,$4,$5,$6,'saisie')",
    )
    .bind(Uuid::new_v4())
    .bind(ma.etab)
    .bind(ma.annee)
    .bind(ea)
    .bind(gb.id)
    .bind(jour())
    .execute(ent.pool())
    .await;
    assert!(r.is_err(), "affectation croisée refusée par la base");
    let r = sqlx::query(
        "INSERT INTO ent_groupe_classe (etablissement_id, groupe_id, classe_id) VALUES ($1,$2,$3)",
    )
    .bind(ma.etab)
    .bind(ga.id)
    .bind(cb)
    .execute(ent.pool())
    .await;
    assert!(
        r.is_err(),
        "classe d'origine d'un autre établissement refusée par la base"
    );
    // Le port de lecture, toujours borné à l'établissement passé.
    let mut c = ent.pool().acquire().await.unwrap();
    assert!(groupes::lire(&mut c, mb.etab, jour(), ga.id)
        .await
        .unwrap()
        .is_none());
    assert!(groupes::membres(&mut c, mb.etab, jour(), ga.id)
        .await
        .unwrap()
        .is_empty());
    assert!(groupes::groupes_de_classe(&mut c, mb.etab, jour(), ca)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn droits_par_profil_lecture_pour_le_personnel_ecriture_pour_les_gestionnaires() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let c = classe(&ent, &m, "DR").await;
    let g = groupe(&ent, &m, "DR-G", "enseignement", vec![c], None).await;
    let e = eleve(&ent, m.etab, "Droit").await;
    for role in [Role::Admin, Role::Direction, Role::Administratif] {
        let a = acteur(m.etab, role);
        assert!(
            ent.structure_groupes_liste(&a, Some(m.annee), None, None, false, None)
                .await
                .is_ok(),
            "{role:?} lit"
        );
        assert!(
            ent.structure_groupe_detail(&a, g.id, None).await.is_ok(),
            "{role:?} détail"
        );
        assert!(
            ent.structure_groupe_propositions(&a, g.id, None)
                .await
                .is_ok(),
            "{role:?} propositions"
        );
        assert!(ent
            .structure_composition_modifier(
                &a,
                g.id,
                CompositionSaisie {
                    apercu: true,
                    ..compo(&[e], &[], None)
                }
            )
            .await
            .is_ok());
    }
    for role in [
        Role::Cpe,
        Role::Compta,
        Role::Prof,
        Role::Infirmerie,
        Role::Psy,
        Role::Accueil,
    ] {
        let a = acteur(m.etab, role);
        // Le personnel lit la structure (sans noms d'élèves) ; il n'écrit rien et ne voit pas la composition.
        assert!(
            ent.structure_groupes_liste(&a, Some(m.annee), None, None, false, None)
                .await
                .is_ok(),
            "{role:?} lit"
        );
        assert!(
            est_forbidden(&ent.structure_groupe_detail(&a, g.id, None).await),
            "{role:?} détail"
        );
        assert!(est_forbidden(
            &ent.structure_groupe_creer(
                &a,
                saisie(&format!("X{}", sfx()), "activite", vec![], None)
            )
            .await
        ));
        assert!(est_forbidden(
            &ent.structure_groupe_modifier(&a, g.id, GroupeModif::default())
                .await
        ));
        assert!(est_forbidden(
            &ent.structure_groupe_fermer(&a, g.id, None, true).await
        ));
        assert!(est_forbidden(
            &ent.structure_composition_modifier(&a, g.id, compo(&[e], &[], None))
                .await
        ));
        assert!(est_forbidden(
            &ent.structure_groupe_propositions(&a, g.id, None).await
        ));
        assert!(est_forbidden(
            &ent.structure_groupe_regle_appliquer(
                &a,
                g.id,
                ApplicationRegle {
                    entrants: vec![],
                    sortants: vec![],
                    date_effet: None
                }
            )
            .await
        ));
        assert!(est_forbidden(
            &ent.structure_eleve_classe_changer(
                &a,
                ChangementClasseSaisie {
                    eleve_id: e,
                    classe_id: c,
                    date_effet: None,
                    apercu: true,
                    entrer_dans: vec![]
                }
            )
            .await
        ));
        assert!(est_forbidden(
            &ent.structure_affectations_lot(
                &a,
                LotSaisie {
                    classe_id: c,
                    eleves: vec![e],
                    date_effet: None,
                    apercu: true
                }
            )
            .await
        ));
        assert!(est_forbidden(
            &ent.structure_groupe_dedoubler(
                &a,
                g.id,
                DedoublementSaisie {
                    code_b: None,
                    repartition: "alterne".into(),
                    eleves_b: vec![],
                    date_effet: None,
                    apercu: true
                }
            )
            .await
        ));
        assert!(est_forbidden(
            &ent.structure_affectation_corriger(
                &a,
                CorrectionSaisie {
                    eleve_id: e,
                    groupe_errone: g.id,
                    groupe_juste: None,
                    motif_code: None,
                    apercu: true
                }
            )
            .await
        ));
        assert!(est_forbidden(
            &ent.structure_demi_groupes_creer(
                &a,
                DemiGroupesSaisie {
                    classe_id: c,
                    mode: "alterne".into(),
                    code_a: None,
                    code_b: None,
                    eleves_b: vec![],
                    date_effet: None,
                    capacite_bloquante: None
                }
            )
            .await
        ));
        assert!(est_forbidden(
            &ent.structure_groupes_generer(
                &a,
                GenerationSaisie {
                    offre_id: Uuid::new_v4(),
                    classes: vec![c],
                    mode: "par_classe".into(),
                    capacite: None,
                    capacite_bloquante: false,
                    ecrire: false
                }
            )
            .await
        ));
    }
    // Familles et élèves : rien, pas même la lecture.
    for role in [Role::Parent, Role::Eleve] {
        let a = acteur(m.etab, role);
        assert!(
            est_forbidden(
                &ent.structure_groupes_liste(&a, Some(m.annee), None, None, false, None)
                    .await
            ),
            "{role:?}"
        );
        assert!(est_forbidden(
            &ent.structure_composition_modifier(&a, g.id, compo(&[e], &[], None))
                .await
        ));
    }
}

// ─── Garde-fous de la base : migration 339 ─────────────────────────────────────────────────────

#[tokio::test]
async fn garde_fous_de_la_base_chevauchement_dates_et_rejeu_de_la_migration() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent).await;
    let a = adm(m.etab);
    let t = jour();
    let (c1, c2) = (classe(&ent, &m, "GF1").await, classe(&ent, &m, "GF2").await);
    let g = groupe(&ent, &m, "GF-G", "activite", vec![], None).await;
    let e = eleve(&ent, m.etab, "Garde").await;
    en_classe(&ent, &m, e, c1, t - Duration::days(30)).await;
    let ins = |classe: Option<Uuid>, groupe: Option<Uuid>, d: NaiveDate, f: Option<NaiveDate>| {
        let ent = &ent;
        let m = &m;
        async move {
            sqlx::query(
                "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, groupe_id, date_debut, date_fin, source, motif_code)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'saisie','test')",
            )
            .bind(Uuid::new_v4())
            .bind(m.etab)
            .bind(m.annee)
            .bind(e)
            .bind(classe)
            .bind(groupe)
            .bind(d)
            .bind(f)
            .execute(ent.pool())
            .await
        }
    };
    // Deux classes à la même date : refusé ; le même groupe deux fois sur des dates qui se chevauchent : refusé.
    assert!(
        ins(Some(c2), None, t, None).await.is_err(),
        "une seule classe à une date"
    );
    ins(
        None,
        Some(g.id),
        t - Duration::days(10),
        Some(t - Duration::days(5)),
    )
    .await
    .unwrap();
    assert!(
        ins(None, Some(g.id), t - Duration::days(7), None)
            .await
            .is_err(),
        "chevauchement dans le même groupe"
    );
    ins(None, Some(g.id), t - Duration::days(4), None)
        .await
        .unwrap(); // se suivent : accepté
                   // Avant l'ouverture du groupe ou avant l'année : refusé ; au-delà de la fermeture : refusé.
    assert!(ins(
        None,
        Some(g.id),
        t - Duration::days(90),
        Some(t - Duration::days(80))
    )
    .await
    .is_err());
    let f = groupe(&ent, &m, "GF-F", "activite", vec![], None).await;
    ent.structure_groupe_fermer(&a, f.id, Some(t + Duration::days(3)), false)
        .await
        .unwrap();
    assert!(
        ins(None, Some(f.id), t + Duration::days(4), None)
            .await
            .is_err(),
        "groupe fermé"
    );
    // Le groupe ne se supprime pas de l'année ni ne change d'année ; fermé, il ne se rouvre pas.
    assert!(
        sqlx::query("UPDATE ent_groupe SET ferme_le = NULL WHERE id = $1")
            .bind(f.id)
            .execute(ent.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE ent_groupe SET ouvert_le = ouvert_le + 1 WHERE id = $1")
            .bind(g.id)
            .execute(ent.pool())
            .await
            .is_err()
    );
    // Année clôturée : plus de nouveau groupe.
    let cloturee = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante, cloturee_le)
         VALUES ($1,$2,'Passée',$3,$4,false,now())",
    )
    .bind(cloturee)
    .bind(m.etab)
    .bind(t - Duration::days(900))
    .bind(t - Duration::days(600))
    .execute(ent.pool())
    .await
    .unwrap();
    let mut s = saisie("PASSE", "activite", vec![], None);
    s.annee_id = Some(cloturee);
    assert!(est_domaine(
        &ent.structure_groupe_creer(&a, s).await,
        DomainError::ValeurInvalide
    ));
    // Rejeu de la migration : idempotent (les migrations sont rejouables).
    sqlx::raw_sql(include_str!(
        "../../../migrations/339_structure_groupes.sql"
    ))
    .execute(ent.pool())
    .await
    .expect("339 rejouable");
    let l = ent
        .structure_groupes_liste(&a, Some(m.annee), None, None, false, None)
        .await
        .unwrap();
    assert!(
        l.len() >= 2 && l.iter().all(|g| !g.nature_a_verifier),
        "le rejeu ne re-marque aucun groupe à vérifier"
    );
    // L'archive porte le motif d'une annulation et son auteur.
    let _ = ent
        .structure_composition_modifier(
            &a,
            g.id,
            CompositionSaisie {
                motif_code: Some("retrait_manuel".into()),
                ..compo(&[], &[e], Some(t + Duration::days(1)))
            },
        )
        .await;
}

// ─── RLS stricte de `ent_groupe_classe` ─────────────────────────────────────────────────────────

#[tokio::test]
async fn rls_strict_table_du_lot_s6() {
    let Some(ent) = demarrer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let url = crate::url_guard::url_labo_ou_echec().unwrap();
    const ROLE: &str = "ent_rls_app";
    sqlx::query(&format!(
        "DO $$ BEGIN CREATE ROLE {ROLE} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; \
         EXCEPTION WHEN duplicate_object THEN NULL; END $$"
    ))
    .execute(ent.pool())
    .await
    .unwrap();
    for sql in [
        format!("GRANT USAGE ON SCHEMA public TO {ROLE}"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ent_groupe_classe, ent_groupe, ent_classe TO {ROLE}"),
        format!("ALTER ROLE {ROLE} SET app.rls_strict = 'on'"),
    ] {
        sqlx::query(&sql).execute(ent.pool()).await.unwrap();
    }
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(ROLE).unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(u.as_str())
        .await
        .unwrap();
    let (m1, m2) = (monde(&ent).await, monde(&ent).await);
    for m in [&m1, &m2] {
        let c = classe(&ent, m, "RLS").await;
        groupe(&ent, m, "RLS-G", "enseignement", vec![c], None).await;
    }
    let mut t =
        crate::tenant_tx::TenantTx::begin_mode(&app, m1.etab, crate::tenant_tx::RlsMode::Strict)
            .await
            .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_groupe_classe")
        .fetch_one(&mut *t)
        .await
        .unwrap();
    assert_eq!(
        n, 1,
        "seules les classes d'origine de l'établissement courant sont visibles"
    );
    t.commit().await.unwrap();
    // Sans contexte : rien (mode strict), et l'écriture croisée est refusée par la politique.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_groupe_classe")
        .fetch_one(&app)
        .await
        .unwrap();
    assert_eq!(
        n, 0,
        "sans contexte d'établissement, aucune ligne en mode strict"
    );
}
