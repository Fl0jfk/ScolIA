# Recette par personas n° 1 — sur le logiciel réel

**Date :** 2026-11-07 — **Branche :** `ent-recette-personas-1` (base `ent-integration-lot7` + documentation `ent-mc0hoe`) — **Statut :** recette faite ; six failles ouvertes fermées et manques RP1-02 / RP1-06 livrés dans `ent-recette-1-correctifs` (2026-11-07) ; **à relire par Florian**.
**Méthode (D-021, étape 4) :** serveur `ent-api` compilé avec la feature `labo`, lancé en local (`ENT_ENV=lab`, `ENT_ALLOW_LABO=1`) sur une base PostgreSQL 16 locale jetable (`ent_recette_lab`) ; deux établissements de labo (A et B) créés par `POST /labo/bootstrap` ; toutes les actions jouées **par l'API HTTP réelle** (client Python `urllib`, jetons Bearer), comme le ferait l'interface. Aucune base, aucun secret, aucun déploiement de production ; Next.js non touché.
**Écrans Leptos** : non joués dans un navigateur cette fois (les écrans `/web/*` appellent les mêmes routes ; la recette visuelle — clavier, 375 px, lecteur d'écran — reste à faire).

Légende des statuts : **corrigé + test** (correctif dans cette branche, test qui échouait avant et passe après) ; **test ajouté en échec** (test écrit, marqué `#[ignore = "FAILLE OUVERTE …"]`, il échoue tant que la faille existe : `cargo test … -- --ignored`) ; **signalé** (décrit ici, sans test ni correctif).

---

## 0. Résumé

### 0.1 Chiffres

| Type | Critique | Élevée | Moyenne | Faible | Total |
|---|---|---|---|---|---|
| Faille | 1 | 5 | 8 | 4 | 18 |
| Fonction manquante | 0 | 3 | 3 | 0 | 6 |
| Défaut | 0 | 0 | 4 | 6 | 10 |
| Lenteur | 0 | 0 | 0 | 0 | 0 |
| **Total** | **1** | **9** | **16** | **8** | **34** |

Statuts : **8 corrigés + test** (RP1-01, 05, 09, 10, 21, 22, 23, 24), **6 tests ajoutés en échec** (failles ouvertes RP1-03, 04, 07, 11, 20, 27), **20 signalés**. Détail en §3 et §3.4.

Aucune lenteur : sur ~1 500 requêtes, aucune au-dessus de 500 ms (base de labo de 10 élèves : ne dit rien de la tenue en charge).

### 0.2 Les 10 constats les plus importants

1. **RP1-23 (critique, corrigé)** — D-022 cassée de bout en bout : dès que la direction place un cours de **groupe** dans l'emploi du temps, le professeur du groupe « suit » **toute la classe** (repli « classe texte » de `perimetre.rs`) : notes, absences, appréciations, journal, punitions, compétences, relevé de Ginny, qui n'est pas dans son groupe.
2. **RP1-24 (élevée, corrigé)** — L'**appel** d'un cours de groupe liste toute la classe et accepte de marquer absente une élève qui n'est pas dans le groupe : de fausses absences partent vers les familles.
3. **RP1-01 (élevée, corrigé)** — Un **professeur exporte** LSL, LSU, Notanet et Parcoursup (INE et notes de toute la classe), contraire à D-022 ; un professeur d'un seul groupe exporte aussi les élèves hors groupe.
4. **RP1-11 (élevée, test en échec)** — **Parents séparés** : le père (autre foyer) lit le fil de messagerie privé ouvert par la mère, où elle donne sa nouvelle adresse, et les relances de facture du foyer de la mère.
5. **RP1-21 (élevée, corrigé — RF-16)** — Le professeur de la classe lit sur la fiche élève le **motif « ordonnance de protection »** et la référence du jugement, et qui paie.
6. **RP1-22 (élevée, corrigé)** — **Conseil de classe** : le professeur d'un groupe voit, dans la préparation, les élèves hors de son groupe (moyenne générale, appréciation générale, décision, vœux).
7. **RP1-02 (élevée, signalé)** — **Aucune route** pour mettre un élève dans un groupe : la portée au groupe (D-022) est inutilisable sans SQL ; la recette a dû écrire en base.
8. **RP1-06 (élevée, signalé)** — **Pas de profil de base minimal** : un compte accueil ou AED doit partir du profil « administratif » (journal d'audit, exports ONDE, factures, demandes d'accès aux documents…) ou « CPE » ; les rôles D-017 ne font qu'ajouter des droits.
9. **RP1-29 (élevée, signalé)** — **Pas de certificat de scolarité** ni d'édition prête (convocation, courrier) : premier document du secrétariat dans Charlemagne, contraire à D-023.
10. **RP1-05 (moyenne, corrigé — RF-15)** — Avec le module santé **fermé**, tout professeur (même sans classe) liste les documents de n'importe quel élève, dont le titre « PAP Harry Potter ».

### 0.3 Ce qui tient (vérifié sur le serveur réel)
- **Cloisonnement des établissements** : aucune donnée de A lue ou modifiée depuis B (≈ 290 routes en lecture avec les identifiants de A et un jeton de B ; 16 écritures sensibles : justificatifs, punitions, identité, suspension de lien, messagerie, bulletins, conseil, appel, rôles, professeur principal, déclaration d'absence, travaux, demandes d'accès) ; en-têtes `X-Etablissement-Id`, `Host`, `X-Forwarded-Host` et paramètre `etablissement_id` sans effet.
- **Familles et élèves hors de leur périmètre** : aucun accès aux objets d'un autre enfant par les ≈ 290 routes en lecture (Draco élève, Narcissa et Molly Weasley contre les objets de Harry) ni par 18 écritures (remise de devoir, « fait », justificatif, absence, messagerie, documents à signer, sorties, PAI, carnet, vœux, identité, personnes, contacts, téléphone, e-mail) ; pièce d'un autre élève refusée dans une déclaration d'absence.
- **Sessions** : jeton révoqué à la déconnexion ; autres sessions révoquées au changement de mot de passe ; jetons forgés, tronqués ou en majuscules refusés ; connexion sur un autre établissement refusée ; retrait de rôle et suspension de lien **immédiats**.
- **Rôles paramétrables** : profil Prof ou CPE sans `roles.gerer` refusé ; rôle à un parent ou un élève refusé ; rôle d'un autre établissement introuvable ; action de sécurité hors portée « établissement » refusée.
- **Module santé fermé** : toutes les routes santé / PAI / accompagnement / aménagements répondent 403 (sauf fuites annexes RP1-05, RP1-20).
- **Saisie de notes, grille, vue de classe, relevé, appréciation** du professeur de groupe bornées à son groupe (tant qu'aucun créneau d'EDT de groupe n'existait, voir RP1-23) ; punitions bornées aux élèves suivis.
- **Méthodes HTTP inattendues** : 405 ; dates absurdes : 400 / 422 ; 5 000 identifiants : 414 ; recherche avec `%`, `_`, 5 000 caractères : sans effet.
- **Liens de fichiers** : jeton altéré refusé ; lien non délivré hors droits.

---

## 1. Personas jouées

Établissement A (collège de labo, 10 élèves en 6A, 5A, 4B, 3C) ; établissement B identique (cible des attaques inter-établissements).

| Persona | Compte | Comment il a été obtenu |
|---|---|---|
| Direction (Albus Dumbledore) | `direction1-…@ent.lab` | labo |
| Admin | `admin-…` | labo |
| Secrétariat (Argus Rusard) | profil `administratif` | labo |
| CPE (Remus Lupin) | `cpe` | labo |
| **AED** (deux variantes) | profil `cpe` seul ; profil `prof` + rôle « AED » (`absences.lire`, `absences.declarer`, `appel.verifier`, `punitions.donner`, `sorties.controler`, établissement) | créés par la direction : `POST /etablissement/personnel`, `POST /roles`, `PUT /roles/{id}/membres/{user}` |
| **Accueil** | profil `administratif` + rôle « Accueil » (`absences.declarer`) | idem (aucun profil plus petit n'existe : RP1-06) |
| Comptabilité (Horace Slughorn) | `compta` | labo |
| Infirmerie, psy | `infirmerie`, `psy` | labo |
| Professeur de classe entière (Severus Rogue, Potions 4B) | `prof` | labo (service 4B + créneau EDT 4B) |
| **Professeur d'un seul groupe** dans une classe mixte (Filius Flitwick, « 4B LV2 allemand » : Harry et Hermione, pas Ginny) | `prof` | groupe et service créés par la direction (`POST /referentiel/groupes`, `POST /referentiel/services`) ; élèves mis dans le groupe **en base de labo** (aucune route : RP1-02) ; créneau de groupe créé ensuite (`POST /edt/admin/creneaux`) |
| **Professeur principal** (Minerva McGonagall, 5A) | `prof` | `PUT /referentiel/classes/{5A}/professeur-principal` |
| Professeur sans aucune classe (« Prof 5 ») | `prof` | labo |
| Parent (Lily Potter, mère de Harry, tous droits) | `parent` | labo |
| **Parent séparé** (James Potter, père de Harry, autre foyer, autorité parentale, sans droit « sorties ») | `parent` | `POST /foyers` puis `POST /foyers/{id}/liens` par le secrétariat ; lien d'activation non envoyé (RP1-13) : mot de passe de labo posé en base de labo |
| Parent d'une autre famille (Narcissa Malefoy) ; parent sans droits sur un enfant (Molly Weasley) | `parent` | labo |
| **Élève mineur** (Harry, 14 ans) | `eleve` | labo |
| **Élève majeur** (Neville, 18 ans) | `eleve` | date de naissance passée à 2008 par la direction (`PUT /eleves/{id}/identite`) |
| Élève d'une autre famille (Draco) | `eleve` | labo |
| Direction de l'établissement B | `direction` | labo (B) |

---

## 2. Parcours joués

**Une semaine du professeur de groupe** : ses services (`/evaluations/services`), devoir créé, notes saisies (Ginny et un élève de B refusés dans le rapport de saisie), grille, vue de classe, relevé, appréciation, publication ; travail à rendre donné au groupe (types `a_faire` / `a_rendre` / `controle` seulement) ; suivi des remises ; créneau de groupe placé à l'EDT ; appel ; conseil de classe.
**Élève mineur** : agenda, remise d'un devoir avec fichier PDF, relevé, bulletins publiés, tentatives sur un autre élève ; déclaration de sa propre absence ; signature du règlement intérieur.
**Élève majeur** : déclaration et justification de ses absences, sorties, réinscription, factures, messagerie ; vue des parents après la majorité.
**Parents** : enfants, déclarations d'absence (deux routes), justificatifs, messagerie, rencontres parents-professeurs (réservation), formulaire « droit à l'image », factures ; parent séparé : tout ce que voit l'autre parent ; suspension du lien (ordonnance) puis tout ce qui reste accessible.
**Direction / secrétariat** : groupe, service, professeur principal, créneau d'EDT, fermeture de période, génération et publication des bulletins (4B et 5A), conseil de classe, rencontres, formulaire, campagne d'admission, ajout d'un responsable, suspension de lien, rôles AED et Accueil, sortie d'un élève, recherche du certificat de scolarité.
**Accueil** : absence signalée au téléphone par un parent (`/vs/declarations/accueil`), vue par la CPE.
**CPE / AED** : tableau du jour, vérification des appels, décompte des demi-journées, signalements d'assiduité proposés, types de punition, punitions.
**Comptabilité** : paramètres, articles, tarifs, factures, rejets, relances, exports ; et tout ce qu'elle peut lire d'un élève.
**Attaque** : balayage automatique des **≈ 290 routes en lecture** (gabarits extraits du source des routeurs, identifiants de Harry injectés, paramètres de requête usuels ajoutés) par six attaquants (élève d'une autre famille, parent d'une autre famille, parent sans droits, professeur sans classe, comptabilité, direction de B) ; écritures ciblées ; sessions ; méthodes ; paramètres ; liens de fichiers.

---

## 3. Constats

Format : **RP1-nn** — type, gravité — constat. *Preuve* (requête → réponse). **Statut.**

### 3.1 Failles

**RP1-23 — faille, critique — Un créneau d'EDT de groupe ouvre toute la classe au professeur du groupe (D-022).**
*Preuve* : avant le créneau, Flitwick → `GET /eleves/{Ginny}/notes` 403. Direction : `POST /edt/admin/creneaux {"classe":"4B","groupe_id":"<4B-ALL>","matiere_libelle":"Français","prof_user_id":"<Flitwick>","jour_semaine":6,…}` → 201. Ensuite Flitwick → `GET /eleves/{Ginny}/notes|absences|appreciations|journal|punitions|competences`, `GET /evaluations/eleves/{Ginny}/releve` → **200**. Cause : `perimetre.rs` (repli « emploi du temps historique ») relie le créneau à l'élève par le seul texte de classe (`c.classe = e.classe`), sans regarder `c.groupe_id`.
**Corrigé + test** : `ent-db/src/perimetre.rs` (`prof_suit_eleve_le`, `eleves_suivis_par_prof`) — un créneau de groupe ne vaut que pour les élèves affectés au groupe à la date ; test `rp1_c23_creneau_de_groupe_n_ouvre_pas_la_classe` (`ent-api/tests/recette_personas_1.rs`). Reste (socle 5) : supprimer ce repli par texte de classe (déjà prévu, `05_STRUCTURE.md` lot S0).

**RP1-24 — faille, élevée — L'appel d'un cours de groupe liste toute la classe.**
*Preuve* : Flitwick → `POST /appels {"creneau_id":"<créneau de groupe>","jour":"2026-11-07"}` → 201 ; `GET /appels/{id}` → lignes Harry, **Ginny**, Hermione ; `POST /appels/{id}/lignes {"eleve_id":"<Ginny>","etat":"absent"}` → **204**. Cause : `ouvrir_appel` (`service.rs`) remplit l'appel par `list_en_classe_au(classe texte)`.
**Corrigé + test** : `ouvrir_appel` ne garde que les élèves affectés au groupe du créneau à la date (un appel déjà ouvert avant le correctif garde ses lignes) ; test `rp1_c24_appel_de_groupe_borne_au_groupe`.

**RP1-01 — faille, élevée — Un professeur exporte LSL, LSU, Notanet, Parcoursup (D-022).**
*Preuve* : Rogue (classe entière) et Flitwick (groupe de 2) → `GET /exports/lsl?classe=4B&periode_id=<T1>` → 200 `ine;nom;prenom;classe;matiere;note_20;appreciation;periode` avec les trois élèves de 4B, idem `/exports/lsun`, `/exports/notanet`, `/exports/parcoursup?classe=4B&annee_scolaire=2026-2027`. (La route récente `POST /evaluations/exports` refuse bien les professeurs.)
**Corrigé + test** : `service.rs`, exports réservés à `eleves.gerer` (direction, secrétariat, admin) ; tests `rp1_c01_professeur_n_exporte_jamais` (`ent-db/src/tests_recette_personas.rs`) ; tests existants (`rf04d`, `ent-lab`, scénarios) adaptés : ils faisaient exporter un professeur.

**RP1-11 — faille, élevée — Parents séparés : le second parent lit les fils privés et les relances de l'autre.**
*Preuve* : Lily → `POST /messages/fils {"eleve_id":"<Harry>","sujet":"Situation familiale — confidentiel","message":"… notre déménagement au 4 rue des Lilas."}` → 201. James (autre foyer) → `GET /messages/fils` → ce fil + « Relance facture LAB-HARRY-2 » (foyer de Lily) ; `GET /messages/fils/{id}` → 200 avec le message. Les coordonnées de Lily sont bien masquées dans `/dossier/eleves/{H}/responsables`. Les fils sont rattachés à l'élève, pas à la personne (RF-34, décision produit en attente).
**Corrigé + test** (décision du pilote, D-014 et minimisation : *un responsable ne lit que les fils auxquels il participe et les documents de son propre foyer*). Migration 375 : `ent_msg_fil.prive_user_id` (fil ouvert par un responsable : privé entre lui et l'établissement ; l'élève ne le lit pas non plus) et `ent_msg_fil.foyer_id` (relance de facture : lisible des seuls **payeurs actifs** du foyer, jamais d'un élève) ; rattrapage des fils existants dans la migration. Un fil ouvert par l'établissement (carnet, alerte d'assiduité) reste visible de chaque responsable de l'élève. Contrôle unique `actor_can_access_fil` (lecture, réponse, pièce, archivage, classement) et filtre des listes ; les notifications push d'un fil privé ou de foyer ne vont qu'à ses destinataires ; les e-mails de facture et de quittance ne vont qu'aux payeurs du foyer ; un document du tiroir « famille » déposé par un responsable n'est lisible (ni listé) que des responsables du même foyer. Tests : `rp1_c11_parent_separe_ne_lit_pas_le_fil_de_l_autre_parent`, `rp1_c11_bis_documents_famille_cloisonnes_par_foyer` (relance + document). **Reste** : les événements `msg_fil_ouvert` (sujet du fil) alimentent la chronologie du personnel, jamais celle de la famille ; à vérifier au socle 4 si la chronologie s'ouvre aux familles.

**RP1-21 — faille, élevée — Le professeur voit le motif judiciaire d'une suspension de lien et les payeurs (RF-16).**
*Preuve* : direction → `POST /dossier/liens/{lien James}/suspendre {"motif":"ordonnance de protection","reference":"JAF 2026/123"}` → 200 ; Rogue → `GET /dossier/eleves/{Harry}/fiche` → `…"suspension_motif":"ordonnance de protection","suspension_reference":"JAF 2026/123",…"est_payeur":true,"part_paiement":100`.
**Corrigé + test** : `ent-db/src/dossier_personnes.rs` (vue « pédagogique » pour le personnel qui n'administre pas les personnes : liens actifs seulement, sans motif, référence, ni paiement) ; test `rp1_rf16_professeur_ne_voit_ni_motif_judiciaire_ni_payeurs` (`ent-api/tests/recette_personas_rf16.rs`).

**RP1-22 — faille, élevée — Conseil de classe : le professeur d'un groupe voit toute la classe (D-022, précision du 2026-11-07).**
*Preuve* : Flitwick → `GET /evaluations/conseils/{conseil 4B T1}/preparation` → élèves Hermione, Harry, **Ginny** (`moyenne_generale`, `appreciation_generale`, `decision`, `voeux` ; seules absences et sanctions masquées).
**Corrigé + test** : `ent-db/src/evaluations_conseil.rs` (`preparation_conseil_for_actor` ne garde que les élèves de la portée `notes.lire` de l'acteur ; direction et professeur de classe entière inchangés) ; test `rp1_c22_conseil_borne_au_groupe`.

**RP1-05 — faille, moyenne — Tout professeur liste les documents de tout élève, titres restreints compris (RF-15).**
*Preuve* : Prof 5 (aucune classe) → `GET /eleves/{Harry}/documents` → 200, 6 documents dont `{"titre":"PAP Harry Potter","tiroir":"scolaire","confidentialite":"restreint"}` (module santé fermé). Fichiers et liens bien refusés (403).
**Corrigé + test** : `list_documents_meta_eleve_for_actor` exige `dossier.lire` sur l'élève ; titre neutre « Document à accès restreint » pour un document restreint inaccessible ; test `rp1_c05_documents_hors_perimetre_et_titre_restreint`.

**RP1-09 — faille, moyenne — Un élève mineur déclare sa propre absence (RF-22).**
*Preuve* : Harry (14 ans) → `POST /eleves/{Harry}/absences {"debut":"2026-11-16","fin":"2026-11-16","motif":"appel"}` → **201** ; par `/vs/declarations/famille` → 403 (règle « majeur » correcte). Deux règles pour le même geste.
**Corrigé + test** : `actor_can_declare_absence` exige la majorité pour un compte élève ; test `rp1_c09_eleve_mineur_ne_declare_pas_son_absence`. L'élève majeur (Neville) déclare et justifie toujours.

**RP1-10 — faille, moyenne — Un élève mineur signe seul un document à signer.**
*Preuve* : Harry → `POST /docs-a-signer/{règlement intérieur}/signer` → 200, statut `signe` : les responsables n'ont plus rien à signer.
**Corrigé + test** : `signer_doc_a_signer_for_actor` refuse la signature d'un compte élève mineur ; l'élève majeur signe toujours ; test `rp1_c10_eleve_mineur_ne_signe_pas_seul`. Reste à décider quels documents un mineur peut signer lui-même (Q-RP1-6).

**RP1-03 — faille, moyenne — Le professeur d'un groupe voit la liste et le trombinoscope de toute la classe.**
*Preuve* : Flitwick → `GET /referentiel/classes/{4B}` → `eleves` : Hermione, Harry, Ginny ; `GET /classes/4B/trombi` → les trois photos. Aussi Prof 5 (sans classe) et la comptabilité.
**Corrigé + test** : `PgEnt::eleves_suivis_restreints` (action `suivi_eleve` : `None` = établissement, sinon l'ensemble des élèves suivis ; vide pour la comptabilité) filtre `GET /referentiel/classes/{id}`, `GET /classes/{classe}/trombi` et `GET /classes/{classe}/effectif` (la comptabilité garde l'effectif, identité minimisée) ; test `rp1_c03_classe_et_trombi_bornes_au_groupe`.

**RP1-04 — faille, moyenne — Tout professeur lit la fiche d'identité de tout élève.**
*Preuve* : Prof 5 (sans classe) → `GET /eleves/{Draco}`, `/eleves/{id}/dossier` (sections identité, scolarité, options), `/dossier/eleves/{id}/fiche`, `/dossier/eleves/{id}/chronologie` (« Photo mise à jour », « Passage dans l'année suivante »…), `/eleves/{id}/photo`, `/eleves/{id}/edt`, `/eleves/{id}/periodes`, `/classes/5A/trombi`, `/eleves/search`, `/eleves/labels` → 200. Les sections sensibles (notes, VS, famille, documents) sont bien masquées. C'est un « annuaire » plus large que D-022 (« toute donnée élève ») : à trancher (Q-RP1-2) — annuaire minimal (nom, classe) pour tout le personnel, fiche et chronologie pour les seuls élèves suivis.
**Corrigé + test pour la chronologie et la photo** : `chronologie_eleve_for_actor` et `photo_cle_for_actor` passent par `exiger_perimetre_prof` (action `suivi_eleve`) ; test `rp1_c04_professeur_sans_classe_ne_lit_pas_la_chronologie`. **Reste ouvert (Q-RP1-2, décision attendue)** : l'« annuaire » (`/eleves/{id}`, `/eleves/search`, `/eleves/labels`, `/eleves/{id}/dossier` identité) reste lisible de tout professeur ; ce n'est pas une régression, mais plus large que D-022.

**RP1-07 — faille, moyenne — La comptabilité lit la vie scolaire et le suivi de tout élève.**
*Preuve* : Compta → `GET /eleves/{Harry}/journal` (« Consultation du PAI »…), `/eleves/{Harry}/alertes-cpe`, `/eleves/{Harry}/dispenses-vs` (« dispense EPS (asthme, certificat médical) »), `/eleves/{Harry}/conseils` (appréciation et décision du conseil), `/eleves/{Harry}/docs-a-signer`, `/eleves/{Harry}/sorties`, `/eleves/{Harry}/internat`, `/dossier/eleves/{Harry}/chronologie`, `/classes/4B/effectif` → 200. Ses besoins : factures, repas, bourses, payeurs.
**Corrigé + test** : `ent-auth/src/actions.rs` (modèle par défaut) — la comptabilité n'a plus `suivi_eleve` ni `journal.lire` (elle garde `dossier.lire`, `facturation.*`, `bourses.*`, `repas.*`, `payeurs.definir`). Conséquences : journal, alertes CPE, dispenses VS, conseils, documents à signer, sorties, internat, chronologie, photo et trombinoscope lui sont refusés ou vides (l'effectif de classe, à identité minimisée, lui reste ouvert). Test `rp1_c07_comptabilite_hors_metier`. Un rôle D-017 peut toujours rendre ces droits à un compte comptabilité précis.

**RP1-20 — faille, moyenne — Données de santé en clair hors du module santé fermé.**
*Preuve* : Rogue (professeur de la classe), CPE, infirmerie, parent, élève → `GET /eleves/{Harry}/dispenses-vs` → `"commentaire":"Seed audit — dispense EPS (asthme, certificat médical)."` ; `GET /eleves/{Harry}/journal` → « Consultation du PAI », « … dossier psychologue » ; `GET /eleves/{Harry}/dossier` → `dispositifs: [{"kind":"pap","libelle":"Plan d'accompagnement personnalisé"}]` pour le professeur (la règle S2-11 ne lui donne que des aménagements, jamais le type de dispositif). Lié à RF-15 / RF-18.
**Corrigé + test** : (1) fiche d'un élève suivi par un professeur : `dispositifs` retiré (`dossier_sections_for_actor`) ; les aménagements restent servis par la règle S2-11 ; (2) dispenses VS : commentaire libre vidé et motif « médical » ramené à « autre » pour le personnel hors direction, CPE, infirmerie, psychologue ; (3) journal d'un élève : les traces `consultation` (« Consultation du PAI »…) ne sont servies qu'avec `audit.lire`. Test `rp1_c20_professeur_ne_voit_ni_motif_medical_ni_type_de_dispositif`. **Reste** : la CPE voit toujours le commentaire de dispense (elle gère les dispenses).

**RP1-27 — faille, moyenne — Rencontres parents-professeurs : un professeur de groupe est proposé pour un élève hors du groupe.**
*Preuve* : Molly → `GET /famille/rencontres/{rencontre 4B}/enfants/{Ginny}/creneaux` → Flitwick et Rogue (Ginny n'est pas dans le groupe de Flitwick) ; réservation possible. Même cause que RP1-23 (liste des enseignants par classe).
**Corrigé + test** : `ent-db/src/rencontres.rs` (`enseignants_de_eleve`) — côté famille (créneaux proposés et réservation), un enseignant d'un groupe n'est proposé que si l'enfant y est affecté aujourd'hui ; la vue de gestion de la direction garde tous les enseignants de la classe. Test `rp1_c27_rencontre_sans_prof_hors_groupe`.

**RP1-08 — faille, faible — Réponses « par défaut » pour un élève d'un autre établissement.**
*Preuve* : direction B → `GET /eleves/{Harry de A}/absent?jour=…` → 200 `{"absent":false}` ; `/points-vs` → `{"solde":20,"depart":20}` ; `/sorties` → `{"autorisation":null}`. Rien ne fuit, mais la réponse diffère d'un refus (oracle d'existence faible). **Signalé.**

**RP1-32 — faille, faible — Lien de fichier signé rejouable (RF-28 confirmé).**
*Preuve* : `POST /cahier/pieces/{id}/lien` (Harry) → `/fichiers/<jeton>` ; `GET` sans session → 200 (PDF), rejoué → 200, avec une session de B → 200 ; jeton altéré → 403. La charge utile du jeton (base64) laisse lire la clé de stockage `…/cahier/<id>.pdf`. **Signalé** (connu, durée 60 s).

**RP1-33 — faille, faible — Un professeur déclare un incident sur n'importe quel élève.**
*Preuve* : Prof 5 (sans classe) → `POST /eleves/{Harry}/incidents` → 201. Décision produit déjà notée (`ROLES_ET_DROITS.md`, `incidents.declarer`). **Signalé.**

**RP1-17 — faille, faible — Le corps est validé avant l'autorisation.**
*Preuve* : Draco élève → `POST /absences/{id de Harry}/justifier {}` → 422 « missing field `motif_justif` » (puis 403 avec le bon corps) : un compte sans droit découvre la forme des requêtes. **Signalé.**

### 3.2 Fonctions manquantes

**RP1-02 — manque, élevée — Mettre un élève dans un groupe. [LIVRÉ]** Aucune route n'appelle `referentiel_affecter_eleve_groupe` (seuls des tests l'utilisent) ; l'import n'écrit que la classe. Sans cela, pas de LV2, d'options ni d'ateliers, donc pas de D-022. *Preuve* : grep du dépôt ; la recette a dû écrire `ent_affectation` en base de labo. **Livré** : `GET|POST /referentiel/groupes/{id}/membres`, `DELETE /referentiel/groupes/{id}/membres/{eleve_id}?date_fin=` ; écriture = action `referentiel.gerer` (direction, admin, secrétariat ; ni professeur, ni comptabilité, ni famille, ni élève) ; lecture = `referentiel.lire` bornée aux élèves suivis (un professeur ne liste que ses élèves) ; 409 sur doublon, 404 hors établissement ; année verrouillée respectée ; chaque ajout et retrait écrit dans le journal (`domaine=referentiel`, `groupe_membre_ajoute` / `groupe_membre_retire`, auteur, groupe, date d'effet). Test `rp1_c02_gerer_les_membres_d_un_groupe` ; matrice d'isolation (`separation.rs`) : routes inventoriées, 401 sans session, groupe et élève d'un autre établissement, comptes famille / élève / compta / professeur refusés. **Non fait** : vérifier que l'élève appartient à la classe du groupe (la classe est encore en partie un texte, voir `perimetre.rs`) ; affectation en masse (import) ; interface Leptos.

**RP1-06 — manque, élevée — Profil de base minimal pour l'accueil, les AED, les agents. [LIVRÉ]** `POST /etablissement/personnel` n'accepte que les dix profils historiques (`agent`, `aed` → 403) ; un rôle D-017 n'ajoute que des droits. *Preuve* : l'accueil créé en `administratif` + rôle « Accueil » lit `/audit/journal`, `/mails`, `/sms`, `/facturation/factures`, `/exports/onde`, `/documents/grants/pending`, `/dossier/eleves/{id}/responsables` (200 partout) ; l'AED créé en `cpe` lit les notes de tout élève. **Livré** : nouveau profil `personnel` (`Role::Personnel`, `POST /etablissement/personnel {"role":"personnel"}`) **sans aucun droit propre** : `modele_par_defaut` est vide, tous les `can_*` historiques sont faux ; les rôles D-017 ajoutent les droits (ex. « Accueil » = `absences.declarer`, « AED » = appel, absences, punitions, sorties). Test `rp1_c06_profil_personnel_minimal` : sans rôle, refus partout (journal d'audit, e-mails, SMS, factures, ONDE, demandes d'accès, responsables, notes, absences, journal, chronologie, déclaration d'absence) ; avec le rôle « Accueil », la déclaration devient possible et rien d'autre. **À noter** : l'interface historique (`ui.rs`, F-001) n'est pas étendue ; les profils `agent` / `aed` restent refusés (le nom de profil est `personnel`, le métier est porté par le rôle). Le socle 6 (lot P0) reprendra ce profil comme base de ses modèles de rôle.

**RP1-29 — manque, élevée — Certificat de scolarité et éditions prêtes.** Aucune route de certificat (`/eleves/{id}/certificat*` → 404) ; `GET /editions-vs` → `[]` (les modèles sont à écrire de zéro). Charlemagne : certificat en un clic, courriers types. Contraire à D-023 (trames prêtes). **Signalé** (socle 7).

**RP1-12 — manque, moyenne — Élève majeur.** Neville (18 ans) : sorties `PUT /eleves/{id}/sorties` → 403 ; réinscription `/reinscription/famille`, `/inscriptions/famille/reinscriptions` → 403 ; aucune façon de s'opposer à l'information de ses parents (ils gardent notes, absences, bulletins). **Signalé** (socles 2 et 3 : bascule de majorité, ELV-xx).

**RP1-28 — manque, moyenne — Le professeur ne voit pas les consentements de sa classe.** Rogue → `GET /consentements/droit_image/classes/{4B}` → 403, `/formulaires/{id}/resultats` → 403 : il ne sait pas qui refuse la photo avant un projet. **Signalé.**

**RP1-30 — manque, moyenne — Types de punition absents à l'ouverture, la CPE ne peut pas les poser. [CORRIGÉ]** `GET /vs/punition-types` → `[]` ; CPE → `POST /vs/punition-types/amorcer` → 403 (direction seulement) : aucune punition possible le premier jour. **Corrigé + test** : `GET /vs/punition-types` amorce les quatre types par défaut s'il n'y en a aucun (`types_punition_ou_amorce`, idempotent) ; l'amorçage explicite reste réservé à la direction ; test `rp1_c30_types_de_punition_amorces_pour_la_vie_scolaire`.

### 3.3 Défauts

**RP1-13 — défaut, moyenne — Responsable ajouté sans lien d'activation, sans le dire.** `POST /foyers/{id}/liens` → 201, compte créé, mais aucune ligne dans `ent_mail_outbox` ni `ent_password_reset` (pas de messagerie en labo) ; le secrétariat ne le sait pas, le parent ne peut jamais se connecter. Même blocage pour la préinscription (`/preinscription/*`, vérification par e-mail). **Signalé** (réponse explicite « activation non envoyée » + bouton « renvoyer » + remise d'un code au guichet).

**RP1-18 — défaut, moyenne — Déclarations en double ; absence prévenue absente du tableau du jour.** Trois `POST /vs/declarations/accueil` identiques (même élève, même jour) → trois 201 ; Draco déclaré absent aujourd'hui n'apparaît pas dans `GET /vs/tableau-jour` (seul Harry, `prevenue:false`). **Signalé.**

**RP1-26 — défaut, moyenne — Message trompeur à la création d'un créneau.** `POST /edt/admin/creneaux` avec `"salle":"B12"` (inexistante) → 404 « élève introuvable », alors que l'aperçu `?apercu=true` répond « enregistre ». Cause : `map_db` (RF-29). **Signalé.**

**RP1-31 — défaut, moyenne — Deux modules de facturation.** `/facturation/*` (historique) et `/finances/*` (S2-9) coexistent : la facture en retard « LAB-HARRY-2 », relancée par l'ancien module, n'est pas vue par `POST /finances/relances/proposer` → `{"proposees":[]}`. **Signalé** (bascule à décider).

**RP1-15 — défaut, faible — Le secrétariat ne peut pas suspendre un lien.** `POST /dossier/liens/{id}/suspendre` par l'administratif → 403 (direction seulement) alors que c'est lui qui reçoit l'ordonnance. Choix à confirmer. **Signalé.**

**RP1-14 — défaut, faible — Second payeur en deux temps.** `POST /foyers/{id}/liens {"est_payeur":true,"part_paiement":50}` → 400 « somme des parts payeurs ≠ 100 % » ; il faut l'ajouter non payeur puis `PUT /dossier/eleves/{id}/payeurs`. **Signalé.**

**RP1-16 — défaut, faible — Messages d'erreur trompeurs.** « élève introuvable » pour un utilisateur d'un autre établissement (`PUT /roles/{id}/membres/{user}`), une salle, un rôle ou un conseil d'un autre établissement ; « valeur invalide (contrainte non respectée) » pour un rôle attribué à un parent. **Signalé** (RF-29).

**RP1-19 — défaut, faible — Génération de bulletins silencieuse.** `POST /evaluations/bulletins/generer` sur la période T0 → 200 `{"bulletins":[],"inchanges":0}` sans dire pourquoi (aucune note ?). **Signalé.**

**RP1-25 — défaut, faible — Créneau de groupe présenté comme un cours de classe.** `GET /edt/mes-creneaux` → `{"classe":"4B","groupe_id":null}` pour le cours d'allemand. **Signalé.**

**RP1-34 — défaut, faible — Statistiques du devoir à la publication.** `POST /evaluations/devoirs/{id}/publier` juste après la saisie de deux notes → `"stats":{"moyenne":null,"effectif":0}` (la vue de classe donne bien 16,5). **Signalé.**

### 3.4 État après correctifs (mis à jour en fin de recette)

| Constat | Gravité | Statut | Fichier corrigé | Test |
|---|---|---|---|---|
| RP1-23 | critique | corrigé + test | `ent-db/src/perimetre.rs` | `recette_personas_1::rp1_c23_…` |
| RP1-24 | élevée | corrigé + test | `ent-db/src/service.rs` (`ouvrir_appel`) | `recette_personas_1::rp1_c24_…` |
| RP1-01 | élevée | corrigé + test | `service.rs` (4 exports) | `tests_recette_personas::rp1_c01_…` |
| RP1-21 | élevée | corrigé + test | `ent-db/src/dossier_personnes.rs` | `recette_personas_rf16::rp1_rf16_…` |
| RP1-22 | élevée | corrigé + test | `ent-db/src/evaluations_conseil.rs` | `recette_personas_1::rp1_c22_…` |
| RP1-05 | moyenne | corrigé + test | `service.rs` (`list_documents_meta_eleve_for_actor`) | `tests_recette_personas::rp1_c05_…` |
| RP1-09 | moyenne | corrigé + test | `service.rs` (`actor_can_declare_absence`) | `tests_recette_personas::rp1_c09_…` |
| RP1-10 | moyenne | corrigé + test | `service.rs` (`signer_doc_a_signer_for_actor`) | `recette_personas_1::rp1_c10_…` |
| RP1-11 | élevée | corrigé + test (décision du pilote) | `ent-db/src/msg_repo.rs`, `service.rs`, `foyer_repo.rs`, migration 375 | `rp1_c11_…`, `rp1_c11_bis_…` |
| RP1-03 | moyenne | corrigé + test | `service.rs` (`eleves_suivis_restreints`), `routes.rs`, `referentiel.rs` | `rp1_c03_…` |
| RP1-04 | moyenne | corrigé + test (chronologie, photo) ; annuaire : décision Q-RP1-2 | `dossier_personnes.rs`, `service.rs` | `rp1_c04_…` |
| RP1-07 | moyenne | corrigé + test | `ent-auth/src/actions.rs` | `rp1_c07_…` |
| RP1-20 | moyenne | corrigé + test | `service.rs` | `rp1_c20_…` |
| RP1-27 | moyenne | corrigé + test | `ent-db/src/rencontres.rs` | `rp1_c27_…` |
| RP1-02 | élevée (manque) | livré + test | `referentiel_repo.rs`, `ent-api/src/referentiel.rs` | `rp1_c02_…`, matrice `separation.rs` |
| RP1-06 | élevée (manque) | livré + test | `ent-auth/src/roles.rs` | `rp1_c06_…` |
| RP1-30 | moyenne (manque) | corrigé + test | `discipline_vs.rs` | `rp1_c30_…` |
| autres | — | signalés | — | — |

**Tests existants adaptés** (ils encodaient l'ancien comportement, aucun supprimé ni désactivé) : `tests_revue_finale::rf04d` (un professeur est désormais refusé aussi pour sa propre classe en LSL / LSUN) ; matrice `ent-api/tests/separation.rs`, ligne « documents-liste » (élève non lié : `STAFF_HORS_PROF`, comme la ligne « journal ») ; scénarios `ent-db/src/scenarios.rs` et `ent-lab/src/main.rs` (exports faits par l'administratif au lieu du professeur).

**Effets de bord connus des correctifs** : la comptabilité ne reçoit plus l'alerte de fiche « payeur avec lien suspendu » (elle ne voit plus les liens suspendus) ; un professeur ne liste plus du tout les documents d'un élève qu'il ne suit pas (il ne peut donc plus en demander l'accès depuis cette liste).

**Validation** : voir §5 bis.

---

## 4. Comparaison avec Charlemagne (fonctions jouées)

Sources : `docs/MATRICE_CHARLEMAGNE.md` (dont les limites s'appliquent : fonctions de Charlemagne déduites, non lues dans sa documentation).

| Fonction (matrice) | État annoncé | Ce que la recette a vu |
|---|---|---|
| VS-01 appel par séance | codé | fonctionne pour une classe ; **faux pour un groupe** (RP1-24) |
| VS-03 vérification des appels non faits | codé | fonctionne (`/vs/verification-appels`) |
| VS-04 absences à la séance, demi-journées | codé | décompte et signalements proposés corrects |
| VS-06 déclaration par la famille | codé | fonctionne (deux routes aux règles différentes : RP1-09) |
| VS-11 absence déclarée par l'accueil | codé | fonctionne ; doublons et tableau du jour (RP1-18) ; profil accueil trop large (RP1-06) |
| VS-16 autorisations de sortie | codé | droits cochés respectés ; élève majeur exclu (RP1-12) |
| Discipline : punitions | codé | bornées ; types absents à l'ouverture (RP1-30) |
| Notes, grille, relevé, bulletins publiés | codé | fonctionne, borné au groupe ; exports historiques ouverts aux professeurs (RP1-01) |
| Conseil de classe | codé | fonctionne ; portée au groupe absente (RP1-22) |
| Exports LSU / LSL / ONDE / Notanet | codé | fonctionne pour le secrétariat |
| Cahier de textes, travaux, remises | codé | fonctionne (remise de fichier, suivi, lien de pièce) |
| Rencontres parents-professeurs | codé | fonctionne (réservation) ; enseignants de groupe mal ciblés (RP1-27) |
| Formulaires, droit à l'image | codé | fonctionne ; consentements invisibles aux professeurs (RP1-28) |
| Groupes, options, LV2 | partiel | **pas de route d'affectation** (RP1-02) |
| Certificat de scolarité, courriers | — | **absent** (RP1-29) |
| Parents séparés | codé | coordonnées masquées, suspension efficace ; **messagerie et relances partagées** (RP1-11) |
| Facturation, relances | codé | deux modules (RP1-31) |
| Profils accueil, surveillant | — | **absents** (RP1-06) |

---

## 5. Tests ajoutés

| Fichier | Test | Constat | État |
|---|---|---|---|
| `ent-db/src/tests_recette_personas.rs` | `rp1_c01_professeur_n_exporte_jamais` | RP1-01 | vert (corrigé) |
| idem | `rp1_c09_eleve_mineur_ne_declare_pas_son_absence` | RP1-09 | vert (corrigé) |
| idem | `rp1_c05_documents_hors_perimetre_et_titre_restreint` | RP1-05 | vert (corrigé) |
| `ent-api/tests/recette_personas_rf16.rs` | `rp1_rf16_professeur_ne_voit_ni_motif_judiciaire_ni_payeurs` | RP1-21 | vert (corrigé) |
| `ent-api/tests/recette_personas_1.rs` | `rp1_c22_…`, `rp1_c23_…`, `rp1_c24_…`, `rp1_c10_…` | RP1-22, 23, 24, 10 | verts (corrigés) |
| idem | `rp1_c03_…`, `rp1_c04_…`, `rp1_c07_…`, `rp1_c11_…`, `rp1_c20_…`, `rp1_c27_…` | RP1-03, 04, 07, 11, 20, 27 | **`#[ignore = "FAILLE OUVERTE …"]`**, échouent avec `--ignored` (vérifié) |

Lancer les tests des failles ouvertes : `cargo test --locked -p ent-api --features labo --test recette_personas_1 -- --ignored --test-threads=1` (ils doivent échouer tant que la faille existe). Le nom de la base locale doit contenir `test`, `proto` ou `lab` (garde du harnais).

## 5 bis. Validation

En local, Rust 1.99.0, PostgreSQL 16 (CI : 18), base vierge, après tous les correctifs (commit `1626ec5` + documentation) :
- `cargo test --locked --workspace --features ent-api/labo,ent-lab/labo -- --test-threads=1` : **796 réussis, 0 échec, 7 ignorés** (les 6 failles ouvertes de `recette_personas_1` + 1 exemple de documentation déjà ignoré), dont la matrice de séparation, `recette_personas_1` (4 verts), `recette_personas_rf16` (1) et `tests_recette_personas` (3) ; code de sortie 0 (≈ 1 h 40, surtout `ent-db`).
- `cargo test --locked -p ent-api -p ent-db -- --test-threads=1` (sans labo) : **162 réussis, 0 échec, 1 ignoré**.
- Passe `--ignored` de `recette_personas_1` : les 6 tests de failles ouvertes **échouent**, chacun pour la raison attendue (passe faite par le sous-agent avant les correctifs RP1-24 et RP1-10, qui ne touchent pas ces failles).
- `cargo clippy --locked --workspace --all-targets` et la même avec `--features ent-api/labo,ent-lab/labo` : **0 erreur** (105 / 143 avertissements préexistants ; aucun sur les lignes modifiées ni dans les nouveaux fichiers).
- Formatage : `cargo fmt … -p ent-web-ui -p ent-web-api -p ent-web-app -p ent-web-server -p ent-web-session --check` et `cargo fmt … -p ent-types --check` : OK ; nouveaux fichiers de test vérifiés par `rustfmt --edition 2021 --check`. Aucun `cargo fmt --all`.
- Non faits : CI GitHub, `cargo deny` / `audit`, tests du frontend Leptos (frontend non modifié), navigateur.

## 5 ter. Validation des correctifs (branche `ent-recette-1-correctifs`)

PostgreSQL 16 local : `ent-db` (`--features ent-db/labo --lib`) **314 réussis, 0 échec** ; `ent-api` (`--features ent-api/labo`) bibliothèque **97**, `recette_personas_1` **14** (plus aucun `#[ignore]`), `recette_personas_rf16` 1, `referentiel` 4, `separation` **41**, `fichiers_*` 16, `tenant_hote` 8, `span_requete` 1 ; `ent-auth` 33 ; clippy avec et sans labo : 0 erreur ; formatage : OK. Tests existants adaptés (anciens comportements) : matrice `separation.rs` ligne « journal » (la comptabilité n'y a plus accès, RP1-07). Non faits : CI GitHub, `cargo deny` / `audit`, navigateur.

## 6. Questions pour Florian

- **Q-RP1-1** (RP1-11) : messagerie et parents séparés — un fil ouvert par un responsable reste-t-il privé à ce responsable et au personnel ? Les relances de facture vont-elles aux seuls payeurs du foyer ?
- **Q-RP1-2** (RP1-04) : quel « annuaire » des élèves pour un professeur qui ne les a pas (nom et classe seulement ? photo ?) ?
- **Q-RP1-3** (RP1-06) : créer un profil de base « personnel » sans droits propres (accueil, AED, agent, intendance), droits portés entièrement par les rôles ?
- **Q-RP1-4** (RP1-12) : élève majeur — par défaut, ses parents gardent-ils l'accès (opposition possible de l'élève) ou le perdent-ils (accord de l'élève) ?
- **Q-RP1-5** (RP1-15) : le secrétariat peut-il suspendre un lien sur présentation d'une ordonnance ?
- **Q-RP1-6** (RP1-10) : quels documents un élève mineur peut-il signer (charte numérique) et lesquels exigent un responsable (règlement intérieur) ?

## 7. Faits, hypothèses, limites
- **Faits** : chaque preuve ci-dessus a été rejouée sur le serveur local (journal de requêtes de la session de recette) ; les correctifs sont testés comme indiqué en §3.4 et §5.
- **Limites** : base de 10 élèves par établissement (pas de mesure de charge) ; pas de navigateur (écrans Leptos non joués) ; pas de messagerie (activation, préinscription, notifications non jouées) ; pas d'expiration de session réelle (délais trop longs) ; pas de passkey (le labo accepte le mot de passe) ; module santé laissé fermé (sa version ouverte est couverte par la matrice de séparation) ; paie, comptabilité d'organisme, génération d'EDT non codées.
- **Données de labo complétées** (uniquement dans la base locale) : affectation de Harry et Hermione au groupe « 4B-ALL » (aucune route) ; mot de passe de labo posé sur le compte de James Potter (aucune messagerie).
