//! Socle 42, lot V0a (sorties et voyages, `docs/socles/42_SORTIES_VOYAGES.md`) contre la base
//! locale de test. Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! * circuit complet d'un voyage facultatif (états, droits à chaque marche, journal, événements) ;
//! * droits et projections par rôle (§21.4) : professeur non porteur, accueil, comptabilité,
//!   famille, autre établissement ;
//! * participants : scolarisés à la date (R-SV2), liste confirmée rouverte à toute modification,
//!   désistement, retrait financier (R-SV19), export tracé et sans injection de formule ;
//! * encadrants du personnel seulement ; séries hebdomadaires (fériés écartés) et validation par
//!   séance (VD-18) ; réouverture, refus, annulation, requalification, passage sans devis ;
//! * correctifs de Next.js : circulaire sans contrôle de droits, envoi aux prestataires (droit
//!   `voyages.prestataires` réservé) ;
//! * journal en ajout seul ; port « en sortie » ; RLS des huit tables.

use super::*;
use crate::ref_commun::Acteur;
use crate::voyages::Commande;
use crate::voyages_repo::{
    EtapeSaisie, FiltreVoyages, ParamsCommande, SerieSaisie, SortieLigne, SortieSaisie,
};
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use ent_auth::{Action, Role};
use uuid::Uuid;

const MDP: &str = "MotDePasse-V0a-Aa9!";

pub(crate) async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

pub(crate) async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let email = format!("v0a-{role}-{}@ent.local", Uuid::new_v4().simple());
    ent.register_user(
        &email,
        MDP,
        Some(&format!(
            "Nom {role} {}",
            &Uuid::new_v4().simple().to_string()[..6]
        )),
        etab,
        role,
    )
    .await
    .expect("compte")
    .id
}

pub(crate) async fn personne_de(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2")
        .bind(etab)
        .bind(user)
        .fetch_one(ent.pool())
        .await
        .expect("fiche personne du compte")
}

pub(crate) async fn agent_de(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
    sqlx::query_scalar(
        "SELECT a.id FROM ent_agent a JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND p.user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .fetch_one(ent.pool())
    .await
    .expect("agent du compte")
}

pub(crate) struct Cadre {
    pub(crate) ent: PgEnt,
    pub(crate) etab: Uuid,
    pub(crate) dir: Uuid,
    pub(crate) sec: Uuid,
    pub(crate) compta: Uuid,
    pub(crate) prof: Uuid,
    pub(crate) prof2: Uuid,
    pub(crate) accueil: Uuid,
    pub(crate) parent: Uuid,
    pub(crate) classe: Uuid,
    pub(crate) eleves: Vec<Uuid>,
    pub(crate) jour: NaiveDate,
}

impl Cadre {
    pub(crate) async fn nouveau() -> Option<Cadre> {
        let ent = connecter().await?;
        let etab = Uuid::new_v4();
        let dir = compte(&ent, etab, "direction").await;
        let sec = compte(&ent, etab, "administratif").await;
        let compta = compte(&ent, etab, "compta").await;
        let prof = compte(&ent, etab, "prof").await;
        let prof2 = compte(&ent, etab, "prof").await;
        let accueil = compte(&ent, etab, "accueil").await;
        let parent = compte(&ent, etab, "parent").await;
        let annee = ent.referentiel_annee_courante_assuree(etab).await.unwrap();
        // Un jour de sortie à l'intérieur de l'année, au plus tôt dans trois jours.
        let jour = (domain_eleve::horloge::jour_paris() + Duration::days(3))
            .clamp(annee.date_debut, annee.date_fin - Duration::days(1));
        let sfx = Uuid::new_v4().simple().to_string()[..6].to_string();
        let classe_texte = format!("V0A{sfx}");
        let classe = ent
            .referentiel_creer_classe(
                "administratif",
                etab,
                Some(annee.id),
                &classe_texte,
                Some("4e"),
                Some("4EME"),
            )
            .await
            .expect("classe");
        let mut eleves = Vec::new();
        for (i, nom) in ["Alpha", "Bravo", "Charlie"].iter().enumerate() {
            let e = ent
                .create_eleve(domain_eleve::CreateEleve {
                    etablissement_id: etab,
                    nom: if i == 2 {
                        "=Formule".to_string()
                    } else {
                        nom.to_string()
                    },
                    prenom: format!("Prénom{i}"),
                    ine: None,
                    sexe: None,
                    date_naissance: None,
                    numero_badge: None,
                    redoublant: false,
                    classe: Some(classe_texte.clone()),
                    regime: None,
                    mef: None,
                    site: None,
                    date_entree: Some(annee.date_debut),
                    date_sortie: None,
                    motif_sortie: None,
                    actor_user_id: Some("v0a".into()),
                })
                .await
                .expect("élève");
            // Affectation à la classe dès le début de l'année (scolarisé à la date de la sortie).
            ent.referentiel_affecter_eleve_classe(
                "administratif",
                etab,
                classe.id,
                e.id,
                Some(annee.date_debut),
            )
            .await
            .expect("affectation");
            eleves.push(e.id);
        }
        sqlx::query(
            "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id) VALUES ($1,$2,$3,$4)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(eleves[0])
        .bind(parent)
        .execute(ent.pool())
        .await
        .expect("lien parent");
        Some(Cadre {
            ent,
            etab,
            dir,
            sec,
            compta,
            prof,
            prof2,
            accueil,
            parent,
            classe: classe.id,
            eleves,
            jour,
        })
    }

    pub(crate) fn a<'x>(&self, user: Uuid, role: Role) -> Acteur<'x> {
        Acteur {
            etablissement_id: self.etab,
            user_id: user,
            role: role.as_str(),
        }
    }

    pub(crate) fn at(&self, h: u32) -> DateTime<Utc> {
        Utc.from_utc_datetime(&self.jour.and_hms_opt(h, 0, 0).unwrap())
    }

    pub(crate) fn saisie(&self, titre: &str, forme: &str) -> SortieSaisie {
        let fin = if forme == "voyage" {
            self.at(8) + Duration::days(2)
        } else {
            self.at(16)
        };
        SortieSaisie {
            titre: titre.into(),
            objectifs: Some("Découvrir".into()),
            nature: "facultative".into(),
            forme: forme.into(),
            destination: "france".into(),
            pays: None,
            destination_libelle: Some("Lieu-dit".into()),
            programme_libelle: None,
            debut: self.at(8),
            fin,
            classes: vec![self.classe],
            groupes: vec![],
            effectif_mode: None,
            places: None,
            effectif_eleves_declare: Some(3),
            effectif_adultes_declare: Some(2),
            besoin_car: false,
            car_sur_place: false,
            point_rendez_vous: Some("Parvis".into()),
            consignes_transport: None,
            places_encadrant_a_pourvoir: 0,
        }
    }

    pub(crate) async fn creer(&self, titre: &str, forme: &str) -> SortieLigne {
        self.ent
            .voyage_creer(&self.a(self.prof, Role::Prof), self.saisie(titre, forme))
            .await
            .expect("création")
    }

    pub(crate) async fn cmd(
        &self,
        user: Uuid,
        role: Role,
        id: Uuid,
        c: Commande,
    ) -> Result<SortieLigne, EntDbError> {
        // Les décisions portent sur la version lue du dossier.
        let version: i32 = sqlx::query_scalar("SELECT version FROM ent_sortie WHERE id = $1")
            .bind(id)
            .fetch_one(self.ent.pool())
            .await
            .unwrap();
        // « Valider le budget » porte sur la version de budget lue : brouillon en cours, sinon validée.
        let (budget_numero, budget_rev) = if matches!(c, Commande::ValiderBudget) {
            sqlx::query_as::<_, (i32, i32)>(
                "SELECT numero, rev FROM ent_sortie_budget_version WHERE sortie_id = $1
                  ORDER BY (etat = 'brouillon') DESC, numero DESC LIMIT 1",
            )
            .bind(id)
            .fetch_optional(self.ent.pool())
            .await
            .unwrap()
            .map_or((None, None), |(n, r)| (Some(n), Some(r)))
        } else {
            (None, None)
        };
        self.ent
            .voyage_commande(
                &self.a(user, role),
                id,
                c,
                ParamsCommande {
                    version: Some(version),
                    budget_numero,
                    budget_rev,
                    ..Default::default()
                },
            )
            .await
    }

    /// Budget minimal équilibré (V0b) : un car de 1 500 € pour trois payants, saisi par la
    /// comptabilité. Retourne (numéro, rev) du brouillon.
    pub(crate) async fn budget_minimal(&self, id: Uuid) -> (i32, i32) {
        let compta = self.a(self.compta, Role::Compta);
        let v = self
            .ent
            .voyage_budget_version_creer(&compta, id, None)
            .await
            .expect("version de budget");
        let b = v.brouillon.expect("brouillon");
        let v = self
            .ent
            .voyage_budget_ligne_ajouter(
                &compta,
                id,
                b.numero,
                b.rev,
                crate::voyages_budget_repo::LigneSaisie {
                    sens: "depense".into(),
                    poste: "transport".into(),
                    libelle: "Car".into(),
                    montant: "1 500".into(),
                    motif: Some("Devis agence".into()),
                    affectation: None,
                },
            )
            .await
            .expect("ligne");
        let b = v.brouillon.expect("brouillon");
        (b.numero, b.rev)
    }

    pub(crate) async fn cmd_motif(
        &self,
        user: Uuid,
        role: Role,
        id: Uuid,
        c: Commande,
        motif: &str,
        vers: Option<&str>,
    ) -> Result<SortieLigne, EntDbError> {
        self.ent
            .voyage_commande(
                &self.a(user, role),
                id,
                c,
                ParamsCommande {
                    version: None,
                    motif: Some(motif.into()),
                    vers: vers.map(String::from),
                    ..Default::default()
                },
            )
            .await
    }

    /// Porteur : ajoute un encadrant responsable et les trois élèves ; soumet.
    pub(crate) async fn preparer_jusqu_a_soumise(&self, id: Uuid) {
        let p2 = agent_de(&self.ent, self.etab, self.prof2).await;
        self.ent
            .voyage_encadrant_ajouter(&self.a(self.prof, Role::Prof), id, p2, "responsable")
            .await
            .expect("encadrant");
        self.cmd(self.prof, Role::Prof, id, Commande::Soumettre)
            .await
            .expect("soumettre");
    }

    /// Mène une sortie sans nuitée facultative à `ouverte`.
    pub(crate) async fn jusqu_a_ouverte(&self, id: Uuid) {
        self.preparer_jusqu_a_soumise(id).await;
        self.cmd(self.dir, Role::Direction, id, Commande::ValiderPedagogie)
            .await
            .unwrap();
        self.cmd(self.prof, Role::Prof, id, Commande::PasserAuxFinances)
            .await
            .unwrap();
        self.cmd(self.dir, Role::Direction, id, Commande::Autoriser)
            .await
            .unwrap();
    }
}

pub(crate) fn est_forbidden(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Forbidden)
}

pub(crate) fn est_introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound))
}

pub(crate) fn est_conflit_detail(e: &EntDbError) -> bool {
    matches!(e, EntDbError::ConflitDossierDetail(_))
}

pub(crate) fn est_refus(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Refus(_, _))
}

pub(crate) async fn actions_journal(c: &Cadre, id: Uuid) -> Vec<String> {
    sqlx::query_scalar("SELECT action FROM ent_sortie_journal WHERE sortie_id = $1 ORDER BY a, id")
        .bind(id)
        .fetch_all(c.ent.pool())
        .await
        .unwrap()
}

async fn codes(c: &Cadre, user: Uuid, role: Role, id: Uuid) -> Vec<String> {
    let mut v: Vec<String> = c
        .ent
        .voyage_detail(&c.a(user, role), id)
        .await
        .unwrap()
        .commandes
        .into_iter()
        .map(|x| x.code.to_string())
        .collect();
    v.sort();
    v
}

// ─────────────────────────── Circuit complet ───────────────────────────

#[tokio::test]
async fn voyage_facultatif_parcourt_le_circuit_avec_les_bons_droits() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Voyage à Rome", "voyage").await;
    assert_eq!(s.etat, "brouillon");
    assert_eq!(s.version, 1);
    assert!(s.reference.starts_with("VOY-"));
    // Le serveur propose au porteur de soumettre et d'annuler ; un professeur tiers ne voit rien.
    assert_eq!(
        codes(&c, c.prof, Role::Prof, s.id).await,
        vec!["annuler", "soumettre"]
    );
    // La direction (qui a `voyages.modifier`) peut aussi soumettre un brouillon et le requalifier.
    assert_eq!(
        codes(&c, c.dir, Role::Direction, s.id).await,
        vec!["annuler", "requalifier", "soumettre"]
    );

    // Soumission refusée tant qu'il manque un encadrant : le message dit quoi.
    let e = c
        .cmd(c.prof, Role::Prof, s.id, Commande::Soumettre)
        .await
        .unwrap_err();
    assert!(est_conflit_detail(&e), "{e:?}");
    // Un professeur qui n'est pas porteur ne peut ni voir ni soumettre (introuvable, pas « refusé »).
    let e = c
        .cmd(c.prof2, Role::Prof, s.id, Commande::Soumettre)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");

    c.preparer_jusqu_a_soumise(s.id).await;
    let d = c
        .ent
        .voyage_detail(&c.a(c.prof, Role::Prof), s.id)
        .await
        .unwrap();
    assert_eq!(d.sortie.etat, "soumise");
    assert_eq!(d.prochaine_etape.libelle.is_empty(), false);
    assert!(d.acces.porteur && !d.acces.valider);
    // Soumise : le porteur ne peut que l'annuler ; la direction valide, demande, refuse.
    assert_eq!(codes(&c, c.prof, Role::Prof, s.id).await, vec!["annuler"]);
    let cd = codes(&c, c.dir, Role::Direction, s.id).await;
    for attendu in [
        "valider-pedagogie",
        "demander-modifications",
        "refuser",
        "annuler",
    ] {
        assert!(cd.contains(&attendu.to_string()), "{attendu} : {cd:?}");
    }
    assert!(!cd.contains(&"soumettre".to_string()));

    // Le porteur ne valide pas sa propre pédagogie ; la direction oui.
    let e = c
        .cmd(c.prof, Role::Prof, s.id, Commande::ValiderPedagogie)
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    let s = c
        .cmd(c.dir, Role::Direction, s.id, Commande::ValiderPedagogie)
        .await
        .unwrap();
    assert_eq!(s.etat, "preparation");

    // Voyage : passage aux finances, validé par la comptabilité (pas par la direction).
    let s = c
        .cmd(c.prof, Role::Prof, s.id, Commande::PasserAuxFinances)
        .await
        .unwrap();
    assert_eq!(s.etat, "budget_a_valider");
    // Le budget se valide à la comptabilité seule, pas à la direction.
    assert!(codes(&c, c.compta, Role::Compta, s.id)
        .await
        .contains(&"valider-budget".to_string()));
    assert!(!codes(&c, c.dir, Role::Direction, s.id)
        .await
        .contains(&"valider-budget".to_string()));
    let e = c
        .cmd(c.dir, Role::Direction, s.id, Commande::ValiderBudget)
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    // V0b : sans version de budget lue, la comptabilité ne valide rien.
    let e = c
        .cmd(c.compta, Role::Compta, s.id, Commande::ValiderBudget)
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    c.budget_minimal(s.id).await;
    let s = c
        .cmd(c.compta, Role::Compta, s.id, Commande::ValiderBudget)
        .await
        .unwrap();
    assert_eq!(s.etat, "a_autoriser");

    // Validation finale : la liste n'est pas confirmée, donc « ouverte » (NX-08).
    let e = c
        .cmd(c.sec, Role::Administratif, s.id, Commande::Autoriser)
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    let s = c
        .cmd(c.dir, Role::Direction, s.id, Commande::Autoriser)
        .await
        .unwrap();
    assert_eq!(s.etat, "ouverte");
    // Ouverte : le porteur confirme la liste ; la direction peut rouvrir.
    assert!(codes(&c, c.prof, Role::Prof, s.id)
        .await
        .contains(&"confirmer-liste".to_string()));
    let cd = codes(&c, c.dir, Role::Direction, s.id).await;
    assert!(cd.contains(&"rouvrir".to_string()), "{cd:?}");
    let rouvrir = c
        .ent
        .voyage_detail(&c.a(c.dir, Role::Direction), s.id)
        .await
        .unwrap();
    let r = rouvrir
        .commandes
        .iter()
        .find(|x| x.code == "rouvrir")
        .unwrap();
    assert!(
        r.motif_requis && r.vers.len() == 5,
        "cinq étapes amont (NX-83)"
    );

    // Choix des élèves dans le registre.
    let r = c
        .ent
        .voyage_participants_ajouter(&c.a(c.prof, Role::Prof), s.id, c.eleves.clone())
        .await
        .unwrap();
    assert_eq!(r.ajoutes.len(), 3);
    // Confirmation refusée : un voyage exige dépôt et reprise (NX-28).
    let e = c
        .cmd(c.prof, Role::Prof, s.id, Commande::ConfirmerListe)
        .await
        .unwrap_err();
    assert!(est_conflit_detail(&e), "{e:?}");
    c.ent
        .voyage_etapes_remplacer(
            &c.a(c.prof, Role::Prof),
            s.id,
            vec![
                EtapeSaisie {
                    type_etape: "reprise".into(),
                    debut: c.at(8) + Duration::days(2),
                    fin: None,
                    duree_minutes: None,
                    lieu: Some("Parvis".into()),
                    note: None,
                    appel_requis: false,
                },
                EtapeSaisie {
                    type_etape: "depot".into(),
                    debut: c.at(7),
                    fin: None,
                    duree_minutes: Some(30),
                    lieu: Some("Parvis".into()),
                    note: None,
                    appel_requis: true,
                },
            ],
        )
        .await
        .unwrap();
    let s = c
        .cmd(c.prof, Role::Prof, s.id, Commande::ConfirmerListe)
        .await
        .unwrap();
    assert_eq!(s.etat, "prete");
    assert!(s.liste_confirmee_le.is_some());

    // Une modification de la liste la rouvre (NX-26) et ramène la sortie à « ouverte ».
    c.ent
        .voyage_participants_ajouter(&c.a(c.prof, Role::Prof), s.id, vec![c.eleves[0]])
        .await
        .unwrap();
    let r = c
        .ent
        .voyage_participant_retirer(
            &c.a(c.prof, Role::Prof),
            s.id,
            c.ent
                .voyage_participants(&c.a(c.prof, Role::Prof), s.id)
                .await
                .unwrap()[0]
                .id,
            "exigence_manquante",
            None,
        )
        .await;
    assert!(r.is_ok(), "{r:?}");
    let d = c
        .ent
        .voyage_detail(&c.a(c.prof, Role::Prof), s.id)
        .await
        .unwrap();
    assert_eq!(d.sortie.etat, "ouverte");
    assert!(d.sortie.liste_confirmee_le.is_none());

    // Journal complet, ajout seul, dans l'ordre.
    let j = actions_journal(&c, s.id).await;
    for attendu in [
        "creee",
        "encadrant_ajoute",
        "soumise",
        "validee_pedagogie",
        "passee_aux_finances",
        "budget_valide",
        "autorisee",
        "participant_inscrit",
        "programme_modifie",
        "liste_confirmee",
        "participant_retire",
        "liste_rouverte",
    ] {
        assert!(
            j.iter().any(|a| a == attendu),
            "journal sans « {attendu} » : {j:?}"
        );
    }
    let pos = |a: &str| j.iter().position(|x| x == a).unwrap();
    assert!(
        pos("soumise") < pos("validee_pedagogie") && pos("validee_pedagogie") < pos("autorisee")
    );

    // Événements dans la même transaction, sans identité d'élève.
    let (n, avec_eleve): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(eleve_id) FROM ent_metier_event
          WHERE etablissement_id = $1 AND aggregate_id = $2 AND type LIKE 'voyages.%'",
    )
    .bind(c.etab)
    .bind(s.id)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert!(n >= 10, "{n} événements voyages.*");
    assert_eq!(avec_eleve, 0, "aucun événement ne porte d'identité d'élève");
}

#[tokio::test]
async fn sortie_obligatoire_proximite_circuit_court() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let mut sa = c.saisie("Visite du collège voisin", "sans_nuitee");
    sa.nature = "obligatoire".into();
    sa.destination = "proximite".into();
    let s = c
        .ent
        .voyage_creer(&c.a(c.prof, Role::Prof), sa)
        .await
        .unwrap();
    c.preparer_jusqu_a_soumise(s.id).await;
    // Une seule validation de la direction suffit (§8.2) : pas de préparation, pas de budget.
    let s = c
        .cmd(c.dir, Role::Direction, s.id, Commande::ValiderPedagogie)
        .await
        .unwrap();
    assert_eq!(s.etat, "a_autoriser");
    let s = c
        .cmd(c.dir, Role::Direction, s.id, Commande::Autoriser)
        .await
        .unwrap();
    assert_eq!(s.etat, "ouverte");
}

#[tokio::test]
async fn demande_de_modifications_refus_annulation_reouverture() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Cinéma", "sans_nuitee").await;
    c.preparer_jusqu_a_soumise(s.id).await;

    // Modifications demandées : motif obligatoire, retour mémorisé.
    let e = c
        .cmd(
            c.dir,
            Role::Direction,
            s.id,
            Commande::DemanderModifications,
        )
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    let s = c
        .cmd_motif(
            c.dir,
            Role::Direction,
            s.id,
            Commande::DemanderModifications,
            "Précisez les objectifs",
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        (s.etat.as_str(), s.etat_retour.as_deref()),
        ("a_completer", Some("soumise"))
    );
    assert_eq!(s.motif_decision.as_deref(), Some("Précisez les objectifs"));
    // Le porteur renvoie : retour à l'étape d'origine.
    let s = c
        .cmd(c.prof, Role::Prof, s.id, Commande::Renvoyer)
        .await
        .unwrap();
    assert_eq!((s.etat.as_str(), s.etat_retour), ("soumise", None));
    // Demande puis annulation de la demande par la direction.
    let s = c
        .cmd_motif(
            c.dir,
            Role::Direction,
            s.id,
            Commande::DemanderModifications,
            "À revoir",
            None,
        )
        .await
        .unwrap();
    let s = c
        .cmd(c.dir, Role::Direction, s.id, Commande::AnnulerDemande)
        .await
        .unwrap();
    assert_eq!(s.etat, "soumise");

    // Refus : motif obligatoire, direction seule.
    let e = c
        .cmd_motif(c.prof, Role::Prof, s.id, Commande::Refuser, "non", None)
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    let e = c
        .cmd(c.dir, Role::Direction, s.id, Commande::Refuser)
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    let refusee = c
        .cmd_motif(
            c.dir,
            Role::Direction,
            s.id,
            Commande::Refuser,
            "Hors programme",
            None,
        )
        .await
        .unwrap();
    assert_eq!(refusee.etat, "refusee");
    // Un dossier refusé ne bouge plus.
    let e = c
        .cmd(c.dir, Role::Direction, s.id, Commande::ValiderPedagogie)
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");

    // Annulation : le porteur avant l'ouverture, la direction après.
    let t = c.creer("Musée", "sans_nuitee").await;
    let t = c
        .cmd_motif(
            c.prof,
            Role::Prof,
            t.id,
            Commande::Annuler,
            "Plus de car",
            None,
        )
        .await
        .unwrap();
    assert_eq!(t.etat, "annulee");
    let u = c.creer("Théâtre", "sans_nuitee").await;
    c.jusqu_a_ouverte(u.id).await;
    let e = c
        .cmd_motif(c.prof, Role::Prof, u.id, Commande::Annuler, "x", None)
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    // Réouverture vers une étape amont (cinq possibles, NX-83) ; une étape inconnue est refusée.
    let e = c
        .cmd_motif(
            c.dir,
            Role::Direction,
            u.id,
            Commande::Rouvrir,
            "erreur d'horaire",
            Some("prete"),
        )
        .await
        .unwrap_err();
    assert!(matches!(e, EntDbError::Domain(_)), "{e:?}");
    let r = c
        .cmd_motif(
            c.dir,
            Role::Direction,
            u.id,
            Commande::Rouvrir,
            "erreur d'horaire",
            Some("a_signer"),
        )
        .await
        .unwrap();
    assert_eq!(r.etat, "a_signer");
    let a = c
        .cmd_motif(
            c.dir,
            Role::Direction,
            u.id,
            Commande::Annuler,
            "Grève des transports",
            None,
        )
        .await
        .unwrap();
    assert_eq!(a.etat, "annulee");
    assert_eq!(a.motif_decision.as_deref(), Some("Grève des transports"));
}

#[tokio::test]
async fn requalification_et_passage_sans_devis_signe() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Sortie en forêt", "sans_nuitee").await;
    c.preparer_jusqu_a_soumise(s.id).await;
    c.cmd(c.dir, Role::Direction, s.id, Commande::ValiderPedagogie)
        .await
        .unwrap();
    // La direction seule requalifie en « avec car » ; le dossier retourne en préparation.
    let e = c
        .cmd(c.prof, Role::Prof, s.id, Commande::Requalifier)
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    let s = c
        .cmd(c.dir, Role::Direction, s.id, Commande::Requalifier)
        .await
        .unwrap();
    assert!(s.besoin_car && s.etat == "preparation");
    // Avec un car, on ne saute pas la signature du devis.
    let e = c
        .cmd(c.prof, Role::Prof, s.id, Commande::PasserAuxFinances)
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    let s = c
        .cmd(c.prof, Role::Prof, s.id, Commande::DemanderSignature)
        .await
        .unwrap();
    assert_eq!(s.etat, "a_signer");
    // Passage sans devis signé : direction, motif obligatoire, tracé (NX-07).
    let e = c
        .cmd(c.dir, Role::Direction, s.id, Commande::PasserAuxFinances)
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    let e = c
        .cmd_motif(
            c.prof,
            Role::Prof,
            s.id,
            Commande::PasserAuxFinances,
            "vite",
            None,
        )
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    let s = c
        .cmd_motif(
            c.dir,
            Role::Direction,
            s.id,
            Commande::PasserAuxFinances,
            "Transporteur habituel, devis oral",
            None,
        )
        .await
        .unwrap();
    assert_eq!(s.etat, "budget_a_valider");
    assert!(s.contournement_devis_motif.is_some());
    let d = c
        .ent
        .voyage_detail(&c.a(c.dir, Role::Direction), s.id)
        .await
        .unwrap();
    assert!(d.audit.iter().any(|p| p.code == "devis_contourne"));
}

// ─────────────────────────── Droits et projections ───────────────────────────

#[tokio::test]
async fn droits_et_projections_par_role() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let brouillon = c.creer("Brouillon secret", "sans_nuitee").await;
    let ouverte = c.creer("Sortie ouverte", "sans_nuitee").await;
    c.jusqu_a_ouverte(ouverte.id).await;

    // Professeur non porteur : ni détail, ni brouillon, mais la liste des sorties ouvertes, minimale.
    let p2 = c.a(c.accueil, Role::Accueil);
    let _ = p2;
    let autre = compte(&c.ent, c.etab, "prof").await;
    let autre_a = c.a(autre, Role::Prof);
    let e = c
        .ent
        .voyage_detail(&autre_a, brouillon.id)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    let liste = c
        .ent
        .voyages_liste(&autre_a, FiltreVoyages::default())
        .await
        .unwrap();
    assert!(
        liste.iter().all(|s| s.id != brouillon.id),
        "brouillon d'un autre visible"
    );
    let vue = liste
        .iter()
        .find(|s| s.id == ouverte.id)
        .expect("sortie ouverte visible");
    assert!(!vue.acces_detail && vue.porteur_nom.is_none() && vue.nb_inscrits.is_none());
    let e = c.ent.voyage_detail(&autre_a, ouverte.id).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    let e = c
        .ent
        .voyage_participants(&autre_a, ouverte.id)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // Il ne peut pas non plus modifier, ni gérer l'encadrement.
    let e = c
        .ent
        .voyage_modifier(&autre_a, ouverte.id, 1, c.saisie("Piraté", "sans_nuitee"))
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");

    // Le porteur voit tout de la sienne, y compris son brouillon et la liste complète.
    let liste = c
        .ent
        .voyages_liste(&c.a(c.prof, Role::Prof), FiltreVoyages::default())
        .await
        .unwrap();
    assert!(liste.iter().any(|s| s.id == brouillon.id && s.est_porteur));
    let mes = c
        .ent
        .voyages_liste(
            &c.a(c.prof, Role::Prof),
            FiltreVoyages {
                mes: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(mes.iter().all(|s| s.est_porteur || s.acces_detail));

    // L'accueil : liste minimale, aucun droit de création, rien du détail.
    let acc = c.a(c.accueil, Role::Accueil);
    let e = c
        .ent
        .voyage_creer(&acc, c.saisie("Accueil", "sans_nuitee"))
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    let e = c.ent.voyage_detail(&acc, ouverte.id).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");

    // Comptabilité : lit le détail de toutes les sorties, ne modifie pas.
    let cpt = c.a(c.compta, Role::Compta);
    assert!(c.ent.voyage_detail(&cpt, brouillon.id).await.is_ok());
    let e = c
        .ent
        .voyage_modifier(&cpt, brouillon.id, 1, c.saisie("Compta", "sans_nuitee"))
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");

    // Secrétariat : peut tout modifier (verrou optimiste) et change le porteur.
    let sec = c.a(c.sec, Role::Administratif);
    let mut m = c.saisie("Titre corrigé par le secrétariat", "sans_nuitee");
    m.classes = vec![c.classe];
    let apres = c
        .ent
        .voyage_modifier(&sec, brouillon.id, brouillon.version, m.clone())
        .await
        .unwrap();
    assert_eq!(apres.titre, "Titre corrigé par le secrétariat");
    assert_eq!(apres.version, brouillon.version + 1);
    // Version périmée : refusée (les écritures concurrentes ne s'écrasent plus).
    let e = c
        .ent
        .voyage_modifier(&sec, brouillon.id, brouillon.version, m)
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    let nouveau = agent_de(&c.ent, c.etab, autre).await;
    let r = c
        .ent
        .voyage_changer_porteur(&sec, brouillon.id, nouveau)
        .await
        .unwrap();
    assert_eq!(
        r.porteur_personne_id,
        personne_de(&c.ent, c.etab, autre).await
    );
    let e = c
        .ent
        .voyage_changer_porteur(&c.a(c.prof, Role::Prof), ouverte.id, nouveau)
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    // Un porteur ne peut pas être un adulte hors du personnel : un agent inconnu est refusé.
    assert!(c
        .ent
        .voyage_changer_porteur(&sec, ouverte.id, Uuid::new_v4())
        .await
        .is_err());
    // Le journal des modifications garde l'ancienne valeur.
    let j = c.ent.voyage_journal(&sec, brouillon.id).await.unwrap();
    assert!(j
        .iter()
        .any(|l| l.champ.as_deref() == Some("titre")
            && l.avant.as_deref() == Some("Brouillon secret")));
    // Journal : pas pour un professeur étranger au dossier.
    let e = c
        .ent
        .voyage_journal(&autre_a, ouverte.id)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
}

#[tokio::test]
async fn un_autre_etablissement_ne_voit_rien() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Sortie A", "sans_nuitee").await;
    let etab_b = Uuid::new_v4();
    let dir_b = compte(&c.ent, etab_b, "direction").await;
    let b = Acteur {
        etablissement_id: etab_b,
        user_id: dir_b,
        role: "direction",
    };
    let e = c.ent.voyage_detail(&b, s.id).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    assert!(c
        .ent
        .voyages_liste(&b, FiltreVoyages::default())
        .await
        .unwrap()
        .is_empty());
    let e = c
        .ent
        .voyage_commande(
            &b,
            s.id,
            Commande::ValiderPedagogie,
            ParamsCommande::default(),
        )
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    let e = c
        .ent
        .voyage_participants_ajouter(&b, s.id, c.eleves.clone())
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // Et la direction de B ne peut pas viser une classe de A.
    let mut sa = c.saisie("Sortie B", "sans_nuitee");
    sa.classes = vec![c.classe];
    c.ent
        .referentiel_annee_courante_assuree(etab_b)
        .await
        .unwrap();
    let e = c.ent.voyage_creer(&b, sa).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // Le port « en sortie » ne mélange pas les établissements.
    let v = c
        .ent
        .voyages_eleves_en_sortie(
            etab_b,
            c.at(0) - Duration::days(1),
            c.at(23) + Duration::days(5),
        )
        .await
        .unwrap();
    assert!(v.is_empty());
}

#[tokio::test]
async fn famille_ne_voit_que_son_enfant() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Sortie à la mer", "sans_nuitee").await;
    c.jusqu_a_ouverte(s.id).await;
    c.ent
        .voyage_participants_ajouter(&c.a(c.prof, Role::Prof), s.id, c.eleves.clone())
        .await
        .unwrap();
    let parent = c.a(c.parent, Role::Parent);
    // Le parent d'Alpha voit la sortie, pour son seul enfant ; pas de liste ni de détail.
    let v = c.ent.voyages_famille(&parent).await.unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].enfant_prenom, "Prénom0");
    assert_eq!(v[0].titre, "Sortie à la mer");
    assert!(c
        .ent
        .voyages_liste(&parent, FiltreVoyages::default())
        .await
        .is_err());
    let e = c.ent.voyage_detail(&parent, s.id).await.unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    // Désisté : disparaît de la projection famille.
    let pid = c
        .ent
        .voyage_participants(&c.a(c.prof, Role::Prof), s.id)
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.eleve_id == c.eleves[0])
        .unwrap()
        .id;
    c.ent
        .voyage_participant_desister(&c.a(c.prof, Role::Prof), s.id, pid, "famille", None)
        .await
        .unwrap();
    assert!(c.ent.voyages_famille(&parent).await.unwrap().is_empty());
    // Un brouillon n'est jamais visible d'une famille, même avec un participant.
    let b = c.creer("Futur projet", "sans_nuitee").await;
    c.ent
        .voyage_participants_ajouter(&c.a(c.prof, Role::Prof), b.id, vec![c.eleves[1]])
        .await
        .unwrap();
    assert!(c.ent.voyages_famille(&parent).await.unwrap().is_empty());
}

// ─────────────────────────── Participants et encadrants ───────────────────────────

#[tokio::test]
async fn participants_scolarises_a_la_date_et_sans_doublon() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Sortie", "sans_nuitee").await;
    let prof = c.a(c.prof, Role::Prof);
    // Un élève d'un autre établissement est refusé (R-SV2), un inconnu aussi.
    let autre_etab = Uuid::new_v4();
    let dans_b = c
        .ent
        .create_eleve(domain_eleve::CreateEleve {
            etablissement_id: autre_etab,
            nom: "Autre".into(),
            prenom: "Etab".into(),
            ine: None,
            sexe: None,
            date_naissance: None,
            numero_badge: None,
            redoublant: false,
            classe: Some("X".into()),
            regime: None,
            mef: None,
            site: None,
            date_entree: None,
            date_sortie: None,
            motif_sortie: None,
            actor_user_id: Some("v0a".into()),
        })
        .await
        .unwrap()
        .id;
    let r = c
        .ent
        .voyage_participants_ajouter(
            &prof,
            s.id,
            vec![c.eleves[0], dans_b, Uuid::new_v4(), c.eleves[0]],
        )
        .await
        .unwrap();
    assert_eq!(r.ajoutes, vec![c.eleves[0]]);
    assert_eq!(r.refuses.len(), 2);
    assert!(r.refuses.contains(&dans_b));
    // Deuxième ajout : « déjà », pas de doublon en base.
    let r = c
        .ent
        .voyage_participants_ajouter(&prof, s.id, vec![c.eleves[0]])
        .await
        .unwrap();
    assert_eq!((r.ajoutes.len(), r.deja.len()), (0, 1));
    // Élève dont l'affectation à la classe a pris fin avant la sortie : plus scolarisé à cette date.
    sqlx::query(
        "UPDATE ent_affectation SET date_fin = $2 WHERE etablissement_id = $1 AND eleve_id = $3",
    )
    .bind(c.etab)
    .bind(c.jour - Duration::days(1))
    .bind(c.eleves[1])
    .execute(c.ent.pool())
    .await
    .unwrap();
    let r = c
        .ent
        .voyage_participants_ajouter(&prof, s.id, vec![c.eleves[1]])
        .await
        .unwrap();
    assert_eq!(
        r.refuses,
        vec![c.eleves[1]],
        "élève parti avant la date de la sortie"
    );
    // Registre des élèves : classe visée, nom et prénom seulement ; consultation tracée.
    let cand = c.ent.voyage_candidats(&prof, s.id, None).await.unwrap();
    assert_eq!(cand.len(), 1);
    assert!(cand[0]
        .eleves
        .iter()
        .any(|e| e.eleve_id == c.eleves[0] && e.deja_inscrit));
    assert!(
        !cand[0].eleves.iter().any(|e| e.eleve_id == c.eleves[1]),
        "élève parti proposé"
    );
    assert!(actions_journal(&c, s.id)
        .await
        .iter()
        .any(|a| a == "registre_consulte"));
    // Un autre professeur ne consulte pas le registre pour cette sortie.
    let e = c
        .ent
        .voyage_candidats(&c.a(c.prof2, Role::Prof), s.id, None)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
}

#[tokio::test]
async fn desistement_retrait_et_motif_financier() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Séjour", "sans_nuitee").await;
    let prof = c.a(c.prof, Role::Prof);
    c.ent
        .voyage_participants_ajouter(&prof, s.id, vec![c.eleves[0], c.eleves[1], c.eleves[2]])
        .await
        .unwrap();
    let liste = c.ent.voyage_participants(&prof, s.id).await.unwrap();
    let pid = |i: usize| liste.iter().find(|p| p.eleve_id == c.eleves[i]).unwrap().id;
    // Motif inconnu refusé.
    assert!(c
        .ent
        .voyage_participant_desister(&prof, s.id, pid(0), "caprice", None)
        .await
        .is_err());
    // Désistement daté, avec motif codé ; une seconde fois : conflit.
    c.ent
        .voyage_participant_desister(&prof, s.id, pid(0), "sante", Some("certificat"))
        .await
        .unwrap();
    assert!(c
        .ent
        .voyage_participant_desister(&prof, s.id, pid(0), "sante", None)
        .await
        .is_err());
    let (etat, avant, apres, code): (String, Option<String>, String, Option<String>) = sqlx::query_as(
        "SELECT p.etat, f.etat_avant, f.etat_apres, f.motif_code
           FROM ent_sortie_participant p JOIN ent_sortie_participant_fait f ON f.participant_id = p.id
          WHERE p.id = $1 AND f.type = 'desistement'",
    )
    .bind(pid(0))
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(
        (
            etat.as_str(),
            avant.as_deref(),
            apres.as_str(),
            code.as_deref()
        ),
        ("desiste", Some("inscrit"), "desiste", Some("sante"))
    );
    // Retrait pour raison financière : refusé au porteur et au secrétariat…
    for (u, r) in [(c.prof, Role::Prof), (c.sec, Role::Administratif)] {
        let e = c
            .ent
            .voyage_participant_retirer(&c.a(u, r), s.id, pid(1), "financier", Some("impayé"))
            .await
            .unwrap_err();
        assert!(est_refus(&e), "{e:?}");
    }
    // … et à la direction elle-même sans décision motivée ; avec motif, c'est tracé.
    let dir = c.a(c.dir, Role::Direction);
    let e = c
        .ent
        .voyage_participant_retirer(&dir, s.id, pid(1), "financier", None)
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    c.ent
        .voyage_participant_retirer(
            &dir,
            s.id,
            pid(1),
            "financier",
            Some("Décision motivée après échange avec la famille"),
        )
        .await
        .unwrap();
    // Retiré puis réinscrit : réintégration tracée.
    let r = c
        .ent
        .voyage_participants_ajouter(&prof, s.id, vec![c.eleves[1]])
        .await
        .unwrap();
    assert_eq!(r.ajoutes, vec![c.eleves[1]]);
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_sortie_participant_fait WHERE participant_id = $1 AND type = 'reintegration'",
    )
    .bind(pid(1))
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn export_trace_et_sans_formule() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Export", "sans_nuitee").await;
    let prof = c.a(c.prof, Role::Prof);
    c.ent
        .voyage_participants_ajouter(&prof, s.id, c.eleves.clone())
        .await
        .unwrap();
    let csv = c.ent.voyage_participants_csv(&prof, s.id).await.unwrap();
    assert!(csv.starts_with('\u{feff}'));
    assert!(csv.contains("Nom;Prénom;Classe"));
    // Le nom « =Formule » est neutralisé (pas d'exécution dans le tableur).
    assert!(csv.contains("'=Formule"), "{csv}");
    assert!(!csv.contains(";=") && !csv.contains("\r\n="));
    assert!(actions_journal(&c, s.id)
        .await
        .iter()
        .any(|a| a == "liste_exportee"));
    // Un simple encadrant n'exporte pas la liste.
    let p2 = agent_de(&c.ent, c.etab, c.prof2).await;
    c.ent
        .voyage_encadrant_ajouter(&prof, s.id, p2, "encadrant")
        .await
        .unwrap();
    let enc = c.a(c.prof2, Role::Prof);
    assert!(
        c.ent.voyage_detail(&enc, s.id).await.is_ok(),
        "l'encadrant lit sa sortie"
    );
    let e = c.ent.voyage_participants_csv(&enc, s.id).await.unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    // La direction exporte (droit de lecture) : l'export est journalisé à chaque fois.
    c.ent
        .voyage_participants_csv(&c.a(c.dir, Role::Direction), s.id)
        .await
        .unwrap();
    let n = actions_journal(&c, s.id)
        .await
        .iter()
        .filter(|a| *a == "liste_exportee")
        .count();
    assert_eq!(n, 2);
}

#[tokio::test]
async fn encadrants_du_personnel_seulement() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Encadrement", "sans_nuitee").await;
    let prof = c.a(c.prof, Role::Prof);
    // Seuls les agents du personnel encadrent (VD-19 : jamais un nom libre ni un tiers) : un
    // identifiant inconnu est refusé.
    assert!(c
        .ent
        .voyage_encadrant_ajouter(&prof, s.id, Uuid::new_v4(), "encadrant")
        .await
        .is_err());
    // Une personne d'un autre établissement : introuvable ou refusée.
    let autre = Uuid::new_v4();
    let dir_b = compte(&c.ent, autre, "prof").await;
    let pb = agent_de(&c.ent, autre, dir_b).await;
    assert!(c
        .ent
        .voyage_encadrant_ajouter(&prof, s.id, pb, "encadrant")
        .await
        .is_err());
    // Un agent du personnel : accepté ; doublon : conflit ; rôle inconnu refusé.
    let p2 = agent_de(&c.ent, c.etab, c.prof2).await;
    assert!(c
        .ent
        .voyage_encadrant_ajouter(&prof, s.id, p2, "chef")
        .await
        .is_err());
    let enc = c
        .ent
        .voyage_encadrant_ajouter(&prof, s.id, p2, "responsable")
        .await
        .unwrap();
    assert_eq!(enc.role, "responsable");
    assert!(c
        .ent
        .voyage_encadrant_ajouter(&prof, s.id, p2, "encadrant")
        .await
        .is_err());
    // Un non-porteur sans droit d'encadrement ne retire pas : introuvable.
    let tiers = compte(&c.ent, c.etab, "prof").await;
    let e = c
        .ent
        .voyage_encadrant_retirer(&c.a(tiers, Role::Prof), s.id, enc.id)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    c.ent
        .voyage_encadrant_retirer(&prof, s.id, enc.id)
        .await
        .unwrap();
    assert!(c
        .ent
        .voyage_encadrant_retirer(&prof, s.id, enc.id)
        .await
        .is_err());
    // L'audit signale à nouveau l'absence d'encadrant.
    let d = c.ent.voyage_detail(&prof, s.id).await.unwrap();
    assert!(d.audit.iter().any(|p| p.code == "encadrant_absent"));
}

// ─────────────────────────── Séries ───────────────────────────

#[tokio::test]
async fn serie_hebdomadaire_ecarte_les_feries_et_se_valide_seance_par_seance() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    // Quatre semaines à partir du jour de la sortie, le même jour de la semaine.
    let du = c.jour;
    let au = c.jour + Duration::days(27);
    let ferie = c.jour + Duration::days(14);
    sqlx::query("INSERT INTO ent_jour_calendrier (id, etablissement_id, jour, kind, libelle) VALUES ($1,$2,$3,'ferie','Férié test')")
        .bind(Uuid::new_v4())
        .bind(c.etab)
        .bind(ferie)
        .execute(c.ent.pool())
        .await
        .unwrap();
    let mut modele = c.saisie("Atelier théâtre", "sans_nuitee");
    modele.effectif_eleves_declare = Some(3);
    let serie = SerieSaisie {
        libelle: "Atelier du mardi".into(),
        jour_semaine: chrono::Datelike::weekday(&du).number_from_monday(),
        heure_debut: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
        heure_fin: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
        du,
        au,
        modele,
    };
    let prof = c.a(c.prof, Role::Prof);
    // Il faut le droit de création.
    assert!(c
        .ent
        .voyage_serie_creer(&c.a(c.accueil, Role::Accueil), serie.clone())
        .await
        .is_err());
    let r = c.ent.voyage_serie_creer(&prof, serie).await.unwrap();
    assert_eq!(r.sorties.len(), 3, "4 semaines moins le jour férié");
    assert_eq!(r.ecartees.len(), 1);
    assert_eq!(r.ecartees[0].jour, ferie);
    assert_eq!(r.ecartees[0].motif, "ferie");
    assert!(r
        .sorties
        .iter()
        .all(|s| s.serie_id == Some(r.serie_id) && s.etat == "brouillon"));
    // Chaque séance est une sortie qui a son journal.
    for s in &r.sorties {
        assert!(actions_journal(&c, s.id)
            .await
            .contains(&"creee".to_string()));
    }
    // Valider toute la série : la direction doit avoir soumis les séances (sinon, rien n'est validé,
    // séance par séance) ; un professeur n'a pas le droit pour aucune d'elles (VD-18).
    let e = c
        .ent
        .voyage_serie_valider(&prof, r.serie_id)
        .await
        .unwrap_err();
    assert!(est_forbidden(&e), "{e:?}");
    // Préparer et soumettre deux séances sur trois.
    let p2 = agent_de(&c.ent, c.etab, c.prof2).await;
    for s in r.sorties.iter().take(2) {
        c.ent
            .voyage_encadrant_ajouter(&prof, s.id, p2, "responsable")
            .await
            .unwrap();
        c.cmd(c.prof, Role::Prof, s.id, Commande::Soumettre)
            .await
            .unwrap();
    }
    let res = c
        .ent
        .voyage_serie_valider(&c.a(c.dir, Role::Direction), r.serie_id)
        .await
        .unwrap();
    assert_eq!(res.iter().filter(|x| x.ok).count(), 2);
    assert_eq!(res.iter().filter(|x| !x.ok).count(), 1);
    assert!(res.iter().find(|x| !x.ok).unwrap().raison.is_some());
    // Annuler une séance : seule celle-ci passe à « annulée ».
    let une = r.sorties[0].id;
    // Le porteur annule une séance avant l'ouverture (NX-10) ; un professeur tiers n'y a pas accès.
    let tiers = compte(&c.ent, c.etab, "prof").await;
    let e = c
        .cmd_motif(
            tiers,
            Role::Prof,
            une,
            Commande::Annuler,
            "Intervenant absent",
            None,
        )
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    c.cmd_motif(
        c.prof,
        Role::Prof,
        une,
        Commande::Annuler,
        "Intervenant absent",
        None,
    )
    .await
    .unwrap();
    let etats: Vec<String> =
        sqlx::query_scalar("SELECT etat FROM ent_sortie WHERE serie_id = $1 ORDER BY debut")
            .bind(r.serie_id)
            .fetch_all(c.ent.pool())
            .await
            .unwrap();
    assert_eq!(etats, vec!["annulee", "preparation", "brouillon"]);
    // Les exclusions sont en base, typées.
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_sortie_serie_exclusion WHERE serie_id = $1")
            .bind(r.serie_id)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn serie_ecarte_aussi_les_periodes_de_vacances_et_refuse_une_serie_vide() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let prof = c.a(c.prof, Role::Prof);
    let jour_sem = chrono::Datelike::weekday(&c.jour).number_from_monday();
    // Une semaine de vacances (période du calendrier de l'établissement) autour de la 2e séance.
    sqlx::query(
        "INSERT INTO ent_periode_vacances (id, etablissement_id, libelle, kind, debut, fin, created_by_user_id)
         VALUES ($1,$2,'Vacances test','vacances',$3,$4,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(c.etab)
    .bind(c.jour + Duration::days(5))
    .bind(c.jour + Duration::days(11))
    .bind(c.dir)
    .execute(c.ent.pool())
    .await
    .unwrap();
    let mk = |du: NaiveDate, au: NaiveDate| SerieSaisie {
        libelle: "Atelier".into(),
        jour_semaine: jour_sem,
        heure_debut: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
        heure_fin: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
        du,
        au,
        modele: c.saisie("Atelier", "sans_nuitee"),
    };
    let r = c
        .ent
        .voyage_serie_creer(&prof, mk(c.jour, c.jour + Duration::days(20)))
        .await
        .unwrap();
    assert_eq!(
        r.sorties.len(),
        2,
        "3 semaines moins la semaine de vacances"
    );
    assert_eq!(r.ecartees.len(), 1);
    assert_eq!(r.ecartees[0].motif, "vacances");
    // Une plage entièrement en vacances : refusée, rien n'est créé.
    let avant: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_sortie_serie WHERE etablissement_id = $1")
            .bind(c.etab)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    let e = c
        .ent
        .voyage_serie_creer(
            &prof,
            mk(c.jour + Duration::days(7), c.jour + Duration::days(7)),
        )
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    let apres: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_sortie_serie WHERE etablissement_id = $1")
            .bind(c.etab)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!(avant, apres, "aucune série vide en base");
    // Une série de plus d'un an est refusée.
    assert!(c
        .ent
        .voyage_serie_creer(&prof, mk(c.jour, c.jour + Duration::days(500)))
        .await
        .is_err());
}

#[tokio::test]
async fn revue_securite_garde_fous_v0a() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let prof = c.a(c.prof, Role::Prof);
    let s = c.creer("Garde-fous", "sans_nuitee").await;
    c.jusqu_a_ouverte(s.id).await;
    let s = c.ent.voyage_detail(&prof, s.id).await.unwrap().sortie;
    // Décision sans version : refusée.
    let t = c.creer("Sans version", "sans_nuitee").await;
    c.preparer_jusqu_a_soumise(t.id).await;
    c.cmd(c.dir, Role::Direction, t.id, Commande::ValiderPedagogie)
        .await
        .unwrap();
    c.cmd(c.prof, Role::Prof, t.id, Commande::PasserAuxFinances)
        .await
        .unwrap();
    let e = c
        .ent
        .voyage_commande(
            &c.a(c.dir, Role::Direction),
            t.id,
            Commande::Autoriser,
            ParamsCommande::default(),
        )
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    // Le porteur ne change plus la nature ni la destination d'un dossier autorisé ; la direction oui.
    let mut m = c.saisie("Garde-fous", "sans_nuitee");
    m.destination = "etranger".into();
    m.pays = Some("Italie".into());
    let e = c
        .ent
        .voyage_modifier(&prof, s.id, s.version, m.clone())
        .await
        .unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    assert!(c
        .ent
        .voyage_modifier(&c.a(c.dir, Role::Direction), s.id, s.version, m)
        .await
        .is_ok());
    // Désistement « financier » : jamais décidé par le porteur ni le secrétariat.
    c.ent
        .voyage_participants_ajouter(&prof, s.id, vec![c.eleves[0]])
        .await
        .unwrap();
    let pid = c.ent.voyage_participants(&prof, s.id).await.unwrap()[0].id;
    for (u, r) in [(c.prof, Role::Prof), (c.sec, Role::Administratif)] {
        let e = c
            .ent
            .voyage_participant_desister(&c.a(u, r), s.id, pid, "financier", Some("x"))
            .await
            .unwrap_err();
        assert!(est_refus(&e), "{e:?}");
    }
    // Validation d'une série : droit requis avant toute information.
    let r = c
        .ent
        .voyage_serie_valider(&prof, Uuid::new_v4())
        .await
        .unwrap_err();
    assert!(est_forbidden(&r), "{r:?}");
    // Famille : un lien échu ne donne plus accès.
    let sp = c.creer("Famille", "sans_nuitee").await;
    c.jusqu_a_ouverte(sp.id).await;
    c.ent
        .voyage_participants_ajouter(&prof, sp.id, vec![c.eleves[0]])
        .await
        .unwrap();
    let parent = c.a(c.parent, Role::Parent);
    assert!(!c.ent.voyages_famille(&parent).await.unwrap().is_empty());
    sqlx::query("UPDATE ent_eleve_lien SET date_fin = ent_aujourdhui() - 1 WHERE etablissement_id = $1 AND user_id = $2")
        .bind(c.etab)
        .bind(c.parent)
        .execute(c.ent.pool())
        .await
        .unwrap();
    assert!(c.ent.voyages_famille(&parent).await.unwrap().is_empty());
}

// ─────────────────────────── Corrections des défauts de Next.js ───────────────────────────

#[tokio::test]
async fn circulaire_et_envoi_aux_prestataires_controlent_les_droits() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Voyage en Alsace", "sans_nuitee").await;
    let prof = c.a(c.prof, Role::Prof);
    // Avant l'ouverture (R-SV5) : pas de circulaire, même pour le porteur.
    let e = c.ent.voyage_circulaire(&prof, s.id).await.unwrap_err();
    assert!(est_refus(&e), "{e:?}");
    c.jusqu_a_ouverte(s.id).await;
    // N'importe quel autre membre du personnel ne la génère pas (VD-14 : plus de génération libre).
    let tiers = c.a(compte(&c.ent, c.etab, "prof").await, Role::Prof);
    assert!(c.ent.voyage_circulaire(&tiers, s.id).await.is_err());
    assert!(c
        .ent
        .voyage_circulaire(&c.a(c.accueil, Role::Accueil), s.id)
        .await
        .is_err());
    assert!(c
        .ent
        .voyage_circulaire(&c.a(c.parent, Role::Parent), s.id)
        .await
        .is_err());
    // Le porteur et le secrétariat la génèrent ; le contenu vient des champs du dossier seuls.
    let circ = c.ent.voyage_circulaire(&prof, s.id).await.unwrap();
    assert!(circ.corps.contains("Voyage en Alsace") && circ.corps.contains("Parvis"));
    assert!(circ.corps.contains("facultative"));
    assert!(c
        .ent
        .voyage_circulaire(&c.a(c.sec, Role::Administratif), s.id)
        .await
        .is_ok());
    assert!(actions_journal(&c, s.id)
        .await
        .contains(&"circulaire_generee".to_string()));

    // Prestataires : le droit `voyages.prestataires` n'est ni celui d'un professeur ni d'un parent
    // ni de l'accueil ; il est celui du secrétariat, de la comptabilité et de la direction.
    for (r, attendu) in [
        (Role::Prof, false),
        (Role::Parent, false),
        (Role::Accueil, false),
        (Role::Cpe, false),
        (Role::Administratif, true),
        (Role::Compta, true),
        (Role::Direction, true),
    ] {
        let droits = ent_auth::modele_par_defaut(r);
        let a_droit = droits
            .iter()
            .any(|(a, _)| *a == Action::VoyagesPrestataires);
        assert_eq!(a_droit, attendu, "{r:?} voyages.prestataires");
    }
}

// ─────────────────────────── Port « en sortie », journal, RLS ───────────────────────────

#[tokio::test]
async fn port_en_sortie_pour_l_appel_et_la_cantine() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Journée à la ferme", "sans_nuitee").await;
    let prof = c.a(c.prof, Role::Prof);
    c.jusqu_a_ouverte(s.id).await;
    c.ent
        .voyage_participants_ajouter(&prof, s.id, vec![c.eleves[0], c.eleves[1]])
        .await
        .unwrap();
    // Tant que la liste n'est pas confirmée (sortie « ouverte »), personne n'est « en sortie ».
    let fenetre = (c.at(0), c.at(23));
    assert!(c
        .ent
        .voyages_eleves_en_sortie(c.etab, fenetre.0, fenetre.1)
        .await
        .unwrap()
        .is_empty());
    c.cmd(c.prof, Role::Prof, s.id, Commande::ConfirmerListe)
        .await
        .unwrap();
    let v = c
        .ent
        .voyages_eleves_en_sortie(c.etab, fenetre.0, fenetre.1)
        .await
        .unwrap();
    let mut ids: Vec<Uuid> = v.iter().map(|x| x.eleve_id).collect();
    ids.sort();
    let mut attendus = vec![c.eleves[0], c.eleves[1]];
    attendus.sort();
    assert_eq!(ids, attendus);
    // Hors de la plage horaire de la sortie : personne.
    assert!(c
        .ent
        .voyages_eleves_en_sortie(c.etab, c.at(17), c.at(23))
        .await
        .unwrap()
        .is_empty());
    assert!(c
        .ent
        .voyages_eleves_en_sortie(
            c.etab,
            c.at(0) - Duration::days(2),
            c.at(1) - Duration::days(1)
        )
        .await
        .unwrap()
        .is_empty());
    // Un élève désisté n'est plus « en sortie » (corrige VD-42 : il était absent sans alerte).
    let pid = c.ent.voyage_participants(&prof, s.id).await.unwrap()[0].id;
    c.ent
        .voyage_participant_desister(&prof, s.id, pid, "famille", None)
        .await
        .unwrap();
    let v = c
        .ent
        .voyages_eleves_en_sortie(c.etab, fenetre.0, fenetre.1)
        .await
        .unwrap();
    assert_eq!(v.len(), 1);
    // Une sortie annulée libère tout le monde.
    c.cmd_motif(
        c.dir,
        Role::Direction,
        s.id,
        Commande::Annuler,
        "Météo",
        None,
    )
    .await
    .unwrap();
    assert!(c
        .ent
        .voyages_eleves_en_sortie(c.etab, fenetre.0, fenetre.1)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn journaux_en_ajout_seul() {
    let Some(c) = Cadre::nouveau().await else {
        return;
    };
    let s = c.creer("Immuable", "sans_nuitee").await;
    c.ent
        .voyage_participants_ajouter(&c.a(c.prof, Role::Prof), s.id, vec![c.eleves[0]])
        .await
        .unwrap();
    for sql in [
        "UPDATE ent_sortie_journal SET action = 'x' WHERE sortie_id = $1",
        "DELETE FROM ent_sortie_journal WHERE sortie_id = $1",
    ] {
        let r = sqlx::query(sql).bind(s.id).execute(c.ent.pool()).await;
        let code = match &r {
            Err(sqlx::Error::Database(d)) => d.code().map(|x| x.to_string()),
            _ => None,
        };
        assert_eq!(code.as_deref(), Some("42501"), "{sql} : {r:?}");
    }
    let r = sqlx::query(
        "DELETE FROM ent_sortie_participant_fait WHERE participant_id IN (SELECT id FROM ent_sortie_participant WHERE sortie_id = $1)",
    )
    .bind(s.id)
    .execute(c.ent.pool())
    .await;
    assert!(r.is_err(), "le fil du participant est en ajout seul");
}

#[cfg(feature = "labo")]
#[tokio::test]
async fn rls_des_neuf_tables_du_lot_v0a() {
    crate::tests_rls_lots::verifier_tables(&[
        "ent_sortie",
        "ent_sortie_serie",
        "ent_sortie_serie_exclusion",
        "ent_sortie_public",
        "ent_sortie_etape",
        "ent_sortie_journal",
        "ent_sortie_participant",
        "ent_sortie_participant_fait",
        "ent_sortie_encadrant",
    ])
    .await;
    let Some(e) = crate::tests_rls_lots::preparer().await else {
        return;
    };
    crate::tests_rls_lots::verifier_isolation_concrete(&e, "ent_sortie_serie", |id, etab, _| {
        format!(
            "INSERT INTO ent_sortie_serie (id, etablissement_id, libelle, jour_semaine, heure_debut, heure_fin, du, au)
             VALUES ('{id}', '{etab}', 's', 2, '08:00', '09:00', ent_aujourdhui(), ent_aujourdhui())"
        )
    })
    .await;
}

#[test]
fn aucune_colonne_json_dans_les_migrations_du_socle_42() {
    // D-036 : le journal du séjour est en texte, plus aucune exception jsonb.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    for f in [
        "640_sortie_coeur.sql",
        "641_sortie_participants.sql",
        "642_sortie_encadrants.sql",
    ] {
        let sql: String = std::fs::read_to_string(dir.join(f))
            .unwrap()
            .to_lowercase()
            .lines()
            .map(|l| l.split("--").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!sql.contains("json"), "{f} contient du JSON (D-036)");
        assert!(
            !sql.contains("bytea"),
            "{f} contient du BYTEA (fichiers : StorageProvider)"
        );
    }
}
