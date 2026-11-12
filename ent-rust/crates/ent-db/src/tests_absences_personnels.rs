//! Socle 15, lots A0 et A1 — absences des personnels, contre la base locale de test (feature `labo`).
//! Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! Un établissement neuf par test (isolation, pas de dépendance d'ordre). Chaque anomalie corrigée a son
//! test (règle 14 de `docs/PILOTAGE.md`), nommé d'après le défaut qu'il verrouille :
//!
//! * A0 : AR-01 (motif lisible de tous), AR-02 (motif libre des exceptions), AR-04 (e-mail vers les
//!   familles), AR-06 (auto-validation), AR-08/14/18 (route historique), AR-09 (concurrence), AR-11
//!   (annulation d'une absence non validée ou d'un autre), AR-13 (auteur écrasé), AR-15 (levée par
//!   suppression), AR-16 (refus, annulation), AR-17 (journal) ;
//! * A1 : circuit figé, double engagement, signalement non confirmé (R-AP46), nature jamais hors du cercle
//!   (R-AP2, R-AP3), circuit externe (R-AP47), natures non discrétionnaires (R-AP12), faits (requalifier,
//!   prolonger, écourter, annuler, rouvrir, traiter, clore), port « absences validées non remplacées »
//!   (D-058), droits, cloisonnement d'établissement, ajout seul.

use super::*;
use crate::edt_remplacement::CmdRemplacement;
use crate::ref_commun::Acteur;
use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use domain_eleve::{
    today_utc, DeclarerAbsenceRh, DomainError, EdtExceptionKind, PoserEdtException,
};
use ent_types::absences_personnels::*;
use uuid::Uuid;

const MDP: &str = "MotDePasse-A15-Aa9!";

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let email = format!("a15-{role}-{}@ent.local", Uuid::new_v4().simple());
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

/// Lundi de la semaine J+14 (ou, en fin d'année scolaire, de la dernière semaine pleine).
fn lundi_futur(debut: NaiveDate, fin: NaiveDate) -> NaiveDate {
    let lundi_de = |d: NaiveDate| d - Duration::days(d.weekday().num_days_from_monday() as i64);
    let mut l = lundi_de(today_utc() + Duration::days(14));
    if l + Duration::days(13) > fin {
        l = lundi_de(fin) - Duration::days(14);
    }
    assert!(l >= debut, "année scolaire trop courte pour le test");
    l
}

struct Cadre {
    ent: PgEnt,
    etab: Uuid,
    dir: Uuid,
    sec: Uuid,
    compta: Uuid,
    accueil: Uuid,
    cpe: Uuid,
    admin: Uuid,
    prof: Uuid,
    prof2: Uuid,
    prof3: Uuid,
    lundi: NaiveDate,
    classe: String,
    classe_id: Uuid,
}

impl Cadre {
    async fn nouveau() -> Option<Cadre> {
        let ent = connecter().await?;
        let etab = Uuid::new_v4();
        let dir = compte(&ent, etab, "direction").await;
        let sec = compte(&ent, etab, "administratif").await;
        let compta = compte(&ent, etab, "compta").await;
        let accueil = compte(&ent, etab, "accueil").await;
        let cpe = compte(&ent, etab, "cpe").await;
        let admin = compte(&ent, etab, "admin").await;
        let prof = compte(&ent, etab, "prof").await;
        let prof2 = compte(&ent, etab, "prof").await;
        let prof3 = compte(&ent, etab, "prof").await;
        let annee = ent.referentiel_annee_courante_assuree(etab).await.unwrap();
        let sfx = Uuid::new_v4().simple().to_string()[..6].to_string();
        let classe = format!("A{sfx}");
        let classe_id = ent
            .referentiel_creer_classe(
                "administratif",
                etab,
                Some(annee.id),
                &classe,
                Some("4e"),
                Some("4EME"),
            )
            .await
            .unwrap()
            .id;
        let lundi = lundi_futur(annee.date_debut, annee.date_fin);
        Some(Cadre {
            ent,
            etab,
            dir,
            sec,
            compta,
            accueil,
            cpe,
            admin,
            prof,
            prof2,
            prof3,
            lundi,
            classe,
            classe_id,
        })
    }

    fn jour(&self, iso: i64) -> NaiveDate {
        self.lundi + Duration::days(iso - 1)
    }

    fn acteur<'a>(&self, user: Uuid, role: &'a str) -> Acteur<'a> {
        Acteur {
            etablissement_id: self.etab,
            user_id: user,
            role,
        }
    }

    fn d(&self) -> Acteur<'static> {
        self.acteur(self.dir, "direction")
    }

    async fn agent(&self, user: Uuid) -> Uuid {
        agent_de(&self.ent, self.etab, user).await
    }

    /// Un créneau hebdomadaire pour `prof` dans la classe du cadre (ou `classe`).
    async fn creneau(
        &self,
        classe: &str,
        prof: Uuid,
        jour_iso: i16,
        d: (u32, u32),
        f: (u32, u32),
    ) -> Uuid {
        let s = CreneauSaisie {
            classe: classe.into(),
            groupe_id: None,
            matiere_libelle: "Mathématiques".into(),
            prof_user_id: prof,
            jour_semaine: jour_iso,
            heure_debut: t(d.0, d.1),
            heure_fin: t(f.0, f.1),
            salle: None,
            site: String::new(),
            semaine_ab: "toutes".into(),
        };
        match self
            .ent
            .edt_creneau_creer(&self.d(), &s, false)
            .await
            .unwrap()
        {
            ResultatCreneau::Enregistre { creneau: Some(c) } => c.id,
            autre => panic!("créneau refusé : {autre:?}"),
        }
    }

    async fn classe_bis(&self) -> String {
        let annee = self
            .ent
            .referentiel_annee_courante_assuree(self.etab)
            .await
            .unwrap();
        let code = format!("B{}", &Uuid::new_v4().simple().to_string()[..6]);
        self.ent
            .referentiel_creer_classe(
                "administratif",
                self.etab,
                Some(annee.id),
                &code,
                Some("4e"),
                Some("4EME"),
            )
            .await
            .unwrap();
        code
    }

    /// Déclaration de la propre absence de `user` (agent connecté), nature `code`, jours inclusifs.
    async fn declarer(
        &self,
        user: Uuid,
        role: &str,
        code: Option<&str>,
        j1: i64,
        j2: i64,
    ) -> DeclarationDto {
        self.ent
            .declarer_absence_for_actor(
                self.etab,
                user,
                role,
                &DeclarerAbsenceBody {
                    nature_code: code.map(str::to_string),
                    debut: self.jour(j1),
                    fin: self.jour(j2),
                    ..Default::default()
                },
            )
            .await
            .unwrap_or_else(|e| panic!("déclaration refusée : {e:?}"))
    }

    async fn valider(&self, id: Uuid) -> AbsenceDto {
        self.ent
            .valider_absence_for_actor(
                self.etab,
                self.dir,
                "direction",
                id,
                &DecisionBody::default(),
            )
            .await
            .unwrap_or_else(|e| panic!("validation refusée : {e:?}"))
    }

    async fn nb_events(&self, type_name: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = $2",
        )
        .bind(self.etab)
        .bind(type_name)
        .fetch_one(self.ent.pool())
        .await
        .unwrap()
    }

    async fn compter(&self, sql: &str) -> i64 {
        sqlx::query_scalar(sql)
            .bind(self.etab)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    /// Ajoute un second engagement (employeur de droit privé : l'OGEC) à un agent.
    async fn engagement_ogec(&self, agent: Uuid) {
        sqlx::query(
            "INSERT INTO ent_agent_engagement (id, etablissement_id, agent_id, du, nature, employeur_type)
             VALUES ($1,$2,$3, current_date - 400, 'salarie_organisme', 'organisme_gestion')",
        )
        .bind(Uuid::new_v4())
        .bind(self.etab)
        .bind(agent)
        .execute(self.ent.pool())
        .await
        .expect("engagement OGEC");
    }
}

macro_rules! cadre {
    () => {
        match Cadre::nouveau().await {
            Some(c) => c,
            None => {
                eprintln!("skip : ENT_DATABASE_URL");
                return;
            }
        }
    };
}

fn est_refus(e: &EntDbError, nature: crate::service::RefusMetier) -> bool {
    matches!(e, EntDbError::Refus(n, _) if *n == nature)
}

// ═══════════════════════════════════════════════ LOT A0 ═══════════════════════════════════════════════

/// AR-01 / NJ-18 : le motif d'une absence de personnel n'est lisible ni de tout le personnel ni du
/// métier : accueil, comptabilité, professeur, CPE (qui organise), administrateur technique.
#[tokio::test]
async fn a0_ar01_le_motif_n_est_lisible_que_du_cercle_habilite() {
    let x = cadre!();
    // Ancienne ligne, saisie avant le lot (route historique oubliée) : motif médical en clair.
    let abs = Uuid::new_v4();
    let (prof_user, jour) = (x.prof, x.jour(2));
    sqlx::query(
        "INSERT INTO ent_absence_rh (id, etablissement_id, user_id, debut, fin, motif, statut, created_by_user_id)
         VALUES ($1,$2,$3,$4,$4,'chimiothérapie, cancer du sein','validee',$3)",
    )
    .bind(abs)
    .bind(x.etab)
    .bind(prof_user)
    .bind(jour)
    .execute(x.ent.pool())
    .await
    .unwrap();
    let lire = |user: Uuid, role: &'static str| {
        let ent = &x.ent;
        let etab = x.etab;
        async move {
            ent.list_absences_rh_for_actor(etab, user, role, jour, jour)
                .await
        }
    };
    // Accueil, comptabilité, professeur tiers, administrateur technique : aucune ligne d'autrui.
    for (u, r) in [
        (x.accueil, "accueil"),
        (x.compta, "compta"),
        (x.prof2, "prof"),
        (x.admin, "admin"),
    ] {
        let l = lire(u, r).await.unwrap();
        assert!(
            l.iter().all(|a| a.id != abs),
            "{r} voit l'absence d'un collègue"
        );
        assert!(!format!("{l:?}").contains("chimio"), "{r} lit le motif");
    }
    // Le CPE organise : il voit l'absence, jamais le motif.
    let l = lire(x.cpe, "cpe").await.unwrap();
    let a = l
        .iter()
        .find(|a| a.id == abs)
        .expect("le CPE voit l'absence");
    assert_eq!(a.motif, "Absence");
    // La direction et le secrétariat (lire_nature) lisent ; l'agent lit la sienne.
    for (u, r) in [
        (x.dir, "direction"),
        (x.sec, "administratif"),
        (x.prof, "prof"),
    ] {
        let l = lire(u, r).await.unwrap();
        let a = l
            .iter()
            .find(|a| a.id == abs)
            .unwrap_or_else(|| panic!("{r} doit voir l'absence"));
        assert!(
            a.motif.contains("cancer"),
            "{r} doit lire le motif : {}",
            a.motif
        );
    }
    // La table est classée avec la marque santé, hors du domaine « assiduité » des élèves (AR-03).
    let c = crate::registre_donnees::REGISTRE
        .iter()
        .find(|c| c.nom == "ent_absence_rh")
        .unwrap();
    assert!(c.sante && c.domaine == "personnels_absences");
}

/// AR-01 par la route historique : une absence déclarée aujourd'hui n'expose jamais son motif libre
/// (la projection porte « Absence ») ; AR-03.
#[tokio::test]
async fn a0_ar01_la_projection_historique_ne_porte_jamais_le_motif() {
    let x = cadre!();
    let a = x
        .ent
        .declarer_absence_rh_for_actor(
            "prof",
            DeclarerAbsenceRh {
                etablissement_id: x.etab,
                user_id: x.prof,
                debut: x.jour(1),
                fin: x.jour(1),
                motif: "dépression, arrêt maladie".into(),
                actor_user_id: x.prof,
            },
        )
        .await
        .unwrap();
    assert_eq!(a.motif, "Absence");
    let en_base: String = sqlx::query_scalar("SELECT motif FROM ent_absence_rh WHERE id = $1")
        .bind(a.id)
        .fetch_one(x.ent.pool())
        .await
        .unwrap();
    assert_eq!(
        en_base, "Absence",
        "la nature ne sort jamais de ent_absp_absence"
    );
    // L'ancien motif libre est en zone de qualification, jamais ailleurs.
    let zone: Option<String> =
        sqlx::query_scalar("SELECT zone_qualification FROM ent_absp_absence WHERE id = $1")
            .bind(a.id)
            .fetch_one(x.ent.pool())
            .await
            .unwrap();
    assert_eq!(zone.as_deref(), Some("dépression, arrêt maladie"));
}

/// AR-06 / NJ-19 : personne ne valide ni ne refuse sa propre absence — au service, et en base.
#[tokio::test]
async fn a0_ar06_auto_validation_refusee_au_service_et_en_base() {
    let x = cadre!();
    let a = x
        .ent
        .declarer_absence_rh_for_actor(
            "direction",
            DeclarerAbsenceRh {
                etablissement_id: x.etab,
                user_id: x.dir,
                debut: x.jour(1),
                fin: x.jour(1),
                motif: "rdv".into(),
                actor_user_id: x.dir,
            },
        )
        .await
        .unwrap();
    for e in [
        x.ent
            .valider_absence_rh_for_actor(x.etab, "direction", a.id, x.dir)
            .await
            .unwrap_err(),
        x.ent
            .refuser_absence_rh_for_actor(x.etab, "direction", a.id, x.dir)
            .await
            .unwrap_err(),
    ] {
        assert!(
            est_refus(&e, crate::service::RefusMetier::Interdit),
            "{e:?}"
        );
    }
    // Même sur le domaine : le détenteur de `valider` qui est l'absent est refusé.
    let e = x
        .ent
        .valider_absence_for_actor(x.etab, x.dir, "direction", a.id, &DecisionBody::default())
        .await
        .unwrap_err();
    assert!(
        est_refus(&e, crate::service::RefusMetier::Interdit),
        "{e:?}"
    );
    // En base : ni la table historique, ni la décision, ni le circuit ne l'acceptent.
    let r = sqlx::query("UPDATE ent_absence_rh SET valide_par_user_id = user_id WHERE id = $1")
        .bind(a.id)
        .execute(x.ent.pool())
        .await;
    assert!(r.is_err(), "ent_absence_rh : valideur = absent accepté");
    let agent = x.agent(x.dir).await;
    let r = sqlx::query(
        "INSERT INTO ent_absp_decision (id, etablissement_id, absence_id, type, auteur_agent_id, auteur_user_id)
         VALUES ($1,$2,$3,'validation',$4,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(x.etab)
    .bind(a.id)
    .bind(agent)
    .bind(x.dir)
    .execute(x.ent.pool())
    .await;
    assert!(r.is_err(), "ent_absp_decision : auto-validation acceptée");
    let r = sqlx::query(
        "INSERT INTO ent_absp_circuit (id, etablissement_id, absence_id, role, agent_id, source)
         VALUES ($1,$2,$3,'valideur',$4,'regle')",
    )
    .bind(Uuid::new_v4())
    .bind(x.etab)
    .bind(a.id)
    .bind(agent)
    .execute(x.ent.pool())
    .await;
    assert!(r.is_err(), "ent_absp_circuit : valideur = absent accepté");
}

/// AR-13 : la validation n'écrase plus l'auteur de la déclaration ; valideur et date en colonnes.
#[tokio::test]
async fn a0_ar13_declarant_et_valideur_sont_conserves() {
    let x = cadre!();
    let a = x
        .ent
        .declarer_absence_rh_for_actor(
            "prof",
            DeclarerAbsenceRh {
                etablissement_id: x.etab,
                user_id: x.prof,
                debut: x.jour(3),
                fin: x.jour(3),
                motif: "formation".into(),
                actor_user_id: x.prof,
            },
        )
        .await
        .unwrap();
    let v = x
        .ent
        .valider_absence_rh_for_actor(x.etab, "direction", a.id, x.dir)
        .await
        .unwrap();
    assert_eq!(v.statut, "validee");
    assert_eq!(
        v.created_by_user_id, x.prof,
        "l'auteur de la déclaration est conservé"
    );
    assert_eq!(v.valide_par_user_id, Some(x.dir));
    assert!(v.valide_le.is_some());
    let ligne: (Uuid, Option<Uuid>) = sqlx::query_as(
        "SELECT created_by_user_id, valide_par_user_id FROM ent_absence_rh WHERE id = $1",
    )
    .bind(a.id)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert_eq!(ligne, (x.prof, Some(x.dir)));
}

/// AR-16 : refus et annulation possibles ; une absence annulée ou seulement signalée par un tiers ne
/// bloque plus la personne comme remplaçante ; une absence confirmée la bloque.
#[tokio::test]
async fn a0_ar16_refus_annulation_et_remplacante_liberee() {
    let x = cadre!();
    let cr = x.creneau(&x.classe, x.prof, 1, (14, 0), (15, 0)).await; // l'absent
    x.creneau(&x.classe_bis().await, x.prof2, 1, (16, 0), (17, 0))
        .await; // compétent, libre à 14 h
    x.creneau(&x.classe_bis().await, x.prof3, 1, (16, 0), (17, 0))
        .await;
    let candidats = |lot: &'static str| {
        let (ent, d, jour) = (&x.ent, x.d(), x.jour(1));
        async move {
            let l = ent
                .edt_candidats_remplacants(&d, cr, jour, true)
                .await
                .unwrap();
            let ids: Vec<Uuid> = l.iter().map(|c| c.user_id).collect();
            eprintln!("{lot} : {} candidat(s)", ids.len());
            ids
        }
    };
    assert!(candidats("départ").await.contains(&x.prof2));
    // prof2 déclare lui-même une absence : confirmée, elle le bloque.
    let a2 = x
        .ent
        .declarer_absence_rh_for_actor(
            "prof",
            DeclarerAbsenceRh {
                etablissement_id: x.etab,
                user_id: x.prof2,
                debut: x.jour(1),
                fin: x.jour(1),
                motif: "x".into(),
                actor_user_id: x.prof2,
            },
        )
        .await
        .unwrap();
    assert!(!candidats("prof2 absent").await.contains(&x.prof2));
    // …puis l'annule (saisie par erreur, avant son début) : il redevient candidat. Rien n'est supprimé.
    let ann = x
        .ent
        .annuler_absence_rh_for_actor(x.etab, "prof", a2.id, x.prof2)
        .await
        .unwrap();
    assert_eq!(ann.statut, "annulee");
    assert!(ann.annule_le.is_some());
    assert!(candidats("prof2 annulée").await.contains(&x.prof2));
    assert_eq!(x.compter("SELECT count(*) FROM ent_absence_rh WHERE etablissement_id = $1 AND statut = 'annulee'").await, 1);
    // prof3 : absence saisie par la direction, non confirmée — elle ne bloque pas.
    let a3 = x
        .ent
        .declarer_absence_rh_for_actor(
            "direction",
            DeclarerAbsenceRh {
                etablissement_id: x.etab,
                user_id: x.prof3,
                debut: x.jour(1),
                fin: x.jour(1),
                motif: "x".into(),
                actor_user_id: x.dir,
            },
        )
        .await
        .unwrap();
    assert!(!a3.confirmee);
    assert!(
        candidats("prof3 signalée").await.contains(&x.prof3),
        "une absence non confirmée ne bloque pas"
    );
    // Un refus est possible tant que la nature le permet ; il est tracé.
    let a4 = x
        .ent
        .refuser_absence_rh_for_actor(x.etab, "direction", a3.id, x.dir)
        .await;
    assert!(a4.is_err(), "non confirmée : confirmer d'abord");
    x.ent
        .confirmer_absence_for_actor(x.etab, x.dir, "direction", a3.id, &Default::default(), true)
        .await
        .unwrap();
    assert!(
        !candidats("prof3 confirmée").await.contains(&x.prof3),
        "confirmée : elle bloque"
    );
    let refus = x
        .ent
        .refuser_absence_rh_for_actor(x.etab, "direction", a3.id, x.dir)
        .await
        .unwrap();
    assert_eq!(refus.statut, "refusee");
    assert_eq!(refus.refuse_par_user_id, Some(x.dir));
    // Un agent ne peut pas annuler l'absence d'un collègue (404/403, pas de fuite).
    let e = x
        .ent
        .annuler_absence_rh_for_actor(x.etab, "prof", a3.id, x.prof2)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
}

/// AR-08 / AR-14 / AR-18 : la route historique `POST /edt/exceptions` passe par les commandes contrôlées :
/// un parent, un absent, un occupé ne peuvent plus être désignés ; la délégation de l'ancien remplaçant
/// est révoquée ; le lien absence → séance n'est plus perdu.
#[tokio::test]
async fn a0_ar08_14_18_la_route_historique_controle_le_remplacant() {
    let x = cadre!();
    let parent = compte(&x.ent, x.etab, "parent").await;
    let cr = x.creneau(&x.classe, x.prof, 2, (10, 0), (11, 0)).await;
    x.creneau(&x.classe_bis().await, x.prof2, 2, (17, 0), (18, 0))
        .await;
    x.creneau(&x.classe_bis().await, x.prof3, 2, (17, 0), (18, 0))
        .await;
    let occupe = compte(&x.ent, x.etab, "prof").await;
    x.creneau(&x.classe_bis().await, occupe, 2, (10, 0), (11, 0))
        .await;
    let pose = |remplacant: Uuid, motif: &str| PoserEdtException {
        etablissement_id: x.etab,
        creneau_id: cr,
        jour: x.jour(2),
        kind: EdtExceptionKind::Remplacement,
        remplacant_user_id: Some(remplacant),
        nouvelle_salle: None,
        motif: motif.into(),
        actor_user_id: x.dir,
    };
    // Un parent, un enseignant occupé à la même heure : refus (conflit), rien n'est écrit.
    for faux in [parent, occupe] {
        let e = x
            .ent
            .poser_edt_exception_for_actor("direction", pose(faux, "x"))
            .await
            .unwrap_err();
        assert!(
            matches!(e, EntDbError::Domain(DomainError::Conflit)),
            "{e:?}"
        );
    }
    assert_eq!(
        x.compter("SELECT count(*) FROM ent_edt_exception WHERE etablissement_id = $1")
            .await,
        0
    );
    // Un remplaçant valide : exception + délégation d'un jour ; le motif libre n'est pas conservé (AR-02).
    let ex = x
        .ent
        .poser_edt_exception_for_actor("direction", pose(x.prof2, "maladie de Mme X"))
        .await
        .unwrap();
    assert_eq!(ex.motif, "");
    let motif_en_base: String =
        sqlx::query_scalar("SELECT motif FROM ent_edt_exception WHERE id = $1")
            .bind(ex.id)
            .fetch_one(x.ent.pool())
            .await
            .unwrap();
    assert_eq!(motif_en_base, "");
    // Lien absence → séance (organisé avec une absence validée), puis changement de remplaçant par la route
    // historique : le lien survit (AR-14) et la délégation du premier est révoquée (AR-18).
    let abs = x
        .declarer(x.prof, "prof", Some("formation"), 2, 2)
        .await
        .absences[0]
        .id;
    x.valider(abs).await;
    x.ent
        .edt_organiser_remplacement(
            &x.d(),
            &CmdRemplacement {
                creneau_id: cr,
                jour: x.jour(2),
                remplacant_user_id: x.prof2,
                absence_rh_id: Some(abs),
                nouvelle_salle: None,
            },
        )
        .await
        .unwrap();
    x.ent
        .poser_edt_exception_for_actor("direction", pose(x.prof3, "changement"))
        .await
        .unwrap();
    let lien: Option<Uuid> = sqlx::query_scalar(
        "SELECT absence_rh_id FROM ent_edt_exception WHERE creneau_id = $1 AND jour = $2",
    )
    .bind(cr)
    .bind(x.jour(2))
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert_eq!(
        lien,
        Some(abs),
        "AR-14 : le lien absence → séance est conservé"
    );
    let actives: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_delegation WHERE etablissement_id = $1 AND source = 'remplacement' AND statut = 'accordee'",
    )
    .bind(x.etab)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert_eq!(
        actives, 1,
        "AR-18 : une seule délégation active (celle de prof3)"
    );
    // L'état précédent est gardé, pas écrasé (R-AP8).
    assert!(x.compter("SELECT count(*) FROM ent_edt_exception_levee WHERE etablissement_id = $1 AND cause = 'remplacee'").await >= 1);
    // Un professeur n'a pas le droit de poser (403).
    let mut p = pose(x.prof2, "");
    p.actor_user_id = x.prof;
    assert!(matches!(
        x.ent
            .poser_edt_exception_for_actor("prof", p)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
}

/// AR-09 : deux remplacements simultanés du même agent à la même heure — un seul passe.
#[tokio::test]
async fn a0_ar09_deux_remplacements_simultanes_un_seul_passe() {
    let x = cadre!();
    let cl2 = x.classe_bis().await;
    let c1 = x.creneau(&x.classe, x.prof, 3, (9, 0), (10, 0)).await;
    let c2 = x.creneau(&cl2, x.prof3, 3, (9, 0), (10, 0)).await;
    x.creneau(&x.classe_bis().await, x.prof2, 3, (15, 0), (16, 0))
        .await;
    let (jour, etab, dir, remplacant) = (x.jour(3), x.etab, x.dir, x.prof2);
    let mut taches = Vec::new();
    for creneau in [c1, c2] {
        // Une connexion distincte par tâche : de vraies transactions concurrentes.
        let ent = connecter().await.unwrap();
        taches.push(tokio::spawn(async move {
            let a = Acteur {
                etablissement_id: etab,
                user_id: dir,
                role: "direction",
            };
            ent.edt_organiser_remplacement(
                &a,
                &CmdRemplacement {
                    creneau_id: creneau,
                    jour,
                    remplacant_user_id: remplacant,
                    absence_rh_id: None,
                    nouvelle_salle: None,
                },
            )
            .await
        }));
    }
    let mut ok = 0;
    let mut conflit = 0;
    for t in taches {
        match t.await.unwrap() {
            Ok(_) => ok += 1,
            Err(EntDbError::Domain(DomainError::Conflit)) => conflit += 1,
            Err(e) => panic!("erreur inattendue : {e:?}"),
        }
    }
    assert_eq!(
        (ok, conflit),
        (1, 1),
        "le même remplaçant ne peut pas tenir deux séances simultanées"
    );
    assert_eq!(
        x.compter("SELECT count(*) FROM ent_edt_exception WHERE etablissement_id = $1 AND kind = 'remplacement'").await,
        1
    );
}

/// AR-11 : l'annulation d'une séance citant une absence exige une absence **validée**, de **l'agent en
/// saisie**, couvrant le jour.
#[tokio::test]
async fn a0_ar11_annuler_une_seance_exige_une_absence_validee_du_bon_agent() {
    let x = cadre!();
    let cr = x.creneau(&x.classe, x.prof, 4, (8, 0), (9, 0)).await;
    let jour = x.jour(4);
    let abs_non_validee = x
        .declarer(x.prof, "prof", Some("formation"), 4, 4)
        .await
        .absences[0]
        .id;
    let e = x
        .ent
        .edt_annuler_seance(&x.d(), cr, jour, Some(abs_non_validee))
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(DomainError::Conflit)),
        "non validée : {e:?}"
    );
    // Une absence validée… d'un autre enseignant : refus.
    let abs_autre = x
        .declarer(x.prof2, "prof", Some("formation"), 4, 4)
        .await
        .absences[0]
        .id;
    x.valider(abs_autre).await;
    let e = x
        .ent
        .edt_annuler_seance(&x.d(), cr, jour, Some(abs_autre))
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(DomainError::Conflit)),
        "autre agent : {e:?}"
    );
    // Validée, du bon agent, jour couvert : accepté.
    x.valider(abs_non_validee).await;
    let r = x
        .ent
        .edt_annuler_seance(&x.d(), cr, jour, Some(abs_non_validee))
        .await
        .unwrap();
    assert!(!r.deja_fait);
    // Une absence d'un autre établissement : « introuvable », jamais un oracle.
    let ailleurs = Uuid::new_v4();
    let e = x
        .ent
        .edt_annuler_seance(&x.d(), cr, x.jour(4) + Duration::days(7), Some(ailleurs))
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(DomainError::NotFound)),
        "{e:?}"
    );
}

/// AR-15 : lever une exception ne supprime plus rien — l'état levé est archivé avec qui et quand ; la
/// séance redevient normale ; une décision remplacée est aussi gardée.
#[tokio::test]
async fn a0_ar15_la_levee_est_conservee() {
    let x = cadre!();
    let cr = x.creneau(&x.classe, x.prof, 5, (14, 0), (15, 0)).await;
    x.creneau(&x.classe_bis().await, x.prof2, 5, (17, 0), (18, 0))
        .await;
    let jour = x.jour(5);
    x.ent
        .edt_organiser_remplacement(
            &x.d(),
            &CmdRemplacement {
                creneau_id: cr,
                jour,
                remplacant_user_id: x.prof2,
                absence_rh_id: None,
                nouvelle_salle: None,
            },
        )
        .await
        .unwrap();
    assert!(x.ent.edt_lever_exception(&x.d(), cr, jour).await.unwrap());
    assert_eq!(
        x.compter("SELECT count(*) FROM ent_edt_exception WHERE etablissement_id = $1")
            .await,
        0
    );
    let (kind, rempl, cause, par): (String, Option<Uuid>, String, Uuid) = sqlx::query_as(
        "SELECT kind, remplacant_user_id, cause, levee_par_user_id FROM ent_edt_exception_levee WHERE etablissement_id = $1",
    )
    .bind(x.etab)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert_eq!(
        (kind.as_str(), rempl, cause.as_str(), par),
        ("remplacement", Some(x.prof2), "levee", x.dir)
    );
    // L'archive est en ajout seul.
    let r = sqlx::query("DELETE FROM ent_edt_exception_levee WHERE etablissement_id = $1")
        .bind(x.etab)
        .execute(x.ent.pool())
        .await;
    assert!(r.is_err(), "archive de levée supprimable");
    // Déplacement levé : archivé aussi.
    let dep = x
        .ent
        .edt_deplacer_seance(
            &x.d(),
            &crate::edt_remplacement::CmdDeplacement {
                creneau_id: cr,
                jour: x.jour(5),
                nouveau_jour: x.jour(5) + Duration::days(7),
                heure_debut: t(9, 0),
                heure_fin: t(10, 0),
                salle: None,
            },
        )
        .await;
    if dep.is_ok() {
        assert!(x
            .ent
            .edt_lever_exception(&x.d(), cr, x.jour(5))
            .await
            .unwrap());
        assert_eq!(
            x.compter("SELECT count(*) FROM ent_edt_deplacement_leve WHERE etablissement_id = $1")
                .await,
            1
        );
        assert_eq!(
            x.compter("SELECT count(*) FROM ent_edt_deplacement WHERE etablissement_id = $1")
                .await,
            0
        );
    }
}

/// AR-02 : qui ne fait que l'appel apprend l'exception d'emploi du temps, jamais son motif libre.
#[tokio::test]
async fn a0_ar02_le_motif_libre_des_exceptions_n_est_pas_lisible_des_professeurs() {
    let x = cadre!();
    let cr = x.creneau(&x.classe, x.prof, 1, (8, 0), (9, 0)).await;
    sqlx::query(
        "INSERT INTO ent_edt_exception (id, etablissement_id, creneau_id, jour, kind, motif, created_by_user_id, nature)
         VALUES ($1,$2,$3,$4,'annulation','Mme X est en arrêt maladie (cancer)',$5,'annulation')",
    )
    .bind(Uuid::new_v4())
    .bind(x.etab)
    .bind(cr)
    .bind(x.jour(1))
    .bind(x.dir)
    .execute(x.ent.pool())
    .await
    .unwrap();
    let prof = x
        .ent
        .list_edt_exceptions_jour(x.etab, "prof", x.jour(1))
        .await
        .unwrap();
    assert_eq!(prof.len(), 1, "le professeur apprend l'annulation");
    assert_eq!(prof[0].motif, "", "…jamais son motif libre");
    let dir = x
        .ent
        .list_edt_exceptions_jour(x.etab, "direction", x.jour(1))
        .await
        .unwrap();
    assert!(
        dir[0].motif.contains("cancer"),
        "la direction gère : elle lit"
    );
}

/// AR-04 : les familles ne voient jamais l'e-mail d'un personnel ; à défaut de nom affiché :
/// « Un enseignant de l'établissement ».
#[tokio::test]
async fn a0_ar04_aucun_e_mail_de_personnel_vers_les_familles() {
    let x = cadre!();
    let cr = x.creneau(&x.classe, x.prof, 2, (14, 0), (15, 0)).await;
    x.creneau(&x.classe_bis().await, x.prof2, 2, (17, 0), (18, 0))
        .await;
    sqlx::query("UPDATE ent_user SET display_name = NULL WHERE id = $1")
        .bind(x.prof2)
        .execute(x.ent.pool())
        .await
        .unwrap();
    x.ent
        .edt_organiser_remplacement(
            &x.d(),
            &CmdRemplacement {
                creneau_id: cr,
                jour: x.jour(2),
                remplacant_user_id: x.prof2,
                absence_rh_id: None,
                nouvelle_salle: None,
            },
        )
        .await
        .unwrap();
    // Vue de la semaine (lue aussi par les familles) : nom public de repli.
    let s = x
        .ent
        .edt_semaine(
            &x.d(),
            crate::edt_admin::CibleEdt::Classe(x.classe_id),
            x.lundi,
        )
        .await
        .unwrap();
    let json = serde_json::to_string(&s).unwrap();
    assert!(
        !json.contains("@ent.local"),
        "e-mail dans la vue des familles : {json}"
    );
    assert!(json.contains("Un enseignant de l'établissement"), "{json}");
    // Le nom public tient aussi pour la liste interne des noms « publics ».
    let n = x.ent.noms_publics(x.etab, &[x.prof2]).await.unwrap();
    assert_eq!(
        n.get(&x.prof2).map(String::as_str),
        Some("Un enseignant de l'établissement")
    );
}

/// AR-17 : déclaration, validation, refus, annulation sont journalisés (ajout seul), sans motif ; les
/// événements ne portent ni motif ni nature (R-AP2).
#[tokio::test]
async fn a0_ar17_journal_et_evenements_sans_motif() {
    let x = cadre!();
    let a = x
        .ent
        .declarer_absence_rh_for_actor(
            "prof",
            DeclarerAbsenceRh {
                etablissement_id: x.etab,
                user_id: x.prof,
                debut: x.jour(1),
                fin: x.jour(1),
                motif: "cancer".into(),
                actor_user_id: x.prof,
            },
        )
        .await
        .unwrap();
    x.ent
        .valider_absence_rh_for_actor(x.etab, "direction", a.id, x.dir)
        .await
        .unwrap();
    x.ent
        .annuler_absence_rh_for_actor(x.etab, "direction", a.id, x.dir)
        .await
        .unwrap();
    let actions: Vec<String> =
        sqlx::query_scalar("SELECT action FROM ent_absp_journal WHERE etablissement_id = $1 AND objet_id = $2 ORDER BY le, id")
            .bind(x.etab)
            .bind(a.id)
            .fetch_all(x.ent.pool())
            .await
            .unwrap();
    for attendu in ["creee", "validation", "annulation"] {
        assert!(
            actions.iter().any(|v| v == attendu),
            "journal sans « {attendu} » : {actions:?}"
        );
    }
    let r = sqlx::query("UPDATE ent_absp_journal SET action = 'x' WHERE etablissement_id = $1")
        .bind(x.etab)
        .execute(x.ent.pool())
        .await;
    assert!(r.is_err(), "le journal doit être en ajout seul");
    let charges: Vec<String> = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND (type LIKE 'absence_personnel.%' OR type LIKE 'rh.%')",
    )
    .bind(x.etab)
    .fetch_all(x.ent.pool())
    .await
    .unwrap();
    assert!(!charges.is_empty());
    for c in &charges {
        assert!(
            !c.contains("cancer") && !c.contains("motif") && !c.contains("nature"),
            "charge d'événement : {c}"
        );
    }
}

// ═══════════════════════════════════════════════ LOT A1 ═══════════════════════════════════════════════

/// Circuit figé (§9.4) : le valideur est le chef d'établissement, **jamais l'absent** ; une règle de
/// l'établissement s'applique aux absences suivantes sans toucher aux anciennes.
#[tokio::test]
async fn a1_le_circuit_est_fige_et_ne_designe_jamais_l_absent() {
    let x = cadre!();
    let d = x.declarer(x.prof, "prof", Some("formation"), 1, 1).await;
    let a = &d.absences[0];
    let dir_agent = x.agent(x.dir).await;
    assert_eq!(a.etat, "declaree");
    let valideurs: Vec<Uuid> = a
        .circuit
        .iter()
        .filter(|c| c.role == "valideur")
        .map(|c| c.agent_id)
        .collect();
    assert_eq!(
        valideurs,
        vec![dir_agent],
        "à défaut de règle : le chef d'établissement"
    );
    assert!(a.circuit.iter().all(|c| c.agent_id != a.agent_id));
    // Le chef d'établissement absent : il n'est pas son propre valideur.
    let da = x
        .declarer(x.dir, "direction", Some("formation"), 2, 2)
        .await;
    assert!(da.absences[0]
        .circuit
        .iter()
        .all(|c| c.role != "valideur" || c.agent_id != dir_agent));
    // Règle de l'établissement (paramétrage réservé) : le secrétariat valide pour l'État.
    let sec_agent = x.agent(x.sec).await;
    let r = x
        .ent
        .creer_regle_circuit_for_actor(
            x.etab,
            x.admin,
            "admin",
            &CreerRegleCircuitBody {
                employeur: "etat".into(),
                famille: None,
                role: "valideur".into(),
                agent_id: sec_agent,
                rang: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(r.role, "valideur");
    let apres = x.declarer(x.prof2, "prof", Some("formation"), 3, 3).await;
    assert!(apres.absences[0]
        .circuit
        .iter()
        .any(|c| c.agent_id == sec_agent && c.source == "regle"));
    // L'ancienne absence n'a pas changé (copie figée).
    let ancien = x
        .ent
        .absence_for_actor(x.etab, x.dir, "direction", a.id)
        .await
        .unwrap();
    assert!(!ancien.circuit.iter().any(|c| c.agent_id == sec_agent));
    // Le paramétrage n'appartient ni au professeur ni à l'accueil.
    for (u, r) in [
        (x.prof, "prof"),
        (x.accueil, "accueil"),
        (x.sec, "administratif"),
    ] {
        let e = x
            .ent
            .creer_regle_circuit_for_actor(
                x.etab,
                u,
                r,
                &CreerRegleCircuitBody {
                    employeur: "etat".into(),
                    famille: None,
                    role: "valideur".into(),
                    agent_id: sec_agent,
                    rang: None,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{r} : {e:?}");
    }
}

/// R-AP4 + §23.4 point 1 : un agent à deux engagements (État et OGEC) a deux absences liées pour une
/// seule déclaration ; la comptabilité ne voit que la partie OGEC, le secrétariat que celle de l'État.
#[tokio::test]
async fn a1_double_engagement_deux_absences_liees_et_cloisonnees() {
    let x = cadre!();
    let agent = x.agent(x.prof).await;
    x.engagement_ogec(agent).await;
    let d = x.declarer(x.prof, "prof", Some("convenance"), 1, 1).await;
    assert_eq!(d.absences.len(), 2, "une absence par engagement touché");
    let (a, b) = (&d.absences[0], &d.absences[1]);
    assert_eq!(
        a.absence_liee_id,
        Some(b.id),
        "les deux absences se référencent"
    );
    let employeurs: Vec<String> = sqlx::query_scalar(
        "SELECT employeur FROM ent_absp_absence WHERE etablissement_id = $1 AND agent_id = $2 ORDER BY employeur",
    )
    .bind(x.etab)
    .bind(agent)
    .fetch_all(x.ent.pool())
    .await
    .unwrap();
    assert_eq!(employeurs, vec!["etat", "ogec"]);
    let id_ogec: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_absp_absence WHERE etablissement_id = $1 AND employeur = 'ogec'",
    )
    .bind(x.etab)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    let id_etat = if id_ogec == a.id { b.id } else { a.id };
    // La comptabilité lit la partie OGEC, pas celle de l'État.
    assert!(x
        .ent
        .absence_for_actor(x.etab, x.compta, "compta", id_ogec)
        .await
        .is_ok());
    let e = x
        .ent
        .absence_for_actor(x.etab, x.compta, "compta", id_etat)
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(DomainError::NotFound)),
        "{e:?}"
    );
    // Le secrétariat l'inverse.
    assert!(x
        .ent
        .absence_for_actor(x.etab, x.sec, "administratif", id_etat)
        .await
        .is_ok());
    assert!(x
        .ent
        .absence_for_actor(x.etab, x.sec, "administratif", id_ogec)
        .await
        .is_err());
    // Une nature réservée à l'OGEC (congé payé) ne se déclare que pour l'engagement OGEC.
    let cp = x.declarer(x.prof, "prof", Some("conge_paye"), 3, 3).await;
    assert_eq!(cp.absences.len(), 1);
}

/// R-AP46 : une absence signalée (accueil, collègue) est organisable mais sans effet extérieur tant
/// qu'elle n'est pas confirmée ; elle se valide seulement après confirmation par l'agent ou rappel tracé.
#[tokio::test]
async fn a1_un_signalement_non_confirme_n_a_aucun_effet_exterieur() {
    let x = cadre!();
    let agent = x.agent(x.prof).await;
    let jour = x.jour(2);
    let s = x
        .ent
        .signaler_absence_for_actor(
            x.etab,
            x.accueil,
            "accueil",
            &SignalerAbsenceBody {
                agent_id: agent,
                debut: jour,
                fin: jour,
                heure_debut: None,
                heure_fin: None,
            },
        )
        .await
        .unwrap();
    let id = s.absences[0].id;
    assert_eq!(s.absences[0].etat, "signalee");
    assert!(s.absences[0].nature_code.is_none(), "sans nature");
    // L'accueil et le professeur n'ont pas ce droit… sauf l'accueil, qui signale.
    let e = x
        .ent
        .signaler_absence_for_actor(
            x.etab,
            x.prof2,
            "prof",
            &SignalerAbsenceBody {
                agent_id: agent,
                debut: jour,
                fin: jour,
                heure_debut: None,
                heure_fin: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden));
    // Visible de ceux qui organisent, marquée « non confirmée ».
    let org = x
        .ent
        .absences_organisation_for_actor(x.etab, x.cpe, "cpe", jour, jour)
        .await
        .unwrap();
    let ligne = org
        .iter()
        .find(|o| o.id == id)
        .expect("l'impact est visible pour organiser");
    assert!(!ligne.confirmee);
    // Aucun effet extérieur : ni libération de séance (port), ni validation.
    let cr = x.creneau(&x.classe, x.prof, 2, (9, 0), (10, 0)).await;
    assert!(x
        .ent
        .absences_validees_non_remplacees(x.etab, jour, jour)
        .await
        .unwrap()
        .is_empty());
    let e = x
        .ent
        .valider_absence_for_actor(x.etab, x.dir, "direction", id, &DecisionBody::default())
        .await
        .unwrap_err();
    assert!(
        matches!(
            e,
            EntDbError::Refus(crate::service::RefusMetier::Conflit, _)
        ),
        "{e:?}"
    );
    // Un tiers ne confirme pas à la place de l'agent ; le rappel est réservé à l'organisation.
    let e = x
        .ent
        .confirmer_absence_for_actor(x.etab, x.prof2, "prof", id, &Default::default(), false)
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden));
    let e = x
        .ent
        .confirmer_absence_for_actor(x.etab, x.accueil, "accueil", id, &Default::default(), true)
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Forbidden),
        "l'accueil ne confirme pas par rappel : {e:?}"
    );
    // L'agent confirme (et précise la nature) : l'absence entre dans le circuit.
    let c = x
        .ent
        .confirmer_absence_for_actor(
            x.etab,
            x.prof,
            "prof",
            id,
            &ConfirmerBody {
                nature_code: Some("formation".into()),
            },
            false,
        )
        .await
        .unwrap();
    assert_eq!(
        (c.etat.as_str(), c.confirmee_par.as_deref()),
        ("declaree", Some("agent"))
    );
    x.valider(id).await;
    assert_eq!(
        x.ent
            .absences_validees_non_remplacees(x.etab, jour, jour)
            .await
            .unwrap()[0]
            .creneau_id,
        cr
    );
}

/// R-AP2 / R-AP3 : la nature (sensible) ne sort jamais du cercle RH — vue organisation sans champ de
/// nature, événements sans nature, texte libre refusé, accueil / comptabilité / technique : rien.
#[tokio::test]
async fn a1_la_nature_ne_sort_jamais_du_cercle_rh() {
    let x = cadre!();
    let d = x.declarer(x.prof, "prof", Some("maladie"), 1, 2).await;
    let a = &d.absences[0];
    let id = a.id;
    // L'agent lit la sienne, nature comprise.
    assert_eq!(a.nature_code.as_deref(), Some("maladie"));
    // La vue organisation n'a AUCUN champ de nature (structure, pas valeur).
    let org = x
        .ent
        .absences_organisation_for_actor(x.etab, x.cpe, "cpe", x.jour(1), x.jour(2))
        .await
        .unwrap();
    assert_eq!(org.len(), 1);
    let json = serde_json::to_string(&org).unwrap();
    for interdit in [
        "nature",
        "maladie",
        "traitement",
        "texte",
        "decision",
        "circuit",
    ] {
        assert!(
            !json.contains(interdit),
            "« {interdit} » dans la vue organisation : {json}"
        );
    }
    // Ceux qui n'ont pas le droit : « introuvable » (pas d'oracle).
    for (u, r) in [
        (x.accueil, "accueil"),
        (x.compta, "compta"),
        (x.admin, "admin"),
        (x.prof2, "prof"),
        (x.cpe, "cpe"),
    ] {
        let e = x.ent.absence_for_actor(x.etab, u, r, id).await.unwrap_err();
        assert!(
            matches!(e, EntDbError::Domain(DomainError::NotFound)),
            "{r} : {e:?}"
        );
    }
    // …et la vue organisation leur est refusée (accueil, comptabilité, professeur, technique).
    for (u, r) in [
        (x.accueil, "accueil"),
        (x.compta, "compta"),
        (x.prof2, "prof"),
        (x.admin, "admin"),
    ] {
        let e = x
            .ent
            .absences_organisation_for_actor(x.etab, u, r, x.jour(1), x.jour(2))
            .await
            .unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{r} : {e:?}");
    }
    // La direction et le secrétariat (État) lisent la nature.
    assert_eq!(
        x.ent
            .absence_for_actor(x.etab, x.dir, "direction", id)
            .await
            .unwrap()
            .nature_code
            .as_deref(),
        Some("maladie")
    );
    assert_eq!(
        x.ent
            .absence_for_actor(x.etab, x.sec, "administratif", id)
            .await
            .unwrap()
            .nature_code
            .as_deref(),
        Some("maladie")
    );
    // Texte libre : refusé pour une nature sensible, et un mot de santé est bloqué pour toute nature (R-AP41).
    for (code, texte) in [
        ("maladie", "grippe"),
        ("formation", "rendez-vous chez le médecin"),
        ("formation", &"a".repeat(201)[..]),
    ] {
        let e = x
            .ent
            .declarer_absence_for_actor(
                x.etab,
                x.prof2,
                "prof",
                &DeclarerAbsenceBody {
                    nature_code: Some(code.into()),
                    debut: x.jour(3),
                    fin: x.jour(3),
                    texte_libre: Some(texte.into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
        assert!(
            matches!(
                e,
                EntDbError::Refus(crate::service::RefusMetier::Invalide, _)
            ),
            "{code} / {texte} : {e:?}"
        );
    }
    let ok = x
        .ent
        .declarer_absence_for_actor(
            x.etab,
            x.prof2,
            "prof",
            &DeclarerAbsenceBody {
                nature_code: Some("formation".into()),
                debut: x.jour(3),
                fin: x.jour(3),
                texte_libre: Some("Réunion de bassin à Lyon".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        ok.absences[0].texte_libre.as_deref(),
        Some("Réunion de bassin à Lyon")
    );
    // La base refuse aussi un texte libre sur une nature sensible (déclencheur).
    let r = sqlx::query("UPDATE ent_absp_absence SET texte_libre = 'x' WHERE id = $1")
        .bind(id)
        .execute(x.ent.pool())
        .await;
    assert!(r.is_err(), "texte libre accepté sur une nature sensible");
    // Les événements ne portent ni la nature ni un libellé de nature.
    x.valider(id).await;
    let charges: Vec<String> = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1",
    )
    .bind(x.etab)
    .fetch_all(x.ent.pool())
    .await
    .unwrap();
    for c in charges.iter().filter(|c| c.contains(&id.to_string())) {
        for interdit in ["maladie", "Arrêt de travail", "nature_code", "nature_id"] {
            assert!(
                !c.contains(interdit),
                "« {interdit} » dans un événement : {c}"
            );
        }
    }
    // La projection historique ne porte pas la nature non plus.
    let m: String = sqlx::query_scalar("SELECT motif FROM ent_absence_rh WHERE id = $1")
        .bind(id)
        .fetch_one(x.ent.pool())
        .await
        .unwrap();
    assert_eq!(m, "Absence");
}

/// R-AP47 (circuit externe) et R-AP12 (natures non discrétionnaires) : pour un agent de l'État, la
/// maladie est « prise en compte » (l'employeur décide), le refus est impossible, le traitement est imposé ;
/// pour un salarié de l'OGEC la direction décide, le maintien de salaire est imposé.
#[tokio::test]
async fn a1_circuit_externe_prise_en_compte_et_natures_non_discretionnaires() {
    let x = cadre!();
    let d = x.declarer(x.prof, "prof", Some("maladie"), 1, 3).await;
    let id = d.absences[0].id;
    assert_eq!(d.absences[0].decision, "externe");
    // Refus impossible (R-AP12, R-AP47).
    let e = x
        .ent
        .refuser_absence_for_actor(x.etab, x.dir, "direction", id, &DecisionBody::default())
        .await
        .unwrap_err();
    assert!(
        matches!(
            e,
            EntDbError::Refus(crate::service::RefusMetier::Conflit, _)
        ),
        "{e:?}"
    );
    // Le valideur choisit un traitement : ignoré, la nature l'impose (transmission au rectorat).
    let v = x
        .ent
        .valider_absence_for_actor(
            x.etab,
            x.dir,
            "direction",
            id,
            &DecisionBody {
                traitement: Some("service_non_fait".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(v.etat, "prise_en_compte");
    assert_eq!(v.traitement.as_deref(), Some("transmission_rectorat"));
    // Décision de l'employeur, enregistrée par l'instructeur ; l'état suit.
    let dd = x
        .ent
        .enregistrer_decision_externe_for_actor(
            x.etab,
            x.sec,
            "administratif",
            id,
            &DecisionExterneBody {
                employeur: "rectorat".into(),
                decision: "accordee".into(),
                nature_code: None,
                le: x.jour(1),
            },
        )
        .await
        .unwrap();
    assert_eq!(dd.etat, "decision_employeur");
    // Un professeur n'enregistre pas la décision.
    let e = x
        .ent
        .enregistrer_decision_externe_for_actor(
            x.etab,
            x.prof2,
            "prof",
            id,
            &DecisionExterneBody {
                employeur: "rectorat".into(),
                decision: "refusee".into(),
                nature_code: None,
                le: x.jour(1),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden));
    // OGEC : décision interne, maintien de salaire imposé.
    let ogec = compte(&x.ent, x.etab, "prof").await;
    let ag = x.agent(ogec).await;
    sqlx::query("UPDATE ent_agent_engagement SET employeur_type = 'organisme_gestion' WHERE etablissement_id = $1 AND agent_id = $2")
        .bind(x.etab)
        .bind(ag)
        .execute(x.ent.pool())
        .await
        .unwrap();
    let o = x.declarer(ogec, "prof", Some("maladie"), 4, 4).await;
    assert_eq!(o.absences[0].decision, "interne");
    let ov = x.valider(o.absences[0].id).await;
    assert_eq!(
        (ov.etat.as_str(), ov.traitement.as_deref()),
        ("validee", Some("maintien_salaire"))
    );
    // Nature discrétionnaire : refus possible (convenance) ; traitement hors liste refusé.
    let c = x.declarer(x.prof2, "prof", Some("convenance"), 1, 1).await;
    let e = x
        .ent
        .valider_absence_for_actor(
            x.etab,
            x.dir,
            "direction",
            c.absences[0].id,
            &DecisionBody {
                traitement: Some("retenue_salaire".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(
        matches!(
            e,
            EntDbError::Refus(crate::service::RefusMetier::Invalide, _)
        ),
        "retenue : OGEC seulement : {e:?}"
    );
    let r = x
        .ent
        .refuser_absence_for_actor(
            x.etab,
            x.dir,
            "direction",
            c.absences[0].id,
            &DecisionBody::default(),
        )
        .await
        .unwrap();
    assert_eq!(r.etat, "refusee");
}

/// Les faits qui modifient une absence : requalifier, prolonger, écourter, annuler, rouvrir, traiter,
/// clore — chacun tracé (décision + journal), jamais une réécriture ; les règles d'état sont tenues.
#[tokio::test]
async fn a1_les_faits_modifient_sans_reecrire() {
    let x = cadre!();
    let d = x.declarer(x.prof, "prof", Some("inconnue"), 1, 1).await;
    let id = d.absences[0].id;
    // Requalification par l'agent avant décision ; le traitement se recalcule à la validation.
    let r = x
        .ent
        .requalifier_absence_for_actor(
            x.etab,
            x.prof,
            "prof",
            id,
            &RequalifierBody {
                nature_code: "convenance".into(),
                raison_code: None,
                note: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(r.nature_code.as_deref(), Some("convenance"));
    // Un autre professeur ne requalifie pas.
    assert!(matches!(
        x.ent
            .requalifier_absence_for_actor(
                x.etab,
                x.prof2,
                "prof",
                id,
                &RequalifierBody {
                    nature_code: "formation".into(),
                    raison_code: None,
                    note: None
                }
            )
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    // Prolonger : la fin recule, une plage s'étend, l'impact s'étend (la projection suit).
    let fin_avant = r.fin;
    let p = x
        .ent
        .prolonger_absence_for_actor(
            x.etab,
            x.prof,
            "prof",
            id,
            &ChangerFinBody {
                fin: x.jour(3),
                heure_fin: None,
            },
        )
        .await
        .unwrap();
    assert!(p.fin > fin_avant);
    let (de, a): (NaiveDate, NaiveDate) =
        sqlx::query_as("SELECT debut, fin FROM ent_absence_rh WHERE id = $1")
            .bind(id)
            .fetch_one(x.ent.pool())
            .await
            .unwrap();
    assert_eq!((de, a), (x.jour(1), x.jour(3)));
    assert_eq!(x.compter("SELECT count(*) FROM ent_absp_plage WHERE etablissement_id = $1 AND retiree_le IS NULL").await, 1);
    // Prolonger vers l'arrière est refusé ; écourter vers l'avant aussi.
    assert!(x
        .ent
        .prolonger_absence_for_actor(
            x.etab,
            x.prof,
            "prof",
            id,
            &ChangerFinBody {
                fin: x.jour(2),
                heure_fin: None
            }
        )
        .await
        .is_err());
    assert!(x
        .ent
        .ecourter_absence_for_actor(
            x.etab,
            x.prof,
            "prof",
            id,
            &ChangerFinBody {
                fin: x.jour(5),
                heure_fin: None
            }
        )
        .await
        .is_err());
    // Écourter (reprise anticipée) : jamais de suppression.
    let e = x
        .ent
        .ecourter_absence_for_actor(
            x.etab,
            x.cpe,
            "cpe",
            id,
            &ChangerFinBody {
                fin: x.jour(2),
                heure_fin: None,
            },
        )
        .await
        .unwrap();
    assert!(e.fin < p.fin);
    let a: NaiveDate = sqlx::query_scalar("SELECT fin FROM ent_absence_rh WHERE id = $1")
        .bind(id)
        .fetch_one(x.ent.pool())
        .await
        .unwrap();
    assert_eq!(a, x.jour(2));
    // Valider, traiter, clore : la clôture est interdite tant qu'un traitement est dû (R-AP13).
    let v = x
        .ent
        .valider_absence_for_actor(
            x.etab,
            x.dir,
            "direction",
            id,
            &DecisionBody {
                traitement: Some("transmission_rectorat".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(v.etat, "validee");
    let e = x
        .ent
        .clore_absence_for_actor(x.etab, x.dir, "direction", id, &DecisionBody::default())
        .await
        .unwrap_err();
    assert!(
        matches!(
            e,
            EntDbError::Refus(crate::service::RefusMetier::Conflit, _)
        ),
        "traitement dû : {e:?}"
    );
    let tr = x
        .ent
        .traiter_absence_for_actor(
            x.etab,
            x.sec,
            "administratif",
            id,
            &DecisionBody {
                voie_transmission: Some("rectorat".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(tr.etat, "traitee");
    assert_eq!(
        x.ent
            .clore_absence_for_actor(x.etab, x.dir, "direction", id, &DecisionBody::default())
            .await
            .unwrap()
            .etat,
        "close"
    );
    // Rouvrir (valideur seulement) : retour à « declaree », rien n'est effacé.
    let ro = x
        .ent
        .rouvrir_absence_for_actor(x.etab, x.dir, "direction", id, &DecisionBody::default())
        .await
        .unwrap();
    assert_eq!(ro.etat, "declaree");
    assert!(matches!(
        x.ent
            .rouvrir_absence_for_actor(x.etab, x.prof, "prof", id, &DecisionBody::default())
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    // Annuler : l'agent avant le début, ou le valideur ; l'annulation retire l'impact (projection).
    let an = x
        .ent
        .annuler_absence_for_actor(x.etab, x.dir, "direction", id, &DecisionBody::default())
        .await
        .unwrap();
    assert_eq!(an.etat, "annulee");
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT statut FROM ent_absence_rh WHERE id = $1")
            .bind(id)
            .fetch_one(x.ent.pool())
            .await
            .unwrap(),
        "annulee"
    );
    // On ne valide plus une absence annulée ; on ne la prolonge pas.
    assert!(x
        .ent
        .valider_absence_for_actor(x.etab, x.dir, "direction", id, &DecisionBody::default())
        .await
        .is_err());
    assert!(x
        .ent
        .prolonger_absence_for_actor(
            x.etab,
            x.prof,
            "prof",
            id,
            &ChangerFinBody {
                fin: x.jour(4),
                heure_fin: None
            }
        )
        .await
        .is_err());
    // Historique : une décision par fait, en ajout seul, et un journal complet pour l'agent.
    let types: Vec<String> = sqlx::query_scalar("SELECT type FROM ent_absp_decision WHERE etablissement_id = $1 AND absence_id = $2 ORDER BY le, id")
        .bind(x.etab)
        .bind(id)
        .fetch_all(x.ent.pool())
        .await
        .unwrap();
    for t in [
        "declaration",
        "requalification",
        "prolongation",
        "ecourtement",
        "validation",
        "traitement",
        "cloture",
        "reouverture",
        "annulation",
    ] {
        assert!(
            types.iter().any(|v| v == t),
            "décision « {t} » absente : {types:?}"
        );
    }
    assert!(
        sqlx::query("UPDATE ent_absp_decision SET note = 'x' WHERE absence_id = $1")
            .bind(id)
            .execute(x.ent.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM ent_absp_decision WHERE absence_id = $1")
            .bind(id)
            .execute(x.ent.pool())
            .await
            .is_err()
    );
    let h = x
        .ent
        .historique_absence_for_actor(x.etab, x.prof, "prof", id)
        .await
        .unwrap();
    assert!(h.len() >= 9, "{}", h.len());
    assert!(matches!(
        x.ent
            .historique_absence_for_actor(x.etab, x.prof2, "prof", id)
            .await
            .unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
}

/// Retard (R-AP14) : au plus 4 h, le même jour ; deux plages d'une absence ne se chevauchent pas ;
/// une absence partielle n'est comptée que sur ses heures.
#[tokio::test]
async fn a1_retard_plages_et_chevauchements() {
    let x = cadre!();
    let corps =
        |nature: &str, j1: i64, j2: i64, h: Option<((u32, u32), (u32, u32))>| DeclarerAbsenceBody {
            nature_code: Some(nature.into()),
            debut: x.jour(j1),
            fin: x.jour(j2),
            heure_debut: h.map(|v| t(v.0 .0, v.0 .1)),
            heure_fin: h.map(|v| t(v.1 .0, v.1 .1)),
            ..Default::default()
        };
    let decl = |u: Uuid, b: DeclarerAbsenceBody| {
        let ent = &x.ent;
        let etab = x.etab;
        async move { ent.declarer_absence_for_actor(etab, u, "prof", &b).await }
    };
    assert!(decl(x.prof, corps("retard", 1, 1, Some(((8, 0), (10, 0)))))
        .await
        .is_ok());
    assert!(
        decl(x.prof, corps("retard", 2, 2, Some(((8, 0), (14, 0)))))
            .await
            .is_err(),
        "plus de 4 h"
    );
    assert!(
        decl(x.prof, corps("retard", 2, 3, None)).await.is_err(),
        "retard : le même jour"
    );
    // Fin avant le début : refusé.
    assert!(decl(x.prof, corps("formation", 3, 2, None)).await.is_err());
    // Discontinue : deux plages disjointes passent, deux plages qui se chevauchent non.
    let plages = |a: &[(i64, i64)]| {
        a.iter()
            .map(|(d, f)| PlageBody {
                debut: x.jour(*d),
                fin: x.jour(*f),
                heure_debut: None,
                heure_fin: None,
            })
            .collect::<Vec<_>>()
    };
    let mut b = corps("formation", 1, 5, None);
    b.plages = plages(&[(1, 1), (3, 3)]);
    assert!(decl(x.prof2, b).await.is_ok());
    let mut b = corps("formation", 1, 5, None);
    b.plages = plages(&[(1, 3), (3, 4)]);
    assert!(decl(x.prof3, b).await.is_err(), "plages qui se chevauchent");
    // Chevauchement avec une absence existante : proposé, jamais bloqué (AD-06).
    let d2 = decl(x.prof, corps("formation", 1, 1, Some(((9, 0), (11, 0)))))
        .await
        .unwrap();
    assert_eq!(d2.chevauchements.len(), 1);
}

/// D-058 — port « absences validées non remplacées » (socles 10 et 19) : une séance dont l'enseignant est
/// absent (absence décidée) et qui n'a pas de remplaçant ; aucun motif, aucune nature, aucun nom ; un
/// remplaçant nommé, une absence annulée, ou seulement signalée / refusée la fait disparaître.
#[tokio::test]
async fn a1_port_absences_validees_non_remplacees() {
    let x = cadre!();
    let cl2 = x.classe_bis().await;
    let c1 = x.creneau(&x.classe, x.prof, 1, (8, 0), (9, 0)).await;
    let c2 = x.creneau(&x.classe, x.prof, 1, (14, 0), (15, 0)).await;
    x.creneau(&cl2, x.prof2, 1, (17, 0), (18, 0)).await;
    let jour = x.jour(1);
    let port = || async {
        x.ent
            .absences_validees_non_remplacees(x.etab, jour, jour)
            .await
            .unwrap()
    };
    assert!(port().await.is_empty(), "aucune absence : rien");
    // Déclarée mais pas décidée : rien (D-058 : pas sur simple soupçon).
    let a = x
        .declarer(x.prof, "prof", Some("maladie"), 1, 1)
        .await
        .absences[0]
        .id;
    assert!(port().await.is_empty());
    // Décidée (ici : prise en compte, nature externe) : les deux séances sont sans enseignant.
    x.valider(a).await;
    let l = port().await;
    assert_eq!(
        l.iter().map(|s| s.creneau_id).collect::<Vec<_>>(),
        vec![c1, c2]
    );
    assert!(l
        .iter()
        .all(|s| s.statut == "non_remplacee" && s.absence_id == a));
    // Aucun motif, aucune nature, aucun nom dans le port.
    let json = serde_json::to_string(&l).unwrap();
    for interdit in [
        "maladie",
        "nature",
        "motif",
        "traitement",
        "Nom prof",
        "agent_id",
        "@ent.local",
    ] {
        assert!(
            !json.contains(interdit),
            "« {interdit} » dans le port : {json}"
        );
    }
    // Un remplaçant nommé : la séance sort du port (le cours est rétabli ; la salle n'est plus libre).
    x.ent
        .edt_organiser_remplacement(
            &x.d(),
            &CmdRemplacement {
                creneau_id: c1,
                jour,
                remplacant_user_id: x.prof2,
                absence_rh_id: Some(a),
                nouvelle_salle: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        port()
            .await
            .iter()
            .map(|s| s.creneau_id)
            .collect::<Vec<_>>(),
        vec![c2]
    );
    // Séance annulée (le cours est supprimé) : elle reste dans le port, statut « annulee ».
    x.ent
        .edt_annuler_seance(&x.d(), c2, jour, Some(a))
        .await
        .unwrap();
    let l = port().await;
    assert_eq!((l.len(), l[0].statut.as_str()), (1, "annulee"));
    // Retirer le remplaçant : la séance revient.
    assert!(x.ent.edt_lever_exception(&x.d(), c1, jour).await.unwrap());
    assert_eq!(port().await.len(), 2);
    // L'absence annulée : plus rien.
    x.ent
        .annuler_absence_for_actor(x.etab, x.dir, "direction", a, &DecisionBody::default())
        .await
        .unwrap();
    assert!(port().await.is_empty());
    // Refusée : l'impact reste pour l'organisation, mais elle ne libère pas de salle (non décidée).
    let r = x
        .declarer(x.prof, "prof", Some("convenance"), 1, 1)
        .await
        .absences[0]
        .id;
    x.ent
        .refuser_absence_for_actor(x.etab, x.dir, "direction", r, &DecisionBody::default())
        .await
        .unwrap();
    assert!(port().await.is_empty());
    // Droits de lecture HTTP du port : edt.gerer, lire_organisation ou lire_nature ; pas l'accueil.
    let a2 = x
        .declarer(x.prof, "prof", Some("formation"), 1, 1)
        .await
        .absences[0]
        .id;
    x.valider(a2).await;
    assert!(x
        .ent
        .absences_validees_non_remplacees_for_actor(x.etab, x.cpe, "cpe", jour, jour)
        .await
        .is_ok());
    for (u, r) in [
        (x.accueil, "accueil"),
        (x.compta, "compta"),
        (x.prof3, "prof"),
    ] {
        let e = x
            .ent
            .absences_validees_non_remplacees_for_actor(x.etab, u, r, jour, jour)
            .await
            .unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{r} : {e:?}");
    }
}

/// Absence partielle : de 14 h à 15 h seulement — la séance de 8 h n'est pas touchée.
#[tokio::test]
async fn a1_une_absence_partielle_ne_touche_que_ses_heures() {
    let x = cadre!();
    let c_matin = x.creneau(&x.classe, x.prof, 2, (8, 0), (9, 0)).await;
    let c_aprem = x.creneau(&x.classe, x.prof, 2, (14, 0), (15, 0)).await;
    let jour = x.jour(2);
    let d = x
        .ent
        .declarer_absence_for_actor(
            x.etab,
            x.prof,
            "prof",
            &DeclarerAbsenceBody {
                nature_code: Some("formation".into()),
                debut: jour,
                fin: jour,
                heure_debut: Some(t(13, 30)),
                heure_fin: Some(t(15, 30)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    x.valider(d.absences[0].id).await;
    let l = x
        .ent
        .absences_validees_non_remplacees(x.etab, jour, jour)
        .await
        .unwrap();
    let ids: Vec<Uuid> = l.iter().map(|s| s.creneau_id).collect();
    assert_eq!(ids, vec![c_aprem]);
    assert!(!ids.contains(&c_matin));
}

/// Droits (R-AP11, AR-07) : valider n'est pas déduit d'un droit sur les élèves ; le professeur, l'accueil,
/// le CPE, la comptabilité, l'administrateur technique ne valident pas ; un agent sans droit ne déclare que
/// pour lui ; les natures proposées sont celles de l'employeur de l'agent.
#[tokio::test]
async fn a1_matrice_de_droits() {
    let x = cadre!();
    let a = x
        .declarer(x.prof, "prof", Some("formation"), 1, 1)
        .await
        .absences[0]
        .id;
    for (u, r) in [
        (x.prof2, "prof"),
        (x.accueil, "accueil"),
        (x.cpe, "cpe"),
        (x.compta, "compta"),
        (x.admin, "admin"),
        (x.sec, "administratif"),
    ] {
        let e = x
            .ent
            .valider_absence_for_actor(x.etab, u, r, a, &DecisionBody::default())
            .await
            .unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{r} valide : {e:?}");
    }
    // File « à valider » : la direction seule.
    assert_eq!(
        x.ent
            .file_a_valider_for_actor(x.etab, x.dir, "direction")
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(matches!(
        x.ent
            .file_a_valider_for_actor(x.etab, x.prof2, "prof")
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    // Déclarer pour un collègue : secrétariat, comptabilité, direction ; pas un professeur.
    let pour = |u: Uuid, r: &'static str| {
        let ent = &x.ent;
        let (etab, agent, jour) = (x.etab, x.prof3, x.jour(2));
        async move {
            let ag = agent_de(ent, etab, agent).await;
            ent.declarer_absence_for_actor(
                etab,
                u,
                r,
                &DeclarerAbsenceBody {
                    agent_id: Some(ag),
                    nature_code: Some("formation".into()),
                    debut: jour,
                    fin: jour,
                    ..Default::default()
                },
            )
            .await
        }
    };
    assert!(matches!(
        pour(x.prof2, "prof").await.unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(matches!(
        pour(x.accueil, "accueil").await.unwrap_err(),
        EntDbError::Forbidden
    ));
    let d = pour(x.sec, "administratif").await.unwrap();
    assert_eq!(
        d.absences[0].etat, "signalee",
        "saisie par un tiers : l'agent confirme"
    );
    // Natures proposées : jamais celles réservées à la direction ni posées par un autre module.
    let n: Vec<String> = x
        .ent
        .natures_for_actor(x.etab, x.prof, "prof")
        .await
        .unwrap()
        .into_iter()
        .map(|n| n.code)
        .collect();
    for interdit in ["absence_injustifiee", "autre_service", "conge_paye"] {
        assert!(
            !n.iter().any(|c| c == interdit),
            "{interdit} proposée à un agent de l'État : {n:?}"
        );
    }
    assert!(n.contains(&"maladie".to_string()) && n.contains(&"inconnue".to_string()));
    // Parent et élève : aucun accès au domaine.
    let parent = compte(&x.ent, x.etab, "parent").await;
    assert!(matches!(
        x.ent
            .mes_absences_for_actor(x.etab, parent, "parent")
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(matches!(
        x.ent
            .declarer_absence_for_actor(
                x.etab,
                parent,
                "parent",
                &DeclarerAbsenceBody {
                    debut: x.jour(1),
                    fin: x.jour(1),
                    ..Default::default()
                }
            )
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
}

/// Cloisonnement d'établissement (R-AP42, D-013) : une absence d'un établissement n'est jamais lisible,
/// ni décidable, d'un autre — même avec les mêmes droits.
#[tokio::test]
async fn a1_une_absence_n_est_jamais_lisible_d_un_autre_etablissement() {
    let x = cadre!();
    let a = x
        .declarer(x.prof, "prof", Some("maladie"), 1, 1)
        .await
        .absences[0]
        .id;
    let autre = Uuid::new_v4();
    let dir_b = compte(&x.ent, autre, "direction").await;
    for e in [
        x.ent
            .absence_for_actor(autre, dir_b, "direction", a)
            .await
            .unwrap_err(),
        x.ent
            .valider_absence_for_actor(autre, dir_b, "direction", a, &DecisionBody::default())
            .await
            .unwrap_err(),
        x.ent
            .annuler_absence_for_actor(autre, dir_b, "direction", a, &DecisionBody::default())
            .await
            .unwrap_err(),
    ] {
        assert!(
            matches!(e, EntDbError::Domain(DomainError::NotFound)),
            "{e:?}"
        );
    }
    assert!(x
        .ent
        .absences_organisation_for_actor(autre, dir_b, "direction", x.jour(1), x.jour(1))
        .await
        .unwrap()
        .is_empty());
    assert!(x
        .ent
        .absences_validees_non_remplacees(autre, x.jour(1), x.jour(1))
        .await
        .unwrap()
        .is_empty());
}

/// Arrêt (R-AP16) : un arrêt attendu porte une échéance de 48 h ; l'instructeur note sa réception.
#[tokio::test]
async fn a1_arret_echeance_et_reception() {
    let x = cadre!();
    let id = x
        .declarer(x.prof, "prof", Some("maladie"), 1, 2)
        .await
        .absences[0]
        .id;
    let (n, recu): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(recu_le) FROM ent_absp_arret WHERE etablissement_id = $1 AND absence_id = $2 AND echeance_le IS NOT NULL",
    )
    .bind(x.etab)
    .bind(id)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert_eq!((n, recu), (1, 0));
    // La comptabilité (circuit OGEC) n'instruit pas l'État ; le secrétariat oui.
    assert!(matches!(
        x.ent
            .noter_arret_recu_for_actor(x.etab, x.compta, "compta", id, &Default::default())
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    x.ent
        .noter_arret_recu_for_actor(x.etab, x.sec, "administratif", id, &Default::default())
        .await
        .unwrap();
    assert_eq!(
        x.compter("SELECT count(recu_le) FROM ent_absp_arret WHERE etablissement_id = $1")
            .await,
        1
    );
    // Une prolongation est un nouvel arrêt.
    x.ent
        .prolonger_absence_for_actor(
            x.etab,
            x.prof,
            "prof",
            id,
            &ChangerFinBody {
                fin: x.jour(4),
                heure_fin: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        x.compter("SELECT count(*) FROM ent_absp_arret WHERE etablissement_id = $1")
            .await,
        2
    );
    // Plus d'arrêt en attente après réception du second : un second appel est un conflit.
    x.ent
        .noter_arret_recu_for_actor(x.etab, x.sec, "administratif", id, &Default::default())
        .await
        .unwrap();
    assert!(x
        .ent
        .noter_arret_recu_for_actor(x.etab, x.sec, "administratif", id, &Default::default())
        .await
        .is_err());
}

/// Reprise de l'existant (migration 2302) : une ligne de `ent_absence_rh` devient une absence de nature
/// « à préciser », son motif en zone de qualification, jamais en clair dans la projection.
#[tokio::test]
async fn a1_reprise_de_l_existant_sans_motif_en_clair() {
    let x = cadre!();
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_absence_rh (id, etablissement_id, user_id, debut, fin, motif, statut, created_by_user_id)
         VALUES ($1,$2,$3,$4,$4,'opération du genou','signalee',$3)",
    )
    .bind(id)
    .bind(x.etab)
    .bind(x.prof)
    .bind(x.jour(1))
    .execute(x.ent.pool())
    .await
    .unwrap();
    // La migration est rejouable (E8) : le lanceur ne la rejoue plus une fois enregistrée, on rejoue donc
    // le fichier lui-même, comme le ferait une restauration ou un rejeu manuel.
    rejouer_2302(&x).await;
    let (etat, zone, motif): (String, Option<String>, String) = sqlx::query_as(
        "SELECT a.etat, a.zone_qualification, r.motif FROM ent_absp_absence a JOIN ent_absence_rh r ON r.id = a.id WHERE a.id = $1",
    )
    .bind(id)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert_eq!(etat, "declaree");
    assert_eq!(zone.as_deref(), Some("opération du genou"));
    assert_eq!(
        motif, "Absence",
        "l'ancien motif ne reste pas dans la projection"
    );
    // La reprise ne double pas (rejouée deux fois).
    rejouer_2302(&x).await;
    assert_eq!(
        x.compter("SELECT count(*) FROM ent_absp_absence WHERE etablissement_id = $1")
            .await,
        1
    );
}

async fn rejouer_2302(x: &Cadre) {
    sqlx::raw_sql(include_str!("../../../migrations/2302_absp_absence.sql"))
        .execute(x.ent.pool())
        .await
        .expect("rejeu de la migration 2302");
}
