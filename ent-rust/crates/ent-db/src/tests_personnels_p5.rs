//! Socle 6, lot P5 : responsables de groupes datés (PRS-66), équipes pédagogiques calculées (PRS-70),
//! équipes constituées (PRS-71 à 75). Migrations 356 et 358. Sans `ENT_DATABASE_URL` : ignorés en local et
//! **en échec en CI**.
//!
//! * l'équipe pédagogique calculée est JUSTE sur un jeu de labo (groupe mixte, suppléance, assistant, service
//!   prévu ou clos, classe voisine) et à une date passée ;
//! * **aucune équipe n'ouvre de droit** (test exigé) : ni membre ni coordinateur ne gagne un droit sur les élèves ;
//! * responsabilités de groupe datées : le droit de `mes_groupes` s'éteint le soir de la fin, début futur sans droit ;
//! * confidentialité (équipe éducative autour d'un élève), cloisonnement par établissement, refus hors périmètre ;
//! * migrations 356 et 358 rejouables.

use super::*;
use chrono::{Duration, NaiveDate};
use domain_eleve::DomainError;
use ent_auth::{Action, Portee};
use ent_types::personnels_equipes::*;
use ent_types::personnels_missions::*;
use ent_types::personnels_services::*;
use uuid::Uuid;

const MDP: &str = "MotDePasse-P5-Aa9!";

async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let email = format!("p5-{role}-{}@ent.local", Uuid::new_v4().simple());
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

async fn eleve(ent: &PgEnt, etab: Uuid, nom: &str) -> Uuid {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: "P5".into(),
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
    .expect("élève")
    .id
}

fn est_conflit(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Refus(RefusMetier::Conflit, _))
}
fn est_invalide(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Refus(RefusMetier::Invalide, _))
}
fn est_introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(DomainError::NotFound))
}
fn est_interdit(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Forbidden)
}

struct Cadre {
    ent: PgEnt,
    etab: Uuid,
    dir: Uuid,
    jour: NaiveDate,
    debut: NaiveDate,
    annee: Uuid,
    classe: Uuid,
    voisine: Uuid,
    /// e1, e2 : classe ; e3 : classe (hors groupe) ; autre : classe voisine.
    e1: Uuid,
    e3: Uuid,
    autre: Uuid,
}

impl Cadre {
    async fn nouveau() -> Option<Cadre> {
        let ent = connecter().await?;
        let etab = Uuid::new_v4();
        let dir = compte(&ent, etab, "direction").await;
        let annee = ent.referentiel_annee_courante_assuree(etab).await.unwrap();
        let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
        let sfx = Uuid::new_v4().simple().to_string()[..6].to_string();
        let mut cl = Vec::new();
        for p in ["A", "B"] {
            cl.push(
                ent.referentiel_creer_classe(
                    "administratif",
                    etab,
                    Some(annee.id),
                    &format!("{p}{sfx}"),
                    Some("4e"),
                    Some("4EME"),
                )
                .await
                .unwrap()
                .id,
            );
        }
        let mut els = Vec::new();
        for (i, c) in [cl[0], cl[0], cl[1]].into_iter().enumerate() {
            let e = eleve(&ent, etab, &format!("Eleve{i}{sfx}")).await;
            ent.referentiel_affecter_eleve_classe(
                "administratif",
                etab,
                c,
                e,
                Some(annee.date_debut),
            )
            .await
            .unwrap();
            els.push(e);
        }
        Some(Cadre {
            ent,
            etab,
            dir,
            jour,
            debut: annee.date_debut,
            annee: annee.id,
            classe: cl[0],
            voisine: cl[1],
            e1: els[0],
            e3: els[1],
            autre: els[2],
        })
    }

    fn j(&self, n: i64) -> NaiveDate {
        self.jour + Duration::days(n)
    }

    async fn prof(&self) -> (Uuid, Uuid) {
        let u = compte(&self.ent, self.etab, "prof").await;
        (u, agent_de(&self.ent, self.etab, u).await)
    }

    async fn matiere(&self) -> Uuid {
        let sfx = Uuid::new_v4().simple().to_string()[..5].to_string();
        self.ent
            .create_matiere(
                CreateMatiere {
                    etablissement_id: self.etab,
                    code: format!("X{sfx}"),
                    libelle: format!("Matière {sfx}"),
                    source_nomenclature: None,
                },
                None,
            )
            .await
            .unwrap()
            .id
    }

    async fn groupe(&self, membres: &[Uuid]) -> Uuid {
        let g = self
            .ent
            .referentiel_creer_groupe(
                "administratif",
                self.etab,
                Some(self.annee),
                &format!("G{}", &Uuid::new_v4().simple().to_string()[..6]),
                Some("Groupe"),
                None,
            )
            .await
            .unwrap()
            .id;
        for e in membres {
            self.ent
                .referentiel_affecter_eleve_groupe(
                    "administratif",
                    self.etab,
                    g,
                    *e,
                    Some(self.debut),
                )
                .await
                .unwrap();
        }
        g
    }

    async fn service(
        &self,
        classe: Option<Uuid>,
        groupe: Option<Uuid>,
        titulaire: Uuid,
        etat: Option<&str>,
    ) -> ServiceDto {
        let m = self.matiere().await;
        self.ent
            .creer_service_for_actor(
                self.etab,
                self.dir,
                "direction",
                &CreerServiceBody {
                    annee_id: Some(self.annee),
                    classe_id: classe,
                    groupe_id: groupe,
                    matiere_id: Some(m),
                    heures_hebdo: Some(2.0),
                    etat: etat.map(str::to_string),
                    titulaire: Some(TitulaireInitialBody {
                        agent_id: titulaire,
                        du: Some(self.debut),
                        heures_hebdo: None,
                    }),
                    ..Default::default()
                },
            )
            .await
            .expect("service")
    }

    async fn equipe(
        &self,
        user: Uuid,
        role: &str,
        nature: &str,
        cible: Uuid,
        date: Option<NaiveDate>,
    ) -> Result<EquipeCalculeeDto, EntDbError> {
        self.ent
            .equipe_calculee_for_actor(
                self.etab,
                user,
                role,
                &FiltreEquipeCalculee {
                    nature: nature.into(),
                    cible_id: Some(cible),
                    discipline_code: None,
                    date,
                },
            )
            .await
    }

    async fn agents_de(&self, nature: &str, cible: Uuid, date: Option<NaiveDate>) -> Vec<Uuid> {
        let mut v: Vec<Uuid> = self
            .equipe(self.dir, "direction", nature, cible, date)
            .await
            .unwrap()
            .membres
            .into_iter()
            .map(|m| m.agent_id)
            .collect();
        v.sort();
        v
    }
}

fn tries(mut v: Vec<Uuid>) -> Vec<Uuid> {
    v.sort();
    v
}

// ─────────────────────────────── équipe pédagogique calculée ───────────────────────────────

#[tokio::test]
async fn equipe_pedagogique_de_la_classe_calculee_juste_sur_le_jeu_de_labo() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let (_, titulaire) = c.prof().await; // service A (classe)
    let (_, tit_b) = c.prof().await; // service B, suppléé
    let (_, suppl) = c.prof().await; // suppléant de B
    let (_, prof_groupe) = c.prof().await; // groupe qui contient e1 (élève de la classe)
    let (_, prof_groupe_autre) = c.prof().await; // groupe qui ne contient QUE l'élève de la voisine
    let (_, assistant) = c.prof().await;
    let (_, prof_voisine) = c.prof().await;
    let (_, prof_prevu) = c.prof().await;
    let (_, prof_clos) = c.prof().await;
    let (_, pp) = c.prof().await;

    let sa = c.service(Some(c.classe), None, titulaire, None).await;
    let sb = c.service(Some(c.classe), None, tit_b, None).await;
    c.ent
        .nommer_suppleant_for_actor(
            c.etab,
            c.dir,
            "direction",
            sb.id,
            &NommerSuppleantBody {
                agent_id: suppl,
                du: c.jour,
                au: None,
                origine: Some("absence".into()),
                origine_ref: None,
            },
        )
        .await
        .unwrap();
    c.ent
        .ajouter_intervenant_for_actor(
            c.etab,
            c.dir,
            "direction",
            sa.id,
            &AjouterIntervenantBody {
                agent_id: assistant,
                nature: "assistant".into(),
                du: Some(c.debut),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let g_mixte = c.groupe(&[c.e1, c.autre]).await;
    let g_voisin = c.groupe(&[c.autre]).await;
    c.service(None, Some(g_mixte), prof_groupe, None).await;
    c.service(None, Some(g_voisin), prof_groupe_autre, None)
        .await;
    c.service(Some(c.voisine), None, prof_voisine, None).await;
    c.service(Some(c.classe), None, prof_prevu, Some("prevu"))
        .await;
    let s_clos = c.service(Some(c.classe), None, prof_clos, None).await;
    c.ent
        .clore_service_for_actor(
            c.etab,
            c.dir,
            "direction",
            s_clos.id,
            &CloreServiceBody { au: Some(c.j(-3)) },
        )
        .await
        .expect("clore");
    c.ent
        .nommer_mission_for_actor(
            c.etab,
            c.dir,
            "direction",
            &NommerMissionBody {
                type_code: Some("professeur_principal".into()),
                agent_id: pp,
                perimetres: vec![PerimetreMissionDto {
                    nature: "classe".into(),
                    cible_id: Some(c.classe),
                    discipline_code: None,
                }],
                annee_id: Some(c.annee),
                du: Some(c.j(-2)),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Aujourd'hui : titulaires (dont celui de B, qui reste titulaire pendant la suppléance), suppléant, groupe mixte,
    // PP. Ni l'assistant, ni le service prévu, ni le service clos, ni la classe voisine, ni le groupe voisin.
    let attendu = tries(vec![titulaire, tit_b, suppl, prof_groupe, pp]);
    assert_eq!(c.agents_de("classe", c.classe, None).await, attendu);

    // Raisons : le suppléant est « suppleant », le PP « mission », le professeur du groupe « titulaire ».
    let e = c
        .equipe(c.dir, "direction", "classe", c.classe, None)
        .await
        .unwrap();
    let raison = |agent: Uuid| -> Vec<String> {
        e.membres
            .iter()
            .find(|m| m.agent_id == agent)
            .unwrap()
            .raisons
            .iter()
            .map(|r| r.code.clone())
            .collect()
    };
    assert_eq!(raison(suppl), vec!["suppleant"]);
    assert_eq!(raison(pp), vec!["mission"]);
    assert_eq!(raison(prof_groupe), vec!["titulaire"]);

    // Hier : pas de suppléant ; le PP (mission depuis -2) est là ; le service clos (jusqu'à -3) l'était il y a 5 jours.
    assert_eq!(
        c.agents_de("classe", c.classe, Some(c.j(-1))).await,
        tries(vec![titulaire, tit_b, prof_groupe, pp])
    );
    assert_eq!(
        c.agents_de("classe", c.classe, Some(c.j(-5))).await,
        tries(vec![titulaire, tit_b, prof_groupe, prof_clos])
    );

    // Équipe d'un ÉLÈVE : e1 a sa classe et le groupe mixte ; e3 (classe, sans groupe) n'a pas le professeur du groupe.
    assert_eq!(c.agents_de("eleve", c.e1, None).await, attendu);
    assert_eq!(
        c.agents_de("eleve", c.e3, None).await,
        tries(vec![titulaire, tit_b, suppl, pp])
    );
    assert_eq!(
        c.agents_de("eleve", c.autre, None).await,
        tries(vec![prof_voisine, prof_groupe, prof_groupe_autre])
    );

    // Équipe d'un GROUPE : intervenants du groupe + responsables datés.
    let (_, resp) = c.prof().await;
    c.ent
        .ajouter_responsable_groupe_for_actor(
            c.etab,
            c.dir,
            "direction",
            g_mixte,
            &AjouterResponsableBody {
                agent_id: resp,
                nature: "animateur".into(),
                du: None,
                au: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        c.agents_de("groupe", g_mixte, None).await,
        tries(vec![prof_groupe, resp])
    );

    // Équipe d'un NIVEAU : union des classes du niveau (les deux classes sont en 4e).
    let niveau = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_niveau (id, etablissement_id, code, libelle) VALUES ($1, $2, '4E', 'Quatrième')")
        .bind(niveau).bind(c.etab).execute(c.ent.pool()).await.unwrap();
    sqlx::query("UPDATE ent_classe SET niveau_id = $2 WHERE id = ANY($1)")
        .bind(vec![c.classe, c.voisine])
        .bind(niveau)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let n = c.agents_de("niveau", niveau, None).await;
    for a in [
        titulaire,
        tit_b,
        suppl,
        prof_groupe,
        pp,
        prof_voisine,
        prof_groupe_autre,
    ] {
        assert!(n.contains(&a), "niveau : {a}");
    }
    assert!(!n.contains(&assistant) && !n.contains(&prof_prevu) && !n.contains(&prof_clos));
}

#[tokio::test]
async fn equipe_discipline_par_matiere_et_coordinateur() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let (_, p1) = c.prof().await;
    let (_, p2) = c.prof().await;
    let (_, coord) = c.prof().await;
    let m = c.matiere().await;
    let autre_m = c.matiere().await;
    c.ent
        .creer_service_for_actor(
            c.etab,
            c.dir,
            "direction",
            &CreerServiceBody {
                annee_id: Some(c.annee),
                classe_id: Some(c.classe),
                matiere_id: Some(m),
                heures_hebdo: Some(2.0),
                titulaire: Some(TitulaireInitialBody {
                    agent_id: p1,
                    du: Some(c.debut),
                    heures_hebdo: None,
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_agent_discipline (id, etablissement_id, agent_id, matiere_id, principale, du)
                 VALUES ($1, $2, $3, $4, true, $5)")
        .bind(Uuid::new_v4()).bind(c.etab).bind(p2).bind(m).bind(c.debut).execute(c.ent.pool()).await.unwrap();
    c.ent
        .nommer_mission_for_actor(
            c.etab,
            c.dir,
            "direction",
            &NommerMissionBody {
                type_code: Some("coordinateur_discipline".into()),
                agent_id: coord,
                perimetres: vec![PerimetreMissionDto {
                    nature: "discipline".into(),
                    cible_id: None,
                    discipline_code: Some("maths".into()),
                }],
                annee_id: Some(c.annee),
                du: Some(c.jour),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    sqlx::query(
        "UPDATE ent_agent_discipline SET code_discipline_poste = 'maths' WHERE agent_id = $1",
    )
    .bind(p2)
    .execute(c.ent.pool())
    .await
    .unwrap();
    let e = c
        .ent
        .equipe_calculee_for_actor(
            c.etab,
            c.dir,
            "direction",
            &FiltreEquipeCalculee {
                nature: "discipline".into(),
                cible_id: Some(m),
                discipline_code: Some("maths".into()),
                date: None,
            },
        )
        .await
        .unwrap();
    let ids = tries(e.membres.iter().map(|x| x.agent_id).collect());
    assert_eq!(ids, tries(vec![p1, p2, coord]));
    // Une autre matière : personne.
    let e2 = c
        .ent
        .equipe_calculee_for_actor(
            c.etab,
            c.dir,
            "direction",
            &FiltreEquipeCalculee {
                nature: "discipline".into(),
                cible_id: Some(autre_m),
                discipline_code: None,
                date: None,
            },
        )
        .await
        .unwrap();
    assert!(e2.membres.is_empty());
}

#[tokio::test]
async fn equipe_cloisonnee_par_etablissement_et_refus_hors_perimetre() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let (prof_user, titulaire) = c.prof().await;
    c.service(Some(c.classe), None, titulaire, None).await;
    // Un autre établissement ne voit pas notre classe (404) ni nos élèves.
    let autre_etab = Uuid::new_v4();
    let dir2 = compte(&c.ent, autre_etab, "direction").await;
    let r = c
        .ent
        .equipe_calculee_for_actor(
            autre_etab,
            dir2,
            "direction",
            &FiltreEquipeCalculee {
                nature: "classe".into(),
                cible_id: Some(c.classe),
                ..Default::default()
            },
        )
        .await;
    assert!(est_introuvable(&r.unwrap_err()));
    let r = c
        .ent
        .equipe_calculee_for_actor(
            autre_etab,
            dir2,
            "direction",
            &FiltreEquipeCalculee {
                nature: "eleve".into(),
                cible_id: Some(c.e1),
                ..Default::default()
            },
        )
        .await;
    assert!(est_introuvable(&r.unwrap_err()));
    // Le professeur sans lien avec l'élève ne lit pas son équipe (404 : pas de fuite d'existence) ; le titulaire de
    // sa classe la lit.
    let (inconnu_user, _) = c.prof().await;
    let r = c.equipe(inconnu_user, "prof", "eleve", c.e1, None).await;
    assert!(est_introuvable(&r.unwrap_err()), "prof hors périmètre");
    assert!(c
        .equipe(prof_user, "prof", "eleve", c.e1, None)
        .await
        .is_ok());
    // Un parent ne lit ni une classe ni l'élève d'autrui.
    let parent = compte(&c.ent, c.etab, "parent").await;
    assert!(est_interdit(
        &c.equipe(parent, "parent", "classe", c.classe, None)
            .await
            .unwrap_err()
    ));
    assert!(est_introuvable(
        &c.equipe(parent, "parent", "eleve", c.e1, None)
            .await
            .unwrap_err()
    ));
    // Nature inconnue, cible manquante : invalide.
    let r = c
        .equipe(c.dir, "direction", "planete", c.classe, None)
        .await;
    assert!(est_invalide(&r.unwrap_err()));
    let r = c
        .ent
        .equipe_calculee_for_actor(
            c.etab,
            c.dir,
            "direction",
            &FiltreEquipeCalculee {
                nature: "classe".into(),
                ..Default::default()
            },
        )
        .await;
    assert!(est_invalide(&r.unwrap_err()));
}

// ─────────────────────────────── responsables de groupes datés ───────────────────────────────

#[tokio::test]
async fn responsable_de_groupe_date_le_droit_mes_groupes_s_eteint_le_soir_de_la_fin() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let (user, agent) = c.prof().await;
    let g = c.groupe(&[c.e1]).await;
    let role = c
        .ent
        .creer_role_for_actor(
            c.etab,
            c.dir,
            "direction",
            RoleSaisie {
                nom: format!("Atelier {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::AbsencesLire,
                    portee: Portee::MesGroupes,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    c.ent
        .attribuer_role_for_actor(c.etab, c.dir, "direction", role.id, user, None)
        .await
        .unwrap();
    let voit = || async {
        c.ent
            .actor_can_read_absences(c.etab, user, "prof", c.e1)
            .await
            .unwrap()
    };
    assert!(!voit().await, "pas encore responsable");

    // Début futur : aucun droit aujourd'hui.
    let futur = c
        .ent
        .ajouter_responsable_groupe_for_actor(
            c.etab,
            c.dir,
            "direction",
            g,
            &AjouterResponsableBody {
                agent_id: agent,
                nature: "responsable".into(),
                du: Some(c.j(3)),
                au: None,
            },
        )
        .await
        .unwrap();
    assert!(!futur.active);
    assert!(!voit().await, "début futur : pas de droit");
    // Chevauchement refusé (409), début dans le passé refusé (422).
    let r = c
        .ent
        .ajouter_responsable_groupe_for_actor(
            c.etab,
            c.dir,
            "direction",
            g,
            &AjouterResponsableBody {
                agent_id: agent,
                nature: "animateur".into(),
                du: Some(c.j(5)),
                au: None,
            },
        )
        .await;
    assert!(est_conflit(&r.unwrap_err()));
    let r = c
        .ent
        .ajouter_responsable_groupe_for_actor(
            c.etab,
            c.dir,
            "direction",
            g,
            &AjouterResponsableBody {
                agent_id: agent,
                nature: "animateur".into(),
                du: Some(c.j(-1)),
                au: None,
            },
        )
        .await;
    assert!(est_invalide(&r.unwrap_err()));
    // Une responsabilité qui n'a pas commencé se termine en la retirant de la période : on la fait démarrer plus tôt
    // (reprise de données) pour tester la fin.
    sqlx::query("UPDATE ent_groupe_responsable SET du = $2 WHERE id = $1")
        .bind(futur.id)
        .bind(c.j(-10))
        .execute(c.ent.pool())
        .await
        .unwrap();
    assert!(voit().await, "responsable depuis 10 jours");
    // Fin AUJOURD'HUI : le droit tient jusqu'au soir (jour de l'établissement).
    let t = c
        .ent
        .terminer_responsable_groupe_for_actor(
            c.etab,
            c.dir,
            "direction",
            futur.id,
            &TerminerResponsableBody { au: Some(c.jour) },
        )
        .await
        .unwrap();
    assert!(t.active && t.au == Some(c.jour));
    assert!(voit().await, "le dernier jour, le droit tient encore");
    // Terminer deux fois : 409.
    sqlx::query("UPDATE ent_groupe_responsable SET au = $2 WHERE id = $1")
        .bind(futur.id)
        .bind(c.j(-1))
        .execute(c.ent.pool())
        .await
        .unwrap();
    assert!(!voit().await, "le lendemain de la fin : plus de droit");
    let r = c
        .ent
        .terminer_responsable_groupe_for_actor(
            c.etab,
            c.dir,
            "direction",
            futur.id,
            &TerminerResponsableBody::default(),
        )
        .await;
    assert!(est_conflit(&r.unwrap_err()));
    // L'historique reste lisible.
    let l = c
        .ent
        .responsables_groupe_for_actor(c.etab, c.dir, "direction", g)
        .await
        .unwrap();
    assert_eq!(l.len(), 1);
    assert!(!l[0].active);
}

#[tokio::test]
async fn responsables_de_groupe_gardes_de_droit_et_d_etablissement() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let (user, agent) = c.prof().await;
    let g = c.groupe(&[c.e1]).await;
    // Le professeur ne désigne personne (403) ; il lit (tout le personnel lit la répartition).
    let r = c
        .ent
        .ajouter_responsable_groupe_for_actor(
            c.etab,
            user,
            "prof",
            g,
            &AjouterResponsableBody {
                agent_id: agent,
                nature: "responsable".into(),
                du: None,
                au: None,
            },
        )
        .await;
    assert!(est_interdit(&r.unwrap_err()));
    assert!(c
        .ent
        .responsables_groupe_for_actor(c.etab, user, "prof", g)
        .await
        .unwrap()
        .is_empty());
    // Nature inconnue : 422. Agent d'un autre établissement : 404. Groupe d'un autre établissement : 404.
    let r = c
        .ent
        .ajouter_responsable_groupe_for_actor(
            c.etab,
            c.dir,
            "direction",
            g,
            &AjouterResponsableBody {
                agent_id: agent,
                nature: "chef".into(),
                du: None,
                au: None,
            },
        )
        .await;
    assert!(est_invalide(&r.unwrap_err()));
    let autre_etab = Uuid::new_v4();
    let dir2 = compte(&c.ent, autre_etab, "direction").await;
    let u2 = compte(&c.ent, autre_etab, "prof").await;
    let a2 = agent_de(&c.ent, autre_etab, u2).await;
    let r = c
        .ent
        .ajouter_responsable_groupe_for_actor(
            c.etab,
            c.dir,
            "direction",
            g,
            &AjouterResponsableBody {
                agent_id: a2,
                nature: "responsable".into(),
                du: None,
                au: None,
            },
        )
        .await;
    assert!(
        est_introuvable(&r.unwrap_err()),
        "agent d'un autre établissement"
    );
    let r = c
        .ent
        .ajouter_responsable_groupe_for_actor(
            autre_etab,
            dir2,
            "direction",
            g,
            &AjouterResponsableBody {
                agent_id: a2,
                nature: "responsable".into(),
                du: None,
                au: None,
            },
        )
        .await;
    assert!(
        est_introuvable(&r.unwrap_err()),
        "groupe d'un autre établissement"
    );
    // La base refuse un responsable d'un autre établissement posé en SQL (garde de la migration 356).
    let r = sqlx::query(
        "INSERT INTO ent_groupe_responsable (id, groupe_id, etablissement_id, agent_id, du)
                         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(g)
    .bind(autre_etab)
    .bind(a2)
    .bind(c.jour)
    .execute(c.ent.pool())
    .await;
    assert!(
        r.is_err(),
        "groupe d'un autre établissement refusé par la base"
    );
}

#[tokio::test]
async fn remplacement_en_bloc_historique_conserve_et_projection_compte() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let (u1, a1) = c.prof().await;
    let (u2, a2) = c.prof().await;
    let g = c.groupe(&[c.e1]).await;
    c.ent
        .definir_responsables_groupe_for_actor(c.etab, c.dir, "direction", g, &[u1])
        .await
        .unwrap();
    sqlx::query("UPDATE ent_groupe_responsable SET du = $2 WHERE groupe_id = $1")
        .bind(g)
        .bind(c.j(-30))
        .execute(c.ent.pool())
        .await
        .unwrap();
    // Le compte et l'agent sont reliés par la base (projection) ; la nature par défaut est « responsable ».
    let (agent, nature): (Uuid, String) =
        sqlx::query_as("SELECT agent_id, nature FROM ent_groupe_responsable WHERE groupe_id = $1")
            .bind(g)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!(agent, a1);
    assert_eq!(nature, "responsable");
    // Remplacer u1 par u2 : u1 est TERMINÉ (hier), pas supprimé ; u2 commence aujourd'hui.
    c.ent
        .definir_responsables_groupe_for_actor(c.etab, c.dir, "direction", g, &[u2])
        .await
        .unwrap();
    let l = c
        .ent
        .responsables_groupe_for_actor(c.etab, c.dir, "direction", g)
        .await
        .unwrap();
    assert_eq!(l.len(), 2);
    let ancien = l.iter().find(|r| r.agent_id == a1).unwrap();
    assert!(!ancien.active && ancien.au == Some(c.j(-1)));
    assert!(l.iter().find(|r| r.agent_id == a2).unwrap().active);
}

// ─────────────────────────────── équipes constituées ───────────────────────────────

#[tokio::test]
async fn aucune_equipe_n_ouvre_de_droit_sur_les_eleves() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let (user, agent) = c.prof().await;
    let avant = c
        .ent
        .eleves_autorises(c.etab, user, "prof", Action::AbsencesLire)
        .await
        .unwrap();
    let avant_dossier = c
        .ent
        .autoriser_eleve(c.etab, user, "prof", Action::DossierLire, c.e1)
        .await
        .unwrap();
    // Équipe éducative autour de e1, confidentielle, avec le professeur COORDINATEUR ; cellule de veille aussi.
    let e = c
        .ent
        .creer_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            &CreerEquipeBody {
                type_code: "equipe_educative_eleve".into(),
                libelle: "Suivi e1".into(),
                annee_id: None,
                du: None,
                au: None,
                objet_eleve_id: Some(c.e1),
                liste_diffusion: true,
                visible_annuaire: true,
                confidentielle: false,
                membres: vec![NouveauMembreBody {
                    agent_id: Some(agent),
                    role_dans_equipe: Some("coordinateur".into()),
                    ..Default::default()
                }],
            },
        )
        .await
        .unwrap();
    assert!(e.confidentielle && !e.visible_annuaire && !e.ouvre_des_droits);
    c.ent
        .creer_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            &CreerEquipeBody {
                type_code: "cellule_veille".into(),
                libelle: "Veille".into(),
                confidentielle: true,
                membres: vec![NouveauMembreBody {
                    agent_id: Some(agent),
                    ..Default::default()
                }],
                annee_id: None,
                du: None,
                au: None,
                objet_eleve_id: None,
                liste_diffusion: false,
                visible_annuaire: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        c.ent
            .eleves_autorises(c.etab, user, "prof", Action::AbsencesLire)
            .await
            .unwrap(),
        avant
    );
    assert_eq!(
        c.ent
            .autoriser_eleve(c.etab, user, "prof", Action::DossierLire, c.e1)
            .await
            .unwrap(),
        avant_dossier
    );
    assert!(
        !avant_dossier,
        "le professeur n'avait aucun droit sur e1 et n'en gagne pas par l'équipe"
    );
    // Aucune table de droits ne lit les équipes : garde statique sur les sources.
    for f in ["droits_repo.rs", "perimetre.rs", "droits_ctx.rs"] {
        let t =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/").to_string() + f)
                .unwrap_or_default();
        assert!(
            !t.contains("ent_equipe"),
            "{f} lit les équipes : une équipe ne donne aucun droit"
        );
    }
    let m = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/355_missions.sql"
    ))
    .unwrap_or_default();
    assert!(
        !m.contains("ent_equipe_membre"),
        "la fonction D-022 ne lit pas les équipes"
    );
}

#[tokio::test]
async fn equipe_confidentielle_visible_des_seuls_membres_et_de_la_direction() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let (membre_user, membre) = c.prof().await;
    let (autre_user, _) = c.prof().await;
    let (coord_user, coord) = c.prof().await;
    let e = c
        .ent
        .creer_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            &CreerEquipeBody {
                type_code: "cellule_veille".into(),
                libelle: "Veille".into(),
                confidentielle: true,
                annee_id: None,
                du: None,
                au: None,
                objet_eleve_id: None,
                liste_diffusion: true,
                visible_annuaire: false,
                membres: vec![
                    NouveauMembreBody {
                        agent_id: Some(membre),
                        ..Default::default()
                    },
                    NouveauMembreBody {
                        agent_id: Some(coord),
                        role_dans_equipe: Some("coordinateur".into()),
                        ..Default::default()
                    },
                ],
            },
        )
        .await
        .unwrap();
    // Le membre la voit ; l'autre professeur non (404, aucune fuite d'existence) ; la liste ne la montre pas.
    assert_eq!(
        c.ent
            .equipe_for_actor(c.etab, membre_user, "prof", e.id)
            .await
            .unwrap()
            .membres
            .len(),
        2
    );
    assert!(est_introuvable(
        &c.ent
            .equipe_for_actor(c.etab, autre_user, "prof", e.id)
            .await
            .unwrap_err()
    ));
    assert!(c
        .ent
        .equipes_for_actor(c.etab, autre_user, "prof", &FiltreEquipes::default())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        c.ent
            .equipes_for_actor(c.etab, membre_user, "prof", &FiltreEquipes::default())
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        c.ent
            .equipes_for_actor(c.etab, c.dir, "direction", &FiltreEquipes::default())
            .await
            .unwrap()
            .len(),
        1
    );
    // Un autre établissement : 404 aussi, même pour sa direction.
    let autre_etab = Uuid::new_v4();
    let dir2 = compte(&c.ent, autre_etab, "direction").await;
    assert!(est_introuvable(
        &c.ent
            .equipe_for_actor(autre_etab, dir2, "direction", e.id)
            .await
            .unwrap_err()
    ));
    // Le membre simple ne modifie rien (403) ; le non-membre : 404 ; le coordinateur ajoute un collègue.
    let (_, nouveau) = c.prof().await;
    let ajout = NouveauMembreBody {
        agent_id: Some(nouveau),
        ..Default::default()
    };
    assert!(est_interdit(
        &c.ent
            .ajouter_membre_equipe_for_actor(c.etab, membre_user, "prof", e.id, &ajout)
            .await
            .unwrap_err()
    ));
    assert!(est_introuvable(
        &c.ent
            .ajouter_membre_equipe_for_actor(c.etab, autre_user, "prof", e.id, &ajout)
            .await
            .unwrap_err()
    ));
    let apres = c
        .ent
        .ajouter_membre_equipe_for_actor(c.etab, coord_user, "prof", e.id, &ajout)
        .await
        .unwrap();
    assert_eq!(apres.membres.len(), 3);
    // Doublon : 409. Agent d'un autre établissement : 404.
    assert!(est_conflit(
        &c.ent
            .ajouter_membre_equipe_for_actor(c.etab, coord_user, "prof", e.id, &ajout)
            .await
            .unwrap_err()
    ));
    let u2 = compte(&c.ent, autre_etab, "prof").await;
    let a2 = agent_de(&c.ent, autre_etab, u2).await;
    let etranger = NouveauMembreBody {
        agent_id: Some(a2),
        ..Default::default()
    };
    assert!(est_introuvable(
        &c.ent
            .ajouter_membre_equipe_for_actor(c.etab, c.dir, "direction", e.id, &etranger)
            .await
            .unwrap_err()
    ));
}

#[tokio::test]
async fn equipe_membres_dates_liste_de_diffusion_calculee_et_cloture() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let (_, a1) = c.prof().await;
    let (_, a2) = c.prof().await;
    let (_, a3) = c.prof().await;
    let e = c
        .ent
        .creer_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            &CreerEquipeBody {
                type_code: "equipe_projet".into(),
                libelle: "Voyage".into(),
                annee_id: Some(c.annee),
                du: None,
                au: Some(c.j(30)),
                objet_eleve_id: None,
                liste_diffusion: true,
                visible_annuaire: true,
                confidentielle: false,
                membres: vec![
                    NouveauMembreBody {
                        agent_id: Some(a1),
                        ..Default::default()
                    },
                    NouveauMembreBody {
                        agent_id: Some(a2),
                        du: Some(c.j(5)),
                        ..Default::default()
                    },
                ],
            },
        )
        .await
        .unwrap();
    assert!(e.visible_annuaire && e.etat == "active");
    // Liste de diffusion calculée AU MOMENT de l'envoi : a2 n'est pas encore membre, a3 le devient.
    let d = c
        .ent
        .equipe_destinataires(c.etab, e.id, c.jour, false)
        .await
        .unwrap();
    assert_eq!(d.agents, vec![a1]);
    assert_eq!(d.comptes.len(), 1);
    c.ent
        .ajouter_membre_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            e.id,
            &NouveauMembreBody {
                agent_id: Some(a3),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let d = c
        .ent
        .equipe_destinataires(c.etab, e.id, c.jour, false)
        .await
        .unwrap();
    assert_eq!(tries(d.agents), tries(vec![a1, a3]));
    let d5 = c
        .ent
        .equipe_destinataires(c.etab, e.id, c.j(5), false)
        .await
        .unwrap();
    assert_eq!(d5.agents.len(), 3);
    // Un membre ne déborde pas la période de l'équipe (422).
    let r = c
        .ent
        .ajouter_membre_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            e.id,
            &NouveauMembreBody {
                agent_id: Some(a3),
                du: Some(c.j(40)),
                ..Default::default()
            },
        )
        .await;
    assert!(r.is_err());
    // Terminer un membre : il part le jour dit ; un membre à venir est retiré.
    let eq = c
        .ent
        .equipe_for_actor(c.etab, c.dir, "direction", e.id)
        .await
        .unwrap();
    let m1 = eq
        .membres
        .iter()
        .find(|m| m.agent_id == Some(a1))
        .unwrap()
        .id;
    let m2 = eq
        .membres
        .iter()
        .find(|m| m.agent_id == Some(a2))
        .unwrap()
        .id;
    c.ent
        .terminer_membre_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            e.id,
            m1,
            &TerminerMembreBody { au: Some(c.jour) },
        )
        .await
        .unwrap();
    c.ent
        .terminer_membre_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            e.id,
            m2,
            &TerminerMembreBody { au: Some(c.jour) },
        )
        .await
        .unwrap();
    let eq = c
        .ent
        .equipe_for_actor(c.etab, c.dir, "direction", e.id)
        .await
        .unwrap();
    assert_eq!(
        eq.membres.len(),
        2,
        "le membre à venir est retiré, l'autre part aujourd'hui"
    );
    // Clôture : tous partent, l'équipe est close, on ne la rouvre pas, plus de membre.
    let close = c
        .ent
        .clore_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            e.id,
            &ClorePeriodeBody { au: Some(c.jour) },
        )
        .await
        .unwrap();
    assert_eq!(close.etat, "close");
    assert!(est_conflit(
        &c.ent
            .clore_equipe_for_actor(
                c.etab,
                c.dir,
                "direction",
                e.id,
                &ClorePeriodeBody::default()
            )
            .await
            .unwrap_err()
    ));
    assert!(est_conflit(
        &c.ent
            .ajouter_membre_equipe_for_actor(
                c.etab,
                c.dir,
                "direction",
                e.id,
                &NouveauMembreBody {
                    agent_id: Some(a1),
                    ..Default::default()
                }
            )
            .await
            .unwrap_err()
    ));
    let d = c
        .ent
        .equipe_destinataires(c.etab, e.id, c.j(1), false)
        .await
        .unwrap();
    assert!(
        d.agents.is_empty(),
        "après la clôture, plus de destinataire"
    );
    // Une équipe qui n'est pas une liste de diffusion refuse le port.
    let sans = c
        .ent
        .creer_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            &CreerEquipeBody {
                type_code: "autre".into(),
                libelle: "Autre".into(),
                annee_id: None,
                du: None,
                au: None,
                objet_eleve_id: None,
                liste_diffusion: false,
                visible_annuaire: false,
                confidentielle: false,
                membres: vec![],
            },
        )
        .await
        .unwrap();
    assert!(est_invalide(
        &c.ent
            .equipe_destinataires(c.etab, sans.id, c.jour, false)
            .await
            .unwrap_err()
    ));
}

#[tokio::test]
async fn equipe_educative_pp_constitue_famille_invitee_et_gardes() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let (pp_user, pp) = c.prof().await;
    let (autre_user, _) = c.prof().await;
    c.ent
        .nommer_mission_for_actor(
            c.etab,
            c.dir,
            "direction",
            &NommerMissionBody {
                type_code: Some("professeur_principal".into()),
                agent_id: pp,
                perimetres: vec![PerimetreMissionDto {
                    nature: "classe".into(),
                    cible_id: Some(c.classe),
                    discipline_code: None,
                }],
                annee_id: Some(c.annee),
                du: Some(c.j(-2)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let corps = |eleve: Uuid| CreerEquipeBody {
        type_code: "equipe_educative_eleve".into(),
        libelle: "Suivi".into(),
        annee_id: None,
        du: None,
        au: None,
        objet_eleve_id: Some(eleve),
        liste_diffusion: true,
        visible_annuaire: false,
        confidentielle: true,
        membres: vec![NouveauMembreBody {
            agent_id: Some(pp),
            ..Default::default()
        }],
    };
    // Le PP constitue l'équipe d'un élève de SA classe ; pas celle d'un élève d'une autre classe (403) ; un autre
    // professeur ne le peut pas (403).
    let e = c
        .ent
        .creer_equipe_for_actor(c.etab, pp_user, "prof", &corps(c.e1))
        .await
        .unwrap();
    assert!(e.confidentielle);
    assert!(est_interdit(
        &c.ent
            .creer_equipe_for_actor(c.etab, pp_user, "prof", &corps(c.autre))
            .await
            .unwrap_err()
    ));
    assert!(est_interdit(
        &c.ent
            .creer_equipe_for_actor(c.etab, autre_user, "prof", &corps(c.e1))
            .await
            .unwrap_err()
    ));
    // Pas de type sans objet pour un type « élève », ni d'objet pour un autre type (422).
    let mut sans = corps(c.e1);
    sans.objet_eleve_id = None;
    assert!(est_invalide(
        &c.ent
            .creer_equipe_for_actor(c.etab, c.dir, "direction", &sans)
            .await
            .unwrap_err()
    ));
    let mut en_trop = corps(c.e1);
    en_trop.type_code = "equipe_projet".into();
    assert!(est_invalide(
        &c.ent
            .creer_equipe_for_actor(c.etab, c.dir, "direction", &en_trop)
            .await
            .unwrap_err()
    ));
    // Type inconnu : 404.
    let mut inconnu = corps(c.e1);
    inconnu.type_code = "inexistant".into();
    assert!(est_introuvable(
        &c.ent
            .creer_equipe_for_actor(c.etab, c.dir, "direction", &inconnu)
            .await
            .unwrap_err()
    ));
    // Élève d'un autre établissement : refusé par la base (404).
    let autre_etab = Uuid::new_v4();
    let dir2 = compte(&c.ent, autre_etab, "direction").await;
    let r = c
        .ent
        .creer_equipe_for_actor(autre_etab, dir2, "direction", &corps(c.e1))
        .await;
    assert!(r.is_err(), "élève d'un autre établissement");
    // Famille invitée : une personne qui n'est pas responsable de l'élève n'est pas invitable (404).
    let inconnue: Uuid = sqlx::query_scalar(
        "INSERT INTO ent_personne (id, etablissement_id, nom, source) VALUES ($1, $2, 'Tiers', 'saisie') RETURNING id")
        .bind(Uuid::new_v4()).bind(c.etab).fetch_one(c.ent.pool()).await.unwrap();
    let r = c
        .ent
        .ajouter_membre_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            e.id,
            &NouveauMembreBody {
                personne_id: Some(inconnue),
                ..Default::default()
            },
        )
        .await;
    assert!(est_introuvable(&r.unwrap_err()));
    // Un membre est un agent OU une personne.
    let r = c
        .ent
        .ajouter_membre_equipe_for_actor(
            c.etab,
            c.dir,
            "direction",
            e.id,
            &NouveauMembreBody::default(),
        )
        .await;
    assert!(est_invalide(&r.unwrap_err()));
}

#[tokio::test]
async fn familles_et_eleves_ne_lisent_que_l_equipe_de_leur_enfant_avec_identite_protegee() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let (_, titulaire) = c.prof().await;
    let (_, menacee) = c.prof().await;
    c.service(Some(c.classe), None, titulaire, None).await;
    c.service(Some(c.classe), None, menacee, None).await;
    sqlx::query("UPDATE ent_agent SET identite_restreinte = true WHERE id = $1")
        .bind(menacee)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let parent = compte(&c.ent, c.etab, "parent").await;
    sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, lien) VALUES ($1,$2,$3,$4,'pere')")
        .bind(Uuid::new_v4()).bind(c.etab).bind(c.e1).bind(parent).execute(c.ent.pool()).await.expect("lien");
    let e = c
        .equipe(parent, "parent", "eleve", c.e1, None)
        .await
        .unwrap();
    assert_eq!(e.membres.len(), 2);
    let nom_menace = e
        .membres
        .iter()
        .find(|m| m.agent_id == menacee)
        .unwrap()
        .nom
        .clone()
        .unwrap();
    assert!(
        nom_menace.ends_with('.') && nom_menace.chars().count() <= 6,
        "identité restreinte : initiale seule, {nom_menace}"
    );
    // L'autre élève : 404 pour ce parent.
    assert!(est_introuvable(
        &c.equipe(parent, "parent", "eleve", c.e3, None)
            .await
            .unwrap_err()
    ));
}

#[tokio::test]
async fn migrations_356_et_358_rejouables() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    for f in ["356_groupes_responsables_dates", "358_equipes"] {
        let sql = std::fs::read_to_string(format!(
            "{}/../../migrations/{f}.sql",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        for _ in 0..2 {
            rejouer(&c.ent, &sql, f).await;
        }
    }
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_equipe_type WHERE etablissement_id IS NULL")
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!(n, 8, "huit types fournis, sans doublon");
}

/// Rejoue un fichier de migration ; le DDL concurrent d'un autre test peut provoquer un interblocage, on réessaie.
async fn rejouer(ent: &PgEnt, sql: &str, nom: &str) {
    for essai in 0..8 {
        match sqlx::raw_sql(sql).execute(ent.pool()).await {
            Ok(_) => return,
            Err(e) if e.to_string().contains("deadlock") && essai < 7 => {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            }
            Err(e) => panic!("{nom} rejouée : {e}"),
        }
    }
}
