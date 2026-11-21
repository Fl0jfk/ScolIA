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
    assert!(fx
        .ent
        .cdt_c8_repondre(
            &fx.prof(),
            Uuid::new_v4(),
            ObservationBody { texte: "x".into() }
        )
        .await
        .is_err_and(|e| est_introuvable(&e)));
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
    let e = fx
        .ent
        .cdt_c8_export_professeur(&fx.prof(), annee)
        .await
        .unwrap();
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
    assert!(fx
        .ent
        .cdt_c8_rouvrir(
            &fx.direction(),
            ReouvrirBody {
                annee_id: annee.id,
                motif: "court".into()
            }
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Invalide)));
    fx.ent
        .cdt_c8_rouvrir(
            &fx.direction(),
            ReouvrirBody {
                annee_id: annee.id,
                motif: "Erreur de saisie à corriger".into(),
            },
        )
        .await
        .unwrap();
    assert!(fx
        .ent
        .cdt_c8_rouvrir(
            &fx.direction(),
            ReouvrirBody {
                annee_id: annee.id,
                motif: "Une deuxième fois".into()
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
    assert_eq!(n, 3, "clôture, réouverture, clôture : rien n'est effacé");
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
                motif: "Motif suffisamment long".into()
            }
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
}
