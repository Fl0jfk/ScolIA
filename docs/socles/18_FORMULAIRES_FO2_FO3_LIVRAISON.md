# Socle 18 — lots FO2 (création de formulaires) et FO3 (accords sur objets) : livraison

Conception suivie : `docs/socles/18_FORMULAIRES.md` (branche `ent-mc0hoe`) §6.2, §7, §10, §11, §14, §21.1–21.3, §27, §29, §31 ; livrable précédent : `18_FORMULAIRES_FO0_FO1_LIVRAISON.md` ; brique de signature : `DOCUMENTS_D6.md`.
Branche : `ent-socle-18-code-fo23`, partie d'`origin/ENT` (5142dc98, D6 comprise). Migrations **2364 à 2367** (plage 2360–2379 ; **2368–2379 libres**). `MIGRATION_TAG` inchangé.
Code : `ent-db/src/frm_moteur.rs` (pur : types, modèles prêts, lint de sensibilité, contrôle des questions, conditions sans boucle, empreinte, contrôle et visibilité des réponses), `frm_service.rs` (création, structure, audience, publication, validation, versions, cycle de vie), `frm_reponse.rs` (répondre, questions « accord »), `accords_fo3.rs` (règle, désaccord, `creer_pour`, dossier de preuve D6) ; `accords.rs`, `accords_service.rs`, `accords_verifier.rs` (FD-17, désaccord, preuve) ; `ent-api/src/formulaires_s18.rs` (routes `/frm/*` et `/accords/*`) ; `ent-auth/src/actions.rs` (`socle_18_formulaires`).
Tests : `tests_formulaires_fo2.rs` (8), `tests_accords_fo3.rs` (7), tests unitaires de `frm_moteur.rs` (8) et d'`actions.rs`.

**Niveau** : implémenté, testé en local (PostgreSQL 16, mode normal et mode strict de RLS), **non validé par Florian**, **pas de revue de sécurité indépendante** (auto-revue, à lancer par le pilote), **pas d'écran** (aucun Leptos dans ce lot, `ui.rs` non touché), rien n'est prêt pilote.

## Lot FO2 — création par les personnels

| Action | Livré | Test |
|---|---|---|
| FRM-120, 126, 130, 140 | création : nature (`information`, `collecte`, `mixte`), unité de réponse (5), confidentialité (`nominatif`, `confidentiel` + destinataire du personnel), ouverture, échéance, relances. **Anonyme refusé** (lot FO4 : aucune promesse d'anonymat sans le mécanisme) | `fo2_modeles_types_et_sensibilite` |
| FRM-120, 132 | **audience en lignes** (inclure / exclure ; établissement, niveau, classe, groupe, élève), **dans la portée** de l'auteur (un professeur : ses classes, tout débordement est refusé, jamais tronqué), aperçu en nombres (exclus fondus) | `fo2_audience_portee_exclusions_et_apercu` |
| FRM-121 | 5 modèles prêts (coupon de réunion, choix d'atelier, enquête de satisfaction, sortie, invitation) ; « sortie » exige l'objet d'accord « sortie facultative » actif et publié | `fo2_modeles_types_et_sensibilite`, test unitaire des modèles |
| FRM-122, 125 | sections et questions : 15 types du §11.2 (`fichier`, `creneau`, `choix_tarifie` refusés : socles 7, 19, 24) ; sensibilité classée : **C3 (santé, handicap) refusée**, **C2 réservée à la direction** et à déclarer, lint de libellé | idem, tests unitaires |
| FRM-123 | **conditions d'affichage sans boucle** : une question ne dépend que d'une question placée avant (code **et** déclencheur de base, plus contrôle de boucle à la publication) | `fo2_conditions_sans_boucle_code_et_base` |
| FRM-133, 134, 135 | publication (questions **figées par la base**, empreinte de structure), nouvelle version (copie du dernier état), qualification `forme` / `fond` (`forme` refusée si l'ossature a changé), **correction de libellé tracée** (ancienne valeur conservée) | `fo2_publication_versions_figees_et_correction` |
| FRM-145 | **validation à large audience** (seuil 200) : formulaire « soumis », `formulaires.valider`, **jamais par l'auteur ni un co-auteur** ; refus motivé ; la nouvelle version d'un formulaire à large audience attend aussi une validation | `fo2_validation_quatre_yeux_au_dela_du_seuil` |
| FRM-136, 137, 138, 139, 149 | dupliquer, prolonger (jamais raccourcir après publication), clore, rouvrir, archiver (irréversible), co-auteurs, suppression d'un brouillon jamais publié (seule suppression permise) | `fo2_cycle_de_vie_duplication_et_co_auteurs` |
| §11.3, §11.4 | **répondre** : répondant **calculé** (titulaire communicant avec autorité, ou compte de l'élève selon l'unité et la capacité), valeurs **typées** (une colonne par type, une seule renseignée, contrôle en base), conditions appliquées, remplacement d'une réponse (l'ancienne garde sa date), unités `eleve_un_titulaire` (l'autre titulaire refusé sans contenu), `eleve_chaque_titulaire`, `personne`, `eleve_lui_meme`, clôture et échéance | `fo2_repondre_valeurs_unites_et_conditions` |

Droits (D-017) : `formulaires.creer` (établissement ou **mes classes** ; direction, administratif, CPE : établissement, professeur : mes classes), `formulaires.valider` (direction seule), `formulaires.gerer` (existant : lecture de tous les formulaires). Un formulaire d'un autre auteur est **introuvable** (404, indiscernable d'un identifiant inconnu).

## Lot FO3 — accords sur objets

| Sujet | Livré | Test |
|---|---|---|
| **FD-17** « un seul accord suffit toujours » | règle `tous` **exigée** pour l'image : refus à la création d'un objet de la famille `image` à `un` (champ `desaccord_objet` ajouté à `NouvelObjet`) ; relèvement de règle (`accords_relever_regle` : on monte, jamais l'inverse, journalisé) ; refus d'utiliser comme question un objet d'image à `un` ; **règle effective** au calcul (une ligne ancienne à `un` se calcule à `tous`). L'ensemble `droit_image`, `voix`, `mention_nominative`, `enregistrement_visio`, `voyage_etranger` (FO0) est inchangé | `fd17_*` |
| FRM-124 | **question « accord » liée à un objet existant** (actif, version publiée, règle `tous` pour l'image, sujet de l'objet cohérent avec l'unité, formulaire « mixte »). Répondre écrit la soumission **et** les décisions dans la même transaction (tout ou rien), par le **même calcul** que `accords_decider` ; la réponse ne contient que **sa** décision (FD-03) ; la décision garde le lien vers la soumission | `frm124_*` |
| Règle `tous` / `un` par objet | portée par l'objet (FO1) et exercée par les formulaires : `un` (accord + refus = désaccord), `tous` (un refus refuse, le silence d'un titulaire = en attente), droit du lien exigé par l'objet | `frm124_*` |
| FRM-111, 112 | **désaccord remonté au dossier de l'élève** pour `image` et `sortie` : un refus après un accord (règle `un`) ouvre un désaccord **sur l'objet visé** ; un titulaire ou l'établissement peut déclarer un désaccord de **famille** (`image`, `sortie`, `general`) ; ouvert, il rend le verdict `desaccord` (jamais autorisé) ; **levée seulement par une pièce du dossier de l'élève** et un motif ; événements `eleve.desaccord_declare` / `_leve` publiés pour le socle 4 ; table en ajout seul | `frm111_frm112_*` |
| Objet créé par un autre socle | `accords_creer_pour` : sortie facultative, voyage en France, voyage à l'étranger ; **idempotent par source** (`ent_acc_objet_source`), règle, droit du lien, preuve et validité « événement » repris du catalogue, version 1 **publiée** aussitôt avec le texte de départ **provisoire** ; la fin suit l'événement reporté. Pas de route (le socle appelant contrôle son droit ; la brique exige un membre du personnel) | `creer_pour_*` |
| **Dossier de preuve D6** (FRM-119) | `accords_lier_document` (le document émis du socle 7 que le décideur signe, rattaché à la **version** de l'accord et à l'**élève**), `accords_decider_signe` (la décision retient une signature D6 : signataire = décideur, document rattaché, niveau suffisant, une fois), `accords_dossier_preuve` (décision, signature, circuit, niveau, méthode, date, empreintes, état de l'horodatage, **mentions des limites**) ; lecture journalisée, réservée à `autorisations.gerer`. **S2 jamais simulé** : sans signature d'un niveau S2, refus ; une signature d'un circuit **annulé ou rendu obsolète** ne prouve plus (le verdict retombe à `inconnu`) | `d6_*` |

### Limites de D6, respectées
- **Horodatage** « reçu, non vérifié » : le dossier de preuve le dit tel quel (`recu_non_verifie`) ou dit qu'il n'a pas été demandé ; aucune mention ne le présente comme qualifié ou vérifié (test sur les textes).
- **Guichet G0 interdit par défaut** : seules les signatures **personnelles du décideur** (passkey S2, code SMS S1) sont retenues. G0, G (agent), P (papier) ne sont **pas** des preuves ici (la table accepte `G` et `P` pour l'avenir, le code non).

## Compteurs (règle 28, base `origin/ENT` 5142dc98 — `origin/ENT` n'a pas bougé, fusion sans objet)

| Compteur | Avant | Après |
|---|---|---|
| `Action::TOUTES` | 247 | **249** (`formulaires.creer`, `formulaires.valider`) ; une seule ligne d'appel (`socle_18_formulaires`) |
| RLS (`tests_rls.rs`) | base ENT | **+18** tables : 11 (2364) + 3 (2365) + 2 (2366) + 2 (2367) ; toutes à RLS forcée, clés étrangères composites sur l'établissement |
| Registre des données | — | **+18** tables (`registre_donnees.rs`, domaine `dossier_eleve`) |
| `TABLES_ELEVE` (fusion de fiches) | — | **+2** : `ent_frm_cible`, `ent_frm_soumission` (restent sur la fiche d'origine, raison écrite) |
| Sources de routes (matrice de séparation) | 58 | **59** (`formulaires_s18.rs` dans `SOURCES` et `NOMS_SOURCES`) ; toute route répond 401 sans session |
| `MIGRATIONS` | — | +4 fichiers (2364–2366 après 2363 ; 2367 en fin de tableau, après les tables de signature) |
| `MIGRATION_TAG` | `3808_ntf_gels_liens_a_prevenir` | inchangé |
| Colonnes JSON (D-036) | — | 0 (un test le vérifie sur `ent_frm_*`) |
| Dépendances | — | **aucune** (`cargo metadata --locked` des deux `Cargo.lock`) |

## Écarts avec la conception (faits, à valider)

1. **Plan de migrations** : la conception numérote 2364–2365 (FO2), 2366 (anonymes), 2367 (usages déclarés). Ici : 2364 formulaires, 2365 cibles et soumissions, 2366 désaccords et objets de source, 2367 dossier de preuve. **FO4 prendra 2368 et suivants** (jusqu'à 2379). Les tables `ent_frm_participation`, `ent_frm_soumission_anonyme`, `ent_frm_valeur_anonyme`, `ent_frm_reglage`, `ent_frm_export`, `ent_frm_tirage` ne sont **pas** créées.
2. **Désaccord et socle 4** : `ent_eleve_desaccord` (socle 4, lot A4) n'est pas livrée. Le désaccord est écrit dans `ent_acc_desaccord` et **publié** comme événement `eleve.desaccord_declare` / `_leve` (contrat de `04_ADMINISTRATIF.md` §983) ; le socle 4 le reprendra, la table devenant alors une projection. Agrégat d'événement : `Annonce` réutilisé (comme FO0 et FO1).
3. **Portée d'un désaccord né d'un refus** : il vise **l'objet** concerné (`objet_ref`), pas toute la famille ; un désaccord déclaré (titulaire, établissement) vise la **famille**. La conception (§7.1.4) dit « famille » pour le désaccord déclaré, sans trancher le cas d'un refus après accord : choix **à valider**.
4. **Décision de justice** (« sans l'autorisation de l'autre » → règle `tous` **imposée** pour l'image de cet élève) : non faite ; le désaccord `image` ouvert bloque déjà la famille.
5. **Répondre** : la conception place la réponse dans FO2 ; faits ici : valeurs typées, unités, conditions, remplacement, accord. **Non faits** : brouillon enregistré automatiquement, accusé avec copie au coffre (socle 7), préremplissage (FO7), sans compte / lien nominatif, guichet, papier, reprise attestée (FO3 suite), quotas, liste d'attente, tirage au sort (FRM-128, 129), question `fichier`, `creneau`, `choix_tarifie`, fratrie « mêmes réponses » (§11.3), langue, sondage anonyme (FO4).
6. **Résultats** : aucune lecture des réponses par l'auteur (`formulaires.reponses.lire`, FO6) ; un formulaire confidentiel enregistre son destinataire mais ne lui sert rien encore. Un auteur ne voit donc que le **nombre** d'audience.
7. **Unité `foyer`** : traitée comme `personne` (une réponse par répondant, tous enfants confondus) ; la notion de foyer n'est pas distinguée.
8. **Seuil de validation** : constante 200 (`ent_frm_reglage` non créée). Rendu testable par un réglage de test du fil courant (aucun test ne dépend de l'heure ni de 200 élèves).
9. **Notifications, relances** : aucune notification à la publication (l'événement `formulaires.publie` est publié, **sans consommateur**) ; relances J-7/J-3/J-1 : le réglage s'enregistre, rien n'est envoyé (FO8). `afaire_sources.rs` (socle 29) lit toujours l'ancien module.
10. **Droits** : seuls `formulaires.creer` et `formulaires.valider` sont ajoutés ; `formulaires.guichet`, `formulaires.reponses.lire` et `formulaires.parametrer` du §23.1 attendent leurs lots. Tout professeur reçoit `formulaires.creer` en « mes classes » (la conception dit PP et PRF : la distinction du professeur principal n'existe pas encore comme portée).
11. **D6** : aucune nature « formulaires » n'est ajoutée aux natures de signature (le document à signer est émis par le module appelant avec une nature D6 existante, par exemple `voyages.autorisation_parentale`) ; **aucun document d'accord n'est émis par le socle 18** (émission des natures : D8). Guichet et papier **non retenus** comme preuves de décision (voir ci-dessus). Le dossier de preuve expose identifiants, niveaux et empreintes ; les PDF se lisent par les fonctions de lecture de D6 (qui n'ont pas de route).
12. **Fusion de fiches** : `ent_acc_desaccord.sujet_eleve_id`, `ent_acc_document_lien.sujet_eleve_id` ne portent pas le nom `eleve_id` et ne sont donc pas vus par `TABLES_ELEVE` (même limite que `ent_acc_decision`, FO1 limite 5).
13. **Ancien module** : les routes `/formulaires` (`communication_s28.rs`) restent en service ; le nouveau moteur est sous `/frm` jusqu'au retrait de 193 (FO13).
14. **Réponse à un formulaire et preuve** : les décisions prises en répondant à un formulaire gardent le canal `en_ligne` (objet S1, sans signature D6) ; un objet S2 y est **refusé** (jamais simulé). La voie qui retient une signature D6 est `accords_decider_signe` (hors formulaire) ; l'exigence « session récente ou code » pour S1 (limite 1 de FO1) n'est donc pas encore imposée aux réponses de formulaire.
15. **Objet de source** : `accords_creer_pour` n'a pas de route et se contente de vérifier que l'acteur est un membre du personnel de l'établissement ; le socle 42 doit contrôler le droit d'organiser. À brancher quand le 42 appellera la brique.

## Questions
1. Un refus après un accord (règle `un`) doit-il bloquer **toute la famille** (toutes les sorties de l'élève) ou seulement **la sortie concernée** ? (livré : la sortie concernée ; la déclaration explicite bloque la famille.)
2. La **décision de justice** impose-t-elle une règle `tous` **par élève** (table de règles imposées) en plus du désaccord ouvert ? Si oui, où la déclare-t-on (socle 4, `liens.juridique.gerer`) ?
3. Quel droit pour **lier un document** à une version d'accord et lancer la signature : `autorisations.gerer` (livré) ou l'organisateur de la sortie (socle 42) ?
4. Le seuil de validation (200) est-il bien **un nombre d'élèves** et non de destinataires (parents) ?
5. Faut-il retenir, pour une décision, la signature **guichet confirmé (G)** ou **papier contrôlé (P)** de D6 ? Elles supposent que l'agent saisit la décision au nom du titulaire (canal `guichet` / `papier`, parcours FO3 suite).

## Réponses du pilote (2026-11-17, à consigner, aucun code dans ce lot)
1. Refus après accord : on bloque **la sortie concernée** (comme livré), jamais toute la famille.
2. Décision de justice : la règle `tous` est **imposée par élève**, déclarée dans le dossier de l'élève (socle 4, autorité parentale) par la direction ou le secrétariat, avec la pièce justificative. **Suite : lot du socle 4.**
3. Lier un document à une version d'accord : `autorisations.gerer` pour l'instant ; l'organisateur du socle 42 passera par `creer_pour` quand V1 sera branché.
4. Le seuil de 200 compte les **élèves concernés** (comme livré).
5. Preuve de décision : on retient **G (guichet confirmé)** et **P (papier contrôlé)**, niveau écrit dans le dossier de preuve ; G0 jamais. **Suite** : le code ne retient aujourd'hui que passkey et code SMS (écart n°11), le parcours guichet/papier reste à écrire.

## Vérifications faites
- `cargo +1.99.0 clippy --workspace --all-targets` (avec et sans la feature `labo`) : **0 avertissement** ; `cargo check --workspace --tests` : 0 avertissement rustc.
- Tests Postgres 16 local (données fictives), `--test-threads=1` : FO2 8/8, FO3 7/7, FO0 et FO1 (régression : leur purge supprime d'abord les tables FO2 et FO3), `tests_rls`, registres, fusion.
- Jamais `cargo fmt --all` : seuls les **fichiers nouveaux** sont formatés (`rustfmt`) ; aucun `#[ignore]` ; aucun test ne dépend de l'heure réelle (jours lus en base avec de larges marges, expirations écrites à l'instant voulu).
- `cargo +1.99.0 metadata --locked` : voir le message de fin de lot.
