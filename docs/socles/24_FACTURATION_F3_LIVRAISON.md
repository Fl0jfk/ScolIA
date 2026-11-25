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
| **M-A** | Tarif choisi **mois par mois**, au 1er jour de chaque mois dû (génération annuelle et changement de régime). Les mois voisins tarifés par les mêmes lignes forment un segment (prorata de ses mois) ; le premier segment garde la clé de source historique, les suivants `:m<AAAA-MM>`. `jour_de_l_etablissement` et `facturer_annee_eleve_au` supprimés. `tarifs_tx` / `GET /finances/tarifs` acceptent `a_la_date` (défaut : jour de l'établissement). | `f3s_ma_rentree_votee_en_juin…` (grille du 1er sept. votée en juin, élève facturé) ; `f3r_m1` : année entière 1 260 €, entrée en novembre 1 020 €, en janvier 780 €, `tarifs_tx` aux 31/08, 01/09, 31/12, 01/01, rejeu réel sans doublon (ajouté par le pilote) |
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
`Action::TOUTES` **282** (inchangé ; « 269 » était une valeur périmée, corrigée par le pilote) ; tables RLS inchangées (aucune table) ; migrations : +531 (`MIGRATION_TAG` inchangé) ; `SOURCES` **75** (inchangé). Le pilote ajoutera `531_finances_f3_suites` à `migrations_versees.txt`.

### 8.4 Vérifications
Base locale neuve `ent_f3s_test` (PostgreSQL 16), `--locked`, toolchain 1.99.0, un seul `CARGO_TARGET_DIR`. `tests_finances_f3` : 23 tests verts. `ent-db` élargi (338 tests) : 336 verts ; deux `tests_search_path_t1` ont échoué sur « statement timeout » sous charge (contention des rejeux de migration en parallèle) et passent isolés (le troisième `t1_gel_du_cahier…` est passé isolé à la 2e exécution) ; `ent-api` : `separation` 63, `recette_finances_f0` 17, `recette_securite_3` 10 (un timeout parallèle, vert isolé). Clippy 1.99.0 `-D warnings` : 3 commandes de la CI propres (`ent-lab` compris) ; `cargo metadata --locked` dans `ent-rust/web` propre ; rustfmt ciblé sur `tests_finances_f3.rs` (`finances.rs` n'était déjà pas formaté sur ENT : non reformaté) ; pas de `cargo fmt --all`, pas de `#[ignore]`. Dates fixes, aucune horloge réelle.

### 8.5 Tête et CI
Tête de code : `9cbc5542` (fusion d'`origin/ENT` : migration 493 d'ENT puis 531, conflits d'ajout en fin de liste). **CI « ENT Rust » verte** (run 45303329656, tentative 2) : à la première tentative, seul le job « Backend — clippy, compilation production, séparation L4 » avait été annulé en cours de compilation (runner), les 37 autres jobs étaient verts ; ses jobs en échec ont été relancés une fois. PR n° 139 laissée en brouillon. Le commit suivant ne touche que ce document.

## Correctifs du pilote après contre-revue des suites (2026-11-24)
Rapport complet : `docs/pilotage/revues/24_FACTURATION_F3_SUITES_CONTRE_REVUE.md` (branche `ent-mc0hoe`).
- **Élevé 1, double facturation à la régénération** : le découpage mois par mois ne regardait pas les lignes déjà posées ; une
  génération faite avec v1 seule puis rejouée après la mise en vigueur de v2 facturait janvier-juin deux fois. Désormais
  (`lignes_annuelles`), si un article de l'élève est déjà facturé pour l'année et que le découpage produit une clé nouvelle,
  **rien n'est écrit** : anomalie « le tarif a changé depuis la génération précédente … régularisez par un complément ou un
  avoir » (D-069 : la régularisation reste une décision humaine, F4). Tests dans `f3r_m1` : élève facturé avant v2 puis
  régénéré (refus, 1 200 € inchangés) ; vrai rejeu de la même génération (2 lignes, 1 260 €).
- **Moyen 1, mois sans tarif** : un mois dû sans aucun tarif applicable rend l'anomalie « aucun tarif applicable à partir du
  … » au lieu d'être gratuit en silence. Limite : un trou propre à un seul article n'est pas détecté (en file).
- Compteurs corrigés ci-dessus (`Action::TOUTES` 282, `SOURCES` 75). Autres constats : en file (rapport).
- `531_finances_f3_suites` inscrite à `migrations_versees.txt` au versement.

## Suites, 2 (consigne 24, session du 2026-11-24)

Branche `ent-socle-24-code-f3-suites-2` (partie d'`origin/ENT`), PR en brouillon. **Aucune migration** (532 non consommée) : tout est en code de service. **Statut : implémenté et testé en local ; non validé par Florian, non prêt pilote, non prêt production.** Source : contre-revue `24_FACTURATION_F3_SUITES_CONTRE_REVUE.md` (points « en file »).

### Fait

| Réf. | Correctif | Test |
|---|---|---|
| **Moyen 1 (suite)** | Un article tarifé certains mois seulement (pour un foyer payeur donné) n'est **jamais gratuit en silence** : anomalie bloquante « aucun tarif applicable pour l'article « … » à partir du JJ/MM/AAAA », rien d'écrit. Un mois sans aucune ligne de grille applicable garde le message d'ensemble (« aucun tarif applicable à partir du … ») et passe en premier. | `f3s2_premiere_grille_au_premier_octobre_sans_historique` (première grille au 01/10, sans historique : refus « à partir du 01/09/2030 ») ; `f3s2_trou_propre_a_un_article` (historique pour la contribution seule, grille au 01/10 : la demi-pension n'a aucun tarif en septembre ; un externe, lui, est facturé) |
| **Moyen 2** | Test « contributeurs » discriminant : la direction n'a créé la grille ni touché un tarif, elle ne devient contributrice que **parce qu'elle l'a proposée** ; après retour en brouillon et nouvelle proposition par la comptabilité elle reste exclue du vote (service `interdit`, SQL direct 23001 « le votant est l'auteur de la grille »). | `f3s2_proposant_sans_tarif_reste_exclu_du_vote_apres_retour_en_brouillon` |
| **a** | Alerte du second regard : **jamais l'e-mail** de l'auteur (repli « un compte sans nom ») ; **plus de date en dur** (« rentrée 2027 » retirée) ; servie seulement à qui a `parametrer` ou `voter` (un compte qui lit seulement voit les réglages, pas l'alerte). | `f3s_alerte_second_regard_…` (direction sans nom, secrétariat avec rôle délégué `facturation.lire`) ; `f3r_m7` (alerte de qualification sans date) |
| **b** | Publication d'une version `close` refusée (409) si une version plus récente de l'année est **déjà entrée en vigueur** au jour de référence : seule la version applicable la plus récente se publie. Jour de référence injectable (`publier_grille_au_jour`), jour de l'établissement sinon. | `f3s_publication_close_…` (v1 publiable au 01/07/2030, refusée au 01/01/2031 ; v2 publiable) |
| **c** | Découpage **par article et par contenu** : les mois consécutifs où un article a le même montant (par foyer payeur) forment une seule ligne, quelles que soient les versions de grille et les autres articles. Un article inchangé n'est ni coupé ni arrondi deux fois ; un article révisé a un tronçon par montant (clé `:m<AAAA-MM>` à partir du second). Les tronçons sont traités dans l'ordre (mois de début, puis code d'article) : identique à l'ancien ordre quand rien ne change. | `f3s2_article_inchange_non_coupe_quand_un_autre_change` (v2 recopie la contribution, la demi-pension passe à 500 € : 1 ligne contribution sans `:m`, 2 tronçons demi-pension) ; `f3r_m1` |
| **d** | Cache **par jour de vigueur distinct** : une requête de tarifs par version de grille (et une pour l'absence de version) au lieu d'une par mois et par élève. | `f3r_m1`, `f3s_ma_…` (résultats inchangés) |
| **e** | Commentaire de l'API `a_la_date` corrigé (grilles `en_vigueur` ou `close`, pas « votée »). | — |
| **f** | Test « les simulations ne s'effacent pas » : l'erreur attendue est précisée (23503, clés `ent_fin_simulation_regle_fk` et `ent_fin_famille_type_enfant_fk`). Constat : depuis F5 aucun déclencheur n'interdit le DELETE ; ce sont les clés étrangères des enfants qui le refusent, et la suppression passe par le service (journalisée). | `f3_simulation_effectifs_familles_types_…` |

### Règles métier non tranchées : valeur par défaut prudente (D-069)
- **Trou d'un article** : bloquant (aucune facture écrite), jamais « gratuit » ni « tarif précédent reconduit ». Corollaire à valider : un article **actif** dont le tarif disparaît d'une grille en cours d'année bloque la génération de tous les élèves concernés ; le retrait délibéré se fait en **désactivant l'article** (il sort alors de la génération).
- **Alerte du second regard** : réservée à `parametrer` / `voter` ; un compte lecteur seul n'est pas alerté (à valider : faut-il l'afficher à la comptabilité ?).
- **Version `close` supplantée** : se publie tant qu'aucune version plus récente n'est **en effet** ; une version à date future ne supplante pas l'ancienne.
- Ordre des tronçons de lignes quand plusieurs articles changent à des dates différentes : (mois de début, article) ; l'ordre de consommation d'une prise en charge d'organisme peut en différer d'un cas multi-tronçons à l'autre (cas nouveau de D-069).

### Non fait
Rien de la liste. Hors lot (inchangés) : régularisation des écarts de tarif déjà facturés (F4), écrans Leptos.

### Compteurs (règle 28)
`Action::TOUTES` **282** (inchangé) ; tables RLS inchangées ; migrations : aucune (532 libre, `migrations_versees.txt` non touché) ; `SOURCES` 75 (inchangé).

### Vérifications et CI (Suites, 2)
Base locale neuve `ent_f3t_test` (PostgreSQL 16), toolchain 1.99.0, `--locked`, un seul `CARGO_TARGET_DIR`, dates fixes (aucune horloge réelle). `tests_finances_f3` : 29 tests verts (26 + 3 nouveaux : `f3s2_*`, plus un test scindé) ; `ent-db` élargi (finances, factur, inscription, payeurs, comptes, registre, rls, conservation : 382 tests) : 379 verts, 3 échecs de parallélisme (deadlock de rejeu de migration, compteurs partagés) verts isolés. Clippy 1.99.0 `-D warnings` : 3 commandes de la CI propres (`ent-lab` compris) ; `cargo metadata --locked` dans `ent-rust/web` propre ; rustfmt ciblé sur `tests_finances_f3.rs` ; pas de `cargo fmt --all`, pas de `#[ignore]`. Non rejoués en local : `ent-api` (`separation`, `recette_*`) et `ent-web-server`, couverts par la CI.
Tête de code : `1236cea1`. **CI « ENT Rust » verte** (run 45390937393 : 41 jobs, 2 sautés comme prévu). PR n° 146 laissée en brouillon.

### Correctifs du pilote après contre-revue des suites, 2 (2026-11-24)
Rapport complet : `docs/pilotage/revues/24_F3_SUITES_2_CONTRE_REVUE.md` (branche `ent-mc0hoe`). Aucun Élevé ; garde-fou anti
double facturation intact.
- **Moyen 2 (D-069)** : seul un trou de tarif qui **suit** un mois tarifé bloque ; un trou **en tête** (service arrivé en
  cours d'année) est facturé à partir du premier mois tarifé (test
  `f3s2_trou_en_tete_d_un_article_facture_a_partir_du_premier_mois_tarife` : demi-pension d'octobre à juin, 9/10). Un mois
  sans aucun tarif (grille oubliée) reste bloquant. Constat : une grille ne peut pas être proposée avec un article actif
  sans tarif (« anomalies bloquantes ») ; un trou en queue ne naît donc que d'un article désactivé, et **désactiver un
  article n'est pas la voie de retrait** en cours d'année (les mois dus ne seraient plus facturés) : en file, une fin de
  tarif datée.
- **Moyen 1** : une régularisation de changement de régime non facturable est journalisée (`warn!`) ; en file : tâche
  d'anomalie dédiée (nature nouvelle, migration).
- En file : faibles 1 à 6 du rapport, dont la **couverture d'une prise en charge d'organisme vérifiée sur toute la période**
  du tronçon (prioritaire).
- Compteurs : `Action::TOUTES` 282, `SOURCES` 75 (inchangés).

## Suites, 3 (consigne 24, session du 2026-11-24)

Branche `ent-socle-24-code-f3-suites-3` (partie d'`origin/ENT`), PR en brouillon. **Migration 532** (réserve 532–537) placée à la fin du tableau de `migrate.rs` (règle 38) ; elle ne crée aucune fonction. `migrations_versees.txt` non touché. **Statut : implémenté et testé en local ; non validé par Florian, non prêt pilote, non prêt production.** Source : contre-revue `24_F3_SUITES_2_CONTRE_REVUE.md` (points « en file »).

### Fait

| Réf. | Correctif | Test |
|---|---|---|
| **Faible 3 (prioritaire)** | Couverture d'organisme **sur toute la période** : pour un foyer payeur de type organisme, le découpage en tronçons tient aussi compte des prises en charge actives couvrant le **premier jour de chaque mois** ; une prise qui expire en décembre coupe le tronçon (janvier-juin rouvrent `part_non_couverte`, tâche « depuis le 01/01 »), une prise qui commence en janvier coupe aussi (tâche depuis septembre). Une prise échue ne bascule jamais d'office sur le payeur suivant (E2). | `f3s3_prise_expirant_en_decembre_…` (18 000 € à l'organisme, tâche, rejeu sans doublon, avec et sans payeur suivant) ; `f3s3_prise_commencant_en_cours_d_annee` (clé `:m2031-01`) |
| **Moyen 1 (suite)** | `facturer_changement_regime` non facturable (`Ok(Err(motif))`) : en plus du `warn!`, tâche `ent_fin_tache` de nature **`regularisation_regime_non_facturee`** (migration 532), une seule ouverte par élève (idempotente), visible de qui a `engagements` ou lecture financière de l'établissement ; **close** quand la régularisation passe (rejeu de l'événement). | `f3s3_changement_de_regime_sur_un_trou_…` (service : ouverte, rejeu sans doublon, visible du secrétariat, refusée au professeur et au parent, autre établissement, contrainte SQL directe 23514, clôture à la régularisation, 2 × 45 €) |
| **Fin de tarif datée** | `ent_fin_article.retire_a_partir_du` + `retirer_article_a_la_date_for_actor` (droit `tarifs`, motif obligatoire, journal `article.retirer`, date au **1er d'un mois**, `None` rétablit). Les mois commencés avant la date restent dus, les suivants ne le sont plus ; le trou de la grille après la fin n'est pas bloquant ; une grille sans l'article se propose (contrôle de continuité ignoré pour un article retiré). Distinct de « désactiver l'article ». Ne réécrit aucune ligne déjà posée (régularisation = F4). | `f3s3_fin_de_tarif_datee` (4 × 45 €, contribution entière, élève entré en février sans l'article, levée de la fin = trou bloquant) |
| **Faible 1** | Troncature par tronçon **documentée** (commentaire du moteur) et testée : chaque tronçon et chaque réduction est arrondi au centime inférieur ; écart au prorata entier < 1 centime par tronçon, jamais en trop pour la famille. | `f3s3_troncature_par_troncon_reductions_et_deux_foyers` (50 003 : 18 000 + 30 001 ; remise 1 350 + 2 250) |
| **Faible 2** | Segmentation par **(article, foyer payeur)** : un foyer dont le tarif ne change pas n'est pas coupé quand l'autre change. | `f3s3_segmentation_par_article_et_par_foyer` |
| **Faible 4** | Le refus 409 d'une version `close` supplantée ne vaut que pour la **première** publication. | `f3s3_publication_close_deja_publiee_reste_relisible` (+ `f3s_publication_close_…` adapté) |
| **Faible 5** | Tests : clé du second tronçon, rejeu, changement de régime sur un trou, deux foyers. | voir ci-dessus |
| **Faible 6** | `.is_ok()` de l'alerte du second regard remplacé : seul `Forbidden` écarte l'alerte, une erreur technique remonte. | `f3s_alerte_second_regard_…` (inchangé, vert) |

### Règles métier non tranchées : valeur par défaut prudente (D-069)
- **Prise échue en cours de tronçon** : la part non couverte n'est facturée à personne sans décision (tâche), même s'il existe un payeur suivant ; seul le dépassement de plafond d'une prise active passe au payeur suivant (règle E2 inchangée).
- **Couverture d'un mois** : prise active couvrant le **premier jour** du mois ; une prise qui commence en cours de mois ne couvre le mois qu'à partir du suivant. À valider.
- **Régénération d'un élève déjà facturé** avant ce correctif avec une prise coupée en cours d'année : la clé du second tronçon est nouvelle, la garde « le tarif a changé » refuse (régularisation F4).
- **Fin de tarif** : premier jour d'un mois obligatoire ; article retiré = pas d'avoir automatique pour ce qui est déjà facturé.
- **Tâche de changement de régime** : une seule ouverte par élève (le détail garde le premier motif).

### Non fait
Route HTTP de la fin de tarif datée (service et type `RetraitArticleBody` seulement : aucune route ajoutée, `SOURCES` inchangé) ; écran Leptos ; régularisation des écarts déjà facturés (F4).

### Compteurs (règle 28)
`Action::TOUTES` **282** (inchangé) ; tables RLS inchangées (aucune table) ; migrations : +532 (`MIGRATION_TAG` inchangé) ; `SOURCES` **75** (inchangé). Le pilote ajoutera `532_finances_f3_suites3` à `migrations_versees.txt`.

### Vérifications
- Base locale neuve `ent_f3u_test` (PostgreSQL 16, features `labo`) : `tests_finances_f3` **35 tests verts** (28 antérieurs + 7 `f3s3_*`) ; régression ent-db élargie (`tests_finances tests_factur tests_inscription garde_search migrations rls`) : **273 passés, 0 échec**.
- Clippy 1.99.0 `-D warnings` sur les 3 commandes de la CI : 0 avertissement ; `cargo metadata --locked` (`ent-rust/web`) : OK. rustfmt ciblé sur `tests_finances_f3.rs` seulement (pas de `cargo fmt --all`). Dates fixes, aucun `#[ignore]`.
- `ent-api` et `ent-web-server` non rejoués en local : couverts par la CI.

### Tête de code et CI
- Tête de code testée : `89537860e5249670f06ee85204ca5a370f0c60ad` (PR n° 150, brouillon, vers `ENT`).
- CI « ENT Rust » sur cette tête : **verte** (run 45451407819). Le commit de documentation qui suit ne change aucun code.

## Suites, 3, correctifs (contre-revue `24_F3_SUITES_3_CONTRE_REVUE.md`, 2026-11-24)

Branche `ent-socle-24-code-f3-suites-3` (PR n° 150, brouillon, retenue), `origin/ENT` fusionné d'abord (règle 28). **Migration 532 corrigée en place** (non versée) : colonne `ent_fin_tache.evenement_id` et index d'unicité des tâches ouvertes recréé avec elle (NULL pour toutes les autres natures : comportement inchangé) ; aucune fonction créée, aucune migration de plus, `migrations_versees.txt` non touché. **Statut : implémenté et testé en local ; non validé par Florian, non prêt pilote, non prêt production.**

### Fait

| Réf. | Correctif | Tests |
|---|---|---|
| **E1 + E2 (garde par périodes)** | La garde « le tarif a changé » (clé nouvelle refusée) est remplacée par `garde_periodes` (`finances.rs`), fondée sur le **chevauchement des périodes** et le **montant** des lignes déjà posées pour (élève, article, foyer), réductions et `:suivant:` compris (le groupe d'un foyer = sa ligne principale et ses `:suivant:`, un même mois étant porté par l'une ou l'autre). Période **disjointe** → facturable telle quelle (renouvellement par une nouvelle prise). **Même clé, même période** → rejeu (ligne laissée, la base l'ignore ; montant plus fort refusé explicitement, `:suivant:` jamais comparé car il suit le restant des plafonds). **Même clé, période différente** ou période en partie couverte (prolongation) → seuls les **mois manquants** sont facturés, par suites ininterrompues, sous une clé nouvelle `:c<AAAA-MM>` ; aucune ligne posée n'est refacturée. Mois déjà couverts à un **autre tarif** (au mois, aux arrondis près) → **refus explicite** (« a changé depuis la génération précédente pour des mois déjà facturés… régularisez par un complément ou un avoir », F4), jamais d'ignorance silencieuse. | `f3s4_e1_prolongation_de_prise_puis_rejeu_facture_les_six_mois_manquants` (180 € + 270 €, rejeu et double rejeu inchangés) ; `f3s4_e2_renouvellement_par_une_nouvelle_prise_est_facturable` (180 € + 270 €, clé `:m2031-01`, rejeu inchangé) ; `f3r_m1` (changement de tarif sur des mois facturés : refus, 1 200 € inchangés) ; `f3s3_prise_*` (rejeu sans modification) |
| **M1** | `prises_couvrantes` : statut `active` **ou** `echue` ; seule `annulee` est exclue. La date borne (`du ≤ jour ≤ au`). `e2_m1_organisme_…` (suite F2) corrigé : sa prise « échue » avait été forcée sur une période encore valide ; elle a maintenant une période révolue. | `f3s4_m1_prise_echue_valide_pour_ses_mois_annulee_exclue` (alerte passée avant la génération : 180 € à l'organisme ; prise annulée : 0 et tâche) |
| **M2** | Tâche `regularisation_regime_non_facturee` liée à l'**événement** (`evenement_id`) ; `ouvrir_tache_evenement` / `resoudre_taches_evenement` ; une tâche ouverte par (élève, événement) ; close uniquement par la régularisation de **son** événement. | `f3s4_m2_tache_de_regime_close_par_la_regularisation_de_son_evenement` (A échoue, B réussit : la tâche de A reste ouverte, seul le rejeu de A la ferme) ; `f3s3_changement_de_regime_…` (inchangé, vert) |
| **M3** | Remise arrondie **vers le haut** (`finances_calcul::remise_haute`, bornée au montant), en faveur de la famille ; commentaire du moteur, doc et test alignés (le prorata d'un tronçon reste tronqué). | test unitaire `taux_et_regimes` ; `f3s3_troncature_…` (remises 1 350 + 2 251) |
| **M4** | Test de cloisonnement SQL discriminant : pool du **rôle applicatif strict** (non propriétaire, sans BYPASSRLS), contexte de l'autre établissement → **0** ligne, contexte propre → 1 (contrôle positif). L'ancien `let _ = autre;` est supprimé. | `f3s4_m4_cloisonnement_sql_des_taches_sous_le_role_strict` |
| **Faible 1** | Contrôles de proposition : avertissement non bloquant `article_retire_tarife` quand une grille tarifie un article à fin de tarif datée (la date ne porte pas d'année, elle peut être périmée). | `f3s4_faible1_avertissement_grille_qui_tarifie_un_article_retire` |
| **Faible 2** | Fin de tarif : retrait au 01/03 avec trou de grille dès janvier = bloquant (message sur le 01/01/2031, rien d'écrit) ; tous les articles retirés dès septembre = refus explicite, rien d'écrit. | `f3s4_faible2_fin_de_tarif_au_premier_mars_…` |
| **Faible 4** | Plafond consommé au rejeu en deux tronçons avec payeur suivant (plafond 400 € : 180 € puis 220 € à l'organisme, 80 € au payeur suivant) : rejeu et double rejeu sans aucune ligne de plus ; les lignes `:suivant:` du premier tronçon, reformées à neuf, sont couvertes par le principal et écartées. | `f3s4_faible4_plafond_consomme_au_rejeu_en_deux_troncons` |

### Règles métier non tranchées : valeur par défaut prudente (D-069)
- **Faible 3** (prise commençant le 15 du mois) : un mois n'est couvert que si la prise est valide le **premier jour** ; la prise commençant le 15 ne couvre le mois qu'à partir du mois suivant (part non couverte → tâche). Règle prudente à valider.
- **Prolongation** : les mois manquants sont facturés au tarif d'aujourd'hui (prorata de la ligne générée, arrondi inférieur ; remises arrondies vers le haut) ; si le tarif des mois déjà posés diffère au-delà des arrondis (3 centimes par mois cumulés), refus explicite plutôt que complément (régularisation = F4).
- **Montant plus fort à clé et période identiques** (plafond relevé, par exemple) : refus explicite ; plus faible : rejeu (plafond consommé).
- **Lignes sans période** (anciennes) : traitées comme un rejeu.
- **Faible 5** (détail de tâche figé au premier motif) : assumé ; désormais un détail par événement.
- La réactivation d'une prise échue exige une nouvelle fin postérieure à l'horloge réelle (`aujourd_hui()`) : le test utilise 2031, aucune dépendance au jour d'exécution jusque-là.

### Non fait
Route HTTP de la fin de tarif datée et écrans Leptos (hors lot) ; régularisation des écarts déjà facturés (F4).

### Compteurs (règle 28)
`Action::TOUTES` **282** (inchangé) ; tables RLS inchangées (aucune table) ; migrations : +532 (`MIGRATION_TAG` inchangé) ; `SOURCES` **75** (inchangé). Le pilote ajoutera `532_finances_f3_suites3` à `migrations_versees.txt`.

### Vérifications
Base locale neuve `ent_f3v_test` (PostgreSQL 16, `ENT_ENV=test`, features `labo`), `--locked`, toolchain 1.99.0, un seul `CARGO_TARGET_DIR`, dates fixes. `tests_finances_f3` : **42 tests verts** (35 + 7 `f3s4_*`) ; régression ent-db (`finances factur garde_search migrations rls payeurs comptes`) : **197 passés, 0 échec** ; `tests_finances_f2::e2_m1_…` adapté (voir M1) et vert. Hors périmètre : deux tests d'inscriptions (`i4_refus_bloque_…`, `i4_reponse_en_ligne_sans_compte`) échouent en local sur « aucun lien d'accord envoyé » (courriel de démonstration, sans lien avec la facturation) ; à confirmer par la CI. Clippy 1.99.0 `-D warnings` sur les 3 commandes de la CI (`ent-lab` compris) : 0 avertissement ; `cargo metadata --locked` (`ent-rust/web`) : OK. rustfmt ciblé uniquement ; pas de `cargo fmt --all`, pas de `#[ignore]`.

### Tête de code et CI
Tête de code : `9d6ae8e2` (fusion d'`origin/ENT` ; migration 532 replacée en fin de tableau après 656). CI « ENT Rust » : **verte** (run 45522239612). Tests `finances` après fusion : 93 passés, 0 échec. PR 150 reste en brouillon.

### Correctifs du pilote après contre-revue des correctifs des suites 3
Rapport : `docs/pilotage/revues/24_F3_SUITES_3_CORRECTIFS_CONTRE_REVUE.md` (branche `ent-mc0hoe`). Élevés E1 et E2 **fermés**.
- **M2** : une clé nouvelle n'est jamais posée à côté d'une ligne ancienne **sans période** du même groupe (article, foyer) : refus explicite (« déjà facturé sans période »), rien n'est facturé deux fois. Test `f3s4_pilote_ligne_ancienne_sans_periode_jamais_doublee`.
- En file (suites 4) : payeur suivant désigné après coup (M1), clôture de la tâche `part_non_couverte` (M3), faux refus « le tarif a changé » (M4), faibles 5, 7, 8, 9.
- Local : 94 tests `finances` verts, clippy 1.99 vert. 532 inscrite aux migrations versées.

## Suites, 4 (consigne 24, session du 2026-11-25)

Branche `ent-socle-24-code-f3-suites-4` (partie d'`origin/ent-socle-24-code-f3-suites-3`, PR 150), `origin/ENT` fusionné avant le push (règle 28), PR en brouillon vers `ENT`. **Aucune migration** (533 non consommée, `migrations_versees.txt` non touché) : tout est en code de service. **Statut : implémenté et testé en local ; non validé par Florian, non prêt pilote, non prêt production.** Source : contre-revue `24_F3_SUITES_3_CORRECTIFS_CONTRE_REVUE.md` (points « en file »).

### Fait

| Réf. | Correctif | Test |
|---|---|---|
| **Moyen 1 (prioritaire), payeur suivant désigné après coup** | `restant_prise` ne compte plus les lignes que **la génération en cours reforme** (même préfixe `annuel:`/`regime:` + élève, mêmes articles) : la génération recalcule le partage plafonné à neuf et consomme le plafond dans l'ordre des tronçons. Plafond 400 € sur 450 € dus : 400 € à l'organisme, 50 € non couverts ; un payeur suivant désigné ensuite reçoit **50 €, une seule fois** (clé `:suivant:` d'origine). Les mois déjà posés mais **insuffisants** (cas où la ligne principale couvre toute la période) sont complétés par la garde (`completer_plafonnee` : manque = dû × mois couverts / mois − déjà posé), jamais écartés comme « couverts ». | `f3s5_m1_payeur_suivant_designe_apres_coup_recoit_la_part_non_couverte_une_fois` (400 € / 0 puis 400 € / 50 €, tâche close, rejeu et double rejeu inchangés) ; `f3s4_faible4_…` (inchangé, vert) |
| **Moyen 3** | La tâche `part_non_couverte` est **close** quand le tronçon dont elle porte la date `depuis` est entièrement couvert (prise renouvelée ou prolongée) ou porté par un payeur suivant (`resoudre_taches_periode`). Les tâches des autres périodes restent ouvertes. | `f3s5_m3_tache_part_non_couverte_close_par_la_prolongation_et_le_renouvellement` (E1 : 450 €, E2 : 450 €, tâche 1 → 0) ; assertions « tâche close » ajoutées à `f3s4_e1_…` et `f3s4_e2_…` |
| **Moyen 4 (a)** | Prolongation **sous plafond serré** : le plafond est cumulatif, pas mensuel ; pour les mois manquants le principal prend `min(dû de la suite, plafond non consommé)` et le reste va au payeur suivant (clé `:c<AAAA-MM>`) ; plus de faux refus « le tarif a changé ». Un tarif **réellement baissé** sur des mois déjà posés est refusé avec un motif propre (« a baissé »). | `f3s5_m4_prolongation_sous_plafond_serre_facture_le_complement` (180 € posés, prolongation, plafond 400 € : 400 € à l'organisme, 50 € à la famille, rejeu et double rejeu inchangés) |
| **Moyen 4 (b)** | Taux de remise qui change à clé et période identiques : **motif distinct** (« la remise sur l'article … a changé … taux de remise modifié »), jamais « le tarif de l'article ». | `f3s5_m4_taux_de_remise_modifie_motif_distinct` |
| **Faible 5** | Requête de préfixe de la garde : `LIKE préfixe%` (préfixe échappé), indexable ; `left()` conservé pour l'égalité exacte. | couverte par tous les tests de la garde |
| **Faible 7** | `compte_lie_a_eleve` : prise `active` **ou** `echue` (cohérent avec M1 des suites 3). | `f3s5_m1_…` (désignation du suivant) ; régression `payeurs` verte |
| **Faible 8** | Le scénario E1 du nouveau test n'appelle plus `aujourd_hui()` : la réactivation se fait en base à dates fixes (`prolonger_prise`) ; les tests historiques `f3s4_e1` gardent la voie du service (valable jusqu'au 30/06/2031). | `f3s5_m3_…`, `f3s5_m4_prolongation_…` |
| **Faible 9** | Couverture croisée **vérifiée, et fautive** : externe → demi-pension (`regime:`) puis rejeu annuel facturait la demi-pension de **toute l'année** en plus (81 000 c pour 45 000 dus). Corrigé : un rejeu `annuel:` ne pose jamais une ligne d'un (article, foyer) déjà porté par un `regime:` (le changement de régime fait foi). Sens inverse non lu : un changement de régime reste facturable après un avoir. | `f3s5_faible9_rejeu_annuel_apres_changement_de_regime_sans_double_facturation` (36 000 c avant et après) |

### Règles métier non tranchées : valeur par défaut prudente (D-069)
- **Annuel après changement de régime** : les mois **avant** le changement ne sont pas facturés au régime d'arrivée (l'événement fait foi) ; si cette lecture est fausse pour un cas (élève arrivé en demi-pension et jamais facturé), la régularisation est humaine (F4). À valider.
- **Plafond relevé** après une première génération (le principal posé est inférieur au nouveau plafond) : toujours refusé explicitement (« montant plus fort »), pas de complément automatique du principal (inchangé).
- **Complément sous plafond** : la remise d'un complément reste proportionnelle au tronçon (la remise ne suit pas la part plafonnée) ; écart d'au plus quelques centimes, en faveur de la famille.
- **Part non couverte sans payeur suivant** pendant une prolongation : non facturée, la tâche reste ouverte.
- Faible 6 (une base persistante ayant appliqué l'ancienne 532 doit être recréée) : **non traité**, sans effet en production (532 non versée à l'époque de la modification en place).

### Non fait
Écrans Leptos, route HTTP de la fin de tarif datée, régularisation des écarts déjà facturés (F4) : hors lot. Aucun test dédié à la clause « plafond relevé » ni à la remise d'un complément plafonné.

### Compteurs (règle 28)
`Action::TOUTES` **282** (inchangé) ; tables RLS inchangées (aucune table) ; migrations : **aucune** (533 libre) ; `SOURCES` **75** (inchangé).

### Vérifications
Base locale neuve `ent_f3w_test` (PostgreSQL 16, `ENT_ENV=test`, features `labo`), `--locked`, toolchain 1.99.0, un seul `CARGO_TARGET_DIR`, dates fixes. `tests_finances_f3` : **48 tests verts** (43 + 5 `f3s5_*`, dont l'assertion de tâche close ajoutée à `f3s4_e1/e2`) ; régression ent-db (`tests_finances`, `tests_payeurs`, `tests_comptes`, `garde_search`) : **91 passés, 0 échec**. Clippy 1.99.0 `-D warnings` sur les 3 commandes de la CI (`ent-lab` compris) : 0 avertissement ; `cargo metadata --locked` (`ent-rust/web`) : OK ; rustfmt ciblé sur `tests_finances_f3.rs` seulement ; pas de `cargo fmt --all`, pas de `#[ignore]`. `ent-api` et `ent-web-server` non rejoués en local : couverts par la CI. `origin/ENT` déjà à jour (fusion sans changement).

### Tête de code et CI
Tête de code testée : `69157d68` (PR n° 156, brouillon vers `ENT`). CI « ENT Rust » : **verte** (run 45541230144, après attente en file des runners). Ce commit documentaire ne change aucun code.

## Suites, 4, correctifs (contre-revue `24_F3_SUITES_4_CONTRE_REVUE.md`, 2026-11-25)

Branche `ent-socle-24-code-f3-suites-4` (PR 156, brouillon, retenue), `origin/ENT` fusionné d'abord (règle 28). **Aucune migration** (533 non consommée, `migrations_versees.txt` non touché). **Statut : implémenté et testé en local ; non validé par Florian, non prêt pilote, non prêt production.**

### Fait

| Réf. | Correctif | Tests |
|---|---|---|
| **E-1 (élevé)** | Couverture croisée `regime:` / `annuel:` **par année et par mois**. La lecture ne retient que les lignes de régime dont la **période chevauche** celle des lignes annuelles à poser ; seuls les **mois effectivement couverts** sont écartés de la ligne annuelle (les autres sont facturés sous `:c<AAAA-MM>`) ; une ligne de régime **créditée par avoir** ne couvre que les mois que l'avoir lui laisse (entièrement créditée : rien). | `f3s6_e1_changement_de_regime_en_n_puis_generation_annuelle_n_plus_1` (360 € + 450 €) ; `f3s6_e1_changement_en_cours_d_annee_puis_rejeu_annuel_chaque_mois_une_fois` (450 € en tout, rejeux neutres) ; `f3s6_e1_ligne_de_regime_creditee_par_avoir_ne_couvre_que_ce_qui_reste` (avoir total : 450 € ; avoir partiel : 450 €) |
| **M-1** | Les tâches `part_non_couverte` ne sont closes **qu'après** la garde réussie (collectées pendant le calcul, closes à la fin de `lignes_annuelles`, dans la transaction qui insère ensuite ; une erreur d'insertion annule tout). Un refus de la garde laisse la tâche ouverte. | `f3s6_m1_refus_de_la_garde_laisse_la_tache_ouverte` |
| **M-2** | Les lignes d'une prise **plafonnée** passent toujours par `completer_plafonnee` (jamais refusées comme « le tarif a changé »). Plafond **relevé** : le principal prend le plafond libre en complément (clé `:c<premier mois libre>`), le reste va au payeur suivant. Plafond **abaissé** sous le facturé : rien n'est refacturé ni crédité, une tâche `part_non_couverte` le signale. | `f3s6_m2_plafond_releve_facture_le_complement_puis_abaisse_signale` (400 → 450 € puis 300 €, rejeux neutres) |
| **F-1** | Requête de préfixe **une par préfixe** (motif constant pour le planificateur, plus de jointure sur `unnest`). | `f3s6_f1_lecture_par_prefixe_bornee_par_l_index_unique` (EXPLAIN) |
| **F-2** | `restant_prise` n'exclut les lignes propres que si leur période tombe dans la **période générée** : les lignes hors période reformée comptent dans le plafond. | régression `f3s4_faible4`, `f3s5_m1`, `f3s5_m4` vertes |
| **F-3** | Assertion stricte du test du faible 9 : 360 € après le changement, 450 € après le rejeu annuel, rejeu et double rejeu sans ligne de plus. | `f3s5_faible9_…` |

### Règles métier non tranchées : valeur par défaut prudente (D-069)
- **Remplace** la règle « mois avant le changement non facturés » des suites 4 : un rejeu annuel facture septembre-octobre au régime d'arrivée (l'annuel porte l'année entière au régime courant, seuls les mois couverts par le régime sont écartés). Sur un élève qui n'a jamais été en demi-pension avant le changement, c'est une surfacturation possible de ces mois : **à valider**, la correction est humaine (avoir).
- **Avoir partiel d'une ligne de régime** : mois conservés = part non créditée arrondie au mois supérieur (jamais de double facturation).
- **Ligne de régime sans période** : ne couvre rien (n'existe pas en pratique).
- **Plafond relevé** : le complément va d'abord à l'organisme (plafond libre), puis au payeur suivant. **Plafond abaissé** : jamais de remboursement automatique, tâche seulement. Une hausse de tarif sur une prise plafonnée non bridée est facturée en complément à l'organisme (plafond libre) au lieu d'être refusée.
- **F-1** : sous une collation autre que `C`, le `LIKE` n'est pas servi par la borne de plage de l'index unique ; la lecture reste bornée par l'établissement et le préfixe. Un index `text_pattern_ops` demanderait la migration 533 : non faite, à décider.
- **F-2** : une ligne propre dans la période générée mais que la génération ne reforme plus (prise annulée) reste exclue du restant : limite assumée.

### Non fait
Index `text_pattern_ops` (migration 533) ; régularisation des écarts déjà facturés (F4) ; écrans Leptos.

### Compteurs (règle 28)
`Action::TOUTES` **282**, tables RLS, `SOURCES` **75**, migrations : inchangés.

### Vérifications
Base locale neuve `ent_ci_test` (PostgreSQL 16, `ENT_ENV=test`, features `labo`), `--locked`, toolchain 1.99.0, un seul `CARGO_TARGET_DIR`, dates fixes. `f3s6_*` : **6 tests verts** ; régression ent-db (`tests_finances tests_payeurs tests_comptes tests_factur garde_search migrations`) : **105 passés, 0 échec**. Clippy 1.99.0 `-D warnings` sur les 3 commandes de la CI, rustfmt ciblé, `cargo metadata --locked` (`web`) : voir ci-dessous. Pas de `cargo fmt --all`, pas de `#[ignore]`.

### Tête de code et CI
(à compléter)
