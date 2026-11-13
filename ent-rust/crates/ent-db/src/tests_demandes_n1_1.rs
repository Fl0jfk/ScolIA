//! Socle N1 (demandes), lot N1-1 : natures, files, membres calculés, routage, repli RH, accusé de réception,
//! contre la base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, les tests sont ignorés en local
//! (échec explicite en CI). Chaque test crée son propre établissement : il ne vérifie que ses propres lignes.
//!
//! Défauts du registre `ALERTES_NEXTJS_PROD.md` (§5 ter) fermés par ce lot (règle 14 de `PILOTAGE.md`) :
//!
//! | Défaut | Test(s) |
//! |---|---|
//! | NJ-85 (corbeille ouverte à tout le personnel) | `t_nj85_file_de_tri_restreinte`, `t_rdm7_aucune_file_tout_le_personnel` |
//! | NJ-94 (organisation par listes d'adresses) | `t_nj94_membres_calcules_du_socle_6`, `t_nj94_depart_rend_les_demandes`, `t_nj94_aucune_adresse_dans_les_files` |
//! | NJ-95 (assistance sans suivi, HTML non échappé) | `t_nj95_assistance_suivie`, `t_nj95_rendu_echappe` |
//!
//! Règles : R-DM1 (`t_rdm1_raccourcis_aucune_demande`), R-DM3 (idem), R-DM4/R-DM5 (cœur N1-0), R-DM6
//! (`t_rdm6_toute_demande_a_file_echeance_et_repli`), R-DM7, R-DM8 (NJ-94), R-DM47/Q-DM14
//! (`t_rdm47_visee_jamais_destinataire`, `t_q_dm14_repli_rh_parametrable`). Cas limites : CDM-02
//! (`t_cdm02_defaut_puis_tri`), CDM-03 (`t_cdm03_file_vide_repli`), CDM-04 (`t_cdm04_site_du_lieu_prioritaire`),
//! CDM-05 (`t_cdm05_routage_suit_l_eleve`), CDM-06 (`t_cdm06_fermeture_accuse_et_delai`), CDM-07
//! (`t_cdm07_nature_desactivee`), CDM-08 (`t_cdm08_nature_version_conservee`). CDM-01 et CDM-11 (requalification),
//! CDM-09 (débit, N1-0) et CDM-10 (alerte, socle 8) ne relèvent pas de ce lot.

use super::*;
use crate::demandes::*;
use crate::demandes_catalogue as cat;
use crate::demandes_files::*;
use crate::demandes_routage::*;
use crate::structure_calendrier::calendrier;
use crate::tests_demandes::{depot, depot_maintenance, fiche, labo, monde, Acteur, Monde};
use chrono::NaiveDate;
use uuid::Uuid;

struct Ctx {
    ent: PgEnt,
    cast: LaboCast,
    m: Monde,
    jour: NaiveDate,
}

async fn ctx() -> Option<Ctx> {
    let (ent, cast) = labo().await?;
    let m = monde(&ent, &cast).await;
    let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())").bind(m.etab).fetch_one(ent.pool_verif()).await.unwrap();
    Some(Ctx { ent, cast, m, jour })
}

impl Ctx {
    fn dir(&self) -> &Acteur {
        &self.m.direction
    }

    async fn init(&self) -> RapportInit {
        self.ent.initialiser_files_defaut_for_actor(self.m.etab, self.dir().user, "direction").await.expect("préréglages")
    }

    async fn fid(&self, code: &str) -> Uuid {
        let (id,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_dem_file WHERE etablissement_id = $1 AND code = $2")
            .bind(self.m.etab)
            .bind(code)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        id
    }

    async fn engager(&self, qui: &Acteur) {
        sqlx::query(
            "INSERT INTO ent_agent_engagement (id, etablissement_id, agent_id, du, nature, employeur_type)
             VALUES (gen_random_uuid(), $1, $2, $3, 'titulaire', 'etat') ON CONFLICT DO NOTHING",
        )
        .bind(self.m.etab)
        .bind(qui.agent.expect("agent"))
        .bind(self.jour - chrono::Duration::days(400))
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
    }

    /// L'agent a cette fonction (socle 6) depuis il y a 30 jours ; il est engagé.
    async fn fonction(&self, qui: &Acteur, code: &str) {
        self.engager(qui).await;
        sqlx::query(
            "INSERT INTO ent_agent_fonction (id, etablissement_id, agent_id, fonction_id, principale, du)
             SELECT gen_random_uuid(), $1, $2, id, false, $3 FROM ent_fonction WHERE code = $4 AND etablissement_id IS NULL",
        )
        .bind(self.m.etab)
        .bind(qui.agent.expect("agent"))
        .bind(self.jour - chrono::Duration::days(30))
        .bind(code)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
    }

    async fn mission(&self, qui: &Acteur, code: &str, du: NaiveDate, au: Option<NaiveDate>) -> Uuid {
        self.engager(qui).await;
        let id = Uuid::new_v4();
        let r = sqlx::query(
            "INSERT INTO ent_mission (id, etablissement_id, type_id, agent_id, du, au)
             SELECT $1, $2, id, $3, $4, $5 FROM ent_mission_type WHERE code = $6 AND etablissement_id IS NULL",
        )
        .bind(id)
        .bind(self.m.etab)
        .bind(qui.agent.expect("agent"))
        .bind(du)
        .bind(au)
        .bind(code)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(r.rows_affected(), 1, "type de mission fourni introuvable");
        id
    }

    async fn activer(&self, code: &str) {
        let decl = nature(code).expect("nature");
        self.ent
            .regler_nature_for_actor(
                self.m.etab,
                self.dir().user,
                "direction",
                code,
                ReglageNature { actif: true, publics: decl.publics.to_vec(), ..Default::default() },
            )
            .await
            .unwrap_or_else(|e| panic!("activation de {code} : {e:?}"));
    }

    async fn sources(&self, qui: &Acteur, d: Option<&DepotDemande>) -> SourcesFiles {
        self.ent.sources_demandes_for_actor(self.m.etab, qui.user, qui.role, d).await.expect("sources")
    }

    async fn deposer(&self, qui: &Acteur, d: DepotDemande) -> Result<EtatDemande, RefusDemande> {
        let s = self.sources(qui, Some(&d)).await;
        self.ent.deposer_demande_for_actor(self.m.etab, qui.user, qui.role, &s, d).await
    }

    async fn file_de(&self, id: Uuid) -> (Uuid, Option<String>) {
        let (f, mot): (Uuid, Option<String>) = sqlx::query_as(
            "SELECT d.file_id, (SELECT motif_code FROM ent_dem_fait x WHERE x.etablissement_id = d.etablissement_id
                                  AND x.demande_id = d.id AND x.type = 'routage')
               FROM ent_dem_demande d WHERE d.etablissement_id = $1 AND d.id = $2",
        )
        .bind(self.m.etab)
        .bind(id)
        .fetch_one(self.ent.pool_verif())
        .await
        .unwrap();
        (f, mot)
    }

    async fn ma_file(&self, qui: &Acteur) -> Vec<DemandeTraitant> {
        let s = self.sources(qui, None).await;
        self.ent.ma_file_for_actor(self.m.etab, qui.user, qui.role, &s).await.expect("ma file")
    }

    async fn nb_demandes(&self) -> i64 {
        let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_demande WHERE etablissement_id = $1")
            .bind(self.m.etab)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        n
    }
}

fn depot_assistance() -> DepotDemande {
    let mut d = depot("numerique.assistance");
    d.valeurs = vec![ValeurSaisie { champ: "zone".into(), rang: 0, valeur: Valeur::Code("compte".into()) }];
    d
}

// ───────────────────────── Kit : l'assemblage du catalogue ─────────────────────────

#[test]
fn t_kit_nature_complete_catalogue() {
    cat::verifier_catalogue().expect("le catalogue complet respecte §8.5");
    for code in [
        "numerique.assistance",
        "numerique.acces_compte",
        "maintenance.intervention",
        "entretien.demande",
        "rh.demande_agent",
        "comptabilite.demande_interne",
        "achat.fournitures",
        "secretariat.demande_interne",
        "direction.question",
        "vie_scolaire.demande_interne",
        "scolarite.attestation",
        "scolarite.duplicata",
        "scolarite.carte_eleve",
        "secretariat.demande_famille",
        "comptabilite.demande_famille",
    ] {
        assert!(nature(code).is_some(), "nature {code} absente du catalogue");
    }
    // Familles et élèves compris (D-061) : au moins une nature ouverte à chaque public.
    for p in [PublicNature::Personnel, PublicNature::Responsable, PublicNature::EleveMajeur, PublicNature::EleveMineur] {
        assert!(cat::toutes_les_natures().any(|n| n.publics.contains(&p)), "aucune nature pour {p:?}");
    }
    // Une nature C2+ (RH) ne va jamais vers une file ordinaire.
    let rh = nature("rh.demande_agent").unwrap();
    assert_eq!(cat::file_decl(rh.file_defaut).unwrap().type_, "confidentielle");
    // Les natures d'avis et de tâches sont déclarées pour chacune.
    for m in cat::METAS {
        assert!(m.avis.contains(&"demandes.prise_en_charge"), "{} : l'avis de prise en charge est activé par défaut", m.code);
    }
    // Un défaut de catalogue est refusé : la file par défaut inconnue fait échouer le contrôle d'une nature copiée.
    let mauvaise = NatureDecl { file_defaut: "n_existe_pas", ..*nature("secretariat.demande_interne").unwrap() };
    assert!(cat::file_decl(mauvaise.file_defaut).is_none());
}

#[test]
fn t_rdm1_raccourcis_aucune_demande() {
    // Un raccourci n'est jamais une nature : aucun champ, un lien profond interne, aucune demande créée (R-DM1, R-DM3).
    assert!(!cat::RACCOURCIS.is_empty());
    for r in cat::RACCOURCIS {
        assert!(nature(r.code).is_none(), "{} est un raccourci, pas une nature", r.code);
        assert!(r.lien.starts_with('/') && !r.lien.contains("://"), "{} : lien profond interne", r.code);
    }
    for code in ["absences.declarer", "agenda.salle", "sorties.projet", "scolarite.certificat", "assiduite.justifier"] {
        assert!(cat::raccourci(code).is_some(), "raccourci {code} absent");
    }
    // Ce qui peut être servi tout de suite (certificat de scolarité) ne devient pas une demande (R-DM3).
    assert!(nature("scolarite.certificat").is_none());
}

// ───────────────────────── R-DM1 / R-DM3 : le guichet ne crée rien pour un raccourci ─────────────────────────

#[tokio::test]
async fn t_rdm1_raccourci_refuse_au_depot() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    let avant = c.nb_demandes().await;
    let r = c.deposer(&c.m.parent, depot("scolarite.certificat")).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Invalide("Cette démarche n'existe pas."));
    assert_eq!(c.nb_demandes().await, avant, "aucune demande créée pour un raccourci");
    // Le catalogue d'un responsable propose le raccourci, jamais une demande.
    let cata = c.ent.catalogue_for_actor(c.m.etab, c.m.parent.user, "parent").await.unwrap();
    assert!(cata.raccourcis.iter().any(|r| r.code == "scolarite.certificat"));
    assert!(cata.natures.iter().all(|n| n.code != "scolarite.certificat"));
    assert_eq!(cata.numeros_urgence, ["3018", "119", "15", "17"]);
    assert!(cata.signalement_grave.contains("situation grave"));
}

// ───────────────────────── NJ-85 : la file de tri a des membres désignés ─────────────────────────

#[tokio::test]
async fn t_nj85_file_de_tri_restreinte() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    // Le prof est le secrétaire de direction désigné : il est le seul membre du tri.
    c.fonction(&c.m.prof, "secretaire_direction").await;
    c.engager(&c.m.cpe).await;
    let tri = c.fid("tri").await;
    // Une nature routée vers la file de tri (réglage de l'établissement).
    c.ent
        .regler_nature_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            "secretariat.demande_interne",
            ReglageNature { actif: true, publics: vec![PublicNature::Personnel], file_defaut_code: Some("tri".into()), ..Default::default() },
        )
        .await
        .unwrap();
    let etat = c.deposer(&c.m.compta, depot("secretariat.demande_interne")).await.expect("dépôt");
    let (file, motif) = c.file_de(etat.id).await;
    assert_eq!(file, tri);
    assert_eq!(motif.as_deref(), Some("file_de_la_nature"));
    // Le membre désigné la voit dans sa file…
    assert!(c.ma_file(&c.m.prof).await.iter().any(|d| d.numero == etat.numero));
    // …les autres membres du personnel, non (NJ-85) : ni dans leur file, ni par accès direct (404 neutre).
    for autre in [&c.m.cpe, &c.m.direction] {
        assert!(c.ma_file(autre).await.iter().all(|d| d.numero != etat.numero), "{} lit la corbeille", autre.role);
        let s = c.sources(autre, None).await;
        let r = c.ent.demande_traitant_for_actor(c.m.etab, autre.user, autre.role, &s, RefDemande::Id(etat.id)).await;
        assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
        assert!(!s.files_de(c.m.etab, autre.personne).contains(&tri));
    }
}

#[tokio::test]
async fn t_rdm7_aucune_file_tout_le_personnel() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    let tri = c.fid("tri").await;
    // La base refuse toute source qui désignerait « tout le personnel » ou une adresse (CHECK de 3042).
    for source in ["tous", "personnel", "email", "*"] {
        let r = sqlx::query(
            "INSERT INTO ent_dem_file_membre_regle (id, etablissement_id, file_id, source, du) VALUES (gen_random_uuid(), $1, $2, $3, $4)",
        )
        .bind(c.m.etab)
        .bind(tri)
        .bind(source)
        .bind(c.jour)
        .execute(c.ent.pool_verif())
        .await;
        assert!(r.is_err(), "source « {source} » acceptée");
    }
    // Une règle « mission » sans type, ou « agent » sans agent, est refusée aussi (forme exacte).
    let r = sqlx::query("INSERT INTO ent_dem_file_membre_regle (id, etablissement_id, file_id, source, du) VALUES (gen_random_uuid(), $1, $2, 'mission', $3)")
        .bind(c.m.etab)
        .bind(tri)
        .bind(c.jour)
        .execute(c.ent.pool_verif())
        .await;
    assert!(r.is_err());
    // Une personne n'est désignée directement que dans un repli ; une file sans repli est refusée (R-DM6).
    let r = c
        .ent
        .ajouter_regle_membre_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            tri,
            RegleMembreSaisie { source: SourceMembre::Personne { personne_id: c.m.cpe.personne }, responsable: false, rang: 1, du: None, au: None },
            true,
        )
        .await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
    let r = c
        .ent
        .creer_file_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            NouvelleFile {
                code: "sans_repli".into(),
                libelle: "Sans repli".into(),
                role_traitant: None,
                type_: TypeFile::Service,
                site_id: None,
                service_calendrier: None,
                delai_vise_jours: None,
                message_accueil: None,
                repli_id: None,
            },
        )
        .await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
    // Une file ne se paramètre pas sans le droit (le prof n'a pas `demandes.parametrer`).
    let r = c.ent.membres_de_file_for_actor(c.m.etab, c.m.prof.user, "prof", tri, None).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
}

// ───────────────────────── NJ-94 : membres calculés depuis le socle 6 ─────────────────────────

#[tokio::test]
async fn t_nj94_membres_calcules_du_socle_6() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    let numerique = c.fid("numerique").await;
    let j = c.jour;
    let membres = |date: Option<NaiveDate>| {
        let (c, numerique) = (&c, numerique);
        async move { c.ent.membres_de_file_for_actor(c.m.etab, c.dir().user, "direction", numerique, date).await.unwrap() }
    };
    assert!(membres(None).await.is_empty(), "aucune mission : aucun membre");
    // Mission « référent numérique » du prof : il devient membre le jour même, sans geste sur la file.
    let mission = c.mission(&c.m.prof, "referent_numerique", j - chrono::Duration::days(20), None).await;
    let m = membres(None).await;
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].personne_id, c.m.prof.personne);
    assert!(m[0].responsable, "règle « référent numérique » : responsable de la file");
    // Fonction « technicien informatique » du cpe : second membre, par une autre source.
    c.fonction(&c.m.cpe, "technicien_informatique").await;
    assert_eq!(membres(None).await.len(), 2);
    // Aperçu à la date : avant la mission du prof, il n'était pas membre.
    let hier = membres(Some(j - chrono::Duration::days(25))).await;
    assert!(hier.iter().all(|x| x.personne_id != c.m.prof.personne));
    // La mission se termine hier : le membre disparaît aujourd'hui, mais reste visible dans l'aperçu daté.
    sqlx::query("UPDATE ent_mission SET au = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(c.m.etab)
        .bind(mission)
        .bind(j - chrono::Duration::days(1))
        .execute(c.ent.pool_verif())
        .await
        .unwrap();
    let apres = membres(None).await;
    assert_eq!(apres.len(), 1);
    assert_eq!(apres[0].personne_id, c.m.cpe.personne);
    assert!(membres(Some(j - chrono::Duration::days(5))).await.iter().any(|x| x.personne_id == c.m.prof.personne));
    // Un agent sans engagement actif n'est jamais membre (départ) : fin de l'engagement du cpe.
    sqlx::query("UPDATE ent_agent_engagement SET au = $3 WHERE etablissement_id = $1 AND agent_id = $2")
        .bind(c.m.etab)
        .bind(c.m.cpe.agent.unwrap())
        .bind(j - chrono::Duration::days(1))
        .execute(c.ent.pool_verif())
        .await
        .unwrap();
    assert!(membres(None).await.is_empty(), "un agent parti n'est plus membre");
    // Désignation d'un agent, datée : membre à partir de sa date, pas avant.
    c.engager(&c.m.compta).await;
    let r = c
        .ent
        .ajouter_regle_membre_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            numerique,
            RegleMembreSaisie {
                source: SourceMembre::Agent { agent_id: c.m.compta.agent.unwrap() },
                responsable: false,
                rang: 3,
                du: Some(j + chrono::Duration::days(3)),
                au: None,
            },
            false,
        )
        .await
        .unwrap();
    assert!(r.applique);
    assert!(membres(None).await.is_empty());
    assert_eq!(membres(Some(j + chrono::Duration::days(3))).await.len(), 1);
}

#[tokio::test]
async fn t_nj94_aucune_adresse_dans_les_files() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    // Aucune colonne ne peut porter une adresse e-mail ni une liste d'identifiants : la structure l'interdit (NJ-94).
    let cols: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text, data_type::text FROM information_schema.columns
          WHERE table_schema = 'public' AND table_name IN ('ent_dem_file', 'ent_dem_file_membre_regle', 'ent_dem_file_palier',
                'ent_dem_reponse_type', 'ent_dem_nature_etab', 'ent_dem_nature_public', 'ent_dem_routage_regle', 'ent_dem_repli_rh')",
    )
    .fetch_all(c.ent.pool_verif())
    .await
    .unwrap();
    assert!(cols.len() > 40);
    for (t, col, ty) in cols {
        let c = col.to_lowercase();
        assert!(!c.contains("mail") && !c.contains("adresse") && !c.contains("courriel"), "{t}.{col} : une adresse dans une file");
        assert!(ty != "ARRAY" && ty != "jsonb" && ty != "json" && ty != "bytea", "{t}.{col} : type {ty} interdit (D-036)");
    }
    // Les migrations du lot : ni JSON, ni tableau, ni binaire (contrôle de revue D-036).
    for sql in [include_str!("../../../migrations/3042_demandes_n1_1_files.sql"), include_str!("../../../migrations/3043_demandes_n1_1_routage.sql")] {
        let code: String = sql.lines().filter(|l| !l.trim_start().starts_with("--")).collect::<Vec<_>>().join("\n").to_lowercase();
        assert!(!code.contains("json") && !code.contains("bytea") && !code.contains("[]"), "type interdit dans la migration");
    }
}

#[tokio::test]
async fn t_nj94_depart_rend_les_demandes() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    let j = c.jour;
    // Le prof est le seul référent numérique ; le cpe est secrétaire de direction (membre du tri, repli du numérique).
    c.mission(&c.m.prof, "referent_numerique", j - chrono::Duration::days(60), None).await;
    c.fonction(&c.m.cpe, "secretaire_direction").await;
    c.activer("numerique.assistance").await;
    let numerique = c.fid("numerique").await;
    let tri = c.fid("tri").await;
    let a = c.deposer(&c.m.compta, depot_assistance()).await.expect("dépôt 1");
    assert_eq!(c.file_de(a.id).await.0, numerique);
    assert!(c.ma_file(&c.m.prof).await.iter().any(|d| d.numero == a.numero));
    assert!(c.ma_file(&c.m.cpe).await.iter().all(|d| d.numero != a.numero), "le cpe n'est pas de la file du numérique");
    // Départ : fin de l'engagement du prof. La file n'a plus de membre actif.
    sqlx::query("UPDATE ent_agent_engagement SET au = $3, motif_fin = 'mutation' WHERE etablissement_id = $1 AND agent_id = $2")
        .bind(c.m.etab)
        .bind(c.m.prof.agent.unwrap())
        .bind(j - chrono::Duration::days(1))
        .execute(c.ent.pool_verif())
        .await
        .unwrap();
    // La demande n'est pas orpheline : elle revient aux membres de la file de repli (le tri), le jour même…
    assert!(c.ma_file(&c.m.cpe).await.iter().any(|d| d.numero == a.numero), "demande orpheline après le départ");
    assert!(c.ma_file(&c.m.prof).await.iter().all(|d| d.numero != a.numero), "l'agent parti garde un accès");
    // …et un nouveau dépôt part directement au repli, avec la raison tracée (CDM-03).
    let b = c.deposer(&c.m.compta, depot_assistance()).await.expect("dépôt 2");
    let (file, motif) = c.file_de(b.id).await;
    assert_eq!(file, tri);
    assert_eq!(motif.as_deref(), Some("repli_file_vide"));
    // Un membre du personnel étranger à la file et à son repli ne la lit toujours pas.
    let s = c.sources(&c.m.direction, None).await;
    let r = c.ent.demande_traitant_for_actor(c.m.etab, c.m.direction.user, "direction", &s, RefDemande::Id(a.id)).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
}

// ───────────────────────── NJ-95 : assistance suivie, rendu échappé ─────────────────────────

#[tokio::test]
async fn t_nj95_assistance_suivie() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    c.mission(&c.m.prof, "referent_numerique", c.jour - chrono::Duration::days(10), None).await;
    c.activer("numerique.assistance").await;
    // Le « ticket » n'est plus un e-mail : une demande numérotée, avec statut, faits chaînés et file.
    let e = c.deposer(&c.m.compta, depot_assistance()).await.expect("dépôt");
    assert_eq!(e.statut, "deposee");
    let mes = c.ent.mes_demandes_for_actor(c.m.etab, c.m.compta.user).await.unwrap();
    assert!(mes.iter().any(|d| d.numero == e.numero && d.statut == "deposee"));
    assert_eq!(c.ent.verifier_chaine_demande(c.m.etab, e.id).await.unwrap(), None, "chaîne de faits intacte");
    let (faits,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(c.m.etab)
        .bind(e.id)
        .fetch_one(c.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(faits, 2, "dépôt et raison du routage sont des faits");
    // Une réponse du référent se rattache à la demande (prise en charge tracée), le demandeur suit l'état.
    let s = c.sources(&c.m.prof, None).await;
    c.ent.agir_sur_demande_for_actor(c.m.etab, c.m.prof.user, "prof", &s, e.id, e.version, Geste::Prendre).await.expect("prise");
    let vu = c.ent.demande_demandeur_for_actor(c.m.etab, c.m.compta.user, e.id).await.unwrap();
    assert_eq!(vu.statut, "prise");
}

#[test]
fn t_nj95_rendu_echappe() {
    // Le nom de l'utilisateur est inséré tel quel dans un message : il est échappé, jamais interprété (NJ-95).
    let piege = "<script>alert(1)</script> & \"x\" 'y' <img src=x onerror=alert(2)>";
    let v = VariablesReponse { prenom: piege, numero: "D2026-00001", date: "2026-11-13", service: "l'assistance" };
    let html = rendre_reponse_html("Bonjour {{prenom}}, votre demande {{numero}} est prise en compte par {{service}}.", &v).unwrap();
    assert!(!html.contains('<') && !html.contains('>'), "balisage non échappé : {html}");
    assert!(html.contains("&lt;script&gt;") && html.contains("&amp;") && html.contains("&quot;") && html.contains("&#39;"));
    // Même le service (« l'assistance ») est échappé : aucune valeur n'échappe à l'échappement.
    assert!(html.contains("l&#39;assistance"));
    // Texte brut : aucun caractère de commande injecté.
    let t = rendre_reponse_texte("{{prenom}}", &VariablesReponse { prenom: "a\u{0007}b\u{001b}[31m", ..Default::default() }).unwrap();
    assert!(!t.chars().any(|c| c.is_control() && c != '\n'));
    // Un modèle n'accepte ni balisage, ni variable libre, ni accolade mal fermée.
    assert!(valider_modele_reponse("Bonjour <b>{{prenom}}</b>").is_err());
    assert!(valider_modele_reponse("Votre mot de passe : {{mot_de_passe}}").is_err());
    assert!(valider_modele_reponse("Bonjour {{prenom").is_err());
    assert!(valider_modele_reponse("{{prenom}} {{numero}} {{date}} {{service}}").is_ok());
}

#[tokio::test]
async fn t_nj95_reponses_types_en_base() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let r = c.init().await;
    assert!(r.reponses_creees >= 1);
    let f = c.fid("secretariat_scolarite").await;
    let ok = c.ent.creer_reponse_type_for_actor(c.m.etab, c.dir().user, "direction", f, "Retard", "Bonjour {{prenom}}, votre demande {{numero}} est en cours.").await;
    assert!(ok.is_ok());
    for mauvais in ["Bonjour <b>{{prenom}}</b>", "{{adresse_mail}}", "<script>x</script>"] {
        let r = c.ent.creer_reponse_type_for_actor(c.m.etab, c.dir().user, "direction", f, "Piège", mauvais).await;
        assert!(matches!(r, Err(RefusDemande::Invalide(_))), "modèle accepté : {mauvais}");
    }
    // La base refuse aussi le balisage (ceinture et bretelles).
    let r = sqlx::query("INSERT INTO ent_dem_reponse_type (id, etablissement_id, file_id, libelle, texte) VALUES (gen_random_uuid(), $1, $2, 'x', 'a <b> c')")
        .bind(c.m.etab)
        .bind(f)
        .execute(c.ent.pool_verif())
        .await;
    assert!(r.is_err());
    // Les membres de la file les lisent ; un autre agent non.
    c.fonction(&c.m.prof, "secretaire_scolarite").await;
    let l = c.ent.reponses_types_for_actor(c.m.etab, c.m.prof.user, "prof", f).await.unwrap();
    assert!(l.len() >= 2);
    let r = c.ent.reponses_types_for_actor(c.m.etab, c.m.cpe.user, "cpe", f).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
}

// ───────────────────────── R-DM6 : une file, une échéance, un repli ─────────────────────────

#[tokio::test]
async fn t_rdm6_toute_demande_a_file_echeance_et_repli() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    c.fonction(&c.m.prof, "secretaire_direction").await;
    c.activer("secretariat.demande_interne").await;
    let e = c.deposer(&c.m.compta, depot("secretariat.demande_interne")).await.expect("dépôt");
    let (file, _) = c.file_de(e.id).await;
    let (repli, actif): (Option<Uuid>, bool) = sqlx::query_as("SELECT file_repli_id, actif FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2")
        .bind(c.m.etab)
        .bind(file)
        .fetch_one(c.ent.pool_verif())
        .await
        .unwrap();
    assert!(actif && repli.is_some(), "toute file a un repli");
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2 AND type IN ('prise_en_charge', 'traitement')")
        .bind(c.m.etab)
        .bind(e.id)
        .fetch_one(c.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 2);
    // La base elle-même refuse une file (hors repli de dernier recours) sans repli.
    let r = sqlx::query("INSERT INTO ent_dem_file (id, etablissement_id, code, libelle, type) VALUES (gen_random_uuid(), $1, 'orpheline', 'Orpheline', 'service')")
        .bind(c.m.etab)
        .execute(c.ent.pool_verif())
        .await;
    assert!(r.is_err());
    // Sans aucune file (établissement non initialisé), rien n'est déposé : fail-closed, jamais « tout le personnel ».
    let Some(c2) = ctx().await else { return };
    c2.activer("secretariat.demande_interne").await;
    let r = c2.deposer(&c2.m.compta, depot("secretariat.demande_interne")).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Indisponible);
}

// ───────────────────────── Routage : règles, défaut, tri, site, élève ─────────────────────────

async fn creer_site(c: &Ctx, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_site (id, etablissement_id, code, libelle) VALUES ($1, $2, $3, $3)")
        .bind(id)
        .bind(c.m.etab)
        .bind(code)
        .execute(c.ent.pool_verif())
        .await
        .unwrap();
    id
}

async fn creer_salle(c: &Ctx, site: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_salle (id, etablissement_id, code, libelle, site_id, created_by_user_id) VALUES ($1, $2, $3, $3, $4, $5)")
        .bind(id)
        .bind(c.m.etab)
        .bind(format!("S{}", &id.simple().to_string()[..10]))
        .bind(site)
        .bind(c.m.direction.user)
        .execute(c.ent.pool_verif())
        .await
        .unwrap();
    id
}

async fn nouvelle_file(c: &Ctx, code: &str, repli: Uuid) -> Uuid {
    c.ent
        .creer_file_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            NouvelleFile {
                code: code.into(),
                libelle: format!("File {code}"),
                role_traitant: Some(format!("l'équipe {code}")),
                type_: TypeFile::Service,
                site_id: None,
                service_calendrier: Some("intendance".into()),
                delai_vise_jours: None,
                message_accueil: None,
                repli_id: Some(repli),
            },
        )
        .await
        .expect("file")
}

#[tokio::test]
async fn t_cdm04_site_du_lieu_prioritaire() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    let (tri, direction) = (c.fid("tri").await, c.fid("direction").await);
    c.fonction(&c.m.prof, "agent_maintenance").await; // membre de « maintenance »
    c.fonction(&c.m.direction, "chef_etablissement").await; // membre de « direction »
    c.fonction(&c.m.cpe, "secretaire_direction").await; // membre du tri
    c.activer("maintenance.intervention").await;
    let (site_a, site_b) = (creer_site(&c, "SITEA").await, creer_site(&c, "SITEB").await);
    let (salle_a, salle_b) = (creer_salle(&c, site_a).await, creer_salle(&c, site_b).await);
    let maint_b = nouvelle_file(&c, "maintenance_b", tri).await;
    c.ent
        .ajouter_regle_membre_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            maint_b,
            RegleMembreSaisie { source: SourceMembre::Fonction { code: "agent_maintenance".into() }, responsable: false, rang: 1, du: None, au: None },
            false,
        )
        .await
        .unwrap();
    let regle_b = c
        .ent
        .ajouter_regle_routage_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            NouvelleRegleRoutage {
                nature_code: "maintenance.intervention".into(),
                categorie_code: None,
                site_id: Some(site_b),
                niveau_code: None,
                public: None,
                urgence: None,
                destination_file_id: maint_b,
                ordre: 10,
                du: None,
                au: None,
            },
        )
        .await
        .unwrap();
    // Règle plus tardive : une panne « bloquante » va à la direction.
    c.ent
        .ajouter_regle_routage_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            NouvelleRegleRoutage {
                nature_code: "maintenance.intervention".into(),
                categorie_code: Some("bloquante".into()),
                site_id: None,
                niveau_code: None,
                public: None,
                urgence: None,
                destination_file_id: direction,
                ordre: 20,
                du: None,
                au: None,
            },
        )
        .await
        .unwrap();
    let avec = |salle: Option<Uuid>, gravite: &str| {
        let mut d = depot("maintenance.intervention");
        d.valeurs = vec![ValeurSaisie { champ: "gravite".into(), rang: 0, valeur: Valeur::Code(gravite.into()) }];
        if let Some(s) = salle {
            d.valeurs.push(ValeurSaisie { champ: "lieu_id".into(), rang: 0, valeur: Valeur::Ref(s) });
        }
        d
    };
    // Panne dans la salle du site B : la règle du site B (ordre 10) gagne sur la règle « bloquante » (ordre 20).
    let e = c.deposer(&c.m.compta, avec(Some(salle_b), "bloquante")).await.unwrap();
    let (file, motif) = c.file_de(e.id).await;
    assert_eq!((file, motif.as_deref()), (maint_b, Some("regle")));
    let (reg,): (Option<Uuid>,) = sqlx::query_as("SELECT ref_id FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'routage'")
        .bind(c.m.etab)
        .bind(e.id)
        .fetch_one(c.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(reg, Some(regle_b), "la raison du routage garde la règle appliquée");
    let (site,): (Option<Uuid>,) = sqlx::query_as("SELECT site_id FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2").bind(c.m.etab).bind(e.id).fetch_one(c.ent.pool_verif()).await.unwrap();
    assert_eq!(site, Some(site_b), "le site du lieu est conservé sur la demande");
    // Même gravité, salle du site A : la règle « bloquante » s'applique.
    let e = c.deposer(&c.m.compta, avec(Some(salle_a), "bloquante")).await.unwrap();
    assert_eq!(c.file_de(e.id).await.0, direction);
    // Sans règle applicable : la file par défaut de la nature (CDM-02).
    let e = c.deposer(&c.m.compta, avec(Some(salle_a), "gene")).await.unwrap();
    let (file, motif) = c.file_de(e.id).await;
    assert_eq!((file, motif.as_deref()), (c.fid("maintenance").await, Some("defaut_nature")));
    // Le simulateur donne la même réponse, sans rien créer (DEM-334), avec la raison lisible.
    let avant = c.nb_demandes().await;
    let sim = c
        .ent
        .simuler_routage_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            SimulationRoutage { nature_code: "maintenance.intervention".into(), site_id: Some(site_b), public: Some(PublicNature::Personnel), ..Default::default() },
        )
        .await
        .unwrap();
    assert_eq!((sim.file_code.as_str(), sim.motif), ("maintenance_b", "regle"));
    assert!(sim.raison.contains("règle n° 10") && sim.raison.contains("maintenance_b"));
    assert_eq!(c.nb_demandes().await, avant);
    // Une règle désactivée ne s'applique plus.
    c.ent.desactiver_regle_routage_for_actor(c.m.etab, c.dir().user, "direction", regle_b).await.unwrap();
    let sim = c
        .ent
        .simuler_routage_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            SimulationRoutage { nature_code: "maintenance.intervention".into(), site_id: Some(site_b), ..Default::default() },
        )
        .await
        .unwrap();
    assert_eq!(sim.file_code, "maintenance");
}

#[tokio::test]
async fn t_cdm02_defaut_puis_tri() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    c.fonction(&c.m.cpe, "secretaire_direction").await; // membre du tri
    c.activer("maintenance.intervention").await;
    // Aucune règle, et la file par défaut de la nature n'existe plus : la file de tri du site, jamais nulle part.
    sqlx::query("UPDATE ent_dem_file SET actif = false WHERE etablissement_id = $1 AND code = 'maintenance'").bind(c.m.etab).execute(c.ent.pool_verif()).await.unwrap();
    let e = c.deposer(&c.m.compta, depot_maintenance()).await.unwrap();
    let (file, motif) = c.file_de(e.id).await;
    assert_eq!(file, c.fid("tri").await);
    assert_eq!(motif.as_deref(), Some("tri_du_site"));
}

#[tokio::test]
async fn t_cdm03_file_vide_repli() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    c.fonction(&c.m.cpe, "secretaire_direction").await; // le tri a un membre, la maintenance aucun
    c.activer("maintenance.intervention").await;
    let e = c.deposer(&c.m.compta, depot_maintenance()).await.unwrap();
    let (file, motif) = c.file_de(e.id).await;
    assert_eq!(file, c.fid("tri").await, "file vide : file de repli");
    assert_eq!(motif.as_deref(), Some("repli_file_vide"));
    // La file visée à l'origine est retenue dans le fait, pour le signal au responsable et à la direction.
    let (r,): (Option<Uuid>,) = sqlx::query_as("SELECT ref_id FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'routage'")
        .bind(c.m.etab)
        .bind(e.id)
        .fetch_one(c.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(r, Some(c.fid("maintenance").await));
    // Le simulateur l'explique en clair.
    let sim = c
        .ent
        .simuler_routage_for_actor(c.m.etab, c.dir().user, "direction", SimulationRoutage { nature_code: "maintenance.intervention".into(), ..Default::default() })
        .await
        .unwrap();
    assert!(sim.repli_utilise && sim.file_initiale.as_deref() == Some("maintenance") && sim.raison.contains("aucun membre actif"));
}

#[tokio::test]
async fn t_cdm05_routage_suit_l_eleve() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    c.fonction(&c.m.prof, "secretaire_scolarite").await;
    c.fonction(&c.m.cpe, "secretaire_direction").await;
    let (tri, vie) = (c.fid("tri").await, c.fid("vie_scolaire").await);
    c.fonction(&c.m.compta, "cpe").await; // membre de la vie scolaire
    // Règle par niveau de l'élève : les élèves de 2NDE vont à la vie scolaire.
    c.ent
        .ajouter_regle_routage_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            NouvelleRegleRoutage {
                nature_code: "scolarite.attestation".into(),
                categorie_code: None,
                site_id: None,
                niveau_code: Some("2NDE".into()),
                public: Some(PublicNature::Responsable),
                urgence: None,
                destination_file_id: vie,
                ordre: 10,
                du: None,
                au: None,
            },
        )
        .await
        .unwrap();
    let sim = |niveau: &str| SimulationRoutage {
        nature_code: "scolarite.attestation".into(),
        niveau_code: Some(niveau.into()),
        public: Some(PublicNature::Responsable),
        ..Default::default()
    };
    let a = c.ent.simuler_routage_for_actor(c.m.etab, c.dir().user, "direction", sim("2NDE")).await.unwrap();
    assert_eq!(a.file_code, "vie_scolaire", "le niveau de l'élève sujet pilote le routage");
    let b = c.ent.simuler_routage_for_actor(c.m.etab, c.dir().user, "direction", sim("6EME")).await.unwrap();
    assert_eq!(b.file_code, "secretariat_scolarite");
    let _ = tri;
    // Le dépôt d'un responsable pour un élève exécute la dérivation du niveau sans erreur (aucune inscription : défaut).
    c.activer("scolarite.attestation").await;
    let mut d = depot("scolarite.attestation");
    d.eleve_id = Some(c.m.eleve);
    d.valeurs = vec![ValeurSaisie { champ: "type_attestation".into(), rang: 0, valeur: Valeur::Code("assiduite".into()) }];
    let e = c.deposer(&c.m.parent, d).await.expect("dépôt famille");
    assert_eq!(c.file_de(e.id).await.0, c.fid("secretariat_scolarite").await);
}

// ───────────────────────── Natures activées : bornes, versions, désactivation ─────────────────────────

#[tokio::test]
async fn t_activation_nature_bornes() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    let r = |reglage: ReglageNature, code: &'static str| {
        let c = &c;
        async move { c.ent.regler_nature_for_actor(c.m.etab, c.dir().user, "direction", code, reglage).await }
    };
    let pers = vec![PublicNature::Personnel];
    // Aucun public : on n'active pas dans le vide.
    assert!(matches!(r(ReglageNature { actif: true, ..Default::default() }, "rh.demande_agent").await, Err(RefusDemande::Invalide(_))));
    // Un public que la nature n'autorise pas (jamais une nature RH aux familles).
    let fam = ReglageNature { actif: true, publics: vec![PublicNature::Responsable], ..Default::default() };
    assert!(matches!(r(fam, "rh.demande_agent").await, Err(RefusDemande::Invalide(_))));
    // Un délai se raccourcit, ne se rallonge pas.
    let long = ReglageNature { actif: true, publics: pers.clone(), delai_prise_jours: Some(4), ..Default::default() };
    assert!(matches!(r(long, "rh.demande_agent").await, Err(RefusDemande::Invalide(_))));
    // Une nature confidentielle n'est jamais routée vers une file ordinaire.
    let ordinaire = ReglageNature { actif: true, publics: pers.clone(), file_defaut_code: Some("secretariat".into()), ..Default::default() };
    assert!(matches!(r(ordinaire, "rh.demande_agent").await, Err(RefusDemande::Invalide(_))));
    // Une file inconnue est refusée.
    let inconnue = ReglageNature { actif: true, publics: pers.clone(), file_defaut_code: Some("fantome".into()), ..Default::default() };
    assert!(matches!(r(inconnue, "secretariat.demande_interne").await, Err(RefusDemande::Invalide(_))));
    // Un délai raccourci s'applique à l'échéance, calculée en jours ouvrés du calendrier du service.
    c.fonction(&c.m.prof, "secretaire_direction").await;
    let court = ReglageNature { actif: true, publics: pers, delai_prise_jours: Some(1), delai_traitement_jours: Some(2), ..Default::default() };
    r(court, "secretariat.demande_interne").await.unwrap();
    let e = c.deposer(&c.m.compta, depot("secretariat.demande_interne")).await.unwrap();
    let attendu = {
        let mut conn = c.ent.pool_verif().acquire().await.unwrap();
        calendrier::echeance_jours_ouvres(&mut conn, c.m.etab, c.jour, 2, "secretariat").await.unwrap()
    };
    let acc = c.ent.accuse_reception_for_actor(c.m.etab, c.m.compta.user, e.id).await.unwrap();
    assert_eq!(acc.delai_indicatif, Some(attendu), "délai de 2 jours ouvrés, non de 5");
    // Le journal garde les changements de réglage (qui, quand, avant, après).
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_reglage_journal WHERE etablissement_id = $1 AND sujet = 'nature' AND cible = 'secretariat.demande_interne'")
        .bind(c.m.etab)
        .fetch_one(c.ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 1);
    // Sans le droit de paramétrer : refus.
    let r = c.ent.regler_nature_for_actor(c.m.etab, c.m.prof.user, "prof", "secretariat.demande_interne", ReglageNature::default()).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
}

#[tokio::test]
async fn t_cdm07_nature_desactivee() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    c.fonction(&c.m.prof, "secretaire_direction").await;
    let avant = c.nb_demandes().await;
    // Jamais activée : « cette démarche n'est plus proposée », aucune demande créée.
    let r = c.deposer(&c.m.compta, depot("secretariat.demande_interne")).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Invalide(NATURE_NON_PROPOSEE));
    c.activer("secretariat.demande_interne").await;
    assert!(c.deposer(&c.m.compta, depot("secretariat.demande_interne")).await.is_ok());
    // Désactivée depuis (ancien lien) : même réponse, rien de créé.
    let nb = c.nb_demandes().await;
    assert_eq!(nb, avant + 1);
    c.ent
        .regler_nature_for_actor(c.m.etab, c.dir().user, "direction", "secretariat.demande_interne", ReglageNature { actif: false, ..Default::default() })
        .await
        .unwrap();
    let r = c.deposer(&c.m.compta, depot("secretariat.demande_interne")).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Invalide(NATURE_NON_PROPOSEE));
    assert_eq!(c.nb_demandes().await, nb);
    // Elle disparaît aussi du catalogue du demandeur.
    let cata = c.ent.catalogue_for_actor(c.m.etab, c.m.compta.user, "compta").await.unwrap();
    assert!(cata.natures.iter().all(|n| n.code != "secretariat.demande_interne"));
}

#[tokio::test]
async fn t_cdm08_nature_version_conservee() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    c.fonction(&c.m.prof, "secretaire_direction").await;
    c.activer("secretariat.demande_interne").await;
    let e = c.deposer(&c.m.compta, depot("secretariat.demande_interne")).await.unwrap();
    // La demande porte la version de la nature au dépôt ; un réglage ultérieur n'y touche pas.
    c.ent
        .regler_nature_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            "secretariat.demande_interne",
            ReglageNature { actif: true, publics: vec![PublicNature::Personnel], delai_traitement_jours: Some(1), ..Default::default() },
        )
        .await
        .unwrap();
    let (v,): (i16,) = sqlx::query_as("SELECT nature_version FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2").bind(c.m.etab).bind(e.id).fetch_one(c.ent.pool_verif()).await.unwrap();
    assert_eq!(v, nature("secretariat.demande_interne").unwrap().version);
}

// ───────────────────────── Fermetures et accusé de réception ─────────────────────────

#[tokio::test]
async fn t_cdm06_fermeture_accuse_et_delai() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    c.fonction(&c.m.prof, "secretaire_direction").await;
    c.activer("secretariat.demande_interne").await;
    // Fermeture du secrétariat de dix jours à compter d'aujourd'hui (jour métier).
    let (annee,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante").bind(c.m.etab).fetch_one(c.ent.pool_verif()).await.unwrap();
    let ev = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_calendrier_evenement (id, etablissement_id, annee_id, type, libelle, du, au, effet) VALUES ($1, $2, $3, 'fermeture', 'Fermeture de test', $4, $5, 'sans_cours')")
        .bind(ev)
        .bind(c.m.etab)
        .bind(annee)
        .bind(c.jour - chrono::Duration::days(1))
        .bind(c.jour + chrono::Duration::days(10))
        .execute(c.ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_calendrier_cible (id, etablissement_id, evenement_id, service) VALUES (gen_random_uuid(), $1, $2, 'secretariat')").bind(c.m.etab).bind(ev).execute(c.ent.pool_verif()).await.unwrap();
    let e = c.deposer(&c.m.compta, depot("secretariat.demande_interne")).await.expect("dépôt pendant la fermeture");
    let acc = c.ent.accuse_reception_for_actor(c.m.etab, c.m.compta.user, e.id).await.unwrap();
    // Accusé immédiat avec la mention de fermeture ; le délai part à la réouverture (R-DM22).
    let reouverture = acc.mention_fermeture.expect("mention de fermeture");
    assert!(reouverture > c.jour + chrono::Duration::days(10), "réouverture après la fermeture : {reouverture}");
    assert!(acc.delai_indicatif.unwrap() > reouverture, "le délai court depuis la réouverture");
}

#[tokio::test]
async fn t_accuse_reception_service_role_delai() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    c.fonction(&c.m.prof, "secretaire_scolarite").await;
    c.activer("scolarite.attestation").await;
    let mut d = depot("scolarite.attestation");
    d.eleve_id = Some(c.m.eleve);
    d.valeurs = vec![ValeurSaisie { champ: "type_attestation".into(), rang: 0, valeur: Valeur::Code("scolarite_periode".into()) }];
    let e = c.deposer(&c.m.parent, d).await.expect("dépôt famille");
    let acc = c.ent.accuse_reception_for_actor(c.m.etab, c.m.parent.user, e.id).await.unwrap();
    // Le service et le rôle, un délai indicatif daté ; jamais le nom d'une personne avant la prise (R-DM53).
    assert_eq!(acc.service, "Secrétariat des élèves");
    assert_eq!(acc.role_traitant, "le secrétariat des élèves");
    assert!(acc.delai_indicatif.is_some_and(|d| d > c.jour));
    assert_eq!(acc.mention_fermeture, None);
    assert_eq!(acc.mentions, MENTIONS_ACCUSE_EPLE.to_vec(), "mentions EPLE d'une nature à décision de l'usager");
    assert!(acc.message_accueil.is_some());
    // Seuls le demandeur et le déposant la lisent : un autre responsable (hors lien) ou un membre de la file : 404.
    let tom = fiche(&c.ent, c.m.etab, c.cast.parent_tom.user_id, "parent", false).await;
    assert_eq!(c.ent.accuse_reception_for_actor(c.m.etab, tom.user, e.id).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(c.ent.accuse_reception_for_actor(c.m.etab, c.m.prof.user, e.id).await.unwrap_err(), RefusDemande::Introuvable);
}

// ───────────────────────── Familles et élèves ─────────────────────────

#[tokio::test]
async fn t_familles_publics_liens_et_catalogue() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    c.fonction(&c.m.prof, "secretaire_scolarite").await;
    let famille = |code: &str, d: &mut DepotDemande| {
        d.eleve_id = Some(c.m.eleve);
        d.valeurs = vec![ValeurSaisie { champ: "type_attestation".into(), rang: 0, valeur: Valeur::Code("assiduite".into()) }];
        let _ = code;
    };
    // Nature non ouverte aux responsables : « n'est plus proposée » (CDM-07/78), rien créé.
    let mut d = depot("scolarite.attestation");
    famille("a", &mut d);
    c.ent
        .regler_nature_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            "scolarite.attestation",
            ReglageNature { actif: true, publics: vec![PublicNature::EleveMajeur], ..Default::default() },
        )
        .await
        .unwrap();
    assert_eq!(c.deposer(&c.m.parent, d.clone()).await.unwrap_err(), RefusDemande::Invalide(NATURE_NON_PROPOSEE));
    c.activer("scolarite.attestation").await;
    assert!(c.deposer(&c.m.parent, d.clone()).await.is_ok());
    // Un responsable sans lien actif avec l'élève ne dépose rien pour lui (droit de lien du socle 4).
    let tom = fiche(&c.ent, c.m.etab, c.cast.parent_tom.user_id, "parent", false).await;
    assert_eq!(c.deposer(&tom, d.clone()).await.unwrap_err(), RefusDemande::Interdit);
    // Une famille ne dépose jamais une nature du personnel (RH, maintenance).
    c.activer("maintenance.intervention").await;
    assert_eq!(c.deposer(&c.m.parent, depot_maintenance()).await.unwrap_err(), RefusDemande::Interdit);
    // Catalogue : le responsable ne voit que ce qu'il peut faire ; le personnel ne voit pas les natures des familles.
    let cp = c.ent.catalogue_for_actor(c.m.etab, c.m.parent.user, "parent").await.unwrap();
    assert!(cp.natures.iter().any(|n| n.code == "scolarite.attestation" && n.eleve_requis));
    assert!(cp.natures.iter().all(|n| n.code != "maintenance.intervention"));
    assert!(cp.raccourcis.iter().all(|r| r.publics.contains(&PublicNature::Responsable)));
    let cs = c.ent.catalogue_for_actor(c.m.etab, c.m.compta.user, "compta").await.unwrap();
    assert!(cs.natures.iter().any(|n| n.code == "maintenance.intervention"));
    assert!(cs.natures.iter().all(|n| n.code != "scolarite.attestation"));
    assert!(cs.raccourcis.iter().any(|r| r.code == "absences.declarer"));
    // Un public « élève mineur » n'est jamais ouvert sur une nature qui ne le déclare pas (CDM-75).
    let r = c
        .ent
        .regler_nature_for_actor(
            c.m.etab,
            c.dir().user,
            "direction",
            "scolarite.attestation",
            ReglageNature { actif: true, publics: vec![PublicNature::EleveMineur], ..Default::default() },
        )
        .await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
}

// ───────────────────────── R-DM47, Q-DM14 : la personne visée, le repli RH réglable ─────────────────────────

/// Établissement où la RH, le repli par défaut (gestionnaire, direction) et leurs membres existent.
async fn monde_rh(c: &Ctx) {
    c.init().await;
    c.fonction(&c.m.prof, "secretaire_direction").await; // seul membre de la file RH
    c.fonction(&c.m.compta, "gestionnaire").await; // repli RH : attaché de gestion
    c.fonction(&c.m.direction, "chef_etablissement").await; // repli RH et direction
    c.activer("rh.demande_agent").await;
}

fn sim_rh(vises: Vec<Uuid>) -> SimulationRoutage {
    SimulationRoutage { nature_code: "rh.demande_agent".into(), vises, ..Default::default() }
}

#[tokio::test]
async fn t_rdm47_visee_jamais_destinataire() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    monde_rh(&c).await;
    let s = |v: Vec<Uuid>| {
        let c = &c;
        async move { c.ent.simuler_routage_for_actor(c.m.etab, c.dir().user, "direction", sim_rh(v)).await }
    };
    // Sans personne visée : la file RH (confidentielle), un destinataire.
    let a = s(vec![]).await.unwrap();
    assert_eq!((a.file_code.as_str(), a.nb_destinataires, a.repli_utilise), ("rh", 1, false));
    // La personne visée est le seul membre de la file RH : la demande part au repli RH, sans elle.
    let b = s(vec![c.m.prof.personne]).await.unwrap();
    assert_eq!((b.file_code.as_str(), b.motif), ("repli_rh", "repli_personne_visee"));
    assert_eq!(b.nb_destinataires, 2, "attaché de gestion et direction");
    assert_eq!(b.role_traitant, "la direction et l'attaché de gestion", "le demandeur voit à qui part sa demande");
    // Un destinataire du repli est lui-même visé : on passe au suivant.
    let d = s(vec![c.m.prof.personne, c.m.compta.personne]).await.unwrap();
    assert_eq!((d.file_code.as_str(), d.nb_destinataires), ("repli_rh", 1));
    // Tous les destinataires du repli et de la direction sont visés : aucune destination, rien n'est ouvert à tous.
    let e = s(vec![c.m.prof.personne, c.m.compta.personne, c.m.direction.personne]).await;
    assert_eq!(e.unwrap_err(), RefusDemande::Indisponible);
    // Le dépôt réel suit la même logique : la demande RH arrive à la file RH, lisible du seul membre.
    let r = c.deposer(&c.m.cpe, depot_rh()).await.expect("dépôt RH");
    assert_eq!(c.file_de(r.id).await.0, c.fid("rh").await);
    let acc = c.ent.accuse_reception_for_actor(c.m.etab, c.m.cpe.user, r.id).await.unwrap();
    assert_eq!(acc.role_traitant, "la gestion du personnel");
    assert!(c.ma_file(&c.m.prof).await.iter().any(|x| x.numero == r.numero));
    assert!(c.ma_file(&c.m.compta).await.iter().all(|x| x.numero != r.numero), "le repli ne lit pas la RH tant que la file a un membre");
}

fn depot_rh() -> DepotDemande {
    let mut d = depot("rh.demande_agent");
    d.valeurs = vec![ValeurSaisie { champ: "type_demande".into(), rang: 0, valeur: Valeur::Code("formation".into()) }];
    d
}

#[tokio::test]
async fn t_q_dm14_repli_rh_parametrable() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    monde_rh(&c).await;
    let dir = c.dir().user;
    // Défaut : attaché de gestion et direction ; l'explication est en clair à l'écran.
    let vue = c.ent.repli_rh_for_actor(c.m.etab, dir, "direction").await.unwrap().expect("repli par défaut posé");
    assert_eq!(vue.code, "repli_rh");
    assert_eq!(vue.membres.len(), 2);
    assert!(vue.explication.contains("jamais envoyée") && vue.explication.contains("OGEC") && vue.explication.contains("journalisé"));
    // Seuls les profils qui paramètrent le guichet le règlent.
    assert_eq!(c.ent.repli_rh_for_actor(c.m.etab, c.m.prof.user, "prof").await.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(c.ent.definir_repli_rh_for_actor(c.m.etab, c.m.prof.user, "prof", vue.file_id, true).await.unwrap_err(), RefusDemande::Interdit);
    // Un repli sans destinataire est refusé (au moins un destinataire).
    let vide = c
        .ent
        .creer_file_for_actor(
            c.m.etab,
            dir,
            "direction",
            NouvelleFile {
                code: "ogec".into(),
                libelle: "Président de l'OGEC".into(),
                role_traitant: Some("le président de l'OGEC".into()),
                type_: TypeFile::Repli,
                site_id: None,
                service_calendrier: None,
                delai_vise_jours: None,
                message_accueil: None,
                repli_id: None,
            },
        )
        .await
        .unwrap();
    assert!(matches!(c.ent.definir_repli_rh_for_actor(c.m.etab, dir, "direction", vide, true).await, Err(RefusDemande::Invalide(_))));
    // Une file de service n'est pas un repli : il est hors du service concerné.
    assert!(matches!(c.ent.definir_repli_rh_for_actor(c.m.etab, dir, "direction", c.fid("rh").await, true).await, Err(RefusDemande::Invalide(_))));
    // Tout choix est possible : une personne extérieure (le président de l'OGEC), après avertissement.
    let exterieur = fiche(&c.ent, c.m.etab, c.cast.parent_tom.user_id, "parent", false).await;
    let regle = RegleMembreSaisie { source: SourceMembre::Personne { personne_id: exterieur.personne }, responsable: true, rang: 1, du: None, au: None };
    let r1 = c.ent.ajouter_regle_membre_for_actor(c.m.etab, dir, "direction", vide, regle.clone(), false).await.unwrap();
    assert!(!r1.applique && r1.avertissements == [Avertissement::PersonneExterieure { personne_id: exterieur.personne }]);
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_file_membre_regle WHERE etablissement_id = $1 AND file_id = $2").bind(c.m.etab).bind(vide).fetch_one(c.ent.pool_verif()).await.unwrap();
    assert_eq!(n, 0, "rien n'est écrit tant que l'avertissement n'est pas confirmé");
    let r2 = c.ent.ajouter_regle_membre_for_actor(c.m.etab, dir, "direction", vide, regle, true).await.unwrap();
    assert!(r2.applique);
    // Le choix du repli rappelle l'avertissement, puis s'applique ; le changement est journalisé.
    let p1 = c.ent.definir_repli_rh_for_actor(c.m.etab, dir, "direction", vide, false).await.unwrap();
    assert!(!p1.applique && !p1.avertissements.is_empty());
    let p2 = c.ent.definir_repli_rh_for_actor(c.m.etab, dir, "direction", vide, true).await.unwrap();
    assert!(p2.applique);
    let j: Vec<(Option<String>, Option<String>, Option<Uuid>)> =
        sqlx::query_as("SELECT avant, apres, par_user_id FROM ent_dem_reglage_journal WHERE etablissement_id = $1 AND sujet = 'repli_rh' ORDER BY le DESC, id LIMIT 1")
            .bind(c.m.etab)
            .fetch_all(c.ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(j[0], (Some("repli_rh".to_string()), Some("ogec".to_string()), Some(dir)));
    // Le routage suit le réglage, et la personne visée reste exclue : ici le président de l'OGEC est visé → direction.
    let s = |v: Vec<Uuid>| {
        let c = &c;
        async move { c.ent.simuler_routage_for_actor(c.m.etab, dir, "direction", sim_rh(v)).await.unwrap() }
    };
    let a = s(vec![c.m.prof.personne]).await;
    assert_eq!((a.file_code.as_str(), a.nb_destinataires, a.role_traitant.as_str()), ("ogec", 1, "le président de l'OGEC"));
    let b = s(vec![c.m.prof.personne, exterieur.personne]).await;
    assert_eq!(b.file_code, "direction", "repli sans destinataire : la direction");
    // Au moins un destinataire : on ne retire pas le dernier du repli réglé.
    let regle_id = r2.regle_id.unwrap();
    let r = c.ent.retirer_regle_membre_for_actor(c.m.etab, dir, "direction", regle_id).await;
    assert!(matches!(r, Err(RefusDemande::Conflit(_))));
    let (restantes,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_file_membre_regle WHERE etablissement_id = $1 AND file_id = $2 AND annule_le IS NULL").bind(c.m.etab).bind(vide).fetch_one(c.ent.pool_verif()).await.unwrap();
    assert_eq!(restantes, 1, "le retrait refusé est défait");
    // Le journal des réglages est en ajout seul.
    let r = sqlx::query("UPDATE ent_dem_reglage_journal SET apres = 'x' WHERE etablissement_id = $1").bind(c.m.etab).execute(c.ent.pool_verif()).await;
    assert!(r.is_err());
    let r = sqlx::query("DELETE FROM ent_dem_reglage_journal WHERE etablissement_id = $1").bind(c.m.etab).execute(c.ent.pool_verif()).await;
    assert!(r.is_err());
}

// ───────────────────────── Préréglages ─────────────────────────

#[tokio::test]
async fn t_preglages_files_idempotents() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let r = c.init().await;
    assert_eq!(r.files_creees as usize, cat::FILES_DEFAUT.len());
    assert!(r.repli_rh_pose && r.regles_creees > 10);
    // Rejouable : aucune file en double, aucun repli reposé.
    let r2 = c.init().await;
    assert_eq!((r2.files_creees, r2.regles_creees, r2.repli_rh_pose), (0, 0, false));
    // Chaque file a un repli (sauf le repli de dernier recours) ; confidentielle et direction : repli hors service.
    let l: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT f.code, f.type, r.type FROM ent_dem_file f LEFT JOIN ent_dem_file r ON r.etablissement_id = f.etablissement_id AND r.id = f.file_repli_id
          WHERE f.etablissement_id = $1",
    )
    .bind(c.m.etab)
    .fetch_all(c.ent.pool_verif())
    .await
    .unwrap();
    for (code, type_, repli) in &l {
        if type_ != "repli" {
            assert!(repli.is_some(), "file {code} sans repli");
        }
        if type_ == "confidentielle" || type_ == "direction" {
            assert_eq!(repli.as_deref(), Some("repli"), "file {code} : repli hors service");
        }
    }
    // Les natures restent désactivées tant que l'établissement ne les active pas (D-023).
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_nature_etab WHERE etablissement_id = $1 AND actif").bind(c.m.etab).fetch_one(c.ent.pool_verif()).await.unwrap();
    assert_eq!(n, 0);
    // Réservé à qui paramètre le guichet.
    let r = c.ent.initialiser_files_defaut_for_actor(c.m.etab, c.m.prof.user, "prof").await;
    assert_eq!(r.unwrap_err(), RefusDemande::Interdit);
}

// ───────────────────────── Cloisonnement entre établissements ─────────────────────────

#[tokio::test]
async fn t_separation_files_entre_etablissements() {
    let Some(c) = ctx().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    c.init().await;
    let Some(autre) = ctx().await else { return };
    // Un autre établissement ne voit ni les files, ni les règles, ni les réglages du premier (RLS).
    autre.init().await;
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_file WHERE etablissement_id = $1").bind(c.m.etab).fetch_one(c.ent.pool_verif()).await.unwrap();
    assert_eq!(n as usize, cat::FILES_DEFAUT.len());
    assert_ne!(c.m.etab, autre.m.etab);
    // Une file d'un autre établissement est introuvable : l'identifiant seul ne donne aucun accès.
    let fb = autre.fid("tri").await;
    let l = c.ent.membres_de_file_for_actor(c.m.etab, c.m.direction.user, "direction", fb, None).await;
    assert_eq!(l.unwrap_err(), RefusDemande::Introuvable);
    let r = c.ent.definir_repli_rh_for_actor(c.m.etab, c.m.direction.user, "direction", fb, true).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
    // Les règles de routage ne pointent jamais une file d'un autre établissement.
    let r = c
        .ent
        .ajouter_regle_routage_for_actor(
            c.m.etab,
            c.m.direction.user,
            "direction",
            NouvelleRegleRoutage {
                nature_code: "secretariat.demande_interne".into(),
                categorie_code: None,
                site_id: None,
                niveau_code: None,
                public: None,
                urgence: None,
                destination_file_id: fb,
                ordre: 10,
                du: None,
                au: None,
            },
        )
        .await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))));
}
