# Socle 21 — lot V3 « grille et saisie » : livraison

**Date :** 2026-11-19 — **Branche :** `ent-socle-21-code-v3` (depuis `origin/ENT` 7402da72, qui n'a pas avancé pendant le lot) — **PR en brouillon vers `ENT`** : voir la fin.
**Niveau :** *implémenté + testé en local* (PostgreSQL 16 local, mode normal) ; **CI GitHub : à voir sur la PR** ; **non validé par Florian ; non prêt pilote.**
**À lire avec :** `21_EVALUATIONS.md` (§8, §9, §10, §11, §14, §17, §18, §20, §23), `21_EVALUATIONS_V1_V2_LIVRAISON.md`, `MIGRATIONS.md` (mise à jour du 2026-11-19), `docs/FRONTEND_LEPTOS.md`.

## 1. Fait

### 1.1 La grille (écran Leptos, `/web/evaluations/grille`)
`ent-rust/web/crates/ent-web-app/src/modules/evaluations/grille_v3/` : `modele.rs` (logique pure, 15 tests natifs) et `mod.rs` (vues). Style : `web/style/grille-notes.css`. L'ancienne grille reste sur `/web/evaluations/grille-historique` (jusqu'à la bascule V12).

| Besoin | Réalisation |
|---|---|
| Grille élèves × évaluations au clavier | un champ par case ; flèches, **Entrée** (bas), **Maj+Entrée** (haut), **Tab**, **Échap** (annule la frappe) ; les flèches gauche/droite ne quittent la case qu'au bord du texte ; enregistrement **continu** (pause de frappe de 0,9 s, ou au changement de case) et bouton « Enregistrer maintenant » ; une seule requête en vol, la suite attend |
| Codes rapides | `abs`, `ar`, `disp`, `nr`, `nn` (et les mots entiers) ; analyseur **unique** (`ent_types::evaluations_v3::analyser_cellule`) rejoué par le serveur ; une case vide ne supprime rien (R-V6) |
| Barème, coefficient | visibles dans l'en-tête de chaque colonne, avec la date, l'état de publication et la moyenne de l'évaluation |
| Moyennes | **moteur unique de V2** (élève, classe, évaluation, explication ligne à ligne) ; **aucun calcul côté client** : après chaque enregistrement la grille est relue et seules les moyennes sont mises à jour |
| Collage d'un bloc | collage sur une case (`paste`) ou fenêtre « Coller un bloc » (accessible, utile sur téléphone) ; aperçu **avant** toute écriture, rapprochement par identifiant de ligne |
| Import d'un fichier du professeur | CSV / TSV / texte de tableur (séparateur détecté, guillemets, BOM, Excel français `;`) ; rapprochement par **nom dans n'importe quel ordre, sans accent ni casse**, puis par **rang** ; colonnes reconnues par le **titre** de l'évaluation ; **aperçu ligne à ligne** (rapproché, non rapproché, ambigu, hors public ; case nouvelle / modifiée / inchangée / erreur / verrouillée) ; confirmation par **empreinte** : si une note change entre l'aperçu et la confirmation, rien n'est écrit ; dans **ses** évaluations (droit de saisie sur le cours, M-447) |
| Conflits de version | chaque case porte sa **version** ; version périmée = **conflit** (jamais d'écrasement) ; le serveur renvoie l'état actuel (valeur, auteur) ; l'écran liste les conflits : « Garder ma saisie » (renvoie avec la version actuelle) ou « Prendre la valeur en base » |
| Évaluation depuis la grille | « Nouvelle évaluation » : titre, date, barème, coefficient, comptage (normal / si améliore / ne compte pas / bonus), points au-delà du barème, **sous-matière**, **public restreint** (élèves cochés), annoncée, publication ; en-tête de colonne → réglages, publication, retrait, annulation |
| Publication | maintenant ; **programmée** (heure de l'établissement, convertie par le serveur) ; à l'arrêt des notes ; **retrait** (professeur dans les 24 h, ensuite la direction, motif obligatoire, la famille verra « retiré pour vérification ») ; **« en cours de correction »** ; **délai maximal** d'établissement (§10.1) : publication automatique, sauf « en cours de correction », avec **un rappel** deux jours avant ; planificateur horaire (`passages_quotidiens`) |
| Historique d'une note corrigée | panneau « case sélectionnée » : qui, quand, avant, après, motif, source, marque « correction après l'arrêt » |
| Accessibilité | table native (`caption`, `th scope`), un `aria-label` par case (« élève, évaluation sur barème »), `aria-invalid`, zones `aria-live`, focus visible, aucun style en ligne (CSP), 375 px : première colonne collée, défilement horizontal **dans le cadre** (jamais la page), cibles de 36 px ; **axe-core et lecteur d'écran : non faits** (voir 3) |

### 1.2 Backend
`ent-db/src/evaluations_v3.rs` (grille, droits, saisie par lot, historique), `evaluations_v3_eval.rs` (créer, régler, annuler, publier, publications automatiques), `evaluations_v3_import.rs` (aperçu et confirmation), `ent-api/src/evaluations_v3.rs` (**14 routes `/eva/*`**), `ent-types/src/evaluations_v3.rs` (DTO et analyseur de cellule, compile pour `wasm32`), client `ent-web-api/src/evaluations_v3.rs`.
- **La grille se fait par cours** (socle 5), pas par « service avec matière » : **cause du blocage « service sans matière » corrigée** (l'ancienne grille exigeait une matière du référentiel ; un cours à libellé libre a maintenant sa grille, listé dans « mes enseignements », test dédié).
- **Droits** (`ent_prof_sur_cours` du socle 6) : saisie = titulaire, co-enseignant, **suppléant en saisie** à la date ; lecture = idem + `notes.lire` à portée établissement (**direction, secrétariat** : lecture seule) ; **famille et élève : jamais** (403) ; un autre professeur : refusé ; autre établissement : introuvable. Après l'arrêt de la période **de l'élève** : seule la direction (`notes.corriger`), **avec motif**. Suppléant : saisit, lit l'historique, crée ses évaluations, **ne règle pas** celles du titulaire sans droit de correction (CE-44) ; le titulaire de retour retrouve la main (CE-45).
- **Cas limites** testés : élève arrivé ou parti en cours de période (case « non concerné », phrase d'information), période arrêtée pour une classe seulement (création refusée si tous les élèves sont arrêtés, cases verrouillées sinon), évaluation à public restreint (**respectée en base**), évaluation commune (indicateur), cours sans matière, élève protégé (garde du socle 2 sur la grille et l'écriture), barème changé après saisie (motif obligatoire, jamais sous une note saisie), date changée (jamais si un élève noté sortirait du public).
- **Événements neutres** : identifiants seulement (test : ni note, ni motif, ni nom, ni titre).

### 1.3 Suites de V1-V2 (migration **471**)
- **F8** : une note déjà reprise puis corrigée par l'ancien écrivain est **recopiée** (une ligne d'historique par correction, **A → B → A** compris) ; si la nouvelle grille a écrit entre-temps, le conflit est **listé** (`deux_ecrivains`) et rien n'est écrasé. Test.
- **F7** : deux reprises simultanées se suivent (verrou consultatif) ; un doublon de course est compté « inchangé », jamais « refusé erreur_sql » ; invariant du rapport conservé. Test (`tokio::join!` de deux reprises).
- **F12** : un calcul impossible (débordement, réglage incohérent) n'échoue **que la matière concernée** (« moyenne indisponible » + avertissement), la moyenne générale s'établit sur les autres. Test avec neuf barèmes de dénominateurs premiers entre eux.
- **Défaut de V1 découvert par les tests de V3 et corrigé** : `ent_trg_eva_resultat` refusait **toute** mise à jour d'une ligne de source `reprise` (aucun écrivain ne pouvait donc plus corriger une note reprise, y compris l'écrivain de V1). Désormais la source prend celle de l'écrivain, une fois. **Le public restreint n'était pas contrôlé en base** : il l'est.

## 2. Tests
| Niveau | Résultat |
|---|---|
| `ent-types` (analyseur de cellule) | 7 verts |
| Modèle d'écran (`ent-web-app`, natif) | **15 verts** (collage, tableur, fichier, clavier, envoi, retours, conflits) |
| Base (`ent-db`, labo) — tests d'évaluations (V0, V1, revue, V3, anciens) | **69 verts** (21 nouveaux V3 : grille et moteur, cours sans matière, droits, arrêt et correction, conflits, erreurs par case, public restreint, arrivée/départ, suppléant, publication / retrait / journal, délai maximal et rappel, événements neutres, import, collage, **F7, F8, F12**, mesure) |
| Gardes (`ent-db`) | `migrate::`, `registre_donnees::` (ordre alphabétique corrigé), `tests_rls::`, `isolation_sql_statique`, `jour_utc_interdit` verts |
| `ent-api` | `separation` **56** et `recette_securite_3` **10** verts (inventaire des routes : +1 source) |
| **Frontend contre le vrai backend** (`ent-web-server`, `tests/grille_v3.rs`) | **2 verts** : rendu serveur (200, CSP, `lang`, pas de style en ligne) et parcours complet avec le client du frontend (cours, périodes, création, saisie, code rapide, hors-barème refusé, moyenne du moteur, conflit, historique, collage, aperçu, confirmation, empreinte périmée = 409, import par nom, publication, retrait ; **refus** : famille, direction en lecture, autre professeur, sans session) ; + les 2 tests de l'ancienne grille (`evaluations.rs`) toujours verts |
| Mode RLS stricte | **non lancé** (les 12 tranches relèvent de la CI sur `ENT`) |

Aucun `#[ignore]` ajouté ; aucune horloge réelle dans les tests de base (jour fixe 2031-10-20, « maintenant » fixe, planificateur à horloge fournie). Le test web ancre ses dates sur le début de période du cours (l'API prend le jour de l'établissement, comme l'ancien test).

## 3. Mesures et accessibilité
**Grille 40 élèves × 20 évaluations** (base locale, un fil, `--release`, test `v3_mesure_grille_40_eleves_x_20_evaluations`) : créer 20 évaluations **1,36 s** (68 ms chacune) ; écrire **800 cases** d'un seul envoi **3,45 s** (4,3 ms par case : droit, public, période, historique, événement) ; **lire la grille** (40 moyennes, 20 statistiques, statistique de classe, par le moteur) **52 ms** (médiane de 5 : 53 ms) ; **une frappe isolée** (une case, version lue) **30 ms**.
**Non mesuré : le rendu et la saisie dans le navigateur.** `cargo-leptos` et `wasm-bindgen-cli` ne sont pas installés dans ce conteneur et le build wasm complet n'a pas été fait ; **aucune mesure de rendu (40 × 20 = 800 champs) ni test clavier / 375 px / axe-core dans un navigateur n'a été fait**. La conception pose les mêmes précautions que `FRONTEND_LEPTOS.md` §4 (un signal par case, aucune relecture de tout le tableau à une frappe) mais **le rendu de 800 champs reste à mesurer** (hypothèse : 0,3 à 0,6 s d'après la mesure de 400 lignes citée dans le guide). À faire avant de déclarer la grille « sans latence perceptible » : parcours Chromium + Playwright + axe-core 4.10 (méthode de `15_ABSENCES_A11_DEF_captures/parcours.mjs`).

## 4. Vérifications avant push (commandes de la CI, toolchain 1.99.0, `--locked`)
`cargo clippy --workspace --all-targets -D warnings` **0** ; avec `ent-api/labo,ent-lab/labo` **0** ; `-p ent-types` **0** ; frontend : clippy SSR et wasm32 (`hydrate`) `-D warnings` **0** ; fmt **ciblé** par paquet (`ent-types`, `ent-web-ui/api/app/server/session` ; jamais `--all`) ; `rustfmt` des nouveaux fichiers du backend. `cd ent-rust/web && cargo metadata --locked` : **à vérifier par le pilote** (règle 34 : aucune dépendance ajoutée ; seules des *features* de `web-sys` sont ajoutées : `ClipboardEvent`, `DataTransfer`, `HtmlTextAreaElement`).

## 5. Écarts, limites, à trancher
1. **Cohabitation (M3, non faite).** La nouvelle grille **écrit dans `eva_*`** ; les lecteurs historiques (bulletin, conseil, relevé, exports) **lisent encore `ent_note` / `ent_devoir`** : une note saisie dans la nouvelle grille n'y apparaît pas tant que le lot de bascule (V4–V8, V12) n'a pas eu lieu. **Question au pilote** : fait-on le pont (écriture miroir dans les anciennes tables) ou la bascule des lecteurs avant tout usage réel ? Les vues `…_v` ne sont toujours pas renommées.
2. **Publication programmée** : le planificateur est **horaire** (le même que l'arrêt automatique) : retard possible d'une heure au plus.
3. **Création d'une évaluation par positionnement, barème détaillé / exercices (EVA-262 à 265), QCM, duplication multiple, conversion des notes à un changement de barème (CE-39), déplacement vers un autre cours (CE-51)** : **non faits** (le réglage refuse de passer sous une note saisie ; il propose « garder les valeurs » seulement).
4. **Absents proposés par l'appel, dispenses pré-remplies, rattrapages** : relèvent de V4 (ports 12, 34, 35), non faits. Le statut « à rattraper » se saisit (`ar`), sans séance de rattrapage.
5. **Saisie hors ligne et mobile (V9)** : non faits ; la version par case et l'empreinte sont prêtes pour R-V27.
6. **Direction** : en lecture seule sur une période ouverte ; **correction après l'arrêt depuis la grille** (champ « motif des corrections ») : implémentée côté serveur et testée, **non parcourue dans un navigateur**.
7. **Décision à faire valider** : l'import et le collage sont réservés au droit de **saisie** (la direction, même détentrice de `notes.corriger`, ne les utilise pas) ; une évaluation « annoncée » n'est pas encore exposée aux familles (V7).
8. **Statistiques d'évaluation** : calculées sur les élèves du public ayant un résultat (moteur) ; la médiane est celle des réglages de l'établissement.

## 6. Compteurs (règle 28) et documents tenus
`tests_rls.rs` **+2** tables avec RLS (`… + 31 + 13 + 2`) ; registre des données **+2** ; `Action::TOUTES` **inchangé** ; sources de routes **66 → 67** (14 routes) ; `TABLES_ELEVE` **inchangé** (aucune des deux tables n'a `eleve_id`) ; `MIGRATION_TAG` **inchangé** (3808). Migration **471** seule (472–479 réservées) ; aucune migration versée n'est modifiée. Documents : `MIGRATIONS.md`, ce rapport.

## 7. Base utilisée
PostgreSQL 16 **local** du conteneur, base `ent_v3_test`, rôle de test local. Aucune base de production, aucun secret, aucune donnée réelle. Une base de test doit être **recréée** après toute modification d'une migration non versée.
