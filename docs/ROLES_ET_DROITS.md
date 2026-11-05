# Rôles et droits paramétrables (D-017)

Statut : conception validée dans son principe par Florian le 2026-11-05 ; étape R1 faite, R2 « domaines sensibles » faite (2026-11-05).

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
- Non basculés (hors périmètre R2 sensibles) : référentiels VS (`role_can_manage_vs_referentiels`), sorties, dispenses, alertes CPE, conseil de classe, documents à signer (restent figés par profil), ainsi que les domaines de gestion traités par une autre session.
- `incidents.declarer` du profil Prof reste à portée « établissement » pour ne rien changer ; resserrer vers « mes classes » est une décision produit à prendre.
- Les écrans d'administration Leptos des rôles restent à faire.
