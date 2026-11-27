Tu es une session ScolIA (Sonnet). Réponds en français. Le coordinateur, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au coordinateur et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `git show origin/ent-mc0hoe:docs/pilotage/conceptions/24_F3_REGIME_MOIS.md` : **en entier**, c'est la conception qui
    fait foi (régime du mois, régularisation, mois couverts nets, réaction, défaut 2) ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/24_F3_SUITES_4_CORRECTIFS_CONTRE_REVUE.md` (élevé préexistant, M2, M3) ;
  - `docs/socles/24_FACTURATION_F3_LIVRAISON.md` (sections « Suites, 3 », « Suites, 4 » et « Suites, 4, correctifs ») ;
  - le code : `crates/ent-db/src/finances.rs` (`lignes_annuelles`, `restant_prise`, `garde_periodes`,
    `completer_plafonnee`, `facturer_annee_eleve`, `facturer_changement_regime`, `credit_au_mois`, `credit_aux_jours`,
    `generer_annee_for_actor`), `finances_reactions.rs`, `facturation_payeurs.rs` (tâches), `scolarite.rs`
    (`ecrire_changement_attribut`), `domain-eleve/src/periode.rs` (`plan_cutover`, charge de l'événement).
- **Branche :** `ent-socle-24-code-f3-regime-mois`, créée depuis **`origin/ENT`** (règle 25 ; jamais depuis `ent-mc0hoe`).
  PR en brouillon vers `ENT`.
- **Migrations :** **aucune prévue** (historique `ent_eleve_periode`, périodes des lignes d'avoir et nature de tâche
  `regularisation_regime_non_facturee` existent). Si une migration se révèle indispensable, écris pourquoi dans la doc et
  prends **534** (533 est inscrite pour F16 ; 533–539 vérifiés libres sur les 333 branches distantes le 2026-11-26) ;
  revérifie-le (`git ls-tree` de `ent-rust/migrations` sur chaque `origin/*`) ; place-la **à la fin** du tableau de
  `migrate.rs` (règle 38) ; toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et
  qualifie ses relations `public.` ; les migrations versées (520–523, 531, 532…) ne changent **jamais** ; ne touche pas
  `migrations_versees.txt` ; mets à jour `MIGRATION_TAG` seulement si la règle du test l'exige.
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ;
  `cargo fmt --all`, `#[ignore]` ; affaiblir une assertion de montant d'un test existant pour le faire passer (un test qui
  attend une clé `regime:` peut changer de clé, jamais de montant sans justification écrite dans la doc) ; modifier
  POINT_QUOTIDIEN, PILOTAGE ou tout fichier de `docs/pilotage/` ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans
  les tests (dates fixes, `d(2030, …)`).
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt
  ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul
  `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour ; arithmétique d'argent contrôlée (`checked_*`, `i128` pour les
  produits) ; une revue `review-sonnet` du diff avant la CI, une passe de correctifs.

## Travail (dans cet ordre)
1. **Régime du mois** (conception § 2.1) : fonction `regimes_par_mois(conn, etab, eleve, mois) -> Vec<RegimeMois>`
   (`Connu(String)`, `Repli(String)` pour un élève sans aucune période, `Inconnu`), lue dans `ent_eleve_periode`
   (`kind = 'regime'`, valeur normalisée) : période en vigueur le 1er du mois, à défaut première période commencée dans le
   mois, à défaut `Repli(ent_eleve.regime)` si l'élève n'a aucune période, sinon `Inconnu`. Tests unitaires purs de la
   règle (changement le 1er, le 15, arrivée le 3, aucun historique, mois avant la première période).
2. **Annuel mois par mois** (§ 2.2) : `lignes_annuelles` prend le régime de chaque mois pour `categorie_due` et
   `choisir_tarif` ; un mois non dû par le régime n'est pas un trou de grille ; `regime_force` disparaît ; payeurs d'une
   catégorie lus au premier mois où elle est due ; mois `Inconnu` : restauration et internat non facturés, tâche
   `regularisation_regime_non_facturee` sans événement (détail du § 2.1), une seule ouverte par élève (index existant).
3. **Régularisation** (§ 2.3 et 2.4) : `regulariser_regime_annee` (verrou d'élève d'abord ; avoirs par suite de mois sous
   la clé `regime:mois:avoir:{ligne}:m{AAAA-MM}`, période propre ; jamais sur un mois `Repli` ou `Inconnu`) ; mois
   couverts nets dans `garde_periodes`, `completer_plafonnee`, `credit_au_mois`, `credit_aux_jours` (ancien format
   d'avoir de régime : `floor`, en faveur de la couverture ; sortie et absence ne découvrent rien).
4. **Réaction** (§ 2.5) : `eleve.regime_changed` n'exige plus que `eleve_id` et `date_effet` (les deux formes de charge,
   finances et ELV-10, sont traitées) ; régularisation puis annuel limité aux catégories gagnées, préfixe `annuel:{année}`,
   `depuis` factorisé avec `generer_annee_for_actor` (inscription validée, à défaut `date_entree`) ; plus aucune ligne
   `regime:` créée ; tâche de l'événement ouverte sur refus, close sur réussite (comportement actuel).
5. **Génération annuelle** (§ 2.6) : `facturer_annee_eleve` régularise avant de calculer ; avoirs comptés dans le rapport.
6. **Défaut 2** (§ 1.2 et 2.8) : fenêtre propre = période complète de la génération (variable renommée, plus d'ombrage) ;
   ligne principale plafonnée inscrite dans `dus` et transmise à la garde même à part 0 ; aucune ligne à 0 € insérée ;
   clôture de `part_non_couverte` seulement si la part non couverte est réellement portée après la garde.
7. **Couverture croisée `regime:`** conservée pour les lignes déjà posées (§ 2.7), sans nouveau code qui en crée.
8. Toute règle métier non tranchée en cours de route : valeur par défaut prudente, notée dans la doc (D-069) ; n'écris
   pas à Florian.

## Tests attendus (chiffrés ; demi-pension 450 €/an = 45 €/mois sur 10 mois, sauf mention)
- **R1, cas du rapport** : externe toute l'année (période initiale), passage en demi-pension au 1er juin écrit **sans
  réaction de facturation** (période insérée par le chemin ELV-10 ou directement), puis génération annuelle : **45 €**
  facturés (juin), pas 450 €.
- **R2, chemin ELV-10** : `changer_regime_ou_site` externe → demi-pension au 15 octobre, réaction traitée : novembre à juin
  facturés (**360 €**) ; rejeu de la réaction et de l'annuel : aucune ligne nouvelle.
- **R3, M2** : changement externe → demi-pension au 1er septembre : **450 €** (septembre compris).
- **R4, M3** : historique écrit, annuel rejoué **avant** la réaction, puis réaction : chaque mois facturé une fois (360 €),
  dans les deux ordres.
- **R5, aller-retour** : demi-pension, externe au 15 novembre, demi-pension au 1er mars : avoir de décembre à juin
  (7 mois), refacturation de mars à juin sous `annuel:` ; net facturé = septembre à novembre + mars à juin = **315 €** ;
  rejeu de la génération : neutre.
- **R6, départ au 1er** : demi-pension → externe au 1er novembre : novembre crédité ; net **90 €**.
- **R7, mois inconnus** : élève sans période initiale (`ent_eleve.regime` = demi-pension), ligne annuelle posée de
  septembre à juin, passage externe au 1er février par le chemin finances : avoir de février à juin seulement (net
  **225 €**), aucun avoir sur septembre à janvier ; tâche « régime inconnu » si la génération recalcule des mois inconnus
  non posés.
- **R8, non-régression sans historique** : élève sans aucune période : année entière au régime de la fiche, aucun avoir.
- **R9, arrivée en cours de mois** : période initiale demi-pension au 3 septembre : septembre dû (**450 €**).
- **R10, sortie après aller-retour** : R5 puis sortie au 10 avril : net = septembre à novembre + mars à avril =
  **225 €** (aucun mois après avril facturé).
- **R11, absence et sortie ne découvrent rien** : remise d'ordre de 3 mois puis rejeu annuel : aucune refacturation ;
  sortie puis rejeu annuel : aucune refacturation.
- **R12, payeur daté** : payeur de restauration désigné à partir de novembre seulement, externe → demi-pension au
  15 octobre : facturé, aucune tâche « payeur manquant ».
- **R13, année suivante** : `f3s6_e1` (changement en N, annuel N+1 = 450 €) reste vert.
- **D2a, défaut 2** : restauration 450 €/an de septembre à décembre puis 600 €/an de janvier à juin (grille au 1er
  janvier), organisme à 100 %, prise plafonnée à 400 € sur l'année : 1re génération = organisme 180 € + 220 €, 140 € non
  couverts, tâche ouverte ; payeur suivant désigné, rejeu : **140 €** au suivant, une fois ; organisme toujours **400 €** ;
  tâche close ; second rejeu neutre.
- **D2b, part nulle** : même cas, plafond abaissé à 180 € avant le rejeu (payeur suivant désigné) : **140 €** au suivant
  (360 € dus − 220 € déjà posés sur le second tronçon), tâche « plafond modifié » ouverte, rien recrédité à l'organisme.
- **D2c, invariant** : dans les tests D2, aucune tâche `part_non_couverte` n'est close pour une période dont la part non
  couverte n'est pas facturée.
- Les tests existants de régime (`f3s3_…`, `f3s4_m2_…`, `f3s5_faible9_…`, `f3s6_e1_…`) restent verts ; une attente de
  clé `regime:` peut devenir `annuel:` (le montant ne change pas, sauf R3/R6 où la nouvelle règle du 1er s'applique :
  écris l'écart dans la doc).

## Fin
- Pousse sur ta branche et ouvre la PR en **brouillon** vers `ENT`.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min, jusqu'à ce que tous les runs soient `completed`.
  - CI rouge : lis le journal du job, corrige la cause, repousse, relance une seule boucle. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
  - Les runners GitHub sont parfois saturés : une CI « queued » longtemps n'est pas une panne ; attends jusqu'à 2 h, puis écris au coordinateur.
- Ajoute à `docs/socles/24_FACTURATION_F3_LIVRAISON.md` une section « Régime mois par mois » : fait / non fait, règles
  D-069 appliquées, écarts de montant des tests existants, tests, compteurs, migration (aucune ou 534), tête, CI.
- Écris ensuite au coordinateur la tête finale, le numéro de PR et l'état de la CI.
