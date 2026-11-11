# Socle 14 — Accueil, sorties et sûreté

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** le code. Aucun code n'a été écrit ni modifié. Aucun accès à une base, à un secret ou à un environnement de production. Le code Next.js a été **lu seulement**, de façon ciblée (module `accueil`), sur la copie de travail de la branche (rien exécuté, rien construit, rien modifié).
**Branche :** `ent-socle-14-conception`, partie de `origin/ent-mc0hoe` (`7467f2d`).
**Passe personas (même branche) :** `docs/socles/PERSONAS_14.md`. Ajouts faits **ici**, marqués **« ajout personas »**, réunis au **§32**. **En cas de contradiction, le §32 l'emporte** ; rien n'a été supprimé.
**Périmètre** (`SOCLES.md` §14) : accueil (registre des visiteurs, badge visiteur, appel d'un parent transformé en déclaration d'absence) ; régimes et autorisations de sortie ; contrôle aux portes ; **remise de l'élève à une personne autorisée** (vérifiée contre le socle 4) ; sortie anticipée ; **mise en sûreté et évacuation** (PPMS : liste des présents en temps réel, appel d'urgence, compte rendu d'exercice) ; main courante. **Hors périmètre** : sorties et voyages collectifs (42), internat (41), absences des personnels (15), incidents et sanctions (13).
**Lien avec le socle 42 :** le 42 possède les sorties **collectives** et ses droits sont `voyages.*` (42 §33.2, V0a-1) ; le 14 garde `sorties.gerer` et `sorties.controler` (sorties **individuelles**). Le 42 consomme du 14 la **remise au retour** (`accueil::remise_preparer`, VOY-207) et la **liste des présents** en alerte (`surete::presents`) ; le 14 consomme du 42 le port « en sortie » (garde-fou de l'accueil, comptage).
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-017, D-021, D-023, D-031, **D-034**, **D-035** (et ses deux précisions), **D-036**, D-037/D-038 (proposées) ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; `docs/TRANSITION_T0_INVENTAIRE.md` §3.0 (module « Accueil (saisie d'absences) », maturité **Partiel**), Q-T0-07 ; `docs/SOCLE2_DOMAINES.md` VS-06, VS-12, INT-05 à INT-07 ; conceptions `04_ADMINISTRATIF.md` (§8.8 remise et interdictions, ADM-41 à 45, ADM-87 à 91), `06_PERSONNELS.md` (PRS-135, 136), `08_NOTIFICATIONS.md` (§8.5, §14 urgence, NTF-85 à 93, 139 à 143, R-N60), `10_EMPLOI_DU_TEMPS.md` (ports `edt::*`, EDT-177, EDT-229), `16_MESSAGERIE.md` (R-M52), `34_SANTE.md` (SAN-142, SAN-312, CS-18), `42_SORTIES_VOYAGES.md` (§15.4, §21, §33), `IA_COEUR.md` (§5, §6.2). Modèles de forme : `10_EMPLOI_DU_TEMPS.md`, `21_EVALUATIONS.md`, `42_SORTIES_VOYAGES.md`.

**Conventions.** **[F]** fait vérifié dans le code (Rust : chemins depuis `ent-rust/` ; Next.js : chemins depuis `app/`) ; **[V]** règle lue sur une source officielle (URL et date au §5) ; **[NV]** non vérifiée ; **[H]** hypothèse de terrain ; **[R]** recommandation, non décidée.

Numérotation propre à ce socle (préfixes vérifiés libres dans `docs/` le 2026-11-11 ; `ACC-` est pris par l'accompagnement) :

| Élément | Numéros |
|---|---|
| Actions Next.js reprises | **NXA-01**… |
| Défauts Next.js | **VA-01**… |
| Défauts de l'existant Rust | **DR-01**… |
| Actions cibles | **ACS-100** à **ACS-329** (puis ACS-330… au §32) |
| Règles | **R-AS1**… |
| Cas limites | **CAS-01**… |
| Questions | **Q-AS1**… |
| Réglages signalés | **S-1400** à **S-1429** |
| Manques de la passe personas | **M-1400** à **M-1499** |
| Lots | **AS0** à **AS14** |

**Plage de migrations imposée : 2280–2299** (inscrite dans `MIGRATIONS.md`).

**Méthode.** (1) Lecture de la mission, des décisions, de la carte, de l'inventaire T0, des exigences E1–E14 et des conceptions 42, 10, 21 et de leurs passes personas ; lecture ciblée de 04, 06, 08, 10, 16, 34. (2) État des lieux du code Rust par un sous-agent en lecture seule (migrations, domaine, dépôts, routes, catalogue, écrans Leptos et `ui.rs`). (3) Lecture ciblée du module Next.js `accueil` (types, droits, base, routes, outil d'assistant). (4) Recherche réglementaire par un sous-agent (Légifrance, BO, éduscol, SGDSN, CNIL) le 2026-11-11. (5) Aucune donnée réelle lue.

Sommaire :

| Partie | Sections |
|---|---|
| Synthèse | §0 En bref · §1 Thèse |
| Existant | §2 Rust, lot 0, Next.js, à ne pas perdre |
| Métier | §3 Acteurs · §4 La journée et l'année · §5 Réglementaire |
| Cœur | §6 Branchements · §7 Concepts |
| Parcours | §8 Visiteurs · §9 Appels à l'accueil · §10 Régimes de sortie · §11 Portes · §12 Remise · §13 Élève manquant · §14 PPMS, alertes, exercices · §15 Main courante · §16 Mobile et hors ligne |
| Règles et sécurité | §17 Cas limites · §18 IA · §19 Actions · §20 Règles · §21 Droits et minimisation |
| Technique | §22 Événements · §23 Notifications · §24 Écrans · §25 Modèle de données |
| Mise en œuvre | §26 Reprise · §27 Lots et migrations · §28 Tests |
| Décisions | §29 Réglages · §30 Décisions et questions · §31 Faits, hypothèses, contradictions |
| Ajout personas | §32 |

---

## 0. En bref

1. **Ce que le socle apporte** : savoir à chaque instant **qui est dans l'établissement, qui en sort, avec qui et pourquoi** ; et, en cas d'alerte, que **chaque adulte voie sur son téléphone qui il doit avoir avec lui**, et la cellule de crise, zone par zone, qui manque.
2. **L'existant Rust est utile mais fragile** (§2.1–2.2) : un module d'autorisations de sortie (062–065) marche dans l'ancienne interface, mais **ses requêtes ne posent pas l'établissement pour la RLS**, le contrôle au portail **ne revérifie ni les règles ni le lien du parent**, l'heure est **fournie par le client et en UTC**, le régime des demi-pensionnaires est **mal lu**, les autorisations **n'ont pas d'historique** et un élève mineur peut déposer seul une demande. **Lot AS0** (9 correctifs, migrations 2280–2281) avant toute nouvelle fonction.
3. **Next.js accueil est partiel mais utilisé** pour déclarer absences et retards (§2.3) : **10 actions reprises** (NXA-01 à NXA-10), **7 défauts** à ne pas reproduire (dont un contrôle de droits qui renvoie toujours « vrai » et un motif libre où passent des données de santé). **Tout le reste du socle n'existe nulle part** : remise tracée, visiteurs, main courante, élève manquant, PPMS, alertes, exercices.
4. **Réglementaire vérifié** (§5) : **PPMS unique** (circulaire du 8 juin 2023, BO n° 26) à adopter **au plus tard pour 2027-2028**, communiqué avant le **1er juillet**, répertoire de crise à chaque rentrée, **au moins deux exercices PPMS annoncés** (septembre-octobre, avant les vacances d'hiver), retours d'expérience avant le **1er avril** ; **évacuation incendie** dont la première dans le mois de la rentrée, consignée au registre de sécurité (R 33, CCH R. 143-44) ; **vérification d'identité** des visiteurs et contrôle visuel des sacs (Vigipirate) ; **aucun registre des visiteurs imposé**, repère CNIL de 3 mois ; **pas de biométrie** ; au **collège**, pas de sortie entre deux cours ni de sortie anticipée hors fin de demi-journée (fiche 34, circulaire 96-248) ; **aucun texte national** sur la remise à un tiers ni sur l'élève manquant : nos procédures sont des recommandations. Privé sous contrat : PPMS non obligatoire au sens de la circulaire, appliqué comme bonne pratique.
5. **Principes** (§1, §20) : décision de sortie **calculée côté serveur et explicable** ; **le 14 vérifie, le 4 sait** (aucune copie des personnes autorisées) ; **interdictions en tête, jamais le motif** ; **aucune remise forcée** ; **exceptions journalisées, allées et venues ordinaires non** ; **hors ligne borné et chiffré** ; **ScolIA ne signale jamais lui-même** aux forces de l'ordre ; alerte et exercice jamais confondus.
6. **Le plan** :

   | Élément | Volume | Numéros |
   |---|---|---|
   | Actions | **230** | ACS-100 à ACS-329 |
   | Cas limites | **63** | CAS-01 à CAS-86 |
   | Règles | 20 | R-AS1 à R-AS20 |
   | Réglages signalés | 11 | S-1400 à S-1410 |
   | Lots | **15** | AS0 à AS14 |
   | Tables nouvelles | 57 | §25, sans JSON |

   **Parcours minimal rentrée 2027** (usage réel, D-035) : AS0 → AS5 ; **AS9 (PPMS) au printemps 2027**, AS10–AS11 avant le premier exercice de septembre 2027.
7. **Décisions appliquées par défaut (D-031)** : 15 (§30.1). **Soumis à Florian** : registre de sécurité dématérialisé, réquisitions, documents réels (règlement intérieur, PPMS), reprise des déclarations Next.js, date d'arrêt de Next.js accueil, consignes du réseau, avis du DPO, matériel (§30.2).
8. **Ajout personas** (§32, `PERSONAS_14.md`, 16 personas dont des attaquants et le DPO) : cinq bloquants tranchés — **mode doublon** (déclarations reportées dans Charlemagne pendant 2027-2028), **comptage à trois voies** sans obligation d'appli personnelle, **contrôle par liste négative** à la sortie de masse (pas de QR au collège), **AIPD** avant l'usage réel, **examen unique** d'une demande d'un titulaire contre l'autre ; 19 actions ajoutées (ACS-330 à 348), 8 règles (R-AS21 à 28). Total : **249 actions**, 71 cas limites, 28 règles.

---

## 1. Thèse : savoir à chaque instant qui est là, qui sort, avec qui, et pourquoi

1. **Une décision de sortie est un calcul, pas une case.** Le droit de sortir à 15 h dépend du règlement, du choix de la famille, de l'emploi du temps **réel** du jour, de l'âge, d'une sortie exceptionnelle, d'une restriction, d'une alerte. Il est **recalculé** à chaque contrôle, côté serveur, et **explicable** (« dernier cours à 15 h, option fin de journée cochée le 2 septembre par Mme X »). On ne stocke pas « autorisé le matin : oui ».
2. **Les faits sont datés et en ajout seul.** Autorisation choisie, version de règles, sortie demandée et décidée, remise, passage refusé, visite, fait de main courante, alerte, comptage : chacun est un fait horodaté (heure du serveur, fuseau de l'établissement), jamais réécrit, corrigé par un fait suivant.
3. **Le 14 vérifie, le 4 sait.** Qui peut récupérer un enfant, qui ne le peut pas, qui est titulaire : **socle 4**, lu par port, interdictions en tête. Le 14 enregistre **à qui** l'enfant a été remis. Aucune copie locale des personnes autorisées.
4. **On enregistre les exceptions, pas les allées et venues.** Un établissement n'est pas un lieu de traçage des mineurs : par défaut, les sorties ordinaires autorisées ne sont pas journalisées nominativement ; les refus, remises, sorties exceptionnelles et retours le sont (minimisation, §21).
5. **La sûreté doit marcher quand tout le reste tombe.** Liste de comptage précalculée à la réception d'une alerte, fiche de secours du jour sur la tablette de l'accueil, plan B de la direction (socle 8) : chiffrés, effacés vite.
6. **En alerte, chaque adulte voit qui il doit avoir avec lui** (SOCLES.md §14, « faire mieux »), et la cellule de crise voit, zone par zone, qui est compté, qui manque et où il a été vu pour la dernière fois.
7. **L'humain décide, ScolIA prépare.** Aucune remise forcée, aucun signalement aux forces de l'ordre, aucune alerte, aucun message envoyé par le logiciel ou l'IA de leur propre chef.

---

## 2. Évaluation franche de l'existant

### 2.1 Côté Rust [F] (état des lieux du 2026-11-11, branche `origin/ent-mc0hoe`, lecture seule)

**Ce qui existe.** Un module d'**autorisations de sortie** issu des premiers lots (migrations 062–065), fonctionnel côté API, avec écrans dans l'ancienne interface `ui.rs` seulement (`/app/sorties` l.42129, `/famille/sorties` l.51358, vue élève l.26107), et peu de tests.

| Migration / table | Contenu | Remarques |
|---|---|---|
| `062` `ent_sortie_autorisation` | matin, midi, soir, `interdit_etablissement`, `updated_by/at` ; unique (établissement, élève) | **UPSERT écrasant** (`sortie_repo.rs:19-48`) : aucun historique, aucune date, aucune année scolaire |
| `062` `ent_sortie_demande` | jour, créneau (matin/midi/soir), motif, statut (en_attente/acceptee/refusee), décision | statut mis à jour en place ; pas d'heure précise ; pas d'annulation |
| `062` `ent_sortie_controle` | jour, créneau, verdict, source, motif, auteur | insertion seule de fait, sans déclencheur ; pas d'heure réelle, pas de poste, pas de personne qui récupère |
| `063` `ent_sortie_regle_niveau` | niveau (texte ≤ 32), matin/midi/soir autorisé, `choix_parent_interdit` | UPSERT écrasant |
| `064` `ent_sortie_regle_regime` | régime ∈ EXT/DP/INT | UPSERT écrasant |
| `065` `ent_sortie_param` (+ `ent_edt_creneau.site`) | `autoriser_changement_site` | — |
| `131` `ent_absence_declaration` (**socle 12**) | déclarations de l'accueil et des familles : période, motif, `declarant_nom/qualite`, statut active/annulée | utilisée par l'écran Leptos « Journée » de la vie scolaire (`modules/journee_vs`) ; pas de FK sur `declarant_user_id` ni `piece_document_id` ; rapprochements supprimés physiquement à l'annulation (`vie_scolaire.rs:714`) |
| `036` `ent_repas_passage` (**socle 40**) | passage au self par badge | seule lecture de badge existante ; sans FK vers l'élève ; `actor_user_id` jamais renseigné |
| `015`, `281` `ent_eleve.numero_badge` (**socle 2**) | numéro de badge historisé (système BADGE) | réutilisable pour la carte de sortie |

Toutes ces tables ont `etablissement_id NOT NULL` et une politique RLS posée en 581 / 587.

**Code.** Domaine `domain-eleve/src/sortie.rs` (914 lignes, 9 tests unitaires : demande, décision, évaluation du contrôle, règles niveau et régime) ; dépôt `ent-db/src/sortie_repo.rs` ; services `ent-db/src/service.rs:12295-12753` ; routes `ent-api/src/routes.rs:402-431` (`/eleves/{id}/sorties`, `/vs/sorties/demandes`, `/vs/sorties/controle`, `/vs/sorties/regles`, `/regles-regime`, `/param`). Droits du catalogue (`ent-auth/src/actions.rs`) : **`sorties.gerer`**, **`sorties.controler`** (direction, admin, CPE par défaut), `absences.declarer`, `remise.lire`, `remise.gerer`, `remise.interdire`, `eleves.badge.gerer`, `repas.scanner`. Le modèle de rôle **Accueil** a `absences.declarer`, `responsables.lire`, `remise.lire`, **mais pas `sorties.controler`** (test l.2020).

**Socle 4, prêt à servir** : `ent_eleve_lien_actif` (vue des liens en vigueur, suspension), `ent_eleve_lien.peut_autoriser_sorties`, `ent_eleve_contact_urgence.peut_recuperer` (sans dates ni pièce, à migrer vers `ent_eleve_remise` au lot A5 du socle 4), `ent_eleve_interdiction` (271 : objets dont `remise`, fondement, motif scellé, consigne lisible avec `remise.lire`), `ent_adresse_protegee` (267), consentement `photo_remise`. Test `tests_administratif_a1.rs:553` : l'interdiction de remise est visible de l'accueil et de la vie scolaire. **Mais aucun lien n'existe entre ces tables et le contrôle de sortie** : `controle_sortie_for_actor` ne lit ni les liens actifs, ni `peut_recuperer`, ni les interdictions.

**Rien** pour : registre des visiteurs, accès refusé, remise tracée, main courante, élève manquant, PPMS, alertes, comptage, exercices. Le module Leptos `modules/accueil/` est la page « Aujourd'hui » (tuiles), **pas** le poste d'accueil. Le menu « Sorties » de la navigation Leptos ouvre le module voyages du socle 42.

### 2.2 Défauts de l'existant Rust (DR-01…) et lot 0

| N° | Gravité | Défaut [F] | Preuve | Correctif | Lot |
|---|---|---|---|---|---|
| DR-01 | **Critique** | Les dépôts des sorties (et du self) utilisent directement le pool, **sans la transaction locataire** : la variable d'établissement n'est jamais posée, la RLS reste en mode permissif ; en mode strict, ces tables paraîtraient vides. L'isolation ne repose que sur les `WHERE etablissement_id` | `sortie_repo.rs`, `repas_repo.rs` (0 occurrence de `tenant_tx`) ; `100_rls_pilote.sql:15-23` | passer `PgSortieRepo` sous `tenant_tx` ; test RLS strict des 6 tables (et signaler `PgRepasRepo` au socle 40) | **AS0** |
| DR-02 | **Critique** | **Contrôle au portail qui ne revérifie rien** : ni les règles en vigueur, ni le lien du parent qui a donné l'autorisation (un lien suspendu ou clos la laisse valable), ni le statut actif de l'élève | `service.rs:12662` | le contrôle recalcule la décision (règles + autorisation + lien actif + élève actif) à chaque appel | **AS0** |
| DR-03 | **Critique** | **Heure du contrôle fournie par le client** (jour et heure libres, antidatage possible) et, par défaut, `Utc::now().time()` comparée à des horaires d'EDT en heure locale : décalage d'une ou deux heures | `routes.rs:7805` | heure du serveur dans le fuseau de l'établissement (socle 5, `etablissement_fuseau` 265) ; une heure passée n'est acceptée que comme « saisie a posteriori » avec motif, auteur habilité, marquée | **AS0** |
| DR-04 | Haute | **Règle des demi-pensionnaires ignorée** quand le régime a été écrit par les finances (`demi_pension` n'est pas reconnu par `normalize_sortie_regime`, qui ne connaît que EXT/DP/INT) ; lecture du régime courant (`ent_eleve.regime`, texte libre) et non de la période datée | `sortie.rs:595-602`, `finances.rs:2967`, `service.rs:12354` | lire la période datée `ent_eleve_periode` (kind `regime`) via le port du socle 2 ; table de correspondance des deux vocabulaires, testée | **AS0** |
| DR-05 | Haute | **Règles appliquées une seule fois**, à l'enregistrement par le parent : le personnel enregistre sans règle ; une règle modifiée ne change rien aux autorisations existantes | `sortie.rs:550, 658` | la règle s'applique **au calcul** (DR-02), pas à l'écriture | **AS0** (calcul), AS2 (modèle) |
| DR-06 | Haute | **Autorisation sans historique** : UPSERT écrasant, sans dates ni année ; seule trace : `ent_metier_event`, dont la charge utile JSONB porte **le motif de la demande en clair**, table hors des politiques RLS 58x | `sortie_repo.rs:19-48`, `service.rs:12568-12573` | migration **2280** : historique en ajout seul alimenté par déclencheur ; ne plus écrire le motif dans l'événement ; signaler `ent_metier_event` hors RLS au pilote (socle 1) | **AS0** |
| DR-07 | Moyenne | **Un élève mineur peut déposer seul une demande de sortie exceptionnelle** (aucun contrôle de majorité) | `service.rs:12545-12552` | demande par un titulaire `autoriser_sorties`, par l'élève **majeur**, ou par le personnel ; un mineur peut seulement **préparer** une demande que le titulaire confirme [R] | **AS0** |
| DR-08 | Moyenne | Le rôle **Accueil** n'a pas `sorties.controler` : l'accueil ne peut pas contrôler une sortie ni voir les sorties attendues | `actions.rs` l.1359-1362, test l.2020 | ajouter `sorties.controler` au modèle Accueil ; adapter le test | **AS0** |
| DR-09 | Moyenne | `ent_sortie_controle` : pas de déclencheur d'ajout seul ; ni poste, ni heure réelle, ni personne à qui l'élève est remis | `062:46` | migration **2281** : déclencheur d'immuabilité ; colonnes `controle_le` (heure serveur), `saisie_a_posteriori`, `poste_libelle` | **AS0** |
| DR-10 | Moyenne | **Préfixe `ent_sortie_*` partagé** entre les sorties individuelles (14) et les sorties et voyages (42, 640–642) | migrations 062–065 et 640–642 | les **nouvelles** tables du 14 prennent les préfixes `ent_regime_sortie_*`, `ent_accueil_*`, `ent_surete_*` ; les tables 062–065 sont reprises au lot AS2 puis retirées (2299, après décision) | AS2 |
| DR-11 | Faible | Écrans des sorties seulement dans `ui.rs` (F-001, figée) | `ui.rs:42129-42534, 51358` | écrans Leptos aux lots AS2–AS5 ; `ui.rs` non étendue | AS2–AS5 |
| DR-12 | Faible | Créneaux codés **matin / midi / soir** : trop grossiers (sortie entre deux cours, cours annulé, permanence en dernière heure) | `062`, `063`, `sortie.rs` | créneaux du §10.2 | AS2 |
| DR-13 | Faible (autre socle) | `DELETE` physique d'un contact « peut récupérer » (`dossier_personnes.rs:1954`) et des rapprochements de déclarations (`vie_scolaire.rs:714`) | idem | signalé aux socles 4 (lot A5) et 12 ; hors du lot AS0 | — |

**Lot AS0 (correctifs urgents de l'existant)** = DR-01 à DR-09 : code dans `domain-eleve/src/sortie.rs`, `ent-db/src/sortie_repo.rs`, `service.rs`, `ent-api/src/routes.rs`, `ent-auth/src/actions.rs` ; migrations **2280** (`2280_sortie_autorisation_historique` : table `ent_sortie_autorisation_hist` en ajout seul, déclencheur `AFTER INSERT OR UPDATE` sur `ent_sortie_autorisation`, reprise de la ligne actuelle comme première version, RLS, registre) et **2281** (`2281_sortie_controle_immuable` : colonnes `controle_le TIMESTAMPTZ NOT NULL DEFAULT now()`, `saisie_a_posteriori BOOLEAN NOT NULL DEFAULT false`, `poste_libelle TEXT NULL`, déclencheur refusant `UPDATE`/`DELETE`). Tests : RLS strict des 6 tables, contrôle avec lien suspendu → refusé, élève sorti des effectifs → refusé, demi-pensionnaire écrit `demi_pension` → règle DP appliquée, heure locale à 7 h 55 et 15 h 05 un jour de changement d'heure, mineur → demande refusée, accueil → contrôle permis. **Aucune nouvelle fonction** dans AS0 ; il rend sûr ce qui existe en attendant AS2.

### 2.3 Côté Next.js : module « Accueil » [F, lecture seule et ciblée]

Inventaire T0 (`TRANSITION_T0_INVENTAIRE.md` §3.0) : **Accueil (saisie d'absences) : 19 fichiers, 4 550 lignes, 8 routes, 0 page dédiée hors `(admin)/accueil/absences`, 3 fichiers de test, maturité « Partiel » [H]**, données en tables typées (`vs_absence_eleve`, `absence`), `pg_dump` suffisant. Le même dossier `accueil/` contient les **portes ouvertes** (M17, socle 3) — hors de ce socle.

**Actions existantes (NXA-…), toutes reprises :**

| N° | Action Next.js | Preuve | Reprise |
|---|---|---|---|
| NXA-01 | Déclarer une absence ou un retard **d'élève** à l'accueil (aujourd'hui, plage horaire, plusieurs jours ; canal téléphone / sur place / courriel ; motif libre ≤ 400 caractères ; retard 8 h 00–8 h 30 par défaut) | `api/accueil/absences/route.ts:16-35`, `lib/accueil-absences-db.ts:333-420` | ACS-110 (commande du socle 12) |
| NXA-02 | Déclarer une absence ou un retard **de professeur ou de personnel OGEC** à l'accueil, avec avis à la direction pour validation | `accueil-absences-db.ts:220-330` (`createRhAccueilAbsence`, `notifyAbsenceCreated`) | ACS-116 → **socle 15** |
| NXA-03 | **Garde-fou « en sortie »** : un élève participant d'une sortie scolaire ce jour-là n'est pas déclaré absent sans forçage explicite (code `ELEVE_EN_SORTIE`, 409) | `lib/accueil-absence-occupancy.ts`, `api/accueil/absences/presence/route.ts` | ACS-112 (port `voyages::en_sortie`) |
| NXA-04 | **Tableau du jour** des déclarations de l'accueil (élèves, professeurs, personnels OGEC), filtré par rôle (lignes OGEC visibles de l'administration, de la comptabilité, de la direction et de l'accueil) | `api/accueil/absences/route.ts:41-53`, `lib/accueil-absences-access.ts` | ACS-113 |
| NXA-05 | **Annuler** une déclaration de l'accueil (élève : note CPE ; personnel : refus et clôture avec historique, impossible si déjà validée par la direction) | `accueil-absences-db.ts:676-732` | ACS-114 |
| NXA-06 | **Consultation** des absences déclarées à l'accueil par la vie scolaire (module séparé, lecture seule) | `(admin)/vie-scolaire/absences-accueil`, `api/vie-scolaire/absences-accueil/route.ts` | ACS-115 (écran du socle 12) |
| NXA-07 | Recherche unifiée élève / professeur / personnel, avec cycle (école / collège / lycée) et classe | `lib/accueil-absences-search.ts` | ACS-100 |
| NXA-08 | Synchronisation de la déclaration vers un port d'absences (`getAbsenceSyncPort().pushAbsence`) et invalidation des signaux du tableau de bord | `accueil-absences-db.ts:405-420, 681-688` | événement `accueil.declaration_transmise` + outbox |
| NXA-09 | **Outil d'assistant** `create_accueil_absence` (« Paul est absent 2 jours », assistant pas à pas et confirmation) et `cancel_accueil_absence` | `lib/brain-ai/tools/registry.ts:248-275` | §18, capacité IA N2 |
| NXA-10 | Rôle « Accueil » à **périmètre réduit** : liste des sorties et voyages sans détail, liste des élèves sans fiche | `lib/accueil-access.ts:1-60` | §21.2 |

**Défauts relevés (VA-…), à ne pas reproduire :**

| N° | Gravité | Défaut | Preuve | Correctif ScolIA |
|---|---|---|---|---|
| VA-01 | Haute | `canDeclareAccueilAbsence` renvoie **toujours vrai** : tout titulaire du module déclare pour tout élève et tout personnel | `lib/accueil-absences-access.ts:10-12` | droit `absences.declarer` du catalogue, portée établissement, testé |
| VA-02 | Haute | **Motif libre de 400 caractères** sans contrôle : santé, situation familiale (« hospitalisé », « garde chez le père ») s'y retrouvent, visibles de tout le tableau | `route.ts:28` | motif **codé** (socle 12) + commentaire court passé au **lint de minimisation** (R-AS6) |
| VA-03 | Moyenne | **Identité de l'appelant non enregistrée** (seul le nom de l'agent d'accueil) : on ne sait pas qui a annoncé l'absence | `accueil-absences-db.ts:394-403` | appelant choisi parmi les personnes liées ou « autre, à rappeler » (§9.5) |
| VA-04 | Moyenne | Auteur stocké **en texte** (`created_by_nom`), heures en texte (`heure_debut` `text`) | `drizzle/0028_accueil_absences.sql` | auteur = utilisateur ; heures typées |
| VA-05 | Moyenne | Absences des **personnels** mêlées aux absences d'élèves dans le même écran, avec un rôle « accueil » qui voit les absences du personnel OGEC | `accueil-absences-access.ts:19-28` | socle 15, droits du 15 (l'accueil voit « absent aujourd'hui », jamais le motif) |
| VA-06 | Faible | « Forcer » le garde-fou en sortie sans motif enregistré | `route.ts:34` | motif obligatoire, journalisé |
| VA-07 | Faible | L'outil d'assistant déclare via la même fonction sans plafond ni contrôle de portée propre | `registry.ts:248` | capacité IA déclarée, mêmes droits que l'écran, N2 (§18) |

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)
- La **vitesse** de la saisie à l'accueil (recherche unique, trois modes de période, valeur par défaut du retard) ;
- le **garde-fou « en sortie »** ;
- le **tableau du jour** de l'accueil ;
- la déclaration d'un **professeur absent** depuis l'accueil (raccourci vers le 15) ;
- l'assistant « Paul est absent deux jours » ;
- le périmètre réduit du rôle Accueil.

### 2.5 Verdict
- **Next.js accueil** : **partiel, en usage** pour la seule saisie d'absences et de retards. On **reprend ses dix actions** (NXA-01 à NXA-10) dans le lot AS1, branchées sur le socle 12 (élèves) et le socle 15 (personnels) ; **tout le reste du socle 14 n'existe pas dans Next.js** et se conçoit de zéro.
- **Rust 062–065** : **base utile mais fragile** ; on la **rend sûre** (AS0), puis on la **remplace** par le modèle du §25 (AS2), avec reprise des autorisations en cours.
- **Socle 4** : les briques de la remise existent ; il manque le **branchement** (AS4).

---

## 3. Acteurs, besoins réels, irritants

Acteurs notés comme `SOCLE2_DOMAINES.md` §3 et `04_ADMINISTRATIF.md` §9 : **ACC** accueil (standard, loge), **AED** assistant d'éducation ou surveillant (dont surveillant de nuit et maître d'internat), **CPE**, **VS** vie scolaire au sens large, **DIR** direction (chef d'établissement, adjoint, directeur délégué), **ADF** secrétariat, **PRF** professeur, **INF** infirmier·e, **AGT** agent (entretien, cuisine, maintenance, souvent agent de la collectivité ou de l'OGEC), **REF** référent sûreté ou correspondant PPMS, **PAR** titulaire de l'autorité parentale, **TIE** autre personne liée (personne autorisée, grand-parent, assistant familial), **ELV** élève (mineur), **MAJ** élève majeur, **VIS** visiteur, **SYS** système.

| Acteur | Ce qu'il fait vraiment | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Accueil (loge, standard)** | Répond au téléphone (parents qui annoncent une absence, un retard, un départ), ouvre le portail, reçoit les visiteurs, voit partir les élèves en sortie anticipée, remet les élèves aux parents, réceptionne les livraisons, donne les badges visiteurs | Le classeur papier des autorisations jamais à jour ; chercher où est un élève (« il est en quelle salle, là ? ») ; appeler la vie scolaire pour savoir si un élève peut sortir ; recopier la même absence sur papier puis dans un logiciel ; le parent qui attend pendant qu'on cherche | **Remettre un enfant à quelqu'un qui n'en a pas le droit** ; laisser entrer un inconnu ; refuser à tort un parent légitime devant l'enfant ; être seul face à un parent agressif | Taper trois lettres, voir la photo de l'élève, **où il est maintenant**, **qui peut le chercher aujourd'hui** (interdictions en tête), un bouton « appeler en vie scolaire » et un bouton « remis à… » ; un registre des visiteurs qui se remplit en dix secondes | toute la journée |
| **AED, surveillant** | Contrôle les sorties au portail aux heures de sortie, vérifie les carnets ou cartes, gère la permanence, cherche un élève manquant, accompagne une évacuation | Le carnet oublié, la carte prêtée, la photo de la carte vieille de trois ans ; 400 élèves qui sortent en dix minutes ; les régimes de sortie qui changent quand un cours est annulé | Laisser sortir un 6ᵉ qui n'a pas le droit, perdre un élève pendant une évacuation | Un téléphone qui dit **« sortie autorisée / refusée »** au passage de la carte ou au nom tapé, **même sans réseau**, avec la raison ; la liste de « ses » élèves en cas d'alerte | quotidienne, aux sonneries |
| **CPE** | Fixe les régimes de sortie avec les familles en septembre, traite les demandes de sortie exceptionnelle, est appelée pour chaque cas douteux, gère les élèves partis sans autorisation | Les coupons de régime de sortie papier à saisir à la rentrée ; les appels de l'accueil pour chaque cas ; les familles qui changent d'avis par téléphone | Un élève parti sans autorisation qui a un accident ; ne pas savoir qui était là en cas de problème | Régimes saisis **par les familles** en ligne à la rentrée, validés en un écran ; décisions déjà prises par les règles, ne remonter que les vrais cas ; un fil « élève manquant » qui organise la recherche | quotidienne |
| **Direction** | Arrête le règlement intérieur (régimes de sortie), élabore et actualise le **PPMS**, organise les exercices, déclenche une alerte, préside la cellule de crise, signe les comptes rendus, décide des interdictions d'accès | Le PPMS dans un classeur que personne n'ouvre, l'annuaire de crise périmé, le compte rendu d'exercice rédigé de mémoire, les listes d'élèves imprimées au mauvais moment | En cas d'alerte réelle : **ne pas savoir qui est où**, ne pas joindre les adultes, faire une faute que la presse relèvera ; ne pas pouvoir prouver que les exercices ont eu lieu | Un PPMS vivant (zones, rôles, annuaire à jour, mallettes vérifiées) ; un bouton d'alerte qui lance les bons messages et **affiche en direct qui est compté, zone par zone** ; un compte rendu pré-rempli avec le chronométrage | alerte : rare mais critique ; exercices : au moins 2 PPMS et au moins 2 évacuations incendie par an [V, §5] |
| **Référent sûreté, correspondant PPMS** (souvent l'adjoint, le gestionnaire ou un CPE) | Tient à jour plans, zones, mallettes, annuaire ; prépare les exercices ; fait le lien avec la mairie, les secours, le référent académique | Mises à jour à la main à chaque arrivée de personnel ; vérification des mallettes oubliée | Une mallette sans piles le jour J ; un plan qui ne correspond plus aux bâtiments | Rappels automatiques, liste des écarts (« 3 personnels arrivés depuis le dernier exercice n'ont pas de zone ») | mensuelle |
| **Professeur** | Fait l'appel, garde ses élèves en cas d'alerte, compte, signale les absents et les élèves en plus, laisse sortir un élève appelé par la vie scolaire | Ne pas savoir s'il peut laisser sortir un élève qui « a rendez-vous chez le dentiste » ; recompter à la main dans le noir pendant un confinement | Perdre un élève ; un élève d'une autre classe réfugié dans sa salle qu'il ne connaît pas | Sur son téléphone, en alerte : **la liste des élèves qu'il doit avoir** (d'après l'EDT de l'instant), cocher présents, ajouter un élève réfugié en tapant son nom, signaler un blessé sans détail médical | alerte : rare |
| **Infirmier·e** | Renvoie un élève malade au domicile, appelle la famille, prépare la sortie, fournit les trousses PPMS | Courir à l'accueil pour dire qui vient chercher l'élève | Qu'un élève malade parte seul ou avec une personne interdite | La sortie santé préparée à l'infirmerie apparaît à l'accueil (qui, à quelle heure), **sans motif médical** (socle 34 SAN-142) | quotidienne |
| **Secrétariat** | Saisit les personnes autorisées (socle 4), imprime les listes, accueille parfois | Les mêmes informations demandées par l'accueil, la vie scolaire, l'infirmerie | — | Une seule saisie (socle 4), lue partout | hebdomadaire |
| **Agents (entretien, cuisine, maintenance)** | Présents dans l'établissement sans poste informatique ; ont un rôle dans le PPMS (coupures, fermeture des accès) | Ne pas être prévenus d'un exercice ou d'une alerte | Être oubliés dans le comptage | Être joints par SMS, être comptés | rare |
| **Titulaires de l'autorité parentale** | Annoncent une absence ou un retard, demandent une sortie anticipée (rendez-vous médical), désignent qui vient chercher l'enfant, choisissent le régime de sortie en septembre | Remplir le même coupon tous les ans, appeler le standard qui ne répond pas, donner un mot écrit pour chaque rendez-vous | Que l'enfant sorte sans qu'ils le sachent ; que l'autre parent (séparé) vienne le chercher contre une décision ; en cas d'alerte : ne rien savoir | Demander une sortie en deux gestes dans l'appli, recevoir **« Léa est sortie à 15 h 12, remise à Mme X »**, être prévenus d'une sortie anticipée due à un cours annulé, recevoir l'alerte et sa levée | hebdomadaire |
| **Familles séparées, enfant placé, ordonnance de protection** | Un parent veut empêcher l'autre ; l'ASE décide ; un juge a interdit un contact | Expliquer leur situation à chaque nouvel AED | Une remise à la personne interdite ; une adresse protégée révélée | L'interdiction posée **une fois** au socle 4 et respectée partout, sans que le motif soit lu | rare, critique |
| **Personnes autorisées (grand-mère, nourrice, voisine)** | Viennent chercher l'enfant | Être refusées faute de papier | — | Être reconnues (photo si l'établissement et elles l'acceptent), recevoir les sorties anticipées (personne de confiance du socle 8, R-N60) | quotidienne |
| **Élève mineur** | Sort selon son régime, oublie sa carte, demande à sortir pour un rendez-vous, se trouve au CDI pendant une alerte | Être bloqué au portail alors que son cours est annulé ; ne pas savoir s'il peut sortir | Être puni pour une sortie qu'il croyait autorisée | Voir dans l'appli **« aujourd'hui tu peux sortir à 15 h »** et pourquoi ; une carte dans son téléphone | quotidienne |
| **Élève majeur** | Peut demander lui-même ses sorties et en être responsable selon le règlement intérieur [V-à vérifier §5] ; ses parents ne sont plus destinataires par défaut | Qu'on lui demande un mot de ses parents | — | Gérer ses sorties lui-même, informer ses parents seulement s'il le choisit (socle 4, R-A24) | quotidienne |
| **Élève en situation de handicap, AESH** | L'AESH accompagne l'élève, y compris dans une évacuation ; un élève en fauteuil a un **espace d'attente sécurisé** ou un accompagnement nominatif [H] | Évacuation non prévue pour lui ; AESH absente le jour de l'exercice | Qu'il reste seul à l'étage | Le PPMS sait qui a besoin d'aide pour évacuer (sans diagnostic : « aide à l'évacuation : oui », socle 35) et qui l'aide aujourd'hui | rare |
| **Visiteur** (parent convoqué, intervenant, livreur, artisan, inspecteur, journaliste, ancien élève, candidat à l'inscription) | Se présente, attend, est accompagné ou non, repart | Attendre qu'on retrouve son rendez-vous ; donner trois fois son nom | Être traité comme suspect | Être attendu (« vous avez rendez-vous avec M. Martin à 14 h, il est prévenu »), un badge prêt, un départ en un geste | quotidienne |
| **Prestataire, entreprise de travaux** | Intervient plusieurs jours, avec ses salariés | Refaire l'accueil chaque matin | — | Un accès temporaire déclaré une fois, avec la liste des intervenants (sans pièce d'identité conservée) | ponctuelle |
| **Mairie, secours, référent académique, police** | Reçoivent ou envoient des alertes, attendent un compte rendu | Pas d'interlocuteur joignable, pas de plan à jour | — | Un annuaire de crise et un plan accessibles, un compte rendu standard | rare |
| **Délégué à la protection des données (DPO)** | Vérifie registre des traitements, durées, minimisation, droits d'accès | Registre des visiteurs conservé indéfiniment ; photocopies de cartes d'identité ; badge des élèves qui trace tout | Un fichier de surveillance des allées et venues des élèves | Des durées courtes appliquées automatiquement, aucune biométrie, des journaux d'accès consultables | annuelle |
| **Attaquant** (personne interdite, élève qui veut sortir, curieux, intrus) | Cherche à faire sortir un enfant, à se faire passer pour un parent, à lire l'adresse d'un parent, à falsifier une autorisation, à abuser d'une alerte | — | — | — | — |

---

## 4. La journée et l'année vues du terrain [H, à confirmer par Florian]

**Journée type (lycée pilote, privé sous contrat)**

| Heure | Ce qui se passe | Ce que ScolIA fait |
|---|---|---|
| 7 h 30 – 8 h 15 | Appels des familles (« Léa est malade », « Hugo sera en retard ») ; ouverture du portail | Déclaration d'absence ou de retard à l'accueil en dix secondes, versée à l'appel (socle 12) ; identité de l'appelant et rappel si doute |
| 8 h – 8 h 20 | Retardataires | Billet d'entrée (socle 12) ; badge ou nom tapé ; contrôle Vigipirate à l'entrée [V §5] |
| Matinée | Visiteurs (parents convoqués, intervenants, livreurs) ; sorties pour rendez-vous médical | Registre des visiteurs, badge, accompagnement ; sortie anticipée demandée par la famille, contrôlée à la sortie |
| 10 h – 12 h | Cours annulé à 11 h (absence de professeur, socle 15) | Régimes de sortie recalculés : les élèves qui ont le droit peuvent sortir à 11 h, les familles des autres sont prévenues si la règle le prévoit (socle 8 `edt.sortie_anticipee`) |
| 12 h – 14 h | Externes sortent et rentrent, demi-pensionnaires restent (régime scolaire, socle 2 / 40) | Contrôle au portail selon le régime de sortie de la pause méridienne |
| Après-midi | Infirmerie : un élève renvoyé au domicile | Sortie santé préparée par l'infirmière, remise à l'accueil à une personne autorisée |
| 16 h 30 – 17 h 30 | Sorties massives ; parents qui viennent chercher les plus jeunes | Contrôle de sortie ; remise vérifiée pour les élèves qui ne sortent pas seuls |
| Soir | Internes (socle 41) ; retour d'un car de voyage à 22 h (socle 42) | Remise au retour du car |

**Année type**

| Période | Ce qui se passe |
|---|---|
| Juin – juillet | Révision du règlement intérieur (régimes de sortie) ; révision du PPMS pour l'année suivante ; vérification des mallettes |
| Fin août – septembre | Régimes de sortie choisis par les familles (formulaire de rentrée, socle 4 / 18), validés par la CPE ; cartes et photos (socle 2) ; nouveaux personnels affectés à une zone PPMS ; **répertoire de crise mis à jour et transmis** (public : à la DSDEN, qui le transmet avant le 15 septembre) [V §5] ; **premier exercice d'évacuation incendie dans le mois qui suit la rentrée** [V §5] ; information de la communauté éducative (pré-rentrée, premier conseil d'administration) et des parents sur le PPMS [V §5] |
| Septembre – octobre | **Premier exercice PPMS** (annoncé, sans effet de surprise) [V §5] ; compte rendu ; plan d'actions |
| Novembre – février | **Deuxième exercice PPMS avant les vacances d'hiver**, scénario différent du premier [V §5] ; deuxième exercice d'évacuation incendie |
| Mars | **Retours d'expérience** des exercices transmis à la DSDEN et à la collectivité **au plus tard le 1er avril** (public) et présentés au conseil d'administration [V §5] |
| Avril – juin | Voyages et retours de cars (42) ; révision du PPMS si risque ou locaux changent ; **PPMS (nouveau ou révisé) validé et communiqué à la DSDEN, à la collectivité et au maire au plus tard le 1er juillet** avant son entrée en vigueur (public) [V §5] ; bilan annuel sûreté pour l'OGEC (privé) [H] |

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche faite par un sous-agent le 2026-11-11, sur Légifrance (versions en vigueur au 11/11/2026), les PDF officiels du BO et d'éduscol, sgdsn.gouv.fr (PDF) et cnil.fr. Les pages HTML d'education.gouv.fr, de sgdsn.gouv.fr et d'interieur.gouv.fr refusent les accès automatisés : on a lu les PDF. **[V]** lu sur la source officielle ; **[NV]** non vérifié (résumé de recherche seulement).

### 5.1 PPMS [V]
- **Texte en vigueur** : circulaire du **8 juin 2023**, NOR MENE2307453C, **BO n° 26 du 29 juin 2023** (Éducation, Intérieur, Transition écologique), qui abroge la circulaire n° 2015-205 et la partie « attentat-intrusion » de l'instruction du 12 avril 2017. Fondements : C. éduc. L. 312-13-1, L. 411-4, D. 312-40 ; C. sécurité intérieure L. 721-1, R. 741-1. PDF : https://www.education.gouv.fr/sites/default/files/Bulletin_officiel_MENJS_2023_06_29_BO26-1688057946.pdf
- **PPMS unique** : « Le PPMS risques majeurs et le PPMS attentat-intrusion sont fusionnés dans un même document », en trois parties (description de l'établissement ; organisation interne et conduites à tenir ; outils, facultative). **Échéance** : PPMS unifié adopté « au plus tard pour l'année scolaire 2027-2028 » — c'est **l'année de doublon** de ScolIA (D-034) : le module PPMS est utile **dès le printemps 2027** pour préparer le document.
- **Chef d'établissement** (collèges, lycées) : élabore le PPMS (identification des risques par la DSDEN, diagnostics, retours d'expérience), le valide et le communique **à la DSDEN, à la collectivité et au maire au plus tard le 1er juillet** avant son entrée en vigueur ; la DSDEN le transmet aux forces de sécurité et au SDIS avant le 15 juillet. Valable sans limite de durée « sous réserve que les exercices soient réalisés et qu'il soit actualisé » ; révisé si un risque apparaît ou si les locaux changent ; information de la communauté éducative (pré-rentrée, premier CA) et des parents à la rentrée.
- **Répertoire de crise** : mis à jour **à chaque rentrée** et transmis à la DSDEN, qui le transmet à la préfecture, aux forces de sécurité et au SDIS **au plus tard le 15 septembre** ; données « confidentielles et [qui] ne peuvent être utilisées à d'autres fins qu'à l'anticipation ou à la gestion d'une situation de crise ».
- **Activation** : « Toute personne témoin d'un danger imminent peut déclencher l'alarme » ; le chef d'établissement est responsable de l'activation et de la mise en œuvre jusqu'à la levée par les autorités, « y compris si l'événement s'étend au-delà du temps scolaire ».
- **Exercices** : « au moins deux exercices PPMS distincts des exercices incendie chaque année (l'un en septembre-octobre, l'autre avant les vacances d'hiver) », « sans effet de surprise et sans mise en scène exagérément réaliste », « l'utilisation d'arme factice est proscrite » ; scénarios variés pour éprouver toutes les postures ; forces de sécurité et communauté éducative prévenues à l'avance ; attention particulière aux élèves en situation de handicap et aux élèves bénéficiant d'un PAP.
- **Retours d'expérience** : associent élèves, personnels et parents (notamment leurs représentants) ; transmis à la DSDEN et à la collectivité **au plus tard le 1er avril** ; présentés au conseil d'administration.
- **Notice et modèle (éduscol, février 2024)** : Fascicule 1 « Notice explicative » (https://eduscol.education.fr/document/57357/download) et Fascicule 2 « Modèle » (https://eduscol.education.fr/document/57360/download). **Quatre postures** : évacuation, mise à l'abri simple, mise à l'abri améliorée (calfeutrage), « s'échapper ou se cacher en se barricadant » (décision qui « peut relever d'un individu ou d'un petit groupe ») ; **au moins deux moyens d'alerte distincts de l'alarme incendie** ; **cellule de crise** sous la direction du chef d'établissement ; **main courante** = « registre, qui recense les événements… dans le cadre de l'activation du PPMS et les décisions prises » ; fiche du **responsable de zone** (« transmettre rapidement les effectifs présents », « établir la liste des absents / manquants ») ; fiche « **Recensement des personnes présentes** » : NOM / PRÉNOM / CLASSE / PRÉSENT / ABSENT / MANQUANT / BLESSÉ ; « Un élève **manquant** à l'appel est un élève qui était présent en début de journée mais qui manque à l'appel au moment de l'événement » ; annexes « Annuaire de crise » et « Liste des personnes nécessitant une attention particulière » (handicap, PAI…), **réservées au chef d'établissement et à la cellule de crise** : « Ces informations sont confidentielles ».
- **Mallette PPMS** : présente dans le guide de 2017, **non retrouvée** dans le modèle unifié de 2024 [NV] : bonne pratique, pas obligation explicite.
- **Privé sous contrat** : la circulaire vise le public ; les établissements privés sous contrat « peuvent s'inspirer des directives » ; ils « sont invités » à transmettre leur répertoire de crise. **Pas d'obligation formelle** : ScolIA applique au privé les règles du public **comme bonnes pratiques** (même choix que le 42, Q-V1), sans transmission obligatoire.

### 5.2 Exercices d'évacuation incendie et registre de sécurité [V]
- **Arrêté du 25 juin 1980** (règlement de sécurité des ERP), **art. R 33** (version depuis le 14 mai 2004) : exercices pratiques au cours de l'année scolaire, **exercices de nuit** s'il y a des locaux de sommeil (internat) ; « le premier exercice doit se dérouler durant le mois qui suit la rentrée » ; « les conditions de leur déroulement et le temps d'évacuation doivent être consignés sur le registre de sécurité ». https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000025134178. Le texte ne fixe plus de nombre ; le ministère retient « **au moins deux par an** » (notice PPMS 2024). Public et privé (tout ERP).
- **Registre de sécurité** : CCH **R. 143-44** (version en vigueur depuis le 1er juillet 2026, décret n° 2025-1100 du 19 novembre 2025) : travaux, état nominatif du service de sécurité, consignes (dont évacuation selon les types de handicap), contrôles, et **« les dates des exercices de sécurité incendie »**. https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000043819037. L'arrêté du 19 février 2026 (en vigueur le 1er juin 2027) modifie le règlement ERP ; aucune modification de R 33 vue [NV, lecture partielle].

### 5.3 Vigipirate et contrôle des accès [V sauf mention]
- Plan rénové « VIGIPIRATE – Faire face ensemble » (SGDSN, PDF) : trois niveaux (« Vigilance », « Vigilance renforcée », « Alerte attentat ») ; le niveau sommital vaut 12 jours puis revient au niveau intermédiaire sauf renouvellement. Posture « été-automne 2026 » au niveau « Vigilance renforcée » depuis le 22 juin 2026 **[NV]** : la posture est **saisie** par la direction, jamais devinée par ScolIA.
- « Guide pratique pour la sécurité des élèves, des personnels et des enceintes scolaires » (éduscol, 2024) : « L'accueil à l'entrée… est assuré par un adulte » ; « **Un contrôle visuel des sacs** doit être effectué ; en cas de refus, le… chef d'établissement peut refuser l'accès » ; « **L'identité des personnes étrangères à l'établissement est systématiquement vérifiée** ; en cas de doute… peut refuser l'accès » ; éviter les attroupements ; signaler tout comportement ou objet suspect.
- Guide des chefs d'établissement (2016) : contrôle visuel des sacs des adultes « avec le consentement de leurs propriétaires ; seul un OPJ peut contraindre à la fouille » ; le refus permet seulement de refuser l'accès.
- **Pouvoirs du chef d'établissement (EPLE)** : C. éduc. **R. 421-10, 3°** (en vigueur depuis le 4 juillet 2025) : toutes dispositions « pour assurer la sécurité des personnes et des biens » ; **R. 421-12** : en cas d'urgence, « interdire l'accès de ces enceintes ou locaux à toute personne relevant ou non de l'établissement » et « suspendre des enseignements », puis informer le CA et rendre compte. C'est le fondement de l'**accès refusé** (§8.5) dans le public ; dans le privé, le pouvoir de police interne du chef d'établissement et le règlement intérieur [H].

### 5.4 Registre des visiteurs, badges, vidéo, biométrie
- **Aucun texte n'impose un registre des visiteurs** en EPLE [V : rien trouvé] ; l'obligation est la **vérification d'identité** (5.3). Le registre est un **choix** de l'établissement, soumis au RGPD (finalité, minimisation, information, durée).
- **Repère CNIL** : mention d'information type pour l'accès par badge aux locaux professionnels : données « conservées **trois mois** à compter du jour de votre visite » (https://www.cnil.fr/fr/RGPD-exemple-information-acces-aux-locaux-professionnels-par-badge) ; ancienne norme simplifiée NS-42 (identité, date et heure, personne visitée ; trois mois) [NV pour le détail ; repère sans valeur contraignante depuis le RGPD]. Le guide sécurité de la CNIL (2024) recommande de journaliser l'arrivée et le départ des visiteurs.
- **Vidéo** (CNIL, « Les dispositifs vidéo dans les établissements scolaires », 12 septembre 2025) : accès et circulations filmables ; lieux de vie exclus « sauf cas exceptionnels » ; conservation « quelques jours ». ScolIA **ne traite pas** la vidéo (hors périmètre) ; un fait de main courante peut seulement mentionner « images demandées à la direction ».
- **Biométrie** : la CNIL a jugé la reconnaissance faciale à l'entrée de deux lycées « contraire aux grands principes de proportionnalité et de minimisation », le **badge** étant l'alternative moins intrusive (29 octobre 2019, https://www.cnil.fr/fr/experimentation-de-la-reconnaissance-faciale-dans-deux-lycees-la-cnil-precise-sa-position). **Aucune biométrie** dans le 14 (R-AS7).

### 5.5 Sorties des élèves et régimes de sortie
- **C. éduc. R. 421-5** (version du 1er septembre 2019) : le règlement intérieur, adopté par le CA, définit droits et devoirs, prévoit la « prise en charge progressive par les élèves eux-mêmes de la responsabilité de certaines de leurs activités » et les modalités de l'obligation d'assiduité [V]. https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000039016548
- **Circulaire n° 96-248 du 25 octobre 1996** (BO n° 39 du 31 octobre 1996), modifiée par la circulaire n° 2004-054 : texte original **non lu** [NV] ; toujours citée par le guide ministériel des sorties et voyages de décembre 2025 ; contenu lu dans le **Guide juridique du chef d'établissement, fiche 34 « La surveillance »** (2009) [V] :
  - la surveillance couvre « la totalité du temps scolaire… déterminée par l'emploi du temps » : demi-journée pour l'externe, journée pour le demi-pensionnaire, permanente pour l'interne ; « directe et continue » pour les jeunes collégiens, moins étroite à partir de 16 ans ;
  - **collège** : les élèves « ne sont pas admis à quitter l'établissement durant les périodes scolaires définies par l'emploi du temps… même si celles-ci comportent des temps libres » ; le règlement peut permettre aux parents d'autoriser la sortie en cas d'absence inopinée d'un professeur **en fin de période** (demi-journée ou journée), autorisation « toujours écrite » ; à défaut d'information préalable des familles, la surveillance est assurée par l'établissement « de préférence sous forme d'heures de permanence ou d'études surveillées » ;
  - **lycée** : mêmes règles, et le règlement peut « prévoir la sortie des élèves durant les temps libres entre les cours », des déplacements seuls sur courte distance et des sorties en petits groupes ;
  - **sorties hors règlement** : accordées par le chef d'établissement « sur demande écrite des parents » ; en cas de « nécessité impérieuse », sortie possible « après avoir, si possible, prévenu ses représentants légaux » ;
  - **élève majeur** : « peut justifier lui-même de ses absences » ; le chef d'établissement apprécie les motifs ;
  - **responsabilité** : C. éduc. L. 911-4 (substitution de l'État pour une faute de surveillance d'un agent public).

### 5.6 Remise à un tiers, autorité parentale [V sauf mention]
- **C. civ. 372-2** : « A l'égard des tiers de bonne foi, chacun des parents est réputé agir avec l'accord de l'autre, quand il fait seul un acte usuel de l'autorité parentale relativement à la personne de l'enfant. » Le guide DGESCO « L'exercice de l'autorité parentale en milieu scolaire » (2011) précise que la présomption « tombe en cas de désaccord manifeste de l'autre parent » (TA Lille, 11 mars 2009) et qu'« aucune disposition juridique ne permet aux parents de conférer, par eux-mêmes, à un tiers… le pouvoir d'accomplir tous les actes usuels » (délégation = jugement, art. 377-1) ; CE, 13 avril 2018, n° 392949 : l'administration apprécie si l'acte est réellement usuel. **Conséquence** : désigner une personne qui vient chercher l'enfant est un acte usuel ponctuel d'un titulaire ; le **désaccord connu** de l'autre titulaire le bloque (R-AS3) ; une « procuration générale » n'existe pas (cohérent avec ADM-83 du socle 4, limitée et datée).
- **C. civ. 373-2** (version du 20 mars 2024) : la séparation est sans incidence sur la dévolution de l'autorité parentale ; **373-2-6** : interdiction de sortie du territoire ordonnée par le juge ; **515-11** (ordonnance de protection, version du 15 juin 2024) : interdictions de contact, dissimulation du domicile [NV pour le détail]. Tous trois alimentent les **interdictions du socle 4**, que le 14 applique sans les interpréter.
- **Aucun texte national** propre au second degré n'organise la remise d'un élève à un tiers (décharge, pièce d'identité) [V : rien trouvé] : la procédure du §12 est une **recommandation** [R], cohérente avec la vérification d'identité de Vigipirate et le règlement intérieur.

### 5.7 Élève manquant, mineur en danger
- Définition de l'élève manquant : celle du modèle PPMS (5.1). Sortie sans autorisation : famille avisée par le chef d'établissement (fiche 34) [V]. **Aucune procédure nationale** dédiée à l'élève qui quitte l'établissement sans autorisation [V : rien trouvé] : la procédure du §13 est une recommandation [R].
- Mineur en danger (guide pratique 2024) [V] : signalement au procureur (C. proc. pén. art. 40) « de manière systématique et immédiate pour chaque situation préoccupante », information de la CRIP (CASF L. 226-3) et du cabinet du DASEN ; danger imminent : 15, 17, DASEN, référent police ou gendarmerie. Disparition inquiétante d'un mineur : alerter sans délai police ou gendarmerie [NV]. **ScolIA rappelle ces numéros et gestes ; il ne signale pas lui-même** (R-AS5).

### 5.8 Intrusion [V]
- **C. pén. 431-22** (version du 26 janvier 2023) : pénétrer ou se maintenir dans l'enceinte d'un établissement scolaire sans habilitation ni autorisation « dans le but de troubler la tranquillité ou le bon ordre » : 1 an et 7 500 € ; amende forfaitaire délictuelle possible.
- **C. pén. R. 645-12** : contravention de 5ᵉ classe pour pénétrer ou se maintenir sans habilitation ni autorisation dans un établissement scolaire **public ou privé**.
- Pas de liste réglementaire des personnes habilitées : autorisées = habilitées par un texte, ou autorisées par le chef d'établissement.

### 5.9 Main courante et faits établissement
- La main courante n'est définie que dans le cadre du PPMS (5.1) ; **aucune obligation générale** d'un registre des incidents [V : rien trouvé], hors application **« Faits établissement »** (guide pratique 2024) : aviser les services « via l'application Faits établissement et, en cas d'urgence, par téléphone », signalement « priorisé, court et clair », « au plus tôt », trois niveaux de gravité (1 interne, 2 grave, 3 très grave), structure Quoi / Qui (anonymisé) / Quand / Où / Pourquoi + premières mesures ; extension au privé sous contrat annoncée en 2024, **état actuel non vérifié** [NV]. ScolIA prépare ce récapitulatif (structure reprise, ACS-229) ; la déclaration reste un geste de la direction dans l'outil national.

### 5.10 Ce qui reste à vérifier [NV]
- Texte original de la circulaire 96-248 et de sa modification 2004-054 (BO inaccessible).
- Posture Vigipirate en vigueur au moment de la mise en service.
- Extension de « Faits établissement » au privé sous contrat.
- Forme admise d'un registre de sécurité dématérialisé (commission de sécurité).
- Contenu exact des art. 373-2-9 et 515-11 C. civ. pour les cas limites de remise.
- Règles propres de l'Enseignement catholique (Sgec) sur le PPMS et les sorties [NV, à demander par Florian].

---

## 6. Branchements sur le cœur

### 6.1 Qui possède quoi

| Donnée | Propriétaire | Le 14 en fait |
|---|---|---|
| Élève, classe, niveau, âge, majorité, régime scolaire daté, photo, numéro de badge | **2** (et 5 pour classes et niveaux) | lit (port), jamais de copie |
| Titulaires, liens actifs, droit `autoriser_sorties`, personnes autorisées à la remise, interdictions, joignables, adresse protégée | **4** | lit `personnes::remise`, `personnes::titulaires`, `personnes::joignables` ; propose (ADM-87, 88, 89) |
| Lieux (sites, bâtiments, étages, salles), calendrier, fuseau | **5** | rattache les zones PPMS aux lieux ; heure locale |
| Agents, postes, planning de service, intervenants attendus, contacts professionnels, mobile d'urgence | **6** | lit `planning::de_service`, `personnels::attendus` (PRS-135, 136) |
| Pièces (PPMS signé, compte rendu, photo d'un mot d'excuse) | **7** | dépose, référence |
| Messages, alertes, accusés, personnes de confiance | **8** | demande l'envoi (`surete.*`, `accueil.*`), lit les accusés |
| Séances, annulations, localisation à l'instant | **10** | lit `edt::seances`, `edt::seance_en_cours`, `edt::localiser` (journalisé) |
| Appel, absences, déclarations, retards, billets | **12** | **commande** `assiduite::declarer`, lit l'appel de l'instant ; reçoit `surete.remise_effectuee`, `sortie.effectuee` |
| Incidents, punitions, retenues | **13** | lit « en retenue maintenant » ; propose un incident |
| **Régimes de sortie, règles, autorisations, sorties exceptionnelles, restrictions, passages, remises, visites, accès refusés, main courante, recherches d'élève, PPMS, alertes, comptages, exercices, comptes rendus, plans d'actions** | **14** | possède |
| Absences des personnels | **15** | commande (raccourci NXA-02) ; lit « absent aujourd'hui » |
| Formulaires de rentrée et consentements | **18** (moteur), **4** (fiche de rentrée) | fournit les options de sortie à présenter ; reçoit la réponse |
| Rendez-vous | **19** | lit les rendez-vous du jour (visiteurs attendus) |
| Infirmerie, renvoi au domicile, trousses PPMS, fiche PPMS santé | **34** | lit `sante::mouvements` (présence à l'infirmerie, sans motif) ; reçoit `sante.renvoi_prepare` |
| Aide à l'évacuation (besoin, aidant) | **35** | lit le seul indicateur « aide à l'évacuation » |
| Informations préoccupantes | **36** | propose (cercle restreint) |
| Internat, sorties d'internat | **41** | lit « interne présent ce soir » pour les présents de nuit |
| Sorties et voyages collectifs, participants, encadrants | **42** | lit `voyages::en_sortie`, `voyages::participants_du_jour` ; fournit `accueil::remise_preparer`, `surete::presents` |
| Indicateurs | **47** | expose |

### 6.2 Ports consommés

| Port | Socle | Usage |
|---|---|---|
| `eleves::identite_minimale(eleve)` (nom, prénom, classe, photo), `eleves::majeur(eleve, date)`, `eleves::regime(eleve, date)` | 2 | recherche, décision de sortie, contrôle |
| `personnes::remise(eleve, date) -> { interdits, autorises }` | 4 | remise, fiche de secours |
| `personnes::titulaires_autoriser_sorties(eleve, date)`, `personnes::desaccord_declare(eleve, objet)` | 4 | qui peut signer l'autorisation, qui peut demander une sortie |
| `personnes::joignables(eleve, nature)` | 4 | élève manquant, accord à distance |
| `personnes::demander_accord_remise` (ADM-87), `personnes::consigne_remise` (ADM-88), `personnes::proposer_responsable_inconnu` (ADM-89) | 4 | §12.2 |
| `lieux::arbre(site)`, `calendrier::jour(date)`, `etablissement::fuseau()` | 5 | zones, heure locale |
| `planning::de_service(instant)`, `personnels::attendus(date)`, `personnels::contacts_crise()` | 6 | présents, visiteurs attendus, cellule |
| `edt::seances(eleve, jour)`, `edt::seance_en_cours(eleve, instant)`, `edt::localiser(eleve, instant, motif)` | 10 | décision, présents, recherche |
| `assiduite::declarer(cmd)`, `assiduite::appel_instant(seance)`, `assiduite::statut_eleve(eleve, instant)` | 12 | NXA-01, présents |
| `discipline::en_retenue(eleve, instant)` | 13 | décision |
| `rh::absents(date)` | 15 | présents |
| `notifications::urgence_declencher`, `notifications::avis`, `notifications::accuses(alerte)` | 8 | alertes, avis de remise, compte rendu |
| `sante::mouvements(eleve, instant) -> { a_l_infirmerie: bool }` | 34 | présents, recherche |
| `inclusion::aide_evacuation(eleve) -> Option<Aidant>` | 35 | PPMS |
| `voyages::en_sortie(eleve, jour)`, `voyages::participants_partis(instant)` | 42 | garde-fou, présents |

### 6.3 Ports exposés par le 14

| Port | Pour | Nature |
|---|---|---|
| `sortie::decider(eleve, instant) -> Decision` | 12 (pré-remplir l'appel), 10 (fin de journée réelle, EDT-229), appli famille et élève, 41 | lecture, pure, explicable |
| `sortie::sorties_du_jour(eleve)` | 12, 13, 21 | lecture |
| `accueil::remise_preparer(eleves, lieu, heure)` ; `accueil::remettre(cmd)` | 42 (VOY-207), 34 (SAN-142), 41 | commande, interdictions en tête |
| `surete::presents(instant, perimetre)` | 42, 41, 34 | lecture, en alerte seulement pour la liste nominative |
| `surete::alerte_en_cours()` | 8, 16 (silence de crise R-M52), 12 (suspendre les relances), tous | lecture |
| `accueil::visiteurs_presents()` | 6, cellule de crise | lecture |
| `accueil::fait_main_courante(cmd)` | 4 (ADM-89), 34 (CS-18), 42 | commande |

### 6.4 Répercussions automatiques (le cœur bouge, le 14 suit)

| Événement reçu | Effet dans le 14 | Mode |
|---|---|---|
| `eleve.regime_changed` (2) | décisions recalculées (pas de stockage) ; si une option ne s'applique plus, autorisation marquée « à revoir » pour la CPE | auto |
| `eleve.majeur` (2, à la date) | l'autorisation de la famille reste valable jusqu'au choix de l'élève ; l'élève est invité à confirmer ses options ; avis aux titulaires selon R-A24 | auto + humain |
| `eleve.sorti_effectifs` / départ | autorisations closes à la date ; sorties exceptionnelles futures annulées ; aucune remise possible après | auto |
| `lien.clos`, `lien.suspendu`, `lien.qualite_changee` (4) | une autorisation signée par ce titulaire est **revue** (reste valable si l'autre titulaire la confirme ; sinon régime le plus restrictif) ; demandes futures de ce titulaire annulées | auto |
| `eleve.interdiction_changee` (4) | fiches de secours et écrans de remise recalculés ; alerte à l'accueil **sans motif** ; si une sortie exceptionnelle prévoit une remise à la personne désormais interdite → annulée, CPE prévenue | auto |
| `edt.seance_modifiee`, `edt.fermeture_exceptionnelle` (10) | sortie anticipée calculée (§10.6) | auto |
| `rh.absence_validee` (15) | idem | auto |
| `sante.renvoi_prepare` (34) | sortie santé en attente de remise à l'accueil | auto |
| `sorties.retour`, `sorties.retard_retour` (42) | écran de remise du retour préparé, mis à jour | auto |
| `personnel.arrive`, `personnel.parti` (6) | « personnels sans zone PPMS » au tableau du référent | auto |
| `lieu.cree`, `lieu.ferme` (5) | zones à revoir | auto |
| `annee.ouverte` (5) | nouvelle version des règles de sortie à approuver ; calendrier d'exercices proposé | humain |

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]

| Terme | Définition | Où |
|---|---|---|
| **Régime scolaire** | Externe, demi-pensionnaire, interne… : lié à la restauration et à l'hébergement. **Possédé par le socle 2** (`ent_eleve_periode` kind `regime`) | 2 |
| **Régime de sortie** | Ce que l'élève a le droit de faire **en dehors des heures de cours prévues** (sortir entre deux cours, à la pause méridienne, en fin de journée après le dernier cours effectif, en cas d'absence de professeur…). Fixé par le règlement intérieur, choisi par la famille dans le cadre qu'il ouvre, validé par l'établissement. **Possédé par le socle 14** | 14 |
| **Règle de sortie** | Ligne du règlement intérieur traduite en règle déterministe : public visé (niveau, classe, régime scolaire, âge, majeur), créneau (entre cours, midi, fin de journée, permanence en dernière heure, cours annulé), ce qui est permis (sortie libre, sortie seulement si l'option a été choisie, jamais) | 14 |
| **Autorisation de sortie** | Le choix daté de la famille (ou de l'élève majeur) parmi les options que les règles ouvrent, et sa validation par la vie scolaire | 14 |
| **Sortie exceptionnelle** | Une dérogation ponctuelle (rendez-vous médical, convocation, motif familial) demandée par un titulaire ou l'élève majeur, décidée par la vie scolaire | 14 |
| **Sortie anticipée** | Fin de la journée plus tôt que prévu pour un groupe, du fait de l'EDT (cours annulé, fermeture) : calculée, pas demandée | 14 (calcul), 10 (cause), 8 (avis) |
| **Remise** | Le fait de confier un élève, physiquement, à une personne. Vérifiée contre `personnes::remise` (socle 4) ; **enregistrée** au registre des sorties du 14 | 4 (qui peut), 14 (qui a reçu) |
| **Mouvement** | Toute entrée ou sortie de l'établissement constatée : passage au portail (carte ou nom), sortie remise, retour, départ en sortie collective (42), arrivée tardive | 14 |
| **Visite** | La présence d'une personne extérieure : attendue ou non, entrée, badge, hôte, sortie | 14 |
| **Accès refusé** | Personne à ne pas laisser entrer (décision du chef d'établissement, interdiction d'accès), distincte de l'interdiction de **remise** du socle 4 | 14 (décision), 4 (personnes) |
| **Zone de mise en sûreté** | Lieu où un groupe se met à l'abri ou se confine, avec ses responsables, sa mallette, sa capacité [V : PPMS §5] | 14 (sur les lieux du socle 5) |
| **Point de rassemblement** | Lieu extérieur d'évacuation | 14 |
| **Alerte** | Un événement réel de sûreté : mise à l'abri, confinement, évacuation, attentat-intrusion (s'échapper, se cacher), fin d'alerte. Le message est porté par le socle 8 (`surete.alerte`) ; la gestion (zones, comptage, cellule, compte rendu) par le 14 | 14 + 8 |
| **Exercice** | Une alerte simulée, planifiée, chronométrée, avec compte rendu ; jamais confondue avec une alerte réelle | 14 + 8 |
| **Comptage** | Pour chaque responsable de groupe, la liste des personnes qu'il **devrait** avoir (calculée à l'instant de l'alerte) et le pointage de celles qu'il **a** | 14 |
| **Présents** | L'estimation, à un instant, des élèves, personnels et visiteurs dans l'établissement, avec la source de chaque information et son degré de certitude | 14 |
| **Main courante** | Le registre chronologique des faits de l'accueil et de la sûreté (intrusion, appel suspect, personne inconnue, refus de remise, objet abandonné…) ; qualifiés, jamais réécrits | 14 (faits d'accueil), 13 (incidents disciplinaires) |
| **Élève manquant** | Un élève attendu, introuvable, ou parti sans autorisation ; ouvre une procédure de recherche horodatée | 14 |

### 7.2 Distinctions qui manquent aujourd'hui [R]
1. **Régime scolaire ≠ régime de sortie.** L'existant Rust (062–065) et le code Next.js les mêlent ou les déduisent l'un de l'autre ; ce sont deux faits distincts (un demi-pensionnaire peut avoir le droit de sortir en fin de journée ; un externe de 6ᵉ peut ne pas avoir le droit de sortir entre deux cours).
2. **Interdiction de remise (socle 4) ≠ accès refusé (socle 14).** La première protège un enfant d'une personne ; la seconde protège l'établissement d'une personne. Elles se croisent (une personne interdite de remise peut aussi être refusée à l'entrée) mais n'ont ni le même auteur ni le même fondement.
3. **Absence déclarée par l'accueil ≠ absence constatée.** L'accueil **déclare** (socle 12 l'enregistre comme déclaration « prévenue ») ; l'appel **constate**. L'accueil n'écrit jamais l'absence lui-même.
4. **Sortie individuelle (14) ≠ sortie collective (42).** Le 42 organise ; le 14 voit partir et revenir, et remet au retour.
5. **Personne de confiance (socle 8) ≠ personne autorisée à la remise (socle 4).** La première reçoit des avis ; la seconde peut emmener l'enfant. Souvent la même personne, mais deux désignations.
6. **Main courante (14) ≠ incident disciplinaire (13) ≠ information préoccupante (36).** Un fait d'accueil peut **proposer** un incident ou une information préoccupante ; il ne les crée pas.

---

## 8. Accueil et visiteurs

### 8.1 Principe
Le registre des visiteurs sert à **savoir qui est dans l'établissement en ce moment** (sûreté, PPMS) et à **retrouver après coup qui était là** en cas d'incident. Il n'est pas un fichier de personnes : il garde le **minimum**, peu de temps [V : CNIL, §5].

### 8.2 Ce qu'on enregistre [R]
- **Visiteur** : nom et prénom tels que déclarés ; qualité (parent d'élève, intervenant, prestataire, livreur, autorité, candidat, autre) ; organisme facultatif ; **hôte** (personnel visité ou service) ; motif court codé (rendez-vous, livraison, travaux, réunion, inscription, autre) ; heure d'entrée, heure de sortie ; numéro de badge remis ; accompagné ou non.
- **Pièce d'identité** : **vérifiée, jamais photocopiée ni photographiée** par défaut [R, CNIL §5] ; on note seulement « pièce présentée : oui / non / refusée » et son type (carte, passeport, permis, carte professionnelle).
- **Rattachement** : si le visiteur est une **personne connue** du socle 4 (parent, personne autorisée) ou un agent du socle 6 (intervenant habituel), le registre la référence (`personne_id`) au lieu de recopier son nom ; la saisie libre reste possible pour l'inconnu.
- **Jamais** : motif détaillé d'une convocation, d'un rendez-vous médical ou social, nom de l'élève concerné par une convocation disciplinaire (le registre ne dit pas « convoqué pour l'exclusion de Hugo »), numéro de la pièce d'identité, photo du visiteur, plaque d'immatriculation [R].

### 8.3 Visite attendue
- Tout personnel peut **annoncer** un visiteur (rendez-vous du socle 19, réunion, intervenant du socle 6, entreprise de travaux) ; l'accueil voit la liste « attendus aujourd'hui » (PRS-136 du socle 6, rendez-vous du 19) ; à l'arrivée, un geste : « arrivé », l'hôte est prévenu (in-app, push).
- **Accès temporaire** pour un prestataire (plusieurs jours, liste des intervenants nommés, référent interne) : saisi une fois, pointé chaque jour.
- **Portes ouvertes, cérémonies, réunions de parents** : les inscrits du socle 3 (portes ouvertes, M16/M17 de Next.js) ou du 19 sont **attendus en nombre** ; l'accueil pointe sans tout ressaisir ; les prospects restent des **contacts**, pas des élèves.

### 8.4 Badge visiteur
- Badge physique numéroté (stock de l'établissement) ou étiquette imprimée (nom, hôte, date, heure, sans logo de l'élève visité) [R] ; le numéro de badge est noté ; **un badge non rendu à la fermeture** apparaît en alerte douce le soir (« 2 visiteurs non sortis : Mme X arrivée 14 h, hôte M. Y »).
- Un visiteur non sorti à l'heure de fermeture est **compté comme présent** dans la liste des présents jusqu'à clôture manuelle.

### 8.5 Accès refusé
- Décision du chef d'établissement (ou de son délégué) d'**interdire l'accès** à une personne (comportement violent, intrusion antérieure, mesure administrative), datée, avec consigne courte et motif **scellé** (chiffré, jamais affiché à l'accueil) [R, sur le modèle de l'interdiction du socle 4].
- À la saisie d'un visiteur dont le nom approche une personne refusée, l'accueil voit **« Ne pas laisser entrer — prévenir la direction »**, jamais le motif. Le rapprochement est **par nom** (personne inconnue du système) ou **par `personne_id`** (personne connue) ; aucun traitement biométrique, aucune photo par défaut.
- L'accès refusé **ne prive pas** un titulaire de l'autorité parentale de ses droits sur la scolarité (il reçoit ses informations, rencontre l'équipe ailleurs ou sur rendez-vous accompagné) : la consigne le dit [R, Q-AS8].

---

## 9. Appels et déclarations à l'accueil (parité Next.js)

Le module accueil de Next.js est **partiel** (T0 : 19 fichiers, 8 routes, 3 tests) mais **utilisé** : l'accueil y déclare réellement des absences et des retards d'élèves et de personnels [F, `app/lib/accueil-absences-db.ts`, migration `drizzle/0028`]. On reprend ses **actions** (D-035), pas son code.

1. **Déclarer une absence ou un retard d'élève reçu par téléphone, sur place ou par courriel** (NXA-01) : recherche de l'élève, période (aujourd'hui, plage horaire, plusieurs jours), nature (absence / retard), canal, **identité de l'appelant** (personne du dossier choisie dans la liste des titulaires et personnes liées, ou « autre, à rappeler »), motif **codé** (socle 12) et commentaire court **sans donnée de santé** (lint de minimisation, R-AS6). Effet : **commande du socle 12** `assiduite::declarer(source = accueil)` ; le 14 garde la trace de l'appel (main d'œuvre de l'accueil), pas l'absence.
2. **Garde-fou « en sortie »** (NXA-03, repris) : si l'élève est participant d'une sortie collective ce jour-là (port `voyages::en_sortie` du 42), l'accueil le voit et ne déclare qu'en forçant avec motif.
3. **Tableau du jour** (NXA-04) : déclarations de l'accueil du jour, par nature, avec l'auteur ; annulation par le déclarant ou la vie scolaire tant que l'appel n'a pas tranché (NXA-05), **jamais de suppression** : annulation datée.
4. **Absences et retards des personnels** (NXA-02) : **sortent du socle 14** et passent au **socle 15** (absences des personnels) par sa commande ; l'écran de l'accueil garde un raccourci « un professeur appelle pour prévenir » qui ouvre la déclaration du 15 avec les droits du 15 [R].
5. **Appelant douteux** (cas réel : un élève qui imite un parent) : si l'appelant n'est pas reconnu (numéro absent du dossier, voix douteuse), la déclaration est marquée « à confirmer » et le 12 la traite comme **non prévenue** tant qu'un titulaire n'a pas confirmé (rappel sur un numéro **du dossier**, R-A13 du socle 4).
6. **Appel « mon enfant sera récupéré par… »** : ouvre la consigne téléphonique de remise du socle 4 (ADM-88) ; l'autorisation ponctuelle n'existe qu'après rappel.
7. **Appel « je viens chercher mon enfant à 14 h »** : crée une **sortie exceptionnelle à valider** (§10.5) et prépare la remise (§12).

---

## 10. Régimes et autorisations de sortie

### 10.1 Ce que fixe le règlement intérieur [V, §5]
Le règlement intérieur, adopté par le conseil d'administration dans le public (C. éduc. R. 421-5) et par l'organisme de gestion dans le privé [H], fixe les conditions de sortie des élèves pendant le temps scolaire. Repères de la circulaire n° 96-248 (toujours citée par le ministère, §5) : **au collège**, pas de sortie pendant les périodes de l'emploi du temps, même libres ; sortie en cas d'absence inopinée d'un professeur **seulement en fin de demi-journée ou de journée**, sur autorisation **écrite** des parents prévue par le règlement ; **au lycée**, le règlement peut en plus permettre la sortie entre les cours. ScolIA fournit donc **deux modèles de règles** (collège, lycée) qui respectent ces repères, et signale toute règle saisie qui s'en écarte (R-AS16). ScolIA **ne décide pas** des règles : il les **traduit** en tables versionnées, à partir d'un modèle proposé, puis les **applique** de façon déterministe et explicable.

### 10.2 Règles de sortie (moteur) [R]
- Une **version de règles** par année scolaire (et par site si besoin), datée, approuvée par la direction ; une nouvelle version s'applique à une date.
- Chaque **règle** : public (niveaux, classes, régime scolaire, âge minimal, majeur), **créneau** (codé : `entre_cours`, `pause_meridienne`, `fin_journee`, `cours_annule`, `permanence_derniere_heure`, `permanence_premiere_heure`, `journee_sans_cours`), **effet** (`libre`, `sur_option_famille`, `interdit`), heures limites facultatives, ordre de priorité.
- **Options proposées aux familles** : chaque règle `sur_option_famille` devient une case du formulaire de rentrée (« J'autorise mon enfant à quitter l'établissement après son dernier cours effectif, y compris en cas d'absence d'un professeur »). Le texte de chaque option est **figé par version** (empreinte), pour que la famille sache ce qu'elle a signé.
- **Explication** : chaque décision du moteur (« sortie autorisée à 15 h ») dit **quelle règle** et **quelle option** l'ont produite.

### 10.3 Autorisation de sortie de l'élève [R]
- Choix de la famille, **une par élève et par année** (puis versions datées si changement en cours d'année), recueilli par le **formulaire de rentrée** (socle 4 / 18) ou au guichet ; signé par **un titulaire** ayant le droit `autoriser_sorties` (socle 4) ; l'accord de l'autre titulaire est **présumé** pour cet acte usuel sauf désaccord déclaré (C. civ. 372-2) [V, voir 04 §8.8] ; un **désaccord connu** (l'autre titulaire a déclaré s'opposer) bloque l'option la plus large et remonte à la CPE.
- **Élève majeur** : choisit lui-même, dans les limites du règlement ; ses parents sont informés **seulement** si l'élève l'a accepté (socle 4, R-A24) [V-à vérifier, §5].
- **Validation** par la vie scolaire (une liste à valider, en masse pour les cas simples) ; tant qu'elle n'est pas validée, le **régime le plus restrictif** s'applique.
- **Restriction** décidée par l'établissement (mesure éducative, élève qui a abusé de son autorisation) : datée, motivée, notifiée aux titulaires ; c'est une décision du 14 qui peut être **proposée** par le 13 (discipline), jamais une sanction déguisée [R, Q-AS4].

### 10.4 Calcul du jour : « peut-il sortir maintenant ? » [R]
Fonction pure, testée, **sans effet de bord** : `sortie::decider(eleve, instant) -> Decision { autorisee, raison, regle, option, prochaine_sortie }`. Entrées :
1. l'EDT réel de l'élève à l'instant (socle 10 : séances, annulations, permanences) ;
2. son régime scolaire (2) et son âge ou sa majorité (2) ;
3. les règles en vigueur et son autorisation validée (14) ;
4. ses sorties exceptionnelles du jour (14) ;
5. une **restriction** en cours (14) ;
6. un **événement** qui l'empêche : participant d'une sortie collective (42), retenue (13), interne et règle de l'internat (41), élève à l'infirmerie (34, sans motif), **alerte en cours** (tout est interdit pendant une alerte).
Sortie : `autorisee` / `refusee` / `a_verifier` avec la raison en une phrase (« dernier cours à 15 h, option fin de journée cochée ») et, pour l'élève et la famille, **« prochaine sortie autorisée à … »**.

### 10.5 Sortie exceptionnelle [R]
- **Demandée** par un titulaire (droit `autoriser_sorties`) ou l'élève majeur, dans l'appli (deux gestes : date, heure, motif codé — rendez-vous médical, administratif, familial, autre — commentaire facultatif sans détail de santé, qui vient chercher : l'élève seul s'il en a le droit, ou une personne autorisée), ou par l'accueil sur appel (§9.7) ; un **mot papier** est saisi par la vie scolaire (photo du mot facultative, socle 7, conservée peu).
- **Décidée** par la vie scolaire (accepter, refuser motivé, accepter avec remise obligatoire) ; **automatique** si l'établissement l'a choisi pour les cas simples (élève majeur, motif codé, aucune évaluation ce créneau) [S-1403].
- **Effets** : l'élève voit l'autorisation ; le professeur du créneau la voit sur sa liste d'appel (socle 12 : « sortie autorisée à 14 h ») ; l'appel pré-remplit « sorti, autorisé » après l'heure ; l'accueil la voit dans « sorties attendues aujourd'hui ».
- **Conflit** : évaluation prévue (21), examen (38), retenue (13) → signalé au décideur, jamais refusé automatiquement.

### 10.6 Sortie anticipée (cours annulé) [R]
- Quand l'EDT change (socle 10 `edt.seance_modifiee` / annulation, socle 15), le 14 recalcule pour les élèves concernés la **fin de journée réelle** et leur droit de partir plus tôt ; il publie `surete.sortie_anticipee_calculee` que le socle 8 porte (`edt.sortie_anticipee`, U2, titulaires **et personnes de confiance**, R-N60) **seulement** pour les élèves qui ont le droit de partir ; pour les autres : « Léa reste en permanence jusqu'à 16 h 30 ».
- **Délai minimal** paramétrable entre l'annonce et la sortie effective (pour un collège : la famille doit être prévenue avant) [S-1402] ; en deçà, la sortie anticipée n'est pas proposée et l'élève reste en permanence.

---

## 11. Contrôle aux portes

### 11.1 Moyens [R]
- **Nom tapé** (trois lettres, photo de l'élève) : toujours possible, c'est le mode de base.
- **Carte de l'élève** (code-barres ou QR imprimé, numéro de badge existant `ent_eleve.numero_badge` [F]) lue par la caméra d'une tablette ou d'un téléphone ; **QR dans l'appli de l'élève**, **dynamique** (renouvelé toutes les 30 s, signé, lié à l'appareil) pour éviter la capture d'écran prêtée [R].
- **Lecteur sans contact** (badge RFID d'un système de contrôle d'accès existant) : **connecteur** facultatif, plus tard (lot AS12), via le socle 9 ; ScolIA ne pilote pas de portique [R].
- **Jamais de biométrie** (empreinte, reconnaissance faciale) pour le contrôle des élèves [V : CNIL, §5].

### 11.2 Ce que voit le contrôleur
Photo, nom, classe, et **un seul mot en grand** : **AUTORISÉ** (vert) / **REFUSÉ** (rouge) / **À VÉRIFIER** (orange), avec la raison en une ligne. Jamais : le régime scolaire de l'élève, ses absences, sa santé, sa situation familiale. Une **interdiction de remise** ne s'affiche pas au contrôle de sortie d'un élève qui sort seul (elle n'a pas d'objet) ; elle s'affiche à la **remise** (§12).

### 11.3 Enregistrement des passages [R, minimisation]
- Par défaut, **on n'enregistre que les exceptions** : refus, sortie exceptionnelle effectuée, remise, sortie anticipée effectuée, retour d'un élève sorti en cours de journée. Les sorties **ordinaires** autorisées par le régime ne sont **pas** journalisées nominativement [R, Q-AS2, CNIL §5 : proportionnalité d'un suivi des allées et venues des mineurs].
- Option de l'établissement (S-1401) : journaliser toutes les sorties (internat, établissement qui le justifie), conservation courte (§21.3).
- Le contrôle marche **hors ligne** : la tablette garde, pour la journée, la **décision précalculée** de chaque élève par créneau (pas ses données), chiffrée, effacée à minuit ; les passages hors ligne sont envoyés au retour du réseau (R-AS11).

### 11.4 Refus et élève qui passe quand même
Un refus est noté (élève, heure, raison). Un élève qui force la sortie : le contrôleur appuie sur « parti sans autorisation » → **procédure élève manquant** (§13), avec avis immédiat à la vie scolaire et, selon la règle, aux titulaires.

---

## 12. Remise de l'élève à une personne

### 12.1 Quand
Sortie exceptionnelle avec remise obligatoire ; élève dont le régime impose la remise (les plus jeunes, un élève en situation de handicap selon son projet) ; renvoi au domicile par l'infirmerie (34, SAN-142) ; fin d'une exclusion de cours avec remise (13) ; retour d'un car de sortie (42, VOY-207) ; fin d'alerte avec **remise aux familles** au point de regroupement (§14.7) ; internat le vendredi (41).

### 12.2 Écran de remise [R]
1. L'accueil tape le nom de l'élève **ou** la personne se présente et l'accueil tape son nom.
2. ScolIA affiche, **interdictions en tête** (socle 4, `personnes::remise(eleve, aujourd'hui)`) : la consigne de chaque interdiction (« ne pas remettre, prévenir la direction »), **jamais le motif** ; puis les personnes autorisées **aujourd'hui** (titulaires, permanentes, jours, ponctuelles) avec la mention « pièce d'identité exigée » et la photo si l'établissement l'a activée et que la personne y a consenti (04 §8.3 `photo_remise`).
3. L'accueil choisit la personne, coche « pièce vérifiée » (type seulement, jamais le numéro), et appuie sur **« remis à … »**. Horodatage, auteur, lieu.
4. **Personne non listée** : trois chemins, jamais « remettre quand même » :
   - **demander l'accord à distance** d'un titulaire (ADM-87 du socle 4 : notification « X est à l'accueil pour Léo ; l'autorisez-vous ? », réponse en 10 min, puis appel) ;
   - **consigne téléphonique** à confirmer par rappel sur un numéro **du dossier** (ADM-88) ;
   - **personne inconnue se disant responsable** (ADM-89) : aucune remise, fait de main courante, proposition au secrétariat.
5. **Personne interdite** : le bouton « remis » est **désactivé** ; un bouton « prévenir la direction » ouvre un fait de main courante et notifie la direction (U3 interne) ; un titulaire non interdit est appelé. Une interdiction `demande_titulaire` (non bloquante, 04 §8.8) s'affiche comme **« signalement à vérifier : appeler la direction avant remise »** et exige l'accord d'un membre de la direction pour remettre (deux gestes, journalisé) [R, R-AS3].
6. **Élève majeur** : pas de remise ; il sort selon ses droits. **Élève de 6ᵉ seul** : remise obligatoire selon la règle de l'établissement.

### 12.3 Après la remise
- Le 14 publie `surete.remise_effectuee` (sans nom de la personne) ; le 12 pré-remplit l'appel des séances suivantes (« sorti, remis ») ; le socle 8 envoie aux titulaires ayant le droit l'avis **« Léa a quitté l'établissement à 15 h 12, remise à Mme X »** (nature `surete.remise`, U1) — **jamais** à un titulaire visé par une interdiction de communication.
- **Retour dans la journée** (rendez-vous médical puis retour) : un geste « revenu à 16 h 05 » ; l'appel reprend.

### 12.4 Hors ligne
La **fiche de secours du jour** (ADM-91 du socle 4) contient les interdictions et les autorisés de la journée, chiffrée sur la tablette de l'accueil, effacée à minuit ; les remises faites hors ligne sont saisies au retour du réseau avec leur heure réelle.

---

## 13. Élève manquant, départ sans autorisation, fugue

### 13.1 Déclencheurs
- Élève **attendu** (appel du socle 12 : présent à 10 h, absent à 11 h sans sortie enregistrée) ;
- élève parti sans autorisation (contrôle au portail §11.4, infirmerie CS-18 du socle 34, cours) ;
- élève non revenu d'une sortie exceptionnelle à l'heure annoncée ;
- élève introuvable au comptage d'une alerte ou d'un exercice ;
- élève non remis au retour d'un car à l'heure + 30 min (42, VOY-208).

### 13.2 Procédure guidée [R, à confronter aux consignes académiques, §5]
1. **Ouverture** par un personnel (AED, CPE, professeur, accueil) : un geste, l'heure est notée.
2. **Vérifications rapides** proposées dans l'ordre, cochées une à une, horodatées : dernier lieu et dernière présence connus (appel, EDT `edt::localiser`, journalisé), infirmerie (port `sante::mouvements` : « à l'infirmerie : oui/non », sans motif), sortie collective (42), sortie exceptionnelle ou remise enregistrée, CDI, permanence, toilettes et lieux habituels (liste de l'établissement).
3. **Appel des titulaires** (numéros du dossier, socle 4 joignables ; jamais un titulaire visé par une interdiction de communication) : « votre enfant est-il avec vous ? ».
4. **Escalade** à la direction à T + N minutes (paramètre S-1405, défaut 15 min pour un collégien, 30 min pour un lycéen [R]) ; la direction décide du **signalement aux forces de l'ordre** — **ScolIA ne l'envoie jamais lui-même** [R].
5. **Clôture** : retrouvé (où, quand, par qui), remis, ou signalé ; un compte rendu court, et **proposition** (jamais création automatique) d'un incident (13) ou d'une information préoccupante (36) si les faits le justifient.

### 13.3 Ce que la procédure ne fait pas
Pas de géolocalisation du téléphone de l'élève ; pas de diffusion à d'autres familles ; pas de photo de l'élève hors des personnels qui cherchent ; pas d'IA qui « devine » où il est.

---

## 14. Sûreté : PPMS, alertes, exercices

### 14.1 Le PPMS dans ScolIA [V pour le contenu obligatoire, §5 ; R pour la forme]
ScolIA **ne rédige pas** le PPMS à la place de la direction et **ne remplace pas** les canaux de l'État ; il en tient la **partie vivante**, celle qui périme :
- **Zones** de mise à l'abri, de confinement et d'évacuation, rattachées aux **lieux** du socle 5 (bâtiments, étages, salles), avec capacité, consigne par type d'alerte, **responsable(s)** et suppléants, **mallette** rattachée ;
- **Points de rassemblement** extérieurs et **point de regroupement des familles** (lieu où l'on remet les élèves après une crise) ;
- **Cellule de crise** : rôles (chef, suppléant, liaison avec les secours, liaison familles, gestion des listes, communication), personnes désignées et suppléants, numéros professionnels ; **annuaire de crise** (secours, mairie, préfecture, autorité académique, transporteurs, gaz, électricité) — organisations du socle 4, contacts professionnels seulement ;
- **Mallettes** : contenu attendu (liste de l'établissement), dernière vérification, date de péremption des éléments, responsable ; la **trousse PPMS santé** est du socle 34 (SAN-312) : le 14 n'en connaît que l'existence et l'emplacement ;
- **Personnes à besoins particuliers pour l'évacuation** : « aide à l'évacuation : oui », aidant désigné et suppléant (souvent l'AESH, socle 35), **jamais de diagnostic** [R] ; élèves à consigne santé : le 14 sait seulement « fiche PPMS santé de la zone disponible » (34) ;
- **Document PPMS** lui-même (le PDF signé) : dépôt du socle 7, version, date de transmission à l'autorité académique (public) ou au réseau (privé) [V/H, §5] ;
- **Fiches réflexes** par type d'alerte et par rôle, éditées par l'établissement à partir des modèles officiels [V, §5].

### 14.2 La liste des présents (cœur du « faire mieux ») [R]
Calcul **à la demande** (`surete::presents(instant, perimetre)`), jamais stocké en continu, à partir de :
- **élèves** : EDT de l'instant (10) corrigé par l'appel déjà fait (12), les absences déclarées, les sorties exceptionnelles et remises (14), les participants partis en sortie collective (42, comptés « hors établissement, sortie X, responsable Y »), l'infirmerie (34 : « à l'infirmerie », sans motif), l'internat la nuit (41) ;
- **personnels** : EDT, planning de service (6, PRS-135), absences (15) ;
- **visiteurs** : registre (§8) non sortis ;
- **prestataires** : accès temporaires pointés.
Chaque ligne porte sa **source** et son **degré de certitude** (`constate` par un appel ou un pointage de moins de N minutes, `prevu` par l'EDT seulement, `inconnu`). La liste est **découpée par zone** : chaque responsable reçoit **les personnes qu'il devrait avoir**, d'après la salle de la séance en cours rattachée à sa zone.

### 14.3 Alerte réelle : déroulé [R]
1. **Déclenchement** (direction ou personne habilitée, `surete.alerte.declencher` ★) : type (mise à l'abri, confinement, évacuation, attentat-intrusion), périmètre (tout l'établissement, site, bâtiment, NTF-140), deux gestes + passkey ; **sur mobile en priorité**. Le 14 ouvre l'**alerte** et demande au socle 8 de porter le message (`surete.alerte`, U3, modèles validés à froid, NTF-85/87) ; en attentat-intrusion, **aucun son** côté ScolIA sur les téléphones des personnels (vibration seulement) et **silence de crise** de la messagerie (R-M52 du socle 16) [R].
2. **Comptage** : chaque responsable de zone ou de groupe voit sur son téléphone **sa liste** (§14.2), coche les présents, ajoute un **élève en plus** (réfugié d'une autre classe : nom tapé, il disparaît de la liste de son professeur d'origine comme « compté ailleurs »), signale **manquant**, **blessé** (sans détail médical : « blessé, prise en charge nécessaire »), **accompagné hors zone**. Hors ligne : la liste est précalculée à la réception de l'alerte et le pointage remonte dès que possible (R-AS11). **Bouton « RAS, tout le monde est là »** pour les cas simples.
3. **Tableau de la cellule de crise** : zones comptées / non comptées, totaux attendus et constatés, **manquants nominatifs** (avec dernière localisation), élèves comptés deux fois, visiteurs et prestataires, personnels non joints, accusés du message (socle 8) ; journal horodaté des décisions (main courante de crise).
4. **Points de situation** aux familles et personnels (NTF-139 du socle 8), rédigés par la direction ; l'IA **peut préparer** un brouillon de point de situation à partir des faits non nominatifs (N3, jamais envoyé sans la direction) [R].
5. **Fin d'alerte** (`surete.alerte.lever` ★) : message de fin (8), puis éventuellement **remise aux familles au point de regroupement** (§14.7).
6. **Clôture** : compte rendu pré-rempli (§14.6).

### 14.4 Ce que ScolIA ne fait pas en alerte [R]
Il **ne prévient pas** les secours à la place de l'établissement (le 17 / 112 se compose au téléphone) ; il ne diffuse **aucun nom** de victime ou de blessé ; il n'envoie rien aux personnes sous interdiction de communication (04 §8.12, NTF-87) ; il ne **géolocalise** personne ; il ne fait pas de **diffusion sonore** (l'alarme et le signal PPMS sont des dispositifs de l'établissement) ; il n'est **pas** un moyen d'alerte suffisant à lui seul : le PPMS garde ses moyens propres (signal sonore distinct, téléphones) [V/H, §5].

### 14.5 Exercices [V : obligations §5 ; R : forme]
- **Calendrier annuel** proposé à la rentrée selon les obligations (§5) : **au moins deux exercices PPMS distincts des exercices incendie** (le premier en septembre-octobre, le second avant les vacances d'hiver), **scénarios variés** pour éprouver toutes les postures ; **exercices d'évacuation incendie** (le premier dans le mois qui suit la rentrée ; au moins deux par an selon le ministère ; exercices de nuit si internat) ; rappels à la direction ; **compteur de conformité** de l'année (« incendie : 1 sur 2 ; PPMS : 1 sur 2 ; retours d'expérience transmis : non, échéance 1er avril »).
- **Lancement** d'un exercice : même déroulé qu'une alerte, mode **exercice** (préfixe « EXERCICE » partout, NTF-90 ; audience personnels par défaut ; **information des familles** avant ou après, NTF-143) ; **jamais confondu** avec une alerte réelle (couleur, mot, table d'état distincte). Un exercice PPMS est **annoncé** : « sans effet de surprise et sans mise en scène exagérément réaliste », forces de sécurité et communauté éducative **prévenues à l'avance** [V §5] ; ScolIA rappelle ces préalables avant le lancement et n'offre pas de mode « inopiné » pour le PPMS (R-AS15). Attention particulière aux élèves en situation de handicap et aux élèves bénéficiant d'un PAP [V §5].
- **Chronométrage** automatique : déclenchement → premier comptage → dernière zone comptée → fin ; temps par zone.
- **Observateurs** désignés (personnels ou extérieurs) qui saisissent leurs constats depuis leur téléphone (texte court, photo d'un lieu sans personne identifiable).

### 14.6 Compte rendu et retour d'expérience
- **Pré-rempli** : type, date, heure, durées, effectifs attendus et constatés, zones, manquants retrouvés, accusés et non-joints (NTF-92 du socle 8), constats des observateurs, décisions horodatées.
- **Complété et signé** par la direction (socle 7) ; le retour d'expérience **associe élèves, personnels et parents** (notamment leurs représentants) [V §5] : ScolIA propose un court questionnaire (socle 18) dont la synthèse est jointe ; **dysfonctionnements** listés et transformés en **plan d'actions** (action, responsable, échéance, état) suivi jusqu'à l'exercice suivant ; **transmission** à la DSDEN et à la collectivité au plus tard le 1er avril (public) [V §5] — **le geste de transmission est celui de la direction** (export PDF, ou plus tard connecteur du socle 9).
- **Registre de sécurité** (ERP, CCH R. 143-44, qui y inscrit depuis le 1er juillet 2026 « les dates des exercices de sécurité incendie » ; arrêté du 25 juin 1980, art. R 33 : « conditions de déroulement et temps d'évacuation » consignés) : ScolIA **produit l'extrait** à reporter ou à joindre, il ne **remplace pas** le registre réglementaire tant que sa forme dématérialisée n'est pas validée par l'établissement et sa commission de sécurité [V §5 ; NV pour la forme, Q-AS6].

### 14.7 Après une crise : remise aux familles
Au point de regroupement, l'écran de remise (§12) passe en **mode crise** : liste des élèves de la zone de regroupement, interdictions en tête, remise **uniquement** aux titulaires et personnes autorisées, pièce d'identité exigée, chaque remise horodatée ; la direction suit en direct « remis / restant ». Mode hors ligne avec la fiche de secours du jour.

### 14.8 Posture Vigipirate et contrôle d'accès
- Le niveau de posture (fixé par l'État) est **saisi** par la direction (pas récupéré automatiquement) ; il ajuste les consignes affichées à l'accueil (contrôle visuel des sacs, vérification d'identité des personnes extérieures, attroupements devant l'établissement) [V §5].
- ScolIA ne publie **jamais** les mesures de sûreté sur un espace visible des familles ou du public (plans, zones, codes, horaires de ronde) ; seule l'information générale prévue par le PPMS (consignes aux familles) est publiée [R].

---

## 15. Main courante de l'accueil et de la sûreté

- **Fait** : date et heure, lieu, catégorie codée (intrusion ou tentative, personne inconnue se disant responsable, refus de remise, personne refusée qui se présente, appel ou message suspect, objet abandonné, attroupement, dégradation constatée, malaise d'un visiteur, élève parti sans autorisation, autre), description courte **factuelle** (lint de minimisation : pas de santé, pas d'opinion sur une famille), personnes concernées **référencées** (élève, personne, agent) quand c'est utile, auteur.
- **Ajout seul** : un fait n'est jamais modifié ; on ajoute des **suites** (appel direction, appel police, retrouvé, classé) et des **corrections** liées.
- **Qualification** par la direction : simple trace / incident à ouvrir au 13 (proposition) / information préoccupante à évaluer au 36 (proposition au cercle restreint) / **fait établissement** à déclarer dans l'application nationale [V/NV §5] — ScolIA prépare le récapitulatif, la déclaration reste un geste de la direction dans l'outil officiel.
- **Visibilité** : l'auteur, l'accueil (faits de l'accueil des 7 derniers jours), la vie scolaire, la direction ; **jamais** les familles ni les professeurs sauf mention d'un de leurs élèves qui leur est transmise par la CPE.

---

## 16. Mobile et hors ligne

| Appareil | Qui | Ce qu'il fait | Hors ligne |
|---|---|---|---|
| **Tablette d'accueil** (poste partagé, session personnelle avec passkey et verrouillage court) | ACC | recherche d'élève, remise, visiteurs, déclarations, main courante | fiche de secours du jour (interdictions, autorisés, décisions de sortie) chiffrée, effacée à minuit |
| **Téléphone de l'AED** | AED | contrôle de sortie (caméra ou nom), élève manquant, comptage en alerte | décisions de sortie du jour par créneau, liste de comptage reçue à l'alerte |
| **Téléphone du professeur** | PRF | comptage en alerte, élève en plus, manquant | liste de comptage reçue à l'alerte |
| **Téléphone de la direction** | DIR, REF | déclenchement, tableau de crise, fin d'alerte, exercices | plan B du socle 8 (NTF-142) ; dernier état du comptage |
| **Appli famille** | PAR, MAJ, ELV | demande de sortie exceptionnelle, régime choisi, avis de sortie et de remise, QR de l'élève, « prochaine sortie autorisée » | QR dynamique : non (exige le réseau) ; carte imprimée en secours |

Règles hors ligne : chiffrement par une clé d'appareil, effacement à minuit ou à la fin de l'alerte + 24 h, aucune coordonnée de famille hors ligne hors fiche de secours de l'accueil (R-AS11), resynchronisation avec horodatage réel et détection des conflits (deux remises du même élève, R-AS12).

---

## 17. Cas limites (catalogue CAS-01…)

### 17.1 Sorties et régimes

| N° | Cas | Ce que fait ScolIA |
|---|---|---|
| CAS-01 | Parents séparés : la mère coche « sortie libre en fin de journée », le père a déclaré s'opposer à toute sortie seule | désaccord connu → option la plus restrictive appliquée, CPE alertée, les deux titulaires informés de la règle appliquée ; jamais d'arbitrage automatique (C. civ. 372-2) |
| CAS-02 | Élève qui devient majeur le 14 mars | jusqu'au choix de l'élève, l'autorisation de la famille s'applique ; à partir du 14 mars il choisit ; ses parents ne voient plus ses sorties sauf accord (R-A24) |
| CAS-03 | Cours de 16 h annulé à 15 h 50 | délai minimal non respecté (S-1402) → l'élève reste en permanence ; pas d'avis de sortie anticipée |
| CAS-04 | Cours annulé la veille | sortie anticipée calculée, avis aux titulaires et personnes de confiance (8), décision visible de l'élève |
| CAS-05 | Élève externe de 6ᵉ, journée qui finit à 15 h, mais la règle du collège impose la prise en charge jusqu'à 16 h 30 pour les 6ᵉ | règle `interdit` pour le niveau → permanence ; « prochaine sortie autorisée 16 h 30 » |
| CAS-06 | Élève en retenue de 16 h à 17 h | décision « refusée : retenue jusqu'à 17 h » au portail, sans le motif de la retenue |
| CAS-07 | Élève à l'infirmerie au moment d'une sortie autorisée | « à vérifier : à l'infirmerie » ; jamais le motif |
| CAS-08 | Élève participant d'une sortie collective revenue à 15 h | la remise du retour vaut sortie (42, CV-46) ; la décision de sortie applique ensuite le régime |
| CAS-09 | Changement de règles en cours d'année (nouveau règlement intérieur voté en mars) | nouvelle version datée ; les options qui n'existent plus deviennent « à reconfirmer » ; ancienne version gardée et lisible |
| CAS-10 | Famille qui retire son autorisation par téléphone | l'accueil ne modifie pas l'autorisation : il crée une **restriction temporaire** « à la demande d'un titulaire », valable jusqu'à confirmation écrite (48 h), CPE prévenue |
| CAS-11 | Lycéen interne qui sort le mercredi après-midi | règle de l'internat (41) ; le 14 applique le régime de sortie de jour, le 41 ses propres autorisations |
| CAS-12 | Élève qui change de site dans la journée (lycée sur deux sites) | l'ancienne option `autoriser_changement_site` (065) devient une règle `trajet_inter_sites` ; un retard au second site est un retard ordinaire (12) |
| CAS-13 | Élève de l'unité localisée pour l'inclusion scolaire (ULIS) dont le projet prévoit une remise systématique | règle individuelle « remise obligatoire » posée par la CPE (sans motif) ; s'impose au régime |
| CAS-14 | Demande de sortie exceptionnelle pendant une évaluation | signalée au décideur (« évaluation de mathématiques prévue ») ; décision humaine |
| CAS-15 | Demande déposée par un mineur pour lui-même | préparée seulement ; le titulaire reçoit « Hugo demande à sortir à 14 h ; confirmez-vous ? » (DR-07) |
| CAS-16 | Mot papier présenté par l'élève, signature douteuse | la vie scolaire appelle un titulaire (numéro du dossier) ; jamais de validation sur le seul papier si l'établissement l'a choisi (S-1404) |
| CAS-17 | Jour de grève : pas de cours l'après-midi | fermeture partielle (5, 10) → sortie anticipée pour ceux qui ont le droit ; accueil des autres |
| CAS-18 | Élève sous mesure de protection (placement ASE) | `autoriser_sorties` porté par le service gardien ou l'assistant familial selon le dossier (4, C9) ; les parents ne signent pas |

### 17.2 Remise

| N° | Cas | Ce que fait ScolIA |
|---|---|---|
| CAS-20 | Le père, titulaire non interdit, se présente sans pièce d'identité ; l'établissement exige une pièce | « pièce exigée » ; si l'accueil reconnaît la personne (photo activée, ou connue), un membre de la vie scolaire peut confirmer l'identité (deux gestes) ; sinon accord à distance de l'autre titulaire ou attente |
| CAS-21 | Personne interdite de remise qui se présente | bouton « remis » désactivé ; « prévenir la direction » ; main courante ; jamais le motif à l'écran |
| CAS-22 | Interdiction `demande_titulaire` (non bloquante, sans décision de justice) | « signalement à vérifier » ; remise seulement après accord d'un membre de la direction, journalisé (R-AS3) |
| CAS-23 | Grand-mère autorisée le mercredi qui vient un jeudi | « non autorisée aujourd'hui » ; accord à distance proposé |
| CAS-24 | Deux personnes autorisées se présentent en même temps pour le même enfant (conflit familial) | la remise reste possible à l'une ; l'accueil peut appeler la direction ; fait de main courante si tension |
| CAS-25 | Personne autorisée mineure (grand frère de 16 ans) | autorisée seulement si le titulaire l'a choisi explicitement et que le règlement le permet (S-1406, défaut : 16 ans minimum) [R, Q-AS3] |
| CAS-26 | Taxi ou ambulance mandatés par la famille | personne morale : remise au chauffeur seulement si la famille l'a déclaré (nom de la société, heure) ; pièce professionnelle ; fait tracé |
| CAS-27 | Panne réseau à 16 h 30 | fiche de secours du jour ; remises saisies hors ligne, envoyées au retour, horodatage réel ; conflit détecté si deux remises (R-AS12) |
| CAS-28 | Remise après une alerte, au point de regroupement, parent en panique sans pièce | mode crise : titulaire ou autorisé seulement ; si pas de pièce, reconnaissance par deux personnels ou accord d'un autre titulaire ; jamais à un inconnu |
| CAS-29 | Enfant qui refuse de partir avec la personne autorisée | aucune remise forcée ; CPE ou direction appelée ; fait de main courante ; proposition d'information préoccupante si les faits l'appellent (36) |
| CAS-30 | Personne autorisée en état apparent d'ivresse | l'accueil peut **refuser** la remise (fait de main courante, direction, autre titulaire appelé) : le logiciel permet le refus d'une remise autorisée, jamais l'inverse |
| CAS-31 | Élève renvoyé au domicile par l'infirmerie, famille injoignable | la sortie santé reste « en attente » ; l'élève reste à l'infirmerie ; aucun renvoi seul d'un mineur sans accord (34) |

### 17.3 Visiteurs, accès, main courante

| N° | Cas | Ce que fait ScolIA |
|---|---|---|
| CAS-40 | Visiteur dont le nom ressemble à une personne refusée (homonyme) | alerte « nom proche d'une personne à ne pas laisser entrer — vérifier avec la direction » ; jamais de refus automatique ; la direction confirme ou écarte |
| CAS-41 | Journaliste devant l'établissement | consigne de la direction ; main courante ; aucune information sur des élèves |
| CAS-42 | Prestataire de travaux pendant quinze jours avec cinq ouvriers | accès temporaire, liste nommée, pointage du jour ; présents en cas d'alerte |
| CAS-43 | Visiteur non sorti à la fermeture | alerte douce du soir ; clôture par l'accueil (« parti sans badge ») ; compté présent jusque-là |
| CAS-44 | Parent convoqué en conseil de discipline | registre : « rendez-vous direction » ; jamais l'objet |
| CAS-45 | Ancien élève qui veut « dire bonjour » | visite ordinaire ; accès selon consigne ; pas de lien avec son ancienne fiche (archivée) |
| CAS-46 | Inspecteur ou autorité | qualité « autorité » ; pièce professionnelle ; hôte direction |
| CAS-47 | Livreur sans entrer | aucun enregistrement nominatif par défaut (S-1407 : livraisons enregistrées ou non) |
| CAS-48 | Personne refusée d'accès qui est aussi titulaire de l'autorité parentale | accès refusé ≠ perte des droits sur la scolarité ; la consigne dit « rendez-vous accompagné possible avec la direction » ; ses avis continuent (R-AS9) |
| CAS-49 | Fait de main courante écrit avec une donnée de santé (« l'élève a fait une crise d'épilepsie ») | lint : « information de santé détectée : la saisir à l'infirmerie (34) » ; le texte est refusé tant qu'il n'est pas reformulé |
| CAS-50 | Demande d'accès au registre des visiteurs par la police | réquisition : geste de la direction, export horodaté, journalisé ; jamais d'accès direct (§21) [NV : forme de la réquisition, Q-AS7] |

### 17.4 Sûreté

| N° | Cas | Ce que fait ScolIA |
|---|---|---|
| CAS-60 | Alerte attentat-intrusion à 10 h 12, 900 élèves, 3 bâtiments | listes de comptage par zone d'après l'EDT de 10 h 12 ; vibration seulement ; silence de crise ; tableau de la cellule |
| CAS-61 | Élève réfugié dans une autre salle que celle de son cours | ajouté par le professeur d'accueil (nom tapé) ; retiré de la liste d'origine comme « compté ailleurs » |
| CAS-62 | Élève compté dans deux zones | doublon signalé à la cellule ; pas de blocage |
| CAS-63 | Professeur absent non remplacé : sa classe est en permanence | la liste de la permanence contient la classe (EDT réel, 10) |
| CAS-64 | Alerte pendant la pause méridienne | pas de séance : élèves attendus = demi-pensionnaires et internes présents (appel de cantine si badge, sinon `prevu`), externes `inconnu` ; listes par zone de la cour et du self selon le plan |
| CAS-65 | Alerte pendant une sortie collective | participants comptés « hors établissement, sortie X » ; l'encadrant reçoit une alerte « information » (pas les consignes de confinement) |
| CAS-66 | Alerte déclenchée par erreur | « annuler — envoyé par erreur » (NTF-91) ; l'alerte passe `annulee_erreur`, compte rendu court obligatoire |
| CAS-67 | ScolIA injoignable pendant l'alerte | plan B (NTF-142) ; comptage papier (liste imprimable du jour par zone, générée chaque matin et mise dans les mallettes, S-1408) ; saisie après coup |
| CAS-68 | La direction veut un exercice PPMS « surprise » | refusé pour le PPMS (la circulaire de 2023 exige des exercices sans effet de surprise, préparés avec les forces de sécurité) ; liste des préalables à cocher avant lancement (R-AS15) |
| CAS-69 | Personnel arrivé depuis une semaine sans zone PPMS | dans sa liste de comptage figure la séance de l'EDT ; tableau du référent : « 1 personnel sans zone » |
| CAS-70 | Élève en fauteuil à l'étage pendant une évacuation, AESH absente | aidant suppléant désigné (35) ; s'il manque, alerte à la cellule « aide à l'évacuation non affectée : salle B204 » |
| CAS-71 | Interne la nuit (alerte incendie à 2 h) | présents de nuit (41) ; zones de l'internat ; surveillant de nuit responsable |
| CAS-72 | Alerte limitée au site B | périmètre site ; élèves dont l'EDT est au site B ; familles concernées seulement (NTF-140) |
| CAS-73 | Parents qui arrivent en masse pendant un confinement | consigne de la direction (« ne venez pas chercher vos enfants ») portée par le 8 ; remise seulement après la fin d'alerte et au point de regroupement |
| CAS-74 | Compte rendu d'exercice jamais signé | rappel à J+7, J+15 ; compteur de conformité « exercice non clos » |

### 17.5 Données, droits, transition

| N° | Cas | Ce que fait ScolIA |
|---|---|---|
| CAS-80 | Un AED consulte « où est » un élève sans motif | `edt::localiser` exige un motif codé (remise, recherche, urgence) ; consultation journalisée, visible de la direction |
| CAS-81 | Un professeur veut la liste des sorties de ses élèves | il voit, sur sa liste d'appel, « sorti à 14 h, autorisé » pour sa séance ; pas l'historique |
| CAS-82 | Un parent demande l'historique des sorties de son enfant | il voit les sorties exceptionnelles et remises de son enfant (année en cours) ; pas les sorties ordinaires non journalisées |
| CAS-83 | Fusion de deux fiches élève (2, E6) | autorisations, demandes, remises et passages suivent l'élève conservé |
| CAS-84 | Élève parti de l'établissement en cours d'année | autorisations closes ; données conservées selon §21.3 puis purgées |
| CAS-85 | Établissement qui garde Next.js accueil pendant la transition | §26 : une seule source de saisie par jour ; Next.js arrêté par Florian |
| CAS-86 | Deux établissements du même organisme | RLS ; aucune remise, aucun visiteur, aucune alerte visible de l'autre (sauf site commun déclaré, S-1409) |

---

## 18. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md` §5, §6.2)

### 18.1 Ce que l'IA fait [R]
- **Déclaration dictée à l'accueil** (reprise de NXA-09) : « Paul Martin absent deux jours, sa mère a appelé » → aperçu de la déclaration (élève reconnu, période, appelant parmi les personnes liées) ; **N2** ; l'agent valide.
- **Traduction du règlement intérieur** en règles de sortie : à partir du texte du règlement (déposé au socle 7), l'IA **propose** un jeu de règles et d'options, chacune citant la phrase source ; la direction relit règle par règle ; **N2**, rien ne s'applique sans approbation de la version (ACS-122).
- **Préparation du PPMS** : à partir du modèle officiel et des lieux du socle 5, proposition de zones, de fiches réflexes et d'une liste des écarts (« la salle B204 n'est dans aucune zone ») ; **N1** (brouillon).
- **Compte rendu d'exercice** : rédaction du brouillon à partir des faits chronométrés et des constats des observateurs (non nominatifs) ; **N1** ; la direction signe.
- **Point de situation en crise** : brouillon court à partir de faits **non nominatifs** ; **N3** (envoi par la direction seule, socle 8).
- **Questions** : « qui peut chercher Léa aujourd'hui ? » → réponse **identique à l'écran de remise** (interdictions en tête, sans motif), journalisée comme une consultation `remise.lire` ; « Hugo peut-il sortir à 15 h ? » → décision et raison (`sortie::decider`).
- **Récapitulatif « Faits établissement »** à partir d'un fait de main courante qualifié par la direction (structure Quoi / Qui anonymisé / Quand / Où / Pourquoi) ; **N1** ; jamais transmis par ScolIA.

### 18.2 Ce que l'IA ne fait pas [R]
- Elle ne **remet** aucun élève, ne **décide** aucune sortie exceptionnelle, ne **lève** ni ne **pose** de restriction ou d'accès refusé, ne **déclenche** ni ne **lève** d'alerte, ne **clôt** pas une recherche d'élève manquant, ne **qualifie** pas un fait de main courante : **fermé à l'IA**.
- Elle ne lit **jamais** : le motif d'une interdiction ou d'un accès refusé, les données de santé (même « à l'infirmerie » au-delà du oui/non), l'aide à l'évacuation au-delà de « aide : oui », la liste des personnes nécessitant une attention particulière du PPMS, l'annuaire de crise, l'adresse protégée, les comptages nominatifs d'une alerte en cours.
- Elle ne **devine** pas où se trouve un élève manquant et ne **reconnaît** aucun visage.
- Pendant une **alerte en cours**, l'assistant est **désactivé** pour tout ce qui touche la sûreté, sauf le brouillon de point de situation demandé par la direction [R].

### 18.3 Capacités déclarées (registre `ent-ia`)

| Capacité | Droit | Portée | Sensibilité | Confirmation | Réversible |
|---|---|---|---|---|---|
| `accueil.declarer_absence` | `absences.declarer` | établissement | moyenne | N2 | oui (annulation) |
| `sortie.decider_lire` | `sorties.controler` ou lien famille | élève | moyenne | N0 | — |
| `remise.lire` | `remise.lire` | élève, jour | **haute** | N0 (journalisé) | — |
| `sortie.regles_proposer` | `sorties.regles.gerer` | établissement | faible | N2 | oui |
| `surete.ppms_brouillon` | `surete.ppms.gerer` | établissement | moyenne | N1 | oui |
| `surete.compte_rendu_brouillon` | `surete.exercices.gerer` | exercice | moyenne | N1 | oui |
| `surete.point_situation_brouillon` | `surete.alerte.piloter` | alerte | haute | N3 | non (envoi) |
| `accueil.faits_etab_recap` | `accueil.main_courante.qualifier` | fait | haute | N1 | oui |

### 18.4 Cadre
Mêmes règles que le cœur IA : l'IA n'a jamais plus de droits que l'utilisateur, passe par les services métier, chaque proposition affiche ses sources ; jeu d'évaluation du socle : 40 déclarations dictées (homonymes, dates relatives, appelant inconnu), 10 règlements intérieurs réels anonymisés [H : à fournir par Florian], 5 comptes rendus ; tests de garde (équivalence écran / IA sur `remise.lire`, refus sur toutes les commandes fermées, aucune donnée de santé dans les contextes envoyés au modèle). Étude juridique du règlement européen sur l'IA avant mise en service des fonctions à modèle de langage (comme D-032).

---

## 19. Actions métier (ACS-100 à ACS-329)

Colonnes : acteur et droit (D-017, catalogue au §21.1) ; règles (niveau de confirmation N1 à N3 de `IA_COEUR.md` §6.2, valable aussi sans IA pour les écritures sensibles ; ★ = action sensible avec récapitulatif et passkey récente) ; événement émis ; lot. **(NXA-xx)** = action existante de Next.js reprise ; **(DR-xx)** = correctif de l'existant Rust ; « nouveau » sinon.

### 19.1 Accueil, appels et déclarations

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-100 | Rechercher un élève, un personnel ou une personne en trois lettres, avec photo et classe (NXA-07) | ACC, VS · `accueil.rechercher` | résultat minimisé (nom, prénom, classe, photo) ; homonymes distingués par la classe ; même établissement | — | AS1 |
| ACS-101 | Voir **où est l'élève maintenant** (séance en cours, salle, professeur) | ACC, VS · `accueil.localiser` | motif codé obligatoire (remise, recherche, urgence, parent présent) ; consultation journalisée (`edt::localiser`) | — | AS1 |
| ACS-102 | Voir le **tableau de l'accueil du jour** : déclarations, sorties attendues, remises prévues, visiteurs attendus, retours de car | ACC · `accueil.tableau` | tuiles Bento ; aucune donnée de santé | — | AS1 |
| ACS-103 | Appeler la vie scolaire depuis la fiche (raccourci téléphone, message interne) | ACC | numéro professionnel du poste (6) | — | AS1 |
| ACS-104 | Noter l'**appelant** d'une déclaration : personne liée choisie, ou « autre, à rappeler » (VA-03) | ACC | liste des titulaires et personnes liées (4), jamais les coordonnées affichées en entier | — | AS1 |
| ACS-105 | Marquer une déclaration « **à confirmer** » (appelant non reconnu, doute) | ACC | le 12 la traite comme non prévenue jusqu'à confirmation | `accueil.declaration_a_confirmer` | AS1 |
| ACS-106 | Confirmer une déclaration par **rappel sur un numéro du dossier** | ACC, VS | jamais le numéro donné par l'appelant (R-A13) | `accueil.declaration_confirmee` | AS1 |
| ACS-107 | Enregistrer un **message reçu** à transmettre (« le père de Léa demande à être rappelé par la CPE ») | ACC | fil interne vers le destinataire (16) ; pas de contenu sensible (lint) | `messagerie.mot_interne` | AS1 |
| ACS-108 | Voir les **professeurs absents aujourd'hui** (sans motif) pour répondre aux familles | ACC · `rh.absents.lire` | socle 15 ; « absent » seulement | — | AS1 |
| ACS-109 | Voir les **séances annulées** et les sorties anticipées du jour | ACC | 10, 14 | — | AS1 |
| ACS-110 | **Déclarer une absence ou un retard d'élève** reçu par téléphone, sur place ou par courriel (NXA-01) | ACC, VS · `absences.declarer` | commande `assiduite::declarer(source = accueil)` du 12 ; période aujourd'hui / plage / plusieurs jours ; retard 8 h 00–8 h 30 par défaut ; motif codé ; commentaire ≤ 200 caractères lint de minimisation (VA-02) ; N2 si l'IA prépare | `accueil.declaration_transmise` (+ `vs.absence_declaree` du 12) | AS1 |
| ACS-111 | Déclarer pour **plusieurs enfants** d'un même foyer en un geste (« mes deux enfants sont malades ») | ACC | fratrie lue au socle 4 (élèves de l'établissement seulement) | idem | AS1 |
| ACS-112 | **Garde-fou « en sortie »** : avertir que l'élève est en sortie collective ce jour-là ; forcer avec motif (NXA-03, VA-06) | ACC · `absences.declarer` | port `voyages::en_sortie` (42) ; motif obligatoire si forcé | — | AS1 |
| ACS-113 | **Tableau des déclarations de l'accueil du jour** avec auteur, appelant, état (transmise, à confirmer, annulée, rapprochée) (NXA-04) | ACC, VS | filtré par droits ; pas de ligne de personnel hors socle 15 | — | AS1 |
| ACS-114 | **Annuler** une déclaration de l'accueil (NXA-05) | déclarant, VS · `absences.declarer` | annulation datée motivée ; impossible après décision du 12 ; jamais de suppression (corrige DR-13 côté 12) | `accueil.declaration_annulee` | AS1 |
| ACS-115 | Consulter les déclarations de l'accueil depuis la vie scolaire (NXA-06) | CPE, VS | écran du 12 ; filtre « source accueil » | — | AS1 |
| ACS-116 | **Raccourci « un personnel appelle pour prévenir »** : ouvre la déclaration du socle 15 avec les droits du 15 (NXA-02, VA-05) | ACC · droit du 15 | aucun motif visible de l'accueil | (15) | AS1 |
| ACS-117 | Recevoir un **appel « je viens chercher mon enfant à … »** : crée une sortie exceptionnelle à valider et prépare la remise | ACC | → ACS-146, ACS-182 | `sortie.exception_demandee` | AS3 |
| ACS-118 | Recevoir une **consigne de remise par téléphone** (« son oncle viendra ») : ouvre ADM-88 du socle 4 | ACC · `remise.lire` | autorisation ponctuelle seulement après rappel | (4) | AS4 |
| ACS-119 | Imprimer ou exporter la **liste du jour** de l'accueil (sorties attendues, remises, visiteurs attendus) | ACC | papier marqué « confidentiel, détruire ce soir » ; aucune interdiction imprimée hors fiche de secours (ACS-195) | `accueil.liste_imprimee` (journal) | AS4 |

### 19.2 Règles et autorisations de sortie

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-120 | Créer une **version de règles de sortie** pour l'année (et le site) à partir d'un **modèle** collège ou lycée | DIR, CPE · `sorties.regles.gerer` | modèles conformes à la fiche 34 (§5.5) ; brouillon | `sortie.regles_brouillon` | AS2 |
| ACS-121 | Ajouter, modifier, ordonner une **règle** (public, créneau, effet, heures) | DIR, CPE · `sorties.regles.gerer` | créneaux du §10.2 ; **écart au modèle signalé** (R-AS16) | — | AS2 |
| ACS-122 | **Approuver** une version de règles, avec date d'effet ★ | DIR · `sorties.regles.approuver` | N2 ; empreinte des textes d'option figée ; référence de la délibération du CA (public) ou de l'organisme (privé) facultative | `sortie.regles_approuvees` | AS2 |
| ACS-123 | **Simuler** une version de règles sur les élèves réels (« combien d'élèves changent de droit ? ») avant approbation | DIR, CPE | lecture ; liste des élèves touchés, par niveau | — | AS2 |
| ACS-124 | Rédiger le **texte des options** proposées aux familles, par langue | CPE | relu ; figé par version | — | AS2 |
| ACS-125 | Proposer les options de sortie dans la **fiche de rentrée** (4 / 18) | SYS | une question par option `sur_option_famille` ; texte de la version | — | AS2 |
| ACS-126 | **Choisir les options** de sortie de son enfant (en ligne) | PAR · lien `autoriser_sorties` | titulaire au lien actif ; accord de l'autre présumé (C. civ. 372-2) sauf désaccord déclaré ; N2 côté famille (récapitulatif) | `sortie.autorisation_choisie` | AS2 |
| ACS-127 | Choisir ses options **lui-même** (élève majeur) | MAJ | dans les limites du règlement ; parents informés selon R-A24 | idem | AS2 |
| ACS-128 | Saisir les options d'après un **coupon papier** | VS · `sorties.gerer` | signataire choisi parmi les titulaires ; photo du coupon facultative (7, conservation courte) | idem | AS2 |
| ACS-129 | **Valider** les autorisations (en masse pour les cas simples) | CPE · `sorties.gerer` | tant que non validée : régime le plus restrictif ; N2 | `sortie.autorisation_validee` | AS2 |
| ACS-130 | Refuser une option choisie, motif visible de la famille | CPE | motif court codé | `sortie.autorisation_refusee` | AS2 |
| ACS-131 | Enregistrer un **désaccord** d'un titulaire sur les options | PAR, VS | option la plus restrictive ; CPE alertée ; les deux informés (CAS-01) | `sortie.desaccord` | AS2 |
| ACS-132 | **Modifier** les options en cours d'année (nouvelle version datée) | PAR, MAJ, VS | effet à la date de validation ; historique | `sortie.autorisation_modifiee` | AS2 |
| ACS-133 | Voir l'**historique** des options et des validations d'un élève | CPE, PAR (son enfant) | versions datées, auteur | — | AS2 |
| ACS-134 | Voir les **élèves sans autorisation choisie** et relancer les familles | CPE | relance par le socle 8 (nature `sortie.autorisation_a_choisir`) ; N3 pour l'envoi en nombre | `notifications.diffusion_lancee` | AS2 |
| ACS-135 | Poser une **règle individuelle** (remise obligatoire, sortie interdite le midi) sans motif visible | CPE · `sorties.gerer` | date de fin ; motif scellé facultatif (CAS-13) | `sortie.regle_individuelle` | AS2 |
| ACS-136 | Marquer les autorisations « **à revoir** » après changement de régime, de lien, de majorité, de règles | SYS | §6.4 | `sortie.autorisation_a_revoir` | AS2 |
| ACS-137 | Reprendre les autorisations et règles de l'**ancien modèle** 062–065 | SYS, ADM | correspondance matin/midi/soir → créneaux ; rapport de reprise ; anciens enregistrements gardés en lecture | `sortie.reprise_faite` | AS2 |
| ACS-138 | Voir le **tableau des régimes de sortie** par classe (combien d'élèves par option) | CPE, DIR | agrégé ; nominatif à la demande | — | AS2 |
| ACS-139 | Exporter la liste des autorisations d'une classe (pour un remplaçant, un AED nouveau) | CPE | journalisé ; sans coordonnées | `sortie.export` | AS2 |
| ACS-140 | Paramétrer les **créneaux** de l'établissement (heures de la pause méridienne, fin de journée minimale par niveau) | DIR · `sorties.regles.gerer` | lu du socle 5 quand il existe | — | AS2 |
| ACS-141 | Paramétrer les **trajets inter-sites** (ancien `autoriser_changement_site`) | DIR | règle `trajet_inter_sites` | — | AS2 |
| ACS-142 | Consulter la **règle appliquée** et sa source (phrase du règlement) | tous les personnels habilités, PAR, ELV | explication en une phrase | — | AS2 |
| ACS-143 | Inviter l'élève qui **devient majeur** à confirmer ses options | SYS | avis à l'élève ; titulaires selon R-A24 | `sortie.majorite_invitation` | AS2 |
| ACS-144 | Clore les autorisations à la **sortie des effectifs** | SYS | à la date | `sortie.autorisation_close` | AS2 |

### 19.3 Sorties exceptionnelles, restrictions, décision du jour

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-145 | **Demander une sortie exceptionnelle** dans l'appli (date, heure, motif codé, retour prévu, qui vient chercher) | PAR · `autoriser_sorties` ; MAJ | commentaire sans santé (lint) ; personne choisie parmi les autorisées ou « seul » si permis | `sortie.exception_demandee` | AS3 |
| ACS-146 | Saisir une demande pour une famille (appel, mot papier) | ACC, VS · `sorties.gerer` | auteur et canal notés ; photo du mot facultative | idem | AS3 |
| ACS-147 | **Préparer** une demande (élève mineur), confirmée par un titulaire (DR-07) | ELV | sans confirmation à H-1 : caduque | `sortie.exception_preparee` | AS3 |
| ACS-148 | **Décider** une demande : accepter, refuser motivé, accepter avec remise obligatoire | CPE, VS · `sorties.gerer` | conflits signalés (évaluation 21, examen 38, retenue 13) ; N2 | `sortie.exception_decidee` | AS3 |
| ACS-149 | Acceptation **automatique** des cas simples (si l'établissement l'a choisi) | SYS | S-1403 ; jamais pendant une évaluation ni pour un mineur sans titulaire | idem | AS3 |
| ACS-150 | Annuler une demande (famille, avant l'heure) | PAR, MAJ, VS | datée | `sortie.exception_annulee` | AS3 |
| ACS-151 | Autoriser une **sortie pour nécessité impérieuse** (direction, sans demande écrite) ★ | DIR · `sorties.imperieuse` | « après avoir, si possible, prévenu » les représentants légaux (fiche 34) : appel tenté obligatoirement noté | `sortie.imperieuse` | AS3 |
| ACS-152 | Voir les **sorties exceptionnelles du jour** (accueil, vie scolaire, professeur de la séance) | ACC, VS, PRF (sa séance) | professeur : « sort à 14 h, autorisé » seulement | — | AS3 |
| ACS-153 | Marquer la sortie **effectuée** et le **retour** (« revenu à 16 h 05 ») | ACC, AED · `sorties.controler` | horodatage serveur (DR-03) ; l'appel reprend | `sortie.effectuee`, `sortie.retour` | AS3 |
| ACS-154 | Alerter si l'élève **n'est pas revenu** à l'heure annoncée + N minutes | SYS | → recherche (ACS-235) proposée | `sortie.retour_manquant` | AS3 |
| ACS-155 | Poser une **restriction** de sortie datée et motivée ★ | CPE, DIR · `sorties.restreindre` | motif notifié aux titulaires ; durée maximale paramétrable ; peut être **proposée** par le 13, jamais imposée par lui | `sortie.restriction_posee` | AS3 |
| ACS-156 | Lever une restriction | CPE, DIR | datée | `sortie.restriction_levee` | AS3 |
| ACS-157 | Enregistrer une **restriction temporaire à la demande d'un titulaire** par téléphone (CAS-10) | ACC, VS | 48 h sans confirmation écrite ; CPE prévenue | `sortie.restriction_posee` | AS3 |
| ACS-158 | Calculer **« peut-il sortir maintenant ? »** (`sortie::decider`) (DR-02, DR-05) | SYS | fonction pure ; heure serveur dans le fuseau (DR-03) ; lien actif, élève actif, régime daté (DR-04) | — | AS0 / AS3 |
| ACS-159 | Calculer la **prochaine sortie autorisée** de l'élève du jour | SYS | affichée à l'élève, à la famille, à l'accueil | — | AS3 |
| ACS-160 | Calculer la **sortie anticipée** après une modification d'EDT | SYS | délai minimal (S-1402) ; collège : fin de demi-journée ou de journée seulement (R-AS16) | `surete.sortie_anticipee_calculee` | AS3 |
| ACS-161 | Faire **prévenir** familles et personnes de confiance d'une sortie anticipée | SYS → 8 | nature `edt.sortie_anticipee` (U2) ; seulement les élèves autorisés | `notifications.avis_cree` | AS3 |
| ACS-162 | Voir les **élèves à garder en permanence** après une annulation (non autorisés à sortir) | VS, CPE | vers l'appel de permanence (12, EDT-177) | — | AS3 |
| ACS-163 | Pré-remplir l'appel des séances suivantes « **sorti, autorisé** » ou « sorti, remis » | SYS → 12 | port du 12 | (12) | AS3 |
| ACS-164 | Voir l'**historique des sorties exceptionnelles** d'un élève (année) | CPE, PAR (son enfant), MAJ | décisions, effectuées, retours | — | AS3 |

### 19.4 Contrôle aux portes

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-165 | **Contrôler une sortie par le nom** (trois lettres, photo) | AED, ACC · `sorties.controler` | AUTORISÉ / REFUSÉ / À VÉRIFIER + raison ; DR-02, DR-03 | `sortie.controle` (exceptions seulement, §11.3) | AS0 / AS5 |
| ACS-166 | **Contrôler par la carte** (code-barres ou QR imprimé, numéro de badge du socle 2) | AED, ACC | lecture caméra ; carte déclarée perdue → refus « carte perdue » | idem | AS5 |
| ACS-167 | **Contrôler par le QR dynamique** de l'appli élève | AED, ACC | QR signé, renouvelé toutes les 30 s, lié à l'appareil ; capture refusée | idem | AS5 |
| ACS-168 | Enregistrer un **refus** (élève, heure, raison) | AED | toujours journalisé | `sortie.refusee` | AS5 |
| ACS-169 | Signaler un **élève parti sans autorisation** | AED, ACC, PRF | ouvre ACS-235 ; avis vie scolaire U3 interne | `sortie.sans_autorisation` | AS5 |
| ACS-170 | Ouvrir le **mode portail** d'un appareil (poste nommé, heure, opérateur) | AED, ACC | appareil enregistré ; session personnelle | `accueil.poste_ouvert` | AS5 |
| ACS-171 | **Contrôle hors ligne** : décisions du jour précalculées, envoi différé | SYS | chiffré, effacé à minuit (R-AS11) | `sortie.controles_synchronises` | AS5 |
| ACS-172 | Voir la **file des sorties attendues** à la prochaine sonnerie (sorties exceptionnelles, remises) | AED, ACC | — | — | AS5 |
| ACS-173 | **Journaliser toutes les sorties** (option de l'établissement) | DIR · `sorties.regles.gerer` | S-1401 ; information des familles obligatoire ; conservation courte | — | AS5 |
| ACS-174 | Saisir un contrôle **a posteriori** (panne, oubli) | VS · `sorties.gerer` | marqué, motif, heure réelle déclarée (DR-03) | `sortie.controle` | AS0 |
| ACS-175 | Voir le **tableau des refus** du jour et de la semaine | CPE | nominatif ; aide à repérer un élève qui tente souvent | — | AS5 |
| ACS-176 | Déclarer une **carte perdue** et en faire imprimer une nouvelle (socle 2) | VS · `eleves.badge.gerer` | ancienne carte refusée immédiatement | `eleve.badge_perdu` (2) | AS5 |
| ACS-177 | Afficher à l'élève **sa carte** dans l'appli (QR) et « aujourd'hui tu peux sortir à … » | ELV, MAJ | — | — | AS5 |
| ACS-178 | Brancher un **lecteur de contrôle d'accès** existant (connecteur, socle 9) | ADM | lecture seule des passages ; ScolIA ne pilote pas de portique | `accueil.connecteur` | AS12 |
| ACS-179 | Contrôler l'**entrée** d'un élève sorti dans la journée (retour) | AED, ACC | → ACS-153 | `sortie.retour` | AS5 |

### 19.5 Remise de l'élève

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-180 | Ouvrir l'**écran de remise** d'un élève : interdictions en tête (consignes, jamais le motif), autorisés **aujourd'hui** avec pièce exigée et photo si activée | ACC, VS, INF · `remise.lire` | port `personnes::remise(eleve, jour)` ; consultation journalisée | — | AS4 |
| ACS-181 | Retrouver l'élève **à partir de la personne** qui se présente (« je viens chercher Léa et Tom ») | ACC · `remise.lire` | recherche par personne connue du socle 4 : ses enfants dans l'établissement **pour lesquels elle est autorisée aujourd'hui** seulement | — | AS4 |
| ACS-182 | **Préparer une remise** (sortie exceptionnelle, renvoi au domicile, retour de car) | VS, INF, SYS · `remise.lire` | port `accueil::remise_preparer` ; visible de l'accueil | `accueil.remise_preparee` | AS4 |
| ACS-183 | **Remettre** l'élève à une personne autorisée, pièce vérifiée (type seulement) | ACC, VS · `remise.effectuer` | interdiction → bouton désactivé ; `demande_titulaire` → accord direction (R-AS3) ; horodatage serveur ; N2 | `surete.remise_effectuee` | AS4 |
| ACS-184 | Faire **confirmer l'identité** d'une personne sans pièce par un second personnel | VS, DIR | deux gestes, deux personnes ; journalisé (CAS-20) | idem | AS4 |
| ACS-185 | **Demander l'accord à distance** d'un titulaire (ADM-87) | ACC · `remise.lire` | réponse en 10 min sinon appel ; l'interdiction l'emporte | (4) | AS4 |
| ACS-186 | Traiter une **personne inconnue se disant responsable** (ADM-89) | ACC → DIR | aucune remise ; fait de main courante automatique | `accueil.fait_cree` | AS4 |
| ACS-187 | **Refuser une remise** à une personne pourtant autorisée (ivresse, comportement) | ACC, VS | motif codé ; direction prévenue ; autre titulaire appelé (CAS-30) | `accueil.remise_refusee` | AS4 |
| ACS-188 | **Prévenir la direction** d'une tentative de remise par une personne interdite | ACC | U3 interne ; fait de main courante ; aucun motif affiché | `accueil.remise_interdite_tentee` | AS4 |
| ACS-189 | Remettre au **chauffeur** d'un taxi ou d'une ambulance mandatés par la famille | ACC | déclaration préalable du titulaire (société, heure) ; pièce professionnelle (CAS-26) | `surete.remise_effectuee` | AS4 |
| ACS-190 | Faire **avertir les titulaires** de la remise (« remise à Mme X à 15 h 12 ») | SYS → 8 | nature `surete.remise` (U1) ; jamais un titulaire sous interdiction de communication | `notifications.avis_cree` | AS4 |
| ACS-191 | Voir la **sortie santé** préparée par l'infirmerie (qui, quand), sans motif | ACC · `remise.lire` | `sante.renvoi_prepare` (34, SAN-142) | — | AS4 |
| ACS-192 | **Remise au retour d'un car** (42, VOY-207) : liste des participants, autorisés, interdictions en tête, « part seul (autorisé) » | ACC, encadrant · `remise.effectuer` (portée sortie, jour du retour) | port `accueil::remise_preparer` | `surete.remise_effectuee` | AS4 |
| ACS-193 | Alerter quand un participant n'est **pas remis** à l'heure + 30 min (VOY-208) | SYS | — | `sorties.alerte_remise` (42) | AS4 |
| ACS-194 | Voir le **registre des remises** du jour et d'un élève | CPE, DIR ; PAR (son enfant, remises seulement) | nominatif ; parent : « remis à Mme X » si la personne est connue de lui | — | AS4 |
| ACS-195 | Générer la **fiche de secours du jour** (interdictions et autorisés du jour) sur la tablette de l'accueil, chiffrée (ADM-91) | SYS | effacée à minuit ; papier seulement sur demande de la direction | `accueil.fiche_secours_generee` | AS4 |
| ACS-196 | Saisir des remises **faites hors ligne** avec leur heure réelle | ACC | détection de conflit (R-AS12) | `surete.remise_effectuee` | AS4 |
| ACS-197 | **Corriger** une remise (mauvais élève, mauvaise personne) | VS · `remise.effectuer` | correction liée, jamais réécriture ; motif | `surete.remise_corrigee` | AS4 |
| ACS-198 | Enregistrer qu'un élève **est parti seul** alors que la remise était obligatoire | ACC, VS | ouvre ACS-235 | `sortie.sans_autorisation` | AS4 |
| ACS-199 | Remise **en mode crise** au point de regroupement (§14.7) | ACC, VS, DIR · `surete.alerte.remettre` | titulaires et autorisés seulement ; pièce exigée ; suivi « remis / restant » | `surete.remise_effectuee` | AS10 |

### 19.6 Visiteurs, accès temporaires, accès refusés

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-200 | **Enregistrer l'arrivée** d'un visiteur (nom, qualité, hôte, motif codé, pièce présentée : type seulement) | ACC · `accueil.visiteurs` | jamais de photo ni de numéro de pièce (§8.2) ; rapprochement accès refusé (ACS-216) | `accueil.visite_arrivee` | AS6 |
| ACS-201 | Rattacher le visiteur à une **personne connue** (parent, autorisé, intervenant habituel) | ACC | `personne_id` au lieu d'un nom recopié | — | AS6 |
| ACS-202 | Remettre un **badge visiteur** (numéro) ou imprimer une étiquette | ACC | étiquette : nom, hôte, date ; rien sur l'élève visité | — | AS6 |
| ACS-203 | **Prévenir l'hôte** de l'arrivée | SYS → 8 | in-app, push | `notifications.avis_cree` | AS6 |
| ACS-204 | Enregistrer la **sortie** du visiteur et le retour du badge | ACC | — | `accueil.visite_sortie` | AS6 |
| ACS-205 | **Annoncer un visiteur attendu** (date, heure, hôte, motif codé) | tout personnel · `accueil.visiteurs.annoncer` | — | `accueil.visite_annoncee` | AS6 |
| ACS-206 | Voir les **visiteurs attendus aujourd'hui** (annoncés, rendez-vous du 19, intervenants du 6) | ACC | PRS-136 | — | AS6 |
| ACS-207 | Pointer en un geste un **visiteur attendu** | ACC | pré-rempli | `accueil.visite_arrivee` | AS6 |
| ACS-208 | Pointer en nombre les inscrits d'un **événement** (portes ouvertes, réunion de parents, cérémonie) | ACC | inscrits du 3 ou du 19 ; prospects = contacts | `accueil.visite_arrivee` | AS6 |
| ACS-209 | Créer un **accès temporaire** pour un prestataire (période, référent, intervenants nommés) | ADF, DIR, gestionnaire · `accueil.acces_temporaire` | sans pièce conservée ; fin obligatoire | `accueil.acces_temporaire_cree` | AS6 |
| ACS-210 | Pointer chaque jour les **intervenants** d'un accès temporaire | ACC | présents en alerte | `accueil.visite_arrivee` | AS6 |
| ACS-211 | Voir les **visiteurs présents** maintenant | ACC, DIR, cellule | — | — | AS6 |
| ACS-212 | **Alerte du soir** : visiteurs non sortis, badges non rendus | SYS | à l'heure de fermeture (5) | `accueil.visiteurs_non_sortis` | AS6 |
| ACS-213 | Clore une visite non sortie (« parti sans badge ») | ACC | motif | `accueil.visite_sortie` | AS6 |
| ACS-214 | Gérer le **stock de badges** visiteurs (numéros, perdus) | ACC, gestionnaire | — | — | AS6 |
| ACS-215 | **Interdire l'accès** à une personne ★ | DIR · `accueil.acces_refuser` | fondement (R. 421-12 dans le public ; décision de direction) ; consigne ≤ 300 caractères ; motif **scellé** ; date de fin ; informe le CA ou l'organisme selon le cas | `accueil.acces_refuse_pose` (sans nom) | AS6 |
| ACS-216 | **Rapprocher** un visiteur saisi d'une personne refusée (nom proche ou `personne_id`) | SYS | alerte « vérifier avec la direction », jamais de refus automatique (CAS-40) | — | AS6 |
| ACS-217 | Lever un accès refusé | DIR | daté, motivé | `accueil.acces_refuse_leve` | AS6 |
| ACS-218 | Voir la liste des **accès refusés en vigueur** (consignes seulement) | ACC, VS, DIR | motif visible de la direction seule | — | AS6 |
| ACS-219 | Appliquer la **posture Vigipirate** saisie (consignes d'accueil affichées) | DIR · `surete.posture` | niveau saisi, daté ; consignes de l'établissement par niveau | `surete.posture_changee` | AS6 |
| ACS-220 | Noter le **contrôle visuel des sacs** refusé (accès refusé de fait) | ACC | fait de main courante léger | `accueil.fait_cree` | AS6 |
| ACS-221 | Afficher la **mention d'information** RGPD du registre (affiche, QR) | ACC | texte modèle (§21.5) | — | AS6 |
| ACS-222 | **Exporter** le registre des visiteurs sur réquisition ★ | DIR · `accueil.visiteurs.exporter` | période, motif, journalisé (CAS-50) | `accueil.registre_exporte` | AS6 |
| ACS-223 | Voir l'**historique des visites** d'une personne connue (parent) | DIR | 3 mois au plus (§21.3) | — | AS6 |
| ACS-224 | Enregistrer une **livraison** (si l'établissement l'a choisi) | ACC | S-1407 ; société, heure, sans nom par défaut | `accueil.livraison` | AS6 |

### 19.7 Main courante de l'accueil

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-225 | **Inscrire un fait** (catégorie codée, lieu, heure, description factuelle, personnes référencées) | ACC, VS, DIR · `accueil.main_courante` | lint de minimisation (santé, opinions) (CAS-49) ; ajout seul | `accueil.fait_cree` | AS7 |
| ACS-226 | Ajouter une **suite** (appel direction, police, retrouvé, classé) | auteur, VS, DIR | ajout seul | `accueil.fait_suite` | AS7 |
| ACS-227 | Ajouter une **correction** liée (erreur de saisie) | auteur, DIR | l'original reste lisible | `accueil.fait_corrige` | AS7 |
| ACS-228 | **Qualifier** un fait : simple trace / proposer un incident (13) / proposer une information préoccupante (36) / à déclarer en « Faits établissement » ★ | DIR · `accueil.main_courante.qualifier` | propositions, jamais créations | `accueil.fait_qualifie` | AS7 |
| ACS-229 | Préparer le **récapitulatif « Faits établissement »** (Quoi / Qui anonymisé / Quand / Où / Pourquoi, premières mesures, gravité 1 à 3) | DIR | N1 si l'IA prépare ; geste de déclaration dans l'outil national | `accueil.faits_etab_prepare` | AS7 |
| ACS-230 | Voir la main courante (filtres date, catégorie, lieu) | VS, DIR ; ACC (ses 7 derniers jours) | — | — | AS7 |
| ACS-231 | Voir les faits qui **concernent un élève** (sur sa fiche, pour la CPE) | CPE, DIR | jamais les familles | — | AS7 |
| ACS-232 | Rattacher un fait à une **alerte** (main courante de crise) | SYS, cellule | §14.3 | — | AS10 |
| ACS-233 | Exporter la main courante d'une période (CA, assureur, justice) ★ | DIR | journalisé ; pseudonymisation proposée | `accueil.main_courante_exportee` | AS7 |
| ACS-234 | Recevoir un fait **d'un autre socle** (ADM-89 du 4, CS-18 du 34, 42) | SYS | port `accueil::fait_main_courante` | `accueil.fait_cree` | AS7 |

### 19.8 Élève manquant

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-235 | **Ouvrir une recherche** d'élève manquant | tout personnel · `surete.recherche.ouvrir` | un geste ; heure notée ; vie scolaire et CPE prévenues (U3 interne) | `surete.recherche_ouverte` | AS8 |
| ACS-236 | Voir les **vérifications rapides** proposées et les cocher (dernière présence, EDT, infirmerie oui/non, sortie collective, remise, lieux habituels) | VS, CPE · `surete.recherche.mener` | chaque vérification horodatée ; `edt::localiser` journalisé | `surete.recherche_etape` | AS8 |
| ACS-237 | **Appeler les titulaires** joignables depuis la recherche | CPE, VS | numéros du dossier ; jamais un titulaire sous interdiction de communication | `surete.recherche_etape` | AS8 |
| ACS-238 | **Escalader** à la direction (automatique à T + N min, S-1405) | SYS, VS | — | `surete.recherche_escaladee` | AS8 |
| ACS-239 | Noter la **décision de signalement** aux forces de l'ordre (faite au téléphone par la direction) | DIR · `surete.recherche.decider` | ScolIA ne signale jamais lui-même (R-AS5) ; numéros rappelés | `surete.recherche_signalee` | AS8 |
| ACS-240 | **Clore** : retrouvé (où, quand, par qui), remis, signalé | CPE, DIR | compte rendu court | `surete.recherche_close` | AS8 |
| ACS-241 | **Proposer** un incident (13) ou une information préoccupante (36) depuis la recherche close | CPE, DIR | jamais automatique | — | AS8 |
| ACS-242 | Voir les **recherches de l'année** (durées, issues) | DIR, CPE | indicateur (47) | — | AS8 |
| ACS-243 | Partager la **photo** de l'élève aux personnels qui cherchent | CPE | durée de la recherche ; jamais hors personnels | — | AS8 |
| ACS-244 | Ouvrir une recherche **depuis un comptage** d'alerte (manquant) | SYS, cellule | lien avec l'alerte | `surete.recherche_ouverte` | AS10 |

### 19.9 PPMS : référentiel vivant

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-245 | Créer le **plan de sûreté** de l'année (PPMS unifié, version) | DIR, REF · `surete.ppms.gerer` | modèle éduscol 2024 (§5.1) ; brouillon | `surete.ppms_brouillon` | AS9 |
| ACS-246 | Définir les **zones** (mise à l'abri, confinement, évacuation) sur les lieux du socle 5 | REF | capacité ; consignes par posture | — | AS9 |
| ACS-247 | Rattacher **salles et lieux** aux zones ; voir les **lieux sans zone** | REF | écart signalé | — | AS9 |
| ACS-248 | Désigner les **responsables de zone** et suppléants | DIR, REF | personnels du 6 | `surete.zone_responsable` | AS9 |
| ACS-249 | Définir les **points de rassemblement** et le **point de regroupement des familles** | REF | — | — | AS9 |
| ACS-250 | Constituer la **cellule de crise** (rôles, titulaires, suppléants) | DIR | rôles du modèle ; numéros professionnels | `surete.cellule_modifiee` | AS9 |
| ACS-251 | Tenir l'**annuaire de crise** (secours, mairie, DSDEN, préfecture, transporteurs, réseaux) | DIR, REF · `surete.annuaire.gerer` | organisations du 4 ; données confidentielles, usage crise seulement (§5.1) | — | AS9 |
| ACS-252 | **Mettre à jour et exporter le répertoire de crise** de la rentrée pour la DSDEN | DIR | rappel en septembre ; export PDF ; geste de transmission de la direction (privé : facultatif) | `surete.repertoire_exporte` | AS9 |
| ACS-253 | Gérer les **mallettes** (contenu attendu, emplacement, responsable) | REF | bonne pratique [NV] | — | AS9 |
| ACS-254 | Enregistrer une **vérification** de mallette (date, manques, péremptions) | REF, AGT | rappel périodique (S-1410) | `surete.mallette_verifiee` | AS9 |
| ACS-255 | Voir les **personnes à aide d'évacuation** et leurs aidants (sans diagnostic) | DIR, cellule, responsable de zone (sa zone) | port du 35 ; liste « attention particulière » réservée au chef et à la cellule (§5.1) | — | AS9 |
| ACS-256 | Désigner un **aidant suppléant** quand l'aidant est absent | REF, CPE | alerte le jour même si aucun aidant (CAS-70) | `surete.aidant_manquant` | AS9 |
| ACS-257 | Rédiger les **fiches réflexes** par posture et par rôle | DIR, REF | modèles officiels ; versions | — | AS9 |
| ACS-258 | Déposer le **PPMS signé** et noter sa **communication** (DSDEN, collectivité, maire, au plus tard le 1er juillet) | DIR | socle 7 ; compteur d'échéance | `surete.ppms_communique` | AS9 |
| ACS-259 | **Informer** la communauté éducative et les familles du PPMS à la rentrée | DIR → 8 | modèle ; N3 pour l'envoi | `notifications.diffusion_lancee` | AS9 |
| ACS-260 | Voir le **tableau du référent** : personnels sans zone, lieux sans zone, mallettes à vérifier, aidants manquants, échéances | REF, DIR | — | — | AS9 |
| ACS-261 | Générer chaque matin la **liste papier de comptage** par zone (secours en cas de panne) | SYS | S-1408 ; à placer dans les mallettes ; détruite le soir | `surete.liste_papier_generee` | AS9 |
| ACS-262 | Réviser le PPMS (nouveau risque, locaux modifiés) | DIR | nouvelle version ; ancienne gardée | `surete.ppms_revise` | AS9 |
| ACS-263 | Définir les **moyens d'alerte** de l'établissement (au moins deux, distincts de l'alarme incendie) | REF | rappel de l'exigence (§5.1) ; ScolIA en est un, pas le seul | — | AS9 |
| ACS-264 | Préparer la **partie publique** destinée aux familles (consignes « ne venez pas chercher vos enfants »…) | DIR | jamais plans, zones ni codes (R-AS10) | — | AS9 |

### 19.10 Alertes, comptage, présents

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-265 | **Déclencher une alerte** (posture, périmètre) ★ | DIR, personne habilitée · `surete.alerte.declencher` | deux gestes + passkey ; mobile ; message par le 8 (NTF-87) | `surete.alerte_declenchee` | AS10 |
| ACS-266 | **Signaler un danger** (tout personnel témoin) | tout personnel · `surete.alerte.signaler` | « toute personne témoin… peut déclencher l'alarme » (§5.1) : signal immédiat à la direction et à la cellule, avec lieu ; la direction confirme l'alerte générale | `surete.danger_signale` | AS10 |
| ACS-267 | Recevoir **sa liste de comptage** (personnes attendues dans sa zone ou son groupe, d'après l'instant) | PRF, AED, responsable de zone | précalculée, hors ligne (R-AS11) | — | AS10 |
| ACS-268 | **Pointer** présent / absent / manquant / blessé (sans détail) / accompagné hors zone | PRF, AED | fiche officielle de recensement (§5.1) | `surete.comptage_ligne` | AS10 |
| ACS-269 | Ajouter un **élève en plus** (réfugié) | PRF, AED | retiré de sa liste d'origine comme « compté ailleurs » (CAS-61) | idem | AS10 |
| ACS-270 | Déclarer « **RAS, tout le monde est là** » | PRF, AED | un geste | `surete.comptage_termine` | AS10 |
| ACS-271 | Voir le **tableau de la cellule** : zones comptées, totaux, manquants nominatifs avec dernière localisation, doublons, visiteurs, personnels non joints, accusés | cellule · `surete.alerte.piloter` | — | — | AS10 |
| ACS-272 | Calculer les **présents** (`surete::presents`) avec source et certitude | SYS | à la demande ; jamais stocké en continu | — | AS10 |
| ACS-273 | Inscrire une **décision** dans la main courante de crise | cellule | horodatée, ajout seul | `surete.decision` | AS10 |
| ACS-274 | Envoyer un **point de situation** (NTF-139) | DIR · `notifications.urgence.declencher` | N3 ; brouillon IA possible | `notifications.urgence_suivi` | AS10 |
| ACS-275 | Changer de **posture** en cours d'alerte (confinement → évacuation) | DIR | nouveau message (8) ; listes recalculées par zone d'évacuation | `surete.alerte_posture` | AS10 |
| ACS-276 | **Lever l'alerte** ★ | DIR · `surete.alerte.lever` | message de fin (8) ; levée par les autorités notée si elle vient d'elles | `surete.alerte_levee` | AS10 |
| ACS-277 | **Annuler une alerte envoyée par erreur** ★ | déclencheur, DIR | correctif immédiat (NTF-91) ; compte rendu court obligatoire | `surete.alerte_annulee` | AS10 |
| ACS-278 | Voir les **participants des sorties collectives** dans les présents (hors établissement, responsable) | cellule | port du 42 | — | AS10 |
| ACS-279 | Prévenir les **encadrants de sorties en cours** (information, sans consignes de confinement) | SYS → 8 | — | `notifications.avis_cree` | AS10 |
| ACS-280 | Suspendre les **relances et envois non urgents** pendant l'alerte (silence de crise) | SYS | port `surete::alerte_en_cours` lu par 8, 12, 16 (R-M52) | — | AS10 |
| ACS-281 | Basculer l'écran de remise en **mode crise** (§14.7) | DIR | → ACS-199 | `surete.remise_crise_ouverte` | AS10 |
| ACS-282 | Voir **remis / restant** au point de regroupement | DIR, cellule | — | — | AS10 |
| ACS-283 | Alerte **par site, bâtiment** (NTF-140) | DIR | listes limitées au périmètre | — | AS10 |
| ACS-284 | Alerte **de nuit** à l'internat | surveillant de nuit, DIR | présents de nuit (41) ; zones de l'internat | — | AS10 |
| ACS-285 | Signaler un **blessé** (sans détail médical) ; l'infirmière voit la zone | PRF, AED | « prise en charge nécessaire » ; détails au 34 | `surete.blesse_signale` | AS10 |
| ACS-286 | Saisir **après coup** les comptages et décisions faits sur papier (plan B) | cellule | marqué « saisi après coup » | `surete.comptage_ligne` | AS10 |
| ACS-287 | Voir les **personnels présents** à l'instant (EDT, planning, absences) et les joindre | cellule | port du 6 | — | AS10 |
| ACS-288 | Clore l'alerte et **générer le compte rendu** | DIR | → ACS-296 | `surete.alerte_close` | AS10 |
| ACS-289 | Effacer les **données hors ligne** de l'alerte (24 h après la clôture) | SYS | R-AS11 | — | AS10 |

### 19.11 Exercices, comptes rendus, conformité

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-290 | Proposer le **calendrier des exercices** de l'année (PPMS ×2, incendie ×2 au moins, nuit si internat) | SYS → DIR | §5.1, §5.2 ; échéances | `surete.calendrier_propose` | AS11 |
| ACS-291 | **Planifier** un exercice (type, posture, scénario, date, observateurs) | DIR, REF · `surete.exercices.gerer` | scénario différent du précédent signalé | `surete.exercice_planifie` | AS11 |
| ACS-292 | Cocher les **préalables** (forces de sécurité prévenues, communauté éducative informée, élèves à besoins particuliers préparés, pas d'arme factice) | DIR | blocage du lancement d'un exercice PPMS tant que non cochés (R-AS15) | — | AS11 |
| ACS-293 | **Informer les familles** avant ou après l'exercice (NTF-143) | DIR | — | `notifications.diffusion_lancee` | AS11 |
| ACS-294 | **Lancer** l'exercice (mode exercice) | DIR · `surete.exercices.lancer` | préfixe EXERCICE ; table d'état distincte | `surete.exercice_lance` | AS11 |
| ACS-295 | **Chronométrer** automatiquement (déclenchement, premier et dernier comptage, fin, temps par zone) | SYS | — | — | AS11 |
| ACS-296 | Rédiger le **compte rendu** pré-rempli (exercice ou alerte) | DIR | brouillon IA possible (N1) | `surete.compte_rendu_brouillon` | AS11 |
| ACS-297 | Saisir les **constats d'observateur** (texte court, photo d'un lieu sans personne) | observateur | — | `surete.constat` | AS11 |
| ACS-298 | Recueillir le **retour d'expérience** des élèves, personnels et parents (questionnaire du 18) | DIR | anonyme par défaut | `formulaires.campagne_ouverte` | AS11 |
| ACS-299 | **Signer** le compte rendu (socle 7) | DIR | — | `surete.compte_rendu_signe` | AS11 |
| ACS-300 | Transformer les dysfonctionnements en **plan d'actions** (action, responsable, échéance, état) | DIR, REF | suivi jusqu'à l'exercice suivant | `surete.action_creee` | AS11 |
| ACS-301 | Suivre et clore les **actions** | responsable | rappels | `surete.action_close` | AS11 |
| ACS-302 | Produire l'**extrait pour le registre de sécurité** (date, déroulement, temps d'évacuation) | DIR | R 33, R. 143-44 | `surete.extrait_registre` | AS11 |
| ACS-303 | Noter la **transmission** des retours d'expérience (DSDEN, collectivité, avant le 1er avril) et la présentation au CA | DIR | compteur d'échéance ; privé : facultatif | `surete.rex_transmis` | AS11 |
| ACS-304 | Voir le **compteur de conformité** de l'année (exercices faits, comptes rendus signés, échéances PPMS et répertoire) | DIR, REF | — | — | AS11 |

### 19.12 Familles et élèves

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-305 | Voir **« aujourd'hui, Léa peut sortir à … »** et pourquoi | PAR, ELV, MAJ | `sortie::decider` ; EDT-229 | — | AS3 |
| ACS-306 | Recevoir l'avis de **sortie exceptionnelle effectuée** et de **remise** | PAR (droit), personnes de confiance pour les horaires | socle 8 ; jamais un titulaire sous interdiction de communication | — | AS4 |
| ACS-307 | Recevoir l'avis d'un **départ sans autorisation** | PAR (droit) | U2 ; texte neutre | — | AS5 |
| ACS-308 | Voir l'**historique** des sorties exceptionnelles et remises de son enfant | PAR, MAJ | année en cours | — | AS4 |
| ACS-309 | **Annoncer un retard ou une absence** dans l'appli (renvoi vers le 12, VS-07) | PAR, MAJ | — | (12) | AS1 |
| ACS-310 | Recevoir les **alertes**, points de situation et levée (8) | PAR, personnes de confiance | sauf interdiction de communication | — | AS10 |
| ACS-311 | Lire les **consignes PPMS destinées aux familles** | PAR | partie publique seulement (ACS-264) | — | AS9 |
| ACS-312 | Répondre au **retour d'expérience** d'un exercice | PAR, ELV, personnels | questionnaire du 18 | — | AS11 |
| ACS-313 | Voir **qui peut venir me chercher** (élève) | ELV | liste des personnes autorisées **sans** les interdictions (04 : l'enfant ne voit pas les interdictions) [R, Q-AS9] | — | AS4 |
| ACS-314 | Demander une **carte** de remplacement | ELV, PAR | → ACS-176 | — | AS5 |

### 19.13 Pilotage, conservation, administration

| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-315 | Voir les **indicateurs** : sorties exceptionnelles par mois, refus au portail, remises, visiteurs, recherches et durées, exercices et temps d'évacuation | DIR · `pilotage.lire` | agrégés (47) | — | AS14 |
| ACS-316 | Paramétrer les **réglages signalés** (§29) | DIR | peu de réglages (D-023) | `accueil.reglage_modifie` | AS2 |
| ACS-317 | **Purger** selon les durées (§21.3) | SYS | tâche quotidienne ; rapport | `accueil.purge` | AS14 |
| ACS-318 | Voir le **journal des consultations** sensibles (remise, localisation) | DIR, DPO · `audit.lire` | — | — | AS14 |
| ACS-319 | Répondre à une **demande d'accès RGPD** d'un visiteur (ses passages) | DIR, DPO | export minimal | `rgpd.acces_repondu` (27) | AS14 |
| ACS-320 | Enregistrer un **appareil** d'accueil ou de portail (tablette, téléphone) et le révoquer | ADM · `accueil.appareils` | clé d'appareil pour le hors ligne ; révocation efface au prochain contact | `accueil.appareil_enregistre` | AS5 |
| ACS-321 | Déclarer les **traitements** du socle au registre (registre des visiteurs, contrôle des sorties, PPMS) | DPO, DIR | modèles de fiches (27) | — | AS14 |
| ACS-322 | **Reprendre** les déclarations d'accueil de Next.js de l'année en cours (si décidé, Q-T0-07) | ADM | §26 | `accueil.reprise_faite` | AS1 |
| ACS-323 | Afficher l'**écran par rôle** « ce que vous voyez, et pourquoi pas plus » | tous | comme SAN-331 | — | AS14 |
| ACS-324 | Exporter le **bilan annuel sûreté** (exercices, alertes, actions, conformité) pour le CA ou l'OGEC | DIR | — | `surete.bilan_exporte` | AS14 |
| ACS-325 | Créer les **rôles par défaut** du socle (accueil, AED, référent sûreté) dans le catalogue | ADM | D-017 | — | AS0 / AS2 |
| ACS-326 | Clôturer l'**année** : autorisations closes, nouvelle version de règles à approuver, compteur remis à zéro | SYS | `annee.ouverte` | `sortie.annee_close` | AS2 |
| ACS-327 | Tester la **chaîne d'alerte** sans alerter (envoi aux seuls personnels de la cellule) | DIR | mode test, préfixe TEST | `surete.test_chaine` | AS10 |
| ACS-328 | Retirer les **anciennes tables** 062–065 après reprise vérifiée ★ | ADM | décision de Florian ; migration 2299 ; point de sauvegarde (E8) | — | AS2 + décision |
| ACS-329 | Voir l'**état hors ligne** des appareils (dernière synchronisation, données présentes) | ADM, REF | — | — | AS5 |

---

## 20. Règles transverses (R-AS1 à R-AS20) [R]

| N° | Règle | Lot |
|---|---|---|
| R-AS1 | **Décision calculée côté serveur**, à l'heure du serveur dans le fuseau de l'établissement ; le client n'envoie jamais l'heure d'un contrôle (DR-03) ; une saisie a posteriori est marquée | AS0 |
| R-AS2 | **Le 14 ne copie pas le 4** : personnes autorisées, interdictions, titulaires sont lus par port au moment de l'usage ; seules la fiche de secours du jour et les listes de comptage en alerte sont des copies, chiffrées et éphémères | AS4 |
| R-AS3 | **Interdiction prioritaire**, désaccord bloquant : une interdiction de remise désactive la remise ; une interdiction `demande_titulaire` ou un désaccord connu d'un titulaire exige l'accord d'un membre de la direction (deux gestes, journalisé) ; jamais d'interprétation automatique d'un jugement (C. civ. 372-2, guide DGESCO 2011) | AS4 |
| R-AS4 | **Aucune remise forcée** : le logiciel permet de refuser une remise autorisée, jamais de remettre à une personne non autorisée | AS4 |
| R-AS5 | **ScolIA ne signale ni n'appelle les secours ou les forces de l'ordre** ; il rappelle les numéros et note les gestes de la direction | AS8, AS10 |
| R-AS6 | **Lint de minimisation** sur tout texte libre du 14 (déclaration, demande, main courante, constat) : pas de santé, pas de situation familiale, pas d'opinion ; message « à saisir à l'infirmerie (34) » | AS1 |
| R-AS7 | **Aucune biométrie**, aucune reconnaissance faciale, aucune géolocalisation d'élève (CNIL, §5.4) | tous |
| R-AS8 | **Exceptions journalisées, passages ordinaires non** (sauf option S-1401 annoncée aux familles) | AS5 |
| R-AS9 | **Accès refusé ≠ perte des droits** d'un titulaire sur la scolarité de son enfant | AS6 |
| R-AS10 | **Aucune mesure de sûreté publiée** hors des personnels habilités (plans, zones, codes, annuaire, liste des personnes à attention particulière) | AS9 |
| R-AS11 | **Hors ligne borné** : décisions du jour, fiche de secours, listes de comptage ; chiffrés par une clé d'appareil inscrite à l'inventaire (E11) ; effacés à minuit ou 24 h après la clôture d'alerte ; jamais de coordonnées de famille hors fiche de secours | AS4, AS5, AS10 |
| R-AS12 | **Synchronisation sans écrasement** : chaque fait hors ligne garde son heure réelle et son appareil ; deux remises du même élève ou deux statuts contradictoires d'un comptage sont des **conflits** montrés à un humain | AS4, AS10 |
| R-AS13 | **Faits en ajout seul** : mouvements, remises, comptages, décisions de crise, faits de main courante, étapes de recherche ; correction = fait lié | tous |
| R-AS14 | **Alerte réelle et exercice ne se confondent jamais** : mode stocké, mot affiché, couleur, préfixe des messages ; une alerte ne peut pas devenir exercice ni l'inverse | AS10, AS11 |
| R-AS15 | **Exercice PPMS annoncé** : pas de mode surprise ; lancement bloqué tant que les préalables ne sont pas cochés ; pas d'arme factice (circulaire du 8 juin 2023) | AS11 |
| R-AS16 | **Règles conformes aux repères** de la fiche 34 / circulaire 96-248 : au collège, pas de sortie entre deux cours ni de sortie anticipée hors fin de demi-journée ou de journée, autorisation écrite ; toute règle qui s'en écarte est signalée à l'approbation | AS2 |
| R-AS17 | **Pendant une alerte, toute sortie est refusée** et toute remise suspendue jusqu'à la levée, sauf remise en mode crise | AS10 |
| R-AS18 | **Silence de crise** : 8, 12 et 16 lisent `surete::alerte_en_cours` et suspendent relances et avis non urgents | AS10 |
| R-AS19 | **Localisation unitaire et motivée** : `edt::localiser` d'un élève à l'instant, avec motif, journalisé ; aucune liste « où sont tous les élèves » hors alerte | AS1 |
| R-AS20 | **Pas de nom dans les événements** : les événements du 14 portent des identifiants, jamais de nom, de motif ni de consigne | tous |

---

## 21. Droits et minimisation

### 21.1 Catalogue d'actions proposé (D-017) [R]

Existants conservés [F] : `sorties.gerer`, `sorties.controler`, `absences.declarer`, `remise.lire`, `remise.gerer`, `remise.interdire` (socle 4), `eleves.badge.gerer` (socle 2). Nouveaux :

| Droit | Portée | Modèles de rôle par défaut |
|---|---|---|
| `accueil.rechercher`, `accueil.tableau` | établissement | accueil, vie scolaire, CPE, direction |
| `accueil.localiser` | élève, instant, motif | accueil, vie scolaire, CPE, direction, infirmerie |
| `accueil.visiteurs` | établissement | accueil, secrétariat |
| `accueil.visiteurs.annoncer` | soi | tout personnel |
| `accueil.visiteurs.exporter` | établissement | direction |
| `accueil.acces_temporaire` | établissement | secrétariat, gestionnaire, direction |
| `accueil.acces_refuser` | établissement | chef d'établissement, adjoint (délégation datée) |
| `accueil.main_courante` | établissement | accueil (lecture 7 jours), vie scolaire, CPE, direction |
| `accueil.main_courante.qualifier` | établissement | direction |
| `accueil.appareils` | établissement | administrateur de l'établissement |
| `remise.effectuer` | élève, jour ; ou sortie collective, jour du retour | accueil, vie scolaire, CPE, encadrant de sortie (42) |
| `sorties.controler` | établissement | **+ accueil** (DR-08), AED, CPE, direction |
| `sorties.regles.gerer` | établissement | CPE, direction |
| `sorties.regles.approuver` | établissement | chef d'établissement |
| `sorties.restreindre` | établissement | CPE, direction |
| `sorties.imperieuse` | établissement | direction, CPE (délégation) |
| `surete.posture` | établissement | direction |
| `surete.recherche.ouvrir` | établissement | tout personnel |
| `surete.recherche.mener` | établissement | vie scolaire, CPE, direction |
| `surete.recherche.decider` | établissement | direction |
| `surete.ppms.gerer`, `surete.annuaire.gerer` | établissement | direction, référent sûreté |
| `surete.alerte.signaler` | établissement | **tout personnel** |
| `surete.alerte.declencher`, `surete.alerte.lever` | établissement | chef d'établissement, adjoints, personnes désignées au PPMS |
| `surete.alerte.piloter` | alerte | cellule de crise |
| `surete.alerte.remettre` | alerte | cellule, accueil, vie scolaire |
| `surete.exercices.gerer`, `surete.exercices.lancer` | établissement | direction, référent sûreté |

Le comptage en alerte n'est pas un droit : tout personnel **présent** reçoit la liste de son groupe ou de sa zone ; un professeur ne voit que la sienne.

### 21.2 Ce que voit chacun (projections) [R]

| Information | Accueil | AED | CPE | Direction | Professeur | Infirmerie | Famille | Élève |
|---|---|---|---|---|---|---|---|---|
| Décision de sortie et raison | ✔ | ✔ | ✔ | ✔ | sa séance | — | son enfant | soi |
| Options d'autorisation, historique | — | — | ✔ | ✔ | — | — | son enfant | soi (majeur) |
| Restriction et règle individuelle | « refusée » seulement | idem | ✔ (motif) | ✔ | — | — | motif notifié | « refusée » |
| Sorties exceptionnelles du jour | ✔ | ✔ | ✔ | ✔ | sa séance | — | son enfant | soi |
| Interdictions de remise | consigne, le jour de l'usage | consigne | consigne | ✔ (motif : socle 4) | — | consigne | jamais celles qui les visent | — |
| Personnes autorisées | aujourd'hui | aujourd'hui | ✔ | ✔ | — | aujourd'hui | les siennes | sans interdictions (Q-AS9) |
| Remises | du jour | du jour | ✔ | ✔ | — | ses renvois | son enfant | soi |
| Visiteurs | ✔ | — | — | ✔ | ses visiteurs | — | — | — |
| Accès refusés | consigne | consigne | consigne | ✔ (motif) | — | — | — | — |
| Main courante | ses faits, 7 jours | — | ✔ | ✔ | — | — | — | — |
| Recherche d'élève | celles en cours | celles en cours | ✔ | ✔ | si concerné | « à l'infirmerie : oui/non » | appel reçu | — |
| PPMS : zones, rôles | sa zone | sa zone | sa zone | ✔ | sa zone | sa zone | partie publique | partie publique |
| Annuaire de crise, personnes à attention particulière | — | — | — | chef et cellule | — | — | — | — |
| Comptage en alerte | sa liste | sa liste | sa liste | tout (cellule) | sa liste | blessés de la zone | — | — |

### 21.3 Conservation [R, durées soumises au DPO]

| Donnée | Durée proposée | Fondement |
|---|---|---|
| Visites (registre des visiteurs) | **3 mois** après la visite, puis suppression | repère CNIL (§5.4) |
| Accès temporaires et intervenants | fin de l'accès + 3 mois | idem |
| Accès refusés | levée + 1 an [H] | preuve de la décision |
| Passages ordinaires (si S-1401) | **1 mois** [H] | proportionnalité |
| Refus, sorties exceptionnelles, retours, sorties sans autorisation | fin de l'année scolaire + 1 an [H] | réclamations, responsabilité |
| Remises | fin de l'année scolaire + 1 an [H] | preuve de la remise |
| Autorisations de sortie et règles | durée de scolarité + 1 an ; versions de règles : 5 ans [H] | preuve de l'accord |
| Photos de mots ou coupons papier | 1 mois après la décision | minimisation |
| Main courante | 3 ans [H, Q-AS5] ; faits qualifiés en incident : durée du 13 | — |
| Recherches d'élève manquant | 3 ans [H] | responsabilité |
| PPMS, comptes rendus, plans d'actions | 5 ans puis archives [H] | contrôle, retour d'expérience |
| Comptages nominatifs d'alerte réelle | 1 an [H], puis seuls les totaux | enquête éventuelle |
| Comptages nominatifs d'exercice | **1 mois**, puis seuls les totaux et les temps | minimisation |
| Données hors ligne des appareils | minuit / clôture + 24 h | R-AS11 |
| Journal des consultations (remise, localisation) | 1 an (socle 1) | traçabilité |

### 21.4 Pièges à tester
- L'accueil ne voit jamais un motif d'interdiction, de restriction ou d'accès refusé.
- Un professeur ne voit que la sortie de **sa** séance, pas l'historique ni les autres élèves.
- Un titulaire visé par une interdiction de communication ne reçoit ni avis de remise, ni alerte, ni appel de recherche.
- Un parent ne voit que ses enfants ; un parent dont le lien est suspendu ne demande plus de sortie et son autorisation passée est revue.
- Un mineur ne dépose pas seul une sortie exceptionnelle.
- La liste des personnes à attention particulière n'est lisible que du chef d'établissement et de la cellule.
- Un AED hors de son établissement (deux établissements du même organisme) ne voit rien (RLS).
- Un élève ne lit pas les interdictions qui le concernent.
- L'IA ne reçoit aucun motif, aucune donnée de santé, aucune coordonnée, aucun comptage nominatif.
- Un contrôle envoyé avec une heure par le client est refusé (ou marqué a posteriori avec le droit `sorties.gerer`).
- Une QR capturée et rejouée 60 s après est refusée.

### 21.5 Information des personnes (RGPD)
Mentions modèles : registre des visiteurs (affiche à l'accueil, QR vers la mention complète : finalité sûreté et sécurité des personnes, base légale mission d'intérêt public ou intérêt légitime selon le statut [NV, DPO], destinataires, 3 mois, droits) ; contrôle des sorties (règlement intérieur, fiche de rentrée) ; PPMS (information des personnels). Fiches de traitement préparées pour le registre (socle 27).

---

## 22. Événements (outbox, dans la transaction du fait) [R]

**Publiés** (identifiants seulement, R-AS20) :
- accueil : `accueil.declaration_transmise`, `_a_confirmer`, `_confirmee`, `_annulee` ; `accueil.visite_annoncee`, `_arrivee`, `_sortie` ; `accueil.visiteurs_non_sortis` ; `accueil.acces_temporaire_cree` ; `accueil.acces_refuse_pose`, `_leve` ; `accueil.fait_cree`, `_suite`, `_corrige`, `_qualifie` ; `accueil.remise_preparee`, `accueil.remise_refusee`, `accueil.remise_interdite_tentee` ; `accueil.fiche_secours_generee` ; `accueil.poste_ouvert`, `accueil.appareil_enregistre` ; `accueil.purge` ;
- sorties : `sortie.regles_brouillon`, `sortie.regles_approuvees` ; `sortie.autorisation_choisie`, `_validee`, `_refusee`, `_modifiee`, `_a_revoir`, `_close` ; `sortie.desaccord` ; `sortie.regle_individuelle` ; `sortie.exception_demandee`, `_preparee`, `_decidee`, `_annulee` ; `sortie.imperieuse` ; `sortie.restriction_posee`, `_levee` ; `sortie.effectuee`, `sortie.retour`, `sortie.retour_manquant`, `sortie.refusee`, `sortie.sans_autorisation`, `sortie.controle` ;
- sûreté : `surete.remise_effectuee`, `surete.remise_corrigee` ; `surete.sortie_anticipee_calculee` ; `surete.recherche_ouverte`, `_etape`, `_escaladee`, `_signalee`, `_close` ; `surete.ppms_brouillon`, `_communique`, `_revise` ; `surete.zone_responsable`, `surete.cellule_modifiee`, `surete.mallette_verifiee`, `surete.aidant_manquant` ; `surete.danger_signale`, `surete.alerte_declenchee`, `_posture`, `_levee`, `_annulee`, `_close` ; `surete.comptage_ligne`, `surete.comptage_termine`, `surete.decision`, `surete.blesse_signale` ; `surete.exercice_planifie`, `_lance` ; `surete.compte_rendu_signe`, `surete.action_creee`, `_close`, `surete.rex_transmis` ; `surete.posture_changee`.

**Consommés** : ceux du §6.4, plus `notifications.accuses` (8), `formulaires.reponse` (18, options de sortie de la fiche de rentrée), `documents.signe` (7).

**File d'effets de bord** : aucune nouvelle file (les envois passent par le socle 8, déjà déclaré « à neutraliser à la restauration », E9) ; **les alertes sont neutralisées à la restauration** (une alerte `en_cours` restaurée passe `close` sans envoi) [R].

## 23. Notifications (natures déclarées au socle 8) [R]

| Nature | Urgence | Classe | Destinataires | Remarque |
|---|---|---|---|---|
| `surete.alerte` (existe, 8) | U3 | C0 | familles sauf interdictions, personnels | modèles à froid ; vibration seule en attentat-intrusion |
| `notifications.urgence_suivi` (existe) | U3 | C0 | idem | points de situation |
| `surete.danger_signale` | U3 | C0 | direction, cellule | interne |
| `surete.recherche` | U3 | C1 | vie scolaire, CPE, direction | interne |
| `surete.remise` | U1 | C1 | titulaires avec droit (hors interdiction de communication) | « remise à Mme X à 15 h 12 » |
| `sortie.effectuee` | U1 | C1 | idem | sortie exceptionnelle |
| `sortie.sans_autorisation` | U2 | C1 | idem | texte neutre |
| `edt.sortie_anticipee` (existe, 8) | U2 | C1 | titulaires + personnes de confiance | seulement les autorisés |
| `sortie.exception_decidee` | U1 | C1 | demandeur | — |
| `sortie.autorisation_a_choisir` | U0 | C1 | titulaires | relance de rentrée |
| `sortie.restriction_posee` | U1 | C1 | titulaires | motif court |
| `accueil.visite_arrivee` | U1 | C1 | hôte | interne |
| `accueil.remise_interdite_tentee` | U3 | C1 | direction | interne, sans motif |
| `surete.exercice_information` (= `notifications.exercice_information`, NTF-143) | U0 | C0 | familles | jamais confondu avec l'alerte |
| `surete.echeance` | U0 | C1 | direction, référent | 1er juillet, 15 septembre, 1er avril, exercices |

## 24. Écrans (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`)

1. **Poste d'accueil** (tablette, web) : barre de recherche unique en haut ; tuiles Bento « attendus aujourd'hui » (visiteurs, sorties, remises, cars), « déclarations du jour », « présents maintenant (visiteurs) » ; fiche élève minimale : photo, classe, **où il est maintenant** (sur geste motivé), **décision de sortie** en un mot, bouton **Remise**.
2. **Écran de remise** : bandeau rouge des interdictions (consignes), liste des autorisés aujourd'hui (photo, pièce exigée), trois chemins pour une personne non listée, gros bouton « Remis à … ».
3. **Mode portail** (téléphone, tablette) : caméra plein écran, résultat en un mot et une couleur, dernière décision en mémoire, bandeau « hors ligne » quand il y a lieu.
4. **Vie scolaire, sorties** : file des demandes à décider (tableau dense, décisions en masse), autorisations à valider, restrictions, refus du jour, recherches en cours.
5. **Règles de sortie** : éditeur de version (règles en lignes, simulation « 37 élèves changent de droit »), textes d'options, approbation.
6. **Famille / élève** : carte « Aujourd'hui tu peux sortir à 15 h — pourquoi ? », demande de sortie en deux gestes, historique, QR de l'élève.
7. **Registre des visiteurs** : arrivée en dix secondes (nom, qualité, hôte, motif, pièce), présents, attendus, badges.
8. **Main courante** : chronologie filtrable ; saisie courte ; qualification par la direction.
9. **Recherche d'élève** : liste de vérifications cochables, chronomètre, bouton « appeler les responsables », escalade.
10. **PPMS** : plan de l'établissement par lieux (arbre du 5), zones colorées, responsables, mallettes, tableau du référent (écarts).
11. **Alerte, côté adulte** (mobile) : un seul écran : ma liste, cocher, « élève en plus », « manquant », « blessé », « RAS ».
12. **Alerte, côté cellule** : carte des zones (comptées / non comptées), totaux, manquants avec dernière localisation, accusés, main courante de crise, bouton « point de situation », « lever ».
13. **Exercices** : calendrier et compteur de conformité, préalables, chronométrage, compte rendu, plan d'actions.

Accessibilité : contrastes AA, rouge / vert doublés d'un mot et d'une icône (daltonisme), commande au pouce sur mobile, mode sombre pour un confinement dans le noir (luminosité réduite, son coupé).

---

## 25. Modèle de données cible [R]

Règles communes (E1–E14, D-036) : `id UUID` généré par l'application ; `etablissement_id UUID NOT NULL` sur **toutes** les tables, filles comprises ; clé unique `(etablissement_id, id)` ; **FK composites** `(etablissement_id, x_id)` ; `cree_le TIMESTAMPTZ NOT NULL DEFAULT now()`, `modifie_le` sur les tables modifiables ; pas de `DELETE` sur les faits (déclencheur d'immuabilité sur les tables en ajout seul) ; codes en `TEXT` avec `CHECK` ; **aucune colonne JSON/JSONB** ; textes scellés chiffrés au format `enc:v2:<id_cle>:` (E11) ; RLS `ent_rls_poser` (E13) ; chaque table classée au registre `registre_donnees` (E12) ; fichiers par `ent_objet` / socle 7 (E5). Préfixes : `ent_regime_sortie_*` (règles et autorisations), `ent_accueil_*` (accueil, mouvements, remises, visites, main courante), `ent_surete_*` (recherche, PPMS, alertes, exercices) — jamais `ent_sortie_*` (42, DR-10).

### 25.1 Règles et autorisations (2283–2284)
```
ent_regime_sortie_version     id, etablissement_id, annee_id → annee (5), site_id NULL → site (5),
                              etat CHECK ('brouillon','approuvee','remplacee'), effet_du DATE NULL,
                              modele CHECK ('college','lycee','autre'), source_document_id NULL → document (7),
                              reference_deliberation TEXT NULL, approuvee_par UUID NULL, approuvee_le TIMESTAMPTZ NULL,
                              cree_le, modifie_le
ent_regime_sortie_option      id, etablissement_id, version_id, code TEXT, ordre SMALLINT, cree_le
ent_regime_sortie_option_texte id, etablissement_id, option_id, langue TEXT, texte TEXT, empreinte TEXT (SHA-256), cree_le
ent_regime_sortie_regle       id, etablissement_id, version_id, ordre SMALLINT,
                              creneau CHECK ('entre_cours','pause_meridienne','fin_journee','cours_annule',
                                             'permanence_premiere_heure','permanence_derniere_heure',
                                             'journee_sans_cours','trajet_inter_sites'),
                              effet CHECK ('libre','sur_option_famille','interdit'), option_id NULL,
                              heure_min TIME NULL, heure_max TIME NULL, age_min SMALLINT NULL,
                              majeur_seulement BOOLEAN NOT NULL DEFAULT false,
                              ecart_modele BOOLEAN NOT NULL DEFAULT false, ecart_justification TEXT NULL, cree_le
ent_regime_sortie_regle_niveau id, etablissement_id, regle_id, niveau_id → niveau (5)     -- vide = tous niveaux
ent_regime_sortie_regle_regime id, etablissement_id, regle_id, regime_code TEXT → ent_param_regime (2)
ent_regime_sortie_autorisation id, etablissement_id, eleve_id → eleve (2), version_id,
                              etat CHECK ('choisie','validee','refusee','a_revoir','close'),
                              du DATE NOT NULL, au DATE NULL, remplace_id NULL (auto-référence),
                              signataire_personne_id NULL → personne (4), signataire_user_id NULL,
                              signataire_qualite CHECK ('titulaire','eleve_majeur','service_gardien','saisie_personnel'),
                              canal CHECK ('appli','fiche_rentree','papier','guichet'), piece_document_id NULL,
                              valide_par NULL, valide_le NULL, refus_motif_code NULL, cree_le
ent_regime_sortie_autorisation_option id, etablissement_id, autorisation_id, option_id, accordee BOOLEAN NOT NULL
ent_regime_sortie_desaccord   id, etablissement_id, eleve_id, personne_id → personne (4), declare_le, declare_par,
                              leve_le NULL, leve_par NULL
ent_regime_sortie_regle_individuelle id, etablissement_id, eleve_id,
                              effet CHECK ('remise_obligatoire','sortie_interdite'), creneau NULL (même CHECK),
                              du, au NULL, motif_scelle TEXT NULL, pose_par, pose_le, leve_par NULL, leve_le NULL
```
`ent_regime_sortie_autorisation` est en **versions** (une nouvelle ligne par changement, `remplace_id`), jamais modifiée sauf `etat` / validation (journalisés en `ent_journal_modif`).

### 25.2 Sorties exceptionnelles et restrictions (2286)
```
ent_regime_sortie_exception   id, etablissement_id, eleve_id, jour DATE, heure_sortie TIME, heure_retour TIME NULL,
                              motif_code CHECK ('medical','administratif','familial','convocation','autre'),
                              commentaire TEXT NULL (≤ 200, lint),
                              demandeur_qualite CHECK ('titulaire','eleve_majeur','eleve_mineur_prepare','personnel'),
                              demandeur_personne_id NULL, demandeur_user_id NULL, canal CHECK ('appli','telephone','mot_papier','guichet'),
                              mot_document_id NULL, remise_requise BOOLEAN, remise_personne_id NULL → personne (4),
                              etat CHECK ('preparee','demandee','acceptee','refusee','annulee','caduque'),
                              decide_par NULL, decide_le NULL, decision_motif_code NULL, automatique BOOLEAN NOT NULL DEFAULT false,
                              imperieuse BOOLEAN NOT NULL DEFAULT false, representants_prevenus CHECK ('oui','tentative','non') NULL,
                              cree_le, modifie_le
ent_regime_sortie_restriction id, etablissement_id, eleve_id, creneau NULL, du, au,
                              origine CHECK ('etablissement','demande_titulaire','proposition_discipline'),
                              motif_code, motif_texte TEXT NULL (visible des titulaires), pose_par, pose_le,
                              confirmee_le NULL, leve_par NULL, leve_le NULL, leve_motif NULL
```

### 25.3 Mouvements et remises (2287) — ajout seul
```
ent_accueil_mouvement         id, etablissement_id, eleve_id, instant TIMESTAMPTZ NOT NULL (serveur),
                              nature CHECK ('sortie_exception','sortie_anticipee','sortie_ordinaire','retour','refus',
                                            'sans_autorisation','remise'),
                              decision CHECK ('autorisee','refusee','a_verifier') NULL, raison_code TEXT NULL,
                              source CHECK ('nom','carte','qr','connecteur','hors_ligne','a_posteriori'),
                              poste_id NULL → ent_accueil_poste, auteur_user_id NOT NULL,
                              exception_id NULL, remise_id NULL, instant_declare TIMESTAMPTZ NULL (hors ligne, a posteriori),
                              corrige_id NULL, cree_le
ent_accueil_remise_preparee   id, etablissement_id, eleve_id, contexte CHECK ('exception','sante','car','internat','discipline','autre'),
                              contexte_ref UUID NULL, heure_prevue TIMESTAMPTZ, lieu_libelle TEXT NULL,
                              etat CHECK ('attendue','remise','annulee'), cree_par, cree_le, modifie_le
ent_accueil_remise            id, etablissement_id, eleve_id, preparee_id NULL, instant TIMESTAMPTZ (serveur),
                              personne_id NULL → personne (4), organisme_libelle TEXT NULL, chauffeur_libelle TEXT NULL,
                              qualite CHECK ('titulaire','autorise_permanent','autorise_jour','autorise_ponctuel',
                                             'accord_distance','service_gardien','transport_mandate'),
                              piece_type CHECK ('carte_identite','passeport','titre_sejour','permis','carte_pro','reconnu','aucune'),
                              identite_confirmee_par NULL, accord_direction_par NULL,
                              mode CHECK ('normal','crise','hors_ligne'), alerte_id NULL,
                              auteur_user_id, instant_declare NULL, corrige_id NULL, correction_motif NULL, cree_le
ent_accueil_remise_refus      id, etablissement_id, eleve_id, instant, personne_id NULL, personne_libelle TEXT NULL,
                              raison CHECK ('interdiction','non_autorisee','comportement','identite','eleve_refuse','autre'),
                              fait_id NULL → ent_accueil_fait, auteur_user_id, cree_le
```

### 25.4 Appareils (2288)
```
ent_accueil_poste             id, etablissement_id, libelle, type CHECK ('tablette_accueil','telephone','poste_fixe'),
                              cle_publique TEXT, id_cle TEXT (inventaire des clés, E11), enregistre_par, enregistre_le,
                              derniere_synchro TIMESTAMPTZ NULL, revoque_le NULL, revoque_par NULL
ent_accueil_qr_cle            id, etablissement_id, id_cle TEXT, active_du, active_au NULL, cree_le   -- clé de signature des QR (secret hors base)
```

### 25.5 Visiteurs et accès (2289)
```
ent_accueil_badge             id, etablissement_id, numero TEXT, etat CHECK ('disponible','remis','perdu','retire'), cree_le, modifie_le
ent_accueil_visite_attendue   id, etablissement_id, nom TEXT, prenom TEXT NULL, personne_id NULL, qualite,
                              hote_agent_id NULL → agent (6), hote_service TEXT NULL, motif_code, attendu_le TIMESTAMPTZ,
                              source CHECK ('annonce','rdv','intervenant','evenement'), source_ref UUID NULL,
                              annonce_par, etat CHECK ('attendue','arrivee','annulee','absente'), cree_le, modifie_le
ent_accueil_acces_temporaire  id, etablissement_id, organisme_libelle, objet TEXT, referent_agent_id, du DATE, au DATE NOT NULL,
                              cree_par, cree_le, clos_le NULL
ent_accueil_acces_temporaire_intervenant id, etablissement_id, acces_id, nom, prenom, cree_le, retire_le NULL
ent_accueil_visite            id, etablissement_id, nom TEXT NULL, prenom TEXT NULL, personne_id NULL, agent_id NULL,
                              qualite CHECK ('parent','personne_liee','intervenant','prestataire','livreur','autorite',
                                             'candidat','ancien_eleve','presse','autre'),
                              organisme_libelle NULL, hote_agent_id NULL, hote_service NULL,
                              motif_code CHECK ('rendez_vous','reunion','livraison','travaux','inscription','evenement','autre'),
                              piece_presentee CHECK ('oui','non','refusee'), piece_type NULL (même CHECK que la remise),
                              badge_id NULL, attendue_id NULL, intervenant_id NULL,
                              arrivee_le TIMESTAMPTZ, sortie_le NULL, sortie_mode CHECK ('normale','sans_badge','cloture_soir') NULL,
                              auteur_user_id, cree_le, modifie_le
                              -- CHECK : (nom IS NOT NULL) OR (personne_id IS NOT NULL) OR (agent_id IS NOT NULL)
ent_accueil_acces_refuse      id, etablissement_id, personne_id NULL, nom TEXT NULL, prenom TEXT NULL,
                              fondement CHECK ('r421_12_urgence','decision_chef_etablissement','decision_organisme','mesure_judiciaire'),
                              consigne TEXT NOT NULL (≤ 300), motif_scelle TEXT NULL (chiffré), du, au NULL,
                              decide_par, decide_le, leve_par NULL, leve_le NULL, leve_motif NULL
ent_surete_posture            id, etablissement_id, niveau CHECK ('vigilance','vigilance_renforcee','alerte_attentat'),
                              du TIMESTAMPTZ, saisi_par, cree_le
```

### 25.6 Main courante (2290) — ajout seul
```
ent_accueil_fait              id, etablissement_id, survenu_le TIMESTAMPTZ, lieu_id NULL → lieu (5), lieu_libelle NULL,
                              categorie CHECK ('intrusion','personne_inconnue_responsable','refus_remise','acces_refuse_presente',
                                               'appel_suspect','objet_abandonne','attroupement','degradation','malaise_visiteur',
                                               'sortie_sans_autorisation','refus_controle_sac','autre'),
                              description TEXT (≤ 1000, lint), auteur_user_id, source_socle SMALLINT NULL, alerte_id NULL, cree_le
ent_accueil_fait_personne     id, etablissement_id, fait_id, eleve_id NULL, personne_id NULL, agent_id NULL, visite_id NULL,
                              role CHECK ('concerne','temoin','auteur_presume','victime')
ent_accueil_fait_suite        id, etablissement_id, fait_id, nature CHECK ('suite','correction','qualification'),
                              qualification CHECK ('trace','incident_propose','ip_proposee','faits_etablissement') NULL,
                              gravite SMALLINT NULL CHECK (gravite BETWEEN 1 AND 3), texte TEXT NULL, auteur_user_id, cree_le
```

### 25.7 Recherche d'élève manquant (2291) — ajout seul pour les étapes
```
ent_surete_recherche          id, etablissement_id, eleve_id, ouverte_le, ouverte_par,
                              origine CHECK ('appel','portail','infirmerie','retour_manquant','alerte','car','autre'),
                              alerte_id NULL, etat CHECK ('ouverte','escaladee','close'),
                              escaladee_le NULL, signalement_decide_le NULL, signalement_decide_par NULL,
                              close_le NULL, close_par NULL, issue CHECK ('retrouve','remis','signale','erreur') NULL,
                              lieu_retrouve TEXT NULL, compte_rendu TEXT NULL (lint), cree_le, modifie_le
ent_surete_recherche_etape    id, etablissement_id, recherche_id, code CHECK ('derniere_presence','edt','infirmerie','sortie_collective',
                              'remise','sortie_exception','lieux_habituels','appel_titulaire','escalade','autre'),
                              resultat CHECK ('positif','negatif','sans_reponse'), note TEXT NULL, auteur_user_id, fait_le
```

### 25.8 PPMS (2292)
```
ent_surete_plan               id, etablissement_id, annee_id, version SMALLINT, etat CHECK ('brouillon','valide','remplace'),
                              document_id NULL → document (7), valide_par NULL, valide_le NULL, cree_le, modifie_le
ent_surete_plan_communication id, etablissement_id, plan_id, destinataire CHECK ('dsden','collectivite','mairie','reseau','autre'),
                              objet CHECK ('ppms','repertoire_crise','rex'), communique_le DATE, par, cree_le
ent_surete_zone               id, etablissement_id, plan_id, code, libelle,
                              nature CHECK ('mise_abri','confinement','evacuation','mixte'), capacite INTEGER NULL, cree_le, modifie_le
ent_surete_zone_lieu          id, etablissement_id, zone_id, lieu_id → lieu (5)
ent_surete_zone_consigne      id, etablissement_id, zone_id,
                              posture CHECK ('evacuation','mise_abri_simple','mise_abri_amelioree','echapper_cacher'), texte TEXT
ent_surete_zone_responsable   id, etablissement_id, zone_id, agent_id → agent (6), rang CHECK ('titulaire','suppleant'), du, au NULL
ent_surete_point              id, etablissement_id, plan_id, nature CHECK ('rassemblement','regroupement_familles'), libelle, lieu_id NULL
ent_surete_cellule_role       id, etablissement_id, plan_id, role CHECK ('chef','suppleant_chef','liaison_secours','liaison_familles',
                              'listes','communication','logistique','autre'), agent_id, rang SMALLINT
ent_surete_contact_crise      id, etablissement_id, plan_id, organisme_id → organisme (4),
                              role CHECK ('secours','police_gendarmerie','mairie','dsden','prefecture','transporteur','energie','reseau','autre'), ordre
ent_surete_moyen_alerte       id, etablissement_id, plan_id, libelle, distinct_incendie BOOLEAN, scolia BOOLEAN
ent_surete_fiche_reflexe      id, etablissement_id, plan_id, posture (même CHECK), role TEXT, version SMALLINT, document_id NULL, texte TEXT NULL
ent_surete_mallette           id, etablissement_id, plan_id, zone_id NULL, emplacement, responsable_agent_id NULL, cree_le, modifie_le
ent_surete_mallette_element   id, etablissement_id, mallette_id, libelle, quantite SMALLINT, peremption DATE NULL
ent_surete_mallette_verification id, etablissement_id, mallette_id, verifiee_le, par, conforme BOOLEAN, note TEXT NULL
ent_surete_aidant             id, etablissement_id, eleve_id, aidant_agent_id, rang CHECK ('titulaire','suppleant'), du, au NULL
                              -- le besoin (« aide à l'évacuation ») est lu au socle 35 ; le 14 ne stocke que l'aidant désigné
```

### 25.9 Alertes et comptages (2293)
```
ent_surete_alerte             id, etablissement_id, mode CHECK ('reelle','exercice','test'), exercice_id NULL,
                              posture CHECK (comme ci-dessus), perimetre CHECK ('etablissement','site','batiment'), perimetre_lieu_id NULL,
                              etat CHECK ('signalee','declenchee','levee','annulee_erreur','close'),
                              signalee_par NULL, signalee_le NULL, declenchee_par NULL, declenchee_le NULL,
                              levee_par NULL, levee_le NULL, levee_par_autorites BOOLEAN NULL,
                              ntf_urgence_id NULL (socle 8), close_le NULL, cree_le, modifie_le
                              -- CHECK : mode immuable (déclencheur), R-AS14
ent_surete_alerte_posture     id, etablissement_id, alerte_id, posture, depuis TIMESTAMPTZ, decide_par        -- ajout seul
ent_surete_comptage           id, etablissement_id, alerte_id, zone_id NULL, responsable_user_id, seance_id NULL, groupe_libelle,
                              attendus INTEGER, termine_le NULL, ras BOOLEAN NOT NULL DEFAULT false, cree_le
ent_surete_comptage_ligne     id, etablissement_id, alerte_id, comptage_id, eleve_id NULL, agent_id NULL, visite_id NULL,
                              statut CHECK ('present','absent','manquant','blesse','hors_zone','en_plus','compte_ailleurs'),
                              pointe_le, pointe_par, appareil_id NULL, hors_ligne BOOLEAN, saisi_apres_coup BOOLEAN, cree_le   -- ajout seul
ent_surete_decision           id, etablissement_id, alerte_id, decidee_le, auteur_user_id, texte TEXT (lint), cree_le          -- ajout seul
ent_surete_chrono             id, etablissement_id, alerte_id, zone_id NULL,
                              jalon CHECK ('declenchement','premier_comptage','zone_comptee','dernier_comptage','levee'), le    -- ajout seul
```
Les **présents** ne sont pas stockés : ils sont calculés (§14.2) ; seules les **listes de comptage** reçues en alerte sont figées (`ent_surete_comptage`, `attendus`).

### 25.10 Exercices et comptes rendus (2294)
```
ent_surete_exercice           id, etablissement_id, plan_id NULL, nature CHECK ('ppms','incendie','incendie_nuit'),
                              posture NULL, scenario TEXT, prevu_le TIMESTAMPTZ, etat CHECK ('planifie','pret','realise','annule'),
                              cree_par, cree_le, modifie_le
ent_surete_exercice_prealable id, etablissement_id, exercice_id,
                              code CHECK ('forces_securite_prevenues','communaute_informee','familles_informees',
                                          'eleves_besoins_prepares','sans_arme_factice','observateurs_designes'),
                              coche_par, coche_le
ent_surete_observateur        id, etablissement_id, exercice_id, agent_id NULL, nom_externe TEXT NULL
ent_surete_constat            id, etablissement_id, alerte_id, auteur_user_id NULL, observateur_id NULL, texte TEXT (lint),
                              objet_id NULL → ent_objet (photo sans personne), cree_le                                          -- ajout seul
ent_surete_compte_rendu       id, etablissement_id, alerte_id, etat CHECK ('brouillon','signe'), document_id NULL,
                              signe_par NULL, signe_le NULL, cree_le, modifie_le
ent_surete_action             id, etablissement_id, compte_rendu_id, libelle, responsable_agent_id NULL, echeance DATE NULL,
                              etat CHECK ('ouverte','faite','abandonnee'), close_le NULL, cree_le, modifie_le
```

### 25.11 Lot 0 (2280–2281) et socle 12 (2282)
- **2280** `ent_sortie_autorisation_hist` : `id`, `etablissement_id`, `eleve_id`, `matin`, `midi`, `soir`, `interdit_etablissement`, `modifie_par`, `modifie_le`, `cree_le` ; déclencheur `AFTER INSERT OR UPDATE` sur `ent_sortie_autorisation` ; reprise de la ligne actuelle ; ajout seul ; RLS ; registre.
- **2281** `ent_sortie_controle` : `controle_le TIMESTAMPTZ NOT NULL DEFAULT now()`, `saisie_a_posteriori BOOLEAN NOT NULL DEFAULT false`, `poste_libelle TEXT NULL` ; déclencheur d'immuabilité.
- **2282** `ent_absence_declaration` (**table du socle 12**, migration écrite dans la plage du 14 avec l'accord du propriétaire, comme le 42 l'a fait) : `appelant_personne_id UUID NULL` (FK composite vers `ent_personne`), `a_confirmer BOOLEAN NOT NULL DEFAULT false`, `confirmee_par UUID NULL`, `confirmee_le TIMESTAMPTZ NULL`, `commentaire_lint_ok BOOLEAN` ; FK composites ajoutées sur `declarant_user_id` et `piece_document_id` (DR-13 partiel).

### 25.12 Exceptions D-036
**Aucune.** Le journal métier utilise `ent_journal_modif` (instantanés, exception déjà permise) ; les charges utiles de l'outbox sont transitoires (exception permise, sans nom, R-AS20).

---

## 26. Reprise des données

1. **Next.js accueil** (`vs_absence_eleve` source `accueil`, `absence` source `accueil`) : **perdable** par défaut (Charlemagne est officiel pour l'assiduité pendant le doublon, D-034 ; T0 Q-T0-07). Si Florian décide de reprendre les déclarations de l'année en cours : import en déclarations « reprises » du socle 12 (élève rapproché par `eleve_id`, puis INE, puis file humaine), auteur « reprise Next.js », jamais d'appelant inventé (ACS-322) [R].
2. **Bascule** : une seule source de saisie par jour ; le jour J, l'accueil saisit dans ScolIA ; Next.js accueil est arrêté par Florian (D-035 point 3) ; pendant une semaine, le tableau du jour ScolIA affiche aussi les déclarations Next.js de la veille en lecture (import quotidien) [R, si Florian fournit l'export].
3. **Rust 062–065** : reprise en AS2 (ACS-137) : `ent_sortie_regle_niveau` et `_regime` → première version de règles (créneaux `matin` → `fin_journee` de la demi-journée du matin, `midi` → `pause_meridienne`, `soir` → `fin_journee`) marquée « reprise, à relire » ; `ent_sortie_autorisation` → autorisation validée, signataire `saisie_personnel`, avec la date de 2280 ; `ent_sortie_demande` en cours → sorties exceptionnelles ; `ent_sortie_controle` → mouvements (lecture seule, conservation §21.3). Rapport de reprise ; anciennes tables gardées en lecture jusqu'à 2299.
4. **Aucune donnée de PPMS** n'existe dans Next.js ni dans Rust : saisie neuve (avec l'aide IA du §18.1), à partir du PPMS papier de l'établissement fourni par Florian [à fournir].

## 27. Lots de code (équipe Sonnet) et migrations

### 27.1 Tableau des lots

| Lot | Contenu | Actions | Migrations | Dépend de | Recette |
|---|---|---|---|---|---|
| **AS0** | **Correctifs urgents** DR-01 à DR-09 | ACS-158 (partiel), 165, 174, 325 | **2280, 2281** | — | contrôle d'un parent au lien suspendu refusé ; heure locale ; RLS strict |
| **AS1** | **Parité Next.js accueil** : poste d'accueil, déclarations (appelant, à confirmer), garde-fou en sortie, tableau du jour, raccourci 15, localisation motivée, assistant | ACS-100 à 116, 309, 322 | **2282** | socle 12 (commande), 42 (port) | les 10 NXA rejouées ; VA-01 à VA-07 corrigés |
| **AS2** | **Règles et autorisations de sortie** v2 : versions, options, fiche de rentrée, validation, désaccord, reprise 062–065, écrans Leptos | ACS-120 à 144, 316, 326 | **2283, 2284, 2285** | 4 (fiche de rentrée), 5, 18 (si prêt, sinon formulaire propre) | modèle collège et lycée ; CAS-01, 02, 09 |
| **AS3** | **Sorties exceptionnelles, restrictions, décision du jour, sortie anticipée** | ACS-117, 145 à 164, 305 | **2286** | 10, 13, 15, 8 | CAS-03 à 08, 10, 14, 15, 17 |
| **AS4** | **Remise** : écran, ports du 4, accord à distance, consigne téléphonique, inconnu, sortie santé, retour de car, fiche de secours, avis | ACS-118, 119, 180 à 198, 306, 308, 313 | **2287** | 4 (A5 : `ent_eleve_remise`), 34, 42, 8 | CAS-20 à 31 |
| **AS5** | **Contrôle aux portes** : nom, carte, QR dynamique, hors ligne, appareils | ACS-165 à 179, 307, 314, 320, 329 | **2288** | 2 (badge), mobile (11) | QR rejoué refusé ; hors ligne 2 h |
| **AS6** | **Visiteurs, accès temporaires, accès refusés, posture** | ACS-200 à 224 | **2289** | 6, 19 | CAS-40 à 48 |
| **AS7** | **Main courante** | ACS-225 à 231, 233, 234 | **2290** | 13, 36 (propositions) | CAS-49 ; lint |
| **AS8** | **Élève manquant** | ACS-235 à 243 | **2291** | 10, 12, 34, 42 | parcours chronométré |
| **AS9** | **PPMS** référentiel vivant | ACS-245 à 264, 311 | **2292** | 5, 6, 4, 35, 7 | modèle éduscol rempli pour le lycée pilote [données à fournir] |
| **AS10** | **Alertes, comptage, présents, cellule, remise en crise** | ACS-199, 232, 244, 265 à 289, 310, 327 | **2293** | 8 (urgence), 10, 12, 6, 41, 42, 11 | exercice simulé 900 élèves, 3 bâtiments, réseau coupé à mi-parcours |
| **AS11** | **Exercices, comptes rendus, retours d'expérience, plan d'actions, registre** | ACS-290 à 304, 312 | **2294** | 7, 18 | calendrier et compteur ; échéances |
| **AS12** | Connecteurs (lecteurs d'accès existants) | ACS-178 | 2295 (réservée) | 9 | — |
| **AS13** | IA (§18) | capacités du §18.3 | — | cœur IA | jeu d'évaluation |
| **AS14** | Pilotage, conservation, RGPD, bilan | ACS-315, 317 à 319, 321, 323, 324 | — | 27, 47 | purge datée |

**Parcours minimal pour la rentrée 2027** (D-035 : le 14 est un module d'**usage réel**, Next.js accueil remplacé) : **AS0, AS1, AS2, AS3, AS4, AS5** ; **AS9 au printemps 2027** (le PPMS unifié doit être adopté pour 2027-2028, §5.1) puis **AS10, AS11 avant le premier exercice de septembre-octobre 2027**. AS6, AS7, AS8 dans l'épaississement (février – août 2027). Démo de décembre 2026 : AS0 et la partie « déclarations » d'AS1 suffisent.

### 27.2 Migrations (plage imposée **2280–2299**, registre `MIGRATIONS.md`)

| N° | Nom | Lot |
|---|---|---|
| 2280 | `2280_sortie_autorisation_historique` | AS0 |
| 2281 | `2281_sortie_controle_immuable` | AS0 |
| 2282 | `2282_absence_declaration_appelant` (table du socle 12) | AS1 |
| 2283 | `2283_regime_sortie_regles` | AS2 |
| 2284 | `2284_regime_sortie_autorisations` | AS2 |
| 2285 | `2285_regime_sortie_reprise_062_065` (données, idempotente) | AS2 |
| 2286 | `2286_regime_sortie_exceptions` | AS3 |
| 2287 | `2287_accueil_mouvements_remises` | AS4 |
| 2288 | `2288_accueil_appareils` | AS5 |
| 2289 | `2289_accueil_visiteurs_acces` | AS6 |
| 2290 | `2290_accueil_main_courante` | AS7 |
| 2291 | `2291_surete_recherche` | AS8 |
| 2292 | `2292_surete_ppms` | AS9 |
| 2293 | `2293_surete_alertes` | AS10 |
| 2294 | `2294_surete_exercices` | AS11 |
| 2295 | réservée (connecteurs AS12) | AS12 |
| 2296–2298 | réserve | — |
| 2299 | `2299_retrait_sorties_062_065` — **seulement après décision de Florian**, point de sauvegarde vérifié (E8) | — |

Ordre : croissant ; aucune dépendance vers une migration d'un socle non fusionné sauf 2287 (attend `ent_eleve_remise` du lot A5 du socle 4 ; repli : lecture de `ent_eleve_contact_urgence.peut_recuperer` avec avertissement). Vérification d'absence de collision à refaire au moment d'écrire (lecture de `ent-rust/migrations` sur chaque branche distante).

## 28. Tests et recette

- **Unitaires purs** : `sortie::decider` (au moins 60 cas : chaque créneau × niveau × régime × option × événement, fuseau et changement d'heure, majorité le jour même), rapprochement d'un nom avec un accès refusé (homonymes, accents, inversions), calcul des listes de comptage (élève en permanence, cours annulé, pause méridienne, sortie collective, infirmerie).
- **Intégration** : chaque action ACS avec son droit (catalogue) ; matrice d'isolation (toutes les routes `/accueil/*`, `/sorties/*`, `/surete/*`) ; RLS strict des 40 tables ; immuabilité (UPDATE / DELETE refusés sur les tables en ajout seul) ; événements sans nom (R-AS20, test qui lit les charges utiles) ; lint de minimisation.
- **Garde de code** : aucune écriture SQL du 14 sur les tables du 4 (`ent_eleve_remise*`, `ent_eleve_interdiction`…) ; aucun `jsonb` dans 2280–2299 ; registre des tables complet.
- **Hors ligne** : tablette sans réseau 2 h (contrôles, remises), resynchronisation, conflit de deux remises.
- **Recette métier** : rejouer les 10 actions Next.js (NXA) ; journée type du §4 ; exercice PPMS complet avec chronométrage et compte rendu ; alerte annulée par erreur ; remise au retour d'un car à 22 h.
- **Équivalence écran / IA** sur `remise.lire` et `sortie.decider_lire` ; refus de l'IA sur toutes les commandes fermées.

## 29. Réglages signalés (D-023 : peu de réglages) — S-1400 à S-1410

| N° | Réglage | Défaut [R] |
|---|---|---|
| S-1400 | Pièce d'identité exigée à la remise (repris du socle 4) | oui |
| S-1401 | Journaliser toutes les sorties (pas seulement les exceptions) | non |
| S-1402 | Délai minimal entre l'annonce d'une sortie anticipée et la sortie | collège : la veille ; lycée : 2 h |
| S-1403 | Acceptation automatique des sorties exceptionnelles simples | non (oui possible pour les majeurs) |
| S-1404 | Mot papier accepté sans rappel d'un titulaire | non |
| S-1405 | Délai d'escalade d'une recherche d'élève manquant | 15 min collège, 30 min lycée |
| S-1406 | Âge minimal d'une personne autorisée à la remise | 16 ans |
| S-1407 | Enregistrer les livraisons au registre | non |
| S-1408 | Liste papier de comptage du jour par zone | oui |
| S-1409 | Site commun à deux établissements (visiteurs et alertes partagés) | non |
| S-1410 | Périodicité de vérification des mallettes | trimestrielle |

## 30. Décisions appliquées par défaut (D-031) et questions à Florian

### 30.1 Appliquées par défaut (D-031, à relire)

| N° | Question | Décision appliquée |
|---|---|---|
| Q-AS1 | Le lot 0 corrige-t-il l'existant 062–065 avant de le remplacer ? | **Oui** (AS0), car il est servi en production de l'ancienne interface et sa RLS n'est pas posée |
| Q-AS2 | Journaliser toutes les sorties ? | **Non** : exceptions seulement (R-AS8), option S-1401 |
| Q-AS3 | Âge minimal d'une personne autorisée mineure | **16 ans**, choisi explicitement par un titulaire (S-1406) |
| Q-AS4 | Une restriction de sortie est-elle une sanction ? | **Non** : décision éducative datée du 14, proposée éventuellement par le 13 ; une sanction reste au 13 |
| Q-AS5 | Conservation de la main courante | **3 ans** [H], à valider par le DPO |
| Q-AS8 | Accès refusé à un titulaire | possible (sûreté), **sans** toucher à ses droits sur la scolarité (R-AS9) |
| Q-AS9 | L'élève voit-il qui peut venir le chercher ? | **oui, sans les interdictions** |
| Q-AS10 | Nommage des tables | `ent_regime_sortie_*`, `ent_accueil_*`, `ent_surete_*` ; droits `voyages.*` pour le 42, `sorties.*` gardés pour le 14 |
| Q-AS11 | Absences des personnels déclarées à l'accueil | **socle 15** (raccourci depuis l'accueil) |
| Q-AS12 | Exercices PPMS surprises | **non** (circulaire de 2023) |
| Q-AS13 | Privé sous contrat | règles du public **comme bonnes pratiques**, transmissions facultatives |
| Q-AS14 | Biométrie, vidéo | **aucune** dans ScolIA |
| Q-AS15 | ScolIA signale-t-il aux forces de l'ordre ? | **jamais** ; geste de la direction |
| Q-AS16 | Registre des visiteurs | **oui**, 3 mois, sans photo ni numéro de pièce |
| Q-AS17 | Migration 2282 sur une table du 12 | **oui**, dans la plage du 14, avec accord du propriétaire (comme le 42) |

### 30.2 Soumis à Florian (engagent l'établissement ou sont des faits que lui seul connaît)

| N° | Question |
|---|---|
| Q-AS6 | Le **registre de sécurité** de l'établissement peut-il être tenu (en partie) par extrait ScolIA ? (commission de sécurité, assureur) |
| Q-AS7 | Procédure de **réquisition** du registre des visiteurs par la police : qui signe, quelle forme ? |
| Q-AS18 | Fournir le **règlement intérieur** (régimes de sortie) et le **PPMS** actuels du lycée pilote, anonymisés pour les essais |
| Q-AS19 | Faut-il reprendre les **déclarations d'accueil** de Next.js de l'année en cours (Q-T0-07) ? Recommandation : non |
| Q-AS20 | Date d'**arrêt du module accueil** de Next.js (geste de Florian, D-035) |
| Q-AS21 | **Réseau de l'Enseignement catholique** : consignes propres sur le PPMS et les transmissions ? |
| Q-AS22 | **DPO** : durées du §21.3, base légale du registre des visiteurs dans le privé, mention d'information |
| Q-AS23 | **Matériel** : tablette d'accueil, téléphones des AED, impression de cartes à QR (coût) |
| Q-AS24 | Connecteur vers un éventuel **contrôle d'accès** existant (fournisseur, contrat) |

## 31. Faits, hypothèses, contradictions

**Faits [F]** : §2.1 (code Rust, chemins et lignes) ; §2.3 (Next.js) ; migrations et droits existants.
**Hypothèses [H]** : la journée et l'année du §4 ; les durées de conservation ; l'usage de cartes à QR ; les besoins des agents.
**Contradictions relevées** :
1. `SOCLES.md` §14 cite « passage par badge (036) » comme existant du 14 : **036 est le passage au self** (socle 40) ; il n'existe aucun passage au portail par badge.
2. `SOCLES.md` §14 annonce « appel d'un parent transformé en déclaration d'absence » au 14 ; la **déclaration appartient au 12** (table 131) : le 14 en est la porte d'entrée (AS1), pas le propriétaire.
3. Le 42 consomme `accueil::remise_preparer` et `surete::presents` : **confirmés** ici (§6.3), avec le droit `remise.effectuer` en portée « sortie, jour du retour » qui remplace `sorties.remise` du §21.1 du 42 (devenu `voyages.*` en V0a).
4. Le 08 parle d'un « registre de sécurité du socle 14 » : il s'agit du **compte rendu** et de l'**extrait** pour le registre réglementaire, pas du registre lui-même (Q-AS6).
5. `04_ADMINISTRATIF.md` §8.8 porte un paramétrage `remise JSONB` (ligne 683) : contraire à D-036 ; à normaliser par le socle 4 (signalé, hors de ce socle).
6. `ent_metier_event` (charge utile JSONB, hors des politiques RLS 58x) contient le motif en clair des demandes de sortie (DR-06) : signalé au pilote (socle 1).

---

## 32. Ajout personas (`PERSONAS_14.md`, 2026-11-11)

> Ce paragraphe l'emporte sur les §0 à §31 en cas de contradiction. Rien n'a été supprimé plus haut.

### 32.1 Cinq bloquants tranchés (D-031)
| # | Bloquant | Décision appliquée |
|---|---|---|
| A | Année de doublon : l'assiduité officielle est Charlemagne (D-034) alors que le 14 est en usage réel (D-035) | **Mode doublon** (R-AS21) : déclarations dans ScolIA → socle 12 **et** file « à reporter dans Charlemagne » de la vie scolaire, case « reporté » (qui, quand), export du jour ; aucune écriture vers Charlemagne |
| B | Comptage en alerte dépendant du téléphone personnel | **Trois voies** (R-AS22) : appli, **poste fixe de la salle**, **responsable de zone** ; liste papier ; jamais d'obligation d'appli personnelle |
| C | Contrôle de masse à 16 h 30 ; téléphone interdit au collège | **Contrôle par exception** (R-AS23) : liste négative du créneau par portail ; QR élève au **lycée seulement** par défaut (S-1411) |
| D | Pas d'AIPD | **AIPD du socle** et fiches de traitement avant AS4–AS6 en usage réel (ACS-347) ; bases légales à valider par le DPO |
| E | Accord de la direction à chaque remise sur simple demande d'un titulaire | **Examen unique** sous 48 h (R-AS24) ; en attendant, accord d'un CPE ou de la direction ; **résidence du jour** affichée |

### 32.2 Règles ajoutées (R-AS21 à R-AS28)
| N° | Règle | Lot |
|---|---|---|
| R-AS21 | **Mode doublon** tant que l'établissement n'a pas basculé l'assiduité : toute déclaration de l'accueil apparaît dans la file « à reporter dans Charlemagne » ; la file se ferme à la bascule (réglage d'établissement, pas d'utilisateur) | AS1 |
| R-AS22 | **Comptage à trois voies équivalentes** ; le tableau de la cellule affiche la voie (« adulte », « responsable de zone », « poste de salle », « non remonté ») | AS10 |
| R-AS23 | **Contrôle par exception** : à chaque sonnerie, liste des élèves du portail **non autorisés maintenant** et des sorties **attendues** ; les autres sortent sans contrôle individuel | AS5 |
| R-AS24 | **Examen unique** d'une interdiction `demande_titulaire` : maintenue (accord à chaque remise, motivé), classée (sans effet), transformée (sorties seules) ; remplace la partie « à chaque remise » de R-AS3 | AS4 |
| R-AS25 | **Information préalable** d'une sortie anticipée au collège : l'avis doit partir avant 18 h la veille ; sinon l'élève reste en permanence | AS3 |
| R-AS26 | **Signal d'un témoin** : direction et cellule prévenues ; sans confirmation en 2 min (S-1413), l'alerte part **aux personnels seulement** sur la posture choisie ; les familles ne sont alertées que par une personne habilitée | AS10 |
| R-AS27 | **Avis de remise** : aux titulaires ayant `autoriser_sorties` ; pour un enfant confié, au service gardien et à l'assistant familial seulement | AS4 |
| R-AS28 | **Aucune visite liée à un élève** au registre ; un intervenant de soins a la qualité « intervenant » et un hôte **service** | AS6 |

Précisions : R-AS16 — le modèle collège comprend l'option `sortie_seul_fin_journee` (non cochée = remise obligatoire) ; le **mineur émancipé** enregistré est traité comme un majeur pour les sorties [NV, C. civ. 413-6].

### 32.3 Actions ajoutées (ACS-330 à ACS-348)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ACS-330 | **Relève de poste** : changer d'utilisateur en un geste (passkey) sur un poste partagé sans perdre l'état (remises préparées, visiteurs présents, à confirmer) | ACC | l'état appartient au **poste**, les actions à l'utilisateur | `accueil.poste_releve` | AS1 |
| ACS-331 | **Reconnaître l'appelant par son numéro** (saisi ou collé) : « numéro connu : mère de Léa » ; inconnu → à confirmer | ACC | port `personnes::par_telephone` minimal (4) ; date de dernière vérification affichée (M-1448) | — | AS1 |
| ACS-332 | **Appeler l'élève** à l'accueil depuis l'écran de remise (message à la vie scolaire ou au professeur de la séance) | ACC | heure d'appel notée ; plusieurs remises en parallèle | `accueil.eleve_appele` | AS4 |
| ACS-333 | Recevoir la **liste négative** du créneau pour son portail | AED, ACC · `sorties.controler` | précalculée ; hors ligne | — | AS5 |
| ACS-334 | Déclarer les **portails** (lieux du 5) et rattacher les postes | DIR, REF | — | — | AS5 |
| ACS-335 | Saisir les **coupons papier en masse** : grille par classe, ou lecture proposée par l'IA depuis la photo (N2) | CPE, VS · `sorties.gerer` | validation ligne par ligne | `sortie.autorisation_choisie` | AS2 |
| ACS-336 | Envoyer au professeur principal la **liste des coupons non rendus** de sa classe | CPE | sans les options des autres élèves | — | AS2 |
| ACS-337 | **Escalade d'un signal de témoin** sans réponse de la direction | SYS | R-AS26 | `surete.alerte_declenchee` (personnels) | AS10 |
| ACS-338 | Étendre les présents **hors temps scolaire** (étude, internat, association sportive, réunions) | SYS | ports des socles 43, 19, 41 | — | AS10 |
| ACS-339 | **Examiner une demande de titulaire** : maintenir, classer, transformer | DIR, CPE · `remise.interdire` (socle 4) ou délégation | sous 48 h ; motif ; l'autre titulaire n'en voit rien | `eleve.interdiction_changee` (4) | AS4 |
| ACS-340 | **Remise quotidienne au transport adapté** (organisme permanent, chauffeur non nommé, carte professionnelle) | ACC, VS | autorisation permanente à l'organisme lue au 4 | `surete.remise_effectuee` | AS4 |
| ACS-341 | Désigner l'**espace d'attente sécurisé** de chaque étage pour un élève à aide d'évacuation | REF | lieu du 5 ; registre de sécurité (R. 143-44 3°) | — | AS9 |
| ACS-342 | **Pousser au responsable de zone** l'existence d'une trousse d'urgence pour un élève de sa zone, sans diagnostic | SYS | port du 34 (SAN-312) | — | AS10 |
| ACS-343 | **Compter depuis le poste de la salle** ou **pour plusieurs salles** (responsable de zone) | PRF, responsable de zone | R-AS22 | `surete.comptage_ligne` | AS10 |
| ACS-344 | Compter et joindre les **agents sans compte** (collectivité, OGEC) | responsable de zone, cellule | SMS au numéro déclaré au 6 | `surete.comptage_ligne` | AS10 |
| ACS-345 | **Détecter les localisations répétées** d'un même élève par une même personne | SYS | S-1414 ; alerte à la direction | `accueil.localisation_anormale` | AS14 |
| ACS-346 | Tenir la **file « à reporter dans Charlemagne »** et cocher « reporté » | VS, CPE | R-AS21 ; export du jour | `accueil.declaration_reportee` | AS1 |
| ACS-347 | Préparer l'**AIPD** et les fiches de traitement du socle | pilote, DPO, DIR | modèle CNIL ; avant usage réel | — | AS14 (avancé) |
| ACS-348 | **Déclarer un appareil perdu** : effacement à distance, fait de main courante, information du DPO | ACC, ADM | violation potentielle à qualifier sous 72 h | `accueil.appareil_perdu` | AS5 |

Précisions aux actions existantes : ACS-252 et ACS-258 — **filigrane nominatif** sur les PDF exportés du PPMS ; ACS-296 / ACS-299 — compte rendu signé **sans nom d'élève** (effectifs et zones) ; ACS-145 — limite de 5 demandes par jour et par élève ; ACS-267 — la liste va au **suppléant** quand la séance est remplacée (15) et inclut l'**AESH** présente ; ACS-262 — rappel de révision à toute fermeture ou création de lieu (5) ; ACS-304 — compteur « exercice de nuit » si internat.

### 32.4 Cas limites ajoutés (CAS-87 à CAS-94)
| N° | Cas | Ce que fait ScolIA |
|---|---|---|
| CAS-87 | Élève refusé qui sort en courant | « parti sans autorisation » ; pas de poursuite ; recherche proposée |
| CAS-88 | La mère vient chercher l'enfant la semaine du père | remise permise (titulaire) ; pas d'information automatique du père ; pas d'arbitrage |
| CAS-89 | Parent d'un enfant placé qui se présente le jour d'une visite médiatisée au lycée | aucune remise ; visite attendue organisée par le service, hôte = référent ASE |
| CAS-90 | Mineure émancipée de 17 ans | traitée comme majeure pour les sorties si l'émancipation est enregistrée |
| CAS-91 | Élève majeur sous tutelle | le tuteur est traité comme un titulaire (socle 4) [NV] |
| CAS-92 | Alerte pendant une séance assurée par un suppléant | la liste va au suppléant (EDT réel) |
| CAS-93 | Orthophoniste qui vient voir un élève chaque mardi | visite « intervenant », hôte « vie scolaire » ; aucun lien à l'élève (R-AS28) |
| CAS-94 | Une élève sort avec une camarade non autorisée en disant « elle a le droit » | la camarade figure sur la liste négative (photo) : refusée |

### 32.5 Réglages ajoutés
S-1411 (QR élève : lycée oui, collège non) ; S-1412 (fiche papier de secours proposée : non) ; S-1413 (escalade d'un signal : 2 min) ; S-1414 (localisations répétées : 5 en 7 jours).

### 32.6 Lots révisés
- **AS1** reçoit ACS-330, 331, 346 (mode doublon) : sans eux, l'accueil ne peut pas remplacer Next.js pendant l'année de doublon.
- **AS4** reçoit ACS-332, 339, 340 ; R-AS24 et R-AS27.
- **AS5** reçoit ACS-333, 334, 348 ; R-AS23 ; S-1411.
- **AS10** reçoit ACS-337, 338, 342 à 344 ; R-AS22, R-AS26.
- **AS14 (partie AIPD, ACS-347)** est **avancé** avant la mise en usage réel d'AS4, AS5 et AS6.
- **Parcours minimal rentrée 2027** inchangé dans son principe (AS0 → AS5, AS9 au printemps, AS10–AS11 avant le premier exercice), **plus l'AIPD** ; bascule de Next.js accueil au plus tôt à la **rentrée de Toussaint 2027**, date finale de Florian (Q-AS26).

### 32.7 Questions ajoutées pour Florian
Q-AS25 (Charlemagne porte-t-il les régimes de sortie ?), Q-AS26 (date d'arrêt de Next.js accueil), Q-AS28 (DPO : bases légales, AIPD, durées), Q-AS29 (postes fixes dans les salles ?), Q-AS30 (transport adapté, internat). Volume final : **249 actions** (ACS-100 à ACS-348), **71 cas limites**, **28 règles**, **15 réglages**.
