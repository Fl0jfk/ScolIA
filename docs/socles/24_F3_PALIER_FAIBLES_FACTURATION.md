# Socle 24, F3 : palier des faibles de facturation — conception courte et livraison

Palier de correctifs (règle 39), branche `claude/project-thread-48uw09`, partie d'`origin/ENT` (`016636d9`) le 2026-11-27.
Il solde la file laissée par `24_F3_PALIER_SOLDE_REGIME_LIVRAISON.md` et `PALIER_FAIBLES_2_LIVRAISON.md` (« écarté du
palier : finances ») : faibles de la contre-revue des lots 2 et 3 du solde du régime
(`docs/pilotage/revues/24_F3_PALIER_SOLDE_LOTS_2_3_CONTRE_REVUE.md`, dossier du Projet) et F-7 (conception).

**Sans migration, sans action, sans route.** Fichiers touchés : `finances.rs`, `finances_regime.rs`,
`tests_finances_f3.rs` et ce document.

## 1. Conception : cas chiffrés

Grille v1 des tests : contribution (scolarité) 1 200 €, demi-pension 450 €, dix mois (septembre 2030 à juin 2031).
« Organisme » = employeur avec prise en charge ; « famille » = payeur suivant (rang 2). Montants en euros ; les tests
les écrivent en centimes.

### C1. F-5 par le vrai chemin, avec un avoir partiel (faible F-1 de la contre-revue)

Situation réelle : l'employeur conteste une partie de la scolarité ; la comptabilité lui accorde un **avoir manuel
partiel** sur sa facture émise ; plus tard, l'élève passe de l'externat à la demi-pension.

| Étape | Organisme | Famille |
|---|---|---|
| Prise « toutes » plafonnée à 1 300 €, élève externe, génération annuelle | scolarité 1 200 € | 0 |
| Facture de l'organisme émise, avoir manuel partiel de 200 € (`creer_avoir_for_actor`) | net 1 000 € | 0 |
| Externe → demi-pension au 15 octobre (historique + réaction) : restauration due 360 € (8 mois) | **+300 €** | **+60 €** |

Plafond restant à la réaction = 1 300 − (1 200 − 200) = 300 €. L'ancien calcul (brut) donnait 1 300 − 1 200 = 100 € :
100 € à l'organisme et 260 € à la famille, soit 200 € de trop pour la famille. Rejeux (annuel puis réaction) neutres.
Le cas existant `regime_mois_f5_*` (avoir total posé en SQL) reste.

### C2. Décrément du plafond libre par le complément (faible F-2)

| Étape | Organisme | Famille |
|---|---|---|
| Demi-pension (450 € dus), prise « restauration » plafonnée à 350 € ; état d'avant : dû de 400 € | 350 € | 50 € |
| Plafond porté à 400 € ; génération | **+50 €** (complément « plafond relevé ») | 50 € (inchangé) |

Dû 450 € = 400 € organisme (plafond atteint) + 50 € famille : rien à signaler. Le plafond libre vaut 50 € avant le
complément et **0** après : aucune tâche « plafond non consommé ». Sans le décrément (`plafond_libre -= extra`), la
garde voyait encore 50 € libres face aux 50 € de la famille et ouvrait une tâche à tort (vérifié : le test échoue).
Rejeu neutre.

« État d'avant » : les corrections de grille ne peuvent entrer en vigueur qu'**après** la version appliquée, donc
ouvrent un tronçon nouveau, sous une clé nouvelle. Un dû plus élevé sur le **même** tronçon (même clé, même période)
vient d'une correction qui porte sur toute l'année, par exemple le niveau de l'élève. Les tests le mettent en scène en
ramenant en SQL la ligne posée du payeur suivant au montant de l'état d'avant (`suivant_pose_a`).

Doc corrigée : le décrément ne retire que le complément posé **dans la passe** ; le complément déjà posé est compté par
`pose_principal`.

### C3. Dû en hausse sur une prise plafonnée **épuisée** avec un payeur suivant déjà facturé (F-7, défaut trouvé)

En concevant le cas chiffré de F-7, un défaut de montant est apparu (plus qu'un faible de tâche) :

| Étape | Organisme | Famille |
|---|---|---|
| Demi-pension (450 € dus), plafond 400 € ; état d'avant : dû de 420 € | 400 € | 20 € |
| Génération | 400 € | **+30 €** attendus |

`completer_plafonnee` calculait bien le manque de 30 € pour la famille, mais le posait sous la clé `:suivant:`
**d'origine**, déjà en base : la base l'ignorait. **30 € n'étaient facturés à personne** (vérifié : la famille restait
à 20 €), et une tâche ouverte aurait été close car une ligne `:suivant:` « chevauchait » la période (F-7). Correctif :

- le manque d'un payeur suivant dont la ligne est déjà posée va sous une **clé de complément**
  `:c<premier mois libre>…:suivant:…` (comme le complément de l'organisme), le mois libre étant cherché parmi les clés
  posées **et** celles de la passe ; un payeur suivant désigné après coup garde la clé d'origine ;
- invariant de clôture par **montant** (le vrai correctif de F-7) : la garde rend, pour chaque ligne principale
  plafonnée qu'elle examine, le montant **non porté** = dû du tronçon (hors mois déjà facturés par un changement de
  régime) − déjà posé (mois couverts nets) − lignes nouvelles réellement insérables (une clé déjà posée, ou rendue deux
  fois dans la passe, ne compte qu'une fois ou pas du tout). Une tâche `part_non_couverte` n'est close que si ce reste
  est nul aux arrondis près (3 c par mois + 3 c), en plus de la condition de chevauchement existante.

Attendus : famille 50 € (20 + 30 de complément), organisme 400 € ; rejeu neutre (le complément est posé : plus de
manque) ; aucune tâche ouverte. D2a à D2c (plafond abaissé, payeur suivant après coup) inchangés.

### C4. Arithmétique contrôlée de la régularisation (faible F-3)

`regulariser_regime_annee` et le replafonnement de la bourse passent de `saturating_*` et de `.sum()` nus à des
opérations **contrôlées** (`checked_*`, somme contrôlée) : un dépassement rend une erreur `montant hors limites` (rien
n'est posé, la transaction est annulée) au lieu d'un montant faux en silence. Inatteignable en centimes réalistes ;
c'est la lettre de la consigne d'origine. `imputer_hors_regime` (pure, chaque mois borné à sa valeur) et
`credit_prorata` (calcul en `i128`) gardent leur forme, écrite.

### Hors palier

- Faible 7 « couvert en partie » (test qui force la demi-pension) : laissé, déjà noté.
- Un avoir manuel sur une ligne de l'organisme libère son plafond (décision de gestion, notée au palier précédent) :
  C1 l'éprouve désormais par le vrai chemin.

## 2. Livraison

### Tests

| Test | Cas | Échoue sans le correctif |
|---|---|---|
| `faibles_fac_c1_avoir_manuel_partiel_libere_le_plafond_par_le_vrai_chemin` | C1, émission et avoir par l'API, réaction de régime ; plus aucune fausse tâche de plafond au rejeu | oui (100 € au lieu de 300 € ; fausse tâche sans M-2) |
| `faibles_fac_c2_complement_du_plafond_releve_consomme_le_plafond_libre` | C2 | oui (tâche ouverte à tort) |
| `faibles_fac_c3_hausse_sur_plafond_epuise_facturee_au_payeur_suivant_deja_pose` | C3 | oui (famille à 20 €) |
| `tests_faibles_fac_plafonnee::c3_*`, `f7_*`, `m1_*` (purs, `finances.rs`) | C3, non porté de F-7, M-1 | `m1_*` oui (clé rendue deux fois) |
| `faibles_fac_c4_arithmetique_controlee_de_la_regularisation` (pur) | C4 | — |

« Échoue sans le correctif » : vérifié en retirant chaque correctif, test relancé, puis code rétabli.

### Contre-revue indépendante

`docs/pilotage/revues/24_F3_PALIER_FAIBLES_FACTURATION_CONTRE_REVUE.md` (dossier du Projet,
`revues/24_F3_PALIER_FAIBLES_FACTURATION_CONTRE_REVUE.md`), lecture seule. Verdict : **versable après correctifs**,
aucun élevé. C3 sans double facturation (rejeux, payeurs suivants multiples ou changés, prise « toutes », tronçons,
lignes de régime, index unique), D2a à D2c et f3s5_m1 inchangés, formule du non porté juste, arithmétique contrôlée
identique hors dépassement.

- **M-1 (corrigé)** : dans une même passe, la clé de complément du manque pouvait être celle du complément « mois
  manquants » du même payeur suivant (septembre crédité par le régime puis redevenu dû, dû en hausse) ; la base
  ignorait la seconde ligne et le non porté la comptait deux fois. Le mois libre est désormais cherché aussi parmi les
  clés de la passe (complément de l'organisme compris), et le non porté compte chaque clé une fois. Test `m1_*`.
- **M-2 (corrigé, antérieur au palier)** : au rejeu annuel de C1, une fausse tâche « plafond de la prise en charge
  modifié » s'ouvrait : une ligne **propre** (recalculée brute par la génération) gardait son avoir manuel hors du
  restant. `restant_prise` rend désormais au restant les avoirs **hors régime** des lignes propres (les avoirs de régime
  découvrent des mois, que la garde refacture). C1 vérifie l'absence de tâche.
- Faibles mis en file : un non porté positif empêche de clore une tâche mais n'en ouvre pas (perte silencieuse si une
  tâche n'existait pas : à traiter avec F-7 suite, ouverture d'une tâche « non porté ») ; clôture par catégorie alors
  que l'invariant est par article (deux articles d'une même catégorie) ; F-7 n'est éprouvé qu'en fonction pure ; le
  libellé du complément du payeur suivant ne dit pas sa cause ; `b.deja_credite.abs()` non contrôlé et un dépassement
  ferait échouer en boucle la réaction (inatteignable en centimes). Commentaire de `suivant_pose_a` et chiffres de C2
  et C3 corrigés (faibles F-3 et F-4 de la revue).

Effet de bord de M-2 à connaître : un avoir de **sortie** ou de **remise d'ordre** sur une ligne propre de l'organisme
libère aussi son plafond pour les autres catégories, comme un avoir manuel (même règle que F-5 pour les lignes non
propres) ; l'organisme ne dépasse jamais son plafond en net.

### Compteurs (règle 28)

Aucune action, table, migration, route ni source ; `migrate.rs` et `MIGRATION_TAG` inchangés.

### Vérifications locales (code final)

Toolchain 1.99.0, PostgreSQL 16 local, base `ent_ci_test`, `ENT_ENV=test`.
- `cargo test -p ent-db --features labo --lib -- finances facturation regime --test-threads 1` : **192 passés, 0 échec**
  (dont D2a à D2c, f3s5, f3s6, `regime_mois_*`, M4a, M4b, S4M5, F-5).
- `cargo clippy -p ent-db --all-targets -D warnings`, avec et sans `labo` : 0 avertissement. La commande de CI sur tout
  l'espace de travail n'a pas été relancée (seul `ent-db` est touché).
- `rustfmt --check` sur `finances_regime.rs` et `tests_finances_f3.rs` : propre ; `finances.rs` (non formaté sur ENT)
  mis en forme à la main.
- Suite nextest complète non relancée en local : la CI de la PR fait foi.
