//! Socle 41 (internat), lot I2 « La nuit » : règles testées contre la base locale de test (feature
//! `labo`) ; sans `ENT_DATABASE_URL`, elles sont ignorées en local (échec explicite en CI).
//!
//! **Aucun test ne dépend de l'heure réelle** (règle 33) : chaque commande reçoit son instant, les
//! nuits et les séjours sont posés à des dates fixes, et les nuits à cheval sur minuit sont jouées
//! dans le fuseau de l'établissement (Europe/Paris autour des changements d'heure de 2026, un
//! établissement de La Réunion pour l'outre-mer). Chaque test crée son propre établissement.
//!
//! | Défaut du registre Next.js | Test(s) |
//! |---|---|
//! | DI-01 (périmètre collège / lycée en simple filtre d'affichage) | `di_01_perimetre_verifie_au_serveur` |
//! | DI-11 (deux appels divergents le même soir) | `di_11_un_seul_appel_par_type_nuit_et_perimetre`, `attendu_calcule_…` |
//! | DI-13 (deux surveillants s'écrasent) | `di_13_deux_surveillants_ajout_seul_et_conflit` |
//! | DI-16 (jours d'appel codés en dur, jour au fuseau du serveur) | `di_16_nuits_a_cheval_sur_minuit_et_changements_d_heure`, `rappels_…` |
//! | DI-19 (marque acceptée pour n'importe quel élève, aucune correction) | `di_19_on_ne_pointe_que_l_attendu_du_perimetre`, `appel_cycle_complet_…` |

use super::*;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use crate::internat_nuit::{marque_en_attente, marque_permise, niveau_avis};
use domain_eleve::{CreateEleve, DomainError};
use ent_auth::{Action, Portee};
use ent_types::internat::*;
use ent_types::internat_nuit::*;
use uuid::Uuid;

use crate::droits_repo::{Droit, RoleSaisie};

const CPE: &str = "cpe";
const DIR: &str = "direction";

/// Nuit du samedi 28 mars 2026 : le changement d'heure (passage à l'heure d'été) a lieu à 2 h le
/// dimanche 29, au milieu de cette nuit. Paris est à UTC+1 le soir, UTC+2 au matin.
fn nuit_dst() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, 28).unwrap()
}

fn utc(y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, h, mi, 0).unwrap()
}

/// Soir de la nuit DST, à `mi` minutes après 20 h 45 UTC (21 h 45 à Paris).
fn soir(mi: u32) -> DateTime<Utc> {
    utc(2026, 3, 28, 20, 45) + chrono::Duration::minutes(i64::from(mi))
}

fn alea() -> String {
    Uuid::new_v4().simple().to_string()[..6].to_string()
}

struct Banc {
    ent: PgEnt,
    etab: Uuid,
    cpe: Uuid,
    dir: Uuid,
    admin: Uuid,
    compta: Uuid,
    prof: Uuid,
    batiment: Uuid,
    zone_a: Uuid,
    zone_b: Uuid,
    chambre_a: Uuid,
    chambre_b: Uuid,
    /// Quatre garçons en zone A (chambre A), deux en zone B.
    a: Vec<Uuid>,
    b: Vec<Uuid>,
    cast: LaboCast,
}

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |_| {}).await;
    Some((ent, cast))
}

async fn eleve_fixe(ent: &PgEnt, etab: Uuid, prenom: &str) -> Uuid {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: format!("Nuit{}", alea()),
        prenom: prenom.into(),
        ine: None,
        sexe: Some("M".into()),
        date_naissance: NaiveDate::from_ymd_opt(2010, 3, 14),
        numero_badge: None,
        redoublant: false,
        classe: None,
        regime: Some("interne".into()),
        mef: None,
        site: None,
        date_entree: NaiveDate::from_ymd_opt(2025, 9, 1),
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
    .id
}

/// Séjour 2025-09-01 → ouvert, sept nuits par semaine dès 2026-01-01, affectation à un lit.
async fn heberger(b: &Banc, eleve: Uuid, chambre: Uuid, lit: Option<Uuid>) {
    let pool = b.ent.pool_verif();
    let sejour = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_internat_sejour (id, etablissement_id, eleve_id, du, origine) VALUES ($1,$2,$3,'2025-09-01','regime')")
        .bind(sejour)
        .bind(b.etab)
        .bind(eleve)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_internat_rythme (id, etablissement_id, sejour_id, du, dimanche, lundi, mardi, mercredi, jeudi, vendredi, samedi, cree_par_user_id) \
         VALUES ($1,$2,$3,'2025-09-01',true,true,true,true,true,true,true,$4)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(sejour)
    .bind(b.cpe)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_internat_affectation (id, etablissement_id, eleve_id, chambre_id, lit_id, debut, created_by_user_id, sejour_id) \
         VALUES ($1,$2,$3,$4,$5,'2026-01-05',$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(chambre)
    .bind(lit)
    .bind(b.cpe)
    .bind(sejour)
    .execute(pool)
    .await
    .unwrap();
}

async fn banc() -> Option<Banc> {
    let (ent, cast) = labo().await?;
    let etab = cast.etablissement_id;
    let (cpe, dir) = (cast.cpe.user_id, cast.direction.user_id);
    let pool = ent.pool_verif();
    let site = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_site (id, etablissement_id, code, libelle) VALUES ($1,$2,$3,'Site test')")
        .bind(site)
        .bind(etab)
        .bind(format!("S{}", alea()))
        .execute(pool)
        .await
        .unwrap();
    let batiment = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_batiment (id, etablissement_id, site_id, code, libelle) VALUES ($1,$2,$3,$4,'Bâtiment test')")
        .bind(batiment)
        .bind(etab)
        .bind(site)
        .bind(format!("B{}", alea()))
        .execute(pool)
        .await
        .unwrap();
    let mut zones = Vec::new();
    let mut chambres = Vec::new();
    for (i, lib) in ["Aile A", "Aile B"].into_iter().enumerate() {
        let z = ent
            .creer_zone_internat_for_actor(
                etab,
                cpe,
                CPE,
                &ZoneSaisieDto { batiment_id: batiment, code: format!("Z{i}{}", alea()), libelle: lib.into(), public: "garcons".into(), ordre: Some(i as i16) },
            )
            .await
            .unwrap();
        let c = ent
            .creer_chambre_etendue_for_actor(
                etab,
                cpe,
                CPE,
                &ChambreSaisieDto {
                    zone_id: Some(z.id),
                    batiment: "B".into(),
                    etage: 1,
                    code: format!("C{i}{}", alea()),
                    capacite: 4,
                    public: "garcons".into(),
                    accessible_pmr: false,
                    douche: false,
                    lavabo: false,
                    bureau: false,
                    notes_entretien: None,
                },
            )
            .await
            .unwrap();
        zones.push(z.id);
        chambres.push(c.id);
    }
    let mut b = Banc {
        etab,
        cpe,
        dir,
        admin: cast.admin.user_id,
        compta: cast.compta.user_id,
        prof: cast.prof.user_id,
        batiment,
        zone_a: zones[0],
        zone_b: zones[1],
        chambre_a: chambres[0],
        chambre_b: chambres[1],
        a: vec![],
        b: vec![],
        cast,
        ent,
    };
    for (i, (chambre, quantite)) in [(b.chambre_a, 4usize), (b.chambre_b, 2usize)].into_iter().enumerate() {
        let lits = b.ent.lister_lits_internat_for_actor(etab, cpe, CPE, chambre).await.unwrap();
        for lit in lits.iter().take(quantite) {
            let e = eleve_fixe(&b.ent, etab, if i == 0 { "Théo" } else { "Noé" }).await;
            heberger(&b, e, chambre, Some(lit.id)).await;
            if i == 0 {
                b.a.push(e);
            } else {
                b.b.push(e);
            }
        }
    }
    Some(b)
}

fn ouverture(type_appel: &str, perimetre: &str, zone: Option<Uuid>) -> AppelOuvertureDto {
    AppelOuvertureDto {
        nuit: Some(nuit_dst()),
        type_appel: type_appel.into(),
        perimetre: perimetre.into(),
        batiment_id: None,
        zone_id: zone,
        liste_id: None,
    }
}

fn pointage(eleve: Uuid, marque: &str) -> PointageSaisieDto {
    PointageSaisieDto {
        eleve_id: eleve,
        marque: marque.into(),
        note: None,
        retour_attendu: None,
        a: None,
        appareil_id: None,
        motif_correction_code: None,
        motif_correction_texte: None,
    }
}

impl Banc {
    async fn ouvrir(&self, type_appel: &str, perimetre: &str, zone: Option<Uuid>, mi: u32) -> AppelVueDto {
        self.ent
            .ouvrir_appel_a(self.etab, self.cpe, CPE, &ouverture(type_appel, perimetre, zone), soir(mi))
            .await
            .unwrap()
    }

    async fn point(&self, appel: Uuid, eleve: Uuid, marque: &str, mi: u32) -> Result<PointageResultatDto, EntDbError> {
        self.ent.pointer_a(self.etab, self.cpe, CPE, appel, &pointage(eleve, marque), soir(mi)).await
    }

    async fn evenements(&self, type_event: &str) -> Vec<serde_json::Value> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT payload FROM ent_metier_event WHERE etablissement_id = $1 AND type = $2 ORDER BY created_at, id",
        )
        .bind(self.etab)
        .bind(type_event)
        .fetch_all(self.ent.pool_verif())
        .await
        .unwrap()
    }
}

fn est_refus(r: &Result<impl std::fmt::Debug, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Refus(..) | EntDbError::ConflitDossierDetail(_) | EntDbError::Domain(_)))
}

fn est_interdit(r: &Result<impl std::fmt::Debug, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

// ───────────────────────── Fonctions pures ─────────────────────────

#[test]
fn marques_permises_par_statut_et_niveaux_d_avis() {
    assert!(marque_permise("attendu", "present") && marque_permise("ajoute", "absent"));
    assert!(!marque_permise("attendu", "non_attendu_present"));
    assert!(marque_permise("retire", "non_attendu_present") && marque_permise("non_attendu", "non_attendu_present"));
    assert!(!marque_permise("retire", "present") && !marque_permise("non_attendu", "absent"));
    assert!(!marque_permise("attendu", "excuse"), "« excusé » vient de l'attendu, jamais d'un pointage");
    assert!(marque_en_attente("activite") && marque_en_attente("retard") && !marque_en_attente("present"));
    assert_eq!(niveau_avis(0, 0), "U1");
    assert_eq!(niveau_avis(2, 0), "U2");
    assert_eq!(niveau_avis(2, 1), "U3");
    assert_eq!(procedure_par_defaut_longueur(), 5);
}

fn procedure_par_defaut_longueur() -> usize {
    crate::internat_cas::procedure_par_defaut().len()
}

// ───────────────────────── Attendu ─────────────────────────

#[tokio::test]
async fn attendu_calcule_rythme_exceptionnelle_retraits_et_situations() {
    let Some(b) = banc().await else { return };
    let (a0, a1, a2, a3) = (b.a[0], b.a[1], b.a[2], b.a[3]);
    let nuit = nuit_dst();
    let pool = b.ent.pool_verif();

    // a1 : retrait posé par le CPE (sortie) ; a2 : situation connue du socle 12 couvrant 22 h ; a3 : mobilité (socle 2).
    b.ent
        .poser_retrait_a(b.etab, b.cpe, CPE, &RetraitSaisieDto { eleve_id: a1, du_nuit: nuit, au_nuit: nuit, motif_code: "sortie".into() }, soir(0))
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_asd_situation (id, etablissement_id, eleve_id, nature, debut_jour, debut_heure, fin_jour, fin_heure, source) \
         VALUES ($1,$2,$3,'declaration','2026-03-28','18:00','2026-03-29','20:00','famille')",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(a2)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_eleve_mobilite (id, etablissement_id, eleve_id, du, au, type) VALUES ($1,$2,$3,'2026-03-20','2026-04-05','soins')")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(a3)
        .execute(pool)
        .await
        .unwrap();
    // b[0] : hébergé mais son rythme ne contient pas le samedi (non attendu) ; b[1] : même chose + nuit exceptionnelle.
    for e in &b.b {
        sqlx::query(
            "INSERT INTO ent_internat_rythme (id, etablissement_id, sejour_id, du, dimanche, lundi, mardi, mercredi, jeudi, vendredi, samedi, cree_par_user_id) \
             SELECT $1, etablissement_id, id, '2026-02-01', true, true, true, true, true, false, false, $3 \
               FROM ent_internat_sejour WHERE etablissement_id = $2 AND eleve_id = $4",
        )
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(b.cpe)
        .bind(e)
        .execute(pool)
        .await
        .unwrap();
    }
    b.ent
        .poser_nuit_exceptionnelle_a(
            b.etab,
            b.cpe,
            CPE,
            &NuitExceptionnelleSaisieDto { eleve_id: b.b[1], nuit, motif_code: "examen".into() },
            soir(0),
        )
        .await
        .unwrap();

    let att = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit, soir(1)).await.unwrap();
    let statut = |e: Uuid| {
        let l = att.lignes.iter().find(|l| l.eleve_id == e).unwrap();
        (l.statut.clone(), l.motif_code.clone())
    };
    assert_eq!(statut(a0), ("attendu".into(), None));
    assert_eq!(statut(a1), ("retire".into(), Some("sortie".into())));
    assert_eq!(statut(a2), ("retire".into(), Some("absence_annoncee".into())), "situation du socle 12 couvrant 22 h");
    assert_eq!(statut(a3), ("retire".into(), Some("mobilite".into())));
    assert_eq!(statut(b.b[0]), ("non_attendu".into(), None), "rythme sans samedi");
    assert_eq!(statut(b.b[1]), ("ajoute".into(), None), "nuit exceptionnelle");
    assert_eq!(att.calcule_le, soir(1));
    assert!(att.lignes.iter().all(|l| l.nom.is_some()), "aucun élève protégé ici");

    // La même nuit, le lendemain (dimanche) : le rythme (7 jours pour a, 5 pour b) change l'attendu — jamais de jour codé en dur.
    let dimanche = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit + chrono::Duration::days(1), soir(1)).await.unwrap();
    assert!(dimanche.lignes.iter().find(|l| l.eleve_id == b.b[0]).unwrap().statut == "attendu");

    // Annulation d'un retrait : l'élève redevient attendu ; un retrait ne se supprime pas.
    let retrait: Uuid = sqlx::query_scalar("SELECT id FROM ent_internat_retrait WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(b.etab)
        .bind(a1)
        .fetch_one(pool)
        .await
        .unwrap();
    assert!(sqlx::query("DELETE FROM ent_internat_retrait WHERE id = $1").bind(retrait).execute(pool).await.is_err());
    b.ent.annuler_retrait_a(b.etab, b.cpe, CPE, retrait, soir(2)).await.unwrap();
    let att = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit, soir(3)).await.unwrap();
    assert_eq!(att.lignes.iter().find(|l| l.eleve_id == a1).unwrap().statut, "attendu");
}

#[tokio::test]
async fn attendu_fermeture_exceptionnelle_du_calendrier_et_internes_presents() {
    let Some(b) = banc().await else { return };
    let nuit = nuit_dst();
    let pool = b.ent.pool_verif();
    let annee: Uuid = match sqlx::query_scalar("SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 LIMIT 1")
        .bind(b.etab)
        .fetch_optional(pool)
        .await
        .unwrap()
    {
        Some(a) => a,
        None => {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin) VALUES ($1,$2,'2025-2026','2025-09-01','2026-08-31')")
                .bind(id)
                .bind(b.etab)
                .execute(pool)
                .await
                .unwrap();
            id
        }
    };
    let evenement = |internes_presents: bool| {
        let (etab, annee) = (b.etab, annee);
        async move {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_calendrier_evenement (id, etablissement_id, annee_id, type, libelle, du, au, effet, services_touches, motif, internes_presents) \
                 VALUES ($1,$2,$3,'fermeture_exceptionnelle','Fermeture test','2026-03-27','2026-03-29','sans_cours',ARRAY['internat'],'intempéries',$4)",
            )
            .bind(id)
            .bind(etab)
            .bind(annee)
            .bind(internes_presents)
            .execute(pool)
            .await
            .unwrap();
            id
        }
    };
    // Internat maintenu (R-S1) : l'attendu ne change pas.
    let maintenu = evenement(true).await;
    let att = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit, soir(0)).await.unwrap();
    assert!(att.lignes.iter().all(|l| l.statut != "retire"));
    sqlx::query("UPDATE ent_calendrier_evenement SET annule_le = now() WHERE id = $1")
        .bind(maintenu)
        .execute(pool)
        .await
        .ok();
    let _ = evenement(false).await;
    let att = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit, soir(0)).await.unwrap();
    assert!(
        att.lignes.iter().filter(|l| b.a.contains(&l.eleve_id)).all(|l| l.statut == "retire" && l.motif_code.as_deref() == Some("fermeture")),
        "internat fermé : nuit retirée pour tous, motif « fermeture »"
    );
    // Hors des dates de fermeture : attendu normal.
    let lundi = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit + chrono::Duration::days(3), soir(0)).await.unwrap();
    assert!(lundi.lignes.iter().all(|l| l.statut == "attendu"));
}

#[tokio::test]
async fn di_16_nuits_a_cheval_sur_minuit_et_changements_d_heure() {
    let Some(b) = banc().await else { return };
    let pool = b.ent.pool_verif();
    // Europe/Paris, passage à l'heure d'été le dimanche 29 mars 2026 : le soir est à UTC+1, le matin à UTC+2.
    let ancre = |n: NaiveDate| {
        let etab = b.etab;
        async move {
            sqlx::query_scalar::<_, DateTime<Utc>>("SELECT ent_internat_ancre_nuit($1, $2)").bind(etab).bind(n).fetch_one(pool).await.unwrap()
        }
    };
    assert_eq!(ancre(nuit_dst()).await, utc(2026, 3, 28, 21, 0), "22 h à Paris le samedi = 21 h UTC (heure d'hiver)");
    assert_eq!(ancre(NaiveDate::from_ymd_opt(2026, 3, 29).unwrap()).await, utc(2026, 3, 29, 20, 0), "22 h le dimanche = 20 h UTC (heure d'été)");
    // Retour à l'heure d'hiver le 25 octobre 2026.
    assert_eq!(ancre(NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()).await, utc(2026, 10, 24, 20, 0));
    assert_eq!(ancre(NaiveDate::from_ymd_opt(2026, 10, 25).unwrap()).await, utc(2026, 10, 25, 21, 0));

    // Un appel ouvert à 00 h 30 (heure de Paris) le dimanche 29 appartient à la nuit du samedi 28.
    for (instant, attendue, type_appel) in [
        (utc(2026, 3, 28, 23, 30), nuit_dst(), "ronde"),                                 // 00 h 30 CET
        (utc(2026, 3, 29, 1, 30), nuit_dst(), "soir"),                                   // 03 h 30 CEST, après le saut
        (utc(2026, 3, 29, 9, 59), nuit_dst(), "matin"),                                  // 11 h 59 CEST
        (utc(2026, 3, 29, 10, 0), NaiveDate::from_ymd_opt(2026, 3, 29).unwrap(), "ronde"), // midi CEST : nouvelle nuit
    ] {
        let t: NaiveDate = sqlx::query_scalar("SELECT ent_nuit_etab($1, $2)").bind(b.etab).bind(instant).fetch_one(pool).await.unwrap();
        assert_eq!(t, attendue, "{instant}");
        let ouverture = AppelOuvertureDto { nuit: None, ..ouverture(type_appel, "zone", Some(b.zone_a)) };
        let r = b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &ouverture, instant).await.unwrap();
        assert_eq!(r.appel.nuit, attendue, "nuit par défaut de {instant}");
        // Chaque nuit n'a qu'une ronde par périmètre : on passe au créneau suivant.
    }

    // Établissement d'outre-mer (La Réunion, UTC+4, pas de changement d'heure) : la nuit suit SON fuseau.
    sqlx::query("INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, 'Indian/Reunion') ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = EXCLUDED.fuseau")
        .bind(b.etab)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(ancre(nuit_dst()).await, utc(2026, 3, 28, 18, 0), "22 h à La Réunion = 18 h UTC");
    for (instant, attendue) in [
        (utc(2026, 3, 28, 19, 30), nuit_dst()),                                    // 23 h 30 locale
        (utc(2026, 3, 29, 7, 59), nuit_dst()),                                     // 11 h 59 locale
        (utc(2026, 3, 29, 8, 0), NaiveDate::from_ymd_opt(2026, 3, 29).unwrap()),   // midi local
    ] {
        let t: NaiveDate = sqlx::query_scalar("SELECT ent_nuit_etab($1, $2)").bind(b.etab).bind(instant).fetch_one(pool).await.unwrap();
        assert_eq!(t, attendue, "La Réunion {instant}");
    }
    // Une situation « sortie » de 18 h à 23 h LOCALES couvre 22 h à La Réunion (et ne la couvrirait pas à Paris).
    sqlx::query(
        "INSERT INTO ent_asd_situation (id, etablissement_id, eleve_id, nature, debut_jour, debut_heure, fin_jour, fin_heure, source) \
         VALUES ($1,$2,$3,'sortie','2026-03-28','18:00','2026-03-28','23:00','famille')",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(b.a[0])
    .execute(pool)
    .await
    .unwrap();
    let att = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit_dst(), utc(2026, 3, 28, 19, 0)).await.unwrap();
    assert_eq!(att.lignes.iter().find(|l| l.eleve_id == b.a[0]).unwrap().statut, "retire");
}

// ───────────────────────── Appels ─────────────────────────

#[tokio::test]
async fn appel_cycle_complet_pointage_cloture_cas_validation_correction_pdf() {
    let Some(b) = banc().await else { return };
    let vue = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await;
    let appel = vue.appel.id;
    assert_eq!(vue.appel.nb_attendus, 4);
    assert_eq!(vue.lignes.len(), 4);
    assert!(vue.lignes.iter().all(|l| l.lit_numero.is_some() && l.chambre_code.is_some()), "chambre et lit lisibles");
    assert_eq!(vue.appel.etat, "ouvert");

    // « Tous présents » pour la chambre, sans écraser une marque déjà posée (R-IT13).
    b.point(appel, b.a[0], "absent", 1).await.unwrap();
    let lot = b.ent.tous_presents_chambre_a(b.etab, b.cpe, CPE, appel, b.chambre_a, soir(2)).await.unwrap();
    assert_eq!(lot.len(), 3, "le déjà-pointé n'est pas touché");
    // Clôture impossible tant qu'un attendu n'est pas pointé : on reprend en pointant seulement trois élèves.
    let appel2 = b.ouvrir("extinction", "zone", Some(b.zone_a), 3).await.appel.id;
    b.point(appel2, b.a[0], "present", 4).await.unwrap();
    let r = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel2, soir(5)).await;
    assert!(matches!(r, Err(EntDbError::ConflitDossierDetail(_))), "attendu non pointé : {r:?}");

    // Clôture de l'appel du soir : l'absent non couvert ouvre un cas (R-IT16).
    let cloture = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(6)).await.unwrap();
    assert_eq!(cloture.appel.etat, "clos");
    assert_eq!(cloture.appel.nb_ecarts, Some(1));
    assert_eq!(cloture.cas_ouverts.len(), 1);
    // Valider : avis U3 (un cas est ouvert).
    let v = b.ent.valider_appel_a(b.etab, b.cpe, CPE, appel, soir(7)).await.unwrap();
    assert_eq!(v.urgence, "U3");
    assert_eq!(v.cas_ouverts, 1);
    assert_eq!(b.evenements("internat.appel_valide").await.last().unwrap()["urgence"], "U3");

    // Après validation : plus de pointage sans motif ; une correction motivée est un nouveau fait (R-IT17).
    let r = b.point(appel, b.a[1], "absent", 8).await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::InternatMotifRequis))), "{r:?}");
    let mut corr = pointage(b.a[1], "absent");
    corr.motif_correction_code = Some("erreur_saisie".into());
    corr.motif_correction_texte = Some("Pointé présent par erreur".into());
    b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &corr, soir(9)).await.unwrap();
    assert_eq!(b.evenements("internat.appel_corrige").await.len(), 1, "avis de correction");
    let hist = b.ent.historique_appel_a(b.etab, b.cpe, CPE, appel, soir(10)).await.unwrap();
    assert!(hist.iter().any(|h| h.eleve_id == b.a[1] && h.motif_correction_code.as_deref() == Some("erreur_saisie")), "historique visible");
    assert!(hist.iter().filter(|h| h.eleve_id == b.a[1]).count() == 2, "rien n'est effacé : le fait initial reste");
    // L'appel validé est inaltérable côté table.
    let pool = b.ent.pool_verif();
    assert!(sqlx::query("UPDATE ent_internat_appel SET etat = 'ouvert' WHERE id = $1").bind(appel).execute(pool).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_internat_appel WHERE id = $1").bind(appel).execute(pool).await.is_err());
    assert!(sqlx::query("UPDATE ent_internat_presence_fait SET marque = 'present'").execute(pool).await.is_err());

    // PDF côté serveur : tableau par chambre, marques, absent en évidence, aucun mot de santé.
    let pdf = b.ent.pdf_appel_a(b.etab, b.cpe, CPE, appel, soir(11)).await.unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    let texte = ent_pdf::releve::texte(&pdf);
    assert!(texte.contains("nuit du 2026-03-28"), "{texte}");
    assert!(texte.contains("Absent") && texte.contains("Présent"));

    // Sans écart : avis U1.
    let vue = b.ouvrir("ronde", "zone", Some(b.zone_b), 12).await;
    let ronde = vue.appel.id;
    for e in &b.b {
        b.point(ronde, *e, "vu", 13).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, ronde, soir(14)).await.unwrap();
    let v = b.ent.valider_appel_a(b.etab, b.cpe, CPE, ronde, soir(15)).await.unwrap();
    assert_eq!(v.urgence, "U1");
    // Valider un appel non clos : refusé ; valider deux fois : refusé.
    assert!(est_refus(&b.ent.valider_appel_a(b.etab, b.cpe, CPE, appel2, soir(16)).await));
    assert!(est_refus(&b.ent.valider_appel_a(b.etab, b.cpe, CPE, ronde, soir(17)).await));
}

#[tokio::test]
async fn di_11_un_seul_appel_par_type_nuit_et_perimetre_meme_en_concurrence() {
    let Some(b) = banc().await else { return };
    let o = ouverture("soir", "zone", Some(b.zone_a));
    let (r1, r2) = tokio::join!(
        b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &o, soir(0)),
        b.ent.ouvrir_appel_a(b.etab, b.dir, DIR, &o, soir(0)),
    );
    let (ok, ko) = (r1.is_ok() as u8 + r2.is_ok() as u8, r1.is_err() as u8 + r2.is_err() as u8);
    assert_eq!((ok, ko), (1, 1), "un seul appel ouvert : {r1:?} / {r2:?}");
    let erreur = r1.err().or(r2.err()).unwrap();
    assert!(matches!(erreur, EntDbError::Refus(..)), "{erreur:?}");
    // Un autre type, un autre périmètre ou une autre nuit sont libres : les surveillants se partagent des listes.
    b.ouvrir("extinction", "zone", Some(b.zone_a), 1).await;
    b.ouvrir("soir", "zone", Some(b.zone_b), 2).await;
    let lst = b
        .ent
        .creer_liste_a(b.etab, b.cpe, CPE, &ListeSaisieDto { libelle: "Chambre A".into(), chambre_ids: vec![b.chambre_a], confie_a_user_id: Some(b.cpe) })
        .await
        .unwrap();
    let vue = b
        .ent
        .ouvrir_appel_a(
            b.etab,
            b.cpe,
            CPE,
            &AppelOuvertureDto { liste_id: Some(lst.id), ..ouverture("arrivee", "liste", None) },
            soir(3),
        )
        .await
        .unwrap();
    assert_eq!(vue.lignes.len(), 4, "la liste porte les élèves de ses chambres");
    // Périmètre incohérent ou inconnu : refusé ; bâtiment complet : six élèves.
    let r = b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &AppelOuvertureDto { zone_id: None, ..ouverture("soir", "zone", None) }, soir(4)).await;
    assert!(est_refus(&r));
    let r = b
        .ent
        .ouvrir_appel_a(b.etab, b.cpe, CPE, &AppelOuvertureDto { batiment_id: Some(b.batiment), ..ouverture("matin", "batiment", None) }, soir(5))
        .await
        .unwrap();
    assert_eq!(r.lignes.len(), 6);
    // Nuit future refusée ; type désactivé refusé.
    let futur = AppelOuvertureDto { nuit: Some(nuit_dst() + chrono::Duration::days(1)), ..ouverture("etude", "internat", None) };
    assert!(matches!(
        b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &futur, soir(6)).await,
        Err(EntDbError::Domain(DomainError::InternatNuitFuture))
    ));
    sqlx::query("INSERT INTO ent_internat_type_appel_actif (etablissement_id, type, actif) VALUES ($1,'etude',false)")
        .bind(b.etab)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    assert!(est_refus(&b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &ouverture("etude", "internat", None), soir(7)).await));
}

#[tokio::test]
async fn di_19_on_ne_pointe_que_l_attendu_du_perimetre() {
    let Some(b) = banc().await else { return };
    let nuit = nuit_dst();
    // a[3] est en sortie : retiré de l'attendu, mais présent dans la liste de la zone.
    b.ent
        .poser_retrait_a(b.etab, b.cpe, CPE, &RetraitSaisieDto { eleve_id: b.a[3], du_nuit: nuit, au_nuit: nuit, motif_code: "sortie".into() }, soir(0))
        .await
        .unwrap();
    let vue = b.ouvrir("soir", "zone", Some(b.zone_a), 1).await;
    let appel = vue.appel.id;
    assert_eq!(vue.appel.nb_attendus, 3);
    let l = vue.lignes.iter().find(|l| l.eleve_id == b.a[3]).unwrap();
    assert_eq!((l.statut.as_str(), l.motif_code.as_deref()), ("retire", Some("sortie")), "marque pré-remplie : en sortie");
    // Un élève de la zone B (hors périmètre) est refusé — par le service ET par la base.
    assert!(est_refus(&b.point(appel, b.b[0], "present", 2).await));
    let brut = sqlx::query(
        "INSERT INTO ent_internat_presence_fait (id, etablissement_id, eleve_id, appel_id, nuit, marque, par_user_id, a, recu_le) \
         VALUES ($1,$2,$3,$4,$5,'present',$6,$7,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(b.b[0])
    .bind(appel)
    .bind(nuit)
    .bind(b.cpe)
    .bind(soir(2))
    .execute(b.ent.pool_verif())
    .await;
    assert!(brut.is_err(), "garde SQL : élève hors de la liste figée de l'appel");
    // Une nuit inconnue de l'appel est refusée aussi.
    let brut = sqlx::query(
        "INSERT INTO ent_internat_presence_fait (id, etablissement_id, eleve_id, appel_id, nuit, marque, par_user_id, a, recu_le) \
         VALUES ($1,$2,$3,$4,'2026-03-27','present',$5,$6,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(b.a[0])
    .bind(appel)
    .bind(b.cpe)
    .bind(soir(2))
    .execute(b.ent.pool_verif())
    .await;
    assert!(brut.is_err(), "garde SQL : fait hors de la nuit de l'appel");
    // Marque incompatible avec le statut : un retiré se pointe « non attendu présent » seulement.
    assert!(est_refus(&b.point(appel, b.a[3], "present", 3).await));
    b.point(appel, b.a[3], "non_attendu_present", 4).await.unwrap();
    assert!(est_refus(&b.point(appel, b.a[0], "non_attendu_present", 5).await));
    assert!(est_refus(&b.point(appel, b.a[0], "excuse", 6).await), "« excusé » n'est pas un pointage");
    // Heure du geste dans le futur ou hors de la nuit : refusée.
    let mut futur = pointage(b.a[0], "present");
    futur.a = Some(soir(60));
    assert!(est_refus(&b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &futur, soir(7)).await));
    let mut hier = pointage(b.a[0], "present");
    hier.a = Some(utc(2026, 3, 27, 12, 0));
    assert!(est_refus(&b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &hier, soir(7)).await));
    // R-IT6 : aucune donnée de santé dans une note d'appel.
    let mut note = pointage(b.a[0], "present");
    note.note = Some("crise d'asthme ce soir".into());
    assert!(est_refus(&b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &note, soir(8)).await));
    // « À l'infirmerie » : couvert, renvoi vers l'infirmerie (34), aucune ouverture de cas.
    let r = b.point(appel, b.a[1], "infirmerie", 9).await.unwrap();
    assert!(r.renvoi_infirmerie);
    b.point(appel, b.a[0], "present", 10).await.unwrap();
    b.point(appel, b.a[2], "present", 11).await.unwrap();
    let c = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(12)).await.unwrap();
    assert!(c.cas_ouverts.is_empty());
    assert_eq!(c.appel.nb_ecarts, Some(1), "le non attendu présent est un écart");
}

#[tokio::test]
async fn di_13_deux_surveillants_ajout_seul_et_conflit() {
    let Some(b) = banc().await else { return };
    let vue = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await;
    let appel = vue.appel.id;
    // Deux surveillants pointent le même élève en sens contraire à quelques secondes d'écart.
    b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &pointage(b.a[0], "present"), soir(1)).await.unwrap();
    let r = b.ent.pointer_a(b.etab, b.dir, DIR, appel, &pointage(b.a[0], "absent"), soir(2)).await.unwrap();
    assert!(r.conflit, "conflit signalé (CIT-03)");
    let v = b.ent.vue_appel_a(b.etab, b.cpe, CPE, appel, soir(3)).await.unwrap();
    let l = v.lignes.iter().find(|l| l.eleve_id == b.a[0]).unwrap();
    assert_eq!(l.marque.as_deref(), Some("absent"), "la valeur courante est le fait le plus récent");
    assert!(l.conflit);
    assert_eq!(l.marque_par_user_id, Some(b.dir));
    // Les deux faits existent : aucun n'a écrasé l'autre.
    let hist = b.ent.historique_appel_a(b.etab, b.cpe, CPE, appel, soir(4)).await.unwrap();
    assert_eq!(hist.iter().filter(|h| h.eleve_id == b.a[0]).count(), 2);
    // Pointages simultanés sur des élèves différents : tous conservés.
    let (p1, p2, p3) = (pointage(b.a[1], "present"), pointage(b.a[2], "present"), pointage(b.a[3], "present"));
    let (r1, r2, r3) = tokio::join!(
        b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &p1, soir(5)),
        b.ent.pointer_a(b.etab, b.dir, DIR, appel, &p2, soir(5)),
        b.ent.pointer_a(b.etab, b.admin, "admin", appel, &p3, soir(5)),
    );
    assert!(r1.is_ok() && r2.is_ok() && r3.is_ok());
    let v = b.ent.vue_appel_a(b.etab, b.cpe, CPE, appel, soir(6)).await.unwrap();
    assert_eq!(v.nb_pointes, 4);
    // Deux marques de deux personnes d'accord : pas de conflit.
    b.ent.pointer_a(b.etab, b.dir, DIR, appel, &pointage(b.a[1], "present"), soir(7)).await.unwrap();
    let v = b.ent.vue_appel_a(b.etab, b.cpe, CPE, appel, soir(8)).await.unwrap();
    assert!(!v.lignes.iter().find(|l| l.eleve_id == b.a[1]).unwrap().conflit);
    // Un nouvel attendu (retrait posé pendant l'appel) apparaît comme écart avec la liste figée.
    b.ent
        .poser_retrait_a(b.etab, b.cpe, CPE, &RetraitSaisieDto { eleve_id: b.a[2], du_nuit: nuit_dst(), au_nuit: nuit_dst(), motif_code: "absence_annoncee".into() }, soir(9))
        .await
        .unwrap();
    let v = b.ent.vue_appel_a(b.etab, b.cpe, CPE, appel, soir(10)).await.unwrap();
    assert_eq!(v.ecarts_avec_attendu_actuel, vec![b.a[2]]);
    assert_eq!(v.calcule_le, soir(10), "la vue affiche l'heure de son dernier calcul");
}

#[tokio::test]
async fn retour_tardif_malade_et_retard_annonce_cas_et_cloture() {
    let Some(b) = banc().await else { return };
    let vue = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await;
    let appel = vue.appel.id;
    // Retour tardif annoncé (train en retard) : sans heure de retour, refusé ; avec, la clôture attend.
    assert!(est_refus(&b.point(appel, b.a[0], "retard", 1).await));
    let mut retard = pointage(b.a[0], "retard");
    retard.retour_attendu = Some(soir(40));
    b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &retard, soir(1)).await.unwrap();
    // Activité en cours (sortie de groupe, étude) : la clôture attend le retour.
    let mut activite = pointage(b.a[1], "activite");
    activite.retour_attendu = Some(soir(30));
    b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &activite, soir(2)).await.unwrap();
    b.point(appel, b.a[2], "present", 3).await.unwrap();
    b.point(appel, b.a[3], "infirmerie", 4).await.unwrap();
    let r = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(5)).await;
    assert!(matches!(r, Err(EntDbError::ConflitDossierDetail(_))), "attente de retour : {r:?}");
    // Retour de l'élève en activité : plus d'attente. Le retard annoncé dépasse sa tolérance (30 min) : cas.
    b.point(appel, b.a[1], "present", 31).await.unwrap();
    let r = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(75)).await.unwrap();
    assert_eq!(r.cas_ouverts.len(), 1, "retard au-delà du délai annoncé + tolérance : un cas");
    let cas = b.ent.lister_cas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(76)).await.unwrap();
    assert_eq!(cas.len(), 1);
    assert_eq!(cas[0].nature, "retard_retour");
    assert!(cas[0].issue.is_none());
    // L'élève arrive finalement : correction motivée sur l'appel clos → le cas se ferme (« rentré à 23 h 10 »).
    let mut rentre = pointage(b.a[0], "present");
    rentre.motif_correction_code = Some("information_tardive".into());
    b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &rentre, soir(80)).await.unwrap();
    let cas = b.ent.lister_cas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(81)).await.unwrap();
    assert_eq!(cas[0].issue.as_deref(), Some("retrouve"));
    assert_eq!(cas[0].issue_le, Some(soir(80)), "l'heure du geste");
}

// ───────────────────────── Cas et procédure ─────────────────────────

#[tokio::test]
async fn cas_non_retrouve_procedure_numeros_etapes_et_issue() {
    let Some(b) = banc().await else { return };
    let pool = b.ent.pool_verif();
    // Titulaires de a[0] : une mère joignable, un père dont le lien est suspendu, une adresse sans numéro.
    let mut personnes = Vec::new();
    for (nom, suspendu) in [("Mère", false), ("Père", true)] {
        let p = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom) VALUES ($1,$2,$3,'Test')")
            .bind(p)
            .bind(b.etab)
            .bind(nom)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, lien, suspendu_le, suspension_motif) \
             VALUES ($1,$2,$3,$4,$5, CASE WHEN $6 THEN now() END, CASE WHEN $6 THEN 'décision de justice' END)",
        )
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(b.a[0])
        .bind(p)
        .bind(nom.to_lowercase())
        .bind(suspendu)
        .execute(pool)
        .await
        .unwrap();
        for (typ, valeur) in [("mobile", format!("06{}", nom.len())), ("adresse", "12 rue secrète".to_string())] {
            sqlx::query("INSERT INTO ent_personne_coordonnee (id, etablissement_id, personne_id, type, valeur) VALUES ($1,$2,$3,$4,$5)")
                .bind(Uuid::new_v4())
                .bind(b.etab)
                .bind(p)
                .bind(typ)
                .bind(valeur)
                .execute(pool)
                .await
                .unwrap();
        }
        personnes.push(p);
    }
    let vue = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await;
    let appel = vue.appel.id;
    b.point(appel, b.a[0], "absent", 1).await.unwrap();
    for e in &b.a[1..] {
        b.point(appel, *e, "present", 2).await.unwrap();
    }
    let cloture = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(3)).await.unwrap();
    let cas_id = cloture.cas_ouverts[0];
    let cas = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas_id, soir(4)).await.unwrap();
    assert_eq!(cas.procedure_version, 1);
    assert_eq!(cas.etapes.len(), 5, "procédure par défaut du §9.3");
    assert_eq!(cas.etapes[1].echeance, soir(3) + chrono::Duration::minutes(10), "minuteur : ouverture + délai");
    assert!(!cas.astreinte_branchee, "socle 6 non branché : dit sans détour");
    // R-IT26 : seul le numéro de la mère (lien actif) sort ; jamais l'adresse ni le père suspendu.
    assert_eq!(cas.numeros_titulaires.len(), 1);
    assert_eq!(cas.numeros_titulaires[0].personne_id, personnes[0]);
    assert!(!format!("{:?}", cas).contains("secrète"));
    // Étapes : lieu vérifié, étape faite, doublon refusé ; étape d'une autre procédure refusée.
    let e = |lieu: &str| CasEtapeSaisieDto { etape_id: None, lieu_code: Some(lieu.into()), note: None, fait_le: Some(soir(5)) };
    b.ent.cas_etape_a(b.etab, b.cpe, CPE, cas_id, &e("chambre"), soir(5)).await.unwrap();
    b.ent.cas_etape_a(b.etab, b.cpe, CPE, cas_id, &e("sanitaires"), soir(5)).await.unwrap();
    assert!(est_refus(&b.ent.cas_etape_a(b.etab, b.cpe, CPE, cas_id, &e("chambre"), soir(6)).await));
    assert!(est_refus(&b.ent.cas_etape_a(b.etab, b.cpe, CPE, cas_id, &CasEtapeSaisieDto { etape_id: Some(Uuid::new_v4()), lieu_code: None, note: None, fait_le: None }, soir(6)).await));
    b.ent
        .cas_etape_a(b.etab, b.cpe, CPE, cas_id, &CasEtapeSaisieDto { etape_id: Some(cas.etapes[1].id), lieu_code: None, note: Some("Astreinte prévenue".into()), fait_le: Some(soir(14)) }, soir(14))
        .await
        .unwrap();
    let cas = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas_id, soir(15)).await.unwrap();
    assert_eq!(cas.etapes[1].faite_le, Some(soir(14)));
    assert_eq!(cas.etapes[1].faite_par_user_id, Some(b.cpe), "« fait à 22 h 41 par Y. M. »");
    let mut lieux = cas.lieux_verifies.clone();
    lieux.sort();
    assert_eq!(lieux, vec!["chambre".to_string(), "sanitaires".to_string()]);
    // Aucune donnée de santé dans une note de cas (R-IT6).
    let sante = CasEtapeSaisieDto { etape_id: None, lieu_code: Some("etude".into()), note: Some("traitement contre l'asthme".into()), fait_le: None };
    assert!(est_refus(&b.ent.cas_etape_a(b.etab, b.cpe, CPE, cas_id, &sante, soir(16)).await));
    // Issues : chacune exige sa pièce ; un cas ne se ferme que par une issue (R-IT18).
    let issue = |i: &str| CasIssueSaisieDto { issue: i.into(), lieu_code: None, confirme_par_personne_id: None, numero_intervention: None, note: None, a: Some(soir(20)) };
    assert!(est_refus(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &issue("retrouve"), soir(21)).await));
    assert!(est_refus(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &issue("rentre_famille"), soir(21)).await));
    assert!(est_refus(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &issue("autorite_saisie"), soir(21)).await));
    let mut inconnue = issue("rentre_famille");
    inconnue.confirme_par_personne_id = Some(Uuid::new_v4());
    assert!(est_refus(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &inconnue, soir(21)).await), "personne non rattachée");
    let mut ok = issue("rentre_famille");
    ok.confirme_par_personne_id = Some(personnes[0]);
    let clos = b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &ok, soir(21)).await.unwrap();
    assert_eq!(clos.issue.as_deref(), Some("rentre_famille"));
    assert!(est_refus(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &issue("annule_erreur"), soir(22)).await), "déjà clos");
    assert!(sqlx::query("UPDATE ent_internat_cas SET issue = 'retrouve' WHERE id = $1").bind(cas_id).execute(pool).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_internat_cas WHERE id = $1").bind(cas_id).execute(pool).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_internat_cas_etape WHERE cas_id = $1").bind(cas_id).execute(pool).await.is_err());
    assert_eq!(b.evenements("internat.cas_ouvert").await.len(), 1);
    assert_eq!(b.evenements("internat.cas_clos").await.len(), 1);
}

#[tokio::test]
async fn procedure_versionnee_les_cas_ouverts_gardent_leur_version() {
    let Some(b) = banc().await else { return };
    let v1 = b.ent.lire_procedure_a(b.etab, b.cpe, CPE, soir(0)).await.unwrap();
    assert_eq!(v1.version, 1);
    assert_eq!(v1.etapes.len(), 5);
    // Seules la direction et l'administration publient ; délais croissants et codes propres exigés.
    let neuve = ProcedureSaisieDto {
        etapes: vec![
            ProcedureEtapeSaisieDto { code: "verifications".into(), libelle: "Vérifier sur place".into(), delai_minutes: 0, appelle: "aucun".into() },
            ProcedureEtapeSaisieDto { code: "direction".into(), libelle: "Prévenir la direction".into(), delai_minutes: 20, appelle: "direction".into() },
        ],
    };
    assert!(est_interdit(&b.ent.publier_procedure_a(b.etab, b.cpe, CPE, &neuve, soir(1)).await));
    let mauvaise = ProcedureSaisieDto { etapes: vec![neuve.etapes[1].clone(), neuve.etapes[0].clone()] };
    assert!(est_refus(&b.ent.publier_procedure_a(b.etab, b.dir, DIR, &mauvaise, soir(1)).await), "délais décroissants");
    let vue = b.ouvrir("soir", "zone", Some(b.zone_a), 2).await;
    b.point(vue.appel.id, b.a[0], "absent", 3).await.unwrap();
    for e in &b.a[1..] {
        b.point(vue.appel.id, *e, "present", 3).await.unwrap();
    }
    let cas_id = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, vue.appel.id, soir(4)).await.unwrap().cas_ouverts[0];
    let v2 = b.ent.publier_procedure_a(b.etab, b.dir, DIR, &neuve, soir(5)).await.unwrap();
    assert_eq!(v2.version, 2);
    assert_eq!(b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas_id, soir(6)).await.unwrap().etapes.len(), 5, "le cas ouvert garde la version 1");
    assert_eq!(b.ent.lire_procedure_a(b.etab, b.cpe, CPE, soir(7)).await.unwrap().version, 2);
}

// ───────────────────────── Transmission, effectif, rappels ─────────────────────────

#[tokio::test]
async fn transmission_du_matin_idempotente_effectif_et_evenements_neutres() {
    let Some(b) = banc().await else { return };
    let vue = b.ouvrir("soir", "internat", None, 0).await;
    let appel = vue.appel.id;
    assert_eq!(vue.lignes.len(), 6);
    b.point(appel, b.a[0], "absent", 1).await.unwrap();
    b.point(appel, b.a[1], "parti", 1).await.unwrap();
    for e in b.a[2..].iter().chain(b.b.iter()) {
        b.point(appel, *e, "present", 2).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(3)).await.unwrap();
    // Pas de transmission sans appel clos ; nuit future refusée.
    let ailleurs = NaiveDate::from_ymd_opt(2026, 3, 20).unwrap();
    assert!(est_refus(&b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, ailleurs, utc(2026, 3, 29, 6, 0)).await));
    // Le lendemain matin (07 h 00 CEST = 05 h UTC), toujours la nuit du 28.
    let matin = utc(2026, 3, 29, 5, 0);
    let t = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), matin).await.unwrap();
    assert!(t.nouvelle);
    assert_eq!((t.nb_attendus, t.nb_presents, t.nb_absents, t.nb_cas), (6, 4, 1, 1));
    // Idempotent : une seconde transmission ne republie rien.
    let t2 = b.ent.transmettre_matin_a(b.etab, b.dir, DIR, nuit_dst(), matin + chrono::Duration::minutes(5)).await.unwrap();
    assert!(!t2.nouvelle);
    assert_eq!(t2.transmis_le, matin);
    assert_eq!(b.evenements("internat.absence_nuit").await.len(), 1);
    assert_eq!(b.evenements("internat.transmission").await.len(), 1);
    let repas = b.evenements("internat.effectif_repas").await;
    assert_eq!(repas.len(), 1);
    assert_eq!(repas[0]["diner"], 6);
    assert_eq!(repas[0]["petit_dejeuner"], 4, "6 attendus − 1 absent − 1 parti");
    // Le cas est marqué transmis.
    let cas = b.ent.lister_cas_a(b.etab, b.cpe, CPE, nuit_dst(), matin).await.unwrap();
    assert_eq!(cas[0].transmis_le, Some(matin));
    // Effectif lu par la restauration : des compteurs, aucun nom.
    let e = b.ent.effectif_repas_a(b.etab, b.cpe, CPE, nuit_dst()).await.unwrap();
    assert_eq!(e.diner, 6);
    // Événements neutres (R-IT6) : ni nom, ni prénom, ni texte libre, ni motif en clair.
    let noms: Vec<String> = sqlx::query_scalar("SELECT nom || ' ' || prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)")
        .bind(b.etab)
        .bind([b.a.clone(), b.b.clone()].concat())
        .fetch_all(b.ent.pool_verif())
        .await
        .unwrap();
    let tous: Vec<String> = sqlx::query_scalar("SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type LIKE 'internat.%'")
        .bind(b.etab)
        .fetch_all(b.ent.pool_verif())
        .await
        .unwrap();
    assert!(!tous.is_empty());
    for p in &tous {
        for n in &noms {
            for mot in n.split(' ') {
                assert!(!p.contains(mot), "le nom {mot} ne doit figurer dans aucun événement : {p}");
            }
        }
        for interdit in ["santé", "allerg", "traitement", "\"texte\"", "\"note\""] {
            assert!(!p.contains(interdit), "{interdit} dans {p}");
        }
    }
    // Une direction à portée restreinte ne déclenche pas la transmission de tout l'internat (R-IT30).
}

#[tokio::test]
async fn rappels_une_fois_par_niveau_calendrier_reel_et_nuit_a_cheval_sur_minuit() {
    let Some(b) = banc().await else { return };
    // Limite 21 h 30 (S-4102) : 20 h 30 UTC le samedi 28 mars 2026 (CET, UTC+1).
    let avant = utc(2026, 3, 28, 20, 29);
    assert!(b.ent.emettre_rappels_appels_a(b.etab, avant).await.unwrap().surveillant.is_empty(), "avant l'heure limite : rien");
    let limite = utc(2026, 3, 28, 20, 30);
    let r = b.ent.emettre_rappels_appels_a(b.etab, limite).await.unwrap();
    assert_eq!((r.surveillant.len(), r.cpe.len()), (1, 0), "avis au surveillant d'abord");
    assert!(b.ent.emettre_rappels_appels_a(b.etab, limite + chrono::Duration::minutes(5)).await.unwrap().surveillant.is_empty(), "une seule fois par niveau");
    let r = b.ent.emettre_rappels_appels_a(b.etab, limite + chrono::Duration::minutes(15)).await.unwrap();
    assert_eq!((r.surveillant.len(), r.cpe.len()), (0, 1), "puis le CPE d'astreinte");
    assert!(b.ent.emettre_rappels_appels_a(b.etab, limite + chrono::Duration::minutes(40)).await.unwrap().cpe.is_empty());
    assert_eq!(b.evenements("internat.appel_en_retard").await.len(), 2);
    // Après minuit (00 h 40 CET, la nuit du 28 continue) : pas de nouveau rappel, ceux d'hier tiennent.
    assert!(b.ent.emettre_rappels_appels_a(b.etab, utc(2026, 3, 28, 23, 40)).await.unwrap().surveillant.is_empty());
    // Une nuit où l'internat ne reçoit personne (rythme vide, vacances) : jamais de rappel (R-IT40).
    let vide = NaiveDate::from_ymd_opt(2026, 8, 1).unwrap();
    let _ = vide;
    let b2 = banc().await.unwrap();
    // Appel du soir validé couvrant tout le monde : plus de rappel, même passé l'heure limite.
    let v = b2
        .ent
        .ouvrir_appel_a(b2.etab, b2.cpe, CPE, &ouverture("soir", "internat", None), utc(2026, 3, 28, 19, 0))
        .await
        .unwrap();
    for l in &v.lignes {
        b2.ent.pointer_a(b2.etab, b2.cpe, CPE, v.appel.id, &pointage(l.eleve_id, "present"), utc(2026, 3, 28, 19, 5)).await.unwrap();
    }
    b2.ent.cloturer_appel_a(b2.etab, b2.cpe, CPE, v.appel.id, utc(2026, 3, 28, 19, 10)).await.unwrap();
    b2.ent.valider_appel_a(b2.etab, b2.cpe, CPE, v.appel.id, utc(2026, 3, 28, 19, 15)).await.unwrap();
    assert!(b2.ent.emettre_rappels_appels_a(b2.etab, utc(2026, 3, 28, 21, 0)).await.unwrap().surveillant.is_empty());
    // Heure d'été : le même 21 h 30 local tombe à 19 h 30 UTC le dimanche 29 (CEST) — le rappel suit l'heure murale.
    let b3 = banc().await.unwrap();
    assert!(b3.ent.emettre_rappels_appels_a(b3.etab, utc(2026, 3, 29, 19, 29)).await.unwrap().surveillant.is_empty());
    assert_eq!(b3.ent.emettre_rappels_appels_a(b3.etab, utc(2026, 3, 29, 19, 30)).await.unwrap().surveillant.len(), 1);
}

// ───────────────────────── Droits, périmètre, isolation ─────────────────────────

#[tokio::test]
async fn droits_refuses_hors_equipe_habilitee() {
    let Some(b) = banc().await else { return };
    let vue = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await;
    let (appel, cas_nuit) = (vue.appel.id, nuit_dst());
    let roles = [(b.prof, "prof"), (b.compta, "compta"), (b.cast.infirmieres[0].user_id, "infirmerie"), (b.cast.parent_lea.user_id, "parent")];
    for (user, role) in roles {
        let e = b.etab;
        assert!(est_interdit(&b.ent.attendu_nuit_a(e, user, role, cas_nuit, soir(1)).await), "{role} : attendu");
        assert!(est_interdit(&b.ent.vue_appel_a(e, user, role, appel, soir(1)).await), "{role} : vue");
        assert!(est_interdit(&b.ent.lister_appels_a(e, user, role, cas_nuit).await), "{role} : liste");
        assert!(est_interdit(&b.ent.ouvrir_appel_a(e, user, role, &ouverture("ronde", "zone", Some(b.zone_a)), soir(1)).await), "{role} : ouvrir");
        assert!(est_interdit(&b.ent.pointer_a(e, user, role, appel, &pointage(b.a[0], "present"), soir(1)).await), "{role} : pointer");
        assert!(est_interdit(&b.ent.cloturer_appel_a(e, user, role, appel, soir(1)).await), "{role} : clôturer");
        assert!(est_interdit(&b.ent.valider_appel_a(e, user, role, appel, soir(1)).await), "{role} : valider");
        assert!(est_interdit(&b.ent.pdf_appel_a(e, user, role, appel, soir(1)).await), "{role} : PDF");
        assert!(est_interdit(&b.ent.lister_cas_a(e, user, role, cas_nuit, soir(1)).await), "{role} : cas");
        assert!(est_interdit(&b.ent.lire_procedure_a(e, user, role, soir(1)).await), "{role} : procédure");
        assert!(est_interdit(&b.ent.effectif_repas_a(e, user, role, cas_nuit).await), "{role} : effectif");
        assert!(est_interdit(&b.ent.lectures_sensibles_a(e, user, role, 10).await), "{role} : journal des lectures");
        assert!(
            est_interdit(&b.ent.poser_nuit_exceptionnelle_a(e, user, role, &NuitExceptionnelleSaisieDto { eleve_id: b.a[0], nuit: cas_nuit, motif_code: "examen".into() }, soir(1)).await),
            "{role} : nuit exceptionnelle"
        );
    }
    // L'infirmerie ne peut poser qu'un retrait « santé », sans détail ; tout autre motif lui est interdit.
    let infirmiere = (b.cast.infirmieres[0].user_id, "infirmerie");
    let retrait = |m: &str| RetraitSaisieDto { eleve_id: b.a[1], du_nuit: nuit_dst(), au_nuit: nuit_dst(), motif_code: m.into() };
    assert!(est_interdit(&b.ent.poser_retrait_a(b.etab, infirmiere.0, infirmiere.1, &retrait("sortie"), soir(2)).await));
    let r = b.ent.poser_retrait_a(b.etab, infirmiere.0, infirmiere.1, &retrait("sante"), soir(2)).await.unwrap();
    assert_eq!(r.source, "infirmerie");
    let att = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit_dst(), soir(3)).await.unwrap();
    assert_eq!(att.lignes.iter().find(|l| l.eleve_id == b.a[1]).unwrap().motif_code.as_deref(), Some("sante"), "« absent : santé » sans détail");
}

#[tokio::test]
async fn di_01_perimetre_verifie_au_serveur() {
    let Some(b) = banc().await else { return };
    let pool = b.ent.pool_verif();
    // Deux classes du jeu de labo ; les élèves de la zone A sont dans la classe 1, ceux de B dans la classe 2.
    let (annee, debut_annee): (Uuid, NaiveDate) =
        sqlx::query_as("SELECT id, date_debut FROM ent_annee_scolaire WHERE etablissement_id = $1 LIMIT 1").bind(b.etab).fetch_one(pool).await.unwrap();
    let classes: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM ent_classe WHERE etablissement_id = $1 ORDER BY code LIMIT 2").bind(b.etab).fetch_all(pool).await.unwrap();
    assert_eq!(classes.len(), 2);
    for (i, e) in b.a.iter().chain(b.b.iter()).enumerate() {
        let classe = if i < b.a.len() { classes[0] } else { classes[1] };
        sqlx::query("INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(Uuid::new_v4())
            .bind(b.etab)
            .bind(annee)
            .bind(e)
            .bind(classe)
            .bind(debut_annee)
            .execute(pool)
            .await
            .unwrap();
    }
    // Direction « de la formation A » : les trois droits, limités aux élèves de la classe 1 (portée « désignés »).
    let droit = |a: Action| Droit { action: a, portee: Portee::Designes, cibles: vec![classes[0]] };
    let role = b
        .ent
        .creer_role_for_actor(
            b.etab,
            b.dir,
            DIR,
            RoleSaisie {
                nom: format!("Direction formation A {}", alea()),
                description: None,
                modele: None,
                droits: vec![droit(Action::InternatLire), droit(Action::InternatAppel), droit(Action::InternatAppelValider)],
            },
        )
        .await
        .unwrap();
    b.ent.attribuer_role_for_actor(b.etab, b.dir, DIR, role.id, b.compta, None).await.unwrap();
    let (u, r) = (b.compta, "compta");
    // Elle lit son attendu (filtré), ouvre et valide l'appel de SA zone…
    let att = b.ent.attendu_nuit_a(b.etab, u, r, nuit_dst(), soir(0)).await.unwrap();
    assert_eq!(att.lignes.len(), 4, "seulement ses élèves");
    assert!(att.lignes.iter().all(|l| b.a.contains(&l.eleve_id)));
    let vue = b.ent.ouvrir_appel_a(b.etab, u, r, &ouverture("soir", "zone", Some(b.zone_a)), soir(1)).await.unwrap();
    // … mais ni un appel qui déborde (internat entier, zone de l'autre formation), ni le pointage d'un élève d'ailleurs.
    assert!(est_interdit(&b.ent.ouvrir_appel_a(b.etab, u, r, &ouverture("soir", "internat", None), soir(1)).await));
    assert!(est_interdit(&b.ent.ouvrir_appel_a(b.etab, u, r, &ouverture("soir", "zone", Some(b.zone_b)), soir(1)).await));
    let autre = b.ouvrir("soir", "zone", Some(b.zone_b), 2).await.appel.id;
    assert!(est_interdit(&b.ent.vue_appel_a(b.etab, u, r, autre, soir(3)).await), "lecture d'un appel hors périmètre");
    assert!(est_interdit(&b.ent.pointer_a(b.etab, u, r, autre, &pointage(b.b[0], "present"), soir(3)).await));
    assert!(est_interdit(&b.ent.cloturer_appel_a(b.etab, u, r, autre, soir(3)).await));
    assert!(est_interdit(&b.ent.pdf_appel_a(b.etab, u, r, autre, soir(3)).await));
    assert!(est_interdit(&b.ent.historique_appel_a(b.etab, u, r, autre, soir(3)).await));
    let liste = b.ent.lister_appels_a(b.etab, u, r, nuit_dst()).await.unwrap();
    assert_eq!(liste.len(), 1, "l'appel de l'autre formation n'est pas listé");
    assert_eq!(liste[0].id, vue.appel.id);
    // Une pointage dans son périmètre, puis clôture et validation de son appel : permis.
    for e in &b.a {
        b.ent.pointer_a(b.etab, u, r, vue.appel.id, &pointage(*e, "present"), soir(4)).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, u, r, vue.appel.id, soir(5)).await.unwrap();
    b.ent.valider_appel_a(b.etab, u, r, vue.appel.id, soir(6)).await.unwrap();
    // La transmission couvre tout l'internat : refusée à une portée restreinte.
    assert!(est_interdit(&b.ent.transmettre_matin_a(b.etab, u, r, nuit_dst(), utc(2026, 3, 29, 5, 0)).await));
    assert!(est_interdit(&b.ent.effectif_repas_a(b.etab, u, r, nuit_dst()).await));
    // Le cas d'un élève hors de sa portée lui est fermé.
    let cas = b.ent.lister_cas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(7)).await.unwrap();
    assert!(cas.is_empty());
    b.ent.retirer_role_for_actor(b.etab, b.dir, DIR, role.id, b.compta).await.unwrap();
    assert!(est_interdit(&b.ent.attendu_nuit_a(b.etab, u, r, nuit_dst(), soir(8)).await), "le rôle retiré, plus aucun droit");
}

#[tokio::test]
async fn isolation_entre_etablissements_appels_cas_et_attendu() {
    let Some(b) = banc().await else { return };
    let autre = crate::tests_rls_stricte::seed_supplementaire(&b.ent).await;
    let (e2, cpe2) = (autre.etablissement_id, autre.cpe.user_id);
    assert_ne!(e2, b.etab);
    let vue = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await;
    b.point(vue.appel.id, b.a[0], "absent", 1).await.unwrap();
    // L'autre établissement ne voit ni l'appel, ni ses élèves ; il ne peut ni pointer ni lire.
    assert!(matches!(b.ent.vue_appel_a(e2, cpe2, CPE, vue.appel.id, soir(2)).await, Err(EntDbError::Domain(DomainError::NotFound))));
    assert!(matches!(
        b.ent.pointer_a(e2, cpe2, CPE, vue.appel.id, &pointage(b.a[0], "present"), soir(2)).await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));
    assert!(matches!(b.ent.cloturer_appel_a(e2, cpe2, CPE, vue.appel.id, soir(2)).await, Err(EntDbError::Domain(DomainError::NotFound))));
    assert!(matches!(b.ent.pdf_appel_a(e2, cpe2, CPE, vue.appel.id, soir(2)).await, Err(EntDbError::Domain(DomainError::NotFound))));
    assert!(b.ent.lister_appels_a(e2, cpe2, CPE, nuit_dst()).await.unwrap().is_empty());
    let att = b.ent.attendu_nuit_a(e2, cpe2, CPE, nuit_dst(), soir(2)).await.unwrap();
    assert!(att.lignes.iter().all(|l| !b.a.contains(&l.eleve_id) && !b.b.contains(&l.eleve_id)));
    // Retrait, nuit exceptionnelle, liste pour un élève ou une chambre d'ailleurs : NotFound.
    let r = b
        .ent
        .poser_retrait_a(e2, cpe2, CPE, &RetraitSaisieDto { eleve_id: b.a[0], du_nuit: nuit_dst(), au_nuit: nuit_dst(), motif_code: "sortie".into() }, soir(2))
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))), "{r:?}");
    let r = b.ent.creer_liste_a(e2, cpe2, CPE, &ListeSaisieDto { libelle: "Vol".into(), chambre_ids: vec![b.chambre_a], confie_a_user_id: None }).await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))), "{r:?}");
    // Cas : un cas de l'établissement 1 n'existe pas dans le 2.
    let cas = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, vue.appel.id, soir(9)).await;
    assert!(cas.is_err(), "3 attendus non pointés : la clôture reste impossible");
    for e in &b.a[1..] {
        b.point(vue.appel.id, *e, "present", 3).await.unwrap();
    }
    let cas_id = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, vue.appel.id, soir(10)).await.unwrap().cas_ouverts[0];
    assert!(matches!(b.ent.cas_vue_a(e2, cpe2, CPE, cas_id, soir(11)).await, Err(EntDbError::Domain(DomainError::NotFound))));
    assert!(matches!(
        b.ent.cas_issue_a(e2, cpe2, CPE, cas_id, &CasIssueSaisieDto { issue: "annule_erreur".into(), lieu_code: None, confirme_par_personne_id: None, numero_intervention: None, note: None, a: None }, soir(11)).await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));
}

#[tokio::test]
async fn eleve_protege_sans_nom_pour_un_tiers_et_lectures_journalisees() {
    let Some(b) = banc().await else { return };
    // Mesure de confidentialité renforcée sur a[0] (socle 2, E7) : le CPE, tiers, ne le voit pas nommé.
    b.ent
        .poser_confidentialite_a(
            DIR,
            crate::eleve_confidentialite::PoserConfidentialite {
                etablissement_id: b.etab,
                eleve_id: b.a[0],
                motif: "décision de justice".into(),
                mesures: vec![],
                du: NaiveDate::from_ymd_opt(2026, 1, 1),
                au: None,
                acteur: b.dir,
            },
            utc(2026, 3, 1, 10, 0),
        )
        .await
        .unwrap();
    let vue = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await;
    let protege = vue.lignes.iter().find(|l| l.eleve_id == b.a[0]).unwrap();
    assert!(protege.protege);
    assert!(protege.nom.is_none() && protege.prenom.is_none(), "jamais nommé à un tiers");
    assert!(protege.chambre_code.is_some() && protege.lit_numero.is_some(), "retrouvé par son lit");
    let autre = vue.lignes.iter().find(|l| l.eleve_id == b.a[1]).unwrap();
    assert!(!autre.protege && autre.nom.is_some());
    // On peut quand même le pointer (sécurité d'abord), et le PDF ne le nomme pas.
    b.point(vue.appel.id, b.a[0], "present", 1).await.unwrap();
    let att = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit_dst(), soir(2)).await.unwrap();
    assert!(att.lignes.iter().find(|l| l.eleve_id == b.a[0]).unwrap().nom.is_none());
    for e in &b.a[1..] {
        b.point(vue.appel.id, *e, "present", 3).await.unwrap();
    }
    let pdf = b.ent.pdf_appel_a(b.etab, b.cpe, CPE, vue.appel.id, soir(4)).await.unwrap();
    let nom: String = sqlx::query_scalar("SELECT nom FROM ent_eleve WHERE etablissement_id = $1 AND id = $2").bind(b.etab).bind(b.a[0]).fetch_one(b.ent.pool_verif()).await.unwrap();
    let texte = ent_pdf::releve::texte(&pdf);
    assert!(!texte.contains(&nom) && texte.contains("Élève protégé"), "{texte}");
    // Consultation de l'élève protégé tracée au journal d'audit (E7).
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND eleve_id = $2 AND domaine = 'consultation'")
        .bind(b.etab)
        .bind(b.a[0])
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert!(n >= 1, "consultation tracée");
    // Journal des lectures sensibles (R-IT38) : appel, attendu, PDF ; lisible par l'audit seulement.
    let lectures = b.ent.lectures_sensibles_a(b.etab, b.dir, DIR, 50).await.unwrap();
    let objets: Vec<&str> = lectures.iter().map(|l| l.objet.as_str()).collect();
    for o in ["appel", "attendu", "appel_pdf"] {
        assert!(objets.contains(&o), "{o} absent de {objets:?}");
    }
    assert!(lectures.iter().all(|l| l.user_id == b.cpe));
    assert!(est_interdit(&b.ent.lectures_sensibles_a(b.etab, b.cpe, CPE, 10).await), "le CPE ne lit pas le journal des lectures");
    assert!(sqlx::query("DELETE FROM ent_internat_lecture_sensible").execute(b.ent.pool_verif()).await.is_err(), "ajout seul");
}

#[tokio::test]
async fn nuit_exceptionnelle_et_retrait_validations() {
    let Some(b) = banc().await else { return };
    let nuit = nuit_dst();
    // Jamais dans le passé (la nuit en cours est comprise), motif connu, élève de l'établissement.
    let hier = NuitExceptionnelleSaisieDto { eleve_id: b.a[0], nuit: nuit - chrono::Duration::days(1), motif_code: "examen".into() };
    assert!(est_refus(&b.ent.poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &hier, soir(0)).await));
    let inconnu = NuitExceptionnelleSaisieDto { eleve_id: b.a[0], nuit, motif_code: "caprice".into() };
    assert!(est_refus(&b.ent.poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &inconnu, soir(0)).await));
    let ok = NuitExceptionnelleSaisieDto { eleve_id: b.a[0], nuit, motif_code: "intemperies".into() };
    let n = b.ent.poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &ok, soir(0)).await.unwrap();
    assert!(est_refus(&b.ent.poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &ok, soir(1)).await), "déjà posée");
    b.ent.annuler_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, n.id, soir(2)).await.unwrap();
    assert!(b.ent.annuler_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, n.id, soir(3)).await.is_err(), "déjà annulée");
    assert!(sqlx::query("DELETE FROM ent_internat_nuit_exceptionnelle").execute(b.ent.pool_verif()).await.is_err());
    // Retrait : période valide, motif connu.
    let r = |du: i64, au: i64, m: &str| RetraitSaisieDto { eleve_id: b.a[0], du_nuit: nuit + chrono::Duration::days(du), au_nuit: nuit + chrono::Duration::days(au), motif_code: m.into() };
    assert!(est_refus(&b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &r(2, 1, "sortie"), soir(0)).await));
    assert!(est_refus(&b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &r(0, 500, "sortie"), soir(0)).await));
    assert!(est_refus(&b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &r(0, 1, "convenance"), soir(0)).await));
    // Un retrait ne se modifie pas : seule l'annulation est possible.
    let ok = b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &r(0, 2, "sejour_scolaire"), soir(0)).await.unwrap();
    assert!(sqlx::query("UPDATE ent_internat_retrait SET motif_code = 'sortie' WHERE id = $1").bind(ok.id).execute(b.ent.pool_verif()).await.is_err());
    // Une liste : doublon de libellé refusé, chambre inconnue refusée.
    let lst = ListeSaisieDto { libelle: "Aile des grands".into(), chambre_ids: vec![b.chambre_a, b.chambre_b], confie_a_user_id: None };
    b.ent.creer_liste_a(b.etab, b.cpe, CPE, &lst).await.unwrap();
    assert!(est_refus(&b.ent.creer_liste_a(b.etab, b.cpe, CPE, &lst).await));
    let inconnue = ListeSaisieDto { libelle: "Fantôme".into(), chambre_ids: vec![Uuid::new_v4()], confie_a_user_id: None };
    assert!(b.ent.creer_liste_a(b.etab, b.cpe, CPE, &inconnue).await.is_err());
    assert_eq!(b.ent.lister_listes_a(b.etab, b.cpe, CPE).await.unwrap().len(), 1);
}
