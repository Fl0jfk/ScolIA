# Socle 27, lot G3a — **guichet des demandes de droits** (livraison)

Branche `ent-socle-27-code-g3a`, partie d'`origin/ENT` (règle 25). Migration **2544**.
Conception : `docs/socles/27_RGPD_ARCHIVES.md` §9 (entier), §4.1–4.2, §12, §13.1, §14.3, §17.4, §21, §26.
Première moitié du lot G3 : **le guichet**. L'extrait assemblé, les masquages de tiers et la remise
chiffrée viennent en **G3b** (migration 2545, non écrite ici).

---

## 1. En bref

| | |
|---|---|
| Migration | **2544** (`2544_rgpd_g3_demandes.sql`), en **fin** du tableau de `migrate.rs` (règle 38) |
| Tables créées | `rgp_demande`, `rgp_demande_droit`, `rgp_demande_identite`, `rgp_demande_journal`, `rgp_demande_compteur` (5) |
| Actions ajoutées au catalogue | 4 : `droits.recevoir`, `droits.traiter`, `droits.remettre`, `droits.arbitrer` |
| `Action::TOUTES` | **282 → 286** (règle 28) |
| `MIGRATION_TAG` | **inchangé** (3808 ; 2544 est sous ce numéro) |
| Code serveur | `ent-db/src/droits_demandes.rs` (service), `ent-api/src/rgpd_droits.rs` (17 routes), `ent-types/src/rgpd_droits.rs` (contrat JSON) |
| Code frontend | `ent-web-api/src/rgpd_droits.rs` (client typé), `ent-web-app/src/modules/rgpd_droits/` (5 écrans + modèle pur), `web/style/rgpd-droits.css` |
| Tests | 20 (ent-db, labo) + 10 (ent-db, natifs) + 3 (ent-api, HTTP) + 12 (modèle d'écran, natif) + 1 (droits d'affichage) + 4 (rendu serveur web) = **50** |
| Avertissements ajoutés | **0** (rustc et clippy 1.99, avec labo et tests) ; aucun `#[allow]` nouveau |
| `#[ignore]` ajoutés | **aucun** |

---

## 2. Fait

### 2.1 Les quatre entrées du guichet (§9.1, RGP-140 à RGP-143)

| Entrée | Fonction | Ce qui est particulier |
|---|---|---|
| Dépôt **connecté** (parent, élève, élève majeur, personnel) | `droits_deposer_connecte` | identité acquise par la session, **qualité constatée par le socle 4 à la date de la demande** ; accusé de réception automatique |
| Dépôt **sans compte** (ancien élève, parent sans compte) | `droits_deposer_sans_compte` | route **publique** ; réponse **invariable** qui ne dit jamais si l'établissement détient des données (R-RG42, CL27-66) ; aucun identifiant rendu ; débit limité |
| **Saisie au guichet** d'un courrier, d'un courriel ou d'un passage | `droits_saisir_au_guichet` | **date de réception = date du courrier** ; l'écart avec la saisie est conservé et signalé au-delà de 5 jours (RGP-303, CL27-64) |
| Transmission par le **DPO** ou la **CNIL** | `droits_saisir_au_guichet` (canaux `dpo`, `cnil`) | même circuit, canal distinct au journal |

**Numéro de demande par établissement** (E3) : `rgp_demande_compteur` + `rgp_demande_numero_suivant()`,
même mécanique que les compteurs du socle 7 (3702). L'incrément vit **dans la transaction du dépôt** : une
transaction annulée ne consomme pas de numéro — « sans trou visible ». Le compteur suit l'**année de
réception** (une demande de décembre saisie en janvier garde la référence de décembre) ; un déclencheur
interdit qu'il recule ou disparaisse. Référence lisible : `2027-014`.

**Accusé de réception** (RGP-145, nature `rgpd.demande_accusee` du socle 8) : date de réception, échéance et
contact du DPO quand il est nommé. **Aucun nom d'élève** dans la notification : la demande est nommée par sa
référence (consigne du lot, E14, R-RG32). Un demandeur sans compte dans l'établissement ne reçoit **rien**
par l'ENT : la réponse passe par un canal déjà au dossier (R-RG42), et le journal l'enregistre.

### 2.2 Identité et qualité (§9.2, §9.3, R-RG16)

* `droits_verifier_identite` : mode (`session_forte`, `code_courriel`, `piece`, `mandat`), résultat
  (`verifiee` / `echouee`), **qualité recalculée à la date de réception** par le socle 4 — un lien ouvert ou
  fermé depuis la demande ne change pas ce qui était vrai à la réception.
* `droits_demander_identite` : précision demandée, état `identite_a_verifier`, attente **comptée**.
* **Pièce d'identité** : référence d'un document du coffre (socle 7), **détruite à la clôture**
  (R-RG16, CL27-70) ; une contrainte de base interdit qu'une pièce « détruite » reste référencée. Seule la
  mention « vérifiée le, par » subsiste. La destruction du document lui-même reste au socle propriétaire
  (R-RG3) : le 27 relâche la référence et le trace.
* **Qualité constatée** : `confirmee`, `confirmee_sans_autorite` (CL27-03), `restreinte` (protection au
  dossier : adresse mise de côté ou « ne pas solliciter » — CL27-02 → arbitrage de la direction), `absente`
  (refus), `sans_objet`.
* **Grille de vérification d'un demandeur sans compte** (RGP-307) : six étapes, dont deux obligatoires
  (rapprocher un dossier ; reprendre contact par un canal du dossier). Servie par
  `GET /rgpd/guichet/referentiel` et affichée sur l'écran de saisie.
* **Qui peut demander quoi** (§9.3) appliqué au dépôt en ligne :
  * `titulaire_autorite` sur un élève **majeur** → refusé (l'élève exerce seul) ;
  * `titulaire_autorite` sans lien actif à la date → refusé ;
  * `eleve_mineur` sur un élève majeur → refusé ;
  * `tiers` et `parent_sans_autorite` → **renvoyés au guichet**, avec les pièces (CL27-07, R-RG38) ;
  * un sujet d'un autre établissement → **introuvable** (404 neutre), jamais « refusé ».
* **CL27-04 / S-550** : quand l'élève mineur dépose pour lui-même, `droits_titulaires_information`
  enregistre la décision d'informer (ou non) ses titulaires. Informer relève de `droits.traiter` ; **ne pas
  informer exige un motif sérieux écrit et relève de `droits.arbitrer`** (la direction prend la
  responsabilité). Les deux décisions sont tracées et révisables.

### 2.3 Horloge (§4.1, RGP-148)

* Échéance = **réception + 1 mois**, calculée au **jour de l'établissement** (`ent_aujourdhui_etab`). Les
  fins de mois saturent (31 janvier → 28 ou 29 février).
* Prolongation de **deux mois au plus**, **une seule fois**, et **seulement avant l'échéance du premier
  mois** (art. 12.3 : l'information est due dans ce premier mois ; passé ce délai, le retard est constaté tel
  quel, il ne s'efface pas). Motif obligatoire ; notification `rgpd.demande_prolongee` au demandeur **s'il a
  un compte ici**.
* **Alertes avant échéance** (S-544) : J-10 et J-3, aux comptes qui portent réellement `droits.traiter`
  (calcul par le moteur de droits, pas par une liste de rôles figée), idempotentes (une alerte par palier),
  sans nom d'élève. Aucune famille n'est destinataire.
* **Réouverture** (RGP-167) : l'horloge **ne repart pas à zéro**. L'échéance d'origine reste la référence et
  le retard éventuel reste visible ; le compteur de réouvertures et le journal gardent l'histoire.
* **Suspension pour précision d'identité** : voir §5 (règle non tranchée, valeur par défaut prudente).

### 2.4 Droits, cloisonnement, journal (§12.1, R-RG40)

* Quatre actions au catalogue, toutes de portée **établissement** :

  | Action | Qui l'a par défaut | Ce qu'elle ouvre |
  |---|---|---|
  | `droits.recevoir` | Direction, Secrétariat | enregistrer une demande, vérifier l'identité, demander une précision |
  | `droits.traiter` | Direction, Secrétariat | qualifier, affecter, ajouter un droit, accorder, informer les titulaires, clore |
  | `droits.remettre` | Direction seule | remettre l'extrait (consommée par le lot G3b) |
  | `droits.arbitrer` | Direction seule | prolonger, refuser, partiel, refuser une demande excessive, ne pas informer les titulaires, rouvrir |

  Le secrétariat **n'arbitre pas** (R-RG17) et la **remise** ne s'attribue que nommément (« secrétariat
  habilité », §12.1) : aucun profil historique ne la porte hors direction. Règle 21 respectée : seule la
  fonction `socle_27_rgpd` d'`actions.rs` est touchée, avec une ligne d'appel inchangée.
* **RLS stricte** sur les cinq tables (`ent_rls_poser`), clés composites `(etablissement_id, id)` pour les
  références, et `etablissement_id` toujours issu de la **session** (jamais du corps de la requête).
* **Journal des transitions** `rgp_demande_journal`, en **ajout seul** (ni `UPDATE`, ni `DELETE`, ni
  `TRUNCATE`), écrit **dans la transaction du fait** (R-RG40). Son `detail` est un texte court et neutre :
  jamais une donnée d'élève (E14, vérifié par un test).
* **Colonnes figées** par un déclencheur : numéro, année, canal, date de réception et date de saisie. Une
  demande ne se **supprime pas** (sa conservation suit la catégorie `rgpd.demande`).
* **Registre des tables** (E12) à jour : les cinq tables classées `conformite`, par établissement,
  `rgp_demande_compteur` sans données personnelles, `rgp_demande_identite` en `Fichiers::Reference`.
  **Neutralisation à la restauration : aucune** — une demande restaurée est vraie, son délai court depuis sa
  réception, et rien n'est à ré-émettre (la file du socle 8 a sa propre neutralisation). Catégorie de
  conservation : **`rgpd.demande`** (clôture + 60 mois), ajoutée à `categorie_de_table`.
* **R-RG19 (gel)** : `demandes_ouvertes_pour_sujet` est le point de lecture que le moteur de conservation du
  lot G6 interrogera pour geler les sorts finaux d'une personne. Appel système, exposé par aucune route.

### 2.5 API (17 routes) et écrans

Routes `/rgpd/*` (`ent-api/src/rgpd_droits.rs`). Refus **neutres** : 401 sans session, 403 quand le rôle n'a
pas le droit, 404 quand l'objet n'existe pas **dans cet établissement**, 409 sur un conflit d'état, 400 sur
une saisie invalide. `POST /rgpd/demandes/sans-compte` est la seule route publique ; son établissement vient
de l'**adresse** (jamais du corps), et son débit est limité par IP (5 / 15 min) et par contact (3 / h).

Écrans **Leptos** (`modules/rgpd_droits/`), `ui.rs` **non étendu** :

| Écran | Adresse | Contenu |
|---|---|---|
| File du guichet | `/web/rgpd/demandes` | compteurs, six filtres, recherche par référence, tableau dense (référence, arrivée, qualité, droits, état, échéance colorée) |
| Saisie d'un courrier | `/web/rgpd/demandes/saisir` | formulaire étiqueté, date de réception expliquée, grille de vérification d'identité |
| Fiche d'une demande | `/web/rgpd/demandes/:id` | horloge, demandeur, droits et décisions, vérifications d'identité, gestes du guichet, arbitrages de la direction, clôture, journal |
| Mes données | `/web/mes-donnees` | suivi de ses demandes, phrase d'avancement, aucune information interne |
| Déposer une demande | `/web/mes-donnees/demande` | qualité, élève concerné, droits, périmètre, rappel du masquage des tiers |

Le **formulaire de saisie est rendu par le serveur**, sans attendre `GET /rgpd/guichet/referentiel` (le
serveur n'a pas la session) : un vocabulaire de repli, strictement inclus dans ce que le serveur accepte,
donne la structure tout de suite, et la grille de vérification d'identité s'ajoute quand le référentiel
répond (`FRONTEND_LEPTOS.md` §3). Le serveur reste seul juge : une valeur qu'il ne connaît pas est refusée.

Conventions `FRONTEND_LEPTOS.md` respectées : logique d'écran **pure** isolée dans `modele.rs` (12 tests
natifs) ; données chargées par le navigateur (`LocalResource`) ; écritures par `noyau/ecriture.rs` ; aucune
page bloquée sur l'absence de droit côté client ; aucun `Uuid::new_v4` ; aucun style en ligne (CSP à nonce) ;
valeurs dérivées calculées **avant** `view!` (piège §10) ; feuille dédiée `rgpd-droits.css` sur les seuls
jetons de `ui.css`, lisible dès 375 px.

### 2.6 Minimisation (R-RG32) — vérifiée par des tests

* La **file** ne porte que des métadonnées : aucun nom d'élève, aucun texte de périmètre (test sur le JSON
  sérialisé, côté `ent-db` **et** côté HTTP).
* Le **HTML rendu par le serveur** ne porte aucune donnée d'établissement (test de rendu).
* Les **notifications** (accusé, prolongation, alerte d'échéance) ne portent aucun nom d'élève.
* Le **journal** ne porte aucun nom d'élève.
* Le **suivi du demandeur** n'expose ni journal, ni gestionnaire, ni note interne ; il est filtré sur le
  compte (un autre parent ne voit pas la demande).

---

## 3. Actions couvertes

| Numéro | Action | État |
|---|---|---|
| RGP-140 | Déposer une demande en ligne (connecté) | fait |
| RGP-141 | Qualifier la demande (RGPD, CADA, document), motivée et modifiable | fait |
| RGP-142 | Déposer une demande sans compte | fait |
| RGP-143 | Saisir une demande reçue par courrier, courriel ou au guichet | fait |
| RGP-144 | Vérifier l'identité et la qualité ; pièce détruite à la clôture | fait |
| RGP-145 | Accuser réception (date, échéance, contact) | fait |
| RGP-146 | Geler les sorts finaux pendant la demande | **point de lecture fourni** ; le gel s'exécute au lot G6 (moteur de conservation) |
| RGP-147 | Affecter la demande à un gestionnaire | fait |
| RGP-148 | Prolonger de deux mois, motif, information avant l'échéance | fait |
| RGP-149 à RGP-154 | Assembler l'extrait, relire les masquages, remettre | **G3b** (2545) |
| RGP-155 | Suivre l'avancement (côté demandeur) | fait |
| RGP-156 | Traiter une rectification | **décision enregistrée** ; le renvoi au socle propriétaire est en G3b |
| RGP-157 | Examiner un effacement par catégorie ; motif par article | fait (décision ; l'exécution est en G6) |
| RGP-159 | Retirer un consentement (renvoi au socle 18) | **non fait** : le socle 18 porte déjà le retrait ; le lien de traçabilité vient en G3b |
| RGP-160, RGP-161 | Poser et lever une limitation | **décision enregistrée** ; le statut sur l'objet contesté est en G6 |
| RGP-162 | Examiner une opposition (jamais de refus par défaut) | fait |
| RGP-163 | Extrait réutilisable (CSV) | **G3b** |
| RGP-164 | Refuser une demande manifestement excessive | fait |
| RGP-166 | Clore (réponse, éléments remis) | fait (le résumé ; les empreintes des éléments remis en G3b) |
| RGP-167 | Rouvrir sur réclamation | fait |
| RGP-169 | Tableau des demandes (échéances, retards, gestionnaire) | fait |
| RGP-302 | Demande à plusieurs sujets (fratrie) | **non fait** : `rgp_demande_sujet` est attribuée à la **migration 2556** (§26.6), hors de ce lot |
| RGP-303 | Distinguer « reçu le » et « saisi le », alerte au-delà de 5 jours | fait |
| RGP-304 | Modèles de réponse par cas fréquent | fait (8 modèles éditables, servis par le référentiel) |
| RGP-305 | Regénérer les documents émis pour l'extrait | **G3b** |
| RGP-306 | Retrait de photo demandé par un élève mineur | **partiel** : la demande est reçue et la décision des titulaires tracée ; la limitation provisoire sur l'objet relève du socle 18 |
| RGP-307 | Grille de vérification d'un ancien élève sans compte | fait |
| RGP-316 | Demande CADA d'un tiers (document agrégé ou occulté) | **partiel** : la qualification CADA et le modèle de réponse L311-6 sont là ; l'occultation du document est en G3b |

---

## 4. Cas limites traités (et comment)

| N° | Traitement dans ce lot |
|---|---|
| CL27-01 | la demande d'un titulaire est reçue sur l'élève ; le masquage des données propres de l'autre parent est l'affaire de l'extrait (G3b), mais le dépôt refuse déjà la qualité `tiers` sur une personne (R-RG38) |
| CL27-02 | une protection au dossier (adresse mise de côté, « ne pas solliciter ») rend la qualité **`restreinte`** : arbitrage de la direction avant toute suite |
| CL27-03 | un lien sans exercice de l'autorité est constaté `confirmee_sans_autorite`, jamais refusé en silence |
| CL27-04 | l'élève de 14 ans dépose ; l'information des titulaires est la règle, la non-information exige un motif sérieux de la direction, tracé |
| CL27-05 | un ancien élève qui revient est une personne du dossier : sa demande se dépose connectée, sans objet particulier ici |
| CL27-07 | la qualité `mandataire` n'est pas acceptée au dépôt en ligne : elle passe par le guichet, avec le mandat (grille d'identité, étape `mandat`) |
| CL27-11 | cinq demandes identiques : le refus pour demande excessive est une décision **motivée de la direction**, jamais automatique (R-RG17) |
| CL27-12 | les vacances ne suspendent rien : l'horloge est inchangée, les alertes J-10 et J-3 partent aux habilités |
| CL27-15 | `representant_majeur_protege` est une qualité du catalogue ; la pièce se vérifie au guichet |
| CL27-64 | un courrier reçu pendant la fermeture est saisi à sa **date de réception** ; l'écart est signalé comme un retard de relève |
| CL27-65 | une demande « au nom de l'enfant » depuis le compte du père est la demande **du père** : la qualité le dit, le sujet reste l'enfant |
| CL27-66 | un ancien élève inconnu reçoit la **même phrase** que n'importe qui : aucune confirmation de scolarisation |
| CL27-70 | la pièce photographiée au guichet est une référence de document, détruite à la clôture |

**Cas limites déduits** (non listés par la conception, exigence « masterclass ») :

* une demande **close** ne se modifie plus : il faut la **rouvrir** (409 sinon) — on ne réécrit pas une
  réponse déjà remise ;
* la **clôture est refusée** tant qu'un droit exercé reste `en_examen` : on ne clôt pas sans avoir répondu à
  chacun des droits (l'écran le dit avant d'envoyer) ;
* un **refus global** motive **tous** les droits restés en examen avec le même motif (`art12_5`) : aucune
  décision muette ;
* une **seconde prolongation** est refusée (le texte n'en prévoit qu'une) ;
* une **prolongation demandée après l'échéance** du premier mois est refusée (l'information due dans ce
  premier mois n'a plus d'objet) ;
* une **réception future** ou **antérieure de plus de deux ans** est une erreur de saisie, refusée ;
* un **doublon de droit** sur la même demande est refusé (409), pas ajouté deux fois ;
* un **droit non demandé** ne se décide pas (404) ;
* une **demande d'un autre établissement** est introuvable (404), jamais « refusée » : rien ne dit qu'elle
  existe ailleurs ;
* un **échec de vérification d'identité** ne clôt pas la demande : la direction décide du refus motivé ;
* une **qualité constatée absente** refuse le dépôt (403) plutôt que d'ouvrir un dossier sur une qualité
  fausse ;
* la décision « informer les titulaires » n'a de sens que sur une demande d'**élève mineur** (409 sinon) ;
* une qualité **« pour moi-même »** ne vaut que **sur soi** : trouvé en relecture de ce lot, un compte
  pouvait ouvrir un dossier sur la personne de son choix en invoquant `soi_meme` avec l'identifiant d'un
  autre — et aurait reçu son extrait au lot G3b. Désormais le sujet est **résolu depuis le compte** quand il
  n'est pas donné (l'usager n'a aucun identifiant interne à saisir) et **refusé** quand il désigne quelqu'un
  d'autre ; un compte sans fiche dans l'établissement passe par le guichet. `mandataire` et
  `representant_majeur_protege` sont renvoyés au guichet comme `tiers` : ces qualités exigent une pièce.

---

## 5. Règle métier non tranchée — valeur par défaut prudente (D-069)

**La vérification d'identité suspend-elle le délai légal ?**

* La conception dit **non** (§9.2 : « le délai court dès la réception, la vérification ne le suspend pas »),
  en marquant la position `[H, à confirmer DPO]`.
* La consigne du lot demandait une **suspension** pendant la demande de précision d'identité.

**Choix appliqué : le délai court** (`IDENTITE_SUSPEND_DELAI = false`). C'est la lecture **la plus
protectrice pour la personne** et la plus prudente pour l'établissement : si l'horloge ne s'arrête pas,
l'établissement répond dans le mois, et aucun dépassement n'est imputable à une attente interne. L'inverse
exposerait à un retard réel face à la CNIL.

Ce qui est fait pour que le DPO puisse trancher avec les faits en main : l'attente est **enregistrée**
(`identite_demandee_le`, `identite_fournie_le`), **comptée** (`identite_attente_jours`), **affichée** sur la
fiche (« X jour(s) — le délai légal continue de courir ») et **exposée par l'API**
(`identite_suspend_delai` du référentiel, faux). Le jour où le DPO décide l'inverse, la bascule tient en un
réglage d'établissement : `echeance_effective()` ajoute déjà les jours comptés derrière cette constante.

À porter dans `27_RGPD_ARCHIVES.md` §23 comme **réglage signalé S-555** quand le pilote versera le lot
(aucun fichier de conception modifié par ce lot de code).

---

## 6. Non fait (et pourquoi)

| Non fait | Raison |
|---|---|
| Extrait assemblé, sections par socle, masquages de tiers relus, remise chiffrée, extrait CSV | lot **G3b**, migration **2545** (découpage imposé par la consigne) |
| `rgp_demande_sujet` (demande à plusieurs sujets, RGP-302) | migration **2556** attribuée par la conception (§26.6, §26.8) : hors plage de ce lot |
| Exécution d'un effacement accordé, limitation posée sur l'objet, gel réel | lot **G6** (moteur de conservation) ; ce lot fournit la **décision** et le point de lecture du gel |
| Journal des accès visible (« qui a consulté ») | lot **G4** ; le droit `consultations` est reçu et décidé, sa réponse viendra de `acc_trace` |
| Page publique de dépôt sans compte (écran) | la **route** publique existe et est testée ; l'écran public (hors coquille, sans session) est à écrire avec la page « Vos données » publique du lot G4, pour ne pas dupliquer sa mise en page |
| Retrait de consentement relié au 27 (RGP-159) | le socle 18 porte déjà le retrait ; le lien de traçabilité est en G3b |
| Événements d'`outbox` (`droits.demande_recue`, `droits.qualifiee`… §16) | **aucun consommateur** aujourd'hui, et les lots G0 à G2 déjà versés n'en émettent pas non plus : le journal en ajout seul, écrit dans la transaction du fait, porte la même information. À brancher avec le premier consommateur (moteur de conservation G6, tableau de bord G2) pour ne pas poser une file que personne ne vide |
| Captures d'écran | aucune chaîne `cargo-leptos` / navigateur dans ce conteneur (pas de `_outils-scolia`) : le rendu est vérifié par les tests de `ent-web-server` (structure, étiquettes, CSP, absence de donnée) |

---

## 7. Tests

### 7.1 `ent-db` — 20 tests de labo (`tests_rgpd_g3a.rs`)

| Test | Ce qu'il prouve |
|---|---|
| `g3a_numero_par_etablissement_sans_trou_visible` | numéros consécutifs ; une saisie refusée n'avance pas le compteur ; la référence porte l'année de réception |
| `g3a_horloge_un_mois_puis_deux_au_plus` | échéance à un mois ; prolongation bornée, motivée, unique ; aucun avis à une adresse fournie par la demande |
| `g3a_saisie_tardive_signalee_et_dates_figees` | écart réception / saisie signalé ; numéro, date de réception, canal figés ; suppression interdite |
| `g3a_reception_trop_ancienne_refusee` | garde-fou de saisie |
| `g3a_depot_connecte_accuse_sans_nom_d_eleve` | identité par session, qualité constatée, accusé **sans nom d'élève**, suivi filtré sur le compte |
| `g3a_depot_connecte_qualites_refusees` | `tiers` et `parent_sans_autorite` renvoyés au guichet ; lien absent → 403 ; sujet d'ailleurs → 404 ; liste de droits vide refusée |
| `g3a_eleve_mineur_depose_et_information_des_titulaires` | CL27-04 : information par défaut, non-information motivée réservée à la direction, décision révisable |
| `g3a_qualite_pour_soi_ne_vaut_que_sur_soi` | le sujet est résolu depuis le compte ; « pour moi-même » sur la fiche d'un autre → 403 ; un élève ne demande pas les données d'un autre élève ; les qualités à pièce restent au guichet |
| `g3a_depot_sans_compte_ne_confirme_rien` | **même réponse** pour une personne connue et inconnue ; aucun avis, aucun envoi ; qualité réservée refusée |
| `g3a_identite_tracee_et_piece_detruite_a_la_cloture` | R-RG16 de bout en bout ; clôture bloquée tant qu'un droit est en examen ; demande close non modifiable |
| `g3a_precision_identite_comptee_sans_repousser_lecheance` | §9.2 : l'attente est comptée, l'échéance ne bouge pas |
| `g3a_echec_identite_puis_refus_motive_de_la_direction` | l'échec ne clôt rien ; refus réservé et motivé ; aucun droit muet |
| `g3a_qualification_motivee_et_decisions_par_droit` | qualification motivée et requalifiable, les deux au journal ; accorder ≠ refuser (droits différents) ; refus motivé par article **et** texte |
| `g3a_cloture_leve_le_gel_et_reouverture_garde_l_horloge` | R-RG19 ; réouverture sans remise à zéro |
| `g3a_droits_et_minimisation_du_tableau` | cinq profils refusés ; secrétariat sans arbitrage ; **aucun nom d'élève ni périmètre dans le tableau** |
| `g3a_role_maison_porteur_du_droit` | un rôle paramétré par l'établissement ouvre le guichet (D-017), sans ouvrir l'arbitrage |
| `g3a_cloisonnement_entre_etablissements` | invisibilité des demandes d'un autre établissement ; 404 neutre |
| `g3a_alerte_avant_echeance_idempotente` | J-10, destinataires réellement habilités, une alerte par palier, aucune famille, aucun nom d'élève |
| `g3a_journal_en_ajout_seul` | ni `UPDATE`, ni `DELETE`, ni `TRUNCATE` ; compteur qui ne recule pas ; journal sans donnée d'élève |
| `g3a_registre_des_tables_et_categorie` | classement E12 et catégorie `rgpd.demande` ; la gouvernance garde la sienne |

Plus 10 tests unitaires natifs dans `droits_demandes.rs` (échéances et fins de mois, bornes de prolongation,
référence lisible, validation des droits, bornes de texte, états clos, attente comptée, phrase d'avancement,
cohérence de la grille et des modèles).

### 7.2 `ent-api` — 3 tests HTTP (`tests_rgpd_g3a_http.rs`)

`g3a_http_guichet_parcours_et_refus` (401 sans session, 403 par profil, parcours complet 201 → 200 → 409,
minimisation de la file), `g3a_http_depot_et_suivi_du_demandeur` (dépôt, suivi sans détail interne, 403 sur
la fiche du guichet, filtrage par compte), `g3a_http_depot_sans_compte_reponse_invariable` (même phrase pour
une personne connue et inconnue, établissement imposé par l'adresse, 404 neutre sans adresse connue).

### 7.3 Frontend

* 12 tests natifs du **modèle d'écran** (`modules/rgpd_droits/modele.rs`) : libellés sans trou, six filtres,
  recherche sur la référence **seulement**, aller-retour des codes de filtre, teintes et phrases d'échéance,
  obstacle de clôture nommant les droits restants, décision refusée exigeant article et texte, bornes de
  motif, complétude d'un dépôt, tri du journal, tri du suivi.
* 1 test de **droits d'affichage** (`noyau/droits_moteur.rs`) : le guichet exige la portée établissement, le
  secrétariat n'arbitre pas, un moteur vide n'ouvre rien.
* 4 tests de **rendu serveur** (`ent-web-server/tests/rgpd_droits.rs`) : les cinq pages dans la coquille
  (CSP à nonce, `lang="fr"`, aucun style en ligne, lien d'évitement), étiquettes de la saisie et explication
  de la date de réception, annonce du masquage des tiers au dépôt, **aucune donnée d'établissement dans le
  HTML du serveur**.

### 7.4 Règle 33 — aucun test ne dépend de l'heure

Chaque test fixe lui-même sa référence : la date de **réception** d'une demande, la date d'un lien, ou un
écart calculé à partir de `aujourdhui_etablissement` (qui est précisément l'horloge que le guichet utilise).
Le test d'alerte **cherche** la date de réception qui place l'échéance à J-10 au lieu de supposer un jour.
Aucune assertion sur une date absolue, aucun `jour_utc`, aucune heure réelle.

---

## 8. Compteurs et gardes (règle 28)

| Compteur | Avant | Après |
|---|---|---|
| `Action::TOUTES` | `[Action; 282]` | `[Action; 286]` |
| `MIGRATION_TAG` | 3808 | **3808** (inchangé : 2544 < 3808) |
| Tables du registre `registre_donnees.rs` | — | **+5** |
| Migrations du tableau de `migrate.rs` | — | **+1**, en fin de tableau (règle 38) |
| `migrations_versees.txt` | — | **non touché** (le pilote l'ajoute au versement) |

Gardes vérifiées en local sur base neuve : `l_ordre_des_migrations_versees_est_fige_regle_38`,
`migration_tag_suit_la_derniere_migration`, `aucune_colonne_json_hors_liste_d036`,
`garde_search_path` (base neuve **et** après rejeu), `chaque_table_de_la_base_est_classee_et_conforme`,
`chaque_table_de_donnees_personnelles_a_une_categorie`,
`chaque_table_de_donnees_personnelles_est_couverte` (R-RG14),
`neutralisation_s_execute_sur_toute_la_base_sans_effet_persistant`, `socle_27_droits_par_defaut`,
`codes_uniques_et_relus`.

**Règle 38** : les quatre fonctions créées par 2544 (`rgp_demande_numero_suivant`,
`rgp_demande_compteur_garde`, `rgp_demande_journal_ajout_seul`, `rgp_demande_garde`) portent
`SET search_path = pg_catalog, public, pg_temp` et qualifient leurs relations `public.`.

**D-036** : aucune colonne JSON, aucun tableau, aucun `BYTEA` (`grep -i "json\|bytea\|\[\]"` sur 2544 : rien).

---

## 9. Vérifications locales

| Commande | Résultat |
|---|---|
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | voir §11 (tête et CI) |
| `cargo clippy --locked --workspace --all-targets --features ent-api/labo,ent-lab/labo -- -D warnings` | voir §11 |
| `cargo clippy --locked -p ent-types --all-targets -- -D warnings` | vert |
| `cargo clippy` frontend (SSR, serveur, serveur+labo, wasm32) | vert |
| `cargo metadata --locked` dans `ent-rust/web` (règle 34) | **vert** (aucune entrée manquante) |
| `cargo fmt` ciblé (jamais `--all`) | appliqué aux paquets touchés |
| Base locale **neuve** dont le nom contient « test » | `scolia_g3a_test`, recréée avant la série |
| Un seul `CARGO_TARGET_DIR` | oui |

---

## 10. Interdits respectés

Next.js, `main`, `dev` : non touchés. Aucun push sur `ENT`. Aucune base ni secret de production, aucune
donnée réelle (cast de labo fictif). Pas de `cargo fmt --all`, pas de `#[ignore]`. `POINT_QUOTIDIEN.md` et
`PILOTAGE.md` non modifiés. `ui.rs` non étendu. Aucune nouvelle dépendance (D-062). Aucune heure réelle ni
jour UTC dans les tests (règle 33).

---

## 11. Fusion d'ENT, tête et CI

`origin/ENT` a été fusionnée avant la CI finale (règle 30, pour que le versement du pilote porte sur un
arbre identique à celui que la CI vérifie). Un seul conflit, dans le tableau de `migrate.rs` : les deux
côtés ajoutaient en fin de tableau. Résolu dans l'ordre imposé par la règle 38 — **2420 d'abord** (versée
sur ENT, présente dans `migrations_versees.txt`), **2544 ensuite**.

Compteurs recalculés depuis la base commune (`git merge-base` = `4fc2c701`), conformément à la règle 28 :

| Compteur | Base commune | ENT | Ce lot | Après fusion |
|---|---|---|---|---|
| `Action::TOUTES` | 282 | 282 (inchangé) | +4 | **286** |
| `MIGRATION_TAG` | 3808 | 3808 | 3808 | **3808** |
| Tables du registre | — | +0 (2420 n'ajoute aucune table) | +5 | **+5** |
| Migrations du tableau | — | +1 (2420) | +1 (2544) | **+2**, dans cet ordre |

Tête de code livrée : `5eb8756c` (fusion d'`origin/ENT` : conflits `migrate.rs` — 2544 après 2227 —, inventaire des routes 77 sources, compteur RLS +1 +5).
CI : run 45604088988 sur `5eb8756c` = **success**. Historique : le run 45572874024 était rouge sur trois gardes (compteur RLS, deux inventaires de routes), corrigées avant la fusion ; les tests de rendu `ent-web-server` n'ont tourné qu'en CI (échec de compilation locale par manque de mémoire/disque).
PR #158 laissée en brouillon vers `ENT`.
