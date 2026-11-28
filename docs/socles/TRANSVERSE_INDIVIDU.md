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
