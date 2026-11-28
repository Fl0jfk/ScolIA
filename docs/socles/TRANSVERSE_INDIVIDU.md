# Transverse : la deuxième personne fondée sur la personne physique

Conception courte du palier (2026-11-27), puis livraison (§9, qui fait foi là où elle diffère). Origine : faible 9 de la contre-revue V3
(`42_VOYAGES_V3_LIVRAISON.md`) : « la règle 3 repose sur `user_id`, pas sur la personne physique : un même individu à
deux comptes n'est pas reconnu ». Le même défaut existe partout où une règle dit « une autre personne » ou « jamais pour
soi ni pour son enfant » et compare deux comptes. Réserve connue de `04_ADMINISTRATIF.md` §22 : « un agent qui est
aussi parent a deux comptes ; seule la déclaration protège ».

## 1. Le problème

- Un compte (`ent_user`) a au plus **une** fiche personne par établissement (`ent_personne_compte_uq`), et une fiche a
  au plus un compte. Le professeur-parent qui a **un seul compte** (casquettes, migration 357) a une seule fiche : il est
  déjà reconnu.
- Mais le cas courant reste **deux comptes** : l'adresse professionnelle (casquette personnel) et l'adresse personnelle
  (casquette parent), donc **deux fiches** sans rien qui les relie. Ou bien une fiche parent **sans compte** (parent
  saisi par le secrétariat) et un agent sur une autre fiche.
- Conséquences aujourd'hui :
  1. **Règle 3 des voyages** (`ent_sortie_argent_decideur_interdit`, 653) : le comptable qui est parent d'un élève
     accorde une aide à son propre enfant depuis son compte professionnel ; ni l'application ni la base ne le voient.
  2. **Quatre yeux** : la seconde validation compare `par_user_id` et `demande_par_user_id`. Deux comptes du même
     humain habilités passent pour deux personnes.
  3. **Conflit d'intérêts (R-A18)** : la détection automatique ne voit que la fiche du compte de l'agent.

## 2. Principes

1. **L'individu ne fait que restreindre.** Reconnaître deux comptes comme le même individu ajoute des refus, jamais un
   droit. Une règle qui *permet* parce que c'est soi (l'autrice révoque son autorisation, l'auteur modifie son brouillon,
   l'agent annule sa propre absence) reste sur le **compte**.
2. **Pas de fusion de comptes** (D-013) ni de fiches (123 refuse déjà deux fiches à comptes différents) : on **relie**
   des fiches, chacune garde ses coordonnées, ses liens et son compte. Le lien est réversible et tracé.
3. **Un humain décide.** Une ressemblance (même téléphone, même adresse, même nom) donne une **proposition**, jamais un
   rapprochement automatique (D-069 : faux positif = refus à tort d'un collègue, faux négatif = statu quo d'aujourd'hui).
4. **Dans un établissement seulement.** Aucune visibilité ni aucun rapprochement entre établissements (D-013).
5. **Une seule définition**, en base, servie à l'application (message clair) et aux déclencheurs (garde) comme la règle
   3 aujourd'hui. Aucun module ne réécrit sa propre requête « même personne ».

## 3. Modèle (migration 279, socle 4)

`ent_individu_lien` : deux fiches du même établissement sont le même individu.

| Colonne | Sens |
|---|---|
| `personne_a`, `personne_b` | les deux fiches, rangées (`a < b`), distinctes, clés composites `(etablissement_id, id)` |
| `origine` | `declaration_direction`, `declaration_confirmee` (déclarée depuis un compte, confirmée depuis l'autre), `proposition_acceptee` |
| `motif_code` | `deux_comptes`, `fiche_sans_compte`, `doublon_agent`, `autre` (aucun texte libre en base) |
| `pose_par_user_id`, `pose_le` | l'auteur (jamais le sujet pour `declaration_direction`) |
| `leve_par_user_id`, `leve_le`, `leve_motif_code` | levée unique ; ligne sinon immuable |

Une seule ligne active par paire (index unique partiel). La relation est **transitive** : l'individu d'une fiche est la
fermeture des liens actifs, plus la chaîne de fusion (`fusionnee_dans`).

`ent_individu_declaration` : déclaration en deux temps depuis un compte (A) vers un autre compte (B) du même
établissement ; B confirme dans les 24 h depuis sa propre session (preuve de possession des deux comptes) ; sinon
elle expire. Ajout seul ; l'issue est une colonne posée une fois.

`ent_individu_proposition_ecartee` : paire écartée par la direction (motif codé) : jamais reproposée.

Fonctions (SQL, `STABLE`, `search_path` figé, `SECURITY INVOKER`) :
- `ent_individu_personnes(etab, personne) → uuid[]` : fermeture.
- `ent_individu_personnes_du_compte(etab, user) → uuid[]` : fiches de l'individu du compte (fiche du compte, fiche de
  l'élève dont c'est le compte, plus leur fermeture).
- `ent_individu_comptes(etab, user) → uuid[]` : tous les comptes de l'individu (fiches, comptes d'élève), `user`
  compris.
- `ent_meme_individu(etab, u1, u2) → bool`.

Port Rust `ent_db::individu` : `meme_individu`, `comptes_de`, `personnes_de` (une requête, dans la transaction de
l'appelant), et les commandes ci-dessous.

## 4. Commandes et routes (API seule, pas d'écran : frontend en retrait)

| Geste | Qui | Garde |
|---|---|---|
| Déclarer « cet autre compte est aussi moi » (`POST /individu/declarations`, cible par e-mail du compte) | tout compte de l'établissement | réponse neutre si l'e-mail est inconnu (anti-énumération) ; 5 par jour |
| Confirmer (`POST /individu/declarations/{id}/confirmer`) | le compte cible seulement | dans les 24 h, pas depuis le compte déclarant |
| Relier deux fiches (`POST /individu/liens`) | `personnes.individu.relier` (direction, secrétariat) | motif codé ; fiches du même établissement, pas déjà le même individu (relier ses propres fiches est permis : cela ne fait qu'ajouter des refus, voir §9) |
| Lever (`POST /individu/liens/{id}/lever`) | `personnes.individu.lever` (direction seule) | **jamais par l'individu concerné** (lever retire des refus) ; motif codé |
| Propositions (`GET /individu/propositions`) | `personnes.individu.relier` | paires de fiches de comptes différents, dont l'une est un agent, qui partagent un téléphone E.164 ou une adresse e-mail (fiche ou compte) ; nombres et désignations, jamais la coordonnée |
| Écarter une proposition | `personnes.individu.relier` | motif codé |
| Lire mes liens (`GET /individu/moi`) | le compte | sans les fiches des autres |

Journal : les tables elles-mêmes, en ajout seul (auteur, date, motif codé, levée). Aucun événement métier n'est émis
dans ce palier (voir §9).

## 5. Où l'appliquer (inventaire de ce palier)

**Restrictions converties en « même individu »** :

| Socle | Règle | Aujourd'hui | Lot |
|---|---|---|---|
| 42 | Règle 3 : pas pour son enfant ni pour son compte (`ent_sortie_argent_decideur_interdit`) | `l.user_id = usr`, `p.user_id = usr`, `ec.user_id = usr` | 2 |
| 42 | Seconde validation : le valideur n'est pas le demandeur (application + déclencheur 653) | `par_user_id = demande_par_user_id` | 2 |
| 42 | Annulation au-dessus du seuil : ni l'auteur ni le demandeur | égalité de comptes | 2 |
| 42 | Contournement : la demandeuse ne décide pas (application + 658) | égalité de comptes | 2 |
| 42 | Bordereau remis par une autre personne que la saisie | égalité de comptes | 2 |
| 4 | Conflit d'intérêts R-A18, détection automatique | fiche du compte seulement | 3 |
| 18 | FRM-145 : un formulaire ne se valide pas soi-même (auteur, co-auteurs) | égalité | 3 |
| 20 | C8 : réouverture du cahier validée par une autre personne (application + CHECK de 2407) ; accès d'inspection jamais à soi | égalité | 3 |
| 41 | Sortie de groupe : l'auteur ne la décide pas | égalité | 3 |
| 15 | Absence d'un personnel : jamais validée ni refusée par l'absent | égalité | 3 |
| 7 | Signature : l'agent ne constate pas sa propre signature, ne contrôle pas son propre papier | égalité | 3 |
| 6 | Délégation : jamais à soi-même | égalité d'agents | 3 |
| 29 / N1 | Reprographie R-DM40 : jamais sur sa propre demande | égalité | 3 |

**Gardées sur le compte** (principe 1) : révocation d'un contournement par son autrice, auto-validation d'une réservation
de salle par son valideur (`agenda_lieux`), droits d'auteur sur un brouillon, annulation de sa propre absence.

**Hors palier, notés** : noter son propre enfant (Q-P3, avertissement seulement, socle 21) ; quatre yeux des
diffusions (NTF-50, pas de commande encore) ; RGPD (PR 165, 166 en cours : ne pas chevaucher) ; écrans.

## 6. Cas limites

- **Fiche sans compte** reliée à un agent : la règle 3 la voit par `personne_id` des liens et du compte payeur, pas par
  un compte (c'est le gain principal pour les fiches saisies par le secrétariat).
- **Chaîne** A–B, B–C : A et C sont le même individu ; lever A–B ne lève pas B–C.
- **Fusion** de fiches : la fiche absorbée suit sa cible (`fusionnee_dans`), défusion = retour.
- **Lien levé** : n'agit plus dès la levée ; les écritures passées restent (rien n'est rejugé).
- **Concurrence** : le déclencheur juge au moment de l'écriture ; un lien posé entre le contrôle et l'écriture donne le
  `23514` déjà traduit (`db_pouvoir`), tracé comme la règle 3.
- **Direction unique** sujette : elle ne lève pas son propre lien ; un autre membre de la direction, ou personne
  (refus tant qu'il n'y en a pas : prudent).
- **Élève majeur AED** (P48) relié : ses propres classes lui restent fermées (R-P5, inchangé) ; la règle 3 le voit aussi.
- **Performance** : fermeture récursive bornée (profondeur 16, établissement seulement) ; index par fiche.

## 7. Lots

1. **Socle de l'individu** : 279 (tables, fonctions, RLS, gardes d'ajout seul), port `individu`, actions
   `personnes.individu.relier/lever`, routes, propositions, tests (fermeture, levée, chaîne, fusion, déclaration en deux
   temps, expiration, anti-énumération, isolation entre établissements, garde d'ajout seul).
2. **Voyages** : 659 redéfinit `ent_sortie_argent_decideur_interdit` et les gardes de seconde personne et de
   contournement sur l'individu ; application convertie ; tests sur un individu à deux comptes et sur une fiche sans
   compte (nouveau fichier de tests : `tests_voyages_v3.rs` est touché par la PR 167).
3. **Autres socles** : conversions du tableau, CHECK de 2407 doublé d'un déclencheur (migration 2421), déclencheurs
   R-AP10 des absences (migration 2316), conflit d'intérêts ; un test par règle convertie ; garde statique : aucune comparaison `== a.user_id` restante dans les
   refus listés.
4. **Docs** : livraison (§9), `PROJECT_STATUS.md`, `ROLES_ET_DROITS.md`, `MIGRATIONS.md`.

## 8. Choix prudents (D-069)

- Rapprochement jamais automatique ; propositions limitées aux paires où un agent est en jeu.
- Lever retire des refus : direction seule, jamais l'individu concerné.
- Déclaration par la personne : confirmation depuis l'autre compte, sinon elle ne vaut rien.
- Les règles qui accordent restent sur le compte.

## 9. Livraison (2026-11-27, branche `claude/project-thread-lfmutb`)

**Niveau : implémenté + testé en local (PostgreSQL 16, feature `labo`, `ENT_ENV=test`) ; CI sur la PR en brouillon ;
non validé par Florian, pas prêt pilote.** Pas d'écran (frontend en retrait) : API seule.

### 9.1 Ce qui est livré

- **Migration 279** `personne_individu` : `ent_individu_lien`, `ent_individu_declaration`,
  `ent_individu_proposition_ecartee` (RLS, clés composites, motifs codés, ajout seul tenu par la base : TRUNCATE refusé,
  DELETE seulement par la cascade d'une fiche supprimée, levée et issue posées une fois ; une déclaration expirée ne se
  confirme pas ; verrou consultatif par établissement à chaque pose ; refus si les deux fiches sont déjà le même
  individu). Fonctions `ent_individu_personnes`, `ent_individu_personnes_du_compte`, `ent_individu_comptes`,
  `ent_meme_individu` (`STABLE`, `search_path` figé, relations `public.`, fermeture bornée à 16).
- **Port Rust** `ent_db::individu` : `meme_individu`, `meme_individu_opt`, `comptes_de`, `personnes_du_compte`,
  `compte_est_agent`, `agents_meme_individu`, et les commandes `individu_*_for_actor`.
- **Actions** `personnes.individu.relier` (direction, secrétariat) et `personnes.individu.lever` (direction) ;
  `TOUTES` 286 → 288.
- **Routes** (dans `dossier.rs`) : `GET /dossier/individu/moi`, `POST /dossier/individu/declarations` (202, réponse
  neutre), `POST /dossier/individu/declarations/{id}/reponse`, `GET|POST /dossier/individu/liens`,
  `POST /dossier/individu/liens/{id}/lever`, `GET /dossier/individu/propositions`,
  `POST /dossier/individu/propositions/ecarter`.
- **Migration 659** `sortie_regle3_individu` (lot 2) : `ent_sortie_argent_decideur_interdit` voit les liens familiaux,
  comptes d'élève et comptes payeurs de **toutes** les fiches de l'individu (par `personne_id`, donc aussi une fiche sans
  compte) ; seconde validation (653), décision de contournement (658) et contournement ouvert (657) jugés par
  `ent_meme_individu`. Application : annulation au-dessus du seuil, validation, contournement, bordereau.
- **Migration 2421** `cdt_individu` (lot 3) : réouverture du cahier et accès d'inspection (nouveau déclencheur ; les
  CHECK de 2407 restent, plus faibles et toujours vrais).
- **Migration 2316** `absp_individu` (lot 3) : `ent_trg_absence_rh_valideur`, `ent_trg_absp_circuit_valideur`,
  `ent_trg_absp_decision` (R-AP10) recréées sur l'individu (2300 et 2302 inchangées, règle 25).
- **Conversions de l'application (lot 3)** : formulaires (FRM-145, auteur et co-auteurs), cahier (accès d'inspection,
  prolongation, réouverture), sorties de groupe de l'internat (auteur), absences des personnels (R-AP10 : seulement les
  branches qui refusent ; `suis_absent` garde ce qu'il accorde à l'absent), projection historique `ent_absence_rh`,
  signature (constat au guichet, contrôle du papier : par compte **ou** par fiche du signataire), délégations (fiches
  d'agent reliées), reprographie R-DM40 (fiches reliées au demandeur ou au déposant), conflit d'intérêts R-A18
  (détection par les fiches de l'individu et par les conflits déclarés sur ses autres comptes).

### 9.2 Écarts avec la conception (font foi)

- **Relier ses propres fiches** est permis à la direction et au secrétariat : un lien ne fait qu'ajouter des refus
  (principe 1). Seule la **levée** est interdite à l'individu concerné.
- **Aucun événement** `individu.lien_pose` / `individu.lien_leve` : le journal est la table en ajout seul elle-même.
  Un événement (notification de l'intéressé) est une suite possible, pas un manque de sécurité.
- **Migration 2316** ajoutée (absences, en base) ; la conception ne citait que 2421.
- **Propositions** : sur l'adresse e-mail (fiche ou compte) ou le téléphone E.164 (`ent_tel_e164`) partagés, au moins un
  agent dans la paire, 200 au plus, sans la coordonnée ; aucune ne relie rien.

### 9.3 Tests

- `tests_individu.rs` (7) : chaîne et levée, la direction ne lève pas son propre lien, déclaration en deux temps
  (expiration, quota, anti-énumération), ajout seul, cloisonnement entre établissements, fusion de fiches,
  propositions sans rapprochement automatique.
- `tests_individu_voyages.rs` (5) : règle 3 par un second compte parent et par une fiche parent sans compte, seconde
  validation et annulation (application et base), contournement, bordereau.
- `tests_cahier_individu.rs` (sous-module de `tests_cahier`) : réouverture refusée à un second compte du demandeur,
  dans l'application et en base (2421).
- `tests_individu_socles.rs` : R-AP10 (application et les trois déclencheurs de 2316), délégation, accès d'inspection
  (2421, en base), conflit d'intérêts R-A18 par une fiche parent sans compte reliée, et une **garde statique** : aucune
  égalité de comptes ne reste dans les refus convertis.

### 9.4 Limites connues (faits)

- Formulaires, sorties de groupe, signature et reprographie : la conversion est un appel au port (testé) ; la garde
  statique vérifie que l'ancien motif a disparu **et** que l'appel au port est présent ; pas de test de bout en bout pour
  ces quatre-là.
- Le tableau de bord des conflits d'intérêts (direction) liste toujours les conflits par compte ; seule la détection
  (qui ferme les groupes) voit l'individu.
- Hors palier, inchangés : noter son propre enfant (Q-P3), quatre yeux des diffusions (NTF-50), RGPD (PR 165, 166),
  écrans.

### 9.5 Contre-revue indépendante (2026-11-27)

Rapport : `/mnt/project-files/revues/TRANSVERSE_INDIVIDU_CONTRE_REVUE.md`. **Aucun constat élevé, aucune escalade.**
Fonctions recréées comparées aux originaux : identiques sauf la comparaison convertie.

| Constat | Suite |
|---|---|
| M1 propositions quadratiques | **corrigé** : la jointure part des fiches d'agent ; une coordonnée partagée par plus de 5 fiches (standard, remplissage d'import) ne propose rien. |
| M2 noms de fiches protégées | **corrigé** : la liste des liens masque le nom d'une fiche fermée au lecteur (`personnes_en_conflit` : R-A18 et adresse protégée) ; une proposition qui en touche une n'est pas montrée. |
| M3 tests insuffisants | **corrigé** : cloisonnement avec des fiches dans les deux établissements et lien inter-établissements refusé en base ; contournement refusé par le déclencheur de 659 en SQL direct ; réouverture du cahier (2421) testée ; garde statique positive. |
| F2 levée en SQL direct | **corrigé** : la base refuse la levée par l'individu concerné (et sans auteur). |
| F3 fermeture non bornée | **corrigé** : la pose d'un lien est refusée au-delà de 16 fiches (test). |
| F1 défusion | noté : défaire une fusion retire l'effet de la fusion, pas un lien d'individu ; c'est un geste de `personnes.fusionner` (admin, direction), sans la garde « jamais l'intéressé ». Suite possible. |
| F4 écarter irréversible | noté (D-069) : écarter ne masque qu'une suggestion ; la direction peut toujours relier les deux fiches. |
| F5 nom du déclarant montré à la cible | noté : la confirmation se fait depuis la session de la cible, qui sait si l'autre compte est le sien ; aucune coordonnée montrée. |
| F6 élève sans compte signataire | noté : un élève signataire sans compte n'a pas de fiche `ent_personne` à relier. |
| F7, F8 | notés : restrictions d'usage, pas de fuite ; la notification de l'intéressé à la pose d'un lien est une suite produit. |

Branche signalée `absences_personnels.rs` (prolonger, écourter) : elle **accorde** à l'absent ; elle reste sur le compte
(principe 1).

## 10. Complément : écrans et avis à l'intéressé (2026-11-28, branche `claude/project-thread-isgco5`, sur la PR 170)

**Niveau : implémenté + testé en local (PostgreSQL 16, feature `labo`) ; CI sur la PR en brouillon ; non validé par
Florian, pas prêt pilote.** Lève deux « non fait » de §9 : les écrans et la notification de l'intéressé. Aucune migration.

### 10.1 Avis à l'intéressé

Dans l'ENT (toujours) et par e-mail ou push selon ses préférences (jamais de SMS), source `systeme`, une clé par geste
(idempotent), **dans la transaction du geste** (une erreur annule les deux ; l'expédition reste différée).

| Geste | Prévenus | Pas prévenus |
|---|---|---|
| Lien posé (direction, secrétariat) | tous les comptes de l'individu ainsi formé (fiches reliées, fusions, comptes d'élève) | l'auteur |
| Lien levé (direction) | les comptes des deux côtés, calculés **avant** la levée | l'auteur |
| Déclaration reçue | le compte cible | le déclarant (réponse neutre inchangée) |
| Déclaration confirmée | tous les autres comptes de l'individu ainsi formé, déclarant compris | la cible, qui vient de confirmer |
| Déclaration refusée | personne (anti-énumération) | |

Textes sans nom, sans fiche, sans coordonnée (ils peuvent sortir par e-mail), au plus 240 caractères (un push coupe
au-delà ; testé). Après un **refus**, le même déclarant ne peut plus redemander à la même cible pendant 30 jours : même
réponse neutre, aucun avis (contre-revue, M1 : sans cela, refuser rouvrait la porte à des avis répétés).

« Moi » (`GET /dossier/individu/moi`) renvoie aussi `liens` : origine, motif, date des liens actifs de l'individu,
**jamais l'autre fiche** (un lien posé à tort désignerait une autre personne). Les types vivent dans `ent-types::individu`
(une seule définition, réexportée par `ent-db`) avec les listes fermées des motifs.

### 10.2 Écrans (Leptos, `modules/individu/`)

- **Mes autres comptes** (`/web/securite/mes-comptes`, tout compte, lien depuis « Sécurité du compte ») : ce que
  l'établissement reconnaît (nombres, liens datés), demandes reçues (« C'est moi » avec confirmation qui rappelle
  l'effet, « Ce n'est pas moi »), déclaration d'un autre compte par son adresse (contrôle local, réponse identique que
  l'adresse soit connue ou non, saisie effacée seulement après l'accord du serveur), « Pourquoi ? ».
- **Fiches reliées** (`/web/direction/fiches-reliees`, menu si `personnes.individu.relier` sur l'établissement) :
  propositions (raison lisible sans la coordonnée ; « Relier… », « Écarter… » avec motif), relier deux fiches choisies par
  la recherche existante (`personnes.gerer` sur l'établissement ; nom, compte, élèves liés, **aucune coordonnée**
  affichée), liens actifs en tableau (cartes sous 640 px), « Lever… » si `personnes.individu.lever`. Une boîte de
  dialogue par action, motif en liste fermée copiée d'`ent-types`, écritures par `Ecriture::lancer`. Aucune feuille de
  style nouvelle (classes existantes : moins de conflits avec les écrans écrits en parallèle).

### 10.3 Tests

- `tests_individu_avis.rs` (4) : pose et levée (destinataires, auteur exclu, chaîne, e-mail planifié, pas de SMS, « moi »
  sans fiche), déclaration (cible seule, un avis pour deux demandes, rien au refus, pas de relance après refus),
  confirmation qui prévient un compte déjà relié, longueur des textes.
- `tests_individu.rs` : l'expiration passe par un autre déclarant (la relance après refus est désormais bloquée).
- Modèle d'écran (8) : libellés de chaque motif du serveur, raisons, résumés, adresse, tris, fiche trouvée sans
  coordonnée, choix des deux fiches.
- `ent-web-server/tests/individu.rs` (3) contre le vrai backend, établissement neuf : pages rendues (CSP, aucun style en
  ligne), relier et lever selon les droits (professeur 403, secrétariat relie mais ne lève pas, direction lève), « Mes
  autres comptes » (réponse neutre, demande vue de la seule cible, refus sans lien).

### 10.4 Contre-revue indépendante (2026-11-28)

Rapport : `/mnt/project-files/revues/TRANSVERSE_INDIVIDU_ECRANS_CONTRE_REVUE.md`. Aucun constat élevé.

| Constat | Suite |
|---|---|
| M1 avis répétés après refus | **corrigé** : blocage silencieux de 30 jours de la même paire après un refus (test). |
| M2 temps de réponse selon que l'adresse existe | noté : l'écart existait (PR 170 : insertion et validation) ; borné par le quota de 5 par jour. Égaliser le temps est une suite possible. |
| F1 confirmation : seul le déclarant prévenu | **corrigé** : tout l'individu formé, sauf la cible (test). |
| F2 texte coupé dans un push ; renvoi à un écran absent du mobile | **corrigé** : 240 caractères au plus (test) ; les textes disent « dans ScolIA sur le web ». |
| F3 droit de recherche testé sans la portée | **corrigé** : `a_etablissement("personnes.gerer")`. |
| F4 saisies effacées avant la réponse ; contrôle refait à la main | **corrigé** : effacées après l'accord du serveur ; `lien_a_envoyer` utilisé. |
| F5 nombre de fiches trouvées non annoncé ; focus après « Choisir » | nombre annoncé (`role="status"`) ; le focus reste une limite (pas de test au lecteur d'écran). |
| F6 libellé de recherche ; nom d'une fiche protégée dans la recherche | libellé corrigé ; la recherche (`/dossier/personnes`) est antérieure et inchangée : noté. |
| F7 texte « réservé » avant la réponse du moteur | texte neutre. |
| F8 « 1 liens actifs », surtitre, « Ce n'est pas moi » sans confirmation | pluriel et surtitre corrigés ; le refus reste direct (la cible peut toujours déclarer elle-même ; l'écran dit le blocage de 30 jours). |
| F9 ce que les tests ne prouvent pas | en partie couvert (relance après refus, confirmation) ; restent notés : destinataires sur un compte d'élève ou une fiche fusionnée (couverts par la fonction SQL déjà testée, pas par un test d'avis), masquage des boutons sans droit (pas de test de rendu avec session). |

### 10.5 Limites (faits)

- Applis mobiles : levé au §11 (« Mes autres comptes » dans « Mon compte »).
- Parcours navigateur : voir §11.4. Aucun test au lecteur d'écran réel.
- Le tableau de bord des conflits d'intérêts liste toujours par compte (inchangé, §9.4).

## 11. Complément : applis mobiles et parcours navigateur (2026-11-28, branche `claude/project-thread-7n7s4t`, sur la PR 172)

**Niveau : implémenté ; cœur Kotlin testé ; test de contrat Rust contre le vrai serveur vert ; Swift, interface Compose
et tests Robolectric écrits mais non compilés ici (pas de SDK iOS, dépôt Google Maven inaccessible) ; non validé par
Florian.** Aucune migration, aucune route nouvelle.

### 11.1 Ce qui est porté, et ce qui ne l'est pas

- **« Mes autres comptes »** dans les deux applis, pour **tout compte** (comme sur le web) : Android, section de
  « Mon compte » (`ecrans/MesAutresComptes.kt`) ; iOS, écran ouvert depuis « Mon compte » (`MesAutresComptes.swift`).
  Ce que l'établissement reconnaît (nombres, liens datés : origine et motif, **jamais l'autre fiche**), demandes reçues
  (« C'est moi », « Ce n'est pas moi »), déclaration d'un autre compte par son adresse (réponse neutre identique au
  web), « Pourquoi ? ». Mêmes textes que le web (copiés de `modele.rs`, testés des deux côtés).
- **Écarts voulus avec le web** : le refus demande aussi une confirmation (un appui de travers est plus probable sur un
  téléphone, et il bloque ce compte 30 jours) ; l'adresse du compte connecté est refusée localement (le clavier la
  propose ; le serveur l'ignorerait en silence, la refuser ne révèle rien) ; heures dans le fuseau de l'appareil.
- **Direction et secrétariat : rien sur mobile.** Relier deux fiches demande la recherche de fiches, un motif et une
  vérification : c'est une tâche de bureau (D-003). Les propositions portent des noms de personnels qu'il vaut mieux ne
  pas garder sur un téléphone. Les avis renvoient au web pour ce geste.
- **Jamais en cache hors ligne** : demandes datées, réponses qui exigent le réseau.
- **Écritures** : clé d'idempotence gardée par intention (`ClesIntention`), une écriture à la fois, saisie effacée
  après l'accord du serveur, liste rechargée après un refus définitif (demande expirée ou déjà traitée), 401 → session
  perdue, 426 → écran de mise à jour. Résultat annoncé (TalkBack `liveRegion`, VoiceOver `announcement`).
- **Avis** (`ent-db/src/individu.rs`) : « Mes autres comptes (Compte sur le web, Mon compte dans l'appli) » au lieu de
  « sur le web » ; 240 caractères au plus (inchangé, testé).

### 11.2 Contrat

Les trois routes appelées (`GET /dossier/individu/moi`, `POST …/declarations`, `POST …/declarations/{id}/reponse`) sont
listées dans `ent-rust/contrat/mobile/individu/routes.txt` ; `ent-web-server/tests/individu_contrat_mobile.rs` les joue
en Bearer avec l'en-tête des applis contre le vrai backend, et compare les réponses normalisées aux fixtures du même
dossier (`ENT_CONTRAT_ECRIRE=1` pour régénérer). Les cœurs Kotlin (`IndividuTest`) et Swift (`IndividuTests`) les
décodent avec les vraies méthodes et vérifient la liste blanche. **Dossier à part** de `contrat/mobile/` et méthodes en
fonctions d'extension (fichiers `Individu.kt`, `Individu.swift`) pour ne pas toucher `routes.txt`, le banc MB2,
`ApiClient` et `Modeles`, modifiés par les PR 171 et 173 : **à fondre dans le banc MB2 une fois celles-ci fusionnées**
(et la PR 173 vérifie que chaque clé de fixture est lue : les trois champs de chaque type le sont).

### 11.3 Tests

- Kotlin `:core` : 11 tests dans `IndividuTest` (fixtures réelles, 202 vide, `null`, chemin encodé, clé gardée sur une
  issue incertaine et oubliée sur un 409, liste blanche, contrôle d'adresse en octets et adresse du compte, libellés,
  tris, dates) ; 76 tests du module, tous verts.
- Swift `ScolIACoreTests/IndividuTests.swift` : mêmes cas ; **non exécuté** (pas de chaîne Swift ici).
- Robolectric `MesAutresComptesComposeTest` (3) : confirmer et refuser passent par une confirmation, contrôle local
  puis réponse neutre ; **non exécuté** (Google Maven inaccessible).
- `ent-web-server --test individu_contrat_mobile` : vert (lancé quatre fois) ; `--test individu` (PR 172) : 3 verts ;
  modèle d'écran web : 8 verts ; clippy `-D warnings` wasm `hydrate` et serveur `labo` ; `cargo fmt --check` web.

### 11.4 Parcours navigateur (web, PR 172 et ce complément)

`ent-rust/web/tests-navigateur/individu.cjs` (Playwright, Chromium sans fenêtre, axe-core 4.10 WCAG 2.1 A et AA) contre
le serveur de labo construit (`cargo leptos build --bin-features labo`) : « Mes autres comptes » et « Fiches reliées »
à **375 px et 1280 px**, **au clavier seulement** (Tab, Entrée, Échap) : confirmer une demande reçue, déclarer une
adresse fausse puis juste, chercher et choisir deux fiches, relier avec un motif, lever. **72 contrôles verts, deux
lancements de suite** : aucune violation axe-core (page, résultats de recherche, chaque boîte de dialogue ouverte, après
écriture), aucun défilement horizontal, focus dans chaque boîte et rendu au bouton à la fermeture par Échap, aucune
erreur dans la console. Il a trouvé deux défauts de la PR 172, corrigés ici :

- le champ e-mail de « Déclarer un autre compte » (`type=email`) laissait la bulle du navigateur bloquer l'envoi
  sans le message de l'écran : formulaire en `novalidate`, le contrôle de l'écran s'affiche et s'annonce ;
- après « Choisir » (et « Changer »), le focus tombait sur la page (F5 de la contre-revue des écrans) : il va
  désormais sur « Changer » (et sur le champ de recherche).

Outillage, hors CI : Playwright et axe-core installés à part ; une fixture de contraste attend la fin des animations
d'entrée (une mesure pendant le fondu donne de faux échecs). À brancher sur la CI (job frontend) en suite.

### 11.5 Contre-revue indépendante (2026-11-28)

Rapport : `/mnt/project-files/revues/TRANSVERSE_INDIVIDU_MOBILE_CONTRE_REVUE.md`. Aucun constat élevé.

| Constat | Suite |
|---|---|
| M1 parcours navigateur : confirmation impossible sans fiche | **corrigé** : la direction rattache une fiche à chaque compte ; parcours vert. |
| F1 Android : rien quand aucune demande ; intertitres non annoncés comme titres | **corrigé**. |
| F2 nom accessible sans le libellé visible (WCAG 2.5.3) | **corrigé** sur les deux applis **et** le web (« C'est moi : … »). |
| F3 iOS propose l'adresse du compte, réponse « enregistrée » trompeuse | **corrigé** : refus local de l'adresse du compte (deux applis). |
| F4 demande expirée ou traitée restée affichée après le refus du serveur | **corrigé** : rechargement après un refus définitif. |
| F5 iOS : rechargement qui vide l'écran | **corrigé** : relance du chargeur. |
| F6 boutons côte à côte en grande taille de texte | **corrigé** : `FlowRow`, `ViewThatFits` (non vu sur appareil). |
| F7 longueur en unités UTF-16, saisie coupée à 254 | **corrigé** : octets, plus de coupe. |
| F8 test de contrat instable (nombre de chiffres de la fraction) | **corrigé** : seule la présence de la fraction est gardée. |
| F9 fixture sans vrai nom, commentaires faux, rejeu et 409 non joués | commentaires corrigés ; le nom affiché vaut « Un autre compte » pour un compte sans nom (fait du serveur) ; rejeu et 409 couverts côté cœur (`IndividuTest`). |
| F10 test du refus sur un 429 (incertain) | **corrigé** : 409 définitif (clé oubliée) et 503 incertain (clé gardée). |
| F11 heure de l'appareil, refus confirmé sur mobile seulement, 426, doc, anciennes applis | 426 traité ; doc à jour ; le reste est voulu (§11.1) ou noté (§11.6). |

### 11.6 Limites (faits)

- Swift (cœur, tests, écran) et interface Compose (écran, Robolectric) **non compilés ni exécutés** ici.
- Aucun test au lecteur d'écran réel (TalkBack, VoiceOver, NVDA).
- Une appli antérieure à ce complément n'a pas l'écran vers lequel l'avis renvoie (le web l'a).
- `docs/MOBILE.md` (modifié par la PR 173) n'est pas mis à jour : à faire avec la fusion du contrat dans le banc MB2.
- Le parcours navigateur n'est pas dans la CI.
