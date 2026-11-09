# Socle 2 — L'élève

**Date :** 2026-11-06 — **Statut :** conception, **non décidée** (à valider par Florian). Aucun code écrit, rien compilé ni testé pour ce document. Aucun accès à la base ni aux secrets de production ; l'application Next.js a été lue en lecture seule ciblée, sans copie de code.
**Branche :** `ent-socle-eleve-conception`.
**À lire avec :** `CLAUDE.md`, `docs/VISION.md`, `docs/PRODUCT_SPEC.md`, `docs/DECISIONS.md` (D-013, D-014, D-017, D-019, D-020), `docs/ROLES_ET_DROITS.md`, `docs/SOCLE2_DOMAINES.md` (§2.1, §3 D2/D3, §6.2, §6.3), `docs/INTEROPERABILITE_NATIONALE.md`, `docs/IDENTITE_ET_PROFILS_FAMILLES.md`, `docs/RLS_ISOLATION.md`.

**Conventions.** **[F]** fait vérifié dans le dépôt (chemin cité) ; **[S]** fait sourcé repris de `INTEROPERABILITE_NATIONALE.md` ; **[H]** hypothèse (mémoire ou déduction, à vérifier) ; **[NV]** non vérifié, à vérifier en priorité sur un texte officiel avant de coder ; **[R]** recommandation (non décidée). Les identifiants d'actions métier `ELV-xx` sont stables.

**[personas] Passe « personas » du 2026-11-06.** Ajouts marqués **[personas]** : actions ELV-29 à ELV-48 (§11), règles R-E1 à R-E24 et cas limites (§16 bis), questions Q-E18 à Q-E23 (§17). Origine de chaque ajout (persona, manque, gravité) : `docs/socles/PERSONAS_02-04.md`. **Trois ajouts changent le lot E2 en cours** : date de naissance incomplète ou présumée (R-E1), motif codé (R-E4), sensibilité forcée (R-E3) ; un quatrième est recommandé (écriture d'origine, R-E2). Voir `PERSONAS_02-04.md` §0.

**Numérotation des socles — contradiction à lever.** `docs/ROADMAP.md` dit encore « socle 2 = domaines et actions métier, socle 3 = fonctions différenciantes », et `docs/SOCLE2_DOMAINES.md` nomme ses lots `S2-0` à `S2-15`. La demande de ce document fixe une autre numérotation : **socle 2 = l'élève, socle 3 = inscriptions et réinscriptions, socle 4 = dossier administratif** (foyers, responsables, coordonnées). Ce document suit la nouvelle numérotation ; `ROADMAP.md` est à mettre à jour après validation (question Q-E1). Les lots S2-1 (dossier, personnes) et S2-2 (référentiel) déjà réalisés restent la base de départ.

Sommaire : §1 Thèse · §2 Évaluation de l'existant · §3 Acteurs et besoins · §4 Frontières · §5 Concepts · §6 Historique · §7 Modèle de données cible · §8 Données venues de l'extérieur · §9 Logiciel ou paramétrage · §10 Cloisonnement, sécurité, données sensibles · §11 Actions métier · §12 Droits · §13 Événements · §14 Écrans · §15 Plan de migration · §16 Lots de code · §17 Questions à trancher · §18 Récapitulatif faits / hypothèses.

---

## 1. Thèse : ce que veut dire « l'élève est le centre »

Un ENT met l'élève au centre s'il peut répondre en une seconde, pour n'importe quel élève et selon les droits de celui qui regarde : *qui est-il, où en est-il, que s'est-il passé, que puis-je faire pour lui*. Cela ne veut **pas** dire qu'une table « élève » possède tout. Six principes [R] :

1. **Une clé, partout.** L'élève est une ligne stable (`ent_eleve.id`) que **toutes** les données qui le concernent référencent, avec l'établissement (`etablissement_id`), par une clé étrangère composée qui empêche en base de rattacher une donnée à l'élève d'un autre établissement. Aujourd'hui 12 tables portent `eleve_id` sans aucune clé étrangère [F, §2].
2. **Une identité vivante et prouvée.** Tout se modifie (nom, prénoms, date de naissance, sexe, lieu de naissance…), rien ne s'efface : chaque valeur garde sa date d'effet, sa nature (correction d'erreur ou changement réel), sa source, son auteur et sa pièce justificative. On sait toujours *qui a dit quoi, quand, sur quelle preuve*.
3. **Une scolarité racontée dans le temps.** L'élève entre, change de classe, de régime, d'options, monte de niveau, part, revient parfois. Ces faits sont des périodes datées, jamais des colonnes écrasées. Le statut (« à venir », « scolarisé », « sorti ») se **déduit** de ces périodes : il ne se saisit pas.
4. **Les données venues d'ailleurs n'entrent jamais en silence.** INE, état civil SIECLE, demandes des familles, imports : tout passe par une **proposition** (valeur actuelle → valeur proposée, source, preuve) acceptée par une règle ou par une personne. L'établissement décide, champ par champ, qui fait foi.
5. **Le centre de la navigation, pas le propriétaire des données.** La fiche de l'élève agrège ce que chaque domaine expose (vie scolaire, notes, santé, finances…) sous forme de tuiles et de pastilles, filtrées par les droits du lecteur ; chaque action reste exécutée par le domaine qui la possède. C'est le principe déjà posé dans `SOCLE2_DOMAINES.md` §2.1 [F], ici rendu concret.
6. **Le moins de données possible, montrées au moins de gens possible.** Les champs sont rangés en groupes (désignation, scolarité, état civil, identifiants nationaux…), chaque groupe s'ouvre par une action du catalogue `ent_auth::Action`, avec une portée. La minimisation est une règle du serveur, en un seul point, pas un choix d'écran.

Ce qui fera la différence au quotidien, par rapport aux logiciels du marché [H — comparaison de mémoire, non vérifiée] : une identité **bitemporelle** (date d'effet et date d'enregistrement) au lieu d'une fiche écrasée ; une **file de propositions** au lieu d'imports qui écrasent ; la **qualité des données** comme écran de travail (élèves sans INE, conflits, doublons, champs manquants avant un export) ; une **recherche instantanée** tolérante aux accents et aux fautes ; une **fusion de doublons annulable** ; une **confidentialité renforcée** pour les élèves protégés ; et une fiche qui **explique** pourquoi un champ est masqué ou verrouillé (« géré par SIECLE », « réservé à la vie scolaire »).

---

## 2. Évaluation franche de l'existant

Sources : migrations `ent-rust/migrations/` (001, 006, 014–018, 039, 068, 072, 091, 092, 100, 110–112, 120–123), `ent-db/src/{eleve_repo,eleve_compte_repo,dossier_personnes,perimetre,droits_ctx,referentiel_*}.rs`, `domain-eleve/src/{lib,dossier,ine,homonyme}.rs`, `ent-api/src/{routes,dossier,dto}.rs`, `ent-types/src/dossier.rs`, `web/crates/ent-web-app/src/modules/dossier/`, `ent-api/src/ui.rs` (recherche ciblée), et l'application Next.js (`db/schema.ts`, `app/lib/`, `app/(admin)/eleves/`), lecture seule.

### 2.1 Ce qui tient (à garder)

| Élément | Pourquoi c'est bon | Preuve [F] |
|---|---|---|
| `ent_eleve.id` comme pivot, `etablissement_id` sur chaque ligne | Clé déjà utilisée par plus de 100 tables et par l'enveloppe des événements | `001_ent_socle.sql:4-15` ; `ent_metier_event.eleve_id` |
| Personne distincte du compte (`ent_personne`), liens d'autorité datés et suspendables, vue `ent_eleve_lien_actif` lue en un point unique | Socle solide pour le socle 4 ; règle d'accès famille centralisée | `120`, `121`, `PgFoyerRepo::lien_parent_eleve` |
| Historique daté de classe, régime, MEF, site (`ent_eleve_periode`, une seule période ouverte par type) | Bonne forme d'historique, à généraliser | `006_ent_eleve_periodes.sql:13-37` |
| Scolarité par année (`ent_eleve_scolarite_annee`, unique par élève et année, source tracée) | Bonne intention : un enregistrement par année | `018_eleve_continuum.sql:18-47` |
| Affectations datées au référentiel (`ent_affectation`, classe XOR groupe, une classe ouverte par année) | Le référentiel S2-2 est la bonne source des classes et groupes | `092`, `110`, `referentiel_annee.rs` |
| Fusion de personnes réversible 30 jours avec instantané | Le patron de fusion réversible est le bon ; à étendre aux élèves | `123_personne_fusion.sql`, `dossier_personnes.rs:1943` |
| Minimisation en un point (`Eleve::minimise`, trois niveaux) | Principe juste (un seul point de retrait des champs) | `domain-eleve/src/lib.rs:374-411`, `service.rs:4060-4068` |
| Catalogue d'actions D-017 avec portées, anti-escalade, modèles par défaut testés contre l'ancien comportement | Mécanisme prêt à recevoir des actions « élève » fines | `ent-auth/src/actions.rs`, 47 actions |
| Fiche 360° Leptos avec actions envoyées par le serveur, alertes et chronologie filtrée par droits | Bon point de départ du « hub » | `web/.../modules/dossier/`, `GET /dossier/eleves/{id}/fiche` |
| Score d'homonymes pur, normalisation des accents | Réutilisable pour la détection de doublons | `domain-eleve/src/homonyme.rs:28-85` |
| Journal en ajout seul (trigger qui refuse UPDATE/DELETE) | Bonne garantie d'audit | `091` |
| Informations complémentaires paramétrables (`ent_info_comp_champ`/`_valeur`, typées) | Réponse existante au besoin « champs propres à l'établissement » | `072` |
| Outbox transactionnelle (S2-0) | Permet « fait + journal + événement » dans une transaction | `102_outbox.sql`, `tx.publier` |

### 2.2 Ce qui est à revoir

| Problème | Conséquence | Preuve [F] | Correction [R] |
|---|---|---|---|
| **Aucun historique d'identité.** `PUT /eleves/{id}/identite` écrase nom, prénom, sexe, date de naissance, INE ; le journal ne garde que les **nouveaux** nom et prénom, sans valeur précédente, et l'INE n'y figure pas | Impossible de savoir ce qui a changé, quand, pourquoi ; impossible de distinguer une erreur corrigée d'un changement réel | `service.rs:3972-4016`, `068` | Identité versionnée (§6, §7.2) |
| **Écritures hors transaction, erreurs de journal ignorées.** `create_eleve` et `update_identite` enchaînent des écritures séparées sur le pool ; `let _ =` sur le journal | Un élève peut exister sans événement ni journal ; une erreur d'audit passe inaperçue. Contredit le principe écrit dans `dossier_personnes.rs:16-17` | `service.rs:3896-3908`, `:4003`, `:4464` | Une transaction `TenantTx` par commande : fait + version + journal + événement (outbox) |
| **Données personnelles dans les événements.** `eleve.identity_changed` porte nom, prénom, sexe, date de naissance, badge **en clair** dans `ent_metier_event.payload` | Copie non chiffrée et non minimisée de l'état civil dans un journal durable | `service.rs:3984-4002` | Charge utile = identifiant de version + noms des champs changés, jamais les valeurs (comme `personne.coordonnees_modifiees` [F]) ; purge des anciens payloads après validation |
| **Fuite de minimisation dans la recherche d'homonymes.** `POST /eleves/homonymes` renvoie la date de naissance à tout profil qui lit les élèves (CPE, comptabilité, professeurs, psy, infirmerie), sans `niveau_identite` | Contourne la règle « Réduit » | `service.rs:13288-13300`, `dto.rs:2817-2835` (vérifié) | Correctif immédiat (lot E0) |
| **Annuaire ouvert à tout le personnel.** `GET /eleves/search` (`require_staff`) et `GET /eleves/{id}` (`can_read_all_eleves`, qui inclut le profil Prof) donnent à tout personnel la désignation de **tous** les élèves, alors que `perimetre.rs` pose « un professeur ne voit que ses élèves » | Écart entre la règle affichée et le comportement ; acceptable pour un annuaire réduit, mais à décider explicitement | `routes.rs:968-983`, `roles.rs:107-118` (vérifié) | Action `eleves.annuaire` explicite (§12) ; question Q-E3 |
| **Statut et régime sans contrainte.** `status`, `regime`, `sexe` en texte libre, aucun CHECK | Valeurs incohérentes possibles, exports nationaux fragiles | `001:10-12`, `015:4` | Codes contrôlés, statut dérivé (§7.4) |
| **INE sans règle.** Format toléré 9–16 caractères alphanumériques, aucune unicité, aucune provenance, aucun statut | Doublons d'INE possibles ; impossible de savoir si un INE est confirmé par l'administration ou recopié d'un bulletin | `ine.rs:7-20`, `001:9` | Identifiants externes avec statut et provenance (§7.3, §8) |
| **12 tables sans clé étrangère vers l'élève** (`ent_eleve_periode`, `_scolarite_annee`, `_option`, `_option_annee`, `_compte`, `ent_eleve_lien`, `_document`, `_dispositif`, `_grille_repas`, `_bourse`, `_dispense_repas`, `ent_metier_event`) | Lignes orphelines possibles ; aucune garantie que l'élève est du même établissement | rapport migrations | Clés étrangères composées `(etablissement_id, eleve_id)`, ajoutées `NOT VALID` puis validées (§15) |
| **Projections en double.** `classe` texte et `classe_id` ; `mef`, `site`, `regime` à la fois sur `ent_eleve`, dans `ent_eleve_periode` et dans `ent_eleve_scolarite_annee` ; `date_entree`/`date_sortie` sur l'élève et nulle part ailleurs | Plusieurs sources de vérité ; risque de divergence | `015:2` (« projections »), `111` | Une source par fait ; colonnes d'`ent_eleve` maintenues par la base, retirées ensuite |
| **Année en texte** dans `ent_eleve_scolarite_annee.annee_scolaire`, alors que `ent_affectation` utilise `annee_id` | Deux façons de désigner l'année | `018`, `092` | `annee_id` obligatoire |
| **Options en double** : `ent_eleve_option` (015) et `ent_eleve_option_annee` (018) | Ambiguïté | rapport migrations | Une seule notion, rattachée à l'inscription annuelle et aux groupes du référentiel |
| **Lien élève ↔ compte à trois endroits** : `ent_eleve_compte`, `ent_eleve_lien.user_id` (élève majeur lié à lui-même ?), `ent_personne.eleve_id` | Aucune contrainte de cohérence | `014`, `120` | Une seule règle (§7.9) |
| **Pas de RLS sur les tables élève** alors que la santé, la psy et la facturation sont pilotes | L'isolation des données élève repose sur les seuls filtres applicatifs | `100_rls_pilote.sql:28-36` | Étendre la RLS (§10.3) |
| **`updated_at` jamais mis à jour** sur `ent_eleve` | Date trompeuse | `001:14` | Remplacée par la date de la version courante |
| **Écran : ni photo, ni INE, ni date de naissance, pas de liste d'élèves, pas de recherche dédiée, pas de trombinoscope** dans le web Leptos ; `/web/eleves` n'existe pas | La fiche n'est atteignable que depuis la liste d'une classe | `ent-web-app/src/lib.rs:59,78` | Écrans §14 |
| **Écran : droit « famille » calculé côté client** pour proposer un contact | Contredit « le serveur décide de tout » (le POST reste contrôlé côté serveur [H]) | `modules/dossier/mod.rs:91,120,168` | Action proposée par le serveur |
| **Écran : ajout d'un responsable en deux appels non atomiques** (créer la personne, puis le lien) | Personne orpheline possible | `modules/dossier/actions.rs:71-74` | Commande serveur unique (relève du socle 4) |

### 2.3 Ce qui manque

- État civil complet : nom de naissance et nom d'usage, prénoms multiples et prénom usuel, prénom d'usage, lieu et pays de naissance, nationalité (si l'établissement doit la collecter) [F : absents des migrations ; Next.js n'a que `lieu_naissance` en texte libre].
- Identifiants externes autres que l'INE : identifiant élève SIECLE, ONDE, clé de jointure AAF, identifiant de l'ancien logiciel (reprise). Next.js range l'identifiant SIECLE dans un JSON hors schéma relationnel [F Next : `export-siecle.ts:141-145`] : à ne pas reproduire.
- Numéro d'élève interne (matricule) paramétrable.
- Scolarisation (séjour dans l'établissement : entrée, origine, sortie, destination, motif), réintégration d'un ancien élève, capacité juridique (majorité, émancipation, mesure de protection).
- File de propositions et règles de « source qui fait foi » par champ.
- Paramétrage des champs de l'élève par établissement (visible, obligatoire, saisissable par la famille, géré par une source externe).
- Confidentialité renforcée (élève protégé), photos versionnées, coordonnées propres de l'élève (lycéen, étudiant de BTS).
- Fusion de deux dossiers élèves (DOS-15 n'est livré que pour les personnes [F, `SOCLE2_DOMAINES.md` §6.2]).
- Indicateurs (pastilles) de la fiche alimentés par les domaines ; écran de qualité des données ; imports d'élèves (aucun dans le Rust [F] ; Next.js a des imports SIECLE XML et Excel, et un export SIECLE en ZIP [F Next : `siecle-eleves-parse.ts`, `eleves-import.ts`, `export-siecle.ts`]).

### 2.4 Ce que Next.js fait et qu'il ne faut pas perdre (lecture seule)

| Fonction Next.js [F Next] | Où | Reprise dans ce socle |
|---|---|---|
| Import SIECLE XML : `NOM_DE_FAMILLE`, `PRENOM`, `ID_NATIONAL` (INE), `CODE_SEXE`, `DATE_NAISS`, `CODE_REGIME`, `CODE_MEF`, `CODE_STRUCTURE`, `CODE_DIVISION`, `DATE_SORTIE`, `ELEVE_ID`, `MEL` | `app/lib/siecle-eleves-parse.ts:48-92` | Liste de champs utile pour le modèle (§7) ; import par propositions (lot E5, en lien avec IOP-L1/L3). **Aucune copie de code.** |
| Import Excel venant d'autres logiciels (Pronote, École Directe) | `app/lib/eleves-import.ts:681` | Reprise d'un ancien logiciel = source « reprise » des propositions |
| Export SIECLE (ZIP) élèves, responsables, MEF, division, régime | `export-siecle.ts:311-450` | Remontée BEE du privé (IOP-L4) ; à noter : `codeStatut` y est toujours `ST` quel que soit le statut (`:447`), défaut à ne pas reproduire |
| Clé de rapprochement `source_key` (`ine:…` ou `person:nom§prénom`), « une valeur vide n'écrase pas » | `db/schema.ts:349`, `eleves-import.ts:433-514` | Principe repris et durci : rien n'écrase sans proposition |
| Unicité partielle de l'INE par établissement | `db/schema.ts:388-390` | Reprise (§7.3) |
| Photos en masse avec rapprochement par nom de fichier | `ElevePhotosBulkPanel.tsx`, `eleve-photos-match.ts` | Écran photos en lot (§14) |
| Scolarité par année, régimes datés (base de facturation), établissement précédent | `db/schema.ts:398-468` | Inscription annuelle et scolarisation (§7.4) |
| Audit des accès élève et des exports sensibles | `db/schema.ts:624-666` | Journal de consultation (§10.5) |
| Statuts `preinscrit`, `inscrit`, `ancien`, `archive` | `db/schema.ts:367` | Statut dérivé (§7.4) ; la préinscription relève du socle 3 |
| Suppression définitive d'un élève (documents, notes, absences compris) | `eleve-delete-permanent.ts` | **Non reprise** : remplacée par l'annulation d'une création erronée, la fusion ou l'archivage (Q-E8) |
| Doublons de coordonnées parents sur l'élève (`parent1_email`…) | `db/schema.ts:356-361` | **Non repris** : les coordonnées des responsables relèvent du socle 4 |

---

## 3. Acteurs et besoins

| Acteur | Besoin principal vis-à-vis de l'élève | Fréquence | Ce qu'il ne doit pas voir |
|---|---|---|---|
| Secrétariat de scolarité (administratif) | Créer, corriger, compléter, rapprocher des imports, préparer les exports, gérer entrées et sorties, traiter les demandes des familles | quotidienne, par lots à la rentrée | Santé, psy, notes de suivi confidentielles |
| Direction | Vue d'ensemble, décisions sensibles (radiation, fusion, confidentialité renforcée), qualité des données avant échéances nationales | hebdomadaire | Santé détaillée sauf droit donné |
| CPE, surveillants, accueil | Trouver un élève en deux secondes (y compris par badge ou photo), voir classe, régime, contacts d'urgence, pastilles utiles (PAI, interne, sortie autorisée) | plusieurs fois par heure | État civil complet, INE, finances |
| Professeur, professeur principal | Ses élèves : désignation, photo, groupes, pastilles pédagogiques (PAP, aménagements) ; le PP voit un peu plus | quotidienne | État civil, coordonnées, santé, autres classes (selon Q-E3) |
| Comptabilité | Identité de facturation, régime et ses dates, payeurs (socle 4) | quotidienne | Santé, vie scolaire détaillée |
| Infirmerie, psychologue | Identité suffisante pour un soin ou un entretien (nom, date de naissance, classe), pastilles santé | quotidienne | Finances, discipline au-delà du nécessaire |
| Responsable légal | Voir l'identité de son enfant telle que l'école la connaît, signaler une erreur avec une pièce, mettre à jour ce que l'établissement lui permet | ponctuelle | Autres élèves, données internes, historique interne |
| Élève (mineur) | Son profil, ses coordonnées propres (lycée), une demande de prénom d'usage si l'établissement l'ouvre | ponctuelle | Données internes ; selon l'âge, rien de plus que la famille |
| Élève majeur ou émancipé | Mêmes droits qu'un responsable sur ses propres données, choix sur l'accès de ses parents (Q10 de `SOCLE2_DOMAINES.md`, non tranchée) | ponctuelle | — |
| Administrateur de l'établissement | Paramétrer les champs, la numérotation, les régimes, les motifs, les sources qui font foi | rentrée | Contenu des dossiers sans droit |
| Systèmes externes (SIECLE/AAF, BEE, ONDE, ÉduConnect, GAR) | Échanger des données d'identité et de structure selon des conventions | quotidienne à annuelle | Tout ce qui n'est pas dans la convention (minimisation) |
| Domaines internes (vie scolaire, notes, facturation, restauration…) | Une clé d'élève stable, un statut fiable à une date, des événements quand l'identité ou la scolarité change | permanente | — |

---

## 4. Frontières

| Sujet | Socle 1 (sécurité, fait) | **Socle 2 (l'élève, ce document)** | Socle 3 (inscriptions, réinscriptions) | Socle 4 (dossier administratif) | Autres domaines |
|---|---|---|---|---|---|
| Comptes, sessions, passkeys, ÉduConnect côté connexion | **possède** | lie un compte à l'élève (accès de l'élève), règles d'âge | — | lie un compte à un responsable | — |
| Identité de l'élève (état civil, versions, pièces) | — | **possède** | fournit les valeurs de départ via une commande du socle 2 | — | lisent |
| Identifiants externes (INE, SIECLE, ONDE, jointure AAF) | — | **possède** le modèle et les règles | peut déclarer un INE fourni par la famille | — | IOP les alimente et les exporte |
| Candidature, préinscription, dossier d'admission, pièces, décision, campagnes | — | — | **possède** ; crée l'élève ou réintègre un ancien élève **en appelant** le socle 2 | — | — |
| Scolarisation (entrée, sortie, origine, destination) et inscription annuelle (année, niveau, MEF, régime, statut de l'apprenant) | — | **possède les données et les commandes** | **possède les processus** qui les déclenchent (validation d'inscription, réinscription, départ annoncé, exeat) | — | référentiel (affectations), facturation, restauration lisent |
| Classes, groupes, affectations | — | lit ; l'inscription annuelle s'y relie | déclenche l'affectation initiale | — | **référentiel** (S2-2) possède |
| Responsables, foyers, autorité parentale, garde, parents séparés, payeurs, contacts d'urgence, adresses, téléphones, e-mails des responsables | — | affiche ; dépend de la capacité juridique (majorité) | invite le second parent (D-014) | **possède** (S2-1 a déjà posé `ent_personne`, liens, contacts) | facturation lit les payeurs |
| Coordonnées propres de l'élève (e-mail, mobile) | — | **possède** | — | adresse de résidence : **socle 4** (l'élève réside dans un ou des foyers) | messagerie lit |
| Capacité juridique (mineur, majeur, émancipé, mesure de protection) | — | **possède** | — | désigne le tuteur ou curateur comme responsable | droits famille la lisent |
| Confidentialité renforcée (élève protégé) | moteur d'autorisation | **possède** le marquage et ses effets sur l'élève | — | possède les interdictions visant une personne (« ne pas remettre à… ») | tous la respectent |
| Santé, PAI, PAP, PPS, dispositifs | — | affiche des **pastilles** fournies par les domaines | — | — | santé, accompagnement possèdent |
| Documents de l'élève | stockage, URL pré-signées | pièces justificatives d'identité (référence) | pièces d'inscription | pièces de responsables | documents (D9) possède |
| Chronologie, notifications | outbox | fournit ses événements | idem | idem | D0 possède |

Règle de frontière [R] : **le socle 3 ne fait jamais d'`INSERT` dans les tables du socle 2** ; il appelle des commandes (`creer_eleve`, `ouvrir_scolarisation`, `enregistrer_inscription_annuelle`, `cloturer_scolarisation`, `reintegrer`). Idem pour les imports (IOP) : ils déposent des propositions ou appellent ces mêmes commandes. Le socle 4 ne modifie pas l'identité de l'élève.

---

## 5. Concepts

| Concept | Définition | Table [R] |
|---|---|---|
| **Élève** | Un enfant ou jeune **dans un établissement** (tenant). Même personne dans deux établissements ScolIA = deux élèves sans lien visible (D-013). Clé stable, jamais réutilisée. | `ent_eleve` |
| **Version d'identité** | État civil et noms d'usage valables à partir d'une date d'effet, avec nature, source, auteur, pièce | `ent_eleve_identite` |
| **Identifiant externe** | Valeur attribuée par un système extérieur (INE…), avec statut (déclaré, confirmé, contesté…) et provenance | `ent_eleve_identifiant` |
| **Numéro d'élève** | Numéro interne de l'établissement, format paramétrable, jamais réutilisé | colonne `ent_eleve.numero` |
| **Scolarisation** | Séjour continu dans l'établissement : entrée (date, motif, origine), sortie (date, motif, destination). Un élève qui part et revient a deux scolarisations | `ent_eleve_scolarisation` |
| **Inscription annuelle** | Pour une année scolaire : niveau, MEF, statut de l'apprenant, état (prévue, confirmée, en cours, terminée, annulée), décision de passage | `ent_eleve_inscription` (succède à `ent_eleve_scolarite_annee`) |
| **Période d'attribut** | Valeur datée d'un attribut qui change en cours d'année : régime, site | `ent_eleve_periode` (existante, kinds réduits) |
| **Statut** | Déduit, à une date : `a_venir`, `scolarise`, `sorti`, `archive` | vue `ent_eleve_statut` + colonne de cache |
| **Capacité juridique** | Mineur / majeur (calculé), émancipé, majeur sous mesure de protection (déclarés, avec pièce) | `ent_eleve_capacite` |
| **Confidentialité renforcée** | Marquage d'un élève protégé, qui réduit sa visibilité et trace chaque consultation | `ent_eleve_confidentialite` |
| **Proposition** | Changement demandé par une source (import, famille, élève, ancien établissement), en attente d'une décision | `ent_eleve_proposition` |
| **Règle de champ** | Paramétrage établissement d'un champ du catalogue : visible, obligatoire (et à quel moment), saisissable par la famille, source qui fait foi | `ent_param_eleve_champ` |
| **Pastille** | Indicateur affiché sur la fiche (PAI, interne, boursier, majeur, sans INE…), fourni par un domaine, protégé par une action | `ent_eleve_indicateur` (projection) |
| **Fusion** | Réunion de deux élèves en doublon, avec registre des lignes déplacées, annulable | `ent_eleve_fusion`, `ent_eleve_fusion_ligne` |

---

## 6. L'historique : comment « tout se modifie » sans rien perdre

### 6.1 Trois mécanismes, chacun à sa place

| Mécanisme | Sert à | Exemple | Existant |
|---|---|---|---|
| **Versions métier datées** | Répondre à « quelle était la valeur à telle date, et pourquoi a-t-elle changé ? » | nom de l'élève au 1ᵉʳ septembre 2025 ; régime en janvier | `ent_eleve_periode` [F] ; à créer pour l'identité |
| **Journal d'audit en ajout seul** | Répondre à « qui a fait quoi, quand » pour toute écriture, y compris celles qui ne sont pas datées métier | qui a accepté telle proposition | `ent_journal_modif` [F], à compléter par l'avant/après (noms de champs + identifiants de versions) |
| **Événements (outbox)** | Prévenir les autres domaines | `eleve.identity_changed` → recalcul de la recherche, notification | outbox S2-0 [F] |

Le journal **ne remplace pas** les versions : il est fait pour l'audit, pas pour reconstruire un état à une date. Les versions **ne remplacent pas** le journal : elles ne disent pas qui a consulté.

### 6.2 Correction ou changement : la distinction qui manque partout

Chaque modification d'identité déclare sa **nature** :

| Nature | Sens | Date d'effet | Effet sur les documents déjà émis | Exemple |
|---|---|---|---|---|
| `correction` | La valeur précédente était **fausse depuis le début** | celle de la version corrigée | Une réédition utilise la valeur corrigée ; les documents figés déjà émis ne changent pas (ils sont figés par leur domaine) | « Dupond » saisi au lieu de « Dupont » |
| `changement_etat_civil` | La personne a **changé** à une date (décision d'état civil, adoption, reconnaissance, changement de nom ou de prénom, mention de sexe) | date de la décision | Une réédition d'un document d'une période antérieure garde l'ancienne valeur, sauf décision contraire (ex. droit de faire rééditer [H]) | changement de nom par jugement |
| `usage` | Ajout ou retrait d'un **nom ou prénom d'usage**, sans changer l'état civil | date de la demande acceptée | Affichage quotidien ; les documents officiels gardent l'état civil, sauf règle contraire | prénom d'usage (§7.2) |
| `creation`, `reprise`, `import`, `fusion` | Origine de la valeur | — | — | — |

Modèle **bitemporel** [R] : chaque version a une **date d'effet** (`effet_du`) et une **date d'enregistrement** (`enregistre_le`). Une correction ne crée pas de nouvelle période : elle **remplace** une version (`remplace_version_id`), qui reste lisible dans l'historique (« corrigé le 12/10 par le secrétariat, pièce : acte de naissance »). On peut donc répondre à deux questions différentes : « que savait l'établissement le 1ᵉʳ octobre ? » et « quelle était la vraie valeur au 1ᵉʳ octobre ? ».

### 6.3 Une seule écriture possible

- La valeur **courante** reste copiée sur `ent_eleve` (nom, prénom affiché, sexe, date de naissance…), parce que des dizaines de requêtes la lisent [F]. Cette copie est **maintenue par la base** (trigger après insertion d'une version) et **protégée** : un trigger refuse toute modification directe des colonnes d'identité d'`ent_eleve` hors de la synchronisation [R]. Plus aucune divergence possible entre la fiche et l'historique.
- Toute commande d'écriture du socle 2 = **une transaction** (`TenantTx`) qui écrit le fait, la version, le journal (avant/après sous forme de noms de champs et d'identifiants de versions) et l'événement (outbox). Plus de `let _ =` sur le journal [R].
- Même mécanisme pour les autres faits datés : scolarisations, inscriptions annuelles, périodes de régime et de site, identifiants, photos, capacité, confidentialité. Chacun garde ses lignes ; rien ne s'efface hors des règles de conservation (§10.6).

---

## 7. Modèle de données cible

Notation : pseudo-DDL PostgreSQL, à préciser par le lot qui l'implémente. Toutes les tables portent `etablissement_id UUID NOT NULL`, une clé étrangère **composée** `(etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id)` (qui suppose `UNIQUE (etablissement_id, id)` sur `ent_eleve`, à ajouter [R]), la RLS (§10.3), `cree_le`/`cree_par`. Les codes en `CHECK` sont des **codes du logiciel** ; les libellés affichés peuvent être paramétrés. Numéros de migration proposés : bloc **140–149** (libre [F] : la séquence actuelle s'arrête à 134).

### 7.1 `ent_eleve` (pivot, conservé et resserré)

```
ent_eleve
  id                  UUID PK                         -- inchangé, jamais réutilisé
  etablissement_id    UUID NOT NULL                   -- + UNIQUE (etablissement_id, id)
  numero              TEXT NOT NULL                   -- numéro d'élève interne, UNIQUE (etablissement_id, numero), jamais réutilisé
  -- projection de la version d'identité courante (écrite par trigger seulement)
  nom                 TEXT NOT NULL                   -- nom affiché = nom d'usage sinon nom de naissance (compatibilité)
  prenom              TEXT NOT NULL                   -- prénom affiché = prénom d'usage (si autorisé) sinon prénom usuel
  nom_naissance       TEXT NOT NULL
  prenoms             TEXT[] NOT NULL
  sexe                TEXT NULL CHECK (sexe IN ('M','F'))   -- sexe à l'état civil [H : codes BEE 1/2 à mapper à l'export]
  date_naissance      DATE NULL
  cle_recherche       TEXT NOT NULL                   -- normalisée (minuscules, sans accents, noms + prénoms + usages), index trigramme
  identite_version_id UUID NOT NULL                   -- version courante
  -- projections de scolarité (écrites par la base)
  statut              TEXT NOT NULL CHECK (statut IN ('a_venir','scolarise','sorti','archive'))  -- cache de la vue ent_eleve_statut
  confidentialite     TEXT NOT NULL DEFAULT 'normale' CHECK (confidentialite IN ('normale','renforcee'))
  photo_id            UUID NULL                       -- photo courante validée
  fusionne_dans       UUID NULL REFERENCES ent_eleve  -- doublon absorbé : lecture seule, redirige
  creation_source     TEXT NOT NULL CHECK (creation_source IN ('saisie','admission','import','reprise','labo'))
  creation_ref        UUID NULL                       -- dossier d'admission, lot d'import…
  cree_le, cree_par
  -- colonnes héritées, en sursis (§15) : classe, classe_id, regime, mef, site, date_entree, date_sortie,
  --   motif_sortie, redoublant, ine, numero_badge, photo_key, status, updated_at
```

Remarques [R] :
- `nom`/`prenom` gardent leur sens actuel (« ce qu'on affiche ») pour ne casser aucune lecture ; le détail vit dans la version.
- Le badge de cantine devient un identifiant (§7.3, système `BADGE`) ; `numero_badge` reste en projection le temps de migrer `get_by_badge` [F `eleve_repo.rs:38`].
- Pas de colonne « classe » dans la cible : la classe à une date se lit dans `ent_affectation` (référentiel, Q14 de `SOCLE2_DOMAINES.md`).

### 7.2 `ent_eleve_identite` (versions d'identité)

```
ent_eleve_identite
  id                    UUID PK
  etablissement_id, eleve_id
  nom_naissance         TEXT NOT NULL CHECK (length BETWEEN 1 AND 120)   -- « nom de famille » de l'état civil
  nom_usage             TEXT NULL                                         -- ex. double nom d'usage [H : règles d'usage à vérifier]
  prenoms               TEXT[] NOT NULL CHECK (cardinality BETWEEN 1 AND 10)  -- dans l'ordre de l'état civil
  rang_prenom_usuel     SMALLINT NOT NULL DEFAULT 1                       -- prénom usuel parmi les prénoms
  prenom_usage          TEXT NULL                                          -- prénom d'usage (voir ci-dessous)
  sexe                  TEXT NULL CHECK (sexe IN ('M','F'))
  date_naissance        DATE NULL
  naissance_commune_cog TEXT NULL                 -- code commune INSEE (COG) en vigueur à la date de naissance
  naissance_commune     TEXT NULL                 -- libellé saisi ou importé
  naissance_pays_cog    TEXT NULL                 -- code pays INSEE (COG) [NV : nomenclature exacte attendue par BEE]
  naissance_lieu_etranger TEXT NULL               -- ville à l'étranger, texte libre
  nationalite_cog       TEXT NULL                 -- seulement si le champ est activé (§9, Q-E5)
  nature                TEXT NOT NULL CHECK (nature IN ('creation','reprise','import','correction','changement_etat_civil','usage','fusion'))
  effet_du              DATE NOT NULL
  remplace_version_id   UUID NULL REFERENCES ent_eleve_identite   -- pour une correction
  remplacee_par         UUID NULL                                   -- posée quand une correction la remplace
  motif                 TEXT NULL CHECK (length <= 500)            -- obligatoire pour correction et changement
  piece_document_id     UUID NULL                                   -- acte, jugement, livret de famille… (D9)
  source                TEXT NOT NULL CHECK (source IN ('saisie','admission','import','famille','eleve','ancien_etablissement','reprise','fusion'))
  source_ref            UUID NULL                                   -- proposition, lot d'import, dossier d'admission
  sensible              BOOLEAN NOT NULL DEFAULT false              -- version dont l'existence est sensible (ex. changement de prénom ou de sexe)
  valeurs_scellees      TEXT NULL                                   -- si sensible et non courante : valeurs chiffrées (champs_chiffres), colonnes en clair vidées
  enregistre_le         TIMESTAMPTZ NOT NULL DEFAULT now()
  enregistre_par        UUID NOT NULL
  -- contraintes
  UNIQUE partiel : une seule version « en vigueur » par (eleve_id, effet_du) parmi les non remplacées
  CHECK : nature IN ('correction','changement_etat_civil') => motif IS NOT NULL
  CHECK : nature = 'correction' => remplace_version_id IS NOT NULL
```

Règles [R] :
- **Version courante** = la version non remplacée dont la date d'effet est la plus récente et ≤ aujourd'hui. Une version datée dans le futur (changement annoncé) devient courante à sa date (tâche quotidienne qui recalcule les projections, comme pour le statut).
- **Prénom d'usage** : utilisé à l'affichage quotidien (listes, appel, trombinoscope, messagerie) quand l'établissement l'a activé ; les documents officiels et les exports nationaux utilisent l'état civil. Cadre **[V]** (circulaire du 29-9-2021, BO n° 36 du 30-9-2021, lue dans une copie du BO ; VERIF R17) : pour un mineur, l'aménagement exige **l'accord des deux représentants légaux** ; le prénom d'usage remplace l'état civil « dans tous les documents qui relèvent de l'organisation interne (listes d'appel, carte de cantine …) ainsi que dans les espaces numériques (ENT, etc.) » ; l'état civil reste dans les systèmes de notation qui servent aux diplômes. Le logiciel fournit le mécanisme et la traçabilité ; la procédure (accord des parents, entretien) est un **paramétrage** et une décision humaine. Ces versions sont marquées `sensible` : l'historique n'est visible qu'avec une action dédiée (§12) et les anciennes valeurs sont scellées.
- **Lieu de naissance** : le code commune s'entend à la date de naissance (une commune fusionnée depuis garde son ancien code) [H : le COG INSEE publie l'historique des communes ; licence ouverte à vérifier, NV]. Saisie assistée par recherche dans le COG, avec libellé libre de secours.
- **Normalisation** : valeurs stockées telles que saisies (Unicode NFC, accents conservés) ; la transformation vers un format externe (majuscules sans accents, longueur maximale…) est faite **à l'export**, par le connecteur concerné [R]. Contrôles de saisie : pas de chiffres dans un nom, espaces multiples réduits, apostrophes et traits d'union normalisés (paramètre du logiciel, pas de l'établissement).

### 7.3 `ent_eleve_identifiant` (INE et autres identifiants externes)

```
ent_systeme_identifiant                         -- référentiel du logiciel (pas de l'établissement)
  code TEXT PK            -- INE, SIECLE_ELEVE, ONDE, AAF_JOINTURE, BADGE, ANCIEN_LOGICIEL…
  libelle, format_regle, unique_par_etab BOOLEAN, un_seul_actif_par_eleve BOOLEAN,
  lisible_par_famille BOOLEAN, exportable_vers TEXT[]

ent_eleve_identifiant
  id UUID PK
  etablissement_id, eleve_id
  systeme          TEXT NOT NULL REFERENCES ent_systeme_identifiant
  valeur           TEXT NOT NULL                  -- normalisée selon le système
  statut           TEXT NOT NULL CHECK (statut IN ('declare','a_verifier','confirme','conteste','revoque'))
  provenance       TEXT NOT NULL CHECK (provenance IN ('saisie','document_famille','ancien_etablissement','import_aaf','import_bee','retour_bee','import_onde','educonnect','reprise_logiciel','badge'))
  preuve_document_id UUID NULL
  valide_du        DATE NOT NULL
  valide_au        DATE NULL
  confirme_le, confirme_par_source TEXT NULL     -- ex. lot d'import officiel
  motif            TEXT NULL                      -- obligatoire pour conteste / revoque
  enregistre_le, enregistre_par
  -- contraintes
  UNIQUE partiel (etablissement_id, systeme, valeur) WHERE statut IN ('declare','a_verifier','confirme') AND valide_au IS NULL   -- si le système l'exige
  UNIQUE partiel (eleve_id, systeme)          WHERE statut IN ('declare','a_verifier','confirme') AND valide_au IS NULL   -- un INE actif par élève
```

Règles [R] :
- L'INE est un **attribut de l'élève dans l'établissement**, unique **dans l'établissement** seulement (D-013 : aucune vérification entre établissements, un établissement ne doit pas apprendre qu'un INE existe ailleurs).
- Un INE **déclaré** (recopié d'un bulletin, donné par la famille) n'a pas la même valeur qu'un INE **confirmé** par un flux officiel. Les exports nationaux (BEE, LSU, LSL) n'utilisent qu'un INE confirmé, ou signalent l'écart à l'écran de qualité (§14).
- Une collision (le même INE sur deux élèves) n'échoue pas en silence : la seconde valeur est enregistrée `conteste`, une alerte apparaît dans la qualité des données, et c'est souvent le signe d'un **doublon** (§11, fusion).
- Validation de format **par famille d'INE**, dans un module isolé et testé : le format exact (longueur, caractère de contrôle, familles selon l'origine : premier degré, second degré, apprentissage, agriculture…) est **[NV]**. Aujourd'hui : 9 à 16 caractères alphanumériques [F `ine.rs`], trop large. Ne pas coder de clé de contrôle de mémoire (D-019).
- Les identifiants servant à un service tiers (GAR, ÉduConnect) sont **par personne et par usage**, opaques, jamais l'INE ni l'identifiant interne (`INTEROPERABILITE_NATIONALE.md` §5.3 [R]). Ils relèvent des lots IOP ; le système `AAF_JOINTURE` sert au rattachement ÉduConnect (IOP-L5).

### 7.4 Scolarisation, inscription annuelle, statut

```
ent_eleve_scolarisation                    -- un séjour continu dans l'établissement
  id UUID PK
  etablissement_id, eleve_id
  date_entree        DATE NOT NULL
  motif_entree       TEXT NOT NULL CHECK (motif_entree IN ('premiere_inscription','changement_etablissement','retour','passage_interne','reprise'))
  origine_uai        TEXT NULL            -- établissement d'origine (UAI si connu)
  origine_libelle    TEXT NULL            -- établissement d'origine, ou « instruction en famille », « étranger »…
  origine_type       TEXT NULL CHECK (origine_type IN ('public','prive_sous_contrat','prive_hors_contrat','instruction_famille','etranger','inconnu'))
  date_sortie        DATE NULL
  motif_sortie       TEXT NULL            -- code du logiciel (fin_de_cycle, changement_etablissement, demenagement, exclusion_definitive, abandon, fin_obligation, instruction_famille, etranger, deces, autre) + motif local paramétrable  -- [personas] instruction_famille, etranger, deces ajoutés (R-E12)
  motif_sortie_local UUID NULL REFERENCES ent_param_motif_sortie
  destination_uai, destination_libelle, destination_type
  exeat_document_id  UUID NULL            -- certificat de radiation / exeat [H : terminologie et obligations à vérifier]
  source, source_ref, decide_par, cree_le
  CHECK date_sortie IS NULL OR date_sortie >= date_entree
  EXCLUDE USING gist (eleve_id WITH =, daterange(date_entree, date_sortie, '[]') WITH &&)   -- pas de chevauchement (btree_gist) [R]

ent_eleve_inscription                       -- succède à ent_eleve_scolarite_annee (§15)
  id UUID PK
  etablissement_id, eleve_id
  scolarisation_id   UUID NOT NULL REFERENCES ent_eleve_scolarisation
  annee_id           UUID NOT NULL REFERENCES ent_annee_scolaire
  niveau_id          UUID NULL REFERENCES ent_niveau
  mef_id             UUID NULL REFERENCES ent_mef
  statut_apprenant   TEXT NOT NULL DEFAULT 'scolaire' CHECK (statut_apprenant IN ('scolaire','apprenti','autre'))  -- [H] à préciser avec les nomenclatures
  etat               TEXT NOT NULL CHECK (etat IN ('prevue','confirmee','en_cours','terminee','annulee'))
  redoublement       BOOLEAN NULL         -- calculé (même niveau que l'année précédente), modifiable avec motif
  decision_passage   TEXT NULL            -- posée par le domaine évaluations (conseil de classe)
  date_debut, date_fin
  source TEXT NOT NULL CHECK (source IN ('saisie','admission','reinscription','montee','import','reprise'))
  source_ref UUID NULL
  UNIQUE partiel (eleve_id, annee_id) WHERE etat <> 'annulee'
```

- Les **classes et groupes** de l'année restent dans `ent_affectation` (référentiel) ; on y ajoute `inscription_id` facultatif pour relier une affectation à son inscription [R].
- Les **options et enseignements suivis** deviennent des affectations à des groupes du référentiel (option = groupe), ce qui supprime les deux tables d'options parallèles [R] ; le vœu d'option pour l'année suivante relève du socle 3 (réinscription).
- Le **régime** (externe, demi-pensionnaire, interne… [H : codes nationaux à mapper]) et le **site** restent des périodes datées dans `ent_eleve_periode` (kinds `regime`, `site`, avec codes contrôlés par `ent_param_regime` et `ent_site`), car ils changent en cours d'année et la facturation en dépend ; les kinds `classe` et `mef` y deviennent des projections du référentiel et de l'inscription, puis disparaissent (§15).
- **Statut dérivé** (vue `ent_eleve_statut(eleve_id, a_la_date)`) : `a_venir` si la prochaine scolarisation commence après la date ; `scolarise` si une scolarisation couvre la date ; `sorti` si la dernière scolarisation est close ; `archive` quand la durée de conservation active est dépassée (règle DPO, §10.6). La colonne `ent_eleve.statut` est un cache recalculé à chaque écriture et chaque nuit. Plus jamais de « statut saisi » incohérent avec les dates.
- **Réintégration** d'un ancien élève : nouvelle scolarisation sur **le même** `ent_eleve` (on retrouve son historique, son INE, ses documents, selon les règles de conservation). Proposée automatiquement par la détection de doublons à la création (§11, ELV-01).
- **Continuité entre degrés** : si un établissement (tenant) regroupe école, collège et lycée sur plusieurs sites, l'élève garde la même fiche ; le passage est une inscription annuelle d'un autre niveau, pas une sortie. L'**UAI est porté par le site** (`ent_site.uai` existe déjà [F `110:14-15`]), car un tenant peut couvrir plusieurs UAI (question Q-E13).

### 7.5 Capacité juridique

```
ent_eleve_capacite
  id UUID PK, etablissement_id, eleve_id
  nature    TEXT NOT NULL CHECK (nature IN ('emancipation','tutelle','curatelle','sauvegarde_justice','habilitation_familiale'))  -- [H : liste à valider juridiquement]
  du        DATE NOT NULL, au DATE NULL
  piece_document_id UUID NOT NULL, motif TEXT NULL
  saisi_par, saisi_le, levee_par, levee_le
```
- **Majorité** : calculée depuis la date de naissance courante (fuseau ~~Europe/Paris~~ **de l'établissement**, Europe/Paris par défaut — [personas] R-E15), jamais saisie ; jamais calculée sur une date de naissance `contestee` (R-E1). Le passage à 18 ans est détecté chaque nuit et publie `eleve.majeur` (DOS-14). Les effets sur les droits des parents dépendent de Q10 (non tranchée) ; le socle 2 ne fait que fournir le fait.
- **Mineur émancipé** : « capable, comme un majeur, de tous les actes de la vie civile » et « cesse d'être sous l'autorité de ses père et mère » **[V]** (C. civ. 413-6 et 413-7 ; VERIF R13). **Majeur protégé** : le tuteur ou curateur est un responsable (socle 4) avec des droits propres ; le socle 2 porte la mesure et sa pièce.

### 7.6 Confidentialité renforcée

```
ent_eleve_confidentialite
  id UUID PK, etablissement_id, eleve_id
  niveau       TEXT NOT NULL CHECK (niveau IN ('renforcee'))
  motif_scelle TEXT NOT NULL            -- chiffré (champs_chiffres), jamais dans un événement ni un journal en clair
  mesures      TEXT[] NOT NULL          -- masquer_photo, hors_annuaire, hors_trombinoscope_exporte, adresse_protegee (socle 4), alerte_consultation
  du DATE NOT NULL, au DATE NULL
  decide_par, decide_le, leve_par, leve_le
```
Effets [R] : l'élève sort de l'annuaire et de la recherche pour qui n'a pas de lien direct avec lui (portée « mes classes », lien d'enseignement, responsable de groupe, ou action `eleves.confidentiel.lire`) ; photo masquée hors des personnes autorisées ; chaque consultation est journalisée comme accès sensible (mécanisme `tracer_acces_sensible` existant [F]) ; les exports nominatifs signalent la présence d'élèves protégés avant envoi ; les envois de messages qui concerneraient une personne interdite sont bloqués par le socle 4. Cas d'usage [H] : enfant placé (aide sociale à l'enfance) dont l'adresse ne doit pas être connue, mesure de protection judiciaire, élève exposé médiatiquement.

### 7.7 Photo, coordonnées propres de l'élève

```
ent_eleve_photo
  id UUID PK, etablissement_id, eleve_id
  object_key  TEXT NOT NULL            -- stockage objet ; jamais servie sans URL pré-signée courte après contrôle des droits
  prise_le    DATE NULL
  source      TEXT NOT NULL CHECK (source IN ('etablissement','photographe','famille','eleve','import'))
  statut      TEXT NOT NULL CHECK (statut IN ('proposee','validee','retiree'))
  valide_par, valide_le, retire_motif
  -- ent_eleve.photo_id = photo validée la plus récente ; anciennes conservées N mois (paramètre), puis supprimées

ent_eleve_coordonnee                    -- coordonnées de l'élève lui-même (pas celles des responsables : socle 4)
  id UUID PK, etablissement_id, eleve_id
  type     TEXT NOT NULL CHECK (type IN ('email','mobile'))
  valeur   TEXT NOT NULL, verifiee_le TIMESTAMPTZ NULL
  usages   TEXT[] NOT NULL              -- notifications, compte, urgence
  source, du, au
```
La photo administrative (reconnaître un élève) et le **droit à l'image** (publication, journal, site web) sont deux choses différentes [H] : le second relève des consentements (D24, socle 4 ou formulaires), pas de ce socle.

### 7.8 Propositions et règles de champs

```
ent_eleve_proposition
  id UUID PK, etablissement_id
  eleve_id        UUID NULL             -- NULL = « élève inconnu » proposé par un import
  cible           TEXT NOT NULL CHECK (cible IN ('identite','identifiant','scolarisation','inscription','coordonnee','photo','nouvel_eleve'))
  champs          JSONB NOT NULL        -- {champ: {actuel, propose}} ; valeurs sensibles scellées
  source          TEXT NOT NULL CHECK (source IN ('import','famille','eleve','ancien_etablissement','educonnect','admission','systeme','personnel','verification_campagne'))  -- [personas] 'personnel' (ELV-40), 'verification_campagne' (ELV-32)
  source_ref      UUID NULL             -- lot d'import (IOP-L1), demande famille, dossier d'admission
  piece_document_id UUID NULL
  statut          TEXT NOT NULL CHECK (statut IN ('en_attente','acceptee','acceptee_partiellement','refusee','expiree','appliquee_auto','obsolete'))
  regle_auto      TEXT NULL             -- règle qui l'a appliquée automatiquement, le cas échéant
  decision_motif  TEXT NULL, decide_par, decide_le
  cree_le, expire_le

ent_param_eleve_champ                   -- paramétrage établissement, sur un catalogue de champs fourni par le logiciel
  etablissement_id, champ TEXT          -- champ du catalogue (nom_usage, prenom_usage, nationalite, naissance_commune, photo, email_eleve…)
  visible            BOOLEAN NOT NULL
  obligatoire_a      TEXT[] NOT NULL    -- moments : creation, inscription, export_bee, export_gar… (vide = facultatif)
  saisie_famille     TEXT NOT NULL CHECK (saisie_famille IN ('non','proposer','modifier'))
  source_qui_fait_foi TEXT NOT NULL CHECK (source_qui_fait_foi IN ('scolia','siecle','onde','reprise'))
  modification_locale TEXT NOT NULL CHECK (modification_locale IN ('libre','par_proposition','interdite'))
  PRIMARY KEY (etablissement_id, champ)
```
- Le **catalogue des champs** (noms, types, règles de format, champs jamais modifiables par la famille comme l'INE) est du **logiciel** ; leur réglage est du **paramétrage**, pré-rempli par un **profil d'établissement** (§9).
- Une **règle d'application automatique** est possible et toujours explicite (ex. « import officiel, champ géré par SIECLE, pas de conflit → appliqué ») ; elle est tracée sur la proposition.
- Une proposition acceptée devient une version (nature `import`, `correction` ou `changement_etat_civil` selon le choix fait à la décision), avec la proposition en `source_ref`.

### 7.9 Accès de l'élève à l'ENT

- Une seule règle [R] : `ent_eleve_compte (etablissement_id, eleve_id, user_id)` est **la** source du lien élève ↔ compte (existant [F `014`]). `ent_personne.eleve_id` (élève majeur comme personne payeuse ou contact, S2-1) **lit** le compte de l'élève au lieu d'en porter un autre ; un lien `ent_eleve_lien` « eleve_majeur » ne porte pas de `user_id` propre. Contrainte ou test de cohérence à ajouter.
- Règles d'âge et d'ouverture des comptes élèves : **paramétrage** (ex. pas de compte en maternelle et en élémentaire, compte à partir de la 6ᵉ ; ou compte de classe en primaire [H]).

### 7.10 Fusion de doublons d'élèves

```
ent_eleve_fusion
  id UUID PK, etablissement_id
  absorbe_id UUID NOT NULL, conserve_id UUID NOT NULL
  choix_champs JSONB NOT NULL           -- pour chaque champ d'identité, version retenue
  conflits_resolus JSONB NOT NULL       -- ex. deux inscriptions pour la même année : laquelle garder
  motif TEXT NOT NULL, fusionne_par, fusionne_le, annule_par, annule_le
ent_eleve_fusion_ligne
  fusion_id, table_nom TEXT, ligne_id UUID, colonne TEXT   -- chaque ligne déplacée, pour l'annulation
```
- **Registre des tables rattachées** dans le code (`TABLES_ELEVE`, comme `TABLES_CLASSE` [F `referentiel_classe_id.rs:19`]) avec une stratégie par table (`deplacer`, `conflit_unique` à résoudre avant, `projection` recalculée). Un **test de garde** lit `information_schema` et échoue si une colonne `eleve_id` n'est pas déclarée dans le registre : aucune table nouvelle n'échappe à la fusion (même esprit que la garde SQL statique [F `isolation_sql_statique.rs`]).
- L'élève absorbé n'est pas supprimé : `fusionne_dans` posé, toute lecture redirige. Annulation dans un délai paramétré par le logiciel (30 jours comme les personnes [F]), refusée si des lignes déplacées ont été modifiées depuis (liste montrée).

### 7.11 Indicateurs (pastilles)

```
ent_eleve_indicateur                    -- projection reconstruite depuis les événements (consommateur D0)
  etablissement_id, eleve_id, code TEXT, domaine TEXT, valeur JSONB NULL, du, au
  PRIMARY KEY (eleve_id, code)
ent_catalogue_indicateur                -- logiciel : code, domaine propriétaire, action requise pour le voir, libellé, icône, gravité
```
Exemples [R] : `pai_actif` (santé, action `sante.lire` ou pastille neutre « protocole » pour la vie scolaire, à décider par le domaine), `interne`, `boursier` (`bourses.gerer`), `majeur`, `confidentialite_renforcee`, `sans_ine`, `ine_conteste`, `absence_non_justifiee_en_cours`, `sortie_autorisee`. Chaque domaine **déclare** ses pastilles et l'action qui les ouvre ; le socle 2 ne fait que les afficher.

---

## 8. Données qui viennent de l'extérieur

### 8.1 L'INE : qui l'attribue, comment on le récupère

Faits sourcés et hypothèses (voir `INTEROPERABILITE_NATIONALE.md`) :
- L'INE est attribué par l'administration (répertoire national des identifiants élèves) lors de l'immatriculation dans les bases nationales : **ONDE** au premier degré, **SIECLE BEE** au second degré [H, mécanisme d'attribution exact NV]. Il suit l'élève d'un établissement à l'autre [H].
- **Public, second degré** : les données arrivent par l'**AAF** (SIECLE + STS-web → AAF → ENT), sous convention avec la DSI académique [S] ; des exports BEE existent aussi [S-tiers].
- **Privé sous contrat, second degré** : le logiciel de gestion **agréé** de l'établissement **envoie** les dossiers vers BEE (NetSynchro, puis OMOGEN) [S] ; l'INE est alors attribué ou retrouvé par BEE et revient au logiciel [H, canal de retour NV].
- **Premier degré** (public et privé sous contrat) : ONDE ; pour le privé, « interface école » par un éditeur validé [S]. Hors périmètre initial (Q8 de `SOCLE2_DOMAINES.md`), mais le modèle doit l'accepter.
- **Hors contrat** : pas d'accès à SIECLE ni à ONDE [H]. L'élève peut avoir un INE s'il vient d'un établissement public ou sous contrat ; sinon il n'en a pas. L'établissement hors contrat a des obligations déclaratives propres (liste des élèves au maire ou à l'autorité académique ? [NV]).

Conséquence pour le modèle [R] — l'INE se récupère par **cinq voies**, chacune avec sa provenance et son statut :

| Voie | Provenance | Statut à l'entrée | Pour qui |
|---|---|---|---|
| Import officiel (AAF, BEE, ONDE) | `import_aaf`, `import_bee`, `import_onde` | `confirme` (sauf conflit) | public ; privé selon les flux |
| Retour de la remontée BEE | `retour_bee` | `confirme` | privé sous contrat, quand ScolIA remonte (IOP-L4) |
| Dossier de l'ancien établissement (exeat, bulletin, livret LSU) | `ancien_etablissement` | `a_verifier` | tous |
| Famille (lors de l'inscription, socle 3) | `document_famille` | `declare` | tous, surtout privé et hors contrat |
| Reprise d'un ancien logiciel | `reprise_logiciel` | `a_verifier` | à la mise en service |

Jamais d'INE inventé ni obligatoire pour créer un élève : un élève sans INE est normal (maternelle, hors contrat, arrivée de l'étranger). Le champ est `obligatoire_a = export_bee` (par exemple), pas `creation`.

### 8.2 Les autres flux, vus depuis l'élève

| Flux | Sens | Ce qu'il apporte à l'élève | Comment il entre | Lot |
|---|---|---|---|---|
| AAF (public) | entrant | état civil, INE, structures, jointure ÉduConnect | propositions, application automatique paramétrée (« SIECLE fait foi ») | IOP-L2 + E5 |
| BEE exports | entrant | idem, plus MEF, divisions | propositions | IOP-L3 + E5 |
| Remontée BEE (privé) | sortant | — (envoie l'état civil, la scolarité) | export depuis les données confirmées ; rejets → propositions de correction | IOP-L4 |
| ONDE (premier degré) | entrant/sortant | idem premier degré | propositions | hors socle 2 initial |
| ÉduConnect | entrant (connexion) | rien sur l'identité scolaire ; rattache un compte par jointure | socle 1 / IOP-L5 | IOP-L5 |
| GAR | sortant | annuaire minimal (pas d'INE) | export | IOP-L7 |
| Ancien établissement (exeat, dossier) | entrant | INE, origine, parfois dossier | pièce + propositions | socle 3 |
| Famille (préinscription, portail) | entrant | état civil, coordonnées de l'élève, photo | propositions (`saisie_famille = proposer`) ou modification directe si paramétrée | socle 3 + E5 |
| Ancien logiciel (Charlemagne, Pronote, École Directe…) | entrant, une fois | tout | lot de reprise, propositions en masse, rapport | E5 + IOP-L1 |
| Nomenclatures (BCN : MEF ; COG INSEE : communes, pays) | entrant | listes de référence | référentiel du logiciel, versionné | IOP-L0 |

### 8.3 Règle de rapprochement d'un import [R]

1. INE confirmé identique → même élève.
2. Identifiant source déjà connu (identifiant élève SIECLE, ONDE) → même élève.
3. Sinon score d'homonymes (nom de naissance, prénoms, date de naissance, sexe) : ≥ seuil haut → proposition « même élève ? » à confirmer ; zone grise → proposition « nouvel élève ou existant ? » ; sinon « nouvel élève » (`eleve_id` NULL).
4. Jamais d'écrasement : chaque différence de champ devient une ligne de proposition ; l'application automatique ne joue que pour les champs dont l'établissement a dit que la source fait foi, et jamais quand une modification locale plus récente existe (elle passe alors en conflit visible).

---

## 9. Ce qui relève du logiciel, ce qui relève du paramétrage

### 9.1 Profil d'établissement (préréglages)

Le logiciel propose des **profils** qui pré-remplissent le paramétrage ; l'établissement les ajuste. Champs à ajouter au niveau de l'établissement [R] : `statut_juridique` (`public`, `prive_sous_contrat_association`, `prive_sous_contrat_simple`, `prive_hors_contrat`), `degres` (premier, second), `tutelle` (éducation nationale, agriculture… [H]), UAI par site (existe [F]).

| Réglage | Public 2ᵈ degré (AAF) | Privé sous contrat 2ᵈ degré (ScolIA remonte vers BEE) | Privé sous contrat, cohabitation avec un logiciel agréé | Hors contrat | Premier degré |
|---|---|---|---|---|---|
| Source qui fait foi pour l'état civil et l'INE | SIECLE | ScolIA | le logiciel agréé (via imports) | ScolIA | ONDE |
| Modification locale de l'état civil | par proposition (« à corriger dans SIECLE ») | libre (avec nature et pièce) | par proposition | libre | par proposition |
| INE | confirmé par import | confirmé par retour BEE | confirmé par import | déclaré seulement | confirmé par ONDE |
| Comptes élèves | à partir de la 6ᵉ | idem | idem | au choix | non, ou compte de classe |

### 9.2 Tableau logiciel / paramétrage

| Sujet | Logiciel (identique partout, non paramétrable) | Paramétrage de l'établissement |
|---|---|---|
| Structure des données, versions, historique, nature des modifications | ✔ | — |
| Catalogue des champs, formats, contrôles (INE par famille, codes COG, sexe à l'état civil) | ✔ | — |
| Champs visibles, obligatoires (et à quel moment), saisissables par la famille | — | ✔ (`ent_param_eleve_champ`) |
| Source qui fait foi par champ, modification locale | catalogue des sources | ✔ |
| Prénom d'usage | mécanisme, traçabilité, protection | activation, procédure (accord des parents, entretien) |
| Nationalité | champ disponible, protégé | collecte activée ou non (Q-E5) |
| Numéro d'élève | unicité, non-réutilisation | format (`{AAAA}-{seq:5}`, préfixe de site…), remise à zéro annuelle ou non |
| Statuts de l'élève | liste fixe et dérivée | libellés affichés |
| Motifs d'entrée et de sortie | codes nationaux du logiciel | motifs locaux rattachés à un code |
| Régimes | codes du logiciel (à mapper aux codes nationaux) | régimes locaux et libellés (ex. « interne-externé »), rattachés à un code |
| Informations complémentaires | types, historique, protection | champs propres à l'établissement (existant [F `072`]) ; interdiction affichée d'y mettre des données de santé |
| Photos | stockage privé, URL courtes, versions | qui peut proposer (famille, élève), durée de conservation des anciennes photos dans les bornes du logiciel |
| Comptes élèves | sécurité (socle 1) | âge ou niveau d'ouverture |
| Rôles et droits | catalogue d'actions, garde-fous, portées | rôles, actions cochées, portées (D-017) |
| Confidentialité renforcée | mécanisme et effets minimaux non désactivables | mesures supplémentaires |
| Délai d'annulation d'une fusion, durées de conservation | bornes (validées DPO) | — (pas de réglage local des durées légales) |
| Règles d'application automatique des propositions | règles proposées | activation par champ et par source |

---

## 10. Cloisonnement, sécurité, données sensibles

### 10.1 Cloisonnement par établissement

- `etablissement_id NOT NULL` partout, déduit de la session et de l'adresse (D-012), jamais du client.
- **Clés étrangères composées** `(etablissement_id, eleve_id)` → impossible en base de rattacher une donnée à l'élève d'un autre établissement (plus fort que le trigger actuel qui ne protège que `classe_id` [F `111`]).
- Un même enfant dans deux établissements ScolIA = deux élèves. **Aucun** rapprochement, aucune unicité d'INE, aucune recherche entre établissements (D-013). Le transfert de données d'un établissement à l'autre ne passe que par les voies officielles (exeat, dossier transmis) ou par la famille (pré-remplissage D-014, socle 3).
- Tests de séparation (`ent-api/tests/separation.rs` [F]) étendus à toutes les nouvelles routes, imports et exports ; garde SQL statique étendue aux nouvelles tables.

### 10.2 Groupes de champs et minimisation

La minimisation actuelle par profil (`NiveauIdentite` Complet / Santé / Réduit [F]) est remplacée par des **groupes de champs ouverts par des actions** (§12), calculés en un point unique côté serveur ; les DTO n'exposent que les groupes autorisés et disent **pourquoi** un groupe est masqué (code de raison, pour l'écran).

| Groupe | Champs | Ouvert par |
|---|---|---|
| G0 désignation | nom et prénom affichés, classe, photo (sauf confidentialité), statut | `eleves.annuaire` |
| G1 scolarité | numéro d'élève, niveau, MEF, régime, site, groupes, dates d'entrée et de sortie, âge, majeur | `eleves.lire` |
| G2 état civil | nom de naissance, nom d'usage, prénoms, sexe, date et lieu de naissance | `eleves.etat_civil.lire` |
| G3 identifiants | INE, identifiants SIECLE et ONDE, statut et provenance | `eleves.identifiants.lire` |
| G4 nationalité (si collectée) | nationalité | `eleves.etat_civil.lire` + champ activé |
| G5 historique | versions d'identité non sensibles, propositions décidées | `eleves.historique.lire` |
| G6 sensible | versions sensibles, motif de confidentialité | `eleves.confidentiel.gerer` |

Les familles et l'élève (hors rôles libres, D-017) voient G0 à G3 de **leur** enfant ou d'eux-mêmes (selon lien actif et droits du lien), jamais G5 ni G6, et les propositions qu'ils ont déposées.

### 10.3 RLS

Ajouter à la RLS (migration dédiée, mode permissif puis strict selon `RLS_ISOLATION.md`) : `ent_eleve`, `ent_eleve_identite`, `ent_eleve_identifiant`, `ent_eleve_scolarisation`, `ent_eleve_inscription`, `ent_eleve_periode`, `ent_eleve_capacite`, `ent_eleve_confidentialite`, `ent_eleve_photo`, `ent_eleve_coordonnee`, `ent_eleve_proposition`, `ent_eleve_fusion*`, `ent_eleve_indicateur`, `ent_param_eleve_champ`, puis `ent_personne`, `ent_eleve_lien`, `ent_eleve_contact_urgence` (socle 4). Les dépôts correspondants passent par `TenantTx`. Signaler au passage l'écart de nom de variable relevé dans `outbox.rs:638` (`ent.etablissement_id` contre `app.etablissement_id`) [F rapport, à vérifier].

### 10.4 Chiffrement applicatif (champs scellés)

| Donnée | Chiffrée ? | Raison [R] |
|---|---|---|
| Nom, prénoms, date de naissance **courants** | non | nécessaires à la recherche et aux listes ; protégés par la RLS, les droits et la minimisation |
| Versions d'identité **sensibles** non courantes (ancien prénom, ancienne mention de sexe) | **oui** (`valeurs_scellees`) | révèlent une information intime ; jamais cherchées |
| Motif de confidentialité renforcée | **oui** | souvent judiciaire ou social |
| Nationalité | non par défaut ; **oui** si collectée [R, à valider DPO] | donnée qui peut révéler une origine ; rarement lue |
| INE | non | identifiant, nécessaire aux exports ; minimisé (G3) et jamais envoyé au GAR |
| Pièces justificatives d'identité | stockage objet privé, URL courtes (D9) | — |
| Charges utiles d'événements | **aucune donnée d'identité** | remplacer les valeurs par des identifiants de versions et des noms de champs |
| Valeurs sensibles dans les propositions | **oui** | même raison que les versions |

Rappel des limites existantes [F `SOCLE1_SECURITE.md` D3] : pas encore de rotation de clé (`enc:v2:` avec identifiant de clé à prévoir avant production).

### 10.5 Journal de consultation

Toute lecture de G2, G3, G5, G6, toute ouverture de la fiche d'un élève en confidentialité renforcée, tout export nominatif est journalisé (acteur réel, objet, motif quand il est demandé). Visible de la direction (`audit.lire`) ; visibilité par la famille selon Q9 de `SOCLE2_DOMAINES.md` (recommandation : par service).

### 10.6 Conservation, effacement, droits des personnes

- **Pas de suppression définitive** d'un élève, sauf **annulation d'une création erronée** (aucune donnée rattachée, délai court, trace conservée) [R, Q-E8]. Un doublon se fusionne ; un élève parti se clôt.
- Archivage et anonymisation planifiés selon des **durées à valider par le DPO** (ne pas inventer de durée) ; les dossiers d'élèves relèvent des règles d'archives publiques dans le public **[V]** (C. patr. L211-4 1°, élimination soumise au visa, R212-14 ; VERIF R39-R41) et **peut-être aussi dans le privé sous contrat** : L211-4 2° vise les documents « qui procèdent … de l'exercice d'une mission de service public par des personnes de droit privé » (avis du juriste requis).
- Droit d'accès et de rectification : l'extrait RGPD de l'élève (PLT-08) et la **proposition** (rectification tracée, avec pièce) en sont les outils.
- Responsable de traitement : l'établissement pour les données de l'élève ; ScolIA sous-traitant (`IDENTITE_ET_PROFILS_FAMILLES.md` §2).

---

## 11. Actions métier

Notation des acteurs comme `SOCLE2_DOMAINES.md` §3 : ADF administratif, DIR direction, CPE, PRF professeur, PP, INF, PSY, CPT comptabilité, PAR responsable, ELV élève, SYS système. ★ = action sensible (confirmation, et ré-authentification quand elle sera branchée). Correspondance avec les anciens identifiants `DOS-xx`/`INS-xx` indiquée.

| ID | Action | Acteur · droit | Règles clés | Effets | Événements | Remplace |
|---|---|---|---|---|---|---|
| ELV-01 | Créer un élève | ADF · `eleves.creer` ; SYS (socle 3, imports) | détection de doublons **obligatoire** (score) ; si un ancien élève correspond → proposer ELV-12 (réintégration) ; numéro attribué ; version d'identité `creation` ; aucune exigence d'INE | élève + version + scolarisation éventuelle | `eleve.created` | DOS-01 |
| ELV-02 | Annuler une création erronée | ADF, DIR · `eleves.creer` ★ | aucune donnée rattachée (registre des tables), délai court | élève marqué annulé (trace) | `eleve.creation_annulee` | — |
| ELV-03 | Corriger l'identité | ADF · `eleves.identite.modifier` | nature `correction`, motif obligatoire, pièce selon paramétrage ; champ verrouillé si une source externe fait foi → proposition « à corriger dans la source » | version de remplacement | `eleve.identity_changed` | DOS-02 |
| ELV-04 | Enregistrer un changement d'état civil | ADF · `eleves.identite.modifier` ★ | date d'effet, pièce **obligatoire** | nouvelle version | `eleve.identity_changed` | DOS-02 |
| ELV-05 | Définir ou retirer un nom ou prénom d'usage | ADF · `eleves.identite.modifier` ; demande PAR/ELV | procédure paramétrée ; versions `sensible` pour le prénom d'usage | nouvelle version | `eleve.identity_changed` (sans valeur) | — |
| ELV-06 | Consulter l'historique d'identité | `eleves.historique.lire` ; versions sensibles : `eleves.confidentiel.gerer` | lecture journalisée | — | — | — |
| ELV-07 | Ajouter, confirmer, contester, révoquer un identifiant (INE…) | ADF · `eleves.identifiants.gerer` ; SYS (import) | format par système ; collision → `conteste` + alerte | identifiant | `eleve.identifiant_change` | — |
| ELV-08 | Ouvrir une scolarisation (entrée) | ADF · `eleves.scolarite.gerer` ; SYS (socle 3 : inscription validée) | pas de chevauchement ; origine | scolarisation, statut recalculé | `scolarite.opened` (réservé [F], enfin émis) | INS-05 (partie) |
| ELV-09 | Enregistrer ou modifier une inscription annuelle | ADF · `eleves.scolarite.gerer` ; SYS (réinscription, montée) | une par année ; niveau / MEF cohérents avec la classe | inscription | `eleve.scolarite_annee_enregistree` [F] | INS-09, INS-11 |
| ELV-10 | Changer de régime ou de site (daté) | ADF · `eleves.scolarite.gerer` | date d'effet ; impacts signalés (facturation, restauration, internat) | période | `eleve.regime_changed` [F], `scolarite.site_changed` [F] | INS-10 |
| ELV-11 | Clôturer la scolarisation (sortie, radiation) | ADF, DIR · `eleves.sortie` ★ | date, motif, destination ; affectations fermées ; soldes et documents en cours signalés par les domaines | scolarisation close, statut recalculé | `eleve.sortie` [F], `scolarite.closed` (réservé [F]) | INS-12 |
| ELV-12 | Réintégrer un ancien élève | ADF · `eleves.scolarite.gerer` | même fiche ; nouvelle scolarisation ; revue de l'identité proposée | scolarisation | `eleve.reintegre` | — |
| ELV-13 | Déclarer ou lever une capacité (émancipation, mesure de protection) | ADF, DIR · `eleves.identite.modifier` ★ | pièce obligatoire | capacité ; droits recalculés | `eleve.capacite_changee` | — |
| ELV-14 | Passage à la majorité | SYS (chaque nuit) | date de naissance courante | droits recalculés selon Q10 | `eleve.majeur` | DOS-14 |
| ELV-15 | Poser ou lever la confidentialité renforcée | DIR · `eleves.confidentiel.gerer` ★ | motif chiffré ; mesures | marquage | `eleve.confidentialite_changee` (sans motif) | — |
| ELV-16 | Proposer, valider, retirer une photo | ADF · `eleves.photo.gerer` ; PAR/ELV si paramétré | type de fichier contrôlé (D2) ; une validée courante | photo | `eleve.photo` [F] | DOS-11 |
| ELV-17 | Gérer les coordonnées propres de l'élève | ADF · `eleves.identite.modifier` ; ELV (soi), PAR (proposition) | vérification de l'e-mail par code | coordonnée | `eleve.coordonnees_modifiees` (noms de champs) | — |
| ELV-18 | Déposer une proposition (signaler une erreur) | PAR (lien actif), ELV (soi, majeur), SYS (import) | pièce selon le champ ; jamais sur un champ interdit à la famille | proposition | `eleve.proposition_deposee` | — |
| ELV-19 | Décider une proposition (accepter tout ou partie, refuser) | ADF · `eleves.propositions.traiter` | nature choisie à l'acceptation ; refus motivé, motif visible du demandeur | version(s) | `eleve.proposition_decidee` | — |
| ELV-20 | Rechercher un élève (annuaire) | personnel · `eleves.annuaire` | tolérante aux accents et aux fautes ; INE et badge seulement avec les droits correspondants ; confidentialité renforcée exclue | lecture | — | PLT-10 (partie) |
| ELV-21 | Consulter la fiche (hub) | selon groupes et portées ; PAR/ELV par lien | sections et pastilles selon les droits ; raisons de masquage | lecture + journal si sensible | — | DOS-13 |
| ELV-22 | Détecter les doublons | SYS (nuit) ; ADF | score ; INE en conflit | liste | — | — |
| ELV-23 | Fusionner deux élèves | DIR · `eleves.fusionner` ★ | aperçu par domaine ; conflits résolus avant ; registre complet | fusion | `eleve.fusionne` | DOS-15 (élèves) |
| ELV-24 | Annuler une fusion | DIR · `eleves.fusionner` ★ | délai ; aucune ligne déplacée modifiée depuis | défusion | `eleve.defusionne` | — |
| ELV-25 | Exporter une liste nominative | `eleves.exporter` | colonnes limitées aux groupes autorisés ; élèves protégés signalés ; journalisé | fichier (URL courte) | `export.realise` | — |
| ELV-26 | Paramétrer les champs, la numérotation, régimes, motifs | ADM · `eleves.parametrer` | dans les bornes du logiciel | paramétrage | `parametrage.eleve_modifie` | — |
| ELV-27 | Consulter la qualité des données | ADF, DIR · `eleves.lire` + `eleves.identifiants.lire` | indicateurs calculés | lecture | — | — |
| ELV-28 | Ouvrir ou fermer le compte de l'élève | ADF · `eleves.gerer` (existant) | règles d'âge paramétrées ; invitation (existant [F]) | compte | `eleve.compte_*` [F] | DOS-09 (partie) |
| ELV-29 [personas] | Enregistrer une date de naissance **incomplète ou présumée** (précision : jour, mois, année, inconnue ; statut : déclarée, justifiée, présumée, contestée) | ADF · `eleves.identite.modifier` | jamais de « 1ᵉʳ janvier » inventé sans marque ; affichage « né(e) en 2012 (date présumée) » ; âge et majorité calculés sur la date retenue avec avertissement, **suspendus** si `contestee` (mineur non accompagné dont la minorité est en cours d'évaluation) ; les exports appliquent la règle de leur connecteur et signalent la précision [NV] (R-E1) | version | `eleve.identity_changed` (champs) | — |
| ELV-30 [personas] | Enregistrer l'écriture d'origine du nom et ses autres graphies (translittérations) | ADF · `eleves.identite.modifier` | l'écriture d'origine est dans la version (facultative) ; les autres graphies servent **seulement** à la recherche et au rapprochement des imports, jamais aux documents officiels ; lues avec G2 (R-E2) | version ; alias | `eleve.identity_changed` (champs) | — |
| ELV-31 [personas] | Corriger **en lot** la typographie (majuscules, accents, espaces, apostrophes) après un import ou une reprise | ADF · `eleves.identite.modifier` ★ | aperçu avant / après élève par élève, décochable ; nature `correction`, motif codé `harmonisation_typographique`, sans pièce ; jamais sur un champ dont une source externe fait foi (proposition « à corriger dans la source ») | versions | `eleve.identity_changed` (une par élève, regroupées pour les consommateurs) | — |
| ELV-32 [personas] | Lancer une **campagne de vérification de l'état civil** par les familles (avant les inscriptions aux examens, avant la remontée BEE, à l'entrée en 3ᵉ ou en terminale) | ADF · `eleves.identite.modifier` + `eleves.propositions.traiter` | chaque titulaire, et l'élève majeur, voit l'état civil **exactement tel qu'il sera imprimé** (accents, ordre des prénoms, lieu de naissance) ; « c'est exact » ou proposition avec pièce ; relances plafonnées ; tableau de complétude par classe ; aucune modification directe | propositions (source `verification_campagne`) | `eleve.verification_identite_ouverte` / `_close` | — |
| ELV-33 [personas] | Déclarer deux fiches comme **distinctes** (jumeaux, homonymes) | ADF · `eleves.identite.modifier` | décision mémorisée ; la paire n'est plus proposée par la détection, sauf si l'identité de l'une change (R-E20) | marque | `eleve.non_doublon_declare` | — |
| ELV-34 [personas] | **Enregistrer le décès d'un élève** ★ | DIR · `eleves.sortie` | scolarisation close, motif `deces` ; **arrêt immédiat de tous les envois automatiques** le concernant (relances, réinscription, fiche de rentrée, factures et relances de paiement, alertes d'absence, notifications de notes) ; retrait des appels et trombinoscopes à la date ; aucun courrier ni certificat automatique ; fratrie signalée à la direction ; information des équipes rédigée par la direction (R-E12) | scolarisation close | `eleve.decede` (sans détail) | — |
| ELV-35 [personas] | Ouvrir ou fermer une **période de mobilité** (année ou trimestre à l'étranger, soins longs, CNED temporaire) avec retour prévu | ADF · `eleves.scolarite.gerer` | la scolarisation **n'est pas close** ; pas de certificat de radiation ; place réservée pour N+1 (socle 3, INS-71) ; appel neutralisé sur la période (socle 12) ; retour constaté ou transformé en sortie (R-E13) | période | `scolarite.mobilite_ouverte` / `_fermee` | — |
| ELV-36 [personas] | Enregistrer une **modalité de scolarisation** datée (temps partiel, double inscription CNED, scolarisation partagée avec un établissement médico-social, UPE2A, ULIS [NV]) | ADF · `eleves.scolarite.gerer` | codes du logiciel, libellés paramétrables ; lue par les socles 10, 12, 23, 35 ; le contenu pédagogique reste au socle 35 (R-E14) | modalité | `scolarite.modalite_changee` | — |
| ELV-37 [personas] | Éditer le **registre des élèves** (entrées et sorties, ordre chronologique) à une date | ADF, DIR · `eleves.exporter` | reproductible à toute date passée ; colonnes minimales (R-E18) ; filigrane ; journalisé ; utile en inspection et contrôle [NV] | fichier (URL courte) | `export.realise` | — |
| ELV-38 [personas] | Délivrer une **attestation de scolarité pour une période passée** (ancien élève) | ADF · `eleves.lire` ; ancien élève (demande) | lit les scolarisations et inscriptions conservées ; identité du demandeur vérifiée ; modèle bilingue possible (R-E17) ; impossible si les données ont été anonymisées (le dire) | document (socle 7) | `document.emis` | — |
| ELV-39 [personas] | **Préparer la majorité** (J-30) | SYS | un mois avant : l'élève est informé de ce qui change et des choix à faire (ce que ses parents continuent de recevoir, ses coordonnées propres, ses justifications) ; les titulaires reçoivent un message neutre ; tâche au secrétariat si l'élève n'a ni e-mail ni mobile vérifié ; rien ne bascule avant la date (socle 4, R-A25) | tâche, messages | `eleve.majorite_proche` | — |
| ELV-40 [personas] | **Signaler une erreur d'identité vue sur le terrain** (« il s'appelle Yanis, pas Yannis ») | personnel · `eleves.annuaire` | crée une proposition `source = personnel` vers le secrétariat ; aucune modification directe ; l'auteur est informé de la décision | proposition | `eleve.proposition_deposee` | — |
| ELV-41 [personas] | Déclarer un **badge perdu** et attribuer un nouveau badge | ADF, VS · `eleves.badge.gerer` (nouvelle action, système `BADGE` seulement) | l'ancien est révoqué immédiatement (cantine, portail) ; le nouveau actif tout de suite ; refacturation éventuelle (socle 24) | identifiant | `eleve.identifiant_change` | — |
| ELV-42 [personas] | Enregistrer le **refus de la photo administrative** | ADF · `eleves.photo.gerer` | silhouette dans les listes ; l'élève sort de « photos manquantes » ; aucune relance ; révocable par la famille | marque | `eleve.photo` | — |
| ELV-43 [personas] | Indiquer la **prononciation** du prénom et du nom | ELV (soi), PAR (proposition), ADF | texte phonétique simple ou court enregistrement audio, facultatif ; visible de qui voit G0 dans l'établissement ; jamais exporté | donnée facultative | — | — |
| ELV-44 [personas] | Ouvrir un **accès exceptionnel motivé** (« bris de glace ») à la scolarité et à l'état civil d'un élève hors de sa portée | personnel désigné · `eleves.acces_urgence` (nouvelle action) | motif choisi dans une liste (disparition, danger, urgence médicale) ; durée courte (défaut 2 h, bornes 30 min – 24 h) ; jamais G6 ni santé ; alerte immédiate à la direction ; revue a posteriori obligatoire (R-E11) | accès temporaire | `acces.urgence_ouvert` | — |
| ELV-45 [personas] | Voir les **arrivées, départs et changements** récents de mes classes et groupes | PRF, PP, CPE · portée | depuis une date choisie (défaut : 30 jours) ; sans motif de sortie ; lien vers la fiche | lecture | — | — |
| ELV-46 [personas] | **Déclarer un lien personnel** avec un élève (son enfant, un proche) — conflit d'intérêts | personnel (soi) ; DIR ; SYS (détection à partir des socles 4 et 6) | accès à la fiche de cet élève par la casquette professionnelle tracé spécialement ; aucune décision sensible sur lui (fusion, confidentialité, sortie, correction d'identité) ; données des autres responsables fermées (socle 4, R-A18) (R-E10) | marque | `personnel.lien_declare` (sans détail) | — |
| ELV-47 [personas] | Faire suivre les **identifiants de connexion et l'adresse de messagerie** interne après un changement de prénom d'usage ou de nom | ELV (demande), ADF | le socle 1 renomme ; l'ancien identifiant reste un alias caché un temps borné, jamais affiché aux autres ; aucune perte d'historique (R-E16) | (socle 1) | `compte.identifiant_renomme` | — |
| ELV-48 [personas] | Choisir l'établissement d'origine ou de destination dans l'**annuaire national des établissements** (UAI) | ADF · `eleves.scolarite.gerer` ; SYS (socles 3, 9) | recherche par nom, commune, UAI ; texte libre de secours (étranger, instruction en famille) ; référentiel du logiciel mis à jour par le socle 9 (R-E23) — couverture Charlemagne (D-024) | scolarisation | — | — |

---

## 12. Droits (catalogue `ent_auth::Action`)

### 12.1 Existant [F]

`eleves.gerer` (établissement seulement ; Admin, Direction, Administratif) couvre aujourd'hui création, identité, classe, liens, comptes et inscriptions. `dossier.lire` (quatre portées) ouvre la fiche ; il n'existe **pas** d'action de lecture d'identité : la lecture passe par `can_read_all_eleves` (tout le personnel) et `NiveauIdentite` par profil.

### 12.2 Actions proposées [R]

| Action | Libellé | Portées admises | Modèle par défaut (= comportement actuel) |
|---|---|---|---|
| `eleves.annuaire` | Trouver un élève et voir sa désignation (G0) | établissement, mes classes, mes groupes, désignés | tous les profils du personnel, portée établissement (comportement actuel de `/eleves/search`) |
| `eleves.lire` | Voir la scolarité (G1) | 4 portées | Admin, Direction, Administratif, CPE, Compta, Infirmerie, Psy : établissement ; Prof : établissement (comportement actuel de `GET /eleves/{id}`, question Q-E3 pour « mes classes ») |
| `eleves.etat_civil.lire` | Voir l'état civil (G2) | 4 portées | Admin, Direction, Administratif ; Infirmerie (actuel niveau « Santé ») |
| `eleves.identifiants.lire` | Voir l'INE et les identifiants (G3) | 4 portées | Admin, Direction, Administratif |
| `eleves.historique.lire` | Voir l'historique d'identité (G5) | établissement | Admin, Direction, Administratif |
| `eleves.creer` | Créer un élève, annuler une création erronée | établissement | Admin, Direction, Administratif |
| `eleves.identite.modifier` | Corriger, changer l'état civil, usages, capacité, coordonnées de l'élève | établissement | idem |
| `eleves.identifiants.gerer` | Gérer INE et identifiants | établissement | idem |
| `eleves.scolarite.gerer` | Scolarisations, inscriptions annuelles, régime, site, réintégration | établissement | idem |
| `eleves.sortie` | Clôturer une scolarisation (radiation) | établissement | Admin, Direction, Administratif |
| `eleves.photo.gerer` | Gérer les photos | établissement | Admin, Direction, Administratif, CPE [R] (aujourd'hui : rôles du poste de la photo [F, à vérifier]) |
| `eleves.propositions.traiter` | Décider les propositions | établissement | Admin, Direction, Administratif |
| `eleves.fusionner` | Fusionner et défusionner des élèves | établissement | Admin, Direction (comme `personnes.fusionner`) |
| `eleves.confidentiel.gerer` | Confidentialité renforcée, versions sensibles (G6) | établissement | Admin, Direction |
| `eleves.exporter` | Exports nominatifs | établissement | Admin, Direction, Administratif |
| `eleves.parametrer` | Paramétrage élève de l'établissement | établissement | Admin |

Règles [R] :
- **Compatibilité** : `eleves.gerer` reste dans le catalogue ; dans les modèles par défaut, il est accompagné des nouvelles actions d'écriture, si bien que rien ne change pour les profils existants. Test oracle profil × action identique à `r2_modele_identique_aux_capacites_historiques` [F]. Un rôle créé par l'établissement peut ensuite séparer (« Accueil » : `eleves.annuaire` + `eleves.lire` ; « Secrétariat d'un site » : écritures sur un site seulement, voir Q-E9).
- **Inclusions explicites** (comme `facturation.gerer` inclut `facturation.lire` [F]) : `eleves.identite.modifier` inclut `eleves.etat_civil.lire` ; `eleves.identifiants.gerer` inclut `eleves.identifiants.lire`. Aucune autre inclusion implicite.
- **Anti-escalade** inchangée ; `eleves.confidentiel.gerer` et `eleves.fusionner` ne peuvent être donnés que par qui les détient.
- **Familles et élèves** hors rôles libres : leurs droits sur l'identité viennent du lien actif (`ent_eleve_lien_actif`) et de la capacité juridique, avec les champs autorisés par `ent_param_eleve_champ.saisie_famille`.
- Les routes ne font qu'un pré-filtre ; le service refait le contrôle précis (`autoriser_eleve`), comme en R2 [F].

---

## 13. Événements (outbox, dans la transaction du fait)

| Événement | Charge utile (jamais de valeur d'identité) | Consommateurs [R] |
|---|---|---|
| `eleve.created` [F] | `eleve_id`, numéro, source, identifiant de version | recherche, indicateurs, chronologie |
| `eleve.creation_annulee` | `eleve_id`, motif codé | idem |
| `eleve.identity_changed` [F] | identifiant de version, nature, **noms** des champs changés, sensible oui/non | recherche, projections d'affichage des domaines, notification de l'établissement si la source est externe, GAR/AAF plus tard |
| `eleve.identifiant_change` | système, statut avant/après (pas la valeur) | qualité des données, IOP |
| `scolarite.opened` / `scolarite.closed` [F, réservés] | `scolarisation_id`, dates, motif codé | droits (accès famille), facturation, restauration, internat, référentiel (affectations) |
| `eleve.scolarite_annee_enregistree` [F] | `inscription_id`, année, état | référentiel, réinscriptions (socle 3) |
| `eleve.regime_changed` [F], `scolarite.site_changed` [F] | période | facturation, restauration, internat |
| `eleve.status_changed` [F] | ancien et nouveau statut | tous les domaines (accès, listes) |
| `eleve.sortie` [F], `eleve.reintegre` | `scolarisation_id` | idem |
| `eleve.majeur`, `eleve.capacite_changee` | nature | droits famille, notifications (Q10) |
| `eleve.confidentialite_changee` | posé / levé (sans motif ni mesures détaillées) | recherche, trombinoscope, messagerie |
| `eleve.photo` [F] | `photo_id`, statut | trombinoscope, appli mobile |
| `eleve.proposition_deposee` / `_decidee` | `proposition_id`, cible, source, décision | notification du demandeur (famille), qualité des données |
| `eleve.fusionne` / `eleve.defusionne` | deux `eleve_id`, `fusion_id` | tous les domaines qui gardent des caches ; recherche |

Convention : les nouveaux noms suivent le style français des derniers événements (`personne.creee`, `lien.suspendu` [F]) ; les noms existants sont conservés pour ne rien casser.

---

## 14. Écrans clés

Leptos 0.8 (conventions de `docs/FRONTEND_LEPTOS.md`), données et actions fournies par le serveur, design system maison (tableau virtualisé, formulaires). Toutes les actions proposées viennent du serveur (corriger l'écart actuel du bouton « Proposer un contact » calculé côté client [F]).

| Écran | Route [R] | Contenu | Pour qui |
|---|---|---|---|
| **Recherche instantanée** | partout (raccourci clavier) | nom, prénom, nom d'usage, numéro, INE (si droit), badge (si droit) ; tolérante aux accents et fautes ; photo, classe, statut ; « ouvrir la fiche » | tout le personnel |
| **Liste des élèves** | `/web/eleves` | tableau dense virtualisé (5 000 lignes) ; filtres : année, statut, niveau, classe, groupe, régime, site, pastilles, qualité ; colonnes au choix limitées aux groupes autorisés ; vues enregistrées ; actions groupées (exporter, trombinoscope, message, étiquettes) | personnel |
| **Fiche élève (hub)** | `/web/eleves/{id}` | **en-tête « carte »** : photo, nom affiché, classe, âge, numéro, statut, pastilles ; **barre d'actions** du serveur ; **onglets** : Synthèse · Identité · Scolarité · Responsables (socle 4) · Vie scolaire · Évaluations · Santé · Documents · Finances · Chronologie · Historique. La **Synthèse** est une grille de tuiles, une par domaine, chaque tuile fournie par son domaine et masquée sans droit ; chaque champ masqué dit pourquoi | selon droits |
| **Identité** | onglet | valeurs courantes par groupe ; cadenas « géré par SIECLE » ; bouton **Modifier** → choix de la nature (« corriger une erreur », « enregistrer un changement d'état civil », « nom ou prénom d'usage ») → champs → date d'effet → pièce → motif → **récapitulatif avant / après** → confirmation | secrétariat |
| **Historique d'identité** | onglet Historique | frise par champ : valeur, nature, date d'effet, enregistré par, source, pièce ; corrections visibles comme telles ; versions sensibles réservées | droit G5 / G6 |
| **Identifiants** | dans Identité | INE avec pastille d'état (déclaré, à vérifier, confirmé, contesté), provenance, preuve ; « confirmer », « contester » | secrétariat |
| **Scolarité (parcours)** | onglet | frise multi-années : scolarisations (entrée / sortie, origine / destination), inscriptions annuelles, classes et groupes (référentiel), régime et site ; actions : changer de régime, enregistrer une sortie, réintégrer | secrétariat, direction |
| **Créer un élève** | `/web/eleves/nouveau` | identité minimale → **doublons en direct** (y compris anciens élèves : « réintégrer ? ») → scolarisation (date, niveau, origine) → récapitulatif. En temps normal la création vient du socle 3 ; cet écran sert aux cas hors campagne | secrétariat |
| **Propositions** | `/web/eleves/propositions` | file filtrable (source, champ, ancienneté) ; par élève, différences avant / après ; accepter tout ou partie, choisir la nature, refuser avec motif ; traitement par lots pour les imports | secrétariat |
| **Qualité des données** | `/web/eleves/qualite` | tuiles : sans INE, INE non confirmé, INE contestés, doublons probables, champs obligatoires manquants **pour une échéance** (export BEE, GAR, rentrée), photos manquantes ; chaque tuile ouvre la liste filtrée | secrétariat, direction |
| **Doublons et fusion** | `/web/eleves/doublons` | comparaison côte à côte ; choix par champ ; aperçu des données rattachées par domaine ; conflits à résoudre ; confirmation ★ ; annulation dans le délai | direction |
| **Trombinoscope** | `/web/classes/{id}/trombinoscope` | par classe ou groupe ; respecte la confidentialité ; impression / PDF (export journalisé) ; prénom d'usage | personnel |
| **Photos en lot** | `/web/eleves/photos` | dépôt d'un lot, rapprochement par nom de fichier ou numéro, validation | secrétariat |
| **Paramétrage élève** | `/web/direction/parametrage/eleves` | profil d'établissement ; champs (visible, obligatoire et quand, saisie famille, source qui fait foi) ; numérotation ; régimes ; motifs ; informations complémentaires ; photos ; prénom d'usage ; règles d'application automatique | administrateur |
| **Portail famille : mon enfant** | `/web/famille/enfants/{id}/identite` | identité telle que l'école la connaît (G0–G3) ; « signaler une erreur » avec pièce ; suivi des demandes et de leur décision | responsables |
| **Portail élève : mon profil** | `/web/moi` | identité, coordonnées propres (modifiables selon paramétrage), demande de prénom d'usage si ouverte | élève |
| **Mobile** (appli unique, D-003) | — | fiche allégée : photo, nom, classe, pastilles autorisées, contacts d'urgence (socle 4), actions du jour (appel, signaler) ; recherche ; jamais l'état civil complet ni les identifiants | personnel ; famille et élève : identité en lecture |

Critères d'ergonomie [R] : clavier complet, lecteur d'écran, 375 px, contraste ; recherche < 100 ms côté serveur sur 5 000 élèves (index trigramme, à mesurer) ; pas plus de deux clics de n'importe quelle liste à la fiche ; tout masquage expliqué.

---

## 15. Plan de migration depuis l'existant

Règles : migrations **additives** d'abord ; reprise idempotente ; aucune migration ne doit échouer sur des données imprévues (elle les range en « à traiter » et le signale) ; répétition sur base de labo et jeux anonymisés, **jamais** sur la base de production (interdite) ; ancienne interface `ui.rs` figée ; anciennes routes conservées puis dépréciées.

| Étape | Contenu | Reprise / compatibilité | Vérification |
|---|---|---|---|
| M0 Correctifs immédiats | minimisation des homonymes ; journal non ignoré ; transaction unique pour `create_eleve` et `update_identite` ; événements sans valeurs d'identité | aucun changement de schéma | tests de minimisation et de rollback |
| M1 Clés et contraintes | `UNIQUE (etablissement_id, id)` sur `ent_eleve` ; clés étrangères composées sur les 12 tables sans clé, ajoutées `NOT VALID` puis `VALIDATE` séparément ; rapport des orphelins (labo) | orphelins listés, pas supprimés | requête d'orphelins à zéro en labo |
| M2 Identité versionnée | `ent_eleve_identite` ; version 1 par élève : `nom_naissance = nom`, `prenoms = [prenom]`, `sexe` normalisé (valeurs inconnues → NULL + rapport), `date_naissance`, nature `reprise`, `effet_du` = date d'entrée ou de création ; `identite_version_id` ; triggers de projection | `nom`/`prenom` gardent leur sens ; le nom repris est présumé nom de naissance (signalé à l'écran « à vérifier ») | égalité projection ↔ version pour tous les élèves |
| M3 Identifiants | `ent_systeme_identifiant`, `ent_eleve_identifiant` ; INE existants → `a_verifier`, provenance `reprise_logiciel` ; doublons d'INE dans un établissement → `conteste` (l'index unique est créé ensuite) ; badges → système `BADGE` | `ine`, `numero_badge` restent en projection | aucun INE perdu ; doublons listés |
| M4 Numéro d'élève | `numero` attribué aux élèves existants (ordre de création) selon le format par défaut | — | unicité |
| M5 Scolarisations et inscriptions | scolarisations depuis `date_entree`/`date_sortie`/`motif_sortie` (et périodes) ; `ent_eleve_scolarite_annee` → `ent_eleve_inscription` (`annee_scolaire` texte → `annee_id`, années créées si besoin comme le fait déjà le trigger `classe_id` [F]) ; statut dérivé calculé et comparé au statut texte (écarts rapportés) | vue de compatibilité `ent_eleve_scolarite_annee` le temps de migrer les lecteurs | écarts statut dérivé / statut texte à zéro ou expliqués |
| M6 Options | `ent_eleve_option` et `_option_annee` → affectations à des groupes du référentiel | vues de compatibilité | comparaison avant / après |
| M7 Écritures | tous les chemins d'écriture (`eleve_repo::upsert`, `create_eleve`, `update_identite`, seed et labo, imports) passent par les commandes ; puis trigger qui **refuse** l'écriture directe des colonnes d'identité d'`ent_eleve` | — | grep des `UPDATE ent_eleve` ; test du refus |
| M8 Droits | nouvelles actions, modèles par défaut équivalents ; `NiveauIdentite` dérivé des actions | oracle profil × action | tests de non-régression |
| M9 RLS | tables élève ajoutées à la RLS (permissif), dépôts en `TenantTx`, puis strict | — | tests sous rôle non propriétaire |
| M10 Écrans | nouveaux écrans ; `/web/eleves/{id}` enrichi ; anciens écrans `ui.rs` laissés en l'état | — | tests de rendu, navigateur, accessibilité |
| M11 Retrait des projections | après bascule de tous les lecteurs (plan Q14 pour `classe`) : retrait de `regime`, `mef`, `site`, `date_entree`, `date_sortie`, `motif_sortie`, `redoublant`, `status`, `ine`, `numero_badge`, `photo_key` d'`ent_eleve`, et des kinds `classe`/`mef` de `ent_eleve_periode` | migration dédiée, après validation | grep des lectures à zéro |

---

## 16. Découpage en lots de code (équipe Sonnet)

Chaque lot livre : migration(s) additives, dépôt (`ent-db`), service avec transaction unique, routes (`ent-api`) et types (`ent-types`), tests Postgres (dont isolation entre établissements et matrice de droits), écrans quand le lot en a, documentation (`SOCLE2_DOMAINES.md`/ce document, `PROJECT_STATUS.md`). Revue `review-sonnet` sur chaque lot ; revue sécurité (skill `security-review`) sur E0, E1, E6, E7. Travaux parallèles en sessions cloud séparées ou worktrees vérifiés (leçon F-007), quatre compilations au plus en même temps (F-006). Effort : **S** ≤ 3 j, **M** 1–2 sem., **L** > 2 sem. [H].

| Lot | Contenu | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|
| **E0 Correctifs** | M0 + M1 (clés composées, orphelins) | — | homonymes minimisés ; création et identité en une transaction ; événements sans état civil ; clés validées en labo | S |
| **E1 Droits et minimisation** | actions §12, groupes de champs, raisons de masquage, `NiveauIdentite` dérivé ; oracle | E0 | matrice profils × actions identique ; tests par groupe ; rôle « Accueil » de démonstration | M |
| **E2 Identité versionnée** | M2, M7 (partie identité), commandes ELV-03/04/05/06, historique, projection protégée, pièces | E0, E1 | reprise idempotente ; correction ≠ changement testés ; refus d'écriture directe ; aucun état civil dans les événements | M–L |
| **E3 Identifiants et numéro** | M3, M4, ELV-07, validation par famille d'INE **après vérification du format** (sinon garder la règle large) | E2 | collisions → `conteste` ; unicité par établissement ; aucune vérification entre établissements (test) | M |
| **E4 Scolarité** | M5, M6, ELV-08 à ELV-14, statut dérivé, réintégration, capacité, majorité, tâche nocturne ; commandes appelées par le socle 3 | E2 | non-chevauchement ; statut dérivé = statut attendu sur données de labo ; événements réservés enfin émis | L |
| **E5 Propositions et paramétrage** | `ent_eleve_proposition`, `ent_param_eleve_champ`, profils d'établissement, ELV-18/19/26, règles automatiques ; sert de base au moteur d'import IOP-L1 | E2, E3 | aucune écriture de source externe sans proposition ; champ verrouillé respecté ; famille ne peut proposer que l'autorisé | M |
| **E6 Recherche, doublons, fusion** | index trigramme et `unaccent`, ELV-20/22/23/24, registre `TABLES_ELEVE` + test de garde, aperçu et conflits | E2, E4 | recherche < 100 ms sur 5 000 élèves (labo) ; fusion puis défusion à l'identique ; test de garde du registre | L |
| **E7 Confidentialité, photos, coordonnées, RLS** | ELV-15/16/17, scellement des versions sensibles et motifs, journal de consultation, M9 | E1, E2 | élève protégé absent de l'annuaire pour un tiers ; consultation tracée ; RLS stricte testée | M |
| **E8 Pastilles et synthèse** | catalogue d'indicateurs, consommateur D0, contrat des tuiles par domaine | E1, outbox | trois pastilles réelles (interne, PAI, sans INE) alimentées par événements, filtrées par droits | M |
| **E9 Écrans** | en sous-lots : E9a liste + recherche ; E9b fiche hub + identité + historique ; E9c scolarité ; E9d propositions + qualité ; E9e doublons + paramétrage + photos ; E9f portails famille / élève | lots back correspondants | tests de rendu et contre le vrai backend ; clavier, 375 px, axe-core | L (total) |
| **E10 Mobile et types** | DTO `ent-types` de la fiche allégée, route dédiée, minimisation | E1, E8 | contrat JSON testé ; aucune donnée G2/G3 | S–M |
| **E11 Exports de l'élève** | liste nominative (ELV-25), extrait RGPD de l'élève (PLT-08), export de réversibilité élève (SDET) | E1, E2, E4 | colonnes limitées aux droits ; journalisé ; URL courte | M |

Ordre recommandé : E0 → E1 → E2 → (E3, E4 en parallèle) → (E5, E6, E7 en parallèle) → E8 → E9 au fil de l'eau → E10, E11. Les lots IOP-L0 (nomenclatures BCN, COG) et IOP-L1 (moteur d'import) s'appuient sur E3 et E5 ; le socle 3 peut commencer dès que E4 expose ses commandes.

---

### 16.1 État des lots (2026-11-07, branches `ent-socle-eleve-e0-e1`, `ent-socle-eleve-e2`, `ent-socle-eleve-e3-e4` et `ent-socle-eleve-e5`)

| Lot | État | Détail |
|---|---|---|
| **E0** | implémenté, testé en local | `create_eleve` et `update_identite` : fait + journal + événement dans une `TenantTx` (journal en erreur = tout est défait, test avec déclencheur de panne) ; événements et journal sans valeur d'identité (noms de champs) ; homonymes : date de naissance masquée aux profils sans état civil et non prise en compte dans leur score ; migration `240_eleve_cles_composees.sql` : `UNIQUE (etablissement_id, id)` sur `ent_eleve`, clés composées `NOT VALID` puis validées si aucun orphelin, sur 12 tables (§2.2) **et** `ent_inscription_dossier`, `ent_import_ligne`, `ent_travail_remise`, `ent_travail_eleve` (lot 5) ; orphelins listés dans `ent_eleve_orphelin` (jamais supprimés), `ent_eleve_valider_cles()` relance la validation. Tests : `ent-db/src/tests_eleve_socle.rs`. |
| **E1** | implémenté, testé en local | 16 actions `eleves.*` au catalogue (portées, inclusions §12.2, modèles par défaut) ; groupes G0–G6 en un point (`domain-eleve::champs`, `ent-db::eleve_champs`) ; raisons de masquage (`droit_manquant`, `hors_portee`, `reserve_au_personnel`) dans `EleveDto.masques` et `FicheEleveDto.masques` ; `NiveauIdentite` dérivé des droits (ancienne règle par profil conservée comme oracle de test) ; portée « désignés » étendue aux sites et niveaux (Q-E9) ; gabarit de rôle « Accueil » (web). |
| **E2** | implémenté, testé en local (résultats chiffrés : `PROJECT_STATUS.md`) | Migration `280_eleve_identite_versionnee.sql` (M2 + M7 identité) : `ent_eleve_identite` (versions immuables : date d'effet, nature `correction` / `changement_etat_civil` / `usage` / `creation` / `reprise`, source, auteur, motif, pièce, sensible ; remplacement tracé par `remplacee_par`), projection **maintenue par la base** sur `ent_eleve` (`nom`, `prenom`, `sexe`, `date_naissance`, `nom_naissance`, `prenoms`, `identite_version_id`), **garde** qui refuse (SQLSTATE 42501) toute écriture directe de ces colonnes et toute modification ou suppression d'une version, version initiale écrite par trigger pour *tout* chemin d'insertion (création, import, inscription, labo), reprise idempotente `ent_eleve_identite_reprendre()` (nom présumé nom de naissance ; sexe hors M/F → NULL, valeur d'origine gardée dans `ent_eleve_identite_a_traiter`), `ent_eleve_identite_resynchroniser(jour)` (versions annoncées devenues effectives), RLS permissive sur la nouvelle table. Logique pure : `domain-eleve::identite` (natures, validation, sensibilité, filtrage de l'historique). Commandes `ent-db::identite` : ELV-03 `corriger_identite` (remplace une version, garde sa date d'effet, motif obligatoire), ELV-04 `enregistrer_changement_etat_civil` (version datée, motif **et pièce** obligatoires), ELV-05 `definir_usage` (nom/prénom d'usage ; le prénom d'usage est `sensible` ; le même jour, remplace la version du jour), ELV-06 `historique_identite_pour_acteur` (G5 `eleves.historique.lire` ; versions sensibles G6 `eleves.confidentiel.gerer`, absentes sans trace pour les autres ; lecture journalisée). Chaque commande = une `TenantTx` : verrou de l'élève, version, événement `eleve.identity_changed` (identifiant de version, nature, **noms** de champs, `sensible` ; jamais de valeur), journal (`visible_famille = false` pour une version sensible). Pièces : `piece_document_id` référence un document de **cet** élève (`ent_eleve_document`), lu par le mécanisme existant (`POST /documents/{id}/lien`, URL pré-signée). Routes : `POST /eleves/{id}/identite/{corrections,changements,usages}`, `GET /eleves/{id}/identite/historique` (DTO `ent-types::identite`), entrées de la matrice d'isolation. L'ancienne saisie `PUT /eleves/{id}/identite` est conservée : un changement de nom, prénom, sexe ou date de naissance y devient une **correction** versionnée (refusé si un nom ou prénom d'usage masquerait la valeur). |
| **E3** | implémenté, testé en local (résultats chiffrés : `PROJECT_STATUS.md`) | Migrations `281_eleve_identifiants.sql` (M3) et `282_eleve_numero.sql` (M4). **Identifiants** : référentiel `ent_systeme_identifiant` (INE, SIECLE_ELEVE, ONDE, AAF_JOINTURE, BADGE, ANCIEN_LOGICIEL), `ent_eleve_identifiant` (statut `declare` / `a_verifier` / `confirme` / `conteste` / `revoque`, provenance, pièce, dates de validité, lignes **immuables** hors statut, jamais supprimées), **unicité par établissement seulement** (index dont la clé commence par `etablissement_id` ; la fonction de collision ne lit jamais hors de l'établissement), une seule fonction SQL d'enregistrement `ent_eleve_identifiant_enregistrer` (sérialisée par valeur : collision → `conteste` avec l'élève concerné, remplacement de l'identifiant actif, un identifiant `confirme` jamais remplacé par une valeur moins sûre), **pont des écritures héritées** (tout chemin qui écrit encore `ent_eleve.ine` ou `numero_badge` — création, import, inscription, labo, ancienne saisie — devient un identifiant ; les colonnes restent des projections, rien n'est perdu même en cas de collision), reprise idempotente `ent_eleve_identifiant_reprendre()` (INE existants → `a_verifier`/`reprise_logiciel`, un même INE porté par plusieurs élèves → **tous** `conteste`, badges → `confirme`). Logique pure `domain-eleve::identifiant` (systèmes, statuts, provenances → statut initial §8.1, normalisation, transitions, motif obligatoire pour contester et révoquer). Commandes `ent-db::identifiants` : ELV-07 `ajouter_identifiant` (opérateur : provenances `saisie` / `document_famille` / `ancien_etablissement` seulement), `ajouter_identifiant_systeme` (socle 3 et imports : toutes provenances, sans rôle, jamais une route), `confirmer_identifiant`, `contester_identifiant`, `revoquer_identifiant`, ELV-41 `declarer_badge_perdu` (ancien révoqué aussitôt, nouveau actif, même transaction), lectures G3 `identifiants_pour_acteur` (journalisée) et `identifiants_contestes` (qualité des données, avec l'autre élève concerné). Chaque commande = une `TenantTx` : identifiant + événement `eleve.identifiant_change` (système, action, statuts, provenance, drapeau de collision — **jamais la valeur**) + journal (`visible_famille = false`). Nouvelle action `eleves.badge.gerer` (Admin, Direction, Administratif, CPE ; système BADGE seulement). **Numéro d'élève** : `ent_eleve.numero`, attribué par trigger à tout insert (sauf la branche « existe déjà » d'un `INSERT … ON CONFLICT`, pour ne pas brûler de numéro), compteur atomique par établissement et par année (`ent_eleve_numero_compteur`), format `ent_param_eleve_numero` (défaut `{AAAA}-{seq:5}`, contrôlé : compteur obligatoire, `{AAAA}` obligatoire si remise à zéro annuelle), jamais tronqué (le numéro s'allonge), **immuable**, unique par établissement, reprise dans l'ordre de création. Routes : `GET`/`POST /eleves/{id}/identifiants`, `POST …/identifiants/{iid}/{confirmer,contester,revoquer}`, `POST /eleves/{id}/badge/perdu`, `GET /eleves/identifiants/contestes` (DTO `ent-types::identifiants`), toutes dans la matrice d'isolation. |
| **E4** | implémenté, testé en local (résultats chiffrés : `PROJECT_STATUS.md`) | Migrations `283_eleve_scolarisation.sql` (M5 : scolarisations, statut dérivé, fuseau), `284_eleve_inscription.sql` (M5 : inscriptions), `285_eleve_capacite_majorite.sql` (capacité, majorité, mobilité, modalités, dates de régime), `286_eleve_options.sql` (M6 : options = groupes). **Scolarisations** `ent_eleve_scolarisation` : séjour continu `[entrée, sortie[` (même convention que l'existant : `date_sortie` = premier jour d'absence), origine et destination (UAI facultatif, libellé de secours), motif d'entrée et de sortie **codés**, exeat (pièce), **non-chevauchement garanti par la base** (`EXCLUDE USING gist`) et une seule scolarisation ouverte par élève, jamais supprimée. **Statut dérivé, jamais saisi** : `ent_eleve_statut_a(établissement, élève, jour)` (miroir exact de `domain-eleve::scolarite::statut_derive`, vérifié aux bornes), mis en cache dans `ent_eleve.statut` ; les colonnes héritées `status`, `date_entree`, `date_sortie`, `motif_sortie` deviennent des **projections** maintenues par la base, et tout ancien chemin qui les écrit (`sortir`, `change_status`, création, import, labo) est **converti** en changement de la scolarisation la plus récente (pont) ; reprise idempotente `ent_eleve_scolarisation_reprendre()` (écarts rapportés dans `ent_eleve_scolarite_ecart`, rien supprimé). **Fuseau de l'établissement (R-E15)** : « aujourd'hui » = `ent_jour_etablissement` (Europe/Paris par défaut ; table provisoire `ent_param_eleve_fuseau` en attendant STR-03). **Inscription annuelle** `ent_eleve_inscription` (une par élève et par année, `annee_id`, niveau, MEF, statut d'apprenant, état), reliée à la scolarisation qui couvre l'année, avec **pont bidirectionnel** avec `ent_eleve_scolarite_annee` (conservée, ses lecteurs et écrivains continuent de fonctionner ; lignes non reprises dans `ent_eleve_inscription_a_traiter`). **Options** : une option est une affectation à un **groupe** du référentiel ; pont à sens unique des anciennes options (`ent_eleve_option`, `_option_annee` retenues) vers `ent_affectation`, `ent_eleve_option_groupe` (comparaison avant / après), `ent_eleve_option_a_traiter`. **Capacité** `ent_eleve_capacite` (pièce obligatoire, une mesure active par nature, immuable, levée conservée). **Majorité** : `ent_eleve_majorite_le` (jamais sur une date imprécise ou contestée, R-E1), marqueurs `ent_eleve_majorite_evenement`. **Mobilité** (R-E13) et **modalités** (R-E14), sans chevauchement. **Dates de changement de régime** (R-E5) : `ent_param_eleve_regime` (libre par défaut, fin de période, fin de trimestre, liste) ; hors des dates autorisées, dérogation **motivée par un code**, tracée sans détail libre. Logique pure `domain-eleve::scolarite`. Commandes `ent-db::scolarite`, **appelables par le socle 3** (variantes `*_systeme`, sans rôle) : ELV-08 `ouvrir_scolarisation`, ELV-12 `reintegrer_eleve` (même fiche, motif « retour », refusée après un décès), ELV-11 `cloturer_scolarisation` (affectations fermées la veille, répercussions transverses existantes appliquées une fois le fait enregistré), ELV-34 `enregistrer_deces` (motif `deces`, `eleve.decede` **sans aucun détail**, journal non visible de la famille), ELV-09 `enregistrer_inscription_annuelle` / `changer_classe` / `changer_options`, ELV-10 `changer_regime_ou_site`, ELV-13 `declarer_capacite` / `lever_capacite`, ELV-35 `ouvrir_mobilite` / `fermer_mobilite`, ELV-36 `enregistrer_modalite` / `terminer_modalite`, lecture G1 `scolarite_pour_acteur`. **Événements réservés enfin émis** : `scolarite.opened`, `scolarite.closed`, plus `eleve.reintegre`, `eleve.capacite_changee`, `eleve.majeur`, `eleve.majorite_proche`, `eleve.decede`, `scolarite.mobilite_*`, `scolarite.modalite_changee` ; `eleve.status_changed`, `eleve.sortie`, `scolarite.classe_changed`, `eleve.regime_changed` désormais publiés dans la transaction du fait. Aucune charge utile ne contient d'identité (pas même la date de naissance : `eleve.majeur` ne porte que `date_presumee`). **Tâche quotidienne** `tache_nocturne_eleves` (idempotente, **planifiée** dans `ent-api` toutes les heures — « le jour » dépend du fuseau de chaque établissement ; `ENT_TACHES_ELEVES=0` la désactive) : identités annoncées devenues effectives (`resynchroniser_identites`, enfin planifiée), statuts changés avec la date (+ répercussions de sortie), `eleve.majeur` le jour des 18 ans, `eleve.majorite_proche` à J-30 (ELV-39), une seule fois par élève (marqueur) ; au-delà de 14 jours de retard (première mise en service, panne longue) le marqueur est posé sans rien publier. Routes (toutes dans la matrice d'isolation) : `GET /eleves/{id}/scolarite`, `POST /eleves/{id}/scolarisations[/reintegrer|/cloturer]`, `POST /eleves/{id}/deces`, `PUT /eleves/{id}/inscriptions/{annee_id}`, `POST /eleves/{id}/scolarite/{classe,options,regime,site}`, `POST /eleves/{id}/capacites[/{id}/lever]`, `POST /eleves/{id}/mobilites[/{id}/fermer]`, `POST /eleves/{id}/modalites[/{id}/terminer]` (DTO `ent-types::scolarite`). |

| **E5** | implémenté, testé en local (résultats chiffrés : `PROJECT_STATUS.md`) | Migrations `287_eleve_param_champ.sql`, `288_eleve_proposition.sql`, `289_eleve_proposition_garde.sql`. **Principe : aucune donnée venue de l'extérieur n'écrit directement dans la fiche** (import SIECLE, famille, élève, ancien établissement, personnel qui signale une erreur) : elle devient une **proposition** (valeur actuelle → valeur proposée, source, origine, pièce, statut), appliquée par une règle explicite ou par une personne. **Paramétrage** (`ent_param_eleve_champ`, ELV-26) : pour chacun des 13 champs du catalogue du logiciel (nom et prénoms de naissance, noms d'usage, sexe, date et lieu de naissance, INE, identifiant SIECLE, ONDE) : visible, obligatoire à quels moments (création, inscription, export BEE, export GAR), saisie par la famille (`non` / `proposer`), **qui fait foi** (établissement, SIECLE, ONDE, reprise ; `famille` prévu au schéma), modification locale (`libre` / `par_proposition` / `interdite`) et **verrou** (gel total). Sans ligne, valeur par défaut du catalogue (tout libre, rien d'ouvert à la famille) : un établissement existant ne change pas de comportement tant qu'il n'a rien réglé. Bornes du logiciel contrôlées dans `domain-eleve::proposition` : l'INE n'est jamais proposable par la famille, « interdite » exige une source externe qui fait foi, un champ sensible n'a jamais de règle automatique. **Profils d'établissement** (`ent_param_eleve_profil`) : `public` (SIECLE fait foi, état civil **interdit localement** par défaut, réglable par champ — Q-E7 —, règles automatiques d'import), `prive_sous_contrat`, `prive_sous_contrat_logiciel_agree`, `hors_contrat`, `premier_degre` (ONDE) ; un profil s'applique en bloc puis devient « personnalisé » au premier réglage manuel. **Propositions** (`ent_eleve_proposition` + `_champ`, ELV-18/19/40) : valeurs **scellées** (`enc:v1:`, `champs_chiffres`), proposition décidée **immuable** (déclencheur), jamais supprimée ; dépôt par la famille (responsable à lien actif ; élève majeur ou émancipé seulement), par le personnel (`eleves.annuaire` sur l'élève, ELV-40) ou par un traitement du serveur (`deposer_proposition_systeme`, **interface générique du futur moteur d'import** : interface identique pour IOP-L1 et le socle 3, sans route) ; valeurs identiques à la fiche écartées (import rejoué = rien), proposition plus récente du même déposant sur le même champ → l'ancienne devient `obsolete`. **Règles automatiques** (`ent_param_eleve_regle_auto`) : toujours explicites (champ × source), flux machine seulement (`import`, `educonnect`, `systeme`), jamais sur un champ sensible (prénoms, sexe, prénom d'usage), jamais si le champ est verrouillé, si l'origine n'est pas l'autorité qui fait foi ou s'il existe un conflit ; un lot mixte est **partagé** (la part automatique s'applique, le reste attend une personne, avec la raison) ; application dans un point de reprise (échec → la proposition reste en attente). **Décision** (ELV-19) : accepter tout ou partie, nature choisie (`correction` / `changement_etat_civil` ; `import` déduit des flux machine), refus avec motif codé visible du déposant, **conflit** si la fiche a changé depuis le dépôt (acceptation refusée, refus possible), champ verrouillé ou « interdite » : seule la proposition de l'autorité qui fait foi s'accepte (sinon « à corriger dans la source »). **Garde SQL (289)** : une version d'identité de source `import`, `famille` ou `eleve` ne s'insère que si elle référence une proposition décidée du même élève. **Verrous des commandes directes** : ELV-03/04/05, l'ancienne saisie `PUT …/identite` et ELV-07 (ajout d'identifiant) refusent un champ verrouillé ou non libre. Le chemin système d'identifiant (`ajouter_identifiant_systeme`) **refuse les provenances officielles** (AAF, BEE, ONDE, retour BEE, ÉduConnect) : elles passent par une proposition. Événements `eleve.proposition_deposee`, `eleve.proposition_decidee`, `parametrage.eleve_modifie` (identifiants et **noms** de champs, jamais une valeur). Routes (matrice d'isolation) : `POST|GET /eleves/{id}/propositions`, `GET /eleves/{id}/propositions/champs`, `GET /eleves/propositions[/{id}]`, `POST /eleves/propositions/{id}/decision|retrait`, `GET /eleves/parametrage`, `PUT /eleves/parametrage/champs/{champ}`, `POST /eleves/parametrage/profil`, `PUT /eleves/parametrage/regles-auto` (DTO `ent-types::propositions`). |
| **E6** | implémenté, testé en local (résultats : `PROJECT_STATUS.md`) | Migrations `290_eleve_recherche.sql` à `294_eleve_fusion_securite.sql`. **Recherche (ELV-20)** : extensions `pg_trgm` et `unaccent`, `ent_cle_recherche` (UNE définition SQL pour la colonne et la requête), **deux clés** maintenues par déclencheur : `cle_recherche` (désignation affichée, G0) et `cle_recherche_civil` (nom de naissance et prénoms, G2) — l'état civil n'est interrogeable que par qui voit G2 partout (pas d'oracle sur un prénom d'état civil derrière un prénom d'usage) ; INE et badge en valeur complète seulement (G3, jamais de préfixe), début de numéro d'élève avec G1. Tous les droits sont des **conditions SQL** (portée de l'annuaire, protégés) : tri et limite viennent après le filtrage. Fautes : similarité de mots trigramme (seuil 0,5) ; classe cherchable (« dubois 6a »). **Élèves protégés** (`ent_eleve_confidentialite`, table minimale du §7.6 que E7 complète) : exclus de la recherche, des doublons et de la liste des fusions sauf pour qui détient G6 ou a un lien direct (portée restreinte). **Doublons (ELV-22)** : score pur `domain-eleve::doublon` (INE identique, identité exacte/proche, nom et prénom inversés, jour et mois permutés ; jumeaux et homonymes nés un autre jour jamais proposés ; INE différents = score −15), paires candidates par INE, désignation, date de naissance + similarité, index trigramme ; cache `ent_eleve_doublon`, tâche quotidienne (un passage par jour et par établissement) et relance à la demande. **ELV-33** (`ent_eleve_non_doublon`, avec les versions d'identité du moment : la paire revient si une identité change). **Fusion (ELV-23)** : registre `TABLES_ELEVE` (98 tables, `Deplacer` ou `Reste` avec raison) + **test de garde** qui lit le schéma (tables, clés primaires) ; aperçu et fusion exécutent le même code (l'aperçu abandonne sa transaction) ; conflits d'unicité/exclusion détectés par essai sous point de reprise : la ligne du conservé l'emporte, chaque conflit doit être acquitté ; registre des lignes déplacées (table, clé, empreinte, jamais le contenu) ; gardes de scolarisation/identifiant/capacité assouplies **pour le seul repointage de `eleve_id`** (drapeau local `ent.fusion_eleve`) ; projections d'`ent_eleve` recalculées ; événement `eleve.fusionne` et journal sans valeur d'identité. **Défusion (ELV-24)** : 30 jours, refusée si une ligne déplacée a changé (liste montrée) ou si le conservé a été absorbé depuis ; **fusion puis défusion à l'identique** vérifié sur toutes les tables du registre (hors journaux d'audit en ajout seul). Redirection : `get_eleve_for_actor` suit `fusionne_dans`. Routes (matrice d'isolation) : `GET|POST /eleves/doublons[/detecter|/non-doublon]`, `GET|POST /eleves/fusions[/apercu|/{id}/annuler]` (DTO `ent-types::fusion`). |
**Écarts, limites et points à valider par Florian (lot E6).**
- **Conflit : seul « le conservé l'emporte »** est offert ligne par ligne (échanger deux lignes sous contrainte d'unicité n'est pas faisable proprement) ; pour garder la ligne de l'absorbé, fusionner dans l'autre sens (l'aperçu est possible dans les deux sens). Les lignes en conflit restent sur l'absorbé.
- **Restent sur l'absorbé** (stratégie `Reste`) : journal d'audit, événements, files d'envoi, bulletins émis (figés), versions d'identité, propositions, diagnostics de reprise. La fusion **ne modifie jamais l'identité du conservé** (corriger ensuite par ELV-03).
- **« Absorbé en lecture seule » n'est pas imposé par la base** : filtré dans la recherche, les doublons et la redirection de `GET /eleves/{id}` ; les autres lectures par identifiant ne redirigent pas et aucune garde n'interdit d'écrire sur un absorbé (revue de sécurité, point 4) → lot suivant (garde générique + filtre commun). Le compte d'élève resté sur l'absorbé (deux comptes) est signalé dans l'aperçu, à fermer à la main.
- **Confidentialité** : la mesure de l'absorbé suit les lignes déplacées (donc le conservé devient protégé) ; sa fiche absorbée n'est plus protégée. Choix de modèle à valider. Accès direct par identifiant à un élève protégé : lot E7.
- **Revue de sécurité (`review-sonnet`)** : corrigés — projections de la fiche conservée écrasées à la défusion (restitution seulement si inchangée, sinon recalcul), lignes non verrouillées entre vérification et repointage (`FOR UPDATE`), noms de tables de domaines protégés (santé, psychologie) exposés dans les conflits (référence opaque), drapeau de fusion laissé armé (désarmé après les déplacements), statut d'une paire déclarée distincte écrasé à la défusion. **Non corrigés, assumés** : interblocage possible détection nocturne / fusion simultanée (réessayer sur 40P01), deux connexions par requête (aperçus en rafale), lectures hors `TenantTx` (à passer en transaction avant le RLS strict), raisons de doublon et avertissement d'INE non filtrés par G2/G3 pour des rôles personnalisés, compteurs de doublons incluant les paires protégées.
- **Non fait** : alias de recherche (ELV-30, R-E2), choix d'identité champ par champ, écrans (E9e), notification, brique « doublons à la création » (ELV-01 continue d'utiliser la recherche d'homonymes existante).
**Écarts, limites et points à valider par Florian (lot E5).**
- **Sens de `modification_locale`** (à valider, c'est le cœur du verrou) : `libre` = les commandes directes du secrétariat restent permises ; `par_proposition` = aucune commande directe, toute modification locale passe par la file (dépôt puis décision, avec trace) ; `interdite` = ni commande directe ni acceptation d'une demande locale (famille, personnel) — le secrétariat ne peut que **refuser** avec le motif codé `a_corriger_dans_la_source` ; seule la proposition de l'autorité qui fait foi (import de la bonne origine) s'accepte. Le profil `public` pose `interdite` sur l'état civil et l'INE (Q-E7, recommandation « interdire par défaut, réglable par champ »).
- **`verrouille`** (gel total) est un ajout à la spécification du §7.8 (le message demandait « champ verrouillé ou non ») : rien ne s'applique tant qu'un champ est verrouillé (commande directe, acceptation, règle automatique) ; les propositions peuvent toujours être déposées et attendent. **`source_qui_fait_foi = famille`** est au schéma mais **aucun champ du catalogue ne l'accepte encore** (coordonnées et photo : lot E7).
- **Cibles non appliquées** : le schéma accepte les sept cibles du §7.8, mais le service ne sait appliquer que `identite` et `identifiant` ; `scolarisation`, `inscription`, `coordonnee`, `photo` et `nouvel_eleve` (élève inconnu proposé par un import, `eleve_id` NULL) sont **refusés explicitement au dépôt**. La création d'un élève depuis un import relève d'IOP-L1 / du socle 3. Le **rapprochement** des imports (§8.3 : INE confirmé, identifiant source, score d'homonymes) **n'est pas fait** : le dépôt suppose l'élève déjà identifié.
- **`saisie_famille = modifier`** (modification directe par la famille) est au schéma mais **inatteignable** : le maximum du catalogue est `proposer` pour tous les champs d'identité, et l'INE est à `non` (bornes du logiciel). Même la saisie « directe » devrait de toute façon passer par une proposition appliquée par règle (principe du lot).
- **Prénoms et sexe sont sensibles** (jamais appliqués automatiquement) : un import SIECLE qui change les prénoms attend donc toujours une personne. À valider (le prénom d'état civil venant de SIECLE est l'autorité même, mais une version qui le change est « intime » au sens de E2).
- **Nom d'usage et état civil dans une seule version** : une version d'identité étant un instantané, une proposition qui change l'état civil **et** un nom d'usage écrit **une** version (nature choisie) ; un nom d'usage seul est une version `usage`. **Limite E2 préexistante, non corrigée** : une version datée dans le futur n'est pas prise en compte quand on en insère une plus ancienne (la version future recopie son ancienne base) ; ELV-04 daté de demain suivi d'un nom d'usage aujourd'hui perd le nom d'usage demain.
- **Pas d'expiration par défaut** : `expire_le` n'est posé que par un dépôt système ; `expirer_propositions_echues()` existe mais **n'est pas planifiée** ; aucune durée de conservation des propositions décidées n'est codée (Q-E15, DPO).
- **Familles** : un dépôt exige un lien actif (`ent_eleve_lien_actif`) ; il ne tient pas compte des interdictions A1 (« ne pas remettre à ») ni de « ne pas solliciter » : ces règles protègent les envois, pas la saisie du responsable légal. Plafond de 20 propositions en attente par compte (anti-inondation). Un responsable ne voit **que ses propres** propositions (jamais celles de l'autre responsable : un prénom d'usage demandé par l'un ne se voit pas de l'autre), jamais la valeur de la fiche. **Pas d'écran** (E9d/E9f) ; aucune notification au déposant n'est branchée (l'événement existe).
- **Conflit** : détecté par comparaison de valeur (valeur de la fiche ≠ valeur vue au dépôt) ; un aller-retour A → B → A n'est pas détecté. Une règle automatique ne peut pas rencontrer de conflit (valeurs lues dans la transaction du dépôt, élève verrouillé).
- **Chemin système d'identifiants** : les provenances officielles y sont refusées, mais la base ne garde que les **versions d'identité** (289) ; les identifiants restent protégés par le service seulement (la fonction SQL d'enregistrement des identifiants reste appelable par les ponts hérités).
- **Contrat IA** : toute capacité (`eleve.proposition.deposer`, `.decider`, `parametrage.modifier`) appellera les **mêmes services** que les routes ; décision et paramétrage : **jamais ouverts à l'IA** (N3 : changement de la source qui fait foi), dépôt d'une proposition : N2 corrigeable, valeurs jamais envoyées au modèle.

**Écarts et limites du lot E2 (à valider par Florian).**
- **Scellement des valeurs sensibles non fait** : les versions sensibles (prénom d'usage, mention de sexe) sont *masquées* par les droits (G6) mais leurs valeurs restent en clair dans `ent_eleve_identite` (la colonne `valeurs_scellees` du §7.2 arrive avec E7, qui apporte aussi la rotation de clé `enc:v2:`). Aucune de ces valeurs n'est dans un événement ni dans le journal.
- **Pas de paramétrage** (E5) : la pièce d'une *correction* est facultative ; l'activation du prénom d'usage et la procédure (accord des parents) ne sont pas encore réglables par l'établissement ; le verrou « source externe qui fait foi » (ELV-03) n'existe pas encore.
- **COG** : `naissance_commune_cog` et `naissance_pays_cog` sont de simples codes (≤ 10 caractères alphanumériques) ; la nomenclature n'est pas vérifiée (IOP-L0, [NV]).
- **Nationalité, `cle_recherche`, `numero`** du §7.1/7.2 : hors lot (E3, E6).
- Le trigger de reprise rattache *tout* élève inséré par un chemin ancien (import, inscription, labo) à une version `creation` ; `source_ref` n'est pas encore renseigné par ces chemins.
- La tâche quotidienne `resynchroniser_identites()` existe mais n'est **pas encore planifiée** (même cas que les autres tâches nocturnes, lot E4).
- **Revue sécurité (skill `security-review`, relecture indépendante)** : corrigés — sensibilité *collante* (une version bâtie sur une version sensible l'est aussi, sinon déduction des valeurs masquées), plus aucune trace de l'existence d'une version masquée dans `remplacee_par`, pièces limitées aux tiroirs `administratif`/`famille` hors santé, RLS sur `ent_eleve_identite_a_traiter`, `PUT …/identite` exige aussi `eleves.identite.modifier`, remplacement vérifié (`rows_affected`). **Risques résiduels assumés** : la garde SQL protège contre l'erreur de code, pas contre un compte applicatif propriétaire des tables ni une injection SQL (rôle d'exécution distinct à prévoir) ; un `INSERT` direct dans `ent_eleve_identite` n'est pas gardé ; lecture de l'historique hors `TenantTx` ; `ent.identite_source` et l'auteur non posés par les chemins d'import ; caractères Unicode bidirectionnels non filtrés dans les noms ; reprise non verrouillée entre deux instances démarrant ensemble.
- Le prénom *usuel* affiché n'est pas modifiable seul (`rang_prenom_usuel` suit les prénoms) ; nom d'usage / prénom d'usage ne sont pas « autorisés par établissement » (E5).

**Écarts, limites et points à valider par Florian (lots E3 et E4).**
- **Format de l'INE : règle large conservée, volontairement.** Une recherche ciblée (extraits de sources officielles, pages non ouvertes depuis la session) donne : le répertoire national est créé par l'arrêté du 16 février 2012 (RNIE), un manuel de référence Onde décrirait 11 caractères (année de création sur 2 caractères, numéro d'ordre sur 7, clé de contrôle sur 2), tandis qu'un ancien document SIECLE (agriculture) décrit une autre structure ; **l'algorithme de la clé n'est pas sourcé** et les familles (second degré, premier degré, apprentissage, agriculture) ne sont pas confirmées. Conformément à la consigne (D-019), **aucune validation par famille n'est codée** : la règle reste 9 à 16 caractères alphanumériques, isolée dans `domain-eleve::identifiant` pour qu'une validation par famille s'y branche après lecture d'un texte officiel. Sources à ouvrir : manuel de référence Onde (Eduscol), délibération CNIL 2023-03 du 8 juin 2023, arrêtés du 16 février 2012 et du 18 octobre 2022.
- **Convention `date_sortie`** : premier jour où l'élève n'est plus dans l'établissement (celle de l'existant : `eleve_repo`, effectifs, absences), et non « dernier jour de présence » comme le laissait lire le §7.4 (`'[]'`). **À valider** ; un changement de convention toucherait tous les lecteurs hérités.
- **Colonnes héritées d'`ent_eleve`** (`ine`, `numero_badge`, `status`, `date_entree`, `date_sortie`, `motif_sortie`) : conservées comme projections (retrait = M11, après bascule des lecteurs) ; les écritures héritées sont **converties** en commandes (ponts SQL) plutôt que refusées (contrairement à l'identité de l'E2) afin de ne casser aucun chemin existant (`sortir`, `change_status`, imports, inscriptions, labo). Par ces chemins il n'y a ni événement `scolarite.*` ni historique : la scolarisation la plus récente est modifiée sur place. Les ponts (`ent_eleve_identifiant_pont`, `ent_eleve_scolarite_pont`, options, inscriptions) sont explicites et à retirer avec M11.
- **Statut texte hérité** : projeté en `actif` (scolarisé) ou `sorti` ; pour un élève « à venir » il reste tel quel (jamais `preinscrit` imposé, des lecteurs hérités filtrent sur `actif`). `archive` n'est **pas dérivé** : aucune durée de conservation codée (Q-E15, DPO).
- **Options (M6)** : l'ancienne option a un code et un libellé mais pas de groupe ; la reprise crée le groupe de l'année (`code` de l'option) s'il manque. Les affectations issues d'options héritées ou des commandes portent la source `saisie` (la contrainte de source d'`ent_affectation` est rejouée par la migration 110 à chaque démarrage : aucune valeur n'y est ajoutée) ; leur origine se lit dans `ent_eleve_option_groupe`. Les anciennes tables ne reçoivent **pas** les changements d'options faits par la nouvelle commande (pont à sens unique) : tout lecteur qui passe aux affectations voit tout, les lecteurs d'`ent_eleve_option` ne voient que l'ancien.
- **Inscriptions (M5)** : `ent_eleve_scolarite_annee` n'est ni supprimée ni transformée en vue (pont bidirectionnel à la place) ; niveau et MEF repris par correspondance exacte de code, sinon `NULL` et ligne rangée « à traiter ». Redoublement et décision de passage : colonnes présentes, **non alimentées** (domaine évaluations, socle 23).
- **Pas de paramétrage encore (lot E5)** : format du numéro, dates de changement de régime, fuseau de l'établissement sont des tables lues par le code mais **sans commande ni écran** de réglage (définis en base) ; motifs de sortie et d'entrée locaux (`ent_param_motif_sortie`) non faits ; annuaire des établissements (R-E23, socle 9) : UAI saisi tel quel (lettres et chiffres, 20 au plus), sans vérification de format ni d'existence ; R-E22 (ancien élève anonymisé) sans objet tant qu'aucune anonymisation n'existe.
- **Majorité** : jour des 18 ans calculé sur la date de naissance **retenue**, seulement pour une précision « jour » et une date non contestée (R-E1) ; une date « présumée » au jour est calculée et le drapeau `date_presumee` accompagne l'événement ; 29 février → 28 février des années non bissextiles **[H : à valider juridiquement]** ; les effets sur les droits des parents dépendent de Q10 (non tranchée) : le socle 2 ne fournit que le fait (`eleve.majeur`). Un élève majeur depuis plus de 14 jours à la première exécution reçoit un marqueur silencieux, aucun événement. Si la date de naissance est corrigée après l'événement, le marqueur n'est pas rejoué.
- **Décès (ELV-34, R-E12)** : le socle 2 clôt la scolarisation et publie `eleve.decede` sans détail ; **l'arrêt effectif des envois, relances, appels, factures** relève des consommateurs (socles 8, 3, 4, 12, 24, 25, 29) qui n'existent pas encore pour la plupart ; la signalisation de la fratrie à la direction n'est pas faite (socle 4). Les répercussions *existantes* de la sortie (cantine, internat, accès, documents à signer) sont appliquées une fois la sortie effective.
- **Mobilité (R-E13)** : la tâche « retour prévu dépassé sans nouvelle » n'est pas faite (aucun consommateur de tâches avant le socle 29) ; la place réservée N+1 et l'appel neutralisé relèvent des socles 3 et 12. **Modalités (R-E14)** : codes du logiciel, **intitulés officiels [NV]**, libellés paramétrables non faits.
- **Non faits dans ces lots** (hors périmètre E3/E4 du §16) : ELV-35 retour automatique, ELV-37 registre des élèves et ELV-38 attestation passée (E11), ELV-44/46 bris de glace et conflit d'intérêts (E1/E7), ELV-45 arrivées et départs récents d'une classe, ELV-48 annuaire des établissements, pastilles `nouvel_arrivant`, `en_mobilite`… (E8), écrans (E9), `scolarite.mef_changed` (kind MEF des périodes, projection appelée à disparaître).
- **Tâche quotidienne** : planifiée dans le binaire `ent-api` (un processus par instance, idempotente et protégée par des marqueurs uniques) ; une planification hors processus (cron, tâche dédiée) reste possible sans changement de code (`tache_nocturne_eleves` est publique).
- **Droits** : `eleves.sortie` (Admin, Direction, Administratif) couvre aussi le décès — le tableau §11 réserve ELV-34 à la direction : **à valider** (retirer l'action au secrétariat, ou ajouter une action propre). La lecture G1 d'`GET /eleves/{id}/scolarite` suit `eleves.lire` : Admin, Direction, Administratif (écart n° 1 du §12.2, inchangé) ; les lectures de la famille et de l'élève ne sont pas ouvertes ici (E9f).

**Contrat IA (E3–E4)** — `docs/socles/IA_COEUR.md` §5 définit le contrat ; le registre de capacités (`ent-ia`) n'existe pas encore dans le code, ce contrat est donc **déclaré ici** et deviendra des entrées du registre. Toute capacité appellera le **même service** que la route (même `autoriser_eleve`), jamais une requête dédiée.

| Capacité | Genre | Service | Droit | Ouverte à l'IA | Sensibilité / exclusions |
|---|---|---|---|---|---|
| `eleve.scolarite.lire` | lecture | `scolarite_pour_acteur` | `eleves.lire` | oui | C1 ; vue sans motif de sortie libre ni identifiants d'établissement d'origine détaillés |
| `eleve.statut.lire` | lecture | `statut_eleve_a` | `eleves.lire` | oui | C1 |
| `eleve.identifiants.lire` | lecture | `identifiants_pour_acteur` | `eleves.identifiants.lire` | **statuts seulement** : **jamais les valeurs** (INE, badge) envoyées au modèle | C2 ; valeur exclue |
| `eleve.identifiants.contestes.lister` | lecture | `identifiants_contestes` | `eleves.identifiants.lire` | oui, sans valeur | C2 |
| ELV-07 ajouter, confirmer, contester, révoquer un identifiant | commande | `*_identifiant` | `eleves.identifiants.gerer` | **non** (simulation absente : exigée par le contrat) | niveau N2 prévu ; corrigeable (révocation) |
| ELV-41 badge perdu | commande | `declarer_badge_perdu` | `eleves.badge.gerer` | **non** pour l'instant | N2 ; l'ancien badge est révoqué aussitôt |
| ELV-08, ELV-09, ELV-10, ELV-35, ELV-36 | commandes | `ouvrir_scolarisation`, `enregistrer_inscription_annuelle`, `changer_*`, `*_mobilite`, `*_modalite` | `eleves.scolarite.gerer` | **non** tant que la simulation (aperçu avant / après) n'est pas écrite | N2 ; corrigeables |
| ELV-11, ELV-12 | commandes | `cloturer_scolarisation`, `reintegrer_eleve` | `eleves.sortie`, `eleves.scolarite.gerer` | **non** | N3 (sortie) ; confirmation humaine imposée |
| ELV-13 capacité, ELV-34 décès | commandes | `declarer_capacite`, `lever_capacite`, `enregistrer_deces` | `eleves.identite.modifier`, `eleves.sortie` | **jamais** | N4 ; exclusions explicites : motif et pièce d'une mesure de protection, décès |

Exclusions explicites (même avec droit) : valeurs d'INE et de badge, motifs libres de sortie, pièces, nature et motif des capacités, modalités de scolarisation (« sans détail »), décès. Jeu d'évaluation (§5.3 point 4) : à écrire avec le registre.

**Écarts assumés par rapport au §12.2 (lots E0–E1, à valider par Florian).**
1. `eleves.lire` (G1 : régime, MEF, dates d'entrée et de sortie, motif de sortie, redoublant) est donné par défaut à Admin, Direction, Administratif **seulement**, et non à CPE, Compta, Infirmerie, Psy, Prof comme dans le tableau : ces profils ne voyaient pas ces champs (niveaux « santé » et « réduit »), et la consigne était de ne rien élargir. L'accès à la fiche (`GET /eleves/{id}`) passe par `eleves.annuaire`, donné à tout le personnel. Ouvrir G1 à un rôle = cocher `eleves.lire`.
2. Le badge de cantine est rangé en G3 (identifiants), avec l'INE.
3. Le filtre « confidentialité renforcée » de l'annuaire (Q-E3) n'existe pas encore : il arrive avec E7.

Reste (non fait) : `eleves.photo.gerer` n'est pas encore branché sur les routes photo ; les nouvelles actions d'écriture ne remplacent pas encore `eleves.gerer` sur les routes ; lots E6 à E11.

## 16 bis. Ajouts de la passe « personas » [personas]

Origine, personas et gravité de chaque point : `docs/socles/PERSONAS_02-04.md`. Le lot indiqué entre crochets est celui qui porte l'ajout ; **E2** signale un changement du lot en cours.

### 16 bis.1 Règles et paramétrages

| # | Règle [personas] | Lot |
|---|---|---|
| R-E1 | **Date de naissance incomplète ou présumée.** `ent_eleve_identite` (et la projection `ent_eleve`) reçoit `date_naissance_precision TEXT NOT NULL DEFAULT 'jour' CHECK (… IN ('jour','mois','annee','inconnue'))` et `date_naissance_statut TEXT NOT NULL DEFAULT 'declaree' CHECK (… IN ('declaree','justifiee','presumee','contestee'))`. La date stockée est la date **retenue** (convention choisie et affichée) ; l'écran montre toujours la précision et le statut. `contestee` suspend le calcul de la majorité et lève une alerte à la direction. Reprise : `jour` / `declaree` pour tous les élèves existants | **E2** |
| R-E2 | **Écriture d'origine et autres graphies.** Colonnes facultatives `nom_ecriture_origine`, `prenoms_ecriture_origine` dans la version (recommandé dans E2, sinon E6) ; table `ent_eleve_alias (eleve_id, valeur, nature ('translitteration','ancienne_graphie','usage_familial'), source)` lue **seulement** par la recherche et le rapprochement des imports, visible avec G2 | E2 (colonnes), E6 (alias) |
| R-E3 | **Sensibilité forcée.** Les motifs codés `adoption`, `changement_prenom`, `changement_mention_sexe`, `protection` posent `sensible = true` sans possibilité de le retirer à la saisie ; la pièce est rangée en catégorie sensible (socle 7). Une **demande** de prénom d'usage non aboutie est scellée et n'apparaît dans aucune vue famille ni aucun historique G5 | **E2** |
| R-E4 | **Motif codé.** `motif_code` (liste du logiciel : `faute_saisie`, `harmonisation_typographique`, `erreur_import`, `decision_etat_civil`, `adoption`, `changement_prenom`, `changement_mention_sexe`, `protection`, `autre`) ; la contrainte devient « `motif_code` obligatoire pour `correction` et `changement_etat_civil` », le texte libre n'étant obligatoire que pour `autre` | **E2** |
| R-E5 | **Changement de régime à des dates autorisées.** `ent_param_regime.dates_changement` : libre, fin de période, fin de trimestre, ou liste de dates. ELV-10 propose la prochaine date autorisée ; une **dérogation motivée** (motif codé : déménagement, raison médicale sans détail, situation familiale, décision de l'établissement) reste possible et tracée. Préréglage public : fin de trimestre [H, NV] | E4 |
| R-E6 | **Modalités de régime.** Un régime local peut porter des modalités (forfait 4 ou 5 jours, jours de repas) déclarées par le socle 40 ; le socle 2 ne stocke que le code et la référence | E4 |
| R-E7 | **Filigrane nominatif.** Toute liste nominative exportée ou imprimée (ELV-25, ELV-37, trombinoscope) porte l'auteur, la date, la mention « document confidentiel » et l'identifiant de l'export (journal) ; non désactivable | E11, E9 |
| R-E8 | **Désambiguïsation.** Quand deux élèves d'une même classe ou d'un même groupe ont la même désignation (prénom + initiale), les listes ajoutent la deuxième lettre du nom, puis le second prénom, puis le numéro ; jamais la date de naissance | E9 |
| R-E9 | **Pastilles ajoutées** au catalogue : `nouvel_arrivant` (première année dans l'établissement, ou entrée depuis moins de 30 jours), `arrive_en_cours_annee`, `redoublant`, `majorite_proche` (J-30, secrétariat et PP), `en_mobilite`, `modalite_particuliere` (sans détail) | E8 |
| R-E10 | **Conflit d'intérêts.** Une personne du personnel qui est aussi responsable, contact, payeur ou ancien membre de foyer d'un élève (détection par les socles 4 et 6, ou déclaration ELV-46) : accès à la fiche de cet élève par la casquette professionnelle **tracé comme accès sensible** et signalé à la direction ; aucune décision sensible sur cet élève ; groupes F2–F4 des **autres** responsables fermés (socle 4, R-A18). Sa casquette de parent est inchangée | E1, E7 |
| R-E11 | **Bris de glace.** Action `eleves.acces_urgence`, accordée par rôle (modèles : Direction, CPE, Infirmerie) ; motif obligatoire dans une liste ; durée par défaut 2 h (bornes 30 min – 24 h) ; ouvre G1 à G3 d'un élève hors portée, jamais G6 ni les données de santé ; alerte immédiate à la direction ; revue a posteriori sous 72 h (« justifié / non justifié ») ; journal d'accès sensible | E1, E7 |
| R-E12 | **Décès d'un élève.** Motif de sortie `deces`. L'événement `eleve.decede` est consommé par : 8 (annule tous les envois programmés et futurs le concernant), 3 (clôt les dossiers de réinscription sans message), 4 (ferme la fiche de rentrée), 12 (retire des appels), 24 et 25 (suspendent relances et prélèvements, sans message), 29 (retire les tâches). La fiche reste consultable. Les frères et sœurs scolarisés sont signalés à la direction, qui décide seule de toute information des équipes | E4 |
| R-E13 | **Mobilité.** `ent_eleve_mobilite (eleve_id, du, au, type ('etranger','soins','cned','autre'), retour_prevu DATE NULL)` ; le statut dérivé reste `scolarise` ; pastille `en_mobilite` ; au retour prévu dépassé sans nouvelle : tâche « confirmer le retour ou enregistrer une sortie » | E4 |
| R-E14 | **Modalités de scolarisation.** `ent_eleve_modalite (eleve_id, code, du, au, source, piece_document_id NULL)` ; codes du logiciel (`temps_partiel`, `double_inscription_cned`, `scolarisation_partagee`, `upe2a`, `ulis`, `autre`) [NV pour les intitulés officiels] ; libellés paramétrables | E4 |
| R-E15 | **Fuseau.** Toutes les dates métier du socle (majorité, statut dérivé, versions datées dans le futur) sont calculées dans le **fuseau de l'établissement** (paramètre du socle 5, STR-03), Europe/Paris par défaut ; test avec un établissement à UTC+4 | E4 |
| R-E16 | **Identifiants dérivés du nom.** Identifiant de connexion, adresse de messagerie interne et pseudonymes générés à partir du nom affiché suivent le prénom d'usage quand il est actif ; le socle 1 consomme `eleve.identity_changed` et **propose** le renommage (ELV-47) | socle 1 |
| R-E17 | **Attestations bilingues.** Les gabarits de certificats et d'attestations existent en français et en une seconde langue (anglais par défaut) ; seules les données sont insérées, aucun texte libre n'est traduit automatiquement ; la version française fait foi | E11 (+7) |
| R-E18 | **Registre des élèves** (ELV-37) : numéro, nom de naissance, prénoms, date de naissance (avec précision), date et motif d'entrée, origine, date et motif de sortie, destination ; à une date ; rien d'autre [NV : contenu exigé selon le statut de l'établissement] | E11 |
| R-E19 | **Extrait de l'élève demandé par un parent** (droit d'accès) : exclut les coordonnées, adresses, motifs et pièces des **autres** responsables, les versions sensibles et tout ce qui concerne d'autres personnes ; l'extrait demandé par l'élève majeur suit d'autres règles (socle 27) | E11 |
| R-E20 | **Non-doublon.** `ent_eleve_non_doublon (eleve_a, eleve_b, motif_code, decide_par, decide_le)` ; la détection ignore la paire tant qu'aucune des deux identités ne change | E6 |
| R-E21 | **Propositions venues du terrain et des campagnes.** Sources `personnel` (ELV-40) et `verification_campagne` (ELV-32) ajoutées à `ent_eleve_proposition.source` | E5 |
| R-E22 | **Ancien élève anonymisé.** Après anonymisation (durées DPO), la réintégration sur la même fiche est impossible : une nouvelle fiche est créée et l'écran le dit ; l'attestation historique (ELV-38) ne s'appuie que sur ce qui est légalement conservé | E4, E11 |
| R-E23 | **Annuaire des établissements.** Référentiel du logiciel (UAI, nom, commune, type, secteur), commun à tous les établissements, importé par le socle 9 depuis la source officielle [NV : source et licence] ; `origine_uai` et `destination_uai` (§7.4) le référencent ; un UAI inconnu reste saisissable avec alerte — repris de Next.js et de Charlemagne (D-024) | E4 (+9) |
| R-E24 | **Documents en trames prêtes (D-023).** Certificat de scolarité, attestation de scolarité passée, registre des élèves, listes et trombinoscopes imprimés, extrait des données : 3 ou 4 trames par type ; l'établissement choisit et n'ajuste que son identité (logo, couleurs, coordonnées, signature, texte libre encadré) ; contenu rempli seul depuis les données ; pas d'éditeur de mise en page. Réglages lourds du socle signalés pour décision commune : `PERSONAS_02-04.md` §5 bis.2 (S-01 à S-06, S-23) | E11 (+7) |

**Droits ajoutés** : `eleves.badge.gerer` (Admin, Direction, Administratif, CPE ; portée établissement), `eleves.acces_urgence` (aucun modèle par défaut hors Direction ; à accorder explicitement). **Événements ajoutés** : `eleve.decede`, `eleve.non_doublon_declare`, `eleve.verification_identite_ouverte` / `_close`, `eleve.majorite_proche`, `scolarite.mobilite_ouverte` / `_fermee`, `scolarite.modalite_changee`, `acces.urgence_ouvert`, `personnel.lien_declare` ; charges utiles sans aucune valeur d'identité.

### 16 bis.2 Cas limites ajoutés

| Cas [personas] | Ce que fait le socle |
|---|---|
| Enfant arrivé de l'étranger, date de naissance connue à l'année seulement | ELV-29 : précision `annee`, statut `presumee` ; âge affiché « environ » |
| Mineur non accompagné dont la minorité est en cours d'évaluation | Statut `contestee` ; aucune majorité calculée ; alerte direction ; la scolarisation n'est pas bloquée |
| Nom écrit de trois façons selon les papiers | ELV-30 : une graphie retenue, les autres en alias de recherche |
| Élève adopté pendant sa scolarité (adoption plénière) | Changement d'état civil avec motif `adoption` : versions antérieures scellées d'office (R-E3) |
| Demande de prénom d'usage refusée par les parents | Rien ne change ; la demande est scellée, invisible des vues famille (R-E3) ; l'accompagnement reste humain |
| Jumeaux, ou deux homonymes nés le même jour | ELV-33 une fois ; plus jamais proposés comme doublons |
| Élève parti un an à l'étranger avec retour prévu | ELV-35 : pas de sortie, place gardée (socle 3) |
| Élève à mi-temps au CNED ou en établissement médico-social | ELV-36 |
| Décès d'un élève | ELV-34 : arrêt de tous les automatismes (R-E12) |
| Secrétaire dont l'enfant est élève de l'établissement | R-E10 : accès tracé, décisions sensibles interdites, données de l'autre parent fermées |

---

## 17. Questions à trancher par le porteur du projet

| # | Question | Options | Recommandation |
|---|---|---|---|
| **Q-E1** | Nouvelle numérotation des socles (2 = élève, 3 = inscriptions, 4 = dossier administratif) : mettre à jour `ROADMAP.md` et rattacher les lots S2-x existants ? | oui / non | **oui** ; les lots S2-0 à S2-4 réalisés deviennent l'acquis de départ des socles 2 à 4 |
| **Q-E2** | Identité portée par l'élève (versions dédiées) ou par une table « personne » générique commune aux élèves, responsables et personnels ? | élève / personne | **élève** : la clé `eleve_id` est partout, l'identité scolaire a ses règles propres (INE, état civil, prénom d'usage) ; `ent_personne` reste pour les responsables et l'élève majeur payeur |
| **Q-E3** | Annuaire : tout le personnel voit-il la désignation (nom, classe, photo) de **tous** les élèves (comme aujourd'hui), ou seulement de ceux de sa portée ? | tous / portée | **annuaire pour tout le personnel** (accueil, couloirs, cantine), sauf élèves en confidentialité renforcée ; la scolarité (G1) et au-delà selon la portée |
| Q-E4 | Prénom d'usage dans le socle 2 ? | oui / plus tard | **oui**, désactivable par l'établissement, procédure paramétrée, versions sensibles ; cadre vérifié **[V]** (circulaire du 29-9-2021 ; accord des deux représentants légaux pour un mineur ; VERIF R17) |
| Q-E5 | Nationalité : la collecter ? | jamais / paramétrable / toujours | **paramétrable, désactivée par défaut**, chiffrée si activée ; à confirmer : exigée par BEE ou une statistique nationale ? [NV] |
| Q-E6 | Unicité de l'INE dans l'établissement seulement (rien entre établissements, D-013) ? | oui / non | **oui** |
| Q-E7 | Public : quand SIECLE fait foi, interdire la modification locale de l'état civil (proposition « à corriger dans SIECLE ») ? | interdire / permettre avec alerte | **interdire par défaut**, réglable par champ |
| Q-E8 | Suppression définitive d'un élève (comme Next.js) ? | oui / non | **non** : seulement l'annulation d'une création erronée sans données rattachées ; sinon fusion, sortie, archivage |
| Q-E9 | Ajouter une portée par **site** (ou désigner des sites / niveaux) pour un secrétariat de site ? | oui / non | **oui**, en étendant la portée « désignés » aux sites et niveaux, dans le lot E1 |
| Q-E10 | Un ancien élève qui revient garde-t-il sa fiche (réintégration) ? | même fiche / nouvelle fiche | **même fiche**, sous réserve des durées de conservation |
| Q-E11 | Statut **dérivé** des scolarisations, plus jamais saisi ? | oui / non | **oui** |
| Q-E12 | Photo proposée par la famille ou l'élève ? | non / paramétrable | **paramétrable, désactivé par défaut**, validation par l'établissement |
| Q-E13 | Un établissement (tenant) peut-il regrouper plusieurs UAI (école + collège + lycée) avec une fiche continue ? | oui / non | **oui**, UAI porté par le site ; à distinguer de l'« ensemble scolaire » entre tenants (Q13, non) |
| Q-E14 | Élève majeur : accès des parents (Q10 de `SOCLE2_DOMAINES.md`, toujours ouverte) | — | inchangée ; le socle 2 fournit le fait `eleve.majeur`, la règle reste à décider (DPO) |
| Q-E15 | Durées de conservation (élève sorti, anciennes photos, versions, propositions refusées) | — | **à demander au DPO** ; aucune durée ne sera codée avant |
| Q-E16 | Délai d'annulation d'une fusion d'élèves | 30 jours / autre | **30 jours**, comme les personnes |
| Q-E17 | Démarches externes pour le format exact de l'INE, l'attribution et le retour BEE, le COG : qui les porte, et autoriser les domaines officiels dans l'environnement de travail ? | — | rejoindre les démarches déjà listées (`INTEROPERABILITE_NATIONALE.md` §12, §15 décisions 2 et 3) |
| **Q-E18** [personas] | Intégrer dans le lot **E2 en cours** la date de naissance incomplète ou présumée (R-E1), le motif codé (R-E4) et la sensibilité forcée (R-E3) ? | maintenant / plus tard | **maintenant** : les ajouter après la reprise obligerait à repasser sur toutes les versions |
| Q-E19 [personas] | Écriture d'origine du nom (R-E2) dans E2 ou plus tard ? | E2 / E6 | **E2** (deux colonnes facultatives, coût nul) |
| Q-E20 [personas] | Bris de glace (ELV-44) : quels rôles, quelle durée, qui fait la revue ? | — | Direction, CPE, Infirmerie ; 2 h ; revue par la direction sous 72 h ; à valider par le DPO |
| Q-E21 [personas] | Décès d'un élève : la liste des automatismes à couper (R-E12) est-elle complète, et qui peut enregistrer le décès ? | — | direction seulement ; liste à compléter par chaque socle (test de garde : tout envoi programmé vérifie `eleve.decede`) |
| Q-E22 [personas] | Nom transmis au GAR et aux ressources numériques : état civil ou prénom d'usage ? | état civil / usage | **usage** quand il est actif, si la réglementation du GAR le permet [NV, à vérifier] ; les examens et exports officiels restent à l'état civil |
| Q-E23 [personas] | Changement de régime limité à des dates autorisées (R-E5) : préréglage pour le public et le privé ? | libre / fin de trimestre | **fin de trimestre** pour le public [H], **libre** pour le privé, dérogation motivée toujours possible |

---

## 18. Récapitulatif : faits, hypothèses, contradictions

**Faits vérifiés [F]** (chemins au §2) : identité écrasée sans historique ; écritures de création et d'identité hors transaction, journal ignoré en cas d'erreur ; état civil en clair dans les événements ; date de naissance exposée par la recherche d'homonymes à des profils « Réduit » (vérifié `service.rs:13288`, `dto.rs:2817`) ; recherche et lecture d'élève ouvertes à tout le personnel (vérifié `routes.rs:968`, `roles.rs:107`) ; INE sans unicité ni provenance, format 9–16 caractères ; 12 tables sans clé étrangère vers l'élève ; aucune RLS sur les tables élève ; UAI présent sur `ent_site` ; aucun import d'élèves dans le Rust ; Next.js : imports SIECLE XML et Excel, export SIECLE ZIP, identifiant SIECLE hors schéma, suppression définitive, pas d'historique de fiche.

**Vérification du 2026-11-09** (`docs/VERIF_REGLEMENTAIRE.md`) : prénom d'usage, émancipation, archives publiques et droit à l'image (C. civ. 372-1 : les parents protègent **en commun** le droit à l'image de l'enfant mineur, VERIF R15) sont désormais [V] ; restent [NV] l'INE, le GAR et les droits des parents d'un élève majeur (R16).

**Hypothèses principales [H] / non vérifié [NV]** : format et familles de l'INE ; mécanisme d'attribution et de retour de l'INE (RNIE, BEE, ONDE) ; obligations déclaratives du hors contrat ; cadre du prénom d'usage (circulaire de 2021) ; codes nationaux de sexe, régime, motifs de sortie, statut de l'apprenant ; nomenclature COG attendue ; capacité juridique (liste des mesures) ; règles d'archives publiques ; comparaison avec les logiciels du marché ; volumes des lots.

**Contradictions signalées** :
- `ROADMAP.md` (socle 2 = domaines) contre la numérotation de cette demande (socle 2 = élève) — Q-E1.
- `perimetre.rs` (« un professeur ne voit que ses élèves ») contre `GET /eleves/search` et `GET /eleves/{id}` ouverts à tout le personnel — Q-E3.
- `dossier_personnes.rs:16-17` (« une transaction = fait + journal + événement ») contre `create_eleve` et `update_identite` — lot E0.
- Écran dossier : droit « famille » calculé côté client, contre le principe « le serveur décide » — lot E9b.
- `outbox.rs:638` nomme `ent.etablissement_id`, la RLS lit `app.etablissement_id` (rapport d'exploration, à vérifier) — lot E7.
