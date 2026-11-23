# Revue des suites M12 (PR 123, tête 96b73e09) — revue indépendante du pilote

Numéros de ligne : ceux de la tête revue (96b73e09) ; les relire sur la base actuelle.

## ÉLEVÉ

**E1. [V par lecture, non exécuté] Corriger puis annuler un différé échoue, et le message part quand même (défaut préexistant, non corrigé par le lot).**
- La table `ent_msg_version` a une clé étrangère vers `ent_msg` sans `ON DELETE` (migrations/482_msg_m1_participants.sql:114) et un déclencheur BEFORE DELETE en ajout seul (:125).
- `annuler_message_differe` (ent-db/src/msg_repo.rs:1382-1444) supprime cartes, accès, lectures, pièces et accusés, puis `DELETE FROM ent_msg`. Elle ne touche jamais aux versions. La suppression viole donc la clé étrangère (23503), qui devient `ReferenceInconnue` via `map_sqlx`.
- L'exception de la migration 490 (`ent.msg_annulation`) ne couvre pas `ent_msg_version`.
- Conséquences :
  - Pas de perte de données. Tout est dans une seule transaction qui est annulée. Les objets de pièces ne sont détruits qu'après le succès, et l'avis planifié (service.rs:18775) n'est pas annulé.
  - Mais le message reste planifié et sera remis, avec le texte corrigé, alors que l'auteur voulait l'annuler. C'est le pire sens possible : l'auteur a corrigé précisément parce qu'il hésitait.
- Contournement existant : `retirer_message_for_actor` accepte un différé (`maintenant - effectif` est négatif, msg_m4.rs:677). Il le masque avant la remise.
- Correctif : étendre l'exception bornée (déclencheur de la migration 490) à `ent_msg_version`, supprimer les versions dans la même transaction avant `ent_msg`, et ajouter un test « corriger puis annuler ». Il faut aussi que l'erreur ne soit pas un code inexploitable côté API.

## MOYEN

**M1. [V] La borne du curseur repose sur `xact_start` de `pg_stat_activity` et non sur `pg_snapshot_xmin`.** msg_repo.rs:870-875 (patch 870) et :1082-1090.
- Exactitude (point 1, votre question précise) :
  - Pour tout `INSERT`, `created_at = clock_timestamp()` est ≥ `xact_start` de sa transaction. Toute transaction encore ouverte à l'instant de lecture est donc ≥ borne.
  - Une transaction validée après une autre plus récente est bien vue : `evenements_apres` renvoie tout jusqu'à `now()`, et le curseur rendu est `borne`, donc le message tardif est relu au tour suivant.
  - En cas de troncature, le second passage borné par `borne` (patch 988-999) est correct.
  - Le seul chemin d'écriture dans `ent_msg` est msg_repo.rs:815.
- Limites réelles :
  - Une transaction plus vieille que 540 s (fenêtre − réserve de 60 s) est plafonnée par `borne_sure`. Un message inséré tôt dans cette transaction, puis validé après le passage du curseur, est sauté sans signal (documenté mais silencieux). Avec `pg_snapshot_xmin` ou une séquence, il n'y aurait ni plafond ni saut.
  - Cela repose sur l'horloge murale de la base, donc sensible à un recul d'horloge (NTP). [H]
  - Les transactions préparées (2PC) sont invisibles dans `pg_stat_activity`. [H]
- Correctif recommandé : colonne `xid8 DEFAULT pg_current_xact_id()` sur `ent_msg` (et pour `retire_le`), curseur = `pg_snapshot_xmin(pg_current_snapshot())`. Cela supprime la dépendance aux privilèges, l'horloge murale, le plafond et la fuite F1.

**M2. [V] Le test « transaction longue » ne teste pas une transaction longue.** tests_messagerie_revue.rs, `revue_ma_curseur_progresse…` (patch 1422-1445).
- La transaction ouverte vient d'être créée (`xact_start ≈ now`). La borne vaut donc `now − 5 s` et `curseur > now − 300` est trivialement vrai.
- Seule la fonction pure `borne_sure` (test 1404) couvre le plafonnement. Aucun test d'intégration n'a de transaction plus vieille que le plancher, ni de transaction ouverte puis écriture validée tard avec troncature combinée.
- `revue_m3` affirme `ancienne <= debut_tx`. Sur un labo partagé, cela se vérifie déjà avec n'importe quelle autre transaction.
- Piste : injecter la borne ou le « maintenant » dans `evenements_depuis`, comme `entrer_a` le fait pour la limite, ou utiliser `xid`.

**M3. [V] Répétition permanente des événements et boucle serrée de l'interrogation longue.**
- Le curseur rendu est `borne` (≤ now − 5 s) alors que les événements rendus vont jusqu'à `now()` (non tronqué, msg_repo.rs:1005 et patch 1004).
- Après un événement, chaque rappel renvoie donc le même événement immédiatement, pendant au moins 5 s.
- Avec une transaction ancienne de plus de 9 min, c'est pendant 9 min. Chaque appareil fait alors un cycle à 1 req/s, sans jamais attendre, car `fils` est non vide (msg_m12.rs:113).
- Le client doit dédoublonner par (fil, instant, genre). Ce n'est ni documenté côté serveur ni testé. Il y a un coût base en continu.
- Correctif : dans la boucle d'attente, ne considérer comme « nouveau » que ce qui est strictement après la dernière position servie, ou renvoyer aussi `max(instant)` rendu comme position de dédoublonnage.

**M4. [V] Tests faibles ou non probants pour F-6 et pour le temps réel.**
- `revue_f6` (patch 1616-1662) :
  - Les 200 fils sont tous dans la même famille et le même élève (`fil_de_lily`). L'équivalence lot/unitaire ne couvre donc ni plusieurs élèves, ni élève protégé, ni fil de boîte, ni fil réservé, ni parent sans lien communicant. Or les branches boîte, protégé et périmètre sont précisément celles réécrites.
  - Le test affirme `duree_lot < duree_boucle` et `< 2 s` : preuve par horloge réelle, donc sujette à scintillement (contraire au point 8). Le sans-N+1 devrait se prouver en comptant les requêtes.
- Test HTTP (tests_flux_m12.rs) :
  - `long` attend réellement `attente=3` (3 s d'horloge).
  - `limite_flux().vider()` vide l'état global du processus pendant que d'autres tests peuvent s'en servir en parallèle. [H : fragile]
  - Aucun test HTTP n'assert l'en-tête `Retry-After` (seulement `retry_after_secondes() >= 1` en unitaire).
  - Aucun test HTTP « rappel après `tronque` non refusé ».
  - Aucun test que le plafond de 8 n'écrase pas une pagination (voir F2).

## FAIBLE

**F1. [V] Fuite inter-établissements par canal auxiliaire (borne).** `pg_stat_activity` n'est pas cloisonné par établissement. Le `curseur` renvoyé au client vaut `min(xact_start de toute transaction du cluster, now − 5 s)`. Il révèle donc à un établissement l'instant de début de transactions d'un autre, ou de l'autovacuum. Contenu : un instant seulement, jamais de données ni de requête. Un correctif en xid8 (M1) élimine le problème.

**F2. [V] « Pagination jamais refusée » n'est pas absolue.** flux_limite.rs:197 (patch) : le plafond par compte (8) est testé avant `rappel_libre`. Un compte avec 8 appels longs en cours refuse même un rappel de pagination (429 `Concurrent`). Il faut soit exempter les rappels libres du plafond, soit documenter la limite.

**F3. [V] Exception de garde trop large.** garde_rls_stricte.rs:58 (patch 633) :
- L'exception est identifiée par fichier (`msg_repo`) et par sous-chaîne `.fetch_one(&self.pool)`. Tout futur accès direct de ce genre dans `msg_repo.rs` (fichier contenant des données d'élèves) passerait silencieusement, y compris une requête sur une table d'établissement.
- Elle sert deux appels : `verifier_visibilite_transactions` et la sonde de test `#[cfg(test)]`, que la garde (qui lit le texte) compte aussi.
- Motivation réelle : le catalogue lu ne contient aucune table d'établissement (pg_roles, pg_stat_activity). Le contexte d'établissement est donc réellement inutile. L'exception est justifiée dans son principe mais mal bornée.
- Correctif : isoler les deux requêtes dans un module/fichier dédié (ex. `catalogue_pg.rs`) avec un extrait exact et unique, ou une fonction nommée.
- Ce que fait réellement `maintenant_base` : il passe par `TenantTx` (msg_m4.rs:523), donc il n'a pas besoin d'exception. L'exception ne concerne que le catalogue.

**F4. [V/H] Le contrôle `pg_read_all_stats` est trop strict.**
- msg_repo.rs:1023-1047 : un rôle voit toujours `xact_start` des sessions du même rôle (c'est le cas de l'application, qui utilise un seul rôle). `pg_read_all_stats` n'est donc nécessaire que pour voir les transactions d'autres rôles (migration, sauvegarde).
- Le journal d'erreur est un faux positif dans le cas courant. Le test `assert!(verifier_flux_messagerie().await)` dépend du rôle du labo. [H pour l'effet CI]

**F5. [V] Égalité de borne.** msg_repo.rs:985 (`*instant <= borne`). `retire_le = now()` (msg_m4.rs:692) est égal à `xact_start` de la transaction de retrait. Si cette transaction est la plus ancienne, sa borne vaut exactement son `instant`. Un curseur tronqué `(instant==borne, fil, genre)` plus grand que son tuple pourrait la sauter. C'est très étroit (deux transactions dans la même microseconde), mais le correctif est trivial : `<` strict.

**F6. [V] Ordre de contrôle dans `annuler_message_differe_for_actor` (service.rs:18740-18764).** Un message remis dans un fil non lisible par l'acteur renvoie `Forbidden` (non-auteur), alors qu'un identifiant inconnu renvoie `NotFound`. Cela distingue un message existant d'un inconnu, ce qui n'est pas le cas pour un différé (point 3 bien traité). `message_de_l_auteur` (msg_m4.rs:549-559) a le bon ordre (lisibilité du fil avant l'auteur). Il faut aligner. Impact limité : identifiants UUID.

**F7. [V] « Une seule horloge » : le périmètre demandé (10 et 15 min) est conforme, mais il reste des `Utc::now()` dans la messagerie.**
- msg_m4.rs:180 (expiration de lot), :230 (`confirmer_lot_a`), :485 (`repondre_programme_a`, plage calme/différé), msg_m6.rs:346 et :602 (disponibilité), msg_m3.rs:949, 971, 1806, 1933 (horaires de boîte).
- Le calcul de la remise du différé (:485) est comparé ensuite à `now()` de la base : un décalage d'horloge peut avancer ou retarder une remise.
- `maintenant_base` ouvre une transaction séparée juste avant (msg_m4.rs:523-525). Le décalage est de quelques ms, sans conséquence pour des délais de 10 et 15 min. Mais il coûte une connexion supplémentaire par appel.

## CONFORME (vérifié)
- **Point 1 :**
  - La fonction pure `borne_sure` ne dépasse jamais la fenêtre (borne ≥ now − 540 s < 600 s), donc pas de boucle de resynchronisation causée par une transaction longue.
  - L'ordre (instant, fil, genre) est total et cohérent entre WHERE et ORDER BY.
  - La troncature est bornée par la limite sûre.
  - Le plancher et le non-recul du curseur ne reculent pas.
- **Point 2 :**
  - La limite par session avec `session_id` unique est correcte.
  - `Retry-After` est présent sur 429 (routes.rs patch 408-413).
  - Le plafond par compte est décrémenté à la chute de la place (`Drop`), y compris sur annulation de requête.
  - `rappel_libre` est mémorisé côté serveur : aucun contournement par `apres_fil`.
  - Horloge injectée dans les tests unitaires de débit : pas de `sleep` pour ceux-là.
- **Point 3 :**
  - `message_differe_d_autrui` est appelé avant tout jugement d'auteur sur corriger/retirer (msg_m4.rs:545), annuler (service.rs:18746) et pièce (service.rs:19104).
  - Téléchargement, lien et ouverture de carte renvoient `introuvable` de façon uniforme (`piece_a_servir` msg_m5.rs:264 ; `cartes_du_fil_for_actor` filtre les différés).
  - `revue_mc` vérifie les trois routes avec un identifiant inconnu comme témoin, plus le témoin `Forbidden` sur un message remis.
- **Point 4 :** corriger et retirer utilisent la base pour `maintenant`, `created_at = clock_timestamp()` à l'insertion, `ORDER BY created_at, id` stable. Le test `revue_f5` place `created_at` par SQL (12 et 16 min), sans horloge applicative.
- **Point 5 :**
  - `actor_can_access_fils_lot` suit la même structure que le jugement unitaire (service.rs:17893).
  - `acces_personnel_fils_boite_lot` est équivalent à `acces_personnel_fil_boite` (même fonction SQL `ent_msg_fil_lisible`, même filtre `nature = 'boite'`).
  - Les protégés passent par la même fonction `proteges_caches`.
  - Le périmètre passe par `eleves_autorises`, dont le code est parallèle à `autoriser_eleve`. Écarts connus : la branche `est_saisie_enseignement` ne concerne pas `MessagerieEleve` ; les branches groupes et désignés utilisent une requête différente de `eleve_dans_groupes_de` / `eleve_dans_cibles` [H : équivalence non démontrée ni testée, voir M4].
  - Le rôle de session est transmis ; un rôle inconnu → ensemble vide.
  - Le lot n'est pas plus large que l'unitaire sur ce que j'ai pu vérifier : même `fils_lisibles_famille`/`fils_lisibles_personnel`, et `get_fils_lot` filtre par établissement avec `TenantTx`.
  - Côté famille, il reste un appel par élève distinct (`actor_can_access_msg_fil`), nombre faible (N+1 limité aux élèves distincts, pas aux fils).
- **Point 6 (cloisonnement) :** les requêtes d'exception ne lisent que `current_user`, `pg_roles` (+ `pg_has_role`) et `min(xact_start)` : aucune table d'établissement, aucune lecture de texte de requête. La fuite possible est limitée à un instant (F1).

Fichiers clés (copie m12r) :
- /tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/m12r/ent-rust/crates/ent-db/src/msg_repo.rs
- /tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/m12r/ent-rust/crates/ent-db/src/msg_m12.rs
- /tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/m12r/ent-rust/crates/ent-db/src/msg_m4.rs
- /tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/m12r/ent-rust/crates/ent-db/src/service.rs
- /tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/m12r/ent-rust/crates/ent-db/src/garde_rls_stricte.rs
- /tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/m12r/ent-rust/crates/ent-api/src/flux_limite.rs
- /tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/m12r/ent-rust/migrations/482_msg_m1_participants.sql
- /tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/m12r/ent-rust/migrations/490_msg_correctifs_revue.sql

