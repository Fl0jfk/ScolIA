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

Voir la fin du fichier (mise à jour avec la tête finale et la CI).
