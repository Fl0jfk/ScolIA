//! Socle 7 (documents), lot D5 : pièces justificatives, contre la base locale de test (feature `labo`) ; sans
//! `ENT_DATABASE_URL`, les tests sont ignorés en local (échec explicite en CI ou avec `ENT_EXIGER_BASE`). Chaque test crée
//! son propre établissement. En mode strict (`ENT_RLS_MODE=strict`) le service tourne sous un rôle sans privilège
//! (`NOBYPASSRLS`) : tous ces tests prouvent aussi le cloisonnement par la base.
//!
//! Critères de fin du lot (§23, ligne D5) :
//!
//! | Exigence | Test(s) |
//! |---|---|
//! | fichier de test EICAR refusé et jamais servi | `t_eicar_refuse_et_jamais_servi` |
//! | antivirus arrêté → quarantaine maintenue | `t_antivirus_indisponible_quarantaine_maintenue` |
//! | PDF avec JavaScript désarmé | `t_pdf_javascript_desarme`, `t_pdf_non_inspectable_rejete` |
//! | pièce de foyer pour deux enfants | `t_piece_de_foyer_pour_deux_enfants` |
//! | EXIF (position GPS) retiré, photos assemblées en un PDF | `t_photos_en_un_pdf_sans_exif` |
//! | contrôle en rafale, un seul contrôle par pièce | `t_controle_en_rafale`, `t_controle_concurrent_une_seule_decision` |
//! | réutilisation d'une pièce déjà fournie, jamais en silence | `t_reutilisation_confirmee` |
//! | « ce qui manque » | `t_ce_qui_manque` |
//! | identité : vue seulement, pas de copie | `t_identite_sans_copie` |
//! | dépôts refusés (forme, droits, lien suspendu, mineur, débit) | `t_depots_refuses_a_l_entree` |
//! | retrait par le déposant | `t_retrait_par_le_deposant` |
//! | port de reprise N1-5 | `t_port_reprise_n1_5` |
//! | séparation entre établissements | `t_separation_entre_etablissements` |

use super::*;
use crate::demandes_reprise::{PieceSource, PortPiecesReprise};
use crate::documents_pieces::*;
use crate::ref_commun::Acteur;
use crate::service::{EntDbError, RefusMetier};
use crate::tests_demandes::{fiche, labo, monde, Acteur as ActeurTest, Monde};
use chrono::NaiveDate;
use ent_object::upload::{AnalyseurAntivirus, ContexteAnalyse, VerdictAnalyse};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

// ───────────────────────── Doublures et fichiers de test ─────────────────────────

const EICAR: &[u8] = b"X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*";

/// Antivirus de test : reconnaît EICAR comme le vrai, et peut être « arrêté ».
struct Av {
    arrete: AtomicBool,
}

impl Av {
    fn sain() -> Arc<Av> {
        Arc::new(Av { arrete: AtomicBool::new(false) })
    }
}

impl AnalyseurAntivirus for Av {
    fn nom(&self) -> &'static str {
        "test"
    }
    fn analyser(&self, ctx: &ContexteAnalyse<'_>) -> VerdictAnalyse {
        if self.arrete.load(Ordering::SeqCst) {
            return VerdictAnalyse::Indisponible;
        }
        if ctx.octets.windows(EICAR.len()).any(|w| w == EICAR) {
            return VerdictAnalyse::Infecte("Eicar-Test-Signature".into());
        }
        VerdictAnalyse::Sain
    }
}

fn segment(m: u8, corps: &[u8]) -> Vec<u8> {
    let mut v = vec![0xff, m];
    v.extend_from_slice(&((corps.len() + 2) as u16).to_be_bytes());
    v.extend_from_slice(corps);
    v
}

/// Un JPEG minimal, avec un EXIF qui porte une « position GPS » et l'orientation 6.
fn photo(secret: &str) -> Vec<u8> {
    let mut t = b"II*\0".to_vec();
    t.extend_from_slice(&8u32.to_le_bytes());
    t.extend_from_slice(&1u16.to_le_bytes());
    t.extend_from_slice(&0x0112u16.to_le_bytes());
    t.extend_from_slice(&3u16.to_le_bytes());
    t.extend_from_slice(&1u32.to_le_bytes());
    t.extend_from_slice(&6u16.to_le_bytes());
    t.extend_from_slice(&[0, 0]);
    t.extend_from_slice(&0u32.to_le_bytes());
    t.extend_from_slice(format!("GPS={secret}").as_bytes());
    let mut exif = b"Exif\0\0".to_vec();
    exif.extend_from_slice(&t);
    let mut v = vec![0xff, 0xd8];
    v.extend_from_slice(&segment(0xe1, &exif));
    v.extend_from_slice(&segment(0xdb, &[0u8; 65]));
    v.extend_from_slice(&segment(0xc0, &[8, 0, 30, 0, 40, 3, 1, 0x11, 0, 2, 0x11, 0, 3, 0x11, 0]));
    v.extend_from_slice(&segment(0xc4, &[0u8; 20]));
    v.extend_from_slice(&segment(0xda, &[3, 1, 0, 2, 0, 3, 0, 0, 63, 0]));
    v.extend_from_slice(&[0x12, 0x34, 0xff, 0x00, 0x56, 0xff, 0xd9]);
    v
}

fn pdf(extra_catalogue: &str, extra_objets: &str) -> Vec<u8> {
    format!(
        "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R {extra_catalogue} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>\nendobj\n{extra_objets}\
         trailer\n<< /Root 1 0 R >>\nstartxref\n0\n%%EOF\n"
    )
    .into_bytes()
}

fn pdf_simple() -> Vec<u8> {
    pdf("", "")
}

// ───────────────────────── Le monde de test ─────────────────────────

struct W {
    ent: PgEnt,
    cast: LaboCast,
    m: Monde,
    lily: ActeurTest,
    tom: ActeurTest,
    jour: NaiveDate,
    av: Arc<Av>,
}

impl W {
    async fn nouveau() -> Option<W> {
        let (ent, cast) = labo().await?;
        let m = monde(&ent, &cast).await;
        let lily = fiche(&ent, m.etab, cast.parent_lea.user_id, "parent", false).await;
        let tom = fiche(&ent, m.etab, cast.parent_tom.user_id, "parent", false).await;
        let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1)")
            .bind(m.etab)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
        Some(W { ent, cast, m, lily, tom, jour, av: Av::sain() })
    }

    fn acteur(&self, user: Uuid, role: &'static str) -> Acteur<'static> {
        Acteur { etablissement_id: self.m.etab, user_id: user, role }
    }
    fn dir(&self) -> Acteur<'static> {
        self.acteur(self.cast.direction.user_id, "direction")
    }
    fn adm(&self) -> Acteur<'static> {
        self.acteur(self.cast.administratif.user_id, "administratif")
    }
    fn prof(&self) -> Acteur<'static> {
        self.acteur(self.cast.prof.user_id, "prof")
    }
    fn accueil(&self) -> Acteur<'static> {
        // Même compte de test, profil « accueil » : le modèle de droits par défaut est celui du profil.
        self.acteur(self.cast.cpe.user_id, "accueil")
    }
    fn parent(&self, p: &ActeurTest) -> Acteur<'static> {
        self.acteur(p.user, "parent")
    }
    fn lily_a(&self) -> Acteur<'static> {
        self.parent(&self.lily)
    }
    fn tom_a(&self) -> Acteur<'static> {
        self.parent(&self.tom)
    }

    async fn eleve_ne_le(&self, nom: &str, prenom: &str, naissance: NaiveDate) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, classe, date_naissance, date_entree, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'actif', '4B', $5, $6, now(), now())",
        )
        .bind(id)
        .bind(self.m.etab)
        .bind(nom)
        .bind(prenom)
        .bind(naissance)
        .bind(self.jour - chrono::Duration::days(60))
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
        id
    }

    async fn eleve(&self, nom: &str) -> Uuid {
        self.eleve_ne_le(nom, "Test", NaiveDate::from_ymd_opt(2012, 3, 10).unwrap()).await
    }

    async fn lier(&self, eleve: Uuid, p: &ActeurTest, autorite: bool, foyer: Option<Uuid>) {
        sqlx::query(
            "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale, foyer_id)
             VALUES ($1, $2, $3, $4, $5, 'parent', $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(self.m.etab)
        .bind(eleve)
        .bind(p.user)
        .bind(p.personne)
        .bind(autorite)
        .bind(foyer)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
    }

    async fn foyer(&self) -> Uuid {
        let f = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_foyer (id, etablissement_id, libelle) VALUES ($1, $2, 'Foyer de test')")
            .bind(f)
            .bind(self.m.etab)
            .execute(self.ent.pool_verif())
            .await
            .unwrap();
        f
    }

    fn demande(&self, type_code: &str, sujet: SujetPiece) -> DemandePiece {
        DemandePiece {
            type_code: type_code.into(),
            sujet,
            motif_affiche: "Dossier d'inscription".into(),
            echeance: None,
            obligatoire: true,
            brique: "inscriptions".into(),
            reference_externe: None,
        }
    }

    fn depot(&self, type_code: &str, sujet: SujetPiece, demande: Option<Uuid>, fichiers: Vec<Vec<u8>>) -> DepotPiece {
        DepotPiece { type_code: type_code.into(), sujet, demande_id: demande, motif_autre: None, fichiers }
    }

    /// Dépose puis analyse (antivirus sain) : la pièce est « reçue ».
    async fn deposer_et_analyser(&self, a: &Acteur<'_>, d: DepotPiece) -> Uuid {
        let p = self.ent.deposer_piece_for_actor(a, d).await.expect("dépôt");
        assert_eq!(p.etat, "en_quarantaine");
        let r = self.ent.analyser_piece(self.m.etab, p.piece_id, self.av.clone()).await.unwrap();
        assert_eq!(r, ResultatAnalyse::Recue);
        p.piece_id
    }

    async fn etat(&self, piece: Uuid) -> String {
        let (e,): (String,) = sqlx::query_as("SELECT etat FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2")
            .bind(self.m.etab)
            .bind(piece)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        e
    }

    async fn nb_pieces(&self) -> i64 {
        let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_doc_piece WHERE etablissement_id = $1")
            .bind(self.m.etab)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        n
    }

    async fn octets_servis(&self, piece: Uuid) -> Option<String> {
        let (c,): (Option<String>,) =
            sqlx::query_as("SELECT cle_objet_servi FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2")
                .bind(self.m.etab)
                .bind(piece)
                .fetch_one(self.ent.pool_verif())
                .await
                .unwrap();
        c
    }
}

fn introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound))
}

fn est_refus(e: &EntDbError, attendu: RefusMetier) -> bool {
    matches!(e, EntDbError::Refus(r, _) if *r == attendu)
}

fn contient(o: &[u8], s: &[u8]) -> bool {
    o.windows(s.len()).any(|w| w == s)
}

// ───────────────────────── Demander ─────────────────────────

#[tokio::test]
async fn t_demander_et_droits() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Durand").await;
    let s = SujetPiece::Eleve(eleve);
    let c = w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", s)).await.unwrap();
    assert!(!c.deja_ouverte && c.proposition_piece_id.is_none());
    // Idempotent : une demande ouverte existante est rendue telle quelle.
    let c2 = w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", s)).await.unwrap();
    assert!(c2.deja_ouverte);
    assert_eq!(c2.demande_id, c.demande_id);
    // Droits : le professeur ne demande rien ; un parent non plus.
    assert!(matches!(
        w.ent.demander_piece_for_actor(&w.prof(), w.demande("acte_naissance", s)).await.unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(matches!(
        w.ent.demander_piece_for_actor(&w.lily_a(), w.demande("acte_naissance", s)).await.unwrap_err(),
        EntDbError::Forbidden
    ));
    // Une pièce de justice (C3) exige le droit du domaine : le secrétariat ne l'a pas par défaut, la direction oui.
    assert!(matches!(
        w.ent.demander_piece_for_actor(&w.adm(), w.demande("decision_justice", s)).await.unwrap_err(),
        EntDbError::Forbidden
    ));
    w.ent.demander_piece_for_actor(&w.dir(), w.demande("decision_justice", s)).await.unwrap();
    // Catalogue fermé, sujet cohérent avec la portée, type technique non demandable.
    for (t, sj) in [
        ("titre_sejour", s),
        ("justificatif_domicile", SujetPiece::Personne(w.lily.personne)),
        ("piece_identite", s),
        ("piece_de_demande", SujetPiece::Personne(w.lily.personne)),
    ] {
        assert!(w.ent.demander_piece_for_actor(&w.adm(), w.demande(t, sj)).await.is_err(), "{t}");
    }
    // « Autre » : motif exigé et jamais pour une pièce interdite (§11.2).
    let mut interdit = w.demande("autre", s);
    interdit.motif_affiche = "Copie du titre de séjour".into();
    assert!(est_refus(&w.ent.demander_piece_for_actor(&w.adm(), interdit).await.unwrap_err(), RefusMetier::Conflit));
    let mut ok = w.demande("autre", s);
    ok.motif_affiche = "Autorisation de sortie exceptionnelle".into();
    w.ent.demander_piece_for_actor(&w.adm(), ok.clone()).await.unwrap();
    let encore = w.ent.demander_piece_for_actor(&w.adm(), ok).await.unwrap();
    assert!(!encore.deja_ouverte, "« autre » n'est jamais fusionnée : le motif change à chaque demande");
    // Annuler : motif obligatoire.
    assert!(w.ent.annuler_demande_piece_for_actor(&w.adm(), c.demande_id, " ").await.is_err());
    w.ent.annuler_demande_piece_for_actor(&w.adm(), c.demande_id, "Pièce déjà reçue au guichet").await.unwrap();
    assert!(est_refus(
        &w.ent.annuler_demande_piece_for_actor(&w.adm(), c.demande_id, "Encore").await.unwrap_err(),
        RefusMetier::Conflit
    ));
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'documents.piece_demandee'",
    )
    .bind(w.m.etab)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 4, "justificatif, justice, deux « autre »");
}

// ───────────────────────── Déposer, quarantaine, analyse ─────────────────────────

#[tokio::test]
async fn t_photos_en_un_pdf_sans_exif() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Garcia").await;
    w.lier(eleve, &w.lily, true, None).await;
    let s = SujetPiece::Eleve(eleve);
    let dem = w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", s)).await.unwrap();
    let originales = vec![photo("48.8566N-2.3522E-SECRET-1"), photo("48.8566N-2.3522E-SECRET-2")];
    let dep = w
        .ent
        .deposer_piece_for_actor(&w.lily_a(), w.depot("justificatif_domicile", s, Some(dem.demande_id), originales.clone()))
        .await
        .unwrap();
    // Quarantaine : rien n'est servi, pour personne, ni la famille ni le secrétariat ni la direction.
    assert_eq!(dep.etat, "en_quarantaine");
    assert!(w.octets_servis(dep.piece_id).await.is_none());
    for qui in [w.lily_a(), w.adm(), w.dir()] {
        assert!(introuvable(&w.ent.lire_piece_justificative_for_actor(&qui, dep.piece_id).await.unwrap_err()), "{}", qui.role);
    }
    let manque = w.ent.ce_qui_manque_for_actor(&w.lily_a(), s).await.unwrap();
    assert_eq!(manque[0].etat, EtatManque::EnAnalyse);
    // Analyse en tâche de fond : photos assemblées en un seul PDF de deux pages.
    let r = w.ent.analyser_pieces_en_attente(w.m.etab, w.av.clone(), 10).await.unwrap();
    assert_eq!(r, RapportAnalyse { recues: 1, ..Default::default() });
    assert_eq!(w.etat(dep.piece_id).await, "recue");
    let manque = w.ent.ce_qui_manque_for_actor(&w.lily_a(), s).await.unwrap();
    assert_eq!(manque[0].etat, EtatManque::RecueEnAttente);
    let lue = w.ent.lire_piece_justificative_for_actor(&w.lily_a(), dep.piece_id).await.unwrap();
    assert!(lue.octets.starts_with(b"%PDF-1.4"));
    assert_eq!(lue.nb_pages, 2);
    // EXIF (dont la position GPS) retiré : ni dans le PDF servi, ni nulle part ailleurs de lisible ; l'original (scellé) les garde.
    for interdit in [&b"GPS"[..], b"SECRET", b"Exif"] {
        assert!(!contient(&lue.octets, interdit), "{}", String::from_utf8_lossy(interdit));
    }
    assert!(contient(&originales[0], b"SECRET-1"));
    assert_eq!(lue.empreinte_sha256, ent_object::sha256_hex(&lue.octets));
    // L'original n'est jamais servi : l'objet servi n'est pas celui de l'original.
    let (orig, servi): (String, String) = sqlx::query_as(
        "SELECT cle_objet_original, cle_objet_servi FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.m.etab)
    .bind(dep.piece_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_ne!(orig, servi);
    assert!(orig.contains("/pieces-quarantaine/") && servi.contains("/pieces/"));
    // Le secrétariat lit à son tour ; la lecture est tracée au journal ; le professeur ne lit pas.
    assert!(w.ent.lire_piece_justificative_for_actor(&w.adm(), dep.piece_id).await.is_ok());
    assert!(introuvable(&w.ent.lire_piece_justificative_for_actor(&w.prof(), dep.piece_id).await.unwrap_err()));
    assert!(introuvable(&w.ent.lire_piece_justificative_for_actor(&w.tom_a(), dep.piece_id).await.unwrap_err()), "un autre parent");
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_journal WHERE etablissement_id = $1 AND action = 'piece_consultee'",
    )
    .bind(w.m.etab)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 2);
    // Événement `documents.depose` publié APRÈS l'analyse (jamais au dépôt).
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'documents.depose' AND aggregate_id = $2",
    )
    .bind(w.m.etab)
    .bind(dep.piece_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn t_eicar_refuse_et_jamais_servi() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Leroy").await;
    w.lier(eleve, &w.lily, true, None).await;
    let s = SujetPiece::Eleve(eleve);
    w.ent.demander_piece_for_actor(&w.adm(), w.demande("acte_naissance", s)).await.unwrap();
    // Un PDF d'apparence banale qui porte la chaîne de test EICAR.
    let piege = pdf("", &format!("4 0 obj\n<< /Length 70 >>\nstream\n{}\nendstream\nendobj\n", String::from_utf8_lossy(EICAR)));
    let dep = w.ent.deposer_piece_for_actor(&w.lily_a(), w.depot("acte_naissance", s, None, vec![piege])).await.unwrap();
    let r = w.ent.analyser_piece(w.m.etab, dep.piece_id, w.av.clone()).await.unwrap();
    assert_eq!(r, ResultatAnalyse::Infectee);
    assert_eq!(w.etat(dep.piece_id).await, "infectee");
    assert!(w.octets_servis(dep.piece_id).await.is_none(), "aucun fichier servi");
    for qui in [w.lily_a(), w.adm(), w.dir()] {
        assert!(introuvable(&w.ent.lire_piece_justificative_for_actor(&qui, dep.piece_id).await.unwrap_err()));
    }
    // Ni acceptée, ni contrôlée : elle n'est pas à contrôler.
    let e = w
        .ent
        .controler_piece_for_actor(&w.adm(), dep.piece_id, ControleDecision::Accepter { valable_jusqu_au: None })
        .await
        .unwrap_err();
    assert!(est_refus(&e, RefusMetier::Conflit), "{e:?}");
    assert!(w.ent.pieces_a_controler_for_actor(&w.adm(), 50).await.unwrap().is_empty());
    // Ré-analyser ne change rien.
    assert_eq!(w.ent.analyser_piece(w.m.etab, dep.piece_id, w.av.clone()).await.unwrap(), ResultatAnalyse::DejaTraitee);
    // La base elle-même refuse de la sortir de quarantaine ou de lui donner un fichier servi (propriétaire compris).
    let pool = w.ent.pool_verif();
    let (orig,): (String,) =
        sqlx::query_as("SELECT cle_objet_original FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2")
            .bind(w.m.etab)
            .bind(dep.piece_id)
            .fetch_one(pool)
            .await
            .unwrap();
    for sql in [
        "UPDATE ent_doc_piece SET etat = 'recue' WHERE etablissement_id = $1 AND id = $2",
        "UPDATE ent_doc_piece SET etat = 'acceptee' WHERE etablissement_id = $1 AND id = $2",
    ] {
        assert!(sqlx::query(sql).bind(w.m.etab).bind(dep.piece_id).execute(pool).await.is_err(), "{sql}");
    }
    assert!(
        sqlx::query("UPDATE ent_doc_piece SET cle_objet_servi = $3, sha256_servi = repeat('a', 64), nb_pages = 1 WHERE etablissement_id = $1 AND id = $2")
            .bind(w.m.etab)
            .bind(dep.piece_id)
            .bind(&orig)
            .execute(pool)
            .await
            .is_err(),
        "le fichier servi ne peut pas être posé sur une pièce infectée"
    );
    assert!(sqlx::query("DELETE FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2")
        .bind(w.m.etab)
        .bind(dep.piece_id)
        .execute(pool)
        .await
        .is_err());
    // La famille voit un fichier refusé à l'analyse (sans le détail de la menace) et la demande reste à fournir.
    let manque = w.ent.ce_qui_manque_for_actor(&w.lily_a(), s).await.unwrap();
    assert_eq!((manque[0].etat, manque[0].refus_motif.as_deref()), (EtatManque::AFournir, Some("analyse")));
    // L'événement d'exploitation est publié, sans nom de menace.
    let (p,): (serde_json::Value,) = sqlx::query_as(
        "SELECT payload FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'documents.refuse_analyse' AND aggregate_id = $2",
    )
    .bind(w.m.etab)
    .bind(dep.piece_id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(p["motif"], "infectee");
    assert!(!p.to_string().contains("Eicar"));
}

#[tokio::test]
async fn t_antivirus_indisponible_quarantaine_maintenue() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Roche").await;
    w.lier(eleve, &w.lily, true, None).await;
    let s = SujetPiece::Eleve(eleve);
    let dep = w.ent.deposer_piece_for_actor(&w.lily_a(), w.depot("acte_naissance", s, None, vec![pdf_simple()])).await.unwrap();
    // Antivirus arrêté : la pièce reste en quarantaine, on réessaiera ; une seule ligne d'historique.
    w.av.arrete.store(true, Ordering::SeqCst);
    for _ in 0..3 {
        assert_eq!(w.ent.analyser_piece(w.m.etab, dep.piece_id, w.av.clone()).await.unwrap(), ResultatAnalyse::EnQuarantaine);
    }
    assert_eq!(w.etat(dep.piece_id).await, "en_quarantaine");
    assert!(w.octets_servis(dep.piece_id).await.is_none());
    assert!(introuvable(&w.ent.lire_piece_justificative_for_actor(&w.lily_a(), dep.piece_id).await.unwrap_err()));
    let (t, n): (i32, i64) = sqlx::query_as(
        "SELECT p.analyse_tentatives, (SELECT count(*) FROM ent_doc_piece_decision d WHERE d.piece_id = p.id AND d.decision = 'analyse_indisponible')
           FROM ent_doc_piece p WHERE p.etablissement_id = $1 AND p.id = $2",
    )
    .bind(w.m.etab)
    .bind(dep.piece_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((t, n), (3, 1));
    // Le vrai client ClamAV, démon injoignable : même résultat (fermé par défaut, jamais « sain »).
    let adr = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().to_string()
    };
    let clam: Arc<dyn AnalyseurAntivirus> = Arc::new(
        ent_object::clamav::ClamAv::nouveau(adr)
            .avec_delais(std::time::Duration::from_millis(300), std::time::Duration::from_millis(300)),
    );
    assert_eq!(w.ent.analyser_piece(w.m.etab, dep.piece_id, clam).await.unwrap(), ResultatAnalyse::EnQuarantaine);
    assert_eq!(w.etat(dep.piece_id).await, "en_quarantaine");
    // Le démon revient : la pièce est analysée, convertie, reçue.
    w.av.arrete.store(false, Ordering::SeqCst);
    assert_eq!(w.ent.analyser_piece(w.m.etab, dep.piece_id, w.av.clone()).await.unwrap(), ResultatAnalyse::Recue);
    assert!(w.ent.lire_piece_justificative_for_actor(&w.lily_a(), dep.piece_id).await.is_ok());
}

#[tokio::test]
async fn t_pdf_javascript_desarme() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Morin").await;
    w.lier(eleve, &w.lily, true, None).await;
    let s = SujetPiece::Eleve(eleve);
    let piege = pdf(
        "/OpenAction 5 0 R /Names << /JavaScript 6 0 R >>",
        "5 0 obj\n<< /S /JavaScript /JS (app.alert\\('piege'\\);) >>\nendobj\n6 0 obj\n<< /S /#4Aava#53cript /J#53 (x) >>\nendobj\n",
    );
    let id = w.deposer_et_analyser(&w.lily_a(), w.depot("acte_naissance", s, None, vec![piege.clone()])).await;
    let lue = w.ent.lire_piece_justificative_for_actor(&w.lily_a(), id).await.unwrap();
    assert!(lue.octets.starts_with(b"%PDF-1.4") && lue.nb_pages == 1);
    for interdit in [&b"/JavaScript"[..], b"/JS", b"/OpenAction", b"#4Aava"] {
        assert!(!contient(&lue.octets, interdit), "{} subsiste", String::from_utf8_lossy(interdit));
    }
    assert!(contient(&piege, b"/JavaScript"), "l'original les contenait");
    assert_eq!(lue.octets.len(), piege.len(), "neutralisé sur place : mêmes longueurs, xref exacte");
    // Le contrôleur est prévenu de ce qui a été neutralisé.
    let a_controler = w.ent.pieces_a_controler_for_actor(&w.adm(), 10).await.unwrap();
    let d = a_controler[0].desarmement.clone().unwrap();
    assert!(d.contains("JavaScript") && d.contains("OpenAction"), "{d}");
}

#[tokio::test]
async fn t_pdf_non_inspectable_rejete() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Perrin").await;
    w.lier(eleve, &w.lily, true, None).await;
    let s = SujetPiece::Eleve(eleve);
    let objstm = pdf("", "4 0 obj\n<< /Type /ObjStm /N 1 /First 4 /Length 4 >>\nstream\nabcd\nendstream\nendobj\n");
    let chiffre = pdf("/Encrypt 9 0 R", "");
    for (fichier, attendu) in [(objstm, "compressés"), (chiffre, "chiffré")] {
        let dep = w.ent.deposer_piece_for_actor(&w.lily_a(), w.depot("acte_naissance", s, None, vec![fichier])).await.unwrap();
        let r = w.ent.analyser_piece(w.m.etab, dep.piece_id, w.av.clone()).await.unwrap();
        assert!(matches!(&r, ResultatAnalyse::Rejetee(m) if m.contains(attendu)), "{r:?}");
        assert_eq!(w.etat(dep.piece_id).await, "rejetee");
        assert!(w.octets_servis(dep.piece_id).await.is_none());
        assert!(introuvable(&w.ent.lire_piece_justificative_for_actor(&w.dir(), dep.piece_id).await.unwrap_err()));
    }
    // Une photo PNG transparente n'est pas prise en charge non plus : rejetée avec un message utile.
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut ihdr = 4u32.to_be_bytes().to_vec();
    ihdr.extend_from_slice(&4u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let crc = |t: &[u8], d: &[u8]| {
        let mut c = 0xFFFF_FFFFu32;
        for &b in t.iter().chain(d) {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    };
    for (t, d) in [(&b"IHDR"[..], &ihdr[..]), (b"IDAT", &[0x78, 0x01, 1, 0, 0, 0xff, 0xff, 0, 0, 0, 1]), (b"IEND", &[])] {
        png.extend_from_slice(&(d.len() as u32).to_be_bytes());
        png.extend_from_slice(t);
        png.extend_from_slice(d);
        png.extend_from_slice(&crc(t, d).to_be_bytes());
    }
    let dep = w.ent.deposer_piece_for_actor(&w.lily_a(), w.depot("acte_naissance", s, None, vec![png])).await.unwrap();
    assert!(matches!(
        w.ent.analyser_piece(w.m.etab, dep.piece_id, w.av.clone()).await.unwrap(),
        ResultatAnalyse::Rejetee(m) if m.contains("transparence")
    ));
}

// ───────────────────────── Contrôle ─────────────────────────

#[tokio::test]
async fn t_controle_en_rafale() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Simon").await;
    w.lier(eleve, &w.lily, true, None).await;
    let s = SujetPiece::Eleve(eleve);
    let (d1, d2, d3) = (
        w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", s)).await.unwrap(),
        w.ent.demander_piece_for_actor(&w.adm(), w.demande("acte_naissance", s)).await.unwrap(),
        w.ent.demander_piece_for_actor(&w.adm(), w.demande("attestation_assurance", s)).await.unwrap(),
    );
    let p1 = w.deposer_et_analyser(&w.lily_a(), w.depot("justificatif_domicile", s, Some(d1.demande_id), vec![pdf_simple()])).await;
    let p2 = w.deposer_et_analyser(&w.lily_a(), w.depot("acte_naissance", s, Some(d2.demande_id), vec![pdf_simple()])).await;
    let p3 = w.deposer_et_analyser(&w.lily_a(), w.depot("attestation_assurance", s, Some(d3.demande_id), vec![pdf_simple()])).await;
    // La file : les plus anciennes d'abord, avec ce que la démarche attend et le nom du dossier.
    let file = w.ent.pieces_a_controler_for_actor(&w.adm(), 50).await.unwrap();
    assert_eq!(file.iter().map(|f| f.piece_id).collect::<Vec<_>>(), [p1, p2, p3]);
    assert_eq!(file[0].attendu.as_deref(), Some("Dossier d'inscription"));
    assert!(file[0].sujet_nom.contains("Simon") && file[0].nb_pages == 1);
    // Le professeur et la famille n'ont pas cette file.
    assert!(matches!(w.ent.pieces_a_controler_for_actor(&w.prof(), 10).await.unwrap_err(), EntDbError::Forbidden));
    assert!(matches!(w.ent.pieces_a_controler_for_actor(&w.lily_a(), 10).await.unwrap_err(), EntDbError::Forbidden));
    // Une rafale : accepter, refuser (illisible), sans objet, plus un doublon et une pièce inconnue.
    let res = w
        .ent
        .controle_en_rafale_for_actor(
            &w.adm(),
            vec![
                (p1, ControleDecision::Accepter { valable_jusqu_au: None }),
                (p2, ControleDecision::Refuser { motif: "illisible".into(), commentaire: None }),
                (p3, ControleDecision::SansObjet { commentaire: "Couvert par l'assurance de l'école".into() }),
                (p1, ControleDecision::Accepter { valable_jusqu_au: None }),
                (Uuid::new_v4(), ControleDecision::Accepter { valable_jusqu_au: None }),
            ],
        )
        .await
        .unwrap();
    assert_eq!(res.iter().map(|r| r.ok).collect::<Vec<_>>(), [true, true, true, false, false]);
    assert_eq!((w.etat(p1).await.as_str(), w.etat(p2).await.as_str(), w.etat(p3).await.as_str()), ("acceptee", "refusee", "sans_objet"));
    // Une pièce déjà décidée ne se redécide pas.
    assert!(est_refus(
        &w.ent.controler_piece_for_actor(&w.adm(), p2, ControleDecision::Accepter { valable_jusqu_au: None }).await.unwrap_err(),
        RefusMetier::Conflit
    ));
    // L'acceptation a posé la validité du domicile (3 mois) et satisfait la demande ; le refus laisse la demande ouverte
    // avec son motif ; la famille le voit.
    let (v,): (Option<NaiveDate>,) =
        sqlx::query_as("SELECT valable_jusqu_au FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2")
            .bind(w.m.etab)
            .bind(p1)
            .fetch_one(w.ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(v, w.jour.checked_add_months(chrono::Months::new(3)));
    let manque = w.ent.ce_qui_manque_for_actor(&w.lily_a(), s).await.unwrap();
    assert_eq!(manque.len(), 2, "acte de naissance (refusée) et assurance (sans objet : la demande reste ouverte)");
    let acte = manque.iter().find(|m| m.type_code == "acte_naissance").unwrap();
    assert_eq!((acte.etat, acte.refus_motif.as_deref()), (EtatManque::Refusee, Some("illisible")));
    // Refus : motif type obligatoire parmi la liste ; « autre » exige un commentaire.
    let p4 = w.deposer_et_analyser(&w.lily_a(), w.depot("acte_naissance", s, Some(d2.demande_id), vec![pdf_simple()])).await;
    assert!(w.ent.controler_piece_for_actor(&w.adm(), p4, ControleDecision::Refuser { motif: "inventé".into(), commentaire: None }).await.is_err());
    assert!(w.ent.controler_piece_for_actor(&w.adm(), p4, ControleDecision::Refuser { motif: "autre".into(), commentaire: None }).await.is_err());
    w.ent
        .controler_piece_for_actor(&w.adm(), p4, ControleDecision::Refuser { motif: "perime".into(), commentaire: Some("Daté de 2019".into()) })
        .await
        .unwrap();
    // Historique : une ligne de décision par geste, en ajout seul.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_piece_decision WHERE etablissement_id = $1 AND decision IN ('acceptee', 'refusee', 'sans_objet')",
    )
    .bind(w.m.etab)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 4);
    assert!(sqlx::query("UPDATE ent_doc_piece_decision SET decision = 'acceptee' WHERE etablissement_id = $1")
        .bind(w.m.etab)
        .execute(w.ent.pool_verif())
        .await
        .is_err());
    // L'accueil ne contrôle que l'identité : une pièce de domicile lui est fermée.
    let p5 = w.deposer_et_analyser(&w.lily_a(), w.depot("justificatif_domicile", s, None, vec![pdf_simple()])).await;
    assert!(matches!(
        w.ent.controler_piece_for_actor(&w.accueil(), p5, ControleDecision::Accepter { valable_jusqu_au: None }).await.unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(w.ent.pieces_a_controler_for_actor(&w.accueil(), 10).await.unwrap().is_empty());
}

#[tokio::test]
async fn t_controle_concurrent_une_seule_decision() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Bernard").await;
    w.lier(eleve, &w.lily, true, None).await;
    let s = SujetPiece::Eleve(eleve);
    let p = w.deposer_et_analyser(&w.lily_a(), w.depot("acte_naissance", s, None, vec![pdf_simple()])).await;
    let (adm, dir) = (w.adm(), w.dir());
    let (a, b) = tokio::join!(
        w.ent.controler_piece_for_actor(&adm, p, ControleDecision::Accepter { valable_jusqu_au: None }),
        w.ent.controler_piece_for_actor(&dir, p, ControleDecision::Refuser { motif: "incomplet".into(), commentaire: None }),
    );
    assert_eq!(a.is_ok() as u8 + b.is_ok() as u8, 1, "une seule décision : {a:?} / {b:?}");
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_piece_decision WHERE etablissement_id = $1 AND piece_id = $2 AND decision IN ('acceptee', 'refusee')",
    )
    .bind(w.m.etab)
    .bind(p)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn t_identite_sans_copie() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let s = SujetPiece::Personne(w.lily.personne);
    let p = w.deposer_et_analyser(&w.lily_a(), w.depot("piece_identite", s, None, vec![photo("carte")])).await;
    // L'accueil contrôle l'identité (et elle seule) ; le contrôleur la voit le temps du contrôle.
    let file = w.ent.pieces_a_controler_for_actor(&w.accueil(), 10).await.unwrap();
    assert_eq!(file.len(), 1);
    assert!(w.ent.lire_piece_justificative_for_actor(&w.accueil(), p).await.is_ok());
    w.ent.controler_piece_for_actor(&w.accueil(), p, ControleDecision::Accepter { valable_jusqu_au: None }).await.unwrap();
    // Décision prise : plus de copie. Les deux fichiers sont marqués supprimés au registre des objets et ne se relisent plus.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_objet o JOIN ent_doc_piece p ON p.etablissement_id = o.etablissement_id
            AND o.cle_objet IN (p.cle_objet_original, p.cle_objet_servi)
          WHERE p.etablissement_id = $1 AND p.id = $2 AND o.supprime_le IS NOT NULL",
    )
    .bind(w.m.etab)
    .bind(p)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 2);
    for qui in [w.accueil(), w.dir(), w.lily_a()] {
        assert!(est_refus(&w.ent.lire_piece_justificative_for_actor(&qui, p).await.unwrap_err(), RefusMetier::Conflit), "{}", qui.role);
    }
    assert_eq!(w.etat(p).await, "acceptee", "la décision, elle, est conservée");
}

// ───────────────────────── Foyer, réutilisation, ce qui manque ─────────────────────────

#[tokio::test]
async fn t_piece_de_foyer_pour_deux_enfants() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let foyer = w.foyer().await;
    let autre_foyer = w.foyer().await;
    let (a, b, c) = (w.eleve("Aîné").await, w.eleve("Cadet").await, w.eleve("Voisin").await);
    w.lier(a, &w.lily, true, Some(foyer)).await;
    w.lier(b, &w.lily, true, Some(foyer)).await;
    w.lier(c, &w.tom, true, Some(autre_foyer)).await;
    let (da, db_, dc) = (
        w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", SujetPiece::Eleve(a))).await.unwrap(),
        w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", SujetPiece::Eleve(b))).await.unwrap(),
        w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", SujetPiece::Eleve(c))).await.unwrap(),
    );
    assert_eq!(w.ent.ce_qui_manque_for_actor(&w.lily_a(), SujetPiece::Eleve(b)).await.unwrap().len(), 1);
    // Un seul dépôt, pour l'aîné : accepté, il satisfait la demande du cadet (même foyer), pas celle du voisin.
    let p = w.deposer_et_analyser(&w.lily_a(), w.depot("justificatif_domicile", SujetPiece::Eleve(a), Some(da.demande_id), vec![pdf_simple()])).await;
    w.ent.controler_piece_for_actor(&w.adm(), p, ControleDecision::Accepter { valable_jusqu_au: None }).await.unwrap();
    let etat_demande = |d: Uuid| async move {
        let (e, pid): (String, Option<Uuid>) =
            sqlx::query_as("SELECT etat, piece_id FROM ent_doc_piece_demande WHERE etablissement_id = $1 AND id = $2")
                .bind(w.m.etab)
                .bind(d)
                .fetch_one(w.ent.pool_verif())
                .await
                .unwrap();
        (e, pid)
    };
    assert_eq!(etat_demande(da.demande_id).await, ("satisfaite".to_string(), Some(p)));
    assert_eq!(etat_demande(db_.demande_id).await, ("satisfaite".to_string(), Some(p)), "une pièce de foyer pour deux enfants");
    assert_eq!(etat_demande(dc.demande_id).await.0, "ouverte", "un autre foyer n'est pas couvert");
    assert_eq!(w.nb_pieces().await, 1);
    assert!(w.ent.ce_qui_manque_for_actor(&w.lily_a(), SujetPiece::Eleve(b)).await.unwrap().is_empty());
    // Le voisin ne se voit pas proposer la pièce d'un autre foyer.
    assert!(w.ent.ce_qui_manque_for_actor(&w.tom_a(), SujetPiece::Eleve(c)).await.unwrap()[0].proposition_piece_id.is_none());
    // Un frère de plus dans le foyer : la pièce valable est PROPOSÉE à la demande.
    let d = w.eleve("Benjamin").await;
    w.lier(d, &w.lily, true, Some(foyer)).await;
    let dd = w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", SujetPiece::Eleve(d))).await.unwrap();
    assert_eq!(dd.proposition_piece_id, Some(p));
    // Une pièce d'élève (acte de naissance) ne se propose jamais à un frère.
    let acte = w.deposer_et_analyser(&w.lily_a(), w.depot("acte_naissance", SujetPiece::Eleve(a), None, vec![pdf_simple()])).await;
    w.ent.controler_piece_for_actor(&w.adm(), acte, ControleDecision::Accepter { valable_jusqu_au: None }).await.unwrap();
    let dn = w.ent.demander_piece_for_actor(&w.adm(), w.demande("acte_naissance", SujetPiece::Eleve(b))).await.unwrap();
    assert!(dn.proposition_piece_id.is_none());
}

#[tokio::test]
async fn t_reutilisation_confirmee() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let foyer = w.foyer().await;
    let (a, b) = (w.eleve("Un").await, w.eleve("Deux").await);
    w.lier(a, &w.lily, true, Some(foyer)).await;
    let da = w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", SujetPiece::Eleve(a))).await.unwrap();
    let p = w.deposer_et_analyser(&w.lily_a(), w.depot("justificatif_domicile", SujetPiece::Eleve(a), Some(da.demande_id), vec![pdf_simple()])).await;
    w.ent.controler_piece_for_actor(&w.adm(), p, ControleDecision::Accepter { valable_jusqu_au: None }).await.unwrap();
    // Le second enfant arrive dans le foyer après coup.
    w.lier(b, &w.lily, true, Some(foyer)).await;
    let db_ = w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", SujetPiece::Eleve(b))).await.unwrap();
    // Proposée, jamais utilisée en silence : la demande reste ouverte jusqu'à confirmation.
    assert_eq!(db_.proposition_piece_id, Some(p));
    let manque = w.ent.ce_qui_manque_for_actor(&w.lily_a(), SujetPiece::Eleve(b)).await.unwrap();
    assert_eq!((manque[0].etat, manque[0].proposition_piece_id), (EtatManque::AFournir, Some(p)));
    // Un parent qui n'a pas la main sur l'élève ne confirme rien ; une pièce hors dossier ne convient pas.
    assert!(introuvable(&w.ent.reutiliser_piece_for_actor(&w.tom_a(), db_.demande_id, p).await.unwrap_err()));
    let autre = w.eleve("Etranger").await;
    w.lier(autre, &w.tom, true, None).await;
    let pt = w.deposer_et_analyser(&w.tom_a(), w.depot("justificatif_domicile", SujetPiece::Eleve(autre), None, vec![pdf_simple()])).await;
    w.ent.controler_piece_for_actor(&w.adm(), pt, ControleDecision::Accepter { valable_jusqu_au: None }).await.unwrap();
    assert!(est_refus(&w.ent.reutiliser_piece_for_actor(&w.lily_a(), db_.demande_id, pt).await.unwrap_err(), RefusMetier::Conflit));
    let acte = w.deposer_et_analyser(&w.lily_a(), w.depot("acte_naissance", SujetPiece::Eleve(a), None, vec![pdf_simple()])).await;
    w.ent.controler_piece_for_actor(&w.adm(), acte, ControleDecision::Accepter { valable_jusqu_au: None }).await.unwrap();
    assert!(est_refus(&w.ent.reutiliser_piece_for_actor(&w.lily_a(), db_.demande_id, acte).await.unwrap_err(), RefusMetier::Conflit), "mauvais type");
    // La famille confirme : la demande est satisfaite par la même pièce, aucun nouveau fichier.
    let avant = w.nb_pieces().await;
    w.ent.reutiliser_piece_for_actor(&w.lily_a(), db_.demande_id, p).await.unwrap();
    assert_eq!(w.nb_pieces().await, avant);
    assert!(w.ent.ce_qui_manque_for_actor(&w.lily_a(), SujetPiece::Eleve(b)).await.unwrap().is_empty());
    assert!(est_refus(&w.ent.reutiliser_piece_for_actor(&w.lily_a(), db_.demande_id, p).await.unwrap_err(), RefusMetier::Conflit), "demande close");
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_piece_decision WHERE etablissement_id = $1 AND decision = 'reutilisee' AND piece_id = $2 AND demande_id = $3",
    )
    .bind(w.m.etab)
    .bind(p)
    .bind(db_.demande_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1);
    // Pièce périmée : plus proposée, et signalée « à renouveler » (validité forcée dans le passé pour le test, déclencheurs
    // suspendus le temps d'une transaction du propriétaire seulement).
    let mut tx = w.ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE ent_doc_piece SET valable_jusqu_au = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(w.m.etab)
        .bind(p)
        .bind(w.jour - chrono::Duration::days(1))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let c = w.eleve("Trois").await;
    w.lier(c, &w.lily, true, Some(foyer)).await;
    let dc = w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", SujetPiece::Eleve(c))).await.unwrap();
    assert!(dc.proposition_piece_id.is_none(), "une pièce périmée ne se propose pas");
    let manque = w.ent.ce_qui_manque_for_actor(&w.lily_a(), SujetPiece::Eleve(a)).await.unwrap();
    let perimee = manque.iter().find(|m| m.etat == EtatManque::Perimee).expect("à renouveler");
    assert_eq!(perimee.perimee_le, Some(w.jour - chrono::Duration::days(1)));
}

#[tokio::test]
async fn t_ce_qui_manque() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Marchand").await;
    w.lier(eleve, &w.lily, true, None).await;
    w.lier(eleve, &w.tom, false, None).await; // sans autorité parentale
    let s = SujetPiece::Eleve(eleve);
    let mut facultative = w.demande("exeat", s);
    facultative.obligatoire = false;
    facultative.echeance = Some(w.jour + chrono::Duration::days(30));
    w.ent.demander_piece_for_actor(&w.adm(), facultative).await.unwrap();
    let dom = w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", s)).await.unwrap();
    let ass = w.ent.demander_piece_for_actor(&w.adm(), w.demande("attestation_assurance", s)).await.unwrap();
    w.ent.demander_piece_for_actor(&w.dir(), w.demande("decision_justice", s)).await.unwrap();
    let manque = w.ent.ce_qui_manque_for_actor(&w.lily_a(), s).await.unwrap();
    assert_eq!(manque.len(), 4);
    assert!(manque.iter().all(|m| m.etat == EtatManque::AFournir));
    // L'échéance d'abord : l'exeat (échéance dans 30 jours) précède les demandes sans échéance.
    assert_eq!(manque[0].type_code, "exeat");
    assert!(!manque[0].obligatoire && manque.iter().filter(|m| m.obligatoire).count() == 3);
    assert!(!w.ent.dossier_complet_pieces(&w.lily_a(), s).await.unwrap());
    // Chaque état.
    let p_dom = w.deposer_et_analyser(&w.lily_a(), w.depot("justificatif_domicile", s, Some(dom.demande_id), vec![pdf_simple()])).await;
    let p_ass = w.ent.deposer_piece_for_actor(&w.lily_a(), w.depot("attestation_assurance", s, Some(ass.demande_id), vec![pdf_simple()])).await.unwrap();
    let etat_de = |m: &[LigneManque], t: &str| m.iter().find(|x| x.type_code == t).map(|x| x.etat);
    let manque = w.ent.ce_qui_manque_for_actor(&w.lily_a(), s).await.unwrap();
    assert_eq!(etat_de(&manque, "justificatif_domicile"), Some(EtatManque::RecueEnAttente));
    assert_eq!(etat_de(&manque, "attestation_assurance"), Some(EtatManque::EnAnalyse));
    assert_eq!(etat_de(&manque, "exeat"), Some(EtatManque::AFournir));
    // Le secrétariat voit la même chose, sans la pièce de justice (droit de domaine) ; la direction la voit.
    let vue_adm = w.ent.ce_qui_manque_for_actor(&w.adm(), s).await.unwrap();
    assert!(etat_de(&vue_adm, "decision_justice").is_none() && vue_adm.len() == 3);
    assert_eq!(w.ent.ce_qui_manque_for_actor(&w.dir(), s).await.unwrap().len(), 4);
    // Ni le parent sans autorité, ni le professeur, ni une personne étrangère au dossier.
    assert!(matches!(w.ent.ce_qui_manque_for_actor(&w.tom_a(), s).await.unwrap_err(), EntDbError::Forbidden));
    assert!(matches!(w.ent.ce_qui_manque_for_actor(&w.prof(), s).await.unwrap_err(), EntDbError::Forbidden));
    // Dossier complet : tout ce qui est obligatoire est reçu ou en analyse (le reste de la démarche est à l'agent).
    w.ent.controler_piece_for_actor(&w.adm(), p_dom, ControleDecision::Accepter { valable_jusqu_au: None }).await.unwrap();
    w.ent.analyser_piece(w.m.etab, p_ass.piece_id, w.av.clone()).await.unwrap();
    w.ent.controler_piece_for_actor(&w.adm(), p_ass.piece_id, ControleDecision::Refuser { motif: "incomplet".into(), commentaire: None }).await.unwrap();
    let manque = w.ent.ce_qui_manque_for_actor(&w.lily_a(), s).await.unwrap();
    assert_eq!(etat_de(&manque, "attestation_assurance"), Some(EtatManque::Refusee));
    assert_eq!(etat_de(&manque, "justificatif_domicile"), None, "acceptée : ne manque plus");
}

// ───────────────────────── Dépôts : refus à l'entrée, droits, retrait ─────────────────────────

#[tokio::test]
async fn t_depots_refuses_a_l_entree() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Fabre").await;
    w.lier(eleve, &w.lily, true, None).await;
    let s = SujetPiece::Eleve(eleve);
    let lily = w.lily_a();
    // Formes refusées sans rien stocker : exécutable, page web, archive, vide, trop gros, mélange, trop de pages.
    let trop_gros = {
        let mut v = b"%PDF-1.4\n".to_vec();
        v.resize(FICHIER_MAX + 1, b' ');
        v
    };
    let cas: Vec<(&str, Vec<Vec<u8>>)> = vec![
        ("exécutable", vec![b"MZ\x90\x00\x03".to_vec()]),
        ("page web", vec![b"<html><script>alert(1)</script></html>".to_vec()]),
        ("archive", vec![b"PK\x03\x04....".to_vec()]),
        ("vide", vec![Vec::new()]),
        ("aucun fichier", vec![]),
        ("trop gros", vec![trop_gros]),
        ("pdf et photo", vec![pdf_simple(), photo("x")]),
        ("deux pdf", vec![pdf_simple(), pdf_simple()]),
        ("cinquante et une photos", (0..51).map(|_| photo("x")).collect()),
    ];
    for (nom, fichiers) in cas {
        assert!(w.ent.deposer_piece_for_actor(&lily, w.depot("acte_naissance", s, None, fichiers)).await.is_err(), "{nom}");
    }
    assert_eq!(w.nb_pieces().await, 0, "aucun refus n'a laissé de ligne");
    // Le motif d'une pièce « autre » est exigé, et jamais interdit.
    assert!(w.ent.deposer_piece_for_actor(&lily, w.depot("autre", s, None, vec![pdf_simple()])).await.is_err());
    let mut d = w.depot("autre", s, None, vec![pdf_simple()]);
    d.motif_autre = Some("Copie de la carte vitale".into());
    assert!(w.ent.deposer_piece_for_actor(&lily, d).await.is_err());
    let mut d = w.depot("autre", s, None, vec![pdf_simple()]);
    d.motif_autre = Some("Autorisation de sortie exceptionnelle".into());
    assert!(w.ent.deposer_piece_for_actor(&lily, d).await.is_ok());
    // Une demande qui ne correspond pas au dépôt (autre type).
    let dem = w.ent.demander_piece_for_actor(&w.adm(), w.demande("justificatif_domicile", s)).await.unwrap();
    assert!(est_refus(
        &w.ent.deposer_piece_for_actor(&lily, w.depot("acte_naissance", s, Some(dem.demande_id), vec![pdf_simple()])).await.unwrap_err(),
        RefusMetier::Conflit
    ));
    // Droits : parent sans autorité, parent sans lien, parent dont le lien est suspendu, élève mineur, professeur.
    w.lier(eleve, &w.tom, false, None).await;
    assert!(matches!(
        w.ent.deposer_piece_for_actor(&w.tom_a(), w.depot("acte_naissance", s, None, vec![pdf_simple()])).await.unwrap_err(),
        EntDbError::Forbidden
    ));
    let autre_eleve = w.eleve("Etranger").await;
    assert!(matches!(
        w.ent.deposer_piece_for_actor(&lily, w.depot("acte_naissance", SujetPiece::Eleve(autre_eleve), None, vec![pdf_simple()])).await.unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(matches!(
        w.ent.deposer_piece_for_actor(&w.prof(), w.depot("acte_naissance", s, None, vec![pdf_simple()])).await.unwrap_err(),
        EntDbError::Forbidden
    ));
    sqlx::query("UPDATE ent_eleve_lien SET suspendu_le = now(), suspension_motif = 'test' WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3")
        .bind(w.m.etab)
        .bind(eleve)
        .bind(w.lily.personne)
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    assert!(matches!(
        w.ent.deposer_piece_for_actor(&lily, w.depot("acte_naissance", s, None, vec![pdf_simple()])).await.unwrap_err(),
        EntDbError::Forbidden
    ), "lien suspendu");
    // Élève mineur : il ne dépose pas lui-même ; majeur, il dépose pour lui.
    let mineur = w.eleve_ne_le("Petit", "Mineur", w.jour - chrono::Duration::days(365 * 15)).await;
    let majeur = w.eleve_ne_le("Grand", "Majeur", w.jour - chrono::Duration::days(365 * 19)).await;
    let mut comptes = Vec::new();
    for e in [mineur, majeur] {
        let user = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at) VALUES ($1, $2, 'x', 'Élève', now(), now())")
            .bind(user)
            .bind(format!("eleve-{}@ent.lab", user.simple()))
            .execute(w.ent.pool_verif())
            .await
            .unwrap();
        sqlx::query("INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id, created_at) VALUES ($1, $2, $3, now())")
            .bind(w.m.etab)
            .bind(e)
            .bind(user)
            .execute(w.ent.pool_verif())
            .await
            .unwrap();
        comptes.push(w.acteur(user, "eleve"));
    }
    assert!(matches!(
        w.ent.deposer_piece_for_actor(&comptes[0], w.depot("acte_naissance", SujetPiece::Eleve(mineur), None, vec![pdf_simple()])).await.unwrap_err(),
        EntDbError::Forbidden
    ));
    assert!(w.ent.deposer_piece_for_actor(&comptes[1], w.depot("acte_naissance", SujetPiece::Eleve(majeur), None, vec![pdf_simple()])).await.is_ok());
    assert!(matches!(
        w.ent.deposer_piece_for_actor(&comptes[1], w.depot("acte_naissance", s, None, vec![pdf_simple()])).await.unwrap_err(),
        EntDbError::Forbidden
    ), "un élève majeur ne dépose pas pour un autre");
    // Le guichet : un agent dépose pour un élève (scan), avec le droit `documents.deposer`.
    assert!(w.ent.deposer_piece_for_actor(&w.adm(), w.depot("acte_naissance", s, None, vec![pdf_simple()])).await.is_ok());
}

#[tokio::test]
async fn t_debit_des_depots() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Leclerc").await;
    w.lier(eleve, &w.lily, true, None).await;
    let s = SujetPiece::Eleve(eleve);
    for i in 0..DEPOTS_PAR_JOUR {
        w.ent
            .deposer_piece_for_actor(&w.lily_a(), w.depot("exeat", s, None, vec![pdf(&format!("% {i}"), "")]))
            .await
            .unwrap_or_else(|e| panic!("dépôt {i} : {e:?}"));
    }
    let e = w.ent.deposer_piece_for_actor(&w.lily_a(), w.depot("exeat", s, None, vec![pdf_simple()])).await.unwrap_err();
    assert!(est_refus(&e, RefusMetier::Conflit), "{e:?}");
    // Le compte du voisin n'est pas touché.
    let autre = w.eleve("Autre").await;
    w.lier(autre, &w.tom, true, None).await;
    assert!(w.ent.deposer_piece_for_actor(&w.tom_a(), w.depot("exeat", SujetPiece::Eleve(autre), None, vec![pdf_simple()])).await.is_ok());
}

#[tokio::test]
async fn t_retrait_par_le_deposant() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Colin").await;
    w.lier(eleve, &w.lily, true, None).await;
    w.lier(eleve, &w.tom, true, None).await;
    let s = SujetPiece::Eleve(eleve);
    // Déposée par erreur, retirée avant tout contrôle (en quarantaine ou reçue).
    let p = w.ent.deposer_piece_for_actor(&w.lily_a(), w.depot("acte_naissance", s, None, vec![pdf_simple()])).await.unwrap();
    assert!(introuvable(&w.ent.retirer_depot_for_actor(&w.tom_a(), p.piece_id).await.unwrap_err()), "pas son dépôt : aucun indice");
    w.ent.retirer_depot_for_actor(&w.lily_a(), p.piece_id).await.unwrap();
    assert_eq!(w.etat(p.piece_id).await, "retiree");
    assert_eq!(w.ent.analyser_piece(w.m.etab, p.piece_id, w.av.clone()).await.unwrap(), ResultatAnalyse::DejaTraitee);
    let q = w.deposer_et_analyser(&w.lily_a(), w.depot("acte_naissance", s, None, vec![pdf_simple()])).await;
    w.ent.retirer_depot_for_actor(&w.lily_a(), q).await.unwrap();
    assert!(w.ent.pieces_a_controler_for_actor(&w.adm(), 10).await.unwrap().is_empty(), "retirée : plus à contrôler");
    // Une fois contrôlée, plus de retrait.
    let r = w.deposer_et_analyser(&w.lily_a(), w.depot("acte_naissance", s, None, vec![pdf_simple()])).await;
    w.ent.controler_piece_for_actor(&w.adm(), r, ControleDecision::Accepter { valable_jusqu_au: None }).await.unwrap();
    assert!(est_refus(&w.ent.retirer_depot_for_actor(&w.lily_a(), r).await.unwrap_err(), RefusMetier::Conflit));
}

// ───────────────────────── Reprise N1-5 ─────────────────────────

#[tokio::test]
async fn t_port_reprise_n1_5() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let saine = pdf_simple();
    let piegee = pdf("", &format!("4 0 obj\n<< /Length 70 >>\nstream\n{}\nendstream\nendobj\n", String::from_utf8_lossy(EICAR)));
    let js = pdf("/OpenAction 5 0 R", "5 0 obj\n<< /S /JavaScript /JS (x) >>\nendobj\n");
    let pieces = vec![
        PieceARepriser { origine_id: "p-saine".into(), personne_id: w.lily.personne, octets: saine.clone() },
        PieceARepriser { origine_id: "p-infectee".into(), personne_id: w.lily.personne, octets: piegee },
        PieceARepriser { origine_id: "p-js".into(), personne_id: w.lily.personne, octets: js },
        PieceARepriser { origine_id: "p-photo".into(), personne_id: w.tom.personne, octets: photo("x") },
        PieceARepriser { origine_id: "p-exe".into(), personne_id: w.tom.personne, octets: b"MZ\x90\x00".to_vec() },
    ];
    let port = w.ent.preparer_reprise_pieces(w.m.etab, pieces.clone(), w.av.clone()).await.unwrap();
    let source = |id: &'static str| PieceSource { origine_id: id, nom_fichier: "x.pdf", cle_stockage: "k", type_contenu: None };
    // Seules les pièces saines sont reprises ; les autres sont signalées au rapport (None), jamais en silence.
    for (id, attendu) in [("p-saine", true), ("p-js", true), ("p-photo", true), ("p-infectee", false), ("p-exe", false), ("p-inconnue", false)] {
        assert_eq!(port.copier(w.m.etab, &source(id)).is_some(), attendu, "{id}");
    }
    assert_eq!(port.nombre(), 3);
    let doc = port.copier(w.m.etab, &source("p-saine")).unwrap();
    assert_eq!(doc.pages, Some(1));
    assert_eq!(w.etat(doc.document_id).await, "acceptee");
    // Idempotent : rejouer la reprise retrouve les mêmes documents, sans doublon.
    let avant = w.nb_pieces().await;
    let encore = w.ent.preparer_reprise_pieces(w.m.etab, pieces[..1].to_vec(), w.av.clone()).await.unwrap();
    assert_eq!(encore.copier(w.m.etab, &source("p-saine")).unwrap().document_id, doc.document_id);
    assert_eq!(w.nb_pieces().await, avant);
    // Antivirus arrêté : rien n'est repris (les pièces restent en quarantaine), toutes signalées.
    let av = Av::sain();
    av.arrete.store(true, Ordering::SeqCst);
    let rien = w
        .ent
        .preparer_reprise_pieces(
            w.m.etab,
            vec![PieceARepriser { origine_id: "p-nouvelle".into(), personne_id: w.lily.personne, octets: pdf("% autre", "") }],
            av,
        )
        .await
        .unwrap();
    assert_eq!(rien.nombre(), 0);
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_piece WHERE etablissement_id = $1 AND etat = 'en_quarantaine'",
    )
    .bind(w.m.etab)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1);
    // Le port « absent » de la reprise reste valable : sans socle 7, tout est signalé.
    assert!(crate::demandes_reprise::PiecesAbsentes.copier(w.m.etab, &source("p-saine")).is_none());
}

// ───────────────────────── Séparation entre établissements ─────────────────────────

#[tokio::test]
async fn t_separation_entre_etablissements() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eleve = w.eleve("Gauthier").await;
    w.lier(eleve, &w.lily, true, None).await;
    let s = SujetPiece::Eleve(eleve);
    let dem = w.ent.demander_piece_for_actor(&w.adm(), w.demande("acte_naissance", s)).await.unwrap();
    let p = w.deposer_et_analyser(&w.lily_a(), w.depot("acte_naissance", s, Some(dem.demande_id), vec![pdf_simple()])).await;
    // Un autre établissement, ses comptes.
    let cast_b = crate::tests_rls_stricte::seed_supplementaire(&w.ent).await;
    let mb = monde(&w.ent, &cast_b).await;
    let parent_b = fiche(&w.ent, mb.etab, cast_b.parent_lea.user_id, "parent", false).await;
    let adm_b = Acteur { etablissement_id: mb.etab, user_id: cast_b.administratif.user_id, role: "administratif" };
    let dir_b = Acteur { etablissement_id: mb.etab, user_id: cast_b.direction.user_id, role: "direction" };
    let par_b = Acteur { etablissement_id: mb.etab, user_id: parent_b.user, role: "parent" };
    // Lire, contrôler, retirer, réutiliser, déposer, demander : tout est « introuvable » ou refusé, aucun indice.
    assert!(introuvable(&w.ent.lire_piece_justificative_for_actor(&dir_b, p).await.unwrap_err()));
    assert!(introuvable(&w.ent.lire_piece_justificative_for_actor(&par_b, p).await.unwrap_err()));
    assert!(introuvable(
        &w.ent.controler_piece_for_actor(&adm_b, p, ControleDecision::Accepter { valable_jusqu_au: None }).await.unwrap_err()
    ));
    assert!(introuvable(&w.ent.retirer_depot_for_actor(&par_b, p).await.unwrap_err()));
    assert!(w.ent.reutiliser_piece_for_actor(&par_b, dem.demande_id, p).await.is_err());
    assert!(w.ent.deposer_piece_for_actor(&par_b, w.depot("acte_naissance", s, None, vec![pdf_simple()])).await.is_err());
    assert!(w.ent.demander_piece_for_actor(&adm_b, w.demande("livret_famille", s)).await.is_err());
    assert!(w.ent.ce_qui_manque_for_actor(&dir_b, s).await.is_err());
    assert!(w.ent.pieces_a_controler_for_actor(&adm_b, 50).await.unwrap().is_empty());
    assert_eq!(w.ent.analyser_piece(mb.etab, p, w.av.clone()).await.err().map(|e| introuvable(&e)), Some(true));
    // Le compte de A, présenté dans le contexte de B (même identité, autre établissement) : rien non plus.
    let lily_chez_b = Acteur { etablissement_id: mb.etab, ..w.lily_a() };
    assert!(introuvable(&w.ent.lire_piece_justificative_for_actor(&lily_chez_b, p).await.unwrap_err()));
    // Les files de B ne contiennent rien de A, même après une pièce déposée chez B.
    let eleve_b = cast_b.eleve_lea_id;
    let adm_b_a = adm_b;
    let sb = SujetPiece::Eleve(eleve_b);
    w.ent.demander_piece_for_actor(&adm_b_a, w.demande("acte_naissance", sb)).await.unwrap();
    let pb = w.ent.deposer_piece_for_actor(&adm_b_a, w.depot("acte_naissance", sb, None, vec![pdf("% b", "")])).await.unwrap();
    w.ent.analyser_piece(mb.etab, pb.piece_id, w.av.clone()).await.unwrap();
    let file_b = w.ent.pieces_a_controler_for_actor(&adm_b_a, 50).await.unwrap();
    assert_eq!(file_b.iter().map(|f| f.piece_id).collect::<Vec<_>>(), [pb.piece_id]);
    let file_a = w.ent.pieces_a_controler_for_actor(&w.adm(), 50).await.unwrap();
    assert_eq!(file_a.iter().map(|f| f.piece_id).collect::<Vec<_>>(), [p]);
    // La clé d'objet commence par l'établissement : jamais l'objet d'un autre cloisonnement.
    let (o, s_): (String, String) = sqlx::query_as(
        "SELECT cle_objet_original, cle_objet_servi FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.m.etab)
    .bind(p)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert!(o.starts_with(&w.m.etab.to_string()) && s_.starts_with(&w.m.etab.to_string()));
}
