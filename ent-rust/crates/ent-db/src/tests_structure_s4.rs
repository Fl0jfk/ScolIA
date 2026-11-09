//! Socle 5, lot S4 : l'objet « cours », le poste vacant, le contrat de lecture, les membres
//! individuels rattachés au cours, la reprise de l'existant. Base locale de test (feature
//! `labo`) ; sans `ENT_DATABASE_URL`, ignorés en local et **en échec en CI**.

use super::*;
use crate::ref_commun::Acteur;
use crate::structure_cours::{cours, CoursSaisie};
use crate::tests_d022_bis::{labo, monde, Monde};
use chrono::Duration;
use domain_eleve::{today_utc, CreateEleve};
use ent_auth::{Action, Role};
use std::collections::HashSet;
use uuid::Uuid;

const ADMIN: &str = "administratif";

fn adm(cast: &LaboCast) -> Acteur<'static> {
    Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: cast.administratif.user_id,
        role: Role::Administratif.as_str(),
    }
}

fn prof_acteur(etab: Uuid, user: Uuid) -> Acteur<'static> {
    Acteur {
        etablissement_id: etab,
        user_id: user,
        role: Role::Prof.as_str(),
    }
}

async fn eleve_libre(ent: &PgEnt, etab: Uuid, nom: &str) -> Uuid {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: "S4".into(),
        ine: None,
        sexe: None,
        date_naissance: None,
        numero_badge: None,
        redoublant: false,
        classe: None,
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
    .id
}

async fn suivis(ent: &PgEnt, etab: Uuid, prof: Uuid) -> HashSet<Uuid> {
    ent.eleves_autorises(etab, prof, Role::Prof.as_str(), Action::SuiviEleve)
        .await
        .unwrap()
        .expect("borné")
}

async fn cours_de(ent: &PgEnt, etab: Uuid, service: Uuid) -> Uuid {
    let mut c = ent.pool().acquire().await.unwrap();
    cours::du_service(&mut c, etab, service)
        .await
        .unwrap()
        .expect("service sans cours")
}

fn ens(v: &[Uuid]) -> HashSet<Uuid> {
    v.iter().copied().collect()
}

/// Un service reçoit son cours ; un deuxième professeur sur le même public et la même matière
/// partage le cours (deux services, un cours) ; l'identifiant d'un service reste accepté.
#[tokio::test]
async fn deux_professeurs_sur_la_meme_cible_partagent_un_cours() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = monde(&ent, &cast).await;
    let a = adm(&cast);
    let c = cours_de(&ent, etab, m.s_all).await;
    let detail = ent.structure_cours_detail(&a, c).await.unwrap();
    assert_eq!(detail.cours.nb_services, 1);
    assert_eq!(detail.services.len(), 1);
    assert!(!detail.cours.vacant);
    assert_eq!(
        detail.cours.source, "service",
        "créé par le déclencheur d'un écrivain historique"
    );

    // Co-intervention : le professeur d'espagnol prend aussi le cours d'allemand.
    let s2 = ent
        .structure_cours_pourvoir(&a, c, m.prof_esp, false)
        .await
        .unwrap();
    assert_eq!(cours_de(&ent, etab, s2.service_id).await, c, "même cours");
    assert_eq!(
        ent.structure_cours_detail(&a, c)
            .await
            .unwrap()
            .cours
            .nb_services,
        2
    );
    // Le même professeur ne peut pas être posé deux fois.
    assert!(matches!(
        ent.structure_cours_pourvoir(&a, c, m.prof_esp, false).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::Conflit))
    ));
    // Un utilisateur qui n'est pas du personnel de l'établissement : introuvable.
    assert!(matches!(
        ent.structure_cours_pourvoir(&a, c, Uuid::new_v4(), false)
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Le public du cours est vu des deux professeurs (règle unique).
    let tous: HashSet<_> = ens(&m.all);
    assert!(suivis(&ent, etab, m.prof_all).await.is_superset(&tous));
    assert!(suivis(&ent, etab, m.prof_esp).await.is_superset(&tous));
    // Les identifiants de service du lot S1 restent acceptés pour les membres et le public.
    let t = today_utc();
    let via_service = ent
        .cours_public_pour_acteur(&prof_acteur(etab, m.prof_all), m.s_all, t)
        .await
        .unwrap();
    let via_cours = ent
        .cours_public_pour_acteur(&prof_acteur(etab, m.prof_all), c, t)
        .await
        .unwrap();
    assert_eq!(via_service, via_cours);
    // Cours d'un autre établissement ou inconnu : introuvable, sans oracle.
    assert!(matches!(
        ent.structure_cours_detail(
            &Acteur {
                etablissement_id: Uuid::new_v4(),
                ..a
            },
            c
        )
        .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
}

/// Poste vacant = cours sans service : créé, listé, vu d'aucun professeur, public calculé ; la
/// pose d'un service le pourvoit ; fermeture définitive.
#[tokio::test]
async fn poste_vacant_cours_sans_service() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = monde(&ent, &cast).await;
    let a = adm(&cast);
    let t = today_utc();
    let saisie = || CoursSaisie {
        classe_id: Some(m.classe),
        groupe_id: None,
        matiere_id: None,
        libelle: Some("Soutien de SVT".into()),
        offre_id: None,
        heures_hebdo: Some(1.5),
        semaines: Some("A".into()),
        ouvert_le: None,
    };
    let v = ent.structure_cours_creer(&a, saisie()).await.unwrap();
    assert!(v.vacant && v.nb_services == 0 && v.source == "saisie" && v.heures_hebdo == Some(1.5));
    // Un seul cours ouvert par public et matière : conflit explicite.
    assert!(matches!(
        ent.structure_cours_creer(&a, saisie()).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::Conflit))
    ));
    // Validations : public absent ou double, libellé manquant sans matière, semaines inconnues.
    for mauvais in [
        CoursSaisie {
            classe_id: None,
            ..saisie()
        },
        CoursSaisie {
            groupe_id: Some(Uuid::new_v4()),
            ..saisie()
        },
        CoursSaisie {
            libelle: None,
            ..saisie()
        },
        CoursSaisie {
            semaines: Some("C".into()),
            ..saisie()
        },
        CoursSaisie {
            heures_hebdo: Some(99.0),
            ..saisie()
        },
    ] {
        assert!(ent.structure_cours_creer(&a, mauvais).await.is_err());
    }
    // Public d'une autre classe / établissement : introuvable.
    assert!(ent
        .structure_cours_creer(
            &Acteur {
                etablissement_id: Uuid::new_v4(),
                ..a
            },
            saisie()
        )
        .await
        .is_err());

    // Listé parmi les postes vacants ; aucun professeur ne le voit.
    let vac = ent.structure_cours_vacants(&a).await.unwrap();
    assert!(vac.iter().any(|c| c.id == v.id));
    let liste = ent
        .structure_cours_liste(&a, None, Some(m.classe), None, true, false)
        .await
        .unwrap();
    assert_eq!(liste.iter().map(|c| c.id).collect::<Vec<_>>(), vec![v.id]);
    for p in [m.prof_all, m.prof_esp] {
        assert!(ent
            .public_cours(etab, t, Some(v.id), Some(p), None)
            .await
            .unwrap()
            .is_empty());
    }
    // Son public existe pourtant (la classe entière).
    let mut conn = ent.pool().acquire().await.unwrap();
    let public = cours::public(&mut conn, etab, t, v.id).await.unwrap();
    assert_eq!(
        public.into_iter().collect::<HashSet<_>>(),
        ens(&m.all).union(&ens(&m.esp)).copied().collect()
    );
    drop(conn);

    // Droits : un professeur lit, n'écrit pas ; son détail (avec les professeurs) est refusé.
    let p = prof_acteur(etab, m.prof_all);
    assert!(ent
        .structure_cours_liste(&p, None, None, None, false, false)
        .await
        .is_ok());
    assert!(matches!(
        ent.structure_cours_creer(&p, saisie()).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.structure_cours_detail(&p, v.id).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.structure_cours_fermer(&p, v.id, t).await,
        Err(EntDbError::Forbidden)
    ));

    // Pourvu : plus vacant, vu du professeur posé.
    ent.structure_cours_pourvoir(&a, v.id, m.prof_esp, false)
        .await
        .unwrap();
    assert!(
        !ent.structure_cours_detail(&a, v.id)
            .await
            .unwrap()
            .cours
            .vacant
    );
    assert!(!ent
        .structure_cours_vacants(&a)
        .await
        .unwrap()
        .iter()
        .any(|c| c.id == v.id));
    assert_eq!(
        ent.public_cours(etab, t, Some(v.id), Some(m.prof_esp), None)
            .await
            .unwrap()
            .len(),
        4
    );
    assert!(ent
        .public_cours(etab, t, Some(v.id), Some(m.prof_all), None)
        .await
        .unwrap()
        .is_empty());

    // Modification : libellé et volume ; jamais le public (garde de la base).
    let mod_ = ent
        .structure_cours_modifier(
            &a,
            v.id,
            crate::structure_cours::CoursModif {
                libelle: Some("Soutien SVT".into()),
                offre_id: None,
                heures_hebdo: Some(2.0),
                semaines: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (mod_.libelle.as_str(), mod_.heures_hebdo),
        ("Soutien SVT", Some(2.0))
    );
    for sql in [
        "UPDATE ent_cours SET classe_id = NULL, groupe_id = gen_random_uuid() WHERE id = $1",
        "UPDATE ent_cours SET annee_id = gen_random_uuid() WHERE id = $1",
        "UPDATE ent_cours SET matiere_id = gen_random_uuid() WHERE id = $1",
        "DELETE FROM ent_cours WHERE id = $1",
    ] {
        assert!(
            sqlx::query(sql)
                .bind(v.id)
                .execute(ent.pool())
                .await
                .is_err(),
            "{sql}"
        );
    }

    // Fermeture : définitive, public vide après la date, aucune réouverture ni modification.
    assert!(ent
        .structure_cours_fermer(&a, v.id, t - Duration::days(4000))
        .await
        .is_err());
    let f = ent.structure_cours_fermer(&a, v.id, t).await.unwrap();
    assert_eq!(f.cours.ferme_le, Some(t));
    assert_eq!(
        ent.public_cours(etab, t, Some(v.id), None, None)
            .await
            .unwrap()
            .len(),
        4,
        "dernier jour inclus"
    );
    assert!(ent
        .public_cours(etab, t + Duration::days(1), Some(v.id), None, None)
        .await
        .unwrap()
        .is_empty());
    assert!(matches!(
        ent.structure_cours_fermer(&a, v.id, t).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::Conflit))
    ));
    assert!(
        sqlx::query("UPDATE ent_cours SET ferme_le = NULL WHERE id = $1")
            .bind(v.id)
            .execute(ent.pool())
            .await
            .is_err()
    );
    // Fermé, il laisse la place à un nouveau cours pour le même public (changer de public = fermer + ouvrir).
    assert!(ent.structure_cours_creer(&a, saisie()).await.is_ok());
    // Le journal garde la trace (création, modification, pose, fermeture).
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_structure_journal WHERE etablissement_id = $1 AND objet IN ('cours', 'cours_service')
            AND objet_ref IN ($2, $3)",
    )
    .bind(etab)
    .bind(v.id.to_string())
    .bind(v.id.to_string())
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(n >= 3, "journal : {n}");
}

/// Périmètre du professeur par les cours (D-022) : LV2 mixte, groupe, inclusion et exclusion
/// individuelles. Un membre posé une fois vaut pour tous les professeurs du cours.
#[tokio::test]
async fn professeur_ne_voit_que_les_eleves_de_ses_cours() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = monde(&ent, &cast).await;
    let a = adm(&cast);
    let t = today_utc();
    // LV2 mixte : chaque professeur de langue ne voit que son groupe ; le professeur principal, la classe.
    // (base partagée entre les tests : les mêmes professeurs ont d'autres classes, on contrôle
    // seulement les élèves de ce monde.)
    let (v_all, v_esp) = (
        suivis(&ent, etab, m.prof_all).await,
        suivis(&ent, etab, m.prof_esp).await,
    );
    assert!(
        v_all.is_superset(&ens(&m.all)) && v_all.is_disjoint(&ens(&m.esp)),
        "allemand ne voit pas l'espagnol"
    );
    assert!(
        v_esp.is_superset(&ens(&m.esp)) && v_esp.is_disjoint(&ens(&m.all)),
        "espagnol ne voit pas l'allemand"
    );
    let pp = suivis(&ent, etab, m.prof_principal).await;
    assert!(pp.is_superset(&ens(&m.all)) && pp.is_superset(&ens(&m.esp)));

    let c_all = cours_de(&ent, etab, m.s_all).await;
    // Inclusion individuelle (ULIS) : un élève hors classe suit le cours d'allemand.
    let x = eleve_libre(&ent, etab, "Inclus").await;
    let inc = ent
        .cours_membre_ajouter(
            &a,
            c_all,
            x,
            "inclusion",
            "ulis",
            t - Duration::days(1),
            Some(t + Duration::days(3)),
        )
        .await
        .unwrap();
    assert_eq!(inc.cours_id, c_all);
    assert!(suivis(&ent, etab, m.prof_all).await.contains(&x));
    assert!(
        !suivis(&ent, etab, m.prof_esp).await.contains(&x),
        "l'autre groupe ne le voit pas"
    );
    // Co-intervenant sur le cours d'allemand : voit aussi l'inclus (un membre par cours, plus par ligne).
    ent.structure_cours_pourvoir(&a, c_all, m.prof_esp, false)
        .await
        .unwrap();
    assert!(suivis(&ent, etab, m.prof_esp).await.contains(&x));
    // Exclusion individuelle d'un élève du groupe d'allemand : le titulaire ne le voit plus.
    let exclu = m.all[0];
    let ex = ent
        .cours_membre_ajouter(
            &a,
            c_all,
            exclu,
            "exclusion",
            "echange",
            t - Duration::days(1),
            None,
        )
        .await
        .unwrap();
    assert!(!suivis(&ent, etab, m.prof_all).await.contains(&exclu));
    assert!(suivis(&ent, etab, m.prof_all).await.contains(&m.all[1]));
    // Mais le professeur principal le voit encore par le cours de classe entière.
    assert!(suivis(&ent, etab, m.prof_principal).await.contains(&exclu));
    // Annulation : retour au public d'origine.
    ent.cours_membre_annuler(&a, ex.id).await.unwrap();
    ent.cours_membre_annuler(&a, inc.id).await.unwrap();
    assert!(suivis(&ent, etab, m.prof_all).await.contains(&exclu));
    assert!(!suivis(&ent, etab, m.prof_all).await.contains(&x));
    // Un chevauchement de deux inclusions actives du même cours est refusé.
    ent.cours_membre_ajouter(
        &a,
        c_all,
        x,
        "inclusion",
        "autre",
        t,
        Some(t + Duration::days(5)),
    )
    .await
    .unwrap();
    assert!(ent
        .cours_membre_ajouter(
            &a,
            c_all,
            x,
            "inclusion",
            "autre",
            t + Duration::days(2),
            None
        )
        .await
        .is_err());
    // Journal du membre sans motif (donnée sensible).
    let motif_au_journal: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_structure_journal WHERE etablissement_id = $1 AND objet = 'cours_membre'
            AND (apres::text ILIKE '%ulis%' OR apres::text ILIKE '%echange%')",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(motif_au_journal, 0);
}

/// Reprise de l'existant : chaque service reçoit son cours ; deux services de la même cible =
/// un cours ; les membres suivent ; rejouer ne change rien ; le périmètre est identique avant / après.
#[tokio::test]
async fn reprise_idempotente_sans_perte_et_perimetre_identique() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m: Monde = monde(&ent, &cast).await;
    let a = adm(&cast);
    let t = today_utc();
    let c_all = cours_de(&ent, etab, m.s_all).await;
    // Un service co-intervenant, un membre individuel et un créneau qui vise le service.
    ent.structure_cours_pourvoir(&a, c_all, m.prof_esp, false)
        .await
        .unwrap();
    let x = eleve_libre(&ent, etab, "Repris").await;
    let membre = ent
        .cours_membre_ajouter(
            &a,
            c_all,
            x,
            "inclusion",
            "decloisonnement",
            t - Duration::days(1),
            None,
        )
        .await
        .unwrap();
    let creneau = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine,
                heure_debut, heure_fin, service_id)
         VALUES ($1,$2,'EB',$3,$4,2,'08:00','09:00',$5)",
    )
    .bind(creneau)
    .bind(etab)
    .bind("Allemand")
    .bind(m.prof_all)
    .bind(m.s_all)
    .execute(ent.pool())
    .await
    .unwrap();
    let cr: Option<Uuid> = sqlx::query_scalar("SELECT cours_id FROM ent_edt_creneau WHERE id = $1")
        .bind(creneau)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(cr, Some(c_all), "le créneau hérite du cours de son service");

    let profs = [m.prof_all, m.prof_esp, m.prof_principal];
    let mut avant = Vec::new();
    for p in profs {
        avant.push(suivis(&ent, etab, p).await);
    }
    let etat_avant = ent.structure_cours_reprise(&a).await.unwrap();
    assert_eq!(etat_avant.services, etat_avant.services_avec_cours);

    // Retour à l'état d'avant le lot S4 pour cet établissement.
    let mut tx = ent.pool().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_cours_membre DISABLE TRIGGER ent_cours_membre_coherence")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE ent_cours_membre DROP CONSTRAINT IF EXISTS ent_cours_membre_cours_fk")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE ent_cours_membre mb SET service_origine_id = NULL,
                cours_id = (SELECT s.id FROM ent_service_enseignement s
                             WHERE s.etablissement_id = mb.etablissement_id AND s.cours_id = mb.cours_id
                             ORDER BY s.created_at LIMIT 1)
          WHERE mb.etablissement_id = $1 AND EXISTS (SELECT 1 FROM ent_cours c WHERE c.id = mb.cours_id)",
    )
    .bind(etab)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("ALTER TABLE ent_cours_membre ENABLE TRIGGER ent_cours_membre_coherence")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    // (la clé étrangère retirée est rétablie par `migrate`, comme au démarrage de l'application)
    sqlx::query("ALTER TABLE ent_service_enseignement DISABLE TRIGGER ent_service_cours")
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_service_enseignement SET cours_id = NULL WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    // Deux services « d'avant » sur une même classe et une même matière (aucun cours encore) :
    // la reprise doit en faire UN cours et DEUX services.
    let (annee, matiere): (Uuid, Option<Uuid>) =
        sqlx::query_as("SELECT annee_id, matiere_id FROM ent_service_enseignement WHERE id = $1")
            .bind(m.s_all)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    let sfx = Uuid::new_v4().simple().to_string()[..6].to_string();
    let classe_legacy = ent
        .referentiel_creer_classe(
            ADMIN,
            etab,
            Some(annee),
            &format!("EL{sfx}"),
            Some("4e"),
            Some("4EME"),
        )
        .await
        .unwrap();
    let mut legacy = Vec::new();
    for p in [m.prof_all, m.prof_esp] {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_service_enseignement (id, etablissement_id, annee_id, prof_user_id, matiere_id, matiere_libelle, classe_id)
             VALUES ($1,$2,$3,$4,$5,'',$6)",
        )
        .bind(id)
        .bind(etab)
        .bind(annee)
        .bind(p)
        .bind(matiere)
        .bind(classe_legacy.id)
        .execute(ent.pool())
        .await
        .unwrap();
        legacy.push(id);
    }
    sqlx::query("ALTER TABLE ent_service_enseignement ENABLE TRIGGER ent_service_cours")
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_edt_creneau SET cours_id = NULL WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();

    // Reprise (au démarrage : `migrate`) : tout est repris, une deuxième exécution ne fait rien.
    let n1: i64 = sqlx::query_scalar("SELECT ent_cours_reprendre()")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(
        n1 >= 6,
        "au moins les services de ce monde et les deux « d'avant » : {n1}"
    );
    let (cl0, cl1) = (
        cours_de(&ent, etab, legacy[0]).await,
        cours_de(&ent, etab, legacy[1]).await,
    );
    assert_eq!(
        cl0, cl1,
        "deux professeurs, même classe, même matière : un seul cours"
    );
    let dl = ent.structure_cours_detail(&a, cl0).await.unwrap();
    assert_eq!(
        (dl.services.len(), dl.cours.source.as_str()),
        (2, "reprise")
    );
    let n2: i64 = sqlx::query_scalar("SELECT ent_cours_reprendre()")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n2, 0, "idempotente");
    let etat = ent.structure_cours_reprise(&a).await.unwrap();
    assert_eq!(
        etat.services, etat.services_avec_cours,
        "aucun service sans cours"
    );
    assert_eq!(etat.services_a_traiter, 0);
    assert_eq!(
        etat.services,
        etat_avant.services + 2,
        "aucun service perdu ni créé (hors les deux ajoutés pour le test)"
    );
    assert_eq!(etat.membres, etat_avant.membres, "aucun membre perdu");
    assert_eq!(
        etat.membres_sur_un_cours, etat.membres,
        "tous rattachés à un cours"
    );
    assert_eq!(etat.creneaux, etat_avant.creneaux);
    assert_eq!(etat.creneaux_sans_cours, etat_avant.creneaux_sans_cours);
    // Cours dédoublonnés : les deux services d'allemand partagent le cours d'origine (même clé).
    let c2 = cours_de(&ent, etab, m.s_all).await;
    assert_eq!(
        ent.structure_cours_detail(&a, c2)
            .await
            .unwrap()
            .services
            .len(),
        2
    );
    // Le membre est sur le cours, avec la trace de la ligne de service d'origine.
    let (cid, origine): (Uuid, Option<Uuid>) =
        sqlx::query_as("SELECT cours_id, service_origine_id FROM ent_cours_membre WHERE id = $1")
            .bind(membre.id)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(cid, c2);
    assert!(origine.is_some());
    // Le périmètre de chaque professeur est identique avant / après la reprise.
    for (i, p) in profs.iter().enumerate() {
        assert_eq!(
            suivis(&ent, etab, *p).await,
            avant[i],
            "périmètre du professeur {i}"
        );
    }
    // Rétablir la clé étrangère retirée par la simulation (la migration la recrée à chaque démarrage).
    ent.migrate().await.unwrap();
}
