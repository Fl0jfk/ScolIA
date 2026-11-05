# Rôles et droits paramétrables (D-017)

Statut : conception validée dans son principe par Florian le 2026-11-05 ; étape R1 en cours ; R2 (domaines de gestion) implémentée et testée en local le 2026-11-05.

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
- **R1 (en cours)** : catalogue, portées, tables, modèles par défaut équivalents à l'existant, moteur `autoriser(acteur, action, cible)`, API d'administration (catalogue, rôles, attributions, responsables de groupes), anti-escalade, journal ; branchement des **lectures et écritures par élève** déjà restreintes au socle 1 (notes, saisie, compétences, absences et vie scolaire, dossier, journal, messagerie, listes VS). Un rôle « Professeur de théâtre » avec `absences.lire` portée `mes_groupes` ouvre alors la vie scolaire des élèves de l'atelier.
- **R2 (domaines de gestion : implémenté + testé en local)** — voir « Catalogue R2 » ci-dessous. Reste de R2 : migration domaine par domaine des autres contrôles (`can_*` d'`ent-auth/src/roles.rs` et `role_can_*` de `service.rs`) vers `autoriser` ; le profil de base ne sert plus que de modèle ; écrans d'administration Leptos.

## Catalogue R2 — domaines de gestion
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
