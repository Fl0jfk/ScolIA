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
    fx.ent.cahier_modifier_travail(&fx.direction(), t.id, modif("Exercices (direction)")).await.unwrap();
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
    assert!(fx.ent.cahier_lien_piece(&fx.parent(&fx.x), copie_x).await.is_ok());
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
    assert!(fait_de(&fx, fx.parent(&fx.x), Some(fx.x.id), du, au).await, "la famille voit l'état de son enfant");
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
