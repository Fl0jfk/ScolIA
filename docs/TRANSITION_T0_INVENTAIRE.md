# Transition T0 — Inventaire du schéma Next.js et plan de reprise

**Date :** 2026-11-10 — **Statut :** **proposé par le pilote, à valider par Florian**. Conception seulement : aucun code écrit, rien exécuté ni construit côté Next.js, **aucun accès à la base de production ni à ses secrets**. Le code Next.js a été lu en lecture seule et ciblée, uniquement dans le dépôt (`db/*.ts`, `drizzle/*.sql`, `scripts/*.sql`, `app/lib/*`, `app/api/*`).
**À lire avec :** `docs/TRANSITION_NEXT_RUST.md` (§2 bis : la reprise est une **fusion dans le cœur**, pas une copie de tables), `docs/DECISIONS.md` D-009, D-034, D-035 (dont **précision 2** : maturité des modules), **D-036** (plus aucun JSON qui sert de mémoire), `docs/MATRICE_CHARLEMAGNE.md` §4, `docs/socles/42_SORTIES_VOYAGES.md` §26 (premier plan de reprise détaillé, repris ici), `docs/socles/02_ELEVE.md` §7.10 (fusion de doublons, E6).

**Conventions.** **[F]** fait vérifié dans le dépôt (chemin cité) ; **[H]** hypothèse (déduction, à confirmer par Florian ou sur un export) ; **[R]** recommandation (non décidée). Classement proposé : **À conserver** (reprise sans perte) · **Archive** (à conserver, mais en lecture seule, sans fusion dans les tables vivantes) · **Perdable** (on repart de zéro sans dommage) · **Vide** (rien à reprendre, ou presque) · **Rapprochement** (référentiel en double : le cœur fait foi, on ne reprend que la correspondance des identifiants). Le pilote **ne voit pas les données** : le classement repose sur des indices de code ; Florian le corrige d'après l'usage réel (colonne « Avis de Florian »).

Sommaire : §1 Méthode et chiffres · §2 Où dorment les données (et D-036) · §3.0 Maturité et stockage par module · §3 Inventaire par module et classement · §4 Correspondance Next.js → ScolIA · §5 Tables abandonnées · §6 Rapprochement des doublons, comptes et mots de passe · §7 Risques et rapport de reprise · §8 Outil `ent-reprise` · §9 Outil d'anonymisation · §10 Découpage en lots · §11 À valider par Florian · §12 Faits, hypothèses, contradictions.

---

## 1. Méthode et chiffres

**Sources lues [F]**
- `db/schema*.ts` et `db/etablissement-table.ts` (5 600 lignes) : c'est le **schéma final** déclaré par Drizzle (`drizzle.config.ts` : `schema: "./db/schema.ts"`, `out: "./drizzle"`). Il a été préféré au rejeu des migrations, puis croisé avec elles.
- `drizzle/*.sql` : **78 fichiers** (numéros 0000 à 0067, plusieurs numéros doublés : 0050 à 0058), **176 `CREATE TABLE`**, **2 `DROP TABLE`** (`0007_drop_json_mirror.sql` : `tenant_document`, `ent_entity`), **1 renommage** (`clerk_user_mapping` → `auth_user_mapping`, `0002`).
- **10 tables créées hors migrations** par des scripts : `messaging_*` (6 tables, `scripts/messaging-core-ddl.sql`, `scripts/ensure-messaging-*.mjs`) et `portes_ouvertes_*` (4 tables, `scripts/sql/portes-ouvertes.sql`). Elles existent dans le schéma TypeScript mais **pas** dans `drizzle/` : il faut vérifier sur l'export qu'elles existent bien en production (Q-T0-01).
- Usage : pour chaque table, recherche de son nom d'export Drizzle dans `app/**` (hors tests), regroupée par route `app/api/<module>` et par bibliothèque `app/lib/*`. Une table **jamais référencée** dans le code est très probablement vide ou morte.

**Chiffres [F]**
| Élément | Nombre |
|---|---|
| Tables déclarées dans `db/*.ts` | 186 |
| dont supprimées par la migration 0007 (encore déclarées en TS) | 2 |
| dont créées par scripts hors `drizzle/` | 10 |
| **Tables vivantes attendues en production** | **184** [H : si les scripts ont été passés] |
| Tables jamais lues ni écrites par le code | 17 (liste au §5.3) |
| Collections JSON stockées dans `ent_collection_record/attr` (chemins `*.json` distincts relevés) | environ 90 chemins, une soixantaine de collections |

## 2. Trois endroits où dorment les données (point capital)

Une reprise fondée sur les seules tables typées **perdrait des modules entiers**. Next.js range ses données à trois endroits [F] :

1. **Tables typées** (voyages, demandes, absences RH, fiches de dialogue, santé, facturation…).
2. **Collections clé-valeur** `ent_collection_record` + `ent_collection_attr` (et, de même forme, `travel_attr`, `request_attr`, `personnel_attr`, `stage_*_attr`, `certificate_*_attr`, `tenant_setting_attr`, `request_routing_attr`, `request_org_attr`, `platform_document_attr`). Les anciens fichiers JSON du stockage objet (`getJson("internat/rooms.json")`…) sont redirigés vers Postgres : le chemin devient une collection (`internat__rooms`, séparateur `__`) et le document est **aplati** en lignes `(path, value)` (`app/lib/s3-storage.ts:29-44`, `app/lib/ent-json-postgres.ts:13-46`, codec `app/lib/ent-attr-codec.ts`). Modules **entièrement** dans ces collections : internat (en grande partie), certificats, planning par domaines, photocopies couleur, demandes d'HSE, covoiturage, bien-être, onboarding RH et « pouls », RGPD, affiches, site public, répartition des classes, fiches de rentrée, partages de documents, paramétrages (`settings/*`).
3. **Stockage objet S3** (un bucket par établissement, `etablissement.data_bucket`, `app/lib/s3-storage.ts:19-27`) : **fichiers** référencés par des clés (`*_s3_key`, `*_key`, `file_url`, `document_keys`) — photos d'élèves, pièces des élèves, justificatifs d'absence, pièces jointes des demandes et des messages, PDF de fiches de dialogue, photos du blog de voyage, PAP des rendez-vous d'inscription, logos et signatures, factures PDF, et **l'espace documents personnel des personnels** (`documents/users/<id>/`, `app/lib/documents-cloud.ts:64-76`). Certains fichiers sont aussi **dans la base en base64** (`fd_pdf_archive.pdf_base64`, `fd_signature.signature_png_base64`, `famille_thread_attachment.content_base64`, `sst_*.signature_png_base64`).

4. **JSON encore stockés directement dans S3** (lus par `GetObjectCommand` + `JSON.parse`, hors de la redirection vers Postgres) [F] :
   - **dépôts de fiches de rentrée en attente** : `rentree/submissions/pending/<jeton>/` (métadonnées JSON + fichier, `app/lib/rentree-submissions.ts:43, 104-125`) ;
   - **demandes en attente de vérification de l'e-mail** : `requests/pending/<jeton>/` (`app/lib/request-pending-verify.ts:39, 110-125`) ; pièces des demandes sous `requests/<id>/files/` ;
   - **devis reçus par e-mail pour les voyages** : marqueurs d'ingestion JSON (`app/lib/travel-email-ingest.ts:233-300`) ;
   - **base de connaissances de l'assistant** : `knowledge/*.json` (`app/lib/knowledge.ts:40-60`, alimentée par `api/chatbot/ingest`) ;
   - **registre des établissements de la plateforme** : `tenants/index.json` et, à côté, un **dossier de secrets par établissement** (`app/lib/tenant-registry.ts:14, 47-50`) — **à exclure de tout export** (secrets) ;
   - documents de plateforme, en repli S3 si la base n'est pas configurée (`app/lib/platform-storage.ts:23-37`).
   S'y ajoutent probablement des **JSON historiques** restés dans le bucket depuis l'époque « tout en S3 » (`scripts/restore-travels-from-s3.ts`, `migrate-*-from-collection`) : ce ne sont plus des sources de vérité, mais une donnée jamais migrée peut s'y trouver [H] ; l'outil les **inventorie** et compare avec la base, sans les reprendre par défaut.
5. **Colonnes JSONB porteuses de données métier** [F] (`db/*.ts`) : `eleve.options`, `eleve_scolarite.grille_repas`, `personnel.payload`, `fd_campagne.catalogue / appel_config / classes_cibles / eleve_ids_cibles`, `fd_fiche.options_actuelles / parent_emails / acceptation / appel / parent_accord / conflict_payload`, `fd_reponse.payload`, `partenariat_offre.cycles / niveaux / tarifs / cta_links`, `famille_messaging_settings.roles_can_initiate`, `sst_fiche.workflow`, `portes_ouvertes_slot_staff.meta`, `preinscription.payload`, `ref_nomenclature.metadata_json`. Plus les tables clé-valeur `*_attr` et `ent_collection_attr`, qui sont du JSON aplati.

**Règle D-036 appliquée à la reprise [R]** : rien de ce qui est JSON côté Next.js (collections, `*_attr`, colonnes JSONB, JSON sur S3) n'arrive en JSON dans ScolIA. Chaque document est **décodé puis ventilé dans des tables normalisées** du socle compétent (colonnes typées, clés étrangères, contraintes) ; une clé inconnue du décodeur n'est pas « gardée en vrac » : elle est **refusée et listée** au rapport. Seule exception : l'**instantané figé** de la source (copie brute de l'objet Next.js d'origine, jamais relue comme vérité) peut être conservé dans la zone de reprise le temps de la vérification, puis supprimé (exception « instantané d'audit » de D-036, à lister dans `docs/socles/MIGRATIONS.md`).

**Conséquence [R]** : un `pg_dump` **ne suffit pas**. Florian fournit **deux exports** : `pg_dump` de la base **et** copie du bucket S3 (ou au moins de la liste des objets avec taille et empreinte, puis les objets des modules basculés), **en excluant le préfixe des secrets des établissements**. L'outil reconstruit les documents aplatis avant toute conversion (comme prévu au `42_SORTIES_VOYAGES.md` §26.3).

---

## 3.0 Maturité de chaque module Next.js et lieu de ses données

D-035 précision 2 : seuls les modules **en usage et fiables** servent de référence (« reprendre les actions métier ») ; les **ébauches** sont refaites de zéro par la méthode des socles et leurs données sont probablement perdables. Maturité proposée : **Fiable** (en usage, fiable) · **Partiel** · **Ébauche** (à refaire). Indices [F] mesurés sur le code (fichiers `.ts/.tsx` des routes `app/api/<module>`, pages, composants et `app/lib/<module>*`) : nombre de fichiers, lignes, routes, pages, fichiers de test, occurrences de `TODO/FIXME/stub/placeholder` (compte bruité : `placeholder` sert aussi dans les champs de saisie). Le volume de code ne prouve pas l'usage : quand Florian a tranché (voyages, salles, comptabilité et paie, RH, pilotage), sa parole prime. Stockage : **T** tables typées · **KV** JSON en base (collections, `*_attr`) · **JB** colonnes JSONB métier · **S3-J** JSON sur S3 · **S3-F** fichiers sur S3. « pg_dump seul » = non dès qu'il y a S3-J ou S3-F.

| Module | Fichiers / lignes / routes / pages / tests / TODO | Maturité proposée | Où vivent les données | pg_dump seul suffit ? | Avis de Florian |
|---|---|---|---|---|---|
| Voyages et séjours (M01) | 169 / 32 500 / 46 / 1 / 23 / 38 | **Fiable** (Florian) | T + KV (`travel_attr`) + S3-F (photos, devis) + S3-J (devis reçus par e-mail) | non | |
| Réservation de salles (M02) | ≈ 20 / 4 000 / 8 / 1 / 0 / 0 (`reservation-rooms` + `prof-room`) | **Fiable** (Florian) ; aucun test | T (ancien KV migré) | oui | |
| Planning par domaines (M03) | 25 / 4 100 / 13 / 1 / 1 / 14 | Partiel | KV seulement (tables typées mortes) | oui | |
| Demandes (M04) | 39 / 7 700 / 9 / 0 / 1 / 10 | Partiel | T + KV + S3-J (en attente de vérification) + S3-F | non | |
| Absences du personnel (M05) | 57 / 12 200 / 11 / 0 / 11 / 3 | Partiel à fiable [H] : le plus testé après les voyages ; à confirmer, car rattaché aux « RH » | T + S3-F (justificatifs) | non | |
| RH : dossier, congés, onboarding, pouls (M06) | 98 / 14 000 / 35 / 4 / 1 / 2 | **Ébauche** (Florian) malgré le volume ; 1 seul test | T (`personnel` + `payload` JB) + KV (`personnel_attr`, `rh__*`) + S3-F | non | |
| Registre SST (M07) | 8 / 500 / 8 / 0 / 0 / 0 | Partiel [H] (récent : 0064, 0067) | T + JB (`workflow`) + base64 | oui | |
| Paie (dans M05/M06) | 5 / 660 / 1 / 1 / 0 / 4 | **Ébauche** (Florian) | T (`paie_*`) | oui | |
| Comptabilité de l'établissement (M15, partie trésorerie et dépenses) | 5 / 1 240 / 2 / 1 / 0 / 5 | **Ébauche** (Florian) | T | oui | |
| Facturation des familles (M15) | 12 / 2 570 / 5 / 1 / 0 / 5 | Ébauche [H] (rattachée à la comptabilité ; aucun test) | T + S3-F (PDF) | non | |
| Demandes d'HSE (M08) | 4 / 1 250 / 2 / 1 / 0 / 5 | Partiel [H] | KV | oui | |
| Photocopies couleur (M08) | 13 / 2 930 / 6 / 2 / 0 / 4 | Partiel [H] | KV | oui | |
| Certificats (M09) | 29 / 2 730 / 16 / 1 / 0 / 0 | Partiel [H] | KV + S3-F | non | |
| Fiches de dialogue (M10) | 20 / 7 000 / 11 / 0 / 1 / 8 | Partiel à fiable [H] (3 versions, audit personas v2) | T + JB (nombreuses colonnes) + base64 + S3-F | non | |
| Internat (M11) | 80 / 15 700 / 28 / 0 / 5 / 63 | Partiel [H] (deux générations, beaucoup de TODO) | T + KV | oui | |
| Stages (M12) | 128 / 34 400 / 36 / 2 / 8 / 38 | Partiel à fiable [H] (gros volume, testé) | T + KV + S3-F (conventions) | non | |
| Santé et infirmerie (M13) | 37 / 4 800 / 7 / 9 / 0 / 25 | Partiel [H] ; reprise bloquée (HDS) | T + S3-F | non | |
| Cantine, passages (M14) | 12 / 1 900 / 1 / 0 / 2 / 1 | Partiel [H] | T + JB (`grille_repas`) + KV (réglages) | oui | |
| RDV d'inscription (M16) | 38 / 10 900 / 14 / 4 / 4 / 21 | Partiel à fiable [H] (10 migrations successives) | T + S3-F (PAP) | non | |
| Fiches de rentrée (M16) | 16 / 2 200 / 4 / 2 / 0 / 2 | Partiel [H] | **S3-J** (dépôts en attente) + KV + S3-F | non | |
| Portes ouvertes (M17) | 13 / 4 000 / 2 / 0 / 0 / 5 | Partiel [H] | T + JB (`meta`) | oui | |
| Accueil (saisie d'absences) | 19 / 4 550 / 8 / 0 / 3 / 4 | Partiel [H] | T (`vs_absence_eleve`, `absence`) | oui | |
| Invitations (M17) | 22 / 5 450 / 11 / 1 / 1 / 7 | Partiel [H] | T | oui | |
| Partenariats (M17) | 20 / 3 700 / 10 / 2 / 0 / 10 | Partiel [H] | T + JB (cycles, niveaux, tarifs) + S3-F | non | |
| Bien-être (M18) | 17 / 1 300 / 4 / 3 / 0 / 2 | Ébauche à partiel [H] | KV | oui | |
| Espace documents (M19) | 39 / 5 100 / 15 / 5 / 0 / 5 | Partiel [H] | KV (partages) + **S3-F** (fichiers des utilisateurs) | non | |
| Covoiturage (M20) | 7 / 1 170 / 2 / 1 / 0 / 2 | Ébauche [H] | KV | oui | |
| **Pilotage, tableau de bord, supervision** | 67 / 10 000 / 14 / 1 / 10 / 5 | **Ébauche** (Florian) | KV (`pilotage__*`, copies) | oui | |
| Notes, bulletins (C06) | 19 / 3 400 / 5 / 0 / 3 / 9 | Ébauche [H] (Charlemagne officiel) | T + JB (`options`) | oui | |
| Vie scolaire élèves (C07) | 43 / 7 600 / 12 / 0 / 7 / 17 | Partiel [H] | T | oui | |
| Espace famille (C03, P07) | 49 / 4 650 / 17 / 10 / 2 / 2 | Partiel [H] | T + JB | oui | |
| Messagerie interne (P06) | 39 / 8 200 / 17 / 0 / 0 / 5 | Partiel [H] | T + S3-F (pièces) | non | |
| RGPD (P04) | 30 / 5 100 / 10 / 0 / 0 / 12 | Partiel [H] | KV | oui | |
| Assistant IA, OCR (P05) | 65 / 13 000 / 24 / 0 / 2 / 8 | sans objet (non repris) | T + **S3-J** (`knowledge/*.json`) | non (mais perdable) | |

**Effet sur le classement** : les modules **ébauches** (RH hors absences, paie, comptabilité et trésorerie, facturation si Florian le confirme, pilotage, covoiturage) passent en **Perdable ou Vide probable** : on ne reprend pas leur logique, et leurs données seulement si Florian signale une donnée réelle à garder (ex. un document RH déposé). Leurs socles (6, 15, 24–25, 47, 48, 49) sont conçus de zéro.

## 3. Inventaire par module et classement proposé

Colonnes : tables typées · collections JSON · fichiers S3 · indices (pourquoi ce classement) · classement proposé · avis de Florian (vide).

### 3.1 Modules de l'add-on (usage réel, D-035)

| # | Module | Tables typées | Collections JSON | Fichiers S3 / base64 | Indices [F] | Classement proposé | Avis de Florian |
|---|---|---|---|---|---|---|---|
| M01 | **Voyages et sorties** | `travel`, `travel_attr` (dossier aplati : budget, devis, programme, accompagnateurs…), `travel_participant`, `travel_history`, `travel_message`, `travel_message_read`, `travel_parent_blog`, `travel_parent_blog_post`, `travel_parent_blog_photo`, `travel_image_catalog` (global) ; `depense.travel_id` | `travels__email-devis-unmatched`, `settings__modules__travels` | photos du blog, devis, pièces | 44 routes, 42 fichiers qui lisent `travel` ; scripts `restore-travels-from-s3.ts`, `fix-travel-auto-image-acl.mjs` ; migrations récentes 0039, 0041, 0051, 0058, 0063 | **À conserver** (séjours en cours et à venir : intégral ; séjours terminés : **archive** pour l'historique et la synthèse APEL). Blog parents : **perdable** (`42` §26). | |
| M02 | **Réservation de salles** | `reservation_room`, `reservation_room_booking` | `reservation-rooms__*` (ancien format, migré), `settings__modules__prof-room` | — | scripts `migrate-reservation-rooms-from-collection.mjs`, `backfill-reservation-booking-names.mjs`, migrations 0025, 0031, 0034 | **À conserver** : salles + réservations **futures** ; réservations passées : **perdable** (ou archive 1 an si Florian s'en sert pour des statistiques). | |
| M03 | **Planning par domaines** (sessions, inscriptions, réservations de coordinateurs) | `domain_planning_domain/booking/session/signup` : **jamais utilisées par le code** | `domain-planning__domains`, `__bookings`, `__sessions`, `__signups`, `settings__modules__domain-planning` | — | données réelles dans les collections, tables typées mortes | **À vérifier** : à conserver si une campagne est en cours, sinon perdable. | |
| M04 | **Demandes** (assistance, demandes internes, routage) | `request`, `request_attr`, `request_comment`, `request_history`, `request_attachment` ; configuration `request_routing(_attr)`, `request_org(_attr)` | `requests__index` (ancien), `settings__requests-routing`, `settings__requests-org` | pièces jointes | 77 fichiers, `purge_at` sur `request` (purge déjà prévue) ; migration 0026, script `migrate-requests-config-typed.ts` | **À conserver** : demandes **ouvertes** intégralement ; closes : **archive** jusqu'à `purge_at`. Routage et organigramme : repris comme **proposition de paramétrage** relue par Florian. | |
| M05 | **Absences du personnel** (professeurs et OGEC, rattrapages, traitement des heures, justificatifs, import par OCR) | `absence` (48 colonnes), `absence_history`, `absence_message`, `absence_ingest_job` ; `paie_periode`, `paie_element.absence_id` | — | justificatifs (`document_keys`, `justification_file_url`), purge déjà tracée (`privacy_documents_purged_at`) | 108 fichiers, migrations 0028–0033, 0051 ; validateurs OGEC, synchronisation paie | **À conserver** si Florian confirme qu'il est en usage (année en cours + précédente : droits, paie, contentieux) ; s'il fait partie des « RH ébauche », perdable. `paie_*` : **ébauche, perdable**. `absence_ingest_job` : **abandonnée** (file technique). | |
| M06 | **RH du personnel** (dossier, documents, formations, habilitations, entretiens, congés, documents partagés, onboarding, « pouls ») | `personnel` (+ `payload` jsonb), `personnel_attr` (les sous-objets RH y sont aplatis : `ent-core-db.ts:1004`), `personnel_leave`, `personnel_shared_doc` ; `personnel_document/formation/habilitation/entretien` **jamais utilisées** | `personnel-ogec__*`, `meta-rh`, `personnel-index`, `rh__onboarding__records`, `rh__onboarding__tokens`, `rh__mood-pulse`, `settings__staff-directory`, `settings__organigramme` | pièces du dossier, documents partagés | 201 fichiers lisent `personnel` ; API `rh/` (espace, onboarding, planning, registre, sst, mood-pulse) | **Ébauche (D-035 précision 2) → Perdable / vide probable**, sauf ce que Florian désigne (ex. pièces déposées, échéances d'habilitations encore utiles). « Pouls » : **perdable**. Le socle 6 est conçu de zéro. | |
| M07 | **Registre santé et sécurité au travail** (SST) | `sst_campagne`, `sst_emargement`, `sst_fiche`, `sst_consultation`, `sst_proxy_deposant` | — | signatures en base64 | migrations 0064, 0067 (récentes) | **Archive** obligatoire (registre réglementaire, [H] conservation longue) + fiches ouvertes **à conserver**. | |
| M08 | **Demandes d'HSE** et **photocopies couleur** | — | `demandes-hse__*`, `photocopies-couleur__*` | — | API dédiées ; HSE = heures supplémentaires, alimente la paie | HSE : **à conserver** (année en cours). Photocopies : **perdable** (sauf compteurs si refacturés). | |
| M09 | **Certificats et attestations** (programmes, attributions, vérification publique) | `certificate_program(_attr)`, `certificate_award(_attr)` : **jamais utilisées** | `certificates__programs`, `__awards`, `__verify`, index | PDF | `app/certificates/verify/[token]` : des **liens de vérification déjà remis** (QR imprimés) | **À conserver** : attributions + **jetons de vérification** (les liens déjà distribués doivent continuer de répondre, §7 R-09). | |
| M10 | **Fiches de dialogue** (orientation, campagnes, étapes, signatures, appel) | `fd_campagne`, `fd_etape`, `fd_fiche`, `fd_reponse`, `fd_signature`, `fd_token`, `fd_pdf_archive` | — | PDF (S3 et base64), signatures base64 | 3 versions (0027, 0040, 0053), 0065, 0066 (appel), audit personas v2 | Campagne **en cours** : **à conserver** (à reprendre seulement si la bascule tombe en pleine campagne — sinon finir dans Next.js). Campagnes closes : **archive** (PDF signés versés au dossier de l'élève). `fd_token` : **abandonné** (jetons d'accès). | |
| M11 | **Internat** | `internat_batiment`, `internat_chambre`, `internat_affectation`, `internat_appel`, `internat_appel_ligne`, `internat_sortie` | `internat__roster`, `__students`, `__rooms`, `__buildings`, `__outings` (+ index), `__incidents`, `__journal`, `__messages`, `__activities`, `__study-groups`, `__supervisor-shifts`, `__installation-config`, `__installation-bookings`, `settings__modules__internat` | — | **deux générations coexistent** (tables typées + JSON) ; scripts `import-internat-excel.ts`, `import-siecle-eleves-internat.ts`, `internat-dedupe` | **À conserver** : bâtiments, chambres, affectations, autorisations de sortie, incidents, journal de l'année. Appels passés : **archive** (3 mois [R]). Messages, créneaux d'installation : **perdable**. | |
| M12 | **Stages** (offres, candidatures, conventions, jetons de signature) | `stage_offer(_attr)`, `stage_application(_attr)`, `stage_convention(_attr)`, `stage_token(_attr)` | `stages__offers`, `__conventions` (+ index), `__student-tokens`, `__sign-tokens`, `__offer-candidature-tokens`, `__auto-purge-state`, réglages `stage-*` | conventions signées | script `migrate-stages-to-typed.mjs` ; double stockage typé + collection | **À conserver** : conventions (pièce contractuelle), entreprises et tuteurs, offres ouvertes. Jetons : **abandonnés**. | |
| M13 | **Santé et infirmerie** | `infirmerie_passage`, `infirmerie_fiche`, `sante_pai`, `sante_extrait`, `sante_inaptitude_eps`, `sante_medicament_prise`, `sante_accident` | — | pièces (`document_id`) | migrations 0054–0056 ; `travels-sante-extraits` | **À conserver mais bloqué** : données de santé, **pas de reprise avant un hébergeur HDS** (D-034 : module santé désactivé). D'ici là : reste dans Next.js. | |
| M14 | **Cantine, passages** | `passage` ; réglages dans `tenant_setting_attr` ; `eleve_scolarite.grille_repas`, `repas_par_semaine` | — | — | `passages-facturation-db` : les passages alimentent la facturation | Passages **non encore facturés** : **à conserver** ; facturés : **perdable** (la facture fait foi). | |
| M15 | **Facturation et comptabilité de l'établissement** | `tarif`, `foyer_facturation` (**IBAN, BIC, RUM en clair**), `facture`, `facture_ligne`, `facture_echeance`, `encaissement`, `facture_encaissement`, `tresorerie_compte`, `depense`, `mouvement_tresorerie` | — | PDF de factures et quittances | 25 fichiers ; migration 0053 | **Ébauche (D-035 précision 2) → Vide probable.** Si Florian signale des factures réellement émises : **archive** comptable (pièces) + soldes ouverts ; sinon rien. Les socles 24, 25, 48 sont conçus de zéro. | |
| M16 | **Inscriptions : rendez-vous, préinscriptions, fiches de rentrée, modèles** | `rdv_inscription_config`, `rdv_inscription_direction`, `rdv_inscription_booking`, `rdv_inscription_email_gate`, `preinscription` | `rentree__submissions__*`, `documents__inscription__settings`, `documents__generated__*`, `settings__mef-secteurs` | PAP, pièces de rentrée | 10 migrations 0042–0052 ; route publique | Campagne **en cours** : **à conserver** (RDV futurs, préinscriptions non traitées, fiches de rentrée non intégrées). Passé : **perdable**. Jeton Google : **abandonné** (secret). | |
| M17 | **Accueil, portes ouvertes, invitations, partenariats** | `portes_ouvertes_config/slot/registration/slot_staff`, `invitation_page/eligible/rsvp`, `partenariat_offre/evenement/inscription` ; saisie accueil dans `vs_absence_eleve.source` | — | logos, signatures d'engagement | `backfill-portes-ouvertes-sql.mjs`, `resend-po-last-confirmations.ts` ; migrations 0054, 0057, 0059 | Portes ouvertes : prospects **de la campagne en cours** à conserver (avec consentement), sinon perdable. Invitations (cérémonie) : **perdable** après l'événement. Partenariats : offres et inscriptions de l'année **à conserver**. | |
| M18 | **Bien-être, signalements** | — | `bien-etre__signalements`, `settings__bien-etre` | — | API `bien-etre/signaler`, `signalements` | **À conserver** (signalements = suivi de situations d'élèves, accès très restreint) — à confirmer par Florian (nature exacte, [H] harcèlement). | |
| M19 | **Espace documents des personnels** (cloud, partages, Office) | — | `documents__shares`, `documents__file-shares` | `documents/users/<id>/`, `documents/shared/<id>/` | API `documents/*` (upload, move, shares, quota) | **À conserver** : ce sont les fichiers des utilisateurs (perte = incident). | |
| M20 | **Covoiturage, affiches, site public, tableau de semaine, répartition des classes** | — | `covoiturage__profiles/matches`, `posters__*`, `site-cms__posts`, `dashboard__week-sheet`, `class-allocation__audit-log`, `settings__modules__class-allocation` | affiches générées | petits modules | Covoiturage, tableau de semaine, répartition des classes (après la rentrée) : **perdable**. Affiches et site public : **à vérifier** (contenu réutilisable). | |

### 3.2 Doublons du cœur (copies locales de Charlemagne/SIECLE) — rapprochement

| # | Module | Tables typées | Collections | Indices | Classement proposé | Avis de Florian |
|---|---|---|---|---|---|---|
| C01 | Établissement, sites, années | `etablissement` (slug, `data_bucket`), `etablissement_site` (directeur, couleur, signature, niveaux), `annee_scolaire`, `calendrier_scolaire`, `ref_etablissement` | `settings__establishments`, `tenants__index` | une ligne par établissement [H : un seul en production, plusieurs sites] | **Rapprochement** ; signatures et couleurs des sites : à reprendre comme paramétrage. | |
| C02 | Élèves | `eleve` (INE, nom, prénom, naissance, classe en texte, e-mails et téléphones des parents à plat, régime, options), `eleve_scolarite`, `eleve_regime_periode` | `siecle__eleve-id-map`, `eleves__photo-index`, `pilotage__*` | importés une fois (`import-siecle-fixtures.ts`, `enrich-eleves-fill-only.ts`) ; doublons connus (`merge-eleve-person-ine-duplicates.ts`) | **Rapprochement** (le cœur fait foi). Photos : **à vérifier** (reprendre seulement si le cœur n'en a pas). | |
| C03 | Familles | `foyer`, `foyer_responsable` (`user_id` : lien vers le compte parent), `eleve_foyer_link` | — | — | **Rapprochement** ; les comptes parents rattachés sont la donnée utile. | |
| C04 | Classes, groupes, enseignants, EDT | `classe_site_mapping`, `school_class_assignment`, `school_roster_meta`, `groupe_pedagogique(_membre)`, `enseignant`, `teacher_planning`, `teacher_planning_slot`, `teacher_planning_replacement`, `edt_creneau` | `settings__modules__teaching-groups`, `__timetable-grids` | classes en **texte libre** partout (`classe`, `classes`) | **Rapprochement** ; plannings de professeurs : **perdable** (l'EDT du cœur, socle 10, fait foi). | |
| C05 | Nomenclatures | `ref_nomenclature`, `nomenclature_import_log` | `settings__mef-secteurs` | imports SIECLE | **Abandonnée** (le cœur a ses référentiels, socle 9). | |
| C06 | Notes, compétences, bulletins, conseils (« phases Charlemagne » p1–p5) | `note_*` (9 tables), `bulletin`, `bulletin_ligne` (inutilisée), `conseil_seance`, `conseil_avis` (inutilisées), `export_academique` (inutilisée), `annee_bascule` (inutilisée), `cahier_texte` | — | tables récentes, peu de code, Charlemagne officiel | **Vide probable** ; sinon **perdable** (Charlemagne fait foi pendant le doublon, D-034). | |
| C07 | Vie scolaire élèves | `vs_appel`, `vs_appel_ligne`, `vs_absence_eleve` (dont saisie **accueil**), `vs_sanction`, `vs_sanction_type`, `vs_carnet_entree` | — | l'accueil saisit réellement des absences d'élèves (0028) | **Perdable** (Charlemagne officiel) ; à vérifier si des **absences saisies par l'accueil** de l'année en cours doivent suivre (Q-T0-07). | |

### 3.3 Plateforme, sécurité, technique

| # | Module | Tables | Classement proposé | Avis de Florian |
|---|---|---|---|---|
| P01 | Comptes et rôles | `user` (e-mail, nom, `platform_admin`, `org_admin`, `must_change_password`), `user_role`, `user_membership`, `auth_user_mapping`, `account` (méthode `credential` = mot de passe ; fournisseurs OAuth) | **À conserver** par **rapprochement** (§6.6) : identité, rôles et rattachements ; **pas** les mots de passe ni les jetons. | |
| P02 | Authentification technique | `session`, `verification`, `two_factor` (secret TOTP chiffré), `passkey`, `rate_limit`, `app_rate_limit` | **Abandonnée** (§5, §6.7). | |
| P03 | Journaux | `security_audit_log`, `data_access_audit`, `eleve_access_audit`, `document_access_request`, `metier_event` | **Archive** (export figé, consultable, conservation [H] 6 mois à 1 an pour les journaux de sécurité ; `document_access_request` ouvertes à reprendre). Pas de fusion dans le journal en ajout seul du cœur. | |
| P04 | RGPD | — (collections `rgpd__workspace`, `rgpd__incidents`) | **À conserver** : registre des violations et demandes en cours (obligation de documentation). | |
| P05 | Assistant IA | `scolia_conversation`, `scolia_message` ; collections `chat__messages`, `channels__channels` | **Perdable** (conversations avec l'assistant, anciens canaux). | |
| P06 | Messagerie interne des personnels | `messaging_conversation`, `messaging_participant`, `messaging_message`, `messaging_attachment`, `messaging_reaction`, `messaging_presence` | **À vérifier** : [R] **archive** lecture seule pendant la durée de rétention existante (`scripts/messaging-retention.ts`), pas de reprise dans le socle 16. Présence : abandonnée. | |
| P07 | Messagerie familles | `famille_thread`, `famille_thread_message`, `famille_thread_attachment`, `famille_notif`, `famille_messaging_settings` | **À vérifier** : fils **ouverts** à conserver (repris en lecture seule dans le socle 16), sinon archive. Notifications : abandonnées. | |
| P08 | Abonnement plateforme | API `billing/*` (EasyTransac, relances), `platform_document(_attr)`, collection `microsoft-license-request` | **Abandonnée** (facturation du service, pas de l'établissement). | |
| P09 | Conteneurs techniques | `ent_collection_record/attr`, `tenant_setting_section/attr`, `ent_entity`, `tenant_document` (supprimées en 0007), `absence_ingest_job` | Décodées puis **abandonnées** (le contenu utile part dans les modules ci-dessus). | |
| P10 | Paramétrages | collections `settings__*` (site, intégrations, notifications, liens externes, accès aux modules, toolbox…) | **Perdable**, avec une **liste lisible** remise à Florian pour reparamétrer ScolIA (D-023 : peu de réglages). | |

---

## 4. Correspondance Next.js → ScolIA

Principe (`TRANSITION_NEXT_RUST.md` §2 bis) : toute ligne reprise est **rattachée à une fiche du cœur** (élève, personne, agent, classe, salle, année) par la table de correspondance (§8.4) ; les colonnes « copie » (nom de l'élève, classe en texte, nom de l'enseignant) ne sont **pas** reprises : elles servent seulement au rapprochement et au contrôle. Cibles : **[existe]** table présente dans `ent-rust/migrations/` ; **[conçu]** table décrite dans `docs/socles/` ; **[à venir]** socle non conçu.

### 4.1 Voyages et sorties (socle 42) — détail au `42_SORTIES_VOYAGES.md` §26
| Next.js | ScolIA | Remarque |
|---|---|---|
| `travel` (type, statut, dates, destination, titre) | `ent_sortie` [conçu] | type SIMPLE/COMPLEX → sortie ou voyage ; statut → état (§8.1 du 42) |
| `travel.classes` (texte) | `ent_sortie_public` → `ent_classe` [existe] | proposition, sinon liste d'élèves |
| `travel.owner_*` | personne / agent du socle 6 (`ent_agent`) [existe] | via correspondance des comptes |
| `travel_attr` : budget, devis, programme, restauration, accompagnateurs | `ent_sortie_budget(_ligne)`, `ent_sortie_devis`, `ent_sortie_etape`, `ent_sortie_restauration`, `ent_sortie_encadrant` [conçu] | dossier **reconstruit** puis converti ; montants en centimes, douteux signalés |
| `travel_participant` (`eleve_id`, `eleve_key`, nom, classe, droit à l'image, panier) | `ent_sortie_participant` → `ent_eleve` [existe] ; `ent_sortie_panier` [conçu] | `eleve_id`, puis INE, puis file humaine (CV-60) |
| `travel_history` | `ent_sortie_journal` [conçu] | marqué « repris » |
| `travel_message`, `travel_message_read` | fil interne en lecture seule (socle 16) | auteur = libellé texte : pas de rattachement fiable |
| `travel_parent_blog*` | **non repris** | contenu éphémère |
| `travel_image_catalog` | catalogue d'images du socle 42 | **vérifier les licences** colonne par colonne (`license`, `attribution_url`) |
| `depense.travel_id` | `ent_sortie_budget_ligne` réalisée / socle 48 | |

### 4.2 Réservations et planning
| Next.js | ScolIA | Remarque |
|---|---|---|
| `reservation_room` (nom, bâtiment, type, réservable) | `ent_salle` [existe], `ent_batiment`, `ent_lieu` [existe] (socle 5) | **rapprochement** par nom normalisé + bâtiment ; salle inconnue → file humaine (jamais de création silencieuse) |
| `reservation_room_booking` | réservation de salle du socle 5/44 **[à venir]** (`ent_salle_indisponibilite` [existe] ne suffit pas : pas de demandeur ni de motif) | demandeur → `ent_user` ; `class_name` → `ent_classe` si reconnu |
| collections `domain-planning__*` | socle 19 (agenda, réunions, rendez-vous) **[à venir]** ; `ent_rencontre`, `ent_creneau_rdv` [existe] à étudier | `external_user_id` → compte |

### 4.3 Demandes (socle à venir : « demandes et assistance », hors carte actuelle — voir `SOCLES.md` §7)
| Next.js | ScolIA | Remarque |
|---|---|---|
| `request` + `request_attr` | **socle à venir** ; en attendant, aucune cible | demandeur : `requester_user_id` → compte ; sinon e-mail |
| `request_comment`, `request_history`, `request_attachment` | idem ; pièces → dépôt du socle 7 (`ent_doc_document` [conçu]) | |
| `request_routing`, `request_org` | paramétrage du futur socle (proposition relue) | services → équipes du socle 6 (`ent_equipe` [conçu]) |

**Point à trancher (Q-T0-04)** : les demandes n'ont pas de socle propre dans `SOCLES.md` ; leur bascule (prévue en T3) suppose de concevoir ce socle d'ici février.

### 4.4 Absences et RH du personnel (socles 6, 15, 49)
| Next.js | ScolIA | Remarque |
|---|---|---|
| `personnel` (e-mails, nom, catégorie, poste, date d'entrée, actif, `manager_id`) | `ent_agent` [existe 352], `ent_agent_fonction` [existe 353], `ent_agent_engagement` [existe], `ent_personne` [existe] | rapprochement §6.5 |
| `personnel_attr` (formations, habilitations, documents, entretiens aplatis) | `ent_agent_qualification` [existe], `ent_habilitation_signature`, `ent_agent_dossier_piece` [conçu] ; entretiens **[à venir]** | |
| `personnel_leave` | congés : socle 15 **[à venir]** | |
| `absence` (+ history, message) | `ent_absence_rh` [existe 080] **trop pauvre** (dates, motif, 3 statuts) : rattrapages, traitement des heures, décision du chef, justificatif, confidentialité du motif → socle 15 **[à venir]** | la bascule M05 attend la conception du socle 15 |
| `paie_periode`, `paie_element` | socle 49 **[à venir]** | variables de paie |
| collections `demandes-hse__*` | socle 49 ou 15 **[à venir]** | |
| `sst_*` | socle « santé et sécurité au travail » **[à venir]** (hors carte : à placer, Q-T0-05) | registre |
| `rh__onboarding__*` | `ent_parcours_agent`, `ent_parcours_etape` [conçu, 06] | |
| `rh__mood-pulse` | `ent_pouls_campagne/reponse` [conçu, 06] | non repris nominativement [R] |
| `personnel_shared_doc` | dépôt du socle 7 | |

### 4.5 Élève et autour (socles 7, 14, 34, 37, 39, 40, 41)
| Next.js | ScolIA | Remarque |
|---|---|---|
| `eleve_document` (+ fichier S3) | `ent_eleve_document` [existe 016] → `ent_doc_document` [conçu 07] | tiroir et confidentialité à faire correspondre |
| `fd_*` (fiches de dialogue) | socle 37 orientation **[à venir]** ; PDF signés → socle 7 | `fd_fiche.eleve_id` puis INE (`eleve_ine` copié) |
| `certificates__*` | `ent_doc_programme_attestation`, `ent_doc_attestation_ligne` [conçu 07] ; jetons de vérification conservés | |
| `stage_*` | `ent_convention_stage`, `ent_entreprise` [existe 082] ; socle 39 **[à venir]** pour le reste | |
| `internat_*` + `internat__*` | `ent_internat_chambre`, `ent_internat_affectation`, `ent_internat_presence`, `ent_internat_suivi` [existe 066], `ent_batiment` [existe] ; socle 41 **[à venir]** | les deux générations sont **fusionnées** : les tables typées priment, les JSON complètent (sorties, incidents, journal) |
| `infirmerie_passage`, `infirmerie_fiche`, `sante_pai`, `sante_inaptitude_eps`, `sante_accident`, `sante_medicament_prise`, `sante_extrait` | `ent_infirmerie_passage` [existe 040/213], `ent_fiche_sante` [existe 045], `ent_pai` [existe 052/210], `ent_san_inaptitude`, `ent_san_mouvement` [conçu 34] | **bloqué HDS** |
| `passage` | `ent_repas_passage` [existe 036] | sens/lieu à faire correspondre |
| `portes_ouvertes_*`, `rdv_inscription_*`, `preinscription`, `rentree__submissions` | `ent_inscription_accueil(_creneau, _presence)`, `ent_inscription_rdv` [conçu 03], `ent_preinscription_verification` [existe], `ent_fiche_rentree` [conçu 04] | prospects : personnes « contacts », pas élèves |
| `partenariat_*` | socle 43 périscolaire **[à venir]** | |
| `invitation_*` | aucune (perdable) | |
| `bien-etre__signalements` | socle 36 suivi social **[à venir]** | accès restreint |

### 4.6 Facturation (socles 24–25)
| Next.js | ScolIA | Remarque |
|---|---|---|
| `tarif` | `ent_fin_tarif`, `ent_fin_article` [existe 200] | |
| `foyer_facturation` (IBAN, BIC, RUM, mandat) | `ent_fac_sepa_mandat` [existe 030], `ent_foyer_iban_histo` [existe] (chiffré) | IBAN en clair dans Next.js → chiffré à l'arrivée ; **mandat SEPA** : la RUM et la date de signature doivent être conservées telles quelles pour rester valides [H] |
| `facture`, `facture_ligne`, `facture_echeance` | `ent_fac_facture`, `ent_fac_ligne`, `ent_fac_echeance` [existe] | numéros **conservés** (numérotation continue) |
| `encaissement`, `facture_encaissement` | `ent_fin_reglement`, `ent_fin_lettrage` [existe 203] | totaux contrôlés au centime |
| `tresorerie_compte`, `mouvement_tresorerie`, `depense` | socle 48 **[à venir]** | |

### 4.7 Comptes (socle 1)
| Next.js | ScolIA | Remarque |
|---|---|---|
| `user` | `ent_user` [existe 005], `ent_personne` [existe 120] | clé : e-mail normalisé ; `password_hash` du cœur reste à une valeur **inutilisable** jusqu'à l'activation (§6.7) |
| `user_role` (17 rôles d'intranet), `org_admin`, `platform_admin` | `ent_role_attribution` [existe 098], `ent_membership(_casquette)` [existe] | table de passage rôle Next.js → rôle paramétrable, **relue par Florian** ; `platform_admin` **non repris** (super-administration hors établissement) |
| `user_membership` | `ent_membership` | |
| `foyer_responsable.user_id` | `ent_eleve_lien` + `ent_personne` [existe] | le compte parent est rattaché à la personne responsable du cœur |
| `account` (Microsoft, Google) | `ent_oidc_identity` [existe 090] | seulement l'identifiant chez le fournisseur (`account_id`, `provider_id`), **jamais** les jetons |

---

## 5. Tables abandonnées (non reprises) et pourquoi

### 5.1 Copies locales et miroirs (le contenu utile passe par le rapprochement)
`eleve`, `eleve_scolarite`, `eleve_regime_periode`, `foyer`, `foyer_responsable`, `eleve_foyer_link`, `enseignant`, `school_class_assignment`, `school_roster_meta`, `classe_site_mapping`, `groupe_pedagogique`, `groupe_pedagogique_membre`, `annee_scolaire`, `calendrier_scolaire`, `etablissement_site` (hors signature et couleur), `ref_etablissement`, `ref_nomenclature`, `teacher_planning*`, `edt_creneau`, collections `pilotage__*`, `personnel-index`, `eleves__photo-index` (index). Raison : le cœur, alimenté par Charlemagne/SIECLE, fait foi ; ces tables servent **d'entrée au rapprochement** (§6), puis ne sont plus lues.

### 5.2 Technique, secrets, jetons
| Table / colonne | Raison |
|---|---|
| `session`, `verification`, `rate_limit`, `app_rate_limit` | état éphémère |
| `two_factor` | secret TOTP chiffré avec la clé de Next.js (on ne touche pas aux secrets) ; remplacé par la passkey (D-009) |
| `account.password`, `account.access_token/refresh_token/id_token` | empreinte de format différent (§6.7) ; jetons OAuth = secrets |
| `rdv_inscription_config.google_refresh_token`, `google_linked_*` | secret ; Florian reconnecte l'agenda dans ScolIA |
| `fd_token`, `stage_token(_attr)`, `rdv_inscription_email_gate`, `stage__*-tokens`, `rh__onboarding__tokens`, `travel_parent_blog.token` | jetons d'accès à usage unique : les reprendre prolongerait des accès ; on réémet si besoin. **Exception** : jetons de vérification des certificats (§7 R-09). |
| `absence_ingest_job` | file de traitement |
| `nomenclature_import_log` | journal d'import Next.js |
| `ent_entity`, `tenant_document` | supprimées en 0007 (miroir JSON) |
| `ent_collection_record/attr`, `*_attr`, `tenant_setting_*` | conteneurs : décodés puis répartis |
| `messaging_presence`, `famille_notif`, `travel_message_read` | état d'interface |
| `platform_document(_attr)`, `billing/*`, `microsoft-license-request` | plateforme Next.js |
| `scolia_*`, `chat__messages`, `channels__channels` | conversations IA et anciens canaux |

### 5.3 Tables jamais utilisées par le code [F] (probablement vides)
`annee_bascule`, `bulletin_ligne`, `certificate_program`, `certificate_program_attr`, `certificate_award`, `certificate_award_attr`, `conseil_seance`, `conseil_avis`, `domain_planning_domain`, `domain_planning_booking`, `domain_planning_session`, `domain_planning_signup`, `export_academique`, `personnel_document`, `personnel_formation`, `personnel_habilitation`, `personnel_entretien` (+ `ent_entity`, `tenant_document`, supprimées). Le rapport de reprise **compte quand même leurs lignes** : une table « morte » non vide est signalée (une ancienne version du code a pu l'écrire).

---

## 6. Règles de rapprochement des doublons

**Principes [R]**
1. **Le cœur fait foi.** On ne crée jamais une fiche du cœur à partir de Next.js sans décision humaine (sauf objets propres à l'add-on : séjours, salles réservables reconnues, demandes).
2. **Clé forte d'abord**, puis clé composée, puis **file humaine**. Aucun rapprochement approximatif automatique (ressemblance de nom, faute de frappe) : il est **proposé**, jamais appliqué.
3. **Annulable** : chaque rattachement est inscrit dans la table de correspondance avec sa règle, son auteur et sa date ; on peut le défaire et rejouer. Si un doublon **dans le cœur** est découvert à cette occasion, on utilise la fusion annulable du socle 2 (E6 : `ent_eleve_fusion`, `ent_eleve_fusion_ligne`, migrations 292–294) ou celle des personnes (`ent_personne_fusion`, 123).
4. **Un résultat par fiche** : `rapprochée (règle)`, `à décider`, `ignorée (raison)`, `créée (décision de X)`.
5. La file humaine est présentée à **Florian ou au secrétariat**, avec les deux fiches côte à côte et les champs différents surlignés.

### 6.1 Établissements et sites
- Établissement : par **UAI/RNE** (`ref_etablissement.code_rne` ou paramètre) ; à défaut, choix manuel (il y en a un, [H]). Tout le reste de l'export est rattaché à cet établissement du cœur.
- Sites : `etablissement_site.site_id/label/grades` → `ent_site` par libellé normalisé ; choix manuel au premier passage, mémorisé.

### 6.2 Années scolaires
- `annee_scolaire.label` (« 2025-2026 ») et dates → `ent_annee_scolaire` par **dates de début et de fin** ; écart de quelques jours accepté et signalé. Une année absente du cœur (année passée) n'est **pas** créée : les données de cette année vont en archive.

### 6.3 Classes et groupes (texte libre partout dans Next.js)
- Normalisation (casse, espaces, « 2nde » / « 2de » / « Seconde », « 2B » / « 2nde B ») **proposée** avec une table de synonymes relue une fois par Florian, puis appliquée par l'année : `(année, libellé normalisé)` → `ent_classe`.
- `classe_site_mapping.siecle_code` sert de clé forte quand il est rempli.
- Libellé non reconnu (« 2nde B, Autres ») : le public de l'objet devient une **liste d'élèves** (42 CV-61).

### 6.4 Élèves
Ordre des règles :
1. `eleve_id` Next.js déjà relié par une reprise précédente (table de correspondance) ;
2. **INE** identique et unique des deux côtés (après normalisation majuscules, sans espaces ; contrôle de la clé de l'INE) ;
3. **nom + prénom + date de naissance** identiques après normalisation (accents, tirets, noms composés, ordre des prénoms) et **unique** dans le cœur ;
4. même règle 3 avec le **nom d'usage** ou un prénom secondaire → **à décider** (proposée) ;
5. `siecle__eleve-id-map` (correspondance SIECLE gardée par Next.js) : indice supplémentaire, jamais suffisant seul.
Cas toujours présentés à un humain : jumeaux (même nom et naissance, prénoms différents → règle 3 les sépare, mais on vérifie), INE identique avec date de naissance différente, deux élèves Next.js pour un seul élève du cœur (doublon Next.js : les deux sont rattachés au même élève, le rapport le note), élève Next.js sorti absent du cœur (archive seulement).
Les objets qui ne portent qu'un **nom en texte** (`travel_participant` sans `eleve_id`, `invitation_eligible`, `partenariat_inscription`, `rdv_inscription_booking`) passent par les règles 3–4 avec la **classe de l'année** comme discriminant.

### 6.5 Personnels
1. `personnel.external_user_id` / compte déjà relié ;
2. **e-mail professionnel** normalisé (`email_pro`, puis `email`) = e-mail d'un `ent_agent` ou d'un `ent_user` ;
3. nom + prénom + date d'entrée → **à décider** ;
4. `enseignant` (copie) : par e-mail, puis nom + prénom.
L'e-mail **personnel** (`email_perso`) n'est jamais une clé de rapprochement automatique (partagé, changeant) : seulement un indice.

### 6.6 Comptes
- Clé : **e-mail normalisé** (minuscules, sans espaces ; pas de suppression des points ni des « + »). Un compte du cœur existe → rattachement ; sinon création **en attente d'activation** si la personne est rapprochée (agent ou responsable) et que Florian valide la liste.
- Parents : `foyer_responsable.user_id` → responsable du cœur rapproché par l'élève (§6.4) puis par e-mail ; un e-mail partagé par deux parents (fréquent) → **à décider**.
- Rôles : table de passage des 17 rôles Next.js (`app/lib/intranet-roles.ts`) vers les rôles paramétrables (D-017), relue par Florian ; un rôle sans équivalent n'est **pas** accordé.
- Comptes sans personne rapprochée (anciens personnels, comptes de test) : **ignorés**, listés.

### 6.7 Mots de passe, double authentification et passkeys
- **Format dans Next.js [F + H]** : better-auth 1.7.2 (`package.json`), `emailAndPassword` sans fonction de hachage personnalisée (`app/lib/auth-server.ts:388-395`) : l'empreinte est dans `account.password` (ligne `provider_id = 'credential'`) au format **par défaut de better-auth, scrypt** (`sel:empreinte` en hexadécimal) [H : format par défaut, à confirmer sur un export anonymisé — sans lire aucune empreinte réelle].
- **Format dans ScolIA [F]** : `ent_user.password_hash` en **Argon2id** (`ent-auth`, D-009).
- **Conséquence [R]** : on **ne reprend pas** les empreintes. Vérifier un scrypt au premier login puis réécrire en Argon2id serait possible techniquement, mais garderait une seconde voie de vérification dans le code de sécurité pour un gain faible, alors que D-009 vise la passkey pour tous. **Première connexion par lien sécurisé** (envoi par lots, lien court, à usage unique — le mécanisme de récupération D-018 existe) **puis enregistrement d'une passkey**.
- **TOTP** : non repris (secret chiffré par Next.js).
- **Passkeys** : Next.js utilise `rpID` = `scolia.fr` (`app/lib/passkey-rp.ts:3`) ; ScolIA lit `ENT_RP_ID` (`ent-auth/src/passkey.rs:55-74`). Si ScolIA garde le **même domaine enregistrable**, une clé publique better-auth (`passkey.public_key`, `credential_id`, `counter`) pourrait être convertie au format `webauthn-rs` [H : faisabilité à prouver sur données fictives]. [R] **Ne pas le faire en V0** : ré-enrôlement simple, et cela évite de dépendre d'un format interne de better-auth. À rouvrir seulement si l'enrôlement pose problème (Q-T0-09).

---

## 7. Risques de perte et contrôles

### 7.1 Risques
| N° | Risque | Parade |
|---|---|---|
| R-01 | Oublier les **collections JSON** et ne reprendre que les tables typées | inventaire des collections **à partir de l'export** (`SELECT DISTINCT collection`) comparé à la liste du §3 ; toute collection inconnue bloque la reprise |
| R-02 | Oublier les **fichiers S3** (photos, justificatifs, conventions, espace documents) | export du bucket ; contrôle : chaque clé référencée en base existe (taille, empreinte) ; objets orphelins listés |
| R-03 | **Double génération** (internat, stages, réservations, demandes : typé + JSON) | règle « le typé prime, le JSON complète », écarts listés |
| R-04 | Tables créées hors migrations absentes ou différentes en prod | l'outil lit le **catalogue réel** de l'export (`information_schema`) et compare au schéma attendu ; écart = arrêt |
| R-05 | Textes libres (classes, noms) mal rapprochés | aucune fusion approximative automatique ; file humaine ; échantillons relus |
| R-06 | Montants mal convertis (texte « 1.350 », virgule, `numeric`) | conversion en centimes ; totaux par voyage et par facture comparés au centime ; valeurs ambiguës refusées et listées |
| R-07 | Fuseau et dates en texte (`travel_history.at`, `start_time` en texte) | interprétation en Europe/Paris, explicite ; dates invalides refusées |
| R-08 | Données de **santé** reprises sans hébergeur HDS | module santé exclu par défaut ; option refusée tant que le paramètre HDS n'est pas vrai |
| R-09 | **Liens déjà distribués** cassés (vérification des certificats, suivi de voyage, liens de rendez-vous) | jetons de vérification des certificats repris ; anciennes adresses **redirigées** vers ScolIA (action de Florian au moment de la bascule) ; autres liens : courriel d'information |
| R-10 | Double saisie pendant la bascule | module Next.js en lecture seule (Florian) avant l'export final |
| R-11 | Fuite pendant les répétitions | essais seulement sur export **anonymisé** ; données réelles après accord écrit et avis du DPO (D-035) ; rapports sans donnée personnelle en clair |
| R-12 | IBAN en clair dans l'export | chiffrés dès le chargement en zone de reprise ; l'export réel est chiffré au repos et détruit après la bascule (date notée au rapport) |
| R-13 | Pièces en base64 volumineuses | extraites en fichiers, empreinte calculée, déposées au socle 7 |
| R-15 | Données restées en **JSON sur S3** (fiches de rentrée et demandes en attente, devis reçus par e-mail, JSON historiques jamais migrés) absentes du `pg_dump` | inventaire du bucket par préfixe ; décodage et ventilation en tables (D-036) ; préfixes inconnus listés au rapport |
| R-16 | **Secrets** dans le bucket (dossier de secrets du registre des établissements) copiés par erreur | export S3 par **liste blanche de préfixes**, secrets exclus ; l'outil refuse un objet dont le chemin correspond au préfixe des secrets |
| R-17 | Une clé JSON non prévue est perdue lors de la ventilation en tables | décodeur strict : toute clé inconnue = ligne refusée, listée ; jamais « rangée en JSON » côté ScolIA (D-036) |
| R-14 | Signatures (fiches de dialogue, SST, partenariats) qui perdent leur valeur | reprises **avec** leur horodatage, signataire et PDF scellé d'origine ; marquées « signée dans l'ancien outil » |

### 7.2 Contrôles du rapport de reprise (par module, à chaque exécution)
1. **Comptages** : lignes lues par table et par collection ; lignes reprises, archivées, ignorées (raison), refusées (raison), à décider. Invariant : `lues = reprises + archivées + ignorées + refusées + à décider`.
2. **Totaux** : montants par voyage (budget, encaissé, dépensé), par facture et par foyer (dû, payé, solde) ; heures d'absence par agent et par mois ; nombre de participants par séjour ; nuitées.
3. **Rapprochement** : nombre et pourcentage par règle (§6) ; liste « à décider ».
4. **Échantillons** : 20 objets tirés au hasard par module (graine notée, rejouable), présentés côte à côte Next.js / ScolIA pour relecture humaine.
5. **Fichiers** : référencés, trouvés, manquants, orphelins ; empreintes concordantes.
6. **Refus** : fichier détaillé (identifiant source, table, règle, message), sans donnée personnelle en clair dans le résumé.
7. **Écarts de schéma** : tables ou colonnes inattendues, tables « mortes » non vides.
8. **Comparaison avec l'exécution précédente** (répétitions) : ce qui a changé.
Le rapport est un document lisible (HTML ou PDF) + un fichier machine (JSON) ; Florian le relit avant chaque ouverture de module (`TRANSITION_NEXT_RUST.md` §3.2).

---

## 8. Esquisse de l'outil `ent-reprise` (Rust)

### 8.1 Forme
Binaire de ligne de commande dans `ent-rust/crates/ent-reprise` [R], réutilisant `ent-db` (sqlx) et les services métier du cœur **pour écrire** (pas d'insertion directe dans les tables d'un socle : on appelle ses commandes, règle 1 de `SOCLES.md` §1) ; journal et outbox comme toute action ; l'acteur est un compte technique « reprise » nommé par Florian.

### 8.2 Entrées
- `--source` : une base PostgreSQL **locale et jetable** restaurée depuis le `pg_dump` fourni par Florian (anonymisé d'abord), jamais la production ; l'outil refuse de démarrer si l'adresse ne ressemble pas à une base locale (garde comme `scripts/test-database-guard.mjs` côté Next.js) et ouvre la source **en lecture seule** (`default_transaction_read_only`).
- `--fichiers` : copie du bucket S3 (répertoire ou bucket de reprise).
- `--cible` : base ScolIA (répétition : base jetable ; T2 : pré-production fermée).
- `--modules` : liste (`voyages,salles,…`), `--annee`, `--etablissement`.
- `--decisions` : fichier des décisions humaines de rapprochement (rejoué à chaque exécution).

### 8.3 Étapes
1. **Inspection** : catalogue réel de la source comparé au schéma attendu (version du schéma Next.js connue de l'outil) ; inventaire des collections ; comptages. Arrêt si écart non connu.
2. **Chargement en zone de reprise** : schéma `reprise` dans la cible (tables `reprise_*` brutes, une par source), documents aplatis **reconstruits** ; rien dans les tables du cœur.
3. **Référentiels** : établissement, sites, années, classes (§6.1–6.3).
4. **Personnes** : élèves, responsables, personnels, comptes (§6.4–6.6) → table de correspondance ; file « à décider ».
5. **Modules** demandés, dans l'ordre des dépendances ; chaque objet appelle la commande du socle propriétaire, avec `source = 'reprise'`.
6. **Fichiers** : copie vers le dépôt du socle 7 avec empreinte.
7. **Rapport** (§7.2).
Mode `--simulation` (défaut) : tout dans une transaction **annulée** à la fin, rapport complet produit. `--appliquer` : validé, seulement avec la confirmation explicite de Florian.

### 8.4 Table de correspondance des identifiants
`reprise_correspondance (etablissement_id, source_systeme = 'next', source_table, source_id, cible_table, cible_id, regle, statut, decide_par, decide_le, execution_id)`, unique sur `(etablissement_id, source_systeme, source_table, source_id)`. Elle sert d'**idempotence** (un objet déjà repris est mis à jour, jamais dupliqué), de **traçabilité** (d'où vient cette ligne) et d'**annulation** (retirer une exécution = défaire les objets de cet `execution_id`, dans l'ordre inverse).

### 8.5 Idempotence et répétitions
- Rejouer sur le même export = aucun changement ; sur un export plus récent = mises à jour + nouveautés, rapport des différences.
- **Mode répétition** : base cible jetable recréée à chaque fois (`--cible-jetable`), chronométrage par étape (la bascule doit tenir dans un vendredi soir), comparaison avec la répétition précédente.
- Les objets repris restent marqués `source = 'reprise'` et `source_id` : une modification faite ensuite dans ScolIA n'est **pas** écrasée par une nouvelle exécution (la ligne passe « modifiée depuis la reprise », signalée).

### 8.6 Ce que l'outil ne fait jamais
Se connecter à la production ; lire un secret ; écrire dans la source ; reprendre une donnée de santé sans le paramètre HDS ; créer une fiche du cœur sans décision ; envoyer un message (les courriels d'activation sont une étape séparée, déclenchée par Florian).

---

## 9. Outil d'anonymisation (`ent-reprise anonymiser`)

**But** : produire, **chez Florian**, à partir du `pg_dump` réel restauré dans une base locale, un export qui garde **la structure, les volumes, les liens et les cas difficiles**, sans identité réelle. Le pilote ne reçoit que ce résultat.

**Règles [R]**
1. **Cohérent** : une même valeur réelle donne toujours la même valeur fictive dans tout l'export (pseudonyme déterministe **HMAC** avec une clé générée sur la machine de Florian, jamais transmise, détruite après) ; les liens et les doublons restent donc visibles (deux fiches du même élève restent deux fiches du même faux élève).
2. **Remplacé** : noms, prénoms (par des listes fictives, en gardant le sexe et la longueur approximative), dates de naissance (décalées de ± 60 jours, même année scolaire de naissance), lieux de naissance, adresses (rue fictive, code postal gardé au département), téléphones (format gardé, numéros fictifs de la plage réservée à la fiction), e-mails (`prenom.nom.<hash>@exemple.invalid`, domaine gardé en catégorie : académique, perso, entreprise), **INE** (faux INE avec clé valide), **IBAN/BIC/RUM** (IBAN fictif valide par sa clé, même pays), UAI de l'établissement d'origine (gardé : donnée publique), `external_user_id`, adresses IP, agents utilisateurs.
3. **Vidé** : textes libres à risque (motifs d'absence, notes CPE, observations SST, corps des messages, commentaires de demandes, signalements bien-être, notes infirmières) remplacés par un texte fictif **de même longueur** ; images et signatures base64 remplacées par une image neutre ; jetons et secrets (`account.password`, `two_factor`, `google_refresh_token`, `token`, `secure_code`) **vidés**.
4. **Gardé** : identifiants techniques, statuts, dates de création (décalées d'un même nombre de jours par export si Florian le souhaite), **montants**, quantités, libellés de nomenclature, noms de classes et de salles, structure des documents aplatis (chemins des `*_attr`).
5. **Fichiers S3** : pas copiés ; remplacés par des fichiers factices de **même taille et même type**, empreinte recalculée.
6. **Contrôle final** : recherche dans l'export anonymisé des valeurs réelles les plus fréquentes (noms de famille, domaines d'e-mail, INE) — zéro occurrence exigée, sinon l'export n'est pas livré ; rapport des colonnes traitées, **toute colonne non classée bloque** (liste blanche explicite, pas de « par défaut on garde »).
7. Les données de santé sont **exclues** de l'export anonymisé par défaut (tables `sante_*`, `infirmerie_*`).

---

## 10. Découpage en lots (proposition)

| Lot | Contenu | Prérequis | Période |
|---|---|---|---|
| **R0** | Squelette `ent-reprise` : inspection d'un dump, comparaison au schéma attendu, inventaire des collections, rapport de comptages. **Jeu de données fictif au format Next.js** (construit par le pilote à partir du schéma, sans donnée réelle) | — | novembre |
| **R1** | Outil d'anonymisation (§9) avec liste blanche des 184 tables et des collections ; livré à Florian | R0 | fin novembre |
| **R2** | Zone de reprise, décodage strict des documents aplatis et des JSON (base et S3) vers des tables normalisées (D-036), table de correspondance, mode simulation | R0 | début décembre |
| **R3** | Référentiels et personnes : rapprochement établissement, sites, années, classes, élèves, responsables, personnels, comptes ; file « à décider » ; décisions rejouables | R2, socles 2, 4, 5, 6 | décembre |
| **R4** | **Première répétition** sur export anonymisé (T1, vacances de Noël) : R0–R3 + rapport | R1–R3, export de Florian | 19 déc. → 4 janv. |
| **R5** | Module salles (M02) | socle 5 (salles), réservation | janvier–février |
| **R6** | Module voyages (M01), selon `42` §26 | socle 42 lot V0 | février–mars |
| **R7** | Fichiers : copie S3 → dépôt du socle 7, empreintes | socle 7 | février |
| **R8** | Absences et RH du personnel (M05–M08) | conception et code du socle 15 ; socle 6 | mars–avril |
| **R9** | Demandes (M04) | socle « demandes » à concevoir (Q-T0-04) | mars–avril |
| **R10** | Activation des comptes : envoi par lots des liens de première connexion, suivi de l'enrôlement des passkeys | socle 1, R3 | à chaque bascule |
| **R11** | Modules suivants (internat, stages, certificats, fiches de dialogue, inscriptions, espace documents…) | socles correspondants | été 2027 |

Chaque lot : tests sur données fictives (comptages, totaux, idempotence : deux exécutions = même résultat ; annulation d'une exécution), aucune donnée réelle dans le dépôt.

---

## 11. À faire valider par Florian

| N° | Question | Proposition du pilote |
|---|---|---|
| Q-T0-01 | Les tables `messaging_*` et `portes_ouvertes_*` (créées par scripts) existent-elles en production ? | à voir sur le premier export (l'outil le dira) |
| Q-T0-02 | **Classement des modules** (colonne « Avis » du §3) | à corriger d'après l'usage réel, en priorité M03, M15, M18, M20, C07, P06, P07 |
| Q-T0-03 | Un seul établissement dans la base Next.js ? Combien de sites ? | [H] un établissement, plusieurs sites |
| Q-T0-04 | Les **demandes** n'ont pas de socle dans la carte : en créer un (« demandes et assistance ») ? | oui, à concevoir avant février |
| Q-T0-05 | Où placer le **registre SST** et les **HSE** dans la carte ? | SST : socle 6 ou un socle « santé et sécurité au travail » ; HSE : socle 49 |
| Q-T0-06 | Next.js facture-t-il réellement (cantine, voyages) via `facture` ? | si oui : archive comptable + soldes ouverts |
| Q-T0-07 | Les absences d'élèves saisies par l'**accueil** doivent-elles suivre, ou Charlemagne suffit-il ? | perdable (Charlemagne officiel) |
| Q-T0-08 | Durée de conservation en archive : messagerie interne, journaux de sécurité, appels d'internat, réservations passées | à fixer avec le DPO |
| Q-T0-09 | Mots de passe : accord pour **ne pas** reprendre les empreintes (lien de première connexion + passkey) ? | oui (§6.7) |
| Q-T0-10 | Photos d'élèves de Next.js : à reprendre si le cœur n'en a pas ? | oui, après accord de l'établissement |
| Q-T0-11 | Signalements **bien-être** : de quoi s'agit-il exactement, et qui doit y accéder dans ScolIA ? | socle 36, accès très restreint |
| Q-T0-12 | Forme de l'export S3 : copie complète du bucket, ou liste puis copie module par module ? | liste d'abord (taille, empreinte), puis copie par module |
| Q-T0-14 | **Maturité** (§3.0) : confirmer Fiable / Partiel / Ébauche par module ; en particulier les **absences du personnel** (fiables ou partie des « RH ébauche » ?), la **facturation des familles**, les **stages**, l'**internat**, les **fiches de dialogue** et les **RDV d'inscription** | voir §3.0 |
| Q-T0-15 | Export S3 : quels préfixes peut-on exporter (liste blanche), et confirmer l'exclusion des secrets du registre ? | liste blanche par module basculé |
| Q-T0-13 | Liens déjà distribués (certificats, suivi de voyage) : redirection des anciennes adresses vers ScolIA au moment de la bascule ? | oui pour les certificats |

## 12. Faits, hypothèses, contradictions

**Faits [F]** : JSON encore lus directement sur S3 par six chemins (§2.4) ; une vingtaine de colonnes JSONB métier ; schéma final de 186 tables déclarées (184 vivantes attendues) ; 78 migrations, 176 créations, 2 suppressions, 1 renommage ; 10 tables hors migrations ; stockage JSON redirigé vers `ent_collection_*` ; fichiers en S3 par établissement ; IBAN en clair dans `foyer_facturation` ; 17 tables jamais utilisées ; better-auth 1.7.2 avec TOTP et passkeys sur `scolia.fr` ; ScolIA en Argon2id ; `ent_absence_rh` bien plus pauvre que `absence` de Next.js.
**Hypothèses [H]** : empreintes scrypt (format par défaut de better-auth) ; un seul établissement ; usage réel de chaque module (le pilote ne voit pas les données) ; conservation longue du registre SST et des mandats SEPA ; faisabilité de la conversion des passkeys.
**Contradictions et écarts relevés**
- `TRANSITION_NEXT_RUST.md` §1 annonce « environ 178 tables » : le décompte exact est 176 créations dans `drizzle/`, 184 tables vivantes attendues avec les scripts ; à corriger dans ce document à la prochaine mise à jour.
- `MATRICE_CHARLEMAGNE.md` §4 range « Voyages, stages, internat, santé » ensemble : leurs régimes de reprise diffèrent fortement (santé bloquée par HDS, internat à double génération).
- `TRANSITION_NEXT_RUST.md` §3 cite les « demandes » parmi les gros modules de T3 alors qu'aucun socle ne les porte (Q-T0-04).
- Les tables RH filles (`personnel_document`, `personnel_formation`…) sont déclarées mais inutilisées : les données RH sont dans `personnel_attr`. La conception du socle 6 (§2.4) cite les « tables RH filles » : la reprise doit lire `personnel_attr`, pas ces tables.
