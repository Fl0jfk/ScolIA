# Tests dépendants du jour ou de la date (règle 33) — correctif du 21-11-2026

Branche `ent-tests-jour-semaine`, issue d'`origin/ENT` (d23a6f66). Aucune migration, aucun changement de
comportement de production.

## Constat (samedi 21-11, reproduit en local)

Avec ENT d23a6f66, huit tests échouaient le samedi. L'analyse montre **deux causes différentes**, dont une seule est
le jour de la semaine :

| Cause | Tests | Nature |
|---|---|---|
| **Jour sans cours** (week-end) | AS6 : `as6_relance_automatique_unique_a_debut_plus_delai`, `as6_aucune_relance_pour_une_seance_annulee_validee_ou_reprise` ; web-server : `as14e_seuil_proposition_decision_et_rectification` | Le moteur de vérification d'assiduité ne relance ni n'alerte un jour fermé au calendrier (socle 5) : **comportement voulu**. Les tests prenaient « aujourd'hui », donc un samedi. |
| **Fin du 1ᵉʳ trimestre du labo (20 décembre)** | évaluations : `periode_complete_de_la_grille_au_bulletin_et_a_l_export`, `coefficients_de_matiere_et_regles_de_calcul`, `e2e_v3b_une_note_saisie_puis_publiee_est_lue_partout_avec_la_meme_moyenne` (`tests_evaluations`), `les_lectures_historiques_utilisent_le_moteur_exact`, `une_absence_n_est_jamais_un_zero_et_le_rattrapage_reste_possible` (`tests_evaluations_v0`) | **Pas un jour de la semaine** : la période de notes de test valait `aujourd'hui − 10 … aujourd'hui + 30`. À partir du **21 novembre**, `+30 jours` dépasse le 20 décembre (fin du T1 du jeu de labo) ; la période de notes déborde alors sur deux périodes du référentiel, la période lue par le moteur n'est plus celle des devoirs du jour et toutes les moyennes valent `None`. Le vendredi 20-11 (jour de la dernière CI verte) tombait pile sur le 20 décembre : le test était vert **par hasard de calendrier**. Ils auraient échoué tous les jours du 21-11 au 20-12 (simulé : mardi 1ᵉʳ décembre rouge, mardi 10 novembre vert). |

Les cinq tests d'évaluations sont donc une **bombe à date**, pas à jour de semaine. `as6_eleve_exclu_de_cours_non_arrive_alerte_a_dix_minutes`
(cité « déjà connu ») **passe** en local le samedi : il avait déjà été corrigé (instant fixé à 10 h) ; il est de plus passé
au jour de référence `jour_ouvre()` et vérifié aux quatre dates simulées. Les contrôles « aucun test ne disparaît » et le
verdict ne sont que la conséquence de ces échecs.

## Corrections

### 1. Évaluations — `tests_evaluations.rs`, `tests_d022_bis.rs`
`fenetre_periode_note(annee_debut, annee_fin, jour)` remplace la fenêtre `−10/+30` : avant le 20 décembre elle finit au
plus tard le 20 (et commence au plus tôt le 2 septembre) ; après, elle commence au plus tôt le 21. Elle ne chevauche
donc **jamais** la frontière du T1 du seed. `tests_d022_bis` utilisait la même fenêtre (non rouge, mais exposé : même
correctif). Assertions inchangées.

Le 2 septembre (et non le 1ᵉʳ) : si le début de la période de test est le même jour que le T1 du seed, deux périodes du
même référentiel commencent le même jour et `ent_periode_de_note_pour_classe` les départage par `p.id` (UUID aléatoire)
→ test instable constaté (5 septembre simulé, un échec sur deux) puis corrigé. Seul le jour de la rentrée lui-même reste
exposé à cet aléa (voir « Restes »).

### 2. Assiduité AS6 — `tests_assiduite_as6.rs`
* `jour_ouvre()` : dernier **jour de cours** (calendrier du socle 5) à la date de l'établissement ou avant (vendredi un
  samedi/dimanche). Utilisé par les cinq scénarios qui appellent `verifier_jour_systeme` avec un instant explicite.
* `as6_aucune_relance_d_une_seance_non_assuree_ni_un_jour_ferme` : l'assertion conditionnelle
  (`i64::from(ouvert)`, vraie quel que soit le jour) devient **ferme** : jour de cours ⇒ relancé (1) ; et un **samedi**
  explicite ⇒ aucune relance (0). Le test éprouve maintenant la règle « jour sans cours » au lieu de la contourner.
* `as6_faire_l_appel_a_la_place…` : la règle « le jour même ou la veille » lisait l'horloge réelle ; avec `jour` =
  vendredi un samedi, `jour + 1` n'était plus « futur ». **Seul changement de code hors tests** :
  `appel_a_la_place_au(…, aujourdhui_force: Option<NaiveDate>)` (`assiduite_as6.rs`), corps de
  `appel_a_la_place_for_actor` qui garde le même ordre de contrôles ; la fonction publique passe `None` (horloge de
  l'établissement : **comportement identique**). Les tests passent `Some(jour)` et éprouvent en plus la règle : la veille
  n'est pas refusée comme invalide, deux jours avant l'est.

### 3. Web-server — `assiduite_as14de.rs` (as14e)
Le test prenait « les quatre derniers jours du mois précédent **de même jour de semaine qu'aujourd'hui** » : un samedi,
quatre samedis, jour sans cours, décompte vide. Le créneau 4B du seed de labo est lui-même posé au jour de semaine du
jour de lancement. Le test crée maintenant **son propre créneau 4B du mardi 15 h–16 h** (administration, idempotent) et
joue les quatre derniers mardis du mois précédent (`JOUR_DES_APPELS`). Fichier reformaté (il était propre).

## Preuve de stabilité

Horloge simulée avec `libfaketime` sur **le binaire de test ET sur le serveur PostgreSQL** (le `now()` SQL suit :
`PG now` consigné), base locale neuve `ent_ci_test`, `RUST_MIN_STACK=16777216`. Script : relancer le serveur sous
`LD_PRELOAD=libfaketime.so.1 FAKETIME='@<date>'`, puis le binaire sous `faketime -f '@<date>'`.

| Date simulée (UTC) | Jour Paris | AS6 (12 tests) + 5 évaluations | as14d + as14e |
|---|---|---|---|
| 2026-11-21 12:00 | samedi | 17/17 ✔ | 2/2 ✔ |
| 2026-11-22 12:00 | dimanche | 17/17 ✔ | 2/2 ✔ |
| 2026-11-23 12:00 | lundi | 17/17 ✔ | 2/2 ✔ |
| 2026-11-27 22:30 | vendredi 23 h 30 Paris = **samedi** | 17/17 ✔ | 2/2 ✔ |

Frontières de calendrier (évaluations + `tests_d022_bis`, 7 tests) : 19-12 sam ✔, 20-12 23 h 30 UTC dim ✔,
21-12 lun ✔, 25-12 ✔, 10-01-2027 ✔, 5-09-2026 (×2, après correctif du départage) ✔, 10-07-2027 ✔.
Avant correctif : 1-12 et 21-11 rouges, 10-11 et 20-11 verts (cause établie).
`rd06` (agenda, connu « rappel émis : 0 » la nuit) : 21-11 00 h 30, 21-11 23 h 30 et 22-11 03 h UTC ✔ — non
reproduit, laissé tel quel.

## Autres tests du dépôt (point 4)

Recensement (`tests_*.rs`, `crates/*/tests`, `web/crates/*/tests`) : `jour_paris()` 115 occurrences/61 fichiers,
`Utc::now()` 141/35, `today_utc()` 140/25, `aujourdhui_etablissement` 42/16, `ent_aujourdhui` 97/53, `CURRENT_DATE` 1,
`Local::now` 0, `weekday/number_from_monday/num_days_from_monday` ≈ 55. Les 2 456 tests ne peuvent pas tous être rejoués
à plusieurs dates (≈ 5 h par date) : **audit ciblé, pas exhaustif**.

* **Corrigés** : ci-dessus (fenêtre de période de notes ×2, AS6 ×5 + 1, as14e).
* **Déjà conformes à la règle 33 (relus)** : les ≈ 40 usages de `weekday` qui *calculent* explicitement un lundi/jour
  ouvré (EDT t0–t5, agenda ag1/ag3, structure s3/s7, `tests_assiduite_as0::jour_ouvre`, `tests_portail_p7` en 2099,
  `tests_vie_scolaire`, `recette_personas_1`, `edt_s2` web…). Un garde-fou existe déjà (`ent-db/tests/jour_utc_interdit.rs`).
* **Laissés, avec raison** : `contre_backend.rs:386` (assertion que `jour_neuf` a le jour de semaine d'aujourd'hui : vraie
  par construction) ; `tests_agenda_ag0::rd06` (non reproduit, voir ci-dessus) ; les usages `Utc::now()`/`ent_aujourdhui`
  qui ne servent qu'à horodater ou à produire une date relative sans assertion sur le calendrier (aucun échec le
  samedi en CI : la CI du 21-11 a exécuté la suite entière et n'en a rouge que huit).
* **Non audité en détail** : le reste des occurrences `today_utc() ± N jours` (≈ 40 lignes, dont `tests_eleve_scolarite`,
  `tests_structure_s5/s7`, `tests_personnels_p0`) : aucune n'a de borne de calendrier connue comme celle du T1.

## Défauts et fragilités trouvés dans le code (non corrigés ici, à décider)

1. **Le jeu de labo (`seed_labo_cast`) casse à la rentrée 2027** : simulé au 2027-09-01/05 et 2027-10-05, il échoue
   (`Domain(NotFound)`) ; il passe encore le 2027-08-20. Presque **tous** les tests à base de labo (la quasi-totalité)
   rougiraient le jour de la rentrée 2027 (dates en dur 2025-2026 / `entree = 2026-09-01` dans `seed.rs`). À corriger
   avant septembre 2027 ; hors périmètre (le seed n'est pas un test).
2. **Périodes qui se chevauchent dans une même périodisation** : `ent_periode_de_note_pour_classe` (ORDER BY recouvrement,
   début, `id`) et `eva_periode_eleve_jour` (sans ORDER BY) peuvent désigner deux périodes différentes si deux
   périodes de la même périodisation se recouvrent. Le test en produisait par construction ; en production, vérifier
   que l'invariant « pas de recouvrement » est garanti (contrainte), sinon les moyennes disparaissent en silence.
3. **« Le jour même ou la veille »** (`appel_a_la_place`) : calendaire, pas ouvré. Le lundi, la vie scolaire ne peut pas
   faire à la place l'appel du vendredi (et le dimanche, aucun jour de cours n'est éligible). Question de produit, non
   modifiée.

## Vérifications

`cargo +1.99.0 test --locked -p ent-db --features labo --lib` (filtres ci-dessus) et web `assiduite_as14de` (compilé
avec `CARGO_PROFILE_DEV_DEBUG=0` en local : le lien débogage dépasse 4 Go, sans effet sur la CI). Zéro avertissement
rustc ; `cargo +1.99.0 clippy --locked -p ent-db --tests --features labo` : aucun message dans les fichiers touchés. Fichiers backend non formatés avant le lot
(pas de `cargo fmt --all`), non reformatés ; le fichier web, propre avant, l'est resté (`rustfmt`).
