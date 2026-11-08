//! S2-5 contre la base locale de test (feature `labo`). Sans `ENT_DATABASE_URL`, ignorés.
//!
//! Couvre : conflits professeur / salle / classe / groupe / indisponibilité, semaines A/B,
//! vacances, vue de la semaine (personnel, classe, salle, élève), absence → séances touchées →
//! remplaçants → remplacement validé → délégation + notifications, annulation en masse,
//! déplacement, délégations datées (expiration, révocation, anti-escalade), notifications
//! idempotentes, isolation entre établissements et droits.

use super::*;
use crate::edt_admin::{
    CreneauConflit, Indisponibilite, NatureConflit, detecter_conflits, tous_les_conflits,
};
use crate::outbox::ConfigDistribution;
use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use domain_eleve::{DomainError, EdtSemaineAb, today_utc};
use ent_auth::{Action, Portee, Role};
use uuid::Uuid;

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

// ───────────────────────── Logique pure (sans base) ─────────────────────────

fn c(
    classe: &str,
    prof: Uuid,
    salle: Option<&str>,
    jour: i16,
    d: (u32, u32),
    f: (u32, u32),
    ab: EdtSemaineAb,
) -> CreneauConflit {
    CreneauConflit {
        id: Some(Uuid::new_v4()),
        classe: classe.into(),
        groupe_id: None,
        prof_user_id: prof,
        salle: salle.map(str::to_string),
        site: String::new(),
        jour_semaine: jour,
        heure_debut: t(d.0, d.1),
        heure_fin: t(f.0, f.1),
        semaine_ab: ab,
    }
}

#[test]
fn conflits_professeur_salle_classe() {
    let (p1, p2) = (Uuid::new_v4(), Uuid::new_v4());
    let base = c(
        "4B",
        p1,
        Some("B12"),
        1,
        (8, 0),
        (9, 0),
        EdtSemaineAb::Toutes,
    );
    let natures = |autre: &CreneauConflit| -> Vec<NatureConflit> {
        detecter_conflits(autre, std::slice::from_ref(&base), &[])
            .into_iter()
            .map(|x| x.nature)
            .collect()
    };
    // Même professeur, autre classe, autre salle.
    assert_eq!(
        natures(&c(
            "5A",
            p1,
            Some("C3"),
            1,
            (8, 30),
            (9, 30),
            EdtSemaineAb::Toutes
        )),
        vec![NatureConflit::Professeur]
    );
    // Même salle (casse et espaces ignorés), autre prof, autre classe.
    assert_eq!(
        natures(&c(
            "5A",
            p2,
            Some(" b12 "),
            1,
            (8, 0),
            (9, 0),
            EdtSemaineAb::Toutes
        )),
        vec![NatureConflit::Salle]
    );
    // Même classe.
    assert_eq!(
        natures(&c(
            "4b",
            p2,
            Some("C3"),
            1,
            (8, 59),
            (10, 0),
            EdtSemaineAb::Toutes
        )),
        vec![NatureConflit::Classe]
    );
    // Les trois à la fois.
    assert_eq!(
        natures(&c(
            "4B",
            p1,
            Some("B12"),
            1,
            (8, 0),
            (9, 0),
            EdtSemaineAb::Toutes
        ))
        .len(),
        3
    );
    // Bornes qui se touchent, autre jour : aucun conflit.
    assert!(
        natures(&c(
            "4B",
            p1,
            Some("B12"),
            1,
            (9, 0),
            (10, 0),
            EdtSemaineAb::Toutes
        ))
        .is_empty()
    );
    assert!(
        natures(&c(
            "4B",
            p1,
            Some("B12"),
            2,
            (8, 0),
            (9, 0),
            EdtSemaineAb::Toutes
        ))
        .is_empty()
    );
}

#[test]
fn conflits_semaines_ab_et_groupes() {
    let p = Uuid::new_v4();
    let a = c("4B", p, Some("B12"), 3, (14, 0), (15, 0), EdtSemaineAb::A);
    let mut b = a.clone();
    b.id = Some(Uuid::new_v4());
    b.semaine_ab = EdtSemaineAb::B;
    // A et B ne se rencontrent jamais : même prof, même salle, même classe, aucun conflit.
    assert!(detecter_conflits(&b, std::slice::from_ref(&a), &[]).is_empty());
    // Une semaine « toutes » rencontre A comme B.
    let mut t_ = a.clone();
    t_.id = Some(Uuid::new_v4());
    t_.semaine_ab = EdtSemaineAb::Toutes;
    assert_eq!(
        detecter_conflits(&t_, std::slice::from_ref(&a), &[]).len(),
        3
    );
    assert_eq!(
        detecter_conflits(&t_, std::slice::from_ref(&b), &[]).len(),
        3
    );
    // Groupes : deux groupes distincts de la même classe cohabitent (autre prof, autre salle) ;
    // la classe entière est en conflit avec chacun ; un même groupe aussi.
    let (g1, g2) = (Uuid::new_v4(), Uuid::new_v4());
    let mut grp1 = c(
        "4B",
        Uuid::new_v4(),
        Some("L1"),
        3,
        (14, 0),
        (15, 0),
        EdtSemaineAb::Toutes,
    );
    grp1.groupe_id = Some(g1);
    let mut grp2 = c(
        "4B",
        Uuid::new_v4(),
        Some("L2"),
        3,
        (14, 0),
        (15, 0),
        EdtSemaineAb::Toutes,
    );
    grp2.groupe_id = Some(g2);
    assert!(detecter_conflits(&grp2, std::slice::from_ref(&grp1), &[]).is_empty());
    let entiere = c(
        "4B",
        Uuid::new_v4(),
        Some("L3"),
        3,
        (14, 0),
        (15, 0),
        EdtSemaineAb::Toutes,
    );
    let n = detecter_conflits(&entiere, std::slice::from_ref(&grp1), &[]);
    assert_eq!(n.len(), 1);
    assert_eq!(n[0].nature, NatureConflit::Classe);
    let mut grp1bis = grp1.clone();
    grp1bis.id = Some(Uuid::new_v4());
    grp1bis.prof_user_id = Uuid::new_v4();
    grp1bis.salle = Some("L9".into());
    assert_eq!(
        detecter_conflits(&grp1bis, std::slice::from_ref(&grp1), &[]).len(),
        1
    );
}

#[test]
fn conflits_sites_et_indisponibilites() {
    let p = Uuid::new_v4();
    let mut a = c(
        "4B",
        p,
        Some("B12"),
        1,
        (8, 0),
        (9, 0),
        EdtSemaineAb::Toutes,
    );
    a.site = "Nord".into();
    let mut autre = c(
        "5A",
        Uuid::new_v4(),
        Some("B12"),
        1,
        (8, 0),
        (9, 0),
        EdtSemaineAb::Toutes,
    );
    autre.site = "Sud".into();
    // Même code de salle sur deux sites : deux salles différentes.
    assert!(detecter_conflits(&autre, std::slice::from_ref(&a), &[]).is_empty());
    autre.site = "nord".into();
    assert_eq!(
        detecter_conflits(&autre, std::slice::from_ref(&a), &[]).len(),
        1
    );
    // Indisponibilités du professeur et de la salle.
    let indispos = [
        Indisponibilite {
            prof_user_id: Some(p),
            salle_code: None,
            jour_semaine: Some(1),
            heure_debut: Some(t(8, 30)),
            heure_fin: Some(t(12, 0)),
        },
        Indisponibilite {
            prof_user_id: None,
            salle_code: Some("b12".into()),
            jour_semaine: None,
            heure_debut: None,
            heure_fin: None,
        },
    ];
    let natures: Vec<_> = detecter_conflits(&a, &[], &indispos)
        .into_iter()
        .map(|x| x.nature)
        .collect();
    assert_eq!(
        natures,
        vec![
            NatureConflit::IndisponibiliteProfesseur,
            NatureConflit::IndisponibiliteSalle
        ]
    );
    // Aucun doublon dans le détail d'un ensemble (chaque paire une fois).
    let x = c("4B", p, None, 4, (8, 0), (9, 0), EdtSemaineAb::Toutes);
    let mut y = x.clone();
    y.id = Some(Uuid::new_v4());
    assert_eq!(tous_les_conflits(&[x, y], &[]).len(), 2); // professeur + classe
}

// ───────────────────────── Aides pour la base ─────────────────────────

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn acteur<'a>(user: Uuid, role: Role, cast: &LaboCast) -> Acteur<'a> {
    Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: user,
        role: role.as_str(),
    }
}

fn dir(cast: &LaboCast) -> Acteur<'static> {
    acteur(cast.direction.user_id, Role::Direction, cast)
}

fn adm(cast: &LaboCast) -> Acteur<'static> {
    acteur(cast.administratif.user_id, Role::Administratif, cast)
}

fn prof_a(cast: &LaboCast, i: usize) -> Acteur<'static> {
    acteur(cast.profs[i].user_id, Role::Prof, cast)
}

fn config(etab: Uuid) -> ConfigDistribution {
    ConfigDistribution {
        taille_lot: 200,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    }
}

async fn distribuer(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(config(etab))
        .distribuer_tout()
        .await
        .unwrap();
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

struct Ctx {
    ent: PgEnt,
    cast: LaboCast,
    lundi: NaiveDate,
}

async fn ctx() -> Option<Ctx> {
    let (ent, cast) = labo().await?;
    let annee = ent
        .referentiel_annee_courante_assuree(cast.etablissement_id)
        .await
        .unwrap();
    let lundi = lundi_futur(annee.date_debut, annee.date_fin);
    // Les créneaux « du jour » du jeu de labo ne servent pas ici : emploi du temps vierge.
    sqlx::query("DELETE FROM ent_edt_creneau WHERE etablissement_id = $1")
        .bind(cast.etablissement_id)
        .execute(ent.pool())
        .await
        .expect("retrait des créneaux du labo");
    Some(Ctx { ent, cast, lundi })
}

impl Ctx {
    fn e(&self) -> Uuid {
        self.cast.etablissement_id
    }

    async fn salle(&self, code: &str) {
        sqlx::query("INSERT INTO ent_salle (id, etablissement_id, code, libelle, created_by_user_id) VALUES ($1,$2,$3,$3,$4)")
            .bind(Uuid::new_v4())
            .bind(self.e())
            .bind(code)
            .bind(self.cast.admin.user_id)
            .execute(self.ent.pool())
            .await
            .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn saisie(
        &self,
        classe: &str,
        prof: Uuid,
        jour: i16,
        d: (u32, u32),
        f: (u32, u32),
        salle: Option<&str>,
        ab: &str,
    ) -> CreneauSaisie {
        CreneauSaisie {
            classe: classe.into(),
            groupe_id: None,
            matiere_libelle: "Mathématiques".into(),
            prof_user_id: prof,
            jour_semaine: jour,
            heure_debut: t(d.0, d.1),
            heure_fin: t(f.0, f.1),
            salle: salle.map(str::to_string),
            site: String::new(),
            semaine_ab: ab.into(),
        }
    }

    /// Crée un créneau (doit réussir) et renvoie son identifiant.
    async fn creneau(&self, s: &CreneauSaisie) -> Uuid {
        match self
            .ent
            .edt_creneau_creer(&dir(&self.cast), s, false)
            .await
            .unwrap()
        {
            ResultatCreneau::Enregistre { creneau: Some(c) } => c.id,
            autre => panic!("créneau refusé : {autre:?}"),
        }
    }

    /// Jour de la semaine de test (1 = lundi).
    fn jour(&self, iso: i64) -> NaiveDate {
        self.lundi + Duration::days(iso - 1)
    }

    async fn notifs(&self, user: Uuid, kind: &str) -> Vec<crate::reactions::NotificationInterne> {
        self.ent
            .mes_notifications(self.e(), user, 200)
            .await
            .unwrap()
            .into_iter()
            .filter(|n| n.kind == kind)
            .collect()
    }

    async fn nb_events(&self, type_name: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = $2",
        )
        .bind(self.e())
        .bind(type_name)
        .fetch_one(self.ent.pool())
        .await
        .unwrap()
    }
}

// ───────────────────────── Créneaux, conflits, droits ─────────────────────────

#[tokio::test]
async fn creneaux_conflits_apercu_modification_et_droits() {
    let Some(x) = ctx().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (p1, p2) = (x.cast.profs[0].user_id, x.cast.profs[1].user_id);
    x.salle("S1").await;
    x.salle("S2").await;
    let a = dir(&x.cast);
    let id = x
        .creneau(&x.saisie("4B", p1, 2, (14, 0), (15, 0), Some("S1"), "toutes"))
        .await;
    let refus = |r: ResultatCreneau| match r {
        ResultatCreneau::Conflits { conflits } => {
            conflits.into_iter().map(|c| c.nature).collect::<Vec<_>>()
        }
        autre => panic!("conflit attendu : {autre:?}"),
    };
    // Même professeur ; même salle ; même classe.
    let r = x
        .ent
        .edt_creneau_creer(
            &a,
            &x.saisie("5A", p1, 2, (14, 30), (15, 30), Some("S2"), "toutes"),
            false,
        )
        .await
        .unwrap();
    assert_eq!(refus(r), vec![NatureConflit::Professeur]);
    let r = x
        .ent
        .edt_creneau_creer(
            &a,
            &x.saisie("5A", p2, 2, (14, 0), (15, 0), Some("S1"), "toutes"),
            false,
        )
        .await
        .unwrap();
    assert_eq!(refus(r), vec![NatureConflit::Salle]);
    let r = x
        .ent
        .edt_creneau_creer(
            &a,
            &x.saisie("4B", p2, 2, (14, 0), (15, 0), Some("S2"), "toutes"),
            false,
        )
        .await
        .unwrap();
    assert_eq!(refus(r), vec![NatureConflit::Classe]);
    // Rien n'a été écrit par les refus.
    let nb: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_edt_creneau WHERE etablissement_id = $1 AND jour_semaine = 2 AND heure_debut = '14:00'")
        .bind(x.e()).fetch_one(x.ent.pool()).await.unwrap();
    assert_eq!(nb, 1);
    // Semaine B contre semaine A : même professeur, salle et classe, pas de conflit.
    let ab = x
        .ent
        .edt_creneau_modifier(
            &a,
            id,
            &x.saisie("4B", p1, 2, (14, 0), (15, 0), Some("S1"), "a"),
            false,
        )
        .await
        .unwrap();
    assert!(matches!(ab, ResultatCreneau::Enregistre { .. }));
    let b = x
        .creneau(&x.saisie("4B", p1, 2, (14, 0), (15, 0), Some("S1"), "b"))
        .await;
    assert_ne!(b, id);
    // Aperçu : valide sans écrire.
    let ap = x
        .ent
        .edt_creneau_creer(
            &a,
            &x.saisie("5A", p2, 3, (14, 0), (15, 0), Some("S2"), "toutes"),
            true,
        )
        .await
        .unwrap();
    assert_eq!(ap, ResultatCreneau::Enregistre { creneau: None });
    let nb: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_edt_creneau WHERE etablissement_id = $1 AND jour_semaine = 3 AND heure_debut = '14:00'")
        .bind(x.e()).fetch_one(x.ent.pool()).await.unwrap();
    assert_eq!(nb, 0);
    // Détection d'ensemble : un conflit introduit hors service (SQL direct) est listé.
    assert!(x.ent.edt_conflits(&a).await.unwrap().is_empty());
    sqlx::query("INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine, heure_debut, heure_fin, salle)
                 VALUES ($1,$2,'3C','Autre',$3,2,'14:00','15:00','Z9')")
        .bind(Uuid::new_v4()).bind(x.e()).bind(p1).execute(x.ent.pool()).await.unwrap();
    let conflits = x.ent.edt_conflits(&a).await.unwrap();
    assert!(
        conflits
            .iter()
            .any(|c| c.nature == NatureConflit::Professeur),
        "{conflits:?}"
    );
    // Validation de la saisie : classe fantôme, professeur d'ailleurs, salle inconnue, heures.
    let invalide = |e: EntDbError| matches!(e, EntDbError::Domain(DomainError::ValeurInvalide));
    let introuvable = |e: EntDbError| matches!(e, EntDbError::Domain(DomainError::NotFound));
    assert!(invalide(
        x.ent
            .edt_creneau_creer(
                &a,
                &x.saisie("ZZ9", p2, 4, (14, 0), (15, 0), None, "toutes"),
                false
            )
            .await
            .unwrap_err()
    ));
    assert!(invalide(
        x.ent
            .edt_creneau_creer(
                &a,
                &x.saisie("4B", p2, 4, (15, 0), (14, 0), None, "toutes"),
                false
            )
            .await
            .unwrap_err()
    ));
    assert!(invalide(
        x.ent
            .edt_creneau_creer(
                &a,
                &x.saisie("4B", p2, 8, (14, 0), (15, 0), None, "toutes"),
                false
            )
            .await
            .unwrap_err()
    ));
    assert!(invalide(
        x.ent
            .edt_creneau_creer(
                &a,
                &x.saisie("4B", p2, 4, (14, 0), (15, 0), None, "c"),
                false
            )
            .await
            .unwrap_err()
    ));
    assert!(introuvable(
        x.ent
            .edt_creneau_creer(
                &a,
                &x.saisie("4B", Uuid::new_v4(), 4, (14, 0), (15, 0), None, "toutes"),
                false
            )
            .await
            .unwrap_err()
    ));
    assert!(introuvable(
        x.ent
            .edt_creneau_creer(
                &a,
                &x.saisie(
                    "4B",
                    x.cast.parent_lea.user_id,
                    4,
                    (14, 0),
                    (15, 0),
                    None,
                    "toutes"
                ),
                false
            )
            .await
            .unwrap_err()
    ));
    assert!(introuvable(
        x.ent
            .edt_creneau_creer(
                &a,
                &x.saisie("4B", p2, 4, (14, 0), (15, 0), Some("INCONNUE"), "toutes"),
                false
            )
            .await
            .unwrap_err()
    ));
    // Droits : professeur et comptabilité refusés ; administratif autorisé (modèle par défaut).
    for refuse in [
        prof_a(&x.cast, 0),
        acteur(x.cast.compta.user_id, Role::Compta, &x.cast),
    ] {
        let e = x
            .ent
            .edt_creneau_creer(
                &refuse,
                &x.saisie("4B", p2, 4, (14, 0), (15, 0), None, "toutes"),
                false,
            )
            .await
            .unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "{e:?}");
        assert!(matches!(
            x.ent.edt_conflits(&refuse).await.unwrap_err(),
            EntDbError::Forbidden
        ));
    }
    assert!(matches!(
        x.ent
            .edt_creneau_creer(
                &adm(&x.cast),
                &x.saisie("4B", p2, 4, (14, 0), (15, 0), None, "toutes"),
                true
            )
            .await
            .unwrap(),
        ResultatCreneau::Enregistre { .. }
    ));
    // Suppression : refusée tant que des séances existent, possible sinon ; événements publiés.
    let annee = x
        .ent
        .referentiel_annee_courante_assuree(x.e())
        .await
        .unwrap();
    let fk = x
        .creneau(&x.saisie("4B", p2, 5, (16, 0), (17, 0), None, "toutes"))
        .await;
    x.ent
        .ref_generer_seances(
            &dir(&x.cast),
            annee.id,
            Some(x.lundi),
            Some(x.lundi + Duration::days(6)),
        )
        .await
        .unwrap();
    let e = x.ent.edt_creneau_supprimer(&a, fk).await.unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(DomainError::Conflit)),
        "{e:?}"
    );
    sqlx::query("DELETE FROM ent_edt_seance WHERE etablissement_id = $1")
        .bind(x.e())
        .execute(x.ent.pool())
        .await
        .unwrap();
    x.ent.edt_creneau_supprimer(&a, fk).await.unwrap();
    x.ent.edt_creneau_supprimer(&a, b).await.unwrap();
    assert!(matches!(
        x.ent.edt_creneau_supprimer(&a, b).await.unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    assert_eq!(x.nb_events("edt.creneau_created").await, 3);
    assert!(x.nb_events("edt.creneau_modifie").await >= 1);
    assert_eq!(x.nb_events("edt.creneau_supprime").await, 2);
}

// ───────────────────────── Semaine : A/B, vacances, cibles, accès ─────────────────────────

#[tokio::test]
async fn semaine_ab_vacances_cibles_et_acces() {
    let Some(x) = ctx().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let p1 = x.cast.profs[0].user_id;
    x.salle("S1").await;
    // Créneau A le mardi, créneau B le mardi (autre heure), créneau de toutes les semaines le jeudi.
    let ca = x
        .creneau(&x.saisie("4B", p1, 2, (14, 0), (15, 0), Some("S1"), "a"))
        .await;
    let cb = x
        .creneau(&x.saisie("4B", p1, 2, (15, 0), (16, 0), Some("S1"), "b"))
        .await;
    let ct = x
        .creneau(&x.saisie("4B", p1, 4, (14, 0), (15, 0), Some("S1"), "toutes"))
        .await;
    let v = dir(&x.cast);
    // Sans calendrier d'alternance : seuls les créneaux « toutes » sont attendus (rien d'inventé).
    let sem = x
        .ent
        .edt_semaine(&v, CibleEdt::Personnel(p1), x.lundi)
        .await
        .unwrap();
    assert_eq!(sem.semaine_ab, None);
    assert_eq!(
        sem.seances.iter().map(|s| s.creneau_id).collect::<Vec<_>>(),
        vec![ct]
    );
    // Calendrier : la semaine de test est A, la suivante B.
    sqlx::query("INSERT INTO ent_edt_calendrier_ab (etablissement_id, reference_lundi, created_by_user_id, updated_at) VALUES ($1,$2,$3,now())")
        .bind(x.e()).bind(x.lundi).bind(x.cast.admin.user_id).execute(x.ent.pool()).await.unwrap();
    let s1 = x
        .ent
        .edt_semaine(&v, CibleEdt::Personnel(p1), x.lundi)
        .await
        .unwrap();
    assert_eq!(s1.semaine_ab.as_deref(), Some("A"));
    let ids1: Vec<Uuid> = s1.seances.iter().map(|s| s.creneau_id).collect();
    assert!(
        ids1.contains(&ca) && ids1.contains(&ct) && !ids1.contains(&cb),
        "{ids1:?}"
    );
    // Un jour quelconque de la semaine donne la même semaine.
    let s1b = x
        .ent
        .edt_semaine(&v, CibleEdt::Personnel(p1), x.jour(5))
        .await
        .unwrap();
    assert_eq!(s1b.lundi, x.lundi);
    let s2 = x
        .ent
        .edt_semaine(&v, CibleEdt::Personnel(p1), x.lundi + Duration::days(7))
        .await
        .unwrap();
    assert_eq!(s2.semaine_ab.as_deref(), Some("B"));
    let ids2: Vec<Uuid> = s2.seances.iter().map(|s| s.creneau_id).collect();
    assert!(
        ids2.contains(&cb) && ids2.contains(&ct) && !ids2.contains(&ca),
        "{ids2:?}"
    );
    // Autres cibles : classe, salle, élève (Harry est en 4B), classe sans cours.
    let c4b: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_classe WHERE etablissement_id = $1 AND code = '4B' LIMIT 1",
    )
    .bind(x.e())
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    let par_classe = x
        .ent
        .edt_semaine(&v, CibleEdt::Classe(c4b), x.lundi)
        .await
        .unwrap();
    assert_eq!(par_classe.seances.len(), 2);
    let par_salle = x
        .ent
        .edt_semaine(&v, CibleEdt::Salle("s1".into()), x.lundi)
        .await
        .unwrap();
    assert_eq!(par_salle.seances.len(), 2);
    assert!(
        x.ent
            .edt_semaine(&v, CibleEdt::Salle("S2".into()), x.lundi)
            .await
            .unwrap()
            .seances
            .is_empty()
    );
    let par_eleve = x
        .ent
        .edt_semaine(&v, CibleEdt::Eleve(x.cast.eleve_lea_id), x.lundi)
        .await
        .unwrap();
    assert_eq!(par_eleve.seances.len(), 2);
    assert!(par_eleve.seances.iter().all(|s| s.prof_nom.is_some()));
    // Vacances sur la semaine : plus aucune séance, jours fermés signalés.
    let annee = x
        .ent
        .referentiel_annee_courante_assuree(x.e())
        .await
        .unwrap();
    x.ent
        .ref_creer_vacances(
            &v,
            "Vacances test",
            "vacances",
            x.lundi,
            x.lundi + Duration::days(6),
        )
        .await
        .unwrap();
    let sv = x
        .ent
        .edt_semaine(&v, CibleEdt::Classe(c4b), x.lundi)
        .await
        .unwrap();
    assert!(sv.seances.is_empty());
    assert!(!sv.jours_fermes.is_empty());
    assert!(annee.date_debut <= x.lundi);
    // Accès : l'élève voit le sien, pas celui d'un autre ; le parent voit l'enfant lié seulement ;
    // l'élève/parent ne voient ni classe ni salle ; élève d'un autre établissement = introuvable.
    let harry = &x.cast.eleves[0];
    let compte = acteur(harry.compte.as_ref().unwrap().user_id, Role::Eleve, &x.cast);
    assert!(
        x.ent
            .edt_semaine(
                &compte,
                CibleEdt::Eleve(harry.id),
                x.lundi + Duration::days(7)
            )
            .await
            .is_ok()
    );
    let autre_eleve = x.cast.eleves[2].id;
    assert!(matches!(
        x.ent
            .edt_semaine(&compte, CibleEdt::Eleve(autre_eleve), x.lundi)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    let parent = acteur(harry.parent.user_id, Role::Parent, &x.cast);
    assert!(
        x.ent
            .edt_semaine(&parent, CibleEdt::Eleve(harry.id), x.lundi)
            .await
            .is_ok()
    );
    assert!(matches!(
        x.ent
            .edt_semaine(&parent, CibleEdt::Eleve(x.cast.eleve_tom_id), x.lundi)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(matches!(
        x.ent
            .edt_semaine(&parent, CibleEdt::Classe(c4b), x.lundi)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(matches!(
        x.ent
            .edt_semaine(&compte, CibleEdt::Salle("S1".into()), x.lundi)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(matches!(
        x.ent
            .edt_semaine(&compte, CibleEdt::Personnel(p1), x.lundi)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    let (autre, cast2) = {
        let c2 = x.ent.seed_labo_cast().await.unwrap();
        (acteur(c2.direction.user_id, Role::Direction, &c2), c2)
    };
    // Une direction d'un autre établissement n'a ni la classe ni l'élève d'ici (404, pas 403).
    assert!(matches!(
        x.ent
            .edt_semaine(&autre, CibleEdt::Classe(c4b), x.lundi)
            .await
            .unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    assert!(matches!(
        x.ent
            .edt_semaine(&autre, CibleEdt::Eleve(harry.id), x.lundi)
            .await
            .unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    let _ = cast2;
}

// ───────────────────────── Absence → remplacement → familles prévenues ─────────────────────────

#[tokio::test]
async fn absence_remplacement_delegation_et_notifications() {
    let Some(x) = ctx().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (absent, libre, occupe, indispo, autre_mat) = (
        x.cast.profs[0].user_id,
        x.cast.profs[1].user_id,
        x.cast.profs[2].user_id,
        x.cast.profs[3].user_id,
        x.cast.profs[4].user_id,
    );
    let d = dir(&x.cast);
    // Le professeur absent enseigne les maths à la 4B lundi et mardi.
    let lundi_cr = x
        .creneau(&x.saisie("4B", absent, 1, (14, 0), (15, 0), None, "toutes"))
        .await;
    let mardi_cr = x
        .creneau(&x.saisie("4B", absent, 2, (14, 0), (15, 0), None, "toutes"))
        .await;
    // « libre » enseigne aussi les maths (5A, autre heure) : compétent et disponible.
    x.creneau(&x.saisie("5A", libre, 1, (16, 0), (17, 0), None, "toutes"))
        .await;
    // « occupe » enseigne les maths à la même heure : conflit.
    x.creneau(&x.saisie("5A", occupe, 1, (14, 0), (15, 0), None, "toutes"))
        .await;
    // « indispo » enseigne les maths mais déclare une indisponibilité le lundi après-midi.
    x.creneau(&x.saisie("3C", indispo, 3, (14, 0), (15, 0), None, "toutes"))
        .await;
    sqlx::query("INSERT INTO ent_edt_contrainte (id, etablissement_id, kind, prof_user_id, jour_semaine, heure_debut, heure_fin, created_by_user_id)
                 VALUES ($1,$2,'prof_indispo',$3,1,'13:00','18:00',$4)")
        .bind(Uuid::new_v4()).bind(x.e()).bind(indispo).bind(x.cast.admin.user_id)
        .execute(x.ent.pool()).await.unwrap();
    // « autre_mat » enseigne une autre matière : libre mais non compétent.
    let mut s = x.saisie("3C", autre_mat, 5, (14, 0), (15, 0), None, "toutes");
    s.matiere_libelle = "Latin".into();
    x.creneau(&s).await;

    // Absence déclarée mais pas validée : aucune séance à traiter n'est présentée.
    let abs_decl = x
        .ent
        .declarer_absence_rh_for_actor(
            d.role,
            domain_eleve::DeclarerAbsenceRh {
                etablissement_id: x.e(),
                user_id: absent,
                debut: x.jour(1),
                fin: x.jour(2),
                motif: "Maladie".into(),
                actor_user_id: d.user_id,
            },
        )
        .await
        .unwrap();
    let liste = x
        .ent
        .edt_absences(&d, x.lundi, x.lundi + Duration::days(6))
        .await
        .unwrap();
    let ligne = liste.iter().find(|l| l.absence.id == abs_decl.id).unwrap();
    assert_eq!(
        (ligne.absence.statut.as_str(), ligne.a_traiter),
        ("signalee", 0)
    );
    x.ent
        .valider_absence_rh_for_actor(x.e(), d.role, abs_decl.id, d.user_id)
        .await
        .unwrap();
    // Validée deux fois : un seul événement `rh.absence_validee` (outbox), sans motif.
    x.ent
        .valider_absence_rh_for_actor(x.e(), d.role, abs_decl.id, d.user_id)
        .await
        .unwrap();
    assert_eq!(x.nb_events("rh.absence").await, 1);
    assert_eq!(x.nb_events("rh.absence_validee").await, 1);
    let charge: serde_json::Value = sqlx::query_scalar("SELECT e.payload FROM ent_metier_event e WHERE e.etablissement_id = $1 AND e.type = 'rh.absence'")
        .bind(x.e()).fetch_one(x.ent.pool()).await.unwrap();
    assert!(
        charge.get("motif").is_none(),
        "motif libre dans l'événement : {charge}"
    );

    // Séances touchées listées automatiquement.
    let touchees = x.ent.edt_absence_seances(&d, abs_decl.id).await.unwrap();
    assert_eq!(touchees.seances.len(), 2);
    assert_eq!(touchees.a_traiter, 2);
    assert!(
        touchees
            .seances
            .iter()
            .all(|s| s.statut == "prevue" && s.role_absent == "titulaire")
    );

    // Remplaçants : compétent + libre seulement ; ni l'absent, ni l'occupé, ni l'indisponible.
    let cands = x
        .ent
        .edt_candidats_remplacants(&d, lundi_cr, x.jour(1), false)
        .await
        .unwrap();
    assert_eq!(
        cands.iter().map(|c| c.user_id).collect::<Vec<_>>(),
        vec![libre],
        "{cands:?}"
    );
    assert!(cands[0].competent);
    // `tous` ajoute le non compétent libre (autre matière) mais jamais un conflit.
    let tous = x
        .ent
        .edt_candidats_remplacants(&d, lundi_cr, x.jour(1), true)
        .await
        .unwrap();
    let ids: Vec<Uuid> = tous.iter().map(|c| c.user_id).collect();
    assert!(ids.contains(&libre) && ids.contains(&autre_mat));
    assert!(
        !ids.contains(&absent) && !ids.contains(&occupe) && !ids.contains(&indispo),
        "{ids:?}"
    );
    assert_eq!(ids[0], libre, "les compétents d'abord");

    // Remplacement refusé pour un conflit, un non-personnel, un autre établissement.
    let cmd = |rempl: Uuid, creneau: Uuid, jour: NaiveDate| CmdRemplacement {
        creneau_id: creneau,
        jour,
        remplacant_user_id: rempl,
        absence_rh_id: Some(abs_decl.id),
        nouvelle_salle: None,
    };
    let e = x
        .ent
        .edt_organiser_remplacement(&d, &cmd(occupe, lundi_cr, x.jour(1)))
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(DomainError::Conflit)),
        "{e:?}"
    );
    let e = x
        .ent
        .edt_organiser_remplacement(&d, &cmd(absent, lundi_cr, x.jour(1)))
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(DomainError::Conflit)),
        "{e:?}"
    );
    let e = x
        .ent
        .edt_organiser_remplacement(&d, &cmd(Uuid::new_v4(), lundi_cr, x.jour(1)))
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(DomainError::NotFound)),
        "{e:?}"
    );
    // Hors période de l'absence (mercredi) : refus.
    x.creneau(&x.saisie("4B", absent, 3, (16, 0), (17, 0), None, "toutes"))
        .await;
    let mercredi = x.ent.edt_plans(x.e(), x.jour(3), x.jour(3)).await.unwrap();
    let cr_mer = mercredi
        .iter()
        .find(|p| p.plan.prof_user_id == absent)
        .unwrap()
        .plan
        .creneau_id;
    let e = x
        .ent
        .edt_organiser_remplacement(&d, &cmd(libre, cr_mer, x.jour(3)))
        .await
        .unwrap_err();
    assert!(
        matches!(e, EntDbError::Domain(DomainError::Conflit)),
        "{e:?}"
    );
    // Absence non validée : refus.
    let abs_non = x
        .ent
        .declarer_absence_rh_for_actor(
            d.role,
            domain_eleve::DeclarerAbsenceRh {
                etablissement_id: x.e(),
                user_id: absent,
                debut: x.jour(3),
                fin: x.jour(3),
                motif: "RDV".into(),
                actor_user_id: d.user_id,
            },
        )
        .await
        .unwrap();
    let mut c3 = cmd(libre, cr_mer, x.jour(3));
    c3.absence_rh_id = Some(abs_non.id);
    assert!(matches!(
        x.ent.edt_organiser_remplacement(&d, &c3).await.unwrap_err(),
        EntDbError::Domain(DomainError::Conflit)
    ));
    // Droits : un professeur ne peut pas organiser.
    assert!(matches!(
        x.ent
            .edt_organiser_remplacement(&prof_a(&x.cast, 1), &cmd(libre, lundi_cr, x.jour(1)))
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    assert_eq!(x.nb_events("edt.remplacement_organise").await, 0);

    // Remplacement validé : exception, séance, délégation datée, événements.
    let r = x
        .ent
        .edt_organiser_remplacement(&d, &cmd(libre, lundi_cr, x.jour(1)))
        .await
        .unwrap();
    assert!(!r.deja_fait);
    let deleg = r.delegation_id.expect("délégation de remplacement");
    // Idempotent : même demande, rien de plus.
    let r2 = x
        .ent
        .edt_organiser_remplacement(&d, &cmd(libre, lundi_cr, x.jour(1)))
        .await
        .unwrap();
    assert!(r2.deja_fait);
    assert_eq!(
        (r2.exception_id, r2.delegation_id),
        (r.exception_id, Some(deleg))
    );
    assert_eq!(x.nb_events("edt.remplacement_organise").await, 1);
    assert_eq!(x.nb_events("delegation.accordee").await, 1);
    let touchees = x.ent.edt_absence_seances(&d, abs_decl.id).await.unwrap();
    assert_eq!(touchees.a_traiter, 1);
    let lundi_s = touchees
        .seances
        .iter()
        .find(|s| s.creneau_id == lundi_cr)
        .unwrap();
    assert_eq!(
        (lundi_s.statut.as_str(), lundi_s.remplacant_user_id),
        ("remplacee", Some(libre))
    );
    // Le remplaçant n'est plus libre à cette heure : il ne remplace pas un second cours simultané.
    // Droits du remplaçant : `absences.lire` limité à la classe, uniquement ce jour-là (délégation).
    let droits = x
        .ent
        .droits_effectifs(x.e(), libre, Role::Prof.as_str())
        .await
        .unwrap();
    let c4b: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_classe WHERE etablissement_id = $1 AND code = '4B' LIMIT 1",
    )
    .bind(x.e())
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    let delegue_aujourdhui = droits.iter().any(|dr| {
        dr.action == Action::AbsencesLire && dr.portee == Portee::Designes && dr.cibles == vec![c4b]
    });
    assert_eq!(
        delegue_aujourdhui,
        x.jour(1) == today_utc(),
        "la délégation ne vaut que le jour du remplacement"
    );
    let dele = x.ent.delegations_lister(&d, false).await.unwrap();
    let dl = dele.iter().find(|y| y.id == deleg).unwrap();
    assert_eq!(
        (dl.source.as_str(), dl.debut, dl.fin, dl.delegue_user_id),
        ("remplacement", x.jour(1), x.jour(1), libre)
    );
    let _ = c4b;

    // Familles et élèves prévenus après distribution : parent de Harry et compte élève (4B),
    // pas ceux de la 5A ; le remplaçant est prévenu ; aucun motif.
    distribuer(&x.ent, x.e()).await;
    let harry = &x.cast.eleves[0];
    let n_parent = x.notifs(harry.parent.user_id, "edt_remplacement").await;
    assert_eq!(n_parent.len(), 1, "{n_parent:?}");
    assert!(n_parent[0].corps.contains("autre enseignant"));
    assert!(!n_parent[0].corps.contains("Maladie") && !n_parent[0].titre.contains("Maladie"));
    assert_eq!(
        x.notifs(harry.compte.as_ref().unwrap().user_id, "edt_remplacement")
            .await
            .len(),
        1
    );
    let neville = x
        .cast
        .eleves
        .iter()
        .find(|e| e.prenom == "Neville")
        .unwrap();
    assert!(
        x.notifs(neville.parent.user_id, "edt_remplacement")
            .await
            .is_empty(),
        "classe 3C non concernée"
    );
    assert_eq!(x.notifs(libre, "edt_remplacement_assigne").await.len(), 1);
    // Rejeu / seconde distribution : aucune notification en double.
    distribuer(&x.ent, x.e()).await;
    distribuer(&x.ent, x.e()).await;
    assert_eq!(
        x.notifs(harry.parent.user_id, "edt_remplacement")
            .await
            .len(),
        1
    );
    assert_eq!(x.notifs(libre, "edt_remplacement_assigne").await.len(), 1);

    // Changer de remplaçant révoque la délégation précédente et crée la nouvelle.
    let r3 = x
        .ent
        .edt_organiser_remplacement(&d, &cmd(autre_mat, lundi_cr, x.jour(1)))
        .await
        .unwrap();
    assert!(!r3.deja_fait);
    assert_eq!(r3.exception_id, r.exception_id);
    let ancien: String = sqlx::query_scalar("SELECT statut FROM ent_delegation WHERE id = $1")
        .bind(deleg)
        .fetch_one(x.ent.pool())
        .await
        .unwrap();
    assert_eq!(ancien, "revoquee");
    // Annulation du même cours : la délégation du dernier remplaçant est révoquée aussi.
    let an = x
        .ent
        .edt_annuler_seance(&d, lundi_cr, x.jour(1), Some(abs_decl.id))
        .await
        .unwrap();
    assert!(!an.deja_fait);
    assert!(
        x.ent
            .edt_annuler_seance(&d, lundi_cr, x.jour(1), None)
            .await
            .unwrap()
            .deja_fait
    );
    let actives: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_delegation WHERE etablissement_id = $1 AND source = 'remplacement' AND statut = 'accordee'")
        .bind(x.e()).fetch_one(x.ent.pool()).await.unwrap();
    assert_eq!(actives, 0);
    // Lever l'annulation rétablit le cours et prévient.
    assert!(
        x.ent
            .edt_lever_exception(&d, lundi_cr, x.jour(1))
            .await
            .unwrap()
    );
    assert!(
        !x.ent
            .edt_lever_exception(&d, lundi_cr, x.jour(1))
            .await
            .unwrap()
    );
    distribuer(&x.ent, x.e()).await;
    assert_eq!(x.notifs(harry.parent.user_id, "edt_retabli").await.len(), 1);
    // L'annulation a été notifiée une fois ; l'état courant (levée) n'est plus une annulation :
    // la notification d'annulation, distribuée après la levée, n'aurait pas été émise.
    let touchees = x.ent.edt_absence_seances(&d, abs_decl.id).await.unwrap();
    assert_eq!(touchees.a_traiter, 2);
    let _ = mardi_cr;
}

#[tokio::test]
async fn notification_d_etat_perime_et_isolation_entre_etablissements() {
    let Some(x) = ctx().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let p1 = x.cast.profs[0].user_id;
    let d = dir(&x.cast);
    let cr = x
        .creneau(&x.saisie("4B", p1, 1, (14, 0), (15, 0), None, "toutes"))
        .await;
    // Annulation puis levée AVANT distribution : l'état courant prime, aucune notification périmée.
    x.ent
        .edt_annuler_seance(&d, cr, x.jour(1), None)
        .await
        .unwrap();
    x.ent.edt_lever_exception(&d, cr, x.jour(1)).await.unwrap();
    distribuer(&x.ent, x.e()).await;
    let harry = &x.cast.eleves[0];
    assert!(
        x.notifs(harry.parent.user_id, "edt_annulation")
            .await
            .is_empty()
    );
    assert_eq!(x.notifs(harry.parent.user_id, "edt_retabli").await.len(), 1);
    // Annulation maintenue : une notification, toujours une seule après rejeu du consommateur.
    x.ent
        .edt_annuler_seance(&d, cr, x.jour(1), None)
        .await
        .unwrap();
    distribuer(&x.ent, x.e()).await;
    assert_eq!(
        x.notifs(harry.parent.user_id, "edt_annulation").await.len(),
        1
    );
    assert!(
        x.ent
            .distributeur(config(x.e()))
            .rejouer_consommateur(crate::edt_reactions::CONSOMMATEUR_EDT, x.e(), 0)
            .await
            .is_err(),
        "consommateur non rejouable"
    );
    distribuer(&x.ent, x.e()).await;
    assert_eq!(
        x.notifs(harry.parent.user_id, "edt_annulation").await.len(),
        1
    );

    // Isolation : un autre établissement ne peut ni voir, ni annuler, ni remplacer ce créneau,
    // et un événement forgé chez lui citant ce créneau ne notifie personne.
    let c2 = x.ent.seed_labo_cast().await.unwrap();
    let d2 = acteur(c2.direction.user_id, Role::Direction, &c2);
    assert!(matches!(
        x.ent
            .edt_annuler_seance(&d2, cr, x.jour(2), None)
            .await
            .unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    assert!(matches!(
        x.ent
            .edt_organiser_remplacement(
                &d2,
                &CmdRemplacement {
                    creneau_id: cr,
                    jour: x.jour(1),
                    remplacant_user_id: c2.profs[1].user_id,
                    absence_rh_id: None,
                    nouvelle_salle: None
                }
            )
            .await
            .unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    assert!(matches!(
        x.ent.edt_creneau_supprimer(&d2, cr).await.unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    assert!(matches!(
        x.ent
            .edt_candidats_remplacants(&d2, cr, x.jour(1), true)
            .await
            .unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    let mut tx = x.ent.pool().begin().await.unwrap();
    {
        use crate::outbox::Publier;
        let conn: &mut sqlx::PgConnection = &mut tx;
        conn.publier(ent_events::MetierEventDraft {
            etablissement_id: c2.etablissement_id,
            type_name: "edt.exception".into(),
            aggregate: ent_events::AggregateKind::Edt,
            aggregate_id: cr,
            eleve_id: None,
            payload: serde_json::json!({ "creneau_id": cr, "jour": x.jour(1), "nature": "annulation" }),
            actor_user_id: None,
        })
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
    distribuer(&x.ent, c2.etablissement_id).await;
    let n2: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND kind LIKE 'edt_%'",
    )
    .bind(c2.etablissement_id)
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    assert_eq!(n2, 0);
}

// ───────────────────────── Annulation en masse ─────────────────────────

#[tokio::test]
async fn annulation_en_masse_apercu_vacances_ab_cle_et_droits() {
    let Some(x) = ctx().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (p1, p2) = (x.cast.profs[0].user_id, x.cast.profs[1].user_id);
    let d = dir(&x.cast);
    // 4B : lundi (toutes) ; 5A : mardi semaine A seulement. Deux semaines : A puis B.
    let c_4b = x
        .creneau(&x.saisie("4B", p1, 1, (14, 0), (15, 0), None, "toutes"))
        .await;
    let c_5a = x
        .creneau(&x.saisie("5A", p2, 2, (14, 0), (15, 0), None, "a"))
        .await;
    sqlx::query("INSERT INTO ent_edt_calendrier_ab (etablissement_id, reference_lundi, created_by_user_id, updated_at) VALUES ($1,$2,$3,now())")
        .bind(x.e()).bind(x.lundi).bind(x.cast.admin.user_id).execute(x.ent.pool()).await.unwrap();
    // Vacances le lundi de la 2ᵉ semaine : exclu d'office.
    let lundi2 = x.lundi + Duration::days(7);
    x.ent
        .ref_creer_vacances(&d, "Fermeture", "vacances", lundi2, lundi2)
        .await
        .unwrap();
    let cmd = |apercu: bool, cle: Option<&str>| CmdAnnulationMasse {
        du: x.lundi,
        au: x.lundi + Duration::days(13),
        portee: "etablissement".into(),
        cibles: vec![],
        heure_debut: None,
        heure_fin: None,
        motif_public: "Grève".into(),
        cle: cle.map(str::to_string),
        apercu,
    };
    // Aperçu : 4B (lundi 1) + 5A (mardi A) ; lundi 2 fermé ; mardi B hors alternance.
    let ap = x
        .ent
        .edt_annulation_masse(&d, &cmd(true, None))
        .await
        .unwrap();
    assert!(ap.apercu);
    assert_eq!((ap.nb_seances, ap.nb_classes), (2, 2), "{ap:?}");
    assert_eq!(x.nb_events("edt.annulation_masse").await, 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM ent_edt_exception WHERE etablissement_id = $1"
        )
        .bind(x.e())
        .fetch_one(x.ent.pool())
        .await
        .unwrap(),
        0
    );
    // Droits : l'administratif (edt.gerer) et le professeur n'ont pas edt.annuler_masse ; la direction oui.
    assert!(matches!(
        x.ent
            .edt_annulation_masse(&adm(&x.cast), &cmd(true, None))
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(matches!(
        x.ent
            .edt_annulation_masse(&prof_a(&x.cast, 0), &cmd(false, Some("k")))
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    // Validations.
    let mut mauvais = cmd(true, None);
    mauvais.motif_public = "  ".into();
    assert!(matches!(
        x.ent.edt_annulation_masse(&d, &mauvais).await.unwrap_err(),
        EntDbError::Domain(DomainError::ValeurInvalide)
    ));
    mauvais = cmd(true, None);
    mauvais.au = x.lundi - Duration::days(1);
    assert!(x.ent.edt_annulation_masse(&d, &mauvais).await.is_err());
    mauvais = cmd(true, None);
    mauvais.portee = "classes".into();
    mauvais.cibles = vec![Uuid::new_v4()];
    assert!(matches!(
        x.ent.edt_annulation_masse(&d, &mauvais).await.unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    // Portée par classe : seulement la 4B.
    let c4b: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_classe WHERE etablissement_id = $1 AND code = '4B' LIMIT 1",
    )
    .bind(x.e())
    .fetch_one(x.ent.pool())
    .await
    .unwrap();
    let mut par_classe = cmd(true, None);
    par_classe.portee = "classes".into();
    par_classe.cibles = vec![c4b];
    let ap4 = x.ent.edt_annulation_masse(&d, &par_classe).await.unwrap();
    assert_eq!((ap4.nb_seances, ap4.nb_classes), (1, 1));
    // Application avec clé d'idempotence : un lot, un événement ; le double envoi ne recrée rien.
    let r = x
        .ent
        .edt_annulation_masse(&d, &cmd(false, Some("greve-1")))
        .await
        .unwrap();
    assert!(!r.deja_applique);
    assert_eq!((r.nb_seances, r.nb_classes), (2, 2));
    let lot = r.lot_id.unwrap();
    let r2 = x
        .ent
        .edt_annulation_masse(&d, &cmd(false, Some("greve-1")))
        .await
        .unwrap();
    assert!(r2.deja_applique);
    assert_eq!(r2.lot_id, Some(lot));
    // Même clé, demande différente : refusée.
    let mut autre = cmd(false, Some("greve-1"));
    autre.motif_public = "Autre".into();
    assert!(matches!(
        x.ent.edt_annulation_masse(&d, &autre).await.unwrap_err(),
        EntDbError::Domain(DomainError::Conflit)
    ));
    assert_eq!(x.nb_events("edt.annulation_masse").await, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM ent_edt_annulation_masse WHERE etablissement_id = $1"
        )
        .bind(x.e())
        .fetch_one(x.ent.pool())
        .await
        .unwrap(),
        1
    );
    // Rien d'autre à annuler : pas de lot vide ; les séances sont annulées dans la vue.
    assert!(matches!(
        x.ent
            .edt_annulation_masse(&d, &cmd(false, Some("greve-2")))
            .await
            .unwrap_err(),
        EntDbError::Domain(DomainError::Conflit)
    ));
    let sem = x
        .ent
        .edt_semaine(&d, CibleEdt::Personnel(p1), x.lundi)
        .await
        .unwrap();
    assert!(sem.seances.iter().all(|s| s.statut == "annulee"), "{sem:?}");
    let _ = (c_4b, c_5a);
    // Familles : UNE notification par destinataire et par lot, motif public seulement.
    distribuer(&x.ent, x.e()).await;
    distribuer(&x.ent, x.e()).await;
    let harry = &x.cast.eleves[0];
    let n = x.notifs(harry.parent.user_id, "edt_annulation_masse").await;
    assert_eq!(n.len(), 1, "{n:?}");
    assert!(n[0].corps.contains("Grève") && n[0].corps.starts_with("1 cours est annulé"));
    let ron = x.cast.eleves.iter().find(|e| e.prenom == "Ron").unwrap();
    assert_eq!(
        x.notifs(ron.parent.user_id, "edt_annulation_masse")
            .await
            .len(),
        1
    );
    // Une classe sans cours dans le lot (3C) n'est pas notifiée.
    let neville = x
        .cast
        .eleves
        .iter()
        .find(|e| e.prenom == "Neville")
        .unwrap();
    assert!(
        x.notifs(neville.parent.user_id, "edt_annulation_masse")
            .await
            .is_empty()
    );
    // Vue famille des changements : annulation du lot avec motif public, sans motif interne.
    let parent = acteur(harry.parent.user_id, Role::Parent, &x.cast);
    let ch = x
        .ent
        .edt_changements(&parent, harry.id, x.lundi, x.lundi + Duration::days(13))
        .await
        .unwrap();
    assert_eq!(ch.len(), 1, "{ch:?}");
    assert_eq!(
        (ch[0].nature.as_str(), ch[0].motif_public.as_deref()),
        ("annulation", Some("Grève"))
    );
    assert!(matches!(
        x.ent
            .edt_changements(&parent, x.cast.eleve_tom_id, x.lundi, x.lundi)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
}

// ───────────────────────── Déplacement et changement de salle ─────────────────────────

#[tokio::test]
async fn deplacement_changement_de_salle_et_conflits() {
    let Some(x) = ctx().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (p1, p2) = (x.cast.profs[0].user_id, x.cast.profs[1].user_id);
    let d = dir(&x.cast);
    x.salle("S1").await;
    x.salle("S2").await;
    let cr = x
        .creneau(&x.saisie("4B", p1, 1, (14, 0), (15, 0), Some("S1"), "toutes"))
        .await;
    x.creneau(&x.saisie("5A", p2, 3, (14, 0), (15, 0), Some("S2"), "toutes"))
        .await;
    // Déplacement du lundi vers le mercredi 14 h : la salle S2 est prise, la 4B libre, p1 libre.
    let mut cmd = CmdDeplacement {
        creneau_id: cr,
        jour: x.jour(1),
        nouveau_jour: x.jour(3),
        heure_debut: t(14, 0),
        heure_fin: t(15, 0),
        salle: Some("S2".into()),
    };
    assert!(matches!(
        x.ent.edt_deplacer_seance(&d, &cmd).await.unwrap_err(),
        EntDbError::Domain(DomainError::Conflit)
    ));
    cmd.salle = Some("S1".into());
    let r = x.ent.edt_deplacer_seance(&d, &cmd).await.unwrap();
    assert!(!r.deja_fait);
    assert!(x.ent.edt_deplacer_seance(&d, &cmd).await.unwrap().deja_fait);
    assert_eq!(x.nb_events("edt.exception").await, 1);
    // Semaine : le lundi est « déplacé », le mercredi porte la séance ajoutée.
    let sem = x
        .ent
        .edt_semaine(&d, CibleEdt::Personnel(p1), x.lundi)
        .await
        .unwrap();
    let origine = sem.seances.iter().find(|s| s.jour == x.jour(1)).unwrap();
    assert_eq!(
        (origine.statut.as_str(), origine.deplace_vers),
        ("deplacee", Some(x.jour(3)))
    );
    let arrivee = sem.seances.iter().find(|s| s.jour == x.jour(3)).unwrap();
    assert_eq!(
        (arrivee.statut.as_str(), arrivee.deplace_depuis),
        ("ajoutee", Some(x.jour(1)))
    );
    // Le professeur est occupé au nouveau créneau : plus candidat à un remplacement simultané.
    x.creneau(&x.saisie("3C", p2, 4, (14, 0), (15, 0), None, "toutes"))
        .await;
    let cr_p2 = x
        .ent
        .edt_plans(x.e(), x.jour(4), x.jour(4))
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.plan.prof_user_id == p2)
        .unwrap()
        .plan
        .creneau_id;
    let c = x
        .ent
        .edt_candidats_remplacants(&d, cr_p2, x.jour(4), true)
        .await
        .unwrap();
    assert!(c.iter().any(|y| y.user_id == p1));
    // Un second déplacement vers la même heure du mercredi pour un autre cours de p1 est un conflit.
    let cr2 = x
        .creneau(&x.saisie("3C", p1, 2, (14, 0), (15, 0), None, "toutes"))
        .await;
    let conflit = CmdDeplacement {
        creneau_id: cr2,
        jour: x.jour(2),
        nouveau_jour: x.jour(3),
        heure_debut: t(14, 30),
        heure_fin: t(15, 30),
        salle: None,
    };
    assert!(matches!(
        x.ent.edt_deplacer_seance(&d, &conflit).await.unwrap_err(),
        EntDbError::Domain(DomainError::Conflit)
    ));
    // Jour fermé : refus.
    x.ent
        .ref_creer_vacances(&d, "Pont", "vacances", x.jour(5), x.jour(5))
        .await
        .unwrap();
    let ferme = CmdDeplacement {
        creneau_id: cr2,
        jour: x.jour(2),
        nouveau_jour: x.jour(5),
        heure_debut: t(14, 0),
        heure_fin: t(15, 0),
        salle: None,
    };
    assert!(matches!(
        x.ent.edt_deplacer_seance(&d, &ferme).await.unwrap_err(),
        EntDbError::Domain(DomainError::Conflit)
    ));
    // Heures incohérentes.
    let mauvais = CmdDeplacement {
        heure_fin: t(13, 0),
        ..cmd.clone()
    };
    assert!(matches!(
        x.ent.edt_deplacer_seance(&d, &mauvais).await.unwrap_err(),
        EntDbError::Domain(DomainError::ValeurInvalide)
    ));
    // Notification : déplacement annoncé avec la nouvelle date ; la vue famille le montre.
    distribuer(&x.ent, x.e()).await;
    let harry = &x.cast.eleves[0];
    let n = x.notifs(harry.parent.user_id, "edt_deplacement").await;
    assert_eq!(n.len(), 1, "{n:?}");
    assert!(
        n[0].corps
            .contains(&x.jour(3).format("%d/%m/%Y").to_string())
    );
    let ch = x
        .ent
        .edt_changements(
            &acteur(harry.parent.user_id, Role::Parent, &x.cast),
            harry.id,
            x.lundi,
            x.lundi + Duration::days(6),
        )
        .await
        .unwrap();
    assert_eq!(
        ch.iter()
            .filter(|c| c.nature == "deplacement" && c.nouveau_jour == Some(x.jour(3)))
            .count(),
        1
    );
    // Lever le déplacement : le lundi redevient normal, la séance du mercredi disparaît.
    assert!(x.ent.edt_lever_exception(&d, cr, x.jour(1)).await.unwrap());
    let sem = x
        .ent
        .edt_semaine(&d, CibleEdt::Personnel(p1), x.lundi)
        .await
        .unwrap();
    assert!(sem.seances.iter().all(|s| s.statut == "prevue"), "{sem:?}");
    assert_eq!(sem.seances.iter().filter(|s| s.creneau_id == cr).count(), 1);
    // Changement de salle : S2 occupée le mercredi par la 5A, libre le lundi.
    assert!(matches!(
        x.ent
            .edt_changer_salle(&d, cr, x.jour(1), "inconnue")
            .await
            .unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    let r = x
        .ent
        .edt_changer_salle(&d, cr, x.jour(1), "S2")
        .await
        .unwrap();
    assert!(!r.deja_fait);
    assert!(
        x.ent
            .edt_changer_salle(&d, cr, x.jour(1), "S2")
            .await
            .unwrap()
            .deja_fait
    );
    let sem = x
        .ent
        .edt_semaine(&d, CibleEdt::Salle("S2".into()), x.lundi)
        .await
        .unwrap();
    assert!(
        sem.seances
            .iter()
            .any(|s| s.creneau_id == cr && s.jour == x.jour(1))
    );
    distribuer(&x.ent, x.e()).await;
    assert_eq!(x.notifs(harry.parent.user_id, "edt_salle").await.len(), 1);
}

// ───────────────────────── Délégations (ADM-04) ─────────────────────────

async fn a_edt(ent: &PgEnt, etab: Uuid, user: Uuid) -> bool {
    ent.droits_effectifs(etab, user, Role::Prof.as_str())
        .await
        .unwrap()
        .iter()
        .any(|y| y.action == Action::EdtGerer)
}

#[tokio::test]
async fn delegations_datees_expiration_revocation_et_anti_escalade() {
    let Some(x) = ctx().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let d = dir(&x.cast);
    let delegue = x.cast.profs[3].user_id;
    let aujourdhui = today_utc();
    let droit = |a: Action, p: Portee, cibles: Vec<Uuid>| DroitDelegue {
        action: a.code().into(),
        portee: p.code().into(),
        cibles,
    };
    let saisie = |droits: Vec<DroitDelegue>, fin: NaiveDate| DelegationSaisie {
        delegue_user_id: delegue,
        droits,
        debut: None,
        fin,
        motif: "Sortie de niveau".into(),
    };
    assert!(!a_edt(&x.ent, x.e(), delegue).await);
    // Garde-fous de saisie : fin obligatoire dans le futur, durée max, droits non délégables,
    // portée non admise, doublon, vide, à soi-même, famille, hors établissement.
    let invalide = |e: EntDbError| matches!(e, EntDbError::Domain(DomainError::ValeurInvalide));
    let edt = || vec![droit(Action::EdtGerer, Portee::Etablissement, vec![])];
    assert!(invalide(
        x.ent
            .delegation_accorder(&d, &saisie(edt(), aujourdhui - Duration::days(1)))
            .await
            .unwrap_err()
    ));
    assert!(invalide(
        x.ent
            .delegation_accorder(&d, &saisie(edt(), aujourdhui + Duration::days(400)))
            .await
            .unwrap_err()
    ));
    for interdit in [
        Action::RolesGerer,
        Action::SecuritePolitique,
        Action::DelegationsGerer,
    ] {
        assert!(
            invalide(
                x.ent
                    .delegation_accorder(
                        &d,
                        &saisie(
                            vec![droit(interdit, Portee::Etablissement, vec![])],
                            aujourdhui + Duration::days(3)
                        )
                    )
                    .await
                    .unwrap_err()
            ),
            "{interdit:?}"
        );
    }
    assert!(invalide(
        x.ent
            .delegation_accorder(
                &d,
                &saisie(
                    vec![droit(Action::EdtGerer, Portee::MesClasses, vec![])],
                    aujourdhui + Duration::days(3)
                )
            )
            .await
            .unwrap_err()
    ));
    assert!(invalide(
        x.ent
            .delegation_accorder(&d, &saisie(vec![], aujourdhui + Duration::days(3)))
            .await
            .unwrap_err()
    ));
    assert!(invalide(
        x.ent
            .delegation_accorder(
                &d,
                &saisie(
                    vec![
                        droit(Action::EdtGerer, Portee::Etablissement, vec![]),
                        droit(Action::EdtGerer, Portee::Etablissement, vec![])
                    ],
                    aujourdhui + Duration::days(3)
                )
            )
            .await
            .unwrap_err()
    ));
    let mut soi = saisie(edt(), aujourdhui + Duration::days(3));
    soi.delegue_user_id = d.user_id;
    assert!(invalide(
        x.ent.delegation_accorder(&d, &soi).await.unwrap_err()
    ));
    let mut parent = saisie(edt(), aujourdhui + Duration::days(3));
    parent.delegue_user_id = x.cast.parent_lea.user_id;
    assert!(invalide(
        x.ent.delegation_accorder(&d, &parent).await.unwrap_err()
    ));
    let mut etranger = saisie(edt(), aujourdhui + Duration::days(3));
    etranger.delegue_user_id = Uuid::new_v4();
    assert!(matches!(
        x.ent.delegation_accorder(&d, &etranger).await.unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    // Cible « désignée » inexistante / d'ailleurs : introuvable.
    let faux = saisie(
        vec![droit(
            Action::AbsencesLire,
            Portee::Designes,
            vec![Uuid::new_v4()],
        )],
        aujourdhui + Duration::days(3),
    );
    assert!(matches!(
        x.ent.delegation_accorder(&d, &faux).await.unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    // Droits : ni l'administratif (edt.gerer mais pas delegations.gerer) ni un professeur ne délèguent.
    assert!(matches!(
        x.ent
            .delegation_accorder(
                &adm(&x.cast),
                &saisie(edt(), aujourdhui + Duration::days(3))
            )
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(matches!(
        x.ent
            .delegation_accorder(
                &prof_a(&x.cast, 0),
                &saisie(edt(), aujourdhui + Duration::days(3))
            )
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(matches!(
        x.ent
            .delegations_lister(&adm(&x.cast), false)
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    // Anti-escalade : la direction ne délègue pas un droit qu'elle n'a pas (comptabilité).
    let dir_sans = x
        .ent
        .delegation_accorder(
            &d,
            &saisie(
                vec![droit(
                    Action::FacturationGerer,
                    Portee::Etablissement,
                    vec![],
                )],
                aujourdhui + Duration::days(3),
            ),
        )
        .await;
    assert!(
        dir_sans.is_ok(),
        "la direction détient la facturation (modèle par défaut) : {dir_sans:?}"
    );
    // Un droit que la direction n'a pas (s'il en existe un au catalogue) ne se délègue pas.
    let manquant = Action::TOUTES.into_iter().find(|a| {
        !ent_auth::modele_par_defaut(Role::Direction)
            .iter()
            .any(|(x, _)| x == a)
            && a.portees_admises().contains(&Portee::Etablissement)
    });
    if let Some(m) = manquant {
        let r = x
            .ent
            .delegation_accorder(
                &d,
                &saisie(
                    vec![droit(m, Portee::Etablissement, vec![])],
                    aujourdhui + Duration::days(3),
                ),
            )
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{m:?} : {r:?}");
    }
    // Délégation valable : le délégué a edt.gerer, la liste et « mes délégations » la montrent.
    let dl = x
        .ent
        .delegation_accorder(&d, &saisie(edt(), aujourdhui + Duration::days(2)))
        .await
        .unwrap();
    assert_eq!(
        (dl.etat.as_str(), dl.source.as_str()),
        ("active", "manuelle")
    );
    assert!(a_edt(&x.ent, x.e(), delegue).await);
    let ma = acteur(delegue, Role::Prof, &x.cast);
    assert!(
        x.ent
            .edt_creneau_creer(
                &ma,
                &x.saisie("4B", delegue, 2, (17, 0), (18, 0), None, "toutes"),
                true
            )
            .await
            .is_ok()
    );
    assert_eq!(x.ent.delegations_mes(&ma).await.unwrap().len(), 2); // facturation + edt
    assert!(
        x.ent
            .delegations_lister(&d, true)
            .await
            .unwrap()
            .iter()
            .any(|y| y.id == dl.id)
    );
    assert_eq!(x.nb_events("delegation.accordee").await, 2); // facturation + edt
    // Pas de chaîne : le délégué ne peut pas déléguer (delegations.gerer non délégable).
    assert!(matches!(
        x.ent
            .delegation_accorder(&ma, &saisie(edt(), aujourdhui + Duration::days(1)))
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    // Révocation immédiate : droit éteint, événement, idempotent.
    let rv = x.ent.delegation_revoquer(&d, dl.id).await.unwrap();
    assert_eq!(rv.etat, "revoquee");
    assert!(!a_edt(&x.ent, x.e(), delegue).await);
    assert!(matches!(
        x.ent
            .edt_creneau_creer(
                &ma,
                &x.saisie("4B", delegue, 2, (17, 0), (18, 0), None, "toutes"),
                true
            )
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    x.ent.delegation_revoquer(&d, dl.id).await.unwrap();
    assert_eq!(x.nb_events("delegation.revoquee").await, 1);

    // Expiration : une délégation échue n'agit plus (simulation du temps par la date de fin) ;
    // le balayage la marque et publie `delegation.expiree` UNE fois.
    let dl2 = x
        .ent
        .delegation_accorder(&d, &saisie(edt(), aujourdhui + Duration::days(1)))
        .await
        .unwrap();
    assert!(a_edt(&x.ent, x.e(), delegue).await);
    sqlx::query("UPDATE ent_delegation SET debut = $2, fin = $3 WHERE id = $1")
        .bind(dl2.id)
        .bind(aujourdhui - Duration::days(5))
        .bind(aujourdhui - Duration::days(1))
        .execute(x.ent.pool())
        .await
        .unwrap();
    assert!(
        !a_edt(&x.ent, x.e(), delegue).await,
        "droit temporaire expiré toujours actif"
    );
    assert!(matches!(
        x.ent
            .edt_creneau_creer(
                &ma,
                &x.saisie("4B", delegue, 2, (17, 0), (18, 0), None, "toutes"),
                true
            )
            .await
            .unwrap_err(),
        EntDbError::Forbidden
    ));
    let vue = x.ent.delegations_lister(&d, false).await.unwrap();
    assert_eq!(vue.iter().find(|y| y.id == dl2.id).unwrap().etat, "expiree");
    assert!(
        !x.ent
            .delegations_lister(&d, true)
            .await
            .unwrap()
            .iter()
            .any(|y| y.id == dl2.id)
    );
    let n1 = x.ent.delegations_balayer(aujourdhui).await.unwrap();
    assert!(n1 >= 1);
    assert_eq!(
        x.ent.delegations_balayer(aujourdhui).await.unwrap(),
        0,
        "le second balayage ne republie rien"
    );
    assert_eq!(x.nb_events("delegation.expiree").await, 1);
    // Début futur : pas encore active.
    let mut futur = saisie(edt(), aujourdhui + Duration::days(10));
    futur.debut = Some(aujourdhui + Duration::days(5));
    let dl3 = x.ent.delegation_accorder(&d, &futur).await.unwrap();
    assert_eq!(dl3.etat, "a_venir");
    assert!(!a_edt(&x.ent, x.e(), delegue).await);
    // Isolation : la délégation d'un établissement n'agit pas dans un autre.
    let c2 = x.ent.seed_labo_cast().await.unwrap();
    assert!(
        !x.ent
            .droits_effectifs(c2.etablissement_id, delegue, Role::Prof.as_str())
            .await
            .unwrap()
            .iter()
            .any(|y| y.action == Action::EdtGerer)
    );
    let d2 = acteur(c2.direction.user_id, Role::Direction, &c2);
    assert!(
        x.ent
            .delegations_lister(&d2, false)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        x.ent.delegation_revoquer(&d2, dl3.id).await.unwrap_err(),
        EntDbError::Domain(DomainError::NotFound)
    ));
    // Une délégation révoquée après coup ne ressuscite pas.
    assert_eq!(
        x.ent.delegation_revoquer(&d, dl3.id).await.unwrap().etat,
        "revoquee"
    );
}
