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
             VALUES ($1,$2,$3, ent_aujourdhui() - 400, 'salarie_organisme', 'organisme_gestion')",
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

// ═══════════════════════════════════════════════ LOT A3 : IMPACT ═══════════════════════════════════════════════
// Critères de fin (§28.1) : impact juste sur CAP-05, 06, 25, 26, 27 et 46 ; tableau « ce matin » ; le 15
// n'écrit jamais dans les tables du 10 (il lit le port `edt_seances`).

impl Cadre {
    /// Une séance datée du socle 10 pour `creneau`, le jour ISO `iso` de la semaine du cadre (insérée comme le
    /// ferait la génération : le test ne dépend pas de la chaîne de reprise de l'EDT).
    async fn seance(&self, creneau: Uuid, prof: Uuid, iso: i64, d: (u32, u32), f: (u32, u32), etat: &str) -> Uuid {
        let annee: Uuid = sqlx::query_scalar(
            "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 ORDER BY date_debut DESC LIMIT 1",
        )
        .bind(self.etab)
        .fetch_one(self.ent.pool())
        .await
        .unwrap();
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_edt_seance (id, etablissement_id, annee_id, creneau_id, jour, classe_id, classe, prof_user_id,
                    matiere_libelle, heure_debut, heure_fin, etat, statut, origine)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'Mathématiques',$9,$10,$11,'prevue','plan')
             ON CONFLICT (etablissement_id, creneau_id, jour) DO UPDATE SET prof_user_id = $8, etat = $11, id = ent_edt_seance.id",
        )
        .bind(id)
        .bind(self.etab)
        .bind(annee)
        .bind(creneau)
        .bind(self.jour(iso))
        .bind(self.classe_id)
        .bind(&self.classe)
        .bind(prof)
        .bind(t(d.0, d.1))
        .bind(t(f.0, f.1))
        .bind(etat)
        .execute(self.ent.pool())
        .await
        .expect("séance");
        sqlx::query_scalar("SELECT id FROM ent_edt_seance WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3")
            .bind(self.etab)
            .bind(creneau)
            .bind(self.jour(iso))
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    /// Pose l'état d'une séance comme le feront les lots T3/T4 du socle 10 (`neutralisee`, `non_assuree`…) : le
    /// déclencheur historique dérive `etat` de `statut`, on l'écarte le temps de l'écriture.
    async fn forcer_etat_seance(&self, id: Uuid, etat: &str) {
        let mut tx = self.ent.pool_verif().begin().await.unwrap();
        sqlx::query("ALTER TABLE ent_edt_seance DISABLE TRIGGER ent_edt_seance_derive_tg")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE ent_edt_seance SET etat = $2 WHERE id = $1")
            .bind(id)
            .bind(etat)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE ent_edt_seance ENABLE TRIGGER ent_edt_seance_derive_tg")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }

    async fn impacts(&self, absence: Uuid) -> Vec<ImpactDto> {
        self.ent
            .impact_absence_for_actor(self.etab, self.dir, "direction", absence)
            .await
            .unwrap_or_else(|e| panic!("impact refusé : {e:?}"))
    }
}

fn heures(i: &[ImpactDto]) -> Vec<(u32, u32)> {
    use chrono::Timelike;
    let mut v: Vec<(u32, u32)> = i
        .iter()
        .filter_map(|x| x.seance.as_ref())
        .map(|s| (s.heure_debut.hour(), s.heure_debut.minute()))
        .collect();
    v.sort();
    v
}

/// CAP-05 : absence d'une demi-journée seulement : ne touche que les séances de l'après-midi. Le DTO ne porte
/// ni nature ni motif ; l'organisation et l'agent le lisent, un collègue non.
#[tokio::test]
async fn a3_cap05_absence_d_apres_midi_ne_touche_que_l_apres_midi() {
    let x = cadre!();
    for (iso_h, f) in [((9, 0), (10, 0)), ((14, 0), (15, 0)), ((16, 0), (17, 0))] {
        let c = x.creneau(&x.classe, x.prof, 1, iso_h, f).await;
        x.seance(c, x.prof, 1, iso_h, f, "prevue").await;
    }
    let d = x
        .ent
        .declarer_absence_for_actor(
            x.etab,
            x.prof,
            "prof",
            &DeclarerAbsenceBody {
                nature_code: Some("maladie".into()),
                debut: x.jour(1),
                fin: x.jour(1),
                heure_debut: Some(t(13, 30)),
                heure_fin: Some(t(17, 0)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let abs = d.absences[0].id;
    let i = x.impacts(abs).await;
    assert_eq!(heures(&i), vec![(14, 0), (16, 0)], "{i:?}");
    assert!(i.iter().all(|x| x.etat == "a_traiter" && !x.a_revoir));
    let json = serde_json::to_string(&i).unwrap();
    for interdit in ["maladie", "nature", "motif", "traitement"] {
        assert!(!json.contains(interdit), "« {interdit} » dans l'impact : {json}");
    }
    // Lecture : l'organisation (CPE), l'agent ; pas un collègue ni l'administrateur technique.
    assert!(x.ent.impact_absence_for_actor(x.etab, x.cpe, "cpe", abs).await.is_ok());
    assert!(x.ent.impact_absence_for_actor(x.etab, x.prof, "prof", abs).await.is_ok());
    for (u, r) in [(x.prof2, "prof"), (x.admin, "admin")] {
        let e = x.ent.impact_absence_for_actor(x.etab, u, r, abs).await.unwrap_err();
        assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{r} : {e:?}");
    }
}

/// CAP-06 : absence discontinue (un rendez-vous chaque lundi matin pendant un mois) : une absence, quatre
/// plages ; seules les séances de ces plages sont touchées — ni les autres jours, ni l'après-midi.
#[tokio::test]
async fn a3_cap06_absence_discontinue_touche_l_union_des_plages() {
    let x = cadre!();
    let lundi_matin = x.creneau(&x.classe, x.prof, 1, (9, 0), (10, 0)).await;
    let mardi = x.creneau(&x.classe, x.prof, 2, (9, 0), (10, 0)).await;
    let lundi_pm = x.creneau(&x.classe, x.prof, 1, (14, 0), (15, 0)).await;
    for semaine in 0..4i64 {
        // La séance du lundi matin de chaque semaine, celle du lundi après-midi et celle du mardi.
        x.seance(lundi_matin, x.prof, 1 + 7 * semaine, (9, 0), (10, 0), "prevue").await;
        x.seance(lundi_pm, x.prof, 1 + 7 * semaine, (14, 0), (15, 0), "prevue").await;
        x.seance(mardi, x.prof, 2 + 7 * semaine, (9, 0), (10, 0), "prevue").await;
    }
    let plages: Vec<PlageBody> = (0..4i64)
        .map(|s| PlageBody { debut: x.jour(1 + 7 * s), fin: x.jour(1 + 7 * s), heure_debut: Some(t(8, 0)), heure_fin: Some(t(12, 0)) })
        .collect();
    let d = x
        .ent
        .declarer_absence_for_actor(
            x.etab,
            x.prof,
            "prof",
            &DeclarerAbsenceBody {
                nature_code: Some("maladie".into()),
                debut: x.jour(1),
                fin: x.jour(22),
                plages,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(d.absences.len(), 1, "une seule absence, quatre plages");
    let i = x.impacts(d.absences[0].id).await;
    assert_eq!(i.len(), 4, "{i:?}");
    assert!(heures(&i).iter().all(|h| *h == (9, 0)));
    let jours: Vec<NaiveDate> = i.iter().map(|x| x.seance.as_ref().unwrap().jour).collect();
    assert!((0..4i64).all(|s| jours.contains(&x.jour(1 + 7 * s))), "{jours:?}");
    // Rien n'est écrit dans les tables du socle 10 : les séances sont inchangées.
    let touchees: i64 = x.compter("SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND etat <> 'prevue'").await;
    assert_eq!(touchees, 0, "le 15 n'écrit jamais dans le 10");
}

/// CAP-25 : absence d'un co-enseignant : la séance n'est pas touchée si l'autre intervenant en saisie est
/// présent (R-AP25) ; elle l'est si l'autre est absent lui aussi.
#[tokio::test]
async fn a3_cap25_co_enseignement_l_autre_intervenant_present_suffit() {
    let x = cadre!();
    let c = x.creneau(&x.classe, x.prof, 1, (9, 0), (10, 0)).await;
    let s = x.seance(c, x.prof, 1, (9, 0), (10, 0), "prevue").await;
    let co = x.agent(x.prof2).await;
    sqlx::query(
        "INSERT INTO ent_edt_seance_intervenant (seance_id, etablissement_id, agent_id, nature, saisie) VALUES ($1,$2,$3,'ajoute',true)",
    )
    .bind(s)
    .bind(x.etab)
    .bind(co)
    .execute(x.ent.pool())
    .await
    .unwrap();
    let a1 = x.declarer(x.prof, "prof", Some("maladie"), 1, 1).await.absences[0].id;
    assert!(x.impacts(a1).await.is_empty(), "le co-enseignant est présent : séance non touchée");
    // Le co-enseignant est absent à son tour : la séance n'a plus personne.
    let a2 = x.declarer(x.prof2, "prof", Some("maladie"), 1, 1).await.absences[0].id;
    x.valider(a2).await;
    let i = x.impacts(a1).await;
    assert_eq!(i.len(), 1, "{i:?}");
    // Et l'inverse : le titulaire est absent (validé) ; l'impact du co-enseignant porte la séance.
    x.valider(a1).await;
    assert_eq!(x.impacts(a2).await.len(), 1);
}

/// CAP-26 : séance neutralisée par le 10 (sortie de toute la classe) : impact « sans objet », pas à couvrir.
/// CAP-27 : pendant les vacances (aucune séance) : aucun impact, mais l'absence existe (organisation, paie).
#[tokio::test]
async fn a3_cap26_cap27_sortie_de_la_classe_et_vacances() {
    let x = cadre!();
    let c = x.creneau(&x.classe, x.prof, 1, (9, 0), (10, 0)).await;
    let s = x.seance(c, x.prof, 1, (9, 0), (10, 0), "prevue").await;
    x.forcer_etat_seance(s, "neutralisee").await;
    let a = x.declarer(x.prof, "prof", Some("maladie"), 1, 1).await.absences[0].id;
    let i = x.impacts(a).await;
    assert_eq!(i.len(), 1);
    assert_eq!(i[0].etat, "sans_objet", "CAP-26 : {i:?}");
    // CAP-27 : un jour sans séance (vacances) : aucun impact ; l'absence reste visible de l'organisation.
    let v = x.declarer(x.prof2, "prof", Some("maladie"), 3, 3).await.absences[0].id;
    assert!(x.impacts(v).await.is_empty());
    let org = x.ent.absences_organisation_for_actor(x.etab, x.dir, "direction", x.jour(3), x.jour(3)).await.unwrap();
    assert!(org.iter().any(|o| o.id == v), "l'absence existe pour la paie et l'organisation");
}

/// CAP-46 : nouvelle version d'EDT : le recalcul conserve l'état et la réponse des séances qui ont gardé leur
/// identité ; celles qui ont disparu avec une réponse posée sont SIGNALÉES ; une séance sans réponse disparue
/// est simplement retirée ; une séance nouvelle devient « à traiter ».
#[tokio::test]
async fn a3_cap46_nouvelle_version_d_edt_conserve_signale_et_recalcule() {
    let x = cadre!();
    let c1 = x.creneau(&x.classe, x.prof, 1, (9, 0), (10, 0)).await;
    let c2 = x.creneau(&x.classe, x.prof, 1, (10, 0), (11, 0)).await;
    let c3 = x.creneau(&x.classe, x.prof, 1, (11, 0), (12, 0)).await;
    let s1 = x.seance(c1, x.prof, 1, (9, 0), (10, 0), "prevue").await;
    let s2 = x.seance(c2, x.prof, 1, (10, 0), (11, 0), "prevue").await;
    let _s3 = x.seance(c3, x.prof, 1, (11, 0), (12, 0), "prevue").await;
    let a = x.declarer(x.prof, "prof", Some("maladie"), 1, 1).await.absences[0].id;
    assert_eq!(x.impacts(a).await.len(), 3);
    // Réponses posées (le lot A4 les posera ; ici on les simule en base) : s1 sollicitée, s2 en continuité
    // avec une réponse active.
    sqlx::query("UPDATE ent_absp_impact SET etat = 'sollicite' WHERE etablissement_id = $1 AND seance_id = $2")
        .bind(x.etab)
        .bind(s1)
        .execute(x.ent.pool())
        .await
        .unwrap();
    let imp2: Uuid = sqlx::query_scalar("SELECT id FROM ent_absp_impact WHERE etablissement_id = $1 AND seance_id = $2")
        .bind(x.etab)
        .bind(s2)
        .fetch_one(x.ent.pool())
        .await
        .unwrap();
    let rep = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_absp_reponse (id, etablissement_id, impact_id, type, posee_par_user_id) VALUES ($1,$2,$3,'continuite',$4)")
        .bind(rep).bind(x.etab).bind(imp2).bind(x.dir).execute(x.ent.pool()).await.unwrap();
    sqlx::query("UPDATE ent_absp_impact SET etat = 'continuite', reponse_active_id = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(x.etab).bind(imp2).bind(rep).execute(x.ent.pool()).await.unwrap();
    // Nouvelle version d'EDT : s1 garde son identité ; s2 et s3 disparaissent (retirées) ; une séance nouvelle
    // apparaît à 14 h.
    for s in [s2, _s3] {
        sqlx::query("UPDATE ent_edt_seance SET etat = 'retiree', statut = 'retiree', retiree_le = now() WHERE id = $1")
            .bind(s).execute(x.ent.pool()).await.unwrap();
    }
    let c4 = x.creneau(&x.classe, x.prof, 1, (14, 0), (15, 0)).await;
    x.seance(c4, x.prof, 1, (14, 0), (15, 0), "prevue").await;
    let i = x.impacts(a).await;
    let par_heure = |h: (u32, u32)| i.iter().find(|x| x.seance.as_ref().is_some_and(|s| (s.heure_debut.hour(), s.heure_debut.minute()) == h));
    use chrono::Timelike;
    assert_eq!(par_heure((9, 0)).map(|x| (x.etat.as_str(), x.a_revoir)), Some(("sollicite", false)), "{i:?}");
    assert_eq!(par_heure((14, 0)).map(|x| (x.etat.as_str(), x.a_revoir)), Some(("a_traiter", false)), "{i:?}");
    // La séance retirée avec une réponse posée est conservée et signalée ; sans réponse, elle disparaît.
    let signalee: Option<(String, bool)> =
        sqlx::query_as("SELECT etat, a_revoir FROM ent_absp_impact WHERE etablissement_id = $1 AND id = $2")
            .bind(x.etab).bind(imp2).fetch_optional(x.ent.pool()).await.unwrap();
    assert_eq!(signalee, Some(("continuite".to_string(), true)), "réponse conservée et signalée");
    let restantes: i64 = x
        .compter("SELECT count(*) FROM ent_absp_impact WHERE etablissement_id = $1")
        .await;
    assert_eq!(restantes, 3, "s1, s2 (signalée), s4 : la séance retirée sans réponse a disparu");
    // Idempotent : un second recalcul ne change rien.
    let avant = x.impacts(a).await;
    let apres = x.impacts(a).await;
    assert_eq!(avant, apres);
}

/// Reprise des liens : un remplacement posé par l'écran existant (exception liée à l'absence) donne l'état
/// « remplacé » et une réponse reprise ; la levée de l'exception rend la séance « à traiter ».
#[tokio::test]
async fn a3_un_remplacement_deja_pose_est_repris_comme_reponse() {
    let x = cadre!();
    let c = x.creneau(&x.classe, x.prof, 1, (9, 0), (10, 0)).await;
    x.seance(c, x.prof, 1, (9, 0), (10, 0), "prevue").await;
    let a = x.declarer(x.prof, "prof", Some("convenance"), 1, 1).await.absences[0].id;
    x.valider(a).await;
    assert_eq!(x.impacts(a).await[0].etat, "a_traiter");
    x.ent
        .edt_organiser_remplacement(
            &x.d(),
            &CmdRemplacement {
                creneau_id: c,
                jour: x.jour(1),
                remplacant_user_id: x.prof2,
                absence_rh_id: Some(a),
                nouvelle_salle: None,
            },
        )
        .await
        .unwrap();
    let i = x.impacts(a).await;
    assert_eq!(i[0].etat, "remplace", "{i:?}");
    let (n, ref_ok): (i64, bool) = sqlx::query_as(
        "SELECT count(*), bool_and(modification_edt_ref IS NOT NULL AND type = 'remplacement') FROM ent_absp_reponse
          WHERE etablissement_id = $1 AND etat = 'active'",
    )
    .bind(x.etab)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert_eq!((n, ref_ok), (1, true));
    // Idempotent : pas de seconde réponse.
    x.impacts(a).await;
    assert_eq!(x.compter("SELECT count(*) FROM ent_absp_reponse WHERE etablissement_id = $1").await, 1);
    // L'exception disparaît (levée) : la réponse reprise est levée, la séance redevient à traiter.
    assert!(x.ent.edt_lever_exception(&x.d(), c, x.jour(1)).await.unwrap());
    let i = x.impacts(a).await;
    assert_eq!(i[0].etat, "a_traiter", "{i:?}");
    assert_eq!(x.compter("SELECT count(*) FROM ent_absp_reponse WHERE etablissement_id = $1 AND etat = 'active'").await, 0);
}

/// Tableau « ce matin » : impacts par heure et par état, absents du jour ; confidentialité renforcée : la
/// vie scolaire voit « séance à couvrir » sans le nom ; aucune nature nulle part ; compteurs du tableau de bord.
#[tokio::test]
async fn a3_tableau_ce_matin_compteurs_et_confidentialite() {
    let x = cadre!();
    let c1 = x.creneau(&x.classe, x.prof, 1, (9, 0), (10, 0)).await;
    let c2 = x.creneau(&x.classe, x.prof2, 1, (10, 0), (11, 0)).await;
    x.seance(c1, x.prof, 1, (9, 0), (10, 0), "prevue").await;
    x.seance(c2, x.prof2, 1, (10, 0), (11, 0), "prevue").await;
    let a1 = x.declarer(x.prof, "prof", Some("maladie"), 1, 1).await.absences[0].id;
    let a2 = x.declarer(x.prof2, "prof", Some("convenance"), 1, 1).await.absences[0].id;
    let jour = x.jour(1);
    let tab = x.ent.tableau_jour_for_actor(x.etab, x.cpe, "cpe", jour).await.unwrap();
    assert_eq!(tab.lignes.len(), 2, "deux heures : 9 h et 10 h");
    assert_eq!(tab.lignes.iter().map(|l| l.impacts.len()).sum::<usize>(), 2);
    assert_eq!(tab.compteurs.a_traiter, 2);
    assert_eq!(tab.absents.len(), 2);
    assert!(tab.lignes.iter().flat_map(|l| &l.impacts).all(|i| i.agent_label.is_some()));
    let json = serde_json::to_string(&tab).unwrap();
    for interdit in ["maladie", "convenance", "nature", "motif"] {
        assert!(!json.contains(interdit), "« {interdit} » dans le tableau : {json}");
    }
    // Confidentialité renforcée sur a1 : le CPE ne voit plus le nom, ni l'absent ; la direction voit tout.
    x.ent.definir_confidentialite_renforcee_for_actor(x.etab, x.dir, "direction", a1, true).await.unwrap();
    let tab = x.ent.tableau_jour_for_actor(x.etab, x.cpe, "cpe", jour).await.unwrap();
    assert_eq!(tab.lignes.iter().map(|l| l.impacts.len()).sum::<usize>(), 2, "la séance reste à couvrir");
    let sans_nom = tab.lignes.iter().flat_map(|l| &l.impacts).filter(|i| i.agent_label.is_none()).count();
    assert_eq!(sans_nom, 1, "un seul impact sans nom : celui de l'absence renforcée");
    assert_eq!(tab.absents.len(), 1);
    assert!(x.ent.impact_absence_for_actor(x.etab, x.cpe, "cpe", a1).await.is_err());
    let tab = x.ent.tableau_jour_for_actor(x.etab, x.dir, "direction", jour).await.unwrap();
    assert!(tab.lignes.iter().flat_map(|l| &l.impacts).all(|i| i.agent_label.is_some()));
    // Droits : ni le professeur, ni l'accueil ; fenêtre bornée.
    for (u, r) in [(x.prof, "prof"), (x.accueil, "accueil")] {
        let e = x.ent.tableau_jour_for_actor(x.etab, u, r, jour).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{r} : {e:?}");
    }
    let e = x.ent.tableau_jour_for_actor(x.etab, x.dir, "direction", today_utc() + Duration::days(60)).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)), "{e:?}");
    // Compteurs : la direction voit les absences à valider.
    let c = x.ent.compteurs_tableau_de_bord_for_actor(x.etab, x.dir, "direction").await.unwrap();
    assert!(c.a_valider >= 1, "{c:?}");
    // File à traiter (instructeur de l'État) : la maladie d'un agent de l'État, une fois prise en compte.
    x.valider(a1).await;
    let f = x.ent.file_a_traiter_for_actor(x.etab, x.sec, "administratif").await.unwrap();
    assert!(f.iter().any(|a| a.id == a1), "{f:?}");
    assert!(x.ent.file_a_traiter_for_actor(x.etab, x.prof, "prof").await.is_err());
    let _ = a2;
}

/// ABP-172 « prochaine étape », ABP-189 « sans objet », ABP-272 interrupteur « EDT de référence ».
#[tokio::test]
async fn a3_prochaine_etape_sans_objet_et_interrupteur_edt() {
    let x = cadre!();
    let c = x.creneau(&x.classe, x.prof, 1, (9, 0), (10, 0)).await;
    x.seance(c, x.prof, 1, (9, 0), (10, 0), "prevue").await;
    let a = x.declarer(x.prof, "prof", Some("convenance"), 1, 1).await.absences[0].id;
    let e = x.ent.prochaine_etape_for_actor(x.etab, x.prof, "prof", a).await.unwrap();
    assert_eq!((e.acteur.as_str(), e.action.as_str()), ("valideur", "valider"));
    assert!(x.ent.prochaine_etape_for_actor(x.etab, x.prof2, "prof", a).await.is_err(), "hors cercle");
    // « Sans objet » : à traiter seulement, droit d'organiser l'EDT + lecture d'organisation, journalisé.
    let imp = x.impacts(a).await[0].id;
    for (u, r) in [(x.prof, "prof"), (x.admin, "admin"), (x.accueil, "accueil")] {
        let e = x.ent.marquer_impact_sans_objet_for_actor(x.etab, u, r, imp).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{r} : {e:?}");
    }
    let r = x.ent.marquer_impact_sans_objet_for_actor(x.etab, x.dir, "direction", imp).await.unwrap();
    assert_eq!(r.etat, "sans_objet");
    let e = x.ent.marquer_impact_sans_objet_for_actor(x.etab, x.dir, "direction", imp).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)), "déjà sans objet : {e:?}");
    // Interrupteur : refusé sans le droit ; à la pose, l'EDT doit avoir des séances dans les 14 jours.
    let e = x.ent.definir_edt_reference_for_actor(x.etab, x.sec, "administratif", true).await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    assert!(!x.ent.edt_reference_for_actor(x.etab, x.dir, "direction").await.unwrap().actif);
    sqlx::query("UPDATE ent_edt_seance SET jour = ent_aujourdhui_etab($1, now()) + 60 WHERE etablissement_id = $1")
        .bind(x.etab).execute(x.ent.pool()).await.unwrap();
    let e = x.ent.definir_edt_reference_for_actor(x.etab, x.dir, "direction", true).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)), "aucune séance dans les 14 jours : {e:?}");
    sqlx::query("UPDATE ent_edt_seance SET jour = ent_aujourdhui_etab($1, now()) + 2 WHERE etablissement_id = $1")
        .bind(x.etab).execute(x.ent.pool()).await.unwrap();
    let r = x.ent.definir_edt_reference_for_actor(x.etab, x.dir, "direction", true).await.unwrap();
    assert!(r.actif && r.pose_par_user_id == Some(x.dir));
    assert!(x.ent.edt_reference_for_actor(x.etab, x.cpe, "cpe").await.unwrap().actif);
    assert!(!x.ent.definir_edt_reference_for_actor(x.etab, x.dir, "direction", false).await.unwrap().actif);
}

/// Cloisonnement d'établissement et tâche planifiée : l'impact d'un établissement n'est lisible d'aucun autre ;
/// le recalcul planifié passe sur les absences à venir sans erreur et est idempotent.
#[tokio::test]
async fn a3_cloisonnement_et_recalcul_planifie() {
    let x = cadre!();
    let c = x.creneau(&x.classe, x.prof, 1, (9, 0), (10, 0)).await;
    x.seance(c, x.prof, 1, (9, 0), (10, 0), "prevue").await;
    let a = x.declarer(x.prof, "prof", Some("maladie"), 1, 1).await.absences[0].id;
    let autre = Uuid::new_v4();
    let dir_b = compte(&x.ent, autre, "direction").await;
    let e = x.ent.impact_absence_for_actor(autre, dir_b, "direction", a).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    assert!(x.ent.tableau_jour_for_actor(autre, dir_b, "direction", x.jour(1)).await.unwrap().lignes.is_empty());
    let b1 = x.ent.recalculer_impacts_a_venir().await.unwrap();
    assert!(b1.absences >= 1, "{b1:?}");
    let avant = x.impacts(a).await;
    x.ent.recalculer_impacts_a_venir().await.unwrap();
    assert_eq!(x.impacts(a).await, avant, "idempotent");
    // Une absence annulée ne touche plus rien : ses impacts sans réponse disparaissent.
    x.ent.annuler_absence_for_actor(x.etab, x.prof, "prof", a, &DecisionBody::default()).await.unwrap();
    assert!(x.impacts(a).await.is_empty());
}

// ═══════════════════════════════════════════════ LOT A4 : RÉPONSE ET SOLLICITATION ═══════════════════════════════════════════════
// Critères de fin (§28.1) : jamais deux plages acceptées qui se chevauchent (test de concurrence) ; sortie
// anticipée refusée sans information (R-AP24) ; aucune nature dans les messages (R-AP41). + AR-10, AR-26.

impl Cadre {
    /// Une absence validée (nature « convenance » : décision interne) de `prof` sur le jour ISO 1, avec une
    /// séance `(d, f)` ; renvoie (absence, créneau, impact).
    async fn absence_avec_seance(&self, prof: Uuid, classe: &str, d: (u32, u32), f: (u32, u32)) -> (Uuid, Uuid, Uuid) {
        self.horloge_de_service().await;
        let c = self.creneau(classe, prof, 1, d, f).await;
        self.seance(c, prof, 1, d, f, "prevue").await;
        let a = self.declarer(prof, "prof", Some("convenance"), 1, 1).await.absences[0].id;
        self.valider(a).await;
        let impact = self.impacts(a).await.into_iter().find(|i| i.seance.as_ref().is_some_and(|s| s.creneau_id == c)).expect("impact").id;
        (a, c, impact)
    }

    /// La plage calme (§11.3) retient toute sollicitation hors de 6 h 30 – 20 h, heure de l'établissement : un
    /// test ne doit pas dépendre de l'heure où on le lance. On règle donc le fuseau de l'établissement de test
    /// sur un décalage fixe (`Etc/GMT±N`) qui met l'heure locale vers midi, à n'importe quelle heure UTC.
    async fn horloge_de_service(&self) {
        self.horloge_locale(12).await;
    }

    /// Fuseau fixe qui met l'heure locale de l'établissement vers `heure` (0..24), quelle que soit l'heure UTC.
    async fn horloge_locale(&self, heure: i32) {
        use chrono::Timelike;
        let decalage = (heure - chrono::Utc::now().hour() as i32).rem_euclid(24);
        let decalage = if decalage > 12 { decalage - 24 } else { decalage }; // -11 ..= 12
        let fuseau = if decalage >= 0 { format!("Etc/GMT-{decalage}") } else { format!("Etc/GMT+{}", -decalage) };
        sqlx::query(
            "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, $2)
             ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = $2",
        )
        .bind(self.etab)
        .bind(&fuseau)
        .execute(self.ent.pool())
        .await
        .unwrap_or_else(|e| panic!("fuseau {fuseau} : {e}"));
    }

    async fn agent_user(&self, user: Uuid) -> Uuid {
        self.agent(user).await
    }

    async fn etat_impact(&self, impact: Uuid) -> String {
        sqlx::query_scalar("SELECT etat FROM ent_absp_impact WHERE etablissement_id = $1 AND id = $2")
            .bind(self.etab)
            .bind(impact)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    async fn etat_sollicitation(&self, id: Uuid) -> String {
        sqlx::query_scalar("SELECT etat FROM ent_absp_sollicitation WHERE etablissement_id = $1 AND id = $2")
            .bind(self.etab)
            .bind(id)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }
}

/// Parcours nominal : propositions (avec raisons), sollicitation (message neutre, événement sans nature),
/// acceptation (le remplacement est appliqué à l'emploi du temps et repris comme réponse), puis levée.
#[tokio::test]
async fn a4_proposer_solliciter_accepter_puis_lever() {
    let x = cadre!();
    // prof2 enseigne déjà la matière (un créneau un autre jour) : il est « même discipline ».
    x.creneau(&x.classe, x.prof2, 3, (14, 0), (15, 0)).await;
    let (abs, _c, impact) = x.absence_avec_seance(x.prof, &x.classe, (9, 0), (10, 0)).await;
    let ag2 = x.agent_user(x.prof2).await;
    let props = x.ent.propositions_impact_for_actor(x.etab, x.dir, "direction", impact).await.unwrap();
    let p2 = props.iter().find(|p| p.agent_id == ag2).expect("prof2 proposé");
    assert!(p2.meme_discipline, "{props:?}");
    let ag_absent = x.agent_user(x.prof).await;
    assert!(props.iter().all(|p| p.agent_id != ag_absent), "l'absent n'est jamais proposé");
    let json = serde_json::to_string(&props).unwrap();
    for interdit in ["convenance", "maladie", "nature", "motif", "\"age\"", "anciennete", "sante", "temps_partiel"] {
        assert!(!json.contains(interdit), "« {interdit} » dans les propositions (R-AP37) : {json}");
    }
    // Sollicitation : message neutre, événement sans nature, impact « sollicité ».
    let s = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", impact, ag2).await.unwrap();
    assert_eq!((s.etat.as_str(), s.mode.as_str()), ("envoyee", "sollicitation"));
    assert!(s.message.contains("Mathématiques"), "{}", s.message);
    for interdit in ["convenance", "maladie", "absent", "absence"] {
        assert!(!s.message.to_lowercase().contains(interdit), "« {interdit} » dans le message : {}", s.message);
    }
    assert_eq!(x.etat_impact(impact).await, "sollicite");
    let payload: String = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'absence_personnel.sollicitation_envoyee'",
    )
    .bind(x.etab)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    for interdit in ["convenance", "maladie", "nature", "motif"] {
        assert!(!payload.contains(interdit), "« {interdit} » dans l'événement : {payload}");
    }
    // Une seule demande ouverte par séance.
    let ag3 = x.agent_user(x.prof3).await;
    let e = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", impact, ag3).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)), "{e:?}");
    // ABP-158 : le remplaçant voit sa demande ; personne d'autre.
    let mes = x.ent.mes_sollicitations_for_actor(x.etab, x.prof2, "prof").await.unwrap();
    assert_eq!(mes.len(), 1);
    assert!(x.ent.mes_sollicitations_for_actor(x.etab, x.prof3, "prof").await.unwrap().is_empty());
    let e = x.ent.accepter_sollicitation_for_actor(x.etab, x.prof3, "prof", s.id).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "demande d'un autre : {e:?}");
    // Acceptation : le remplacement est appliqué, l'impact devient « remplacé » avec une réponse reprise.
    let ok = x.ent.accepter_sollicitation_for_actor(x.etab, x.prof2, "prof", s.id).await.unwrap();
    assert_eq!(ok.etat, "acceptee");
    assert_eq!(x.etat_impact(impact).await, "remplace");
    let (n, lie): (i64, bool) = sqlx::query_as(
        "SELECT count(*), bool_and(s.reponse_id IS NOT NULL) FROM ent_absp_sollicitation s WHERE s.etablissement_id = $1 AND s.etat = 'acceptee'",
    )
    .bind(x.etab)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert_eq!((n, lie), (1, true));
    let rem: Option<Uuid> = sqlx::query_scalar("SELECT remplacant_user_id FROM ent_edt_exception WHERE etablissement_id = $1")
        .bind(x.etab)
        .fetch_one(x.ent.pool())
        .await
        .unwrap();
    assert_eq!(rem, Some(x.prof2), "l'emploi du temps connaît le remplaçant");
    // Levée : l'exception est levée (délégation révoquée), la demande retenue annulée, la séance à traiter.
    let r = x.ent.lever_reponse_for_actor(x.etab, x.dir, "direction", impact).await.unwrap();
    assert_eq!(r.etat, "a_traiter");
    assert_eq!(x.etat_sollicitation(s.id).await, "annulee");
    assert_eq!(x.compter("SELECT count(*) FROM ent_edt_exception WHERE etablissement_id = $1").await, 0);
    let h = x.ent.historique_absence_for_actor(x.etab, x.dir, "direction", abs).await.unwrap();
    for a in ["sollicitation_envoyee", "sollicitation_acceptee", "reponse_levee"] {
        assert!(h.iter().any(|j| j.action == a), "{a} non journalisé");
    }
}

/// CAP-31 / R-AP19 / AR-09 : un agent n'a jamais deux séances acceptées qui se chevauchent — **test de
/// concurrence** : deux acceptations simultanées, une seule passe ; et la base refuse tout contournement.
#[tokio::test]
async fn a4_cap31_deux_acceptations_simultanees_qui_se_chevauchent() {
    let x = cadre!();
    let classe_b = x.classe_bis().await;
    let (_a1, _c1, i1) = x.absence_avec_seance(x.prof, &x.classe, (9, 0), (10, 0)).await;
    let (_a2, _c2, i2) = x.absence_avec_seance(x.prof3, &classe_b, (9, 30), (10, 30)).await;
    let ag2 = x.agent_user(x.prof2).await;
    let s1 = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", i1, ag2).await.unwrap();
    let s2 = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", i2, ag2).await.unwrap();
    let (r1, r2) = tokio::join!(
        x.ent.accepter_sollicitation_for_actor(x.etab, x.prof2, "prof", s1.id),
        x.ent.accepter_sollicitation_for_actor(x.etab, x.prof2, "prof", s2.id),
    );
    let gagnants = [r1.is_ok(), r2.is_ok()].iter().filter(|b| **b).count();
    assert_eq!(gagnants, 1, "une seule acceptation passe : {r1:?} / {r2:?}");
    let perdant = if r1.is_ok() { r2 } else { r1 };
    assert!(matches!(perdant, Err(EntDbError::Refus(..))), "refus lisible, pas une panne : {perdant:?}");
    let acceptees: i64 = x
        .compter("SELECT count(*) FROM ent_absp_sollicitation WHERE etablissement_id = $1 AND etat IN ('acceptee', 'designee')")
        .await;
    assert_eq!(acceptees, 1);
    // La base refuse tout contournement : forcer la seconde en « acceptée » viole l'exclusion.
    let (id_perdant,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_absp_sollicitation WHERE etablissement_id = $1 AND etat = 'envoyee' ORDER BY id LIMIT 1",
    )
    .bind(x.etab)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    let force = sqlx::query("UPDATE ent_absp_sollicitation SET etat = 'acceptee' WHERE id = $1").bind(id_perdant).execute(x.ent.pool()).await;
    assert!(force.is_err(), "deux plages acceptées qui se chevauchent ont été acceptées par la base");
    // Une désignation sur la plage déjà prise est refusée de la même façon.
    let autre = if r1_est(&x, i1).await { i2 } else { i1 };
    let e = x.ent.designer_for_actor(x.etab, x.dir, "direction", autre, ag2).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)), "{e:?}");
}

async fn r1_est(x: &Cadre, impact: Uuid) -> bool {
    x.etat_impact(impact).await == "remplace"
}

/// ABP-169, 178, 247 : désignation d'urgence (la plage est réservée, le remplacement appliqué), accusé de
/// réception du seul remplaçant, alerte quand il manque une heure avant la séance (une seule fois).
#[tokio::test]
async fn a4_designation_accuse_de_reception_et_alerte() {
    let x = cadre!();
    let (_a, _c, impact) = x.absence_avec_seance(x.prof, &x.classe, (9, 0), (10, 0)).await;
    let ag2 = x.agent_user(x.prof2).await;
    // Le droit d'organiser : ni un professeur, ni l'administrateur technique, ni l'accueil.
    for (u, r) in [(x.prof, "prof"), (x.admin, "admin"), (x.accueil, "accueil")] {
        let e = x.ent.designer_for_actor(x.etab, u, r, impact, ag2).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{r} : {e:?}");
    }
    // L'absent ne peut pas être désigné pour sa propre séance.
    let ag_absent = x.agent_user(x.prof).await;
    assert!(x.ent.designer_for_actor(x.etab, x.dir, "direction", impact, ag_absent).await.is_err());
    let d = x.ent.designer_for_actor(x.etab, x.dir, "direction", impact, ag2).await.unwrap();
    assert_eq!((d.etat.as_str(), d.mode.as_str()), ("designee", "designation"));
    assert_eq!(x.etat_impact(impact).await, "remplace");
    // Une seconde désignation sur la séance déjà pourvue est refusée.
    let ag3 = x.agent_user(x.prof3).await;
    assert!(x.ent.designer_for_actor(x.etab, x.dir, "direction", impact, ag3).await.is_err());
    // Accusé de réception : le remplaçant seul ; idempotent.
    let e = x.ent.accuser_reception_for_actor(x.etab, x.prof3, "prof", d.id).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    // Alerte : sans accusé, une heure avant la séance (la plage est avancée en base pour le test).
    sqlx::query("UPDATE ent_absp_sollicitation SET plage = tstzrange(now() + interval '30 minutes', now() + interval '90 minutes') WHERE id = $1")
        .bind(d.id)
        .execute(x.ent.pool())
        .await
        .unwrap();
    let b = x.ent.traiter_sollicitations().await.unwrap();
    assert!(b.accuses_manquants >= 1, "{b:?}");
    assert_eq!(x.nb_events("absence_personnel.accuse_manquant").await, 1);
    let b = x.ent.traiter_sollicitations().await.unwrap();
    assert_eq!(b.accuses_manquants, 0, "une seule alerte par désignation");
    let a = x.ent.accuser_reception_for_actor(x.etab, x.prof2, "prof", d.id).await.unwrap();
    assert!(a.accuse_le.is_some());
    let a2 = x.ent.accuser_reception_for_actor(x.etab, x.prof2, "prof", d.id).await.unwrap();
    assert_eq!(a.accuse_le, a2.accuse_le, "idempotent");
}

/// Expiration (S-1505), refus libre et sans motif, acceptation périmée refusée proprement (ABP-299).
#[tokio::test]
async fn a4_expiration_refus_et_acceptation_perimee() {
    let x = cadre!();
    let (_a, _c, impact) = x.absence_avec_seance(x.prof, &x.classe, (9, 0), (10, 0)).await;
    let ag2 = x.agent_user(x.prof2).await;
    let s = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", impact, ag2).await.unwrap();
    assert!(s.expire_le.is_some());
    sqlx::query("UPDATE ent_absp_sollicitation SET expire_le = now() - interval '1 minute' WHERE id = $1")
        .bind(s.id)
        .execute(x.ent.pool())
        .await
        .unwrap();
    // Acceptation tardive, avant même la tâche : refus propre, la demande est marquée expirée.
    let e = x.ent.accepter_sollicitation_for_actor(x.etab, x.prof2, "prof", s.id).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)) && format!("{e:?}").contains("expiré"), "{e:?}");
    assert_eq!(x.etat_sollicitation(s.id).await, "expiree");
    // La tâche : une demande échue expire, la séance redevient à traiter, la suivante peut être proposée.
    let s2 = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", impact, ag2).await.unwrap();
    sqlx::query("UPDATE ent_absp_sollicitation SET expire_le = now() - interval '1 minute' WHERE id = $1")
        .bind(s2.id)
        .execute(x.ent.pool())
        .await
        .unwrap();
    let b = x.ent.traiter_sollicitations().await.unwrap();
    assert!(b.expirees >= 1, "{b:?}");
    assert_eq!(x.etat_sollicitation(s2.id).await, "expiree");
    assert_eq!(x.etat_impact(impact).await, "a_traiter");
    assert_eq!(x.nb_events("absence_personnel.sollicitation_expiree").await, 1);
    // Refus : libre et sans motif ; un motif inconnu est refusé ; la séance redevient à traiter.
    let s3 = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", impact, ag2).await.unwrap();
    let e = x.ent.refuser_sollicitation_for_actor(x.etab, x.prof2, "prof", s3.id, Some("parce que")).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)), "{e:?}");
    let e = x.ent.refuser_sollicitation_for_actor(x.etab, x.prof3, "prof", s3.id, None).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    let r = x.ent.refuser_sollicitation_for_actor(x.etab, x.prof2, "prof", s3.id, None).await.unwrap();
    assert_eq!(r.etat, "refusee");
    assert_eq!(x.etat_impact(impact).await, "a_traiter");
    // Séance déjà pourvue : une demande restée ouverte est refusée proprement à l'acceptation.
    let s4 = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", impact, ag2).await.unwrap();
    sqlx::query("UPDATE ent_absp_impact SET etat = 'remplace' WHERE etablissement_id = $1 AND id = $2")
        .bind(x.etab)
        .bind(impact)
        .execute(x.ent.pool())
        .await
        .unwrap();
    let e = x.ent.accepter_sollicitation_for_actor(x.etab, x.prof2, "prof", s4.id).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)), "{e:?}");
    assert_eq!(x.etat_sollicitation(s4.id).await, "annulee");
}

/// ABP-185 : appel à volontaires — le premier qui accepte est retenu, les autres sont annulés (« prise »).
#[tokio::test]
async fn a4_appel_a_volontaires_premier_accord_retenu() {
    let x = cadre!();
    let (_a, _c, impact) = x.absence_avec_seance(x.prof, &x.classe, (9, 0), (10, 0)).await;
    let (ag2, ag3) = (x.agent_user(x.prof2).await, x.agent_user(x.prof3).await);
    assert!(x.ent.appel_volontaires_for_actor(x.etab, x.dir, "direction", impact, &[ag2]).await.is_err(), "au moins deux");
    let v = x.ent.appel_volontaires_for_actor(x.etab, x.dir, "direction", impact, &[ag2, ag3]).await.unwrap();
    assert_eq!(v.len(), 2);
    let s2 = v.iter().find(|s| s.agent_id == ag2).unwrap().id;
    let s3 = v.iter().find(|s| s.agent_id == ag3).unwrap().id;
    x.ent.accepter_sollicitation_for_actor(x.etab, x.prof2, "prof", s2).await.unwrap();
    assert_eq!(x.etat_sollicitation(s3).await, "annulee", "prévenu : la séance est prise");
    let e = x.ent.accepter_sollicitation_for_actor(x.etab, x.prof3, "prof", s3).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)), "{e:?}");
    assert_eq!(x.etat_impact(impact).await, "remplace");
}

/// R-AP24 : aucune sortie anticipée sans information préalable des familles ; annuler sans sortie anticipée
/// est possible (surveillance dans les horaires habituels) et rend l'état « annulé ».
#[tokio::test]
async fn a4_r_ap24_pas_de_sortie_anticipee_sans_information() {
    let x = cadre!();
    let (_a, c, impact) = x.absence_avec_seance(x.prof, &x.classe, (9, 0), (10, 0)).await;
    let e = x.ent.annuler_seance_for_actor(x.etab, x.dir, "direction", impact, true).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)) && format!("{e:?}").contains("information préalable"), "{e:?}");
    assert_eq!(x.etat_impact(impact).await, "a_traiter", "rien n'a été écrit");
    assert_eq!(x.compter("SELECT count(*) FROM ent_edt_exception WHERE etablissement_id = $1").await, 0);
    for (u, r) in [(x.prof, "prof"), (x.accueil, "accueil")] {
        let e = x.ent.annuler_seance_for_actor(x.etab, u, r, impact, false).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{r} : {e:?}");
    }
    let r = x.ent.annuler_seance_for_actor(x.etab, x.dir, "direction", impact, false).await.unwrap();
    assert_eq!(r.etat, "annule");
    let kind: String = sqlx::query_scalar("SELECT kind FROM ent_edt_exception WHERE etablissement_id = $1 AND creneau_id = $2")
        .bind(x.etab)
        .bind(c)
        .fetch_one(x.ent.pool())
        .await
        .unwrap();
    assert_eq!(kind, "annulation");
    // Levée : la séance redevient normale.
    assert_eq!(x.ent.lever_reponse_for_actor(x.etab, x.dir, "direction", impact).await.unwrap().etat, "a_traiter");
    assert!(x.ent.lever_reponse_for_actor(x.etab, x.dir, "direction", impact).await.is_err(), "rien à lever");
}

/// Confidentialité renforcée et cloisonnement : une absence en confidentialité renforcée n'est répondable que
/// de la direction ; un autre établissement ne voit rien ; l'absent ne peut pas être sollicité pour lui-même.
#[tokio::test]
async fn a4_confidentialite_cloisonnement_et_nature_absente() {
    let x = cadre!();
    let (a, _c, impact) = x.absence_avec_seance(x.prof, &x.classe, (9, 0), (10, 0)).await;
    let ag2 = x.agent_user(x.prof2).await;
    x.ent.definir_confidentialite_renforcee_for_actor(x.etab, x.dir, "direction", a, true).await.unwrap();
    // Le secrétariat (droit d'organiser l'EDT ? non : edt.gerer) ou la vie scolaire ne répondent pas à une
    // absence renforcée ; la direction oui.
    for (u, r) in [(x.sec, "administratif"), (x.cpe, "cpe")] {
        let e = x.ent.propositions_impact_for_actor(x.etab, u, r, impact).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden | EntDbError::Domain(DomainError::NotFound)), "{r} : {e:?}");
    }
    assert!(x.ent.propositions_impact_for_actor(x.etab, x.dir, "direction", impact).await.is_ok());
    // Autre établissement : introuvable.
    let autre = Uuid::new_v4();
    let dir_b = compte(&x.ent, autre, "direction").await;
    let e = x.ent.solliciter_for_actor(autre, dir_b, "direction", impact, ag2).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    // Nature absente de tout ce que voit le remplaçant, y compris sur une nature sensible.
    let m = x.declarer(x.prof3, "prof", Some("maladie"), 1, 1).await.absences[0].id;
    let c3 = x.creneau(&x.classe_bis().await, x.prof3, 1, (11, 0), (12, 0)).await;
    x.seance(c3, x.prof3, 1, (11, 0), (12, 0), "prevue").await;
    let i3 = x.impacts(m).await[0].id;
    let s = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", i3, ag2).await.unwrap();
    let mes = serde_json::to_string(&x.ent.mes_sollicitations_for_actor(x.etab, x.prof2, "prof").await.unwrap()).unwrap();
    for interdit in ["maladie", "nature", "motif", "arrêt", "convenance"] {
        assert!(!mes.contains(interdit), "« {interdit} » dans ce que voit le remplaçant : {mes}");
    }
    assert!(!s.message.to_lowercase().contains("maladie"));
}

/// AR-10 : un cours déplacé vers un jour occupe celui qui l'ASSURE — son remplaçant, pas son titulaire.
#[tokio::test]
async fn a4_ar10_un_cours_deplace_occupe_son_remplacant() {
    let x = cadre!();
    // Le cours de `prof` (lundi 9 h) est déplacé au mardi 14 h, où `prof2` le remplace.
    let c = x.creneau(&x.classe, x.prof, 1, (9, 0), (10, 0)).await;
    sqlx::query(
        "INSERT INTO ent_edt_deplacement (id, etablissement_id, creneau_id, jour_origine, nouveau_jour, nouvelle_heure_debut, nouvelle_heure_fin, cree_par_user_id)
         VALUES ($1,$2,$3,$4,$5,'14:00','15:00',$6)",
    )
    .bind(Uuid::new_v4()).bind(x.etab).bind(c).bind(x.jour(1)).bind(x.jour(2)).bind(x.dir)
    .execute(x.ent.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO ent_edt_exception (id, etablissement_id, creneau_id, jour, kind, remplacant_user_id, motif, created_by_user_id)
         VALUES ($1,$2,$3,$4,'remplacement',$5,'x',$6)",
    )
    .bind(Uuid::new_v4()).bind(x.etab).bind(c).bind(x.jour(2)).bind(x.prof2).bind(x.dir)
    .execute(x.ent.pool()).await.unwrap();
    // Un autre cours, le mardi 14 h : qui est proposé pour le remplacer ?
    let cible = x.creneau(&x.classe_bis().await, x.prof3, 2, (14, 0), (15, 0)).await;
    let candidats = x.ent.edt_candidats_remplacants(&x.d(), cible, x.jour(2), true).await.unwrap();
    let users: Vec<Uuid> = candidats.iter().map(|c| c.user_id).collect();
    assert!(!users.contains(&x.prof2), "le remplaçant du cours déplacé est occupé : {users:?}");
    assert!(users.contains(&x.prof), "le titulaire, lui, est libre ce jour-là : {users:?}");
}

/// Plage calme (§11.3, Q-AP14) : la nuit, la sollicitation n'est ni visible ni acceptable avant 6 h 30 ; dans la
/// fenêtre de service, elle part tout de suite. Le test règle le fuseau pour ne pas dépendre de l'heure réelle.
#[tokio::test]
async fn a4_plage_calme_differe_l_envoi_de_nuit() {
    let x = cadre!();
    let (_a, _c, impact) = x.absence_avec_seance(x.prof, &x.classe, (9, 0), (10, 0)).await;
    let ag2 = x.agent_user(x.prof2).await;
    // En service (12 h locale) : envoyée tout de suite.
    let s = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", impact, ag2).await.unwrap();
    let diff: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar("SELECT envoi_prevu_le FROM ent_absp_sollicitation WHERE id = $1")
        .bind(s.id).fetch_one(x.ent.pool()).await.unwrap();
    assert!(diff.is_none(), "dans la fenêtre de service, l'envoi n'est pas différé");
    x.ent.refuser_sollicitation_for_actor(x.etab, x.prof2, "prof", s.id, None).await.unwrap();
    // La nuit (23 h locale) : l'envoi attend 6 h 30 ; la demande n'est ni visible ni acceptable.
    x.horloge_locale(23).await;
    let n = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", impact, ag2).await.unwrap();
    let (prevu, expire): (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        "SELECT envoi_prevu_le, expire_le FROM ent_absp_sollicitation WHERE id = $1",
    )
    .bind(n.id).fetch_one(x.ent.pool()).await.unwrap();
    let local: chrono::NaiveTime = sqlx::query_scalar(
        "SELECT (envoi_prevu_le AT TIME ZONE (SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1))::time
           FROM ent_absp_sollicitation WHERE id = $2",
    )
    .bind(x.etab).bind(n.id).fetch_one(x.ent.pool()).await.unwrap();
    assert_eq!(local, t(6, 30), "envoi à 6 h 30 locale");
    assert!(expire > prevu, "l'expiration court à partir de l'envoi, pas de la demande");
    assert!(x.ent.mes_sollicitations_for_actor(x.etab, x.prof2, "prof").await.unwrap().is_empty(), "pas encore visible");
    let e = x.ent.accepter_sollicitation_for_actor(x.etab, x.prof2, "prof", n.id).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "pas encore acceptable : {e:?}");
    assert_eq!(x.nb_events("absence_personnel.sollicitation_envoyee").await, 1, "aucun événement n'est parti la nuit (seule la demande de jour)");
    // 6 h 30 venue : visible et acceptable.
    sqlx::query("UPDATE ent_absp_sollicitation SET envoi_prevu_le = now() - interval '1 minute', expire_le = now() + interval '20 minutes' WHERE id = $1")
        .bind(n.id).execute(x.ent.pool()).await.unwrap();
    assert_eq!(x.ent.mes_sollicitations_for_actor(x.etab, x.prof2, "prof").await.unwrap().len(), 1);
    assert_eq!(x.ent.accepter_sollicitation_for_actor(x.etab, x.prof2, "prof", n.id).await.unwrap().etat, "acceptee");
}

// ═══════════════════════════════════════════════ LOT A5 : ABSENCE LONGUE ET SUPPLÉANCE ═══════════════════════════════════════════════
// Critères de fin (§28.1) : deux semaines de RCD puis suppléant (CAP-36) ; état du remplacement publié une fois par
// étape (ABP-239) ; les deux notions de remplacement reliées (AR-23). + ABP-180, ABP-181, CAP-37.

struct Longue {
    absence: Uuid,
    service: Uuid,
    /// Impacts par jour ISO de l'absence (1, 8, 15, 22 : les quatre lundis).
    impacts: Vec<(i64, Uuid)>,
}

impl Cadre {
    /// Absence validée de `prof` sur quatre semaines (jours 1 à 28), un cours le lundi relié à un service de `prof`
    /// (le service est celui de la séance : même cours). Renvoie l'absence, le service et les quatre impacts.
    async fn absence_longue(&self, prof: Uuid) -> Longue {
        self.horloge_de_service().await;
        let c = self.creneau(&self.classe, prof, 1, (9, 0), (10, 0)).await;
        let annee: Uuid = sqlx::query_scalar(
            "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 ORDER BY date_debut DESC LIMIT 1",
        )
        .bind(self.etab)
        .fetch_one(self.ent.pool())
        .await
        .unwrap();
        let agent = self.agent(prof).await;
        let service = self
            .ent
            .creer_service_for_actor(
                self.etab,
                self.dir,
                "direction",
                &ent_types::personnels_services::CreerServiceBody {
                    annee_id: Some(annee),
                    classe_id: Some(self.classe_id),
                    matiere_libelle: Some("Mathématiques".into()),
                    titulaire: Some(ent_types::personnels_services::TitulaireInitialBody {
                        agent_id: agent,
                        du: Some(self.jour(1) - Duration::days(60)),
                        heures_hebdo: None,
                    }),
                    ..Default::default()
                },
            )
            .await
            .unwrap_or_else(|e| panic!("service refusé : {e:?}"));
        for iso in [1, 8, 15, 22] {
            self.seance(c, prof, iso, (9, 0), (10, 0), "prevue").await;
        }
        sqlx::query("UPDATE ent_edt_seance SET cours_id = $3 WHERE etablissement_id = $1 AND creneau_id = $2")
            .bind(self.etab)
            .bind(c)
            .bind(service.cours_id.expect("cours du service"))
            .execute(self.ent.pool())
            .await
            .unwrap();
        let a = self.declarer(prof, "prof", Some("convenance"), 1, 28).await.absences[0].id;
        self.valider(a).await;
        let tous = self.impacts(a).await;
        let mut impacts = Vec::new();
        for iso in [1i64, 8, 15, 22] {
            let jour = self.jour(iso);
            let i = tous.iter().find(|i| i.seance.as_ref().is_some_and(|s| s.jour == jour)).expect("impact du lundi");
            impacts.push((iso, i.id));
        }
        Longue { absence: a, service: service.id, impacts }
    }

    fn impact_du(&self, l: &Longue, iso: i64) -> Uuid {
        l.impacts.iter().find(|(i, _)| *i == iso).unwrap().1
    }

    async fn etat_remplacement(&self, absence: Uuid) -> Option<EtatRemplacementDto> {
        self.ent
            .etat_remplacement_for_actor(self.etab, self.dir, "direction", absence)
            .await
            .unwrap_or_else(|e| panic!("état du remplacement refusé : {e:?}"))
    }

    async fn aujourdhui(&self) -> NaiveDate {
        sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, now())").bind(self.etab).fetch_one(self.ent.pool()).await.unwrap()
    }

    async fn nommer(&self, a: &Longue, agent: Uuid, du: NaiveDate, au: Option<NaiveDate>) -> NominationSuppleantDto {
        self.ent
            .nommer_suppleant_absence_for_actor(
                self.etab,
                self.dir,
                "direction",
                a.absence,
                &NommerSuppleantAbsenceBody { service_id: a.service, agent_id: agent, du, au },
            )
            .await
            .unwrap_or_else(|e| panic!("nomination refusée : {e:?}"))
    }
}

/// CAP-36 : les deux premières semaines en RCD (réponses de séance, A4), puis le suppléant : à la date de
/// suppléance, les séances suivantes passent `supplee` ; celles des deux premières semaines gardent leur réponse.
/// L'état du remplacement est publié UNE fois par étape (nommé, puis en cours), jamais deux.
#[tokio::test]
async fn a5_cap36_deux_semaines_de_rcd_puis_suppleant() {
    let x = cadre!();
    let l = x.absence_longue(x.prof).await;
    let (rcd, suppleant) = (x.agent(x.prof3).await, x.agent(x.prof2).await);
    // Semaines 1 et 2 : remplacement de courte durée (réponse de séance).
    for iso in [1, 8] {
        x.ent.designer_for_actor(x.etab, x.dir, "direction", x.impact_du(&l, iso), rcd).await.unwrap();
        assert_eq!(x.etat_impact(x.impact_du(&l, iso)).await, "remplace");
    }
    assert!(x.etat_remplacement(l.absence).await.is_none(), "aucune étape tant que rien n'est demandé ni nommé");
    // Semaine 3 : le suppléant, à partir du lundi 15.
    let n = x.nommer(&l, suppleant, x.jour(15), None).await;
    assert_eq!((n.seances_supplees, n.seances_deja_repondues), (2, 0), "{n:?}");
    assert_eq!(n.remplacement.etape, "nomme");
    assert_eq!(n.remplacement.a_partir_du, Some(x.jour(15)));
    assert_eq!(n.remplacement.suppleant_agent_id, Some(suppleant));
    for iso in [15, 22] {
        assert_eq!(x.etat_impact(x.impact_du(&l, iso)).await, "supplee", "jour {iso}");
    }
    for iso in [1, 8] {
        assert_eq!(x.etat_impact(x.impact_du(&l, iso)).await, "remplace", "les deux premières semaines gardent le RCD (jour {iso})");
    }
    // La suppléance est un fait du socle 6 relié à l'absence (AR-23).
    let origine: (String, Option<Uuid>) = sqlx::query_as(
        "SELECT origine, origine_ref FROM ent_service_intervenant WHERE etablissement_id = $1 AND nature = 'suppleant' AND agent_id = $2",
    )
    .bind(x.etab)
    .bind(suppleant)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert_eq!(origine, ("absence".to_string(), Some(l.absence)));
    // Un recalcul d'impact ne défait rien : `supplee` est conservé.
    x.ent.recalculer_impact_absence(x.etab, x.dir, l.absence).await.unwrap();
    assert_eq!(x.etat_impact(x.impact_du(&l, 15)).await, "supplee");
    // Une notification pour l'étape « nommé », et rejouer la tâche n'en ajoute pas.
    assert_eq!(x.nb_events("absence_personnel.etat_remplacement").await, 1);
    x.ent.avancer_etats_remplacement().await.unwrap();
    x.ent.avancer_etats_remplacement().await.unwrap();
    assert_eq!(x.nb_events("absence_personnel.etat_remplacement").await, 1, "une seule fois par étape");
    // Le jour de la suppléance arrive (on avance la date en base : le test ne dépend pas de l'heure réelle).
    let jour = x.aujourdhui().await;
    // L'intervention est immuable pour tout le monde : le test écarte le déclencheur le temps de l'écriture
    // (même technique que `forcer_etat_seance`), dans une transaction.
    let mut tx = x.ent.pool_verif().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_service_intervenant DISABLE TRIGGER ent_service_intervenant_garde").execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE ent_service_intervenant SET du = $3 WHERE etablissement_id = $1 AND service_id = $2 AND nature = 'suppleant'")
        .bind(x.etab)
        .bind(l.service)
        .bind(jour)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE ent_service_intervenant ENABLE TRIGGER ent_service_intervenant_garde").execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    x.ent.avancer_etats_remplacement().await.unwrap();
    let e = x.etat_remplacement(l.absence).await.unwrap();
    assert_eq!((e.etape.as_str(), e.a_partir_du), ("en_cours", Some(jour)));
    assert_eq!(x.nb_events("absence_personnel.etat_remplacement").await, 2, "nommé puis en cours");
    x.ent.avancer_etats_remplacement().await.unwrap();
    assert_eq!(x.nb_events("absence_personnel.etat_remplacement").await, 2);
    // Les séances déjà couvertes par le RCD à partir de cette date ne sont jamais écrasées : elles sont comptées.
    assert_eq!(x.etat_impact(x.impact_du(&l, 1)).await, "remplace");
    // L'événement ne porte que des identifiants, des dates et un code d'étape : jamais la nature.
    let payload: String = sqlx::query_scalar(
        "SELECT string_agg(payload::text, ' ') FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'absence_personnel.etat_remplacement'",
    )
    .bind(x.etab)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert!(!payload.contains("convenance") && !payload.contains("nature"), "{payload}");
}

/// ABP-180 : demande de suppléance (trace, réponse, proposition), droits, états, une seule demande en cours ;
/// ABP-181 : seule la direction nomme ; la demande est acceptée par la nomination du suppléant proposé.
#[tokio::test]
async fn a5_abp180_181_demande_reponse_nomination_et_droits() {
    let x = cadre!();
    let l = x.absence_longue(x.prof).await;
    let (suppleant, autre) = (x.agent(x.prof2).await, x.agent(x.prof3).await);
    let corps = DemanderSuppleanceBody { destinataire: "rectorat".into(), reference: Some("DOS-2027-0412".into()), souhaitee_du: Some(x.jour(15)) };
    // Droits : direction et secrétariat ; ni le professeur, ni le CPE, ni l'accueil, ni la comptabilité, ni l'admin.
    for (u, r) in [(x.prof, "prof"), (x.cpe, "cpe"), (x.accueil, "accueil"), (x.compta, "compta"), (x.admin, "admin")] {
        let e = x.ent.demander_suppleance_for_actor(x.etab, u, r, l.absence, &corps).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{r} : {e:?}");
    }
    // Entrées : destinataire codé, référence bornée et sans mot exclu.
    let mauvais = DemanderSuppleanceBody { destinataire: "n'importe qui".into(), ..corps.clone() };
    assert!(x.ent.demander_suppleance_for_actor(x.etab, x.sec, "administratif", l.absence, &mauvais).await.is_err());
    let exclu = DemanderSuppleanceBody { reference: Some("dossier cancer".into()), ..corps.clone() };
    assert!(x.ent.demander_suppleance_for_actor(x.etab, x.sec, "administratif", l.absence, &exclu).await.is_err());
    // Une absence seulement signalée n'a aucun effet extérieur (R-AP46) : pas de demande.
    let signalee = x.ent
        .signaler_absence_for_actor(x.etab, x.accueil, "accueil", &SignalerAbsenceBody { agent_id: x.agent(x.prof3).await, debut: x.jour(2), fin: x.jour(3), heure_debut: None, heure_fin: None })
        .await;
    if let Ok(s) = signalee {
        let e = x.ent.demander_suppleance_for_actor(x.etab, x.sec, "administratif", s.absences[0].id, &corps).await.unwrap_err();
        assert!(matches!(e, EntDbError::Refus(..)), "{e:?}");
    }
    // La secrétaire demande ; la demande est tracée et l'état passe « en recherche » (une notification).
    let d = x.ent.demander_suppleance_for_actor(x.etab, x.sec, "administratif", l.absence, &corps).await.unwrap();
    assert_eq!((d.etat.as_str(), d.destinataire.as_str()), ("demandee", "rectorat"));
    assert_eq!(x.etat_remplacement(l.absence).await.unwrap().etape, "en_recherche");
    assert_eq!(x.nb_events("absence_personnel.etat_remplacement").await, 1);
    // Une seule demande en cours.
    let e = x.ent.demander_suppleance_for_actor(x.etab, x.dir, "direction", l.absence, &corps).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)), "{e:?}");
    // Réponse : une proposition sans agent est refusée ; un refus laisse la recherche ouverte (rien de republié).
    let sans_agent = ReponseSuppleanceBody { issue: "proposee".into(), suppleant_agent_id: None, suppleant_du: None };
    assert!(x.ent.repondre_demande_suppleance_for_actor(x.etab, x.sec, "administratif", d.id, &sans_agent).await.is_err());
    let refus = ReponseSuppleanceBody { issue: "refusee".into(), suppleant_agent_id: None, suppleant_du: None };
    let r = x.ent.repondre_demande_suppleance_for_actor(x.etab, x.sec, "administratif", d.id, &refus).await.unwrap();
    assert_eq!(r.etat, "refusee");
    assert_eq!(x.etat_remplacement(l.absence).await.unwrap().etape, "en_recherche");
    assert_eq!(x.nb_events("absence_personnel.etat_remplacement").await, 1, "un refus n'est pas une nouvelle étape");
    let e = x.ent.repondre_demande_suppleance_for_actor(x.etab, x.sec, "administratif", d.id, &refus).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(..)), "réponse définitive : {e:?}");
    // Nouvelle demande : le rectorat propose un suppléant (l'absent ne peut pas l'être).
    let d2 = x.ent.demander_suppleance_for_actor(x.etab, x.sec, "administratif", l.absence, &corps).await.unwrap();
    let soi = ReponseSuppleanceBody { issue: "proposee".into(), suppleant_agent_id: Some(x.agent(x.prof).await), suppleant_du: None };
    assert!(x.ent.repondre_demande_suppleance_for_actor(x.etab, x.sec, "administratif", d2.id, &soi).await.is_err());
    let prop = ReponseSuppleanceBody { issue: "proposee".into(), suppleant_agent_id: Some(suppleant), suppleant_du: Some(x.jour(15)) };
    let r = x.ent.repondre_demande_suppleance_for_actor(x.etab, x.sec, "administratif", d2.id, &prop).await.unwrap();
    assert_eq!((r.etat.as_str(), r.suppleant_agent_id), ("proposee", Some(suppleant)));
    let e = x.etat_remplacement(l.absence).await.unwrap();
    assert_eq!((e.etape.as_str(), e.suppleant_agent_id, e.a_partir_du), ("propose", Some(suppleant), Some(x.jour(15))));
    assert_eq!(x.nb_events("absence_personnel.etat_remplacement").await, 2);
    // Nomination : la direction seule (le secrétariat demande mais ne nomme pas) ; l'absent n'est pas son suppléant.
    let corps_n = NommerSuppleantAbsenceBody { service_id: l.service, agent_id: suppleant, du: x.jour(15), au: None };
    let e = x.ent.nommer_suppleant_absence_for_actor(x.etab, x.sec, "administratif", l.absence, &corps_n).await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    let soi_n = NommerSuppleantAbsenceBody { agent_id: x.agent(x.prof).await, ..corps_n.clone() };
    assert!(x.ent.nommer_suppleant_absence_for_actor(x.etab, x.dir, "direction", l.absence, &soi_n).await.is_err());
    // Un service qui n'est pas celui de l'absent est refusé.
    let autre_service = x.ent
        .creer_service_for_actor(
            x.etab,
            x.dir,
            "direction",
            &ent_types::personnels_services::CreerServiceBody {
                annee_id: Some(sqlx::query_scalar("SELECT annee_id FROM ent_service_enseignement WHERE id = $1").bind(l.service).fetch_one(x.ent.pool()).await.unwrap()),
                classe_id: Some(x.classe_id),
                matiere_libelle: Some("Histoire".into()),
                titulaire: Some(ent_types::personnels_services::TitulaireInitialBody { agent_id: autre, du: Some(x.jour(1) - Duration::days(60)), heures_hebdo: None }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let hors = NommerSuppleantAbsenceBody { service_id: autre_service.id, ..corps_n.clone() };
    assert!(x.ent.nommer_suppleant_absence_for_actor(x.etab, x.dir, "direction", l.absence, &hors).await.is_err());
    let n = x.nommer(&l, suppleant, x.jour(15), None).await;
    assert_eq!(n.remplacement.etape, "nomme");
    let demandes = x.ent.demandes_suppleance_for_actor(x.etab, x.dir, "direction", l.absence).await.unwrap();
    assert_eq!(demandes.iter().map(|d| d.etat.as_str()).collect::<Vec<_>>(), vec!["acceptee", "refusee"], "la plus récente d'abord");
    assert_eq!(x.nb_events("absence_personnel.etat_remplacement").await, 3, "en recherche, proposé, nommé");
    // Journal : codes seulement.
    let journal: i64 = x.compter("SELECT count(*) FROM ent_absp_journal WHERE etablissement_id = $1 AND action IN ('suppleance_demandee', 'suppleance_reponse', 'suppleant_nomme')").await;
    assert_eq!(journal, 5);
    // Cloisonnement : un autre établissement ne lit ni la demande ni l'état.
    let autre_etab = Uuid::new_v4();
    let dir_b = compte(&x.ent, autre_etab, "direction").await;
    let e = x.ent.demandes_suppleance_for_actor(autre_etab, dir_b, "direction", l.absence).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    let e = x.ent.etat_remplacement_for_actor(autre_etab, dir_b, "direction", l.absence).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    // Un professeur ne lit pas l'organisation du remplacement d'une absence.
    let e = x.ent.etat_remplacement_for_actor(x.etab, x.prof2, "prof", l.absence).await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
}

/// AR-23 : une suppléance posée par la route du socle 6 (`origine_ref` = l'absence) a les mêmes effets qu'une
/// nomination faite ici ; écourtée au socle 6, elle rend `a_traiter` les séances qu'elle ne couvre plus.
#[tokio::test]
async fn a5_ar23_la_suppleance_du_socle_6_est_reliee_et_une_fin_anticipee_rend_les_seances() {
    let x = cadre!();
    let l = x.absence_longue(x.prof).await;
    let suppleant = x.agent(x.prof2).await;
    let s = x.ent
        .nommer_suppleant_for_actor(
            x.etab,
            x.dir,
            "direction",
            l.service,
            &ent_types::personnels_services::NommerSuppleantBody { agent_id: suppleant, du: x.jour(15), au: None, origine: None, origine_ref: Some(l.absence) },
        )
        .await
        .unwrap();
    assert_eq!(x.etat_impact(x.impact_du(&l, 15)).await, "a_traiter", "rien n'est dérivé avant la synchronisation");
    x.ent.avancer_etats_remplacement().await.unwrap();
    for iso in [15, 22] {
        assert_eq!(x.etat_impact(x.impact_du(&l, iso)).await, "supplee");
    }
    assert_eq!(x.etat_impact(x.impact_du(&l, 8)).await, "a_traiter", "avant la suppléance : inchangé");
    assert_eq!(x.etat_remplacement(l.absence).await.unwrap().etape, "nomme");
    assert_eq!(x.nb_events("absence_personnel.etat_remplacement").await, 1);
    // Fin anticipée au socle 6 (retour du titulaire) : la suppléance s'arrête le jour 18.
    let interv = s.intervenants.iter().find(|i| i.nature == "suppleant").unwrap().id;
    x.ent
        .terminer_intervention_for_actor(x.etab, x.dir, "direction", interv, &ent_types::personnels_services::TerminerInterventionBody { au: Some(x.jour(18)) })
        .await
        .unwrap();
    x.ent.avancer_etats_remplacement().await.unwrap();
    assert_eq!(x.etat_impact(x.impact_du(&l, 15)).await, "supplee");
    assert_eq!(x.etat_impact(x.impact_du(&l, 22)).await, "a_traiter", "plus couverte : jamais déclarée couverte sans suppléant");
}

/// CAP-37 : un suppléant engagé « en remplacement » dont la prise de fonctions n'est pas constatée n'a pas encore
/// de séance : elles restent à traiter, l'état affiche « à partir du … » ; à l'installation, elles passent `supplee`.
#[tokio::test]
async fn a5_cap37_suppleant_nomme_mais_pas_encore_installe() {
    let x = cadre!();
    let l = x.absence_longue(x.prof).await;
    let suppleant = x.agent(x.prof2).await;
    sqlx::query("UPDATE ent_agent_engagement SET motif_arrivee = 'remplacement', installe_le = NULL WHERE etablissement_id = $1 AND agent_id = $2")
        .bind(x.etab)
        .bind(suppleant)
        .execute(x.ent.pool())
        .await
        .unwrap();
    let n = x.nommer(&l, suppleant, x.jour(15), None).await;
    assert_eq!(n.seances_supplees, 0, "pas installé : aucune séance ne lui est confiée");
    assert_eq!((n.remplacement.etape.as_str(), n.remplacement.a_partir_du), ("nomme", Some(x.jour(15))));
    for iso in [15, 22] {
        assert_eq!(x.etat_impact(x.impact_du(&l, iso)).await, "a_traiter");
    }
    // Procès-verbal d'installation (socle 6) : les séances suivantes passent `supplee`.
    sqlx::query("UPDATE ent_agent_engagement SET installe_le = $3 WHERE etablissement_id = $1 AND agent_id = $2 AND motif_arrivee = 'remplacement'")
        .bind(x.etab)
        .bind(suppleant)
        .bind(x.jour(15))
        .execute(x.ent.pool())
        .await
        .unwrap();
    x.ent.avancer_etats_remplacement().await.unwrap();
    for iso in [15, 22] {
        assert_eq!(x.etat_impact(x.impact_du(&l, iso)).await, "supplee");
    }
    assert_eq!(x.nb_events("absence_personnel.etat_remplacement").await, 1, "toujours l'étape « nommé » : pas de nouvelle notification");
}

// ═══════════════════════════════════════════════ LOT A6 : RCD ═══════════════════════════════════════════════
// Critères de fin (§28.1) : plan versionné ; volontaires et compteurs ; remontée sans nom (test).

impl Cadre {
    async fn annee_courante(&self) -> (Uuid, NaiveDate, NaiveDate) {
        sqlx::query_as("SELECT id, date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante")
            .bind(self.etab)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    async fn plan(&self, objectifs: &str) -> PlanRcdDto {
        self.ent
            .creer_plan_rcd_for_actor(
                self.etab,
                self.dir,
                "direction",
                &PlanRcdBody { objectifs: Some(objectifs.into()), delai_prevenance_heures: Some(24), ..Default::default() },
            )
            .await
            .unwrap_or_else(|e| panic!("plan refusé : {e:?}"))
    }

    async fn engager(&self, user: Uuid, role: &str, volume: f64) -> Result<EngagementRcdDto, EntDbError> {
        let (_, debut, fin) = self.annee_courante().await;
        self.ent
            .engager_rcd_for_actor(
                self.etab,
                user,
                role,
                &EngagementRcdBody { agent_id: None, volume_heures: volume, statut: "pacte".into(), du: debut, au: fin },
            )
            .await
    }
}

/// Critère « plan versionné » : une version vivante par année, le contenu d'une version présentée ne change plus
/// (service ET base), la nouvelle version archive l'ancienne qui reste lisible, rien ne se supprime ; les faits de
/// procédure sont cohérents ; le plan se lit de tout le personnel une fois présenté, se tient à la direction seule.
#[tokio::test]
async fn a6_le_plan_est_versionne_et_une_version_presentee_ne_change_plus() {
    let x = cadre!();
    let aujourdhui = x.aujourdhui().await;
    let v1 = x.plan("Assurer la continuité sans perte d'heures").await;
    assert_eq!((v1.version, v1.etat.as_str()), (1, "brouillon"));
    // Un seul plan vivant par année : le deuxième se versionne, il ne se crée pas.
    let e = x.ent.creer_plan_rcd_for_actor(x.etab, x.dir, "direction", &PlanRcdBody::default()).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    // Droits : direction seule ; le professeur ne voit pas un brouillon, la direction oui.
    for (u, r) in [(x.sec, "administratif"), (x.prof, "prof"), (x.cpe, "cpe"), (x.compta, "compta")] {
        let e = x.ent.creer_plan_rcd_for_actor(x.etab, u, r, &PlanRcdBody::default()).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{r} : {e:?}");
    }
    assert!(x.ent.plan_courant_rcd_for_actor(x.etab, x.prof, "prof").await.unwrap().is_none(), "brouillon invisible du personnel");
    assert!(x.ent.plan_courant_rcd_for_actor(x.etab, x.dir, "direction").await.unwrap().is_some());
    // Brouillon modifiable ; mot exclu refusé (R-AP41) ; délai hors bornes refusé.
    let m = x
        .ent
        .modifier_plan_rcd_for_actor(
            x.etab,
            x.dir,
            "direction",
            v1.id,
            &PlanRcdBody { delai_prevenance_heures: Some(48), continuite_numerique: Some(true), ..Default::default() },
        )
        .await
        .unwrap();
    assert_eq!((m.delai_prevenance_heures, m.continuite_numerique), (48, true));
    let e = x
        .ent
        .modifier_plan_rcd_for_actor(x.etab, x.dir, "direction", v1.id, &PlanRcdBody { objectifs: Some("remplacer les malades".into()), ..Default::default() })
        .await
        .unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Invalide), "{e:?}");
    let e = x
        .ent
        .modifier_plan_rcd_for_actor(x.etab, x.dir, "direction", v1.id, &PlanRcdBody { delai_prevenance_heures: Some(200), ..Default::default() })
        .await
        .unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Invalide), "{e:?}");
    // Procédure : pas de transmission avant présentation, pas de date future, pas de désordre.
    let proc = |avis, pres, inst: Option<&str>, tr| ProcedurePlanRcdBody { avis_cp_le: avis, presente_le: pres, instance: inst.map(str::to_string), transmis_le: tr };
    for mauvais in [
        proc(None, None, None, Some(aujourdhui)),
        proc(Some(aujourdhui + Duration::days(1)), None, None, None),
        proc(Some(aujourdhui - Duration::days(1)), Some(aujourdhui - Duration::days(5)), Some("ca"), None),
        proc(None, None, None, None),
    ] {
        let e = x.ent.procedure_plan_rcd_for_actor(x.etab, x.dir, "direction", v1.id, &mauvais).await.unwrap_err();
        assert!(est_refus(&e, crate::service::RefusMetier::Invalide), "{mauvais:?} : {e:?}");
    }
    // L'avis seul laisse un brouillon ; avis + présentation + instance : présenté.
    let p = x.ent.procedure_plan_rcd_for_actor(x.etab, x.dir, "direction", v1.id, &proc(Some(aujourdhui - Duration::days(9)), None, None, None)).await.unwrap();
    assert_eq!(p.etat, "brouillon");
    let p = x
        .ent
        .procedure_plan_rcd_for_actor(x.etab, x.dir, "direction", v1.id, &proc(None, Some(aujourdhui - Duration::days(5)), Some("instance_etablissement"), None))
        .await
        .unwrap();
    assert_eq!(p.etat, "presente");
    // Présenté : le contenu ne change plus (service), ni en base (déclencheur), et la version ne se supprime pas.
    let e = x
        .ent
        .modifier_plan_rcd_for_actor(x.etab, x.dir, "direction", v1.id, &PlanRcdBody { objectifs: Some("autre".into()), ..Default::default() })
        .await
        .unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    let ecrit = sqlx::query("UPDATE ent_absp_rcd_plan SET objectifs = 'réécrit' WHERE etablissement_id = $1 AND id = $2")
        .bind(x.etab)
        .bind(v1.id)
        .execute(x.ent.pool())
        .await;
    assert!(ecrit.is_err(), "la base refuse de réécrire une version présentée");
    let suppr = sqlx::query("DELETE FROM ent_absp_rcd_plan WHERE etablissement_id = $1 AND id = $2").bind(x.etab).bind(v1.id).execute(x.ent.pool()).await;
    assert!(suppr.is_err(), "une version ne se supprime pas");
    let recule = sqlx::query("UPDATE ent_absp_rcd_plan SET etat = 'brouillon' WHERE etablissement_id = $1 AND id = $2").bind(x.etab).bind(v1.id).execute(x.ent.pool()).await;
    assert!(recule.is_err(), "l'état ne recule pas");
    // Le personnel lit le plan présenté (délai de prévenance), pas plus.
    let vu = x.ent.plan_courant_rcd_for_actor(x.etab, x.prof, "prof").await.unwrap().expect("plan présenté lisible");
    assert_eq!(vu.delai_prevenance_heures, 48);
    assert!(x.ent.plans_rcd_for_actor(x.etab, x.prof, "prof", None).await.is_err(), "l'historique des versions est à la direction");
    // Transmission au recteur : version transmise, puis plus aucune correction.
    let t = x.ent.procedure_plan_rcd_for_actor(x.etab, x.dir, "direction", v1.id, &proc(None, None, None, Some(aujourdhui - Duration::days(2)))).await.unwrap();
    assert_eq!(t.etat, "transmis");
    let e = x.ent.procedure_plan_rcd_for_actor(x.etab, x.dir, "direction", v1.id, &proc(Some(aujourdhui - Duration::days(30)), None, None, None)).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    // Campagne : une fois par version, un événement sans nom ni nature.
    let c = x.ent.lancer_campagne_rcd_for_actor(x.etab, x.dir, "direction", v1.id).await.unwrap();
    assert!(c.campagne_lancee_le.is_some());
    assert!(x.ent.lancer_campagne_rcd_for_actor(x.etab, x.dir, "direction", v1.id).await.is_err());
    assert_eq!(x.nb_events("absence_personnel.rcd_campagne_lancee").await, 1);
    // Nouvelle version : l'ancienne est archivée et reste lisible, la nouvelle repart en brouillon sans procédure.
    let v2 = x
        .ent
        .versionner_plan_rcd_for_actor(x.etab, x.dir, "direction", v1.id, &PlanRcdBody { delai_prevenance_heures: Some(12), ..Default::default() })
        .await
        .unwrap();
    assert_eq!((v2.version, v2.etat.as_str(), v2.delai_prevenance_heures), (2, "brouillon", 12));
    assert_eq!(v2.objectifs, v1.objectifs, "le contenu non modifié est repris");
    assert!(v2.avis_cp_le.is_none() && v2.presente_le.is_none() && v2.transmis_le.is_none() && v2.campagne_lancee_le.is_none());
    let versions = x.ent.plans_rcd_for_actor(x.etab, x.dir, "direction", None).await.unwrap();
    assert_eq!(versions.iter().map(|p| (p.version, p.etat.as_str())).collect::<Vec<_>>(), vec![(2, "brouillon"), (1, "archive")]);
    let ancienne = versions.iter().find(|p| p.version == 1).unwrap();
    assert_eq!((ancienne.delai_prevenance_heures, ancienne.transmis_le.is_some()), (48, true), "l'archive garde son contenu");
    let e = x.ent.versionner_plan_rcd_for_actor(x.etab, x.dir, "direction", v1.id, &PlanRcdBody::default()).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    let e = x.ent.versionner_plan_rcd_for_actor(x.etab, x.dir, "direction", v2.id, &PlanRcdBody::default()).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "un brouillon se modifie, il ne se versionne pas : {e:?}");
    // Journal : création, états, versionnement.
    let actions: Vec<String> = sqlx::query_scalar("SELECT action FROM ent_absp_journal WHERE etablissement_id = $1 AND objet = 'rcd_plan' ORDER BY le")
        .bind(x.etab)
        .fetch_all(x.ent.pool())
        .await
        .unwrap();
    for a in ["plan_cree", "plan_etat", "campagne_lancee", "plan_versionne"] {
        assert!(actions.iter().any(|j| j == a), "{a} non journalisé : {actions:?}");
    }
}

/// Critère « volontaires et compteurs » : engagement de soi-même, un seul actif par année, modification et fin à
/// date ; compteurs sollicité / accepté / refusé (motif légitime) / fait / restant tirés des demandes réelles
/// (A4) ; alerte au volume atteint sans blocage ; un agent ne voit que sa position, jamais les autres ; cloisonnement.
#[tokio::test]
async fn a6_volontaires_compteurs_et_position_sans_nommer_les_autres() {
    let x = cadre!();
    let ag2 = x.agent(x.prof2).await;
    let ag3 = x.agent(x.prof3).await;
    // Engagement : de soi-même, sans droit particulier ; statut et volume validés ; hors année refusé.
    let e2 = x.engager(x.prof2, "prof", 6.0).await.unwrap();
    assert_eq!((e2.agent_id, e2.statut.as_str(), e2.etat.as_str()), (ag2, "pacte", "actif"));
    let e = x.engager(x.prof2, "prof", 6.0).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "un seul engagement actif : {e:?}");
    let (_, debut, fin) = x.annee_courante().await;
    for mauvais in [
        EngagementRcdBody { agent_id: None, volume_heures: 0.0, statut: "pacte".into(), du: debut, au: fin },
        EngagementRcdBody { agent_id: None, volume_heures: 5.0, statut: "bénévole".into(), du: debut, au: fin },
        EngagementRcdBody { agent_id: None, volume_heures: 5.0, statut: "hse".into(), du: debut - Duration::days(400), au: fin },
        EngagementRcdBody { agent_id: None, volume_heures: 5.0, statut: "hse".into(), du: fin, au: debut },
    ] {
        let e = x.ent.engager_rcd_for_actor(x.etab, x.prof3, "prof", &mauvais).await.unwrap_err();
        assert!(est_refus(&e, crate::service::RefusMetier::Invalide), "{mauvais:?} : {e:?}");
    }
    // Engager un autre : droit `rcd.gerer` seulement.
    let pour_3 = EngagementRcdBody { agent_id: Some(ag3), volume_heures: 20.0, statut: "hse".into(), du: debut, au: fin };
    assert!(matches!(x.ent.engager_rcd_for_actor(x.etab, x.prof2, "prof", &pour_3).await.unwrap_err(), EntDbError::Forbidden));
    let e3 = x.ent.engager_rcd_for_actor(x.etab, x.dir, "direction", &pour_3).await.unwrap();
    assert_eq!(e3.agent_id, ag3);
    // Les demandes réelles : prof2 accepte une séance, prof3 en refuse une avec un motif légitime.
    let (_a, _c, impact) = x.absence_avec_seance(x.prof, &x.classe, (9, 0), (10, 30)).await;
    let s = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", impact, ag2).await.unwrap();
    x.ent.accepter_sollicitation_for_actor(x.etab, x.prof2, "prof", s.id).await.unwrap();
    let (_a2, _c2, impact2) = x.absence_avec_seance(x.prof, &x.classe, (11, 0), (12, 0)).await;
    let s3 = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", impact2, ag3).await.unwrap();
    x.ent
        .refuser_sollicitation_for_actor(x.etab, x.prof3, "prof", s3.id, Some("autre_service"))
        .await
        .unwrap();
    let v = x.ent.vivier_rcd_for_actor(x.etab, x.dir, "direction").await.unwrap();
    assert_eq!((v.volontaires, v.volume_engage), (2, 26.0));
    let c2 = v.compteurs.iter().find(|c| c.engagement.agent_id == ag2).unwrap();
    assert_eq!((c2.sollicitees, c2.acceptees, c2.refusees), (1, 1, 0));
    assert_eq!((c2.heures_acceptees, c2.heures_restantes, c2.volume_atteint), (1.5, 4.5, false));
    assert_eq!(c2.heures_faites, 0.0, "la séance est à venir : rien de fait");
    let c3 = v.compteurs.iter().find(|c| c.engagement.agent_id == ag3).unwrap();
    assert_eq!((c3.sollicitees, c3.acceptees, c3.refusees, c3.refusees_motif_legitime), (1, 0, 1, 1));
    assert_eq!(c3.heures_acceptees, 0.0);
    // Vivier : direction seule ; compteur personnel : sa position, jamais les autres.
    for (u, r) in [(x.sec, "administratif"), (x.prof2, "prof"), (x.cpe, "cpe")] {
        assert!(matches!(x.ent.vivier_rcd_for_actor(x.etab, u, r).await.unwrap_err(), EntDbError::Forbidden), "{r}");
    }
    let moi = x.ent.mon_compteur_rcd_for_actor(x.etab, x.prof2, "prof").await.unwrap().expect("mon compteur");
    assert_eq!((moi.acceptees, moi.position.as_str()), (1, "au_dessus"), "médiane de {{1,5 ; 0}} = 0,75");
    let json = serde_json::to_string(&moi).unwrap();
    assert!(!json.contains(&ag3.to_string()), "l'autre volontaire n'est pas nommé : {json}");
    let moi3 = x.ent.mon_compteur_rcd_for_actor(x.etab, x.prof3, "prof").await.unwrap().unwrap();
    assert_eq!(moi3.position, "en_dessous");
    assert!(x.ent.mon_compteur_rcd_for_actor(x.etab, x.cpe, "cpe").await.unwrap().is_none(), "pas d'engagement, pas de compteur");
    // Volume atteint : alerte, pas de blocage (l'accord vaut).
    let petit = x
        .ent
        .modifier_engagement_rcd_for_actor(x.etab, x.prof2, "prof", e2.id, &ModifierEngagementRcdBody { volume_heures: Some(1.0), statut: Some("hse".into()), ..Default::default() })
        .await
        .unwrap();
    assert_eq!((petit.volume_heures, petit.statut.as_str()), (1.0, "hse"));
    let v = x.ent.vivier_rcd_for_actor(x.etab, x.dir, "direction").await.unwrap();
    let c2 = v.compteurs.iter().find(|c| c.engagement.agent_id == ag2).unwrap();
    assert!(c2.volume_atteint && c2.heures_restantes == 0.0);
    // Pas de modification de l'engagement d'un autre (introuvable, pas d'oracle) ; la direction le peut.
    let e = x.ent.modifier_engagement_rcd_for_actor(x.etab, x.prof2, "prof", e3.id, &ModifierEngagementRcdBody { terminer: true, ..Default::default() }).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    // Fin : l'engagement est terminé, sort du vivier, et un nouvel engagement redevient possible.
    let fini = x
        .ent
        .modifier_engagement_rcd_for_actor(x.etab, x.dir, "direction", e3.id, &ModifierEngagementRcdBody { terminer: true, ..Default::default() })
        .await
        .unwrap();
    assert_eq!(fini.etat, "termine");
    assert!(fini.au <= x.aujourdhui().await, "la fin est ramenée à aujourd'hui");
    let v = x.ent.vivier_rcd_for_actor(x.etab, x.dir, "direction").await.unwrap();
    assert_eq!(v.volontaires, 1);
    let e = x.ent.modifier_engagement_rcd_for_actor(x.etab, x.dir, "direction", e3.id, &ModifierEngagementRcdBody { volume_heures: Some(3.0), ..Default::default() }).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "un engagement terminé ne se modifie plus : {e:?}");
    x.ent.engager_rcd_for_actor(x.etab, x.dir, "direction", &pour_3).await.expect("nouvel engagement après une fin");
    // Journal de l'engagement.
    let actions: Vec<String> = sqlx::query_scalar("SELECT action FROM ent_absp_journal WHERE etablissement_id = $1 AND objet = 'rcd_engagement'")
        .bind(x.etab)
        .fetch_all(x.ent.pool())
        .await
        .unwrap();
    for a in ["engagement_cree", "engagement_volume", "engagement_statut", "engagement_termine"] {
        assert!(actions.iter().any(|j| j == a), "{a} non journalisé : {actions:?}");
    }
    // Cloisonnement : un autre établissement ne voit rien de ces engagements.
    let autre = Cadre::nouveau().await.unwrap();
    let e = autre.ent.modifier_engagement_rcd_for_actor(autre.etab, autre.dir, "direction", e2.id, &ModifierEngagementRcdBody { terminer: true, ..Default::default() }).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    assert_eq!(autre.ent.vivier_rcd_for_actor(autre.etab, autre.dir, "direction").await.unwrap().volontaires, 0);
}

/// Critère « remontée sans nom » : la remontée n'a **aucun** identifiant de personne (ni en colonne, ni dans le DTO,
/// ni dans le CSV, ni dans l'événement), aucune nature fine (seulement la catégorie nationale) ; les heures sont
/// celles de l'EDT ; elle ne part qu'après contrôle, ne change plus une fois transmise, et reste cloisonnée.
#[tokio::test]
async fn a6_la_remontee_hebdomadaire_ne_porte_ni_nom_ni_nature_fine() {
    let x = cadre!();
    x.plan("Plan de test").await;
    let ag2 = x.agent(x.prof2).await;
    let ag_absent = x.agent(x.prof).await;
    // Une absence « convenance » remplacée par un collègue, une autre non traitée (mode « aucune »).
    let (abs, _c, impact) = x.absence_avec_seance(x.prof, &x.classe, (9, 0), (10, 0)).await;
    let s = x.ent.solliciter_for_actor(x.etab, x.dir, "direction", impact, ag2).await.unwrap();
    x.ent.accepter_sollicitation_for_actor(x.etab, x.prof2, "prof", s.id).await.unwrap();
    let c2 = x.creneau(&x.classe, x.prof, 1, (14, 0), (16, 0)).await;
    x.seance(c2, x.prof, 1, (14, 0), (16, 0), "prevue").await;
    x.ent.recalculer_impact_absence(x.etab, x.dir, abs).await.unwrap();
    // Droits : direction et secrétariat ; ni professeur, ni CPE, ni compta.
    let corps = PreparerRemonteeBody { semaine: Some(x.lundi) };
    for (u, r) in [(x.prof, "prof"), (x.cpe, "cpe"), (x.compta, "compta"), (x.accueil, "accueil")] {
        assert!(matches!(x.ent.preparer_remontee_rcd_for_actor(x.etab, u, r, &corps).await.unwrap_err(), EntDbError::Forbidden), "{r}");
    }
    // Un mardi n'est pas une semaine.
    let e = x.ent.preparer_remontee_rcd_for_actor(x.etab, x.sec, "administratif", &PreparerRemonteeBody { semaine: Some(x.lundi + Duration::days(1)) }).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Invalide), "{e:?}");
    let r = x.ent.preparer_remontee_rcd_for_actor(x.etab, x.sec, "administratif", &corps).await.unwrap();
    assert_eq!((r.etat.as_str(), r.semaine), ("preparee", x.lundi));
    let maths: Vec<&LigneRemonteeRcdDto> = r.lignes.iter().filter(|l| l.discipline_code == "mathématiques").collect();
    assert!(!maths.is_empty(), "{:?}", r.lignes);
    assert!(maths.iter().all(|l| l.niveau_code == "4EME" && l.categorie_nationale == "convenance_personnelle"), "{maths:?}");
    let remplace = maths.iter().find(|l| l.mode_continuite == "remplacement").expect("séance remplacée");
    assert_eq!((remplace.heures_absence, remplace.heures_remplacees), (1.0, 1.0));
    let sans = maths.iter().find(|l| l.mode_continuite == "aucune").expect("séance sans réponse");
    assert_eq!((sans.heures_absence, sans.heures_remplacees), (2.0, 0.0));
    let prevues: f64 = r.lignes.iter().map(|l| l.heures_prevues).sum();
    assert!(prevues >= 3.0, "les heures prévues de l'EDT sont comptées une fois : {prevues}");
    assert!(r.lignes.iter().all(|l| l.heures_prevues >= 0.0));
    // Aucune personne, aucune nature fine, nulle part.
    let json = serde_json::to_string(&r).unwrap();
    for interdit in [ag2.to_string(), ag_absent.to_string(), x.prof.to_string(), x.prof2.to_string(), abs.to_string(), "Nom prof".to_string(), "a15-prof".to_string(), "nature_code".to_string(), "\"convenance\"".to_string()] {
        assert!(!json.contains(&interdit), "« {interdit} » dans la remontée : {json}");
    }
    let colonnes: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns WHERE table_name IN ('ent_absp_rcd_remontee_ligne', 'ent_absp_rcd_remontee')",
    )
    .fetch_all(x.ent.pool())
    .await
    .unwrap();
    for c in &colonnes {
        // Seuls les opérateurs de la remontée (qui a contrôlé, qui a transmis) figurent à l'en-tête : jamais le personnel
        // concerné.
        if c.ends_with("_par_user_id") {
            continue;
        }
        for mot in ["agent", "user", "personne", "impact", "seance", "nature_", "nom", "email", "absence_id"] {
            assert!(!c.contains(mot), "colonne « {c} » : la remontée n'a aucun lien avec une personne ou une nature fine");
        }
    }
    // Contrôle avant départ : ni export ni transmission d'une remontée jamais relue.
    let e = x.ent.transmettre_remontee_rcd_for_actor(x.etab, x.sec, "administratif", r.id).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    let e = x.ent.exporter_remontee_rcd_csv_for_actor(x.etab, x.sec, "administratif", r.id).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    let c = x.ent.controler_remontee_rcd_for_actor(x.etab, x.sec, "administratif", r.id).await.unwrap();
    assert_eq!(c.etat, "controlee");
    // Recalcul : la remontée contrôlée revient à « préparée » (il faudra la relire).
    let r2 = x.ent.preparer_remontee_rcd_for_actor(x.etab, x.sec, "administratif", &corps).await.unwrap();
    assert_eq!((r2.id, r2.etat.as_str()), (r.id, "preparee"));
    assert_eq!(r2.lignes.len(), r.lignes.len(), "recalcul idempotent, pas de doublon de lignes");
    x.ent.controler_remontee_rcd_for_actor(x.etab, x.sec, "administratif", r.id).await.unwrap();
    // Tableau équivalent (Q-AP3) : CSV sans personne, tracé au journal.
    let csv = x.ent.exporter_remontee_rcd_csv_for_actor(x.etab, x.sec, "administratif", r.id).await.unwrap();
    assert!(csv.lines().count() == 1 + r.lignes.len(), "{csv}");
    for interdit in [ag2.to_string(), x.prof.to_string(), abs.to_string(), "@".to_string()] {
        assert!(!csv.contains(&interdit), "« {interdit} » dans le CSV : {csv}");
    }
    // Transmission : événement sans nom, puis plus aucune modification (service ET base).
    let t = x.ent.transmettre_remontee_rcd_for_actor(x.etab, x.sec, "administratif", r.id).await.unwrap();
    assert_eq!(t.etat, "transmise");
    assert!(t.transmise_le.is_some());
    let payload: String = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'absence_personnel.rcd_remontee_transmise'",
    )
    .bind(x.etab)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    for interdit in [ag2.to_string(), x.prof.to_string(), abs.to_string(), "convenance".to_string()] {
        assert!(!payload.contains(&interdit), "« {interdit} » dans l'événement : {payload}");
    }
    assert!(x.nb_events("absence_personnel.rcd_remontee_prete").await >= 2);
    let e = x.ent.preparer_remontee_rcd_for_actor(x.etab, x.sec, "administratif", &corps).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    let e = x.ent.rejeter_remontee_rcd_for_actor(x.etab, x.sec, "administratif", r.id).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    let suppr = sqlx::query("DELETE FROM ent_absp_rcd_remontee_ligne WHERE etablissement_id = $1 AND remontee_id = $2").bind(x.etab).bind(r.id).execute(x.ent.pool()).await;
    assert!(suppr.is_err(), "les lignes d'une remontée transmise ne changent plus");
    let modif = sqlx::query("UPDATE ent_absp_rcd_remontee SET etat = 'preparee', transmise_le = NULL WHERE etablissement_id = $1 AND id = $2").bind(x.etab).bind(r.id).execute(x.ent.pool()).await;
    assert!(modif.is_err(), "une remontée transmise ne change plus");
    // Compte rendu depuis les remontées contrôlées ou transmises : agrégats seulement.
    let plan = x.ent.plan_courant_rcd_for_actor(x.etab, x.dir, "direction").await.unwrap().unwrap();
    let cr = x
        .ent
        .produire_compte_rendu_rcd_for_actor(x.etab, x.dir, "direction", plan.id, &CompteRenduRcdBody { periode_du: x.lundi, periode_au: x.lundi + Duration::days(6) })
        .await
        .unwrap();
    assert_eq!(cr.remontees, 1);
    assert_eq!((cr.heures_absence, cr.heures_remplacees), (3.0, 1.0));
    let e = x.ent.produire_compte_rendu_rcd_for_actor(x.etab, x.dir, "direction", plan.id, &CompteRenduRcdBody { periode_du: x.lundi + Duration::days(30), periode_au: x.lundi + Duration::days(36) }).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "pas de compte rendu sans remontée : {e:?}");
    let ecrit = sqlx::query("UPDATE ent_absp_rcd_compte_rendu SET heures_absence = 0 WHERE etablissement_id = $1").bind(x.etab).execute(x.ent.pool()).await;
    assert!(ecrit.is_err(), "un compte rendu est en ajout seul");
    assert_eq!(x.ent.plan_courant_rcd_for_actor(x.etab, x.dir, "direction").await.unwrap().unwrap().comptes_rendus, 1);
    // Cloisonnement : une autre école ne lit pas cette remontée.
    let autre = Cadre::nouveau().await.unwrap();
    let e = autre.ent.controler_remontee_rcd_for_actor(autre.etab, autre.sec, "administratif", r.id).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    assert!(autre.ent.remontees_rcd_for_actor(autre.etab, autre.sec, "administratif").await.unwrap().is_empty());
}

// ═══════════════════════════════════════════════ LOT A7 : HEURES ET ÉTATS ═══════════════════════════════════════════════
// Critères de fin (§28.1) : heure constatée seulement sur une séance assurée (R-AP28) ; régularisation après export ;
// unicité des éléments de paie.

impl Cadre {
    /// Premier jour du mois situé `n` mois avant celui du jour de l'établissement.
    async fn mois_avant(&self, n: u32) -> NaiveDate {
        use chrono::Datelike;
        let j = self.aujourdhui().await;
        let total = j.year() * 12 + j.month0() as i32 - n as i32;
        NaiveDate::from_ymd_opt(total.div_euclid(12), (total.rem_euclid(12) + 1) as u32, 1).unwrap()
    }

    /// Instant postérieur à toute séance du cadre (la séance est à J+14 ou plus) : le test ne dépend pas de l'heure
    /// où il est lancé.
    fn apres_les_seances() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now() + Duration::days(400)
    }

    /// Une séance de `prof` (absent) remplacée par `remplacant` (acceptation d'une sollicitation de l'A4).
    async fn seance_remplacee(&self, prof: Uuid, remplacant: Uuid, d: (u32, u32), f: (u32, u32)) -> (Uuid, Uuid, Uuid) {
        let (abs, c, impact) = self.absence_avec_seance(prof, &self.classe, d, f).await;
        let ag = self.agent(remplacant).await;
        let s = self.ent.solliciter_for_actor(self.etab, self.dir, "direction", impact, ag).await.unwrap();
        self.ent.accepter_sollicitation_for_actor(self.etab, remplacant, "prof", s.id).await.unwrap();
        let seance: Uuid = sqlx::query_scalar("SELECT seance_id FROM ent_absp_impact WHERE etablissement_id = $1 AND id = $2")
            .bind(self.etab)
            .bind(impact)
            .fetch_one(self.ent.pool())
            .await
            .unwrap();
        (abs, c, seance)
    }

    async fn appel_valide(&self, seance: Uuid, auteur: Uuid) {
        sqlx::query(
            "INSERT INTO ent_appel (id, etablissement_id, classe, jour, auteur_user_id, statut, seance_id, etat)
             SELECT $1, $2, sc.classe, sc.jour, $4, 'clos', sc.id, 'valide' FROM ent_edt_seance sc
              WHERE sc.etablissement_id = $2 AND sc.id = $3",
        )
        .bind(Uuid::new_v4())
        .bind(self.etab)
        .bind(seance)
        .bind(auteur)
        .execute(self.ent.pool())
        .await
        .expect("appel validé");
    }

    async fn heures_de(&self, agent: Uuid) -> Vec<(String, i32, String, String)> {
        sqlx::query_as("SELECT nature, duree_minutes, preuve, etat FROM ent_absp_heure WHERE etablissement_id = $1 AND agent_id = $2 ORDER BY cree_le")
            .bind(self.etab)
            .bind(agent)
            .fetch_all(self.ent.pool())
            .await
            .unwrap()
    }

    async fn saisie(&self, agent: Uuid, nature: &str, minutes: i32, le: NaiveDate) -> HeureDto {
        self.ent
            .saisir_heure_for_actor(self.etab, self.dir, "direction", &SaisirHeureBody { agent_id: agent, nature: nature.into(), duree_minutes: minutes, le })
            .await
            .unwrap_or_else(|e| panic!("saisie refusée : {e:?}"))
    }
}

/// R-AP28 : une heure constatée naît d'un FAIT. Jamais d'une séance à venir, d'une séance non remplacée, de
/// l'attestation d'un tiers, d'une saisie de remplacement — ni en base. L'appel fait (socle 12) la constate sans geste ;
/// sans appel, le remplaçant l'atteste (CAP-56) ; rejouer ne double rien ; HSE sur une convenance : alerte sans blocage.
#[tokio::test]
async fn a7_r_ap28_une_heure_n_est_constatee_que_sur_une_seance_assuree() {
    let x = cadre!();
    let ag2 = x.agent(x.prof2).await;
    let (_a, _c, seance) = x.seance_remplacee(x.prof, x.prof2, (9, 0), (10, 30)).await;
    let apres = Cadre::apres_les_seances();
    let attester = |u: Uuid, r: &'static str, s: Uuid, m| {
        let ent = &x.ent;
        let etab = x.etab;
        async move { ent.attester_seance_a(etab, u, r, &AttesterBody { seance_id: s }, m).await }
    };
    // 1. Séance à venir : personne ne l'atteste.
    let e = attester(x.prof2, "prof", seance, chrono::Utc::now()).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "séance non terminée : {e:?}");
    assert!(x.heures_de(ag2).await.is_empty());
    // 2. Séance terminée, mais attestation d'un tiers ou de l'absent : introuvable (pas d'oracle).
    for (u, r) in [(x.prof3, "prof"), (x.prof, "prof"), (x.compta, "compta")] {
        let e = attester(u, r, seance, apres).await.unwrap_err();
        assert!(matches!(e, EntDbError::Domain(DomainError::NotFound) | EntDbError::Forbidden), "{r} : {e:?}");
    }
    assert!(x.heures_de(ag2).await.is_empty(), "aucune heure sans fait");
    // 3. Le remplaçant atteste : heure constatée (RCD, 90 minutes), preuve « attestation », une seule fois.
    let h = attester(x.prof2, "prof", seance, apres).await.unwrap();
    assert_eq!((h.nature.as_str(), h.duree_minutes, h.preuve.as_str(), h.etat.as_str()), ("rcd", 90, "attestation", "constatee"));
    assert_eq!(h.agent_id, ag2);
    let encore = attester(x.prof2, "prof", seance, apres).await.unwrap();
    assert_eq!(encore.id, h.id, "rejouer ne crée pas de doublon");
    assert_eq!(x.heures_de(ag2).await.len(), 1);
    assert_eq!(x.nb_events("absence_personnel.heure_constatee").await, 1);
    // 4. Séance non remplacée : rien à attester, même par la direction.
    let (_a2, _c2, impact2) = x.absence_avec_seance(x.prof, &x.classe, (11, 0), (12, 0)).await;
    let seance2: Uuid = sqlx::query_scalar("SELECT seance_id FROM ent_absp_impact WHERE etablissement_id = $1 AND id = $2")
        .bind(x.etab)
        .bind(impact2)
        .fetch_one(x.ent.pool())
        .await
        .unwrap();
    let e = attester(x.dir, "direction", seance2, apres).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    // 5. La base refuse le contournement : pas de remplacement saisi, pas de preuve sans séance, pas de séance sans preuve.
    let abs = x.agent(x.prof3).await;
    for (nature, preuve, seance_id) in [("rcd", "saisie", None), ("hse", "attestation", None), ("hse", "saisie", Some(seance)), ("rcd", "appel", None)] {
        let r = sqlx::query(
            "INSERT INTO ent_absp_heure (id, etablissement_id, agent_id, seance_id, nature, duree_minutes, le, preuve, saisie_par_user_id)
             VALUES ($1,$2,$3,$4,$5,60,$6,$7,$8)",
        )
        .bind(Uuid::new_v4())
        .bind(x.etab)
        .bind(abs)
        .bind(seance_id)
        .bind(nature)
        .bind(x.aujourdhui().await)
        .bind(preuve)
        .bind(x.dir)
        .execute(x.ent.pool())
        .await;
        assert!(r.is_err(), "la base accepte ({nature}, {preuve}, séance {seance_id:?}) : R-AP28 violée");
    }
    // 6. Par le service : une saisie de remplacement, future ou pour soi-même est refusée.
    let today = x.aujourdhui().await;
    for (nature, le) in [("rcd", today), ("rattrapage_du", today), ("hse", today + Duration::days(3))] {
        let e = x
            .ent
            .saisir_heure_for_actor(x.etab, x.dir, "direction", &SaisirHeureBody { agent_id: ag2, nature: nature.into(), duree_minutes: 60, le })
            .await
            .unwrap_err();
        assert!(est_refus(&e, crate::service::RefusMetier::Invalide), "{nature} {le} : {e:?}");
    }
    let moi = x.agent(x.dir).await;
    let e = x.ent.saisir_heure_for_actor(x.etab, x.dir, "direction", &SaisirHeureBody { agent_id: moi, nature: "hse".into(), duree_minutes: 60, le: today }).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Interdit), "{e:?}");
    let e = x.ent.saisir_heure_for_actor(x.etab, x.prof, "prof", &SaisirHeureBody { agent_id: ag2, nature: "hse".into(), duree_minutes: 60, le: today }).await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    // 7. L'appel fait (socle 12) constate sans geste, une seule fois ; ce qui manque disparaît de « mes heures ».
    let (_a3, _c3, seance3) = x.seance_remplacee(x.prof, x.prof2, (14, 0), (15, 0)).await;
    let mes = x.ent.mes_heures_a(x.etab, x.prof2, "prof", None, apres).await.unwrap();
    assert_eq!(mes.a_attester.len(), 1, "séance terminée sans appel : à attester ({:?})", mes.a_attester);
    assert_eq!(mes.a_attester[0].seance_id, seance3);
    assert_eq!(mes.heures.len(), 1);
    x.ent.constater_heures_assurees_a(apres).await.unwrap();
    assert_eq!(x.heures_de(ag2).await.len(), 1, "sans appel, la tâche ne constate rien");
    x.appel_valide(seance3, x.prof2).await;
    x.ent.constater_heures_assurees_a(apres).await.unwrap();
    let hs = x.heures_de(ag2).await;
    assert_eq!(hs.len(), 2, "{hs:?}");
    assert_eq!((hs[1].1, hs[1].2.as_str()), (60, "appel"));
    x.ent.constater_heures_assurees_a(apres).await.unwrap();
    assert_eq!(x.heures_de(ag2).await.len(), 2, "la tâche est idempotente");
    let mes = x.ent.mes_heures_a(x.etab, x.prof2, "prof", None, apres).await.unwrap();
    assert!(mes.a_attester.is_empty());
    assert_eq!(mes.heures.len(), 2);
    // L'agent ne lit que les siennes.
    assert!(x.ent.mes_heures_a(x.etab, x.prof3, "prof", None, apres).await.unwrap().heures.is_empty());
    // 8. R-AP33 : HSE (engagement « HSE ») pour remplacer une absence de convenance → alerte, sans blocage et sans nature.
    x.ent
        .engager_rcd_for_actor(
            x.etab,
            x.prof3,
            "prof",
            &{
                let (_, debut, fin) = x.annee_courante().await;
                EngagementRcdBody { agent_id: None, volume_heures: 10.0, statut: "hse".into(), du: debut, au: fin }
            },
        )
        .await
        .unwrap();
    let (_a4, _c4, seance4) = x.seance_remplacee(x.prof, x.prof3, (16, 0), (17, 0)).await;
    let h = x.ent.attester_seance_a(x.etab, x.prof3, "prof", &AttesterBody { seance_id: seance4 }, apres).await.unwrap();
    assert_eq!(h.nature, "hse", "l'engagement HSE donne une heure HSE (R-04)");
    assert_eq!(x.nb_events("absence_personnel.heure_a_verifier").await, 1, "alerte R-AP33");
    let payload: String = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'absence_personnel.heure_a_verifier'",
    )
    .bind(x.etab)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert!(!payload.contains("convenance") && !payload.contains("nature"), "{payload}");
    // Cloisonnement d'établissement.
    let autre = Cadre::nouveau().await.unwrap();
    let e = autre.ent.contester_heure_for_actor(autre.etab, autre.dir, "direction", h.id, &ContesterHeureBody { raison_code: "doublon".into(), note: None }).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
}

/// Cycle d'une heure et de l'état mensuel : contestation, correction, validation (jamais ses propres heures), figeure
/// après le mois seulement et sans heure en suspens, signature par la direction, export par le bon employeur, heure
/// figée immuable ; **régularisation après export** : l'heure garde sa durée exportée, une ligne signée est reportée une
/// seule fois sur l'état suivant, rattachée à la ligne d'origine.
#[tokio::test]
async fn a7_cycle_des_heures_etat_mensuel_et_regularisation_apres_export() {
    let x = cadre!();
    let (ag2, ag3) = (x.agent(x.prof2).await, x.agent(x.prof3).await);
    let (m2, m1) = (x.mois_avant(2).await, x.mois_avant(1).await);
    let h1 = x.saisie(ag2, "mission_pacte", 120, m2 + Duration::days(3)).await;
    let h1b = x.saisie(ag3, "hse", 60, m2 + Duration::days(4)).await;
    assert_eq!((h1.etat.as_str(), h1.preuve.as_str(), h1.employeur.as_str(), h1.mois), ("constatee", "saisie", "etat", m2));
    // Contestation par l'agent (motif codé, mot exclu refusé) ; par un tiers : introuvable.
    let e = x.ent.contester_heure_for_actor(x.etab, x.prof2, "prof", h1.id, &ContesterHeureBody { raison_code: "autre".into(), note: Some("j'étais malade".into()) }).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Invalide), "{e:?}");
    let e = x.ent.contester_heure_for_actor(x.etab, x.prof2, "prof", h1.id, &ContesterHeureBody { raison_code: "inconnu".into(), note: None }).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Invalide), "{e:?}");
    let e = x.ent.contester_heure_for_actor(x.etab, x.prof3, "prof", h1.id, &ContesterHeureBody { raison_code: "doublon".into(), note: None }).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound)), "{e:?}");
    let c = x.ent.contester_heure_for_actor(x.etab, x.prof2, "prof", h1.id, &ContesterHeureBody { raison_code: "duree_erronee".into(), note: Some("1 h 30 et non 2 h".into()) }).await.unwrap();
    assert_eq!(c.etat, "contestee");
    // Validation : les contestées restent ; la figeure est refusée tant que tout n'est pas réglé.
    let b = x.ent.valider_heures_for_actor(x.etab, x.dir, "direction", &ValiderHeuresBody { mois: m2, agent_id: None }).await.unwrap();
    assert_eq!((b.validees, b.contestees_restantes), (1, 1));
    let figer = |employeur: &'static str, mois| {
        let ent = &x.ent;
        let (etab, dir) = (x.etab, x.dir);
        async move { ent.figer_etat_for_actor(etab, dir, "direction", &FigerEtatBody { mois, employeur: employeur.into() }).await }
    };
    let e = figer("etat", m2).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "heure contestée : {e:?}");
    // Correction par la direction (1 h 30) : l'heure est validée ; pas de correction de ses propres heures.
    let moi = x.agent(x.dir).await;
    let propre: Uuid = sqlx::query_scalar(
        "INSERT INTO ent_absp_heure (id, etablissement_id, agent_id, nature, duree_minutes, le, preuve, saisie_par_user_id)
         VALUES ($1,$2,$3,'hse',30,$4,'saisie',$5) RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(x.etab)
    .bind(moi)
    .bind(m2 + Duration::days(5))
    .bind(x.sec)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    let e = x.ent.corriger_heure_for_actor(x.etab, x.dir, "direction", propre, &CorrigerHeureBody { duree_minutes: 45, raison_code: "autre".into(), note: None }).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Interdit), "{e:?}");
    let b = x.ent.valider_heures_for_actor(x.etab, x.dir, "direction", &ValiderHeuresBody { mois: m2, agent_id: None }).await.unwrap();
    assert_eq!((b.validees, b.laissees_a_un_autre), (0, 1), "la direction ne valide pas ses propres heures");
    let k = x.ent.corriger_heure_for_actor(x.etab, x.dir, "direction", h1.id, &CorrigerHeureBody { duree_minutes: 90, raison_code: "duree_erronee".into(), note: None }).await.unwrap();
    assert_eq!((k.duree_minutes, k.etat.as_str()), (90, "validee"));
    // Les heures du valideur sont validées par le secrétariat… qui n'a pas le droit : la direction attend un autre.
    let e = x.ent.valider_heures_for_actor(x.etab, x.sec, "administratif", &ValiderHeuresBody { mois: m2, agent_id: None }).await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    sqlx::query("UPDATE ent_absp_heure SET etat = 'validee' WHERE etablissement_id = $1 AND id = $2").bind(x.etab).bind(propre).execute(x.ent.pool()).await.unwrap();
    // Cloisonnement par employeur : l'OGEC ne voit pas l'État.
    assert!(matches!(
        x.ent.heures_for_actor(x.etab, x.compta, "compta", &FiltreHeures::default()).await.unwrap_err(),
        EntDbError::Forbidden
    ));
    assert_eq!(x.ent.heures_for_actor(x.etab, x.sec, "administratif", &FiltreHeures { mois: Some(m2), ..Default::default() }).await.unwrap().len(), 3);
    // Figeure : refusée pour le mois courant, pour l'employeur d'un autre, accordée au secrétariat de l'État.
    let e = figer("etat", x.mois_avant(0).await).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "le mois n'est pas terminé : {e:?}");
    let e = x.ent.figer_etat_for_actor(x.etab, x.sec, "administratif", &FigerEtatBody { mois: m2, employeur: "ogec".into() }).await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    let e = x.ent.figer_etat_for_actor(x.etab, x.compta, "compta", &FigerEtatBody { mois: m2, employeur: "etat".into() }).await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    let e1 = x.ent.figer_etat_for_actor(x.etab, x.sec, "administratif", &FigerEtatBody { mois: m2, employeur: "etat".into() }).await.unwrap();
    assert_eq!(e1.etat, "fige");
    assert_eq!(e1.lignes.len(), 3);
    assert_eq!(e1.total_minutes, 90 + 60 + 30);
    let e = figer("etat", m2).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "un seul état par mois et employeur : {e:?}");
    // Heure figée : plus de contestation, de correction ni de réécriture (service et base).
    let e = x.ent.contester_heure_for_actor(x.etab, x.prof2, "prof", h1.id, &ContesterHeureBody { raison_code: "doublon".into(), note: None }).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    let e = x.ent.corriger_heure_for_actor(x.etab, x.dir, "direction", h1.id, &CorrigerHeureBody { duree_minutes: 30, raison_code: "autre".into(), note: None }).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    for sql in [
        "UPDATE ent_absp_heure SET duree_minutes = 10 WHERE etablissement_id = $1",
        "DELETE FROM ent_absp_heure WHERE etablissement_id = $1",
        "UPDATE ent_absp_etat_ligne SET duree_minutes = 1 WHERE etablissement_id = $1",
        "DELETE FROM ent_absp_etat_ligne WHERE etablissement_id = $1",
        "DELETE FROM ent_absp_etat WHERE etablissement_id = $1",
    ] {
        assert!(sqlx::query(sql).bind(x.etab).execute(x.ent.pool()).await.is_err(), "la base accepte : {sql}");
    }
    // Signature : la direction seule ; export : après signature, par l'employeur concerné.
    let e = x.ent.exporter_etat_csv_for_actor(x.etab, x.sec, "administratif", e1.id).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "non signé : {e:?}");
    let e = x.ent.signer_etat_for_actor(x.etab, x.sec, "administratif", e1.id).await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    let s = x.ent.signer_etat_for_actor(x.etab, x.dir, "direction", e1.id).await.unwrap();
    assert_eq!(s.etat, "signe");
    assert!(x.ent.signer_etat_for_actor(x.etab, x.dir, "direction", e1.id).await.is_err(), "se signe une fois");
    let e = x.ent.exporter_etat_csv_for_actor(x.etab, x.compta, "compta", e1.id).await.unwrap_err();
    assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
    let csv = x.ent.exporter_etat_csv_for_actor(x.etab, x.sec, "administratif", e1.id).await.unwrap();
    assert_eq!(csv.lines().count(), 1 + 3, "{csv}");
    assert!(csv.contains(",mission_pacte,90,non"), "{csv}");
    let csv2 = x.ent.exporter_etat_csv_for_actor(x.etab, x.sec, "administratif", e1.id).await.unwrap();
    assert_eq!(csv, csv2, "export rejouable");
    let etats: Vec<String> = sqlx::query_scalar("SELECT DISTINCT etat FROM ent_absp_heure WHERE etablissement_id = $1 AND etat_id = $2")
        .bind(x.etab)
        .bind(e1.id)
        .fetch_all(x.ent.pool())
        .await
        .unwrap();
    assert_eq!(etats, vec!["exportee"]);
    assert!(x.nb_events("absence_personnel.etat_heures_exporte").await == 1, "un seul événement d'export");
    // RÉGULARISATION après export (ABP-208) : jamais une réécriture. Pas de régularisation de ses propres heures, ni
    // d'une heure non figée, ni sans changement.
    let e = x.ent.regulariser_heure_for_actor(x.etab, x.dir, "direction", propre, &CorrigerHeureBody { duree_minutes: 10, raison_code: "autre".into(), note: None }).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Interdit), "{e:?}");
    let e = x.ent.regulariser_heure_for_actor(x.etab, x.dir, "direction", h1.id, &CorrigerHeureBody { duree_minutes: 90, raison_code: "autre".into(), note: None }).await.unwrap_err();
    assert!(est_refus(&e, crate::service::RefusMetier::Conflit), "{e:?}");
    let e = x.ent.regulariser_heure_for_actor(x.etab, x.prof2, "prof", h1.id, &CorrigerHeureBody { duree_minutes: 60, raison_code: "autre".into(), note: None }).await.unwrap_err();
    assert!(matches!(e, EntDbError::Domain(DomainError::NotFound) | EntDbError::Forbidden), "{e:?}");
    let r = x.ent.regulariser_heure_for_actor(x.etab, x.dir, "direction", h1.id, &CorrigerHeureBody { duree_minutes: 60, raison_code: "duree_erronee".into(), note: None }).await.unwrap();
    assert_eq!((r.duree_avant, r.duree_apres, r.delta_minutes), (90, 60, -30));
    let mes = x.ent.mes_heures_a(x.etab, x.prof2, "prof", Some(m2), Cadre::apres_les_seances()).await.unwrap();
    assert_eq!((mes.heures[0].duree_minutes, mes.heures[0].regularisation_minutes), (90, -30), "l'heure garde sa durée exportée");
    let regul_avant = x.compter("SELECT count(*) FROM ent_absp_etat_ligne WHERE etablissement_id = $1 AND regularisation_de_id IS NOT NULL").await;
    assert_eq!(regul_avant, 0, "rien n'est écrit sur l'état déjà exporté");
    // L'état SUIVANT porte la régularisation, une seule fois, rattachée à la ligne d'origine.
    x.saisie(ag3, "vacation", 45, m1 + Duration::days(2)).await;
    x.ent.valider_heures_for_actor(x.etab, x.dir, "direction", &ValiderHeuresBody { mois: m1, agent_id: None }).await.unwrap();
    let e2 = x.ent.figer_etat_for_actor(x.etab, x.sec, "administratif", &FigerEtatBody { mois: m1, employeur: "etat".into() }).await.unwrap();
    assert_eq!(e2.lignes.len(), 2, "{:?}", e2.lignes);
    let l_regul = e2.lignes.iter().find(|l| l.regularisation_de_id.is_some()).expect("ligne de régularisation");
    let l_origine = e1.lignes.iter().find(|l| l.heure_id == Some(h1.id)).unwrap();
    assert_eq!((l_regul.duree_minutes, l_regul.regularisation_de_id, l_regul.heure_id), (-30, Some(l_origine.id), Some(h1.id)));
    assert_eq!(e2.total_minutes, 45 - 30);
    assert_eq!(x.compter("SELECT count(*) FROM ent_absp_etat_ligne WHERE etablissement_id = $1 AND regularisation_de_id IS NOT NULL").await, 1);
    // Un fait de régularisation ne se reporte qu'une fois, même par la base.
    let doublon = sqlx::query(
        "INSERT INTO ent_absp_etat_ligne (id, etablissement_id, etat_id, agent_id, nature, duree_minutes, heure_id, regularisation_de_id, heure_fait_id)
         SELECT gen_random_uuid(), etablissement_id, etat_id, agent_id, nature, duree_minutes, heure_id, regularisation_de_id, heure_fait_id
           FROM ent_absp_etat_ligne WHERE etablissement_id = $1 AND regularisation_de_id IS NOT NULL",
    )
    .bind(x.etab)
    .execute(x.ent.pool())
    .await;
    assert!(doublon.is_err(), "la base laisse reporter deux fois la même régularisation");
    // Mes heures sur l'année : durée effective (régularisation comprise). Jour courant : toujours dans l'année.
    let aujourdhui = x.aujourdhui().await;
    x.saisie(ag2, "mission_pacte", 30, aujourdhui).await;
    let an = x.ent.etat_annuel_agent_for_actor(x.etab, x.prof2, "prof", None, None).await.unwrap();
    assert_eq!(an.agent_id, ag2);
    assert!(an.par_mois.iter().any(|l| l.mois == premier_du_mois_test(aujourdhui) && l.minutes == 30));
    assert!(x.ent.etat_annuel_agent_for_actor(x.etab, x.prof3, "prof", Some(ag2), None).await.is_err(), "pas les heures d'un autre");
    assert!(x.ent.etat_annuel_agent_for_actor(x.etab, x.sec, "administratif", Some(ag2), None).await.is_ok());
}

fn premier_du_mois_test(d: NaiveDate) -> NaiveDate {
    crate::absences_personnels::heures::premier_du_mois(d)
}

/// Éléments de paie de l'OGEC : un par (absence ou heure, mois, nature) — unicité EN BASE et préparation rejouable,
/// même en concurrence ; nature codée ; un mois envoyé reçoit une régularisation sur le mois ouvert, jamais une
/// réécriture ; file DSN à J+5 avec « signalé le ».
#[tokio::test]
async fn a7_elements_de_paie_uniques_regularisation_et_dsn() {
    let x = cadre!();
    let ag = x.agent(x.prof).await;
    x.engagement_ogec(ag).await;
    let m1 = x.mois_avant(1).await;
    // Une heure de l'OGEC (engagement antérieur au jour) : mission Pacte, 2 h, validée.
    let h = x.saisie(ag, "mission_pacte", 120, m1 + Duration::days(4)).await;
    assert_eq!(h.employeur, "ogec", "seul l'engagement de l'OGEC couvre ce jour");
    x.ent.valider_heures_for_actor(x.etab, x.dir, "direction", &ValiderHeuresBody { mois: m1, agent_id: None }).await.unwrap();
    // Une absence « convenance » de l'OGEC, un jour ouvré du mois passé.
    let jour_ouvre = {
        use chrono::Datelike;
        let mut j = m1 + Duration::days(9);
        while j.weekday().number_from_monday() > 5 {
            j += Duration::days(1);
        }
        j
    };
    let d = x
        .ent
        .declarer_absence_for_actor(
            x.etab,
            x.prof,
            "prof",
            &DeclarerAbsenceBody { nature_code: Some("convenance".into()), debut: jour_ouvre, fin: jour_ouvre, ..Default::default() },
        )
        .await
        .unwrap();
    let _ = d;
    let ogec_id: Uuid = sqlx::query_scalar("SELECT id FROM ent_absp_absence WHERE etablissement_id = $1 AND employeur = 'ogec'")
        .bind(x.etab)
        .fetch_one(x.ent.pool())
        .await
        .expect("absence OGEC");
    x.valider(ogec_id).await;
    // Droits : comptabilité de l'OGEC et direction ; ni le secrétariat de l'État, ni un professeur.
    for (u, r) in [(x.sec, "administratif"), (x.prof, "prof"), (x.cpe, "cpe")] {
        let e = x.ent.preparer_elements_paie_for_actor(x.etab, u, r, &PreparerPaieBody { mois: m1 }).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{r} : {e:?}");
    }
    let b = x.ent.preparer_elements_paie_for_actor(x.etab, x.compta, "compta", &PreparerPaieBody { mois: m1 }).await.unwrap();
    assert_eq!(b.crees, 2, "une heure et une absence : {b:?}");
    let els = x.ent.elements_paie_for_actor(x.etab, x.compta, "compta", m1, None).await.unwrap();
    assert_eq!(els.len(), 2);
    let e_h = els.iter().find(|e| e.heure_id.is_some()).unwrap();
    let e_a = els.iter().find(|e| e.absence_id.is_some()).unwrap();
    assert_eq!((e_h.nature_code.as_str(), e_h.quantite, e_h.unite.as_str()), ("mission_pacte", 2.0, "heure"));
    assert_eq!((e_a.nature_code.as_str(), e_a.quantite, e_a.unite.as_str()), ("convenance", 1.0, "jour"));
    // Unicité : rejouer ne crée rien ; deux préparations simultanées non plus ; la base refuse le doublon.
    let corps_paie = PreparerPaieBody { mois: m1 };
    let (r1, r2) = tokio::join!(
        x.ent.preparer_elements_paie_for_actor(x.etab, x.compta, "compta", &corps_paie),
        x.ent.preparer_elements_paie_for_actor(x.etab, x.dir, "direction", &corps_paie),
    );
    assert_eq!(r1.unwrap().crees + r2.unwrap().crees, 0);
    assert_eq!(x.compter("SELECT count(*) FROM ent_absp_element_paie WHERE etablissement_id = $1").await, 2);
    let doublon = sqlx::query(
        "INSERT INTO ent_absp_element_paie (id, etablissement_id, agent_id, absence_id, mois, nature_code, quantite, unite)
         SELECT gen_random_uuid(), etablissement_id, agent_id, absence_id, mois, nature_code, 3, 'jour'
           FROM ent_absp_element_paie WHERE etablissement_id = $1 AND absence_id IS NOT NULL",
    )
    .bind(x.etab)
    .execute(x.ent.pool())
    .await;
    assert!(doublon.is_err(), "la base laisse deux éléments pour la même (absence, mois, nature) : R-AP29 violée");
    // Export : « préparé » → « envoyé », rejouable, nature codée sans libellé ni identifiant d'absence.
    let csv = x.ent.exporter_elements_paie_csv_for_actor(x.etab, x.compta, "compta", m1).await.unwrap();
    assert_eq!(csv.lines().count(), 3, "{csv}");
    assert!(csv.contains(",convenance,1.00,jour,non") && csv.contains(",mission_pacte,2.00,heure,non"), "{csv}");
    assert!(!csv.contains(&ogec_id.to_string()), "l'identifiant d'absence ne sort pas : {csv}");
    assert_eq!(x.ent.exporter_elements_paie_csv_for_actor(x.etab, x.dir, "direction", m1).await.unwrap(), csv, "export rejouable");
    assert_eq!(x.compter("SELECT count(*) FROM ent_absp_element_paie WHERE etablissement_id = $1 AND etat = 'envoye'").await, 2);
    // Un élément envoyé ne se réécrit pas (base).
    assert!(sqlx::query("UPDATE ent_absp_element_paie SET quantite = 9 WHERE etablissement_id = $1").bind(x.etab).execute(x.ent.pool()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_absp_element_paie WHERE etablissement_id = $1").bind(x.etab).execute(x.ent.pool()).await.is_err());
    // L'heure est figée puis régularisée (2 h → 1 h 30) : le mois ouvert reçoit une régularisation de −0,50, l'élément
    // envoyé reste intact et passe « régularisé » ; rejouer ne la double pas.
    let m0 = x.mois_avant(0).await;
    let etat = x.ent.figer_etat_for_actor(x.etab, x.compta, "compta", &FigerEtatBody { mois: m1, employeur: "ogec".into() }).await.unwrap();
    assert_eq!(etat.lignes.len(), 1);
    x.ent.regulariser_heure_for_actor(x.etab, x.dir, "direction", h.id, &CorrigerHeureBody { duree_minutes: 90, raison_code: "duree_erronee".into(), note: None }).await.unwrap();
    let b = x.ent.preparer_elements_paie_for_actor(x.etab, x.compta, "compta", &PreparerPaieBody { mois: m0 }).await.unwrap();
    assert_eq!((b.crees, b.regularisations), (0, 1), "{b:?}");
    let ouvert = x.ent.elements_paie_for_actor(x.etab, x.compta, "compta", m0, None).await.unwrap();
    assert_eq!(ouvert.len(), 1);
    assert_eq!((ouvert[0].nature_code.as_str(), ouvert[0].quantite, ouvert[0].regularise_de_id), ("regul_mission_pacte", -0.5, Some(e_h.id)));
    let ancien = x.ent.elements_paie_for_actor(x.etab, x.compta, "compta", m1, Some(ag)).await.unwrap();
    assert_eq!(ancien.iter().find(|e| e.id == e_h.id).map(|e| (e.quantite, e.etat.as_str())), Some((2.0, "regularise")));
    let b = x.ent.preparer_elements_paie_for_actor(x.etab, x.compta, "compta", &PreparerPaieBody { mois: m0 }).await.unwrap();
    assert_eq!((b.crees, b.regularisations), (0, 0), "la régularisation n'est posée qu'une fois : {b:?}");
    assert_eq!(x.ent.elements_paie_for_actor(x.etab, x.compta, "compta", m0, None).await.unwrap().len(), 1);
    // Une absence annulée après envoi : régularisation négative sur le mois ouvert, jamais une réécriture.
    x.ent.annuler_absence_for_actor(x.etab, x.dir, "direction", ogec_id, &DecisionBody::default()).await.ok();
    // DSN J+5 (R-16) : arrêt de l'OGEC, file à jour, « signalé le » une seule fois, jamais dans le futur.
    let d = x.declarer(x.prof2, "prof", Some("maladie"), 1, 1).await;
    let ag2 = x.agent(x.prof2).await;
    x.engagement_ogec(ag2).await;
    let _ = d;
    let file = x.ent.dsn_file_for_actor(x.etab, x.compta, "compta").await.unwrap();
    let aujourdhui = x.aujourdhui().await;
    assert!(file.iter().all(|f| f.echeance_le >= aujourdhui || f.en_retard));
    for f in &file {
        assert!(f.signale_le.is_none());
        let e = x.ent.marquer_dsn_signale_for_actor(x.etab, x.compta, "compta", f.absence_id, x.aujourdhui().await + Duration::days(2)).await.unwrap_err();
        assert!(est_refus(&e, crate::service::RefusMetier::Invalide), "{e:?}");
        let ok = x.ent.marquer_dsn_signale_for_actor(x.etab, x.compta, "compta", f.absence_id, x.aujourdhui().await).await.unwrap();
        assert_eq!(ok.signale_le, Some(x.aujourdhui().await));
        assert!(x.ent.marquer_dsn_signale_for_actor(x.etab, x.compta, "compta", f.absence_id, x.aujourdhui().await).await.is_err());
    }
    assert!(x.ent.dsn_file_for_actor(x.etab, x.compta, "compta").await.unwrap().is_empty(), "la file se vide une fois signalée");
    assert!(matches!(x.ent.dsn_file_for_actor(x.etab, x.sec, "administratif").await.unwrap_err(), EntDbError::Forbidden));
}
