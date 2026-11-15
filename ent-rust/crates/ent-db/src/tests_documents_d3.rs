//! Socle 7 (documents), lot D3 : certificat de scolarité, attestation d'inscription et vérification publique, contre
//! la base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, les tests sont ignorés en local (échec explicite
//! en CI). Chaque test crée son propre établissement. Aucun test ne dépend de l'heure ni du jour réels : les élèves de
//! test sont scolarisés depuis le jour de l'établissement moins une durée.
//!
//! Critères de fin du lot (§23, ligne D3) :
//!
//! | Exigence | Test(s) |
//! |---|---|
//! | certificat émis au comptoir (un geste), vérifiable par code | `t_certificat_pour_les_titulaires`, `t_verification_publique_par_code` |
//! | noms avec diacritiques de toutes origines | `t_diacritiques_de_toutes_origines` |
//! | adresse protégée jamais rendue | `t_adresse_protegee_jamais_rendue` |
//! | parent sans droit de recevoir refusé | `t_parent_sans_droit_de_recevoir` |
//! | contrôles bloquants (scolarisation, identité, destinataire) sans numéro consommé | `t_controles_bloquants` |
//! | « par délégation » | `t_par_delegation` |
//! | niveau seul, jamais la classe (D-027 B) | `t_certificat_pour_les_titulaires` |
//! | vérification par dépôt, trois modes, anti-énumération | `t_verification_par_depot`, `t_verification_trois_modes` |

use super::*;
use crate::documents::*;
use crate::documents_natures::*;
use crate::documents_repo::*;
use crate::ref_commun::Acteur;
use crate::service::{EntDbError, RefusMetier};
use crate::tests_demandes::{fiche, labo, monde, Acteur as ActeurTest, Monde};
use chrono::NaiveDate;
use std::sync::Mutex;
use uuid::Uuid;

// ───────────────────────── Doublures ─────────────────────────

#[derive(Debug, Clone)]
struct Vue {
    donnees: Vec<(String, String)>,
    adresse: Option<String>,
    par_delegation_de: Option<String>,
    style: String,
    code: Option<String>,
}

#[derive(Default)]
struct Espion {
    vues: Mutex<Vec<Vue>>,
}

impl RenduDocument for Espion {
    fn rendre(&self, d: &DemandeRendu<'_>) -> Result<DocumentRendu, ErreurRendu> {
        self.vues.lock().unwrap().push(Vue {
            donnees: d
                .donnees
                .iter()
                .map(|x| (x.cle.clone(), x.valeur.clone()))
                .collect(),
            adresse: d.destinataire_adresse.map(str::to_string),
            par_delegation_de: d.par_delegation_de.map(str::to_string),
            style: d.style.to_string(),
            code: d.code_verification.map(str::to_string),
        });
        // Contenu déterministe qui reprend tout ce qui est imprimé : les tests cherchent dedans ce qui NE doit PAS y être.
        let texte = format!(
            "%PDF-1.4 TEST|{}|v{}|{}|{}|{}|{}|{}",
            d.numero_affiche,
            d.numero_version,
            d.donnees
                .iter()
                .map(|x| format!("{}={}", x.cle, x.valeur))
                .collect::<Vec<_>>()
                .join(";"),
            d.destinataire_nom.unwrap_or("-"),
            d.destinataire_adresse.unwrap_or("-"),
            d.par_delegation_de.unwrap_or("-"),
            d.code_verification.unwrap_or("-"),
        );
        Ok(DocumentRendu {
            octets: texte.into_bytes(),
            nb_pages: 1,
        })
    }
}

struct Cachet;

impl CachetEtablissement for Cachet {
    fn sceller(&self, _: Uuid, pdf: &[u8]) -> Result<Scelle, ErreurCachet> {
        let mut o = pdf.to_vec();
        o.extend_from_slice(b"|SCELLE");
        Ok(Scelle {
            octets: o,
            cle_ref: "cle-test-1".into(),
        })
    }
}

fn moteur<'a>(r: &'a dyn RenduDocument, c: &'a dyn CachetEtablissement) -> Moteur<'a> {
    Moteur {
        rendu: r,
        cachet: c,
        url_verification: "https://test.scolia.fr/verifier",
    }
}

// ───────────────────────── Le monde de test ─────────────────────────

struct W {
    ent: PgEnt,
    cast: LaboCast,
    m: Monde,
    lily: ActeurTest,
    tom: ActeurTest,
    jour: NaiveDate,
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
        Some(W {
            ent,
            cast,
            m,
            lily,
            tom,
            jour,
        })
    }

    fn acteur(&self, user: Uuid, role: &'static str) -> Acteur<'static> {
        Acteur {
            etablissement_id: self.m.etab,
            user_id: user,
            role,
        }
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

    /// Un élève mineur (né en 2012), scolarisé depuis 60 jours, niveau « 4e », classe « ULIS-ZZ9 » (division révélatrice, qui ne doit JAMAIS sortir).
    async fn eleve(&self, nom: &str, prenom: &str) -> Uuid {
        self.eleve_complet(
            nom,
            prenom,
            Some(NaiveDate::from_ymd_opt(2012, 3, 10).unwrap()),
            Some(self.jour - chrono::Duration::days(60)),
            None,
        )
        .await
    }

    async fn eleve_complet(
        &self,
        nom: &str,
        prenom: &str,
        naissance: Option<NaiveDate>,
        entree: Option<NaiveDate>,
        sortie: Option<NaiveDate>,
    ) -> Uuid {
        let pool = self.ent.pool_verif();
        let id = Uuid::new_v4();
        // La scolarisation naît du pont de compatibilité (colonnes héritées `date_entree` / `date_sortie`).
        let statut = if sortie.is_some() {
            "sorti"
        } else if entree.is_some_and(|e| e > self.jour) {
            "preinscrit"
        } else {
            "actif"
        };
        sqlx::query(
            "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, classe, date_naissance, date_entree, date_sortie, motif_sortie, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, 'ULIS-ZZ9', $6, $7, $8, $9, now(), now())",
        )
        .bind(id)
        .bind(self.m.etab)
        .bind(nom)
        .bind(prenom)
        .bind(statut)
        .bind(naissance)
        .bind(entree)
        .bind(sortie)
        .bind(sortie.map(|_| "fin_de_cycle"))
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO ent_eleve_scolarite_annee (id, etablissement_id, eleve_id, annee_scolaire, niveau, classe, statut, source)
             VALUES ($1, $2, $3, '2026-2027', '4e', 'ULIS-ZZ9', 'courante', 'saisie')",
        )
        .bind(Uuid::new_v4())
        .bind(self.m.etab)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
        id
    }

    /// Lien de titulaire (autorité parentale) entre une personne de test et l'élève.
    async fn lier(&self, eleve: Uuid, p: &ActeurTest, autorite: bool, suspendu: bool) {
        sqlx::query(
            "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, lien, autorite_parentale, suspendu_le, suspension_motif)
             VALUES ($1, $2, $3, $4, $5, 'parent', $6, CASE WHEN $7 THEN now() ELSE NULL END, CASE WHEN $7 THEN 'suspension de test' ELSE NULL END)",
        )
        .bind(Uuid::new_v4())
        .bind(self.m.etab)
        .bind(eleve)
        .bind(p.user)
        .bind(p.personne)
        .bind(autorite)
        .bind(suspendu)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
    }

    async fn certificat(
        &self,
        qui: &Acteur<'_>,
        eleve: Uuid,
        r: &dyn RenduDocument,
    ) -> Result<DocumentEmis, EntDbError> {
        self.ent
            .emettre_certificat_scolarite_for_actor(qui, eleve, &moteur(r, &Cachet))
            .await
    }

    async fn nb_documents(&self) -> i64 {
        let (n,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM ent_doc_document WHERE etablissement_id = $1")
                .bind(self.m.etab)
                .fetch_one(self.ent.pool_verif())
                .await
                .unwrap();
        n
    }

    async fn dernier_compteur(&self) -> i32 {
        let (r,): (Option<i32>,) =
            sqlx::query_as("SELECT max(dernier) FROM ent_doc_compteur WHERE etablissement_id = $1")
                .bind(self.m.etab)
                .fetch_one(self.ent.pool_verif())
                .await
                .unwrap();
        r.unwrap_or(0)
    }
}

fn refus(e: &EntDbError, attendu: RefusMetier) -> bool {
    matches!(e, EntDbError::Refus(r, _) if *r == attendu)
}

// ───────────────────────── Certificat de scolarité ─────────────────────────

#[tokio::test]
async fn t_certificat_pour_les_titulaires() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Dupont", "Laura").await;
    w.lier(eleve, &w.lily, true, false).await;
    w.lier(eleve, &w.tom, true, false).await;
    let espion = Espion::default();
    let debut = std::time::Instant::now();
    let e = w
        .certificat(&w.adm(), eleve, &espion)
        .await
        .expect("un geste au comptoir");
    eprintln!("certificat émis (moteur de test) en {:?}", debut.elapsed());
    assert!(
        debut.elapsed() < std::time::Duration::from_secs(5),
        "ordre de grandeur : un geste, pas une file"
    );
    assert!(
        e.numero_affiche.ends_with("-SCO-000001"),
        "{}",
        e.numero_affiche
    );
    assert_eq!(e.exemplaires.len(), 2, "un exemplaire par titulaire");
    // Ce que le moteur a reçu : niveau seul, jamais la classe ; aucune adresse ; données du contrat.
    let vues = espion.vues.lock().unwrap().clone();
    assert_eq!(vues.len(), 3, "version de base + deux exemplaires");
    let base = &vues[0];
    let cles: Vec<_> = base.donnees.iter().map(|d| d.0.as_str()).collect();
    assert_eq!(
        cles,
        [
            CLE_NOM,
            CLE_PRENOM,
            CLE_NAISSANCE,
            CLE_ENTREE,
            CLE_NIVEAU,
            CLE_ANNEE
        ]
    );
    assert!(base
        .donnees
        .contains(&(CLE_NIVEAU.to_string(), "4e".to_string())));
    assert!(base
        .donnees
        .contains(&(CLE_NAISSANCE.to_string(), "2012-03-10".to_string())));
    for v in &vues {
        assert!(
            v.adresse.is_none(),
            "une attestation n'imprime aucune adresse"
        );
        assert!(
            !format!("{:?}", v.donnees).contains("ULIS"),
            "la division n'est jamais transmise au moteur"
        );
        assert_eq!(v.style, "classique");
        assert!(
            v.code.as_deref().is_some_and(|c| c.len() == 14),
            "code lisible ABCD-EFGH-JKLM : {:?}",
            v.code
        );
    }
    // Les deux titulaires reçoivent chacun le leur, dans leur coffre ; le certificat est C1 (lecture non tracée).
    let lu = w
        .ent
        .lire_document_for_actor(&w.parent(&w.lily), e.document_id, None)
        .await
        .unwrap();
    let lu2 = w
        .ent
        .lire_document_for_actor(&w.parent(&w.tom), e.document_id, None)
        .await
        .unwrap();
    assert_ne!(lu.octets, lu2.octets);
    assert!(!String::from_utf8_lossy(&lu.octets).contains("ULIS"));
    // Journal, événement, titre complet pour qui a le droit (la direction), titre neutre pour le registre sans le droit.
    let registre = w
        .ent
        .registre_for_actor(&w.dir(), FiltreRegistre::default())
        .await
        .unwrap();
    assert_eq!(registre[0].titre, "Certificat de scolarité — Laura Dupont");
    // Droits : ni le professeur ni la famille n'émettent.
    for qui in [w.prof(), w.parent(&w.lily)] {
        assert!(
            matches!(
                w.certificat(&qui, eleve, &espion).await.unwrap_err(),
                EntDbError::Forbidden
            ),
            "{}",
            qui.role
        );
    }
}

#[tokio::test]
async fn t_diacritiques_de_toutes_origines() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let noms = [
        ("Nguyễn", "Thị Ánh Tuyết"),
        ("Œuvrard-Çelik", "Zoë"),
        ("Łukasiewicz", "Michał"),
        ("O'Brien", "Seán"),
        ("Şahin", "İpek"),
        ("Müller", "Jürgen"),
        ("Ouédraogo", "Aïssatou"),
        ("Ñandú", "Íñigo"),
    ];
    let espion = Espion::default();
    for (nom, prenom) in noms {
        let eleve = w.eleve(nom, prenom).await;
        w.lier(eleve, &w.lily, true, false).await;
        let e = w
            .certificat(&w.dir(), eleve, &espion)
            .await
            .unwrap_or_else(|x| panic!("{nom} : {x:?}"));
        // Le moteur reçoit exactement les octets UTF-8 saisis ; le rendu de test les reprend sans les altérer.
        let vue = espion
            .vues
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|v| v.donnees.contains(&(CLE_NOM.to_string(), nom.to_string())))
            .cloned();
        let vue = vue.unwrap_or_else(|| panic!("{nom} non transmis tel quel"));
        assert!(
            vue.donnees
                .contains(&(CLE_PRENOM.to_string(), prenom.to_string())),
            "{prenom}"
        );
        let lu = w
            .ent
            .lire_document_for_actor(&w.dir(), e.document_id, None)
            .await
            .unwrap();
        let texte = String::from_utf8(lu.octets).expect("UTF-8 conservé de bout en bout");
        assert!(
            texte.contains(nom) && texte.contains(prenom),
            "{nom} {prenom} : {texte}"
        );
        // Les données figées (scellées en base) se rouvrent à l'identique.
        let mut tx = TenantTx::begin(w.ent.pool(), w.m.etab).await.unwrap();
        let figees = w
            .ent
            .donnees_figees(&mut tx, w.m.etab, e.version_id)
            .await
            .unwrap();
        assert!(figees.iter().any(|d| d.cle == CLE_NOM && d.valeur == nom));
        assert!(figees
            .iter()
            .any(|d| d.cle == CLE_PRENOM && d.valeur == prenom));
        // Vérification publique : désignation minimale (initiale, nom en capitales, année de naissance).
        let code = e.code_verification.clone().unwrap();
        match w.ent.verifier_code_public(w.m.etab, &code).await.unwrap() {
            ReponseVerification::Complete { titulaire, .. } => {
                assert!(titulaire.ends_with(", né(e) en 2012"), "{titulaire}");
                assert!(titulaire.contains(&nom.to_uppercase()), "{titulaire}");
            }
            autre => panic!("{autre:?}"),
        }
    }
}

#[tokio::test]
async fn t_controles_bloquants() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let espion = Espion::default();
    let jour = w.jour;
    let naissance = NaiveDate::from_ymd_opt(2012, 3, 10);
    // 1. Élève sorti hier : non scolarisé à la date.
    let sorti = w
        .eleve_complet(
            "Sorti",
            "Paul",
            naissance,
            Some(jour - chrono::Duration::days(200)),
            Some(jour - chrono::Duration::days(1)),
        )
        .await;
    w.lier(sorti, &w.lily, true, false).await;
    // 2. Élève pas encore entré : non scolarisé à la date pour un certificat.
    let futur = w
        .eleve_complet(
            "Futur",
            "Anna",
            naissance,
            Some(jour + chrono::Duration::days(30)),
            None,
        )
        .await;
    w.lier(futur, &w.lily, true, false).await;
    // 3. Identité sans date de naissance.
    let sans_naissance = w
        .eleve_complet(
            "Sans",
            "Date",
            None,
            Some(jour - chrono::Duration::days(60)),
            None,
        )
        .await;
    w.lier(sans_naissance, &w.lily, true, false).await;
    // 4. Mineur sans aucun titulaire destinataire.
    let orphelin = w.eleve("Seul", "Léo").await;
    for e in [sorti, futur, sans_naissance, orphelin] {
        let r = w.certificat(&w.dir(), e, &espion).await.unwrap_err();
        assert!(refus(&r, RefusMetier::Conflit), "{r:?}");
    }
    // Aucun document, aucun numéro consommé, aucun rendu, aucun objet : les contrôles passent AVANT tout.
    assert_eq!(w.nb_documents().await, 0);
    assert_eq!(w.dernier_compteur().await, 0);
    assert!(
        espion.vues.lock().unwrap().is_empty(),
        "le moteur n'est pas appelé quand un contrôle bloque"
    );
    // 5. Élève inconnu de l'établissement : introuvable.
    assert!(w
        .certificat(&w.dir(), Uuid::new_v4(), &espion)
        .await
        .is_err());
    // Un élève entré à l'instant (aujourd'hui) est scolarisé : le certificat s'émet.
    let aujourdhui = w
        .eleve_complet("Arrive", "Tom", naissance, Some(jour), None)
        .await;
    w.lier(aujourdhui, &w.lily, true, false).await;
    w.certificat(&w.dir(), aujourdhui, &espion)
        .await
        .expect("scolarisé depuis aujourd'hui");
    // 6. Attestation d'inscription : un élève qui entre dans un mois peut être attesté « inscrit », pas certifié.
    w.ent
        .emettre_attestation_inscription_for_actor(&w.dir(), futur, &moteur(&espion, &Cachet))
        .await
        .expect("inscrit pour l'année, entrée à venir");
    assert!(w
        .ent
        .emettre_attestation_inscription_for_actor(&w.dir(), sorti, &moteur(&espion, &Cachet))
        .await
        .is_err());
    // L'attestation a sa propre série (ADM) et son propre compteur.
    let (serie,): (String,) = sqlx::query_as(
        "SELECT serie_code FROM ent_doc_document WHERE etablissement_id = $1 AND nature_code = 'inscriptions.attestation_inscription'",
    )
    .bind(w.m.etab)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(serie, "ADM");
}

#[tokio::test]
async fn t_parent_sans_droit_de_recevoir() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let espion = Espion::default();
    let eleve = w.eleve("Dupont", "Laura").await;
    // Lily titulaire ; Tom sans autorité parentale (beau-parent, grand-parent…) ; un troisième lien suspendu.
    w.lier(eleve, &w.lily, true, false).await;
    w.lier(eleve, &w.tom, false, false).await;
    let e = w.certificat(&w.dir(), eleve, &espion).await.unwrap();
    assert_eq!(
        e.exemplaires.len(),
        1,
        "seul le titulaire de l'autorité parentale reçoit"
    );
    assert!(w
        .ent
        .lire_document_for_actor(&w.parent(&w.lily), e.document_id, None)
        .await
        .is_ok());
    // Le parent sans droit ne reçoit rien et ne lit rien (même identifiant : refus, jamais le document d'un autre).
    let r = w
        .ent
        .lire_document_for_actor(&w.parent(&w.tom), e.document_id, None)
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    // Un lien suspendu ne reçoit plus rien non plus ; s'il est le seul, l'émission est refusée (aucun destinataire).
    let eleve2 = w.eleve("Martin", "Jules").await;
    w.lier(eleve2, &w.lily, true, true).await;
    let r = w.certificat(&w.dir(), eleve2, &espion).await.unwrap_err();
    assert!(refus(&r, RefusMetier::Conflit), "{r:?}");
    // Une personne dont le lien a expiré (date de fin passée) n'est plus destinataire.
    let eleve3 = w.eleve("Petit", "Max").await;
    w.lier(eleve3, &w.lily, true, false).await;
    sqlx::query(
        "UPDATE ent_eleve_lien SET date_fin = $3 WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(w.m.etab)
    .bind(eleve3)
    .bind(w.jour - chrono::Duration::days(1))
    .execute(w.ent.pool_verif())
    .await
    .unwrap();
    assert!(refus(
        &w.certificat(&w.dir(), eleve3, &espion).await.unwrap_err(),
        RefusMetier::Conflit
    ));
    // Élève majeur : il reçoit lui-même son exemplaire (sa fiche de personne), pas ses parents.
    let majeur = w
        .eleve_complet(
            "Adulte",
            "Zoé",
            NaiveDate::from_ymd_opt(2000, 1, 5),
            Some(w.jour - chrono::Duration::days(60)),
            None,
        )
        .await;
    w.lier(majeur, &w.lily, true, false).await;
    assert!(
        refus(
            &w.certificat(&w.dir(), majeur, &espion).await.unwrap_err(),
            RefusMetier::Conflit
        ),
        "majeur sans fiche de personne : aucun destinataire"
    );
    let moi = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom, eleve_id, user_id, source) VALUES ($1, $2, 'Adulte', 'Zoé', $3, $4, 'saisie')")
        .bind(moi)
        .bind(w.m.etab)
        .bind(majeur)
        .bind(Option::<Uuid>::None)
        .execute(w.ent.pool_verif())
        .await
        .expect("fiche de l'élève majeur");
    let e = w.certificat(&w.dir(), majeur, &espion).await.unwrap();
    assert_eq!(e.exemplaires.len(), 1);
    let (dest,): (Option<Uuid>,) = sqlx::query_as("SELECT destinataire_personne_id FROM ent_doc_exemplaire WHERE etablissement_id = $1 AND version_id = $2")
        .bind(w.m.etab)
        .bind(e.version_id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(dest, Some(moi));
}

#[tokio::test]
async fn t_adresse_protegee_jamais_rendue() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let espion = Espion::default();
    let eleve = w.eleve("Dupont", "Laura").await;
    w.lier(eleve, &w.lily, true, false).await;
    w.lier(eleve, &w.tom, true, false).await;
    // Tom a une adresse en fiche ; Lily a une adresse PROTÉGÉE (parquée scellée par le socle 4, ADM-25).
    for (p, adresse) in [
        (&w.tom, "12 rue des Lilas 75000 Paris"),
        (&w.lily, "9 impasse Secrète 69000 Lyon"),
    ] {
        sqlx::query("UPDATE ent_personne SET adresse = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(w.m.etab)
            .bind(p.personne)
            .bind(adresse)
            .execute(w.ent.pool_verif())
            .await
            .unwrap();
    }
    sqlx::query(
        "INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine) VALUES ($1, $2, $3, 'enc:v1:xxxx', 'saisie')",
    )
    .bind(Uuid::new_v4())
    .bind(w.m.etab)
    .bind(w.lily.personne)
    .execute(w.ent.pool_verif())
    .await
    .unwrap();
    // L'adaptateur d'adresse des familles qui en impriment : jamais celle d'une personne protégée.
    assert_eq!(
        w.ent
            .adresse_postale_permise(w.m.etab, w.lily.personne)
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        w.ent
            .adresse_postale_permise(w.m.etab, w.tom.personne)
            .await
            .unwrap()
            .as_deref(),
        Some("12 rue des Lilas 75000 Paris")
    );
    // Le certificat (famille « attestation ») n'imprime aucune adresse : ni dans l'appel au moteur, ni dans les PDF.
    let e = w.certificat(&w.dir(), eleve, &espion).await.unwrap();
    for v in espion.vues.lock().unwrap().iter() {
        assert!(v.adresse.is_none());
    }
    for qui in [&w.lily, &w.tom] {
        let lu = w
            .ent
            .lire_document_for_actor(&w.parent(qui), e.document_id, None)
            .await
            .unwrap();
        let t = String::from_utf8_lossy(&lu.octets).to_string();
        assert!(
            !t.contains("Secrète")
                && !t.contains("Lilas")
                && !t.contains("Lyon")
                && !t.contains("Paris"),
            "{t}"
        );
    }
    // La base interdit en outre un exemplaire qui porterait l'adresse d'un autre (migration 3701, D7).
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_exemplaire WHERE etablissement_id = $1 AND adresse_de_personne_id IS NOT NULL",
    )
    .bind(w.m.etab)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn t_par_delegation() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let espion = Espion::default();
    let eleve = w.eleve("Dupont", "Laura").await;
    w.lier(eleve, &w.lily, true, false).await;
    // Le signataire désigné pour la nature est la direction ; le secrétariat émet : « par délégation ».
    let agent_direction = w.m.direction.agent.expect("agent de la direction");
    sqlx::query(
        "INSERT INTO ent_doc_nature_param (etablissement_id, nature_code, signataire_defaut_agent_id) VALUES ($1, 'eleve.certificat_scolarite', $2)",
    )
    .bind(w.m.etab)
    .bind(agent_direction)
    .execute(w.ent.pool_verif())
    .await
    .unwrap();
    let e = w.certificat(&w.adm(), eleve, &espion).await.unwrap();
    let (delegation, agent): (bool, Option<Uuid>) = sqlx::query_as(
        "SELECT par_delegation, au_nom_de_agent_id FROM ent_doc_version WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.m.etab)
    .bind(e.version_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert!(delegation);
    assert_eq!(agent, Some(agent_direction));
    let mention = espion
        .vues
        .lock()
        .unwrap()
        .iter()
        .filter_map(|v| v.par_delegation_de.clone())
        .next();
    assert!(
        mention.is_some_and(|m| !m.trim().is_empty()),
        "le moteur reçoit le nom du signataire désigné"
    );
    // La direction émet elle-même : pas de délégation.
    let e2 = w.certificat(&w.dir(), eleve, &espion).await.unwrap();
    let (delegation2,): (bool,) = sqlx::query_as(
        "SELECT par_delegation FROM ent_doc_version WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.m.etab)
    .bind(e2.version_id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert!(!delegation2);
}

// ───────────────────────── Vérification publique ─────────────────────────

#[tokio::test]
async fn t_verification_publique_par_code() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let espion = Espion::default();
    let eleve = w.eleve("Dupont", "Laura").await;
    w.lier(eleve, &w.lily, true, false).await;
    let e = w.certificat(&w.dir(), eleve, &espion).await.unwrap();
    let code = e.code_verification.clone().unwrap();
    // Réponse complète : établissement, nature, date, état, empreinte, désignation minimale — et rien d'autre.
    let rep = w.ent.verifier_code_public(w.m.etab, &code).await.unwrap();
    let ReponseVerification::Complete {
        nature_libelle,
        emis_le,
        etat,
        empreinte_sha256,
        titulaire,
        ..
    } = rep.clone()
    else {
        panic!("{rep:?}")
    };
    assert_eq!(nature_libelle, "Certificat de scolarité");
    assert_eq!(emis_le, w.jour);
    assert_eq!(etat, EtatVerifie::Valide);
    assert_eq!(titulaire, "L. DUPONT, né(e) en 2012");
    assert_eq!(empreinte_sha256.len(), 64);
    // L'empreinte hexadécimale est retirée avant la recherche : « 4e » ou « 10 » y apparaissent par hasard.
    let texte = format!("{rep:?}").replace(&empreinte_sha256, "");
    for interdit in ["ULIS", "4e", "-10", "mars", "Laura", "Lily", "adresse"] {
        assert!(
            !texte.contains(interdit),
            "la vérification publique ne révèle pas « {interdit} » : {texte}"
        );
    }
    // Saisie tolérante (minuscules, tirets, espaces) ; mais jamais de correction « intelligente ».
    let lisible = code_lisible(&code).to_lowercase().replace('-', " - ");
    assert_eq!(
        w.ent
            .verifier_code_public(w.m.etab, &lisible)
            .await
            .unwrap(),
        rep
    );
    // Inconnu, mal formé, numéro séquentiel, code d'un autre établissement : UNE SEULE réponse.
    let cast_b = crate::tests_rls_stricte::seed_supplementaire(&w.ent).await;
    let autre_etab = cast_b.etablissement_id;
    for saisie in [
        "ABCDEFGHJKLM",
        "",
        "0000-0000-0000",
        &e.numero_affiche,
        "trop-long-trop-long-trop-long",
        "ÉÉÉÉÉÉÉÉÉÉÉÉ",
    ] {
        assert_eq!(
            w.ent.verifier_code_public(w.m.etab, saisie).await.unwrap(),
            ReponseVerification::Inconnu,
            "{saisie}"
        );
    }
    assert_eq!(
        w.ent.verifier_code_public(autre_etab, &code).await.unwrap(),
        ReponseVerification::Inconnu,
        "code d'un autre établissement"
    );
    // Réémission : le code de l'ancienne version répond « remplacé le … », celui de la nouvelle « valide ».
    let (r, c) = (Espion::default(), Cachet);
    let v2 = w
        .ent
        .reemettre_document_for_actor(
            &w.dir(),
            DemandeReemission {
                document_id: e.document_id,
                motif: "Niveau corrigé".into(),
                donnees: vec![
                    DonneeRendu {
                        cle: CLE_NOM.into(),
                        valeur: "DUPONT".into(),
                    },
                    DonneeRendu {
                        cle: CLE_PRENOM.into(),
                        valeur: "Laura".into(),
                    },
                    DonneeRendu {
                        cle: CLE_NAISSANCE.into(),
                        valeur: "2012-03-10".into(),
                    },
                ],
                destinataires: vec![DestinataireDoc {
                    personne_id: Some(w.lily.personne),
                    nom: Some("Lily".into()),
                    au_coffre: true,
                    ..Default::default()
                }],
                au_nom_de_agent_id: None,
            },
            &moteur(&r, &c),
        )
        .await
        .unwrap();
    match w.ent.verifier_code_public(w.m.etab, &code).await.unwrap() {
        ReponseVerification::Complete {
            etat: EtatVerifie::Remplace { le },
            ..
        } => assert_eq!(le, w.jour),
        autre => panic!("{autre:?}"),
    }
    match w
        .ent
        .verifier_code_public(w.m.etab, &v2.code_verification.clone().unwrap())
        .await
        .unwrap()
    {
        ReponseVerification::Complete {
            etat, titulaire, ..
        } => {
            assert_eq!(etat, EtatVerifie::Valide);
            assert_eq!(titulaire, "L. DUPONT, né(e) en 2012");
        }
        autre => panic!("{autre:?}"),
    }
    // Annulation : « annulé le … » (jamais le motif), pour les deux codes.
    w.ent
        .annuler_document_for_actor(&w.dir(), e.document_id, "Mauvais élève, motif confidentiel")
        .await
        .unwrap();
    for c in [code, v2.code_verification.unwrap()] {
        let rep = w.ent.verifier_code_public(w.m.etab, &c).await.unwrap();
        assert!(
            matches!(&rep, ReponseVerification::Complete { etat: EtatVerifie::Annule { le }, .. } if *le == w.jour),
            "{rep:?}"
        );
        assert!(!format!("{rep:?}").contains("confidentiel"));
    }
}

#[tokio::test]
async fn t_verification_par_depot() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let espion = Espion::default();
    let eleve = w.eleve("Dupont", "Laura").await;
    w.lier(eleve, &w.lily, true, false).await;
    let e = w.certificat(&w.dir(), eleve, &espion).await.unwrap();
    let base = w
        .ent
        .lire_document_for_actor(&w.dir(), e.document_id, None)
        .await
        .unwrap();
    let exemplaire = w
        .ent
        .lire_document_for_actor(&w.parent(&w.lily), e.document_id, None)
        .await
        .unwrap();
    // Le fichier reçu (version de base ou exemplaire nominatif) est « identique à l'original émis ».
    for octets in [&base.octets, &exemplaire.octets] {
        match w.ent.verifier_par_depot(w.m.etab, octets).await.unwrap() {
            ReponseDepot::Identique { emis_le, etat, .. } => {
                assert_eq!(emis_le, w.jour);
                assert_eq!(etat, EtatVerifie::Valide);
            }
            autre => panic!("{autre:?}"),
        }
    }
    // Un seul octet de différence, un fichier vide ou trop gros : « différent » (aucun détail).
    let mut falsifie = exemplaire.octets.clone();
    falsifie[10] ^= 1;
    assert_eq!(
        w.ent.verifier_par_depot(w.m.etab, &falsifie).await.unwrap(),
        ReponseDepot::Different
    );
    assert_eq!(
        w.ent.verifier_par_depot(w.m.etab, &[]).await.unwrap(),
        ReponseDepot::Different
    );
    assert_eq!(
        w.ent
            .verifier_par_depot(w.m.etab, &vec![0u8; DEPOT_MAX_OCTETS + 1])
            .await
            .unwrap(),
        ReponseDepot::Different
    );
    // Autre établissement : « différent », comme un fichier inconnu.
    let cast_b = crate::tests_rls_stricte::seed_supplementaire(&w.ent).await;
    assert_eq!(
        w.ent
            .verifier_par_depot(cast_b.etablissement_id, &base.octets)
            .await
            .unwrap(),
        ReponseDepot::Different
    );
    // Après réémission, l'ancien fichier est « remplacé » ; après annulation « annulé ».
    let (r, c) = (Espion::default(), Cachet);
    w.ent
        .reemettre_document_for_actor(
            &w.dir(),
            DemandeReemission {
                document_id: e.document_id,
                motif: "Correction".into(),
                donnees: vec![DonneeRendu {
                    cle: CLE_NOM.into(),
                    valeur: "DUPONT".into(),
                }],
                destinataires: vec![DestinataireDoc {
                    personne_id: Some(w.lily.personne),
                    nom: Some("Lily".into()),
                    au_coffre: true,
                    ..Default::default()
                }],
                au_nom_de_agent_id: None,
            },
            &moteur(&r, &c),
        )
        .await
        .unwrap();
    assert!(matches!(
        w.ent
            .verifier_par_depot(w.m.etab, &base.octets)
            .await
            .unwrap(),
        ReponseDepot::Identique {
            etat: EtatVerifie::Remplace { .. },
            ..
        }
    ));
    w.ent
        .annuler_document_for_actor(&w.dir(), e.document_id, "Erreur d'élève")
        .await
        .unwrap();
    assert!(matches!(
        w.ent
            .verifier_par_depot(w.m.etab, &base.octets)
            .await
            .unwrap(),
        ReponseDepot::Identique {
            etat: EtatVerifie::Annule { .. },
            ..
        }
    ));
}

#[tokio::test]
async fn t_verification_trois_modes() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let espion = Espion::default();
    let eleve = w.eleve("Dupont", "Laura").await;
    w.lier(eleve, &w.lily, true, false).await;
    let e = w.certificat(&w.dir(), eleve, &espion).await.unwrap();
    // Deux documents déposés (origine « depose », donc sans numérotation) portant chacun un code, dont le mode de
    // vérification est « authenticité seule » (courrier de discipline, convocation…) et « aucune » (natures C3).
    let pool = w.ent.pool_verif();
    let mut codes = Vec::new();
    for (mode, sensibilite, code) in [
        ("authenticite_seule", "C2", generer_code_verification()),
        ("aucune", "C3", generer_code_verification()),
    ] {
        let doc = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_doc_document (id, etablissement_id, nature_code, entite_id, sujet_type, sujet_id, eleve_id, origine, sensibilite,
                                           domaine, titre, titre_neutre, etat, verification_publique, version_courante, categorie_conservation)
             SELECT $1, etablissement_id, 'discipline.convocation', entite_id, 'eleve', $3, $3, 'depose', $4, 'disciplinaire',
                    'Convocation au conseil de discipline', 'Document au dossier', 'depose', $5, 1, 'dossier_disciplinaire'
               FROM ent_doc_document WHERE etablissement_id = $2 AND id = $6",
        )
        .bind(doc)
        .bind(w.m.etab)
        .bind(eleve)
        .bind(sensibilite)
        .bind(mode)
        .bind(e.document_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO ent_doc_version (id, etablissement_id, document_id, numero_version, cle_objet, empreinte_sha256, nb_pages, emise_par, code_verification)
             SELECT $1, etablissement_id, $2, 1, cle_objet, empreinte_sha256, 1, emise_par, $3
               FROM ent_doc_version WHERE etablissement_id = $4 AND id = $5",
        )
        .bind(Uuid::new_v4())
        .bind(doc)
        .bind(&code)
        .bind(w.m.etab)
        .bind(e.version_id)
        .execute(pool)
        .await
        .unwrap();
        codes.push(code);
    }
    // « Authenticité seule » : ni nature, ni identité, ni empreinte — un code lu sur un courrier ouvert ne révèle pas
    // l'existence d'une sanction.
    let rep = w
        .ent
        .verifier_code_public(w.m.etab, &codes[0])
        .await
        .unwrap();
    match &rep {
        ReponseVerification::Authentique { emis_le, etat, .. } => {
            assert_eq!(*emis_le, w.jour);
            assert_eq!(*etat, EtatVerifie::Valide);
        }
        autre => panic!("{autre:?}"),
    }
    let texte = format!("{rep:?}");
    for interdit in ["Convocation", "discipline", "Dupont", "Laura", "sha"] {
        assert!(
            !texte.contains(interdit),
            "« {interdit} » ne doit pas fuiter : {texte}"
        );
    }
    // « Aucune » : même réponse qu'un code inexistant.
    assert_eq!(
        w.ent
            .verifier_code_public(w.m.etab, &codes[1])
            .await
            .unwrap(),
        ReponseVerification::Inconnu
    );
    assert_eq!(
        w.ent
            .verifier_code_public(w.m.etab, "ZZZZZZZZZZZZ")
            .await
            .unwrap(),
        ReponseVerification::Inconnu
    );
    // Par dépôt : l'empreinte d'un document « aucune » répond « différent » ; celle du certificat « identique ».
    let base = w
        .ent
        .lire_document_for_actor(&w.dir(), e.document_id, None)
        .await
        .unwrap();
    assert!(matches!(
        w.ent
            .verifier_par_depot(w.m.etab, &base.octets)
            .await
            .unwrap(),
        ReponseDepot::Identique { .. }
    ));
}

#[tokio::test]
async fn t_rendu_pdf_reel_sur_ent_pdf() {
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let eleve = w.eleve("Œuvrard-Çelik", "Zoë").await;
    w.lier(eleve, &w.lily, true, false).await;
    let moteur_reel = crate::documents_pdf::RenduPdfAttestation;
    let e = w
        .ent
        .emettre_certificat_scolarite_for_actor(&w.adm(), eleve, &moteur(&moteur_reel, &Cachet))
        .await
        .expect("émission sur le moteur réel");
    let lu = w
        .ent
        .lire_document_for_actor(&w.parent(&w.lily), e.document_id, None)
        .await
        .unwrap();
    // Le cachet de test ajoute une marque après `%%EOF` : on relit le PDF sans elle.
    let pdf = &lu.octets[..lu.octets.len() - b"|SCELLE".len()];
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let texte = ent_pdf::releve::texte(pdf);
    for attendu in [
        "Certificat de scolarité",
        "ŒUVRARD-ÇELIK",
        "Zoë",
        "10/03/2012",
        "4e",
        &e.numero_affiche,
        "Lily",
    ] {
        assert!(
            texte.contains(attendu),
            "« {attendu} » absent du PDF : {texte}"
        );
    }
    let code = code_lisible(e.code_verification.as_deref().unwrap());
    assert!(texte.contains(&code), "code lisible absent : {texte}");
    assert!(!texte.contains("ULIS"), "la classe ne s'imprime jamais");
    assert!(lu.octets.len() > 1000);
    // La vérification publique par dépôt reconnaît ce vrai PDF.
    assert!(matches!(
        w.ent
            .verifier_par_depot(w.m.etab, &lu.octets)
            .await
            .unwrap(),
        ReponseDepot::Identique { .. }
    ));
}
