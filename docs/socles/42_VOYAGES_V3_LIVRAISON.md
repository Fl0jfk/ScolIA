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

- **Appel de départ (V3b)** : le passage d'un participant à « parti » n'existe pas encore (aucune fonction ne l'écrit hors tests). Il devra appeler `internat_resynchroniser` dans sa transaction. **Correction (contre-revue M-1, migration 655)** : l'ancienne phrase « l'attendu de la nuit est correct sans retrait » était **fausse** pour un élève parti : la branche directe retirait l'élève sans aucune ligne de retrait, donc sans `id` que le CPE puisse annuler. Désormais **une seule source de vérité** : l'attendu ne retire un élève que s'il existe un retrait effectif posé par la synchronisation ; un participant passé « parti » par une écriture directe, sans resynchronisation, **reste attendu**. Les nuits d'une sortie ouverte ne sont donc retirées qu'après la synchronisation, et c'est le sens sûr.
- Écrans (Leptos) : demande, validation par la seconde personne, avertissement de motif, retour anticipé.
- Un participant « remis » / « parti seul » avant la fin du séjour n'est pas repris par la règle du retour anticipé (seul « rentré » l'est).
- Notification de la seconde personne (demande en attente) : à brancher au socle 8.

## 6. Suites de la contre-revue (D-069, branche `ent-socle-42-code-v3-suites`)

Rapport : `docs/pilotage/revues/42_VOYAGES_V3_CONTRE_REVUE.md` (branche `ent-mc0hoe`). Branche créée depuis `origin/ENT` ; **migration 655** (plage 640–659), ajoutée à la fin du tableau (règle 38) ; 653 et 654 sont versées et ne changent pas. **Statut : implémenté et testé en local ; non validé, non prêt pilote, non prêt production.**

### 6.1 Fait / non fait

| Réf. | Statut | Ce qui est fait | Test |
|---|---|---|---|
| **M-1** | fait | Source unique : la branche directe de `ent_internat_attendu_calcule` n'exclut un élève que s'il existe un retrait **effectif** posé par la synchronisation (655, corps identique à 2829 hors cette condition). Phrase fausse du §5.5 corrigée. | `m1_un_depart_ecrit_sans_resynchronisation…` (« parti » direct → **attendu** ; après resynchronisation → retiré ; le CPE annule le retrait) ; `fi_la_fonction_d_attendu_de_655…` |
| **M-2** | fait | Le seuil porte sur le **cumul** (écritures appliquées nettes + demandes en attente) par sortie, participant et payeur, pour les aides comme pour les reports. **Limite documentée** : la règle « fonds social » reste *déclarative* (la `source` est saisie) ; le cumul vaut pour **toutes** les sources, ce qui empêche le fractionnement et l'étiquette « autre » de passer sous le seuil, mais une aide du fonds social étiquetée autrement n'est soumise qu'au seuil de montant. | `m2_le_seuil_de_seconde_validation…` (149,99 € × 2 ; 0,01 € ensuite ; autre participant indépendant ; reports) |
| **M-3** | fait (décision du pilote : oui) | `exiger_decideur_libre` + déclencheur de base (`23514`) pour : plan, pointage et versements (dont remise de bordereau), annulation de versement, frais réel (saisie, annulation), décision d'avoir, annulation collective d'avoirs, acomptes fournisseurs (création, paiement ; annulation en application seule : la base ne garde pas l'auteur). Les pouvoirs sans compte précis (frais, annulation collective, acompte) vérifient aussi « payeur d'un compte qui porte une part d'un participant concerné ». Le périmètre des acomptes (aucun élève en particulier) est : **aucun participant de la sortie**. | `m3_la_regle_3_couvre_plans_versements…` (10 pouvoirs, parent **et** payeur refusés, contrôles de la base) |
| **M-4** | fait | Colonne `ent_sortie.annulee_le` (posée à la transition par l'application, sinon par la base ; immuable ; rattrapage des sorties déjà annulées) ; la synchronisation ne lit plus `modifie_le`. | `m4_la_date_d_annulation_ne_se_lit_plus_dans_modifie_le` (`modifie_le` déplacé de 2 jours : rien ne bouge) |
| **F-a** | fait | Ordre **personnel → droit → verrou → existence** sur les routes d'argent de V3, de la synchronisation et du retour anticipé (`avant_verrou`) ; `consigne_motif` réservée au personnel. Conséquence : un professeur sans droit d'argent reçoit 403 même pour une sortie inconnue (avant : 404) ; famille et élèves : 404 comme avant. | `fa_personnel_puis_droit_puis_existence…` ; HTTP |
| **F-b** | fait | Retour anticipé : `retour_le` optionnel, **borné** (≤ maintenant, ≥ départ constaté), l'instant de saisie est tracé dans le fait ; refusé (409) sur une sortie rentrée, close ou annulée. « Remis » / « parti seul » : **non fait** (déjà au §5.5). | `fb_retour_anticipe_borne_trace…` |
| **F-c** | fait | (1) Une demande en attente depuis plus de 30 jours **expire** (`DELAI_EXPIRATION_DEMANDE_JOURS`) : validation refusée (409), refus et retrait toujours possibles, statut `expiree` dans la liste. (2) L'annulation d'une aide ou d'un report soumis au seuil exige une **autre** personne que celle qui a décidé et que le demandeur d'origine. (3) Les refus de la règle 3 sont journalisés dans `ent_sortie_argent_refus` (propre transaction, aucun nom ni montant, ajout seul, jamais dans le journal de l'organisateur). | `fc_demande_expiree_annulation_soumise…` ; `m3_…` (10 refus tracés) |
| **F-d** | fait | Clé `…_retrait_fin_lien_fk`, participants de la demande liés à la sortie (clé composite), aide et report cohérents avec leur demande (déclencheurs). | `fd_la_base_garde_la_coherence…` |
| **F-e** | fait | `voyage_modifier_a`, `voyage_commande_a`, `voyage_participant_desister_a`, `voyage_participant_retirer_a` (horloge en paramètre ; l'annulation est datée par elle) ; le test d'annulation en cours de séjour n'utilise plus `now()`. `modifie_le = now()` reste côté base mais ne sert plus à aucun calcul d'attendu. | `internat_annulee_apres_le_depart…` (instant fixe) |
| **F-f** | fait | Une sortie **rentrée** n'est plus ignorée : seuls les participants « rentrés » gardent leurs nuits passées dehors ; un participant resté « parti » n'est plus retiré (sens sûr). Une sortie close ne change plus rien. | `ff_une_sortie_rentree_ne_garde_pas…` |
| **F-g** | fait | Le retour arrière de la reprise appelle la synchronisation (verrou de la sortie d'abord). Inatteignable avec des retraits (refusé si des faits de participants existent) : couvert par les tests existants de la reprise (`tests_voyages_v0d`). | — |
| **F-h** | fait pour 655 | 655 ne rejoue ni `DROP TRIGGER` ni `CREATE TRIGGER` (créés seulement s'ils manquent). Les rejeux de 653 (versée, règle 25) restent tels quels. | rejeu de la CI |
| **F-i** | fait | Test qui compare le corps de 655 à celui de 2829 (aux `public.` près, hors le bloc M-1) et vérifie que les conditions « sejour_scolaire » de 654 y figurent. | `fi_la_fonction_d_attendu_de_655…` |
| **F-j** | documenté | L'annulation du CPE bloque la repose du retrait du participant pour toute la sortie (`ecartes`) : sens sûr (faux « attendu » possibles, jamais faux « retiré »), au prix de fausses alertes si l'élève repart réellement. Choix gardé : le CPE rétablit, la direction ou le porteur constate le retour pour clore. | — |
| **Tests HTTP** | fait | `ent-api`, `routes_d_argent_de_v3_repondent_201_202_403_404_422` : aides, reports, demandes (validation, refus), annulations, consigne du motif, synchronisation, retour anticipé : **201, 202, 403, 404, 422** (et 401, 409). Les saisies illisibles des routes de V3 rendent **422** (`map_v3`), plus 400. | idem |
| **M-5** | sans objet (réponse du pilote) | 653 et 654 sont versées. | — |

### 6.2 Compteurs (règle 28)

| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | **271**, inchangé |
| Tables avec RLS (`tests_rls.rs`) | base ENT **+ 1** (`ent_sortie_argent_refus`) ; le total du lot V3 reste **+ 6** de 653/654, auquel s'ajoute ce **+ 1** |
| Registre des données | **+ 1** (`ent_sortie_argent_refus`, domaine `sorties_voyages`, données personnelles) |
| Migrations | **+ 1** : 655 en fin de `migrate.rs` ; `migrations_versees.txt` laissé au pilote (règle 38) ; `MIGRATION_TAG` inchangé (3808) ; `docs/socles/MIGRATIONS.md` à jour |
| Routes | inchangées (un champ optionnel `retour_le` sur le retour anticipé) |
| Tests | `tests_voyages_v3` : 29 (était 19) ; `ent-api` : + 1 test HTTP |

### 6.3 Vérifications (locales, PostgreSQL 16 neuve `ent_suites_test`, `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, `--locked`, un seul `CARGO_TARGET_DIR`)

- clippy 1.99.0, trois commandes de la CI (`--workspace --all-targets`, avec `ent-api/labo,ent-lab/labo` — `ent-lab` compris —, `-p ent-types`) : **0 avertissement** (`-D warnings`).
- rustfmt ciblé : `voyages_argent_v3.rs`, `tests_voyages_v3.rs`, `tests_voyages_v3_http.rs` seulement (jamais `cargo fmt --all`).
- Non touchés : Next.js, `main`, `dev`, `ui.rs`, `POINT_QUOTIDIEN`, `PILOTAGE`, base et secrets de production, `ENT`.

### 6.4 Résultat des suites

Tête de code `a626af19` (fusion d'`origin/ENT` incluse) : CI « ENT Rust » **verte** (https://github.com/Fl0jfk/ScolIA/actions/runs/45304240363). Une première tête (`bb888183`) avait échoué sur le test `m9` d'`tests_internat_i2` (conséquence voulue de M-1) : test adapté. PR n° 138 laissée en brouillon vers `ENT`. Statut : implémenté et testé ; non validé, non prêt pilote, non prêt production.

### Résultat final

Tête de code `5a895818` : CI « ENT Rust » verte (https://github.com/Fl0jfk/ScolIA/actions/runs/45017674799). Le commit suivant ne change que cette documentation ; sa propre CI est vérifiée avant envoi au pilote. PR n° 117 laissée en brouillon.

## Correctifs du pilote après contre-revue des suites (2026-11-24)
Rapport complet : `docs/pilotage/revues/42_VOYAGES_V3_SUITES_CONTRE_REVUE.md` (branche `ent-mc0hoe`). Aucun Élevé.
- **Moyen 1** : la **remise d'un bordereau** est un pouvoir sur l'argent (elle fixe écarts et alertes) : `voyage_bordereau_remettre`
  exige que la personne qui reçoit ne soit liée à aucun élève ni payeuse d'aucun compte des chèques du bordereau
  (`exiger_libre_des_participants`). Contrôle applicatif seul (pas de déclencheur sur `ent_sortie_bordereau_remise`) ; le
  commentaire de 655 est rectifié. Test `m3_la_remise_d_un_bordereau_est_refusee_au_parent_et_au_payeur`.
- **Faible 3** : `avant_verrou` en tête de `voyage_bordereau_saisir` et `voyage_bordereau_remettre` (parent et élève écartés
  avant le verrou de la sortie).
- **Moyen 3 (D-069)** : la règle « sortie entière » reste stricte (désistés et liste d'attente compris) : l'intégrité prime ;
  un contournement tracé par une seconde personne est en file avec V3b.
- En file (V3b) : Moyen 2 (sortie « rentrée » rétroactive, `rentree_le`), Moyen 4 (tests SQL directs 23514 manquants),
  faibles 1, 2, 4 à 7.
- Local : `tests_voyages` 147 verts ; clippy 1.99 (labo) vert.

## Suites V3b (2026-11-24, branche `ent-socle-42-code-v3b-suites`, base `origin/ENT`, migration 656)
Liste de travail : points « en file » de `docs/pilotage/revues/42_VOYAGES_V3_SUITES_CONTRE_REVUE.md` (branche `ent-mc0hoe`). Consigne : `docs/pilotage/consignes/42_VOYAGES_V3B_SUITES.md`. **Statut : implémenté et testé en local ; non validé, non prêt pilote, non prêt production.** 640–655 sont versées et ne changent pas (règle 25) ; **656** est en fin de tableau (règle 38), ses fonctions portent `SET search_path = pg_catalog, public, pg_temp` (sauf l'attendu de l'internat, famille « qualifiée » : relations `public.`, SQL inlinée) ; `migrations_versees.txt` est laissé au pilote.

### Fait / non fait
| Réf. | Statut | Ce qui est fait | Test |
|---|---|---|---|
| **Moyen 2** | fait | Colonne `ent_sortie.rentree_le`, posée à la transition vers « rentrée » (comme `annulee_le` : instant de l'application, sinon l'heure de la base), **immuable** tant que la sortie est rentrée ou close, vide ailleurs (contrainte + déclencheur). `internat_resynchroniser` borne le retrait d'un participant resté « parti » à la **veille du jour de `rentree_le`** : les nuits passées dehors restent retirées, les nuits à venir qu'il ne passera pas dehors le laissent attendu. Le rentré individuel garde son plafond (veille de son retour). | `ff_une_sortie_rentree_garde_les_nuits_passees_dehors…` (séjour de 4 nuits, rentrée au jour 2 : nuits 0 et 1 retirées, 2 et 3 attendues ; immuabilité ; réouverture efface) |
| **Moyen 3** | fait | **Contournement tracé** de la règle « sortie entière » (acompte : création, paiement, annulation ; annulation collective d'avoirs). La règle reste stricte (D-069). La personne liée **demande** (action, motif obligatoire) ; une **seconde personne**, distincte, habilitée (`voyages.argent`) et **libre** de tout lien avec la sortie, **autorise** ou refuse (motif au refus) ; la demandeuse **utilise** une seule fois, pour une seule écriture (la cible est tracée). Trois faits en ajout seul (`ent_sortie_argent_contournement`, `…_evt`), sans nom ni montant, hors journal de l'organisateur. **La base garde** : un acompte d'une personne liée n'est accepté que si un contournement autorisé par une autre personne libre a été utilisé pour **cette** écriture. 3 routes : `GET/POST …/argent/contournements`, `POST …/contournements/{cid}/decision` ; `contournement_id` optionnel sur les 4 actions. | `v3b_contournement_trace_de_la_sortie_entiere…` (refus, doublon, demandeuse ou personne liée qui décide, usage unique, autre action, autre personne, expiration, annulation collective, ajout seul), `v3b_la_base_n_accepte_un_acompte…` (gardes directes), HTTP `routes_d_argent_de_v3_repondent_…` |
| **Moyen 4** | fait | Tests SQL directs (`23514`) : frais réel (parent, payeur), décision d'avoir **INSERT et UPDATE** (retenu, écarté ; l'exclusion de la décision « signée système » passe), paiement d'acompte (UPDATE), annulation d'acompte (656), `ent_sortie_issue_coherente` (aide : source, compte, nature de la demande, demande inconnue ; report : montant, origine, destination, compte, nature). | `m4_les_declencheurs_de_655_refusent_en_base…`, `m4_ent_sortie_issue_coherente_refuse_en_base…` |
| **Faible 1** | fait | Branche directe « séjour scolaire » de l'attendu **supprimée** (redondante avec le CTE `retraits`, et plus large : son `EXISTS` acceptait un lien terminé). Un élève n'est retiré pour un voyage que par une ligne de retrait active et effective. | `fi_la_fonction_d_attendu_de_655…` (656 = 655 moins cette branche, aucune ligne nouvelle) ; `tests_internat_i2` |
| **Faible 2** | fait, une limite | (a) L'annulation d'une aide ou d'un report est soumise à la seconde personne quand le **cumul** engagé (écritures nettes) dépasse le seuil, pas seulement l'écriture annulée. (b) Une demande **expirée** (30 jours) ne compte plus dans le cumul. **Limite gardée (déclarative par nature)** : la règle « fonds social » repose sur la `source` saisie ; le cumul couvre toutes les sources. | `v3b_le_seuil_vaut_a_l_annulation_sur_le_cumul…` |
| **Faible 4** | fait | Sont tracés **tous** les refus d'un pouvoir sur l'argent exercé seul ou pour soi : règle 3, seconde personne exigée (validation, annulation), contournement. Un `23514` des déclencheurs venu **après** le contrôle (course) est reconnu (`db_pouvoir`) : même refus « interdit », tracé ; plus de 400 muet. **Plafond de débit** : au plus 30 traces par personne et par heure (le refus lui-même n'est jamais masqué). | `v3b_les_refus_de_seconde_personne_sont_traces…` |
| **Faible 5** | fait / constat | Le rattrapage `annulee_le` de 655 (versée, inchangeable) dépend du mode RLS de la session de migration. **Constat (raisonnement, non vérifié sur base peuplée : aucun accès à une base réelle)** : sous un rôle en politique stricte, l'`UPDATE` ne verrait aucune ligne ; mais la contrainte `ent_sortie_annulee_le_chk` est validée sur **toutes** les lignes sans tenir compte de la RLS : la migration **échouerait bruyamment**, jamais en silence. 656 rejoue le rattrapage (`annulee_le` et `rentree_le`) **établissement par établissement** (variable `app.etablissement_id` posée), idempotent. À confirmer par le pilote sur une copie peuplée. | — |
| **Faible 6** | fait | `ent_sortie_acompte_fournisseur.annule_par_user_id` ; contrainte `NOT VALID` (les acomptes déjà annulés n'ont pas d'auteur connu, jamais inventé) ; la garde d'immutabilité de 651 est recréée pour accepter la colonne ; le déclencheur de pouvoir garde désormais l'annulation côté base. | `v3b_contournement_trace…`, `m4_les_declencheurs…` |
| **Faible 7** | fait | `ent_sortie_retrait_effectif` recréée, relations `public.` et `SET search_path`. | garde `garde_search_path` |
| Faible 3 | déjà corrigé par le pilote | — | — |

### Choix prudents (D-069, aucune question posée)
1. Contournement : valable **30 jours** (demande non décidée, ou autorisation non utilisée) ; une seule demande ouverte par personne, sortie et action ; la personne qui décide est celle qui est **libre de la sortie entière** (même règle que l'action) ; elle n'a pas besoin d'être la seule : aucune exception silencieuse. L'annulation collective n'a pas de garde de base (elle ne fait que proposer des avoirs) : contrôle applicatif, usage tracé.
2. Un contournement ne contourne **jamais** la règle sur un pouvoir individuel (plan, versement, frais, décision d'un avoir) : seuls les cinq pouvoirs sur la sortie entière.
3. Sortie rentrée sans `rentree_le` (cas de rattrapage) : repli sur `modifie_le`, jamais sur la date de fin prévue.
4. L'écran (Leptos) de demande et de décision n'est **pas** fait (hors périmètre, comme au §5.5) ; notification de la seconde personne : à brancher au socle 8.

### Compteurs (règle 28)
| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | **271**, inchangé |
| Tables avec RLS (`tests_rls.rs`) | base ENT **+ 2** (`ent_sortie_argent_contournement`, `ent_sortie_argent_contournement_evt`) |
| Registre des données | **+ 2**, domaine `sorties_voyages`, données personnelles |
| Migrations | **+ 1** : 656 en fin de `migrate.rs` ; `migrations_versees.txt` au pilote ; `MIGRATION_TAG` inchangé (3808) ; `docs/socles/MIGRATIONS.md` à jour |
| Routes | **+ 3** (liste et demande de contournement, décision) ; un champ optionnel `contournement_id` sur 4 routes existantes |
| Tests | `tests_voyages_v3` : 36 (était 30) ; HTTP : assertions ajoutées au test des routes de V3 |
