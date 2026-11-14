# Sauvegardes, réplication Scaleway + OVHcloud et plan de reprise d'activité (PRA)

Statut : **proposé, à valider par Florian** (2026-11-10, branche `ent-architecture-pra`, base `ent-mc0hoe`). Réponse à la commande `docs/missions/MISSION_PRA_SAUVEGARDES.md` (§1 à §16 ; le §17 est traité dans `docs/PORTABILITE_EXPORTS.md`).
Mission de réflexion : **aucun code modifié, aucune ressource cloud créée, aucun identifiant demandé ou utilisé, aucune base de production consultée**.

Conventions : **[F]** fait vérifié (code, migration, page officielle consultée le 2026-11-10) ; **[H]** hypothèse ; **[R]** recommandation ; **[Q]** question ouverte. Les prix sont HT, relevés le **2026-11-10** sur les pages officielles citées en §17 ; certains n'ont pu être lus qu'en extrait (moteur de recherche), ils sont marqués « à reconfirmer ».

Ce document **complète et corrige** `docs/ent-rust-observabilite-pra.md` (observabilité, inventaire d'intégrité `/pra/inventaire`, scripts `pg_dump`), qui reste la référence pour ce qu'il décrit. Deux affirmations de ce document sont contredites ici (§2.3, signalées aussi dans ce document et dans `DECISIONS_AND_FAILURES.md`, F-013).

---

## Sommaire
1. Réponses courtes aux questions du §16
2. Audit de l'existant
3. Ce qui est déjà compatible, ce qui manque
4. Architecture cible
5. PostgreSQL : sauvegardes cohérentes et PITR
6. Documents et objets
7. File de réplication, états, indisponibilités
8. Ransomware, erreurs humaines, comptes
9. Chiffrement et clés
10. Restauration globale
11. Restauration d'un seul établissement
12. Tests automatiques de restauration
13. Supervision et alertes
14. Objectifs de reprise et scénarios de panne
15. Coûts et dimensionnement
16. Architecture Rust
17. Alternatives rejetées
18. Feuille de route, prérequis, décisions, exigences pour les lots en cours
19. Risques résiduels
20. Sources

---

## 1. Réponses courtes aux questions du §16

| Question | Réponse |
|---|---|
| **L'architecture actuelle permet-elle la cible ?** | **Partiellement.** Le modèle de données s'y prête bien (317 tables sur 329 portent `etablissement_id`, identifiants UUID, clés objet préfixées par l'établissement, inventaire d'intégrité existant). Trois blocages : (1) la base **managée Scaleway** ne documente ni PITR ni accès aux WAL (consulté le 2026-11-10) : le RPO 15 min et la double destination des WAL sont **impossibles** sur cette offre telle que documentée ; (2) `ent-object` n'écrit pas réellement dans S3 (disque local source de vérité, envoi HTTP non signé et « best-effort ») ; (3) aucun moteur ne sait extraire ou réinjecter **un** établissement. |
| **Composants à prévoir dès maintenant ?** | Des **règles de modèle de données** (§18.4) et le **registre de classement des tables** testé en CI ; la sortie des fichiers de la base (`BYTEA`) ; l'identifiant de clé dans le format chiffré ; la décision sur l'hébergement PostgreSQL (§5.2). Pas de code d'infrastructure de sauvegarde avant T1. |
| **Ce qui peut attendre ?** | Troisième copie froide, infrastructure de reprise « tiède » chez OVHcloud, chiffrement côté client des documents, chaîne de hachage du journal, exports planifiés. |
| **Coût ?** | Hors base de production (déjà nécessaire) : **≈ 10–20 €/mois** à 1–10 établissements, **≈ 70–100 €/mois** à ~50, **≈ 350–450 €/mois** à ~300 (§15). Soit 1,3 à 3 € par établissement et par mois. |
| **Si Scaleway tombe ?** | **L'ENT est indisponible** (production sur un seul fournisseur) ; **les données sont protégées** chez OVHcloud (perte ≤ quelques minutes pour la base, ≤ retard de réplication pour les documents). Remonter le service chez OVHcloud prend **8 à 24 h** les premières fois, **≈ 4 h** seulement après répétitions et automatisation (§14). La double sauvegarde ne garde pas l'ENT accessible. |
| **Si OVHcloud est indisponible ?** | Aucun impact pour les utilisateurs. **Protection dégradée** affichée : une seule copie hors production (Scaleway). Rattrapage automatique au retour. |
| **Si les deux le sont temporairement ?** | Si la production (Scaleway) tourne : le service continue, les WAL et objets s'accumulent localement (disque dimensionné pour plusieurs jours), alerte critique « aucune copie externe récente ». Si la production **et** OVHcloud sont tombées : rien à restaurer avant le retour de l'un des deux ; c'est le cas que seule une troisième copie (froide, hors ligne) couvrirait. |
| **Restaurer toute la plateforme ?** | PITR de la base depuis l'un des deux dépôts, restauration des objets par versions, réinjection des secrets depuis le séquestre, image applicative au bon `MIGRATION_TAG`, vérification par inventaire, réouverture (§10). |
| **Restaurer un établissement ?** | Jamais directement en production : PITR complet dans un **bac isolé**, extraction de l'établissement par le moteur d'extraction, comparaison, validation humaine, puis **réintégration contrôlée** (sélective ou remplacement), journalisée (§11). |
| **Garantir qu'une sauvegarde est restaurable ?** | Seulement en la **restaurant** : tests automatiques hebdomadaires depuis **chaque** destination, mensuels pour un établissement, trimestriels pour la reprise complète depuis OVHcloud seul, annuels pour la récupération des clés (§12). État « restauration vérifiée » distinct de « copie présente ». |
| **Risques résiduels ?** | §19 : compromission simultanée des deux comptes et du séquestre, bug logique non détecté au-delà de la rétention, perte des clés, opérateur de plateforme malveillant, RTO non tenu à grande échelle pour les documents. |

---

## 2. Audit de l'existant (lecture seule, 2026-11-10)

Mesures par analyse statique des 210 fichiers de `ent-rust/migrations/` (script de comptage hors dépôt ; ordres de grandeur, une table créée dynamiquement peut échapper au comptage).

### 2.1 Modèle de données

| Constat | Mesure | Conséquence |
|---|---|---|
| Tables portant `etablissement_id` | **317 / 329** [F] ; `NULL` autorisé sur 2 (`ent_eleve_orphelin`, `ent_fonction`) | Excellent point de départ pour extraire un établissement. |
| Tables **sans** `etablissement_id` | 12 [F] : `ent_user`, `ent_etablissement`, `ent_oidc_identity`, `ent_password_reset`, `ent_login_guard`, `ent_rate_limit`, `ent_profil_personnel`, `ent_systeme_identifiant`, `ent_absence_motif`, `ent_coord_reprise_marque`, `ent_note_doublon_archive`, `ref_calendrier_officiel` | Chacune doit être classée : globale de référence, identité partagée, volatile, ou **à corriger** (§18.4, E1). |
| Identité partagée | `ent_user` est référencée par **163** clés étrangères [F] ; une personne a plusieurs établissements (D-013) | Une restauration d'établissement **ne doit jamais** réécrire `ent_user` (mot de passe, passkeys, autres rattachements changés depuis) ; elle ne fait que rattacher. |
| Clés étrangères | 566 au total, **69** composites `(etablissement_id, …)` [F] | Les ~500 autres ne garantissent pas en base qu'une ligne ne pointe pas vers un autre établissement. L'extraction devra **vérifier** cette fermeture (§11.3) ; le lot C5 (FK composites, `RLS_ISOLATION.md`) la rendrait structurelle. |
| Référence à `ent_etablissement` | **1** seule FK vers `ent_etablissement(id)` (`ent_etablissement_domaine`) [F] | `etablissement_id` n'est presque nulle part contraint : des identifiants orphelins sont possibles ; le contrôle doit être fait par l'extracteur. |
| Identifiants | 269 tables à clé `id UUID` [F] ; 2 séquences : `ent_outbox.seq` (BIGSERIAL global), `ent_eleve_fusion_ligne` [F] | UUID = réinsertion sans collision entre établissements. `ent_outbox` est volatile (à neutraliser, §11.4). |
| Suppressions en cascade | 102 `ON DELETE CASCADE` [F] | Un `DELETE` erroné efface en profondeur : à restaurer par PITR ; un remplacement d'établissement doit les maîtriser. |
| Horodatages | 30 tables sans `created_at`/`*_le` visible ; 70 avec date de modification [F] | Les comparaisons « qu'est-ce qui a changé depuis T » sont impossibles sur ces tables ; exigence E6. |
| JSON | 22 tables avec colonne JSON/JSONB [F] (D-036 en cours de reprise) | Réduit la lisibilité des exports ; D-036 aide directement la portabilité. |
| Contenu binaire en base | `ent_msg_piece.contenu` et `ent_pai_piece.contenu` en `BYTEA` [F] | Gonfle la base, les WAL et chaque sauvegarde ; contourne les URL pré-signées. À sortir (E5). |

### 2.2 Isolation et droits
- **RLS** [F] : ≈ 63 tables couvertes (migrations 100, 206, 215, 280–292, 330–361, 460, 480), mode **`off` par défaut** (`ENT_RLS_MODE`), variable `app.etablissement_id` posée par transaction (`TenantTx`). Rôles `ent_owner` / `ent_app` conçus mais l'application rejoue encore les migrations avec le même rôle (`RLS_ISOLATION.md` §2.1). **Atout pour l'extraction** : un extracteur qui lit sous `TenantTx` en mode strict ne peut matériellement pas lire un autre établissement sur ces tables.
- **Journal `ent_journal_modif`** [F] : ajout seul par déclencheur (091) ; écriture « best-effort » ; `TRUNCATE` non bloqué ; le propriétaire peut désactiver le déclencheur ; `payload JSONB` (instantané, permis par D-036) ; `eleve_id` référencé **sans** cascade (bloque la suppression d'un élève journalisé — voulu pour la conservation, à prévoir dans les purges).
- **Délégations** (`172_delegation`) [F] : datées, fin obligatoire, `droits JSONB` (à normaliser, D-036). Réutilisables pour la suppléance des approbateurs d'export (`PORTABILITE_EXPORTS.md` §5).
- **Catalogue d'actions** [F] : `exports.officiels`, `dossier_admin.exporter`, `livrets.exporter`… (`ROLES_ET_DROITS.md`). Aucune action d'export **complet** de l'établissement.

### 2.3 Sauvegardes et PRA existants — et deux corrections
- `scripts/sauvegarde-postgres.sh` / `restauration-postgres.sh` / `pra-exercice.sh` [F] : `pg_dump --format=custom` + SHA-256 + manifeste + rétention locale ; garde-fou loopback. Logique, **pas de PITR**, pas de destination distante.
- `/pra/inventaire` [F] : empreinte ligne à ligne par table (établissement ou global) + empreinte de schéma + `migration_tag`. **Brique précieuse** : c'est la preuve de fidélité d'une restauration, réutilisée §12.
- **Correction 1** : `ent-rust-observabilite-pra.md` §2.1 annonce « snapshots + PITR de la RDB Scaleway, RPO quelques minutes ». Les pages Scaleway consultées le 2026-11-10 décrivent une **sauvegarde automatique quotidienne** (dump logique, rétention 7 jours par défaut, stockée dans une autre région) et des **snapshots** de disque ; **aucun PITR ni accès aux WAL** n'y est documenté [F, à reconfirmer auprès du support Scaleway, §18.2 P1]. RPO réel de la base managée seule : **jusqu'à 24 h**.
- **Correction 2** : le même document annonce des objets « bucket S3/Scaleway, versionné (`ent-object`) ». Dans le code [F] (`ent-object/src/lib.rs`), le **disque local** (`ENT_OBJECT_DIR`) est la source de vérité ; l'envoi S3 est un `PUT` HTTP **non signé** (pas de SigV4), facultatif (`ENT_OBJECT_S3_HTTP`) et dont l'échec est **ignoré** (`let _ = http_put_object(…)`). Seule la lecture pré-signée SigV4 est réelle (`presign.rs`). Le versionnage n'existe que si on l'active sur le bucket.
- **Migrations** [F] (`ent-db/src/migrate.rs`) : les 210 fichiers sont **rejoués intégralement à chaque démarrage**, instruction par instruction, **hors transaction**, sous verrou advisory, sans table de suivi. Conséquences pour une restauration : (a) une base restaurée à T puis démarrée avec une image plus récente reçoit les migrations postérieures à T — sûr **seulement** si chaque migration est additive et rejouable (règle actuelle, F-009) ; (b) une image plus **ancienne** que la base restaurée ne doit jamais être démarrée (migrations « inconnues » déjà appliquées, code incompatible) ; (c) un démarrage interrompu laisse une migration à moitié appliquée. D'où l'exigence de noter `MIGRATION_TAG` et `GIT_SHA` dans chaque manifeste de sauvegarde (E8).
- **Image** [F] : `Dockerfile` multi-étages, `GIT_SHA` injecté, `/health` expose `version`, `git_sha`, `migration_tag`. Bon : on sait toujours quelle image correspond à quelle base.
- **Secrets** [F] : `ENT_SECRETS_KEY` (AES-256-GCM, champs IBAN/santé/psy, format `enc:v1:` **sans identifiant de clé**), `ENT_OBJECT_SIGNING_KEY`, `ENT_OBJECT_S3_ACCESS_KEY`/`SECRET_KEY`, clés SMS OVH, push. Sans `ENT_SECRETS_KEY`, une sauvegarde restaurée a ses champs sensibles **illisibles** : cette clé est aussi critique que la sauvegarde elle-même (§9).
- **Authentification** [F] : sessions avec `assurance` (`mot_de_passe` / `passkey` / `sso`) et `portee` (095) ; réauthentification **par mot de passe** sur routes sensibles (`reauth:<user_id>`, 5 échecs / 15 min). **Aucune réauthentification récente par passkey** (pas d'horodatage d'authentification forte dans la session) : manque pour la double validation des exports (§17 de la commande).
- **Tables volatiles** [F] (`TABLES_VOLATILES`) : `ent_session`, `ent_passkey_challenge`, `ent_oidc_state`, `ent_login_guard`, `ent_password_reset` — déjà exclues de l'inventaire. **Manquent** : `ent_outbox`, file de courriels, SMS, push (§11.4).

### 2.4 Ce qui rend difficile la restauration ou l'export d'**un** établissement (synthèse)
1. Identité partagée (`ent_user`, passkeys par personne, D-013) : 163 FK vers une table globale.
2. ~500 FK simples : rien en base n'empêche une référence vers un autre établissement ; il faut le contrôler.
3. `etablissement_id` non contraint vers `ent_etablissement` : orphelins possibles.
4. Fichiers en `BYTEA` et fichiers sur disque local : deux circuits différents.
5. Effets de bord en attente (outbox, courriels, SMS) : une restauration naïve **renverrait** des messages aux familles.
6. Journal en ajout seul : on n'efface pas l'historique de production ; la restauration **ajoute** des écritures.
7. Migrations rejouées : le bac de restauration doit être porté au schéma courant avant comparaison.
8. Tables sans horodatage : impossible de savoir ce qui a changé depuis T sur 30 tables.
9. Une PITR PostgreSQL est **globale** (toute l'instance) : pas de PITR d'un seul établissement sans passer par un bac isolé.

---

## 3. Déjà compatible / manques

| Déjà compatible [F] | Manques |
|---|---|
| `etablissement_id` quasi partout, UUID, RLS pilote, `TenantTx` | PITR et WAL hors fournisseur (base managée) |
| Clés objet `{etablissement}/{espace}/{id}.{ext}`, URL pré-signées courtes, `key_belongs_to_etab` | Écriture S3 réelle et signée ; empreinte et version stockées en base |
| Inventaire d'intégrité ligne à ligne (`/pra/inventaire`) | Moteur d'extraction / réinjection par établissement |
| Journal en ajout seul, catalogue d'actions D-017, délégations datées | Réauthentification par passkey ; action d'export complet ; double validation |
| `GIT_SHA` + `MIGRATION_TAG` exposés | Manifeste de sauvegarde, file de réplication, états par destination, supervision |
| Chiffrement applicatif des champs sensibles | Séquestre des clés hors Scaleway ; identifiant de clé ; rotation |
| Image conteneur reproductible | Infrastructure décrite en code (pour remonter ailleurs), registre d'images hors Scaleway, DNS hors Scaleway |

---

## 4. Architecture cible (progressive)

```
                 PRODUCTION (Scaleway, fr-par)                     DESTINATIONS DE SAUVEGARDE (actives toutes les deux)
 ┌─────────────────────────────────────────────┐          ┌───────────────────────────────────────────────┐
 │ ent-api (conteneur)                         │          │ A. Scaleway — organisation/projet « sauvegarde »│
 │  └─ écrit les fichiers dans le bucket prod  │──objets─▶│    séparé de la prod, bucket Object Lock,      │
 │ PostgreSQL (instance, cf. §5.2)             │          │    versionné, fr-par (France)                  │
 │  └─ pgBackRest : archive WAL asynchrone     │──WAL────▶│    repo1 pgBackRest + copie des objets         │
 │     + sauvegardes complètes/diff par dépôt  │──WAL────▶├───────────────────────────────────────────────┤
 │ worker ent-sauvegarde (même image ou petit  │──objets─▶│ B. OVHcloud — compte distinct, Paris 3-AZ,     │
 │  conteneur) : file, réplication objets,     │          │    bucket Object Lock, versionné               │
 │  manifestes, vérifications, tests           │          │    repo2 pgBackRest + copie des objets         │
 └─────────────────────────────────────────────┘          ├───────────────────────────────────────────────┤
          │  manifestes signés (dans chaque destination)    │ C. (plus tard) copie froide mensuelle         │
          ▼                                                 │    immuable : OVHcloud Cold Archive           │
   métriques /metrics + tableau de bord                     └───────────────────────────────────────────────┘
   séquestre des clés : hors Scaleway (§9)
```

Principes [R] :
- **Deux destinations symétriques**, chacune suffisante seule pour restaurer ; aucune n'est « secours » de l'autre.
- **Pas de seconde production** : pas de base multi-maître, pas d'application permanente chez OVHcloud au lancement.
- Les sauvegardes Scaleway natives de la base managée (si on la garde) sont un **bonus**, pas une destination comptée.
- La destination Scaleway est dans une **organisation (ou au minimum un projet) distincte** de la production, avec des identifiants qui ne peuvent qu'écrire (§8).
- Les deux destinations sont **en France** (Scaleway fr-par, OVHcloud Paris 3-AZ), cohérent avec `MISE_EN_SERVICE_DONNEES.md` A6 ; à confirmer par le DPO.

---

## 5. PostgreSQL : sauvegardes cohérentes et PITR

### 5.1 Ce qui est cohérent, ce qui ne l'est pas
Une copie de fichiers d'une base en marche n'est **jamais** une sauvegarde. Sont cohérents : `pg_dump` (instantané transactionnel, logique, sans PITR) ; une sauvegarde physique prise par l'API de sauvegarde de PostgreSQL (`pg_backup_start`/`stop`, `pg_basebackup`, pgBackRest, WAL-G) **plus** les WAL de la fenêtre ; un snapshot de disque du fournisseur pris de façon atomique.

### 5.2 Où faire tourner PostgreSQL — décision structurante [R, D-039 proposée]
Faits [F, 2026-11-10] : **Scaleway Managed Database** : sauvegarde automatique quotidienne, rétention 7 j par défaut, snapshots ; PITR et accès WAL **non documentés** ; HA et réplicas de lecture disponibles. **OVHcloud Public Cloud Databases** : PITR continu documenté (RPO « quelques minutes ») mais dans le stockage d'OVH, sans archivage vers nos propres dépôts ; rétention 2 à 30 j selon l'offre. Tous les outils PITR (pgBackRest, WAL-G, Barman) exigent `archive_command` ou la réplication physique : **impossible sur une base managée** qui ne les ouvre pas.

| Option | RPO base | Double destination des WAL | Exploitation | Verdict |
|---|---|---|---|---|
| A. Base managée Scaleway + nos `pg_dump` horaires vers A et B | ≈ 1 h (24 h si on ne compte que Scaleway) | non (dumps seulement) | minimale | **Acceptable pour T1 et l'année de doublon** (Charlemagne reste officiel, D-034) ; insuffisant pour la cible. |
| B. **PostgreSQL autogéré sur instance Scaleway + pgBackRest, repo1 Scaleway, repo2 OVHcloud** | ≤ 1–5 min (`archive_timeout` 60 s) | **oui** | mises à jour, supervision, HA à construire | **Cible [R]** avant le premier usage réel des modules de l'add-on (rentrée 2027, D-035). |
| C. Base managée OVHcloud (PITR inclus) | minutes, mais chez OVH | non | minimale | Rejetée : déplace la production et ne donne pas de copie indépendante. |
| D. Base managée Scaleway + réplication logique vers un PostgreSQL autogéré qui, lui, archive | minutes | oui | complexe (DDL non répliqué, migrations rejouées) | Rejetée : fragile avec nos migrations rejouées. |

**[Q-PRA1]** Demander par écrit au support Scaleway si un PITR (ou l'accès aux WAL / à la réplication physique) existe ou est prévu sur Managed Database for PostgreSQL. S'il existe : l'option A' (managée + PITR Scaleway + `pg_dump` horaires vers OVHcloud) devient une alternative sérieuse à B pour la première version.

### 5.3 Paramètres proposés (option B) [R, à mesurer]

| Paramètre | Valeur proposée | Pourquoi |
|---|---|---|
| Outil | **pgBackRest** (MIT) ; WAL-G (Apache-2.0) en alternative | multi-dépôts documenté, chiffrement client, `verify`, `--db-include`, S3 path-style, reprise des transferts |
| Dépôts | `repo1` = Scaleway (bucket sauvegarde), `repo2` = OVHcloud | archive WAL vers **les deux** ; sauvegardes planifiées **par dépôt** (chacun complet seul) |
| Archivage | `archive-async=y`, file locale (`spool`) sur disque dédié | un dépôt en panne ne bloque pas l'autre ; avec l'asynchrone, les dépôts sains avancent |
| `archive_timeout` | **60 s** | borne la perte à ~1 min même en période creuse (vacances : volume quasi nul, segments compressés) |
| `archive-push-queue-max` | dimensionné à **plusieurs jours** de WAL, alerte bien avant | au-delà, pgBackRest **abandonne** les WAL : PITR rompu jusqu'à la sauvegarde complète suivante — à éviter absolument |
| Complète | hebdomadaire (dimanche nuit) par dépôt | |
| Différentielle | quotidienne (nuit) par dépôt | temps de restauration borné |
| Compression | `zst` niveau 3 | bon ratio, peu de CPU |
| Chiffrement | `repo-cipher-type=aes-256-cbc`, **une phrase de passe distincte par dépôt**, séquestrée (§9) | le fournisseur ne lit rien ; perte d'un dépôt ≠ perte de l'autre clé |
| Rétention PITR | **35 jours** sur chaque dépôt (5 complètes) | couvre un mois scolaire + détection tardive ; à valider DPO |
| Rétention historique | 1 complète par mois gardée **12 mois** (plus tard : copie froide C) | corruption logique découverte tard ; à valider DPO |
| Vérification | `pgbackrest check` quotidien, `pgbackrest verify` hebdomadaire par dépôt, test de restauration (§12) | |
| Volume WAL [H] | ≈ 50–150 Mo/jour compressés par établissement en période scolaire, ≈ 0 en vacances | à mesurer en T1 avec `pg_stat_wal` |

Option A (transitoire) : `pg_dump --format=custom` horaire par le worker, chiffré, envoyé aux deux destinations avec le même manifeste ; garder les 48 derniers horaires, 35 quotidiens, 12 mensuels.

### 5.4 Contrôle des chaînes
- pgBackRest vérifie la continuité des WAL de chaque sauvegarde ; `verify` détecte les segments manquants ou corrompus ; le worker publie par dépôt l'âge du **dernier WAL archivé** et du **dernier segment vérifié**.
- La base restaurée en test est comparée par `/pra/inventaire` (global) à un inventaire pris à l'instant cible (§12).

---

## 6. Documents et objets

### 6.1 Capacités réelles des fournisseurs (2026-11-10)

| | Scaleway Object Storage | OVHcloud Object Storage (S3) |
|---|---|---|
| Versionnage | oui ; une fois activé, seulement suspendable | oui |
| Object Lock | oui, GOVERNANCE et COMPLIANCE, rétention par défaut du bucket, legal hold ; **à activer à la création** du bucket, versionnage requis | oui, GOVERNANCE et COMPLIANCE, rétention par défaut, legal hold ; **à la création seulement** ; **pas sur Cold Archive** d'après la FAQ (la page produit dit l'inverse : à vérifier) |
| Cycle de vie | expiration, transitions vers One Zone / Glacier, expiration des versions non courantes | expiration, transitions (dont Cold Archive) |
| Réplication native | **non trouvée** | asynchrone entre buckets **d'un même projet**, versionnage requis, objets SSE-C non répliqués, suppressions non répliquées par défaut |
| Chiffrement serveur | SSE-ONE, SSE-C, SSE-KMS (Key Manager) | SSE-OMK, SSE-C |
| Droits fins | IAM : écriture et suppression séparées (`ObjectStorageObjectsWrite` / `…Delete`), bucket policies | utilisateurs S3 par projet, politiques JSON par utilisateur avec `Deny` ; **bucket policies pas encore disponibles** |
| Classes froides | Glacier (0,00246 €/Go/mois, restauration 0,009 €/Go, délai minutes à 48 h) | Cold Archive (0,00166 €/Gio/mois, bandes, minimum 180 j, extraction 0,009 €/Gio, délai « en heures ») |

Conséquence : **S3 n'est pas identique partout**. La réplication **ne peut pas** reposer sur les fonctions natives (absentes chez Scaleway, limitées au même projet chez OVHcloud) : c'est **notre worker** qui copie, vers deux comptes indépendants.

### 6.2 Règles [R]
1. **Objets immuables** : une clé n'est jamais réécrite ; une nouvelle version d'un document = nouvelle clé (UUID). Supprimer = marquer en base, l'objet expire plus tard. Cela rend la réplication triviale (copier ce qui manque) et neutralise le « chiffrement malveillant » par réécriture.
2. **Métadonnées en base** pour chaque objet : `cle_objet`, `taille`, `sha256`, `mime`, `cree_le`, `supprime_le` (E5). L'empreinte est calculée à l'écriture, avant l'envoi.
3. **Bucket de production** : versionné, privé, SSE-ONE au minimum (A3). Les identifiants de l'application ont écriture et lecture, **pas** la suppression de versions.
4. **Buckets de sauvegarde (A et B)** : versionnés, **Object Lock activé à la création**, rétention par défaut **GOVERNANCE 35 jours** au lancement (passage en COMPLIANCE après une période d'essai, une fois la procédure maîtrisée : en COMPLIANCE personne, pas même nous, ne peut raccourcir).
5. **Suppressions** : le worker ne propage une suppression aux destinations qu'**après le délai de rétention** (35 j) : une suppression ou une corruption en production n'atteint pas immédiatement les copies.
6. **Manifeste** par lot de réplication et **manifeste quotidien complet** par destination : liste `cle, taille, sha256, version` ; stocké dans chaque destination, signé (Ed25519). C'est la base de la réconciliation (§7.4).
7. **Vérification après transfert** : comparer la somme de contrôle renvoyée par le fournisseur (en-tête `x-amz-checksum-sha256` si pris en charge, sinon relecture d'un échantillon et `ETag` hors multipart) à l'empreinte en base ; échantillon quotidien relu intégralement (1 % des objets, et tous ceux de moins de 24 h).
8. **Transferts reprenables** : envoi multipart au-delà de 16 Mo ; un multipart interrompu est repris ou abandonné (règle de cycle de vie « abort incomplete multipart » à 2 jours).
9. **Préfixe d'exports exclu** de la réplication (les archives d'export sont temporaires, `PORTABILITE_EXPORTS.md` §8).

---

## 7. File de réplication, états, indisponibilités

### 7.1 Ce qui est répliqué par quoi
- **WAL et sauvegardes PostgreSQL** : pgBackRest gère sa propre file (spool local) et ses deux dépôts. Le worker ne fait que **lire son état** (`pgbackrest info --output=json`) et l'enregistrer.
- **Objets** et **manifestes** : le worker `ent-sauvegarde`.

### 7.2 Persistance de la file [R]
- File dans PostgreSQL (tables `ent_sauvegarde_*`, §16.2) : transactionnelle, déjà sauvegardée. Si Scaleway tombe entièrement, la file tombe avec la production — mais **plus rien n'est produit** ; il n'y a rien à rattraper côté production.
- **La file n'est pas la vérité** : l'état réel se reconstruit à tout moment en comparant la **liste de la source** (base : objets référencés ; bucket prod) aux **manifestes et listes de chaque destination**. Une file perdue se reconstruit par réconciliation. C'est ce qui rend la file indépendante du fournisseur dont on tolère la panne.
- Chaque tâche a un **identifiant stable** : `(objet_id, version, destination)` pour un objet ; `label pgBackRest` pour une sauvegarde. Rejouer une tâche est sans effet (PUT d'un objet identique à clé identique, vérifié par empreinte).

### 7.3 États explicites (par élément et par destination)

| État | Condition stricte |
|---|---|
| `produite` | sauvegarde ou objet écrit à la source, empreinte connue |
| `copie_scaleway_confirmee` | écrit **et** vérifié (empreinte) dans A |
| `copie_ovh_confirmee` | écrit **et** vérifié dans B |
| `copie_unique` | une seule des deux confirmée |
| `double_copie_confirmee` | **les deux** confirmées et vérifiées ; jamais déduit d'un simple PUT réussi |
| `replication_en_retard` | non confirmée sur une destination au-delà du seuil (WAL : 15 min ; objet : 1 h ; sauvegarde : 6 h) |
| `integrite_echouee` | empreinte divergente ou `verify` en erreur → recopie depuis la source, alerte |
| `restauration_possible` | chaîne complète (complète + WAL continus) présente sur la destination |
| `restauration_verifiee` | une restauration réelle depuis cette destination a réussi et l'inventaire concorde (date du test) |

État **global** affiché : `double protection confirmée` / `protection dégradée (une destination)` / `aucune copie externe récente` (critique).

### 7.4 Comportement en cas d'indisponibilité
- Tentatives avec **temporisation exponentielle** (30 s → 1 h, plafond), par destination, indépendantes.
- Une destination en panne ne retarde jamais l'autre (files séparées par destination).
- **Au retour** : réconciliation (liste destination vs. manifeste source), on ne recopie **que les manquants** ; aucun objet existant n'est recréé (PUT conditionnel `If-None-Match: *` si pris en charge, sinon test d'existence + empreinte).

| Scénario | Traitement |
|---|---|
| Panne partielle (erreurs 5xx intermittentes) | tentatives ; état `en_retard` si le seuil est franchi |
| Panne totale d'une destination | `protection dégradée`, accumulation, rattrapage au retour |
| Doublons (tâche rejouée deux fois) | sans effet : même clé, même empreinte |
| Transfert interrompu | multipart repris ; sinon recommencé ; jamais confirmé sans vérification |
| Divergence entre manifestes | l'**empreinte en base** fait foi ; objet absent ou différent → recopié ; objet présent en destination mais inconnu en base → signalé (jamais supprimé automatiquement) |
| Disque de spool WAL qui se remplit | alerte à 50 % ; à 80 % alerte critique ; ne jamais atteindre `archive-push-queue-max` |

---

## 8. Ransomware, erreurs humaines, comptes

Deux copies synchronisées ne protègent pas d'une suppression ou d'un chiffrement propagés. Protections [R] :
1. **Immutabilité** : Object Lock sur les deux destinations (§6.2) ; rétention 35 j ; versions conservées.
2. **Comptes séparés** : production (organisation Scaleway « prod ») ; sauvegarde Scaleway (organisation ou projet distinct, autre administrateur que celui du quotidien) ; OVHcloud (compte distinct). Double authentification forte sur chaque compte humain (A5).
3. **Identifiants de service minimaux** : le worker de production a sur chaque destination **écriture et lecture, sans suppression** ; la purge des versions expirées est faite par les **règles de cycle de vie** du bucket, pas par un identifiant. Les identifiants de restauration (lecture seule) sont séparés et hors production.
4. **Compte de production compromis** : l'attaquant peut détruire la production et écrire des objets dans les destinations, **pas** effacer les versions verrouillées. Il ne peut pas lire les sauvegardes PostgreSQL (chiffrées par pgBackRest avec des phrases de passe que la production n'a… **qu'à condition** de les tenir hors de la machine, §9.3).
5. **Corruption logique** (bug, migration fautive, erreur de saisie de masse) : PITR à l'instant d'avant dans un bac isolé, puis restauration sélective (§11) ; la rétention de 35 j + 12 mensuelles conserve des générations saines.
6. **Chiffrement malveillant** via l'application : objets immuables (§6.2-1) + versions verrouillées ; détection par alerte « taux de réécriture/suppression anormal » (§13).
7. **Troisième copie** : utile surtout contre la perte simultanée des deux comptes ou une erreur dans notre propre worker. Coût faible en Cold Archive (§15) ; **non nécessaire au lancement**, recommandée à ~50 établissements ou avant la commercialisation (D-044 proposée).

---

## 9. Chiffrement et clés

### 9.1 Couches
| Couche | Mécanisme |
|---|---|
| Transit | TLS partout (pgBackRest, worker, application) |
| Repos (fournisseur) | SSE-ONE / SSE-KMS (Scaleway), SSE-OMK (OVHcloud) ; chiffrement au repos du volume PostgreSQL (A2) |
| Avant transfert (client) | sauvegardes PostgreSQL chiffrées par pgBackRest (AES-256) — le fournisseur ne lit rien ; documents : SSE au lancement, **chiffrement côté client** en v2 (§18.1) |
| Applicatif | champs sensibles `ENT_SECRETS_KEY` (déjà en place) ; donc **double** chiffrement dans les sauvegardes |

### 9.2 Inventaire des clés nécessaires à une restauration
| Clé | Sans elle | Séquestre |
|---|---|---|
| Phrase de passe pgBackRest repo1 | repo1 illisible | oui |
| Phrase de passe pgBackRest repo2 | repo2 illisible | oui |
| `ENT_SECRETS_KEY` (et anciennes versions) | IBAN, santé, psy **illisibles** dans toute restauration | **oui, critique** |
| Clé de signature des manifestes (Ed25519) | on ne peut plus signer ; la vérification reste possible avec la clé publique | oui |
| Identifiants d'accès aux destinations | se recréent depuis les consoles (si l'accès aux comptes est conservé) | accès aux comptes : oui |
| `ENT_OBJECT_SIGNING_KEY`, clés SMS/push/SMTP | se régénèrent ou se redemandent au fournisseur | non critique |

### 9.3 Règles [R]
- Aucune clé de restauration ne dépend **uniquement** de Scaleway : production dans **Secret Manager** (A4) ; **séquestre** en deux exemplaires hors ligne (enveloppes scellées ou clés matérielles chiffrées, une chez Florian, une chez un tiers de confiance de l'établissement ou chez un notaire) **et** une copie dans un coffre hors Scaleway (gestionnaire de mots de passe d'équipe, ou OVHcloud KMS / Secret Manager si retenu).
- pgBackRest a besoin de ses phrases de passe sur la machine PostgreSQL pour écrire. Compromis [R] : elles sont dans un fichier de configuration lisible par l'utilisateur `postgres` seulement ; le risque « compromission de la machine DB = lecture des sauvegardes » est accepté (l'attaquant a déjà la base) ; la protection contre l'**effacement** vient d'Object Lock, pas du chiffrement.
- **Format chiffré avec identifiant de clé** : `enc:v2:<id_cle>:…` (E11), pour permettre la rotation sans tout rechiffrer en une fois, et pour savoir quelle clé historique une vieille sauvegarde exige.
- **Rotation** : `ENT_SECRETS_KEY` annuelle ou après incident ; les anciennes versions restent au séquestre tant qu'une sauvegarde chiffrée avec elles existe (35 j + 12 mois).
- **Exercice annuel** : reconstruire les clés depuis le séquestre seul, sans accès à Scaleway, et restaurer depuis OVHcloud (scénario du §7 de la commande).
- Jamais de mécanisme qui rendrait irrécupérables les sauvegardes si un compte est perdu : pas de chiffrement « KMS du fournisseur seulement » pour les sauvegardes (une clé KMS Scaleway non exportable perdue = sauvegardes perdues ; l'export de clés Key Manager **n'est pas documenté** au 2026-11-10).

---

## 10. Restauration globale (runbook cible)

1. **Décider** (humain, Florian) : incident qualifié, cible (dernier instant ou instant avant corruption), destination source (A ou B).
2. **Geler** : mettre l'ENT en maintenance (page d'état), couper les workers d'envoi.
3. **Infrastructure** : instance PostgreSQL + instance applicative chez le fournisseur choisi (décrites en code : Terraform/OpenTofu MPL-2.0, ou scripts `scw`/`openstack`) ; image depuis un **registre hors Scaleway** aussi (copie de l'image dans GHCR ou le registre OVHcloud).
4. **Clés** depuis Secret Manager, ou depuis le séquestre si Scaleway est perdu.
5. **Base** : `pgbackrest --repo=N restore --type=time --target=…` ; démarrage en recovery ; promotion.
6. **Image** : celle dont `MIGRATION_TAG` ≥ celui du manifeste de la sauvegarde ; démarrer **une** instance (les migrations rejouées amènent au schéma courant).
7. **Neutraliser les effets de bord** : marquer « non à renvoyer » les éléments en attente des files `ent_outbox`, courriels, SMS, push créés avant l'instant cible et jamais confirmés (§11.4), après décision.
8. **Objets** : bucket de production restauré depuis la destination (copie des versions ≤ instant cible) ; en cas de gros volume, l'application peut **lire en repli** depuis la destination pendant la recopie (v2).
9. **Vérifier** : `/pra/inventaire` global comparé à l'inventaire de la sauvegarde ; contrôle « chaque `cle_objet` en base existe avec la bonne empreinte » ; échantillons fonctionnels (connexion, fiche élève, appel).
10. **DNS** (fournisseur DNS **indépendant** de Scaleway, à vérifier pour `scolia.fr`) ; réouverture ; information des établissements ; journal de l'incident ; notification CNIL sous 72 h si violation (B4).

---

## 11. Restauration d'un seul établissement (point stratégique)

### 11.1 Principe
Une PITR est **globale** : on ne restaure jamais un établissement « en place ». On restaure **tout** dans un **bac isolé**, puis on **extrait** l'établissement et on le **réintègre** sous contrôle. Le même moteur d'extraction sert aux exports de portabilité (`PORTABILITE_EXPORTS.md`) : un seul code, testé deux fois plus.

### 11.2 Procédure
1. Demande tracée (direction de l'établissement + Florian), motif, instant cible, périmètre (tout l'établissement, un domaine, des objets précis).
2. **Bac isolé** : instance éphémère, réseau fermé, aucun envoi possible (SMTP/SMS/push désactivés en dur), accès nominatif ; PITR à l'instant cible depuis A ou B.
3. Démarrage de l'image **courante** sur le bac → migrations rejouées → même schéma que la production.
4. **Extraction** de l'établissement X (format technique, §11.3) depuis le bac, et extraction de X depuis la production (même format).
5. **Comparaison** table par table (inventaire par établissement existant + différences ligne à ligne) → rapport lisible : créées, modifiées, supprimées depuis T.
6. **Validation humaine** du rapport par l'établissement (deux personnes, mêmes règles que les exports à double approbation).
7. **Réintégration**, au choix :
   - **sélective** (recommandée) : réinsertion des lignes supprimées et remise des valeurs modifiées pour le périmètre validé, **par les commandes des domaines** quand elles existent (règles métier, outbox, journal) ; sinon insertion directe dans l'ordre des dépendances ;
   - **remplacement complet** de X : fenêtre de maintenance **pour X seulement** (connexions de X refusées), suppression des lignes de X puis insertion, dans **une transaction**, sous `TenantTx` strict ; réservé à une corruption massive de X.
8. **Contrôles** : contraintes, fermeture des références (aucune ligne de X ne pointe hors de X, sauf vers les tables globales autorisées), objets présents avec empreinte, inventaire final.
9. **Journal** : chaque ligne réintégrée produit une entrée `ent_journal_modif` (`domaine = 'restauration'`), l'historique de production n'est jamais effacé.

### 11.3 Règles de l'extracteur
- Lecture dans **une** transaction `REPEATABLE READ READ ONLY` (instantané cohérent ; `pg_export_snapshot` pour paralléliser), sous `TenantTx(X)` en mode RLS strict.
- **Liste des tables par le catalogue** (`information_schema` : colonnes `etablissement_id`) croisée avec le **registre de classement** (E12) : une table non classée **fait échouer** l'extraction et la CI.
- **Ordre** : tri topologique du graphe des clés étrangères (cycles résolus par contraintes `DEFERRABLE` ou insertion en deux temps).
- **Tables globales** : `ent_user` n'est jamais réécrite ; on exporte seulement l'identifiant et les champs minimaux des comptes qui ont un rattachement à X ; à la réintégration, un compte absent est **recréé sans secret** (pas de mot de passe ni de passkey : réinvitation) ; un compte présent est simplement relié. Référentiels globaux (`ref_calendrier_officiel`, `ent_absence_motif` si global) : vérifiés identiques, jamais réécrits.
- **Fermeture** : pour chaque FK simple, vérifier que la cible appartient à X ; toute exception est listée (c'est aussi un **audit de fuite** entre établissements, utile en soi).
- **Objets** : la liste vient de la base (`cle_objet` + `sha256` + version) à l'instantané ; préfixes `{X}/…` et `reinscription/{X}/…` (`key_belongs_to_etab`).

### 11.4 Tables à neutraliser (jamais réintégrées telles quelles)
Sessions, défis passkey, états OIDC, verrous de connexion, réinitialisations de mot de passe (déjà `TABLES_VOLATILES`) **+** `ent_outbox`, file des courriels (`055_mail_outbox`), SMS, push, jetons d'invitation non utilisés, liens de téléchargement. Une restauration ne doit **jamais** renvoyer un message à une famille.

### 11.5 Délais réalistes
Bac + PITR : 1 à 3 h selon la taille totale ; extraction et comparaison : minutes à 1 h ; validation : selon l'établissement ; réintégration sélective : 1 à 4 h. **RTO réaliste : une demi-journée à deux jours ouvrés**, dominé par la validation humaine.

---

## 12. Tests automatiques de restauration

| Fréquence | Test | Depuis | Résultat attendu |
|---|---|---|---|
| quotidien | `pgbackrest check`, vérification d'échantillon d'objets (empreintes) | A et B | état `integrite` |
| hebdomadaire | restauration PITR complète dans une instance jetable, démarrage de l'image courante, `/pra/inventaire` global comparé, contrôle objets référencés | **alternativement A puis B** | `restauration_verifiee` + durée mesurée |
| mensuel | restauration d'**un** établissement tiré au sort dans le bac (extraction + comparaison, sans réintégration) | A ou B | rapport de fermeture vide, inventaire concordant |
| trimestriel | exercice « Scaleway perdu » : tout depuis B, clés depuis le séquestre, infrastructure chez OVHcloud, chronométré | B | RTO mesuré, écarts au runbook |
| annuel | récupération des clés sans accès à Scaleway ; revue des rétentions avec le DPO (A12) | séquestre | |

Règles : instance jetable sans réseau sortant, **aucun envoi**, données détruites à la fin, accès nominatif, jamais de cible de production (garde-fou existant de `pra-lib.sh` étendu : refus de toute URL de production). Rapport stocké en base et dans les deux destinations : date, source, instant cible, durée par étape, résultat, anomalies.

---

## 13. Supervision et tableau de bord

Métriques à ajouter à `ent-obs` (mêmes règles : pas de donnée nominative, cardinalité bornée par `destination` ∈ {scaleway, ovh}) :

| Métrique | Alerte |
|---|---|
| `ent_sauvegarde_complete_age_secondes{destination}` | > 8 j |
| `ent_wal_archive_age_secondes{destination}` | > 15 min (RPO) |
| `ent_wal_spool_octets` / capacité | > 50 % avertissement, > 80 % critique |
| `ent_replication_objets_en_retard{destination}` | > 0 depuis 1 h |
| `ent_integrite_echecs_total{destination}` | toute hausse |
| `ent_restauration_test_age_secondes{type,destination}` et `…_resultat` | test hebdomadaire absent > 9 j ou échoué |
| `ent_protection_etat` (2 double, 1 dégradée, 0 aucune) | < 2 pendant 1 h ; 0 immédiatement |
| `ent_objets_suppressions_taux` / `ent_objets_reecritures_taux` | anomalie (rançongiciel, bug) |
| expiration des certificats et des rétentions Object Lock | 30 j avant |

Tableau de bord (page opérateur, pas d'établissement) : dernière complète par dépôt, dernier WAL par dépôt, état de chaque destination, retard, volumes, erreurs, dernier test par type, **état global en grand** : « Double protection confirmée » / « Protection dégradée : OVHcloud injoignable depuis 2 h » / « Aucune copie externe récente ».

---

## 14. Objectifs de reprise et scénarios de panne

Cible indicative de la commande : **RPO 15 min, RTO 4 h**. Production sur **un seul fournisseur**, deux destinations. Valeurs pour l'option B (§5.2) ; option A entre parenthèses.

| Scénario | RPO réaliste | RTO réaliste | Commentaire |
|---|---|---|---|
| Perte de la base (instance, disque) | ≤ 1–5 min (≤ 1 h) | 1–2 h à 100 Go ; 3–5 h à 600 Go | PITR depuis A ; RTO 4 h tenu jusqu'à ~300 Go sans réplica chaud |
| Perte du stockage documentaire de production | ≤ retard de réplication (minutes à 1 h) | 1–3 h à 1,5 To ; **6–12 h à 9 To** | au-delà de ~2 To, RTO 4 h **non tenu** sans lecture en repli depuis la destination (v2) |
| **Panne Scaleway totale** (fr-par ou compte) | ≤ 5 min base (≤ 1 h), ≤ 1 h objets | **8–24 h** tant que non répété ; **≈ 4 h** après IaC + répétitions trimestrielles | ENT **inaccessible** pendant ce temps ; la double sauvegarde ne l'évite pas |
| Panne OVHcloud | 0 | 0 | protection dégradée, rattrapage automatique |
| Destinations toutes deux injoignables, prod OK | 0 tant que la prod vit | — | spool local, alerte critique ; si la prod tombe pendant ce temps : perte = depuis la dernière copie externe |
| Scaleway et OVHcloud tombés ensemble | dernière copie hors ligne (aucune au lancement) | attente du retour d'un fournisseur | justifie la troisième copie plus tard |
| Corruption logique (bug, migration, saisie de masse) | 0 sur les données d'avant T (si détectée < 35 j ; < 12 mois au mois près) | global : 2–4 h mais perte des écritures après T ; sélectif : ½ à 2 j | préférer la restauration sélective |
| Compromission des accès de production | ≤ 5 min | comme « Scaleway perdu » si on reconstruit hors compte compromis | Object Lock protège les versions ; rotation de tous les secrets |
| Restauration d'un seul établissement | selon l'instant demandé | ½ à 2 jours ouvrés | validation humaine dominante |

Ce qui n'est **pas** possible avec une seule production : rester en ligne pendant une panne complète de Scaleway. Pour un RTO < 1 h sur ce scénario, il faudrait une infrastructure de reprise chaude chez OVHcloud (réplica PostgreSQL en flux + application en veille) : ≈ 150–400 €/mois selon l'échelle, **non justifié au lancement** (§15, D-045 proposée).

---

## 15. Coûts et dimensionnement

### 15.1 Hypothèses de volume [H, à mesurer en T1 puis pendant le doublon]
Par établissement (lycée ou collège de 600–1 000 élèves, tous modules) : base **2 Go** ; WAL **≈ 100 Mo/jour** compressés en période scolaire ; documents **30 Go** (photos, pièces, bulletins PDF, signatures), croissance ≈ 10 Go/an. Dépôt pgBackRest par destination ≈ **7 Go** par établissement (5 complètes compressées + différentielles + 35 j de WAL). Copie documentaire ≈ **33 Go** (avec versions retenues 35 j). Total ≈ **40 Go par établissement et par destination**.

### 15.2 Prix unitaires retenus (2026-11-10, HT)
| Poste | Scaleway | OVHcloud |
|---|---|---|
| Stockage standard | Multi-AZ 0,01606 €/Go/mois ; One Zone 0,00803 | 3-AZ 0,0140 €/Gio/mois ; 1-AZ 0,0071 |
| Froid | Glacier 0,00246, restauration 0,009 €/Go | Cold Archive 0,00166, min. 180 j, extraction 0,009 €/Gio |
| Requêtes | incluses | gratuites |
| Sortie (egress) | 75 Go/mois gratuits puis 0,01 €/Go | gratuite (sauf extraction des classes froides) |
| Petite instance | DEV1-S ≈ 6,55 €/mois ; PLAY2-PICO ≈ 10,42 ; PRO2-XXS ≈ 41 | d2-2 5,71 ; d2-4 11,44 ; d2-8 20,60 |
| Base managée | DB-DEV-S ≈ 11,4 €/mois (non adaptée à la prod) ; sauvegardes 0,03 €/Go/mois | Essential ≈ 54 €/mois ; Business ≈ 139 €/mois |
| Clés | Key Manager 0,06 €/version/mois ; Secret Manager 0,04 €/version/mois | KMS logiciel 0,06 €/clé/mois |

### 15.3 Estimation (coût **additionnel** des sauvegardes, hors production)

| Poste (€/mois) | Petite : 1–10 étab. (calcul à 5 : base 10 Go, docs 150 Go) | Moyenne : ~50 (base 100 Go, docs 1,5 To) | Grande : ~300 (base 600 Go, docs 9 To) |
|---|---|---|---|
| Destination A Scaleway (One Zone, ~200 Go / 2 To / 12 To) | 1,6 | 16 | 96 |
| Destination B OVHcloud (3-AZ) | 2,8 | 28 | 168 (ou 85 en 1-AZ) |
| Sortie Scaleway → OVHcloud (WAL + nouveaux objets) | 0 (sous 75 Go) | ≈ 2–3 | ≈ 10–20 |
| Worker de réplication (dans l'image, ou petite instance) | 0–7 | 7–11 | 20–40 |
| Tests de restauration (instances à l'heure) | 3–5 | 15–25 | 40–70 |
| Clés et secrets | < 1 | < 1 | 1–2 |
| **Sous-total première version** | **≈ 10–20** | **≈ 70–85** | **≈ 340–400** |
| + Troisième copie froide (OVHcloud Cold Archive, 12 mensuelles) | ≈ 0,3 | ≈ 3,5 | ≈ 20 |
| + Infrastructure de reprise chaude (réplica + appli en veille chez OVHcloud) | non justifié | ≈ 100–150 | ≈ 250–400 |
| Amorçage (copie initiale vers OVHcloud, une fois) | 0–1 | ≈ 15 | ≈ 90 |

Coût PostgreSQL **de production**, rappelé à part (pas un coût de sauvegarde) : option A managée ≈ 50–150 €/mois selon offre et HA ; option B autogérée : instance (≈ 40–80 €/mois au départ) + bloc + **temps d'exploitation**, qui est le vrai coût de l'option B.

Lecture : la sauvegarde double coûte **1,3 à 3 € par établissement et par mois** ; le stockage domine à grande échelle, les tests et le worker à petite échelle. Leviers : classe 1-AZ pour B (le multi-fournisseur apporte déjà la redondance géographique), transition des documents de plus d'un an vers une classe moins chère dans les **destinations** (pas en production), fréquence des tests.

---

## 16. Architecture Rust

### 16.1 Crate `ent-sauvegarde` (nom indicatif) [R]
Peu de dépendances : client S3 maison minimal (SigV4 existe déjà dans `ent-object/src/presign.rs` pour le GET ; à étendre à PUT/HEAD/LIST/multipart) **ou** `aws-sdk-s3` / `object_store` (Apache-2.0) après revue (`cargo deny`). Pas de logique de fournisseur dans le cœur.

```rust
/// Destination de sauvegarde : aucune hypothèse au-delà du S3 commun.
pub trait DestinationSauvegarde: Send + Sync {
    fn nom(&self) -> NomDestination;                         // "scaleway" | "ovh"
    async fn deposer(&self, cle: &CleObjet, flux: Flux, empreinte: Sha256) -> Result<Confirmation, ErreurDestination>;
    async fn verifier(&self, cle: &CleObjet, attendu: &Sha256) -> Result<Verification, ErreurDestination>;
    async fn lister(&self, prefixe: &str, reprise: Option<Curseur>) -> Result<Page<EntreeListe>, ErreurDestination>;
    async fn lire(&self, cle: &CleObjet, version: Option<&VersionId>) -> Result<Flux, ErreurDestination>;
    fn capacites(&self) -> Capacites;                         // checksum SHA-256 natif ? If-None-Match ? Object Lock ?
}
pub struct S3Generique { /* endpoint, région, style de chemin, identifiants */ }
// Scaleway et OVHcloud = S3Generique + `Capacites` différentes + réglages ; pas de sous-classe tant que rien ne l'exige.
```
- `ErreurDestination` distingue **transitoire** (réseau, 5xx, 429 → nouvelle tentative) et **définitive** (403, 404 bucket, empreinte divergente → alerte).
- Idempotence par clé stable ; tests avec un **faux S3 maison** en mémoire (comme `ObjectStore::memory_only`) ; MinIO et Garage sont sous AGPL-3.0, donc écartés même pour la CI [R].
- Lecture de l'état pgBackRest par `pgbackrest info --output=json` (processus séparé, pas de lien de code).

### 16.2 Tables (normalisées, D-036)
`ent_sauvegarde` (id stable, type : `pg_complete|pg_diff|objet|manifeste`, produite_le, taille, sha256, `migration_tag`, `git_sha`) ; `ent_sauvegarde_copie` (sauvegarde_id, destination, etat, confirmee_le, verifiee_le, tentatives, prochaine_tentative, derniere_erreur) unique `(sauvegarde_id, destination)` ; `ent_restauration_test` (type, destination, instant_cible, debut, fin, resultat, anomalies en lignes filles). Tables **de plateforme** sans `etablissement_id` : à classer « globale opérateur » dans le registre (E12).

### 16.3 Crate `ent-extraction` (partagée PRA / portabilité / reprise)
Catalogue → registre de classement → graphe FK → instantané → flux par table (JSON Lines typé) + liste d'objets → manifeste. Utilisé par : restauration sélective (§11), exports (`PORTABILITE_EXPORTS.md`), tests mensuels (§12), et en miroir par `ent-reprise` (D-035, `TRANSITION_T0_INVENTAIRE.md`) pour le format de manifeste.

### 16.4 Correction de `ent-object` (prérequis, lot dédié)
Écriture S3 réelle **signée**, **erreur remontée** (plus de `let _ =`), disque local réservé au labo, empreinte SHA-256 calculée à l'écriture et rendue à l'appelant pour être stockée en base.

---

## 17. Alternatives rejetées

| Alternative | Pourquoi rejetée |
|---|---|
| Un fournisseur primaire + un secours | contraire à la commande ; et une seule destination « vraie » = pas d'indépendance |
| PostgreSQL multi-maître / deux productions | complexité et risque de divergence sans rapport avec le besoin (sauvegarde, pas haute disponibilité inter-fournisseurs) |
| Réplication native des buckets | absente chez Scaleway ; limitée au même projet chez OVHcloud (pas d'indépendance de compte) ; ne réplique ni SSE-C ni les suppressions de la même façon |
| `rclone sync` en tâche planifiée | propage les suppressions et corruptions ; pas d'états par élément ; acceptable seulement en **copie** (`rclone copy`) de dépannage (MIT) |
| Copie de fichiers de la base en marche, snapshots de disque comme sauvegarde principale | non cohérent / lié au fournisseur |
| Barman comme outil principal | GPL-3.0 (acceptable en outil séparé) mais pas de chiffrement client documenté, multi-destinations non documenté |
| Clés uniquement dans le KMS d'un fournisseur | export non documenté (Scaleway) : risque d'irrécupérabilité |
| MinIO / Garage embarqués | AGPL-3.0 (CLAUDE.md) |
| Infrastructure de reprise chaude dès le lancement | coût fixe non justifié pour 1 établissement en doublon avec Charlemagne |
| Restauration d'établissement par `pg_restore` partiel | impossible proprement : identité partagée, FK, effets de bord (§2.4) |

---

## 18. Feuille de route, prérequis, décisions, exigences

### 18.1 Recommandation en quatre niveaux
1. **Minimum à concevoir dès maintenant** (coût nul, contraint le modèle) : exigences E1–E14 (§18.4) imposées aux lots en cours ; registre de classement des tables + test CI ; décision sur l'hébergement PostgreSQL (D-039) et question Q-PRA1 posée à Scaleway ; correction des deux affirmations du doc existant (fait dans ce lot, documentation seulement).
2. **Première version de sauvegarde en production** (T1 Noël 2026 → printemps 2027, avant les vraies données) : `ent-object` réellement S3 ; destinations A (organisation/projet séparé) et B (compte OVHcloud) avec versionnage + Object Lock GOVERNANCE 35 j ; option A (managée + `pg_dump` horaires chiffrés vers A et B) au départ ; worker de réplication des objets avec états et réconciliation ; métriques et alertes ; test hebdomadaire de restauration alterné A/B ; séquestre des clés ; runbook global. **Puis, avant la rentrée 2027** (premiers usages réels, D-035) : bascule vers l'option B (PostgreSQL autogéré + pgBackRest double dépôt, RPO ≤ 5 min) si Q-PRA1 est négative.
3. **Protections avancées ensuite** (≈ 50 établissements ou commercialisation) : `ent-extraction` + restauration d'un établissement outillée ; Object Lock COMPLIANCE ; troisième copie froide ; chiffrement client des documents ; IaC complète et exercice trimestriel « Scaleway perdu » ; chaîne de hachage du journal ; lecture en repli des objets depuis une destination.
4. **Non justifié au lancement** : infrastructure de reprise chaude chez OVHcloud ; base multi-région ; HSM dédié ; tests de restauration quotidiens complets ; rétentions au-delà de 12 mois (sauf obligation, avis DPO).

### 18.2 Prérequis avant toute implémentation
- P1 : réponse écrite de Scaleway (Q-PRA1 : PITR / WAL / réplication physique sur la base managée).
- P2 : vérification sur console (par Florian) : Object Lock à la création, compatibilité pgBackRest avec Object Lock (l'`expire` échoue sur un objet encore verrouillé : rétention du verrou < rétention pgBackRest, à tester), en-têtes de somme de contrôle SHA-256, `If-None-Match`.
- P3 : avis DPO sur les rétentions des sauvegardes (35 j + 12 mois) et sur l'effacement différé dans les sauvegardes (droit à l'effacement : effectif à l'expiration de la rétention, documenté dans la notice).
- P4 : fournisseur DNS de `scolia.fr` indépendant de Scaleway ? (Florian).
- P5 : création des comptes et organisations par **Florian** (le pilote ne crée rien) ; identifiants saisis dans les secrets, jamais dans le chat.
- P6 : lot `ent-object` S3 réel ; sortie des `BYTEA` (E5).
- P7 : mesure des volumes réels pendant T1 (`pg_stat_wal`, taille des objets) pour recalculer §15.

### 18.3 Propositions de décisions
Inscrites dans `docs/DECISIONS.md` (D-037 à D-046, numéros provisoires si une autre session en ajoute entre-temps), **toutes « proposé, à valider par Florian »**. À prendre **maintenant** (elles contraignent le modèle de données ou le lot `ent-object`) : D-037, D-038, D-039, D-040, D-041. Peuvent **attendre** : D-042 (avant T1), D-043 à D-046.

### 18.4 Exigences à imposer dès maintenant aux lots en cours (à transmettre par le pilote)
| # | Exigence | Vérification |
|---|---|---|
| **E1** | Toute table qui contient une donnée d'établissement porte `etablissement_id UUID NOT NULL`, **y compris les tables filles** (même si déductible du parent). Les tables sans `etablissement_id` sont listées et justifiées dans le registre (E12). | revue + test du registre |
| **E2** | Toute référence vers une ligne d'établissement est une **FK composite** `(etablissement_id, x_id)` → `(etablissement_id, id)` (lot C5) pour les nouvelles tables. | `grep REFERENCES` en revue |
| **E3** | Clé primaire **UUID générée par l'application** ; pas de `SERIAL`/`IDENTITY` pour une donnée métier ; numéros lisibles (n° d'élève, de facture) par **compteur par établissement**. | revue |
| **E4** | Pas de nouvelle table globale qui contient une donnée d'établissement ; une table d'identité partagée ne contient aucune donnée métier. | revue |
| **E5** | **Fichiers hors base** : aucun nouveau `BYTEA` de contenu ; chaque fichier = objet immuable `{etablissement}/{espace}/{uuid}.{ext}` + ligne en base avec `cle_objet`, `taille`, `sha256`, `mime`, `cree_le`, `supprime_le`. Un document modifié = nouvel objet. Reprise de `ent_msg_piece` et `ent_pai_piece` à planifier. | `grep -i bytea` en revue |
| **E6** | `cree_le TIMESTAMPTZ NOT NULL DEFAULT now()` partout ; `modifie_le` sur toute table modifiable ; suppression d'un objet métier à historique = changement d'état daté plutôt que `DELETE`. | revue |
| **E7** | Toute action sensible journalisée dans `ent_journal_modif` (instantané, D-036) ; aucune purge hors procédure ; à terme blocage de `TRUNCATE` et rôle non propriétaire. | revue |
| **E8** | Migrations **additives et rejouables** sur une base issue de **n'importe quel état antérieur** (c'est ce que fait une PITR suivie d'un démarrage) ; aucune migration destructrice sans décision, point de sauvegarde vérifié et note dans `MIGRATIONS.md`. | revue + test de rejeu |
| **E9** | Toute nouvelle file d'effets de bord (outbox, courriel, SMS, push, webhooks, jetons) est déclarée **« à neutraliser à la restauration »** dans le registre. | test du registre |
| **E10** | D-036 : pas de JSON qui sert de mémoire (aide directe à la portabilité). | `grep -i jsonb` |
| **E11** | Toute nouvelle clé de chiffrement est inscrite à l'inventaire des clés (§9.2) avec son séquestre ; format chiffré avec **identifiant de clé**. | revue sécurité |
| **E12** | **Registre de classement des tables** (dans le code, ex. `ent-db/src/registre_donnees.rs`) : pour chaque table, domaine, portée (`etablissement` / `globale_reference` / `identite_partagee` / `plateforme` / `volatile`), catégorie d'export (`simple` / `double` / `particuliere`, `PORTABILITE_EXPORTS.md` §4), neutralisation à la restauration. **Un test CI échoue** si une table du schéma n'est pas classée (comme le test de `MIGRATION_TAG`). | test CI |
| **E13** | Nouvelles tables sous RLS (politique standard, test strict). | test RLS |
| **E14** | Aucune donnée d'établissement dans les journaux applicatifs ni dans les noms d'objets (déjà la règle : UUID seulement). | revue |

---

## 19. Risques résiduels
- Compromission simultanée des comptes Scaleway (prod + sauvegarde) **et** OVHcloud **et** du séquestre : perte possible ; atténué par séparation des comptes, double authentification, COMPLIANCE plus tard, troisième copie.
- Corruption logique silencieuse découverte après la rétention (35 j / 12 mois) : non restaurable à l'état exact.
- Perte de `ENT_SECRETS_KEY` et de son séquestre : champs sensibles perdus dans **toutes** les sauvegardes.
- Opérateur de plateforme malveillant (accès propriétaire à la base) : peut toujours extraire ; atténué par journal d'infrastructure (Audit Trail Scaleway), procédure et contrat, pas supprimable techniquement.
- RTO 4 h non tenu pour les documents à grande échelle et pour la panne totale de Scaleway tant que la reprise n'est pas répétée.
- Dépendance à pgBackRest (un mainteneur principal ; sponsor historique devenu « passé ») : alternative WAL-G documentée.
- Option A pendant le doublon : RPO ≈ 1 h ; accepté car Charlemagne reste officiel (D-034) — **pas** accepté pour les modules en usage réel (D-035).
- Prix et capacités relevés le 2026-11-10, en partie de seconde main : à reconfirmer avant engagement.

## 20. Sources (consultées le 2026-11-10)
Scaleway : https://www.scaleway.com/en/pricing/storage/ · https://www.scaleway.com/en/docs/object-storage/how-to/use-object-lock/ · https://www.scaleway.com/en/docs/object-storage/concepts/ · https://www.scaleway.com/en/docs/object-storage/api-cli/lifecycle-rules-api/ · https://www.scaleway.com/en/docs/object-storage/how-to/use-bucket-versioning/ · https://www.scaleway.com/en/docs/object-storage/reference-content/s3-iam-permissions-equivalence/ · https://www.scaleway.com/en/docs/object-storage/api-cli/bucket-policy/ · https://www.scaleway.com/en/pricing/managed-databases/ · https://www.scaleway.com/en/docs/managed-databases-for-postgresql-and-mysql/how-to/manage-backups/ · https://www.scaleway.com/en/docs/managed-databases-for-postgresql-and-mysql/how-to/manage-snapshots/ · https://www.scaleway.com/en/docs/tutorials/backup-postgresql-pgbackrest-s3/ · https://www.scaleway.com/en/pricing/security-and-account/ · https://www.scaleway.com/en/docs/key-manager/faq/ · https://www.scaleway.com/en/pricing/virtual-instances/ (pages Scaleway lues en extrait via moteur de recherche : **à reconfirmer**).
OVHcloud : https://www.ovhcloud.com/fr/public-cloud/prices/ · https://www.ovhcloud.com/fr/public-cloud/object-storage/classes/ · https://www.ovhcloud.com/fr/public-cloud/cold-archive/ · https://docs.ovhcloud.com/en/guides/storage-and-backup/object-storage/s3-faq · https://docs.ovhcloud.com/en/guides/storage-and-backup/object-storage/s3-asynchronous-replication · https://docs.ovhcloud.com/en/guides/storage-and-backup/object-storage/s3-identity-and-access-management · https://help.ovhcloud.com/csm/en-public-cloud-storage-s3-managing-object-lock?id=kb_article_view&sysparm_article=KB0047399 · https://docs.ovhcloud.com/en/guides/public-cloud/databases/backups · https://www.ovhcloud.com/fr/identity-security-operations/pricing/ · https://help.ovhcloud.com/csm/en-kms-kmip?id=kb_article_view&sysparm_article=KB0065015.
Outils PostgreSQL : https://github.com/pgbackrest/pgbackrest (MIT, 2.60.0 du 18-10-2026) · https://pgbackrest.org/configuration.html · https://github.com/wal-g/wal-g (Apache-2.0, lzo optionnel GPL) · https://github.com/EnterpriseDB/barman (GPL-3.0) · https://www.postgresql.org/docs/17/app-pgbasebackup.html (PostgreSQL License). **Aucun n'est sous AGPL** ; tous seraient exécutés comme programmes séparés, non embarqués dans nos crates.
