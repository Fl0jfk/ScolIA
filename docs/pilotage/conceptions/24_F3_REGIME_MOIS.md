# Conception — facturation au régime mois par mois (socle 24, F3) — 2026-11-26

Concepteur : session Opus du Projet. Lecture seule du code d'`ENT` à `b65892dd` (`ent-rust/crates/ent-db/src/finances.rs`,
`finances_reactions.rs`, `facturation_payeurs.rs`, `scolarite.rs`, `domain-eleve/src/periode.rs`). Rien n'a été exécuté.
Source : `revues/24_F3_SUITES_4_CORRECTIFS_CONTRE_REVUE.md` (élevé préexistant, moyens 2 et 3) et `MIGRATION_PROJETS.md`
§ 4, point 2. Consigne de code : `consignes/24_F3_REGIME_MOIS.md`.

Les règles métier non tranchées reçoivent une valeur prudente, notée **(D-069)** : jamais de surfacturation, jamais de
perte silencieuse ; ce qui ne peut pas être facturé sûrement ouvre une tâche.

---

## 1. Constat

### 1.1 Défaut 1 (élevé, préexistant) : l'annuel facture l'année au régime courant

Faits, lus dans le code :

- `lignes_annuelles` (`finances.rs:1078`) prend **un seul** régime pour toute l'année : `eleve_facturable` lit
  `ent_eleve.regime` (`:848`), ou `regime_force`. `categorie_due` et `choisir_tarif` reçoivent ce régime pour chaque mois.
- L'historique daté **existe** : `ent_eleve_periode` (`kind = 'regime'`, `valeur`, `date_debut`, `date_fin`), tenu par les
  deux chemins de changement (`changer_regime_for_actor`, `finances.rs:4210`, et ELV-10 `changer_regime_ou_site` →
  `ecrire_changement_attribut`, `scolarite.rs:970`) et ouvert à la création de l'élève (`plan_open_initial`,
  `domain-eleve/src/lib.rs:921`). La facturation ne le lit jamais.
- Le passé est déduit des lignes `regime:<événement>` (couverture croisée de `garde_periodes`, `:1786-1843`). Sans ligne
  de régime, rien ne dit à l'annuel que l'élève était externe avant le changement : un rejeu annuel facture l'année
  entière au nouveau régime (externe → demi-pension en juin : 450 € au lieu de 45 €).

**Cause aggravante trouvée à la lecture (non vérifiée par un test)** : le chemin ELV-10 publie `eleve.regime_changed`
avec la charge du domaine (`kind`, `ancienne_valeur`, `nouvelle_valeur`, `date_effet`, `periode.rs:205`), alors que la
réaction de facturation lit `ancien` et `nouveau` (`finances_reactions.rs:74-80`) et **sort sans rien faire** quand ils
manquent. Tout changement de régime fait par la scolarité (ELV-10) n'est donc jamais facturé ni crédité : c'est le
« changement sans ligne de régime » le plus probable en usage réel, pas seulement la facturation en échec.

Moyens du même rapport, même cause :
- **M2** : changement au 1er septembre → septembre jamais facturé au nouveau régime (la ligne de régime part du mois
  suivant).
- **M3** : annuel rejoué après l'écriture de l'historique et avant la réaction → la réaction pose ensuite ses lignes
  `regime:` sur des mois déjà facturés par l'annuel (clés différentes, garde non symétrique) : double facturation.

### 1.2 Défaut 2 (relevé à la lecture) : payeur suivant désigné après coup sur une prise en deux tronçons — **confirmé**

Mécanisme, ligne à ligne :

1. Dans la boucle des tronçons (`:1381`), `periode` est **redéfinie** par le tronçon (`:1386`). Le restant d'une prise est
   calculé **une seule fois**, au premier tronçon rencontré, puis mis en cache dans `restants` (`:1434-1440`) ;
   `restant_prise` reçoit `propres = (préfixe, articles, période du tronçon)` (`:1436`).
2. `restant_prise` (`:1010`) n'exclut que les lignes propres dont `periode_debut` tombe **dans ce premier tronçon**. La
   ligne déjà posée du second tronçon (`:m<AAAA-MM>`, débutant après) est comptée comme « déjà facturée » contre le plafond.
3. Le second tronçon reprend le restant du cache, **déjà diminué de sa propre ligne** : il reçoit trop peu, voire 0.
4. Si sa part tombe à 0, `continue` (`:1525`) saute `dus.insert` : la ligne `:suivant:` n'est pas rattachée à une ligne
   principale plafonnée et passe dans `garde_periodes` comme une ligne ordinaire ; ses mois sont couverts par la ligne
   principale déjà posée (même groupe `:{foyer}`), il n'y a « aucun mois manquant » : **elle est écartée**.
5. Pendant ce temps, `a_resoudre` a reçu la période (`:1490`) : la tâche `part_non_couverte` est **close** alors que rien
   n'est facturé.

Exemple chiffré (repris en test) : restauration tarifée 450 €/an de septembre à décembre puis 600 €/an de janvier à juin
(nouvelle grille au 1er janvier) ; organisme à 100 %, prise plafonnée à 400 € sur l'année. Dû : 180 € + 360 € = 540 €.
- 1re génération, sans payeur suivant : organisme 180 € + 220 € = 400 €, 140 € non couverts, tâche ouverte. Juste.
- Payeur suivant désigné, rejeu : restant au 1er tronçon = 400 − 220 (ligne du 2e tronçon, non exclue) = 180 ; le
  1er tronçon prend 180, le 2e tronçon trouve 0 → part 0, ligne `:suivant:` de 360 € écartée par la garde. **Le payeur
  suivant ne reçoit rien, 140 € ne sont jamais facturés, la tâche est close.**

Le test existant `f3s5_m1` ne le voit pas : un seul tronçon. Deux défauts distincts à corriger : (a) fenêtre des lignes
propres limitée au premier tronçon ; (b) une part principale nulle sur une prise plafonnée sort la ligne `:suivant:` de
l'examen de `completer_plafonnee`. Le (b) existe aussi sans le (a), dès que le plafond est abaissé entre deux générations
jusqu'à épuiser le second tronçon.

---

## 2. Principe retenu

**Le régime se lit mois par mois dans l'historique daté, comme le tarif (M-A) ; il n'y a plus qu'un espace de clés,
`annuel:`, et une seule fonction de régularisation, appelée par la réaction au changement de régime et par la génération
annuelle.**

Écarté : garder les lignes `regime:<événement>` et rendre la couverture croisée symétrique. Cela laisse deux calculs
(annuel et événement) qui doivent se mettre d'accord par la lecture de leurs lignes, ce qui a produit les quatre
correctifs successifs de la couverture croisée et ne règle pas le changement sans ligne.

### 2.1 Régime d'un mois

`regime_du_mois(m)` est lu dans `ent_eleve_periode` (`kind = 'regime'`), valeur normalisée par `regime_normalise` :

1. la période en vigueur le **1er jour** du mois (`date_debut <= 1er` et `date_fin IS NULL OR date_fin >= 1er`) ;
2. à défaut, la **première période qui commence dans le mois** (mois d'arrivée : l'élève entré le 3 septembre en
   demi-pension doit septembre, comme aujourd'hui avec `depuis`) ;
3. à défaut :
   - l'élève n'a **aucune** période de régime → `ent_eleve.regime` pour tous les mois (comportement actuel : sans
     historique, rien ne dit qu'il a changé) ;
   - il en a, mais aucune ne couvre ce mois (mois avant la première période connue) → **inconnu**.

Conséquences (D-069, valeur prudente) :
- **Changement en cours de mois** : le mois reste à l'ancien régime (en vigueur le 1er), le nouveau part du mois suivant.
  C'est la règle actuelle des deux sens (« mois commencé dû » côté départ, « à partir du mois suivant » côté arrivée).
- **Changement le 1er du mois** : le mois est au nouveau régime, dans les deux sens. Ferme M2 (septembre facturé). En
  départ de demi-pension au 1er novembre, novembre n'est plus dû (aujourd'hui il l'est : surfacturation d'un mois).
- **Mois inconnu** : restauration et internat **ni facturés ni crédités** pour ce mois ; tâche
  `regularisation_regime_non_facturee` ouverte (sans événement) « régime inconnu du … au … : restauration et internat non
  facturés pour ces mois ; complétez l'historique de régime de l'élève ». Le tarif des autres catégories (scolarité,
  frais) est choisi avec `ent_eleve.regime`, comme aujourd'hui.
- Un mois dont le régime vient du repli `ent_eleve.regime` (élève sans historique) n'est **jamais crédité** par la
  régularisation (§ 2.3) : seul un régime lu dans l'historique autorise un avoir.

### 2.2 L'annuel, mois par mois

Dans `lignes_annuelles` :
- `regime: String` devient `regimes: Vec<RegimeMois>` aligné sur `mois_tarifes` ; `regime_force` disparaît (seul
  l'ancien chemin `regime:` l'utilisait).
- Pour chaque article et chaque mois : non dû si `categorie_due(cat, regime_du_mois)` est faux ou si le mois est inconnu
  (restauration, internat) ; sinon `choisir_tarif(…, regime_du_mois, …)`.
- Un mois **non dû par le régime** n'est **pas un trou de grille** : le contrôle « trou après un mois tarifé »
  (`:1300-1328`) ne porte que sur les mois dus. Le découpage en tronçons suit déjà la signature (montant, prises) : un
  aller-retour (DP, externe, DP) donne deux tronçons, le second sous `:m<AAAA-MM>`.
- Un article est écarté seulement s'il n'est dû **aucun** mois.
- **Payeurs** : pour chaque catégorie, `payeurs_pour_facturer` est lu au **premier mois où la catégorie est due**, pas au
  premier mois de l'année (sinon une désignation de payeur de restauration posée au changement de novembre serait vue
  comme manquante). Le reste de la lecture des payeurs est inchangé.
- Messages d'anomalie : « régime {r} » cite le régime du mois en cause.

### 2.3 Régularisation des mois facturés hors régime (avoirs)

Nouvelle fonction `regulariser_regime_annee(conn, etab, annee, eleve, evenement: Option<Uuid>, actor, causalite)
-> ResultatInsertion` :

1. Verrou de l'élève pour la régularisation (`pg_advisory_xact_lock` sur `fin-regime:{etab}:{eleve}`), pris **avant**
   toute lecture.
2. Lignes de facture de l'année (`lignes_eleve_annee`) de catégorie `restauration` ou `internat`, et les réductions de
   ces catégories (`categorie = 'reduction'`, `categorie_article` restauration ou internat), avec période.
3. Pour chaque ligne : mois **couverts nets** = mois de sa période moins les mois déjà crédités par un avoir de régime
   (§ 2.4). Mois **à créditer** = mois couverts nets dont le régime est **lu dans l'historique** (§ 2.1, pas inconnu, pas
   repli) et pour lesquels la catégorie n'est pas due.
4. Un avoir par suite ininterrompue de mois à créditer : montant = prorata `total × mois de la suite / mois de la ligne`,
   arrondi **en faveur de la famille** (ligne positive : crédit au centime **supérieur**, comme `credit_au_mois` qui
   arrondit le dû au centime inférieur ; réduction : reprise au centime **inférieur** en valeur absolue), borné par ce qui
   reste sur la ligne (`total − deja_credite`, même signe que `total`).
   Clé : `regime:mois:avoir:{ligne}:m{AAAA-MM}` (premier mois de la suite) ; période de l'avoir = la suite créditée.
   La clé ne porte pas l'événement : deux régularisations concurrentes ou successives du même état produisent la même
   clé, l'index d'unicité empêche le double avoir. Un mois crédité ne redevient jamais couvert sur la même ligne (un retour
   au régime est refacturé par une ligne nouvelle), donc (ligne, premier mois) est unique.
5. Rien d'autre n'est crédité : ni la scolarité, ni les mois inconnus, ni les mois de repli.

### 2.4 Mois couverts d'une ligne posée

Partout où une ligne posée « couvre » des mois (garde `garde_periodes`, `completer_plafonnee`, régularisation,
`credit_au_mois`, `credit_aux_jours`), les mois **crédités par un avoir de régime** sont retirés :
- avoir de régime **avec sa période propre** (nouveau format, § 2.3) : ses mois exactement ;
- avoir de régime de l'**ancien format** (`regime:{evt}:avoir:{ligne}`, période recopiée de la ligne d'origine) : les
  derniers mois, au nombre de `floor(mois × crédit / total)` (**arrondi en faveur de la couverture** : un mois en partie
  crédité reste couvert, donc n'est jamais refacturé — D-069) ;
- les avoirs de **sortie** et d'**absence** ne découvrent **aucun** mois : un rejeu après une sortie ou une remise d'ordre
  ne refacture jamais rien.

Effets à garder en tête :
- `garde_periodes` : `couverts` et `pose_total` (dans `completer_plafonnee`) se calculent sur les mois couverts nets ; le
  contrôle de tarif « même nature » continue de comparer le montant mensuel de la ligne entière.
- `credit_au_mois` (sortie) : crédite les mois **après** la date **encore couverts nets**, au prorata de la ligne, borné
  au reste ; aujourd'hui il crédite `total − dû − deja_credite`, ce qui donnerait 0 après un avoir de milieu de période
  (aller-retour) et laisserait facturés les mois après la sortie.
- `credit_aux_jours` (absence) : les jours tombant dans un mois crédité ne sont pas crédités une seconde fois.

### 2.5 Réaction au changement de régime

`facturer_changement_regime` est remplacée par :

1. lire l'élève et la `date_effet` de l'événement (**seules** `eleve_id` et `date_effet` sont exigées ; `ancien` et
   `nouveau`, ou `ancienne_valeur` et `nouvelle_valeur`, ne servent plus qu'au journal) : les deux chemins (finances et
   ELV-10) sont facturés ;
2. année = `annee_de(date_effet)` ;
3. `regulariser_regime_annee(…, Some(evenement), …)` ;
4. catégories **gagnées** à la date d'effet, lues dans l'historique : restauration ou internat dues au régime du mois de
   `date_effet` (§ 2.1, règle du 1er) ou du mois suivant, et non dues au régime du mois précédent ; repli sur la charge
   de l'événement si l'historique ne permet pas de conclure ;
5. si des catégories sont gagnées : `lignes_annuelles(annee, eleve, depuis, categories = gagnées, préfixe annuel:{annee})`
   puis `inserer_lignes` ; `depuis` = date de la première inscription validée de l'année (même requête que
   `generer_annee_for_actor`, à factoriser), à défaut `ent_eleve.date_entree`, à défaut aucun ;
6. refus (`Err(motif)`) : tâche `regularisation_regime_non_facturee` liée à l'événement, comme aujourd'hui ; réussite :
   clôture de la tâche de cet événement.

Le changement ne pose **plus aucune ligne `regime:`**. Les clés de l'annuel étant les mêmes que celles de la réaction, un
annuel rejoué avant ou après la réaction est un rejeu (M3 fermé dans les deux ordres).

Interne ↔ demi-pension à tarifs de restauration différents (faible 9 du rapport) : seule la catégorie internat est
gagnée ou perdue ; la restauration garde sa ligne au tarif d'origine. La génération annuelle complète, elle, verra le
tarif changer et **refusera** explicitement (comportement actuel, régularisation F4). Hors périmètre, noté.

### 2.6 Génération annuelle

`facturer_annee_eleve` appelle `regulariser_regime_annee(…, None, …)` **avant** `lignes_annuelles`. Les avoirs comptent
dans le rapport (`ResultatInsertion::ajouter` : lignes créées et montant, négatif). L'aperçu reste annulé par la
transaction. Ainsi un changement dont la réaction n'a jamais tourné (charge ELV-10 aujourd'hui, réaction en échec) est
régularisé par la génération suivante, dans les deux sens.

### 2.7 Lignes `regime:` existantes

Aucune base n'est déployée, mais les tests en posent. La couverture croisée de `garde_periodes` est **conservée telle
quelle** pour les lignes `regime:` déjà posées (lecture seule de l'ancien format) ; plus aucun code n'en crée. Une
simplification (retrait) pourra suivre quand Florian confirmera qu'aucune base de recette n'en contient.

### 2.8 Défaut 2

- (a) La fenêtre des lignes propres passée à `restant_prise` est la **période complète calculée** par la génération
  (premier mois dû → dernier jour de l'année), pas celle du tronçon. Renommer la variable externe (`periode_gen`) pour
  supprimer l'ombrage.
- (b) Sur une prise **plafonnée**, la ligne principale est toujours inscrite dans `dus` et transmise à la garde, **même à
  part 0** ; `completer_plafonnee` la traite (manque au dû → payeur suivant ; plafond abaissé → signal) ; une ligne à 0 €
  n'est jamais insérée (filtrée en sortie de garde ou à l'insertion).
- (c) **Invariant de clôture** : une tâche `part_non_couverte` n'est close pour une période que si, après la garde, la part
  non couverte de cette période est effectivement portée par des lignes sorties de la garde (ou déjà posées), ou s'il n'y
  a plus rien de non couvert. Concrètement : `a_resoudre` est filtré après la garde sur les clés `:suivant:` (ou leurs
  compléments `:c`) réellement présentes dans la sortie ou déjà en base.

---

## 3. Règles prudentes notées (D-069)

| Règle | Valeur retenue | Pourquoi |
|---|---|---|
| Mois du changement | régime en vigueur le 1er du mois ; à défaut, premier régime commencé dans le mois | règle actuelle des deux sens, ferme M2 |
| Mois avant le premier régime connu | ni facturé ni crédité (restauration, internat) + tâche | ni surfacturation, ni avoir sur une hypothèse |
| Élève sans historique | `ent_eleve.regime` pour l'année, jamais d'avoir automatique | comportement actuel |
| Avoir partiel ancien format | mois en partie crédité = encore couvert | jamais de refacturation |
| Sortie, absence | ne découvrent aucun mois | un rejeu ne refacture rien |
| Tarif dépendant du régime (restauration DP ≠ interne) | refus explicite à la génération (F4) | inchangé, hors périmètre |
| Payeurs d'une catégorie | lus au premier mois où elle est due | une désignation datée du changement n'est pas « manquante » |
| Plafond abaissé | signal (tâche), rien de recrédité d'office | inchangé (M-2) |

Candidat paramètre d'établissement, **non créé dans ce lot** : « mois du changement de régime : ancien / nouveau régime ».
La valeur prudente ci-dessus suffit ; pas de migration pour un réglage que personne n'a demandé.

## 4. Migration

**Aucune** : l'historique, les colonnes de période des lignes d'avoir et la nature de tâche existent. Si l'implémentation
en démontre une indispensable : **534** (533 est inscrite pour F16 « soldes de sortie » au registre ; 533 à 539 vérifiés
libres sur les 333 branches distantes le 2026-11-26), en fin du tableau de `migrate.rs` (règle 38).

## 5. Ce qui est fermé

- Élevé préexistant (rapport 24 suites 4 correctifs) : annuel au régime courant, changement sans ligne de régime.
- M2 (changement au 1er du mois), M3 (annuel rejoué avant la réaction).
- Charge ELV-10 ignorée par la réaction (trouvé ici).
- Défaut 2 (payeur suivant après coup sur deux tronçons) et sa variante plafond abaissé à part 0.
- Sortie après un aller-retour de régime (trouvé ici, conséquence des avoirs de milieu de période).

Hors périmètre : faible 9 (tarifs de restauration différents par régime), simplification de la couverture croisée,
paramètre « mois du changement », arrêt de l'annuel à la date de sortie.
