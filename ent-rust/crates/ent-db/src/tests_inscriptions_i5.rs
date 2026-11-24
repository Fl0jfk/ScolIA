//! Socle 3, lot I5 (décisions, avis, commissions, liste d'attente, propositions de place, conditions), contre la
//! base locale de test (feature `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés en local.
//!
//! Les instants passés au **service** sont posés (2027) ; l'horloge de la base (`now()`, jour courant de `ent_acces_ouvert`) reste lue
//! par quelques écritures (horodatages de ligne, accès ouverts), mais aucune assertion ne la compare à un instant de test : les dates
//! qui en dépendent sont fixées très loin du présent (2020, 2099). Voir aussi la règle 33 de PILOTAGE.
//! Les délais de réponse se comparent à ces instants, jamais à « maintenant ».
//!
//! | Critère | Test(s) |
//! |---|---|
//! | décision sans motif interne refusée ; ajout seul ; revue = nouvelle ligne | `i5_motif_interne_obligatoire_et_ajout_seul` |
//! | capacité respectée, dépassement confirmé et tracé | `i5_capacite_et_depassement_confirme` |
//! | admission sous réserve : condition de réserve, confirmation mécanique | `i5_sous_reserve_et_confirmation_mecanique` |
//! | place libérée → suivant désigné, envoi sur confirmation, rien d'automatique | `i5_place_liberee_suivant_designe_envoi_sur_confirmation` |
//! | passer son tour, expiration constatée par une personne | `i5_passer_son_tour_et_expiration_constatee` |
//! | revue d'une admission : place libérée, suivant désigné | `i5_revue_libere_la_place` |
//! | publication programmée : invisible avant l'heure, plage calme confirmée | `i5_publication_programmee` |
//! | conditions de campagne, réponse de la famille, levée avec motif | `i5_conditions_reponse_famille_et_levee` |
//! | commission : liste, places en direct, lot, récapitulatif, publication groupée, clôture | `i5_commission_lot_et_publication` |
//! | avis internes, déport | `i5_avis_et_deport` |
//! | la famille ne lit ni motif interne, ni avis, ni autre responsable | `i5_la_famille_ne_lit_que_le_publie` |
//! | pièces en quarantaine : une pièce non contrôlée bloque la confirmation | `i5_piece_en_quarantaine_bloque_la_confirmation` |
//! | dossier bloqué pour désaccord : ni décision, ni désignation | `i5_desaccord_bloque_decision_et_designation` |
//! | isolation entre établissements, droits | `i5_droits_et_isolation` |

use chrono::{DateTime, Utc};
use ent_types::inscriptions::*;
use ent_types::inscriptions_decisions::*;
use ent_types::inscriptions_pieces::*;
use uuid::Uuid;

use super::tests_inscriptions::{
    adm, campagne, courriels_vers, dir, formulaire, labo, second_parent, verifier,
};
use super::tests_inscriptions_i3_i4::pdf_simple;
use super::*;

pub(super) fn t(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

/// Instant de référence des tests (mercredi 3 mars 2027, 9 h à Paris).
pub(super) fn maintenant() -> DateTime<Utc> {
    t("2027-03-03T08:00:00Z")
}

pub(super) fn email(prefixe: &str) -> String {
    format!(
        "{}-{}@exemple.test",
        prefixe.to_lowercase(),
        Uuid::new_v4().simple()
    )
}

pub(super) struct Dos {
    pub(super) id: Uuid,
    pub(super) jeton: String,
    pub(super) email: String,
}

/// Campagne d'admission ouverte, offre unique de capacité `capacite` (6e).
pub(super) async fn campagne_cap(
    ent: &PgEnt,
    c: &LaboCast,
    capacite: Option<i32>,
    annee: &str,
) -> (Uuid, Uuid) {
    let (d, dr) = dir(c);
    let mut b = campagne("admission", capacite);
    b.pieces = vec![];
    b.annee_cible = annee.into();
    let camp = ent
        .creer_campagne_for_actor(c.etablissement_id, d, dr, &b)
        .await
        .unwrap();
    ent.ouvrir_campagne_for_actor(c.etablissement_id, d, dr, camp.id)
        .await
        .unwrap();
    let offre: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_inscription_offre WHERE etablissement_id = $1 AND campagne_id = $2 AND archivee_le IS NULL",
    )
    .bind(c.etablissement_id)
    .bind(camp.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    (camp.id, offre)
}

/// Dossier déposé par un déposant vérifié, pris en charge par le secrétariat.
pub(super) async fn dossier(ent: &PgEnt, c: &LaboCast, camp: Uuid, prenom: &str, second: Option<&str>) -> Dos {
    let etab = c.etablissement_id;
    let (a, ar) = adm(c);
    let mail = email(prenom);
    let jeton = verifier(ent, etab, &mail).await;
    let id = ent
        .deposer_preinscription(
            etab,
            &jeton,
            &formulaire(camp, prenom, second.map(second_parent)),
        )
        .await
        .unwrap()
        .id;
    ent.changer_statut_for_actor(
        etab,
        a,
        ar,
        id,
        &StatutBody {
            statut: "en_instruction".into(),
            motif: None,
        },
    )
    .await
    .unwrap();
    Dos {
        id,
        jeton,
        email: mail,
    }
}

pub(super) fn corps(sens: &str, offre: Option<Uuid>, motif: &str) -> DecisionCompleteBody {
    DecisionCompleteBody {
        sens: sens.into(),
        offre_id: offre,
        motif_interne: motif.into(),
        ..Default::default()
    }
}

pub(super) async fn decider(
    ent: &PgEnt,
    c: &LaboCast,
    d: &Dos,
    b: &DecisionCompleteBody,
) -> Result<DecisionDto, EntDbError> {
    let (u, r) = dir(c);
    ent.decider_complete_for_actor(c.etablissement_id, u, r, d.id, b, false, maintenant())
        .await
}

pub(super) async fn revoir(
    ent: &PgEnt,
    c: &LaboCast,
    d: &Dos,
    b: &DecisionCompleteBody,
) -> Result<DecisionDto, EntDbError> {
    let (u, r) = dir(c);
    ent.decider_complete_for_actor(c.etablissement_id, u, r, d.id, b, true, maintenant())
        .await
}

pub(super) async fn expedier(ent: &PgEnt) {
    for _ in 0..4 {
        ent.expedier_envois_inscriptions(200).await.unwrap();
    }
}

pub(super) async fn etat(ent: &PgEnt, etab: Uuid, d: &Dos) -> String {
    sqlx::query_scalar(
        "SELECT etat FROM ent_inscription_dossier WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(d.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

pub(super) async fn proposition_ouverte(ent: &PgEnt, etab: Uuid, d: &Dos) -> Option<(Uuid, String)> {
    sqlx::query_as(
        "SELECT id, statut FROM ent_inscription_proposition_place
          WHERE etablissement_id = $1 AND dossier_id = $2 AND statut IN ('designee', 'envoyee')",
    )
    .bind(etab)
    .bind(d.id)
    .fetch_optional(ent.pool_verif())
    .await
    .unwrap()
}

pub(super) fn erreur(e: EntDbError) -> String {
    e.to_string()
}

// ───────────────────────────── Décisions ─────────────────────────────

/// Aucune décision sans motif interne ; une décision est une ligne en ajout seul : ni modifiée ni supprimée, une
/// revue ajoute une ligne qui remplace la précédente.
#[tokio::test]
async fn i5_motif_interne_obligatoire_et_ajout_seul() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Motif", None).await;
    for vide in ["", "   ", "ok"] {
        assert!(
            decider(&ent, &c, &d, &corps("refuse", Some(offre), vide))
                .await
                .is_err(),
            "motif « {vide} »"
        );
    }
    assert!(
        decider(&ent, &c, &d, &corps("inconnu", None, "motif valable"))
            .await
            .is_err()
    );
    let mut reserve_sans_texte = corps("admis_sous_reserve", Some(offre), "motif valable");
    assert!(
        decider(&ent, &c, &d, &reserve_sans_texte).await.is_err(),
        "réserve obligatoire"
    );
    reserve_sans_texte.reserve = Some("bulletin du 2e trimestre".into());
    reserve_sans_texte.sens = "admis".into();
    assert!(
        decider(&ent, &c, &d, &reserve_sans_texte).await.is_err(),
        "pas de réserve hors admission sous réserve"
    );
    assert_eq!(
        etat(&ent, etab, &d).await,
        "en_instruction",
        "aucune décision partielle"
    );

    let premiere = decider(
        &ent,
        &c,
        &d,
        &corps("refuse", Some(offre), "Dossier hors secteur"),
    )
    .await
    .unwrap();
    assert_eq!(
        (premiere.sens.as_str(), premiere.origine.as_str()),
        ("refuse", "direction")
    );
    // Déjà décidé : il faut revoir, jamais redécider.
    assert!(decider(
        &ent,
        &c,
        &d,
        &corps("liste_attente", Some(offre), "autre avis")
    )
    .await
    .is_err());
    assert!(
        revoir(&ent, &c, &d, &corps("admis", Some(offre), ""))
            .await
            .is_err(),
        "motif de la revue obligatoire"
    );
    assert!(
        revoir(
            &ent,
            &c,
            &d,
            &corps("refuse", Some(offre), "toujours refusé")
        )
        .await
        .is_err(),
        "rien ne change"
    );
    let seconde = revoir(
        &ent,
        &c,
        &d,
        &corps("admis", Some(offre), "Place trouvée après recours"),
    )
    .await
    .unwrap();
    assert_eq!(seconde.remplace_decision_id, Some(premiere.id));
    assert_eq!(etat(&ent, etab, &d).await, "admis");

    // Ajout seul : ni modification, ni suppression directe (seule la purge du dossier supprime en cascade).
    for sql in [
        "UPDATE ent_inscription_decision SET motif_interne = 'réécrit' WHERE id = $1",
        "UPDATE ent_inscription_decision SET sens = 'admis' WHERE id = $1",
        "DELETE FROM ent_inscription_decision WHERE id = $1",
    ] {
        let e = sqlx::query(sql)
            .bind(premiere.id)
            .execute(ent.pool_verif())
            .await;
        assert!(e.is_err(), "{sql}");
    }
    let (u, r) = dir(&c);
    let h = ent
        .decisions_dossier_for_actor(etab, u, r, d.id)
        .await
        .unwrap();
    assert_eq!(
        h.iter().map(|x| x.sens.as_str()).collect::<Vec<_>>(),
        ["refuse", "admis"]
    );
    assert_eq!(
        h[0].motif_interne.as_deref(), Some("Dossier hors secteur"),
        "l'ancienne ligne est intacte"
    );
}

/// La capacité de l'offre est respectée : un dépassement se confirme et se trace, jamais en silence.
#[tokio::test]
async fn i5_capacite_et_depassement_confirme() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let a = dossier(&ent, &c, camp, "Alice", None).await;
    let b = dossier(&ent, &c, camp, "Bruno", None).await;
    let da = decider(&ent, &c, &a, &corps("admis", Some(offre), "Première place"))
        .await
        .unwrap();
    assert!(!da.depassement_capacite);
    let e = erreur(
        decider(&ent, &c, &b, &corps("admis", Some(offre), "Seconde place"))
            .await
            .unwrap_err(),
    );
    assert!(e.contains("capacité"), "{e}");
    assert_eq!(
        etat(&ent, c.etablissement_id, &b).await,
        "en_instruction",
        "rien n'a été écrit"
    );
    let mut conf = corps("admis", Some(offre), "Dépassement assumé par la direction");
    conf.confirmer_depassement = true;
    let db = decider(&ent, &c, &b, &conf).await.unwrap();
    assert!(db.depassement_capacite, "le dépassement est tracé");
    // Et la liste d'attente ne consomme pas de place.
    let (camp2, offre2) = campagne_cap(&ent, &c, Some(1), "2028-2029").await;
    let x = dossier(&ent, &c, camp2, "Xavier", None).await;
    let y = dossier(&ent, &c, camp2, "Yvan", None).await;
    decider(&ent, &c, &x, &corps("admis", Some(offre2), "Place"))
        .await
        .unwrap();
    decider(
        &ent,
        &c,
        &y,
        &corps("liste_attente", Some(offre2), "Offre complète"),
    )
    .await
    .unwrap();
    let (u, r) = dir(&c);
    let l = ent
        .liste_attente_for_actor(c.etablissement_id, u, r, offre2)
        .await
        .unwrap();
    assert_eq!(
        (l.disponibles, l.lignes.len(), l.lignes[0].position),
        (Some(0), 1, 1)
    );
}

/// Admission sous réserve : la condition de réserve se lève (établissement) ; tant qu'elle ne l'est pas, ni
/// confirmation ni validation ; levée, le dossier est confirmé mécaniquement, puis validé par une personne.
#[tokio::test]
async fn i5_sous_reserve_et_confirmation_mecanique() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Reserve", None).await;
    let mut b = corps("admis_sous_reserve", Some(offre), "Sous réserve du passage");
    b.reserve = Some("Décision de passage en 6e".into());
    let dec = decider(&ent, &c, &d, &b).await.unwrap();
    assert_eq!(dec.reserve.as_deref(), Some("Décision de passage en 6e"));
    let (u, r) = dir(&c);
    let (a, ar) = adm(&c);
    let cond = ent
        .conditions_dossier_for_actor(etab, a, ar, d.id)
        .await
        .unwrap();
    assert!(!cond.toutes_remplies);
    assert!(cond
        .conditions
        .iter()
        .any(|x| x.code == "reserve" && !x.remplie));
    // La famille a accepté la place proposée (D-069 règle 3) ; la réserve, elle, n'est pas levée.
    ent.repondre_proposition_famille(
        etab,
        &d.jeton,
        d.id,
        &ReponsePropositionBody {
            reponse: "acceptee".into(),
        },
        maintenant(),
    )
    .await
    .unwrap();
    // Une validation tout de suite est refusée : la réserve n'est pas levée.
    let e = erreur(
        ent.valider_inscription_for_actor(etab, u, r, d.id, &ValidationBody::default())
            .await
            .unwrap_err(),
    );
    assert!(e.contains("conditions de confirmation"), "{e}");
    // Une réserve posée par la direction ne se lève pas au secrétariat.
    assert!(ent
        .remplir_condition_for_actor(
            etab,
            a,
            ar,
            d.id,
            "reserve",
            &RemplirConditionBody::default(),
        maintenant(),
    )
        .await
        .is_err());
    let c1 = ent
        .remplir_condition_for_actor(
            etab,
            u,
            r,
            d.id,
            "reserve",
            &RemplirConditionBody {
                motif: Some("décision reçue".into()),
                source_ref: None,
            },
        maintenant(),
    )
        .await
        .unwrap();
    assert!(c1.confirme, "toutes les conditions remplies : confirmé");
    assert_eq!(etat(&ent, etab, &d).await, "confirme");
    ent.valider_inscription_for_actor(etab, u, r, d.id, &ValidationBody::default())
        .await
        .unwrap();
    assert_eq!(etat(&ent, etab, &d).await, "validee");
    // Revue vers une admission sans réserve : la condition disparaît (dossier non validé).
}

/// Une place se libère : le système **désigne** le suivant, rien ne part ; une personne confirme l'envoi (la
/// décision « admis » est alors prise et publiée) ; le suivant est désigné quand la famille décline.
#[tokio::test]
async fn i5_place_liberee_suivant_designe_envoi_sur_confirmation() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let a = dossier(&ent, &c, camp, "Premier", None).await;
    let b = dossier(&ent, &c, camp, "Deuxieme", None).await;
    let d3 = dossier(&ent, &c, camp, "Troisieme", None).await;
    decider(&ent, &c, &a, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    decider(
        &ent,
        &c,
        &b,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    decider(
        &ent,
        &c,
        &d3,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    expedier(&ent).await;
    let mails_b = courriels_vers(&ent, &b.email);
    assert!(
        proposition_ouverte(&ent, etab, &b).await.is_none(),
        "aucune place libre : rien de désigné"
    );

    // La famille A se désiste : la place est libre.
    ent.desister_famille(etab, &a.jeton, a.id).await.unwrap();
    let p = proposition_ouverte(&ent, etab, &b)
        .await
        .expect("le deuxième est désigné");
    assert_eq!(p.1, "designee");
    assert!(
        proposition_ouverte(&ent, etab, &d3).await.is_none(),
        "un seul suivant pour une seule place"
    );
    // Rien n'a été décidé ni envoyé par le système.
    assert_eq!(etat(&ent, etab, &b).await, "liste_attente");
    expedier(&ent).await;
    assert_eq!(
        courriels_vers(&ent, &b.email),
        mails_b,
        "aucun message avant la confirmation humaine"
    );
    let f = ent.decision_famille(etab, &b.jeton, b.id).await.unwrap();
    assert!(
        f.proposition.is_none(),
        "la désignation n'est pas visible de la famille"
    );
    // Les rangs se suivent sans trou : le troisième est en position 2.
    let l = ent
        .liste_attente_for_actor(etab, u, r, offre)
        .await
        .unwrap();
    assert_eq!(
        l.lignes.iter().map(|x| x.position).collect::<Vec<_>>(),
        [1, 2]
    );

    // L'envoi se confirme explicitement.
    let e = erreur(
        ent.envoyer_proposition_for_actor(
            etab,
            u,
            r,
            p.0,
            &EnvoyerPropositionBody::default(),
            maintenant(),
        )
        .await
        .unwrap_err(),
    );
    assert!(e.contains("confirme"), "{e}");
    // Un agent sans droit de décision n'envoie pas.
    let (ad, adr) = adm(&c);
    assert!(ent
        .envoyer_proposition_for_actor(
            etab,
            ad,
            adr,
            p.0,
            &EnvoyerPropositionBody {
                confirmer: true,
                ..Default::default()
            },
            maintenant()
        )
        .await
        .is_err());
    let envoyee = ent
        .envoyer_proposition_for_actor(
            etab,
            u,
            r,
            p.0,
            &EnvoyerPropositionBody {
                confirmer: true,
                ..Default::default()
            },
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!(envoyee.statut, "envoyee");
    assert!(envoyee.repondre_avant.is_some());
    assert_eq!(etat(&ent, etab, &b).await, "admis");
    let h = ent
        .decisions_dossier_for_actor(etab, u, r, b.id)
        .await
        .unwrap();
    assert_eq!(h.last().unwrap().origine, "liste_attente");
    assert_eq!(
        h.last().unwrap().decide_par,
        Some(u),
        "la décision a un auteur humain"
    );
    assert!(h.last().unwrap().publiee_le.is_some());
    expedier(&ent).await;
    assert_eq!(
        courriels_vers(&ent, &b.email),
        mails_b + 1,
        "message à la famille après la décision"
    );
    // La famille répond : elle voit sa place, pas celle des autres.
    let vue = ent.decision_famille(etab, &b.jeton, b.id).await.unwrap();
    assert_eq!(vue.sens.as_deref(), Some("admis"));
    assert_eq!(
        vue.proposition.as_ref().map(|x| x.statut.as_str()),
        Some("envoyee")
    );
    assert!(vue.actions.contains(&"repondre".to_string()));
    assert!(
        ent.repondre_proposition_famille(
            etab,
            &d3.jeton,
            b.id,
            &ReponsePropositionBody {
                reponse: "acceptee".into()
            },
            maintenant()
        )
        .await
        .is_err(),
        "pas le déposant"
    );
    // Elle décline : le dossier se clôt, la place se libère, le suivant est désigné.
    ent.repondre_proposition_famille(
        etab,
        &b.jeton,
        b.id,
        &ReponsePropositionBody {
            reponse: "declinee".into(),
        },
        maintenant(),
    )
    .await
    .unwrap();
    assert_eq!(etat(&ent, etab, &b).await, "desiste");
    assert_eq!(
        proposition_ouverte(&ent, etab, &d3).await.unwrap().1,
        "designee"
    );
    // Place d'une offre : jamais plus d'admis que la capacité.
    let l = ent
        .liste_attente_for_actor(etab, u, r, offre)
        .await
        .unwrap();
    assert_eq!(l.disponibles, Some(1));
}

/// Passer son tour (motif) désigne le suivant ; un délai dépassé se constate par une personne, jamais seul.
#[tokio::test]
async fn i5_passer_son_tour_et_expiration_constatee() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let a = dossier(&ent, &c, camp, "Occupant", None).await;
    let b = dossier(&ent, &c, camp, "Suivant", None).await;
    let d3 = dossier(&ent, &c, camp, "Dernier", None).await;
    decider(&ent, &c, &a, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    decider(
        &ent,
        &c,
        &b,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    decider(
        &ent,
        &c,
        &d3,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    ent.desister_famille(etab, &a.jeton, a.id).await.unwrap();
    let pb = proposition_ouverte(&ent, etab, &b).await.unwrap().0;
    // Passer son tour : motif obligatoire.
    assert!(ent
        .ecarter_proposition_for_actor(
            etab,
            u,
            r,
            pb,
            &MotifBody { motif: "".into() },
            maintenant()
        )
        .await
        .is_err());
    ent.ecarter_proposition_for_actor(
        etab,
        u,
        r,
        pb,
        &MotifBody {
            motif: "Famille injoignable".into(),
        },
        maintenant(),
    )
    .await
    .unwrap();
    assert_eq!(
        etat(&ent, etab, &b).await,
        "liste_attente",
        "le dossier garde sa place dans la liste"
    );
    let pd = proposition_ouverte(&ent, etab, &d3)
        .await
        .expect("le suivant est désigné")
        .0;
    // Envoi, puis expiration : refusée avant la date limite.
    let envoyee = ent
        .envoyer_proposition_for_actor(
            etab,
            u,
            r,
            pd,
            &EnvoyerPropositionBody {
                confirmer: true,
                ..Default::default()
            },
            maintenant(),
        )
        .await
        .unwrap();
    let limite = envoyee.repondre_avant.unwrap();
    let e = erreur(
        ent.constater_expiration_for_actor(etab, u, r, pd, maintenant())
            .await
            .unwrap_err(),
    );
    assert!(e.contains("délai"), "{e}");
    let liste = ent
        .propositions_for_actor(etab, u, r, None, Some("envoyee".into()), maintenant())
        .await
        .unwrap();
    assert!(liste.iter().any(|p| p.id == pd && !p.expiree_a_constater));
    let apres = t("2027-12-01T08:00:00Z");
    let liste = ent
        .propositions_for_actor(etab, u, r, None, Some("envoyee".into()), apres)
        .await
        .unwrap();
    assert!(
        liste.iter().any(|p| p.id == pd && p.expiree_a_constater),
        "l'expiration est proposée (limite {limite})"
    );
    // Aucune expiration automatique : le dossier est toujours admis tant qu'une personne n'a pas constaté.
    assert_eq!(etat(&ent, etab, &d3).await, "admis");
    let p = ent
        .constater_expiration_for_actor(etab, u, r, pd, apres)
        .await
        .unwrap();
    assert_eq!(p.statut, "expiree");
    assert_eq!(etat(&ent, etab, &d3).await, "sans_suite");
    // La place est libre (une place **ultérieure**) : le deuxième, écarté auparavant pour l'autre place, redevient
    // désignable à son rang (D-069 M11) — désigné, jamais envoyé.
    assert!(proposition_ouverte(&ent, etab, &b).await.is_some_and(|p| p.1 == "designee"));
    let l = ent
        .liste_attente_for_actor(etab, u, r, offre)
        .await
        .unwrap();
    assert_eq!((l.disponibles, l.lignes.len()), (Some(1), 1));
}

/// Revoir une admission en refus libère la place et désigne le suivant ; la revue ne réécrit rien.
#[tokio::test]
async fn i5_revue_libere_la_place() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let a = dossier(&ent, &c, camp, "Revu", None).await;
    let b = dossier(&ent, &c, camp, "Attend", None).await;
    decider(&ent, &c, &a, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    decider(
        &ent,
        &c,
        &b,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    revoir(
        &ent,
        &c,
        &a,
        &corps("refuse", Some(offre), "Pièce falsifiée constatée"),
    )
    .await
    .unwrap();
    assert_eq!(etat(&ent, etab, &a).await, "refuse");
    assert_eq!(
        proposition_ouverte(&ent, etab, &b).await.unwrap().1,
        "designee"
    );
    assert!(
        proposition_ouverte(&ent, etab, &a).await.is_none(),
        "la proposition de A est annulée"
    );
    // Un dossier revu vers la liste d'attente ne reprend jamais d'office la place qu'il vient de libérer.
    let (camp2, offre2) = campagne_cap(&ent, &c, Some(1), "2028-2029").await;
    let x = dossier(&ent, &c, camp2, "Seul", None).await;
    decider(&ent, &c, &x, &corps("admis", Some(offre2), "Admis"))
        .await
        .unwrap();
    revoir(
        &ent,
        &c,
        &x,
        &corps("liste_attente", Some(offre2), "Revue en attente"),
    )
    .await
    .unwrap();
    assert_eq!(etat(&ent, etab, &x).await, "liste_attente");
    assert!(
        proposition_ouverte(&ent, etab, &x).await.is_none(),
        "pas de désignation pour la place qu'on vient de quitter"
    );
}

// ───────────────────────────── Publication ─────────────────────────────

/// Publication programmée : la décision est prise mais invisible de la famille jusqu'à l'heure ; aucun message ne
/// part avant ; une heure en plage calme se confirme ; la tâche de fond publie à l'heure, pas avant.
#[tokio::test]
async fn i5_publication_programmee() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Programme", None).await;
    let mails = courriels_vers(&ent, &d.email);
    // 22 h à Paris (21 h UTC en hiver) : plage calme, à confirmer.
    let mut b = corps("refuse", Some(offre), "Hors secteur");
    b.publier_le = Some(t("2027-03-10T21:00:00Z"));
    let e = erreur(decider(&ent, &c, &d, &b).await.unwrap_err());
    assert!(e.contains("20 h"), "{e}");
    assert_eq!(
        etat(&ent, etab, &d).await,
        "en_instruction",
        "refus : rien n'est écrit"
    );
    // Un dimanche aussi (14 mars 2027).
    b.publier_le = Some(t("2027-03-14T09:00:00Z"));
    assert!(decider(&ent, &c, &d, &b).await.is_err());
    // Mercredi 10 h à Paris : sans confirmation.
    let heure = t("2027-03-10T09:00:00Z");
    b.publier_le = Some(heure);
    let dec = decider(&ent, &c, &d, &b).await.unwrap();
    assert_eq!(dec.publication_prevue_le, Some(heure));
    assert!(dec.publiee_le.is_none());
    let vue = ent.decision_famille(etab, &d.jeton, d.id).await.unwrap();
    assert!(!vue.publiee && vue.sens.is_none(), "la famille ne lit rien");
    let dossier_famille = ent.dossier_famille(etab, &d.jeton, d.id).await.unwrap();
    assert_eq!(
        (
            dossier_famille.statut.as_str(),
            dossier_famille.etat.as_str()
        ),
        ("en_instruction", "en_instruction")
    );
    assert!(dossier_famille.decision_motif.is_none());
    assert!(dossier_famille
        .suivi
        .iter()
        .all(|s| s.kind != "statut" || s.statut_apres.as_deref() != Some("refuse")));
    expedier(&ent).await;
    assert_eq!(
        courriels_vers(&ent, &d.email),
        mails,
        "aucun message avant l'heure"
    );
    // La tâche de fond : rien avant l'heure, publication à l'heure.
    ent.publier_decisions_echues_tache(t("2027-03-10T08:59:00Z"))
        .await
        .unwrap();
    assert!(
        !ent.decision_famille(etab, &d.jeton, d.id)
            .await
            .unwrap()
            .publiee
    );
    ent.publier_decisions_echues_tache(heure).await.unwrap();
    let vue = ent.decision_famille(etab, &d.jeton, d.id).await.unwrap();
    assert_eq!(vue.sens.as_deref(), Some("refuse"));
    assert_eq!(
        ent.dossier_famille(etab, &d.jeton, d.id)
            .await
            .unwrap()
            .etat,
        "refuse"
    );
    expedier(&ent).await;
    assert_eq!(courriels_vers(&ent, &d.email), mails + 1);
    // Une décision publiée ne se republie pas.
    let (u, r) = dir(&c);
    assert!(ent
        .publier_decision_for_actor(etab, u, r, dec.id, &PublierBody::default(), heure)
        .await
        .is_err());
    // Plage calme confirmée : acceptée.
    let (camp2, offre2) = campagne_cap(&ent, &c, None, "2028-2029").await;
    let d2 = dossier(&ent, &c, camp2, "Soir", None).await;
    let mut b2 = corps("refuse", Some(offre2), "Hors secteur");
    b2.publier_le = Some(t("2027-03-10T21:00:00Z"));
    b2.confirmer_plage_calme = true;
    assert!(decider(&ent, &c, &d2, &b2).await.is_ok());
}

// ───────────────────────────── Conditions ─────────────────────────────

/// Conditions de la campagne : réponse de la famille (dans son espace seulement), contrat constaté ; la levée
/// d'une condition se motive et se trace ; la validation exige le tout.
#[tokio::test]
async fn i5_conditions_reponse_famille_et_levee() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (a, ar) = adm(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    assert!(ent
        .definir_conditions_campagne_for_actor(
            etab,
            a,
            ar,
            camp,
            &ConditionsCampagneBody {
                codes: vec!["inconnue".into()]
            }
        )
        .await
        .is_err());
    let codes = ent
        .definir_conditions_campagne_for_actor(
            etab,
            a,
            ar,
            camp,
            &ConditionsCampagneBody {
                codes: vec![
                    "reponse_famille".into(),
                    "contrat_signe".into(),
                    "frais_payes".into(),
                ],
            },
        )
        .await
        .unwrap();
    assert_eq!(codes.len(), 3);
    let d = dossier(&ent, &c, camp, "Conditions", None).await;
    decider(&ent, &c, &d, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let cond = ent
        .conditions_dossier_for_actor(etab, a, ar, d.id)
        .await
        .unwrap();
    assert_eq!(cond.conditions.iter().filter(|x| !x.remplie).count(), 3);
    // Le personnel ne répond pas à la place de la famille.
    assert!(ent
        .remplir_condition_for_actor(
            etab,
            a,
            ar,
            d.id,
            "reponse_famille",
            &RemplirConditionBody::default(),
        maintenant(),
    )
        .await
        .is_err());
    assert!(ent
        .valider_inscription_for_actor(etab, u, r, d.id, &ValidationBody::default())
        .await
        .is_err());
    // La famille accepte : sa condition est remplie.
    let vue = ent
        .repondre_proposition_famille(
            etab,
            &d.jeton,
            d.id,
            &ReponsePropositionBody {
                reponse: "acceptee".into(),
            },
            maintenant(),
        )
        .await
        .unwrap();
    assert!(vue
        .conditions
        .iter()
        .any(|x| x.code == "reponse_famille" && x.remplie));
    assert!(
        vue.conditions.iter().all(|x| x.motif.is_none()),
        "jamais le motif d'une levée à la famille"
    );
    // Une seconde réponse est refusée.
    assert!(ent
        .repondre_proposition_famille(
            etab,
            &d.jeton,
            d.id,
            &ReponsePropositionBody {
                reponse: "declinee".into()
            },
            maintenant()
        )
        .await
        .is_err());
    // Contrat constaté par le secrétariat (source tracée), frais levés par la direction avec motif.
    ent.remplir_condition_for_actor(
        etab,
        a,
        ar,
        d.id,
        "contrat_signe",
        &RemplirConditionBody {
            motif: Some("contrat reçu en main propre".into()),
            source_ref: None,
        },
        maintenant(),
    )
    .await
    .unwrap();
    assert!(
        ent.remplir_condition_for_actor(
            etab,
            a,
            ar,
            d.id,
            "contrat_signe",
            &RemplirConditionBody::default(),
        maintenant(),
    )
        .await
        .is_err(),
        "déjà remplie"
    );
    assert!(
        ent.lever_condition_for_actor(
            etab,
            a,
            ar,
            d.id,
            "frais_payes",
            &MotifBody {
                motif: "geste commercial".into()
            },
        maintenant(),
    )
        .await
        .is_err(),
        "levée réservée à la direction"
    );
    assert!(
        ent.lever_condition_for_actor(
            etab,
            u,
            r,
            d.id,
            "frais_payes",
            &MotifBody { motif: "x".into() },
        maintenant(),
    )
        .await
        .is_err(),
        "motif obligatoire"
    );
    let fin = ent
        .lever_condition_for_actor(
            etab,
            u,
            r,
            d.id,
            "frais_payes",
            &MotifBody {
                motif: "Famille en difficulté, échelonnement convenu".into(),
            },
        maintenant(),
    )
        .await
        .unwrap();
    assert!(fin.confirme && fin.toutes_remplies);
    assert_eq!(etat(&ent, etab, &d).await, "confirme");
    let frais = fin
        .conditions
        .iter()
        .find(|x| x.code == "frais_payes")
        .unwrap();
    assert_eq!(frais.source.as_deref(), Some("levee"));
    assert!(frais.motif.as_deref().unwrap().contains("échelonnement"));
    ent.valider_inscription_for_actor(etab, u, r, d.id, &ValidationBody::default())
        .await
        .unwrap();
    // Les conditions d'une campagne ne s'appliquent qu'aux décisions prises ensuite.
    let d2 = dossier(&ent, &c, camp, "Sans", None).await;
    ent.definir_conditions_campagne_for_actor(
        etab,
        a,
        ar,
        camp,
        &ConditionsCampagneBody { codes: vec![] },
    )
    .await
    .unwrap();
    decider(&ent, &c, &d2, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    assert_eq!(
        ent.conditions_dossier_for_actor(etab, a, ar, d2.id)
            .await
            .unwrap()
            .conditions
            .iter()
            .filter(|x| x.source.as_deref() != Some("calculee"))
            .count(),
        0
    );
}

/// Une pièce déposée reste en quarantaine tant qu'elle n'est pas analysée puis contrôlée : elle ne compte pas pour
/// la confirmation (condition calculée), même quand les conditions saisies sont toutes remplies.
#[tokio::test]
async fn i5_piece_en_quarantaine_bloque_la_confirmation() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (a, ar) = adm(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    ent.definir_regles_pieces_for_actor(
        etab,
        u,
        r,
        camp,
        &[ReglePieceDto {
            code_type: "livret_famille".into(),
            libelle: "Livret de famille".into(),
            consigne: None,
            moment: "depot".into(),
            bloquante: true,
            portee: "dossier".into(),
            conditions: vec![],
        }],
    )
    .await
    .unwrap();
    ent.definir_conditions_campagne_for_actor(
        etab,
        a,
        ar,
        camp,
        &ConditionsCampagneBody {
            codes: vec!["contrat_signe".into()],
        },
    )
    .await
    .unwrap();
    let d = dossier(&ent, &c, camp, "Piece", None).await;
    decider(&ent, &c, &d, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    ent.remplir_condition_for_actor(
        etab,
        a,
        ar,
        d.id,
        "contrat_signe",
        &RemplirConditionBody::default(),
        maintenant(),
    )
    .await
    .unwrap();
    let cond = ent
        .conditions_dossier_for_actor(etab, a, ar, d.id)
        .await
        .unwrap();
    assert!(!cond.toutes_remplies, "la pièce bloquante manque");
    assert_eq!(etat(&ent, etab, &d).await, "admis");
    // Déposée : en quarantaine (statut non accepté) tant que le contrôle n'a pas eu lieu.
    ent.deposer_fichiers_famille(etab, &d.jeton, d.id, "livret_famille", vec![pdf_simple()])
        .await
        .unwrap();
    let cond = ent
        .conditions_dossier_for_actor(etab, a, ar, d.id)
        .await
        .unwrap();
    assert!(
        cond.conditions
            .iter()
            .any(|x| x.code == "pieces_bloquantes" && !x.remplie),
        "déposée n'est pas acceptée"
    );
    assert!(ent
        .valider_inscription_for_actor(etab, u, r, d.id, &ValidationBody::default())
        .await
        .is_err());
    // Analyse puis contrôle humain : seulement alors la condition est remplie.
    ent.analyser_pieces_en_attente(etab, super::tests_inscriptions_i3_i4::av(), 100)
        .await
        .unwrap();
    ent.controler_piece_inscription_for_actor(
        etab,
        a,
        ar,
        d.id,
        "livret_famille",
        &ControlePieceBody {
            decision: "accepter".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let fin = ent
        .reevaluer_conditions_for_actor(etab, a, ar, d.id)
        .await
        .unwrap();
    assert!(fin.confirme, "{:?}", fin.conditions);
}

// ───────────────────────────── Commission ─────────────────────────────

/// Séance de commission : liste, places en direct, décisions en lot avec récapitulatif (un échec n'annule pas les
/// autres), dossier hors liste refusé, publication groupée, clôture sans décider les dossiers non examinés.
#[tokio::test]
async fn i5_commission_lot_et_publication() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (a, ar) = adm(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let d1 = dossier(&ent, &c, camp, "Un", None).await;
    let d2 = dossier(&ent, &c, camp, "Deux", None).await;
    let d3 = dossier(&ent, &c, camp, "Trois", None).await;
    let hors = dossier(&ent, &c, camp, "Hors", None).await;
    let seance = ent
        .creer_commission_admission_for_actor(
            etab,
            u,
            r,
            &CommissionBody {
                campagne_id: camp,
                intitule: "Commission de mars".into(),
                seance_le: t("2027-03-20T09:00:00Z"),
                membres: vec![u],
                dossier_ids: vec![d1.id, d2.id, d3.id],
            },
        )
        .await
        .unwrap();
    assert_eq!(seance.dossiers.len(), 3);
    assert_eq!(
        (seance.places[0].capacite, seance.places[0].disponibles),
        (Some(1), Some(1))
    );
    assert!(
        ent.creer_commission_admission_for_actor(
            etab,
            a,
            ar,
            &CommissionBody {
                campagne_id: camp,
                intitule: "Interdit".into(),
                seance_le: maintenant(),
                ..Default::default()
            }
        )
        .await
        .is_err(),
        "décider : direction"
    );
    let recap = ent
        .decider_en_lot_for_actor(
            etab,
            u,
            r,
            seance.id,
            &DecisionsEnLotBody {
                decisions: vec![
                    DecisionEnLotItem {
                        dossier_id: d1.id,
                        decision: corps("admis", Some(offre), "Admis en commission"),
                    },
                    // Offre pleine, dépassement non confirmé : refusé, les autres passent.
                    DecisionEnLotItem {
                        dossier_id: d2.id,
                        decision: corps("admis", Some(offre), "Admis en commission"),
                    },
                    DecisionEnLotItem {
                        dossier_id: d3.id,
                        decision: corps("liste_attente", Some(offre), "Liste d'attente"),
                    },
                    // Hors liste de la séance.
                    DecisionEnLotItem {
                        dossier_id: hors.id,
                        decision: corps("refuse", Some(offre), "Hors séance"),
                    },
                    // Sans motif.
                    DecisionEnLotItem {
                        dossier_id: d2.id,
                        decision: corps("refuse", None, ""),
                    },
                ],
            },
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!(
        (recap.reussies, recap.refusees),
        (2, 3),
        "{:?}",
        recap.resultats
    );
    assert!(recap.resultats[1]
        .erreur
        .as_deref()
        .unwrap()
        .contains("capacité"));
    assert!(recap.resultats[3]
        .erreur
        .as_deref()
        .unwrap()
        .contains("séance"));
    assert_eq!(etat(&ent, etab, &d1).await, "admis");
    assert_eq!(
        etat(&ent, etab, &d2).await,
        "en_instruction",
        "refusé : rien n'est écrit"
    );
    assert_eq!(etat(&ent, etab, &hors).await, "en_instruction");
    let vue = ent
        .commission_admission_for_actor(etab, u, r, seance.id)
        .await
        .unwrap();
    assert_eq!(vue.places[0].disponibles, Some(0), "places en direct");
    assert_eq!(
        vue.dossiers
            .iter()
            .filter(|x| x.examine_le.is_some())
            .count(),
        2
    );
    // Un dossier décidé en séance ne se retire pas de la liste.
    assert!(ent
        .modifier_dossiers_commission_for_actor(
            etab,
            u,
            r,
            seance.id,
            &CommissionDossiersBody {
                retirer: vec![d1.id],
                ..Default::default()
            }
        )
        .await
        .is_err());
    // Publication groupée : la décision passe, deux familles prévenues.
    let p = ent
        .publier_commission_for_actor(etab, u, r, seance.id, &PublierBody::default(), maintenant())
        .await
        .unwrap();
    assert_eq!(p.publiees, 2);
    assert!(
        ent.decision_famille(etab, &d1.jeton, d1.id)
            .await
            .unwrap()
            .publiee
    );
    // Clôture : le dossier non examiné reste à instruire.
    let fin = ent
        .cloturer_commission_for_actor(
            etab,
            u,
            r,
            seance.id,
            &CommissionClotureBody {
                pv_texte: Some("Séance tenue, deux décisions.".into()),
            },
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!(fin.statut, "cloturee");
    assert_eq!(etat(&ent, etab, &d2).await, "en_instruction");
    assert!(ent
        .decider_en_lot_for_actor(
            etab,
            u,
            r,
            seance.id,
            &DecisionsEnLotBody {
                decisions: vec![DecisionEnLotItem {
                    dossier_id: d2.id,
                    decision: corps("refuse", None, "Trop tard")
                }]
            },
            maintenant()
        )
        .await
        .is_err());
    assert!(ent
        .cloturer_commission_for_actor(
            etab,
            u,
            r,
            seance.id,
            &CommissionClotureBody::default(),
            maintenant()
        )
        .await
        .is_err());
}

// ───────────────────────────── Avis et déport ─────────────────────────────

/// Avis internes en ajout seul : la direction les lit tous, un membre du secrétariat ne lit que les siens. Se
/// déporter retire le dossier, ses avis et la décision à celui qui s'en déporte.
#[tokio::test]
async fn i5_avis_et_deport() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (a, ar) = adm(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Avis", None).await;
    let b = AvisBody {
        contexte: "entretien".into(),
        sens: "favorable".into(),
        texte: "Élève motivée, projet clair.".into(),
    };
    assert!(ent
        .donner_avis_for_actor(
            etab,
            a,
            ar,
            d.id,
            &AvisBody {
                sens: "peut-être".into(),
                ..b.clone()
            }
        )
        .await
        .is_err());
    assert!(ent
        .donner_avis_for_actor(
            etab,
            a,
            ar,
            d.id,
            &AvisBody {
                texte: "no".into(),
                ..b.clone()
            }
        )
        .await
        .is_err());
    assert!(
        ent.donner_avis_for_actor(etab, c.prof.user_id, "enseignant", d.id, &b)
            .await
            .is_err(),
        "un professeur sans rôle de commission"
    );
    ent.donner_avis_for_actor(etab, a, ar, d.id, &b)
        .await
        .unwrap();
    ent.donner_avis_for_actor(
        etab,
        u,
        r,
        d.id,
        &AvisBody {
            contexte: "commission".into(),
            sens: "reserve".into(),
            texte: "Attendre le bulletin.".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        ent.avis_dossier_for_actor(etab, u, r, d.id)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        ent.avis_dossier_for_actor(etab, a, ar, d.id)
            .await
            .unwrap()
            .len(),
        1,
        "le secrétariat ne lit que le sien"
    );
    // L'avis est en ajout seul.
    let e = sqlx::query(
        "UPDATE ent_inscription_avis SET texte = 'réécrit' WHERE etablissement_id = $1",
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await;
    assert!(e.is_err());
    // Déport : proposé (adresse commune) jamais imposé ; effectif et définitif pour soi.
    assert!(
        !ent.deport_etat_for_actor(etab, a, ar, d.id)
            .await
            .unwrap()
            .deporte
    );
    assert!(ent
        .se_deporter_for_actor(etab, a, ar, d.id, &DeportBody { motif: "".into() })
        .await
        .is_err());
    ent.se_deporter_for_actor(
        etab,
        a,
        ar,
        d.id,
        &DeportBody {
            motif: "Mon neveu est candidat".into(),
        },
    )
    .await
    .unwrap();
    assert!(
        ent.dossier_for_actor(etab, a, ar, d.id).await.is_err(),
        "le dossier n'est plus visible"
    );
    assert!(ent.avis_dossier_for_actor(etab, a, ar, d.id).await.is_err());
    assert!(ent
        .donner_avis_for_actor(etab, a, ar, d.id, &b)
        .await
        .is_err());
    assert!(
        ent.list_dossiers_for_actor(etab, a, ar, None, None)
            .await
            .unwrap()
            .iter()
            .all(|x| x.id != d.id),
        "ni dans les listes"
    );
    // La direction n'est pas touchée ; un déport de la direction l'empêche de décider.
    assert!(ent.dossier_for_actor(etab, u, r, d.id).await.is_ok());
    ent.se_deporter_for_actor(
        etab,
        u,
        r,
        d.id,
        &DeportBody {
            motif: "Conflit d'intérêts déclaré".into(),
        },
    )
    .await
    .unwrap();
    // D-069 règle 2 : le dossier n'existe plus pour qui s'en est déporté (refus exact : « introuvable »).
    assert!(matches!(
        decider(&ent, &c, &d, &corps("admis", Some(offre), "Admis")).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
}

// ───────────────────────────── Famille ─────────────────────────────

/// La famille ne lit que la décision publiée : jamais le motif interne, jamais un avis, jamais ce qui concerne le
/// second responsable ou les autres candidats ; le message ne contient pas la décision.
#[tokio::test]
async fn i5_la_famille_ne_lit_que_le_publie() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let pere = email("pere-separe");
    let d = dossier(&ent, &c, camp, "Intime", Some(&pere)).await;
    let autre = dossier(&ent, &c, camp, "Autre", None).await;
    ent.donner_avis_for_actor(
        etab,
        u,
        r,
        d.id,
        &AvisBody {
            contexte: "entretien".into(),
            sens: "defavorable".into(),
            texte: "SECRET-AVIS élève agitée".into(),
        },
    )
    .await
    .unwrap();
    let mut b = corps(
        "liste_attente",
        Some(offre),
        "SECRET-MOTIF-INTERNE conflit avec la mère",
    );
    b.motif_communique = Some("Offre complète à ce jour".into());
    decider(&ent, &c, &autre, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    decider(&ent, &c, &d, &b).await.unwrap();
    expedier(&ent).await;
    let vue = ent.decision_famille(etab, &d.jeton, d.id).await.unwrap();
    let json = serde_json::to_string(&vue).unwrap();
    assert_eq!(vue.sens.as_deref(), Some("liste_attente"));
    assert_eq!(
        vue.motif_communique.as_deref(),
        Some("Offre complète à ce jour")
    );
    assert_eq!(vue.position_attente, Some(1));
    for interdit in ["SECRET", "Ollivander", "Garrick", &pere, "Autre"] {
        assert!(!json.contains(interdit), "{interdit} dans {json}");
    }
    let dossier_famille =
        serde_json::to_string(&ent.dossier_famille(etab, &d.jeton, d.id).await.unwrap()).unwrap();
    assert!(!dossier_famille.contains("SECRET"), "{dossier_famille}");
    for (_, sujet, corps) in ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .filter(|(to, _, _)| to == &d.email)
    {
        assert!(
            !corps.contains("SECRET")
                && !corps.contains("liste d'attente")
                && !corps.contains(&pere),
            "{sujet}"
        );
    }
    // Un autre déposant n'a aucun accès (introuvable), le second parent sans compte non plus.
    assert!(ent
        .decision_famille(etab, &autre.jeton, d.id)
        .await
        .is_err());
    assert!(ent
        .decision_famille(etab, "jeton-forge", d.id)
        .await
        .is_err());
    assert!(ent
        .repondre_proposition_famille(
            etab,
            &autre.jeton,
            d.id,
            &ReponsePropositionBody {
                reponse: "acceptee".into()
            },
            maintenant()
        )
        .await
        .is_err());
}

// ───────────────────────────── Désaccord, droits, isolation ─────────────────────────────

/// Un dossier bloqué pour désaccord ne reçoit aucune décision et n'est pas désigné sur la liste d'attente.
#[tokio::test]
async fn i5_desaccord_bloque_decision_et_designation() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let a = dossier(&ent, &c, camp, "Occupe", None).await;
    let b = dossier(&ent, &c, camp, "Bloque", None).await;
    let d3 = dossier(&ent, &c, camp, "Libre", None).await;
    decider(&ent, &c, &a, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    decider(
        &ent,
        &c,
        &b,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    decider(
        &ent,
        &c,
        &d3,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    ent.blocage_for_actor(
        etab,
        u,
        r,
        b.id,
        &BlocageBody {
            bloquer: true,
            motif: "Désaccord déclaré par un titulaire".into(),
        },
    )
    .await
    .unwrap();
    let e = erreur(
        revoir(&ent, &c, &b, &corps("admis", Some(offre), "Admis"))
            .await
            .unwrap_err(),
    );
    assert!(e.contains("bloqué"), "{e}");
    ent.desister_famille(etab, &a.jeton, a.id).await.unwrap();
    // Le dossier bloqué est sauté (sa famille n'a pas à être sollicitée) : le suivant sans blocage est désigné.
    assert!(proposition_ouverte(&ent, etab, &b).await.is_none());
    assert_eq!(
        proposition_ouverte(&ent, etab, &d3).await.unwrap().1,
        "designee"
    );
}

/// Droits et isolation : un parent, un professeur n'accèdent à rien ; un agent d'un autre établissement n'obtient
/// que « introuvable » ; le secrétariat n'a pas le droit de décider.
#[tokio::test]
async fn i5_droits_et_isolation() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let Some((_, c2)) = labo().await else { return };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (a, ar) = adm(&c);
    let (u2, r2) = dir(&c2);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Droits", None).await;
    // Secrétariat : instruit, ne décide pas.
    assert!(ent
        .decider_complete_for_actor(
            etab,
            a,
            ar,
            d.id,
            &corps("refuse", Some(offre), "Refus interdit"),
            false,
            maintenant()
        )
        .await
        .is_err());
    assert!(ent
        .creer_commission_admission_for_actor(
            etab,
            a,
            ar,
            &CommissionBody {
                campagne_id: camp,
                intitule: "Séance".into(),
                seance_le: maintenant(),
                ..Default::default()
            }
        )
        .await
        .is_err());
    // Parent et professeur : rien.
    for (uid, role) in [
        (c.parent_lea.user_id, "parent"),
        (c.prof.user_id, "enseignant"),
    ] {
        assert!(ent
            .decisions_dossier_for_actor(etab, uid, role, d.id)
            .await
            .is_err());
        assert!(ent
            .conditions_dossier_for_actor(etab, uid, role, d.id)
            .await
            .is_err());
        assert!(ent
            .liste_attente_for_actor(etab, uid, role, offre)
            .await
            .is_err());
        assert!(ent
            .propositions_for_actor(etab, uid, role, None, None, maintenant())
            .await
            .is_err());
    }
    // Autre établissement : « introuvable », jamais le contenu.
    let dec = decider(&ent, &c, &d, &corps("refuse", Some(offre), "Hors secteur"))
        .await
        .unwrap();
    assert!(ent
        .decisions_dossier_for_actor(c2.etablissement_id, u2, r2, d.id)
        .await
        .is_err());
    assert!(ent
        .decider_complete_for_actor(
            c2.etablissement_id,
            u2,
            r2,
            d.id,
            &corps("admis", None, "Intrusion"),
            true,
            maintenant()
        )
        .await
        .is_err());
    assert!(ent
        .courrier_decision_for_actor(c2.etablissement_id, u2, r2, dec.id)
        .await
        .is_err());
    assert!(ent
        .publier_decision_for_actor(
            c2.etablissement_id,
            u2,
            r2,
            dec.id,
            &PublierBody::default(),
            maintenant()
        )
        .await
        .is_err());
    assert!(ent
        .liste_attente_for_actor(c2.etablissement_id, u2, r2, offre)
        .await
        .is_err());
    assert!(ent
        .designer_suivants_for_actor(c2.etablissement_id, u2, r2, offre)
        .await
        .is_err());
    assert!(ent
        .donner_avis_for_actor(
            c2.etablissement_id,
            u2,
            r2,
            d.id,
            &AvisBody {
                contexte: "entretien".into(),
                sens: "favorable".into(),
                texte: "avis étranger".into()
            }
        )
        .await
        .is_err());
    // Le courrier ne contient jamais le motif interne.
    let courrier = ent
        .courrier_decision_for_actor(etab, u, r, dec.id)
        .await
        .unwrap();
    assert!(courrier.corps.contains("Droits") && !courrier.corps.contains("Hors secteur"));
}

// ───────────────────────────── Revue de sécurité (passe 1) ─────────────────────────────

/// Une chaîne de décisions non publiées reste invisible partout où la famille lit : vue de la décision, espace,
/// démarche, dossier, suivi ; elle voit encore la dernière décision publiée.
#[tokio::test]
async fn i5_masque_famille_chaine_non_publiee() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Chaine", None).await;
    decider(
        &ent,
        &c,
        &d,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    let mut b = corps("admis_sous_reserve", Some(offre), "Place trouvée");
    b.reserve = Some("TEXTE-RESERVE".into());
    b.publier_le = Some(t("2027-03-10T09:00:00Z"));
    revoir(&ent, &c, &d, &b).await.unwrap();
    let mut b2 = corps("refuse", Some(offre), "Finalement non");
    b2.publier_le = Some(t("2027-03-10T09:00:00Z"));
    revoir(&ent, &c, &d, &b2).await.unwrap();
    let vue = ent.decision_famille(etab, &d.jeton, d.id).await.unwrap();
    assert_eq!(vue.sens.as_deref(), Some("liste_attente"));
    assert!(vue.conditions.is_empty() && vue.proposition.is_none() && vue.actions.is_empty());
    let espace = ent.espace_preinscription(etab, &d.jeton).await.unwrap();
    assert!(espace
        .dossiers
        .iter()
        .all(|x| x.etat == "liste_attente" && x.statut == "liste_attente"));
    let json =
        serde_json::to_string(&ent.dossier_famille(etab, &d.jeton, d.id).await.unwrap()).unwrap();
    assert!(
        !json.contains("TEXTE-RESERVE")
            && !json.contains("\"refuse\"")
            && !json.contains("\"admis\""),
        "{json}"
    );
}

/// Se déporter ferme aussi la validation, l'instruction et la lecture des accords et des pièces.
#[tokio::test]
async fn i5_deport_ferme_toutes_les_routes() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (a, ar) = adm(&c);
    let (camp, offre) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Deporte", None).await;
    decider(&ent, &c, &d, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    ent.se_deporter_for_actor(
        etab,
        a,
        ar,
        d.id,
        &DeportBody {
            motif: "Parent du candidat".into(),
        },
    )
    .await
    .unwrap();
    assert!(ent
        .valider_inscription_for_actor(etab, a, ar, d.id, &ValidationBody::default())
        .await
        .is_err());
    assert!(ent
        .changer_statut_for_actor(
            etab,
            a,
            ar,
            d.id,
            &StatutBody {
                statut: "en_instruction".into(),
                motif: None
            }
        )
        .await
        .is_err());
    assert!(ent
        .accords_dossier_for_actor(etab, a, ar, d.id)
        .await
        .is_err());
    assert!(ent
        .pieces_dossier_for_actor(etab, a, ar, d.id)
        .await
        .is_err());
    assert!(ent
        .commenter_for_actor(
            etab,
            a,
            ar,
            d.id,
            &CommentaireBody {
                texte: "note".into(),
                visible_famille: false
            }
        )
        .await
        .is_err());
    // La purge d'un dossier (cascade) passe par l'unique chemin de purge du dépôt (`SET LOCAL`), jamais autrement.
    assert!(
        sqlx::query("DELETE FROM ent_inscription_dossier WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(d.id)
            .execute(ent.pool_verif())
            .await
            .is_err(),
        "sans le chemin de purge, la cascade est refusée"
    );
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL ent.purge_etablissement = 'on'")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("DELETE FROM ent_inscription_dossier WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(d.id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}
