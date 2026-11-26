# Contre-revue — régime mois par mois (F3, PR 159) — 2026-11-26

Relecteur indépendant, lecture seule. Branche `ent-socle-24-code-f3-regime-mois`. **Tête de code : `0832e99d`.** La tête de la branche est
`c59e2d99` (« docs(finances): régime mois par mois, tête et CI ») : `git diff --stat 0832e99d c59e2d99` = un seul fichier,
`docs/socles/24_FACTURATION_F3_LIVRAISON.md` (+4 lignes). Aucun code après `0832e99d`.

Méthode : lecture de `git diff origin/ENT...<tête>` (9 fichiers, 1913 ajouts) contre la conception (`24_F3_REGIME_MOIS.md`) et la consigne.
**Rien n'a été exécuté** (pas de base locale) : les montants ci-dessous sont des calculs à la main sur le code, et les tests ont été lus, pas rejoués.
La CI verte (run 45819916699, `0832e99d`) fait foi pour l'exécution des 15 tests `regime_mois_*`.

## Verdict : **versable**

Aucun élevé. Les montants des tests R1 à R12 et D2a à D2c sont conformes à la consigne et non affaiblis. Les moyens ci-dessous sont des cas limites ;
**M-1 est introduit par le lot** (correctif de revue interne) : à corriger dans un lot court juste après le versement, ou avant si le pilote applique
D-069 (« jamais de surfacturation ») à la lettre.

## Élevé

Aucun.

## Moyens

**M-1. Le crédit de régime sous-crédite quand une sortie ou une remise d'ordre est déjà posée sur un mois conservé (surfacturation de la famille).**
`finances_regime.rs:314-329` (« reste créditable » = `total − deja_credite − réserve`). La réserve vaut les mois « conservés » (couverts nets non
crédités par le régime), mais un mois déjà crédité par une sortie ou une absence reste « couvert net » (§ 2.4 : ils ne découvrent rien) : il est compté deux
fois (déjà crédité et réservé). Exemple chiffré : ligne de 450 € sur 10 mois ; sortie au 10 avril → avoir 90 € (mai-juin, `deja_credite` = 9000) ; puis historique
« externe » de décembre à février posé après coup. À créditer : 3 mois = 135 €. Code : conservés = 7, réserve = 315 €, reste = 450 − 90 − 315 = **45 €** → avoir de 45 € au
lieu de 135 € ; net facturé 315 € au lieu de 225 € (**90 € de trop**). La clé `regime:mois:avoir:{ligne}:m2030-12` existant déjà, le rejeu ne corrige jamais. Même effet
avec une remise d'ordre sur un mois conservé (la remise est « réabsorbée »). R10 ne le voit pas (sortie **après** l'aller-retour). Le borne ne sert qu'au
chevauchement (mois à la fois crédité par une sortie et à créditer par le régime) : réserver seulement les mois conservés **non encore crédités** par sortie/absence.
Aucun test dédié (la doc le dit : « sans test dédié »). *Calcul à la main, non exécuté.*

**M-2. Clés de complément d'un foyer dont l'UUID commence par `c` (préexistant, non corrigé).** `finances.rs:2239` `reste_cle` retire 9 caractères dès que
`r[..2]` vaut `:m` ou `:c`, **sans vérifier les chiffres** (contrairement à `groupe_de_cle`, `:1809`, qui les vérifie). Un foyer dont l'UUID commence par `c` (1 sur 16) voit
sa clé de complément `…:c2030-12` + UUID tronqué de 8 caractères : à la relecture, `groupe_de_cle` range cette ligne dans un groupe différent de la principale ; les mois
du complément ne sont plus vus comme couverts par le groupe, d'où risque de double facturation dès qu'un rejeu recalcule un complément de plage différente (même plage :
l'index d'unicité sauve). Le lot rend le chemin plus fréquent (retour au régime, plafond relevé, `:c` après avoir). Correctif trivial : même test de chiffres que
`groupe_de_cle` ; à ajouter un test avec un foyer à UUID fixé `c…`. **À classer moyen, en file, hors lot.** (Les tests de complément qui tirent un UUID au hasard ont une chance sur 16
d'être instables.)

**M-3. Bourse non replafonnée après un avoir de régime (préexistant, non corrigé).** `finances.rs:1648-1688` : la bourse est plafonnée à la somme des lignes restauration/internat **de la
génération en cours**, puis posée sous une clé fixe `…:bourse:{id}:{foyer}`. Un avoir de régime (`finances_regime.rs`) ne touche pas la catégorie `bourse` ; le rejeu annuel ne la réduit
jamais (même clé, montant plus faible ignoré par la garde). Exemple : DP → externe au 1er novembre, restauration nette 90 €, bourse 300 € : facture nette −210 € à restituer, alors que
l'excédent « n'est pas imputé » (règle R-F14). Même défaut avant le lot ; le lot le rend atteignable par une simple réaction. **Moyen (perte côté établissement), préexistant, en file.** Côté réaction :
`categories = [restauration]` exclut aussi la bourse (`:1628`) : une restauration **gagnée** (externe → DP) n'est jamais compensée par la bourse tant qu'une génération complète n'a pas tourné.

**M-4. Les deux correctifs de la revue interne n'ont aucun test (point 1 du coordinateur : voir « Point 1 » plus bas).** Aucun test `regime_mois_*` ne combine régime et prise plafonnée
(`plafonner(` n'apparaît que dans D2) ni régime et réduction/bourse.

## Faibles

- **F-1.** Doc : « 18 tests `regime_mois_*` » ; il y en a **15** (R1 à R12 + D2a à D2c). R13 = `f3s6_e1_*` (vert). 9 tests unitaires purs : exact.
- **F-2.** R2 publie à la main l'événement avec la charge d'ELV-10 (`ancienne_valeur`/`nouvelle_valeur`, identique à `domain-eleve/src/periode.rs:205`), sans passer par `changer_regime_ou_site` comme la consigne l'écrit. La correction est réelle (la réaction lit les deux formes, `finances_reactions.rs:74-91`) ; un test de bout en bout éviterait toute dérive de la charge.
- **F-3.** R11 asserte `0 < net < 45 000` et l'égalité avant/après rejeu, pas un montant exact de la remise ; le rejeu neutre (le fond du test) est bien vérifié.
- **F-4.** Arithmétique : `finances_regime.rs:321-329,333-345` soustractions `i64` nues et `unwrap_or(i64::MAX)` ; la consigne demandait `checked_*`. Pas de risque pratique (centimes), non conforme à la lettre.
- **F-5.** `restant_prise` (`finances.rs:1037`) ne retire jamais les avoirs ; depuis le correctif, un article **plus dû aucun mois** (tous ses mois crédités) n'est plus exclu des lignes propres et compte encore contre le plafond : sous-consommation du plafond (famille un peu surfacturée sur une prise « toutes »). Cas rare, prudent côté établissement.
- **F-6.** La réaction régénère toute l'année des catégories gagnées : elle hérite des refus de la garde (hausse de tarif rétroactive sur des mois déjà posés). Jamais silencieux (tâche liée à l'événement, rejouable) : acceptable, à connaître.
- **F-7.** `suivant_porte` vérifie qu'une ligne `:suivant:` chevauche la période, pas que son montant égale la part non couverte : une ligne plus ancienne et plus petite suffit à clore la tâche.
- **F-8.** Branche en retard de 13 commits sur `ENT` (base `b65892dd`). `git merge-tree` propre ; seul `lib.rs` a bougé côté ENT (+5 lignes de modules). Le lot ne touche aucun compteur : règle 28 neutre, mais à relire après fusion (règle 17).

## Vérifications demandées (tout conforme sauf mention)

- **Montants.** Pas de double facturation : clés uniques `annuel:` (plus aucune `regime:` créée, test `lignes_regime_residuelles`), réaction et annuel confondus, rejeu neutre dans les deux ordres (R4). Aucun mois oublié : mois non dus ≠ trous de grille, tronçons scindés par la signature (R5). Avoirs en faveur de la famille (plafond, réduction au centime inférieur), bornés au reste, par suite de mois, clé `regime:mois:avoir:{ligne}:m{mois}`. Exception : M-1. Écarts R3 (450 € au lieu de 405 €) et R6 (90 € au lieu de 135 €) : voulus par la règle du 1er (§ 2.1), écrits dans la doc.
- **Tests R et D.** Présents, montants de la consigne : R1 45 € (`4_500`), R2 360 €, R3 450 €, R4 360 € (deux ordres), R5 315 €, R6 90 €, R7 225 €, R8 450 €, R9 450 €, R10 225 €, D2a 140 € au suivant et 400 € à l'organisme, D2b 140 € et rien de recrédité, D2c aucune tâche close à tort ; aucune assertion de montant retirée des tests existants (`git diff` : seules des mises en scène supprimées).
- **ELV-10.** Corrigée : seules `eleve_id` et `date_effet` sont exigées, `ancien`/`nouveau` ou `ancienne_valeur`/`nouvelle_valeur` ne servent qu'en repli ; le régime se lit dans l'historique, donc indépendant de la charge. (F-2 sur le test.)
- **Défaut 2.** Corrigé : fenêtre propre = `periode_gen` (`finances.rs:1133`, plus d'ombrage) ; ligne principale plafonnée transmise à la garde à part 0 (`:1580`) ; zéro € filtré en sortie de garde (`:2181`) ; clôture conditionnée par `suivant_porte` (`:1735`, `:1762`), paramètres SQL `$1..$6` liés dans l'ordre. (F-7.)
- **R-F1.** Conforme : `finances_regime.rs` n'a plus d'`INSERT/UPDATE/DELETE` (l'`UPDATE ent_fin_tache` est passé dans `facturation_payeurs.rs`, fichier du service, `0832e99d`) ; les avoirs passent par `inserer_avoirs` (`finances.rs`) et les tâches par `ouvrir_tache*`. Note : `finances_regime.rs` n'est pas dans `FICHIERS_DU_SERVICE` : il ne doit rien écrire directement ; le garde le vérifie (CI verte).
- **Idempotence, verrous, RLS.** Verrou `pg_advisory_xact_lock` `fin-regime:{etab}:{eleve}` pris avant toute lecture (`finances_regime.rs:279`), clé incluant l'établissement ; toutes les requêtes ajoutées filtrent `etablissement_id` ; la réaction pose `app.etablissement_id` (`finances_reactions.rs:53`). Génération groupée : verrous par élève tenus jusqu'à la fin de la transaction, ordre identique entre génération et réaction (pas d'inversion repérée).
- **Règle 25.** `git log origin/ENT..<tête>` : 3 commits (`5c1af3df`, `0832e99d`, `c59e2d99`), tous du lot ; aucun ne touche `POINT_QUOTIDIEN.md`, `PILOTAGE.md` ni `docs/pilotage/`. Base `b65892dd` (fusion d'ENT), pas `ent-mc0hoe`.
- **Migration.** Aucune (`git diff --stat` sans `.sql`, `migrate.rs` ni `migrations_versees.txt` ; 534 non consommée).

## Point 1 du coordinateur — élevé de la revue interne (plafond surévalué sous prise plafonnée)

**Corrigé dans le code, pour le cas décrit ; test à ajouter.** Avant : `restant_prise` excluait de la somme des lignes déjà facturées toutes les lignes `annuel:{année}:{élève}:` de **tous** les articles
(`propres_articles = articles.map(id)`). Une réaction de régime ne recalcule que les catégories gagnées : la scolarité posée par l'annuel, exclue à tort, n'était pas comptée contre le plafond
d'une prise « toutes » : plafond surévalué, donc trop facturé à l'organisme. Après (`finances.rs:1282`) : seuls les articles dont la catégorie est demandée **et** due au moins un mois sont exclus ; la scolarité compte contre le plafond. La logique est juste.
Il **faut un test** (le correctif n'en a pas, M-4) : prise plafonnée « toutes » à 300 € ; scolarité annuelle posée à 200 € ; externe → demi-pension au 15 octobre ; réaction ; attendre que
la restauration (360 €) n'aille à l'organisme que pour 100 €, le reste au payeur suivant ou en tâche `part_non_couverte`. Ce test échoue avec l'ancien code. Le même test devrait couvrir le « moyen corrigé » (borne du crédit, voir M-1 : le correctif a introduit le défaut M-1).

## Points 2 et 3 du coordinateur — classement

- Bourse non replafonnée après un avoir de régime : **moyen, préexistant** (M-3), en file ; à traiter avec le socle 26 (reversement de l'excédent).
- Clés de complément d'un foyer dont l'UUID commence par `c` : **moyen, préexistant** (M-2), en file ; correctif de deux lignes et un test, à faire dès le prochain lot financier car le lot actuel élargit son atteinte.

## Décision

Lot **versable** (aucun élevé, tous les tests attendus présents, ELV-10 et défaut 2 réellement fermés, R-F1, règle 25 et absence de migration conformes). À mettre en file avec priorité :
M-1 (introduit par le lot), M-4 (deux tests), puis M-2 et M-3. Doc à corriger : « 18 tests » → 15.

## Moyens — vérification des correctifs (`c59e2d99..eb08f406`, pilote, 2026-11-26)

Relecture en lecture seule du diff (4 fichiers : `finances.rs`, `finances_regime.rs`, `tests_finances_f3.rs`, doc de livraison). Aucun élevé.

- **M-1 — corrigé, testé, mais le correctif rouvre le chevauchement (nouveau moyen M-5).** La réserve vaut désormais `valeur(conservés) − (déjà crédité − valeur(découverts))`, bornée à 0. Sur l'exemple de la revue (450 €, sortie au 10 avril = 90 € sur mai-juin, puis externe décembre-février) : réserve 225 €, reste 135 €, net 225 € : juste. Test `regime_mois_m1_sortie_deja_posee_puis_regime_externe_credite_en_entier` (net 225 €, rejeux neutres).
- **M-2 — corrigé, testé.** `sans_suffixe_mois` (chiffres vérifiés) remplace les trois découpes (`groupe_de_cle`, `garde_periodes`, `reste_cle`). Test pur (UUID `c…`, `m…`, `:c2030` sans mois) et test en base avec un foyer à UUID imposé `c…` (février refacturé une fois, rejeux neutres).
- **M-3 — corrigé, testé.** `regulariser_regime_annee` replafonne la bourse sur la restauration et l'internat restant dus par foyer, clé `bourse:replafonnage:{ligne}:d{déjà crédité}` (hors du motif `regime:%:avoir:%`, ne découvre aucun mois). Test `regime_mois_m3_…` : bourse ramenée à 90 €, rejeux neutres. Le cas « restauration gagnée non compensée par la bourse » reste écrit dans la doc du lot.
- **M-4 — corrigé.** `regime_mois_m4a_…` (prise « toutes » plafonnée, scolarité comptée contre le plafond : 100 € à l'organisme, 260 € au payeur suivant ; échoue avec l'ancien code) et `regime_mois_m4b_…` (régime et prise plafonnée, net 315 €).

**M-5 (nouveau, moyen, introduit par le correctif M-1, en file).** Les avoirs de sortie et d'absence ne découvrent aucun mois (`mois_decouverts`) : un mois déjà crédité par une sortie et que le régime rend non dû est à la fois dans `a_crediter` et dans `credit_hors_regime`. Exemple (calcul à la main, non exécuté) : 450 € sur 10 mois, sortie au 10 avril (90 € sur mai-juin), puis historique externe d'avril à juin. Dû : 7 mois, 315 €. Code : conservés 7, réserve 315 − 90 = 225 €, reste 135 €, trois mois crédités (135 €) : net **225 €**, soit 90 € de **sous-facturation** (mai-juin crédités deux fois). L'ancienne borne donnait 315 €, juste ici, mais faux sur l'exemple de M-1. Sens favorable à la famille (D-069 : jamais de surfacturation), perte côté établissement, d'où **moyen**, pas élevé. Correctif : imputer `credit_hors_regime` mois par mois (périodes des avoirs de sortie et d'absence) et retirer de `a_crediter` les mois déjà crédités, avec un test sortie puis externe sur les mêmes mois.

**Faible (nouveau, F-9).** Le plafond de la bourse ne compte que les lignes de restauration et d'internat **avec période** (filtre de `lignes`) : une telle ligne sans période ferait replafonner la bourse à tort (famille surfacturée). Les lignes annuelles portent toujours une période ; à vérifier si un autre chemin en crée sans.

**Verdict inchangé : versable.** File : M-5 (priorité, introduit par le lot), F-9, puis F-2 à F-8.
