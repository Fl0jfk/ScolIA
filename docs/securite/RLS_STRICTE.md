# RLS stricte : chemins d'accès sans contexte, conversion par domaine, accès transverses

Statut : **conversion des accès d'`ent-db` faite et gardée par un test statique ; un seul domaine (élèves et responsables) rejoué en mode strict en CI.
Mode strict NON activé par défaut, ni en production** (voir §8 pour les conditions de bascule). Aucune base de production consultée.
Vocabulaire : *fait* (vérifié dans le code ou par un test), *décision de conception* (à valider), *reste à faire*.

Lié : `docs/RLS_ISOLATION.md` (conception de la RLS, migration 100), migrations 580–587 (150 tables), 600 (adhésion),
`ent-db/src/tenant_tx.rs`, `ent-db/src/registre_donnees.rs` (D-037, `docs/PRA_SAUVEGARDES.md` §18.4 sur `ent-mc0hoe`).

## 1. Point de départ et limite
Les 303 tables « par établissement » sont sous `FORCE ROW LEVEL SECURITY` (politique `ent_rls_etablissement`). Mais la politique reste
**permissive sans contexte** hors mode strict : `etablissement_id = app.etablissement_id OR (app.etablissement_id IS NULL AND NOT app.rls_strict)`.
Seuls les chemins qui ouvrent un `TenantTx` (pose de `app.etablissement_id` pour la seule transaction) étaient filtrés par la base ; le reste
n'était protégé que par le filtre `etablissement_id` écrit à la main dans chaque requête (c'est précisément ce que la RLS doit rattraper).
Le mode strict (`app.rls_strict = on` au niveau du rôle : 0 ligne et écriture refusée sans contexte) n'était donc pas activable.

## 2. Mécanisme
- **Chemin normal** : `TenantTx::begin(pool, etablissement_id)` (déréférence en `PgConnection`, `commit()` explicite, annulation à la destruction).
  Lecture seule : une transaction « jetable » `&mut *TenantTx::begin(...)` suffit (annulation sans effet) ; **écriture : `commit()` obligatoire**
  (y compris `INSERT … RETURNING` lu par `fetch_*` : un `fetch_*` n'est pas une lecture seule). Une écriture oubliée sans `commit()` serait annulée en silence : c'est le
  piège de la conversion ; il est couvert par les suites de domaine rejouées en strict (la donnée écrite n'est plus relue).
- **Accès transverses (§4)** : jamais de contournement. Un traitement qui doit parcourir plusieurs établissements **liste les établissements dans deux
  tables sans RLS** (`ent_etablissement` ∪ `ent_membership`) via `etablissements_systeme(pool, ContexteSysteme)` puis ouvre **un `TenantTx` par établissement** ;
  `lire_par_etablissement(pool, ContexteSysteme, sql)` fait la même chose pour une sélection sans paramètre. Chaque usage est **nommé**
  (`ContexteSysteme`, liste fermée) et **journalisé** (`tracing`, cible `ent_rls_systeme`, champs `contexte` et `etablissements` : aucune donnée métier, E14).
  Aucun rôle « tout voir », aucun `BYPASSRLS`, aucune politique « système », aucune fonction `SECURITY DEFINER`.
- **Déclencheurs sur tables sans RLS (§4.3)** : migration 600.

## 3. Inventaire (ent-db, 123 fichiers avec accès base ; ent-api n'ouvre aucun SQL hors tests)
Méthode : analyse statique du code (hors modules de tests) des accès base exécutés **sur le pool**, sans contexte d'établissement
(`self.pool`, `self.pool()`, `&pool`, `.pool()`), regroupés par domaine d'après le registre `registre_donnees.rs` (domaine majoritaire des tables citées par le fichier).
**Avant** (origin/ENT au 2026-11-10) : 1 055 accès directs au pool, 211 accès via `TenantTx`.

| Domaine (ordre de conversion) | Fichiers | Accès via contexte avant | Accès directs au pool avant |
|---|---:|---:|---:|
| 1. Élèves et responsables | 29 | 29 | 232 |
| 2. Finances | 7 | 43 | 14 |
| 3. Inscriptions | 4 | 1 | 88 |
| 4. Santé | 3 | 27 | 4 |
| 5. Messagerie / communication | 6 | 0 | 64 |
| 6. Vie scolaire (discipline, assiduité, vie pratique) | 17 | 29 | 164 |
| 7. Évaluations et pédagogie | 12 | 2 | 186 |
| 8. Reste (structure, emploi du temps, personnels, divers) | 33 | 79 | 211 |
| Transverses sans RLS (identité, sessions, OIDC, outbox…) | 12 | 1 | 92 |
| **Total** | **123** | **211** | **1 055** |

**Après** (cette branche) : voir §5 (inventaire fichier par fichier). Les accès directs restants hors fichiers transverses sont **13 exceptions déclarées**
dans `ent-db/src/garde_rls_stricte.rs`, chacune motivée (tables sans RLS : `ent_membership`, `ent_user`, `ent_session`, jetons d'invitation et de préinscription,
`ref_calendrier_officiel`) ou délégant à un assistant qui ouvre son propre contexte.

Chemins hors dépôts, par nature :
| Nature | Où | Traitement |
|---|---|---|
| Dépôts `Pg*Repo` | `*_repo.rs`, `fac_repo.rs`… | `TenantTx` par méthode (établissement fourni par la commande ou la ligne) |
| Services `impl PgEnt` | `service.rs` (façade), `dossier_*.rs`, `inscriptions*.rs`, `scolarite.rs`… | `TenantTx` par cas d'usage |
| Jobs planifiés | `scolarite::tache_nocturne_eleves`, `eleve_doublons::tache_doublons_eleves`, `propositions::expirer_propositions_echues`, `delegation::delegations_balayer`, `rencontres::emettre_rappels_rdv`, `structure_calendrier::structure_fermetures_levees_tache`, `identite::resynchroniser_identites` | contexte système nommé (§4) |
| Workers d'envoi | `notifications::Expediteur::expedier` (notifications e-mail / SMS / push) | contexte système `WorkerEnvoi`, un passage par établissement |
| Rechiffrement au démarrage | `champs_chiffres::rechiffrer_champs_sensibles` | contexte système `Rechiffrement` |
| Distributeur d'événements | `outbox.rs` | aucune table sous RLS (§4.2) ; contexte posé par événement dans `traiter_par` (existant) |
| Authentification, sessions, passkeys, OIDC | `auth_repo.rs`, `passkey_repo.rs`, `oidc_repo.rs`, `recuperation_repo.rs`, `email_change_repo.rs`, `rate_limit.rs`, `etablissement_repo.rs` | tables sans RLS (registre) : lues avant le choix d'établissement |
| Migrations, PRA | `migrate.rs`, `pra.rs`, `registre_donnees::neutraliser_*` | rôle propriétaire, hors application |

## 4. Accès légitimement transverses : liste et mécanisme
### 4.1 Contextes système nommés (`ContexteSysteme`, `ent-db/src/tenant_tx.rs`)
| Contexte | Qui | Pourquoi transverse | Mécanisme |
|---|---|---|---|
| `DoublonsEleves` | `tache_doublons_eleves` | sélectionne les établissements à traiter | `lire_par_etablissement` (un `TenantTx` par établissement) puis `detecter_doublons(etab)` |
| `TacheNocturneEleves` | `tache_nocturne_eleves`, `resynchroniser_identites` | fonctions SQL `ent_eleve_statut_recalculer()`, `ent_eleve_majorites_a_traiter()`, `ent_eleve_identite_resynchroniser()` qui balaient les élèves | chaque fonction est appelée **dans le contexte de chaque établissement** (la RLS restreint ses lignes) ; les lignes d'un autre établissement sont écartées côté Rust (le mode `off` ne filtre pas) |
| `ExpirationPropositions` | `expirer_propositions_echues` | propositions échues de tous les établissements | `lire_par_etablissement` |
| `WorkerEnvoi` | `Expediteur::expedier` | envois échus (e-mail, SMS, push) | un passage par établissement (réclamation `FOR UPDATE SKIP LOCKED` dans son contexte) ; un seul établissement si la configuration en désigne un |
| `Rechiffrement` | `rechiffrer_champs_sensibles` | tables chiffrées des 303 | par établissement, filtre `etablissement_id` explicite + `TenantTx` ; tables sans établissement : pool |
| `JobPlanifie(nom)` | `delegations_balayer` (`delegations_balayage`), `emettre_rappels_rdv` (`rappels_rdv`), `structure_fermetures_levees_tache` (`fermetures_levees`) | tâches périodiques « tous établissements » | idem |
| `JobsFinances` | réservé (relances / échéances) | **non utilisé aujourd'hui** : les relances de finances se font à la demande d'un acteur dans son établissement | — |

Règle d'ajout : une nouvelle variante = un nouvel accès transverse = une ligne dans ce tableau et dans la revue de sécurité.

### 4.2 Tables sans RLS lues avant tout contexte (registre, exceptions `SansRls`)
`ent_user`, `ent_membership`, `ent_session`, `ent_passkey_*`, `ent_oidc_*`, `ent_password_reset`, `ent_rate_limit`, `ent_login_guard`, `ent_etablissement`,
`ent_etablissement_domaine`, `ent_inscription_invitation`, `ent_preinscription_verification`, `ent_inscription_envoi`, `ent_email_change`, `ent_recuperation_passkey`,
`ent_outbox`, `ent_outbox_agregat`, `ent_metier_event`, `ent_event_traitement`. Elles ne portent pas de politique ; l'isolation y reste applicative (inchangé).

### 4.3 Déclencheurs d'adhésion (migration 600)
`ent_membership` (sans RLS) déclenche `ent_membership_agent_sync` (création de la fiche personne, de l'agent, de son engagement, de ses fonctions) et
`ent_membership_garde_dernier_admin` (lecture des agents). Ces déclencheurs touchent des tables **sous RLS de l'établissement de la ligne** : sans contexte, l'ouverture
d'une adhésion était refusée en strict (constaté par la suite de domaine). La migration 600 encadre ces déclencheurs par deux déclencheurs (`…_a_ctx_pose_*` / `…_zz_ctx_rend_*`) qui
posent `app.etablissement_id` = établissement **de la ligne** pour la seule transaction puis rendent la valeur précédente. Additive, rejouable (E8), aucune politique modifiée, aucune table.

## 5. Inventaire après conversion (fichier par fichier)
« Accès via contexte » = occurrences de `TenantTx::begin` / `tenant(` / `lire_par_etablissement`. « Accès directs restants » = **détection large** (inclut des
passages de pool à des assistants qui ouvrent leur propre contexte et les tables sans RLS) ; les vrais résidus sont les 13 exceptions de `garde_rls_stricte.rs`.

#### Élèves et responsables

| Fichier | Accès via contexte | Accès directs restants | Fonctions restantes (à examiner) |
|---|---:|---:|---|
| `accompagnement.rs` | 11 | 0 | — |
| `annonce_repo.rs` | 3 | 0 | — |
| `doc_signer_repo.rs` | 7 | 0 | — |
| `document_repo.rs` | 11 | 0 | — |
| `dossier_contact.rs` | 12 | 0 | — |
| `dossier_coordonnees.rs` | 9 | 0 | — |
| `dossier_droits.rs` | 11 | 0 | — |
| `dossier_personnes.rs` | 23 | 0 | — |
| `dossier_personnes_ports.rs` | 7 | 0 | — |
| `dossier_protection.rs` | 8 | 0 | — |
| `eleve_compte_repo.rs` | 7 | 0 | — |
| `eleve_doublons.rs` | 5 | 1 | `doublons_pour_acteur` |
| `eleve_fusion.rs` | 6 | 1 | `apercu_fusion_pour_acteur` |
| `eleve_repo.rs` | 9 | 0 | — |
| `formulaires.rs` | 12 | 0 | — |
| `foyer_repo.rs` | 21 | 0 | — |
| `identifiants.rs` | 3 | 0 | — |
| `identite.rs` | 3 | 0 | — |
| `imports.rs` | 7 | 0 | — |
| `info_comp_repo.rs` | 5 | 0 | — |
| `journal_repo.rs` | 3 | 0 | — |
| `objets.rs` | 2 | 0 | — |
| `periode_repo.rs` | 6 | 0 | — |
| `propositions.rs` | 14 | 0 | — |
| `reactions.rs` | 1 | 0 | — |
| `referentiel_classe_id.rs` | 2 | 0 | — |
| `scolarite.rs` | 13 | 1 | `regle_dates_changement_regime` |
| `stage_repo.rs` | 6 | 0 | — |
| **total** | **227** | **3** | |

#### Finances

| Fichier | Accès via contexte | Accès directs restants | Fonctions restantes (à examiner) |
|---|---:|---:|---|
| `bank_repo.rs` | 3 | 0 | — |
| `fac_repo.rs` | 42 | 0 | — |
| `finances.rs` | 1 | 0 | — |
| `iban_histo_repo.rs` | 2 | 0 | — |
| `quotient_repo.rs` | 2 | 0 | — |
| `tarif_conso_repo.rs` | 3 | 0 | — |
| `tva_repo.rs` | 2 | 0 | — |
| **total** | **55** | **0** | |

#### Inscriptions

| Fichier | Accès via contexte | Accès directs restants | Fonctions restantes (à examiner) |
|---|---:|---:|---|
| `continuum_repo.rs` | 10 | 0 | — |
| `filiere_repo.rs` | 6 | 0 | — |
| `inscription_repo.rs` | 3 | 0 | — |
| `inscriptions.rs` | 40 | 5 | `dossier_dto`, `etablissement_du_jeton_preinscription`, `etablissement_de_demande_preinscription`, `etablissement_de_invitation` |
| `inscriptions_modele.rs` | 22 | 1 | `exiger_secretariat_ouvert` |
| **total** | **81** | **6** | |

#### Santé

| Fichier | Accès via contexte | Accès directs restants | Fonctions restantes (à examiner) |
|---|---:|---:|---|
| `champs_chiffres.rs` | 1 | 0 | — |
| `psy_repo.rs` | 10 | 0 | — |
| `sante_acc.rs` | 17 | 1 | `signer_pai_version_for_actor` |
| **total** | **28** | **1** | |

#### Messagerie / communication

| Fichier | Accès via contexte | Accès directs restants | Fonctions restantes (à examiner) |
|---|---:|---:|---|
| `campagnes.rs` | 8 | 0 | — |
| `mail_repo.rs` | 5 | 0 | — |
| `msg_repo.rs` | 23 | 0 | — |
| `notifications.rs` | 13 | 1 | `expedier` |
| `push_repo.rs` | 6 | 0 | — |
| `sms_repo.rs` | 4 | 0 | — |
| **total** | **59** | **1** | |

#### Vie scolaire (discipline, assiduité, vie pratique)

| Fichier | Accès via contexte | Accès directs restants | Fonctions restantes (à examiner) |
|---|---:|---:|---|
| `absence_repo.rs` | 7 | 0 | — |
| `absence_rh_repo.rs` | 2 | 0 | — |
| `alerte_cpe_repo.rs` | 5 | 0 | — |
| `appel_repo.rs` | 5 | 0 | — |
| `carnet_repo.rs` | 5 | 0 | — |
| `discipline_vs.rs` | 21 | 0 | — |
| `dispense_vs_repo.rs` | 5 | 0 | — |
| `edition_vs_repo.rs` | 4 | 0 | — |
| `internat_repo.rs` | 15 | 0 | — |
| `menu_cantine_repo.rs` | 2 | 0 | — |
| `motif_repo.rs` | 3 | 0 | — |
| `repas_repo.rs` | 24 | 0 | — |
| `retard_repo.rs` | 3 | 0 | — |
| `sanction_repo.rs` | 29 | 0 | — |
| `signal_repo.rs` | 7 | 0 | — |
| `sortie_repo.rs` | 18 | 0 | — |
| `vie_scolaire.rs` | 16 | 0 | — |
| **total** | **171** | **0** | |

#### Évaluations et pédagogie

| Fichier | Accès via contexte | Accès directs restants | Fonctions restantes (à examiner) |
|---|---:|---:|---|
| `cahier.rs` | 38 | 0 | — |
| `cahier_texte_repo.rs` | 2 | 0 | — |
| `competence_repo.rs` | 6 | 0 | — |
| `conseil_classe_repo.rs` | 9 | 0 | — |
| `evaluations.rs` | 24 | 0 | — |
| `evaluations_bulletin.rs` | 13 | 0 | — |
| `evaluations_conseil.rs` | 18 | 0 | — |
| `matiere_repo.rs` | 4 | 0 | — |
| `note_repo.rs` | 14 | 0 | — |
| `referentiel_structure.rs` | 19 | 0 | — |
| `structure_periodes.rs` | 2 | 0 | — |
| `vacances_repo.rs` | 4 | 0 | — |
| **total** | **153** | **0** | |

#### Reste (structure, emploi du temps, personnels, accès, divers)

| Fichier | Accès via contexte | Accès directs restants | Fonctions restantes (à examiner) |
|---|---:|---:|---|
| `cours.rs` | 7 | 0 | — |
| `delegation.rs` | 8 | 0 | — |
| `droits_repo.rs` | 13 | 0 | — |
| `edt_admin.rs` | 14 | 0 | — |
| `edt_contrainte_repo.rs` | 2 | 0 | — |
| `edt_masse.rs` | 4 | 0 | — |
| `edt_remplacement.rs` | 17 | 0 | — |
| `edt_repo.rs` | 13 | 0 | — |
| `eleve_champs.rs` | 1 | 0 | — |
| `enseignements.rs` | 1 | 0 | — |
| `module_sante.rs` | 2 | 0 | — |
| `option_repo.rs` | 2 | 0 | — |
| `param_eleve.rs` | 4 | 0 | — |
| `param_etab_repo.rs` | 3 | 0 | — |
| `perimetre.rs` | 3 | 0 | — |
| `personnels.rs` | 12 | 0 | — |
| `personnels_acces.rs` | 13 | 0 | — |
| `personnels_fiche.rs` | 19 | 0 | — |
| `personnels_services.rs` | 17 | 0 | — |
| `referentiel_annee.rs` | 10 | 0 | — |
| `referentiel_groupes.rs` | 1 | 0 | — |
| `referentiel_repo.rs` | 31 | 1 | `eleves_par_ids` |
| `referentiel_seances.rs` | 4 | 0 | — |
| `rencontres.rs` | 17 | 4 | `enseignants_de_classe`, `ajouter_creneaux`, `creneaux_famille`, `reserver_creneau` |
| `salle_repo.rs` | 4 | 0 | — |
| `structure_calendrier.rs` | 3 | 1 | `calendrier_officiel_referencer` |
| `structure_cours.rs` | 1 | 0 | — |
| `structure_enseignements.rs` | 1 | 0 | — |
| `structure_etab.rs` | 8 | 0 | — |
| `structure_grilles.rs` | 1 | 0 | — |
| `structure_lieux.rs` | 1 | 0 | — |
| `structure_offre.rs` | 1 | 0 | — |
| **total** | **238** | **6** | |

#### Accès transverses sans RLS (identité, sessions, outbox, migrations)

| Fichier | Accès via contexte | Accès directs restants | Fonctions restantes (à examiner) |
|---|---:|---:|---|
| `acces.rs` | 0 | 4 | `aujourdhui_etablissement`, `acces_etat`, `changer_etat_acces`, `acces_programmer_fin_for_actor` |
| `auth_repo.rs` | 0 | 37 | `verify_credentials`, `issue_session`, `completer_session_apres_enrolement`, `list_sessions_actives`, `revoke_session_of_user`, `login_attempt`, `login_attempt_refund`, `clear_login_failures`, `acces_ouvert`, `role_in`, `set_password`, `resolve_user_email`, `user_profile`, `purge_expired_sessions`, `resolve_bearer`, `first_member_with_roles`, `list_user_ids_with_roles`, `is_member`, `find_member_id_by_email`, `set_telephone`, `email_membre_de`, `set_email`, `revoke_bearer`, `change_password`, `revoke_other_sessions_for_user`, `revoke_all_for_user`, `find_user_id_by_email`, `invalidate_pending_resets`, `verify_current_password`, `prune_password_resets`, `list_memberships_for_user`, `user_labels`, `get_staff_member`, `search_staff` |
| `email_change_repo.rs` | 0 | 4 | `abandonner`, `demandes_recentes`, `est_gele`, `purger` |
| `etablissement_repo.rs` | 0 | 7 | `resoudre_domaine`, `etablissement_info`, `declarer_etablissement_for_actor`, `domaines_for_actor`, `ajouter_domaine_for_actor`, `retirer_domaine_for_actor` |
| `event_store.rs` | 0 | 2 | `publish`, `events_for_eleve` |
| `migrate.rs` | 0 | 2 | `apply_socle_migrations`, `apply_sql_file` |
| `oidc_repo.rs` | 0 | 13 | `upsert_provider`, `set_client_secret_raw`, `list_plaintext_secret_ids`, `list_providers`, `list_public`, `get_provider`, `get_provider_by_id`, `delete_provider`, `find_identity`, `link_identity`, `insert_state`, `take_state`, `prune_expired_states` |
| `outbox.rs` | 0 | 3 | `etat`, `file_des_echecs`, `rejouer_echec` |
| `passkey_repo.rs` | 0 | 10 | `list_credential_ids`, `list_passkeys`, `list_cred_rows`, `count_for_user`, `update_credential`, `delete_credential`, `delete_all_for_user`, `insert_challenge`, `take_challenge`, `prune_expired_challenges` |
| `rate_limit.rs` | 0 | 2 | `verifier`, `purger` |
| `recuperation_repo.rs` | 0 | 5 | `abandonner`, `confirmer`, `annuler_par_jeton`, `annuler_pour_user`, `purger` |
| `securite_repo.rs` | 1 | 3 | `politique_securite`, `definir_politique_securite_for_actor` |
| **total** | **1** | **92** | |


## 6. Tests
1. **Garde statique** `garde_rls_stricte::aucun_acces_direct_au_pool_hors_exception` (sans base) : aucun accès direct au pool dans `ent-db` hors fichiers transverses déclarés
   et hors 13 exceptions motivées ; une exception inutilisée fait aussi échouer le test. Un nouveau dépôt qui oublie le contexte échoue ici.
2. **Suites de domaine en mode strict** (`tests_rls_stricte.rs`) : `ENT_RLS_MODE=strict` bascule le harnais. La base est migrée et le jeu de labo semé par le **propriétaire**
   (préparation, hors périmètre testé) ; le code testé (`PgEnt` et tous ses dépôts) tourne sur un rôle `ent_suite_stricte` **non propriétaire, NOBYPASSRLS,
   `app.rls_strict = on` au niveau du rôle** : un accès sans contexte voit 0 ligne et ne peut rien écrire. Les vérifications SQL directes des tests passent par
   `PgEnt::pool_verif()` (propriétaire) ; les migrations rejouées par un test aussi. Le test `harnais_strict_est_bien_strict` prouve que le harnais est strict
   (0 ligne sans contexte, rôle réel, `TenantTx` voit son établissement).
3. **CI** (`.github/workflows/ent-rust.yml`, job `tests-rls-stricte`, 12 tranches) : rejoue en `ENT_RLS_MODE=strict` les suites du domaine **élèves et responsables** (`tests_eleve_*`,
   `tests_dossier_personnes`) et le harnais. Les mêmes tests tournent aussi en mode normal (`tests-labo`), donc la parité normal / strict est vérifiée. Résultat : voir §9.
4. Tests existants adaptés : `tests_rls.rs` (le rechiffrement sous rôle strict **fonctionne** désormais, par établissement) ; `tests_eleve_doublons` (la tâche parcourt les établissements connus de la plateforme).

## 7. Périmètre prouvé et reste à faire
**Domaine prouvé en mode strict (CI `tests-rls-stricte`) : élèves et responsables** (`tests_eleve_*`, `tests_dossier_personnes`, harnais). C'est le seul
domaine rejoué en strict à ce stade (décision du pilote après deux tours rouges : réduire le périmètre au domaine prouvé).

**Code converti mais NON rejoué en strict (donc en mode permissif)** : finances, inscriptions, santé, messagerie/communication, vie scolaire, évaluations,
structure, emploi du temps, personnels. Les accès y passent par `TenantTx` (la garde statique l'impose), la suite normale les couvre, mais leurs suites
n'ont pas été prouvées en strict en CI : elles sont **à ajouter au filtre du job `tests-rls-stricte`, domaine par domaine**, en commençant par les harnais déjà
adaptés (`tests_finances`, `tests_inscriptions*`, `tests_administratif_a0/a1/a2`, `tests_sante`, `tests_messagerie_m0a`, `tests_vie_scolaire`, `tests_evaluations`,
`tests_roles` : `labo_selon_mode` + `pool_verif()`).

**Suites dont le harnais n'est pas converti** : `tests_communication`, `tests_cahier`, `tests_evaluations_v0`, `tests_outbox`, `tests_administratif_a0bis`, structure / EDT / personnels
(`tests_structure_*`, `tests_edt_s2`, `tests_personnels_*`, `tests_d022*`…). Elles utilisent leur propre connexion et du SQL direct sur `ent.pool()`.

**Leçons de la conversion (à appliquer pour chaque domaine ajouté)**
1. Un `fetch_*` n'est pas une lecture seule : `INSERT … RETURNING` et **fonctions SQL volatiles** (`ent_rattacher_classes`, `ent_eleve_statut_recalculer`,
   `ent_assurer_annee`…) écrivent. Toute écriture exige `commit()`. Corrigés à la main : `referentiel_affecter_eleve_groupe`, `ent_rattacher_classes`, `statut_recalculer`.
   Liste des fonctions volatiles : `SELECT proname FROM pg_proc WHERE provolatile='v'`.
2. Un job qui parcourt les établissements ne doit pas filtrer côté Rust une fonction qui écrit (mode `off` : la fonction traite tout au premier appel) : `appeler_par_etablissement`.
3. Les tests directs sur `ent.pool()` (SQL de vérification) passent par `pool_verif()` en strict ; un test qui crée ses propres connexions n'est pas rejouable tel quel.
4. L'analyse statique d'un fichier ne doit pas s'arrêter au premier `#[cfg(test)]` (un champ de test isolé avait masqué ~20 accès directs dans `service.rs`).

## 8. Conditions pour basculer le mode strict en production (rien n'est basculé)
1. Tous les domaines verts en CI strict (§9) ; ceux qui ne sont pas dans le filtre (§7) à convertir **et** à passer en strict d'abord.
2. `ALTER ROLE ent_app SET app.rls_strict = 'on'` **avec** `ENT_RLS_MODE=strict` (ou `permissif`) : avec `off`, `TenantTx` ne pose rien et les tables deviennent illisibles.
3. Rôle applicatif non propriétaire (`ENT_DATABASE_MIGRATION_URL` pour les migrations) : sinon `FORCE` s'applique mais la superutilisation contourne tout.
4. Mesure de charge (3 allers-retours par appel migré, pool de 5 connexions : `RLS_ISOLATION.md` §2.9) avant d'activer.
5. Passage en environnement de test, puis pilote : la prochaine étape après le strict est la suppression du repli permissif (migration qui remplace la politique).

## 9. Résultats
- Run 42875935093 (`cf8109fd`) : rouge. Causes corrigées : écriture d'affectation de groupe non validée (régression en mode normal), `service.rs` non converti, filtre CI trop large.
- Run 42897077926 (`8b6ab91a`) : rouge, y compris en mode normal. Causes corrigées (vérifiées localement une à une) : exceptions périmées de `isolation_sql_statique`
  (les sondes de `service.rs` filtrent maintenant par établissement) ; rechiffrement sous rôle strict (droits de lecture des deux tables d'établissements + mode de contexte du test) ;
  tâche nocturne élèves (fonction `ent_eleve_statut_recalculer` dont l'écriture n'était pas validée) ; `ent_rattacher_classes` (écriture non validée).
- Résultat du dernier run : voir l'en-tête du rapport final de la session (numéro de run, SHA).

## 10. Exigences E1–E14, D-036, registre
| Exigence | Effet de ce lot |
|---|---|
| E1, E2, E3, E4, E5, E6, E10, E11 | aucune table ni colonne créée : sans objet |
| E7 | accès transverses journalisés (`ent_rls_systeme`) ; le journal d'audit métier est inchangé ; l'adhésion n'écrit pas dans `ent_journal_modif` |
| E8 | migration 600 additive, rejouable (`DROP TRIGGER IF EXISTS` + `CREATE`), idempotente ; `MIGRATION_TAG` = `600_rls_stricte_adhesion_contexte` (aucune autre branche ne porte de migration ≥ 588) |
| E9 | aucune nouvelle file d'effets de bord |
| E12 | registre inchangé (aucune table) ; son test passe |
| E13 | « test strict » : le harnais `tests_rls_stricte` rejoue les suites ; `tests_rls_lots` inchangé |
| E14 | le journal système ne contient ni identifiant d'élève ni donnée métier (nom du contexte et nombre d'établissements) |
| D-036 | aucun JSON ajouté ; les migrations 560–563 (branche `ent-d036-normalisation-json`) ne sont pas touchées ; 600+ évite toute collision |
