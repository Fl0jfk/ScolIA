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
1. Contournement : valable **30 jours** (demande non décidée, ou autorisation non utilisée) ; une seule demande ouverte par personne, sortie et action ; la personne qui décide est celle qui est **libre de la sortie entière** (même règle que l'action) ; elle n'a pas besoin d'être la seule : aucune exception silencieuse. [Corrigé par « Suites V3b, correctifs » ci-dessous : l'annulation collective a désormais sa garde de base.]
2. Un contournement ne contourne **jamais** la règle sur un pouvoir individuel (plan, versement, frais, décision d'un avoir) : seuls les cinq pouvoirs sur la sortie entière.
3. Sortie rentrée sans `rentree_le` (cas de rattrapage) : repli sur `modifie_le`, jamais sur la date de fin prévue.
4. L'écran (Leptos) de demande et de décision n'est **pas** fait (hors périmètre, comme au §5.5) ; notification de la seconde personne : à brancher au socle 8.

### Compteurs (règle 28)
| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | **282**, inchangé par ce lot (la première rédaction annonçait 271 : erreur de la livraison, corrigée ; vérifié dans `ent-auth/src/actions.rs`) |
| Tables avec RLS (`tests_rls.rs`) | base ENT **+ 2** (`ent_sortie_argent_contournement`, `ent_sortie_argent_contournement_evt`) |
| Registre des données | **+ 2**, domaine `sorties_voyages`, données personnelles |
| Migrations | **+ 1** : 656 en fin de `migrate.rs` ; `migrations_versees.txt` au pilote ; `MIGRATION_TAG` inchangé (3808) ; `docs/socles/MIGRATIONS.md` à jour |
| Routes | **+ 3** (liste et demande de contournement, décision) ; un champ optionnel `contournement_id` sur 4 routes existantes |
| Tests | `tests_voyages_v3` : 36 (était 30) ; HTTP : assertions ajoutées au test des routes de V3 |

### Vérifications et résultat (Suites V3b)
- Local (PostgreSQL 16 neuve, `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, `--locked`, un seul `CARGO_TARGET_DIR`) : `ent-db` lib complète 776 verts (3 échecs `tests_reprise_r2/r3` = pollution parallèle connue, 18/18 verts à un thread) ; tranche `labo` internat/voyages : verts, 4 délais d'attente de statement sous charge parallèle, verts relancés à 2 threads ; `ent-api` (labo) tests voyages : 5 verts. clippy 1.99 sur les 3 commandes de la CI : **0 avertissement** ; `cargo metadata --locked` dans `ent-rust/web` : ok ; rustfmt ciblé sur les fichiers touchés du lot.
- La première CI a révélé un test d'I4 (`m6_l_attendu_garde_la_branche…`) qui cherchait la trace du CPE dans la branche supprimée : adapté (l'annulation du CPE tient au CTE des retraits).
- Tête de code `8576b959` (`origin/ENT` fusionné, aucune migration ni compteur d'ENT touché) : CI « ENT Rust » **verte**. PR n° 145 laissée en brouillon vers `ENT`.
- Non touchés : Next.js, `main`, `dev`, `ui.rs`, `POINT_QUOTIDIEN`, `PILOTAGE`, `migrations_versees.txt`, base et secrets de production, `ENT`.

## Suites V3b, correctifs (2026-11-24, contre-revue `42_VOYAGES_V3B_CONTRE_REVUE.md`, même branche, même PR 145 en brouillon)
Liste de travail : Moyens 1 à 3 (bloquants) puis faibles 1 à 9 du rapport. **656 n'est pas versée** : elle est corrigée en place, toujours **en fin de tableau** de `migrate.rs` (règle 38, `search_path` et relations `public.` sur toute fonction créée ou recréée). 640–655 inchangées ; `migrations_versees.txt` laissé au pilote. **Statut : implémenté et testé en local ; non validé, non prêt pilote, non prêt production.**

### Fait / non fait
| Réf. | Statut | Ce qui est fait | Test |
|---|---|---|---|
| **Moyen 1**, pas d'autorisation « en blanc » | fait | La demande porte `cible_id` (acompte visé : payer, annuler), `montant_max_centimes` et `prestataire_id` (créer, payer) ; la contrainte `…_portee_chk` impose la portée par action (l'annulation collective vise la sortie entière, rien de plus). L'API de liste (`ContournementVue`) montre la portée à l'autorisatrice ; la demande d'API est `DemandeContournement` (`montant_max` en euros). `ent_sortie_contournement_valide` / `_ouvert` et le déclencheur de pouvoir refusent toute écriture hors cible, au-delà du plafond ou pour un autre prestataire ; le fait « utilisé » refuse une autre cible. Côté service, une demande de paiement exige que l'acompte existe sur la sortie, ne soit ni payé ni annulé et soit au même prestataire. | `v3b_une_autorisation_ne_couvre_que_ce_que_la_demande_porte_service_et_sql_direct` (autre acompte, montant supérieur, autre prestataire : service **et** SQL direct) ; HTTP : demande sans portée = 422 |
| **Moyen 2**, contrôle en base complet | fait | **Expiration** : `now() <= autorisation + 30 jours` pour l'usage (fonction et déclencheur), demande de moins de 30 jours pour la décision. **Autorisatrice** : membre de l'établissement avec le profil `admin`, `direction` ou `compta` (`ent_sortie_habilite_argent` : la règle de `voyages.argent` dans `ent-auth`, R-SV44), **dont l'accès est ouvert** (`ent_acces_ouvert` : état « ouvert », date de fin non passée, agent non suspendu ; correctif du pilote B1) **et** libre de la sortie. **Horodatages** : `demande_le` et `le` posés par la base (`DEFAULT now()` + déclencheurs) ; une valeur fournie est ignorée. **Annulation collective** : `ent_sortie_avoir_propose.propose_par_user_id` (`NOT VALID`) ; un avoir `annulation_collective` d'une personne liée, ou sans auteur, est refusé (`23514`), sauf contournement utilisé pour la sortie. | même test (membre non habilité, personne extérieure, `le` forcé à 2000, expiration à 31 jours par `ent_sortie_contournement_valide(…, now() + 31 jours)`, avoirs directs) |
| **Moyen 3**, rattrapage sous RLS stricte | fait | Énumération `ent_etablissement UNION ent_membership` (comme 2220), réglage `app.etablissement_id` précédent restauré ; le bloc est encadré par `-- DEBUT-DONNEES` / `-- FIN-DONNEES`. | `v3b_le_rattrapage_de_656_atteint_les_lignes_sous_la_rls_stricte` (rôle strict : un `UPDATE` naïf touche 0 ligne ; le bloc rattrape `rentree_le` et `annulee_le`) |
| Faible 1 | fait | La fenêtre d'une demande « ouverte » (doublon refusé) est comptée depuis l'**autorisation** pour une demande autorisée (statut `autorise` 30 jours), depuis la demande tant qu'elle attend : plus de deux autorisations vivantes pour la même portée. Une demande identique = même (personne, sortie, action, cible, prestataire). | test de portée (doublon d'une demande autorisée) |
| Faible 2 | fait | Le droit de l'autorisatrice (membre habilitée et libre) est revérifié **à l'usage** (fait « utilisé » et écriture). | test de portée (profil retiré entre l'autorisation et l'usage : SQL direct et service) |
| Faible 3 | fait | 403/404 harmonisés : sans droit d'argent sur la sortie, la liste, la demande et la décision répondent **404** comme `acomptes` ; le 403 ne vient plus que du droit applicatif général (personnel sans `voyages.argent`). | HTTP existant (porteur : 403 par le droit ; famille : 404) |
| Faible 4 | fait (écran : à faire) | Le motif est libre (500 caractères). Consigne « n'indique aucun nom d'élève ni de famille » à afficher sur l'écran de demande (l'écran Leptos n'est pas fait, hors périmètre, §5.5) ; **durée de conservation** : tant que la sortie existe et au plus 10 ans (pièces comptables, `CONSERVATION_CONTOURNEMENT_ANS`) ; la purge est un traitement d'archivage à brancher. | — (documenté) |
| Faible 5 | fait | Plafond de 30 traces de refus **par (personne, action) et par heure** ; au-delà, **une** ligne `plafond_atteint` par (personne, action) et par heure signale le dépassement (la table est en ajout seul : pas de compteur modifiable). | `v3b_les_refus_de_seconde_personne_sont_traces…` (35 refus : 30 traces + 1 marqueur ; une autre action n'est pas masquée) |
| Faible 6 | documenté | `rentree_le = modifie_le` au rattrapage est une **approximation** : la dernière modification de la sortie, jamais antérieure au passage réel en « rentrée » ; elle ne peut que raccourcir la période des nuits retirées d'un participant resté « parti ». Aucune donnée réelle n'a été consultée pour mesurer l'écart ; à confirmer par le pilote sur une copie peuplée. | — |
| Faible 7 | fait | `rentree_le` fourni en SQL direct est refusé s'il dépasse `now()` de plus de **7 jours** (`23514`, tolérance prudente D-069 : les fixtures de test datent la sortie quelques jours en avant) ; les instants passés (rattrapage) restent acceptés. | `ff_une_sortie_rentree_garde_les_nuits_passees_dehors…` |
| Faible 8 | documenté | La contrainte `ent_sortie_acompte_fournisseur_annule_par_chk` reste `NOT VALID` (les acomptes déjà annulés n'ont pas d'auteur connu, jamais inventé) ; de même `ent_sortie_avoir_propose_collective_auteur_chk`. Elles valent pour toute écriture nouvelle ou modifiée. | `m4_les_declencheurs_de_655…` |
| Faible 9 | **traité** (2026-11-27, transverse « individu ») | La règle 3 repose sur `user_id`, pas sur la personne physique (antérieur à ce lot) : un même individu à deux comptes n'était pas reconnu. Désormais jugée sur l'individu (migrations 279 et 659), avec la seconde validation, le contournement et le bordereau : voir `TRANSVERSE_INDIVIDU.md` §9. | `tests_individu_voyages.rs` |
| Compteur | corrigé | `Action::TOUTES` vaut **282** (`ent-auth/src/actions.rs`) ; l'annonce « 271 » était fausse. | — |

### Choix prudents (D-069, aucune question posée)
1. Une demande de contournement vaut 30 jours ; une autorisation, 30 jours depuis l'autorisation ; la décision se prend sur une demande de moins de 30 jours.
2. Plafond : en euros, par écriture (montant d'un acompte ; montant payé, ou montant dû si non précisé). Un contournement couvre **une seule** écriture (usage unique).
3. Habilitation en base = rôle de l'établissement (`admin`, `direction`, `compta`) : les délégations et casquettes de l'application ne sont pas relues en base (plus strict que l'application, jamais plus large).
4. Tolérance de 7 jours pour `rentree_le`.

### Compteurs (règle 28)
| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | **282**, inchangé |
| Tables avec RLS (`tests_rls.rs`) | inchangé par ce passage (+2 déjà comptées pour 656) |
| Registre des données | inchangé (+2 déjà comptées) ; colonnes ajoutées : `ent_sortie_argent_contournement.{cible_id, montant_max_centimes, prestataire_id}`, `ent_sortie_avoir_propose.propose_par_user_id`, `ent_sortie_argent_refus.plafond_atteint` |
| Migrations | 656 corrigée en place, en fin de tableau ; `MIGRATION_TAG` inchangé (3808) |
| Routes | inchangées (+3 déjà comptées) ; corps de `POST …/argent/contournements` : `cible_id`, `montant_max`, `prestataire_id` |
| Tests | `tests_voyages_v3` : +2 (portée et gardes de base ; rattrapage sous RLS stricte), test de contournement et de plafond adaptés |

### Vérifications et résultat (correctifs)
- Local (PostgreSQL 16 **neuve** `ent_v3b_test`, `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, `--locked`, un seul `CARGO_TARGET_DIR`, à 2 threads) : `ent-db` lib (labo), filtres `tests_voyages`, `tests_internat`, `tests_rls::`, `registre_donnees::`, `migrate::`, `garde_search_path`, `regle_38` : **446 verts, 0 échec** ; `ent-api` (labo) tests voyages : **5 verts** (HTTP : demande sans portée = 422). La suite complète d'`ent-db` n'a pas été rejouée en local : c'est le rôle de la CI.
- clippy 1.99 sur les 3 commandes de la CI : **0 avertissement** ; rustfmt 1.99 ciblé sur les quatre fichiers propres au départ (les trois autres touchés, `voyages_argent_flux.rs`, `voyages_argent_repo.rs`, `tests_rls_stricte.rs`, `voyages_argent.rs`, portent une dette de formatage antérieure et ne sont pas reformatés) ; `cargo metadata --locked` dans `ent-rust/web` : ok.
- Tests : aucune heure réelle ni jour UTC dans les assertions : les expirations se jouent par rapport à l'horloge de la base (`instant_base`, `now() + intervalle`), puisque la base pose seule les horodatages.
- Non touchés : Next.js, `main`, `dev`, `ui.rs`, `POINT_QUOTIDIEN`, `PILOTAGE`, `migrations_versees.txt`, base et secrets de production, `ENT`.
- **Tête de code** `125bd85a1` (`origin/ENT` fusionné en tête de travail : conflit de `migrate.rs` résolu en gardant 317 puis 656 en fin de tableau, règle 38) : CI « ENT Rust » **verte** (37 jobs : 36 réussis, 1 sauté, aucun échec ni relance). Le commit qui consigne ce résultat ne touche que la documentation. PR n° 145 laissée en **brouillon** vers `ENT`.


### Correctifs du pilote après contre-revue des correctifs V3b
Rapport : `docs/pilotage/revues/42_VOYAGES_V3B_CORRECTIFS_CONTRE_REVUE.md` (branche `ent-mc0hoe`).
- **B1** : l'habilitation exige un accès ouvert (`ent_acces_ouvert`) : un comptable suspendu, fermé ou dont l'accès est échu n'autorise plus et son autorisation ne sert plus. Test SQL direct (trois cas).
- **M1** : un acompte se crée ni payé ni annulé (23514) ; seul le créateur est évalué à la création ; le paiement passe toujours par l'autorisation « payer » et son plafond. Test SQL direct.
- **M2** (D-069) : une autorisation « annulation collective » couvre **toutes** les propositions d'avoir de la sortie pendant sa validité (l'annulation collective porte par nature sur l'ensemble des participants) ; ces avoirs restent des propositions, dont la décision est gardée. La phrase « une seule écriture » vaut pour les acomptes. Borne par versement en file.
- **En file (suites V3b, 2)** : M3 (tests de l'expiration par les déclencheurs, horodatages en UPDATE/DELETE, `portee_chk`, annulation hors cible en SQL direct, autorisatrice d'un autre établissement, restauration et idempotence du rattrapage) ; faibles 1 à 6. Une base de labo ayant appliqué l'ancienne 656 doit être recréée.

## Suites V3b, 2 (2026-11-24, branche `ent-socle-42-code-v3b-suites-2`, base `origin/ENT`, migration 657)
Liste de travail : points « en file » de `docs/pilotage/revues/42_VOYAGES_V3B_CORRECTIFS_CONTRE_REVUE.md` (branche `ent-mc0hoe`). Consigne : `docs/pilotage/consignes/42_VOYAGES_V3B_SUITES_2.md`. **Statut : implémenté et testé en local ; non validé, non prêt pilote, non prêt production.** 640–656 sont versées et **ne changent pas** (règle 25, y compris 656) ; **657** est en fin de tableau (règle 38), ses fonctions portent `SET search_path = pg_catalog, public, pg_temp` et des relations `public.` ; `migrations_versees.txt` est laissé au pilote. Aucune donnée touchée, aucune table nouvelle.

### Fait / non fait
| Réf. | Statut | Ce qui est fait | Test |
|---|---|---|---|
| **M3**, expiration à la décision et à l'usage | fait (tests) | La décision d'une demande de 31 jours est refusée par le déclencheur (29 jours : acceptée) ; le fait « utilisé » d'une autorisation de 31 jours est refusé (29 jours : accepté) ; l'écriture d'un acompte dont l'autorisation a plus de 30 jours est refusée par le déclencheur de pouvoir (29 jours : acceptée). Faits antidatés par l'horloge de la base : déclencheur d'horodatage ou de garde écarté dans une transaction de test, comme le test de rattrapage ; aucune heure réelle. | `v3b2_l_expiration_se_joue_a_la_decision_et_a_l_usage…` |
| **M3**, UPDATE / DELETE de `demande_le` et `le` | fait (tests) | Refusés en SQL direct (`42501`, ajout seul), sur les deux tables ; rien n'a bougé. | `v3b2_les_horodatages_de_contournement_ne_se_modifient…` |
| **M3**, `portee_chk` en SQL direct | fait (tests) | 13 portées incohérentes refusées (`23514`) : plafond, prestataire ou cible manquants ou en trop pour chaque action, plafond nul ; 4 portées bien formées acceptées. | `v3b2_la_contrainte_de_portee_refuse_en_sql_direct…` |
| **M3**, annulation d'acompte hors cible | fait (tests) | Annuler un autre acompte que la cible en SQL direct : `23514` ; annuler la cible : passe. | `v3b2_annulation_d_acompte_hors_cible…` |
| **M3**, autrice d'un autre établissement | fait (tests) | La comptable réelle d'un autre établissement (cadre `tests_voyages_v0d`) ne décide pas : `23514`, aucun fait écrit. | idem |
| **M3**, rattrapage | fait (tests) | Le réglage `app.etablissement_id` précédent est restauré à l'identique ; sans réglage préalable, rien ne reste posé ; second passage sans effet (même instant). Le test rejoue 656 puis **657** à la fin (même base que les autres tests). Le test existant du rattrapage rejoue aussi 657 après 656. | `v3b2_le_rattrapage_restaure_le_reglage…` |
| **M2 (revue)**, borne de l'annulation collective | fait | Fonction `ent_sortie_paye_net` (même calcul que `verses_cash`) ; un avoir « annulation collective » dont `paye_net_centimes` est nul ou dépasse l'argent net versé par ce payeur pour ce participant est refusé (`23514`) : couple sans versement (participant ou payeur), au-delà, montant nul. Le service ne propose déjà que `paye > 0` et le net : aucun flux légitime n'est touché. | `v3b2_un_avoir_collectif_ne_depasse_pas_l_argent_verse…` |
| **Faible 1** | fait | La garde de l'avoir collectif porte aussi sur `NEW.participant_id` et sur le payeur `NEW.compte_id` (et non seulement sur les participants qui ont versé) ; la décision portée par la ligne (`decide_par_user_id`, statut retenu ou écarté) est contrôlée dans la branche collective (parent de l'élève, payeur du compte : refusés ; personne libre : acceptée), sans contournement possible pour une décision. | idem |
| **Faible 4** | documenté | Le rattrapage reste limité à `etat = 'rentree'` (repli applicatif sur `modifie_le`). Une sortie **close** ne change plus rien à l'attendu ; lui poser un `rentree_le` approximatif serait trompeur (une sortie close sans rentrée réelle n'a pas d'instant de rentrée). Limite : une sortie close avant 656 puis rouverte en « rentrée » reçoit l'instant de la réouverture. À confirmer par le pilote sur une copie peuplée (aucun accès à une base réelle). | — |
| **Faible 5** | fait | **Révocation** par l'autrice : nouveau fait « revoque » (ajout seul) ; seule l'autrice de l'autorisation, tant qu'elle n'a pas servi (une seule fois) ; une autorisation révoquée n'ouvre plus rien et ne s'utilise plus (base) ; statut `revoque` dans la liste ; service `voyage_contournement_revoquer`, route `POST …/argent/contournements/{cid}/revocation` (404 / 403 comme les autres). | `v3b2_l_autrice_revoque_son_autorisation…` ; HTTP `routes_d_argent_de_v3_repondent_…` |
| **B1, à l'usage** | fait (test) | Autorisation donnée PUIS autrice suspendue : le fait « utilisé » et, si elle a déjà servi, l'écriture sont refusés tant que l'accès n'est pas rétabli ; rétabli, ils passent. | `v3b2_une_autorisation_dont_l_autrice_est_suspendue…` |
| Faibles 2, 3, 6 | documentés (revue) | Faible 2 : `rentree_le` passé arbitraire accepté en SQL direct (choix documenté, tolérance à l'avenir de 7 jours seulement). Faible 3 : une base de labo ayant appliqué l'ancienne 656 doit être recréée (empreinte modifiée) ; sans effet en production. Faible 6 : `ent_internat_attendu_calcule` : fonctions non qualifiées de la famille documentée. | — |

### Choix prudents (D-069, aucune question posée)
1. La révocation n'exige **pas** que l'autrice soit encore habilitée (retirer un pouvoir n'en donne aucun) ; elle est possible aussi sur une autorisation expirée non utilisée, jamais sur une autorisation utilisée.
2. La borne de l'avoir collectif est l'argent **net** (versements et reports reçus moins annulations et reports cédés), comme le calcul de l'application ; elle vaut à l'insertion.
3. Une sortie close n'est pas rattrapée (voir Faible 4).

### Compteurs (règle 28)
| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | **282**, inchangé |
| Tables avec RLS (`tests_rls.rs`) | inchangé (aucune table) |
| Registre des données | inchangé |
| Migrations | **+ 1** : 657 en fin de `migrate.rs` ; `MIGRATION_TAG` inchangé (3808) ; `docs/socles/MIGRATIONS.md` à jour |
| Routes | **+ 1** (`POST …/contournements/{cid}/revocation`) |
| Tests | `tests_voyages_v3` : 46 (était 38, **+ 8**) ; HTTP : assertions ajoutées (401, 404, 403, 404 de la révocation) |

### Vérifications locales (PostgreSQL 16 neuve `ent_suites2_test`, `ENT_EXIGER_BASE=1`, `--locked`, un seul `CARGO_TARGET_DIR`, 2 threads)
- `ent-db` (labo) : filtres `tests_voyages` (163), `tests_internat` (266), `tests_rls::` (9), `registre_donnees::` (14), `migrate::` (4), `garde_search_path` (3), `regle_38` (1) : **tous verts** ; `ent-api` (labo) tests voyages : 5 verts.
- clippy 1.99.0 sur les 3 commandes de la CI (`ent-lab` compris) : **0 avertissement** ; `cargo metadata --locked` dans `ent-rust/web` : ok ; rustfmt ciblé sur `tests_voyages_v3.rs` (les autres fichiers touchés ne changent pas de formatage ; `voyages_argent.rs` et `migrate.rs` portent une dette antérieure, non reformatée).
- Non touchés : Next.js, `main`, `dev`, `ui.rs`, `POINT_QUOTIDIEN`, `PILOTAGE`, `migrations_versees.txt`, base et secrets de production, `ENT`.

### Résultat (Suites V3b, 2)
Le premier push (`d06fcb4f`) n'a déclenché aucune CI : conflit avec `ENT` (318 de l'I5 en fin de tableau). `origin/ENT` fusionné (règle 28) : `migrate.rs` résolu en gardant **656, 318, puis 657** (ordre de `migrations_versees.txt`, règle 38 ; `regle_38` et `migrate::` verts). Tête de code `8ccd2f1f` : CI « ENT Rust » **verte** (https://github.com/Fl0jfk/ScolIA/actions/runs/45517877355). Le commit qui consigne ce résultat ne touche que la documentation. PR n° 153 laissée en **brouillon** vers `ENT`.

### Correctifs du pilote après contre-revue des suites V3b, 2
Rapport : `docs/pilotage/revues/42_VOYAGES_V3B_SUITES_2_CONTRE_REVUE.md` (branche `ent-mc0hoe`). Aucun élevé.
- **M-1** : la borne collective vaut pour le **cumul** : un second avoir actif (proposé, retenu, à revoir, désistement compris) pour le même (participant, compte) est refusé en base, sous le verrou de la sortie (clé de `verrou_sortie`) ; même règle que l'application. Test corrigé (cumul refusé ; gardes de personne sur un autre couple).
- **M-4** : le test de rattrapage ne rejoue plus 656 ni 657 sur la base commune ; seule la contrainte écartée est rétablie.
- **M-3** (D-069) : la portée d'un contournement collectif reste la sortie entière ; chaque couple n'a qu'un avoir actif.
- En file : verrou de sortie dans les déclencheurs de faits (M-2), faibles F-1 à F-8.
- Local : 233 tests voyages verts, clippy 1.99 vert. 657 inscrite aux migrations versées.
