# Fiabilité des tests — lot 1

Branche `ent-fiabilite-tests-1` (base `origin/ENT`). Aucun changement fonctionnel, aucun test retiré ni affaibli, aucun `#[ignore]`. Tous les changements sont dans du code de test, sauf le garde `#[cfg(test)]` de `seed_labo_cast` (compilé uniquement en test).

## 1. Jeu d'essai concurrent (`seed_labo_cast`) — corrigé

**Cause exacte (constatée, pas supposée).** `deadlock detected` (SQLSTATE 40P01, classe 40 → `DomainError::Reessayer`). Un test voisin qui appelle `migrate::rejouer_tout` (DDL : `ALTER TABLE … DROP CONSTRAINT`, `DROP TRIGGER`, verrous `ACCESS EXCLUSIVE`) pendant qu'un autre test sème le labo forme un cycle de verrous avec les écritures du semis. Le journal PostgreSQL le dit : `Process … waits for AccessExclusiveLock on relation … ; blocked by …` sur `ALTER TABLE ent_eleve_lien DROP CONSTRAINT …`. Dans `tests_inscriptions_i2`, `i2_reprise_idempotente` fait ce rejeu pendant que les deux autres tests sèment : d'où les passages avec `--test-threads=1`. Le même cycle peut aussi faire du rejeu la victime (`i2_reprise_idempotente` a échoué ainsi, panique sur `rejouer_tout`).

**Correctif à la racine.**
- `seed_labo_cast` tient le verrou de rejeu **partagé** (`migrate::garde_rejeu_partagee`, le mécanisme existant) pendant tout le semis, uniquement sous `cfg(test)` : aucun rejeu ne s'intercale, les semis entre eux ne se gênent pas.
- `rejouer_tout` rejoue un fichier (idempotent) sur 40P01/40001, 5 tentatives au plus, attente croissante : ceinture contre un test qui n'a pas pris la garde.
- Découvert en chemin : `i2_reprise_idempotente` comptait les lignes de six tables **sans filtre d'établissement** ; les tests voisins en parallèle y écrivent (`(73,…) ≠ (74,…)`). Les décomptes sont maintenant bornés à l'établissement du test (même assertion, même sens).

**Preuves.** Avant : 4 lancements sur 5 de {`i2_reprise_idempotente`, `i2_references_uniques_sous_concurrence`, `i2_retrait_et_desistement_de_la_famille`, `i2_adresse…`} échouaient (3 « deadlock detected » au journal, plus l'écart de décompte). Après : **8 lancements sur 8 verts**, `0` occurrence de « deadlock » dans le journal PostgreSQL.

## 2. Rejeu 644/648 — corrigé (option A validée par le pilote)

**Cause.** 644 recrée `ent_sortie_document_type_v0c_chk` (6 types) quand elle est absente ; 648 l'a supprimée en posant `…_v1_chk` ; au rejeu, l'ADD de 644 échoue dès qu'une ligne a un type V1.

**Correctif.** `migrate.rs` : tableau `GARDES_DE_REJEU` (une entrée : 644 / `ent_sortie_document_type_v0c_chk`, avec le commentaire de la cause). Avant de rejouer 644, `rejouer_tout` pose une contrainte factice `CHECK (true)` du même nom : la garde de 644 saute son bloc, 648 la supprime. Les migrations ne sont pas modifiées (règle 25) ; `rejouer_tout` est sous `cfg(test)` et n'est appelé que par des tests (vérifié).

**Test** : `tests_d036::rejeu_644_648_avec_document_v1_garde_la_contrainte_v1` (document `ordre_mission`, `rejouer_tout` deux fois, seule `_v1_chk` subsiste). **Il échoue sans la garde** (panique sur `rejouer_tout`) et passe avec.

> **À tenir à jour** : tout futur couple « migration qui recrée une contrainte qu'une migration plus récente remplace » doit être ajouté à `GARDES_DE_REJEU`.

## 3. AS6 (exclusion + 11 minutes) — corrigé

`as6_eleve_exclu_de_cours_non_arrive_alerte_a_dix_minutes` : l'exclusion est datée par le service à `now()` ; plus 11 minutes, on franchit minuit entre 23 h 49 et 0 h (heure de Paris) et le jour du passage change. Le test fixe maintenant l'instant d'exclusion à 10 h 00 le même jour (UPDATE de `exclusion_cours_le`) et part de cet instant (règle 33). Aucune assertion modifiée. Vérifié à 23 h 50 heure de Paris simulée (libfaketime, PostgreSQL compris) : vert.

## 4. AG0 `rd06…` — **cause non reproduite** (honnêteté)

Le pilote a constaté un échec nocturne attribué à la plage calme. Lecture du code : `notifier_personne` insère `ent_notification` à toute heure ; seules les lignes `ent_notification_envoi` (`planifie_pour`) dépendent des heures calmes (20 h 30–7 h, dimanche), et `rd06` n'en lit aucune. Avec le test **d'origine**, je n'ai pas pu le faire échouer : horloge simulée (Rust seul) à 0 h 30, 1 h 30, 2 h 30, 4 h 30…8 h 30, 18 h 30–21 h 30, 23 h 30 UTC, jeudi et lundi ; puis horloge Rust **et** PostgreSQL à 23 h 50 Paris (mardi) et à 3 h 00 (dimanche), module AG0 entier (12 tests) : tout vert. De même avec une plage calme couvrant toute la journée.
Ce qui est fait : le `labo()` d'AG0 pose la même paramétrisation que le lot N1 (`ent_ntf_parametre` sans heure calme ni jour calme), pour que les envois planifiés ne dépendent plus de l'heure. C'est une précaution **non prouvée par un échec**, pas une cause établie. **Il faudrait la trace de l'échec (message, heure, jour)** pour conclure.

## Autres tests qui lisent l'heure réelle (non corrigés sauf mention)

Recherche : `Utc::now()` dans des `tests_*.rs` (82 fichiers en lisent l'heure ; la plupart l'emploient en relatif, sans risque). À regarder, par ordre de probabilité :
- `tests_assiduite_as6.rs:481-483` (`expedier_avis_dus(… Utc::now() + 12/20 min)` — avis d'absence après présence ; la remise dépend des heures calmes ; passe aux heures testées mais non fixé).
- `tests_messagerie_m4.rs:394-429` (`Utc::now() + 11/14/16 min` pour des fenêtres de correction de 15 min : relatif, sans minuit).
- `tests_cahier.rs:1726` (`Utc::now().hour() < 12` choisit un fuseau : conçu pour dépendre de l'heure, couvre les deux cas).
- `tests_absences_personnels.rs:2892` (décalage d'heure calculé sur l'heure UTC réelle).
- `tests_agenda_ag0.rs:80`, `tests_communication.rs:749` (`(now + 3 j)` à 9 h UTC : dates relatives, pas de minuit en jeu).
- `tests_agenda_ag1.rs:60`, `ent-api/src/tests_absences_personnels.rs`, `tests_personnels_p0/p2.rs`, `tests_voyages_v0d_v1.rs` : jours relatifs (+3, +20, +30 j…), sans effet de minuit tant que l'assertion reste relative au même `now`.
Aucun n'est évident au sens du lot ; non touchés.

## Environnement et vérifications

- Postgres 16 local (le CI utilise 18), `RUST_MIN_STACK=16777216` comme le CI (sans quoi pile débordée dans `tests_inscriptions_i2` en debug).
- **Toolchain locale : rustc 1.97.0, pas 1.99.0** : la conformité clippy 1.99.0 n'a pas pu être vérifiée localement ; elle repose sur la CI (voir fin du rapport pour le résultat clippy 1.97 si terminé).
- Mémoire : le conteneur tue `rustc` en compilation parallèle (`-j1` obligatoire).
