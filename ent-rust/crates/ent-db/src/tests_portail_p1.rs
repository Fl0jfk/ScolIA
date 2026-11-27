//! Socle 29, lot P1 : « Aujourd'hui » par profil (`docs/socles/29_PORTAILS.md` §9, §15.4, §24).
//! Base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL` le scénario s'arrête (échec en CI,
//! garde `url_labo_ou_echec`). Chaque scénario commence par un **témoin** (l'acteur légitime voit
//! sa tuile) avant la mesure de protection.

use super::*;
use chrono::{NaiveDate, NaiveDateTime};
use ent_types::portail::{EtatTuile, PortailAujourdhui, ProfilPortail};

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |_| {}).await;
    Some((ent, cast))
}

fn instant() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 11, 12).unwrap().and_hms_opt(9, 30, 0).unwrap()
}

async fn page(ent: &PgEnt, etab: uuid::Uuid, user: uuid::Uuid, role: ent_auth::Role) -> PortailAujourdhui {
    ent.portail_aujourdhui(etab, user, role.as_str(), instant())
        .await
        .unwrap_or_else(|e| panic!("portail {role:?} : {e:?}"))
}

fn codes(p: &PortailAujourdhui) -> Vec<&str> {
    p.tuiles.iter().map(|t| t.code.as_str()).collect()
}

fn texte(p: &PortailAujourdhui) -> String {
    serde_json::to_string(p).unwrap()
}

#[tokio::test]
async fn chaque_profil_recoit_ses_tuiles_et_pas_celles_des_autres() {
    let Some((ent, c)) = labo().await else { return };
    let e = c.etablissement_id;

    let direction = page(&ent, e, c.direction.user_id, c.direction.role).await;
    assert_eq!(direction.profil, ProfilPortail::Direction);
    assert_eq!(direction.jour, instant().date(), "le jour vient de l'appelant (heure de l'établissement)");
    assert!(codes(&direction).contains(&"direction.protections"), "témoin direction : {:?}", codes(&direction));
    assert!(codes(&direction).contains(&"personnels.absences"));

    let cpe = page(&ent, e, c.cpe.user_id, c.cpe.role).await;
    assert_eq!(cpe.profil, ProfilPortail::VieScolaire);
    assert!(codes(&cpe).contains(&"vs.jour"), "témoin CPE : {:?}", codes(&cpe));
    assert!(!codes(&cpe).contains(&"direction.protections"));
    assert!(!codes(&cpe).contains(&"personnels.absences"), "absences de personnels : direction seule");

    let prof = page(&ent, e, c.prof.user_id, c.prof.role).await;
    assert_eq!(prof.profil, ProfilPortail::Professeur);
    assert!(codes(&prof).contains(&"edt.journee"));
    for interdit in ["vs.jour", "direction.protections", "personnels.absences", "inscriptions.instruire"] {
        assert!(!codes(&prof).contains(&interdit), "le professeur ne doit pas recevoir {interdit}");
    }

    let secr = page(&ent, e, c.administratif.user_id, c.administratif.role).await;
    assert_eq!(secr.profil, ProfilPortail::Secretariat);
    assert!(codes(&secr).contains(&"inscriptions.instruire"), "témoin secrétariat : {:?}", codes(&secr));

    let compta = page(&ent, e, c.compta.user_id, c.compta.role).await;
    assert_eq!(compta.profil, ProfilPortail::Comptabilite);
    assert!(!codes(&compta).contains(&"vs.jour"));

    // Aucune tuile d'erreur 403 : une tuile est soit permise, soit absente (DP-06).
    for p in [&direction, &cpe, &prof, &secr, &compta] {
        assert!(p.tuiles.iter().all(|t| t.etat != EtatTuile::Indisponible), "source indisponible : {:?}", codes(p));
    }
}

#[tokio::test]
async fn la_famille_voit_ses_enfants_et_aucune_note_ni_autre_foyer() {
    let Some((ent, c)) = labo().await else { return };
    let e = c.etablissement_id;

    let lea = page(&ent, e, c.parent_lea.user_id, c.parent_lea.role).await;
    assert_eq!(lea.profil, ProfilPortail::Famille);
    for attendu in ["afaire.famille", "edt.enfants", "messagerie.non_lus", "annonces.une", "notes.geste"] {
        assert!(codes(&lea).contains(&attendu), "témoin famille : {attendu} dans {:?}", codes(&lea));
    }
    // DP-01 : la tuile annonces n'est plus une erreur « non disponible pour votre profil ».
    assert!(lea.tuiles.iter().all(|t| t.etat != EtatTuile::Indisponible));
    // DP-03 : la tuile notes ne transporte aucune note, elle attend un geste.
    let notes = lea.tuiles.iter().find(|t| t.code == "notes.geste").unwrap();
    assert!(notes.sur_geste && notes.lignes.is_empty() && notes.nombre.is_none());
    // Ni vie scolaire, ni inscription, ni protection pour une famille.
    for interdit in ["vs.jour", "inscriptions.instruire", "direction.protections", "personnels.absences", "accueil.absence"] {
        assert!(!codes(&lea).contains(&interdit), "la famille ne doit pas recevoir {interdit}");
    }

    // Aucune fuite entre foyers : le foyer de Tom ne contient rien du foyer de Léa, et inversement.
    let tom = page(&ent, e, c.parent_tom.user_id, c.parent_tom.role).await;
    let (t_lea, t_tom) = (texte(&lea), texte(&tom));
    let lea_prenom = &c.eleves[0].prenom;
    let tom_prenom = &c.eleves[1].prenom;
    assert!(!t_tom.contains(lea_prenom.as_str()) && !t_tom.contains(&c.eleves[0].nom), "fuite Léa → Tom");
    assert!(!t_lea.contains(tom_prenom.as_str()) && !t_lea.contains(&c.eleves[1].nom), "fuite Tom → Léa");
}

#[tokio::test]
async fn parent_sans_droit_de_decision_n_a_aucune_tache_et_une_mention_sobre() {
    let Some((ent, c)) = labo().await else { return };
    // Le lien de Draco n'a pas `peut_justifier_absence` (graine) : informé, jamais sollicité (R-PO8).
    let tom = page(&ent, c.etablissement_id, c.parent_tom.user_id, c.parent_tom.role).await;
    let t = tom.tuiles.iter().find(|t| t.code == "afaire.famille").expect("tuile à faire");
    assert_eq!(t.nombre, Some(0));
    assert!(t.lignes.iter().any(|l| l.libelle.contains("titulaires de l'autorité parentale")));
}

#[tokio::test]
async fn un_autre_etablissement_ne_revele_rien() {
    let Some((ent, c)) = labo().await else { return };
    let autre = ent.seed_labo_cast().await.expect("second établissement");
    if autre.etablissement_id == c.etablissement_id {
        return; // le semis réutilise l'établissement : rien à mesurer ici
    }
    // Témoin : dans son établissement, la famille voit son enfant.
    let ok = page(&ent, c.etablissement_id, c.parent_lea.user_id, c.parent_lea.role).await;
    assert!(texte(&ok).contains(c.eleves[0].prenom.as_str()) || ok.tuiles.iter().any(|t| t.code == "edt.enfants"));
    // Attaque : mêmes identifiants de compte présentés à l'autre établissement (contexte forgé).
    let croise = ent
        .portail_aujourdhui(autre.etablissement_id, c.parent_lea.user_id, c.parent_lea.role.as_str(), instant())
        .await;
    if let Ok(p) = croise {
        let t = texte(&p);
        assert!(!t.contains(c.eleves[0].prenom.as_str()), "fuite inter-établissements");
        assert!(!t.contains(autre.eleves[0].prenom.as_str()) || p.tuiles.iter().all(|x| x.lignes.iter().all(|l| !l.libelle.contains(&autre.eleves[0].nom))), "enfant d'un autre établissement");
    }
}

#[tokio::test]
async fn l_eleve_voit_sa_journee_sans_note() {
    let Some((ent, c)) = labo().await else { return };
    let Some(compte) = c.eleves.iter().find_map(|e| e.compte.clone()) else { return };
    let p = page(&ent, c.etablissement_id, compte.user_id, compte.role).await;
    assert_eq!(p.profil, ProfilPortail::Eleve);
    assert!(codes(&p).contains(&"edt.journee"));
    let n = p.tuiles.iter().find(|t| t.code == "notes.geste").unwrap();
    assert!(n.sur_geste && n.lignes.is_empty());
}

#[tokio::test]
async fn role_inconnu_refuse() {
    let Some((ent, c)) = labo().await else { return };
    let r = ent.portail_aujourdhui(c.etablissement_id, c.direction.user_id, "pirate", instant()).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)));
}
