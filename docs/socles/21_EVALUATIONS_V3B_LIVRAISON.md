# Socle 21 — lot V3b « les lecteurs passent sur `eva_*` » et suites de la revue sécurité V3 : livraison

**Date :** 2026-11-19 — **Branche :** `ent-socle-21-code-v3b` (depuis `origin/ENT` 1724f3bc) — **PR en brouillon vers `ENT`**.
**Niveau :** *implémenté + testé en local* (PostgreSQL 16 local dédié, base neuve) ; **CI GitHub : voir la PR** ; **non validé par Florian ; non prêt pilote.**
**À lire avec :** `21_EVALUATIONS.md` (§14, §22 M2/M3), `21_EVALUATIONS_V3_LIVRAISON.md`, `MIGRATIONS.md` (mise à jour du 2026-11-19, lot V3b).

## 1. Partie 1 — M2/M3 : tous les lecteurs lisent `eva_*`

### 1.1 Ce qui est fait
- **Migration 472** (461–471 intactes) : `ent_devoir` et `ent_note` sont **renommées** en `ent_devoir_archive` / `ent_note_archive` (aucune perte : données, index, contraintes, RLS conservés) ; les noms d'origine deviennent des **vues de compatibilité en lecture seule** sur `eva_*` (`security_invoker`). `ent_note_periode` reste une table (registre des périodes, cible des clés étrangères) avec `statut` **figé** par déclencheur. La vue `ent_note_periode_v` (relecture de l'état) et la fonction `ent_periode_de_note_pour_classe` relient période de notes et période de la périodisation de la classe.
- **Plus aucune double source ni écriture miroir** : l'ancien écrivain de notes n'existe plus que comme voie d'**amorçage** (labo/tests) vers l'archive, reprise ensuite dans `eva_*` par la fonction de reprise (idempotente, insérante seule).
- **Lecteurs basculés un par un** (chacun testé) : relevé élève/famille/PP/mobile (`releve_eleve_for_actor`), vue de classe, moyennes (`moyennes_eleve_periode`, `moyenne_matiere`), bulletin (`evaluations_bulletin.rs`), conseil (`evaluations_conseil.rs`), agenda des évaluations (`ent_devoir`), notifications de publication (consommateur d'événements : il écoute désormais aussi `evaluations.publiee`), lecteur `note_repo`, cahier de textes (C3–C5), référentiel de classe, `ent-lab`, UI JavaScript existante (lecture seule, sans extension de `ui.rs`). Les anciennes routes d'écriture répondent 410 (`MSG_GRILLE_FERMEE`).
- **Une seule règle de calcul** : tous les lecteurs appellent le moteur V2 (arithmétique exacte), calcul **identique au centième** à celui de la grille.
- **Seules les notes publiées** (`eva_resultat_publie` / colonne `publiee` de la vue) sont visibles côté élève et famille, y compris le retour de note du cahier (C5) : **choix assumé** — une note non publiée n'est jamais montrée, même si le professeur a coché « montrer la note » (le test C5 attend désormais la publication). Les autres droits du §14 sont inchangés (notamment : un droit d'établissement « notes.saisir » n'ouvre pas la saisie de la grille V3, liée au cours).

### 1.2 Test d'or et test de bout en bout
- **Test d'or** (`tests_evaluations_v1.rs`) : mêmes moyennes avant/après sur les données du labo, pour chaque lecteur (oracle de calcul historique rejoué contre le moteur unique).
- **Bout en bout** (`e2e_v3b_une_note_saisie_puis_publiee_est_lue_partout_avec_la_meme_moyenne`) : note saisie dans la grille V3 → non publiée : absente partout côté famille/élève ; publiée → relevé famille, conseil, bulletin : même moyenne ; rien n'est écrit dans l'ancienne table.

### 1.3 Limites assumées
- Une **note sans évaluation** (ancienne voie fermée en V0) reste dans l'archive : la reprise l'**ignore et la liste** (`note_sans_devoir`) ; aucun lecteur ne la montre.
- Le rejeu de tous les fichiers de migration (tests) **ignore** les ordres anciens qui visent `ent_note`/`ent_devoir` comme *tables* (colonnes, index, `SELECT *`…) lorsqu'ils échouent parce que ce sont des vues (`migrate.rs::apply_sql_file_rejeu`, erreurs 42809/55000/42601 uniquement) ; en production les migrations ne s'appliquent qu'une fois.

## 2. Partie 2 — suites de la revue sécurité V3

| Point | Réalisation | Test |
|---|---|---|
| M1 | écriture sur case vide = `INSERT` simple (23505 → conflit) ; `UPDATE … WHERE version` | deux transactions réellement concurrentes (2 tests) |
| M2 | reprise F8 : correction dans un sous-bloc `EXCEPTION`, mêmes contrôles que l'insertion, « refusée », note non écrite | `v3b_*` reprise |
| M3 | période ouverte : l'évaluation appartient au professeur ; un porteur de `notes.corriger` seul ne change ni barème/coef/comptage ni n'annule ; après l'arrêt : correction avec motif ≥ 3 caractères, journalisée | `v3b_*` |
| F1 | suppléant : saisit les notes du titulaire ; « en cours de correction » et modification d'évaluation suivent CE-44 | `v3b_*` |
| F2 | `eva_periode_etat` verrouillé `FOR SHARE` ; la base vérifie (déclencheur, attestation `app.eva_correcteur`) qu'une écriture après l'arrêt vient d'un porteur de `notes.corriger` | `v3b_*` |
| F3 | `origine = import` seulement pour la confirmation d'import ; `/eva/saisie` toujours `saisie` | `v3b_*` |
| F4 | retrait sous 24 h compté depuis la **première** publication (`premiere_publiee_le`, migration 473) | `v3b_*` |
| F5 | pas d'`eleve_id` dans `evaluations.resultats_saisis` / `evaluations.corrigee` | test de charge |
| F6 | historique : professeur du cours + direction seulement (EVA-139) | `v3b_*` |
| F7 | publication programmée : passage au moins toutes les 5 minutes (boucle serveur), retard maximal documenté (`RETARD_MAX_PUBLICATION_PROGRAMMEE_SECONDES`) ; `maintenant` des tests fixé à un jour explicite | `v3b_*` |
| F8 | bornes (`restreinte_a` 300, texte de case 40, motif 500) ; doublons de `cibles` ignorés ; `REVOKE EXECUTE … FROM PUBLIC` sur `eva_reprendre_existant`/`eva_reprendre_tout` ; contexte d'établissement obligatoire ; course barème/saisie réglée (verrou séparé) | `v3b_*` |
| F9 | le test des charges d'événements compare la **liste de clés autorisées** (plus des sous-chaînes) | `tests_evaluations_v3.rs` |
| Conversion de barème | **choix : on refuse de changer le barème dès qu'une note existe** (aucune conversion silencieuse de notes déjà saisies ; la direction ou le professeur recrée une évaluation) | `v3_creer_regler_et_annuler_une_evaluation` |

## 3. Compteurs (règle 28)
Aucune table ajoutée (`tests_rls.rs`, registre `TABLES`/`SOURCES`/`NOMS_SOURCES`, `TABLES_ELEVE` : seuls les noms `ent_devoir`→`ent_devoir_archive`, `ent_note`→`ent_note_archive` sont mis à jour ; garde `fusion`, `registre_donnees`, `tests_rls` verts) ; `MIGRATION_TAG` suit 473.

## 4. Vérifications locales
Base locale neuve dédiée (`CREATE DATABASE`), `CARGO_TARGET_DIR` propre à cette copie, toolchain 1.99.0, `--locked`.
- `ent-db` (feature `labo`) : suite complète 2 345 tests ; premiers échecs de la passe parallèle (33) traités : défauts réels corrigés (rejeu de migrations, notifications de publication, retour C5, scénario, rattachement, mapping de période) ou tests relatifs à un établissement du labo qui contient désormais des lignes `eva_*` issues de la reprise de l'amorçage ; les cas restants ont été rejoués **en série** et passent. Gardes `migrate::`, `registre_donnees::`, `tests_rls::`, `isolation_sql_statique`, `jour_utc_interdit`, `fusion` : verts.
- Clippy (commandes de la CI) et formatage ciblé (`ent-types`, paquets web) : voir l'état de la PR.
- Interdits respectés : aucun `#[ignore]`, aucun `cargo fmt --all`, aucune heure réelle ni jour UTC comme « maintenant » de test (`Utc::now()` réservé à l'horodatage de publication, hors assertion de jour).

## 5. Reste à faire / non fait
- V4–V8 (rattrapages collectifs, etc.) : migrations 474–479 réservées, non commencées.
- Validation par Florian, passe pilote : non faites.
- Suppression future des archives (`ent_*_archive`) : décision à prendre, rien n'est supprimé ici.
