//! Socle 2, lot S2-3 : admissions, préinscriptions, inscriptions, réinscriptions,
//! import, contre la base locale de test (feature `labo`). Sans
//! `ENT_DATABASE_URL`, les scénarios sont ignorés.
//!
//! Labo : `parent_lea` est responsable de Harry (`eleve_lea_id`), `parent_tom`
//! de Draco (`eleve_tom_id`). Chaque `seed_labo_cast` crée un établissement neuf.

use super::*;
use crate::inscriptions_reactions::{
    CONSOMMATEUR_ACCUEIL, CONSOMMATEUR_AFFECTATION, CONSOMMATEUR_INVITATIONS,
};
use crate::outbox::{ConfigDistribution, Distributeur, ErreurRejeu};
use ent_auth::Role;
use ent_events::types;
use ent_types::inscriptions::*;
use std::time::Duration;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_base_de_test()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_mail_dry_run();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn distributeur(ent: &PgEnt, etab: Uuid) -> Distributeur {
    ent.distributeur(ConfigDistribution {
        taille_lot: 100,
        max_tentatives: 3,
        delai_base: Duration::ZERO,
        delai_max: Duration::ZERO,
        intervalle: Duration::from_millis(20),
        etablissement: Some(etab),
    })
}

fn dir(c: &LaboCast) -> (Uuid, &'static str) {
    (c.direction.user_id, Role::Direction.as_str())
}

fn adm(c: &LaboCast) -> (Uuid, &'static str) {
    (c.administratif.user_id, Role::Administratif.as_str())
}

async fn compter(ent: &PgEnt, sql: &str, id: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql)
        .bind(id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    n
}

/// Classe de l'année courante (créée au besoin, avec l'année).
async fn classe_courante(ent: &PgEnt, etab: Uuid, code: &str) -> Uuid {
    let annee: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante",
    )
    .bind(etab)
    .fetch_optional(ent.pool())
    .await
    .unwrap();
    let annee = match annee {
        Some((a,)) => a,
        None => {
            let a = Uuid::new_v4();
            let today = chrono::Utc::now().date_naive();
            sqlx::query(
                "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
                 VALUES ($1,$2,$3,$4,$5,true)",
            )
            .bind(a)
            .bind(etab)
            .bind(format!("courante-{}", &a.simple().to_string()[..6]))
            .bind(today - chrono::Duration::days(60))
            .bind(today + chrono::Duration::days(240))
            .execute(ent.pool())
            .await
            .unwrap();
            a
        }
    };
    let c: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_classe WHERE etablissement_id = $1 AND annee_id = $2 AND code = $3",
    )
    .bind(etab)
    .bind(annee)
    .bind(code)
    .fetch_optional(ent.pool())
    .await
    .unwrap();
    if let Some((c,)) = c {
        return c;
    }
    let c = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle) VALUES ($1,$2,$3,$4,$4)")
        .bind(c)
        .bind(etab)
        .bind(annee)
        .bind(code)
        .execute(ent.pool())
        .await
        .unwrap();
    c
}

fn campagne(nature: &str, capacite: Option<i32>) -> CampagneBody {
    CampagneBody {
        nature: nature.into(),
        libelle: format!("Campagne {nature}"),
        annee_cible: "2027-2028".into(),
        niveaux: vec![NiveauCampagneDto {
            code: "6EME".into(),
            capacite,
        }],
        pieces: vec![
            PieceCampagneDto {
                code: "livret".into(),
                libelle: "Livret scolaire".into(),
                obligatoire: true,
            },
            PieceCampagneDto {
                code: "photo".into(),
                libelle: "Photo".into(),
                obligatoire: false,
            },
        ],
        ..Default::default()
    }
}

async fn campagne_ouverte(ent: &PgEnt, c: &LaboCast, nature: &str, capacite: Option<i32>) -> Uuid {
    let (d, r) = dir(c);
    let camp = ent
        .creer_campagne_for_actor(c.etablissement_id, d, r, &campagne(nature, capacite))
        .await
        .unwrap();
    ent.ouvrir_campagne_for_actor(c.etablissement_id, d, r, camp.id)
        .await
        .unwrap();
    camp.id
}

/// Dernier code reçu par `email` (messagerie dry-run).
fn code_recu(ent: &PgEnt, email: &str) -> String {
    let (_, _, corps) = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .rev()
        .find(|(to, sujet, _)| to == email && sujet.contains("code de vérification"))
        .expect("code envoyé");
    let i = corps.find("est : ").expect("code dans le message") + "est : ".len();
    corps[i..i + 6].to_string()
}

/// Jeton reçu dans un lien `…?jeton=` envoyé à `email`.
fn jeton_invitation_recu(ent: &PgEnt, email: &str) -> String {
    let (_, _, corps) = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .rev()
        .find(|(to, sujet, _)| to == email && (sujet.contains("Invitation") || sujet.contains("Confirmez")))
        .expect("invitation envoyée");
    let i = corps.find("jeton=").expect("lien") + "jeton=".len();
    corps[i..].split_whitespace().next().unwrap().to_string()
}

async fn verifier(ent: &PgEnt, etab: Uuid, email: &str) -> String {
    let d = ent.demarrer_preinscription(etab, email).await.unwrap();
    let code = code_recu(ent, email);
    ent.verifier_preinscription(etab, d.demande_id, &code)
        .await
        .unwrap()
        .jeton
}

fn formulaire(
    campagne_id: Uuid,
    prenom: &str,
    second: Option<ResponsableFormulaireDto>,
) -> PreinscriptionBody {
    PreinscriptionBody {
        campagne_id,
        utiliser_mes_infos: false,
        enregistrer_profil: false,
        eleve: EleveFormulaireDto {
            nom: "Lovegood".into(),
            prenom: prenom.into(),
            date_naissance: chrono::NaiveDate::from_ymd_opt(chrono::Utc::now().year() - 11, 3, 4),
            sexe: Some("F".into()),
            niveau_demande: Some("6EME".into()),
            regime_demande: Some("demi_pension".into()),
            ..Default::default()
        },
        responsable: ResponsableFormulaireDto {
            nom: "Lovegood".into(),
            prenom: Some("Xenophilius".into()),
            lien: Some("pere".into()),
            meme_foyer: true,
            autorite_parentale: true,
            ..Default::default()
        },
        second_parent: second,
    }
}

use chrono::Datelike;

/// Admission de bout en bout : démarrage indiscernable, vérification, dépôt,
/// second parent séparé, instruction, décision, validation, réactions,
/// invitation acceptée par le second parent.
#[tokio::test]
async fn parcours_famille_de_bout_en_bout() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;

    // Anti-énumération : même réponse, même e-mail, que le compte existe ou non.
    let inconnu = format!("famille-{}@exemple.test", Uuid::new_v4().simple());
    let connu = c.parent_lea.email.clone();
    let r1 = ent.demarrer_preinscription(etab, &inconnu).await.unwrap();
    let r2 = ent.demarrer_preinscription(etab, &connu).await.unwrap();
    assert_eq!(r1.message, r2.message);
    assert_ne!(r1.demande_id, r2.demande_id);
    let sujets: Vec<String> = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .filter(|(to, s, _)| (to == &inconnu || to == &connu) && s.contains("code de vérification"))
        .map(|(_, s, corps)| format!("{s}|{}", corps.replace(char::is_numeric, "#")))
        .collect();
    assert_eq!(sujets.len(), 2, "un code envoyé dans les deux cas");
    assert_eq!(sujets[0], sujets[1], "messages indiscernables");
    // Aucune information sans vérification : mauvais code → erreur unique.
    let code = code_recu(&ent, &inconnu);
    let faux = if code == "000000" { "111111" } else { "000000" };
    let e = ent
        .verifier_preinscription(etab, r1.demande_id, faux)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("code invalide ou expiré"));
    // Même erreur pour une demande inexistante.
    let e2 = ent
        .verifier_preinscription(etab, Uuid::new_v4(), faux)
        .await
        .unwrap_err();
    assert_eq!(e.to_string(), e2.to_string());
    assert!(ent
        .espace_preinscription(etab, "jeton-forge")
        .await
        .is_err());
    let jeton = ent
        .verifier_preinscription(etab, r1.demande_id, &code)
        .await
        .unwrap()
        .jeton;
    // Un code ne sert qu'une fois.
    assert!(ent
        .verifier_preinscription(etab, r1.demande_id, &code)
        .await
        .is_err());
    let espace = ent.espace_preinscription(etab, &jeton).await.unwrap();
    assert!(!espace.connu && espace.profil.is_none());
    assert!(espace.campagnes.iter().any(|x| x.id == camp));
    assert!(espace.message_confidentialite.contains("Aucune donnée"));
    let code2 = code_recu(&ent, &connu);
    let jeton_connu = ent
        .verifier_preinscription(etab, r2.demande_id, &code2)
        .await
        .unwrap()
        .jeton;
    assert!(
        ent.espace_preinscription(etab, &jeton_connu)
            .await
            .unwrap()
            .connu
    );

    // Parents séparés sans e-mail → refus (D-014) ; avec e-mail → accepté.
    let second_email = format!("second-{}@exemple.test", Uuid::new_v4().simple());
    let mut second = ResponsableFormulaireDto {
        nom: "Lovegood".into(),
        prenom: Some("Pandora".into()),
        lien: Some("mere".into()),
        meme_foyer: false,
        autorite_parentale: true,
        ..Default::default()
    };
    let refus = ent
        .deposer_preinscription(
            etab,
            &jeton,
            &formulaire(camp, "Luna", Some(second.clone())),
        )
        .await;
    assert!(matches!(refus, Err(EntDbError::SaisieInvalide(_))));
    second.email = Some(second_email.clone());
    let dossier = ent
        .deposer_preinscription(etab, &jeton, &formulaire(camp, "Luna", Some(second)))
        .await
        .unwrap();
    assert_eq!(dossier.statut, "deposee");
    assert_eq!(dossier.responsables.len(), 2);
    assert_eq!(
        dossier.responsables[0].formulaire.email.as_deref(),
        Some(inconnu.as_str())
    );
    assert_eq!(dossier.pieces.len(), 2);
    // Un autre e-mail vérifié ne voit pas ce dossier.
    assert!(matches!(
        ent.dossier_famille(etab, &jeton_connu, dossier.id).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    assert!(ent
        .espace_preinscription(etab, &jeton_connu)
        .await
        .unwrap()
        .dossiers
        .is_empty());

    // Instruction (administratif) : étape, pièce refusée puis redéposée et acceptée.
    let (a, ar) = adm(&c);
    ent.changer_statut_for_actor(
        etab,
        a,
        ar,
        dossier.id,
        &StatutBody {
            statut: "en_instruction".into(),
            motif: None,
        },
    )
    .await
    .unwrap();
    assert!(ent
        .decider_piece_for_actor(
            etab,
            a,
            ar,
            dossier.id,
            "livret",
            &PieceDecisionBody {
                statut: "refusee".into(),
                motif: None
            }
        )
        .await
        .is_err());
    ent.deposer_piece_famille(etab, &jeton, dossier.id, "livret", "livret.pdf")
        .await
        .unwrap();
    ent.decider_piece_for_actor(
        etab,
        a,
        ar,
        dossier.id,
        "livret",
        &PieceDecisionBody {
            statut: "refusee".into(),
            motif: Some("illisible".into()),
        },
    )
    .await
    .unwrap();
    ent.commenter_for_actor(
        etab,
        a,
        ar,
        dossier.id,
        &CommentaireBody {
            texte: "note interne".into(),
            visible_famille: false,
        },
    )
    .await
    .unwrap();
    let vu_famille = ent.dossier_famille(etab, &jeton, dossier.id).await.unwrap();
    assert!(vu_famille
        .suivi
        .iter()
        .all(|s| s.texte.as_deref() != Some("note interne")));
    assert_eq!(
        vu_famille
            .pieces
            .iter()
            .find(|p| p.code == "livret")
            .unwrap()
            .statut,
        "refusee"
    );
    // Relance des pièces manquantes, pas deux fois de suite.
    assert_eq!(
        ent.relancer_for_actor(etab, a, ar, dossier.id)
            .await
            .unwrap(),
        1
    );
    assert!(ent
        .relancer_for_actor(etab, a, ar, dossier.id)
        .await
        .is_err());
    ent.deposer_piece_famille(etab, &jeton, dossier.id, "livret", "livret-v2.pdf")
        .await
        .unwrap();
    ent.decider_piece_for_actor(
        etab,
        a,
        ar,
        dossier.id,
        "livret",
        &PieceDecisionBody {
            statut: "acceptee".into(),
            motif: None,
        },
    )
    .await
    .unwrap();

    // Décision : réservée (administratif refusé), direction admet ; validation.
    let decision = DecisionBody {
        decision: "admis".into(),
        motif: None,
    };
    assert!(matches!(
        ent.decider_for_actor(etab, a, ar, dossier.id, &decision)
            .await,
        Err(EntDbError::Forbidden)
    ));
    let (d, dr) = dir(&c);
    // Coordonnées bancaires avant validation : refusées.
    assert!(ent
        .coordonnees_bancaires_famille(etab, &jeton, dossier.id)
        .await
        .is_err());
    ent.decider_for_actor(etab, d, dr, dossier.id, &decision)
        .await
        .unwrap();
    let classe = classe_courante(&ent, etab, "6A").await;
    let v = ent
        .valider_inscription_for_actor(
            etab,
            d,
            dr,
            dossier.id,
            &ValidationBody {
                classe_id: Some(classe),
                regime: None,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    // Le fait et l'événement sont dans la même transaction : l'événement est dans l'outbox.
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_outbox WHERE event_id = $1",
            v.evenement_id
        )
        .await,
        1
    );
    let (payload,): (serde_json::Value,) =
        sqlx::query_as("SELECT payload FROM ent_metier_event WHERE id = $1")
            .bind(v.evenement_id)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    let texte = payload.to_string();
    assert!(
        !texte.contains(&inconnu) && !texte.contains("Xenophilius"),
        "charge utile minimale"
    );
    // Élève, personnes, foyers distincts (parents séparés), liens.
    assert_eq!(
        compter(
            &ent,
            "SELECT count(DISTINCT foyer_id) FROM ent_eleve_lien WHERE eleve_id = $1",
            v.eleve_id
        )
        .await,
        2
    );
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_eleve_lien WHERE eleve_id = $1 AND est_payeur",
            v.eleve_id
        )
        .await,
        1
    );

    // Réactions.
    let dist = distributeur(&ent, etab);
    dist.distribuer_tout().await.unwrap();
    ent.expedier_envois_inscriptions(100).await.unwrap();
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_affectation WHERE eleve_id = $1 AND classe_id IS NOT NULL",
            v.eleve_id
        )
        .await,
        1
    );
    let (statut, classe_eleve): (String, Option<Uuid>) =
        sqlx::query_as("SELECT status, classe_id FROM ent_eleve WHERE id = $1")
            .bind(v.eleve_id)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!((statut.as_str(), classe_eleve), ("actif", Some(classe)));
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_inscription_invitation WHERE dossier_id = $1 AND statut = 'envoyee'", dossier.id).await,
        2,
        "déposant et second parent invités"
    );
    assert!(ent
        .mailer()
        .dry_run_corps()
        .iter()
        .any(|(to, s, _)| to == &inconnu && s.contains("Bienvenue")));

    // Le second parent accepte : compte créé, rattaché, accès à l'enfant.
    let jeton_inv = jeton_invitation_recu(&ent, &second_email);
    let acc = ent.accepter_invitation(etab, &jeton_inv).await.unwrap();
    assert_eq!(acc.resultat, "compte_cree");
    assert!(
        ent.accepter_invitation(etab, &jeton_inv).await.is_err(),
        "usage unique"
    );
    let (user2,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_user WHERE email = $1")
        .bind(&second_email)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(ent
        .lien_parent_eleve(etab, user2, v.eleve_id)
        .await
        .unwrap()
        .is_some());
    // La famille voit l'invitation acceptée dans son suivi.
    let vu = ent.dossier_famille(etab, &jeton, dossier.id).await.unwrap();
    assert_eq!(vu.statut, "validee");
    assert_eq!(vu.responsables[1].invitation.as_deref(), Some("acceptee"));
    assert!(vu.actions.contains(&"coordonnees_bancaires".to_string()));
}

/// IBAN d'un autre établissement reproposé masqué (identité réutilisée) ;
/// nouveau mandat signé dans le nouvel établissement.
#[tokio::test]
async fn iban_masque_et_nouveau_mandat() {
    let Some((ent, a)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let b = ent.seed_labo_cast().await.unwrap();
    // Dans A, parent_lea a un mandat actif sur son foyer (payeur).
    sqlx::query("UPDATE ent_eleve_lien SET est_payeur = true, part_paiement = 100 WHERE etablissement_id = $1 AND user_id = $2")
        .bind(a.etablissement_id)
        .bind(a.parent_lea.user_id)
        .execute(ent.pool())
        .await
        .unwrap();
    // Mandats éventuels du jeu de labo retirés : un seul mandat actif connu dans A.
    sqlx::query("UPDATE ent_fac_sepa_mandat SET statut = 'revoke' WHERE etablissement_id = $1")
        .bind(a.etablissement_id)
        .execute(ent.pool())
        .await
        .unwrap();
    ent.creer_mandat_sepa_for_actor(
        Role::Compta.as_str(),
        a.compta.user_id,
        domain_eleve::CreerMandatSepa {
            etablissement_id: a.etablissement_id,
            foyer_id: a.foyer_lea_id,
            rum: format!("RUM-{}", &Uuid::new_v4().simple().to_string()[..8]),
            iban: "FR7630006000011234567890189".into(),
            bic: None,
            titulaire: "Lily Potter".into(),
            signe_at: chrono::Utc::now(),
            actor_user_id: a.compta.user_id,
        },
    )
    .await
    .unwrap();

    // Dans B, la même personne dépose en réutilisant son identité.
    let camp = campagne_ouverte(&ent, &b, "admission", None).await;
    let jeton = verifier(&ent, b.etablissement_id, &a.parent_lea.email).await;
    let mut f = formulaire(camp, "Rose", None);
    f.utiliser_mes_infos = true;
    f.enregistrer_profil = true;
    let dossier = ent
        .deposer_preinscription(b.etablissement_id, &jeton, &f)
        .await
        .unwrap();
    assert!(dossier.reutilise_identite);
    // Profil enregistré à la demande de la personne : pré-remplissage ensuite.
    let espace = ent
        .espace_preinscription(b.etablissement_id, &jeton)
        .await
        .unwrap();
    assert_eq!(
        espace.profil.unwrap().prenom.as_deref(),
        Some("Xenophilius")
    );
    let (d, dr) = dir(&b);
    for code in ["livret"] {
        ent.deposer_piece_famille(b.etablissement_id, &jeton, dossier.id, code, "x.pdf")
            .await
            .unwrap();
        ent.decider_piece_for_actor(
            b.etablissement_id,
            d,
            dr,
            dossier.id,
            code,
            &PieceDecisionBody {
                statut: "acceptee".into(),
                motif: None,
            },
        )
        .await
        .unwrap();
    }
    ent.decider_for_actor(
        b.etablissement_id,
        d,
        dr,
        dossier.id,
        &DecisionBody {
            decision: "admis".into(),
            motif: None,
        },
    )
    .await
    .unwrap();
    ent.valider_inscription_for_actor(
        b.etablissement_id,
        d,
        dr,
        dossier.id,
        &ValidationBody::default(),
    )
    .await
    .unwrap();

    let cb = ent
        .coordonnees_bancaires_famille(b.etablissement_id, &jeton, dossier.id)
        .await
        .unwrap();
    assert_eq!(cb.propositions.len(), 1);
    assert_eq!(cb.propositions[0].iban_masque, "FR76 •••• 0189");
    assert!(
        !format!("{cb:?}").contains("30006000011234567890"),
        "IBAN jamais en clair"
    );
    let m = ent
        .demander_mandat_famille(
            b.etablissement_id,
            &jeton,
            dossier.id,
            &MandatBody {
                proposition_id: Some(cb.propositions[0].proposition_id),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (m.statut.as_str(), m.source.as_str()),
        ("a_signer", "reprise")
    );
    // Une proposition inventée est refusée.
    assert!(ent
        .demander_mandat_famille(
            b.etablissement_id,
            &jeton,
            dossier.id,
            &MandatBody {
                proposition_id: Some(Uuid::new_v4()),
                ..Default::default()
            },
        )
        .await
        .is_err());
    let signe = ent
        .signer_mandat_famille(b.etablissement_id, &jeton, dossier.id)
        .await
        .unwrap();
    assert_eq!(signe.statut, "signe");
    assert!(signe.rum.is_some());
    // Nouveau mandat dans B, chiffré ; A inchangé ; reprise tracée au niveau ENT.
    let (iban_b,): (String,) = sqlx::query_as(
        "SELECT iban FROM ent_fac_sepa_mandat WHERE etablissement_id = $1 AND statut = 'actif'",
    )
    .bind(b.etablissement_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(iban_b.starts_with("enc:"), "IBAN chiffré au repos");
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_fac_sepa_mandat WHERE etablissement_id = $1 AND statut = 'actif'",
            a.etablissement_id
        )
        .await,
        1,
        "établissement d'origine inchangé"
    );
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_identite_journal WHERE user_id = $1 AND action = 'iban_repris'",
            a.parent_lea.user_id
        )
        .await,
        1
    );
    // Sans réutilisation de l'identité, aucune proposition.
    let jeton2 = verifier(&ent, b.etablissement_id, &a.parent_lea.email).await;
    let mut f2 = formulaire(camp, "Lily", None);
    f2.eleve.nom = "Autre".into();
    let d2 = ent
        .deposer_preinscription(b.etablissement_id, &jeton2, &f2)
        .await
        .unwrap();
    assert!(!d2.reutilise_identite);
}

/// Capacité du niveau, droits par profil et rôle paramétré.
#[tokio::test]
async fn decision_capacite_et_droits() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let camp = campagne_ouverte(&ent, &c, "admission", Some(1)).await;
    let jeton = verifier(
        &ent,
        etab,
        &format!("cap-{}@exemple.test", Uuid::new_v4().simple()),
    )
    .await;
    let d1 = ent
        .deposer_preinscription(etab, &jeton, &formulaire(camp, "Un", None))
        .await
        .unwrap();
    let d2 = ent
        .deposer_preinscription(etab, &jeton, &formulaire(camp, "Deux", None))
        .await
        .unwrap();
    let (d, dr) = dir(&c);
    let admis = DecisionBody {
        decision: "admis".into(),
        motif: None,
    };
    ent.decider_for_actor(etab, d, dr, d1.id, &admis)
        .await
        .unwrap();
    assert!(matches!(
        ent.decider_for_actor(etab, d, dr, d2.id, &admis).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Liste d'attente : motif obligatoire.
    assert!(ent
        .decider_for_actor(
            etab,
            d,
            dr,
            d2.id,
            &DecisionBody {
                decision: "liste_attente".into(),
                motif: None
            }
        )
        .await
        .is_err());
    ent.decider_for_actor(
        etab,
        d,
        dr,
        d2.id,
        &DecisionBody {
            decision: "liste_attente".into(),
            motif: Some("complet".into()),
        },
    )
    .await
    .unwrap();
    // Validation impossible tant que les pièces obligatoires ne sont pas acceptées.
    assert!(matches!(
        ent.valider_inscription_for_actor(etab, d, dr, d1.id, &ValidationBody::default())
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Droits : prof, CPE, compta, parent n'accèdent pas à l'instruction.
    for (u, r) in [
        (c.prof.user_id, Role::Prof.as_str()),
        (c.cpe.user_id, Role::Cpe.as_str()),
        (c.compta.user_id, Role::Compta.as_str()),
        (c.parent_lea.user_id, Role::Parent.as_str()),
    ] {
        assert!(
            matches!(
                ent.list_dossiers_for_actor(etab, u, r, None, None).await,
                Err(EntDbError::Forbidden)
            ),
            "{r}"
        );
        assert!(
            matches!(
                ent.dossier_for_actor(etab, u, r, d1.id).await,
                Err(EntDbError::Forbidden)
            ),
            "{r}"
        );
        assert!(matches!(
            ent.creer_campagne_for_actor(etab, u, r, &campagne("admission", None))
                .await,
            Err(EntDbError::Forbidden)
        ));
    }
    let (a, ar) = adm(&c);
    let liste = ent
        .list_dossiers_for_actor(etab, a, ar, Some(camp), None)
        .await
        .unwrap();
    assert_eq!(liste.len(), 2);
    let vu = ent.dossier_for_actor(etab, a, ar, d1.id).await.unwrap();
    assert!(!vu.actions.contains(&"valider".to_string()));
    // Rôle paramétré (D-017) : la direction accorde `inscriptions.decider` au CPE.
    let role = ent
        .creer_role_for_actor(
            etab,
            c.admin.user_id,
            Role::Admin.as_str(),
            crate::RoleSaisie {
                nom: "Admissions".into(),
                description: None,
                modele: None,
                droits: vec![crate::Droit {
                    action: ent_auth::Action::InscriptionsDecider,
                    portee: ent_auth::Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(
        etab,
        c.admin.user_id,
        Role::Admin.as_str(),
        role.id,
        c.cpe.user_id,
        None,
    )
    .await
    .unwrap();
    assert!(ent
        .list_dossiers_for_actor(etab, c.cpe.user_id, Role::Cpe.as_str(), None, None)
        .await
        .is_ok());
}

/// Réinscription : dossiers préparés (idempotent), confirmation par le responsable
/// lié, départ annoncé, validation, message d'accueil, cloisonnement des familles.
#[tokio::test]
async fn reinscription_campagne() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = ent
        .creer_campagne_for_actor(
            etab,
            d,
            dr,
            &CampagneBody {
                pieces: vec![],
                ..campagne("reinscription", None)
            },
        )
        .await
        .unwrap();
    let o = ent
        .ouvrir_campagne_for_actor(etab, d, dr, camp.id)
        .await
        .unwrap();
    let actifs = compter(
        &ent,
        "SELECT count(*) FROM ent_eleve WHERE etablissement_id = $1 AND status = 'actif'",
        etab,
    )
    .await;
    assert_eq!(o.dossiers_prepares, actifs);
    // Rouvrir ne duplique rien.
    ent.fermer_campagne_for_actor(etab, d, dr, camp.id)
        .await
        .unwrap();
    let o2 = ent
        .ouvrir_campagne_for_actor(etab, d, dr, camp.id)
        .await
        .unwrap();
    assert_eq!(o2.dossiers_prepares, 0);
    // Une seconde campagne de même nature et année ne s'ouvre pas.
    let autre = ent
        .creer_campagne_for_actor(etab, d, dr, &campagne("reinscription", None))
        .await
        .unwrap();
    assert!(matches!(
        ent.ouvrir_campagne_for_actor(etab, d, dr, autre.id).await,
        Err(EntDbError::ConflitDossier(_))
    ));

    let lea = (c.parent_lea.user_id, Role::Parent.as_str());
    let tom = (c.parent_tom.user_id, Role::Parent.as_str());
    let mes = ent
        .reinscriptions_famille(etab, lea.0, lea.1)
        .await
        .unwrap();
    assert!(mes.iter().any(|x| x.eleve_id == Some(c.eleve_lea_id)));
    assert!(
        mes.iter().all(|x| x.eleve_id != Some(c.eleve_tom_id)),
        "seulement ses enfants"
    );
    let harry = mes
        .iter()
        .find(|x| x.eleve_id == Some(c.eleve_lea_id))
        .unwrap();
    assert!(!harry.responsables.is_empty(), "responsables pré-remplis");
    // Un autre responsable ne peut pas confirmer pour Harry.
    assert!(ent
        .confirmer_reinscription_famille(
            etab,
            tom.0,
            tom.1,
            harry.id,
            &ConfirmationReinscriptionBody::default()
        )
        .await
        .is_err());
    let conf = ent
        .confirmer_reinscription_famille(
            etab,
            lea.0,
            lea.1,
            harry.id,
            &ConfirmationReinscriptionBody {
                regime: Some("interne".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(conf.statut, "deposee");
    // Draco : départ annoncé.
    let draco = ent
        .reinscriptions_famille(etab, tom.0, tom.1)
        .await
        .unwrap()
        .into_iter()
        .find(|x| x.eleve_id == Some(c.eleve_tom_id))
        .unwrap();
    let dep = ent
        .annoncer_depart_famille(
            etab,
            tom.0,
            tom.1,
            draco.id,
            &DepartBody {
                motif: "déménagement".into(),
                date_depart: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(dep.statut, "desistement");
    assert!(ent
        .valider_inscription_for_actor(etab, d, dr, draco.id, &ValidationBody::default())
        .await
        .is_err());
    // Validation de Harry, classe de l'année courante.
    let classe = classe_courante(&ent, etab, "5B").await;
    let v = ent
        .valider_inscription_for_actor(
            etab,
            d,
            dr,
            harry.id,
            &ValidationBody {
                classe_id: Some(classe),
                regime: None,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(v.eleve_id, c.eleve_lea_id);
    distributeur(&ent, etab).distribuer_tout().await.unwrap();
    let notes = ent.mes_notifications(etab, lea.0, 50).await.unwrap();
    assert!(notes.iter().any(|n| n.kind == "reinscription_validee"));
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_inscription_invitation WHERE dossier_id = $1",
            harry.id
        )
        .await,
        0,
        "responsables déjà titulaires d'un compte : pas d'invitation"
    );
    // Événements de la famille dans la chronologie (titres neutres).
    let types_publies: Vec<(String,)> = sqlx::query_as(
        "SELECT type FROM ent_metier_event WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(etab)
    .bind(c.eleve_tom_id)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(types_publies
        .iter()
        .any(|(t,)| t == types::SCOLARITE_DEPART_ANNONCE));
}

/// Réactions de l'outbox : idempotence (double distribution, relivraison forcée),
/// consommateurs à envoi externe non rejouables.
#[tokio::test]
async fn reactions_idempotentes() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let email = format!("idem-{}@exemple.test", Uuid::new_v4().simple());
    let jeton = verifier(&ent, etab, &email).await;
    let dossier = ent
        .deposer_preinscription(etab, &jeton, &formulaire(camp, "Idem", None))
        .await
        .unwrap();
    let (d, dr) = dir(&c);
    ent.deposer_piece_famille(etab, &jeton, dossier.id, "livret", "l.pdf")
        .await
        .unwrap();
    ent.decider_piece_for_actor(
        etab,
        d,
        dr,
        dossier.id,
        "livret",
        &PieceDecisionBody {
            statut: "acceptee".into(),
            motif: None,
        },
    )
    .await
    .unwrap();
    ent.decider_for_actor(
        etab,
        d,
        dr,
        dossier.id,
        &DecisionBody {
            decision: "admis".into(),
            motif: None,
        },
    )
    .await
    .unwrap();
    let classe = classe_courante(&ent, etab, "6C").await;
    let v = ent
        .valider_inscription_for_actor(
            etab,
            d,
            dr,
            dossier.id,
            &ValidationBody {
                classe_id: Some(classe),
                regime: None,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let dist = distributeur(&ent, etab);
    dist.distribuer_tout().await.unwrap();
    dist.distribuer_tout().await.unwrap();
    ent.expedier_envois_inscriptions(100).await.unwrap();
    let mails = |ent: &PgEnt| {
        ent.mailer()
            .dry_run_corps()
            .iter()
            .filter(|(to, _, _)| to == &email)
            .count()
    };
    let avant = mails(&ent);
    let etat = |pool: sqlx::PgPool| async move {
        let (a,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM ent_affectation WHERE eleve_id = $1")
                .bind(v.eleve_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let (i,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM ent_inscription_invitation WHERE dossier_id = $1")
                .bind(dossier.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        (a, i)
    };
    assert_eq!(etat(ent.pool().clone()).await, (1, 1));
    for nom in [
        CONSOMMATEUR_AFFECTATION,
        CONSOMMATEUR_INVITATIONS,
        CONSOMMATEUR_ACCUEIL,
    ] {
        assert_eq!(
            compter(
                &ent,
                &format!("SELECT count(*) FROM ent_event_traitement WHERE event_id = $1 AND consommateur = '{nom}'"),
                v.evenement_id
            )
            .await,
            1,
            "{nom}"
        );
    }
    // Relivraison forcée (marques effacées, événement remis en file) : aucun doublon,
    // aucun nouvel envoi.
    sqlx::query("DELETE FROM ent_event_traitement WHERE event_id = $1")
        .bind(v.evenement_id)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_outbox SET statut = 'a_distribuer', prochaine_tentative = now() WHERE event_id = $1")
        .bind(v.evenement_id)
        .execute(ent.pool())
        .await
        .unwrap();
    dist.distribuer_tout().await.unwrap();
    ent.expedier_envois_inscriptions(100).await.unwrap();
    assert_eq!(etat(ent.pool().clone()).await, (1, 1));
    assert_eq!(
        mails(&ent),
        avant,
        "clé d'envoi unique : la relivraison ne renvoie aucun message"
    );
    // Rejouer un consommateur à envoi externe est refusé ; l'affectation est rejouable.
    assert!(matches!(
        dist.rejouer_consommateur(CONSOMMATEUR_INVITATIONS, etab, 0)
            .await,
        Err(ErreurRejeu::NonRejouable)
    ));
    dist.rejouer_consommateur(CONSOMMATEUR_AFFECTATION, etab, 0)
        .await
        .unwrap();
    dist.distribuer_tout().await.unwrap();
    assert_eq!(etat(ent.pool().clone()).await, (1, 1));
}

/// Import : prévisualisation, rejets ligne à ligne, application, rejets traités,
/// réimport sans doublon ni écrasement.
#[tokio::test]
async fn import_avec_rejets() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    classe_courante(&ent, etab, "6D").await;
    let s = Uuid::new_v4().simple().to_string()[..6].to_string();
    let csv = format!(
        "nom;prenom;date_naissance;sexe;classe;resp1_nom;resp1_email\n\
Weasley{s};Fred;01/04/2014;M;6D;Weasley;molly-{s}@exemple.test\n\
Weasley{s};George;01/04/2014;M;6D;Weasley;molly-{s}@exemple.test\n\
Weasley{s};Percy;31/02/2013;M;6D;;\n\
Weasley{s};Fred;01/04/2014;M;6D;;\n\
Weasley{s};Bill;01/04/2010;M;ZZ;;\n"
    );
    let body = ImportBody {
        nom_fichier: "eleves.csv".into(),
        contenu: csv.clone(),
    };
    assert!(matches!(
        ent.previsualiser_import_for_actor(etab, c.prof.user_id, Role::Prof.as_str(), &body)
            .await,
        Err(EntDbError::Forbidden)
    ));
    let (a, ar) = adm(&c);
    let lot = ent
        .previsualiser_import_for_actor(etab, a, ar, &body)
        .await
        .unwrap();
    assert_eq!((lot.nb_lignes, lot.nb_a_creer, lot.nb_rejets), (5, 2, 3));
    let percy = lot
        .lignes
        .iter()
        .find(|l| l.eleve_prenom == "Percy")
        .unwrap();
    assert!(percy.motifs.iter().any(|m| m.contains("date de naissance")));
    assert!(lot
        .lignes
        .iter()
        .any(|l| l.motifs.iter().any(|m| m.contains("doublon"))));
    assert!(lot
        .lignes
        .iter()
        .any(|l| l.motifs.iter().any(|m| m.contains("classe inconnue"))));
    let eleves = "SELECT count(*) FROM ent_eleve WHERE etablissement_id = $1";
    let avant = compter(&ent, eleves, etab).await;
    assert_eq!(
        compter(&ent, eleves, etab).await,
        avant,
        "prévisualiser n'écrit aucun élève"
    );
    let lot = ent
        .appliquer_import_for_actor(etab, a, ar, lot.id)
        .await
        .unwrap();
    assert_eq!((lot.nb_creees, lot.nb_rejets), (2, 3));
    assert_eq!(compter(&ent, eleves, etab).await, avant + 2);
    assert!(
        ent.appliquer_import_for_actor(etab, a, ar, lot.id)
            .await
            .is_err(),
        "une seule application"
    );
    // Fratrie : une seule personne pour Molly, liée aux deux enfants.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND email = $2",
    )
    .bind(etab)
    .bind(format!("molly-{s}@exemple.test"))
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);
    // Rejets : Percy corrigé (créé aussitôt), doublon ignoré.
    let corr = ent
        .traiter_ligne_import_for_actor(
            etab,
            a,
            ar,
            percy.id,
            &TraiterLigneBody {
                action: "corriger".into(),
                corrections: [("date_naissance".to_string(), "28/02/2013".to_string())]
                    .into_iter()
                    .collect(),
            },
        )
        .await
        .unwrap();
    assert_eq!(corr.statut, "creee");
    assert!(corr.eleve_id.is_some());
    let doublon = lot
        .lignes
        .iter()
        .find(|l| l.motifs.iter().any(|m| m.contains("doublon")))
        .unwrap();
    let ign = ent
        .traiter_ligne_import_for_actor(
            etab,
            a,
            ar,
            doublon.id,
            &TraiterLigneBody {
                action: "ignorer".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(ign.statut, "ignoree");
    assert!(ent
        .traiter_ligne_import_for_actor(
            etab,
            a,
            ar,
            doublon.id,
            &TraiterLigneBody {
                action: "ignorer".into(),
                ..Default::default()
            }
        )
        .await
        .is_err());
    // Réimport du même fichier : élèves « existants », jamais modifiés.
    let lot2 = ent
        .previsualiser_import_for_actor(etab, a, ar, &body)
        .await
        .unwrap();
    assert_eq!(lot2.nb_existantes, 2);
    assert_eq!(lot2.nb_a_creer, 0);
    let evts = compter(
        &ent,
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'import.lot_applique'",
        etab,
    )
    .await;
    assert_eq!(evts, 1);
}

/// Isolation entre établissements : dossiers, jetons, invitations, lots.
#[tokio::test]
async fn isolation_etablissements() {
    let Some((ent, a)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let b = ent.seed_labo_cast().await.unwrap();
    let camp = campagne_ouverte(&ent, &a, "admission", None).await;
    let email = format!("iso-{}@exemple.test", Uuid::new_v4().simple());
    let jeton = verifier(&ent, a.etablissement_id, &email).await;
    let dossier = ent
        .deposer_preinscription(a.etablissement_id, &jeton, &formulaire(camp, "Iso", None))
        .await
        .unwrap();
    let (db_, dbr) = dir(&b);
    // Le jeton de A ne vaut rien dans B ; la campagne de A n'existe pas dans B.
    assert!(ent
        .espace_preinscription(b.etablissement_id, &jeton)
        .await
        .is_err());
    let jeton_b = verifier(&ent, b.etablissement_id, &email).await;
    assert!(matches!(
        ent.deposer_preinscription(b.etablissement_id, &jeton_b, &formulaire(camp, "Iso", None))
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    assert!(ent
        .espace_preinscription(b.etablissement_id, &jeton_b)
        .await
        .unwrap()
        .dossiers
        .is_empty());
    // La direction de B ne voit ni ne touche le dossier de A.
    assert!(matches!(
        ent.dossier_for_actor(b.etablissement_id, db_, dbr, dossier.id)
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    assert!(ent
        .list_dossiers_for_actor(b.etablissement_id, db_, dbr, None, None)
        .await
        .unwrap()
        .is_empty());
    assert!(ent
        .decider_for_actor(
            b.etablissement_id,
            db_,
            dbr,
            dossier.id,
            &DecisionBody {
                decision: "admis".into(),
                motif: None
            }
        )
        .await
        .is_err());
    assert!(ent
        .list_campagnes_for_actor(b.etablissement_id, db_, dbr)
        .await
        .unwrap()
        .iter()
        .all(|x| x.id != camp));
    // Classe d'un autre établissement refusée à la validation.
    let (da, dar) = dir(&a);
    ent.deposer_piece_famille(a.etablissement_id, &jeton, dossier.id, "livret", "l.pdf")
        .await
        .unwrap();
    ent.decider_piece_for_actor(
        a.etablissement_id,
        da,
        dar,
        dossier.id,
        "livret",
        &PieceDecisionBody {
            statut: "acceptee".into(),
            motif: None,
        },
    )
    .await
    .unwrap();
    ent.decider_for_actor(
        a.etablissement_id,
        da,
        dar,
        dossier.id,
        &DecisionBody {
            decision: "admis".into(),
            motif: None,
        },
    )
    .await
    .unwrap();
    let classe_b = classe_courante(&ent, b.etablissement_id, "6Z").await;
    assert!(ent
        .valider_inscription_for_actor(
            a.etablissement_id,
            da,
            dar,
            dossier.id,
            &ValidationBody {
                classe_id: Some(classe_b),
                regime: None,
                ..Default::default()
            }
        )
        .await
        .is_err());
    ent.valider_inscription_for_actor(
        a.etablissement_id,
        da,
        dar,
        dossier.id,
        &ValidationBody::default(),
    )
    .await
    .unwrap();
    // Un distributeur borné à B ne traite pas les événements de A ; l'invitation
    // de A n'est pas acceptable dans B.
    distributeur(&ent, b.etablissement_id)
        .distribuer_tout()
        .await
        .unwrap();
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_inscription_invitation WHERE dossier_id = $1",
            dossier.id
        )
        .await,
        0
    );
    distributeur(&ent, a.etablissement_id)
        .distribuer_tout()
        .await
        .unwrap();
    ent.expedier_envois_inscriptions(100).await.unwrap();
    let inv = jeton_invitation_recu(&ent, &email);
    assert!(ent
        .accepter_invitation(b.etablissement_id, &inv)
        .await
        .is_err());
    assert_eq!(
        ent.accepter_invitation(a.etablissement_id, &inv)
            .await
            .unwrap()
            .resultat,
        "compte_cree"
    );
    // Lot d'import de A invisible depuis B.
    let (aa, aar) = adm(&a);
    let lot = ent
        .previsualiser_import_for_actor(
            a.etablissement_id,
            aa,
            aar,
            &ImportBody {
                nom_fichier: "a.csv".into(),
                contenu: "nom;prenom;date_naissance\nX;Y;01/01/2014\n".into(),
            },
        )
        .await
        .unwrap();
    let (ab, abr) = adm(&b);
    assert!(ent
        .lot_import_for_actor(b.etablissement_id, ab, abr, lot.id)
        .await
        .is_err());
    assert!(ent
        .appliquer_import_for_actor(b.etablissement_id, ab, abr, lot.id)
        .await
        .is_err());
}

// ---------------------------------------------------------------------------
// Lot I0 (socle 3) : rapprochement humain, fratrie, ancien élève, envois, HMAC
// ---------------------------------------------------------------------------

/// Dossier d'admission déposé avec `email_deposant` vérifié, instruit et admis.
/// Renvoie (id du dossier, jeton famille).
async fn dossier_admis(
    ent: &PgEnt,
    c: &LaboCast,
    camp: Uuid,
    email_deposant: &str,
    f: PreinscriptionBody,
) -> (Uuid, String) {
    let etab = c.etablissement_id;
    let jeton = verifier(ent, etab, email_deposant).await;
    let dossier = ent.deposer_preinscription(etab, &jeton, &f).await.unwrap();
    let (a, ar) = adm(c);
    ent.changer_statut_for_actor(
        etab,
        a,
        ar,
        dossier.id,
        &StatutBody {
            statut: "en_instruction".into(),
            motif: None,
        },
    )
    .await
    .unwrap();
    for p in dossier.pieces.iter().filter(|p| p.obligatoire) {
        ent.deposer_piece_famille(etab, &jeton, dossier.id, &p.code, "p.pdf")
            .await
            .unwrap();
        ent.decider_piece_for_actor(
            etab,
            a,
            ar,
            dossier.id,
            &p.code,
            &PieceDecisionBody {
                statut: "acceptee".into(),
                motif: None,
            },
        )
        .await
        .unwrap();
    }
    let (d, dr) = dir(c);
    ent.decider_for_actor(
        etab,
        d,
        dr,
        dossier.id,
        &DecisionBody {
            decision: "admis".into(),
            motif: None,
        },
    )
    .await
    .unwrap();
    (dossier.id, jeton)
}

async fn resp_id(ent: &PgEnt, dossier: Uuid, rang: i16) -> Uuid {
    let (id,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_inscription_responsable WHERE dossier_id = $1 AND rang = $2",
    )
    .bind(dossier)
    .bind(rang)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    id
}

fn second_parent(email: &str) -> ResponsableFormulaireDto {
    ResponsableFormulaireDto {
        nom: "Ollivander".into(),
        prenom: Some("Garrick".into()),
        lien: Some("pere".into()),
        email: Some(email.into()),
        meme_foyer: false,
        autorite_parentale: true,
        ..Default::default()
    }
}

async fn personne_de(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    let (p,): (Uuid,) =
        sqlx::query_as("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2")
            .bind(etab)
            .bind(user)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    p
}

fn valider_corps(resp: Vec<ResolutionResponsable>, eleve: Option<ChoixEleve>) -> ValidationBody {
    ValidationBody {
        responsables: resp,
        eleve,
        ..Default::default()
    }
}

/// TEST D'ATTAQUE (§2.2 pt 1) : un dossier qui nomme l'e-mail d'un parent existant ne
/// lui donne AUCUN accès — ni à la validation, ni tant qu'il n'a pas confirmé lui-même.
#[tokio::test]
async fn attaque_email_parent_existant_aucun_acces() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let lea = c.parent_lea.user_id;
    let lea_personne = personne_de(&ent, etab, lea).await;
    let deposant = format!("deposant-{}@exemple.test", Uuid::new_v4().simple());
    // Le dossier désigne comme second parent l'adresse d'un parent déjà connu.
    let (dossier, _) = dossier_admis(
        &ent,
        &c,
        camp,
        &deposant,
        formulaire(camp, "Rowena", Some(second_parent(&c.parent_lea.email))),
    )
    .await;

    // La validation présente la correspondance ; sans décision humaine, elle est refusée
    // et RIEN n'est créé (ni élève, ni personne, ni lien).
    let corr = ent
        .correspondances_for_actor(etab, d, dr, dossier)
        .await
        .unwrap();
    let rang2 = corr.responsables.iter().find(|r| r.rang == 2).unwrap();
    let cand = rang2
        .candidats
        .iter()
        .find(|p| p.personne_id == lea_personne)
        .expect("la fiche existante est proposée");
    assert!(cand.a_un_compte && cand.raison == "email");
    assert!(
        !cand.email_masque.as_deref().unwrap_or("").contains(&c.parent_lea.email),
        "adresse masquée"
    );
    let r = ent
        .valider_inscription_for_actor(etab, d, dr, dossier, &ValidationBody::default())
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_eleve WHERE etablissement_id = $1 AND prenom = 'Rowena'",
            etab
        )
        .await,
        0
    );
    // Aucun e-mail n'est parti pour un fait annulé.
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_inscription_envoi WHERE etablissement_id = $1",
            etab
        )
        .await,
        0
    );

    // Une personne de l'établissement confirme le rapprochement.
    let resp2 = resp_id(&ent, dossier, 2).await;
    let v = ent
        .valider_inscription_for_actor(
            etab,
            d,
            dr,
            dossier,
            &valider_corps(
                vec![ResolutionResponsable {
                    responsable_id: resp2,
                    personne: ChoixPersonne::Existante {
                        personne_id: lea_personne,
                    },
                    foyer: Some(ChoixFoyer::Nouveau),
                }],
                None,
            ),
        )
        .await
        .unwrap();
    // Le lien est EN ATTENTE : aucun accès, aucune ligne de lien, aucune lecture possible.
    assert!(ent
        .lien_parent_eleve(etab, lea, v.eleve_id)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        compter(
            &ent,
            &format!("SELECT count(*) FROM ent_eleve_lien WHERE personne_id = '{lea_personne}' AND eleve_id = $1"),
            v.eleve_id
        )
        .await,
        0
    );
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_lien_attente WHERE source = 'inscription' AND source_ref IN (SELECT id FROM ent_inscription_responsable WHERE dossier_id = $1) AND statut = 'en_attente'",
            dossier
        )
        .await,
        1
    );
    // Rien ne part avant la validation de la transaction du distributeur, puis la file part.
    let dist = distributeur(&ent, etab);
    let avant = ent.mailer().dry_run_corps().len();
    dist.distribuer_tout().await.unwrap();
    assert_eq!(ent.mailer().dry_run_corps().len(), avant, "rien n'est envoyé dans la transaction");
    ent.expedier_envois_inscriptions(100).await.unwrap();
    // La confirmation est demandée au titulaire du compte, à l'adresse DU COMPTE.
    let (_, sujet, _) = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .rev()
        .find(|(to, s, _)| to == &c.parent_lea.email && s.contains("Confirmez"))
        .expect("demande de confirmation envoyée au parent existant");
    assert!(sujet.contains("Confirmez"));
    // Toujours aucun accès tant qu'elle n'a pas confirmé.
    assert!(ent.lien_parent_eleve(etab, lea, v.eleve_id).await.unwrap().is_none());
    // Elle confirme : le lien devient actif, avec les droits du paramétrage (autorité → oui).
    let jeton = jeton_invitation_recu(&ent, &c.parent_lea.email);
    let acc = ent.accepter_invitation(etab, &jeton).await.unwrap();
    assert_eq!(acc.resultat, "compte_rattache");
    assert!(ent.lien_parent_eleve(etab, lea, v.eleve_id).await.unwrap().is_some());
    assert!(ent.accepter_invitation(etab, &jeton).await.is_err(), "usage unique");
}

/// Un parent existant qui refuse : aucun lien n'est jamais créé.
#[tokio::test]
async fn refus_du_parent_existant_aucun_lien() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let lea = c.parent_lea.user_id;
    let lea_personne = personne_de(&ent, etab, lea).await;
    let (dossier, _) = dossier_admis(
        &ent,
        &c,
        camp,
        &format!("dep-{}@exemple.test", Uuid::new_v4().simple()),
        formulaire(camp, "Fleur", Some(second_parent(&c.parent_lea.email))),
    )
    .await;
    let resp2 = resp_id(&ent, dossier, 2).await;
    let v = ent
        .valider_inscription_for_actor(
            etab,
            d,
            dr,
            dossier,
            &valider_corps(
                vec![ResolutionResponsable {
                    responsable_id: resp2,
                    personne: ChoixPersonne::Existante {
                        personne_id: lea_personne,
                    },
                    foyer: Some(ChoixFoyer::Nouveau),
                }],
                None,
            ),
        )
        .await
        .unwrap();
    distributeur(&ent, etab).distribuer_tout().await.unwrap();
    ent.expedier_envois_inscriptions(100).await.unwrap();
    let jeton = jeton_invitation_recu(&ent, &c.parent_lea.email);
    ent.refuser_invitation(etab, &jeton).await.unwrap();
    assert!(ent.accepter_invitation(etab, &jeton).await.is_err());
    assert!(ent.lien_parent_eleve(etab, lea, v.eleve_id).await.unwrap().is_none());
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_lien_attente WHERE source = 'inscription' AND source_ref IN (SELECT id FROM ent_inscription_responsable WHERE dossier_id = $1) AND statut = 'refuse'",
            dossier
        )
        .await,
        1
    );
}

/// Fratrie (§2.2 pt 2) : un foyer existant est proposé et confirmé, pas de nouveau foyer.
#[tokio::test]
async fn fratrie_foyer_existant_confirme() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let lea_personne = personne_de(&ent, etab, c.parent_lea.user_id).await;
    // Léa (déjà connue, avec un foyer) inscrit un autre enfant.
    let mut f = formulaire(camp, "Ginny", None);
    f.responsable.nom = "Lea".into();
    let (dossier, _) = dossier_admis(&ent, &c, camp, &c.parent_lea.email, f).await;
    let resp1 = resp_id(&ent, dossier, 1).await;
    let corr = ent
        .correspondances_for_actor(etab, d, dr, dossier)
        .await
        .unwrap();
    let cand = corr.responsables[0]
        .candidats
        .iter()
        .find(|p| p.personne_id == lea_personne)
        .expect("fiche connue proposée");
    assert!(cand.foyers.iter().any(|f| f.foyer_id == c.foyer_lea_id), "foyer proposé");
    let foyers_avant = compter(&ent, "SELECT count(*) FROM ent_foyer WHERE etablissement_id = $1", etab).await;
    // Sans confirmation du foyer : refus.
    let sans_foyer = valider_corps(
        vec![ResolutionResponsable {
            responsable_id: resp1,
            personne: ChoixPersonne::Existante {
                personne_id: lea_personne,
            },
            foyer: None,
        }],
        None,
    );
    assert!(ent
        .valider_inscription_for_actor(etab, d, dr, dossier, &sans_foyer)
        .await
        .is_err());
    let avec_foyer = valider_corps(
        vec![ResolutionResponsable {
            responsable_id: resp1,
            personne: ChoixPersonne::Existante {
                personne_id: lea_personne,
            },
            foyer: Some(ChoixFoyer::Existant {
                foyer_id: c.foyer_lea_id,
            }),
        }],
        None,
    );
    let v = ent
        .valider_inscription_for_actor(etab, d, dr, dossier, &avec_foyer)
        .await
        .unwrap();
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_foyer WHERE etablissement_id = $1", etab).await,
        foyers_avant,
        "aucun nouveau foyer"
    );
    // Le lien (en attente de confirmation de Léa) pointera vers le foyer existant.
    let (foyer,): (Option<Uuid>,) = sqlx::query_as(
        "SELECT foyer_id FROM ent_lien_attente WHERE source = 'inscription' AND source_ref IN (SELECT id FROM ent_inscription_responsable WHERE dossier_id = $1)",
    )
    .bind(dossier)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(foyer, Some(c.foyer_lea_id));
    assert!(ent.lien_parent_eleve(etab, c.parent_lea.user_id, v.eleve_id).await.unwrap().is_none());
}

/// Doublon d'élève (§2.2 pt 3) : plus de refus sec, rattachement à la fiche existante.
#[tokio::test]
async fn ancien_eleve_rattache_a_la_fiche_existante() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let (nom, prenom, naissance): (String, String, Option<chrono::NaiveDate>) = sqlx::query_as(
        "SELECT nom, prenom, date_naissance FROM ent_eleve WHERE id = $1",
    )
    .bind(c.eleve_lea_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    let mut f = formulaire(camp, &prenom, None);
    f.eleve.nom = nom;
    f.eleve.date_naissance = naissance;
    let (dossier, _) = dossier_admis(
        &ent,
        &c,
        camp,
        &format!("ancien-{}@exemple.test", Uuid::new_v4().simple()),
        f,
    )
    .await;
    let eleves_avant = compter(&ent, "SELECT count(*) FROM ent_eleve WHERE etablissement_id = $1", etab).await;
    let corr = ent
        .correspondances_for_actor(etab, d, dr, dossier)
        .await
        .unwrap();
    assert!(corr.eleves.iter().any(|e| e.eleve_id == c.eleve_lea_id));
    // Choix humain obligatoire.
    assert!(ent
        .valider_inscription_for_actor(etab, d, dr, dossier, &ValidationBody::default())
        .await
        .is_err());
    // La fiche a déjà des responsables : sans confirmation explicite, refus (INE copié).
    assert!(corr.eleves.iter().any(|e| e.responsables_lies > 0));
    assert!(ent
        .valider_inscription_for_actor(
            etab,
            d,
            dr,
            dossier,
            &valider_corps(
                vec![],
                Some(ChoixEleve::Existant {
                    eleve_id: c.eleve_lea_id,
                    confirme_responsables_existants: false,
                }),
            ),
        )
        .await
        .is_err());
    let v = ent
        .valider_inscription_for_actor(
            etab,
            d,
            dr,
            dossier,
            &valider_corps(
                vec![],
                Some(ChoixEleve::Existant {
                    eleve_id: c.eleve_lea_id,
                    confirme_responsables_existants: true,
                }),
            ),
        )
        .await
        .unwrap();
    assert_eq!(v.eleve_id, c.eleve_lea_id, "rattaché à la fiche existante");
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_eleve WHERE etablissement_id = $1", etab).await,
        eleves_avant,
        "aucune fiche créée"
    );
}

/// Envois (§2.2 pts 6, 17) : un échec d'envoi est tracé et repris, jamais avalé ; le
/// code de vérification est haché avec un secret serveur (pt 20).
#[tokio::test]
async fn echec_d_envoi_trace_et_code_hmac() {
    let Some((mut ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (d, dr) = dir(&c);
    let camp = campagne_ouverte(&ent, &c, "admission", None).await;
    let email = format!("hmac-{}@exemple.test", Uuid::new_v4().simple());
    let demande = ent.demarrer_preinscription(etab, &email).await.unwrap();
    let code = code_recu(&ent, &email);
    let (h,): (String,) = sqlx::query_as("SELECT code_hash FROM ent_preinscription_verification WHERE id = $1")
        .bind(demande.demande_id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_ne!(
        h,
        ent_auth::hash_token(&format!("preinscription:{}:{code}", demande.demande_id)),
        "plus un SHA-256 sans secret"
    );
    let jeton = ent
        .verifier_preinscription(etab, demande.demande_id, &code)
        .await
        .unwrap()
        .jeton;
    let dossier = ent
        .deposer_preinscription(etab, &jeton, &formulaire(camp, "Dobby", None))
        .await
        .unwrap();
    let (a, ar) = adm(&c);
    for p in dossier.pieces.iter().filter(|p| p.obligatoire) {
        ent.deposer_piece_famille(etab, &jeton, dossier.id, &p.code, "p.pdf").await.unwrap();
        ent.decider_piece_for_actor(etab, a, ar, dossier.id, &p.code, &PieceDecisionBody { statut: "acceptee".into(), motif: None }).await.unwrap();
    }
    ent.decider_for_actor(etab, d, dr, dossier.id, &DecisionBody { decision: "admis".into(), motif: None }).await.unwrap();
    ent.valider_inscription_for_actor(etab, d, dr, dossier.id, &ValidationBody::default())
        .await
        .unwrap();
    // La messagerie « n'envoie rien de confirmé » : l'échec est consigné et repris plus tard.
    ent.installer_mailer_de_test(false, false);
    distributeur(&ent, etab).distribuer_tout().await.unwrap();
    assert!(ent.expedier_envois_inscriptions(100).await.unwrap() >= 1);
    let (statut, tentatives, erreur): (String, i32, Option<String>) = sqlx::query_as(
        "SELECT statut, tentatives, derniere_erreur FROM ent_inscription_envoi
          WHERE dossier_id = $1 AND kind = 'invitation'",
    )
    .bind(dossier.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((statut.as_str(), tentatives), ("a_envoyer", 1));
    assert!(erreur.is_some(), "échec tracé");
    // Le délai de reprise n'est pas écoulé : rien n'est rejoué tout de suite.
    assert_eq!(ent.expedier_envois_inscriptions(100).await.unwrap(), 0);
    // Reprises jusqu'à l'échec définitif : message et invitation marqués, trace conservée.
    for _ in 0..6 {
        sqlx::query("UPDATE ent_inscription_envoi SET prochain_essai = now() WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool())
            .await
            .unwrap();
        ent.expedier_envois_inscriptions(100).await.unwrap();
    }
    let (statut,): (String,) = sqlx::query_as(
        "SELECT statut FROM ent_inscription_envoi WHERE dossier_id = $1 AND kind = 'invitation'",
    )
    .bind(dossier.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(statut, "echec");
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_inscription_invitation WHERE dossier_id = $1 AND statut = 'non_envoyee'",
            dossier.id
        )
        .await,
        1
    );
}
