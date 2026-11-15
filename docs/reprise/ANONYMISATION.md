# Anonymiser l'export du Next.js — mode d'emploi (Florian)

Statut : outil **implémenté et testé en local sur des données fictives** (lot R1 du socle 28, 2026-11-14) ;
**jamais essayé sur un export réel** : votre premier essai en est la première épreuve. Rien ici ne
prouve que l'export est « prêt production » : c'est un outil de répétition (Noël, D-057).

## En une phrase

Vous lancez le programme `ent-reprise` **sur votre poste**, sur un **fichier** d'export de la base du
Next.js (et la liste des fichiers du stockage). Il en fabrique une copie où **plus aucune personne
réelle ne figure**, avec un rapport, et vous nous envoyez **seulement cette copie**.

Le programme **ne se connecte jamais** à une base ni au stockage : il ne lit que des fichiers. Il refuse
même qu'on lui donne une adresse (`postgres://…`, `s3://…`, `https://…`).

## Ce qu'il vous faut

1. Le programme `ent-reprise` (ou `ent-reprise.exe`) que l'équipe vous remet, compilé pour votre machine.
2. **Un export de la base** au **format texte** (recommandé) :
   ```
   pg_dump --format=plain --no-owner --no-privileges --encoding=UTF8 --file=export.sql <votre connexion habituelle>
   ```
   Le format « répertoire » marche aussi **s'il n'est pas compressé** (`pg_dump --format=directory -Z 0 …`).
   Le format répertoire compressé (par défaut) n'est **pas** pris en charge : le programme le dit et
   vous indique quoi faire. Pas d'option `--inserts`.
3. **La liste des fichiers du stockage** (un bucket par établissement), un fichier texte avec une ligne
   par fichier : `clé<tabulation>taille`. Par exemple avec l'outil `aws` et `jq` (adaptez l'adresse du
   service et le nom du bucket) :
   ```
   aws s3api list-objects-v2 --bucket <bucket> --endpoint-url <adresse du stockage> \
       --query 'Contents[].[Key,Size]' --output json | jq -r '.[] | @tsv' > objets_s3.tsv
   ```
   Une troisième colonne facultative peut contenir l'empreinte SHA-256 de chaque fichier. Les fichiers
   eux-mêmes ne sont **pas** copiés.
4. Un dossier de travail **sur votre poste** (rien ne doit en sortir sauf le dossier de résultat).

## Étape 0 — s'entraîner avec le jeu fictif (5 minutes)

```
ent-reprise jeu-fictif --sortie essai
ent-reprise generer-cle --fichier cle-essai.txt
ent-reprise inspecter --dump essai/dump.sql --objets-s3 essai/objets_s3.tsv --sommes essai/SHA256SUMS
ent-reprise anonymiser --dump essai/dump.sql --objets-s3 essai/objets_s3.tsv --cle cle-essai.txt --sortie essai-anonyme
ent-reprise verifier --dossier essai-anonyme
```
Le jeu fictif est entièrement inventé (personnages imaginaires, e-mails en `example.test`).

## Étape 1 — créer votre clé

```
ent-reprise generer-cle --fichier cle.txt
```
La clé sert à fabriquer les pseudonymes : la même personne reçoit toujours le même faux nom, donc les
liens et les doublons restent visibles. **Ne la transmettez jamais** (ni à nous, ni dans un message,
ni dans un dossier partagé) : sans elle, personne ne peut retrouver une vraie personne.

*Détruire ou garder ?* Par défaut, **détruisez-la** quand vous êtes sûr de ne plus en avoir besoin.
Si vous gardez la clé, vos exports suivants (mi-janvier, début février) donneront les **mêmes**
pseudonymes : pratique pour comparer deux répétitions. À vous de décider ; dans tous les cas, elle reste
sur votre poste.

## Étape 2 — inspecter l'export (sans rien transformer)

```
ent-reprise inspecter --dump export.sql --objets-s3 objets_s3.tsv
```
Il affiche : les tables attendues (187 : 184 de l'inventaire d'octobre + 3 ajoutées depuis), les lignes
par table et par collection, les fichiers par dossier, et **« CONFORME »** ou **« NON CONFORME »**.
Un rapport « NON CONFORME » liste les tables ou colonnes que l'outil ne connaît pas : **ne continuez pas**,
envoyez-nous ce texte (il ne contient ni nom ni donnée, seulement des noms de tables).

## Étape 3 — anonymiser

```
ent-reprise anonymiser --dump export.sql --objets-s3 objets_s3.tsv --cle cle.txt --sortie anonymise
```
Le dossier `anonymise` ne doit pas exister (ou être vide). Selon la taille, comptez de quelques secondes
à quelques minutes.

## Ce qui sort (dossier `anonymise/`)

| Fichier | Contenu |
|---|---|
| `dump.sql` | l'export anonymisé, au même format que l'entrée (se restaure avec `psql`) |
| `objets_s3.tsv` | la liste des fichiers, chemins réécrits ; aucun fichier « secret » ; les empreintes d'origine ne sont pas reprises |
| `rapport_anonymisation.txt` / `.json` | ce qui a été transformé (règle et nombre de cellules par colonne), **sans aucune valeur réelle** |
| `SHA256SUMS` | l'empreinte SHA-256 de chacun des fichiers ci-dessus |
| `EMPREINTE.txt` | **l'empreinte de l'export** (celle du fichier `SHA256SUMS`) |

Le programme affiche aussi cette empreinte à la fin.

## Ce qui est transformé, ce qui est gardé

- **Remplacé par un pseudonyme** (toujours le même pour la même valeur) : noms, prénoms, e-mails
  (devenus `…@perso.example.test`, `…@ac.example.test` ou `…@entreprise.example.test` selon le type du
  domaine d'origine), téléphones (plages fictives réservées), INE, IBAN (fictifs mais valides),
  adresses, codes postaux (département gardé), villes, dates de naissance (décalées de 1 à 60 jours),
  adresses IP, identifiants externes, noms de fichiers.
- **Textes libres** (motifs, commentaires, messages, observations…) : remplacés par un texte de **même
  longueur et même mise en forme**, sans aucun mot d'origine.
- **Effacés** : mots de passe, jetons d'accès, secrets de double authentification, clés d'agenda ; les
  jetons à usage unique sont remplacés par d'autres jetons.
- **Images et PDF** stockés dans la base : remplacés par une image et un PDF neutres.
- **Santé** (infirmerie, PAI, accidents, médicaments) : **exclue** — la structure est gardée, aucune ligne.
- **JSON et collections** (`ent_collection_*`, tables `*_attr`, colonnes JSON) : décodés ; chaque valeur
  texte est traitée selon le **nom de sa clé** ; une clé que l'outil ne connaît pas voit son texte
  remplacé et son nom apparaît au rapport.
- **Fichiers du stockage** : seule la liste est reprise ; dossiers et noms qui pourraient être des noms
  de personnes sont remplacés ; les identifiants techniques (UUID) et les dossiers de structure
  (`documents/`, `eleves/photos/`…) sont gardés ; le registre `tenants/…` et les dossiers de secrets sont **écartés**.
- **Gardé tel quel** : identifiants techniques (UUID), liens entre tables, montants, quantités, statuts,
  dates de création, noms de classes, de salles et de matières, libellés de nomenclature.

L'outil est **fermé par défaut** : chaque colonne des tables du Next.js est déclarée une par une
(`ent-rust/crates/ent-reprise/donnees/liste_blanche.tsv`). Si l'export contient une table ou une colonne
qui n'y est pas, il **s'arrête avant d'écrire quoi que ce soit** (code de sortie 2) et en donne la liste.

## Comment vérifier le résultat

1. **Le contrôle automatique.** À la fin, l'outil relit tout son résultat et cherche les noms, e-mails,
   téléphones, INE et IBAN **réels** qu'il avait vus. Le rapport doit dire
   **« Aucune valeur réelle retrouvée dans l'export »** et **« Résultat : ZÉRO occurrence »**. Sinon
   l'export n'est **pas** livré (les fichiers sont supprimés), le programme affiche les colonnes en cause
   (jamais les valeurs) et sort avec le code 3.
2. **L'empreinte.** `ent-reprise verifier --dossier anonymise` recalcule les empreintes, relit le
   schéma et doit afficher **« CONFORME »**. Vous pouvez aussi utiliser votre propre outil :
   `sha256sum -c SHA256SUMS` (dans le dossier `anonymise`).
3. **Un sondage à la main (2 minutes, recommandé).** Cherchez dans `anonymise/dump.sql` quelques noms
   que vous connaissez (le vôtre, celui d'un collègue, d'un élève), par exemple
   `grep -i -c "dupont" anonymise/dump.sql` : le résultat doit être **0**. Regardez aussi
   `anonymise/objets_s3.tsv` et quelques lignes du `dump.sql` (table `eleve`, `foyer_responsable`, `user`).
4. **Le rapport.** Parcourez `rapport_anonymisation.txt` : vous devez y reconnaître les tables sensibles
   (`eleve`, `foyer_responsable`, `user`, `personnel`, messages, absences…) avec une règle « pseudo:… ».

## Ce que vous nous envoyez

- **Oui** : le dossier `anonymise/` (ou au moins `dump.sql`, `objets_s3.tsv`, `rapport_anonymisation.txt`
  et `SHA256SUMS`), et **l'empreinte de `EMPREINTE.txt` par un autre canal** (message ou appel) : nous
  la recalculons à la réception.
- **Non, jamais** : l'export d'origine, la liste d'objets d'origine, la clé, un extrait avec de vraies valeurs.
- Le rapport de l'étape 2 (`inspecter`) peut nous être envoyé : il ne contient que des noms de tables,
  de collections « sûres » et des nombres.

## Si l'outil refuse

| Code | Message | Que faire |
|---|---|---|
| 1 | erreur (fichier absent, adresse de connexion, clé trop courte, dossier de sortie non vide, dump compressé…) | lire le message, corriger la commande |
| 2 | « REFUS : l'export contient des éléments non déclarés… » ou « NON CONFORME » | **ne rien envoyer d'autre que le texte affiché** ; nous complétons la liste blanche |
| 3 | « REFUS : le contrôle final a retrouvé des valeurs réelles… » | relancer avec **une autre clé** (une collision fortuite est possible) ; si l'écart persiste, nous envoyer le texte (colonnes seulement) |

Faux positif possible : un mot courant qui est aussi le nom d'une personne réelle (par exemple une salle
« Salle Descartes » et un élève de ce nom). Après avis de l'équipe, on peut lever le contrôle pour ce
mot : `--mots-autorises mots.txt` (un mot par ligne) ; la cellule entière reste contrôlée.

## Limites à connaître (honnêtes)

- Les dictionnaires de clés JSON sont des **hypothèses** construites sans aucune donnée réelle : une clé
  inconnue est traitée prudemment (texte remplacé) et listée au rapport ; elle nous servira à les compléter.
- Un nom propre perdu **dans un texte libre** est supprimé parce que **tout** le texte l'est ; mais le
  contrôle final ne retrouve que les valeurs vues dans les colonnes d'identité (noms de 6 lettres et plus,
  e-mails, téléphones, INE, IBAN).
- Les noms composés (« auteur », « enseignant ») ne sont pas liés au pseudonyme du prénom et du nom de la
  même personne ailleurs : seules les valeurs identiques entre elles le sont.
- L'INE pseudonymisé a la forme d'un INE mais pas une clé de contrôle valide.
- Un identifiant en minuscules et chiffres seulement (sans tiret) d'au moins 16 caractères est gardé comme
  « technique » ; un identifiant formé d'un nom avec tirets ne l'est jamais.
- Archive « répertoire » issue de PostgreSQL 17 ou 18 : prévue mais **non vérifiée sur un vrai export** ;
  en cas de doute, utilisez `--format=plain`.
- Les fichiers du stockage ne sont ni lus ni copiés : l'anonymisation de leur **contenu** (photos, PDF) est
  hors périmètre ; elle se fera par des fichiers factices lors de la répétition.

## Pour l'équipe

Code : `ent-rust/crates/ent-reprise`. Liste blanche et schéma : `donnees/`, régénérés par
`outils/classer.py` (toute colonne non classée y fait échouer la génération). Garde de CI :
`schema::tests::garde_liste_blanche_couvre_le_schema`. Décisions : D-057 ; conception : socle 28 §6.4 et T0 §9.
