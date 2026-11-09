# Socle 4 — Le dossier administratif de l'élève

**Date :** 2026-11-06 — **Statut :** conception, **non décidée** (à valider par Florian). Aucun code écrit, rien compilé ni testé pour ce document. Aucun accès à la base ni aux secrets de production ; l'application Next.js a été lue en lecture seule ciblée, sans copie de code.
**Branche :** `ent-socle-administratif-conception`.
**À lire avec :** `CLAUDE.md`, `docs/VISION.md`, `docs/PRODUCT_SPEC.md`, `docs/DECISIONS.md` (D-013, D-014, D-017, D-019, D-020, D-021), `docs/ROLES_ET_DROITS.md`, `docs/SOCLES.md` (§3, socle 4), **`docs/socles/02_ELEVE.md`** et **`docs/socles/03_INSCRIPTIONS.md`** (sur lesquels ce document s'appuie), `docs/SOCLE2_DOMAINES.md` (§3 D2, §6.2 S2-1), `docs/IDENTITE_ET_PROFILS_FAMILLES.md`, `docs/INTEROPERABILITE_NATIONALE.md`, `docs/socles/MIGRATIONS.md`.

**Conventions.** **[F]** fait vérifié dans le dépôt (chemin cité) ; **[F Next]** fait lu dans l'application Next.js (lecture seule) ; **[S]** fait sourcé repris d'un document du dépôt qui cite sa source ; **[H]** hypothèse (mémoire ou déduction) ; **[NV]** règle réglementaire citée de mémoire, **à vérifier sur un texte officiel avant de coder** ; **[R]** recommandation (non décidée). Les identifiants d'actions métier `ADM-xx` sont nouveaux et stables ; la correspondance avec les anciens `DOS-xx` de `SOCLE2_DOMAINES.md` est indiquée. **Plage de migrations : 260–279** (`docs/socles/MIGRATIONS.md`).

**Méthode et limites de ce document.** Les sous-agents d'exploration lancés au début ont été arrêtés par une limite d'usage : toutes les vérifications dans le code ont été faites directement (lecture ciblée des migrations 007, 015, 024–032, 062, 075, 078, 120–123, de `ent-db/src/dossier_personnes.rs`, `foyer_repo.rs`, `champs_chiffres.rs`, `ent-api/src/dossier.rs`, `ent-auth/src/actions.rs`, et de quatre fichiers Next.js). Ce qui n'a pas été relu ligne à ligne est signalé.

**[personas] Passe « personas » du 2026-11-06.** Ajouts marqués **[personas]** : cas C28 à C47 (§4), actions ADM-71 à ADM-102 (§9), règles R-A1 à R-A43 (§17 bis), questions Q-A21 à Q-A27 (§18). Origine de chaque ajout : `docs/socles/PERSONAS_02-04.md`. **Quatre ajouts changent le lot A0 en cours** : fuseau de l'établissement au lieu d'Europe/Paris en dur (R-A38) ; champs réservés du contrat `DeclarationResponsables`, dont l'adresse à protéger jamais écrite en clair (R-A39) ; aucune invitation vers une personne « à ne pas solliciter » (R-A39) ; confirmation du lien au guichet ou en un geste depuis un compte existant (ADM-82, R-A40). Voir `PERSONAS_02-04.md` §0.

Sommaire : §1 Thèse · §2 Évaluation de l'existant · §3 Acteurs et ce qu'ils vivent · §4 Les situations familiales réelles · §5 Frontières · §6 Concepts · §7 Historique · §8 Modèle de données cible · §9 Actions métier · §10 Logiciel ou paramétrage · §11 Échanges avec l'extérieur · §12 Sécurité et données sensibles · §13 Droits · §14 Événements · §15 Écrans clés · §16 Plan de migration · §17 Lots de code · §18 Questions à trancher · §19 Récapitulatif · §20 État du lot A0 · §21 État du lot A0 bis.

---

## 1. Thèse : ce que doit savoir l'établissement autour d'un élève

Le socle 2 dit **qui est l'élève** ; le socle 3, **comment il entre, reste et sort**. Le socle 4 dit **qui répond de lui, où il vit, qui joindre, à qui le confier, qui paie, et à qui l'établissement doit écrire**. C'est la partie la plus humaine du logiciel : derrière chaque ligne il y a une séparation, un remariage, un placement, un deuil, une violence parfois. C'est aussi celle où une erreur coûte le plus cher : un bulletin envoyé à l'adresse cachée d'une mère protégée, un enfant remis à un père qui n'en a pas le droit, un SMS d'absence envoyé au parent qui n'a pas l'enfant cette semaine, une facture adressée à un grand-parent décédé.

Neuf principes [R] :

1. **On enregistre des faits, pas des étiquettes.** Pas de champ « situation familiale : divorcés ». On enregistre qui vit où (résidence, avec ses périodes et son rythme), qui exerce l'autorité parentale (avec la pièce quand elle est exclusive ou retirée), qui paie quoi, qui peut venir chercher l'enfant, qui ne le peut pas. Les étiquettes (« garde alternée », « parents séparés ») se **déduisent** et s'affichent, elles ne se saisissent pas.
2. **Quatre questions distinctes, quatre réponses distinctes.** *Qui décide* (autorité parentale, tutelle, délégation), *qui héberge* (résidence), *qui paie* (payeurs), *qui peut-on joindre ou à qui confier l'enfant* (contacts, personnes autorisées, interdictions). Les logiciels mélangent ces axes sur une seule ligne « responsable 1 / responsable 2 » ; l'existant Rust aussi, en partie (payeur et droits fins portés par le lien de responsabilité, §2).
3. **Une personne, plusieurs rôles, une seule fiche.** La grand-mère peut être contact d'urgence de deux petits-enfants, personne autorisée à les chercher, et payeuse de la cantine de l'un. Elle existe **une fois** (`ent_personne`, D-020), ses coordonnées aussi.
4. **Le foyer est un lieu de vie, pas un compte.** Un foyer = des personnes qui vivent ensemble à une adresse, à une période. L'existant a fait du foyer le compte de facturation (factures, mandats SEPA, quittances, rappels, quotient : tous rattachés à `ent_foyer` [F]) ; on garde ces tables, mais on sépare les notions (§6.2).
5. **Chaque envoi a des destinataires calculés et expliqués.** « Qui recevra ce bulletin, cette facture, ce SMS d'absence, cette convocation ? » a une réponse **calculée par le socle 4** à partir des faits (autorité, résidence du jour, payeurs, droits, interdictions, préférences), affichée avant l'envoi et rejouable après (« pourquoi ai-je reçu ce message ? », socle 8). Aujourd'hui les destinataires sont les **comptes** liés, pas les personnes [F, §2.2 n° 1].
6. **Tout se modifie, rien ne s'efface, et la famille participe.** Coordonnées, adresses, liens, autorité, résidence, payeurs : tout est daté et versionné. La famille met à jour ses propres coordonnées (directement ou par proposition selon le champ), signale un changement de situation avec sa pièce ; l'établissement valide ce qui doit l'être. Une fois par an, une **fiche de rentrée** fait vérifier le tout en cinq minutes, sans ressaisie.
7. **Le dossier se protège lui-même.** Adresse protégée, interdiction de remise, enfant placé, parent privé d'autorité : le logiciel **empêche** les fuites (documents, exports, listes, envois, affichage aux autres parents) au lieu de compter sur la vigilance du secrétariat. Les cas graves sont visibles là où ils servent (accueil, sortie, appel) et nulle part ailleurs.
8. **Le logiciel ne tranche pas le droit de la famille.** Il ne qualifie pas un acte (usuel ou non), ne décide pas d'un conflit entre parents, n'interprète pas un jugement : il demande la pièce, applique la règle **choisie** par l'établissement, bloque en cas de désaccord déclaré, et trace (même posture que `03_INSCRIPTIONS.md` §4.4).
9. **Minimiser par défaut.** On ne collecte ni la situation matrimoniale, ni la nationalité ou la date de naissance des parents, ni la profession si aucun flux ne l'exige vraiment. Chaque donnée a une raison, un moment de collecte, un groupe de lecture et une durée.

Ce qui fera la différence au quotidien [H — comparaison de mémoire avec les logiciels du marché, non vérifiée] : la **garde alternée native** (le parent qui a l'enfant cette semaine reçoit l'appel ou le SMS) ; le **déménagement en un geste** (une adresse change pour tous les membres et tous les enfants du foyer, avec date d'effet et conséquences affichées) ; la **séparation en un assistant** (deux foyers, deux comptes, deux adresses, deux répartitions, sans perdre l'historique) ; un **écran « qui peut venir chercher cet enfant aujourd'hui »** à l'accueil et sur le téléphone du surveillant ; des **documents nominatifs par destinataire** (chaque parent reçoit son bulletin à son adresse, jamais celle de l'autre) ; une **fiche de rentrée** pré-remplie, multi-enfants, sur téléphone ; une **qualité des données familles** qui dit ce qui manque avant que ça manque (aucune autorité active, parents séparés sans e-mail, adresse non normalisée, e-mail qui rebondit, personne interdite aussi autorisée).

---

## 2. Évaluation franche de l'existant

Sources lues : migrations `007_ent_foyer_liens.sql`, `015_dossier_eleve.sql`, `024_facturation.sql`, `027`, `030_sepa.sql`, `032`, `062_sortie_autorisation.sql`, `075_quotient_tva.sql`, `078_inscription_iban.sql`, `120_personne.sql` à `123_personne_fusion.sql` (en entier pour 007, 120–123) ; `ent-db/src/dossier_personnes.rs` (lu par extraits : normalisation, liens, modification de personne, publication) ; `ent-db/src/foyer_repo.rs` (lecture du lien actif, e-mails et téléphones des responsables) ; `ent-db/src/champs_chiffres.rs` ; `ent-api/src/dossier.rs` (table des routes) ; `ent-auth/src/actions.rs` (actions) ; `docs/SOCLE2_DOMAINES.md` §6.2 (compte rendu S2-1). Next.js : `db/schema.ts` (foyer, responsables, lien élève–foyer), `db/schema-fiches-dialogue.ts`, `app/lib/nomenclature-import/siecle-responsables-parse.ts` et `siecle-people-import.ts`.

### 2.1 Ce qui tient (à garder)

| Élément | Pourquoi c'est bon | Preuve [F] |
|---|---|---|
| **Personne distincte du compte** (`ent_personne`), propre à l'établissement, compte facultatif, une personne par compte et par établissement | Parent sans e-mail, contact, élève majeur payeur : tout tient. Applique D-013, D-014, D-020 | `120_personne.sql:13-46` |
| Déclencheurs de cohérence lien ↔ personne ↔ compte (personne d'un autre établissement refusée, compte recopié sur les liens) | Le code ancien qui lit `user_id` reste juste ; invariant tenu par la base | `120_personne.sql:88-146` |
| **Lien actif lu en un point unique** (`ent_eleve_lien_actif` : non suspendu, dans ses dates, personne non fusionnée) ; ~60 contrôles famille l'utilisent | Couper un accès = un fait, effet immédiat partout | `121_lien_autorite.sql:37-50` ; `foyer_repo.rs:541-563` ; `SOCLE2_DOMAINES.md` §6.2 |
| Suspension motivée (motif 3–500 caractères obligatoire, référence), motif **ni** dans l'événement **ni** dans le journal | Bonne minimisation d'une donnée judiciaire | `121:26-33` ; §6.2 S2-1 |
| Commandes du domaine **en une transaction** : fait + journal + événement outbox ; charges utiles = **noms des champs**, jamais les valeurs | Exactement la règle que `02_ELEVE.md` veut généraliser | `dossier_personnes.rs:1109-1185` (`modifier_personne_for_actor`), `:601-622` (`publier_tx`) |
| Normalisation des saisies (e-mail en minuscules et contrôlé, téléphone sur un alphabet restreint, longueurs) | Base de validation à étendre | `dossier_personnes.rs:109-160` |
| Contact d'urgence = une **personne** (pas une chaîne de caractères), avec statut `propose` quand un responsable le saisit | Bon principe « la famille propose, l'établissement valide » | `122_contact_urgence.sql` |
| **Fusion de personnes réversible 30 jours**, instantané JSON, jamais deux comptes différents fusionnés | Patron juste (repris par `02_ELEVE.md` pour les élèves) | `123_personne_fusion.sql` ; `dossier_personnes.rs:1943-2326` |
| Familles minimisées : un responsable ne voit pas les coordonnées ni les parts des autres, ni les liens inactifs, ni les motifs ; ne parcourt jamais l'annuaire des personnes | Indispensable pour les situations conflictuelles | `SOCLE2_DOMAINES.md` §6.2 |
| Alertes de fiche : parents séparés sans e-mail, payeur sans foyer ou suspendu, aucune autorité active | Début de la « qualité des données familles » | idem |
| Invariant des parts payeurs (somme = 100 %) calculé sur **tous** les liens | Règle juste | `dossier_personnes.rs:626-641` |
| **IBAN chiffrés** : `ent_fac_sepa_mandat` et `ent_foyer_iban_histo` sont déclarés dans les champs chiffrés (`iban`, `bic`, `titulaire`) | Répond à la question Q-I16 de `03_INSCRIPTIONS.md` (à confirmer par un test de lecture brute) | `champs_chiffres.rs:28-29` |
| Tests S2-1 : 8 scénarios Postgres (parent sans compte, parents séparés, majeur, suspension sur 12 points d'accès, matrice de droits, isolation, fusion), tests HTTP, tests d'écran | Filet de sécurité réel pour la refonte | `tests_dossier_personnes.rs`, `ent-api/src/tests_dossier.rs` (cités par §6.2, non relus ici) |

### 2.2 Ce qui est à revoir

Classé par gravité. Les points 1 à 4 sont des **défauts de comportement** (lot A0) ; les suivants des limites de modèle.

| # | Problème | Conséquence | Preuve [F] | Correction [R] |
|---|---|---|---|---|
| 1 | **Les envois aux familles lisent les comptes, pas les personnes.** `emails_parents_eleve` et `telephones_parents_eleve` joignent `ent_eleve_lien_actif` à `ent_user` et prennent `u.email`, `u.telephone` | Un parent **sans compte** (cas que S2-1 a justement rendu possible) ne reçoit **aucun** message, ni e-mail ni SMS d'absence ; un parent qui corrige son e-mail sur sa fiche (`ent_personne.email`) continue de recevoir à l'ancienne adresse du compte ; deux sources d'e-mail divergent sans alerte | `foyer_repo.rs:565-600` | Port unique `destinataires(eleve, nature_envoi, date)` du socle 4, lisant les **coordonnées de la personne** (§8.4, §9 ADM-60) ; le compte n'est qu'une coordonnée de plus (in-app, push) |
| 2 | **Coordonnées écrasées sans historique.** `UPDATE ent_personne SET … email, telephone, adresse` ; le journal ne garde que les **noms** des champs | Impossible de répondre « à quelle adresse a-t-on envoyé la facture de mars ? », de prouver une notification, de revenir en arrière après une erreur | `dossier_personnes.rs:1141-1157` | Coordonnées et adresses datées (§8.4, §8.5) |
| 3 | **Adresse en texte libre** (500 caractères) sur la personne ; le foyer n'a **aucune** adresse | Pas de code postal ni de commune exploitables : la liste des élèves par **commune de résidence** que doit produire le socle 3 (`03_INSCRIPTIONS.md` §4.5, INS-45) est impossible ; pas de publipostage propre ; pas de transport ni de tarif par commune. **Régression** par rapport à Next.js, dont le foyer a adresse, code postal et ville [F Next `db/schema.ts:493-509`] | `120_personne.sql:22` ; `007:4-9` | Adresse structurée et normalisée (§8.5) |
| 4 | **Dates des liens calculées en UTC** (`now() AT TIME ZONE 'UTC'`) | Un lien qui se termine le 31 août reste actif jusqu'à 2 h du matin heure de Paris le 1ᵉʳ septembre (été), ou s'ouvre deux heures trop tard | `121_lien_autorite.sql:46-48` | Dates métier en `Europe/Paris`, partout dans le socle |
| 5 | **Le foyer n'est qu'un libellé** ; l'appartenance passe par `ent_eleve_lien.foyer_id`, **un par lien** (donc par enfant) | Un même parent peut être rangé dans deux foyers différents pour ses deux enfants ; une personne sans lien (grand-mère contact) n'a pas de foyer ; le foyer ne connaît pas ses membres adultes ; impossible de déménager « un foyer » | `007:4-31` | Membres et adresses du foyer (§8.5) |
| 6 | **Le foyer est en réalité le compte de facturation** : `ent_fac_facture`, `ent_fac_quittance`, `ent_fac_rappel`, `ent_fac_sepa_mandat`, `ent_foyer_iban_histo`, `ent_foyer_quotient` référencent tous `ent_foyer` (clé simple) | Toute refonte du foyer touche la facturation ; une séparation « déplace » des factures ; le quotient familial (notion fiscale) est accroché au même objet que le lieu de vie | migrations `024:6`, `027:6`, `030:14,39`, `032:6`, `075:6`, `078:24` | On **garde** `ent_foyer.id` comme débiteur pour ces tables (compatibilité), on ajoute le foyer « lieu de vie » à côté sans les confondre (§6.2, Q-A1) |
| 7 | **Payeur porté par le lien de responsabilité** (`est_payeur`, `part_paiement` sur `ent_eleve_lien`) | Seul un responsable peut payer : grand-parent payeur, employeur, service de l'aide sociale à l'enfance, organisme : impossibles sans en faire un « responsable » avec des droits ; répartition **par nature de frais** impossible (« la cantine chez la mère, la scolarité chez le père ») ; pas de dates | `015_dossier_eleve.sql:12-22` | Payeurs séparés, datés, par catégorie de frais (§8.9) |
| 8 | **Nature du lien en texte libre** (1 à 40 caractères, aucun vocabulaire) | Pas d'export vers les codes de parenté nationaux (SIECLE `CODE_PARENTE` [F Next]) ; pas de droits par défaut selon la nature ; statistiques impossibles ; fautes de frappe | `dossier_personnes.rs:205-222` | Vocabulaire du logiciel, libellés paramétrables (§8.6) |
| 9 | **Autorité parentale = un booléen** présumé vrai pour tous les liens repris ; pas d'« exercice exclusif », pas de pièce, pas d'historique ; `garde` (`exclusive`, `alternee`, `aucune`) sur le lien, sans calendrier ni lieu | Un retrait d'autorité ou un jugement de résidence ne laisse aucune trace datée ni justificatif ; la garde alternée n'a aucun effet (envois, appel du soir) | `121:9-15` | Qualité juridique versionnée avec pièce (§8.6) ; résidence avec rythme (§8.7) |
| 10 | **Droits fins figés en colonnes** (`peut_voir_notes`, `peut_voir_vs`, `peut_justifier_absence`, `peut_autoriser_sorties`) **cochés à vrai par défaut** à la création | Chaque nouveau droit famille (signer, recevoir les factures, payer en ligne, répondre aux formulaires…) = une migration ; une erreur de saisie donne tout | `007:20-23` ; `015:12-13` ; `dossier_personnes.rs:1367-1389` | Droits du lien en lignes, catalogue du logiciel, valeurs par défaut **par nature de lien paramétrées** (§8.6) |
| 11 | **Contact d'urgence et personne autorisée à récupérer confondus** (`peut_recuperer` sur le contact d'urgence) ; relation en texte libre ; aucune **interdiction de remise** ; pas de jours ni de périodes ; pas d'exigence de pièce d'identité | L'accueil ne peut pas savoir « qui peut chercher Léo le mercredi » ni « à qui ne jamais le remettre » ; un oncle autorisé pour les vacances de février le reste à vie | `122_contact_urgence.sql` | Remise et interdictions séparées, datées (§8.8) |
| 12 | **Clés étrangères simples** (`personne_id REFERENCES ent_personne (id)`, `eleve_id` sans clé sur `ent_eleve_lien`) ; aucune RLS sur ces tables | Même faiblesse que celle relevée par `02_ELEVE.md` §2.2 ; isolation par filtres applicatifs seulement | `007`, `120`, `122` ; `02_ELEVE.md` §10.3 | Clés composées `(etablissement_id, x_id)` et RLS (§12.1) |
| 13 | **Reprise des noms** : la personne créée depuis un compte reçoit le nom affiché du compte dans `nom` (« Marie Dupont » en un seul champ) | Nom et prénom mélangés pour toutes les personnes reprises ; publipostage et exports faux | `120:59-72` (choix assumé et documenté) | Écran de qualité « noms à séparer » avec proposition de découpage, validée humainement (§15) |
| 14 | **Pas de mise à jour par la famille** (DOS-10), **pas de séparation de foyer** (DOS-16), **pas d'invitation depuis une personne sans compte** (DOS-09) | Le secrétariat ressaisit ce que les familles pourraient faire elles-mêmes | `SOCLE2_DOMAINES.md` §6.2 « Reste à faire » | ADM-30 à ADM-35, ADM-20, ADM-08 |
| 15 | **Aucun consommateur** de `personne.coordonnees_modifiees` | Une famille qui change d'adresse n'en prévient personne (facturation, transport, mairie) | recherche dans `reactions.rs`, `outbox.rs` : aucun résultat | Consommateurs déclarés (§14.2) |
| 16 | Écran : ajout d'un responsable **en deux appels** (créer la personne puis le lien) | Personne orpheline en cas d'erreur | `02_ELEVE.md` §2.2 (`modules/dossier/actions.rs:71-74`) | Commande unique `ajouter_responsable` (§9) |
| 17 | **Écritures directes du socle 3 (branche `ent-integration-lot5`)** dans `ent_personne`, `ent_foyer`, `ent_eleve_lien`, rapprochement silencieux par e-mail, un nouveau foyer à chaque validation | Accès donné à un tiers sans son accord ; fratries dédoublées | `03_INSCRIPTIONS.md` §2.2 n° 1–3 | Ports `correspondances` et `integrer_declaration` du socle 4 (§5.3), livrés en premier (lot A0) |

### 2.3 Ce qui manque

- **Adresses structurées et normalisées**, communes au code officiel géographique (COG INSEE), adresse protégée.
- **Membres d'un foyer** et **résidence de l'élève** dans un ou plusieurs foyers, avec périodes et rythme (garde alternée par semaines, jours fixes, vacances).
- **Coordonnées multiples par personne** (mobile, fixe, professionnel, e-mails), avec usages (urgence, notifications, facturation), vérification, rebonds, horaires de joignabilité.
- **Qualité juridique fine** : titulaire de l'autorité parentale (conjointe, exclusive, retirée), tuteur, délégataire, service gardien (aide sociale à l'enfance), tiers digne de confiance, assistant familial, mesure de protection d'un majeur (le socle 2 porte la mesure, le socle 4 désigne la personne).
- **Interdictions** (ne pas remettre l'enfant à…, ne pas communiquer d'information à…), personnes autorisées par jour et période.
- **Payeurs** non responsables et **organismes** (service départemental, employeur, collectivité), répartition par catégorie de frais.
- **Préférences** (langue, canal, papier, accessibilité), **consentements** propres au dossier administratif (diffusion des coordonnées aux associations de parents, transport).
- **Propositions de la famille** et **fiche de rentrée annuelle**.
- **Identifiants officiels des responsables** (identifiant de personne SIECLE, clé de jointure ÉduConnect, ancien logiciel).
- **Assurance scolaire** de l'élève (attestation annuelle).
- **Destinataires calculés** pour chaque nature d'envoi.
- **Situations de vie** : décès d'un responsable, séparation, recomposition, placement, majorité, déménagement à l'étranger.

### 2.4 Ce que Next.js fait et qu'il ne faut pas perdre (lecture seule)

| Fonction [F Next] | Où | Reprise [R] |
|---|---|---|
| Foyer avec **adresse structurée** (adresse, code postal, ville) et `payeur_est_foyer` | `db/schema.ts:493-509` | Adresse structurée (§8.5) ; le payeur reste une personne ou un organisme (§8.9) |
| Responsable de foyer avec **quatre rôles distincts** : autorité parentale, contact d'urgence, payeur, **`peut_recuperer`** (commentaire : « distinct de l'autorité, du payeur et de l'urgence »), rang | `db/schema.ts:511-539` | Bonne intuition, reprise et généralisée (§6.1) ; le rang devient un **ordre d'appel** par usage |
| Lien élève ↔ foyer avec `relation` (`principal`…) | `db/schema.ts:541-563` | Résidence de l'élève (§8.7) |
| Import SIECLE des responsables : entités `PERSONNE` (identifiant, nom, prénom, téléphones domicile / portable / personnel, e-mail, `ADRESSE_ID`), `ADRESSE` (lignes, code postal, ville, libellé postal), `RESPONSABLE_ELEVE` (`NIVEAU_RESPONSABILITE`, `CODE_PARENTE`, `PAIE_FRAIS_SCOLAIRES` ou `RESP_FINANCIER`, `A_CONTACTER_EN_PRIORITE`, `HEBERGE_ELEVE`) | `app/lib/nomenclature-import/siecle-responsables-parse.ts:60-140` | Liste de champs **précieuse** pour le modèle et l'IOP : l'adresse est une entité partagée, l'hébergement est explicite, la priorité de contact aussi. Signification exacte des codes (`NIVEAU_RESPONSABILITE` 1/2/3…, codes de parenté) **[NV]**. **Aucune copie de code** |
| Foyer **retrouvé par son libellé** dérivé de l'adresse (« Foyer 12 rue…, Ville ») à l'import | `siecle-people-import.ts:74-76, 207-230` | **Défaut à ne pas reproduire** : un libellé n'est pas une clé ; rapprochement par identifiant d'adresse source puis par adresse normalisée, avec choix humain |
| Fiches de dialogue avec états `en_attente_accord_parent2`, `en_conflit`, résolution par la direction | `db/schema-fiches-dialogue.ts:37-38, 181` | Même besoin que l'accord des deux parents du socle 3 (`03_INSCRIPTIONS.md` §4.4) : le socle 4 fournit les **titulaires** à consulter (port `titulaires_autorite`), la mécanique d'accord reste à chaque démarche |
| Coordonnées parents dupliquées sur l'élève (`parent1_email`, `parent2_email`, `parent1_phone`…) | `db/schema.ts:357-361` | **Non repris** (déjà acté par `02_ELEVE.md` §2.4) |
| Messagerie famille adressée au **foyer** (`famille_thread.foyer_id`) | `db/schema-famille-messaging.ts:28` | **Non repris** tel quel : une conversation s'adresse à des **personnes** (parents séparés : deux fils distincts) ; le foyer n'est qu'un raccourci de sélection (socle 16) |

---

## 3. Acteurs et ce qu'ils vivent

| Acteur | Ce qu'il vit aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|
| **Secrétariat de scolarité** | Ressaisit chaque rentrée les fiches papier ; reçoit des changements d'adresse par mail, téléphone, sur un coin de bulletin ; doit deviner qui est « responsable 1 » ; tient à jour deux logiciels (gestion et SIECLE) | Envoyer un document à la mauvaise adresse ; ne pas avoir vu un jugement ; l'erreur qui remonte dans SIECLE | Une file « changements à valider » triée ; des changements qui arrivent **avec** leur pièce ; un déménagement saisi une fois pour toute la fratrie ; zéro ressaisie à la rentrée | quotidienne, pic en septembre |
| **Accueil, loge** | Un parent ou un inconnu se présente pour chercher un enfant ; le classeur papier des autorisations n'est pas à jour | Remettre un enfant à quelqu'un qui n'en a pas le droit ; refuser à tort un parent légitime devant l'enfant | Taper trois lettres, voir la photo de l'enfant, la liste des personnes autorisées **aujourd'hui** (et leur photo si l'établissement l'a choisi), une alerte rouge s'il existe une interdiction, et le bouton « appeler le parent » | plusieurs fois par jour, pic à 16 h 30 et le mercredi |
| **Vie scolaire (CPE, AED)** | Appelle les parents pour une absence, un malaise, une exclusion de cours ; ne sait pas lequel appeler en garde alternée | Joindre le mauvais parent ; ne joindre personne lors d'un incident | « Appeler » propose dans l'ordre le parent qui a l'enfant cette semaine, puis l'autre, puis les contacts d'urgence, avec les horaires où chacun est joignable | plusieurs fois par jour |
| **Infirmerie** | Doit faire venir un parent ou appeler les secours et prévenir la famille | Ne joindre personne, ou remettre l'enfant malade à une personne non autorisée | Contacts d'urgence fiables, personnes autorisées, en un écran | quotidienne |
| **Direction** | Reçoit les conflits parentaux (« je ne veux pas que son père soit informé »), les jugements, les signalements ; arbitre | Prendre parti à tort ; violer une décision de justice ; une fuite d'adresse protégée | Un dossier qui applique les décisions de justice sans interprétation, qui bloque ce qui doit l'être, et qui trace tout ; une vue des situations sensibles réservée | hebdomadaire, crises ponctuelles |
| **Comptabilité** | Factures aux mauvais payeurs après une séparation ; grand-parent qui paie sans être dans le logiciel ; prélèvements à réaffecter | L'impayé né d'une erreur d'adressage ; le mandat SEPA d'un parent qui ne paie plus | Payeurs datés, répartition par type de frais, un changement de payeur qui dit quelles factures futures il concerne | hebdomadaire, pic à la rentrée |
| **Professeur principal** | Doit appeler les familles, convoquer aux rencontres parents-professeurs | Écrire au parent qui ne doit pas être joint | Les moyens de contact **utiles** (sans l'adresse si elle est protégée), qui reçoit quoi | hebdomadaire |
| **Parent (titulaire de l'autorité)** | Remplit la même fiche papier tous les ans, pour chaque enfant ; ne sait pas ce que l'école connaît de lui | Que l'autre parent voie son adresse ; ne pas être informé de la scolarité ; recevoir des factures qu'il ne doit pas | Voir ce que l'école sait, corriger en un geste, valider la fiche de rentrée de tous ses enfants en cinq minutes, choisir comment être prévenu | ponctuelle, pic à la rentrée |
| **Parent non gardien, ou séparé en conflit** | Est oublié des envois, ou au contraire reçoit tout en double avec l'autre | Être écarté sans base légale ; voir l'autre parent écrire à l'école à sa place | Recevoir ses propres envois, à son adresse, et pouvoir le prouver | ponctuelle |
| **Beau-parent** | N'a aucune existence légale mais vit avec l'enfant, vient le chercher, parfois paie | Être bloqué au portail ; recevoir des informations auxquelles il n'a pas droit sans le savoir | Être reconnu pour ce qu'il fait (chercher l'enfant, payer, être contact) sans droits qu'il n'a pas | quotidienne |
| **Grands-parents, voisins, nourrice** | Contacts d'urgence et personnes qui viennent chercher l'enfant | Être refusés au portail | Être sur la liste, avec les jours prévus | hebdomadaire |
| **Service de l'aide sociale à l'enfance (ASE), référent, assistant familial** | Doit faire reconnaître qui décide quoi pour un enfant placé ; l'adresse de la famille d'accueil peut devoir rester inconnue des parents [H, NV] | Une fuite de l'adresse d'accueil ; un parent qui récupère l'enfant malgré une décision | Un dossier qui sait que le service gardien décide, que l'assistant familial gère le quotidien, que les parents gardent les droits qu'ils ont, et que l'adresse d'accueil est protégée | rare mais grave |
| **Tuteur, délégataire, tiers digne de confiance** | Doivent produire leur jugement à chaque démarche | Ne pas être reconnus | Une pièce déposée une fois, reconnue partout | rare |
| **Élève mineur** | Rien à faire ici, mais ses coordonnées propres sont au socle 2 | — | — | — |
| **Élève majeur** | Devient le premier interlocuteur ; ses parents restent souvent payeurs | Que ses parents voient tout sans son accord ; ou qu'on coupe brutalement ce qu'il souhaite garder | Choisir ce que ses parents continuent de recevoir ; désigner un payeur | rare (lycée, post-bac) |
| **Associations de parents d'élèves, élus parents** | Ont besoin des coordonnées des parents qui l'acceptent | — | Une liste propre, avec consentement daté [H, NV] | annuelle |
| **Mairie, transport, collectivité** | Listes par commune (socle 3), transport scolaire | — | Données justes et minimales | annuelle |
| **Domaines internes** | Facturation, notifications, messagerie, vie scolaire, accueil, élections, sorties attendent : destinataires, payeurs, autorisés, commune, résidence du jour | Des données incohérentes ou périmées | Des ports de lecture fiables et des événements sans données personnelles | permanent |

---

## 4. Les situations familiales réelles (catalogue de cas limites)

Chaque cas doit pouvoir être saisi **sans contournement** (pas de « mettre le beau-père en responsable 2 faute de mieux ») et produire les bons effets. Les règles juridiques citées étaient de mémoire ; celles marquées **[V]** ont été vérifiées le 2026-11-09 sur le texte consolidé (`docs/VERIF_REGLEMENTAIRE.md`) ; les autres restent **[NV]** (Q-A16).

| # | Situation | Ce que le dossier enregistre | Effets attendus |
|---|---|---|---|
| C1 | Parents ensemble, même adresse | 1 foyer, 2 membres titulaires de l'autorité, résidence principale de l'enfant dans ce foyer | Un courrier papier au foyer (si choisi), notifications à chaque parent |
| C2 | Parents séparés, résidence chez la mère, autorité conjointe (le cas le plus courant [H]) | 2 foyers ; 2 liens titulaires conjoints ; résidence principale foyer A ; foyer B : « droit de visite » optionnel, sans effet sur l'école | Les deux parents reçoivent bulletins et informations importantes, chacun à son adresse ; appels d'absence d'abord au foyer de résidence [R] ; e-mail obligatoire pour chacun (D-014) |
| C3 | Garde alternée par semaines (paires / impaires) | 2 foyers ; résidence alternée avec rythme et date de référence | « Foyer du jour » calculé : SMS d'absence et appel prioritaire au parent qui a l'enfant ; transport (deux arrêts) ; personnes autorisées par foyer |
| C4 | Alternance non hebdomadaire (lundi-mardi / mercredi-jeudi / week-ends alternés, 2-2-5-5) | Rythme par jours avec cycle et date de référence | Idem ; aperçu calendrier à la saisie pour vérifier |
| C5 | Famille recomposée : la mère, son conjoint et leurs enfants communs + l'enfant d'une première union | 1 foyer, 2 adultes (la mère titulaire pour tous, le conjoint titulaire pour ses enfants, **beau-parent sans autorité** pour l'aîné) | Le beau-parent peut être contact, autorisé à récupérer, payeur, sans droits d'accès sauf choix explicite **limité** (§8.6, Q-A6) |
| C6 | Autorité parentale exclusive (jugement) | Lien de l'autre parent : `autorite = non`, motif « exercice exclusif à l'autre parent », pièce | Le parent sans autorité garde, si le jugement ne dit rien d'autre, un droit d'être **informé** des choix importants **[V]** (C. civ. 373-2-1 : il « conserve le droit et le devoir de surveiller l'entretien et l'éducation de l'enfant. Il doit être informé des choix importants » ; le guide eduscol garantit l'information aux deux parents « qu'ils exercent ou non l'autorité parentale », VERIF R11) : paramétrage par défaut « reçoit les bulletins » ; la direction peut restreindre au vu de la pièce |
| C7 | Retrait de l'autorité parentale, interdiction de contact, ordonnance de protection | Lien suspendu (existant) ou `autorite = retiree` avec pièce ; **interdiction** de remise et de communication ; éventuellement adresse protégée de l'autre parent | Aucun accès, aucun envoi, alerte à l'accueil ; l'adresse protégée n'apparaît sur aucun document ni écran hors droit dédié **[V]** (C. civ. 515-11, 6° et 6° bis : le juge peut autoriser la partie demanderesse « à dissimuler son domicile ou sa résidence », VERIF R14) |
| C8 | Parent décédé | Fait « décès » daté sur la personne (avec pièce selon paramétrage) | Tous ses liens se ferment à la date ; plus **aucun** envoi ni relance (et suppression des relances programmées) ; facturation prévenue ; message humain uniquement ; l'autre parent devient seul titulaire **de fait** [NV] |
| C9 | Enfant confié à l'ASE (placement) | Lien « service gardien » (organisme + référent), lien « assistant familial » (lieu d'accueil, résidence), parents : titulaires mais avec droits **paramétrés au vu de la décision** | Le service et l'assistant familial gèrent le quotidien (absences, sorties) ; adresse d'accueil protégée vis-à-vis des parents si la décision le prévoit **[V]** (C. civ. 375-7 : les parents « continuent à exercer tous les attributs de l'autorité parentale qui ne sont pas inconciliables avec cette mesure », VERIF R12) ; payeur : le service si convenu |
| C10 | Tutelle d'un mineur (parents décédés ou privés) | Lien « tuteur » avec jugement ; éventuellement conseil de famille (hors logiciel) | Le tuteur a les droits d'un titulaire |
| C11 | Délégation d'autorité parentale, tiers digne de confiance | Lien de la nature correspondante, avec pièce et dates | Droits selon la décision ; parents conservant ou non une part [NV : art. 377 et 375-3 C. civ.] |
| C12 | Élève majeur | Le socle 2 publie `eleve.majeur` ; les liens de ses parents passent `autorite = non` (fin légale), leurs droits suivent la règle Q10 | Proposition à l'élève : « ce que vos parents continuent de recevoir » ; parents restent payeurs s'ils le sont (§8.9) |
| C13 | Majeur sous tutelle ou curatelle | Le socle 2 porte la mesure ; le socle 4 lie le tuteur ou curateur avec sa pièce | Droits du tuteur selon la mesure [NV] |
| C14 | Mineur émancipé | Le socle 2 porte l'émancipation | Comme un majeur ; ses parents n'ont plus l'autorité parentale **[V]** (C. civ. 413-6, 413-7) |
| C15 | Parent sans e-mail, parent ne lisant pas le français, parent malvoyant | Préférences de la personne : courrier papier, langue, gros caractères, SMS seulement | Envois adaptés ; le parent sans e-mail est toujours destinataire (papier, SMS), jamais oublié |
| C16 | Parent à l'étranger | Adresse étrangère (pays COG, lignes libres) ; téléphone international | Format d'adresse international ; décalage horaire pour les appels [R] |
| C17 | Grands-parents qui élèvent l'enfant sans décision | Liens « grand-parent » **sans** autorité ; alerte « aucune autorité active » | La qualité des données le signale ; le secrétariat demande la pièce (délégation, tutelle) |
| C18 | Fratrie dans l'établissement, avec pères différents | Un foyer (mère + enfants) ; chaque enfant ses propres liens | Une seule fiche de rentrée pour la mère, deux pour les pères selon leurs enfants |
| C19 | Même parent dans deux établissements ScolIA | Deux personnes sans lien visible (D-013) | Proposition « mettre aussi à jour chez B » **à l'initiative du parent**, jamais de l'établissement |
| C20 | Changement de nom d'un parent (mariage, divorce) | Version de l'identité de la personne (nature `usage` ou `changement`) | Courriers au nouveau nom ; historique conservé |
| C21 | Déménagement d'un foyer | Nouvelle adresse du foyer à date d'effet | Tous les membres et enfants résidents suivent ; impacts affichés (transport, commune pour les listes mairie, tarif de cantine par commune, secteur dans le public [H]) |
| C22 | Séparation en cours d'année | Assistant : le foyer se scinde en deux, chaque adulte rejoint son foyer, résidence de l'enfant redéfinie, payeurs revus, second e-mail exigé (D-014) | Aucune donnée perdue ; factures passées inchangées, factures futures selon nouveaux payeurs |
| C23 | Désaccord entre parents déclaré à l'établissement | Fait « désaccord déclaré » sur l'élève (objet, date, déclarant), sans motif rédigé visible des autres | Les démarches qui exigent l'accord des titulaires (socle 3, sorties, formulaires) se **bloquent** ; message neutre ; levée avec pièce (`03_INSCRIPTIONS.md` §4.4) |
| C24 | Personne autorisée ponctuellement (« ma voisine vient le chercher vendredi ») | Autorisation de remise **datée** (un jour, une plage), saisie par un titulaire depuis l'appli, sans validation si le paramétrage le permet | Visible à l'accueil ce jour-là seulement ; disparaît ensuite |
| C25 | Personne interdite de remise mais titulaire d'un droit de visite | Interdiction avec plage (« hors week-ends prévus par le jugement ») — **trop fin** pour un logiciel : interdiction simple + note « voir pièce » [R] | L'accueil voit l'alerte et appelle la direction ; jamais d'interprétation automatique d'un jugement |
| C26 | Responsable mineur (parent adolescent) | Personne titulaire ; **pas** de règle spéciale dans le logiciel [NV : l'autorité d'un parent mineur] | Signalé à la direction pour vérification |
| C27 | Enfant sans aucun responsable connu (mineur non accompagné, arrivée en urgence) | Alerte bloquante d'information « aucune autorité active » ; contact de l'organisme | Procédure humaine ; le dossier n'empêche pas la scolarisation |
| C28 [personas] | Garde alternée, échange ponctuel (« cette semaine il est chez moi ») | Exception de résidence datée, déclarée par un titulaire (ADM-78) | Foyer du jour corrigé pour la période ; l'autre titulaire informé sans détail ; en cas de contestation, appel des deux |
| C29 [personas] | Vacances partagées par moitié ou une année sur deux | Partage des vacances dans le rythme (ADM-79) | Sorties, internat, stages pendant les congés : bon destinataire |
| C30 [personas] | Famille en hébergement d'urgence, domiciliée par une association | Adresse de domiciliation distincte du lieu de vie ; foyer `sans_domicile_stable` (ADM-77) | Courrier à la domiciliation ; rien de bloquant ; commune des listes mairie selon la règle vérifiée [NV] |
| C31 [personas] | Parent qui ne parle pas français ; l'enfant traduit d'habitude | Préférence de langue, besoin d'interprète ; relais de communication éventuel (ADM-85) | « Appeler la famille » affiche la langue et le relais ; alerte si le relais proposé est un mineur |
| C32 [personas] | Grand-mère sans téléphone portable ni compte | Personne sans compte ; mise à jour proposée par un titulaire (ADM-73) | Ses coordonnées restent justes sans passer par le guichet |
| C33 [personas] | Ordonnance de placement provisoire reçue à 11 h, copie le lendemain | Interdiction provisoire d'urgence, pièce attendue (ADM-90) | Effet immédiat à l'accueil et sur les téléphones ; rappel à 48 h si la pièce manque |
| C34 [personas] | Changement de famille d'accueil, puis retour en famille | Assistant « changer de lieu d'accueil » (ADM-80) | Ancienne assistante familiale fermée à la date ; nouvelle ouverte ; adresse protégée reconduite |
| C35 [personas] | Parent hospitalisé ; sa sœur signe les autorisations pendant deux mois | Procuration temporaire pour les actes du quotidien (ADM-83) | Droits limités et datés ; l'autre titulaire informé |
| C36 [personas] | Interne dont la famille vit à l'étranger | Correspondant désigné avec mandat (ADM-84) | Il justifie les absences, récupère l'élève, est appelé en premier sur place |
| C37 [personas] | Un homme se présente à l'accueil comme le père ; aucun père déclaré | Procédure « personne inconnue » (ADM-89) | Pas de remise ; main courante ; proposition de vérification au secrétariat |
| C38 [personas] | « Son oncle viendra le chercher à 15 h » par téléphone | Consigne téléphonique confirmée par rappel sur un numéro du dossier (ADM-88) | Autorisation ponctuelle créée après confirmation seulement |
| C39 [personas] | Les deux parents déclarent des choses contraires dans la fiche de rentrée | Propositions en conflit (ADM-96) | Rien n'est appliqué automatiquement ; arbitrage humain ; chacun ignore la déclaration de l'autre |
| C40 [personas] | Une secrétaire de l'établissement est la mère séparée d'un élève | Conflit d'intérêts détecté (R-A18) | Par sa casquette professionnelle, elle ne lit pas les données de l'autre parent ; accès tracé ; sa casquette de parent est inchangée |
| C41 [personas] | Responsable incarcéré | Adresse institutionnelle ; préférences « courrier seulement » | Aucune remise ; autorité inchangée sauf décision [NV, à vérifier] |
| C42 [personas] | Responsable lui-même majeur protégé | Son tuteur ou curateur enregistré comme contact du responsable | Destinataire selon la mesure [NV, à vérifier] |
| C43 [personas] | Parent qui change de prénom ou de civilité | Versions sensibles de la personne (R-A28) | Courriers au nouveau nom ; anciennes valeurs scellées |
| C44 [personas] | Élève majeure qui quitte le domicile familial | Foyer propre de l'élève majeure (R-A26) | Ses parents ne voient pas sa nouvelle adresse sauf accord ; ils restent payeurs s'ils le sont |
| C45 [personas] | Couple séparé qui partageait une adresse e-mail | Alerte « adresse partagée » (R-A19) | La séparation exige une adresse propre par parent (D-014) |
| C46 [personas] | Mère protégée par une ordonnance, déclarée dès la préinscription | Adresse parquée scellée, jamais écrite en clair (R-A39 ; socle 3 R-I9) | La direction pose la protection avant toute écriture ; aucune invitation au père |
| C47 [personas] | Panne de réseau à 16 h 30 | Fiche de secours du jour (ADM-91) | L'accueil garde la liste des interdictions et autorisés, datée de sa génération |

---

## 5. Frontières

### 5.1 Tableau

| Sujet | Socle 2 (élève) | Socle 3 (inscriptions) | **Socle 4 (ce document)** | Autres socles |
|---|---|---|---|---|
| Identité de l'élève, INE et identifiants **de l'élève**, capacité juridique (majorité, émancipation, mesure de protection), confidentialité renforcée de l'élève, coordonnées **propres** de l'élève | **possède** | appelle | lit (la capacité change les droits des liens) | — |
| Personnes (responsables, contacts, tiers, élève majeur en tant que payeur), leur identité et leurs coordonnées | lit | **déclare** (copie du formulaire), ne crée rien avant validation | **possède** | 1 (compte) |
| Foyers, adresses, membres, résidence de l'élève, rythme d'alternance | lit (adresse affichée) | déclare (`foyer_cle`) | **possède** | 43 (transport), 40 (tarif par commune) lisent |
| Liens de responsabilité, qualité juridique, autorité, droits du lien, suspension, pièces | — | déclare ; recueille les accords | **possède** | tous les contrôles famille lisent le lien actif |
| Contacts d'urgence, personnes autorisées à récupérer, interdictions | — | — | **possède** | 14 (accueil, sorties), 34 (infirmerie), 12 (appel) lisent |
| Payeurs et répartition | — | propose (`payeur_propose`) | **possède** la désignation | **24** possède les factures et le débiteur comptable ; **25** possède IBAN et mandats |
| Quotient familial, ressources, bourses, numéro allocataire | — | — | — (le foyer de résidence n'est pas le foyer fiscal) | **26** possède (Q-A12) |
| Préférences de communication, langue, accessibilité | — | — | **possède** les préférences de la personne | **8** les applique |
| Destinataires d'un envoi | — | — | **calcule** (port `destinataires`) | **8** envoie, **16** messagerie, **7** documents nominatifs |
| Consentements administratifs (diffusion des coordonnées aux associations de parents, au transport) | — | — | **définit** les codes ; stockage chez 18 quand il existe | **18** possède le moteur de consentements |
| Autorisations parentales (sorties, droit à l'image, activités) | — | — | fournit **qui peut signer** | **18** possède |
| Pièces (jugement, livret de famille, justificatif de domicile, pièce d'identité, attestation d'assurance) | pièces d'identité de l'élève | pièces du dossier | **qualifie** la pièce et la rattache au fait qu'elle fonde | **7** stocke, analyse, conserve |
| Fiche de rentrée annuelle | photo, coordonnées de l'élève (sections) | **ouvre** la tâche après validation | **possède** | 34 (santé), 18 (autorisations) fournissent leurs sections |
| Identifiants officiels **des responsables** (identifiant SIECLE de la personne, jointure ÉduConnect) | — | — | **possède** | **9** les alimente et les exporte ; **1** connexion |
| Assurance scolaire de l'élève | — | pièce demandée | **possède** (attestation datée) | 42 (sorties) lit |
| Élections des représentants de parents | — | — | fournit les **électeurs** (titulaires) | **46** possède |
| Comptes de connexion, invitations, passkeys | — | déclenche à la validation | rattache le compte à la personne, déclenche l'invitation | **1** possède |
| Situation sociale, signalements, protection de l'enfance (contenu) | — | — | porte seulement les **effets administratifs** (lien service gardien, interdiction, adresse protégée) | **36** possède le suivi |

**Contradiction signalée** : `SOCLES.md` §3 range les « identifiants officiels (INE, identifiants SIECLE / ONDE, rattachement ÉduConnect) » dans le socle 4 ; `02_ELEVE.md` §4 et §7.3 met l'INE et les identifiants **de l'élève** au socle 2, avec un modèle complet. Ce document suit `02_ELEVE.md` : **identifiants de l'élève → socle 2 ; identifiants des personnes → socle 4** ; la fiche administrative affiche les deux. `SOCLES.md` est à corriger après validation (Q-A2).

### 5.2 Règles de frontière [R]

1. **Personne n'écrit dans les tables du socle 4** hors de ses commandes : ni le socle 3 (déjà acté, `03_INSCRIPTIONS.md` §5.2), ni les imports (socle 9 : propositions ou commandes), ni la facturation (elle lit les payeurs, elle ne les crée pas).
2. **Le socle 4 n'écrit pas l'identité de l'élève** ; il lit sa capacité juridique et réagit à `eleve.majeur`, `eleve.capacite_changee`, `eleve.confidentialite_changee`.
3. **Les contrôles d'accès famille** restent un point unique (`ent_eleve_lien_actif`, puis une fonction `droits_lien(personne, eleve, droit, date)`), utilisés par tous les socles.
4. **Garde de code** : comme `03_INSCRIPTIONS.md` §5.2 point 5, un test lit le SQL des autres modules et échoue s'il contient `INSERT`, `UPDATE` ou `DELETE` sur une table du socle 4 (`ent_personne*`, `ent_foyer*`, `ent_eleve_lien*`, `ent_eleve_residence`, `ent_eleve_remise*`, `ent_eleve_interdiction`, `ent_eleve_payeur`, `ent_adresse`, `ent_dossier_proposition`, `ent_fiche_rentree*`). Exception documentée : la facturation garde ses propres tables qui **référencent** `ent_foyer`.

### 5.3 Ports exposés (commandes et lectures appelées par les autres socles)

Chaque port est une fonction Rust qui prend la transaction en cours (`&mut TenantTx`) pour les écritures.

| Port | Appelé par | Sens | Remarques |
|---|---|---|---|
| `personnes::correspondances(declaration) -> Vec<Correspondance>` | socle 3 (validation), socle 9 (imports) | lecture minimisée | Candidats : même e-mail **vérifié**, même téléphone, même nom + prénom (+ adresse) ; score et raisons ; jamais d'autre établissement (D-013). Ne renvoie que ce qui sert à décider (nom, initiale du prénom, enfants déjà liés dans l'établissement, présence d'un compte), pas les coordonnées |
| `personnes::integrer_declaration(eleve_id, declaration, resolutions) -> Resultat` | socle 3 (validation) | écriture | Contrat déjà posé par `03_INSCRIPTIONS.md` §5.2 ; `resolutions` = choix humain par déclaration (`nouvelle_personne`, `meme_personne(id)`) et par foyer (`nouveau_foyer`, `meme_foyer(id)`) ; **un lien vers une personne qui a un compte n'ouvre aucun droit tant que la personne n'a pas confirmé** (invitation, §9 ADM-08) |
| `personnes::responsables_actifs(eleve_id, date) -> Vec<ResponsableResume>` | 3, 12, 14, 16, 18 | lecture | Nature, qualité juridique, droits, foyer du jour |
| `personnes::titulaires_autorite(eleve_id, date) -> Vec<PersonneId>` | 3 (accords), 18 (autorisations), 46 (électeurs) | lecture | Titulaires actifs ; signale `desaccord_declare` |
| `personnes::droits_lien(personne_id, eleve_id, droit, date) -> bool` | tous les contrôles famille | lecture | Remplace la lecture directe des colonnes `peut_*` |
| `personnes::destinataires(eleve_id, nature_envoi, date) -> Vec<Destinataire>` | 8, 16, 7, 24 | lecture | §8.12 ; chaque destinataire porte la **raison** (règle appliquée) et les canaux |
| `personnes::joignables(eleve_id, motif, date) -> Vec<ContactOrdonne>` | 12, 14, 34 | lecture | Ordre d'appel : foyer du jour, autre titulaire, contacts d'urgence ; horaires de joignabilité |
| `personnes::remise(eleve_id, date) -> Remise { autorises, interdits }` | 14 (accueil, sorties), application mobile | lecture | Interdictions toujours en tête ; journal de consultation |
| `personnes::foyer_du_jour(eleve_id, date) -> Option<FoyerId>` | 12, 43 | lecture | Selon le rythme de résidence |
| `personnes::commune_residence(eleve_id, date) -> Option<CodeCommune>` | 3 (listes mairies), 43 | lecture | Résidence principale ; en alternance : les deux communes |
| `personnes::payeurs(eleve_id, categorie_frais, date) -> Vec<PartPayeur>` | 24 | lecture | §8.9 |
| `personnes::adresse_postale(personne_id ou foyer_id, date) -> AdressePostale` | 7, 24 | lecture | Format postal ; **refusé** pour une adresse protégée si le demandeur n'est pas autorisé |

---

## 6. Concepts

### 6.1 Vocabulaire fixé [R]

| Concept | Définition | Table |
|---|---|---|
| **Personne** | Une personne physique dans **un** établissement (copie propre, D-013), avec ou sans compte | `ent_personne` (existante, enrichie) |
| **Organisme** | Une personne morale tierce : service départemental de l'aide sociale à l'enfance, employeur, collectivité, association, mutuelle. A des personnes de contact | `ent_organisme` |
| **Coordonnée** | Moyen de joindre une personne : e-mail, mobile, fixe, professionnel ; avec usages, vérification, état de délivrabilité, période | `ent_personne_coordonnee` |
| **Adresse** | Adresse postale structurée et normalisée, éventuellement protégée | `ent_adresse` |
| **Foyer** | Lieu de vie : des personnes qui vivent ensemble à une adresse, pour une période | `ent_foyer` (existant, enrichi) + `ent_foyer_membre`, `ent_foyer_adresse` |
| **Résidence de l'élève** | Le ou les foyers où vit l'élève, avec mode (principale, alternée, secondaire, internat) et rythme | `ent_eleve_residence` |
| **Lien** | Relation d'une personne à un élève : nature (mère, beau-parent, tuteur, service gardien…), qualité juridique, droits, dates | `ent_eleve_lien` (existant, enrichi) + `ent_eleve_lien_version`, `ent_eleve_lien_droit` |
| **Qualité juridique** | Ce que le droit reconnaît : titulaire de l'autorité parentale (conjointe ou exclusive), sans autorité, tuteur, délégataire, service gardien, tiers digne de confiance, représentant d'un majeur protégé, aucune | colonne de version du lien |
| **Droit du lien** | Ce que la personne peut faire ou recevoir pour cet élève dans l'ENT (voir les notes, justifier, signer, recevoir les factures…), issu de la nature et de la qualité, ajustable | `ent_eleve_lien_droit` |
| **Contact d'urgence** | Personne à appeler si on ne joint pas les responsables, dans un ordre | `ent_eleve_contact_urgence` (existante, recentrée) |
| **Personne autorisée** | Personne à qui l'enfant peut être remis, en permanence ou à des dates / jours | `ent_eleve_remise` |
| **Interdiction** | Personne à qui l'enfant ne doit pas être remis, ou à qui rien ne doit être communiqué, avec fondement | `ent_eleve_interdiction` |
| **Payeur** | Personne ou organisme qui paie une part d'une catégorie de frais, sur une période | `ent_eleve_payeur` |
| **Débiteur de facturation** | Objet comptable du socle 24 auquel s'adressent factures, mandats, relances. Aujourd'hui `ent_foyer.id` [F] | (socle 24) |
| **Proposition** | Changement demandé par la famille, un import ou un autre établissement, en attente de décision | `ent_dossier_proposition` |
| **Fiche de rentrée** | Vérification annuelle par chaque titulaire, pour tous ses enfants, des données du dossier | `ent_fiche_rentree_campagne`, `ent_fiche_rentree` |
| **Préférence** | Langue, canal préféré, courrier papier, accessibilité, horaires de joignabilité | `ent_personne_preference` |
| **Fait de vie** | Événement daté de la personne : décès, changement de nom | `ent_personne_version` (nature) |
| **Désaccord déclaré** | Déclaration par un titulaire de son désaccord sur un objet ; bloque les démarches qui exigent l'accord de tous | `ent_eleve_desaccord` |

### 6.2 Le mot « foyer » a trois sens : on les sépare

| Sens | Question | Porté par | Exemple qui casse si on confond |
|---|---|---|---|
| **Lieu de vie** | Où vit l'enfant, avec qui ? | socle 4 : `ent_foyer` + membres + adresse + résidence | Garde alternée : l'enfant a **deux** lieux de vie |
| **Débiteur** | À qui adresse-t-on la facture ? | socle 24 (aujourd'hui `ent_foyer.id`) | Le grand-père paie mais ne vit pas avec l'enfant |
| **Foyer fiscal** | Quel quotient, quelles ressources, quelles aides ? | socle 26 (`ent_foyer_quotient` aujourd'hui [F]) | Couple recomposé non marié : deux foyers fiscaux, un lieu de vie |

Recommandation [R] (Q-A1) : `ent_foyer` **reste** l'identifiant historique et devient le **lieu de vie** (membres, adresse). Le débiteur de facturation reste, par compatibilité, un `ent_foyer` : pour un payeur qui n'appartient à aucun foyer de l'élève (grand-père, organisme), le socle 4 lui crée ou retrouve **son propre** foyer (personne seule à son adresse) ou, pour un organisme, un foyer « organisme ». Le socle 24, quand il sera refait, pourra introduire son propre « débiteur » qui pointe une personne ou un organisme ; ce document ne le fait pas pour ne pas casser six tables de facturation. Le quotient familial est déplacé vers le socle 26 quand il sera refait ; d'ici là il reste accroché au foyer, avec un avertissement à l'écran (« le quotient concerne le foyer fiscal déclaré »).

---

## 7. L'historique : tout se modifie, rien ne s'efface

Mêmes trois mécanismes que `02_ELEVE.md` §6 : **versions métier datées**, **journal en ajout seul** (`ent_journal_modif` [F]), **événements outbox** (charges utiles sans valeurs).

| Donnée | Mécanisme d'historique [R] | Nature d'une modification |
|---|---|---|
| Identité d'une personne (nom, prénom, civilité) | `ent_personne_version` (bitemporel léger : `effet_du`, `enregistre_le`, `remplace_version_id`) ; projection courante sur `ent_personne` maintenue par la base | `creation`, `correction`, `changement` (mariage, jugement), `usage`, `deces`, `import`, `fusion` |
| Coordonnées | lignes datées (`du`, `au`) ; une correction ferme et remplace ; un rebond d'e-mail change l'état, pas la valeur | `ajout`, `correction`, `remplacement`, `retrait`, `invalide` (rebond, numéro non attribué) |
| Adresse d'un foyer | `ent_foyer_adresse` datée ; l'adresse elle-même est immuable (une nouvelle adresse = une nouvelle ligne `ent_adresse`) | `demenagement` (date d'effet réelle), `correction` (faute de frappe : remplace sans nouvelle période) |
| Membres d'un foyer | `ent_foyer_membre` daté | `arrivee`, `depart` |
| Résidence de l'élève | `ent_eleve_residence` datée | `debut`, `fin`, `changement_rythme` |
| Lien : nature, qualité juridique, autorité | `ent_eleve_lien_version` (pièce, motif) | `creation`, `correction`, `decision` (jugement), `majorite`, `deces`, `fin` |
| Droits du lien | `ent_eleve_lien_droit` datés, origine (`defaut_nature`, `ajuste`, `decision`, `majorite`) | — |
| Suspension | existante (colonnes) + versions | `suspension`, `retablissement` |
| Contacts, personnes autorisées, interdictions | lignes datées, jamais supprimées ; retrait = fermeture | — |
| Payeurs | lignes datées par catégorie ; une nouvelle répartition ferme la précédente à la date d'effet | — |
| Préférences, consentements | lignes datées ; consentement : texte et version du texte | `donne`, `retire` |

Règles [R] :
- **Une commande = une transaction** : fait, version, journal (noms de champs + identifiants de versions), événement. Plus aucune écriture sans journal (aujourd'hui respecté dans `dossier_personnes.rs` [F] ; à imposer aux nouveaux chemins).
- **Date d'effet** obligatoire pour tout ce qui a un sens dans le temps (déménagement, jugement, séparation) ; par défaut aujourd'hui ; antérieure possible (avec effet sur les envois **futurs** seulement : on ne réécrit pas l'histoire des envois) ; postérieure possible (déménagement annoncé : l'adresse change seule le jour J, tâche quotidienne).
- **Correction ≠ changement** (même distinction que `02_ELEVE.md` §6.2) : corriger une faute de numéro de rue ne crée pas de « déménagement » et ne déclenche ni recalcul de tarif ni liste mairie.
- **Lecture à une date** : tous les ports du §5.3 prennent une date ; « à qui a-t-on écrit en mars ? » se répond par le **journal des envois** du socle 8 (destinataires figés au moment de l'envoi), pas en recalculant.

---

## 8. Modèle de données cible

Notation : pseudo-DDL PostgreSQL, à préciser par le lot qui l'implémente. Toutes les tables portent `etablissement_id UUID NOT NULL`, des clés étrangères **composées** `(etablissement_id, x_id)` vers les tables du même établissement (qui supposent `UNIQUE (etablissement_id, id)` sur `ent_personne`, `ent_foyer`, `ent_eleve`, `ent_eleve_lien`, `ent_adresse`, `ent_organisme`), la RLS (§12.1), `cree_le`, `cree_par`. Les codes en `CHECK` sont des **codes du logiciel** ; les libellés affichés sont paramétrables. Dates métier en **Europe/Paris**.

### 8.1 `ent_personne` (existante, enrichie)

```
ent_personne
  id, etablissement_id                                  -- inchangés
  -- projection de la version courante (écrite par trigger seulement, comme 02_ELEVE.md §6.3)
  nom                 TEXT NOT NULL                     -- nom d'usage s'il existe, sinon nom de naissance (sens actuel : « nom affiché »)
  nom_naissance       TEXT NULL                         -- collecté seulement si un flux l'exige [NV : SIECLE ?]
  prenom              TEXT NULL
  civilite            TEXT NULL CHECK (civilite IN ('mme','m','autre'))   -- existant ; 'autre' = ne pas utiliser de civilité dans les courriers
  version_id          UUID NOT NULL                     -- version courante
  decede_le           DATE NULL                         -- fait de vie (projection)
  -- rattachements existants
  user_id, eleve_id, fusionnee_dans, source             -- source étendue : + 'import_aaf','import_bee','declaration','reprise_logiciel'
  -- données facultatives, activées par paramétrage (§10) et justifiées par un flux
  pcs_code            TEXT NULL                         -- profession et catégorie socioprofessionnelle (nomenclature INSEE [NV]) ; 'non_communique' possible
  -- colonnes héritées en sursis (§16) : email, telephone, telephone_2, adresse, updated_at
```

Pas de date de naissance, de nationalité, de situation matrimoniale ni de profession en clair pour les responsables (minimisation, Q-A9). La catégorie socioprofessionnelle n'est collectée **que si** l'établissement l'active (profil « public » : activée si la remontée nationale l'exige [NV] ; privé hors flux national : désactivée), avec la réponse « ne souhaite pas répondre ».

```
ent_personne_version
  id, etablissement_id, personne_id
  nom, nom_naissance, prenom, civilite
  nature        TEXT NOT NULL CHECK (nature IN ('creation','reprise','import','declaration','correction','changement','usage','deces','fusion'))
  effet_du      DATE NOT NULL
  remplace_version_id UUID NULL
  motif         TEXT NULL CHECK (length <= 500)        -- obligatoire pour correction, changement, deces
  piece_document_id UUID NULL                          -- acte de décès, jugement (socle 7)
  source, source_ref
  enregistre_le, enregistre_par
```

### 8.2 `ent_organisme`

```
ent_organisme
  id, etablissement_id
  type          TEXT NOT NULL CHECK (type IN ('ase','collectivite','employeur','association','mutuelle_assurance','etablissement_scolaire','autre'))
  nom           TEXT NOT NULL
  siret         TEXT NULL                              -- facultatif, pour la facturation d'un organisme
  adresse_id    UUID NULL → ent_adresse
  email, telephone   TEXT NULL                          -- coordonnées génériques de l'organisme
  actif         BOOLEAN NOT NULL DEFAULT true
ent_organisme_contact                                  -- référent ASE, gestionnaire RH…
  organisme_id, personne_id, fonction TEXT, du, au
```

Un organisme est payeur (§8.9) ou titulaire d'un lien « service gardien » (§8.6) ; ses contacts sont des personnes.

### 8.3 Préférences et consentements de la personne

```
ent_personne_preference
  personne_id PK, etablissement_id
  langue          TEXT NOT NULL DEFAULT 'fr'           -- code de langue ; traduction des envois : socle 8/30 plus tard
  courrier_papier BOOLEAN NOT NULL DEFAULT false        -- reçoit aussi (ou seulement) par voie postale
  canaux_refuses  TEXT[] NOT NULL DEFAULT '{}'          -- ex. 'sms' (un parent sourd préfère l'écrit ; un autre ne lit pas les e-mails)
  accessibilite   TEXT[] NOT NULL DEFAULT '{}'          -- 'gros_caracteres','pas_d_appel_vocal','interprete'
  joignable       JSONB NULL                            -- plages horaires (ex. travail de nuit) ; pour l'appel, jamais pour bloquer une urgence
  fuseau          TEXT NULL                             -- parent à l'étranger
  maj_le, maj_par

ent_personne_consentement                              -- consentements PROPRES au dossier administratif ; migreront vers le socle 18
  id, etablissement_id, personne_id
  objet       TEXT NOT NULL CHECK (objet IN ('diffusion_associations_parents','diffusion_transport','photo_remise','annuaire_parents_classe'))
  reponse     TEXT NOT NULL CHECK (reponse IN ('accorde','refuse'))
  texte_version TEXT NOT NULL                           -- texte exact présenté
  du, retire_le, canal ('portail','papier','fiche_rentree')
```

**[V]** (C. éduc. D111-8 ; VERIF R5) : les associations de parents d'élèves peuvent obtenir la liste des parents (noms, adresses postale et électronique) « à la condition que ceux-ci aient donné leur accord exprès à cette communication » ; d'où un consentement explicite, refusé par défaut.

### 8.4 Coordonnées

```
ent_personne_coordonnee
  id, etablissement_id, personne_id
  type          TEXT NOT NULL CHECK (type IN ('email','mobile','fixe','professionnel'))
  valeur        TEXT NOT NULL                           -- e-mail en minuscules ; téléphone en E.164 (+33…), affichage national
  libelle       TEXT NULL                               -- « portable perso », « standard du travail »
  usages        TEXT[] NOT NULL                          -- 'notifications','urgence','facturation','connexion' (le compte) — au moins un
  principal     BOOLEAN NOT NULL DEFAULT false           -- un principal par (personne, type ∈ {email}, {mobile,fixe})
  verifiee_le   TIMESTAMPTZ NULL                         -- code reçu (e-mail, SMS) ou compte vérifié
  delivrabilite TEXT NOT NULL DEFAULT 'inconnue' CHECK (delivrabilite IN ('inconnue','ok','rebond_temporaire','rebond_definitif','non_attribue','desinscrit'))
  delivrabilite_le TIMESTAMPTZ NULL                      -- écrit par le socle 8 (retours fournisseurs)
  source        TEXT NOT NULL                            -- saisie, famille, compte, import_aaf, import_bee, declaration, reprise
  du            TIMESTAMPTZ NOT NULL DEFAULT now()
  au            TIMESTAMPTZ NULL                          -- retirée ou remplacée
  remplace_id   UUID NULL
  UNIQUE partiel (personne_id, type, valeur) WHERE au IS NULL
```

Règles [R] :
- **Le compte est une coordonnée** : l'e-mail de connexion (`ent_user.email`) n'est **plus** l'adresse d'envoi par défaut ; à la création ou au rattachement d'un compte, l'e-mail vérifié du compte est **proposé** comme coordonnée (`usages = {connexion}` et, si la personne le confirme, `notifications`). Fin de la divergence n° 1 du §2.2.
- **Vérification** : un e-mail ou un mobile saisi par la famille est vérifié par code avant d'être utilisé pour des envois sensibles (factures, documents) [R] ; un numéro saisi par le secrétariat au guichet est marqué « déclaré ». Les envois d'urgence n'attendent jamais la vérification.
- **Rebonds** : `rebond_definitif` retire la coordonnée des envois et crée une alerte de qualité + une tâche « demander une nouvelle adresse » ; jamais de suppression.
- **Téléphones** : normalisation E.164 avec pays (indicatif) ; le logiciel accepte les numéros étrangers ; règles de format par pays : bibliothèque libre à choisir au lot A2 (licence à vérifier) [R].
- **Un parent séparé a son propre e-mail** (D-014) : contrainte vérifiée à la création d'un second foyer (§9 ADM-21) et alerte de qualité sinon.

### 8.5 Adresses et foyers

```
ent_adresse                                            -- immuable : une nouvelle adresse = une nouvelle ligne
  id, etablissement_id
  pays_cog          TEXT NOT NULL DEFAULT '99100'       -- code pays INSEE [NV : code de la France dans le COG pays]
  -- France (format postal à 6 lignes [NV : norme postale]) :
  numero, indice_repetition ('bis','ter'…), voie, complement_1 (bâtiment, escalier), complement_2 (lieu-dit, BP)
  code_postal       TEXT NULL
  commune_cog       TEXT NULL                            -- code commune INSEE en vigueur à la date de saisie
  commune_libelle   TEXT NULL                            -- libellé postal
  -- Étranger :
  lignes_libres     TEXT[] NULL                          -- jusqu'à 6 lignes
  -- Normalisation :
  normalisee        TEXT NOT NULL CHECK (normalisee IN ('non','proposee','validee','forcee'))   -- forcée = la personne confirme une adresse que la base ne connaît pas
  ban_id            TEXT NULL                            -- identifiant de la Base Adresse Nationale [NV]
  geo               POINT NULL                           -- seulement si l'établissement active le transport / la sectorisation (§10)
  protegee          BOOLEAN NOT NULL DEFAULT false        -- voir §12.3 ; valeurs chiffrées quand vraie
  valeurs_scellees  TEXT NULL                            -- si protégée : lignes chiffrées (champs_chiffres), colonnes en clair vidées sauf commune_cog si la direction l'autorise (listes mairie)

ent_foyer                                              -- existant (007), enrichi
  id, etablissement_id, libelle                          -- libellé calculé (« Foyer Martin – Durand ») et modifiable
  nature    TEXT NOT NULL DEFAULT 'famille' CHECK (nature IN ('famille','personne_seule','accueil_ase','internat_tiers','organisme'))
  ferme_le  DATE NULL                                    -- foyer qui n'existe plus (séparation, départ)
  remplace_par UUID[] NULL                               -- séparation : les foyers issus
  -- existant conservé : created_at ; références de facturation inchangées

ent_foyer_membre
  id, etablissement_id, foyer_id, personne_id
  du DATE NOT NULL, au DATE NULL
  EXCLUDE : une personne n'est membre que d'UN foyer à une date (gist sur personne_id, daterange)   -- [R] ; cas réel « deux domiciles » d'un adulte : un seul retenu pour le dossier

ent_foyer_adresse
  id, etablissement_id, foyer_id, adresse_id
  du DATE NOT NULL, au DATE NULL
  nature TEXT NOT NULL CHECK (nature IN ('demenagement','correction','creation','import'))
  EXCLUDE : une seule adresse par foyer à une date
```

Règles [R] :
- **L'adresse est portée par le foyer**, pas par la personne : un déménagement est **une** écriture qui vaut pour tous les membres et tous les enfants résidents (C21). Une personne sans foyer de l'élève (grand-père payeur, nourrice) a son foyer `personne_seule`.
- **Normalisation sans fuite** : la recherche d'adresse se fait sur une **copie locale** de la Base Adresse Nationale (données ouvertes, licence à vérifier [NV]) ou, à défaut, sans normalisation ; **pas** d'appel à une API externe avec l'adresse saisie par défaut (souveraineté, minimisation) [R, Q-A10]. La saisie reste possible sans normalisation (`forcee`).
- Le code commune sert aux listes mairies (socle 3), au transport et aux tarifs par commune ; il est obligatoire pour une adresse en France normalisée.
- `geo` n'est calculé que si un module qui l'exige est activé.

### 8.6 Liens de responsabilité

```
ent_lien_nature                                        -- catalogue du LOGICIEL (libellés paramétrables)
  code  TEXT PK   -- mere, pere, parent, beau_parent, grand_parent, frere_soeur, oncle_tante, autre_famille,
                  -- tuteur, delegataire, tiers_digne_confiance, service_gardien_ase, assistant_familial,
                  -- representant_majeur_protege, eleve_majeur, autre
  qualite_par_defaut TEXT                               -- ex. mere/pere/parent → titulaire ; beau_parent → aucune ; tuteur → tuteur
  droits_par_defaut  TEXT[]                             -- surchargés par le paramétrage (§10)
  code_siecle   TEXT NULL                               -- correspondance CODE_PARENTE [NV]
  exige_piece   BOOLEAN                                 -- tuteur, délégataire, tiers digne de confiance, service gardien : oui

ent_eleve_lien                                          -- existant, conservé comme pivot (id stable, contrôles famille)
  id, etablissement_id, eleve_id, personne_id, user_id (projection, existant)
  organisme_id     UUID NULL → ent_organisme             -- lien « service gardien » porté par un organisme (personne_id = référent)
  nature_code      TEXT NOT NULL → ent_lien_nature       -- remplace « lien » texte (reprise §16) ; libellé libre conservé dans nature_precision
  nature_precision TEXT NULL                             -- « marraine », « compagne du père »… (affichage seulement)
  -- projection de la version courante :
  qualite          TEXT NOT NULL CHECK (qualite IN ('titulaire_conjoint','titulaire_exclusif','sans_autorite','autorite_retiree','tuteur','delegataire','service_gardien','tiers_digne_confiance','representant_majeur','aucune'))
  autorite_parentale BOOLEAN NOT NULL                    -- existant, devient une projection : qualite ∈ titulaires/tuteur/délégataire
  version_id       UUID NOT NULL
  date_debut, date_fin, suspendu_le, suspendu_par, suspension_motif, suspension_reference   -- existants
  confirme_par_personne_le TIMESTAMPTZ NULL              -- la personne (avec compte) a accepté le lien (§9 ADM-08) ; NULL = droits d'accès inactifs
  ordre_contact    SMALLINT NULL                          -- « à contacter en priorité » (SIECLE A_CONTACTER_EN_PRIORITE [F Next])
  -- colonnes héritées en sursis : lien, garde, foyer_id, peut_voir_notes, peut_voir_vs, peut_justifier_absence,
  --   peut_autoriser_sorties (maintenues par trigger depuis ent_eleve_lien_droit), est_payeur, part_paiement (depuis ent_eleve_payeur)

ent_eleve_lien_version
  id, etablissement_id, lien_id
  nature_code, qualite
  nature     TEXT NOT NULL CHECK (nature IN ('creation','reprise','correction','decision','majorite','deces','fin','fusion'))
  effet_du   DATE NOT NULL, remplace_version_id UUID NULL
  motif      TEXT NULL                                    -- obligatoire pour decision, correction ; chiffré si sensible
  piece_document_id UUID NULL                             -- jugement, ordonnance, acte : obligatoire si la qualité est exclusive, retirée, tuteur, délégataire, tiers, service gardien (paramétrable en « demandée » seulement)
  enregistre_le, enregistre_par

ent_lien_droit_catalogue                                -- LOGICIEL : codes stables, liés aux actions des domaines
  code TEXT PK   -- voir_scolarite, voir_notes, voir_vie_scolaire, justifier_absence, autoriser_sorties,
                 -- signer_documents, repondre_formulaires, recevoir_bulletins, recevoir_informations,
                 -- recevoir_factures, payer_en_ligne, messagerie, voir_emploi_du_temps, prendre_rendez_vous,
                 -- recevoir_urgences, etre_electeur (déduit, non ajustable)
  domaine, libelle, exige_qualite TEXT[] NULL            -- ex. signer_documents exige une qualité titulaire ou tuteur

ent_eleve_lien_droit
  id, etablissement_id, lien_id, droit_code
  accorde    BOOLEAN NOT NULL
  origine    TEXT NOT NULL CHECK (origine IN ('defaut_nature','ajuste','decision','majorite','fusion'))
  du, au, motif NULL, decide_par
```

Règles [R] :
- **Qualité juridique ≠ nature ≠ droits.** La nature dit qui est la personne pour l'enfant ; la qualité dit ce que le droit reconnaît ; les droits disent ce que l'ENT ouvre. Une mère peut être `autorite_retiree` ; un beau-père `aucune` mais `recevoir_urgences` et `messagerie`.
- **Droits par défaut paramétrés par nature et qualité** (§10) ; à la création, ils sont recopiés en lignes `defaut_nature` ; ajuster crée une ligne `ajuste` motivée. Un changement du paramétrage **ne modifie pas** les liens existants, sauf action explicite « appliquer aux liens existants » (avec aperçu du nombre de liens touchés).
- **Garde-fous non paramétrables** : un lien `sans_autorite`, `aucune` ou `autorite_retiree` ne peut **jamais** recevoir `signer_documents` ni `etre_electeur` ; un lien suspendu n'ouvre rien (existant) ; un lien non confirmé par une personne qui a un compte n'ouvre aucun accès aux données (§9 ADM-08) ; une interdiction de communication (§8.8) l'emporte sur tout droit.
- **Parent sans autorité** (C6) : par défaut `recevoir_bulletins` et `voir_scolarite` accordés [NV : droit d'être informé], ajustables par la direction au vu de la pièce ; l'écran le rappelle.
- **Majorité** (C12) : sur `eleve.majeur`, les versions des liens parentaux passent en `sans_autorite` (nature `majorite`) ; les droits suivent la règle choisie à Q10 (recommandation : maintenus jusqu'à la réponse de l'élève majeur, qui choisit ; voir Q-A7).
- **Unicité** : une personne n'a qu'un lien par élève (existant `ent_eleve_lien_personne_uq` [F]).

### 8.7 Résidence de l'élève

```
ent_eleve_residence
  id, etablissement_id, eleve_id, foyer_id
  mode       TEXT NOT NULL CHECK (mode IN ('principale','alternee','secondaire','internat_tiers','accueil'))
  rythme     JSONB NULL         -- alternance : {type: 'semaines', cycle: 2, rang: 0|1, ref: 'AAAA-MM-JJ', bascule: 'lundi'|'vendredi_soir'}
                                --              {type: 'jours', cycle_jours: 14, jours: [0,1,7,8,…], ref}
                                --              {type: 'libre'}  -- pas de calcul, « appeler les deux »
  du DATE NOT NULL, au DATE NULL
  fondement  TEXT NOT NULL CHECK (fondement IN ('declaration','decision','accord_parents','import'))
  piece_document_id UUID NULL
  -- contraintes : à une date, soit une seule 'principale', soit deux ou plus 'alternee' (contrôle en service + test)
```

- **Foyer du jour** = calcul pur (module `domain-*` testé, sans base) à partir du rythme ; aperçu des quatre prochaines semaines à la saisie ; jours fériés et vacances **ignorés** en v1 (le rythme décrit l'année scolaire ; les vacances sont rarement alternées de la même façon) [R].
- Un élève **sans** résidence connue est signalé en qualité (mais pas bloqué).
- L'**internat** (socle 41) n'est pas une résidence au sens du socle 4 : c'est un régime (socle 2) ; `internat_tiers` sert aux élèves hébergés chez un correspondant ou une famille d'accueil hors ASE [H].

### 8.8 Urgence, remise, interdictions

```
ent_eleve_contact_urgence                               -- existante (122), recentrée sur « qui appeler »
  id, etablissement_id, eleve_id, personne_id
  relation_code TEXT NOT NULL → ent_lien_nature (hors qualités juridiques) ; relation TEXT (existante) → nature_precision
  ordre, statut ('valide','propose','refuse'), commentaire       -- existants (+ 'refuse')
  du, au
  -- peut_recuperer (existant) migré vers ent_eleve_remise puis retiré

ent_eleve_remise                                        -- personnes à qui l'enfant peut être remis
  id, etablissement_id, eleve_id, personne_id
  portee      TEXT NOT NULL CHECK (portee IN ('permanente','jours','ponctuelle'))
  jours       SMALLINT[] NULL                           -- 1 = lundi … 7
  du DATE NOT NULL, au DATE NULL                        -- ponctuelle : un jour ou une plage
  horaires    TEXT NULL                                 -- « après l'étude »
  statut      TEXT NOT NULL CHECK (statut IN ('valide','propose','refuse','retire'))
  declare_par_personne_id UUID NULL                     -- titulaire qui l'a demandée (lien actif avec droit autoriser_sorties)
  piece_identite_exigee BOOLEAN NOT NULL                 -- paramétrage par défaut
  photo_object_key TEXT NULL                            -- seulement si l'établissement active les photos des personnes autorisées ET avec consentement (§8.3 photo_remise)
  valide_par, valide_le

ent_eleve_interdiction
  id, etablissement_id, eleve_id, personne_id NULL
  personne_libelle TEXT NULL                            -- personne non enregistrée (nom tel qu'écrit dans la décision)
  objet       TEXT[] NOT NULL                           -- 'remise','communication','visite','contact_eleve'
  fondement   TEXT NOT NULL CHECK (fondement IN ('decision_justice','ordonnance_protection','decision_ase','demande_titulaire','decision_etablissement'))
  piece_document_id UUID NULL                           -- obligatoire si fondement = décision ou ordonnance (paramétrable en « attendue »)
  motif_scelle TEXT NULL                                -- chiffré ; jamais dans un événement ni un journal en clair
  consigne    TEXT NOT NULL CHECK (length <= 300)       -- ce que l'accueil doit faire : « ne pas remettre, prévenir la direction et appeler la mère »
  du DATE NOT NULL, au DATE NULL
  decide_par, decide_le, levee_par, levee_le, levee_motif
```

Règles [R] :
- **Interdiction prioritaire** : si une personne est à la fois autorisée et interdite, l'interdiction l'emporte et une **alerte de cohérence** est levée pour la direction.
- **Demande d'un titulaire** contre l'autre parent (« je ne veux pas qu'il vienne le chercher ») **sans** décision de justice : enregistrée `demande_titulaire`, **non bloquante** par défaut [**[V]** : les deux parents exercent en commun l'autorité parentale (C. civ. 372) et, pour un acte usuel, chacun est réputé agir avec l'accord de l'autre à l'égard des tiers de bonne foi (372-2) ; une demande sans décision ne retire rien à l'autre ; VERIF R9] ; affichée comme « signalement à vérifier » et transmise à la direction. Le logiciel ne prive pas un titulaire sur la seule parole de l'autre.
- Les titulaires ne voient **pas** les interdictions qui les visent (ni leur fondement) ; ils voient les leurs s'ils les ont demandées.
- Autorisation ponctuelle déposée par un titulaire depuis l'appli : validée automatiquement si le paramétrage le permet (« autorisations ponctuelles sans validation : oui ») et si aucune interdiction ne vise la personne.

### 8.9 Payeurs

```
ent_categorie_frais                                     -- LOGICIEL + paramétrage : scolarite, restauration, internat, activites, voyages, fournitures, autres
ent_eleve_payeur
  id, etablissement_id, eleve_id
  categorie   TEXT NOT NULL                             -- 'toutes' ou une catégorie
  personne_id UUID NULL, organisme_id UUID NULL          -- exactement un des deux
  part_pct    NUMERIC(5,2) NOT NULL CHECK (part_pct > 0 AND part_pct <= 100)
  debiteur_foyer_id UUID NOT NULL → ent_foyer           -- compatibilité facturation (§6.2) : foyer auquel la facture est adressée
  du DATE NOT NULL, au DATE NULL
  motif TEXT NULL, decide_par
  -- invariant (service + test) : pour chaque catégorie effective à une date, somme des parts = 100
```

- **Une catégorie précise l'emporte sur `toutes`** (« la mère paie tout, sauf la cantine payée par le père »).
- Un payeur peut être **sans lien de responsabilité** (grand-parent, organisme, élève majeur via sa fiche personne, existant [F]).
- Changer de payeur prend effet à une date ; le socle 24 reçoit `payeurs.modifies` et décide de ses factures **futures** ; jamais de réaffectation silencieuse des factures émises.
- Le **mandat SEPA** reste au socle 25 et suit le débiteur ; changer de payeur affiche « le mandat de M. X ne servira plus pour cet élève à partir du … ».

### 8.10 Propositions et fiche de rentrée

```
ent_dossier_proposition                                 -- même principe que ent_eleve_proposition (02_ELEVE.md §7.8)
  id, etablissement_id, eleve_id NULL, personne_id NULL, foyer_id NULL
  cible   TEXT NOT NULL CHECK (cible IN ('coordonnee','adresse_foyer','identite_personne','contact_urgence','remise','residence','lien','situation','assurance','nouvelle_personne'))
  champs  JSONB NOT NULL                                -- {champ: {actuel, propose}} ; valeurs sensibles scellées
  source  TEXT NOT NULL CHECK (source IN ('famille','fiche_rentree','eleve_majeur','import','autre_etablissement_par_famille','declaration_inscription','systeme'))
  depose_par_personne_id UUID NULL
  piece_document_id UUID NULL
  effet_du DATE NULL
  statut  TEXT NOT NULL CHECK (statut IN ('en_attente','appliquee_auto','acceptee','acceptee_partiellement','refusee','expiree','obsolete'))
  regle_auto TEXT NULL, decision_motif TEXT NULL, decide_par, decide_le
  cree_le, expire_le

ent_fiche_rentree_campagne
  id, etablissement_id, annee_id
  ouverture_le, cloture_le, relances JSONB
  sections TEXT[] NOT NULL                              -- coordonnees, adresse, responsables, urgence, remise, preferences, consentements, assurance,
                                                        -- + sections fournies par d'autres socles : sante (34), autorisations (18), photo (2), regime (2)
  message, statut ('brouillon','ouverte','close')

ent_fiche_rentree                                       -- une par (campagne, personne titulaire) : couvre TOUS ses enfants
  id, etablissement_id, campagne_id, personne_id
  enfants UUID[] NOT NULL
  etat    TEXT NOT NULL CHECK (etat IN ('a_faire','en_cours','envoyee','traitee','dispensee'))
  sections_etat JSONB NOT NULL                          -- {section: 'inchange' | 'modifie' | 'a_faire'}
  envoyee_le, traitee_le, traitee_par
```

- **Champ par champ, qui fait foi** (§10) : `modifier` (la famille modifie directement, ex. son mobile après vérification par code), `proposer` (validation par l'établissement, ex. adresse), `non` (ex. qualité juridique : seule une pièce et le secrétariat la changent).
- La fiche de rentrée ne **redemande rien** : elle affiche ce qui est connu ; « rien n'a changé » valide une section en un geste ; une modification devient une proposition ou une écriture directe selon le champ.
- **Multi-établissements (D-013)** : quand un parent modifie ses coordonnées depuis son profil, l'ENT lui propose « mettre aussi à jour chez : ☐ établissement B » ; chaque établissement coché reçoit une **proposition** `autre_etablissement_par_famille` (jamais une écriture directe), et ne sait rien des autres établissements (`IDENTITE_ET_PROFILS_FAMILLES.md` §2). La proposition est créée par la couche ENT au nom de la personne, établissement par établissement.

### 8.11 Identifiants des personnes, assurance, désaccords

```
ent_personne_identifiant                                -- même structure que ent_eleve_identifiant (02_ELEVE.md §7.3)
  id, etablissement_id, personne_id
  systeme   TEXT NOT NULL → ent_systeme_identifiant     -- SIECLE_PERSONNE, ONDE_RESPONSABLE, AAF_JOINTURE, ANCIEN_LOGICIEL
  valeur, statut ('declare','a_verifier','confirme','conteste','revoque'), provenance, valide_du, valide_au
  UNIQUE partiel (etablissement_id, systeme, valeur) WHERE actif

ent_eleve_assurance
  id, etablissement_id, eleve_id, annee_id
  assureur TEXT NOT NULL, numero_contrat TEXT NULL
  garanties TEXT[] NOT NULL                             -- 'responsabilite_civile','individuelle_accident' [NV : garanties exigées pour les activités facultatives]
  du DATE NOT NULL, au DATE NOT NULL
  piece_document_id UUID NULL, statut ('declaree','verifiee','refusee')

ent_eleve_desaccord                                     -- désaccord déclaré par un titulaire (C23)
  id, etablissement_id, eleve_id, declarant_personne_id
  objet   TEXT NOT NULL CHECK (objet IN ('inscription','reinscription','depart','sortie','orientation','image','general'))
  objet_ref UUID NULL                                   -- démarche visée (dossier socle 3, sortie…)
  declare_le, piece_document_id NULL
  leve_le, leve_par, leve_motif, leve_piece_id
```

### 8.12 Destinataires d'un envoi (règles, pas une table)

Le port `destinataires(eleve, nature_envoi, date)` applique une **règle par nature d'envoi**, définie par le logiciel et ajustable dans des bornes par l'établissement (§10). Chaque destinataire retourné porte la règle qui l'a retenu (« titulaire de l'autorité parentale », « payeur 50 % restauration », « parent qui a l'enfant cette semaine ») et ses canaux (selon préférences et délivrabilité).

| Nature d'envoi | Règle par défaut [R] | Bornes non paramétrables |
|---|---|---|
| Bulletin, livret, décision d'orientation, convocation à un conseil de discipline | **chaque** lien avec `recevoir_bulletins`, **à son adresse**, document nominatif | jamais une personne interdite de communication ; jamais l'adresse d'un autre destinataire sur le document |
| Information générale de l'établissement | liens avec `recevoir_informations` | — |
| Absence, retard, incident du jour (SMS, appel) | liens avec `voir_vie_scolaire` ; en alternance : **d'abord le foyer du jour**, puis les autres titulaires selon le paramétrage (« prévenir les deux » / « le foyer du jour seulement ») | jamais une personne interdite |
| Urgence (malaise, accident, évacuation) | `joignables` : foyer du jour, autres titulaires, contacts d'urgence dans l'ordre | les heures calmes ne s'appliquent pas ; une interdiction s'applique toujours |
| Facture, relance, échéancier | payeurs de la catégorie à la date, à l'adresse du débiteur | jamais un non-payeur (sauf choix « copie aux titulaires » activé) |
| Document à signer, formulaire d'autorisation | liens avec `signer_documents` (titulaires) selon la règle d'accord du formulaire (un seul, tous) | jamais un `sans_autorite` |
| Message de messagerie d'un enseignant « aux parents » | liens avec `messagerie` ; parents séparés = **conversations distinctes** par défaut [R] | — |
| Courrier papier | personnes avec `courrier_papier` ou sans aucune coordonnée électronique valide | adresse protégée : uniquement pour son titulaire |

Le socle 8 **fige** la liste au moment de l'envoi (journal des envois) ; le socle 4 ne garde que la règle.

### 8.13 Paramétrage du domaine

```
ent_param_dossier_admin                                 -- une ligne par établissement
  saisie_famille        JSONB    -- par champ : 'non' | 'proposer' | 'modifier'
  pieces                JSONB    -- par fait (qualité exclusive, interdiction, décès…) : 'exigee' | 'attendue' | 'non'
  droits_par_nature     JSONB    -- par (nature, qualité) : liste de droits accordés par défaut (dans les bornes §8.6)
  regles_envoi          JSONB    -- §8.12, dans les bornes
  remise                JSONB    -- {piece_identite_exigee, photos_autorises: bool, ponctuelle_sans_validation: bool, nb_max}
  urgence               JSONB    -- {min_contacts, max_contacts}
  collecte              JSONB    -- {pcs: bool, nom_naissance_responsables: bool, assurance: bool, geo: bool}
  fiche_rentree         JSONB    -- sections, obligatoires, relances
  ent_lien_nature_libelle (etablissement_id, code, libelle, actif)   -- libellés et natures proposées
```

---

## 9. Actions métier

Notation des acteurs comme `SOCLE2_DOMAINES.md` §3 : ADF administratif, DIR direction, CPE, VS vie scolaire, ACC accueil, PRF professeur, PP, INF, CPT comptabilité, PAR responsable (titulaire), TIE autre personne liée, ELV élève majeur, SYS système. ★ = action sensible (récapitulatif avant / après et confirmation ; ré-authentification quand elle sera branchée). Droits : §13.

### 9.1 Personnes et coordonnées

| ID | Action | Acteur · droit | Règles clés | Événements | Remplace |
|---|---|---|---|---|---|
| ADM-01 | Rechercher une personne (homonymes) | ADF · `personnes.gerer` | même établissement ; résultat minimisé | — | DOS-03 (partie) |
| ADM-02 | Créer une personne (avec ou sans e-mail) | ADF · `personnes.gerer` ; SYS (3, 9) | doublons proposés **avant** création ; version `creation` | `personne.creee` [F] | DOS-03 |
| ADM-03 | Corriger ou changer l'identité d'une personne | ADF · `personnes.gerer` | nature, motif, pièce selon nature | `personne.identite_modifiee` (noms de champs) | DOS-03 |
| ADM-04 | Enregistrer un décès ★ | ADF, DIR · `personnes.gerer` | date, pièce selon paramétrage ; ferme liens, remise, contacts, payeurs à la date ; **annule toutes les relances programmées** ; alerte direction et comptabilité ; aucun envoi automatique à la famille | `personne.decedee` | — |
| ADM-05 | Ajouter, corriger, retirer une coordonnée | ADF · `personnes.gerer` ; PAR (soi) | vérification par code pour la famille ; usages ; un principal par type | `personne.coordonnees_modifiees` [F] | DOS-10 (partie) |
| ADM-06 | Marquer une coordonnée invalide (rebond, numéro non attribué) | SYS (8) ; ADF | jamais de suppression ; tâche « demander une nouvelle adresse » | `personne.coordonnee_invalide` | — |
| ADM-07 | Définir ses préférences (langue, canaux, papier, accessibilité, horaires) | PAR, TIE (soi) ; ADF pour une personne sans compte | — | `personne.preferences_modifiees` | — |
| ADM-08 | Inviter une personne à créer ou rattacher son compte ; confirmer le lien | ADF · `personnes.gerer` ; PAR (accepte) | invitation à usage unique vers une coordonnée **vérifiable** ; le lien n'ouvre les accès qu'après acceptation (`confirme_par_personne_le`) ; rattachement d'une identité existante (D-013) | `personne.invitee`, `personne.compte_rattache` [F], `lien.confirme` | DOS-09 |
| ADM-09 | Fusionner deux personnes en doublon ★ / annuler | DIR · `personnes.fusionner` | existant [F] étendu aux nouvelles tables (registre `TABLES_PERSONNE` + test de garde, comme `02_ELEVE.md` §7.10) | `personne.fusionnee` / `defusionnee` [F] | DOS-15 |
| ADM-10 | Séparer nom et prénom des personnes reprises | ADF · `personnes.gerer` | proposition de découpage par le logiciel, validation humaine, en lot | `personne.identite_modifiee` | — |
| ADM-11 | Gérer les organismes et leurs contacts | ADF · `personnes.gerer` | — | `organisme.modifie` | — |
| ADM-12 | Enregistrer ou retirer un consentement administratif | PAR (soi) ; ADF (papier, avec pièce) | texte versionné ; retrait à tout moment | `personne.consentement_modifie` | — |
| ADM-13 | Gérer les identifiants d'une personne (SIECLE, jointure ÉduConnect) | ADF · `personnes.identifiants.gerer` ; SYS (9) | collision → `conteste` + alerte | `personne.identifiant_change` | — |
| ADM-71 [personas] | Déclarer deux personnes **distinctes** (père et fils homonymes) | ADF · `personnes.gerer` | décision mémorisée ; la paire n'est plus proposée par la détection de doublons ni par `correspondances`, sauf changement d'identité | `personne.non_doublon_declare` | — |
| ADM-72 [personas] | **Vérifier l'identité au guichet** et rétablir l'accès d'une personne (e-mail perdu) ★ | ADF · `personnes.gerer` | pièce d'identité **vue** (type et auteur de la vérification tracés, jamais la copie) ; nouvelle coordonnée vérifiée par code sur place ; déclenche la récupération du socle 1 avec son délai de sécurité (D-018) ; alerte à l'ancienne adresse | `personne.identite_verifiee_guichet` | — |
| ADM-73 [personas] | **Proposer la mise à jour d'un contact** ou d'une personne autorisée que l'on a déclaré (grand-mère sans compte) | PAR | proposition au secrétariat, mêmes règles que ADM-61 ; seulement pour les personnes qu'il a déclarées et qui n'ont pas de compte (R-A33) | `dossier.proposition_deposee` | — |
| ADM-74 [personas] | Enregistrer une **coordonnée de foyer** (téléphone fixe du domicile) | ADF ; PAR (proposition) | partagée par les membres ; proposée à la fermeture au déménagement (R-A34) | `foyer.coordonnees_modifiees` | — |
| ADM-75 [personas] | **Signaler une coordonnée erronée** depuis un appel (« numéro non attribué », « ce n'est pas la mère ») | VS, INF, PP, ACC · `responsables.lire` | un clic ; état `a_verifier` ; tâche au secrétariat ; `joignables` passe au suivant | `personne.coordonnee_signalee` | — |
| ADM-76 [personas] | **Signaler le décès d'un responsable** depuis le portail famille | PAR, TIE | proposition prioritaire ; mesure conservatoire immédiate : plus aucune relance vers la personne en attendant la validation (ADM-04) ; message humain seulement | `dossier.proposition_deposee` | — |

### 9.2 Foyers, adresses, résidence

| ID | Action | Acteur · droit | Règles clés | Événements | Remplace |
|---|---|---|---|---|---|
| ADM-20 | Créer un foyer, y ajouter ou retirer un membre | ADF · `personnes.gerer` | une personne dans un seul foyer à une date | `foyer.cree` (réservé [F]), `foyer.membres_modifies` | DOS-04 |
| ADM-21 | **Séparer un foyer** (assistant) ★ | ADF · `personnes.gerer` | un foyer → deux ou plus ; qui va où ; nouvelles adresses ; résidence des enfants ; payeurs ; **un e-mail par parent exigé** (D-014) ; invitations ; date d'effet ; aperçu complet avant confirmation | `foyer.separe` | DOS-16 |
| ADM-22 | Réunir (recomposition) : une personne rejoint un foyer | ADF · `personnes.gerer` ; PAR (proposition) | — | `foyer.membres_modifies` | — |
| ADM-23 | **Déménager un foyer** | ADF · `personnes.gerer` ; PAR (proposition, pièce selon paramétrage) | date d'effet ; aperçu des impacts (enfants résidents, commune, transport, tarif, secteur [H]) ; consommateurs prévenus | `foyer.adresse_changee` (commune avant/après, pas l'adresse) | DOS-04 |
| ADM-24 | Corriger une adresse (sans déménagement) | ADF · `personnes.gerer` | remplace la ligne sans nouvelle période ; pas de recalcul de tarif | `foyer.adresse_corrigee` | — |
| ADM-25 | Protéger ou déprotéger une adresse ★ | DIR · `responsables.protege.gerer` | motif scellé ; pièce attendue ; effets §12.3 | `adresse.protection_changee` (sans motif) | — |
| ADM-26 | Définir la résidence de l'élève (principale, alternée et rythme) | ADF · `personnes.gerer` ; PAR (proposition) | aperçu calendrier ; fondement ; pièce si décision | `eleve.residence_changee` | — |
| ADM-27 | Normaliser les adresses en lot | ADF · `personnes.gerer` | base locale ; propositions, jamais de remplacement silencieux | — | — |
| ADM-77 [personas] | Déclarer une **adresse de domiciliation** distincte du lieu de vie, ou une situation **sans domicile stable** | ADF ; PAR (proposition) | domiciliation (centre communal d'action sociale, association) pour le courrier ; lieu de vie facultatif ; jamais bloquant (R-A5) | `foyer.adresse_changee` | — |
| ADM-78 [personas] | Déclarer un **échange ponctuel de garde** | PAR (titulaire résidant) ; ADF | période courte (paramètre, défaut 14 jours) ; foyer du jour remplacé ; l'autre titulaire informé sans détail ; contestation → « appeler les deux » jusqu'à arbitrage (R-A2) | `eleve.residence_exception` | — |
| ADM-79 [personas] | Définir le **partage des vacances** dans le rythme (moitiés, années paires et impaires) | ADF ; PAR (proposition) | aperçu calendrier sur l'année ; facultatif (R-A3) | `eleve.residence_changee` | — |
| ADM-80 [personas] | **Changer de lieu d'accueil** d'un enfant confié (ou retour en famille) ★ | ADF · `liens.juridique.gerer` | en une commande : résidence, remise, droits de l'ancien et du nouvel accueillant, transport, adresse protégée reconduite, date d'effet ; aperçu avant / après | `eleve.residence_changee`, `lien.clos`, `lien.qualite_changee` | — |
| ADM-81 [personas] | Voir la **constellation familiale** d'un élève | ADF, DIR · `responsables.juridique.lire` | tous les foyers, personnes, frères et sœurs et demi-frères et sœurs scolarisés, rôles ; adresses protégées masquées ; cliquable | — | — |

### 9.3 Liens, autorité, droits

| ID | Action | Acteur · droit | Règles clés | Événements | Remplace |
|---|---|---|---|---|---|
| ADM-30 | **Ajouter un responsable** (personne + lien + foyer + coordonnées, une commande) | ADF · `personnes.gerer` | doublons proposés ; nature ; qualité par défaut ; droits par défaut paramétrés ; pièce si la nature l'exige ; invitation proposée | `personne.creee`, `eleve.lien_updated` [F] | DOS-05 (corrige l'écran en deux appels) |
| ADM-31 | Modifier la nature ou les dates d'un lien | ADF · `personnes.gerer` | version `correction` | `eleve.lien_updated` [F] | DOS-05 |
| ADM-32 | **Enregistrer une décision sur l'autorité** (exclusive, retrait, tutelle, délégation, placement) ★ | DIR, ADF · `liens.juridique.gerer` | pièce ; date d'effet ; motif scellé ; droits recalculés avec aperçu ; information des personnes concernées **décidée humainement** | `lien.qualite_changee` (qualité avant/après, sans motif) | — |
| ADM-33 | Ajuster les droits d'un lien | ADF · `liens.juridique.gerer` | dans les bornes §8.6 ; motif | `lien.droits_modifies` (codes) | DOS-05 |
| ADM-34 | Suspendre / rétablir un lien ★ | DIR · `liens.suspendre` [F] | existant | `lien.suspendu` / `lien.retabli` [F] | DOS-06 |
| ADM-35 | Clore un lien (fin de relation, décès, majorité) | ADF · `personnes.gerer` ; SYS | date | `lien.clos` | — |
| ADM-36 | Appliquer la majorité aux liens parentaux | SYS sur `eleve.majeur` | versions `majorite` ; droits selon Q10 ; demande de choix à l'élève | `lien.qualite_changee` | DOS-14 |
| ADM-37 | Enregistrer / lever un désaccord déclaré | ADF, DIR · `liens.juridique.gerer` ; PAR (déclare) | bloque les démarches qui exigent l'accord de tous ; levée avec pièce | `eleve.desaccord_declare` / `_leve` | — |
| ADM-38 | Choisir ce que ses parents continuent de recevoir | ELV (majeur) | droits des liens parentaux ajustés (origine `majorite`) ; révocable | `lien.droits_modifies` | — |
| ADM-82 [personas] | **Confirmer un lien** au guichet (identité vérifiée) ou en un geste depuis un compte existant | ADF · `personnes.gerer` ; PAR (connecté) | trois voies : invitation (ADM-08), bouton « je confirme être responsable de X » pour une personne déjà connectée (professeur-parent, parent d'un aîné), vérification au guichet (type de pièce et auteur tracés) ; ouvre les accès (R-A40) | `lien.confirme` | — |
| ADM-83 [personas] | Accorder une **procuration temporaire** pour les actes du quotidien | PAR (titulaire) ; ADF valide | droits limités (`signer_quotidien`, `justifier_absence`, remise, `recevoir_urgences`) ; dates obligatoires (3 mois au plus, renouvelable) ; révocable ; l'autre titulaire informé ; jamais au profit d'une personne interdite (R-A16) | `lien.procuration_accordee` / `_revoquee` | — |
| ADM-84 [personas] | Désigner un **correspondant** pour un interne dont la famille est éloignée | ADF · `liens.juridique.gerer` ; PAR (demande) | nature `correspondant`, mandat écrit (pièce), droits limités, dates annuelles (R-A17) | `eleve.lien_updated` | — |
| ADM-85 [personas] | Déclarer un **relais de communication** (interprète familial, médiateur) | ADF ; PAR | nature `relais_communication`, aucun droit sur la scolarité ; reçoit une copie seulement si un titulaire l'a demandé ; alerte si le relais est mineur (R-A6) | `eleve.lien_updated` | — |
| ADM-86 [personas] | **Expliquer à un responsable ses droits**, enfant par enfant, et pourquoi | PAR (soi) ; ADF (vue de ce que voit le parent) | issu des droits du lien ; motifs scellés jamais montrés : texte neutre (« décision de l'établissement, contactez la direction ») ; après la majorité : « votre enfant est majeur ; voici ce que vous recevez » (R-A24) | — | — |

### 9.4 Urgence, remise, interdictions

| ID | Action | Acteur · droit | Règles clés | Événements | Remplace |
|---|---|---|---|---|---|
| ADM-40 | Ajouter, ordonner, retirer un contact d'urgence | ADF · `personnes.gerer` ; PAR (proposition) | min / max paramétrés ; un contact ne peut pas être un titulaire (déjà appelé) [R] | `eleve.contacts_urgence_changes` [F] | DOS-08 |
| ADM-41 | Autoriser une personne à récupérer l'élève (permanente, jours, ponctuelle) | ADF · `remise.gerer` ; PAR (droit `autoriser_sorties`) | pièce d'identité exigée à l'accueil selon paramétrage ; ponctuelle validée seule si permis | `eleve.remise_modifiee` | DOS-08 |
| ADM-42 | Valider ou refuser une proposition de remise ou de contact | ADF · `remise.gerer` | refus motivé, visible du demandeur | idem | DOS-08 |
| ADM-43 | **Poser ou lever une interdiction** ★ | DIR · `remise.interdire` | fondement ; pièce selon fondement ; consigne ; motif scellé ; conflit avec une autorisation → alerte | `eleve.interdiction_changee` (posée/levée, objets, sans personne ni motif) | — |
| ADM-44 | Consulter « qui peut récupérer cet élève aujourd'hui » | ACC, VS, INF · `remise.lire` | interdictions en tête ; consultation journalisée ; photos si activées | — | — |
| ADM-45 | Enregistrer une remise effective (qui est venu, pièce vérifiée) | ACC, VS · `remise.lire` | tracée (socle 14 possède le registre des sorties ; le socle 4 fournit la vérification) | (socle 14) | — |
| ADM-46 | Obtenir la liste ordonnée des personnes à appeler | VS, INF, PP · `responsables.lire` | foyer du jour, horaires, contacts | — | — |
| ADM-87 [personas] | **Demander à distance l'accord d'un titulaire** pour une remise non prévue | ACC, VS, INF · `remise.lire` | notification aux titulaires ayant `autoriser_sorties` : « X est à l'accueil pour Léo ; l'autorisez-vous ? » ; réponse oui / non en 10 min (paramètre), sinon appel ; une interdiction l'emporte toujours ; la réponse crée une remise ponctuelle (R-A12) | `eleve.remise_accord_demande` / `_repondu` | — |
| ADM-88 [personas] | Enregistrer une **consigne téléphonique** de remise, à confirmer | ACC, VS · `remise.lire` | rappel obligatoire sur un numéro **du dossier**, jamais celui donné par l'appelant ; autorisation ponctuelle créée seulement après confirmation (R-A13) | `eleve.remise_modifiee` | — |
| ADM-89 [personas] | Traiter une **personne inconnue qui se dit responsable** | ACC → DIR | aucune remise ; main courante (socle 14) ; proposition « personne se déclarant responsable » au secrétariat avec pièces à demander ; les déclarations des titulaires connus restent valables (R-A14) | `dossier.proposition_deposee` | — |
| ADM-90 [personas] | Poser une **interdiction provisoire d'urgence** (décision annoncée, pièce à venir) ★ | DIR · `remise.interdire` | effet immédiat ; fondement `decision_annoncee` ; rappel à 48 h si la pièce manque ; à J+7 sans pièce, la direction confirme ou lève (jamais de levée silencieuse) (R-A9) | `eleve.interdiction_changee` | — |
| ADM-91 [personas] | Produire la **fiche de secours** (remises, interdictions et urgences du jour, hors ligne) | ACC, VS, DIR · `remise.lire` | PDF du jour ou cache chiffré sur le poste et le téléphone ; heure de génération visible ; filigrane ; expiration en fin de journée ; journalisé (R-A11) | `export.realise` | — |
| ADM-92 [personas] | Émettre ou révoquer une **carte de retrait** (QR) pour une personne autorisée | ADF · `remise.gerer` | jeton signé sans donnée lisible ; papier ou téléphone ; scan → photo de l'élève et statut du jour ; révocation propagée (R-A15) | `eleve.carte_retrait_emise` / `_revoquee` | — |

### 9.5 Payeurs, assurance

| ID | Action | Acteur · droit | Règles clés | Événements | Remplace |
|---|---|---|---|---|---|
| ADM-50 | Définir les payeurs par catégorie et période | ADF, CPT · `payeurs.definir` [F] | somme = 100 % par catégorie ; payeur personne ou organisme ; débiteur ; date d'effet ; impacts mandat affichés | `payeurs.modifies` (remplace `foyer.payeur_defini` [F]) | DOS-07 |
| ADM-51 | Enregistrer l'assurance scolaire de l'année | ADF · `personnes.gerer` ; PAR (dépôt) | pièce ; garanties ; vérification | `eleve.assurance_enregistree` | — |
| ADM-93 [personas] | Définir une répartition **« au foyer du jour »** pour une catégorie de frais (cantine selon la semaine de garde) | ADF, CPT · `payeurs.definir` | catégories à consommation datée seulement (restauration, garderie, études) ; calculée depuis la résidence et ses exceptions ; les frais forfaitaires gardent leurs parts (R-A21) | `payeurs.modifies` | — |
| ADM-94 [personas] | Déterminer la **fratrie** selon une règle (tarif, priorité, envois groupés) | SYS (port `fratrie`) ; ADF (consultation) | critères : même titulaire, même foyer de résidence, même payeur ; la règle est choisie par usage et affichée (R-A22) | — | — |

### 9.6 Destinataires, propositions, fiche de rentrée, qualité, exports

| ID | Action | Acteur · droit | Règles clés | Événements | Remplace |
|---|---|---|---|---|---|
| ADM-60 | Calculer et **expliquer** les destinataires d'un envoi (aperçu « qui recevra ») | tous les émetteurs (8, 16, 7, 24) | règles §8.12 ; raison par destinataire | — | — |
| ADM-61 | Proposer un changement (coordonnée, adresse, contact, remise, situation) | PAR, TIE (soi), ELV | pièce selon champ ; « aussi chez B » (D-013) | `dossier.proposition_deposee` | DOS-10 |
| ADM-62 | Décider une proposition (tout ou partie) | ADF · `dossier_admin.propositions.traiter` | nature choisie (correction / changement) ; refus motivé visible du demandeur | `dossier.proposition_decidee` | — |
| ADM-63 | Ouvrir, relancer, clore une campagne de fiche de rentrée | ADF · `fiche_rentree.gerer` | sections ; relances plafonnées (socle 8) ; fiches par titulaire, multi-enfants | `fiche_rentree.ouverte` / `close` | — |
| ADM-64 | Remplir la fiche de rentrée (tous ses enfants) | PAR | « rien n'a changé » par section ; modifications → propositions ou écritures directes selon le champ | `fiche_rentree.envoyee` | — |
| ADM-65 | Traiter les fiches de rentrée (file, en lot) | ADF · `dossier_admin.propositions.traiter` | une décision par proposition ; « tout accepter » pour les champs sans risque | `fiche_rentree.traitee` | — |
| ADM-66 | Consulter la qualité des données familles | ADF, DIR · `responsables.lire` | alertes §15 | — | — |
| ADM-67 | Exporter (listes de responsables, étiquettes, publipostage, liste électorale) | `dossier_admin.exporter` | colonnes limitées aux groupes autorisés ; adresses protégées exclues ou remplacées par « adresse protégée, voir direction » ; consentements respectés (associations) ; journalisé | `export.realise` | — |
| ADM-68 | Paramétrer le domaine | ADM · `dossier_admin.parametrer` | dans les bornes du logiciel | `parametrage.dossier_admin_modifie` | — |
| ADM-69 | Extrait des données d'une personne (droit d'accès RGPD) | DIR · `dossier_admin.exporter` ; PAR (soi, à terme) | tout ce qui la concerne dans l'établissement, sans les données d'autres personnes ni les motifs scellés qui ne la concernent pas | `export.realise` | — |
| ADM-70 | Intégrer la déclaration d'une inscription (port pour le socle 3) | SYS (3) | §5.3 ; choix humains | `personne.creee`, `eleve.lien_updated`, `foyer.cree` | INS-05 (partie) |
| ADM-95 [personas] | **Fiches de rentrée papier** pré-remplies et saisie des retours par différences | ADF · `fiche_rentree.gerer` | publipostage par titulaire (tous ses enfants) avec un code de rattachement ; à la saisie, seules les cases modifiées ; signature scannée ; même traitement qu'une fiche en ligne | `fiche_rentree.envoyee` | — |
| ADM-96 [personas] | Détecter et traiter un **conflit de déclarations** entre titulaires | SYS ; ADF, DIR · `dossier_admin.propositions.traiter` | deux propositions contradictoires (résidence, remise, contacts, payeurs) : aucune appliquée ; statut « en conflit » ; arbitrage humain, pièce si besoin ; aucun des deux ne voit la déclaration de l'autre (R-A1) | `dossier.conflit_declarations` | — |
| ADM-97 [personas] | **Signaler un changement de situation constaté** par un personnel (« il vit chez son père depuis un mois ») | personnel · `responsables.lire` | proposition source `personnel`, jamais écriture directe ; marquée sensible si besoin | `dossier.proposition_deposee` | — |
| ADM-98 [personas] | Ouvrir un **accès exceptionnel motivé** (« bris de glace ») aux adresses et à la situation d'un élève | personnel désigné · `responsables.acces_urgence` | comme `02_ELEVE.md` ELV-44 ; ouvre F2 et F3, **jamais F4** (adresse protégée) sans la direction (R-A32) | `acces.urgence_ouvert` | — |
| ADM-99 [personas] | Enregistrer et suivre une **demande d'exercice des droits** (accès, rectification, opposition, effacement, limitation) | DIR, DPO · `dossier_admin.exporter` | preuve d'identité ; échéance légale **[V]** (RGPD art. 12.3 : un mois, prolongeable de deux mois avec information dans le premier mois ; VERIF R20) ; réponse tracée ; un parent qui demande les données de son enfant n'obtient pas celles de l'autre parent (R-A31) | `rgpd.demande_recue` / `_repondue` | — |
| ADM-100 [personas] | **Identifier un appelant** à l'accueil (« je suis la mère de Léo ») avant de recevoir une déclaration | ACC, VS · `responsables.lire` | le numéro appelant ou le nom donné est comparé aux responsables et contacts **de cet élève** ; réponse « correspond à : mère, titulaire » / « ne correspond à aucun responsable » sans afficher d'autre donnée ; la déclaration d'absence (socle 12) garde la mention « appelant vérifié / non vérifié » (R-A42) — action de terrain absente de Charlemagne, présente dans Next.js (D-023) | `responsables.appelant_verifie` | — |
| ADM-101 [personas] | Demander un changement **sans compte** (adresse, téléphone) par un formulaire public | personne liée sans compte | e-mail ou mobile vérifié par code ; le formulaire ne montre rien du dossier ; crée une proposition, jamais une écriture ; anti-abus comme les routes publiques du socle 3 — repris du guichet de demandes de Next.js | `dossier.proposition_deposee` | — |
| ADM-102 [personas] | Déclarer, **par jour**, la sortie seul et la présence aux temps périscolaires (midi, soir, étude, garderie) | PAR (titulaire avec `autoriser_sorties`) ; ADF | grille hebdomadaire datée ; distincte des personnes autorisées ; lue par l'accueil (« sort seul aujourd'hui ? ») et par les socles 14, 40, 43 (R-A43) — présente dans le dossier Next.js | `eleve.remise_modifiee` | — |

---

## 10. Ce qui relève du logiciel, ce qui relève du paramétrage

### 10.1 Tableau

| Sujet | Logiciel (identique partout) | Paramétrage de l'établissement |
|---|---|---|
| Structure : personnes, foyers, adresses, liens, versions, historique, transactions | ✔ | — |
| Natures de lien, qualités juridiques, catalogue des droits du lien | codes et garde-fous (§8.6) | libellés, natures proposées à la saisie |
| Droits par défaut d'un lien selon nature et qualité | bornes (jamais `signer` sans autorité, jamais rien pour un lien suspendu ou interdit) | valeurs par défaut |
| Règles de destinataires | natures d'envoi, bornes (§8.12) | choix dans les bornes (« prévenir les deux parents d'une absence ») |
| Pièces exigées | catalogue des types (socle 7) | exigée / attendue / non, par fait |
| Saisie par la famille | champs jamais modifiables par la famille (qualité juridique, interdictions, payeurs) | par champ : non / proposer / modifier |
| Personnes autorisées à récupérer | mécanisme, interdiction prioritaire | pièce d'identité exigée, photos des personnes autorisées, autorisations ponctuelles sans validation, nombre maximal |
| Contacts d'urgence | mécanisme | nombre minimal et maximal |
| Catégories de frais pour les payeurs | catégories de base | catégories supplémentaires rattachées à une catégorie de base |
| Collecte facultative (profession, nom de naissance des responsables, assurance, géolocalisation) | champs disponibles, protégés | activée ou non (profil d'établissement) |
| Fiche de rentrée | sections disponibles, multi-enfants, « rien n'a changé » | sections actives, obligatoires, dates, relances, message |
| Consentements administratifs | objets, versionnement des textes | textes affichés |
| Normalisation des adresses | base locale, jamais de remplacement silencieux | activée ou non |
| Durées de conservation | bornes validées par le DPO | — (aucune durée légale réglable localement) |
| Délai d'annulation d'une fusion de personnes | 30 jours [F] | — |

### 10.2 Préréglages par profil d'établissement [R]

Le champ `statut_juridique` proposé par `02_ELEVE.md` §9.1 sert ici aussi.

| Réglage | Public 2ᵈ degré | Privé sous contrat | Hors contrat | Premier degré |
|---|---|---|---|---|
| Source qui fait foi pour responsables et adresses | SIECLE via AAF (propositions « à corriger dans SIECLE ») [S, `INTEROPERABILITE_NATIONALE.md` §2.7] | ScolIA (remontée BEE) | ScolIA | ONDE (hors périmètre initial) |
| Saisie famille des coordonnées | proposer (ScolIA ne peut pas écrire dans SIECLE) | modifier (après vérification) | modifier | proposer |
| Profession (PCS) des responsables | activée si exigée par la remontée [NV] | activée si BEE l'exige [NV] | désactivée | [NV] |
| Payeurs et catégories | restauration, internat, voyages | toutes (contribution des familles) | toutes | restauration, périscolaire |
| Fiche de rentrée | activée | activée | activée | activée |
| Pièce exigée pour une autorité exclusive | attendue | attendue | attendue | attendue |

---

## 11. Échanges avec l'extérieur

| Flux | Sens | Ce qu'il apporte ou emporte | Comment il entre ou sort [R] | Socle porteur |
|---|---|---|---|---|
| **AAF** (public : SIECLE → AAF → ENT), fichier des responsables | entrant | personnes, adresses, liens, parenté, niveau de responsabilité, payeur, contact prioritaire, hébergement [S pour l'existence du fichier ; champs : F Next pour l'import SIECLE, à confirmer pour l'AAF NV] | **propositions** du socle 4, rapprochement par identifiant de personne SIECLE puis par identité ; application automatique seulement si « SIECLE fait foi » et pas de modification locale plus récente (même règle que `02_ELEVE.md` §8.3) | 9 + 4 |
| **SIECLE BEE** exports | entrant | idem | idem | 9 + 4 |
| **Remontée BEE** (privé) | sortant | responsables des élèves (identité, adresse, parenté, profession si exigée, payeur) [NV : contenu exact] | depuis les données courantes **validées** ; adresses protégées : règle à définir avec l'administration (Q-A11) ; rejets → propositions de correction | 9 |
| **ONDE** | entrant/sortant | premier degré | hors périmètre initial | 9 |
| **ÉduConnect** | entrant (connexion) | un compte responsable national ; activation par **téléphone ou e-mail** connus de SIECLE [S, `INTEROPERABILITE_NATIONALE.md` §4] | rattachement par **clé de jointure** importée (`AAF_JOINTURE`) à une personne existante ; jamais de création ; la qualité des coordonnées du socle 4 conditionne l'activation côté public | 1 + 9 |
| **Base Adresse Nationale, COG INSEE** | entrant (référentiel) | normalisation des adresses, codes communes, pays | **copie locale** versionnée, mise à jour périodique ; licence à vérifier [NV] ; pas d'appel en ligne avec l'adresse saisie par défaut | 9 (IOP-L0) |
| **Ancien logiciel** (reprise) | entrant, une fois | personnes, foyers, coordonnées | lot de reprise → propositions en masse ; rapport des rapprochements | 9 |
| **Autre établissement ScolIA** | entrant | coordonnées mises à jour par le parent | **seulement** à l'initiative du parent (D-013), en proposition | 4 (couche ENT) |
| **Préinscription** (socle 3) | entrant | déclaration de responsables | `integrer_declaration` à la validation | 3 → 4 |
| **Fournisseurs d'envoi** (e-mail, SMS OVH D-004) | entrant (retours) | rebonds, numéros invalides, désinscriptions | socle 8 écrit la délivrabilité (ADM-06) | 8 |
| **Mairies** | sortant | élèves par commune de résidence (rentrée, mutations) [S, `03_INSCRIPTIONS.md` §4.5] | socle 3 produit, socle 4 fournit `commune_residence` ; adresse protégée : commune seulement si la direction l'autorise | 3 |
| **Transport scolaire** (région, collectivité) [H] | sortant | élèves, adresse ou arrêt, responsables | seulement avec consentement `diffusion_transport` ou base légale vérifiée [NV] | 43 |
| **Associations de parents d'élèves** | sortant | nom et coordonnées des parents | seulement les parents ayant consenti (§8.3) | 46 |
| **Banque** (IBAN, mandats) | — | — | socle 25 ; le socle 4 ne touche jamais un IBAN | 25 |

Règles [R] : **aucune** écriture d'une source externe sans proposition ; un import ne supprime jamais une personne, un lien ou une interdiction locale ; une **interdiction ou une adresse protégée locale l'emporte** toujours sur un import (une donnée sensible saisie localement peut être absente de SIECLE) et un conflit est signalé.

---

## 12. Sécurité et données sensibles

### 12.1 Cloisonnement et intégrité

- `etablissement_id` partout, déduit de la session et de l'adresse (D-012) ; clés étrangères **composées** ; RLS sur toutes les tables du socle (mode permissif puis strict, `docs/RLS_ISOLATION.md`), comme prévu par `02_ELEVE.md` §10.3 (qui liste déjà `ent_personne`, `ent_eleve_lien`, `ent_eleve_contact_urgence`).
- Une personne connue de deux établissements = deux personnes sans lien visible (D-013) ; aucune recherche, aucun rapprochement entre établissements, y compris par e-mail ou téléphone ; la mise à jour « aussi chez B » passe par la couche ENT **au nom de la personne**.
- Tests de séparation étendus à toutes les routes, imports, exports et ports ; garde SQL statique étendue (§5.2).

### 12.2 Groupes de lecture (minimisation)

| Groupe | Contenu | Ouvert par |
|---|---|---|
| F0 désignation | noms des responsables, nature du lien, « joignable / non joignable » | `responsables.lire` |
| F1 joindre | téléphones et e-mails utilisables, ordre d'appel, horaires, langue | `responsables.lire` |
| F2 adresses | adresses postales (hors protégées), résidence et foyer du jour | `responsables.adresses.lire` |
| F3 situation juridique | qualités, autorité, décisions (sans motif), pièces, désaccords, payeurs | `responsables.juridique.lire` |
| F4 protégé | adresses protégées, motifs scellés d'interdiction et de décision | `responsables.protege.lire` |
| R remise | personnes autorisées du jour, interdictions (consigne seulement), contacts d'urgence | `remise.lire` |

Familles : un titulaire voit **ses** coordonnées, ses enfants, le nom et la nature des autres responsables (pas leurs coordonnées, pas leur adresse), les contacts et autorisations qu'il a déclarés, ses propositions ; jamais F3 d'un autre, jamais F4, jamais une interdiction qui le vise (existant [F] étendu).

### 12.3 Adresse protégée, interdictions, enfants placés

Effets **non désactivables** [R] d'une adresse protégée :
- valeurs chiffrées (`valeurs_scellees`) ; colonnes en clair vidées sauf, si la direction l'autorise, la commune (listes mairie) ;
- n'apparaît sur **aucun** document généré pour un autre destinataire (bulletin, facture, convocation), aucun export, aucune liste, aucun écran sans `responsables.protege.lire` ; le document de son propre titulaire peut la porter ;
- toute lecture est journalisée comme accès sensible (`tracer_acces_sensible` [F]) ;
- les exports nationaux qui l'exigeraient sont **bloqués** avec une alerte à la direction (Q-A11) ;
- l'élève concerné reçoit la pastille « confidentialité renforcée » si la direction le choisit (socle 2, `02_ELEVE.md` §7.6 : mesure `adresse_protegee`).

Interdictions : motif scellé ; consigne seule visible de l'accueil ; jamais dans un événement ; vue `remise` journalisée.

### 12.4 Chiffrement applicatif (champs scellés)

| Donnée | Chiffrée ? | Raison [R] |
|---|---|---|
| Noms, coordonnées, adresses courantes ordinaires | non | recherche, envois ; protégés par RLS, droits, minimisation |
| Adresse protégée | **oui** | sécurité des personnes |
| Motifs d'interdiction, de décision sur l'autorité, de désaccord | **oui** | souvent judiciaires |
| Profession (PCS) | non [R, à valider DPO] | rarement lue ; groupe F3 |
| Pièces (jugements, actes) | stockage objet privé, URL courtes (socle 7), catégorie `sensible` | — |
| Charges utiles d'événements | **aucune** donnée personnelle | identifiants, codes, noms de champs |
| IBAN | déjà chiffrés [F `champs_chiffres.rs:28-29`] | socle 25 |

Rappel : pas encore de rotation de clé (`SOCLE1_SECURITE.md` D3 [F cité par `02_ELEVE.md` §10.4]).

### 12.5 Conservation et droits des personnes

- **Aucune suppression** d'une personne, d'un lien ou d'une interdiction par l'interface ; fermeture datée. Purge et anonymisation planifiées selon des **durées à valider par le DPO** (Q-A15) : personnes sans lien actif depuis N années, propositions refusées, coordonnées retirées, fiches de rentrée, consentements (à garder tant que la preuve est utile).
- Droit d'accès : extrait par personne (ADM-69). Droit de rectification : propositions. Droit d'opposition : consentements.
- Responsable de traitement : l'établissement ; ScolIA sous-traitant ; couche « compte personne » : ScolIA (D-013, `IDENTITE_ET_PROFILS_FAMILLES.md` §2). Analyse d'impact probable (données de mineurs, données judiciaires) [R].
- **Données relatives aux décisions de justice** : le RGPD encadre le traitement des données relatives aux condamnations et infractions (art. 10) [NV : un jugement de divorce ou une ordonnance de protection relève-t-il de cette catégorie ? question DPO] ; en attendant : accès F4, chiffrement, journal.

---

## 13. Droits (catalogue `ent_auth::Action`)

### 13.1 Existant [F]

`personnes.gerer` (personnes, liens, contacts ; Admin, Direction, Administratif), `payeurs.definir` (+ Comptabilité), `liens.suspendre` et `personnes.fusionner` (Admin, Direction), toutes à portée « établissement » ; `eleves.gerer` (« élèves, foyers, liens et comptes ») ; `dossier.lire` (fiche) [F `ent-auth/src/actions.rs:25, 93, 106-112, 188-230, 274` ; `ROLES_ET_DROITS.md`]. La lecture des responsables et de leurs coordonnées n'a pas d'action propre : elle suit les sections du dossier.

### 13.2 Actions proposées [R]

| Action | Libellé | Portées admises | Modèle par défaut (= comportement actuel quand il existe) |
|---|---|---|---|
| `responsables.lire` | Voir les responsables et les joindre (F0, F1) | établissement, mes classes, mes groupes, désignés | Admin, Direction, Administratif, CPE, Infirmerie, Compta : établissement ; Prof : mes classes [R, à vérifier contre le comportement actuel de la fiche] ; **Accueil** (modèle nouveau) : établissement |
| `responsables.adresses.lire` | Voir les adresses (F2) | 4 portées | Admin, Direction, Administratif, Compta |
| `responsables.juridique.lire` | Voir la situation juridique et les payeurs (F3) | 4 portées | Admin, Direction, Administratif ; CPE (sans les payeurs : la vue sépare) [R] |
| `responsables.protege.lire` | Voir les données protégées (F4) | établissement | Admin, Direction |
| `responsables.protege.gerer` | Protéger une adresse | établissement | Admin, Direction |
| `personnes.gerer` [F] | Personnes, coordonnées, foyers, adresses, résidence, liens (hors qualité juridique), contacts, assurance | établissement, **sites désignés** (extension proposée par `02_ELEVE.md` Q-E9) | inchangé |
| `liens.juridique.gerer` | Qualité juridique, autorité, droits du lien, désaccords | établissement | Admin, Direction, Administratif [R] (aujourd'hui inclus dans `personnes.gerer` : bascule à l'identique) |
| `liens.suspendre` [F] | Suspendre, rétablir | établissement | inchangé |
| `remise.lire` | Voir qui peut récupérer l'élève, interdictions (consigne), contacts d'urgence | 4 portées | Admin, Direction, Administratif, CPE, Infirmerie ; Accueil ; Prof : mes classes [R] (sorties, voyages) |
| `remise.gerer` | Autorisations de remise, validation des propositions | établissement, sites désignés | Admin, Direction, Administratif, CPE |
| `remise.interdire` | Poser et lever une interdiction | établissement | Admin, Direction |
| `payeurs.definir` [F] | Payeurs et répartition | établissement | inchangé |
| `personnes.fusionner` [F] | Fusionner, défusionner | établissement | inchangé |
| `personnes.identifiants.gerer` | Identifiants des personnes | établissement | Admin, Direction, Administratif |
| `dossier_admin.propositions.traiter` | Décider les propositions, traiter les fiches de rentrée | établissement, sites désignés | Admin, Direction, Administratif |
| `fiche_rentree.gerer` | Campagnes de fiche de rentrée | établissement | Admin, Direction, Administratif |
| `dossier_admin.exporter` | Exports nominatifs, publipostage, liste électorale, extrait RGPD | établissement | Admin, Direction, Administratif |
| `dossier_admin.parametrer` | Paramétrage du domaine | établissement | Admin |

Règles [R] :
- **Compatibilité** : modèles par défaut qui reproduisent exactement les droits actuels (test oracle comme `r2_modele_identique_aux_capacites_historiques` [F]) ; `personnes.gerer` détenu ⇒ `liens.juridique.gerer` accordé dans les modèles existants, pour ne rien retirer.
- **Inclusions explicites** : `personnes.gerer` inclut `responsables.lire` et `responsables.adresses.lire` ; `liens.juridique.gerer` inclut `responsables.juridique.lire` ; `remise.gerer` inclut `remise.lire` ; `responsables.protege.gerer` inclut `responsables.protege.lire`. Aucune autre inclusion implicite.
- `responsables.protege.*` et `remise.interdire` : portée « établissement » seulement, et ne se donnent que par qui les détient (anti-escalade inchangée).
- **Familles, élèves, autres personnes liées** : hors rôles libres (D-017). Leurs droits viennent du **lien actif confirmé**, de sa qualité et des droits du lien (§8.6) ; pour soi-même (coordonnées, préférences, consentements) : être la personne. Un beau-parent, un grand-parent ou un contact avec un compte n'a **que** les droits accordés sur son lien.
- Les routes ne font qu'un pré-filtre ; le service refait le contrôle précis, comme en R2 [F].

---

## 14. Événements

**Charge utile : identifiants, codes, dates, noms de champs ; jamais de nom, d'adresse, de téléphone, d'e-mail, de motif.** Les noms existants sont conservés [F `SOCLE2_DOMAINES.md` §6.2] ; les nouveaux suivent le style français.

### 14.1 Émis

| Événement | Charge utile | Consommateurs [R] |
|---|---|---|
| `personne.creee` [F], `personne.identite_modifiee` | `personne_id`, nature, champs | recherche, chronologie |
| `personne.coordonnees_modifiees` [F], `personne.coordonnee_invalide` | `personne_id`, types, champs | 8 (canaux), 24 (si usage facturation), qualité |
| `personne.preferences_modifiees`, `personne.consentement_modifie` | `personne_id`, codes | 8, 46, 43 |
| `personne.decedee` | `personne_id`, date | 8 (annuler relances), 24, 16, 46, tous les envois programmés |
| `personne.invitee`, `personne.compte_rattache` [F], `lien.confirme` | identifiants | 1, 8 |
| `personne.fusionnee` / `defusionnee` [F] | identifiants | domaines qui gardent des caches |
| `foyer.cree`, `foyer.membres_modifies`, `foyer.separe` | `foyer_id`, foyers issus | 24 (débiteurs), 16 |
| `foyer.adresse_changee`, `foyer.adresse_corrigee` | `foyer_id`, date d'effet, **commune avant/après** (code), élèves résidents | 3 (listes mairie : mutations), 24 (tarif par commune), 43 (transport), 40 |
| `adresse.protection_changee` | `foyer_id`, posée/levée | 7, 24, exports, 2 (confidentialité) |
| `eleve.residence_changee` | `eleve_id`, modes | 12, 43, 8 |
| `eleve.lien_updated` [F], `lien.qualite_changee`, `lien.droits_modifies`, `lien.clos` | `lien_id`, `eleve_id`, qualité avant/après, codes de droits | 8, 16, 18, 46, droits famille |
| `lien.suspendu` / `lien.retabli` [F] | identifiants | idem |
| `eleve.desaccord_declare` / `_leve` | `eleve_id`, objet, `objet_ref` | 3, 18, 42 (bloquer les démarches) |
| `eleve.contacts_urgence_changes` [F], `eleve.remise_modifiee`, `eleve.interdiction_changee` | `eleve_id`, posée/levée, objets | 14, application mobile (cache), 34 |
| `payeurs.modifies` (succède à `foyer.payeur_defini` [F]) | `eleve_id`, catégories, date d'effet | 24, 25 |
| `eleve.assurance_enregistree` | `eleve_id`, année, statut | 42 |
| `dossier.proposition_deposee` / `_decidee` | `proposition_id`, cible, source, décision | 8 (accusé et décision au demandeur), qualité |
| `fiche_rentree.ouverte` / `envoyee` / `traitee` / `close` | identifiants, compteurs | 8, pilotage |
| `personne.identifiant_change` | système, statut | 9 |

### 14.2 Consommés

| Événement | Producteur | Réaction du socle 4 |
|---|---|---|
| `inscription.validee` [F] | 3 | rien d'automatique : l'intégration passe par le port appelé **dans** la transaction de validation ; ouverture de la fiche de rentrée pour les nouveaux (INS-38) |
| `eleve.majeur` | 2 | ADM-36 (liens parentaux), demande de choix à l'élève |
| `eleve.capacite_changee` | 2 | droits du représentant d'un majeur protégé |
| `eleve.confidentialite_changee` | 2 | masquage de la résidence et des adresses selon les mesures |
| `eleve.sortie` [F] / `scolarite.closed` | 2 | liens restent (historique) ; remise et contacts fermés à la date ; plus d'envois hors solde (24) |
| `eleve.fusionne` | 2 | liens, contacts, remise, payeurs suivent le registre de fusion |
| `notification.rebond` (à créer, socle 8) | 8 | ADM-06 |
| `document.signe` / `document.depose` (socle 7) | 7 | pièce rattachée à la proposition ou au fait en attente |

---

## 15. Écrans clés

Leptos 0.8 (`docs/FRONTEND_LEPTOS.md`), actions **proposées par le serveur** (fin du calcul côté client relevé par `02_ELEVE.md` §2.2), design system maison.

| Écran | Route [R] | Contenu | Pour qui |
|---|---|---|---|
| **Onglet « Responsables » de la fiche élève** | `/web/eleves/{id}/responsables` | **cartes par foyer** (adresse, membres, enfants résidents) ; frise de **résidence** (aperçu des quatre semaines : « cette semaine chez Papa ») ; pour chaque personne : nature, qualité juridique (pastille), droits (icônes), coordonnées avec état de délivrabilité, préférences, payeurs ; contacts d'urgence ordonnés ; personnes autorisées ; **bandeau rouge** s'il existe une interdiction (consigne seulement) ; alertes de qualité ; actions du serveur | secrétariat, direction ; vue réduite selon F0–F4 |
| **Fiche personne** | `/web/personnes/{id}` | identité et historique, coordonnées, foyer, **tous les enfants** liés dans l'établissement et rôle pour chacun, compte, consentements, propositions | secrétariat |
| **Ajouter un responsable** | assistant dans l'onglet | recherche d'abord (homonymes, « est-ce la même personne que la mère de Jules ? ») → nature → qualité (pièce si besoin) → foyer (existant ou nouveau, adresse) → coordonnées → droits par défaut affichés et ajustables → récapitulatif → une seule commande | secrétariat |
| **Séparation** | assistant | foyer d'origine → qui part où (glisser) → nouvelles adresses → résidence des enfants (principale, alternée avec rythme) → payeurs → second e-mail → invitations → **récapitulatif avant / après** par enfant → confirmation ★ | secrétariat |
| **Déménagement** | action sur un foyer | nouvelle adresse (recherche locale) → date d'effet → impacts listés (enfants, commune, transport, tarif) → confirmation | secrétariat ; famille en proposition |
| **Décision sur l'autorité** | action sur un lien | type de décision → pièce → date → droits recalculés (aperçu) → information des intéressés (case à cocher, rédaction humaine) ★ | direction, secrétariat |
| **Remise (« qui peut le chercher ? »)** | `/web/accueil/remise` + appli mobile | recherche en trois lettres ou badge ; photo de l'élève ; **interdictions en tête** (rouge, consigne, « appeler la direction ») ; autorisés **aujourd'hui** (photo si activée, pièce à vérifier) ; titulaires ; bouton appeler ; « enregistrer la remise » (socle 14) | accueil, vie scolaire, infirmerie |
| **Appeler la famille** | composant partout (absence, incident, infirmerie) | liste ordonnée par `joignables` avec horaires et langue ; journal de l'appel (socle 12/16) | vie scolaire, infirmerie, PP |
| **Propositions** | `/web/dossier-admin/propositions` | file filtrable ; avant / après ; pièce ; accepter tout ou partie, nature, refuser avec motif ; en lot | secrétariat |
| **Fiche de rentrée — suivi** | `/web/dossier-admin/fiche-rentree` | taux de retour par classe, relances, fiches à traiter | secrétariat, direction |
| **Fiche de rentrée — famille** | `/web/famille/fiche-rentree` + appli | **un écran par section, tous les enfants ensemble** ; « rien n'a changé » ; photo de l'attestation d'assurance ; signature des sections qui le demandent (socle 18) ; 5 minutes visées | responsables |
| **Portail famille : ma famille** | `/web/famille/famille` | mes coordonnées (modifiables selon paramétrage), mon foyer, mes enfants et mes droits pour chacun, autres responsables (nom et nature seulement), contacts et personnes autorisées que j'ai déclarés, autorisation ponctuelle (« ma voisine vendredi »), préférences, consentements, mes demandes en cours ; « aussi chez l'établissement B ? » (D-013) | responsables |
| **Élève majeur : mes parents** | `/web/moi/parents` | ce que chaque parent reçoit, modifiable | élève majeur |
| **Qualité des données familles** | `/web/dossier-admin/qualite` | tuiles : élèves sans autorité active ; parents séparés sans e-mail ; e-mails en rebond ; mobiles invalides ; adresses non normalisées ; noms et prénoms mélangés (reprise) ; personnes interdites aussi autorisées ; payeurs ≠ 100 % ; payeur sans débiteur ; alternance sans rythme ; contacts d'urgence manquants ; fiches de rentrée non rendues ; personnes décédées encore destinataires ; liens non confirmés depuis N jours. Chaque tuile ouvre la liste et l'action | secrétariat, direction |
| **Aperçu des destinataires** | composant des écrans d'envoi (8, 16, 7) | « Ce courrier partira à : … (raison) » ; exclus et pourquoi | tous les émetteurs |
| **Doublons de personnes et fusion** | `/web/personnes/doublons` | API existante [F] ; comparaison, aperçu, annulation | direction |
| **Paramétrage** | `/web/direction/parametrage/dossier-admin` | profil ; natures et libellés ; droits par défaut (matrice nature × droit) ; règles d'envoi ; pièces ; saisie famille par champ ; remise ; fiche de rentrée ; consentements | administrateur |
| **Mobile** (appli unique, D-003) | — | personnel : remise, appeler la famille, contacts d'urgence ; famille : ma famille, autorisation ponctuelle, fiche de rentrée, préférences | selon droits |

Critères d'ergonomie [R] : clavier complet, lecteur d'écran, 375 px ; aucune action sensible sans récapitulatif ; tout masquage expliqué (« adresse protégée — voir la direction ») ; recherche de remise < 1 s en conditions réelles (accueil à 16 h 30).

---

## 16. Plan de migration depuis l'existant

Règles : migrations **additives** ; reprise idempotente ; les données imprévues sont rangées « à traiter » et signalées, jamais rejetées ; répétition sur base de labo et jeux anonymisés, **jamais** sur la base de production (interdite) ; ancienne interface `ui.rs` figée ; colonnes héritées conservées en projection (triggers) tant que des lecteurs existent. Numéros dans la plage **260–279** ; ordre de fusion à coordonner avec 240–249 (socle 2, E0–E1) et 250–254 (socle 3, I0), qui touchent les mêmes tables (clés composées).

| N° | Étape | Contenu | Reprise / compatibilité | Vérification |
|---|---|---|---|---|
| 260 | Clés et fuseau | `UNIQUE (etablissement_id, id)` sur `ent_personne`, `ent_foyer`, `ent_eleve_lien` ; clés composées sur `ent_eleve_lien`, `ent_eleve_contact_urgence`, `ent_personne_fusion` (`NOT VALID` puis `VALIDATE`) ; vue `ent_eleve_lien_actif` recalculée dans le **fuseau de l'établissement**, Europe/Paris par défaut ([personas] R-A38) | orphelins listés, pas supprimés | requête d'orphelins à zéro en labo ; test de bord de journée (dont un établissement à UTC+4) |
| 261 | Natures de lien | `ent_lien_nature` (catalogue) ; `nature_code`, `nature_precision`, `qualite`, `version_id`, `ordre_contact`, `organisme_id`, `confirme_par_personne_le` sur `ent_eleve_lien` ; `ent_eleve_lien_version` | `lien` texte → code par table de correspondance (mere, pere, tuteur…, sinon `autre` + précision = texte d'origine) ; qualité depuis `autorite_parentale` (vrai → `titulaire_conjoint`, faux → `sans_autorite`) ; version 1 `reprise` ; `confirme_par_personne_le` = date de reprise pour les liens existants avec compte (comportement inchangé) | aucun lien sans code ; `lien` et `autorite_parentale` égaux aux projections |
| 262 | Droits du lien | `ent_lien_droit_catalogue`, `ent_eleve_lien_droit` ; trigger qui maintient les quatre colonnes `peut_*` | quatre colonnes → lignes `defaut_nature` | égalité colonnes ↔ lignes pour tous les liens |
| 263 | Personnes versionnées | `ent_personne_version` ; projection protégée ; `decede_le`, `pcs_code`, `nom_naissance` | version 1 `reprise` ; noms mélangés détectés (un seul mot dans `nom` contenant un espace et `prenom` vide) → rapport « à séparer » | projection = version pour toutes les personnes |
| 264 | Coordonnées | `ent_personne_coordonnee` ; trigger qui maintient `email`, `telephone`, `telephone_2` en projection | depuis `ent_personne` (source d'origine), puis e-mail et téléphone du **compte** quand ils diffèrent (ajoutés avec `usages = {connexion}` et signalés) ; téléphones normalisés E.164 quand c'est sûr, sinon gardés tels quels « à vérifier » | aucun e-mail perdu ; rapport des divergences compte ↔ personne |
| 265 | Adresses et foyers | `ent_adresse`, `ent_foyer.nature/ferme_le/remplace_par`, `ent_foyer_membre`, `ent_foyer_adresse` | `ent_personne.adresse` (texte) → `ent_adresse` non normalisée (`lignes_libres`) ; membres déduits de `ent_eleve_lien.foyer_id` (personne → foyer le plus fréquent ; conflits listés) ; personnes sans foyer → foyer `personne_seule` **seulement si** elles sont payeuses (sinon aucun foyer) | rapport : personnes dans plusieurs foyers ; adresses à normaliser |
| 266 | Résidence | `ent_eleve_residence` | depuis `foyer_id` des liens titulaires et `garde` : un foyer → `principale` ; deux foyers et `garde = alternee` → `alternee` avec `rythme = libre` (à préciser) ; sinon `principale` = foyer du lien d'ordre 1, signalé | élèves sans résidence listés |
| 267 | Remise et interdictions | `ent_eleve_remise`, `ent_eleve_interdiction` ; `relation_code`, `du`, `au` sur les contacts | `peut_recuperer = true` → remise `permanente` (personne, statut repris) | une remise par contact `peut_recuperer` |
| 268 | Payeurs et organismes | `ent_organisme`, `ent_organisme_contact`, `ent_categorie_frais`, `ent_eleve_payeur` ; trigger qui maintient `est_payeur` / `part_paiement` | `est_payeur` + `part_paiement` → payeur catégorie `toutes`, débiteur = `foyer_id` du lien (sinon foyer `personne_seule` créé) | sommes = 100 % ; factures inchangées |
| 269 | Préférences, consentements, identifiants | `ent_personne_preference`, `ent_personne_consentement`, `ent_personne_identifiant` | rien à reprendre (Next.js : identifiants SIECLE de personne à importer plus tard par le socle 9) | — |
| 270 | Propositions et fiche de rentrée | `ent_dossier_proposition`, `ent_fiche_rentree_campagne`, `ent_fiche_rentree` | contacts `propose` restent dans leur table (pas de doublon) | — |
| 271 | Désaccords, assurance | `ent_eleve_desaccord`, `ent_eleve_assurance` | — | — |
| 272 | Paramétrage | `ent_param_dossier_admin`, libellés de natures | valeurs par défaut = comportement actuel (droits cochés à vrai pour les natures parentales, comme aujourd'hui) | oracle de comportement |
| 273 | RLS | toutes les tables du socle (permissif puis strict, en deux migrations : 273 et 274) | dépôts en `TenantTx` | tests sous rôle non propriétaire |
| 275 | Bascule des lecteurs | `emails_parents_eleve`, `telephones_parents_eleve`, consommateur `d0.notifications` → port `destinataires` ; contrôles `peut_*` → `droits_lien` | anciennes fonctions conservées comme façades | tests d'envoi : parent sans compte destinataire, foyer du jour |
| 276–279 | Retrait des colonnes héritées | `ent_personne.email/telephone/telephone_2/adresse`, `ent_eleve_lien.lien/garde/foyer_id/peut_*/est_payeur/part_paiement`, `ent_eleve_contact_urgence.peut_recuperer` | **après** bascule de tous les lecteurs (grep à zéro) et validation | migration dédiée, réservée |

---

## 17. Découpage en lots de code (équipe Sonnet)

Chaque lot livre : migrations, dépôt (`ent-db`), service en transaction unique, routes (`ent-api`) et types (`ent-types`), tests Postgres (isolation entre établissements, matrice de droits, cas du §4 concernés), écrans quand le lot en a, documentation (ce document, `PROJECT_STATUS.md`). Revue `review-sonnet` sur chaque lot ; revue sécurité (skill `security-review`) sur A0, A1, A4, A5, A11. Travaux parallèles en sessions ou worktrees séparés (F-007), quatre compilations au plus en même temps (F-006). Effort : **S** ≤ 3 j, **M** 1–2 sem., **L** > 2 sem. [H].

| Lot | Contenu | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|
| **A0 Correctifs et ports pour le socle 3** | migration 260 ; `correspondances` et `integrer_declaration` (sur le schéma actuel, en `*_tx`), sans aucun rapprochement implicite ; un lien vers une personne avec compte = invitation, accès après confirmation ; commande unique `ajouter_responsable` (fin des deux appels) ; dates en Europe/Paris | — ; **débloque le lot I0/I1 du socle 3** | le test du socle 3 « second parent non vérifié ne reçoit aucun accès » passe ; fratrie validée en deux fois = un seul foyer ; test de bord de journée | M |
| **A1 Droits et minimisation** | actions §13, groupes F0–F4 et R, raisons de masquage, oracle | A0 ; socle 2 E1 (même mécanisme) | matrice profils × actions identique à l'existant ; tests par groupe ; rôle « Accueil » de démonstration | M |
| **A2 Personnes et coordonnées** | 263, 264, ADM-01 à 07, 10, 12 ; vérification par code ; normalisation E.164 | A0 | aucune coordonnée perdue ; divergences compte ↔ personne rapportées ; aucune valeur dans les événements | M |
| **A3 Foyers, adresses, résidence** | 265, 266, ADM-20 à 27 ; base adresses locale (après vérification de licence) ; calcul du foyer du jour (pur, testé) | A2 ; IOP-L0 pour le COG | déménagement d'un foyer de 2 adultes et 3 enfants en une commande ; alternance 1/1 et 2-2-5-5 testées sur une année ; adresse sans normalisation possible | L |
| **A4 Liens juridiques et droits** | 261, 262, ADM-30 à 38 ; garde-fous ; décision sur l'autorité avec pièce ; majorité | A1, A2 ; socle 2 E4 (majorité) | les 27 cas du §4 saisissables sans contournement (test de scénario par cas) ; aucun `signer` sans autorité (test) | L |
| **A5 Urgence, remise, interdictions** | 267, ADM-40 à 46 ; écran et route mobile de remise ; journal de consultation | A1, A4 | interdiction prioritaire ; conflit signalé ; consultation tracée ; réponse < 1 s sur 2 000 élèves (labo) | M |
| **A6 Payeurs et organismes** | 268, ADM-50, ADM-11 ; contrat avec le socle 24 (`payeurs`, débiteur) | A3 | sommes par catégorie ; payeur organisme ; factures existantes inchangées (test) | M |
| **A7 Propositions et famille** | 270, ADM-61, 62 ; portail « ma famille » ; « aussi chez B » via la couche ENT (D-013) | A2, A3, A5 | rien d'une source externe sans proposition ; un établissement n'apprend jamais l'existence d'un autre (test) | M |
| **A8 Fiche de rentrée** | 270, ADM-63 à 65 ; sections fournies par 2, 18, 34 (contrat) | A7 | une fiche par titulaire, tous ses enfants ; « rien n'a changé » ; traitement en lot | M |
| **A9 Situations de vie** | ADM-04 (décès), ADM-21 (séparation), ADM-37 (désaccords), 271 | A3, A4, A6 | séparation sans perte (avant / après comparés) ; décès : aucune relance ne part (test) | M |
| **A10 Identifiants et échanges** | 269, ADM-13 ; imports AAF / BEE de responsables **après** spécifications (socle 9) ; assurance (ADM-51) | A7 ; socle 9 | import de fichier d'exemple officiel → propositions ; interdiction locale jamais écrasée | M |
| **A11 Destinataires** | 275, ADM-60 ; port `destinataires` et `joignables` ; documents nominatifs par destinataire (contrat socle 7) ; bascule des lecteurs | A4, A5, A6 ; socle 8 | parent sans compte destinataire ; foyer du jour pour l'absence ; adresse protégée jamais sur le document d'un autre (test) | M |
| **A12 Écrans** | sous-lots : A12a onglet responsables + ajout ; A12b fiche personne + doublons ; A12c séparation + déménagement + décision ; A12d remise (web et accueil) ; A12e propositions + qualité ; A12f fiche de rentrée (personnel et famille) ; A12g portail famille + élève majeur ; A12h paramétrage | lots back | rendu et parcours contre le vrai backend ; clavier, 375 px, axe-core | L (total) |
| **A13 Mobile** | remise, appeler la famille, ma famille, autorisation ponctuelle, fiche de rentrée | A5, A7, A8 ; socle 11 | contrat JSON testé ; parité des droits | M |
| **A14 RLS, conservation, exports** | 273, 274, ADM-67, 69 ; purges après décision DPO | A1 ; DPO (Q-A15) | RLS stricte testée ; exports limités aux droits ; adresses protégées exclues | M |
| **A15 Retrait des projections** | 276–279 | tous | grep des lectures à zéro | S |

Ordre recommandé : **A0** (débloque le socle 3) → A1 → A2 → (A3, A4 en parallèle) → (A5, A6) → A7 → (A8, A9, A11) → A10 → A12 au fil de l'eau → A13, A14 → A15. Parcours minimal vendable (« rentrée d'un établissement privé ») : A0, A1, A2, A3, A4, A5, A7, A8, A11 + A12a, d, f, g.

**[personas] Ajouts au lot A0 en cours** : R-A38 (fuseau de l'établissement), R-A39 (champs réservés du contrat, adresse à protéger parquée, « ne pas solliciter »), R-A40 et ADM-82 (confirmation du lien au guichet ou en un geste). Critères de fin ajoutés : test de bord de journée avec un établissement à UTC+4 ; test « une déclaration avec `adresse_a_proteger` n'écrit aucune adresse en clair » ; test « aucune invitation vers une personne `ne_pas_solliciter` » ; test « un professeur-parent confirme son lien sans e-mail d'invitation ».

---

## 17 bis. Ajouts de la passe « personas » [personas]

Origine, personas et gravité : `docs/socles/PERSONAS_02-04.md`. Le lot qui porte chaque règle est indiqué ; **A0** signale le lot en cours.

| # | Règle [personas] | Lot |
|---|---|---|
| R-A1 | **Conflit de déclarations.** Deux propositions de titulaires différents sur le même objet (résidence, remise, contacts d'urgence, payeurs, préférences de l'enfant), contradictoires et ouvertes en même temps : aucune n'est appliquée automatiquement, même si le champ est en `modifier` ; statut `en_conflit` ; arbitrage par le secrétariat ou la direction ; chaque déclarant voit « votre demande est en cours d'examen », jamais la déclaration de l'autre | A7, A8 |
| R-A2 | **Exception de résidence.** `ent_eleve_residence_exception (eleve_id, foyer_id, du, au, declare_par_personne_id, statut ('active','contestee','annulee'))` ; `foyer_du_jour` lit d'abord les exceptions actives ; durée maximale paramétrée ; contestation par l'autre titulaire : statut `contestee`, et les envois du jour vont aux deux | A3 |
| R-A3 | **Vacances dans le rythme.** Champ facultatif du rythme : `vacances: {mode: 'moities' | 'alternees_par_annee' | 'libre', ref}` ; appuyé sur le calendrier des vacances du socle 5 ; sans réglage, « appeler les deux » pendant les congés | A3 |
| R-A4 | **Résidence temporaire.** Mode `temporaire` ajouté à `ent_eleve_residence.mode` ; date de fin obligatoire ; motif codé (hospitalisation d'un parent, hébergement provisoire, autre) | A3 |
| R-A5 | **Domiciliation et absence de domicile stable.** `ent_foyer_adresse.usage IN ('lieu_de_vie','domiciliation')` ; `ent_foyer.nature` reçoit `sans_domicile_stable` ; le courrier va à la domiciliation ; les listes mairie prennent la commune de domiciliation par défaut [NV, à vérifier] ; aucun contrôle ne bloque une inscription faute de justificatif de domicile | A3 |
| R-A6 | **Relais de communication.** Nature `relais_communication` dans `ent_lien_nature`, sans qualité juridique ni droit par défaut ; `joignables` l'affiche après le titulaire, avec la langue ; un relais mineur (élève, frère ou sœur) déclenche une alerte « pas d'enfant interprète pour une information importante » | A4, A11 |
| R-A7 | **Droits portés par l'organisme.** Le lien `service_gardien` est porté par `organisme_id` ; la personne référente est le **contact courant** de l'organisme (`ent_organisme_contact`, fonction `referent`) ; changer de référente ne recrée pas de lien et ne change pas les droits ; destinataires : référente courante et adresse de l'organisme | A4, A6 |
| R-A8 | **Signer le quotidien.** Catalogue des droits du lien : `signer_quotidien` (autorisations courantes, justifications) distinct de `signer_documents` (démarches importantes) ; assistant familial, correspondant et procuration n'ont que `signer_quotidien` ; chaque formulaire du socle 18 déclare le droit qu'il exige ; le logiciel ne qualifie pas juridiquement l'acte, l'auteur du formulaire choisit [NV, à vérifier : actes usuels de l'assistant familial] | A4 (+18) |
| R-A9 | **Interdiction provisoire d'urgence.** Fondement `decision_annoncee` ajouté ; rappel à 48 h si la pièce manque ; à J+7 sans pièce, tâche bloquante pour la direction : confirmer (avec motif) ou lever ; jamais de levée silencieuse | A5 |
| R-A10 | **Propagation des interdictions.** Toute pose ou levée d'interdiction, et toute remise retirée, est visible sur tous les postes d'accueil et dans les caches mobiles **en moins d'une minute** (invalidation par push, socle 11) ; un poste qui ne peut pas confirmer sa fraîcheur affiche « liste non vérifiée depuis HH:MM » en rouge ; test de bout en bout | A5, A13 |
| R-A11 | **Fiche de secours.** Contenu : élèves présents du jour, personnes autorisées du jour, interdictions (consigne seulement), contacts d'urgence ; heure de génération ; filigrane nominatif ; expiration en fin de journée ; consigne de destruction ; journal de génération | A5 |
| R-A12 | **Accord à distance.** Notification push ou SMS aux titulaires ayant `autoriser_sorties` ; le nom déclaré par la personne présente ; photo prise à l'accueil **seulement** si l'établissement l'a activé et que la personne y consent ; délai de réponse paramétré ; la réponse crée une remise ponctuelle tracée ; une interdiction l'emporte toujours | A5 (+14, 11) |
| R-A13 | **Consigne téléphonique.** Rappel obligatoire sur un numéro **enregistré** au dossier ; l'accueil ne note jamais un numéro donné par l'appelant comme numéro de confirmation ; trace de l'appel | A5 |
| R-A14 | **Personne se déclarant responsable.** Jamais de remise ; si la personne est réellement titulaire (père non déclaré), le secrétariat vérifie les pièces (livret de famille, jugement) et crée le lien par ADM-30 ; cette création n'efface aucune déclaration des titulaires déjà connus et la direction est prévenue | A5 |
| R-A15 | **Carte de retrait.** Jeton signé et révocable, aucune donnée lisible ; fonctionne hors ligne avec la liste des révocations en cache ; révocation propagée comme R-A10 ; l'accueil voit la photo de l'élève et le statut du jour | A5 (+14) |
| R-A16 | **Procuration temporaire.** Trois mois au plus, renouvelable ; droits limités à `signer_quotidien`, `justifier_absence`, remise, `recevoir_urgences` ; l'autre titulaire est informé sans détail ; impossible au profit d'une personne visée par une interdiction | A4 |
| R-A17 | **Correspondant d'interne.** Nature `correspondant` ; mandat écrit des titulaires (pièce) ; dates annuelles ; droits comme une procuration, plus `recevoir_informations` si les titulaires le demandent | A4 (+41) |
| R-A18 | **Conflit d'intérêts (personnel et responsable).** Si un agent (socle 6) est aussi responsable, contact, payeur ou ancien membre de foyer d'un élève : par sa casquette professionnelle, il ne lit **pas** F2 à F4 des **autres** personnes liées à cet élève (adresses, situation juridique, données protégées) ; F0, F1 et R restent accessibles avec trace d'accès sensible ; toute écriture sur ce dossier passe par un collègue ; la direction est alertée ; sa casquette de parent est inchangée. Détection : rapprochement personne ↔ agent (`06_PERSONNELS.md`, l'agent est une personne du socle 4) et déclaration (`02_ELEVE.md` ELV-46) | A1 (+6) |
| R-A19 | **Adresse e-mail partagée.** Une même adresse sur deux personnes est permise dans un même foyer ; alerte dès que les foyers se séparent ; la séparation (ADM-21) exige une adresse propre par parent (D-014) | A2, A9 |
| R-A20 | **Répartition des paiements contestée.** Un payeur seul ne peut pas réduire sa part au détriment d'un autre payeur : sa demande est une proposition, appliquée avec l'accord de l'autre ou sur décision (pièce) ; la comptabilité voit la demande, rien ne bascule seul | A6 |
| R-A21 | **Répartition au foyer du jour.** Mode `selon_residence` pour les catégories à consommation datée ; le port `payeurs(eleve, categorie, date)` renvoie le payeur du foyer du jour (exceptions comprises) ; les frais forfaitaires gardent leurs parts ; le socle 24 facture par jour de consommation | A6 (+24) |
| R-A22 | **Port `fratrie(eleve_id, critere, date)`.** Critères : `meme_titulaire`, `meme_foyer_residence`, `meme_payeur` ; renvoie des identifiants seulement ; l'usage choisit son critère (réduction fratrie : paramétrage du socle 24 ; priorité d'admission : socle 3 ; envois groupés : socle 8) | A6 |
| R-A23 | **Enfant du personnel.** Port `responsables_personnels(eleve_id, date)` : vrai si un titulaire est agent engagé à la date (socle 6) ; aucune autre donnée exposée | A6 (+6) |
| R-A24 | **Explication des droits à la famille** (ADM-86). Texte calculé depuis les droits du lien, en langage simple ; les motifs scellés ne sont jamais montrés ; mention « décision de l'établissement, contactez la direction » quand un droit a été retiré par décision | A12g |
| R-A25 | **Majorité préparée.** Sur `eleve.majorite_proche` (J-30, `02_ELEVE.md` ELV-39) : l'élève reçoit les choix d'ADM-38 à l'avance ; ils prennent effet le jour de la majorité seulement ; les titulaires reçoivent un message neutre sur ce qui change | A4 |
| R-A26 | **Élève majeur et son adresse.** Un élève majeur a une fiche personne (`ent_personne.eleve_id`, existant [F]) ; il peut être membre de son propre foyer (`personne_seule`) ; sa résidence est ce foyer ; ses parents ne voient pas cette adresse sauf accord de l'élève | A3, A4 |
| R-A27 | **Documents à cheval sur une date.** Les droits sont évalués à la **date d'envoi**, pas à la date de la période concernée (bulletin du 2ᵉ trimestre envoyé après la majorité, après une décision sur l'autorité) | A11 |
| R-A28 | **Versions sensibles des personnes.** `ent_personne_version` reçoit `sensible` et `valeurs_scellees` (même règle que `02_ELEVE.md` §7.2) ; changement de prénom ou de civilité d'un parent : anciennes valeurs scellées, lisibles avec F4 seulement | A2 |
| R-A29 | **Responsable incarcéré.** Adresse marquée `institutionnelle` ; préférences « courrier seulement » ; aucune remise ; l'autorité ne change que sur décision [NV, à vérifier] | A3, A4 |
| R-A30 | **Responsable majeur protégé.** Son tuteur ou curateur est un contact rattaché **au responsable** (pas à l'élève), destinataire en copie selon la mesure [NV, à vérifier] | A4 |
| R-A31 | **Demandes d'exercice des droits.** Registre `ent_rgpd_demande (personne_id, objet, recue_le, echeance, identite_verifiee, reponse_le, piece_reponse)` ; l'extrait d'un élève demandé par un parent exclut les coordonnées, adresses, motifs et pièces des autres responsables (`02_ELEVE.md` R-E19) ; migrera au socle 27 | A14 (+27) |
| R-A32 | **Bris de glace sur le dossier administratif.** Action `responsables.acces_urgence` ; mêmes règles que `02_ELEVE.md` R-E11 ; ouvre F2 et F3 d'un élève hors portée, jamais F4 sans la direction | A1 |
| R-A33 | **Mise à jour d'un contact par un titulaire.** Un titulaire peut proposer la mise à jour des coordonnées d'une personne **qu'il a déclarée** (contact, personne autorisée) et qui n'a pas de compte ; jamais celles de l'autre titulaire | A7 |
| R-A34 | **Coordonnée de foyer.** `ent_foyer_coordonnee (foyer_id, type ('fixe'), valeur, du, au)` ; `joignables` l'inclut après les coordonnées personnelles ; au déménagement, proposée à la fermeture | A2, A3 |
| R-A35 | **Absences prévenues à une personne autorisée certains jours.** Une remise par jours peut porter `prevenir_absences` si un titulaire le demande (désactivé par défaut) ; la grand-mère qui attend le mercredi est prévenue ce jour-là seulement | A5 (+8) |
| R-A36 | **Parcours famille multilingue.** Portail famille, fiche de rentrée et explications de droits suivent la langue de la personne (`03_INSCRIPTIONS.md` R-I4) ; les décisions et pièces juridiques restent en français | A12 (+29) |
| R-A37 | **Indicateurs sociaux agrégés.** Catégories socioprofessionnelles et autres données sociales exposées au pilotage **agrégées seulement**, avec un seuil de secret (case masquée sous 10 personnes [R]) | A14 (+47) |
| R-A38 | **Fuseau de l'établissement.** Toutes les dates métier du socle (vue `ent_eleve_lien_actif`, dates d'effet, résidence du jour, échéances) sont calculées dans le fuseau de l'établissement (socle 5, STR-03), Europe/Paris par défaut ; remplace la mention « Europe/Paris » du §2.2 n° 4, du §8 et de la migration 260 | **A0** |
| R-A39 | **Contrat `DeclarationResponsables` (v1) avec champs réservés.** Facultatifs dès A0 : `adresse_a_proteger`, `ne_pas_solliciter` (+ référence scellée côté socle 3), `langue`, `besoin_interprete`, `organisme` (titulaire de type service gardien), `nature_code`. Sur le schéma actuel : une adresse à protéger **n'est jamais écrite** dans `ent_personne.adresse` ; elle est parquée scellée dans une proposition réservée à la direction (alerte bloquante d'information) jusqu'au lot A3 ; aucune invitation n'est déclenchée vers une personne `ne_pas_solliciter` ; un champ que A0 ne sait pas encore stocker produit une tâche « à traiter », jamais une perte silencieuse | **A0** |
| R-A40 | **Confirmation du lien.** En plus de l'invitation (ADM-08) : (1) la personne déjà connectée confirme en un geste (« je confirme être responsable de X ») ; (2) le secrétariat confirme au guichet après avoir vu une pièce d'identité (type de pièce et auteur tracés, jamais de copie) ; dans les deux cas `confirme_par_personne_le` est posé et l'origine de la confirmation est tracée | **A0** |
| R-A41 | **Documents en trames prêtes (D-023).** Fiche de rentrée papier, étiquettes et publipostage, fiche de secours, carte de retrait, extrait des données d'une personne, information d'une décision sur l'autorité : 3 ou 4 trames par type, identité de l'établissement seulement ; aucune trame ne porte une donnée que son destinataire ne doit pas recevoir. Réglages lourds du socle signalés pour décision commune : `PERSONAS_02-04.md` §5 bis.2 (S-15 à S-21) | A8, A12 (+7) |
| R-A42 | **Appelant vérifié.** Comparaison du numéro présenté ou du nom donné aux coordonnées des responsables et contacts **de l'élève concerné seulement** (jamais une recherche inverse dans tout l'annuaire) ; un numéro masqué ou inconnu donne « non vérifié », jamais un refus de prendre la déclaration ; une déclaration non vérifiée est signalée à la vie scolaire, qui rappelle sur un numéro du dossier si besoin | A5 (+12, 14) |
| R-A43 | **Sortie seul et temps périscolaires par jour.** `ent_eleve_sortie_jour (eleve_id, jour_semaine, sortie_seul BOOLEAN, midi, soir, etude, garderie, du, au, declare_par)` ; versions datées ; une interdiction ou une absence d'autorisation l'emporte ; cohérence avec le régime (socle 2) et les inscriptions aux services (40, 43) signalée, pas bloquée | A5 (+14) |

**Droits ajoutés** : `responsables.acces_urgence` (aucun modèle par défaut hors Direction). ADM-80, ADM-84 utilisent `liens.juridique.gerer` ; ADM-87, ADM-88, ADM-91 `remise.lire` ; ADM-90 `remise.interdire` ; ADM-92 `remise.gerer` ; ADM-93 `payeurs.definir`. **Événements ajoutés** : `personne.non_doublon_declare`, `personne.identite_verifiee_guichet`, `personne.coordonnee_signalee`, `foyer.coordonnees_modifiees`, `eleve.residence_exception`, `lien.procuration_accordee` / `_revoquee`, `eleve.remise_accord_demande` / `_repondu`, `eleve.carte_retrait_emise` / `_revoquee`, `dossier.conflit_declarations`, `rgpd.demande_recue` / `_repondue` ; charges utiles sans donnée personnelle. **Consommés** : `eleve.decede` (socle 2 : ferme remise, contacts et fiche de rentrée de l'élève, aucun message), `eleve.majorite_proche` (R-A25).

---

## 18. Questions à trancher par le porteur du projet

| # | Question | Options | Recommandation |
|---|---|---|---|
| **Q-A1** | Le foyer devient le **lieu de vie** ; le débiteur de facturation reste un `ent_foyer` par compatibilité (foyer `personne_seule` ou `organisme` pour un payeur extérieur) ; le quotient part au socle 26 | oui / autre modèle | **oui** : aucune table de facturation cassée, notions séparées |
| **Q-A2** | Identifiants de l'**élève** au socle 2, identifiants des **personnes** au socle 4 (corrige `SOCLES.md`) | oui / non | **oui** |
| **Q-A3** | Les envois lisent les **coordonnées de la personne** (le compte n'est qu'une coordonnée) ; un parent sans compte est destinataire | oui / non | **oui**, correctif prioritaire (A0/A2, A11) |
| **Q-A4** | Un lien vers une personne **qui a déjà un compte** n'ouvre aucun accès tant qu'elle n'a pas confirmé (invitation) | oui / non | **oui** (corrige le défaut n° 1 de `03_INSCRIPTIONS.md` §2.2) |
| Q-A5 | Droits par défaut d'un **parent sans autorité** : informé (bulletins, scolarité) sauf décision contraire | informé / rien | **informé**, ajustable par la direction au vu de la pièce [NV : art. 373-2-1 C. civ.] |
| Q-A6 | **Beau-parent** : peut-il recevoir des droits d'accès (messagerie, vie scolaire) ? | jamais / sur choix d'un titulaire / sur choix de l'établissement | **sur demande écrite d'un titulaire, validée par l'établissement**, jamais `signer` ni `bulletins` sans pièce de délégation |
| Q-A7 | Majorité (Q10 de `SOCLE2_DOMAINES.md`, toujours ouverte) | coupure / maintien / choix de l'élève | **maintien temporaire puis choix de l'élève** (délai paramétré, ex. 30 jours), payeurs inchangés ; à valider par le DPO |
| Q-A8 | Absences en garde alternée : prévenir le foyer du jour seulement ou les deux ? | jour / deux | **les deux par défaut**, avec le foyer du jour **en premier** pour l'appel ; réglable |
| Q-A9 | Ne pas collecter situation matrimoniale, nationalité, date de naissance des responsables ; profession seulement si un flux l'exige | oui / non | **oui** (minimisation) ; vérifier ce que BEE et l'AAF exigent [NV] |
| Q-A10 | Normalisation des adresses par **copie locale** de la Base Adresse Nationale (pas d'API en ligne) | locale / API / aucune | **locale** ; aucune au départ si la licence ou le volume posent problème |
| Q-A11 | Adresse protégée et exports nationaux (BEE, AAF en retour, listes mairies) : que transmettre ? | bloquer / commune seulement / tout | **bloquer et alerter la direction** jusqu'à réponse de l'administration ; question à ajouter à `INTEROPERABILITE_NATIONALE.md` §12 |
| Q-A12 | Quotient familial, numéro allocataire, ressources : socle 26 | 4 / 26 | **26** |
| Q-A13 | Photos des personnes autorisées à récupérer | non / paramétrable | **paramétrable, désactivé par défaut**, avec consentement de la personne photographiée |
| Q-A14 | Une demande d'un titulaire contre l'autre (sans jugement) bloque-t-elle la remise ? | oui / non | **non** : signalement à la direction, non bloquant [NV] |
| Q-A15 | Durées de conservation (personnes sans lien actif, coordonnées retirées, propositions refusées, fiches de rentrée, interdictions levées, consentements) | — | **à demander au DPO** ; aucune durée codée avant |
| Q-A16 | Vérification juridique avant les lots A4, A5, A9 : autorité parentale et parent séparé (art. 371-1 s., 373-2 s. C. civ.), information du parent sans exercice, délégation, tiers digne de confiance, placement et anonymat du lieu d'accueil (art. 375-3, 375-7), ordonnance de protection (art. 515-9 s.), tutelle, émancipation, élections des parents, transmission aux associations, assurance scolaire | — | **consultation juridique** ciblée ; ajout à la liste des démarches de `INTEROPERABILITE_NATIONALE.md` §12 |
| Q-A17 | Un contact d'urgence peut-il être aussi un titulaire ? | oui / non | **non** (déjà appelé en priorité) ; les grands-parents, oui |
| Q-A18 | Messagerie : parents séparés = conversations distinctes par défaut | oui / non | **oui** (socle 16) |
| Q-A19 | Fiche de rentrée : obligatoire pour valider la réinscription (socle 3) ? | bloquante / non bloquante | **non bloquante**, relances ; l'établissement peut en faire une condition de réinscription dans le privé |
| Q-A20 | Ordre de fusion des migrations 240–279 (socles 2, 3, 4 touchent les mêmes tables : clés composées, RLS) | — | **un seul lot transverse « clés et RLS »** partagé entre E0 (socle 2), I0 (socle 3) et A0, pour ne pas écrire trois fois les mêmes contraintes |
| **Q-A21** [personas] | Intégrer **dès A0** le fuseau de l'établissement (R-A38), les champs réservés du contrat (R-A39) et la confirmation au guichet ou en un geste (R-A40) ? | maintenant / plus tard | **maintenant** : sinon la protection d'une adresse déclarée à l'inscription arrive après qu'elle a été écrite en clair, et des parents légitimes restent bloqués à la rentrée |
| **Q-A22** [personas] | Conflit d'intérêts (R-A18) : bloquer F2–F4 des autres responsables pour un personnel lié à l'élève, même s'il est secrétaire ? | bloquer / tracer seulement | **bloquer** (un collègue traite le dossier) ; la trace seule n'empêche pas la lecture de l'adresse d'un ex-conjoint |
| Q-A23 [personas] | Répartition « au foyer du jour » pour la cantine (R-A21) | oui / non | **oui, en option** par catégorie ; à confirmer avec la conception du socle 24 |
| Q-A24 [personas] | Procuration temporaire (R-A16) et correspondant d'interne (R-A17) : durée maximale, information de l'autre titulaire | — | 3 mois renouvelables ; information neutre de l'autre titulaire ; à faire valider juridiquement [NV] |
| Q-A25 [personas] | Accord à distance pour une remise (R-A12) : canal (notification, SMS) et délai | — | notification, SMS en secours ; 10 minutes ; jamais à la place d'une interdiction |
| Q-A26 [personas] | Carte de retrait (QR, R-A15) | oui / non | **oui, désactivée par défaut**, activable par établissement |
| Q-A27 [personas] | Familles domiciliées (R-A5) : commune retenue pour les listes mairie | domiciliation / lieu de vie | **domiciliation** par défaut [NV, à vérifier auprès des textes sur l'obligation scolaire] |

---

## 19. Récapitulatif : faits, hypothèses, contradictions

**Faits vérifiés [F]** : `ent_personne` distincte du compte, propre à l'établissement, déclencheurs de cohérence (120) ; lien actif en un point, suspension motivée, dates calculées en UTC (121) ; contacts d'urgence avec `peut_recuperer` et statut `propose` (122) ; fusion réversible de personnes (123) ; payeur et droits fins en colonnes sur le lien (007, 015) ; foyer réduit à un libellé, utilisé comme débiteur par six tables de facturation (024, 027, 030, 032, 075, 078) ; IBAN chiffrés (`champs_chiffres.rs:28-29`) ; commandes du dossier en transaction avec journal et événements sans valeurs (`dossier_personnes.rs:601-622, 1109-1185, 1332-1420`) ; coordonnées écrasées ; adresse en texte libre ; nature du lien en texte libre 1–40 caractères (`dossier_personnes.rs:205-222`) ; e-mails et téléphones des parents pour les envois lus sur `ent_user` (`foyer_repo.rs:565-600`) ; aucun consommateur de `personne.coordonnees_modifiees` (recherche dans `reactions.rs`, `outbox.rs`) ; actions `personnes.gerer`, `payeurs.definir`, `liens.suspendre`, `personnes.fusionner`, `eleves.gerer`, `dossier.lire` (`actions.rs`) ; 21 routes `/dossier/*` (`ent-api/src/dossier.rs:32-56`). Next.js [F Next] : foyer avec adresse structurée, responsable de foyer avec autorité, urgence, payeur et `peut_recuperer` distincts ; import SIECLE des responsables (personne, adresse, lien avec niveau de responsabilité, parenté, payeur, contact prioritaire, hébergement) ; foyer retrouvé par libellé ; états d'accord du second parent dans les fiches de dialogue ; messagerie famille adressée au foyer.

**Non relu ligne à ligne** (cité depuis `SOCLE2_DOMAINES.md` §6.2) : tests S2-1, écrans `modules/dossier/`, fusion de personnes, chronologie.

**Vérification du 2026-11-09** (`docs/VERIF_REGLEMENTAIRE.md` R5, R9, R11-R14, R16, R20) : 372, 372-2, 373-2-1, 375-7, 413-6/413-7, 515-11, D111-8 et le délai RGPD sont [V] ; restent [NV] la délégation, le tiers digne de confiance, la tutelle, le majeur protégé et les droits des parents d'un élève majeur.

**Hypothèses [H] / non vérifié [NV]** : toutes les règles du code civil citées (autorité parentale, information du parent sans exercice, délégation, tiers digne de confiance, placement, ordonnance de protection, tutelle, émancipation, parent mineur) ; transmission des coordonnées aux associations de parents ; électeurs des élections de parents ; assurance scolaire ; codes SIECLE (`NIVEAU_RESPONSABILITE`, `CODE_PARENTE`) et contenu exact des fichiers AAF et BEE pour les responsables ; profession (PCS) exigée ou non par les flux nationaux ; licence et usage de la Base Adresse Nationale et du COG ; format postal ; données de condamnations (RGPD art. 10) ; transport scolaire ; ce que vivent les acteurs et la comparaison avec les logiciels du marché ; efforts des lots.

**Contradictions signalées** :
- `SOCLES.md` §3 (identifiants officiels, dont l'INE, au socle 4) contre `02_ELEVE.md` §4 et §7.3 (INE au socle 2) — Q-A2.
- S2-1 rend possible le parent sans compte, mais les envois lisent les comptes : un parent sans compte ne reçoit rien (`foyer_repo.rs:565-600`) — Q-A3.
- `SOCLE2_DOMAINES.md` DOS-04 (« foyer : adresse, membres ») contre le schéma (libellé seulement) — §8.5.
- `SOCLE2_DOMAINES.md` §6.2 annonçait « coordonnées sur la personne » comme choix ; ce document les sort de la personne (historique, multiples) — §8.4, avec projection de compatibilité.
- Next.js adresse la messagerie au **foyer**, contre D-014 (« comptes parents uniquement, chaque parent a son espace ») — non repris (§2.4).
- `02_ELEVE.md` §7.1 prévoyait les migrations 140–149 puis `03_INSCRIPTIONS.md` 160–169 / 200–219 ; le registre `MIGRATIONS.md` fait foi (socle 4 : 260–279), et les trois socles touchent les mêmes tables pour les clés composées et la RLS — Q-A20.


---

## 20. État du lot A0 (2026-11-06, branche `ent-socle-administratif-a0`)

**Implémenté et testé en local (PostgreSQL local) ; non validé par Florian, non prêt pilote.** Base : `ent-socle-inscriptions-i0` + `ent-mc0hoe`.

| Élément du tableau §16 / §17 | État | Où |
|---|---|---|
| Migration 260 : fuseau | fait : fonction `ent_aujourdhui()` (Europe/Paris), vue `ent_eleve_lien_actif` recalculée | `migrations/260_administratif_a0.sql` |
| Migration 260 : clés composées | fait : `UNIQUE (etablissement_id, id)` sur personnes, foyers, liens ; clés composées lien→personne, contact→personne, fusion→personnes, `NOT VALID` puis validées **seulement s'il n'y a aucun orphelin** (vue de contrôle `ent_dossier_orphelins_v`). Pas de clé composée lien→foyer (la clé existante est en `SET NULL`) | idem |
| Port `correspondances` | fait (`correspondances_tx`) : lecture, e-mail masqué, jamais de rapprochement ; l'e-mail n'est **pas vérifié** (candidat seulement) | `ent-db/src/dossier_personnes_ports.rs` |
| Port `integrer_declaration` | fait (`integrer_declaration_tx`, dans la transaction de l'appelant) : sans choix humain, une déclaration qui a des correspondances est refusée ; foyer existant à confirmer ; une même personne ne sert pas deux responsables | idem |
| Lien vers une personne avec compte = invitation | fait : table `ent_lien_attente` (socle 4, reprend les lignes de `ent_inscription_lien_attente`) ; `confirmer_liens_attente_tx` / `refuser_liens_attente_tx` ; pour la saisie directe, `GET /dossier/mes-liens-en-attente` et `POST …/{id}/reponse` (seule la personne concernée décide) | idem + `ent-api/src/dossier.rs` |
| Commande unique `ajouter_responsable` | fait (`ajouter_responsable_for_actor`, route `POST /dossier/eleves/{id}/responsables/ajout`, une transaction) ; l'écran Leptos l'appelle en une requête ; `lier_personne_for_actor` applique la même règle d'invitation | idem + `web/crates/ent-web-app/src/modules/dossier/actions.rs` |
| Envois aux familles | fait : `emails_parents_eleve` / `telephones_parents_eleve` lisent la **fiche de la personne** (compte en simple repli) et ses préférences (`ent_personne_preference`, sous-ensemble de la table de §8.4 : `canal_email`, `canal_sms`, `courrier_papier`) ; un parent sans compte reçoit ses messages | `ent-db/src/foyer_repo.rs` |
| Inscriptions via les ports | fait pour personnes, foyers, liens, confirmation et refus d'invitation | `ent-db/src/inscriptions.rs` |

**Critères de fin** : « second parent non vérifié ne reçoit aucun accès » ✔ (`second_parent_non_verifie_ne_recoit_aucun_acces`) ; fratrie validée en deux fois = un seul foyer ✔ (`fratrie_validee_en_deux_fois_un_seul_foyer`) ; bord de journée ✔ (`jour_metier_europe_paris_bord_de_journee`) ; parent sans compte destinataire ✔ ; ajout atomique ✔ ; clés composées sans orphelin ✔.

**Reste, noté pour les lots suivants**
- **I1** : `accepter_invitation` écrit encore `ent_personne.user_id` directement (rattachement du compte) ; `imports.rs` (socle 9) insère encore dans `ent_eleve_lien` ; l'ancienne table `ent_inscription_lien_attente` n'est plus écrite (à supprimer après validation) ; l'écran de rapprochement du frontend reste à faire.
- **A2** : vérification de l'e-mail par code (aujourd'hui une adresse saisie dans un dossier validé devient destinataire sans preuve de possession) ; préférences modifiables par la personne ; normalisation E.164.
- **Interprétation** de « aucun envoi sans confirmation humaine pour un envoi manuel » : le lot n'ajoute aucun envoi ; ajouter un responsable ou une invitation `saisie` n'envoie rien (testé : la boîte d'envoi ne bouge pas). Les envois automatiques existants (suites d'un geste humain) sont inchangés.
- Pas d'écran pour « mes liens en attente » (routes et client web faits, UI à faire) ni de notification à la personne invitée par saisie directe.
- RLS sur `ent_lien_attente` et `ent_personne_preference` : lot A14 (comme les autres tables du socle).
- Une fusion de personnes ne déplace pas les liens en attente de la personne absorbée (ils ne s'activent pas : aucune fuite, mais à traiter en A9).

**Revue de sécurité du lot (skill `security-review` + relecture indépendante `review-sonnet`)**
- Corrigés : (H1) quatre lectures du cahier de textes (`cahier.rs` ×3, `cahier_reactions.rs`) lisaient la table brute `ent_eleve_lien` et ignoraient suspension, dates et fusion : elles lisent maintenant la vue `ent_eleve_lien_actif` (point unique) ; (M3) une demande de lien déjà en attente est remplacée par la plus récente (même origine) ou refusée nettement (origine différente) au lieu d'être reprise silencieusement avec des droits périmés.
- **Non corrigés, à décider** (hors périmètre A0 ou choix de conception) :
  - (H2) `rattacher_compte` par le personnel et la fusion de personnes donnent à un compte les liens **déjà actifs** d'une fiche sans confirmation de la personne : à traiter avec les lots A4/A9 (les passer par l'attente) ; **question pour Florian**.
  - (M1) un e-mail ou un téléphone saisi dans un dossier validé devient destinataire sans preuve de possession (et l'e-mail de la fiche prime sur celui du compte) : vérification par code au lot A2 ; les envois ne tiennent pas compte des droits fins du lien (comme avant).
  - (M2) le refus d'invitation par jeton suspend le lien sans journal (aucun compte authentifié pour le signer ; le motif et la date sont sur le lien).
  - (B1) pas d'annulation ni d'expiration des attentes ; (B2) clés étrangères simples sur `ent_lien_attente` et `ent_personne_preference` (clés composées au lot A14 avec la RLS) ; (B3) `accepter_invitation` ne contrôle pas le rôle du compte existant ; (B6) numéros SMS non normalisés (doublons possibles) ; (B7) la vue `ent_eleve_lien_actif` est recréée à chaque démarrage (comme avant), sans reprise des droits d'accès éventuels.

## 21. État du lot A0 bis (2026-11-06, branche `ent-socle-administratif-a0-bis`)

**Implémenté et testé en local ; non validé par Florian, non prêt pilote.** Base : lot A0 + `origin/ENT`. Migrations **265 à 269** ; `MIGRATION_TAG` = `269_inscription_responsable_reserve`. Origine : `PERSONAS_02-04.md` §0 (A0-a à A0-d). Tests : `ent-db/src/tests_administratif_a0bis.rs` (6) et `tests_inscriptions.rs` (4 ajoutés).

| Changement | État | Où / test |
|---|---|---|
| **A0-a** fuseau de l'établissement | fait : table `ent_etablissement_parametre` (fuseau IANA validé, Europe/Paris par défaut), fonction `ent_aujourdhui_etab(etab, instant)`, vue `ent_eleve_lien_actif` recalculée ; `PgEnt::fuseau_etablissement` / `definir_fuseau_etablissement_for_actor` (droit `referentiel.gerer` ; pas d'écran : socle 5, STR-03) | migration 265 ; `fuseau_etablissement_bord_de_journee_deux_fuseaux` (Paris / La Réunion, Kiritimati / Pago Pago sur la vue) |
| **A0-b** contrat `DeclarationResponsables` v1 | fait : `DeclarationResponsable.reserve: ReserveDeclaree` (`adresse_a_proteger`, `ne_pas_solliciter` + référence scellée, `langue`, `besoin_interprete`, `organisme`, `nature_code`), validée avant toute écriture, stockée dans `ent_personne_reserve` (aucun champ perdu). Une adresse à protéger **n'est jamais écrite** dans `ent_personne.adresse` ni dans le dossier d'inscription : parquée scellée (AES-256-GCM, `champs_chiffres`) dans `ent_adresse_protegee`, alerte pour la direction (`GET /dossier/alertes-protection`), ouverture journalisée (`POST /dossier/adresses-protegees/{id}/ouvrir`, `no-store`). Côté formulaire : `ResponsableFormulaireDto.reserve` (jamais rendu à la famille), colonnes migration 269 | migrations 266, 267, 269 ; `declaration_adresse_a_proteger_jamais_en_clair`, `adresse_protegee_d_une_personne_existante_et_reserve_fusionnee`, `second_parent_ne_pas_solliciter_aucune_invitation` |
| **A0-c** « ne pas solliciter » | fait : règle unique `personne_ne_pas_solliciter_tx` ; aucune invitation / demande d'accord du second parent (`inviter`), aucune relance (`relancer_for_actor` refuse), aucun envoi automatique aux familles (`emails_parents_eleve`, `telephones_parents_eleve`), la personne ne voit ni ne confirme en ligne ses liens en attente (ils révéleraient l'élève). Posé par la famille (déclaration, « à confirmer ») ou par la direction (`POST /dossier/personnes/{id}/ne-pas-solliciter`), levé par la direction seule | `ne_pas_solliciter_aucune_sollicitation_ni_envoi`, `ne_pas_solliciter_pose_avant_l_envoi_bloque_l_invitation`, `relance_refusee_vers_une_personne_ne_pas_solliciter` |
| **A0-d** deux voies de confirmation | fait : (1) la personne connectée confirme en un geste (`mes-liens-en-attente` couvre maintenant les demandes d'inscription) ; (2) le secrétariat confirme au guichet (`POST /dossier/liens-en-attente/{id}/confirmer-guichet`, droit `personnes.gerer`, liste `GET /dossier/eleves/{id}/liens-en-attente`). Voie, **type** de pièce (liste fermée) et auteur tracés sur la demande et au journal ; ni copie ni numéro (contrainte SQL : pièce ⇔ guichet) | migration 268 ; `professeur_parent_confirme_son_lien_en_un_geste`, `parent_connecte_confirme_le_lien_d_inscription_en_un_geste`, `confirmation_au_guichet_piece_et_auteur_traces` |

Toutes les nouvelles routes sont dans la matrice d'isolation (`separation.rs`, domaine « élève / dossier »).

**Interprétations à valider** : (a) la référence de « ne pas solliciter » est un texte libre scellé (≤ 200 car.), non un identifiant de pièce du socle 3 ; (b) une déclaration de la famille pose « ne pas solliciter » **immédiatement** (par prudence pour l'enfant) et crée une alerte « à confirmer » pour la direction, qui peut le lever ; (c) l'ouverture d'une adresse parquée la marque « traitée » (le lot A3 la reprendra comme adresse protégée) ; (d) les droits réutilisent `liens.suspendre` (direction) pour les protections et `personnes.gerer` (secrétariat) pour le guichet : pas de nouvelle action.

**Revue de sécurité (skill `security-review`)**
- Corrigé : la levée de « ne pas solliciter » par la direction laissait le marqueur copié sur le dossier d'inscription, qui bloquait encore invitations et relances (test ajouté).
- **Non corrigés, à décider** : (M) les notifications dans l'application (`reactions.rs`, `cahier_reactions.rs`) visent encore un parent protégé qui a un compte et un lien actif — elles ne sont pas des sollicitations sortantes, mais révèlent de l'activité ; (L) la confirmation au guichet n'exige pas de second contrôle (tracée et auditée seulement) ; (L) un parent mal intentionné peut déclarer l'autre « ne pas solliciter » pour supprimer ses invitations : atténué par l'alerte direction et le guichet, jamais par un blocage d'accès ; (L) pas de RLS sur les trois nouvelles tables (lot A14, comme le reste du socle) ; (L) une adresse protégée déjà présente en clair dans une fiche existante n'est pas rétroactivement scellée (lot A3).
- Écrans Leptos : voir `PROJECT_STATUS.md`.


## 22. État du lot A1 « droits et minimisation » (2026-11-07, branche `ent-socle-administratif-a1`)

**Implémenté et testé en local (PostgreSQL local) ; non validé par Florian, non prêt pilote.** Base : lots A0 et A0 bis. Migrations **270 à 272** (273–274 réservées au lot A14, RLS) ; `MIGRATION_TAG` = `272_contact_declarant`. Le lot E1 du socle 2 (même mécanisme) n'étant **pas encore dans le code**, A1 pose le mécanisme générique (vue de groupes pure + point d'application unique) ; E1 devra le **réutiliser**, pas le redéfinir (`ent_auth::minimisation`, §22.2).

### 22.1 Catalogue (§13) et modèles par défaut

15 actions ajoutées à `ent_auth::Action` (53 → 68) : `responsables.lire`, `.adresses.lire`, `.juridique.lire`, `.protege.lire`, `.protege.gerer`, `liens.juridique.gerer`, `remise.lire`, `.gerer`, `.interdire`, `personnes.identifiants.gerer`, `dossier_admin.propositions.traiter`, `fiche_rentree.gerer`, `dossier_admin.exporter`, `dossier_admin.parametrer`, `responsables.acces_urgence`. Portées : les quatre lectures (`responsables.lire`, `.adresses.lire`, `.juridique.lire`, `remise.lire`) acceptent les quatre portées ; toutes les autres : établissement seulement. Inclusions explicites uniquement (`personnes.gerer` ⊃ F0–F2, `liens.juridique.gerer` ⊃ F3, `remise.gerer` ⊃ `remise.lire`, `responsables.protege.gerer` ⊃ F4) ; le moteur de droits (`autoriser_eleve`, `eleves_autorises`, `a_action_etablissement`) les applique désormais (jusqu'ici seule `peut_etablissement` le faisait).

**Matrice historique identique** : test `matrice_historique_inchangee_par_a1` (`ent-auth/src/actions.rs`) — profils × 53 actions d'avant A1 figés, plus `r2_*` existants. Aucun droit historique n'a changé ; seules les 15 actions nouvelles sont ajoutées aux modèles.

**Nouveau profil de base `accueil`** (`Role::Accueil`, « Accueil (modèle nouveau) » du §13.2) : `responsables.lire`, `remise.lire`, `absences.declarer`, `referentiel.lire` — rien d'autre. C'est le « rôle Accueil de démonstration » ; aucun écran de gestion de rôle ne le propose encore (écrans d'administration des rôles : R2).

| Profil | F0 F1 | F2 | F3 | F4 | R | Payeurs |
|---|---|---|---|---|---|---|
| Admin, Direction | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| Administratif | ✔ | ✔ | ✔ | — | ✔ | ✔ |
| CPE | ✔ | — | ✔ | — | ✔ | — |
| Comptabilité | ✔ | ✔ | — | — | — | ✔ |
| Professeur (mes classes) | ✔ | — | — | — | ✔ | — |
| Infirmerie, Accueil | ✔ | — | — | — | ✔ | — |
| Psychologue | — | — | — | — | — | — |
| Titulaire (famille) | soi : ✔ ; autres : F0 seulement | soi | soi | — | ses contacts | soi |

Oracle : `minimisation::tests::oracle_profils_par_groupe` (profil × groupe, depuis `modele_par_defaut`).

**Écarts voulus avec le comportement d'avant A1** (avant : toute personne ayant la section « famille » lisait *tous* les champs d'un responsable) — **à valider par Florian** : (1) le professeur ne lit plus que F0, F1 et R de ses élèves ; (2) la vie scolaire perd adresses et payeurs ; (3) la comptabilité perd la situation juridique (F3) et les contacts d'urgence, garde adresses et payeurs ; (4) le secrétariat (Administratif) perd F4 (motif et référence d'une suspension de lien, aujourd'hui lisibles en clair) ; (5) l'infirmerie **gagne** F0, F1 et R (contacts d'urgence : avant, rien) ; (6) une famille ne voit plus de l'autre responsable que la désignation (avant : aussi autorité, garde, dates, droits du lien) et ne voit que les contacts qu'elle a déclarés ou saisis par l'établissement (avant : tous). Tous suivent le §13.2 et le §12.2 de la conception.

### 22.2 Mécanisme : un seul point

- **Fonction pure** `ent_auth::minimisation` : `vue_personnel(droits, conflit)` et `vue_titulaire(est_moi)` → `VueGroupes` (F0–F4, R, payeurs, indicateur de conflit, liste des groupes fermés **avec leur raison**). Aucune autre règle de minimisation.
- **Service** `ent-db/src/dossier_droits.rs` : `vue_groupes_eleve` (droits par élève, portées résolues par le moteur ; conflit d'intérêts), `lecture_dossier_admin` (responsables, contacts, interdictions filtrés d'après la vue). `responsables_eleve_for_actor`, `contacts_eleve_for_actor` et `fiche_eleve_for_actor` n'ont plus de filtre propre : elles appellent ce point.
- **Raisons de masquage** : `Responsable.masque` (par personne) et `FicheEleve.masquages` (par groupe) → DTO `MasquageDto` (`groupe`, `libelle`, `raison` = `droit_manquant` | `conflit_interet` | `autre_responsable` | `reserve_etablissement`, `message`, `action_a_demander`). Les valeurs masquées sont neutres (vide / faux) : un champ vide n'est jamais ambigu. L'écran fiche élève (Leptos) affiche le message à la place de la valeur et un bandeau « Interdictions en cours ».
- Bris de glace `responsables.acces_urgence` : **action au catalogue seulement** (modèle : Direction) ; le mécanisme temporaire (motif, durée, alerte, revue à 72 h) relève de `02_ELEVE.md` R-E11 / lot E7 et n'est pas implémenté.

### 22.3 Règles [personas] du lot

| Règle | Implémentation | Test |
|---|---|---|
| **R-A18 conflit d'intérêts** | Détection : l'agent a une fiche personne liée à l'élève (lien, même ancien ou suspendu ; payeur ; contact d'urgence) ou une **déclaration** (`ent_conflit_interet` : par l'agent pour lui-même, ou par la direction pour un autre agent ; natures `ex_conjoint`, `ancien_foyer`, `proche`, `autre` ; seule la direction lève, jamais l'agent). Effet : F2, F3, F4 et payeurs **fermés quels que soient ses droits** (raison `conflit_interet`) ; F0, F1, R ouverts avec **trace d'accès sensible** (journal `conflit_interet`, tableau `GET /dossier/conflits-interet` pour la direction) ; toute écriture sur ce dossier (liens, payeurs, contacts, personnes, interdictions, « ne pas solliciter ») refusée : « doit être faite par un collègue » ; la recherche et la lecture directe d'une personne ferment l'adresse ; la fusion, la confirmation au guichet, les préférences d'envoi et l'ouverture d'une adresse protégée sont aussi refusées ; une personne liée à un **autre** élève avec lequel l'agent est en conflit (l'ex-conjoint, responsable d'un demi-frère) reste fermée partout. Sa casquette de parent (session famille) est inchangée | `conflit_d_interet_agent_parent_de_l_eleve`, `conflit_d_interet_personne_liee_a_un_autre_eleve_et_fusion`, `les_ecritures_renvoient_la_vue_minimisee`, `conflit_d_interet_declare_ex_conjoint`, `conflit_d_interet_contact_d_urgence`, `interdiction_sur_son_propre_dossier_par_un_collegue` |
| **Droit d'accès d'un parent sans les données de l'autre** | `vue_titulaire` : ses données en entier (hors F4), de l'autre responsable la désignation et la nature du lien ; contacts : ses propres déclarations + saisies de l'établissement (`ent_eleve_contact_urgence.declare_par_personne_id`, migration 272) ; jamais d'interdiction, jamais F4. Aucune donnée de l'autre dans la réponse de la fiche (test sur la représentation complète) | `droit_d_acces_d_un_parent_sans_les_donnees_de_l_autre` |
| **Interdiction de remise visible de l'accueil et de la vie scolaire** | Table `ent_eleve_interdiction` (sous-ensemble de §8.8) ; consigne et personne visée avec `remise.lire` (accueil, vie scolaire, infirmerie, secrétariat, direction ; prof : ses élèves) ; fondement avec F3 ; **motif scellé** (AES-256-GCM) lisible avec F4 seulement, consultation tracée ; invisible de la **personne visée** (même agent d'accueil) et de toute famille ; pose et levée par `remise.interdire` (Admin, Direction), levée motivée, jamais de suppression ; `demande_titulaire` enregistrée **non bloquante** ; période (future ou échue = inactive) ; motif jamais en clair en base ni au journal ni dans un événement (aucun événement publié) | `interdiction_de_remise_visible_accueil_et_vie_scolaire`, `interdiction_validation_et_periode` |

Routes ajoutées (toutes dans la matrice d'isolation `separation.rs`, domaine « élève / dossier ») : `GET|POST /dossier/eleves/{id}/interdictions`, `POST /dossier/interdictions/{id}/lever`, `POST /dossier/eleves/{id}/conflit-interet`, `GET /dossier/conflits-interet`, `POST /dossier/conflits-interet/{id}/lever`.

### 22.4 Limites et points à décider

- **Non fait dans A1** : propagation d'une interdiction aux postes et caches mobiles en moins d'une minute (R-A10), rappel à 48 h et pièce justificative (R-A9), alerte de cohérence autorisé + interdit (§8.8), fiche de secours (R-A11) — lot A5, qui reprend la table `ent_eleve_interdiction` posée ici. Détection du conflit par « ancien membre de foyer » : par déclaration seulement (tables de foyers au lot A3). Rapprochement automatique personne ↔ agent (`06_PERSONNELS.md`) : ici, par le compte de la fiche personne (`ent_personne.user_id`). Lecture de F3 *sur la fiche personne* (`get_personne`) : toujours soumise à `personnes.gerer` seul. Extrait RGPD d'un parent (R-A31, ADM-69) : lot A14 — A1 fournit la règle de vue (`vue_titulaire`), pas l'export. Les lectures hors fiche qui exposent des coordonnées de responsables (destinataires, A11 ; documents, socle 7) ne passent pas encore par la vue.
- **Notifications à la direction** : l'alerte de conflit est un tableau (accès tracés sur 30 jours + déclarations) ; pas de notification poussée.
- **Questions pour Florian** : (a) valider les six écarts de §22.1 ; (b) l'infirmerie doit-elle vraiment lire F0/F1/R (conception : oui ; avant : non) ? (c) un agent en conflit d'intérêts qui est **direction** : les écritures sur son dossier passent par une autre personne de la direction — acceptable dans un établissement à direction unique (le chef d'établissement parent d'élève) ? ; (d) le bris de glace doit-il exister avant E7 ?

### 22.5 Revue de sécurité du lot (skill `security-review` + relecture indépendante `review-sonnet`)

**Corrigés** (tests ajoutés) : (H) l'ex-conjoint restait lisible via un **autre élève** dont il est responsable ; (H) la fusion et la défusion de personnes n'avaient pas de garde de conflit (écriture, puis évasion de la détection) ; (H) l'ouverture d'une adresse protégée n'avait pas de garde de conflit ; (M) confirmation au guichet et préférences d'envoi sans garde ; (M) les **réponses des écritures** (payeurs, modification, suspension, rétablissement) renvoyaient les responsables non minimisés (motif de suspension à la comptabilité ou au secrétariat) : elles passent maintenant par la même vue ; (M) la lecture d'un motif protégé (interdiction, motif de suspension) est tracée même par la fiche ; (M) le journal par élève ne montre plus les entrées de conflit d'intérêts ni d'interdiction hors `responsables.protege.lire` ; (M) l'interdiction visant une personne **fusionnée** restait lisible de son compte (la requête suit `fusionnee_dans`) ; (M) un titulaire ne reçoit plus le commentaire interne d'un contact qu'il n'a pas déclaré, ni les coordonnées d'un contact qui est l'autre responsable.

**Non corrigés, à décider** :
- (M) **Un agent qui est aussi parent a deux comptes** (clé `ent_membership` = un rôle par établissement, `rattacher_compte` refuse un compte non parent) : la détection automatique par lien ne se déclenche donc que si le compte professionnel est rattaché à la fiche personne ; dans le cas courant (deux comptes), **seule la déclaration** (agent ou direction) protège. Le test simule le compte unique. Piste : rapprochement à l'embauche (socle 6), déclaration obligatoire, ou casquettes multiples par compte (`06_PERSONNELS.md`). **Question pour Florian.**
- (M) un parent peut proposer un contact avec `peut_recuperer = true` sans `peut_autoriser_sorties` : le statut « proposé » doit empêcher toute remise (lot A5, qui reprend le contact vers `ent_eleve_remise`).
- (L) `get_personne` d'une fiche **déjà fusionnée** par un collègue n'ajoute pas l'adresse de la cible ; la lecture d'une liste de liens en attente n'est pas tracée ; la trace de conflit est « au mieux » (une erreur d'écriture n'empêche pas la lecture) et non dédupliquée ; tri des responsables par autorité (laisse deviner l'autorité de l'autre) et champs `a_compte`/`actif` visibles de l'autre responsable ; clés étrangères non composées sur les trois nouvelles tables (lot A14, RLS) ; tout agent peut déclarer un conflit sur n'importe quel élève de l'établissement (oracle d'existence, bruit dans le tableau).
