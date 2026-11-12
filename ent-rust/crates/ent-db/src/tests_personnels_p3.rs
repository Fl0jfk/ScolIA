//! Socle 6, lot P3 : services d'enseignement et interventions (migrations 354 et 362), contre la
//! base locale de test. Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! * suppléance : une saisie par jour, le titulaire retrouve tout au retour ;
//! * co-enseignement : toutes les saisies ; assistant et observateur ne saisissent jamais ;
//! * service vacant, alertes, événements ; correction d'intervention sans perte d'historique ;
//! * remplacement d'un service et d'un agent ; annulation refusée si saisies ;
//! * projection `prof_user_id` (écrivains historiques inchangés) ; service prévu sans effet ;
//! * cloisonnement par établissement, matrice de droits ; l'agent comme sujet (362).

use super::*;
use chrono::{Duration, NaiveDate};
use ent_types::personnels_services::*;
use uuid::Uuid;

const MDP: &str = "MotDePasse-P3-Aa9!";

async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let email = format!("p3-{role}-{}@ent.local", Uuid::new_v4().simple());
    ent.register_user(
        &email,
        MDP,
        Some(&format!(
            "Nom {role} {}",
            &Uuid::new_v4().simple().to_string()[..6]
        )),
        etab,
        role,
    )
    .await
    .expect("compte")
    .id
}

async fn agent_de(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    sqlx::query_scalar(
        "SELECT a.id FROM ent_agent a JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND p.user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .fetch_one(ent.pool())
    .await
    .expect("agent du compte")
}

struct Cadre {
    ent: PgEnt,
    etab: Uuid,
    dir: Uuid,
    sec: Uuid,
    prof: Uuid,
    prof_user: Uuid,
    prof2: Uuid,
    prof2_user: Uuid,
    prof3: Uuid,
    annee: Uuid,
    debut: NaiveDate,
    classe: Uuid,
    matiere: Uuid,
}

impl Cadre {
    fn d(&self, n: i64) -> NaiveDate {
        self.debut + Duration::days(n)
    }

    async fn nouveau() -> Option<Cadre> {
        let ent = connecter().await?;
        let etab = Uuid::new_v4();
        let dir = compte(&ent, etab, "direction").await;
        let sec = compte(&ent, etab, "administratif").await;
        let prof_user = compte(&ent, etab, "prof").await;
        let prof2_user = compte(&ent, etab, "prof").await;
        let prof3_user = compte(&ent, etab, "prof").await;
        let prof = agent_de(&ent, etab, prof_user).await;
        let prof2 = agent_de(&ent, etab, prof2_user).await;
        let prof3 = agent_de(&ent, etab, prof3_user).await;
        let annee = ent.referentiel_annee_courante_assuree(etab).await.unwrap();
        let sfx = Uuid::new_v4().simple().to_string()[..6].to_string();
        let classe = ent
            .referentiel_creer_classe(
                "administratif",
                etab,
                Some(annee.id),
                &format!("P3{sfx}"),
                Some("4e"),
                Some("4EME"),
            )
            .await
            .unwrap()
            .id;
        let matiere = ent
            .create_matiere(
                CreateMatiere {
                    etablissement_id: etab,
                    code: format!("M{sfx}"),
                    libelle: "Mathématiques".into(),
                    source_nomenclature: None,
                },
                None,
            )
            .await
            .unwrap()
            .id;
        Some(Cadre {
            ent,
            etab,
            dir,
            sec,
            prof,
            prof_user,
            prof2,
            prof2_user,
            prof3,
            annee: annee.id,
            debut: annee.date_debut,
            classe,
            matiere,
        })
    }

    async fn service(&self, titulaire: Option<Uuid>) -> ServiceDto {
        self.service_sur(self.classe, self.matiere, titulaire).await
    }

    async fn service_sur(
        &self,
        classe: Uuid,
        matiere: Uuid,
        titulaire: Option<Uuid>,
    ) -> ServiceDto {
        self.ent
            .creer_service_for_actor(
                self.etab,
                self.dir,
                "direction",
                &CreerServiceBody {
                    annee_id: Some(self.annee),
                    classe_id: Some(classe),
                    matiere_id: Some(matiere),
                    heures_hebdo: Some(4.0),
                    titulaire: titulaire.map(|a| TitulaireInitialBody {
                        agent_id: a,
                        du: Some(self.debut),
                        heures_hebdo: None,
                    }),
                    ..Default::default()
                },
            )
            .await
            .expect("service")
    }

    async fn classe2(&self) -> Uuid {
        self.ent
            .referentiel_creer_classe(
                "administratif",
                self.etab,
                Some(self.annee),
                &format!("Q{}", &Uuid::new_v4().simple().to_string()[..6]),
                Some("3e"),
                Some("3EME"),
            )
            .await
            .unwrap()
            .id
    }

    async fn en_saisie(&self, service: Uuid, jour: NaiveDate) -> Vec<Uuid> {
        let mut v: Vec<Uuid> = self
            .ent
            .en_saisie(self.etab, service, jour)
            .await
            .unwrap()
            .into_iter()
            .map(|i| i.agent_id)
            .collect();
        v.sort();
        v
    }

    async fn prof_user_du_service(&self, service: Uuid) -> Option<Uuid> {
        sqlx::query_scalar("SELECT prof_user_id FROM ent_service_enseignement WHERE id = $1")
            .bind(service)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    async fn evenements(&self, type_name: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = $2",
        )
        .bind(self.etab)
        .bind(type_name)
        .fetch_one(self.ent.pool())
        .await
        .unwrap()
    }
}

fn est_conflit(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Refus(RefusMetier::Conflit, _))
}

fn est_invalide(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Refus(RefusMetier::Invalide, _))
}

// ─────────────────────────────── création, projection, vacance ───────────────────────────────

#[tokio::test]
async fn service_avec_titulaire_projette_le_professeur_historique() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let s = c.service(Some(c.prof)).await;
    assert_eq!(s.etat, "ouvert");
    assert_eq!(s.source, "saisie");
    assert_eq!(s.modalite, "cours");
    assert!(
        s.cours_id.is_some(),
        "le cours du socle 5 est trouvé ou créé"
    );
    assert!(!s.vacant);
    assert_eq!(s.intervenants.len(), 1);
    assert_eq!(s.intervenants[0].nature, "titulaire");
    assert!(s.intervenants[0].saisie_effective);
    // Lecteurs historiques : `prof_user_id` = projection du titulaire ; la liste de classe le voit.
    assert_eq!(c.prof_user_du_service(s.id).await, Some(c.prof_user));
    let lus = c
        .ent
        .referentiel_services_de_classe(c.etab, c.classe)
        .await
        .unwrap();
    assert!(lus
        .iter()
        .any(|x| x.id == s.id && x.prof_user_id == c.prof_user));
    assert_eq!(c.evenements("service.cree").await, 1);
    assert_eq!(c.evenements("service.intervenant_ajoute").await, 1);
    assert_eq!(c.evenements("service.vacant").await, 0);
}

#[tokio::test]
async fn service_vacant_alerte_puis_pourvu_par_un_titulaire() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let s = c.service(None).await;
    assert!(s.vacant);
    assert!(s.intervenants.is_empty());
    assert!(s.alertes.iter().any(|a| a.code == "vacant"));
    assert_eq!(c.prof_user_du_service(s.id).await, None);
    assert_eq!(c.evenements("service.vacant").await, 1);
    // Un service vacant ne casse pas les lecteurs historiques (il n'y figure pas).
    let lus = c
        .ent
        .referentiel_services_de_classe(c.etab, c.classe)
        .await
        .unwrap();
    assert!(!lus.iter().any(|x| x.id == s.id));
    // La répartition le range parmi les vacants.
    let rep = c
        .ent
        .repartition_for_actor(
            c.etab,
            c.sec,
            "administratif",
            &FiltreServices {
                vacants: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(rep.services.iter().any(|x| x.id == s.id));
    assert!(rep.vacants >= 1);

    let s = c
        .ent
        .ajouter_intervenant_for_actor(
            c.etab,
            c.sec,
            "administratif",
            s.id,
            &AjouterIntervenantBody {
                agent_id: c.prof,
                nature: "titulaire".into(),
                du: Some(c.debut),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(!s.vacant);
    assert_eq!(c.prof_user_du_service(s.id).await, Some(c.prof_user));
    assert_eq!(c.evenements("service.pourvu").await, 1);
}

#[tokio::test]
async fn ecrivain_historique_recoit_son_titulaire_et_ne_change_plus_le_professeur_a_la_main() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_service_enseignement (id, etablissement_id, annee_id, prof_user_id, matiere_id, matiere_libelle, classe_id)
         VALUES ($1,$2,$3,$4,$5,'Maths',$6)",
    )
    .bind(id)
    .bind(c.etab)
    .bind(c.annee)
    .bind(c.prof_user)
    .bind(c.matiere)
    .bind(c.classe)
    .execute(c.ent.pool())
    .await
    .unwrap();
    let i = c
        .ent
        .intervenants_du_service(c.etab, id, c.d(10))
        .await
        .unwrap();
    assert_eq!(
        i.len(),
        1,
        "le déclencheur pose l'intervention du titulaire"
    );
    assert_eq!(i[0].nature, "titulaire");
    assert_eq!(i[0].user_id, Some(c.prof_user));
    // Changer `prof_user_id` à la main est refusé : on change le titulaire.
    let r = sqlx::query("UPDATE ent_service_enseignement SET prof_user_id = $2 WHERE id = $1")
        .bind(id)
        .bind(c.prof2_user)
        .execute(c.ent.pool())
        .await;
    assert!(r.is_err(), "prof_user_id est une projection");
    // La migration rejouée ne duplique rien.
    c.ent.migrate().await.unwrap();
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_service_intervenant WHERE service_id = $1")
            .bind(id)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn service_prevu_sans_effet_sur_les_droits_jusqu_a_son_ouverture() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let s = c
        .ent
        .creer_service_for_actor(
            c.etab,
            c.dir,
            "direction",
            &CreerServiceBody {
                annee_id: Some(c.annee),
                classe_id: Some(c.classe),
                matiere_id: Some(c.matiere),
                etat: Some("prevu".into()),
                titulaire: Some(TitulaireInitialBody {
                    agent_id: c.prof,
                    du: Some(c.debut),
                    heures_hebdo: None,
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(s.etat, "prevu");
    assert!(!s.vacant, "un service prévu n'est pas un service vacant");
    assert_eq!(
        c.prof_user_du_service(s.id).await,
        None,
        "aucun droit pour les lecteurs historiques"
    );
    let s = c
        .ent
        .ouvrir_service_for_actor(c.etab, c.dir, "direction", s.id)
        .await
        .unwrap();
    assert_eq!(s.etat, "ouvert");
    assert_eq!(c.prof_user_du_service(s.id).await, Some(c.prof_user));
    // Un service ouvert ne redevient pas prévu.
    let r = sqlx::query("UPDATE ent_service_enseignement SET etat = 'prevu' WHERE id = $1")
        .bind(s.id)
        .execute(c.ent.pool())
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn validations_de_creation() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let base = || CreerServiceBody {
        annee_id: Some(c.annee),
        classe_id: Some(c.classe),
        matiere_id: Some(c.matiere),
        ..Default::default()
    };
    let essaie = |b: CreerServiceBody| {
        let ent = &c.ent;
        let (etab, dir) = (c.etab, c.dir);
        async move {
            ent.creer_service_for_actor(etab, dir, "direction", &b)
                .await
        }
    };
    assert!(est_invalide(
        &essaie(CreerServiceBody {
            modalite: Some("zzz".into()),
            ..base()
        })
        .await
        .unwrap_err()
    ));
    assert!(est_invalide(
        &essaie(CreerServiceBody {
            heures_hebdo: Some(99.0),
            ..base()
        })
        .await
        .unwrap_err()
    ));
    assert!(est_invalide(
        &essaie(CreerServiceBody {
            heures_hebdo: Some(f64::NAN),
            ..base()
        })
        .await
        .unwrap_err()
    ));
    assert!(est_invalide(
        &essaie(CreerServiceBody {
            du: Some(c.d(20)),
            au: Some(c.d(10)),
            ..base()
        })
        .await
        .unwrap_err()
    ));
    assert!(est_invalide(
        &essaie(CreerServiceBody {
            groupe_id: Some(Uuid::new_v4()),
            ..base()
        })
        .await
        .unwrap_err()
    ));
    assert!(est_invalide(
        &essaie(CreerServiceBody {
            matiere_id: None,
            ..base()
        })
        .await
        .unwrap_err()
    ));
    assert!(est_invalide(
        &essaie(CreerServiceBody {
            etat: Some("clos".into()),
            ..base()
        })
        .await
        .unwrap_err()
    ));
    // Matière ou classe d'un autre établissement : 404 uniforme.
    let e = essaie(CreerServiceBody {
        matiere_id: Some(Uuid::new_v4()),
        ..base()
    })
    .await
    .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound)),
        "{e:?}"
    );
    // Deux fois le même service (même cible, matière, modalité, code, titulaire) : conflit ;
    // une autre modalité (TP) coexiste.
    let s1 = c.service(Some(c.prof)).await;
    assert!(s1.id != Uuid::nil());
    let e = c
        .ent
        .creer_service_for_actor(
            c.etab,
            c.dir,
            "direction",
            &CreerServiceBody {
                titulaire: Some(TitulaireInitialBody {
                    agent_id: c.prof,
                    du: Some(c.debut),
                    heures_hebdo: None,
                }),
                ..base()
            },
        )
        .await;
    // Même cible, matière, modalité, code ET même titulaire : doublon refusé (409), jamais une panne.
    assert!(est_conflit(&e.unwrap_err()));
    let tp = essaie(CreerServiceBody {
        modalite: Some("tp".into()),
        code: Some("TP1".into()),
        ..base()
    })
    .await
    .unwrap();
    assert_eq!(tp.modalite, "tp");
    assert_eq!(tp.code.as_deref(), Some("TP1"));
}

// ─────────────────────────────── suppléance, co-enseignement ───────────────────────────────

#[tokio::test]
async fn suppleance_une_saisie_par_jour_et_le_titulaire_retrouve_tout() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let s = c.service(Some(c.prof)).await;
    let (du, au) = (c.d(20), c.d(30));
    let s = c
        .ent
        .nommer_suppleant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &NommerSuppleantBody {
                agent_id: c.prof2,
                du,
                au: Some(au),
                origine: None,
                origine_ref: Some(Uuid::new_v4()),
            },
        )
        .await
        .unwrap();
    let supp = s
        .intervenants
        .iter()
        .find(|i| i.nature == "suppleant")
        .unwrap();
    assert_eq!(
        supp.origine, "absence",
        "titulaire présent : suppléance d'absence"
    );
    assert!(supp.saisie);
    // Avant, pendant, après : une seule saisie par jour.
    assert_eq!(c.en_saisie(s.id, c.d(19)).await, vec![c.prof]);
    assert_eq!(
        c.en_saisie(s.id, du).await,
        vec![c.prof2],
        "le titulaire n'a plus la saisie"
    );
    assert_eq!(c.en_saisie(s.id, au).await, vec![c.prof2]);
    assert_eq!(
        c.en_saisie(s.id, c.d(31)).await,
        vec![c.prof],
        "le titulaire retrouve tout au retour"
    );
    // Le titulaire reste intervenant (lecture) pendant la suppléance ; seule sa saisie est retirée.
    let pendant = c
        .ent
        .intervenants_du_service(c.etab, s.id, c.d(25))
        .await
        .unwrap();
    assert_eq!(pendant.len(), 2);
    assert!(pendant
        .iter()
        .any(|i| i.agent_id == c.prof && !i.saisie_effective));
    // Un deuxième suppléant sur les mêmes jours est refusé (une saisie par jour).
    let e = c
        .ent
        .nommer_suppleant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &NommerSuppleantBody {
                agent_id: c.prof3,
                du: c.d(25),
                au: Some(c.d(40)),
                origine: None,
                origine_ref: None,
            },
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
    // Suppléer sa propre classe en même temps qu'on la tient : refusé.
    let e = c
        .ent
        .nommer_suppleant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &NommerSuppleantBody {
                agent_id: c.prof,
                du: c.d(40),
                au: None,
                origine: None,
                origine_ref: None,
            },
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
    // Fin anticipée : le suppléant s'arrête, le titulaire reprend dès le lendemain.
    let s = c
        .ent
        .terminer_intervention_for_actor(
            c.etab,
            c.dir,
            "direction",
            supp.id,
            &TerminerInterventionBody { au: Some(c.d(22)) },
        )
        .await
        .unwrap();
    assert_eq!(c.en_saisie(s.id, c.d(23)).await, vec![c.prof]);
    assert_eq!(c.evenements("service.suppleance_ouverte").await, 1);
    assert_eq!(c.evenements("service.suppleance_close").await, 1);
    // Le professeur historique reste le titulaire : le suppléant n'ouvre aucun droit avant le lot P4.
    assert_eq!(c.prof_user_du_service(s.id).await, Some(c.prof_user));
}

#[tokio::test]
async fn suppleance_d_un_service_vacant_est_une_suppleance_de_vacance() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let s = c.service(None).await;
    let s = c
        .ent
        .nommer_suppleant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &NommerSuppleantBody {
                agent_id: c.prof2,
                du: c.debut,
                au: None,
                origine: None,
                origine_ref: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(s.intervenants[0].origine, "vacance");
    assert!(!s.vacant, "un suppléant en saisie pourvoit le service");
    assert_eq!(c.en_saisie(s.id, c.d(5)).await, vec![c.prof2]);
}

#[tokio::test]
async fn co_enseignement_toutes_les_saisies_assistant_et_observateur_jamais() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let s = c.service(Some(c.prof)).await;
    for (agent, nature) in [(c.prof2, "co_enseignant"), (c.prof3, "assistant")] {
        c.ent
            .ajouter_intervenant_for_actor(
                c.etab,
                c.dir,
                "direction",
                s.id,
                &AjouterIntervenantBody {
                    agent_id: agent,
                    nature: nature.into(),
                    du: Some(c.debut),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
    }
    let mut attendu = vec![c.prof, c.prof2];
    attendu.sort();
    assert_eq!(
        c.en_saisie(s.id, c.d(5)).await,
        attendu,
        "titulaire et co-enseignant saisissent, pas l'assistant"
    );
    // Un suppléant retire la saisie au titulaire, pas au co-enseignant.
    let autre = compte(&c.ent, c.etab, "prof").await;
    let autre = agent_de(&c.ent, c.etab, autre).await;
    c.ent
        .nommer_suppleant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &NommerSuppleantBody {
                agent_id: autre,
                du: c.d(10),
                au: Some(c.d(12)),
                origine: None,
                origine_ref: None,
            },
        )
        .await
        .unwrap();
    let mut pendant = vec![c.prof2, autre];
    pendant.sort();
    assert_eq!(c.en_saisie(s.id, c.d(11)).await, pendant);
    // La base refuse qu'un assistant porte une saisie.
    let r = sqlx::query(
        "INSERT INTO ent_service_intervenant (id, etablissement_id, service_id, agent_id, nature, du, saisie)
         VALUES (gen_random_uuid(), $1, $2, $3, 'stagiaire_observateur', $4, true)",
    )
    .bind(c.etab)
    .bind(s.id)
    .bind(autre)
    .bind(c.d(100))
    .execute(c.ent.pool())
    .await;
    assert!(r.is_err());
    // Un second titulaire en même temps : conflit lisible.
    let e = c
        .ent
        .ajouter_intervenant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &AjouterIntervenantBody {
                agent_id: autre,
                nature: "titulaire".into(),
                du: Some(c.d(50)),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
    // Droit de saisie inconnu, nature inconnue, suppléant par la voie ordinaire.
    for (nature, droits) in [
        ("zzz", None),
        ("suppleant", None),
        ("intervenant", Some(vec!["admin".to_string()])),
    ] {
        let e = c
            .ent
            .ajouter_intervenant_for_actor(
                c.etab,
                c.dir,
                "direction",
                s.id,
                &AjouterIntervenantBody {
                    agent_id: autre,
                    nature: nature.into(),
                    du: Some(c.d(60)),
                    droits,
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
        assert!(est_invalide(&e), "{nature} : {e:?}");
    }
}

// ─────────────────────────────── corrections, remplacements, annulation ───────────────────────────────

#[tokio::test]
async fn correction_d_intervention_sans_perte_d_historique_ni_d_auteur() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let s = c.service(Some(c.prof)).await;
    let erreur = s.intervenants[0].id;
    // Erreur de saisie : le titulaire est en fait prof2.
    let s2 = c
        .ent
        .corriger_intervention_for_actor(
            c.etab,
            c.sec,
            "administratif",
            erreur,
            &CorrigerInterventionBody {
                motif: "erreur_saisie".into(),
                remplacement: Some(InterventionCorrigeeBody {
                    agent_id: c.prof2,
                    nature: "titulaire".into(),
                    du: c.debut,
                    au: None,
                    heures_hebdo: None,
                }),
            },
        )
        .await
        .unwrap();
    assert_eq!(s2.intervenants.len(), 2, "l'erreur reste visible, annulée");
    assert!(s2
        .intervenants
        .iter()
        .any(|i| i.id == erreur && i.annulee && !i.active));
    assert_eq!(c.en_saisie(s.id, c.d(5)).await, vec![c.prof2]);
    assert_eq!(
        c.prof_user_du_service(s.id).await,
        Some(c.prof2_user),
        "la projection suit la correction"
    );
    // L'auteur réel reste celui qui a créé la ligne ; l'annulation a son propre auteur.
    let (cree_par, annule_par, remplace): (Option<Uuid>, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
        "SELECT (SELECT cree_par_user_id FROM ent_service_intervenant WHERE id = $1),
                (SELECT annule_par_user_id FROM ent_service_intervenant WHERE id = $1),
                (SELECT remplace_intervention_id FROM ent_service_intervenant WHERE agent_id = $2 AND service_id = $3 AND annule_le IS NULL)",
    )
    .bind(erreur)
    .bind(c.prof2)
    .bind(s.id)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(cree_par, Some(c.dir));
    assert_eq!(annule_par, Some(c.sec));
    assert_eq!(remplace, Some(erreur));
    // Corriger deux fois : conflit ; supprimer : refusé par la base ; modifier l'agent : refusé.
    let e = c
        .ent
        .corriger_intervention_for_actor(
            c.etab,
            c.sec,
            "administratif",
            erreur,
            &CorrigerInterventionBody {
                motif: "doublon".into(),
                remplacement: None,
            },
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
    assert!(
        sqlx::query("DELETE FROM ent_service_intervenant WHERE id = $1")
            .bind(erreur)
            .execute(c.ent.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE ent_service_intervenant SET agent_id = $2 WHERE id = $1")
            .bind(erreur)
            .bind(c.prof3)
            .execute(c.ent.pool())
            .await
            .is_err()
    );
    let e = c
        .ent
        .corriger_intervention_for_actor(
            c.etab,
            c.sec,
            "administratif",
            s2.intervenants.iter().find(|i| !i.annulee).unwrap().id,
            &CorrigerInterventionBody {
                motif: "zzz".into(),
                remplacement: None,
            },
        )
        .await
        .unwrap_err();
    assert!(est_invalide(&e));
    assert_eq!(c.evenements("service.intervention_corrigee").await, 1);
}

#[tokio::test]
async fn remplacer_un_agent_sur_tous_ses_services_apercu_puis_application() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let classe2 = c.classe2().await;
    let s1 = c.service(Some(c.prof)).await;
    let s2 = c.service_sur(classe2, c.matiere, Some(c.prof)).await;
    let corps = |apercu| RemplacerAgentBody {
        agent_sortant_id: c.prof,
        agent_entrant_id: c.prof3,
        a_partir_du: c.d(60),
        service_ids: None,
        apercu,
    };
    let apercu = c
        .ent
        .remplacer_agent_for_actor(c.etab, c.dir, "direction", &corps(true))
        .await
        .unwrap();
    assert_eq!(apercu.lignes.len(), 2);
    assert!(apercu.lignes.iter().all(|l| l.resultat == "a_faire"));
    assert_eq!(
        c.en_saisie(s1.id, c.d(70)).await,
        vec![c.prof],
        "l'aperçu ne change rien"
    );
    let r = c
        .ent
        .remplacer_agent_for_actor(c.etab, c.dir, "direction", &corps(false))
        .await
        .unwrap();
    assert_eq!(r.lignes.iter().filter(|l| l.resultat == "fait").count(), 2);
    for s in [&s1, &s2] {
        assert_eq!(
            c.en_saisie(s.id, c.d(59)).await,
            vec![c.prof],
            "avant la date : le sortant"
        );
        assert_eq!(
            c.en_saisie(s.id, c.d(60)).await,
            vec![c.prof3],
            "à la date : le successeur"
        );
    }
    // Rejouer : le successeur est déjà intervenant, plus rien à reprendre pour le sortant.
    let r2 = c
        .ent
        .remplacer_agent_for_actor(c.etab, c.dir, "direction", &corps(false))
        .await
        .unwrap();
    assert!(r2.lignes.iter().all(|l| l.resultat != "fait"), "{r2:?}");
    // Un agent ne se remplace pas lui-même.
    let e = c
        .ent
        .remplacer_agent_for_actor(
            c.etab,
            c.dir,
            "direction",
            &RemplacerAgentBody {
                agent_entrant_id: c.prof,
                ..corps(false)
            },
        )
        .await
        .unwrap_err();
    assert!(est_invalide(&e));
    assert_eq!(c.evenements("service.intervenant_termine").await, 2);
}

#[tokio::test]
async fn changer_la_cible_en_cours_d_annee_clot_l_ancien_et_reprend_les_intervenants() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let s = c.service(Some(c.prof)).await;
    let classe2 = c.classe2().await;
    let r = c
        .ent
        .remplacer_service_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &RemplacerServiceBody {
                a_partir_du: c.d(40),
                classe_id: Some(classe2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(r.nouveau.remplace_service_id, Some(s.id));
    assert_eq!(r.nouveau.classe_id, Some(classe2));
    assert_eq!(r.nouveau.du, Some(c.d(40)));
    assert_eq!(r.devoirs_a_rattacher, 0);
    let ancien = c
        .ent
        .service_for_actor(c.etab, c.dir, "direction", s.id)
        .await
        .unwrap();
    assert_eq!(ancien.etat, "clos");
    assert_eq!(ancien.au, Some(c.d(39)));
    assert_eq!(c.en_saisie(s.id, c.d(39)).await, vec![c.prof]);
    assert!(
        c.en_saisie(s.id, c.d(40)).await.is_empty(),
        "l'ancien service ne saisit plus"
    );
    assert_eq!(c.en_saisie(r.nouveau.id, c.d(40)).await, vec![c.prof]);
    // Un service clos ne se modifie plus ; rien ne change : refusé ; même cible : refusé.
    let e = c
        .ent
        .modifier_service_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &ModifierServiceBody {
                heures_hebdo: Some(2.0),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
    let e = c
        .ent
        .remplacer_service_for_actor(
            c.etab,
            c.dir,
            "direction",
            r.nouveau.id,
            &RemplacerServiceBody {
                a_partir_du: c.d(80),
                classe_id: Some(classe2),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(est_invalide(&e), "{e:?}");
    assert_eq!(c.evenements("service.remplace").await, 1);
}

#[tokio::test]
async fn annuler_un_service_seulement_sans_saisie_clore_sinon() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let vide = c.service(Some(c.prof)).await;
    let annule = c
        .ent
        .annuler_service_for_actor(c.etab, c.dir, "direction", vide.id)
        .await
        .unwrap();
    assert_eq!(annule.etat, "annule");
    assert!(
        annule.intervenants.iter().all(|i| i.annulee),
        "les interventions sont annulées, jamais effacées"
    );
    assert_eq!(
        c.prof_user_du_service(vide.id).await,
        None,
        "un service annulé ne donne plus de droit"
    );
    // Il se recrée (l'annulé ne bloque plus l'unicité).
    let c2 = c.classe2().await;
    let occupe = c.service_sur(c2, c.matiere, Some(c.prof)).await;
    sqlx::query(
        "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine,
                                      heure_debut, heure_fin, service_id)
         VALUES (gen_random_uuid(), $1, 'X', 'Maths', $2, 1, '08:00', '09:00', $3)",
    )
    .bind(c.etab)
    .bind(c.prof_user)
    .bind(occupe.id)
    .execute(c.ent.pool())
    .await
    .unwrap();
    let e = c
        .ent
        .annuler_service_for_actor(c.etab, c.dir, "direction", occupe.id)
        .await
        .unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
    let clos = c
        .ent
        .clore_service_for_actor(
            c.etab,
            c.dir,
            "direction",
            occupe.id,
            &CloreServiceBody { au: Some(c.d(90)) },
        )
        .await
        .unwrap();
    assert_eq!(clos.etat, "clos");
    assert_eq!(clos.intervenants[0].au, Some(c.d(90)));
    // Clos : plus d'intervenant à ajouter, plus de suppléant, plus de nouvelle clôture.
    let e = c
        .ent
        .ajouter_intervenant_for_actor(
            c.etab,
            c.dir,
            "direction",
            occupe.id,
            &AjouterIntervenantBody {
                agent_id: c.prof2,
                nature: "co_enseignant".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
    let e = c
        .ent
        .clore_service_for_actor(
            c.etab,
            c.dir,
            "direction",
            occupe.id,
            &CloreServiceBody::default(),
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&e));
}

#[tokio::test]
async fn modifier_heures_periode_et_code_journal_sans_donnee_personnelle() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let s = c.service(Some(c.prof)).await;
    let m = c
        .ent
        .modifier_service_for_actor(
            c.etab,
            c.sec,
            "administratif",
            s.id,
            &ModifierServiceBody {
                heures_hebdo: Some(3.5),
                code: Some("MATHS-4B".into()),
                du: Some(c.d(5)),
                au: Some(c.d(200)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(m.heures_hebdo, Some(3.5));
    assert_eq!(m.code.as_deref(), Some("MATHS-4B"));
    assert_eq!((m.du, m.au), (Some(c.d(5)), Some(c.d(200))));
    let m = c
        .ent
        .modifier_service_for_actor(
            c.etab,
            c.sec,
            "administratif",
            s.id,
            &ModifierServiceBody {
                effacer_code: true,
                effacer_au: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!((m.code, m.au), (None, None));
    let e = c
        .ent
        .modifier_service_for_actor(
            c.etab,
            c.sec,
            "administratif",
            s.id,
            &ModifierServiceBody {
                du: Some(c.d(500)),
                au: Some(c.d(400)),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(est_invalide(&e));
    // Journal et événements : identifiants, codes, dates ; jamais de nom ni de coordonnée.
    let (journal,): (String,) = sqlx::query_as(
        "SELECT coalesce(string_agg(action || payload::text, ' '), '') FROM ent_journal_modif
          WHERE etablissement_id = $1 AND domaine = 'personnels'",
    )
    .bind(c.etab)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    let (evenements,): (String,) = sqlx::query_as(
        "SELECT coalesce(string_agg(type || payload::text, ' '), '') FROM ent_metier_event WHERE etablissement_id = $1",
    )
    .bind(c.etab)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    for attendu in [
        "service_cree",
        "service_modifie",
        "service_intervenant_ajoute",
    ] {
        assert!(journal.contains(attendu), "journal : {attendu}");
    }
    for interdit in ["Nom prof", "@ent.local"] {
        assert!(
            !journal.contains(interdit) && !evenements.contains(interdit),
            "« {interdit} » ne doit pas figurer"
        );
    }
}

// ─────────────────────────────── droits, cloisonnement, lecture ───────────────────────────────

#[tokio::test]
async fn matrice_de_droits_et_cloisonnement_par_etablissement() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let s = c.service(Some(c.prof)).await;
    let corps = CreerServiceBody {
        annee_id: Some(c.annee),
        classe_id: Some(c.classe),
        matiere_id: Some(c.matiere),
        ..Default::default()
    };
    // Le professeur et le parent ne gèrent pas ; le professeur lit la répartition, pas le parent.
    let e = c
        .ent
        .creer_service_for_actor(c.etab, c.prof_user, "prof", &corps)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden));
    let e = c
        .ent
        .ajouter_intervenant_for_actor(
            c.etab,
            c.prof_user,
            "prof",
            s.id,
            &AjouterIntervenantBody {
                agent_id: c.prof2,
                nature: "co_enseignant".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden));
    let parent = compte(&c.ent, c.etab, "parent").await;
    let e = c
        .ent
        .repartition_for_actor(c.etab, parent, "parent", &FiltreServices::default())
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden));
    let rep = c
        .ent
        .repartition_for_actor(c.etab, c.prof_user, "prof", &FiltreServices::default())
        .await
        .unwrap();
    assert!(rep.services.iter().any(|x| x.id == s.id));
    // Un autre établissement ne voit ni ne touche rien : 404 uniforme sur une référence étrangère.
    let autre = Uuid::new_v4();
    let dir_b = compte(&c.ent, autre, "direction").await;
    let e = c
        .ent
        .service_for_actor(autre, dir_b, "direction", s.id)
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound)),
        "{e:?}"
    );
    let e = c
        .ent
        .ajouter_intervenant_for_actor(
            autre,
            dir_b,
            "direction",
            s.id,
            &AjouterIntervenantBody {
                agent_id: c.prof2,
                nature: "co_enseignant".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound)),
        "{e:?}"
    );
    let e = c
        .ent
        .terminer_intervention_for_actor(
            autre,
            dir_b,
            "direction",
            s.intervenants[0].id,
            &TerminerInterventionBody::default(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound)),
        "{e:?}"
    );
    // Un agent d'un autre établissement ne peut pas intervenir ici.
    let agent_b = {
        let u = compte(&c.ent, autre, "prof").await;
        agent_de(&c.ent, autre, u).await
    };
    let e = c
        .ent
        .ajouter_intervenant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &AjouterIntervenantBody {
                agent_id: agent_b,
                nature: "intervenant".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound)),
        "{e:?}"
    );
    // La répartition de B est vide de nos services.
    let rep_b = c
        .ent
        .repartition_for_actor(autre, dir_b, "direction", &FiltreServices::default())
        .await
        .unwrap();
    assert!(rep_b.services.iter().all(|x| x.id != s.id));
}

#[tokio::test]
async fn mes_services_heures_et_alertes_de_coherence() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    // Heures dues : 4 h par semaine (engagement). Deux services de 4 h ⇒ dépassement signalé.
    sqlx::query(
        "INSERT INTO ent_agent_engagement (id, etablissement_id, agent_id, du, nature, employeur_type, heures_dues_hebdo)
         VALUES (gen_random_uuid(), $1, $2, $3, 'contractuel', 'etat', 4)",
    )
    .bind(c.etab)
    .bind(c.prof)
    .bind(c.debut - Duration::days(30))
    .execute(c.ent.pool())
    .await
    .unwrap();
    let classe2 = c.classe2().await;
    let s1 = c.service(Some(c.prof)).await;
    let _s2 = c.service_sur(classe2, c.matiere, Some(c.prof)).await;
    let moi = c
        .ent
        .mes_services_for_actor(c.etab, c.prof_user, "prof")
        .await
        .unwrap();
    assert_eq!(moi.agent_id, Some(c.prof));
    assert_eq!(moi.services.len(), 2);
    assert_eq!(moi.heures_dues, Some(4.0));
    assert_eq!(moi.heures_attribuees, 8.0);
    assert!(moi.alertes.iter().any(|a| a.code == "heures_depassees"));
    // Un intervenant hors de toute période d'engagement (le sien est fini) : alerte, jamais blocage.
    sqlx::query("UPDATE ent_agent_engagement SET du = $3, au = $4 WHERE etablissement_id = $1 AND agent_id = $2")
        .bind(c.etab)
        .bind(c.prof3)
        .bind(c.debut - Duration::days(100))
        .bind(c.debut + Duration::days(1))
        .execute(c.ent.pool())
        .await
        .unwrap();
    let hors = c
        .ent
        .ajouter_intervenant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s1.id,
            &AjouterIntervenantBody {
                agent_id: c.prof3,
                nature: "intervenant".into(),
                du: Some(c.d(5)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(hors
        .alertes
        .iter()
        .any(|a| a.code == "hors_engagement" && a.agent_id == Some(c.prof3)));
    // Un compte sans agent (le parent) n'a pas de service et n'a pas le droit d'en consulter.
    let parent = compte(&c.ent, c.etab, "parent").await;
    let e = c
        .ent
        .mes_services_for_actor(c.etab, parent, "parent")
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden));
    // Un autre professeur ne voit pas les services du premier dans « mes services ».
    let autre = c
        .ent
        .mes_services_for_actor(c.etab, c.prof2_user, "prof")
        .await
        .unwrap();
    assert!(autre.services.is_empty());
}

#[tokio::test]
async fn port_de_l_eleve_donne_le_titulaire_ou_le_suppleant_du_jour() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let eleve = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, created_at, updated_at) VALUES ($1,$2,'Élève','P3','inscrit', now(), now())")
        .bind(eleve)
        .bind(c.etab)
        .execute(c.ent.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut) VALUES (gen_random_uuid(),$1,$2,$3,$4,$5)")
        .bind(c.etab)
        .bind(c.annee)
        .bind(eleve)
        .bind(c.classe)
        .bind(c.debut)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let s = c.service(Some(c.prof)).await;
    c.ent
        .nommer_suppleant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &NommerSuppleantBody {
                agent_id: c.prof2,
                du: c.d(20),
                au: Some(c.d(25)),
                origine: None,
                origine_ref: None,
            },
        )
        .await
        .unwrap();
    let saisissant = |v: Vec<ent_types::personnels_services::IntervenantPortDto>| {
        let mut ids: Vec<Uuid> = v
            .into_iter()
            .filter(|i| i.saisie_effective)
            .map(|i| i.agent_id)
            .collect();
        ids.sort();
        ids
    };
    let avant = c
        .ent
        .intervenants_de_l_eleve(c.etab, eleve, c.d(10))
        .await
        .unwrap();
    assert_eq!(saisissant(avant), vec![c.prof]);
    let pendant = c
        .ent
        .intervenants_de_l_eleve(c.etab, eleve, c.d(22))
        .await
        .unwrap();
    assert_eq!(saisissant(pendant), vec![c.prof2]);
    // Un élève d'une autre classe n'a aucun de ces intervenants.
    let hors = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, created_at, updated_at) VALUES ($1,$2,'Autre','P3','inscrit', now(), now())")
        .bind(hors)
        .bind(c.etab)
        .execute(c.ent.pool())
        .await
        .unwrap();
    assert!(c
        .ent
        .intervenants_de_l_eleve(c.etab, hors, c.d(22))
        .await
        .unwrap()
        .is_empty());
}

// ─────────────────────────────── 362 : l'agent comme sujet ───────────────────────────────

#[tokio::test]
async fn l_agent_suit_le_compte_dans_creneaux_seances_et_absences() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let creneau = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine, heure_debut, heure_fin)
         VALUES ($1,$2,'X','Maths',$3,2,'08:00','09:00')",
    )
    .bind(creneau)
    .bind(c.etab)
    .bind(c.prof_user)
    .execute(c.ent.pool())
    .await
    .unwrap();
    let agent: Option<Uuid> =
        sqlx::query_scalar("SELECT agent_id FROM ent_edt_creneau WHERE id = $1")
            .bind(creneau)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!(
        agent,
        Some(c.prof),
        "agent_id suit prof_user_id à l'insertion"
    );
    sqlx::query("UPDATE ent_edt_creneau SET prof_user_id = $2 WHERE id = $1")
        .bind(creneau)
        .bind(c.prof2_user)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let agent: Option<Uuid> =
        sqlx::query_scalar("SELECT agent_id FROM ent_edt_creneau WHERE id = $1")
            .bind(creneau)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!(agent, Some(c.prof2), "…et suit le changement de compte");
    let seance = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_seance (id, etablissement_id, annee_id, creneau_id, jour, classe, prof_user_id, matiere_libelle,
                                     heure_debut, heure_fin, remplacant_user_id)
         VALUES ($1,$2,$3,$4,$5,'X',$6,'Maths','08:00','09:00',$7)",
    )
    .bind(seance)
    .bind(c.etab)
    .bind(c.annee)
    .bind(creneau)
    .bind(c.d(3))
    .bind(c.prof_user)
    .bind(c.prof2_user)
    .execute(c.ent.pool())
    .await
    .unwrap();
    let (a, r): (Option<Uuid>, Option<Uuid>) =
        sqlx::query_as("SELECT agent_id, remplacant_agent_id FROM ent_edt_seance WHERE id = $1")
            .bind(seance)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!((a, r), (Some(c.prof), Some(c.prof2)));
    // Absence du personnel : l'agent du compte ; un compte sans agent (parent) laisse NULL, sans refus.
    let parent = compte(&c.ent, c.etab, "parent").await;
    for (user, attendu) in [(c.prof_user, Some(c.prof)), (parent, None)] {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_absence_rh (id, etablissement_id, user_id, debut, fin, motif, created_by_user_id)
             VALUES ($1,$2,$3,$4,$4,'maladie',$5)",
        )
        .bind(id)
        .bind(c.etab)
        .bind(user)
        .bind(c.d(3))
        .bind(c.dir)
        .execute(c.ent.pool())
        .await
        .unwrap();
        let ag: Option<Uuid> =
            sqlx::query_scalar("SELECT agent_id FROM ent_absence_rh WHERE id = $1")
                .bind(id)
                .fetch_one(c.ent.pool())
                .await
                .unwrap();
        assert_eq!(ag, attendu);
    }
    // Indicateur : ce créneau n'a pas de service.
    let sans = c
        .ent
        .creneaux_sans_service_for_actor(c.etab, c.sec, "administratif")
        .await
        .unwrap();
    assert!(sans
        .iter()
        .any(|x| x.creneau_id == creneau && x.agent_id == Some(c.prof2)));
}

#[tokio::test]
async fn la_reprise_est_rejouable_et_un_service_sans_agent_est_tolere() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let s = c.service(Some(c.prof)).await;
    let avant: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_service_intervenant WHERE etablissement_id = $1",
    )
    .bind(c.etab)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    crate::migrate::rejouer_tout(c.ent.pool_verif()).await.unwrap(); // rejeu des fichiers (reprise), hors registre
    crate::migrate::rejouer_tout(c.ent.pool_verif()).await.unwrap(); // rejeu des fichiers (reprise), hors registre
    let apres: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_service_intervenant WHERE etablissement_id = $1",
    )
    .bind(c.etab)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(avant, apres);
    assert_eq!(c.prof_user_du_service(s.id).await, Some(c.prof_user));
    // Rafraîchissement de la projection : idempotent.
    let n: i32 = sqlx::query_scalar("SELECT ent_service_projection_rafraichir($1)")
        .bind(c.etab)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 0);
    // Un titulaire qui prendra ses fonctions plus tard n'est pas encore le professeur historique.
    let classe2 = c.classe2().await;
    let futur = c
        .ent
        .creer_service_for_actor(
            c.etab,
            c.dir,
            "direction",
            &CreerServiceBody {
                annee_id: Some(c.annee),
                classe_id: Some(classe2),
                matiere_id: Some(c.matiere),
                titulaire: Some(TitulaireInitialBody {
                    agent_id: c.prof2,
                    du: Some(c.debut + Duration::days(3000)),
                    heures_hebdo: None,
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(c.prof_user_du_service(futur.id).await, None);
    assert!(futur.vacant, "personne ne saisit encore");
}

#[tokio::test]
async fn la_fusion_de_deux_agents_deplace_interventions_et_references() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    // Un doublon de saisie (agent sans compte) intervient sur un service et figure sur un créneau.
    let doublon = match c
        .ent
        .creer_agent_for_actor(
            c.etab,
            c.dir,
            "direction",
            &ent_types::personnels::CreerAgentBody {
                personne: Some(ent_types::personnels::PersonneAgentBody {
                    nom: "Doublon".into(),
                    prenom: Some("Dora".into()),
                    email: Some(format!("p3-{}@ent.local", Uuid::new_v4().simple())),
                    ..Default::default()
                }),
                confirmer_nouvelle: true,
                ..Default::default()
            },
        )
        .await
        .expect("agent")
    {
        crate::personnels::ResultatCreationAgent::Cree(r) => r.agent_id,
        _ => panic!("création attendue"),
    };
    let s = c.service(Some(c.prof)).await;
    let ajoute = c
        .ent
        .ajouter_intervenant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &AjouterIntervenantBody {
                agent_id: doublon,
                nature: "assistant".into(),
                du: Some(c.d(5)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let intervention = ajoute
        .intervenants
        .iter()
        .find(|i| i.agent_id == doublon)
        .unwrap()
        .id;
    sqlx::query(
        "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine, heure_debut, heure_fin, agent_id)
         VALUES (gen_random_uuid(), $1, 'X', 'Maths', $2, 3, '10:00', '11:00', $3)",
    )
    .bind(c.etab)
    .bind(c.prof_user)
    .bind(doublon)
    .execute(c.ent.pool())
    .await
    .unwrap();
    c.ent
        .fusionner_agents_for_actor(
            c.etab,
            c.dir,
            "direction",
            &ent_types::personnels::FusionnerAgentsBody {
                source_agent_id: doublon,
                cible_agent_id: c.prof2,
                motif: "doublon de saisie".into(),
            },
        )
        .await
        .unwrap();
    let agent: Uuid =
        sqlx::query_scalar("SELECT agent_id FROM ent_service_intervenant WHERE id = $1")
            .bind(intervention)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!(agent, c.prof2, "l'intervention suit l'agent conservé");
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_creneau WHERE etablissement_id = $1 AND agent_id = $2",
    )
    .bind(c.etab)
    .bind(doublon)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 0, "plus aucune référence à l'agent fusionné");
    // Hors fusion, `agent_id` d'une intervention reste immuable.
    assert!(
        sqlx::query("UPDATE ent_service_intervenant SET agent_id = $2 WHERE id = $1")
            .bind(intervention)
            .bind(c.prof3)
            .execute(c.ent.pool())
            .await
            .is_err()
    );
    // Et l'agent fusionné ne peut plus recevoir d'intervention.
    let e = c
        .ent
        .ajouter_intervenant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &AjouterIntervenantBody {
                agent_id: doublon,
                nature: "intervenant".into(),
                du: Some(c.d(9)),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
}
