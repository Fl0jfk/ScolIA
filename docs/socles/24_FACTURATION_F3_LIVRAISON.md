# Socle 24 « Finances » — lot F3 : catalogue qualifié, grilles votées, simulateur — livraison

Branche `ent-socle-24-code-suite` (partie de `origin/ENT` 02659006, fusionnée avec `origin/ENT` avant la CI finale : voir §6). Conception : `docs/socles/24_FINANCES.md` (R-F9, R-F10, R-F23, FD-17, FD-19, FAC-103 à FAC-129, FAC-196, MIG-6, §10, §23) lue par `git show origin/ent-socle-finances-conception:docs/socles/24_FINANCES.md`. Livraisons précédentes : `24_FINANCES_F1_F2_LIVRAISON.md`. **Statut : implémenté et testé en local ; non validé par Florian, non prêt pilote, non prêt production.**

Lot suivant non fait lu dans la conception (§25) : **F3** « catalogue, grilles, nature légale, TVA » (F0, F1, F2 livrés ; F4 « engagements et génération » vient ensuite).

## 1. Livré

- **Articles qualifiés (R-F9, FD-17, FAC-103 à FAC-106)** : nature légale (10 valeurs fermées, R. 442-48), régime de TVA (`exonere_261_4_4`, `taxable` avec taux 0,01–25 %, `hors_champ`, `pour_compte_tiers`), unité, mode (forfait / réel), remboursable, soumis aux réductions et aux remises d'ordre. **20 modèles ScolIA** (D-023 : contribution, demi-pension forfait et au repas, internat, études, garderie, sorties, voyages, fournitures, frais, APEL et assurance « pour compte de tiers », caution « dépôt remboursable », location taxable avec mise en garde…) copiés et renommés par l'établissement (D-022) ; un article créé depuis un modèle est **déjà qualifié**. Usage d'un article (lignes et montants nets par année, sans personne).
- **MIG-6** : les articles existants **ne sont jamais modifiés d'office** ; le catalogue les présente « non qualifiés » avec une **proposition par règle** (catégorie → nature, TVA, unité, avec une phrase d'explication), que la comptabilité valide ou corrige. Une fois qualifiée, la nature d'un article qui porte des lignes ne change plus et la qualification ne se retire pas (déclencheur de la 523, FAC-104). Quand l'établissement **exige** la qualification (réglage), une pièce qui porte un article non qualifié ne s'émet pas (refus par pièce, les autres du lot passent) et un article ne se crée plus « à l'ancienne ».
- **Grilles versionnées (R-F10, FD-19, FAC-116 à FAC-129)** : brouillon → proposée → votée → en vigueur → close (+ abandonnée, jamais supprimée). Création vide, par **copie** d'une version (année N+1 en copiant N) ou par **reprise des tarifs historiques** de l'année (MIG-6). Tarif par défaut (un par article et par version, sans borne haute), tranches de quotient (sans trou ni chevauchement, de 0 à l'infini ; `EXCLUDE USING gist` en base), **hausse** en pourcentage ou en montant avec arrondi à l'euro ou à 5 € (aperçu avant/après sans écriture, par article ou pour tous), **contrôles avant proposition** (grille vide, doublon, tranches manquantes, tarif hors tranche, tarif par défaut manquant ou incohérent, article non qualifié ; avertissement sur montant à 0 €). Retour en brouillon motivé, abandon motivé, vote (date, référence, pièce de la délibération vérifiée dans l'établissement), mise en vigueur à une date (la version précédente de l'année est close ; **une seule en vigueur** par année, index unique), **correction par nouvelle version** (copie en brouillon, motif obligatoire, effet rétroactif seulement *marqué*), publication de la grille (libellés et montants seulement, date posée une fois, idempotente).
- **Gel par la base, pas seulement par le service** : une version non brouillon ne change plus (déclencheur) ; ses tarifs et ses tranches non plus (UPDATE, DELETE, INSERT refusés) ; transitions limitées à celles du cycle ; une grille ne se supprime ni ne se vide ; un tarif historique ne se modifie plus (seule la désactivation reste, FD-19) ; un tarif historique ne se saisit plus sur une année qui a une grille votée ; `ent_fin_reduction_eleve` passe de `CASCADE` à `RESTRICT` (FD-19).
- **La génération facture la grille en vigueur** : `facturer_annee_eleve` ne lit que les tarifs de la version en vigueur de l'année ; sans version en vigueur, seuls les tarifs historiques comptent (jamais un brouillon ni une grille proposée). Les pièces déjà générées ne sont **pas** réécrites.
- **Séparation des tâches (R-F23)** : préparer et proposer = `facturation.tarifs` (comptabilité, direction) ; **voter** et **corriger** = `facturation.voter` (direction seule, la comptabilité ne vote pas ses tarifs) ; **publier** = `facturation.publier` ; et, par défaut, **celui qui a proposé la grille ne l'enregistre pas comme votée** (« second regard », désactivable par l'établissement pour une direction unique, geste journalisé).
- **Simulateur (FAC-121 à FAC-124)** : scénarios (version de base, règles de hausse par article ou générales, effectifs prévus par niveau et régime — à défaut, **effectifs actuels** dénombrés, origine indiquée), produit annuel attendu avant/après par article, **familles types fictives** (enfants par niveau et régime, tranche de quotient, fratrie du rang N comme dans la génération réelle), effet d'une grille sur les familles types sans hausse, **comparaison** de deux scénarios, **export** pour le conseil d'administration (tableau + CSV protégé contre les formules). Aucun nom, aucun montant par personne.
- **État des contributions (FAC-196)** : pièces émises par nature légale (facturé, avoirs, net, lignes), CSV d'inspection ; les articles non qualifiés ressortent « non_qualifie ».
- **Journal et événements** : chaque geste écrit une ligne du journal financier dans sa transaction ; événements `facturation.article_qualifie`, `grille_creee`, `grille_proposee`, `grille_votee`, `grille_en_vigueur`, `grille_publiee`, `simulation_calculee` avec **liste blanche de clés par type** (identifiants, année, numéro, dates ; aucun montant, libellé ni nom). Aucune donnée bancaire ne passe par ce lot.
- **Routes** : 33 opérations sur 27 chemins (`ent-api/src/finances_grilles.rs`, `/finances/catalogue/*`, `/finances/grilles/*`, `/finances/familles-types`, `/finances/simulations/*`, `/finances/contributions`) ; types `ent-types/src/finances_grilles.rs` ; commandes `ent-db/src/facturation_grilles.rs` (ajouté à `FICHIERS_DU_SERVICE`, garde R-F1 respectée : aucune écriture financière hors du service).

## 2. Règles métier incertaines : paramètres à valeur prudente (D-069)

| Réglage | Valeur par défaut | Pourquoi |
|---|---|---|
| `exiger_qualification_articles` | **faux** pour les établissements existants, à activer par la comptabilité (`POST /finances/grilles/reglages`) | MIG-6 prévoit « qualifier d'abord, exiger ensuite » : l'activer d'office bloquerait toute émission d'un établissement dont le catalogue n'a pas encore été validé. Le réglage annonce le nombre d'articles à qualifier. **À activer avant la bascule.** |
| `grille_second_regard` | **vrai** | R-F23 : proposer ≠ voter. |
| Proposition de nature d'un article « scolarité » | `contribution_r442_48_2` (annuités et matériel) | 1° (enseignement religieux) ou 2° (amortissement) selon le contrat d'association : la proposition le dit et la comptabilité tranche. |
| TVA d'une prestation facultative | `exonere_261_4_4` avec mention « à confirmer par l'expert-comptable » | le BOFiP ne traite pas les voyages ni le périscolaire [NV, §5.3] ; un régime `taxable` est possible et signalé au journal (Q-F18). |
| Spécificité d'un tarif | niveau > régime > tranche (règle existante de `choisir_tarif`, inchangée) | un tarif propre à un niveau l'emporte sur un tarif de tranche de quotient : le simulateur le montre ; à confirmer par la direction (Q ci-dessous). |

## 3. Compteurs (règle 28, recalculés après fusion d'`origin/ENT`)

| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | 269 (inchangé : aucune action nouvelle, `tarifs`, `voter`, `simuler`, `publier`, `parametrer` de F1 suffisent) |
| Tables avec RLS (`tests_rls.rs`) | base ENT + **7** (`ent_fin_grille_version`, `_tranche_qf`, `_famille_type`, `_famille_type_enfant`, `_simulation`, `_simulation_regle`, `_simulation_effectif`) |
| Migrations | **523** (plage 520–539 du socle 24, vérifiée libre sur toutes les branches distantes ; 3320–3329 non utilisée) ; `MIGRATION_TAG` inchangé |
| `SOURCES` / `NOMS_SOURCES` (séparation) | 70 (69 + `finances_grilles.rs`) |
| Registre des données | +7 tables (non personnelles : paramétrage, scénarios sans nom) |
| Événements | +7 types, liste blanche par type |
| Conservation | catégorie `fin.piece` (10 ans) par défaut du domaine « finances » ; voir écart 4 |

## 4. Écarts et décisions à valider

1. **Qualification non exigée par défaut** (§2) : à activer par la comptabilité avant la bascule ; le test d'émission couvre les deux états.
2. **Effet rétroactif d'une correction de grille** : seulement **marqué** (`effet_retroactif`, événement `grille_en_vigueur`). Les compléments et avoirs correspondants se **proposent** au lot F4 (recalcul avant émission) ; aucune pièce émise n'est touchée ici (R-F2).
3. **Publication** : le service rend la grille publiable (libellés, montants, tranches, date de publication posée une fois) ; l'**affichage public et le simulateur public** (socle 17, T-18) ne sont pas branchés. Aucune route publique n'est ajoutée.
4. **Durée de conservation des scénarios** : les tables de simulation et de familles types tombent dans `fin.piece` (10 ans) faute de catégorie dédiée ; elles ne contiennent aucune donnée personnelle ; une catégorie plus courte est une décision RGPD à prendre (socle 27).
5. **Effectifs actuels** : dénombrés sur `ent_eleve` (élèves actifs, niveau de la classe, régime) ; le **socle 3** ne fournit pas encore le port « effectifs prévus ». Les effectifs prévus se saisissent dans le scénario.
6. **Version `abandonnée`** : ajoutée aux cinq statuts de §10.2 pour qu'un brouillon écarté reste visible sans être supprimé.
7. **Ordre de spécificité des tarifs** (niveau > régime > tranche) : règle d'avant ce lot, conservée ; un tarif par niveau masque donc un tarif par tranche pour ce niveau. À confirmer par la direction.
8. **Pièce de la délibération** : seule l'existence du document dans l'établissement est contrôlée (`ent_doc_document`) ; la **numérotation et l'empreinte** relèvent du socle 7 (lot F5).

## 5. Non fait (hors lot, tracé)

Écrans Leptos du catalogue, des grilles et du simulateur ; recalcul des brouillons et propositions de compléments / avoirs (F4) ; pièces numérotées par série et document figé (F5) ; export comptable (non touché : les écritures comptables ne se modifient pas, ce lot n'en écrit aucune) ; gel au décès (FAC-209 / CF-10, **à passer avant toute relance automatique**) ; réductions en montant, cumul, plafond et attributions datées (F8) ; imputation des aides (F8) ; IA (F12).

## 6. Vérifications

Base locale **neuve** `ent_f3_test` (PostgreSQL 16, `ENT_ENV=test`, `ENT_EXIGER_BASE=1`), un seul `CARGO_TARGET_DIR`, toolchain 1.99.0, `--locked`. Aucune base ni donnée de production, aucune donnée réelle, aucun secret.

- **Formatage ciblé** : `rustfmt` sur les quatre fichiers nouveaux ; `cargo fmt -p ent-types --check` propre ; **aucun `cargo fmt --all`**. `cargo metadata --locked` dans `ent-rust/web` : propre (aucune crate ajoutée au graphe).
- **Clippy 1.99.0 `-D warnings`** : propre sur les trois passes de la CI (workspace, workspace avec `ent-api/labo,ent-lab/labo`, `-p ent-types`). **Zéro avertissement** rustc et clippy dans les lignes ajoutées ; aucun `#[allow]` ajouté sauf `too_many_arguments` sur la fabrique de modèles (liste statique), avec sa raison dans le code.
- **Tests du lot** (`tests_finances_f3`, 8 scénarios, base locale) : catalogue et nature figée, émission sous qualification exigée, cycle de vie complet (gel par la base, génération sur la grille en vigueur, correction, publication, événements neutres), second regard / retour en brouillon / abandon, hausse / tranches / contrôles / reprise de l'historique, simulation / familles types / comparaison / export, état des contributions, isolation entre établissements (second jeu de labo, rien n'a changé en base). Plus 7 tests unitaires purs (arrondi, hausse, tranches, contrôles, simulation, CSV, modèles). Les dates sont fixes (année 2030-2031, vote le 20 juin 2030) : **aucune dépendance à l'heure ni au jour UTC** (règle 33).
- **Non-régression** : 193 tests `ent-db` (finances F0 à F3, comptes F2, registre, conservation, RLS dont le compte de tables, rôles, charges d'événements), gardes `garde_finances_ecritures`, `garde_inscriptions_ecritures`, `isolation_sql_statique`, `jour_utc_interdit` ; `ent-api` : **test d'or de l'historique** (`or_facturation_historique`, identique), `recette_finances_f0` (17), `recette_securite_3` (10), `separation` (58, dont `domaine_finances_grilles`, 32 routes F3 refusées sur les identifiants d'un autre établissement, et la matrice du double membre).
- **Pas de `#[ignore]`**, aucun test désactivé.
- **CI** : voir le message au pilote (tête finale et état), après fusion d'`origin/ENT` (règle 36).

## 7. Correctifs de revue (D-069, session du 2026-11-22)

Revue du pilote sur la PR n° 118 (brouillon) : la migration **523 n'étant pas versée, elle est modifiée** ; aucune autre migration versée ne change. Branche partie de `5f12c34a`, fusionnée avec `origin/ENT` (compteurs recalculés depuis la base commune : voir plus bas) ; `url` 2.5.9 appliqué comme la PR 121 (`cargo update -p url --precise 2.5.9` dans `ent-rust` et `ent-rust/web`, commit séparé, rien d'autre dans les `Cargo.lock` hormis `form_urlencoded` 1.2.3 tiré par le même `--precise`).

**Statut : implémenté et testé en local ; non validé par Florian, non prêt pilote, non prêt production.**

### 7.1 Fait

| Réf. | Correctif | Où | Test |
|---|---|---|---|
| **M1** | Une grille ne tarifie qu'à partir de son `en_vigueur_du` ; la version précédente (close) reste applicable jusqu'à la veille ; sans version applicable, historique seul. Génération et liste des tarifs lisent la grille **applicable au jour** (jour de l'établissement, `ent_aujourdhui_etab`) ; `facturer_annee_eleve_au` fixe le jour pour les tests. Vote ≤ aujourd'hui (jour de l'établissement) ; vigueur ≥ vote, dans l'année scolaire visée (1er sept. → 31 août), jamais avant une version déjà appliquée ; CHECK `vigueur ≥ vote` en base. | `finances.rs` (`sql_tarif_applicable`, `lignes_annuelles`, `tarifs_tx`), GR `voter_grille_au_jour`, `mettre_en_vigueur_for_actor`, MIG | `f3r_m1_*` : historique en juillet, v1 en septembre et décembre, v2 en janvier ; vote futur, vigueur avant le vote, hors année : refusés |
| **M2** | Second regard = **autre personne** que l'auteur réel (créateur, proposant, auteurs et modificateurs de tarifs — colonne `ent_fin_tarif.modifie_par`) ; vérifié aussi par déclencheur. Il ne se désactive jamais pendant qu'une grille est proposée (service **et** déclencheur sur `ent_fin_parametres`) ; la désactivation exige `voter` **en plus** de `parametrer`, est journalisée (`grille.second_regard_desactive`) et signalée par une alerte du réglage (qui, quand). | GR `auteurs_grille_tx`, `definir_reglages_grilles_for_actor`, MIG | `f3r_m2_*` : direction rédige, comptabilité propose, direction vote → refus ; vote direct par l'auteur en base refusé ; désactivation sans `voter` refusée ; pendant une proposition refusée |
| **M3** | `ent_fin_grille_version` : plus de `RETURN NEW` qui court-circuite ; la transition ne change que ses propres colonnes (liste par transition), vides auparavant ; `en_vigueur_du`, `en_vigueur_par`, `close_le`, vote, proposition figés une fois posés ; `BEFORE INSERT` impose un brouillon vierge. | MIG | `f3r_m3_*` : une dizaine de cas SQL directs (insertion votée, transition + autre colonne, vigueur avant le vote, clôture sans date, colonnes figées, suppression, vidage) |
| **M4** | `ent_fin_tranche_qf` : contrôle OLD **et** NEW en UPDATE, `grille_version_id` et `etablissement_id` figés ; idem `ent_fin_tarif.etablissement_id`. | MIG | `f3r_m4_*` : déplacement figée→brouillon, brouillon→figée, brouillon→brouillon, changement d'établissement |
| **M5** | À la mise en vigueur, la version appliquée jusque-là passe `close` (applicable jusqu'à la veille) ; toute autre version **votée** de l'année passe **`supplantee`** (nouveau statut) et ne peut plus être mise en vigueur (service et base). | GR, MIG | `f3r_m5_*` |
| **M6** | Continuité (règle 4) : article tarifé par la version précédente (ou par l'historique à défaut) et absent de la nouvelle = **bloquant** (`article_oublie`) ; niveau/régime qui n'a plus de tarif = bloquant (`tarif_perdu`) ; article déjà facturé sans tarif = **signalé** (`article_facture_sans_tarif`). Contrôles joués à la proposition **et** rejoués à la mise en vigueur. | GR `controler_continuite`, `controles_tx` | `f3r_m6_*` (historique oublié) + test pur |
| **M7** | Qualification exigée **par défaut** (colonne `DEFAULT true` ; ligne absente = exigée) ; la migration crée la ligne de chaque établissement existant **avec la valeur transitoire fausse** ; alerte « à activer avant la rentrée 2027 » dans le réglage (lue par la direction). Une ligne sans `article_id` compte comme non qualifiée. Un article créé depuis un modèle **taxable** reste « à valider » (`a_valider`, non qualifié) tant qu'il n'est pas validé. | MIG, GR (`reglages_tx`, `alertes_reglages_tx`, `articles_non_qualifies_de_piece_tx`, `article_depuis_modele`) | `f3r_m7_*` |
| **F1** | `ent_fin_simulation_regle.article_id` : clé étrangère **composite** (établissement, article) ; `vote_document_id` : la table `ent_doc_document` est créée par la migration 3701, **postérieure** à la 523, une clé étrangère y est impossible ici → **déclencheur** à la transition de vote (document de l'établissement). | MIG | `f3r_f1_*` |
| **F2** | `23001` (RESTRICT) et `23P01` (exclusion) classés en **conflit** (409). | `pg_err.rs` | test unitaire `classification_sqlstate` |
| **F3** | `FOR SHARE` sur la lecture de la version par les déclencheurs de gel (tarifs, tranches). | MIG | (verrou ; vérifié par relecture) |
| **F4** | `TRUNCATE` bloqué sur `ent_fin_tarif` ; le recalcul d'une simulation **ajoute** une ligne à `ent_fin_simulation_calcul` (ajout seul, non modifiable) au lieu d'écraser. | MIG, GR | `f3r_m4_*`, `f3r_f1_f4_f5_*` |
| **F5** | `opt_texte` répond **422** au lieu de tronquer (tous les appelants du service financier) ; plafonds de **200** familles types et **200** simulations par établissement (409) ; suppression volontaire (`DELETE /finances/familles-types/{id}`, `DELETE /finances/simulations/{id}`, journalisée ; les déclencheurs « sans suppression » de ces tables sont retirés, le vidage reste interdit). | `finances.rs`, GR, API, MIG | `f3r_f1_f4_f5_*`, sondes de séparation sur les deux routes |
| **F6** | CSV : tabulation et retour chariot en tête neutralisés comme `= + - @`. | GR `cellule_csv` | test pur |
| **F7** | Arrondi **symétrique** des baisses (demi éloigné de zéro) ; une hausse **générale en montant** n'atteint jamais les catégories `bourse` / `reduction` (simulation et hausse de grille) sauf règle explicite par article. | GR | tests purs |
| **F8** | Effacement d'un utilisateur : le compte est **anonymisé** (ligne `ent_user` et identifiant conservés), les références d'auteur de ce lot sont **conservées** (preuve du second regard et du vote, `fin.piece` 10 ans) ; écrit dans l'en-tête de la 523. | MIG | (documentaire) |

Ordre des contrôles (règle 5) : personnel, droit, existence ; le futur d'une date de vote se contrôle **après** l'existence de la grille (la sonde de séparation attend un refus d'établissement avant une erreur de saisie). Erreurs de base classées (409 / 422, jamais 500) : les erreurs des déclencheurs utilisent `restrict_violation` (409), `check_violation` (422), `foreign_key_violation`.

### 7.2 Choix à valider

1. **Statut de la version remplacée** : la version appliquée jusque-là passe `close` (elle reste applicable avant la date de la nouvelle), et seules les autres versions **votées** passent `supplantee` : une version remplacée mais encore applicable ne peut pas être « supplantée ».
2. **Année scolaire** pour la date de vigueur : 1er septembre → 31 août (indépendant du calendrier de facturation).
3. **Établissements existants** : la migration insère leur ligne de paramètres (valeur transitoire fausse). Un établissement **créé après** la 523 sans ligne ni réglage est en « exigée » : les jeux d'essai qui créent des articles « à l'ancienne » (`tests_finances` et `tests_finances_f2` de `ent-db`, `tests_finances` de `ent-api`, `ent-web-server/tests/finances.rs`, scénarios F3) se placent explicitement dans l'état « établissement antérieur » (`etablissement_anterieur`, ou le réglage par le service). Constat de la CI du premier push (`a24201fc`) : ces deux derniers jeux d'essai, absents de mon passage local, ont échoué (tranche 15/20 et tranche 2/6) et sont corrigés dans le push suivant.
4. **Désactivation du second regard** : « signalée à la direction » = alerte persistante du réglage (visible par `lire`) et journal ; aucun nouveau type d'événement (liste blanche inchangée).
5. Une correction rédigée par la direction ne peut pas être votée par elle (second regard) : il faut une autre personne titulaire de `voter` (administrateur, autre direction).

### 7.3 Non fait

Rien de la liste n'est laissé de côté. Restent hors lot (inchangés, §5) : écrans Leptos, F4/F5 de la conception (recalcul, pièces numérotées), gel au décès, réductions en montant (F8 de la conception).

### 7.4 Compteurs (règle 28, recalculés depuis la base commune après fusion d'`origin/ENT`)

| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | 269 (inchangé : aucune action ajoutée) |
| Tables avec RLS (`tests_rls.rs`) | base ENT + **8** (7 + `ent_fin_simulation_calcul`) |
| Registre des données | +8 tables |
| Migrations | 523 modifiée (non versée) ; aucune autre ; `MIGRATION_TAG` inchangé |
| Routes F3 | 35 opérations sur 29 chemins (+2 : suppressions) |
| `SOURCES` / `NOMS_SOURCES` | 70 (inchangé) |
| Événements | inchangés (7 types F3) |

### 7.5 Vérifications

Base locale **neuve** `ent_f3r_test` (PostgreSQL 16, `ENT_ENV=test`, `ENT_EXIGER_BASE=1`), un seul `CARGO_TARGET_DIR`, toolchain 1.99.0, `--locked`. Aucune base ni donnée de production.

- `ent-db` : scénarios F3 (8 existants + 8 nouveaux `f3r_*`, dates fixes : 2030-2031, jour de référence injecté, **aucune horloge réelle ni jour UTC**), tests purs, finances F0 à F2, comptes, registre, conservation, RLS (96 + 243 tests des filtres finances / factur / rls / registre / conservation / comptes / payeurs / inscription), gardes `garde_finances_ecritures`, `garde_inscriptions_ecritures`, `isolation_sql_statique`, `jour_utc_interdit`.
- `ent-api` : `recette_finances_f0` (17), `recette_securite_3` (10), test d'or historique, `separation` (58, dont `domaine_finances_grilles` avec les deux sondes ajoutées).
- Clippy 1.99.0 `-D warnings` sur les trois passes de la CI backend et les deux passes `ent-web-server` / `ent-web-session` du frontend (`cargo check --tests` du test web : l'édition de liens locale du binaire de test échoue sur `ring`, sans lien avec le lot) ; rustfmt ciblé sur les fichiers du lot (`facturation_grilles.rs`, `tests_finances_f3.rs`, `ent-api/src/finances_grilles.rs`), `cargo fmt -p ent-types --check`, **aucun `cargo fmt --all`** ; `cargo metadata --locked` dans `ent-rust/web` propre. Aucun `#[ignore]`.
- **Tête de code et CI** : `ff16f6ec` (deux pushs : `a24201fc` rouge sur deux jeux d'essai hors de mon passage local — tranche 15/20 `ent-api tests_finances` et tranche 2/6 `ent-web-server finances`, corrigés ; puis `ff16f6ec`), **CI « ENT Rust » verte sur tous les jobs** (run 45032415707). Le commit suivant ne touche que ce document (`docs/` ne déclenche pas la CI). PR n° 118 laissée en brouillon.

## 8. Suites de la contre-revue (D-069, session du 2026-11-24)

Branche `ent-socle-24-code-f3-suites` (partie d'`origin/ENT`), PR en brouillon. Migration **531** (fin du tableau, règle 38) ; 520 à 523 inchangées. **Statut : implémenté et testé en local ; non validé par Florian, non prêt pilote, non prêt production.**

### 8.1 Fait

| Réf. | Correctif | Test |
|---|---|---|
| **M-A** | Tarif choisi **mois par mois**, au 1er jour de chaque mois dû (génération annuelle et changement de régime). Les mois voisins tarifés par les mêmes lignes forment un segment (prorata de ses mois) ; le premier segment garde la clé de source historique, les suivants `:m<AAAA-MM>`. `jour_de_l_etablissement` et `facturer_annee_eleve_au` supprimés. `tarifs_tx` / `GET /finances/tarifs` acceptent `a_la_date` (défaut : jour de l'établissement). | `f3s_ma_rentree_votee_en_juin…` (grille du 1er sept. votée en juin, élève facturé) ; `f3r_m1` : année entière 1 260 €, entrée en novembre 1 020 €, en janvier 780 €, `tarifs_tx` aux 31/08, 01/09, 31/12, 01/01, rejeu sans doublon |
| **M-B** | Égalité de date de vigueur refusée (service et déclencheur, 23514). | `f3r_m1` (service puis SQL direct, SQLSTATE et message) |
| **M-C** | Verrou `pg_advisory_xact_lock` d'établissement `fin:grille_second_regard:<étab>` dans la proposition et le réglage du second regard. | `f3s_mc_…` : deux connexions, ordre imposé par le verrou et `pg_locks`, sans `sleep` |
| **M-D** | Refus de création d'article et d'émission sous qualification exigée par défaut (messages exacts) ; `f3r_m2` vérifie SQLSTATE et message. | `f3s_md_…`, `f3r_m2` |
| 1 | Bind mort de l'INSERT de tarif supprimé. | compilation et tests F3 |
| 2 | Alerte « par <nom> le JJ/MM/AAAA à HH:MM » à l'heure de l'établissement. Le texte ne dit plus « antérieur à la 523 ». | `f3s_alerte_…` (fuseau Pacific/Auckland) |
| 3 | Une version `close` encore applicable se publie (service et base). | `f3s_publication_close_…` |
| 4 | Suppression de simulation réservée à son auteur ou à `voter` ; journal avec libellé, nombre de calculs et produits avant→après ; DELETE direct d'un calcul refusé en base (cascade depuis la simulation seulement). | `f3s_publication_close_…` |
| 5 | Plafonds (familles types, simulations) comptés sous verrou d'établissement. | `f3s_plafonds_sous_verrou` |
| 6 | Existence avant validation du corps (vote, retour en brouillon, abandon, correction). | `f3s_alerte_…` |
| 7 | Colonne `contributeurs` : tarifs retirés, tranches, hausses, proposant d'avant un retour en brouillon ; refusés au vote (service et base). | `f3s_contributeurs_…` |
| 8 | Identifiants des versions supplantées dans le journal. | `f3r_m5` |
| 9 | Année d'un tarif versionné figée par le déclencheur. | `f3s_contributeurs_…` |
| 10 | Continuité : la facturation `regime:` compte dans `facturees` (par année de la facture). | exécutée à chaque proposition ; **pas de test dédié** |
| 12 | Commentaire `tests_rls.rs` : + 8. | — |

### 8.2 Non fait, avec raison
- **10 (articles actifs seulement)** : un article désactivé qui disparaît de la grille suivante est un retrait délibéré, pas un oubli.
- **11 (effacement des utilisateurs)** : documentaire (en-tête de la 523), chemin d'effacement non touché.
- **2** : le basculement de l'exigence de qualification reste journalisé sous `grille.reglages`, sans distinction « établissement antérieur ».

### 8.3 Compteurs (règle 28)
`Action::TOUTES` 269 (inchangé) ; tables RLS inchangées (aucune table) ; migrations : +531 (`MIGRATION_TAG` inchangé) ; `SOURCES` 70. Le pilote ajoutera `531_finances_f3_suites` à `migrations_versees.txt`.

### 8.4 Vérifications
Base locale neuve `ent_f3s_test` (PostgreSQL 16), `--locked`, toolchain 1.99.0, un seul `CARGO_TARGET_DIR`. `tests_finances_f3` : 23 tests verts. `ent-db` élargi (338 tests) : 336 verts ; deux `tests_search_path_t1` ont échoué sur « statement timeout » sous charge (contention des rejeux de migration en parallèle) et passent isolés (le troisième `t1_gel_du_cahier…` est passé isolé à la 2e exécution) ; `ent-api` : `separation` 63, `recette_finances_f0` 17, `recette_securite_3` 10 (un timeout parallèle, vert isolé). Clippy 1.99.0 `-D warnings` : 3 commandes de la CI propres (`ent-lab` compris) ; `cargo metadata --locked` dans `ent-rust/web` propre ; rustfmt ciblé sur `tests_finances_f3.rs` (`finances.rs` n'était déjà pas formaté sur ENT : non reformaté) ; pas de `cargo fmt --all`, pas de `#[ignore]`. Dates fixes, aucune horloge réelle.

### 8.5 Tête et CI
Tête de code : `9cbc5542` (fusion d'`origin/ENT` : migration 493 d'ENT puis 531, conflits d'ajout en fin de liste). **CI « ENT Rust » verte** (run 45303329656, tentative 2) : à la première tentative, seul le job « Backend — clippy, compilation production, séparation L4 » avait été annulé en cours de compilation (runner), les 37 autres jobs étaient verts ; ses jobs en échec ont été relancés une fois. PR n° 139 laissée en brouillon. Le commit suivant ne touche que ce document.
