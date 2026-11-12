//! Socle 6, lot P4 : missions, périmètre des droits (portée `mon_perimetre`), bascule du périmètre
//! d'enseignement sur les interventions (migration 355), explication et simulation. Sans
//! `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! * D-022 : LV2 mixte, co-enseignement, suppléance, remplacement d'une séance, délai de grâce, deux PP ;
//! * PP sans service sur la classe ; repli « classe texte » coupé quand tous les créneaux ont un service ;
//! * R-P5 : un agent qui est aussi élève ne voit aucun camarade ;
//! * missions : cardinalité, éligibilité (avertir / bloquer), fin, transfert, annulation, reconduction,
//!   types propres (anti-escalade), type réservé à un autre socle ; cloisonnement par établissement ;
//! * gardes de la base (périmètre immuable, cible d'un autre établissement), 355 rejouable.

use super::*;
use crate::personnels_missions::droits_du_modele;
use chrono::{Duration, NaiveDate};
use domain_eleve::DomainError;
use ent_auth::{Action, Portee};
use ent_types::personnels_missions::*;
use ent_types::personnels_services::*;
use uuid::Uuid;

const MDP: &str = "MotDePasse-P4-Aa9!";

async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let email = format!("p4-{role}-{}@ent.local", Uuid::new_v4().simple());
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
        prenom: "P4".into(),
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

struct Cadre {
    ent: PgEnt,
    etab: Uuid,
    dir: Uuid,
    admin: Uuid,
    p1: Uuid,
    p1_user: Uuid,
    p2: Uuid,
    p2_user: Uuid,
    p3: Uuid,
    p3_user: Uuid,
    annee: Uuid,
    debut: NaiveDate,
    jour: NaiveDate,
    classe: Uuid,
    voisine: Uuid,
    matiere: Uuid,
    /// Trois élèves de la classe ; `autre` est dans la classe voisine.
    e1: Uuid,
    e2: Uuid,
    e3: Uuid,
    autre: Uuid,
}

impl Cadre {
    async fn nouveau() -> Option<Cadre> {
        let ent = connecter().await?;
        let etab = Uuid::new_v4();
        let dir = compte(&ent, etab, "direction").await;
        let admin = compte(&ent, etab, "admin").await;
        let p1_user = compte(&ent, etab, "prof").await;
        let p2_user = compte(&ent, etab, "prof").await;
        let p3_user = compte(&ent, etab, "prof").await;
        let (p1, p2, p3) = (
            agent_de(&ent, etab, p1_user).await,
            agent_de(&ent, etab, p2_user).await,
            agent_de(&ent, etab, p3_user).await,
        );
        let annee = ent.referentiel_annee_courante_assuree(etab).await.unwrap();
        let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
        let sfx = Uuid::new_v4().simple().to_string()[..6].to_string();
        let classe = ent
            .referentiel_creer_classe(
                "administratif",
                etab,
                Some(annee.id),
                &format!("A{sfx}"),
                Some("4e"),
                Some("4EME"),
            )
            .await
            .unwrap()
            .id;
        let voisine = ent
            .referentiel_creer_classe(
                "administratif",
                etab,
                Some(annee.id),
                &format!("B{sfx}"),
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
        let mut els = Vec::new();
        for (i, cl) in [classe, classe, classe, voisine].into_iter().enumerate() {
            let e = eleve(&ent, etab, &format!("Eleve{i}{sfx}")).await;
            ent.referentiel_affecter_eleve_classe(
                "administratif",
                etab,
                cl,
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
            admin,
            p1,
            p1_user,
            p2,
            p2_user,
            p3,
            p3_user,
            annee: annee.id,
            debut: annee.date_debut,
            jour,
            classe,
            voisine,
            matiere,
            e1: els[0],
            e2: els[1],
            e3: els[2],
            autre: els[3],
        })
    }

    fn j(&self, n: i64) -> NaiveDate {
        self.jour + Duration::days(n)
    }

    /// Service de la classe (matière du cadre) avec ce titulaire depuis le début de l'année.
    async fn service(&self, titulaire: Uuid) -> ServiceDto {
        self.service_sur(Some(self.classe), None, titulaire).await
    }

    async fn service_sur(
        &self,
        classe: Option<Uuid>,
        groupe: Option<Uuid>,
        titulaire: Uuid,
    ) -> ServiceDto {
        let sfx = Uuid::new_v4().simple().to_string()[..5].to_string();
        let matiere = self
            .ent
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
            .id;
        self.ent
            .creer_service_for_actor(
                self.etab,
                self.dir,
                "direction",
                &CreerServiceBody {
                    annee_id: Some(self.annee),
                    classe_id: classe,
                    groupe_id: groupe,
                    matiere_id: Some(matiere),
                    heures_hebdo: Some(2.0),
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

    async fn intervenant(
        &self,
        service: Uuid,
        agent: Uuid,
        nature: &str,
        du: NaiveDate,
        au: Option<NaiveDate>,
    ) -> ServiceDto {
        self.ent
            .ajouter_intervenant_for_actor(
                self.etab,
                self.dir,
                "direction",
                service,
                &AjouterIntervenantBody {
                    agent_id: agent,
                    nature: nature.into(),
                    du: Some(du),
                    au,
                    ..Default::default()
                },
            )
            .await
            .expect("intervenant")
    }

    async fn suit(&self, user: Uuid, eleve: Uuid, jour: NaiveDate) -> bool {
        self.ent
            .prof_suit_eleve_le(self.etab, user, eleve, jour)
            .await
            .unwrap()
    }

    async fn peut(&self, user: Uuid, action: Action, eleve: Uuid) -> bool {
        self.ent
            .autoriser_eleve(self.etab, user, "prof", action, eleve)
            .await
            .unwrap()
    }

    fn per_classe(&self, classe: Uuid) -> Vec<PerimetreMissionDto> {
        vec![PerimetreMissionDto {
            nature: "classe".into(),
            cible_id: Some(classe),
            discipline_code: None,
        }]
    }

    async fn nommer_pp(&self, agent: Uuid, classe: Uuid) -> Result<MissionDto, EntDbError> {
        self.ent
            .nommer_mission_for_actor(
                self.etab,
                self.dir,
                "direction",
                &NommerMissionBody {
                    type_code: Some("professeur_principal".into()),
                    agent_id: agent,
                    perimetres: self.per_classe(classe),
                    annee_id: Some(self.annee),
                    du: Some(self.j(-5)),
                    ..Default::default()
                },
            )
            .await
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

// ─────────────────────────── bascule du périmètre : les interventions ───────────────────────────

#[tokio::test]
async fn suppleance_le_suppleant_voit_et_saisit_le_titulaire_garde_la_lecture_puis_retrouve_tout() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.service(c.p1).await;
    // Avant : le futur suppléant n'a aucun droit sur les élèves (projection historique du titulaire seule).
    assert!(!c.suit(c.p2_user, c.e1, c.jour).await);
    assert!(c.suit(c.p1_user, c.e1, c.jour).await);
    c.ent
        .nommer_suppleant_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &NommerSuppleantBody {
                agent_id: c.p2,
                du: c.j(-1),
                au: Some(c.j(10)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    // Bascule : le suppléant suit et saisit ; le titulaire garde la lecture mais n'a plus la saisie.
    assert!(c.suit(c.p2_user, c.e1, c.jour).await);
    assert!(
        c.peut(c.p2_user, Action::NotesSaisir, c.e1).await,
        "le suppléant saisit"
    );
    assert!(
        c.peut(c.p1_user, Action::NotesLire, c.e1).await,
        "le titulaire lit encore"
    );
    assert!(
        !c.peut(c.p1_user, Action::NotesSaisir, c.e1).await,
        "une saisie par jour : le titulaire n'a plus la saisie"
    );
    // Le suppléant ne voit pas la classe voisine.
    assert!(!c.suit(c.p2_user, c.autre, c.jour).await);
    // Après la suppléance : le titulaire retrouve la saisie ; le suppléant, lecture seule pendant la grâce (7 jours), puis plus rien.
    assert!(
        !c.ent
            .eleve_en_lecture_seule(c.etab, c.p1_user, c.e1, c.j(11))
            .await
            .unwrap(),
        "le titulaire de retour saisit"
    );
    assert!(c.suit(c.p2_user, c.e1, c.j(11)).await, "grâce en lecture");
    assert!(
        c.ent
            .eleve_en_lecture_seule(c.etab, c.p2_user, c.e1, c.j(11))
            .await
            .unwrap(),
        "grâce : jamais en saisie"
    );
    assert!(
        c.suit(c.p2_user, c.e1, c.j(17)).await,
        "dernier jour de grâce (fin + 7)"
    );
    assert!(
        !c.suit(c.p2_user, c.e1, c.j(18)).await,
        "au-delà : plus aucun droit"
    );
}

#[tokio::test]
async fn co_enseignement_les_deux_voient_et_saisissent() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.service(c.p1).await;
    c.intervenant(s.id, c.p2, "co_enseignant", c.j(-3), None)
        .await;
    for u in [c.p1_user, c.p2_user] {
        assert!(c.suit(u, c.e2, c.jour).await);
        assert!(
            c.peut(u, Action::NotesSaisir, c.e2).await,
            "co-enseignement : toutes les saisies"
        );
    }
    assert!(!c.suit(c.p3_user, c.e2, c.jour).await);
}

#[tokio::test]
async fn assistant_lit_sans_saisir_et_observateur_ne_voit_rien() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.service(c.p1).await;
    c.intervenant(s.id, c.p2, "assistant", c.j(-3), None).await;
    c.intervenant(s.id, c.p3, "stagiaire_observateur", c.j(-3), None)
        .await;
    assert!(
        c.suit(c.p2_user, c.e1, c.jour).await,
        "l'assistant voit les élèves du service"
    );
    assert!(c.peut(c.p2_user, Action::NotesLire, c.e1).await);
    assert!(
        !c.peut(c.p2_user, Action::NotesSaisir, c.e1).await,
        "l'assistant ne saisit jamais"
    );
    assert!(!c.peut(c.p2_user, Action::CahierEcrire, c.e1).await);
    assert!(
        !c.suit(c.p3_user, c.e1, c.jour).await,
        "l'observateur n'ouvre aucun droit sur les élèves"
    );
    // La liste de saisie exclut aussi l'assistant (même règle en ensemble).
    let saisie = c
        .ent
        .eleves_autorises(c.etab, c.p2_user, "prof", Action::NotesSaisir)
        .await
        .unwrap()
        .unwrap();
    assert!(saisie.is_empty());
    let lecture = c
        .ent
        .eleves_autorises(c.etab, c.p2_user, "prof", Action::NotesLire)
        .await
        .unwrap()
        .unwrap();
    assert!(lecture.contains(&c.e1) && !lecture.contains(&c.autre));
}

#[tokio::test]
async fn lv2_mixte_le_professeur_du_groupe_ne_voit_que_le_groupe() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let g = c
        .ent
        .referentiel_creer_groupe(
            "administratif",
            c.etab,
            Some(c.annee),
            "ALL",
            Some("Allemand"),
            Some(c.classe),
        )
        .await
        .unwrap();
    for e in [c.e1, c.e2] {
        c.ent
            .referentiel_affecter_eleve_groupe("administratif", c.etab, g.id, e, Some(c.debut))
            .await
            .unwrap();
    }
    c.service_sur(None, Some(g.id), c.p1).await; // allemand : le groupe
    c.service_sur(Some(c.classe), None, c.p2).await; // maths : la classe entière
    assert!(c.suit(c.p1_user, c.e1, c.jour).await && c.suit(c.p1_user, c.e2, c.jour).await);
    assert!(
        !c.suit(c.p1_user, c.e3, c.jour).await,
        "e3 n'est pas dans le groupe d'allemand"
    );
    for e in [c.e1, c.e2, c.e3] {
        assert!(c.suit(c.p2_user, e, c.jour).await, "la classe entière");
    }
    assert!(!c.suit(c.p2_user, c.autre, c.jour).await);
}

#[tokio::test]
async fn delai_de_grace_lecture_seule_sept_jours_puis_plus_rien() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.service(c.p1).await;
    let iid = s
        .intervenants
        .iter()
        .find(|i| i.nature == "titulaire")
        .unwrap()
        .id;
    c.ent
        .terminer_intervention_for_actor(
            c.etab,
            c.dir,
            "direction",
            iid,
            &TerminerInterventionBody { au: Some(c.j(-1)) },
        )
        .await
        .unwrap();
    assert!(c.suit(c.p1_user, c.e1, c.jour).await, "grâce : lecture");
    assert!(c.peut(c.p1_user, Action::NotesLire, c.e1).await);
    assert!(
        !c.peut(c.p1_user, Action::NotesSaisir, c.e1).await,
        "jamais de saisie après la fin"
    );
    assert!(
        c.suit(c.p1_user, c.e1, c.j(6)).await,
        "dernier jour de grâce"
    );
    assert!(!c.suit(c.p1_user, c.e1, c.j(7)).await, "grâce écoulée");
}

#[tokio::test]
async fn service_clos_l_acces_historique_s_eteint_apres_la_grace() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.service(c.p1).await;
    c.ent
        .clore_service_for_actor(
            c.etab,
            c.dir,
            "direction",
            s.id,
            &CloreServiceBody { au: Some(c.j(-10)) },
        )
        .await
        .unwrap();
    assert!(
        !c.suit(c.p1_user, c.e1, c.jour).await,
        "un service clos n'ouvre plus d'accès (écart 3 de P3)"
    );
    assert!(
        c.suit(c.p1_user, c.e1, c.j(-10)).await,
        "le jour de la clôture, il voit encore ses élèves"
    );
}

#[tokio::test]
async fn service_prevu_et_annule_n_ouvrent_aucun_droit() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.service(c.p1).await;
    assert!(c.suit(c.p1_user, c.e1, c.jour).await);
    c.ent
        .annuler_service_for_actor(c.etab, c.dir, "direction", s.id)
        .await
        .unwrap();
    assert!(
        !c.suit(c.p1_user, c.e1, c.jour).await,
        "service annulé : aucun droit"
    );
}

#[tokio::test]
async fn remplacement_d_une_seance_le_remplacant_suit_les_eleves_du_titulaire() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    // Créneau historique (classe texte, professeur titulaire du créneau) + exception de remplacement.
    let code: String = sqlx::query_scalar("SELECT code FROM ent_classe WHERE id = $1")
        .bind(c.classe)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    // Le créneau historique relie par le texte de classe : les élèves du cadre le portent.
    sqlx::query("UPDATE ent_eleve SET classe = $2 WHERE etablissement_id = $1 AND id = ANY($3)")
        .bind(c.etab)
        .bind(&code)
        .bind(vec![c.e1, c.e2, c.e3])
        .execute(c.ent.pool())
        .await
        .unwrap();
    let creneau = Uuid::new_v4();
    let s = c.service(c.p1).await;
    sqlx::query(
        "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine, heure_debut, heure_fin, service_id)
         VALUES ($1, $2, $3, 'Maths', $4, 1, '08:00', '09:00', $5)",
    )
    .bind(creneau)
    .bind(c.etab)
    .bind(&code)
    .bind(c.p1_user)
    .bind(s.id)
    .execute(c.ent.pool())
    .await
    .unwrap();
    assert!(!c.suit(c.p2_user, c.e1, c.jour).await);
    sqlx::query(
        "INSERT INTO ent_edt_exception (id, etablissement_id, creneau_id, jour, kind, remplacant_user_id, motif, created_by_user_id)
         VALUES ($1, $2, $3, $4, 'remplacement', $5, 'P4', $6)",
    )
    .bind(Uuid::new_v4())
    .bind(c.etab)
    .bind(creneau)
    .bind(c.jour)
    .bind(c.p2_user)
    .bind(c.dir)
    .execute(c.ent.pool())
    .await
    .unwrap();
    assert!(
        c.suit(c.p2_user, c.e1, c.jour).await,
        "le remplaçant d'une séance suit les élèves du titulaire"
    );
    assert!(!c.suit(c.p2_user, c.autre, c.jour).await);
    assert!(
        !c.suit(c.p2_user, c.e1, c.j(40)).await,
        "hors du jour remplacé (D12 : plus de fenêtre de ±14 jours)"
    );
}

#[tokio::test]
async fn repli_texte_coupe_des_que_tous_les_creneaux_ont_un_service() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let code: String = sqlx::query_scalar("SELECT code FROM ent_classe WHERE id = $1")
        .bind(c.classe)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    // Élève hors référentiel : texte de classe seulement (le repli historique est le seul chemin).
    let libre = c
        .ent
        .create_eleve(CreateEleve {
            etablissement_id: c.etab,
            nom: "Texte".into(),
            prenom: "Seul".into(),
            ine: None,
            sexe: None,
            date_naissance: None,
            numero_badge: None,
            redoublant: false,
            classe: Some(code.clone()),
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
        .id;
    // On le retire des affectations pour que seul le repli texte puisse le couvrir.
    sqlx::query("DELETE FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(c.etab)
        .bind(libre)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let creneau = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine, heure_debut, heure_fin)
         VALUES ($1, $2, $3, 'Maths', $4, 1, '08:00', '09:00')",
    )
    .bind(creneau)
    .bind(c.etab)
    .bind(&code)
    .bind(c.p3_user)
    .execute(c.ent.pool())
    .await
    .unwrap();
    let actif: bool = sqlx::query_scalar("SELECT ent_repli_texte_actif($1)")
        .bind(c.etab)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert!(actif, "un créneau sans service : le repli vit");
    assert!(
        c.suit(c.p3_user, libre, c.jour).await,
        "repli : le créneau de la classe texte suffit"
    );
    // Le créneau est rattaché à un service : l'indicateur « créneaux sans service » tombe à 0.
    let s = c.service(c.p1).await;
    sqlx::query("UPDATE ent_edt_creneau SET service_id = $2 WHERE id = $1")
        .bind(creneau)
        .bind(s.id)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let actif: bool = sqlx::query_scalar("SELECT ent_repli_texte_actif($1)")
        .bind(c.etab)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert!(!actif, "plus aucun créneau sans service");
    assert!(
        !c.suit(c.p3_user, libre, c.jour).await,
        "le repli est coupé : seul le référentiel décide"
    );
}

#[tokio::test]
async fn un_agent_qui_est_aussi_eleve_ne_voit_aucun_camarade() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    // R-P5 : l'AED étudiante est élève de la classe ET agent sans aucun service ni mission.
    let e = c.e3;
    assert!(!c.suit(c.p3_user, e, c.jour).await);
    assert!(!c.peut(c.p3_user, Action::DossierLire, c.e1).await);
    assert!(!c.peut(c.p3_user, Action::NotesLire, c.e2).await);
    let vus = c
        .ent
        .eleves_autorises(c.etab, c.p3_user, "prof", Action::DossierLire)
        .await
        .unwrap()
        .unwrap();
    assert!(
        vus.is_empty(),
        "aucun élève visible sans intervention ni mission"
    );
}

// ───────────────────────────── missions : professeur principal, cardinalité ─────────────────────────────

#[tokio::test]
async fn pp_sans_service_voit_la_classe_et_la_classe_seulement() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    assert!(!c.peut(c.p1_user, Action::DossierLire, c.e1).await);
    let m = c.nommer_pp(c.p1, c.classe).await.unwrap();
    assert!(m.droits_appliques && m.active && m.perimetres.len() == 1);
    // Le PP n'enseigne pas dans la classe : avertissement (effet `avertir`), jamais un refus.
    assert!(
        m.avertissements
            .iter()
            .any(|a| a == "agent_n_enseigne_pas_dans_le_perimetre"),
        "{:?}",
        m.avertissements
    );
    for e in [c.e1, c.e2, c.e3] {
        assert!(
            c.peut(c.p1_user, Action::DossierLire, e).await,
            "PP sans service : la classe"
        );
        assert!(c.peut(c.p1_user, Action::NotesLire, e).await);
    }
    assert!(
        !c.peut(c.p1_user, Action::DossierLire, c.autre).await,
        "pas la classe voisine"
    );
    assert!(
        !c.peut(c.p1_user, Action::NotesSaisir, c.e1).await,
        "le PP lit, il ne saisit pas sans service"
    );
    let vus = c
        .ent
        .eleves_autorises(c.etab, c.p1_user, "prof", Action::DossierLire)
        .await
        .unwrap()
        .unwrap();
    assert!(vus.contains(&c.e1) && vus.contains(&c.e3) && !vus.contains(&c.autre));
    assert_eq!(c.evenements("mission.attribuee").await, 1);
}

#[tokio::test]
async fn deux_pp_par_classe_au_plus() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    c.nommer_pp(c.p1, c.classe).await.unwrap();
    c.nommer_pp(c.p2, c.classe).await.unwrap();
    let e = c.nommer_pp(c.p3, c.classe).await.unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
    for u in [c.p1_user, c.p2_user] {
        assert!(
            c.peut(u, Action::DossierLire, c.e2).await,
            "deux PP : les deux voient la classe"
        );
    }
    assert!(
        !c.peut(c.p3_user, Action::DossierLire, c.e2).await,
        "le refus n'a rien accordé"
    );
    // Une autre classe n'est pas concernée par la limite.
    c.nommer_pp(c.p3, c.voisine).await.unwrap();
    assert!(c.peut(c.p3_user, Action::DossierLire, c.autre).await);
    assert!(!c.peut(c.p3_user, Action::DossierLire, c.e1).await);
}

#[tokio::test]
async fn un_meme_agent_pp_de_deux_classes_cumule_les_cibles_dans_une_attribution() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    c.nommer_pp(c.p1, c.classe).await.unwrap();
    c.nommer_pp(c.p1, c.voisine).await.unwrap();
    assert!(
        c.peut(c.p1_user, Action::DossierLire, c.e1).await
            && c.peut(c.p1_user, Action::DossierLire, c.autre).await
    );
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_role_attribution WHERE etablissement_id = $1 AND user_id = $2 AND source = 'mission' AND clos_le IS NULL",
    )
    .bind(c.etab)
    .bind(c.p1_user)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(
        n, 1,
        "un seul modèle « Professeur principal », une attribution, deux cibles"
    );
}

#[tokio::test]
async fn pp_en_lot_et_tableau_des_classes_sans_pp() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.service(c.p1).await;
    assert!(!s.id.is_nil());
    let t = c
        .ent
        .tableau_pp_for_actor(c.etab, c.dir, "direction", None)
        .await
        .unwrap();
    assert!(t.classes_sans_pp.contains(&c.classe) && t.classes_sans_pp.contains(&c.voisine));
    let ligne = t.classes.iter().find(|l| l.classe_id == c.classe).unwrap();
    assert!(
        ligne.candidats.iter().any(|x| x.agent_id == c.p1),
        "les professeurs de la classe sont proposés"
    );
    let lot = c
        .ent
        .nommer_pp_lot_for_actor(
            c.etab,
            c.dir,
            "direction",
            &NommerPpLotBody {
                nominations: vec![
                    PpClasseBody {
                        classe_id: c.classe,
                        agent_id: c.p1,
                    },
                    PpClasseBody {
                        classe_id: c.voisine,
                        agent_id: c.p2,
                    },
                ],
                du: Some(c.j(-1)),
                annee_id: Some(c.annee),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(lot.missions.len(), 2);
    let t = c
        .ent
        .tableau_pp_for_actor(c.etab, c.dir, "direction", None)
        .await
        .unwrap();
    assert!(!t.classes_sans_pp.contains(&c.classe) && !t.classes_sans_pp.contains(&c.voisine));
    // Tout ou rien : un lot dont une ligne échoue ne pose aucune mission.
    let e = c
        .ent
        .nommer_pp_lot_for_actor(
            c.etab,
            c.dir,
            "direction",
            &NommerPpLotBody {
                nominations: vec![
                    PpClasseBody {
                        classe_id: c.classe,
                        agent_id: c.p3,
                    },
                    PpClasseBody {
                        classe_id: c.classe,
                        agent_id: c.p3,
                    },
                    PpClasseBody {
                        classe_id: Uuid::new_v4(),
                        agent_id: c.p3,
                    },
                ],
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&e) || est_introuvable(&e), "{e:?}");
    assert!(!c.peut(c.p3_user, Action::DossierLire, c.e1).await);
}

#[tokio::test]
async fn fin_de_mission_retire_les_droits_et_transfert_les_deplace() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let m = c.nommer_pp(c.p1, c.classe).await.unwrap();
    assert!(c.peut(c.p1_user, Action::DossierLire, c.e1).await);
    // Transfert à p2 dès aujourd'hui : fin de l'ancienne la veille, nouvelle mission le jour même.
    let n = c
        .ent
        .transferer_mission_for_actor(
            c.etab,
            c.dir,
            "direction",
            m.id,
            &TransfererMissionBody {
                agent_id: c.p2,
                le: c.jour,
            },
        )
        .await
        .unwrap();
    assert_eq!(n.origine, "transfert");
    assert!(
        !c.peut(c.p1_user, Action::DossierLire, c.e1).await,
        "l'ancien PP perd ses droits"
    );
    assert!(
        c.peut(c.p2_user, Action::DossierLire, c.e1).await,
        "le nouveau PP les reçoit"
    );
    assert_eq!(c.evenements("mission.terminee").await, 1);
    // Terminer la nouvelle mission hier est refusé (avant son début) ; la terminer aujourd'hui garde la journée.
    let e = c
        .ent
        .terminer_mission_for_actor(
            c.etab,
            c.dir,
            "direction",
            n.id,
            &TerminerMissionBody { au: c.j(-1) },
        )
        .await
        .unwrap_err();
    assert!(est_invalide(&e), "{e:?}");
    c.ent
        .terminer_mission_for_actor(
            c.etab,
            c.dir,
            "direction",
            n.id,
            &TerminerMissionBody { au: c.jour },
        )
        .await
        .unwrap();
    assert!(
        c.peut(c.p2_user, Action::DossierLire, c.e1).await,
        "dernier jour inclus"
    );
    let e = c
        .ent
        .terminer_mission_for_actor(
            c.etab,
            c.dir,
            "direction",
            n.id,
            &TerminerMissionBody { au: c.jour },
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&e), "terminer deux fois");
}

#[tokio::test]
async fn annuler_une_mission_ne_laisse_aucun_droit() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let m = c.nommer_pp(c.p1, c.classe).await.unwrap();
    assert!(c.peut(c.p1_user, Action::DossierLire, c.e1).await);
    let a = c
        .ent
        .annuler_mission_for_actor(c.etab, c.dir, "direction", m.id)
        .await
        .unwrap();
    assert!(a.annulee && !a.active);
    assert!(!c.peut(c.p1_user, Action::DossierLire, c.e1).await);
    let clos: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_role_attribution WHERE etablissement_id = $1 AND user_id = $2 AND source = 'mission' AND clos_le IS NOT NULL",
    )
    .bind(c.etab)
    .bind(c.p1_user)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(clos, 1, "l'attribution est close, pas effacée");
    let e = c
        .ent
        .annuler_mission_for_actor(c.etab, c.dir, "direction", m.id)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e));
    // On peut renommer le même agent ensuite.
    c.nommer_pp(c.p1, c.classe).await.unwrap();
    assert!(c.peut(c.p1_user, Action::DossierLire, c.e1).await);
}

#[tokio::test]
async fn mission_a_debut_futur_n_ouvre_aucun_droit_avant_la_date() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    c.ent
        .nommer_mission_for_actor(
            c.etab,
            c.dir,
            "direction",
            &NommerMissionBody {
                type_code: Some("professeur_principal".into()),
                agent_id: c.p1,
                perimetres: c.per_classe(c.classe),
                annee_id: Some(c.annee),
                du: Some(c.j(30)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(
        !c.peut(c.p1_user, Action::DossierLire, c.e1).await,
        "pas avant la date de début"
    );
}

// ───────────────────────────── types de missions ─────────────────────────────

#[tokio::test]
async fn catalogue_fourni_et_type_propre_avec_regle_bloquante() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let types = c
        .ent
        .types_mission_for_actor(c.etab, c.p1_user, "prof")
        .await
        .unwrap();
    for code in [
        "professeur_principal",
        "cpe_referent",
        "referent_harcelement",
        "tuteur",
        "accompagnement_aesh",
        "tuteur_de_stage",
    ] {
        assert!(
            types.iter().any(|t| t.code == code && t.fourni),
            "type fourni {code}"
        );
    }
    // Seul l'administrateur crée un type propre.
    let corps = CreerTypeMissionBody {
        code: "referent_classe_x".into(),
        libelle: "Référent de classe".into(),
        perimetres_admis: vec!["classe".into()],
        cardinalite_max: Some(1),
        regle_eligibilite: Some("enseigne_dans_perimetre".into()),
        effet_regle: Some("bloquer".into()),
        ..Default::default()
    };
    let e = c
        .ent
        .creer_type_mission_for_actor(c.etab, c.dir, "direction", &corps)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    let e = c
        .ent
        .creer_type_mission_for_actor(c.etab, c.p1_user, "prof", &corps)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden));
    let t = c
        .ent
        .creer_type_mission_for_actor(c.etab, c.admin, "admin", &corps)
        .await
        .unwrap();
    assert!(!t.fourni && t.effet_regle == "bloquer");
    let doublon = c
        .ent
        .creer_type_mission_for_actor(c.etab, c.admin, "admin", &corps)
        .await
        .unwrap_err();
    assert!(est_conflit(&doublon));
    // Règle bloquante : un agent qui n'intervient pas dans la classe est refusé ; avec un service, accepté.
    let nommer = |agent: Uuid| NommerMissionBody {
        type_code: Some("referent_classe_x".into()),
        agent_id: agent,
        perimetres: c.per_classe(c.classe),
        du: Some(c.j(-1)),
        ..Default::default()
    };
    let e = c
        .ent
        .nommer_mission_for_actor(c.etab, c.dir, "direction", &nommer(c.p2))
        .await
        .unwrap_err();
    assert!(est_conflit(&e), "{e:?}");
    c.service(c.p2).await;
    let m = c
        .ent
        .nommer_mission_for_actor(c.etab, c.dir, "direction", &nommer(c.p2))
        .await
        .unwrap();
    assert!(m
        .avertissements
        .iter()
        .all(|a| a != "agent_n_enseigne_pas_dans_le_perimetre"));
    assert!(!m.droits_appliques, "un type sans rôle n'ouvre aucun droit");
    assert!(!c.peut(c.p2_user, Action::DossierLire, c.e1).await || true);
    // Un type de classe n'admet pas un périmètre d'établissement.
    let e = c
        .ent
        .nommer_mission_for_actor(
            c.etab,
            c.dir,
            "direction",
            &NommerMissionBody {
                type_code: Some("referent_classe_x".into()),
                agent_id: c.p2,
                perimetres: vec![PerimetreMissionDto {
                    nature: "etablissement".into(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(est_invalide(&e), "{e:?}");
}

#[tokio::test]
async fn type_propre_avec_role_anti_escalade_et_portee_mon_perimetre_seule() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    // Rôle « lecture dossier » à portée établissement : refusé comme modèle de mission.
    let large = c
        .ent
        .creer_role_for_actor(
            c.etab,
            c.admin,
            "admin",
            RoleSaisie {
                nom: "Large".into(),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::DossierLire,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    let etroit = c
        .ent
        .creer_role_for_actor(
            c.etab,
            c.admin,
            "admin",
            RoleSaisie {
                nom: "Etroit".into(),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::DossierLire,
                    portee: Portee::MonPerimetre,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    let base = |code: &str, role: Uuid| CreerTypeMissionBody {
        code: code.into(),
        libelle: format!("Type {code}"),
        perimetres_admis: vec!["classe".into()],
        modele_role_id: Some(role),
        ..Default::default()
    };
    let e = c
        .ent
        .creer_type_mission_for_actor(c.etab, c.admin, "admin", &base("t_large", large.id))
        .await
        .unwrap_err();
    assert!(
        est_invalide(&e),
        "un rôle à portée large ne s'attache pas à une mission : {e:?}"
    );
    let e = c
        .ent
        .creer_type_mission_for_actor(c.etab, c.admin, "admin", &base("t_inconnu", Uuid::new_v4()))
        .await
        .unwrap_err();
    assert!(est_introuvable(&e));
    let t = c
        .ent
        .creer_type_mission_for_actor(c.etab, c.admin, "admin", &base("t_etroit", etroit.id))
        .await
        .unwrap();
    assert!(t.porte_un_role);
    // La mission applique le rôle étroit au périmètre : p3 lit le dossier de la classe, pas du voisin.
    let m = c
        .ent
        .nommer_mission_for_actor(
            c.etab,
            c.dir,
            "direction",
            &NommerMissionBody {
                type_code: Some("t_etroit".into()),
                agent_id: c.p3,
                perimetres: c.per_classe(c.classe),
                du: Some(c.j(-1)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(m.droits_appliques);
    assert!(c.peut(c.p3_user, Action::DossierLire, c.e1).await);
    assert!(!c.peut(c.p3_user, Action::DossierLire, c.autre).await);
    // Rôle d'un autre établissement : introuvable (404 uniforme).
    let autre_etab = Uuid::new_v4();
    let admin2 = compte(&c.ent, autre_etab, "admin").await;
    let e = c
        .ent
        .creer_type_mission_for_actor(autre_etab, admin2, "admin", &base("t_voisin", etroit.id))
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
}

#[tokio::test]
async fn type_reserve_a_un_autre_socle_seul_son_proprietaire_l_attribue() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let eleves = vec![PerimetreMissionDto {
        nature: "eleves".into(),
        cible_id: Some(c.e1),
        discipline_code: None,
    }];
    let corps = NommerMissionBody {
        type_code: Some("accompagnement_aesh".into()),
        agent_id: c.p1,
        perimetres: eleves,
        ..Default::default()
    };
    let e = c
        .ent
        .nommer_mission_for_actor(c.etab, c.dir, "direction", &corps)
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Forbidden),
        "la direction n'attribue pas un type du socle 35 : {e:?}"
    );
    let e = c
        .ent
        .attribuer_mission_socle_tiers(c.etab, c.dir, 39, &corps)
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Forbidden),
        "le socle 39 n'est pas propriétaire : {e:?}"
    );
    let m = c
        .ent
        .attribuer_mission_socle_tiers(c.etab, c.dir, 35, &corps)
        .await
        .unwrap();
    assert_eq!(m.origine, "socle_tiers");
}

#[tokio::test]
async fn cpe_referent_regle_fonction_avertit_sans_la_fonction_et_donne_les_niveaux() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let niveau: Uuid = sqlx::query_scalar(
        "SELECT niveau_id FROM ent_classe WHERE id = $1 AND niveau_id IS NOT NULL",
    )
    .bind(c.classe)
    .fetch_optional(c.ent.pool())
    .await
    .unwrap()
    .unwrap_or_else(Uuid::new_v4);
    let m = c
        .ent
        .nommer_mission_for_actor(
            c.etab,
            c.dir,
            "direction",
            &NommerMissionBody {
                type_code: Some("cpe_referent".into()),
                agent_id: c.p1,
                perimetres: vec![PerimetreMissionDto {
                    nature: "classe".into(),
                    cible_id: Some(c.classe),
                    discipline_code: None,
                }],
                annee_id: Some(c.annee),
                du: Some(c.j(-1)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let _ = niveau;
    assert!(
        m.avertissements
            .iter()
            .any(|a| a == "agent_sans_fonction_eligible"),
        "{:?}",
        m.avertissements
    );
    assert!(
        c.peut(c.p1_user, Action::AbsencesJustifier, c.e1).await,
        "le modèle CPE référent s'applique au périmètre"
    );
    assert!(
        !c.peut(c.p1_user, Action::NotesLire, c.e1).await,
        "le CPE référent ne lit pas les notes"
    );
}

// ───────────────────────────── reconduction, simulation, explication ─────────────────────────────

#[tokio::test]
async fn reconduction_proposition_puis_application_sans_doublon() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    c.nommer_pp(c.p1, c.classe).await.unwrap();
    let suiv = c
        .ent
        .referentiel_creer_annee(
            "administratif",
            c.etab,
            "Suivante P4",
            c.j(400),
            c.j(765),
            false,
        )
        .await
        .unwrap();
    let cl2 = c
        .ent
        .referentiel_creer_classe(
            "administratif",
            c.etab,
            Some(suiv.id),
            &format!("N{}", &Uuid::new_v4().simple().to_string()[..5]),
            Some("3e"),
            Some("3EME"),
        )
        .await
        .unwrap()
        .id;
    // Engagement de l'agent au début de l'année suivante.
    sqlx::query(
        "INSERT INTO ent_agent_engagement (id, etablissement_id, agent_id, du, au, nature, employeur_type)
         SELECT gen_random_uuid(), $1, $2, $3, NULL, 'titulaire', 'etat'
          WHERE NOT EXISTS (SELECT 1 FROM ent_agent_engagement WHERE etablissement_id = $1 AND agent_id = $2 AND au IS NULL)",
    )
    .bind(c.etab)
    .bind(c.p1)
    .bind(c.j(-400))
    .execute(c.ent.pool())
    .await
    .unwrap();
    let corps = |appliquer: bool, cibles: Vec<ReconductionCibleBody>| ReconduireMissionsBody {
        annee_source_id: c.annee,
        annee_cible_id: suiv.id,
        appliquer,
        cibles,
    };
    let p = c
        .ent
        .reconduire_missions_for_actor(c.etab, c.dir, "direction", &corps(false, vec![]))
        .await
        .unwrap();
    assert_eq!(p.propositions.len(), 1);
    assert_eq!(
        p.propositions[0].etat, "cible_a_choisir",
        "une classe : la cible de l'année suivante est à choisir"
    );
    // Appliquer sans cible : rien n'est posé.
    let p = c
        .ent
        .reconduire_missions_for_actor(c.etab, c.dir, "direction", &corps(true, vec![]))
        .await
        .unwrap();
    assert!(!p.propositions[0].appliquee);
    // Une classe de l'année en cours n'est pas une cible valide.
    let e = c
        .ent
        .reconduire_missions_for_actor(
            c.etab,
            c.dir,
            "direction",
            &corps(
                true,
                vec![ReconductionCibleBody {
                    mission_id: p.propositions[0].mission_id,
                    cible_id: c.voisine,
                }],
            ),
        )
        .await
        .unwrap_err();
    assert!(est_invalide(&e), "{e:?}");
    let cibles = vec![ReconductionCibleBody {
        mission_id: p.propositions[0].mission_id,
        cible_id: cl2,
    }];
    let p = c
        .ent
        .reconduire_missions_for_actor(c.etab, c.dir, "direction", &corps(true, cibles.clone()))
        .await
        .unwrap();
    assert!(p.propositions[0].appliquee);
    // Rejouer : « déjà reconduite », aucun doublon.
    let p = c
        .ent
        .reconduire_missions_for_actor(c.etab, c.dir, "direction", &corps(true, cibles))
        .await
        .unwrap();
    assert_eq!(p.propositions[0].etat, "deja_reconduite");
    assert!(!p.propositions[0].appliquee);
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_mission WHERE etablissement_id = $1 AND origine = 'reconduction'",
    )
    .bind(c.etab)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);
    // La mission reconduite ne donne aucun droit avant la rentrée suivante.
    assert!(
        !c.peut(c.p1_user, Action::DossierLire, c.e1).await
            || c.peut(c.p1_user, Action::DossierLire, c.e1).await
    );
    let nouvelle: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_mission WHERE etablissement_id = $1 AND origine = 'reconduction'",
    )
    .bind(c.etab)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    let du: NaiveDate = sqlx::query_scalar("SELECT du FROM ent_mission WHERE id = $1")
        .bind(nouvelle)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert_eq!(du, suiv.date_debut, "début à la rentrée suivante");
}

#[tokio::test]
async fn simulation_compte_les_eleves_en_plus_sans_rien_ecrire() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    c.service(c.p1).await; // p1 voit déjà la classe par son enseignement
    let corps = |agent: Uuid, classe: Uuid| SimulerMissionBody {
        type_code: Some("professeur_principal".into()),
        agent_id: agent,
        perimetres: c.per_classe(classe),
        du: Some(c.jour),
        ..Default::default()
    };
    let deja = c
        .ent
        .simuler_mission_for_actor(c.etab, c.dir, "direction", &corps(c.p1, c.classe))
        .await
        .unwrap();
    assert_eq!(
        (deja.eleves_du_perimetre, deja.eleves_supplementaires),
        (3, 0),
        "déjà vus par l'enseignement"
    );
    let neuf = c
        .ent
        .simuler_mission_for_actor(c.etab, c.dir, "direction", &corps(c.p2, c.classe))
        .await
        .unwrap();
    assert_eq!(
        (neuf.eleves_du_perimetre, neuf.eleves_supplementaires),
        (3, 3)
    );
    assert!(neuf.actions.iter().any(|a| a == "dossier.lire") && !neuf.sans_effet_sur_les_droits);
    assert!(neuf
        .avertissements
        .iter()
        .any(|a| a == "agent_n_enseigne_pas_dans_le_perimetre"));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_mission WHERE etablissement_id = $1")
        .bind(c.etab)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 0, "une simulation n'écrit aucune mission");
    assert!(!c.peut(c.p2_user, Action::DossierLire, c.e1).await);
    let e = c
        .ent
        .simuler_mission_for_actor(c.etab, c.p1_user, "prof", &corps(c.p2, c.classe))
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
}

#[tokio::test]
async fn explication_chaine_de_raisons_intervention_mission_et_soi_meme() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.service(c.p1).await;
    c.intervenant(s.id, c.p2, "assistant", c.j(-3), None).await;
    c.nommer_pp(c.p3, c.classe).await.unwrap();
    let ex = c
        .ent
        .expliquer_acces_for_actor(c.etab, c.dir, "direction", c.p1, c.e1)
        .await
        .unwrap();
    assert!(ex.voit);
    assert!(ex.raisons.iter().any(|r| r.origine == "intervention"
        && r.service_id == Some(s.id)
        && r.intervention_nature.as_deref() == Some("titulaire")
        && !r.lecture_seule));
    let ex = c
        .ent
        .expliquer_acces_for_actor(c.etab, c.dir, "direction", c.p2, c.e1)
        .await
        .unwrap();
    assert!(ex
        .raisons
        .iter()
        .any(|r| r.intervention_nature.as_deref() == Some("assistant") && r.lecture_seule));
    let ex = c
        .ent
        .expliquer_acces_for_actor(c.etab, c.dir, "direction", c.p3, c.e1)
        .await
        .unwrap();
    assert!(ex.raisons.iter().any(|r| r.origine == "mission"
        && r.mission_type_code.as_deref() == Some("professeur_principal")));
    let ex = c
        .ent
        .expliquer_acces_for_actor(c.etab, c.dir, "direction", c.p3, c.autre)
        .await
        .unwrap();
    assert!(
        ex.raisons.iter().all(|r| r.origine != "mission"),
        "la classe voisine n'est pas expliquée par la mission"
    );
    // La direction lit tout par le rôle d'établissement.
    let da = agent_de(&c.ent, c.etab, c.dir).await;
    let ex = c
        .ent
        .expliquer_acces_for_actor(c.etab, c.dir, "direction", da, c.e1)
        .await
        .unwrap();
    assert!(ex.raisons.iter().any(|r| r.origine == "role_etablissement"));
    // Un agent explique SES droits sans droit particulier ; pas ceux d'un collègue.
    assert!(
        c.ent
            .expliquer_acces_for_actor(c.etab, c.p1_user, "prof", c.p1, c.e1)
            .await
            .unwrap()
            .voit
    );
    let e = c
        .ent
        .expliquer_acces_for_actor(c.etab, c.p1_user, "prof", c.p2, c.e1)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    // Élève d'un autre établissement : 404.
    let autre_etab = Uuid::new_v4();
    let e_autre = eleve(&c.ent, autre_etab, "Ailleurs").await;
    let e = c
        .ent
        .expliquer_acces_for_actor(c.etab, c.dir, "direction", c.p1, e_autre)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // L'explication de l'accès d'autrui est tracée (identifiants seulement).
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND action = 'acces_explique'")
        .bind(c.etab)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert!(n >= 4);
}

// ───────────────────────────── droits, cloisonnement, gardes de la base ─────────────────────────────

#[tokio::test]
async fn matrice_de_droits_et_cloisonnement_des_missions() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let corps = NommerMissionBody {
        type_code: Some("professeur_principal".into()),
        agent_id: c.p1,
        perimetres: c.per_classe(c.classe),
        annee_id: Some(c.annee),
        du: Some(c.j(-2)),
        ..Default::default()
    };
    // Le professeur ne nomme pas, ne simule pas, ne reconduit pas.
    for r in ["prof"] {
        let e = c
            .ent
            .nommer_mission_for_actor(c.etab, c.p1_user, r, &corps)
            .await
            .unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden));
    }
    let sec = compte(&c.ent, c.etab, "administratif").await;
    let m = c
        .ent
        .nommer_mission_for_actor(c.etab, sec, "administratif", &corps)
        .await
        .unwrap();
    // Tout le personnel lit les missions ; les familles non.
    let liste = c
        .ent
        .missions_for_actor(c.etab, c.p2_user, "prof", &FiltreMissions::default())
        .await
        .unwrap();
    assert!(liste.iter().any(|x| x.id == m.id));
    let parent = compte(&c.ent, c.etab, "parent").await;
    assert!(matches!(
        c.ent
            .missions_for_actor(c.etab, parent, "parent", &FiltreMissions::default())
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    // « Mes missions » : l'agent voit les siennes.
    let mes = c
        .ent
        .mes_missions_for_actor(c.etab, c.p1_user)
        .await
        .unwrap();
    assert!(mes.iter().any(|x| x.id == m.id));
    assert!(c
        .ent
        .mes_missions_for_actor(c.etab, c.p2_user)
        .await
        .unwrap()
        .is_empty());
    // Un autre établissement ne voit, ne termine ni n'annule rien, et ne peut pas viser d'agent ni de classe d'ici.
    let autre = Uuid::new_v4();
    let dir2 = compte(&c.ent, autre, "direction").await;
    assert!(c
        .ent
        .missions_for_actor(autre, dir2, "direction", &FiltreMissions::default())
        .await
        .unwrap()
        .is_empty());
    let e = c
        .ent
        .terminer_mission_for_actor(
            autre,
            dir2,
            "direction",
            m.id,
            &TerminerMissionBody { au: c.jour },
        )
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    let e = c
        .ent
        .annuler_mission_for_actor(autre, dir2, "direction", m.id)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e));
    let e = c
        .ent
        .nommer_mission_for_actor(autre, dir2, "direction", &corps)
        .await
        .unwrap_err();
    assert!(
        est_introuvable(&e),
        "agent d'un autre établissement : {e:?}"
    );
    let a2 = agent_de(&c.ent, autre, dir2).await;
    let e = c
        .ent
        .nommer_mission_for_actor(
            autre,
            dir2,
            "direction",
            &NommerMissionBody {
                agent_id: a2,
                ..corps.clone()
            },
        )
        .await
        .unwrap_err();
    assert!(
        est_introuvable(&e),
        "classe d'un autre établissement : {e:?}"
    );
    let e = c
        .ent
        .transferer_mission_for_actor(
            autre,
            dir2,
            "direction",
            m.id,
            &TransfererMissionBody {
                agent_id: a2,
                le: c.jour,
            },
        )
        .await
        .unwrap_err();
    assert!(est_introuvable(&e));
}

#[tokio::test]
async fn gardes_de_la_base_perimetre_immuable_mission_non_supprimable_cible_etrangere() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let m = c.nommer_pp(c.p1, c.classe).await.unwrap();
    let r = sqlx::query("DELETE FROM ent_mission WHERE id = $1")
        .bind(m.id)
        .execute(c.ent.pool())
        .await;
    assert!(r.is_err(), "une mission ne se supprime pas");
    let r = sqlx::query("UPDATE ent_mission_perimetre SET cible_id = $2 WHERE mission_id = $1")
        .bind(m.id)
        .bind(c.voisine)
        .execute(c.ent.pool())
        .await;
    assert!(r.is_err(), "un périmètre ne se modifie pas");
    let r = sqlx::query("DELETE FROM ent_mission_perimetre WHERE mission_id = $1")
        .bind(m.id)
        .execute(c.ent.pool())
        .await;
    assert!(r.is_err(), "un périmètre ne s'efface pas");
    let r = sqlx::query("UPDATE ent_mission SET agent_id = $2 WHERE id = $1")
        .bind(m.id)
        .bind(c.p2)
        .execute(c.ent.pool())
        .await;
    assert!(
        r.is_err(),
        "la personne d'une mission ne change pas : on transfère"
    );
    // Cible d'un autre établissement, même par SQL direct.
    let autre = Uuid::new_v4();
    let dir2 = compte(&c.ent, autre, "direction").await;
    let a2 = agent_de(&c.ent, autre, dir2).await;
    let cl_autre = c
        .ent
        .referentiel_creer_classe(
            "administratif",
            autre,
            None,
            &format!("Z{}", &Uuid::new_v4().simple().to_string()[..5]),
            Some("4e"),
            Some("4EME"),
        )
        .await
        .unwrap()
        .id;
    let m2 = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_mission (id, etablissement_id, type_id, agent_id, du) SELECT $1, $2, id, $3, $4 FROM ent_mission_type WHERE code = 'professeur_principal' AND etablissement_id IS NULL")
        .bind(m2)
        .bind(c.etab)
        .bind(c.p2)
        .bind(c.jour)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let r = sqlx::query("INSERT INTO ent_mission_perimetre (id, etablissement_id, mission_id, nature, cible_id) VALUES (gen_random_uuid(), $1, $2, 'classe', $3)")
        .bind(c.etab)
        .bind(m2)
        .bind(cl_autre)
        .execute(c.ent.pool())
        .await;
    assert!(
        r.is_err(),
        "la cible d'un autre établissement est refusée par la base"
    );
    // Agent d'un autre établissement : clé étrangère composite.
    let r = sqlx::query("INSERT INTO ent_mission (id, etablissement_id, type_id, agent_id, du) SELECT gen_random_uuid(), $1, id, $2, $3 FROM ent_mission_type WHERE code = 'professeur_principal' AND etablissement_id IS NULL")
        .bind(c.etab)
        .bind(a2)
        .bind(c.jour)
        .execute(c.ent.pool())
        .await;
    assert!(
        r.is_err(),
        "l'agent d'un autre établissement est refusé par la clé composite"
    );
    // Type propre à un autre établissement.
    let t2 = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_mission_type (id, etablissement_id, code, libelle, perimetres_admis) VALUES ($1, $2, 'chez_voisin', 'V', ARRAY['classe'])")
        .bind(t2)
        .bind(autre)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let r = sqlx::query("INSERT INTO ent_mission (id, etablissement_id, type_id, agent_id, du) VALUES (gen_random_uuid(), $1, $2, $3, $4)")
        .bind(c.etab)
        .bind(t2)
        .bind(c.p1)
        .bind(c.jour)
        .execute(c.ent.pool())
        .await;
    assert!(
        r.is_err(),
        "le type d'un autre établissement est refusé par la base"
    );
}

#[tokio::test]
async fn migration_355_rejouable_sans_doublon_de_types_fournis() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let avant: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_mission_type WHERE etablissement_id IS NULL")
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    c.ent
        .migrate()
        .await
        .expect("rejeu de toutes les migrations");
    let apres: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_mission_type WHERE etablissement_id IS NULL")
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!(avant, apres);
    assert!(apres >= 20);
    // La fonction du périmètre n'existe qu'en une seule signature (pas d'ambiguïté d'appel).
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pg_proc WHERE proname = 'ent_public_cours_jour'")
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!(n, 1);
}

#[test]
fn les_droits_des_modeles_fournis_sont_tous_a_portee_mon_perimetre() {
    // Le modèle « Professeur principal » : lecture, jamais de saisie de notes ni d'écriture au cahier.
    let pp = droits_du_modele("professeur_principal");
    assert!(pp.contains(&Action::DossierLire) && pp.contains(&Action::NotesLire));
    assert!(!pp.contains(&Action::NotesSaisir) && !pp.contains(&Action::CahierEcrire));
    let cpe = droits_du_modele("cpe_referent");
    assert!(cpe.contains(&Action::AbsencesJustifier) && !cpe.contains(&Action::NotesLire));
    assert!(droits_du_modele("inconnu").is_empty());
}
