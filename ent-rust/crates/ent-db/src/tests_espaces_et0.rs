//! Socle 32, lot ET0 — préalables du noyau : envoi direct à **taille signée**, **quota réservé atomiquement**, objet
//! **non analysé invisible**, réconciliation (`32_BUREAUTIQUE.md` §15, §20.4, §27.1 ; R-ET20, R-ET21 ; CET-14, CET-15, CET-18,
//! CET-19 ; ND32-01, ND32-05, ND32-14, ND32-15).
//!
//! Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**. Aucun test ne dépend de l'heure réelle (règle 33) : les
//! instants sont fixes ; le magasin est le **magasin mémoire** du labo (aucun accès au S3 réel) ; « le fournisseur » qui
//! reçoit l'envoi direct est simulé par une écriture dans ce magasin sous la clé réservée.

use super::*;
use crate::espaces::*;
use crate::tests_formulaires_fo0::labo;
use chrono::{DateTime, Duration, TimeZone, Utc};
use ent_auth::Role;
use ent_object::presign::Presigner;
use ent_object::upload::{AnalyseurAntivirus, ContexteAnalyse, VerdictAnalyse};
use std::sync::Arc;
use uuid::Uuid;

pub(crate) const MIO: usize = 1024 * 1024;
const SECRET: &[u8] = b"0123456789abcdef0123456789abcdef-secret-et0";

/// Instant de référence des tests (un mercredi de mars 2027, midi à Paris) : jamais l'heure réelle.
pub(crate) fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2027, 3, 10, 11, 0, 0).unwrap()
}

/// Un PDF valide de la taille demandée (signature + remplissage), sans actif.
pub(crate) fn pdf(octets: usize) -> Vec<u8> {
    let mut v = b"%PDF-1.4\n1 0 obj << /Type /Catalog >> endobj\n".to_vec();
    v.resize(octets.max(v.len()), b' ');
    v
}

/// Un PDF distinct à chaque appel (contenu différent, donc empreinte différente).
pub(crate) fn pdf_unique(octets: usize) -> Vec<u8> {
    let mut v = format!("%PDF-1.4\n% {}\n", Uuid::new_v4()).into_bytes();
    v.resize(octets.max(v.len()), b' ');
    v
}

pub(crate) struct Verdict(pub VerdictAnalyse);

impl AnalyseurAntivirus for Verdict {
    fn nom(&self) -> &'static str {
        "test"
    }
    fn analyser(&self, _ctx: &ContexteAnalyse<'_>) -> VerdictAnalyse {
        self.0.clone()
    }
}

pub(crate) fn sain() -> Arc<dyn AnalyseurAntivirus> {
    Arc::new(Verdict(VerdictAnalyse::Sain))
}

/// Base de test : magasin mémoire du labo, jetons locaux à secret fixe, session « personnels » du labo.
pub(crate) async fn labo_espaces() -> Option<(PgEnt, crate::LaboCast)> {
    let (mut ent, cast) = labo().await?;
    ent.installer_presigner(Presigner::local(SECRET, 60).unwrap());
    Some((ent, cast))
}

/// La personne (fiche) d'un compte de l'établissement, créée si besoin, **agent actif** depuis longtemps : un collègue
/// de l'annuaire du socle 6 (R-ET12).
pub(crate) async fn collegue(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    let pool = ent.pool_verif();
    let existante: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
    )
    .bind(etab)
    .bind(user)
    .fetch_optional(pool)
    .await
    .unwrap();
    let personne = match existante {
        Some(p) => p,
        None => {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, user_id, source) VALUES ($1, $2, $3, $4, 'compte')")
                .bind(id)
                .bind(etab)
                .bind(format!("Test{}", &Uuid::new_v4().simple().to_string()[..8]))
                .bind(user)
                .execute(pool)
                .await
                .unwrap();
            id
        }
    };
    sqlx::query(
        "INSERT INTO ent_agent (id, etablissement_id, personne_id, source) VALUES ($1, $2, $3, 'saisie')
         ON CONFLICT (etablissement_id, personne_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(personne)
    .execute(pool)
    .await
    .unwrap();
    let agent: Uuid = sqlx::query_scalar("SELECT id FROM ent_agent WHERE etablissement_id = $1 AND personne_id = $2")
        .bind(etab)
        .bind(personne)
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_agent_engagement (id, etablissement_id, agent_id, du, nature, employeur_type)
         SELECT gen_random_uuid(), $1, $2, DATE '2020-01-01', 'titulaire', 'etat'
          WHERE NOT EXISTS (SELECT 1 FROM ent_agent_engagement WHERE etablissement_id = $1 AND agent_id = $2)",
    )
    .bind(etab)
    .bind(agent)
    .execute(pool)
    .await
    .unwrap();
    personne
}

/// Mon espace personnel, ouvert à `t0`.
pub(crate) async fn mon_espace(ent: &PgEnt, etab: Uuid, user: Uuid, role: &str) -> Uuid {
    collegue(ent, etab, user).await;
    ent.espaces_ouvrir_mon_espace(etab, user, role, t0()).await.unwrap().id
}

/// Un espace libre neuf (compteurs vides) : les tests de quota ne dépendent pas de ce que d'autres tests ont déposé.
pub(crate) async fn espace_neuf(ent: &PgEnt, etab: Uuid, user: Uuid, role: &str) -> Uuid {
    collegue(ent, etab, user).await;
    ent.espaces_creer_libre(
        etab,
        user,
        role,
        NouvelEspace {
            libelle: format!("Espace {}", &Uuid::new_v4().simple().to_string()[..8]),
            description: None,
            couleur_code: None,
            icone_code: None,
            membres: vec![],
        },
        t0(),
    )
    .await
    .unwrap()
    .id
}

/// Fixe le quota d'un espace (préparation par le propriétaire de la base).
pub(crate) async fn fixer_quota(ent: &PgEnt, etab: Uuid, espace: Uuid, octets: i64) {
    sqlx::query("UPDATE ent_espace SET quota_octets = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(espace)
        .bind(octets)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

/// Clé d'objet réservée (le navigateur ne la connaît pas : elle est dans le jeton) — lue par le propriétaire de la base.
pub(crate) async fn cle_reservee(ent: &PgEnt, etab: Uuid, reservation: Uuid) -> String {
    sqlx::query_scalar("SELECT cle_objet FROM ent_espace_reservation WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(reservation)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

pub(crate) async fn occupation(ent: &PgEnt, etab: Uuid, espace: Uuid) -> (i64, i64) {
    sqlx::query_as("SELECT octets_confirmes, octets_reserves FROM ent_espace_occupation WHERE etablissement_id = $1 AND espace_id = $2")
        .bind(etab)
        .bind(espace)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

pub(crate) fn demande(nom: &str, taille: usize) -> DemandeDepot {
    DemandeDepot { dossier_id: None, nom: nom.into(), taille: taille as i64, mime: None, commentaire: None, si_existe: None }
}

/// Dépôt complet : demande, « envoi direct » simulé dans le magasin, finalisation.
pub(crate) async fn deposer(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    role: &str,
    espace: Uuid,
    d: DemandeDepot,
    octets: &[u8],
    maintenant: DateTime<Utc>,
) -> Result<ResultatDepot, EntDbError> {
    let r = ent.espaces_demander_depot(etab, user, role, espace, d, maintenant).await?;
    let ReponseDepot::Autorise(a) = r else { panic!("nom déjà pris : {r:?}") };
    let cle = cle_reservee(ent, etab, a.reservation_id).await;
    ent.objects().put(&cle, octets).unwrap();
    ent.espaces_finaliser_depot(etab, user, role, a.reservation_id, maintenant).await
}

/// Dépôt complet puis analyse saine : l'élément est disponible.
pub(crate) async fn deposer_et_analyser(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    role: &str,
    espace: Uuid,
    d: DemandeDepot,
    octets: &[u8],
    maintenant: DateTime<Utc>,
) -> ResultatDepot {
    let r = deposer(ent, etab, user, role, espace, d, octets, maintenant).await.unwrap();
    let b = ent.espaces_analyser_versions(etab, sain(), 200, maintenant).await.unwrap();
    assert!(b.saines >= 1, "analyse : {b:?}");
    r
}

// ───────────────────────── Tests ─────────────────────────

/// ET0 : 20 envois parallèles ne dépassent pas le quota (réservation atomique, R-ET21, ND32-15).
#[tokio::test]
async fn et0_vingt_envois_paralleles_ne_depassent_pas_le_quota() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof) = (cast.etablissement_id, &cast.prof);
    let role = prof.role.as_str();
    let espace = espace_neuf(&ent, etab, prof.user_id, role).await;
    fixer_quota(&ent, etab, espace, (10 * MIO) as i64).await;
    let ent = Arc::new(ent);
    let mut taches = Vec::new();
    for i in 0..20 {
        let ent = ent.clone();
        let (user, role) = (prof.user_id, role.to_string());
        taches.push(tokio::spawn(async move {
            ent.espaces_demander_depot(etab, user, &role, espace, demande(&format!("lot-{i}.pdf"), MIO), t0()).await
        }));
    }
    let (mut ok, mut refus) = (0, 0);
    for t in taches {
        match t.await.unwrap() {
            Ok(ReponseDepot::Autorise(_)) => ok += 1,
            Err(EntDbError::Refus(RefusMetier::Conflit, m)) if m.contains("quota") => refus += 1,
            autre => panic!("réponse inattendue : {autre:?}"),
        }
    }
    assert_eq!((ok, refus), (10, 10), "exactement dix envois de 1 Mio tiennent dans 10 Mio");
    let (confirmes, reserves) = occupation(&ent, etab, espace).await;
    assert_eq!((confirmes, reserves), (0, (10 * MIO) as i64));
    // Le total réservé ne dépasse jamais le quota, et le compteur égale la somme des réservations ouvertes.
    let somme: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(octets), 0)::bigint FROM ent_espace_reservation
          WHERE etablissement_id = $1 AND espace_id = $2 AND confirmee_le IS NULL AND liberee_le IS NULL",
    )
    .bind(etab)
    .bind(espace)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(somme, reserves);
}

/// ET0 : un envoi de 70 Mio sous une URL signée pour 1 Mio est rejeté ; l'objet est effacé, le quota libéré, rien n'est
/// écrit en base (R-ET21, CET-19, ND32-01). Le plafond de 64 Mio est aussi refusé dès la demande.
#[tokio::test]
async fn et0_envoi_de_70_mio_signe_1_mio_est_refuse() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof) = (cast.etablissement_id, &cast.prof);
    let role = prof.role.as_str();
    let espace = espace_neuf(&ent, etab, prof.user_id, role).await;
    // Plafond : 70 Mio annoncés sont refusés dès la demande, sans réservation.
    let e = ent.espaces_demander_depot(etab, prof.user_id, role, espace, demande("gros.pdf", 70 * MIO), t0()).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(RefusMetier::Invalide, _)), "{e:?}");
    assert_eq!(occupation(&ent, etab, espace).await, (0, 0));
    // Envoi annoncé à 1 Mio, mais 70 Mio arrivent sous la clé réservée (un fournisseur qui n'imposerait pas la taille).
    let ReponseDepot::Autorise(a) =
        ent.espaces_demander_depot(etab, prof.user_id, role, espace, demande("annonce.pdf", MIO), t0()).await.unwrap()
    else {
        panic!("autorisé attendu")
    };
    assert_eq!(a.methode, "PUT");
    assert_eq!(a.en_tetes, vec![("Content-Length".to_string(), MIO.to_string())]);
    assert_eq!(occupation(&ent, etab, espace).await, (0, MIO as i64));
    let cle = cle_reservee(&ent, etab, a.reservation_id).await;
    ent.objects().put(&cle, &pdf(70 * MIO)).unwrap();
    let e = ent.espaces_finaliser_depot(etab, prof.user_id, role, a.reservation_id, t0()).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(RefusMetier::Conflit, _)), "{e:?}");
    assert!(!ent.objects().existe(&cle).unwrap(), "l'objet de taille différente est effacé");
    assert_eq!(occupation(&ent, etab, espace).await, (0, 0), "quota libéré");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_espace_element WHERE etablissement_id = $1 AND espace_id = $2")
        .bind(etab)
        .bind(espace)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 0, "aucun élément fantôme");
    let motif: String = sqlx::query_scalar("SELECT liberee_motif FROM ent_espace_reservation WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(a.reservation_id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(motif, "taille_differente");
    // Une réservation rejetée ne se finalise plus (ni ne se rejoue).
    assert!(ent.espaces_finaliser_depot(etab, prof.user_id, role, a.reservation_id, t0()).await.is_err());
    // Taille plus petite que celle signée : même rejet.
    let ReponseDepot::Autorise(b) =
        ent.espaces_demander_depot(etab, prof.user_id, role, espace, demande("petit.pdf", 2 * MIO), t0()).await.unwrap()
    else {
        panic!()
    };
    ent.objects().put(&cle_reservee(&ent, etab, b.reservation_id).await, &pdf(MIO)).unwrap();
    assert!(ent.espaces_finaliser_depot(etab, prof.user_id, role, b.reservation_id, t0()).await.is_err());
    assert_eq!(occupation(&ent, etab, espace).await, (0, 0));
}

/// ET0 : envoi jamais reçu, envoi coupé : le quota est libéré à l'expiration (15 min, CET-18), l'objet tardif est effacé,
/// et une finalisation tardive est refusée.
#[tokio::test]
async fn et0_reservation_expiree_liberee_et_finalisation_tardive_refusee() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof) = (cast.etablissement_id, &cast.prof);
    let role = prof.role.as_str();
    let espace = espace_neuf(&ent, etab, prof.user_id, role).await;
    let ReponseDepot::Autorise(a) =
        ent.espaces_demander_depot(etab, prof.user_id, role, espace, demande("coupe.pdf", 3 * MIO), t0()).await.unwrap()
    else {
        panic!()
    };
    assert_eq!(a.expire_le, t0() + Duration::minutes(15));
    assert_eq!(occupation(&ent, etab, espace).await, (0, (3 * MIO) as i64));
    // Rien n'est reçu : finaliser avant l'échéance dit « non reçu » sans libérer.
    let e = ent.espaces_finaliser_depot(etab, prof.user_id, role, a.reservation_id, t0() + Duration::minutes(5)).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(RefusMetier::Conflit, _)));
    assert_eq!(occupation(&ent, etab, espace).await, (0, (3 * MIO) as i64));
    // Un objet arrive après l'échéance, avant le balayage : la finalisation est refusée et l'objet effacé.
    let cle = cle_reservee(&ent, etab, a.reservation_id).await;
    ent.objects().put(&cle, &pdf(3 * MIO)).unwrap();
    let e = ent.espaces_finaliser_depot(etab, prof.user_id, role, a.reservation_id, t0() + Duration::minutes(16)).await.unwrap_err();
    assert!(matches!(e, EntDbError::Refus(RefusMetier::Conflit, _)), "{e:?}");
    assert!(!ent.objects().existe(&cle).unwrap());
    assert_eq!(occupation(&ent, etab, espace).await, (0, 0));
    // Balayage : une autre réservation expirée est libérée (et son objet orphelin effacé).
    let ReponseDepot::Autorise(b) =
        ent.espaces_demander_depot(etab, prof.user_id, role, espace, demande("orphelin.pdf", MIO), t0()).await.unwrap()
    else {
        panic!()
    };
    let cle_b = cle_reservee(&ent, etab, b.reservation_id).await;
    ent.objects().put(&cle_b, &pdf(MIO)).unwrap();
    assert_eq!(ent.espaces_liberer_reservations_expirees(etab, t0() + Duration::minutes(14)).await.unwrap(), 0);
    assert!(ent.espaces_liberer_reservations_expirees(etab, t0() + Duration::minutes(15)).await.unwrap() >= 1);
    assert!(!ent.objects().existe(&cle_b).unwrap());
    assert_eq!(occupation(&ent, etab, espace).await, (0, 0));
}

/// ET0 : un objet non analysé est invisible des autres membres, même de son auteur pour le téléchargement ; une analyse
/// indisponible le laisse invisible (fermé par défaut, CET-15) ; l'analyse saine le rend visible.
#[tokio::test]
async fn et0_objet_non_analyse_invisible_ferme_par_defaut() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof, dir) = (cast.etablissement_id, &cast.prof, &cast.direction);
    let (role, role_d) = (prof.role.as_str(), dir.role.as_str());
    let espace = espace_neuf(&ent, etab, prof.user_id, role).await;
    let r = deposer(&ent, etab, prof.user_id, role, espace, demande("rapport.pdf", 2 * MIO), &pdf(2 * MIO), t0()).await.unwrap();
    assert_eq!(r.etat, "analyse");
    // L'auteur voit « en cours d'analyse », sans pouvoir le télécharger.
    let c = ent.espaces_parcourir(etab, prof.user_id, role, espace, None, t0()).await.unwrap();
    assert_eq!(c.elements.len(), 1);
    assert_eq!(c.elements[0].etat, "analyse");
    assert!(matches!(
        ent.espaces_lien_telechargement(etab, prof.user_id, role, r.element_id, None, t0()).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Un collègue qui reçoit un partage plus tard ne peut pas partager un fichier non analysé (testé en ET1) ; ici, un tiers
    // qui n'est pas membre ne voit rien : l'espace est introuvable pour lui.
    collegue(&ent, etab, dir.user_id).await;
    assert!(matches!(
        ent.espaces_parcourir(etab, dir.user_id, role_d, espace, None, t0()).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Antivirus indisponible : le fichier reste invisible, une tentative est comptée.
    let indispo: Arc<dyn AnalyseurAntivirus> = Arc::new(Verdict(VerdictAnalyse::Indisponible));
    let b = ent.espaces_analyser_versions(etab, indispo, 50, t0()).await.unwrap();
    assert!(b.indisponibles >= 1 && b.saines == 0, "{b:?}");
    let (etat, tentatives): (String, i32) = sqlx::query_as(
        "SELECT etat, analyse_tentatives FROM ent_espace_version WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(r.version_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((etat.as_str(), tentatives), ("analyse", 1));
    assert!(matches!(
        ent.espaces_lien_telechargement(etab, prof.user_id, role, r.element_id, None, t0()).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Verdict sain : disponible, téléchargeable, événement `espaces.element_depose` émis après l'analyse.
    let b = ent.espaces_analyser_versions(etab, sain(), 50, t0()).await.unwrap();
    assert!(b.saines >= 1);
    let lien = ent.espaces_lien_telechargement(etab, prof.user_id, role, r.element_id, None, t0()).await.unwrap();
    assert!(lien.url.starts_with("/fichiers/"));
    assert_eq!(lien.expire_le, t0() + Duration::seconds(60));
    let evt: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = 'espaces.element_depose' AND aggregate_id = $2")
        .bind(etab)
        .bind(r.element_id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(evt, 1);
}

/// ET0 : un fichier infecté passe en quarantaine : invisible, quota rendu, objet effacé, auteur prévenu sans nom de fichier
/// (CET-14, ND32-05).
#[tokio::test]
async fn et0_fichier_infecte_en_quarantaine_quota_rendu() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof) = (cast.etablissement_id, &cast.prof);
    let role = prof.role.as_str();
    let espace = espace_neuf(&ent, etab, prof.user_id, role).await;
    let r = deposer(&ent, etab, prof.user_id, role, espace, demande("virus-secret.pdf", 2 * MIO), &pdf_unique(2 * MIO), t0()).await.unwrap();
    assert_eq!(occupation(&ent, etab, espace).await.0, (2 * MIO) as i64);
    let cle: String = sqlx::query_scalar(
        "SELECT o.cle_objet FROM ent_espace_version v JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
          WHERE v.etablissement_id = $1 AND v.id = $2",
    )
    .bind(etab)
    .bind(r.version_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let infecte: Arc<dyn AnalyseurAntivirus> = Arc::new(Verdict(VerdictAnalyse::Infecte("Eicar-Test-Signature".into())));
    let b = ent.espaces_analyser_versions(etab, infecte, 50, t0()).await.unwrap();
    assert!(b.infectees >= 1, "{b:?}");
    let (e_etat, v_etat): (String, String) = sqlx::query_as(
        "SELECT e.etat, v.etat FROM ent_espace_element e JOIN ent_espace_version v ON v.etablissement_id = e.etablissement_id AND v.element_id = e.id
          WHERE e.etablissement_id = $1 AND e.id = $2",
    )
    .bind(etab)
    .bind(r.element_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((e_etat.as_str(), v_etat.as_str()), ("quarantaine", "quarantaine"));
    assert!(!ent.objects().existe(&cle).unwrap(), "objet infecté effacé");
    assert_eq!(occupation(&ent, etab, espace).await, (0, 0), "quota rendu");
    assert!(ent.espaces_lien_telechargement(etab, prof.user_id, role, r.element_id, None, t0()).await.is_err());
    // Le nom de fichier ne figure dans aucun avis ni événement (E14).
    let fuite: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND (titre ILIKE '%secret%' OR corps ILIKE '%secret%'))
              + (SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND payload::text ILIKE '%secret%')",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(fuite, 0);
    let avis: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND kind = 'espaces.analyse' AND destinataire_user_id = $2")
        .bind(etab)
        .bind(prof.user_id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(avis, 1, "l'auteur est prévenu");
}

/// ET0 : formes refusées à la finalisation (HTML, macro, type déclaré incohérent) : objet effacé, quota libéré (ND32-05).
#[tokio::test]
async fn et0_contenu_actif_ou_incoherent_refuse_a_la_finalisation() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof) = (cast.etablissement_id, &cast.prof);
    let role = prof.role.as_str();
    let espace = espace_neuf(&ent, etab, prof.user_id, role).await;
    let html = b"<html><script>alert(1)</script></html>".to_vec();
    let e = deposer(&ent, etab, prof.user_id, role, espace, demande("page.pdf", html.len()), &html, t0()).await.unwrap_err();
    assert!(matches!(e, EntDbError::Fichier(_)), "{e:?}");
    assert_eq!(occupation(&ent, etab, espace).await, (0, 0));
    // Nom à extension active : refusé dès la demande.
    let e = ent.espaces_demander_depot(etab, prof.user_id, role, espace, demande("setup.exe", 10), t0()).await.unwrap_err();
    assert!(matches!(e, EntDbError::Fichier(_)), "{e:?}");
    // Type annoncé incohérent avec le contenu réel (un PDF annoncé « image/png »).
    let mut d = demande("faux.pdf", 4096);
    d.mime = Some("image/png".into());
    let e = deposer(&ent, etab, prof.user_id, role, espace, d, &pdf(4096), t0()).await.unwrap_err();
    assert!(matches!(e, EntDbError::Fichier(_)), "{e:?}");
    assert_eq!(occupation(&ent, etab, espace).await, (0, 0));
    // Aucun objet orphelin : toutes les clés de réservation sont absentes du magasin.
    let cles: Vec<String> = sqlx::query_scalar("SELECT cle_objet FROM ent_espace_reservation WHERE etablissement_id = $1 AND espace_id = $2")
        .bind(etab)
        .bind(espace)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap();
    assert!(!cles.is_empty() && cles.iter().all(|c| !ent.objects().existe(c).unwrap()));
}

/// ET0 : une taille négative, nulle ou absurde est refusée côté serveur (ND32-01) ; la clé d'objet est opaque
/// (ni nom de fichier ni donnée personnelle, ND32-10, ND32-18).
#[tokio::test]
async fn et0_taille_invalide_refusee_et_cle_opaque() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof) = (cast.etablissement_id, &cast.prof);
    let role = prof.role.as_str();
    let espace = espace_neuf(&ent, etab, prof.user_id, role).await;
    for taille in [-1i64, 0, i64::MAX] {
        let mut d = demande("a.pdf", 1);
        d.taille = taille;
        assert!(matches!(
            ent.espaces_demander_depot(etab, prof.user_id, role, espace, d, t0()).await,
            Err(EntDbError::Refus(RefusMetier::Invalide, _))
        ));
    }
    let ReponseDepot::Autorise(a) =
        ent.espaces_demander_depot(etab, prof.user_id, role, espace, demande("Bulletin Léa Martin.pdf", 1024), t0()).await.unwrap()
    else {
        panic!()
    };
    let cle = cle_reservee(&ent, etab, a.reservation_id).await;
    assert!(cle.starts_with(&format!("{etab}/espaces/")), "{cle}");
    assert!(!cle.to_lowercase().contains("bulletin") && !cle.contains("Martin"), "{cle}");
    assert!(!a.url.contains(&cle), "la clé n'est pas dans l'URL d'envoi");
    // Le jeton d'envoi est lié au compte et à la taille.
    let jeton = a.url.trim_start_matches("/espaces/envoi/");
    let c = ent.presigner().verify_upload_local(jeton, t0().timestamp()).unwrap();
    assert_eq!((c.user_id, c.taille, c.etablissement_id), (prof.user_id, 1024, etab));
}

/// ET0 : la réconciliation recalcule les compteurs depuis les lignes de vérité (après restauration ou incident).
#[tokio::test]
async fn et0_reconciliation_recalcule_l_occupation() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let (etab, prof) = (cast.etablissement_id, &cast.prof);
    let role = prof.role.as_str();
    let espace = espace_neuf(&ent, etab, prof.user_id, role).await;
    deposer_et_analyser(&ent, etab, prof.user_id, role, espace, demande("a.pdf", 2 * MIO), &pdf_unique(2 * MIO), t0()).await;
    deposer_et_analyser(&ent, etab, prof.user_id, role, espace, demande("b.pdf", MIO), &pdf_unique(MIO), t0()).await;
    let ReponseDepot::Autorise(_) =
        ent.espaces_demander_depot(etab, prof.user_id, role, espace, demande("c.pdf", 4 * MIO), t0()).await.unwrap()
    else {
        panic!()
    };
    assert_eq!(occupation(&ent, etab, espace).await, ((3 * MIO) as i64, (4 * MIO) as i64));
    // Compteurs faussés (restauration partielle, incident).
    sqlx::query("UPDATE ent_espace_occupation SET octets_confirmes = 7, octets_reserves = 9 WHERE etablissement_id = $1 AND espace_id = $2")
        .bind(etab)
        .bind(espace)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert!(ent.espaces_reconcilier_occupation(etab, t0()).await.unwrap() >= 1);
    assert_eq!(occupation(&ent, etab, espace).await, ((3 * MIO) as i64, (4 * MIO) as i64));
}

/// ET0 : sans droit `espaces.utiliser` (parent, élève), aucune commande ne répond ; un compte sans fiche n'a pas d'espace.
#[tokio::test]
async fn et0_sans_droit_aucune_commande() {
    let Some((ent, cast)) = labo_espaces().await else { return };
    let etab = cast.etablissement_id;
    let (parent, role) = (&cast.parent_lea, "parent");
    assert!(matches!(ent.espaces_lister(etab, parent.user_id, role, t0()).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.espaces_ouvrir_mon_espace(etab, parent.user_id, role, t0()).await, Err(EntDbError::Forbidden)));
    assert!(matches!(
        ent.espaces_demander_depot(etab, parent.user_id, role, Uuid::new_v4(), demande("a.pdf", 10), t0()).await,
        Err(EntDbError::Forbidden)
    ));
    let _ = Role::Parent;
}
