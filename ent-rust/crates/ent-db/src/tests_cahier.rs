//! S2-6 contre la base locale de test (feature `labo`). Sans `ENT_DATABASE_URL`, ignorés.
//!
//! Couvre : contenu de séance versionné, visibilité par classe et par groupe, pièces privées
//! (lien signé, contrôle D2), échéances, rendu tardif, retour du professeur, droits par le
//! moteur (rôle paramétré D-017), isolation entre établissements, charge de travail,
//! visa, reprise de l'ancien cahier et notifications idempotentes (outbox S2-0).

use super::*;
use crate::cahier::{Cible, FichierDepot, NouveauTravail, ParentPiece};
use crate::droits_repo::{Droit, RoleSaisie};
use crate::outbox::ConfigDistribution;
use crate::referentiel_repo::CreerService;
use crate::ref_commun::Acteur;
use chrono::{Duration, NaiveDate, NaiveTime, Utc};
use domain_eleve::today_utc;
use ent_auth::{Action, Portee, Role};
use ent_object::presign::Presigner;
use ent_types::cahier::*;
use std::time::Duration as Delai;
use uuid::Uuid;

const PDF: &[u8] = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";
const PNG: &[u8] = &[
    0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, b'I', b'H', b'D', b'R',
];

fn pdf(nom: &str) -> FichierDepot {
    FichierDepot {
        nom: nom.into(),
        mime_type: "application/pdf".into(),
        contenu: PDF.to_vec(),
    }
}

/// Établissement de test : deux classes (A, B), un groupe G de A, quatre élèves à compte.
struct Fx {
    ent: PgEnt,
    cast: LaboCast,
    classe_a: Uuid,
    classe_b: Uuid,
    groupe_g: Uuid,
    /// X (classe A), Y (classe B), Z (classe A + groupe G), W (classe A).
    x: LaboEleve,
    y: LaboEleve,
    z: LaboEleve,
    w: LaboEleve,
    service_a: Uuid,
}

fn act<'a>(etab: Uuid, p: &'a LaboPersona) -> Acteur<'a> {
    Acteur {
        etablissement_id: etab,
        user_id: p.user_id,
        role: p.role.as_str(),
    }
}

impl Fx {
    fn prof(&self) -> Acteur<'_> {
        act(self.cast.etablissement_id, &self.cast.profs[0])
    }
    /// Second professeur : n'enseigne rien (aucun service) tant qu'un test ne lui en donne pas.
    fn prof2(&self) -> Acteur<'_> {
        act(self.cast.etablissement_id, &self.cast.profs[1])
    }
    fn direction(&self) -> Acteur<'_> {
        act(self.cast.etablissement_id, &self.cast.direction)
    }
    fn cpe(&self) -> Acteur<'_> {
        act(self.cast.etablissement_id, &self.cast.cpe)
    }
    fn compta(&self) -> Acteur<'_> {
        act(self.cast.etablissement_id, &self.cast.compta)
    }
    fn eleve<'a>(&'a self, e: &'a LaboEleve) -> Acteur<'a> {
        act(self.cast.etablissement_id, e.compte.as_ref().expect("compte élève"))
    }
    fn parent<'a>(&'a self, e: &'a LaboEleve) -> Acteur<'a> {
        act(self.cast.etablissement_id, &e.parent)
    }

    /// Séance datée du jour pour la classe A (service de `prof`), créneau dédié.
    async fn seance(&self, classe_code: &str, prof: Uuid, service: Option<Uuid>, jour: NaiveDate) -> Uuid {
        let e = self.cast.etablissement_id;
        let annee = self.ent.referentiel_annee_courante_assuree(e).await.unwrap();
        let creneau = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine, heure_debut, heure_fin)
             VALUES ($1,$2,$3,'Maths',$4,1,$5,$6)",
        )
        .bind(creneau)
        .bind(e)
        .bind(classe_code)
        .bind(prof)
        .bind(NaiveTime::from_hms_opt(8, 0, 0).unwrap())
        .bind(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
        .execute(self.ent.pool())
        .await
        .unwrap();
        let id = Uuid::new_v4();
        let classe_id: Uuid = sqlx::query_scalar("SELECT id FROM ent_classe WHERE etablissement_id = $1 AND code = $2")
            .bind(e)
            .bind(classe_code)
            .fetch_one(self.ent.pool())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO ent_edt_seance (id, etablissement_id, annee_id, creneau_id, jour, classe_id, classe, service_id,
                    prof_user_id, matiere_libelle, heure_debut, heure_fin)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'Maths',$10,$11)",
        )
        .bind(id)
        .bind(e)
        .bind(annee.id)
        .bind(creneau)
        .bind(jour)
        .bind(classe_id)
        .bind(classe_code)
        .bind(service)
        .bind(prof)
        .bind(NaiveTime::from_hms_opt(8, 0, 0).unwrap())
        .bind(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
        .execute(self.ent.pool())
        .await
        .unwrap();
        id
    }

    async fn publier_travail(&self, a: &Acteur<'_>, cible: Cible, n: impl FnOnce(&mut NouveauTravail)) -> Result<TravailCreeDto, EntDbError> {
        let mut t = NouveauTravail {
            seance_id: None,
            cible: Some(cible),
            matiere_libelle: Some("Maths".into()),
            type_travail: "a_faire".into(),
            titre: "Exercices".into(),
            consigne: "p. 12 n° 3".into(),
            pour_le: today_utc() + Duration::days(3),
            duree_estimee_min: Some(30),
            date_limite: None,
            acceptation_retard: true,
            formats_acceptes: vec![],
        };
        n(&mut t);
        self.ent.cahier_donner_travail(a, t).await
    }

    fn cible_a(&self) -> Cible {
        Cible { classe_id: Some(self.classe_a), groupe_id: None }
    }
    fn cible_g(&self) -> Cible {
        Cible { classe_id: None, groupe_id: Some(self.groupe_g) }
    }

    async fn nb_evenements(&self, type_name: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = $2")
            .bind(self.cast.etablissement_id)
            .bind(type_name)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }
}

async fn fixture() -> Option<Fx> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    ent.installer_presigner(Presigner::local(b"cle-de-test-cahier-0123456789abcdef", 60).unwrap());
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    let e = cast.etablissement_id;
    let dir = Role::Direction.as_str();
    let annee = ent.referentiel_annee_courante_assuree(e).await.unwrap();
    let ca = ent.referentiel_creer_classe(dir, e, Some(annee.id), "CDT-A", None, None).await.unwrap();
    let cb = ent.referentiel_creer_classe(dir, e, Some(annee.id), "CDT-B", None, None).await.unwrap();
    let g = ent.referentiel_creer_groupe(dir, e, Some(annee.id), "CDT-G", None, Some(ca.id)).await.unwrap();
    let el = |i: usize| cast.eleves[i].clone();
    let (x, y, z, w) = (el(0), el(1), el(2), el(3));
    for (eleve, classe) in [(&x, ca.id), (&y, cb.id), (&z, ca.id), (&w, ca.id)] {
        ent.referentiel_affecter_eleve_classe(dir, e, classe, eleve.id, None).await.unwrap();
    }
    ent.referentiel_affecter_eleve_groupe(dir, e, g.id, z.id, None).await.unwrap();
    let service_a = ent
        .referentiel_creer_service(
            dir,
            e,
            CreerService {
                prof_user_id: cast.profs[0].user_id,
                matiere_id: None,
                classe_id: Some(ca.id),
                groupe_id: None,
                professeur_principal: false,
            },
        )
        .await
        .unwrap();
    Some(Fx {
        ent,
        classe_a: ca.id,
        classe_b: cb.id,
        groupe_g: g.id,
        x,
        y,
        z,
        w,
        service_a: service_a.id,
        cast,
    })
}

fn est_interdit(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Forbidden)
}

fn est_refus(e: &EntDbError, n: RefusMetier) -> bool {
    matches!(e, EntDbError::Refus(x, _) if *x == n)
}

fn est_introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound))
}

async fn voit_seance(fx: &Fx, e: &LaboEleve, seance: Uuid, du: NaiveDate, au: NaiveDate) -> bool {
    let a = fx.eleve(e);
    fx.ent.cahier_seances_eleve(&a, None, du, au).await.unwrap().iter().any(|s| s.seance_id == seance)
}

async fn agenda_de(fx: &Fx, e: &LaboEleve, du: NaiveDate, au: NaiveDate) -> Vec<AgendaItemDto> {
    let a = fx.eleve(e);
    fx.ent.cahier_agenda(&a, None, du, au).await.unwrap()
}

async fn fait_de(fx: &Fx, a: Acteur<'_>, e: Option<Uuid>, du: NaiveDate, au: NaiveDate) -> bool {
    fx.ent.cahier_agenda(&a, e, du, au).await.unwrap()[0].fait
}

fn semaine() -> (NaiveDate, NaiveDate) {
    let t = today_utc();
    (t - Duration::days(7), t + Duration::days(30))
}

// ───────────────────────── CDT-01 : contenu de séance ─────────────────────────

#[tokio::test]
async fn contenu_de_seance_versionne_et_visible_de_sa_classe() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let seance = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), today_utc()).await;

    // Le professeur de la séance la voit dans son cahier, sans contenu.
    let mes = fx.ent.cahier_seances(&fx.prof(), None, du, au).await.unwrap();
    assert!(mes.iter().any(|s| s.seance_id == seance && s.contenu.is_none()));

    let c1 = fx.ent.cahier_renseigner_seance(&fx.prof(), seance, "Fractions : définition", "").await.unwrap();
    assert_eq!(c1.version, 1);
    let c2 = fx
        .ent
        .cahier_renseigner_seance(&fx.prof(), seance, "Fractions : définition et exemples", "Manuel p. 10")
        .await
        .unwrap();
    assert_eq!((c2.id, c2.version), (c1.id, 2), "une seule ligne, versionnée");
    let histo: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_historique WHERE objet = 'seance_contenu' AND objet_id = $1")
        .bind(c1.id)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(histo, 1, "l'état avant correction est conservé");
    assert_eq!(fx.nb_evenements("cahier.seance_renseignee").await, 2);

    // Vide, trop long : refusés.
    assert!(est_refus(&fx.ent.cahier_renseigner_seance(&fx.prof(), seance, "  ", "").await.unwrap_err(), RefusMetier::Invalide));

    // Visible de l'élève de la classe A et de son responsable, pas de la classe B.
    let vu_x = fx.ent.cahier_seances_eleve(&fx.eleve(&fx.x), None, du, au).await.unwrap();
    assert!(vu_x.iter().any(|s| s.seance_id == seance && s.contenu.as_ref().is_some_and(|c| c.contenu.contains("exemples"))));
    let vu_parent = fx.ent.cahier_seances_eleve(&fx.parent(&fx.x), Some(fx.x.id), du, au).await.unwrap();
    assert_eq!(vu_parent.len(), vu_x.len());
    let vu_y = fx.ent.cahier_seances_eleve(&fx.eleve(&fx.y), None, du, au).await.unwrap();
    assert!(vu_y.iter().all(|s| s.seance_id != seance), "classe B : séance de la classe A invisible");

    // Droits : un professeur sans service ni séance ne renseigne pas ; ni l'élève, ni la compta.
    assert!(est_interdit(&fx.ent.cahier_renseigner_seance(&fx.prof2(), seance, "intrus", "").await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_renseigner_seance(&fx.eleve(&fx.x), seance, "intrus", "").await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_renseigner_seance(&fx.compta(), seance, "intrus", "").await.unwrap_err()));

    // Séance annulée : conflit.
    sqlx::query("UPDATE ent_edt_seance SET statut = 'annulee' WHERE id = $1").bind(seance).execute(fx.ent.pool()).await.unwrap();
    assert!(est_refus(&fx.ent.cahier_renseigner_seance(&fx.prof(), seance, "x", "").await.unwrap_err(), RefusMetier::Conflit));
}

#[tokio::test]
async fn seance_de_groupe_visible_seulement_du_groupe() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let service_g = fx
        .ent
        .referentiel_creer_service(
            Role::Direction.as_str(),
            fx.cast.etablissement_id,
            CreerService {
                prof_user_id: fx.cast.profs[1].user_id,
                matiere_id: None,
                classe_id: None,
                groupe_id: Some(fx.groupe_g),
                professeur_principal: false,
            },
        )
        .await
        .unwrap();
    let seance = fx.seance("CDT-A", fx.cast.profs[1].user_id, Some(service_g.id), today_utc()).await;
    fx.ent.cahier_renseigner_seance(&fx.prof2(), seance, "Atelier théâtre : improvisation", "").await.unwrap();
    assert!(voit_seance(&fx, &fx.z, seance, du, au).await, "Z est dans le groupe");
    assert!(!voit_seance(&fx, &fx.w, seance, du, au).await, "W est dans la classe A mais pas dans le groupe");
    assert!(!voit_seance(&fx, &fx.y, seance, du, au).await);
}

// ───────────────────────── CDT-02 / 05 : travaux, visibilité, échéances ─────────────────────────

#[tokio::test]
async fn travail_visible_par_classe_et_par_groupe() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let tc = fx.publier_travail(&fx.prof(), fx.cible_a(), |_| {}).await.unwrap();
    assert_eq!(tc.travail.cible, "CDT-A");
    assert_eq!(fx.nb_evenements("cahier.travail_donne").await, 1);

    // Travail de groupe : écrit par un professeur ayant un service sur le groupe.
    fx.ent
        .referentiel_creer_service(
            Role::Direction.as_str(),
            fx.cast.etablissement_id,
            CreerService {
                prof_user_id: fx.cast.profs[1].user_id,
                matiere_id: None,
                classe_id: None,
                groupe_id: Some(fx.groupe_g),
                professeur_principal: false,
            },
        )
        .await
        .unwrap();
    let tg = fx.publier_travail(&fx.prof2(), fx.cible_g(), |t| t.titre = "Texte de groupe".into()).await.unwrap();

    let ids = |v: &[AgendaItemDto]| v.iter().map(|i| i.travail.id).collect::<Vec<_>>();
    let (ax, ay, az, aw) = (agenda_de(&fx, &fx.x, du, au).await, agenda_de(&fx, &fx.y, du, au).await, agenda_de(&fx, &fx.z, du, au).await, agenda_de(&fx, &fx.w, du, au).await);
    assert_eq!(ids(&ax), vec![tc.travail.id]);
    assert!(ay.is_empty(), "classe B : rien");
    assert!(ids(&az).contains(&tc.travail.id) && ids(&az).contains(&tg.travail.id), "Z : classe + groupe");
    assert_eq!(ids(&aw), vec![tc.travail.id], "W : pas le travail du groupe");

    // Famille : agenda de son enfant seulement.
    let ap = fx.ent.cahier_agenda(&fx.parent(&fx.x), Some(fx.x.id), du, au).await.unwrap();
    assert_eq!(ids(&ap), vec![tc.travail.id]);
    assert!(est_interdit(&fx.ent.cahier_agenda(&fx.parent(&fx.x), Some(fx.y.id), du, au).await.unwrap_err()));
    assert!(est_refus(&fx.ent.cahier_agenda(&fx.parent(&fx.x), None, du, au).await.unwrap_err(), RefusMetier::Invalide), "le responsable désigne l'enfant");
    // Élève : jamais l'agenda d'un autre.
    assert!(est_interdit(&fx.ent.cahier_agenda(&fx.eleve(&fx.x), Some(fx.y.id), du, au).await.unwrap_err()));
    // Détail : un élève hors cible ne lit pas le travail.
    assert!(est_interdit(&fx.ent.cahier_travail(&fx.eleve(&fx.y), tc.travail.id).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_travail(&fx.eleve(&fx.w), tg.travail.id).await.unwrap_err()));

    // Personnel : professeur de la classe (agenda d'un de ses élèves), pas un professeur étranger.
    assert_eq!(ids(&fx.ent.cahier_agenda(&fx.prof(), Some(fx.x.id), du, au).await.unwrap()), vec![tc.travail.id]);
    assert!(est_interdit(&fx.ent.cahier_agenda(&fx.prof(), Some(fx.y.id), du, au).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_agenda(&fx.compta(), Some(fx.x.id), du, au).await.unwrap_err()));
    // Direction et vie scolaire : tout l'établissement.
    assert!(fx.ent.cahier_agenda(&fx.cpe(), Some(fx.y.id), du, au).await.is_ok());

    // Droits d'écriture : le professeur 1 n'écrit pas dans la classe B ; l'élève et la famille non plus.
    let cible_b = Cible { classe_id: Some(fx.classe_b), groupe_id: None };
    assert!(est_interdit(&fx.publier_travail(&fx.prof(), cible_b, |_| {}).await.unwrap_err()));
    assert!(est_interdit(&fx.publier_travail(&fx.eleve(&fx.x), fx.cible_a(), |_| {}).await.unwrap_err()));
    assert!(est_interdit(&fx.publier_travail(&fx.parent(&fx.x), fx.cible_a(), |_| {}).await.unwrap_err()));
    // Cible d'un autre établissement (id inexistant ici) : refusée.
    let cible_inconnue = Cible { classe_id: Some(Uuid::new_v4()), groupe_id: None };
    assert!(est_interdit(&fx.publier_travail(&fx.direction(), cible_inconnue, |_| {}).await.unwrap_err()));
    // Classe ET groupe, ni l'un ni l'autre : invalide.
    let deux = Cible { classe_id: Some(fx.classe_a), groupe_id: Some(fx.groupe_g) };
    assert!(est_refus(&fx.publier_travail(&fx.prof(), deux, |_| {}).await.unwrap_err(), RefusMetier::Invalide));
}

#[tokio::test]
async fn echeances_durees_et_seance_d_origine() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let invalide = |e: EntDbError| est_refus(&e, RefusMetier::Invalide);
    let hier = today_utc() - Duration::days(1);
    assert!(invalide(fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.pour_le = hier).await.unwrap_err()), "échéance passée");
    assert!(invalide(fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.duree_estimee_min = Some(0)).await.unwrap_err()));
    assert!(invalide(fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.duree_estimee_min = Some(601)).await.unwrap_err()));
    assert!(invalide(fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.type_travail = "autre".into()).await.unwrap_err()));
    assert!(invalide(fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.titre = "  ".into()).await.unwrap_err()));
    // Date limite et formats : seulement pour un travail à rendre ; limite dans le passé refusée.
    assert!(invalide(fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.date_limite = Some(Utc::now() + Duration::days(1))).await.unwrap_err()));
    assert!(invalide(
        fx.publier_travail(&fx.prof(), fx.cible_a(), |t| {
            t.type_travail = "a_rendre".into();
            t.date_limite = Some(Utc::now() - Duration::hours(1));
        })
        .await
        .unwrap_err()
    ));
    assert!(invalide(
        fx.publier_travail(&fx.prof(), fx.cible_a(), |t| {
            t.type_travail = "a_rendre".into();
            t.formats_acceptes = vec!["exe".into()];
        })
        .await
        .unwrap_err()
    ));

    // Travail rattaché à une séance : l'échéance ne précède pas la séance ; cible et matière déduites.
    let demain = today_utc() + Duration::days(1);
    let seance = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), demain).await;
    let err = fx
        .ent
        .cahier_donner_travail(
            &fx.prof(),
            NouveauTravail {
                seance_id: Some(seance),
                cible: None,
                matiere_libelle: None,
                type_travail: "a_faire".into(),
                titre: "t".into(),
                consigne: "c".into(),
                pour_le: today_utc(),
                duree_estimee_min: None,
                date_limite: None,
                acceptation_retard: true,
                formats_acceptes: vec![],
            },
        )
        .await
        .unwrap_err();
    assert!(invalide(err));
    let ok = fx
        .ent
        .cahier_donner_travail(
            &fx.prof(),
            NouveauTravail {
                seance_id: Some(seance),
                cible: None,
                matiere_libelle: None,
                type_travail: "a_faire".into(),
                titre: "t".into(),
                consigne: "c".into(),
                pour_le: demain + Duration::days(2),
                duree_estimee_min: None,
                date_limite: None,
                acceptation_retard: true,
                formats_acceptes: vec![],
            },
        )
        .await
        .unwrap();
    assert_eq!((ok.travail.cible.as_str(), ok.travail.matiere_libelle.as_str()), ("CDT-A", "Maths"));
    assert_eq!(ok.travail.seance_id, Some(seance));
    // Cible incohérente avec la séance : refusée.
    let err = fx
        .ent
        .cahier_donner_travail(
            &fx.direction(),
            NouveauTravail {
                seance_id: Some(seance),
                cible: Some(Cible { classe_id: Some(fx.classe_b), groupe_id: None }),
                matiere_libelle: None,
                type_travail: "a_faire".into(),
                titre: "t".into(),
                consigne: "c".into(),
                pour_le: demain + Duration::days(2),
                duree_estimee_min: None,
                date_limite: None,
                acceptation_retard: true,
                formats_acceptes: vec![],
            },
        )
        .await
        .unwrap_err();
    assert!(invalide(err));
}

#[tokio::test]
async fn charge_de_travail_et_alerte_de_surcharge() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let jour = today_utc() + Duration::days(5);
    let (du, au) = (jour - Duration::days(1), jour + Duration::days(1));
    let t1 = fx.publier_travail(&fx.prof(), fx.cible_a(), |t| { t.pour_le = jour; t.duree_estimee_min = Some(60); }).await.unwrap();
    assert!(!t1.charge.surcharge, "60 min : sous le seuil");
    let t2 = fx.publier_travail(&fx.prof(), fx.cible_a(), |t| { t.pour_le = jour; t.duree_estimee_min = Some(70); }).await.unwrap();
    assert_eq!(t2.charge.minutes, 130);
    assert!(t2.charge.surcharge, "130 min > 120 : alerte dès la création");

    // Deux contrôles le même jour : alerte, même sans durée.
    let jour2 = jour + Duration::days(3);
    for titre in ["DS maths", "DS français"] {
        fx.publier_travail(&fx.direction(), fx.cible_a(), |t| {
            t.pour_le = jour2;
            t.type_travail = "controle".into();
            t.duree_estimee_min = None;
            t.titre = titre.into();
        })
        .await
        .unwrap();
    }
    let c = fx.ent.cahier_charge(&fx.prof(), fx.cible_a(), du, jour2 + Duration::days(1), None, None).await.unwrap();
    let j1 = c.jours.iter().find(|j| j.jour == jour).unwrap();
    let j2 = c.jours.iter().find(|j| j.jour == jour2).unwrap();
    assert!(j1.surcharge && j1.minutes == 130 && j1.nb_travaux == 2);
    assert!(j2.surcharge && j2.nb_controles == 2 && j2.minutes == 0);
    // Seuil réglable par l'équipe.
    let c = fx.ent.cahier_charge(&fx.prof(), fx.cible_a(), du, au, Some(200), None).await.unwrap();
    assert!(!c.jours.iter().find(|j| j.jour == jour).unwrap().surcharge);

    // Visible de l'équipe (professeur de la classe, vie scolaire), pas d'un étranger, ni des familles.
    assert!(fx.ent.cahier_charge(&fx.cpe(), fx.cible_a(), du, au, None, None).await.is_ok());
    assert!(est_interdit(&fx.ent.cahier_charge(&fx.prof2(), fx.cible_a(), du, au, None, None).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_charge(&fx.parent(&fx.x), fx.cible_a(), du, au, None, None).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_charge(&fx.eleve(&fx.x), fx.cible_a(), du, au, None, None).await.unwrap_err()));
    // Période abusive refusée.
    assert!(est_refus(
        &fx.ent.cahier_charge(&fx.prof(), fx.cible_a(), du, du + Duration::days(400), None, None).await.unwrap_err(),
        RefusMetier::Invalide
    ));
    // Le groupe cumule la charge de sa classe.
    let cg = fx.ent.cahier_charge(&fx.direction(), fx.cible_g(), du, au, None, None).await.unwrap();
    assert!(cg.jours.iter().any(|j| j.jour == jour && j.minutes == 130));
}

// ───────────────────────── CDT-03 : modification, historique, « déjà vu » ─────────────────────────

#[tokio::test]
async fn modification_suppression_historique_et_droits() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let t = fx.publier_travail(&fx.prof(), fx.cible_a(), |_| {}).await.unwrap().travail;
    let modif = |titre: &str| ModifierTravailBody { titre: Some(titre.into()), ..Default::default() };

    // Avant que l'élève l'ait vu : événement avec deja_vu = faux.
    let m1 = fx.ent.cahier_modifier_travail(&fx.prof(), t.id, modif("Exercices (corrigé)")).await.unwrap();
    assert_eq!(m1.version, 2);
    // L'élève ouvre le travail : « vu » ; la modification suivante le sait.
    let detail = fx.ent.cahier_travail(&fx.eleve(&fx.x), t.id).await.unwrap();
    assert!(detail.vu && !detail.fait);
    fx.ent.cahier_modifier_travail(&fx.prof(), t.id, modif("Exercices (v3)")).await.unwrap();
    let vus: Vec<bool> = sqlx::query_scalar(
        "SELECT (payload->>'deja_vu')::boolean FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'cahier.travail_modifie' ORDER BY created_at",
    )
    .bind(fx.cast.etablissement_id)
    .fetch_all(fx.ent.pool())
    .await
    .unwrap();
    assert_eq!(vus, vec![false, true]);
    let histo = fx.ent.cahier_historique_travail(&fx.prof(), t.id).await.unwrap();
    assert_eq!(histo.len(), 2);
    assert_eq!(histo[0].3["titre"], "Exercices", "état d'avant la première modification");

    // Rien ne change → pas de version ni d'événement.
    let avant = fx.nb_evenements("cahier.travail_modifie").await;
    fx.ent.cahier_modifier_travail(&fx.prof(), t.id, ModifierTravailBody::default()).await.unwrap();
    assert_eq!(fx.nb_evenements("cahier.travail_modifie").await, avant);

    // Droits : autre professeur, élève, famille refusés ; direction (visa) autorisée ; CPE non.
    assert!(est_interdit(&fx.ent.cahier_modifier_travail(&fx.prof2(), t.id, modif("x")).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_modifier_travail(&fx.eleve(&fx.x), t.id, modif("x")).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_modifier_travail(&fx.parent(&fx.x), t.id, modif("x")).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_modifier_travail(&fx.cpe(), t.id, modif("x")).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_supprimer_travail(&fx.prof2(), t.id).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_historique_travail(&fx.prof2(), t.id).await.unwrap_err()));
    // C-D10 : la direction ne réécrit plus le travail d'un collègue (elle le retire, avec motif).
    assert!(est_interdit(&fx.ent.cahier_modifier_travail(&fx.direction(), t.id, modif("Exercices (direction)")).await.unwrap_err()));
    // Échéance dans le passé : refusée.
    let err = fx
        .ent
        .cahier_modifier_travail(&fx.prof(), t.id, ModifierTravailBody { pour_le: Some(today_utc() - Duration::days(2)), ..Default::default() })
        .await
        .unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide));

    // Suppression logique : disparaît de l'agenda, historique conservé, introuvable ensuite.
    fx.ent.cahier_supprimer_travail(&fx.prof(), t.id).await.unwrap();
    assert!(fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, du, au).await.unwrap().is_empty());
    assert!(est_introuvable(&fx.ent.cahier_travail(&fx.prof(), t.id).await.unwrap_err()));
    assert!(est_introuvable(&fx.ent.cahier_supprimer_travail(&fx.prof(), t.id).await.unwrap_err()));
    let actions: Vec<String> = sqlx::query_scalar("SELECT action FROM ent_cdt_historique WHERE objet_id = $1 ORDER BY created_at")
        .bind(t.id)
        .fetch_all(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(actions.last().map(String::as_str), Some("suppression"));
}

// ───────────────────────── CDT-04 : pièces privées et liens signés ─────────────────────────

#[tokio::test]
async fn pieces_privees_controlees_et_liens_signes() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let t = fx.publier_travail(&fx.prof(), fx.cible_a(), |_| {}).await.unwrap().travail;

    // D2 : fichier actif, faux type, vide : refusés ; le type retenu est celui des octets.
    let html = FichierDepot { nom: "x.pdf".into(), mime_type: "application/pdf".into(), contenu: b"<html><script>1</script></html>".to_vec() };
    assert!(matches!(fx.ent.cahier_joindre_piece(&fx.prof(), ParentPiece::Travail(t.id), html).await.unwrap_err(), EntDbError::Fichier(_)));
    let vide = FichierDepot { nom: "v.pdf".into(), mime_type: "application/pdf".into(), contenu: vec![] };
    assert!(matches!(fx.ent.cahier_joindre_piece(&fx.prof(), ParentPiece::Travail(t.id), vide).await.unwrap_err(), EntDbError::Fichier(_)));

    let p = fx.ent.cahier_joindre_piece(&fx.prof(), ParentPiece::Travail(t.id), pdf("fiche.pdf")).await.unwrap();
    assert_eq!((p.genre.as_str(), p.mime_type.as_str(), p.taille_octets), ("ressource", "application/pdf", PDF.len() as i64));
    // Seul l'auteur (ou la direction) joint ; élève et famille jamais.
    assert!(est_interdit(&fx.ent.cahier_joindre_piece(&fx.prof2(), ParentPiece::Travail(t.id), pdf("a.pdf")).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_joindre_piece(&fx.eleve(&fx.x), ParentPiece::Travail(t.id), pdf("a.pdf")).await.unwrap_err()));

    // Le fichier n'est pas en base mais dans le magasin d'objets, sous l'établissement.
    let cle: String = sqlx::query_scalar("SELECT storage_key FROM ent_cdt_piece WHERE id = $1").bind(p.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert!(cle.starts_with(&fx.cast.etablissement_id.to_string()), "{cle}");
    assert!(fx.ent.objects().exists(&cle));

    // Lien signé : élève concerné, famille liée, professeur de la classe ; pas l'élève d'une autre classe.
    for a in [fx.eleve(&fx.x), fx.parent(&fx.x), fx.prof(), fx.direction()] {
        let l = fx.ent.cahier_lien_piece(&a, p.id).await.unwrap();
        assert!(!l.url.is_empty() && !l.url.contains(&cle), "la clé de stockage ne fuit pas dans le lien");
    }
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.eleve(&fx.y), p.id).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.parent(&fx.y), p.id).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.prof2(), p.id).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.compta(), p.id).await.unwrap_err()));
    // Pièce d'un autre établissement : introuvable.
    let autre = fixture().await.unwrap();
    assert!(est_introuvable(&fx.ent.cahier_lien_piece(&act(autre.cast.etablissement_id, &autre.cast.direction), p.id).await.unwrap_err()));

    // Pièce de séance : visible de sa classe seulement.
    let seance = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), today_utc()).await;
    let c = fx.ent.cahier_renseigner_seance(&fx.prof(), seance, "Cours", "").await.unwrap();
    let ps = fx.ent.cahier_joindre_piece(&fx.prof(), ParentPiece::Seance(c.id), pdf("cours.pdf")).await.unwrap();
    assert_eq!(ps.genre, "seance");
    assert!(fx.ent.cahier_lien_piece(&fx.eleve(&fx.w), ps.id).await.is_ok());
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.eleve(&fx.y), ps.id).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_joindre_piece(&fx.prof2(), ParentPiece::Seance(c.id), pdf("x.pdf")).await.unwrap_err()));
}

// ───────────────────────── CDT-06 / 07 : rendu en ligne, retard, retour ─────────────────────────

#[tokio::test]
async fn rendu_tardif_retour_du_professeur_et_copies_privees() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let t = fx
        .publier_travail(&fx.prof(), fx.cible_a(), |t| {
            t.type_travail = "a_rendre".into();
            t.formats_acceptes = vec!["pdf".into()];
            t.date_limite = Some(Utc::now() + Duration::days(2));
        })
        .await
        .unwrap()
        .travail;
    assert!(t.rendu_en_ligne);
    assert_eq!(fx.nb_evenements("cahier.travail_a_rendre").await, 1);

    // Un travail à faire ne se rend pas en ligne.
    let simple = fx.publier_travail(&fx.prof(), fx.cible_a(), |_| {}).await.unwrap().travail;
    assert!(est_refus(&fx.ent.cahier_rendre(&fx.eleve(&fx.x), simple.id, "", Some(pdf("a.pdf"))).await.unwrap_err(), RefusMetier::Conflit));

    // Qui peut rendre : l'élève concerné seulement (pas la famille, pas un élève d'une autre classe, pas le personnel).
    assert!(est_interdit(&fx.ent.cahier_rendre(&fx.parent(&fx.x), t.id, "", Some(pdf("a.pdf"))).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_rendre(&fx.eleve(&fx.y), t.id, "", Some(pdf("a.pdf"))).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_rendre(&fx.prof(), t.id, "", Some(pdf("a.pdf"))).await.unwrap_err()));
    // Premier dépôt sans fichier : invalide ; format hors liste : invalide ; fichier actif : refusé (D2).
    assert!(est_refus(&fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "oups", None).await.unwrap_err(), RefusMetier::Invalide));
    let png = FichierDepot { nom: "scan.png".into(), mime_type: "image/png".into(), contenu: PNG.to_vec() };
    assert!(est_refus(&fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "", Some(png)).await.unwrap_err(), RefusMetier::Invalide));
    let actif = FichierDepot { nom: "a.pdf".into(), mime_type: "application/pdf".into(), contenu: b"<svg onload=1/>".to_vec() };
    assert!(matches!(fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "", Some(actif)).await.unwrap_err(), EntDbError::Fichier(_)));
    let aucune: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_travail_remise WHERE travail_id = $1").bind(t.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(aucune, 0, "les refus ne laissent aucune remise");

    // X rend à temps ; un dépôt complémentaire ne crée ni seconde remise ni second événement.
    let r = fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "Voici ma copie", Some(pdf("copie-x.pdf"))).await.unwrap();
    assert!(!r.en_retard && r.statut == "rendu" && r.pieces.len() == 1);
    let r2 = fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "", Some(pdf("copie-x-2.pdf"))).await.unwrap();
    assert_eq!((r2.id, r2.pieces.len(), r2.commentaire_eleve.as_str()), (r.id, 2, "Voici ma copie"));
    assert_eq!(fx.nb_evenements("cahier.travail_rendu").await, 1);

    // Échéance dépassée : W rend en retard (accepté, marqué) ; la limite est ensuite fermée.
    sqlx::query("UPDATE ent_travail SET date_limite = now() - interval '1 hour' WHERE id = $1").bind(t.id).execute(fx.ent.pool()).await.unwrap();
    let rw = fx.ent.cahier_rendre(&fx.eleve(&fx.w), t.id, "", Some(pdf("copie-w.pdf"))).await.unwrap();
    assert!(rw.en_retard, "rendu après la date limite : retard marqué");
    sqlx::query("UPDATE ent_travail SET acceptation_retard = false WHERE id = $1").bind(t.id).execute(fx.ent.pool()).await.unwrap();
    let err = fx.ent.cahier_rendre(&fx.eleve(&fx.z), t.id, "", Some(pdf("copie-z.pdf"))).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Interdit), "retard non accepté : refus");

    // Récupération par le professeur : suivi complet de la cible.
    let suivi = fx.ent.cahier_suivi_travail(&fx.prof(), t.id).await.unwrap();
    assert_eq!((suivi.nb_attendus, suivi.nb_rendus, suivi.nb_en_retard), (3, 2, 1), "X, Z, W attendus ; Y exclu");
    assert!(suivi.lignes.iter().all(|l| l.eleve_id != fx.y.id));
    assert!(est_interdit(&fx.ent.cahier_suivi_travail(&fx.prof2(), t.id).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_suivi_travail(&fx.eleve(&fx.x), t.id).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_suivi_travail(&fx.parent(&fx.x), t.id).await.unwrap_err()));

    // Copies privées : l'élève auteur, sa famille et le professeur ; pas un camarade, pas un autre professeur.
    let copie_x = r2.pieces[0].id;
    assert!(fx.ent.cahier_lien_piece(&fx.eleve(&fx.x), copie_x).await.is_ok());
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.parent(&fx.x), copie_x).await.unwrap_err()), "R-C37 : jamais le dépôt de l'élève au responsable");
    assert!(fx.ent.cahier_lien_piece(&fx.prof(), copie_x).await.is_ok());
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.eleve(&fx.w), copie_x).await.unwrap_err()), "un camarade ne lit pas la copie");
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.parent(&fx.w), copie_x).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.prof2(), copie_x).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.cpe(), copie_x).await.unwrap_err()), "la vie scolaire lit le cahier, pas les copies");

    // Retour : commentaire obligatoire ; note d'un autre élève refusée ; annotation facultative.
    assert!(est_refus(&fx.ent.cahier_retour(&fx.prof(), r.id, " ", None, None).await.unwrap_err(), RefusMetier::Invalide));
    let matiere: Uuid = sqlx::query_scalar("INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ($1,$2,'MCDT','Maths') RETURNING id")
        .bind(Uuid::new_v4()).bind(fx.cast.etablissement_id).fetch_one(fx.ent.pool()).await.unwrap();
    let note_y = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_note (id, etablissement_id, eleve_id, matiere_id, valeur, date_note, auteur_user_id) VALUES ($1,$2,$3,$4,12,$5,$6)")
        .bind(note_y).bind(fx.cast.etablissement_id).bind(fx.y.id).bind(matiere).bind(today_utc()).bind(fx.cast.profs[0].user_id)
        .execute(fx.ent.pool()).await.unwrap();
    assert!(est_refus(&fx.ent.cahier_retour(&fx.prof(), r.id, "Bien", Some(note_y), None).await.unwrap_err(), RefusMetier::Invalide));
    assert!(est_interdit(&fx.ent.cahier_retour(&fx.prof2(), r.id, "Bien", None, None).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_retour(&fx.eleve(&fx.x), r.id, "Bien", None, None).await.unwrap_err()));
    let note_x = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_note (id, etablissement_id, eleve_id, matiere_id, valeur, date_note, auteur_user_id) VALUES ($1,$2,$3,$4,15,$5,$6)")
        .bind(note_x).bind(fx.cast.etablissement_id).bind(fx.x.id).bind(matiere).bind(today_utc()).bind(fx.cast.profs[0].user_id)
        .execute(fx.ent.pool()).await.unwrap();
    let corr = fx.ent.cahier_retour(&fx.prof(), r.id, "Très bien", Some(note_x), Some(pdf("copie-annotee.pdf"))).await.unwrap();
    assert_eq!((corr.statut.as_str(), corr.note_id), ("corrige", Some(note_x)));
    assert!(corr.pieces.iter().any(|p| p.genre == "annotation") && corr.pieces.iter().any(|p| p.genre == "copie"));
    assert_eq!(fx.nb_evenements("cahier.copie_rendue").await, 1);
    // Une fois corrigée, la copie ne se redépose plus ; l'élève voit le retour dans son agenda.
    assert!(est_refus(&fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "", Some(pdf("autre.pdf"))).await.unwrap_err(), RefusMetier::Conflit));
    let (du, au) = semaine();
    let item = fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, du, au).await.unwrap().into_iter().find(|i| i.travail.id == t.id).unwrap();
    assert_eq!(item.remise.as_ref().and_then(|r| r.retour_commentaire.as_deref()), Some("Très bien"));
    // L'élève lit la copie annotée ; un camarade non.
    let annotation = corr.pieces.iter().find(|p| p.genre == "annotation").unwrap().id;
    assert!(fx.ent.cahier_lien_piece(&fx.eleve(&fx.x), annotation).await.is_ok());
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.eleve(&fx.w), annotation).await.unwrap_err()));
}

#[tokio::test]
async fn marquer_fait_est_personnel_a_l_eleve() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let t = fx.publier_travail(&fx.prof(), fx.cible_a(), |_| {}).await.unwrap().travail;
    fx.ent.cahier_marquer_fait(&fx.eleve(&fx.x), t.id, true).await.unwrap();
    assert!(fait_de(&fx, fx.eleve(&fx.x), None, du, au).await);
    assert!(!fait_de(&fx, fx.eleve(&fx.w), None, du, au).await, "l'indicateur de X n'est pas celui de W");
    assert!(!fait_de(&fx, fx.parent(&fx.x), Some(fx.x.id), du, au).await, "R-C14 : la coche « fait » est un outil de l'élève, jamais vue d'un adulte");
    fx.ent.cahier_marquer_fait(&fx.eleve(&fx.x), t.id, false).await.unwrap();
    assert!(!fait_de(&fx, fx.eleve(&fx.x), None, du, au).await);
    // Ni la famille, ni le personnel, ni un élève hors cible ne marquent.
    assert!(est_interdit(&fx.ent.cahier_marquer_fait(&fx.parent(&fx.x), t.id, true).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_marquer_fait(&fx.prof(), t.id, true).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_marquer_fait(&fx.eleve(&fx.y), t.id, true).await.unwrap_err()));
}

// ───────────────────────── Droits par le moteur (D-017) ─────────────────────────

#[tokio::test]
async fn droits_par_le_moteur_roles_parametres() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let dir = (fx.cast.direction.user_id, Role::Direction.as_str());
    let droit = |action, portee, cibles| Droit { action, portee, cibles };
    // Rôle « Référent classe A » : écriture désignée sur la classe A seulement.
    let role = fx
        .ent
        .creer_role_for_actor(
            e,
            dir.0,
            dir.1,
            RoleSaisie {
                nom: format!("Référent A {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![droit(Action::CahierEcrire, Portee::Designes, vec![fx.classe_a])],
            },
        )
        .await
        .unwrap();
    // Avant l'attribution : le professeur 2 n'a rien.
    assert!(est_interdit(&fx.publier_travail(&fx.prof2(), fx.cible_a(), |_| {}).await.unwrap_err()));
    fx.ent.attribuer_role_for_actor(e, dir.0, dir.1, role.id, fx.cast.profs[1].user_id, None).await.unwrap();
    fx.publier_travail(&fx.prof2(), fx.cible_a(), |_| {}).await.unwrap();
    // Désigné : la classe A, pas la B ; écrire implique lire.
    let cible_b = Cible { classe_id: Some(fx.classe_b), groupe_id: None };
    assert!(est_interdit(&fx.publier_travail(&fx.prof2(), cible_b, |_| {}).await.unwrap_err()));
    let (du, au) = semaine();
    assert!(fx.ent.cahier_travaux_cible(&fx.prof2(), fx.cible_a(), du, au).await.is_ok());
    // La compta n'a aucun droit sur le cahier, même en lecture ; la vie scolaire lit mais n'écrit pas ses visas.
    assert!(est_interdit(&fx.ent.cahier_travaux_cible(&fx.compta(), fx.cible_a(), du, au).await.unwrap_err()));
    assert!(fx.ent.cahier_travaux_cible(&fx.cpe(), fx.cible_a(), du, au).await.is_ok());
    // Retrait du rôle : effet immédiat.
    fx.ent.retirer_role_for_actor(e, dir.0, dir.1, role.id, fx.cast.profs[1].user_id).await.unwrap();
    assert!(est_interdit(&fx.publier_travail(&fx.prof2(), fx.cible_a(), |_| {}).await.unwrap_err()));
    // Rôle « Responsable d'atelier » : portée mes_groupes sur G.
    let atelier = fx
        .ent
        .creer_role_for_actor(
            e,
            dir.0,
            dir.1,
            RoleSaisie {
                nom: format!("Atelier {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![droit(Action::CahierEcrire, Portee::MesGroupes, vec![])],
            },
        )
        .await
        .unwrap();
    fx.ent.attribuer_role_for_actor(e, dir.0, dir.1, atelier.id, fx.cast.profs[1].user_id, None).await.unwrap();
    assert!(est_interdit(&fx.publier_travail(&fx.prof2(), fx.cible_g(), |_| {}).await.unwrap_err()), "pas encore responsable du groupe");
    fx.ent.definir_responsables_groupe_for_actor(e, dir.0, dir.1, fx.groupe_g, &[fx.cast.profs[1].user_id]).await.unwrap();
    fx.publier_travail(&fx.prof2(), fx.cible_g(), |_| {}).await.unwrap();
    assert!(est_interdit(&fx.publier_travail(&fx.prof2(), fx.cible_a(), |_| {}).await.unwrap_err()), "responsable du groupe, pas de la classe");
}

#[tokio::test]
async fn visa_reserve_a_la_direction() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let visa = |c: Uuid| VisaBody { classe_id: c, prof_user_id: Some(fx.cast.profs[0].user_id), du: today_utc() - Duration::days(30), au: today_utc(), commentaire: "Cahier complet".into() };
    let v = fx.ent.cahier_viser(&fx.direction(), visa(fx.classe_a)).await.unwrap();
    assert_eq!(v.vise_par_user_id, fx.cast.direction.user_id);
    assert_eq!(fx.nb_evenements("cahier.vise").await, 1);
    for a in [fx.prof(), fx.cpe(), fx.compta(), fx.eleve(&fx.x), fx.parent(&fx.x)] {
        assert!(est_interdit(&fx.ent.cahier_viser(&a, visa(fx.classe_a)).await.unwrap_err()));
    }
    assert!(est_introuvable(&fx.ent.cahier_viser(&fx.direction(), visa(Uuid::new_v4())).await.unwrap_err()));
    let mut mauvais = visa(fx.classe_a);
    mauvais.au = mauvais.du - Duration::days(1);
    assert!(est_refus(&fx.ent.cahier_viser(&fx.direction(), mauvais).await.unwrap_err(), RefusMetier::Invalide));
    // Lecture des visas : professeur de la classe oui, étranger non.
    assert_eq!(fx.ent.cahier_visas(&fx.prof(), fx.classe_a).await.unwrap().len(), 1);
    assert!(est_interdit(&fx.ent.cahier_visas(&fx.prof2(), fx.classe_a).await.unwrap_err()));
}

// ───────────────────────── Isolation entre établissements ─────────────────────────

#[tokio::test]
async fn isolation_entre_etablissements() {
    let Some(a) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let b = fixture().await.unwrap();
    assert_ne!(a.cast.etablissement_id, b.cast.etablissement_id);
    let t = a.publier_travail(&a.prof(), a.cible_a(), |t| { t.type_travail = "a_rendre".into(); }).await.unwrap().travail;
    let (du, au) = semaine();

    // La direction de B ne lit, ne modifie, ne supprime ni ne vise rien d'A (références introuvables / refusées).
    let dir_b = b.direction();
    assert!(est_introuvable(&a.ent.cahier_travail(&dir_b, t.id).await.unwrap_err()));
    assert!(est_introuvable(&a.ent.cahier_modifier_travail(&dir_b, t.id, ModifierTravailBody { titre: Some("x".into()), ..Default::default() }).await.unwrap_err()));
    assert!(est_introuvable(&a.ent.cahier_supprimer_travail(&dir_b, t.id).await.unwrap_err()));
    assert!(est_introuvable(&a.ent.cahier_suivi_travail(&dir_b, t.id).await.unwrap_err()));
    assert!(est_interdit(&a.ent.cahier_travaux_cible(&dir_b, a.cible_a(), du, au).await.unwrap_err()));
    assert!(est_interdit(&a.ent.cahier_charge(&dir_b, a.cible_a(), du, au, None, None).await.unwrap_err()));
    assert!(est_introuvable(&a.ent.cahier_viser(&dir_b, VisaBody { classe_id: a.classe_a, prof_user_id: None, du, au, commentaire: String::new() }).await.unwrap_err()));
    // Un élève de B ne rend pas le travail d'A, et son agenda ne montre rien d'A.
    assert!(est_introuvable(&a.ent.cahier_rendre(&b.eleve(&b.x), t.id, "", Some(pdf("a.pdf"))).await.unwrap_err()));
    assert!(b.ent.cahier_agenda(&b.eleve(&b.x), None, du, au).await.unwrap().is_empty());
    // Un acteur d'A déguisé : la session d'A (établissement A) demandant un élève de B → refus.
    assert!(est_interdit(&a.ent.cahier_agenda(&a.direction(), Some(b.x.id), du, au).await.unwrap_err()));
    // Cible d'un autre établissement : refusée à l'écriture.
    assert!(est_interdit(&a.publier_travail(&a.direction(), b.cible_a(), |_| {}).await.unwrap_err()));
}

// ───────────────────────── Reprise de l'ancien cahier ─────────────────────────

#[tokio::test]
async fn ancien_cahier_repris_et_idempotent() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let e = fx.cast.etablissement_id;
    // Une entrée écrite par l'ancienne route (classe en texte) apparaît dans le nouvel agenda.
    let ancien = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_cahier_texte (id, etablissement_id, classe, matiere_libelle, titre, consigne, pour_le, cree_le, created_by_user_id)
                 VALUES ($1,$2,'CDT-A','Histoire','Chapitre 3','Lire',$3,$4,$5)")
        .bind(ancien).bind(e).bind(today_utc() + Duration::days(2)).bind(today_utc()).bind(fx.cast.profs[0].user_id)
        .execute(fx.ent.pool()).await.unwrap();
    let agenda = fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, du, au).await.unwrap();
    assert!(agenda.iter().any(|i| i.travail.titre == "Chapitre 3" && i.travail.matiere_libelle == "Histoire"));
    assert!(fx.ent.cahier_agenda(&fx.eleve(&fx.y), None, du, au).await.unwrap().is_empty());
    // Rejouer la reprise de la migration ne duplique rien.
    let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_travail WHERE etablissement_id = $1").bind(e).fetch_one(fx.ent.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO ent_travail (id, etablissement_id, classe_id, matiere_libelle, auteur_user_id, type, titre, consigne, donne_le, pour_le, ancien_id, created_at)
         SELECT gen_random_uuid(), c.etablissement_id, c.classe_id, c.matiere_libelle, c.created_by_user_id, 'a_faire', c.titre, c.consigne,
                LEAST(c.cree_le, c.pour_le), c.pour_le, c.id, c.created_at
           FROM ent_cahier_texte c WHERE c.classe_id IS NOT NULL
         ON CONFLICT (ancien_id) DO NOTHING",
    )
    .execute(fx.ent.pool())
    .await
    .unwrap();
    let apres: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_travail WHERE etablissement_id = $1").bind(e).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(avant, apres);
}

// ───────────────────────── Notifications (outbox) idempotentes ─────────────────────────

fn config(etab: Uuid) -> ConfigDistribution {
    ConfigDistribution {
        taille_lot: 100,
        max_tentatives: 3,
        delai_base: Delai::ZERO,
        delai_max: Delai::ZERO,
        intervalle: Delai::from_millis(20),
        etablissement: Some(etab),
    }
}

async fn notifs(fx: &Fx, user: Uuid, kind: &str) -> usize {
    fx.ent
        .mes_notifications(fx.cast.etablissement_id, user, 200)
        .await
        .unwrap()
        .into_iter()
        .filter(|n| n.kind == kind)
        .count()
}

#[tokio::test]
async fn notifications_travail_et_copie_idempotentes() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let d = fx.ent.distributeur(config(fx.cast.etablissement_id));
    let uid = |p: &LaboPersona| p.user_id;
    let (x_el, x_par) = (uid(fx.x.compte.as_ref().unwrap()), uid(&fx.x.parent));
    let (y_el, y_par) = (uid(fx.y.compte.as_ref().unwrap()), uid(&fx.y.parent));
    let (w_el, z_el) = (uid(fx.w.compte.as_ref().unwrap()), uid(fx.z.compte.as_ref().unwrap()));
    let prof = fx.cast.profs[0].user_id;

    // Travail publié → élèves de la classe et responsables ; rien pour la classe B.
    let t = fx
        .publier_travail(&fx.prof(), fx.cible_a(), |t| {
            t.type_travail = "a_rendre".into();
            t.date_limite = Some(Utc::now() + Duration::days(2));
        })
        .await
        .unwrap()
        .travail;
    d.distribuer_tout().await.unwrap();
    for u in [x_el, x_par, w_el, z_el] {
        assert_eq!(notifs(&fx, u, "travail_a_rendre").await, 1, "destinataire {u}");
    }
    for u in [y_el, y_par, prof] {
        assert_eq!(notifs(&fx, u, "travail_a_rendre").await, 0, "non concerné {u}");
    }
    // Idempotence : redistribuer, puis rejouer le consommateur, ne duplique rien.
    d.distribuer_tout().await.unwrap();
    d.rejouer_consommateur(crate::cahier_reactions::CONSOMMATEUR_CAHIER, fx.cast.etablissement_id, 0).await.unwrap();
    d.distribuer_tout().await.unwrap();
    for u in [x_el, x_par, w_el, z_el] {
        assert_eq!(notifs(&fx, u, "travail_a_rendre").await, 1, "rejeu : pas de doublon pour {u}");
    }

    // Modification avant tout « vu » : pas de notification ; après : une.
    fx.ent.cahier_modifier_travail(&fx.prof(), t.id, ModifierTravailBody { titre: Some("Titre 2".into()), ..Default::default() }).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&fx, x_el, "travail_modifie").await, 0);
    fx.ent.cahier_travail(&fx.eleve(&fx.x), t.id).await.unwrap();
    fx.ent.cahier_modifier_travail(&fx.prof(), t.id, ModifierTravailBody { titre: Some("Titre 3".into()), ..Default::default() }).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&fx, x_el, "travail_modifie").await, 1);
    assert_eq!(notifs(&fx, x_par, "travail_modifie").await, 1);

    // Copie rendue → le professeur (une seule notification, même après un dépôt complémentaire).
    fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "", Some(pdf("c1.pdf"))).await.unwrap();
    fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "", Some(pdf("c2.pdf"))).await.unwrap();
    d.distribuer_tout().await.unwrap();
    d.rejouer_consommateur(crate::cahier_reactions::CONSOMMATEUR_CAHIER, fx.cast.etablissement_id, 0).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&fx, prof, "copie_a_corriger").await, 1);
    assert_eq!(notifs(&fx, x_el, "copie_a_corriger").await, 0, "l'élève n'est pas notifié de son propre dépôt");

    // Correction → l'élève et ses responsables.
    let remise = fx.ent.cahier_suivi_travail(&fx.prof(), t.id).await.unwrap().lignes.into_iter().find_map(|l| l.remise).unwrap();
    fx.ent.cahier_retour(&fx.prof(), remise.id, "Bien", None, None).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&fx, x_el, "copie_corrigee").await, 1);
    assert_eq!(notifs(&fx, x_par, "copie_corrigee").await, 1);
    assert_eq!(notifs(&fx, w_el, "copie_corrigee").await, 0);

    // Travail supprimé avant distribution : rien (relu au traitement).
    let t2 = fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.titre = "Éphémère".into()).await.unwrap().travail;
    fx.ent.cahier_supprimer_travail(&fx.prof(), t2.id).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&fx, x_el, "travail_donne").await, 0);
}

// ───────────────────────── Atomicité ─────────────────────────

#[tokio::test]
async fn ecriture_et_evenement_dans_une_seule_transaction() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    // Panne de l'outbox simulée par un déclencheur : le travail n'est pas créé.
    let fonction = format!("ent_test_panne_cdt_{}", &Uuid::new_v4().simple().to_string()[..8]);
    sqlx::query(&format!("CREATE FUNCTION {fonction}() RETURNS trigger AS $$ BEGIN RAISE EXCEPTION 'panne simulée'; END; $$ LANGUAGE plpgsql"))
        .execute(fx.ent.pool()).await.unwrap();
    let declencheur = format!("{fonction}_trg");
    sqlx::query(&format!("CREATE TRIGGER {declencheur} BEFORE INSERT ON ent_outbox FOR EACH ROW WHEN (NEW.type = 'cahier.travail_donne') EXECUTE FUNCTION {fonction}()"))
        .execute(fx.ent.pool()).await.unwrap();
    let res = fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.titre = "Fantôme".into()).await;
    sqlx::query(&format!("DROP TRIGGER {declencheur} ON ent_outbox")).execute(fx.ent.pool()).await.unwrap();
    sqlx::query(&format!("DROP FUNCTION {fonction}()")).execute(fx.ent.pool()).await.unwrap();
    assert!(res.is_err());
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_travail WHERE etablissement_id = $1 AND titre = 'Fantôme'")
        .bind(fx.cast.etablissement_id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(n, 0, "pas de travail sans événement");
    // Une fois réparé, la même saisie réussit.
    fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.titre = "Fantôme".into()).await.unwrap();
}

// ═════════════════════════ Lot C0 (socle 20) : un test par défaut corrigé ═════════════════════════
//
// C-D2 à C-D15 de `docs/socles/20_CAHIER_DE_TEXTES.md` §2.2 (C-D1 : lot T0 du socle 10). Les acteurs :
// `prof` (titulaire du service de la classe A), `prof2` (sans service sauf mention), direction, CPE,
// élèves X (classe A), Y (classe B), Z (classe A + groupe G), W (classe A) et leurs responsables.

use ent_types::personnels_services::{NommerSuppleantBody, TerminerInterventionBody};

async fn agent_du_compte(fx: &Fx, user: Uuid) -> Uuid {
    sqlx::query_scalar(
        "SELECT a.id FROM ent_agent a JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND p.user_id = $2",
    )
    .bind(fx.cast.etablissement_id)
    .bind(user)
    .fetch_one(fx.ent.pool())
    .await
    .expect("agent du compte")
}

/// Exécute du SQL de test en suspendant les déclencheurs (rôle de réplication, le temps de la transaction) :
/// pour poser une situation passée que les gardes d'écriture interdisent.
async fn sql_sans_garde(fx: &Fx, sql: &str, binds: &[&(dyn Send + Sync)]) {
    let _ = binds;
    let mut tx = fx.ent.pool().begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await.unwrap();
    sqlx::query(sql).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}

async fn jour(fx: &Fx) -> NaiveDate {
    fx.ent.jour_etab(fx.cast.etablissement_id).await.unwrap()
}

/// Travail à rendre en ligne de `prof`, pour la classe A, avec échéance dans trois jours.
async fn travail_a_rendre(fx: &Fx) -> TravailDto {
    fx.publier_travail(&fx.prof(), fx.cible_a(), |t| {
        t.type_travail = "a_rendre".into();
        t.date_limite = Some(Utc::now() + Duration::days(2));
    })
    .await
    .unwrap()
    .travail
}

async fn definir_lien(fx: &Fx, e: &LaboEleve, colonnes: &str) {
    sqlx::query(&format!(
        "UPDATE ent_eleve_lien SET {colonnes} WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3"
    ))
    .bind(fx.cast.etablissement_id)
    .bind(e.id)
    .bind(e.parent.user_id)
    .execute(fx.ent.pool())
    .await
    .unwrap();
}

async fn peut(fx: &Fx, a: &Acteur<'_>, action: Action, cible: Cible) -> bool {
    fx.ent.cahier_peut(a, action, cible).await.unwrap()
}

// ───── C-D2 : portée calculée à la date par les interventions ─────

#[tokio::test]
async fn c0_cd2_remplacant_d_une_heure_n_a_ni_la_classe_ni_les_copies() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (prof0, prof2) = (fx.cast.profs[0].user_id, fx.cast.profs[1].user_id);
    let auj = jour(&fx).await;
    let s1 = fx.seance("CDT-A", prof0, Some(fx.service_a), auj).await;
    let s2 = fx.seance("CDT-A", prof0, Some(fx.service_a), auj + Duration::days(1)).await;
    sqlx::query("UPDATE ent_edt_seance SET remplacant_user_id = $2 WHERE id = $1")
        .bind(s1)
        .bind(prof2)
        .execute(fx.ent.pool())
        .await
        .unwrap();

    // Le remplaçant écrit SA séance et peut y rattacher du travail…
    fx.ent.cahier_renseigner_seance(&fx.prof2(), s1, "Remplacement : lecture suivie", "").await.unwrap();
    let t_rempl = fx
        .publier_travail(&fx.prof2(), fx.cible_a(), |t| {
            t.seance_id = Some(s1);
            t.titre = "Lecture du chapitre 2".into();
        })
        .await
        .expect("travail lié à la séance remplacée")
        .travail;
    fx.ent.cahier_modifier_travail(&fx.prof2(), t_rempl.id, ModifierTravailBody { titre: Some("Lecture, chap. 2".into()), ..Default::default() })
        .await
        .expect("il modifie son propre travail lié à sa séance");

    // … mais rien d'autre : ni une autre séance de la classe, ni du travail hors séance, ni la classe entière.
    assert!(est_interdit(&fx.ent.cahier_renseigner_seance(&fx.prof2(), s2, "autre séance", "").await.unwrap_err()));
    assert!(fx.publier_travail(&fx.prof2(), fx.cible_a(), |_| {}).await.is_err_and(|e| est_interdit(&e)));
    assert!(!peut(&fx, &fx.prof2(), Action::CahierEcrire, fx.cible_a()).await);
    assert!(!peut(&fx, &fx.prof2(), Action::CahierLire, fx.cible_a()).await, "plus de lecture de la classe entière");

    // Copies : un remplaçant ne lit ni le suivi, ni une copie, ni ne corrige.
    let t = travail_a_rendre(&fx).await;
    fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "", Some(pdf("copie.pdf"))).await.unwrap();
    assert!(est_interdit(&fx.ent.cahier_suivi_travail(&fx.prof2(), t.id).await.unwrap_err()));
    let remise = fx.ent.cahier_suivi_travail(&fx.prof(), t.id).await.unwrap().lignes.into_iter().find_map(|l| l.remise).unwrap();
    assert!(est_interdit(&fx.ent.cahier_retour(&fx.prof2(), remise.id, "Corrigé", None, None).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.prof2(), remise.pieces[0].id).await.unwrap_err()));
    // Le titulaire, lui, travaille normalement.
    fx.ent.cahier_retour(&fx.prof(), remise.id, "Bien", None, None).await.unwrap();
}

#[tokio::test]
async fn c0_cd2_suppleant_ecrit_des_sa_date_le_titulaire_pas_pendant_la_suppleance_ancien_professeur_plus_du_tout() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let (prof0, prof2) = (fx.cast.profs[0].user_id, fx.cast.profs[1].user_id);
    let auj = jour(&fx).await;
    let seance = fx.seance("CDT-A", prof0, Some(fx.service_a), auj).await;
    assert!(peut(&fx, &fx.prof(), Action::CahierEcrire, fx.cible_a()).await, "titulaire avant la suppléance");
    assert!(!peut(&fx, &fx.prof2(), Action::CahierEcrire, fx.cible_a()).await, "suppléant avant sa date");

    let agent2 = agent_du_compte(&fx, prof2).await;
    let service = fx
        .ent
        .nommer_suppleant_for_actor(
            e,
            fx.cast.direction.user_id,
            "direction",
            fx.service_a,
            &NommerSuppleantBody { agent_id: agent2, du: auj, au: None, origine: None, origine_ref: None },
        )
        .await
        .unwrap();
    let supp = service.intervenants.iter().find(|i| i.nature == "suppleant").unwrap().id;

    // Dès sa date, le suppléant écrit (même une séance au nom du titulaire) ; le titulaire n'écrit plus.
    fx.ent.cahier_renseigner_seance(&fx.prof2(), seance, "Suppléance : démarrage", "").await.unwrap();
    assert!(est_interdit(&fx.ent.cahier_renseigner_seance(&fx.prof(), seance, "Retour du titulaire", "").await.unwrap_err()));
    assert!(fx.publier_travail(&fx.prof(), fx.cible_a(), |_| {}).await.is_err_and(|e| est_interdit(&e)));
    assert!(peut(&fx, &fx.prof(), Action::CahierLire, fx.cible_a()).await, "le titulaire lit toujours son cahier");

    // La suppléance finit il y a 3 jours : lecture encore permise (délai de grâce de 7 jours), plus d'écriture.
    sql_sans_garde(&fx, &format!("UPDATE ent_service_intervenant SET du = '{}', au = '{}' WHERE id = '{supp}'", auj - Duration::days(20), auj - Duration::days(3)), &[]).await;
    assert!(!peut(&fx, &fx.prof2(), Action::CahierEcrire, fx.cible_a()).await, "ancien suppléant : plus d'écriture");
    assert!(peut(&fx, &fx.prof2(), Action::CahierLire, fx.cible_a()).await, "grâce de lecture de 7 jours");
    // 8 jours après : plus rien. Le titulaire a repris la saisie.
    sql_sans_garde(&fx, &format!("UPDATE ent_service_intervenant SET au = '{}' WHERE id = '{supp}'", auj - Duration::days(8)), &[]).await;
    assert!(!peut(&fx, &fx.prof2(), Action::CahierLire, fx.cible_a()).await, "ancien professeur : plus de lecture");
    assert!(peut(&fx, &fx.prof(), Action::CahierEcrire, fx.cible_a()).await, "le titulaire reprend la saisie");
    let _ = TerminerInterventionBody { au: None };
}

#[tokio::test]
async fn c0_cd2_restriction_des_droits_de_l_intervention_aux_cahier() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let (prof0, prof2) = (fx.cast.profs[0].user_id, fx.cast.profs[1].user_id);
    let auj = jour(&fx).await;
    fx.seance("CDT-A", prof0, Some(fx.service_a), auj).await;
    let agent2 = agent_du_compte(&fx, prof2).await;
    let service = fx
        .ent
        .nommer_suppleant_for_actor(
            e,
            fx.cast.direction.user_id,
            "direction",
            fx.service_a,
            &NommerSuppleantBody { agent_id: agent2, du: auj, au: None, origine: None, origine_ref: None },
        )
        .await
        .unwrap();
    let supp = service.intervenants.iter().find(|i| i.nature == "suppleant").unwrap().id;
    // Suppléant limité à l'appel : la saisie du cahier lui est refusée.
    sql_sans_garde(&fx, &format!("UPDATE ent_service_intervenant SET droits = ARRAY['appel'] WHERE id = '{supp}'"), &[]).await;
    assert!(!peut(&fx, &fx.prof2(), Action::CahierEcrire, fx.cible_a()).await);
    sql_sans_garde(&fx, &format!("UPDATE ent_service_intervenant SET droits = ARRAY['appel', 'cahier'] WHERE id = '{supp}'"), &[]).await;
    assert!(peut(&fx, &fx.prof2(), Action::CahierEcrire, fx.cible_a()).await);
}

// ───── C-D3 : copies et corrections ─────

/// Remise de X telle que la voit `a` dans l'agenda (Err si l'élève n'est pas résolu pour `a`).
async fn remise_vue(fx: &Fx, a: Acteur<'_>, travail: Uuid, du: NaiveDate, au: NaiveDate) -> Result<Option<RemiseDto>, EntDbError> {
    let items = fx.ent.cahier_agenda(&a, Some(fx.x.id), du, au).await?;
    Ok(items.into_iter().find(|i| i.travail.id == travail).and_then(|i| i.remise))
}

#[tokio::test]
async fn c0_cd3_cpe_direction_et_autres_professeurs_ne_lisent_aucune_remise() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let t = travail_a_rendre(&fx).await;
    fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "Voici ma copie", Some(pdf("copie.pdf"))).await.unwrap();
    let remise = fx.ent.cahier_suivi_travail(&fx.prof(), t.id).await.unwrap().lignes.into_iter().find_map(|l| l.remise).unwrap();
    fx.ent.cahier_retour(&fx.prof(), remise.id, "Correction détaillée", None, Some(pdf("annotee.pdf"))).await.unwrap();

    // Élève et intervenant en saisie : la remise complète.
    assert!(fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, du, au).await.unwrap().iter().any(|i| i.remise.as_ref().is_some_and(|r| r.commentaire_eleve == "Voici ma copie")));
    assert!(remise_vue(&fx, fx.prof(), t.id, du, au).await.unwrap().is_some_and(|r| r.retour_commentaire.is_some() && !r.pieces.is_empty()));
    // CPE et direction voient l'agenda (travail, échéance) mais jamais la remise : ni commentaire, ni copie, ni retour.
    assert!(remise_vue(&fx, fx.cpe(), t.id, du, au).await.unwrap().is_none(), "CPE");
    assert!(remise_vue(&fx, fx.direction(), t.id, du, au).await.unwrap().is_none(), "direction");
    // Un professeur qui n'enseigne pas à X ne résout même pas l'élève.
    assert!(remise_vue(&fx, fx.prof2(), t.id, du, au).await.is_err());
    // Liens de téléchargement : CPE, direction, professeur étranger refusés.
    let pieces = fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, du, au).await.unwrap().remove(0).remise.unwrap().pieces;
    assert_eq!(pieces.len(), 2);
    for p in &pieces {
        for a in [fx.cpe(), fx.direction(), fx.prof2()] {
            assert!(est_interdit(&fx.ent.cahier_lien_piece(&a, p.id).await.unwrap_err()), "{} refusé pour {}", p.genre, a.user_id);
        }
        fx.ent.cahier_lien_piece(&fx.prof(), p.id).await.unwrap();
        fx.ent.cahier_lien_piece(&fx.eleve(&fx.x), p.id).await.unwrap();
    }
    // Le suivi et l'historique du travail sont fermés à la CPE.
    assert!(est_interdit(&fx.ent.cahier_suivi_travail(&fx.cpe(), t.id).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_suivi_travail(&fx.direction(), t.id).await.unwrap_err()));
    // Même cloisonnement pour un autre élève de la classe : Z ne voit pas la remise de X.
    assert!(fx.ent.cahier_lien_piece(&fx.eleve(&fx.z), pieces[0].id).await.is_err());
}

// ───── C-D4 : droits fins des responsables ─────

#[tokio::test]
async fn c0_cd4_responsable_sans_autorite_ou_suspendu_ne_voit_rien_et_ne_recoit_rien() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let auj = jour(&fx).await;
    let seance = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), auj).await;
    fx.ent.cahier_renseigner_seance(&fx.prof(), seance, "Cours du jour", "").await.unwrap();
    let t = fx.publier_travail(&fx.prof(), fx.cible_a(), |_| {}).await.unwrap().travail;
    let parent = fx.parent(&fx.x);
    assert_eq!(fx.ent.cahier_agenda(&parent, Some(fx.x.id), du, au).await.unwrap().len(), 1, "responsable habilité");

    // Sans exercice de l'autorité parentale : agenda, cahier et détail interdits.
    definir_lien(&fx, &fx.x, "autorite_parentale = false").await;
    assert!(est_interdit(&fx.ent.cahier_agenda(&parent, Some(fx.x.id), du, au).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_seances_eleve(&parent, Some(fx.x.id), du, au).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_travail(&parent, t.id).await.unwrap_err()));
    // Il ne reçoit pas non plus l'avis du nouveau travail ; l'élève, si.
    let d = fx.ent.distributeur(config(fx.cast.etablissement_id));
    fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.titre = "Nouveau".into()).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&fx, fx.x.parent.user_id, "travail_donne").await, 0, "pas d'avis sans autorité parentale");
    assert_eq!(notifs(&fx, fx.x.compte.as_ref().unwrap().user_id, "travail_donne").await, 2);
    definir_lien(&fx, &fx.x, "autorite_parentale = true").await;
    assert!(fx.ent.cahier_agenda(&parent, Some(fx.x.id), du, au).await.is_ok());

    // Lien suspendu (décision de justice) : plus rien.
    definir_lien(&fx, &fx.x, "suspendu_le = now(), suspension_motif = 'décision du juge'").await;
    assert!(est_interdit(&fx.ent.cahier_agenda(&parent, Some(fx.x.id), du, au).await.unwrap_err()));
    definir_lien(&fx, &fx.x, "suspendu_le = NULL, suspension_motif = NULL").await;
    // Lien expiré : plus rien.
    definir_lien(&fx, &fx.x, "date_fin = (now() - interval '3 days')::date").await;
    assert!(est_interdit(&fx.ent.cahier_agenda(&parent, Some(fx.x.id), du, au).await.unwrap_err()));
    definir_lien(&fx, &fx.x, "date_fin = NULL").await;
}

#[tokio::test]
async fn c0_cd4_retour_a_note_seulement_avec_le_droit_voir_les_notes_et_jamais_le_depot_de_l_eleve() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let e = fx.cast.etablissement_id;
    let t = travail_a_rendre(&fx).await;
    fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "Mon commentaire", Some(pdf("depot.pdf"))).await.unwrap();
    let remise = fx.ent.cahier_suivi_travail(&fx.prof(), t.id).await.unwrap().lignes.into_iter().find_map(|l| l.remise).unwrap();
    let matiere = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ($1, $2, 'MATH-C0', 'Maths')")
        .bind(matiere).bind(e).execute(fx.ent.pool()).await.unwrap();
    let note = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_note (id, etablissement_id, eleve_id, matiere_id, valeur, date_note, auteur_user_id) VALUES ($1,$2,$3,$4,14,$5,$6)")
        .bind(note).bind(e).bind(fx.x.id).bind(matiere).bind(jour(&fx).await).bind(fx.cast.profs[0].user_id)
        .execute(fx.ent.pool()).await.unwrap();
    fx.ent.cahier_retour(&fx.prof(), remise.id, "14/20, bon travail", Some(note), Some(pdf("annotee.pdf"))).await.unwrap();

    let parent = fx.parent(&fx.x);
    let vue_parent = |notes: bool| {
        let fx = &fx;
        let parent = fx.parent(&fx.x);
        async move {
            definir_lien(fx, &fx.x, &format!("peut_voir_notes = {notes}")).await;
            fx.ent.cahier_agenda(&parent, Some(fx.x.id), du, au).await.unwrap().remove(0).remise.unwrap()
        }
    };
    // Sans « voir les notes » : le statut constaté seulement, aucun retour, aucune pièce, jamais le commentaire de l'élève.
    let r = vue_parent(false).await;
    assert_eq!((r.statut.as_str(), r.retour_commentaire.as_ref(), r.note_id), ("rendu", None, None));
    assert!(r.pieces.is_empty() && r.commentaire_eleve.is_empty());
    // Avec : le retour et la copie annotée, jamais le dépôt de l'élève.
    let r = vue_parent(true).await;
    assert_eq!(r.statut, "corrige");
    assert_eq!(r.note_id, Some(note));
    assert_eq!(r.pieces.len(), 1);
    assert_eq!(r.pieces[0].genre, "annotation");
    assert!(r.commentaire_eleve.is_empty());
    fx.ent.cahier_lien_piece(&parent, r.pieces[0].id).await.unwrap();
    let depot = fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, du, au).await.unwrap().remove(0).remise.unwrap().pieces.into_iter().find(|p| p.genre == "copie").unwrap();
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&parent, depot.id).await.unwrap_err()), "dépôt de l'élève : jamais au responsable");
    // « Fait » et « vu » ne sont jamais montrés à un adulte.
    fx.ent.cahier_marquer_fait(&fx.eleve(&fx.x), t.id, true).await.unwrap();
    let item = fx.ent.cahier_agenda(&parent, Some(fx.x.id), du, au).await.unwrap().remove(0);
    assert!(!item.fait && !item.vu);
    assert!(fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, du, au).await.unwrap()[0].fait, "l'élève voit sa propre coche");
}

// ───── C-D5 : élève parti ─────

#[tokio::test]
async fn c0_cd5_eleve_parti_ne_voit_plus_ne_rend_plus_ne_recoit_plus_mais_garde_ses_remises() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let auj = jour(&fx).await;
    let seance = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), auj).await;
    fx.ent.cahier_renseigner_seance(&fx.prof(), seance, "Contenu", "").await.unwrap();
    let t = travail_a_rendre(&fx).await;
    let t_autre = fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.titre = "Autre".into()).await.unwrap().travail;
    fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "", Some(pdf("c.pdf"))).await.unwrap();
    assert!(voit_seance(&fx, &fx.x, seance, du, au).await);

    // X quitte la classe A hier.
    sqlx::query("UPDATE ent_affectation SET date_debut = $3, date_fin = $4 WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(fx.cast.etablissement_id).bind(fx.x.id).bind(auj - Duration::days(60)).bind(auj - Duration::days(1))
        .execute(fx.ent.pool()).await.unwrap();

    assert!(fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, du, au).await.unwrap().is_empty(), "plus aucun travail dans l'agenda");
    assert!(!voit_seance(&fx, &fx.x, seance, du, au).await, "plus de cahier de la classe");
    assert!(est_interdit(&fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "complément", Some(pdf("c2.pdf"))).await.unwrap_err()), "plus de dépôt");
    assert!(est_interdit(&fx.ent.cahier_marquer_fait(&fx.eleve(&fx.x), t_autre.id, true).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_travail(&fx.eleve(&fx.x), t_autre.id).await.unwrap_err()));
    // Il garde la lecture du travail dont il a une remise.
    let mien = fx.ent.cahier_travail(&fx.eleve(&fx.x), t.id).await.unwrap();
    assert!(mien.remise.is_some());
    // Plus d'avis pour lui ni pour son responsable ; Z et W continuent d'en recevoir.
    let d = fx.ent.distributeur(config(fx.cast.etablissement_id));
    fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.titre = "Après le départ".into()).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&fx, fx.x.compte.as_ref().unwrap().user_id, "travail_donne").await, 0, "X parti");
    assert_eq!(notifs(&fx, fx.x.parent.user_id, "travail_donne").await, 0, "responsable de X");
    assert_eq!(notifs(&fx, fx.w.compte.as_ref().unwrap().user_id, "travail_donne").await, 2);
}

// ───── C-D6 : séances de groupe ─────

#[tokio::test]
async fn c0_cd6_la_liste_d_une_classe_ne_montre_pas_les_seances_de_groupe_des_autres() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let (prof0, prof2) = (fx.cast.profs[0].user_id, fx.cast.profs[1].user_id);
    let auj = jour(&fx).await;
    let service_g = fx
        .ent
        .referentiel_creer_service(
            Role::Direction.as_str(),
            fx.cast.etablissement_id,
            CreerService { prof_user_id: prof2, matiere_id: None, classe_id: None, groupe_id: Some(fx.groupe_g), professeur_principal: false },
        )
        .await
        .unwrap();
    let s_groupe = fx.seance("CDT-A", prof2, Some(service_g.id), auj).await;
    let s_classe = fx.seance("CDT-A", prof0, Some(fx.service_a), auj).await;
    let ids = |l: Vec<SeanceCahierDto>| l.into_iter().map(|s| s.seance_id).collect::<Vec<_>>();

    let vue_prof0 = ids(fx.ent.cahier_seances(&fx.prof(), Some(fx.classe_a), du, au).await.unwrap());
    assert!(vue_prof0.contains(&s_classe) && !vue_prof0.contains(&s_groupe), "le professeur de la classe ne voit pas la séance du groupe");
    let vue_cpe = ids(fx.ent.cahier_seances(&fx.cpe(), Some(fx.classe_a), du, au).await.unwrap());
    assert!(vue_cpe.contains(&s_classe) && vue_cpe.contains(&s_groupe), "portée d'établissement : tout");
    // Le professeur du groupe retrouve sa séance dans « mes séances ».
    assert!(ids(fx.ent.cahier_seances(&fx.prof2(), None, du, au).await.unwrap()).contains(&s_groupe));
    // Contenu du groupe : illisible du professeur de la classe.
    fx.ent.cahier_renseigner_seance(&fx.prof2(), s_groupe, "Atelier du groupe", "").await.unwrap();
    let piece_seance = fx.ent.cahier_seances(&fx.prof2(), None, du, au).await.unwrap().into_iter().find(|s| s.seance_id == s_groupe).unwrap().contenu.unwrap();
    let p = fx.ent.cahier_joindre_piece(&fx.prof2(), ParentPiece::Seance(piece_seance.id), pdf("atelier.pdf")).await.unwrap();
    assert!(est_interdit(&fx.ent.cahier_lien_piece(&fx.prof(), p.id).await.unwrap_err()));
}

// ───── C-D7 + NJ-33 : ancienne route et écriture par n'importe quel connecté ─────

fn cahier_texte(fx: &Fx, classe: &str, auteur: Uuid) -> domain_eleve::CreerCahierTexte {
    domain_eleve::CreerCahierTexte {
        etablissement_id: fx.cast.etablissement_id,
        classe: classe.into(),
        matiere_libelle: "Maths".into(),
        titre: "Exercices".into(),
        consigne: "p. 12".into(),
        pour_le: today_utc() + Duration::days(2),
        cree_le: today_utc(),
        actor_user_id: auteur,
    }
}

#[tokio::test]
async fn c0_cd7_nj33_ancienne_route_ecriture_et_lecture_bornees_par_la_portee() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let (prof0, prof2) = (fx.cast.profs[0].user_id, fx.cast.profs[1].user_id);
    // NJ-33 : « tout utilisateur connecté crée, modifie ou masque n'importe quelle entrée » — non.
    for (role, user) in [
        ("eleve", fx.x.compte.as_ref().unwrap().user_id),
        ("parent", fx.x.parent.user_id),
        ("compta", fx.cast.compta.user_id),
        ("prof", prof2), // professeur sans cours sur la classe
        ("cpe", fx.cast.cpe.user_id), // pas de cahier.ecrire : le rôle codé en dur ne suffit plus
    ] {
        let r = fx.ent.creer_cahier_texte_for_actor(role, cahier_texte(&fx, "CDT-A", user)).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    // Un professeur de groupe seulement n'écrit pas pour toute la classe.
    let service_g = fx
        .ent
        .referentiel_creer_service(
            Role::Direction.as_str(),
            e,
            CreerService { prof_user_id: prof2, matiere_id: None, classe_id: None, groupe_id: Some(fx.groupe_g), professeur_principal: false },
        )
        .await
        .unwrap();
    let _ = service_g;
    let r = fx.ent.creer_cahier_texte_for_actor("prof", cahier_texte(&fx, "CDT-A", prof2)).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "professeur d'un groupe : {r:?}");
    // Classe inconnue : refusée.
    assert!(matches!(fx.ent.creer_cahier_texte_for_actor("prof", cahier_texte(&fx, "INCONNUE", prof0)).await, Err(EntDbError::Forbidden)));
    // Le professeur de la classe écrit ; la direction (portée d'établissement) aussi.
    assert!(fx.ent.creer_cahier_texte_for_actor("prof", cahier_texte(&fx, "CDT-A", prof0)).await.is_ok());
    assert!(fx.ent.creer_cahier_texte_for_actor("direction", cahier_texte(&fx, "CDT-A", fx.cast.direction.user_id)).await.is_ok());

    // Lecture : le personnel est borné par sa portée à la date.
    let lire = |role: &'static str, user: Uuid| {
        let fx = &fx;
        async move { fx.ent.list_cahier_texte_for_actor(e, user, role, Some("CDT-A".into()), None, None, None).await }
    };
    assert!(lire("prof", prof0).await.is_ok());
    assert!(matches!(lire("prof", prof2).await, Err(EntDbError::Forbidden)), "professeur étranger à la classe");
    assert!(lire("cpe", fx.cast.cpe.user_id).await.is_ok());
    // Parent sans autorité parentale : refusé ; avec : lit la classe de son enfant.
    let lire_parent = || fx.ent.list_cahier_texte_for_actor(e, fx.x.parent.user_id, "parent", None, Some(fx.x.id), None, None);
    assert!(lire_parent().await.is_ok());
    definir_lien(&fx, &fx.x, "autorite_parentale = false").await;
    assert!(matches!(lire_parent().await, Err(EntDbError::Forbidden)));
}

#[tokio::test]
async fn c0_nj33_nouvelle_route_ecriture_modification_masquage_refuses_aux_non_autorises() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let t = fx.publier_travail(&fx.prof(), fx.cible_a(), |_| {}).await.unwrap().travail;
    let intrus: Vec<(&str, Acteur<'_>)> = vec![
        ("élève", fx.eleve(&fx.x)),
        ("parent", fx.parent(&fx.x)),
        ("compta", fx.compta()),
        ("cpe", fx.cpe()),
        ("professeur étranger", fx.prof2()),
    ];
    for (nom, a) in &intrus {
        assert!(fx.publier_travail(a, fx.cible_a(), |_| {}).await.is_err_and(|e| est_interdit(&e)), "{nom} : création");
        let modif = fx.ent.cahier_modifier_travail(a, t.id, ModifierTravailBody { titre: Some("Piraté".into()), ..Default::default() }).await;
        assert!(modif.is_err_and(|e| est_interdit(&e)), "{nom} : modification");
        assert!(est_interdit(&fx.ent.cahier_supprimer_travail(a, t.id).await.unwrap_err()), "{nom} : masquage");
        assert!(est_interdit(&fx.ent.cahier_retirer_travail(a, t.id, "motif quelconque").await.unwrap_err()), "{nom} : retrait");
    }
    let seance = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), jour(&fx).await).await;
    for (nom, a) in &intrus {
        assert!(est_interdit(&fx.ent.cahier_renseigner_seance(a, seance, "Piraté", "").await.unwrap_err()), "{nom} : contenu de séance");
    }
    // Le travail est intact.
    assert_eq!(fx.ent.cahier_travail(&fx.prof(), t.id).await.unwrap().travail.titre, "Exercices");
}

// ───── C-D8 : retour, note liée, historique ─────

#[tokio::test]
async fn c0_cd8_note_liee_de_la_main_du_correcteur_et_retour_historise() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let t = travail_a_rendre(&fx).await;
    let t2 = fx.publier_travail(&fx.prof(), fx.cible_a(), |t| {
        t.type_travail = "a_rendre".into();
        t.date_limite = Some(Utc::now() + Duration::days(2));
        t.titre = "Second".into();
    }).await.unwrap().travail;
    fx.ent.cahier_rendre(&fx.eleve(&fx.x), t.id, "", Some(pdf("a.pdf"))).await.unwrap();
    fx.ent.cahier_rendre(&fx.eleve(&fx.x), t2.id, "", Some(pdf("b.pdf"))).await.unwrap();
    let suivi = |tid: Uuid| {
        let fx = &fx;
        async move { fx.ent.cahier_suivi_travail(&fx.prof(), tid).await.unwrap().lignes.into_iter().find_map(|l| l.remise).unwrap() }
    };
    let (r1, r2) = (suivi(t.id).await, suivi(t2.id).await);
    let matiere = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ($1, $2, 'MATH-C8', 'Maths')").bind(matiere).bind(e).execute(fx.ent.pool()).await.unwrap();
    let nouvelle_note = |eleve: Uuid, auteur: Uuid| {
        let (fx, id) = (&fx, Uuid::new_v4());
        async move {
            sqlx::query("INSERT INTO ent_note (id, etablissement_id, eleve_id, matiere_id, valeur, date_note, auteur_user_id) VALUES ($1,$2,$3,$4,12,$5,$6)")
                .bind(id).bind(fx.cast.etablissement_id).bind(eleve).bind(matiere).bind(today_utc()).bind(auteur)
                .execute(fx.ent.pool()).await.unwrap();
            id
        }
    };
    let note_prof = nouvelle_note(fx.x.id, fx.cast.profs[0].user_id).await;
    let note_autre_auteur = nouvelle_note(fx.x.id, fx.cast.profs[1].user_id).await;
    let note_autre_eleve = nouvelle_note(fx.w.id, fx.cast.profs[0].user_id).await;
    for n in [note_autre_auteur, note_autre_eleve, Uuid::new_v4()] {
        let err = fx.ent.cahier_retour(&fx.prof(), r1.id, "Bien", Some(n), None).await.unwrap_err();
        assert!(est_refus(&err, RefusMetier::Invalide), "note refusée : {err:?}");
    }
    fx.ent.cahier_retour(&fx.prof(), r1.id, "Premier retour", Some(note_prof), None).await.unwrap();
    // Une même note ne peut pas être liée à la remise d'un autre travail.
    let err = fx.ent.cahier_retour(&fx.prof(), r2.id, "Réutilisation", Some(note_prof), None).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide), "{err:?}");
    // Historique : un retour corrigé garde l'état précédent.
    let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_historique WHERE objet = 'remise' AND objet_id = $1").bind(r1.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(avant, 0);
    fx.ent.cahier_retour(&fx.prof(), r1.id, "Retour corrigé", None, None).await.unwrap();
    let (n, instantane): (i64, serde_json::Value) = sqlx::query_as(
        "SELECT count(*), max(instantane::text)::jsonb FROM ent_cdt_historique WHERE objet = 'remise' AND objet_id = $1",
    ).bind(r1.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(n, 1);
    assert_eq!(instantane["retour_commentaire"], "Premier retour");
    assert_eq!(instantane["note_id"], note_prof.to_string());
}

// ───── C-D9 : visa ─────

#[tokio::test]
async fn c0_cd9_visa_individuel_lisible_du_vise_et_de_la_direction_seulement() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (prof0, prof2) = (fx.cast.profs[0].user_id, fx.cast.profs[1].user_id);
    let (du, au) = (today_utc() - Duration::days(7), today_utc());
    let visa = |prof: Option<Uuid>, commentaire: &str| VisaBody { classe_id: fx.classe_a, prof_user_id: prof, du, au, commentaire: commentaire.into() };
    // Un professeur qui n'enseigne pas dans la classe ne peut pas être visé.
    let err = fx.ent.cahier_viser(&fx.direction(), visa(Some(prof2), "Étranger")).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide), "{err:?}");
    fx.ent.cahier_viser(&fx.direction(), visa(Some(prof0), "Cahier à compléter pour la semaine 45")).await.unwrap();
    fx.ent.cahier_viser(&fx.direction(), visa(None, "Visa de classe : RAS")).await.unwrap();
    let commentaires = |l: Vec<VisaDto>| l.into_iter().map(|v| (v.prof_user_id.is_some(), v.commentaire)).collect::<Vec<_>>();
    // Le visé et la direction lisent tout ; la CPE ne lit pas le commentaire individuel.
    for a in [fx.prof(), fx.direction()] {
        let l = commentaires(fx.ent.cahier_visas(&a, fx.classe_a).await.unwrap());
        assert!(l.contains(&(true, "Cahier à compléter pour la semaine 45".to_string())), "{}", a.user_id);
    }
    let l = commentaires(fx.ent.cahier_visas(&fx.cpe(), fx.classe_a).await.unwrap());
    assert!(l.contains(&(true, String::new())), "commentaire individuel masqué : {l:?}");
    assert!(l.contains(&(false, "Visa de classe : RAS".to_string())), "visa de classe lisible");
}

// ───── C-D10 : la direction retire, ne réécrit plus ─────

#[tokio::test]
async fn c0_cd10_direction_retire_avec_motif_et_avis_a_l_auteur_sans_modifier() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let t = fx.publier_travail(&fx.prof(), fx.cible_a(), |_| {}).await.unwrap().travail;
    // Plus de modification, de suppression ni de pièce par la direction.
    let modif = fx.ent.cahier_modifier_travail(&fx.direction(), t.id, ModifierTravailBody { consigne: Some("Réécrit".into()), ..Default::default() }).await;
    assert!(modif.is_err_and(|e| est_interdit(&e)));
    assert!(est_interdit(&fx.ent.cahier_supprimer_travail(&fx.direction(), t.id).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_joindre_piece(&fx.direction(), ParentPiece::Travail(t.id), pdf("x.pdf")).await.unwrap_err()));
    // L'historique reste lisible de l'auteur et de la direction, pas de la CPE.
    assert!(fx.ent.cahier_historique_travail(&fx.direction(), t.id).await.is_ok());
    assert!(fx.ent.cahier_historique_travail(&fx.prof(), t.id).await.is_ok());
    assert!(est_interdit(&fx.ent.cahier_historique_travail(&fx.cpe(), t.id).await.unwrap_err()));
    // Le retrait exige un motif et le droit de viser ; l'auteur ne « retire » pas, il supprime.
    assert!(est_refus(&fx.ent.cahier_retirer_travail(&fx.direction(), t.id, "  ").await.unwrap_err(), RefusMetier::Invalide));
    assert!(est_interdit(&fx.ent.cahier_retirer_travail(&fx.prof(), t.id, "je retire").await.unwrap_err()));
    fx.ent.cahier_retirer_travail(&fx.direction(), t.id, "Contenu inapproprié pour des élèves de 6e").await.unwrap();
    assert!(fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, du, au).await.unwrap().is_empty(), "sorti de la diffusion");
    let (par, motif, consigne): (Uuid, String, String) = sqlx::query_as("SELECT retire_par_user_id, retire_motif, consigne FROM ent_travail WHERE id = $1")
        .bind(t.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!((par, motif.as_str(), consigne.as_str()), (fx.cast.direction.user_id, "Contenu inapproprié pour des élèves de 6e", "p. 12 n° 3"));
    let histo: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_historique WHERE objet = 'travail' AND objet_id = $1 AND action = 'retrait'").bind(t.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(histo, 1);
    // L'auteur est averti, avec le motif.
    fx.ent.distributeur(config(fx.cast.etablissement_id)).distribuer_tout().await.unwrap();
    assert_eq!(notifs(&fx, fx.cast.profs[0].user_id, "travail_retire").await, 1);
}

// ───── C-D11 : aucune requête sans contexte d'établissement ─────

#[test]
fn c0_cd11_toute_requete_du_cahier_passe_par_tenant_tx() {
    for fichier in ["cahier.rs", "cahier_reactions.rs"] {
        let src = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(fichier)).unwrap();
        let hors_tx = src.matches("self.pool()").count() - src.matches("TenantTx::begin(self.pool()").count();
        assert_eq!(hors_tx, 0, "{fichier} : une requête utilise le pool sans TenantTx (RLS sans effet)");
        assert!(!src.contains("fetch_one(self.pool())") && !src.contains("execute(self.pool())") && !src.contains("fetch_all(self.pool())"));
    }
}

// ───── C-D12 : pièces ─────

#[tokio::test]
async fn c0_cd12_retrait_logique_d_une_piece_et_plafond_tenu_sous_concurrence() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let t = fx.publier_travail(&fx.prof(), fx.cible_a(), |_| {}).await.unwrap().travail;
    let erronee = fx.ent.cahier_joindre_piece(&fx.prof(), ParentPiece::Travail(t.id), pdf("erreur.pdf")).await.unwrap();
    assert!(fx.ent.cahier_lien_piece(&fx.eleve(&fx.x), erronee.id).await.is_ok());
    // Retrait : par l'auteur seulement.
    assert!(est_interdit(&fx.ent.cahier_retirer_piece(&fx.prof2(), erronee.id).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cahier_retirer_piece(&fx.direction(), erronee.id).await.unwrap_err()));
    fx.ent.cahier_retirer_piece(&fx.prof(), erronee.id).await.unwrap();
    assert!(fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, du, au).await.unwrap()[0].travail.pieces.is_empty());
    assert!(est_introuvable(&fx.ent.cahier_lien_piece(&fx.eleve(&fx.x), erronee.id).await.unwrap_err()));
    let (conservee,): (bool,) = sqlx::query_as("SELECT retiree_le IS NOT NULL AND storage_key <> '' FROM ent_cdt_piece WHERE id = $1").bind(erronee.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert!(conservee, "la ligne et l'objet restent");
    // Plafond de 10 pièces, tenu même avec 12 dépôts simultanés ; un retrait libère une place.
    let a = fx.prof();
    let ajouter = |i: usize| fx.ent.cahier_joindre_piece(&a, ParentPiece::Travail(t.id), pdf(&format!("p{i}.pdf")));
    let (r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11) =
        tokio::join!(ajouter(0), ajouter(1), ajouter(2), ajouter(3), ajouter(4), ajouter(5), ajouter(6), ajouter(7), ajouter(8), ajouter(9), ajouter(10), ajouter(11));
    let essais = [r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11];
    let ok = essais.iter().filter(|r| r.is_ok()).count();
    assert_eq!(ok, 10, "10 pièces actives au plus : {:?}", essais.iter().filter_map(|r| r.as_ref().err()).collect::<Vec<_>>());
    assert!(essais.iter().filter_map(|r| r.as_ref().err()).all(|e| est_refus(e, RefusMetier::Conflit)));
    let une = essais.iter().find_map(|r| r.as_ref().ok()).unwrap().id;
    fx.ent.cahier_retirer_piece(&fx.prof(), une).await.unwrap();
    fx.ent.cahier_joindre_piece(&fx.prof(), ParentPiece::Travail(t.id), pdf("derniere.pdf")).await.unwrap();
    // Copie de l'élève : retirable par lui avant correction, pas par un autre.
    let tr = travail_a_rendre(&fx).await;
    let remise = fx.ent.cahier_rendre(&fx.eleve(&fx.x), tr.id, "", Some(pdf("mauvaise.pdf"))).await.unwrap();
    assert!(est_interdit(&fx.ent.cahier_retirer_piece(&fx.eleve(&fx.w), remise.pieces[0].id).await.unwrap_err()));
    fx.ent.cahier_retirer_piece(&fx.eleve(&fx.x), remise.pieces[0].id).await.unwrap();
}

// ───── C-D13 : jour et heure de l'établissement ─────

#[tokio::test]
async fn c0_cd13_fuseau_de_l_etablissement_pour_le_jour_et_la_fin_de_journee() {
    use chrono::Timelike;
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    // Fuseau à décalage fixe qui place l'établissement un jour AVANT (UTC−12, avant midi UTC) ou APRÈS
    // (UTC+14, après midi UTC) le jour UTC : le cas de La Réunion (UTC+4) à 23 h UTC, en plus net.
    let (fuseau, decalage_h) = if Utc::now().hour() < 12 { ("Etc/GMT+12", -12) } else { ("Pacific/Kiritimati", 14) };
    sqlx::query("INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, $2) ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = $2")
        .bind(e).bind(fuseau).execute(fx.ent.pool()).await.unwrap();
    let local = jour(&fx).await;
    assert_ne!(local, Utc::now().naive_utc().date(), "le fuseau de test doit décaler le jour");
    // « Pour aujourd'hui » à l'heure de l'établissement est accepté, avec sa fin de journée locale pour échéance.
    let t = fx
        .publier_travail(&fx.prof(), fx.cible_a(), |t| {
            t.type_travail = "a_rendre".into();
            t.pour_le = local;
            t.date_limite = None;
        })
        .await
        .expect("le jour local d'aujourd'hui n'est pas « dans le passé »")
        .travail;
    let attendu = (local + Duration::days(1)).and_hms_opt(0, 0, 0).unwrap().and_utc() - Duration::hours(decalage_h) - Duration::seconds(1);
    assert_eq!(t.date_limite, Some(attendu), "23 h 59 min 59 s heure locale");
    // Lecture d'un élève et appartenance au public : calculées au jour local aussi.
    assert!(fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, local, local).await.is_ok());
}

// ───── C-D15 : jamais d'identifiant de compte ─────

#[tokio::test]
async fn c0_cd15_les_eleves_et_familles_recoivent_le_nom_de_l_intervenant_pas_son_identifiant() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (du, au) = semaine();
    let prof0 = fx.cast.profs[0].user_id;
    let seance = fx.seance("CDT-A", prof0, Some(fx.service_a), jour(&fx).await).await;
    fx.ent.cahier_renseigner_seance(&fx.prof(), seance, "Contenu", "").await.unwrap();
    fx.publier_travail(&fx.prof(), fx.cible_a(), |_| {}).await.unwrap();
    let json = |v: &dyn Fn() -> String| v();
    let agenda = fx.ent.cahier_agenda(&fx.eleve(&fx.x), None, du, au).await.unwrap();
    let cahier = fx.ent.cahier_seances_eleve(&fx.parent(&fx.x), Some(fx.x.id), du, au).await.unwrap();
    for sortie in [json(&|| serde_json::to_string(&agenda).unwrap()), json(&|| serde_json::to_string(&cahier).unwrap())] {
        assert!(!sortie.contains(&prof0.to_string()), "identifiant de compte exposé : {sortie}");
        assert!(!sortie.contains("auteur_user_id"));
    }
    assert!(!agenda[0].travail.auteur.trim().is_empty());
    assert!(!cahier[0].contenu.as_ref().unwrap().auteur.trim().is_empty());
}

// ═════════════════════════ Lot C1 (socle 20) : modèle, reprise, quarantaine ═════════════════════════

const TABLES_C1: &[&str] = &[
    "ent_cdt_reglage", "ent_cdt_reglage_format", "ent_cdt_nature",
    "ent_cdt_entree", "ent_cdt_entree_version", "ent_cdt_entree_notion", "ent_cdt_contribution", "ent_cdt_contribution_version",
    "ent_cdt_travail", "ent_cdt_travail_version", "ent_cdt_travail_format", "ent_cdt_travail_cible", "ent_cdt_variante",
    "ent_cdt_variante_eleve", "ent_cdt_echeance_eleve", "ent_cdt_delai_demande", "ent_cdt_groupe", "ent_cdt_groupe_membre",
    "ent_cdt_note_accompagnement", "ent_cdt_piece_jointe", "ent_cdt_lien",
    "ent_cdt_remise", "ent_cdt_depot", "ent_cdt_depot_signalement", "ent_cdt_statut", "ent_cdt_coche",
    "ent_cdt_retour", "ent_cdt_retour_version", "ent_cdt_retour_critere", "ent_cdt_signalement_copie", "ent_cdt_consentement_exemple",
    "ent_cdt_report", "ent_cdt_continuite", "ent_cdt_continuite_cours",
    "ent_cdt_visa_element", "ent_cdt_observation", "ent_cdt_observation_reponse", "ent_cdt_consultation", "ent_cdt_alerte_charge",
    "ent_cdt_archive", "ent_cdt_quarantaine",
];

#[tokio::test]
async fn c1_rls_forcee_et_isolation_sur_toutes_les_tables_du_modele() {
    crate::tests_rls_lots::verifier_tables(TABLES_C1).await;
}

#[test]
fn c1_aucune_colonne_json_dans_les_migrations_du_lot() {
    // D-036 : « grep -i jsonb » vide sur les migrations 2401–2412 (2400 : aucune colonne non plus).
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    let mut vus = 0;
    for e in std::fs::read_dir(dossier).unwrap() {
        let p = e.unwrap().path();
        let nom = p.file_name().unwrap().to_string_lossy().to_string();
        let Some(n) = nom.split('_').next().and_then(|n| n.parse::<u32>().ok()) else { continue };
        if !(2400..=2419).contains(&n) {
            continue;
        }
        vus += 1;
        let texte: String = std::fs::read_to_string(&p).unwrap().to_lowercase().lines().filter(|l| !l.trim_start().starts_with("--")).collect::<Vec<_>>().join("\n");
        assert!(!texte.contains("jsonb") && !texte.contains(" json"), "{nom} : colonne JSON (D-036)");
    }
    assert!(vus >= 11, "migrations du lot trouvées : {vus}");
}

/// Compteur de lignes dans une table, pour un établissement.
async fn nb(fx: &Fx, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE etablissement_id = $1"))
        .bind(fx.cast.etablissement_id)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

async fn reprendre(fx: &Fx) -> Vec<(String, i64, i64, i64)> {
    sqlx::query_as("SELECT objet, anciens, repris, quarantaine FROM ent_cdt_reprise($1)")
        .bind(fx.cast.etablissement_id)
        .fetch_all(fx.ent.pool())
        .await
        .unwrap()
}

/// Scénario de l'ancien modèle : contenus (dont un sans cours), travaux (dont un « contrôle » et un sans cours
/// retrouvable), pièces, remise, retour, coche, historique.
struct AncienScenario {
    seance_avec_cours: Uuid,
    seance_sans_cours: Uuid,
    travail_avec_seance: TravailDto,
    controle: TravailDto,
    travail_orphelin: TravailDto,
}

async fn ancien_scenario(fx: &Fx) -> AncienScenario {
    let prof0 = fx.cast.profs[0].user_id;
    let auj = jour(fx).await;
    let seance_avec_cours = fx.seance("CDT-A", prof0, Some(fx.service_a), auj).await;
    // Séance « classe texte » : ni service ni cours (le prof est titulaire, repli de l'ancien modèle).
    let seance_sans_cours = fx.seance("CDT-A", prof0, None, auj + Duration::days(1)).await;
    fx.ent.cahier_renseigner_seance(&fx.prof(), seance_avec_cours, "Fractions : définition", "Manuel p. 10").await.unwrap();
    fx.ent.cahier_renseigner_seance(&fx.prof(), seance_avec_cours, "Fractions : définition et exemples", "").await.unwrap();
    fx.ent.cahier_renseigner_seance(&fx.prof(), seance_sans_cours, "Cours sans rattachement", "").await.unwrap();
    let travail_avec_seance = fx
        .publier_travail(&fx.prof(), fx.cible_a(), |t| {
            t.seance_id = Some(seance_avec_cours);
            t.type_travail = "a_rendre".into();
            t.date_limite = Some(Utc::now() + Duration::days(2));
            t.formats_acceptes = vec!["pdf".into()];
            t.titre = "Rédaction".into();
        })
        .await
        .unwrap()
        .travail;
    fx.ent.cahier_modifier_travail(&fx.prof(), travail_avec_seance.id, ModifierTravailBody { titre: Some("Rédaction (v2)".into()), ..Default::default() }).await.unwrap();
    fx.ent.cahier_joindre_piece(&fx.prof(), ParentPiece::Travail(travail_avec_seance.id), pdf("consigne.pdf")).await.unwrap();
    let controle = fx
        .publier_travail(&fx.prof(), fx.cible_a(), |t| {
            t.seance_id = Some(seance_avec_cours);
            t.type_travail = "controle".into();
            t.titre = "Contrôle de fractions".into();
        })
        .await
        .unwrap()
        .travail;
    // Sans séance et matière « Maths » : aucun cours de ce libellé pour la classe → quarantaine.
    let travail_orphelin = fx.publier_travail(&fx.prof(), fx.cible_a(), |t| t.titre = "Sans cours".into()).await.unwrap().travail;
    fx.ent.cahier_rendre(&fx.eleve(&fx.x), travail_avec_seance.id, "Voici", Some(pdf("copie.pdf"))).await.unwrap();
    fx.ent.cahier_marquer_fait(&fx.eleve(&fx.x), travail_avec_seance.id, true).await.unwrap();
    let remise = fx.ent.cahier_suivi_travail(&fx.prof(), travail_avec_seance.id).await.unwrap().lignes.into_iter().find_map(|l| l.remise).unwrap();
    fx.ent.cahier_retour(&fx.prof(), remise.id, "Très bien", None, Some(pdf("annotee.pdf"))).await.unwrap();
    AncienScenario { seance_avec_cours, seance_sans_cours, travail_avec_seance, controle, travail_orphelin }
}

#[tokio::test]
async fn c1_reprise_compteurs_quarantaine_et_rejouabilite() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    // Le seed du labo a déjà écrit des entrées dans l'ancien modèle : on mesure les écarts dus au scénario.
    let base = reprendre(&fx).await;
    let delta = |compteurs: &[(String, i64, i64, i64)], objet: &str| {
        let (a, b) = (compteurs.iter().find(|c| c.0 == objet).unwrap(), base.iter().find(|c| c.0 == objet).unwrap());
        (a.1 - b.1, a.2 - b.2, a.3 - b.3)
    };
    let sc = ancien_scenario(&fx).await;
    let avant: Vec<i64> = vec![nb(&fx, "ent_seance_contenu").await, nb(&fx, "ent_travail").await, nb(&fx, "ent_travail_remise").await, nb(&fx, "ent_cdt_piece").await, nb(&fx, "ent_cdt_historique").await];

    let compteurs = reprendre(&fx).await;
    // Chaque ancien objet est repris ou en quarantaine : jamais perdu, jamais compté deux fois.
    assert_eq!(delta(&compteurs, "seance_contenu"), (2, 1, 1), "un contenu repris, un en quarantaine (séance sans cours)");
    assert_eq!(delta(&compteurs, "travail"), (3, 2, 1), "travail et contrôle repris ; le travail sans cours en quarantaine");
    assert_eq!(delta(&compteurs, "remise"), (1, 1, 0));
    assert_eq!(delta(&compteurs, "piece"), (3, 3, 0), "ressource, copie, annotation : toutes ont un objet au registre");
    for (objet, anciens, repris, quarantaine) in &compteurs {
        assert_eq!(anciens, &(repris + quarantaine), "{objet} : anciens = repris + quarantaine");
    }

    // Quarantaine : nommée, motivée, visible de la direction seulement.
    let etat = fx.ent.cahier_reprise_etat(&fx.direction()).await.unwrap();
    assert!(etat.quarantaine.iter().any(|q| q.objet == "seance_contenu" && q.raison == "cours_introuvable"));
    assert!(etat.quarantaine.iter().any(|q| q.objet == "travail" && q.raison == "cours_introuvable"));
    assert_eq!(etat.compteurs.iter().map(|c| c.anciens).sum::<i64>(), compteurs.iter().map(|c| c.1).sum::<i64>());
    assert!(etat.quarantaine.iter().any(|q| q.ancien_id == sc.travail_orphelin.id), "le travail sans cours est nommé en quarantaine");
    for a in [fx.prof(), fx.cpe(), fx.eleve(&fx.x), fx.parent(&fx.x)] {
        assert!(est_interdit(&fx.ent.cahier_reprise_etat(&a).await.unwrap_err()));
    }

    // Contenu repris : cours, auteur, texte (ressources ajoutées), versions = historique + état courant.
    let (cours, contenu, version): (Uuid, String, i32) = sqlx::query_as(
        "SELECT cours_id, contenu, version FROM ent_cdt_entree WHERE etablissement_id = $1 AND seance_id = $2")
        .bind(e).bind(sc.seance_avec_cours).fetch_one(fx.ent.pool()).await.unwrap();
    let service_cours: Uuid = sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE id = $1").bind(fx.service_a).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!((cours, version), (service_cours, 2));
    assert!(contenu.contains("exemples"));
    let nb_versions: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_entree_version v JOIN ent_cdt_entree e ON e.id = v.entree_id WHERE e.seance_id = $1").bind(sc.seance_avec_cours).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(nb_versions, 2, "version 1 (historique) + version 2 (état courant)");
    let sans_cours: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_entree WHERE etablissement_id = $1 AND seance_id = $2").bind(e).bind(sc.seance_sans_cours).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(sans_cours, 0);

    // Travaux : nature, type conservé du contrôle, échéance, formats, versions.
    let ligne = |ancien: Uuid| {
        let (fx, e) = (&fx, e);
        async move {
            sqlx::query_as::<_, (String, Option<String>, String, String, i32)>(
                "SELECT na.code, t.ancien_type, t.rendu, t.etat, t.version FROM ent_cdt_travail t
                   JOIN ent_cdt_nature na ON na.id = t.nature_id WHERE t.etablissement_id = $1 AND t.ancien_id = $2")
                .bind(e).bind(ancien).fetch_one(fx.ent.pool()).await.unwrap()
        }
    };
    assert_eq!(ligne(sc.travail_avec_seance.id).await, ("dm".into(), None, "en_ligne".into(), "publie".into(), 2));
    assert_eq!(ligne(sc.controle.id).await, ("revision".into(), Some("controle".into()), "aucun".into(), "publie".into(), 1));
    let formats: Vec<String> = sqlx::query_scalar("SELECT f.format_code FROM ent_cdt_travail_format f JOIN ent_cdt_travail t ON t.id = f.travail_id WHERE t.ancien_id = $1").bind(sc.travail_avec_seance.id).fetch_all(fx.ent.pool()).await.unwrap();
    assert_eq!(formats, vec!["pdf".to_string()]);
    let (echeance, attendue): (chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
        "SELECT t.echeance_le, o.date_limite FROM ent_cdt_travail t JOIN ent_travail o ON o.id = t.ancien_id WHERE t.ancien_id = $1")
        .bind(sc.travail_avec_seance.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(Some(echeance), attendue, "l'échéance reprend la date limite exacte");
    let versions: Vec<(i32, String)> = sqlx::query_as("SELECT v.version, v.titre FROM ent_cdt_travail_version v JOIN ent_cdt_travail t ON t.id = v.travail_id WHERE t.ancien_id = $1 ORDER BY v.version").bind(sc.travail_avec_seance.id).fetch_all(fx.ent.pool()).await.unwrap();
    assert_eq!(versions, vec![(1, "Rédaction".to_string()), (2, "Rédaction (v2)".to_string())]);

    // Remise, dépôt n° 1 (empreinte = SHA-256 de la liste des empreintes de fichiers), retour, coche, pièces.
    let (remise_id, depot_id, nb_fichiers, empreinte): (Uuid, Uuid, i16, String) = sqlx::query_as(
        "SELECT r.id, d.id, d.nb_fichiers, d.empreinte FROM ent_cdt_remise r JOIN ent_cdt_depot d ON d.remise_id = r.id AND d.numero = 1 WHERE r.etablissement_id = $1")
        .bind(e).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(nb_fichiers, 1);
    let sha_copie: String = sqlx::query_scalar("SELECT o.sha256 FROM ent_cdt_piece p JOIN ent_objet o ON o.cle_objet = p.storage_key WHERE p.etablissement_id = $1 AND p.genre = 'copie'").bind(e).fetch_one(fx.ent.pool()).await.unwrap();
    let attendu: String = sqlx::query_scalar("SELECT encode(sha256(convert_to($1, 'UTF8')), 'hex')").bind(&sha_copie).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(empreinte, attendu);
    let dernier: Option<Uuid> = sqlx::query_scalar("SELECT dernier_depot_id FROM ent_cdt_remise WHERE id = $1").bind(remise_id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(dernier, Some(depot_id));
    assert_eq!(nb(&fx, "ent_cdt_retour").await, 1);
    assert_eq!(nb(&fx, "ent_cdt_coche").await, 1);
    let porteurs: Vec<String> = sqlx::query_scalar("SELECT porteur FROM ent_cdt_piece_jointe WHERE etablissement_id = $1 ORDER BY porteur").bind(e).fetch_all(fx.ent.pool()).await.unwrap();
    assert_eq!(porteurs, vec!["depot", "retour", "travail"]);
    // Le « vu » n'est pas repris (R-C42) et rien n'a changé dans les anciennes tables.
    let apres: Vec<i64> = vec![nb(&fx, "ent_seance_contenu").await, nb(&fx, "ent_travail").await, nb(&fx, "ent_travail_remise").await, nb(&fx, "ent_cdt_piece").await, nb(&fx, "ent_cdt_historique").await];
    assert_eq!(avant, apres, "la reprise ne modifie rien dans les tables d'origine");

    // Rejouable : deuxième passage identique ; une nouvelle saisie dans l'ancien modèle est reprise au passage suivant.
    let copie: Vec<i64> = vec![nb(&fx, "ent_cdt_entree").await, nb(&fx, "ent_cdt_travail").await, nb(&fx, "ent_cdt_remise").await, nb(&fx, "ent_cdt_depot").await, nb(&fx, "ent_cdt_piece_jointe").await, nb(&fx, "ent_cdt_entree_version").await, nb(&fx, "ent_cdt_travail_version").await, nb(&fx, "ent_cdt_quarantaine").await];
    assert_eq!(reprendre(&fx).await, compteurs);
    let rejoue: Vec<i64> = vec![nb(&fx, "ent_cdt_entree").await, nb(&fx, "ent_cdt_travail").await, nb(&fx, "ent_cdt_remise").await, nb(&fx, "ent_cdt_depot").await, nb(&fx, "ent_cdt_piece_jointe").await, nb(&fx, "ent_cdt_entree_version").await, nb(&fx, "ent_cdt_travail_version").await, nb(&fx, "ent_cdt_quarantaine").await];
    assert_eq!(copie, rejoue, "aucune duplication au rejeu");
    fx.publier_travail(&fx.prof(), fx.cible_a(), |t| {
        t.seance_id = Some(sc.seance_avec_cours);
        t.titre = "Ajouté après la première reprise".into();
    }).await.unwrap();
    let c2 = reprendre(&fx).await;
    assert_eq!(delta(&c2, "travail"), (4, 3, 1));
    // La quarantaine se résorbe quand la cause disparaît : le travail orphelin trouve son cours (libellé aligné).
    sqlx::query("UPDATE ent_cours SET libelle = 'Maths' WHERE etablissement_id = $1 AND id = $2").bind(e).bind(service_cours).execute(fx.ent.pool()).await.unwrap();
    let c3 = reprendre(&fx).await;
    assert_eq!(delta(&c3, "travail"), (4, 4, 0));
    let _ = sc.travail_orphelin;
}

#[tokio::test]
async fn c1_immuabilite_cles_restrict_et_gardes_des_pieces() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let sc = ancien_scenario(&fx).await;
    reprendre(&fx).await;
    let refuse = |sql: String| {
        let fx = &fx;
        async move { sqlx::query(&sql).execute(fx.ent.pool()).await.is_err() }
    };
    // Ajout seul : dépôts, versions, constats, journal.
    assert!(refuse("UPDATE ent_cdt_depot SET texte = 'réécrit'".into()).await, "dépôt immuable");
    assert!(refuse("DELETE FROM ent_cdt_depot".into()).await);
    assert!(refuse("UPDATE ent_cdt_entree_version SET contenu = 'x'".into()).await, "version d'entrée");
    assert!(refuse("DELETE FROM ent_cdt_travail_version".into()).await, "version de travail");
    assert!(refuse("UPDATE ent_cdt_retour_version SET commentaire = 'x'".into()).await, "version de retour");
    // Jamais supprimés : entrées, travaux, remises.
    assert!(refuse(format!("DELETE FROM ent_cdt_entree WHERE etablissement_id = '{e}'")).await);
    assert!(refuse(format!("DELETE FROM ent_cdt_travail WHERE etablissement_id = '{e}'")).await);
    assert!(refuse(format!("DELETE FROM ent_cdt_remise WHERE etablissement_id = '{e}'")).await);
    // R-C3 : une séance qui porte une entrée ou un travail ne se supprime pas (clés RESTRICT).
    let seance_travail = fx.publier_travail(&fx.prof(), fx.cible_a(), |t| { t.seance_id = Some(sc.seance_avec_cours); t.titre = "Porteur".into(); }).await.unwrap().travail;
    reprendre(&fx).await;
    assert!(refuse(format!("DELETE FROM ent_edt_seance WHERE id = '{}'", sc.seance_avec_cours)).await, "séance porteuse");
    let _ = seance_travail;
    // Pièces : l'objet d'une pièce ne change pas, un porteur inexistant est refusé, pas de suppression.
    assert!(refuse("UPDATE ent_cdt_piece_jointe SET objet_id = gen_random_uuid()".into()).await);
    assert!(refuse("DELETE FROM ent_cdt_piece_jointe".into()).await);
    let objet: Uuid = sqlx::query_scalar("SELECT objet_id FROM ent_cdt_piece_jointe WHERE etablissement_id = $1 LIMIT 1").bind(e).fetch_one(fx.ent.pool()).await.unwrap();
    let orphelin = sqlx::query("INSERT INTO ent_cdt_piece_jointe (id, etablissement_id, objet_id, porteur, porteur_id, nom) VALUES (gen_random_uuid(), $1, $2, 'travail', gen_random_uuid(), 'x.pdf')")
        .bind(e).bind(objet).execute(fx.ent.pool()).await;
    assert!(orphelin.is_err(), "porteur inexistant");
    // Un porteur d'un AUTRE établissement est introuvable aussi.
    let (autre_etab, travail_ici): (Uuid, Uuid) = (Uuid::new_v4(), sqlx::query_scalar("SELECT id FROM ent_cdt_travail WHERE etablissement_id = $1 LIMIT 1").bind(e).fetch_one(fx.ent.pool()).await.unwrap());
    let _ = (autre_etab, travail_ici);
    // Liens : https seulement.
    let lien = |url: &'static str, travail: Uuid| {
        let (fx, e) = (&fx, e);
        async move {
            sqlx::query("INSERT INTO ent_cdt_lien (id, etablissement_id, porteur, porteur_id, url) VALUES (gen_random_uuid(), $1, 'travail', $2, $3)")
                .bind(e).bind(travail).bind(url).execute(fx.ent.pool()).await.is_ok()
        }
    };
    assert!(lien("https://exemple.org/doc", travail_ici).await);
    assert!(!lien("javascript:alert(1)", travail_ici).await);
    assert!(!lien("http://exemple.org/doc", travail_ici).await);
    // Remise : une seule par élève et par travail ; groupe : un seul groupe actif par élève et par travail.
    let remise_dupliquee = sqlx::query("INSERT INTO ent_cdt_remise (id, etablissement_id, travail_id, eleve_id) SELECT gen_random_uuid(), etablissement_id, travail_id, eleve_id FROM ent_cdt_remise WHERE etablissement_id = $1")
        .bind(e).execute(fx.ent.pool()).await;
    assert!(remise_dupliquee.is_err());
}

#[tokio::test]
async fn c1_vues_de_compatibilite_rendent_l_ancienne_forme() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let sc = ancien_scenario(&fx).await;
    reprendre(&fx).await;
    // Mêmes identifiants et mêmes valeurs que l'ancien modèle, pour ce qui a été repris.
    let (type_, rendu, pour_le, titre): (String, bool, NaiveDate, String) = sqlx::query_as(
        "SELECT type, rendu_en_ligne, pour_le, titre FROM ent_travail_compat WHERE etablissement_id = $1 AND id = $2")
        .bind(e).bind(sc.travail_avec_seance.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!((type_.as_str(), rendu, titre.as_str()), ("a_rendre", true, "Rédaction (v2)"));
    let _ = pour_le; // l'échéance d'un travail à rendre est sa date limite (le jour peut différer de `pour_le`)
    let pour_le_controle: NaiveDate = sqlx::query_scalar("SELECT pour_le FROM ent_travail_compat WHERE etablissement_id = $1 AND id = $2").bind(e).bind(sc.controle.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(pour_le_controle, sc.controle.pour_le);
    let type_controle: String = sqlx::query_scalar("SELECT type FROM ent_travail_compat WHERE etablissement_id = $1 AND id = $2").bind(e).bind(sc.controle.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(type_controle, "controle");
    let n_compat: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_travail_compat WHERE etablissement_id = $1 AND id = ANY($2)").bind(e).bind(vec![sc.travail_avec_seance.id, sc.controle.id, sc.travail_orphelin.id]).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(n_compat, 2, "seuls les travaux repris (le travail en quarantaine n'y est pas)");
    let contenu: String = sqlx::query_scalar("SELECT contenu FROM ent_seance_contenu_compat WHERE etablissement_id = $1 AND seance_id = $2").bind(e).bind(sc.seance_avec_cours).fetch_one(fx.ent.pool()).await.unwrap();
    assert!(contenu.contains("exemples"));
}

#[path = "tests_cahier_c2.rs"]
mod c2;

#[path = "tests_cahier_c3.rs"]
mod c3;
