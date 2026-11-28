//! Socle 27, **lot G3b-2** : marquage déterministe et relecture humaine, jeu fictif « parents séparés + adresse
//! protégée » (§22), Pf3 et Pf4. Aucune heure réelle (règle 33) ; base exigée sous `CI` ou `ENT_EXIGER_BASE`.

use super::*;
use crate::droits_demandes::SaisieDemande;
use crate::droits_extrait_masquage::appliquer_masques;
use uuid::Uuid;

pub(super) const DIR: &str = "direction";
pub(super) const ADF: &str = "administratif";
pub(super) const TEL_AUTRE: &str = "0611223344";
pub(super) const ADRESSE_AUTRE: &str = "12 rue Cachée 69000 Lyon";

pub(super) async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
    })
    .await;
    Some((ent, cast))
}

pub(super) async fn personne_de(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    sqlx::query_scalar(
        "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
    )
    .bind(etab)
    .bind(user)
    .fetch_one(ent.pool())
    .await
    .unwrap()
}

/// Parents séparés : le parent de Tom devient **l'autre parent** de Léa (lien actif), protégé ; un dossier
/// d'inscription porte les coordonnées des deux responsables. Rend la personne de l'autre parent.
pub(super) async fn parents_separes(ent: &PgEnt, cast: &LaboCast, protege: bool) -> Uuid {
    let etab = cast.etablissement_id;
    let autre = personne_de(ent, etab, cast.parent_tom.user_id).await;
    let moi = personne_de(ent, etab, cast.parent_lea.user_id).await;
    let p = ent.pool_verif();
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, autorite_parentale)
         VALUES ($1, $2, $3, $4, true) ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(cast.parent_tom.user_id)
    .execute(p)
    .await
    .expect("lien de l'autre parent");
    if protege {
        sqlx::query(
            "INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine)
             VALUES ($1, $2, $3, 'enc:v1:fictif', 'saisie')",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(autre)
        .execute(p)
        .await
        .expect("protection");
    }
    let campagne = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_inscription_campagne (id, etablissement_id, nature, libelle, annee_cible)
         VALUES ($1, $2, 'reinscription', 'Fictive', '2026-2027')",
    )
    .bind(campagne)
    .bind(etab)
    .execute(p)
    .await
    .expect("campagne");
    let dossier = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_inscription_dossier
             (id, etablissement_id, campagne_id, nature, eleve_id, eleve_nom, eleve_prenom)
         VALUES ($1, $2, $3, 'reinscription', $4, 'Granger', 'Léa')",
    )
    .bind(dossier)
    .bind(etab)
    .bind(campagne)
    .bind(cast.eleve_lea_id)
    .execute(p)
    .await
    .expect("dossier");
    for (rang, personne, nom, tel, adr) in [
        (1i16, moi, "Granger", "0600000001", "1 rue Publique"),
        (2i16, autre, "Dupont", TEL_AUTRE, ADRESSE_AUTRE),
    ] {
        sqlx::query(
            "INSERT INTO ent_inscription_responsable
                 (id, etablissement_id, dossier_id, rang, nom, telephone, adresse, lien, personne_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'autre', $8)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(dossier)
        .bind(rang)
        .bind(nom)
        .bind(tel)
        .bind(adr)
        .bind(personne)
        .execute(p)
        .await
        .expect("responsable");
    }
    autre
}

pub(super) async fn demande(
    ent: &PgEnt,
    cast: &LaboCast,
    qualite: &'static str,
    compte: Option<Uuid>,
) -> crate::droits_demandes::DemandeVue {
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
                demandeur_compte_id: compte,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Demandeur fictif"),
                demandeur_contact: Some("courrier postal"),
                qualite,
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
        .expect("saisie");
    ent.droits_verifier_identite(etab, dir, DIR, v.id, "code_courriel", true, None, None)
        .await
        .expect("identité")
}

fn texte(c: &crate::droits_extrait::ContenuExtrait) -> String {
    format!(
        "{:?}",
        c.sections
            .iter()
            .flat_map(|s| s.tables.iter())
            .collect::<Vec<_>>()
    )
}

/// Parents séparés + adresse protégée : l'extrait **remis** du parent 1 ne contient ni les coordonnées du parent 2
/// ni son adresse protégée ; la relecture montre les éléments marqués ; aucune remise tant qu'il en reste ; la
/// levée d'une protection est refusée au secrétariat (CL27-01, CL27-02, R-RG46).
#[tokio::test]
async fn g3b2_parents_separes_adresse_protegee() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, adm) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.administratif.user_id,
    );
    parents_separes(&ent, &cast, true).await;
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    assert_eq!(
        d.etat, "en_cours",
        "Pf3 : la protection de l'autre parent ne bloque pas le titulaire"
    );
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let e = &e[0];
    assert!(
        e.masquages_a_decider > 0 && !e.remissible_en_l_etat,
        "aucune remise tant qu'un marquage est ouvert"
    );
    // La direction voit le contenu brut et les marquages.
    let brut = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, e.id)
        .await
        .unwrap();
    assert!(
        texte(&brut).contains(TEL_AUTRE),
        "le fixture atteint bien l'extrait brut"
    );
    assert!(brut
        .marquages
        .iter()
        .any(|m| m.raison == "adresse_protegee"));
    // Le secrétariat ne voit ni la valeur protégée ni le code de la raison.
    let vu = ent
        .droits_extrait_lire(etab, adm, ADF, d.id, e.id)
        .await
        .unwrap();
    assert!(!texte(&vu).contains(ADRESSE_AUTRE) && !texte(&vu).contains(TEL_AUTRE));
    assert!(!vu.marquages.iter().any(|m| m.raison == "adresse_protegee"));
    // Lever une protection : refus neutre pour le secrétariat (il ne la voit pas ; même avec son identifiant, refus
    // identique à un marquage inexistant).
    assert!(vu
        .marquages
        .iter()
        .filter(|m| m.levee_reservee_direction)
        .all(|m| m.raison == "a_arbitrer"));
    let protege = brut
        .marquages
        .iter()
        .find(|m| m.raison == "adresse_protegee")
        .expect("marquage de protection");
    assert!(matches!(
        ent.droits_masquage_decider(
            etab,
            adm,
            ADF,
            d.id,
            protege.id,
            "leve",
            Some("je veux voir")
        )
        .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Tout accepter : l'extrait devient remissible et le contenu remis est propre.
    for m in &brut.marquages {
        ent.droits_masquage_decider(etab, dir, DIR, d.id, m.id, "masque", None)
            .await
            .unwrap();
    }
    let apres = ent.droits_extraits(etab, dir, DIR, d.id).await.unwrap();
    assert_eq!(apres[0].masquages_a_decider, 0);
    let marques = ent
        .droits_masquages(etab, dir, DIR, d.id, e.id)
        .await
        .unwrap();
    let remis = appliquer_masques(&brut, &marques);
    let t = texte(&remis);
    assert!(
        !t.contains(TEL_AUTRE),
        "téléphone de l'autre parent absent de l'extrait remis"
    );
    assert!(
        !t.contains(ADRESSE_AUTRE),
        "adresse protégée absente de l'extrait remis"
    );
    assert!(t.contains("0600000001"), "les données du demandeur restent");
}

/// Levée d'un marquage : motif obligatoire ; réservé à la direction pour une protection ; accordée à la direction.
#[tokio::test]
async fn g3b2_levee_motivee_et_ajout_manuel() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, adm) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.administratif.user_id,
    );
    parents_separes(&ent, &cast, true).await;
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let c = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, e[0].id)
        .await
        .unwrap();
    let m = &c.marquages[0];
    assert!(
        ent.droits_masquage_decider(etab, dir, DIR, d.id, m.id, "leve", None)
            .await
            .is_err(),
        "motif obligatoire"
    );
    let v = ent
        .droits_masquage_decider(
            etab,
            dir,
            DIR,
            d.id,
            m.id,
            "leve",
            Some("décision de la direction, pièce vue"),
        )
        .await
        .unwrap();
    assert!(v.iter().any(|x| x.id == m.id && x.decision == "leve"));
    // Ajout manuel par le secrétariat : décidé « masqué » d'emblée, avec motif.
    let section = c.sections[0].fournisseur.clone();
    let table = c
        .sections
        .iter()
        .flat_map(|s| s.tables.iter())
        .next()
        .map(|t| t.nom.clone());
    if let Some(table) = table {
        let sec = c
            .sections
            .iter()
            .find(|s| s.tables.iter().any(|t| t.nom == table))
            .map(|s| s.fournisseur.clone())
            .unwrap_or(section);
        let v = ent
            .droits_masquage_ajouter(
                etab,
                adm,
                ADF,
                d.id,
                e[0].id,
                &sec,
                &table,
                None,
                None,
                None,
                "précision de la famille",
            )
            .await
            .unwrap();
        assert!(v
            .iter()
            .any(|x| x.raison == "manuel" && x.decision == "masque"));
    }
}

/// Une décision déjà prise est reportée sur la nouvelle version quand l'élément n'a pas changé.
#[tokio::test]
async fn g3b2_decision_reportee_a_la_nouvelle_version() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    parents_separes(&ent, &cast, true).await;
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let e1 = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let c = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, e1[0].id)
        .await
        .unwrap();
    for m in &c.marquages {
        ent.droits_masquage_decider(etab, dir, DIR, d.id, m.id, "masque", None)
            .await
            .unwrap();
    }
    let e2 = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    assert_eq!(e2[0].version, 2);
    assert_eq!(
        e2[0].masquages_a_decider, 0,
        "rien à relire quand rien n'a changé"
    );
}

/// R-RG48 : pour un élève demandeur, ses parents ne sont réduits qu'à leur identité de responsables.
#[tokio::test]
async fn g3b2_eleve_demandeur_identite_des_parents_seulement() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    parents_separes(&ent, &cast, false).await;
    let d = demande(&ent, &cast, "soi_meme", None).await;
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let c = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, e[0].id)
        .await
        .unwrap();
    let marques = ent
        .droits_masquages(etab, dir, DIR, d.id, e[0].id)
        .await
        .unwrap();
    let remis = texte(&appliquer_masques(&c, &marques));
    assert!(
        !remis.contains(TEL_AUTRE) && !remis.contains("0600000001"),
        "coordonnées des parents masquées"
    );
    assert!(remis.contains("Dupont"), "l'identité du responsable reste");
}

/// Pf3 : la protection du **demandeur** lui-même le rend « restreint » (arbitrage) ; Pf4 : le secrétariat ne voit
/// alors que « à arbitrer », la direction voit le code.
#[tokio::test]
async fn g3b2_pf3_pf4_qualite_restreinte_opaque_pour_le_secretariat() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, adm) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.administratif.user_id,
    );
    let moi = personne_de(&ent, etab, cast.parent_lea.user_id).await;
    sqlx::query(
        "INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine)
         VALUES ($1, $2, $3, 'enc:v1:fictif', 'saisie')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(moi)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    assert_eq!(
        d.etat, "identite_a_verifier",
        "la protection du demandeur impose l'arbitrage"
    );
    let vue_dir = ent.droits_demande(etab, dir, DIR, d.id).await.unwrap();
    assert!(vue_dir
        .identites
        .iter()
        .any(|i| i.qualite_constatee.as_deref() == Some("restreinte")));
    let vue_adm = ent.droits_demande(etab, adm, ADF, d.id).await.unwrap();
    let json = format!("{vue_adm:?}");
    assert!(
        !json.contains("restreinte"),
        "le code ne sort pas du périmètre de l'arbitrage"
    );
    assert!(vue_adm
        .identites
        .iter()
        .any(|i| i.qualite_constatee.as_deref() == Some("a_arbitrer")));
}
