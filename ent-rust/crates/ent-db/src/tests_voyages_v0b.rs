//! Socle 42, lot V0b (budget, prix annoncé figé, PDF, rappels, synthèse APEL, avis du socle 8) contre
//! la base locale de test. Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! * saisie stricte en centimes, calcul de la part, motifs obligatoires, droits par profil ;
//! * version validée **figée** (par la base elle-même), révision motivée n+1, version lue ;
//! * budget équilibré pour être validé, causes du déficit ;
//! * prix annoncé figé à l'autorisation, baisse suivie, hausse sur décision de la direction ;
//! * PDF côté serveur (budget, circulaire), objets relus avec leur empreinte, droits ;
//! * synthèse APEL (états retenus, comptabilité seule), rappels idempotents, avis aux bons profils.

use super::*;
use crate::outbox::ConfigDistribution;
use crate::tests_voyages_v0a::{
    actions_journal, est_conflit_detail, est_forbidden, est_introuvable, est_refus, Cadre,
};
use crate::voyages::Commande;
use crate::voyages_budget_repo::{BudgetVue, LigneSaisie, ParamsBudget};
use crate::voyages_repo::{EtapeSaisie, ParamsCommande, SortieLigne};
use chrono::{DateTime, Duration, Utc};
use ent_auth::Role;
use std::time::Duration as Delai;
use uuid::Uuid;

fn ligne(sens: &str, poste: &str, libelle: &str, montant: &str) -> LigneSaisie {
    LigneSaisie {
        sens: sens.into(),
        poste: poste.into(),
        libelle: libelle.into(),
        montant: montant.into(),
        motif: Some("Devis de l'agence".into()),
        affectation: None,
    }
}

impl Cadre {
    /// Voyage soumis, validé pédagogiquement, passé aux finances : étape « budget à valider ».
    async fn voyage_aux_finances(&self, titre: &str) -> SortieLigne {
        let s = self.creer(titre, "voyage").await;
        self.preparer_jusqu_a_soumise(s.id).await;
        self.cmd(self.dir, Role::Direction, s.id, Commande::ValiderPedagogie)
            .await
            .unwrap();
        self.cmd(self.prof, Role::Prof, s.id, Commande::PasserAuxFinances)
            .await
            .unwrap()
    }

    async fn budget(&self, user: Uuid, role: Role, id: Uuid) -> Result<BudgetVue, EntDbError> {
        self.ent.voyage_budget(&self.a(user, role), id).await
    }

    /// Ajoute une ligne en tant que comptabilité au brouillon courant (`rev` relu à chaque fois).
    async fn ajouter(&self, id: Uuid, l: LigneSaisie) -> BudgetVue {
        let b = self.budget(self.compta, Role::Compta, id).await.unwrap();
        let b = b.brouillon.expect("brouillon");
        self.ent
            .voyage_budget_ligne_ajouter(&self.a(self.compta, Role::Compta), id, b.numero, b.rev, l)
            .await
            .expect("ligne ajoutée")
    }

    /// Voyage jusqu'à `ouverte` avec un budget validé (car 1 500 €, 3 payants : part 500 €).
    async fn voyage_ouvert(&self, titre: &str) -> SortieLigne {
        let s = self.voyage_aux_finances(titre).await;
        self.budget_minimal(s.id).await;
        self.cmd(self.compta, Role::Compta, s.id, Commande::ValiderBudget)
            .await
            .unwrap();
        self.cmd(self.dir, Role::Direction, s.id, Commande::Autoriser)
            .await
            .unwrap()
    }

    /// Révision (version n+1) : un nouveau car à `montant`, puis validation par la comptabilité.
    async fn reviser(&self, id: Uuid, montant: &str) -> Result<BudgetVue, EntDbError> {
        let compta = self.a(self.compta, Role::Compta);
        let v = self
            .ent
            .voyage_budget_version_creer(&compta, id, Some("Nouveau devis"))
            .await?;
        let b = v.brouillon.expect("brouillon de révision");
        let l = b.lignes[0].id;
        let v = self
            .ent
            .voyage_budget_ligne_modifier(
                &compta,
                id,
                b.numero,
                b.rev,
                l,
                ligne("depense", "transport", "Car", montant),
            )
            .await?;
        let b = v.brouillon.expect("brouillon");
        self.ent
            .voyage_budget_valider_revision(&compta, id, b.numero, b.rev)
            .await
    }
}

fn config(etab: Uuid) -> ConfigDistribution {
    ConfigDistribution {
        taille_lot: 100,
        max_tentatives: 3,
        delai_base: Delai::ZERO,
        delai_max: Delai::ZERO,
        intervalle: Delai::from_millis(20),
        etablissement: Some(etab),
    }
}

async fn notifs(c: &Cadre, user: Uuid, kind: &str) -> usize {
    c.ent
        .mes_notifications(c.etab, user, 200)
        .await
        .unwrap()
        .iter()
        .filter(|n| n.kind == kind)
        .count()
}

async fn passage(c: &Cadre, depart: DateTime<Utc>, jours_avant: i64) -> usize {
    c.ent
        .voyages_rappels_traiter(c.etab, depart - Duration::days(jours_avant))
        .await
        .unwrap()
}

/// Saisie refusée : refus métier, ou valeur invalide détectée à la lecture du texte.
fn est_rejet(e: &EntDbError) -> bool {
    est_refus(e) || matches!(e, EntDbError::Domain(domain_eleve::DomainError::ValeurInvalide))
}

fn code_sql(r: &Result<sqlx::postgres::PgQueryResult, sqlx::Error>) -> Option<String> {
    match r {
        Err(sqlx::Error::Database(d)) => d.code().map(|x| x.to_string()),
        _ => None,
    }
}

// ─────────────────────────── Saisie et calcul ───────────────────────────

#[tokio::test]
async fn budget_en_centimes_calcule_la_part_et_refuse_les_saisies_ambigues() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.voyage_aux_finances("Voyage à Rome").await;
    let compta = c.a(c.compta, Role::Compta);
    // Aucune version : la vue le dit, le calcul n'existe pas encore.
    let b = c.budget(c.compta, Role::Compta, s.id).await.unwrap();
    assert!(b.a_budget && b.brouillon.is_none() && b.validee.is_none());
    assert!(b.peut_ecrire && b.peut_valider && !b.peut_decider_hausse);
    // La première version reprend l'effectif déclaré (3) et la marge préréglée du contexte (voyage en France : 7 %).
    let b = c
        .ent
        .voyage_budget_version_creer(&compta, s.id, None)
        .await
        .unwrap();
    let v = b.brouillon.as_ref().unwrap();
    assert_eq!((v.numero, v.nb_payants, v.marge_pct, b.marge_prereglee), (1, 3, 7, 7));
    let v1 = c.ajouter(s.id, ligne("depense", "transport", "Car", "1 500")).await;
    let v2 = c
        .ajouter(s.id, ligne("depense", "hebergement", "Auberge", "1 200,50"))
        .await;
    assert!(v2.brouillon.as_ref().unwrap().rev > v1.brouillon.as_ref().unwrap().rev);
    let k = &v2.brouillon.unwrap().calcul;
    assert_eq!(k.depenses_total, 270_050, "centimes entiers");
    // Marge 7 % de 2 700,50 € = 189,04 € (au centime supérieur) ; plafond : coût réel 900,16 € par élève ;
    // part fixe arrondie à l'EURO INFÉRIEUR : 900 €. Les 0,50 € d'écart sont à la charge de l'établissement.
    assert_eq!(k.marge, 18_904);
    assert_eq!(k.part_familles, Some(90_000));
    assert_eq!(k.total_familles, 270_000, "somme exacte : part × payants");
    assert_eq!((k.deficit, k.arrondi_pris_en_charge), (50, 50));
    assert!(k.equilibre);

    // Format français strict : « 1.350 » est refusé, pas deviné (R-SV16).
    let b = c.budget(c.compta, Role::Compta, s.id).await.unwrap().brouillon.unwrap();
    for faux in ["1.350", "1,350", "12 €", "-5", "abc", "", "1 35"] {
        let e = c
            .ent
            .voyage_budget_ligne_ajouter(&compta, s.id, b.numero, b.rev, ligne("depense", "divers", "x", faux))
            .await
            .unwrap_err();
        assert!(est_refus(&e), "« {faux} » doit être refusé : {e:?}");
    }
    // Motif obligatoire (source « saisie motivée »), poste cohérent avec le sens, libellé non vide,
    // don affecté avec son affectation, affectation réservée aux recettes.
    let mut sans_motif = ligne("depense", "divers", "x", "10");
    sans_motif.motif = None;
    let mut don = ligne("recette", "don_affecte", "Mécène", "100");
    don.affectation = None;
    let mut aff_depense = ligne("depense", "divers", "x", "10");
    aff_depense.affectation = Some("general".into());
    let mut aff_inconnue = ligne("recette", "apel", "x", "10");
    aff_inconnue.affectation = Some("autre".into());
    for l in [
        sans_motif,
        don,
        aff_depense,
        aff_inconnue,
        ligne("depense", "apel", "poste de recette", "10"),
        ligne("recette", "transport", "poste de dépense", "10"),
        ligne("autre", "divers", "x", "10"),
        ligne("depense", "divers", "   ", "10"),
        ligne("depense", "divers", &"x".repeat(201), "10"),
    ] {
        let e = c
            .ent
            .voyage_budget_ligne_ajouter(&compta, s.id, b.numero, b.rev, l)
            .await
            .unwrap_err();
        assert!(est_rejet(&e), "{e:?}");
    }
    // Version d'écritures périmée : conflit (« rechargez »).
    let e = c
        .ent
        .voyage_budget_ligne_ajouter(&compta, s.id, b.numero, b.rev - 1, ligne("depense", "divers", "x", "10"))
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    // Le journal garde la trace de chaque écriture, avec le motif.
    let j = actions_journal(&c, s.id).await;
    assert!(j.iter().filter(|a| *a == "budget_ligne_ajoutee").count() == 2, "{j:?}");
    assert!(j.contains(&"budget_version_creee".to_string()));
}

#[tokio::test]
async fn lignes_modifiables_supprimables_et_limitees_dans_le_brouillon() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.voyage_aux_finances("Lignes").await;
    let compta = c.a(c.compta, Role::Compta);
    c.ent.voyage_budget_version_creer(&compta, s.id, None).await.unwrap();
    let v = c.ajouter(s.id, ligne("depense", "transport", "Car", "800")).await;
    let b = v.brouillon.unwrap();
    let lid = b.lignes[0].id;
    // Modifier : le sens ne change pas (une dépense ne devient pas une recette).
    let e = c
        .ent
        .voyage_budget_ligne_modifier(&compta, s.id, b.numero, b.rev, lid, ligne("recette", "apel", "x", "10"))
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    let v = c
        .ent
        .voyage_budget_ligne_modifier(&compta, s.id, b.numero, b.rev, lid, ligne("depense", "transport", "Car XL", "900"))
        .await
        .unwrap();
    let b = v.brouillon.unwrap();
    assert_eq!(b.lignes[0].libelle, "Car XL");
    assert_eq!(b.lignes[0].montant_centimes, 90_000);
    // Une ligne inconnue ou d'une autre version : introuvable.
    let e = c
        .ent
        .voyage_budget_ligne_supprimer(&compta, s.id, b.numero, b.rev, Uuid::new_v4())
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    let v = c
        .ent
        .voyage_budget_ligne_supprimer(&compta, s.id, b.numero, b.rev, lid)
        .await
        .unwrap();
    assert!(v.brouillon.unwrap().lignes.is_empty());
    let j = actions_journal(&c, s.id).await;
    assert!(j.contains(&"budget_ligne_modifiee".to_string()) && j.contains(&"budget_ligne_supprimee".to_string()));
    // Cent lignes au plus par version.
    for i in 0..100 {
        c.ajouter(s.id, ligne("depense", "divers", &format!("Frais {i}"), "1")).await;
    }
    let b = c.budget(c.compta, Role::Compta, s.id).await.unwrap().brouillon.unwrap();
    let e = c
        .ent
        .voyage_budget_ligne_ajouter(&compta, s.id, b.numero, b.rev, ligne("depense", "divers", "de trop", "1"))
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    // Abandonner le brouillon : la sortie n'a plus de version.
    let b = c.budget(c.compta, Role::Compta, s.id).await.unwrap().brouillon.unwrap();
    let v = c
        .ent
        .voyage_budget_version_abandonner(&compta, s.id, b.numero, b.rev)
        .await
        .unwrap();
    assert!(v.brouillon.is_none() && v.historique.is_empty());
}

#[tokio::test]
async fn marge_modifiee_et_effectif_hors_limites() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.voyage_aux_finances("Marge").await;
    let compta = c.a(c.compta, Role::Compta);
    c.ent.voyage_budget_version_creer(&compta, s.id, None).await.unwrap();
    c.ajouter(s.id, ligne("depense", "transport", "Car", "1 500")).await;
    let b = c.budget(c.compta, Role::Compta, s.id).await.unwrap().brouillon.unwrap();
    let param = |nb: Option<i32>, marge: Option<i32>, motif: Option<&str>| ParamsBudget {
        nb_payants: nb,
        marge_pct: marge,
        marge_motif: motif.map(String::from),
    };
    // Une marge différente du préréglage (7 %) demande un motif ; hors 0–30 % ou effectif > 2000 : refus.
    for (p, nom) in [
        (param(None, Some(10), None), "marge sans motif"),
        (param(None, Some(31), Some("x")), "marge > 30"),
        (param(None, Some(-1), Some("x")), "marge < 0"),
        (param(Some(2001), None, None), "effectif > 2000"),
        (param(Some(-1), None, None), "effectif < 0"),
    ] {
        let e = c
            .ent
            .voyage_budget_parametrer(&compta, s.id, b.numero, b.rev, p)
            .await
            .unwrap_err();
        assert!(est_refus(&e), "{nom} : {e:?}");
    }
    let v = c
        .ent
        .voyage_budget_parametrer(&compta, s.id, b.numero, b.rev, param(Some(4), Some(10), Some("Étranger proche")))
        .await
        .unwrap();
    let b = v.brouillon.unwrap();
    assert_eq!((b.nb_payants, b.marge_pct), (4, 10));
    assert_eq!(b.marge_motif.as_deref(), Some("Étranger proche"));
    // Le journal garde l'ancien et le nouveau, champ par champ, avec le motif de la marge.
    let (avant, apres, motif): (Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT avant, apres, motif FROM ent_sortie_journal
          WHERE sortie_id = $1 AND action = 'budget_parametre' AND champ = 'marge_pct'",
    )
    .bind(s.id)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!((avant.as_deref(), apres.as_deref(), motif.as_deref()), (Some("7"), Some("10"), Some("Étranger proche")));
    // Revenir au préréglage efface le motif ; modifier l'effectif seul garde une marge motivée.
    let v = c
        .ent
        .voyage_budget_parametrer(&compta, s.id, b.numero, b.rev, param(None, Some(7), None))
        .await
        .unwrap();
    let b = v.brouillon.unwrap();
    assert_eq!((b.marge_pct, b.marge_motif), (7, None));
    // Version périmée.
    let e = c
        .ent
        .voyage_budget_parametrer(&compta, s.id, b.numero, b.rev - 1, param(Some(5), None, None))
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
}

// ─────────────────────────── Droits ───────────────────────────

#[tokio::test]
async fn droits_sur_le_budget_par_profil_et_par_etape() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Droits", "voyage").await;
    let prof = c.a(c.prof, Role::Prof);
    // Brouillon : le porteur prépare un budget prévisionnel ; la comptabilité aussi ; le secrétariat aussi.
    let b = c.ent.voyage_budget_version_creer(&prof, s.id, None).await.unwrap();
    assert!(b.peut_ecrire && !b.peut_valider);
    let e = c.ent.voyage_budget_version_creer(&c.a(c.sec, Role::Administratif), s.id, None).await.unwrap_err();
    assert!(est_refus(&e), "un brouillon est déjà en cours : {e:?}");
    // Lecture : porteur, comptabilité, direction, secrétariat. Pas l'accueil ni la famille ni un professeur tiers.
    for (u, r) in [(c.prof, Role::Prof), (c.compta, Role::Compta), (c.dir, Role::Direction), (c.sec, Role::Administratif)] {
        assert!(c.budget(u, r, s.id).await.is_ok(), "{r:?}");
    }
    for (u, r) in [(c.accueil, Role::Accueil), (c.parent, Role::Parent), (c.prof2, Role::Prof)] {
        let e = c.budget(u, r, s.id).await.unwrap_err();
        assert!(est_introuvable(&e), "{r:?} : {e:?}");
    }
    // Un encadrant (qui voit la sortie) n'a pas le budget : refusé, pas « introuvable ».
    c.preparer_jusqu_a_soumise(s.id).await; // prof2 devient encadrant
    let e = c.budget(c.prof2, Role::Prof, s.id).await.unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    let e = c
        .ent
        .voyage_budget_version_creer(&c.a(c.prof2, Role::Prof), s.id, None)
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    // Dossier soumis : le porteur ne modifie plus le budget (la comptabilité reste maîtresse du chiffre).
    let b = c.budget(c.prof, Role::Prof, s.id).await.unwrap();
    assert!(!b.peut_ecrire, "soumise : plus d'écriture pour le porteur");
    let v = b.brouillon.unwrap();
    let e = c
        .ent
        .voyage_budget_ligne_ajouter(&prof, s.id, v.numero, v.rev, ligne("depense", "divers", "x", "1"))
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    // La direction n'écrit pas le budget non plus (elle décide du prix, pas du chiffre).
    assert!(!c.budget(c.dir, Role::Direction, s.id).await.unwrap().peut_valider);
    // Autre établissement : introuvable, jamais « refusé ».
    let autre = Uuid::new_v4();
    let intrus = crate::tests_voyages_v0a::compte(&c.ent, autre, "compta").await;
    let e = c
        .ent
        .voyage_budget(&Acteur { etablissement_id: autre, user_id: intrus, role: "compta" }, s.id)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // Sortie sans voyage ni car : pas de budget à valider.
    let sn = c.creer("Journée", "sans_nuitee").await;
    let e = c
        .ent
        .voyage_budget_version_creer(&prof, sn.id, None)
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    assert!(!c.budget(c.prof, Role::Prof, sn.id).await.unwrap().a_budget);
}

// ─────────────────────────── Validation, figement, révision ───────────────────────────

#[tokio::test]
async fn validation_de_la_version_lue_figement_et_revision_motivee() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.voyage_aux_finances("Figement").await;
    let (numero, rev) = c.budget_minimal(s.id).await;
    let compta = c.a(c.compta, Role::Compta);
    let version = |c: &Cadre| {
        let id = s.id;
        let pool = c.ent.pool().clone();
        async move {
            sqlx::query_scalar::<_, i32>("SELECT version FROM ent_sortie WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap()
        }
    };
    let valider = |numero: Option<i32>, rev: Option<i32>, v: i32| {
        c.ent.voyage_commande(
            &compta,
            s.id,
            Commande::ValiderBudget,
            ParamsCommande { version: Some(v), budget_numero: numero, budget_rev: rev, ..Default::default() },
        )
    };
    // On valide la version lue : sans numéro ni version d'écritures, refus ; périmée, conflit.
    let v = version(&c).await;
    assert!(est_refus(&valider(None, None, v).await.unwrap_err()));
    assert!(est_refus(&valider(Some(numero), None, v).await.unwrap_err()));
    assert!(est_refus(&valider(Some(numero), Some(rev - 1), v).await.unwrap_err()));
    assert!(est_refus(&valider(Some(numero + 1), Some(rev), v).await.unwrap_err()));
    let s2 = valider(Some(numero), Some(rev), v).await.unwrap();
    assert_eq!(s2.etat, "a_autoriser");
    // Scellé : l'état, la part et les totaux sont dans la version ; ils égalent le recalcul.
    let (etat, part, dep, rec): (String, i64, i64, i64) = sqlx::query_as(
        "SELECT etat, part_familles_centimes, total_depenses_centimes, total_recettes_centimes
           FROM ent_sortie_budget_version WHERE sortie_id = $1",
    )
    .bind(s.id)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!((etat.as_str(), part, dep, rec), ("validee", 50_000, 150_000, 0));
    let b = c.budget(c.compta, Role::Compta, s.id).await.unwrap();
    let vv = b.validee.unwrap();
    assert_eq!(vv.calcul.part_familles, Some(part));
    assert!(b.brouillon.is_none() && b.prix_fige.is_empty(), "le prix n'est figé qu'à l'autorisation");

    // La base elle-même refuse de modifier une version validée ou ses lignes (déclencheurs).
    for sql in [
        "UPDATE ent_sortie_budget_ligne SET montant_centimes = 1 WHERE version_id IN (SELECT id FROM ent_sortie_budget_version WHERE sortie_id = $1)",
        "DELETE FROM ent_sortie_budget_ligne WHERE version_id IN (SELECT id FROM ent_sortie_budget_version WHERE sortie_id = $1)",
        "INSERT INTO ent_sortie_budget_ligne (id, etablissement_id, version_id, sens, poste, libelle, montant_centimes, motif)
           SELECT gen_random_uuid(), etablissement_id, id, 'depense', 'divers', 'x', 1, 'x'
             FROM ent_sortie_budget_version WHERE sortie_id = $1",
        "UPDATE ent_sortie_budget_version SET part_familles_centimes = 1 WHERE sortie_id = $1",
        "UPDATE ent_sortie_budget_version SET nb_payants = 9 WHERE sortie_id = $1",
        "DELETE FROM ent_sortie_budget_version WHERE sortie_id = $1",
    ] {
        let r = sqlx::query(sql).bind(s.id).execute(c.ent.pool()).await;
        assert_eq!(code_sql(&r).as_deref(), Some("42501"), "{sql} : {r:?}");
    }
    // Par l'application : version figée, il faut une révision.
    let e = c
        .ent
        .voyage_budget_ligne_ajouter(&compta, s.id, numero, rev + 1, ligne("depense", "divers", "x", "1"))
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    // Révision : motif obligatoire, copie des lignes, un seul brouillon à la fois, porteur exclu à cette étape.
    let e = c.ent.voyage_budget_version_creer(&compta, s.id, None).await.unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    let e = c
        .ent
        .voyage_budget_version_creer(&c.a(c.prof, Role::Prof), s.id, Some("je veux changer"))
        .await
        .unwrap_err();
    assert!(est_refus(&e), "à l'étape d'autorisation, le budget est à la comptabilité : {e:?}");
    let r = c
        .ent
        .voyage_budget_version_creer(&compta, s.id, Some("Nouveau devis du transporteur"))
        .await
        .unwrap();
    let r = r.brouillon.unwrap();
    assert_eq!((r.numero, r.lignes.len(), r.nb_payants), (2, 1, 3));
    assert_eq!(r.motif_revision.as_deref(), Some("Nouveau devis du transporteur"));
    let e = c
        .ent
        .voyage_budget_version_creer(&compta, s.id, Some("encore"))
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    // « Valider le budget » est réservé à l'étape du budget : une révision utilise sa propre commande.
    // Un autre profil ne valide pas.
    let e = c
        .ent
        .voyage_budget_valider_revision(&c.a(c.dir, Role::Direction), s.id, r.numero, r.rev)
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    let b = c.ent.voyage_budget_valider_revision(&compta, s.id, r.numero, r.rev).await.unwrap();
    assert_eq!(b.validee.as_ref().unwrap().numero, 2);
    let etats: Vec<(i32, String)> = b.historique.iter().map(|h| (h.numero, h.etat.clone())).collect();
    assert_eq!(etats, vec![(1, "remplacee".into()), (2, "validee".into())]);
    // L'ancienne version reste lisible et figée (l'historique ne s'efface jamais).
    let r = sqlx::query("UPDATE ent_sortie_budget_version SET etat = 'brouillon' WHERE sortie_id = $1 AND numero = 1")
        .bind(s.id)
        .execute(c.ent.pool())
        .await;
    assert_eq!(code_sql(&r).as_deref(), Some("42501"));
    let j = actions_journal(&c, s.id).await;
    assert_eq!(j.iter().filter(|a| *a == "budget_version_validee").count(), 2, "{j:?}");
}

#[tokio::test]
async fn budget_desequilibre_ne_se_valide_pas_et_dit_pourquoi() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.voyage_aux_finances("Déficit").await;
    let compta = c.a(c.compta, Role::Compta);
    c.ent.voyage_budget_version_creer(&compta, s.id, None).await.unwrap();
    c.ajouter(s.id, ligne("depense", "transport", "Car", "1 500")).await;
    // 600 € d'encadrement et 300 € de surcoût handicap, hors part des familles, sans recette qui les couvre.
    c.ajouter(s.id, ligne("depense", "encadrement", "Accompagnateurs", "600")).await;
    let v = c.ajouter(s.id, ligne("depense", "surcout_handicap", "Aide humaine", "300")).await;
    let k = v.brouillon.unwrap().calcul;
    assert_eq!(k.part_familles, Some(50_000), "l'encadrement et le surcoût ne pèsent pas sur la part");
    assert_eq!((k.deficit, k.equilibre), (90_000, false));
    let e = c.cmd(c.compta, Role::Compta, s.id, Commande::ValiderBudget).await.unwrap_err();
    match &e {
        EntDbError::ConflitDossierDetail(m) => assert!(m.contains("encadrement") && m.contains("900,00"), "{m}"),
        _ => panic!("{e:?}"),
    }
    // Un don affecté à l'encadrement le couvre sans toucher la part ; un don général réduit la part.
    let mut don = ligne("recette", "don_affecte", "Mécène", "900");
    don.affectation = Some("encadrement".into());
    let v = c.ajouter(s.id, don).await;
    let k = v.brouillon.unwrap().calcul;
    assert_eq!((k.part_familles, k.deficit, k.equilibre), (Some(50_000), 0, true));
    let mut apel = ligne("recette", "apel", "APEL", "300");
    apel.affectation = Some("general".into());
    let v = c.ajouter(s.id, apel).await;
    // (1 500 − 300) = 1 200 € ; marge 7 % = 84 € ; (1 200 + 84) ÷ 3 = 428 € (sous le coût réel de 500 €).
    assert_eq!(v.brouillon.unwrap().calcul.part_familles, Some(42_800));
    let st = c.cmd(c.compta, Role::Compta, s.id, Commande::ValiderBudget).await.unwrap();
    assert_eq!(st.etat, "a_autoriser");
    // Sans participant payant, pas de part : refus avec la cause.
    let s2 = c.voyage_aux_finances("Sans payants").await;
    c.ent.voyage_budget_version_creer(&compta, s2.id, None).await.unwrap();
    c.ajouter(s2.id, ligne("depense", "transport", "Car", "100")).await;
    let b = c.budget(c.compta, Role::Compta, s2.id).await.unwrap().brouillon.unwrap();
    c.ent
        .voyage_budget_parametrer(&compta, s2.id, b.numero, b.rev, ParamsBudget { nb_payants: Some(0), marge_pct: None, marge_motif: None })
        .await
        .unwrap();
    let e = c.cmd(c.compta, Role::Compta, s2.id, Commande::ValiderBudget).await.unwrap_err();
    assert!(matches!(&e, EntDbError::ConflitDossierDetail(m) if m.contains("participants payants")), "{e:?}");
}

// ─────────────────────────── Prix annoncé figé ───────────────────────────

#[tokio::test]
async fn prix_fige_a_l_autorisation_baisse_suivie_hausse_sur_decision() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    // Autoriser sans budget validé est impossible (précondition), même pour la direction.
    let s0 = c.voyage_aux_finances("Sans budget validé").await;
    sqlx::query("UPDATE ent_sortie SET etat = 'a_autoriser' WHERE id = $1")
        .bind(s0.id)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let e = c.cmd(c.dir, Role::Direction, s0.id, Commande::Autoriser).await.unwrap_err();
    assert!(matches!(&e, EntDbError::ConflitDossierDetail(m) if m.contains("budget validé")), "{e:?}");

    let s = c.voyage_ouvert("Prix figé").await;
    assert_eq!(s.etat, "ouverte");
    let b = c.budget(c.dir, Role::Direction, s.id).await.unwrap();
    assert_eq!(b.prix_courant_centimes, Some(50_000));
    assert_eq!((b.prix_fige.len(), b.prix_fige[0].cause.as_str()), (1, "ouverture"));
    assert!(b.peut_decider_hausse && !b.hausse_en_attente);
    // En ajout seul : ni modification ni suppression, même en SQL.
    for sql in ["UPDATE ent_sortie_prix_fige SET montant_centimes = 1 WHERE sortie_id = $1", "DELETE FROM ent_sortie_prix_fige WHERE sortie_id = $1"] {
        let r = sqlx::query(sql).bind(s.id).execute(c.ent.pool()).await;
        assert_eq!(code_sql(&r).as_deref(), Some("42501"), "{sql}");
    }
    // Baisse (1 200 € de car → 400 € par élève) : toujours possible, le prix suit, avoirs à proposer (V2).
    let b = c.reviser(s.id, "1 200").await.unwrap();
    assert_eq!(b.prix_courant_centimes, Some(40_000));
    assert_eq!(b.prix_fige.iter().map(|p| p.cause.as_str()).collect::<Vec<_>>(), ["ouverture", "baisse"]);
    // Hausse (1 800 € → 600 €) : la version est validée (le chiffre est le chiffre) mais le prix
    // annoncé ne bouge pas sans décision de la direction.
    let b = c.reviser(s.id, "1 800").await.unwrap();
    assert_eq!(b.validee.as_ref().unwrap().part_familles_centimes, Some(60_000));
    assert_eq!(b.prix_courant_centimes, Some(40_000));
    assert!(b.hausse_en_attente);
    // Ni la comptabilité, ni le porteur, ni le secrétariat ne décident ; la direction, avec un motif.
    for (u, r) in [(c.compta, Role::Compta), (c.prof, Role::Prof), (c.sec, Role::Administratif)] {
        let e = c.ent.voyage_prix_hausser(&c.a(u, r), s.id, "Hausse du carburant").await.unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{r:?} : {e:?}");
    }
    let dir = c.a(c.dir, Role::Direction);
    let e = c.ent.voyage_prix_hausser(&dir, s.id, "   ").await.unwrap_err();
    assert!(est_rejet(&e), "{e:?}");
    let b = c.ent.voyage_prix_hausser(&dir, s.id, "Hausse du carburant").await.unwrap();
    assert_eq!(b.prix_courant_centimes, Some(60_000));
    assert!(!b.hausse_en_attente);
    let dernier = b.prix_fige.last().unwrap();
    assert_eq!((dernier.cause.as_str(), dernier.motif.as_deref()), ("hausse_decidee", Some("Hausse du carburant")));
    // Plus rien à décider : refus (pas de hausse fantôme).
    let e = c.ent.voyage_prix_hausser(&dir, s.id, "encore").await.unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    // Journal : prix figé, baisse (avoirs à proposer), hausse décidée (nouvel avis à proposer).
    let j = actions_journal(&c, s.id).await;
    for a in ["prix_fige", "prix_baisse", "prix_hausse_decidee"] {
        assert!(j.contains(&a.to_string()), "{a} : {j:?}");
    }
    let note: Option<String> = sqlx::query_scalar("SELECT motif FROM ent_sortie_journal WHERE sortie_id = $1 AND action = 'prix_baisse'")
        .bind(s.id)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert!(note.unwrap().contains("avoirs"));
}

#[tokio::test]
async fn reautorisation_apres_reouverture_aligne_le_prix_sans_decision_cachee() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.voyage_ouvert("Réouverture").await;
    let dir = c.a(c.dir, Role::Direction);
    // La direction rouvre à l'autorisation ; la comptabilité révise à la hausse (1 800 € → 600 €).
    c.ent
        .voyage_commande(&dir, s.id, Commande::Rouvrir, ParamsCommande { motif: Some("Nouveau devis".into()), vers: Some("a_autoriser".into()), ..Default::default() })
        .await
        .unwrap();
    let b = c.reviser(s.id, "1 800").await.unwrap();
    assert!(b.hausse_en_attente && b.prix_courant_centimes == Some(50_000));
    // Réautoriser sans motif : la hausse exige une décision motivée de la direction.
    let e = c.cmd(c.dir, Role::Direction, s.id, Commande::Autoriser).await.unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    let version: i32 = sqlx::query_scalar("SELECT version FROM ent_sortie WHERE id = $1")
        .bind(s.id)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    let s2 = c
        .ent
        .voyage_commande(
            &dir,
            s.id,
            Commande::Autoriser,
            ParamsCommande {
                version: Some(version),
                motif: Some("Décision du conseil d'établissement".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(s2.etat, "ouverte");
    let b = c.budget(c.dir, Role::Direction, s.id).await.unwrap();
    assert_eq!(b.prix_courant_centimes, Some(60_000));
    assert_eq!(b.prix_fige.last().unwrap().cause, "hausse_decidee");
}

// ─────────────────────────── PDF ───────────────────────────

fn contient(h: &[u8], a: &str) -> bool {
    h.windows(a.len()).any(|w| w == a.as_bytes())
}

#[tokio::test]
async fn fiche_budget_et_circulaire_en_pdf_cote_serveur() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.voyage_aux_finances("Rome").await;
    let compta = c.a(c.compta, Role::Compta);
    // La fiche se rend à partir d'une version validée.
    let e = c.ent.voyage_budget_pdf(&compta, s.id).await.unwrap_err();
    assert!(est_conflit_detail(&e), "{e:?}");
    c.budget_minimal(s.id).await;
    let e = c.ent.voyage_budget_pdf(&compta, s.id).await.unwrap_err();
    assert!(est_conflit_detail(&e), "un brouillon n'est pas une version validée : {e:?}");
    c.cmd(c.compta, Role::Compta, s.id, Commande::ValiderBudget).await.unwrap();
    // Comptabilité et direction émettent ; le porteur, le secrétariat et l'encadrant non.
    for (u, r) in [(c.prof, Role::Prof), (c.sec, Role::Administratif), (c.prof2, Role::Prof)] {
        let e = c.ent.voyage_budget_pdf(&c.a(u, r), s.id).await.unwrap_err();
        assert!(est_forbidden(&e), "{r:?} : {e:?}");
    }
    let doc = c.ent.voyage_budget_pdf(&compta, s.id).await.unwrap();
    assert_eq!((doc.type_.as_str(), doc.version_budget, doc.actuel), ("budget", Some(1), true));
    let (nom, pdf) = c.ent.voyage_document_lire(&compta, s.id, doc.id).await.unwrap();
    assert!(nom.starts_with("budget-VOY-") && nom.ends_with("-v1.pdf"), "{nom}");
    assert!(pdf.starts_with(b"%PDF-1.4") && pdf.ends_with(b"%%EOF\n"));
    assert_eq!(ent_object::sha256_hex(&pdf), doc.sha256, "l'empreinte du registre est celle du fichier");
    assert_eq!(pdf.len() as i32, doc.taille);
    for attendu in ["Fiche budget", "Transport", "Total des d\\351penses", "1\\240500,00\\240\\200", "500,00\\240\\200"] {
        assert!(contient(&pdf, attendu), "le PDF doit contenir « {attendu} »");
    }
    // Rendu déterministe : mêmes faits, mêmes octets (hors empreinte d'émission).
    let doc2 = c.ent.voyage_budget_pdf(&compta, s.id).await.unwrap();
    assert_eq!(doc2.sha256, doc.sha256);
    let liste = c.ent.voyage_documents(&compta, s.id).await.unwrap();
    assert_eq!(liste.iter().filter(|d| d.actuel).count(), 1, "un seul document actuel par type");
    // Le porteur LIT la fiche (budget en lecture) ; l'encadrant, l'accueil et la famille non.
    assert!(c.ent.voyage_document_lire(&c.a(c.prof, Role::Prof), s.id, doc.id).await.is_ok());
    for (u, r) in [(c.prof2, Role::Prof), (c.accueil, Role::Accueil), (c.parent, Role::Parent)] {
        let e = c.ent.voyage_document_lire(&c.a(u, r), s.id, doc.id).await.unwrap_err();
        assert!(est_introuvable(&e), "{r:?} : {e:?}");
    }
    assert!(c.ent.voyage_documents(&c.a(c.prof2, Role::Prof), s.id).await.unwrap().is_empty());
    // Un autre établissement ne lit pas le document.
    let autre = Uuid::new_v4();
    let intrus = crate::tests_voyages_v0a::compte(&c.ent, autre, "compta").await;
    let e = c
        .ent
        .voyage_document_lire(&Acteur { etablissement_id: autre, user_id: intrus, role: "compta" }, s.id, doc.id)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // Un identifiant de document d'une autre sortie ne passe pas.
    let e = c.ent.voyage_document_lire(&compta, Uuid::new_v4(), doc.id).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");

    // Circulaire : seulement une fois la sortie autorisée, porteur ou gestion ; avec la participation figée.
    let prof = c.a(c.prof, Role::Prof);
    let e = c.ent.voyage_circulaire_pdf(&prof, s.id).await.unwrap_err();
    assert!(est_refus(&e), "pas avant l'autorisation : {e:?}");
    c.cmd(c.dir, Role::Direction, s.id, Commande::Autoriser).await.unwrap();
    for (u, r) in [(c.prof2, Role::Prof), (c.compta, Role::Compta), (c.accueil, Role::Accueil)] {
        let e = c.ent.voyage_circulaire_pdf(&c.a(u, r), s.id).await.unwrap_err();
        assert!(est_forbidden(&e) || est_introuvable(&e), "{r:?} : {e:?}");
    }
    let t = c.ent.voyage_circulaire(&prof, s.id).await.unwrap();
    assert!(t.corps.contains("Participation demandée aux familles : 500,00\u{a0}€ par élève"), "{}", t.corps);
    let c1 = c.ent.voyage_circulaire_pdf(&prof, s.id).await.unwrap();
    let c2 = c.ent.voyage_circulaire_pdf(&c.a(c.sec, Role::Administratif), s.id).await.unwrap();
    let (_, pdf) = c.ent.voyage_document_lire(&prof, s.id, c2.id).await.unwrap();
    assert!(contient(&pdf, "Participation demand\\351e aux familles : 500,00\\240\\200"), "participation figée dans la circulaire");
    assert!(!contient(&pdf, "Fiche budget"));
    // La plus récente remplace l'ancienne, qui reste lisible.
    let liste = c.ent.voyage_documents(&prof, s.id).await.unwrap();
    let circ: Vec<_> = liste.iter().filter(|d| d.type_ == "circulaire").collect();
    assert_eq!(circ.len(), 2);
    assert!(circ.iter().find(|d| d.id == c2.id).unwrap().actuel && !circ.iter().find(|d| d.id == c1.id).unwrap().actuel);
    // Le porteur ne voit pas la fiche budget dans ses documents de circulaire... il la voit (lecture), mais pas l'encadrant.
    assert!(liste.iter().any(|d| d.type_ == "budget"));
    // Les documents sont en ajout seul et leurs objets sont enregistrés.
    let r = sqlx::query("DELETE FROM ent_sortie_document WHERE sortie_id = $1").bind(s.id).execute(c.ent.pool()).await;
    assert_eq!(code_sql(&r).as_deref(), Some("42501"));
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie_document d JOIN ent_objet o
           ON o.etablissement_id = d.etablissement_id AND o.cle_objet = d.cle_objet AND o.sha256 = d.sha256
          WHERE d.sortie_id = $1",
    )
    .bind(s.id)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 4, "chaque document est au registre des objets avec son empreinte");
    let j = actions_journal(&c, s.id).await;
    assert!(j.iter().filter(|a| *a == "document_emis").count() == 4);
}

#[tokio::test]
async fn circulaire_d_une_sortie_obligatoire_ne_demande_aucune_participation() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let mut sa = c.saisie("Visite du musée", "sans_nuitee");
    sa.nature = "obligatoire".into();
    sa.destination = "proximite".into();
    let s = c.ent.voyage_creer(&c.a(c.prof, Role::Prof), sa).await.unwrap();
    c.preparer_jusqu_a_soumise(s.id).await;
    c.cmd(c.dir, Role::Direction, s.id, Commande::ValiderPedagogie).await.unwrap();
    c.cmd(c.dir, Role::Direction, s.id, Commande::Autoriser).await.unwrap();
    let t = c.ent.voyage_circulaire(&c.a(c.prof, Role::Prof), s.id).await.unwrap();
    assert!(t.corps.contains("Aucune participation financière n'est demandée"), "R-SV8 : {}", t.corps);
    assert!(!t.corps.contains("Participation demandée"));
    let d = c.ent.voyage_circulaire_pdf(&c.a(c.prof, Role::Prof), s.id).await.unwrap();
    let (_, pdf) = c.ent.voyage_document_lire(&c.a(c.prof, Role::Prof), s.id, d.id).await.unwrap();
    assert!(contient(&pdf, "Aucune participation financi\\350re"));
    // Sans budget (circuit court) : ni budget ni prix.
    assert!(c.budget(c.prof, Role::Prof, s.id).await.unwrap().prix_fige.is_empty());
}

// ─────────────────────────── Synthèse APEL ───────────────────────────

#[tokio::test]
async fn synthese_apel_etats_retenus_annee_et_droits() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    // Séjour A : budget avec APEL (300 €) et aides individuelles agrégées (120 €), validé.
    let a = c.voyage_aux_finances("Séjour A").await;
    let compta = c.a(c.compta, Role::Compta);
    c.ent.voyage_budget_version_creer(&compta, a.id, None).await.unwrap();
    c.ajouter(a.id, ligne("depense", "transport", "Car", "1 500")).await;
    c.ajouter(a.id, ligne("recette", "apel", "APEL collectif", "300")).await;
    c.ajouter(a.id, ligne("recette", "aides_individuelles", "Fonds social (total)", "120")).await;
    c.ajouter(a.id, ligne("recette", "collectivite", "Département", "100")).await;
    c.cmd(c.compta, Role::Compta, a.id, Commande::ValiderBudget).await.unwrap();
    c.cmd(c.dir, Role::Direction, a.id, Commande::Autoriser).await.unwrap();
    // « ouverte » : pas encore retenue (Next.js comptait aussi les brouillons : VD-36).
    let syn = c.ent.voyages_synthese_apel(&compta, None).await.unwrap();
    assert!(syn.lignes.is_empty(), "{syn:?}");
    // Séjour B : brouillon avec APEL, jamais retenu.
    let b = c.voyage_aux_finances("Séjour B").await;
    c.ent.voyage_budget_version_creer(&compta, b.id, None).await.unwrap();
    c.ajouter(b.id, ligne("recette", "apel", "APEL", "999")).await;
    // La liste confirmée (participants) rend le séjour A « prêt » : il entre dans la synthèse.
    c.ent
        .voyage_participants_ajouter(&c.a(c.prof, Role::Prof), a.id, vec![c.eleves[0], c.eleves[1], c.eleves[2]])
        .await
        .unwrap();
    let etape = |type_etape: &str, debut: DateTime<Utc>| EtapeSaisie {
        type_etape: type_etape.into(),
        debut,
        fin: None,
        duree_minutes: Some(30),
        lieu: Some("Parvis".into()),
        note: None,
        appel_requis: type_etape == "depot",
    };
    c.ent
        .voyage_etapes_remplacer(
            &c.a(c.prof, Role::Prof),
            a.id,
            vec![etape("depot", c.at(7)), etape("reprise", c.at(8) + Duration::days(2))],
        )
        .await
        .unwrap();
    let p = c.cmd(c.prof, Role::Prof, a.id, Commande::ConfirmerListe).await.unwrap();
    assert_eq!(p.etat, "prete");
    let syn = c.ent.voyages_synthese_apel(&compta, None).await.unwrap();
    assert_eq!(syn.lignes.len(), 1);
    assert_eq!((syn.lignes[0].apel_centimes, syn.lignes[0].aides_centimes), (30_000, 12_000));
    assert_eq!((syn.total_apel_centimes, syn.total_aides_centimes, syn.total_centimes), (30_000, 12_000, 42_000));
    assert_eq!(syn.lignes[0].sortie_id, a.id);
    // Année du socle 5 : une autre année n'a rien ; l'année d'un autre établissement est introuvable.
    let annee_autre = Uuid::new_v4();
    let e = c.ent.voyages_synthese_apel(&compta, Some(annee_autre)).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // CSV : en-tête, montants à virgule, total ; aucune formule interprétable ; aucun nom.
    let csv = c.ent.voyages_synthese_apel_csv(&compta, None).await.unwrap();
    let lignes: Vec<&str> = csv.lines().collect();
    assert_eq!(lignes[0], "Référence;Titre;Départ;État;APEL collectif (€);Aides individuelles (€);Total (€)");
    assert!(lignes[1].contains(";Séjour A;") && lignes[1].ends_with(";prete;300,00;120,00;420,00"), "{}", lignes[1]);
    assert_eq!(lignes.last().unwrap(), &"Total;;;;300,00;120,00;420,00");
    // Comptabilité seule : ni la direction, ni le secrétariat, ni le porteur.
    for (u, r) in [(c.dir, Role::Direction), (c.sec, Role::Administratif), (c.prof, Role::Prof), (c.parent, Role::Parent)] {
        let e = c.ent.voyages_synthese_apel(&c.a(u, r), None).await.unwrap_err();
        assert!(est_forbidden(&e), "{r:?} : {e:?}");
    }
}

// ─────────────────────────── Rappels et avis ───────────────────────────

#[tokio::test]
async fn rappels_de_l_organisateur_aux_echeances_idempotents_et_sans_effet_externe() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let d = c.ent.distributeur(config(c.etab));
    // Voyage à J-14 encore en préparation : réunion des parents (J-30) et dossier bloqué (J-14).
    let v = c.creer("Voyage à Berlin", "voyage").await;
    c.preparer_jusqu_a_soumise(v.id).await;
    c.cmd(c.dir, Role::Direction, v.id, Commande::ValiderPedagogie).await.unwrap();
    let depart = v.debut;
    assert_eq!(c.ent.voyages_rappels_traiter(c.etab, depart - Duration::days(45)).await.unwrap(), 0);
    assert_eq!(c.ent.voyages_rappels_traiter(c.etab, depart - Duration::days(14)).await.unwrap(), 2);
    // Idempotent : un second passage, ou un rattrapage plus tard, ne double rien.
    assert_eq!(c.ent.voyages_rappels_traiter(c.etab, depart - Duration::days(14)).await.unwrap(), 0);
    assert_eq!(c.ent.voyages_rappels_traiter(c.etab, depart - Duration::days(13)).await.unwrap(), 0);
    let r = c.ent.voyage_rappels(&c.a(c.prof, Role::Prof), v.id).await.unwrap();
    let mut codes: Vec<&str> = r.iter().map(|x| x.code.as_str()).collect();
    codes.sort();
    assert_eq!(codes, ["dossier_bloque", "reunion_parents"]);
    assert!(r.iter().all(|x| x.jours_avant == 14 && !x.message.is_empty()));
    // Le porteur et la direction les lisent ; un encadrant ou l'accueil non.
    assert!(c.ent.voyage_rappels(&c.a(c.dir, Role::Direction), v.id).await.is_ok());
    let e = c.ent.voyage_rappels(&c.a(c.prof2, Role::Prof), v.id).await.unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    // Avis du socle 8 : le porteur est prévenu (deux rappels), personne d'autre.
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&c, c.prof, "voyages_rappel_organisateur").await, 2);
    for u in [c.dir, c.compta, c.sec, c.prof2, c.parent] {
        assert_eq!(notifs(&c, u, "voyages_rappel_organisateur").await, 0);
    }
    // Aucun envoi aux familles ni aux prestataires (R-SV3) : aucune notification de famille.
    assert_eq!(notifs(&c, c.parent, "voyages_rappel_organisateur").await, 0);

    // Sortie sans nuitée avec car, ouverte, liste non confirmée : J-7 effectif, J-4 liste au car, J0 communication.
    let mut sa = c.saisie("Sortie au château", "sans_nuitee");
    sa.besoin_car = true;
    let s = c.ent.voyage_creer(&c.a(c.prof, Role::Prof), sa).await.unwrap();
    c.preparer_jusqu_a_soumise(s.id).await;
    c.cmd(c.dir, Role::Direction, s.id, Commande::ValiderPedagogie).await.unwrap();
    c.cmd(c.dir, Role::Direction, s.id, Commande::DemanderSignature).await.unwrap();
    c.ent
        .voyage_commande(
            &c.a(c.dir, Role::Direction),
            s.id,
            Commande::PasserAuxFinances,
            ParamsCommande { motif: Some("Devis oral".into()), ..Default::default() },
        )
        .await
        .unwrap();
    c.budget_minimal(s.id).await;
    c.cmd(c.compta, Role::Compta, s.id, Commande::ValiderBudget).await.unwrap();
    c.cmd(c.dir, Role::Direction, s.id, Commande::Autoriser).await.unwrap();
    let dep = s.debut;
    // J-10 : seul le voyage (autre sortie, déjà traité) pourrait en avoir ; celle-ci : rien.
    assert_eq!(passage(&c, dep, 10).await, 0);
    assert_eq!(passage(&c, dep, 7).await, 1, "effectif non confirmé");
    assert_eq!(passage(&c, dep, 5).await, 0);
    assert_eq!(passage(&c, dep, 4).await, 1, "liste au car");
    assert_eq!(passage(&c, dep, 0).await, 1, "communication du jour");
    // Parti ou passé : plus de rappel.
    assert_eq!(passage(&c, dep, -1).await, 0);
    let codes: Vec<String> = c.ent.voyage_rappels(&c.a(c.prof, Role::Prof), s.id).await.unwrap().into_iter().map(|x| x.code).collect();
    assert_eq!(codes, ["effectif", "liste_car", "communication_parents"]);
    // Déclenchement manuel : direction seule ; balayage de tous les établissements sans erreur.
    let e = c.ent.voyages_rappels_declencher(&c.a(c.compta, Role::Compta), dep).await.unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    assert_eq!(c.ent.voyages_rappels_declencher(&c.a(c.dir, Role::Direction), dep).await.unwrap(), 0);
    c.ent.voyages_rappels_traiter_tous(dep - Duration::days(400)).await.unwrap();
}

#[tokio::test]
async fn avis_du_socle_8_aux_bons_profils_sans_texte_libre() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let d = c.ent.distributeur(config(c.etab));
    let s = c.creer("Voyage à Vienne", "voyage").await;
    c.preparer_jusqu_a_soumise(s.id).await; // soumission → direction
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&c, c.dir, "voyages_nouvelle_demande").await, 1);
    for u in [c.prof, c.compta, c.sec, c.prof2, c.parent] {
        assert_eq!(notifs(&c, u, "voyages_nouvelle_demande").await, 0);
    }
    // Modifications demandées : le porteur est prévenu, sans recopier la note de la direction.
    c.ent
        .voyage_commande(&c.a(c.dir, Role::Direction), s.id, Commande::DemanderModifications, ParamsCommande { motif: Some("SECRET-DE-DIRECTION".into()), ..Default::default() })
        .await
        .unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&c, c.prof, "voyages_a_completer").await, 1);
    let textes: Vec<String> = c.ent.mes_notifications(c.etab, c.prof, 50).await.unwrap().into_iter().map(|n| format!("{} {}", n.titre, n.corps)).collect();
    assert!(textes.iter().all(|t| !t.contains("SECRET")), "{textes:?}");
    c.cmd(c.prof, Role::Prof, s.id, Commande::Renvoyer).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&c, c.dir, "voyages_nouvelle_demande").await, 2, "renvoi vers la validation pédagogique");
    c.cmd(c.dir, Role::Direction, s.id, Commande::ValiderPedagogie).await.unwrap();
    // Passage aux finances (NX-64) : la comptabilité est prévenue, pas la direction.
    c.cmd(c.prof, Role::Prof, s.id, Commande::PasserAuxFinances).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&c, c.compta, "voyages_budget_a_valider").await, 1);
    assert_eq!(notifs(&c, c.dir, "voyages_budget_a_valider").await, 0);
    // Autorisation : le porteur est prévenu que la sortie est validée.
    c.budget_minimal(s.id).await;
    c.cmd(c.compta, Role::Compta, s.id, Commande::ValiderBudget).await.unwrap();
    c.cmd(c.dir, Role::Direction, s.id, Commande::Autoriser).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&c, c.prof, "voyages_validee").await, 1);
    // Un rejeu de la distribution ne double aucun avis.
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&c, c.prof, "voyages_validee").await, 1);
    // Refus : avis au porteur d'une autre sortie ; la famille n'en reçoit jamais.
    let r = c.creer("Voyage refusé", "voyage").await;
    c.preparer_jusqu_a_soumise(r.id).await;
    c.ent
        .voyage_commande(&c.a(c.dir, Role::Direction), r.id, Commande::Refuser, ParamsCommande { motif: Some("Budget".into()), ..Default::default() })
        .await
        .unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifs(&c, c.prof, "voyages_refusee").await, 1);
    assert_eq!(notifs(&c, c.parent, "voyages_refusee").await, 0);
}

// ─────────────────────────── Schéma ───────────────────────────

#[test]
fn migration_643_sans_json_ni_bytea_et_sans_identite_d_eleve() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    let sql: String = std::fs::read_to_string(dir.join("643_sortie_budget.sql"))
        .unwrap()
        .to_lowercase()
        .lines()
        .map(|l| l.split("--").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!sql.contains("json"), "D-036 : aucun JSON");
    assert!(!sql.contains("bytea"), "fichiers : StorageProvider, jamais BYTEA");
    assert!(!sql.contains("eleve_id"), "aucune identité d'élève dans le budget (R-SV18)");
    assert!(!sql.contains("float") && !sql.contains("numeric") && !sql.contains("real"), "centimes entiers (R-SV16)");
}

#[cfg(feature = "labo")]
#[tokio::test]
async fn rls_des_cinq_tables_du_lot_v0b() {
    crate::tests_rls_lots::verifier_tables(&[
        "ent_sortie_budget_version",
        "ent_sortie_budget_ligne",
        "ent_sortie_prix_fige",
        "ent_sortie_rappel",
        "ent_sortie_document",
    ])
    .await;
}
