# T3 — lot transverse de fiabilité, suite de T2 (2026-11-25)

Branche `ent-transverse-t3-fiabilite` (depuis `origin/ENT`, tête 53ed4604 au départ). **Aucune migration**, aucune nouvelle action, aucune table RLS : les compteurs de la règle 28 (`TOUTES`, compte RLS de `tests_rls.rs`, `MIGRATION_TAG`, `migrations_versees.txt`) sont **inchangés**. Aucun changement de comportement métier. Aucun `#[ignore]`, aucun `cargo fmt --all`.

## Fait

### 1. T2, M1 suite — jour de l'établissement du test frontend
`web/crates/ent-web-server/tests/assiduite_tableau.rs` ne lit plus `portail_aujourdhui().jour` (jour de Paris sur l'horloge du serveur). Le jour vient d'une route qui renvoie réellement `ent_aujourdhui_etab` : `POST /assiduite/situations/localisation` **sans `jour`** (le service prend `aujourdhui_etablissement`) rend `debut_jour`. La localisation est posée sur un autre élève que Harry (rôle CPE, droit `localisation.poser`), puis levée (`/lever`) : aucune trace pour la suite. Commentaire exact. (La route « règle en vigueur sans jour » ne renvoie pas le jour du serveur : `valide_du` est la date de début de la règle.)
*Non exécuté en local* : le job frontend de la CI le couvre ; compilation et mise en forme vérifiées.

### 2. T2, faibles 1 à 4
1. **Détecteur de clair** (`tests_eleve_protection.rs`) : le cœur base64 d'un nom de 10 octets fait au plus 12 caractères, donc le seuil `> 12` rendait la branche muette. Seuil `>= 8` (probabilité fortuite 64⁻⁸ par position), refus (panique) d'un clair de moins de 7 octets, témoin `le_detecteur_de_clair_retrouve_un_nom_de_dix_octets_en_base64` (trois alignements, vecteurs calculés hors du code, témoin négatif).
2. **Variantes `_conn`** : `tenant_tx::verifier_connexion_etablissement` (debug/test seulement, sans requête en release, sans effet si la RLS est coupée) vérifie que `current_setting('app.etablissement_id')` est celui demandé ; appelée par `aujourdhui_etablissement_conn`, `droits_effectifs_conn`, `proteges_caches_conn`, `eleve_cache_au_tiers_conn` (les autres variantes en dépendent). Test `t3_le_contrat_des_variantes_conn_detecte_la_connexion_d_un_autre_etablissement` (autre établissement, aucun contexte, témoin positif, mode coupé).
3. **`garde_rls_stricte`** : `controle_demarrage` n'est plus exempté au niveau du fichier ; trois exceptions ciblées sur les lignes (`.fetch_one(pool)`, `droit_annulation(self.pool())`, `droit_execution(pool,`), la liste des exceptions inutilisées reste vérifiée.
4. **Contrôle de démarrage** : `droit_execution` (signature en paramètre), `signaler` (journalisation séparée de la lecture). Tests : `t3_le_controle_au_demarrage_voit_une_fonction_absente` (base), `t3_le_controle_au_demarrage_emet_le_journal_attendu` (sans base : `error!` pour Manquant et FonctionAbsente avec le nom de la fonction, `warn!` pour une lecture impossible, rien pour Accorde). L'assertion tautologique de `p4_a_*` est remplacée par `Accorde`. Abonné de test partagé : `journal_limite::collecteur`.

### 3. I4, faibles 3 et 4
- Test `s5_avis_regroupe_texte_identique_reste_lu_et_texte_different_repasse_non_lu` (F1, contrepartie) : direct sur `notifier_personne_selon`, genre d'avis propre au test.
- `s5_avis_regroupe_*` : part d'un état vidé (avis et envois du genre pour le CPE) et le vérifie ; l'envoi est forcé à `en_attente` (`DO UPDATE`) et vérifié. Faible 2 : **documenté, limite assumée** (inchangé).

### 4. I5 passe 7, faibles
- `s7_sante_sentinelle_*` : `SET LOCAL lock_timeout = '3s'` avant tout `ALTER TABLE`/`DROP TRIGGER`.
- `s7_alerte_repli_*` : un compte **famille** à qui une attribution donne `inscriptions.gerer` reçoit **0 alerte**.

### 5. E1 passe 5, faibles 1 et 4
- `signaler_echec_etablissement` : journal limité à une ligne par minute et par contexte (`journal_limite`, cause `&'static str`) ; le **compteur** compte tous les échecs. Test `les_echecs_par_etablissement_sont_comptes_tous_mais_journalises_une_fois_par_minute`.
- `p5_e1_l_enveloppe_et_la_version_brute_rendent_le_meme_verdict` : existant, inconnu, verrouillé (55P03 brut ; l'enveloppe rend `pg_err::map_sqlx` de l'erreur brute).
- `p5_e1_une_annulation_validee_apres_le_report_ecarte_l_avis_avec_message_annule` : avis reporté tant que l'annulation est en cours, puis `ignore` / `message_annule` après sa validation.

## Non fait / hors périmètre
- Aucun point n'a demandé de migration ni de changement de comportement métier.
- Restent en file (non demandés dans ce lot) : cache de 55P03 par message (E1 faible 2), compteur de reports (E1 faible 3), masquage de causes différentes par le limiteur et compteur de lignes supprimées (E1 faible 5), restauration de sentinelle en cas de panique (I5), offre « d'avant » (I5).

## Tests (base locale neuve `ent_ci_test`, PostgreSQL 16, feature `labo`, `CARGO_TARGET_DIR` unique)
64 tests ciblés verts (tous les tests nouveaux ou modifiés : `le_detecteur_*`, `t3_le_*`, `les_echecs_par_etablissement_*`, `p4_a_*`, `aucun_acces_direct_*`, `s5_*`, `s7_*`, `p5_e1_*`, `t2_*`). **Non exécuté en local** : `assiduite_tableau` (web) — la compilation locale du test a été tuée (SIGKILL, mémoire) ; `cargo check --tests --features labo` passe, la CI le couvre. La suite complète n'a pas été rejouée en local (la CI le fait) ; le contrôle `_conn` n'agit qu'avec la RLS active (jobs stricts de la CI).

## Qualité
Clippy 1.99.0, les 3 commandes de la CI : **0 avertissement** (`ent-lab` compris). `cargo metadata --locked` dans `ent-rust/web` : OK. Mise en forme ciblée du frontend (`cargo fmt -p ent-web-*`) : OK.

## Tête et CI
PR #155 (brouillon, vers `ENT`). Tête testée par la CI : `315789250e1ca3aa1af08b18b46d9d7bb452bb78` — workflow « ENT Rust » : **succès** (run 45534021342, longtemps en file : runners saturés). Le test web `assiduite_tableau` est donc couvert et vert en CI. Le commit qui ajoute cette ligne ne change que ce document.
