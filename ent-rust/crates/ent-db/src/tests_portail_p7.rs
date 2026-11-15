//! Socle 29, lot P7 : modes et bandeaux de l'accueil sur une base locale (feature `labo`). Aucune heure réelle :
//! le jour de l'accueil est passé par l'appelant, l'année de labo est lointaine (2098-2099) et les
//! établissements sont neufs. Critères : une fermeture décidée la veille est visible le matin ; la page d'alerte
//! remplace tout, sans détail opérationnel pour les familles.

use super::*;
use crate::portail_modes::AlerteSimulee;
use crate::tests_structure_s3::{acteur_de, etab_avec_annee, fermeture};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Weekday};
use ent_types::portail::{GraviteBandeau, ModePortail, PortailAujourdhui};
use uuid::Uuid;

fn d(a: i32, m: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, j).unwrap()
}

fn a(jour: NaiveDate, h: u32) -> NaiveDateTime {
    jour.and_hms_opt(h, 0, 0).unwrap()
}

/// Mercredi de la semaine lointaine : jour de cours, sans rapport avec le jour réel.
fn matin() -> NaiveDate {
    let m = d(2099, 1, 14);
    assert!(!matches!(m.weekday(), Weekday::Sat | Weekday::Sun), "{m}");
    m
}

async fn page(
    ent: &PgEnt,
    etab: Uuid,
    p: &crate::seed::LaboPersona,
    quand: NaiveDateTime,
    alerte: &dyn crate::portail_modes::PortAlerte,
) -> PortailAujourdhui {
    ent.portail_aujourdhui_avec(etab, p.user_id, p.role.as_str(), quand, alerte)
        .await
        .unwrap_or_else(|e| panic!("portail {:?} : {e:?}", p.role))
}

/// Critère : « une fermeture décidée la veille est visible le matin », pour la famille, le personnel et la
/// direction ; la veille au soir, un bandeau d'information l'annonce déjà.
#[tokio::test]
async fn fermeture_decidee_la_veille_visible_le_matin() {
    let Some((ent, cast)) = crate::tests_d022_bis::labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_avec_annee(&ent, "2098-2099", d(2098, 9, 1), d(2099, 8, 31), false).await;
    let veille = matin() - Duration::days(1);
    let dir = acteur_de(m.etab, &cast.direction);
    let rien = crate::portail_modes::AucuneAlerte;

    // Témoin : avant toute décision, l'accueil est ordinaire et sans bandeau.
    let avant = page(&ent, m.etab, &cast.parent_lea, a(matin(), 7), &rien).await;
    assert_eq!(avant.mode, ModePortail::Ordinaire);
    assert!(avant.bandeaux.is_empty() && avant.bandeaux_replies.is_empty());

    // La veille, la direction décide de fermer demain (publiée aux familles par défaut).
    ent.structure_fermeture_declarer(&dir, fermeture(matin(), matin()), false)
        .await
        .expect("déclarer");

    // La veille au soir : le jour reste ordinaire, mais la fermeture de demain est annoncée.
    let soir = page(&ent, m.etab, &cast.parent_lea, a(veille, 20), &rien).await;
    assert_eq!(soir.mode, ModePortail::Ordinaire);
    assert!(
        soir.bandeaux
            .iter()
            .any(|b| b.code == "structure.fermeture_demain" && b.gravite == GraviteBandeau::Info),
        "{:?}",
        soir.bandeaux
    );

    // Le matin : journée modifiée, bandeau d'attention, tuile « Aujourd'hui modifié » en tête.
    for (nom, p) in [
        ("famille", &cast.parent_lea),
        ("direction", &cast.direction),
        ("prof", &cast.prof),
    ] {
        let pg = page(&ent, m.etab, p, a(matin(), 7), &rien).await;
        assert_eq!(pg.mode, ModePortail::JourneeModifiee, "{nom}");
        let b = pg
            .bandeaux
            .iter()
            .find(|b| b.code == "structure.fermeture")
            .unwrap_or_else(|| panic!("{nom} : {:?}", pg.bandeaux));
        assert!(b.texte.contains("Alerte météo"), "{}", b.texte);
        assert_eq!(
            pg.tuiles[0].code, "journee.modifiee",
            "{nom} : la tuile du mode est en tête"
        );
        assert!(
            !serde_json::to_string(&pg).unwrap().contains("préfecture"),
            "{nom} : le motif interne ne sort pas"
        );
    }

    // Le lendemain, tout est redevenu ordinaire.
    let apres = page(
        &ent,
        m.etab,
        &cast.parent_lea,
        a(matin() + Duration::days(1), 7),
        &rien,
    )
    .await;
    assert_eq!(apres.mode, ModePortail::Ordinaire);

    // Cloisonnement : un autre établissement ne voit pas cette fermeture.
    let autre = etab_avec_annee(&ent, "2098-2099", d(2098, 9, 1), d(2099, 8, 31), false).await;
    let pg = page(&ent, autre.etab, &cast.parent_lea, a(matin(), 7), &rien).await;
    assert_eq!(pg.mode, ModePortail::Ordinaire);
    assert!(pg.bandeaux.is_empty());
}

/// Une fermeture non publiée aux familles reste invisible pour elles, visible de la direction.
#[tokio::test]
async fn fermeture_non_publiee_invisible_des_familles() {
    let Some((ent, cast)) = crate::tests_d022_bis::labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = etab_avec_annee(&ent, "2098-2099", d(2098, 9, 1), d(2099, 8, 31), false).await;
    let dir = acteur_de(m.etab, &cast.direction);
    let mut f = fermeture(matin(), matin());
    f.publie_familles = false;
    ent.structure_fermeture_declarer(&dir, f, false)
        .await
        .expect("déclarer");
    let rien = crate::portail_modes::AucuneAlerte;
    let fam = page(&ent, m.etab, &cast.parent_lea, a(matin(), 7), &rien).await;
    assert_eq!(fam.mode, ModePortail::Ordinaire);
    assert!(fam.bandeaux.is_empty());
    let dirp = page(&ent, m.etab, &cast.direction, a(matin(), 7), &rien).await;
    assert_eq!(dirp.mode, ModePortail::JourneeModifiee);
}

/// Critère : la page d'alerte. La page entière devient la page d'alerte ; familles : consigne seulement.
#[tokio::test]
async fn page_d_alerte_remplace_tout_et_ne_fuit_pas() {
    let Some((ent, cast)) = crate::tests_d022_bis::labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let port = AlerteSimulee::default();
    let quand = a(matin(), 10);

    // Témoin : sans alerte, le professeur reçoit ses tuiles.
    let sans = page(&ent, etab, &cast.prof, quand, &port).await;
    assert_ne!(sans.mode, ModePortail::Alerte);
    assert!(sans.alerte.is_none());

    port.declencher(etab, "Mise à l'abri : restez dans votre salle.");
    port.confier(etab, cast.prof.user_id, "DUPONT Léa — 4B", "présent");

    let prof = page(&ent, etab, &cast.prof, quand, &port).await;
    assert_eq!(prof.mode, ModePortail::Alerte);
    assert!(prof.tuiles.is_empty(), "aucune autre tuile");
    let al = prof.alerte.as_ref().expect("page d'alerte");
    assert_eq!(al.consigne, "Mise à l'abri : restez dans votre salle.");
    assert_eq!(al.eleves_en_charge.len(), 1);
    assert!(al.action_securite.is_none());
    assert_eq!(prof.bandeaux.len(), 1);
    assert_eq!(prof.bandeaux[0].gravite, GraviteBandeau::Securite);
    assert!(!prof.bandeaux[0].masquable, "jamais masquable");

    // Familles : la consigne officielle, aucune liste d'élèves, aucune tuile.
    let fam = page(&ent, etab, &cast.parent_lea, quand, &port).await;
    assert_eq!(fam.mode, ModePortail::Alerte);
    assert!(fam.tuiles.is_empty());
    let al = fam.alerte.as_ref().unwrap();
    assert!(al.eleves_en_charge.is_empty());
    assert!(!serde_json::to_string(&fam).unwrap().contains("DUPONT"));

    // Autre personnel sans élève en charge : la page sans liste.
    let cpe = page(&ent, etab, &cast.cpe, quand, &port).await;
    assert!(cpe.alerte.unwrap().eleves_en_charge.is_empty());

    // Cloisonnement : l'alerte d'un établissement ne touche pas l'autre.
    let autre = etab_avec_annee(&ent, "2098-2099", d(2098, 9, 1), d(2099, 8, 31), false).await;
    let pg = page(&ent, autre.etab, &cast.parent_lea, quand, &port).await;
    assert_ne!(pg.mode, ModePortail::Alerte);

    // Clôture (propre au socle 14) : l'accueil redevient ordinaire.
    port.cloturer(etab);
    let apres = page(&ent, etab, &cast.prof, quand, &port).await;
    assert_ne!(apres.mode, ModePortail::Alerte);
}
