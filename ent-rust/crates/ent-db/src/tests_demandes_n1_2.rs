//! Socle N1 (demandes), lot N1-2 : prise en charge, délégation, réaffectation, consultation, désignation d'une
//! personne visée, requalification, échanges, pièces, abonnés, « Résolu ? » et preuves visibles du demandeur, contre
//! la base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, les tests sont ignorés en local (échec explicite
//! en CI). Chaque test crée son propre établissement : il ne vérifie que ses propres lignes. Aucun test ne dépend de
//! l'heure ni du jour réels : les échéances sont reculées par rapport à `now()` et les durées comparées entre elles.
//!
//! Défauts du registre `ALERTES_NEXTJS_PROD.md` (§5 ter) fermés par ce lot (règle 14 de `PILOTAGE.md`) :
//!
//! | Défaut | Test(s) |
//! |---|---|
//! | NJ-81 (une demande d'une autre file peut être détournée) | `t_nj81_detournement_autre_file`, `t_nj81_transfert_et_prise_dans_la_file` |
//! | NJ-83 (les pièces de toutes les files sont lisibles) | `t_nj83_pieces_cloisonnees` |
//! | NJ-80 étendu aux échanges et commentaires (limite laissée par N1-0) | `t_nj80_echanges_sans_droit_refuse` |
//!
//! Règles et cas : R-DM53 (`t_trois_preuves_visibles`), R-DM54 (`t_retard_montre_et_responsable_prevenu`), R-DM47
//! (`t_designation_personne_visee`), CDM-01 et CDM-11 (`t_requalification_cdm01_cdm11`), CDM-82 (la vue n'est posée
//! que par le détail d'un membre : `t_trois_preuves_visibles`), CDM-83 et DEM-384 (`t_reaffectation_un_geste_absent`),
//! DEM-394 et CDM-86 (`t_resolu_oui_non`), DEM-206 (`t_consultation_collegue`), DEM-399 (`t_vues_non_prises`).

use super::*;
use crate::demandes::*;
use crate::demandes_echanges::*;
use crate::demandes_echanges::MessageEnvoye;
use crate::demandes_files::*;
use crate::demandes_prise::*;
use crate::demandes_routage::*;
use crate::tests_demandes::{depot, depot_maintenance, fiche, labo, monde, Acteur, Monde};
use chrono::NaiveDate;
use uuid::Uuid;

// ───────────────────────── Ports de test ─────────────────────────

/// Socle 7 de test : des documents déposés par des personnes ; un jeton lisible par pièce.
#[derive(Default)]
struct Docs {
    propres: Vec<(Uuid, Uuid)>,
}

impl Docs {
    /// Un nouveau document déposé par `qui` (socle 7 simulé).
    fn doc(&mut self, qui: &Acteur) -> Uuid {
        let d = Uuid::new_v4();
        self.propres.push((d, qui.personne));
        d
    }
}

impl PortDocuments for Docs {
    fn document_depose_par(&self, _: Uuid, document_id: Uuid, personne_id: Uuid) -> bool {
        self.propres.contains(&(document_id, personne_id))
    }
    fn jeton_lecture(&self, _: Uuid, document_id: Uuid, _: Uuid) -> Option<String> {
        let _ = document_id;
        Some(format!("jeton-court-{}", Uuid::new_v4()))
    }
}

struct Messagerie(bool);

impl PortMessagerie for Messagerie {
    fn peut_convertir_fil(&self, _: Uuid, _: Uuid, _: Uuid) -> bool {
        self.0
    }
    fn conversation_depuis_demande(&self, _: Uuid, _: Uuid, _: Uuid) -> Option<Uuid> {
        self.0.then(Uuid::new_v4)
    }
}

// ───────────────────────── Le monde de test ─────────────────────────

/// Un établissement avec : un repli, la file A (prof membre, cpe **responsable**, adm membre), la file B (direction
/// responsable), la file C confidentielle (direction responsable). Les natures de test sont activées.
struct W {
    ent: PgEnt,
    m: Monde,
    jour: NaiveDate,
    adm: Acteur,
    repli: Uuid,
    fa: Uuid,
    fb: Uuid,
    fc: Uuid,
    tom_user: Uuid,
}

fn code_aleatoire(prefixe: &str) -> String {
    format!("{prefixe}{}", &Uuid::new_v4().simple().to_string()[..12])
}

impl W {
    async fn nouveau() -> Option<W> {
        let (ent, cast) = labo().await?;
        let m = monde(&ent, &cast).await;
        let adm = fiche(&ent, m.etab, cast.administratif.user_id, "administratif", true).await;
        let (jour,): (NaiveDate,) =
            sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())").bind(m.etab).fetch_one(ent.pool_verif()).await.unwrap();
        let mut w = W { ent, m, jour, adm, repli: Uuid::nil(), fa: Uuid::nil(), fb: Uuid::nil(), fc: Uuid::nil(), tom_user: cast.parent_tom.user_id };
        for qui in [w.m.prof, w.m.cpe, w.m.direction, w.m.compta, w.adm] {
            w.engager(&qui).await;
        }
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
        w.repli = w.ent.creer_file_for_actor(w.m.etab, dir.user, "direction", nf(code_aleatoire("rp"), TypeFile::Repli, None, "la direction")).await.unwrap();
        w.fa = w.ent.creer_file_for_actor(w.m.etab, dir.user, "direction", nf(code_aleatoire("fa"), TypeFile::Service, Some(w.repli), "le secrétariat des élèves")).await.unwrap();
        w.fb = w.ent.creer_file_for_actor(w.m.etab, dir.user, "direction", nf(code_aleatoire("fb"), TypeFile::Service, Some(w.repli), "la comptabilité")).await.unwrap();
        w.fc = w.ent.creer_file_for_actor(w.m.etab, dir.user, "direction", nf(code_aleatoire("fc"), TypeFile::Confidentielle, Some(w.repli), "la cellule RH")).await.unwrap();
        let (prof, cpe, adm, dir) = (w.m.prof, w.m.cpe, w.adm, w.m.direction);
        for (file, qui, resp, rang) in [
            (w.fa, prof, false, 1),
            (w.fa, cpe, true, 2),
            (w.fa, adm, false, 3),
            (w.fb, dir, true, 1),
            (w.fc, dir, true, 1),
            (w.repli, dir, true, 1),
        ] {
            let r = w
                .ent
                .ajouter_regle_membre_for_actor(
                    w.m.etab,
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
        for (code, file) in [
            ("maintenance.intervention", w.fa),
            ("secretariat.demande_interne", w.fa),
            ("scolarite.attestation", w.fa),
            ("rh.demande_agent", w.fc),
        ] {
            w.activer(code, file).await;
        }
        Some(w)
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

    async fn code_file(&self, id: Uuid) -> String {
        let (c,): (String,) = sqlx::query_as("SELECT code FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2")
            .bind(self.m.etab)
            .bind(id)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        c
    }

    async fn activer(&self, code: &str, file: Uuid) {
        let decl = nature(code).expect("nature");
        let fc = self.code_file(file).await;
        self.ent
            .regler_nature_for_actor(
                self.m.etab,
                self.m.direction.user,
                "direction",
                code,
                ReglageNature { actif: true, publics: decl.publics.to_vec(), file_defaut_code: Some(fc), ..Default::default() },
            )
            .await
            .unwrap_or_else(|e| panic!("activation de {code} : {e:?}"));
    }

    async fn src(&self, qui: &Acteur) -> SourcesFiles {
        self.ent.sources_demandes_for_actor(self.m.etab, qui.user, qui.role, None).await.expect("sources")
    }

    async fn deposer(&self, qui: &Acteur, d: DepotDemande) -> Result<EtatDemande, RefusDemande> {
        let s = self.ent.sources_demandes_for_actor(self.m.etab, qui.user, qui.role, Some(&d)).await.expect("sources");
        self.ent.deposer_demande_for_actor(self.m.etab, qui.user, qui.role, &s, d).await
    }

    /// Une demande de maintenance (C1) déposée par la comptabilité, routée vers la file A.
    async fn maint(&self) -> EtatDemande {
        self.deposer(&self.m.compta.clone(), depot_maintenance()).await.expect("dépôt")
    }

    async fn version(&self, id: Uuid) -> i32 {
        let (v,): (i32,) = sqlx::query_as("SELECT version FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
            .bind(self.m.etab)
            .bind(id)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        v
    }

    async fn ligne(&self, id: Uuid) -> (String, Uuid, Option<Uuid>, String) {
        sqlx::query_as("SELECT statut, file_id, attribuee_a_agent_id, nature_code FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
            .bind(self.m.etab)
            .bind(id)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap()
    }

    async fn prendre(&self, qui: &Acteur, id: Uuid) -> EtatDemande {
        let s = self.src(qui).await;
        let v = self.version(id).await;
        self.ent.agir_sur_demande_for_actor(self.m.etab, qui.user, qui.role, &s, id, v, Geste::Prendre).await.expect("prise")
    }

    async fn cloturer(&self, qui: &Acteur, id: Uuid) -> EtatDemande {
        let s = self.src(qui).await;
        let v = self.version(id).await;
        self.ent
            .agir_sur_demande_for_actor(self.m.etab, qui.user, qui.role, &s, id, v, Geste::Cloturer { issue: Issue::Traitee })
            .await
            .expect("clôture")
    }

    async fn lire(&self, qui: &Acteur, docs: &dyn PortDocuments, piece: Uuid) -> Result<LecturePiece, RefusDemande> {
        let s = self.src(qui).await;
        self.ent.lire_piece_for_actor(self.m.etab, qui.user, qui.role, &s, docs, piece).await
    }

    async fn faits(&self, id: Uuid) -> Vec<String> {
        let l: Vec<(String,)> = sqlx::query_as("SELECT type FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY rang")
            .bind(self.m.etab)
            .bind(id)
            .fetch_all(self.ent.pool_verif())
            .await
            .unwrap();
        l.into_iter().map(|(t,)| t).collect()
    }

    async fn compte(&self, table: &str, id: Uuid) -> i64 {
        let (n,): (i64,) = sqlx::query_as(&format!("SELECT count(*) FROM {table} WHERE etablissement_id = $1 AND demande_id = $2"))
            .bind(self.m.etab)
            .bind(id)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        n
    }

    async fn evenements(&self, id: Uuid) -> i64 {
        let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2")
            .bind(self.m.etab)
            .bind(id)
            .fetch_one(self.ent.pool_verif())
            .await
            .unwrap();
        n
    }

    async fn ecrire(&self, qui: &Acteur, docs: &dyn PortDocuments, id: Uuid, fil: Fil, texte: &str) -> Result<MessageEnvoye, RefusDemande> {
        let s = self.src(qui).await;
        self.ent
            .ecrire_message_for_actor(self.m.etab, qui.user, qui.role, &s, docs, id, NouveauMessage { fil, texte: texte.into(), pieces: vec![] })
            .await
    }

    async fn messages(&self, qui: &Acteur, id: Uuid) -> Result<Vec<MessageVu>, RefusDemande> {
        let s = self.src(qui).await;
        self.ent.messages_for_actor(self.m.etab, qui.user, qui.role, &s, id).await
    }

    async fn detail(&self, qui: &Acteur, id: Uuid) -> Result<DemandeTraitant, RefusDemande> {
        let s = self.src(qui).await;
        self.ent.demande_traitant_for_actor(self.m.etab, qui.user, qui.role, &s, RefDemande::Id(id)).await
    }

    async fn preuves(&self, qui: &Acteur, id: Uuid) -> Result<PreuvesDemandeur, RefusDemande> {
        let s = self.src(qui).await;
        self.ent.preuves_demandeur_for_actor(self.m.etab, qui.user, qui.role, &s, id).await
    }

    /// Reculer une échéance ouverte (le test ne dépend pas du jour réel : tout est relatif à `now()`).
    async fn reculer_echeance(&self, id: Uuid, type_: &str, jours: i32) {
        sqlx::query(
            "UPDATE ent_dem_echeance SET due_le = now() - make_interval(days => $4)
              WHERE etablissement_id = $1 AND demande_id = $2 AND type = $3 AND traitee_le IS NULL",
        )
        .bind(self.m.etab)
        .bind(id)
        .bind(type_)
        .bind(jours)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
    }

    async fn absent(&self, qui: &Acteur) {
        sqlx::query(
            "INSERT INTO ent_absp_absence (id, etablissement_id, agent_id, cause, etat, debut, fin, saisie_par_user_id, saisie_canal)
             VALUES (gen_random_uuid(), $1, $2, 'rh', 'validee', now() - interval '2 days', now() + interval '2 days', $3, 'agent')",
        )
        .bind(self.m.etab)
        .bind(qui.agent.unwrap())
        .bind(qui.user)
        .execute(self.ent.pool_verif())
        .await
        .unwrap();
    }
}

fn depot_rh() -> DepotDemande {
    let mut d = depot("rh.demande_agent");
    d.valeurs = vec![ValeurSaisie { champ: "type_demande".into(), rang: 0, valeur: Valeur::Code("autre".into()) }];
    d
}

fn msg(fil: Fil, texte: &str) -> NouveauMessage {
    NouveauMessage { fil, texte: texte.into(), pieces: vec![] }
}

// ───────────────────────── NJ-81 : une demande d'une autre file ne se détourne pas ─────────────────────────

#[tokio::test]
async fn t_nj81_detournement_autre_file() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let e = w.maint().await;
    let e = w.prendre(&w.m.prof.clone(), e.id).await;
    let (statut, file, porteur, _) = w.ligne(e.id).await;
    let faits = w.compte("ent_dem_fait", e.id).await;
    let evts = w.evenements(e.id).await;
    let v = w.version(e.id).await;
    // La direction est RESPONSABLE de la file B (et du repli) : elle ne voit pourtant pas la demande de la file A.
    let d = w.m.direction;
    let s = w.src(&d).await;
    let (etab, u, r) = (w.m.etab, d.user, "direction");
    let agent_adm = w.adm.agent.unwrap();
    let eg = &w.ent;
    assert_eq!(eg.reaffecter_for_actor(etab, u, r, &s, e.id, v, Cible::Agent(agent_adm), None).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.reaffecter_for_actor(etab, u, r, &s, e.id, v, Cible::File, None).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.deleguer_for_actor(etab, u, r, &s, e.id, v, d.agent.unwrap(), None).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.transferer_for_actor(etab, u, r, &s, e.id, v, w.fb, "mauvaise_nature").await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.requalifier_for_actor(etab, u, r, &s, e.id, v, "secretariat.demande_interne").await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.reprendre_pour_delegant_for_actor(etab, u, r, &s, e.id, v).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.designer_personne_visee_for_actor(etab, u, r, &s, e.id, v, w.m.prof.personne).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.repondre_resolu_for_actor(etab, u, r, &s, e.id, v, false).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.demander_complement_for_actor(etab, u, r, &s, e.id, v, "Il manque une photo").await.unwrap_err(), RefusDemande::Introuvable);
    let c = ConsultationNouvelle { agent_id: agent_adm, question: "Pouvez-vous voir ?".into(), delai_jours: 3, partage_demande: true, pieces_partagees: vec![] };
    assert_eq!(eg.consulter_for_actor(etab, u, r, &s, e.id, c).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.rattacher_conversation_for_actor(etab, u, r, &s, &Messagerie(true), e.id, Uuid::new_v4()).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.ajouter_abonne_for_actor(etab, u, r, &s, e.id, w.adm.personne, QualiteAbonne::CopieInterne).await.unwrap_err(), RefusDemande::Introuvable);
    // Réaffecter « les demandes de l'agent absent » ne ramasse pas les demandes d'une autre file (aucune prise ici).
    w.absent(&w.m.prof.clone()).await;
    assert_eq!(eg.reaffecter_absent_for_actor(etab, u, r, &s, w.m.prof.agent.unwrap(), Cible::File).await.unwrap_err(), RefusDemande::Conflit("Aucune demande à réaffecter."));
    assert!(eg.demandes_des_absents_for_actor(etab, u, r, &s).await.unwrap().is_empty());
    // Rien n'a bougé : ni file, ni porteur, ni état, ni version, ni fait, ni événement.
    assert_eq!(w.ligne(e.id).await, (statut, file, porteur, "maintenance.intervention".to_string()));
    assert_eq!(w.version(e.id).await, v);
    assert_eq!(w.compte("ent_dem_fait", e.id).await, faits);
    assert_eq!(w.evenements(e.id).await, evts);
    // Dans la file A, un membre simple ne réaffecte ni ne détourne la prise d'un collègue : « interdit », pas 404.
    let a = w.adm;
    let sa = w.src(&a).await;
    assert_eq!(eg.reaffecter_for_actor(etab, a.user, "administratif", &sa, e.id, v, Cible::Agent(a.agent.unwrap()), None).await.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(eg.deleguer_for_actor(etab, a.user, "administratif", &sa, e.id, v, a.agent.unwrap(), None).await.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(eg.transferer_for_actor(etab, a.user, "administratif", &sa, e.id, v, w.fb, "mauvaise_nature").await.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(eg.requalifier_for_actor(etab, a.user, "administratif", &sa, e.id, v, "secretariat.demande_interne").await.unwrap_err(), RefusDemande::Interdit);
    let c = ConsultationNouvelle { agent_id: d.agent.unwrap(), question: "Avis ?".into(), delai_jours: 2, partage_demande: false, pieces_partagees: vec![] };
    assert_eq!(eg.consulter_for_actor(etab, a.user, "administratif", &sa, e.id, c).await.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(w.ligne(e.id).await.2, porteur, "la prise n'a pas bougé");
    assert_eq!(eg.verifier_chaine_demande(etab, e.id).await.unwrap(), None);
}

#[tokio::test]
async fn t_nj81_transfert_et_prise_dans_la_file() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let e = w.maint().await;
    let (prof, cpe, dir) = (w.m.prof, w.m.cpe, w.m.direction);
    // La direction (file B) ne prend pas la demande de la file A, même « déjà prise » : 404.
    let sd = w.src(&dir).await;
    let r = w.ent.agir_sur_demande_for_actor(w.m.etab, dir.user, "direction", &sd, e.id, 1, Geste::Prendre).await;
    assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
    let e = w.prendre(&prof, e.id).await;
    let v = e.version;
    let sp = w.src(&prof).await;
    let (etab, eg) = (w.m.etab, &w.ent);
    // Motif obligatoire et fermé ; file inconnue ; même file ; ni confusion ni détournement.
    assert!(matches!(eg.transferer_for_actor(etab, prof.user, "prof", &sp, e.id, v, w.fb, "pour_voir").await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.transferer_for_actor(etab, prof.user, "prof", &sp, e.id, v, Uuid::new_v4(), "mauvaise_nature").await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.transferer_for_actor(etab, prof.user, "prof", &sp, e.id, v, w.fa, "mauvaise_nature").await, Err(RefusDemande::Invalide(_))));
    // Version périmée : 409.
    assert!(matches!(eg.transferer_for_actor(etab, prof.user, "prof", &sp, e.id, v + 7, w.fb, "mauvaise_nature").await, Err(RefusDemande::Conflit(_))));
    // Le porteur transfère vers la file B : plus de prise, « reçue », nouvelle échéance de prise en charge.
    let t = eg.transferer_for_actor(etab, prof.user, "prof", &sp, e.id, v, w.fb, "releve_d_un_autre_service").await.unwrap();
    assert_eq!(t.statut, "deposee");
    assert_eq!(w.ligne(e.id).await.1, w.fb);
    assert_eq!(w.ligne(e.id).await.2, None);
    let (ouvertes,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'prise_en_charge' AND traitee_le IS NULL",
    )
    .bind(etab)
    .bind(e.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert_eq!(ouvertes, 1, "le délai de prise en charge repart pour la nouvelle file");
    assert!(w.faits(e.id).await.contains(&"transferee".to_string()));
    // L'ancienne file ne voit plus la demande (404), la nouvelle la voit.
    for q in [&prof, &cpe] {
        assert_eq!(w.detail(q, e.id).await.unwrap_err(), RefusDemande::Introuvable, "{} lit encore la demande", q.role);
        assert_eq!(w.messages(q, e.id).await.unwrap_err(), RefusDemande::Introuvable);
    }
    assert!(w.detail(&dir, e.id).await.is_ok());
    // Le demandeur, lui, la voit toujours (même demande, même numéro) et son service a changé.
    let p = w.preuves(&w.m.compta.clone(), e.id).await.unwrap();
    assert_eq!(p.envoyee.role.as_deref(), Some("la comptabilité"));
    // La file cible sans destinataire possible est refusée (tout le monde y est visé) : voir t_designation_personne_visee.
    assert_eq!(eg.verifier_chaine_demande(etab, e.id).await.unwrap(), None);
}

// ───────────────────────── NJ-83 : les pièces ne se lisent que dans son périmètre ─────────────────────────

#[tokio::test]
async fn t_nj83_pieces_cloisonnees() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof, cpe, dir) = (w.m.compta, w.m.prof, w.m.cpe, w.m.direction);
    let e = w.maint().await;
    let e2 = w.maint().await; // une autre demande, même file : sert de contrôle croisé
    let mut docs = Docs::default();
    let (etab, eg) = (w.m.etab, &w.ent);
    let sc = w.src(&compta).await;
    let sp = w.src(&prof).await;
    let sd = w.src(&dir).await;
    // Le demandeur joint un justificatif qu'il a déposé ; jamais le document d'un autre (rattachement forcé refusé).
    let doc_c = docs.doc(&compta);
    let doc_p = docs.doc(&prof);
    let pc = eg
        .joindre_piece_for_actor(etab, compta.user, "compta", &sc, &docs, e.id, PieceAJoindre { document_id: doc_c, role: RolePiece::Justificatif, au_demandeur: false })
        .await
        .expect("pièce du demandeur");
    let vole = eg
        .joindre_piece_for_actor(etab, compta.user, "compta", &sc, &docs, e.id, PieceAJoindre { document_id: doc_p, role: RolePiece::Justificatif, au_demandeur: false })
        .await;
    assert!(matches!(vole, Err(RefusDemande::Invalide(_))), "le document d'un autre ne se rattache pas");
    // Un rôle qui n'est pas de son camp est refusé (le demandeur ne produit pas la « réponse »).
    let faux = eg
        .joindre_piece_for_actor(etab, compta.user, "compta", &sc, &docs, e.id, PieceAJoindre { document_id: doc_c, role: RolePiece::Reponse, au_demandeur: false })
        .await;
    assert!(matches!(faux, Err(RefusDemande::Invalide(_))));
    // Sans socle 7, aucune pièce n'est acceptée (jamais de repli vers un stockage direct).
    let sans = eg
        .joindre_piece_for_actor(etab, compta.user, "compta", &sc, &DocumentsAbsents, e.id, PieceAJoindre { document_id: doc_c, role: RolePiece::Annexe, au_demandeur: false })
        .await;
    assert!(matches!(sans, Err(RefusDemande::Invalide(_))));
    // Le service pose une pièce interne et une réponse adressée au demandeur.
    let doc_i = docs.doc(&prof);
    let doc_r = docs.doc(&prof);
    let pi = eg
        .joindre_piece_for_actor(etab, prof.user, "prof", &sp, &docs, e.id, PieceAJoindre { document_id: doc_i, role: RolePiece::Annexe, au_demandeur: false })
        .await
        .unwrap();
    let pr = eg
        .joindre_piece_for_actor(etab, prof.user, "prof", &sp, &docs, e.id, PieceAJoindre { document_id: doc_r, role: RolePiece::Reponse, au_demandeur: true })
        .await
        .unwrap();
    // Le demandeur lit ses pièces et la réponse, pas la pièce interne (elle n'existe pas pour lui : 404).
    macro_rules! lire {
        ($q:expr, $docs:expr, $p:expr) => {
            w.lire($q, $docs, $p)
        };
    }
    assert!(lire!(&compta, &docs, pc).await.unwrap().jeton.starts_with("jeton-court-"));
    assert!(lire!(&compta, &docs, pr).await.is_ok());
    assert_eq!(lire!(&compta, &docs, pi).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.pieces_for_actor(etab, compta.user, "compta", &sc, e.id).await.unwrap().len(), 2);
    assert_eq!(eg.pieces_for_actor(etab, prof.user, "prof", &sp, e.id).await.unwrap().len(), 3);
    // Le socle 7 absent : pas de jeton, donc pas de lecture ; les identifiants de document ne sortent jamais.
    let mut tout = serde_json::to_string(&eg.pieces_for_actor(etab, prof.user, "prof", &sp, e.id).await.unwrap()).unwrap();
    tout.push_str(&serde_json::to_string(&lire!(&compta, &docs, pc).await.unwrap()).unwrap());
    for doc in [doc_c, doc_p, doc_i, doc_r] {
        assert!(!tout.contains(&doc.to_string()), "l'identifiant d'un document du socle 7 est sorti");
    }
    assert_eq!(eg.lire_piece_for_actor(etab, compta.user, "compta", &sc, &DocumentsAbsents, pc).await.unwrap_err(), RefusDemande::Indisponible);
    // NJ-83 : la direction (file B) ne lit AUCUNE pièce de la file A ; ni le CPE d'une autre file non plus.
    for p in [pc, pi, pr] {
        assert_eq!(lire!(&dir, &docs, p).await.unwrap_err(), RefusDemande::Introuvable, "la direction lit une pièce d'une autre file");
    }
    assert_eq!(eg.pieces_for_actor(etab, dir.user, "direction", &sd, e.id).await.unwrap_err(), RefusDemande::Introuvable);
    // Le demandeur d'UNE AUTRE demande ne lit pas les pièces de celle-ci (autre demandeur, même file).
    let parent = w.m.parent;
    assert_eq!(lire!(&parent, &docs, pc).await.unwrap_err(), RefusDemande::Introuvable);
    // Un membre de la file A lit les pièces de la demande 1 et pas une pièce inexistante.
    assert!(lire!(&cpe, &docs, pi).await.is_ok());
    assert_eq!(lire!(&cpe, &docs, Uuid::new_v4()).await.unwrap_err(), RefusDemande::Introuvable);
    // Le retrait : seul le déposant retire sa pièce, tant que la demande n'est pas prise ; la trace reste.
    assert_eq!(eg.retirer_piece_for_actor(etab, cpe.user, "cpe", &w.src(&cpe).await, pc).await.unwrap_err(), RefusDemande::Interdit);
    eg.retirer_piece_for_actor(etab, compta.user, "compta", &sc, pc).await.unwrap();
    assert_eq!(lire!(&compta, &docs, pc).await.unwrap_err(), RefusDemande::Introuvable, "une pièce retirée ne se lit plus");
    assert!(matches!(eg.retirer_piece_for_actor(etab, compta.user, "compta", &sc, pc).await, Err(RefusDemande::Conflit(_))));
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_piece WHERE etablissement_id = $1 AND demande_id = $2")
        .bind(etab)
        .bind(e.id)
        .fetch_one(eg.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 3, "la trace d'une pièce retirée reste en base");
    // 12 pièces au plus par demande.
    for _ in 0..12 {
        let d = docs.doc(&prof);
        let r = eg
            .joindre_piece_for_actor(etab, prof.user, "prof", &sp, &docs, e2.id, PieceAJoindre { document_id: d, role: RolePiece::Annexe, au_demandeur: false })
            .await;
        assert!(r.is_ok());
    }
    let d = docs.doc(&prof);
    let r = eg
        .joindre_piece_for_actor(etab, prof.user, "prof", &sp, &docs, e2.id, PieceAJoindre { document_id: d, role: RolePiece::Annexe, au_demandeur: false })
        .await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))), "la 13e pièce est refusée");
    assert_eq!(eg.verifier_chaine_demande(etab, e.id).await.unwrap(), None);
}

// ───────────────────────── NJ-80 étendu : échanges et commentaires ─────────────────────────

#[tokio::test]
async fn t_nj80_echanges_sans_droit_refuse() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, dir, parent) = (w.m.prof, w.m.direction, w.m.parent);
    let e = w.maint().await;
    let docs = Docs::default();
    let m0 = w.ecrire(&prof, &docs, e.id, Fil::Interne, "Note interne de départ").await.unwrap();
    let (etab, eg) = (w.m.etab, &w.ent);
    let (faits, msgs, pieces, abonnes) = (
        w.compte("ent_dem_fait", e.id).await,
        w.compte("ent_dem_message", e.id).await,
        w.compte("ent_dem_piece", e.id).await,
        w.compte("ent_dem_abonne", e.id).await,
    );
    let evts = w.evenements(e.id).await;
    // Ni la direction (autre file), ni un parent, ni un compte sans fiche ne lit ni n'écrit dans le fil, par aucun
    // des trois fils, ni ne corrige, masque, joint, abonne : 404 neutre partout (NJ-80).
    let inconnu = Acteur { user: Uuid::new_v4(), personne: Uuid::nil(), agent: None, role: "administratif" };
    for q in [&dir, &parent, &inconnu] {
        let s = w.src(q).await;
        let cle = format!("{} {:?}", q.role, q.user);
        for fil in [Fil::Demandeur, Fil::Interne] {
            let r = w.ecrire(q, &docs, e.id, fil, "Je m'invite").await;
            assert_eq!(r.unwrap_err(), RefusDemande::Introuvable, "{cle} écrit dans {fil:?}");
        }
        assert_eq!(w.messages(q, e.id).await.unwrap_err(), RefusDemande::Introuvable, "{cle} lit le fil");
        assert_eq!(eg.corriger_message_for_actor(etab, q.user, q.role, &s, m0.id, "Correction").await.unwrap_err(), RefusDemande::Introuvable, "{cle}");
        assert_eq!(eg.masquer_message_for_actor(etab, q.user, q.role, &s, m0.id).await.unwrap_err(), RefusDemande::Introuvable, "{cle}");
        assert_eq!(eg.pieces_for_actor(etab, q.user, q.role, &s, e.id).await.unwrap_err(), RefusDemande::Introuvable, "{cle}");
        let p = PieceAJoindre { document_id: Uuid::new_v4(), role: RolePiece::Annexe, au_demandeur: true };
        assert_eq!(eg.joindre_piece_for_actor(etab, q.user, q.role, &s, &docs, e.id, p).await.unwrap_err(), RefusDemande::Introuvable, "{cle}");
        assert_eq!(eg.ajouter_abonne_for_actor(etab, q.user, q.role, &s, e.id, prof.personne, QualiteAbonne::CopieInterne).await.unwrap_err(), RefusDemande::Introuvable, "{cle}");
        assert_eq!(eg.preuves_demandeur_for_actor(etab, q.user, q.role, &s, e.id).await.unwrap_err(), RefusDemande::Introuvable, "{cle}");
    }
    // Un parent ne s'abonne pas en « appui » à une panne (il n'est pas du personnel) : 404.
    assert_eq!(eg.appuyer_for_actor(etab, parent.user, "parent", e.id).await.unwrap_err(), RefusDemande::Introuvable);
    // Rien n'a été écrit, aucun événement n'est parti.
    assert_eq!(w.compte("ent_dem_fait", e.id).await, faits);
    assert_eq!(w.compte("ent_dem_message", e.id).await, msgs);
    assert_eq!(w.compte("ent_dem_piece", e.id).await, pieces);
    assert_eq!(w.compte("ent_dem_abonne", e.id).await, abonnes);
    assert_eq!(w.evenements(e.id).await, evts);
}

// ───────────────────────── Échanges : visibilités, correction, masquage ─────────────────────────

#[tokio::test]
async fn t_echanges_visibilites_correction_masquage() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof, cpe) = (w.m.compta, w.m.prof, w.m.cpe);
    let e = w.maint().await;
    let docs = Docs::default();
    let (etab, eg) = (w.m.etab, &w.ent);
    let au_demandeur = w.ecrire(&prof, &docs, e.id, Fil::Demandeur, "Bonjour, nous passons demain.").await.unwrap();
    let interne = w.ecrire(&prof, &docs, e.id, Fil::Interne, "Note interne : attention, famille difficile.").await.unwrap();
    let reponse = w.ecrire(&compta, &docs, e.id, Fil::Demandeur, "Merci, c'est le couloir du 2e.").await.unwrap();
    // Le demandeur ne lit JAMAIS la note interne (NJ-82), ni dans le texte ni dans le compte.
    let vu = w.messages(&compta, e.id).await.unwrap();
    assert_eq!(vu.len(), 2);
    assert!(vu.iter().all(|m| m.fil == "demandeur"));
    let brut = serde_json::to_string(&vu).unwrap();
    assert!(!brut.contains("famille difficile"));
    // Son message est signé « Vous » ; celui du service porte le rôle de la file et le prénom + initiale.
    assert!(vu.iter().any(|m| m.auteur == "Vous" && !m.du_service));
    let signature = vu.iter().find(|m| m.du_service).unwrap().auteur.clone();
    assert!(signature.starts_with("le secrétariat des élèves — "), "signature : {signature}");
    // Le service lit les trois et sait distinguer la note (par un libellé, pas seulement par la couleur côté écran).
    let svc = w.messages(&cpe, e.id).await.unwrap();
    assert_eq!(svc.len(), 3);
    assert_eq!(svc.iter().filter(|m| m.fil == "interne").count(), 1);
    assert!(svc.iter().any(|m| m.auteur == "Demandeur" && !m.du_service));
    // Un message est limité et non vide ; un caractère nul est refusé.
    assert!(matches!(w.ecrire(&prof, &docs, e.id, Fil::Demandeur, "   ").await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(w.ecrire(&prof, &docs, e.id, Fil::Demandeur, &"x".repeat(8001)).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(w.ecrire(&prof, &docs, e.id, Fil::Demandeur, "a\0b").await, Err(RefusDemande::Invalide(_))));
    // Un demandeur n'écrit pas dans le fil interne ; le service n'écrit pas dans le fil d'un consulté inexistant.
    assert_eq!(w.ecrire(&compta, &docs, e.id, Fil::Interne, "interne ?").await.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(w.ecrire(&prof, &docs, e.id, Fil::Consultation(Uuid::new_v4()), "?").await.unwrap_err(), RefusDemande::Introuvable);
    // Correction : un NOUVEAU message ; l'original ne change pas ; seul son auteur corrige ; même fil.
    let sp = w.src(&prof).await;
    let sc = w.src(&compta).await;
    let sx = w.src(&cpe).await;
    let corr = eg.corriger_message_for_actor(etab, prof.user, "prof", &sp, au_demandeur.id, "Correction : nous passons après-demain.").await.unwrap();
    assert_ne!(corr.id, au_demandeur.id);
    let (genre, visibilite, corrige): (String, String, Option<Uuid>) =
        sqlx::query_as("SELECT genre, visibilite, corrige_message_id FROM ent_dem_message WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(corr.id)
            .fetch_one(eg.pool_verif())
            .await
            .unwrap();
    assert_eq!((genre.as_str(), visibilite.as_str(), corrige), ("correction", "demandeur", Some(au_demandeur.id)));
    assert_eq!(eg.corriger_message_for_actor(etab, cpe.user, "cpe", &sx, au_demandeur.id, "Je corrige à sa place").await.unwrap_err(), RefusDemande::Interdit);
    // Aucune modification ni suppression d'un message envoyé, même par la base (NJ-89) ; seul le masquage s'écrit.
    let r = sqlx::query("UPDATE ent_dem_message SET texte = 'réécrit' WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(au_demandeur.id).execute(eg.pool_verif()).await;
    assert!(r.is_err(), "un message envoyé a été réécrit");
    let r = sqlx::query("DELETE FROM ent_dem_message WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(au_demandeur.id).execute(eg.pool_verif()).await;
    assert!(r.is_err(), "un message a été supprimé");
    // Masquer : seule une note interne, par son auteur, dans les 10 minutes ; le fait reste.
    assert_eq!(eg.masquer_message_for_actor(etab, prof.user, "prof", &sp, au_demandeur.id).await.unwrap_err(), RefusDemande::Interdit, "un message au demandeur ne se masque pas");
    assert_eq!(eg.masquer_message_for_actor(etab, cpe.user, "cpe", &sx, interne.id).await.unwrap_err(), RefusDemande::Interdit, "la note d'un autre ne se masque pas");
    assert_eq!(eg.masquer_message_for_actor(etab, compta.user, "compta", &sc, interne.id).await.unwrap_err(), RefusDemande::Interdit);
    eg.masquer_message_for_actor(etab, prof.user, "prof", &sp, interne.id).await.unwrap();
    assert!(matches!(eg.masquer_message_for_actor(etab, prof.user, "prof", &sp, interne.id).await, Err(RefusDemande::Conflit(_))));
    let svc = w.messages(&cpe, e.id).await.unwrap();
    let masque = svc.iter().find(|m| m.id == interne.id).unwrap();
    assert!(masque.masque && masque.texte.is_none(), "la note masquée n'a plus de texte");
    assert!(w.faits(e.id).await.contains(&"message_masque".to_string()));
    // Passé 10 minutes : refus. (Un message ancien est inséré directement : la base accepte l'insertion.)
    let vieux = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_dem_message (id, etablissement_id, demande_id, auteur_personne_id, visibilite, texte, cree_le)
         VALUES ($1, $2, $3, $4, 'interne', 'vieille note', now() - interval '11 minutes')",
    )
    .bind(vieux)
    .bind(etab)
    .bind(e.id)
    .bind(prof.personne)
    .execute(eg.pool_verif())
    .await
    .unwrap();
    assert!(matches!(eg.masquer_message_for_actor(etab, prof.user, "prof", &sp, vieux).await, Err(RefusDemande::Conflit(_))));
    // Les messages ne portent aucun nom de fichier, aucune adresse : l'enveloppe des événements est neutre (NJ-86).
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2 AND payload::text LIKE '%Merci%'",
    )
    .bind(etab)
    .bind(e.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 0, "le texte d'un message est sorti dans un événement");
    assert_eq!(eg.verifier_chaine_demande(etab, e.id).await.unwrap(), None);
}

// ───────────────────────── Complément demandé, décompte suspendu puis repris ─────────────────────────

#[tokio::test]
async fn t_complement_suspend_et_reprise() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof) = (w.m.compta, w.m.prof);
    let e = w.maint().await;
    let e = w.prendre(&prof, e.id).await;
    let (etab, eg) = (w.m.etab, &w.ent);
    let sp = w.src(&prof).await;
    let sc = w.src(&compta).await;
    // Le demandeur ne demande pas de complément à sa propre demande.
    assert_eq!(eg.demander_complement_for_actor(etab, compta.user, "compta", &sc, e.id, e.version, "Je complète").await.unwrap_err(), RefusDemande::Interdit);
    assert!(matches!(eg.demander_complement_for_actor(etab, prof.user, "prof", &sp, e.id, e.version + 5, "Photo ?").await, Err(RefusDemande::Conflit(_))));
    let c = eg.demander_complement_for_actor(etab, prof.user, "prof", &sp, e.id, e.version, "Il manque une photo du plafond.").await.unwrap();
    assert_eq!(c.statut, "en_attente_demandeur");
    // Le demandeur voit « on attend votre réponse » ; le décompte du traitement est suspendu.
    let p = w.preuves(&compta, e.id).await.unwrap();
    assert_eq!(p.etat, EtatSimple::AttenteReponse);
    assert_eq!(p.etat_libelle, "on attend votre réponse");
    let (suspendues,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'traitement'
            AND traitee_le IS NULL AND suspendue_depuis IS NOT NULL",
    )
    .bind(etab)
    .bind(e.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert_eq!(suspendues, 1);
    // Une seule demande de complément à la fois ; pas de retard compté pendant l'attente.
    assert!(matches!(eg.demander_complement_for_actor(etab, prof.user, "prof", &sp, e.id, c.version, "Encore ?").await, Err(RefusDemande::Conflit(_))));
    w.reculer_echeance(e.id, "traitement", 0).await;
    assert!(w.preuves(&compta, e.id).await.unwrap().retard.is_none());
    assert_eq!(eg.signaler_retards(etab).await.unwrap().signalees, 0, "une attente du demandeur ne fait pas de retard");
    // Le demandeur répond : la demande reprend (prise), le décompte repart d'où il s'était arrêté.
    let avant: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
        "SELECT due_le FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'traitement' AND traitee_le IS NULL",
    )
    .bind(etab)
    .bind(e.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    let r = w.ecrire(&compta, &Docs::default(), e.id, Fil::Demandeur, "Voici : la photo est jointe.").await.unwrap();
    assert_eq!(r.statut, "prise");
    let apres: (chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>) = sqlx::query_as(
        "SELECT due_le, suspendue_depuis FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'traitement' AND traitee_le IS NULL",
    )
    .bind(etab)
    .bind(e.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert!(apres.0 > avant.0, "l'échéance est reportée de la durée d'attente");
    assert!(apres.1.is_none(), "le décompte a repris");
    // Faits visibles du demandeur : complément demandé puis reçu.
    let h = eg.historique_demandeur_for_actor(etab, compta.user, e.id).await.unwrap();
    let types: Vec<&str> = h.iter().map(|x| x.etape.as_str()).collect();
    assert!(types.contains(&"complement_demande") && types.contains(&"complement_recu"), "{types:?}");
    assert_eq!(eg.verifier_chaine_demande(etab, e.id).await.unwrap(), None);
}

// ───────────────────────── Les trois preuves visibles du demandeur ─────────────────────────

#[tokio::test]
async fn t_trois_preuves_visibles() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof, cpe, dir) = (w.m.compta, w.m.prof, w.m.cpe, w.m.direction);
    let e = w.maint().await;
    let (etab, eg) = (w.m.etab, &w.ent);
    // 1. ENVOYÉE : le service, son RÔLE (jamais un nom avant la prise), un délai indicatif.
    let p = w.preuves(&compta, e.id).await.unwrap();
    assert_eq!(p.etat, EtatSimple::Envoyee);
    assert_eq!(p.etat_libelle, "envoyée");
    assert_eq!(p.envoyee.role.as_deref(), Some("le secrétariat des élèves"));
    assert!(p.envoyee.delai_indicatif_le.is_some());
    assert!(p.vue.is_none() && p.prise.is_none() && p.retard.is_none());
    // 2. VUE : ni la liste de la file, ni un consulté, ni la direction d'une autre file ne la posent (CDM-82).
    let s = w.src(&prof).await;
    assert!(eg.ma_file_for_actor(etab, prof.user, "prof", &s).await.unwrap().iter().any(|d| d.id == e.id));
    assert_eq!(w.detail(&dir, e.id).await.unwrap_err(), RefusDemande::Introuvable);
    assert!(!w.faits(e.id).await.contains(&"vue_par_service".to_string()));
    assert!(w.preuves(&compta, e.id).await.unwrap().vue.is_none());
    // La première ouverture du détail par un membre la pose, une seule fois ; une seconde ouverture n'ajoute rien.
    let evts = w.evenements(e.id).await;
    w.detail(&prof, e.id).await.unwrap();
    w.detail(&cpe, e.id).await.unwrap();
    w.detail(&prof, e.id).await.unwrap();
    assert_eq!(w.faits(e.id).await.iter().filter(|t| *t == "vue_par_service").count(), 1);
    assert_eq!(w.evenements(e.id).await, evts + 1, "un seul avis « vue »");
    let p = w.preuves(&compta, e.id).await.unwrap();
    assert_eq!(p.etat, EtatSimple::Vue);
    assert_eq!(p.etat_libelle, "vue par le secrétariat des élèves");
    assert!(p.vue.is_some() && p.prise.is_none());
    // 3. PRISE : prénom et initiale de la personne, dès la prise.
    w.prendre(&prof, e.id).await;
    let (prenom, nom): (Option<String>, String) =
        sqlx::query_as("SELECT prenom, nom FROM ent_personne WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(prof.personne).fetch_one(eg.pool_verif()).await.unwrap();
    let attendu = format!("{} {}.", prenom.unwrap_or_default().trim(), nom.trim().chars().next().unwrap().to_uppercase());
    let p = w.preuves(&compta, e.id).await.unwrap();
    assert_eq!(p.etat, EtatSimple::EnCours);
    let prise = p.prise.clone().expect("prise visible");
    assert_eq!((prise.par.as_str(), prise.nominatif), (attendu.trim_start(), true));
    // Aucun identifiant de file, d'agent ni de règle ne sort des preuves (NJ-82).
    let json = serde_json::to_string(&p).unwrap();
    for interdit in [w.fa.to_string(), prof.agent.unwrap().to_string(), prof.personne.to_string()] {
        assert!(!json.contains(&interdit), "identifiant interne dans les preuves : {interdit}");
    }
    // « Service seul » (CDM-84) : réglage par file, sans justification ; le nom disparaît, le service reste.
    sqlx::query("UPDATE ent_dem_file SET affichage_traitant = 'service_seul' WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(w.fa).execute(eg.pool_verif()).await.unwrap();
    let p = w.preuves(&compta, e.id).await.unwrap();
    let prise = p.prise.unwrap();
    assert_eq!((prise.par.as_str(), prise.nominatif), ("le secrétariat des élèves", false));
    w.ecrire(&prof, &Docs::default(), e.id, Fil::Demandeur, "Bonjour").await.unwrap();
    let vu = w.messages(&compta, e.id).await.unwrap();
    let signature = vu.iter().find(|m| m.du_service).unwrap().auteur.clone();
    assert_eq!(signature, "le secrétariat des élèves", "signature nominative malgré « service seul »");
    // Seuls le demandeur, son déposant et ses co-demandeurs lisent les preuves : un membre de la file, non.
    assert_eq!(w.preuves(&prof, e.id).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(w.preuves(&dir, e.id).await.unwrap_err(), RefusDemande::Introuvable);
    // Une nature confidentielle ne dévoile ni le nom de la cellule ni celui de la personne.
    let rh = w.deposer(&compta, depot_rh()).await.unwrap();
    let p = w.preuves(&compta, rh.id).await.unwrap();
    assert_eq!(p.envoyee.service, "l'établissement");
    assert!(p.envoyee.role.is_none());
    w.detail(&dir, rh.id).await.unwrap();
    w.prendre(&dir, rh.id).await;
    let p = w.preuves(&compta, rh.id).await.unwrap();
    assert_eq!(p.prise.unwrap().par, "l'établissement");
}

// ───────────────────────── Un retard est montré, le responsable est prévenu ─────────────────────────

#[tokio::test]
async fn t_retard_montre_et_responsable_prevenu() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof) = (w.m.compta, w.m.prof);
    let e = w.maint().await;
    let (etab, eg) = (w.m.etab, &w.ent);
    assert!(w.preuves(&compta, e.id).await.unwrap().retard.is_none(), "pas de retard le jour du dépôt");
    assert_eq!(eg.signaler_retards(etab).await.unwrap().signalees, 0);
    // L'échéance de prise en charge dépasse de 3 jours : le demandeur le voit AVANT tout passage de l'ordonnanceur.
    w.reculer_echeance(e.id, "prise_en_charge", 3).await;
    let r = w.preuves(&compta, e.id).await.unwrap().retard.expect("retard montré");
    assert_eq!(r.jours, 3);
    assert!(!r.responsable_prevenu, "le responsable n'est pas encore prévenu");
    let evts = w.evenements(e.id).await;
    let rap = eg.signaler_retards(etab).await.unwrap();
    assert_eq!(rap.signalees, 1);
    assert_eq!(w.evenements(e.id).await, evts + 1, "un avis à enveloppe neutre part au responsable");
    let r = w.preuves(&compta, e.id).await.unwrap().retard.unwrap();
    assert!(r.responsable_prevenu, "le demandeur voit que le responsable est prévenu");
    // L'ordonnanceur est rejouable : aucun doublon, aucun nouvel avis.
    assert_eq!(eg.signaler_retards(etab).await.unwrap().signalees, 0);
    assert_eq!(w.faits(e.id).await.iter().filter(|t| *t == "retard_signale").count(), 1);
    // Une fois prise, c'est le retard de traitement qui compte ; l'ancien retard de prise ne se montre plus.
    w.prendre(&prof, e.id).await;
    assert!(w.preuves(&compta, e.id).await.unwrap().retard.is_none());
    w.reculer_echeance(e.id, "traitement", 1).await;
    let r = w.preuves(&compta, e.id).await.unwrap().retard.unwrap();
    assert_eq!(r.jours, 1);
    assert!(!r.responsable_prevenu);
    assert_eq!(eg.signaler_retards(etab).await.unwrap().signalees, 1);
    assert!(w.preuves(&compta, e.id).await.unwrap().retard.unwrap().responsable_prevenu);
    // Le fait est posé par la tâche déclarée de l'ordonnanceur, jamais par une personne ni par l'IA.
    let (taches,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'retard_signale'
            AND auteur_tache = 'ordonnanceur_echeances' AND auteur_personne_id IS NULL",
    )
    .bind(etab)
    .bind(e.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert_eq!(taches, 2);
    assert_eq!(eg.verifier_chaine_demande(etab, e.id).await.unwrap(), None);
}

// ───────────────────────── Réaffectation en un geste, jamais silencieuse ─────────────────────────

#[tokio::test]
async fn t_reaffectation_un_geste_absent() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof, cpe, adm, dir) = (w.m.compta, w.m.prof, w.m.cpe, w.adm, w.m.direction);
    let (etab, eg) = (w.m.etab, &w.ent);
    let mut ids = Vec::new();
    for _ in 0..3 {
        let e = w.maint().await;
        ids.push(w.prendre(&prof, e.id).await.id);
    }
    let autre = w.maint().await; // non prise : ne bouge pas
    let sc = w.src(&cpe).await;
    let sp = w.src(&prof).await;
    // Rien à signaler tant que le porteur n'est pas absent.
    assert!(eg.demandes_des_absents_for_actor(etab, cpe.user, "cpe", &sc).await.unwrap().is_empty());
    assert_eq!(eg.reaffecter_absent_for_actor(etab, cpe.user, "cpe", &sc, prof.agent.unwrap(), Cible::File).await.unwrap_err(), RefusDemande::Conflit("Cet agent n'est pas signalé absent."));
    // Une absence PASSÉE ne rend pas absent aujourd'hui.
    sqlx::query(
        "INSERT INTO ent_absp_absence (id, etablissement_id, agent_id, cause, etat, debut, fin, saisie_par_user_id, saisie_canal)
         VALUES (gen_random_uuid(), $1, $2, 'rh', 'validee', now() - interval '20 days', now() - interval '10 days', $3, 'agent')",
    )
    .bind(etab)
    .bind(prof.agent.unwrap())
    .bind(prof.user)
    .execute(eg.pool_verif())
    .await
    .unwrap();
    assert!(eg.demandes_des_absents_for_actor(etab, cpe.user, "cpe", &sc).await.unwrap().is_empty());
    // Une absence seulement SIGNALÉE (non décidée) non plus.
    sqlx::query(
        "INSERT INTO ent_absp_absence (id, etablissement_id, agent_id, cause, etat, debut, fin, saisie_par_user_id, saisie_canal)
         VALUES (gen_random_uuid(), $1, $2, 'rh', 'signalee', now() - interval '1 day', now() + interval '1 day', $3, 'agent')",
    )
    .bind(etab)
    .bind(prof.agent.unwrap())
    .bind(prof.user)
    .execute(eg.pool_verif())
    .await
    .unwrap();
    assert!(eg.demandes_des_absents_for_actor(etab, cpe.user, "cpe", &sc).await.unwrap().is_empty());
    // Absence décidée : le responsable voit les demandes de l'absent, avec la date de retour et sans motif.
    w.absent(&prof).await;
    let l = eg.demandes_des_absents_for_actor(etab, cpe.user, "cpe", &sc).await.unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!((l[0].agent_id, l[0].nb_demandes), (prof.agent.unwrap(), 3));
    assert!(l[0].absent_jusqu_au >= w.jour);
    // Un membre qui n'est pas responsable ne voit rien et ne réaffecte pas (aucune file dirigée).
    let sa = w.src(&adm).await;
    assert!(eg.demandes_des_absents_for_actor(etab, adm.user, "administratif", &sa).await.unwrap().is_empty());
    assert!(matches!(eg.reaffecter_absent_for_actor(etab, adm.user, "administratif", &sa, prof.agent.unwrap(), Cible::File).await, Err(RefusDemande::Conflit(_))));
    // Cible invalide : une personne absente, un non-membre : refusé, et rien ne bouge (transaction).
    assert!(matches!(eg.reaffecter_absent_for_actor(etab, cpe.user, "cpe", &sc, prof.agent.unwrap(), Cible::Agent(prof.agent.unwrap())).await, Err(RefusDemande::Conflit(_))));
    assert!(matches!(eg.reaffecter_absent_for_actor(etab, cpe.user, "cpe", &sc, prof.agent.unwrap(), Cible::Agent(dir.agent.unwrap())).await, Err(RefusDemande::Invalide(_))));
    for id in &ids {
        assert_eq!(w.ligne(*id).await.2, prof.agent);
    }
    // EN UN GESTE : les trois demandes passent à un membre ; chacune a son fait et son avis (jamais silencieux).
    let evts: Vec<i64> = {
        let mut v = Vec::new();
        for id in &ids {
            v.push(w.evenements(*id).await);
        }
        v
    };
    let n = eg.reaffecter_absent_for_actor(etab, cpe.user, "cpe", &sc, prof.agent.unwrap(), Cible::Agent(adm.agent.unwrap())).await.unwrap();
    assert_eq!(n, 3);
    for (i, id) in ids.iter().enumerate() {
        let (statut, _, porteur, _) = w.ligne(*id).await;
        assert_eq!((statut.as_str(), porteur), ("prise", adm.agent));
        assert!(w.faits(*id).await.contains(&"reaffectee".to_string()));
        assert_eq!(w.evenements(*id).await, evts[i] + 1);
        let (av, ap, motif): (Option<Uuid>, Option<Uuid>, Option<String>) = sqlx::query_as(
            "SELECT agent_avant_id, agent_apres_id, motif_code FROM ent_dem_fait
              WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'reaffectee'",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(eg.pool_verif())
        .await
        .unwrap();
        assert_eq!((av, ap, motif.as_deref()), (prof.agent, adm.agent, Some("absence")));
        assert_eq!(eg.verifier_chaine_demande(etab, *id).await.unwrap(), None);
        // Le demandeur voit le NOUVEAU prénom (CDM-83).
        let p = w.preuves(&compta, *id).await.unwrap();
        let (prenom,): (Option<String>,) =
            sqlx::query_as("SELECT prenom FROM ent_personne WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(adm.personne).fetch_one(eg.pool_verif()).await.unwrap();
        assert!(p.prise.unwrap().par.starts_with(prenom.unwrap_or_default().trim()));
    }
    assert_eq!(w.ligne(autre.id).await.2, None, "la demande non prise ne bouge pas");
    // L'absent, lui, ne porte plus rien : plus de demandes à signaler.
    assert!(eg.demandes_des_absents_for_actor(etab, cpe.user, "cpe", &sc).await.unwrap().is_empty());
    // Réaffectation unitaire : le responsable seul ; le porteur ne se réaffecte pas lui-même en douce.
    let (id0, v0) = (ids[0], w.version(ids[0]).await);
    assert_eq!(eg.reaffecter_for_actor(etab, adm.user, "administratif", &sa, id0, v0, Cible::File, None).await.unwrap_err(), RefusDemande::Interdit);
    assert!(matches!(eg.reaffecter_for_actor(etab, cpe.user, "cpe", &sc, id0, v0, Cible::Agent(adm.agent.unwrap()), None).await, Err(RefusDemande::Conflit(_))), "déjà porté par cette personne");
    assert!(matches!(eg.reaffecter_for_actor(etab, cpe.user, "cpe", &sc, id0, v0, Cible::Agent(prof.agent.unwrap()), None).await, Err(RefusDemande::Conflit(_))), "le destinataire est absent");
    assert!(matches!(eg.reaffecter_for_actor(etab, cpe.user, "cpe", &sc, id0, v0, Cible::File, Some("parce_que")).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.reaffecter_for_actor(etab, cpe.user, "cpe", &sc, id0, v0 - 1, Cible::File, None).await, Err(RefusDemande::Conflit(_))));
    let e = eg.reaffecter_for_actor(etab, cpe.user, "cpe", &sc, id0, v0, Cible::File, Some("surcharge")).await.unwrap();
    assert_eq!(e.statut, "deposee", "remise en file : « reçue »");
    assert_eq!(w.ligne(id0).await.2, None);
    assert!(matches!(eg.reaffecter_for_actor(etab, cpe.user, "cpe", &sc, id0, e.version, Cible::File, None).await, Err(RefusDemande::Conflit(_))));
    let _ = sp;
}

// ───────────────────────── Délégation ponctuelle et reprise « pour » un délégant ─────────────────────────

#[tokio::test]
async fn t_delegation_ponctuelle_et_reprise_pour() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, cpe, adm, dir) = (w.m.prof, w.m.cpe, w.adm, w.m.direction);
    let (etab, eg) = (w.m.etab, &w.ent);
    let e = w.maint().await;
    let e = w.prendre(&prof, e.id).await;
    let sp = w.src(&prof).await;
    let sa = w.src(&adm).await;
    let sc = w.src(&cpe).await;
    // Le porteur délègue à un membre de la file, avec une note interne ; jamais à un non-membre (anti-escalade).
    assert!(matches!(eg.deleguer_for_actor(etab, prof.user, "prof", &sp, e.id, e.version, dir.agent.unwrap(), None).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.deleguer_for_actor(etab, prof.user, "prof", &sp, e.id, e.version, adm.agent.unwrap(), Some("  ")).await, Err(RefusDemande::Invalide(_))));
    let d = eg.deleguer_for_actor(etab, prof.user, "prof", &sp, e.id, e.version, adm.agent.unwrap(), Some("Je suis absent demain, merci.")).await.unwrap();
    assert_eq!(w.ligne(e.id).await.2, adm.agent);
    let notes = w.messages(&cpe, e.id).await.unwrap();
    assert!(notes.iter().any(|m| m.fil == "interne" && m.texte.as_deref() == Some("Je suis absent demain, merci.")));
    assert!(w.messages(&w.m.compta.clone(), e.id).await.unwrap().is_empty(), "la note de délégation n'est pas visible du demandeur");
    // Le nouveau porteur n'est pas responsable : il ne délègue plus « au nom » de l'ancien ; l'ancien porteur non plus.
    assert_eq!(eg.deleguer_for_actor(etab, prof.user, "prof", &sp, e.id, d.version, cpe.agent.unwrap(), None).await.unwrap_err(), RefusDemande::Interdit);
    // Le responsable, lui, peut déléguer la demande de n'importe quel porteur.
    let d2 = eg.deleguer_for_actor(etab, cpe.user, "cpe", &sc, e.id, d.version, prof.agent.unwrap(), None).await.unwrap();
    assert_eq!(w.ligne(e.id).await.2, prof.agent);
    // Délégation DATÉE du socle 6 : « pour » le délégant, d'un geste. Sans délégation active : refusé.
    let v = d2.version;
    assert_eq!(eg.reprendre_pour_delegant_for_actor(etab, adm.user, "administratif", &sa, e.id, v).await.unwrap_err(), RefusDemande::Interdit);
    let deleg = |debut: i64, fin: i64, statut: &'static str| {
        let (delegue, delegant) = (adm.user, prof.user);
        let jour = w.jour;
        async move {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_delegation (id, etablissement_id, delegue_user_id, delegant_user_id, debut, fin, statut)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(id)
            .bind(etab)
            .bind(delegue)
            .bind(delegant)
            .bind(jour + chrono::Duration::days(debut))
            .bind(jour + chrono::Duration::days(fin))
            .bind(statut)
            .execute(eg.pool_verif())
            .await
            .unwrap();
            id
        }
    };
    deleg(-10, -1, "accordee").await; // expirée
    assert_eq!(eg.reprendre_pour_delegant_for_actor(etab, adm.user, "administratif", &sa, e.id, v).await.unwrap_err(), RefusDemande::Interdit, "délégation expirée");
    let rev = deleg(-1, 5, "accordee").await;
    sqlx::query("UPDATE ent_delegation SET statut = 'revoquee', revoquee_le = now() WHERE id = $1").bind(rev).execute(eg.pool_verif()).await.unwrap();
    assert_eq!(eg.reprendre_pour_delegant_for_actor(etab, adm.user, "administratif", &sa, e.id, v).await.unwrap_err(), RefusDemande::Interdit, "délégation révoquée");
    let ok = deleg(-1, 5, "accordee").await;
    // Un autre membre sans délégation ne reprend pas ; le délégataire, oui.
    assert_eq!(eg.reprendre_pour_delegant_for_actor(etab, cpe.user, "cpe", &sc, e.id, v).await.unwrap_err(), RefusDemande::Interdit);
    let r = eg.reprendre_pour_delegant_for_actor(etab, adm.user, "administratif", &sa, e.id, v).await.unwrap();
    assert_eq!(w.ligne(e.id).await.2, adm.agent);
    let (ref_id, motif): (Option<Uuid>, Option<String>) = sqlx::query_as(
        "SELECT ref_id, motif_code FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'reprise_deleguee'",
    )
    .bind(etab)
    .bind(e.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert_eq!((ref_id, motif.as_deref()), (Some(ok), Some("delegation")));
    // On ne reprend pas ce qu'on porte déjà ; une demande non prise ne se « reprend » pas.
    assert!(matches!(eg.reprendre_pour_delegant_for_actor(etab, adm.user, "administratif", &sa, e.id, r.version).await, Err(RefusDemande::Conflit(_))));
    let libre = w.maint().await;
    assert!(matches!(eg.reprendre_pour_delegant_for_actor(etab, adm.user, "administratif", &sa, libre.id, libre.version).await, Err(RefusDemande::Conflit(_))));
    assert_eq!(eg.verifier_chaine_demande(etab, e.id).await.unwrap(), None);
}

// ───────────────────────── Consultation d'un collègue ─────────────────────────

#[tokio::test]
async fn t_consultation_collegue() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof, cpe, adm, dir) = (w.m.compta, w.m.prof, w.m.cpe, w.adm, w.m.direction);
    let (etab, eg) = (w.m.etab, &w.ent);
    let e = w.maint().await;
    let e = w.prendre(&prof, e.id).await;
    let mut docs = Docs::default();
    let sp = w.src(&prof).await;
    let sd = w.src(&dir).await;
    // Deux pièces : une partagée avec le consulté, l'autre non.
    let (d1, d2) = (docs.doc(&prof), docs.doc(&prof));
    let p1 = eg.joindre_piece_for_actor(etab, prof.user, "prof", &sp, &docs, e.id, PieceAJoindre { document_id: d1, role: RolePiece::Annexe, au_demandeur: true }).await.unwrap();
    let p2 = eg.joindre_piece_for_actor(etab, prof.user, "prof", &sp, &docs, e.id, PieceAJoindre { document_id: d2, role: RolePiece::Annexe, au_demandeur: false }).await.unwrap();
    let consult = |agent: Uuid, d: u16, partage: bool, pieces: Vec<Uuid>| ConsultationNouvelle {
        agent_id: agent,
        question: "Le plafond est-il sous votre responsabilité ?".into(),
        delai_jours: d,
        partage_demande: partage,
        pieces_partagees: pieces,
    };
    // Refus : soi-même, le demandeur, un agent inconnu, un délai hors bornes, une pièce d'ailleurs.
    assert!(matches!(eg.consulter_for_actor(etab, prof.user, "prof", &sp, e.id, consult(prof.agent.unwrap(), 3, false, vec![])).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.consulter_for_actor(etab, prof.user, "prof", &sp, e.id, consult(compta.agent.unwrap(), 3, false, vec![])).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.consulter_for_actor(etab, prof.user, "prof", &sp, e.id, consult(Uuid::new_v4(), 3, false, vec![])).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.consulter_for_actor(etab, prof.user, "prof", &sp, e.id, consult(dir.agent.unwrap(), 0, false, vec![])).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.consulter_for_actor(etab, prof.user, "prof", &sp, e.id, consult(dir.agent.unwrap(), 31, false, vec![])).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.consulter_for_actor(etab, prof.user, "prof", &sp, e.id, consult(dir.agent.unwrap(), 3, false, vec![Uuid::new_v4()])).await, Err(RefusDemande::Invalide(_))));
    // Le demandeur ne consulte pas.
    assert_eq!(eg.consulter_for_actor(etab, compta.user, "compta", &w.src(&compta).await, e.id, consult(dir.agent.unwrap(), 3, false, vec![])).await.unwrap_err(), RefusDemande::Interdit);
    let cid = eg.consulter_for_actor(etab, prof.user, "prof", &sp, e.id, consult(dir.agent.unwrap(), 3, true, vec![p1])).await.unwrap();
    // La demande reste chez le traitant ; le consulté la voit dans « mes consultations » mais n'entre pas dans la file.
    assert_eq!(w.ligne(e.id).await.2, prof.agent);
    assert_eq!(eg.mes_consultations_for_actor(etab, dir.user, "direction").await.unwrap().len(), 1);
    assert_eq!(w.detail(&dir, e.id).await.unwrap_err(), RefusDemande::Introuvable, "un consulté ne lit pas le détail de la demande");
    assert!(eg.ma_file_for_actor(etab, dir.user, "direction", &sd).await.unwrap().iter().all(|d| d.id != e.id));
    // Il lit la question, ce qui est partagé (objet, texte, la pièce 1), pas la pièce 2, pas le fil interne.
    let v = eg.consultation_for_actor(etab, dir.user, "direction", cid).await.unwrap();
    assert_eq!(v.numero, e.numero);
    assert!(v.objet.is_some() && v.texte.is_some());
    assert_eq!(v.pieces.iter().map(|p| p.id).collect::<Vec<_>>(), vec![p1]);
    let s = &sd;
    let lire = |p: Uuid| eg.lire_piece_for_actor(etab, dir.user, "direction", s, &docs, p);
    assert!(lire(p1).await.is_ok());
    assert_eq!(lire(p2).await.unwrap_err(), RefusDemande::Introuvable, "une pièce non partagée n'est pas lisible du consulté");
    let note = w.ecrire(&prof, &docs, e.id, Fil::Interne, "note interne secrète").await.unwrap();
    let _ = note;
    let ms = w.messages(&dir, e.id).await.unwrap();
    assert!(ms.iter().all(|m| m.fil == "consultation"), "le consulté ne lit que la consultation");
    // Le traitant écrit dans l'échange ; le consulté répond ; le demandeur ne voit rien de cet échange.
    w.ecrire(&prof, &docs, e.id, Fil::Consultation(cid), "Précision : c'est au 2e étage.").await.unwrap();
    assert!(matches!(eg.repondre_consultation_for_actor(etab, cpe.user, "cpe", &w.src(&cpe).await, cid, "Je réponds à sa place").await, Err(RefusDemande::Introuvable)));
    let (rep,): (Option<chrono::DateTime<chrono::Utc>>,) = sqlx::query_as("SELECT repondue_le FROM ent_dem_consultation WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(cid).fetch_one(eg.pool_verif()).await.unwrap();
    assert!(rep.is_none(), "un membre de la file ne répond pas à la place du consulté");
    eg.repondre_consultation_for_actor(etab, dir.user, "direction", &sd, cid, "Oui, c'est à nous : intervention jeudi.").await.unwrap();
    let (rep,): (Option<chrono::DateTime<chrono::Utc>>,) = sqlx::query_as("SELECT repondue_le FROM ent_dem_consultation WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(cid).fetch_one(eg.pool_verif()).await.unwrap();
    assert!(rep.is_some());
    assert!(eg.mes_consultations_for_actor(etab, dir.user, "direction").await.unwrap().is_empty(), "une consultation répondue n'est plus ouverte");
    assert!(w.messages(&compta, e.id).await.unwrap().iter().all(|m| m.fil == "demandeur"));
    // Seul le consulté lit sa consultation : le CPE et le demandeur : 404.
    assert_eq!(eg.consultation_for_actor(etab, cpe.user, "cpe", cid).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.consultation_for_actor(etab, compta.user, "compta", cid).await.unwrap_err(), RefusDemande::Introuvable);
    // Sans partage de la demande : le consulté ne lit ni l'objet ni le texte.
    let cid2 = eg.consulter_for_actor(etab, prof.user, "prof", &sp, e.id, consult(adm.agent.unwrap(), 5, false, vec![])).await.unwrap();
    let v2 = eg.consultation_for_actor(etab, adm.user, "administratif", cid2).await.unwrap();
    assert!(v2.objet.is_none() && v2.texte.is_none() && v2.pieces.is_empty());
    // Jamais pour une demande confidentielle (C2+) : le tiers n'a pas le droit de la lire.
    let rh = w.deposer(&compta, depot_rh()).await.unwrap();
    w.detail(&dir, rh.id).await.unwrap();
    let rh = w.prendre(&dir, rh.id).await;
    let r = eg.consulter_for_actor(etab, dir.user, "direction", &sd, rh.id, consult(prof.agent.unwrap(), 3, true, vec![])).await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))), "consultation d'une demande C2 : {r:?}");
    assert_eq!(eg.verifier_chaine_demande(etab, e.id).await.unwrap(), None);
}

// ───────────────────────── Désignation d'une personne visée (DEM-360, R-DM47) ─────────────────────────

#[tokio::test]
async fn t_designation_personne_visee() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof, cpe, adm, dir) = (w.m.compta, w.m.prof, w.m.cpe, w.adm, w.m.direction);
    let (etab, eg) = (w.m.etab, &w.ent);
    let e = w.deposer(&compta, depot("secretariat.demande_interne")).await.unwrap();
    let e = w.prendre(&prof, e.id).await;
    let sc = w.src(&compta).await;
    let sp = w.src(&prof).await;
    // Le demandeur voit la liste des membres du service (libellés seulement), pas lui-même ; nul autre ne la voit.
    let l = eg.membres_designables_for_actor(etab, compta.user, "compta", &sc, e.id).await.unwrap();
    assert_eq!(l.len(), 3);
    assert!(l.iter().all(|m| m.personne_id != compta.personne));
    assert_eq!(eg.membres_designables_for_actor(etab, prof.user, "prof", &sp, e.id).await.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(eg.membres_designables_for_actor(etab, dir.user, "direction", &w.src(&dir).await, e.id).await.unwrap_err(), RefusDemande::Introuvable);
    // Désigner un membre : refusé hors du service, soi-même, ou par un non-demandeur.
    assert!(matches!(eg.designer_personne_visee_for_actor(etab, compta.user, "compta", &sc, e.id, e.version, dir.personne).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.designer_personne_visee_for_actor(etab, compta.user, "compta", &sc, e.id, e.version, compta.personne).await, Err(RefusDemande::Invalide(_))));
    assert_eq!(eg.designer_personne_visee_for_actor(etab, prof.user, "prof", &sp, e.id, e.version, adm.personne).await.unwrap_err(), RefusDemande::Interdit);
    // Le porteur est visé : la demande retourne dans la file, il ne la voit plus, ne la prend plus, ne lit plus le fil.
    let r = eg.designer_personne_visee_for_actor(etab, compta.user, "compta", &sc, e.id, e.version, prof.personne).await.unwrap();
    assert_eq!(r.statut, "deposee");
    assert_eq!(w.ligne(e.id).await.2, None);
    assert_eq!(w.detail(&prof, e.id).await.unwrap_err(), RefusDemande::Introuvable);
    assert!(eg.ma_file_for_actor(etab, prof.user, "prof", &w.src(&prof).await).await.unwrap().iter().all(|d| d.id != e.id));
    assert_eq!(eg.agir_sur_demande_for_actor(etab, prof.user, "prof", &w.src(&prof).await, e.id, r.version, Geste::Prendre).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(w.messages(&prof, e.id).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(w.ecrire(&prof, &Docs::default(), e.id, Fil::Interne, "je regarde quand même").await.unwrap_err(), RefusDemande::Introuvable);
    // Il n'est plus destinataire : ni consulté, ni mis en copie, ni destinataire d'une délégation.
    let sc2 = w.src(&cpe).await;
    let c = ConsultationNouvelle { agent_id: prof.agent.unwrap(), question: "Avis ?".into(), delai_jours: 2, partage_demande: false, pieces_partagees: vec![] };
    assert!(matches!(eg.consulter_for_actor(etab, cpe.user, "cpe", &sc2, e.id, c).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.ajouter_abonne_for_actor(etab, cpe.user, "cpe", &sc2, e.id, prof.personne, QualiteAbonne::CopieInterne).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.reaffecter_for_actor(etab, cpe.user, "cpe", &sc2, e.id, w.version(e.id).await, Cible::Agent(prof.agent.unwrap()), None).await, Err(RefusDemande::Invalide(_))));
    // Les autres membres, eux, la voient toujours. Désigner deux fois : refusé.
    assert!(w.detail(&cpe, e.id).await.is_ok());
    assert!(matches!(eg.designer_personne_visee_for_actor(etab, compta.user, "compta", &sc, e.id, w.version(e.id).await, prof.personne).await, Err(RefusDemande::Conflit(_))));
    // Le fait ne nomme pas la personne visée (aucune trace lisible par un membre).
    let (ag,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'personne_visee_designee'
            AND (agent_avant_id IS NOT NULL OR agent_apres_id IS NOT NULL OR ref_id IS NOT NULL)",
    )
    .bind(etab)
    .bind(e.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert_eq!(ag, 0);
    // Tous les membres visés : la demande passe à la file de repli (la direction), jamais à tout le personnel.
    eg.designer_personne_visee_for_actor(etab, compta.user, "compta", &sc, e.id, w.version(e.id).await, cpe.personne).await.unwrap();
    assert_eq!(w.ligne(e.id).await.1, w.fa, "tant qu'un membre reste, la file ne change pas");
    let r = eg.designer_personne_visee_for_actor(etab, compta.user, "compta", &sc, e.id, w.version(e.id).await, adm.personne).await.unwrap();
    assert_eq!(w.ligne(e.id).await.1, w.repli);
    assert_eq!(r.statut, "deposee");
    let (motif,): (Option<String>,) = sqlx::query_as(
        "SELECT motif_code FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'routage' ORDER BY rang DESC LIMIT 1",
    )
    .bind(etab)
    .bind(e.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert_eq!(motif.as_deref(), Some("repli_personne_visee"));
    assert!(w.detail(&dir, e.id).await.is_ok(), "le repli la reçoit");
    for q in [&prof, &cpe, &adm] {
        assert_eq!(w.detail(q, e.id).await.unwrap_err(), RefusDemande::Introuvable, "{} voit encore la demande", q.role);
    }
    // Le demandeur voit à qui part sa demande (le service de repli), pas une personne nommée.
    assert_eq!(w.preuves(&compta, e.id).await.unwrap().envoyee.role.as_deref(), Some("la direction"));
    assert_eq!(eg.verifier_chaine_demande(etab, e.id).await.unwrap(), None);
    // Le demandeur lui-même n'est jamais exclu de sa demande.
    assert!(eg.demande_demandeur_for_actor(etab, compta.user, e.id).await.is_ok());
}

// ───────────────────────── Requalification : CDM-01 et CDM-11 ─────────────────────────

#[tokio::test]
async fn t_requalification_cdm01_cdm11() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof, cpe, adm, dir) = (w.m.compta, w.m.prof, w.m.cpe, w.adm, w.m.direction);
    let (etab, eg) = (w.m.etab, &w.ent);
    // CDM-01 : mauvaise nature (même niveau de confidentialité, même file) : la nature et les échéances suivent.
    let e = w.maint().await;
    let sp = w.src(&prof).await;
    let sa = w.src(&adm).await;
    assert!(matches!(eg.requalifier_for_actor(etab, prof.user, "prof", &sp, e.id, e.version, "maintenance.intervention").await, Err(RefusDemande::Invalide(_))), "même nature");
    assert!(matches!(eg.requalifier_for_actor(etab, prof.user, "prof", &sp, e.id, e.version, "inconnue.nature").await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.requalifier_for_actor(etab, prof.user, "prof", &sp, e.id, e.version, "numerique.assistance").await, Err(RefusDemande::Invalide(_))), "nature non activée");
    // Directement en base, la nature ne change pas : seule la requalification le peut.
    let r = sqlx::query("UPDATE ent_dem_demande SET nature_code = 'secretariat.demande_interne', version = version + 1 WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(e.id)
        .execute(eg.pool_verif())
        .await;
    assert!(r.is_err(), "la nature a changé hors requalification");
    let q = eg.requalifier_for_actor(etab, prof.user, "prof", &sp, e.id, e.version, "secretariat.demande_interne").await.unwrap();
    assert!(!q.file_change);
    assert_eq!(w.ligne(e.id).await.3, "secretariat.demande_interne");
    let (v_nature,): (i16,) = sqlx::query_as("SELECT nature_version FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(e.id).fetch_one(eg.pool_verif()).await.unwrap();
    assert_eq!(v_nature, nature("secretariat.demande_interne").unwrap().version);
    let (ouvertes,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2 AND traitee_le IS NULL",
    )
    .bind(etab)
    .bind(e.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert_eq!(ouvertes, 2, "prise en charge et traitement recalculés pour la nouvelle nature");
    let evenement = w.faits(e.id).await;
    assert!(evenement.contains(&"requalifiee".to_string()));
    // Le délai change ou pas : l'avis au demandeur ne part que s'il change (l'événement le dit).
    let (ev,): (String,) = sqlx::query_as(
        "SELECT payload->>'evenement' FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'demande.requalifiee'",
    )
    .bind(etab)
    .bind(e.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert_eq!(ev, if q.delai_change { "requalifiee_delai_change" } else { "requalifiee" });
    // Un membre qui n'a pas la main (la demande est portée par un autre) ne requalifie pas.
    let e = w.prendre(&prof, e.id).await;
    assert_eq!(eg.requalifier_for_actor(etab, adm.user, "administratif", &sa, e.id, e.version, "maintenance.intervention").await.unwrap_err(), RefusDemande::Interdit);

    // CDM-11 : une demande RH déposée par erreur en « secrétariat » (C1) : le secrétariat la requalifie en demande RH
    // (C2) ; elle QUITTE la file ; les textes déjà écrits sont SCELLÉS ; l'historique de lecture reste.
    let mut d = depot("secretariat.demande_interne");
    d.texte = Some("Je souhaite un entretien confidentiel sur ma situation.".into());
    let e = w.deposer(&compta, d).await.unwrap();
    let docs = Docs::default();
    w.detail(&cpe, e.id).await.unwrap(); // « la lecture avant » : tracée
    w.ecrire(&cpe, &docs, e.id, Fil::Demandeur, "Pouvez-vous préciser ?").await.unwrap();
    w.ecrire(&prof, &docs, e.id, Fil::Interne, "Cela ressemble à une demande RH.").await.unwrap();
    let sc = w.src(&cpe).await;
    let v = w.version(e.id).await;
    // Jamais vers une nature qui n'est pas proposée, jamais moins confidentielle.
    let q = eg.requalifier_for_actor(etab, cpe.user, "cpe", &sc, e.id, v, "rh.demande_agent").await.unwrap();
    assert!(q.file_change);
    assert_eq!(w.ligne(e.id).await.1, w.fc);
    let (conf, scelle, texte): (String, bool, Option<String>) =
        sqlx::query_as("SELECT confidentialite, texte_scelle, texte FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(e.id).fetch_one(eg.pool_verif()).await.unwrap();
    assert_eq!(conf, "C2");
    assert!(scelle && !texte.unwrap().contains("entretien confidentiel"), "le texte n'est plus en clair en base");
    let (clairs,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_message WHERE etablissement_id = $1 AND demande_id = $2 AND NOT texte_scelle").bind(etab).bind(e.id).fetch_one(eg.pool_verif()).await.unwrap();
    assert_eq!(clairs, 0, "les messages déjà écrits sont scellés");
    // L'ancienne file ne la voit plus ; la file confidentielle la lit, texte et messages déchiffrés.
    for p in [&prof, &cpe, &adm] {
        assert_eq!(w.detail(p, e.id).await.unwrap_err(), RefusDemande::Introuvable, "{} voit encore la demande requalifiée", p.role);
        assert_eq!(w.messages(p, e.id).await.unwrap_err(), RefusDemande::Introuvable);
    }
    let d = w.detail(&dir, e.id).await.unwrap();
    assert_eq!(d.texte.as_deref(), Some("Je souhaite un entretien confidentiel sur ma situation."));
    let ms = w.messages(&dir, e.id).await.unwrap();
    assert_eq!(ms.len(), 2);
    assert!(ms.iter().any(|m| m.texte.as_deref() == Some("Cela ressemble à une demande RH.")));
    // L'historique de lecture avant la requalification reste (lecture C1 non tracée, mais les faits restent).
    assert!(w.faits(e.id).await.contains(&"vue_par_service".to_string()));
    // Le demandeur lit toujours son propre texte, déchiffré.
    let vu = eg.demande_demandeur_for_actor(etab, compta.user, e.id).await.unwrap();
    assert_eq!(vu.texte.as_deref(), Some("Je souhaite un entretien confidentiel sur ma situation."));
    // Jamais moins confidentielle : la cellule RH ne redescend pas vers le secrétariat.
    let sd = w.src(&dir).await;
    let r = eg.requalifier_for_actor(etab, dir.user, "direction", &sd, e.id, w.version(e.id).await, "secretariat.demande_interne").await;
    assert!(matches!(r, Err(RefusDemande::Invalide(_))), "{r:?}");
    assert_eq!(eg.verifier_chaine_demande(etab, e.id).await.unwrap(), None);
}

// ───────────────────────── « Résolu ? Oui / Non » ─────────────────────────

#[tokio::test]
async fn t_resolu_oui_non() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof) = (w.m.compta, w.m.prof);
    let (etab, eg) = (w.m.etab, &w.ent);
    let sp = w.src(&prof).await;
    let sc = w.src(&compta).await;
    // Pas de réponse avant la clôture ; seul le demandeur répond.
    let a = w.maint().await;
    let a = w.prendre(&prof, a.id).await;
    assert!(matches!(eg.repondre_resolu_for_actor(etab, compta.user, "compta", &sc, a.id, a.version, true).await, Err(RefusDemande::Conflit(_))));
    let a = w.cloturer(&prof, a.id).await;
    assert_eq!(eg.repondre_resolu_for_actor(etab, prof.user, "prof", &sp, a.id, a.version, true).await.unwrap_err(), RefusDemande::Interdit);
    assert!(matches!(eg.repondre_resolu_for_actor(etab, compta.user, "compta", &sc, a.id, a.version + 3, true).await, Err(RefusDemande::Conflit(_))));
    // « Oui » : merci ; rien ne bouge ; on ne répond qu'une fois ; la réponse n'est rattachée à aucun agent.
    assert_eq!(eg.repondre_resolu_for_actor(etab, compta.user, "compta", &sc, a.id, a.version, true).await.unwrap(), SuiteResolu::Merci);
    assert_eq!(w.ligne(a.id).await.0, "close");
    assert!(matches!(eg.repondre_resolu_for_actor(etab, compta.user, "compta", &sc, a.id, a.version, false).await, Err(RefusDemande::Conflit(_))), "une seule réponse");
    let (agents,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_dem_fait WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'resolu_oui'
            AND (agent_avant_id IS NOT NULL OR agent_apres_id IS NOT NULL OR file_avant_id IS NOT NULL OR file_apres_id IS NOT NULL)",
    )
    .bind(etab)
    .bind(a.id)
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert_eq!(agents, 0, "« Résolu ? » ne touche aucune note d'agent");
    // « Non » dans le délai : la MÊME demande est rouverte, sans porteur, avec de nouvelles échéances.
    let b = w.maint().await;
    let b = w.prendre(&prof, b.id).await;
    let b = w.cloturer(&prof, b.id).await;
    let SuiteResolu::Rouverte(r) = eg.repondre_resolu_for_actor(etab, compta.user, "compta", &sc, b.id, b.version, false).await.unwrap() else { panic!("réouverture attendue") };
    assert_eq!((r.id, r.statut.as_str()), (b.id, "rouverte"));
    let (statut, _, porteur, _) = w.ligne(b.id).await;
    assert_eq!((statut.as_str(), porteur), ("rouverte", None));
    let (clos, issue, conservation): (Option<chrono::DateTime<chrono::Utc>>, Option<String>, Option<NaiveDate>) =
        sqlx::query_as("SELECT clos_le, issue, conservation_jusqu_au FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(b.id).fetch_one(eg.pool_verif()).await.unwrap();
    assert!(clos.is_none() && issue.is_none() && conservation.is_none(), "plus de clôture ni de conservation en cours");
    let (ouvertes,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2 AND traitee_le IS NULL").bind(etab).bind(b.id).fetch_one(eg.pool_verif()).await.unwrap();
    assert_eq!(ouvertes, 2);
    assert!(w.detail(&prof, b.id).await.is_ok());
    let e = w.prendre(&prof, b.id).await;
    assert_eq!(e.statut, "prise");
    let h = eg.historique_demandeur_for_actor(etab, compta.user, b.id).await.unwrap();
    assert!(h.iter().any(|x| x.etape == "rouverte"));
    assert_eq!(w.preuves(&compta, b.id).await.unwrap().etat, EtatSimple::EnCours);
    // « Non » hors délai (CDM-86) : pas de réouverture ; une nouvelle demande liée est conseillée.
    let c = w.maint().await;
    let c = w.prendre(&prof, c.id).await;
    let c = w.cloturer(&prof, c.id).await;
    sqlx::query("UPDATE ent_dem_demande SET clos_le = now() - interval '40 days', version = version + 1 WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(c.id).execute(eg.pool_verif()).await.unwrap();
    let v = w.version(c.id).await;
    assert_eq!(eg.repondre_resolu_for_actor(etab, compta.user, "compta", &sc, c.id, v, false).await.unwrap(), SuiteResolu::HorsDelai { ancienne_demande: c.id });
    assert_eq!(w.ligne(c.id).await.0, "close");
    assert!(w.faits(c.id).await.contains(&"resolu_non_hors_delai".to_string()));
    for id in [a.id, b.id, c.id] {
        assert_eq!(eg.verifier_chaine_demande(etab, id).await.unwrap(), None);
    }
}

// ───────────────────────── Abonnés : co-demandeur, copie interne, appui ─────────────────────────

#[tokio::test]
async fn t_abonnes_co_demandeur_copie_appui() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof, cpe, adm, dir) = (w.m.compta, w.m.prof, w.m.cpe, w.adm, w.m.direction);
    let (etab, eg) = (w.m.etab, &w.ent);
    // Copie interne : un membre tient un collègue informé ; il lit ce qui est visible du demandeur, pas le reste.
    let e = w.maint().await;
    let sp = w.src(&prof).await;
    let docs = Docs::default();
    w.ecrire(&prof, &docs, e.id, Fil::Demandeur, "Message au demandeur").await.unwrap();
    w.ecrire(&prof, &docs, e.id, Fil::Interne, "Note interne").await.unwrap();
    assert_eq!(eg.ajouter_abonne_for_actor(etab, compta.user, "compta", &w.src(&compta).await, e.id, dir.personne, QualiteAbonne::CopieInterne).await.unwrap_err(), RefusDemande::Interdit, "le demandeur ne met personne en copie interne");
    assert!(matches!(eg.ajouter_abonne_for_actor(etab, prof.user, "prof", &sp, e.id, Uuid::new_v4(), QualiteAbonne::CopieInterne).await, Err(RefusDemande::Invalide(_))));
    assert!(matches!(eg.ajouter_abonne_for_actor(etab, prof.user, "prof", &sp, e.id, compta.personne, QualiteAbonne::CopieInterne).await, Err(RefusDemande::Invalide(_))), "le demandeur est déjà là");
    eg.ajouter_abonne_for_actor(etab, prof.user, "prof", &sp, e.id, dir.personne, QualiteAbonne::CopieInterne).await.unwrap();
    assert!(matches!(eg.ajouter_abonne_for_actor(etab, prof.user, "prof", &sp, e.id, dir.personne, QualiteAbonne::CopieInterne).await, Err(RefusDemande::Conflit(_))), "un seul abonnement actif");
    let vu = w.messages(&dir, e.id).await.unwrap();
    assert_eq!(vu.len(), 1);
    assert_eq!(vu[0].fil, "demandeur");
    assert_eq!(w.ecrire(&dir, &docs, e.id, Fil::Demandeur, "Je réponds").await.unwrap_err(), RefusDemande::Interdit, "la copie interne n'écrit pas");
    assert_eq!(w.detail(&dir, e.id).await.unwrap_err(), RefusDemande::Introuvable, "la copie interne ne lit pas le détail du traitement");
    // Jamais en copie sur une demande confidentielle.
    let rh = w.deposer(&compta, depot_rh()).await.unwrap();
    w.detail(&dir, rh.id).await.unwrap();
    assert!(matches!(eg.ajouter_abonne_for_actor(etab, dir.user, "direction", &w.src(&dir).await, rh.id, prof.personne, QualiteAbonne::CopieInterne).await, Err(RefusDemande::Invalide(_))));
    // Appui (+1) : un autre personnel appuie la panne, reçoit l'avis de clôture et ne lit rien.
    assert_eq!(eg.appuyer_for_actor(etab, compta.user, "compta", e.id).await.unwrap_err(), RefusDemande::Conflit("C'est votre propre demande."));
    let sec = w.deposer(&compta, depot("secretariat.demande_interne")).await.unwrap();
    assert_eq!(eg.appuyer_for_actor(etab, cpe.user, "cpe", sec.id).await.unwrap_err(), RefusDemande::Introuvable, "une demande non appuyable n'existe pas pour l'appui");
    // Un tiers du personnel (la direction, hors de la file) appuie une seconde panne : il reçoit l'avis, ne lit rien.
    let e3 = w.maint().await;
    eg.appuyer_for_actor(etab, dir.user, "direction", e3.id).await.expect("appui");
    assert!(matches!(eg.appuyer_for_actor(etab, dir.user, "direction", e3.id).await, Err(RefusDemande::Conflit(_))), "un seul appui actif");
    assert_eq!(w.messages(&dir, e3.id).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.pieces_for_actor(etab, dir.user, "direction", &w.src(&dir).await, e3.id).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(w.detail(&dir, e3.id).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(w.ecrire(&dir, &docs, e3.id, Fil::Demandeur, "j'appuie").await.unwrap_err(), RefusDemande::Interdit);
    let _ = adm;
    // Co-demandeur : l'autre responsable du même élève, jamais sans lien actif, jamais pour un autre rôle.
    let parent = w.m.parent;
    let tom = fiche(eg, etab, w.tom_user, "parent", false).await;
    let mut d = depot("scolarite.attestation");
    d.eleve_id = Some(w.m.eleve);
    d.valeurs = vec![ValeurSaisie { champ: "type_attestation".into(), rang: 0, valeur: Valeur::Code("assiduite".into()) }];
    let att = w.deposer(&parent, d).await.expect("dépôt famille");
    let sf = w.src(&parent).await;
    // Sans lien actif sur l'élève : refusé (interdiction ou absence de lien : même réponse).
    assert!(matches!(eg.ajouter_abonne_for_actor(etab, parent.user, "parent", &sf, att.id, tom.personne, QualiteAbonne::CoDemandeur).await, Err(RefusDemande::Invalide(_))));
    sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id) VALUES (gen_random_uuid(), $1, $2, $3)")
        .bind(etab)
        .bind(w.m.eleve)
        .bind(tom.user)
        .execute(eg.pool_verif())
        .await
        .unwrap();
    // Seul le demandeur ajoute un co-demandeur ; ni le service, ni le co-demandeur lui-même.
    assert_eq!(eg.ajouter_abonne_for_actor(etab, prof.user, "prof", &w.src(&prof).await, att.id, tom.personne, QualiteAbonne::CoDemandeur).await.unwrap_err(), RefusDemande::Interdit);
    eg.ajouter_abonne_for_actor(etab, parent.user, "parent", &sf, att.id, tom.personne, QualiteAbonne::CoDemandeur).await.unwrap();
    // Le co-demandeur voit la demande et le fil visible du demandeur, écrit à son tour, ne l'annule pas.
    let stom = w.src(&tom).await;
    assert!(eg.demande_demandeur_for_actor(etab, tom.user, att.id).await.is_ok());
    assert!(eg.mes_demandes_for_actor(etab, tom.user).await.unwrap().iter().any(|d| d.id == att.id));
    assert!(w.preuves(&tom, att.id).await.is_ok());
    let r = w.ecrire(&tom, &docs, att.id, Fil::Demandeur, "Je confirme").await.unwrap();
    assert_eq!(r.statut, "deposee");
    assert_eq!(w.ecrire(&tom, &docs, att.id, Fil::Interne, "interne ?").await.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(eg.agir_sur_demande_for_actor(etab, tom.user, "parent", &stom, att.id, 1, Geste::Annuler).await.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(eg.ajouter_abonne_for_actor(etab, tom.user, "parent", &stom, att.id, parent.personne, QualiteAbonne::CoDemandeur).await.unwrap_err(), RefusDemande::Interdit);
    // Un co-demandeur n'est jamais traitant : la file ne lui est pas ouverte.
    assert_eq!(w.detail(&tom, att.id).await.unwrap_err(), RefusDemande::Introuvable);
    assert_eq!(eg.verifier_chaine_demande(etab, att.id).await.unwrap(), None);
}

// ───────────────────────── Port du socle 16 : rattacher une conversation ─────────────────────────

#[tokio::test]
async fn t_port_messagerie_conversation() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (prof, cpe, dir) = (w.m.prof, w.m.cpe, w.m.direction);
    let (etab, eg) = (w.m.etab, &w.ent);
    let e = w.maint().await;
    let fil = Uuid::new_v4();
    let sp = w.src(&prof).await;
    // Sans le socle 16 (port « absent »), rien ne se rattache. Un outsider : 404.
    assert_eq!(eg.rattacher_conversation_for_actor(etab, prof.user, "prof", &sp, &MessagerieAbsente, e.id, fil).await.unwrap_err(), RefusDemande::Interdit);
    assert_eq!(eg.rattacher_conversation_for_actor(etab, dir.user, "direction", &w.src(&dir).await, &Messagerie(true), e.id, fil).await.unwrap_err(), RefusDemande::Introuvable);
    // Avec le port, le lien et le fait sont posés ; le même fil ne se rattache pas deux fois.
    eg.rattacher_conversation_for_actor(etab, prof.user, "prof", &sp, &Messagerie(true), e.id, fil).await.unwrap();
    assert!(w.faits(e.id).await.contains(&"conversation_rattachee".to_string()));
    assert!(matches!(eg.rattacher_conversation_for_actor(etab, cpe.user, "cpe", &w.src(&cpe).await, &Messagerie(true), e.id, fil).await, Err(RefusDemande::Conflit(_))));
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_dem_lien WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'issue_de_conversation' AND cible_ref = $3")
        .bind(etab)
        .bind(e.id)
        .bind(fil)
        .fetch_one(eg.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 1);
    assert!(MessagerieAbsente.conversation_depuis_demande(etab, prof.personne, e.id).is_none());
    assert!(Messagerie(true).conversation_depuis_demande(etab, prof.personne, e.id).is_some());
}

// ───────────────────────── DEM-399 : vues mais non prises ─────────────────────────

#[tokio::test]
async fn t_vues_non_prises() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (compta, prof, cpe, dir) = (w.m.compta, w.m.prof, w.m.cpe, w.m.direction);
    let (etab, eg) = (w.m.etab, &w.ent);
    let e = w.maint().await;
    let rh = w.deposer(&compta, depot_rh()).await.unwrap();
    let (sc, sd, sp) = (w.src(&cpe).await, w.src(&dir).await, w.src(&prof).await);
    // Rien n'est « vu » avant une ouverture du détail.
    assert!(eg.vues_non_prises_for_actor(etab, cpe.user, "cpe", &sc, 0).await.unwrap().is_empty());
    w.detail(&prof, e.id).await.unwrap();
    w.detail(&dir, rh.id).await.unwrap();
    // Le responsable de la file A la voit ; un membre simple non ; la direction (supervision) voit tout, sans contenu C2+.
    let l = eg.vues_non_prises_for_actor(etab, cpe.user, "cpe", &sc, 0).await.unwrap();
    assert_eq!(l.iter().map(|x| x.id).collect::<Vec<_>>(), vec![e.id]);
    assert_eq!(l[0].nature_code.as_deref(), Some("maintenance.intervention"));
    assert!(eg.vues_non_prises_for_actor(etab, prof.user, "prof", &sp, 0).await.unwrap().is_empty());
    let l = eg.vues_non_prises_for_actor(etab, dir.user, "direction", &sd, 0).await.unwrap();
    assert_eq!(l.len(), 2);
    assert!(l.iter().find(|x| x.id == rh.id).unwrap().nature_code.is_none(), "la nature d'une demande C2+ ne sort pas");
    // « Depuis plus d'un jour » : une demande vue aujourd'hui n'en fait pas partie.
    assert!(eg.vues_non_prises_for_actor(etab, dir.user, "direction", &sd, 1).await.unwrap().is_empty());
    // Une demande prise disparaît de la liste ; la famille n'y accède pas.
    w.prendre(&prof, e.id).await;
    assert_eq!(eg.vues_non_prises_for_actor(etab, dir.user, "direction", &sd, 0).await.unwrap().len(), 1);
    assert_eq!(eg.vues_non_prises_for_actor(etab, w.m.parent.user, "parent", &sc, 0).await.unwrap_err(), RefusDemande::Interdit);
}

// ───────────────────────── Garde SQL des nouvelles tables (D-036, NJ-89) ─────────────────────────

#[tokio::test]
async fn t_tables_n1_2_sans_json_ni_nom_de_fichier() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let eg = &w.ent;
    let tables = [
        "ent_dem_message",
        "ent_dem_piece",
        "ent_dem_abonne",
        "ent_dem_consultation",
        "ent_dem_consultation_partage",
        "ent_dem_lien",
        "ent_dem_personne_visee",
    ];
    // Aucune colonne JSON, tableau ou binaire (D-036) ; aucun nom de fichier, chemin ni type de fichier stocké.
    let (mauvaises,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM information_schema.columns
          WHERE table_name = ANY($1)
            AND (data_type IN ('json', 'jsonb', 'bytea', 'ARRAY')
                 OR column_name ~ '(nom_fichier|filename|file_name|chemin|path|mime|content_type|storage_key|url)')",
    )
    .bind(tables.iter().map(|t| t.to_string()).collect::<Vec<_>>())
    .fetch_one(eg.pool_verif())
    .await
    .unwrap();
    assert_eq!(mauvaises, 0);
    // Chaque table est cloisonnée par établissement (RLS) et ne se supprime pas.
    for t in tables {
        let (rls,): (bool,) = sqlx::query_as("SELECT relrowsecurity AND relforcerowsecurity FROM pg_class WHERE relname = $1").bind(t).fetch_one(eg.pool_verif()).await.unwrap();
        assert!(rls, "RLS forcée absente sur {t}");
        let r = sqlx::query(&format!("DELETE FROM {t} WHERE etablissement_id = $1")).bind(w.m.etab).execute(eg.pool_verif()).await;
        // Une table vide se « supprime » sans erreur ; on le vérifie plus bas avec des lignes.
        assert!(r.is_ok());
    }
    let e = w.maint().await;
    let docs = Docs::default();
    let m = w.ecrire(&w.m.prof.clone(), &docs, e.id, Fil::Interne, "x").await.unwrap();
    let r = sqlx::query("DELETE FROM ent_dem_message WHERE etablissement_id = $1 AND id = $2").bind(w.m.etab).bind(m.id).execute(eg.pool_verif()).await;
    assert!(r.is_err(), "ent_dem_message supprimable");
    let (c,): (Uuid,) = (Uuid::new_v4(),);
    let _ = c;
    // « Vue par le service » : un seul fait par demande et par file, garanti par la base.
    w.detail(&w.m.prof.clone(), e.id).await.unwrap();
    let r = sqlx::query(
        "INSERT INTO ent_dem_fait (id, etablissement_id, demande_id, type, auteur_personne_id, file_apres_id)
         VALUES (gen_random_uuid(), $1, $2, 'vue_par_service', $3, $4)",
    )
    .bind(w.m.etab)
    .bind(e.id)
    .bind(w.m.prof.personne)
    .bind(w.fa)
    .execute(eg.pool_verif())
    .await;
    assert!(r.is_err(), "deux faits « vue » pour la même file");
}

// ───────────────────────── Séparation entre établissements ─────────────────────────

#[tokio::test]
async fn t_separation_etablissements_n1_2() {
    let Some(w) = W::nouveau().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let e = w.maint().await;
    let docs = Docs::default();
    let msg = w.ecrire(&w.m.prof.clone(), &docs, e.id, Fil::Demandeur, "Bonjour").await.unwrap();
    // Un autre établissement : ses comptes ne lisent ni n'écrivent dans la demande du premier, ni en passant leur
    // propre établissement, ni en passant celui de la demande.
    let (ent, cast2) = labo().await.unwrap();
    let m2 = monde(&ent, &cast2).await;
    for etab in [m2.etab, w.m.etab] {
        let q = m2.prof;
        let s = SourcesAbsentes;
        let r = ent.messages_for_actor(etab, q.user, q.role, &s, e.id).await;
        assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
        let r = ent
            .ecrire_message_for_actor(etab, q.user, q.role, &s, &DocumentsAbsents, e.id, msg_vide())
            .await;
        assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
        let r = ent.corriger_message_for_actor(etab, q.user, q.role, &s, msg.id, "Correction").await;
        assert_eq!(r.unwrap_err(), RefusDemande::Introuvable);
        assert_eq!(ent.preuves_demandeur_for_actor(etab, q.user, q.role, &s, e.id).await.unwrap_err(), RefusDemande::Introuvable);
    }
    assert_eq!(w.compte("ent_dem_message", e.id).await, 1);
}

fn msg_vide() -> NouveauMessage {
    msg(Fil::Demandeur, "intrus")
}
