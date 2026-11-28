# Socle 27, palier G5 — **registre des violations de données** (livraison)

Branche `claude/project-thread-rrdacj`, partie d'`origin/ENT` (règles 25 et 39). Migration **2547**.
Conception : `docs/socles/27_RGPD_ARCHIVES.md` §8.7 (entier), §14.10, §17.6, §18.2 point 5, §21 (actions
RGP-240 à RGP-259, RGP-311, RGP-312 ; règles R-RG21, R-RG32, R-RG36, R-RG40, R-RG50 ; cas CL27-41 à CL27-45).

---

## 1. En bref

| | |
|---|---|
| Migration | **2547** (`2547_rgpd_g5_violations.sql`), en **fin** du tableau de `migrate.rs` (règle 38) |
| Tables créées | `rgp_violation`, `rgp_violation_compteur`, `rgp_violation_categorie`, `rgp_violation_mesure`, `rgp_violation_notification`, `rgp_violation_personne`, `rgp_violation_etape`, `rgp_violation_plateforme_etab` (8, sous RLS) + `rgp_violation_plateforme` (**P**, plateforme, sans RLS) |
| Non créée | `rgp_violation_echange` : échanges avec la CNIL ou les personnes, palier G11 (numéro réservé 2557) |
| Actions ajoutées | 3 : `violations.declarer` (tout personnel), `violations.gerer` (direction par défaut), `violations.decider` ★ (direction) |
| `Action::TOUTES` | **286 → 289** (règle 28) |
| Code serveur | `ent-db/src/violations.rs` (service, grille, brouillon CNIL, CSV, alertes), `violations_information.rs` (personnes, art. 34), `violations_plateforme.rs` (éditeur → établissements), `ent-api/src/rgpd_violations.rs` (16 routes), `ent-types/src/rgpd_violations.rs` |
| Tâche | `violations_echeances` dans `ent-api/src/main.rs` (toutes les 5 min, `ENT_TACHES_VIOLATIONS=0` la coupe) |
| Code frontend | `ent-web-api/src/rgpd_violations.rs`, `ent-web-app/src/modules/rgpd_violations/` (4 écrans + modèle pur), `web/style/rgpd-violations.css` |
| Tests | 15 (ent-db, labo) + 11 (ent-db, natifs) + 4 (ent-types) + 1 (ent-api, HTTP) + 6 (modèle d'écran) + 3 (rendu serveur web) = **40** |
| `#[ignore]` ajoutés | aucun |

## 2. Fait

### 2.1 Déclarer en trente secondes (RGP-240 à RGP-242, CL27-45)

* `/web/incident` : une phrase obligatoire, des précisions et « depuis quand » facultatifs, une case « aucune donnée
  personnelle n'est touchée ». La **prise de connaissance est l'horodatage serveur** de la déclaration. La case ne
  range pas l'incident hors des violations (elle éteindrait l'horloge) : l'avis du déclarant est noté à la
  chronologie et la direction requalifie.
* Le déclarant reçoit la référence (`V-2027-003`) et un merci ; il suit l'**avancement** (reçue, en cours, traitée)
  et rien d'autre : ni qualification, ni risque, ni échéance (R-RG32, test dédié côté base et côté HTTP).
* Les gestionnaires (direction ; DPO et référent sécurité dès qu'ils détiennent `violations.gerer`) sont alertés
  **tout de suite, sur tous leurs canaux, sans attendre la fin d'une plage calme** ; le déclarant lui-même n'est pas
  alerté de sa propre déclaration.
* Saisie par un gestionnaire (`/web/rgpd/violations/saisir`) : la prise de connaissance peut **précéder** la saisie
  (appel de la veille), au plus d'un an, jamais dans le futur.
* Port système `violations_declarer_systeme` (supervision du socle 28, comptes compromis du socle 1) : idempotent
  par (source, objet) sous verrou transactionnel ; la même alerte ne crée jamais deux violations.

### 2.2 Horloge des 72 h (R-RG21)

`echeance_cnil_le = connue_le + 72 h` exactement (contrainte en base, test au passage à l'heure d'été). Compte à
rebours à la lecture, arrondi vers le bas ; **jamais bloquant** : une notification tardive est acceptée avec un
motif obligatoire. Rappels **H-24 et H-6**, une fois par palier (dédupliqués par la chronologie), par la tâche
`violations_echeances` ; aucun rappel quand l'échéance est réglée (notifiée, non-notification justifiée, incident de
sécurité, close).

### 2.3 Qualification, grille, décision (RGP-243 à RGP-246)

* Nature (confidentialité, intégrité, disponibilité), catégories du catalogue de conservation, nombre de personnes
  (estimé ou calculé), mineurs, conséquences.
* **Grille déterministe** `proposer_risque` (pure, testée) : données de santé, ou catégorie particulière divulguée
  ou altérée → **élevé** ; autres données que l'identification divulguées ou altérées, mineurs concernés par une
  divulgation, données particulières indisponibles, plus de 50 personnes → **avéré** ; identification seule (liste
  de classe, CL27-41) ou indisponibilité de données ordinaires → **faible**. Chaque proposition dit ses **raisons**.
* Le DPO donne son **avis**, puis la **direction décide** (`violations.decider` ★) avec un motif. Contrainte en
  base : pas de décision sans avis.

### 2.4 Notification CNIL (RGP-247 à RGP-250)

* **Brouillon** rubrique par rubrique de l'art. 33.3 (`GET …/brouillon-cnil`) ; une rubrique non renseignée le dit
  et figure dans « à compléter ». ScolIA **ne transmet rien** : la notification se fait sur le téléservice.
* Saisie de la date et de la référence CNIL (append-only, rang 1) ; **notifications complémentaires** (rangs
  suivants) ; une notification initiale ne se réécrit pas (déclencheur).
* **Non-notification** : seulement pour un risque décidé faible, motivée, gardée au registre (art. 33.5).

### 2.5 Personnes concernées et information (RGP-244, RGP-251, RGP-252, art. 34)

* Calcul depuis une **classe** (affectations au jour de l'établissement), des **élèves** ou des **comptes** : les
  titulaires de l'autorité parentale joignables, et l'élève lui-même à partir de 15 ans ; un compte désigné doit
  appartenir à l'établissement. Le nombre devient exact.
* **Aperçu** (personnes, destinataires, mineurs, personnes sans destinataire joignable), puis information
  **confirmée par le nombre** de destinataires : si le calcul a changé entre-temps, rien ne part. Message en
  langage clair, contact du DPO ajouté, envoi par le socle 8 (courriel et notification). **Une seule fois** ; le
  calcul est alors figé (déclencheur).
* Risque élevé : information **ou** exception motivée (art. 34.3) exigée pour clore.

### 2.6 Mesures, clôture, registre (RGP-253, RGP-256 à RGP-259)

* Mesures prises ou envisagées ; **chronologie en ajout seul** écrite dans la transaction du fait (R-RG40).
* Clôture avec **retour d'expérience** ; la fiche dit ce qui manque (`manque_pour_clore`, même règle que la
  contrainte de base). Une violation close est **figée** (déclencheur), revue à 30 jours datée.
* Registre trié par urgence, filtres (année, état, nature, à notifier), **export CSV** (formules neutralisées).
* Incident de sécurité sans atteinte : même registre, sans horloge exigée, clos sans risque ni notification.

### 2.7 Violations de la plateforme (RGP-254, RGP-255, RGP-311, R-RG36, CL27-44)

* L'éditeur déclare **une fois** (`P-2027-002`), puis **propage** à chaque établissement concerné avec **ses seuls
  éléments** : une violation d'origine `plateforme` est créée au registre de l'établissement, son horloge part de
  l'instant où l'éditeur l'informe (art. 33.2), sa direction est alertée ; idempotent ; la fiche montre ce que
  l'éditeur sait, sans rien d'un autre établissement (test).
* Couche compte touchée : ScolIA, responsable, saisit sa propre notification (une fois).
* **Appels système** sans route : la console plateforme n'existe pas encore (même régime que les sous-traitants de
  la plateforme, G2 ; exceptions motivées dans `garde_rls_stricte.rs`).

## 3. Défauts prudents (D-069), à relire par Florian

1. Une action ★ distincte `violations.decider` pour la direction (décision du risque, non-notification,
   information des personnes, exception) ; `violations.gerer` à la direction par défaut, à donner au DPO et au
   référent sécurité par rôle personnalisé.
2. L'avis du DPO est **requis** avant la décision du risque (contrainte en base).
3. La notification CNIL peut être saisie **avant** la décision du risque (on notifie quand on peut).
4. Prise de connaissance antérieure : gestionnaire seulement, un an au plus.
5. Seuils de la grille : 50 personnes pour « avéré » ; catégories particulières listées dans `violations.rs`.
6. Élève informé lui-même à partir de **15 ans** ; majorité inconnue ou date de naissance imprécise : traité comme
   mineur (seuls les titulaires sont informés).
7. Information des personnes **refusée** pour un risque faible (on n'inquiète pas sans raison).
8. Alerte urgente : tous canaux, hors plage calme ; le déclarant est exclu des destinataires.
9. Un risque relevé après une non-notification justifiée fait **tomber** la justification (notifier, en retard
   motivé au besoin).
10. Information des personnes refusée quand aucun destinataire n'est joignable : courrier, puis exception motivée.

## 4. Non fait (et pourquoi)

* Échanges tracés avec la CNIL et les personnes (`rgp_violation_echange`) : palier G11.
* Fiches PDF du registre (NX27-18) : **faites dans le complément** (`27_G5_COMPLEMENT_LIVRAISON.md`), en-tête
  textuel en attendant le gabarit du socle 7.
* Console plateforme : **commande d'opérateur faite dans le complément** (`ent-api plateforme-violation`) ; pas de
  route HTTP de l'éditeur.
* Déclaration depuis l'application mobile : **faite dans le complément** (écran « Signaler un incident », contrat MB2).
* Choix des élèves ou des classes par liste à l'écran : la fiche accepte des identifiants collés ; un sélecteur
  viendra avec la fiche hub des élèves.

## 5. Vérifications locales

PostgreSQL 16 local (base `ent_ci_test`), Rust 1.99.0, nextest 0.9.150, `ENT_ENV=test`, conteneur de 15 Go.

| Vérification | Résultat |
|---|---|
| `cargo fmt --check` (frontend, `ent-types`) | vert |
| 3 clippy `-D warnings` (espace de travail, avec labo, `ent-types`) | verts, 0 avertissement |
| Tests ciblés G5, RLS, gardes, `separation` (`ent-db`, `ent-api`, `ent-types`, `ent-auth`) | verts |
| Frontend : clippy SSR, serveur (avec et sans labo), wasm ; tests `ent-web-*` ; rendu serveur (`rgpd_violations`, `rgpd_droits`, `entetes_securite`, `droits_parite`) | verts |
| Suite complète avec labo | **interrompue** à 2 757 / 4 386 tests (limite de 2 h du conteneur, compilation sur un cœur faute de mémoire) ; 11 échecs, tous hors G5 : délais d'instruction sous charge (`57014`) et deux tests globaux de `tests_eleve_scolarite` sur une base partagée de 17 000 élèves ; les quatre échecs relancés seuls sont verts |
| Suite complète sans labo | non menée (compilation tuée par manque de mémoire) |

La CI de la PR fait foi pour la suite complète.

## 6. Contre-revue (relecture indépendante du serveur)

Rien de critique. Corrigé, avec un test quand c'est testable :

| Constat | Correction |
|---|---|
| Information « faite » avec zéro destinataire | refusée, renvoi vers l'exception motivée (art. 34.3) |
| Impasse : risque relevé après une non-notification | la justification tombe à la décision, chronologie (`g5_risque_releve_apres_non_notification`) |
| Propagation de plateforme concurrente : erreur brute au lieu de l'idempotence | verrou transactionnel par (violation, établissement) |
| Un déclarant pouvait éteindre l'horloge par la case « incident de sécurité » | déclaration toujours « violation », avis noté, requalification par la direction (test mis à jour) |
| Rappel d'échéance perdu sans destinataire | l'étape de dédoublonnage n'est posée que si quelqu'un est prévenu |
| Rappels en double entre instances | verrou par (violation, palier) |
| Test de numérotation fragile (1er janvier, tests parallèles) | croissance dans la même année, sans contiguïté |

## 7. Compteurs et conflits attendus à la fusion

`Action::TOUTES` (G3b-G4 passe aussi de 286 à 288 : total attendu 291), nombre de tables sous RLS
(`tests_rls.rs`, +8), `SOURCES` de `routes_src.rs` (+1), `registre_donnees.rs`, ordre de `migrate.rs` (2545, 2546
puis 2547), déclarations de `lib.rs`.
