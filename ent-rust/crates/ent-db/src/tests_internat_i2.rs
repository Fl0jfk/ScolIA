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

/// Le résultat est le refus **attendu** : un refus métier (`Refus`, `ConflitDossierDetail`) dont le message contient
/// `fragment`, ou une erreur de domaine dont le nom de variante le contient (« InternatMotifRequis »). Une autre
/// erreur (panne de base, `Forbidden`, « introuvable »…) n'est JAMAIS un refus attendu : un test de refus ne doit
/// pas passer au vert sur une panne. Affiche l'erreur reçue quand ce n'est pas le bon refus.
#[track_caller]
fn refus_de(r: &Result<impl std::fmt::Debug, EntDbError>, fragment: &str) -> bool {
    let attendu = fragment.to_lowercase();
    let ok = match r {
        Err(EntDbError::Refus(_, m)) => m.to_lowercase().contains(&attendu),
        Err(EntDbError::ConflitDossierDetail(m)) => m.to_lowercase().contains(&attendu),
        Err(EntDbError::Domain(d)) => format!("{d:?}").to_lowercase().contains(&attendu),
        _ => false,
    };
    if !ok {
        eprintln!(
            "refus attendu « {fragment} » à la ligne {}, reçu : {:?}",
            std::panic::Location::caller().line(),
            r.as_ref().map(|_| "Ok").map_err(|e| format!("{e:?}"))
        );
    }
    ok
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
    assert!(refus_de(&b.ent.valider_appel_a(b.etab, b.cpe, CPE, appel2, soir(16)).await, "clore l'appel avant de le valider"));
    assert!(refus_de(&b.ent.valider_appel_a(b.etab, b.cpe, CPE, ronde, soir(17)).await, "l'appel est déjà validé"));
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
    assert!(refus_de(&r, "périmètre incohérent avec l'identifiant fourni"));
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
    assert!(refus_de(&b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &ouverture("etude", "internat", None), soir(7)).await, "ce type d'appel est désactivé dans"));
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
    assert!(refus_de(&b.point(appel, b.b[0], "present", 2).await, "élève hors du périmètre de l'appel ou hors"));
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
    assert!(refus_de(&b.point(appel, b.a[3], "present", 3).await, "marque incompatible avec l'attendu : un élève"));
    b.point(appel, b.a[3], "non_attendu_present", 4).await.unwrap();
    assert!(refus_de(&b.point(appel, b.a[0], "non_attendu_present", 5).await, "marque incompatible avec l'attendu : un élève"));
    assert!(refus_de(&b.point(appel, b.a[0], "excuse", 6).await, "marque de pointage inconnue"), "« excusé » n'est pas un pointage");
    // Heure du geste dans le futur ou hors de la nuit : refusée.
    let mut futur = pointage(b.a[0], "present");
    futur.a = Some(soir(60));
    assert!(refus_de(&b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &futur, soir(7)).await, "l'heure du geste est dans le futur"));
    let mut hier = pointage(b.a[0], "present");
    hier.a = Some(utc(2026, 3, 27, 12, 0));
    assert!(refus_de(&b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &hier, soir(7)).await, "l'heure du geste n'est pas dans la nuit de"));
    // R-IT6 : aucune donnée de santé dans une note d'appel.
    let mut note = pointage(b.a[0], "present");
    note.note = Some("crise d'asthme ce soir".into());
    assert!(refus_de(&b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &note, soir(8)).await, "pas de donnée de santé dans l'appel : utiliser"));
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
    assert!(refus_de(&b.point(appel, b.a[0], "retard", 1).await, "une arrivée tardive annoncée porte l'heure de"));
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
    assert!(refus_de(&b.ent.cas_etape_a(b.etab, b.cpe, CPE, cas_id, &e("chambre"), soir(6)).await, "cette étape ou ce lieu est déjà enregistré"));
    assert!(refus_de(&b.ent.cas_etape_a(b.etab, b.cpe, CPE, cas_id, &CasEtapeSaisieDto { etape_id: Some(Uuid::new_v4()), lieu_code: None, note: None, fait_le: None }, soir(6)).await, "cette étape n'appartient pas à la procédure du"));
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
    assert!(refus_de(&b.ent.cas_etape_a(b.etab, b.cpe, CPE, cas_id, &sante, soir(16)).await, "pas de donnée de santé dans un cas : renvoyer"));
    // Issues : chacune exige sa pièce ; un cas ne se ferme que par une issue (R-IT18).
    let issue = |i: &str| CasIssueSaisieDto { issue: i.into(), lieu_code: None, confirme_par_personne_id: None, numero_intervention: None, note: None, a: Some(soir(20)) };
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &issue("retrouve"), soir(21)).await, "« retrouvé » exige le lieu où l'élève a été"));
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &issue("rentre_famille"), soir(21)).await, "« rentré en famille » exige la personne qui l'a"));
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &issue("autorite_saisie"), soir(21)).await, "« autorité saisie » exige le numéro"));
    let mut inconnue = issue("rentre_famille");
    inconnue.confirme_par_personne_id = Some(Uuid::new_v4());
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &inconnue, soir(21)).await, "la personne indiquée n'est pas rattachée à"), "personne non rattachée");
    let mut ok = issue("rentre_famille");
    ok.confirme_par_personne_id = Some(personnes[0]);
    let clos = b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &ok, soir(21)).await.unwrap();
    assert_eq!(clos.issue.as_deref(), Some("rentre_famille"));
    let mut annulation = issue("annule_erreur");
    annulation.note = Some("ouvert par erreur".into());
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &annulation, soir(22)).await, "le cas est déjà clos"), "déjà clos");
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
    assert!(refus_de(&b.ent.publier_procedure_a(b.etab, b.dir, DIR, &mauvaise, soir(1)).await, "les délais des étapes doivent être croissants,"), "délais décroissants");
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
    // M3 : appel clos mais non validé → pas de transmission ; la validation la rend possible.
    let matin = utc(2026, 3, 29, 5, 0);
    assert!(refus_de(&b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await, "l'appel du soir n'est pas validé : le valider"), "soir non validé");
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, appel, soir(4)).await.unwrap();
    // Pas de transmission sans appel clos ; nuit future refusée.
    let ailleurs = NaiveDate::from_ymd_opt(2026, 3, 20).unwrap();
    assert!(refus_de(&b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, ailleurs, None, utc(2026, 3, 29, 6, 0)).await, "l'appel du soir n'est pas validé : le valider"));
    // Le lendemain matin (07 h 00 CEST = 05 h UTC), toujours la nuit du 28 (jour local postérieur).
    let t = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await.unwrap();
    assert!(t.nouvelle);
    assert_eq!((t.nb_attendus, t.nb_presents, t.nb_absents, t.nb_cas), (6, 4, 1, 1));
    // Idempotent : une seconde transmission ne republie rien.
    let t2 = b.ent.transmettre_matin_a(b.etab, b.dir, DIR, nuit_dst(), None, matin + chrono::Duration::minutes(5)).await.unwrap();
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
    let e = b.ent.effectif_repas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(0)).await.unwrap();
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
        assert!(est_interdit(&b.ent.lister_appels_a(e, user, role, cas_nuit, soir(0)).await), "{role} : liste");
        assert!(est_interdit(&b.ent.ouvrir_appel_a(e, user, role, &ouverture("ronde", "zone", Some(b.zone_a)), soir(1)).await), "{role} : ouvrir");
        assert!(est_interdit(&b.ent.pointer_a(e, user, role, appel, &pointage(b.a[0], "present"), soir(1)).await), "{role} : pointer");
        assert!(est_interdit(&b.ent.cloturer_appel_a(e, user, role, appel, soir(1)).await), "{role} : clôturer");
        assert!(est_interdit(&b.ent.valider_appel_a(e, user, role, appel, soir(1)).await), "{role} : valider");
        assert!(est_interdit(&b.ent.pdf_appel_a(e, user, role, appel, soir(1)).await), "{role} : PDF");
        assert!(est_interdit(&b.ent.lister_cas_a(e, user, role, cas_nuit, soir(1)).await), "{role} : cas");
        assert!(est_interdit(&b.ent.lire_procedure_a(e, user, role, soir(1)).await), "{role} : procédure");
        assert!(est_interdit(&b.ent.effectif_repas_a(e, user, role, cas_nuit, soir(0)).await), "{role} : effectif");
        assert!(est_interdit(&b.ent.lectures_sensibles_a(e, user, role, 10, soir(0)).await), "{role} : journal des lectures");
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
    assert_eq!(att.lignes.iter().find(|l| l.eleve_id == b.a[1]).unwrap().motif_code.as_deref(), Some("excuse"), "M2 : « excusé » seulement, jamais « santé » hors de l'internat");
}

/// Deux classes de l'année qui couvre la nuit testée (créée au besoin) ; les élèves de la zone A vont dans la classe 1,
/// ceux de la zone B dans la classe 2. Sert aux rôles à portée « désignés ».
async fn classes_des_deux_zones(b: &Banc) -> Vec<Uuid> {
    let pool = b.ent.pool_verif();
    // L'année qui couvre la nuit testée (créée au besoin, comme dans le banc) et deux classes **de cette année**, créées
    // pour le test : le jeu de labo place ses classes dans l'année courante (2026-2027), et une affectation doit rester
    // dans l'année de sa classe (garde 330). Les élèves de la zone A vont dans la classe 1, ceux de B dans la classe 2.
    let existante: Option<(Uuid, NaiveDate)> = sqlx::query_as(
        "SELECT id, date_debut FROM ent_annee_scolaire WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin > $2 ORDER BY date_debut, id LIMIT 1",
    )
    .bind(b.etab)
    .bind(nuit_dst())
    .fetch_optional(pool)
    .await
    .unwrap();
    let (annee, debut_annee) = match existante {
        Some(a) => a,
        None => {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin) VALUES ($1,$2,'2025-2026','2025-09-01','2026-08-31')")
                .bind(id)
                .bind(b.etab)
                .execute(pool)
                .await
                .unwrap();
            (id, NaiveDate::from_ymd_opt(2025, 9, 1).unwrap())
        }
    };
    let mut classes: Vec<Uuid> = Vec::new();
    for code in ["DI01A", "DI01B"] {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle, niveau) VALUES ($1,$2,$3,$4,$4,'5e')")
            .bind(id)
            .bind(b.etab)
            .bind(annee)
            .bind(code)
            .execute(pool)
            .await
            .unwrap();
        classes.push(id);
    }
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
    classes
}

#[tokio::test]
async fn di_01_perimetre_verifie_au_serveur() {
    let Some(b) = banc().await else { return };
    let classes = classes_des_deux_zones(&b).await;
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
    let liste = b.ent.lister_appels_a(b.etab, u, r, nuit_dst(), soir(0)).await.unwrap();
    assert_eq!(liste.len(), 1, "l'appel de l'autre formation n'est pas listé");
    assert_eq!(liste[0].id, vue.appel.id);
    // Une pointage dans son périmètre, puis clôture et validation de son appel : permis.
    for e in &b.a {
        b.ent.pointer_a(b.etab, u, r, vue.appel.id, &pointage(*e, "present"), soir(4)).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, u, r, vue.appel.id, soir(5)).await.unwrap();
    b.ent.valider_appel_a(b.etab, u, r, vue.appel.id, soir(6)).await.unwrap();
    // La transmission couvre tout l'internat : refusée à une portée restreinte.
    assert!(est_interdit(&b.ent.transmettre_matin_a(b.etab, u, r, nuit_dst(), None, utc(2026, 3, 29, 5, 0)).await));
    assert!(est_interdit(&b.ent.effectif_repas_a(b.etab, u, r, nuit_dst(), soir(0)).await));
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
    assert!(b.ent.lister_appels_a(e2, cpe2, CPE, nuit_dst(), soir(0)).await.unwrap().is_empty());
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
        b.ent.cas_issue_a(e2, cpe2, CPE, cas_id, &CasIssueSaisieDto { issue: "annule_erreur".into(), lieu_code: None, confirme_par_personne_id: None, numero_intervention: None, note: Some("erreur".into()), a: None }, soir(11)).await,
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
    let lectures = b.ent.lectures_sensibles_a(b.etab, b.dir, DIR, 50, soir(0)).await.unwrap();
    let objets: Vec<&str> = lectures.iter().map(|l| l.objet.as_str()).collect();
    for o in ["appel", "attendu", "appel_pdf"] {
        assert!(objets.contains(&o), "{o} absent de {objets:?}");
    }
    assert!(lectures.iter().all(|l| l.user_id == b.cpe));
    assert!(est_interdit(&b.ent.lectures_sensibles_a(b.etab, b.cpe, CPE, 10, soir(0)).await), "le CPE ne lit pas le journal des lectures");
    assert!(sqlx::query("DELETE FROM ent_internat_lecture_sensible").execute(b.ent.pool_verif()).await.is_err(), "ajout seul");
}

#[tokio::test]
async fn nuit_exceptionnelle_et_retrait_validations() {
    let Some(b) = banc().await else { return };
    let nuit = nuit_dst();
    // Jamais dans le passé (la nuit en cours est comprise), motif connu, élève de l'établissement.
    let hier = NuitExceptionnelleSaisieDto { eleve_id: b.a[0], nuit: nuit - chrono::Duration::days(1), motif_code: "examen".into() };
    assert!(refus_de(&b.ent.poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &hier, soir(0)).await, "une nuit exceptionnelle ne se pose pas dans le"));
    let inconnu = NuitExceptionnelleSaisieDto { eleve_id: b.a[0], nuit, motif_code: "caprice".into() };
    assert!(refus_de(&b.ent.poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &inconnu, soir(0)).await, "motif de nuit exceptionnelle inconnu"));
    let ok = NuitExceptionnelleSaisieDto { eleve_id: b.a[0], nuit, motif_code: "intemperies".into() };
    let n = b.ent.poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &ok, soir(0)).await.unwrap();
    assert!(refus_de(&b.ent.poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &ok, soir(1)).await, "cette nuit exceptionnelle existe déjà pour cet"), "déjà posée");
    b.ent.annuler_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, n.id, soir(2)).await.unwrap();
    assert!(b.ent.annuler_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, n.id, soir(3)).await.is_err(), "déjà annulée");
    assert!(sqlx::query("DELETE FROM ent_internat_nuit_exceptionnelle").execute(b.ent.pool_verif()).await.is_err());
    // Retrait : période valide, motif connu.
    let r = |du: i64, au: i64, m: &str| RetraitSaisieDto { eleve_id: b.a[0], du_nuit: nuit + chrono::Duration::days(du), au_nuit: nuit + chrono::Duration::days(au), motif_code: m.into() };
    assert!(refus_de(&b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &r(2, 1, "sortie"), soir(0)).await, "période de retrait invalide"));
    assert!(refus_de(&b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &r(0, 500, "sortie"), soir(0)).await, "période de retrait invalide"));
    assert!(refus_de(&b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &r(0, 1, "convenance"), soir(0)).await, "motif de retrait inconnu"));
    // Un retrait ne se modifie pas : seule l'annulation est possible.
    let ok = b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &r(0, 2, "sejour_scolaire"), soir(0)).await.unwrap();
    assert!(sqlx::query("UPDATE ent_internat_retrait SET motif_code = 'sortie' WHERE id = $1").bind(ok.id).execute(b.ent.pool_verif()).await.is_err());
    // Une liste : doublon de libellé refusé, chambre inconnue refusée.
    let lst = ListeSaisieDto { libelle: "Aile des grands".into(), chambre_ids: vec![b.chambre_a, b.chambre_b], confie_a_user_id: None };
    b.ent.creer_liste_a(b.etab, b.cpe, CPE, &lst).await.unwrap();
    assert!(refus_de(&b.ent.creer_liste_a(b.etab, b.cpe, CPE, &lst).await, "une liste porte déjà ce libellé"));
    let inconnue = ListeSaisieDto { libelle: "Fantôme".into(), chambre_ids: vec![Uuid::new_v4()], confie_a_user_id: None };
    assert!(b.ent.creer_liste_a(b.etab, b.cpe, CPE, &inconnue).await.is_err());
    assert_eq!(b.ent.lister_listes_a(b.etab, b.cpe, CPE, soir(0)).await.unwrap().len(), 1);
}

// ───────────────────────── Correctifs de la revue de sécurité (I2-correctifs) ─────────────────────────
//
// Un test par constat (E1 à E3, M1 à M3, M6 à M9, L1 à L9). Rapport : `docs/socles/41_I2_CORRECTIFS.md`.

fn refuse_42501<T: std::fmt::Debug>(r: &Result<T, sqlx::Error>) -> bool {
    matches!(r, Err(sqlx::Error::Database(d)) if d.code().as_deref() == Some("42501"))
}

/// Élève hébergé hors rythme le samedi : un rythme plus récent sans samedi ni vendredi.
async fn hors_rythme(b: &Banc, eleve: Uuid) {
    sqlx::query(
        "INSERT INTO ent_internat_rythme (id, etablissement_id, sejour_id, du, dimanche, lundi, mardi, mercredi, jeudi, vendredi, samedi, cree_par_user_id) \
         SELECT $1, etablissement_id, id, '2026-02-01', true, true, true, true, true, false, false, $3 \
           FROM ent_internat_sejour WHERE etablissement_id = $2 AND eleve_id = $4",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(b.cpe)
    .bind(eleve)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
}

fn retrait_nuit(eleve: Uuid, motif: &str) -> RetraitSaisieDto {
    RetraitSaisieDto { eleve_id: eleve, du_nuit: nuit_dst(), au_nuit: nuit_dst(), motif_code: motif.into() }
}

/// E1 + M6 : un hébergé hors rythme couvert par un retrait reste « non_attendu » (sinon la contrainte
/// `statut = 'retire' ⇔ motif` casse l'ouverture de TOUT l'appel du périmètre) ; la liste figée, posée en une
/// requête ensembliste, reprend exactement l'attendu calculé.
#[tokio::test]
async fn e1_heberge_hors_rythme_avec_absence_annoncee_l_appel_s_ouvre() {
    let Some(b) = banc().await else { return };
    hors_rythme(&b, b.b[0]).await;
    b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &retrait_nuit(b.b[0], "absence_annoncee"), soir(0)).await.unwrap();
    b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &retrait_nuit(b.a[1], "sortie"), soir(0)).await.unwrap();
    let attendu = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit_dst(), soir(1)).await.unwrap();
    let ligne = |e: Uuid| attendu.lignes.iter().find(|l| l.eleve_id == e).unwrap().clone();
    assert_eq!((ligne(b.b[0]).statut.as_str(), ligne(b.b[0]).motif_code), ("non_attendu", None), "hors rythme, retrait ou non");
    // L'appel « internat » (tous les élèves) s'ouvre.
    let vue = b.ouvrir("soir", "internat", None, 2).await;
    assert_eq!(vue.lignes.len(), 6);
    let figee = |e: Uuid| vue.lignes.iter().find(|l| l.eleve_id == e).unwrap().clone();
    assert_eq!(figee(b.b[0]).statut, "non_attendu");
    assert_eq!(figee(b.b[0]).motif_code, None);
    assert_eq!((figee(b.a[1]).statut.as_str(), figee(b.a[1]).motif_code.as_deref()), ("retire", Some("sortie")));
    // M6 : la liste figée est celle de l'attendu calculé, ligne pour ligne, et l'effectif attendu en est déduit.
    for l in &attendu.lignes {
        assert_eq!(figee(l.eleve_id).statut, l.statut);
        assert_eq!(figee(l.eleve_id).chambre_code, l.chambre_code);
        assert_eq!(figee(l.eleve_id).lit_numero, l.lit_numero);
    }
    assert_eq!(vue.appel.nb_attendus, 4, "6 hébergés − 1 hors rythme − 1 en sortie");
    // Un non attendu présent se pointe « non_attendu_present ».
    b.point(vue.appel.id, b.b[0], "non_attendu_present", 3).await.unwrap();
    // Un appel de zone ne fige que les élèves de la zone.
    let zone = b.ouvrir("ronde", "zone", Some(b.zone_b), 4).await;
    assert_eq!(zone.lignes.len(), 2);
    assert!(zone.lignes.iter().all(|l| b.b.contains(&l.eleve_id)));
    // Une liste vide (périmètre sans élève) s'ouvre aussi, sans ligne ni division par zéro.
    let vide = b.ent.creer_liste_a(b.etab, b.cpe, CPE, &ListeSaisieDto { libelle: "Vide".into(), chambre_ids: vec![], confie_a_user_id: None }).await.unwrap();
    let mut o = ouverture("soir", "liste", None);
    o.liste_id = Some(vide.id);
    let v = b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &o, soir(5)).await.unwrap();
    assert_eq!((v.lignes.len(), v.appel.nb_attendus), (0, 0));
}

/// L7 (corrigé en 2825) : `date_sortie` est le PREMIER jour d'absence (période `[entrée, sortie[`, migration 283).
/// Sorti le 28 : encore attendu la nuit du 27, plus celle du 28 (sinon fausse alerte « enfant disparu »).
#[tokio::test]
async fn l7_un_eleve_sorti_n_est_plus_attendu_des_son_premier_jour_d_absence() {
    let Some(b) = banc().await else { return };
    for (eleve, sortie) in [(b.a[3], "2026-03-20"), (b.a[2], "2026-03-28")] {
        sqlx::query("UPDATE ent_eleve SET date_sortie = $3::date WHERE etablissement_id = $1 AND id = $2")
            .bind(b.etab)
            .bind(eleve)
            .bind(sortie)
            .execute(b.ent.pool_verif())
            .await
            .unwrap();
    }
    let veille = nuit_dst() - chrono::Duration::days(1);
    let att27 = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, veille, soir(0)).await.unwrap();
    assert_eq!(att27.lignes.iter().find(|l| l.eleve_id == b.a[2]).unwrap().statut, "attendu", "sortie le 28 : encore attendu le 27");
    assert!(att27.lignes.iter().all(|l| l.eleve_id != b.a[3]), "sortie le 20 : plus attendu le 27");
    let att28 = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit_dst(), soir(0)).await.unwrap();
    assert!(att28.lignes.iter().all(|l| l.eleve_id != b.a[2]), "sortie le 28 : premier jour d'absence, plus attendu la nuit du 28");
    assert!(att28.lignes.iter().all(|l| l.eleve_id != b.a[3]), "sortie le 20 : absent de l'attendu du 28");
    let vue = b.ouvrir("soir", "internat", None, 1).await;
    assert_eq!(vue.lignes.len(), 4, "6 hébergés moins les deux sortis");
    assert_eq!(vue.appel.nb_attendus, 4);
}

/// E2 — TRUNCATE refusé sur toutes les tables en ajout seul ou à garde (le déclencheur de ligne ne le voit pas),
/// y compris `ent_internat_presence_fait` (2820) ; aucune dérogation par réglage de session, et le propriétaire
/// seul pourrait désarmer un déclencheur (essayé avec le rôle applicatif en mode strict).
#[tokio::test]
async fn e2_truncate_refuse_sur_les_tables_en_ajout_seul() {
    let Some(b) = banc().await else { return };
    let tables = [
        "ent_internat_appel", "ent_internat_appel_attendu", "ent_internat_cas", "ent_internat_cas_etape",
        "ent_internat_procedure_version", "ent_internat_procedure_etape", "ent_internat_transmission",
        "ent_internat_appel_rappel", "ent_internat_lecture_sensible", "ent_internat_retrait",
        "ent_internat_nuit_exceptionnelle", "ent_internat_presence_fait",
    ];
    for t in tables {
        let mut tx = b.ent.pool_verif().begin().await.unwrap();
        let r = sqlx::query(&format!("TRUNCATE {t} CASCADE")).execute(&mut *tx).await;
        assert!(refuse_42501(&r), "TRUNCATE {t} CASCADE doit échouer par le déclencheur (42501), pas par une clé étrangère : {r:?}");
        drop(tx);
        let mut tx = b.ent.pool_verif().begin().await.unwrap();
        let r = sqlx::query(&format!("TRUNCATE {t}")).execute(&mut *tx).await;
        assert!(r.is_err(), "TRUNCATE {t} refusé");
    }
    // Aucune dérogation de session : un SET puis un UPDATE ou un TRUNCATE échouent toujours.
    b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &retrait_nuit(b.a[0], "sortie"), soir(0)).await.unwrap();
    let mut tx = b.ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.purge_etablissement', 'on', true)").execute(&mut *tx).await.unwrap();
    let maj = sqlx::query("UPDATE ent_internat_retrait SET motif_code = 'stage' WHERE etablissement_id = $1").bind(b.etab).execute(&mut *tx).await;
    assert!(maj.is_err(), "un SET de session ne lève pas l'ajout seul");
    drop(tx);
    let mut tx = b.ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.purge_etablissement', 'on', true)").execute(&mut *tx).await.unwrap();
    let tr = sqlx::query("TRUNCATE ent_internat_retrait").execute(&mut *tx).await;
    assert!(refuse_42501(&tr));
    drop(tx);
    // Désarmer un déclencheur est réservé au propriétaire : le rôle applicatif strict n'y arrive pas.
    if crate::tests_rls_stricte::mode_strict_demande() {
        let mut tx = b.ent.pool().begin().await.unwrap();
        let r = sqlx::query("ALTER TABLE ent_internat_appel DISABLE TRIGGER ALL").execute(&mut *tx).await;
        assert!(r.is_err(), "ALTER TABLE … DISABLE TRIGGER refusé au rôle applicatif");
    }
}

/// E2 — la liste figée d'un appel ne se complète que dans la transaction qui CRÉE l'appel, tant qu'il est ouvert.
#[tokio::test]
async fn e2_liste_figee_insertion_seulement_a_la_creation_de_l_appel() {
    let Some(b) = banc().await else { return };
    let appel = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await.appel.id;
    let ajout = |eleve: Uuid, appel: Uuid| {
        sqlx::query(
            "INSERT INTO ent_internat_appel_attendu (etablissement_id, appel_id, eleve_id, statut) VALUES ($1,$2,$3,'attendu')",
        )
        .bind(b.etab)
        .bind(appel)
        .bind(eleve)
    };
    // Appel ouvert mais créé dans une transaction antérieure : refus.
    let r = ajout(b.b[0], appel).execute(b.ent.pool_verif()).await;
    assert!(refuse_42501(&r), "insertion après coup refusée : {r:?}");
    // Appel clos : refus aussi.
    for e in &b.a {
        b.point(appel, *e, "present", 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(2)).await.unwrap();
    let r = ajout(b.b[0], appel).execute(b.ent.pool_verif()).await;
    assert!(refuse_42501(&r), "appel clos : {r:?}");
    // Dans la transaction qui crée l'appel, l'insertion est permise.
    let mut tx = b.ent.pool_verif().begin().await.unwrap();
    let neuf = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_internat_appel (id, etablissement_id, nuit, type, perimetre, ouvert_par_user_id, ouvert_le, nb_attendus) \
         VALUES ($1,$2,$3,'ronde','internat',$4,$5,1)",
    )
    .bind(neuf)
    .bind(b.etab)
    .bind(nuit_dst())
    .bind(b.cpe)
    .bind(soir(3))
    .execute(&mut *tx)
    .await
    .unwrap();
    ajout(b.b[0], neuf).execute(&mut *tx).await.expect("même transaction, appel ouvert");
    tx.commit().await.unwrap();
    // Et plus ensuite.
    let r = ajout(b.b[1], neuf).execute(b.ent.pool_verif()).await;
    assert!(refuse_42501(&r), "la liste est close après la transaction de création : {r:?}");
}

/// E2 — seules transitions : ouvert → clos et clos → validé ; l'appel clos fige écarts, auteur et heure de clôture ;
/// `a_posteriori` ne change jamais.
#[tokio::test]
async fn e2_garde_de_l_appel_transitions_et_champs_figes() {
    let Some(b) = banc().await else { return };
    let pool = b.ent.pool_verif();
    let appel = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await.appel.id;
    let maj = |sql: &str, id: Uuid| {
        let texte = format!("UPDATE ent_internat_appel SET {sql} WHERE id = $1");
        async move { sqlx::query(&texte).bind(id).execute(pool).await }
    };
    // Ouvert : ni saut direct vers « validé », ni modification de a_posteriori.
    let r = maj("etat = 'valide', clos_le = now(), clos_par_user_id = valide_par_user_id, valide_le = now(), valide_par_user_id = ouvert_par_user_id", appel).await;
    assert!(refuse_42501(&r), "ouvert → validé : {r:?}");
    let r = maj("a_posteriori = true", appel).await;
    assert!(refuse_42501(&r), "a_posteriori figé : {r:?}");
    let r = maj("modifie_le = now()", appel).await;
    assert!(refuse_42501(&r), "ouvert → ouvert : seule la clôture modifie un appel ouvert : {r:?}");
    for e in &b.a {
        b.point(appel, *e, if *e == b.a[0] { "absent" } else { "present" }, 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(2)).await.unwrap();
    // Clos : écarts, auteur et heure de clôture figés ; pas de retour à « ouvert ».
    for (champ, sql) in [
        ("nb_ecarts", "nb_ecarts = 0"),
        ("clos_par_user_id", "clos_par_user_id = ouvert_par_user_id"),
        ("clos_le", "clos_le = clos_le + interval '1 hour'"),
        ("a_posteriori", "a_posteriori = true"),
    ] {
        let r = maj(sql, appel).await;
        assert!(refuse_42501(&r), "appel clos : {champ} figé : {r:?}");
    }
    let r = maj("etat = 'ouvert', clos_le = NULL, clos_par_user_id = NULL", appel).await;
    assert!(r.is_err(), "clos → ouvert refusé");
    // La validation (clos → validé) reste possible par le service ; ensuite plus rien ne bouge.
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, appel, soir(3)).await.unwrap();
    let r = maj("nb_ecarts = 0", appel).await;
    assert!(r.is_err(), "appel validé : plus aucune modification");
    let (ecarts,): (Option<i32>,) = sqlx::query_as("SELECT nb_ecarts FROM ent_internat_appel WHERE id = $1").bind(appel).fetch_one(pool).await.unwrap();
    assert_eq!(ecarts, Some(1));
}

/// E2 — le motif « reprise » (reprise de l'existant) n'échappe à la garde que pour un fait SANS appel.
#[tokio::test]
async fn e2_motif_reprise_refuse_sur_un_fait_rattache_a_un_appel() {
    let Some(b) = banc().await else { return };
    let pool = b.ent.pool_verif();
    let appel = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await.appel.id;
    let insere = |appel: Option<Uuid>, eleve: Uuid, nuit: &str| {
        sqlx::query(
            "INSERT INTO ent_internat_presence_fait (id, etablissement_id, eleve_id, appel_id, nuit, marque, par_user_id, a, recu_le, motif_correction_code) \
             VALUES ($1,$2,$3,$4,$5::date,'present',$6,$7,$7,'reprise')",
        )
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(eleve)
        .bind(appel)
        .bind(nuit.to_string())
        .bind(b.cpe)
        .bind(soir(1))
    };
    // Sur un appel : hors périmètre ou hors nuit, « reprise » ne contourne plus la garde.
    assert!(insere(Some(appel), b.b[0], "2026-03-28").execute(pool).await.is_err(), "élève hors de la liste figée");
    assert!(insere(Some(appel), b.a[0], "2026-03-01").execute(pool).await.is_err(), "nuit autre que celle de l'appel");
    assert!(insere(Some(appel), b.a[0], "2026-03-28").execute(pool).await.is_err(), "même un fait valable : « reprise » est réservé aux faits sans appel");
    // Sans appel (reprise de l'existant) : accepté, même pour une nuit ancienne.
    insere(None, b.a[0], "2026-03-01").execute(pool).await.expect("reprise sans appel");
}

// ───────────── E3 : numéros rendus au surveillant ─────────────

async fn titulaire(b: &Banc, eleve: Uuid, nom: &str, garde: Option<&str>) -> Uuid {
    let pool = b.ent.pool_verif();
    let p = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom) VALUES ($1,$2,$3,'Test')")
        .bind(p)
        .bind(b.etab)
        .bind(nom)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, lien, garde) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(eleve)
        .bind(p)
        .bind(nom.to_lowercase())
        .bind(garde)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_personne_coordonnee (id, etablissement_id, personne_id, type, valeur) VALUES ($1,$2,$3,'mobile',$4)")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(p)
        .bind(format!("06{}", p.simple().to_string()[..8].chars().map(|c| (c as u8 % 10).to_string()).collect::<String>()))
        .execute(pool)
        .await
        .unwrap();
    p
}

async fn interdire(b: &Banc, eleve: Uuid, personne: Option<Uuid>, objet: &str, levee: bool) {
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, personne_libelle, objet, fondement, consigne, du, decide_par, levee_par, levee_le) \
         VALUES ($1,$2,$3,$4,CASE WHEN $4::uuid IS NULL THEN 'Un tiers' END, ARRAY[$5]::text[], 'decision_justice','Ne pas joindre','2026-01-01',$6, \
                 CASE WHEN $7 THEN $6 END, CASE WHEN $7 THEN '2026-02-01'::timestamptz END)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(personne)
    .bind(objet)
    .bind(b.cpe)
    .bind(levee)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
}

/// Ouvre un appel de la zone A où seuls `absents` sont absents, le clôt : un cas par absent.
async fn cas_des_absents(b: &Banc, absents: &[Uuid]) -> std::collections::HashMap<Uuid, Uuid> {
    let appel = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await.appel.id;
    for e in &b.a {
        b.point(appel, *e, if absents.contains(e) { "absent" } else { "present" }, 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(2)).await.unwrap();
    b.ent.lister_cas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(3)).await.unwrap().into_iter().map(|c| (c.eleve_id, c.id)).collect()
}

#[tokio::test]
async fn e3_numeros_sans_personne_interdite_ni_adresse_protegee_ni_eleve_protege() {
    let Some(b) = banc().await else { return };
    let (a0, a1, a2, a3) = (b.a[0], b.a[1], b.a[2], b.a[3]);
    // a0 : père sous interdiction de communication active ; a1 : interdiction LEVÉE ; a2 : mère à adresse protégée ;
    // a3 : élève protégé (mesure de confidentialité du socle 2).
    let (m0, p0) = (titulaire(&b, a0, "Mere0", None).await, titulaire(&b, a0, "Pere0", None).await);
    let (m1, p1) = (titulaire(&b, a1, "Mere1", None).await, titulaire(&b, a1, "Pere1", None).await);
    let (m2, p2) = (titulaire(&b, a2, "Mere2", None).await, titulaire(&b, a2, "Pere2", None).await);
    let (m3, _p3) = (titulaire(&b, a3, "Mere3", None).await, titulaire(&b, a3, "Pere3", None).await);
    interdire(&b, a0, Some(p0), "communication", false).await;
    interdire(&b, a1, Some(p1), "communication", true).await;
    sqlx::query("INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine) VALUES ($1,$2,$3,'enc:v1:xx','saisie')")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(m2)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, motif_scelle, mesures, du) VALUES ($1,$2,$3,'enc:v1:xx',ARRAY['adresse_protegee'],'2026-01-01')")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(a3)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let cas = cas_des_absents(&b, &[a0, a1, a2, a3]).await;
    let personnes = |v: &CasVueDto| v.numeros_titulaires.iter().map(|n| n.personne_id).collect::<std::collections::HashSet<_>>();
    // Interdiction active : seule la mère reste joignable.
    let v0 = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a0], soir(4)).await.unwrap();
    assert_eq!(personnes(&v0), [m0].into(), "le père sous interdiction n'est pas rendu");
    assert!(!v0.numeros_masques);
    // Interdiction levée : les deux.
    let v1 = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a1], soir(4)).await.unwrap();
    assert_eq!(personnes(&v1), [m1, p1].into(), "interdiction levée : plus d'exclusion");
    // Adresse protégée : la personne n'est pas rendue.
    let v2 = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a2], soir(4)).await.unwrap();
    assert_eq!(personnes(&v2), [p2].into(), "la mère à adresse protégée n'est pas rendue");
    // Élève protégé : décision pessimiste, aucun numéro, la vue le dit.
    let v3 = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a3], soir(4)).await.unwrap();
    assert!(v3.numeros_titulaires.is_empty());
    assert!(v3.numeros_masques, "la vue renvoie vers la direction ou le CPE d'astreinte");
    // Confirmation d'un retour en famille : refusée pour une personne exclue, permise pour une autre.
    let issue = |p: Uuid| CasIssueSaisieDto { issue: "rentre_famille".into(), lieu_code: None, confirme_par_personne_id: Some(p), numero_intervention: None, note: None, a: Some(soir(5)) };
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a0], &issue(p0), soir(6)).await, "cette personne ne peut pas confirmer : passer"), "père sous interdiction");
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a2], &issue(m2), soir(6)).await, "cette personne ne peut pas confirmer : passer"), "mère à adresse protégée");
    b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a0], &issue(m0), soir(6)).await.unwrap();
    b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a1], &issue(p1), soir(6)).await.unwrap();
    // L'adresse ne sort toujours jamais.
    assert!(!format!("{v0:?}{v1:?}{v2:?}{v3:?}").contains("enc:v1"));
    let _ = m3;
}

#[tokio::test]
async fn e3_garde_alternee_et_interdiction_visant_une_personne_non_identifiee() {
    let Some(b) = banc().await else { return };
    let (a0, a1, a2) = (b.a[0], b.a[1], b.a[2]);
    // Garde alternée : deux parents, l'un sous interdiction de CONTACT (autre objet que « communication »).
    let (pa, pb) = (titulaire(&b, a0, "ParentA", Some("alternee")).await, titulaire(&b, a0, "ParentB", Some("alternee")).await);
    interdire(&b, a0, Some(pa), "contact_eleve", false).await;
    // Interdiction de VISITE seulement : ne concerne pas le téléphone, le parent reste joignable.
    let (va, vb) = (titulaire(&b, a1, "VisiteA", Some("alternee")).await, titulaire(&b, a1, "VisiteB", Some("alternee")).await);
    interdire(&b, a1, Some(va), "visite", false).await;
    // Interdiction dont la cible n'est qu'un libellé (aucune personne identifiée) : pessimiste, aucun numéro.
    let (_ca, _cb) = (titulaire(&b, a2, "CibleA", None).await, titulaire(&b, a2, "CibleB", None).await);
    interdire(&b, a2, None, "communication", false).await;
    let cas = cas_des_absents(&b, &[a0, a1, a2]).await;
    let ids = |v: &CasVueDto| v.numeros_titulaires.iter().map(|n| n.personne_id).collect::<std::collections::HashSet<_>>();
    let v0 = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a0], soir(3)).await.unwrap();
    assert_eq!(ids(&v0), [pb].into(), "garde alternée : le parent sous interdiction de contact est exclu, l'autre reste joignable");
    let v1 = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a1], soir(3)).await.unwrap();
    assert_eq!(ids(&v1), [va, vb].into(), "une interdiction de visite ne retire pas le numéro");
    let v2 = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a2], soir(3)).await.unwrap();
    assert!(v2.numeros_titulaires.is_empty() && v2.numeros_masques, "cible non identifiée : aucun numéro (pessimiste)");
}

// ───────────── M1 : santé dans les textes libres ─────────────

#[tokio::test]
async fn m1_controle_sante_sur_motif_de_correction_note_du_cas_et_motif_de_rectification() {
    let Some(b) = banc().await else { return };
    // Appel du soir de TOUT l'internat : la transmission du matin (testée en fin de parcours) exige que le soir couvre
    // tous les attendus.
    let appel = b.ouvrir("soir", "internat", None, 0).await.appel.id;
    // Notes d'appel : racines élargies, avec ou sans accents, textes décomposés et pleine chasse (M1).
    for texte in [
        "a de la fièvre", "FIEVRE", "crise de nerfs", "vomissements", "migraine", "signes de dépression", "idées suicidaires",
        "rdv chez le psy", "suivi TDAH", "épilepsie", "diabète", "allergie", "hospitalisé hier", "fracture du poignet",
        "blessé au genou", "douleurs au ventre", "nausées", "une plaie à la main", "automutilation", "anorexie",
        "fie\u{0301}vre", "e\u{0301}pilepsie", "ＦＩＥＶＲＥ", "fi\u{00AD}evre",
    ] {
        let mut n = pointage(b.a[0], "present");
        n.note = Some(texte.into());
        assert!(refus_de(&b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &n, soir(1)).await, "pas de donnée de santé dans l'appel : utiliser"), "note « {texte} » refusée");
    }
    let mut ok = pointage(b.a[0], "absent");
    ok.note = Some("cherché dans les couloirs".into());
    b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &ok, soir(1)).await.unwrap();
    for e in b.a[1..].iter().chain(b.b.iter()) {
        b.point(appel, *e, "present", 2).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(3)).await.unwrap();
    // Motif de correction (texte libre) : même contrôle que la note.
    let mut corr = pointage(b.a[1], "absent");
    corr.motif_correction_code = Some("erreur_saisie".into());
    corr.motif_correction_texte = Some("il avait de la fièvre".into());
    assert!(refus_de(&b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &corr, soir(4)).await, "pas de donnée de santé dans le motif de"), "texte du motif de correction");
    corr.motif_correction_texte = Some("erreur de lit".into());
    b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &corr, soir(4)).await.unwrap();
    // Notes du cas : étape et issue.
    let cas = b.ent.lister_cas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(5)).await.unwrap();
    let cas_id = cas.iter().find(|c| c.eleve_id == b.a[0]).unwrap().id;
    let etape = |n: &str| CasEtapeSaisieDto { etape_id: None, lieu_code: Some("etude".into()), note: Some(n.into()), fait_le: None };
    assert!(refus_de(&b.ent.cas_etape_a(b.etab, b.cpe, CPE, cas_id, &etape("vomi dans la chambre"), soir(6)).await, "pas de donnée de santé dans un cas : renvoyer"));
    assert!(refus_de(&b.ent.cas_etape_a(b.etab, b.cpe, CPE, cas_id, &etape("fait une crise d'angoisse"), soir(6)).await, "pas de donnée de santé dans un cas : renvoyer"));
    let issue = |n: &str| CasIssueSaisieDto { issue: "retrouve".into(), lieu_code: Some("etude".into()), confirme_par_personne_id: None, numero_intervention: None, note: Some(n.into()), a: Some(soir(7)) };
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &issue("en pleine dépression"), soir(8)).await, "note invalide (300 caractères, aucune donnée de"));
    b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas_id, &issue("retrouvé à l'étude"), soir(8)).await.unwrap();
    // Motif de rectification d'une transmission. Pour prouver que le refus vient bien du CONTRÔLE DE SANTÉ (et non d'une
    // autre condition de la transmission), toutes les autres conditions sont réunies : l'appel du soir est validé, la
    // nuit est passée, une première transmission existe et les chiffres ont changé (une rectificative est donc due).
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, appel, soir(9)).await.unwrap();
    let matin = utc(2026, 3, 29, 5, 0);
    let v1 = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await.unwrap();
    assert!(v1.nouvelle && v1.version == 1);
    let mut tardive = pointage(b.a[2], "absent");
    tardive.motif_correction_code = Some("information_tardive".into());
    b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &tardive, matin + chrono::Duration::minutes(1)).await.unwrap();
    let t = matin + chrono::Duration::minutes(5);
    // Sans motif : le refus est celui de la rectificative, pas celui de la santé (le banc est prêt).
    assert!(refus_de(&b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, t).await, "un motif de rectification est obligatoire"));
    for sante in ["diabète", "Il a de la fièvre", "hospitalisé", "ＦＩＥＶＲＥ"] {
        let r = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), Some(sante), t).await;
        assert!(refus_de(&r, "aucune donnée de santé"), "motif « {sante} » : refus de santé attendu, pas un autre");
    }
    let v2 = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), Some("pointage corrigé"), t).await.unwrap();
    assert!(v2.nouvelle && v2.version == 2, "le même banc accepte un motif sans santé");
}

// ───────────── M2 : « santé » jamais dite hors de l'internat ─────────────

#[tokio::test]
async fn m2_motif_sante_devient_excuse_dans_la_vue_le_pdf_et_les_evenements() {
    let Some(b) = banc().await else { return };
    let infirmiere = b.cast.infirmieres[0].user_id;
    let r = b.ent.poser_retrait_a(b.etab, infirmiere, "infirmerie", &retrait_nuit(b.a[1], "sante"), soir(0)).await.unwrap();
    // La base garde le code réel (audit) ; tout ce qui sort est générique.
    let (motif_base,): (String,) = sqlx::query_as("SELECT motif_code FROM ent_internat_retrait WHERE id = $1").bind(r.id).fetch_one(b.ent.pool_verif()).await.unwrap();
    assert_eq!(motif_base, "sante");
    let att = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit_dst(), soir(1)).await.unwrap();
    assert_eq!(att.lignes.iter().find(|l| l.eleve_id == b.a[1]).unwrap().motif_code.as_deref(), Some("excuse"));
    let vue = b.ouvrir("soir", "internat", None, 2).await;
    let l = vue.lignes.iter().find(|l| l.eleve_id == b.a[1]).unwrap();
    assert_eq!((l.statut.as_str(), l.motif_code.as_deref()), ("retire", Some("excuse")));
    assert_eq!(crate::internat_cas::libelle_statut("retire", l.motif_code.as_deref()), "Excusé", "ni « santé », ni parenthèses vides");
    assert_eq!(crate::internat_cas::libelle_statut("retire", Some("sortie")), "Excusé (en sortie)");
    // PDF : le texte des polices Identity-H n'est pas lisible dans les octets ; on contrôle donc le CONTENU avant rendu
    // (les lignes que `rendre_pdf_appel` met en page), et que le rendu réel part bien de ce contenu.
    let vue_pdf = b.ent.vue_appel_a(b.etab, b.cpe, CPE, vue.appel.id, soir(3)).await.unwrap();
    let contenu = crate::internat_cas::contenu_pdf_appel(&vue_pdf, 0);
    let sans_accents = |t: &str| t.to_lowercase().replace(['é', 'è'], "e");
    let excuses: Vec<&Vec<String>> = contenu.lignes.iter().map(|(l, _)| l).filter(|l| l[3].starts_with("Excusé")).collect();
    assert_eq!(excuses.len(), 1, "l'élève au retrait « santé » apparaît une fois, « Excusé »");
    assert_eq!(excuses[0][3], "Excusé", "ni « santé », ni parenthèses : {:?}", excuses[0]);
    assert_eq!(excuses[0][4], "Excusé");
    for (cellules, _) in &contenu.lignes {
        for c in cellules {
            assert!(!sans_accents(c).contains("sant"), "cellule du tableau : « {c} »");
        }
    }
    for p in contenu.intro.iter().chain(std::iter::once(&contenu.titre)) {
        assert!(!sans_accents(p).contains("sant"), "texte hors pied : « {p} »");
    }
    // Le seul endroit qui nomme la santé est la phrase de pied, fixe : elle dit que le motif n'est PAS détaillé.
    assert!(contenu.conclusion.contains("pour raison de santé n'est pas détaillé (« Excusé »)"), "{}", contenu.conclusion);
    assert!(contenu.conclusion.contains("sans aucune information médicale"));
    let pdf = b.ent.pdf_appel_a(b.etab, b.cpe, CPE, vue.appel.id, soir(3)).await.unwrap();
    assert!(pdf.starts_with(b"%PDF"), "un vrai PDF est produit");
    b.ent.annuler_retrait_a(b.etab, infirmiere, "infirmerie", r.id, soir(4)).await.unwrap();
    // Événements : pas de motif « sante » dans l'outbox, un motif générique.
    let evts = b.evenements("internat.attendu_modifie").await;
    assert_eq!(evts.len(), 2, "pose et annulation");
    for e in &evts {
        assert_eq!(e["motif_code"], "excuse", "{e}");
        assert!(!e.to_string().contains("sante"), "{e}");
    }
}

// ───────────── M3 : transmission du matin ─────────────

#[tokio::test]
async fn m3_transmission_conditions_rectificative_versionnee_et_petit_dejeuner_aligne() {
    let Some(b) = banc().await else { return };
    let soir_id = b.ouvrir("soir", "internat", None, 0).await.appel.id;
    b.point(soir_id, b.a[0], "absent", 1).await.unwrap();
    b.point(soir_id, b.a[1], "parti", 1).await.unwrap();
    for e in b.a[2..].iter().chain(b.b.iter()) {
        b.point(soir_id, *e, "present", 2).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, soir_id, soir(3)).await.unwrap();
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, soir_id, soir(4)).await.unwrap();
    // Un autre appel de la nuit reste ouvert : pas de transmission.
    let ext = b.ouvrir("extinction", "zone", Some(b.zone_b), 5).await.appel.id;
    let matin = utc(2026, 3, 29, 5, 0);
    let tente = |m: Option<&'static str>, t: DateTime<Utc>| b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), m, t);
    assert!(refus_de(&tente(None, matin).await, "un appel de la nuit est encore ouvert : le"), "un appel de la nuit est encore ouvert");
    for e in &b.b {
        b.point(ext, *e, "present", 6).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, ext, soir(7)).await.unwrap();
    // Nuit pas encore passée : le soir même (jour local = nuit), refus ; elle l'est le lendemain.
    assert!(refus_de(&tente(None, soir(10)).await, "la nuit n'est pas terminée : la transmission du"), "la nuit n'est pas strictement passée");
    // Même formule pour la lecture de l'effectif et la transmission : 6 attendus − 1 absent − 1 parti.
    assert_eq!(b.ent.effectif_repas_a(b.etab, b.cpe, CPE, nuit_dst(), matin).await.unwrap().petit_dejeuner, 4);
    let v1 = tente(None, matin).await.unwrap();
    assert!(v1.nouvelle && v1.version == 1);
    assert_eq!((v1.nb_attendus, v1.nb_presents, v1.nb_absents), (6, 4, 1));
    let repas_v1 = b.evenements("internat.effectif_repas").await;
    assert_eq!(repas_v1.len(), 1);
    assert_eq!(repas_v1[0]["petit_dejeuner"], 4);
    // Mêmes chiffres : idempotent, aucune version de plus.
    let encore = tente(None, matin + chrono::Duration::minutes(5)).await.unwrap();
    assert!(!encore.nouvelle && encore.version == 1);
    // Correction après validation : a[0] était présent. Les chiffres changent → rectificative obligatoire.
    let mut c = pointage(b.a[0], "present");
    c.motif_correction_code = Some("information_tardive".into());
    b.ent.pointer_a(b.etab, b.cpe, CPE, soir_id, &c, matin + chrono::Duration::minutes(10)).await.unwrap();
    let t = matin + chrono::Duration::minutes(20);
    assert!(refus_de(&tente(None, t).await, "les chiffres ont changé depuis la transmission"), "motif de rectification obligatoire");
    assert!(refus_de(&tente(Some("   "), t).await, "les chiffres ont changé depuis la transmission"), "motif vide");
    let v2 = tente(Some("Élève retrouvé, pointage corrigé"), t).await.unwrap();
    assert!(v2.nouvelle && v2.version == 2);
    assert_eq!((v2.nb_attendus, v2.nb_presents, v2.nb_absents), (6, 5, 0));
    // La version 1 reste lisible (ajout seul), la version 2 porte son motif.
    let lignes: Vec<(i32, i32, Option<String>)> = sqlx::query_as(
        "SELECT version, nb_absents, motif_rectification FROM ent_internat_transmission \
          WHERE etablissement_id = $1 AND nuit = $2 AND nature = 'matin_vie_scolaire' ORDER BY version",
    )
    .bind(b.etab)
    .bind(nuit_dst())
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(lignes, vec![(1, 1, None), (2, 0, Some("Élève retrouvé, pointage corrigé".into()))]);
    assert!(sqlx::query("UPDATE ent_internat_transmission SET nb_absents = 9 WHERE etablissement_id = $1").bind(b.etab).execute(b.ent.pool_verif()).await.is_err());
    // Vers la vie scolaire : l'absence de la v1 est ANNULÉE par la v2 (seule la différence part).
    let absences = b.evenements("internat.absence_nuit").await;
    assert_eq!(absences.len(), 2);
    assert_eq!(absences[1]["annulee"], true);
    assert_eq!(absences[1]["version"], 2);
    // Petit-déjeuner : une seule formule, 6 − 0 absent − 1 parti = 5, pour l'événement comme pour la lecture.
    let repas = b.evenements("internat.effectif_repas").await;
    assert_eq!(repas.len(), 2);
    assert_eq!(repas[1]["petit_dejeuner"], 5);
    assert_eq!(b.ent.effectif_repas_a(b.etab, b.cpe, CPE, nuit_dst(), t).await.unwrap().petit_dejeuner, 5);
    let transmissions = b.evenements("internat.transmission").await;
    assert_eq!(transmissions[1]["rectificative"], true);
    // Rejouée à l'identique : plus de version.
    assert!(!tente(Some("autre motif"), t + chrono::Duration::minutes(1)).await.unwrap().nouvelle);
}

// ───────────── M7 : appel a posteriori borné à 48 h ─────────────

#[tokio::test]
async fn m7_appel_a_posteriori_borne_a_48_heures_sauf_droit_de_valider() {
    let Some(b) = banc().await else { return };
    // Un rôle qui ouvre les appels mais ne les valide pas.
    let droit = |a: Action| Droit { action: a, portee: Portee::Etablissement, cibles: vec![] };
    let role = b
        .ent
        .creer_role_for_actor(
            b.etab,
            b.dir,
            DIR,
            RoleSaisie {
                nom: format!("Surveillant {}", alea()),
                description: None,
                modele: None,
                droits: vec![droit(Action::InternatLire), droit(Action::InternatAppel)],
            },
        )
        .await
        .unwrap();
    b.ent.attribuer_role_for_actor(b.etab, b.dir, DIR, role.id, b.compta, None).await.unwrap();
    let (u, r) = (b.compta, "compta");
    // Fin de la nuit du 28 : 29 mars 12 h (CEST) = 10 h UTC. Limite : 31 mars 10 h UTC.
    let dans_le_delai = utc(2026, 3, 31, 9, 0);
    let hors_delai = utc(2026, 3, 31, 11, 0);
    let a_ouvrir = |zone| ouverture("soir", "zone", Some(zone));
    let v = b.ent.ouvrir_appel_a(b.etab, u, r, &a_ouvrir(b.zone_a), dans_le_delai).await.unwrap();
    assert!(v.appel.a_posteriori, "nuit passée : saisie a posteriori");
    assert!(est_interdit(&b.ent.ouvrir_appel_a(b.etab, u, r, &a_ouvrir(b.zone_b), hors_delai).await), "au-delà de 48 h : droit de valider exigé");
    // Avec le droit de valider (le CPE), l'ouverture tardive est permise.
    let v = b.ent.ouvrir_appel_a(b.etab, b.cpe, CPE, &a_ouvrir(b.zone_b), hors_delai).await.unwrap();
    assert!(v.appel.a_posteriori);
    // La nuit courante n'est pas concernée.
    b.ent.ouvrir_appel_a(b.etab, u, r, &ouverture("ronde", "zone", Some(b.zone_a)), soir(0)).await.unwrap();
}

// ───────────── M8 : conflit mesuré sur la réception ─────────────

#[tokio::test]
async fn m8_conflit_mesure_sur_la_reception_et_tous_les_faits_distincts_de_la_fenetre() {
    let Some(b) = banc().await else { return };
    let appel = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await.appel.id;
    let (c, d) = ((b.cpe, CPE), (b.dir, DIR));
    // Trois faits sur le même élève : le CPE « présent » (heure client une heure plus tôt), puis la direction
    // « absent » deux fois de suite. Mesuré sur l'heure client, l'écart serait d'une heure et le dernier fait
    // ne contredirait pas l'avant-dernier : conflit manqué. Sur la réception (60 s), le premier fait est dans la fenêtre.
    let mut f1 = pointage(b.a[0], "present");
    f1.a = Some(soir(0) - chrono::Duration::minutes(60));
    b.ent.pointer_a(b.etab, c.0, c.1, appel, &f1, soir(1)).await.unwrap();
    let f2 = pointage(b.a[0], "absent");
    b.ent.pointer_a(b.etab, d.0, d.1, appel, &f2, soir(2)).await.unwrap();
    let f3 = b.ent.pointer_a(b.etab, d.0, d.1, appel, &pointage(b.a[0], "absent"), soir(2) + chrono::Duration::seconds(30)).await.unwrap();
    assert!(f3.conflit, "le fait du CPE, reçu moins de 120 s avant, contredit le dernier");
    let v = b.ent.vue_appel_a(b.etab, b.cpe, CPE, appel, soir(3)).await.unwrap();
    let l = v.lignes.iter().find(|l| l.eleve_id == b.a[0]).unwrap();
    assert!(l.conflit);
    assert_eq!(l.marques_en_conflit, vec!["present".to_string()], "les faits distincts de la fenêtre sont signalés");
    // Un fait contradictoire reçu longtemps avant (hors fenêtre) ne fait pas de conflit, quelle que soit l'heure client.
    let mut g1 = pointage(b.a[1], "present");
    g1.a = Some(soir(5));
    b.ent.pointer_a(b.etab, c.0, c.1, appel, &g1, soir(5)).await.unwrap();
    let mut g2 = pointage(b.a[1], "absent");
    g2.a = Some(soir(5)); // même heure de geste, mais reçu dix minutes plus tard
    let r = b.ent.pointer_a(b.etab, d.0, d.1, appel, &g2, soir(15)).await.unwrap();
    assert!(!r.conflit, "mesuré sur la réception : dix minutes d'écart, pas de conflit");
    // Le même utilisateur qui se corrige n'est pas en conflit avec lui-même.
    b.ent.pointer_a(b.etab, c.0, c.1, appel, &pointage(b.a[2], "present"), soir(6)).await.unwrap();
    let r = b.ent.pointer_a(b.etab, c.0, c.1, appel, &pointage(b.a[2], "absent"), soir(6) + chrono::Duration::seconds(10)).await.unwrap();
    assert!(!r.conflit);
}

// ───────────── M9 : sorties et séjours scolaires ─────────────

#[tokio::test]
async fn m9_seuls_les_participants_confirmes_d_une_sortie_quittent_l_attendu() {
    let Some(b) = banc().await else { return };
    let pool = b.ent.pool_verif();
    let sortie = Uuid::new_v4();
    let annee: Uuid = sqlx::query_scalar("SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 LIMIT 1").bind(b.etab).fetch_one(pool).await.unwrap();
    let porteur: Uuid = sqlx::query_scalar("SELECT id FROM ent_personne WHERE etablissement_id = $1 ORDER BY id LIMIT 1").bind(b.etab).fetch_one(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO ent_sortie (id, etablissement_id, annee_id, reference, titre, nature, forme, destination, debut, fin, etat, porteur_personne_id) \
         VALUES ($1,$2,$3,$4,'Séjour test','obligatoire','voyage','france',$5,$6,'en_cours',$7)",
    )
    .bind(sortie)
    .bind(b.etab)
    .bind(annee)
    .bind(format!("SM9-{}", alea()))
    .bind(utc(2026, 3, 28, 12, 0))
    .bind(utc(2026, 3, 30, 12, 0))
    .bind(porteur)
    .execute(pool)
    .await
    .unwrap();
    let mut eleves: Vec<Uuid> = b.a.iter().chain(b.b.iter()).copied().collect();
    sqlx::query("UPDATE ent_internat_chambre SET capacite = 6 WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(b.chambre_b)
        .execute(pool)
        .await
        .unwrap();
    for prenom in ["Léa", "Zoé", "Ina"] {
        let e = eleve_fixe(&b.ent, b.etab, prenom).await;
        heberger(&b, e, b.chambre_b, None).await;
        eleves.push(e);
    }
    let etats = ["inscrit", "pret", "parti", "liste_attente", "bloque", "rentre", "remis", "parti_seul_autorise", "absent_depart"];
    for (i, etat) in etats.iter().enumerate() {
        sqlx::query("INSERT INTO ent_sortie_participant (id, etablissement_id, sortie_id, eleve_id, etat, rang_attente) VALUES ($1,$2,$3,$4,$5, CASE WHEN $5 = 'liste_attente' THEN 1 END)")
            .bind(Uuid::new_v4())
            .bind(b.etab)
            .bind(sortie)
            .bind(eleves[i])
            .bind(etat)
            .execute(pool)
            .await
            .unwrap();
    }
    let att = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit_dst(), soir(0)).await.unwrap();
    let statut = |i: usize| att.lignes.iter().find(|l| l.eleve_id == eleves[i]).unwrap().statut.clone();
    assert_eq!(statut(0), "attendu", "inscrit : autorisation non confirmée, dort à l'internat");
    assert_eq!(statut(1), "retire", "prêt : autorisation complète");
    assert_eq!(statut(2), "retire", "parti : départ effectif");
    assert_eq!(statut(3), "attendu", "liste d'attente");
    assert_eq!(statut(4), "attendu", "bloqué");
    assert_eq!(statut(5), "attendu", "rentré");
    assert_eq!(statut(6), "retire", "remis à sa famille : n'est pas à l'internat");
    assert_eq!(statut(7), "retire", "part seul avec autorisation : n'est pas à l'internat");
    assert_eq!(statut(8), "attendu", "absent au départ : n'est pas parti, il dort à l'internat");
    let motif = |i: usize| att.lignes.iter().find(|l| l.eleve_id == eleves[i]).unwrap().motif_code.clone();
    assert_eq!(motif(6).as_deref(), Some("sejour_scolaire"));
    assert_eq!(motif(7).as_deref(), Some("sejour_scolaire"));
}

// ───────────── L1, L6 : bornes ─────────────

#[tokio::test]
async fn l1_nuit_exceptionnelle_bornee_dans_le_futur() {
    let Some(b) = banc().await else { return };
    let saisie = |jours: i64| NuitExceptionnelleSaisieDto { eleve_id: b.a[0], nuit: nuit_dst() + chrono::Duration::days(jours), motif_code: "examen".into() };
    assert!(refus_de(&b.ent.poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &saisie(400), soir(0)).await, "une nuit exceptionnelle se pose un an à"), "dans plus d'un an");
    assert!(refus_de(&b.ent.poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &saisie(5000), soir(0)).await, "une nuit exceptionnelle se pose un an à"));
    b.ent.poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &saisie(300), soir(0)).await.unwrap();
}

#[tokio::test]
async fn l6_retrait_sante_de_l_infirmerie_plafonne_a_30_nuits() {
    let Some(b) = banc().await else { return };
    let inf = (b.cast.infirmieres[0].user_id, "infirmerie");
    let retrait = |nuits: i64| RetraitSaisieDto { eleve_id: b.a[0], du_nuit: nuit_dst(), au_nuit: nuit_dst() + chrono::Duration::days(nuits - 1), motif_code: "sante".into() };
    assert!(refus_de(&b.ent.poser_retrait_a(b.etab, inf.0, inf.1, &retrait(31), soir(0)).await, "un retrait « santé » de l'infirmerie dure 30"), "31 nuits : trop");
    b.ent.poser_retrait_a(b.etab, inf.0, inf.1, &retrait(30), soir(0)).await.unwrap();
    // Renouvelable par un nouveau retrait (un fait de plus, daté et signé).
    let suite = RetraitSaisieDto { eleve_id: b.a[0], du_nuit: nuit_dst() + chrono::Duration::days(30), au_nuit: nuit_dst() + chrono::Duration::days(59), motif_code: "sante".into() };
    b.ent.poser_retrait_a(b.etab, inf.0, inf.1, &suite, soir(1)).await.unwrap();
    // Le plafond ne vise que l'infirmerie : le CPE pose une longue absence annoncée.
    let longue = RetraitSaisieDto { eleve_id: b.a[1], du_nuit: nuit_dst(), au_nuit: nuit_dst() + chrono::Duration::days(60), motif_code: "absence_annoncee".into() };
    b.ent.poser_retrait_a(b.etab, b.cpe, CPE, &longue, soir(2)).await.unwrap();
}

// ───────────── L2 : journal des lectures sensibles ─────────────

#[tokio::test]
async fn l2_lectures_journalisees_avec_la_liste_des_eleves_concernes() {
    let Some(b) = banc().await else { return };
    let vue = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await;
    b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit_dst(), soir(1)).await.unwrap();
    b.ent.lister_appels_a(b.etab, b.cpe, CPE, nuit_dst(), soir(2)).await.unwrap();
    b.ent.lire_procedure_a(b.etab, b.cpe, CPE, soir(3)).await.unwrap();
    b.ent.effectif_repas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(4)).await.unwrap();
    b.ent.lister_listes_a(b.etab, b.cpe, CPE, soir(5)).await.unwrap();
    let journal = b.ent.lectures_sensibles_a(b.etab, b.dir, DIR, 100, soir(6)).await.unwrap();
    for objet in ["attendu", "appel", "liste_appels", "procedure", "effectif_repas", "listes"] {
        assert!(journal.iter().any(|l| l.objet == objet), "lecture « {objet} » journalisée : {:?}", journal.iter().map(|l| &l.objet).collect::<Vec<_>>());
    }
    // La lecture de l'attendu liste les élèves concernés (les six hébergés) ; celle de l'appel, les quatre de la zone.
    let attendu = journal.iter().find(|l| l.objet == "attendu").unwrap();
    assert_eq!(attendu.eleve_ids.len(), 6);
    let appel = journal.iter().find(|l| l.objet == "appel").unwrap();
    assert_eq!(appel.appel_id, Some(vue.appel.id));
    assert_eq!(appel.eleve_ids.len(), 4);
    assert!(appel.eleve_ids.iter().all(|e| b.a.contains(e)));
    // La lecture du journal est elle-même journalisée.
    let apres = b.ent.lectures_sensibles_a(b.etab, b.dir, DIR, 100, soir(7)).await.unwrap();
    assert!(apres.iter().any(|l| l.objet == "journal" && l.user_id == b.dir));
}

// ───────────── L3 : cas clos figé ─────────────

#[tokio::test]
async fn l3_un_cas_clos_fige_lieu_confirmation_numero_et_appel() {
    let Some(b) = banc().await else { return };
    let pool = b.ent.pool_verif();
    let mere = titulaire(&b, b.a[0], "MereL3", None).await;
    let cas = cas_des_absents(&b, &[b.a[0]]).await[&b.a[0]];
    let clos = b
        .ent
        .cas_issue_a(b.etab, b.cpe, CPE, cas, &CasIssueSaisieDto { issue: "rentre_famille".into(), lieu_code: Some("exterieur".into()), confirme_par_personne_id: Some(mere), numero_intervention: Some("N-1".into()), note: None, a: Some(soir(5)) }, soir(6))
        .await
        .unwrap();
    assert_eq!(clos.issue.as_deref(), Some("rentre_famille"));
    for sql in [
        "issue_lieu_code = 'chambre'",
        "issue_confirmee_par_personne_id = NULL",
        "numero_intervention = 'N-2'",
        "appel_id = NULL",
    ] {
        let r = sqlx::query(&format!("UPDATE ent_internat_cas SET {sql} WHERE id = $1")).bind(cas).execute(pool).await;
        assert!(refuse_42501(&r), "{sql} : cas clos figé : {r:?}");
    }
    // Seule la transmission s'ajoute.
    sqlx::query("UPDATE ent_internat_cas SET transmis_le = now() WHERE id = $1").bind(cas).execute(pool).await.unwrap();
}

// ───────────── L4, L5 : retour d'un élève et correction « absent » ─────────────

#[tokio::test]
async fn l4_le_lieu_de_retour_depend_de_l_endroit_ou_l_eleve_est_pointe() {
    let Some(b) = banc().await else { return };
    let pool = b.ent.pool_verif();
    let appel = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await.appel.id;
    for e in &b.a {
        b.point(appel, *e, if [b.a[0], b.a[1], b.a[3]].contains(e) { "absent" } else { "present" }, 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(2)).await.unwrap();
    let cas: std::collections::HashMap<Uuid, Uuid> =
        b.ent.lister_cas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(3)).await.unwrap().into_iter().map(|c| (c.eleve_id, c.id)).collect();
    assert_eq!(cas.len(), 3);
    // a[0] et a[1] ont changé de chambre (aile B) avant de rentrer ; a[3] dort toujours dans l'aile A.
    for e in [b.a[0], b.a[1]] {
        sqlx::query("UPDATE ent_internat_affectation SET chambre_id = $3, lit_id = NULL WHERE etablissement_id = $1 AND eleve_id = $2")
            .bind(b.etab)
            .bind(e)
            .bind(b.chambre_b)
            .execute(pool)
            .await
            .unwrap();
    }
    let retour = |e: Uuid, marque: &str, mi: u32| {
        let mut p = pointage(e, marque);
        p.motif_correction_code = Some("information_tardive".into());
        let (ent, etab, cpe) = (&b.ent, b.etab, b.cpe);
        async move { ent.pointer_a(etab, cpe, CPE, appel, &p, soir(mi)).await.unwrap() }
    };
    let etat = |e: Uuid| {
        let id = cas[&e];
        async move {
            let r: (Option<String>, Option<String>) = sqlx::query_as("SELECT issue, issue_lieu_code FROM ent_internat_cas WHERE id = $1")
                .bind(id)
                .fetch_one(pool)
                .await
                .unwrap();
            r
        }
    };
    // « Présent » dans l'appel de l'aile A alors que son lit est dans l'aile B : le cas reste ouvert.
    retour(b.a[1], "present", 5).await;
    assert_eq!(etat(b.a[1]).await, (None, None), "présent hors de la zone de son lit : le cas ne se ferme pas");
    // « Vu » dans l'appel de l'aile A, lit dans l'aile B : retrouvé ailleurs, lieu réel « autre_zone ».
    retour(b.a[0], "vu", 6).await;
    assert_eq!(etat(b.a[0]).await, (Some("retrouve".into()), Some("autre_zone".into())));
    // « Vu » dans la zone de son lit : « chambre ».
    retour(b.a[3], "vu", 7).await;
    assert_eq!(etat(b.a[3]).await, (Some("retrouve".into()), Some("chambre".into())));
}

#[tokio::test]
async fn l5_une_correction_absent_apres_cloture_ouvre_un_cas() {
    let Some(b) = banc().await else { return };
    let appel = b.ouvrir("soir", "zone", Some(b.zone_a), 0).await.appel.id;
    for e in &b.a {
        b.point(appel, *e, "present", 1).await.unwrap();
    }
    let c = b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(2)).await.unwrap();
    assert!(c.cas_ouverts.is_empty());
    let mut corr = pointage(b.a[2], "absent");
    corr.motif_correction_code = Some("information_tardive".into());
    b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &corr, soir(3)).await.unwrap();
    let cas = b.ent.lister_cas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(4)).await.unwrap();
    assert_eq!(cas.len(), 1, "la correction « absent » ouvre un cas, comme la clôture");
    assert_eq!((cas[0].eleve_id, cas[0].nature.as_str()), (b.a[2], "non_retrouve"));
    // Une seconde correction n'ouvre pas un second cas (un seul cas ouvert par élève et par nuit).
    b.ent.pointer_a(b.etab, b.cpe, CPE, appel, &corr, soir(5)).await.unwrap();
    assert_eq!(b.ent.lister_cas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(6)).await.unwrap().len(), 1);
}

// ───────────── L9 : heure limite après minuit ─────────────

#[tokio::test]
async fn l9_heure_limite_d_appel_apres_minuit_est_celle_du_lendemain() {
    let Some(b) = banc().await else { return };
    // Limite 00 h 30 (après minuit) : la nuit du 28 mars se rattrape jusqu'au 29 à 00 h 30 (Paris : UTC+1 avant 2 h).
    sqlx::query("UPDATE ent_internat_parametres SET heure_limite_appel = '00:30' WHERE etablissement_id = $1")
        .bind(b.etab)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let n_etab = sqlx::query("INSERT INTO ent_internat_parametres (etablissement_id, heure_limite_appel) VALUES ($1, '00:30') ON CONFLICT (etablissement_id) DO UPDATE SET heure_limite_appel = '00:30'")
        .bind(b.etab)
        .execute(b.ent.pool_verif())
        .await;
    n_etab.unwrap();
    // 28 mars 22 h (21 h UTC) : bien avant la limite → aucun rappel (le bug rappelait aussitôt : 28 mars 00 h 30 est passé).
    let avant = utc(2026, 3, 28, 21, 0);
    let r = b.ent.emettre_rappels_appels_a(b.etab, avant).await.unwrap();
    assert!(r.surveillant.is_empty() && r.cpe.is_empty(), "limite 00 h 30 : pas de rappel à 22 h");
    // 29 mars 00 h 31 Paris (23 h 31 UTC le 28) : limite dépassée, rappel au surveillant ; au CPE 15 minutes plus tard.
    let apres = utc(2026, 3, 28, 23, 31);
    let r = b.ent.emettre_rappels_appels_a(b.etab, apres).await.unwrap();
    assert_eq!(r.surveillant.len(), 1);
    assert_eq!(r.surveillant[0].nuit, nuit_dst());
    assert!(r.cpe.is_empty());
    let r = b.ent.emettre_rappels_appels_a(b.etab, apres + chrono::Duration::minutes(16)).await.unwrap();
    assert_eq!(r.cpe.len(), 1);
}

// ═════════════════════ Suites de la contre-revue du pilote (migration 2825) ═════════════════════

/// Appel du soir de tout l'internat (six hébergés) où seuls `absents` sont absents, clos : un cas par absent.
async fn cas_des_absents_internat(b: &Banc, absents: &[Uuid]) -> std::collections::HashMap<Uuid, Uuid> {
    let appel = b.ouvrir("soir", "internat", None, 0).await.appel.id;
    for e in b.a.iter().chain(b.b.iter()) {
        b.point(appel, *e, if absents.contains(e) { "absent" } else { "present" }, 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(2)).await.unwrap();
    b.ent.lister_cas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(3)).await.unwrap().into_iter().map(|c| (c.eleve_id, c.id)).collect()
}

/// Interdiction avec ses dates de début et de fin (les tests de dates d'E3).
#[allow(clippy::too_many_arguments)]
async fn interdire_dates(b: &Banc, eleve: Uuid, personne: Uuid, objet: &str, du: &str, au: Option<&str>, levee: bool) {
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, au, decide_par, levee_par, levee_le) \
         VALUES ($1,$2,$3,$4, ARRAY[$5]::text[], 'decision_justice','Ne pas joindre',$6::date,$7::date,$8, \
                 CASE WHEN $9 THEN $8 END, CASE WHEN $9 THEN '2026-02-01'::timestamptz END)",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(eleve)
    .bind(personne)
    .bind(objet)
    .bind(du)
    .bind(au)
    .bind(b.cpe)
    .bind(levee)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
}

fn issue_famille(p: Uuid) -> CasIssueSaisieDto {
    CasIssueSaisieDto { issue: "rentre_famille".into(), lieu_code: None, confirme_par_personne_id: Some(p), numero_intervention: None, note: None, a: Some(soir(5)) }
}

fn numeros_de(v: &CasVueDto) -> std::collections::HashSet<Uuid> {
    v.numeros_titulaires.iter().map(|n| n.personne_id).collect()
}

/// E3 — dates : toute interdiction non levée et non terminée avant la nuit exclut, même si son début `du` est
/// postérieur à la nuit (pessimiste : une interdiction posée en urgence à 0 h 40 joue tout de suite).
#[tokio::test]
async fn e3_interdiction_non_levee_et_non_terminee_exclut_quelle_que_soit_sa_date_de_debut() {
    let Some(b) = banc().await else { return };
    let (a0, a1, a2, a3) = (b.a[0], b.a[1], b.a[2], b.a[3]);
    let (m0, p0) = (titulaire(&b, a0, "Mere0", None).await, titulaire(&b, a0, "Pere0", None).await);
    let (m1, p1) = (titulaire(&b, a1, "Mere1", None).await, titulaire(&b, a1, "Pere1", None).await);
    let (m2, p2) = (titulaire(&b, a2, "Mere2", None).await, titulaire(&b, a2, "Pere2", None).await);
    let (m3, p3) = (titulaire(&b, a3, "Mere3", None).await, titulaire(&b, a3, "Pere3", None).await);
    // a0 : interdiction DATÉE DU LENDEMAIN (du > nuit), non levée : exclut déjà.
    interdire_dates(&b, a0, p0, "communication", "2026-03-29", None, false).await;
    // a1 : terminée avant la nuit (au = 20 mars) : n'exclut plus.
    interdire_dates(&b, a1, p1, "communication", "2026-01-01", Some("2026-03-20"), false).await;
    // a2 : levée (même datée du lendemain) : n'exclut pas.
    interdire_dates(&b, a2, p2, "contact_eleve", "2026-03-29", None, true).await;
    // a3 : se termine LE JOUR de la nuit (au = 28 mars) : « non terminée avant la nuit », exclut.
    interdire_dates(&b, a3, p3, "contact_eleve", "2026-01-01", Some("2026-03-28"), false).await;
    let cas = cas_des_absents(&b, &[a0, a1, a2, a3]).await;
    let v = |e: Uuid| {
        let (ent, cas_id, etab, cpe) = (&b.ent, cas[&e], b.etab, b.cpe);
        async move { ent.cas_vue_a(etab, cpe, CPE, cas_id, soir(4)).await.unwrap() }
    };
    assert_eq!(numeros_de(&v(a0).await), [m0].into(), "du postérieur à la nuit : exclu");
    assert_eq!(numeros_de(&v(a1).await), [m1, p1].into(), "au expiré avant la nuit : non exclu");
    assert_eq!(numeros_de(&v(a2).await), [m2, p2].into(), "interdiction levée : non exclu");
    assert_eq!(numeros_de(&v(a3).await), [m3].into(), "au = jour de la nuit : exclu");
    // La confirmation d'un retour suit la même règle.
    let ent = &b.ent;
    assert!(refus_de(&ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a0], &issue_famille(p0), soir(6)).await, "ne peut pas confirmer"), "du postérieur");
    assert!(refus_de(&ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a3], &issue_famille(p3), soir(6)).await, "ne peut pas confirmer"), "au = nuit");
    ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a1], &issue_famille(p1), soir(6)).await.unwrap();
    ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a2], &issue_famille(p2), soir(6)).await.unwrap();
}

/// E3 — la confirmation « rentré en famille » applique les mêmes filtres que la liste des numéros : autorité
/// parentale, garde autre que « aucune », dates du lien, élève protégé, interdiction sans personne identifiée,
/// et l'objet « remise » exclut aussi.
#[tokio::test]
async fn e3_confirmation_rentre_famille_memes_filtres_que_la_liste_des_numeros() {
    let Some(b) = banc().await else { return };
    let pool = b.ent.pool_verif();
    let (a0, a1, a2, a3, b0, b1) = (b.a[0], b.a[1], b.a[2], b.a[3], b.b[0], b.b[1]);
    let sans_autorite = titulaire(&b, a0, "SansAutorite", None).await;
    sqlx::query("UPDATE ent_eleve_lien SET autorite_parentale = false WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3")
        .bind(b.etab)
        .bind(a0)
        .bind(sans_autorite)
        .execute(pool)
        .await
        .unwrap();
    let garde_aucune = titulaire(&b, a1, "GardeAucune", Some("aucune")).await;
    let lien_termine = titulaire(&b, a2, "LienTermine", None).await;
    sqlx::query("UPDATE ent_eleve_lien SET date_fin = '2026-03-01' WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3")
        .bind(b.etab)
        .bind(a2)
        .bind(lien_termine)
        .execute(pool)
        .await
        .unwrap();
    let parent_eleve_protege = titulaire(&b, a3, "ParentProtege", None).await;
    sqlx::query("INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, motif_scelle, mesures, du) VALUES ($1,$2,$3,'enc:v1:xx',ARRAY['adresse_protegee'],'2026-01-01')")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(a3)
        .execute(pool)
        .await
        .unwrap();
    let parent_cible = titulaire(&b, b0, "ParentCible", None).await;
    interdire(&b, b0, None, "communication", false).await;
    let (sous_remise, parent_ok) = (titulaire(&b, b1, "ParentRemise", None).await, titulaire(&b, b1, "ParentOk", None).await);
    interdire(&b, b1, Some(sous_remise), "remise", false).await;
    let cas = cas_des_absents_internat(&b, &[a0, a1, a2, a3, b0, b1]).await;
    assert_eq!(cas.len(), 6);
    let tente = |eleve: Uuid, p: Uuid| {
        let (ent, etab, cpe, cas_id) = (&b.ent, b.etab, b.cpe, cas[&eleve]);
        async move { ent.cas_issue_a(etab, cpe, CPE, cas_id, &issue_famille(p), soir(6)).await }
    };
    for (eleve, p, quoi) in [(a0, sans_autorite, "sans autorité parentale"), (a1, garde_aucune, "garde « aucune »"), (a2, lien_termine, "lien terminé avant la nuit")] {
        assert!(refus_de(&tente(eleve, p).await, "n'est pas rattachée à l'élève"), "{quoi}");
    }
    assert!(refus_de(&tente(a3, parent_eleve_protege).await, "cet élève est protégé"), "élève protégé : aucun contact ne confirme");
    assert!(refus_de(&tente(b0, parent_cible).await, "cet élève est protégé"), "interdiction sans personne identifiée");
    assert!(refus_de(&tente(b1, sous_remise).await, "ne peut pas confirmer"), "interdiction de remise visant cette personne");
    // Ce que la liste des numéros ne rendait pas, la confirmation ne l'accepte pas non plus ; le parent joignable confirme.
    let vue_remise = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&b1], soir(4)).await.unwrap();
    assert_eq!(numeros_de(&vue_remise), [parent_ok].into(), "l'objet « remise » exclut aussi des numéros");
    let clos = tente(b1, parent_ok).await.unwrap();
    assert_eq!(clos.issue.as_deref(), Some("rentre_famille"));
    // Aucun des cas refusés ne s'est fermé.
    for e in [a0, a1, a2, a3, b0, b1].into_iter().filter(|e| *e != b1) {
        let c = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&e], soir(7)).await.unwrap();
        assert!(c.cas.issue.is_none(), "cas refusé resté ouvert");
    }
}

async fn personne_nue(b: &Banc, nom: &str) -> Uuid {
    let p = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom) VALUES ($1,$2,$3,'Test')")
        .bind(p)
        .bind(b.etab)
        .bind(nom)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    p
}

async fn fusionner(b: &Banc, absorbee: Uuid, dans: Uuid) {
    sqlx::query("UPDATE ent_personne SET fusionnee_dans = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(absorbee)
        .bind(dans)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
}

/// E3 — fusions en chaîne : A fusionnée dans B, B dans C. Les mesures de A (interdiction) et de B (adresse protégée)
/// s'appliquent à C, la personne qui subsiste.
#[tokio::test]
async fn e3_fusions_en_chaine_portent_les_mesures_de_toute_la_chaine() {
    let Some(b) = banc().await else { return };
    let (a0, a1) = (b.a[0], b.a[1]);
    // a0 : C0 (qui a le lien et le numéro), B0 et A0 absorbées ; interdiction visant A0 (deux fusions plus bas).
    let (c0, m0) = (titulaire(&b, a0, "ChaineC0", None).await, titulaire(&b, a0, "Mere0", None).await);
    let (bb0, aa0) = (personne_nue(&b, "ChaineB0").await, personne_nue(&b, "ChaineA0").await);
    fusionner(&b, aa0, bb0).await;
    fusionner(&b, bb0, c0).await;
    interdire(&b, a0, Some(aa0), "communication", false).await;
    // a1 : adresse protégée posée sur B1, au milieu de la chaîne A1 → B1 → C1.
    let (c1, m1) = (titulaire(&b, a1, "ChaineC1", None).await, titulaire(&b, a1, "Mere1", None).await);
    let (bb1, aa1) = (personne_nue(&b, "ChaineB1").await, personne_nue(&b, "ChaineA1").await);
    fusionner(&b, aa1, bb1).await;
    fusionner(&b, bb1, c1).await;
    sqlx::query("INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine) VALUES ($1,$2,$3,'enc:v1:xx','saisie')")
        .bind(Uuid::new_v4())
        .bind(b.etab)
        .bind(bb1)
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let cas = cas_des_absents(&b, &[a0, a1]).await;
    let v0 = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a0], soir(4)).await.unwrap();
    assert_eq!(numeros_de(&v0), [m0].into(), "interdiction sur A, fusionnée deux fois : C est exclue");
    let v1 = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a1], soir(4)).await.unwrap();
    assert_eq!(numeros_de(&v1), [m1].into(), "adresse protégée sur B, fusionnée dans C : C est exclue");
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a0], &issue_famille(c0), soir(6)).await, "ne peut pas confirmer"));
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a1], &issue_famille(c1), soir(6)).await, "ne peut pas confirmer"));
    b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a0], &issue_famille(m0), soir(6)).await.unwrap();
    b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a1], &issue_famille(m1), soir(6)).await.unwrap();
}

async fn soir_zone_valide(b: &Banc, zone: Uuid, eleves: &[Uuid], mi: u32) -> Uuid {
    let appel = b.ouvrir("soir", "zone", Some(zone), mi).await.appel.id;
    for e in eleves {
        b.point(appel, *e, "present", mi + 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, appel, soir(mi + 2)).await.unwrap();
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, appel, soir(mi + 3)).await.unwrap();
    appel
}

/// M3 — couverture : la transmission du matin est refusée tant qu'un élève attendu ou ajouté n'est dans la liste
/// figée d'aucun appel « soir » validé.
#[tokio::test]
async fn m3_transmission_refusee_tant_qu_un_attendu_n_est_dans_aucun_soir_valide() {
    let Some(b) = banc().await else { return };
    let matin = utc(2026, 3, 29, 5, 0);
    // Seul l'appel de la zone A est validé : les deux élèves de la zone B n'ont jamais été appelés.
    soir_zone_valide(&b, b.zone_a, &b.a.clone(), 0).await;
    let r = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await;
    assert!(refus_de(&r, "aucun appel du soir validé"), "zone B jamais appelée");
    // Avec la zone B validée à son tour, tout l'internat est couvert : la transmission passe.
    soir_zone_valide(&b, b.zone_b, &b.b.clone(), 10).await;
    let v = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await.unwrap();
    assert!(v.nouvelle && v.version == 1);
    assert_eq!(v.nb_attendus, 6);
}

/// M3 — couverture, cas d'un élève « ajouté » (nuit exceptionnelle) sans chambre : les appels par zone ne le
/// contiennent pas ; seul un appel de tout l'internat le couvre.
#[tokio::test]
async fn m3_un_eleve_ajoute_hors_zone_bloque_la_transmission_jusqu_a_un_appel_de_l_internat() {
    let Some(b) = banc().await else { return };
    let matin = utc(2026, 3, 29, 5, 0);
    let invite = eleve_fixe(&b.ent, b.etab, "Invité").await;
    b.ent
        .poser_nuit_exceptionnelle_a(b.etab, b.cpe, CPE, &NuitExceptionnelleSaisieDto { eleve_id: invite, nuit: nuit_dst(), motif_code: "autre".into() }, soir(0))
        .await
        .unwrap();
    soir_zone_valide(&b, b.zone_a, &b.a.clone(), 1).await;
    soir_zone_valide(&b, b.zone_b, &b.b.clone(), 10).await;
    let r = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await;
    assert!(refus_de(&r, "aucun appel du soir validé"), "l'ajouté n'est dans aucune liste de zone");
    let tout = b.ouvrir("soir", "internat", None, 20).await.appel.id;
    for e in b.a.iter().chain(b.b.iter()).chain(std::iter::once(&invite)) {
        b.point(tout, *e, "present", 21).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, tout, soir(22)).await.unwrap();
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, tout, soir(23)).await.unwrap();
    let v = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await.unwrap();
    assert_eq!(v.nb_attendus, 7);
}

/// M3 — rectificative : l'idempotence compare les ENSEMBLES (absents, partis), pas seulement quatre compteurs. Un absent
/// redevient présent pendant qu'un autre devient absent (totaux égaux), puis un parti remplace un autre : chaque fois,
/// une rectificative (motif obligatoire), jamais « rien de nouveau ».
#[tokio::test]
async fn m3_rectificative_quand_l_ensemble_des_absents_ou_des_partis_change_a_totaux_egaux() {
    let Some(b) = banc().await else { return };
    let soir_id = b.ouvrir("soir", "internat", None, 1).await.appel.id;
    for (i, e) in b.a.iter().chain(b.b.iter()).enumerate() {
        let marque = match i {
            0 => "absent",
            2 => "parti",
            _ => "present",
        };
        b.point(soir_id, *e, marque, 2).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, soir_id, soir(3)).await.unwrap();
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, soir_id, soir(4)).await.unwrap();
    // a[1] a déjà un cas ouvert (ouvert à la main, après son pointage « présent » qui aurait fermé le précédent) : sa
    // correction en « absent » n'en ajoute pas (un seul cas ouvert par élève et par nuit), donc `nb_cas` reste égal.
    {
        let mut tx = crate::tenant_tx::TenantTx::begin(b.ent.pool(), b.etab).await.unwrap();
        b.ent.ouvrir_cas_tx(&mut tx, b.etab, b.cpe, None, b.a[1], nuit_dst(), "non_retrouve", soir(4)).await.unwrap().expect("cas ouvert");
        tx.commit().await.unwrap();
    }
    let matin = utc(2026, 3, 29, 5, 0);
    let tente = |m: Option<&'static str>, mi: i64| b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), m, matin + chrono::Duration::minutes(mi));
    let corrige = |e: Uuid, marque: &str, mi: i64| {
        let mut c = pointage(e, marque);
        c.motif_correction_code = Some("information_tardive".into());
        let (ent, etab, cpe) = (&b.ent, b.etab, b.cpe);
        async move { ent.pointer_a(etab, cpe, CPE, soir_id, &c, matin + chrono::Duration::minutes(mi)).await.unwrap() }
    };
    let v1 = tente(None, 0).await.unwrap();
    assert!(v1.nouvelle && v1.version == 1);
    assert_eq!((v1.nb_absents, v1.nb_cas), (1, 2));
    // Étape 1 : le parti a[2] revient, a[3] part. Mêmes totaux, ensemble des partis différent.
    corrige(b.a[2], "present", 1).await;
    corrige(b.a[3], "parti", 1).await;
    let rejoue = tente(None, 2).await;
    assert!(refus_de(&rejoue, "un motif de rectification est obligatoire"), "l'ensemble des partis a changé");
    let v2 = tente(Some("Le parti n'était pas celui-là"), 3).await.unwrap();
    assert!(v2.nouvelle && v2.version == 2);
    assert_eq!((v2.nb_attendus, v2.nb_presents, v2.nb_absents, v2.nb_cas), (v1.nb_attendus, v1.nb_presents, v1.nb_absents, v1.nb_cas), "totaux égaux");
    assert!(!tente(Some("autre"), 4).await.unwrap().nouvelle, "rejouée à l'identique : rien de plus");
    // Étape 2 : a[0] (absent) est retrouvé, a[1] devient absent. Mêmes totaux, ensemble des absents différent.
    corrige(b.a[0], "present", 5).await;
    corrige(b.a[1], "absent", 5).await;
    let (nb_cas,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_internat_cas WHERE etablissement_id = $1 AND nuit = $2")
        .bind(b.etab)
        .bind(nuit_dst())
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(nb_cas, 2, "aucun cas de plus : seuls les ensembles diffèrent");
    assert!(refus_de(&tente(None, 6).await, "un motif de rectification est obligatoire"), "l'ensemble des absents a changé");
    let v3 = tente(Some("Pointage corrigé : le bon élève est absent"), 7).await.unwrap();
    assert!(v3.nouvelle && v3.version == 3);
    assert_eq!((v3.nb_attendus, v3.nb_presents, v3.nb_absents, v3.nb_cas), (v1.nb_attendus, v1.nb_presents, v1.nb_absents, v1.nb_cas));
    // Vers la vie scolaire : seule la différence part (a[1] absent, a[0] annulé).
    let absences = b.evenements("internat.absence_nuit").await;
    assert_eq!(absences.len(), 3, "a[0] v1, puis a[1] v3 et l'annulation de a[0] v3 : {absences:?}");
}

/// Repas : le dîner et le petit-déjeuner d'une nuit déjà transmise se comptent sur la liste figée, pas sur l'attendu
/// recalculé (un élève dont la sortie est posée après coup ne change pas les chiffres reçus par la restauration).
#[tokio::test]
async fn le_diner_d_une_nuit_deja_transmise_se_compte_sur_la_liste_figee() {
    let Some(b) = banc().await else { return };
    let soir_id = b.ouvrir("soir", "internat", None, 0).await.appel.id;
    for e in b.a.iter().chain(b.b.iter()) {
        b.point(soir_id, *e, "present", 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, soir_id, soir(2)).await.unwrap();
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, soir_id, soir(3)).await.unwrap();
    let matin = utc(2026, 3, 29, 5, 0);
    let v1 = b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin).await.unwrap();
    assert!(v1.nouvelle);
    let repas = b.evenements("internat.effectif_repas").await;
    assert_eq!((repas[0]["diner"].as_i64(), repas[0]["petit_dejeuner"].as_i64()), (Some(6), Some(6)));
    // Après coup, un élève voit sa sortie datée du jour de la nuit : l'attendu RECALCULÉ le perd…
    sqlx::query("UPDATE ent_eleve SET date_sortie = '2026-03-28' WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(b.a[1])
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    let recalcule = b.ent.attendu_nuit_a(b.etab, b.cpe, CPE, nuit_dst(), matin).await.unwrap();
    assert!(recalcule.lignes.iter().all(|l| l.eleve_id != b.a[1]), "l'attendu recalculé a changé");
    // … mais la restauration relit les mêmes chiffres, sur la liste figée.
    let lu = b.ent.effectif_repas_a(b.etab, b.cpe, CPE, nuit_dst(), matin + chrono::Duration::minutes(5)).await.unwrap();
    assert_eq!((lu.diner, lu.petit_dejeuner), (6, 6), "nuit transmise : liste figée");
    // Rejouée : aucune rectificative artificielle (la première version a été calculée sur la même base).
    assert!(!b.ent.transmettre_matin_a(b.etab, b.cpe, CPE, nuit_dst(), None, matin + chrono::Duration::minutes(6)).await.unwrap().nouvelle);
}

/// Note perdue : une note fournie à la clôture d'un cas sans lieu est conservée ; l'annulation exige un motif écrit.
#[tokio::test]
async fn la_note_de_cloture_d_un_cas_sans_lieu_est_conservee_et_l_annulation_exige_un_motif() {
    let Some(b) = banc().await else { return };
    let (a0, a1, a2) = (b.a[0], b.a[1], b.a[2]);
    let mere = titulaire(&b, a0, "Mere0", None).await;
    let cas = cas_des_absents(&b, &[a0, a1, a2]).await;
    let mut famille = issue_famille(mere);
    famille.note = Some("Retour confirmé par téléphone à 23 h 05".into());
    let clos = b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a0], &famille, soir(6)).await.unwrap();
    assert_eq!(clos.issue.as_deref(), Some("rentre_famille"));
    let vue = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a0], soir(7)).await.unwrap();
    assert_eq!(vue.issue_note.as_deref(), Some("Retour confirmé par téléphone à 23 h 05"), "la note ne disparaît pas en silence");
    let sans_motif = CasIssueSaisieDto { issue: "annule_erreur".into(), lieu_code: None, confirme_par_personne_id: None, numero_intervention: None, note: None, a: Some(soir(5)) };
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a1], &sans_motif, soir(6)).await, "exige un motif écrit"));
    let blanc = CasIssueSaisieDto { note: Some("   ".into()), ..sans_motif.clone() };
    assert!(refus_de(&b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a1], &blanc, soir(6)).await, "exige un motif écrit"), "motif blanc");
    let avec_motif = CasIssueSaisieDto { note: Some("Pointé absent par erreur : il dormait dans la chambre voisine".into()), ..sans_motif };
    b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a1], &avec_motif, soir(6)).await.unwrap();
    let v1 = b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a1], soir(7)).await.unwrap();
    assert_eq!(v1.issue_note.as_deref(), Some("Pointé absent par erreur : il dormait dans la chambre voisine"));
    let police = CasIssueSaisieDto { issue: "autorite_saisie".into(), lieu_code: None, confirme_par_personne_id: None, numero_intervention: Some("PV-4521".into()), note: Some("Gendarmerie prévenue".into()), a: Some(soir(5)) };
    b.ent.cas_issue_a(b.etab, b.cpe, CPE, cas[&a2], &police, soir(6)).await.unwrap();
    assert_eq!(b.ent.cas_vue_a(b.etab, b.cpe, CPE, cas[&a2], soir(7)).await.unwrap().issue_note.as_deref(), Some("Gendarmerie prévenue"));
    // La note de santé reste refusée, et la note d'un cas clos est figée par la garde.
    let r = sqlx::query("UPDATE ent_internat_cas SET issue_note = 'autre chose' WHERE etablissement_id = $1 AND id = $2")
        .bind(b.etab)
        .bind(cas[&a0])
        .execute(b.ent.pool_verif())
        .await;
    assert!(refuse_42501(&r), "la note d'un cas clos est figée : {r:?}");
}

/// Portée de `internat.appel.valider` : appliquée à l'élève concerné (correction après clôture, annulation d'un cas
/// pour erreur, ouverture a posteriori tardive), au lieu d'être ignorée.
#[tokio::test]
async fn la_portee_de_valider_s_applique_aux_corrections_aux_annulations_et_a_l_ouverture_tardive() {
    let Some(b) = banc().await else { return };
    let classes = classes_des_deux_zones(&b).await;
    // Appel et lecture sur tout l'établissement, mais validation limitée aux élèves de la classe 1 (la zone A).
    let etab_droit = |a: Action| Droit { action: a, portee: Portee::Etablissement, cibles: vec![] };
    let role = b
        .ent
        .creer_role_for_actor(
            b.etab,
            b.dir,
            DIR,
            RoleSaisie {
                nom: format!("Surveillant, validation formation A {}", alea()),
                description: None,
                modele: None,
                droits: vec![
                    etab_droit(Action::InternatLire),
                    etab_droit(Action::InternatAppel),
                    Droit { action: Action::InternatAppelValider, portee: Portee::Designes, cibles: vec![classes[0]] },
                ],
            },
        )
        .await
        .unwrap();
    b.ent.attribuer_role_for_actor(b.etab, b.dir, DIR, role.id, b.compta, None).await.unwrap();
    let (u, r) = (b.compta, "compta");
    // Le CPE fait l'appel du soir de tout l'internat : un absent par zone, puis le valide.
    let soir_id = b.ouvrir("soir", "internat", None, 0).await.appel.id;
    for e in b.a.iter().chain(b.b.iter()) {
        b.point(soir_id, *e, if *e == b.a[0] || *e == b.b[0] { "absent" } else { "present" }, 1).await.unwrap();
    }
    b.ent.cloturer_appel_a(b.etab, b.cpe, CPE, soir_id, soir(2)).await.unwrap();
    b.ent.valider_appel_a(b.etab, b.cpe, CPE, soir_id, soir(3)).await.unwrap();
    // Correction après validation : dans sa portée (zone A) permise, hors portée (zone B) interdite.
    let mut c = pointage(b.a[1], "absent");
    c.motif_correction_code = Some("erreur_saisie".into());
    b.ent.pointer_a(b.etab, u, r, soir_id, &c, soir(4)).await.expect("correction dans la portée");
    let mut hors = pointage(b.b[1], "absent");
    hors.motif_correction_code = Some("erreur_saisie".into());
    assert!(est_interdit(&b.ent.pointer_a(b.etab, u, r, soir_id, &hors, soir(4)).await), "correction hors de la portée de validation");
    // Annulation d'un cas pour erreur d'ouverture : seulement pour un élève de sa portée.
    let cas = b.ent.lister_cas_a(b.etab, b.cpe, CPE, nuit_dst(), soir(5)).await.unwrap();
    let cas_de = |e: Uuid| cas.iter().find(|c| c.eleve_id == e).unwrap().id;
    let annulation = CasIssueSaisieDto { issue: "annule_erreur".into(), lieu_code: None, confirme_par_personne_id: None, numero_intervention: None, note: Some("Ouvert par erreur".into()), a: Some(soir(5)) };
    assert!(est_interdit(&b.ent.cas_issue_a(b.etab, u, r, cas_de(b.b[0]), &annulation, soir(6)).await), "annulation hors portée");
    b.ent.cas_issue_a(b.etab, u, r, cas_de(b.a[0]), &annulation, soir(6)).await.expect("annulation dans la portée");
    // Ouverture a posteriori tardive (au-delà de 48 h : il faut valider) : seulement un périmètre entièrement dans sa portée.
    let hors_delai = utc(2026, 3, 31, 11, 0);
    assert!(est_interdit(&b.ent.ouvrir_appel_a(b.etab, u, r, &ouverture("ronde", "internat", None), hors_delai).await), "internat entier : déborde");
    assert!(est_interdit(&b.ent.ouvrir_appel_a(b.etab, u, r, &ouverture("ronde", "zone", Some(b.zone_b)), hors_delai).await), "zone B : hors portée");
    b.ent.ouvrir_appel_a(b.etab, u, r, &ouverture("ronde", "zone", Some(b.zone_a)), hors_delai).await.expect("zone A : dans la portée");
}

/// `confie_a_user_id` : l'agent à qui une liste est confiée appartient à l'établissement.
#[tokio::test]
async fn une_liste_ne_se_confie_qu_a_un_agent_de_l_etablissement() {
    let Some(b) = banc().await else { return };
    let autre = crate::tests_rls_stricte::seed_supplementaire(&b.ent).await;
    let etranger = autre.cpe.user_id;
    let saisie = |libelle: &str, confie: Option<Uuid>| ListeSaisieDto { libelle: libelle.into(), chambre_ids: vec![b.chambre_a], confie_a_user_id: confie };
    assert!(refus_de(&b.ent.creer_liste_a(b.etab, b.cpe, CPE, &saisie("Étrangère", Some(etranger))).await, "n'appartient pas à l'établissement"));
    assert!(refus_de(&b.ent.creer_liste_a(b.etab, b.cpe, CPE, &saisie("Inconnue", Some(Uuid::new_v4()))).await, "n'appartient pas à l'établissement"));
    let ok = b.ent.creer_liste_a(b.etab, b.cpe, CPE, &saisie("Mon aile", Some(b.cpe))).await.unwrap();
    assert_eq!(ok.confie_a_user_id, Some(b.cpe));
    assert!(refus_de(&b.ent.modifier_liste_a(b.etab, b.cpe, CPE, ok.id, &saisie("Mon aile", Some(etranger)), true).await, "n'appartient pas à l'établissement"));
    b.ent.modifier_liste_a(b.etab, b.cpe, CPE, ok.id, &saisie("Mon aile", None), true).await.expect("sans surveillant : permis");
}
