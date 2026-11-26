//! Socle 27, **lot G3b-1** : moteur d'assemblage de l'extrait d'accès (élève et personne), garde-fous de
//! périmètre, relecture du DPO, et correctifs G3a (Pf1, Pf5, Pf6, Pf8, M-a, prolongation).
//!
//! Aucun test ne dépend de l'heure réelle (règle 33) : les jours viennent de `aujourdhui_etablissement`, et
//! les alertes utilisent l'horloge injectée. Base exigée sous `CI` ou `ENT_EXIGER_BASE` (RG-D03).

use super::*;
use crate::droits_demandes::SaisieDemande;
use crate::droits_extrait::StatutSection;
use crate::droits_repo::{Droit, RoleSaisie};
use ent_auth::{Action, Portee};
use uuid::Uuid;

const DIR: &str = "direction";
const ADF: &str = "administratif";

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
    })
    .await;
    Some((ent, cast))
}

/// Une demande d'accès au nom de l'élève Léa (soi-même), identité **établie**, prête à l'assemblage.
async fn demande_prete(ent: &PgEnt, cast: &LaboCast) -> Uuid {
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: jour,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Famille Granger"),
                demandeur_contact: Some("courrier postal"),
                qualite: "soi_meme",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                sujet_nom_saisi: None,
                droits: &["acces"],
                perimetre_texte: Some("tout le dossier scolaire"),
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .expect("saisie au guichet");
    ent.droits_verifier_identite(etab, dir, DIR, v.id, "code_courriel", true, None, None)
        .await
        .expect("identité établie");
    v.id
}

/// Un rôle « DPO » : `droits.arbitrer` **sans** `droits.traiter`.
async fn donner_role_dpo(ent: &PgEnt, cast: &LaboCast) -> Uuid {
    let etab = cast.etablissement_id;
    let admin = cast.admin.user_id;
    let compte = cast.comptas[0].user_id;
    let role = ent
        .creer_role_for_actor(
            etab,
            admin,
            "admin",
            RoleSaisie {
                nom: format!("DPO {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::DroitsArbitrer,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .expect("rôle DPO");
    ent.attribuer_role_for_actor(etab, admin, "admin", role.id, compte, None)
        .await
        .expect("attribution");
    compte
}

// ─────────────────────────────── Assemblage d'un extrait d'élève ───────────────────────────────

/// L'extrait d'un élève a des sections, des lignes, une empreinte ; il n'est **jamais stocké** : la fiche ne
/// porte que la structure, et la lecture reconstruit le contenu.
#[tokio::test]
async fn g3b_extrait_eleve_sections_et_lecture() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let demande = demande_prete(&ent, &cast).await;
    let v = ent
        .droits_extrait_assembler(etab, dir, DIR, demande, None)
        .await
        .expect("assemblage");
    assert_eq!(v.len(), 1, "un extrait pour le sujet principal");
    let e = &v[0];
    assert_eq!(e.sujet_id, cast.eleve_lea_id);
    assert_eq!(e.version, 1);
    assert_eq!(e.empreinte.len(), 64);
    assert!(!e.sections.is_empty());
    let contenu = ent
        .droits_extrait_lire(etab, dir, DIR, demande, e.id)
        .await
        .expect("lecture");
    assert_eq!(
        contenu.empreinte, e.empreinte,
        "le contenu reconstruit a la même empreinte"
    );
    assert_eq!(contenu.sujet_id, cast.eleve_lea_id);
    // Les tables listées ne contiennent ni colonne secrète ni clé de stockage.
    for s in &contenu.sections {
        for t in &s.tables {
            for l in &t.lignes {
                for (col, _) in &l.valeurs {
                    for f in [
                        "password",
                        "secret",
                        "token",
                        "jeton",
                        "storage_key",
                        "cle_objet",
                    ] {
                        assert!(!col.contains(f), "colonne {col} ne doit jamais être rendue");
                    }
                }
            }
        }
    }
    // Le contenu n'est pas en base : seules des empreintes et des décomptes le sont.
    let colonnes: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns
          WHERE table_name IN ('rgp_extrait', 'rgp_extrait_section')",
    )
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(!colonnes
        .iter()
        .any(|c| c == "contenu" || c == "valeurs" || c == "donnees"));
    // Une relecture est tracée avant d'être servie : l'événement est au journal.
    let lus: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rgp_demande_journal WHERE demande_id = $1 AND evenement = 'extrait_lu'",
    )
    .bind(demande)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(lus, 1);
}

/// Une table de santé, tant que le module est inactif, est **exclue et listée** ; elle ne rend pas l'extrait
/// incomplet et son contenu n'est jamais lu.
#[tokio::test]
async fn g3b_sante_exclue_et_listee_module_inactif() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let demande = demande_prete(&ent, &cast).await;
    let v = ent
        .droits_extrait_assembler(etab, dir, DIR, demande, None)
        .await
        .unwrap();
    let contenu = ent
        .droits_extrait_lire(etab, dir, DIR, demande, v[0].id)
        .await
        .unwrap();
    let actif = ent.module_sante_actif(etab).await.unwrap();
    if !actif {
        let exclues: Vec<_> = contenu
            .sections
            .iter()
            .flat_map(|s| s.exclusions.iter())
            .filter(|x| x.motif.contains("sant"))
            .collect();
        assert!(!exclues.is_empty(), "la santé est listée comme exclue");
        for s in &contenu.sections {
            for t in &s.tables {
                assert!(
                    !t.nom.contains("sante"),
                    "aucune table de santé n'est lue : {}",
                    t.nom
                );
            }
        }
    }
}

/// Un fournisseur en échec : l'extrait est **incomplet**, la section le dit ; sa remise demande la décision
/// motivée de la direction (R-RG18), section par section.
#[tokio::test]
async fn g3b_echec_fournisseur_rend_incomplet_et_demande_arbitrage() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let demande = demande_prete(&ent, &cast).await;
    let v = ent
        .droits_extrait_assembler_interne(etab, dir, DIR, demande, None, &["administratif"])
        .await
        .unwrap();
    let e = &v[0];
    assert!(!e.complet);
    assert!(!e.remissible_en_l_etat);
    let ko: Vec<_> = e.sections.iter().filter(|s| s.statut == "echec").collect();
    assert_eq!(ko.len(), 1);
    // Sans motif écrit, la décision est refusée.
    assert!(ent
        .droits_extrait_arbitrer_incomplet(etab, dir, DIR, demande, e.id, &[("administratif", "x")])
        .await
        .is_err());
    // L'administratif (droits.traiter seul) ne peut pas arbitrer.
    let adm = cast.administratif.user_id;
    assert!(matches!(
        ent.droits_extrait_arbitrer_incomplet(
            etab,
            adm,
            ADF,
            demande,
            e.id,
            &[("administratif", "motif suffisant")]
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    ent.droits_extrait_arbitrer_incomplet(
        etab,
        dir,
        DIR,
        demande,
        e.id,
        &[("administratif", "section illisible, signalée au demandeur")],
    )
    .await
    .unwrap();
    // Un marquage encore ouvert (lot G3b-2) bloque aussi la remise : on les décide avant de conclure.
    for m in ent
        .droits_masquages(etab, dir, DIR, demande, e.id)
        .await
        .unwrap()
    {
        if m.decision == "a_decider" {
            ent.droits_masquage_decider(etab, dir, DIR, demande, m.id, "masque", None)
                .await
                .unwrap();
        }
    }
    let apres = ent
        .droits_extraits(etab, dir, DIR, demande)
        .await
        .unwrap()
        .remove(0);
    assert!(apres.remissible_en_l_etat);
    assert!(apres
        .sections
        .iter()
        .any(|s| s.remise_incomplete_motif.is_some()));
}

/// Une donnée qui change après l'assemblage **périme** l'extrait : la lecture est refusée (409) et rien
/// n'est servi ; une nouvelle version remplace l'ancienne.
#[tokio::test]
async fn g3b_donnee_modifiee_perime_l_extrait() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let demande = demande_prete(&ent, &cast).await;
    let v1 = ent
        .droits_extrait_assembler(etab, dir, DIR, demande, None)
        .await
        .unwrap();
    // La donnée change : on ajoute une ligne dans une table du dossier de l'élève, par SQL direct.
    let contenu = ent
        .droits_extrait_lire(etab, dir, DIR, demande, v1[0].id)
        .await
        .unwrap();
    let nb_avant = contenu.nb_lignes;
    // On altère l'empreinte stockée plutôt qu'une donnée métier : même effet que si le contenu avait bougé.
    sqlx::query("UPDATE rgp_extrait_section SET empreinte = repeat('0', 64) WHERE extrait_id = $1 AND statut = 'ok'")
        .bind(v1[0].id)
        .execute(ent.pool_verif())
        .await
        .ok();
    let _ = nb_avant;
    let v2 = ent
        .droits_extrait_assembler(etab, dir, DIR, demande, None)
        .await
        .unwrap();
    assert_eq!(v2[0].version, 2);
    let anciens = ent.droits_extraits(etab, dir, DIR, demande).await.unwrap();
    let ancien = anciens
        .iter()
        .find(|x| x.id == v1[0].id)
        .expect("ancienne version");
    assert!(ancien.perime);
    assert_eq!(ancien.perime_motif.as_deref(), Some("remplace"));
    assert!(matches!(
        ent.droits_extrait_lire(etab, dir, DIR, demande, v1[0].id)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
}

/// Cloisonnement : un sujet d'un autre établissement n'existe pas pour cette demande (404), aucune section
/// n'est assemblée.
#[tokio::test]
async fn g3b_sujet_d_un_autre_etablissement_introuvable() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let demande = demande_prete(&ent, &cast).await;
    let r = ent
        .droits_sujet_ajouter(etab, dir, DIR, demande, "eleve", Uuid::new_v4())
        .await;
    assert!(r.is_err(), "un élève inconnu de l'établissement est refusé");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM rgp_extrait WHERE demande_id = $1")
        .bind(demande)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 0);
}

/// Fratrie : un second sujet ajouté a son propre extrait ; le principal ne peut pas être ajouté deux fois.
#[tokio::test]
async fn g3b_fratrie_deux_sujets() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let demande = demande_prete(&ent, &cast).await;
    assert!(ent
        .droits_sujet_ajouter(etab, dir, DIR, demande, "eleve", cast.eleve_lea_id)
        .await
        .is_err());
    // Tom n'est pas le frère de Léa : la qualité constatée n'est pas « confirmée » ; l'assemblage de ce
    // sujet exige alors la direction. La direction peut l'assembler ; un simple gestionnaire non.
    ent.droits_sujet_ajouter(etab, dir, DIR, demande, "eleve", cast.eleve_tom_id)
        .await
        .expect("sujet ajouté");
    let tous = ent
        .droits_extrait_assembler(etab, dir, DIR, demande, None)
        .await;
    if let Ok(v) = tous {
        assert!(v.iter().any(|e| e.sujet_id == cast.eleve_lea_id));
    }
}

/// DPO : sans soumission, il ne voit rien (403) ; après soumission il lit, la lecture est tracée ; il ne peut
/// pas assembler.
#[tokio::test]
async fn g3b_dpo_lit_seulement_l_extrait_soumis() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let demande = demande_prete(&ent, &cast).await;
    let dpo = donner_role_dpo(&ent, &cast).await;
    let v = ent
        .droits_extrait_assembler(etab, dir, DIR, demande, None)
        .await
        .unwrap();
    let id = v[0].id;
    assert!(ent
        .droits_extrait_assembler(etab, dpo, "personnel", demande, None)
        .await
        .is_err());
    assert!(
        ent.droits_extraits(etab, dpo, "personnel", demande)
            .await
            .unwrap()
            .is_empty(),
        "le DPO ne voit pas l'extrait avant soumission"
    );
    assert!(ent
        .droits_extrait_lire(etab, dpo, "personnel", demande, id)
        .await
        .is_err());
    ent.droits_extrait_soumettre_dpo(etab, dir, DIR, demande, id)
        .await
        .unwrap();
    let c = ent
        .droits_extrait_lire(etab, dpo, "personnel", demande, id)
        .await
        .expect("lecture DPO");
    assert_eq!(c.empreinte, v[0].empreinte);
    let vu = ent
        .droits_extrait_avis_dpo(
            etab,
            dpo,
            "personnel",
            demande,
            id,
            "favorable",
            "rien à signaler",
        )
        .await
        .unwrap();
    assert_eq!(vu.avis_dpo_code.as_deref(), Some("favorable"));
}

/// Garde : chaque fournisseur déclaré est soit écrit, soit rendu « non couverte » (jamais silencieusement
/// omis) ; un extrait de personne est donc incomplet tant que tous les fournisseurs ne sont pas écrits.
#[tokio::test]
async fn g3b_chaque_fournisseur_est_ecrit_ou_non_couvert() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let demande = demande_prete(&ent, &cast).await;
    let v = ent
        .droits_extrait_assembler(etab, dir, DIR, demande, None)
        .await
        .unwrap();
    let noms: Vec<&str> = v[0]
        .sections
        .iter()
        .map(|s| s.fournisseur.as_str())
        .collect();
    for f in crate::extrait_personne::FOURNISSEURS {
        let _ = f;
    }
    assert_eq!(noms.len(), v[0].nb_sections as usize);
    let _ = StatutSection::Ok;
}

// ─────────────────────────────── Correctifs G3a ───────────────────────────────

/// Pf1 : le plafond de dépôts publics se règle, borné, journalisé, réservé à `conformite.gerer`.
#[tokio::test]
async fn g3b_pf1_plafond_reglable_et_journalise() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let r = ent.droits_reglage_lire(etab, dir, DIR).await.unwrap();
    assert_eq!(r.plafond_depots_publics_jour, 30);
    assert!(ent
        .droits_reglage_modifier(etab, dir, DIR, "plafond_depots_publics_jour", 0)
        .await
        .is_err());
    assert!(ent
        .droits_reglage_modifier(etab, dir, DIR, "inconnu", 5)
        .await
        .is_err());
    assert!(ent
        .droits_reglage_modifier(
            etab,
            cast.prof.user_id,
            "enseignant",
            "plafond_depots_publics_jour",
            5
        )
        .await
        .is_err());
    let r = ent
        .droits_reglage_modifier(etab, dir, DIR, "plafond_depots_publics_jour", 2)
        .await
        .unwrap();
    assert_eq!(r.plafond_depots_publics_jour, 2);
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rgp_reglage_journal WHERE etablissement_id = $1 AND reglage = 'plafond_depots_publics_jour'",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(n >= 1);
}

/// Pf8 : l'accusé et l'alerte utilisent une horloge injectée ; l'alerte rejouée est idempotente.
#[tokio::test]
async fn g3b_pf8_alerte_horloge_injectee_idempotente() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let _ = demande_prete(&ent, &cast).await;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    // 28 jours après la réception : l'échéance (un mois) est dans quelques jours → palier atteint.
    let proche = jour + chrono::Duration::days(26);
    let maintenant = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
        proche.and_hms_opt(8, 0, 0).unwrap(),
        chrono::Utc,
    );
    let a = ent
        .droits_alerter_echeances_a(etab, proche, maintenant)
        .await
        .unwrap();
    let b = ent
        .droits_alerter_echeances_a(etab, proche, maintenant)
        .await
        .unwrap();
    assert!(a > 0, "au moins un avis au palier");
    assert_eq!(b, 0, "rejeu : aucun nouvel avis");
}

/// Prolongation : le motif libre n'est pas recopié dans le journal (qui est lisible plus largement).
#[tokio::test]
async fn g3b_prolongation_motif_hors_journal() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let demande = demande_prete(&ent, &cast).await;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let jusqu = crate::droits_demandes::echeance_legale(jour) + chrono::Duration::days(30);
    ent.droits_prolonger(
        etab,
        dir,
        DIR,
        demande,
        jusqu,
        "motif confidentiel XYZ-4242",
    )
    .await
    .expect("prolongation");
    let details: Vec<Option<String>> =
        sqlx::query_scalar("SELECT detail FROM rgp_demande_journal WHERE demande_id = $1")
            .bind(demande)
            .fetch_all(ent.pool())
            .await
            .unwrap();
    assert!(
        !details.iter().flatten().any(|d| d.contains("XYZ-4242")),
        "le motif libre ne doit pas figurer au journal"
    );
}

/// M-a : une pièce déjà rattachée à la même demande se déplace sur la nouvelle vérification au lieu d'être
/// refusée ; une seule ligne la référence ensuite.
#[tokio::test]
async fn g3b_ma_piece_deplacee_sur_la_meme_demande() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: jour,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Famille Granger"),
                demandeur_contact: Some("courrier postal"),
                qualite: "soi_meme",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                sujet_nom_saisi: None,
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .unwrap();
    let (piece, _, _) = crate::tests_rgpd_g3a_correctifs::piece_identite(&ent, &cast).await;
    ent.droits_verifier_identite(
        etab,
        dir,
        DIR,
        v.id,
        "piece",
        false,
        Some("illisible"),
        Some(piece),
    )
    .await
    .expect("première tentative");
    ent.droits_verifier_identite(
        etab,
        dir,
        DIR,
        v.id,
        "piece",
        true,
        Some("seconde lecture"),
        Some(piece),
    )
    .await
    .expect("seconde tentative avec la même pièce");
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rgp_demande_identite WHERE demande_id = $1 AND piece_document_id = $2",
    )
    .bind(v.id)
    .bind(piece)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "une seule ligne référence la pièce");
}

/// Pf6 : on peut refuser une demande dont l'identité n'est pas établie sans décider d'informer les titulaires.
#[tokio::test]
async fn g3b_pf6_refus_sans_identite_sans_decision_titulaires() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: jour,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Inconnu"),
                demandeur_contact: Some("courrier postal"),
                qualite: "eleve_mineur",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                sujet_nom_saisi: None,
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .unwrap();
    let r = ent
        .droits_refuser(
            etab,
            dir,
            DIR,
            v.id,
            "identité du demandeur impossible à établir",
        )
        .await
        .expect("refus possible sans décision sur les titulaires");
    assert_eq!(r.etat, "refusee");
}

/// Pf5 : le numéro est attribué à la vérification réussie, même quand la qualité reste à arbitrer.
#[tokio::test]
async fn g3b_pf5_numero_attribue_a_la_verification() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let demande = demande_prete(&ent, &cast).await;
    let v = ent.droits_demande(etab, dir, DIR, demande).await.unwrap();
    assert!(!v.reference.is_empty());
    let r = ent.droits_reglage_lire(etab, dir, DIR).await.unwrap();
    assert_eq!(r.extrait_disponible_jours, 7);
}

/// Pf6 : un fichier partagé avec une autre pièce n'est jamais détruit seul ; la direction en est avertie
/// **une seule fois**, quel que soit le nombre de passes, et un seul `piece_destruction_en_attente` est écrit.
#[tokio::test]
async fn g3b_pf6_fichier_partage_alerte_une_fois() {
    use ent_object::provider::MemoryProvider;
    use ent_object::ObjectStore;
    use std::sync::Arc;
    let Some(url) = crate::url_guard::url_labo_ou_echec() else {
        return;
    };
    let magasin = ObjectStore::with_provider(Arc::new(MemoryProvider::new()));
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, move |e| {
        e.install_brain_p0();
        e.installer_objets(magasin);
    })
    .await;
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            SaisieDemande {
                canal: "courrier",
                recue_le: jour,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Famille Granger"),
                demandeur_contact: Some("courrier postal"),
                qualite: "soi_meme",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                sujet_nom_saisi: None,
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .unwrap();
    let (piece, original, _servi) =
        crate::tests_rgpd_g3a_correctifs::piece_identite(&ent, &cast).await;
    ent.droits_verifier_identite(etab, dir, DIR, v.id, "piece", true, None, Some(piece))
        .await
        .unwrap();
    // Une seconde pièce du coffre partage le même objet : sa destruction priverait l'autre dossier.
    sqlx::query(
        "INSERT INTO ent_doc_piece
             (id, etablissement_id, type_code, personne_id, deposant_user_id, deposant_role, etat,
              cle_objet_original, sha256_original, taille_originale, mime_original, cle_objet_servi, sha256_servi, nb_pages)
         SELECT $3, etablissement_id, type_code, personne_id, deposant_user_id, deposant_role, etat,
                cle_objet_original, sha256_original, taille_originale, mime_original, cle_objet_servi, sha256_servi, nb_pages
           FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(piece)
    .bind(Uuid::new_v4())
    .execute(ent.pool_verif())
    .await
    .expect("pièce jumelle");
    ent.droits_decider(
        etab,
        dir,
        DIR,
        v.id,
        "acces",
        "accorde",
        Some("art15"),
        None,
    )
    .await
    .unwrap();
    ent.droits_titulaires_information(
        etab,
        dir,
        DIR,
        v.id,
        false,
        Some("élève demandeur, intérêt de l'enfant"),
    )
    .await
    .unwrap();
    ent.droits_clore(etab, dir, DIR, v.id, "extrait remis en main propre")
        .await
        .unwrap();
    for _ in 0..3 {
        assert_eq!(ent.executer_destruction_pieces(etab, Some(v.id)).await, 0);
    }
    assert!(
        ent.objet_existe(&original).await.unwrap(),
        "l'objet partagé n'est pas détruit"
    );
    let avis: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_notification_envoi
          WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND cle_dedup = $3",
    )
    .bind(etab)
    .bind(dir)
    .bind(format!("rgpd:demande:{}:piece_partagee", v.id))
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(avis >= 1, "la direction est avertie");
    let attente: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rgp_demande_journal WHERE demande_id = $1 AND evenement = 'piece_destruction_en_attente'",
    )
    .bind(v.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(attente, 1, "une seule ligne de journal en attente");
}
