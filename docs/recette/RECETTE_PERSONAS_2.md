# Recette par personas n° 2 — jalon P1 (socles 2 à 4), sur le logiciel réel

**Date :** 2026-11-08 — **Branche :** `ent-recette-personas-2` (base `ent-integration-lot8b` 4b5428f + documentation `ent-mc0hoe`) — **Statut :** recette faite, un correctif, **à relire par Florian**.
**Méthode (D-021, étape 4) :** celle de la recette 1 (`docs/recette/RECETTE_PERSONAS_1.md`, branche `ent-recette-personas-1`). Serveur `ent-api` compilé avec la feature `labo`, lancé en local (`ENT_ENV=lab`, `ENT_ALLOW_LABO=1`, `ENT_TACHES_ELEVES=0`) sur une base PostgreSQL 16 locale jetable (`ent_recette2_lab`) ; deux établissements de labo (A et B) créés par `POST /labo/bootstrap` ; toutes les actions jouées **par l'API HTTP réelle** (client Python `urllib`, jetons Bearer et jeton de préinscription `x-preinscription`). Environ 1 500 requêtes. Aucune base, aucun secret, aucun déploiement de production ; Next.js non touché.
**Aides de labo** (base locale seulement, comme la recette 1) : pas de messagerie en labo, donc (1) le code de vérification de préinscription est retrouvé à partir de son empreinte (clé de développement publique `SecretBox::dev()`) ; (2) le jeton d'une invitation est remplacé en base par un jeton connu ; (3) le mot de passe de labo est recopié sur les comptes créés pendant la recette (Inès et Marc Martin) ; (4) deux élèves sont mis dans un groupe par SQL (aucune route : RP1-02, toujours ouvert).
**Écrans Leptos** : non joués dans un navigateur (les écrans `/web/*` appellent les mêmes routes).

Légende des statuts (celle de la recette 1) : **corrigé + test** ; **test ajouté en échec** (`#[ignore = "FAILLE OUVERTE …"]` ou `"MANQUE …"`, il échoue tant que le problème existe : `cargo test … -- --ignored`) ; **signalé** (décrit ici, sans test ni correctif).

---

## Verdict P1

**P1 « Le cœur tourne » n'est pas atteint en l'état ; il en est proche pour l'élève et le dossier administratif, pas pour l'inscription de bout en bout.**

**Prêt à montrer à Florian** (joué sur le serveur réel, tient sous attaque) :
- **Identité versionnée** : correction, changement d'état civil avec pièce (adoption), prénom d'usage ; l'adoption est forcée « sensible » ; l'historique est refusé aux familles, aux professeurs, à la CPE, à la comptabilité et à l'infirmerie ; les versions sensibles ne sont lues que par la direction.
- **INE absent puis reçu** : déclaré, collision avec un autre élève détectée et mise en « contesté » avec une liste à traiter, confirmé, révoqué ; un second INE est refusé tant qu'un INE confirmé existe ; INE invisible des professeurs, de la vie scolaire et de la comptabilité.
- **Mère protégée et père à ne pas solliciter, déclarés dès la préinscription** : l'adresse à protéger n'est **écrite en clair nulle part** (vidage complet de la base de labo : 0 occurrence), elle n'est pas renvoyée à la famille, seule la direction peut l'ouvrir (secrétariat, CPE, professeur : 403) ; le père ne reçoit **aucune invitation** ni aucune demande de confirmation de lien (y compris pour le second enfant) ; les deux alertes apparaissent à la direction.
- **Parent séparé** : via les routes du lot A1, le père ne voit de la mère que sa désignation (ni adresse, ni téléphone, ni e-mail) — **après correction de RP2-01** (route historique qui contournait cette règle).
- **Conflit d'intérêts** (secrétaire ex-compagne du père) : adresse fermée, écritures sur le dossier refusées (« doit être faite par un collègue »), accès tracés au tableau de la direction.
- **Professeur d'un seul groupe** (D-022) : sur les routes des socles 2 à 4, il ne voit que les élèves de son groupe (fiche, responsables, chronologie, recherche, trombinoscope) ; les failles RP1-03 et RP1-04 de la recette 1 sont **fermées** par l'intégration du lot 8 bis.
- **Cloisonnement** : la direction de B est refusée partout (lecture et écriture) avec les identifiants de A ; balayage de 48 gabarits de routes des socles 2 à 4 par 8 attaquants : aucune fuite hors RP2-01.
- **Fratrie inscrite en deux fois** : à la seconde préinscription, la mère et le père sont proposés comme personnes existantes (e-mail masqué) et réutilisés ; **ancien élève qui revient** : même fiche, seconde scolarisation au motif « retour » ; **départ en cours d'année** : l'élève sort de la classe et du trombinoscope.

**Ce qui manque pour déclarer P1** (par ordre d'importance) :
1. **RP2-10 — l'inscription validée n'existe pas pour la scolarité** : la validation écrit une inscription « projetée » dans l'ancienne table, que la scolarité du socle 2 (E4) ne lit pas ; à la rentrée, il faut trois saisies séparées (affectation en classe, scolarisation, inscription annuelle) et l'élève reste « préinscrit ». Le parcours « inscrire un élève jusqu'à la rentrée » de la feuille de route n'est donc pas d'un seul tenant.
2. **RP2-11 — le secrétariat ne peut pas valider une inscription** (ni voir les correspondances), contrairement à la conception (INS-05 : « ADF ou DIR ») : la direction doit tout valider elle-même.
3. **RP2-04 — « ne pas solliciter » ne protège pas contre le rattachement** : une fusion de fiches ou une confirmation au guichet ouvre au père l'accès aux enfants, donc lui révèle l'établissement, sans aucun avertissement.
4. **Les correctifs de la recette 1 ne sont pas dans la base de P1** (`ent-recette-personas-1` et `ent-recette-1-correctifs` non fusionnées) : un élève mineur déclare encore sa propre absence (RP2-13 = RP1-09), un professeur sans classe liste encore les documents d'un élève (RP1-05). À fusionner avant de montrer P1.
5. **Aucun écran joué** dans un navigateur, et **aucune route de proposition** (signaler une erreur d'identité, ELV-18 et ELV-40) : la famille ne peut rien corriger elle-même.

---

## 0. Résumé

### 0.1 Chiffres

| Type | Critique | Élevée | Moyenne | Faible | Total |
|---|---|---|---|---|---|
| Faille | 0 | 2 | 4 | 3 | 9 |
| Fonction manquante | 0 | 2 | 5 | 1 | 8 |
| Défaut | 0 | 0 | 2 | 3 | 5 |
| Lenteur | 0 | 0 | 0 | 0 | 0 |
| **Total** | **0** | **4** | **11** | **7** | **22** |

Statuts : **1 corrigé + test** (RP2-01), **8 tests ajoutés en échec** (RP2-02, 03, 04, 05, 10, 11, 12, 13), **13 signalés** ; **6 tests verts** figent ce qui tient (§5). Détail en §3 et §3.4.

Aucune lenteur : sur ≈ 1 500 requêtes, seules la connexion (Argon2, 550–670 ms en compilation de débogage) et l'amorçage du labo (≈ 36 s) dépassent 500 ms ; aucune réponse 5xx.

### 0.2 Les 10 constats les plus importants

1. **RP2-01 (élevée, corrigé)** — La route historique `GET /eleves/{id}/dossier` renvoyait au **père séparé l'e-mail, le foyer et la qualité de payeur de la mère protégée**, et au professeur et à la CPE la qualité de payeur des responsables : elle ne passait pas par la minimisation du lot A1.
2. **RP2-10 (élevée, test en échec)** — Une inscription validée reste invisible de la scolarité ; la rentrée demande trois saisies et l'élève reste « préinscrit ».
3. **RP2-04 (élevée, test en échec)** — Fusionner la fiche du père « à ne pas solliciter » avec une fiche qui a un compte, ou confirmer son lien au guichet, lui ouvre aussitôt les enfants, sans avertissement.
4. **RP2-11 (élevée, test en échec)** — Le secrétariat ne valide pas les inscriptions (conception : ADF ou DIR).
5. **RP2-13 (moyenne, test en échec)** — Régression d'intégration : les correctifs de la recette 1 manquent (élève mineur qui déclare son absence ; documents listés par tout professeur).
6. **RP2-02 (moyenne, test en échec)** — Après une adoption, le secrétariat continue de lire le **nom d'avant l'adoption** dans l'historique (et pas la version courante).
7. **RP2-03 (moyenne, test en échec)** — L'ancien chemin `POST /foyers/{id}/liens` crée une **seconde personne** pour un responsable déjà connu (même e-mail), sans reporter sa marque « ne pas solliciter ».
8. **RP2-05 (moyenne, test en échec)** — La comptabilité peut enregistrer une **liste de payeurs vide** : l'élève n'a plus de payeur.
9. **RP2-12 (moyenne, test en échec)** — Un départ en cours d'année laisse l'inscription annuelle « en cours ».
10. **RP2-17 (moyenne, signalé)** — Aucune route de **proposition** : ni la famille ni le terrain ne peuvent signaler une erreur d'identité.

### 0.3 Ce qui tient (vérifié sur le serveur réel)
- **Adresse à protéger** : jamais en clair en base (vidage complet, recherche du texte et de la référence de l'ordonnance : 0) ; absente des réponses à la famille et au personnel ; ouverture réservée à la direction (`POST /dossier/adresses-protegees/{id}/ouvrir` : secrétariat, CPE, professeur → 403 ; direction → 200).
- **Ne pas solliciter** : aucune invitation (`ent_inscription_invitation` : une ligne, pour la mère seulement), aucune demande de confirmation de lien visible du père, au premier comme au second enfant ; alerte « à confirmer » à la direction.
- **Historique d'identité** : refusé aux familles (« réservé au personnel »), aux professeurs, à la CPE, à la comptabilité, à l'infirmerie (« rôle insuffisant ») ; versions sensibles (adoption, prénom d'usage) lues par la direction seule.
- **Identifiants (INE)** : cycle complet déclaré → contesté → confirmé → révoqué ; collision détectée ; second INE refusé (409) ; lecture et écriture refusées hors secrétariat et direction ; autre établissement : 404.
- **Conflit d'intérêts déclaré** : adresse fermée sur la vue A1 et sur la fiche personne, écritures refusées (409 « doit être faite par un collègue »), tableau `GET /dossier/conflits-interet`.
- **Écritures des socles 2 à 4** (17 routes : corrections, usages, identifiants, clôture, décès, mobilité, ajout de responsable, suspension, interdictions, « ne pas solliciter », personne, payeurs, fusion, décision et statut d'inscription, conflit) jouées par 9 attaquants (autre famille, élève, père séparé, professeur de la classe, professeur sans classe, CPE, comptabilité, infirmerie, direction de B) : **toutes refusées**, sauf la déclaration d'un conflit d'intérêts (RP2-14) et les payeurs par la comptabilité (RP2-05).
- **Lecture** : 48 gabarits de routes des socles 2 à 4, cibles Harry, Luna, Rose (et leurs personnes, foyers, dossier d'inscription), 8 attaquants : aucune donnée protégée lue hors RP2-01.
- **Dossier d'inscription** : refusé au professeur, à la CPE, à la comptabilité, à l'infirmerie, aux parents.
- **Professeur de groupe** (D-022) : élèves hors groupe et autre classe refusés sur `/eleves/{id}`, fiche, responsables, chronologie, scolarité, identifiants, historique, interdictions, contacts, photo, journal ; recherche et trombinoscope bornés au groupe.

---

## 1. Personas jouées

Établissement A (collège de labo, 10 élèves en 6A, 5A, 4B, 3C) ; établissement B identique (cible des attaques inter-établissements).

| Persona | Compte | Comment il a été obtenu |
|---|---|---|
| Direction (Albus Dumbledore), Admin | `direction`, `admin` | labo |
| Secrétariat (Argus Rusard) | `administratif` | labo |
| CPE, comptabilité, infirmerie, psychologue | profils de labo | labo |
| Professeur de la classe (Severus Rogue, 4B) ; professeur sans classe (« Prof 5 ») | `prof` | labo |
| **Professeur d'un seul groupe** (« Prof 3 », « 4B LV2 allemand » : Harry et Hermione, pas Ginny) | `prof` | groupe et service par la direction ; affectation par SQL de labo (RP1-02) |
| **Mère protégée** (Inès Martin) : préinscrit Rose (4ᵉ) puis Jade (6ᵉ), adresse à protéger | parent créé par la recette | préinscription en ligne (code e-mail retrouvé en labo) ; invitation acceptée ; mot de passe de labo posé en base |
| **Père à ne pas solliciter** (Marc Martin), autorité parentale, père de Rose, Jade et Léo (demi-frère en 6A) | parent créé par la recette | déclaré par la mère ; compte créé par l'ancien chemin `/foyers/{id}/liens` pour Léo ; fiches fusionnées par la direction |
| **Père en garde alternée** (James Potter, sans compte) | — | `POST /dossier/eleves/{Harry}/responsables/ajout` (`garde: alternee`) |
| **Tuteur ASE** (« Aide sociale à l'enfance du Rhône ») pour Luna | — | `responsables/ajout`, lien `tuteur` |
| **Secrétaire en conflit d'intérêts** (ex-compagne de James) | `administratif` | déclaration par la direction (`ex_conjoint`) |
| Parents de labo (Lily Potter, Narcissa Malefoy, Molly Weasley) ; élève Harry (14 ans) | labo | labo |
| Élève adoptée (Cho Chang → Diggory), élève au prénom d'usage (Hermione → « Mia »), élève devenu majeur (Neville), élève qui part (Seamus), ancien élève qui revient (Cedric) | — | données de labo modifiées par le secrétariat |
| Direction de l'établissement B | `direction` | labo (B) |

---

## 2. Parcours joués

**Inscription de bout en bout** : campagne d'admission 2027-2028 (niveaux, capacités, pièces) créée et ouverte par le secrétariat ; préinscription en ligne par une mère inconnue (code e-mail, formulaire, second parent, réserves « adresse à protéger » et « ne pas solliciter », dépôt de deux pièces) ; instruction (prise en charge, pièces acceptées) ; décision « admis » (direction) ; correspondances ; validation (direction) ; invitation de la mère, acceptation, compte ; préparation de l'année 2027-2028 (`preparer-suivante`), affectation en 4B 2027-2028 (`affectations-lot`), scolarisation et inscription annuelle saisies à la main (RP2-10).
**Identité** : correction (date de naissance d'après pièce, qui rend Neville majeur), changement d'état civil par adoption avec pièce, prénom d'usage ; historique lu par chaque profil.
**INE** : absent à la validation, déclaré, collision, confirmé, révoqué.
**Parents séparés en garde alternée** (Harry, James) ; **tuteur ASE** (Luna) ; **ne pas solliciter** et **adresse à protéger** (Rose, Jade) ; ouverture de l'adresse par la direction ; **fratrie inscrite en deux fois** (Jade, avec réutilisation des personnes) ; confirmation au guichet ; **ancien élève qui revient** (Cedric : clôture puis réintégration) ; **élève qui devient majeur** (Neville) ; **départ en cours d'année** (Seamus).
**Attaque** : le père séparé (avant et après rattachement de son compte) sur 23 routes ; balayage de 48 gabarits de routes des socles 2 à 4 par 8 attaquants (élève, autre famille, parent sans droits, professeur sans classe, comptabilité, direction de B, père séparé, mère) ; 17 écritures par 9 attaquants ; parent sur l'historique d'identité ; secrétaire en conflit d'intérêts ; professeur de groupe.

---

## 3. Constats

Format : **RP2-nn** — type, gravité — constat. *Preuve* (requête → réponse). **Statut.**

### 3.1 Failles

**RP2-01 — faille, élevée — La route historique du dossier contourne la minimisation A1 (parents séparés, payeurs).**
*Preuve* : Marc (père, autre foyer, rattaché à Rose) → `GET /dossier/eleves/{Rose}/responsables` → Inès : `"email":null,"telephone":null,"adresse":null` (correct) ; mais `GET /eleves/{Rose}/dossier` → `liens: [{"display_name":"Inès Martin","email":"ines.martin@exemple.lab","foyer_id":"ba1f…","est_payeur":true,"part_paiement":100,"user_id":"247f…",…}]`. Professeur de la classe et CPE → `GET /eleves/{Harry}/dossier` → `"est_payeur":true,"part_paiement":100` de Lily (A1 leur ferme les payeurs). Cause : `get_dossier_for_actor` (`ent-db/src/service.rs`) remplit `liens` sans passer par `dossier_droits`.
**Corrigé + test** : `ent-db/src/dossier_droits.rs` (`minimiser_liens_dossier`) appelé par `get_dossier_for_actor` (`service.rs`) avec la lecture `lecture_dossier_admin` : chaque lien est rapproché du responsable déjà minimisé par A1 (aucune seconde règle) ; lien absent de la vue A1 retiré ; e-mail, téléphone et compte nuls si F1 fermé ; foyer nul si F2 fermé (rattachement du foyer à F2, comme `appliquer_vue`) ; droits du lien à faux si F3 fermé ; payeurs neutralisés s'ils sont fermés. `LienDossier.user_id` devient `Option<Uuid>` (clé toujours présente, `null` si masquée). Test `rp2_01_dossier_historique_respecte_la_minimisation_a1` (échoue sans le correctif, passe avec). Effet de bord : dans l'ancienne interface (`ui.rs`), l'encart foyer ne s'affiche plus pour un profil sans F2 (professeur, CPE) — voulu par A1.

**RP2-04 — faille, élevée — « Ne pas solliciter » ne protège pas contre le rattachement du compte.**
*Preuve* : la mère déclare le père « à ne pas solliciter » ; le secrétariat inscrit un demi-frère par l'ancien chemin, ce qui crée un compte au père (RP2-03) ; la direction fusionne les deux fiches du père (`POST /dossier/fusions` → 201, sans avertissement) : `GET /famille/enfants` (Marc) → `["Rose","Léo"]` ; plus tard, le secrétariat confirme au guichet le lien du père vers Jade (`POST /dossier/liens-en-attente/{id}/confirmer-guichet {"piece":"carte_identite"}` → 200) : `["Rose","Léo","Jade"]`. L'établissement de l'enfant est ainsi révélé au parent visé, alors que la marque est encore « à confirmer » par la direction.
**Test ajouté en échec** : `recette_personas_2_ouvertes::rp2_04_…` (fusion). La confirmation au guichet est **signalée** (même cause ; un double contrôle au guichet était déjà noté au lot A0 bis). Proposition : toute opération qui donne à une personne « à ne pas solliciter » un accès (fusion, rattachement de compte, confirmation au guichet, ajout de lien) exige une confirmation explicite de la direction, qui voit la marque.

**RP2-02 — faille, moyenne — Après une adoption, le secrétariat lit le nom d'avant.**
*Preuve* : `POST /eleves/{Cho}/identite/changements {"champs":{"nom_naissance":"Diggory"},"motif_code":"adoption","piece_document_id":…}` → 201, `sensible:true` (correct). Secrétariat → `GET /eleves/{Cho}/identite/historique` → une seule version : `{"nom_naissance":"Chang","nature":"creation","sensible":false}` ; la direction voit les deux. Le secrétariat voit donc à la fois le nom d'avant l'adoption (historique) et le nom actuel (fiche) : l'information intime est reconstituée. La version de création reste en clair en base (le scellement des versions sensibles relève du lot E7).
**Test ajouté en échec** : `rp2_02_adoption_ancien_nom_invisible_hors_g6`. À trancher (Q-RP2-2) : pour les motifs sensibles, c'est la **version remplacée** qu'il faut rendre sensible (et sceller), pas seulement la nouvelle.

**RP2-03 — faille, moyenne — L'ancien chemin `/foyers/{id}/liens` crée un doublon de personne et ignore la réserve.**
*Preuve* : Marc existe (fiche sans compte, « ne pas solliciter ») ; `POST /foyers/{f}/liens {"eleve_id":<Léo>,"email":"marc.martin@exemple.lab","display_name":"Marc Martin",…}` → 201, `activation_envoyee:false` ; `ent_personne` : deux fiches de même e-mail (`Martin / Marc` et `Marc Martin / —`, nom entier dans `nom`), la seconde avec compte et sans réserve. C'est le chemin par lequel RP2-04 arrive.
**Test ajouté en échec** : `rp2_03_ancien_chemin_foyers_ne_cree_pas_de_doublon_de_personne`. Proposition : faire passer ce chemin par la commande unique `ajouter_responsable` (A0), ou le retirer.

**RP2-05 — faille, moyenne — Liste de payeurs vide acceptée.**
*Preuve* : comptabilité → `PUT /dossier/eleves/{Harry}/payeurs {"payeurs":[]}` → 200 ; ensuite `ent_eleve_lien` de Harry : aucun payeur. (L'ajout d'un payeur à 50 % est, lui, refusé : RP1-14.)
**Test ajouté en échec** : `rp2_05_liste_de_payeurs_vide_refusee`. À confirmer (Q-RP2-4) : un élève sans payeur est-il possible (élève majeur qui paie lui-même, prise en charge) ? Si oui, il faut le dire explicitement plutôt que par une liste vide.

**RP2-13 — faille, moyenne — Correctifs de la recette 1 absents de la base P1 (régression d'intégration).**
*Preuve* : Harry (14 ans) → `POST /eleves/{Harry}/absences {"debut":"2026-11-16",…}` → **201** (RP1-09, corrigé sur `ent-recette-personas-1`) ; Prof 5 (sans classe) → `GET /eleves/{Harry}/documents` → 200, liste complète (RP1-05). Les branches `ent-recette-personas-1` et `ent-recette-1-correctifs` ne sont pas fusionnées dans `ent-integration-lot8b` (`PROJECT_STATUS.md`). En revanche les exports par un professeur sont bien refusés (`GET /exports/lsl` → 403, D-022 intégrée).
**Test ajouté en échec** : `rp2_13_eleve_mineur_ne_declare_pas_son_absence_regression` (il passera dès la fusion des correctifs de la recette 1).

**RP2-14 — faille, faible — Tout agent peut déclarer un conflit d'intérêts sur n'importe quel élève.**
*Preuve* : Prof 1, Prof 5, CPE, comptabilité, infirmerie → `POST /dossier/eleves/{Harry}/conflit-interet {"nature":"autre"}` → 201 chacun ; cinq déclarations au tableau de la direction. Connu (`04_ADMINISTRATIF.md` §22.5 : oracle d'existence, bruit). **Signalé.**

**RP2-15 — faille, faible — Le corps est validé avant l'autorisation sur les nouvelles routes (RP1-17 persiste).**
*Preuve* : professeur → `POST /dossier/liens-en-attente/{id}/confirmer-guichet {"piece":"cni"}` → 400 « type de pièce inconnu » (403 avec une pièce valide) ; attaquants → `POST /eleves/{id}/mobilites` → 422 ; secrétariat → `POST /inscriptions/dossiers/{id}/valider` mal formé → 422 « missing field `choix` ». **Signalé.**

**RP2-16 — faille, faible — Réponses 404 et 403 mêlées pour un parent sans lien.**
*Preuve* : Marc avant rattachement → `GET /eleves/{Rose}` et `/dossier/eleves/{Rose}/fiche` → 404, mais `/dossier/eleves/{Rose}/responsables`, `/chronologie`, `/eleves/{Rose}/journal` → 403 « accès refusé pour ce rôle ». Oracle d'existence faible (cf. RP1-08). **Signalé.**

### 3.2 Fonctions manquantes

**RP2-10 — manque, élevée — L'inscription validée n'ouvre rien dans la scolarité ; la rentrée se fait en trois saisies.**
*Preuve* : `POST /inscriptions/dossiers/{Rose}/valider` (direction) → `{"inscription_annuelle_id":"bdef…","affecte_en_classe":false}` ; cette ligne est dans `ent_eleve_scolarite_annee` (`statut: projetee`), mais `GET /eleves/{Rose}/scolarite` → `{"statut":"a_venir","scolarisations":[],"inscriptions":[]}` (`ent_eleve_inscription` : 0 ligne). À la rentrée : `POST /referentiel/annees/{2026-2027}/preparer-suivante` crée 2027-2028 ; `POST /referentiel/annees/{2027-2028}/affectations-lot` affecte Rose en 4B ; il faut encore `POST /eleves/{Rose}/scolarisations` et `PUT /eleves/{Rose}/inscriptions/{2027-2028}` ; Rose reste `status: preinscrit, classe: null`. Le lot I1 avait noté l'écart (« inscription prévue sur `ent_eleve_scolarite_annee` avant E4 ») ; l'intégration E4 + I1 ne l'a pas résolu.
**Test ajouté en échec** : `rp2_10_inscription_validee_visible_dans_la_scolarite`. Proposition : la validation appelle le port E4 (scolarisation à la date de rentrée + inscription annuelle `prevue`) ; l'ouverture de l'année bascule `prevue` → `en_cours`.

**RP2-11 — manque, élevée — Le secrétariat ne peut pas valider une inscription.**
*Preuve* : secrétariat → `POST /inscriptions/dossiers/{id}/valider` → 403, `GET …/correspondances` → 403 ; actions proposées sur un dossier admis : `["commenter","pieces","relancer"]` (direction : `…,"valider"`). Conception : INS-05 « ADF ou DIR · `inscriptions.valider` ». La décision (INS-04) est bien réservée à la direction.
**Test ajouté en échec** : `rp2_11_secretariat_valide_une_inscription`. Changement de droit : à valider par Florian (Q-RP2-1).

**RP2-12 — manque, moyenne — Un départ en cours d'année laisse l'inscription annuelle « en cours ».**
*Preuve* : `POST /eleves/{Seamus}/scolarisations/cloturer {"date_sortie":"2026-11-06","motif":"demenagement",…}` → 200, statut `sorti`, retiré de la classe et du trombinoscope ; mais `GET /eleves/{Seamus}/scolarite` → `inscriptions: [{"etat":"en_cours"}]`.
**Test ajouté en échec** : `rp2_12_sortie_termine_l_inscription_annuelle`.

**RP2-17 — manque, moyenne — Aucune proposition (famille ou terrain).** Aucune route `…/propositions` : la famille ne peut pas signaler une erreur d'identité (ELV-18), ni un assistant d'éducation « il s'appelle Yanis, pas Yannis » (ELV-40), ni le secrétariat décider (ELV-19) ; l'action `eleves.propositions.traiter` n'a rien à traiter. *Preuve* : liste des 592 routes extraites du source. **Signalé** (lot E5 ou suivant).

**RP2-18 — manque, moyenne — Pas d'organisme gardien.** Le tuteur ASE se saisit comme une personne (`nom: "Aide sociale à l'enfance du Rhône"`, civilité `autre`, lien `tuteur`) : pas de référent nommé distinct de l'organisme, pas de droits portés par l'organisme (M-075), pas de « signer les actes du quotidien » distinct (M-076). **Signalé.**

**RP2-19 — manque, moyenne — Majorité sans suite visible.** La correction de la date de naissance rend Neville majeur à l'instant (`majeur:true`), mais rien ne se passe : aucun événement de majorité (tâche quotidienne coupée en labo), droits des parents inchangés, pas de préparation à J-30 (ELV-39), aucun compte élève majeur dans le labo pour le jouer (RP1-12 et Q-RP1-4 toujours ouverts). **Signalé.**

**RP2-20 — manque, moyenne — Garde alternée réduite à un champ.** `garde: "alternee"` est enregistré sur le lien de James, mais rien ne s'en sert : pas de « foyer du jour », pas d'échange ponctuel (M-057), pas de détection de déclarations contradictoires (M-056). **Signalé** (lots A2–A3).

**RP2-21 — manque, moyenne — Activation des comptes sans messagerie, et messages inexacts.** Après acceptation d'une invitation : réponse « Un lien pour choisir votre mot de passe vous a été envoyé » alors qu'aucun lien n'a été créé (aucune ligne `ent_password_reset`, envoi en échec journalisé seulement) ; l'ancien chemin répond honnêtement `activation_envoyee:false`. Même blocage que RP1-13. **Signalé.**

**RP2-22 — manque, faible — La famille ne voit pas l'INE de son enfant.** `GET /eleves/{Rose}/identifiants` par la mère → 403 « réservé au personnel » ; la conception (§10.2) ouvre G0 à G3 aux familles pour leur enfant. **Signalé.**

### 3.3 Défauts

**RP2-23 — défaut, moyenne — Vocabulaires incohérents et messages qui ne guident pas.** Régime `DP` / `EXT` sur la fiche et dans le labo, mais `demi_pension` / `externe` exigés à la préinscription (et `regime: "demi_pension"` affiché ensuite sur la fiche de Rose) ; lien `mère` (labo) contre `mere` (formulaire) ; « civilité inconnue », « motif inconnu pour cette nature de modification », « type de pièce inconnu » sans la liste des valeurs admises. **Signalé.**

**RP2-24 — défaut, moyenne — La mère doit reconfirmer le lien de son second enfant.** Après validation de Jade avec la fiche existante d'Inès, son lien reste « en attente » (`GET /dossier/mes-liens-en-attente` → Jade) alors qu'elle est la déclarante connectée. **Signalé.**

**RP2-25 — défaut, faible — Foyer vide accepté.** `POST /foyers {}` → 201 `{"libelle":null,"nb_eleves":0}`. **Signalé.**

**RP2-26 — défaut, faible — Historique du secrétariat trompeur après un changement sensible.** Il ne montre que l'ancienne version, sans dire qu'une version plus récente existe (lié à RP2-02). **Signalé.**

**RP2-27 — défaut, faible — Données de labo.** L'année 2025-2026 (passée) est au statut `preparation` ; le seed ne contient ni compte d'élève majeur ni rôle « Accueil ». **Signalé.**

### 3.4 État après correctifs

| Constat | Gravité | Statut | Fichier corrigé | Test |
|---|---|---|---|---|
| RP2-01 | élevée | **corrigé + test** | `ent-db/src/service.rs` (`get_dossier_for_actor`) | `recette_personas_2_dossier::rp2_01_…` |
| RP2-04 | élevée | **corrigé + test** (à confirmer) | `dossier_personnes.rs`, `dossier_protection.rs` | `recette_personas_2_ouvertes::rp2_04_…` |
| RP2-10 | élevée | **corrigé + test** (à confirmer) | `eleve_ports.rs` | `rp2_10_…` |
| RP2-11 | élevée | **corrigé + test** (à confirmer) | `ent-auth/actions.rs`, `inscriptions.rs` | `rp2_11_…` |
| RP2-02 | moyenne | **corrigé + test** (à confirmer) | `domain-eleve/identite.rs` | `rp2_02_…` |
| RP2-03 | moyenne | **corrigé + test** | `service.rs`, `dossier_personnes_ports.rs` | `rp2_03_…` |
| RP2-05 | moyenne | **corrigé + test** (à confirmer) | `dossier_personnes.rs` | `rp2_05_…` |
| RP2-12 | moyenne | **corrigé + test** | `scolarite.rs` | `rp2_12_…` |
| RP2-13 | moyenne | **test en échec** | fusion des correctifs de la recette 1 | `rp2_13_…` |
| RP2-14 | faible | **corrigé + test** | `dossier_droits.rs` | `rp2_14_…` |
| autres | — | signalés | — | — |

---

## 4. Comparaison avec Charlemagne (fonctions jouées)

Sources : `docs/MATRICE_CHARLEMAGNE.md` (fonctions de Charlemagne déduites, non lues dans sa documentation).

| Fonction | Ce que la recette a vu |
|---|---|
| Admissions en ligne, pièces, instruction, décision | fonctionne ; secrétariat bloqué à la validation (RP2-11) |
| Inscription jusqu'à la rentrée | **non d'un seul tenant** (RP2-10) |
| Fiche élève, identité, historique | fonctionne, avec un historique daté et motivé que Charlemagne n'a pas ; adoption (RP2-02) |
| INE et identifiants | fonctionne (collision, contestation, révocation) |
| Responsables, foyers, parents séparés | fonctionne par les routes A1 ; route historique corrigée (RP2-01) ; ancien chemin à retirer (RP2-03) |
| Protection (adresse, ne pas solliciter, interdiction de remise) | au-delà de Charlemagne ; contournable par rattachement (RP2-04) |
| Sorties, radiations, retours | fonctionne ; inscription annuelle non close (RP2-12) ; pas de certificat (RP1-29) |
| Fratrie, réutilisation des personnes | fonctionne (correspondances) |
| Corrections par la famille | **absent** (RP2-17) |

---

## 5. Tests ajoutés

| Fichier | Test | Constat | État |
|---|---|---|---|
| `ent-api/tests/recette_personas_2_dossier.rs` | `rp2_01_dossier_historique_respecte_la_minimisation_a1` | RP2-01 | vert (corrigé) |
| `ent-api/tests/recette_personas_2.rs` | `rp2_t01_…` à `rp2_t06_…` (familles et historique, ne pas solliciter et adresse protégée de bout en bout, professeur de groupe, autre établissement, INE, écritures refusées) | ce qui tient (§0.3) | 6 verts |
| `ent-api/tests/recette_personas_2_ouvertes.rs` | `rp2_02_…`, `rp2_03_…`, `rp2_04_…`, `rp2_05_…`, `rp2_10_…`, `rp2_11_…`, `rp2_12_…`, `rp2_13_…` | RP2-02, 03, 04, 05, 10, 11, 12, 13 | `#[ignore = "FAILLE OUVERTE …"]` / `"MANQUE …"` |

Lancer les tests ouverts : `cargo test --locked -p ent-api --features labo --test recette_personas_2_ouvertes -- --ignored --test-threads=1` (ils doivent échouer tant que le problème existe). Base locale dont le nom contient `test`, `proto` ou `lab`, `ENT_ENV=lab`, `ENT_ALLOW_LABO=1`.

## 5 bis. Validation

En local, PostgreSQL 16, base de test jetable `ent_recette2_test`, sur le code final de la branche :
- `cargo test --locked -p ent-api --features labo --test recette_personas_2 --test recette_personas_2_dossier --test recette_personas_2_ouvertes -- --test-threads=1` : **7 réussis, 0 échec, 8 ignorés** (≈ 5 min, surtout les amorçages de labo).
- Passe `--ignored` de `recette_personas_2_ouvertes` : les **8 tests échouent**, chacun pour la raison annoncée (message qui cite le constat).
- Rejoués par le sous-agent du correctif : matrice `ent-api --test separation` **41 réussis** ; `ent-db --features labo --lib` (scénarios qui appellent `get_dossier_for_actor`, `tests_dossier_personnes`) **11 réussis**.
- `cargo clippy --locked --workspace --all-targets` et la même avec `--features ent-api/labo,ent-lab/labo` : **0 erreur** (avertissements préexistants seulement ; aucun sur les lignes modifiées ni dans les nouveaux fichiers).
- Formatage : `cargo fmt … -p ent-web-ui -p ent-web-api -p ent-web-app -p ent-web-server -p ent-web-session --check` et `cargo fmt … -p ent-types --check` : OK ; nouveaux fichiers de test vérifiés par `rustfmt --edition 2021 --check`. Aucun `cargo fmt --all` ; `service.rs` non reformaté.
- Non faits : suite complète `ent-db` et `ent-lab`, CI GitHub, frontend Leptos (non modifié), navigateur.

## 6. Questions pour Florian

- **Q-RP2-1** (RP2-11) : le secrétariat valide-t-il les inscriptions admises par la direction (conception INS-05 : oui) ?
- **Q-RP2-2** (RP2-02) : après une adoption ou un changement de prénom, qui peut encore lire l'ancienne identité ? Proposition : la direction seule (G6), version remplacée scellée.
- **Q-RP2-3** (RP2-04) : la marque « ne pas solliciter », tant qu'elle n'est pas confirmée par la direction, doit-elle déjà bloquer le rattachement d'un compte, la fusion et la confirmation au guichet ? Proposition : oui, avec un écran de confirmation de la direction.
- **Q-RP2-4** (RP2-05) : un élève peut-il n'avoir aucun payeur ?
- **Q-RP2-5** (RP2-10) : à la validation, ouvrir directement la scolarisation à la date de rentrée et l'inscription annuelle « prévue » (proposition) ?

**Décisions du pilote — appliquées, à confirmer par Florian (branche `ent-recette-2-correctifs`)**
- **Q-RP2-5 / RP2-10 — appliqué, à confirmer** : à la validation, la scolarisation à la date de rentrée (ou aujourd'hui si l'année a commencé) et l'inscription annuelle « prévue » sont écrites dans le modèle du socle 2 (E4) ; l'affectation en classe reste une action ensuite. Idempotent, même transaction, journalisé. Limites : si l'année cible n'existe pas encore dans le référentiel, la ligne attend (`ent_eleve_inscription_a_traiter`, cause `annee_future`) jusqu'à la préparation de l'année ; le statut texte hérité reste « préinscrit » jusqu'à la date d'entrée (le statut E4 est « à venir »).
- **Q-RP2-1 / RP2-11 — appliqué, à confirmer** : nouvelle action `inscriptions.valider` (secrétariat et direction) pour valider et lire les correspondances ; `inscriptions.decider` (admission) reste à la direction.
- **Q-RP2-3 / RP2-04 — appliqué, à confirmer** : fusion, rattachement de compte et confirmation au guichet sont refusés (409 explicite, sans rien dire de l'élève) si une personne concernée est « ne pas solliciter », même à confirmer ; ils passent avec `confirmation_direction: true`, accepté de la direction seule (`responsables.protege.gerer`) et journalisé (`ne_pas_solliciter_operation_confirmee`). Le test RP2-04 a dû changer de prémisse : l'ancien chemin ne crée plus de doublon de Marc (RP2-03), la personne à fusionner vient d'un autre e-mail ; l'assertion (fusion refusée) est inchangée.
- **RP2-03 — appliqué** : `POST /foyers/{id}/liens` réutilise la fiche du même e-mail (compte rattaché à la fiche existante) ; pour une personne « ne pas solliciter » il refuse (aucun compte, aucune activation) et renvoie vers l'ajout de responsable du dossier.
- **Q-RP2-2 / RP2-02 — appliqué, à confirmer** : pour un changement d'état civil sensible, les versions antérieures sont scellées (G6 seul) ; le secrétariat lit l'identité courante sur la fiche, pas dans l'historique. Prénom d'usage et corrections ne scellent rien.
- **Q-RP2-4 / RP2-05 — appliqué, à confirmer** : liste de payeurs vide refusée.
- **RP2-12 — appliqué** : la clôture de scolarisation termine l'inscription annuelle en cours (date de fin = départ ; état « terminée » si la date est atteinte) ; historique et prévues intacts.
- **RP2-14 — appliqué** : conflit déclaré par l'agent pour lui-même (professeur : élève de son périmètre) ou par la direction ; sinon refusé. Test ajouté.
- RP2-13 non traité (intégration à part) ; `perimetre.rs` non modifié.
- Restent ouvertes de la recette 1 : Q-RP1-2 (annuaire), Q-RP1-4 (élève majeur), Q-RP1-5 (suspension par le secrétariat).

## 7. Faits, hypothèses, limites

- **Faits** : chaque preuve a été jouée sur le serveur local (journal de requêtes de la session) ; les tests §5 rejouent les constats en process sur le routeur réel.
- **Comportement lié à l'autre session** (corrections de tests en échec sur `ent-integration-lot8b`, en cours) : aucun écart observé sur le serveur qui en découle ; la recette n'a rien modifié de ce que cette session corrige.
- **Limites** : base de 10 élèves par établissement ; pas de navigateur ; pas de messagerie (codes, invitations et activations simulés en base de labo) ; tâche quotidienne coupée (majorité non jouée de bout en bout) ; aucun compte d'élève majeur ; module santé non regardé ; décès d'un élève non joué ; deux comptes agent et parent non rapprochés (limite connue d'A1 : seule la déclaration protège, jouée ici).
- **Données de labo complétées** (base locale) : code de préinscription retrouvé, jeton d'invitation remplacé, mot de passe de labo posé sur les comptes d'Inès et de Marc, affectation de deux élèves à un groupe.
