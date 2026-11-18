# Socle 21 — lots V1 « modèle et reprise » et V2 « moteur de calcul étendu » : livraison

**Date :** 2026-11-18 — **Branche :** `ent-socle-21-code-v1v2` (depuis `origin/ENT` 86f03b82, run 417 vert). **Décision :** D-068 (l'ENT complet d'abord ; évaluations et notes au cœur ; **« comme de l'officiel »**, comparaison de Florian avec Charlemagne : idée I-032).
**Niveau :** *implémenté + testé en local (PostgreSQL 16, mode normal **et** RLS stricte)* ; **CI GitHub : à voir sur la pull request en brouillon** ; **non validé par Florian ; non prêt pilote.** Aucune route HTTP, aucun écran (V3), aucune absence ni dispense (V4), aucune IA : hors périmètre.
**À lire avec :** `21_EVALUATIONS.md` (§8, §9, §21, §22, §23, règles R-V, cas CE ; état V0 au §29, état V1/V2 au §30), **`21_REGLES_CALCUL_CHARLEMAGNE.md`** (règles de calcul, hypothèses [H], questions à Florian, cas-tests), `MIGRATIONS.md` (mise à jour du 2026-11-18).

---

## 1. Ce qui est fait

### 1.1 V2 — moteur de calcul étendu (logique pure)
`crates/domain-eleve/src/evaluation_moteur.rs` (+ `evaluation_moteur/tests.rs`). Module **pur** (ni base, ni horloge, ni réseau) ; le moteur de V0 (`evaluation.rs`) n'est **pas modifié** (les lectures actuelles continuent de l'appeler jusqu'à M3).

| Point du périmètre | Réalisation |
|---|---|
| Calcul exact | fractions d'entiers `Rat` à arithmétique **vérifiée** (un débordement est `ErreurCalcul::Debordement`, jamais une panique ni une valeur fausse) ; entrées en **millièmes** (`NUMERIC(7,3)` / `(6,3)` en base) |
| Statuts du §8.2 | `StatutCalcul` : note, absent, à rattraper, dispensé, non rendu, non noté, non concerné, annulé, en attente — chacun écarté avec sa **raison** |
| Comptage d'évaluation | normal, « compte si elle améliore » (ordre canonique, donc **indépendant de l'ordre de saisie**), « ne compte pas », bonus |
| Coefficients, barèmes | coefficients décimaux et 0 ; barèmes 5, 10, 20, 40… ramenés à l'échelle (10, 20, 100) |
| Sous-matières, matière sur plusieurs cours | `moyenne_matiere` (moyenne des moyennes pondérées, ou « toutes les notes ») ; pool des évaluations de tous les cours de la matière |
| Moyenne générale | pondérée ou non, matières hors moyenne générale, matières sans moyenne ignorées |
| Moyenne annuelle | **les deux méthodes** : moyenne des périodes (poids réglables) ; toutes les évaluations |
| Statistiques | moyenne de classe (moyenne des moyennes), minimum, maximum, **médiane** (effectif pair réglable), **seuil d'effectif de 5** pour les familles ; statistiques d'une évaluation |
| **Explication** | `MoyenneCalculee` porte, ligne à ligne, ce qui a compté, ce qui n'a pas compté et **pourquoi** (`explication_fr`) ; composantes (sous-matières, matières, périodes) |
| Arrondis | `Precision` (millième à unité, dont demi-point et quart) × `ModeArrondi` (demi haut, demi bas, demi pair, supérieur, inférieur) ; **étapes d'arrondi réglables** (note ramenée, sous-matière, matière, période) ; aucune par défaut |
| Règles « comme Charlemagne » | `ReglesMoteur` : chaque point de divergence possible est un réglage (voir le document de règles) ; **aucun réglage ne convertit une absence en zéro** (R-V5) |
| Diagnostic de comparaison | `diagnostic_absents_comptes_zero` : explique un écart avec un logiciel qui compte les absents 0 ; **jamais** branché sur une écriture ni une publication |
| Contrôle de saisie | `valider_saisie` (statut, barème, hors barème autorisé, **pas de saisie** de l'établissement) |

### 1.2 V1 — modèle et reprise (migrations 461 à 469, plage 461–479)
- **30 tables** `eva_*` (§21 hors IA, avec le complément « période par élève » §21.1), RLS, clés étrangères composites, aucune colonne JSON/tableau/BYTEA — détail dans `MIGRATIONS.md`.
- **Règles paramétrables par établissement** : `eva_reglage` (une par année, **historisée** en ajout seul, échelle et méthode annuelle **gelées** au premier arrêt, R-V21) ; couche `PgEnt::eva_v1_regles` et `eva_v1_definir_regles_for_actor` (droit `periodes.verrouiller`).
- **Machine d'états des périodes**, **en base** : `ouverte → notes_arretees → appreciations_closes → conseil_tenu → close`, un cran à la fois ; réouverture (retour à `ouverte` ou `notes_arretees`) **motivée** (3 caractères, **10 pour une période close**), jamais automatique ; chaque changement écrit `eva_periode_transition` (ajout seul). Ferme le défaut D2 : l'administratif ne rouvre pas, un professeur non plus.
- **Période par élève** (R-V29, bloquant A des personas) : `eva_periode_eleve_jour`, `eva_periode_eleve`, `eva_grille_eleves` — classe de rattachement à la date → périodisation → période → état **de cette classe** ; `eva_evaluation.periode_id` n'est qu'**indicative** (un test vérifie qu'**aucune fonction de contrôle ne la lit**).
- **Historique en ajout seul, plus aucune suppression physique** (D6, D7) : un résultat se remplace (l'ancienne valeur reste) ou s'annule (statut `annule`) ; `DELETE` refusé par déclencheur sur résultats, évaluations, règles, états de période ; `UPDATE` et `DELETE` refusés sur tous les journaux.
- **Journal des corrections de note** : `eva_resultat_historique` — **qui, quand, avant, après, motif**, marque `correction` ; après l'arrêt de la période **de l'élève**, une écriture exige un **motif** et le droit `notes.corriger` (direction), sauf rattrapage autorisé (`eva_rattrapage_autorise`).
- **Publication** (R-V7) : la vue `eva_resultat_publie` (sans note privée, sans auteur) ne montre que les évaluations publiées ; lecture par `eva_v1_resultats_publies_for_actor` (lien `peut_voir_notes` ou compte de l'élève).
- **Instantanés d'arrêt** immuables et **normalisés** (`eva_instantane` + `eva_instantane_ligne`, jamais un bloc JSON, §21.2).
- **Reprise M2** (`eva_reprendre_existant`, idempotente, **insertion seule**) : règles de calcul, coefficients de matière par classe, `ent_devoir` → `eva_evaluation`, `ent_note` → `eva_resultat` (DOUBLE → NUMERIC par le chemin décimal exact), `ent_note_correction` → historique (avec auteur, date, motif), `ent_note_appreciation` → `eva_appreciation`, `ent_note_periode` → états par classe. **Rapport** `eva_reprise_execution/compte/ligne` avec **invariant en contrainte** : `lues = créées + inchangées + refusées + ignorées` ; écarts listés (arrondi au millième, note hors barème, cours « reprise » à vérifier, dérive de l'ancien écrivain depuis la reprise). Aucun nom, aucune note, aucun texte d'appréciation dans le rapport.
- **Vues de compatibilité** pour les anciens lecteurs : `ent_devoir_v`, `ent_note_v`, `ent_note_periode_v`, `ent_note_appreciation_v` (lecture seule, `security_invoker`).
- **Couche Rust** `crates/ent-db/src/evaluations_v1.rs` : règles, écriture d'un résultat, historique, états de période, rattrapage autorisé, reprise et rapport, période par élève, grille, chargement des entrées du moteur et **moyennes d'un élève** (`eva_v1_moyennes_eleve_periode` : matières, générale, explication).

### 1.3 Sécurité (consignes du lancement)
| Exigence | Où et comment |
|---|---|
| Isolation par établissement dans toutes les requêtes | `etablissement_id` dans chaque requête (garde `isolation_sql_statique`), `TenantTx`, RLS forcée ; test `rls_sur_les_tables_eva_isole_les_etablissements` (rôle non propriétaire en mode strict : autre établissement et absence de contexte ne voient rien) |
| Une note non publiée n'est visible ni de la famille ni de l'élève | vue `eva_resultat_publie` ; moteur `publiees_seulement` ; test `une_note_non_publiee_n_est_visible_…` (retrait de publication compris, parent sans `voir_notes` refusé, note privée absente de la vue) |
| Élève protégé : gardes du socle 2 | `exiger_eleve_visible` avant toute écriture et lecture par identifiant (`NotFound`, réponse d'élève absent) ; `proteges_caches` sur la grille ; test `eleve_protege_introuvable_pour_un_tiers_…` (secrétariat et professeur ne voient pas l'élève protégé, la direction oui) |
| Événements neutres dans l'outbox | `evaluations.resultats_saisis`, `.corrigee`, `.notes_arretees`, `.periode_rouverte`, `.regles_modifiees`, `.reprise_terminee` : **identifiants seulement** ; test qui contrôle les clés et l'absence de valeur, de motif, de nom, de statut |
| Journal des corrections de note | `eva_resultat_historique` (voir 1.2) ; test `apres_l_arret_seule_la_direction_corrige_…` |
| Aucune migration versée modifiée | 460 et antérieures intactes ; `MIGRATION_TAG` inchangé |

---

## 2. Ce qui est testé

| Niveau | Tests | Contenu |
|---|---|---|
| Moteur (`domain-eleve`) | **71** nouveaux (372 dans le crate) | exemple de référence §8.8 (Léa : **12,80**) ; arrondi à 0,005 près (12,495 → 12,50 ; 13,125 → 13,13), les 5 modes, les 6 précisions ; tous absents, une seule note, coefficient 0, note bonus seule, égalités pour la médiane, effectif pair, seuil de 5 ; **propriétés** (aucune dépendance à l'ordre de saisie sur 40 mélanges et 3 jeux de règles ; une évaluation « si améliore » ne fait jamais baisser ; une évaluation « ne compte pas » ne change rien ; la moyenne reste entre la plus petite et la plus grande note ramenée) ; débordement = erreur ; cohérence avec le moteur de V0 |
| Base (`ent-db`, `tests_evaluations_v1`) | **16** | écriture/historique/suppression refusée ; refus hors barème, hors pas, élève non concerné, autre professeur ; machine d'états (sauts refusés, réouverture motivée, motif renforcé, rôles, journal en ajout seul, gel des règles) ; correction après l'arrêt ; **CE-65** (6 classes → ici 2 classes aux arrêts différents, cellule par cellule) ; **CE-66** (trimestres et semestres) ; aucune lecture de la période indicative ; règles relues fidèlement et historisées ; publication ; élève protégé ; événements neutres ; RLS ; **reprise idempotente et rapport** (invariant, écarts, correction conservée, dérive listée, droits) ; **test d'or** ; vues de compatibilité |
| **Test d'or** (§22 M2) | 1 | le lecteur historique (`eva_calculer_classe`, moteur de V0, sur les anciennes tables) et le moteur étendu (sur `eva_*` reprises) donnent **la même moyenne de matière et la même moyenne générale**, élève par élève, sur un jeu à 3 élèves, 2 matières, 7 devoirs aux barèmes 10/20/40, statuts absent / à rattraper / dispensé / non noté, une note hors barème, coefficients de matière par classe |

Mode strict : les 16 tests passent aussi avec `ENT_RLS_MODE=strict` (rôle applicatif non propriétaire).

**Aucune horloge réelle** dans ces tests : année fictive 2031-2032, dates fixes (le semis du labo, lui, garde ses dates réelles ; les tests ne s'y appuient pas).

### Vérifications avant push (commandes exactes de la CI)
Toutes lancées en local (toolchain **1.99.0**, `--locked`), sur la tête de la branche, avant le push :
- `cargo fmt --manifest-path ent-rust/web/Cargo.toml -p ent-web-ui -p ent-web-api -p ent-web-app -p ent-web-server -p ent-web-session --check` : **propre** ; `cargo fmt --manifest-path ent-rust/Cargo.toml -p ent-types --check` : **propre** (jamais `--all`). Les trois fichiers nouveaux du backend sont en outre passés à `rustfmt`.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` : **0 avertissement** ; `… --features ent-api/labo,ent-lab/labo -- -D warnings` : **0** ; `cargo clippy --locked -p ent-types --all-targets -- -D warnings` : **0** (règle 31 : aucun nouvel avertissement, rustc et clippy).
- `jour_utc_interdit` et `isolation_sql_statique` (`ent-db/tests`) : **verts**.
- Tests des fichiers existants touchés (`migrate`, `registre_donnees`, `tests_rls` : compte RLS +30) et des lectures d'évaluations : `ent-db` (`tests_evaluations`, `tests_evaluations_v0`, `tests_evaluations_v1`, `migrate::`, `registre_donnees::`, `tests_rls::`) **47 verts** ; `ent-api` `tests_evaluations_v0` **2** et `separation` **56** verts ; **web** `ent-web-server --test evaluations` (écrans rendus par le serveur et période complète avec le client du frontend) **2 verts** ; `domain-eleve` **372** verts.
- `cd ent-rust/web && cargo metadata --locked` : **OK** (règle 34 : aucune crate ajoutée, `Cargo.lock` du web inchangé).
- Compteurs (règle 28) recalculés à partir d'`origin/ENT` 86f03b82 : voir §5. `origin/ENT` n'a pas avancé pendant le lot : la fusion est sans effet.
- **Non lancé ici** : la suite complète d'`ent-db` (2 239 tests) et les 12 tranches de RLS stricte, qui relèvent de la CI de la pull request (les 16 tests V1 ont été lancés ici en mode normal et en mode strict).

---

## 3. Écarts à la conception (à faire valider)

1. **Vues de compatibilité nommées `…_v`** au lieu de remplacer `ent_devoir`, `ent_note`, `ent_note_periode`. Les écrivains actuels (grille S2-7, services) écrivent encore dans les anciennes tables : leur substituer une vue casserait la saisie. Le renommage (ancienne table `_ancien`, vue à son nom) est le geste de **M3** (V3 à V8). Forme exacte des vues documentée dans `468` (valeur 0 pour un statut non numérique, `non_rendu` et `annule` lus `non_note`, `publication_explicite` toujours vrai et `publie_le` porte la publication).
2. **Tables du §21 non créées** (hors périmètre ou lot voisin) : `eva_rattrapage_seance`, `eva_rattrapage_inscription`, `eva_sujet_partage` (V4) ; `eva_import_tiers` et ses lignes (V14, cohabitation) ; `eva_style_exemple` (IA) ; `eva_evaluation_element` (§21.2 : dépend des éléments du socle 22, qui n'existent pas encore).
3. **Instantané normalisé** (`eva_instantane_ligne`) au lieu de `contenu JSONB` : conforme à §21.2 et à D-036.
4. **`non_concerne` et `en_attente` ne sont pas stockés** dans `eva_resultat` : le premier est **calculé** (R-V2), le second est la **case vide** (§8.2) ; le moteur les connaît.
5. **Coefficient d'évaluation ≥ 0** (la conception disait > 0) : le coefficient 0 « ne pèse rien » sans supprimer l'évaluation ; nécessaire si Charlemagne l'admet (question C5).
6. **Reprise de la période `fermee`** → `appreciations_closes` (et `partielle_appreciations` → `notes_arretees`) : la conception n'a pas de correspondance écrite ; `close` supposerait un conseil tenu et des bulletins publiés, que l'ancien statut n'affirme pas. À confirmer.
7. **Motif renforcé de 10 caractères** pour rouvrir une période `close` : la conception dit « confirmation renforcée » sans la définir ; choix de cette session, un seul seuil en base.
8. **Notes sans devoir non reprises** (voie historique fermée en V0, `POST /eleves/{id}/notes`) : listées (`note_sans_devoir`), à rattacher par la direction. Une reprise « en bloc » les rangerait dans une évaluation fictive : non fait sans décision.
9. **Formules proposées [R], non décidées** : « compte si elle améliore » (ordre canonique des notes ramenées décroissantes) ; bonus « points au-dessus d'un seuil » (Σ max(0, note ramenée − seuil) × coef / Σ coef de base, ajouté à la moyenne). Elles servent de réglages candidats pour caler Charlemagne ; **rien n'est actif par défaut** (`bonus = ignore`).
10. **Pas encore de lien avec le moteur de V0** : les lectures actuelles (relevé, bulletin, conseil, exports) appellent toujours `evaluation.rs` ; le moteur étendu n'est appelé que par la couche V1 (M3 les bascule).
11. **Droit `recevoir_bulletins`** toujours absent (hérité de V0) : la lecture publiée s'appuie sur `peut_voir_notes`.
12. **Routes HTTP et écrans** : aucun (V3) ; la couche `PgEnt::eva_v1_*` est prête à les recevoir.
13. **Purge d'établissement** : les journaux en ajout seul n'ont pas de porte de purge (comme les autres journaux du dépôt avant le lot RGPD) ; à traiter avec la purge globale.

## 4. Questions

À **Florian** (via le pilote) — toutes dans `21_REGLES_CALCUL_CHARLEMAGNE.md` §6, par ordre de rendement ; les neuf premières se lèvent par saisie de cas-tests dans Charlemagne : barèmes différents (T1), absence = 0 ? (T4), non rendu = 0 ? (T5), moyenne générale sur matières arrondies ? (T10 bis), notes ramenées arrondies ? (T2), arrondi du demi (T3), moyenne annuelle et générale annuelle (T11), sous-matières (T9), moyenne de classe (T13).
Au **pilote** : (a) confirmer les écarts 5 à 8 ci-dessus ; (b) renommage des vues à M3 : dans quel lot (V3 ?) ; (c) la reprise automatique à 469 convient-elle pour la répétition de Noël, ou préfère-t-on la seule reprise explicite ?

## 5. Compteurs (règle 28) et documents tenus

- `tests_rls.rs` : **+30** tables avec RLS (`… + 14 + 3 + 30`) ; registre des données : **+30** ; `Action::TOUTES` **inchangé** ; routes **inchangées** ; `MIGRATION_TAG` **inchangé** (`3808_ntf_gels_liens_a_prevenir`).
- Documents : `21_EVALUATIONS.md` (copie de la version du pilote, §30 ajouté), `21_REGLES_CALCUL_CHARLEMAGNE.md` (nouveau), `MIGRATIONS.md`, `PROJECT_STATUS.md`, ce rapport.

## 6. Base démarrée

PostgreSQL 16 **local** du conteneur (`pg_isready -h 127.0.0.1`), base `ent_ci_test`, rôle de test `ent_ci` créé pour la session ; `ENT_DATABASE_URL` locale, `ENT_ENV=test`, `ENT_EXIGER_BASE=1`. **Aucune** base de production, aucun secret, aucune donnée réelle. Une base de test doit être **recréée** après toute modification d'une migration non versée (l'empreinte sqlx le refuse) : c'est fait à chaque évolution de 461–469 pendant le lot.

## 7. Suite proposée

V3 (grille et saisie : routes, écrans, renommage des vues, M3) → V4 (absences, dispenses, rattrapages) → lot « comparaison avec Charlemagne » (outil d'écarts, I-032) une fois les réponses aux questions C1 à C22 recueillies.

---

## 8. Correctifs de la revue indépendante (2026-11-18, même branche, PR 103)

**Choix de migrations.** 461 à 469 n'étant pas versées sur `ENT`, elles ont été **modifiées directement** (plus propre que d'empiler des correctifs ; la base de test se recrée, l'empreinte l'exige) ; **470** est nouvelle (F9). `migrate.rs` : 470 ajoutée après 469 ; `MIGRATION_TAG` inchangé. **Compteurs (règle 28)** recalculés : RLS **+31** tables du socle 21 (une de plus : `eva_evaluation_historique`), registre des données +1, `Action::TOUTES` et routes inchangés. Niveau : *implémenté + testé en local* (PostgreSQL 16, mode normal et RLS stricte) ; CI GitHub à voir sur la PR ; non validé par Florian.

### 8.1 Tableau des correctifs

| Point | Correctif | Fichiers | Test |
|---|---|---|---|
| **E1** TRUNCATE | `BEFORE TRUNCATE … FOR EACH STATEMENT` (convention 521 et 650) sur les 17 tables en ajout seul ou fondatrices : historiques, transitions, consultations, instantanés, reprise, `eva_resultat`, `eva_evaluation`, `eva_reglage`, `eva_periode_etat`, `eva_appreciation`, coefficients, rattrapages | 461 à 467 | `e1_truncate_est_refuse_sur_toutes_les_tables_protegees` (SQLSTATE 23514, `CASCADE`, y compris `TRUNCATE ent_eleve CASCADE`) |
| **E2** contournements | historique d'`eva_resultat` écrit **sans condition** (plus de `ent.eva_reprise`) ; `source = 'reprise'` refusée à l'INSERT comme à l'UPDATE, `source` immuable ; reprise = fonction **`SECURITY DEFINER`** qui désactive les gardes le temps de sa transaction (comme `ent_fin_purger_etablissement`, 522) et refuse un établissement hors du contexte ; plus de GUC `ent.eva_motif` : le motif est une **colonne d'entrée** (`motif_correction`, remise à NULL par le déclencheur à chaque écriture, copie dans `dernier_motif_correction` pour le journal) | 464, 467, `evaluations_v1.rs` | `e2_aucune_variable_de_session_ne_contourne_l_historique_ni_les_gardes` |
| **E3** gel | au premier arrêt, la ligne `eva_reglage` par défaut est créée (`INSERT … ON CONFLICT DO UPDATE SET fige_le`) ; un jeu inséré après un arrêt naît figé | 461, 463 | `e3_le_premier_arret_fige_les_regles_meme_sans_jeu_pose_et_m3_tout_est_historise` |
| **E4** rattrapage | `eva_rattrapage_en_vigueur` : dernière version, `jusqu_au` ≥ jour de l'établissement ; accepté seulement sur ligne absente ou `absent` / `a_rattraper` / `non_rendu` ; table **journalisée** (versions, ajout seul) ; la couche refuse une fin passée | 463, 464, `evaluations_v1.rs` | `e4_le_rattrapage_expire_et_ne_peut_pas_ecraser_une_note` |
| **M1** moyennes | `eva_v1_moyennes_eleve_periode` devient `#[cfg(all(test, feature = "labo"))]` ; `eva_v1_moyennes_famille_for_actor` (`publiees_seulement` **forcé**, lien famille, garde élève protégé) et `eva_v1_moyennes_personnel_for_actor` (`notes.lire` établissement + garde du socle 2) | `evaluations_v1.rs` | `m1_les_moyennes_passent_par_les_fonctions_d_acteur` (6 négatifs) |
| **M2** classe | dernière classe connue **dans la période**, sinon classe du cours ; coefficient introuvable = **avertissement dans l'explication** (`MoyenneCalculee::avertissements`), jamais un 1 silencieux | `evaluations_v1.rs`, `evaluation_moteur.rs` | `m2_un_eleve_sorti_…` (maths 12 coef 3, français 8 coef 1, sorti le 15/12 : **11,00**) ; `m2_sans_coefficient_trouve_…` |
| **M3** historique, coefficients | `eva_reglage_historique` complet (modes d'arrondi des 4 étapes, `bonus_seuil`, `plafonner_echelle`, statistiques, `figee`) ; **toutes** les règles de calcul figées au premier arrêt (donc recalcul = règles de la date d'arrêt) ; coefficients de sous-matière datés (exclusion de chevauchement) ; garde de non-rétroactivité (critère **de données**, sans horloge : un coefficient ne touche jamais un résultat existant) et lignes datées en ajout seul | 461, `evaluations_v1.rs` | `e3_…`, `m3_un_coefficient_n_est_jamais_retroactif_et_garde_son_historique` |
| **M4** évaluation | dès qu'un résultat existe ou qu'une période du public est arrêtée : `bareme`, `coef`, `comptage`, `mode`, `hors_bareme_autorise`, `date_eval` ne changent que par correction motivée ; **`eva_evaluation_historique`** (avant, après, motif, auteur) | 462 | `m4_une_evaluation_qui_porte_des_resultats_…` |
| **M5** vues | `ent_note_v` filtre la publication ; `ent_note_periode_v` : état le moins avancé de **toutes** les classes (`COALESCE(…,'ouverte')`) + colonne `appreciations_closes` ; `ent_note_appreciation_v` : validée **et** appréciations closes pour la classe de l'élève | 468 | `vues_de_compatibilite_…` (étendu), `m5_le_statut_de_periode_compte_toutes_les_classes_…` |
| **M6** statistiques | plancher dur 3 en base (`CHECK 3..100`), en Rust (`SEUIL_STATS_PLANCHER`) et refus par la couche ; `StatistiquesFamille` (champs privés) **ne se construit que par `pour_famille`**, qui respecte `stats_familles` et `mediane_familles` (médiane masquée par défaut) | 461, `evaluation_moteur.rs` | `le_seuil_des_statistiques_familles_ne_descend_jamais_sous_trois`, `statistiques_familles_respectent_les_reglages_et_le_cas_min_egal_max` |
| **M7** statistiques d'évaluation | `statistiques_evaluation` passe par `classer` (barème, hors barème, annulation, non-rendu) | `evaluation_moteur.rs` | `statistiques_d_une_evaluation_utilisent_le_meme_classement_que_la_moyenne` (10, 10, 10, 10 et 23/20 : moyenne **10**, maximum **10**) |
| **M8** verrouillage, arrêt auto | élève sans classe ni période à la date = verrouillé (base, couche, grille) sauf correction motivée ; `eva_arret_auto` + `eva_v1_arret_auto_au`, branchée sur `passages_quotidiens` (idempotent, jour de l'établissement, jamais après une réouverture, réglage `arret_auto`) | 463, `evaluations_v1.rs`, `passages_quotidiens.rs`, `ent-api/main.rs` | `m8_un_eleve_sans_periode_…`, `m8_arret_automatique_…`, `m8_le_reglage_arret_auto_…` |
| **M9** grille | titulaire du cours ou `notes.lire` à portée établissement ; évaluation inconnue = introuvable | `evaluations_v1.rs` | `m9_la_grille_n_est_lisible_que_du_titulaire_…` |
| **M10** tests | plus de `unwrap_or_default` qui masque une erreur ; RLS sur **toutes** les tables `eva_*` relevées au catalogue (forcée, `USING` et `WITH CHECK`, lecture, UPDATE, DELETE et INSERT hors établissement) ; détecteur de période indicative sur les fonctions SQL **et** sur le SQL de `evaluations_v1.rs` (avec témoins positifs et négatifs) ; échec sans base sous `ENT_EXIGER_BASE=1` vérifié | `tests_evaluations_v1.rs` | `rls_sur_les_tables_eva_isole_les_etablissements`, `aucune_fonction_de_controle_…`, `le_detecteur_de_lecture_…` |
| **F1** | bonus sans base : « bonus non compté : aucune autre évaluation à majorer » | `evaluation_moteur.rs` | `un_bonus_sans_base_est_etiquete_non_compte` |
| **F3** | état `jete` (plus de DELETE), éditeur de chaque version au journal | 465 | `f3_f6_f11_…` |
| **F4** | écriture sans effet : version inchangée, ni journal ni événement | `evaluations_v1.rs`, 464 | `f4_f5_une_ecriture_sans_effet_…` |
| **F5** | journal : commentaire, aménagé, rattrapage, note privée (avant et après) | 464 | `f4_f5_…` |
| **F6** | la réouverture vide `date_arret_effective` | 463 | `f3_f6_f11_…` |
| **F9** | `ent_public_cours_jour` écarte les cours de reprise à vérifier (sauf appel par identifiant) ; aucun placement ni séance d'EDT ne les nomme ; la reprise des appréciations les retrouve par une requête directe | 470, 467 | `f9_un_cours_de_reprise_n_apparait_ni_dans_les_publics_ni_dans_l_edt` |
| **F10** | commentaire de 464 (le motif « variable de transaction ») réécrit avec le mécanisme réel | 464 | — |
| **F11** | `matiere_id` d'une évaluation = sous-matière de la matière du cours | 462 | `f3_f6_f11_…` |
| **F13** | `EntreeCalcul::note(f64)` en `#[cfg(test)]` | `evaluation_moteur.rs` | compilation sans `test` |

### 8.2 Limites et points à trancher

- **E2, motif.** Le motif n'est plus une variable de session, mais le déploiement donne au rôle applicatif tout le DML sur toutes les tables : une colonne posée par l'écrivain reste, en théorie, **falsifiable par un écrivain qui contourne la couche**. La parade réelle est l'autorisation de la couche (`notes.corriger`), le fait que le motif soit **journalisé avec l'auteur** et remis à NULL à chaque écriture. Une barrière absolue exigerait de séparer les privilèges (fonction `SECURITY DEFINER` seule autorisée à écrire, DML retiré au rôle applicatif) : décision d'architecture, non prise ici.
- **M9, secrétariat.** Dans le modèle par défaut, `administratif` (et `cpe`, `direction`, `admin`) **détient `notes.lire` à portée établissement** : la règle demandée lui ouvre donc la grille. Le test qui « donne la grille au secrétariat » a été conservé pour ce qu'il prouve (l'élève protégé n'y figure pas), et complété : un autre professeur et la comptabilité sont refusés. **Si le secrétariat ne doit pas lire les notes de toute la classe, c'est le modèle de droits par défaut (`socle_21`) qu'il faut changer, pas ce code : question au pilote.**
- **M3, rétroactivité.** Critère de données (existence d'un résultat à partir de la date), pas l'horloge : les tests n'en dépendent pas. Un coefficient visant une classe ne compte que les résultats de cette classe ; MEF, niveau et défaut comptent toute la matière (prudent).
- **E4, révocation.** Une autorisation se révoque en en posant une nouvelle ; la couche refuse aujourd'hui une fin déjà passée (pas de geste de révocation exposé) : à voir en V4.
- **M5.** `ent_note_v` ne rend plus les notes d'une évaluation non publiée : un ancien lecteur qui affichait au professeur ses propres notes non publiées via cette vue doit passer par la nouvelle route (V3).
- **F10.** Le point n'étant pas détaillé dans la consigne, il a été traité comme « le commentaire de 464 décrit un mécanisme qui n'existe plus ».
- **À noter pour plus tard (non traités)** : **F2** (arrondi d'étape a en points cumulés : l'étape « note ramenée » n'a pas de sens dans cette pondération), **F7**, **F8** et **F12** : leur texte ne figure pas dans la consigne reçue.
