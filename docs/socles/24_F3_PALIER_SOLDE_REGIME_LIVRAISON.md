# Socle 24, F3 : palier « solde du régime mois par mois » — livraison

Palier de correctifs de la facturation (règle 39), sans migration : `migrate.rs`, `migrations_versees.txt` et les
compteurs de la règle 28 ne sont pas touchés. Base : `origin/ENT` `5d165264` (CI verte). Branche du palier :
`claude/project-thread-s0qnyp` (branche imposée à la session ; aucune PR avant la fin du palier).

Sources : `docs/pilotage/revues/24_F3_REGIME_MOIS_CONTRE_REVUE.md` (M-5, F-9, F-2 à F-8) et
`docs/pilotage/revues/24_F3_SUITES_4_CORRECTIFS_CONTRE_REVUE.md` (moyen 5, faibles 6, 7 et 9), sur `ent-mc0hoe`.

## Lots et commits (pour les contre-revues)

| Lot | Objet | Commits |
|---|---|---|
| 1 | M-5 (sortie ou remise d'ordre puis régime sur les mêmes mois : crédités deux fois) et F-9 (plafond de la bourse) | `5765655b` |

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
