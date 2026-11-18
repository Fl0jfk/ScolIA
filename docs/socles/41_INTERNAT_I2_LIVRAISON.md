# Socle 41 « Internat » — lot I2 « La nuit » : livraison

Branche `ent-socle-41-code-i2` (base `origin/ENT` bcdc2e02). Conception : `docs/socles/41_INTERNAT.md` §9, §17, §20, §21, §22, §24, §25. Niveau : **implémenté, testé en local** (CI GitHub à venir). Ni validé, ni prêt pilote.

**Environnement des tests** : PostgreSQL 16.15 local **démarré** (`pg_isready` : accepting connections), `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, base neuve recréée avant la série finale, Rust **1.99.0**. Aucun test ne dépend de l'heure réelle : chaque commande `*_a` reçoit son instant, séjours et nuits sont posés à des dates fixes.

## 1. Livré

- **Attendu calculé** (`ent_internat_attendu_calcule`, fonction SQL unique) : séjours dont le rythme contient le jour de la nuit + nuits exceptionnelles, moins les retraits. Sources des retraits : retraits posés (table `ent_internat_retrait`, adaptateur des socles 13, 34, 39, I3), **situations connues du socle 12** (`ent_asd_situation` : déclaration, sortie, mobilité, PFMP couvrant 22 h heure murale), **mobilités** du socle 2, **séjours scolaires** du socle 42, **fermeture exceptionnelle** de l'internat du socle 5 (sauf internes présents). Les hébergés hors rythme sont listés « non attendu » (pour « non attendu présent »).
- **Appels** : sept types (S-4101, un type peut être désactivé), périmètres internat / bâtiment / zone / **liste** (groupe de chambres confié), liste **figée** à l'ouverture, pointage en ajout seul, « tous présents » par chambre, valeur courante = fait le plus récent, **conflit signalé** (CIT-03), clôture (R-IT16), validation avec avis U1 / U2 / U3 (R-IT34), **correction après validation** motivée avec avis de correction (R-IT17), historique, **PDF côté serveur** (aucun binaire stocké), appel a posteriori tracé (CIT-48).
- **Cas et procédure** : absent non couvert et retour tardif ouvrent un cas ; procédure par défaut du §9.3 (5 étapes, minuteur par échéance), **versionnée** (un cas garde sa version) ; étapes et vérifications de lieu horodatées et signées, en ajout seul ; issue obligatoire pour fermer ; un élève pointé présent ferme son cas ; numéros des titulaires (socle 4) : liens actifs avec autorité parentale, téléphones seulement, **jamais d'adresse**.
- **Transmission du matin** (une version par nuit, idempotente ; rectificative versionnée avec motif si une correction change les chiffres, voir `41_I2_CORRECTIFS.md`) : événements `internat.absence_nuit` (vers le 12), `internat.transmission`, `internat.effectif_repas` (vers le 40, compteurs sans nom) ; port `effectif_repas_a`.
- **Rappel** de l'appel du soir non validé : surveillant à l'heure limite (S-4102), CPE 15 minutes plus tard, une fois par niveau, suit l'attendu réel (aucun rappel si personne n'est attendu) ; tâche planifiée chaque minute (`internat_rappels_tache`, branchée dans `main.rs`).
- **Droits** : `internat.lire`, `internat.appel`, `internat.appel.valider` (portées établissement et **désignés** : une direction de formation ne lit, n'ouvre, ne clôt ni ne valide que ses élèves, R-IT30 ; défaut : administration, direction, CPE ; aucun droit pour professeur, infirmerie, psychologue, comptabilité, familles). L'infirmerie ne peut poser qu'un retrait « santé », sans détail.
- **Sécurité** : RLS sur les 13 nouvelles tables ; élève protégé (socle 2, E7) jamais nommé à un tiers (ligne sans nom, retrouvée par son lit ; consultation tracée) ; journal des lectures sensibles `ent_internat_lecture_sensible` (appel, attendu, cas, PDF), lisible par `audit.lire` ; aucune donnée de santé (mots de santé refusés dans notes et cas ; marque « à l'infirmerie » = renvoi au socle 34) ; événements neutres (testé : aucun nom, prénom ni texte libre dans les charges).
- **API** : 26 routes `/internat/…` (attendu, effectif, transmission, nuits exceptionnelles, retraits, listes, appels, pointages, clôture, validation, PDF, cas, procédure, journal) ; aucune sans session (testé).

## 2. Migrations

Plage 2824–2825 : **2824** `internat_i2_la_nuit` consommée ; **2825 non consommée** (les absences et sorties de la conception sont au lot I3). `MIGRATION_TAG` inchangé (plus haute = 3808). La fonction de garde de 2820 est remplacée par `CREATE OR REPLACE` ; aucun fichier versé n'est modifié. Inscrite dans `MIGRATIONS.md`.

## 3. Compteurs (règle 28)

RLS `tests_rls.rs` : base ENT + **13**. Registre des données : +13 tables classées. `TABLES_ELEVE` (fusion) : +5 (`appel_attendu`, `cas`, `lecture_sensible`, `nuit_exceptionnelle`, `retrait`). `Action::TOUTES` : 271 → **274**, les trois actions placées avant `FacturationGuichet` (après le catalogue historique repéré par position). Événements : +14.

## 4. Tests

- `ent-db` `tests_internat_i2` : **18 verts** (attendu, fermeture du calendrier, nuits à cheval sur minuit, changements d'heure de mars et octobre 2026, établissement de La Réunion, cycle complet avec PDF, concurrence, un seul appel par périmètre, R-IT14 service et base, conflit, retard et malade, cas, procédure versionnée, transmission, rappels, droits, périmètre R-IT30, isolation entre établissements, élève protégé, journal).
- Tests d'`ent-db` touchés ou partagés rejoués : RLS, registre, conservation, RGPD, rôles, fusion, `tests_internat` (I0/I1) ; `ent-auth` 64 ; `domain-eleve` 301 ; gardes `jour_utc_interdit` et `isolation_sql_statique` ; `ent-api` `tests_internat` 3 (dont un nouveau parcours HTTP).
- `cargo fmt -p ent-types --check` et fmt du frontend : propres. Clippy 1.99 `-D warnings` : les trois commandes de la CI propres (workspace ; workspace avec `ent-api/labo,ent-lab/labo` ; `-p ent-types`).

## 5. Écarts et questions

1. **Astreinte (socle 6) non branchée** : le cas affiche `astreinte_branchee = false` ; le numéro du CPE d'astreinte reste à lire du socle 6.
2. **Socle 12** n'a pas d'« absence de nuit » propre : l'attendu lit ses situations (24 h) et une table de retraits (source `vie_scolaire`). Question : le 12 posera-t-il lui-même des retraits ou ajoutera-t-il une nature « nuit » ?
3. **Stage (39), exclusion d'internat (13), hospitalisation (34)** : pas de lecture directe (pas de drapeau « dort ailleurs » ni de nature « exclusion d'internat » dans leurs tables) ; ils posent un retrait via l'adaptateur. À brancher quand ces socles exposeront leurs ports.
4. **Restauration (40)** : effectif émis par événement neutre, sans consommateur ; le dîner suit l'attendu, le petit-déjeuner retranche absents et partis.
5. **Périmètre « bâtiments confiés »** d'un surveillant : faute de poste de service du socle 6, la portée vient des élèves désignés ; les listes portent `confie_a_user_id` sans effet de droit pour l'instant.
6. **Avis aux directions et au CPE** : seuls les événements (`internat.appel_valide` avec U1/U2/U3) sont émis ; la résolution des destinataires par formation (INT-256) reste à écrire avec le socle 8.
7. Noms d'élèves : lus dans `ent_eleve.nom/prenom` (projection de l'identité) ; le nom d'usage est supposé projeté par le socle 2.
8. `ent_internat_lecture_sensible`, prévue en 2828 par la conception, est créée ici.
9. Pas d'écran (la conception ne met pas d'écran dans I2) ; le calcul de la parité NI-17 à NI-25 est couvert côté API.
10. Hors ligne (R-IT15) : lot I10 ; les faits portent déjà `a` (heure du geste) et `appareil_id`.

### Limites connues bloquant le « prêt pilote » (revue de sécurité du pilote, M4 et M5, hors lot de correctifs)

- **M4 — destinataires réels.** Les avis (`internat.appel_valide` U1/U2/U3, `internat.appel_en_retard`, ouverture et clôture de cas) sont émis **sans destinataire résolu** : ni le CPE d'astreinte (socle 6, non branché), ni la direction de la formation concernée (INT-256, socle 8) ne sont déterminés. Tant que cette résolution n'existe pas, personne n'est prévenu automatiquement d'un cas ouvert la nuit : la nuit repose sur la lecture active des écrans.
- **M5 — périmètre par aile.** La portée d'un surveillant vient des **élèves désignés** (portée « désignés », R-IT30), jamais d'une aile, d'un bâtiment ou d'une liste confiée : `confie_a_user_id` d'une liste n'a aucun effet de droit. Un surveillant « de l'aile B » ne peut donc pas être restreint à son aile par les droits ; il faut lui désigner les élèves un par un (ou le laisser à portée établissement).
- Tant que M4 et M5 ne sont pas traités, l'internat I2 ne peut pas être déclaré **prêt pilote**.

## 6. Qualité

0 nouvel avertissement (rustc et clippy, tests et labo compris) dans les lignes ajoutées. `origin/ENT` n'avait pas avancé (bcdc2e02) : fusion vide, compteurs inchangés. Non rejoué en local : la suite complète d'`ent-db` (≈ 3 h), la RLS stricte et le frontend, laissés à la CI de la PR ; `ent-rust/web/Cargo.lock` non touché.
