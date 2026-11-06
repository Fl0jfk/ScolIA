# Rôles et droits paramétrables (D-017)

Statut : conception validée dans son principe par Florian le 2026-11-05 ; étape R1 faite ; R2 « domaines sensibles » et R2 « domaines de gestion » implémentées et testées en local (2026-11-05), réunies sur la branche d'intégration `ent-integration-lot3`.

## Principe
ScolIA fournit une **coquille sécurisée** : le catalogue des actions, les portées, les garde-fous et le contrôle côté serveur. L'établissement paramètre l'intérieur : ses rôles, les actions cochées et leur portée, l'attribution aux personnes, ses groupes et ateliers.

## Modèle
- **Action** (catalogue fixe, code Rust `ent_auth::Action`, identifiant stable texte, ex. `notes.lire`) : ce qu'une personne peut faire. Le serveur vérifie chaque action ; aucune action ne peut être inventée par un établissement.
- **Portée** (`ent_auth::Portee`) : sur quoi l'action s'applique.
  - `etablissement` : tout l'établissement ;
  - `mes_classes` : élèves que la personne suit par l'enseignement (référentiel `enseigne_a`, EDT, remplacement en cours) ;
  - `mes_groupes` : élèves des groupes et ateliers dont la personne est **responsable** ;
  - `designes` : élèves de classes ou groupes désignés dans le rôle ;
  - `moi` : la personne elle-même (actions sur son propre compte).
- **Rôle** (`ent_role`) : nom libre choisi par l'établissement + ensemble de droits `(action, portée, cibles désignées)` (`ent_role_droit`). Modèles fournis par ScolIA, copiables et modifiables.
- **Attribution** (`ent_role_attribution`) : une personne peut avoir **plusieurs rôles** dans un établissement ; ses droits sont l'**union** de leurs droits. Attribution datée (début, fin facultative).
- **Groupes et ateliers** : créés par l'établissement dans le référentiel (`ent_groupe`) ; responsables désignés (`ent_groupe_responsable`).
- **Profil de base** (`ent_membership.role`, existant) : type de compte et modèle par défaut. Sans rôle attribué, une personne garde exactement les droits de son profil actuel (bascule sans changement de comportement).

## Garde-fous (non paramétrables)
1. **Familles et élèves hors rôles libres** : un responsable ou un élève ne reçoit jamais de rôle paramétrable ; ses droits viennent du lien légal et des droits cochés sur la fiche (`ent_eleve_lien`).
2. **Pas d'escalade** : pour créer, modifier ou attribuer un rôle, il faut l'action `roles.gerer`, et chaque droit accordé doit être couvert par les droits de l'acteur (même action, portée au moins aussi large). Seul le profil Admin échappe à cette règle.
3. **Administrateur indélébile** : au moins un compte au profil Admin par établissement ; les droits de gestion des rôles et de la sécurité ne peuvent pas être retirés au profil Admin.
4. **Actions réservées** : les actions de sécurité (`securite.politique`, `roles.gerer`) n'acceptent que la portée `etablissement`.
5. **Journal** : création, modification, suppression, attribution et retrait de rôle sont journalisés (acteur réel, avant/après).
6. **Effet immédiat** : les droits sont lus à chaque requête (pas copiés dans la session).
7. **Isolation** : rôles, attributions et groupes portent `etablissement_id` ; aucun rôle ne traverse les établissements.

## Étapes
- **R1 (fait)** : catalogue, portées, tables, modèles par défaut équivalents à l'existant, moteur `autoriser(acteur, action, cible)`, API d'administration (catalogue, rôles, attributions, responsables de groupes), anti-escalade, journal ; branchement des **lectures et écritures par élève** déjà restreintes au socle 1 (notes, saisie, compétences, absences et vie scolaire, dossier, journal, messagerie, listes VS). Un rôle « Professeur de théâtre » avec `absences.lire` portée `mes_groupes` ouvre alors la vie scolaire des élèves de l'atelier.
- **R2 (domaines sensibles et domaines de gestion : implémentés + testés en local)** — voir les deux catalogues R2 ci-dessous. Reste de R2 : migration des autres contrôles (`can_*` d'`ent-auth/src/roles.rs` et `role_can_*` de `service.rs`) vers `autoriser` ; le profil de base ne sert plus que de modèle ; écrans d'administration Leptos.

## Catalogue R2 : domaines sensibles (implémenté, testé en local)
Toutes ces actions sont portées par élève (établissement, mes classes, mes groupes, désignés) sauf `audit.lire` (établissement seul). Le modèle par défaut de chaque profil reproduit **exactement** l'ancien `Role::can_*` (test `r2_modele_identique_aux_capacites_historiques` : profil × action).

| Action | Ce qu'elle ouvre | Profils qui l'ont par défaut (établissement) |
|---|---|---|
| `sante.lire` | fiche santé, PAI (et pièces), passages, résumé PDF | Admin, Direction, Infirmerie |
| `sante.saisir` | saisie fiche, PAI, ordonnance, passages | Admin, Direction, Infirmerie |
| `psy.lire` | dossier, entretiens, rendez-vous, courrier d'orientation | Admin, Direction, Psy |
| `psy.saisir` | saisie dossier, entretiens, rendez-vous, envoi du courrier | Admin, Direction, Psy |
| `documents.deposer` | dépôt dans tous les tiroirs | Admin, Direction, Administratif |
| `documents.lire` | lecture sans grant (hors tiroir santé réservé) | Admin, Direction, Administratif |
| `documents.lire_sante` | lecture des documents de confidentialité « santé » sans grant | Admin, Direction, Administratif, Infirmerie |
| `documents.gerer_droits` | demandes d'accès (grants) : liste et décision | Admin, Direction, Administratif |
| `sanctions.prononcer` | sanctions (prononcer, annuler), commissions, soldes de points | Admin, Direction, Administratif, Cpe |
| `incidents.declarer` | déclarer un incident | Admin, Direction, Administratif, Cpe, **Prof (tout l'établissement, comportement historique)** |
| `audit.lire` | journal d'audit, dont les consultations sensibles | Admin, Direction, Administratif |

Règles :
- Contrôle **par élève** (`autoriser_eleve`) pour toute action sur un élève ; `a_action_etablissement` pour les actions sans élève (créer une commission, lister les commissions, journal d'audit) ; `eleves_autorises` pour filtrer les listes du jour (passages, entretiens, rendez-vous, grants en attente, dossiers de commission).
- Les routes ne font plus qu'un **pré-filtre** `EntSession::require_action` (au moins une portée) ; le service refait le contrôle précis. Les `require_saisir_infirmerie|psy|sanctions` et `require_declarer_incident` figés par profil sont supprimés.
- Les règles relationnelles des **familles** (lien légal, droits cochés, signalement à la famille, vues réduites) et des élèves sont inchangées. Les données de santé et psy restent tracées (`tracer_acces_sensible`, domaine `consultation`) avec l'acteur réel.
- Lire n'implique pas saisir (`sante.lire` ≠ `sante.saisir`, `psy.lire` ≠ `psy.saisir`). Un rôle ne peut accorder que ce que l'acteur couvre (anti-escalade) : une CPE avec `roles.gerer` ne peut pas donner `sante.*`, `psy.*`, `audit.lire` ni `documents.lire`.

### Limites et points ouverts R2
- Les profils sans moteur d'identité (seed, labo) gardent `list_grants_pending_for_actor`, `list_commissions_for_actor` et `list_dossiers_commission_for_actor` : ils ne regardent que le modèle du profil de base. Les routes utilisent les variantes `*_pour_acteur`.
- Non basculés par R2 sensibles : référentiels VS (`role_can_manage_vs_referentiels`), dispenses, alertes CPE (restent figés par profil). Sorties, conseils de classe et documents à signer sont basculés par R2 « domaines de gestion » (catalogue ci-dessous).
- `incidents.declarer` du profil Prof reste à portée « établissement » pour ne rien changer ; resserrer vers « mes classes » est une décision produit à prendre.
- Les écrans d'administration Leptos des rôles restent à faire.

## Catalogue R2 : domaines de gestion (implémenté, testé en local)
Toutes ces actions n'admettent que la portée `etablissement` (les contrôles d'origine portaient sur l'établissement entier ; une portée par élève sera ajoutée avec les écrans, jamais implicitement). Colonne « Profils » = modèle par défaut = droits historiques exacts (Admin = `admin`, Dir = `direction`, Adm = `administratif`).

| Action | Libellé | Profils par défaut | Remplace |
|---|---|---|---|
| `facturation.gerer` | Créer, émettre, encaisser, prélever | Admin, Dir, Adm, Compta | `can_manage_facturation` (écritures) |
| `facturation.lire` | Consulter factures, mandats, rappels, TVA, quotient | idem (`facturation.gerer` l'inclut) | `can_manage_facturation` (lectures) |
| `bourses.gerer` | Affecter et trancher les bourses | Admin, Dir, Adm, Compta | `can_manage_bourses` |
| `edt.gerer` | Exceptions, import, calendrier A/B, contraintes | Admin, Dir, Adm, CPE | `can_manage_edt` |
| `annonces.publier` | Publier des annonces | Admin, Dir, Adm, CPE | `can_publier_annonces` |
| `repas.gerer` | Activités, grilles, dispenses, menus | Admin, Dir, Adm, Compta | `can_manage_repas` |
| `repas.scanner` | Scanner les badges de cantine | Admin, Dir, Adm, Compta, CPE | `can_scan_repas` |
| `internat.gerer` | Chambres, affectations, présence, suivis | Admin, Dir, Adm, CPE | `can_gerer_internat` |
| `sorties.gerer` | Règles, autorisations, demandes exceptionnelles | Admin, Dir, Adm, CPE | `can_gerer_sorties` |
| `sorties.controler` | Contrôle au portail | Admin, Dir, Adm, CPE | `can_controle_sortie` |
| `eleves.gerer` | Élèves, foyers, liens, comptes, inscriptions | Admin, Dir, Adm | `can_manage_eleves` |
| `matieres.gerer` | Matières et compétences | Admin, Dir, Adm | `can_manage_matieres` |
| `conseils.gerer` | Conseils de classe, dossiers, mentions | Admin, Dir, Adm, CPE, Prof | `can_gerer_conseil_classe` |
| `docs_a_signer.publier` | Publier et annuler les documents à signer | Admin, Dir, Adm, CPE | `can_publier_docs_a_signer` |

### Mécanisme R2
- **Source unique** : `PgEnt::peut_etablissement(role, action)` (`ent-db/src/droits_ctx.rs`). Les anciens `role_can_*` de `service.rs` en sont des façades ; les `require_*` d'`ent-api/src/auth.rs` les appellent : une seule règle, pas de double contrôle route/service.
- **Contexte par requête** : le middleware `auth::contexte_droits` charge une fois les droits effectifs (`droits_effectifs` : modèle du profil ∪ rôles attribués actifs, lus en base → effet immédiat) et exécute le handler dans ce contexte (`tokio::task_local`).
- **Repli sûr** : sans contexte (test direct, tâche détachée) ou si le profil du contexte diffère du profil passé, on retombe sur le modèle par défaut du profil = droits historiques ; on n'obtient jamais plus que le profil par oubli.
- **Anti-escalade** : inchangée (actions identiques, portée au moins aussi large). `facturation.gerer` ne permet pas de **donner** `facturation.lire` si on ne la détient pas explicitement (les modèles par défaut détiennent les deux).
- **Oracle de non-régression** : les `Role::can_*` d'`ent-auth/src/roles.rs` sont conservés comme table de référence des tests (profil × action) ; ils ne décident plus rien.

### Limites connues de R2
- Les contrôles restent à l'établissement entier ; `can_read_all_eleves` / `can_declare_absence_staff` (lectures par profil hors R1) et `can_manage_continuum` ne sont pas migrés.
- Les tâches détachées (`tokio::spawn`) ne voient pas le contexte (repli profil, donc moins de droits, jamais plus).

## Actions ajoutées par le socle 2, lot S2-1 (2026-11-05)
`personnes.gerer` (responsables, contacts, liens ; modèles admin, direction, administratif), `payeurs.definir` (+ comptabilité), `liens.suspendre` et `personnes.fusionner` (admin, direction). Portée « établissement » seulement (une personne n'est pas rattachée à une classe). Familles : toujours hors rôles libres ; leurs accès passent par le **lien actif** (`ent_eleve_lien_actif` : ni suspendu, ni hors de ses dates), lu en un point unique (`PgFoyerRepo::lien_parent_eleve`). Détail : `docs/SOCLE2_DOMAINES.md` §6.2.

## Actions ajoutées par le socle 2, lot S2-3 (2026-11-05)
`inscriptions.gerer` (campagnes, instruction des dossiers, relances ; modèles admin, direction, administratif), `inscriptions.decider` ★ (décision d'admission et validation de l'inscription ; admin, direction — accordable au secrétariat par un rôle paramétré), `imports.appliquer` (import d'élèves et traitement des rejets ; admin, direction, administratif). Portée « établissement » seulement. Familles hors rôles : préinscription par e-mail vérifié (jeton), réinscription par le lien actif avec autorité parentale. Détail : `docs/SOCLE2_DOMAINES.md` §6.3.

## Actions et délégations ajoutées par le socle 2, lot S2-5 (2026-11-06)
`edt.annuler_masse` (annulation en masse de cours, EDT-08 ★) et `delegations.gerer` (ADM-04) : admin et direction, portée « établissement » seulement. `edt.gerer` (déjà au catalogue) couvre désormais aussi créneaux, remplacements, annulations et déplacements.

**Délégations datées (ADM-04).** Une délégation (`ent_delegation`) accorde à un **personnel** de l'établissement des droits du catalogue pour une période dont la **fin est obligatoire** (≤ 366 jours [H]). `droits_effectifs` (le point unique de décision) y ajoute, à chaque requête, les délégations dont `debut ≤ aujourd'hui ≤ fin` et non révoquées : l'expiration est la date elle-même, sans tâche de fond ; un balayage périodique ne fait qu'enregistrer `expiree` et publier `delegation.expiree` une fois. Garde-fous : jamais à une famille ou un élève (garde-fou 1) ; pas à soi-même ; **anti-escalade** (on ne délègue que ce que l'on détient, portée au plus égale, délégations reçues comprises) ; `roles.gerer`, `securite.politique` et `delegations.gerer` ne sont jamais délégables (pas de chaîne de délégations). Un remplacement validé (EDT-05) émet lui-même une délégation de remplacement (`absences.lire` limité à la classe ou au groupe, pour le jour du remplacement), révoquée si le remplacement est annulé ou changé. Détail : `docs/SOCLE2_DOMAINES.md` §6.5.


## Actions ajoutées par le socle 2, lot S2-7 (2026-11-06)
`periodes.verrouiller` (ouvrir / verrouiller les notes / fermer une période, règles de calcul), `bulletins.publier` (générer et publier les bulletins, appréciation générale hors professeur principal, décisions du conseil), `notes.corriger` (correction motivée après fermeture) : modèles admin, direction. `livrets.exporter` (LSU, LSL) : admin, direction, administratif. Portée « établissement » seulement. Les actions existantes gardent leur portée : la grille exige d'être **le professeur du service** (ou `notes.saisir` sur l'établissement) et chaque élève saisi doit être dans la portée de `notes.saisir` ; un professeur d'une autre matière de la même classe est refusé. Le professeur principal (référentiel, `professeur_principal`) saisit l'appréciation générale, les décisions du conseil et valide le bilan de cycle de sa classe. Familles : lien actif et `peut_voir_notes`, évaluations et bulletins **publiés** seulement. Détail : `docs/SOCLE2_DOMAINES.md` §6.5.
