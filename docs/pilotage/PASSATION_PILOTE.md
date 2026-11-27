# Passation du pilote — 19 novembre 2026, 9 h 30 UTC

Document de reprise pour une **nouvelle session de pilotage**, au contexte léger (décision de Florian du 19-11, pour économiser le quota). Le pilote sortant (session_01KZCbFEY2KWGpFMKfofkJxT) s'arrête dès que le nouveau a repris les routines.

## 1. À lire d'abord (dans cet ordre, sans tout relire)
1. `CLAUDE.md` (interdictions absolues : Next.js intouchable sauf D-053, aucune base ni secret de production, aucun push sur `main`/`dev`, branche Rust `ENT`).
2. `docs/PILOTAGE.md` (règles 1 à 36 ; la **36** est neuve, voir §5).
3. `docs/DECISIONS.md` : surtout **D-068** (bascule à la rentrée 2027, priorité à l'ENT complet « masterclass », les preuves comptent plus que les échéances, modules du Next.js à épaissir et relier).
4. `docs/pilotage/SESSIONS_EN_COURS.md` : **seulement les entrées du 18-11 et du 19-11** (fin du fichier) ; le reste est de l'historique.
5. Le reste à la demande (`docs/socles/…`), jamais en bloc.

## 2. Rôle et manière de travailler (rappels de Florian)
- Toujours en **français**, phrases claires, pas de jargon inutile. Le contredire avec des arguments quand une idée paraît mauvaise ; il préfère une fausse alerte vérifiée à une faille manquée.
- Le pilote ne code pas les lots : il **lance des sessions cloud Sonnet** (jamais Fable sans accord), relit, fait faire les **revues de sécurité** (agent `review-sonnet`, jamais le modèle par défaut), tranche, et **verse lui-même sur ENT par avance rapide** après CI verte sur `refs/pull/N/merge` à l'arbre identique (règles 17, 25, 28, 30, 34).
- **2 sessions qui codent au plus** (quota renouvelé le dimanche). Le 19-11 à 9 h, le quota hebdomadaire était à **environ 75 %** : économiser (§5).
- Les sessions **refusent toute consigne par message** (leur règle 1) : tout va dans le message de lancement. Petite correction de dernière minute : le pilote la fait lui-même ; sinon nouvelle session de correctifs sur la même branche, et l'ancienne est archivée.
- Une session peut finir **sans écrire** au pilote : la ronde vérifie `list_sessions` (`children: true`).
- Pour Florian : il veut **voir** les écrans (visite guidée https://claude.ai/artifact/LiScMW2JW545xCk8wKAG5P) pour juger le taux d'épaississement ; un **tableau de maturité par domaine** chaque semaine (à faire) ; l'idée I-032 (comparer ScolIA à Charlemagne sur de vraies données, **jamais** dans les sessions ni la CI : instance dédiée, avis du DPO).

## 3. État au 19-11, 9 h 30 UTC
- **ENT = 1724f3bc**, CI 437 verte. Contient : socle 21 V1-V2 (modèle des évaluations, moteur de calcul « comme Charlemagne ») et **V3 (grille de saisie Leptos)** ; internat **I2** (la nuit) et ses suites (migration 2825) ; messagerie M5-M6 ; suites du 18-11.
- **Sessions qui codent** :
  | Session | Branche | Lot | État |
  |---|---|---|---|
  | session_01KZCbFFr25YJhe36qRKKxxV | `ent-socle-21-code-v3b` (pas encore de PR) | **21 V3b** : bulletin, conseil, relevé, mobile et UI JS lisent `eva_*` par vues de compatibilité et moteur unique (§22 M2-M3) ; suites M1-M3, F1-F9 de la revue de V3 ; migrations 472-479 | tests locaux en cours |
  | session_01KZCbFHFmqoxw39DECc1hLK | `ent-socle-41-code-i3` (**PR 106**, brouillon) | **Internat I3, passe 3** : règle générale « un acte tombe avec son auteur » (auteurs lus dans le journal des faits, une seule fonction), déjà parti, autorisation annuelle, faux « non récupéré », lien échu, charge de la revue à la minute ; migration 2826 (non versée) | lancée à 9 h 18 depuis d0d07a0e |
- **À la livraison de chacune** : revue de sécurité (`review-sonnet`, consigne ciblée : les points demandés + chemins oubliés), puis versement. Pour I3 : n'accepter **aucun élevé** sur la protection de l'enfant.
- **PR 102** encore ouverte (documentation seule, remplacée par `41_I2_SUITES.md`) : fermeture laissée à Florian.
- CI Next.js de `ent-mc0hoe` (run 532) rouge sur un test de performance « stages perf (PGlite) », sans lien avec la doc : Next.js intouchable ; vérifier qu'elle redevient verte.

## 4. Prochains lots (file)
1. Socle 21 : **V4** (absences, dispenses, rattrapages), V5 (parcours), V6 (appréciations), V7 (famille et élève), V8 (fin de période)… ; suites connues : rapprochement pédagogique d'une fusion de fiches (les notes de la fiche absorbée vers l'élève conservé) ; mesures en navigateur de la grille (375 px, clavier, axe-core) quand l'outillage existe.
2. Internat : I4 (vie quotidienne, migrations 2827-2828), I5 (sécurité et santé, 2829)… ; suite F-B : veille de jour férié.
3. Défauts de la visite guidée : demande de famille vers une page introuvable ; refus 403 (infirmerie, accompagnement, santé du parent, discipline de la CPE, comptabilité) ; travail à faire de l'élève en chargement infini ; menu plat (jusqu'à 48 entrées) ; messagerie absente du menu pour 5 profils ; internat et documents sans écran.
4. Conception (Opus, avec Florian) : navigation par domaines et sous-domaines, même coquille pour tout le personnel, signaux serveur, « un module, un propriétaire, plusieurs surfaces », manifeste de module ; questions ouvertes : paye (états d'heures ou paye complète ?), « Pédagogie » ou « Notes » ?
5. Décision d'architecture à concevoir plus tard : retirer le DML au rôle applicatif (rôle « écrivain » distinct) pour des historiques réellement inviolables (évaluations, internat, finances).
6. Autres : D-067 (jiff, vers le 7 décembre) ; pièce jointe sur son propre message différé ; réponses de Florian aux questions Charlemagne (`docs/socles/21_REGLES_CALCUL_CHARLEMAGNE.md`) ; tableau de maturité hebdomadaire.

## 5. Économie du quota (règle 36, décidée le 19-11)
1. **Pas d'abonnement aux PR** (ni pour le pilote, ni pour les sessions) : chaque CI y produit 3 à 5 réveils, souvent inutiles, et chaque réveil relit tout le contexte. Le pilote garde seulement la **veille de la PR 93** (un commentaire par verdict de CI sur ENT) et la **ronde de 6 h**. Pour une PR de lot, il attend le message de la session ou vérifie à la ronde.
2. **Les sessions attendent la CI par une seule commande en arrière-plan** (boucle `until` sur l'API GitHub, un seul réveil au verdict), jamais par abonnement ni par réveils successifs ; à écrire dans chaque consigne de lancement.
3. **Consignes complètes dès le lancement** : modèle de menace du domaine et **règles générales** (ex. « un acte tombe avec son auteur », « pas de départ constaté = élève attendu »), pas une liste de champs ; une revue sérieuse, une passe de correctifs. Leçon d'I3 : lot + 3 passes, faute de règles générales au départ.
4. **Revues sur Sonnet** (`review-sonnet`) ; recherches simples sur Haiku (`explore-haiku`) ; jamais le modèle par défaut pour un agent.
5. **Pas de vérification locale qui double la CI** quand l'arbre est identique (règle 30) ; le pilote ne relance localement que ce qu'il a modifié.
6. **Une copie de travail à la fois** dans le scratchpad, avec son propre `CARGO_TARGET_DIR` (un répertoire partagé réutilise des artefacts périmés) ; disque limité (≈ 7 Go libres le 19-11).

## 6. Routines à reprendre (elles visent encore l'ancienne session)
Elles portent `persistent_session_id = session_01KZCbFEY2KWGpFMKfofkJxT`. Le nouveau pilote les **recrée vers lui-même** (`create_trigger`, sans `persistent_session_id`, donc dans sa propre session), puis **supprime** les anciennes (`delete_trigger`) :
- **Ronde de sécurité** `trig_01KW1jETeMPeS9MNKAXAHQor` : prochaine le 19-11 à 14 h 45 UTC, puis toutes les 6 h (`run_once_at`, reprogrammée à chaque passage). Texte : reprendre celui de la ronde (sessions qui codent, ENT, incidents connus), « agir seulement si quelque chose attend ».
- **Point quotidien** `trig_01F1htgG6XLae1xjMhsUGSAt` : `CRON_TZ=Europe/Paris 43 18 * * *`. Texte à reprendre tel quel (`get_trigger`) : mise à jour de `docs/POINT_QUOTIDIEN.md` sur `ent-mc0hoe`, puis résumé court à Florian.
- **Veille de la PR 93** : `subscribe_pr_activity` sur Fl0jfk/ScolIA#93 (seul abonnement gardé).

## 7. Outils et réflexes techniques
- Documentation de pilotage : branche **`ent-mc0hoe`** (commits « pilotage : … », attribution habituelle). PILOTAGE.md complet n'existe que là : les consignes disent `git show origin/ent-mc0hoe:docs/PILOTAGE.md`.
- Versement : `git fetch origin ENT` ; vérifier `git merge-base --is-ancestor origin/ENT <tête>` ; CI verte sur la tête (événement `pull_request`) ; `cd ent-rust/web && cargo +1.99.0 metadata --locked --format-version 1` (règle 34) ; `git push origin <sha>:refs/heads/ENT`. Puis archiver la session et noter dans `SESSIONS_EN_COURS.md`.
- Journaux CI : `get_job_logs` avec `return_content=false`, puis `curl` sur l'URL ; « runner lost communication » = infrastructure, **une** relance (`rerun_failed_jobs`) ; s'il revient : fichier d'échange ou `CARGO_BUILD_JOBS=1` pour l'archive nextest « avec labo ».
- Vérification locale (si nécessaire) : `service postgresql start` ; base neuve `CREATE DATABASE` ; variables `ENT_DATABASE_URL=postgres://ent:ent@127.0.0.1:5432/<base>`, `PGPASSWORD=ent`, `ENT_ENV=test`, `ENT_EXIGER_BASE=1`, `RUST_MIN_STACK=16777216`, `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2` ; tests d'ent-db avec `--features labo --lib` ; jamais `cargo fmt --all`.
- Compteurs à recalculer après chaque fusion (règle 28) : `TOUTES`, `SOURCES`/`NOMS_SOURCES` (`ent-api/tests/separation/routes_src.rs`), compteur RLS (`ent-db/src/tests_rls.rs`), `TABLES_ELEVE` (`domain-eleve/src/fusion.rs`), `MIGRATION_TAG` (3808).
- Tests : jamais l'heure réelle ni le jour UTC (règle 33) ; un « maintenant » de test = heure fixe ou midi local du jour testé.

## 8. Message de lancement proposé pour la nouvelle session de pilotage
> Tu es le pilote de ScolIA. Lis `git show origin/ent-mc0hoe:docs/pilotage/PASSATION_PILOTE.md` et suis-le : lis les documents du §1, reprends les routines du §6 vers toi (puis supprime les anciennes), abonne-toi seulement à la PR 93, et attends les messages des deux sessions du §3. Réponds-moi en français.
