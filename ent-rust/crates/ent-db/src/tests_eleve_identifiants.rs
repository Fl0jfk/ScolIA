//! Socle 2 (l'élève), lot E3 : identifiants externes (INE…) et numéro d'élève. Scénarios
//! contre la base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, ils sont ignorés.

use super::*;
use crate::identifiants::AjoutIdentifiant;
use chrono::NaiveDate;
use domain_eleve::{
    ChangeStatus, CreateEleve, EleveStatus, ProvenanceIdentifiant, SystemeIdentifiant,
};
use ent_auth::Role;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

/// Suffixe alphanumérique aléatoire (valeurs uniques d'un test à l'autre).
fn alea(n: usize) -> String {
    Uuid::new_v4().simple().to_string()[..n].to_uppercase()
}

fn ine() -> String {
    format!("{}{}", "1234", alea(7))
}

async fn eleve_avec(
    ent: &PgEnt,
    etab: Uuid,
    nom: &str,
    ine: Option<&str>,
    badge: Option<&str>,
) -> domain_eleve::Eleve {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: format!("{nom}{}", alea(4).replace(char::is_numeric, "x")),
        prenom: "Zélie".into(),
        ine: ine.map(String::from),
        sexe: Some("F".into()),
        date_naissance: NaiveDate::from_ymd_opt(2010, 3, 14),
        numero_badge: badge.map(String::from),
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

fn ajout(etab: Uuid, eleve: Uuid, systeme: SystemeIdentifiant, valeur: &str, acteur: Uuid) -> AjoutIdentifiant {
    AjoutIdentifiant {
        etablissement_id: etab,
        eleve_id: eleve,
        systeme,
        valeur: valeur.into(),
        provenance: ProvenanceIdentifiant::Saisie,
        valide_du: None,
        preuve_document_id: None,
        acteur: Some(acteur),
    }
}

async fn colonne(ent: &PgEnt, sql: &str, eleve: Uuid) -> Option<String> {
    let (v,): (Option<String>,) = sqlx::query_as(sql).bind(eleve).fetch_one(ent.pool()).await.unwrap();
    v
}

/// Ajout d'un INE saisi : `declare`, normalisé, projeté sur la colonne héritée ; ni
/// l'événement ni le journal ne contiennent la valeur.
#[tokio::test]
async fn ajout_ine_declare_projete_sans_valeur_dans_les_traces() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let role = Role::Administratif.as_str();
    let e = eleve_avec(&ent, etab, "Ine", None, None).await;
    let valeur = ine();
    let r = ent
        .ajouter_identifiant(role, ajout(etab, e.id, SystemeIdentifiant::Ine, &valeur.to_lowercase(), admin))
        .await
        .unwrap();
    assert_eq!(r.identifiant.statut, "declare");
    assert_eq!(r.identifiant.valeur, valeur, "INE normalisé en majuscules");
    assert!(!r.collision && !r.existant);
    assert_eq!(
        colonne(&ent, "SELECT ine FROM ent_eleve WHERE id = $1", e.id).await.as_deref(),
        Some(valeur.as_str())
    );
    // Idempotent : le même INE n'écrit rien de plus.
    let encore = ent
        .ajouter_identifiant(role, ajout(etab, e.id, SystemeIdentifiant::Ine, &valeur, admin))
        .await
        .unwrap();
    assert!(encore.existant);
    assert_eq!(encore.identifiant.id, r.identifiant.id);
    // Traces : jamais la valeur.
    let (payload, journal): (serde_json::Value, serde_json::Value) = sqlx::query_as(
        "SELECT (SELECT payload FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.identifiant_change'
                 ORDER BY created_at DESC LIMIT 1),
                (SELECT payload FROM ent_journal_modif WHERE eleve_id = $1 AND action = 'identifiant_ajoute'
                 ORDER BY created_at DESC LIMIT 1)",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(payload["systeme"], "INE");
    assert_eq!(payload["action"], "ajoute");
    for t in [payload.to_string(), journal.to_string()] {
        assert!(!t.to_uppercase().contains(&valeur), "valeur d'INE en clair dans une trace : {t}");
    }
    let (visible,): (bool,) = sqlx::query_as(
        "SELECT visible_famille FROM ent_journal_modif WHERE eleve_id = $1 AND action = 'identifiant_ajoute' LIMIT 1",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(!visible, "le journal des identifiants n'est pas visible de la famille");
    // Format refusé (règle large : 9 à 16 alphanumériques).
    let mauvais = ent
        .ajouter_identifiant(role, ajout(etab, e.id, SystemeIdentifiant::Ine, "court", admin))
        .await;
    assert!(matches!(mauvais, Err(EntDbError::SaisieInvalide(_))), "{mauvais:?}");
}

/// Collision dans l'établissement : le second est enregistré `conteste`, listé en qualité
/// des données ; on ne le confirme qu'après disparition de la collision.
#[tokio::test]
async fn collision_devient_conteste_puis_se_resout() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let role = Role::Administratif.as_str();
    let a = eleve_avec(&ent, etab, "Colla", None, None).await;
    let b = eleve_avec(&ent, etab, "Collb", None, None).await;
    let valeur = ine();
    let ra = ent
        .ajouter_identifiant(role, ajout(etab, a.id, SystemeIdentifiant::Ine, &valeur, admin))
        .await
        .unwrap();
    assert_eq!(ra.identifiant.statut, "declare");
    let rb = ent
        .ajouter_identifiant(role, ajout(etab, b.id, SystemeIdentifiant::Ine, &valeur, admin))
        .await
        .expect("une collision n'échoue pas");
    assert!(rb.collision);
    assert_eq!(rb.collision_eleve_id, Some(a.id));
    assert_eq!(rb.identifiant.statut, "conteste");
    assert!(rb.identifiant.motif.is_some());
    // L'élève A garde son INE actif.
    assert_eq!(colonne(&ent, "SELECT ine FROM ent_eleve WHERE id = $1", a.id).await.as_deref(), Some(valeur.as_str()));
    // Qualité des données : B est listé avec A comme « autre élève ».
    let q = ent.identifiants_contestes(etab, admin, role).await.unwrap();
    let ligne = q.iter().find(|c| c.identifiant_id == rb.identifiant.id).expect("conteste listé");
    assert_eq!(ligne.autre_eleve_id, Some(a.id));
    assert_eq!(ligne.eleve_id, b.id);
    // Confirmer B tant que A porte la valeur : refusé.
    let refus = ent
        .confirmer_identifiant(role, etab, b.id, rb.identifiant.id, admin, "verification_manuelle")
        .await;
    assert!(matches!(refus, Err(EntDbError::ConflitDossier(_))), "{refus:?}");
    // A révoque (c'était la mauvaise fiche) : B peut être confirmé.
    ent.revoquer_identifiant(role, etab, a.id, ra.identifiant.id, admin, Some("saisi sur la mauvaise fiche"))
        .await
        .unwrap();
    assert_eq!(colonne(&ent, "SELECT ine FROM ent_eleve WHERE id = $1", a.id).await, None);
    let ok = ent
        .confirmer_identifiant(role, etab, b.id, rb.identifiant.id, admin, "verification_manuelle")
        .await
        .unwrap();
    assert_eq!(ok.statut, "confirme");
    assert_eq!(ok.confirme_par_source.as_deref(), Some("verification_manuelle"));
    assert!(ok.confirme_le.is_some());
    // Le journal garde la trace des transitions, sans valeur.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.identifiant_change'
         AND payload->>'action' IN ('ajoute', 'confirme')",
    )
    .bind(b.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 2);
}

/// D-013 : l'unicité ne joue QUE dans l'établissement. Le même INE dans deux
/// établissements est accepté partout, sans collision, et le résultat est identique que
/// l'autre établissement l'ait ou non : aucune information ne traverse la frontière.
#[tokio::test]
async fn unicite_par_etablissement_seulement_aucune_verification_entre_etablissements() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab_a, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let etab_b = Uuid::new_v4();
    let role = Role::Administratif.as_str();
    let ea = eleve_avec(&ent, etab_a, "Etaba", None, None).await;
    let eb = eleve_avec(&ent, etab_b, "Etabb", None, None).await;
    let valeur = ine();
    // Avant que A ne l'ait : B enregistre sans collision (état de référence).
    let eb_seul = ent
        .ajouter_identifiant_systeme(ajout(etab_b, eb.id, SystemeIdentifiant::Ine, &valeur, admin))
        .await
        .unwrap();
    assert!(!eb_seul.collision);
    assert_eq!(eb_seul.identifiant.statut, "declare");
    // A enregistre la MÊME valeur : aucune collision, aucun statut particulier.
    let ra = ent
        .ajouter_identifiant(role, ajout(etab_a, ea.id, SystemeIdentifiant::Ine, &valeur, admin))
        .await
        .unwrap();
    assert!(!ra.collision, "une valeur d'un autre établissement ne doit pas être vue");
    assert_eq!(ra.collision_eleve_id, None);
    assert_eq!(ra.identifiant.statut, "declare");
    // Idem pour le badge et la qualité des données : rien dans A ne parle de B.
    let q = ent.identifiants_contestes(etab_a, admin, role).await.unwrap();
    assert!(q.iter().all(|c| c.eleve_id != eb.id && c.autre_eleve_id != Some(eb.id)));
    // L'administratif de A ne peut ni lire ni écrire les identifiants de l'élève de B.
    assert!(matches!(
        ent.identifiants_pour_acteur(etab_a, admin, role, eb.id).await,
        Err(EntDbError::Domain(_))
    ));
    assert!(ent
        .ajouter_identifiant(role, ajout(etab_a, eb.id, SystemeIdentifiant::Ine, &ine(), admin))
        .await
        .is_err());
    // En base : deux lignes actives de même valeur, dans deux établissements.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_eleve_identifiant WHERE lower(valeur) = lower($1) AND statut = 'declare'
         AND etablissement_id IN ($2, $3)",
    )
    .bind(&valeur)
    .bind(etab_a)
    .bind(etab_b)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 2);
}

/// Remplacement : un nouvel INE révoque l'ancien (historique gardé) ; un identifiant
/// confirmé n'est jamais remplacé par une valeur moins sûre.
#[tokio::test]
async fn remplacement_et_protection_de_l_identifiant_confirme() {
    let Some((ent, cast)) = labo().await else { return };
    let (etab, admin) = (cast.etablissement_id, cast.administratif.user_id);
    let role = Role::Administratif.as_str();
    let e = eleve_avec(&ent, etab, "Rempl", None, None).await;
    let (v1, v2) = (ine(), ine());
    let r1 = ent
        .ajouter_identifiant(role, ajout(etab, e.id, SystemeIdentifiant::Ine, &v1, admin))
        .await
        .unwrap();
    let r2 = ent
        .ajouter_identifiant(role, ajout(etab, e.id, SystemeIdentifiant::Ine, &v2, admin))
        .await
        .unwrap();
    assert_eq!(r2.remplace, Some(r1.identifiant.id));
    assert_eq!(colonne(&ent, "SELECT ine FROM ent_eleve WHERE id = $1", e.id).await.as_deref(), Some(v2.as_str()));
    let hist = ent.identifiants_pour_acteur(etab, admin, role, e.id).await.unwrap();
    assert_eq!(hist.len(), 2, "l'ancien identifiant reste lisible");
    assert!(hist.iter().any(|i| i.id == r1.identifiant.id && i.statut == "revoque" && i.valide_au.is_some()));
    // Flux officiel (lot E5) : il n'écrit jamais directement. Le chemin système refuse la
    // provenance officielle ; l'import dépose une proposition, ici décidée par une personne.
    let v3 = ine();
    let direct = ent
        .ajouter_identifiant_systeme(AjoutIdentifiant {
            provenance: ProvenanceIdentifiant::ImportAaf,
            ..ajout(etab, e.id, SystemeIdentifiant::Ine, &v3, admin)
        })
        .await;
    assert!(matches!(direct, Err(EntDbError::SaisieInvalide(_))), "{direct:?}");
    let depot = ent
        .deposer_proposition_systeme(crate::propositions::DepotProposition {
            etablissement_id: etab,
            eleve_id: e.id,
            cible: domain_eleve::CibleProposition::Identifiant,
            source: domain_eleve::SourceProposition::Import,
            origine: Some(domain_eleve::OrigineProposition::Aaf),
            source_ref: Some(Uuid::new_v4()),
            champs: vec![("ine".into(), serde_json::json!(v3))],
            piece_document_id: None,
            commentaire: None,
            expire_le: None,
            depose_par: None,
        })
        .await
        .unwrap();
    let pid = depot.en_attente.expect("aucune règle automatique : en attente").entete.id;
    ent.decider_proposition(
        role,
        crate::propositions::DecisionProposition {
            proposition_id: pid,
            etablissement_id: etab,
            acteur: admin,
            champs_acceptes: vec!["ine".into()],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let hist = ent.identifiants_pour_acteur(etab, admin, role, e.id).await.unwrap();
    let officiel = hist.iter().find(|i| i.valeur == v3).expect("INE officiel enregistré").clone();
    assert_eq!(officiel.statut, "confirme");
    assert_eq!(officiel.provenance, "import_aaf");
    assert!(
        hist.iter().any(|i| i.id == r2.identifiant.id && i.statut == "revoque"),
        "le déclaré est remplacé par le confirmé"
    );
    let refuse = ent
        .ajouter_identifiant(role, ajout(etab, e.id, SystemeIdentifiant::Ine, &ine(), admin))
        .await;
    assert!(matches!(refuse, Err(EntDbError::ConflitDossier(_))), "{refuse:?}");
    // Un opérateur ne peut pas déclarer une provenance réservée aux flux officiels.
    let faux_officiel = ent
        .ajouter_identifiant(role, AjoutIdentifiant {
            provenance: ProvenanceIdentifiant::ImportBee,
            ..ajout(etab, e.id, SystemeIdentifiant::SiecleEleve, "ELV123", admin)
        })
        .await;
    assert!(matches!(faux_officiel, Err(EntDbError::SaisieInvalide(_))), "{faux_officiel:?}");
    // Transition impossible et motif obligatoire.
    let sans_motif = ent
        .contester_identifiant(role, etab, e.id, officiel.id, admin, None)
        .await;
    assert!(matches!(sans_motif, Err(EntDbError::SaisieInvalide(_))));
    let c = ent
        .contester_identifiant(role, etab, e.id, officiel.id, admin, Some("INE différent du livret"))
        .await
        .unwrap();
    assert_eq!(c.statut, "conteste");
    let deux_fois = ent
        .contester_identifiant(role, etab, e.id, officiel.id, admin, Some("encore"))
        .await;
    assert!(matches!(deux_fois, Err(EntDbError::ConflitDossier(_))));
}

/// Pont des écritures héritées : `ent_eleve.ine` / `numero_badge` écrits par un ancien
/// chemin deviennent des identifiants ; une collision y est aussi contestée.
#[tokio::test]
async fn pont_des_ecritures_heritees() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (i1, i2) = (ine(), ine());
    let badge = format!("BDG-{}", alea(6));
    let e = eleve_avec(&ent, etab, "Pont", Some(&i1), Some(&badge)).await;
    let ids: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT systeme, valeur, statut, provenance FROM ent_eleve_identifiant WHERE eleve_id = $1 ORDER BY systeme",
    )
    .bind(e.id)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert_eq!(ids.len(), 2, "{ids:?}");
    assert_eq!((ids[0].0.as_str(), ids[0].2.as_str(), ids[0].3.as_str()), ("BADGE", "confirme", "badge"));
    assert_eq!((ids[1].0.as_str(), ids[1].2.as_str(), ids[1].3.as_str()), ("INE", "declare", "saisie"));
    // Changer l'INE par l'ancienne voie : l'ancien est révoqué, le nouveau déclaré.
    sqlx::query("UPDATE ent_eleve SET ine = $2 WHERE id = $1")
        .bind(e.id)
        .bind(&i2)
        .execute(ent.pool())
        .await
        .unwrap();
    let (actifs, revoques): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE statut = 'declare'), count(*) FILTER (WHERE statut = 'revoque')
         FROM ent_eleve_identifiant WHERE eleve_id = $1 AND systeme = 'INE'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((actifs, revoques), (1, 1));
    // Vider l'INE : révoqué, projection NULL.
    sqlx::query("UPDATE ent_eleve SET ine = NULL WHERE id = $1").bind(e.id).execute(ent.pool()).await.unwrap();
    assert_eq!(colonne(&ent, "SELECT ine FROM ent_eleve WHERE id = $1", e.id).await, None);
    // Collision par l'ancienne voie : conteste, la colonne héritée garde la valeur écrite.
    let autre = eleve_avec(&ent, etab, "Pont2", Some(&i2), None).await;
    let eleve2 = eleve_avec(&ent, etab, "Pont3", Some(&i2), None).await;
    let st: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT eleve_id, statut FROM ent_eleve_identifiant WHERE systeme = 'INE' AND lower(valeur) = lower($1)
         AND etablissement_id = $2 AND statut <> 'revoque'",
    )
    .bind(&i2)
    .bind(etab)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(st.contains(&(autre.id, "declare".into())), "{st:?}");
    assert!(st.contains(&(eleve2.id, "conteste".into())), "{st:?}");
    assert_eq!(
        colonne(&ent, "SELECT ine FROM ent_eleve WHERE id = $1", eleve2.id).await.as_deref(),
        Some(i2.as_str()),
        "rien n'est perdu pour les lecteurs hérités"
    );
    // Une mise à jour héritée qui ne change pas l'INE ne crée rien (change_status → upsert).
    let avant: (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_identifiant WHERE eleve_id = $1")
        .bind(autre.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    ent.change_status(ChangeStatus {
        etablissement_id: etab,
        eleve_id: autre.id,
        nouveau_status: EleveStatus::Actif,
        actor_user_id: None,
    })
    .await
    .unwrap();
    let apres: (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_identifiant WHERE eleve_id = $1")
        .bind(autre.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(avant, apres);
}

/// M3 : la reprise range les INE existants en `a_verifier`, conteste TOUS les doublons
/// d'un établissement, ne perd aucun INE, et est idempotente.
#[tokio::test]
async fn reprise_des_identifiants_existants() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (seul, double) = (ine(), ine());
    let badge = format!("RP-{}", alea(6));
    let (e1, e2, e3) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let mut tx = ent.pool().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_eleve DISABLE TRIGGER ent_eleve_identifiant_pont")
        .execute(&mut *tx)
        .await
        .unwrap();
    for (id, ine, badge) in [
        (e1, Some(seul.clone()), Some(badge.clone())),
        (e2, Some(double.clone()), None),
        (e3, Some(double.to_lowercase()), None),
    ] {
        sqlx::query(
            "INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status, ine, numero_badge)
             VALUES ($1,$2,$3,'Hugo','actif',$4,$5)",
        )
        .bind(id)
        .bind(etab)
        .bind(format!("Reprise{}", alea(5).replace(char::is_numeric, "x")))
        .bind(ine)
        .bind(badge)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    sqlx::query("ALTER TABLE ent_eleve ENABLE TRIGGER ent_eleve_identifiant_pont")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let (n1,): (i64,) = sqlx::query_as("SELECT ent_eleve_identifiant_reprendre()")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(n1 >= 4, "n1 = {n1}");
    let (n2,): (i64,) = sqlx::query_as("SELECT ent_eleve_identifiant_reprendre()")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n2, 0, "reprise idempotente");
    let statut = |eleve: Uuid, systeme: &'static str| {
        let pool = ent.pool().clone();
        async move {
            let (s, p): (String, String) = sqlx::query_as(
                "SELECT statut, provenance FROM ent_eleve_identifiant WHERE eleve_id = $1 AND systeme = $2",
            )
            .bind(eleve)
            .bind(systeme)
            .fetch_one(&pool)
            .await
            .unwrap();
            (s, p)
        }
    };
    assert_eq!(statut(e1, "INE").await, ("a_verifier".into(), "reprise_logiciel".into()));
    assert_eq!(statut(e1, "BADGE").await.0, "confirme");
    assert_eq!(statut(e2, "INE").await.0, "conteste");
    assert_eq!(statut(e3, "INE").await.0, "conteste");
    // Aucun INE perdu : les colonnes héritées sont intactes.
    assert_eq!(colonne(&ent, "SELECT ine FROM ent_eleve WHERE id = $1", e2).await.as_deref(), Some(double.as_str()));
    // Rejouer les migrations ne crée rien non plus.
    ent.migrate().await.unwrap();
    let (n3,): (i64,) = sqlx::query_as("SELECT ent_eleve_identifiant_reprendre()")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n3, 0);
}

/// ELV-41 : badge perdu — l'ancien est révoqué tout de suite, le nouveau actif ; la vie
/// scolaire (CPE) le peut, sans pouvoir toucher à l'INE.
#[tokio::test]
async fn badge_perdu_et_droits_par_systeme() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let (cpe, prof) = (cast.cpe.user_id, cast.prof.user_id);
    let b1 = format!("OLD-{}", alea(6));
    let b2 = format!("NEW-{}", alea(6));
    let e = eleve_avec(&ent, etab, "Badge", None, Some(&b1)).await;
    let r = ent
        .declarer_badge_perdu(Role::Cpe.as_str(), etab, e.id, cpe, Some(&b2))
        .await
        .unwrap()
        .expect("nouveau badge");
    assert_eq!(r.identifiant.statut, "confirme");
    assert_eq!(colonne(&ent, "SELECT numero_badge FROM ent_eleve WHERE id = $1", e.id).await.as_deref(), Some(b2.as_str()));
    let (revoque,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_eleve_identifiant WHERE eleve_id = $1 AND systeme = 'BADGE'
         AND statut = 'revoque' AND motif = 'badge perdu'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(revoque, 2 - 1);
    // Collision de badge : le CPE (sans droit de lecture des identifiants) ne sait pas QUEL élève.
    let autre = eleve_avec(&ent, etab, "Badge2", None, Some(&format!("ZZ-{}", alea(6)))).await;
    let (bautre,): (String,) = sqlx::query_as("SELECT numero_badge FROM ent_eleve WHERE id = $1").bind(autre.id).fetch_one(ent.pool()).await.unwrap();
    let coll = ent.declarer_badge_perdu(Role::Cpe.as_str(), etab, e.id, cpe, Some(&bautre)).await.unwrap().unwrap();
    assert!(coll.collision && coll.collision_eleve_id.is_none());
    // Le CPE ne gère pas l'INE ; le professeur ne gère rien.
    let ine_cpe = ent
        .ajouter_identifiant(Role::Cpe.as_str(), ajout(etab, e.id, SystemeIdentifiant::Ine, &ine(), cpe))
        .await;
    assert!(matches!(ine_cpe, Err(EntDbError::Forbidden)), "{ine_cpe:?}");
    let badge_prof = ent.declarer_badge_perdu(Role::Prof.as_str(), etab, e.id, prof, None).await;
    assert!(matches!(badge_prof, Err(EntDbError::Forbidden)), "{badge_prof:?}");
    // Lecture G3 : refusée au CPE et au professeur, accordée à l'administratif.
    for (u, role) in [(cpe, Role::Cpe), (prof, Role::Prof), (cast.parent_lea.user_id, Role::Parent)] {
        let x = ent.identifiants_pour_acteur(etab, u, role.as_str(), e.id).await;
        assert!(matches!(x, Err(EntDbError::Forbidden)), "{role:?}: {x:?}");
    }
    let l = ent
        .identifiants_pour_acteur(etab, cast.administratif.user_id, Role::Administratif.as_str(), e.id)
        .await
        .unwrap();
    assert_eq!(l.len(), 3, "ancien (révoqué), nouveau (révoqué par le second remplacement), contesté");
    // La lecture est journalisée sans la valeur.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_journal_modif WHERE eleve_id = $1 AND domaine = 'identifiants' AND action = 'consultation'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(n >= 1);
}

// ───────────────────────────── M4 : numéro d'élève ─────────────────────────────

/// Numéro attribué à la création, au format par défaut, unique dans l'établissement,
/// croissant, immuable ; une mise à jour par `upsert` ne brûle aucun numéro.
#[tokio::test]
async fn numero_d_eleve_attribue_unique_immuable() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let a = eleve_avec(&ent, etab, "Num", None, None).await;
    let b = eleve_avec(&ent, etab, "Num", None, None).await;
    let num = |id: Uuid| {
        let pool = ent.pool().clone();
        async move {
            let (n,): (Option<String>,) = sqlx::query_as("SELECT numero FROM ent_eleve WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
            n.expect("numéro attribué")
        }
    };
    let (na, nb) = (num(a.id).await, num(b.id).await);
    let annee = chrono::Utc::now().format("%Y").to_string();
    for n in [&na, &nb] {
        assert!(n.starts_with(&format!("{annee}-")) && n.len() == annee.len() + 6, "{n}");
    }
    assert!(nb > na, "{na} < {nb}");
    // Immuable (même par l'administrateur de la base applicative).
    let modif = sqlx::query("UPDATE ent_eleve SET numero = 'X-1' WHERE id = $1")
        .bind(a.id)
        .execute(ent.pool())
        .await;
    assert!(modif.is_err(), "le numéro a été modifié");
    let efface = sqlx::query("UPDATE ent_eleve SET numero = NULL WHERE id = $1")
        .bind(a.id)
        .execute(ent.pool())
        .await;
    assert!(efface.is_err(), "le numéro a été effacé");
    // Une mise à jour héritée (upsert) ne consomme aucun numéro.
    let compteur = || {
        let pool = ent.pool().clone();
        async move {
            let r: Option<(i64,)> = sqlx::query_as(
                "SELECT derniere FROM ent_eleve_numero_compteur WHERE etablissement_id = $1
                 AND cle = extract(year FROM current_date)::int",
            )
            .bind(etab)
            .fetch_optional(&pool)
            .await
            .unwrap();
            r.map(|x| x.0)
        }
    };
    let avant = compteur().await;
    for st in [EleveStatus::Sorti, EleveStatus::Actif] {
        ent.change_status(ChangeStatus {
            etablissement_id: etab,
            eleve_id: a.id,
            nouveau_status: st,
            actor_user_id: None,
        })
        .await
        .unwrap();
    }
    assert_eq!(compteur().await, avant, "l'upsert a brûlé un numéro");
    assert_eq!(num(a.id).await, na);
    // Unicité dans l'établissement : un doublon est refusé par la base.
    let dup = sqlx::query("UPDATE ent_eleve SET numero = $2 WHERE id = $1 AND false")
        .bind(b.id)
        .bind(&na)
        .execute(ent.pool())
        .await;
    assert!(dup.is_ok());
    let (n_uniq,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM (SELECT numero FROM ent_eleve WHERE etablissement_id = $1 AND numero IS NOT NULL
         GROUP BY numero HAVING count(*) > 1) d",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n_uniq, 0);
}

/// Format paramétré, compteur par établissement (sans fuite d'un établissement à l'autre),
/// numéro qui s'allonge au lieu de se tronquer, formats invalides refusés par la base.
#[tokio::test]
async fn numero_format_parametre_et_compteurs_par_etablissement() {
    let Some((ent, _cast)) = labo().await else { return };
    let (b1, b2) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query(
        "INSERT INTO ent_param_eleve_numero (etablissement_id, format, remise_a_zero_annuelle) VALUES ($1, 'E{seq:2}', false)",
    )
    .bind(b1)
    .execute(ent.pool())
    .await
    .unwrap();
    let mut numeros = Vec::new();
    for _ in 0..3 {
        let e = eleve_avec(&ent, b1, "Fmt", None, None).await;
        let (n,): (String,) = sqlx::query_as("SELECT numero FROM ent_eleve WHERE id = $1")
            .bind(e.id)
            .fetch_one(ent.pool())
            .await
            .unwrap();
        numeros.push(n);
    }
    assert_eq!(numeros, vec!["E01", "E02", "E03"]);
    // Le compteur de l'autre établissement est indépendant et reste au format par défaut.
    let e = eleve_avec(&ent, b2, "Fmt", None, None).await;
    let (n,): (String,) = sqlx::query_as("SELECT numero FROM ent_eleve WHERE id = $1")
        .bind(e.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(n.ends_with("-00001"), "{n}");
    // Dépassement de la largeur : le numéro s'allonge, jamais tronqué.
    sqlx::query("UPDATE ent_eleve_numero_compteur SET derniere = 99 WHERE etablissement_id = $1 AND cle = 0")
        .bind(b1)
        .execute(ent.pool())
        .await
        .unwrap();
    let e = eleve_avec(&ent, b1, "Fmt", None, None).await;
    let (n,): (String,) = sqlx::query_as("SELECT numero FROM ent_eleve WHERE id = $1")
        .bind(e.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, "E100");
    // Formats invalides refusés : sans compteur, caractères interdits, remise à zéro sans année.
    for (fmt, raz) in [("{AAAA}", true), ("E-{seq:5}; DROP", false), ("E{seq:5}", true), ("{seq:0}", false)] {
        let r = sqlx::query(
            "INSERT INTO ent_param_eleve_numero (etablissement_id, format, remise_a_zero_annuelle) VALUES ($1,$2,$3)",
        )
        .bind(Uuid::new_v4())
        .bind(fmt)
        .bind(raz)
        .execute(ent.pool())
        .await;
        assert!(r.is_err(), "format accepté à tort : {fmt}");
    }
}
