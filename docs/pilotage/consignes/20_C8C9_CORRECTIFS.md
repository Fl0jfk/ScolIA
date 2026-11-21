Tu es une session de correctifs ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi ; ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun.

## Cadre
- Lis `CLAUDE.md`, `git show origin/ent-mc0hoe:docs/PILOTAGE.md` (règles 17, 25, 27, 28, 31, 33, 34, 35, 36, 37), `docs/FRONTEND_LEPTOS.md`, la conception `git show origin/ent-mc0hoe:docs/socles/20_CAHIER_DE_TEXTES.md` et `docs/socles/20_CAHIER_C8_C9_LIVRAISON.md`.
- Branche `ent-socle-20-code-c8c9` (PR n° 110, brouillon) : pars de sa tête distante, puis **fusionne `origin/ENT`** (recalcule les compteurs, règle 28, à partir de la base commune). Les migrations **2407 et 2408 ne sont pas versées : modifie-les** ; aucune autre migration versée ne change.
- Interdits : Next.js, `main`, `dev`, pousser sur `ENT`, base ou secret de production, données réelles, `cargo fmt --all`, `#[ignore]`, POINT_QUOTIDIEN, PILOTAGE, étendre `ui.rs`, heure réelle ou jour UTC dans les tests. Zéro avertissement, clippy 1.99 (backend et frontend), `--locked`, fmt ciblé, `cargo metadata --locked` dans `ent-rust/web`. Base locale **neuve dont le nom contient « test »** ; un seul `CARGO_TARGET_DIR`.
- **Règle 36** : pas de `subscribe_pr_activity` ; après push, attends la CI par une seule commande en arrière-plan (boucle `until` sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min). Rouge : corrige la cause.

## Règles générales
1. **Une année close est figée en base**, pas seulement dans le code : aucune écriture sur le cahier d'une année close, quel que soit le chemin, sauf après une réouverture tracée.
2. Ce qui est visé est figé, et tout ce qui change après un visa (modifié **ou ajouté**) est visible du viseur.
3. Un pouvoir = une action : on ne délègue jamais plus que ce qu'on veut déléguer.
4. Ordre des contrôles : personnel, droit, existence.

## Correctifs (revue du pilote ; chemins sous `ent-rust/`)
- **E1 (élevé)** : `ent_cdt_cahier_fige` n'est appelée que par `cdt_exiger_annee_ouverte` (`crates/ent-db/src/cahier_c2.rs` ~535) ; `cdt_entree_retirer`, `cdt_contribuer`, les reports de C6 (`cahier_c6.rs` ~229, ~266, ~831, ~903) et la planification C9 écrivent sans contrôle. Correctif : déclencheur `BEFORE INSERT OR UPDATE` sur `ent_cdt_entree`, `ent_cdt_travail`, `ent_cdt_contribution` et leurs versions, qui lève 23514 si l'année est figée ; seule la réouverture le désarme. Test par chemin (retirer, contribuer, report C6, planification C9).
- **M5** : la clôture et les écritures ne se croisent pas (le déclencheur suffit, ou `FOR SHARE` sur `ent_annee_scolaire` dans la transaction d'écriture). Test de concurrence.
- **M1** : scinder `cahier.exporter` en `cahier.exporter` (export) et `cahier.cloturer` (clôture, réouverture, décision d'archive) ; la réouverture exige un motif d'au moins 20 caractères et une seconde personne de la direction (double validation) ; Admin n'a pas ces droits par défaut (ou justifie-le) ; l'octroyeur ne peut pas s'octroyer à lui-même un accès d'inspection. Compteurs mis à jour.
- **M2** : `GET /cahier/c8/mon-export` n'écrit plus en base : le texte est rendu sans stockage ; les exports persistés passent par POST, un seul par (auteur, année, empreinte) ; taille plafonnée avant insertion (422 au-delà, jamais 500).
- **M3** : message « le professeur concerné est prévenu » corrigé tant qu'aucun avis n'existe ; écrire l'avis dans la liste des suites (socle 8) comme prérequis de l'accès « rendez-vous de carrière ».
- **M4** : mention « aucune donnée structurée d'élève » au lieu de « aucune donnée d'élève » ; avertissement à la saisie qu'un nom d'élève dans le texte libre sera visible des inspecteurs et des exports ; rapport corrigé.
- **M6** : plus de transaction imbriquée sur le pool (`c8_intervenant`, `c8_agent_du_compte`, `c8_cours_ref`, `c9_intervient`) : passer `&mut tx`.
- **M7** : « ajouté depuis le visa » affiché pour les objets publiés de la période absents des éléments du visa.
- Faibles : F1 garde TRUNCATE de `ent_cdt_acces_inspection` ; F2 gardes de déclencheur qui figent `decide_par_user_id`, `decide_le`, `revoque_par_user_id`, lien d'empreinte archive/export ; F3 ordre des contrôles ; F4 comptages d'un visa de classe limités aux cours visibles ; F5 filtre avant `LIMIT` ; F6 rôle et appartenance revérifiés à chaque lecture d'inspection ; F7 copie limitée au même niveau ; F8 progression archivée non planifiable ; F9 clés étrangères composites sur `matiere_id`, `niveau_id` ; F10 registre `.export()` pour les tables concernées ; F11 l'écran d'octroi ne dépend pas de `cahier.viser` ; F12 et F13 documentés ou corrigés.
- Tests : T1 la séparation exige 403 ou 404 explicites (plus `EchecQuelconque`) ; T3 tests des chemins après clôture, de la course, de `mon-export` répété, du changement de rôle d'un inspecteur, d'un export `litige` par un professeur.

## Fin
- Complète `docs/socles/20_CAHIER_C8_C9_LIVRAISON.md` d'une section « Correctifs de revue » (fait / non fait, tests, compteurs, tête, CI). Puis écris au pilote (send_message vers @parent) la tête finale et l'état de la CI.
