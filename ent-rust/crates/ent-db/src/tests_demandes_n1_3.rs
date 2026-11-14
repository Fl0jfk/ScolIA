//! Socle N1 (demandes), lot N1-3 : **HSE** (en heures, validées par la direction, D-061) et **reprographie**, contre la
//! base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, les tests sont ignorés en local (échec explicite en
//! CI). Chaque test crée son propre établissement : il ne vérifie que ses propres lignes. Aucun test ne dépend de
//! l'heure réelle : le jour est celui de l'établissement, les dates passées sont posées par SQL.
//!
//! Défauts du registre `ALERTES_NEXTJS_PROD.md` (§5 ter) fermés par ce lot (règle 14 de `PILOTAGE.md`) :
//!
//! | Défaut | Test(s) |
//! |---|---|
//! | NJ-91 (adresse codée en dur, acceptation sans contrôle, déposant sans trace) | `t_nj91_aucun_destinataire_code`, `t_nj91_decision_directeur_seule`, `t_nj91_attestation_facultative_desactivee_par_defaut`, `t_nj91_deposant_trace` |
//! | NJ-92 (« prête » par un lien public de 30 jours) | `t_nj92_pret_par_geste_authentifie` |
//! | NJ-77 (file de photocopies de tous les sites) | `t_nj77_file_par_site` |
//! | NJ-78 (PDF d'un autre rattachable) | `t_nj78_document_du_deposant` |
//!
//! Règles et cas : R-DM40 (`t_nj91_decision_directeur_seule`), R-DM42 (`t_hse_transmission_idempotente`), R-DM43
//! (`t_nj91_deposant_trace`), R-DM44 (`t_tables_n1_3_sans_montant_ni_json`), R-DM45 (tables `ent_dem_repro*`), R-DM46
//! (`t_nj77_file_par_site`), R-DM50 (`t_nj91_attestation_facultative_desactivee_par_defaut`), CDM-41, CDM-42
//! (`t_hse_alerte_fin_periode`), CDM-43, CDM-46 (`t_hse_declaration_et_avenant`), CDM-45, CDM-67
//! (`t_hse_enveloppe_en_heures`), CDM-51 à CDM-56 (`t_nj77_file_par_site`, `t_repro_avertissements`,
//! `t_repro_probleme_et_transfert`, `t_repro_rappel_et_effacement`), DEM-366 à DEM-368
//! (`t_repro_dupliquer_et_recurrence`, `t_repro_multi_cibles_et_effectifs`), DEM-376, DEM-377
//! (`t_hse_enveloppe_en_heures`, `t_hse_confidentialite`).

use super::*;
use crate::demandes::*;
use crate::demandes_echanges::*;
use crate::demandes_files::*;
use crate::demandes_hse::*;
use crate::demandes_repro::*;
use crate::demandes_routage::*;
use crate::tests_demandes::{depot, fiche, labo, monde, Acteur, Monde};
use chrono::{NaiveDate, NaiveTime};
use std::sync::Mutex;
use uuid::Uuid;

// ───────────────────────── Ports de test ─────────────────────────

/// Socle 15 de test : il constate les heures (ou refuse, comme tant que `ent_absp_heure` n'est pas versé).
struct Heures {
    actif: bool,
    appels: Mutex<Vec<HeureAConstater>>,
}

impl Heures {
    fn actif() -> Self {
        Self { actif: true, appels: Mutex::new(Vec::new()) }
    }
    fn nb(&self) -> usize {
        self.appels.lock().unwrap().len()
    }
}

impl PortHeures for Heures {
    fn constater(&self, _: Uuid, h: &HeureAConstater) -> Option<Uuid> {
        self.appels.lock().unwrap().push(*h);
        self.actif.then(Uuid::new_v4)
    }
}

/// Socle 7 de test : des documents déposés par des personnes, avec leur nombre de pages ; effacement réglable.
#[derive(Default)]
struct Docs {
    propres: Vec<(Uuid, Uuid)>,
    pages: Vec<(Uuid, i32)>,
    efface_ok: bool,
}

impl Docs {
    fn doc(&mut self, qui: &Acteur, pages: i32) -> Uuid {
        let d = Uuid::new_v4();
        self.propres.push((d, qui.personne));
        self.pages.push((d, pages));
        d
    }
}

impl PortDocuments for Docs {
    fn document_depose_par(&self, _: Uuid, document_id: Uuid, personne_id: Uuid) -> bool {
        self.propres.contains(&(document_id, personne_id))
    }
    fn jeton_lecture(&self, _: Uuid, _: Uuid, _: Uuid) -> Option<String> {
        Some(format!("jeton-court-{}", Uuid::new_v4()))
    }
    fn pages_pdf(&self, _: Uuid, document_id: Uuid) -> Option<i32> {
        self.pages.iter().find(|(d, _)| *d == document_id).map(|(_, p)| *p)
    }
    fn effacer(&self, _: Uuid, _: Uuid) -> bool {
        self.efface_ok
    }
}

// ───────────────────────── Le monde de test ─────────────────────────

/// Un établissement avec : un repli, la file direction (direction 1 **et** direction 2 membres), la file reprographie,
/// les natures HSE et copies activées, deux sites, un deuxième professeur et un agent de reprographie par site.
struct W {
    ent: PgEnt,
    m: Monde,
    jour: NaiveDate,
    debut_annee: NaiveDate,
    fin_annee: NaiveDate,
    dir2: Acteur,
    prof2: Acteur,
    adm: Acteur,
    site1: Uuid,
    site2: Uuid,
    lieu1: Uuid,
    lieu2: Uuid,
    classe: Uuid,
}

fn code_aleatoire(prefixe: &str) -> String {
    format!("{prefixe}{}", &Uuid::new_v4().simple().to_string()[..12])
}

impl W {
    async fn nouveau() -> Option<W> {
        let (ent, cast) = labo().await?;
        let m = monde(&ent, &cast).await;
        let etab = m.etab;
        let pool = ent.pool_verif();
        let dir2 = fiche(&ent, etab, cast.directions[1].user_id, "direction", true).await;
        let prof2 = fiche(&ent, etab, cast.profs[1].user_id, "prof", true).await;
        let adm = fiche(&ent, etab, cast.administratif.user_id, "administratif", true).await;
        let (jour,): (NaiveDate,) =
            sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())").bind(etab).fetch_one(pool).await.unwrap();
        let (debut_annee, fin_annee): (NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT date_debut, date_fin FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND (courante OR (date_debut <= $2 AND date_fin >= $2))
              ORDER BY courante DESC, date_debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(jour)
        .fetch_one(pool)
        .await
        .unwrap();
        let mut sites = Vec::new();
        for code in [code_aleatoire("s1"), code_aleatoire("s2")] {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_site (id, etablissement_id, code, libelle) VALUES ($1, $2, $3, $3)")
                .bind(id)
                .bind(etab)
                .bind(code)
                .execute(pool)
                .await
                .unwrap();
            sites.push(id);
        }
        let (classe,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_classe WHERE etablissement_id = $1 ORDER BY code LIMIT 1")
            .bind(etab)
            .fetch_one(pool)
            .await
            .unwrap();
        let mut w = W {
            ent,
            m,
            jour,
            debut_annee,
            fin_annee,
            dir2,
            prof2,
            adm,
            site1: sites[0],
            site2: sites[1],
            lieu1: Uuid::nil(),
            lieu2: Uuid::nil(),
            classe,
        };
        let dir = w.m.direction;
        let nf = |code: String, type_: TypeFile, repli: Option<Uuid>, role: &str| NouvelleFile {
            code,
            libelle: format!("Service {role}"),
            role_traitant: Some(role.to_string()),
            type_,
            site_id: None,
            service_calendrier: None,
            delai_vise_jours: None,
            message_accueil: None,
            repli_id: repli,
        };
        let repli = w.ent.creer_file_for_actor(etab, dir.user, "direction", nf(code_aleatoire("rp"), TypeFile::Repli, None, "la direction")).await.unwrap();
        let fdir = w.ent.creer_file_for_actor(etab, dir.user, "direction", nf(code_aleatoire("fd"), TypeFile::Direction, Some(repli), "la direction")).await.unwrap();
        let frep = w.ent.creer_file_for_actor(etab, dir.user, "direction", nf(code_aleatoire("fr"), TypeFile::Service, Some(repli), "la reprographie")).await.unwrap();
        let (dir1, dir2, adm) = (w.m.direction, w.dir2, w.adm);
        for (file, qui, resp, rang) in [(fdir, dir1, true, 1), (fdir, dir2, false, 2), (frep, adm, true, 1), (repli, dir1, true, 1)] {
            let r = w
                .ent
                .ajouter_regle_membre_for_actor(
                    etab,
                    dir.user,
                    "direction",
                    file,
                    RegleMembreSaisie { source: SourceMembre::Agent { agent_id: qui.agent.unwrap() }, responsable: resp, rang, du: None, au: None },
                    false,
                )
                .await
                .unwrap();
            assert!(r.applique);
        }
        for (code, file) in [(NATURE_HSE, fdir), (NATURE_REPRO, frep)] {
            let decl = nature(code).expect("nature");
            let fc: (String,) = sqlx::query_as("SELECT code FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(file)
                .fetch_one(w.ent.pool_verif())
                .await
                .unwrap();
            w.ent
                .regler_nature_for_actor(
                    etab,
                    dir.user,
                    "direction",
                    code,
                    ReglageNature { actif: true, publics: decl.publics.to_vec(), file_defaut_code: Some(fc.0), ..Default::default() },
                )
                .await
                .unwrap_or_else(|e| panic!("activation de {code} : {e:?}"));
        }
        w.lieu1 = w.ent.ajouter_lieu_retrait_for_actor(etab, dir.user, "direction", w.site1, "Salle des professeurs").await.unwrap();
        w.lieu2 = w.ent.ajouter_lieu_retrait_for_actor(etab, dir.user, "direction", w.site2, "Accueil du site 2").await.unwrap();
        Some(w)
    }

    fn etab(&self) -> Uuid {
        self.m.etab
    }

    async fn src(&self, qui: &Acteur) -> SourcesFiles {
        self.ent.sources_demandes_for_actor(self.etab(), qui.user, qui.role, None).await.expect("sources")
    }

    // ── HSE ──

    /// Période de la demande : dans l'année scolaire, autour d'aujourd'hui.
    fn periode(&self) -> (NaiveDate, NaiveDate) {
        let du = (self.jour - chrono::Duration::days(5)).max(self.debut_annee);
        let au = (self.jour + chrono::Duration::days(5)).min(self.fin_annee);
        (du, au)
    }

    fn hse(&self, volume: i32, seances: Vec<SeanceSaisie>) -> DepotHse {
        let (periode_du, periode_au) = self.periode();
        DepotHse {
            motif: "soutien".into(),
            objet: "Soutien avant le brevet".into(),
            precisions: Some("Groupe de 3e, deux séances.".into()),
            volume_quarts: volume,
            periode_du,
            periode_au,
            lieu: Some("Salle 12".into()),
            classes: vec![self.classe],
            groupes: vec![],
            seances,
            pour_le_compte_de: None,
        }
    }

    fn seance(&self, quarts: i32, fait: bool) -> SeanceSaisie {
        SeanceSaisie { jour: self.jour, debut: NaiveTime::from_hms_opt(17, 0, 0), duree_quarts: quarts, fait, lieu: None }
    }

    async fn deposer_hse(&self, qui: &Acteur, d: DepotHse) -> Result<EtatDemande, RefusDemande> {
        let dep = d.vers_depot_demande()?;
        let s = self.ent.sources_demandes_for_actor(self.etab(), qui.user, qui.role, Some(&dep)).await.expect("sources");
        self.ent.deposer_hse_for_actor(self.etab(), qui.user, qui.role, &s, d).await
    }

    async fn version(&self, id: Uuid) -> i32 {
        let (v,): (i32,) = sqlx::query_as("SELECT version FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
            .bind(self.etab())
            .bind(id)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        v
    }

    async fn decider(&self, qui: &Acteur, heures: &dyn PortHeures, id: Uuid, d: DecisionHse, o: OptionsDecision) -> Result<ResultatDecisionHse, RefusDemande> {
        let s = self.src(qui).await;
        let v = self.version(id).await;
        self.ent.decider_hse_for_actor(self.etab(), qui.user, qui.role, &s, heures, id, v, d, o).await
    }

    async fn accorder(&self, id: Uuid, heures: &dyn PortHeures) -> ResultatDecisionHse {
        let d = self.dir2;
        self.decider(&d, heures, id, DecisionHse::Accorder, OptionsDecision::default()).await.expect("décision")
    }

    async fn detail(&self, qui: &Acteur, id: Uuid) -> Result<DetailHse, RefusDemande> {
        let s = self.src(qui).await;
        self.ent.detail_hse_for_actor(self.etab(), qui.user, qui.role, &s, id).await
    }

    async fn declarer(&self, qui: &Acteur, heures: &dyn PortHeures, id: Uuid, s: SeanceSaisie) -> Result<ResultatSeance, RefusDemande> {
        let src = self.src(qui).await;
        self.ent.declarer_seance_for_actor(self.etab(), qui.user, qui.role, &src, heures, id, s).await
    }

    /// (séances transmises, dont au port 15, dont à l'export de secours)
    async fn transmises(&self, id: Uuid) -> (i64, i64, i64) {
        sqlx::query_as(
            "SELECT count(*) FILTER (WHERE transmission <> 'aucune'),
                    count(*) FILTER (WHERE transmission = 'port_15'),
                    count(*) FILTER (WHERE transmission = 'export_secours')
               FROM ent_dem_hse_seance WHERE etablissement_id = $1 AND demande_id = $2",
        )
        .bind(self.etab())
        .bind(id)
        .fetch_one(self.ent.pool_verif())
        .await
        .unwrap()
    }

    async fn faits(&self, id: Uuid) -> Vec<String> {
        let l: Vec<(String,)> = sqlx::query_as("SELECT type FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY rang")
            .bind(self.etab())
            .bind(id)
            .fetch_all(self.ent.pool_verif())
            .await
            .unwrap();
        l.into_iter().map(|(t,)| t).collect()
    }

    // ── Reprographie ──

    async fn designer(&self, site: Uuid, qui: &Acteur) {
        let dir = self.m.direction;
        self.ent
            .designer_receptionnaire_for_actor(self.etab(), dir.user, "direction", site, qui.agent.unwrap(), true)
            .await
            .unwrap();
    }

    fn copies(&self, docs: Vec<Uuid>, site: Uuid) -> DepotRepro {
        DepotRepro {
            site_id: site,
            objet: "Polycopiés de la semaine".into(),
            motif: None,
            documents: docs,
            format: "a4".into(),
            couleur: false,
            recto_verso: true,
            agrafage: false,
            cibles: vec![],
            exemplaires: Some(30),
            pour_le: self.jour + chrono::Duration::days(7),
            pour_heure: NaiveTime::from_hms_opt(8, 0, 0),
            retrait_lieu_id: None,
            oeuvres: vec![],
            pour_le_compte_de: None,
        }
    }

    async fn deposer_copies(&self, qui: &Acteur, docs: &Docs, d: DepotRepro) -> Result<ResultatRepro, RefusDemande> {
        let dep = d.pour_routage();
        let s = self.ent.sources_demandes_for_actor(self.etab(), qui.user, qui.role, Some(&dep)).await.expect("sources");
        self.ent.deposer_repro_for_actor(self.etab(), qui.user, qui.role, &s, docs, d).await
    }

    /// Une demande de copies en noir et blanc (sous le seuil), déposée par le prof : prête pour l'impression.
    async fn copies_simples(&self, docs: &mut Docs, site: Uuid) -> ResultatRepro {
        let prof = self.m.prof;
        let doc = docs.doc(&prof, 10);
        self.deposer_copies(&prof, docs, self.copies(vec![doc], site)).await.expect("dépôt")
    }
}

// ───────────────────────── HSE : dépôt, schéma, NJ-91 ─────────────────────────

#[tokio::test]
async fn t_hse_depot_atomique_et_en_heures() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let prof = w.m.prof;
    let e = w.deposer_hse(&prof, w.hse(24, vec![w.seance(8, true), w.seance(4, false)])).await.expect("dépôt");
    assert_eq!(e.statut, "deposee");
    let d = w.detail(&prof, e.id).await.unwrap();
    assert_eq!((d.volume_demande_quarts, d.volume_accorde_quarts, d.decision.clone()), (24, None, None));
    assert_eq!(d.type_heure, "hse_enseignement");
    assert_eq!(d.seances.len(), 2);
    assert_eq!(d.seances.iter().filter(|s| s.etat == "faite").count(), 1);
    assert!(!d.deposee_pour_compte && !d.confirmation_requise);
    // Heures décimales, calcul entier : 25 quarts = 6,25 h ; aucun montant nulle part.
    assert_eq!(heures_decimales(25), "6.25");
    assert_eq!(heures_decimales(24), "6.00");
    assert_eq!(heures_decimales(1), "0.25");
    // Le volet et la demande sont écrits ensemble : une saisie refusée dans la transaction ne laisse rien.
    let avant: (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_demande WHERE etablissement_id = $1 AND nature_code = $2")
        .bind(w.etab())
        .bind(NATURE_HSE)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    let mut mauvais = w.hse(24, vec![]);
    mauvais.classes = vec![Uuid::new_v4()];
    assert!(matches!(w.deposer_hse(&prof, mauvais).await, Err(RefusDemande::Invalide(_))));
    let apres: (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_demande WHERE etablissement_id = $1 AND nature_code = $2")
        .bind(w.etab())
        .bind(NATURE_HSE)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(avant.0, apres.0, "aucune demande orpheline");
    // Une demande `rh.hse` sans son volet ne se dépose pas par la voie générique.
    let mut g = depot(NATURE_HSE);
    g.valeurs = vec![ValeurSaisie { champ: "motif".into(), rang: 0, valeur: Valeur::Code("soutien".into()) }];
    let s = w.src(&prof).await;
    let r = w.ent.deposer_demande_for_actor(w.etab(), prof.user, prof.role, &s, g).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
    // Contrôles de saisie : séances au-delà du volume, motif inconnu, séance à venir déclarée faite, passée « prévue ».
    assert!(matches!(w.deposer_hse(&prof, w.hse(4, vec![w.seance(8, true)])).await, Err(RefusDemande::Invalide(_))));
    let mut m = w.hse(8, vec![]);
    m.motif = "vacances".into();
    assert!(matches!(w.deposer_hse(&prof, m).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(w.deposer_hse(&prof, w.hse(0, vec![])).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(w.deposer_hse(&prof, w.hse(2000, vec![])).await, Err(RefusDemande::Invalide(_))));
    // Un compte famille ne dépose pas d'HSE (nature réservée au personnel).
    let parent = w.m.parent;
    let dep = w.hse(8, vec![]).vers_depot_demande().unwrap();
    let sp = w.ent.sources_demandes_for_actor(w.etab(), parent.user, parent.role, Some(&dep)).await.unwrap();
    let r = w.ent.deposer_hse_for_actor(w.etab(), parent.user, parent.role, &sp, w.hse(8, vec![])).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn t_tables_n1_3_sans_montant_ni_json() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    // R-DM44 : aucun montant ; D-036 : aucun JSON, aucun tableau, aucun binaire ; NJ-91/NJ-92 : aucune adresse ni jeton.
    let colonnes: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text, data_type::text FROM information_schema.columns
          WHERE table_schema = current_schema() AND (table_name LIKE 'ent\\_dem\\_hse%' OR table_name LIKE 'ent\\_dem\\_repro%')",
    )
    .fetch_all(w.ent.pool_verif())
    .await
    .unwrap();
    assert!(colonnes.len() > 60, "tables N1-3 présentes");
    for (t, c, ty) in &colonnes {
        assert!(!matches!(ty.as_str(), "json" | "jsonb" | "bytea" | "ARRAY"), "{t}.{c} : type {ty} interdit (D-036)");
        let bas = c.to_lowercase();
        for interdit in ["euro", "montant", "taux", "tarif", "prix", "cout", "email", "courriel", "mail", "telephone", "token", "jeton", "secret", "hash", "url", "nom_fichier", "filename"] {
            assert!(!bas.contains(interdit), "{t}.{c} : « {interdit} » n'a rien à faire dans ces tables");
        }
    }
    // Les migrations ne contiennent ni JSON ni tableau (contrôle de revue du §31).
    for sql in [include_str!("../../../migrations/3046_demandes_n1_3_hse.sql"), include_str!("../../../migrations/3047_demandes_n1_3_reprographie.sql")] {
        let code: String = sql.lines().filter(|l| !l.trim_start().starts_with("--")).collect::<Vec<_>>().join("\n").to_lowercase();
        assert!(!code.contains("json") && !code.contains("bytea") && !code.contains("[]"), "migration N1-3 hors D-036");
    }
}

#[tokio::test]
async fn t_nj91_aucun_destinataire_code() {
    // NJ-91 : le Next.js envoyait les HSE acceptées (avec le PDF) à une adresse nominative codée en dur. Ici : aucune
    // adresse dans le code, aucune colonne d'adresse, aucune pièce jointe ; l'événement est une enveloppe neutre.
    let source = include_str!("demandes_hse.rs");
    for (i, l) in source.lines().enumerate() {
        assert!(!l.contains('@'), "demandes_hse.rs:{} contient « @ » : {l}", i + 1);
    }
    assert!(!source.to_lowercase().contains("hse_ops_email"));
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let prof = w.m.prof;
    let e = w.deposer_hse(&prof, w.hse(8, vec![w.seance(8, true)])).await.unwrap();
    let h = Heures::actif();
    w.accorder(e.id, &h).await;
    let charges: Vec<(String, String)> = sqlx::query_as(
        "SELECT type, payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2",
    )
    .bind(w.etab())
    .bind(e.id)
    .fetch_all(w.ent.pool_verif())
    .await
    .unwrap();
    let types: Vec<&str> = charges.iter().map(|(t, _)| t.as_str()).collect();
    assert!(types.contains(&"demande.hse_decidee") && types.contains(&"demande.hse_transmise"), "{types:?}");
    for (_, p) in &charges {
        assert!(!p.contains('@') && !p.contains("quarts") && !p.contains("volume"), "enveloppe non neutre : {p}");
        controler_enveloppe(&serde_json::from_str(p).unwrap()).expect("enveloppe neutre");
    }
    // Aucune pièce n'est rattachée à une HSE (le PDF d'acceptation n'existe pas en tant que pièce jointe).
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_piece WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(e.id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn t_nj91_decision_directeur_seule() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, dir1, dir2, cpe, prof2) = (w.m.prof, w.m.direction, w.dir2, w.m.cpe, w.prof2);
    let h = Heures::actif();
    let e = w.deposer_hse(&prof, w.hse(8, vec![w.seance(8, true)])).await.unwrap();
    // Le professeur lui-même, un collègue, la vie scolaire : aucun ne décide (404 neutre ou refus).
    for qui in [prof, prof2, cpe] {
        let r = w.decider(&qui, &h, e.id, DecisionHse::Accorder, OptionsDecision::default()).await;
        assert!(matches!(r, Err(RefusDemande::Introuvable | RefusDemande::Interdit)), "{} : {r:?}", qui.role);
    }
    // Un volume accordé supérieur au demandé n'existe pas : seules trois décisions, partielle < demandée.
    let r = w.decider(&dir2, &h, e.id, DecisionHse::AccorderEnPartie { volume_quarts: 8, motif: "budget".into() }, OptionsDecision::default()).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
    let r = w.decider(&dir2, &h, e.id, DecisionHse::AccorderEnPartie { volume_quarts: 4, motif: "libre".into() }, OptionsDecision::default()).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))), "motif codé obligatoire");
    let r = w.decider(&dir2, &h, e.id, DecisionHse::Refuser { motif: String::new() }, OptionsDecision::default()).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
    // CDM-41 : le chef d'établissement demande des HSE ; il ne décide pas sa propre demande (R-DM40).
    let e2 = w.deposer_hse(&dir1, w.hse(8, vec![])).await.unwrap();
    let r = w.decider(&dir1, &h, e2.id, DecisionHse::Accorder, OptionsDecision::default()).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
    // …et la base le tient aussi : un UPDATE direct qui fait décider le bénéficiaire est refusé.
    let r = sqlx::query(
        "UPDATE ent_dem_hse SET decision = 'accordee', volume_accorde_quarts = 8, decide_par_personne_id = $3, decide_le = now()
          WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(w.etab())
    .bind(e2.id)
    .bind(dir1.personne)
    .execute(w.ent.pool_verif())
    .await;
    assert!(r.is_err(), "la base refuse qu'on décide sa propre demande");
    // Le déposant ne décide pas non plus ce qu'il a déposé pour un collègue (R-DM40) ; l'autre direction décide.
    let mut pc = w.hse(8, vec![]);
    pc.pour_le_compte_de = Some(prof.personne);
    let e3 = w.deposer_hse(&dir1, pc).await.unwrap();
    w.ent.confirmer_hse_for_actor(w.etab(), prof.user, prof.role, &w.src(&prof).await, e3.id).await.unwrap();
    let r = w.decider(&dir1, &h, e3.id, DecisionHse::Accorder, OptionsDecision::default()).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(w.decider(&dir2, &h, e3.id, DecisionHse::Accorder, OptionsDecision::default()).await.unwrap().decision, "accordee");
    // La direction 2 décide la demande de la direction 1 ; une seule décision, jamais deux.
    let ok = w.decider(&dir2, &h, e2.id, DecisionHse::Accorder, OptionsDecision::default()).await.unwrap();
    assert_eq!((ok.decision.as_str(), ok.volume_accorde_quarts), ("accordee", 8));
    let r = w.decider(&dir2, &h, e2.id, DecisionHse::Accorder, OptionsDecision::default()).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_) | RefusDemande::Interdit)));
    // Une version périmée est refusée (409).
    let s = w.src(&dir2).await;
    let r = w.ent.decider_hse_for_actor(w.etab(), dir2.user, dir2.role, &s, &h, e.id, 99, DecisionHse::Accorder, OptionsDecision::default()).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))));
    // La décision est un fait nominatif et daté (aucune signature-image, aucun courriel).
    assert!(w.faits(e2.id).await.contains(&"hse_decidee".to_string()));
    let (par, n): (Uuid, i64) = sqlx::query_as(
        "SELECT decide_par_personne_id, (SELECT count(*) FROM ent_dem_hse_decision d WHERE d.etablissement_id = h.etablissement_id AND d.demande_id = h.demande_id)
           FROM ent_dem_hse h WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(w.etab())
    .bind(e2.id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((par, n), (dir2.personne, 1));
}

#[tokio::test]
async fn t_nj91_deposant_trace() {
    // NJ-91 : le dépôt « pour le compte » enregistrait le collègue comme auteur sans trace du déposant.
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, adm, dir2) = (w.m.prof, w.adm, w.dir2);
    let h = Heures::actif();
    let mut d = w.hse(8, vec![w.seance(8, true)]);
    d.pour_le_compte_de = Some(prof.personne);
    let e = w.deposer_hse(&adm, d).await.expect("dépôt pour le compte");
    let (demandeur, deposant): (Uuid, Option<Uuid>) = sqlx::query_as(
        "SELECT demandeur_personne_id, deposant_personne_id FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.etab())
    .bind(e.id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((demandeur, deposant), (prof.personne, Some(adm.personne)));
    let (motif,): (Option<String>,) = sqlx::query_as(
        "SELECT motif_code FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'depose'",
    )
    .bind(w.etab())
    .bind(e.id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(motif.as_deref(), Some("pour_le_compte"));
    // Le déposant et le bénéficiaire la voient ; le bénéficiaire est informé qu'elle attend sa confirmation (R-DM43).
    let vue_dep = w.detail(&adm, e.id).await.unwrap();
    let vue_ben = w.detail(&prof, e.id).await.unwrap();
    assert!(vue_dep.deposee_pour_compte && vue_ben.deposee_pour_compte);
    assert!(vue_ben.confirmation_requise && !vue_ben.confirmee);
    assert_eq!(w.ent.mes_hse_for_actor(w.etab(), prof.user).await.unwrap().len(), 1);
    assert_eq!(w.ent.mes_hse_for_actor(w.etab(), adm.user).await.unwrap().len(), 0, "mes_hse : le bénéficiaire ou le demandeur, pas le déposant");
    // Avant la confirmation du collègue : pas de décision, pas dans la file à décider.
    let r = w.decider(&dir2, &h, e.id, DecisionHse::Accorder, OptionsDecision::default()).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))), "{r:?}");
    let file = w.ent.hse_a_decider_for_actor(w.etab(), dir2.user, dir2.role, &w.src(&dir2).await).await.unwrap();
    assert!(file.iter().all(|l| l.demande_id != e.id));
    // Seul le bénéficiaire confirme.
    let r = w.ent.confirmer_hse_for_actor(w.etab(), adm.user, adm.role, &w.src(&adm).await, e.id).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
    w.ent.confirmer_hse_for_actor(w.etab(), prof.user, prof.role, &w.src(&prof).await, e.id).await.unwrap();
    w.ent.confirmer_hse_for_actor(w.etab(), prof.user, prof.role, &w.src(&prof).await, e.id).await.unwrap(); // idempotent
    let file = w.ent.hse_a_decider_for_actor(w.etab(), dir2.user, dir2.role, &w.src(&dir2).await).await.unwrap();
    assert!(file.iter().any(|l| l.demande_id == e.id));
    assert!(w.decider(&dir2, &h, e.id, DecisionHse::Accorder, OptionsDecision::default()).await.is_ok());
    // CDM-28, DEM-137 : un collègue qui n'a rien demandé retire la demande déposée à son nom.
    let mut d2 = w.hse(8, vec![]);
    d2.pour_le_compte_de = Some(prof.personne);
    let e2 = w.deposer_hse(&adm, d2).await.unwrap();
    let s = w.src(&prof).await;
    let v = w.version(e2.id).await;
    let retiree = w.ent.annuler_hse_for_actor(w.etab(), prof.user, prof.role, &s, e2.id, v, None).await.unwrap();
    assert_eq!(retiree.statut, "annulee");
    // Seul un compte habilité dépose pour le compte d'autrui : le professeur n'en a pas le droit.
    let mut d3 = w.hse(8, vec![]);
    d3.pour_le_compte_de = Some(w.prof2.personne);
    assert_eq!(w.deposer_hse(&prof, d3).await.unwrap_err(), RefusDemande::Interdit);
    // L'accueil dépose pour un collègue qui n'est pas un agent : refus clair (personne sans service).
    let mut d4 = w.hse(8, vec![]);
    d4.pour_le_compte_de = Some(w.m.parent.personne);
    assert!(matches!(w.deposer_hse(&adm, d4).await, Err(RefusDemande::Invalide(_))));
}

#[tokio::test]
async fn t_nj91_attestation_facultative_desactivee_par_defaut() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, dir1, dir2, cpe) = (w.m.prof, w.m.direction, w.dir2, w.m.cpe);
    let h = Heures::actif();
    // D-061 : désactivée par défaut ; l'acceptation par la direction vaut confiance et transmet les heures faites.
    let rg = w.ent.reglage_hse_for_actor(w.etab(), dir1.user, dir1.role).await.unwrap();
    assert_eq!(rg, ReglageHse { attestation_service_fait: false, echantillon_pct: 10 });
    let e = w.deposer_hse(&prof, w.hse(12, vec![w.seance(8, true), w.seance(4, false)])).await.unwrap();
    let r = w.accorder(e.id, &h).await;
    assert_eq!((r.seances_transmises, r.transmission), (1, Some("port_15")), "seule la séance FAITE est transmise");
    assert_eq!(h.nb(), 1);
    // L'attestation n'existe pas tant que le réglage est désactivé.
    let (faite,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_dem_hse_seance WHERE etablissement_id = $1 AND demande_id = $2 AND etat = 'faite'")
        .bind(w.etab())
        .bind(e.id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    let r = w.ent.attester_hse_for_actor(w.etab(), cpe.user, "direction", &h, e.id, &[faite], "appel", false).await;
    assert!(r.is_err(), "pas d'attestation sans action attester");
    // Activation par la direction (journalisée), refusée au professeur.
    let r = w.ent.regler_hse_for_actor(w.etab(), prof.user, prof.role, true, 10).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
    w.ent.regler_hse_for_actor(w.etab(), dir1.user, dir1.role, true, 100).await.unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_reglage_journal WHERE etablissement_id = $1 AND sujet = 'hse'")
        .bind(w.etab())
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 1);
    // Réglage activé : la décision ne transmet plus rien ; les heures partent à l'attestation (R-DM50).
    let e2 = w.deposer_hse(&prof, w.hse(40, vec![w.seance(4, true), w.seance(4, true)])).await.unwrap();
    let r = w.accorder(e2.id, &h).await;
    assert_eq!(r.seances_transmises, 0);
    assert_eq!(w.transmises(e2.id).await.0, 0);
    let seances: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_dem_hse_seance WHERE etablissement_id = $1 AND demande_id = $2 AND etat = 'faite' ORDER BY id")
        .bind(w.etab())
        .bind(e2.id)
        .fetch_all(w.ent.pool_verif())
        .await
        .unwrap();
    let ids: Vec<Uuid> = seances.into_iter().map(|(i,)| i).collect();
    // Le bénéficiaire n'atteste pas ses propres heures ; un fondement valide est exigé.
    let r = w.ent.attester_hse_for_actor(w.etab(), prof.user, prof.role, &h, e2.id, &ids, "appel", false).await;
    assert!(r.is_err());
    let r = w.ent.attester_hse_for_actor(w.etab(), dir2.user, dir2.role, &h, e2.id, &ids, "parole", false).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
    // R-DM50 : déclaration sur l'honneur, échantillon à 100 % → les deux séances sont marquées à contrôler.
    let n = w.ent.attester_hse_for_actor(w.etab(), dir2.user, dir2.role, &h, e2.id, &ids[..1], "declaration_honneur", false).await.unwrap();
    assert_eq!(n, 1);
    let r = w.ent.attester_hse_for_actor(w.etab(), dir2.user, dir2.role, &h, e2.id, &ids[..1], "appel", false).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))), "une séance ne s'atteste qu'une fois");
    let d = w.detail(&prof, e2.id).await.unwrap();
    let s1 = d.seances.iter().find(|s| s.id == ids[0]).unwrap();
    assert_eq!((s1.etat.as_str(), s1.fondement.as_deref(), s1.echantillon, s1.transmission.as_str()), ("attestee", Some("declaration_honneur"), true, "port_15"));
    // Une contestation renvoie la séance à l'enseignant avec un motif.
    w.ent.contester_seance_for_actor(w.etab(), dir2.user, dir2.role, e2.id, ids[1], "Je n'ai pas de trace de cette séance").await.unwrap();
    let r = w.ent.contester_seance_for_actor(w.etab(), dir2.user, dir2.role, e2.id, ids[1], "").await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
    assert_eq!(w.transmises(e2.id).await.0, 1);
    // Attestation en masse sur simple déclaration : confirmation explicite (conf. 2).
    let mut d3 = w.hse(200, vec![]);
    d3.seances = (0..12).map(|_| w.seance(4, true)).collect();
    let e3 = w.deposer_hse(&prof, d3).await.unwrap();
    w.accorder(e3.id, &h).await;
    let lot: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_dem_hse_seance WHERE etablissement_id = $1 AND demande_id = $2 AND etat = 'faite' ORDER BY id")
        .bind(w.etab())
        .bind(e3.id)
        .fetch_all(w.ent.pool_verif())
        .await
        .unwrap();
    let lot: Vec<Uuid> = lot.into_iter().map(|(i,)| i).collect();
    let r = w.ent.attester_hse_for_actor(w.etab(), dir2.user, dir2.role, &h, e3.id, &lot, "declaration_honneur", false).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))));
    assert_eq!(w.ent.attester_hse_for_actor(w.etab(), dir2.user, dir2.role, &h, e3.id, &lot, "declaration_honneur", true).await.unwrap(), 12);
    // Le déterminisme de l'échantillon : mêmes entrées, même résultat ; 0 % rien, 100 % tout.
    let u = Uuid::new_v4();
    assert_eq!(dans_echantillon(u, 10), dans_echantillon(u, 10));
    assert!(dans_echantillon(u, 100) && !dans_echantillon(u, 0));
}

// ───────────────────────── HSE : allocation, transmission, export, enveloppe ─────────────────────────

#[tokio::test]
async fn t_hse_decision_partielle_et_refus() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, dir2) = (w.m.prof, w.dir2);
    let h = Heures::actif();
    // Partiel : 3 h demandées en trois séances de 1 h ; 2 h accordées → 4 + 4 + 0 quarts retenus, dans l'ordre.
    let e = w.deposer_hse(&prof, w.hse(12, vec![w.seance(4, true), w.seance(4, true), w.seance(4, true)])).await.unwrap();
    let r = w.decider(&dir2, &h, e.id, DecisionHse::AccorderEnPartie { volume_quarts: 8, motif: "budget".into() }, OptionsDecision::default()).await.unwrap();
    assert_eq!((r.decision.as_str(), r.volume_accorde_quarts, r.seances_transmises), ("accordee_partiellement", 8, 2));
    let d = w.detail(&prof, e.id).await.unwrap();
    let mut retenus: Vec<i32> = d.seances.iter().map(|s| s.retenu_quarts.unwrap()).collect();
    retenus.sort_unstable();
    assert_eq!(retenus, vec![0, 4, 4]);
    assert_eq!(d.statut, "close");
    assert_eq!(h.nb(), 2);
    // Refus : motif codé, rien n'est transmis, demande close « refusée ».
    let e2 = w.deposer_hse(&prof, w.hse(8, vec![w.seance(8, true)])).await.unwrap();
    let r = w.decider(&dir2, &h, e2.id, DecisionHse::Refuser { motif: "pas_besoin".into() }, OptionsDecision::default()).await.unwrap();
    assert_eq!((r.decision.as_str(), r.volume_accorde_quarts, r.seances_transmises), ("refusee", 0, 0));
    assert_eq!(h.nb(), 2);
    // L'enseignant ne déclare pas de séance sur une demande refusée.
    let r = w.declarer(&prof, &h, e2.id, w.seance(4, true)).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))));
    // Forfait : une demande sans séances détaillées est portée par une séance « forfait » à la fin de la période.
    let e3 = w.deposer_hse(&prof, w.hse(16, vec![])).await.unwrap();
    w.accorder(e3.id, &h).await;
    let d3 = w.detail(&prof, e3.id).await.unwrap();
    assert_eq!(d3.seances.len(), 1);
    assert!(d3.seances[0].forfait && d3.seances[0].retenu_quarts == Some(16));
}

#[tokio::test]
async fn t_hse_transmission_idempotente() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, dir2, dir1) = (w.m.prof, w.dir2, w.m.direction);
    // Tant que le socle 15 ne reçoit pas : l'export de secours garde l'heure (et le dit).
    let secours = Heures { actif: false, appels: Mutex::new(vec![]) };
    let e = w.deposer_hse(&prof, w.hse(12, vec![w.seance(8, true), w.seance(4, false)])).await.unwrap();
    let r = w.decider(&dir2, &secours, e.id, DecisionHse::Accorder, OptionsDecision::default()).await.unwrap();
    assert_eq!((r.seances_transmises, r.transmission), (1, Some("export_secours")));
    assert_eq!(w.transmises(e.id).await, (1, 0, 1));
    // La séance prévue, confirmée « faite », se transmet une seule fois ; confirmer deux fois ne recompte rien.
    let (prevue,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_dem_hse_seance WHERE etablissement_id = $1 AND demande_id = $2 AND etat = 'prevue'")
        .bind(w.etab())
        .bind(e.id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    let src = w.src(&prof).await;
    let ok = w.ent.confirmer_seance_faite_for_actor(w.etab(), prof.user, prof.role, &src, &secours, e.id, prevue).await.unwrap();
    assert!(ok.transmise);
    let encore = w.ent.confirmer_seance_faite_for_actor(w.etab(), prof.user, prof.role, &src, &secours, e.id, prevue).await;
    assert!(matches!(encore, Err(RefusDemande::Conflit(_))));
    assert_eq!(w.transmises(e.id).await, (2, 0, 2));
    // Seul le bénéficiaire confirme.
    let r = w.ent.confirmer_seance_faite_for_actor(w.etab(), dir2.user, dir2.role, &w.src(&dir2).await, &secours, e.id, prevue).await;
    assert!(matches!(r, Err(RefusDemande::Introuvable | RefusDemande::Interdit)));
    // La base interdit de défaire une transmission ou de changer une heure transmise (R-DM42).
    let r = sqlx::query("UPDATE ent_dem_hse_seance SET transmission = 'aucune', transmise_le = NULL WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(e.id)
        .execute(w.ent.pool_verif())
        .await;
    assert!(r.is_err());
    let r = sqlx::query("UPDATE ent_dem_hse_seance SET retenu_quarts = 1 WHERE etablissement_id = $1 AND demande_id = $2 AND transmission <> 'aucune'")
        .bind(w.etab())
        .bind(e.id)
        .execute(w.ent.pool_verif())
        .await;
    assert!(r.is_err());
    let r = sqlx::query("DELETE FROM ent_dem_hse_seance WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(e.id)
        .execute(w.ent.pool_verif())
        .await;
    assert!(r.is_err());
    // Annuler après transmission est refusé : les heures déjà transmises restent dues (DEM-275).
    let v = w.version(e.id).await;
    let r = w.ent.annuler_hse_for_actor(w.etab(), prof.user, prof.role, &src, e.id, v, None).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))));
    // Avec le socle 15 : une décision et trois séances → trois appels, jamais plus ; rejouer ne recompte rien.
    let port = Heures::actif();
    let e2 = w.deposer_hse(&prof, w.hse(12, vec![w.seance(4, true), w.seance(4, true), w.seance(4, true)])).await.unwrap();
    w.accorder(e2.id, &port).await;
    assert_eq!((port.nb(), w.transmises(e2.id).await), (3, (3, 3, 0)));
    // Rien n'est recomptable : une nouvelle déclaration au-delà du volume est refusée, et le port n'est pas rappelé.
    let r = w.declarer(&prof, &port, e2.id, w.seance(4, true)).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))));
    assert_eq!(port.nb(), 3);
    let _ = dir1;
}

#[tokio::test]
async fn t_hse_export_secours_csv() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, dir2, compta) = (w.m.prof, w.dir2, w.m.compta);
    let secours = Heures { actif: false, appels: Mutex::new(vec![]) };
    // Un nom qui ressemble à une formule de tableur ne s'exécute pas dans le CSV.
    sqlx::query("UPDATE ent_personne SET nom = '=CMD(1)', prenom = 'Zoé; \"le\" grand' WHERE etablissement_id = $1 AND id = $2")
        .bind(w.etab())
        .bind(prof.personne)
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    let e = w.deposer_hse(&prof, w.hse(25, vec![w.seance(25, true)])).await.unwrap();
    w.decider(&dir2, &secours, e.id, DecisionHse::Accorder, OptionsDecision::default()).await.unwrap();
    let mois = w.jour;
    // Export réservé à la direction ou à la comptabilité ; le professeur n'y a pas accès.
    let r = w.ent.exporter_hse_mensuel_for_actor(w.etab(), prof.user, prof.role, mois).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
    let x = w.ent.exporter_hse_mensuel_for_actor(w.etab(), compta.user, compta.role, mois).await.unwrap();
    assert_eq!((x.nb_seances, x.total_quarts), (1, 25));
    let lignes: Vec<&str> = x.csv.lines().collect();
    assert_eq!(lignes[0], "nom;prenom;matricule;type_heure;heures;quarts_heure;seances");
    assert!(lignes.iter().any(|l| l.starts_with("'=CMD(1);\"Zoé; \"\"le\"\" grand\";;hse_enseignement;6.25;25;1")), "{:?}", lignes);
    assert!(!x.csv.contains('€') && !x.csv.to_lowercase().contains("euro"), "en heures, jamais en euros");
    // Tracé : un lot par export (ajout seul) ; les mêmes heures reviennent tant que le socle 15 n'est pas prêt.
    let y = w.ent.exporter_hse_mensuel_for_actor(w.etab(), compta.user, compta.role, mois).await.unwrap();
    assert_eq!((y.total_quarts, y.csv.clone()), (25, x.csv.clone()));
    assert_ne!(x.lot_id, y.lot_id);
    let (lots,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_hse_export WHERE etablissement_id = $1")
        .bind(w.etab())
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(lots, 2);
    let r = sqlx::query("DELETE FROM ent_dem_hse_export WHERE etablissement_id = $1").bind(w.etab()).execute(w.ent.pool_verif()).await;
    assert!(r.is_err(), "un lot d'export ne s'efface pas");
    // Un autre mois ne contient rien.
    let autre = w.ent.exporter_hse_mensuel_for_actor(w.etab(), compta.user, compta.role, mois - chrono::Duration::days(70)).await.unwrap();
    assert_eq!(autre.total_quarts, 0);
}

#[tokio::test]
async fn t_hse_enveloppe_en_heures() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, dir1, dir2, compta) = (w.m.prof, w.m.direction, w.dir2, w.m.compta);
    let h = Heures::actif();
    // Aucune enveloppe saisie : aucune alerte, la décision passe (l'établissement n'est pas obligé d'en tenir une).
    let e0 = w.deposer_hse(&prof, w.hse(400, vec![])).await.unwrap();
    let env = w.ent.enveloppe_hse_for_actor(w.etab(), dir2.user, dir2.role, None).await.unwrap();
    assert!(!env.saisie);
    assert!(!w.decider(&dir2, &h, e0.id, DecisionHse::Accorder, OptionsDecision::default()).await.unwrap().depassement_enveloppe);
    // CDM-67 : enveloppe prévisionnelle avant la notification, remplacée par la dotation ensuite (DEM-376).
    w.ent.mouvement_enveloppe_hse_for_actor(w.etab(), compta.user, compta.role, None, "prevision", 2000, Some("avant notification")).await.unwrap();
    let env = w.ent.enveloppe_hse_for_actor(w.etab(), compta.user, compta.role, None).await.unwrap();
    assert_eq!((env.prevision_quarts, env.dotation_quarts, env.total_quarts), (2000, 0, 2000));
    w.ent.mouvement_enveloppe_hse_for_actor(w.etab(), compta.user, compta.role, None, "dotation", 600, None).await.unwrap();
    let env = w.ent.enveloppe_hse_for_actor(w.etab(), compta.user, compta.role, None).await.unwrap();
    assert_eq!(env.total_quarts, 600, "la dotation remplace la prévision");
    assert_eq!(env.engage_quarts, 400, "l'accordé d'avant la saisie compte");
    assert_eq!(env.restant_quarts, 200);
    // CDM-45 : au-delà de l'enveloppe, avertissement : la décision exige confirmation ET motif.
    let e1 = w.deposer_hse(&prof, w.hse(300, vec![])).await.unwrap();
    let r = w.decider(&dir2, &h, e1.id, DecisionHse::Accorder, OptionsDecision::default()).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))), "{r:?}");
    let r = w.decider(&dir2, &h, e1.id, DecisionHse::Accorder, OptionsDecision { confirmer_depassement: true, motif_depassement: None }).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))), "le motif est obligatoire");
    let ok = w
        .decider(&dir2, &h, e1.id, DecisionHse::Accorder, OptionsDecision { confirmer_depassement: true, motif_depassement: Some("Brevet blanc".into()) })
        .await
        .unwrap();
    assert!(ok.depassement_enveloppe);
    let (dep, motif): (bool, Option<String>) = sqlx::query_as(
        "SELECT depassement_enveloppe, motif_code FROM ent_dem_hse_decision WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(w.etab())
    .bind(e1.id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert!(dep && motif.as_deref() == Some("depassement_enveloppe"));
    let env = w.ent.enveloppe_hse_for_actor(w.etab(), dir1.user, dir1.role, None).await.unwrap();
    assert!(env.restant_quarts < 0);
    assert_eq!(env.par_motif, vec![("soutien".to_string(), 700)]);
    // Abondement et retrait : mouvements en ajout seul ; jamais modifiés ni effacés.
    w.ent.mouvement_enveloppe_hse_for_actor(w.etab(), compta.user, compta.role, None, "abondement", 200, None).await.unwrap();
    w.ent.mouvement_enveloppe_hse_for_actor(w.etab(), compta.user, compta.role, None, "retrait", 50, Some("reprise")).await.unwrap();
    let env = w.ent.enveloppe_hse_for_actor(w.etab(), compta.user, compta.role, None).await.unwrap();
    assert_eq!((env.abondement_quarts, env.retrait_quarts, env.total_quarts), (200, 50, 750));
    let r = sqlx::query("UPDATE ent_dem_hse_enveloppe_mvt SET quarts = 1 WHERE etablissement_id = $1").bind(w.etab()).execute(w.ent.pool_verif()).await;
    assert!(r.is_err());
    let r = sqlx::query("DELETE FROM ent_dem_hse_enveloppe_mvt WHERE etablissement_id = $1").bind(w.etab()).execute(w.ent.pool_verif()).await;
    assert!(r.is_err());
    // Droits : le professeur ne gère ni ne lit l'enveloppe ; la direction décide mais ne la modifie pas sans le droit.
    for r in [
        w.ent.mouvement_enveloppe_hse_for_actor(w.etab(), prof.user, prof.role, None, "dotation", 8, None).await,
        w.ent.mouvement_enveloppe_hse_for_actor(w.etab(), dir2.user, dir2.role, None, "bidon", 8, None).await,
        w.ent.mouvement_enveloppe_hse_for_actor(w.etab(), compta.user, compta.role, None, "dotation", 0, None).await,
    ] {
        assert!(r.is_err());
    }
    assert!(w.ent.enveloppe_hse_for_actor(w.etab(), prof.user, prof.role, None).await.is_err());
    // DEM-276 : le non réalisé d'une période close est rendu à l'enveloppe (accordé 300, rien de fait, période passée).
    sqlx::query("UPDATE ent_dem_hse SET periode_du = $3, periode_au = $4 WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(e1.id)
        .bind(w.jour - chrono::Duration::days(10))
        .bind(w.jour - chrono::Duration::days(1))
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    let env = w.ent.enveloppe_hse_for_actor(w.etab(), compta.user, compta.role, None).await.unwrap();
    let forfait_fait: i64 = sqlx::query_as::<_, (i64,)>(
        "SELECT COALESCE(sum(retenu_quarts), 0)::bigint FROM ent_dem_hse_seance WHERE etablissement_id = $1 AND demande_id = $2 AND etat IN ('faite', 'attestee')",
    )
    .bind(w.etab())
    .bind(e1.id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap()
    .0;
    assert_eq!(env.engage_quarts, 400 + forfait_fait, "le non réalisé de la demande close n'est plus engagé");
}

#[tokio::test]
async fn t_hse_declaration_et_avenant() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, dir2) = (w.m.prof, w.dir2);
    let h = Heures::actif();
    let e = w.deposer_hse(&prof, w.hse(8, vec![w.seance(4, true)])).await.unwrap();
    // Avant la décision : pas de déclaration (la demande n'est pas accordée).
    let r = w.declarer(&prof, &h, e.id, w.seance(4, true)).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))));
    w.accorder(e.id, &h).await;
    // CDM-43 : séance pendant une absence déclarée → incohérence signalée (avertissement), jamais un blocage.
    sqlx::query(
        "INSERT INTO ent_absp_absence (id, etablissement_id, agent_id, cause, etat, debut, fin, saisie_par_user_id, saisie_canal)
         VALUES (gen_random_uuid(), $1, $2, 'rh', 'validee', now() - interval '2 days', now() + interval '2 days', $3, 'agent')",
    )
    .bind(w.etab())
    .bind(prof.agent.unwrap())
    .bind(prof.user)
    .execute(w.ent.pool_verif())
    .await
    .unwrap();
    let r = w.declarer(&prof, &h, e.id, w.seance(4, true)).await.unwrap();
    assert_eq!(r.avertissements.len(), 1);
    assert!(r.transmise);
    assert!(w.faits(e.id).await.contains(&"hse_incoherence_absence".to_string()));
    // Volume accordé atteint : demander un avenant (DEM-274). Un collègue ne déclare rien sur cette demande.
    let r = w.declarer(&prof, &h, e.id, w.seance(4, true)).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))));
    let r = w.declarer(&w.prof2.clone(), &h, e.id, w.seance(1, true)).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
    let s = w.src(&prof).await;
    let v = w.version(e.id).await;
    // Un avenant demande plus que l'accordé, et reste dans l'année scolaire.
    let r = w.ent.demander_avenant_hse_for_actor(w.etab(), prof.user, prof.role, &s, e.id, v, 8, None).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
    let r = w.ent.demander_avenant_hse_for_actor(w.etab(), prof.user, prof.role, &s, e.id, v, 16, Some(w.fin_annee + chrono::Duration::days(1))).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))), "CDM-46 : jamais au-delà de l'année d'origine");
    let rouv = w.ent.demander_avenant_hse_for_actor(w.etab(), prof.user, prof.role, &s, e.id, v, 16, None).await.unwrap();
    assert_eq!(rouv.statut, "rouverte");
    let r = w.ent.demander_avenant_hse_for_actor(w.etab(), prof.user, prof.role, &s, e.id, rouv.version, 20, None).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))), "un seul avenant en attente");
    // Nouvelle décision, même demande ; la précédente reste au registre ; seules les heures nouvelles se transmettent.
    let before = h.nb();
    let r = w.accorder(e.id, &h).await;
    assert_eq!(r.volume_accorde_quarts, 16);
    assert_eq!(h.nb(), before, "aucune heure déjà transmise n'est recomptée");
    let (decisions, avenants): (i64, i16) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_dem_hse_decision d WHERE d.etablissement_id = h.etablissement_id AND d.demande_id = h.demande_id), avenants
           FROM ent_dem_hse h WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(w.etab())
    .bind(e.id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((decisions, avenants), (2, 1));
    let r = w.declarer(&prof, &h, e.id, w.seance(4, true)).await.unwrap();
    assert!(r.transmise);
    // Hors période et séance « faite » à venir : refusées. L'enseignant déclare au plus 12 h par séance.
    let hors = SeanceSaisie { jour: w.fin_annee + chrono::Duration::days(3), debut: None, duree_quarts: 4, fait: false, lieu: None };
    assert!(w.declarer(&prof, &h, e.id, hors).await.is_err());
    assert!(matches!(w.declarer(&prof, &h, e.id, w.seance(49, true)).await, Err(RefusDemande::Invalide(_))));
    // Annulée avant toute transmission : l'ordre est refusé s'il y a déjà des heures ; ici oui.
    let v = w.version(e.id).await;
    assert!(matches!(w.ent.annuler_hse_for_actor(w.etab(), prof.user, prof.role, &s, e.id, v, None).await, Err(RefusDemande::Conflit(_))));
}

#[tokio::test]
async fn t_hse_alerte_fin_periode() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, dir2) = (w.m.prof, w.dir2);
    let h = Heures::actif();
    // CDM-42 : 6 h accordées, rien déclaré, période passée → un fait et un événement, une seule fois.
    let e = w.deposer_hse(&prof, w.hse(24, vec![w.seance(4, false)])).await.unwrap();
    w.decider(&dir2, &h, e.id, DecisionHse::Accorder, OptionsDecision::default()).await.unwrap();
    assert_eq!(w.ent.alerter_hse_non_declarees(w.etab()).await.unwrap().signalees, 0, "période en cours : rien");
    sqlx::query("UPDATE ent_dem_hse SET periode_du = $3, periode_au = $4 WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(e.id)
        .bind(w.jour - chrono::Duration::days(10))
        .bind(w.jour - chrono::Duration::days(2))
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(w.ent.alerter_hse_non_declarees(w.etab()).await.unwrap().signalees, 1);
    assert_eq!(w.ent.alerter_hse_non_declarees(w.etab()).await.unwrap().signalees, 0, "rejouable sans doublon");
    assert_eq!(w.faits(e.id).await.iter().filter(|f| *f == "hse_rien_declare").count(), 1);
    // Une demande avec une séance faite n'alerte pas.
    let e2 = w.deposer_hse(&prof, w.hse(8, vec![w.seance(8, true)])).await.unwrap();
    w.accorder(e2.id, &h).await;
    sqlx::query("UPDATE ent_dem_hse SET periode_du = $3, periode_au = $4 WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(e2.id)
        .bind(w.jour - chrono::Duration::days(10))
        .bind(w.jour - chrono::Duration::days(2))
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(w.ent.alerter_hse_non_declarees(w.etab()).await.unwrap().signalees, 0);
}

#[tokio::test]
async fn t_hse_confidentialite() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, prof2, dir1, dir2, cpe, adm) = (w.m.prof, w.prof2, w.m.direction, w.dir2, w.m.cpe, w.adm);
    let e = w.deposer_hse(&prof, w.hse(8, vec![w.seance(8, true)])).await.unwrap();
    // Collègues : rien (§17.8) — ni le détail, ni « mes HSE », ni la répartition, ni la file à décider.
    for qui in [prof2, cpe, adm] {
        assert_eq!(w.detail(&qui, e.id).await.unwrap_err(), RefusDemande::Introuvable);
        assert!(w.ent.mes_hse_for_actor(w.etab(), qui.user).await.unwrap().is_empty());
        assert!(w.ent.repartition_hse_for_actor(w.etab(), qui.user, qui.role).await.is_err());
        assert!(w.ent.hse_a_decider_for_actor(w.etab(), qui.user, qui.role, &w.src(&qui).await).await.is_err());
        let r = w.ent.annuler_hse_for_actor(w.etab(), qui.user, qui.role, &w.src(&qui).await, e.id, 1, None).await;
        assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
    }
    // Un décideur qui n'est pas membre de la file ne voit pas non plus la demande d'une autre nature (404 neutre).
    let g = {
        let mut d = depot("secretariat.demande_interne");
        d.objet = "Autre chose".into();
        d
    };
    let _ = g;
    // La direction voit la file, le détail, la répartition ; jamais sa propre demande dans la file à décider.
    let h = Heures::actif();
    let file = w.ent.hse_a_decider_for_actor(w.etab(), dir2.user, dir2.role, &w.src(&dir2).await).await.unwrap();
    assert_eq!(file.len(), 1);
    assert_eq!(w.detail(&dir2, e.id).await.unwrap().volume_demande_quarts, 8);
    let e_dir = w.deposer_hse(&dir1, w.hse(4, vec![])).await.unwrap();
    let f1 = w.ent.hse_a_decider_for_actor(w.etab(), dir1.user, dir1.role, &w.src(&dir1).await).await.unwrap();
    assert!(f1.iter().all(|l| l.demande_id != e_dir.id), "R-DM40 : sa propre demande n'est pas à décider");
    // DEM-280, DEM-377 : la répartition par agent n'est visible que des décideurs, avec les volumes (heures).
    w.accorder(e.id, &h).await;
    let rep = w.ent.repartition_hse_for_actor(w.etab(), dir2.user, dir2.role).await.unwrap();
    let ligne = rep.iter().find(|r| Some(r.agent_id) == prof.agent).expect("le professeur figure à la répartition");
    assert_eq!((ligne.demandes, ligne.accorde_quarts, ligne.fait_quarts), (1, 8, 8));
    // Une demande d'une autre nature n'est pas une HSE : le détail HSE ne l'ouvre pas.
    let mais = w.detail(&prof, Uuid::new_v4()).await;
    assert_eq!(mais.unwrap_err(), RefusDemande::Introuvable);
    // Séparation entre établissements : un autre établissement ne voit rien, quel que soit l'établissement passé.
    let (ent2, cast2) = labo().await.unwrap();
    let m2 = monde(&ent2, &cast2).await;
    for etab in [m2.etab, w.etab()] {
        let q = m2.direction;
        let r = ent2.detail_hse_for_actor(etab, q.user, q.role, &SourcesAbsentes, e.id).await;
        assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
        let r = ent2.decider_hse_for_actor(etab, q.user, q.role, &SourcesAbsentes, &h, e.id, 1, DecisionHse::Accorder, OptionsDecision::default()).await;
        assert!(matches!(r, Err(RefusDemande::Introuvable | RefusDemande::Interdit)));
    }
}

// ───────────────────────── Reprographie ─────────────────────────

#[tokio::test]
async fn t_nj78_document_du_deposant() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, prof2) = (w.m.prof, w.prof2);
    let mut docs = Docs::default();
    let doc_prof = docs.doc(&prof, 12);
    let doc_autre = docs.doc(&prof2, 8);
    // NJ-78 : le PDF d'un AUTRE ne se rattache pas à ma demande (réponse neutre, aucune demande créée).
    let r = w.deposer_copies(&prof, &docs, w.copies(vec![doc_prof, doc_autre], w.site1)).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))), "{r:?}");
    let r = w.deposer_copies(&prof, &docs, w.copies(vec![Uuid::new_v4()], w.site1)).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))), "un document inconnu non plus");
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_repro WHERE etablissement_id = $1").bind(w.etab()).fetch_one(w.ent.pool_verif()).await.unwrap();
    assert_eq!(n, 0);
    // Déposer « pour le compte » ne contourne pas la règle : le fichier doit être celui du DÉPOSANT, pas du bénéficiaire.
    let adm = w.adm;
    let mut d = w.copies(vec![doc_prof], w.site1);
    d.pour_le_compte_de = Some(prof.personne);
    let r = w.deposer_copies(&adm, &docs, d).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))), "le fichier du bénéficiaire n'est pas celui du déposant");
    let doc_adm = docs.doc(&adm, 5);
    let mut d = w.copies(vec![doc_adm], w.site1);
    d.pour_le_compte_de = Some(prof.personne);
    let ok = w.deposer_copies(&adm, &docs, d).await.expect("pour le compte, avec son propre fichier");
    let (par,): (Uuid,) = sqlx::query_as("SELECT depose_par_personne_id FROM ent_dem_repro_document WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(ok.etat.id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(par, adm.personne, "le déposant est tracé sur le fichier");
    // Pages illisibles, trop ou pas de fichiers, format inconnu : refus.
    let mut sans_pages = Docs::default();
    sans_pages.propres.push((doc_prof, prof.personne));
    assert!(matches!(w.deposer_copies(&prof, &sans_pages, w.copies(vec![doc_prof], w.site1)).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(w.deposer_copies(&prof, &docs, w.copies(vec![], w.site1)).await, Err(RefusDemande::Invalide(_))));
    let mut f = w.copies(vec![doc_prof], w.site1);
    f.format = "a0".into();
    assert!(matches!(w.deposer_copies(&prof, &docs, f).await, Err(RefusDemande::Invalide(_))));
    let mut f = w.copies(vec![doc_prof, doc_prof], w.site1);
    f.exemplaires = Some(10);
    assert!(matches!(w.deposer_copies(&prof, &docs, f).await, Err(RefusDemande::Invalide(_))), "un fichier joint deux fois");
    // Le fichier d'une demande ne s'ouvre ni par un collègue ni avec le document d'une autre demande.
    let id = w.copies_simples(&mut docs, w.site1).await.etat.id;
    let r = w.ent.ouvrir_pdf_for_actor(w.etab(), prof2.user, prof2.role, &docs, id, doc_prof).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
    let r = w.ent.ouvrir_pdf_for_actor(w.etab(), prof.user, prof.role, &docs, id, doc_autre).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Introuvable, "un document d'une autre demande n'est pas rattaché à celle-ci");
}

#[tokio::test]
async fn t_nj77_file_par_site() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, adm, dir1, cpe) = (w.m.prof, w.adm, w.m.direction, w.m.cpe);
    let mut docs = Docs::default();
    // Deux réceptionnaires : adm (site 1) ; cpe (site 2). CDM-51 : un troisième couvre les deux sites.
    w.designer(w.site1, &adm).await;
    w.designer(w.site2, &cpe).await;
    let d1 = w.copies_simples(&mut docs, w.site1).await.etat.id;
    let d2 = w.copies_simples(&mut docs, w.site2).await.etat.id;
    let f1 = w.ent.file_impression_for_actor(w.etab(), adm.user, adm.role).await.unwrap();
    let f2 = w.ent.file_impression_for_actor(w.etab(), cpe.user, cpe.role).await.unwrap();
    assert_eq!(f1.iter().map(|l| l.demande_id).collect::<Vec<_>>(), vec![d1], "NJ-77 : le site 1 ne voit pas le site 2");
    assert_eq!(f2.iter().map(|l| l.demande_id).collect::<Vec<_>>(), vec![d2]);
    // Sans désignation : une file vide, jamais « tout un rôle » (la direction n'a aucun site désigné ici).
    assert!(w.ent.file_impression_for_actor(w.etab(), dir1.user, dir1.role).await.unwrap().is_empty());
    assert!(w.ent.file_impression_for_actor(w.etab(), prof.user, prof.role).await.unwrap().is_empty());
    // Le réceptionnaire du site 1 n'agit pas sur la demande du site 2 (404 neutre) : ni imprimer, ni « prête », ni PDF.
    assert_eq!(w.ent.mettre_en_impression_for_actor(w.etab(), adm.user, adm.role, d2).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(w.ent.marquer_prete_for_actor(w.etab(), adm.user, adm.role, d2, None).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(w.ent.signaler_probleme_repro_for_actor(w.etab(), adm.user, adm.role, d2, "pdf_illisible").await.unwrap_err(), RefusDemande::Introuvable);
    let (doc2,): (Uuid,) = sqlx::query_as("SELECT document_id FROM ent_dem_repro_document WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(d2)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(w.ent.ouvrir_pdf_for_actor(w.etab(), adm.user, adm.role, &docs, d2, doc2).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(w.ent.detail_repro_for_actor(w.etab(), adm.user, adm.role, d2).await.unwrap_err(), RefusDemande::Introuvable);
    // CDM-51 : un réceptionnaire qui couvre deux sites voit les deux files, séparées.
    w.designer(w.site2, &adm).await;
    let f1 = w.ent.file_impression_for_actor(w.etab(), adm.user, adm.role).await.unwrap();
    assert_eq!(f1.len(), 2);
    assert!(f1.iter().any(|l| l.site_id == w.site1) && f1.iter().any(|l| l.site_id == w.site2));
    // Fin de désignation : la file se referme ; la désignation est tracée.
    w.ent.designer_receptionnaire_for_actor(w.etab(), dir1.user, dir1.role, w.site2, adm.agent.unwrap(), false).await.unwrap();
    assert_eq!(w.ent.file_impression_for_actor(w.etab(), adm.user, adm.role).await.unwrap().len(), 1);
    assert!(w.ent.designer_receptionnaire_for_actor(w.etab(), prof.user, prof.role, w.site2, prof.agent.unwrap(), true).await.is_err());
    // Un compte famille n'a pas de file d'impression.
    let parent = w.m.parent;
    assert!(w.ent.file_impression_for_actor(w.etab(), parent.user, parent.role).await.is_err());
    // Séparation entre établissements.
    let (ent2, cast2) = labo().await.unwrap();
    let m2 = monde(&ent2, &cast2).await;
    for etab in [m2.etab, w.etab()] {
        let r = ent2.detail_repro_for_actor(etab, m2.direction.user, m2.direction.role, d1).await;
        assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
        assert!(ent2.file_impression_for_actor(etab, m2.prof.user, m2.prof.role).await.map_or(true, |l| l.is_empty()));
    }
}

#[tokio::test]
async fn t_nj92_pret_par_geste_authentifie() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, adm, prof2, dir1) = (w.m.prof, w.adm, w.prof2, w.m.direction);
    let mut docs = Docs::default();
    // NJ-92 : aucune colonne de jeton ni d'URL ; « prête » n'existe que comme geste de session (le code n'a aucune
    // fonction sans utilisateur : voir la signature `marquer_prete_for_actor(etab, user_id, role, …)`).
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM information_schema.columns WHERE table_schema = current_schema()
            AND table_name LIKE 'ent\\_dem\\_repro%' AND (column_name ~* '(token|jeton|hash|secret|url|lien)')",
    )
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 0);
    w.designer(w.site1, &adm).await;
    let id = w.copies_simples(&mut docs, w.site1).await.etat.id;
    // Ni le demandeur, ni un collègue, ni la direction (sans désignation sur ce site) ne marquent « prête ».
    assert_eq!(w.ent.marquer_prete_for_actor(w.etab(), prof.user, prof.role, id, None).await.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(w.ent.marquer_prete_for_actor(w.etab(), prof2.user, prof2.role, id, None).await.unwrap_err(), RefusDemande::Introuvable);
    assert!(w.ent.marquer_prete_for_actor(w.etab(), dir1.user, dir1.role, id, None).await.is_err());
    // Un utilisateur inexistant (pas de session) n'a aucun effet.
    assert_eq!(w.ent.marquer_prete_for_actor(w.etab(), Uuid::new_v4(), "administratif", id, None).await.unwrap_err(), RefusDemande::Introuvable);
    // Le lieu doit être un lieu de retrait ACTIF du site de la demande.
    let r = w.ent.marquer_prete_for_actor(w.etab(), adm.user, adm.role, id, Some(w.lieu2)).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
    assert_eq!(w.ent.detail_repro_for_actor(w.etab(), prof.user, prof.role, id).await.unwrap().etat, "a_imprimer");
    // Le réceptionnaire désigné, authentifié, avec le lieu de retrait : « prête », nominatif et daté.
    let e = w.ent.marquer_prete_for_actor(w.etab(), adm.user, adm.role, id, Some(w.lieu1)).await.unwrap();
    assert_eq!(e.statut, "traitee");
    let (par, etat): (Uuid, String) = sqlx::query_as(
        "SELECT prete_par_personne_id, etat FROM ent_dem_repro WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(w.etab())
    .bind(id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((par, etat.as_str()), (adm.personne, "prete"));
    // Rejouer le même geste ne change rien (aucun effet de bord d'un lien rouvert).
    assert!(matches!(w.ent.marquer_prete_for_actor(w.etab(), adm.user, adm.role, id, None).await, Err(RefusDemande::Conflit(_))));
    assert!(w.faits(id).await.contains(&"repro_prete".to_string()));
    // La base elle-même exige une personne et une date pour un état « prête ».
    let r = sqlx::query("UPDATE ent_dem_repro SET prete_par_personne_id = NULL WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(id)
        .execute(w.ent.pool_verif())
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn t_repro_circuit_et_validation() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, adm, dir1, dir2) = (w.m.prof, w.adm, w.m.direction, w.dir2);
    let mut docs = Docs::default();
    w.designer(w.site1, &adm).await;
    // Noir et blanc sous le seuil (500 copies) : directement dans la file d'impression.
    let ok = w.copies_simples(&mut docs, w.site1).await;
    assert_eq!((ok.etat_repro.as_str(), ok.validation_requise, ok.copies_total), ("a_imprimer", false, 300));
    // Couleur : validation de la direction (DRM-07). Rien n'entre dans la file avant la décision.
    let doc = docs.doc(&prof, 4);
    let mut c = w.copies(vec![doc], w.site1);
    c.couleur = true;
    let couleur = w.deposer_copies(&prof, &docs, c).await.unwrap();
    assert_eq!((couleur.etat_repro.as_str(), couleur.validation_requise), ("a_valider", true));
    assert!(!w.ent.file_impression_for_actor(w.etab(), adm.user, adm.role).await.unwrap().iter().any(|l| l.demande_id == couleur.etat.id));
    assert!(matches!(w.ent.mettre_en_impression_for_actor(w.etab(), adm.user, adm.role, couleur.etat.id).await, Err(RefusDemande::Conflit(_))));
    // Noir et blanc au-delà du seuil : validation aussi (« noir et blanc » ne contourne pas la règle). Seuil réglé à 100.
    let r = w.ent.regler_site_repro_for_actor(w.etab(), prof.user, prof.role, w.site1, 100, 1).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
    assert!(matches!(w.ent.regler_site_repro_for_actor(w.etab(), dir1.user, dir1.role, w.site1, 0, 1).await, Err(RefusDemande::Invalide(_))));
    w.ent.regler_site_repro_for_actor(w.etab(), dir1.user, dir1.role, w.site1, 100, 1).await.unwrap();
    let gros = w.copies_simples(&mut docs, w.site1).await;
    assert_eq!((gros.etat_repro.as_str(), gros.validation_requise), ("a_valider", true));
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_reglage_journal WHERE etablissement_id = $1 AND sujet = 'reprographie' AND cible LIKE 'site %'")
        .bind(w.etab())
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 1, "le réglage du site est journalisé");
    // La file à valider : le validateur ne voit jamais ses propres demandes ; le professeur n'y a pas accès.
    let a_valider = w.ent.repro_a_valider_for_actor(w.etab(), dir2.user, dir2.role).await.unwrap();
    assert_eq!(a_valider.len(), 2);
    assert!(w.ent.repro_a_valider_for_actor(w.etab(), prof.user, prof.role).await.is_err());
    // R-DM40 : la direction qui a déposé ne valide pas sa propre demande ; une autre direction le fait, une fois.
    let doc_d = docs.doc(&dir1, 3);
    let mut propre = w.copies(vec![doc_d], w.site1);
    propre.couleur = true;
    let dep_dir = w.deposer_copies(&dir1, &docs, propre).await.unwrap();
    assert_eq!(w.ent.valider_repro_for_actor(w.etab(), dir1.user, dir1.role, dep_dir.etat.id, true, None).await.unwrap_err(), RefusDemande::Interdit);
    assert!(w.ent.repro_a_valider_for_actor(w.etab(), dir1.user, dir1.role).await.unwrap().iter().all(|l| l.demande_id != dep_dir.etat.id));
    assert_eq!(w.ent.valider_repro_for_actor(w.etab(), prof.user, prof.role, couleur.etat.id, true, None).await.unwrap_err(), RefusDemande::Interdit);
    assert!(matches!(w.ent.valider_repro_for_actor(w.etab(), dir2.user, dir2.role, couleur.etat.id, false, None).await, Err(RefusDemande::Invalide(_))), "un refus s'explique");
    w.ent.valider_repro_for_actor(w.etab(), dir2.user, dir2.role, couleur.etat.id, true, Some("Pour le brevet")).await.unwrap();
    assert!(matches!(w.ent.valider_repro_for_actor(w.etab(), dir2.user, dir2.role, couleur.etat.id, true, None).await, Err(RefusDemande::Conflit(_))), "une seule décision");
    assert!(w.ent.file_impression_for_actor(w.etab(), adm.user, adm.role).await.unwrap().iter().any(|l| l.demande_id == couleur.etat.id));
    // Refus : la demande se clôt « refusée ».
    let r = w.ent.valider_repro_for_actor(w.etab(), dir2.user, dir2.role, gros.etat.id, false, Some("Trop de copies : utilisez l'ENT")).await.unwrap();
    assert_eq!(r.statut, "close");
    let (statut, issue): (String, Option<String>) = sqlx::query_as("SELECT statut, issue FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
        .bind(w.etab())
        .bind(gros.etat.id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!((statut.as_str(), issue.as_deref()), ("close", Some("refusee")));
    // La base garde DRM-07 : une demande soumise à validation n'entre pas en file sans décision datée.
    let r = sqlx::query("UPDATE ent_dem_repro SET etat = 'a_imprimer' WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(dep_dir.etat.id)
        .execute(w.ent.pool_verif())
        .await;
    assert!(r.is_err());
    let r = sqlx::query("UPDATE ent_dem_repro SET couleur = false WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(dep_dir.etat.id)
        .execute(w.ent.pool_verif())
        .await;
    assert!(r.is_err(), "la couleur ne se change pas après dépôt");
}

#[tokio::test]
async fn t_repro_cycle_complet() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, adm, prof2) = (w.m.prof, w.adm, w.prof2);
    let mut docs = Docs::default();
    w.designer(w.site1, &adm).await;
    let id = w.copies_simples(&mut docs, w.site1).await.etat.id;
    // Le réceptionnaire ouvre le PDF (jeton court, journalisé), puis passe « en impression » : plus annulable.
    let (doc,): (Uuid,) = sqlx::query_as("SELECT document_id FROM ent_dem_repro_document WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    let jeton = w.ent.ouvrir_pdf_for_actor(w.etab(), adm.user, adm.role, &docs, id, doc).await.unwrap();
    assert!(jeton.starts_with("jeton-court-"));
    assert!(w.faits(id).await.contains(&"repro_pdf_ouvert".to_string()));
    let e = w.ent.mettre_en_impression_for_actor(w.etab(), adm.user, adm.role, id).await.unwrap();
    assert_eq!(e.statut, "prise");
    assert!(matches!(w.ent.annuler_repro_for_actor(w.etab(), prof.user, prof.role, id).await, Err(RefusDemande::Conflit(_))));
    assert_eq!(w.ent.annuler_repro_for_actor(w.etab(), prof2.user, prof2.role, id).await.unwrap_err(), RefusDemande::Introuvable);
    // « Prête » avec le lieu : le demandeur voit « prête, pas encore vue » jusqu'à ce qu'il ouvre sa demande.
    w.ent.marquer_prete_for_actor(w.etab(), adm.user, adm.role, id, Some(w.lieu1)).await.unwrap();
    let mes = w.ent.mes_repro_for_actor(w.etab(), prof.user).await.unwrap();
    assert_eq!((mes[0].etat.as_str(), mes[0].retrait_lieu.as_deref()), ("prete", Some("Salle des professeurs")));
    let avant = sqlx::query_as::<_, (Option<chrono::DateTime<chrono::Utc>>,)>("SELECT prete_vue_le FROM ent_dem_repro WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert!(avant.0.is_none());
    let vu = w.ent.detail_repro_for_actor(w.etab(), prof.user, prof.role, id).await.unwrap();
    assert!(vu.prete_non_vue, "la première ouverture voit encore le signal");
    let revu = w.ent.detail_repro_for_actor(w.etab(), prof.user, prof.role, id).await.unwrap();
    assert!(!revu.prete_non_vue, "puis le signal disparaît");
    // Retirée par le demandeur : la demande se clôt, avec sa conservation.
    let c = w.ent.marquer_retiree_for_actor(w.etab(), prof.user, prof.role, id).await.unwrap();
    assert_eq!(c.statut, "close");
    let (cons,): (Option<NaiveDate>,) = sqlx::query_as("SELECT conservation_jusqu_au FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
        .bind(w.etab())
        .bind(id)
        .fetch_one(w.ent.pool_verif())
        .await
        .unwrap();
    assert!(cons.is_some());
    assert!(matches!(w.ent.marquer_retiree_for_actor(w.etab(), prof.user, prof.role, id).await, Err(RefusDemande::Conflit(_))));
    // Annulation avant impression (DEM-303) par le demandeur ; la file n'affiche plus la demande.
    let id2 = w.copies_simples(&mut docs, w.site1).await.etat.id;
    assert_eq!(w.ent.annuler_repro_for_actor(w.etab(), prof.user, prof.role, id2).await.unwrap().statut, "annulee");
    assert!(w.ent.file_impression_for_actor(w.etab(), adm.user, adm.role).await.unwrap().is_empty());
    // Les gestes de la file demandent une session de personnel : un compte famille n'en a aucun.
    let parent = w.m.parent;
    assert!(w.ent.marquer_prete_for_actor(w.etab(), parent.user, parent.role, id2, None).await.is_err());
}

#[tokio::test]
async fn t_repro_avertissements() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let prof = w.m.prof;
    let mut docs = Docs::default();
    // CDM-52, DEM-294 : « pour aujourd'hui » alors que la reprographie demande un délai → avertissement, jamais un refus.
    let doc = docs.doc(&prof, 10);
    let mut d = w.copies(vec![doc], w.site1);
    d.pour_le = w.jour;
    let r = w.deposer_copies(&prof, &docs, d).await.unwrap();
    assert!(r.avertissement_delai && !r.avertissement_cfc);
    let ligne = w.ent.mes_repro_for_actor(w.etab(), prof.user).await.unwrap();
    assert!(ligne[0].avertissement_delai, "tri en tête : la demande reste dans la file, au mieux");
    // Une date lointaine n'avertit pas.
    let doc = docs.doc(&prof, 10);
    let mut d = w.copies(vec![doc], w.site1);
    d.pour_le = w.jour + chrono::Duration::days(30);
    assert!(!w.deposer_copies(&prof, &docs, d).await.unwrap().avertissement_delai);
    // CDM-56, DEM-293 : extraits d'un manuel au-delà de 10 % → avertissement CFC ; sous 10 % rien ; revue à 30 %.
    let o = |genre: &str, copiees, total| OeuvreSaisie { genre: genre.into(), titre: "Manuel de maths".into(), auteur: Some("A. Auteur".into()), editeur: None, pages_copiees: copiees, pages_oeuvre: total };
    assert!(cfc_depasse(&o("livre", 21, 200)) && !cfc_depasse(&o("livre", 20, 200)));
    assert!(cfc_depasse(&o("revue", 31, 100)) && !cfc_depasse(&o("revue", 30, 100)));
    let doc = docs.doc(&prof, 10);
    let mut d = w.copies(vec![doc], w.site1);
    d.oeuvres = vec![o("livre", 40, 200)];
    let r = w.deposer_copies(&prof, &docs, d).await.unwrap();
    assert!(r.avertissement_cfc, "avertissement sans blocage");
    assert_eq!(r.etat.statut, "deposee");
    let doc = docs.doc(&prof, 10);
    let mut d = w.copies(vec![doc], w.site1);
    d.oeuvres = vec![o("livre", 10, 200)];
    assert!(!w.deposer_copies(&prof, &docs, d).await.unwrap().avertissement_cfc);
    // Œuvre incohérente : refus de saisie.
    let doc = docs.doc(&prof, 10);
    let mut d = w.copies(vec![doc], w.site1);
    d.oeuvres = vec![o("livre", 300, 200)];
    assert!(matches!(w.deposer_copies(&prof, &docs, d).await, Err(RefusDemande::Invalide(_))));
    // CDM-53 : 300 pages × 30 exemplaires en couleur → validation, volume trop élevé refusé.
    let doc = docs.doc(&prof, 300);
    let mut d = w.copies(vec![doc], w.site1);
    d.couleur = true;
    let r = w.deposer_copies(&prof, &docs, d).await.unwrap();
    assert_eq!((r.copies_total, r.validation_requise), (9000, true));
    let doc = docs.doc(&prof, 2000);
    let mut d = w.copies(vec![doc], w.site1);
    d.exemplaires = Some(5000);
    assert!(matches!(w.deposer_copies(&prof, &docs, d).await, Err(RefusDemande::Invalide(_))), "10 millions de copies : refusé");
    // Un site inconnu et un lieu de retrait d'un autre site sont refusés.
    let doc = docs.doc(&prof, 10);
    assert!(matches!(w.deposer_copies(&prof, &docs, w.copies(vec![doc], Uuid::new_v4())).await, Err(RefusDemande::Invalide(_))));
    let doc = docs.doc(&prof, 10);
    let mut d = w.copies(vec![doc], w.site1);
    d.retrait_lieu_id = Some(w.lieu2);
    assert!(matches!(w.deposer_copies(&prof, &docs, d).await, Err(RefusDemande::Invalide(_))));
}

#[tokio::test]
async fn t_repro_probleme_et_transfert() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, adm, cpe) = (w.m.prof, w.adm, w.m.cpe);
    let mut docs = Docs::default();
    w.designer(w.site1, &adm).await;
    let id = w.copies_simples(&mut docs, w.site1).await.etat.id;
    // DEM-301 : « PDF illisible » → message fixe au demandeur, la demande attend sa réponse.
    let r = w.ent.signaler_probleme_repro_for_actor(w.etab(), adm.user, adm.role, id, "n'importe quoi").await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
    let e = w.ent.signaler_probleme_repro_for_actor(w.etab(), adm.user, adm.role, id, "pdf_illisible").await.unwrap();
    assert_eq!(e.statut, "en_attente_demandeur");
    let (texte, vis, genre): (String, String, String) = sqlx::query_as(
        "SELECT texte, visibilite, genre FROM ent_dem_message WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(w.etab())
    .bind(id)
    .fetch_one(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((vis.as_str(), genre.as_str()), ("demandeur", "complement_demande"));
    assert!(texte.contains("illisible"));
    assert!(matches!(w.ent.signaler_probleme_repro_for_actor(w.etab(), adm.user, adm.role, id, "pdf_illisible").await, Err(RefusDemande::Conflit(_))));
    // Un demandeur n'a pas ce geste.
    assert_eq!(w.ent.signaler_probleme_repro_for_actor(w.etab(), prof.user, prof.role, id, "pdf_illisible").await.unwrap_err(), RefusDemande::Interdit);
    // CDM-54 : copieur en panne → transfert à la file d'un autre site ; exige un réceptionnaire désigné là-bas.
    let id2 = w.copies_simples(&mut docs, w.site1).await.etat.id;
    assert!(matches!(w.ent.transferer_repro_for_actor(w.etab(), adm.user, adm.role, id2, w.site2).await, Err(RefusDemande::Conflit(_))), "personne sur le site 2");
    assert!(matches!(w.ent.transferer_repro_for_actor(w.etab(), adm.user, adm.role, id2, w.site1).await, Err(RefusDemande::Invalide(_))));
    w.designer(w.site2, &cpe).await;
    w.ent.mettre_en_impression_for_actor(w.etab(), adm.user, adm.role, id2).await.unwrap();
    w.ent.transferer_repro_for_actor(w.etab(), adm.user, adm.role, id2, w.site2).await.unwrap();
    let f1 = w.ent.file_impression_for_actor(w.etab(), adm.user, adm.role).await.unwrap();
    let f2 = w.ent.file_impression_for_actor(w.etab(), cpe.user, cpe.role).await.unwrap();
    assert!(f1.iter().all(|l| l.demande_id != id2));
    assert!(f2.iter().any(|l| l.demande_id == id2 && l.etat == "a_imprimer"), "la nouvelle file reprend la demande, à imprimer");
    // L'ancien réceptionnaire n'a plus la main ; le nouveau marque « prête » avec SON lieu de retrait.
    assert_eq!(w.ent.marquer_prete_for_actor(w.etab(), adm.user, adm.role, id2, None).await.unwrap_err(), RefusDemande::Introuvable);
    assert!(matches!(w.ent.marquer_prete_for_actor(w.etab(), cpe.user, cpe.role, id2, Some(w.lieu1)).await, Err(RefusDemande::Invalide(_))));
    w.ent.marquer_prete_for_actor(w.etab(), cpe.user, cpe.role, id2, Some(w.lieu2)).await.unwrap();
    assert!(w.faits(id2).await.contains(&"repro_transferee".to_string()));
}

#[tokio::test]
async fn t_repro_multi_cibles_et_effectifs() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, adm) = (w.m.prof, w.adm);
    let mut docs = Docs::default();
    w.designer(w.site1, &adm).await;
    // DEM-291 : l'effectif des classes propose les exemplaires (des nombres, jamais des noms).
    let eff = w.ent.effectifs_cibles_for_actor(w.etab(), prof.user, prof.role, &[w.classe], &[]).await.unwrap();
    assert_eq!(eff.len(), 1);
    assert!(eff[0].effectif >= 0 && eff[0].groupe_id.is_none());
    assert!(w.ent.effectifs_cibles_for_actor(w.etab(), w.m.parent.user, w.m.parent.role, &[w.classe], &[]).await.is_err());
    // DEM-368 : exemplaires et lieu de retrait PAR CIBLE ; le total est la somme des cibles.
    let doc = docs.doc(&prof, 6);
    let mut d = w.copies(vec![doc], w.site1);
    d.exemplaires = None;
    d.cibles = vec![
        CibleRepro { classe_id: Some(w.classe), groupe_id: None, exemplaires: 28, retrait_lieu_id: Some(w.lieu1) },
        CibleRepro { classe_id: None, groupe_id: None, exemplaires: 4, retrait_lieu_id: None },
    ];
    let r = w.deposer_copies(&prof, &docs, d).await.unwrap();
    assert_eq!(r.copies_total, 6 * 32);
    let detail = w.ent.detail_repro_for_actor(w.etab(), prof.user, prof.role, r.etat.id).await.unwrap();
    assert_eq!((detail.exemplaires_total, detail.cibles.len()), (32, 2));
    assert_eq!(detail.cibles[0].retrait_lieu_id.or(detail.cibles[1].retrait_lieu_id), Some(w.lieu1));
    // Cible invalide (classe inconnue, deux cibles à la fois, exemplaires hors bornes).
    let doc = docs.doc(&prof, 6);
    let mut d = w.copies(vec![doc], w.site1);
    d.cibles = vec![CibleRepro { classe_id: Some(Uuid::new_v4()), groupe_id: None, exemplaires: 10, retrait_lieu_id: None }];
    assert!(matches!(w.deposer_copies(&prof, &docs, d).await, Err(RefusDemande::Invalide(_))));
    let doc = docs.doc(&prof, 6);
    let mut d = w.copies(vec![doc], w.site1);
    d.cibles = vec![CibleRepro { classe_id: Some(w.classe), groupe_id: Some(Uuid::new_v4()), exemplaires: 10, retrait_lieu_id: None }];
    assert!(matches!(w.deposer_copies(&prof, &docs, d).await, Err(RefusDemande::Invalide(_))));
    let doc = docs.doc(&prof, 6);
    let mut d = w.copies(vec![doc], w.site1);
    d.cibles = vec![CibleRepro { classe_id: None, groupe_id: None, exemplaires: 0, retrait_lieu_id: None }];
    assert!(matches!(w.deposer_copies(&prof, &docs, d).await, Err(RefusDemande::Invalide(_))));
}

#[tokio::test]
async fn t_repro_dupliquer_et_recurrence() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, prof2, adm) = (w.m.prof, w.prof2, w.adm);
    let mut docs = Docs::default();
    w.designer(w.site1, &adm).await;
    let id = w.copies_simples(&mut docs, w.site1).await.etat.id;
    // DEM-366 : options et quantités, jamais les fichiers ; réservé à l'auteur (R-DM52).
    let m = w.ent.dupliquer_repro_for_actor(w.etab(), prof.user, prof.role, id).await.unwrap();
    assert_eq!((m.exemplaires_total, m.format.as_str(), m.couleur, m.site_id), (30, "a4", false, w.site1));
    assert_eq!(w.ent.dupliquer_repro_for_actor(w.etab(), prof2.user, prof2.role, id).await.unwrap_err(), RefusDemande::Introuvable, "pas la demande d'un autre");
    assert_eq!(w.ent.dupliquer_repro_for_actor(w.etab(), adm.user, adm.role, id).await.unwrap_err(), RefusDemande::Introuvable, "ni le réceptionnaire");
    let json = serde_json::to_string(&m).unwrap();
    assert!(!json.contains("document"), "aucun fichier dans le modèle");
    // DEM-367 : une récurrence prépare un rappel (un seul par jour), jamais un dépôt.
    let iso = {
        use chrono::Datelike;
        i16::try_from(w.jour.weekday().number_from_monday()).unwrap()
    };
    let autre_jour = if iso == 1 { 2 } else { 1 };
    assert!(matches!(w.ent.programmer_recurrence_repro_for_actor(w.etab(), prof.user, prof.role, id, 9).await, Err(RefusDemande::Invalide(_))));
    assert_eq!(w.ent.programmer_recurrence_repro_for_actor(w.etab(), prof2.user, prof2.role, id, iso).await.unwrap_err(), RefusDemande::Introuvable);
    let r1 = w.ent.programmer_recurrence_repro_for_actor(w.etab(), prof.user, prof.role, id, autre_jour).await.unwrap();
    assert!(w.ent.recurrences_dues_for_actor(w.etab(), prof.user).await.unwrap().is_empty(), "pas le bon jour");
    let r2 = w.ent.programmer_recurrence_repro_for_actor(w.etab(), prof.user, prof.role, id, iso).await.unwrap();
    assert_eq!(r1, r2, "une seule récurrence active par demande");
    let dues = w.ent.recurrences_dues_for_actor(w.etab(), prof.user).await.unwrap();
    assert_eq!(dues.len(), 1);
    assert_eq!(dues[0].1.exemplaires_total, 30);
    assert!(w.ent.recurrences_dues_for_actor(w.etab(), prof.user).await.unwrap().is_empty(), "un seul rappel par jour");
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_repro WHERE etablissement_id = $1").bind(w.etab()).fetch_one(w.ent.pool_verif()).await.unwrap();
    assert_eq!(n, 1, "aucun dépôt automatique");
}

#[tokio::test]
async fn t_repro_rappel_et_effacement() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, adm) = (w.m.prof, w.adm);
    let mut docs = Docs::default();
    w.designer(w.site1, &adm).await;
    let id = w.copies_simples(&mut docs, w.site1).await.etat.id;
    w.ent.marquer_prete_for_actor(w.etab(), adm.user, adm.role, id, Some(w.lieu1)).await.unwrap();
    // CDM-55, DEM-306 : rappel à J+3 si non retirée ; un seul rappel ; rien avant.
    assert_eq!(w.ent.rappeler_copies_non_retirees(w.etab()).await.unwrap().traitees, 0);
    sqlx::query("UPDATE ent_dem_repro SET prete_le = now() - interval '4 days' WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(id)
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(w.ent.rappeler_copies_non_retirees(w.etab()).await.unwrap().traitees, 1);
    assert_eq!(w.ent.rappeler_copies_non_retirees(w.etab()).await.unwrap().traitees, 0);
    assert!(w.faits(id).await.contains(&"repro_rappel_retrait".to_string()));
    // DEM-256 : PDF effacés à J+60 après « prête » sans retrait — seulement si le socle 7 confirme l'effacement.
    let sans = Docs { efface_ok: false, ..Default::default() };
    assert_eq!(w.ent.effacer_pdf_repro_echus(w.etab(), &sans).await.unwrap().traitees, 0);
    sqlx::query("UPDATE ent_dem_repro SET prete_le = now() - interval '70 days' WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(id)
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(w.ent.effacer_pdf_repro_echus(w.etab(), &sans).await.unwrap().traitees, 0, "le socle 7 n'a pas confirmé : rien n'est marqué");
    let efface = Docs { efface_ok: true, ..Default::default() };
    assert_eq!(w.ent.effacer_pdf_repro_echus(w.etab(), &efface).await.unwrap().traitees, 1);
    assert_eq!(w.ent.effacer_pdf_repro_echus(w.etab(), &efface).await.unwrap().traitees, 0, "rejouable");
    let detail = w.ent.detail_repro_for_actor(w.etab(), prof.user, prof.role, id).await.unwrap();
    assert!(detail.documents.is_empty(), "le fichier effacé n'est plus proposé");
    // Retirée : effacement à J+30.
    let id2 = w.copies_simples(&mut docs, w.site1).await.etat.id;
    w.ent.marquer_prete_for_actor(w.etab(), adm.user, adm.role, id2, None).await.unwrap();
    w.ent.marquer_retiree_for_actor(w.etab(), adm.user, adm.role, id2).await.unwrap();
    assert_eq!(w.ent.effacer_pdf_repro_echus(w.etab(), &efface).await.unwrap().traitees, 0, "trop tôt");
    sqlx::query("UPDATE ent_dem_repro SET retiree_le = now() - interval '31 days' WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(w.etab())
        .bind(id2)
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(w.ent.effacer_pdf_repro_echus(w.etab(), &efface).await.unwrap().traitees, 1);
    assert!(w.faits(id2).await.contains(&"repro_pdf_efface".to_string()));
}

#[tokio::test]
async fn t_repro_compteurs_et_reglages() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, adm, dir1, compta) = (w.m.prof, w.adm, w.m.direction, w.m.compta);
    let mut docs = Docs::default();
    w.designer(w.site1, &adm).await;
    sqlx::query("UPDATE ent_personne SET nom = '@Piege', prenom = 'Ana' WHERE etablissement_id = $1 AND id = $2")
        .bind(w.etab())
        .bind(prof.personne)
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    let id = w.copies_simples(&mut docs, w.site1).await.etat.id;
    w.ent.marquer_prete_for_actor(w.etab(), adm.user, adm.role, id, None).await.unwrap();
    // DEM-304 : compteurs par site, demandeur et couleur ; CSV sans formule. Réservé à la direction et à la comptabilité.
    let (lignes, csv) = w.ent.compteurs_repro_for_actor(w.etab(), compta.user, compta.role, w.jour).await.unwrap();
    assert_eq!(lignes.len(), 1);
    assert_eq!((lignes[0].demandes, lignes[0].copies, lignes[0].couleur), (1, 300, false));
    assert!(csv.contains("'@Piege") && csv.contains(";noir_et_blanc;1;300"), "{csv}");
    assert!(w.ent.compteurs_repro_for_actor(w.etab(), prof.user, prof.role, w.jour).await.is_err());
    assert!(w.ent.compteurs_repro_for_actor(w.etab(), adm.user, adm.role, w.jour).await.is_err(), "le réceptionnaire ne lit pas les compteurs du site");
    assert!(w.ent.compteurs_repro_for_actor(w.etab(), dir1.user, dir1.role, w.jour - chrono::Duration::days(70)).await.unwrap().0.is_empty());
    // Lieux de retrait : unicité par site ; réglage réservé à qui paramètre.
    assert!(matches!(w.ent.ajouter_lieu_retrait_for_actor(w.etab(), dir1.user, dir1.role, w.site1, "  ").await, Err(RefusDemande::Invalide(_))));
    assert!(w.ent.ajouter_lieu_retrait_for_actor(w.etab(), prof.user, prof.role, w.site1, "Couloir").await.is_err());
    assert!(w.ent.ajouter_lieu_retrait_for_actor(w.etab(), dir1.user, dir1.role, w.site1, "salle des professeurs").await.is_err(), "déjà présent");
    assert!(w.ent.ajouter_lieu_retrait_for_actor(w.etab(), dir1.user, dir1.role, Uuid::new_v4(), "Couloir").await.is_err());
}

#[tokio::test]
async fn t_catalogue_n1_3_complet() {
    // Les deux natures sont déclarées complètes (kit §8.5) : assemblage, méta, file par défaut avec repli.
    crate::demandes_catalogue::verifier_catalogue().expect("catalogue complet");
    for code in [NATURE_HSE, NATURE_REPRO] {
        let n = nature(code).expect("nature déclarée");
        verifier_nature(n).unwrap();
        assert_eq!(n.publics, &[PublicNature::Personnel], "personnel seulement : aucune famille ni élève");
        assert!(crate::demandes_catalogue::meta(code).is_some());
    }
    assert_eq!(nature(NATURE_HSE).unwrap().conservation, Conservation::Rh, "HSE : aucune purge automatique");
    // Les HSE sont comptées en quarts d'heure dans la déclaration même (D-061).
    assert!(nature(NATURE_HSE).unwrap().champs.iter().any(|c| c.code == "volume_quarts"));
    assert!(!nature(NATURE_HSE).unwrap().champs.iter().any(|c| c.code.contains("euro") || c.code.contains("montant")));
}
