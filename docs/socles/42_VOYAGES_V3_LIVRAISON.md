# Socle 42 « Sorties et voyages » — lot V3 « argent relié » : livraison

Branche `ent-socle-42-code-suite` (partie de `ENT`, tête 02659006c). Conception : `docs/socles/42_SORTIES_VOYAGES.md` §12.4, §12.5, §6.2, VOY-224, VOY-286 ; D-068 (la comptabilité des voyages est épaissie et reliée au reste de l'ENT, pas refondue) ; écart n° 9 du rapport V2a.

**Statut : implémenté et testé en local, non validé par Florian, non prêt pilote, non prêt production.** Ce rapport a été complété par la passe de correctifs de revue (§5, D-069) : les §1 à §4 décrivent le lot d'origine, le §5 dit ce qui a changé depuis (il prévaut en cas de différence).

## 0. Choix de périmètre (D-069, valeur prudente, aucune question posée)

Le « V3 » du tableau §27.1 (EDT et appel : neutralisation fine, non-partants, appel de départ, conflits) **n'est pas livré ici** : la consigne demande d'épaissir et relier la comptabilité (D-068). Ce lot ferme donc les trois liens les plus utiles laissés ouverts par V2a ; le V3 « EDT et appel » reste à faire (renommé **V3b** par convention de ce rapport).

## 1. Livré

| Lien | Ce qui est fait | Où |
|---|---|---|
| **Aides** (VOY-224, socle 26 à venir) | aide inscrite par la comptabilité sur la part d'**un** payeur (APEL, fonds social, collectivité, entreprise, don affecté, autre) ; elle réduit le reste dû, **n'est jamais de l'argent versé** : un avoir ne la rend pas au payeur (CF-69) ; plafonnée au reste dû (versé + aides) ; les versements aussi tiennent compte des aides ; annulation par écriture inverse motivée ; aide d'un participant parti signalée « à restituer ou réaffecter » ; liste nominative réservée à `voyages.argent`, consultation journalisée | `voyages_argent_v3.rs`, migration 653 |
| **Reports** (écart 9) | report d'un versement d'un participant à un autre **du même payeur** (même compte des deux côtés, imposé par la base) ; jamais plus que le versé du premier ni que le reste dû du second ; l'avoir du partant est recalculé (ancienne proposition écartée « signée système », nouvelle proposée) ; refusé si un avoir est déjà retenu ; annulation inverse motivée, refusée si l'argent n'est plus chez le receveur, et avoirs du receveur mis « à revoir » | idem |
| **Familles** | un payeur voit sa part, **son** aide (jamais sa source), son versé, son reste, les reports de ses enfants (sans le nom de l'autre enfant) ; l'autre payeur du même élève ne voit rien | `voyage_argent_famille` |
| **Responsable désigné** | voit ce qui couvre la part (aide comprise) sans savoir qu'une part est une aide ; l'état agrégé de l'organisateur ne contient aucune aide | `LignePaiementResponsable` |
| **Export** | CSV : lignes `aide`, `aide_annulee`, `report_cede`, `report_annule` | `construire_csv` |
| **Internat** (socle 41) | `POST /voyages/{id}/internat/synchroniser` : **[corrigé en §5 : seul le départ constaté retire l'interne]** chaque **interne** confirmé (prêt ou parti) est retiré de l'attendu des nuits du séjour (nuit du départ → veille du retour, jours de l'établissement), motif `sejour_scolaire`, source `voyage` ; idempotent ; dates changées = ancien retrait annulé, nouveau posé ; désistement/retrait d'un participant = retrait annulé automatiquement (jamais supprimé) ; événement `internat.attendu_modifie` sans nom ni montant | idem, migration 654 |
| **Autorisations** (socle 18) | pas de nouveau lien : le départ d'un participant bloqué par une exigence reste couvert par le motif `exigence_manquante` → avoir (V2a) ; un parent séparé ne voit toujours que son compte | — |

Règles tenues : argent au centime (texte → `centimes_de_texte`, `CHECK` de bornes, `checked_*`) ; aucune donnée de santé ; ajout seul pour tout mouvement (UPDATE/DELETE/TRUNCATE → `42501`, testé) ; aucune écriture dans `ent_fac_*` ni `ent_fin_*`.

## 2. Compteurs (règle 28)

| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | **271** (inchangé : droits `voyages.argent` et `VoyagesModifier` réutilisés) |
| Tables avec RLS (`tests_rls.rs`) | base ENT **+ 3** (`ent_sortie_aide`, `ent_sortie_report`, `ent_sortie_internat_retrait`) |
| Registre des données | **+ 3** tables, domaine `sorties_voyages` |
| `SOURCES` / `NOMS_SOURCES` | 69 inchangé (routes ajoutées dans `voyages_argent.rs`) |
| Routes | **+ 5** : `…/argent/aides`, `…/aides/{aid}/annulation`, `…/argent/reports`, `…/reports/{rid}/annulation`, `…/internat/synchroniser` |
| Migrations | **653** et **654** (plage 650-659 du socle 42 ; 655-659 libres). 654 est appliquée **après 2827** dans `migrate.rs` (elle référence `ent_internat_retrait`) ; `MIGRATION_TAG` inchangé (3808) |

## 3. Écarts, hypothèses et questions

1. **Aide d'un participant parti** : l'aide n'est pas annulée automatiquement ; la comptabilité décide (restituer à la source, réaffecter). Signalée `a_restituer`.
2. **Report** : seulement au sein d'un même payeur (prudence : pas de transfert d'argent d'une famille à une autre sans accord). Un remplaçant d'une autre famille passe par un avoir puis un nouveau versement.
3. **Internat** : seuls les internes dont un séjour d'internat couvre une nuit sont retirés ; la dispense de tarif d'internat/pension (socle 24/40) n'est **pas** faite. La synchronisation est déclenchée par le porteur ou la direction ; aucun planificateur (comme V2a).
4. **Source `voyage`** ajoutée à la contrainte des retraits du socle 41 par remplacement de contrainte (migration additive ; 2824 non modifiée).
5. **Non fait** (suite du socle) : écran famille (Leptos), clôture financière (VOY-227/230/231, V11), demande d'aide par la famille (socle 26), V3b EDT/appel.

## 4. Vérifications (locales, PostgreSQL 16 neuve `ent_test_v3`, `ENT_EXIGER_BASE=1`, `--locked`)

- `ent-db` : 8 tests V3 (`tests_voyages_v3`) ; 195 tests « voyages » ; registre, RLS, conservation, internat, finances, rôles (305 tests) ; gardes `jour_utc_interdit`, `isolation_sql_statique`, `garde_finances_ecritures`, `garde_inscriptions_ecritures` verts.
- **Incident corrigé** : la première version de 654 posait une clé étrangère vers `ent_internat_retrait` ; elle faisait échouer `tests_internat_i2::e2_truncate_refuse_…` (TRUNCATE sans CASCADE). La clé est retirée (le lien est écrit dans la même transaction) ; test d'I2 et V3 reverts.
- clippy 1.99.0 sur les trois commandes de la CI (`--workspace`, avec `ent-api/labo,ent-lab/labo`, `-p ent-types`, `-D warnings`) : **0 avertissement** ; rustfmt appliqué aux seuls fichiers nouveaux ; `cargo metadata --locked` dans `ent-rust/web` : ok.
- Aucune heure réelle ni jour UTC dans les tests (départ = jour du cadre).
- `origin/ENT` fusionné : déjà à jour (02659006c), compteurs inchangés par rapport au §2.
- **Non fait localement** : suite complète d'`ent-db` et tests `ent-api` (séparation, recette sécurité) ; la CI fait foi.
- Next.js, base et secrets de production, `main`, `dev`, `ENT` : non touchés.

## 5. Correctifs de revue (D-069)

Branche `ent-socle-42-code-suite`, partie de la tête distante `208a81ab` (ENT déjà fusionné ; `origin/ENT` = `ebac1517` au lancement, déjà contenu : rien à refusionner). Les migrations **653 et 654 n'étaient pas versées : elles sont modifiées en place** ; 2826 (versée) n'est pas touchée, l'attendu de l'internat est remplacé par `CREATE OR REPLACE` dans 654. **Statut : implémenté et testé en local ; non validé, non prêt pilote, non prêt production.**

### 5.1 Règles générales appliquées

1. **Pas de départ constaté = élève attendu.** Un retrait « voyage » n'est effectif que si le participant lié est *parti* (ou remis / parti seul autorisé) ; un participant *rentré* (retour anticipé) n'est retiré que jusqu'à la veille de son retour. La fonction `ent_sortie_retrait_effectif` (654) porte la règle ; la branche « séjour scolaire » directe de `ent_internat_attendu_calcule` (reprise de 2826) ne retient plus « prêt ».
2. **Une seule fonction de resynchronisation**, `internat_resynchroniser` (`voyages_argent_v3.rs`), appelée dans la transaction de : changement d'état de la sortie (`voyage_commande` : annulation, refus, réouverture, confirmation), modification de dates (`voyage_modifier`), désistement ou retrait d'un participant, retour anticipé, et de la route de synchronisation. Seules les sorties **ouvertes, prêtes, en cours** produisent un retrait ; une sortie *annulée* ne garde que les nuits déjà passées dehors (jusqu'à la veille de l'annulation) ; une sortie *rentrée* ou *close* ne change plus rien (historique des nuits passées dehors conservé, aucun faux cas « non couvert »). Chaque fin de retrait par la synchronisation est tracée (`ent_sortie_internat_retrait_fin` : cause `dates`, `participant`, `sortie`, `retour_anticipe`).
3. **Un pouvoir sur l'argent ne s'exerce jamais pour son propre enfant.** Toute aide et tout report passent par une *demande* (`ent_sortie_argent_demande`, ajout seul) suivie d'une *issue* unique (`ent_sortie_argent_issue`). Le décideur, le valideur et l'auteur d'une annulation ne sont ni liés à l'élève (`ent_eleve_lien`, compte d'élève), ni payeur du compte (foyer payeur, personne du compte, élève majeur désigné) : contrôle applicatif (message) **et** déclencheur de la base (contournement refusé, `23514`). Seuil de seconde validation par établissement au registre des paramètres (socle 5) : `voyages.aide_seuil_seconde_validation_centimes` (défaut 15 000 = 150 €, strictement au-dessus) et `voyages.aide_fonds_social_seconde_validation` (défaut vrai : toute aide du fonds social). Au-delà, la demande attend une **seconde personne** (autre que le demandeur, libre de tout lien) qui valide ; les montants sont revérifiés à la validation. Le demandeur peut retirer sa demande, une autre personne la refuse (motif obligatoire). Motif **obligatoire** partout, tracé avec acteur et instant dans les tables d'ajout seul — jamais dans le journal de l'organisateur (le motif d'une aide ne doit pas lui parvenir).
4. **Ordre des contrôles** : personnel/droit (`voyages.argent`), puis existence, puis lien et règles métier ; compare-et-écrit partout (annulation unique par index, issue unique par demande), motif historisé.

### 5.2 Correctifs

| Réf. | Statut | Ce qui est fait | Test |
|---|---|---|---|
| **E1** | fait | Resynchronisation dans la même transaction (§5.1 règle 2). | `internat_annulation_ou_reouverture…`, `internat_sortie_annulee_en_cours…`, `internat_annulee_apres_le_depart…`, `internat_dates_modifiees_par_l_api…`, `internat_desistement_et_retrait…` |
| **E2** | fait | Règle 1 (retrait posé seulement pour un départ constaté, effectif seulement à partir de celui-ci, 654 remplace la fonction de 2826). Le test d'I2 qui attendait « prêt → retiré » est corrigé (`prêt → attendu`). | `internat_pas_de_depart_constate…` (prêt → attendu, parti → retiré, rentré plus tôt → attendu) |
| **M1** | fait | Annuler un versement dont le net deviendrait négatif à cause d'un report cédé : 409 « annule d'abord le report ». | `m1_annuler_un_versement…` |
| **M2** | fait | **Choix : l'annulation du CPE est mémorisée et respectée** (pas de renvoi obligatoire vers la sortie). Raison : un élève attendu à tort se retrouve en quelques minutes, un élève retiré à tort n'est cherché par personne ; le CPE voit l'enfant, il doit pouvoir le rétablir sans droit sur le voyage. La synchronisation distingue son annulation de la sienne (trace de fin) et ne repose pas le retrait (`respectes` dans le résultat) ; la branche « live » de l'attendu la respecte aussi. Le **retour anticipé** est prévu : `POST /voyages/{id}/participants/{pid}/retour-anticipe` (droit `VoyagesParticipants`), participant « parti » → « rentré », retrait arrêté à la veille du retour, élève attendu à nouveau, dans la même transaction. | `internat_annulation_du_cpe…`, `internat_pas_de_depart_constate…` |
| **M3** | fait | Règle 3 : paramètres du registre, défaut prudent, décideur parent / payeur refusé, seuil, seconde validation, motif vide refusé. | `regle3_le_decideur…`, `regle3_au_dela_du_seuil…` |
| **M4** | fait | Aide, report et pointage de versement refusés sur une sortie close, annulée ou refusée (409). Les **annulations** (écritures inverses de correction, motivées) restent permises : une aide d'une sortie annulée doit pouvoir être restituée. | `m4_aide_report_et_pointage…` |
| **F1** | fait | Clé d'idempotence (`cle`, 8 à 64 caractères) sur aide et report : un double clic, même concurrent, renvoie la même décision, une seule écriture ; clé réutilisée avec un autre contenu = 409 ; validation, refus et annulations en compare-et-écrit (409 au second clic). | `f1_un_double_clic…` |
| **F2** | fait | `AVERTISSEMENT_MOTIF` renvoyé avec chaque décision et par `GET /voyages/{id}/argent/consigne-motif` (« aucun détail de situation sociale, de revenus ou de santé »). L'écran (Leptos) devra l'afficher à la saisie : **non fait ici** (pas de frontend dans ce lot). | `aide_…` (avertissement présent) |
| **F3** | fait | 654 ne retouche plus la contrainte des sources de retrait si `ent_internat_retrait_source_v3_chk` existe (aucun DROP/ADD au rejeu). Le rejeu (`rejouer_tout`) rejoue 2826 puis 654 : la fonction remplacée par 654 reste la dernière. Le test de rejeu est une tranche `labo` : **non exécuté en local**, la CI fait foi. | — |
| **F4** | fait | `maintenant` en paramètre (`…_a`) pour aide, annulation d'aide, report, annulation de report, validation, refus, synchronisation, retour anticipé, et pour les faits datés de participant ; les variantes sans suffixe appellent `Utc::now()` à la frontière (API). Tests à instant fixe. Subsistent `now()` de la base pour les colonnes par défaut des tables **antérieures** (non modifiées). | tous les tests V3 |
| **F5** | fait | L'annulation d'un report qui remet des avoirs « à revoir » écrit un fait du journal (`avoirs_a_revoir`, acteur, nombre, sans nom ni montant). | `f5_l_annulation…` |

### 5.3 Compteurs (règle 28)

| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | **271**, inchangé (aucun nouveau droit : `voyages.argent`, `VoyagesModifier`, `VoyagesParticipants` réutilisés) |
| Tables avec RLS (`tests_rls.rs`) | base ENT **+ 6** (était + 3) : `ent_sortie_argent_demande`, `ent_sortie_aide`, `ent_sortie_report`, `ent_sortie_argent_issue`, `ent_sortie_internat_retrait`, `ent_sortie_internat_retrait_fin` |
| Registre des données | **+ 6** tables, domaine `sorties_voyages` |
| Registre des paramètres (`structure_etab::registre`) | **+ 2** clés (socle propriétaire 42) |
| Routes | **+ 10** (était + 5) : `…/argent/demandes`, `…/demandes/{did}/validation`, `…/demandes/{did}/refus`, `…/argent/consigne-motif`, `…/participants/{pid}/retour-anticipe` s'ajoutent aux cinq d'origine ; `…/argent/aides` et `…/argent/reports` (POST) rendent désormais 201 (appliquée) ou 202 (en attente de la seconde validation) avec une décision, plus un enregistrement brut |
| Migrations | 653 et 654 (modifiées, non versées) ; `MIGRATION_TAG` inchangé |
| `Cargo.lock` | `url` 2.5.8 → 2.5.9 (RUSTSEC-2026-0592), `form_urlencoded` 1.2.2 → 1.2.3, dans `ent-rust` et `ent-rust/web`, rien d'autre (correctif identique à la PR 121) |

### 5.4 Vérifications (locales, PostgreSQL 16 neuve `ent_test_corr`, `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, `--locked`)

- `ent-db` : 19 tests `tests_voyages_v3` verts (11 nouveaux ou refondus) ; suite `ent-db` complète en `cargo test --lib` : 733 verts ; les 3 échecs restants (`tests_reprise_r2/r3`, comparaison de comptes de **toutes** les tables pendant que d'autres tests écrivent) sont une pollution de l'exécution parallèle dans une base partagée, **sans lien avec le lot** : verts à `--test-threads=1` (18/18) ; la CI (nextest, un processus par test) n'est pas concernée.
- Aucune heure réelle ni jour UTC dans les assertions : instant fixe `2031-03-12T11:00:00Z` pour les écritures, nuits relatives au jour du cadre, et la seule lecture de l'annulation en cours de séjour compare à la valeur lue en base (`ent_aujourdhui_etab(modifie_le)`).
- clippy 1.99.0, trois commandes de la CI : voir « Résultat final » ci-dessous ; `cargo metadata --locked` dans `ent-rust/web` ; rustfmt appliqué aux seuls `voyages_argent_v3.rs` et `tests_voyages_v3.rs`.
- Non touchés : Next.js, `main`, `dev`, `ent-rust/crates/ent-db/src/ui.rs`, base et secrets de production, `ENT`.

### 5.5 Reste à faire (non fait)

- **Appel de départ (V3b)** : le passage d'un participant à « parti » n'existe pas encore (aucune fonction ne l'écrit hors tests). Il devra appeler `internat_resynchroniser` dans sa transaction ; en attendant, l'attendu de la nuit est correct sans retrait (branche directe pour une sortie prête ou en cours) mais l'historique (nuits d'une sortie ouverte) ne l'est qu'après une synchronisation.
- Écrans (Leptos) : demande, validation par la seconde personne, avertissement de motif, retour anticipé.
- Un participant « remis » / « parti seul » avant la fin du séjour n'est pas repris par la règle du retour anticipé (seul « rentré » l'est).
- Notification de la seconde personne (demande en attente) : à brancher au socle 8.

### Résultat final

Tête de code `5a895818` : CI « ENT Rust » verte (https://github.com/Fl0jfk/ScolIA/actions/runs/45017674799). Le commit suivant ne change que cette documentation ; sa propre CI est vérifiée avant envoi au pilote. PR n° 117 laissée en brouillon.
