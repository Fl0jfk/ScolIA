# Socle 27 — Conformité RGPD, conservation et archives

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031) sauf ce qui engage l'établissement à l'extérieur (§24.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production. Le code Next.js a été **lu seulement**, en lecture ciblée (`app/lib/rgpd-*.ts`, `app/api/rgpd/*`), sans rien exécuter.
**Branche :** `ent-socle-27-28-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas :** `docs/socles/PERSONAS_27.md` ; ses ajouts sont faits **ici**, marqués **« ajout personas »**, et regroupés au **§26** (qui l'emporte en cas de contradiction ; rien n'est supprimé).
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-013 (identité unique), D-017 (droits portés par les actions), D-020 (personne ≠ compte), D-021, D-026 (kit de brique), D-031, D-034 (année de doublon 2027-2028), D-035, **D-036**, **D-037 à D-046** (sauvegardes, registre des tables, portabilité) ; `docs/PRA_SAUVEGARDES.md` (§11 restauration d'un établissement, **§18.4 exigences E1–E14**) ; `docs/PORTABILITE_EXPORTS.md` (export d'établissement, double approbation) ; `ent-rust/crates/ent-db/src/registre_donnees.rs` (registre de classement des tables) ; `docs/DATA_GOVERNANCE.md` ; `docs/MISE_EN_SERVICE_DONNEES.md` (A7, A8, A11–A15, B3, B4, B10) ; `docs/VERIF_REGLEMENTAIRE.md` (R18–R22, **R39–R46**) ; `docs/IDENTITE_ET_PROFILS_FAMILLES.md` §2 (qui est responsable de quoi) ; conceptions 02 §10.6, 04 §12.5, **07 §13** (conservation des documents, à reprendre ici comme projection), 13 §19.7 (effacement des sanctions), 16 §15, 34 §18 et les §« Conservation » de tous les socles conçus ; `IA_COEUR.md` §6.2 (niveaux N0–N3) et §9. Modèles de forme : `42_SORTIES_VOYAGES.md`, `10_EMPLOI_DU_TEMPS.md`.

**Conventions.** **[F]** fait vérifié dans le code (chemins depuis `ent-rust/` ; pour Next.js, depuis la racine du dépôt) ; **[V]** règle lue sur une source officielle (§4, avec date) ; **[V-ext]** lue en extrait ; **[NV]** non vérifiée ; **[H]** hypothèse ; **[R]** recommandation non décidée.

**Numérotation propre au socle** (vérifiée libre sur les 134 branches distantes le 2026-11-11) :

| Élément | Numéros |
|---|---|
| Actions existantes de Next.js (module « Conformité RGPD ») | NX27-01… |
| Défauts de l'existant (Rust et Next.js) | RG-D01… |
| Actions cibles | **RGP-100 à RGP-299** |
| Règles | **R-RG1…** |
| Cas limites | **CL27-01…** |
| Questions | **Q-RG1…** |
| Réglages signalés | **S-540 à S-559** |
| Manques de la passe personas | M-2700 à M-2799 |
| **Migrations** | **2540–2559** (plage imposée par le pilote, §21) |

---

## 0. En bref

1. **Le socle 27 n'est pas un écran « RGPD » de plus : c'est le service qui rend chaque autre socle conforme.** Chaque socle déclare dans son kit (D-026) ses tables (registre E12), ses **catégories de conservation**, son **fournisseur d'extrait** (ce qu'il sait dire d'une personne), son **exécuteur de sort final** (anonymiser, détruire, archiver ses propres objets) et ses **traces de consultation**. Le 27 possède le **référentiel**, les **décisions** et les **preuves** ; il n'efface jamais lui-même les tables d'un autre socle (règle 1 de `SOCLES.md` §1).
2. **Ce qui existe [F]** : un journal en ajout seul (068, 091), des traces de consultation sur la santé, le psy et le dossier administratif, un registre de classement de 349 tables où **toutes** les durées sont « à définir » (`Retention::ADefinir`), un champ `export_portabilite` que rien ne lit, l'effacement réglé des sanctions sans exécuteur, la clôture d'année en lecture seule. **Rien** pour : l'extrait d'une personne, les demandes de droits, la purge planifiée, l'archivage de sortie, les lots d'élimination, les violations, le registre des traitements, les sous-traitants. Côté Next.js, un module « Conformité RGPD » **partiel** mais utile (questionnaire, 15 modèles de documents, score, registre des incidents) stocké en **un seul JSON** (D-036) ; ses actions sont reprises (NX27-01 à NX27-22), ses défauts non (§2.3).
3. **Cinq piliers** (le reste en découle) :
   - **Référentiel de conservation** : catégories proposées par ScolIA avec leur source (instruction de tri des EPLE de 2005, R511-13, CNIL…), **adoptées et validées par l'établissement et son DPO** ; tant qu'une catégorie n'est pas validée, **rien n'est détruit** et le tableau de bord le dit (R-RG1).
   - **Cycle de vie des dossiers** : actif → clos (fait générateur daté : sortie de l'élève, fin d'année, fin de démarche) → **conservation intermédiaire** (visible du seul service habilité) → **sort final** (destruction, anonymisation, versement, conservation définitive), avec **gel** pour contentieux ou signalement, par **lots** relus, visés quand la loi l'exige (public, et privé sous contrat par prudence tant que le juriste n'a pas tranché), exécutés par les socles propriétaires, prouvés par un certificat.
   - **Droits des personnes** : un guichet unique, un délai calculé (un mois, prolongeable de deux), un **extrait assemblé automatiquement** à partir des fournisseurs des socles, **relu par un humain** avant remise (masquage des tiers : l'autre parent, une adresse protégée, une note interne), une réponse motivée en cas de refus (obligation légale, archives).
   - **Transparence des consultations** : chaque famille et chaque élève majeur voit **qui a consulté quoi** (fonction et service, date, nature, motif quand il est exigé), hors gestes de service courants ; les consultations sensibles sont tracées **avant** d'être servies (lot 0, R-RG8).
   - **Pilotage de conformité de l'établissement** : registre des traitements **généré** à partir des tables réelles et des droits réels (pas un formulaire à remplir à vide), sous-traitants avec leurs régions, AIPD assistée, mentions d'information publiées et versionnées, registre des violations avec horloge des 72 h.
4. **ScolIA a deux casquettes, et le logiciel les sépare** : **sous-traitant** de chaque établissement (art. 28) pour les données scolaires ; **responsable de traitement** pour la couche « compte personne » commune à plusieurs établissements (D-013). D'où deux registres, deux jeux de mentions et deux circuits de demandes (§5).
5. **Réglementaire qui commande le logiciel** (§4) : délais RGPD art. 12.3 ; droit d'opposition écarté pour l'obligation légale, examiné pour la mission d'intérêt public ; **pas de droit à la portabilité** pour un traitement fondé sur la mission d'intérêt public (art. 20.3) — ScolIA remet quand même un extrait réutilisable, par bonne pratique ; archives publiques dans le public **et potentiellement** dans le privé sous contrat (C. patr. L211-4) ; **aucune élimination sans visa** (R212-14) ; journaux d'accès 6 mois à 1 an (CNIL) ; effacement des sanctions (R511-13 IV) ; notification d'une violation sous 72 h (art. 33).
6. **Le plan** :

   | Élément | Volume | Numéros |
   |---|---|---|
   | Actions | 200 | RGP-100 à RGP-299 |
   | Cas limites | 62 | CL27-01 à CL27-62 |
   | Règles | 40 | R-RG1 à R-RG40 |
   | Réglages signalés | 14 | S-540 à S-553 |
   | Lots | 13 | G0 à G12 |

   - **Lot G0 (correctifs urgents, avant toute donnée réelle)** : traces de consultation sensibles **transactionnelles et bloquantes** (aujourd'hui « au mieux », une lecture de santé réussie peut ne laisser aucune trace) ; `TRUNCATE` interdit sur le journal ; test du registre qui **ne passe plus en silence** sans base ; fin du mot de passe de développement dans un commentaire ; carte des catégories de conservation par table (sans durée appliquée) ; procédure écrite des demandes de droits et des violations utilisable dès le premier jour, même sans écran.
   - **Parcours minimal pour l'année de doublon 2027-2028** (premières données réelles) : G0, G1 (référentiel), G2 (registre généré), G3 (guichet des droits et extrait), G4 (journal des accès visible), G5 (violations). Le cycle de vie avec destruction réelle (G6–G8) n'est **pas** nécessaire la première année : aucune donnée n'atteint son sort final avant 2028 au plus tôt (R-RG2).
7. **Décisions appliquées par défaut (D-031)** : 28 (§24.1). **Soumis à Florian** (§24.2) : nomination d'un DPO pour l'établissement pilote et validation des durées ; avis du juriste sur les archives du privé sous contrat (L211-4 2°) ; contrat de sous-traitance type et liste des sous-traitants de la plateforme ; AIPD de l'établissement ; position de ScolIA comme responsable de la couche compte ; désignation d'un référent violation chez l'éditeur.

---

## 1. Thèse : la conformité se fabrique dans chaque socle, elle se prouve dans le 27

Un ENT accumule en dix ans des millions de lignes sur des mineurs. La CNIL ne demande pas un « module RGPD » : elle demande qu'on sache **pourquoi** on garde chaque donnée, **combien de temps**, **qui** la voit, et qu'on puisse le **prouver** et le **rendre** à la personne. Les outils du marché répondent par un formulaire de registre à remplir à la main, déconnecté des données réelles [H] : le registre dit « notes conservées 1 an » pendant que la base garde tout depuis 2009.

ScolIA a un avantage structurel que personne n'a : **le registre de classement des tables est dans le code et vérifié par la CI** (D-037, `registre_donnees.rs`). Chaque table y a déjà un domaine, une portée, un drapeau « données personnelles », « santé », « fichiers ». Le socle 27 **prolonge ce registre** jusqu'à la conformité :
- une table ne peut pas exister sans **catégorie de conservation** (le test CI le vérifiera comme il vérifie déjà le classement) ;
- le **registre des traitements** est généré à partir de ce classement et du catalogue des droits (D-017) : ce que la fiche dit est ce que le logiciel fait ;
- l'**extrait** d'une personne est assemblé par les socles qui détiennent ses données, pas par une requête générique qui oublierait une table ;
- la **destruction** est exécutée par le socle propriétaire, avec ses règles métier (une sanction effacée selon R511-13, une facture gardée dix ans), et prouvée.

Ce que la conformité ne doit **jamais** devenir : un frein quotidien. Le professeur ne voit rien du 27, sauf un motif à choisir quand il ouvre un dossier qui n'est pas celui de ses élèves. Le secrétariat voit une file de demandes avec des échéances. Le DPO voit un tableau de bord, valide des durées une fois, relit des lots deux fois par an.

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust [F] (inventaire du 2026-11-11)

| Brique | Où | État | Utilité pour le 27 |
|---|---|---|---|
| Journal des modifications | `migrations/068_journal_modif.sql:3-23` ; ajout seul `091_journal_ajout_seul.sql:8-17` | `UPDATE`/`DELETE` refusés par déclencheur ligne à ligne ; **`TRUNCATE` non couvert** ; `payload JSONB` (instantané, exception D-036 permise) ; colonne `visible_famille` | base de l'historique ; à compléter (TRUNCATE, rôle non propriétaire) |
| Écritures du journal | transactionnelles : `ent-db/src/scolarite.rs:337-374`, `identite.rs:618-633`, `dossier_personnes.rs:600` ; autonome : `journal_repo.rs:20-51` (propre transaction) ; **au mieux** : `service.rs:3888-3912` (`audit_acces`), `service.rs:3919-3944` (`tracer_acces_sensible`, erreur seulement journalisée, l. 3942) | mixte | les traces de consultation sensibles doivent devenir bloquantes (G0) |
| Traces de consultation | santé et PAI `service.rs:4040, 4063, 4091, 4121, 4151` ; psy `4174` ; `sante_acc.rs:838, 1363, 1486` ; dossier administratif `dossier_droits.rs:419, 443, 637` ; identité, identifiants, scolarité, propositions, cahier, contacts, accompagnement | trace posée **après** la lecture ; toujours `visible_famille = false` | matière première du journal des accès visible (G4) |
| Lecture du journal | `journal_repo.rs:53-97` (`list_eleve`, filtre `visible_famille = true`, l. 67) ; `service.rs:13243-13330` ; routes `/eleves/{id}/journal` (`routes.rs:465`), `/audit/journal` (`routes.rs:192`, droit `audit.lire`) ; conflits d'intérêts exclus (`service.rs:13293-13299`) | la famille voit des **modifications**, jamais des **consultations** | G4 |
| Registre de classement des tables | `ent-db/src/registre_donnees.rs` (1 038 lignes, 349 tables, 19 domaines ; 237 « données personnelles », 14 « santé », 195 « export ») | `Retention` ne vaut que `ADefinir` ou `Ephemere` ; `export_portabilite` **sans aucun lecteur** ; test `chaque_table_de_la_base_est_classee_et_conforme` (l. 837-907) qui **retourne sans rien vérifier** si `ENT_DATABASE_URL` est absente hors CI (l. 829-839) | **épine dorsale** du 27 : on remplace `ADefinir` par une catégorie (G0/G1) |
| Minimisation | `ent-auth/src/minimisation.rs` (`vue_personnel` l. 200-220, `vue_titulaire` l. 226-245, `acces_a_tracer` l. 179-181) | groupes de champs des responsables (F0–F4, payeurs), conflit d'intérêts ; ne couvre ni santé ni journal | modèle à étendre au journal visible (§11) |
| Effacement des sanctions | `ent_regle_effacement` ; `PUT /vs/regles-effacement/{categorie}` (`ent-api/src/vie_scolaire.rs:54, 506-520`) ; `discipline_vs.rs:43-57` | règles enregistrées, **aucun exécuteur** trouvé | premier exécuteur de sort final (socle 13, lot D7) |
| Purges | sessions et jetons seulement (`service.rs:3702, 3774`), au **démarrage** (`ent-api/src/lib.rs:251-256`) | pas de purge planifiée | moteur du 27 (G6) |
| Clôture d'année | `ent-db/src/referentiel_annee.rs:328` (lecture seule) | existe | fait générateur « fin d'année » |
| Catalogue d'actions | `journal.lire` (`actions.rs:465`), `audit.lire` (499), `eleves.exporter` (535), `dossier_admin.exporter` (571, commentaire « extrait RGPD » l. 253, **aucune fonction**) | aucune action `rgpd.*`, `conservation.*`, `archives.*` | §12 |
| Chiffrement applicatif | IBAN, santé, psy (`ENT_SECRETS_KEY`, `enc:v1:` sans identifiant de clé) | E11 demande `enc:v2:<id_cle>:` | destruction cryptographique possible plus tard (§8.4) |

### 2.2 Next.js : module « Conformité RGPD » (P04) — ce qu'il fait, repris [F]
Lecture ciblée : `app/lib/rgpd-types.ts`, `rgpd-catalog.ts`, `rgpd-processing-fiches.ts`, `rgpd-access.ts`, `rgpd-storage.ts`, `rgpd-ai.ts`, `rgpd-scoring.ts`, `app/api/rgpd/*` (2 830 lignes de bibliothèque, 10 routes, 1 page `(admin)/conformite-rgpd`). Maturité « Partiel » (`TRANSITION_T0_INVENTAIRE.md` §3.0) ; T0 §3.3 classe ses données « à conserver » (registre des violations et demandes en cours).

| N° | Ce que fait Next.js | Où | Reprise |
|---|---|---|---|
| NX27-01 | Questionnaire de conformité en 7 étapes (profil, gouvernance, publics et données, traitements sensibles, outils et sous-traitants, mesures actuelles, synthèse), reprise là où on s'est arrêté | `RgpdQuestionnaireWizard.tsx`, `rgpd-types.ts` (`questionnaireStep`) | RGP-200 : **pré-rempli** par ce que ScolIA sait déjà (structure, modules activés, sous-traitants de la plateforme) ; ne demande que ce qu'il ignore |
| NX27-02 | Identité de l'établissement pour le registre : raison sociale, type, niveaux, coordinateur, DPO (interne/externe, contact), **contact du service d'archives**, référent administratif | `RgpdEstablishmentIdentity` | RGP-201 (table `rgp_gouvernance`) |
| NX27-03 | Choix du logiciel de gestion en place (Charlemagne + École Directe, Pronote, École Directe, autre) et d'un second logiciel | `RgpdEntPresetId` | RGP-202 : traitements **hors ScolIA** déclaratifs (l'établissement garde Charlemagne pendant le doublon) |
| NX27-04 | Activités de traitement cochées (ENT, messagerie, dossiers papier, OneDrive et classement par IA, santé, discipline, archives historiques, finances) | `RgpdProcessingActivities` | RGP-203 : les activités ScolIA sont **déduites** des modules actifs ; les autres déclaratives |
| NX27-05 | Effectifs (élèves, enseignants, personnels) pour dimensionner | `studentCount`… | **lus** dans la structure (socle 5), pas ressaisis |
| NX27-06 | Publics concernés (élèves, parents, personnels, prospects, anciens élèves) | `audiences` | RGP-204 (déduits + complétés) |
| NX27-07 | Traitements sensibles (photos et vidéos, publications, internat, santé, vidéosurveillance, biométrie, outils d'IA) | `sensitiveProcessing` | RGP-205 ; déclenchent les critères d'AIPD (§8.9) |
| NX27-08 | Sous-traitants (ENT, Microsoft 365, AWS, Mistral, autres SaaS) et **accords de traitement des plateformes** (Microsoft, AWS, Mistral, better-auth) | `subprocessors`, `platformDpas`, `rgpd-platform-dpas.ts` | RGP-180 à RGP-189 ; liste de la plateforme ScolIA **fournie** et tenue à jour par l'éditeur |
| NX27-09 | Mesures existantes (registre, procédure de violation, charte informatique, charte photo) | `existingMeasures` | RGP-206 |
| NX27-10 | Catalogue de **15 documents** avec niveau (obligatoire, recommandé, conditionnel) : registre des traitements, mentions d'information (élèves, parents, personnel), procédure d'exercice des droits, procédure de violation (72 h), registre des violations, liste des sous-traitants, politique de protection, charte informatique, charte photos, désignation du DPD, AIPD, politique de conservation, mesures de sécurité (PSSI simplifiée), fiches incident violation et sécurité | `rgpd-catalog.ts` | RGP-210 à RGP-224 : modèles versionnés, **générés depuis les données réelles** quand c'est possible (registre, sous-traitants, conservation, mesures) |
| NX27-11 | États d'un document : manquant, brouillon, généré, importé, validé, non applicable (avec raison) | `RgpdDocumentStatus` | RGP-225 |
| NX27-12 | Génération d'un document à partir du questionnaire, export PDF | `api/rgpd/documents/[docId]/generate`, `rgpd-document-pdf.ts` | RGP-226 (socle 7 pour le PDF, version figée) |
| NX27-13 | Import d'un document existant (URL de dépôt), téléchargement | `upload-url`, `download` | RGP-227 (dépôt du socle 7) |
| NX27-14 | **Analyse par IA** d'un document importé contre les critères de l'art. 30 : score, critères présents et manquants, améliorations | `analyze/route.ts`, `rgpd-ai.ts:14` | RGP-228 : **conservée**, sur un document **sans donnée personnelle** (modèle, procédure), avec le fournisseur d'IA autorisé (§10) |
| NX27-15 | Fiches de traitement préconstruites (identité, gestion administrative, messagerie, dossiers papier, OneDrive et IA, santé, discipline, archives historiques, finances) et **fiche de synthèse des durées de conservation** | `rgpd-processing-fiches.ts` | RGP-120 à RGP-129 (fiches générées) ; la synthèse des durées vient du référentiel |
| NX27-16 | **Score de conformité** (documents obligatoires et recommandés présents, questionnaire, qualité moyenne) et liste d'actions suggérées | `rgpd-scoring.ts:30-142` | RGP-230 **transformé** : pas de note sur 100 (fausse assurance, RG-D15) mais une **liste de points ouverts** classés par gravité |
| NX27-17 | Registre des incidents : violation de données ou incident de sécurité, champs (découverte, nature, catégories, nombre de personnes, gravité, mesures immédiates, notification CNIL nécessaire et date prévue, personnes informées, responsable, chronologie) | `RgpdDataBreachFields`, `RgpdSecurityIncidentFields`, `api/rgpd/incidents` | RGP-240 à RGP-259 (champs typés, horloge des 72 h calculée) |
| NX27-18 | Fiche incident en PDF | `incidents/[id]/pdf` | RGP-251 |
| NX27-19 | **Assistant IA conversationnel** pour qualifier un incident et remplir la fiche | `incidents/chat/route.ts`, `rgpd-ai.ts` | RGP-257 **conservé avec garde-fous** (§10 : aucune donnée personnelle envoyée, l'IA n'arbitre jamais la notification) |
| NX27-20 | Historique des actions du module (100 dernières entrées) | `workspace/route.ts:74-75` | journal en ajout seul, sans troncature (RG-D12) |
| NX27-21 | Annuaire des utilisateurs pour désigner le coordinateur ou le DPO | `api/rgpd/directory-users` | RGP-201 (personnes du socle 6, sans exposer les rôles de tous, RG-D16) |
| NX27-22 | Avertissement « ceci n'est pas un avis juridique » | `RgpdDisclaimer.tsx` | gardé sur chaque document généré |

### 2.3 Défauts relevés (à ne pas reproduire ; corrigés en lot G0 ou dans le lot qui reprend l'action)

| N° | Défaut | Où [F] | Gravité | Correctif |
|---|---|---|---|---|
| RG-D01 | Traces de consultation sensibles **au mieux** : une erreur d'écriture est journalisée, la lecture est servie quand même ; la trace est posée **après** la lecture | Rust `service.rs:3919-3944`, `4040-4174` | **critique** | G0 : trace écrite dans la transaction de lecture, refus si elle échoue (R-RG8) |
| RG-D02 | `TRUNCATE` non bloqué sur `ent_journal_modif` ; le propriétaire peut désactiver le déclencheur | `091_journal_ajout_seul.sql` | élevée | G0 : déclencheur `BEFORE TRUNCATE` (instruction) ; rôle applicatif non propriétaire (déjà prévu, `RLS_ISOLATION.md` §2.1) |
| RG-D03 | Test du registre qui passe **sans rien vérifier** quand la base est absente hors CI | `registre_donnees.rs:829-839` | élevée | G0 : échec si `CI` ou `ENT_EXIGER_BASE` est posé ; message explicite sinon |
| RG-D04 | Toutes les tables en `Retention::ADefinir` : aucune durée, aucune catégorie | `registre_donnees.rs:62-67, 146` | élevée | G0 (catégorie par table, sans durée appliquée) puis G1 |
| RG-D05 | `export_portabilite` sans lecteur : un drapeau que rien ne contrôle finit faux | `registre_donnees.rs:110` | moyenne | remplacé par « fournisseur d'extrait déclaré » vérifié par test (R-RG14) |
| RG-D06 | Règles d'effacement des sanctions sans exécuteur : la règle est affichée, rien ne s'efface | `discipline_vs.rs:43-57` | élevée (R511-13 IV) | socle 13, lot D7, branché sur le moteur du 27 (G6) ; d'ici là, tableau des sanctions à effacer (G1) |
| RG-D07 | Commentaire « extrait RGPD » sur `DossierAdminExporter` sans fonction | `ent-auth/src/actions.rs:253` | faible | G3 |
| RG-D08 | Purge des sessions et jetons au seul démarrage | `ent-api/src/lib.rs:251-256` | moyenne | tâche périodique (socle 28, lot MS0) |
| RG-D09 | Journal mêlant modifications (à garder aussi longtemps que l'objet) et consultations (6 mois à 1 an, CNIL) dans la même table en ajout seul : impossible de purger les unes sans les autres | `068`, `service.rs:3905, 3937` | élevée | G4 : table de traces d'accès séparée, partitionnée par mois (§17) |
| RG-D10 | Mot de passe de développement en clair dans un commentaire | `ent-lab/src/main.rs:4` | faible | G0 (retrait du commentaire) |
| RG-D11 | Next.js : tout le module (questionnaire, documents, historique) dans **un** document JSON réécrit à chaque enregistrement ; incidents en JSON | `rgpd-storage.ts:40-101` (`getJson`/`putJson`) | élevée (D-036, écrasements concurrents) | tables typées (§17) ; reprise en ventilant (§19) |
| RG-D12 | Next.js : historique tronqué aux 100 dernières entrées | `workspace/route.ts:74-75` | élevée (une preuve qui s'efface) | journal en ajout seul |
| RG-D13 | Next.js : le rôle **comptabilité** accède aux incidents de violation et au registre | `rgpd-access.ts:5-9` | moyenne (moindre privilège) | droits séparés : `conformite.lire`, `violations.gerer` (§12) |
| RG-D14 | Next.js : l'assistant d'incident envoie à Mistral **le texte saisi, l'historique fourni par le navigateur et les champs** ; une description d'incident contient souvent des noms | `incidents/chat/route.ts:23-44`, `rgpd-ai.ts:26-57` | élevée | §10 : historique tenu côté serveur, texte **pseudonymisé** avant envoi, avertissement, aucune donnée de santé |
| RG-D15 | Next.js : **score de conformité sur 100** à partir de documents présents : un 90/100 rassure alors que les durées ne sont appliquées nulle part | `rgpd-scoring.ts` | moyenne | points ouverts classés, jamais une note |
| RG-D16 | Next.js : l'annuaire expose adresse et rôles de tous les utilisateurs aux détenteurs du module | `directory-users/route.ts:25-29` | faible | recherche nominative, champs minimaux |
| RG-D17 | Next.js : la liste des sous-traitants cite AWS et better-auth, que ScolIA n'utilise pas | `rgpd-platform-dpas.ts` | moyenne (registre faux) | liste de la plateforme tenue par l'éditeur (§8.8) |

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)
Le questionnaire et ses réponses déjà saisies ; les documents importés ou validés par l'établissement (avec leur date) ; **le registre des incidents** (obligation de documentation de l'art. 33.5, toutes les violations, même non notifiées) ; le contact du DPO et du service d'archives ; l'avertissement juridique ; l'aide de l'IA à la rédaction.

### 2.5 Verdict
Le Rust a les **fondations** (journal, registre de tables, traces, chiffrement) mais aucun **processus** ; le Next.js a des **processus documentaires** mais aucun lien avec les données. Le 27 marie les deux : les documents sont **générés depuis les données réelles**, et les données **obéissent** aux documents.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il doit faire | Ce qui lui fait perdre du temps / ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|
| **Chef d'établissement** (responsable de traitement) | répondre de la conformité devant la CNIL, l'académie ou l'OGEC ; signer l'AIPD et les notifications ; arbitrer les refus d'effacement | ne pas savoir ce qui est gardé ; découvrir une violation par la presse ; signer un registre qu'il n'a pas compris | une page « où en est-on » en langage clair ; une alerte à la minute en cas de violation ; des documents prêts à signer |
| **DPO** (souvent mutualisé : académie, diocèse, OGEC, cabinet) [H] | tenir le registre, valider les durées, conseiller l'AIPD, être le point de contact des personnes et de la CNIL | registres périmés ; durées jamais appliquées ; demandes reçues hors délai ; 15 établissements avec 15 outils | un tableau multi-établissements (s'il en suit plusieurs) **sans voir les données des élèves** ; valider une durée une fois et la voir appliquée ; une file des demandes avec échéances |
| **Secrétariat / direction administrative** | recevoir les demandes (souvent par courrier ou au guichet), vérifier l'identité, rassembler les pièces, répondre | chercher les données d'une famille dans dix écrans ; ne pas savoir ce qu'on a le droit de remettre ; l'ex-conjoint qui demande « tout le dossier » | un extrait assemblé en un clic, avec les passages à masquer signalés ; des modèles de réponse ; l'échéance qui se rappelle seule |
| **Archiviste de l'établissement / documentaliste** [H] | trier, éliminer avec visa, verser | bordereaux à la main ; dossiers d'élèves partis il y a 30 ans dans des cartons | bordereaux générés, versements au format des archives publiques |
| **Archiviste départemental** (service d'archives compétent, public) | viser les éliminations, recevoir les versements | bordereaux incomplets ; éliminations sans visa | bordereau conforme au modèle, liste détaillée, échantillons |
| **CPE, vie scolaire** | consulter les dossiers pour agir | devoir justifier chaque clic | **rien à faire** dans la classe habituelle ; motif demandé seulement hors périmètre |
| **Professeurs** | consulter leurs élèves | idem | idem ; savoir que leurs propres consultations sont visibles par fonction, pas par nom (§11.3) |
| **Infirmerie, psychologue** | données de santé et de suivi | être tracés par des personnes qui n'ont pas à voir leurs consultations | santé hors du journal familial standard ; règles du socle 34 |
| **Comptabilité** | garder les pièces 10 ans (privé) ou jusqu'à l'apurement + 1 an (public) | une purge qui effacerait une facture contestée | catégories financières à part, gel automatique d'une créance ouverte |
| **Accueil** | consulter qui vient chercher un élève | — | rien de nouveau |
| **Familles** (dont parents séparés, parent sans exercice de l'autorité, famille d'accueil) | savoir ce qu'on sait de leur enfant, qui l'a consulté, corriger une erreur | demander par courrier recommandé et attendre deux mois ; recevoir les données de l'autre parent | demander en ligne, suivre l'avancement, voir les consultations ; ne jamais voir l'adresse protégée de l'autre parent |
| **Élève mineur** | comprendre ce qu'on garde sur lui ; parfois demander lui-même | un texte juridique illisible | une page « tes données » écrite pour lui (§8.11) |
| **Élève majeur, ancien élève** | exercer ses droits seul ; obtenir ses bulletins ou un certificat des années après | ne plus avoir de compte ; ne pas savoir à qui écrire | un guichet accessible sans compte actif, identité vérifiée |
| **Personnels** (personnes concernées) | accès à leur dossier, à leurs traces d'activité | être surveillés par le journal | savoir qui consulte leur dossier RH (§11.4) ; journal non utilisé pour évaluer (R-RG27) |
| **AESH, intervenants, bénévoles** | idem, données limitées | — | idem |
| **OGEC / organisme de gestion** (privé) | parfois co-responsable (personnels, finances) [H] | flou sur qui est responsable | répartition écrite par traitement (§5) |
| **Florian / éditeur ScolIA** (sous-traitant, et responsable de la couche compte) | aider ses clients, notifier ses violations, tenir son registre de sous-traitant, prouver ses mesures | répondre à 50 questionnaires de DPO différents | un dossier de conformité de la plateforme publié, à jour (§8.8) |
| **CNIL** (contrôle) | — | — | registre, AIPD, journaux, preuves en quelques minutes |
| **Attaquant** (parent malveillant, personnel curieux, ex-conjoint) | utiliser le droit d'accès pour obtenir les données d'un tiers ; masquer ses consultations | — | — (§13, cas limites CL27) |

---

## 4. Ce qui est réglementaire (vérifié le 2026-11-11)

Sources lues pour ce socle ou reprises de `VERIF_REGLEMENTAIRE.md` (2026-11-09) ; voir §4.9 pour les URL. Ce tableau ne remplace ni le DPO ni le juriste : la colonne « Avis » dit qui tranche.

### 4.1 Principes et délais (RGPD)
| Point | Source | Règle | Effet dans ScolIA | Avis |
|---|---|---|---|---|
| Limitation de la conservation | RGPD art. 5.1.e | données conservées « pendant une durée n'excédant pas celle nécessaire au regard des finalités » ; conservation plus longue possible pour l'**archivage dans l'intérêt public** (art. 89) | chaque catégorie a une durée **et** un sort ; le versement aux archives est un sort prévu | DPO |
| Délai de réponse | RGPD art. 12.3 [V, VERIF R20] | un mois, prolongeable de deux mois « compte tenu de la complexité et du nombre de demandes », avec information dans le premier mois | horloge calculée par demande, prolongation motivée et notifiée (RGP-148) | — |
| Demandes infondées ou excessives | RGPD art. 12.5 [V-ext] | refus possible ou frais raisonnables, la charge de la preuve pèse sur le responsable | motif « manifestement excessive » réservé à la direction, jamais automatique (R-RG17) | DPO |
| Identité du demandeur | RGPD art. 12.6 [V-ext] | informations complémentaires demandées en cas de doute raisonnable | vérification d'identité proportionnée (§9.2) | — |
| Droit d'accès | RGPD art. 15 [V-ext] | confirmation, copie, finalités, catégories, **destinataires**, durée, source ; la copie ne porte pas atteinte aux droits d'autrui (15.4) | extrait assemblé, masquage des tiers obligatoire avant remise (R-RG15) | — |
| Consultations du dossier | CJUE, 22-6-2023, C-579/21 [V-ext] | la personne a droit aux dates et finalités des consultations de ses données ; l'identité des salariés qui ont consulté n'est due que si elle est indispensable à l'exercice de ses droits | journal visible par **fonction et service**, pas par nom (§11.3) ; le nom sur demande motivée, après arbitrage | DPO |
| Rectification | RGPD art. 16 | sans délai | propositions de rectification des socles 2 et 4 | — |
| Effacement | RGPD art. 17 ; exceptions 17.3 b (obligation légale, mission d'intérêt public) et d (archivage) | l'effacement ne joue pas quand la donnée est nécessaire à une obligation légale ou à une mission d'intérêt public | refus motivé par catégorie, texte de la réponse pré-rédigé (RGP-157) | DPO |
| Limitation | RGPD art. 18 | gel du traitement pendant une contestation | statut « limité » sur un objet : lisible des seuls gestionnaires, plus de diffusion (RGP-160) | — |
| Portabilité | RGPD art. 20.3 [V-ext] | « ne s'applique pas au traitement nécessaire à l'exécution d'une mission d'intérêt public » ; vaut pour le consentement et le contrat | **pas de droit à la portabilité** sur la mission d'intérêt public (VERIF R18) ; dans le privé, certains traitements reposent sur le **contrat** (facturation) : portabilité de ces données-là [H] ; ScolIA remet de toute façon un extrait réutilisable (CSV) | DPO |
| Opposition | RGPD art. 21 ; VERIF R3 | ne joue pas pour l'obligation légale ; pour la mission d'intérêt public, la personne invoque sa situation particulière et le responsable démontre des motifs impérieux | demande examinée, jamais refusée par défaut sans examen (RGP-162) ; refus d'un **canal**, jamais de l'information due (socle 8) | DPO |
| Registre | RGPD art. 30.1 (responsable) et 30.2 (sous-traitant) | contenu fixé : finalités, catégories de personnes et de données, destinataires, transferts, délais d'effacement, mesures de sécurité | registre **généré** (§8.1) ; ScolIA tient son registre de sous-traitant pour tous ses clients | DPO |
| Sous-traitance | RGPD art. 28.2, 28.3 (a à h) | sous-traitant ultérieur sur autorisation écrite, information de tout changement avec possibilité d'objection ; en fin de prestation, **suppression ou restitution** au choix du responsable (28.3.g) ; aide aux demandes de droits (28.3.e) ; audits (28.3.h) | §8.8 ; réversibilité (`PORTABILITE_EXPORTS.md` §10) | J |
| Violation | RGPD art. 33, 34 [V, VERIF R21] | notification à la CNIL « dans les meilleurs délais et, si possible, 72 heures au plus tard » ; le sous-traitant notifie le responsable « dans les meilleurs délais » (33.2) ; documentation de **toutes** les violations (33.5) ; information des personnes si risque élevé (34) | horloge des 72 h depuis la prise de connaissance ; registre de toutes les violations ; chaîne éditeur → établissements (§8.7) | DPO |
| AIPD | RGPD art. 35 ; liste CNIL [V, VERIF R22] | très probable (mineurs, données sensibles) | AIPD assistée, pré-remplie (§8.9) | DPO |
| Base légale | CNIL, outils collaboratifs (2026) [V-ext, VERIF R18] | mission d'intérêt public pour les établissements publics **et** privés pour leurs missions de service public | base par traitement dans le registre ; consentement réservé aux usages facultatifs (photo, publication) | DPO (hors contrat) |

### 4.2 Mineurs
| Point | Source | Règle | Effet |
|---|---|---|---|
| Exercice des droits par le mineur | Loi Informatique et libertés, art. 45 et CNIL, recommandations sur les droits des mineurs (2021) [V-ext, à relire] | le mineur peut exercer certains droits ; ses titulaires de l'autorité parentale les exercent en principe pour lui ; l'âge de 15 ans ne vaut que pour le consentement aux services en ligne (VERIF R19) | une demande peut venir de l'élève mineur (reçue, réponse adaptée, information des titulaires sauf motif) ou d'un titulaire ; jamais d'un parent pour les données propres de l'autre parent (§9.3) ; Q-RG3 au DPO |
| Information adaptée | RGPD art. 12.1 (« en des termes clairs et simples, en particulier pour toute information destinée spécifiquement à un enfant ») | mentions lisibles par un élève | page « tes données » par tranche d'âge (§8.11) |
| Élève majeur | VERIF R16 [NT] | aucun texte sur les droits restant aux parents | l'élève majeur exerce seul ses droits ; l'accès des parents suit les règles du socle 4 (Q10 du socle 2) |

### 4.3 Archives
| Point | Source | Règle | Effet | Avis |
|---|---|---|---|---|
| Archives publiques | C. patr. L211-4 [V, VERIF R39] | documents des établissements publics ; et documents « qui procèdent de la gestion d'un service public ou de l'exercice d'une mission de service public par des personnes de droit privé » | profil « public » : archives publiques ; profil « privé sous contrat » : **traité comme archives publiques** pour les catégories pédagogiques et de scolarité tant que le juriste n'a pas tranché (R-RG5) | **J** |
| Visa d'élimination | C. patr. L212-2, R212-14 [V, VERIF R40] | « toute élimination est interdite sans ce visa » de l'administration des archives | lot d'élimination bloqué sans référence de visa (R-RG6) | — |
| Durées des EPLE | instruction de tri, BO n° 24 du 16-6-2005 [V, VERIF R41] | registres matricules et listes d'élèves 50 ans (conservation) ; dossiers scolaires 10 ou 50 ans ; registres d'appel 10 ans (destruction) ; mots d'excuse, bulletins d'absence, certificats médicaux 1 an ; conseils de discipline 10 ans (tri) ; bourses 2 ans ; pièces comptables apurement + 1 an | valeurs proposées du référentiel (§8.2), **à valider** | DPO, archives |
| Communicabilité des archives | C. patr. L213-1, L213-2 [V-ext, §4.10] | communicables de plein droit sauf délais : 50 ans (vie privée, appréciation ou jugement de valeur), 25 ans après le décès (secret médical), 75 ans et **100 ans** pour certains dossiers juridictionnels se rapportant à un mineur | un dossier versé ou en conservation définitive n'est plus consultable que selon ces délais : accès restreint au service d'archives | J |
| Comptabilité (privé) | C. com. L123-22 [V, VERIF R46] | 10 ans | catégorie financière | EC |
| Sanctions | C. éduc. R511-13 IV [V, VERIF R34] | avertissement : fin d'année ; blâme et mesure de responsabilisation : fin de l'année suivante ; autres (sauf exclusion définitive) : fin de la deuxième année suivante ; toutes à la fin de la scolarité dans le second degré ; sur demande de l'élève qui change d'établissement | exécuteur du socle 13 branché sur le moteur (G6) | — |
| Journaux d'accès | CNIL, recommandation journalisation (délib. 2021-122) [V, VERIF R43] | 6 mois à 1 an ; jusqu'à 3 ans pour le contrôle interne si justifié | traces d'accès 1 an par défaut (S-541) | DPO |
| Personnels (droit privé) | CNIL, référentiel RH (2026) [V, VERIF R44] | durées par catégorie ; hors champ : agents publics | catégories du socle 6 | DPO |
| Bulletins de paie | C. trav. L3243-2, L3243-4, D3243-8 [V, VERIF R45] | double 5 ans ; bulletin électronique disponible 50 ans | socle 49 | — |
| Format de versement | SEDA 2.3 (27-6-2024), adossé à NF Z44-022 [V-ext, §4.10] | format d'échange des archives publiques | paquet de versement SEDA + PDF/A (§8.5) | archives |

### 4.4 Accès aux documents administratifs (public)
| Point | Source | Règle | Effet |
|---|---|---|---|
| CADA | CRPA L300-2, L311-6, R311-12, R311-13, R311-15 [V-ext, §4.10] | documents administratifs, y compris ceux des personnes de droit privé chargées d'une mission de service public ; vie privée, appréciation, jugement de valeur ou comportement : communicables au seul intéressé ; silence d'un mois = refus ; saisine de la CADA dans les deux mois | dans le public **et probablement le privé sous contrat** (L300-2), une demande de « dossier » peut relever de la CADA autant que du RGPD : le guichet qualifie la demande (RGP-141) et cite le bon fondement |

### 4.5 Ce qui reste non vérifié [NV]
Kit de conservation des ENT (introuvable, VERIF R42) ; durées pour la messagerie, le cahier de textes, les notes saisies hors bulletin ; statut exact des archives du privé sous contrat (R39) ; droits restant aux parents d'un élève majeur (R16) ; qualification d'un jugement de divorce au regard de l'art. 10 (socle 4) ; version la plus récente du tableau de gestion des archives des EPLE ; caractère obligatoire de SEDA pour un versement d'établissement.

### 4.9 Sources (consultées le 2026-11-11 sauf mention)
Reprises de `VERIF_REGLEMENTAIRE.md` (2026-11-09) : RGPD (copie CNIL) https://www.cnil.fr/fr/reglement-europeen-protection-donnees ; instruction de tri des EPLE https://www.education.gouv.fr/bo/BoAnnexes/2005/24/tableaux_encart24.pdf ; Code du patrimoine, Code de l'éducation, Code de commerce et Code du travail via les données ouvertes de la DILA (https://codes.droit.org/payloads/…) ; CNIL journalisation https://www.cnil.fr/fr/la-cnil-publie-une-recommandation-relative-aux-mesures-de-journalisation ; liste CNIL des AIPD https://www.cnil.fr/sites/default/files/atoms/files/liste-traitements-avec-aipd-requise-v2.pdf. Compléments du 2026-11-11 : §4.10.

---

## 5. Qui est responsable de quoi

| Traitement | Responsable de traitement | ScolIA | Conséquence dans le logiciel |
|---|---|---|---|
| Données scolaires, vie scolaire, familles, finances des familles, personnels **dans l'établissement** | **l'établissement** (chef d'établissement ; dans le privé, l'organisme gestionnaire ou le chef d'établissement selon la répartition écrite [H, Q-RG1]) | **sous-traitant** (art. 28) | registre de l'établissement (§8.1) ; demandes traitées par l'établissement ; ScolIA aide (28.3.e) |
| **Couche « compte personne »** commune (identité, passkeys, rattachements multiples, D-013) | **ScolIA** [R, `IDENTITE_ET_PROFILS_FAMILLES.md` §2] | responsable | registre ScolIA ; mentions ScolIA ; une demande sur le compte va au guichet de la plateforme (§9.6) |
| Exploitation de la plateforme (journaux techniques, sécurité, sauvegardes) | ScolIA pour ses propres besoins de sécurité ; pour le compte des établissements pour le reste [H] | les deux | registre de sous-traitant (30.2) + registre propre |
| Personnels du privé (contrat de travail avec l'OGEC) | l'OGEC (employeur) [H] | sous-traitant | traitement « RH » au nom de l'employeur, distinct de « scolarité » |
| Traitements hors ScolIA (Charlemagne pendant le doublon, papier, messagerie académique) | l'établissement | — | déclarés dans le même registre (NX27-03), sans données |

**Règle R-RG9** : le logiciel n'affiche jamais « ScolIA est conforme RGPD ». Il dit ce que ScolIA fait (mesures, sous-traitants, régions) et ce que l'établissement doit décider.

---

## 6. Concepts et vocabulaire [R]

| Terme | Sens dans ScolIA |
|---|---|
| **Catégorie de conservation** | classe de données qui partage une règle : « registre des élèves », « dossier scolaire », « justificatifs d'absence », « traces d'accès »… Identifiée par un **code stable** (`scol.dossier`, `vs.justificatif`, `acc.trace`…), déclarée par le socle propriétaire. |
| **Règle de conservation** | pour une catégorie et un établissement : fait générateur, durée d'utilité courante, durée de conservation intermédiaire, sort final, source, état de validation, version. |
| **Fait générateur** | l'événement daté qui fait courir la durée : fin de l'année scolaire, sortie de l'élève, clôture de la démarche, apurement des comptes, dernière connexion, fin du contrat. |
| **Objet racine** | l'objet auquel le sort s'applique en bloc : l'élève (et ses sous-objets d'une catégorie), une démarche d'inscription, une facture, un agent, une conversation, un compte. Le socle propriétaire sait le parcourir. |
| **Utilité courante** (base active) | l'objet sert ; visible selon les droits habituels. |
| **Conservation intermédiaire** | l'objet ne sert plus au quotidien mais doit être gardé (preuve, obligation) : visible du **seul** service habilité (`conservation.intermediaire.lire`), hors recherche, hors IA, hors exports courants. |
| **Sort final** | `destruction` · `anonymisation` (on garde des chiffres, plus personne) · `versement` (aux archives publiques) · `conservation_definitive` (dans ScolIA, archives historiques de l'établissement, accès restreint) · `tri` (une partie versée, le reste détruit, selon un critère écrit). |
| **Gel** | blocage daté et motivé de tout sort final sur un objet : contentieux, recours, signalement, réquisition, demande de limitation (art. 18). Revu à date. |
| **Lot d'élimination** | ensemble d'objets arrivés à échéance, proposé par le moteur, relu (échantillon), visé si nécessaire, exécuté par les socles, prouvé par un certificat. |
| **Bordereau** | document décrivant un lot (élimination ou versement), au modèle attendu par le service d'archives. |
| **Demande de droit** | demande d'une personne (accès, rectification, effacement, limitation, opposition, portabilité, information sur les consultations), avec son horloge. |
| **Extrait** | réponse à un droit d'accès : ensemble de sections fournies par les socles, relues, masquées si besoin, remises de façon sécurisée. |
| **Trace d'accès** | ligne « qui a consulté quoi, quand, pourquoi » ; distincte d'une **modification** (journal). |
| **Violation** | atteinte à la sécurité des données (perte, divulgation, accès non autorisé, altération, indisponibilité) ; tout incident qui en est une est consigné, notifié ou non. |
| **Fiche de traitement** | entrée du registre (art. 30) : finalité, base, personnes, données, destinataires, durées, transferts, mesures. |

---

## 7. Branchements : le « kit de conformité » de chaque socle

Le 27 ne connaît pas les données des autres socles ; il connaît ce qu'ils **déclarent**. Chaque socle ajoute à son kit (D-026), vérifié par la CI :

| Déclaration | Forme | Vérification |
|---|---|---|
| **Catégorie de chaque table** | `registre_donnees.rs` : `Retention::Categorie("code")` remplace `ADefinir` ; une table `Ephemere` reste éphémère ; une table sans donnée personnelle peut porter `Categorie("tech.parametrage")` | test : toute table « données personnelles » a une catégorie connue du catalogue (R-RG12) |
| **Catalogue des catégories** | constantes dans le code du socle (`ent-conformite` les rassemble) : code, libellé, objet racine, fait générateur, proposition de durée et de sort, **source** (texte, URL, [V]/[NV]), socle propriétaire | test : chaque catégorie a une source ou la mention explicite « aucune source, choix de minimisation » (R-RG13) |
| **Fournisseur d'extrait** | port `ExtraitPersonne` : pour une personne (élève, responsable, agent, compte) et une période, renvoie des **sections typées** (titre, lignes, pièces, destinataires habituels, sources, mentions de tiers à masquer) | test : chaque table « données personnelles » est couverte par au moins un fournisseur ou listée « hors extrait, raison » (R-RG14) |
| **Exécuteur de sort final** | port `SortFinal` : `simuler(objets)`, `executer(objets, sort, lot)` → compte rendu ; utilise les règles métier du socle (ex. 13 : R511-13 ; 24 : facture soldée et non contestée) | test : simulation sans écriture ; exécution idempotente ; refus sur objet gelé |
| **Traces de consultation** | appel `tracer_acces(tx, sujet, nature, motif)` **dans la transaction de lecture** pour les lectures déclarées sensibles | test : lecture refusée si la trace échoue (R-RG8) |
| **Finalités et destinataires** | pour chaque domaine : finalité, base légale proposée, actions D-017 qui lisent → destinataires calculés | registre généré (§8.1) |
| **Gel** | port `Gel` consulté par l'exécuteur ; le socle peut **poser** un gel (13 : recours ; 25 : créance contentieuse ; 36 : signalement) | test : un objet gelé n'est dans aucun lot |

**Ports exposés par le 27** : `conservation::regle(etab, categorie)`, `conservation::etat(objet)`, `conservation::geler(objet, motif, revue_le)`, `acces::tracer(...)`, `droits::demande_ouverte(personne)` (pour que les socles ne détruisent rien pendant une demande en cours), `violations::declarer(...)` (appelé par la supervision du 28 et par tout socle qui détecte une anomalie).

**Événements consommés** : `eleve.sortie` et `scolarite.closed` (socle 2) → fait générateur ; `annee.cloturee` (5) ; `inscription.demarche_close` (3) ; `facture.soldee`, `comptes.apures` (24, 25) ; `agent.depart` (6) ; `compte.inactif` (1) ; `discipline.sanction_prononcee` (13) ; `etablissement.contrat_termine` (28).

**Événements émis** : §16.

---

## 8. Les modules du socle

### 8.1 Registre des traitements (art. 30), généré
1. **Traitements de base** proposés par ScolIA, un par grand domaine (scolarité et dossier, inscriptions, vie scolaire et assiduité, discipline, évaluations et bulletins, communication, documents, finances des familles, restauration, internat, sorties, santé — désactivé tant qu'A7 —, personnels, accueil et sûreté, pilotage, IA, comptes et sécurité), chacun relié aux **domaines du registre des tables**.
2. Pour chaque fiche, le générateur **calcule** : catégories de personnes (élèves, responsables, personnels… depuis les tables), catégories de données (depuis les colonnes déclarées sensibles et les groupes de champs, ex. F0–F4 de `minimisation.rs`), **destinataires** (rôles de l'établissement qui détiennent les actions de lecture du domaine — la liste réelle, pas la théorie), destinataires externes (échanges du socle 9, sous-traitants du §8.8), **durées** (référentiel §8.2, avec l'état « non validée » s'il y a lieu), transferts hors UE (aucun pour ScolIA, sauf notifications Apple/Google sans donnée identifiante, VERIF R59), mesures de sécurité (texte de la plateforme).
3. L'établissement **complète** ce que le logiciel ne peut pas savoir : base légale retenue, responsable désigné, traitements hors ScolIA (NX27-03), commentaires du DPO ; puis le DPO **valide** une version.
4. **Écart détecté** : si une donnée change (nouveau rôle qui lit le dossier santé, nouveau sous-traitant, module activé), la fiche passe « à revoir » et le DPO est averti (R-RG10). C'est là que ScolIA dépasse le marché : un registre **qui ne ment pas**.
5. Export du registre (PDF et tableur) au format attendu par la CNIL [H : modèle de registre CNIL] ; versions figées (socle 7).
6. **Registre de sous-traitant de ScolIA** (art. 30.2), tenu par l'éditeur pour tous ses clients : catégories de traitements effectués pour chaque client, sous-traitants ultérieurs, transferts, mesures. Publié à chaque établissement (lecture).

### 8.2 Référentiel de conservation
1. **Catalogue ScolIA** (versionné, dans le code) : environ 70 catégories proposées au départ (§8.2 bis), chacune avec sa source et sa marque [V]/[NV].
2. **Adoption par l'établissement** : à la mise en service (socle 28), l'établissement reçoit le catalogue correspondant à son profil (public, privé sous contrat, hors contrat ; second degré) ; il peut **ajuster** une durée dans des bornes (jamais en dessous d'un minimum légal connu, jamais au-dessus d'un maximum légal connu, ex. traces d'accès ≤ 3 ans) avec un motif.
3. **Validation par le DPO**, catégorie par catégorie ou en bloc ; tant qu'une catégorie n'est pas validée : **aucun sort final** (on conserve), affichage « non validée » partout où la durée apparaît (registre, mentions, tableau de bord) (R-RG1).
4. Nouvelle version du catalogue ScolIA (texte nouveau, source trouvée) : proposée à l'établissement comme **changement**, jamais appliquée seule ; l'ancienne règle reste en vigueur jusqu'à décision.
5. Une règle porte : fait générateur, durée d'utilité courante, durée intermédiaire, sort final, critère de tri (si `tri`), visa exigé (calculé : public, ou privé sous contrat tant que Q-RG2 n'est pas tranchée), source, version, validée par / le.

### 8.2 bis Catalogue initial proposé (extrait ; **à valider par le DPO et le service d'archives**)

| Code | Catégorie | Fait générateur | Utilité courante | Intermédiaire | Sort final | Source |
|---|---|---|---|---|---|---|
| `scol.registre` | registre des élèves, listes d'élèves | fin d'année | 1 an | 50 ans | **conservation définitive / versement** | instruction de tri 2005 [V, R41] |
| `scol.dossier` | dossier scolaire (identité, scolarité, orientation, décisions) | sortie de l'élève | 1 an | 10 ans (récapitulatif tenu) ou 50 ans | tri | idem [V] |
| `scol.bulletin` | bulletins, relevés | fin d'année | scolarité | suit `scol.dossier` | tri | idem [NV pour l'ENT] |
| `vs.appel` | registres d'appel, présences | fin d'année | 1 an | 10 ans | destruction | idem [V] ; R131-5 |
| `vs.justificatif` | mots d'excuse, justificatifs, certificats d'absence | fin d'année | — | 1 an | destruction | idem [V] |
| `disc.sanction` | sanctions | prononcé | selon R511-13 IV | — | **effacement du dossier** (règle du socle 13) | R511-13 IV [V] |
| `disc.conseil` | dossier du conseil de discipline | décision | 1 an | 10 ans | tri (PV gardé) | instruction 2005 [V] |
| `disc.registre` | registre des sanctions | fin d'année | 1 an | 10 ans | conservation | instruction 2005 [V, §4.10] |
| `ins.demarche_non_aboutie` | dossiers d'admission refusés ou abandonnés | fin de campagne | — | délai de recours [NV] | destruction | minimisation |
| `ins.piece` | pièces justificatives d'une inscription aboutie | validation | démarche | délai de recours [NV] | destruction (sauf pièce durable) | minimisation |
| `adm.decision_justice` | décisions sur l'autorité parentale, protection | fin d'effet | tant qu'elle produit effet | fin de scolarité | destruction | [NV] |
| `adm.coordonnee_retiree` | coordonnées remplacées | remplacement | — | [R] 1 an | destruction | minimisation |
| `fin.piece` (privé) | factures, quittances, pièces comptables | clôture de l'exercice | 1 an | 10 ans | destruction | C. com. L123-22 [V] |
| `fin.piece` (public) | pièces justificatives | apurement des comptes | — | + 1 an | destruction | instruction 2005 [V] |
| `fin.bourse` | dossiers individuels de bourse | fin d'année | — | 2 ans | destruction | instruction 2005 [V] |
| `fin.iban` | coordonnées bancaires, mandats | fin du mandat | — | [NV] 13 mois après le dernier prélèvement | destruction | règles SEPA [NV], socle 25 |
| `msg.conversation` | messagerie | dernier message | année | [R] 1 an | destruction | [NV, VERIF R42] |
| `cdt.contenu` | cahier de textes | fin d'année | 2 ans | 5 ans | destruction (sauf archive pédagogique choisie) | SDET 2024, annexe opérationnelle [V, §4.10] |
| `eva.note` | notes hors bulletin | fin d'année | année | [R] 1 an | destruction (le bulletin fait foi) | [NV] |
| `acc.trace` | traces d'accès (consultations) | consultation | 1 an | — | destruction | CNIL 2021-122 [V] |
| `acc.trace_controle` | traces gardées pour un contrôle interne motivé | ouverture du contrôle | durée du contrôle | ≤ 3 ans | destruction | CNIL [V] |
| `jrn.modification` | journal des modifications | suit l'objet modifié | — | — | **suit le sort de l'objet** (anonymisé avec lui) | E7 |
| `cpt.compte_inactif` | comptes sans rattachement actif | dernière fin de rattachement | — | [R] 1 an | anonymisation du compte | minimisation (couche ScolIA) |
| `cpt.securite` | tentatives de connexion, verrous | événement | 6 mois [R] | — | destruction | CNIL [V] |
| `rh.dossier` (privé) | dossier du salarié | départ | — | référentiel CNIL RH | selon le référentiel | CNIL 2026 [V, R44] |
| `rh.candidat` | candidatures non retenues | dernier contact | — | 2 ans (vivier) / 5 ans (preuve) | destruction | CNIL [V] |
| `sante.*` | santé, PAI, passages | règles du socle 34 | — | — | — | socle 34 (désactivé, A7) |
| `rgpd.demande` | demandes de droits et réponses | clôture | — | [R] 5 ans (preuve) | destruction | prescription [NV] |
| `rgpd.violation` | registre des violations | clôture | — | [R] 5 ans | destruction | art. 33.5 [NV pour la durée] |
| `tech.parametrage` | paramétrage sans donnée personnelle | — | vie de l'établissement | — | suit le contrat | — |
| `export.archive` | archives d'export | génération | 7 jours | — | destruction | `PORTABILITE_EXPORTS.md` §8 |

### 8.3 Cycle de vie et moteur de conservation
1. **Calcul des échéances** (tâche nocturne, par établissement, sous `TenantTx`) : pour chaque objet racine de chaque catégorie validée, date d'échéance = fait générateur + durées ; résultat matérialisé dans `cons_echeance` (une ligne par objet et étape, **sans copier les données**).
2. **Passage en conservation intermédiaire** : automatique à l'échéance de l'utilité courante (pas de destruction, donc pas de lot) ; le socle propriétaire bascule l'objet en accès restreint ; réversible par le service habilité (ex. ancien élève qui revient, CL27-05).
3. **Sort final** : jamais automatique. Le moteur **propose** un lot par catégorie et par période (S-544 : deux fenêtres par an, juillet et janvier) ; le DPO ou la direction relit (§8.4).
4. **Exclusions automatiques du lot** : objet gelé ; personne avec une **demande de droit ouverte** ; créance ouverte (24–25) ; procédure disciplinaire en cours (13) ; signalement (36) ; élève **revenu** entre-temps ; donnée référencée par une archive définitive.
5. **Tableau de bord** : catégories non validées ; objets en intermédiaire par catégorie ; lots à relire ; gels à revoir ; échéances des 12 prochains mois ; dettes (tables sans catégorie, fournisseurs manquants).

### 8.4 Lots d'élimination, destruction, preuve
1. **Proposition** : liste des objets, volumes, catégorie, règle et version appliquées, échantillon tiré au sort (graine notée) que le relecteur ouvre en conservation intermédiaire.
2. **Relecture** par une personne détentrice de `conservation.lots.valider` ; la proposition est faite par le système, la validation par un humain ; **deux personnes** pour une catégorie de sort `destruction` portant sur des dossiers d'élèves (R-RG7, même mécanique que la double approbation des exports, `PORTABILITE_EXPORTS.md` §5.2).
3. **Visa** : si la règle l'exige, génération du **bordereau d'élimination** au modèle du service d'archives (PDF + tableur), envoi par l'établissement (hors ScolIA ou par le socle 16), saisie de la **référence et de la date du visa**, pièce du visa jointe ; sans visa, le lot ne s'exécute pas (R-RG6).
4. **Exécution** : appel des exécuteurs des socles propriétaires, dans une transaction par objet racine ; compte rendu par objet (`detruit`, `anonymise`, `refuse : raison`) ; fichiers supprimés au stockage (socle 7) ; **trace minimale** conservée : « 214 dossiers scolaires 2014-2015 détruits le 12-07-2027, lot n°, visa n° » — sans nom.
5. **Certificat de destruction** (PDF signé par le socle 7) ; le lot et son certificat sont conservés (`rgpd.lot`, [R] durée longue, ce sont des preuves).
6. **Anonymisation** : remplacement des identifiants directs par des valeurs neutres, suppression des textes libres, conservation des agrégats utiles (effectifs, taux) ; contrôle de non-réidentification sur les petits effectifs (moins de 5 personnes par case : agrégat supprimé, R-RG20).
7. **Destruction dans les sauvegardes** : effective à l'expiration de leur rétention (35 jours + 12 mensuelles, D-043), écrite dans les mentions et le contrat ; **après une restauration**, les sorts finaux exécutés après l'instant restauré sont **rejoués** depuis le registre des lots (R-RG22, comme le registre des effacements du socle 13, migration 2261) : une donnée détruite ne revient jamais.
8. **Destruction cryptographique** (plus tard, avec E11 et le chiffrement client des documents, PRA §18.1) : effacer la clé propre d'un fichier rend ses copies de sauvegarde illisibles.

### 8.5 Archives : conservation définitive, versement, tri
1. **Conservation définitive dans ScolIA** (registre des élèves, dossiers retenus au tri) — **seulement en attente de versement** pour les archives publiques, ScolIA n'étant pas un dépositaire agréé (C. patr. L212-4, §4.10, Q-RG11) ; objets en lecture seule, accès restreint au service d'archives de l'établissement, communicabilité selon les délais (§4.3), aucune modification, hors IA.
2. **Versement** aux archives publiques (public ; privé sous contrat si le juriste le confirme) : paquet au format d'échange des archives publiques [SEDA, version à confirmer] avec documents en PDF/A (socle 7) et métadonnées ; bordereau de versement ; accusé de réception saisi ; après accusé, élimination locale proposée (lot).
3. **Privé hors contrat ou privé sous contrat si le juriste écarte L211-4** : « archives historiques de l'établissement » gardées dans ScolIA ou exportées vers l'outil d'archivage de l'établissement.
4. **ScolIA n'est pas un système d'archivage électronique** au sens des normes [H : NF Z42-013] : il conserve des archives intermédiaires et prépare les versements ; la conservation définitive de long terme dans ScolIA est un service rendu, sans valeur probante renforcée — dit clairement aux établissements (R-RG23).

### 8.6 Fin d'année, sortie d'un élève, départ d'un personnel
1. **Fin d'année** (événement `annee.cloturee`, socle 5) : fait générateur des catégories annuelles ; l'année close passe en lecture seule (existant [F]) ; le 27 calcule les échéances.
2. **Sortie d'un élève** (`eleve.sortie`) : le socle 2 clôt ; le 7 propose la transmission du dossier (13.4 du 07) ; le 27 lance les faits générateurs, **demande aux socles** de couper les diffusions futures (comptes famille : plus de nouvelles notifications ; compte élève : désactivé à la date fixée, S-545), et prévient la famille de la date à laquelle son accès se ferme et de ce qu'elle peut télécharger avant (coffre du socle 7).
3. **Élève qui part et demande l'effacement de ses sanctions** (R511-13 IV) : action dédiée, exécutée par le 13 (RGP-170).
4. **Départ d'un personnel** (`agent.depart`) : accès fermés (socle 6), dossier RH en intermédiaire, traces d'accès de ses consultations gardées pour leur durée propre (elles concernent les élèves consultés).
5. **Compte devenu inactif** (couche ScolIA) : plus aucun rattachement actif depuis la durée `cpt.compte_inactif` → compte anonymisé (passkeys supprimées) ; la personne garde le droit de demander ses données à chaque établissement.

### 8.7 Violations de données
1. **Déclaration** par toute personne habilitée (`violations.declarer`, large : un professeur qui a perdu une clé USB doit pouvoir le dire en trente secondes, sans être accusé) ou par le système (supervision du 28 : fuite détectée par la fermeture des références, export anormal, alerte d'intrusion ; socle 1 : comptes compromis).
2. **Horloge** : la prise de connaissance est l'heure de la déclaration ; échéance CNIL = +72 h, affichée en compte à rebours, **jamais bloquante** (on notifie quand on peut, avec motif du retard).
3. **Qualification** guidée : nature (confidentialité, intégrité, disponibilité), catégories de données (liste des catégories du référentiel), catégories et **nombre** de personnes (calculé quand l'origine est connue : un export, une liste de diffusion), conséquences probables, mesures prises et envisagées ; **risque** (faible, avéré, élevé) proposé par une grille déterministe, **décidé** par la direction après avis du DPO.
4. **Notification CNIL** : brouillon pré-rempli selon le contenu de l'art. 33.3 ; la notification se fait sur le téléservice de la CNIL par l'établissement [H : ScolIA ne la transmet pas] ; référence et date saisies ; notifications complémentaires.
5. **Information des personnes** (risque élevé, art. 34) : message rédigé en langage clair, envoyé par le socle 8 **après confirmation humaine** (N3), destinataires calculés, mineurs : titulaires de l'autorité parentale et élève selon l'âge [Q-RG3].
6. **Chaîne éditeur → établissements** : une violation de la plateforme (ScolIA sous-traitant) est déclarée par l'éditeur **une fois**, et apparaît dans le registre de **chaque établissement concerné** avec les seuls éléments le concernant ; horloge « le sous-traitant notifie dans les meilleurs délais » (33.2) suivie ; l'établissement décide ensuite de sa propre notification.
7. **Registre** : toutes les violations, notifiées ou non, avec la justification de la non-notification (33.5) ; fiches PDF (NX27-18).
8. **Retour d'expérience** : actions correctives liées (tâche à faire, socle 29) ; revue à 30 jours.

### 8.8 Sous-traitants et transparence de la plateforme
1. **Liste de la plateforme** tenue par l'éditeur : fournisseur, service, données concernées, région, garanties, accord de traitement (date, version), date d'entrée ; aujourd'hui [F/H] : Scaleway (hébergement, base, stockage, région France), OVHcloud (SMS, D-004 ; sauvegarde B, D-040 proposée), fournisseur de courriel (à choisir, `CLAUDE.md`), Apple et Google (notifications mobiles, sans donnée identifiante), fournisseur d'IA (Mistral, à étudier, `CLAUDE.md` ; aucune donnée personnelle tant que les garanties ne sont pas réunies).
2. **Changement** d'un sous-traitant ultérieur : annoncé à chaque établissement **avant** application, avec délai d'objection [R] 30 jours (art. 28.2) ; journalisé ; la fiche du registre passe « à revoir ».
3. **Sous-traitants propres à l'établissement** (Microsoft 365, outil d'orientation…) : déclaratifs, sans connexion.
4. **Dossier de conformité de la plateforme** (publié, versionné) : mesures de sécurité (`SECURITY.md`, `PRA_SAUVEGARDES.md` résumé), régions, sous-traitants, procédure de violation, procédure d'aide aux demandes, réversibilité ; répond d'avance aux questionnaires des DPO.

### 8.9 AIPD, documents, mentions
1. **AIPD assistée** : critères de la liste CNIL cochés à partir des faits (mineurs, données sensibles si santé activée, croisement, IA) ; description des traitements **reprise du registre** ; risques et mesures pré-remplis depuis le dossier de la plateforme ; l'établissement et le DPO complètent, valident, signent (socle 7). Révision au changement (module activé, nouvelle IA).
2. **Documents** (NX27-10) : catalogue versionné ; chaque document a un état (manquant, brouillon, généré, importé, validé, sans objet + raison) ; ceux qui décrivent des **faits** (registre, sous-traitants, conservation, mesures) sont **générés** et se périment quand les faits changent ; ceux qui décrivent des **choix** (charte informatique, charte photo) sont des modèles à adapter.
3. **Points ouverts** (au lieu du score, RG-D15) : « 12 catégories non validées », « AIPD non signée », « mentions famille non publiées », « 2 demandes en retard » — classés par gravité, chacun avec son bouton.

### 8.10 Réversibilité et fin de contrat
Le mécanisme (extraction, double approbation, archive chiffrée, rétention 7 jours) est celui de `PORTABILITE_EXPORTS.md` (lots EXP-0 à EXP-6) ; le 27 en porte la **partie contractuelle et de preuve** : au terme du contrat (événement `etablissement.contrat_termine`, socle 28), (1) export final proposé, vérifié par l'établissement (manifeste), (2) suppression des données de l'établissement dans la production à la date convenue, (3) **attestation de suppression** générée, (4) suppression dans les sauvegardes à l'expiration de leur rétention, mentionnée, (5) **réserve des archives** : les catégories en conservation définitive ou soumises à visa ne sont pas détruites sans le visa ou sans leur remise à l'établissement (R-RG24).

### 8.11 Information des personnes
1. **Mentions** générées à partir du registre, par public : familles, élèves (deux versions : collège et lycée, langage adapté, art. 12.1), personnels, visiteurs (accueil, socle 14), candidats ; versionnées, publiées dans l'ENT (page « Vos données » toujours accessible, y compris sans connexion pour la version publique), liées depuis chaque formulaire de collecte (socle 18).
2. **Page « Tes données / Vos données »** : ce que l'établissement sait, pourquoi, combien de temps, qui le voit (rôles), les consultations (§11), les demandes possibles (bouton), le contact du DPO.
3. Une nouvelle version substantielle (nouvelle finalité, nouveau destinataire) est **annoncée** (socle 17) ; pas de « j'ai lu » obligatoire (la base n'est pas le consentement).

### 8.12 Sauvegardes, restauration et droits
- Une restauration d'établissement (PRA §11) rejoue les sorts finaux et les effacements postérieurs à l'instant cible (R-RG22) **et** les rectifications faites sur demande de droit (R-RG25) ; le rapport de restauration les liste.
- Une demande d'effacement accordée ne déclenche pas de réécriture des sauvegardes ; la mention le dit (D-043).
- Le bac de restauration (PRA §11.2) est un traitement : il apparaît au registre de sous-traitant, ses accès sont tracés.

---

## 9. Droits des personnes : le guichet

### 9.1 Entrées
Formulaire en ligne (connecté ou non : un ancien élève, un parent sans compte), saisie par le secrétariat d'une demande reçue par courrier, courriel ou au guichet (avec la pièce), demande transmise par le DPO ou la CNIL. Chaque demande a : demandeur, **personne concernée** (souvent différente : parent pour l'enfant), qualité du demandeur (lui-même, titulaire de l'autorité parentale, représentant légal d'un majeur protégé, mandataire avec mandat), droit(s) demandé(s), périmètre (tout, une période, un domaine), canal de réponse souhaité, date de réception (**fait courir le délai**).

### 9.2 Identité et qualité
- Demandeur connecté par passkey ou session forte sur son compte : identité acquise ; qualité vérifiée par les liens du socle 4 **à la date de la demande** (titulaire, résidence, restrictions).
- Demandeur non connecté : vérification proportionnée (courriel connu au dossier avec code, ou pièce d'identité **consultée puis détruite**, jamais gardée au-delà de la clôture, R-RG16) ; le délai court dès la réception, la vérification ne le suspend pas [H, à confirmer DPO].
- Mandataire : mandat joint.
- Échec : réponse « nous ne pouvons pas vous identifier » motivée.

### 9.3 Qui peut demander quoi [R]
| Demandeur | Données de l'enfant mineur | Données propres du demandeur | Données de l'autre parent |
|---|---|---|---|
| Titulaire de l'autorité parentale | oui (à la date de la demande) | oui | **jamais** (masquées, R-RG15) |
| Parent sans exercice de l'autorité (C. civ. 373-2-1) | les informations qu'il doit recevoir (résultats, absences, sanctions, orientation) ; pas les données de l'autre foyer [H, J] | oui | jamais |
| Parent visé par une restriction (décision de justice, adresse protégée) | selon la décision (socle 4) | oui | jamais ; l'adresse protégée n'apparaît dans **aucun** extrait |
| Élève mineur | oui, pour lui-même ; réponse rédigée pour lui ; titulaires informés sauf motif (Q-RG3) | — | — |
| Élève majeur | oui, seul | — | — |
| Ancien élève | oui (y compris données en conservation intermédiaire) | — | — |
| Personnel | son dossier et ses traces d'activité (§11.4) | — | — |
| Famille d'accueil, service de l'ASE | selon les attributs délégués (C. civ. 375-7, socle 4) | — | — |

### 9.4 Traitement
1. **Ouverture** : accusé de réception (date, échéance, contact) ; **gel automatique** des sorts finaux sur la personne pendant la demande (port `droits::demande_ouverte`).
2. **Qualification** : RGPD ou CADA (public) ou simple demande de document (certificat de scolarité, duplicata de bulletin → socles 7 et 23, pas une demande de droit) ; la qualification est modifiable et motivée.
3. **Assemblage de l'extrait** : appel des fournisseurs de chaque socle, sur la période ; sections typées ; liste des **destinataires** (rôles internes, destinataires externes, sous-traitants) ; **durées** ; **source** des données ; **consultations** (§11) ; pièces (coffre du socle 7) ; marquage automatique de ce qui concerne un **tiers** (autre parent, autre élève dans une sanction collective, rédacteur d'une note interne) et de ce qui est **exclu par la loi ou à arbitrer** (note de signalement en cours d'instruction, socle 36 ; santé si le module est actif, socle 34).
4. **Relecture** par le gestionnaire : chaque marquage est accepté (masqué) ou levé avec motif ; rien ne part sans relecture (R-RG15).
5. **Remise** : dossier PDF lisible + annexe tableur (CSV) réutilisable + pièces, dans une archive chiffrée (même mécanique que l'export, sans double approbation : une personne habilitée + passkey récente), lien nominatif court, téléchargement journalisé ; ou remise papier au guichet contre émargement.
6. **Clôture** : date, réponse, éléments remis (empreintes, pas les données), refus partiels motivés ; conservation de la demande (`rgpd.demande`).

### 9.5 Autres droits
- **Rectification** : renvoi vers les propositions des socles 2 et 4 (identité, coordonnées), ou correction par le socle propriétaire ; une donnée de source externe (SIECLE) donne « à corriger dans la source » et une note (socle 9) ; la réponse liste ce qui a été corrigé.
- **Effacement** : examen par catégorie ; accordé quand rien ne s'y oppose (ex. photo publiée sur consentement retiré, pièce surnuméraire, compte d'un candidat non admis) ; refusé avec motif article par article (obligation légale, mission, archives) ; partiel souvent.
- **Limitation** : statut posé sur l'objet contesté (RGP-160).
- **Opposition** : examen motivé ; pour un traitement facultatif (annuaire des anciens, photo de classe publiée), arrêt immédiat.
- **Consentements** : retrait par le socle 18, qui prévient le 27 (traçabilité).
- **Information sur les consultations** : demande spécifique « qui a consulté le dossier de mon enfant ? » — servie par la page du §11 ; au-delà (nom d'un agent), arbitrage motivé (CJUE C-579/21).

### 9.6 Demandes qui concernent la couche ScolIA
Compte, passkeys, rattachements à plusieurs établissements : guichet de la plateforme (éditeur responsable de traitement) ; une demande qui touche plusieurs établissements est **éclatée** en une demande par établissement, chacun répond pour ce qu'il détient ; le demandeur suit l'ensemble.

### 9.7 Ce qui ravit
Le parent voit l'avancement (« reçue le 3, échéance le 3 du mois suivant, en cours d'assemblage ») ; le secrétariat ne cherche rien à la main ; le DPO voit toutes les demandes en retard de tous ses établissements, **sans voir leur contenu**.

---

## 10. Intelligence artificielle (contrat IA, `IA_COEUR.md` §5, §6.2)

### 10.1 Ce que l'IA fait [R]
- **Rédige** : brouillon de réponse à une demande de droit (à partir de l'extrait déjà relu), mentions d'information à partir du registre, bordereau d'élimination, notification CNIL à partir des champs de l'incident, message aux personnes en cas de violation.
- **Explique** : « pourquoi garde-t-on ce dossier ? » (règle, source, échéance) ; « qui peut voir cette donnée ? » (registre des droits réels) — lecture, sous les droits de l'utilisateur.
- **Repère** : textes libres qui ressemblent à une donnée de santé ou à un tiers dans un extrait (suggestion de masquage, jamais masquage silencieux) ; incohérences du registre (« le rôle Accueil lit le domaine santé ») ; documents importés incomplets au regard de l'art. 30 (NX27-14).
- **Qualifie** un incident par conversation (NX27-19) : questions guidées pour remplir les champs.

### 10.2 Ce que l'IA ne fait pas [R]
Elle ne **décide** jamais : ni le refus d'un droit, ni l'acceptation d'un effacement, ni la notification ou non d'une violation, ni le niveau de risque final, ni la validation d'une durée, ni un lot d'élimination. Elle ne lit **jamais** les données de santé (règle constante), ni le contenu des extraits d'une autre personne que celle de la demande en cours. Elle n'envoie rien.

### 10.3 Données envoyées à un modèle
- **Aucune donnée personnelle** vers un fournisseur externe tant que ses garanties ne sont pas validées (`CLAUDE.md`) : la conversation d'incident est **pseudonymisée** côté serveur avant envoi (noms, adresses, numéros remplacés par des jetons, re-substitués à l'affichage) et l'historique est tenu côté serveur (RG-D14) ; un avertissement rappelle de ne pas y coller de données.
- Les documents analysés (NX27-14) sont des modèles et procédures, contrôlés par le même filtre.
- Sans modèle disponible : toutes les fonctions marchent sans IA (modèles de textes, grilles déterministes).

### 10.4 Capacités déclarées (registre `ent-ia`)
| Capacité | Type | Droit | Niveau |
|---|---|---|---|
| `conformite.points_ouverts` | lecture | `conformite.lire` | N0 |
| `conservation.pourquoi(objet)` | lecture | droit de lecture de l'objet | N0 |
| `droits.etat_demande(id)` | lecture | demandeur ou gestionnaire | N0 |
| `droits.brouillon_reponse(id)` | proposition | `droits.traiter` | N1 (relu) |
| `violations.brouillon_notification(id)` | proposition | `violations.gerer` | N1 |
| `violations.qualifier(id)` | conversation | `violations.declarer` | N1 |
| `registre.incoherences` | lecture | `conformite.lire` | N0 |
| aucune commande d'exécution de sort, de remise d'extrait ni de notification | — | — | — |

---

## 11. Transparence des consultations (journal des accès)

### 11.1 Ce qui est tracé
- **Toujours** : consultation d'une fiche ou d'une section hors du périmètre habituel de l'acteur (hors de ses classes, de ses groupes, de son service) — avec **motif** choisi dans une liste courte (S-542) ; consultation des données protégées (F4), de la santé (34), du suivi psy et social (36), des décisions de justice ; export, impression, téléchargement d'un dossier ; consultation par le support (socle 28) ; consultation en conservation intermédiaire ou d'archives.
- **Non tracé individuellement** (bruit, et ce serait surveiller le travail ordinaire) : le professeur qui ouvre la liste de sa classe, l'appel, le cahier de textes de son groupe ; ces gestes sont des usages prévus, déjà décrits par le registre (destinataires).

### 11.2 Ce que voit la famille (et l'élève majeur)
Page « Consultations du dossier » : date, **fonction et service** de la personne (« Conseiller principal d'éducation — vie scolaire », « Secrétariat »), nature (consultation, export, impression), section (« coordonnées », « absences »), motif quand il a été demandé ; regroupée par jour ; sur l'année en cours et la précédente (durée des traces). **Jamais** : le nom de l'agent (§11.3), les consultations liées à un signalement ou à une enquête en cours (socle 36, R-RG28 : différées jusqu'à la clôture ou masquées sur décision motivée tracée), les consultations de santé (vues dans le module santé, règles du 34), les accès du support d'éditeur **sans** mention (ils apparaissent comme « Support ScolIA, intervention autorisée par l'établissement le … »).

### 11.3 Nom ou fonction ? [R, appliqué par défaut, Q-RG4]
Fonction et service par défaut : c'est ce qu'exige le droit d'accès selon l'arrêt C-579/21 [V-ext], et cela protège les personnels (un nom exposé à un parent en conflit). Le nom est communiqué sur **demande motivée** traitée comme une demande de droit, arbitrée par la direction (« indispensable à l'exercice de ses droits » : soupçon d'accès illégitime).

### 11.4 Ce que voit un personnel
Son propre dossier RH et les consultations de celui-ci (même règle). Le journal des accès **n'est pas** un outil d'évaluation du travail : aucun tableau « qui consulte le plus », aucune statistique individuelle, accès de la direction aux traces d'un agent seulement dans le cadre d'un **contrôle interne ouvert**, motivé, daté, tracé lui-même (catégorie `acc.trace_controle`, R-RG27).

### 11.5 Alertes
Consultations en rafale hors périmètre (ex. > 30 dossiers en 10 min par une même personne, S-543), consultation d'un élève à adresse protégée par un compte sans lien, consultation d'archives : alerte à la direction et au DPO (pas au personnel concerné : il pourrait s'agir d'un compte volé). Règle déterministe, jamais l'IA.

### 11.6 Technique
Table de traces **séparée** du journal des modifications (RG-D09), en ajout seul, **partitionnée par mois** pour que la purge à un an soit une suppression de partition **par procédure** (journalisée dans `cons_execution`), jamais un `DELETE` applicatif ; écrite **dans la transaction de lecture** pour les natures sensibles (R-RG8) ; colonnes typées, sans JSON ; la fonction de l'acteur est **figée** au moment de la consultation (un agent qui change de poste n'efface pas son ancienne fonction).

---

## 12. Droits et minimisation

### 12.1 Catalogue d'actions proposé (D-017) [R]
| Action | Libellé | Portée | Modèles par défaut |
|---|---|---|---|
| `conformite.lire` | voir le tableau de bord de conformité, le registre, les points ouverts | établissement | Direction, DPO |
| `conformite.gerer` | compléter le registre, les documents, l'AIPD | établissement | Direction, DPO |
| `conformite.valider` ★ | valider une version du registre, de l'AIPD, des mentions | établissement | Direction |
| `conservation.regles.proposer` | ajuster une règle (dans les bornes) | établissement | Direction, DPO |
| `conservation.regles.valider` ★ | valider une règle | établissement | **DPO** (rôle « DPO » créé à la mise en service, personne interne ou externe) |
| `conservation.intermediaire.lire` | consulter les objets en conservation intermédiaire | établissement, domaine | Direction, Secrétariat (scolarité), Comptabilité (finances) : **par domaine** |
| `conservation.lots.proposer` | lancer une proposition de lot | établissement | Direction, DPO |
| `conservation.lots.valider` ★ | relire et valider un lot (deux personnes pour les dossiers d'élèves) | établissement | Direction, DPO |
| `conservation.visa.saisir` | enregistrer le visa | établissement | Direction, Secrétariat |
| `conservation.geler` | poser ou lever un gel | établissement, domaine | Direction ; socles 13, 25, 36 pour leurs objets |
| `archives.lire` | consulter les archives définitives | établissement | Direction (+ archiviste désigné) |
| `archives.verser` ★ | préparer et enregistrer un versement | établissement | Direction |
| `droits.recevoir` | enregistrer une demande | établissement | Secrétariat, Direction, DPO |
| `droits.traiter` | assembler, relire, répondre | établissement | Secrétariat (scolarité), Direction, DPO |
| `droits.remettre` ★ | remettre l'extrait | établissement | Direction, Secrétariat habilité |
| `droits.arbitrer` | refus, noms des agents, demandes excessives | établissement | Direction |
| `violations.declarer` | déclarer un incident | établissement | **tous les personnels** |
| `violations.gerer` | qualifier, notifier, clore | établissement | Direction, DPO, référent sécurité |
| `acces.consulter_traces` | voir les traces d'un sujet (pour une demande ou un contrôle ouvert) | établissement | Direction, DPO |
| `acces.controle_ouvrir` ★ | ouvrir un contrôle interne sur un agent | établissement | Direction |
| `soustraitants.gerer` | déclarer les sous-traitants propres à l'établissement | établissement | Direction |
| `plateforme.conformite` (éditeur) | tenir la liste des sous-traitants, le dossier de la plateforme, déclarer une violation de plateforme | plateforme | opérateurs ScolIA nommés |
Sensibles (★) : passkey récente (< 5 min, D-041), octroi journalisé et notifié aux détenteurs ; non auto-attribuables.

### 12.2 Le rôle DPO
Le DPO est souvent **extérieur** (mutualisé) [H] : compte rattaché à l'établissement avec le seul rôle « DPO », qui voit **registres, règles, lots, demandes (métadonnées, échéances), violations**, et **pas** les dossiers des élèves, sauf l'échantillon d'un lot qu'il relit et l'extrait d'une demande qu'on lui soumet (traces). Un DPO de plusieurs établissements voit un **tableau consolidé par le guichet central** (même mécanisme que la réunion multi-établissements du socle 29, R-PO34), sans données nominatives.

### 12.3 Minimisation
Les écrans du 27 ne montrent jamais une donnée métier hors de la demande ou du lot qui la justifie ; les tableaux de bord sont des **compteurs** ; l'extrait n'est visible que des gestionnaires de la demande ; un lot n'expose que son échantillon.

---

## 13. Cas limites (CL27-01 à CL27-62)

### 13.1 Familles et demandes
| N° | Cas | Réponse |
|---|---|---|
| CL27-01 | Parent séparé demande « tout le dossier » de l'enfant | extrait de l'enfant, **sans** les données propres de l'autre parent (coordonnées, échanges, situation financière) ; masquages relus |
| CL27-02 | Parent visé par une ordonnance de protection demande l'extrait | adresse protégée et tout indice de localisation (établissement d'origine de la fratrie, transport scolaire) **exclus** ; arbitrage direction |
| CL27-03 | Parent sans exercice de l'autorité parentale | informations que la loi lui garantit ; pas les décisions réservées aux titulaires [J] |
| CL27-04 | Élève de 14 ans demande lui-même ses données et ne veut pas que ses parents le sachent | demande reçue ; réponse adaptée ; information des titulaires **sauf** motif sérieux (sécurité), décision tracée ; Q-RG3 |
| CL27-05 | Ancien élève revient au lycée trois ans après | objets en intermédiaire **réactivés** (rattachés à la nouvelle scolarisation), échéances recalculées |
| CL27-06 | Élève décédé ; les parents demandent le dossier | titulaires de l'autorité au décès ; archives ; douceur des écrans (socle 2) |
| CL27-07 | Demande par un avocat | mandat exigé ; sinon réponse au mandant |
| CL27-08 | Demande d'effacement de la photo de classe publiée | consentement retiré (18) → retrait des publications futures et des pages de l'ENT ; impressions déjà distribuées hors champ |
| CL27-09 | Demande d'effacement d'une sanction | R511-13 IV : effacement réglementaire si la condition est remplie (changement d'établissement, fin de scolarité) ; sinon refus motivé |
| CL27-10 | Demande d'effacement de tout le dossier à la sortie | refus motivé (obligation légale, archives) ; effacement des éléments facultatifs ; échéancier communiqué |
| CL27-11 | Même parent dépose cinq demandes identiques en un mois | regroupées ; art. 12.5 seulement sur décision motivée de la direction |
| CL27-12 | Demande pendant les vacances d'été | horloge inchangée (le RGPD ne suspend pas) ; alerte J-10 à la direction et au DPO ; prolongation motivée si complexe |
| CL27-13 | Demande qui touche trois établissements (fratrie, déménagement) | éclatée par établissement (§9.6) |
| CL27-14 | Famille d'accueil demande le dossier | selon les attributs délégués ; sinon, l'ASE |
| CL27-15 | Majeur protégé (tutelle) | représentant légal avec pièce |
| CL27-16 | Parent demande « qui a consulté » avec le nom d'un CPE | fonction donnée ; nom sur arbitrage (§11.3) |
| CL27-17 | Données issues de SIECLE erronées | rectification « à corriger dans la source », note au dossier, suivi |
| CL27-18 | Demande de copie des messages échangés avec l'autre parent par l'ENT | messages du demandeur ; messages de l'autre parent **à lui** sont aussi à l'autre : arbitrage, masquage des passages relevant de l'autre [J] |
| CL27-19 | Note interne d'un professeur sur l'élève (observation) | incluse si elle est au dossier ; pas de « notes personnelles » cachées dans ScolIA (il n'y en a pas : tout ce qui est stocké est au dossier) |
| CL27-20 | Signalement en cours d'instruction (36) | exclu de l'extrait jusqu'à la clôture sur décision motivée (risque pour l'enfant) [J] |

### 13.2 Conservation et archives
| N° | Cas | Réponse |
|---|---|---|
| CL27-21 | Catégorie non validée depuis deux ans | rien n'est détruit ; point ouvert « grave » ; rappel au DPO à chaque fenêtre de lot |
| CL27-22 | Facture contestée arrivée à échéance | gel automatique (créance ouverte, 25) |
| CL27-23 | Dossier d'un élève exclu définitivement | sanction non effaçable (R511-13 IV) ; dossier de conseil 10 ans |
| CL27-24 | Recours contentieux d'une famille sur une orientation | gel de tout le dossier de l'élève jusqu'à la décision + délai |
| CL27-25 | Réquisition judiciaire | gel ; procédure hors plateforme ; traces de l'extraction ; aucune remise sans la réquisition jointe |
| CL27-26 | Établissement public sans réponse du service d'archives depuis six mois | lot en attente ; rien ne s'exécute ; rappel ; pas de contournement |
| CL27-27 | Privé sous contrat : le juriste confirme L211-4 2° pour les dossiers scolaires | visa exigé, versements possibles ; règles mises à jour par version |
| CL27-28 | Privé sous contrat : le juriste l'écarte | visa non exigé ; décision écrite du chef d'établissement par lot à la place |
| CL27-29 | Ancien élève demande un duplicata de bulletin de 2015, dossier en conservation intermédiaire | délivré par le service habilité (socle 23) ; trace |
| CL27-30 | Lot exécuté puis restauration PITR à une date antérieure | rejeu des sorts finaux (R-RG22) avant réouverture |
| CL27-31 | Une table ajoutée par un socle sans catégorie | CI rouge (R-RG12) |
| CL27-32 | Un fournisseur d'extrait plante pendant l'assemblage | extrait **incomplet signalé**, jamais remis comme complet (R-RG18) |
| CL27-33 | Anonymisation d'une classe de 4 élèves | agrégats supprimés sous le seuil (R-RG20) |
| CL27-34 | Fusion de deux fiches d'élève (socle 2) | échéances recalculées sur la fiche gardée ; traces suivent |
| CL27-35 | Changement de durée validée à la baisse | appliqué aux lots **futurs**, après nouvelle validation ; pas d'effet rétroactif silencieux |
| CL27-36 | Changement de durée à la hausse après une destruction | sans effet sur le détruit ; signalé |
| CL27-37 | Journal des modifications d'un objet anonymisé | anonymisé avec lui (les instantanés `payload` aussi : l'exécuteur les traite) [R] ; le fait « modifié le, par fonction » demeure |
| CL27-38 | Objet partagé entre deux élèves (sanction collective, photo de groupe) | sort à l'échéance la plus tardive ; extrait de chacun avec masquage de l'autre |
| CL27-39 | Fin de contrat avec ScolIA, archives définitives dans ScolIA | remise à l'établissement (export) avant suppression ; jamais de destruction sans visa (R-RG24) |
| CL27-40 | Doublon 2027-2028 : la donnée officielle est dans Charlemagne | ScolIA applique ses propres règles à ses copies ; le registre déclare les deux traitements |

### 13.3 Violations, accès, sécurité
| N° | Cas | Réponse |
|---|---|---|
| CL27-41 | Liste de classe envoyée par erreur à toutes les familles d'un niveau | déclaration ; personnes calculées depuis l'envoi (socle 8) ; risque proposé « faible » (nom + classe) ; direction décide |
| CL27-42 | Ordinateur volé avec session ouverte | révocation des sessions (socle 1) ; traces de l'appareil ; violation déclarée |
| CL27-43 | Export complet téléchargé depuis un pays inhabituel | alerte (PORTABILITE §8) → déclaration proposée |
| CL27-44 | Violation chez un sous-traitant de la plateforme | déclarée par l'éditeur, propagée aux établissements concernés |
| CL27-45 | Le déclarant d'une violation en est l'auteur | déclaration valorisée, sans sanction automatique ; libellé neutre |
| CL27-46 | 72 h dépassées | notification quand même, motif du retard saisi (art. 33.1) |
| CL27-47 | Violation touchant des données de santé (module actif) | risque élevé par défaut ; DPO obligatoire |
| CL27-48 | Agent qui consulte 50 dossiers à 23 h | alerte (§11.5) |
| CL27-49 | Agent soupçonné ; la direction veut ses traces | contrôle interne ouvert, motivé, tracé (R-RG27) |
| CL27-50 | Compte support de l'éditeur consulte un dossier | visible de la famille comme « Support ScolIA » avec la référence de l'autorisation |
| CL27-51 | Un parent consulte le journal et y voit un accès « infirmerie » | consultations santé non montrées dans ce journal (§11.2) ; elles relèvent du 34 |
| CL27-52 | Échec de l'écriture d'une trace sensible | lecture refusée, message neutre, alerte technique (R-RG8) |
| CL27-53 | Traces arrivées à un an alors qu'une demande les concerne | gel des traces de la personne pendant la demande |
| CL27-54 | Personnel parti demande ses traces d'activité | oui, ses propres actions ; pas les données des élèves consultés |

### 13.4 Plateforme, registre, multi-établissements
| N° | Cas | Réponse |
|---|---|---|
| CL27-55 | Nouveau sous-traitant (fournisseur de courriel) | annonce 30 jours avant, objection possible ; fiche « à revoir » |
| CL27-56 | Activation du module santé (HDS obtenu) | AIPD à réviser, registre à revoir, catégories `sante.*` à valider ; activation bloquée tant que ce n'est pas fait (R-RG26) |
| CL27-57 | Un rôle personnalisé reçoit la lecture du domaine discipline | fiche discipline « à revoir » (destinataires changés) |
| CL27-58 | DPO de 12 établissements | tableau consolidé sans données nominatives (§12.2) |
| CL27-59 | Personne présente dans deux établissements | demandes éclatées ; traces jamais croisées entre établissements |
| CL27-60 | Établissement sans DPO désigné | point ouvert « bloquant pour les données réelles » ; le chef d'établissement valide à défaut, mention au registre [DPO, Q-RG5] |
| CL27-61 | Établissement qui ferme (fusion, fermeture) | archives remises à l'établissement repreneur ou au service d'archives ; procédure de fin de contrat |
| CL27-62 | Démo et données fictives | établissement de démonstration hors registre réel, marqué ; jamais de donnée réelle (socle 28) |

---

## 14. Actions métier (RGP-100 à RGP-299)

Acteurs : DIR direction, DPO, ADF secrétariat, CPT comptabilité, ARC archiviste désigné, PER tout personnel, PAR responsable, ELV élève (ELV-M majeur), ANC ancien élève, EDI opérateur de la plateforme, SYS système. Confirmation N1–N3 (`IA_COEUR.md` §6.2). ★ sensible (passkey récente). « (NX27-xx) » = reprise de Next.js.

### 14.1 Gouvernance et registre (RGP-100 à RGP-129)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-100 | Désigner le DPO (personne interne ou externe, contact public) (NX27-02) | DIR · `conformite.gerer` | compte « DPO » créé ou rattaché ; publié dans les mentions | `conformite.dpo_designe` | G2 |
| RGP-101 | Désigner le référent conformité et le référent sécurité | DIR | personnes du socle 6 | — | G2 |
| RGP-102 | Renseigner le responsable de traitement par traitement (établissement, OGEC) | DIR | Q-RG1 | — | G2 |
| RGP-103 | Renseigner le service d'archives compétent (NX27-02) | DIR, ARC | public : obligatoire avant tout lot à visa | — | G7 |
| RGP-104 | Générer le registre à partir des tables, des droits et des sous-traitants | SYS | §8.1 ; jamais de champ vide présenté comme rempli | `conformite.registre_genere` | G2 |
| RGP-105 | Compléter une fiche (base légale, commentaire) | DIR, DPO · `conformite.gerer` | base proposée (mission d'intérêt public) modifiable avec motif | — | G2 |
| RGP-106 | Ajouter un traitement hors ScolIA (Charlemagne, papier, messagerie académique) (NX27-03) | DIR, DPO | déclaratif, sans données | — | G2 |
| RGP-107 | Valider une version du registre ★ | DIR · `conformite.valider` | version figée (socle 7) | `conformite.registre_valide` | G2 |
| RGP-108 | Détecter un écart et passer une fiche « à revoir » | SYS | R-RG10 ; nouveau rôle lecteur, module activé, sous-traitant changé | `conformite.fiche_a_revoir` | G2 |
| RGP-109 | Comparer deux versions du registre | DIR, DPO | différences lisibles | — | G2 |
| RGP-110 | Exporter le registre (PDF, tableur) | DIR, DPO | modèle CNIL [H] | — | G2 |
| RGP-111 | Lire le registre de sous-traitant de ScolIA | DIR, DPO | publié par EDI | — | G2 |
| RGP-112 | Tenir le registre de sous-traitant de la plateforme (art. 30.2) | EDI · `plateforme.conformite` | un par client, généré | `plateforme.registre_publie` | G2 |
| RGP-113 | Tenir le registre propre de ScolIA (couche compte) | EDI | §5 | — | G2 |
| RGP-114 | Voir les destinataires réels d'une donnée (« qui peut voir les adresses ? ») | DIR, DPO | calcul D-017 + délégations en cours | — | G2 |
| RGP-115 | Simuler l'effet d'un changement de rôle sur le registre | DIR | avant d'enregistrer le rôle (socle 1) | — | G9 |
| RGP-116 | Lister les traitements qui exigent une AIPD | SYS | critères CNIL | — | G9 |
| RGP-117 | Lier une fiche à son AIPD | DPO | — | — | G9 |
| RGP-118 | Déclarer les transferts hors UE (aucun par défaut) | EDI | VERIF R59 | — | G2 |
| RGP-119 | Publier le dossier de conformité de la plateforme | EDI | §8.8.4 ; versionné | `plateforme.dossier_publie` | G2 |
| RGP-120 | Fiche « scolarité et dossier » générée (NX27-15) | SYS | — | — | G2 |
| RGP-121 | Fiche « vie scolaire, assiduité, discipline » | SYS | — | — | G2 |
| RGP-122 | Fiche « évaluations et bulletins » | SYS | — | — | G2 |
| RGP-123 | Fiche « communication » (messagerie, annonces, notifications) | SYS | — | — | G2 |
| RGP-124 | Fiche « finances des familles » | SYS | — | — | G2 |
| RGP-125 | Fiche « personnels » (employeur selon le profil) | SYS | — | — | G2 |
| RGP-126 | Fiche « santé » (inactive tant qu'A7) | SYS | R-RG26 | — | G2 |
| RGP-127 | Fiche « accueil et sûreté » (visiteurs) | SYS | — | — | G2 |
| RGP-128 | Fiche « IA » (capacités, fournisseurs, données envoyées) | SYS | `IA_COEUR.md` §9 | — | G2 |
| RGP-129 | Fiche « comptes et sécurité » (journaux, traces) | SYS | — | — | G2 |

### 14.2 Référentiel de conservation (RGP-130 à RGP-139)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-130 | Charger le catalogue du profil (public, privé sous contrat, hors contrat) | SYS (mise en service, 28) | version du catalogue notée | `conservation.catalogue_charge` | G1 |
| RGP-131 | Voir une catégorie : tables, objets racines, règle, source, marque [V]/[NV] | DIR, DPO | — | — | G1 |
| RGP-132 | Ajuster une règle dans ses bornes, avec motif | DIR, DPO · `conservation.regles.proposer` | bornes légales connues ; jamais en dessous d'un minimum | `conservation.regle_proposee` | G1 |
| RGP-133 | Valider une règle ou un bloc de règles ★ | DPO · `conservation.regles.valider` | R-RG1 ; version | `conservation.regle_validee` | G1 |
| RGP-134 | Refuser une proposition (retour avec commentaire) | DPO | — | — | G1 |
| RGP-135 | Recevoir une nouvelle version du catalogue ScolIA (changement proposé) | SYS | jamais appliquée seule | `conservation.catalogue_change` | G1 |
| RGP-136 | Comparer la règle en vigueur et la proposition nouvelle | DPO | — | — | G1 |
| RGP-137 | Exporter la politique de conservation (document) (NX27-15) | DIR, DPO | générée ; « non validée » visible | — | G1 |
| RGP-138 | Voir la règle et l'échéance d'un objet (« pourquoi garde-t-on ce dossier ? ») | quiconque lit l'objet | — | — | G1 |
| RGP-139 | Voir les tables sans catégorie (dette) | DIR, DPO, EDI | doit être vide (CI) | — | G0 |

### 14.3 Demandes de droits (RGP-140 à RGP-169)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-140 | Déposer une demande en ligne (connecté) | PAR, ELV, ELV-M, PER | qualité vérifiée par le socle 4 à la date | `droits.demande_recue` | G3 |
| RGP-141 | Qualifier la demande (RGPD, CADA, demande de document) | ADF · `droits.traiter` | motivée, modifiable | `droits.demande_qualifiee` | G3 |
| RGP-142 | Déposer une demande sans compte (ancien élève, parent sans compte) | ANC, PAR | vérification proportionnée §9.2 | `droits.demande_recue` | G3 |
| RGP-143 | Saisir une demande reçue par courrier, courriel ou au guichet, avec la pièce | ADF · `droits.recevoir` | date de réception = date du courrier reçu | `droits.demande_recue` | G3 |
| RGP-144 | Vérifier l'identité et la qualité | ADF | pièce consultée puis détruite à la clôture (R-RG16) | `droits.identite_verifiee` | G3 |
| RGP-145 | Accuser réception (date, échéance, contact) | SYS | automatique, texte clair | `notifications.avis_cree` | G3 |
| RGP-146 | Geler les sorts finaux de la personne pendant la demande | SYS | port `droits::demande_ouverte` | `conservation.gel_pose` | G3 |
| RGP-147 | Affecter la demande à un gestionnaire | DIR, DPO | — | — | G3 |
| RGP-148 | Prolonger de deux mois, avec motif, informer avant l'échéance d'un mois | DIR · `droits.arbitrer` | art. 12.3 | `droits.prolongee` | G3 |
| RGP-149 | Assembler l'extrait (appel des fournisseurs) | SYS | sections typées, période, extrait incomplet signalé (R-RG18) | `droits.extrait_assemble` | G3 |
| RGP-150 | Relire l'extrait : accepter ou lever chaque masquage, avec motif | ADF · `droits.traiter` | R-RG15 | — | G3 |
| RGP-151 | Ajouter une pièce ou une précision à l'extrait | ADF | — | — | G3 |
| RGP-152 | Faire relire par le DPO | ADF | traces | — | G3 |
| RGP-153 | Rédiger la réponse (modèle, brouillon IA) | ADF | N1, relue | — | G3 |
| RGP-154 | Remettre l'extrait (archive chiffrée, lien court nominatif, ou papier contre émargement) ★ | ADF, DIR · `droits.remettre` | passkey récente ; téléchargement journalisé | `droits.extrait_remis` | G3 |
| RGP-155 | Suivre l'avancement (côté demandeur) | demandeur | états lisibles | — | G3 |
| RGP-156 | Traiter une rectification (renvoi aux socles 2, 4 ou propriétaire) | ADF | réponse liste les corrections | `droits.rectification_faite` | G3 |
| RGP-157 | Examiner un effacement par catégorie ; accorder, refuser ou partiel, motif par article | DIR · `droits.arbitrer` | textes de refus pré-rédigés | `droits.effacement_decide` | G3 |
| RGP-158 | Exécuter un effacement accordé (via l'exécuteur du socle) | SYS | idempotent ; registre des effacements (rejeu R-RG22) | `conservation.efface` | G6 |
| RGP-159 | Retirer un consentement (renvoi au socle 18) | PAR, ELV-M | — | `formulaires.consentement_retire` | G3 |
| RGP-160 | Poser une limitation sur un objet contesté | DIR | lisible des seuls gestionnaires | `droits.limitation_posee` | G3 |
| RGP-161 | Lever la limitation | DIR | — | `droits.limitation_levee` | G3 |
| RGP-162 | Examiner une opposition (situation particulière, motifs impérieux) | DIR | jamais refus par défaut | `droits.opposition_decidee` | G3 |
| RGP-163 | Produire un extrait réutilisable (CSV) même hors portabilité | SYS | §4.1 ; bonne pratique | — | G3 |
| RGP-164 | Refuser une demande manifestement excessive ★ | DIR · `droits.arbitrer` | R-RG17 ; motif | `droits.refusee` | G3 |
| RGP-165 | Communiquer le nom d'un agent ayant consulté (arbitrage) | DIR | §11.3 | — | G4 |
| RGP-166 | Clore la demande (réponse, éléments remis par empreinte) | ADF | lève le gel | `droits.close` | G3 |
| RGP-167 | Rouvrir (réclamation du demandeur) | DIR, DPO | — | `droits.rouverte` | G3 |
| RGP-168 | Éclater une demande multi-établissements | EDI, SYS | §9.6 | — | G10 |
| RGP-169 | Tableau des demandes (échéances, retards, par gestionnaire), consolidé pour le DPO | DIR, DPO | métadonnées seulement | — | G3 |

### 14.4 Fin d'année, sortie, comptes (RGP-170 à RGP-179)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-170 | Demander l'effacement des sanctions au changement d'établissement (R511-13 IV) | ELV, PAR | exécuté par le 13 | `discipline.effacement_demande` | G6 |
| RGP-171 | Prévenir la famille de la fermeture de son accès et de ce qu'elle peut télécharger | SYS (à la sortie) | S-545 | `notifications.avis_cree` | G6 |
| RGP-172 | Couper les diffusions futures d'un élève sorti | SYS | ports des socles 8, 16, 17 | — | G6 |
| RGP-173 | Désactiver le compte élève à la date prévue | SYS | — | `compte.desactive` | G6 |
| RGP-174 | Réactiver à la réintégration | ADF | CL27-05 | — | G6 |
| RGP-175 | Anonymiser un compte inactif (couche ScolIA) | SYS | `cpt.compte_inactif`, passkeys supprimées | `compte.anonymise` | G10 |
| RGP-176 | Calculer les faits générateurs de fin d'année | SYS (`annee.cloturee`) | — | — | G6 |
| RGP-177 | Passer en conservation intermédiaire à échéance | SYS | réversible | `conservation.intermediaire` | G6 |
| RGP-178 | Rendre un objet à l'utilité courante (erreur, retour) | DIR | motif | — | G6 |
| RGP-179 | Transmettre le dossier scolaire au nouvel établissement (renvoi au socle 7) | ADF | INS-74 | — | — |

### 14.5 Sous-traitants (RGP-180 à RGP-189)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-180 | Tenir la liste des sous-traitants de la plateforme (fournisseur, service, données, région, accord) (NX27-08) | EDI | §8.8 | `plateforme.soustraitants_modifies` | G2 |
| RGP-181 | Annoncer un changement de sous-traitant 30 jours avant | EDI | art. 28.2 | `plateforme.soustraitant_annonce` | G2 |
| RGP-182 | Formuler une objection | DIR | traitée par EDI, réponse écrite | `plateforme.objection` | G2 |
| RGP-183 | Déclarer un sous-traitant propre à l'établissement | DIR · `soustraitants.gerer` | déclaratif | — | G2 |
| RGP-184 | Joindre l'accord de traitement signé (établissement ↔ ScolIA) | DIR | pièce (socle 7), date, version | `conformite.contrat_joint` | G2 |
| RGP-185 | Voir les régions et la localisation des données | DIR, DPO | — | — | G2 |
| RGP-186 | Demander un audit (art. 28.3.h) | DIR, DPO | procédure éditeur | — | G10 |
| RGP-187 | Lister les fournisseurs d'IA et ce qu'ils reçoivent | DIR, DPO | §10.3 | — | G2 |
| RGP-188 | Alerte : accord de traitement manquant ou expiré | SYS | point ouvert | — | G2 |
| RGP-189 | Historique des sous-traitants (qui, quand, quel préavis) | DIR, DPO | — | — | G2 |

### 14.6 Journal des accès (RGP-190 à RGP-199)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-190 | Tracer une consultation sensible dans la transaction de lecture | SYS | R-RG8 | — | G0 |
| RGP-191 | Choisir un motif pour une consultation hors périmètre | PER | liste courte S-542 ; un geste | — | G4 |
| RGP-192 | Voir les consultations du dossier de son enfant ou du sien | PAR, ELV-M, PER | §11.2 | — | G4 |
| RGP-193 | Voir les traces d'un sujet pour une demande ou un contrôle ouvert | DIR, DPO · `acces.consulter_traces` | tracé lui-même | — | G4 |
| RGP-194 | Ouvrir un contrôle interne sur un agent ★ | DIR · `acces.controle_ouvrir` | motif, durée, R-RG27 | `acces.controle_ouvert` | G4 |
| RGP-195 | Clore un contrôle | DIR | — | `acces.controle_clos` | G4 |
| RGP-196 | Alerte de consultations anormales | SYS | §11.5 | `acces.alerte` | G4 |
| RGP-197 | Différer l'affichage d'une consultation (signalement en cours) | DIR | R-RG28, tracé | — | G4 |
| RGP-198 | Purger les partitions de traces échues (procédure) | SYS | `cons_execution` | `conservation.traces_purgees` | G4 |
| RGP-199 | Exporter les traces d'une personne (pour sa demande) | ADF | dans l'extrait | — | G4 |

### 14.7 Questionnaire, AIPD (RGP-200 à RGP-209)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-200 | Questionnaire pré-rempli, reprise à l'étape en cours (NX27-01) | DIR, DPO | ne demande que ce que ScolIA ignore | — | G9 |
| RGP-201 | Identité et gouvernance (NX27-02, NX27-21) | DIR | personnes du socle 6 | — | G2 |
| RGP-202 | Logiciels en place (NX27-03) | DIR | — | — | G2 |
| RGP-203 | Activités déduites des modules actifs (NX27-04) | SYS | — | — | G2 |
| RGP-204 | Publics concernés (NX27-06) | SYS, DIR | — | — | G2 |
| RGP-205 | Traitements sensibles (NX27-07) | DIR | déclenchent les critères d'AIPD | — | G9 |
| RGP-206 | Mesures existantes (NX27-09) | DIR | — | — | G9 |
| RGP-207 | Rédiger l'AIPD assistée | DIR, DPO | §8.9 | — | G9 |
| RGP-208 | Valider et signer l'AIPD ★ | DIR | socle 7 | `conformite.aipd_validee` | G9 |
| RGP-209 | Réviser l'AIPD à un changement | SYS → DIR | module activé, IA nouvelle | `conformite.aipd_a_revoir` | G9 |

### 14.8 Documents et information (RGP-210 à RGP-229)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-210 | Voir le catalogue de documents et leur état (NX27-10, NX27-11) | DIR, DPO | — | — | G9 |
| RGP-211 | Générer les mentions d'information « familles » | SYS | depuis le registre | — | G2 |
| RGP-212 | Générer les mentions « élèves » (collège, lycée) | SYS | art. 12.1 | — | G2 |
| RGP-213 | Générer les mentions « personnels » | SYS | — | — | G2 |
| RGP-214 | Générer les mentions « visiteurs » et « candidats » | SYS | socles 14, 3, 6 | — | G9 |
| RGP-215 | Publier une version des mentions ★ | DIR · `conformite.valider` | annonce si substantielle (17) | `conformite.mentions_publiees` | G2 |
| RGP-216 | Page publique « Vos données » | tous | sans connexion | — | G2 |
| RGP-217 | Page « Tes données » de l'élève | ELV | langage adapté | — | G9 |
| RGP-218 | Générer la procédure d'exercice des droits | SYS | — | — | G2 |
| RGP-219 | Générer la procédure de violation | SYS | — | — | G2 |
| RGP-220 | Générer la liste des sous-traitants | SYS | — | — | G2 |
| RGP-221 | Générer les mesures de sécurité (PSSI simplifiée) | SYS | dossier plateforme + mesures locales | — | G9 |
| RGP-222 | Adapter un modèle de charte (informatique, photos) | DIR | modèle à adapter | — | G9 |
| RGP-223 | Importer un document existant (NX27-13) | DIR | dépôt socle 7 | — | G9 |
| RGP-224 | Marquer un document « sans objet » avec raison (NX27-11) | DIR | — | — | G9 |
| RGP-225 | Valider un document ★ | DIR | version figée | `conformite.document_valide` | G9 |
| RGP-226 | Exporter un document en PDF (NX27-12) | DIR, DPO | — | — | G9 |
| RGP-227 | Remplacer un document importé | DIR | versions gardées | — | G9 |
| RGP-228 | Analyse d'un document par IA (critères art. 30) (NX27-14) | DIR, DPO | §10.3 ; N0 | — | G11 |
| RGP-229 | Lier un formulaire de collecte (18) à ses mentions | SYS | contrôle : tout formulaire a ses mentions | — | G9 |

### 14.9 Tableau de bord (RGP-230 à RGP-239)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-230 | Points ouverts classés par gravité (au lieu d'un score) (NX27-16) | DIR, DPO | RG-D15 | — | G2 |
| RGP-231 | Prêt pour les données réelles ? (liste des prérequis A8, A11, A13, A15) | DIR, EDI | bloque l'ouverture réelle (socle 28) | — | G2 |
| RGP-232 | Échéances des 12 prochains mois | DIR, DPO | — | — | G6 |
| RGP-233 | Volumes par catégorie et par étape | DIR, DPO | compteurs | — | G6 |
| RGP-234 | Gels ouverts et dates de revue | DIR | — | — | G6 |
| RGP-235 | Demandes en retard ou proches | DIR, DPO | — | — | G3 |
| RGP-236 | Violations ouvertes et horloges | DIR, DPO | — | — | G5 |
| RGP-237 | Revue annuelle (habilitations, règles, sous-traitants, AIPD) | DIR, DPO | A12 ; tâche annuelle | `conformite.revue_annuelle` | G9 |
| RGP-238 | Tableau consolidé du DPO multi-établissements | DPO | guichet central, sans nominatif | — | G10 |
| RGP-239 | Rapport annuel de conformité (pour le CA, l'OGEC) | DIR | chiffres, sans nominatif | — | G9 |

### 14.10 Violations (RGP-240 à RGP-259)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-240 | Déclarer un incident en 30 secondes (NX27-17) | PER · `violations.declarer` | ton neutre ; horodatage serveur | `violations.declaree` | G5 |
| RGP-241 | Déclaration automatique par la supervision ou un socle | SYS | port `violations::declarer` | `violations.declaree` | G5 |
| RGP-242 | Être prévenu immédiatement (direction, DPO, référent sécurité) | SYS | tous canaux, hors plage calme | — | G5 |
| RGP-243 | Qualifier (nature, données, personnes, conséquences) | DIR, DPO · `violations.gerer` | grille déterministe | — | G5 |
| RGP-244 | Calculer les personnes concernées depuis l'origine (envoi, export, liste) | SYS | ports 8, 16, exports | — | G5 |
| RGP-245 | Décider le niveau de risque ★ | DIR | après avis DPO tracé | `violations.risque_decide` | G5 |
| RGP-246 | Consigner les mesures prises et envisagées | DIR | — | — | G5 |
| RGP-247 | Préparer la notification CNIL (brouillon art. 33.3) | SYS, DIR | N1 | — | G5 |
| RGP-248 | Saisir la référence et la date de notification | DIR | — | `violations.notifiee_cnil` | G5 |
| RGP-249 | Notification complémentaire | DIR | — | — | G5 |
| RGP-250 | Motiver une notification tardive | DIR | art. 33.1 | — | G5 |
| RGP-251 | Fiche incident PDF (NX27-18) | DIR, DPO | — | — | G5 |
| RGP-252 | Informer les personnes (risque élevé) ★ | DIR | N3 ; socle 8 | `violations.personnes_informees` | G5 |
| RGP-253 | Justifier la non-notification | DIR | art. 33.5 | — | G5 |
| RGP-254 | Déclarer une violation de plateforme et la propager | EDI · `plateforme.conformite` | §8.7.6 | `plateforme.violation_declaree` | G5 |
| RGP-255 | Recevoir une violation propagée (vue de l'établissement) | DIR | seuls ses éléments | — | G5 |
| RGP-256 | Clore, retour d'expérience, actions correctives (tâches, 29) | DIR | revue à 30 j | `violations.close` | G5 |
| RGP-257 | Assistant de qualification par conversation (NX27-19) | DIR, DPO | pseudonymisé, historique serveur | — | G11 |
| RGP-258 | Registre des violations (liste, filtres, export) | DIR, DPO | — | — | G5 |
| RGP-259 | Incident de sécurité sans violation (NX27-17) | DIR | même registre, nature distincte | — | G5 |

### 14.11 Cycle de vie, lots, archives (RGP-260 à RGP-289)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-260 | Calculer les échéances chaque nuit | SYS | catégories validées seulement | — | G6 |
| RGP-261 | Proposer un lot par catégorie et fenêtre | SYS, DIR · `conservation.lots.proposer` | exclusions §8.3.4 | `conservation.lot_propose` | G7 |
| RGP-262 | Ouvrir l'échantillon du lot | DIR, DPO | graine notée ; traces | — | G7 |
| RGP-263 | Retirer un objet du lot avec motif | DIR | — | — | G7 |
| RGP-264 | Valider un lot (deux personnes pour les dossiers d'élèves) ★ | DIR, DPO · `conservation.lots.valider` | R-RG7 | `conservation.lot_valide` | G7 |
| RGP-265 | Générer le bordereau d'élimination | SYS | modèle du service d'archives | — | G7 |
| RGP-266 | Saisir la référence du visa et joindre la pièce | DIR, ADF · `conservation.visa.saisir` | R-RG6 | `conservation.visa_saisi` | G7 |
| RGP-267 | Exécuter le lot (exécuteurs des socles) | SYS | transaction par objet ; compte rendu | `conservation.lot_execute` | G8 |
| RGP-268 | Certificat de destruction | SYS | socle 7, signé | — | G8 |
| RGP-269 | Annuler un lot non exécuté | DIR | — | `conservation.lot_annule` | G7 |
| RGP-270 | Simuler un lot (sans écriture) | DIR, DPO | — | — | G7 |
| RGP-271 | Anonymiser un objet racine | SYS (exécuteur) | seuil R-RG20 | `conservation.anonymise` | G8 |
| RGP-272 | Poser un gel (motif, date de revue) | DIR, socles 13, 25, 36 · `conservation.geler` | — | `conservation.gel_pose` | G6 |
| RGP-273 | Lever un gel | DIR | — | `conservation.gel_leve` | G6 |
| RGP-274 | Rappel de revue d'un gel | SYS | — | — | G6 |
| RGP-275 | Consulter un objet en conservation intermédiaire | habilités · `conservation.intermediaire.lire` | tracé | — | G6 |
| RGP-276 | Préparer un versement (SEDA + PDF/A) | ARC, DIR · `archives.verser` | §8.5 | `archives.versement_prepare` | G12 |
| RGP-277 | Générer le bordereau de versement | SYS | — | — | G12 |
| RGP-278 | Enregistrer l'accusé de réception du versement | DIR | — | `archives.verse` | G12 |
| RGP-279 | Proposer l'élimination locale après versement | SYS | lot | — | G12 |
| RGP-280 | Placer en conservation définitive dans ScolIA | SYS | lecture seule, accès restreint | `archives.definitive` | G12 |
| RGP-281 | Consulter une archive définitive | ARC, DIR · `archives.lire` | délais de communicabilité ; tracé | — | G12 |
| RGP-282 | Trier selon un critère écrit (échantillonnage, lettres de l'alphabet, années) | ARC | critère dans la règle | — | G12 |
| RGP-283 | Rejouer les sorts finaux après une restauration | SYS (PRA) | R-RG22 | `conservation.rejeu` | G8 |
| RGP-284 | Registre des lots, des effacements et des certificats | DIR, DPO | preuves | — | G8 |
| RGP-285 | Exporter la preuve d'un lot (pour un contrôle) | DIR, DPO | — | — | G8 |
| RGP-286 | Vérifier qu'aucune donnée d'un lot exécuté ne subsiste (contrôle à J+1) | SYS | requête par objet racine | — | G8 |
| RGP-287 | Purger les fichiers orphelins d'un objet détruit (socle 7) | SYS | — | — | G8 |
| RGP-288 | Remonter une dette (table sans exécuteur, fournisseur manquant) | SYS | point ouvert | — | G6 |
| RGP-289 | Effacement des sanctions à échéance (exécuteur du 13) | SYS | R511-13 IV | `discipline.sanction_effacee` | G6 |

### 14.12 Plateforme, réversibilité, système (RGP-290 à RGP-299)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RGP-290 | Préparer la fin de contrat : export final proposé | DIR (28 émet `etablissement.contrat_termine`) | PORTABILITE §10 | — | G10 |
| RGP-291 | Supprimer les données de l'établissement à la date convenue ★ | EDI + DIR (deux personnes) | R-RG24 | `etablissement.donnees_supprimees` | G10 |
| RGP-292 | Attestation de suppression | SYS | mentionne les sauvegardes | — | G10 |
| RGP-293 | Gérer une réquisition d'autorité (registre, gel, remise tracée) | DIR | hors plateforme ; pièce jointe | `droits.requisition` | G10 |
| RGP-294 | Pseudonymiser un jeu pour le pilotage ou l'évaluation de l'IA | SYS | seuils, clés hors base | — | G11 |
| RGP-295 | Tester les fournisseurs d'extrait et les exécuteurs (CI) | SYS | R-RG14 | — | G0 |
| RGP-296 | Vérifier la catégorie de chaque table (CI) | SYS | R-RG12 | — | G0 |
| RGP-297 | Bloquer `TRUNCATE` et la désactivation du journal | SYS | RG-D02 | — | G0 |
| RGP-298 | Reprendre les données du module RGPD de Next.js | EDI | §19 | — | G2 |
| RGP-299 | Évaluer la capacité IA du socle (jeu d'évaluation) | SYS | `IA_COEUR.md` | — | G11 |

---

## 15. Règles transverses (R-RG1 à R-RG40) [R]

| N° | Règle |
|---|---|
| R-RG1 | Une catégorie non validée ne déclenche **aucun sort final** ; on conserve et on l'affiche partout. |
| R-RG2 | Aucune destruction réelle n'est nécessaire pendant l'année de doublon 2027-2028 ; les lots G6–G8 peuvent venir après, sans retard de conformité (aucune échéance n'est atteinte avant 2028 pour des données entrées en 2027). |
| R-RG3 | Le 27 ne supprime jamais une ligne d'un autre socle : il appelle l'exécuteur du socle propriétaire. |
| R-RG4 | Un sort final ne s'exécute qu'à travers un lot validé par un humain ; jamais par une tâche automatique seule (sauf traces d'accès et données éphémères, par procédure). |
| R-RG5 | Profil « privé sous contrat » : visa et versement traités comme dans le public pour les catégories de scolarité, tant que le juriste n'a pas tranché L211-4 2°. |
| R-RG6 | Lot soumis à visa : pas d'exécution sans référence de visa et pièce jointe. |
| R-RG7 | Destruction de dossiers d'élèves : validation par deux personnes distinctes (comptes, personnes, passkeys). |
| R-RG8 | Une lecture déclarée sensible n'est servie que si sa trace est écrite dans la même transaction ; échec de la trace = refus. |
| R-RG9 | ScolIA ne s'affiche jamais « conforme » ; il décrit ses mesures et les choix de l'établissement. |
| R-RG10 | Tout changement de fait (droits, modules, sous-traitants) passe la fiche du registre concernée « à revoir ». |
| R-RG11 | Le registre généré n'affiche jamais un champ vide comme rempli : « non renseigné » est visible. |
| R-RG12 | Toute table à données personnelles porte une catégorie connue ; la CI échoue sinon. |
| R-RG13 | Toute catégorie porte une source ou la mention « choix de minimisation, sans texte ». |
| R-RG14 | Toute table à données personnelles est couverte par un fournisseur d'extrait ou listée « hors extrait » avec raison ; vérifié en CI. |
| R-RG15 | Aucun extrait n'est remis sans relecture humaine de chaque marquage de tiers. |
| R-RG16 | Une pièce d'identité reçue pour une demande est détruite à la clôture ; seule la mention « identité vérifiée le, par » reste. |
| R-RG17 | Le refus pour demande excessive est une décision motivée de la direction, jamais automatique. |
| R-RG18 | Un extrait dont un fournisseur a échoué est marqué incomplet et ne peut pas être remis comme complet. |
| R-RG19 | Une demande ouverte gèle les sorts finaux de la personne concernée. |
| R-RG20 | Anonymisation : aucun agrégat publié ou conservé sur moins de 5 personnes. |
| R-RG21 | L'horloge des 72 h ne bloque jamais une notification tardive ; elle exige un motif. |
| R-RG22 | Après une restauration, les sorts finaux et effacements exécutés après l'instant restauré sont rejoués avant la réouverture. |
| R-RG23 | ScolIA conserve et prépare les versements ; il ne se présente pas comme un système d'archivage électronique à valeur probante. |
| R-RG24 | En fin de contrat, les archives définitives et les catégories soumises à visa sont remises, jamais détruites sans visa. |
| R-RG25 | Les rectifications faites sur demande sont rejouées après une restauration (comme les effacements). |
| R-RG26 | L'activation d'un module qui ouvre une catégorie sensible (santé) exige AIPD révisée, registre revu et catégories validées. |
| R-RG27 | Le journal des accès ne sert pas à évaluer le travail ; l'accès aux traces d'un agent exige un contrôle interne ouvert, motivé, daté et tracé. |
| R-RG28 | Les consultations liées à un signalement en cours peuvent être différées dans le journal visible, sur décision motivée tracée. |
| R-RG29 | Les consultations de santé et de suivi psy ne figurent pas dans le journal standard de la famille ; elles relèvent des règles des socles 34 et 36. |
| R-RG30 | Les traces d'accès sont stockées à part du journal des modifications et purgées par partition, par procédure journalisée. |
| R-RG31 | La fonction de l'acteur est figée dans la trace au moment de la consultation. |
| R-RG32 | Les écrans de conformité ne montrent que des compteurs, sauf la demande ou le lot qui justifie l'accès à une donnée. |
| R-RG33 | Aucune donnée personnelle n'est envoyée à un modèle d'IA externe sans garanties validées ; les conversations d'incident sont pseudonymisées côté serveur. |
| R-RG34 | L'IA propose ; aucune décision de droit, de risque, de notification, de durée ou de lot ne lui appartient. |
| R-RG35 | Un changement de sous-traitant ultérieur est annoncé avant application, avec délai d'objection. |
| R-RG36 | Une violation de plateforme est propagée à chaque établissement concerné avec ses seuls éléments. |
| R-RG37 | Une demande de droit est traitée par l'établissement qui détient les données ; la couche compte par ScolIA. |
| R-RG38 | Un parent n'obtient jamais, par le droit d'accès, une donnée propre de l'autre parent ni une adresse protégée. |
| R-RG39 | Une durée ajustée reste dans les bornes légales connues ; un changement de règle n'a pas d'effet rétroactif silencieux. |
| R-RG40 | Toute transition d'état du 27 (demande, lot, violation, règle) écrit le journal dans la même transaction. |

---

## 16. Événements (outbox, dans la transaction du fait) [R]

Émis : `conformite.dpo_designe`, `conformite.registre_genere`, `conformite.registre_valide`, `conformite.fiche_a_revoir`, `conformite.mentions_publiees`, `conformite.aipd_validee`, `conformite.aipd_a_revoir`, `conformite.document_valide`, `conformite.revue_annuelle`, `conservation.catalogue_charge`, `conservation.catalogue_change`, `conservation.regle_proposee`, `conservation.regle_validee`, `conservation.intermediaire`, `conservation.gel_pose`, `conservation.gel_leve`, `conservation.lot_propose`, `conservation.lot_valide`, `conservation.visa_saisi`, `conservation.lot_execute`, `conservation.lot_annule`, `conservation.efface`, `conservation.anonymise`, `conservation.rejeu`, `conservation.traces_purgees`, `archives.versement_prepare`, `archives.verse`, `archives.definitive`, `droits.demande_recue`, `droits.demande_qualifiee`, `droits.identite_verifiee`, `droits.prolongee`, `droits.extrait_assemble`, `droits.extrait_remis`, `droits.rectification_faite`, `droits.effacement_decide`, `droits.limitation_posee`, `droits.limitation_levee`, `droits.opposition_decidee`, `droits.refusee`, `droits.close`, `droits.rouverte`, `droits.requisition`, `acces.controle_ouvert`, `acces.controle_clos`, `acces.alerte`, `violations.declaree`, `violations.risque_decide`, `violations.notifiee_cnil`, `violations.personnes_informees`, `violations.close`, `plateforme.*` (registre, sous-traitants, violation). Charges utiles : identifiants seulement, jamais de contenu (E14).

---

## 17. Modèle de données cible (brique `conformite`, D-036, E1–E14) [R]

Toutes les tables : `id UUID` applicatif (E3), `etablissement_id UUID NOT NULL` (E1) sauf les tables de plateforme marquées **P** (portée `Technique`/`ReferentielGlobal` au registre, sans donnée d'établissement, E4), FK composites `(etablissement_id, x_id)` (E2), `cree_le`/`modifie_le` (E6), RLS (E13), **aucune colonne JSON ni tableau** (E10), fichiers par référence au socle 7 (E5), transitions journalisées en transaction (R-RG40). Préfixes : `rgp_` (gouvernance, droits, violations), `cons_` (conservation), `acc_` (traces), `arc_` (archives).

### 17.1 Gouvernance et registre
| Table | Colonnes principales |
|---|---|
| `rgp_gouvernance` | `etablissement_id` (unique), `profil` (`public`, `prive_sous_contrat`, `prive_hors_contrat`), `dpo_personne_id`, `dpo_externe_nom`, `dpo_contact`, `referent_conformite_personne_id`, `referent_securite_personne_id`, `service_archives_nom`, `service_archives_contact`, `responsable_traitement_libelle` |
| `rgp_traitement` | `id`, `etablissement_id`, `code` (traitement de base ou local), `libelle`, `finalite`, `base_legale` (enum), `base_legale_motif`, `responsable` (enum `etablissement`/`ogec`/`autre`), `hors_scolia` (bool), `etat` (`a_completer`/`a_revoir`/`valide`), `aipd_requise` (bool) |
| `rgp_traitement_domaine` | `traitement_id`, `domaine` (code du registre des tables) |
| `rgp_traitement_version` | `traitement_id`, `version`, `valide_par_personne_id`, `valide_le`, `document_id` (instantané PDF socle 7) |
| `rgp_ecart` | `traitement_id`, `nature` (`destinataire_ajoute`, `module_active`, `soustraitant_change`, `categorie_changee`), `detail_code`, `detecte_le`, `traite_le` |
| `rgp_destinataire_externe` | `traitement_id`, `libelle`, `nature` (`administration`, `soustraitant`, `partenaire`), `soustraitant_id` |
| `rgp_soustraitant` **P** | `id`, `nom`, `service`, `region`, `pays`, `garanties`, `accord_version`, `accord_le`, `entree_le`, `sortie_le` |
| `rgp_soustraitant_donnee` **P** | `soustraitant_id`, `categorie_code` |
| `rgp_soustraitant_annonce` **P** | `soustraitant_id`, `nature` (`entree`/`sortie`/`changement`), `annonce_le`, `effet_le` |
| `rgp_soustraitant_objection` | `etablissement_id`, `annonce_id`, `motif`, `par_personne_id`, `le`, `reponse`, `repondu_le` |
| `rgp_soustraitant_local` | `etablissement_id`, `nom`, `service`, `region`, `accord_document_id` |
| `rgp_document` | `etablissement_id`, `code` (catalogue), `etat` (`manquant`/`brouillon`/`genere`/`importe`/`valide`/`sans_objet`), `sans_objet_raison`, `document_id` (socle 7), `perime_le` |
| `rgp_aipd` | `etablissement_id`, `version`, `etat`, `valide_par_personne_id`, `valide_le`, `document_id` |
| `rgp_aipd_critere` | `aipd_id`, `critere_code`, `rempli` (bool), `source` (`deduit`/`declare`) |
| `rgp_mention` | `etablissement_id`, `public` (`familles`/`eleves_college`/`eleves_lycee`/`personnels`/`visiteurs`/`candidats`), `version`, `publiee_le`, `document_id` |
| `rgp_questionnaire_reponse` | `etablissement_id`, `question_code`, `valeur_code` ou `valeur_texte` ou `valeur_nombre` (une colonne typée remplie selon la question), `repondu_par`, `le` |

### 17.2 Conservation
| Table | Colonnes principales |
|---|---|
| `cons_categorie` **P** | `code` (PK), `libelle`, `socle`, `objet_racine` (enum), `fait_generateur` (enum), `source_texte`, `source_url`, `marque` (`V`/`V_ext`/`NV`/`minimisation`), `version_catalogue` |
| `cons_categorie_proposition` **P** | `code`, `profil`, `utilite_mois`, `intermediaire_mois`, `sort_final` (enum), `critere_tri`, `borne_min_mois`, `borne_max_mois`, `visa_exige` (bool) |
| `cons_regle` | `etablissement_id`, `categorie_code`, `version`, `utilite_mois`, `intermediaire_mois`, `sort_final`, `critere_tri`, `visa_exige`, `motif_ajustement`, `etat` (`proposee`/`validee`/`remplacee`), `propose_par`, `valide_par_personne_id`, `valide_le` ; unique (etab, catégorie, version) ; une seule `validee` en vigueur (index partiel) |
| `cons_echeance` | `etablissement_id`, `categorie_code`, `objet_type`, `objet_id`, `fait_generateur_le`, `intermediaire_le`, `sort_le`, `regle_version`, `etat` (`actif`/`intermediaire`/`dans_lot`/`execute`/`exclu`), `calcule_le` |
| `cons_gel` | `etablissement_id`, `objet_type`, `objet_id`, `motif` (enum `contentieux`/`recours`/`signalement`/`requisition`/`limitation`/`demande_droit`/`creance`), `pose_par`, `pose_le`, `revue_le`, `leve_le`, `leve_par` |
| `cons_lot` | `etablissement_id`, `categorie_code`, `regle_version`, `sort_final`, `etat` (`propose`/`en_relecture`/`valide`/`a_viser`/`vise`/`en_execution`/`execute`/`annule`), `graine_echantillon`, `propose_le`, `execute_le`, `certificat_document_id`, `bordereau_document_id` |
| `cons_lot_objet` | `lot_id`, `etablissement_id`, `objet_type`, `objet_id`, `retire_motif`, `resultat` (`detruit`/`anonymise`/`verse`/`refuse`), `resultat_raison`, `traite_le` |
| `cons_lot_validation` | `lot_id`, `etablissement_id`, `par_user`, `par_personne_id`, `credential_id`, `le` ; `UNIQUE(lot_id, par_personne_id)` ; contrôle « deux personnes distinctes » (R-RG7) |
| `cons_visa` | `lot_id`, `etablissement_id`, `reference`, `date_visa`, `service`, `piece_document_id`, `saisi_par`, `saisi_le` |
| `cons_execution` | `etablissement_id` (ou plateforme), `nature` (`lot`/`effacement_droit`/`purge_traces`/`purge_ephemere`/`rejeu`), `reference_id`, `debut_le`, `fin_le`, `objets`, `erreurs` (nombres) — c'est le **registre des effacements** rejoué après restauration (R-RG22) |
| `cons_intermediaire` | `etablissement_id`, `objet_type`, `objet_id`, `depuis_le`, `par` (`sys`/personne), `rendu_le` (retour à l'utilité courante) |

### 17.3 Archives
| Table | Colonnes principales |
|---|---|
| `arc_versement` | `etablissement_id`, `etat` (`prepare`/`transmis`/`accuse`/`elimine_localement`), `service`, `bordereau_document_id`, `paquet_cle_objet`, `paquet_sha256`, `format_version` (`seda_2_x`), `accuse_reference`, `accuse_le` |
| `arc_versement_objet` | `versement_id`, `etablissement_id`, `objet_type`, `objet_id` |
| `arc_definitif` | `etablissement_id`, `objet_type`, `objet_id`, `depuis_le`, `communicable_le` (calculé selon les délais) |

### 17.4 Droits des personnes
| Table | Colonnes principales |
|---|---|
| `rgp_demande` | `etablissement_id`, `numero` (compteur par établissement, E3), `canal` (`en_ligne`/`courrier`/`courriel`/`guichet`/`dpo`/`cnil`), `recue_le`, `echeance_le`, `prolongee_jusqu_au`, `prolongation_motif`, `demandeur_personne_id` (nullable si inconnu), `demandeur_nom_saisi` (pour un demandeur sans fiche), `qualite` (enum), `sujet_type`, `sujet_id`, `qualification` (`rgpd`/`cada`/`document`), `etat` (`recue`/`identite_a_verifier`/`en_cours`/`a_relire`/`prete`/`remise`/`close`/`refusee`), `gestionnaire_personne_id`, `close_le` |
| `rgp_demande_droit` | `demande_id`, `etablissement_id`, `droit` (`acces`/`rectification`/`effacement`/`limitation`/`opposition`/`portabilite`/`consultations`), `decision` (`accorde`/`partiel`/`refuse`/`sans_objet`), `motif_code`, `motif_texte` |
| `rgp_demande_identite` | `demande_id`, `etablissement_id`, `mode` (`session_forte`/`code_courriel`/`piece`/`mandat`), `verifie_par`, `verifie_le`, `piece_document_id` (détruite à la clôture) |
| `rgp_extrait` | `demande_id`, `etablissement_id`, `version`, `assemble_le`, `complet` (bool), `archive_cle_objet`, `archive_sha256`, `remis_le`, `remis_mode` |
| `rgp_extrait_section` | `extrait_id`, `etablissement_id`, `socle`, `fournisseur_code`, `statut` (`ok`/`echec`/`vide`), `lignes` (nombre) |
| `rgp_extrait_masquage` | `extrait_id`, `etablissement_id`, `section_id`, `element_ref`, `raison` (`tiers`/`autre_parent`/`adresse_protegee`/`signalement`/`sante`/`autre`), `propose_par` (`sys`/`ia`/personne), `decision` (`masque`/`leve`), `motif`, `decide_par`, `decide_le` |
| `rgp_requisition` | `etablissement_id`, `autorite`, `reference`, `recue_le`, `piece_document_id`, `perimetre_texte`, `remise_le` |

Le contenu de l'extrait n'est **pas** stocké en base : il est reconstruit à la demande depuis les fournisseurs et figé dans l'archive chiffrée (socle 7), conservée selon `rgpd.demande`.

### 17.5 Traces d'accès
| Table | Colonnes principales |
|---|---|
| `acc_trace` (partitionnée par mois sur `le`) | `id`, `etablissement_id`, `le`, `acteur_user_id`, `acteur_personne_id`, `acteur_fonction_code` (figée), `acteur_service_code`, `acteur_nature` (`personnel`/`support`/`systeme`), `sujet_type` (`eleve`/`personne`/`agent`), `sujet_id`, `domaine`, `section_code`, `nature` (`consultation`/`export`/`impression`/`telechargement`), `motif_code`, `hors_perimetre` (bool), `visible_sujet` (bool), `differee_jusqu_au`, `support_autorisation_id` (socle 28) |
| `acc_controle` | `etablissement_id`, `agent_personne_id`, `motif`, `ouvert_par`, `ouvert_le`, `fin_prevue_le`, `clos_le` |
| `acc_alerte` | `etablissement_id`, `regle_code`, `acteur_user_id`, `debut_le`, `nombre`, `traitee_par`, `traitee_le`, `suite` (`rien`/`violation`/`controle`) |

Pas de clé étrangère vers les sujets (une trace survit à l'anonymisation du sujet, qui la rend anonyme par son exécuteur) [R] ; contrainte composite sur `acteur_user_id` vers les comptes rattachés.

### 17.6 Violations
| Table | Colonnes principales |
|---|---|
| `rgp_violation` | `etablissement_id`, `numero`, `nature_incident` (`violation`/`incident_securite`), `origine` (`declaration`/`supervision`/`plateforme`), `plateforme_violation_id`, `connue_le`, `echeance_cnil_le` (calculée +72 h), `confidentialite`/`integrite`/`disponibilite` (bools), `personnes_nombre`, `personnes_estime` (bool), `consequences`, `risque` (`faible`/`avere`/`eleve`), `risque_decide_par`, `risque_decide_le`, `avis_dpo`, `notifiee_cnil_le`, `reference_cnil`, `retard_motif`, `personnes_informees_le`, `non_notification_motif`, `etat` (`ouverte`/`qualifiee`/`notifiee`/`close`), `close_le` |
| `rgp_violation_categorie` | `violation_id`, `etablissement_id`, `categorie_code` |
| `rgp_violation_mesure` | `violation_id`, `etablissement_id`, `nature` (`prise`/`envisagee`), `texte`, `le`, `par` |
| `rgp_violation_etape` | chronologie en ajout seul : `violation_id`, `etablissement_id`, `le`, `par`, `etape_code`, `texte` |
| `rgp_violation_echange` | conversation d'aide (pseudonymisée) : `violation_id`, `etablissement_id`, `rang`, `role` (`utilisateur`/`assistant`), `texte_pseudonymise`, `le` |
| `rgp_violation_plateforme` **P** | `id`, `declaree_le`, `description`, `etat` |
| `rgp_violation_plateforme_etab` | `violation_plateforme_id`, `etablissement_id`, `informe_le`, `elements` (texte propre à l'établissement) |

### 17.7 Registre des tables (code)
`Retention` devient `Retention::{Ephemere, Categorie(&'static str)}` ; `export_portabilite` est remplacé par `extrait: Extrait::{Fournisseur(&'static str), Hors(&'static str /* raison */)}` ; tests R-RG12 à R-RG14. Classement des nouvelles tables : `acc_trace` « par établissement », catégorie `acc.trace`, **neutralisation : aucune** (une trace restaurée est vraie) ; `cons_execution` « à rejouer » (nouvelle neutralisation `Rejeu`, R-RG22) ; `rgp_soustraitant*`, `cons_categorie*`, `rgp_violation_plateforme` en `ReferentielGlobal`/`Technique`.

### 17.8 Exceptions D-036
**Aucune nouvelle** : pas de JSON, pas de tableau, pas de `BYTEA`. Les instantanés (versions du registre, mentions, AIPD, extraits) sont des **documents figés du socle 7** (PDF), pas des colonnes JSON. Contrôle de revue : `grep -i "json\|bytea\|\[\]"` sur 2540–2559.

---

## 18. Notifications et écrans

### 18.1 Natures déclarées au socle 8 [R]
`rgpd.demande_accusee` (demandeur), `rgpd.demande_prolongee`, `rgpd.extrait_pret`, `rgpd.demande_echeance_proche` (gestionnaire, DPO ; J-10, J-3), `rgpd.violation_declaree` (direction, DPO, référent : **urgent, hors plage calme**), `rgpd.violation_echeance_72h` (H-24, H-6), `rgpd.personnes_violation` (personnes, N3), `conservation.lot_a_relire`, `conservation.categorie_non_validee` (DPO, à chaque fenêtre), `conservation.gel_a_revoir`, `conformite.fiche_a_revoir`, `plateforme.soustraitant_annonce` (direction, DPO), `scolarite.acces_fermeture` (famille d'un élève sorti), `acces.alerte` (direction, DPO).

### 18.2 Écrans (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`)
1. **Conformité — accueil** (tuiles Bento) : points ouverts par gravité ; « prêt pour les données réelles » ; demandes (en cours, en retard) ; violations ouvertes avec horloge ; prochaines échéances de conservation ; dernière revue annuelle.
2. **Registre** : liste des traitements (état), fiche (sections calculées en gris, sections saisies en blanc, écarts en orange), comparaison de versions, export.
3. **Conservation** : catalogue (catégorie, règle, source, état), écran de validation en bloc pour le DPO, échéances, gels, lots (échantillon côte à côte, bordereau, visa, exécution, certificat).
4. **Demandes** : file avec échéances (tableau dense), fiche demande (horloge, demandeur, qualité, droits, extrait par sections, masquages à relire surlignés, réponse, remise).
5. **Violations** : déclaration en un écran (trois champs et « envoyer »), fiche (horloge, qualification, personnes, risque, notification, mesures, chronologie), registre.
6. **Sous-traitants** et **dossier de la plateforme**.
7. **Documents** et **AIPD**.
8. **Famille / élève : « Vos données »** (mentions, consultations, demandes, contact du DPO), sobre, lisible sur téléphone.
9. **Fiche d'un objet** : bandeau discret « conservé jusqu'au … — pourquoi ? » pour les gestionnaires (RGP-138).

### 18.3 Mobile (socle 11)
Famille et élève : page « Vos données », suivi des demandes, consultations. Personnels : **déclarer un incident** (RGP-240), en un geste. Direction : alerte de violation et horloge. Rien d'autre (les tâches complexes restent sur le web, D-003).

---

## 19. Reprise du module RGPD de Next.js (D-035)
Classement T0 : **à conserver** (registre des incidents et documents validés), le reste ventilé.
1. Source : collections `rgpd__workspace` et `rgpd__incidents` (`ent_collection_*`, T0 §3.3) et objets S3 des documents importés et générés (`rgpd-storage.ts`).
2. **Ventilation** (D-036) : réponses du questionnaire → `rgp_questionnaire_reponse` (une ligne par question, clé inconnue = refus listé) ; identité → `rgp_gouvernance` (personnes rapprochées par e-mail, §6.5 de T0) ; états des documents → `rgp_document`, fichiers → socle 7 avec empreinte ; incidents → `rgp_violation` + `rgp_violation_etape` (chronologie) ; **historique de conversation IA non repris** (contient potentiellement des données personnelles non pseudonymisées ; seul le résumé utile, s'il existe, est repris en étape) ; historique des actions (100 dernières) → étapes « reprises de l'ancien outil ».
3. Sous-traitants déclarés (AWS, better-auth…) : **non repris** comme sous-traitants ScolIA ; listés au rapport pour que l'établissement décide (déclarations locales).
4. Score : non repris.
5. Contrôles : nombre d'incidents, dates, documents (empreintes), rapport à Florian (`TRANSITION_T0_INVENTAIRE.md` §7.2).
6. Moment : avec la mise en service réelle (lot R11 de T0), le module Next.js reste consultable en lecture seule.

---

## 20. Lots de code (équipe Sonnet)

| Lot | Contenu | Dépend de | Migrations | Quand |
|---|---|---|---|---|
| **G0** | **Correctifs urgents** : traces sensibles transactionnelles et bloquantes (RG-D01, RGP-190) ; `BEFORE TRUNCATE` sur `ent_journal_modif` (RG-D02) ; test du registre qui échoue sans base sous `CI`/`ENT_EXIGER_BASE` (RG-D03) ; `Retention::Categorie(code)` et catégorie posée sur les 349 tables (sans durée appliquée) + test R-RG12 ; fournisseurs d'extrait déclarés (squelette) + test R-RG14 ; retrait du mot de passe du commentaire (RG-D10) ; procédures écrites (demandes de droits, violation) publiées dans `docs/` en attendant les écrans | — | **2540** | avant la démo de décembre si possible ; **obligatoire avant toute donnée réelle** |
| G1 | Référentiel : catalogue initial (§8.2 bis), règles par établissement, validation DPO, politique générée, « pourquoi garde-t-on ? » ; tableau des sanctions à effacer (en attendant D7) | G0 | 2541 | janvier 2027 |
| G2 | Gouvernance, registre généré, écarts, sous-traitants (plateforme et locaux), mentions, page « Vos données », points ouverts, « prêt pour les données réelles » ; reprise Next.js | G1, socle 28 (MS2) | 2542, 2543 | février 2027 |
| G3 | Guichet des droits : demandes, identité, horloge, extrait assemblé, masquages, remise chiffrée, rectification, effacement décidé (exécution en G6), limitation, opposition | G0 (fournisseurs), socles 2, 4, 7 | 2544, 2545 | février–mars 2027 |
| G4 | Traces d'accès séparées et partitionnées, motifs hors périmètre, journal visible famille et élève majeur, contrôles internes, alertes, purge par procédure | G0 | 2546 | mars 2027 |
| G5 | Violations : déclaration (web et mobile), horloge, qualification, personnes calculées, notification préparée, information des personnes, violation de plateforme propagée, registre | G2 | 2547 | mars 2027 |
| G6 | Moteur de conservation : échéances, gels, conservation intermédiaire, fin d'année, sortie d'élève, effacements accordés, exécuteur des sanctions (avec 13 D7) | G1, exécuteurs des socles | 2548, 2549 | printemps 2027 |
| G7 | Lots : proposition, échantillon, double validation, bordereau, visa | G6 | 2550 | été 2027 |
| G8 | Exécution, anonymisation, certificat, registre des effacements, rejeu après restauration (avec le PRA) | G7, PRA | 2551 | été 2027 (aucune échéance avant 2028, R-RG2) |
| G9 | Questionnaire pré-rempli, AIPD assistée, catalogue de documents, revue annuelle, rapport annuel | G2 | 2552 | printemps 2027 |
| G10 | Multi-établissements (DPO consolidé, demandes éclatées), comptes inactifs (couche ScolIA), fin de contrat et attestation, réquisitions | G3, PORTABILITE EXP-3 | 2553 | 2027-2028 |
| G11 | IA : brouillons, analyse de documents, assistant d'incident pseudonymisé, jeu d'évaluation | G3, G5, socle 30 | — | 2027-2028 |
| G12 | Archives : versement SEDA, conservation définitive, tri, communicabilité | G8, avis du juriste | 2554 | 2028 |

**Parcours minimal avant les premières données réelles (année de doublon)** : G0, G1, G2, G3, G4, G5. Les lots G6–G8 et G12 suivent (R-RG2).

---

## 21. Migrations (plage imposée **2540–2559**, registre `MIGRATIONS.md`)

| N° | Contenu |
|---|---|
| **2540** | G0 : déclencheur `BEFORE TRUNCATE` (niveau instruction) sur `ent_journal_modif` ; aucune autre modification de schéma (les traces bloquantes et le registre sont du code) |
| 2541 | G1 : `cons_categorie`, `cons_categorie_proposition` (P), `cons_regle` |
| 2542 | G2 : `rgp_gouvernance`, `rgp_traitement`, `rgp_traitement_domaine`, `rgp_traitement_version`, `rgp_ecart`, `rgp_destinataire_externe`, `rgp_mention`, `rgp_document` |
| 2543 | G2 : `rgp_soustraitant`, `rgp_soustraitant_donnee`, `rgp_soustraitant_annonce` (P), `rgp_soustraitant_objection`, `rgp_soustraitant_local` |
| 2544 | G3 : `rgp_demande`, `rgp_demande_droit`, `rgp_demande_identite`, compteur de numéros |
| 2545 | G3 : `rgp_extrait`, `rgp_extrait_section`, `rgp_extrait_masquage`, `rgp_requisition` |
| 2546 | G4 : `acc_trace` partitionnée, `acc_controle`, `acc_alerte` ; reprise des traces de consultation existantes de `ent_journal_modif` (copie, sans les supprimer du journal en ajout seul) |
| 2547 | G5 : `rgp_violation`, `rgp_violation_categorie`, `rgp_violation_mesure`, `rgp_violation_etape`, `rgp_violation_echange`, `rgp_violation_plateforme` (P), `rgp_violation_plateforme_etab` |
| 2548 | G6 : `cons_echeance`, `cons_gel`, `cons_intermediaire` |
| 2549 | G6 : `cons_execution` (registre des effacements) |
| 2550 | G7 : `cons_lot`, `cons_lot_objet`, `cons_lot_validation`, `cons_visa` |
| 2551 | G8 : contraintes et déclencheurs d'immuabilité des lots exécutés |
| 2552 | G9 : `rgp_aipd`, `rgp_aipd_critere`, `rgp_questionnaire_reponse` |
| 2553 | G10 : colonnes de fin de contrat (attestation) si non portées par le 28 |
| 2554 | G12 : `arc_versement`, `arc_versement_objet`, `arc_definitif` |
| 2555 | ajout personas : procédure d'anonymisation du journal (`SECURITY DEFINER`, drapeau de lot vérifié par le déclencheur de 091) (§26.1 A) |
| 2556 | ajout personas : `rgp_demande_sujet`, `cons_visa_exclusion`, `cons_categorie.source_ligne` (§26.6) |
| 2557 | réserve |
| **2558** | RLS stricte de toutes les tables `rgp_`, `cons_`, `acc_`, `arc_` |
| **2559** | réservé : retrait éventuel des traces de consultation de `ent_journal_modif` **seulement après décision de Florian** et durée échue (E8 : migration destructrice) |

Aucun fichier de migration écrit par cette conception.

---

## 22. Tests et recette

- **Garde CI** : R-RG12, R-RG13, R-RG14 ; registre qui échoue sans base en CI (RG-D03).
- **Traces** : une lecture sensible dont la trace échoue (déclencheur de test qui refuse l'insertion) renvoie une erreur et rien n'est servi.
- **Journal** : `TRUNCATE` refusé ; `UPDATE`/`DELETE` refusés (existant).
- **Isolation** : chaque table dans la matrice ; un DPO de l'établissement A ne voit rien de B ; une demande ne voit que les fournisseurs de son établissement ; la violation de plateforme ne montre à A que les éléments de A.
- **Extraits** : jeu fictif « parents séparés + adresse protégée » : l'extrait du parent 1 ne contient aucune donnée propre du parent 2 ni l'adresse protégée (CL27-01, CL27-02) ; fournisseur en échec → extrait incomplet non remettable.
- **Conservation** : catégorie non validée → aucun objet dans un lot ; objet gelé ou avec demande ouverte → exclu ; double validation par deux personnes distinctes, refus si même personne avec deux comptes ; lot à visa sans visa → refus ; exécution idempotente ; rejeu après restauration (base restaurée avant un lot → objets détruits à nouveau).
- **Anonymisation** : seuil de 5 ; aucune valeur d'identité résiduelle (recherche des noms du jeu fictif après exécution).
- **Violations** : horloge à +72 h exacte, fuseau Europe/Paris ; notification tardive acceptée avec motif ; IA : texte envoyé au modèle sans aucun nom du jeu fictif (test de pseudonymisation).
- **Recette personas** : `PERSONAS_27.md`.

---

## 23. Réglages signalés (D-023) — S-540 à S-553

| N° | Réglage | Défaut |
|---|---|---|
| S-540 | Fenêtres de proposition des lots | juillet et janvier |
| S-541 | Durée des traces d'accès | 12 mois (borne 6 à 36) |
| S-542 | Liste des motifs de consultation hors périmètre | 6 motifs (suivi éducatif, remplacement, conseil de classe, demande de la famille, urgence, autre + texte) |
| S-543 | Seuil d'alerte de consultations en rafale | 30 dossiers hors périmètre en 10 min |
| S-544 | Délai de rappel des demandes | J-10, J-3 |
| S-545 | Fermeture de l'accès famille après la sortie | 3 mois [R] |
| S-546 | Fermeture du compte élève après la sortie | 3 mois |
| S-547 | Délai d'objection à un nouveau sous-traitant | 30 jours |
| S-548 | Validation par deux personnes pour d'autres catégories que les dossiers d'élèves | non |
| S-549 | Afficher le nom des agents dans le journal des familles | non (Q-RG4) |
| S-550 | Information des titulaires quand l'élève mineur fait une demande | oui, sauf motif |
| S-551 | Durée d'inactivité d'un compte avant anonymisation (couche ScolIA) | **3 mois** après la fin du dernier rattachement, avec avis préalable (ajout personas, aligné sur le SDET 2024 ; était 12 mois) |
| S-552 | Seuil d'anonymisation | 5 personnes |
| S-553 | Taille de l'échantillon d'un lot | 20 objets ou 5 % |
| S-554 | Seuils d'alerte de consultations par rôle (ajout personas) | AED : 15 dossiers hors périmètre en 10 min ; direction et CPE : sans objet |

---

## 24. Décisions appliquées par défaut (D-031) et questions à Florian

### 24.1 Appliquées par défaut (D-031, à valider par Florian)
1. Le 27 possède le référentiel, les décisions et les preuves ; les socles exécutent (R-RG3).
2. `Retention::Categorie` dans le registre des tables, test CI bloquant (R-RG12).
3. Rien n'est détruit sans règle validée par le DPO ni lot validé par un humain (R-RG1, R-RG4).
4. Privé sous contrat traité comme le public pour le visa, en attendant le juriste (R-RG5).
5. Deux personnes pour détruire des dossiers d'élèves (R-RG7).
6. Traces sensibles bloquantes (R-RG8), lot G0 avant toute donnée réelle.
7. Traces d'accès séparées, partitionnées, 12 mois par défaut (R-RG30, S-541).
8. Journal des familles par fonction et service, nom sur arbitrage (§11.3, S-549).
9. Journal non utilisé pour évaluer les personnels (R-RG27).
10. Consultations de santé et psy hors du journal standard (R-RG29).
11. Extrait assemblé par fournisseurs, relu par un humain, remis chiffré (R-RG15, R-RG18).
12. Extrait réutilisable remis même hors portabilité (RGP-163).
13. Pièces d'identité détruites à la clôture (R-RG16).
14. Demande ouverte = gel (R-RG19).
15. Registre généré à partir des faits, « à revoir » automatique (R-RG10, R-RG11).
16. Points ouverts au lieu d'un score (RG-D15).
17. Mentions par public, deux versions pour les élèves, page publique (§8.11).
18. Violation déclarable par tous, en 30 secondes, ton neutre (RGP-240).
19. Horloge des 72 h non bloquante (R-RG21).
20. IA pseudonymisée, jamais décisionnaire, jamais sur la santé (R-RG33, R-RG34).
21. Sous-traitants annoncés 30 jours avant (R-RG35, S-547).
22. Rejeu des effacements et rectifications après restauration (R-RG22, R-RG25).
23. ScolIA ne se présente pas comme SAE (R-RG23).
24. Fin de contrat : archives remises, jamais détruites sans visa (R-RG24).
25. Pas de destruction réelle nécessaire pendant le doublon ; G6–G8 après le parcours minimal (R-RG2).
26. Reprise Next.js : incidents et documents gardés, conversation IA non reprise, sous-traitants non repris (§19).
27. Accès famille fermé 3 mois après la sortie, avec téléchargement proposé avant (S-545).
28. Comptes ScolIA anonymisés 3 mois après la fin du dernier rattachement (S-551, corrigé par la passe personas) — **sous réserve** de Q-RG6.

### 24.2 Soumis à Florian (engagent l'établissement ou l'éditeur à l'extérieur)
| N° | Question | Recommandation |
|---|---|---|
| Q-RG1 | Dans l'établissement pilote (privé sous contrat), qui est responsable de traitement : le chef d'établissement, l'OGEC, ou une répartition par traitement ? | répartition écrite par traitement (scolarité : établissement ; RH et finances : OGEC), signée avec l'accord de A8 |
| Q-RG2 | Avis du **juriste** sur L211-4 2° pour le privé sous contrat (dossiers scolaires = archives publiques ?) | à demander avant la première élimination (B10) ; d'ici là R-RG5 |
| Q-RG3 | Avis du **DPO** : quand un élève mineur demande ses données, informe-t-on ses parents ? | oui sauf motif (S-550) |
| Q-RG4 | Nom des agents dans le journal des familles | non (fonction et service) ; arbitrage au cas par cas |
| Q-RG5 | **DPO** de l'établissement pilote : qui (diocèse, DDEC, cabinet, interne) ? | à désigner avant l'année de doublon (A8, B4) ; sans DPO, pas de validation des durées |
| Q-RG6 | ScolIA **responsable de traitement** de la couche compte (D-013) : l'assumer (registre, mentions, DPO de l'éditeur) ? | oui ; implique un DPO côté éditeur à la commercialisation |
| Q-RG7 | **Contrat de sous-traitance** type (art. 28) et liste des sous-traitants de la plateforme à signer avec l'établissement pilote | à rédiger avec un juriste avant les données réelles (B3) |
| Q-RG8 | Référent « violation » côté éditeur (24 h/24 ?) | Florian au départ ; à revoir à la commercialisation |
| Q-RG9 | Service d'archives compétent pour l'établissement pilote (archives départementales ? diocésaines ?) | à identifier avec le juriste (Q-RG2) |
| Q-RG10 | Durée de conservation des demandes de droits et du registre des violations | 5 ans [R], DPO |
| Q-RG11 | Conservation définitive dans ScolIA : ScolIA n'étant pas agréé (L212-4), les archives définitives publiques doivent-elles toutes être versées (aucune conservation définitive chez l'éditeur) ? | oui pour le public ; privé sous contrat selon Q-RG2 ; juriste |

---

## 25. Faits, hypothèses, contradictions

**Faits [F]** : inventaire du §2.1 (chemins et lignes relevés le 2026-11-11 sur `ent-mc0hoe`) ; module Next.js du §2.2 (lecture ciblée). Rien n'a été compilé ni exécuté pour cette conception.
**Hypothèses [H]** : DPO mutualisé ; répartition établissement / OGEC ; modèle de registre CNIL ; notification CNIL par téléservice ; ScolIA n'est pas un SAE ; parent sans exercice de l'autorité.
**Contradictions et écarts signalés**
- `07_DOCUMENTS.md` §13.6 fait porter au socle 7 une table `ent_doc_regle_conservation` « dès sa livraison » ; le 27 prévoit `cons_regle` commun : la table du 7 devient une **projection** de `cons_regle` pour les documents (consommateur), comme le 07 le prévoit lui-même ; à ne pas doubler la saisie.
- `registre_donnees.rs` porte `export_portabilite` (RGPD art. 20) : or l'art. 20 **ne s'applique pas** à la mission d'intérêt public (20.3) ; le drapeau est remplacé par « fournisseur d'extrait » (droit d'accès, art. 15), plus juste.
- `PORTABILITE_EXPORTS.md` §8 laisse ouverte la durée du journal des exports (« 3 ans ? ») : rattachée ici à `acc.trace_controle` / `rgpd.demande` selon la nature, décision du DPO.
- `MISE_EN_SERVICE_DONNEES.md` A11 propose « journaux d'accès 6 mois à 1 an » : retenu 12 mois (S-541).
- Le socle 13 réserve **2261** pour son propre « registre des effacements » : le 27 fournit le registre commun `cons_execution` (2549) ; le 13 peut garder le sien ou s'y brancher — à harmoniser à l'implémentation (préférence : un seul registre, celui du 27, le 13 y écrit par le port).

### 4.10 Vérification du 2026-11-11 (sous-agent, sources officielles)
Accès : codes.droit.org et EUR-Lex refusés ce jour, BO HTML en 403 ; articles de Légifrance lus par extraits (citations tronquées) ; RGPD lu sur cnil.fr (chapitres 2, 3, 4, 9) ; annexe du BO n° 24 de 2005, SDET 2024 et guide du développeur CNIL lus en entier.

| Point | Source | Constat | Effet sur la conception |
|---|---|---|---|
| Art. 12.3, 12.5, 15.3, 17.3 b et d, 20.1 et 20.3, 28.3 b, g, h, 30.1 et 30.2, 33.1 à 33.5, 34, 35 | RGPD, cnil.fr [V] | délais, exceptions, contenu du registre et de la notification **confirmés** ; 33.1 : notification tardive « accompagnée des motifs du retard » ; 28.3 g : suppression ou restitution « à moins que le droit … n'exige la conservation » | §4.1 confirmé ; R-RG21, R-RG24 fondés |
| Âge et information des mineurs | LIL art. 45 et 48 [V] | 15 ans : seulement le consentement aux services en ligne ; moins de 15 ans : information « dans un langage clair et facilement accessible » | §8.11 (deux versions de mentions) fondé ; **aucun texte** sur l'exercice des droits par le mineur seul (Q-RG3 maintenue) |
| Archives publiques du privé | C. patr. L211-4 2° (loi 2016-925) [V-ext] | documents « qui procèdent de la gestion d'un service public ou de l'exercice d'une mission de service public » par des personnes de droit privé | R-RG5 maintenue ; Q-RG2 au juriste |
| Sélection et données personnelles | C. patr. L212-2, **L212-3** [V-ext] | sélection à l'expiration de la durée prévue par la LIL ; catégories à éliminer « fixées par accord » avec l'administration des archives | lots et bordereaux (§8.4) fondés |
| Dépôt externalisé | C. patr. L212-4 [V-ext] | dépôt possible auprès de personnes **agréées** par l'administration des archives | ScolIA, **non agréé**, ne doit pas se présenter comme dépositaire d'archives publiques définitives (renforce R-RG23) ; Q-RG11 |
| Communicabilité | C. patr. L213-1, L213-2 [V-ext] | 50, 25 (après décès), 75, 100 ans selon les cas | `arc_definitif.communicable_le` |
| Visa | C. patr. R212-14 [NT ce jour ; V le 2026-11-09, VERIF R40] | — | inchangé |
| Instruction de tri des EPLE | DAF/DPACI/RES/2005/003, BO n° 24 du 16-6-2005 [V] | dossiers scolaires 50 ans, 10 ans s'il existe un récapitulatif, tri, « ne doit pas être remis à l'élève » ; registres d'appel 10 ans (D) ; bulletins d'absence 1 an (D) ; conseils de discipline 10 ans (T, PV) ; **registre des sanctions 10 ans (C)** ; cahiers de punitions 1 an (D) ; bourses : dossiers 2 ans (D), listings 5 ans (C) ; budgets et comptes financiers 10 ans (C) ; règles valables « quel que soit le support » | catalogue §8.2 bis complété ; **le dossier scolaire original n'est pas remis à l'élève** : l'extrait est une copie (§9.4) |
| SDET 2024, annexe opérationnelle | eduscol [V] | renvoie à l'instruction de 2005 ; **cahier de textes 7 ans** (2 en base active + 5 en archivage) ; journaux conservés **12 mois au plus** ; base active : l'année ou le cycle ; **suppression 3 mois après la fin du compte** ; journalisation des usagers et des personnels techniques (prise de main à distance comprise) ; clause art. 28 et clause de réversibilité ; cite un « guide d'accompagnement sur la conservation et l'archivage des données des ENT » (non localisé) | `cdt.contenu` corrigé ; S-541 = 12 mois cohérent ; S-545/S-546 = 3 mois cohérents ; journalisation du support (socle 28) fondée |
| Durées de conservation | CNIL, « Les durées de conservation » [V] | trois phases (base active, archivage intermédiaire, archivage définitif) ; séparation physique ou logique ; accès réservé à des personnes spécifiquement habilitées ; pas de référentiel pour l'enseignement scolaire | §6 et §8.3 conformes |
| Anonymisation | CNIL [V] | « rendre impossible, en pratique, toute identification … de manière irréversible » ; distincte de la pseudonymisation | §8.4.6 : un jeu pseudonymisé reste une donnée personnelle (RGP-294) |
| Données réelles en test | guide RGPD du développeur CNIL, fiche 11 [V] | données réelles « ne doivent pas être utilisées » ; jeu fictif ; à défaut, mêmes mesures qu'en production | socle 28 (démo, recette, reprise) |
| Sanctions | C. éduc. R511-13 IV, **modifié par le décret 2026-637 du 16-7-2026** [V-ext] | mêmes paliers ; effacement sur demande au changement d'établissement ; toutes au terme de la scolarité dans le second degré | exécuteur du 13 sur la **version en vigueur** ; signalé au socle 13 |
| Assiduité | C. éduc. R131-5, R131-6 [V-ext] | registre d'appel public et privé ; dossier des absences « ouvert pour la seule année scolaire » | `vs.*` |
| Livret scolaire | C. éduc. D311-9 [V-ext] | transmis aux établissements publics ou privés sous contrat jusqu'à la fin de la scolarité obligatoire | transmission (RGP-179, socle 7) |
| CADA | CRPA L300-2, L311-6, R311-12, -13, -15 [V-ext] | voir §4.4 | qualification RGPD / CADA (RGP-141) |
| Accessibilité | loi 2005-102 art. 47, décret 2019-768 [NT] | — | socle 29 |

URL : https://www.cnil.fr/fr/reglement-europeen-protection-donnees (chapitres 2, 3, 4) · https://www.cnil.fr/fr/le-cadre-national/la-loi-informatique-et-libertes · https://www.legifrance.gouv.fr/jorf/article_jo/JORFARTI000032854635 (L211-4) · https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000019202835 (L212-2) · …/LEGIARTI000037825470 (L212-3) · …/LEGIARTI000045214034 (L212-4) · …/LEGIARTI000039016602 (R511-13) · …/LEGIARTI000032144475 (D311-9) · …/LEGIARTI000037269056 (L311-6) · https://www.education.gouv.fr/bo/BoAnnexes/2005/24/24.pdf · https://eduscol.education.fr/document/58941/download (SDET 2024) · https://www.francearchives.gouv.fr/article/88482501 (SEDA 2.3) · https://www.cnil.fr/fr/passer-laction/les-durees-de-conservation-des-donnees · https://www.cnil.fr/fr/technologies/lanonymisation-de-donnees-personnelles · https://github.com/LINCnil/Guide-RGPD-du-developpeur. Toutes consultées le 2026-11-11.

**Restent [NT]** : R212-14 (texte du jour), dérogation archivistique de l'art. 89, exercice des droits par le mineur seul, instruction de tri postérieure à 2005, guide ENT de conservation cité par le SDET (piste prioritaire pour le DPO), référentiel CNIL scolaire, accessibilité, texte sur la transmission du dossier administratif, NF Z42-013.

---

## 26. Ajout personas (`PERSONAS_27.md`, 2026-11-11) — l'emporte en cas de contradiction

### 26.1 Cinq bloquants tranchés (D-031)
- **A — Journal et sorts finaux.** Le journal en ajout seul (091) n'autorise aucune anonymisation. **Procédure dédiée** : fonction `SECURITY DEFINER` réservée à l'exécuteur de conservation, sous drapeau de transaction `ent.conservation_lot` vérifié par le déclencheur ; un seul `UPDATE` permis par ligne (instantané et libellé neutralisés, `eleve_id` à `NULL`) ; comptage dans `cons_execution`. CL27-37 est précisé en conséquence (R-RG41, migration 2555).
- **B — Remise.** L'extrait n'est remis que sur un canal **déjà connu** du dossier ou en main propre contre pièce ; passkey récente du demandeur pour une remise en ligne ; jamais à une adresse fournie par la demande ; une demande d'un inconnu ne confirme pas la scolarisation (R-RG42).
- **C — Doublon.** Traitement « essai en parallèle » au registre, catégorie `doublon.copie`, durée jusqu'à la décision de bascule + 3 mois, procédure de fin d'essai (R-RG43, Q-RG12).
- **D — Certificats médicaux.** Tant que HDS est faux, un justificatif médical n'est pas conservé : mention « justificatif médical présenté le … » ; signalé au socle 12 (R-RG44, Q-RG13).
- **E — Pièces et journal.** Pièces concernant un tiers exclues et listées (caviardage manuel relu possible) ; sections protégées sous code générique au journal, jamais visibles du parent visé (R-RG45, R-RG46).

### 26.2 Actions ajoutées (RGP-300 à RGP-316)
| N° | Action | Acteur · droit | Règles | Lot |
|---|---|---|---|---|
| RGP-300 | Résumé en une phrase de chaque traitement, généré et relu | SYS, DIR | — | G2 |
| RGP-301 | Désigner le responsable de chaque traitement et exporter la répartition (annexe à signer) | DIR | Q-RG1 | G2 |
| RGP-302 | Demande à plusieurs sujets (fratrie), un extrait par sujet, réponse commune | ADF | `rgp_demande_sujet` | G3 |
| RGP-303 | Distinguer « reçu le » et « saisi le » ; alerte si l'écart dépasse 5 jours | ADF | — | G3 |
| RGP-304 | Modèles de réponse par cas fréquent | ADF | éditables | G3 |
| RGP-305 | Regénérer les documents émis pour l'extrait, champs de tiers masqués | SYS | R-RG47 | G3 |
| RGP-306 | Demande de retrait de photo par un élève mineur : limitation provisoire, décision des titulaires | ADF | socle 18 | G3 |
| RGP-307 | Grille de vérification d'identité d'un ancien élève sans compte | ADF | — | G3 |
| RGP-308 | Bordereau complet (ligne du tableau de tri, dates extrêmes, volume, mode de destruction) | SYS | — | G7 |
| RGP-309 | Visa partiel : exclure des objets du lot, motif | DIR, ADF | `cons_visa_exclusion` | G7 |
| RGP-310 | Prendre en compte les périmètres temporaires (remplacement, conseil, sortie, permanence) | SYS | R-RG49 | G4 |
| RGP-311 | Notification CNIL par ScolIA pour la couche compte | EDI | R-RG50 | G5 |
| RGP-312 | Modèle de message du sous-traitant aux établissements (33.2) | EDI | — | G5 |
| RGP-313 | Trois catégories d'IBAN (mandat actif, mandat révoqué, sans mandat) | SYS | durée [NV] | G1 |
| RGP-314 | Dossier de contrôle en un clic (sans données d'élèves) | DIR, DPO | traces d'un dossier seulement sur demande | G9 |
| RGP-315 | Liste datée et signée de la revue des habilitations | DIR | A12 | G9 |
| RGP-316 | Demande CADA d'un tiers : document agrégé ou occulté | DIR | CRPA L311-6 | G3 |

### 26.3 Règles ajoutées (R-RG41 à R-RG50)
| N° | Règle |
|---|---|
| R-RG41 | Le journal et les traces ne s'anonymisent que par la procédure dédiée, sous un lot validé ; tout autre `UPDATE` reste refusé. |
| R-RG42 | Un extrait n'est remis que sur un canal déjà connu du dossier ou en main propre, avec passkey récente en ligne ; une demande d'un inconnu ne confirme pas la scolarisation. |
| R-RG43 | L'essai en parallèle de 2027-2028 est un traitement déclaré, borné dans le temps, avec procédure de fin d'essai. |
| R-RG44 | Sans HDS, aucun certificat médical n'est conservé ; seule la mention de sa présentation l'est. |
| R-RG45 | Une pièce qui concerne un tiers n'est pas remise ; elle est listée avec le motif. |
| R-RG46 | Le journal visible n'expose jamais l'existence d'une protection au parent qu'elle vise. |
| R-RG47 | Les fichiers remis sont nettoyés de leurs métadonnées ; les documents émis sont regénérés sans les données de tiers. |
| R-RG48 | L'extrait remis à un élève ne contient pas les données propres de ses parents au-delà de leur identité de responsables. |
| R-RG49 | Un geste couvert par un périmètre temporaire connu n'est pas « hors périmètre ». |
| R-RG50 | Une violation qui touche la couche compte est notifiée par ScolIA pour sa part, en plus de l'information des établissements. |

### 26.4 Cas limites ajoutés (CL27-63 à CL27-70)
| N° | Cas | Réponse |
|---|---|---|
| CL27-63 | Changement de chef d'établissement | validations maintenues ; relecture proposée |
| CL27-64 | Courrier reçu pendant la fermeture estivale | délai inchangé ; relève du courrier ou information du DPO (procédure G0) |
| CL27-65 | Demande « au nom de l'enfant » depuis le compte du père | traitée comme demande du père |
| CL27-66 | Ancien élève d'un établissement qui a quitté ScolIA | renvoyé vers l'établissement ; attestation de suppression |
| CL27-67 | Compte de parent volé qui demande un extrait | passkey récente exigée pour la remise (R-RG42) |
| CL27-68 | Texte de santé dans un motif d'absence libre | suggestion de remplacement, jamais silencieuse |
| CL27-69 | Données d'affectation d'une AESH (classées santé) | extrait sans nom de l'élève |
| CL27-70 | Demande reçue au guichet sur tablette | photo de la pièce, détruite à la clôture |

### 26.5 Corrections
- S-551 : **3 mois** (SDET 2024).
- §8.2 bis : `fin.iban` remplacé par `fin.iban_mandat_actif` (durée du mandat), `fin.iban_mandat_revoque` ([NV] délai de contestation, expert-comptable), `fin.iban_sans_mandat` (destruction à la fin de l'usage) ; ajout de `doublon.copie` (fin de l'essai + 3 mois, R-RG43).
- §9.4 point 5 : remise conforme à R-RG42.
- CL27-37 : par la procédure R-RG41.

### 26.6 Modèle de données ajouté
`rgp_demande_sujet` (`demande_id`, `etablissement_id`, `sujet_type`, `sujet_id`, `extrait_id`) — `rgp_demande.sujet_*` devient le sujet principal ; `cons_visa_exclusion` (`lot_id`, `etablissement_id`, `objet_type`, `objet_id`, `motif`, `suite` : `verser`/`garder`) ; `cons_categorie.source_ligne` (référence de la ligne du tableau de tri) ; `rgp_demande.recue_le` distinct de `cree_le`. Aucune colonne JSON.

### 26.7 Lots
G0 ajoute : signalement M-2729 au socle 12, procédures écrites conformes à R-RG42. G3 ajoute RGP-302 à RGP-307, RGP-316. G4 ajoute RGP-310. G7 ajoute RGP-308, RGP-309. G8 ajoute la procédure 2555. G9 ajoute RGP-314, RGP-315.

### 26.8 Migrations
2555 et 2556 attribuées (§21) ; 2557 en réserve.

### 26.9 Bilan chiffré après la passe
217 actions (RGP-100 à RGP-316), 70 cas limites, 50 règles, 15 réglages (S-540 à S-554), 13 lots.
