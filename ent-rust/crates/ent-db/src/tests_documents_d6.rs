//! Socle 7 (documents), lot D6 : la brique de signature, contre la base locale de test (feature `labo`) ; sans
//! `ENT_DATABASE_URL`, les tests sont ignorés en local (échec explicite en CI ou avec `ENT_EXIGER_BASE`). Aucun test ne
//! dépend de l'heure : les jours viennent de la base (`ent_aujourdhui_etab`) avec de larges marges, les expirations se
//! fabriquent en écrivant l'instant voulu, jamais en attendant.
//!
//! | Exigence (§23, ligne D6 ; décisions de Florian du 16-11) | Test(s) |
//! |---|---|
//! | règle « un titulaire » : le premier signe, le circuit est complet, PDF signé et dossier de preuve | `t_regle_un_titulaire_code_sms_et_pieces` |
//! | règle « tous » : deux titulaires, horodatage par défaut de la finalisation d'inscription | `t_regle_tous_et_horodatage` |
//! | mineur ne signe pas seul ; majeur signe seul | `t_mineur_ne_signe_pas_seul_majeur_si` |
//! | nouvelle version → circuit obsolète, signature refusée, nouveau circuit | `t_nouvelle_version_rend_le_circuit_obsolete` |
//! | passkey : défi lié à l'empreinte, vérification, rejeu refusé, lien recalculable | `t_passkey_defi_lie_rejeu_refuse` |
//! | code SMS : essais comptés, expiration, plafond d'envois | `t_code_essais_expiration_plafond` |
//! | guichet : agent identifié, identité vue, confirmation (G) ou simple (G0) | `t_guichet_confirmation_ou_simple` |
//! | papier : dépôt, contrôle humain, refus | `t_papier_depot_et_controle` |
//! | menu par nature : bloqué, signalé, réglable, 4 modes au plus | `t_menu_bloque_signale_et_reglable` |
//! | horodatage RFC 3161 : désactivé par défaut, faux service, relevé, reprise | `t_horodatage_port_releve_et_reprise` |
//! | parapheur (à l'unité), signataire externe (lien + code), tuteur choisit son mode | `t_parapheur_et_externe` |
//! | chaîne : ajout seul, altération détectée, vérification interne | `t_chaine_ajout_seul_et_verification` |
//! | vérification publique à partir du PDF signé seul | `t_verification_par_depot_du_pdf_signe` |
//! | droits côté serveur, refus, lien suspendu | `t_droits_et_lien_suspendu` |
//! | séparation entre établissements | `t_separation_entre_etablissements` |

use super::*;
use crate::documents::*;
use crate::documents_pdf::RenduPdfAttestation;
use crate::documents_repo::*;
use crate::documents_signature::*;
use crate::documents_signature::Niveau;
use crate::documents_signature_repo::*;
use crate::documents_signature_signer::*;
use crate::ref_commun::Acteur;
use crate::service::{EntDbError, RefusMetier};
use crate::tests_demandes::{fiche, labo, monde, Acteur as ActeurTest, Monde};
use chrono::NaiveDate;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use webauthn_authenticator_rs::prelude::WebauthnAuthenticator;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;

// ───────────────────────── Doublures ─────────────────────────

struct Cachet;

impl CachetEtablissement for Cachet {
    fn sceller(&self, _: Uuid, pdf: &[u8]) -> Result<Scelle, ErreurCachet> {
        let mut o = pdf.to_vec();
        o.extend_from_slice(b"\n%SCELLE-TEST\n");
        Ok(Scelle { octets: o, cle_ref: "cle-test-1".into() })
    }
}

struct CachetEnPanne;

impl CachetEtablissement for CachetEnPanne {
    fn sceller(&self, _: Uuid, _: &[u8]) -> Result<Scelle, ErreurCachet> {
        Err(ErreurCachet::Indisponible)
    }
}

#[derive(Default)]
struct Sms {
    vus: Mutex<Vec<(String, String)>>,
}

impl Sms {
    fn dernier_code(&self) -> String {
        let vus = self.vus.lock().unwrap();
        let (_, texte) = vus.last().expect("un SMS a été envoyé");
        texte.split(|c: char| !c.is_ascii_digit()).find(|m| m.len() == 6).expect("code à six chiffres").to_string()
    }
    fn nombre(&self) -> usize {
        self.vus.lock().unwrap().len()
    }
}

impl EnvoiCodeSignature for Sms {
    fn envoyer<'a>(&'a self, t: &'a str, c: &'a str) -> FutureEnvoi<'a> {
        Box::pin(async move {
            self.vus.lock().unwrap().push((t.to_string(), c.to_string()));
            Ok(())
        })
    }
}

/// Faux service RFC 3161 : échoue `echecs` fois, puis rend un jeton opaque.
struct FauxTsa {
    appels: Arc<AtomicUsize>,
    echecs: Arc<AtomicUsize>,
}

struct FauxTsaService {
    appels: Arc<AtomicUsize>,
    echecs: Arc<AtomicUsize>,
}

impl ServiceHorodatage for FauxTsaService {
    fn horodater(&self, empreinte: &[u8; 32]) -> Result<Vec<u8>, ErreurHorodatage> {
        self.appels.fetch_add(1, Ordering::SeqCst);
        if self.echecs.load(Ordering::SeqCst) > 0 {
            self.echecs.fetch_sub(1, Ordering::SeqCst);
            return Err(ErreurHorodatage::Indisponible("service de test en panne".into()));
        }
        let mut jeton = b"JETON-TEST".to_vec();
        jeton.extend_from_slice(empreinte);
        Ok(jeton)
    }
}

impl FournisseurHorodatage for FauxTsa {
    fn service(
        &self,
        _: Uuid,
        _: ModeHorodatage,
        _: Option<&ConfigServiceHorodatage>,
    ) -> Option<Box<dyn ServiceHorodatage>> {
        Some(Box::new(FauxTsaService { appels: self.appels.clone(), echecs: self.echecs.clone() }))
    }
}

fn origine() -> url::Url {
    url::Url::parse("http://localhost:3000").unwrap()
}

// ───────────────────────── Le monde de test ─────────────────────────

struct W {
    ent: PgEnt,
    cast: LaboCast,
    m: Monde,
    lily: ActeurTest,
    tom: ActeurTest,
    jour: NaiveDate,
    cachet: Cachet,
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
        let w = W { ent, cast, m, lily, tom, jour, cachet: Cachet };
        w.mobile(w.lily.personne, "0612345678").await;
        w.mobile(w.tom.personne, "0687654321").await;
        Some(w)
    }

    fn ctx<'a>(&'a self, h: &'a dyn FournisseurHorodatage) -> ContexteSignature<'a> {
        ContexteSignature {
            moteur: Moteur { rendu: &RenduPdfAttestation, cachet: &self.cachet, url_verification: "https://test.scolia.fr/verifier" },
            horodatage: h,
        }
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
    fn parent(&self, p: &ActeurTest) -> Acteur<'static> {
        self.acteur(p.user, "parent")
    }

    async fn mobile(&self, personne: Uuid, tel: &str) {
        // Le labo donne déjà un téléphone principal à ces personnes : on le clôt (la base garde l'historique).
        sqlx::query(
            "UPDATE ent_personne_coordonnee SET au = now() WHERE etablissement_id = $1 AND personne_id = $2
                AND au IS NULL AND type IN ('mobile', 'fixe', 'professionnel')",
        )
        .bind(self.m.etab)
        .bind(personne)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO ent_personne_coordonnee (id, etablissement_id, personne_id, type, valeur, principal, source)
             VALUES ($1, $2, $3, 'mobile', $4, true, 'saisie')",
        )
        .bind(Uuid::new_v4())
        .bind(self.m.etab)
        .bind(personne)
        .bind(tel)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
    }

    async fn eleve(&self, nom: &str, prenom: &str) -> Uuid {
        self.eleve_ne_le(nom, prenom, NaiveDate::from_ymd_opt(2012, 3, 10).unwrap()).await
    }

    async fn eleve_ne_le(&self, nom: &str, prenom: &str, naissance: NaiveDate) -> Uuid {
        let pool = self.ent.pool_verif();
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
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO ent_eleve_scolarite_annee (id, etablissement_id, eleve_id, annee_scolaire, niveau, classe, statut, source)
             VALUES ($1, $2, $3, '2026-2027', '4e', '4B', 'courante', 'saisie')",
        )
        .bind(Uuid::new_v4())
        .bind(self.m.etab)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
        id
    }

    async fn lier(&self, eleve: Uuid, p: &ActeurTest, autorite: bool) {
        sqlx::query(
            "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale)
             VALUES ($1, $2, $3, $4, $5, 'parent', $6)",
        )
        .bind(Uuid::new_v4())
        .bind(self.m.etab)
        .bind(eleve)
        .bind(p.user)
        .bind(p.personne)
        .bind(autorite)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
    }

    async fn suspendre(&self, eleve: Uuid, p: &ActeurTest) {
        sqlx::query(
            "UPDATE ent_eleve_lien SET suspendu_le = now(), suspension_motif = 'suspension de test'
              WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3",
        )
        .bind(self.m.etab)
        .bind(eleve)
        .bind(p.personne)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
    }

    /// Les triggers de la base interdisent de réécrire l'identité d'un document : on les suspend le temps d'une
    /// instruction de préparation, dans une transaction du propriétaire seulement.
    async fn preparer(&self, sql: &str, binds: &[Uuid]) {
        let mut tx = self.ent.pool_verif().begin().await.unwrap();
        sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await.unwrap();
        let mut q = sqlx::query(sql);
        for b in binds {
            q = q.bind(*b);
        }
        q.execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
    }

    /// Un document réellement émis (moteur PDF maison, cachet de test) puis, si besoin, rangé sous une autre nature
    /// déclarée à la signature (les natures des autres socles n'existent pas encore côté émission : D8).
    async fn document(&self, eleve: Uuid, nature: Option<&str>) -> DocumentEmis {
        let (r, c) = (RenduPdfAttestation, Cachet);
        let m = Moteur { rendu: &r, cachet: &c, url_verification: "https://test.scolia.fr/verifier" };
        let e = self.ent.emettre_attestation_inscription_for_actor(&self.adm(), eleve, &m).await.expect("document émis");
        if let Some(n) = nature {
            self.preparer(
                &format!("UPDATE ent_doc_document SET nature_code = '{n}' WHERE etablissement_id = $1 AND id = $2"),
                &[self.m.etab, e.document_id],
            )
            .await;
        }
        e
    }

    async fn lancer(&self, a: &Acteur<'_>, doc: Uuid, titulaires: bool, autres: Vec<SignataireDemande>) -> Result<CircuitLance, EntDbError> {
        self.ent
            .lancer_circuit_for_actor(
                a,
                DemandeCircuit { document_id: doc, ordre: None, echeance: None, titulaires_de_l_eleve: titulaires, signataires: autres },
            )
            .await
    }

    async fn signataire_de(&self, circuit: Uuid, personne: Uuid) -> Uuid {
        let (id,): (Uuid,) = sqlx::query_as(
            "SELECT id FROM ent_doc_signataire WHERE etablissement_id = $1 AND circuit_id = $2 AND personne_id = $3 AND etat <> 'remplace'",
        )
        .bind(self.m.etab)
        .bind(circuit)
        .bind(personne)
        .fetch_one(self.ent.pool_verif())
        .await
        .unwrap();
        id
    }

    /// Signe par code SMS : demande, lit le code envoyé, signe.
    async fn signer_par_code(&self, sms: &Sms, a: &Acteur<'_>, sid: Uuid) -> Result<SignatureFaite, EntDbError> {
        let h = HorodatageAucun;
        let d = self.ent.demander_code_signature_for_actor(a, sms, sid).await?;
        self.ent.signer_code_for_actor(a, &self.ctx(&h), d.defi_id, sid, &sms.dernier_code()).await
    }

    async fn etat_circuit(&self, circuit: Uuid) -> String {
        let (e,): (String,) = sqlx::query_as("SELECT etat FROM ent_doc_circuit WHERE etablissement_id = $1 AND id = $2")
            .bind(self.m.etab)
            .bind(circuit)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        e
    }

    async fn nombre(&self, sql: &str) -> i64 {
        let (n,): (i64,) = sqlx::query_as(sql).bind(self.m.etab).fetch_one(self.ent.pool_verif()).await.unwrap();
        n
    }

    /// Enrôle une passkey de test pour un compte.
    async fn enroler(&self, user: Uuid) -> WebauthnAuthenticator<SoftPasskey> {
        let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
        let (token, options) = self.ent.passkey_register_start_for_actor(self.m.etab, user).await.expect("start enrôlement");
        let ccr: ent_auth::CreationChallengeResponse = serde_json::from_value(options).unwrap();
        let reg = dev.do_registration(origine(), ccr).unwrap();
        self.ent
            .passkey_register_finish_for_actor(self.m.etab, user, &token, &reg, Some("Clé".into()))
            .await
            .expect("enrôlement");
        dev
    }
}

fn est_refus(e: &EntDbError, attendu: RefusMetier) -> bool {
    matches!(e, EntDbError::Refus(r, _) if *r == attendu)
}

fn est_interdit(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Forbidden)
}

fn est_introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound))
}

// ───────────────────────── Règles d'accord, pièces, finalisation ─────────────────────────

#[tokio::test]
async fn t_regle_un_titulaire_code_sms_et_pieces() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let eleve = w.eleve("Dupont", "Laura").await;
    w.lier(eleve, &w.lily, true).await;
    w.lier(eleve, &w.tom, true).await;
    let doc = w.document(eleve, None).await;
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.expect("circuit lancé");
    assert_eq!(c.signataires.len(), 2, "les deux titulaires sont invités");
    let s_lily = w.signataire_de(c.circuit_id, w.lily.personne).await;
    let s_tom = w.signataire_de(c.circuit_id, w.tom.personne).await;
    // Un second circuit sur la même version est refusé tant que le premier est ouvert.
    let deux = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.unwrap_err();
    assert!(est_refus(&deux, RefusMetier::Conflit), "{deux:?}");
    // « Mes documents à signer » : chacun voit sa ligne, pas celle de l'autre.
    let mes = w.ent.mes_documents_a_signer_for_actor(&w.parent(&w.lily)).await.unwrap();
    assert_eq!(mes.len(), 1);
    assert_eq!(mes[0].signataire_id, s_lily);
    // Tom ne peut pas signer à la place de Lily.
    let r = w.signer_par_code(&sms, &w.parent(&w.tom), s_lily).await.unwrap_err();
    assert!(est_interdit(&r) || est_introuvable(&r), "{r:?}");
    // Lily signe par code : règle « un » → le circuit est complet ; Tom n'est plus bloquant, ligne retirée.
    let fait = w.signer_par_code(&sms, &w.parent(&w.lily), s_lily).await.expect("signature");
    assert_eq!(fait.niveau, Niveau::S1);
    assert!(fait.circuit_complet);
    assert_eq!(w.etat_circuit(c.circuit_id).await, "complet");
    let (etat_tom,): (String,) = sqlx::query_as("SELECT etat FROM ent_doc_signataire WHERE etablissement_id = $1 AND id = $2")
        .bind(w.m.etab)
        .bind(s_tom)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(etat_tom, "retire");
    // Une deuxième signature de Lily est impossible (une seule fois).
    assert!(w.signer_par_code(&sms, &w.parent(&w.lily), s_lily).await.is_err());
    // Pièces : PDF signé (original en préfixe + page de signatures) et dossier de preuve, tous deux cachetés.
    let lily = w.parent(&w.lily);
    let signe = w.ent.lire_piece_circuit_for_actor(&lily, c.circuit_id, PieceCircuit::PdfSigne).await.unwrap();
    let preuve = w.ent.lire_piece_circuit_for_actor(&lily, c.circuit_id, PieceCircuit::Preuve).await.unwrap();
    assert!(signe.starts_with(b"%PDF-") && String::from_utf8_lossy(&signe).contains("SCELLE-TEST"));
    assert!(preuve.starts_with(b"%PDF-") && String::from_utf8_lossy(&preuve).contains("SCELLE-TEST"));
    // Tom (non signataire de fait, ligne retirée) lit aussi le dossier : il est signataire du circuit (R-D31).
    assert!(w.ent.lire_piece_circuit_for_actor(&w.parent(&w.tom), c.circuit_id, PieceCircuit::Preuve).await.is_ok());
    // Le PDF signé garde l'original comme préfixe : le document signé n'a pas été modifié.
    let (cle,): (String,) = sqlx::query_as("SELECT cle_objet FROM ent_doc_version WHERE etablissement_id = $1 AND id = $2")
        .bind(w.m.etab)
        .bind(doc.version_id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    let original = w.ent.objet_get(&cle).await.unwrap();
    assert!(signe.starts_with(&original));
    // Le dossier de preuve dit qui, quand, quel mode, quelle empreinte (64 caractères).
    let (empreinte,): (String,) = sqlx::query_as("SELECT empreinte_sha256 FROM ent_doc_version WHERE etablissement_id = $1 AND id = $2")
        .bind(w.m.etab)
        .bind(doc.version_id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(empreinte.len(), 64);
    // La page de signatures est écrite en clair (polices standard) : l'empreinte complète y figure d'un seul tenant.
    let texte = String::from_utf8_lossy(&signe).into_owned();
    assert!(texte.contains(&empreinte), "empreinte complète imprimée sur la page de signatures");
    assert!(texte.contains("code_sms"));
    // Aucune ligne de la preuve ne porte le numéro complet du téléphone.
    assert!(!texte.contains("0612345678"));
    let rapport = w.ent.verifier_signatures_for_actor(&w.adm(), c.circuit_id).await.unwrap();
    assert!(rapport.chaine_valide && rapport.empreinte_version_conforme && rapport.liens_conformes);
    // Un document signé est immuable : on ne peut plus signer ni refuser, ni annuler le circuit.
    assert!(w.ent.refuser_signature_for_actor(&w.parent(&w.tom), s_tom, None).await.is_err());
    assert!(w.ent.annuler_circuit_for_actor(&w.adm(), c.circuit_id, "trop tard").await.is_err());
    // Le cachet en panne bloque la complétion et n'écrit rien (jamais de dossier non scellé).
}

#[tokio::test]
async fn t_cachet_en_panne_annule_la_derniere_signature() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let eleve = w.eleve("Garnier", "Paul").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.document(eleve, None).await;
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.unwrap();
    let s = w.signataire_de(c.circuit_id, w.lily.personne).await;
    let panne = CachetEnPanne;
    let h = HorodatageAucun;
    let ctx = ContexteSignature {
        moteur: Moteur { rendu: &RenduPdfAttestation, cachet: &panne, url_verification: "https://t/verifier" },
        horodatage: &h,
    };
    let lily = w.parent(&w.lily);
    let d = w.ent.demander_code_signature_for_actor(&lily, &sms, s).await.unwrap();
    let r = w.ent.signer_code_for_actor(&lily, &ctx, d.defi_id, s, &sms.dernier_code()).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
    assert_eq!(w.etat_circuit(c.circuit_id).await, "ouvert", "rien n'est écrit");
    assert_eq!(
        w.nombre("SELECT count(*) FROM ent_doc_signature WHERE etablissement_id = $1").await,
        0
    );
}

#[tokio::test]
async fn t_regle_tous_et_horodatage() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let eleve = w.eleve("Martin", "Jade").await;
    w.lier(eleve, &w.lily, true).await;
    w.lier(eleve, &w.tom, true).await;
    let doc = w.document(eleve, Some("inscriptions.finalisation_dossier")).await;
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.unwrap();
    assert!(c.horodater, "la finalisation d'inscription est horodatée par défaut");
    let s_lily = w.signataire_de(c.circuit_id, w.lily.personne).await;
    let s_tom = w.signataire_de(c.circuit_id, w.tom.personne).await;
    let f1 = w.signer_par_code(&sms, &w.parent(&w.lily), s_lily).await.unwrap();
    assert!(!f1.circuit_complet, "règle « tous » : un seul titulaire ne suffit pas");
    assert!(f1.avertissement.is_some(), "le code SMS est signalé faible pour cette nature");
    assert_eq!(w.etat_circuit(c.circuit_id).await, "ouvert");
    let f2 = w.signer_par_code(&sms, &w.parent(&w.tom), s_tom).await.unwrap();
    assert!(f2.circuit_complet);
    assert_eq!(w.etat_circuit(c.circuit_id).await, "complet");
    // Sans service branché : aucune ligne d'horodatage (désactivé par défaut), la preuve le dit.
    assert_eq!(w.nombre("SELECT count(*) FROM ent_doc_horodatage WHERE etablissement_id = $1").await, 0);
    // Un refus d'un titulaire sous la règle « tous » clôt le circuit.
    let eleve2 = w.eleve("Martin", "Louis").await;
    w.lier(eleve2, &w.lily, true).await;
    w.lier(eleve2, &w.tom, true).await;
    let doc2 = w.document(eleve2, Some("inscriptions.finalisation_dossier")).await;
    let c2 = w.lancer(&w.adm(), doc2.document_id, true, vec![]).await.unwrap();
    let s2 = w.signataire_de(c2.circuit_id, w.tom.personne).await;
    assert!(w.ent.refuser_signature_for_actor(&w.parent(&w.tom), s2, Some("pas d'accord")).await.unwrap());
    assert_eq!(w.etat_circuit(c2.circuit_id).await, "refuse");
}

#[tokio::test]
async fn t_mineur_ne_signe_pas_seul_majeur_si() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    // Un mineur seul sur une convention : refusé (RP1-10 généralisé).
    let mineur = w.eleve("Petit", "Hugo").await;
    w.lier(mineur, &w.lily, true).await;
    let doc = w.document(mineur, Some("stages.convention_stage")).await;
    let r = w.lancer(&w.adm(), doc.document_id, false, vec![SignataireDemande::eleve(mineur)]).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
    // Avec ses titulaires : l'élève cosigne (la nature l'exige) ; il lui faut un compte pour signer.
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.expect("titulaires + élève");
    let vue = w.ent.circuit_for_actor(&w.adm(), c.circuit_id).await.unwrap();
    assert_eq!(vue.signataires.iter().filter(|s| s.role == "eleve").count(), 1);
    assert_eq!(vue.signataires.iter().filter(|s| s.role == "titulaire").count(), 1);
    // Majeur : signe seul, ses parents ne sont pas sollicités.
    let majeur = w.eleve("Grand", "Emma").await;
    w.lier(majeur, &w.lily, true).await;
    let doc2 = w.document(majeur, None).await;
    w.preparer(
        "UPDATE ent_eleve SET date_naissance = DATE '2000-01-15' WHERE etablissement_id = $1 AND id = $2",
        &[w.m.etab, majeur],
    )
    .await;
    let c2 = w.lancer(&w.adm(), doc2.document_id, true, vec![]).await.expect("majeur");
    let v2 = w.ent.circuit_for_actor(&w.adm(), c2.circuit_id).await.unwrap();
    assert_eq!(v2.signataires.len(), 1);
    assert_eq!(v2.signataires[0].role, "eleve");
    let _ = sms;
    // Aucun titulaire en droit de signer : refusé (lien sans autorité parentale).
    let orphelin = w.eleve("Seul", "Ines").await;
    w.lier(orphelin, &w.tom, true).await;
    let doc3 = w.document(orphelin, None).await;
    w.suspendre(orphelin, &w.tom).await;
    let r = w.lancer(&w.adm(), doc3.document_id, true, vec![]).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
}

#[tokio::test]
async fn t_nouvelle_version_rend_le_circuit_obsolete() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let eleve = w.eleve("Roux", "Zoé").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.document(eleve, None).await;
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.unwrap();
    let s = w.signataire_de(c.circuit_id, w.lily.personne).await;
    // Lily a un défi en cours quand le document change.
    let lily = w.parent(&w.lily);
    let h = HorodatageAucun;
    let d = w.ent.demander_code_signature_for_actor(&lily, &sms, s).await.unwrap();
    let faux_rendu = RenduFaux;
    let m = Moteur { rendu: &faux_rendu, cachet: &w.cachet, url_verification: "https://t/verifier" };
    w.ent
        .reemettre_document_for_actor(
            &w.dir(),
            DemandeReemission {
                document_id: doc.document_id,
                motif: "classe corrigée".into(),
                donnees: vec![],
                destinataires: vec![DestinataireDoc {
                    personne_id: Some(w.lily.personne),
                    nom: Some("Lily".into()),
                    au_coffre: true,
                    ..Default::default()
                }],
                au_nom_de_agent_id: None,
            },
            &m,
        )
        .await
        .expect("réémission");
    assert_eq!(w.etat_circuit(c.circuit_id).await, "obsolete");
    // Le défi de l'ancienne version ne signe plus rien.
    let r = w.ent.signer_code_for_actor(&lily, &w.ctx(&h), d.defi_id, s, &sms.dernier_code()).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
    assert_eq!(w.nombre("SELECT count(*) FROM ent_doc_signature WHERE etablissement_id = $1").await, 0);
    let (motif,): (String,) = sqlx::query_as("SELECT clos_motif FROM ent_doc_circuit WHERE etablissement_id = $1 AND id = $2")
        .bind(w.m.etab)
        .bind(c.circuit_id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert!(motif.contains("nouvelle version"));
    // Un nouveau circuit sur la nouvelle version est possible et demande de nouvelles signatures.
    let c2 = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.expect("nouveau circuit");
    assert_ne!(c2.circuit_id, c.circuit_id);
    let (v_ancienne, v_nouvelle): (Uuid, Uuid) = sqlx::query_as(
        "SELECT (SELECT version_id FROM ent_doc_circuit WHERE etablissement_id = $1 AND id = $2),
                (SELECT version_id FROM ent_doc_circuit WHERE etablissement_id = $1 AND id = $3)",
    )
    .bind(w.m.etab)
    .bind(c.circuit_id)
    .bind(c2.circuit_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_ne!(v_ancienne, v_nouvelle);
    // L'annulation du document clôt aussi le circuit ouvert.
    w.ent.annuler_document_for_actor(&w.dir(), doc.document_id, "document annulé par erreur").await.unwrap();
    assert_eq!(w.etat_circuit(c2.circuit_id).await, "annule");
}

struct RenduFaux;

impl RenduDocument for RenduFaux {
    fn rendre(&self, d: &DemandeRendu<'_>) -> Result<DocumentRendu, ErreurRendu> {
        Ok(DocumentRendu { octets: format!("%PDF-1.4 TEST v{}", d.numero_version).into_bytes(), nb_pages: 1 })
    }
}

// ───────────────────────── Passkey ─────────────────────────

#[tokio::test]
async fn t_passkey_defi_lie_rejeu_refuse() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Leroy", "Mila").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.document(eleve, None).await;
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.unwrap();
    let s = w.signataire_de(c.circuit_id, w.lily.personne).await;
    let lily = w.parent(&w.lily);
    // Sans passkey enrôlée : repli proposé (refus explicite), jamais de signature.
    let r = w.ent.demarrer_signature_passkey_for_actor(&lily, s).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
    let mut dev = w.enroler(w.lily.user).await;
    let defi = w.ent.demarrer_signature_passkey_for_actor(&lily, s).await.expect("défi");
    assert_eq!(defi.empreinte.len(), 64);
    assert!(defi.intention.contains("responsable légal"));
    // L'intention est affichée avant signature ; ce qui est signé : l'empreinte de la version.
    let a_voir = w.ent.lire_version_a_signer_for_actor(&lily, s).await.unwrap();
    assert_eq!(sha256_hex(&a_voir), defi.empreinte);
    // Un autre compte ne peut pas terminer le défi de Lily.
    let rcr: ent_auth::RequestChallengeResponse = serde_json::from_value(defi.options.clone()).unwrap();
    let assertion = dev.do_authentication(origine(), rcr.clone()).unwrap();
    let h = HorodatageAucun;
    let ctx = w.ctx(&h);
    let r = w.ent.signer_passkey_for_actor(&w.parent(&w.tom), &ctx, defi.defi_id, &assertion).await.unwrap_err();
    assert!(est_introuvable(&r) || est_interdit(&r), "{r:?}");
    // Lily termine : niveau S2, complet (règle « un »).
    let fait = w.ent.signer_passkey_for_actor(&lily, &ctx, defi.defi_id, &assertion).await.expect("signature passkey");
    assert_eq!(fait.niveau, Niveau::S2);
    assert!(fait.circuit_complet);
    // Rejeu : même défi, même assertion → refusé (défi consommé) ; aucune seconde signature.
    let r = w.ent.signer_passkey_for_actor(&lily, &ctx, defi.defi_id, &assertion).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
    assert_eq!(w.nombre("SELECT count(*) FROM ent_doc_signature WHERE etablissement_id = $1").await, 1);
    // Le lien défi ↔ empreinte ↔ intention ↔ signataire ↔ nonce est recalculable (dossier vérifiable).
    let rapport = w.ent.verifier_signatures_for_actor(&w.adm(), c.circuit_id).await.unwrap();
    assert!(rapport.chaine_valide && rapport.liens_conformes && rapport.empreinte_version_conforme, "{rapport:?}");
    // Les éléments bruts de l'assertion sont conservés pour expertise (défi dans clientDataJSON).
    let (client, defi_web): (String, String) = sqlx::query_as(
        "SELECT g.client_data_json, d.defi_webauthn FROM ent_doc_signature g
           JOIN ent_doc_defi d ON d.etablissement_id = g.etablissement_id AND d.id = g.defi_id
          WHERE g.etablissement_id = $1",
    )
    .bind(w.m.etab)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    let decode = |s: &str| -> String {
        // clientDataJSON est en base64url : on cherche le défi dans sa version décodée à la main.
        const A: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut bits = 0u32;
        let mut n = 0;
        let mut out = Vec::new();
        for c in s.trim_end_matches('=').chars() {
            bits = (bits << 6) | A.find(c).unwrap() as u32;
            n += 6;
            if n >= 8 {
                n -= 8;
                out.push(((bits >> n) & 0xff) as u8);
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    };
    assert!(decode(&client).contains(&defi_web), "le défi signé est celui que le serveur avait lié");
}

// ───────────────────────── Code SMS ─────────────────────────

#[tokio::test]
async fn t_code_essais_expiration_plafond() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let h = HorodatageAucun;
    let eleve = w.eleve("Blanc", "Nina").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.document(eleve, None).await;
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.unwrap();
    let s = w.signataire_de(c.circuit_id, w.lily.personne).await;
    let lily = w.parent(&w.lily);
    let d = w.ent.demander_code_signature_for_actor(&lily, &sms, s).await.unwrap();
    assert_eq!(d.canal_masque, "06 ** ** ** 78", "le numéro n'est jamais montré en entier");
    let bon = sms.dernier_code();
    let mauvais = if bon == "000000" { "000001" } else { "000000" };
    // Cinq mauvais essais brûlent le code ; le bon code ne passe plus.
    for _ in 0..5 {
        let r = w.ent.signer_code_for_actor(&lily, &w.ctx(&h), d.defi_id, s, mauvais).await.unwrap_err();
        assert!(est_refus(&r, RefusMetier::Conflit));
    }
    let r = w.ent.signer_code_for_actor(&lily, &w.ctx(&h), d.defi_id, s, &bon).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "trop d'essais : {r:?}");
    // Un nouveau code ; expiré (instant écrit dans le passé) → refusé.
    let d2 = w.ent.demander_code_signature_for_actor(&lily, &sms, s).await.unwrap();
    let bon2 = sms.dernier_code();
    w.preparer(
        "UPDATE ent_doc_defi SET expire_le = now() - interval '1 minute' WHERE etablissement_id = $1 AND id = $2",
        &[w.m.etab, d2.defi_id],
    )
    .await;
    let r = w.ent.signer_code_for_actor(&lily, &w.ctx(&h), d2.defi_id, s, &bon2).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
    // Plafond d'envois : trois par heure.
    let r = w.ent.demander_code_signature_for_actor(&lily, &sms, s).await.unwrap();
    let _ = r;
    let r = w.ent.demander_code_signature_for_actor(&lily, &sms, s).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "plafond : {r:?}");
    assert_eq!(sms.nombre(), 3);
    // Le dernier code (valide) signe ; le code n'est jamais en clair en base.
    let (n_clair,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_defi WHERE etablissement_id = $1 AND code_sha256 = $2",
    )
    .bind(w.m.etab)
    .bind(sms.dernier_code())
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n_clair, 0);
    // Un défi consommé ne se rejoue pas (trigger de la base).
    let derniers: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_doc_defi WHERE etablissement_id = $1 AND methode = 'code_sms' ORDER BY cree_le DESC LIMIT 1")
        .bind(w.m.etab)
        .fetch_all(w.ent.pool_verif())
        .await
        .unwrap();
    let ok = w.ent.signer_code_for_actor(&lily, &w.ctx(&h), derniers[0].0, s, &sms.dernier_code()).await.expect("dernier code valide");
    assert!(ok.circuit_complet);
    let rejeu = sqlx::query("UPDATE ent_doc_defi SET consomme_le = NULL WHERE etablissement_id = $1 AND id = $2")
        .bind(w.m.etab)
        .bind(derniers[0].0)
        .execute(w.ent.pool_verif())
        .await;
    assert!(rejeu.is_err(), "un défi consommé ne se rouvre pas");
}

// ───────────────────────── Guichet ─────────────────────────

fn png() -> Vec<u8> {
    let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
    v.extend_from_slice(&[0u8; 40]);
    v
}

#[tokio::test]
async fn t_guichet_confirmation_ou_simple() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let h = HorodatageAucun;
    let eleve = w.eleve("Morel", "Jules").await;
    w.lier(eleve, &w.lily, true).await;
    w.lier(eleve, &w.tom, true).await;
    let doc = w.document(eleve, Some("discipline.convocation")).await;
    // Convocation disciplinaire : le code SMS est bloqué (trop faible) dès le lancement.
    let r = w
        .lancer(
            &w.adm(),
            doc.document_id,
            false,
            vec![{
                let mut s = SignataireDemande::personne(RoleSignataire::Titulaire, w.lily.personne);
                s.modes = Some(vec![ModeSignature::CodeSms]);
                s
            }],
        )
        .await
        .unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
    // Remise en main propre (défaut de la nature) pour les deux titulaires, l'émetteur fixe le mode.
    let mk = |p: Uuid| {
        let mut s = SignataireDemande::personne(RoleSignataire::Titulaire, p);
        s.modes = Some(vec![ModeSignature::Guichet]);
        s.choix_par = ChoixPar::Emetteur;
        s
    };
    let c = w.lancer(&w.adm(), doc.document_id, false, vec![mk(w.lily.personne), mk(w.tom.personne)]).await.unwrap();
    let s_lily = w.signataire_de(c.circuit_id, w.lily.personne).await;
    let s_tom = w.signataire_de(c.circuit_id, w.tom.personne).await;
    let adm = w.adm();
    // Droits : un professeur ne constate pas ; type de pièce inconnu refusé ; tracé non PNG refusé.
    let constat = |sid: Uuid, confirmation: Option<(Uuid, String)>| ConstatGuichet {
        signataire_id: sid,
        identite_vue: "carte_identite".into(),
        trace: Some(png()),
        confirmation,
    };
    let r = w.ent.constater_guichet_for_actor(&w.prof(), &w.ctx(&h), constat(s_lily, None)).await.unwrap_err();
    assert!(est_interdit(&r), "{r:?}");
    let mut mauvais = constat(s_lily, None);
    mauvais.identite_vue = "selfie".into();
    assert!(w.ent.constater_guichet_for_actor(&adm, &w.ctx(&h), mauvais).await.is_err());
    let mut pas_png = constat(s_lily, None);
    pas_png.trace = Some(b"GIF89a".to_vec());
    assert!(w.ent.constater_guichet_for_actor(&adm, &w.ctx(&h), pas_png).await.is_err());
    // Lily : confirmation par code envoyé à SON téléphone ; une mauvaise saisie ne signe pas, la bonne donne G.
    let d = w.ent.demarrer_confirmation_guichet_for_actor(&adm, &sms, s_lily).await.unwrap();
    let bon = sms.dernier_code();
    let mauvais = if bon == "111111" { "222222" } else { "111111" };
    let r = w.ent.constater_guichet_for_actor(&adm, &w.ctx(&h), constat(s_lily, Some((d.defi_id, mauvais.into())))).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
    let f = w.ent.constater_guichet_for_actor(&adm, &w.ctx(&h), constat(s_lily, Some((d.defi_id, bon)))).await.expect("niveau G");
    assert_eq!(f.niveau, Niveau::G);
    // Tom : tablette sans confirmation → signature simple, affichée comme telle (G0).
    let f2 = w.ent.constater_guichet_for_actor(&adm, &w.ctx(&h), constat(s_tom, None)).await.expect("niveau G0");
    assert_eq!(f2.niveau, Niveau::G0);
    assert!(f2.avertissement.unwrap().contains("simple"));
    assert!(f2.circuit_complet, "règle « tous » satisfaite");
    // La preuve garde l'agent, le type de pièce vu (jamais une copie) et le tracé scellé.
    let (agent, vue, trace): (Uuid, String, Option<String>) = sqlx::query_as(
        "SELECT constate_par_user_id, identite_vue, trace_cle_objet FROM ent_doc_signature
          WHERE etablissement_id = $1 AND niveau = 'G'",
    )
    .bind(w.m.etab)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(agent, adm.user_id);
    assert_eq!(vue, "carte_identite");
    assert!(trace.is_some());
}

// ───────────────────────── Papier ─────────────────────────

#[tokio::test]
async fn t_papier_depot_et_controle() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let h = HorodatageAucun;
    let eleve = w.eleve("Faure", "Elio").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.document(eleve, None).await;
    let mut demande = SignataireDemande::personne(RoleSignataire::Titulaire, w.lily.personne);
    demande.modes = Some(vec![ModeSignature::Papier]);
    demande.choix_par = ChoixPar::Emetteur;
    let c = w.lancer(&w.adm(), doc.document_id, false, vec![demande]).await.unwrap();
    let s = w.signataire_de(c.circuit_id, w.lily.personne).await;
    let scan = {
        let mut d = ent_pdf::Document::new(ent_pdf::Meta::default());
        d.titre("Scan");
        d.octets()
    };
    // Dépôt par la famille elle-même : refusé (le papier est déposé par l'établissement).
    assert!(est_interdit(&w.ent.deposer_papier_for_actor(&w.parent(&w.lily), s, &scan).await.unwrap_err()));
    // Fichier qui n'est ni PDF ni image : refusé.
    assert!(w.ent.deposer_papier_for_actor(&w.adm(), s, b"MZ\x90\x00 programme").await.is_err());
    w.ent.deposer_papier_for_actor(&w.adm(), s, &scan).await.expect("scan déposé");
    // Jusqu'au contrôle : « déposé, à valider », circuit ouvert, rien d'une signature.
    assert_eq!(w.etat_circuit(c.circuit_id).await, "ouvert");
    assert_eq!(w.nombre("SELECT count(*) FROM ent_doc_signature WHERE etablissement_id = $1").await, 0);
    // Refus du contrôle : motif obligatoire, le signataire redevient « invité ».
    assert!(w.ent.controler_papier_for_actor(&w.adm(), &w.ctx(&h), s, DecisionPapier::Refuser("x".into())).await.is_err());
    w.ent.controler_papier_for_actor(&w.adm(), &w.ctx(&h), s, DecisionPapier::Refuser("signature absente".into())).await.unwrap();
    w.ent.deposer_papier_for_actor(&w.adm(), s, &scan).await.expect("nouveau scan");
    // Seul le personnel lit le scan ; la famille jamais.
    assert!(w.ent.lire_papier_for_actor(&w.parent(&w.lily), s).await.is_err());
    assert!(w.ent.lire_papier_for_actor(&w.adm(), s).await.unwrap().starts_with(b"%PDF-"));
    let f = w
        .ent
        .controler_papier_for_actor(&w.adm(), &w.ctx(&h), s, DecisionPapier::Valider)
        .await
        .unwrap()
        .expect("signature papier");
    assert_eq!(f.niveau, Niveau::P);
    assert!(f.circuit_complet);
}

// ───────────────────────── Menu et réglages ─────────────────────────

#[tokio::test]
async fn t_menu_bloque_signale_et_reglable() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let dir = w.dir();
    // Réglage : réservé à `documents.modeles.parametrer` (la direction ; pas le secrétariat).
    let r = w
        .ent
        .parametrer_signature_nature_for_actor(&w.adm(), "discipline.convocation", None, Some(false))
        .await
        .unwrap_err();
    assert!(est_interdit(&r));
    // Nature inconnue de la brique : fermé.
    assert!(w.ent.reglage_signature_for_actor(&dir, "voyages.inconnue").await.is_err());
    // Menu par défaut de la convocation : guichet d'abord ; passkey signalé ; code SMS absent.
    let v = w.ent.reglage_signature_for_actor(&dir, "discipline.convocation").await.unwrap();
    assert_eq!(v.menu[0].mode, ModeSignature::Guichet);
    assert!(v.menu[0].defaut);
    assert!(v.menu.iter().all(|c| c.mode != ModeSignature::CodeSms));
    assert!(v.horodatage_actif);
    // Un menu qui contient un mode bloqué est refusé.
    let r = w
        .ent
        .parametrer_signature_nature_for_actor(
            &dir,
            "discipline.convocation",
            Some(ReglageMenu { modes: vec![ModeSignature::CodeSms, ModeSignature::Papier], defaut: ModeSignature::Papier }),
            None,
        )
        .await
        .unwrap_err();
    assert!(est_refus(&r, RefusMetier::Invalide), "{r:?}");
    // Réglage valable : papier seul, horodatage coupé.
    w.ent
        .parametrer_signature_nature_for_actor(
            &dir,
            "discipline.convocation",
            Some(ReglageMenu { modes: vec![ModeSignature::Papier], defaut: ModeSignature::Papier }),
            Some(false),
        )
        .await
        .unwrap();
    let v = w.ent.reglage_signature_for_actor(&dir, "discipline.convocation").await.unwrap();
    assert_eq!(v.menu.len(), 1);
    assert!(!v.horodatage_actif);
    // Un seul menu par établissement et par nature : le réglage ne fuit pas vers un autre établissement.
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_doc_signature_menu WHERE etablissement_id <> $1 AND nature_code = 'discipline.convocation' AND mode = 'papier' AND defaut AND maj_par = $2")
        .bind(w.m.etab)
        .bind(dir.user_id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 0);
    // Le lancement suit le menu réglé : le mode « guichet » n'est plus au menu.
    let eleve = w.eleve("Henry", "Lou").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.document(eleve, Some("discipline.convocation")).await;
    let mut s = SignataireDemande::personne(RoleSignataire::Titulaire, w.lily.personne);
    s.modes = Some(vec![ModeSignature::Guichet]);
    s.choix_par = ChoixPar::Emetteur;
    let r = w.lancer(&w.adm(), doc.document_id, false, vec![s]).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Invalide) || est_refus(&r, RefusMetier::Conflit), "{r:?}");
    // 4 modes au plus par menu (la base le garde aussi).
    let r = sqlx::query("INSERT INTO ent_doc_signature_menu (etablissement_id, nature_code, mode, rang) VALUES ($1, 'rh.contrat_travail', 'papier', 5)")
        .bind(w.m.etab)
        .execute(w.ent.pool_verif())
        .await;
    assert!(r.is_err());
}

// ───────────────────────── Horodatage ─────────────────────────

#[tokio::test]
async fn t_horodatage_port_releve_et_reprise() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let dir = w.dir();
    // Adresse non HTTPS : refusée.
    let r = w
        .ent
        .parametrer_service_horodatage_for_actor(
            &dir,
            ModeHorodatage::Propre,
            true,
            Some(ConfigServiceHorodatage { url: "http://tsa.exemple".into(), identifiant: None, secret: None }),
        )
        .await
        .unwrap_err();
    assert!(est_refus(&r, RefusMetier::Invalide), "{r:?}");
    w.ent
        .parametrer_service_horodatage_for_actor(
            &dir,
            ModeHorodatage::Propre,
            true,
            Some(ConfigServiceHorodatage { url: "https://tsa.exemple/tsr".into(), identifiant: Some("compte".into()), secret: None }),
        )
        .await
        .unwrap();
    let faux = FauxTsa { appels: Arc::new(AtomicUsize::new(0)), echecs: Arc::new(AtomicUsize::new(1)) };
    let eleve = w.eleve("Perrin", "Alix").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.document(eleve, Some("inscriptions.finalisation_dossier")).await;
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.unwrap();
    let s = w.signataire_de(c.circuit_id, w.lily.personne).await;
    let lily = w.parent(&w.lily);
    let d = w.ent.demander_code_signature_for_actor(&lily, &sms, s).await.unwrap();
    let ctx = w.ctx(&faux);
    // Le titulaire unique signe : le circuit se complète ; le service est en panne → demande « en attente ».
    let f = w.ent.signer_code_for_actor(&lily, &ctx, d.defi_id, s, &sms.dernier_code()).await.unwrap();
    assert!(f.circuit_complet);
    let (etat,): (String,) = sqlx::query_as("SELECT etat FROM ent_doc_horodatage WHERE etablissement_id = $1 AND circuit_id = $2")
        .bind(w.m.etab)
        .bind(c.circuit_id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(etat, "en_attente");
    assert_eq!(releve_du_jour(&w).await, ReleveHorodatage { mutualise_obtenus: 0, propre_obtenus: 0, en_attente: 1, echecs: 0 });
    // Reprise (tâche de fond) : le jeton est obtenu, le relevé compte un jeton « propre ».
    assert_eq!(w.ent.horodater_en_attente(w.m.etab, &faux, 10).await.unwrap(), 1);
    assert_eq!(releve_du_jour(&w).await, ReleveHorodatage { mutualise_obtenus: 0, propre_obtenus: 1, en_attente: 0, echecs: 0 });
    assert_eq!(faux.appels.load(Ordering::SeqCst), 2);
    // Un jeton obtenu ne bouge plus (trigger) ; une seconde reprise n'appelle plus le service.
    assert_eq!(w.ent.horodater_en_attente(w.m.etab, &faux, 10).await.unwrap(), 0);
    assert_eq!(faux.appels.load(Ordering::SeqCst), 2);
    let r = sqlx::query("UPDATE ent_doc_horodatage SET etat = 'echec', jeton_hex = NULL, obtenu_le = NULL WHERE etablissement_id = $1")
        .bind(w.m.etab)
        .execute(w.ent.pool_verif())
        .await;
    assert!(r.is_err());
    // Relevé réservé à la direction (modèles.parametrer).
    assert!(w.ent.releve_horodatage_for_actor(&w.adm(), w.jour, w.jour).await.is_err());
    // Nature non horodatée (autorisation) : aucun jeton demandé même service actif.
    let eleve2 = w.eleve("Perrin", "Noa").await;
    w.lier(eleve2, &w.lily, true).await;
    let doc2 = w.document(eleve2, Some("voyages.autorisation_parentale")).await;
    let c2 = w.lancer(&w.adm(), doc2.document_id, true, vec![]).await.unwrap();
    assert!(!c2.horodater);
}

async fn releve_du_jour(w: &W) -> ReleveHorodatage {
    let (du, au) = (w.jour - chrono::Duration::days(1), w.jour + chrono::Duration::days(1));
    w.ent.releve_horodatage_for_actor(&w.dir(), du, au).await.unwrap()
}

// ───────────────────────── Parapheur et externe ─────────────────────────

#[tokio::test]
async fn t_parapheur_et_externe() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let h = HorodatageAucun;
    // La direction (agent) signe au parapheur : deux circuits, triés par échéance.
    let dir_fiche = fiche(&w.ent, w.m.etab, w.cast.direction.user_id, "direction", true).await;
    let agent = dir_fiche.agent.expect("fiche d'agent de la direction");
    w.mobile(dir_fiche.personne, "0611223344").await;
    let e1 = w.eleve("Aubert", "Léo").await;
    let e2 = w.eleve("Aubert", "Mia").await;
    w.lier(e1, &w.lily, true).await;
    w.lier(e2, &w.lily, true).await;
    let d1 = w.document(e1, Some("stages.convention_stage")).await;
    let d2 = w.document(e2, Some("rh.contrat_travail")).await;
    let lance = |doc: Uuid, jours: i64, s: SignataireDemande| {
        let w = &w;
        async move {
            w.ent
                .lancer_circuit_for_actor(
                    &w.adm(),
                    DemandeCircuit {
                        document_id: doc,
                        ordre: Some(Ordre::Parallele),
                        echeance: Some(w.jour + chrono::Duration::days(jours)),
                        titulaires_de_l_eleve: false,
                        signataires: vec![s],
                    },
                )
                .await
                .unwrap()
        }
    };
    let mut sa = SignataireDemande::agent(RoleSignataire::Agent, agent);
    sa.modes = Some(vec![ModeSignature::Papier, ModeSignature::Passkey]);
    let mut sb = SignataireDemande::agent(RoleSignataire::Agent, agent);
    sb.modes = Some(vec![ModeSignature::Passkey]);
    let loin = lance(d1.document_id, 20, {
        let mut s = SignataireDemande::agent(RoleSignataire::Agent, agent);
        s.modes = Some(vec![ModeSignature::Passkey, ModeSignature::Papier]);
        s
    })
    .await;
    let proche = lance(d2.document_id, 2, sb).await;
    let _ = sa;
    let dir = w.dir();
    let para = w.ent.parapheur_for_actor(&dir).await.unwrap();
    assert_eq!(para.len(), 2);
    assert_eq!(para[0].circuit_id, proche.circuit_id, "l'échéance la plus proche d'abord");
    assert_eq!(para[1].circuit_id, loin.circuit_id);
    // Le parapheur n'est pas « mes documents à signer » de la famille ; il est propre à l'agent.
    assert!(w.ent.parapheur_for_actor(&w.parent(&w.lily)).await.unwrap().is_empty());
    assert!(w.ent.mes_documents_a_signer_for_actor(&dir).await.unwrap().is_empty());
    // Mode non autorisé pour ce signataire (code SMS) : refusé ; la signature se fait à l'unité par passkey.
    let r = w.ent.demander_code_signature_for_actor(&dir, &sms, para[0].signataire_id).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
    let _ = &h;

    // Signataire externe : lien + code ; le tuteur choisit lui-même son mode parmi ceux de la nature.
    let e3 = w.eleve("Aubert", "Tom").await;
    w.lier(e3, &w.lily, true).await;
    let d3 = w.document(e3, Some("stages.convention_stage")).await;
    let ext = SignataireDemande::externe(Externe {
        nom: "M. Tuteur".into(),
        organisation: Some("Atelier du Port".into()),
        telephone: "0699887766".into(),
    });
    let c = w.lancer(&w.adm(), d3.document_id, true, vec![ext]).await.expect("circuit avec externe");
    assert_eq!(c.liens_externes.len(), 1);
    let (sid_ext, jeton) = c.liens_externes[0].clone();
    let vue = w.ent.circuit_for_actor(&w.adm(), c.circuit_id).await.unwrap();
    let ligne = vue.signataires.iter().find(|s| s.id == sid_ext).unwrap();
    assert_eq!(ligne.choix_par, "signataire", "l'externe choisit son mode");
    assert!(ligne.modes.contains(&"papier".to_string()) && ligne.modes.contains(&"code_sms".to_string()));
    // Le jeton n'est jamais stocké en clair, ni le téléphone.
    let (clair,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_signataire WHERE etablissement_id = $1 AND (lien_jeton_sha256 = $2 OR externe_contact_scelle = '0699887766')",
    )
    .bind(w.m.etab)
    .bind(&jeton)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(clair, 0);
    // Lien inconnu ou mal formé : même réponse « introuvable ».
    assert!(est_introuvable(&w.ent.externe_demander_code(w.m.etab, &"0".repeat(64), &sms).await.unwrap_err()));
    assert!(est_introuvable(&w.ent.externe_demander_code(w.m.etab, "court", &sms).await.unwrap_err()));
    let (sid, defi) = w.ent.externe_demander_code(w.m.etab, &jeton, &sms).await.expect("code envoyé");
    assert_eq!(sid, sid_ext);
    assert_eq!(defi.canal_masque, "06 ** ** ** 66");
    let faux = w.ent.externe_signer_code(w.m.etab, &w.ctx(&h), &jeton, defi.defi_id, "000000").await;
    assert!(faux.is_err() || sms.dernier_code() == "000000");
    let f = w.ent.externe_signer_code(w.m.etab, &w.ctx(&h), &jeton, defi.defi_id, &sms.dernier_code()).await.expect("l'externe signe");
    assert_eq!(f.niveau, Niveau::S1);
    // Séquentiel (convention) : l'externe a signé ; il reste le titulaire et l'élève (qui n'a pas de compte ici).
    assert!(!f.circuit_complet);
}

// ───────────────────────── Chaîne et vérification ─────────────────────────

#[tokio::test]
async fn t_chaine_ajout_seul_et_verification() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let eleve = w.eleve("Collin", "Eva").await;
    w.lier(eleve, &w.lily, true).await;
    w.lier(eleve, &w.tom, true).await;
    let doc = w.document(eleve, Some("inscriptions.finalisation_dossier")).await;
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.unwrap();
    let s_lily = w.signataire_de(c.circuit_id, w.lily.personne).await;
    let s_tom = w.signataire_de(c.circuit_id, w.tom.personne).await;
    w.signer_par_code(&sms, &w.parent(&w.lily), s_lily).await.unwrap();
    w.signer_par_code(&sms, &w.parent(&w.tom), s_tom).await.unwrap();
    // Deux entrées, la seconde scelle la première ; la tête est celle du circuit.
    let lignes: Vec<(i32, String, String)> = sqlx::query_as(
        "SELECT rang_chaine, precedent_sha256, entree_sha256 FROM ent_doc_signature WHERE etablissement_id = $1 ORDER BY rang_chaine",
    )
    .bind(w.m.etab)
    .fetch_all(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(lignes.len(), 2);
    assert_eq!(lignes[0].1, tete_initiale(c.circuit_id));
    assert_eq!(lignes[1].1, lignes[0].2);
    let (tete,): (String,) = sqlx::query_as("SELECT tete_chaine_sha256 FROM ent_doc_circuit WHERE etablissement_id = $1 AND id = $2")
        .bind(w.m.etab)
        .bind(c.circuit_id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(tete, lignes[1].2);
    let rapport = w.ent.verifier_signatures_for_actor(&w.adm(), c.circuit_id).await.unwrap();
    assert!(rapport.chaine_valide && rapport.nb_signatures == 2 && rapport.tete_chaine.as_deref() == Some(tete.as_str()));
    // Ajout seul : modifier ou supprimer une signature est refusé par la base.
    let pool = w.ent.pool_verif();
    assert!(sqlx::query("UPDATE ent_doc_signature SET niveau = 'S2' WHERE etablissement_id = $1").bind(w.m.etab).execute(pool).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_doc_signature WHERE etablissement_id = $1").bind(w.m.etab).execute(pool).await.is_err());
    // Circuit complet : plus aucune modification, plus de suppression.
    assert!(sqlx::query("UPDATE ent_doc_circuit SET echeance = echeance + 1 WHERE etablissement_id = $1 AND id = $2")
        .bind(w.m.etab)
        .bind(c.circuit_id)
        .execute(pool)
        .await
        .is_err());
    assert!(sqlx::query("DELETE FROM ent_doc_circuit WHERE etablissement_id = $1").bind(w.m.etab).execute(pool).await.is_err());
    // Une entrée forgée (rang sauté, empreinte précédente inventée) est refusée à l'insertion.
    let forge = sqlx::query(
        "INSERT INTO ent_doc_signature (id, etablissement_id, circuit_id, signataire_id, version_id, rang_chaine, empreinte_signee, methode, niveau,
              intention_texte, intention_sha256, horodatage_serveur, code_canal_masque, defi_id, precedent_sha256, entree_sha256)
         SELECT gen_random_uuid(), etablissement_id, circuit_id, signataire_id, version_id, 9, empreinte_signee, 'code_sms', 'S1',
                intention_texte, intention_sha256, now(), 'x', defi_id, repeat('a', 64), repeat('b', 64)
           FROM ent_doc_signature WHERE etablissement_id = $1 LIMIT 1",
    )
    .bind(w.m.etab)
    .execute(pool)
    .await;
    assert!(forge.is_err());
}

#[tokio::test]
async fn t_verification_par_depot_du_pdf_signe() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let eleve = w.eleve("Renard", "Sam").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.document(eleve, None).await;
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.unwrap();
    let s = w.signataire_de(c.circuit_id, w.lily.personne).await;
    w.signer_par_code(&sms, &w.parent(&w.lily), s).await.unwrap();
    let signe = w.ent.lire_piece_circuit_for_actor(&w.adm(), c.circuit_id, PieceCircuit::PdfSigne).await.unwrap();
    let preuve = w.ent.lire_piece_circuit_for_actor(&w.adm(), c.circuit_id, PieceCircuit::Preuve).await.unwrap();
    // À partir du PDF signé seul : « identique », nombre de signatures, aucun nom.
    match w.ent.verifier_par_depot(w.m.etab, &signe).await.unwrap() {
        crate::documents_natures::ReponseDepot::IdentiqueSigne { nb_signatures, dossier_de_preuve, .. } => {
            assert_eq!(nb_signatures, 1);
            assert!(!dossier_de_preuve);
        }
        autre => panic!("PDF signé non reconnu : {autre:?}"),
    }
    assert!(matches!(
        w.ent.verifier_par_depot(w.m.etab, &preuve).await.unwrap(),
        crate::documents_natures::ReponseDepot::IdentiqueSigne { dossier_de_preuve: true, .. }
    ));
    // Un octet de différence : « différent ».
    let mut altere = signe.clone();
    let n = altere.len() / 2;
    altere[n] ^= 1;
    assert_eq!(w.ent.verifier_par_depot(w.m.etab, &altere).await.unwrap(), crate::documents_natures::ReponseDepot::Different);
    // Et l'original (non signé) reste reconnu comme l'original émis, pas comme un PDF signé.
    let (cle,): (String,) = sqlx::query_as("SELECT cle_objet FROM ent_doc_version WHERE etablissement_id = $1 AND id = $2")
        .bind(w.m.etab)
        .bind(doc.version_id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    let original = w.ent.objet_get(&cle).await.unwrap();
    assert!(matches!(
        w.ent.verifier_par_depot(w.m.etab, &original).await.unwrap(),
        crate::documents_natures::ReponseDepot::Identique { .. }
    ));
}

// ───────────────────────── Droits ─────────────────────────

#[tokio::test]
async fn t_droits_et_lien_suspendu() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let eleve = w.eleve("Lopez", "Ana").await;
    w.lier(eleve, &w.lily, true).await;
    w.lier(eleve, &w.tom, true).await;
    let doc = w.document(eleve, Some("inscriptions.finalisation_dossier")).await;
    // Lancer : ni la famille ni un professeur.
    for a in [w.parent(&w.lily), w.prof()] {
        let r = w.lancer(&a, doc.document_id, true, vec![]).await.unwrap_err();
        assert!(est_interdit(&r), "{:?} : {r:?}", a.role);
    }
    // Nature qui ne se signe pas (certificat) : refusée ; document inexistant : introuvable.
    let cert = w.ent.emettre_certificat_scolarite_for_actor(&w.adm(), eleve, &Moteur { rendu: &RenduPdfAttestation, cachet: &w.cachet, url_verification: "https://t/verifier" }).await.unwrap();
    assert!(est_refus(&w.lancer(&w.adm(), cert.document_id, true, vec![]).await.unwrap_err(), RefusMetier::Invalide));
    assert!(est_introuvable(&w.lancer(&w.adm(), Uuid::new_v4(), true, vec![]).await.unwrap_err()));
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.unwrap();
    let s_lily = w.signataire_de(c.circuit_id, w.lily.personne).await;
    let s_tom = w.signataire_de(c.circuit_id, w.tom.personne).await;
    // Un signataire n'agit que sur SA ligne : Tom ne refuse ni ne lit pour Lily.
    assert!(est_interdit(&w.ent.refuser_signature_for_actor(&w.parent(&w.tom), s_lily, None).await.unwrap_err()));
    assert!(w.ent.lire_version_a_signer_for_actor(&w.parent(&w.tom), s_lily).await.is_err());
    // Ni suivre (la famille), ni annuler, ni prolonger, ni remplacer.
    assert!(w.ent.circuit_for_actor(&w.prof(), c.circuit_id).await.is_err());
    assert!(est_interdit(&w.ent.annuler_circuit_for_actor(&w.parent(&w.lily), c.circuit_id, "je veux").await.unwrap_err()));
    assert!(est_interdit(&w.ent.prolonger_circuit_for_actor(&w.prof(), c.circuit_id, w.jour + chrono::Duration::days(90)).await.unwrap_err()));
    // Prolonger : date postérieure à l'ancienne seulement.
    assert!(w.ent.prolonger_circuit_for_actor(&w.adm(), c.circuit_id, w.jour + chrono::Duration::days(1)).await.is_err());
    w.ent.prolonger_circuit_for_actor(&w.adm(), c.circuit_id, w.jour + chrono::Duration::days(90)).await.unwrap();
    // Le lien de Tom est suspendu : sa signature est refusée (droit relu à chaque signature) et sa ligne disparaît.
    w.suspendre(eleve, &w.tom).await;
    assert!(w.ent.mes_documents_a_signer_for_actor(&w.parent(&w.tom)).await.unwrap().is_empty());
    let r = w.signer_par_code(&sms, &w.parent(&w.tom), s_tom).await.unwrap_err();
    assert!(est_refus(&r, RefusMetier::Conflit), "{r:?}");
    // Remplacer un signataire : motif obligatoire, jamais par lui-même ; l'ancien est « remplacé ».
    assert!(w.ent.remplacer_signataire_for_actor(&w.adm(), s_tom, SignataireDemande::personne(RoleSignataire::Titulaire, w.tom.personne), "x").await.is_err());
    // Annuler : motif obligatoire ; les signataires sont retirés.
    assert!(w.ent.annuler_circuit_for_actor(&w.adm(), c.circuit_id, "x").await.is_err());
    w.ent.annuler_circuit_for_actor(&w.adm(), c.circuit_id, "dossier repris à zéro").await.unwrap();
    assert_eq!(w.etat_circuit(c.circuit_id).await, "annule");
    assert!(w.signer_par_code(&sms, &w.parent(&w.lily), s_lily).await.is_err());
    // Échéance : un circuit échu expire (jour passé par l'appelant, jamais lu ici).
    let doc2 = w.document(eleve, Some("voyages.autorisation_parentale")).await;
    let c2 = w.lancer(&w.adm(), doc2.document_id, false, vec![{
        let mut s = SignataireDemande::personne(RoleSignataire::Titulaire, w.lily.personne);
        s.modes = Some(vec![ModeSignature::Papier]);
        s
    }]).await.unwrap();
    assert_eq!(w.ent.expirer_circuits_echus(w.m.etab, w.jour + chrono::Duration::days(5)).await.unwrap(), 0);
    assert_eq!(w.ent.expirer_circuits_echus(w.m.etab, w.jour + chrono::Duration::days(365)).await.unwrap(), 1);
    assert_eq!(w.etat_circuit(c2.circuit_id).await, "expire");
}

// ───────────────────────── Séparation entre établissements ─────────────────────────

#[tokio::test]
async fn t_separation_entre_etablissements() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sms = Sms::default();
    let eleve = w.eleve("Garcia", "Eli").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.document(eleve, None).await;
    let c = w.lancer(&w.adm(), doc.document_id, true, vec![]).await.unwrap();
    let s = w.signataire_de(c.circuit_id, w.lily.personne).await;
    w.signer_par_code(&sms, &w.parent(&w.lily), s).await.unwrap();
    // Un autre établissement : rien de ce qui est à A n'est visible, lisible, signable ni annulable.
    let cast_b = crate::tests_rls_stricte::seed_supplementaire(&w.ent).await;
    let mb = monde(&w.ent, &cast_b).await;
    let dir_b = Acteur { etablissement_id: mb.etab, user_id: cast_b.direction.user_id, role: "direction" };
    let adm_b = Acteur { etablissement_id: mb.etab, user_id: cast_b.administratif.user_id, role: "administratif" };
    assert!(est_introuvable(&w.ent.circuit_for_actor(&adm_b, c.circuit_id).await.unwrap_err()));
    assert!(est_introuvable(&w.ent.lire_piece_circuit_for_actor(&adm_b, c.circuit_id, PieceCircuit::Preuve).await.unwrap_err()));
    assert!(w.ent.annuler_circuit_for_actor(&dir_b, c.circuit_id, "tentative").await.is_err());
    assert!(w.ent.verifier_signatures_for_actor(&dir_b, c.circuit_id).await.is_err());
    assert!(w.ent.circuits_du_document_for_actor(&dir_b, doc.document_id).await.unwrap().is_empty());
    assert!(w.ent.parapheur_for_actor(&dir_b).await.unwrap().is_empty());
    // Le relevé et le réglage de B ne voient rien de A.
    let releve = w.ent.releve_horodatage_for_actor(&dir_b, w.jour - chrono::Duration::days(1), w.jour + chrono::Duration::days(1)).await.unwrap();
    assert_eq!(releve, ReleveHorodatage { mutualise_obtenus: 0, propre_obtenus: 0, en_attente: 0, echecs: 0 });
    // La vérification publique de B ne reconnaît pas le PDF signé de A.
    let signe = w.ent.lire_piece_circuit_for_actor(&w.adm(), c.circuit_id, PieceCircuit::PdfSigne).await.unwrap();
    assert_eq!(w.ent.verifier_par_depot(mb.etab, &signe).await.unwrap(), crate::documents_natures::ReponseDepot::Different);
}
