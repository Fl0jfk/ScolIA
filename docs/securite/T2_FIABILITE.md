# T2 — lot transverse de fiabilité (2026-11-24)

Branche `ent-transverse-t2-fiabilite` (depuis `origin/ENT`). Aucune migration, aucune nouvelle action, aucune table RLS : les compteurs de la règle 28 (`TOUTES`, compte RLS, `MIGRATION_TAG`) sont inchangés.

## Fait
1. **Test frontend dépendant de l'heure** (`web/crates/ent-web-server/tests/assiduite_tableau.rs`) : le jour n'est plus calculé par le test (`jour_paris()`) mais lu auprès du serveur (`portail_aujourdhui().jour`). **Correction du pilote après contre-revue (M1)** : ce jour est le jour de Paris sur l'horloge du serveur (`maintenant_local`), **pas** `ent_aujourdhui_etab` ; il coïncide avec lui pour l'établissement du labo (fuseau de Paris). Le changement ne supprime qu'un écart d'horloge entre le test et le serveur ; la cause de l'échec CI initial **n'est pas établie**. *Non exécuté en local* (le job frontend de la CI le couvre). En file : lire le jour par une route qui renvoie `ent_aujourdhui_etab` (ex. règle en vigueur sans jour).
2. **Test instable** (`tests_eleve_protection`) : marqueur long de lettres (`ZorglubMarqueurUsageTresLong`) ; absence du clair vérifiée en clair, hexadécimal, base64 et base64url aux trois alignements ; témoin positif `le_detecteur_de_clair_retrouve_le_marqueur_sous_toutes_ses_formes`.
3. **Internat, connexions imbriquées** : `exiger_eleve_visible_conn`, `a_action_etablissement_conn` (sur `droits_effectifs_conn`), utilisées dans `vie.rs` (654, 1680/1687, 1798/1804, 2193) et `groupe_appel_exiger`. Tests sur pool d'**une** connexion : `t2_visibilite_et_droit_d_etablissement_*`, `t2_appel_de_groupe_exiger_*`, `t2_lever_une_vigilance_*` (résultats comparés à ceux des variantes par le pool). Les appels hors transaction (801, 1812, 2090, 513, 571, 1220…) sont inchangés.
4. **Messagerie** : `controle_demarrage.rs`, appelé par `boot_from_env` ; `error!` sans donnée personnelle si `has_function_privilege(current_user, 'ent_msg_annulation_ouvrir(uuid,uuid,uuid)', 'EXECUTE')` est faux (ou fonction absente) ; ne bloque pas le démarrage. Test `p4_a_le_controle_au_demarrage_*` (rôle applicatif : accordé ; rôle sans GRANT : manquant).
5. **Recherche de l'heure réelle dans les tests** : ~79 fichiers `ent-db/src/tests_*.rs` et ~27 fichiers de `tests/` utilisent `Utc::now()`, `jour_paris()` ou `today_utc()`. Les `jour_paris()` des tests web sont cohérents avec le semis (jour de Paris). Plus gros utilisateurs de `Utc::now()` à revoir : `tests_afaire_p2` (24), `tests_communication` (17), `tests_assiduite_as5` (12), `tests_afaire_p3` (12), `tests_cahier` (9), `tests_messagerie_m0a` (8), `tests_absences_personnels` (api 7, db 5), `tests_messagerie_m0b` (5), `tests_assiduite_as6` (5, AS6 déjà corrigé). **Non corrigés** (pas simples sans lecture de chacun) ; connu et non traité : `tests_agenda_ag0::rd06_rappels_emis_*`.

## Tests exécutés (base locale neuve `ent_t2_test`, labo)
`t2_*` (4), `p4_a_*`, `les_versions_sensibles_*`, `tests_eleve_protection` (13), `s5_eleve_cache_*` : tous verts. Clippy 3 commandes : voir « Qualité ».

## Qualité
Clippy local en 1.97 (1.99 non installable ici). Commandes 1 et 3 : 0 avertissement. Commande 2 : 2 avertissements, tous deux **antérieurs au lot** et hors de ses lignes (`tests_voyages_v2a.rs:865` et `:877`, `unwrap_err` après `is_err`) ; non corrigés ici (hors périmètre). Aucun avertissement dans les lignes du lot.

## Tête et CI
Tête testée par la CI : `7907cddec1d36b96c2d08ca2f0ae85ad6a6e98e0` — workflow « ENT Rust » : **succès**.
Un premier passage (tête `ac38ca23`) était rouge pour deux causes propres au lot, corrigées : mise en forme de `assiduite_tableau.rs`, et garde RLS (`controle_demarrage` déclaré dans `FICHIERS_SANS_CONTEXTE` : il ne lit que le catalogue). Un échec de `a4_cap31_…` (`Reessayer`, test de concurrence non modifié) n'est pas réapparu sur la tête finale.
La PR #147 reste en brouillon vers `ENT`.
