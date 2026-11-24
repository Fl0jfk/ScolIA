//! Socle 20, lot C8 : visa figé, inspection bornée et journalisée, exports figés, clôture et archive (`20_CAHIER_DE_TEXTES.md`
//! §14), contre la base locale de test (feature `labo`).
//!
//! Sous-module de `tests_cahier` (fixture `Fx`, un établissement neuf par test). Aucun résultat ne dépend de l'heure réelle : les
//! jours sont ceux de l'établissement (`jour_etab`), les séances sont créées à des jours explicites.

use super::*;
use chrono::Duration;
use ent_types::cahier_c2::*;
use ent_types::cahier_c8::*;

async fn jour(fx: &Fx) -> NaiveDate {
    fx.ent.jour_etab(fx.cast.etablissement_id).await.unwrap()
}

async fn cours_a(fx: &Fx) -> Uuid {
    sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE id = $1")
        .bind(fx.service_a)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

/// Premier jour ouvré à partir de `d` (une échéance ne tombe jamais un jour fermé).
async fn ouvre(fx: &Fx, d: NaiveDate) -> NaiveDate {
    sqlx::query_scalar("SELECT x::date FROM generate_series($2::date, $2::date + 60, interval '1 day') AS x WHERE NOT ent_cdt_jour_ferme($1, x::date) ORDER BY 1 LIMIT 1")
        .bind(fx.cast.etablissement_id)
        .bind(d)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

/// Séance du cours A (classe CDT-A, service de `prof0`) au jour donné.
async fn seance_a(fx: &Fx, j: NaiveDate) -> Uuid {
    fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), j)
        .await
}

async fn entree(fx: &Fx, cours: Uuid, seance: Uuid, contenu: &str, publication: &str) -> EntreeDto {
    fx.ent
        .cdt_entree_enregistrer(
            &fx.prof(),
            cours,
            seance,
            EntreeBody {
                titre: Some("Séance".into()),
                contenu: contenu.into(),
                notions: vec!["Fractions".into()],
                publication: publication.into(),
                publie_a: None,
                publie_jour: None,
                publie_heure: None,
                sans_avis: false,
                motif: None,
            },
        )
        .await
        .unwrap()
}

async fn travail(fx: &Fx, cours: Uuid, titre: &str, publication: &str) -> Uuid {
    let j = ouvre(fx, jour(fx).await + Duration::days(7)).await;
    fx.ent
        .cdt_travaux_creer(
            &fx.prof(),
            TravailC2Body {
                cours_ids: vec![cours],
                seance_donnee_id: None,
                echeance_seance_id: None,
                echeance_prochaine_seance: false,
                echeance_jour: Some(j),
                echeance_heure: NaiveTime::from_hms_opt(18, 0, 0),
                nature: "exercices".into(),
                titre: titre.into(),
                consigne: "Consigne commune.".into(),
                duree_estimee_min: Some(30),
                rendu: "aucun".into(),
                retard_accepte: true,
                formats: vec![],
                eleve_ids: vec![],
                ia_autorisee: "non".into(),
                publication: publication.into(),
                publie_a: None,
                publie_jour: None,
                publie_heure: None,
            },
        )
        .await
        .unwrap()
        .travaux[0]
        .id
}

fn visa_cours(
    cours: Uuid,
    du: NaiveDate,
    au: NaiveDate,
    observation: Option<&str>,
) -> VisaCahierBody {
    VisaCahierBody {
        cible: VISA_COURS.into(),
        cours_id: Some(cours),
        classe_id: None,
        agent_id: None,
        du,
        au,
        observation: observation.map(Into::into),
        confirmer_vide: false,
    }
}

async fn agent_de(fx: &Fx, user: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Option<Uuid>>("SELECT ent_cdt_agent_du_compte($1, $2)")
        .bind(fx.cast.etablissement_id)
        .bind(user)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
        .expect("agent du professeur")
}

fn texte(v: &impl serde::Serialize) -> String {
    serde_json::to_string(v).unwrap()
}

// ───────────────────────── visa ─────────────────────────

#[tokio::test]
async fn c8_visa_fige_la_reference_et_une_modification_apres_visa_est_tracee() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let s1 = seance_a(&fx, auj - Duration::days(2)).await;
    let s2 = seance_a(&fx, auj - Duration::days(1)).await;
    let e1 = entree(
        &fx,
        cours,
        s1,
        "Version vue par la direction.",
        "maintenant",
    )
    .await;
    entree(&fx, cours, s2, "Deuxième séance.", "maintenant").await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj).await,
        "Brouillon du jour.",
        "brouillon",
    )
    .await;
    travail(&fx, cours, "Exercices 1", "maintenant").await;
    travail(&fx, cours, "Brouillon de travail", "brouillon").await;

    let v = fx
        .ent
        .cdt_c8_viser(
            &fx.direction(),
            visa_cours(
                cours,
                auj - Duration::days(5),
                auj,
                Some("Pensez à détailler les notions."),
            ),
        )
        .await
        .unwrap();
    // 2 entrées publiées + 1 travail publié figés ; ni brouillon d'entrée ni brouillon de travail.
    assert_eq!(v.nb_elements, 3);
    assert!(!v.vide);
    assert_eq!(v.modifies_apres_visa, 0);
    assert_eq!(v.observations.len(), 1);

    // Le professeur modifie l'entrée vue : nouvelle version, jamais silencieuse.
    let e1b = fx
        .ent
        .cdt_entree_enregistrer(
            &fx.prof(),
            cours,
            s1,
            EntreeBody {
                titre: Some("Séance".into()),
                contenu: "Version modifiée après le visa.".into(),
                notions: vec![],
                publication: "maintenant".into(),
                publie_a: None,
                publie_jour: None,
                publie_heure: None,
                sans_avis: false,
                motif: Some("correction".into()),
            },
        )
        .await
        .unwrap();
    assert!(e1b.version > e1.version);
    let d = fx
        .ent
        .cdt_c8_visa_detail(&fx.direction(), v.id)
        .await
        .unwrap();
    assert_eq!(d.visa.modifies_apres_visa, 1);
    let el = d.elements.iter().find(|e| e.objet_id == e1.id).unwrap();
    assert!(el.modifie_apres_visa);
    assert_eq!(
        el.contenu_vise.as_deref(),
        Some("Version vue par la direction.")
    );
    assert_eq!(
        el.contenu_actuel.as_deref(),
        Some("Version modifiée après le visa.")
    );
    assert_eq!(el.modifications.len(), (e1b.version - e1.version) as usize);
    assert_eq!(el.modifications[0].motif.as_deref(), Some("correction"));
    // L'auteur voit lui aussi la modification (CC-58) ; les éléments non modifiés ne portent aucun texte.
    let dp = fx.ent.cdt_c8_visa_detail(&fx.prof(), v.id).await.unwrap();
    assert_eq!(dp.visa.modifies_apres_visa, 1);
    assert!(dp
        .elements
        .iter()
        .filter(|e| !e.modifie_apres_visa)
        .all(|e| e.contenu_vise.is_none() && e.contenu_actuel.is_none()));
    // Chronologie (écran 3) : la marque suit.
    let ch = fx
        .ent
        .cdt_c8_chronologie(&fx.direction(), cours, auj - Duration::days(5), auj)
        .await
        .unwrap();
    assert!(ch
        .items
        .iter()
        .any(|i| i.objet_id == e1.id && i.modifie_apres_visa));
    assert_eq!(
        ch.items.iter().filter(|i| i.objet == "entree").count(),
        2,
        "les brouillons n'y figurent pas"
    );
    // Tableau de tenue : le compte « modifiés après visa » du cours.
    let t = fx
        .ent
        .cdt_c8_tenue(&fx.direction(), auj - Duration::days(5), auj)
        .await
        .unwrap();
    let l = t.lignes.iter().find(|l| l.cours_id == cours).unwrap();
    assert_eq!(l.modifies_apres_visa, 1);
    assert!(l.dernier_visa_le.is_some() && !l.a_viser);
    // Le visa est figé : aucune réécriture, aucun effacement, aucun TRUNCATE (R-C17).
    assert!(
        sqlx::query("UPDATE ent_cdt_visa SET au = au + 1 WHERE id = $1")
            .bind(v.id)
            .execute(fx.ent.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM ent_cdt_visa_element WHERE visa_id = $1")
            .bind(v.id)
            .execute(fx.ent.pool())
            .await
            .is_err()
    );
    assert!(sqlx::query("TRUNCATE ent_cdt_visa_element")
        .execute(fx.ent.pool())
        .await
        .is_err());
    assert!(
        sqlx::query("UPDATE ent_cdt_observation SET texte = 'x' WHERE visa_id = $1")
            .bind(v.id)
            .execute(fx.ent.pool())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn c8_visa_droits_et_separation_aucun_autre_que_la_direction_ne_vise() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj).await,
        "Contenu.",
        "maintenant",
    )
    .await;
    let b = visa_cours(cours, auj, auj, Some("Observation."));
    for a in [
        fx.prof(),
        fx.prof2(),
        fx.cpe(),
        fx.compta(),
        fx.eleve(&fx.x),
        fx.parent(&fx.x),
    ] {
        assert!(
            fx.ent
                .cdt_c8_viser(&a, b.clone())
                .await
                .is_err_and(|e| est_interdit(&e)),
            "{} ne vise pas",
            a.role
        );
        assert!(fx
            .ent
            .cdt_c8_tenue(&a, auj, auj)
            .await
            .is_err_and(|e| est_interdit(&e)));
    }
    let v = fx.ent.cdt_c8_viser(&fx.direction(), b).await.unwrap();
    // Lecture du visa : l'intervenant oui ; l'autre professeur, la CPE, la comptabilité, l'élève et le parent non.
    assert!(fx.ent.cdt_c8_visa_detail(&fx.prof(), v.id).await.is_ok());
    assert_eq!(
        fx.ent.cdt_c8_visas(&fx.prof(), None).await.unwrap().len(),
        1
    );
    assert!(fx
        .ent
        .cdt_c8_visa_detail(&fx.prof2(), v.id)
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(
        fx.ent
            .cdt_c8_visas(&fx.prof2(), None)
            .await
            .unwrap()
            .is_empty(),
        "l'autre professeur ne voit pas le visa d'un cours qui n'est pas le sien"
    );
    for a in [fx.cpe(), fx.compta(), fx.eleve(&fx.x), fx.parent(&fx.x)] {
        assert!(
            fx.ent
                .cdt_c8_visa_detail(&a, v.id)
                .await
                .is_err_and(|e| est_interdit(&e)),
            "{} ne lit pas un visa",
            a.role
        );
        assert!(fx
            .ent
            .cdt_c8_visas(&a, None)
            .await
            .is_err_and(|e| est_interdit(&e)));
        assert!(fx
            .ent
            .cdt_c8_chronologie(&a, cours, auj, auj)
            .await
            .is_err_and(|e| est_interdit(&e)));
    }
    assert!(fx
        .ent
        .cdt_c8_chronologie(&fx.prof2(), cours, auj, auj)
        .await
        .is_err_and(|e| est_interdit(&e)));
    // Un autre établissement ne voit rien de ce visa (cloisonnement).
    let Some(autre) = fixture().await else { return };
    assert!(autre
        .ent
        .cdt_c8_visa_detail(&autre.direction(), v.id)
        .await
        .is_err_and(|e| est_introuvable(&e)));
    assert!(autre
        .ent
        .cdt_c8_visas(&autre.direction(), Some(cours))
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn c8_visa_d_un_cahier_vide_exige_une_confirmation_explicite() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let mut b = visa_cours(
        cours,
        auj - Duration::days(30),
        auj - Duration::days(20),
        None,
    );
    assert!(fx
        .ent
        .cdt_c8_viser(&fx.direction(), b.clone())
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    b.confirmer_vide = true;
    let v = fx.ent.cdt_c8_viser(&fx.direction(), b).await.unwrap();
    assert!(v.vide && v.nb_elements == 0);
    // Période inversée ou démesurée, cible absente ou inconnue : refus sans écriture.
    assert!(fx
        .ent
        .cdt_c8_viser(
            &fx.direction(),
            visa_cours(cours, auj, auj - Duration::days(1), None)
        )
        .await
        .is_err());
    assert!(fx
        .ent
        .cdt_c8_viser(
            &fx.direction(),
            visa_cours(cours, auj - Duration::days(800), auj, None)
        )
        .await
        .is_err());
    assert!(fx
        .ent
        .cdt_c8_viser(&fx.direction(), visa_cours(Uuid::new_v4(), auj, auj, None))
        .await
        .is_err_and(|e| est_introuvable(&e)));
    let mut sans_cible = visa_cours(cours, auj, auj, None);
    sans_cible.cours_id = None;
    assert!(fx
        .ent
        .cdt_c8_viser(&fx.direction(), sans_cible)
        .await
        .is_err());
    let mut inconnue = visa_cours(cours, auj, auj, None);
    inconnue.cible = "tous".into();
    assert!(fx
        .ent
        .cdt_c8_viser(&fx.direction(), inconnue)
        .await
        .is_err());
}

#[tokio::test]
async fn c8_observation_adressee_au_cours_ou_au_professeur_et_reponse() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj).await,
        "Contenu.",
        "maintenant",
    )
    .await;
    // Visa de classe : un visa seul, aucune observation (pas de destinataire précis).
    let mut classe = VisaCahierBody {
        cible: VISA_CLASSE.into(),
        cours_id: None,
        agent_id: None,
        classe_id: Some(fx.classe_a),
        du: auj,
        au: auj,
        observation: Some("x".into()),
        confirmer_vide: false,
    };
    assert!(fx
        .ent
        .cdt_c8_viser(&fx.direction(), classe.clone())
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    classe.observation = None;
    let vc = fx.ent.cdt_c8_viser(&fx.direction(), classe).await.unwrap();
    assert!(fx
        .ent
        .cdt_c8_observer(
            &fx.direction(),
            vc.id,
            ObservationBody { texte: "x".into() }
        )
        .await
        .is_err());
    // Le professeur de la classe voit le visa mais jamais d'observation.
    let vu = fx.ent.cdt_c8_visas(&fx.prof(), None).await.unwrap();
    assert!(vu
        .iter()
        .any(|v| v.id == vc.id && v.observations.is_empty()));

    let v = fx
        .ent
        .cdt_c8_viser(&fx.direction(), visa_cours(cours, auj, auj, None))
        .await
        .unwrap();
    let v = fx
        .ent
        .cdt_c8_observer(
            &fx.direction(),
            v.id,
            ObservationBody {
                texte: "Détaillez les objectifs.".into(),
            },
        )
        .await
        .unwrap();
    let o = v.observations[0].id;
    // HTML refusé (R-C22), vide refusé.
    assert!(fx
        .ent
        .cdt_c8_observer(
            &fx.direction(),
            v.id,
            ObservationBody {
                texte: "<b>x</b>".into()
            }
        )
        .await
        .is_err());
    assert!(fx
        .ent
        .cdt_c8_observer(
            &fx.direction(),
            v.id,
            ObservationBody { texte: "  ".into() }
        )
        .await
        .is_err());
    // Seule la direction observe ; seuls les intervenants répondent.
    assert!(fx
        .ent
        .cdt_c8_observer(&fx.prof(), v.id, ObservationBody { texte: "x".into() })
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c8_repondre(
            &fx.prof2(),
            o,
            ObservationBody {
                texte: "intrus".into()
            }
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c8_repondre(
            &fx.cpe(),
            o,
            ObservationBody {
                texte: "intrus".into()
            }
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    // F3 : une observation inconnue donne le même refus (403) qu'une observation d'un autre cours.
    assert!(fx
        .ent
        .cdt_c8_repondre(
            &fx.prof(),
            Uuid::new_v4(),
            ObservationBody { texte: "x".into() }
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    let r = fx
        .ent
        .cdt_c8_repondre(
            &fx.prof(),
            o,
            ObservationBody {
                texte: "Ajouté au prochain cours.".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(r.observations[0].reponses.len(), 1);
    // La direction relit la réponse ; l'autre professeur ne lit pas l'observation du cours d'un collègue.
    assert_eq!(
        fx.ent
            .cdt_c8_visa_detail(&fx.direction(), v.id)
            .await
            .unwrap()
            .visa
            .observations[0]
            .reponses
            .len(),
        1
    );
    assert!(fx
        .ent
        .cdt_c8_visa_detail(&fx.prof2(), v.id)
        .await
        .is_err_and(|e| est_interdit(&e)));

    // Visa d'un professeur : l'observation est pour lui seul.
    let ag = agent_de(&fx, fx.cast.profs[0].user_id).await;
    let va = fx
        .ent
        .cdt_c8_viser(
            &fx.direction(),
            VisaCahierBody {
                cible: VISA_AGENT.into(),
                cours_id: None,
                classe_id: None,
                agent_id: Some(ag),
                du: auj,
                au: auj,
                observation: Some("Bravo pour la tenue.".into()),
                confirmer_vide: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(va.nb_elements, 1);
    assert!(
        fx.ent
            .cdt_c8_visa_detail(&fx.prof(), va.id)
            .await
            .unwrap()
            .visa
            .observations
            .len()
            == 1
    );
    assert!(fx
        .ent
        .cdt_c8_visa_detail(&fx.prof2(), va.id)
        .await
        .is_err_and(|e| est_interdit(&e)));
}

#[tokio::test]
async fn c8_tenue_taux_et_agregats_sans_aucun_nom_d_eleve() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let mut seances = Vec::new();
    for k in 1..=4 {
        seances.push(seance_a(&fx, auj - Duration::days(k)).await);
    }
    // Une séance annulée ne compte pas dans les attendues.
    let annulee = seance_a(&fx, auj - Duration::days(5)).await;
    sqlx::query("UPDATE ent_edt_seance SET statut = 'annulee' WHERE id = $1")
        .bind(annulee)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    entree(&fx, cours, seances[0], "Une.", "maintenant").await;
    entree(&fx, cours, seances[1], "Deux.", "maintenant").await;
    entree(
        &fx,
        cours,
        seances[2],
        "Brouillon, ne compte pas.",
        "brouillon",
    )
    .await;
    travail(&fx, cours, "Travail", "maintenant").await;
    let t = fx
        .ent
        .cdt_c8_tenue(&fx.direction(), auj - Duration::days(10), auj)
        .await
        .unwrap();
    let l = t
        .lignes
        .iter()
        .find(|l| l.cours_id == cours)
        .expect("ligne du cours");
    assert_eq!(
        (l.seances_passees, l.seances_renseignees, l.taux_pct),
        (4, 2, Some(50))
    );
    assert_eq!(l.travaux_donnes, 1);
    assert_eq!(l.derniere_entree, Some(auj - Duration::days(1)));
    assert!(l.a_viser, "jamais visé");
    assert_eq!(l.public, "CDT-A");
    let json = texte(&t);
    for e in [&fx.x, &fx.y, &fx.z, &fx.w] {
        assert!(
            !json.contains(&e.nom) && !json.contains(&e.prenom),
            "aucun nom d'élève dans le tableau de tenue"
        );
    }
}

// ───────────────────────── inspection ─────────────────────────

fn acces(inspecteur: Uuid, cours: Uuid, du: NaiveDate, au: NaiveDate) -> AccesInspectionBody {
    AccesInspectionBody {
        beneficiaire_user_id: inspecteur,
        cours_ids: vec![cours],
        agent_id: None,
        du_cahier: du,
        au_cahier: au,
        duree_jours: None,
        motif: "inspection".into(),
    }
}

#[tokio::test]
async fn c8_inspection_bornee_journalisee_et_sans_donnee_d_eleve() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let s = seance_a(&fx, auj - Duration::days(1)).await;
    entree(
        &fx,
        cours,
        s,
        "Entrée publiée lue par l'inspecteur.",
        "maintenant",
    )
    .await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj).await,
        "Brouillon invisible.",
        "brouillon",
    )
    .await;
    travail(&fx, cours, "Consigne publiée", "maintenant").await;
    // Un élève et son responsable déposent une remise et un retour existe : l'inspecteur n'en voit rien.
    let insp = &fx.cast.compta;
    let a_insp = act(fx.cast.etablissement_id, insp);

    // Seule la direction accorde ; ni un élève ni un responsable ne peut être bénéficiaire.
    for a in [
        fx.prof(),
        fx.cpe(),
        fx.compta(),
        fx.eleve(&fx.x),
        fx.parent(&fx.x),
    ] {
        assert!(fx
            .ent
            .cdt_c8_acces_accorder(&a, acces(insp.user_id, cours, auj - Duration::days(7), auj))
            .await
            .is_err_and(|e| est_interdit(&e)));
    }
    let eleve_user = fx.x.compte.as_ref().unwrap().user_id;
    assert!(fx
        .ent
        .cdt_c8_acces_accorder(&fx.direction(), acces(eleve_user, cours, auj, auj))
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    assert!(fx
        .ent
        .cdt_c8_acces_accorder(&fx.direction(), acces(Uuid::new_v4(), cours, auj, auj))
        .await
        .is_err_and(|e| est_introuvable(&e)));
    let mut trop = acces(insp.user_id, cours, auj, auj);
    trop.duree_jours = Some(91);
    assert!(fx
        .ent
        .cdt_c8_acces_accorder(&fx.direction(), trop)
        .await
        .is_err());
    let mut vide = acces(insp.user_id, cours, auj, auj);
    vide.cours_ids.clear();
    assert!(fx
        .ent
        .cdt_c8_acces_accorder(&fx.direction(), vide)
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));

    let acc = fx
        .ent
        .cdt_c8_acces_accorder(
            &fx.direction(),
            acces(insp.user_id, cours, auj - Duration::days(7), auj),
        )
        .await
        .unwrap();
    assert_eq!(acc.etat, ACCES_ACTIF);
    assert_eq!(acc.cours.len(), 1);
    assert_eq!(
        (acc.valable_au - acc.valable_du).num_days(),
        30,
        "30 jours par défaut"
    );
    assert_eq!(fx.ent.cdt_c8_mes_acces(&a_insp).await.unwrap().len(), 1);
    assert!(fx.ent.cdt_c8_mes_acces(&fx.cpe()).await.unwrap().is_empty());

    let c = fx
        .ent
        .cdt_c8_inspection_lire(&a_insp, acc.id, cours)
        .await
        .unwrap();
    assert_eq!(c.entrees.len(), 1);
    assert_eq!(c.entrees[0].contenu, "Entrée publiée lue par l'inspecteur.");
    assert_eq!(c.entrees[0].notions, vec!["Fractions".to_string()]);
    assert_eq!(c.travaux.len(), 1);
    // Aucune donnée d'élève, de remise, de retour ni de variante dans ce qui est servi.
    let json = texte(&c);
    for e in [&fx.x, &fx.y, &fx.z, &fx.w] {
        assert!(!json.contains(&e.nom) && !json.contains(&e.prenom));
    }
    for interdit in ["remise", "retour", "variante", "eleve"] {
        assert!(
            !json.to_lowercase().contains(interdit),
            "« {interdit} » ne figure pas dans la lecture d'inspection"
        );
    }
    // Chaque lecture est journalisée ; le journal est lu de la direction et du professeur du cours, de personne d'autre.
    fx.ent
        .cdt_c8_inspection_lire(&a_insp, acc.id, cours)
        .await
        .unwrap();
    let journal = fx
        .ent
        .cdt_c8_acces_journal(&fx.direction(), acc.id)
        .await
        .unwrap();
    assert_eq!(journal.len(), 2);
    assert_eq!(
        fx.ent
            .cdt_c8_acces_journal(&fx.prof(), acc.id)
            .await
            .unwrap()
            .len(),
        2
    );
    assert!(fx
        .ent
        .cdt_c8_acces_journal(&fx.prof2(), acc.id)
        .await
        .is_err_and(|e| est_interdit(&e)));
    for a in [fx.cpe(), fx.compta(), fx.eleve(&fx.x), fx.parent(&fx.x)] {
        assert!(
            fx.ent
                .cdt_c8_acces_journal(&a, acc.id)
                .await
                .is_err_and(|e| est_interdit(&e)),
            "{}",
            a.role
        );
    }
    let liste = fx.ent.cdt_c8_acces_lister(&fx.prof()).await.unwrap();
    assert_eq!(liste.len(), 1);
    assert_eq!(liste[0].nb_consultations, 2);
    assert!(
        fx.ent
            .cdt_c8_acces_lister(&fx.prof2())
            .await
            .unwrap()
            .is_empty(),
        "un professeur étranger au cours n'est pas concerné"
    );
    assert!(fx
        .ent
        .cdt_c8_acces_lister(&fx.cpe())
        .await
        .is_err_and(|e| est_interdit(&e)));

    // Hors accès : un autre compte, un cours non ouvert, un accès inconnu → 403.
    assert!(fx
        .ent
        .cdt_c8_inspection_lire(&fx.cpe(), acc.id, cours)
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c8_inspection_lire(&a_insp, acc.id, Uuid::new_v4())
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c8_inspection_lire(&a_insp, Uuid::new_v4(), cours)
        .await
        .is_err_and(|e| est_interdit(&e)));
    // Le journal ne s'efface ni ne se réécrit.
    assert!(
        sqlx::query("DELETE FROM ent_cdt_consultation WHERE acces_id = $1")
            .bind(acc.id)
            .execute(fx.ent.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE ent_cdt_acces_inspection SET motif = 'autre' WHERE id = $1")
            .bind(acc.id)
            .execute(fx.ent.pool())
            .await
            .is_err()
    );
    assert!(sqlx::query("UPDATE ent_cdt_acces_inspection SET valable_au = valable_au - interval '1 day' WHERE id = $1").bind(acc.id).execute(fx.ent.pool()).await.is_err());

    // Révocation : effet immédiat, trace conservée, plus de prolongation.
    let r = fx
        .ent
        .cdt_c8_acces_revoquer(&fx.direction(), acc.id)
        .await
        .unwrap();
    assert_eq!(r.etat, ACCES_REVOQUE);
    assert!(fx
        .ent
        .cdt_c8_inspection_lire(&a_insp, acc.id, cours)
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c8_acces_prolonger(&fx.direction(), acc.id, ProlongerBody { jours: 5 })
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    assert!(fx
        .ent
        .cdt_c8_acces_revoquer(&fx.direction(), acc.id)
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
}

#[tokio::test]
async fn c8_inspection_expiree_403_prolongation_par_la_direction_seule_et_plafond_90_jours() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj).await,
        "Contenu.",
        "maintenant",
    )
    .await;
    let insp = act(fx.cast.etablissement_id, &fx.cast.compta);
    let etab = fx.cast.etablissement_id;
    // Un accès déjà expiré (octroyé il y a 40 jours pour 30) : inséré directement, la base accepte l'historique.
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_cdt_acces_inspection (id, etablissement_id, beneficiaire_user_id, du_cahier, au_cahier, valable_du, valable_au, motif, accorde_par_user_id)
         VALUES ($1, $2, $3, $4, $5, now() - interval '40 days', now() - interval '10 days', 'inspection', $6)",
    )
    .bind(id)
    .bind(etab)
    .bind(fx.cast.compta.user_id)
    .bind(auj)
    .bind(auj)
    .bind(fx.cast.direction.user_id)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_cdt_acces_inspection_cours (etablissement_id, acces_id, cours_id) VALUES ($1, $2, $3)").bind(etab).bind(id).bind(cours).execute(fx.ent.pool()).await.unwrap();
    assert!(
        fx.ent
            .cdt_c8_inspection_lire(&insp, id, cours)
            .await
            .is_err_and(|e| est_interdit(&e)),
        "CC-59 : accès expiré → 403"
    );
    assert_eq!(
        fx.ent.cdt_c8_mes_acces(&insp).await.unwrap()[0].etat,
        ACCES_EXPIRE
    );
    // L'inspecteur ne se prolonge pas lui-même ; la prolongation est de la direction seule.
    for a in [act(etab, &fx.cast.compta), fx.prof(), fx.cpe()] {
        assert!(fx
            .ent
            .cdt_c8_acces_prolonger(&a, id, ProlongerBody { jours: 10 })
            .await
            .is_err_and(|e| est_interdit(&e)));
        assert!(fx
            .ent
            .cdt_c8_acces_revoquer(&a, id)
            .await
            .is_err_and(|e| est_interdit(&e)));
    }
    // 40 jours déjà écoulés depuis le début : 90 jours au total au plus (plafond), jamais plus.
    assert!(fx
        .ent
        .cdt_c8_acces_prolonger(&fx.direction(), id, ProlongerBody { jours: 0 })
        .await
        .is_err());
    assert!(fx
        .ent
        .cdt_c8_acces_prolonger(&fx.direction(), id, ProlongerBody { jours: 80 })
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    let p = fx
        .ent
        .cdt_c8_acces_prolonger(&fx.direction(), id, ProlongerBody { jours: 30 })
        .await
        .unwrap();
    assert_eq!(p.etat, ACCES_ACTIF);
    assert!(fx
        .ent
        .cdt_c8_inspection_lire(&insp, id, cours)
        .await
        .is_ok());
    // Un accès n'existe pas pour un autre établissement.
    assert!(fx
        .ent
        .cdt_c8_acces_prolonger(&fx.direction(), Uuid::new_v4(), ProlongerBody { jours: 5 })
        .await
        .is_err_and(|e| est_introuvable(&e)));
}

#[tokio::test]
async fn c8_inspection_d_un_professeur_ouvre_ses_cours_de_la_periode() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj).await,
        "Contenu.",
        "maintenant",
    )
    .await;
    let ag = agent_de(&fx, fx.cast.profs[0].user_id).await;
    let mut b = acces(fx.cast.compta.user_id, cours, auj, auj);
    b.cours_ids.clear();
    b.agent_id = Some(ag);
    b.motif = "rdv_carriere".into();
    let acc = fx
        .ent
        .cdt_c8_acces_accorder(&fx.direction(), b)
        .await
        .unwrap();
    assert!(acc.cours.iter().any(|c| c.cours_id == cours));
    assert!(acc.agent_cible.is_some());
    // Le professeur visé est concerné : il lit la liste et le journal de son rendez-vous de carrière.
    assert_eq!(
        fx.ent.cdt_c8_acces_lister(&fx.prof()).await.unwrap().len(),
        1
    );
    assert!(fx
        .ent
        .cdt_c8_acces_journal(&fx.prof(), acc.id)
        .await
        .is_ok());
}

// ───────────────────────── exports figés ─────────────────────────

#[tokio::test]
async fn c8_export_fige_empreinte_stable_et_sans_donnee_d_eleve() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let s = seance_a(&fx, auj - Duration::days(1)).await;
    entree(&fx, cours, s, "Texte exporté.", "maintenant").await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj).await,
        "Brouillon non exporté.",
        "brouillon",
    )
    .await;
    travail(&fx, cours, "Devoir exporté", "maintenant").await;
    let b = ExportBody {
        nature: EXPORT_LITIGE.into(),
        cours_id: Some(cours),
        agent_id: None,
        du: auj - Duration::days(5),
        au: auj,
        motif: Some("Demande de communication de la famille".into()),
    };
    for a in [
        fx.prof(),
        fx.cpe(),
        fx.compta(),
        fx.eleve(&fx.x),
        fx.parent(&fx.x),
    ] {
        assert!(
            fx.ent
                .cdt_c8_export_produire(&a, b.clone())
                .await
                .is_err_and(|e| est_interdit(&e)),
            "{}",
            a.role
        );
        assert!(fx
            .ent
            .cdt_c8_exports_lister(&a)
            .await
            .is_err_and(|e| est_interdit(&e)));
    }
    let e1 = fx
        .ent
        .cdt_c8_export_produire(&fx.direction(), b.clone())
        .await
        .unwrap();
    let e2 = fx
        .ent
        .cdt_c8_export_produire(&fx.direction(), b.clone())
        .await
        .unwrap();
    assert_ne!(e1.id, e2.id);
    assert_eq!(
        e1.empreinte, e2.empreinte,
        "empreinte stable : même cahier, même texte"
    );
    assert_eq!((e1.nb_entrees, e1.nb_travaux), (1, 1));
    let c = fx
        .ent
        .cdt_c8_export_lire(&fx.direction(), e1.id)
        .await
        .unwrap();
    assert!(c.contenu.contains("Texte exporté.") && c.contenu.contains("Devoir exporté"));
    assert!(!c.contenu.contains("Brouillon non exporté."));
    for e in [&fx.x, &fx.y, &fx.z, &fx.w] {
        assert!(!c.contenu.contains(&e.nom) && !c.contenu.contains(&e.prenom));
    }
    // Empreinte recalculée par la base : un export ne peut pas être falsifié, ni modifié, ni vidé.
    let (ok,): (bool,) =
        sqlx::query_as("SELECT encode(sha256(convert_to($1, 'UTF8')), 'hex') = $2")
            .bind(&c.contenu)
            .bind(&c.export.empreinte)
            .fetch_one(fx.ent.pool())
            .await
            .unwrap();
    assert!(ok);
    assert!(
        sqlx::query("UPDATE ent_cdt_export SET contenu = 'x' WHERE id = $1")
            .bind(e1.id)
            .execute(fx.ent.pool())
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM ent_cdt_export WHERE id = $1")
        .bind(e1.id)
        .execute(fx.ent.pool())
        .await
        .is_err());
    assert!(sqlx::query("TRUNCATE ent_cdt_export")
        .execute(fx.ent.pool())
        .await
        .is_err());
    let faux = sqlx::query(
        "INSERT INTO ent_cdt_export (id, etablissement_id, nature, du, au, titre, contenu, empreinte, nb_entrees, nb_travaux, produit_par_user_id)
         VALUES ($1, $2, 'autre', $3, $3, 't', 'contenu', $4, 0, 0, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(fx.cast.etablissement_id)
    .bind(auj)
    .bind("0".repeat(64))
    .bind(fx.cast.direction.user_id)
    .execute(fx.ent.pool())
    .await;
    assert!(
        faux.is_err(),
        "une empreinte qui ne correspond pas au contenu est refusée par la base"
    );
    // Litige sans motif, nature inconnue, cible double ou absente : refus.
    let mut sans_motif = b.clone();
    sans_motif.motif = None;
    assert!(fx
        .ent
        .cdt_c8_export_produire(&fx.direction(), sans_motif)
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    let mut inconnue = b.clone();
    inconnue.nature = "tout".into();
    assert!(fx
        .ent
        .cdt_c8_export_produire(&fx.direction(), inconnue)
        .await
        .is_err());
    let mut deux = b.clone();
    deux.agent_id = Some(agent_de(&fx, fx.cast.profs[0].user_id).await);
    assert!(fx
        .ent
        .cdt_c8_export_produire(&fx.direction(), deux)
        .await
        .is_err());
    assert!(
        fx.ent
            .cdt_c8_exports_lister(&fx.direction())
            .await
            .unwrap()
            .len()
            >= 2
    );
    assert!(fx
        .ent
        .cdt_c8_export_lire(&fx.prof(), e1.id)
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c8_export_lire(&fx.direction(), Uuid::new_v4())
        .await
        .is_err_and(|e| est_introuvable(&e)));
}

#[tokio::test]
async fn c8_export_du_professeur_est_prive_et_sans_eleve() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj).await,
        "Ma séance.",
        "maintenant",
    )
    .await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj - Duration::days(1)).await,
        "Mon brouillon.",
        "brouillon",
    )
    .await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(fx.cast.etablissement_id)
        .await
        .unwrap()
        .id;
    // M2 : l'aperçu (GET) ne stocke rien ; l'export conservé (POST) est un document figé à son nom.
    let apercu = fx
        .ent
        .cdt_c8_export_professeur(&fx.prof(), annee)
        .await
        .unwrap();
    let (avant,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_cdt_export WHERE etablissement_id = $1 AND nature = 'professeur'",
    )
    .bind(fx.cast.etablissement_id)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap();
    assert_eq!(avant, 0, "un aperçu n'écrit rien en base");
    let e = fx
        .ent
        .cdt_c8_export_professeur_conserver(&fx.prof(), annee)
        .await
        .unwrap();
    assert_eq!(
        apercu.empreinte, e.export.empreinte,
        "même texte, même empreinte"
    );
    assert_eq!(apercu.contenu, e.contenu);
    assert!(
        e.contenu.contains("Ma séance.")
            && e.contenu.contains("Mon brouillon.")
            && e.contenu.contains("[brouillon]")
    );
    for el in [&fx.x, &fx.y, &fx.z, &fx.w] {
        assert!(!e.contenu.contains(&el.nom) && !e.contenu.contains(&el.prenom));
    }
    // Autre professeur : n'exporte que le sien (rien ici) ; la direction ne lit pas un export personnel ; élèves et familles non.
    let autre = fx
        .ent
        .cdt_c8_export_professeur(&fx.prof2(), annee)
        .await
        .unwrap();
    assert!(!autre.contenu.contains("Ma séance."));
    assert!(fx
        .ent
        .cdt_c8_export_lire(&fx.prof2(), e.export.id)
        .await
        .is_err_and(|x| est_interdit(&x)));
    assert!(fx
        .ent
        .cdt_c8_export_lire(&fx.direction(), e.export.id)
        .await
        .is_err_and(|x| est_interdit(&x)));
    assert!(fx
        .ent
        .cdt_c8_export_lire(&fx.prof(), e.export.id)
        .await
        .is_ok());
    assert!(fx
        .ent
        .cdt_c8_exports_lister(&fx.direction())
        .await
        .unwrap()
        .iter()
        .all(|x| x.nature != "professeur"));
    for a in [fx.eleve(&fx.x), fx.parent(&fx.x), fx.cpe(), fx.compta()] {
        assert!(
            fx.ent
                .cdt_c8_export_professeur(&a, annee)
                .await
                .is_err_and(|x| est_interdit(&x)),
            "{}",
            a.role
        );
    }
    assert!(fx
        .ent
        .cdt_c8_export_professeur(&fx.prof(), Uuid::new_v4())
        .await
        .is_err_and(|x| est_introuvable(&x)));
}

// ───────────────────────── clôture et archive ─────────────────────────

#[tokio::test]
async fn c8_cloture_fige_le_cahier_archive_avec_empreinte_et_rouverture_tracee() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap();
    let s = seance_a(&fx, auj - Duration::days(1)).await;
    entree(&fx, cours, s, "Contenu archivé.", "maintenant").await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj).await,
        "Brouillon jamais publié.",
        "brouillon",
    )
    .await;
    travail(&fx, cours, "Travail archivé", "maintenant").await;

    for a in [
        fx.prof(),
        fx.cpe(),
        fx.compta(),
        fx.eleve(&fx.x),
        fx.parent(&fx.x),
    ] {
        assert!(
            fx.ent
                .cdt_c8_cloture_controle(&a, annee.id)
                .await
                .is_err_and(|e| est_interdit(&e)),
            "{}",
            a.role
        );
        assert!(fx
            .ent
            .cdt_c8_cloturer(
                &a,
                ClotureBody {
                    annee_id: annee.id,
                    confirmer: true
                }
            )
            .await
            .is_err_and(|e| est_interdit(&e)));
        assert!(fx
            .ent
            .cdt_c8_rouvrir(
                &a,
                ReouvrirBody {
                    annee_id: annee.id,
                    motif: "motif assez long".into()
                }
            )
            .await
            .is_err_and(|e| est_interdit(&e)));
        assert!(fx
            .ent
            .cdt_c8_archives(&a, None)
            .await
            .is_err_and(|e| est_interdit(&e)));
    }
    // Contrôle : ce cours porte du contenu publié mais n'est pas visé ; un brouillon d'entrée reste.
    let c = fx
        .ent
        .cdt_c8_cloture_controle(&fx.direction(), annee.id)
        .await
        .unwrap();
    assert!(!c.deja_fige);
    assert!(c.cours_non_vises.iter().any(|x| x.cours_id == cours));
    assert_eq!(c.entrees_brouillon, 1);
    // Sans confirmation : refusé (CC-60), rien n'est figé.
    assert!(fx
        .ent
        .cdt_c8_cloturer(
            &fx.direction(),
            ClotureBody {
                annee_id: annee.id,
                confirmer: false
            }
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    assert!(
        !fx.ent
            .cdt_c8_cloture_controle(&fx.direction(), annee.id)
            .await
            .unwrap()
            .deja_fige
    );
    entree(
        &fx,
        cours,
        s,
        "Encore modifiable avant la clôture.",
        "maintenant",
    )
    .await;

    let r = fx
        .ent
        .cdt_c8_cloturer(
            &fx.direction(),
            ClotureBody {
                annee_id: annee.id,
                confirmer: true,
            },
        )
        .await
        .unwrap();
    assert!(r.fige && r.archives_produites >= 1);
    assert!(
        fx.ent
            .cdt_c8_cloture_controle(&fx.direction(), annee.id)
            .await
            .unwrap()
            .deja_fige
    );
    assert!(
        fx.ent
            .cdt_c8_cloturer(
                &fx.direction(),
                ClotureBody {
                    annee_id: annee.id,
                    confirmer: true
                }
            )
            .await
            .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)),
        "déjà figé"
    );
    // Plus aucune écriture dans le cahier de l'année (R-C19).
    let ecrire = fx
        .ent
        .cdt_entree_enregistrer(
            &fx.prof(),
            cours,
            s,
            EntreeBody {
                titre: None,
                contenu: "Après la clôture.".into(),
                notions: vec![],
                publication: "maintenant".into(),
                publie_a: None,
                publie_jour: None,
                publie_heure: None,
                sans_avis: false,
                motif: None,
            },
        )
        .await;
    assert!(
        ecrire.is_err_and(|e| est_refus(&e, RefusMetier::Conflit)),
        "cahier figé"
    );

    // Archive : texte figé, empreinte contrôlée par la base, sans donnée d'élève ; illisible hors direction.
    let archives = fx
        .ent
        .cdt_c8_archives(&fx.direction(), Some(annee.id))
        .await
        .unwrap();
    let arc = archives
        .iter()
        .find(|x| x.cours.cours_id == cours)
        .expect("archive du cours");
    assert_eq!((arc.version, arc.decision.as_str()), (1, "a_decider"));
    let contenu = fx
        .ent
        .cdt_c8_archive_contenu(&fx.direction(), arc.id)
        .await
        .unwrap();
    assert_eq!(contenu.export.empreinte, arc.empreinte);
    assert!(
        contenu
            .contenu
            .contains("Encore modifiable avant la clôture.")
            && contenu.contenu.contains("Travail archivé")
    );
    assert!(!contenu.contenu.contains("Brouillon jamais publié."));
    for e in [&fx.x, &fx.y, &fx.z, &fx.w] {
        assert!(!contenu.contenu.contains(&e.nom) && !contenu.contenu.contains(&e.prenom));
    }
    assert!(fx
        .ent
        .cdt_c8_archive_contenu(&fx.prof(), arc.id)
        .await
        .is_err_and(|e| est_interdit(&e)));
    // Conservation : échéance = fin d'année + 6 ans ; aucune décision avant ; l'archive est immuable.
    let (attendue,): (NaiveDate,) = sqlx::query_as(
        "SELECT (date_fin + interval '6 years')::date FROM ent_annee_scolaire WHERE id = $1",
    )
    .bind(annee.id)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap();
    assert_eq!(arc.echeance_archive, attendue);
    assert!(fx
        .ent
        .cdt_c8_archive_decider(
            &fx.direction(),
            arc.id,
            ArchiveDecisionBody {
                decision: "detruite".into()
            }
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    assert!(
        sqlx::query("UPDATE ent_cdt_archive SET empreinte = $2 WHERE id = $1")
            .bind(arc.id)
            .bind("1".repeat(64))
            .execute(fx.ent.pool())
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM ent_cdt_archive WHERE id = $1")
        .bind(arc.id)
        .execute(fx.ent.pool())
        .await
        .is_err());
    assert!(sqlx::query("TRUNCATE ent_cdt_archive")
        .execute(fx.ent.pool())
        .await
        .is_err());
    assert!(
        sqlx::query("DELETE FROM ent_cdt_cloture WHERE annee_id = $1")
            .bind(annee.id)
            .execute(fx.ent.pool())
            .await
            .is_err()
    );
    assert!(sqlx::query("TRUNCATE ent_cdt_cloture")
        .execute(fx.ent.pool())
        .await
        .is_err());

    // Réouverture : motif écrit obligatoire, tracée ; le cahier redevient modifiable ; une nouvelle clôture archive en version 2.
    // M1 : motif de 20 caractères au moins ; la demande ne rouvre rien ; une SECONDE personne de la direction valide.
    assert!(fx
        .ent
        .cdt_c8_rouvrir(
            &fx.direction(),
            ReouvrirBody {
                annee_id: annee.id,
                motif: "Erreur de saisie".into() // 16 caractères : trop court
            }
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    let demande = fx
        .ent
        .cdt_c8_rouvrir(
            &fx.direction(),
            ReouvrirBody {
                annee_id: annee.id,
                motif: "Erreur de saisie à corriger avant l'archivage".into(),
            },
        )
        .await
        .unwrap();
    assert!(
        demande.fige && demande.reouverture_demandee,
        "la demande seule ne rouvre pas le cahier"
    );
    let c = fx
        .ent
        .cdt_c8_cloture_controle(&fx.direction(), annee.id)
        .await
        .unwrap();
    assert!(c.deja_fige);
    assert!(c
        .reouverture_en_attente
        .as_ref()
        .is_some_and(|d| d.demandee_par_vous));
    assert!(fx
        .ent
        .cdt_c8_cloture_controle(&fx.direction2(), annee.id)
        .await
        .unwrap()
        .reouverture_en_attente
        .is_some_and(|d| !d.demandee_par_vous));
    // Le demandeur ne valide pas sa propre demande (403) ; une deuxième demande en attente : conflit.
    assert!(fx
        .ent
        .cdt_c8_rouvrir_valider(
            &fx.direction(),
            ValiderReouvertureBody { annee_id: annee.id }
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c8_rouvrir(
            &fx.direction2(),
            ReouvrirBody {
                annee_id: annee.id,
                motif: "Une deuxième demande en attente".into()
            }
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    // Toujours figé : l'écriture est refusée tant que la seconde personne n'a pas validé.
    assert!(fx
        .ent
        .cdt_entree_enregistrer(
            &fx.prof(),
            cours,
            s,
            EntreeBody {
                titre: Some("Séance".into()),
                contenu: "Écrit pendant la demande.".into(),
                notions: vec![],
                publication: "maintenant".into(),
                publie_a: None,
                publie_jour: None,
                publie_heure: None,
                sans_avis: false,
                motif: None,
            }
        )
        .await
        .is_err());
    // Un administrateur technique n'a pas le droit (M1), ni un professeur.
    assert!(fx
        .ent
        .cdt_c8_rouvrir_valider(&fx.admin(), ValiderReouvertureBody { annee_id: annee.id })
        .await
        .is_err_and(|e| est_interdit(&e)));
    let ok = fx
        .ent
        .cdt_c8_rouvrir_valider(
            &fx.direction2(),
            ValiderReouvertureBody { annee_id: annee.id },
        )
        .await
        .unwrap();
    assert!(!ok.fige && !ok.reouverture_demandee);
    // Plus aucune demande en attente : valider de nouveau est un conflit ; rouvrir un cahier ouvert aussi.
    assert!(fx
        .ent
        .cdt_c8_rouvrir_valider(
            &fx.direction2(),
            ValiderReouvertureBody { annee_id: annee.id }
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    assert!(fx
        .ent
        .cdt_c8_rouvrir(
            &fx.direction(),
            ReouvrirBody {
                annee_id: annee.id,
                motif: "Une deuxième fois, cahier déjà ouvert".into()
            }
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    entree(&fx, cours, s, "Corrigé après la réouverture.", "maintenant").await;
    fx.ent
        .cdt_c8_cloturer(
            &fx.direction(),
            ClotureBody {
                annee_id: annee.id,
                confirmer: true,
            },
        )
        .await
        .unwrap();
    let v2 = fx
        .ent
        .cdt_c8_archives(&fx.direction(), Some(annee.id))
        .await
        .unwrap();
    assert!(
        v2.iter()
            .any(|x| x.cours.cours_id == cours && x.version == 2),
        "nouvelle archive, la précédente est gardée"
    );
    assert!(v2
        .iter()
        .any(|x| x.cours.cours_id == cours && x.version == 1));
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_cdt_cloture WHERE annee_id = $1")
        .bind(annee.id)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(
        n, 4,
        "clôture, demande, réouverture, clôture : rien n'est effacé"
    );
}

#[tokio::test]
async fn c8_archive_decision_apres_echeance_une_seule_fois_et_annee_close_au_socle_5() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap();
    entree(
        &fx,
        cours,
        seance_a(&fx, auj).await,
        "Contenu.",
        "maintenant",
    )
    .await;
    fx.ent
        .cdt_c8_cloturer(
            &fx.direction(),
            ClotureBody {
                annee_id: annee.id,
                confirmer: true,
            },
        )
        .await
        .unwrap();
    // Échéance simulée dans le passé (insertion d'une archive ancienne : l'échéance d'une archive ne se modifie pas).
    let export_id: Uuid = sqlx::query_scalar(
        "SELECT export_id FROM ent_cdt_archive WHERE etablissement_id = $1 LIMIT 1",
    )
    .bind(etab)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap();
    let ancienne = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_cdt_archive (id, etablissement_id, annee_id, cours_id, export_id, empreinte, produite_le, version, echeance_archive)
         SELECT $2, etablissement_id, annee_id, cours_id, export_id, empreinte, produite_le, 9, ($3::date - 1) FROM ent_cdt_archive WHERE export_id = $1 LIMIT 1",
    )
    .bind(export_id)
    .bind(ancienne)
    .bind(auj)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    assert!(fx
        .ent
        .cdt_c8_archive_decider(
            &fx.direction(),
            ancienne,
            ArchiveDecisionBody {
                decision: "peut-être".into()
            }
        )
        .await
        .is_err());
    assert!(fx
        .ent
        .cdt_c8_archive_decider(
            &fx.prof(),
            ancienne,
            ArchiveDecisionBody {
                decision: "versee".into()
            }
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    let d = fx
        .ent
        .cdt_c8_archive_decider(
            &fx.direction(),
            ancienne,
            ArchiveDecisionBody {
                decision: "versee".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(d.decision, "versee");
    assert!(d.decide_le.is_some());
    assert!(
        fx.ent
            .cdt_c8_archive_decider(
                &fx.direction(),
                ancienne,
                ArchiveDecisionBody {
                    decision: "detruite".into()
                }
            )
            .await
            .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)),
        "décision définitive"
    );
    assert!(fx
        .ent
        .cdt_c8_archive_decider(
            &fx.direction(),
            Uuid::new_v4(),
            ArchiveDecisionBody {
                decision: "versee".into()
            }
        )
        .await
        .is_err_and(|e| est_introuvable(&e)));
    // Année close au socle 5 : la réouverture du cahier est refusée (c'est l'année qu'il faut rouvrir d'abord).
    sqlx::query("UPDATE ent_annee_scolaire SET cloturee_le = now() WHERE id = $1")
        .bind(annee.id)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    assert!(fx
        .ent
        .cdt_c8_rouvrir(
            &fx.direction(),
            ReouvrirBody {
                annee_id: annee.id,
                motif: "Motif suffisamment long pour la demande".into()
            }
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
}

// ───────────────────────── correctifs de revue : E1, M5, M1, M2, M7, F6 ─────────────────────────

/// Code SQLSTATE d'une écriture refusée par la base (jamais une erreur serveur anonyme).
fn code_sql(r: Result<sqlx::postgres::PgQueryResult, sqlx::Error>) -> Option<String> {
    match r {
        Err(sqlx::Error::Database(d)) => d.code().map(|c| c.to_string()),
        _ => None,
    }
}

fn corps_entree_c8(contenu: &str) -> EntreeBody {
    EntreeBody {
        titre: Some("Séance".into()),
        contenu: contenu.into(),
        notions: vec![],
        publication: "maintenant".into(),
        publie_a: None,
        publie_jour: None,
        publie_heure: None,
        sans_avis: false,
        motif: None,
    }
}

/// Clôture (direction) puis, au besoin, réouverture à double validation.
async fn cloturer(fx: &Fx, annee: Uuid) {
    fx.ent
        .cdt_c8_cloturer(
            &fx.direction(),
            ClotureBody {
                annee_id: annee,
                confirmer: true,
            },
        )
        .await
        .unwrap();
}

async fn rouvrir_a_deux(fx: &Fx, annee: Uuid) {
    fx.ent
        .cdt_c8_rouvrir(
            &fx.direction(),
            ReouvrirBody {
                annee_id: annee,
                motif: "Réouverture motivée par une erreur de saisie".into(),
            },
        )
        .await
        .unwrap();
    fx.ent
        .cdt_c8_rouvrir_valider(&fx.direction2(), ValiderReouvertureBody { annee_id: annee })
        .await
        .unwrap();
}

#[tokio::test]
async fn c8_annee_figee_en_base_aucun_chemin_n_ecrit_et_seule_la_reouverture_desarme() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap()
        .id;
    let p0 = fx.cast.profs[0].user_id;
    // Contenu publié, travail attendu à la séance s1 (futur), séance d'arrivée s2, et une progression à planifier.
    let s0 = seance_a(&fx, auj - Duration::days(1)).await;
    let e1 = entree(&fx, cours, s0, "Contenu archivé.", "maintenant").await;
    let j1 = ouvre(&fx, auj + Duration::days(3)).await;
    let j2 = ouvre(&fx, j1 + Duration::days(1)).await;
    let s1 = fx.seance("CDT-A", p0, Some(fx.service_a), j1).await;
    let s2 = fx.seance("CDT-A", p0, Some(fx.service_a), j2).await;
    let t = fx
        .ent
        .cdt_travaux_creer(
            &fx.prof(),
            TravailC2Body {
                cours_ids: vec![cours],
                seance_donnee_id: None,
                echeance_seance_id: Some(s1),
                echeance_prochaine_seance: false,
                echeance_jour: None,
                echeance_heure: None,
                nature: "exercices".into(),
                titre: "Travail attendu".into(),
                consigne: "Consigne.".into(),
                duree_estimee_min: Some(30),
                rendu: "aucun".into(),
                retard_accepte: true,
                formats: vec![],
                eleve_ids: vec![],
                ia_autorisee: "non".into(),
                publication: "maintenant".into(),
                publie_a: None,
                publie_jour: None,
                publie_heure: None,
            },
        )
        .await
        .unwrap()
        .travaux[0]
        .id;
    sqlx::query("UPDATE ent_cdt_travail SET visible_a = now() - interval '1 minute' WHERE id = $1 AND visible_a > now()")
        .bind(t)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_edt_seance SET statut = 'annulee' WHERE id = $1")
        .bind(s1)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    let reports = fx.ent.cdt_reports_lister(&fx.prof()).await.unwrap();
    let proposition = reports
        .propositions
        .iter()
        .find(|p| p.objet == "travail")
        .expect("proposition de report du travail")
        .clone();
    let prog = fx
        .ent
        .cdt_c9_progression_creer(
            &fx.prof(),
            ent_types::cahier_c9::ProgressionBody {
                titre: "Progression".into(),
                matiere_id: None,
                niveau_id: None,
                annee_id: None,
                etapes: vec![ent_types::cahier_c9::EtapeBody {
                    id: None,
                    titre: "Étape".into(),
                    notions: "".into(),
                    duree_seances: 1,
                }],
            },
        )
        .await
        .unwrap();

    // Un administrateur technique n'a pas le droit de clôturer (M1) ; la direction le fait.
    assert!(fx
        .ent
        .cdt_c8_cloturer(
            &fx.admin(),
            ClotureBody {
                annee_id: annee,
                confirmer: true
            }
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    cloturer(&fx, annee).await;

    // Chaque chemin d'écriture refuse, avec un message clair (409) quand le code le contrôle…
    assert!(
        fx.ent
            .cdt_entree_retirer(&fx.direction(), e1.id, "retrait après la clôture")
            .await
            .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)),
        "retirer"
    );
    assert!(
        fx.ent
            .cdt_contribuer(&fx.prof(), e1.id, "Contribution après la clôture")
            .await
            .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)),
        "contribuer"
    );
    assert!(
        fx.ent
            .cdt_report_traiter(
                &fx.prof(),
                proposition.id,
                ent_types::cahier_c6::ReportDecisionBody {
                    decision: "accepter".into(),
                    vers_seance_id: None,
                    confirmer_avance: false,
                    motif: None
                },
            )
            .await
            .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)),
        "report C6"
    );
    assert!(
        fx.ent
            .cdt_c9_planifier(
                &fx.prof(),
                prog.resume.id,
                ent_types::cahier_c9::PlanifierBody {
                    cours_id: cours,
                    a_partir_du: None,
                    jusqu_au: None,
                    apercu: false
                },
            )
            .await
            .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)),
        "planification C9"
    );
    assert!(
        fx.ent
            .cdt_entree_enregistrer(
                &fx.prof(),
                cours,
                s0,
                corps_entree_c8("Réécrit après la clôture.")
            )
            .await
            .is_err(),
        "saisie"
    );

    // … et surtout la BASE refuse, quel que soit le chemin (23514) : le code n'est qu'un confort.
    let pool = fx.ent.pool();
    assert_eq!(
        code_sql(sqlx::query("UPDATE ent_cdt_entree SET etat = 'retiree', retire_motif = 'sql', retire_par_user_id = $2 WHERE id = $1").bind(e1.id).bind(p0).execute(pool).await)
            .as_deref(),
        Some("23514"),
        "retirer"
    );
    assert_eq!(
        code_sql(sqlx::query("INSERT INTO ent_cdt_contribution (id, etablissement_id, entree_id, libelle_externe, contenu) VALUES ($1, $2, $3, 'Externe', 'x')")
            .bind(Uuid::new_v4()).bind(etab).bind(e1.id).execute(pool).await)
            .as_deref(),
        Some("23514"),
        "contribuer"
    );
    assert_eq!(
        code_sql(sqlx::query("UPDATE ent_cdt_travail SET echeance_seance_id = $2, version = version + 1 WHERE id = $1").bind(t).bind(s2).execute(pool).await).as_deref(),
        Some("23514"),
        "report C6 (travail)"
    );
    assert_eq!(
        code_sql(
            sqlx::query(
                "UPDATE ent_cdt_entree SET seance_id = $2, version = version + 1 WHERE id = $1"
            )
            .bind(e1.id)
            .bind(s2)
            .execute(pool)
            .await
        )
        .as_deref(),
        Some("23514"),
        "report C6 (entrée)"
    );
    assert_eq!(
        code_sql(sqlx::query("INSERT INTO ent_cdt_entree (id, etablissement_id, cours_id, seance_id, annee_id, contenu, etat, auteur_user_id) VALUES ($1, $2, $3, $4, $5, 'planifié', 'brouillon', $6)")
            .bind(Uuid::new_v4()).bind(etab).bind(cours).bind(s2).bind(annee).bind(p0).execute(pool).await)
            .as_deref(),
        Some("23514"),
        "planification C9"
    );
    assert_eq!(
        code_sql(sqlx::query("INSERT INTO ent_cdt_entree_version (id, etablissement_id, entree_id, version, contenu, etat) VALUES ($1, $2, $3, 99, 'x', 'publiee')")
            .bind(Uuid::new_v4()).bind(etab).bind(e1.id).execute(pool).await)
            .as_deref(),
        Some("23514"),
        "version d'entrée"
    );
    let (etat,): (String,) = sqlx::query_as("SELECT etat FROM ent_cdt_entree WHERE id = $1")
        .bind(e1.id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(etat, "publiee", "aucune écriture n'a abouti");
    // Les lectures, elles, continuent : l'annulation d'une séance ne casse pas la projection des publications échues.
    assert!(fx.ent.cdt_reports_lister(&fx.prof()).await.is_ok());

    // Seule la réouverture (tracée, à deux) désarme la garde.
    rouvrir_a_deux(&fx, annee).await;
    fx.ent
        .cdt_entree_retirer(&fx.direction(), e1.id, "retrait après la réouverture")
        .await
        .unwrap();
}

#[tokio::test]
async fn c8_mon_export_repete_n_ecrit_rien_un_seul_export_par_empreinte_et_taille_plafonnee() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap()
        .id;
    let s = seance_a(&fx, auj).await;
    entree(&fx, cours, s, "Ma séance.", "maintenant").await;
    let compte = || async {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ent_cdt_export WHERE etablissement_id = $1 AND nature = 'professeur'")
            .bind(etab)
            .fetch_one(fx.ent.pool())
            .await
            .unwrap()
    };
    // GET répété : aucune ligne.
    for _ in 0..3 {
        fx.ent
            .cdt_c8_export_professeur(&fx.prof(), annee)
            .await
            .unwrap();
    }
    assert_eq!(compte().await, 0);
    // POST répété : un seul export par (auteur, année, empreinte), le même à chaque fois.
    let a = fx
        .ent
        .cdt_c8_export_professeur_conserver(&fx.prof(), annee)
        .await
        .unwrap();
    let b = fx
        .ent
        .cdt_c8_export_professeur_conserver(&fx.prof(), annee)
        .await
        .unwrap();
    assert_eq!(a.export.id, b.export.id);
    assert_eq!(compte().await, 1);
    // Un cahier modifié change l'empreinte : nouvel export, l'ancien est gardé.
    entree(
        &fx,
        cours,
        seance_a(&fx, auj - Duration::days(1)).await,
        "Une autre séance.",
        "maintenant",
    )
    .await;
    let c = fx
        .ent
        .cdt_c8_export_professeur_conserver(&fx.prof(), annee)
        .await
        .unwrap();
    assert_ne!(c.export.id, a.export.id);
    assert_ne!(c.export.empreinte, a.export.empreinte);
    assert_eq!(compte().await, 2);
    // Un autre professeur a son propre export (même année, autre auteur) et ne voit pas celui-là.
    let autre = fx
        .ent
        .cdt_c8_export_professeur_conserver(&fx.prof2(), annee)
        .await
        .unwrap();
    assert_ne!(autre.export.id, a.export.id);
    // Taille plafonnée AVANT l'insertion : 422 (invalide), jamais une erreur serveur.
    let mut tx = TenantTx::begin(fx.ent.pool(), etab).await.unwrap();
    let trop = "x".repeat(EXPORT_TAILLE_MAX + 1);
    let r = crate::cahier_c8_acces::inserer_export(
        &mut tx,
        etab,
        "professeur",
        None,
        None,
        Some(annee),
        (auj, auj),
        "Trop gros",
        None,
        fx.cast.profs[0].user_id,
        (trop, 0, 0),
    )
    .await;
    assert!(r.is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
}

#[tokio::test]
async fn c8_export_litige_par_un_professeur_est_refuse_et_l_octroyeur_ne_s_octroie_rien() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    // T3 : un professeur ne produit pas d'export pour litige (ni aucun autre export de direction).
    assert!(fx
        .ent
        .cdt_c8_export_produire(
            &fx.prof(),
            ExportBody {
                nature: "litige".into(),
                cours_id: Some(cours),
                agent_id: None,
                du: auj - Duration::days(10),
                au: auj,
                motif: Some("Contestation d'une note".into()),
            }
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    // M1 : un octroyeur ne s'accorde pas un accès d'inspection ; une seconde personne le fait, et lui seul ne le prolonge pas.
    let moi = fx.cast.direction.user_id;
    assert!(fx
        .ent
        .cdt_c8_acces_accorder(
            &fx.direction(),
            acces(moi, cours, auj - Duration::days(7), auj)
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_cdt_acces_inspection WHERE beneficiaire_user_id = $1",
    )
    .bind(moi)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 0);
    // La base refuse aussi, quel que soit le chemin.
    assert_eq!(
        code_sql(sqlx::query("INSERT INTO ent_cdt_acces_inspection (id, etablissement_id, beneficiaire_user_id, du_cahier, au_cahier, valable_du, valable_au, motif, accorde_par_user_id)
                              VALUES ($1, $2, $3, $4, $4, now(), now() + interval '5 days', 'inspection', $3)")
            .bind(Uuid::new_v4()).bind(fx.cast.etablissement_id).bind(moi).bind(auj).execute(fx.ent.pool()).await)
            .as_deref(),
        Some("23514")
    );
    let a = fx
        .ent
        .cdt_c8_acces_accorder(
            &fx.direction2(),
            acces(moi, cours, auj - Duration::days(7), auj),
        )
        .await
        .unwrap();
    assert!(fx
        .ent
        .cdt_c8_acces_prolonger(&fx.direction(), a.id, ProlongerBody { jours: 5 })
        .await
        .is_err_and(|e| est_interdit(&e)));
    fx.ent
        .cdt_c8_acces_prolonger(&fx.direction2(), a.id, ProlongerBody { jours: 5 })
        .await
        .unwrap();
}

#[tokio::test]
async fn c8_inspecteur_devenu_eleve_ou_parti_ne_lit_plus() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let s = seance_a(&fx, auj - Duration::days(1)).await;
    entree(&fx, cours, s, "Entrée publiée.", "maintenant").await;
    let insp = &fx.cast.compta;
    let a_insp = act(etab, insp);
    let acc = fx
        .ent
        .cdt_c8_acces_accorder(
            &fx.direction(),
            acces(insp.user_id, cours, auj - Duration::days(7), auj),
        )
        .await
        .unwrap();
    assert!(fx
        .ent
        .cdt_c8_inspection_lire(&a_insp, acc.id, cours)
        .await
        .is_ok());
    // F6 : son rôle change (il devient responsable) : la lecture est refusée à la lecture suivante, sans attendre l'expiration.
    sqlx::query(
        "UPDATE ent_membership SET role = 'parent' WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(insp.user_id)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    assert!(fx
        .ent
        .cdt_c8_inspection_lire(&a_insp, acc.id, cours)
        .await
        .is_err_and(|e| est_interdit(&e)));
    // Il quitte l'établissement : plus rien non plus.
    sqlx::query("DELETE FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2")
        .bind(etab)
        .bind(insp.user_id)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    assert!(fx
        .ent
        .cdt_c8_inspection_lire(&a_insp, acc.id, cours)
        .await
        .is_err_and(|e| est_interdit(&e)));
}

#[tokio::test]
async fn c8_ajoute_depuis_le_visa_est_visible_du_viseur() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let s1 = seance_a(&fx, auj - Duration::days(3)).await;
    let s2 = seance_a(&fx, auj - Duration::days(2)).await;
    let e1 = entree(&fx, cours, s1, "Vue au visa.", "maintenant").await;
    let visa = fx
        .ent
        .cdt_c8_viser(
            &fx.direction(),
            visa_cours(cours, auj - Duration::days(5), auj, None),
        )
        .await
        .unwrap();
    // Une entrée publiée APRÈS le visa, sur une séance de la période visée : ajoutée depuis (M7).
    let e2 = entree(&fx, cours, s2, "Ajoutée après le visa.", "maintenant").await;
    let d = fx
        .ent
        .cdt_c8_visa_detail(&fx.direction(), visa.id)
        .await
        .unwrap();
    assert_eq!(d.elements.len(), 1);
    assert_eq!(d.ajoutes_depuis_visa.len(), 1);
    assert_eq!(d.ajoutes_depuis_visa[0].objet_id, e2.id);
    assert_eq!(d.visa.nb_elements, 1, "ce qui est visé reste figé");
    let ch = fx
        .ent
        .cdt_c8_chronologie(&fx.direction(), cours, auj - Duration::days(5), auj)
        .await
        .unwrap();
    assert!(
        ch.items
            .iter()
            .find(|i| i.objet_id == e2.id)
            .unwrap()
            .ajoute_depuis_visa
    );
    assert!(
        !ch.items
            .iter()
            .find(|i| i.objet_id == e1.id)
            .unwrap()
            .ajoute_depuis_visa
    );
    let tenue = fx
        .ent
        .cdt_c8_tenue(&fx.direction(), auj - Duration::days(5), auj)
        .await
        .unwrap();
    assert_eq!(
        tenue
            .lignes
            .iter()
            .find(|l| l.cours_id == cours)
            .unwrap()
            .ajoutes_apres_visa,
        1
    );
    // Un nouveau visa couvre l'ajout : plus rien n'est « ajouté depuis » (le dernier visa du cours fait foi).
    fx.ent
        .cdt_c8_viser(
            &fx.direction(),
            visa_cours(cours, auj - Duration::days(5), auj, None),
        )
        .await
        .unwrap();
    let tenue = fx
        .ent
        .cdt_c8_tenue(&fx.direction(), auj - Duration::days(5), auj)
        .await
        .unwrap();
    assert_eq!(
        tenue
            .lignes
            .iter()
            .find(|l| l.cours_id == cours)
            .unwrap()
            .ajoutes_apres_visa,
        0
    );
}

// ───────────────────────── correctifs de revue, passe 2 (D-069) ─────────────────────────

/// Attente CONDITIONNELLE, jamais un délai fixe pris pour une preuve : sonde `pg_stat_activity` jusqu'à ce qu'une requête dont
/// le texte contient `motif` soit bloquée sur un verrou (l'observation du blocage est la preuve). Échoue après 30 s.
async fn attendre_verrou(pool: &sqlx::PgPool, motif: &str) {
    let motif = format!("%{motif}%");
    let debut = std::time::Instant::now();
    loop {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM pg_stat_activity
              WHERE datname = current_database() AND wait_event_type = 'Lock' AND query ILIKE $1 AND pid <> pg_backend_pid()",
        )
        .bind(&motif)
        .fetch_one(pool)
        .await
        .unwrap();
        if n > 0 {
            return;
        }
        assert!(
            debut.elapsed() < std::time::Duration::from_secs(30),
            "aucune requête bloquée sur un verrou : {motif}"
        );
        // Cadence de la sonde seulement (la condition observée fait foi, pas ce délai).
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

/// Une ligne par table fille du cahier : (table, INSERT valide, UPDATE neutre, DELETE) — ids interpolés (UUID, sans risque).
struct JeuGel {
    annee: Uuid,
    lignes: Vec<(&'static str, String, String, String)>,
}

async fn jeu_gel(fx: &Fx) -> JeuGel {
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(fx).await;
    let auj = jour(fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap()
        .id;
    let p0 = fx.cast.profs[0].user_id;
    let eleve = fx.cast.eleves[0].id;
    let s0 = seance_a(fx, auj - Duration::days(1)).await;
    let s1 = seance_a(fx, auj - Duration::days(2)).await;
    let e1 = entree(fx, cours, s0, "Contenu du jeu.", "maintenant")
        .await
        .id;
    let t1 = travail(fx, cours, "Travail du jeu", "maintenant").await;
    let pool = fx.ent.pool();
    let (contrib, contrib_v, variante, lien, objet, pj) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    for q in [
        format!("INSERT INTO ent_cdt_contribution (id, etablissement_id, entree_id, libelle_externe, contenu) VALUES ('{contrib}', '{etab}', '{e1}', 'Externe', 'x')"),
        format!("INSERT INTO ent_cdt_contribution_version (id, etablissement_id, contribution_id, version, contenu) VALUES ('{contrib_v}', '{etab}', '{contrib}', 1, 'x')"),
        format!("INSERT INTO ent_cdt_travail_cible (etablissement_id, travail_id, eleve_id) VALUES ('{etab}', '{t1}', '{eleve}')"),
        format!("INSERT INTO ent_cdt_travail_format (etablissement_id, travail_id, format_code) VALUES ('{etab}', '{t1}', 'pdf')"),
        format!("INSERT INTO ent_cdt_variante (id, etablissement_id, travail_id, libelle_interne, consigne) VALUES ('{variante}', '{etab}', '{t1}', 'V', 'c')"),
        format!("INSERT INTO ent_cdt_variante_eleve (etablissement_id, variante_id, eleve_id) VALUES ('{etab}', '{variante}', '{eleve}')"),
        format!("INSERT INTO ent_cdt_lien (id, etablissement_id, porteur, porteur_id, url) VALUES ('{lien}', '{etab}', 'entree', '{e1}', 'https://exemple.fr/a')"),
        format!("INSERT INTO ent_objet (id, etablissement_id, cle_objet, taille, sha256) VALUES ('{objet}', '{etab}', '{etab}/cdt/{objet}', 1, '{}')", "0".repeat(64)),
        format!("INSERT INTO ent_cdt_piece_jointe (id, etablissement_id, objet_id, porteur, porteur_id, nom) VALUES ('{pj}', '{etab}', '{objet}', 'entree', '{e1}', 'a.pdf')"),
        format!("INSERT INTO ent_cdt_report (id, etablissement_id, origine_seance_id, objet, objet_id) VALUES ('{}', '{etab}', '{s0}', 'entree', '{e1}')", Uuid::new_v4()),
    ] {
        sqlx::query(&q).execute(pool).await.unwrap_or_else(|e| panic!("jeu : {q} : {e}"));
    }
    let nouvelle_seance = s1;
    let mut lignes: Vec<(&'static str, String, String, String)> = Vec::new();
    let mut ajoute =
        |t: &'static str, ins: String, upd: String, del: String| lignes.push((t, ins, upd, del));
    ajoute(
        "ent_cdt_entree",
        format!("INSERT INTO ent_cdt_entree (id, etablissement_id, cours_id, seance_id, annee_id, contenu, etat, auteur_user_id) VALUES ('{}', '{etab}', '{cours}', '{nouvelle_seance}', '{annee}', 'x', 'brouillon', '{p0}')", Uuid::new_v4()),
        format!("UPDATE ent_cdt_entree SET modifie_le = modifie_le WHERE id = '{e1}'"),
        format!("DELETE FROM ent_cdt_entree WHERE id = '{e1}'"),
    );
    ajoute(
        "ent_cdt_entree_version",
        format!("INSERT INTO ent_cdt_entree_version (id, etablissement_id, entree_id, version, contenu, etat) VALUES ('{}', '{etab}', '{e1}', 900, 'x', 'publiee')", Uuid::new_v4()),
        format!("UPDATE ent_cdt_entree_version SET cree_le = cree_le WHERE entree_id = '{e1}'"),
        format!("DELETE FROM ent_cdt_entree_version WHERE entree_id = '{e1}'"),
    );
    ajoute(
        "ent_cdt_entree_notion",
        format!("INSERT INTO ent_cdt_entree_notion (etablissement_id, entree_id, ordre, libelle) VALUES ('{etab}', '{e1}', 900, 'N')"),
        format!("UPDATE ent_cdt_entree_notion SET libelle = libelle WHERE entree_id = '{e1}'"),
        format!("DELETE FROM ent_cdt_entree_notion WHERE entree_id = '{e1}'"),
    );
    ajoute(
        "ent_cdt_travail",
        format!("INSERT INTO ent_cdt_travail (id, etablissement_id, cours_id, annee_id, donne_le, echeance_mode, echeance_le, nature_id, titre, consigne, auteur_user_id)
                 SELECT '{}', etablissement_id, cours_id, annee_id, donne_le, echeance_mode, echeance_le, nature_id, 'Copie', consigne, auteur_user_id FROM ent_cdt_travail WHERE id = '{t1}'", Uuid::new_v4()),
        format!("UPDATE ent_cdt_travail SET modifie_le = modifie_le WHERE id = '{t1}'"),
        format!("DELETE FROM ent_cdt_travail WHERE id = '{t1}'"),
    );
    ajoute(
        "ent_cdt_travail_version",
        format!("INSERT INTO ent_cdt_travail_version (id, etablissement_id, travail_id, version, titre, consigne, etat) VALUES ('{}', '{etab}', '{t1}', 900, 't', 'c', 'publie')", Uuid::new_v4()),
        format!("UPDATE ent_cdt_travail_version SET cree_le = cree_le WHERE travail_id = '{t1}'"),
        format!("DELETE FROM ent_cdt_travail_version WHERE travail_id = '{t1}'"),
    );
    ajoute(
        "ent_cdt_travail_cible",
        format!("INSERT INTO ent_cdt_travail_cible (etablissement_id, travail_id, eleve_id) VALUES ('{etab}', '{t1}', '{}')", fx.cast.eleves[1].id),
        format!("UPDATE ent_cdt_travail_cible SET cree_le = cree_le WHERE travail_id = '{t1}'"),
        format!("DELETE FROM ent_cdt_travail_cible WHERE travail_id = '{t1}'"),
    );
    ajoute(
        "ent_cdt_travail_format",
        format!("INSERT INTO ent_cdt_travail_format (etablissement_id, travail_id, format_code) VALUES ('{etab}', '{t1}', 'docx')"),
        format!("UPDATE ent_cdt_travail_format SET cree_le = cree_le WHERE travail_id = '{t1}'"),
        format!("DELETE FROM ent_cdt_travail_format WHERE travail_id = '{t1}'"),
    );
    ajoute(
        "ent_cdt_variante",
        format!("INSERT INTO ent_cdt_variante (id, etablissement_id, travail_id, libelle_interne, consigne) VALUES ('{}', '{etab}', '{t1}', 'W', 'c')", Uuid::new_v4()),
        format!("UPDATE ent_cdt_variante SET cree_le = cree_le WHERE id = '{variante}'"),
        format!("DELETE FROM ent_cdt_variante WHERE id = '{variante}'"),
    );
    ajoute(
        "ent_cdt_variante_eleve",
        format!("INSERT INTO ent_cdt_variante_eleve (etablissement_id, variante_id, eleve_id) VALUES ('{etab}', '{variante}', '{}')", fx.cast.eleves[1].id),
        format!("UPDATE ent_cdt_variante_eleve SET cree_le = cree_le WHERE variante_id = '{variante}'"),
        format!("DELETE FROM ent_cdt_variante_eleve WHERE variante_id = '{variante}'"),
    );
    ajoute(
        "ent_cdt_contribution",
        format!("INSERT INTO ent_cdt_contribution (id, etablissement_id, entree_id, libelle_externe, contenu) VALUES ('{}', '{etab}', '{e1}', 'Autre', 'y')", Uuid::new_v4()),
        format!("UPDATE ent_cdt_contribution SET cree_le = cree_le WHERE id = '{contrib}'"),
        format!("DELETE FROM ent_cdt_contribution WHERE id = '{contrib}'"),
    );
    ajoute(
        "ent_cdt_contribution_version",
        format!("INSERT INTO ent_cdt_contribution_version (id, etablissement_id, contribution_id, version, contenu) VALUES ('{}', '{etab}', '{contrib}', 900, 'y')", Uuid::new_v4()),
        format!("UPDATE ent_cdt_contribution_version SET cree_le = cree_le WHERE contribution_id = '{contrib}'"),
        format!("DELETE FROM ent_cdt_contribution_version WHERE contribution_id = '{contrib}'"),
    );
    ajoute(
        "ent_cdt_lien",
        format!("INSERT INTO ent_cdt_lien (id, etablissement_id, porteur, porteur_id, url) VALUES ('{}', '{etab}', 'travail', '{t1}', 'https://exemple.fr/b')", Uuid::new_v4()),
        format!("UPDATE ent_cdt_lien SET cree_le = cree_le WHERE id = '{lien}'"),
        format!("DELETE FROM ent_cdt_lien WHERE id = '{lien}'"),
    );
    ajoute(
        "ent_cdt_piece_jointe",
        format!("INSERT INTO ent_cdt_piece_jointe (id, etablissement_id, objet_id, porteur, porteur_id, nom) VALUES ('{}', '{etab}', '{objet}', 'travail', '{t1}', 'b.pdf')", Uuid::new_v4()),
        format!("UPDATE ent_cdt_piece_jointe SET cree_le = cree_le WHERE id = '{pj}'"),
        format!("DELETE FROM ent_cdt_piece_jointe WHERE id = '{pj}'"),
    );
    ajoute(
        "ent_cdt_report",
        format!("INSERT INTO ent_cdt_report (id, etablissement_id, origine_seance_id, objet, objet_id) VALUES ('{}', '{etab}', '{s0}', 'travail', '{t1}')", Uuid::new_v4()),
        format!("UPDATE ent_cdt_report SET cree_le = cree_le WHERE objet_id = '{e1}'"),
        format!("DELETE FROM ent_cdt_report WHERE objet_id = '{e1}'"),
    );
    JeuGel { annee, lignes }
}

/// Tables de versions : en ajout seul (UPDATE et DELETE y sont refusés par leur propre garde, avec le même code 23514).
fn ajout_seul(table: &str) -> bool {
    table.ends_with("_version") || table == "ent_cdt_continuite_cours"
}

/// Avant le gel, l'INSERT passe sur chaque table (le jeu est valide : un refus plus tard vient bien du gel) et l'UPDATE touche
/// au moins une ligne (hors tables en ajout seul).
async fn inserer_avant_gel(fx: &Fx, jeu: &JeuGel) {
    for (table, ins, upd, _) in &jeu.lignes {
        sqlx::query(ins)
            .execute(fx.ent.pool())
            .await
            .unwrap_or_else(|e| panic!("avant gel : INSERT {table} : {e}"));
        if !ajout_seul(table) {
            let n = sqlx::query(upd)
                .execute(fx.ent.pool())
                .await
                .unwrap()
                .rows_affected();
            assert!(
                n > 0,
                "avant gel : UPDATE {table} touche au moins une ligne"
            );
        }
    }
}

/// Chaque table, chaque verbe (INSERT, UPDATE, DELETE) : refusé par la base (23514), sans exception ; rien n'a changé.
async fn verifier_gel(fx: &Fx, jeu: &JeuGel, contexte: &str) {
    let pool = fx.ent.pool();
    for (table, ins, upd, del) in &jeu.lignes {
        let (avant,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM {table} WHERE etablissement_id = $1"
        ))
        .bind(fx.cast.etablissement_id)
        .fetch_one(pool)
        .await
        .unwrap();
        for (verbe, q) in [("INSERT", ins), ("UPDATE", upd), ("DELETE", del)] {
            assert_eq!(
                code_sql(sqlx::query(q).execute(pool).await).as_deref(),
                Some("23514"),
                "{contexte} : {verbe} sur {table} doit être refusé par la base"
            );
        }
        let (apres,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM {table} WHERE etablissement_id = $1"
        ))
        .bind(fx.cast.etablissement_id)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(avant, apres, "{contexte} : {table} inchangée");
    }
}

#[tokio::test]
async fn c8_gel_en_base_de_toutes_les_tables_du_cahier_apres_cloture_du_cahier() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let jeu = jeu_gel(&fx).await;
    inserer_avant_gel(&fx, &jeu).await;
    cloturer(&fx, jeu.annee).await;
    verifier_gel(&fx, &jeu, "cahier clôturé").await;
    // Seule la réouverture tracée désarme le gel : le même jeu repasse.
    rouvrir_a_deux(&fx, jeu.annee).await;
    for (table, _, upd, _) in jeu.lignes.iter().filter(|l| !ajout_seul(l.0)) {
        let n = sqlx::query(upd)
            .execute(fx.ent.pool())
            .await
            .unwrap()
            .rows_affected();
        assert!(n > 0, "après réouverture : UPDATE {table}");
    }
}

#[tokio::test]
async fn c8_gel_en_base_d_une_annee_close_au_socle_5_sans_cloture_du_cahier() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let jeu = jeu_gel(&fx).await;
    inserer_avant_gel(&fx, &jeu).await;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    // Une publication programmée, due, dans l'année : elle ne doit pas se publier dans une année close au socle 5.
    let s = seance_a(&fx, auj - Duration::days(3)).await;
    let prog = fx
        .ent
        .cdt_entree_enregistrer(
            &fx.prof(),
            cours,
            s,
            EntreeBody {
                titre: None,
                contenu: "Programmée.".into(),
                notions: vec![],
                publication: "programmer".into(),
                publie_a: None,
                publie_jour: Some(auj + Duration::days(2)),
                publie_heure: NaiveTime::from_hms_opt(8, 0, 0),
                sans_avis: false,
                motif: None,
            },
        )
        .await
        .unwrap()
        .id;
    sqlx::query("UPDATE ent_cdt_entree SET visible_a = now() - interval '2 days' WHERE id = $1")
        .bind(prog)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE ent_annee_scolaire SET cloturee_le = now() WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(jeu.annee)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    verifier_gel(&fx, &jeu, "année close au socle 5").await;
    // La publication programmée n'aboutit pas (aucune erreur, aucune écriture) : l'entrée reste programmée.
    fx.ent.cdt_projeter_echus(etab).await.unwrap();
    let (etat,): (String,) = sqlx::query_as("SELECT etat FROM ent_cdt_entree WHERE id = $1")
        .bind(prog)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(etat, "programmee");
}

#[tokio::test]
async fn c8_journal_de_cloture_coherent_par_construction_demande_expiree_et_admin_sans_les_trois_droits(
) {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap()
        .id;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj - Duration::days(1)).await,
        "Contenu.",
        "maintenant",
    )
    .await;
    let pool = fx.ent.pool();
    let d1 = fx.cast.direction.user_id;
    let insere = |action: &'static str,
                  par: Uuid,
                  demandeur: Option<Uuid>,
                  motif: Option<&'static str>| {
        let q = sqlx::query("INSERT INTO ent_cdt_cloture (id, etablissement_id, annee_id, action, motif, par_user_id, demandeur_user_id) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(annee)
            .bind(action)
            .bind(motif)
            .bind(par)
            .bind(demandeur);
        async move { code_sql(q.execute(pool).await) }
    };
    let motif = Some("Motif suffisamment long pour la demande");
    // Cahier non clôturé : ni demande ni réouverture.
    assert_eq!(
        insere("reouverture_demandee", d1, None, motif)
            .await
            .as_deref(),
        Some("23514")
    );
    assert_eq!(
        insere(
            "reouverture",
            fx.cast.directions[1].user_id,
            Some(d1),
            motif
        )
        .await
        .as_deref(),
        Some("23514")
    );
    cloturer(&fx, annee).await;
    // Clôturé : pas de seconde clôture, et une `reouverture` en SQL direct sans demande est refusée par la base.
    assert_eq!(
        insere("cloture", d1, None, None).await.as_deref(),
        Some("23514"),
        "double clôture"
    );
    assert_eq!(
        insere(
            "reouverture",
            fx.cast.directions[1].user_id,
            Some(d1),
            motif
        )
        .await
        .as_deref(),
        Some("23514"),
        "réouverture sans demande"
    );
    // Demande par d1 : la valider soi-même est refusé (code et base) ; une autre personne la valide.
    fx.ent
        .cdt_c8_rouvrir(
            &fx.direction(),
            ReouvrirBody {
                annee_id: annee,
                motif: "Motif suffisamment long pour la demande".into(),
            },
        )
        .await
        .unwrap();
    assert!(fx
        .ent
        .cdt_c8_rouvrir_valider(&fx.direction(), ValiderReouvertureBody { annee_id: annee })
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert_eq!(
        insere("reouverture", d1, Some(d1), motif).await.as_deref(),
        Some("23514"),
        "auto-validation en SQL"
    );
    assert_eq!(
        insere(
            "reouverture",
            fx.cast.directions[1].user_id,
            Some(d1),
            Some("Un autre motif, suffisamment long")
        )
        .await
        .as_deref(),
        Some("23514"),
        "la réouverture reprend le motif de la demande"
    );
    // Demande expirée (délai de l'établissement : 14 jours par défaut) : rétrodatée sous `replica` pour ce seul test.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_cdt_cloture SET cree_le = cree_le - interval '15 days' WHERE etablissement_id = $1 AND annee_id = $2 AND action = 'reouverture_demandee'")
        .bind(etab)
        .bind(annee)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        insere(
            "reouverture",
            fx.cast.directions[1].user_id,
            Some(d1),
            motif
        )
        .await
        .as_deref(),
        Some("23514"),
        "demande expirée : la base refuse"
    );
    assert!(fx
        .ent
        .cdt_c8_rouvrir_valider(&fx.direction2(), ValiderReouvertureBody { annee_id: annee })
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    // Une demande expirée ne bloque pas une nouvelle demande ; celle-ci se valide.
    rouvrir_a_deux(&fx, annee).await;
    // Un pouvoir = une action : l'administrateur technique n'a ni clôture, ni export, ni octroi d'accès d'inspection.
    cloturer(&fx, annee).await;
    assert!(fx
        .ent
        .cdt_c8_cloturer(
            &fx.admin(),
            ClotureBody {
                annee_id: annee,
                confirmer: true
            }
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c8_rouvrir(
            &fx.admin(),
            ReouvrirBody {
                annee_id: annee,
                motif: "Motif suffisamment long pour la demande".into()
            }
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c8_export_produire(
            &fx.admin(),
            ExportBody {
                nature: "inspection".into(),
                cours_id: Some(cours),
                agent_id: None,
                du: auj - Duration::days(7),
                au: auj,
                motif: None,
            },
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c8_acces_accorder(
            &fx.admin(),
            acces(fx.cast.compta.user_id, cours, auj - Duration::days(7), auj)
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c8_archives(&fx.admin(), Some(annee))
        .await
        .is_err_and(|e| est_interdit(&e)));
}

#[tokio::test]
async fn c8_cloture_projette_les_publications_echues_et_n_en_perd_aucune() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap()
        .id;
    let s = seance_a(&fx, auj - Duration::days(3)).await;
    let prog = fx
        .ent
        .cdt_entree_enregistrer(
            &fx.prof(),
            cours,
            s,
            EntreeBody {
                titre: None,
                contenu: "Publication programmée, due avant la clôture.".into(),
                notions: vec![],
                publication: "programmer".into(),
                publie_a: None,
                publie_jour: Some(auj + Duration::days(2)),
                publie_heure: NaiveTime::from_hms_opt(8, 0, 0),
                sans_avis: false,
                motif: None,
            },
        )
        .await
        .unwrap()
        .id;
    // Échue : rétrodatée (valeur fixe dans le passé, aucune dépendance à l'heure du test), jamais projetée.
    sqlx::query("UPDATE ent_cdt_entree SET visible_a = now() - interval '2 days' WHERE id = $1")
        .bind(prog)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    cloturer(&fx, annee).await;
    let (etat,): (String,) = sqlx::query_as("SELECT etat FROM ent_cdt_entree WHERE id = $1")
        .bind(prog)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(
        etat, "publiee",
        "la publication due a été projetée avant la clôture"
    );
    let a = fx
        .ent
        .cdt_c8_archives(&fx.direction(), Some(annee))
        .await
        .unwrap();
    let a = a
        .iter()
        .find(|x| x.cours.cours_id == cours)
        .expect("archive du cours");
    let c = fx
        .ent
        .cdt_c8_archive_contenu(&fx.direction(), a.id)
        .await
        .unwrap();
    assert!(
        c.contenu
            .contains("Publication programmée, due avant la clôture."),
        "présente dans l'archive"
    );
}

#[tokio::test]
async fn c8_erreurs_de_concurrence_de_la_base_sont_un_conflit_reessayable_jamais_une_erreur_serveur(
) {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    for code in ["40001", "40P01"] {
        let err = sqlx::query(&format!(
            "DO $$ BEGIN RAISE EXCEPTION 'x' USING ERRCODE = '{code}'; END $$"
        ))
        .execute(fx.ent.pool())
        .await
        .unwrap_err();
        let e = crate::cahier_c8::db(err);
        assert!(est_refus(&e, RefusMetier::Conflit), "{code} : {e:?}");
    }
    // Le refus d'une garde de base (année figée) est lui aussi un conflit (409), pas un 400.
    let err = sqlx::query("DO $$ BEGIN RAISE EXCEPTION 'cahier de textes : l''année est figée (clôturée)' USING ERRCODE = '23514'; END $$")
        .execute(fx.ent.pool())
        .await
        .unwrap_err();
    assert!(est_refus(&crate::cahier_c8::db(err), RefusMetier::Conflit));
}

#[tokio::test]
async fn c8_cloture_et_ecritures_se_serialisent_par_verrou_dans_les_deux_sens_sans_delai() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap()
        .id;
    let s = seance_a(&fx, auj - Duration::days(1)).await;
    let s_nouvelle = seance_a(&fx, auj - Duration::days(2)).await;
    let e1 = entree(&fx, cours, s, "Contenu initial.", "maintenant").await;
    let pool = fx.ent.pool();

    // Sens 1 : une écriture en cours (non validée) retient la clôture ; on observe la clôture bloquée sur le verrou de l'année,
    // PUIS on valide l'écriture : la clôture la voit et l'archive.
    let mut ecrivain = pool.begin().await.unwrap();
    sqlx::query(
        "UPDATE ent_cdt_entree SET contenu = 'Contenu écrit pendant la clôture.' WHERE id = $1",
    )
    .bind(e1.id)
    .execute(&mut *ecrivain)
    .await
    .unwrap();
    let dir = fx.direction();
    let (clot, comm) = tokio::join!(
        fx.ent.cdt_c8_cloturer(
            &dir,
            ClotureBody {
                annee_id: annee,
                confirmer: true
            }
        ),
        async {
            attendre_verrou(pool, "FROM ent_annee_scolaire").await;
            ecrivain.commit().await
        }
    );
    comm.unwrap();
    clot.unwrap();
    let archives = fx
        .ent
        .cdt_c8_archives(&fx.direction(), Some(annee))
        .await
        .unwrap();
    let a = archives
        .iter()
        .find(|x| x.cours.cours_id == cours)
        .expect("archive du cours");
    let contenu = fx
        .ent
        .cdt_c8_archive_contenu(&fx.direction(), a.id)
        .await
        .unwrap();
    assert!(
        contenu
            .contenu
            .contains("Contenu écrit pendant la clôture."),
        "l'archive contient l'écriture concurrente"
    );
    rouvrir_a_deux(&fx, annee).await;

    // Sens 2 : la clôture RÉELLE (`cdt_c8_cloturer`) est retenue après avoir pris le verrou de l'année (une archive fantôme,
    // non validée, occupe la version 1 de l'archive du cours : l'insertion de la clôture attend). Une écriture RÉELLE démarre
    // alors, puis attend à son tour. On observe les deux blocages, puis on lève l'obstacle : la clôture aboutit, l'écriture
    // est refusée avec le conflit exact « année close », sans aucune trace.
    let mut obstacle = pool.begin().await.unwrap();
    let fantome = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_cdt_export (id, etablissement_id, nature, cours_id, annee_id, du, au, titre, contenu, empreinte, nb_entrees, nb_travaux, produit_par_user_id)
                 VALUES ($1, $2, 'archive', $3, $4, $5, $5, 'fantôme', 'x', encode(sha256(convert_to('x', 'UTF8')), 'hex'), 0, 0, $6)")
        .bind(fantome)
        .bind(etab)
        .bind(cours)
        .bind(annee)
        .bind(auj)
        .bind(fx.cast.direction.user_id)
        .execute(&mut *obstacle)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_cdt_archive (id, etablissement_id, annee_id, cours_id, export_id, empreinte, produite_le, version, echeance_archive)
                 SELECT $1, etablissement_id, annee_id, cours_id, id, empreinte, now(),
                        COALESCE((SELECT max(version) FROM ent_cdt_archive WHERE etablissement_id = $4 AND annee_id = $5 AND cours_id = $6), 0) + 1, $3
                   FROM ent_cdt_export WHERE id = $2")
        .bind(Uuid::new_v4())
        .bind(fantome)
        .bind(auj + Duration::days(900))
        .bind(etab)
        .bind(annee)
        .bind(cours)
        .execute(&mut *obstacle)
        .await
        .unwrap();
    let prof = fx.prof();
    let dir = fx.direction();
    let (clot, ecriture, ()) = tokio::join!(
        fx.ent.cdt_c8_cloturer(
            &dir,
            ClotureBody {
                annee_id: annee,
                confirmer: true
            }
        ),
        async {
            // Démarre quand la clôture réelle est bloquée (elle détient donc le verrou de l'année).
            attendre_verrou(pool, "INSERT INTO ent_cdt_archive").await;
            fx.ent
                .cdt_entree_enregistrer(
                    &prof,
                    cours,
                    s_nouvelle,
                    corps_entree_c8("Écrit pendant la clôture, perdu."),
                )
                .await
        },
        async {
            attendre_verrou(pool, "INSERT INTO ent_cdt_archive").await;
            attendre_verrou(pool, "ent_cdt_entree").await;
            obstacle.rollback().await.unwrap();
        }
    );
    clot.unwrap();
    assert!(
        ecriture.is_err_and(|e| est_refus(&e, RefusMetier::Conflit)),
        "l'écriture qui croise la clôture échoue avec le conflit « année close »"
    );
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_cdt_entree WHERE seance_id = $1")
        .bind(s_nouvelle)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "la tentative n'a laissé aucune trace");
    let (c,): (String,) = sqlx::query_as("SELECT contenu FROM ent_cdt_entree WHERE id = $1")
        .bind(e1.id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(c, "Contenu écrit pendant la clôture.");
}

#[tokio::test]
async fn c8_deux_post_simultanes_de_mon_export_conservent_un_seul_export() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap()
        .id;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj - Duration::days(1)).await,
        "Contenu du professeur.",
        "maintenant",
    )
    .await;
    let (a, b) = (fx.prof(), fx.prof());
    let (r1, r2) = tokio::join!(
        fx.ent.cdt_c8_export_professeur_conserver(&a, annee),
        fx.ent.cdt_c8_export_professeur_conserver(&b, annee)
    );
    let (r1, r2) = (r1.unwrap(), r2.unwrap());
    assert_eq!(
        r1.export.id, r2.export.id,
        "le même export est renvoyé aux deux appels"
    );
    assert_eq!(r1.export.empreinte, r2.export.empreinte);
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_cdt_export WHERE etablissement_id = $1 AND nature = 'professeur'",
    )
    .bind(etab)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "un seul export conservé");
}

#[tokio::test]
async fn c8_cours_trop_gros_pour_une_archive_est_archive_par_morceaux_sans_bloquer_la_cloture() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap();
    let t0 = travail(&fx, cours, "Modèle", "maintenant").await;
    // 260 travaux publiés de 20 000 caractères sur 260 jours : plus que le plafond d'une archive (4 000 000 caractères).
    sqlx::query(
        "INSERT INTO ent_cdt_travail (id, etablissement_id, cours_id, annee_id, donne_le, echeance_mode, echeance_le, nature_id,
                titre, consigne, etat, publie_a, visible_a, auteur_user_id)
         SELECT gen_random_uuid(), t.etablissement_id, t.cours_id, t.annee_id, $2::date + g, t.echeance_mode, t.echeance_le, t.nature_id,
                'Volumineux ' || g, repeat('x', 20000), 'publie', now(), now() - interval '1 day', t.auteur_user_id
           FROM ent_cdt_travail t, generate_series(1, 260) g WHERE t.id = $1",
    )
    .bind(t0)
    .bind(annee.date_debut)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    let r = fx
        .ent
        .cdt_c8_cloturer(
            &fx.direction(),
            ClotureBody {
                annee_id: annee.id,
                confirmer: true,
            },
        )
        .await
        .unwrap();
    assert!(
        r.archives_produites >= 2,
        "plusieurs morceaux : {}",
        r.archives_produites
    );
    let (parts, total): (i64, i64) = sqlx::query_as(
        "SELECT count(*), COALESCE(sum(x.nb_travaux), 0)::bigint FROM ent_cdt_archive a
           JOIN ent_cdt_export x ON x.etablissement_id = a.etablissement_id AND x.id = a.export_id
          WHERE a.etablissement_id = $1 AND a.annee_id = $2 AND a.cours_id = $3",
    )
    .bind(etab)
    .bind(annee.id)
    .bind(cours)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap();
    assert!(parts >= 2);
    assert_eq!(
        total, 261,
        "aucun travail perdu : tous sont dans un morceau"
    );
}

#[tokio::test]
async fn c8_inspecteur_les_casquettes_comptent_pour_le_role_a_l_octroi_et_a_chaque_lecture() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj - Duration::days(1)).await,
        "Entrée publiée.",
        "maintenant",
    )
    .await;
    let insp = &fx.cast.compta;
    let a_insp = act(etab, insp);
    let casquette = |etat: &'static str| {
        sqlx::query("INSERT INTO ent_membership_casquette (user_id, etablissement_id, casquette, profil_base, du, etat)
                     VALUES ($1, $2, 'personnel', 'compta', $3, $4)
                     ON CONFLICT (user_id, etablissement_id, casquette) DO UPDATE SET etat = EXCLUDED.etat, du = EXCLUDED.du")
            .bind(insp.user_id)
            .bind(etab)
            .bind(auj - Duration::days(30))
            .bind(etat)
            .execute(fx.ent.pool())
    };
    // Casquette personnelle fermée : le profil principal n'y change rien, l'octroi est refusé.
    casquette("ferme").await.unwrap();
    assert!(fx
        .ent
        .cdt_c8_acces_accorder(
            &fx.direction(),
            acces(insp.user_id, cours, auj - Duration::days(7), auj)
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    casquette("ouvert").await.unwrap();
    let acc = fx
        .ent
        .cdt_c8_acces_accorder(
            &fx.direction(),
            acces(insp.user_id, cours, auj - Duration::days(7), auj),
        )
        .await
        .unwrap();
    assert!(fx
        .ent
        .cdt_c8_inspection_lire(&a_insp, acc.id, cours)
        .await
        .is_ok());
    // Profil principal devenu « parent » mais casquette personnelle ouverte : il reste un personnel, la lecture passe.
    sqlx::query(
        "UPDATE ent_membership SET role = 'parent' WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(insp.user_id)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    assert!(fx
        .ent
        .cdt_c8_inspection_lire(&a_insp, acc.id, cours)
        .await
        .is_ok());
    // Casquette personnelle fermée : plus rien, à la lecture suivante.
    casquette("ferme").await.unwrap();
    assert!(fx
        .ent
        .cdt_c8_inspection_lire(&a_insp, acc.id, cours)
        .await
        .is_err_and(|e| est_interdit(&e)));
}

#[tokio::test]
async fn c8_f4_un_visa_de_classe_ne_compte_pour_un_professeur_que_les_cours_qu_il_voit() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let auj = jour(&fx).await;
    let cours1 = cours_a(&fx).await;
    // Second cours de la même classe, tenu par un collègue (autre matière).
    let p1 = fx.cast.profs[1].user_id;
    let matiere: Uuid =
        sqlx::query_scalar("SELECT id FROM ent_matiere WHERE etablissement_id = $1 LIMIT 1")
            .bind(etab)
            .fetch_one(fx.ent.pool())
            .await
            .unwrap();
    let service2 = fx
        .ent
        .referentiel_creer_service(
            Role::Direction.as_str(),
            etab,
            CreerService {
                prof_user_id: p1,
                matiere_id: Some(matiere),
                classe_id: Some(fx.classe_a),
                groupe_id: None,
                professeur_principal: false,
            },
        )
        .await
        .unwrap()
        .id;
    let cours2: Uuid =
        sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE id = $1")
            .bind(service2)
            .fetch_one(fx.ent.pool())
            .await
            .unwrap();
    assert_ne!(cours1, cours2);
    entree(
        &fx,
        cours1,
        seance_a(&fx, auj - Duration::days(1)).await,
        "Cours 1.",
        "maintenant",
    )
    .await;
    let s2 = fx
        .seance("CDT-A", p1, Some(service2), auj - Duration::days(1))
        .await;
    fx.ent
        .cdt_entree_enregistrer(
            &fx.prof2(),
            cours2,
            s2,
            corps_entree_c8("Cours 2 du collègue."),
        )
        .await
        .unwrap();
    let visa = fx
        .ent
        .cdt_c8_viser(
            &fx.direction(),
            VisaCahierBody {
                cible: VISA_CLASSE.into(),
                cours_id: None,
                classe_id: Some(fx.classe_a),
                agent_id: None,
                du: auj - Duration::days(2),
                au: auj,
                observation: None,
                confirmer_vide: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(visa.nb_elements, 2, "la direction voit les deux cours");
    // Le professeur du cours 1 : seul son cours est compté ; aucun nombre ne trahit celui du collègue.
    let vus = fx.ent.cdt_c8_visas(&fx.prof(), None).await.unwrap();
    let v = vus
        .iter()
        .find(|x| x.id == visa.id)
        .expect("le visa de classe est visible du professeur de la classe");
    assert_eq!(v.nb_elements, 1, "comptage limité aux cours visibles");
    let detail = fx
        .ent
        .cdt_c8_visa_detail(&fx.prof(), visa.id)
        .await
        .unwrap();
    assert_eq!(detail.visa.nb_elements, 1);
}

#[tokio::test]
async fn c8_f11_l_octroi_d_acces_d_inspection_ne_depend_pas_de_cahier_viser() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj - Duration::days(1)).await,
        "Entrée.",
        "maintenant",
    )
    .await;
    let dir = (fx.cast.direction.user_id, Role::Direction.as_str());
    // Rôle libre « Référent inspections » : `cahier.acces_inspection` seul, sans `cahier.viser`.
    let role = fx
        .ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            RoleSaisie {
                nom: format!("Inspections {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::CahierAccesInspection,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    let delegue = &fx.cast.profs[1];
    let a_delegue = act(etab, delegue);
    // Avant l'attribution : rien, ni liste de cours ni octroi.
    assert!(fx
        .ent
        .cdt_c8_cours_ouvrables(&a_delegue)
        .await
        .is_err_and(|e| est_interdit(&e)));
    fx.ent
        .attribuer_role_for_actor(etab, dir.0, dir.1, role.id, delegue.user_id, None)
        .await
        .unwrap();
    assert!(
        !fx.ent
            .cdt_droit_etab(&a_delegue, Action::CahierViser)
            .await
            .unwrap(),
        "il ne vise pas"
    );
    let ouvrables = fx.ent.cdt_c8_cours_ouvrables(&a_delegue).await.unwrap();
    assert!(
        ouvrables.iter().any(|c| c.cours_id == cours),
        "la liste des cours vient du droit d'octroi"
    );
    let acc = fx
        .ent
        .cdt_c8_acces_accorder(
            &a_delegue,
            acces(fx.cast.compta.user_id, cours, auj - Duration::days(7), auj),
        )
        .await
        .unwrap();
    assert_eq!(acc.motif, "inspection");
    // Jamais à soi-même, même avec le droit.
    assert!(fx
        .ent
        .cdt_c8_acces_accorder(
            &a_delegue,
            acces(delegue.user_id, cours, auj - Duration::days(7), auj)
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
}

// ───────────────────────── passe 3 : restes de la passe 2 ─────────────────────────

/// Faible 7 : un SEUL jour au-dessus du plafond d'une archive est tronqué avec une mention explicite, sans bloquer la clôture.
#[tokio::test]
async fn c8_un_jour_trop_gros_est_tronque_avec_mention_explicite_sans_bloquer_la_cloture() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap();
    let t0 = travail(&fx, cours, "Modèle", "maintenant").await;
    // 260 travaux de 20 000 caractères publiés le MÊME jour : 5 200 000 caractères, plus que le plafond (4 000 000).
    sqlx::query(
        "INSERT INTO ent_cdt_travail (id, etablissement_id, cours_id, annee_id, donne_le, echeance_mode, echeance_le, nature_id,
                titre, consigne, etat, publie_a, visible_a, auteur_user_id)
         SELECT gen_random_uuid(), t.etablissement_id, t.cours_id, t.annee_id, t.donne_le, t.echeance_mode, t.echeance_le, t.nature_id,
                'Même jour ' || g, repeat('x', 20000), 'publie', now(), now() - interval '1 day', t.auteur_user_id
           FROM ent_cdt_travail t, generate_series(1, 260) g WHERE t.id = $1",
    )
    .bind(t0)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    let r = fx
        .ent
        .cdt_c8_cloturer(
            &fx.direction(),
            ClotureBody {
                annee_id: annee.id,
                confirmer: true,
            },
        )
        .await
        .unwrap();
    assert!(r.archives_produites >= 1);
    let (tronquees, plafond_ok): (i64, bool) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE x.contenu LIKE '%ARCHIVE TRONQUÉE%'), bool_and(length(x.contenu) <= 4000000)
           FROM ent_cdt_archive a
           JOIN ent_cdt_export x ON x.etablissement_id = a.etablissement_id AND x.id = a.export_id
          WHERE a.etablissement_id = $1 AND a.annee_id = $2 AND a.cours_id = $3",
    )
    .bind(etab)
    .bind(annee.id)
    .bind(cours)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap();
    assert!(
        tronquees >= 1,
        "la troncature est écrite dans le texte de l'archive"
    );
    assert!(plafond_ok, "aucune archive ne dépasse le plafond");
}

/// Un octroi croisé (A donne à B, puis B donne à A) est signalé par `croise` sur le second, jamais sur le premier.
#[tokio::test]
async fn c8_octroi_croise_est_signale_sur_le_second_octroi_seulement() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    entree(
        &fx,
        cours,
        seance_a(&fx, auj - Duration::days(1)).await,
        "Entrée.",
        "maintenant",
    )
    .await;
    let (d1, d2) = (fx.cast.directions[0].user_id, fx.cast.directions[1].user_id);
    let du = auj - Duration::days(7);
    let premier = fx
        .ent
        .cdt_c8_acces_accorder(&fx.direction(), acces(d2, cours, du, auj))
        .await
        .unwrap();
    assert!(!premier.croise, "l'octroi simple n'est pas croisé");
    let second = fx
        .ent
        .cdt_c8_acces_accorder(&fx.direction2(), acces(d1, cours, du, auj))
        .await
        .unwrap();
    assert!(
        second.croise,
        "B donne à A après que A a donné à B : signalé"
    );
}

/// `cahier.cloturer` seul (sans `cahier.exporter`) : décider d'une archive et la lister, jamais lire son texte.
#[tokio::test]
async fn c8_cloturer_sans_exporter_decide_et_liste_les_archives_sans_lire_leur_texte() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap();
    entree(
        &fx,
        cours,
        seance_a(&fx, auj).await,
        "Contenu.",
        "maintenant",
    )
    .await;
    fx.ent
        .cdt_c8_cloturer(
            &fx.direction(),
            ClotureBody {
                annee_id: annee.id,
                confirmer: true,
            },
        )
        .await
        .unwrap();
    let ancienne = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_cdt_archive (id, etablissement_id, annee_id, cours_id, export_id, empreinte, produite_le, version, echeance_archive)
         SELECT $2, etablissement_id, annee_id, cours_id, export_id, empreinte, produite_le, 9, ($3::date - 1)
           FROM ent_cdt_archive WHERE etablissement_id = $1 LIMIT 1",
    )
    .bind(etab)
    .bind(ancienne)
    .bind(auj)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    let dir = (fx.cast.direction.user_id, Role::Direction.as_str());
    let role = fx
        .ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            RoleSaisie {
                nom: format!("Clôture {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::CahierCloturer,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    let delegue = &fx.cast.profs[1];
    fx.ent
        .attribuer_role_for_actor(etab, dir.0, dir.1, role.id, delegue.user_id, None)
        .await
        .unwrap();
    let a = act(etab, delegue);
    assert!(
        !fx.ent
            .cdt_droit_etab(&a, Action::CahierExporter)
            .await
            .unwrap(),
        "il n'exporte pas"
    );
    assert!(!fx
        .ent
        .cdt_c8_archives(&a, Some(annee.id))
        .await
        .unwrap()
        .is_empty());
    let d = fx
        .ent
        .cdt_c8_archive_decider(
            &a,
            ancienne,
            ArchiveDecisionBody {
                decision: "versee".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(d.decision, "versee");
    assert!(fx
        .ent
        .cdt_c8_archive_contenu(&a, ancienne)
        .await
        .is_err_and(|e| est_interdit(&e)));
}

// ───────────────────────── suites : tables filles classées (F2), gel fermé (F3), reprise réservée ─────────────────────────

/// F2 : une ligne par table fille classée par 2419 (groupes de remise, appartenances, notes d'accompagnement, cours d'une
/// continuité) : (table, INSERT valide, UPDATE neutre, DELETE).
async fn jeu_gel_filles(fx: &Fx) -> JeuGel {
    let etab = fx.cast.etablissement_id;
    let cours = cours_a(fx).await;
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap()
        .id;
    let t1 = travail(fx, cours, "Travail des tables filles", "maintenant").await;
    let (g1, cont) = (Uuid::new_v4(), Uuid::new_v4());
    let agent: Uuid =
        sqlx::query_scalar("SELECT id FROM ent_agent WHERE etablissement_id = $1 LIMIT 1")
            .bind(etab)
            .fetch_one(fx.ent.pool())
            .await
            .unwrap();
    for q in [
        format!("INSERT INTO ent_cdt_groupe (id, etablissement_id, travail_id, libelle) VALUES ('{g1}', '{etab}', '{t1}', 'G1')"),
        format!("INSERT INTO ent_cdt_continuite (id, etablissement_id, eleve_id, du) VALUES ('{cont}', '{etab}', '{}', DATE '2026-01-15')", fx.cast.eleves[0].id),
    ] {
        sqlx::query(&q).execute(fx.ent.pool()).await.unwrap_or_else(|e| panic!("jeu : {q} : {e}"));
    }
    let eleve = fx.cast.eleves[1].id;
    let lignes = vec![
        (
            "ent_cdt_groupe",
            format!("INSERT INTO ent_cdt_groupe (id, etablissement_id, travail_id, libelle) VALUES ('{}', '{etab}', '{t1}', 'G2')", Uuid::new_v4()),
            format!("UPDATE ent_cdt_groupe SET libelle = libelle WHERE id = '{g1}'"),
            format!("DELETE FROM ent_cdt_groupe WHERE id = '{g1}'"),
        ),
        (
            "ent_cdt_groupe_membre",
            format!("INSERT INTO ent_cdt_groupe_membre (etablissement_id, groupe_id, eleve_id) VALUES ('{etab}', '{g1}', '{eleve}')"),
            format!("UPDATE ent_cdt_groupe_membre SET cree_le = cree_le WHERE groupe_id = '{g1}'"),
            format!("DELETE FROM ent_cdt_groupe_membre WHERE groupe_id = '{g1}'"),
        ),
        (
            "ent_cdt_note_accompagnement",
            format!("INSERT INTO ent_cdt_note_accompagnement (id, etablissement_id, travail_id, eleve_id, agent_aesh_id, texte) VALUES ('{}', '{etab}', '{t1}', '{eleve}', '{agent}', 'Note')", Uuid::new_v4()),
            format!("UPDATE ent_cdt_note_accompagnement SET cree_le = cree_le WHERE travail_id = '{t1}'"),
            format!("DELETE FROM ent_cdt_note_accompagnement WHERE travail_id = '{t1}'"),
        ),
        (
            "ent_cdt_continuite_cours",
            format!("INSERT INTO ent_cdt_continuite_cours (etablissement_id, continuite_id, cours_id) VALUES ('{etab}', '{cont}', '{cours}')"),
            format!("UPDATE ent_cdt_continuite_cours SET cree_le = cree_le WHERE continuite_id = '{cont}'"),
            format!("DELETE FROM ent_cdt_continuite_cours WHERE continuite_id = '{cont}'"),
        ),
    ];
    JeuGel { annee, lignes }
}

#[tokio::test]
async fn c8_suites_f2_tables_filles_gelees_apres_cloture_du_cahier() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let jeu = jeu_gel_filles(&fx).await;
    inserer_avant_gel(&fx, &jeu).await;
    cloturer(&fx, jeu.annee).await;
    verifier_gel(&fx, &jeu, "cahier clôturé (tables filles)").await;
    // Réouverture tracée : les écritures repassent (le gel est bien celui de l'année, pas une interdiction définitive).
    rouvrir_a_deux(&fx, jeu.annee).await;
    for (table, _, upd, _) in jeu.lignes.iter().filter(|l| !ajout_seul(l.0)) {
        let n = sqlx::query(upd)
            .execute(fx.ent.pool())
            .await
            .unwrap()
            .rows_affected();
        assert!(n > 0, "après réouverture : UPDATE {table}");
    }
}

#[tokio::test]
async fn c8_suites_f2_tables_filles_gelees_dans_une_annee_close_au_socle_5() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let jeu = jeu_gel_filles(&fx).await;
    inserer_avant_gel(&fx, &jeu).await;
    sqlx::query(
        "UPDATE ent_annee_scolaire SET cloturee_le = now() WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(fx.cast.etablissement_id)
    .bind(jeu.annee)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    verifier_gel(&fx, &jeu, "année close au socle 5 (tables filles)").await;
}

/// Hors gel (contre-revue du pilote, Moyen 2 et Faible 1) : les cours d'une continuité sont en ajout seul ; aucune des
/// quatre tables filles ne se vide par TRUNCATE ; groupe, appartenance et note restent supprimables (minimisation), le gel
/// d'année protégeant seul les années closes. Vérifié SANS année close : un refus ne vient donc pas du gel.
#[tokio::test]
async fn c8_suites_f2_tables_filles_hors_gel_ajout_seul_et_sans_truncate() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let jeu = jeu_gel_filles(&fx).await;
    inserer_avant_gel(&fx, &jeu).await;
    for (table, _, upd, del) in &jeu.lignes {
        if table == &"ent_cdt_continuite_cours" {
            for (quoi, q) in [("UPDATE", upd), ("DELETE", del)] {
                assert_eq!(
                    code_sql(sqlx::query(q).execute(fx.ent.pool()).await).as_deref(),
                    Some("23514"),
                    "{quoi} {table} : ajout seul"
                );
            }
        }
        // TRUNCATE dans une transaction toujours annulée : s'il passait, rien ne serait perdu.
        let mut tx = fx.ent.pool_verif().begin().await.unwrap();
        let r = sqlx::query(&format!("TRUNCATE {table} CASCADE")).execute(&mut *tx).await;
        tx.rollback().await.unwrap();
        assert_eq!(code_sql(r).as_deref(), Some("42501"), "TRUNCATE {table}");
    }
}

/// Exécute `ordres` sous le rôle applicatif (non propriétaire, RLS stricte, `SET LOCAL ROLE`) dans une transaction toujours
/// annulée ; `masquer_annees` rend la ligne de chaque année invisible à ce rôle (politique RESTRICTIVE posée par le
/// propriétaire dans la même transaction : l'écriture visée, elle, reste visible). Rend le résultat de chaque ordre.
async fn sous_role_applicatif(
    fx: &Fx,
    masquer_annees: bool,
    ordres: &[String],
) -> Vec<Result<u64, (String, String)>> {
    let role = crate::tests_rls_stricte::ROLE_STRICT;
    crate::tests_rls_stricte::assurer_role_strict(fx.ent.pool_verif()).await;
    let mut tx = fx.ent.pool_verif().begin().await.unwrap();
    if masquer_annees {
        for q in [
            "ALTER TABLE ent_annee_scolaire ENABLE ROW LEVEL SECURITY".to_string(),
            format!("CREATE POLICY suites_f3_masque ON ent_annee_scolaire AS RESTRICTIVE FOR SELECT TO {role} USING (false)"),
        ] {
            sqlx::query(&q).execute(&mut *tx).await.unwrap_or_else(|e| panic!("{q} : {e}"));
        }
    }
    sqlx::query(&format!("SET LOCAL ROLE {role}"))
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(fx.cast.etablissement_id.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let mut rendu = Vec::new();
    for q in ordres {
        // Un point de sauvegarde par ordre : un refus n'annule pas la transaction.
        sqlx::query("SAVEPOINT ordre")
            .execute(&mut *tx)
            .await
            .unwrap();
        let r = sqlx::query(q).execute(&mut *tx).await;
        match r {
            Ok(x) => rendu.push(Ok(x.rows_affected())),
            Err(e) => {
                let d = e
                    .as_database_error()
                    .unwrap_or_else(|| panic!("erreur hors base : {e}"));
                rendu.push(Err((
                    d.code().map(|c| c.to_string()).unwrap_or_default(),
                    d.message().to_string(),
                )));
                sqlx::query("ROLLBACK TO SAVEPOINT ordre")
                    .execute(&mut *tx)
                    .await
                    .unwrap();
            }
        }
    }
    tx.rollback().await.unwrap();
    rendu
}

/// F3 : si la ligne de l'année n'est pas visible (RLS, ligne absente), le gel REFUSE l'écriture. Témoin : les mêmes écritures
/// passent sous le même rôle quand la ligne de l'année est visible.
#[tokio::test]
async fn c8_suites_f3_gel_echoue_ferme_quand_l_annee_est_invisible() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let jeu = jeu_gel_filles(&fx).await;
    let t1: Uuid = sqlx::query_scalar(
        "SELECT travail_id FROM ent_cdt_groupe WHERE etablissement_id = $1 LIMIT 1",
    )
    .bind(etab)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap();
    // Une table mère (gel de 2407) et une table fille (gel de 2419).
    let ordres = vec![
        format!("UPDATE ent_cdt_travail SET modifie_le = modifie_le WHERE id = '{t1}'"),
        format!("INSERT INTO ent_cdt_groupe (id, etablissement_id, travail_id, libelle) VALUES ('{}', '{etab}', '{t1}', 'F3')", Uuid::new_v4()),
    ];
    // Témoin : année visible, année non gelée → les deux écritures passent.
    let temoin = sous_role_applicatif(&fx, false, &ordres).await;
    assert!(
        temoin.iter().all(|r| matches!(r, Ok(1))),
        "témoin : {temoin:?}"
    );
    // Année invisible → refus du gel (23514), jamais un succès silencieux.
    let masque = sous_role_applicatif(&fx, true, &ordres).await;
    for (q, r) in ordres.iter().zip(&masque) {
        match r {
            Err((code, _)) => assert_eq!(code, "23514", "{q}"),
            Ok(n) => panic!("année invisible : l'écriture « {q} » a passé ({n} ligne) alors que le gel doit échouer fermé"),
        }
    }
    // Fonction de garde : année absente → gelée ; année connue et ouverte → non gelée ; pas d'année du tout → non gelée.
    let gelee = |annee: Option<Uuid>| {
        let fx = &fx;
        async move {
            sqlx::query_scalar::<_, bool>("SELECT ent_cdt_annee_gelee($1, $2)")
                .bind(etab)
                .bind(annee)
                .fetch_one(fx.ent.pool())
                .await
                .unwrap()
        }
    };
    assert!(
        gelee(Some(Uuid::new_v4())).await,
        "année absente : gelée (échec fermé)"
    );
    assert!(!gelee(Some(jeu.annee)).await, "année ouverte : non gelée");
    assert!(!gelee(None).await, "aucune année : pas de gel");
}

/// 2419 : `ent_cdt_reprise(uuid)` n'est plus exécutable par PUBLIC ni par le rôle applicatif ; le propriétaire (migrations,
/// tests de reprise) la garde.
#[tokio::test]
async fn c8_suites_reprise_refusee_au_role_applicatif() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let role = crate::tests_rls_stricte::ROLE_STRICT;
    let etab = fx.cast.etablissement_id;
    let r = sous_role_applicatif(
        &fx,
        false,
        &[
            format!("SELECT count(*) FROM ent_cdt_reprise('{etab}')"),
            "SELECT count(*) FROM ent_cdt_reprise(NULL)".to_string(),
        ],
    )
    .await;
    for x in &r {
        let (code, msg) = x
            .as_ref()
            .err()
            .unwrap_or_else(|| panic!("le rôle applicatif a pu exécuter la reprise : {r:?}"));
        assert_eq!(code, "42501", "refus de privilège attendu : {msg}");
        assert!(msg.contains("ent_cdt_reprise"), "{msg}");
    }
    let (applicatif, public, proprietaire): (bool, bool, bool) = sqlx::query_as(
        "SELECT has_function_privilege($1, 'public.ent_cdt_reprise(uuid)', 'EXECUTE'),
                EXISTS (SELECT 1 FROM pg_proc p, aclexplode(p.proacl) a
                         WHERE p.oid = 'public.ent_cdt_reprise(uuid)'::regprocedure AND a.grantee = 0 AND a.privilege_type = 'EXECUTE'),
                has_function_privilege(current_user, 'public.ent_cdt_reprise(uuid)', 'EXECUTE')",
    )
    .bind(role)
    .fetch_one(fx.ent.pool_verif())
    .await
    .unwrap();
    assert!(
        !applicatif && !public,
        "ni le rôle applicatif ni PUBLIC n'exécutent la reprise"
    );
    assert!(proprietaire, "le propriétaire la garde");
}

/// Contre-revue du pilote (suites cahier, Moyen 1) : la fusion d'élève ne déplace que les notes d'accompagnement d'une
/// année NON figée (condition du registre `domain_eleve::TABLES_ELEVE`) ; celles d'une année close restent sur l'absorbé,
/// au lieu de faire échouer toute la fusion par le gel. Même règle pour les cibles de travail et les variantes d'élève.
#[tokio::test]
async fn c8_suites_fusion_ne_deplace_que_les_lignes_d_une_annee_ouverte() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let jeu = jeu_gel_filles(&fx).await;
    inserer_avant_gel(&fx, &jeu).await;
    let cond = |table: &str| {
        domain_eleve::TABLES_ELEVE
            .iter()
            .find(|t| t.table == table)
            .map(|t| t.condition)
            .expect("table au registre")
    };
    for table in ["ent_cdt_note_accompagnement", "ent_cdt_travail_cible", "ent_cdt_variante_eleve"] {
        assert!(!cond(table).is_empty(), "{table} : déplacement conditionné à l'année ouverte");
    }
    let deplacables = |etab: Uuid| {
        let q = format!(
            "SELECT count(*) FROM ent_cdt_note_accompagnement t WHERE t.etablissement_id = $1 AND ({})",
            cond("ent_cdt_note_accompagnement")
        );
        let pool = fx.ent.pool_verif().clone();
        async move { sqlx::query_scalar::<_, i64>(&q).bind(etab).fetch_one(&pool).await.unwrap() }
    };
    let etab = fx.cast.etablissement_id;
    assert_eq!(deplacables(etab).await, 1, "année ouverte : la note suit l'élève conservé");
    sqlx::query("UPDATE ent_annee_scolaire SET cloturee_le = now() WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(jeu.annee)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(deplacables(etab).await, 0, "année close : la note reste sur l'absorbé");
}
