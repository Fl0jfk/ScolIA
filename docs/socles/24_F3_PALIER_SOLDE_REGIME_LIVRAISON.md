# Socle 24, F3 : palier « solde du régime mois par mois » — livraison

Palier de correctifs de la facturation (règle 39), sans migration : `migrate.rs`, `migrations_versees.txt` et les
compteurs de la règle 28 ne sont pas touchés. Base : `origin/ENT` `5d165264` (CI verte). Branche du palier :
`claude/project-thread-s0qnyp` (branche imposée à la session ; aucune PR avant la fin du palier).

Sources : `docs/pilotage/revues/24_F3_REGIME_MOIS_CONTRE_REVUE.md` (M-5, F-9, F-2 à F-8) et
`docs/pilotage/revues/24_F3_SUITES_4_CORRECTIFS_CONTRE_REVUE.md` (moyen 5, faibles 6, 7 et 9), sur `ent-mc0hoe`.

## Lots et commits (pour les contre-revues)

| Lot | Objet | Commits |
|---|---|---|
| 1 | M-5 (sortie ou remise d'ordre puis régime sur les mêmes mois : crédités deux fois) et F-9 (plafond de la bourse) | `5765655b` (code), `3cb4566e` (doc) |
| 2 | Faibles F-2 à F-5 du régime mois par mois ; F-6 à F-8 examinés | `da0de074` |
| 3 | Moyen 5 des suites 4 (plafond relevé avec un payeur suivant déjà facturé) ; faibles 6, 7 et 9 examinés | `b32622ce` |

## Lot 1 : M-5 et F-9

**M-5.** `regulariser_regime_annee` (`finances_regime.rs`) ne déduit plus « le déjà-crédité moins la valeur des mois
découverts » d'un bloc. Les avoirs posés sur la ligne sont lus (`avoirs_des_lignes`) et rangés en deux familles :

- avoirs de **régime** (`regime:%:avoir:%`, ancien et nouveau format) : leur total réel ;
- **autres** avoirs, imputés mois par mois par `imputer_hors_regime` (fonction pure) :
  - sortie (`sortie:…`) : remplissage depuis le dernier mois non découvert, comme `credit_au_mois` les crédite ;
  - remise d'ordre (`absence:…`) : au prorata des jours de la période propre de l'avoir dans chaque mois ;
  - avoir manuel, changement de payeur : aucun mois (« sans mois ») ;
  - chaque mois reçoit au plus sa valeur (centime supérieur) ; le surplus reste « sans mois ».

Calcul, en valeur absolue puis du signe du total :
- un mois à créditer par le régime n'est crédité que de sa valeur moins ce qui lui est déjà imputé ;
- réserve = valeur des mois conservés (centime inférieur) − ce qui leur est imputé − crédits sans mois, bornée à 0
  (les crédits sans mois sont imputés d'abord aux mois conservés : D-069, jamais de surfacturation) ;
- chaque avoir reste borné par le reste de la ligne moins la réserve.

L'avoir de remise d'ordre (`facturer_absence_prolongee`) porte désormais comme période les jours de l'absence sur la
ligne (avant : la période de la ligne). Aucun lecteur ne s'appuyait sur cette période (`mois_decouverts` ne lit que les
avoirs de régime). Ancien format, période de la ligne : réparti sur toute la ligne ; cela réduit la réserve, donc
favorise la famille (prudent).

Exemples (ligne de 450 € sur dix mois, septembre à juin) :
- M-1 (inchangé) : sortie au 10 avril, puis externe de décembre à février : net 225 €.
- M-5 : sortie au 10 avril, puis externe d'avril à juin : net **315 €** (l'ancien code donnait 225 €).
- M-5, remise d'ordre : du 10 au 24 mai (22,27 €), puis externe dès mai : net **360 €** (l'ancien code donnait 337,73 €).

**F-9.** Le plafond de la bourse replafonnée compte aussi la restauration et l'internat de l'élève pour l'année
**sans période complète** (consommation saisie sans dates, par exemple), lus par
`finances::lignes_eleve_annee_sans_periode`. Sinon la bourse serait réduite à tort et la famille surfacturée.
Constat : la génération annuelle ne pose que des lignes avec période. Le seul chemin qui en crée sans période est
`facturer_consommation_for_actor` (`periode_debut`/`periode_fin` facultatives).

**Tests** (`tests_finances_f3.rs`) :
- `regime_mois_m5_sortie_puis_regime_externe_sur_les_memes_mois_credites_une_fois` (315 €, rejeux neutres) ;
- `regime_mois_m5_remise_d_ordre_puis_regime_externe_sur_le_meme_mois` (360 €, période de l'avoir, rejeux neutres) ;
- `regime_mois_m5_imputation_des_sorties_et_absences` (pur : sortie, mois découvert, absence sur deux mois, avoir
  manuel, plafond d'un mois, réduction) ;
- `regime_mois_f9_consommation_sans_periode_comptee_dans_le_plafond_de_la_bourse` (bourse replafonnée à 290 € et non
  à 90 €, rejeux neutres).

Les trois tests en base **échouent avec l'ancien code** (vérifié : 225 €, période de la ligne, bourse à 90 €).

**Vérifications locales** (toolchain 1.99.0, PostgreSQL 16, base `ent_ci_test`, `ENT_ENV=test`) :
- 3 commandes clippy de la CI, `-D warnings` : 0 avertissement ;
- `rustfmt --check` sur `finances_regime.rs` et `tests_finances_f3.rs` (propres sur ENT) : OK ; `finances.rs` n'est pas
  formaté sur ENT, donc les ajouts y sont mis en forme à la main (pas de `cargo fmt --all`) ;
- `cargo test -p ent-db --features labo --lib -- finances facturation regime` : **182 passés, 0 échec**.

Règles prudentes (D-069) : un crédit sans mois est imputé d'abord aux mois conservés ; une remise d'ordre de
l'ancien format est répartie sur toute la ligne.

## Lot 2 : faibles F-2 à F-8 du régime mois par mois

- **F-2** : nouveau test `regime_mois_f2_changement_par_le_chemin_elv10_de_bout_en_bout`. Il passe par
  `changer_regime_ou_site`, avec la charge publiée par la scolarité et non écrite à la main, puis par le distributeur :
  novembre à juin, 360 €, rejeux neutres.
- **F-3** : R11 vérifie des montants exacts : 136,63 € de remise d'ordre (92 jours sur 303), puis 90 € après la
  sortie du 20 octobre.
- **F-4** : la boucle de replafonnement de la bourse est en arithmétique saturée. La régularisation l'est depuis le
  lot 1. `credit_prorata` garde `unwrap_or(i64::MAX)`, qui est sans risque en centimes.
- **F-5** : `restant_prise` calcule ce qui est facturé au compte **net des avoirs** posés sur ces lignes. Une ligne
  entièrement créditée ne consomme plus le plafond de la prise. Test
  `regime_mois_f5_ligne_creditee_ne_consomme_plus_le_plafond_de_la_prise` : 1 200 € à l'organisme, rien au payeur
  suivant. L'ancien code donnait 1 100 € et 100 € (vérifié).
- **F-6** : laissé en l'état, comme la revue le juge acceptable. La réaction hérite des refus de la garde, par
  exemple une hausse de tarif rétroactive. Ces refus ne sont jamais silencieux : la tâche est liée à l'événement et
  peut être rejouée.
- **F-7** : **examiné, laissé en l'état, à reprendre en conception**. Le correctif essayé comparait les lignes
  `:suivant:` au montant non couvert prévu **avant** la garde. Or, sur une prise plafonnée, la garde
  (`completer_plafonnee`) recalcule la part du payeur suivant à partir de ce qui est déjà posé. Le montant prévu
  n'est donc pas la bonne référence : D2b (plafond abaissé avant le rejeu, 140 € au suivant pour 360 € prévus) gardait
  à tort sa tâche ouverte. Un vrai correctif demande que la garde rende, par porteur, le montant qu'elle a retenu.
- **F-8** : sans objet, la branche part d'ENT `5d165264`.

## Lot 3 : restes des suites 4 de F3

- **Moyen 5** : le plafond est relevé, ou libéré par un avoir de régime, alors qu'une part est déjà facturée au payeur
  suivant. Avant, la famille gardait sa part sans aucun signal. Désormais, `completer_plafonnee` ouvre une tâche
  `part_non_couverte` : « plafond de la prise en charge non consommé alors qu'une part est facturée au payeur suivant ».
  La tâche ne donne pas de montant : le calcul se fait par tronçon et il n'y a qu'une tâche par catégorie, donc le
  montant d'un tronçon serait partiel. Rien n'est crédité ni refacturé d'office. C'est la même règle prudente que pour
  le plafond abaissé (D-069), avec un seul signal par ligne. Le plafond libre retire aussi le complément « plafond
  relevé » posé dans la même passe.
  Test `regime_mois_s4m5_plafond_releve_avec_payeur_suivant_deja_facture_signale` : plafond de 400 € puis 450 €, rien
  ne bouge, une tâche ouverte, rejeu neutre.
- **Contre-revue des lots 2 et 3** (`docs/pilotage/revues/24_F3_PALIER_SOLDE_LOTS_2_3_CONTRE_REVUE.md`, dans le
  dossier du Projet). Verdict : versable, avec deux moyens, corrigés dans le commit de correctifs :
  - **M-1** : le libellé disait « relevé » alors que le signal se déclenche aussi après un avoir de régime (M4b), et
    le montant était partiel. Le libellé est devenu neutre, sans montant. M4b vérifie maintenant la tâche : une seule,
    ouverte.
  - **M-2** : `resoudre_taches_periode` fermait la tâche de signalement, puis la garde la rouvrait à chaque rejeu
    (rejeu non neutre sur les tâches et les événements). Désormais, les tâches dont le détail commence par « plafond
    de la prise en charge » ne sont jamais closes par la facturation : elles attendent une régularisation humaine.
    Cela vaut aussi pour le signal de plafond abaissé, qui avait le même défaut avant le palier. Les tests comptent
    les tâches tous statuts confondus.
  - Faibles mis en file :
    - test F-5 sans cas par le vrai chemin de régime ni avoir partiel ;
    - arithmétique saturée plutôt que `checked_*`, avec quelques `.sum()` nus ;
    - le faible 7 n'est couvert qu'en partie.
  - Décision de gestion notée : un avoir manuel sur une ligne facturée à l'organisme libère aussi son plafond (F-5).
- **Faible 6** (hausse de tarif sur une prise plafonnée, facturée à l'organisme sans validation) : **laissé en
  l'état** (D-069, valeur par défaut prudente). Le complément reste dans le plafond que l'organisme a lui-même
  accordé, donc rien n'est facturé au-delà de son engagement.
- **Faible 7** (test « avoir total » artificiel) : couvert par les tests de régime mois par mois en base. R5, R10,
  M-1 et M-5 font de vrais allers-retours et de vraies sorties, sans forcer la demi-pension.
- **Faible 9** (internat ↔ demi-pension à tarifs différents) : comportement voulu. Le refus de tarif est rejouable,
  voir F-6.

## Fin de palier : équivalent local de la CI (tête `b32622ce`, ENT inchangé à `5d165264`)

Toolchain 1.99.0, PostgreSQL 16 local, `ENT_ENV=test`, une base neuve par tranche dont le nom contient « test ».

- `cargo fmt --check` : le palier ne touche ni `web/` ni `ent-types` (partie bloquante). `finances_regime.rs` et
  `tests_finances_f3.rs` passent `rustfmt --check`.
- Clippy, les 3 commandes de la CI avec `-D warnings` : **0 avertissement**.
- nextest **avec labo**, 4 tranches avec `--test-threads 1` : 4 344 tests, **4 342 passés, 2 échecs hors finances**.
  Les deux tests en échec sont `tests_messagerie_p3::p3_m2_les_bornes_reelles_du_curseur_forge_sont_celles_de_la_base`
  et `tests_messagerie_revue::revue_f2_flux_n_annonce_pas_ses_propres_messages_et_signale_un_retrait`. Ils reposent
  sur les numéros de transaction (xid), qui sont communs à tout le serveur ; or les 4 tranches tournaient sur le même
  serveur, alors que la CI donne un serveur PostgreSQL à chaque tranche. Relancés seuls, ils **passent**. Aucun
  fichier de messagerie n'est touché par le palier.
- nextest **sans labo** (`ent-api`, `ent-db`) : **921 passés**, 0 échec.
- `cargo metadata --locked` dans `ent-rust/web` : OK.
- Aucune migration, aucun `#[ignore]` ajouté, aucun compteur de la règle 28 touché. Les seuls fichiers modifiés sont
  `finances.rs`, `finances_regime.rs`, `tests_finances_f3.rs` et ce document.
