//! Socle 2, lot E9 (partie 1) : suites d'E7-E8 — catalogue E5 (photo, e-mail, mobile), pastille
//! neutre « protocole », pastilles R-E9, plafond journalier des SMS (M7), photo sans métadonnées et
//! sans objet orphelin (F2), statut jamais NULL (F3), trace de la liste des protégés (F4), contraintes
//! validées (F5). Scénarios contre la base locale de test (feature `labo`) ; instants et jours
//! **explicites** (aucune dépendance à l'horloge dans les assertions).

use super::*;
use crate::eleve_confidentialite::PoserConfidentialite;
use crate::eleve_coordonnee::{
    AjouterCoordonneeEleve, SMS_VERIFICATION_PAR_JOUR_ET_ELEVE,
    SMS_VERIFICATION_PAR_JOUR_ET_ETABLISSEMENT,
};
use crate::eleve_photo::ProposerPhoto;
use crate::outbox::ConfigDistribution;
use crate::param_eleve::ModifChamp;
use crate::seed::LaboCast;
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use domain_eleve::{CodeModalite, CreateEleve, SaisieFamille, UpsertPai};
use ent_auth::Role;
use ent_object::provider::MemoryProvider;
use ent_object::ObjectStore;
use std::sync::Arc;
use uuid::Uuid;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 15, 10, 0, 0).unwrap()
}

fn plus(min: i64) -> DateTime<Utc> {
    t0() + Duration::minutes(min)
}

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (mut ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |_e| {}).await;
    ent.install_mail_dry_run();
    Some((ent, cast))
}

fn lettres(n: usize) -> String {
    Uuid::new_v4()
        .as_bytes()
        .iter()
        .take(n)
        .map(|b| (b'a' + b % 26) as char)
        .collect()
}

async fn eleve_le(
    ent: &PgEnt,
    etab: Uuid,
    prenom: &str,
    naissance: NaiveDate,
    entree: Option<NaiveDate>,
) -> domain_eleve::Eleve {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: format!("Neuf{}", lettres(6)),
        prenom: prenom.into(),
        ine: None,
        sexe: Some("F".into()),
        date_naissance: Some(naissance),
        numero_badge: None,
        redoublant: false,
        classe: Some("6A".into()),
        regime: None,
        mef: None,
        site: None,
        date_entree: entree,
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
}

async fn eleve(ent: &PgEnt, etab: Uuid, prenom: &str) -> domain_eleve::Eleve {
    eleve_le(
        ent,
        etab,
        prenom,
        NaiveDate::from_ymd_opt(2012, 3, 14).unwrap(),
        None,
    )
    .await
}

fn user(cast: &LaboCast, r: Role) -> (Uuid, &'static str) {
    let p = match r {
        Role::Admin => &cast.admin,
        Role::Direction => &cast.direction,
        Role::Administratif => &cast.administratif,
        Role::Cpe => &cast.cpe,
        Role::Prof => &cast.prof,
        Role::Infirmerie => &cast.infirmieres[0],
        _ => unreachable!(),
    };
    (p.user_id, r.as_str())
}

async fn distribuer(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(ConfigDistribution {
        etablissement: Some(etab),
        ..Default::default()
    })
    .distribuer_tout()
    .await
    .unwrap();
}

fn forbidden<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

fn absent<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(
        r,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    )
}

fn conflit_avec<T>(r: &Result<T, EntDbError>, morceau: &str) -> bool {
    matches!(r, Err(EntDbError::ConflitDossier(m)) if m.contains(morceau))
}

/// Pastilles de la projection (vérité de la base, sans droit), après recalcul à un jour explicite.
async fn recalculer(ent: &PgEnt, etab: Uuid, eleve: Uuid, jour: NaiveDate) -> Vec<String> {
    ent.reconstruire_indicateurs(etab, None, Some(jour))
        .await
        .unwrap();
    sqlx::query_scalar("SELECT code FROM ent_eleve_indicateur WHERE eleve_id = $1 ORDER BY code")
        .bind(eleve)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap()
}

/// Codes des pastilles qu'un rôle voit sur la fiche (service, droits compris) ; `None` = fiche absente.
async fn vues_par(
    ent: &PgEnt,
    cast: &LaboCast,
    role: Role,
    eleve: Uuid,
) -> Option<Vec<&'static str>> {
    let (u, r) = user(cast, role);
    match ent
        .synthese_eleve_pour_acteur(r, cast.etablissement_id, u, eleve)
        .await
    {
        Ok(s) => Some(s.pastilles.iter().map(|p| p.code).collect()),
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound)) => None,
        Err(e) => panic!("{} : {e:?}", r),
    }
}

/// Début de l'année courante de l'établissement (assurée, jamais devinée) : toutes les dates des
/// scénarios s'en déduisent, aucune ne dépend du jour réel.
async fn debut_annee(ent: &PgEnt, etab: Uuid) -> (NaiveDate, NaiveDate) {
    sqlx::query("SELECT ent_assurer_annee($1, ent_aujourdhui())")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query_as("SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante")
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

fn pai(etab: Uuid, eleve: Uuid, par: Uuid, debut: NaiveDate) -> UpsertPai {
    UpsertPai {
        etablissement_id: etab,
        eleve_id: eleve,
        motif: "allergie sévère aux arachides".into(),
        mesures_accueil: "trousse d'urgence".into(),
        protocole_urgence: "appeler le 15".into(),
        note_interne: String::new(),
        date_debut: debut,
        date_fin: None,
        actif: true,
        actor_user_id: par,
    }
}

// ───────────────────────── (a) catalogue E5 : photo, e-mail, mobile ─────────────────────────

fn jpeg_valide() -> Vec<u8> {
    crate::tests_eleve_protection::JPEG.to_vec()
}

#[tokio::test]
async fn e9a_photo_email_et_mobile_entrent_au_catalogue_et_ferment_la_saisie_famille() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let (admin, radmin) = user(&cast, Role::Admin);
    let (parent, rparent) = (cast.parent_lea.user_id, Role::Parent.as_str());
    let harry = cast.eleve_lea_id;
    let nettoyer = || async {
        sqlx::query("DELETE FROM ent_param_eleve_champ WHERE etablissement_id = $1 AND champ IN ('photo','email_eleve','mobile_eleve')")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .unwrap();
    };
    nettoyer().await;

    // Les trois champs figurent au paramétrage, la famille PROPOSE par défaut (comportement conservé).
    let p = ent.parametrage_eleve(radmin, etab, admin).await.unwrap();
    for code in ["photo", "email_eleve", "mobile_eleve"] {
        let c = p
            .champs
            .iter()
            .find(|c| c.code == code)
            .unwrap_or_else(|| panic!("{code} absent du paramétrage"));
        assert!(!c.regle);
        assert_eq!(c.reglage.saisie_famille, SaisieFamille::Proposer, "{code}");
        assert!(c.reglage.visible && c.reglage.obligatoire_a.is_empty());
    }
    // Contrôle positif : la famille propose une photo et une adresse.
    let mut d = ProposerPhoto {
        etablissement_id: etab,
        eleve_id: harry,
        acteur: parent,
        content_type: "image/jpeg".into(),
        octets: jpeg_valide(),
        prise_le: None,
        valider_aussitot: false,
        photographe: false,
    };
    let (adm, radm) = user(&cast, Role::Administratif);
    let ph = ent
        .proposer_photo_a(rparent, d.clone(), t0())
        .await
        .unwrap();
    assert_eq!(ph.source, "famille");
    // Les propositions de la famille sont rejetées au fil du test : le plafond d'attente (3) ne
    // s'accumule pas d'un passage à l'autre sur la base partagée.
    ent.rejeter_photo(radm, etab, adm, ph.id, "illisible")
        .await
        .unwrap();
    let mail = |s: &str| AjouterCoordonneeEleve {
        etablissement_id: etab,
        eleve_id: harry,
        acteur: parent,
        type_: "email".into(),
        valeur: format!("{s}.{}@exemple.test", lettres(6)),
        usages: vec![],
    };
    let c1 = ent
        .ajouter_coordonnee_eleve_a(rparent, mail("ouvert"), t0())
        .await
        .unwrap();
    ent.retirer_coordonnee_eleve_a(rparent, etab, parent, c1.id, "erreur_de_saisie", plus(1))
        .await
        .unwrap();

    // L'administrateur ferme la saisie famille du champ « photo » : la famille est refusée, le
    // secrétariat non ; réglage d'un autre champ : sans effet sur celui-ci.
    let ferme = |champ: &str| ModifChamp {
        champ: champ.into(),
        saisie_famille: Some(SaisieFamille::Non),
        ..Default::default()
    };
    ent.parametrer_champ(radmin, etab, admin, ferme("photo"))
        .await
        .unwrap();
    assert!(forbidden(
        &ent.proposer_photo_a(rparent, d.clone(), plus(2)).await
    ));
    d.acteur = adm;
    d.valider_aussitot = true;
    assert!(
        ent.proposer_photo_a(radm, d.clone(), plus(2)).await.is_ok(),
        "le secrétariat garde la main"
    );
    assert!(
        ent.ajouter_coordonnee_eleve_a(rparent, mail("autre"), plus(2))
            .await
            .is_ok(),
        "e-mail : réglage distinct"
    );
    // Fermer « email_eleve » ferme l'e-mail pour la famille, pas le mobile.
    ent.parametrer_champ(radmin, etab, admin, ferme("email_eleve"))
        .await
        .unwrap();
    assert!(forbidden(
        &ent.ajouter_coordonnee_eleve_a(rparent, mail("ferme"), plus(3))
            .await
    ));
    let mobile = AjouterCoordonneeEleve {
        type_: "mobile".into(),
        valeur: "06 12 34 56 71".into(),
        ..mail("x")
    };
    assert!(ent
        .ajouter_coordonnee_eleve_a(rparent, mobile.clone(), plus(3))
        .await
        .is_ok());
    ent.parametrer_champ(radmin, etab, admin, ferme("mobile_eleve"))
        .await
        .unwrap();
    let mobile2 = AjouterCoordonneeEleve {
        valeur: "06 12 34 56 72".into(),
        ..mobile
    };
    assert!(forbidden(
        &ent.ajouter_coordonnee_eleve_a(rparent, mobile2, plus(4))
            .await
    ));
    // Réglage d'un autre aspect (visible) : ne rouvre ni ne ferme la saisie (défaut conservé).
    ent.parametrer_champ(
        radmin,
        etab,
        admin,
        ModifChamp {
            champ: "photo".into(),
            visible: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    // Réouverture.
    ent.parametrer_champ(
        radmin,
        etab,
        admin,
        ModifChamp {
            champ: "photo".into(),
            saisie_famille: Some(SaisieFamille::Proposer),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    d.acteur = parent;
    d.valider_aussitot = false;
    let rouverte = ent.proposer_photo_a(rparent, d, plus(5)).await.unwrap();
    ent.rejeter_photo(radm, etab, adm, rouverte.id, "illisible")
        .await
        .unwrap();
    // Le prof n'a pas la main sur le paramétrage.
    let (prof, rprof) = user(&cast, Role::Prof);
    assert!(forbidden(
        &ent.parametrer_champ(rprof, etab, prof, ferme("photo"))
            .await
    ));
    nettoyer().await;
}

// ───────────────────────── (b) pastille neutre « protocole » ─────────────────────────

#[tokio::test]
async fn e9b_protocole_signale_l_infirmerie_a_la_vie_scolaire_sans_rien_de_sante() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let (admin, radmin) = user(&cast, Role::Admin);
    ent.activer_module_sante_test(etab).await;
    let e = eleve(&ent, etab, "Protocole").await;
    let jour = ent.jour_etablissement(etab).await.unwrap();
    let (inf, rinf) = user(&cast, Role::Infirmerie);
    ent.enregistrer_pai_for_actor(rinf, pai(etab, e.id, inf, jour - Duration::days(10)), None)
        .await
        .unwrap();
    distribuer(&ent, etab).await;
    let proj = recalculer(&ent, etab, e.id, jour).await;
    assert!(
        proj.contains(&"pai_actif".to_string()) && proj.contains(&"protocole".to_string()),
        "{proj:?}"
    );

    // Vie scolaire (CPE) : la pastille neutre, JAMAIS « pai_actif ». Rien de santé dans ce qu'elle reçoit.
    let (cpe, rcpe) = user(&cast, Role::Cpe);
    let s = ent
        .synthese_eleve_pour_acteur(rcpe, etab, cpe, e.id)
        .await
        .unwrap();
    let codes: Vec<_> = s.pastilles.iter().map(|p| p.code).collect();
    assert!(
        codes.contains(&"protocole") && !codes.contains(&"pai_actif"),
        "{codes:?}"
    );
    let p = s.pastilles.iter().find(|p| p.code == "protocole").unwrap();
    assert_eq!(
        (p.domaine, p.valeur_code.clone(), p.valeur_date),
        ("classe", None, None),
        "ni nature, ni échéance"
    );
    assert_eq!(p.du, jour, "pas même la date de début du PAI");
    let rendu = format!("{s:?}");
    for mot in [
        "allergie",
        "arachide",
        "trousse",
        "appeler le 15",
        "croix de sante",
    ] {
        assert!(
            !rendu.contains(mot),
            "la synthèse ne doit rien laisser filtrer : {mot}"
        );
    }
    let sante = s.tuiles.iter().find(|t| t.domaine == "sante").unwrap();
    assert!(
        !matches!(sante.etat, crate::eleve_indicateurs::EtatTuile::Visible),
        "la tuile santé reste fermée à la vie scolaire"
    );
    // Le PAI reste réservé à sante.lire : l'infirmerie voit « pai_actif » et pas deux pastilles pour un fait.
    let codes = vues_par(&ent, &cast, Role::Infirmerie, e.id).await.unwrap();
    assert!(
        codes.contains(&"pai_actif") && !codes.contains(&"protocole"),
        "{codes:?}"
    );
    // Le secrétariat (contraintes.lire) la voit aussi ; le professeur non (aucun droit de santé ni de contraintes).
    assert!(vues_par(&ent, &cast, Role::Administratif, e.id)
        .await
        .unwrap()
        .contains(&"protocole"));
    assert!(!vues_par(&ent, &cast, Role::Prof, e.id)
        .await
        .unwrap_or_default()
        .contains(&"protocole"));

    // Liste de travail par pastille : ouverte à la vie scolaire, avec le module ; compteurs idem.
    let l = ent
        .eleves_par_pastille_pour_acteur(rcpe, etab, cpe, "protocole", 100)
        .await
        .unwrap();
    assert!(l.contains(&e.id));
    assert!(ent
        .compteurs_pastilles_pour_acteur(rcpe, etab, cpe)
        .await
        .unwrap()
        .iter()
        .any(|(c, _)| *c == "protocole"));
    // Un PAI ne se lit toujours pas par la liste « pai_actif » côté vie scolaire.
    assert!(forbidden(
        &ent.eleves_par_pastille_pour_acteur(rcpe, etab, cpe, "pai_actif", 100)
            .await
    ));

    // Élève protégé : absent pour la vie scolaire, avec ou sans protocole.
    let (dir, rdir) = user(&cast, Role::Direction);
    ent.poser_confidentialite_a(
        rdir,
        PoserConfidentialite {
            etablissement_id: etab,
            eleve_id: e.id,
            motif: "Mesure de protection".into(),
            mesures: vec![],
            du: Some(jour - Duration::days(1)),
            au: None,
            acteur: dir,
        },
        t0(),
    )
    .await
    .unwrap();
    assert!(absent(
        &ent.synthese_eleve_pour_acteur(rcpe, etab, cpe, e.id).await
    ));
    let l = ent
        .eleves_par_pastille_pour_acteur(rcpe, etab, cpe, "protocole", 100)
        .await
        .unwrap();
    assert!(
        !l.contains(&e.id),
        "un protégé ne figure pas dans la liste d'un tiers"
    );

    // M6 : sans le module santé, le protocole n'existe plus (ni pastille, ni liste, ni compteur).
    let e2 = eleve(&ent, etab, "Protocole2").await;
    ent.enregistrer_pai_for_actor(rinf, pai(etab, e2.id, inf, jour - Duration::days(3)), None)
        .await
        .unwrap();
    distribuer(&ent, etab).await;
    recalculer(&ent, etab, e2.id, jour).await;
    assert!(
        vues_par(&ent, &cast, Role::Cpe, e2.id)
            .await
            .unwrap()
            .contains(&"protocole"),
        "contrôle positif"
    );
    ent.definir_module_sante(etab, admin, radmin, false)
        .await
        .unwrap();
    assert!(!vues_par(&ent, &cast, Role::Cpe, e2.id)
        .await
        .unwrap()
        .contains(&"protocole"));
    assert!(ent
        .eleves_par_pastille_pour_acteur(rcpe, etab, cpe, "protocole", 100)
        .await
        .is_err());
    assert!(!ent
        .compteurs_pastilles_pour_acteur(rcpe, etab, cpe)
        .await
        .unwrap()
        .iter()
        .any(|(c, _)| *c == "protocole"));
    ent.definir_module_sante(etab, admin, radmin, true)
        .await
        .unwrap();
}

// ───────────────────────── (c) pastilles R-E9 ─────────────────────────

#[tokio::test]
async fn e9c_nouvel_arrivant_et_arrive_en_cours_d_annee() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let (debut, fin) = debut_annee(&ent, etab).await;
    let n = NaiveDate::from_ymd_opt(2012, 3, 14).unwrap();

    // À la rentrée : première année dans l'établissement → nouvel arrivant toute l'année, pas « en cours d'année ».
    let rentree = eleve_le(&ent, etab, "Rentree", n, Some(debut + Duration::days(3))).await;
    let p = recalculer(&ent, etab, rentree.id, debut + Duration::days(200)).await;
    assert!(
        p.contains(&"nouvel_arrivant".to_string()),
        "première année : {p:?}"
    );
    assert!(!p.contains(&"arrive_en_cours_annee".to_string()));

    // Arrivé en cours d'année (entrée au-delà des deux premières semaines).
    let entree = debut + Duration::days(100);
    assert!(entree < fin);
    let tardif = eleve_le(&ent, etab, "Tardif", n, Some(entree)).await;
    let p = recalculer(&ent, etab, tardif.id, entree + Duration::days(10)).await;
    assert!(
        p.contains(&"nouvel_arrivant".to_string())
            && p.contains(&"arrive_en_cours_annee".to_string()),
        "{p:?}"
    );
    // Avant son entrée : élève « à venir », la scolarisation ne couvre pas encore le jour → aucune des deux.
    let p = recalculer(&ent, etab, tardif.id, entree - Duration::days(5)).await;
    assert!(
        !p.contains(&"nouvel_arrivant".to_string())
            && !p.contains(&"arrive_en_cours_annee".to_string()),
        "{p:?}"
    );

    // Déjà connu de l'établissement (scolarisation antérieure) : « nouvel arrivant » seulement 30 jours.
    let revenu = eleve_le(&ent, etab, "Revenu", n, Some(entree)).await;
    sqlx::query(
        "INSERT INTO ent_eleve_scolarisation (id, etablissement_id, eleve_id, date_entree, motif_entree, date_sortie, motif_sortie, source)
         VALUES (gen_random_uuid(), $1, $2, $3, 'premiere_inscription', $4, 'autre', 'saisie')",
    )
    .bind(etab)
    .bind(revenu.id)
    .bind(debut - Duration::days(400))
    .bind(debut - Duration::days(300))
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let p = recalculer(&ent, etab, revenu.id, entree + Duration::days(29)).await;
    assert!(
        p.contains(&"nouvel_arrivant".to_string()),
        "moins de 30 jours : {p:?}"
    );
    let p = recalculer(&ent, etab, revenu.id, entree + Duration::days(31)).await;
    assert!(
        !p.contains(&"nouvel_arrivant".to_string()),
        "plus de 30 jours, déjà connu : {p:?}"
    );
    assert!(
        p.contains(&"arrive_en_cours_annee".to_string()),
        "l'arrivée en cours d'année dure toute l'année"
    );

    // Un passage interne n'est pas une arrivée.
    let interne = eleve_le(&ent, etab, "Interne", n, Some(entree)).await;
    sqlx::query(
        "UPDATE ent_eleve_scolarisation SET motif_entree = 'passage_interne' WHERE eleve_id = $1",
    )
    .bind(interne.id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let p = recalculer(&ent, etab, interne.id, entree + Duration::days(5)).await;
    assert!(
        !p.contains(&"nouvel_arrivant".to_string())
            && !p.contains(&"arrive_en_cours_annee".to_string()),
        "{p:?}"
    );

    // Visibilité : secrétariat, direction et vie scolaire (tuile scolarité ouverte), pas le professeur
    // (tuile fermée). La fiche « rentrée » est reprojetée à un jour où la pastille est vraie.
    let p = recalculer(&ent, etab, rentree.id, debut + Duration::days(40)).await;
    assert!(p.contains(&"nouvel_arrivant".to_string()), "{p:?}");
    for (role, attendu) in [
        (Role::Administratif, true),
        (Role::Direction, true),
        (Role::Cpe, true),
        (Role::Prof, false),
    ] {
        let vues = vues_par(&ent, &cast, role, rentree.id)
            .await
            .unwrap_or_default();
        assert_eq!(
            vues.contains(&"nouvel_arrivant"),
            attendu,
            "{role:?} : {vues:?}"
        );
    }
}

#[tokio::test]
async fn e9c_majorite_proche_est_un_booleen_sans_date() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let jour = ent.jour_etablissement(etab).await.unwrap();
    // Naissance choisie pour que les 18 ans tombent dans `delta` jours (calcul exact par la base).
    let naissance_pour = |delta: i32| -> NaiveDate {
        let cible = jour + Duration::days(i64::from(delta));
        cible
            .with_year(cible.year() - 18)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(cible.year() - 18, 2, 28).unwrap())
    };
    let proche = eleve_le(&ent, etab, "Proche", naissance_pour(10), None).await;
    let limite = eleve_le(&ent, etab, "Limite", naissance_pour(30), None).await;
    let loin = eleve_le(&ent, etab, "Loin", naissance_pour(31), None).await;
    let aujourdhui = eleve_le(&ent, etab, "Aujourdhui", naissance_pour(0), None).await;
    let deja = eleve_le(&ent, etab, "Deja", naissance_pour(-20), None).await;
    ent.reconstruire_indicateurs(etab, None, Some(jour))
        .await
        .unwrap();
    let a = |p: &[String], c: &str| p.contains(&c.to_string());
    let proj = |id: Uuid| {
        let ent = &ent;
        async move {
            sqlx::query_scalar::<_, String>(
                "SELECT code FROM ent_eleve_indicateur WHERE eleve_id = $1",
            )
            .bind(id)
            .fetch_all(ent.pool_verif())
            .await
            .unwrap()
        }
    };
    assert!(a(&proj(proche.id).await, "majorite_proche"));
    assert!(a(&proj(limite.id).await, "majorite_proche"), "J-30 inclus");
    assert!(
        !a(&proj(loin.id).await, "majorite_proche"),
        "J-31 : pas encore"
    );
    let p0 = proj(aujourdhui.id).await;
    assert!(
        !a(&p0, "majorite_proche") && a(&p0, "majeur"),
        "le jour des 18 ans : majeur, plus « proche »"
    );
    let pd = proj(deja.id).await;
    assert!(!a(&pd, "majorite_proche") && a(&pd, "majeur"));

    // UN BOOLÉEN : ni valeur, ni date, jusque dans la projection ; `du` = le jour du calcul.
    let (vc, vd, du): (Option<String>, Option<NaiveDate>, NaiveDate) = sqlx::query_as(
        "SELECT valeur_code, valeur_date, du FROM ent_eleve_indicateur WHERE eleve_id = $1 AND code = 'majorite_proche'",
    )
    .bind(proche.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((vc, vd, du), (None, None, jour));
    // Servie par le service : toujours sans date, aux seuls droits de l'état civil.
    let (dir, rdir) = user(&cast, Role::Direction);
    let s = ent
        .synthese_eleve_pour_acteur(rdir, etab, dir, proche.id)
        .await
        .unwrap();
    let p = s
        .pastilles
        .iter()
        .find(|p| p.code == "majorite_proche")
        .expect("visible de la direction");
    assert_eq!(
        (p.valeur_code.clone(), p.valeur_date, p.du),
        (None, None, jour)
    );
    assert!(
        !format!("{s:?}").contains(&naissance_pour(10).to_string()),
        "la date de naissance ne fuit pas"
    );
    // L'administration et la vie scolaire la voient (booléen).
    assert!(vues_par(&ent, &cast, Role::Cpe, proche.id)
        .await
        .unwrap()
        .contains(&"majorite_proche"));
    assert!(!vues_par(&ent, &cast, Role::Prof, proche.id)
        .await
        .unwrap_or_default()
        .contains(&"majorite_proche"));
}

#[tokio::test]
async fn e9c_modalite_particuliere_sans_detail() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let e = eleve(&ent, etab, "Modalite").await;
    let jour = ent.jour_etablissement(etab).await.unwrap();
    assert!(!recalculer(&ent, etab, e.id, jour)
        .await
        .contains(&"modalite_particuliere".to_string()));
    ent.enregistrer_modalite(
        radm,
        etab,
        e.id,
        CodeModalite::Ulis,
        jour - Duration::days(5),
        None,
        None,
        adm,
    )
    .await
    .unwrap();
    distribuer(&ent, etab).await;
    let p = recalculer(&ent, etab, e.id, jour).await;
    assert!(p.contains(&"modalite_particuliere".to_string()), "{p:?}");
    let (vc, vd): (Option<String>, Option<NaiveDate>) =
        sqlx::query_as("SELECT valeur_code, valeur_date FROM ent_eleve_indicateur WHERE eleve_id = $1 AND code = 'modalite_particuliere'")
            .bind(e.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(
        (vc, vd),
        (None, None),
        "ni le code (ULIS…) ni la date de fin ne sont dans la pastille"
    );
    let s = ent
        .synthese_eleve_pour_acteur(radm, etab, adm, e.id)
        .await
        .unwrap();
    assert!(s
        .pastilles
        .iter()
        .any(|p| p.code == "modalite_particuliere"));
    assert!(!format!("{s:?}").to_lowercase().contains("ulis"));
    // La vie scolaire la voit (booléen) ; le professeur d'une autre classe n'a même pas la fiche.
    assert!(vues_par(&ent, &cast, Role::Cpe, e.id)
        .await
        .unwrap()
        .contains(&"modalite_particuliere"));
    assert!(!vues_par(&ent, &cast, Role::Prof, e.id)
        .await
        .unwrap_or_default()
        .contains(&"modalite_particuliere"));
    // Modalité terminée : la pastille disparaît (événement `scolarite.modalite_changee` écouté).
    let (id,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_eleve_modalite WHERE eleve_id = $1")
        .bind(e.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    ent.terminer_modalite(radm, etab, e.id, id, jour, adm)
        .await
        .unwrap();
    distribuer(&ent, etab).await;
    let apres: Vec<String> =
        sqlx::query_scalar("SELECT code FROM ent_eleve_indicateur WHERE eleve_id = $1")
            .bind(e.id)
            .fetch_all(ent.pool_verif())
            .await
            .unwrap();
    assert!(
        !apres.contains(&"modalite_particuliere".to_string()),
        "{apres:?}"
    );
}

/// Une pastille n'est visible que si sa tuile l'est : vérifié pour TOUTES les pastilles du catalogue
/// (dont les cinq nouvelles) et pour tous les rôles du personnel.
#[tokio::test]
async fn e9c_une_pastille_ne_depasse_jamais_sa_tuile_pour_tout_le_catalogue() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    ent.activer_module_sante_test(etab).await;
    let jour = ent.jour_etablissement(etab).await.unwrap();
    // Un élève qui porte TOUTES les pastilles datées possibles : on force la projection.
    let e = eleve(&ent, etab, "Toutes").await;
    for d in domain_eleve::CATALOGUE_INDICATEURS {
        sqlx::query(
            "INSERT INTO ent_eleve_indicateur (etablissement_id, eleve_id, code, du) VALUES ($1, $2, $3, $4)
             ON CONFLICT (etablissement_id, eleve_id, code) DO NOTHING",
        )
        .bind(etab)
        .bind(e.id)
        .bind(d.code)
        .bind(jour)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    }
    for role in [
        Role::Admin,
        Role::Direction,
        Role::Administratif,
        Role::Cpe,
        Role::Prof,
        Role::Infirmerie,
    ] {
        let (u, r) = user(&cast, role);
        let Ok(s) = ent.synthese_eleve_pour_acteur(r, etab, u, e.id).await else {
            continue;
        };
        for p in &s.pastilles {
            let tuile = s
                .tuiles
                .iter()
                .find(|t| t.domaine == p.domaine)
                .expect("tuile du domaine");
            assert!(
                matches!(tuile.etat, crate::eleve_indicateurs::EtatTuile::Visible),
                "{r} voit « {} » alors que la tuile « {} » est fermée",
                p.code,
                p.domaine
            );
        }
    }
}

// ───────────────────────── M7 : plafond journalier des SMS ─────────────────────────

#[tokio::test]
async fn m7_plafond_journalier_de_sms_par_eleve_et_par_etablissement() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let e = eleve(&ent, etab, "Sms").await;
    let num = format!(
        "06 {:02} {:02} {:02} {:02}",
        10 + e.id.as_u128() % 80,
        10 + (e.id.as_u128() >> 8) % 80,
        10 + (e.id.as_u128() >> 16) % 80,
        10 + (e.id.as_u128() >> 24) % 80
    );
    let c = ent
        .ajouter_coordonnee_eleve_a(
            radm,
            AjouterCoordonneeEleve {
                etablissement_id: etab,
                eleve_id: e.id,
                acteur: adm,
                type_: "mobile".into(),
                valeur: num,
                usages: vec![],
            },
            t0(),
        )
        .await
        .unwrap();
    // 3 par heure (règle existante) : deux fois trois envois répartis sur deux heures = 6 en 24 h.
    assert_eq!(SMS_VERIFICATION_PAR_JOUR_ET_ELEVE, 6);
    for min in [1, 2, 3, 61, 62, 63] {
        ent.demander_verification_coordonnee_eleve_a(radm, etab, adm, c.id, plus(min))
            .await
            .unwrap();
    }
    // Septième SMS dans la même journée, deux heures plus tard : plafond quotidien (message distinct de l'horaire).
    let r = ent
        .demander_verification_coordonnee_eleve_a(radm, etab, adm, c.id, plus(125))
        .await;
    assert!(conflit_avec(&r, "quotidien"), "{r:?}");
    // 24 heures après le premier envoi, la fenêtre glisse : de nouveau permis.
    assert!(ent
        .demander_verification_coordonnee_eleve_a(radm, etab, adm, c.id, plus(24 * 60 + 5))
        .await
        .is_ok());

    // Un e-mail n'est pas concerné par le plafond des SMS (le plafond horaire existant seul s'applique).
    let mail = ent
        .ajouter_coordonnee_eleve_a(
            radm,
            AjouterCoordonneeEleve {
                etablissement_id: etab,
                eleve_id: e.id,
                acteur: adm,
                type_: "email".into(),
                valeur: format!("m7.{}@exemple.test", lettres(6)),
                usages: vec![],
            },
            plus(130),
        )
        .await
        .unwrap();
    assert!(ent
        .demander_verification_coordonnee_eleve_a(radm, etab, adm, mail.id, plus(131))
        .await
        .is_ok());

    // Plafond d'établissement : (instants éloignés dans le futur, hors de tout autre test) l'établissement
    // atteint son quota ; un autre élève est refusé à son premier SMS.
    let loin = Utc.with_ymd_and_hms(2031, 6, 1, 10, 0, 0).unwrap();
    let autre = eleve(&ent, etab, "SmsEtab").await;
    let num2 = format!(
        "07 {:02} {:02} {:02} {:02}",
        10 + autre.id.as_u128() % 80,
        10 + (autre.id.as_u128() >> 8) % 80,
        10 + (autre.id.as_u128() >> 16) % 80,
        10 + (autre.id.as_u128() >> 24) % 80
    );
    let c2 = ent
        .ajouter_coordonnee_eleve_a(
            radm,
            AjouterCoordonneeEleve {
                etablissement_id: etab,
                eleve_id: autre.id,
                acteur: adm,
                type_: "mobile".into(),
                valeur: num2,
                usages: vec![],
            },
            loin,
        )
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_coordonnee_verification
            (id, etablissement_id, eleve_id, coordonnee_id, canal, code_hash, expire_le, demande_par, envoye_le, statut)
         SELECT gen_random_uuid(), $1, $2, $3, 'sms', 'x', $4, $5, $4 - interval '2 hours', 'annulee' FROM generate_series(1, $6)",
    )
    .bind(etab)
    .bind(e.id)
    .bind(c.id)
    .bind(loin)
    .bind(adm)
    .bind(i32::try_from(SMS_VERIFICATION_PAR_JOUR_ET_ETABLISSEMENT).unwrap())
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let r = ent
        .demander_verification_coordonnee_eleve_a(radm, etab, adm, c2.id, loin)
        .await;
    assert!(
        conflit_avec(&r, "quotidien"),
        "quota d'établissement : {r:?}"
    );
    // Le lendemain, plus rien ne pèse.
    assert!(ent
        .demander_verification_coordonnee_eleve_a(
            radm,
            etab,
            adm,
            c2.id,
            loin + Duration::hours(21)
        )
        .await
        .is_err());
    assert!(ent
        .demander_verification_coordonnee_eleve_a(
            radm,
            etab,
            adm,
            c2.id,
            loin + Duration::hours(26)
        )
        .await
        .is_ok());
    // Nettoyage : ces lignes ne servent qu'à ce test.
    sqlx::query("DELETE FROM ent_eleve_coordonnee_verification WHERE etablissement_id = $1 AND code_hash = 'x'")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

// ───────────────────────── F2 : photo sans EXIF, pas d'orphelin ─────────────────────────

fn segment(marqueur: u8, corps: &[u8]) -> Vec<u8> {
    let mut v = vec![0xff, marqueur];
    v.extend_from_slice(&u16::try_from(corps.len() + 2).unwrap().to_be_bytes());
    v.extend_from_slice(corps);
    v
}

/// JPEG minimal complet (gris 1×1) portant un EXIF avec GPS, un nom d'appareil et un commentaire.
fn jpeg_exif_gps() -> Vec<u8> {
    let mut exif = b"Exif\0\0MM\0*\0\0\0\x08\0\x02".to_vec();
    exif.extend_from_slice(&[0x01, 0x12, 0x00, 0x03, 0, 0, 0, 1, 0, 6, 0, 0]);
    exif.extend_from_slice(&[0x88, 0x25, 0x00, 0x04, 0, 0, 0, 1, 0, 0, 0, 0]);
    exif.extend_from_slice(b"\0\0\0\0GPSLatitude=48.8566N GPSLongitude=2.3522E Appareil=PhoneX");
    let mut j = vec![0xff, 0xd8];
    j.extend(segment(0xe1, &exif));
    j.extend(segment(0xfe, b"commentaire prive"));
    j.extend(segment(0xdb, &[0u8; 65]));
    j.extend(segment(0xc0, &[8, 0, 1, 0, 1, 1, 1, 0x11, 0]));
    j.extend(segment(
        0xc4,
        &[0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ));
    j.extend(segment(0xda, &[1, 1, 0, 0, 63, 0]));
    j.extend_from_slice(&[0x12, 0x34, 0xff, 0xd9]);
    j
}

fn depot_photo(etab: Uuid, eleve: Uuid, acteur: Uuid, octets: Vec<u8>) -> ProposerPhoto {
    ProposerPhoto {
        etablissement_id: etab,
        eleve_id: eleve,
        acteur,
        content_type: "image/jpeg".into(),
        octets,
        prise_le: None,
        valider_aussitot: true,
        photographe: false,
    }
}

#[tokio::test]
async fn f2_la_photo_stockee_n_a_ni_gps_ni_commentaire_et_garde_son_orientation() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let e = eleve(&ent, etab, "Exif").await;
    let original = jpeg_exif_gps();
    let ph = ent
        .proposer_photo_a(radm, depot_photo(etab, e.id, adm, original.clone()), t0())
        .await
        .unwrap();
    let (cle, taille): (String, i32) =
        sqlx::query_as("SELECT object_key, taille_octets FROM ent_eleve_photo WHERE id = $1")
            .bind(ph.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    let stocke = ent.objet_get(&cle).await.unwrap();
    assert_eq!(
        usize::try_from(taille).unwrap(),
        stocke.len(),
        "la taille enregistrée est celle de l'objet nettoyé"
    );
    assert!(stocke.len() < original.len());
    let texte = String::from_utf8_lossy(&stocke);
    for interdit in ["GPS", "48.8566", "PhoneX", "commentaire"] {
        assert!(
            !texte.contains(interdit),
            "{interdit} ne doit pas rester dans la photo servie"
        );
    }
    assert!(
        stocke.windows(4).any(|w| w == b"Exif"),
        "l'orientation (6) est conservée, seule"
    );
    // Servie à un personnel : les mêmes octets nettoyés.
    let (dir, rdir) = user(&cast, Role::Direction);
    let servi = ent
        .get_photo_eleve_bytes_for_actor(etab, dir, rdir, e.id)
        .await
        .unwrap()
        .expect("photo");
    assert!(!String::from_utf8_lossy(&servi).contains("GPS"));
    // Une image illisible est refusée (pas de stockage d'un contenu qu'on ne sait pas nettoyer).
    let r = ent
        .proposer_photo_a(
            radm,
            depot_photo(
                etab,
                e.id,
                adm,
                vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1],
            ),
            plus(1),
        )
        .await;
    assert!(r.is_err(), "{r:?}");
}

#[tokio::test]
async fn f2_une_photo_dont_la_transaction_echoue_ne_laisse_pas_d_objet_orphelin() {
    let Some((mut ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let (adm, radm) = user(&cast, Role::Administratif);
    let memoire = Arc::new(MemoryProvider::new());
    ent.installer_objets(ObjectStore::with_provider(memoire.clone()));
    let e = eleve(&ent, etab, "Orphelin").await;
    // Contrôle positif : une photo qui réussit laisse bien UN objet.
    ent.proposer_photo_a(radm, depot_photo(etab, e.id, adm, jpeg_exif_gps()), t0())
        .await
        .unwrap();
    assert_eq!(memoire.len(), 1);

    // Panne simulée APRÈS l'écriture de l'objet : un déclencheur refuse l'insertion pour CET élève.
    let (e2, nom) = (
        eleve(&ent, etab, "Panne").await,
        format!("ent_test_f2_{}", lettres(5)),
    );
    sqlx::query(&format!(
        "CREATE FUNCTION {nom}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'panne simulée'; END $$"
    ))
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(&format!(
        "CREATE TRIGGER {nom} BEFORE INSERT ON ent_eleve_photo FOR EACH ROW WHEN (NEW.eleve_id = '{}') EXECUTE FUNCTION {nom}()",
        e2.id
    ))
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let r = ent
        .proposer_photo_a(
            radm,
            depot_photo(etab, e2.id, adm, jpeg_exif_gps()),
            plus(1),
        )
        .await;
    assert!(r.is_err(), "la panne doit faire échouer la proposition");
    assert_eq!(
        memoire.len(),
        1,
        "l'objet écrit avant l'échec est retiré du stockage"
    );
    let (registre,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_objet WHERE etablissement_id = $1 AND cle_objet LIKE $2 AND supprime_le IS NULL")
        .bind(etab)
        .bind(format!("{etab}/trombi/{}/%", e2.id))
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(registre, 0, "et sa ligne du registre des objets");
    sqlx::query(&format!("DROP TRIGGER {nom} ON ent_eleve_photo"))
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(&format!("DROP FUNCTION {nom}()"))
        .execute(ent.pool_verif())
        .await
        .unwrap();
    // Sans panne, la même élève reçoit sa photo et l'objet est conservé.
    ent.proposer_photo_a(
        radm,
        depot_photo(etab, e2.id, adm, jpeg_exif_gps()),
        plus(2),
    )
    .await
    .unwrap();
    assert_eq!(memoire.len(), 2);
}

// ───────────────────────── F3 et F5 : statut jamais NULL, contraintes validées ─────────────────────────

#[tokio::test]
async fn f3_le_statut_n_est_plus_jamais_null() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    // Insertion brute sans statut (ancien chemin) : la base le dérive.
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_eleve (id, etablissement_id, nom, prenom, status) VALUES ($1, $2, 'Brut', 'Statut', 'preinscrit')")
        .bind(id)
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let (s,): (Option<String>,) = sqlx::query_as("SELECT statut FROM ent_eleve WHERE id = $1")
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(s.is_some(), "statut dérivé à l'insertion");
    // Tenter de l'effacer ne le rend pas NULL (la valeur précédente est gardée).
    sqlx::query("UPDATE ent_eleve SET statut = NULL WHERE id = $1")
        .bind(id)
        .execute(ent.pool_verif())
        .await
        .ok();
    let (s2,): (Option<String>,) = sqlx::query_as("SELECT statut FROM ent_eleve WHERE id = $1")
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(s2, s);
    // Aucune ligne n'est restée NULL (reprise de la migration, déclencheur ensuite).
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve WHERE statut IS NULL")
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 0);
    // Une valeur hors vocabulaire reste refusée.
    assert!(
        sqlx::query("UPDATE ent_eleve SET statut = 'inconnu' WHERE id = $1")
            .bind(id)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn f5_les_contraintes_de_298_et_du_statut_sont_validees() {
    let Some((ent, _cast)) = labo().await else {
        return;
    };
    let lignes: Vec<(String, bool)> = sqlx::query_as(
        "SELECT conname, convalidated FROM pg_constraint
          WHERE conname IN ('ent_eleve_confidentialite_regles_chk', 'ent_eleve_statut_chk') ORDER BY conname",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(lignes.len(), 2, "{lignes:?}");
    assert!(
        lignes.iter().all(|(_, v)| *v),
        "contraintes encore NOT VALID : {lignes:?}"
    );
    // Aucune contrainte posée par 298 n'est restée NOT VALID.
    let (reste,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM pg_constraint c JOIN pg_class t ON t.oid = c.conrelid
          WHERE NOT c.convalidated AND t.relname IN
            ('ent_eleve_confidentialite', 'ent_eleve_photo', 'ent_eleve_photo_refus', 'ent_eleve_coordonnee',
             'ent_eleve_coordonnee_verification', 'ent_eleve_identite_scellee')",
    )
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(reste, 0);
}

// ───────────────────────── F4 : la liste des protégés est tracée ─────────────────────────

#[tokio::test]
async fn f4_lire_la_liste_des_eleves_proteges_est_tracee() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let (dir, rdir) = user(&cast, Role::Direction);
    let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND domaine = 'consultation' AND action = 'liste_proteges'")
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    ent.eleves_proteges_pour_acteur(rdir, etab, dir, 50)
        .await
        .unwrap();
    let apres: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND domaine = 'consultation' AND action = 'liste_proteges'")
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(apres, avant + 1);
    // Aucun nom d'élève dans la trace ; l'auteur est celui qui a lu.
    let (acteur, libelle, payload): (Option<Uuid>, String, String) = sqlx::query_as(
        "SELECT actor_user_id, libelle, payload::text FROM ent_journal_modif
          WHERE etablissement_id = $1 AND action = 'liste_proteges' ORDER BY created_at DESC LIMIT 1",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(acteur, Some(dir));
    assert!(!libelle.is_empty() && payload.len() < 200);
    // Un refus n'est pas tracé comme une lecture (aucune donnée n'a été servie).
    let (prof, rprof) = user(&cast, Role::Prof);
    assert!(forbidden(
        &ent.eleves_proteges_pour_acteur(rprof, etab, prof, 50).await
    ));
    let (cpe, rcpe) = user(&cast, Role::Cpe);
    assert!(forbidden(
        &ent.eleves_proteges_pour_acteur(rcpe, etab, cpe, 50).await
    ));
    let encore: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND domaine = 'consultation' AND action = 'liste_proteges'")
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(encore, apres);
}

// ───────────────────────── isolation par établissement ─────────────────────────

#[tokio::test]
async fn e9_isolation_par_etablissement_des_pastilles_et_des_reglages() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let autre = Uuid::new_v4();
    let (dir, rdir) = user(&cast, Role::Direction);
    let (cpe, rcpe) = user(&cast, Role::Cpe);
    // Un élève d'un autre établissement : fiche, pastilles, listes — absents.
    let e = eleve(&ent, autre, "Ailleurs").await;
    ent.reconstruire_indicateurs(autre, None, None)
        .await
        .unwrap();
    assert!(absent(
        &ent.synthese_eleve_pour_acteur(rdir, etab, dir, e.id).await
    ));
    assert!(absent(
        &ent.pastilles_eleve_pour_acteur(rcpe, etab, cpe, e.id).await
    ));
    for code in [
        "protocole",
        "nouvel_arrivant",
        "arrive_en_cours_annee",
        "majorite_proche",
        "modalite_particuliere",
    ] {
        let ids = ent
            .eleves_par_pastille_pour_acteur(rdir, etab, dir, code, 5000)
            .await;
        if let Ok(ids) = ids {
            assert!(
                !ids.contains(&e.id),
                "{code} : élève d'un autre établissement dans la liste"
            );
        }
    }
    // Le recalcul d'un établissement ne touche pas l'autre : la projection de `e` reste à `autre`.
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_eleve_indicateur WHERE eleve_id = $1 AND etablissement_id <> $2",
    )
    .bind(e.id)
    .bind(autre)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 0);
}

// ───────────────────────── E9a : la liste des élèves ─────────────────────────

fn filtre<'a>(q: &'a str) -> crate::FiltreListeEleves<'a> {
    crate::FiltreListeEleves {
        q,
        limite: 50,
        ..Default::default()
    }
}

#[tokio::test]
async fn e9a_liste_filtre_pagine_et_respecte_les_droits() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let (dir, rdir) = user(&cast, Role::Direction);
    let (cpe, rcpe) = user(&cast, Role::Cpe);
    let (prof, rprof) = user(&cast, Role::Prof);
    let marque = format!("Liste{}", lettres(5));
    // Quatre élèves au nom reconnaissable, dans une classe à eux, dont un protégé.
    let mut ids = Vec::new();
    for (i, prenom) in ["Ada", "Bruno", "Chloé", "Dario"].iter().enumerate() {
        let e = ent
            .create_eleve(CreateEleve {
                etablissement_id: etab,
                nom: format!("{marque}{i}"),
                prenom: (*prenom).into(),
                ine: None,
                sexe: Some("F".into()),
                date_naissance: Some(NaiveDate::from_ymd_opt(2012, 3, 14).unwrap()),
                numero_badge: None,
                redoublant: false,
                classe: Some("E9Z".into()),
                regime: None,
                mef: None,
                site: None,
                date_entree: None,
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .await
            .unwrap();
        ids.push(e.id);
    }
    // Page, total, ordre alphabétique, classes proposées.
    let tout = ent
        .lister_eleves_pour_acteur(rdir, etab, dir, &filtre(&marque))
        .await
        .unwrap();
    assert_eq!(tout.total, 4);
    assert_eq!(tout.eleves.len(), 4);
    assert!(tout.portee_etendue && !tout.tronque);
    assert_eq!(tout.classes, ["E9Z"]);
    let noms: Vec<_> = tout.eleves.iter().map(|v| v.eleve.nom.clone()).collect();
    let mut tries = noms.clone();
    tries.sort();
    assert_eq!(noms, tries, "ordre alphabétique conservé");
    let p1 = ent
        .lister_eleves_pour_acteur(
            rdir,
            etab,
            dir,
            &crate::FiltreListeEleves {
                limite: 3,
                ..filtre(&marque)
            },
        )
        .await
        .unwrap();
    let p2 = ent
        .lister_eleves_pour_acteur(
            rdir,
            etab,
            dir,
            &crate::FiltreListeEleves {
                limite: 3,
                decalage: 3,
                ..filtre(&marque)
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (p1.eleves.len(), p1.total, p2.eleves.len(), p2.total),
        (3, 4, 1, 4)
    );
    let hors = ent
        .lister_eleves_pour_acteur(
            rdir,
            etab,
            dir,
            &crate::FiltreListeEleves {
                decalage: 99,
                ..filtre(&marque)
            },
        )
        .await
        .unwrap();
    assert!(
        hors.eleves.is_empty() && hors.total == 4,
        "page au-delà de la fin : vide, total juste"
    );
    // Filtres : classe (insensible à la casse), statut, valeur inconnue refusée.
    let c = ent
        .lister_eleves_pour_acteur(
            rdir,
            etab,
            dir,
            &crate::FiltreListeEleves {
                classe: Some("e9z"),
                ..filtre(&marque)
            },
        )
        .await
        .unwrap();
    assert_eq!(c.total, 4);
    let autre = ent
        .lister_eleves_pour_acteur(
            rdir,
            etab,
            dir,
            &crate::FiltreListeEleves {
                classe: Some("ZZZ"),
                ..filtre(&marque)
            },
        )
        .await
        .unwrap();
    assert_eq!(autre.total, 0);
    assert_eq!(
        autre.classes,
        ["E9Z"],
        "le sélecteur garde les classes des résultats même quand le filtre n'en laisse aucun"
    );
    let sortis = ent
        .lister_eleves_pour_acteur(
            rdir,
            etab,
            dir,
            &crate::FiltreListeEleves {
                statut: Some("sorti"),
                ..filtre(&marque)
            },
        )
        .await
        .unwrap();
    assert_eq!(sortis.total, 0);
    assert!(matches!(
        ent.lister_eleves_pour_acteur(
            rdir,
            etab,
            dir,
            &crate::FiltreListeEleves {
                statut: Some("n'importe quoi"),
                ..filtre(&marque)
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Recherche tolérante aux accents et aux fautes (E6) : « chloe » retrouve Chloé.
    let chloe = ent
        .lister_eleves_pour_acteur(rdir, etab, dir, &filtre(&format!("chloe {marque}")))
        .await
        .unwrap();
    assert_eq!(chloe.total, 1);
    assert_eq!(chloe.eleves[0].eleve.prenom, "Chloé");

    // Le professeur ne voit que ses classes : périmètre, jamais ces élèves (autre classe).
    let p = ent
        .lister_eleves_pour_acteur(rprof, etab, prof, &filtre(&marque))
        .await
        .unwrap();
    assert_eq!(
        p.total, 0,
        "le professeur ne voit pas les élèves d'une classe qui n'est pas la sienne"
    );
    assert!(
        !p.portee_etendue,
        "et la liste le dit : périmètre restreint"
    );
    // Un protégé disparaît pour un tiers (CPE) mais pas pour la direction (G6), avant la pagination.
    let (d2, r2) = user(&cast, Role::Direction);
    ent.poser_confidentialite_a(
        r2,
        PoserConfidentialite {
            etablissement_id: etab,
            eleve_id: ids[1],
            motif: "Mesure de protection".into(),
            mesures: vec![],
            du: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            au: None,
            acteur: d2,
        },
        t0(),
    )
    .await
    .unwrap();
    let cpe_liste = ent
        .lister_eleves_pour_acteur(
            rcpe,
            etab,
            cpe,
            &crate::FiltreListeEleves {
                limite: 3,
                ..filtre(&marque)
            },
        )
        .await
        .unwrap();
    assert_eq!(
        cpe_liste.total, 3,
        "le protégé n'est ni listé ni compté pour un tiers"
    );
    assert_eq!(
        cpe_liste.eleves.len(),
        3,
        "la page est remplie par les autres, sans trou"
    );
    assert!(cpe_liste.eleves.iter().all(|v| v.eleve.id != ids[1]));
    assert_eq!(
        ent.lister_eleves_pour_acteur(rdir, etab, dir, &filtre(&marque))
            .await
            .unwrap()
            .total,
        4
    );

    // Liste de travail par pastille : droits de la pastille exigés ; un autre établissement n'y paraît pas.
    let jour = ent.jour_etablissement(etab).await.unwrap();
    ent.reconstruire_indicateurs(etab, None, Some(jour))
        .await
        .unwrap();
    let sans_ine = ent
        .lister_eleves_pour_acteur(
            rdir,
            etab,
            dir,
            &crate::FiltreListeEleves {
                pastille: Some("sans_ine"),
                ..filtre(&marque)
            },
        )
        .await
        .unwrap();
    assert_eq!(sans_ine.total, 4, "aucun de ces élèves n'a d'INE");
    assert!(forbidden(
        &ent.lister_eleves_pour_acteur(
            rcpe,
            etab,
            cpe,
            &crate::FiltreListeEleves {
                pastille: Some("sans_ine"),
                ..filtre(&marque)
            }
        )
        .await
    ));
    assert!(matches!(
        ent.lister_eleves_pour_acteur(
            rdir,
            etab,
            dir,
            &crate::FiltreListeEleves {
                pastille: Some("inconnue"),
                ..filtre(&marque)
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let ailleurs = eleve(&ent, Uuid::new_v4(), "Ailleurs").await;
    let l = ent
        .lister_eleves_pour_acteur(rdir, etab, dir, &filtre("Ailleurs"))
        .await
        .unwrap();
    assert!(
        l.eleves.iter().all(|v| v.eleve.id != ailleurs.id),
        "isolation par établissement"
    );
    // Les comptes famille n'ont pas de liste.
    let parent = cast.parent_lea.user_id;
    assert!(forbidden(
        &ent.lister_eleves_pour_acteur(Role::Parent.as_str(), etab, parent, &filtre(""))
            .await
    ));
}

// ───────────────────────── enseignants : pastilles d'organisation, jamais de santé ─────────────────────────

/// Décision du pilote : l'enseignant de l'élève (et son professeur principal) voit `protocole`,
/// `nouvel_arrivant`, `arrive_en_cours_annee` et `modalite_particuliere` (booléens, sans détail) ; jamais
/// `majorite_proche` ni le PAI ; un enseignant sans lien avec l'élève ne voit rien (fiche absente) ; un
/// élève protégé reste protégé ; la règle « une pastille suit sa tuile » reste unique.
#[tokio::test]
async fn e9d_l_enseignant_de_l_eleve_voit_les_pastilles_d_organisation_et_pas_les_autres() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    ent.activer_module_sante_test(etab).await;
    let (prof, rprof) = user(&cast, Role::Prof);
    let (dir, rdir) = user(&cast, Role::Direction);
    let harry = cast.eleve_lea_id; // élève de la classe du professeur du labo
    let jour = ent.jour_etablissement(etab).await.unwrap();
    let codes_prof = ["protocole", "nouvel_arrivant", "arrive_en_cours_annee", "modalite_particuliere", "majorite_proche", "pai_actif"];
    for c in codes_prof {
        sqlx::query(
            "INSERT INTO ent_eleve_indicateur (etablissement_id, eleve_id, code, du) VALUES ($1, $2, $3, $4)
             ON CONFLICT (etablissement_id, eleve_id, code) DO NOTHING",
        )
        .bind(etab)
        .bind(harry)
        .bind(c)
        .bind(jour)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    }
    let v = codes_vus_par(&ent, etab, rprof, prof, harry).await.expect("le professeur de la classe voit la fiche");
    for attendu in ["protocole", "nouvel_arrivant", "arrive_en_cours_annee", "modalite_particuliere"] {
        assert!(v.contains(&attendu), "l'enseignant doit voir {attendu} : {v:?}");
    }
    assert!(!v.contains(&"majorite_proche"), "la majorité reste à l'administration et à la vie scolaire : {v:?}");
    assert!(!v.contains(&"pai_actif"), "le PAI reste réservé à sante.lire : {v:?}");
    // Aucune valeur ni date dans ce qu'il reçoit.
    let s = ent.synthese_eleve_pour_acteur(rprof, etab, prof, harry).await.unwrap();
    assert!(s.pastilles.iter().filter(|p| p.domaine == "classe").all(|p| p.valeur_code.is_none() && p.valeur_date.is_none()));
    // Règle unique : chaque pastille visible a sa tuile ouverte.
    for p in &s.pastilles {
        let t = s.tuiles.iter().find(|t| t.domaine == p.domaine).unwrap();
        assert!(matches!(t.etat, crate::eleve_indicateurs::EtatTuile::Visible), "{}", p.code);
    }
    // Sans module santé, le protocole disparaît aussi pour l'enseignant (M6).
    let (admin, radmin) = user(&cast, Role::Admin);
    ent.definir_module_sante(etab, admin, radmin, false).await.unwrap();
    let v = codes_vus_par(&ent, etab, rprof, prof, harry).await.unwrap();
    assert!(!v.contains(&"protocole"), "{v:?}");
    ent.definir_module_sante(etab, admin, radmin, true).await.unwrap();

    // Un enseignant sans lien avec l'élève : la fiche n'existe pas pour lui, donc aucune pastille.
    let sans_lien = eleve(&ent, etab, "SansLien").await;
    ent.reconstruire_indicateurs(etab, None, Some(jour)).await.unwrap();
    assert!(absent(&ent.synthese_eleve_pour_acteur(rprof, etab, prof, sans_lien.id).await));
    let l = ent.eleves_par_pastille_pour_acteur(rprof, etab, prof, "protocole", 100).await;
    assert!(forbidden(&l) || l.is_ok_and(|v| !v.contains(&sans_lien.id)), "pas de liste de travail pour un enseignant");

    // Élève protégé : la vie scolaire (tiers) ne le voit plus ; l'enseignant de sa classe garde la fiche
    // (portée pédagogique, décision E4) mais n'y voit ni la pastille ni la tuile de protection.
    proteger_harry(&ent, &cast, rdir, dir, harry, jour).await;
    let (cpe, rcpe) = user(&cast, Role::Cpe);
    assert!(absent(&ent.synthese_eleve_pour_acteur(rcpe, etab, cpe, harry).await), "un protégé reste protégé d'un tiers");
    let sp = ent.synthese_eleve_pour_acteur(rprof, etab, prof, harry).await.unwrap();
    assert!(sp.pastilles.iter().all(|p| p.code != "confidentialite_renforcee"));
    assert!(sp.tuiles.iter().all(|t| t.domaine != "protection" || !matches!(t.etat, crate::eleve_indicateurs::EtatTuile::Visible)));
    ent.lever_confidentialite_a(rdir, etab, dir, harry, "fin_de_mesure", t0()).await.unwrap();
    for c in codes_prof {
        sqlx::query("DELETE FROM ent_eleve_indicateur WHERE eleve_id = $1 AND code = $2")
            .bind(harry)
            .bind(c)
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
}

async fn proteger_harry(ent: &PgEnt, cast: &LaboCast, rdir: &str, dir: Uuid, eleve: Uuid, jour: NaiveDate) {
    ent.poser_confidentialite_a(
        rdir,
        PoserConfidentialite {
            etablissement_id: cast.etablissement_id,
            eleve_id: eleve,
            motif: "Mesure de protection".into(),
            mesures: vec![],
            du: Some(jour - Duration::days(1)),
            au: None,
            acteur: dir,
        },
        t0(),
    )
    .await
    .unwrap();
}

async fn codes_vus_par(ent: &PgEnt, etab: Uuid, role: &str, u: Uuid, e: Uuid) -> Result<Vec<&'static str>, EntDbError> {
    ent.synthese_eleve_pour_acteur(role, etab, u, e).await.map(|s| s.pastilles.iter().map(|p| p.code).collect())
}
