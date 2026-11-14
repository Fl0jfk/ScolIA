//! Socle 6, lot P6 : délégations par objet (rôle, mission, signature, intérim de direction), pouvoirs de signature et
//! port `peut_signer` pour le socle 7. Migration 359. Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! * délégation expirée ⇒ droits retirés LE SOIR MÊME, au jour de l'établissement (deux fuseaux) ;
//! * anti-escalade : un délégant ne délègue jamais plus que ce qu'il a (droits, mission, signature, intérim) ;
//! * `peut_signer` : chef par fonction, habilitation, délégation, intérim ; formule « par délégation » ; révocation
//!   immédiate ; cloisonnement par établissement ;
//! * révocation (PRS-83) : ferme la mission copiée et le pouvoir de signature ; expiration et avertissement à J-3 (PRS-84) ;
//! * migration 359 rejouable.

use super::*;
use crate::delegation::{DelegationSaisie, DroitDelegue};
use crate::personnels_delegations::{droits_interim, EXCLUS_INTERIM};
use crate::ref_commun::Acteur;
use chrono::{Duration, NaiveDate};
use domain_eleve::DomainError;
use ent_auth::{modele_par_defaut, Action, Portee, Role};
use ent_types::personnels_delegations::*;
use ent_types::personnels_missions::*;
use uuid::Uuid;

const MDP: &str = "MotDePasse-P6-Aa9!";

async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let email = format!("p6-{role}-{}@ent.local", Uuid::new_v4().simple());
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

async fn agent_de(ent: &PgEnt, etab: Uuid, user: Uuid) -> Uuid {
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

fn est_conflit(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Refus(RefusMetier::Conflit, _))
}
fn est_invalide(e: &EntDbError) -> bool {
    matches!(
        e,
        EntDbError::Refus(RefusMetier::Invalide, _)
            | EntDbError::Domain(DomainError::ValeurInvalide)
    )
}
fn est_introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(DomainError::NotFound))
}
fn est_interdit(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Forbidden)
}

/// Un compte et son agent.
#[derive(Clone, Copy)]
struct Cpt {
    user: Uuid,
    agent: Uuid,
}

struct Cadre {
    ent: PgEnt,
    etab: Uuid,
    jour: NaiveDate,
    /// Chef d'établissement (profil direction, fonction `chef_etablissement`).
    chef: Cpt,
    /// Adjoint : profil direction, fonction `adjoint` (PAS chef d'établissement).
    adjoint: Cpt,
    admin: Cpt,
    sec: Cpt,
    prof: Cpt,
}

impl Cadre {
    async fn compte(&self, role: &str) -> Cpt {
        let user = compte(&self.ent, self.etab, role).await;
        Cpt {
            user,
            agent: agent_de(&self.ent, self.etab, user).await,
        }
    }

    async fn nouveau() -> Option<Cadre> {
        let ent = connecter().await?;
        let etab = Uuid::new_v4();
        let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
        let mk = |role: &'static str| {
            let ent = &ent;
            async move {
                let user = compte(ent, etab, role).await;
                Cpt {
                    user,
                    agent: agent_de(ent, etab, user).await,
                }
            }
        };
        let (chef, adjoint, admin, sec, prof) = (
            mk("direction").await,
            mk("direction").await,
            mk("admin").await,
            mk("administratif").await,
            mk("prof").await,
        );
        // Le profil « direction » propose la fonction de chef d'établissement : l'adjoint est requalifié.
        sqlx::query("UPDATE ent_agent_fonction SET annule_le = now() WHERE etablissement_id = $1 AND agent_id = $2")
            .bind(etab)
            .bind(adjoint.agent)
            .execute(ent.pool())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO ent_agent_fonction (id, etablissement_id, agent_id, fonction_id, principale, du)
             SELECT gen_random_uuid(), $1, $2, f.id, true, $3 FROM ent_fonction f WHERE f.etablissement_id IS NULL AND f.code = 'adjoint'",
        )
        .bind(etab)
        .bind(adjoint.agent)
        .bind(jour - Duration::days(30))
        .execute(ent.pool())
        .await
        .unwrap();
        Some(Cadre {
            ent,
            etab,
            jour,
            chef,
            adjoint,
            admin,
            sec,
            prof,
        })
    }

    fn j(&self, n: i64) -> NaiveDate {
        self.jour + Duration::days(n)
    }

    async fn signe(&self, agent: Uuid, nature: &str, date: NaiveDate) -> PouvoirSignatureDto {
        self.ent
            .peut_signer(self.etab, agent, nature, date)
            .await
            .unwrap()
    }

    async fn habiliter(
        &self,
        par: Cpt,
        role: &str,
        agent: Uuid,
        docs: &[&str],
        du: Option<NaiveDate>,
        au: Option<NaiveDate>,
    ) -> Result<HabilitationDto, EntDbError> {
        self.ent
            .habiliter_signature_for_actor(
                self.etab,
                par.user,
                role,
                &HabiliterBody {
                    agent_id: agent,
                    documents: docs.iter().map(|s| s.to_string()).collect(),
                    du,
                    au,
                    formule: None,
                },
            )
            .await
    }

    async fn deleguer_signature(
        &self,
        par: Cpt,
        role: &str,
        vers: Uuid,
        docs: &[&str],
        au: NaiveDate,
    ) -> Result<DelegationObjetDto, EntDbError> {
        self.ent
            .deleguer_signature_for_actor(
                self.etab,
                par.user,
                role,
                &DeleguerSignatureBody {
                    delegataire_agent_id: vers,
                    documents: docs.iter().map(|s| s.to_string()).collect(),
                    du: None,
                    au,
                    motif_code: Some("absence".into()),
                    formule: None,
                },
            )
            .await
    }

    async fn revoquer(&self, par: Cpt, role: &str, id: Uuid) {
        self.ent
            .delegation_revoquer(
                &Acteur {
                    etablissement_id: self.etab,
                    user_id: par.user,
                    role,
                },
                id,
            )
            .await
            .unwrap();
    }

    async fn droits(&self, c: Cpt, role: &str) -> Vec<Droit> {
        self.ent
            .droits_effectifs(self.etab, c.user, role)
            .await
            .unwrap()
    }

    async fn evenements(&self, type_name: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = $2",
        )
        .bind(self.etab)
        .bind(type_name)
        .fetch_one(self.ent.pool())
        .await
        .unwrap()
    }
}

fn a_droit(d: &[Droit], a: Action) -> bool {
    d.iter()
        .any(|x| x.action == a && x.portee == Portee::Etablissement)
}

// ─────────────────────────────── peut_signer ───────────────────────────────

#[tokio::test]
async fn peut_signer_chef_habilitation_delegation_et_formule() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P6: ENT_DATABASE_URL");
        return;
    };
    // Le chef signe tout par sa fonction, en son nom (pas de formule) ; le secrétariat ne signe rien.
    let p = c.signe(c.chef.agent, "certificat_scolarite", c.jour).await;
    assert!(p.autorise && p.origine.as_deref() == Some("fonction") && p.formule.is_none());
    assert!(
        !c.signe(c.sec.agent, "certificat_scolarite", c.jour)
            .await
            .autorise
    );
    assert!(
        !c.signe(c.adjoint.agent, "certificat_scolarite", c.jour)
            .await
            .autorise,
        "l'adjoint n'est pas chef"
    );

    // Seul le chef d'établissement habilite : l'adjoint (profil direction), l'administrateur et le professeur : refus.
    for (par, role) in [
        (c.adjoint, "direction"),
        (c.admin, "admin"),
        (c.prof, "prof"),
    ] {
        let r = c
            .habiliter(
                par,
                role,
                c.sec.agent,
                &["certificat_scolarite"],
                None,
                None,
            )
            .await;
        assert!(
            est_interdit(&r.unwrap_err()),
            "{role} ne peut pas habiliter"
        );
    }
    // Natures invalides : `*` est réservé à l'intérim ; vide ; majuscules.
    for docs in [vec!["*"], vec![], vec!["Certificat"]] {
        let r = c
            .habiliter(c.chef, "direction", c.sec.agent, &docs, None, None)
            .await;
        assert!(est_invalide(&r.unwrap_err()), "{docs:?}");
    }
    // Début dans le passé, fin avant le début, soi-même : 422.
    assert!(est_invalide(
        &c.habiliter(
            c.chef,
            "direction",
            c.sec.agent,
            &["attestation"],
            Some(c.j(-1)),
            None
        )
        .await
        .unwrap_err()
    ));
    assert!(est_invalide(
        &c.habiliter(
            c.chef,
            "direction",
            c.sec.agent,
            &["attestation"],
            Some(c.j(3)),
            Some(c.j(2))
        )
        .await
        .unwrap_err()
    ));
    assert!(est_invalide(
        &c.habiliter(
            c.chef,
            "direction",
            c.chef.agent,
            &["attestation"],
            None,
            None
        )
        .await
        .unwrap_err()
    ));

    // Habilitation permanente du secrétariat : deux natures, à partir d'aujourd'hui.
    let h = c
        .habiliter(
            c.chef,
            "direction",
            c.sec.agent,
            &["certificat_scolarite", "attestation"],
            None,
            None,
        )
        .await
        .unwrap();
    assert!(h.active && h.origine == "habilitation" && h.au.is_none());
    let p = c.signe(c.sec.agent, "attestation", c.jour).await;
    assert!(
        p.autorise
            && p.origine.as_deref() == Some("habilitation")
            && p.formule.is_none()
            && p.habilitation_id == Some(h.id)
    );
    assert!(
        !c.signe(c.sec.agent, "bulletin", c.jour).await.autorise,
        "autre nature : refus"
    );
    assert!(
        !c.signe(c.sec.agent, "attestation", c.j(-1)).await.autorise,
        "avant le début : refus"
    );
    assert!(
        c.signe(c.sec.agent, "attestation", c.j(400)).await.autorise,
        "permanente"
    );
    // Cloisonnement : cet agent n'a aucun pouvoir dans un autre établissement.
    let autre = Uuid::new_v4();
    compte(&c.ent, autre, "direction").await;
    assert!(
        !c.ent
            .peut_signer(autre, c.sec.agent, "attestation", c.jour)
            .await
            .unwrap()
            .autorise
    );
    assert!(
        !c.ent
            .peut_signer(autre, c.chef.agent, "attestation", c.jour)
            .await
            .unwrap()
            .autorise
    );
    // Lecture de ses propres pouvoirs, sans droit ; l'administrateur liste tout.
    assert_eq!(
        c.ent
            .mes_pouvoirs_signature(c.etab, c.sec.user)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        c.ent
            .habilitations_for_actor(c.etab, c.chef.user, "direction")
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(est_interdit(
        &c.ent
            .habilitations_for_actor(c.etab, c.prof.user, "prof")
            .await
            .unwrap_err()
    ));
    // Retrait : effet immédiat ; deux fois : 409.
    c.ent
        .retirer_habilitation_for_actor(c.etab, c.chef.user, "direction", h.id)
        .await
        .unwrap();
    assert!(!c.signe(c.sec.agent, "attestation", c.jour).await.autorise);
    assert!(est_conflit(
        &c.ent
            .retirer_habilitation_for_actor(c.etab, c.chef.user, "direction", h.id)
            .await
            .unwrap_err()
    ));
    assert_eq!(c.evenements("signature.habilitation_modifiee").await, 2);

    // Délégation de signature du chef à l'adjoint, jusqu'à J+5 : « par délégation », jusqu'au soir du dernier jour.
    let d = c
        .deleguer_signature(
            c.chef,
            "direction",
            c.adjoint.agent,
            &["ordre_mission"],
            c.j(5),
        )
        .await
        .unwrap();
    assert_eq!(d.objet, "signature");
    assert!(d.habilitation_id.is_some());
    let p = c.signe(c.adjoint.agent, "ordre_mission", c.jour).await;
    assert!(p.autorise && p.origine.as_deref() == Some("delegation"));
    assert_eq!(p.formule.as_deref(), Some(FORMULE_PAR_DELEGATION));
    assert_eq!(p.jusqu_au, Some(c.j(5)));
    assert!(
        c.signe(c.adjoint.agent, "ordre_mission", c.j(5))
            .await
            .autorise,
        "le dernier jour, jusqu'au soir"
    );
    assert!(
        !c.signe(c.adjoint.agent, "ordre_mission", c.j(6))
            .await
            .autorise,
        "le lendemain de la fin"
    );

    // Anti-escalade : l'adjoint ne SOUS-DÉLÈGUE pas ce qu'il tient d'une délégation ; ni ce qu'il n'a pas.
    let r = c
        .deleguer_signature(
            c.adjoint,
            "direction",
            c.sec.agent,
            &["ordre_mission"],
            c.j(3),
        )
        .await;
    assert!(est_interdit(&r.unwrap_err()), "pas de sous-délégation");
    let r = c
        .deleguer_signature(c.adjoint, "direction", c.sec.agent, &["bulletin"], c.j(3))
        .await;
    assert!(est_interdit(&r.unwrap_err()), "nature non détenue");
    // Le secrétariat (pas de droit de délégation) : refus ; fin obligatoire et plafonnée : 422 ; soi-même : 422.
    assert!(est_interdit(
        &c.deleguer_signature(
            c.sec,
            "administratif",
            c.prof.agent,
            &["attestation"],
            c.j(3)
        )
        .await
        .unwrap_err()
    ));
    assert!(est_invalide(
        &c.deleguer_signature(c.chef, "direction", c.sec.agent, &["attestation"], c.j(400))
            .await
            .unwrap_err()
    ));
    assert!(est_invalide(
        &c.deleguer_signature(c.chef, "direction", c.chef.agent, &["attestation"], c.j(3))
            .await
            .unwrap_err()
    ));
    // Un délégué sans compte ou d'un autre établissement : 404.
    let etranger = compte(&c.ent, autre, "prof").await;
    let a_etranger = agent_de(&c.ent, autre, etranger).await;
    assert!(est_introuvable(
        &c.deleguer_signature(c.chef, "direction", a_etranger, &["attestation"], c.j(3))
            .await
            .unwrap_err()
    ));

    // Révocation (PRS-83) : effet immédiat, le pouvoir disparaît avec la délégation ; l'état est lisible.
    c.revoquer(c.chef, "direction", d.id).await;
    assert!(
        !c.signe(c.adjoint.agent, "ordre_mission", c.jour)
            .await
            .autorise
    );
    let l = c
        .ent
        .delegations_objets_for_actor(c.etab, c.chef.user, "direction")
        .await
        .unwrap();
    assert_eq!(l.iter().find(|x| x.id == d.id).unwrap().etat, "revoquee");
}

#[tokio::test]
async fn interim_de_direction_droits_calcules_exclusions_signature_et_extinction() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P6: ENT_DATABASE_URL");
        return;
    };
    let interim = c.compte("administratif").await;
    // Une action du modèle de direction que le secrétariat n'a pas : ouverte par l'intérim seulement.
    let modele_sec = modele_par_defaut(Role::Administratif);
    let (gagnee, _) = droits_interim()
        .into_iter()
        .find(|(a, _)| !modele_sec.iter().any(|(b, _)| b == a))
        .expect("le modèle de direction contient une action que le secrétariat n'a pas");
    assert!(!a_droit(&c.droits(interim, "administratif").await, gagnee));
    assert!(!c.signe(interim.agent, "attestation", c.jour).await.autorise);

    // Refus : professeur ; fin obligatoire, passée, plafonnée ; soi-même (le chef) ; un parent n'est pas un agent.
    let corps = |vers: Uuid, au: NaiveDate| OrganiserInterimBody {
        delegataire_agent_id: vers,
        du: None,
        au,
        motif_code: Some("absence".into()),
        formule: None,
    };
    assert!(est_interdit(
        &c.ent
            .organiser_interim_for_actor(c.etab, c.prof.user, "prof", &corps(interim.agent, c.j(5)))
            .await
            .unwrap_err()
    ));
    assert!(est_invalide(
        &c.ent
            .organiser_interim_for_actor(
                c.etab,
                c.chef.user,
                "direction",
                &corps(interim.agent, c.j(-1))
            )
            .await
            .unwrap_err()
    ));
    assert!(est_invalide(
        &c.ent
            .organiser_interim_for_actor(
                c.etab,
                c.chef.user,
                "direction",
                &corps(interim.agent, c.j(500))
            )
            .await
            .unwrap_err()
    ));
    assert!(est_invalide(
        &c.ent
            .organiser_interim_for_actor(
                c.etab,
                c.chef.user,
                "direction",
                &corps(c.chef.agent, c.j(5))
            )
            .await
            .unwrap_err()
    ));

    // Le chef organise l'intérim jusqu'à J+5.
    let d = c
        .ent
        .organiser_interim_for_actor(
            c.etab,
            c.chef.user,
            "direction",
            &corps(interim.agent, c.j(5)),
        )
        .await
        .unwrap();
    assert_eq!(d.objet, "interim_direction");
    assert_eq!(d.etat, "active");
    let droits = c.droits(interim, "administratif").await;
    assert!(a_droit(&droits, gagnee), "droits du chef pendant l'intérim");
    for exclu in EXCLUS_INTERIM {
        assert!(
            !a_droit(&droits, exclu),
            "{exclu:?} n'est jamais donné par un intérim"
        );
    }
    // Aucun droit au-delà du modèle de direction (anti-escalade) : tout droit gagné est dans ce modèle.
    let direction = modele_par_defaut(Role::Direction);
    for x in &droits {
        let propre = modele_sec
            .iter()
            .any(|(a, p)| *a == x.action && *p == x.portee);
        let chef = direction
            .iter()
            .any(|(a, p)| *a == x.action && *p == x.portee);
        assert!(propre || chef, "droit hors modèle : {:?}", x.action);
    }
    // Signature : toutes les natures, avec la formule ; un second intérim sur la période : 409.
    let p = c
        .signe(interim.agent, "n_importe_quelle_nature", c.jour)
        .await;
    assert!(
        p.autorise
            && p.origine.as_deref() == Some("interim")
            && p.formule.as_deref() == Some(FORMULE_PAR_DELEGATION)
    );
    let autre = c.compte("administratif").await;
    assert!(est_conflit(
        &c.ent
            .organiser_interim_for_actor(
                c.etab,
                c.chef.user,
                "direction",
                &corps(autre.agent, c.j(3))
            )
            .await
            .unwrap_err()
    ));
    // L'intérimaire ne peut ni organiser un intérim ni habiliter (exclusions) ; il n'a pas `delegations.gerer`.
    assert!(est_interdit(
        &c.ent
            .organiser_interim_for_actor(
                c.etab,
                interim.user,
                "administratif",
                &corps(autre.agent, c.j(3))
            )
            .await
            .unwrap_err()
    ));
    // L'administrateur peut organiser pour le chef unique... ici deux comptes direction sont chefs par défaut : ambigu.
    let r = c
        .ent
        .organiser_interim_for_actor(c.etab, c.admin.user, "admin", &corps(autre.agent, c.j(30)))
        .await;
    assert!(r.is_err());

    // Le dernier jour, le droit tient ; le lendemain il a disparu (le jour de fin passe à la veille).
    sqlx::query("UPDATE ent_delegation SET debut = $2, fin = $3 WHERE id = $1")
        .bind(d.id)
        .bind(c.j(-5))
        .bind(c.jour)
        .execute(c.ent.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_habilitation_signature SET du = $2, au = $3 WHERE origine_ref = $1")
        .bind(d.id)
        .bind(c.j(-5))
        .bind(c.jour)
        .execute(c.ent.pool())
        .await
        .unwrap();
    assert!(
        a_droit(&c.droits(interim, "administratif").await, gagnee),
        "dernier jour : jusqu'au soir"
    );
    sqlx::query("UPDATE ent_delegation SET fin = $2 WHERE id = $1")
        .bind(d.id)
        .bind(c.j(-1))
        .execute(c.ent.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_habilitation_signature SET au = $2 WHERE origine_ref = $1")
        .bind(d.id)
        .bind(c.j(-1))
        .execute(c.ent.pool())
        .await
        .unwrap();
    assert!(
        !a_droit(&c.droits(interim, "administratif").await, gagnee),
        "lendemain : plus de droit"
    );
    assert!(!c.signe(interim.agent, "attestation", c.jour).await.autorise);
    // Révoquée : plus de droit non plus, même dans les dates.
    let d2 = c
        .ent
        .organiser_interim_for_actor(
            c.etab,
            c.chef.user,
            "direction",
            &corps(interim.agent, c.j(5)),
        )
        .await
        .unwrap();
    assert!(a_droit(&c.droits(interim, "administratif").await, gagnee));
    c.revoquer(c.chef, "direction", d2.id).await;
    assert!(!a_droit(&c.droits(interim, "administratif").await, gagnee));
    assert!(!c.signe(interim.agent, "attestation", c.jour).await.autorise);
}

#[tokio::test]
async fn un_delegant_ne_delegue_jamais_plus_que_ce_qu_il_a() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P6: ENT_DATABASE_URL");
        return;
    };
    let vers = c.compte("administratif").await;
    let acteur = |c: &Cadre, par: Cpt, role: &'static str| Acteur {
        etablissement_id: c.etab,
        user_id: par.user,
        role,
    };
    let saisie = |action: Action| DelegationSaisie {
        delegue_user_id: vers.user,
        droits: vec![DroitDelegue {
            action: action.code().into(),
            portee: "etablissement".into(),
            cibles: vec![],
        }],
        debut: None,
        fin: c.j(5),
        motif: "test".into(),
    };
    // La direction ne détient PAS `missions.types.gerer` (Admin seul) : elle ne le délègue pas.
    assert!(!c
        .ent
        .a_action_etablissement(c.etab, c.chef.user, "direction", Action::MissionsTypesGerer)
        .await
        .unwrap());
    assert!(c
        .ent
        .delegation_accorder(
            &acteur(&c, c.chef, "direction"),
            &saisie(Action::MissionsTypesGerer)
        )
        .await
        .is_err());
    // Les droits de gouvernance ne se délèguent jamais, même à qui les détient (pas de chaîne de délégations).
    for a in [
        Action::RolesGerer,
        Action::SecuritePolitique,
        Action::DelegationsGerer,
        Action::SignatureHabiliter,
    ] {
        assert!(
            c.ent
                .delegation_accorder(&acteur(&c, c.chef, "direction"), &saisie(a))
                .await
                .is_err(),
            "{a:?}"
        );
    }
    // Ce qu'elle détient se délègue ; le secrétariat (sans `delegations.gerer`) ne délègue rien.
    let ok = c
        .ent
        .delegation_accorder(
            &acteur(&c, c.chef, "direction"),
            &saisie(Action::AbsencesLire),
        )
        .await
        .unwrap();
    assert_eq!(ok.objet, "role");
    assert!(c
        .ent
        .delegation_accorder(
            &acteur(&c, vers, "administratif"),
            &saisie(Action::AbsencesLire)
        )
        .await
        .is_err());
}

// ─────────────────────────────── délégation de mission ───────────────────────────────

#[tokio::test]
async fn delegation_de_mission_copie_revocation_et_garde_fous() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P6: ENT_DATABASE_URL");
        return;
    };
    let (p1, p2, p3) = (
        c.compte("prof").await,
        c.compte("prof").await,
        c.compte("prof").await,
    );
    let annee = c
        .ent
        .referentiel_annee_courante_assuree(c.etab)
        .await
        .unwrap();
    let sfx = Uuid::new_v4().simple().to_string()[..6].to_string();
    let classe = c
        .ent
        .referentiel_creer_classe(
            "administratif",
            c.etab,
            Some(annee.id),
            &format!("A{sfx}"),
            Some("4e"),
            Some("4EME"),
        )
        .await
        .unwrap()
        .id;
    let eleve = c
        .ent
        .create_eleve(CreateEleve {
            etablissement_id: c.etab,
            nom: format!("E{sfx}"),
            prenom: "P6".into(),
            ine: None,
            sexe: None,
            date_naissance: None,
            numero_badge: None,
            redoublant: false,
            classe: None,
            regime: None,
            mef: None,
            site: None,
            date_entree: None,
            date_sortie: None,
            motif_sortie: None,
            actor_user_id: None,
        })
        .await
        .unwrap()
        .id;
    c.ent
        .referentiel_affecter_eleve_classe(
            "administratif",
            c.etab,
            classe,
            eleve,
            Some(annee.date_debut),
        )
        .await
        .unwrap();
    let per = vec![PerimetreMissionDto {
        nature: "classe".into(),
        cible_id: Some(classe),
        discipline_code: None,
    }];
    let mission = c
        .ent
        .nommer_mission_for_actor(
            c.etab,
            c.chef.user,
            "direction",
            &NommerMissionBody {
                type_code: Some("professeur_principal".into()),
                agent_id: p1.agent,
                perimetres: per.clone(),
                annee_id: Some(annee.id),
                du: Some(c.j(-3)),
                au: Some(c.j(20)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let voit = |u: Cpt| {
        let ent = &c.ent;
        let etab = c.etab;
        async move {
            ent.autoriser_eleve(etab, u.user, "prof", Action::DossierLire, eleve)
                .await
                .unwrap()
        }
    };
    assert!(voit(p1).await && !voit(p2).await);
    let corps = |mission_id: Uuid, vers: Uuid, au: NaiveDate| DeleguerMissionBody {
        mission_id,
        delegataire_agent_id: vers,
        du: None,
        au,
        motif_code: Some("absence".into()),
    };

    // Refus : sans droit ; mission inconnue ou d'un autre établissement ; déborde la mission ; soi-même ; passé.
    assert!(est_interdit(
        &c.ent
            .deleguer_mission_for_actor(
                c.etab,
                p1.user,
                "prof",
                &corps(mission.id, p2.agent, c.j(10))
            )
            .await
            .unwrap_err()
    ));
    assert!(est_introuvable(
        &c.ent
            .deleguer_mission_for_actor(
                c.etab,
                c.chef.user,
                "direction",
                &corps(Uuid::new_v4(), p2.agent, c.j(10))
            )
            .await
            .unwrap_err()
    ));
    let autre = Uuid::new_v4();
    let dir2 = compte(&c.ent, autre, "direction").await;
    assert!(est_introuvable(
        &c.ent
            .deleguer_mission_for_actor(
                autre,
                dir2,
                "direction",
                &corps(mission.id, p2.agent, c.j(10))
            )
            .await
            .unwrap_err()
    ));
    assert!(
        est_invalide(
            &c.ent
                .deleguer_mission_for_actor(
                    c.etab,
                    c.chef.user,
                    "direction",
                    &corps(mission.id, p2.agent, c.j(30))
                )
                .await
                .unwrap_err()
        ),
        "au-delà de la mission"
    );
    assert!(
        est_invalide(
            &c.ent
                .deleguer_mission_for_actor(
                    c.etab,
                    c.chef.user,
                    "direction",
                    &corps(mission.id, p1.agent, c.j(10))
                )
                .await
                .unwrap_err()
        ),
        "à soi-même"
    );
    let mut passe = corps(mission.id, p2.agent, c.j(10));
    passe.du = Some(c.j(-1));
    assert!(est_invalide(
        &c.ent
            .deleguer_mission_for_actor(c.etab, c.chef.user, "direction", &passe)
            .await
            .unwrap_err()
    ));

    // La direction délègue la mission de p1 à p2 jusqu'à J+10 : p2 reçoit une copie, p1 garde la sienne.
    let d = c
        .ent
        .deleguer_mission_for_actor(
            c.etab,
            c.chef.user,
            "direction",
            &corps(mission.id, p2.agent, c.j(10)),
        )
        .await
        .unwrap();
    assert_eq!(d.objet, "mission");
    let derivee = d.mission_derivee_id.expect("mission copiée");
    let (origine, origine_ref): (String, Option<Uuid>) =
        sqlx::query_as("SELECT origine, origine_ref FROM ent_mission WHERE id = $1")
            .bind(derivee)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert_eq!((origine.as_str(), origine_ref), ("delegation", Some(d.id)));
    assert!(
        voit(p2).await && voit(p1).await,
        "le délégataire voit la classe, le titulaire aussi"
    );
    assert!(!voit(p3).await);
    // Une troisième personne ne reçoit pas de PP en plus (cardinalité 2 : p1 + p2) : 409.
    assert!(est_conflit(
        &c.ent
            .deleguer_mission_for_actor(
                c.etab,
                c.chef.user,
                "direction",
                &corps(mission.id, p3.agent, c.j(10))
            )
            .await
            .unwrap_err()
    ));
    // Révocation (PRS-83) : la mission copiée est annulée, p2 perd le droit tout de suite, p1 le garde.
    c.revoquer(c.chef, "direction", d.id).await;
    assert!(!voit(p2).await && voit(p1).await);
    let annulee: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT annule_le FROM ent_mission WHERE id = $1")
            .bind(derivee)
            .fetch_one(c.ent.pool())
            .await
            .unwrap();
    assert!(annulee.is_some());
    // Le délégant peut révoquer la sienne sans droit de délégation ; une délégation révoquée ne se révoque plus.
    let d2 = c
        .ent
        .deleguer_mission_for_actor(
            c.etab,
            c.chef.user,
            "direction",
            &corps(mission.id, p3.agent, c.j(10)),
        )
        .await
        .unwrap();
    assert!(voit(p3).await);
    let l = c
        .ent
        .delegations_objets_for_actor(c.etab, c.chef.user, "direction")
        .await
        .unwrap();
    assert_eq!(l.len(), 2);
    c.revoquer(c.chef, "direction", d2.id).await;
    assert!(!voit(p3).await);
}

// ─────────────────────────────── heure de l'établissement, expiration, avertissement ───────────────────────────────

async fn fixer_fuseau(ent: &PgEnt, etab: Uuid, fuseau: &str) {
    sqlx::query(
        "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, $2)
         ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = EXCLUDED.fuseau",
    )
    .bind(etab)
    .bind(fuseau)
    .execute(ent.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn delegation_expiree_retire_ses_droits_le_soir_meme_a_l_heure_de_l_etablissement() {
    let Some(ent) = connecter().await else {
        eprintln!("skip P6: ENT_DATABASE_URL");
        return;
    };
    // Kiritimati (UTC+14) et Pago Pago (UTC-11) : un jour civil d'écart au moins, à tout instant.
    let (kiri, pago) = (Uuid::new_v4(), Uuid::new_v4());
    fixer_fuseau(&ent, kiri, "Pacific/Kiritimati").await;
    fixer_fuseau(&ent, pago, "Pacific/Pago_Pago").await;
    let (j_kiri, j_pago) = (
        ent.aujourdhui_etablissement(kiri).await.unwrap(),
        ent.aujourdhui_etablissement(pago).await.unwrap(),
    );
    assert!(j_kiri > j_pago, "deux fuseaux, deux jours");
    // Pour chaque établissement : un délégué (secrétariat) reçoit NotesLire par délégation de droits dont la fin est
    // « aujourd'hui à Pago Pago », une habilitation de signature (`au` identique) et un intérim (`fin` identique).
    for (etab, attendu) in [(pago, true), (kiri, false)] {
        let chef = compte(&ent, etab, "direction").await;
        let delegue = compte(&ent, etab, "compta").await;
        let (a_chef, a_deleg) = (
            agent_de(&ent, etab, chef).await,
            agent_de(&ent, etab, delegue).await,
        );
        let d_droits = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_delegation (id, etablissement_id, delegue_user_id, delegant_user_id, debut, fin, motif)
                     VALUES ($1, $2, $3, $4, $5, $6, 'bord de journée')")
            .bind(d_droits).bind(etab).bind(delegue).bind(chef).bind(j_pago - Duration::days(10)).bind(j_pago)
            .execute(ent.pool()).await.unwrap();
        sqlx::query("INSERT INTO ent_delegation_droit (etablissement_id, delegation_id, ordre, action, portee) VALUES ($1, $2, 0, 'notes.lire', 'etablissement')")
            .bind(etab).bind(d_droits).execute(ent.pool()).await.unwrap();
        let d_sig = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_delegation (id, etablissement_id, delegue_user_id, delegant_user_id, debut, fin, motif, objet)
                     VALUES ($1, $2, $3, $4, $5, $6, 'signature', 'signature')")
            .bind(d_sig).bind(etab).bind(delegue).bind(chef).bind(j_pago - Duration::days(10)).bind(j_pago)
            .execute(ent.pool()).await.unwrap();
        sqlx::query("INSERT INTO ent_habilitation_signature (id, etablissement_id, agent_id, documents, du, au, origine, origine_ref)
                     VALUES ($1, $2, $3, ARRAY['attestation'], $4, $5, 'delegation', $6)")
            .bind(Uuid::new_v4()).bind(etab).bind(a_deleg).bind(j_pago - Duration::days(10)).bind(j_pago).bind(d_sig)
            .execute(ent.pool()).await.unwrap();
        let _ = a_chef;
        // L'engagement créé avec le compte commence au jour de Paris : on le recule pour que l'agent soit en poste
        // dans les deux fuseaux (sinon le test dépend de l'heure à laquelle il tourne).
        sqlx::query("UPDATE ent_agent_engagement SET du = $2 WHERE etablissement_id = $1")
            .bind(etab)
            .bind(j_pago - Duration::days(30))
            .execute(ent.pool())
            .await
            .unwrap();
        let droits = ent.droits_effectifs(etab, delegue, "compta").await.unwrap();
        assert_eq!(
            a_droit(&droits, Action::NotesLire),
            attendu,
            "droits délégués, {etab}"
        );
        let p = ent
            .peut_signer_compte(etab, delegue, "attestation")
            .await
            .unwrap();
        assert_eq!(p.autorise, attendu, "signature déléguée, {etab}");
        // Le lendemain (jour local) : plus rien, nulle part.
        sqlx::query("UPDATE ent_delegation SET fin = $2 WHERE id = ANY($1)")
            .bind(vec![d_droits, d_sig])
            .bind(ent.aujourdhui_etablissement(etab).await.unwrap() - Duration::days(1))
            .execute(ent.pool())
            .await
            .unwrap();
        assert!(!a_droit(
            &ent.droits_effectifs(etab, delegue, "compta").await.unwrap(),
            Action::NotesLire
        ));
    }
}

#[tokio::test]
async fn balayage_previent_le_delegant_a_j_moins_3_et_expire_une_seule_fois() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P6: ENT_DATABASE_URL");
        return;
    };
    let vers = c.compte("administratif").await;
    let acteur = Acteur {
        etablissement_id: c.etab,
        user_id: c.chef.user,
        role: "direction",
    };
    let d = c
        .ent
        .delegation_accorder(
            &acteur,
            &DelegationSaisie {
                delegue_user_id: vers.user,
                droits: vec![DroitDelegue {
                    action: "absences.lire".into(),
                    portee: "etablissement".into(),
                    cibles: vec![],
                }],
                debut: None,
                fin: c.j(3),
                motif: "absence".into(),
            },
        )
        .await
        .unwrap();
    // J-4 : rien. J-3 : un avertissement, une seule fois. Fin passée : une expiration, une seule fois.
    c.ent.delegations_balayer(c.j(-1)).await.unwrap();
    assert_eq!(c.evenements("delegation.expiration_proche").await, 0);
    c.ent.delegations_balayer(c.jour).await.unwrap();
    assert_eq!(
        c.evenements("delegation.expiration_proche").await,
        1,
        "à trois jours de la fin"
    );
    c.ent.delegations_balayer(c.j(1)).await.unwrap();
    assert_eq!(
        c.evenements("delegation.expiration_proche").await,
        1,
        "jamais deux fois"
    );
    assert_eq!(c.evenements("delegation.expiree").await, 0);
    c.ent.delegations_balayer(c.j(4)).await.unwrap();
    c.ent.delegations_balayer(c.j(5)).await.unwrap();
    assert_eq!(c.evenements("delegation.expiree").await, 1);
    let _ = d;
}

#[tokio::test]
async fn migration_359_rejouable_et_gardes_de_la_base() {
    let Some(c) = Cadre::nouveau().await else {
        eprintln!("skip P6: ENT_DATABASE_URL");
        return;
    };
    let sql = std::fs::read_to_string(format!(
        "{}/../../migrations/359_delegations_signature.sql",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    for _ in 0..2 {
        rejouer(&c.ent, &sql, "359").await;
    }
    // Les délégations existantes ont leurs agents projetés ; l'objet par défaut est « role ».
    let acteur = Acteur {
        etablissement_id: c.etab,
        user_id: c.chef.user,
        role: "direction",
    };
    let vers = c.compte("administratif").await;
    let d = c
        .ent
        .delegation_accorder(
            &acteur,
            &DelegationSaisie {
                delegue_user_id: vers.user,
                droits: vec![DroitDelegue {
                    action: "absences.lire".into(),
                    portee: "etablissement".into(),
                    cibles: vec![],
                }],
                debut: None,
                fin: c.j(3),
                motif: String::new(),
            },
        )
        .await
        .unwrap();
    let (objet, a1, a2): (String, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
        "SELECT objet, delegant_agent_id, delegataire_agent_id FROM ent_delegation WHERE id = $1",
    )
    .bind(d.id)
    .fetch_one(c.ent.pool())
    .await
    .unwrap();
    assert_eq!(
        (objet.as_str(), a1, a2),
        ("role", Some(c.chef.agent), Some(vers.agent))
    );
    // Une délégation de signature sans fin est refusée par la base ; une habilitation à une autre origine aussi.
    let r = sqlx::query("INSERT INTO ent_habilitation_signature (id, etablissement_id, agent_id, documents, du, origine, origine_ref)
                         VALUES ($1, $2, $3, ARRAY['attestation'], $4, 'delegation', $5)")
        .bind(Uuid::new_v4()).bind(c.etab).bind(vers.agent).bind(c.jour).bind(d.id).execute(c.ent.pool()).await;
    assert!(r.is_err(), "délégation sans fin");
    let r = sqlx::query("INSERT INTO ent_habilitation_signature (id, etablissement_id, agent_id, documents, du, origine)
                         VALUES ($1, $2, $3, ARRAY['attestation'], $4, 'habilitation')")
        .bind(Uuid::new_v4()).bind(Uuid::new_v4()).bind(vers.agent).bind(c.jour).execute(c.ent.pool()).await;
    assert!(r.is_err(), "agent d'un autre établissement");
}

/// Rejoue un fichier de migration ; le DDL concurrent d'un autre test peut provoquer un interblocage, on réessaie.
async fn rejouer(ent: &PgEnt, sql: &str, nom: &str) {
    for essai in 0..8 {
        match sqlx::raw_sql(sql).execute(ent.pool()).await {
            Ok(_) => return,
            Err(e) if e.to_string().contains("deadlock") && essai < 7 => {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            }
            Err(e) => panic!("{nom} rejouée : {e}"),
        }
    }
}
