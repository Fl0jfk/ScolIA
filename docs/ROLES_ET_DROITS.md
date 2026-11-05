# Rôles et droits paramétrables (D-017)

Statut : conception validée dans son principe par Florian le 2026-11-05 ; étape R1 en cours.

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
- **R2** : migration domaine par domaine des autres contrôles (`can_*` d'`ent-auth/src/roles.rs` et `role_can_*` de `service.rs`) vers `autoriser` ; le profil de base ne sert plus que de modèle ; écrans d'administration Leptos.

## Actions ajoutées par le socle 2, lot S2-1 (2026-11-05)
`personnes.gerer` (responsables, contacts, liens ; modèles admin, direction, administratif), `payeurs.definir` (+ comptabilité), `liens.suspendre` et `personnes.fusionner` (admin, direction). Portée « établissement » seulement (une personne n'est pas rattachée à une classe). Familles : toujours hors rôles libres ; leurs accès passent par le **lien actif** (`ent_eleve_lien_actif` : ni suspendu, ni hors de ses dates), lu en un point unique (`PgFoyerRepo::lien_parent_eleve`). Détail : `docs/SOCLE2_DOMAINES.md` §6.2.
