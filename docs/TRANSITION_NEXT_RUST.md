# Transition de l'add-on Next.js vers ScolIA (Rust)

Statut : **proposé par le pilote, à valider par Florian** (2026-11-10). Dates des vacances de l'établissement données par Florian. Complète D-034 et D-035. Rien ici n'autorise le pilote à toucher à la production : chaque action sur la prod (base, Next.js, branche `main`) est faite ou autorisée explicitement par Florian.

> **Mise à jour D-057 (2026-11-11)** : Florian veut **une seule bascule complète**, invisible pour les utilisateurs, aux **vacances d'hiver (20 févr. → 8 mars 2027)** — ou aux vacances suivantes si le Rust n'est pas prêt. Pas de bascule module par module ni de double système en production. Noël = démo et répétition hors production. Le tableau ci-dessous est conservé pour l'historique ; D-057 fait foi.

## 1. Faits
- [F] Le Next.js de production tourne **en Docker chez Scaleway** (Amplify n'est plus utilisé). Scaleway héberge déjà tout le nécessaire.
- [F] Sa base est **PostgreSQL**, schéma géré par Drizzle : 78 migrations, environ 178 tables (`drizzle/*.sql`, lisibles dans le dépôt sans accéder à la prod). Exemples : `eleve`, `personnel`, `travel`, `travel_participant`, `request`, `domain_planning_booking`, `personnel_leave`…
- [F] La base ScolIA est **différente** (schéma relationnel du cœur, RLS par établissement, plus de 200 migrations dans `ent-rust/migrations/`).
- [F] Règle CLAUDE.md : le pilote n'accède jamais à la base de production, même en lecture.

## 2. Principe recommandé : on ne migre pas la base Next.js « sur place »
On ne transforme **pas** la base de production en base ScolIA. On fait une **reprise** :
1. La base Next.js reste **intacte**, source de vérité tant que le module correspondant n'est pas basculé.
2. Un outil Rust de reprise (`ent-reprise`, à écrire) lit une **copie** de la base Next.js (export `pg_dump` fait par Florian) et remplit la base ScolIA : correspondance table par table, identifiants conservés dans une table de correspondance, opération **rejouable** (on peut la relancer autant de fois que voulu, sans doublon).
3. Chaque reprise produit un **rapport de contrôle** : nombres de lignes par objet, totaux (montants des voyages, soldes), échantillons comparés, lignes refusées avec la raison. Rien n'est « perdu en silence ».
4. **Retour arrière trivial** : si quelque chose ne va pas, on continue sur Next.js, dont la base n'a pas bougé.
5. **Répétitions** : la reprise est répétée plusieurs fois sur des exports anonymisés (outil d'anonymisation fourni par le pilote, lancé par Florian), puis une fois sur un export réel en pré-production fermée, avant toute bascule.
6. Comptes : les comptes et rôles sont repris ; les mots de passe ne sont pas transférés tels quels si leur format diffère — première connexion par lien sécurisé puis passkey (D-009).

## 2 bis. La reprise est une fusion, pas une copie de tables (précision de Florian, 2026-11-10)
L'add-on était seul ; il devient une partie de l'ENT. Les données ne sont donc **pas recopiées table pour table** : elles sont **ventilées dans les tables du cœur** et **fusionnées** avec ce qui y existe déjà.
- **Doublons à fusionner** : élèves, classes, personnels, comptes, établissements, années, sites existent dans les deux bases. Règle : le cœur ScolIA fait foi (base élèves alimentée par Charlemagne/SIECLE) ; chaque fiche Next.js est **rapprochée** d'une fiche du cœur (INE, nom + prénom + date de naissance, adresse e-mail pour les personnels), puis ses données rattachées à la fiche du cœur. Rapprochement incertain : liste de cas présentée à Florian ou au secrétariat, jamais de fusion automatique douteuse (même outil que la fusion de doublons du socle 2, E6, annulable).
- **Ventilation** : ce qui était local à l'add-on rejoint le socle compétent (ex. participants d'un voyage → inscriptions au séjour rattachées à l'élève du cœur ; paiements d'un voyage → finances 24–25 ; congés du personnel → socle 6 ; réservations → salles du socle 5).
- **Tables qui n'ont plus lieu d'être** : copies locales d'élèves ou de classes, miroirs, tables techniques de l'ancienne authentification, etc. Elles ne sont pas reprises ; leur contenu utile passe par le rapprochement ci-dessus.
- **Classement de chaque module** avant toute bascule (inventaire T0, à valider par Florian) : **à conserver** (données à reprendre sans perte, ex. voyages, RH), **perdable** (on repart de zéro sans dommage), **vide** (rien à reprendre). Le pilote propose le classement à partir du schéma ; Florian le corrige d'après l'usage réel (le pilote ne voit pas la base de production).
- **Inventaire T0 fait (proposé, 2026-11-10)** : `docs/TRANSITION_T0_INVENTAIRE.md` — classement des modules, correspondance Next.js → ScolIA, règles de rapprochement, contrôles, esquisse de `ent-reprise` et de l'anonymisation. Point clé : une partie des modules vit dans des collections JSON (`ent_collection_*`) et des fichiers S3, pas seulement dans des tables typées.

## 3. Bascule module par module, pendant les vacances
Pour chaque module basculé (voyages, réservations de salles, demandes, absences et RH du personnel…) :
1. Le vendredi soir des vacances : le module Next.js passe en **lecture seule** (action de Florian).
2. Export final, reprise, rapport de contrôle relu par Florian.
3. Ouverture du module dans ScolIA ; **deux semaines de vacances** pour corriger les petits défauts avant le retour des utilisateurs.
4. Le module Next.js reste consultable en lecture seule quelques semaines, puis Florian l'arrête.
Les modules non encore basculés continuent sur Next.js comme aujourd'hui (données importées une fois).

## 4. Calendrier proposé
| Période | Phase | Contenu |
|---|---|---|
| Maintenant → mi-décembre | T0 | Inventaire du schéma Next.js (lecture du dépôt), table de correspondance Next → ScolIA, écriture et tests de `ent-reprise` sur des données fictives au format Next.js, outil d'anonymisation. |
| **Vacances de Noël** (sam. 19 déc. 2026 → reprise lun. 4 janv. 2027) | T1 | **ScolIA mis en production technique** chez Scaleway (conteneur et base séparés, sauvegardes, supervision), avec données de démonstration : c'est la démo de décembre (D-034). **Première répétition** de la reprise sur un export anonymisé de la base Next.js. Aucun utilisateur basculé : les modules Rust équivalents à l'add-on ne sont pas encore prêts. |
| Vacances d'hiver (sam. 20 févr. → reprise lun. 8 mars 2027) | T2 | Deuxième répétition (export réel en pré-production fermée, après accord écrit de l'établissement). **Première bascule** d'un module simple si prêt (ex. réservation de salles) ; sinon, répétition seulement. |
| Vacances de printemps (sam. 17 avr. → reprise lun. 3 mai 2027) | T3 | Bascule des gros modules : voyages et séjours (branchés sur élèves, administratif, finances), absences et RH du personnel, demandes. |
| Été 2027 | T4 | Derniers modules ; arrêt de Next.js ; **`ENT` devient `main`** et le code Next.js est retiré du dépôt (une étiquette d'archive le garde). Rentrée 2027 : usage réel des modules de l'add-on, doublon pour le reste (D-034, D-035). |

## 5. Pourquoi pas une bascule réelle à Noël
Les modules Rust qui remplacent l'add-on dépendent du cœur (élèves, administratif, personnels, finances), dont la V1 complète est prévue fin janvier–février 2027. À Noël, on peut mettre ScolIA en production et répéter la reprise, mais pas encore remplacer un module utilisé.

## 6. À fournir ou décider par Florian
- Accord écrit de l'établissement et avis du DPO avant toute reprise sur données réelles (avant T2).
- Les exports `pg_dump` (anonymisés d'abord), lancés par Florian.
- Le classement des modules (à conserver / perdable / vide) proposé par le pilote en T0.
- Pour chaque bascule : passage en lecture seule du module Next.js, relecture du rapport de contrôle, arrêt final.
- En T4 : autorisation explicite de faire de `ENT` la branche `main` et de retirer le code Next.js.
