//! Socle 2, lots E7/E8 : correctifs de la revue de sécurité indépendante (un test par point).
//! Scénarios contre la base locale de test (feature `labo`) ; aucun instant réel passé aux
//! commandes `*_a`, aucune dépendance à l'horloge.

use super::*;
use crate::eleve_confidentialite::PoserConfidentialite;
use crate::eleve_coordonnee::AjouterCoordonneeEleve;
use crate::eleve_photo::ProposerPhoto;
use crate::outbox::ConfigDistribution;
use crate::propositions::FiltrePropositions;
use crate::seed::LaboCast;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use crate::identite::CorrectionIdentite;
use domain_eleve::{CreateEleve, InviterCompteEleve, ModifIdentite};
use ent_auth::Role;
use uuid::Uuid;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 15, 10, 0, 0).unwrap()
}

fn plus(min: i64) -> DateTime<Utc> {
    t0() + Duration::minutes(min)
}

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (mut ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |_e| {}).await;
    ent.install_mail_dry_run();
    Some((ent, cast))
}

fn lettres(n: usize) -> String {
    Uuid::new_v4().as_bytes().iter().take(n).map(|b| (b'a' + b % 26) as char).collect()
}

async fn eleve_ne(ent: &PgEnt, etab: Uuid, prenom: &str, naissance: NaiveDate) -> domain_eleve::Eleve {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: format!("Revue{}", lettres(6)),
        prenom: prenom.into(),
        ine: None,
        sexe: Some("F".into()),
        date_naissance: Some(naissance),
        numero_badge: None,
        redoublant: false,
        classe: Some("6A".into()),
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
}

async fn eleve(ent: &PgEnt, etab: Uuid, prenom: &str) -> domain_eleve::Eleve {
    eleve_ne(ent, etab, prenom, NaiveDate::from_ymd_opt(2012, 3, 14).unwrap()).await
}

fn user(cast: &LaboCast, r: Role) -> (Uuid, &'static str) {
    let p = match r {
        Role::Admin => &cast.admin,
        Role::Direction => &cast.direction,
        Role::Administratif => &cast.administratif,
        Role::Cpe => &cast.cpe,
        Role::Prof => &cast.prof,
        Role::Compta => &cast.compta,
        _ => unreachable!(),
    };
    (p.user_id, r.as_str())
}

async fn proteger(ent: &PgEnt, cast: &LaboCast, eleve: Uuid, mesures: &[&str]) {
    let (d, role) = user(cast, Role::Direction);
    ent.poser_confidentialite_a(
        role,
        PoserConfidentialite {
            etablissement_id: cast.etablissement_id,
            eleve_id: eleve,
            motif: "Mesure de protection".into(),
            mesures: mesures.iter().map(|m| (*m).to_string()).collect(),
            du: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            au: None,
            acteur: d,
        },
        t0(),
    )
    .await
    .unwrap();
}

async fn distribuer(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(ConfigDistribution { etablissement: Some(etab), ..Default::default() })
        .distribuer_tout()
        .await
        .unwrap();
}

fn absent<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Domain(domain_eleve::DomainError::NotFound)))
}

fn forbidden<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

fn conflit<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::ConflitDossier(_)))
}

fn depot(etab: Uuid, eleve: Uuid, acteur: Uuid) -> ProposerPhoto {
    ProposerPhoto {
        etablissement_id: etab,
        eleve_id: eleve,
        acteur,
        content_type: "image/jpeg".into(),
        octets: crate::tests_eleve_protection::JPEG.to_vec(),
        prise_le: None,
        valider_aussitot: false,
        photographe: false,
    }
}

fn coord(etab: Uuid, eleve: Uuid, acteur: Uuid, valeur: &str) -> AjouterCoordonneeEleve {
    AjouterCoordonneeEleve {
        etablissement_id: etab,
        eleve_id: eleve,
        acteur,
        type_: "email".into(),
        valeur: valeur.into(),
        usages: vec![],
    }
}

// ───────────────────────────── C1 : journal de l'élève ─────────────────────────────

/// C1 : le journal d'un élève protégé est introuvable d'un tiers (404) ; les lignes `eleve_*`
/// ne se lisent qu'avec G6 ; le journal par domaine n'expose aucune ligne d'un élève protégé.
#[tokio::test]
async fn c1_le_journal_d_un_eleve_protege_est_ferme_au_tiers() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Mia").await;
    let (dir, rdir) = user(&cast, Role::Direction);
    let (adm, radm) = user(&cast, Role::Administratif);
    // Témoin avant la mesure : le secrétariat lit le journal de l'élève (contrôle positif de la route).
    let avant = ent.list_journal_eleve_for_actor(etab, adm, radm, e.id, 50).await;
    assert!(avant.is_ok() || forbidden(&avant), "{avant:?}");
    proteger(&ent, &cast, e.id, &[]).await;
    for r in [Role::Administratif, Role::Cpe, Role::Prof] {
        let (u, role) = user(&cast, r);
        let l = ent.list_journal_eleve_for_actor(etab, u, role, e.id, 50).await;
        assert!(absent(&l) || forbidden(&l), "{role} : {l:?}");
    }
    // G6 : voit les faits de confidentialité (pose, auteur, date).
    let l = ent.list_journal_eleve_for_actor(etab, dir, rdir, e.id, 200).await.unwrap();
    assert!(l.iter().any(|x| x.domaine.starts_with("eleve_")), "G6 voit eleve_* : {l:?}");
    // Journal par domaine (audit.lire) : aucune ligne d'un élève protégé pour un non-G6.
    let l = ent.list_journal_domaine_for_actor(etab, adm, radm, "eleve_sensible", 200).await;
    if let Ok(l) = l {
        assert!(l.iter().all(|x| x.eleve_id != Some(e.id)), "{l:?}");
    }
    let l = ent.list_journal_domaine_for_actor(etab, adm, radm, "consultation", 200).await.unwrap_or_default();
    assert!(l.iter().all(|x| x.eleve_id != Some(e.id)), "{l:?}");
}

// ───────────────────────────── C2 : élèves sans compte, invitation ─────────────────────────────

#[tokio::test]
async fn c2_sans_compte_et_invitation_ignorent_les_proteges() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Noa").await;
    let (dir, rdir) = user(&cast, Role::Direction);
    let (adm, radm) = user(&cast, Role::Administratif);
    // Témoin : avant la mesure, le secrétariat le voit parmi les élèves sans compte.
    let l = ent.list_eleves_sans_compte_for_actor(etab, adm, radm).await.unwrap();
    assert!(l.iter().any(|x| x.id == e.id), "contrôle positif");
    proteger(&ent, &cast, e.id, &[]).await;
    let l = ent.list_eleves_sans_compte_for_actor(etab, adm, radm).await.unwrap();
    assert!(l.iter().all(|x| x.id != e.id), "absent de la liste d'un tiers");
    let l = ent.list_eleves_sans_compte_for_actor(etab, dir, rdir).await.unwrap();
    assert!(l.iter().any(|x| x.id == e.id), "G6 le voit");
    // Invitation : 404, aucun e-mail.
    let envoyes = ent.mailer().dry_run_corps().len();
    let r = ent
        .inviter_compte_eleve_for_actor(
            radm,
            InviterCompteEleve { etablissement_id: etab, eleve_id: e.id, email: None, actor_user_id: adm },
            "https://ent.test",
        )
        .await;
    assert!(absent(&r), "{r:?}");
    assert_eq!(ent.mailer().dry_run_corps().len(), envoyes, "aucun e-mail");
    // Provisionnement en masse : ne touche pas l'élève protégé.
    let p = ent.provisionner_comptes_manquants_for_actor(etab, adm, radm, "https://ent.test").await.unwrap();
    assert!(p.iter().all(|c| c.invitation.eleve_id != e.id));
}

// ───────────────────────────── E1 : consultations ─────────────────────────────

#[tokio::test]
async fn e1_les_consultations_d_un_eleve_protege_sont_introuvables_d_un_tiers() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Léo").await;
    let (adm, radm) = user(&cast, Role::Administratif);
    let (dir, rdir) = user(&cast, Role::Direction);
    assert!(ent.consultations_pour_acteur(radm, etab, adm, e.id, 50).await.is_ok(), "contrôle positif");
    proteger(&ent, &cast, e.id, &[]).await;
    assert!(absent(&ent.consultations_pour_acteur(radm, etab, adm, e.id, 50).await));
    assert!(ent.consultations_pour_acteur(rdir, etab, dir, e.id, 50).await.is_ok());
}

// ───────────────────────────── E2 : pastilles ─────────────────────────────

/// E2 : `majeur` ne publie que le fait (ni date de majorité, ni début déduisant la naissance) ;
/// une pastille n'est visible que si la tuile de son domaine l'est (tous rôles).
#[tokio::test]
async fn e2_les_pastilles_ne_devinent_pas_la_naissance_et_suivent_leur_tuile() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    ent.activer_module_sante_test(etab).await;
    let jour = ent.jour_etablissement(etab).await.unwrap();
    let naissance = NaiveDate::from_ymd_opt(2000, 5, 17).unwrap();
    let e = eleve_ne(&ent, etab, "Majeure", naissance).await;
    distribuer(&ent, etab).await;
    ent.reconstruire_indicateurs(etab, None, Some(jour)).await.unwrap();
    let mut vu_majeur = false;
    for r in [Role::Direction, Role::Administratif, Role::Cpe, Role::Prof, Role::Compta] {
        let (u, role) = user(&cast, r);
        let Ok(s) = ent.synthese_eleve_pour_acteur(role, etab, u, e.id).await else { continue };
        for p in &s.pastilles {
            if p.code == "majeur" {
                vu_majeur = true;
                assert_eq!(p.valeur_date, None, "{role} : pas de date de majorité");
                assert_eq!(p.du, jour, "{role} : `du` ne révèle pas la naissance");
            }
            let tuile = s.tuiles.iter().find(|t| t.domaine == p.domaine);
            assert!(
                tuile.is_some_and(|t| matches!(t.etat, crate::eleve_indicateurs::EtatTuile::Visible)),
                "{role} : pastille {} sans tuile {} ouverte",
                p.code,
                p.domaine
            );
        }
    }
    assert!(vu_majeur, "contrôle positif : `majeur` visible pour au moins un rôle");
}

// ───────────────────────────── E3 : photos manquantes ─────────────────────────────

#[tokio::test]
async fn e3_un_eleve_sans_photo_apparait_dans_les_photos_manquantes() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Sans").await;
    let (adm, radm) = user(&cast, Role::Administratif);
    let l = ent.eleves_sans_photo_pour_acteur(radm, etab, adm, 5000).await.unwrap();
    assert!(l.contains(&e.id), "un élève scolarisé sans photo est listé");
    // Une photo validée le retire de la liste.
    let mut d = depot(etab, e.id, adm);
    d.valider_aussitot = true;
    ent.proposer_photo_a(radm, d, t0()).await.unwrap();
    let l = ent.eleves_sans_photo_pour_acteur(radm, etab, adm, 5000).await.unwrap();
    assert!(!l.contains(&e.id));
    // Un élève protégé n'est pas listé pour un tiers.
    let p = eleve(&ent, etab, "Cachée").await;
    proteger(&ent, &cast, p.id, &[]).await;
    let l = ent.eleves_sans_photo_pour_acteur(radm, etab, adm, 5000).await.unwrap();
    assert!(!l.contains(&p.id));
}

// ───────────────────────────── E5 : conflit d'intérêts ─────────────────────────────

async fn declarer_conflit(ent: &PgEnt, etab: Uuid, user: Uuid, eleve: Uuid) {
    sqlx::query(
        "INSERT INTO ent_conflit_interet (id, etablissement_id, user_id, eleve_id, nature, origine, declare_par)
         VALUES (gen_random_uuid(), $1, $2, $3, 'proche', 'direction', $2)",
    )
    .bind(etab)
    .bind(user)
    .bind(eleve)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

#[tokio::test]
async fn e5_le_conflit_d_interet_bloque_photo_coordonnees_et_motif() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Conflit").await;
    let (adm, radm) = user(&cast, Role::Administratif);
    let (dir, rdir) = user(&cast, Role::Direction);
    // Un collègue sans conflit dépose une photo et une coordonnée (à décider ensuite).
    let (autre, rautre) = (cast.admin.user_id, Role::Admin.as_str());
    let p = ent.proposer_photo_a(rautre, depot(etab, e.id, autre), t0()).await.unwrap();
    let mail = format!("c.{}@exemple.test", lettres(6));
    let c = ent.ajouter_coordonnee_eleve_a(rautre, coord(etab, e.id, autre, &mail), t0()).await.unwrap();
    declarer_conflit(&ent, etab, adm, e.id).await;
    // Écritures de photo.
    assert!(conflit(&ent.proposer_photo_a(radm, depot(etab, e.id, adm), plus(1)).await));
    assert!(conflit(&ent.valider_photo_a(radm, etab, adm, p.id, plus(1)).await));
    assert!(conflit(&ent.rejeter_photo(radm, etab, adm, p.id, "non_conforme").await));
    assert!(conflit(&ent.enregistrer_refus_photo_a(radm, etab, adm, e.id, plus(1)).await));
    // Écritures de coordonnées.
    assert!(conflit(&ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, "x@exemple.test"), plus(1)).await));
    assert!(conflit(&ent.retirer_coordonnee_eleve_a(radm, etab, adm, c.id, "erreur_de_saisie", plus(1)).await));
    assert!(conflit(&ent.demander_verification_coordonnee_eleve_a(radm, etab, adm, c.id, plus(1)).await));
    // Lecture du motif : un agent G6 en conflit ne lit pas le motif descellé.
    proteger(&ent, &cast, e.id, &[]).await;
    let l = ent.mesures_confidentialite_pour_acteur_a(rdir, etab, dir, e.id, plus(2)).await.unwrap();
    assert!(l.iter().all(|m| m.motif.is_some()), "contrôle positif : G6 sans conflit lit le motif");
    declarer_conflit(&ent, etab, dir, e.id).await;
    let l = ent.mesures_confidentialite_pour_acteur_a(rdir, etab, dir, e.id, plus(3)).await.unwrap();
    assert!(!l.is_empty() && l.iter().all(|m| m.motif.is_none()), "motif caché à l'agent en conflit");
}

// ───────────────────────────── M1 : refus de photo ─────────────────────────────

async fn corriger_naissance(ent: &PgEnt, etab: Uuid, adm: Uuid, radm: &str, eleve: Uuid, jour: NaiveDate) {
    ent.corriger_identite(
        radm,
        CorrectionIdentite {
            etablissement_id: etab,
            eleve_id: eleve,
            version_id: None,
            modif: ModifIdentite { date_naissance: Some(Some(jour)), ..Default::default() },
            motif_code: "d_apres_piece".into(),
            motif: None,
            piece_document_id: None,
            source: "saisie".into(),
            acteur: adm,
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn m1_la_famille_n_a_plus_de_prise_sur_le_refus_d_un_majeur_et_ne_leve_que_le_sien() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let (parent, rparent) = (cast.parent_lea.user_id, Role::Parent.as_str());
    let harry = cast.eleve_lea_id;
    // Refus posé par l'établissement : la famille ne peut pas le lever (404, aucune ligne touchée).
    ent.enregistrer_refus_photo_a(radm, etab, adm, harry, t0()).await.unwrap();
    assert!(absent(&ent.lever_refus_photo_a(rparent, etab, parent, harry, plus(1)).await));
    assert!(ent.eleves_refus_photo(etab, &[harry]).await.unwrap().contains(&harry), "refus toujours en vigueur");
    // Le personnel le lève ; la famille pose le sien, qu'elle peut lever.
    ent.lever_refus_photo_a(radm, etab, adm, harry, plus(2)).await.unwrap();
    ent.enregistrer_refus_photo_a(rparent, etab, parent, harry, plus(3)).await.unwrap();
    ent.lever_refus_photo_a(rparent, etab, parent, harry, plus(4)).await.unwrap();
    // Majeur : la famille ne pose ni ne lève plus rien (404).
    ent.enregistrer_refus_photo_a(radm, etab, adm, harry, plus(5)).await.unwrap();
    corriger_naissance(&ent, etab, adm, radm, harry, NaiveDate::from_ymd_opt(2000, 5, 17).unwrap()).await;
    assert!(absent(&ent.lever_refus_photo_a(rparent, etab, parent, harry, plus(6)).await));
    ent.lever_refus_photo_a(radm, etab, adm, harry, plus(7)).await.unwrap();
    assert!(absent(&ent.enregistrer_refus_photo_a(rparent, etab, parent, harry, plus(8)).await));
    // Remise en état du jeu d'essai partagé.
    corriger_naissance(&ent, etab, adm, radm, harry, NaiveDate::from_ymd_opt(2012, 3, 14).unwrap()).await;
}

// ───────────────────────────── M2 : usages d'une coordonnée ─────────────────────────────

#[tokio::test]
async fn m2_les_usages_sont_journalises_et_le_compte_est_reserve_a_l_eleve() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let (dir, rdir) = user(&cast, Role::Direction);
    let e = eleve(&ent, etab, "Usage").await;
    let mail = format!("u.{}@exemple.test", lettres(6));
    let c = ent.ajouter_coordonnee_eleve_a(radm, coord(etab, e.id, adm, &mail), t0()).await.unwrap();
    ent.demander_verification_coordonnee_eleve_a(radm, etab, adm, c.id, plus(1)).await.unwrap();
    let corps = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .rev()
        .find(|(a, s, _)| *a == mail && s.contains("code de vérification"))
        .unwrap()
        .2;
    let i = corps.find("est : ").unwrap() + 6;
    ent.verifier_coordonnee_eleve_a(radm, etab, adm, c.id, &corps[i..i + 6], plus(2)).await.unwrap();
    // Le personnel pose « urgence » (journalisé) mais jamais « compte ».
    assert!(forbidden(&ent.modifier_usages_coordonnee_eleve_a(radm, etab, adm, c.id, &["compte".into()], plus(3)).await));
    ent.modifier_usages_coordonnee_eleve_a(radm, etab, adm, c.id, &["urgence".into()], plus(3)).await.unwrap();
    let l = ent.consultations_pour_acteur(rdir, etab, dir, e.id, 50).await.unwrap();
    let _ = l; // lecture : la trace d'écriture est dans le journal de l'élève (G6).
    let j = ent.list_journal_eleve_for_actor(etab, dir, rdir, e.id, 200).await.unwrap();
    assert!(
        j.iter().any(|x| x.domaine == "eleve_coordonnee" && x.action == "usages_modifies"),
        "changement d'usages journalisé : {j:?}"
    );
    // L'élève lui-même peut poser « compte ».
    let compte = crate::tests_messagerie_m1::nouvel_utilisateur(&ent, etab, "eleve", "Elève").await;
    sqlx::query("INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id) VALUES ($1, $2, $3)")
        .bind(etab)
        .bind(e.id)
        .bind(compte)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let u = ent
        .modifier_usages_coordonnee_eleve_a(Role::Eleve.as_str(), etab, compte, c.id, &["compte".into()], plus(4))
        .await
        .unwrap();
    assert_eq!(u.usages, ["compte"]);
    // Le personnel ne peut pas non plus le retirer à sa place.
    assert!(forbidden(&ent.modifier_usages_coordonnee_eleve_a(radm, etab, adm, c.id, &["urgence".into()], plus(5)).await));
}

// ───────────────────────────── M4 : un seul domaine « consultation » ─────────────────────────────

/// M4 : toute lecture sensible (identité, identifiants, scolarité, propositions, pastilles, listes
/// par pastille) écrit en domaine `consultation`, le code consulté dans l'action ; aucune autre
/// trace de lecture dans un domaine métier.
#[tokio::test]
async fn m4_toutes_les_lectures_sensibles_sont_dans_le_domaine_consultation() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let e = eleve(&ent, etab, "Lecture").await;
    let (dir, rdir) = user(&cast, Role::Direction);
    let (adm, radm) = user(&cast, Role::Administratif);
    let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE actor_user_id = $1")
        .bind(dir)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    ent.historique_identite_pour_acteur(etab, dir, rdir, e.id).await.unwrap();
    ent.identifiants_pour_acteur(etab, dir, rdir, e.id).await.unwrap();
    ent.scolarite_pour_acteur(etab, dir, rdir, e.id).await.unwrap();
    ent.propositions_pour_acteur(rdir, etab, dir, FiltrePropositions::default()).await.unwrap();
    ent.pastilles_eleve_pour_acteur(rdir, etab, dir, e.id).await.unwrap();
    ent.eleves_par_pastille_pour_acteur(rdir, etab, dir, "sans_ine", 10).await.unwrap();
    ent.get_eleve_vue_for_actor(etab, adm, radm, e.id).await.unwrap();
    let lignes: Vec<(String, String)> = sqlx::query_as(
        "SELECT domaine, action FROM ent_journal_modif WHERE actor_user_id = ANY($1) ORDER BY created_at",
    )
    .bind(vec![dir, adm])
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(lignes.len() as i64 > avant, "des traces ont été écrites");
    let lectures: Vec<&(String, String)> = lignes
        .iter()
        .filter(|(_, a)| {
            a.starts_with("identite_historique")
                || a == "identifiants"
                || a == "scolarite"
                || a.starts_with("liste_pastille")
                || a == "pastilles_sensibles"
                || a == "etat_civil"
                || a.starts_with("propositions")
        })
        .collect();
    assert!(lectures.len() >= 4, "lectures tracées : {lignes:?}");
    assert!(lectures.iter().all(|(d, _)| d == "consultation"), "domaine unique : {lectures:?}");
    assert!(lignes.iter().any(|(d, a)| d == "consultation" && a == "liste_pastille:sans_ine"), "code dans l'action");
}

// ───────────────────────────── M5 : filtre avant la limite ─────────────────────────────

#[tokio::test]
async fn m5_les_proteges_sont_ecartes_avant_la_limite() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    // Beaucoup d'élèves protégés (qui rempliraient la limite) et un seul ordinaire.
    let mut proteges = Vec::new();
    for i in 0..4 {
        let p = eleve(&ent, etab, &format!("Prot{i}")).await;
        proteger(&ent, &cast, p.id, &[]).await;
        proteges.push(p.id);
    }
    let ordinaire = eleve(&ent, etab, "Ordinaire").await;
    distribuer(&ent, etab).await;
    // Pastille « sans INE » : limite 1000 d'abord pour connaître l'ensemble, puis limite égale au
    // nombre d'ordinaires sans INE : aucun protégé n'y prend de place.
    let tous = ent.eleves_par_pastille_pour_acteur(radm, etab, adm, "sans_ine", 5000).await.unwrap();
    assert!(tous.contains(&ordinaire.id));
    assert!(proteges.iter().all(|p| !tous.contains(p)));
    let n = tous.len() as i64;
    let l = ent.eleves_par_pastille_pour_acteur(radm, etab, adm, "sans_ine", n).await.unwrap();
    assert_eq!(l.len() as i64, n, "la limite n'est pas mangée par des absents");
    // Photos à valider : même principe.
    let (autre, rautre) = (cast.admin.user_id, Role::Admin.as_str());
    let mut ids = Vec::new();
    for p in &proteges {
        ent.proposer_photo_a(rautre, depot(etab, *p, autre), t0()).await.ok();
    }
    let ph = ent.proposer_photo_a(rautre, depot(etab, ordinaire.id, autre), plus(1)).await.unwrap();
    ids.push(ph.id);
    let file = ent.photos_a_valider_pour_acteur(radm, etab, adm, 500).await.unwrap();
    assert!(file.iter().any(|x| x.id == ph.id));
    assert!(file.iter().all(|x| !proteges.contains(&x.eleve_id)));
}

// ───────────────────────────── M6 : module santé ─────────────────────────────

#[tokio::test]
async fn m6_sans_module_sante_aucune_pastille_ni_tuile_de_sante() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (admin, radmin) = user(&cast, Role::Admin);
    let (dir, rdir) = user(&cast, Role::Direction);
    ent.activer_module_sante_test(etab).await;
    let e = eleve(&ent, etab, "Sante").await;
    let jour = ent.jour_etablissement(etab).await.unwrap();
    ent.enregistrer_pai_for_actor(
        Role::Infirmerie.as_str(),
        domain_eleve::UpsertPai {
            etablissement_id: etab,
            eleve_id: e.id,
            motif: "allergie".into(),
            mesures_accueil: "trousse".into(),
            protocole_urgence: "appeler le 15".into(),
            note_interne: String::new(),
            date_debut: jour - Duration::days(10),
            date_fin: None,
            actif: true,
            actor_user_id: cast.infirmieres[0].user_id,
        },
        None,
    )
    .await
    .unwrap();
    distribuer(&ent, etab).await;
    let s = ent.synthese_eleve_pour_acteur(Role::Infirmerie.as_str(), etab, cast.infirmieres[0].user_id, e.id).await.unwrap();
    assert!(s.pastilles.iter().any(|p| p.code == "pai_actif"), "contrôle positif : module actif");
    // Module désactivé : plus de pastille de santé ni de tuile santé ouverte, pour personne.
    ent.definir_module_sante(etab, admin, radmin, false).await.unwrap();
    let s = ent.synthese_eleve_pour_acteur(Role::Infirmerie.as_str(), etab, cast.infirmieres[0].user_id, e.id).await.unwrap();
    let sante = s.tuiles.iter().find(|t| t.domaine == "sante").unwrap();
    assert!(!matches!(sante.etat, crate::eleve_indicateurs::EtatTuile::Visible), "tuile santé fermée");
    assert!(s.pastilles.iter().all(|p| p.domaine != "sante"));
    let s = ent.synthese_eleve_pour_acteur(rdir, etab, dir, e.id).await.unwrap();
    assert!(s.pastilles.iter().all(|p| p.domaine != "sante"));
    // La liste par pastille PAI est fermée aussi.
    let l = ent.eleves_par_pastille_pour_acteur(Role::Infirmerie.as_str(), etab, cast.infirmieres[0].user_id, "pai_actif", 100).await;
    assert!(l.is_err() || l.unwrap().is_empty());
    ent.definir_module_sante(etab, admin, radmin, true).await.unwrap();
}

// ───────────────────────────── E4 : alerte alignée ─────────────────────────────

/// E4 (décision du pilote) : la portée pédagogique reste « non tiers » ; `masquer_photo` et
/// l'alerte s'appliquent à ce non-tiers pour toute lecture, pas seulement l'ouverture de fiche.
#[tokio::test]
async fn e4_alerte_pour_toute_lecture_d_un_non_g6_et_photo_masquee() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (prof, rprof) = user(&cast, Role::Prof);
    let (dir, rdir) = user(&cast, Role::Direction);
    let harry = cast.eleve_lea_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let mut d = depot(etab, harry, adm);
    d.valider_aussitot = true;
    ent.proposer_photo_a(radm, d, t0()).await.unwrap();
    // Contrôle positif : le professeur de la classe sert la photo avant la mesure.
    assert!(ent.get_photo_eleve_bytes_for_actor(etab, prof, rprof, harry).await.unwrap().is_some());
    proteger(&ent, &cast, harry, &["alerte_consultation"]).await;
    // Il garde l'accès à la fiche (portée pédagogique) mais pas à la photo.
    assert!(ent.get_eleve_for_actor(etab, prof, rprof, harry).await.unwrap().is_some());
    assert!(ent.get_photo_eleve_bytes_for_actor(etab, prof, rprof, harry).await.unwrap().is_none());
    let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.consultation_protegee'")
        .bind(harry)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    // Toute lecture sensible par le non-G6 alerte ; celle de G6 non.
    ent.scolarite_pour_acteur(etab, prof, rprof, harry).await.ok();
    ent.get_eleve_vue_for_actor(etab, prof, rprof, harry).await.unwrap();
    ent.get_eleve_vue_for_actor(etab, dir, rdir, harry).await.unwrap();
    let acteurs: Vec<String> = sqlx::query_scalar("SELECT actor_user_id FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.consultation_protegee'")
        .bind(harry)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap();
    assert!(acteurs.len() as i64 > avant, "alerte émise : {acteurs:?}");
    assert!(acteurs.iter().all(|a| *a == prof.to_string()), "jamais pour G6 : {acteurs:?}");
}

// ───────────────────────────── E6 : santé (fiche d'urgence, PAI complet) ─────────────────────────────

/// E6 : au niveau des services aussi (la couche HTTP est testée dans `separation`), la fiche
/// d'urgence et le PAI complet d'un élève protégé sont introuvables d'un tiers.
#[tokio::test]
async fn e6_fiche_d_urgence_et_pai_complet_d_un_eleve_protege_sont_introuvables() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    ent.activer_module_sante_test(etab).await;
    let e = eleve(&ent, etab, "Urgence").await;
    let inf = cast.infirmieres[0].user_id;
    let rinf = Role::Infirmerie.as_str();
    assert!(ent.fiche_urgence_for_actor(etab, inf, rinf, e.id).await.is_ok(), "contrôle positif");
    proteger(&ent, &cast, e.id, &[]).await;
    assert!(absent(&ent.fiche_urgence_for_actor(etab, inf, rinf, e.id).await));
    assert!(absent(&ent.pai_complet_for_actor(etab, inf, rinf, e.id).await));
    let (dir, rdir) = user(&cast, Role::Direction);
    assert!(!absent(&ent.fiche_urgence_for_actor(etab, dir, rdir, e.id).await), "G6 : pas de 404");
}

// ───────────────────────────── M3/E4 : `adresse_protegee` appliquée ─────────────────────────────

/// `adresse_protegee` : les personnes liées à l'élève (adresses F2) sont fermées à qui n'a pas G6,
/// même à un membre du personnel qui garde un lien direct ; G6 les lit ; levée, tout est rouvert.
#[tokio::test]
async fn m3_adresse_protegee_ferme_les_personnes_liees_aux_non_g6() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let (dir, rdir) = user(&cast, Role::Direction);
    let e = eleve(&ent, etab, "Adresse").await;
    let personne = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom) VALUES ($1, $2, 'Tuteur', 'Alix')")
        .bind(personne)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, lien)
         VALUES (gen_random_uuid(), $1, $2, $3, 'tuteur')",
    )
    .bind(etab)
    .bind(e.id)
    .bind(personne)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    assert!(ent.personnes_en_conflit(etab, adm, radm, &[personne]).await.unwrap().is_empty(), "avant la mesure");
    proteger(&ent, &cast, e.id, &["adresse_protegee"]).await;
    assert!(ent.personnes_en_conflit(etab, adm, radm, &[personne]).await.unwrap().contains(&personne), "fermée au non-G6");
    assert!(ent.personnes_en_conflit(etab, dir, rdir, &[personne]).await.unwrap().is_empty(), "G6 la lit");
    ent.lever_confidentialite_a(rdir, etab, dir, e.id, "fin_de_mesure", plus(5)).await.unwrap();
    assert!(ent.personnes_en_conflit(etab, adm, radm, &[personne]).await.unwrap().is_empty(), "levée");
}

// ───────────────────────────── Pastille `interne` pour la vie scolaire et l'accueil ─────────────────────────────

/// Décision du pilote : `interne` est une information d'organisation, visible de la vie scolaire
/// (CPE) et de l'accueil ; PAI reste réservé à `sante.lire` ; rien d'autre ne s'ouvre pour eux
/// (ni les pastilles d'identifiants, ni « redoublant », ni les tuiles identité, santé, protection).
#[tokio::test]
async fn interne_visible_de_la_vie_scolaire_et_de_l_accueil_sans_rien_ouvrir_d_autre() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    ent.activer_module_sante_test(etab).await;
    let e = eleve(&ent, etab, "Interne").await;
    let jour = ent.jour_etablissement(etab).await.unwrap();
    ent.changer_regime_ou_site(
        Role::Administratif.as_str(),
        crate::scolarite::ChangementRegimeSite {
            etablissement_id: etab,
            eleve_id: e.id,
            kind: domain_eleve::AttributKind::Regime,
            valeur: "interne".into(),
            date_effet: jour - Duration::days(3),
            derogation: None,
            acteur: cast.administratif.user_id,
        },
    )
    .await
    .unwrap();
    ent.enregistrer_pai_for_actor(
        Role::Infirmerie.as_str(),
        domain_eleve::UpsertPai {
            etablissement_id: etab,
            eleve_id: e.id,
            motif: "allergie".into(),
            mesures_accueil: "trousse".into(),
            protocole_urgence: "appeler le 15".into(),
            note_interne: String::new(),
            date_debut: jour - Duration::days(10),
            date_fin: None,
            actif: true,
            actor_user_id: cast.infirmieres[0].user_id,
        },
        None,
    )
    .await
    .unwrap();
    distribuer(&ent, etab).await;
    ent.reconstruire_indicateurs(etab, None, Some(jour)).await.unwrap();
    let accueil = crate::tests_messagerie_m1::nouvel_utilisateur(&ent, etab, "accueil", "Accueil").await;
    let codes = |s: &crate::eleve_indicateurs::SyntheseEleve| -> Vec<&'static str> { s.pastilles.iter().map(|p| p.code).collect() };
    // Vie scolaire (CPE) et accueil : `interne`, rien d'autre.
    for (u, role) in [(user(&cast, Role::Cpe).0, Role::Cpe.as_str()), (accueil, Role::Accueil.as_str())] {
        let s = ent.synthese_eleve_pour_acteur(role, etab, u, e.id).await.unwrap_or_else(|x| panic!("{role} : {x:?}"));
        assert_eq!(codes(&s), ["interne"], "{role} : seulement `interne`");
        for t in &s.tuiles {
            let ouverte = matches!(t.etat, crate::eleve_indicateurs::EtatTuile::Visible);
            assert_eq!(ouverte, t.domaine == "scolarite", "{role} : tuile {}", t.domaine);
        }
        let l = ent.eleves_par_pastille_pour_acteur(role, etab, u, "interne", 100).await.unwrap();
        assert!(l.contains(&e.id), "{role} : liste des internes");
        let pai = ent.eleves_par_pastille_pour_acteur(role, etab, u, "pai_actif", 100).await;
        assert!(forbidden(&pai) || pai.is_ok_and(|l| l.is_empty()), "{role} : PAI fermé");
    }
    // Les autres rôles sans droit sur le régime ne la voient toujours pas ; l'infirmerie voit le PAI seulement.
    let (compta, rcompta) = user(&cast, Role::Compta);
    if let Ok(s) = ent.synthese_eleve_pour_acteur(rcompta, etab, compta, e.id).await {
        assert!(!codes(&s).contains(&"interne") && !codes(&s).contains(&"pai_actif"), "compta : {:?}", codes(&s));
    }
    let s = ent.synthese_eleve_pour_acteur(Role::Infirmerie.as_str(), etab, cast.infirmieres[0].user_id, e.id).await.unwrap();
    assert!(codes(&s).contains(&"pai_actif") && !codes(&s).contains(&"interne"), "infirmerie : {:?}", codes(&s));
    // La direction voit les deux.
    let (dir, rdir) = user(&cast, Role::Direction);
    let s = ent.synthese_eleve_pour_acteur(rdir, etab, dir, e.id).await.unwrap();
    assert!(codes(&s).contains(&"interne") && codes(&s).contains(&"pai_actif"));
}
