# Socle 20 — Cahier de textes et travail personnel

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian), rédigée **après** le code (S2-6, migration 150) pour le rattraper (`docs/PILOTAGE.md` §4). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni au service Next.js (code Next.js lu de façon ciblée seulement, CLAUDE.md).
**Branche :** `ent-socle-20-conception`, partie de `origin/ent-mc0hoe` (`7467f2d`, tête d'`ENT` au 2026-11-10 : socle 6 P4, socle 42 V0a, RLS 580–587).
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (D-017, D-021, D-022, D-023, D-024, D-026, D-031, D-032, D-034, D-035, D-036, D-038) ; `docs/SOCLES.md` §20 ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; `docs/socles/10_EMPLOI_DU_TEMPS.md` (séance datée, R-E2, T0, AESH, PFMP) ; `06_PERSONNELS.md` (interventions, suppléances, lots P3–P4, migrations 354–355) ; `21_EVALUATIONS.md` (EVA-114, EVA-117, EVA-265, Q-EV1) ; `16_MESSAGERIE.md` (R-18) ; `08_NOTIFICATIONS.md` ; `07_DOCUMENTS.md` ; `34_SANTE.md` ; `IA_COEUR.md` ; `docs/socles/MIGRATIONS.md`.
**Passe personas** (même session) : `docs/socles/PERSONAS_20.md` (17 personas, 78 manques) ; ses ajouts sont au **§32** et marqués « ajout personas » ailleurs ; rien n'est supprimé.
**Conventions.** **[F]** fait vérifié dans le dépôt (chemin et ligne) ; **[V]** règle lue sur une source officielle (URL, consultée le 2026-11-11) ; **[NV]** non vérifiée ; **[H]** hypothèse ; **[R]** recommandation. Pour éviter les collisions avec les sessions parallèles, tous les identifiants propres au socle portent son préfixe : défauts **C-D1…**, actions **CDT-100…** (CDT-01 à CDT-10 de `SOCLE2_DOMAINES.md` repris), règles **R-C1…**, cas limites **CC-01…**, réglages **S20-01…**, questions **Q-CDT1…**, règles réglementaires **R-01…** (locales au §5).
**Plage de migrations imposée :** **2400–2419** (registre `MIGRATIONS.md`).

Sommaire : §0 En bref · §1 Thèse · §2 Existant et défauts · §3 Acteurs · §4 L'année · §5 Réglementaire · §6 Frontières · §7 Concepts · §8 Cahier du cours · §9 Travail personnel · §10 Rendre et rendre la copie · §11 Charge · §12 Continuité · §13 Adapter sans exposer · §14 Visa, inspection, archive · §15 Exercices · §16 Progressions · §17 IA · §18 Cas limites · §19 Droits · §20 Actions · §21 Règles · §22 Événements et avis · §23 Écrans · §24 Modèle de données · §25 Plan de migration · §26 Lots · §27 Tests · §28 Réglages · §29 Questions · §30 Faits et contradictions · §31 Décisions prioritaires · §32 Ajouts personas.

---

## 0. En bref

1. **L'existant est riche mais dangereux.** S2-6 a livré contenu de séance, travail à faire et à rendre, remises, retours annotés, charge, visa, historique et trois écrans ; mais **17 défauts** (§2.2), dont cinq graves : le **contenu des séances est détruit** quand les séances sont régénérées (C-D1, le D1 du socle 10, toujours là) ; un remplaçant d'**une seule heure** garde à vie l'écriture et la **lecture des copies** de toute la classe (C-D2) ; la **CPE et les autres professeurs lisent les copies et corrections** par l'agenda (C-D3) ; les **droits des liens parentaux** sont ignorés (C-D4) ; un **élève parti** dépose encore (C-D5). Le module Next.js est une ébauche sans contrôle de rôle (NX-CDT-1), rien à reprendre. → **lot C0** avant tout (§26).
2. **Le réglementaire tranche** [V, §5] : le cahier est un **document officiel à valeur juridique**, permanent pour élèves et familles, visé par la **direction et l'inspection**, organisé par discipline et par groupe, avec **date de don et échéance** et **tableau de la charge de travail** obligatoires (circulaire 2010-136) ; il reste **lisible un an** après l'année puis est **archivé cinq ans** ; l'**arrêté du 7-9-2026** rend **opposable** la plage calme (aucun devoir diffusé de 20 h à 7 h ni le week-end) sous dix mois, autorise l'IA générative des élèves **à partir de la 4ᵉ** et fait d'un usage non autorisé une **fraude** ; le règlement européen sur l'IA classe à haut risque l'**évaluation des acquis** : l'IA ne note ni ne commente une copie.
3. **Le modèle** : le cahier appartient au **cours** (socle 5), l'auteur est l'**intervenant en saisie à la date** (socle 6 : suppléance qui hérite de tout, remplaçant borné à sa séance), l'entrée vit sur la **séance stable** (socle 10) et ne disparaît jamais ; le travail a une **échéance-séance** qui suit les changements par **propositions de report** ; **préparer n'est pas publier** ; la **charge** se calcule **par élève**.
4. **Rendre est un acte prouvé** : dépôts immuables, accusé, empreinte, échéance effective par élève (aménagement, absence, panne), non-rendu = fait, jamais zéro ; le retour appartient à l'élève et suit la publication des notes du socle 21.
5. **Adapter sans exposer** : variantes et aménagements appliqués par défaut, jamais de motif ; AESH limitée aux élèves accompagnés ; encadrants d'étude et enseignants extérieurs (SAPAD) en lecture sans copies.
6. **187 actions** (CDT-100 à CDT-246 et CDT-250 à CDT-289), **90 cas limites**, **44 règles**, **16 lots** (C0, C2a, C1 à C14), modèle de données sans JSON (D-036) conforme à E1–E14, plage **2400–2419**. Parcours minimal vendable : C0, C2a, C1 à C6, C8, C11 (agenda et saisie mobile).
7. **(ajout personas)** Trois bloquants appliqués par défaut : (A) plage calme livrée **juste après C0** (lot C2a) pour la conformité de juillet 2027 ; (B) toute modification publiée **visible des lecteurs** avec ses versions (valeur juridique) ; (C) **corrigés d'exercices jamais envoyés au client**. Et : copie annotée lisible des familles, « vu » supprimé, accès tiers (inspection, SAPAD), lecture préparatoire du suppléant, projets pluriannuels, « travaillé en étude », traduction et fiche papier.
8. **À Florian** (§29, `PERSONAS_20.md` §8.2) : usage réel ou doublon du cahier en 2027-2028 (Q-CDT2) ; durées des copies et AIPD avec le DPO (Q-CDT3, Q-CDT14) ; charte et règlement intérieur (Q-CDT15) ; convention SAPAD (Q-CDT16) ; fournisseur de traduction (Q-CDT17) ; défaut Next.js NX-CDT-1.

---

## 1. Thèse : le cahier de textes est la trace datée de l'enseignement, le travail personnel est la semaine de l'élève

Pour l'**établissement**, le cahier de textes est un **document officiel** : il dit, séance par séance, ce qui a été enseigné et ce qui a été demandé aux élèves ; la direction le vise, l'inspection le consulte, il sert de preuve en cas de litige (contestation d'une évaluation, d'une orientation, d'un conseil de discipline) et de **mémoire** quand un professeur s'absente ou s'en va (§5). Pour le **professeur**, c'est un outil de préparation et de trace qu'il remplit chaque jour, souvent le soir, et qu'il ne veut pas remplir deux fois. Pour l'**élève**, c'est la réponse à trois questions : **qu'est-ce que j'ai à faire, pour quand, et comment je le rends ?** Pour la **famille**, c'est la seule fenêtre sur ce qui se passe en classe : savoir si l'enfant est à jour, l'aider sans se substituer à lui, comprendre une semaine chargée.

Le code existant (S2-6, migration 150) a déjà compris l'essentiel (contenu par séance, travail daté, remise en ligne, retour annoté, charge, visa, historique) ; il a été construit **avant** le cours du socle 5, les interventions datées du socle 6 et la séance stable du socle 10. Ce socle part de là et pose quatorze principes [R] :

1. **Deux objets, pas un.** (a) Le **cahier du cours** : la trace officielle, une entrée par **séance datée** et par **cours** (contenu, objectifs, documents, travail donné ce jour-là) ; (b) le **travail personnel** : ce que l'élève doit faire, avec une **échéance** (une séance du cours, ou une date et une heure), une **durée estimée**, éventuellement un **rendu** en ligne et un **retour**, éventuellement relié à une **évaluation** du socle 21. Un travail est *donné* lors d'une séance et *attendu* pour une autre ; il apparaît aux deux endroits du cahier.
2. **Le cahier appartient au cours, pas au professeur ni à une classe texte.** Le cours (socle 5) dit ce qui est enseigné à qui ; les **interventions** (socle 6, migration 354) disent qui l'enseigne à quelle date. L'auteur d'une entrée est l'intervenant **en saisie à la date de la séance** ; une suppléante qui arrive le 12 janvier écrit dans le **même** cahier, voit tout ce qui précède, et le titulaire de retour retrouve tout, sans copie ni transfert (principe 2 du socle 6).
3. **La séance est stable, le cahier aussi.** Une séance qui porte un contenu ou un travail n'est **jamais supprimée** (R-E2 du socle 10) ; si elle est annulée, déplacée, neutralisée ou retirée, le socle 20 **propose** le report du contenu prévu et des échéances vers la séance suivante du cours ; il ne déplace jamais rien en silence et ne perd jamais rien (défaut D1 du socle 10, toujours présent dans le code : §2).
4. **Ce que voit l'élève est calculé à la date, sur son public réel.** Un élève voit le cahier et les travaux des cours dont il est **membre à la date** (classe, groupe, inclusion ULIS, exclusion d'un cours, changement de groupe : `ent_public_cours_jour` du socle 5) ; un élève arrivé le 4 novembre voit le travail à venir **et** les contenus passés du cours pour rattraper ; un élève parti ne voit plus rien du cours, mais garde ses propres remises et retours.
5. **Préparer n'est pas publier.** Le professeur prépare à l'avance (progression, séances de la semaine prochaine, travail programmé) ; rien n'est visible des élèves et des familles avant la **date de publication** qu'il choisit (par défaut : le jour de la séance) ; la **plage calme** nationale (20 h–7 h et week-end, doctrine du numérique responsable, socle 16 R-18) s'applique : on peut **saisir** à 22 h, la diffusion et les avis partent à 7 h.
6. **La charge se voit avant de donner, par élève et non par classe.** Le professeur voit, au moment de fixer l'échéance, ce que **ses** élèves ont déjà ce jour-là : évaluations annoncées (socle 21), travaux longs, durée cumulée — calculé **sur les élèves réels** (un groupe de spécialité de terminale mélange quatre classes : la charge d'une classe ne veut rien dire). Il voit des **nombres et des natures**, jamais le nom d'un autre élève ni le contenu du travail d'un collègue hors de ses classes. Une alerte se lit, on peut passer outre en le disant ; aucune règle nationale ne fixe un plafond (§5), l'établissement choisit ses repères.
7. **Rendre en ligne est un acte daté et prouvé.** Une remise est horodatée par le serveur, chaque dépôt garde ses fichiers (objets immuables du socle 7, empreinte SHA-256) et l'élève reçoit un **accusé** ; « en retard » se calcule à partir de **son** échéance (prolongation individuelle, aménagement) ; un travail non rendu est un **fait**, jamais un zéro automatique (socle 21, Q-EV1) ; le professeur peut déposer une copie papier scannée au nom de l'élève.
8. **Le retour appartient à l'élève.** La copie annotée et le commentaire sont lisibles de l'élève et de ses responsables qui ont le droit de suivre la scolarité ; s'ils portent une note, ils suivent les règles de **publication** du socle 21 (une copie annotée ne publie jamais une note avant l'évaluation). Aucun camarade, aucun autre professeur, ni la vie scolaire ne lisent une copie ; la direction seulement sur demande motivée et journalisée (contestation).
9. **Adapter sans exposer.** PAP, PPS, PAI, UPE2A, haut potentiel : le professeur peut donner une **variante** de consigne, un délai, un format (police agrandie, audio, moins d'exercices) à certains élèves ; l'élève concerné et ses responsables voient **leur** version, l'AESH celle de l'élève accompagné ; personne ne voit **pourquoi** (aucun motif, aucune donnée de santé, socle 34 et 35).
10. **La continuité est un devoir de l'établissement.** Élève absent (« ce que j'ai manqué », calculé depuis l'appel du socle 12), hospitalisé (SAPAD), exclu temporairement (continuité des apprentissages, §5), professeur absent (travail laissé pour la classe, remplaçant d'une séance, suppléant), fermeture (plan de continuité pédagogique) : le cahier est l'outil, sans nouvelle saisie.
11. **Le visa et l'inspection lisent, ils n'écrivent pas.** La direction vise une **période** du cahier d'un cours ou d'un professeur ; le visa **fige la référence** de ce qui a été vu (versions), ce qui est modifié ensuite apparaît « modifié après visa ». L'inspecteur reçoit un **accès en lecture borné** (cours, dates, durée) accordé par la direction et journalisé, ou un export figé.
12. **L'année se ferme, le cahier s'archive.** À la clôture de l'année (socle 5), chaque cahier de cours est **figé** en lecture seule, exporté en un document daté et signé par empreinte (socle 7), lisible comme un cahier actif pendant l'année scolaire suivante puis archivé cinq ans (§5, R-06) ; le professeur réutilise **sa** progression l'an suivant sans réutiliser le cahier d'une autre classe.
13. **L'IA prépare, le professeur décide.** Reformuler une consigne, proposer une version adaptée, résumer ses notes de séance, suggérer des exercices, signaler une semaine chargée : oui, en brouillon relu. Noter une copie, classer un élève, détecter un « devoir fait par l'IA » pour sanctionner : **non** (règlement européen sur l'IA, annexe III, §5 ; D-032). Jamais sur des données de santé.
14. **Peu de réglages, de bons défauts** (D-023). Publication le jour de la séance, plage calme nationale, repères de charge par niveau à trois positions, formats de rendu usuels, rappel « à rendre demain » à 17 h : l'établissement change peu de chose, le professeur rien s'il ne le veut pas.

**Ce qui fera la différence au quotidien** [H — comparaison de mémoire avec Pronote, École Directe, ENT régionaux, non vérifiée au détail] : la **saisie du contenu en trente secondes** à la fin du cours, depuis la séance (web ou téléphone, photo du tableau comprise) ; la **semaine préparée le dimanche** et publiée seule, au bon moment ; la **charge visible avant de donner**, juste pour les élèves réels ; le **report automatique proposé** quand un cours tombe ; la **suppléance qui hérite de tout** ; la **remise prouvée** (fini « je l'ai envoyé, madame ») ; le **« ce que j'ai manqué »** de l'élève absent ; l'**agenda unique** de l'élève (travaux, évaluations annoncées, sorties, examens) que la famille lit sans surveiller ; le **visa en dix minutes** pour la direction.

---
## 2. Évaluation franche de l'existant

Lu sur `origin/ent-mc0hoe` (`7467f2d`, qui contient la tête d'`ENT` au 2026-11-10 : socle 6 P4, socle 42 V0a, RLS 580–587). Lecture seule, rien exécuté. Un audit indépendant (sous-agent `review-sonnet`) a relu le code ; les défauts C-D1 à C-D3 ont été **relus directement** (`150_cahier_textes.sql:14`, `referentiel_seances.rs:386-394`, `cahier.rs:341-367`, `cahier.rs:1984-2010`). Chemins relatifs à `ent-rust/`.

### 2.1 Ce qu'il y a [F]

| Brique | Où | Ce qu'elle fait |
|---|---|---|
| Ancien cahier | `ent_cahier_texte` (049, `classe_id` en 111) ; `GET/POST /cahier-texte` (`ent-api/src/routes.rs:655`, `service.rs:17087-17172`) | titre, consigne, « pour le » par **classe texte** ; écriture par rôle codé en dur ; **seule route appelée par les quatre prototypes mobiles** (`apps/ios-eleve/EntApiClient.swift:179`, `ios-famille:692`, `android/EntApiClient.kt:209, 1061`) |
| Contenu de séance | `ent_seance_contenu` (150) ; `cahier.rs:738-909` | une ligne par séance datée, version, historique ; publié dès l'enregistrement |
| Travail | `ent_travail` (150) ; `cahier.rs:971-1385` | classe **ou** groupe (CHECK), type `a_faire` / `a_rendre` / `controle`, durée 1–600 min, date limite, retard accepté, formats, suppression logique, version ; reprise idempotente de 049 + déclencheur |
| Remise et retour | `ent_travail_remise` (150) ; `cahier.rs:1700-1919` | une remise par élève, retard marqué, dépôts complémentaires, retour (commentaire, `note_id`) |
| Pièces | `ent_cdt_piece` (150) ; `cahier.rs:1442-1609` ; `ent-object/src/upload.rs` | genres ressource / séance / copie / annotation ; contrôle de signature binaire, liste blanche, 10 Mo ; **clé de stockage jamais exposée**, lien signé court en téléchargement forcé, accès sensible tracé |
| « Fait » et « vu » | `ent_travail_eleve` (150) ; `cahier.rs:1149-1159, 2016` | coche personnelle de l'élève ; « vu » écrit à l'ouverture du détail, sert au seul avis « modifié après lecture » |
| Historique | `ent_cdt_historique` (150, `instantane JSONB` : exception D-036 déclarée) | avant chaque modification ou suppression |
| Visa | `ent_cdt_visa` (150) ; `cahier.rs:2153-2239` | classe (+ professeur facultatif), période, commentaire ; **sans écran** |
| Agenda et charge | `cahier.rs:1951-2126` | agenda élève / famille par échéance ; charge par jour (classe + groupes rattachés), alerte > 120 min ou ≥ 2 contrôles |
| Droits | `ent-auth/src/actions.rs` ; `cahier_peut` (`cahier.rs:300-420`) | `cahier.lire`, `cahier.ecrire`, `cahier.viser` par le moteur D-017 ; portée « mes classes » calculée **à part** |
| Événements, avis | `cahier.*` dans l'outbox ; `cahier_reactions.rs` (consommateur `d7.notifications`, rejouable, idempotent) | travail donné, modifié après lecture, copie reçue, copie corrigée |
| Écrans Leptos | `web/crates/ent-web-app/src/modules/cahier/` | `/web/cahier` (professeur), `/web/cahier/travaux/:id` (copies), `/web/agenda` (élève, famille) ; non testés dans un navigateur |
| RLS | 585 | permissive sur les 11 tables (sans variable d'établissement : tout passe tant que `app.rls_strict` est inactif) ; le code n'utilise jamais `TenantTx` |
| Registre | `ent-db/src/registre_donnees.rs:268-276, 544, 573-575` | tables classées `evaluations_pedagogie`, portée établissement, données personnelles, export ; aucune durée de conservation |
| Tests | `ent-db/src/tests_cahier.rs` (21), `web/.../tests/cahier.rs` (3), `modele.rs` (11) | isolation entre établissements, pièces privées, retard, idempotence ; **rien** sur la suspension ou les droits fins d'un lien, le départ d'un élève, le suppléant (354), la régénération des séances, les parents séparés |

**Application Next.js** (lecture ciblée, CLAUDE.md) [F] : table `cahier_texte` (`db/schema-socle-faits.ts:373-399` : date de séance, classe texte, groupe, créneau, matière, contenu, travail, « à rendre le », « visible famille ») ; routes `app/api/cahier-texte/route.ts` et `app/api/famille/cahier-texte/route.ts` ; écrans `CahierTexteStaffClient.tsx`, `FamilleCahierClient.tsx`. **Ébauche** : l'écriture ne vérifie que la connexion (`requireAuth`, `route.ts:14, 32`) — **tout utilisateur connecté à l'intranet peut créer, modifier ou masquer n'importe quelle entrée** de l'établissement ; la famille voit par **classe texte** (aucun groupe), `visible_famille` vrai par défaut. Inventaire T0 : C06, « vide probable, sinon perdable ». Conformément à D-035 (précision 2), le module **ne fait pas foi** : il est refait par la méthode des socles, sans reprise de données (Q-CDT1 pour confirmation). Défaut signalé à Florian (Next.js intouchable) : **NX-CDT-1**.

### 2.2 Défauts trouvés (à corriger avant toute extension)

Gravité : **G** grave (sécurité ou perte de données), **I** important (résultat faux ou droit trop large), **C** confort ou dette.

| N° | Gravité | Défaut | Preuve | Correction (lot) |
|---|---|---|---|---|
| C-D1 | **G** | **Le contenu des séances est détruit** quand les séances sont régénérées (vacances ajoutées, créneau retiré) ou qu'un déplacement est refait : `DELETE FROM ent_edt_seance` sans garde, `ent_seance_contenu` et ses pièces en `ON DELETE CASCADE`, travaux détachés (`SET NULL`), historique orphelin, objets orphelins. C'est le D1 du socle 10 : **toujours présent** | `150:14, 29, 99-101` ; `referentiel_seances.rs:386-394` ; `edt_remplacement.rs:1170` [relu] | **C0** (avec T0 du socle 10) : clés `RESTRICT` composites ; la régénération ne supprime plus une séance porteuse (état `retiree`) ; test rouge puis vert |
| C-D2 | **G** | **Droit d'écriture et de lecture des copies permanent et trop large** : quiconque a été professeur ou **remplaçant d'une seule séance** d'une classe, ou a un service sur elle, garde `cahier.ecrire` sur toute la classe, **sans date ni état** : il liste et lit les copies et envoie des retours indéfiniment ; le titulaire garde la saisie pendant une suppléance (contraire à 354) ; le suppléant n'a **aucun** droit tant que la séance ne porte pas son nom ; la restriction `droits {cahier}` des interventions n'est jamais lue | `cahier.rs:341-367, 817, 1686-1695, 1801, 1868` [relu] | **C0** : portée calculée par `ent_prof_sur_cours(…, p_saisie)` du socle 6 (355) à la **date de l'action**, `droits ∋ 'cahier'` ; remplaçant d'une séance borné à **cette** séance |
| C-D3 | **G** | **Les copies et corrections fuient entre personnels** : l'agenda d'un élève renvoie la remise complète (commentaire de l'élève, commentaire de correction, `note_id`, pièces) à **tout** lecteur `cahier.lire` de l'élève (CPE, professeur d'une autre matière) | `cahier.rs:1984-2010` [relu] ; test `tests_cahier.rs:727` qui ne couvre que le lien de téléchargement | **C0** : remise seulement pour l'élève, le responsable habilité, l'intervenant en saisie sur le cours |
| C-D4 | **G** | **Droits fins des responsables ignorés** : tout lien actif lit consignes, copies, retours et reçoit les avis ; ni l'exercice de l'autorité parentale ni le droit « voir les notes » ne sont lus (la suspension judiciaire et les dates, elles, sont bien gérées par `ent_eleve_lien_actif`) | `cahier.rs:418-434, 1690` ; `cahier_reactions.rs:96-98, 160-163` | **C0** : `droits_lien` du socle 4 (« suivre la scolarité » pour le cahier, « voir les notes » pour un retour porteur de note) |
| C-D5 | **I** | **Élève parti ou changé de groupe** : il voit encore le travail, peut le rendre, le cocher, reçoit les avis (test de chevauchement sur [donné le, pour le], `date_sortie` non lue) | `cahier.rs:122-125, 577-595, 1718, 2026` ; `cahier_reactions.rs:146` | **C0** : public à la date de l'action (`ent_public_cours_jour`) ; l'élève garde la lecture de **ses** remises |
| C-D6 | **I** | **Séances de groupe traitées comme séances de classe** quand le service manque : contenu d'un cours de LV2 visible de toute la classe ; la liste des séances d'une classe renvoie celles des groupes d'autres professeurs à tout lecteur de la classe | `cahier.rs:644-666, 746-766, 928-929, 1670` | **C0** (contrôle par le cours du socle 5 dès qu'il existe), **C1** |
| C-D7 | **I** | **Ancienne route `/cahier-texte`** : écriture par rôle codé, par classe texte (un professeur d'un groupe écrit pour toute la classe), lecture du personnel **sans périmètre** ; le déclencheur recopie ces entrées dans `ent_travail` sans historique ni avis | `service.rs:17087, 17144-17148, 5354-5379` ; `150:174-191` | **C0** : lecture filtrée par le périmètre, écriture **fermée** (410 explicite) sauf pour les prototypes mobiles le temps de C11 (écriture redirigée vers la commande `cahier`) |
| C-D8 | **I** | **Retour sans historique et note d'un autre** : le retour écrase commentaire et `note_id` ; une remise corrigée ne peut plus être rouverte ; `note_id` n'est contrôlé que sur l'élève (on peut lier une note non publiée d'un autre professeur) | `cahier.rs:1733, 1868-1919` | **C0** (contrôle du `note_id` : même cours, même auteur ; historique) ; **C5** (cycle de retour) |
| C-D9 | **I** | **Visa** : le commentaire d'un visa individuel est lisible de tous les collègues de la classe ; aucun contrôle que le professeur visé enseigne dans la classe ; rien de figé de ce qui a été vu | `cahier.rs:2153-2239` | **C0** (lecture : visé et direction) ; **C8** (référence figée) |
| C-D10 | **I** | **La direction modifie et supprime le travail d'un collègue** avec `cahier.viser` ; l'historique garde l'acteur, pas l'auteur initial ; l'historique d'un travail est ouvert à tout autre écrivant de la cible | `cahier.rs:1233-1248, 2290-2299` | **C0** : la direction **retire de la diffusion** (contenu inapproprié, motif, avis à l'auteur), ne réécrit jamais ; historique : auteur, intervenants en saisie, direction |
| C-D11 | **I** | **Défense en profondeur absente** : pas de `TenantTx` (RLS sans effet), clés étrangères simples (classe, groupe, séance, travail, remise, pièce, visa), PK de `ent_travail_eleve` sans établissement, cascades latentes (remises, coches, pièces) | `cahier.rs` (0 `TenantTx`) ; `150:74, 99-101, 122` ; `240:45-46, 103` | **C0** : `TenantTx` ; **C1** : clés composites, `RESTRICT` |
| C-D12 | **I** | **Fichiers** : antivirus branché sur `AucuneAnalyse` par défaut ; liste blanche « tous documents » ; transport base64 dans du JSON ; pas de lien `ent_cdt_piece → ent_objet` (empreinte, immuabilité, E5) ; plafond de pièces vérifié sans verrou ; **impossible de retirer une pièce erronée** | `ent-object/src/upload.rs:3, 18, 193, 228` ; `cahier.rs:1442-1453` | **C0** (retrait logique d'une pièce, verrou) ; **C1** (registre `ent_objet`) ; antivirus et quarantaine au socle 7 (lot D0) |
| C-D13 | **C** | **Date limite en UTC** (fin de journée UTC), « pour le » dans le passé refusé (pas de saisie rétroactive), contenu publié dès l'enregistrement (pas de brouillon, pas de date de publication), séance « remplacée » inscriptible par le titulaire avec écrasement de l'auteur | `cahier.rs:813, 996` ; `SOCLE2_DOMAINES.md` §6.3 S2-6 | **C0** pour le fuseau (jour de l'établissement, `etablissement_fuseau` 265) ; **C2** pour le reste |
| C-D14 | **C** | **Charge** calculée par classe : ignore les groupes inter-classes (`classe_id` NULL) ; titres des travaux de groupes d'autres professeurs visibles d'un professeur de la classe | `cahier.rs:2069-2126` | **C4** (charge par élève, natures et nombres seulement) |
| C-D15 | **C** | `auteur_user_id` (identifiant de compte) exposé aux élèves et familles ; « vu » nominatif sans durée de conservation | `ent-types/src/cahier.rs` ; `cahier.rs:1149-1159` | **C0** (nom d'affichage de l'intervenant, plus d'identifiant) ; **C3** (purge du « vu » à la fin de l'année) |
| C-D16 | **C** | Clients incomplets : le client web n'expose ni modification, ni historique, ni visa, ni pièce de séance ; les prototypes mobiles ne voient ni groupes, ni remises, ni nouveaux travaux | `web/crates/ent-web-api/src/cahier.rs` ; `apps/*` | **C2**, **C8**, **C11** |
| C-D17 | **C** | Tests : les manques listés au §2.1 ; tests conditionnés à une base de labo (déjà relevé aux socles 3, 5, 9, 10) | `tests_cahier.rs` | chaque lot ; règle du kit (échec si la base manque en CI) |

### 2.3 Ce qui est à garder, à corriger, à remplacer

| Verdict | Quoi | Pourquoi |
|---|---|---|
| **Garder** | Filtre d'établissement dans tous les `SELECT_*` et test d'isolation ; contrôle d'accès aux pièces **par genre**, lien signé court, accès sensible tracé, clé jamais exposée, contrôle D2 par signature binaire ; suppression logique, version et historique avant modification, verrous `FOR UPDATE`, remise unique ; outbox dans la transaction ; avis rejouables et idempotents ; « fait » personnel ; « vu » jamais montré aux familles ; lecture des liens par `ent_eleve_lien_actif` ; modèle d'écran pur testé en natif (`modele.rs`) | Bien orientés et testés : c'est la base du modèle cible |
| **Corriger tout de suite** (lot **C0**) | C-D1 à C-D5, C-D7, C-D8 (contrôle du `note_id`), C-D9 (lecture), C-D10, C-D11 (`TenantTx`), C-D12 (retrait de pièce, verrou), C-D13 (fuseau), C-D15 (identifiant) | Perte de données et fuites dans du code **déjà intégré** |
| **Remplacer** | La cible classe XOR groupe (→ **cours** du socle 5) ; `cahier_peut` (→ portée du socle 6 et moteur) ; le type `controle` (→ évaluation annoncée du socle 21, lue par le port `evaluations::calendrier`) ; `ent_cdt_historique` JSON (→ tables de versions en ajout seul, §24) ; `ent_cdt_piece.storage_key` (→ `ent_objet`) ; l'ancienne route `/cahier-texte` et `ent_cahier_texte` (→ retrait après C11) | Construits avant le cours, les interventions et la séance stable |
| **Déplacer** | Le type « contrôle annoncé » vers le socle 21 (EVA-117) ; les copies « déposées au dossier » vers le socle 7 ; les pièces vers le service `documents` du socle 7 | Frontières (§6) |

**Écrans Leptos** : `prof.rs` (semaine du professeur) est la bonne base de l'écran « mon cahier » (§23, écran 1) ; `agenda.rs` devient l'agenda de l'élève et de la famille (écran 5) ; `copies.rs` l'écran de correction (écran 4). Les trois doivent passer par le composant « séance » commun du socle 10.

---
## 3. Acteurs, besoins réels, irritants

Hypothèses de terrain [H] (pratique courante en collège et lycée, public et privé sous contrat), à confronter à la passe personas (`PERSONAS_20.md`).

| Acteur | Ce qu'il fait avec le cahier | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Professeur** | Renseigne le contenu de 15 à 25 séances par semaine, donne le travail, prépare la semaine, joint des documents, ramasse et corrige des copies, annonce les évaluations | Saisir deux fois (cahier papier de l'élève + ENT + tableau) ; retrouver la séance ; recopier la progression de l'an passé classe par classe ; télécharger 30 fichiers un par un ; « je ne l'ai pas vu » ; les élèves qui rendent par courriel | Être accusé d'avoir donné du travail non noté au cahier ; un document protégé diffusé hors de la classe ; un parent qui conteste un devoir ; être surveillé par la direction ; une copie perdue | Saisie en 30 s depuis la séance, au téléphone ; la semaine préparée le dimanche, publiée seule ; copier une séance sur ses trois classes de même niveau ; la charge visible **avant** ; les copies en une liste, annotées dans le navigateur, rendues d'un geste | plusieurs fois par jour |
| **Professeur principal** | Coordonne la charge de sa classe (semaine de contrôles, brevet blanc), suit les élèves qui ne rendent rien | Demander à chaque collègue ses dates ; découvrir trois contrôles le même jeudi | Être tenu responsable d'une semaine impossible | Le calendrier de la classe en une vue, l'alerte avant, « élèves qui ne rendent rien depuis 3 semaines » (faits, sans jugement) | hebdomadaire |
| **Suppléant, remplaçant** | Reprend un cours en janvier, ou assure une seule séance | Ne pas savoir où en était le titulaire ; ne pas avoir accès ; écrire dans un cahier « à côté » | Contredire le titulaire ; ne pas voir les aménagements des élèves | Tout le cahier et la progression du cours dès sa prise de poste ; la séance du jour avec le travail laissé par le titulaire | ponctuelle, décisive |
| **Co-intervenant, assistant de langue, intervenant extérieur** | Co-anime (enseignement scientifique, AP, DNL) | Pas de place dans le cahier ; saisie par le titulaire | Écrire sous le nom d'un autre | Sa contribution signée dans la même séance | hebdomadaire |
| **Direction, adjoint** | Vise les cahiers, arbitre la charge, répond aux familles, prépare l'inspection | Ouvrir trente cahiers un par un ; savoir quels cahiers ne sont pas tenus | Un litige sans trace ; une inspection sur un cahier vide ; être accusé de surveiller | Un tableau « cahiers à jour » par cours (taux de séances renseignées), le visa d'une période en un écran, un export figé pour l'inspection | mensuelle à trimestrielle |
| **Inspecteur (IA-IPR, IEN)** | Consulte le cahier d'un professeur avant un rendez-vous de carrière ou une inspection | Recevoir des identifiants partagés ou des PDF hétérogènes | Voir des données d'élèves dont il n'a pas besoin | Un accès en lecture, borné, à la demande de la direction, sans nom d'élève | quelques fois par an |
| **CPE, vie scolaire, AED, étude, « Devoirs faits »** | Aide les élèves en étude, en permanence, à l'internat | Ne pas savoir ce que l'élève a à faire ; l'élève qui dit « rien » | Lire des copies ou des corrections qui ne la regardent pas | L'agenda de l'élève (consignes et échéances, sans copies ni notes) pour le dispositif d'aide aux devoirs | quotidienne |
| **AESH** | Accompagne un ou plusieurs élèves, en classe et parfois en étude | Recopier le travail sur le cahier de l'élève ; ne pas avoir les documents adaptés | Voir les élèves qu'elle n'accompagne pas | Le cahier et l'agenda **des seuls élèves accompagnés**, avec leurs variantes adaptées | quotidienne |
| **Professeur documentaliste** | Séances de recherche documentaire, EMI | Pas de cours « à lui » | — | Un cours ou une séance d'intervention où il écrit | hebdomadaire |
| **Élève mineur** | Lit ce qu'il a à faire, rend, reçoit son retour | Devoirs donnés au dernier moment ; consignes sur trois supports ; « le fichier ne passe pas » | Être noté en retard alors qu'il a rendu ; que ses camarades voient sa copie ; que ses parents voient tout en temps réel | Un agenda juste, un accusé de dépôt, savoir combien de temps prévoir, photographier sa copie au téléphone | plusieurs fois par jour |
| **Élève à besoins particuliers** (PAP, PPS, PAI, UPE2A, ULIS) | Idem, avec une version adaptée | Consignes illisibles ; délais inadaptés ; devoir de se signaler devant la classe | Que les autres sachent | Sa version adaptée (police, audio, moins d'exercices, délai), sans que personne d'autre ne le sache | quotidienne |
| **Élève majeur** | Idem ; décide de ce que voient ses parents (socle 4) | — | Que ses parents lisent sans qu'il le sache | Choisir, et le voir écrit | quotidienne |
| **Responsable** (dont parents séparés) | Lit l'agenda et le cahier, aide, rencontre le professeur | Deux enfants, trois ENT ; ne pas savoir si un travail est rendu ; être prévenu à 22 h | Découvrir au bulletin que rien n'était rendu ; être l'autre parent qui ne reçoit rien | Une semaine lisible par enfant, « rendu / non rendu » pour les travaux en ligne, un résumé hebdomadaire, des avis à l'heure juste | hebdomadaire |
| **Secrétariat** | Rarement : copie d'un cahier pour un litige, une demande d'un juge, un changement d'établissement | — | Diffuser trop | Un export daté, journalisé, sur demande de la direction | rare |
| **DPO** | Fixe les durées de conservation, répond aux demandes d'accès | Données sans durée ; copies conservées indéfiniment | Copies d'élèves et « vu » nominatifs gardés des années | Durées appliquées seules, registre clair, export par élève | annuelle |

---

## 4. L'année du cahier vue du terrain [H]

| Moment | Ce qui se passe | Ce que ScolIA doit faire |
|---|---|---|
| **Juin–août** | Le professeur prépare sa progression de l'an prochain ; il ne connaît pas encore ses classes | Progressions personnelles indépendantes du cours ; copie depuis l'an passé ; rattachement au cours dès que le service est attribué (socle 6) |
| **Rentrée** | Premières séances, présentation, liste de fournitures, règles de rendu | Cahier ouvert le jour d'ouverture du cours ; modèles de première séance ; publication le jour même |
| **Septembre–juin, chaque jour** | Contenu de séance, travail donné, documents ; corrections ; questions des élèves par messagerie | Saisie depuis la séance (web, mobile) ; travail lié à une échéance-séance ; remises ; question sur un travail ouvre un fil du socle 16 relié au travail |
| **Semaines d'évaluations, brevet blanc, bac blanc** | Concentration des contrôles | Charge par élève et par classe visible du PP, de la direction et de chaque professeur au moment de donner ; calendrier des épreuves communes (socle 38) dans l'agenda |
| **Absence d'un professeur** | Travail laissé, remplaçant, suppléant | Travail « pour la séance du… en mon absence » ; remplaçant écrit la séance ; suppléant hérite du cours (§11) |
| **Grève, intempéries, fermeture** | Cours annulés en masse ; continuité pédagogique à distance | Report proposé en lot ; plan de continuité : travail à distance publié sur les séances annulées, sans les rattacher à la présence |
| **Fin de période** | Visa de la direction ; préparation du conseil | Tableau de tenue des cahiers ; visa en un écran ; non-rendus de la période visibles au PP (faits) |
| **Inspection, rendez-vous de carrière** | L'inspecteur demande le cahier | Accès temporaire en lecture ou export figé |
| **Fin d'année** | Derniers travaux, rendu des copies, clôture | Rappel des copies non rendues ; clôture : cahier figé, archive, purge des « vu », conservation des copies selon la durée fixée |
| **Été** | Familles et élèves relisent | Lecture des cahiers de l'année close jusqu'à la date fixée (S20-03), puis seulement l'archive |

---
## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche par sous-agent sur sources officielles ; les pages HTML d'education.gouv.fr, d'eduscol et de cnil.fr renvoyaient une erreur 403 ce jour-là : les **PDF officiels**, Légifrance, EUR-Lex et le site de l'Assemblée nationale ont été lus. **[V]** lu sur la source ; **[NV]** non vérifié (extrait de recherche ou source secondaire seulement).

| N° | Règle | Source | Statut | Conséquence pour ScolIA |
|---|---|---|---|---|
| R-01 | Le cahier de textes numérique **se substitue** au cahier papier depuis la rentrée 2011 ; c'est un **document officiel à valeur juridique**, « de façon permanente » à la disposition des élèves et de leurs responsables légaux ; il sert de référence aux cahiers individuels et assure la **continuité** en cas d'absence ou de mutation d'un professeur, avec son suppléant ou son successeur | Circulaire n° 2010-136 du 6-9-2010, BO n° 32 du 9-9-2010, p. 14-15 — https://cache.media.education.gouv.fr/file/32/61/3/bulletin_officiel_men_32-09-09-10_153613.pdf | [V] (PDF lu en entier) ; aucune abrogation trouvée, toujours citée par le SDET 2024 et justice.fr (15-9-2025) ; statut sur circulaires.legifrance [NV] | principe 1, 2 ; historique en ajout seul ; continuité (§12.2) ; famille et élève lisent en permanence (R-C6, R-C7) |
| R-02 | Organisation **par discipline et par dispositif** ; tenu par chaque professeur concerné ; **à la disposition des personnels de direction et d'inspection qui devront les viser** ; accès par l'emploi du temps et par discipline ; activités des **groupes et sous-groupes** mentionnées | idem | [V] | cahier par cours (R-C1) ; accès depuis la séance (écran 1) ; visa direction (§14.1) et inspection (§14.2) |
| R-03 | « **Un tableau de la charge de travail** donnée à l'élève sera accessible » | idem | [V] | §11 est une **exigence**, pas un confort : la charge est visible de l'élève, de la famille et de l'équipe (C4 dans le parcours minimal) |
| R-04 | Contenu : contenu de la séance et travail à effectuer, avec documents, fichiers ou liens ; chaque travail porte **la date où il est donné** et **le jour où il doit être présenté ou remis** ; les **textes des devoirs et des contrôles** y figurent, ainsi que les exercices absents des manuels | idem | [V] | date de don et échéance obligatoires (contrainte en base) ; **sujet de l'évaluation** joint au cahier après l'épreuve (CDT-243, ajout §5) |
| R-05 | Le cahier numérique **ne dispense pas** l'élève de tenir un cahier individuel | idem | [V] | l'agenda de ScolIA aide, il ne remplace pas ; rien à imposer |
| R-06 | **Conservation** : en fin d'année, le cahier reste accessible **une année scolaire entière** « dans les conditions des cahiers de textes actifs » (enseignants, conseils d'enseignement, conseil pédagogique, conseils de classe, inspecteurs) ; il est ensuite **archivé et conservé cinq ans** ; le chef d'établissement choisit ceux qui sont versés aux archives de l'établissement. Le SDET (annexe opérationnelle 2024, § 3.16.2) résume : 7 ans au total (2 en base active, 5 en archivage conforme au code du patrimoine) | Circulaire 2010-136 ; SDET 2024 — https://eduscol.education.fr/document/58941/download | [V] | §14.3, §24.3 et S20-03 **corrigés** : lecture comme cahier actif jusqu'à la fin de l'année scolaire suivante, puis archive cinq ans, puis versement choisi par le chef d'établissement ou destruction |
| R-07 | La circulaire ne fixe **aucun délai** de renseignement ni aucune règle sur les données des élèves dans le cahier | idem | [V] (absence constatée) | S20-02 reste un repère ; R-C26 (ajout §5) : aucune donnée nominative d'élève dans une entrée ou une consigne (contrôle doux) |
| R-08 | **Aucun texte national** ne fixe un nombre maximal de contrôles par jour ni une durée de travail personnel au collège et au lycée | recherche négative | [V] (absence) | repères d'établissement, alerte non bloquante (§11.2, Q-CDT5) |
| R-09 | **Devoirs faits** : obligatoire pour tous les élèves de 6ᵉ depuis la rentrée 2023, facultatif de la 5ᵉ à la 3ᵉ | Note de service « Une nouvelle sixième », BO n° 16 du 20-4-2023 (MENE2302487N) — https://www.education.gouv.fr/bo/2023/Hebdo16/MENE2302487N | [NV] (extraits) ; circulaire de rentrée 2025 non lue | l'agenda de l'élève est lisible des encadrants de Devoirs faits (CDT-154), consignes sans copies |
| R-10 | Le **professeur principal** coordonne le suivi des acquis et de l'évaluation | Circulaire n° 2018-108 du 10-10-2018, BO n° 37 du 11-10-2018 | [NV] | vue de charge de la classe (CDT-153) |
| R-11 | **PAP** : pour un trouble des apprentissages, après avis du médecin de l'éducation nationale ; mesures pédagogiques ; révisé chaque année | C. éduc. D311-13 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000029781253 | [V] | les mesures sont des faits utiles du socle 35 ; jamais le motif (R-C8) |
| R-12 | **PAI** : le secret médical encadre la communication ; seuls les responsables ou l'élève majeur peuvent révéler le motif | Circulaire du 10-2-2021, BO n° 9 du 4-3-2021 ; fiche IH2EF | [NV] pour le BO ; cohérent avec le socle 34 [V] | aucune donnée de santé dans le socle 20 (§13.1) |
| R-13 | Aucun texte sur l'**accès de l'AESH** au cahier | recherche négative | [V] (absence) | accès borné aux élèves accompagnés (§13.3), à valider par la passe personas |
| R-14 | **Cadre de référence du numérique pour l'éducation** : par défaut, les ENT et logiciels de vie scolaire **ne diffusent aucune nouvelle information entre 20 h et 7 h en semaine ni du vendredi 20 h au lundi 7 h** (messagerie, notes, **devoirs**, vie scolaire) ; consultation et saisie restent possibles, les saisies faites pendant la plage sont visibles à sa fin ; un message rappelle le droit à la déconnexion ; la direction ajuste, y compris en urgence ; mise en conformité sous **10 mois** | Arrêté du 7-9-2026, JORF n° 0222 du 23-9-2026, § 4.4, critères 30-34 — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000054883370 ; opposable par le décret n° 2025-1165 du 5-12-2025 (C. éduc. R. 421-78-3, R. 421-78-4) ; réponse AN n° 11760, JO du 23-6-2026 — https://questions.assemblee-nationale.fr/q17/17-11760QE.htm | [V] (arrêté lu sur Légifrance ; décret par la réponse AN) | **la plage calme est une obligation** (R-C5), avec le message de rappel au professeur ; levée par la direction tracée (CDT-220) ; **échéance de mise en conformité ≈ juillet 2027** : avant l'année de doublon |
| R-15 | Même arrêté : l'**IA générative** est utilisable en classe **à partir de la 4ᵉ**, sous la supervision de l'enseignant ; au lycée, usage autonome possible dans le cadre fixé par l'enseignant ; utiliser l'IA générative pour un devoir **sans autorisation explicite** et sans travail personnel est une **fraude** ; aucune donnée personnelle dans une IA grand public ; IA générative pour les élèves seulement par un accès **anonymisé et sécurisé** (GAR), exigences 10-11, critères 35-36 | idem, section 5 | [V] | **chaque travail indique si l'IA est autorisée** et comment (`ia_autorisee` : non / pour chercher / pour relire / libre ; ajout §5, CDT-244) ; IA côté élève : jamais avant la 4ᵉ, jamais par un fournisseur grand public, désactivée par défaut (§17.2) |
| R-16 | **Règlement (UE) 2024/1689**, annexe III, point 3 : à haut risque les systèmes destinés à (b) **évaluer les acquis d'apprentissage**, y compris pour orienter l'apprentissage, (c) évaluer le niveau d'enseignement approprié, (d) **surveiller et détecter des comportements interdits** lors d'examens ; art. 5 § 1 f : **interdiction d'inférer les émotions** dans les établissements d'enseignement ; application au 2-8-2026, report éventuel au 2-12-2027 par un règlement « omnibus » | EUR-Lex — https://eur-lex.europa.eu/legal-content/FR/TXT/HTML/?uri=CELEX:32024R1689 | [V] pour le texte ; report [NV] | R-C20, R-C21 ; aucune IA qui note ou commente une copie (Q-CDT7), aucune surveillance pendant un exercice (§15), aucune analyse d'émotion |
| R-17 | Données de l'ENT : suppression des comptes **dans les trois mois** après la perte du droit d'accès | Arrêté du 30-11-2006 modifié le 13-10-2017, art. 6 — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000000427578 | [V] pour le texte ; statut actuel incertain (le kit juridique SDET 2025 en parle au passé) | compte supprimé au socle 1 ; les **données** du cahier suivent leurs durées propres (§24.3), les remises d'un élève parti suivent la règle « fin de l'année suivante » |
| R-18 | Responsable de traitement : le **chef d'établissement** (ou l'IA-DASEN), éventuellement conjointement avec la collectivité | Doctrine technique, septembre 2025 — https://www.education.gouv.fr/sites/default/files/document/439539.pdf | [V] (PDF) | ScolIA est sous-traitant ; les durées se valident avec le DPO de l'établissement (Q-CDT3) |
| R-19 | Les **créations numériques des utilisateurs** (dont les élèves) sont protégées par le code de la propriété intellectuelle ; l'**exception pédagogique** (CPI L122-5-4) couvre l'usage d'extraits d'œuvres par les enseignants, pour le public de la classe | Kit juridique SDET 2025, mis à jour en février 2026, § 4.6.1 — https://www.education.gouv.fr/sites/default/files/2025-07/kit-juridique-441540.pdf | [V] (PDF) | copie = œuvre de l'élève (§10.4, consentement pour un exemple) ; documents d'un cours lisibles du **seul public du cours**, jamais par lien public (R-C13, §16) |
| R-20 | SDET : service **cahier de textes** [UTI-AVE-CDT] avec cahiers personnels et de classe ou de groupe, gestion de la visibilité, **accès aux données de l'année précédente** | SDET 2024 (version 2025 existante, non lue) | [V] pour 2024 | cohérent avec §14.3 (lecture de l'année précédente) ; à relire sur la version 2025 avant C8 [NV] |
| R-21 | **Parents** : le parent sans exercice de l'autorité parentale garde le droit et le devoir de surveiller l'éducation et doit être **informé des choix importants** (C. civ. 373-2-1) ; les parents sont « régulièrement informés » des acquis (C. éduc. D111-3) ; tout refus d'information est motivé (D111-4) ; les conditions d'accès des parents à l'ENT relèvent du **règlement intérieur** (D111-5) ; mêmes informations à chaque parent (justice.fr, 15-9-2025) | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000022469781 ; https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006166752/ ; https://www.justice.fr/fiche/college-lycee-information-parents-scolarite-enfant | [V] | les deux responsables ont un accès **égal par défaut** ; restriction seulement sur décision enregistrée au socle 4 ; le refus de lecture (lien suspendu) est motivé par la pièce du socle 4 ; renvoi au règlement intérieur dans les réglages (S20-08) |
| R-22 | **Élève majeur** : aucun texte explicite trouvé sur l'accès des parents au cahier ; l'autorité parentale cesse à la majorité | C. civ. 371-1 [NV] | [NV] | règle du socle 4 (l'élève majeur décide), appliquée ici |
| R-23 | **Fermeture, continuité pédagogique** : réguler la charge de travail et le temps d'écran, alterner visio et travail autonome, donner un calendrier de travail avec les liens, publier **en journée** ; le chef d'établissement peut ajuster la plage de déconnexion | Fiche eduscol « continuité pédagogique » pour les chefs d'établissement, octobre 2026 — https://eduscol.education.gouv.fr/sites/default/files/document/fichechefsetablissementcontinuitepedagogiquepdf-130159.pdf | [V] (PDF) | §12.3 : charge appliquée aussi à distance, calendrier de travail dans l'agenda, levée de la plage calme tracée |
| R-24 | **PFMP** : enseignant référent et suivi individualisé ; aucune règle trouvée sur le cahier pendant les périodes | Circulaire n° 2016-053 du 29-3-2016, BO n° 13 du 31-3-2016 | [NV] | pendant une PFMP, les cours du lycée professionnel sont suspendus pour l'élève (socle 10 §11.9) : aucune échéance n'est posée, l'agenda affiche la période |

**Ce que le réglementaire change dans la conception** : (1) la charge de travail est une **obligation** de la circulaire (R-03) ; (2) la plage calme est **opposable** depuis l'arrêté du 7-9-2026, avec une mise en conformité d'environ dix mois (R-14) ; (3) chaque devoir doit dire si l'**IA est autorisée** (R-15) ; (4) la conservation est **un an actif puis cinq ans d'archive** (R-06) ; (5) les sujets des contrôles appartiennent au cahier (R-04) ; (6) aucun texte ne règle l'AESH, la durée des copies d'élèves ni l'élève majeur : ce sont des recommandations à faire valider (Q-CDT3).

---
## 6. Frontières

### 6.1 Tableau

| Objet | Socle 20 | Autres socles |
|---|---|---|
| Cours (ce qui est enseigné à qui), public à la date | lit | **5** possède (`ent_cours`, `ent_public_cours_jour`) |
| Intervenants d'un cours, saisie à la date, suppléance | lit (`intervenants_du_cours`, `en_saisie`) | **6** possède (354, 355) ; **15** décide du remplacement |
| Séance datée, ses états, annulation, déplacement | lit, **réagit** (report proposé) ; demande au 10 de ne jamais supprimer une séance porteuse | **10** possède |
| Contenu de séance, travail personnel, remises, retours, exercices, progressions, visa, archive du cahier | **possède** | — |
| Évaluation annoncée, résultat, publication d'une note | relie un travail à une évaluation ; affiche l'annonce | **21** possède (EVA-114, EVA-117, EVA-265, EVA-271) |
| Absences des élèves à la séance | lit (« ce que j'ai manqué ») | **12** possède |
| Exclusion de cours, exclusion temporaire, punition « devoir supplémentaire » | fournit le cahier pour la continuité ; un devoir supplémentaire est une **punition** du socle 13, pas un travail du cahier | **13** possède |
| Aménagements (PAP, PPS), AESH, élèves accompagnés | lit les **faits nécessaires** (cet élève a droit à un délai, à une version adaptée), jamais le motif | **35** possède ; **34** pour le PAI |
| Fichiers, objets, antivirus, conversion, coffre | appelle le service `documents` | **7** possède |
| Avis, plage calme, résumé | déclare ses avis (`AvisSurEvenement`) | **8** possède |
| Question d'un élève sur un travail, message aux familles des non-rendus | ouvre un fil relié au travail | **16** possède |
| Épreuves communes, examens blancs | les affiche dans l'agenda | **38** possède |
| Sorties | affiche la sortie dans l'agenda ; travail « à faire pendant / après la sortie » | **42** possède |
| Ressources numériques (manuels, GAR) | lien vers la ressource | **9** (GAR), **45** (CDI) |
| Espaces de travail collaboratif, bureautique en ligne | lien vers un document de l'espace ; rendu d'un document ArtCraft | **32** possède |
| Classes virtuelles | lien vers la visio de la séance à distance | **33** possède |
| Charge de saisie des enseignants (T-29) | fournit le taux de tenue du cahier | **47** pilotage |
| Pilotage, indicateurs | agrégats sans nom | **47** |

### 6.2 Ports exposés par le socle 20 [R]

| Port | Appelé par | Sens |
|---|---|---|
| `cahier::seance(seance_id) -> EntreeCahier` (contenu, travail donné, travail attendu, avec la portée de l'appelant) | 10 (détail d'une séance), 11, 12 (appel : « contenu du jour »), 15 | lecture |
| `cahier::agenda(eleve, plage) -> Vec<Echeance>` (sans copies ni retours sauf droit) | 2 (fiche), 11, 12 (« Devoirs faits », étude), 35 (AESH), 29 (à-faire) | lecture |
| `cahier::charge(eleves | cours, plage) -> Charge` (nombres et natures, jamais de nom ni de contenu hors portée) | 21 (EVA-217), 38, 42, 47 | lecture agrégée |
| `cahier::manque(eleve, plage) -> Vec<EntreeCahier>` (« ce que j'ai manqué » à partir des absences du 12) | 12, 13 (exclusion), 2 | lecture |
| `cahier::non_rendus(eleve | cours, periode) -> Vec<Fait>` | 21 (signalement au bulletin), 23 (conseil), 2 | lecture |
| `cahier::seances_porteuses(seances) -> Vec<SeanceId>` (contenu, travail, remise attachés) | **10** (régénération, retrait, déplacement) | lecture — **garde** de R-E2 |
| `cahier::proposer_report(seance_id, motif)` | 10 (réaction à `edt.seance_annulee`, `_deplacee`, `_retiree`, `_neutralisee`) | commande : crée des propositions, n'applique rien |
| `cahier::travail_lie(evaluation_id)` | 21 | lecture |
| `cahier::taux_tenue(cours | agent, plage)` (séances passées renseignées / attendues) | 47, direction | lecture agrégée |
| trait `ControleCloture` (« cahiers non visés », « copies non rendues », « travaux en brouillon jamais publiés ») | 5 (clôture d'année) | lecture |

### 6.3 Ports consommés [R]
Socle 5 : `cours`, `public_du_cours` / `ent_public_cours_jour`, `annee_a_la_date`, `jours_ouvres`, `etat_annee` (clôture). Socle 6 : `intervenants_du_cours(cours, date)`, `en_saisie(service, date)`, `droits_intervention` (`{cahier}`), `explique_acces`. Socle 10 : `seances(cible, plage)`, `seance(id)`, prochaine séance du cours, états et événements de séance. Socle 4 : `droits_lien` (suivre la scolarité, voir les notes), interdictions. Socle 12 : absences à la séance. Socle 21 : `calendrier(eleve | cours, du, au)`, `creer_brouillon_depuis_travail`, `etat_publication(evaluation)`. Socle 35 : `amenagements_utiles(eleve, cours)` (faits sans motif : délai majoré, version adaptée, police, lecture audio) ; `accompagnes(agent_aesh, date)`. Socle 7 : `deposer`, `lien_signe`, `convertir`, `figer`. Socle 8 : `aviser`. Socle 1 : `autoriser`, `tx`, `publier`, journal.

### 6.4 Points à accorder avec les voisins [R]
- **Socle 10, T0** : le correctif D1 du socle 10 et C-D1 ici sont **le même** ; un seul lot, porté par le premier qui part, avec le test commun « régénérer après l'ajout d'un jour férié conserve le contenu et le travail ». La séance passe à `retiree` ; `cahier::proposer_report` est appelé.
- **Socle 21** : le type `controle` d'`ent_travail` devient une **évaluation annoncée** du 21 ; tant que la brique `evaluations` (V1) n'existe pas, le cahier garde la nature `annonce_evaluation` en lecture seule (reprise, §25). « Ce travail sera noté » appelle `creer_brouillon_depuis_travail` (EVA-114) ; supprimer le travail ne supprime jamais l'évaluation (21 §4.4).
- **Socle 6** : la restriction `droits ⊆ {appel, notes, cahier, appreciations}` d'une intervention est lue ici ; une intervention `assistant` / `intervenant` lit le cahier et peut écrire **sa** contribution à une séance si `cahier` est dans ses droits (ce que 06 §8.5 ne prévoit pas : **proposé**, Q-CDT9).
- **Socle 35** : le port `amenagements_utiles` n'existe pas encore ; en attendant, la variante adaptée est choisie par le professeur à la main, sans motif (C7).
- **Socle 16** : un fil « question sur ce travail » porte la référence du travail ; le professeur répond une fois à tous (réponse publique ajoutée au travail comme précision, version) ou en privé.

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]

| Terme | Définition | À ne pas confondre avec |
|---|---|---|
| **Cahier du cours** | L'ensemble daté des entrées d'un cours (socle 5) pour une année : c'est le « cahier de textes de la classe » réglementaire, découpé par cours | Le cahier personnel de l'élève (son agenda) ; un cahier « du professeur » |
| **Entrée de séance** | Le contenu d'une séance datée pour un cours : titre, contenu, objectifs, documents, contributions des co-intervenants | La séance elle-même (socle 10) |
| **Travail** | Ce que des élèves doivent faire pour une échéance : consigne, nature, durée estimée, mode de rendu | Une évaluation (socle 21) ; une punition (socle 13) |
| **Échéance** | « Pour la séance du mardi 14 » (échéance-séance, qui suit la séance si elle bouge) ou « pour le 14 à 18 h » (échéance-date) | La date où le travail est donné |
| **Échéance effective** | L'échéance d'un élève donné, après prolongation individuelle ou aménagement | L'échéance du travail |
| **Public du travail** | Les élèves du cours à la date de l'échéance, ou une liste restreinte (rattrapage, groupe de besoin) | La classe |
| **Variante** | Une version adaptée de la consigne (texte, documents, durée, échéance) pour certains élèves, sans motif | Un autre travail |
| **Remise** | L'ensemble des dépôts d'un élève (ou d'un groupe d'élèves) pour un travail | Un dépôt |
| **Dépôt** | Un envoi horodaté de fichiers ou de texte, avec accusé et empreinte, jamais modifié | La remise |
| **Retour** | Le commentaire et la copie annotée du professeur sur une remise ; éventuellement relié à un résultat du socle 21 | La note |
| **Statut de suivi** | Ce que le professeur constate : `attendu`, `rendu`, `rendu_en_retard`, `rendu_papier`, `non_rendu`, `dispense`, `excuse` | La coche « fait » de l'élève |
| **Coche « fait »** | Outil personnel de l'élève, vu de lui seul | Un rendu |
| **Publication** | Le moment où une entrée ou un travail devient visible des élèves et familles | L'enregistrement |
| **Proposition de report** | Un report suggéré par le système après un changement de séance, appliqué par un humain ou à l'échéance d'un délai (S20-05) | Un report |
| **Visa** | L'acte daté de la direction qui atteste avoir pris connaissance d'une période du cahier d'un cours, avec la référence figée des versions vues | Une validation du contenu |
| **Accès d'inspection** | Une lecture bornée (cours, dates, durée), accordée par la direction à un inspecteur, journalisée | Un compte de personnel |
| **Progression** | Le plan personnel d'un professeur (étapes, durées, ressources), réutilisable d'année en année, rattachable à un cours | Le cahier |
| **Exercice** | Un ensemble de questions à correction automatique (QCM, réponse courte, numérique, association) joint à un travail | Une évaluation notée |
| **Tentative** | Une passation d'exercice par un élève, avec ses réponses et le score **proposé** | Un résultat |

### 7.2 Cinq distinctions qui manquent aujourd'hui [F → R]
1. **Contenu de séance et travail** sont séparés en base (bien) mais le travail vise une classe ou un groupe, pas le **cours** : un cours de LV2 en barrette ou un groupe de spécialité ne se modélisent pas proprement.
2. **Échéance-séance et échéance-date** sont confondues (`pour_le` DATE + `date_limite`) : quand la séance bouge, l'échéance ne suit pas.
3. **Enregistrer et publier** sont confondus : tout est visible dès la saisie, y compris une séance de la semaine prochaine.
4. **Constat du professeur et coche de l'élève** : « fait » existe, « non rendu » constaté n'existe pas.
5. **Lire et écrire** reposent sur la même portée « mes classes » permanente, au lieu de l'intervention datée.

---

## 8. Le cahier du cours : entrées, publication, report

### 8.1 Une entrée par séance et par cours [R]
- Pour chaque séance datée d'un cours (états `prevue`, `modifiee`, `ajoutee` du socle 10), le cahier présente une **entrée** (vide tant que rien n'est écrit). Une séance commune à deux cours (fusion de groupes, co-enseignement de deux disciplines) a une entrée **par cours**, avec copie proposée de l'une vers l'autre.
- Champs : titre (court), **contenu** (texte mis en forme limitée : titres, listes, gras, liens, formules simples ; balisage maison sans HTML libre, rendu échappé côté serveur), objectifs ou notions (liste libre, ou étapes d'une progression), documents (socle 7), liens (https seulement, ressources GAR), **contributions** des co-intervenants (signées), travail donné (§9).
- **Qui écrit** : l'intervenant en saisie sur le cours **à la date de la séance** (titulaire, co-enseignant, suppléant), le remplaçant de **cette** séance (modification `intervenant` du socle 10), un intervenant `assistant` ou `intervenant` pour sa seule contribution si `cahier` est dans ses droits. Chacun signe ce qu'il écrit (`auteur_agent_id`).
- **Quand** : à tout moment pour une séance future (préparation, brouillon) ; le jour même et après ; une séance passée reste modifiable **jusqu'à la clôture de l'année**, chaque modification versionnée ; après un visa, la modification est marquée « modifiée après visa ».
- **Délai de renseignement** : aucun délai national chiffré n'est retenu comme bloquant (§5) ; un repère d'établissement (S20-02, défaut : le soir du jour de la séance) alimente le tableau de tenue, jamais une sanction automatique.

### 8.2 États d'une entrée et d'un travail [R]

| État | Sens | Visible élève / famille |
|---|---|---|
| `brouillon` | En préparation, visible de l'auteur et des intervenants en saisie du cours | non |
| `programmee` | Prête, avec une date-heure de publication | non, jusqu'à `publie_a` |
| `publiee` | Visible (sous réserve de la plage calme : la visibilité commence à `visible_a` = `publie_a` décalé hors plage calme) | oui |
| `retiree` | Retirée de la diffusion par l'auteur ou la direction (erreur, contenu inapproprié), motif obligatoire, versions gardées | non (l'élève qui l'avait vue voit « retiré par le professeur ») |

**Défaut de publication** (S20-01) : une entrée est publiée **le jour de la séance à l'heure de début** ; un travail est publié **à la fin de la séance où il est donné** (ou tout de suite s'il est donné hors séance). Le professeur peut publier plus tôt (sujet de la semaine prochaine) ou plus tard.

### 8.3 Plage calme [R, socle 16 R-18, socle 8]
- Saisir est toujours possible. Ce qui est **publié** pendant la plage calme (défaut national : 20 h–7 h du lundi au vendredi, du vendredi 20 h au lundi 7 h, vacances) devient visible et déclenche ses avis **à la fin de la plage** ; le professeur le voit écrit (« visible lundi à 7 h »).
- Exception : la direction peut lever la plage pour une diffusion de continuité (fermeture soudaine) ; tracé.
- Une échéance ne peut pas être fixée **pendant** la plage calme (pas de rendu « pour 23 h 59 ») sauf choix explicite du professeur pour un lycéen (S20-06, défaut : interdit avant la seconde, alerte au lycée).
- Un travail publié **après** 17 h pour le lendemain est signalé à l'auteur (« donné moins de 24 h avant ») et apparaît au tableau de charge (G18 de la matrice).

### 8.4 Report quand la séance change [R]

| Changement (socle 10) | Entrée de séance | Travail **donné** ce jour-là | Travail **attendu** ce jour-là (échéance-séance) |
|---|---|---|---|
| Annulée | proposition : déplacer le contenu prévu (brouillon ou publié non encore réalisé) vers la prochaine séance du cours | reste donné (déjà publié) ou suit le contenu (brouillon) | **proposition** : échéance → prochaine séance du cours ; si la proposition n'est pas traitée avant la séance, elle s'applique et l'élève est prévenu (S20-05) |
| Déplacée | suit la séance d'arrivée (même identité de contenu) | suit | suit la séance d'arrivée ; avis si la nouvelle date est **plus tôt** (jamais sans accord du professeur) |
| Neutralisée (sortie, examen) | proposition de report | idem | proposition ; un élève **en sortie** avec la classe n'est pas en retard |
| Retirée (nouvelle version d'EDT) | rattachée à la séance qui la remplace si elle existe, sinon proposition | idem | idem |
| Non assurée (professeur absent non remplacé) | travail laissé « en mon absence » (§11.2) publié ; contenu vide | — | échéance maintenue par défaut (le travail a pu être fait), proposition de report à la prochaine séance |
| Annulation en masse (grève, fermeture) | **une** proposition groupée par professeur, à traiter en un écran | — | idem, groupées |

Règle : **rien ne disparaît**, rien n'est avancé sans accord, l'élève et la famille reçoivent **un seul** avis par changement effectif.

---

## 9. Le travail personnel

### 9.1 Donner un travail [R]
- Depuis la séance (cas normal), depuis le cahier du cours, depuis l'agenda de la classe, ou en **un geste pour plusieurs cours** du professeur (mêmes 3ᵉ) : un travail par cours, reliés.
- Champs : titre, **consigne**, nature (`exercices`, `lecon`, `lecture`, `redaction`, `recherche`, `projet`, `revision`, `oral`, `autre`), **durée estimée** (obligatoire au collège, défaut proposé par nature et niveau, S20-07), **échéance** (séance du cours proposée : la prochaine, la suivante ; ou date-heure), **mode de rendu** (`aucun`, `en_ligne`, `en_classe`), formats acceptés (si en ligne, préréglages : PDF, images, documents bureautiques, audio, texte saisi), retard accepté ou non, **remise individuelle ou par groupes**, documents et liens, exercices à correction automatique (§14), relié à une évaluation (« sera noté », socle 21), **usage de l'IA autorisé** (`non`, `recherche`, `relecture`, `libre` ; défaut `non` ; obligatoire à afficher, arrêté du 7-9-2026, R-15).
- **Public** : le cours à la date de l'échéance (défaut) ; ou une **liste restreinte** (rattrapage d'absents, groupe de besoin, volontaires) ; un élève hors du cours ne peut jamais être ajouté.
- **Contrôles serveur** : échéance après la date de don (ou même jour si séance plus tardive) ; échéance dans l'année ; pas d'échéance pendant les vacances ou un jour fermé (proposée à la séance suivante) ; pendant la plage calme (§8.3) ; charge (§10, alerte non bloquante) ; formats compatibles avec les aménagements connus (§12).
- **Modification** : avant publication libre ; après publication, versionnée (**ajout personas, R-C29** : toujours visible « modifié le … » des lecteurs, versions lisibles ; seul l'avis peut être omis), avec **avis** aux élèves seulement si un élément utile change (échéance, consigne, documents, mode de rendu) — pas pour une faute de frappe (le professeur choisit « modification mineure, sans avis ») ; une échéance **avancée** demande une confirmation explicite et prévient toujours.
- **Suppression** : un travail sans remise peut être retiré (état `retire`, motif) ; avec remises : **retrait de la diffusion** seulement, les remises restent à l'élève.

### 9.2 Ce que voit l'élève [R]
- L'**agenda** : par jour d'échéance, les travaux de ses cours (publiés, visibles), les évaluations annoncées (socle 21), les épreuves communes (38), les sorties (42) qui le concernent ; pour chacun : matière, intervenant (nom d'affichage), consigne, durée, mode de rendu, **son** échéance effective, **son** statut (à faire, rendu, en retard, retour disponible), sa coche « fait ».
- Le **cahier** de chacun de ses cours : contenus des séances passées (depuis son arrivée dans le cours **et** avant, pour rattraper) et à venir publiées.
- **Jamais** : la remise d'un camarade, la liste des non-rendus, les variantes des autres, les statuts « dispensé » ou « excusé » des autres, le brouillon.

### 9.3 Ce que voit la famille [R]
- Un responsable qui a le droit de **suivre la scolarité** (socle 4) voit l'agenda et le cahier de l'enfant comme l'élève, plus le **statut constaté** des travaux à rendre (rendu / non rendu / en retard) ; il ne voit **pas** la coche « fait » (outil de l'élève, R-C14) ni les fichiers déposés par l'élève par défaut (S20-08 : l'établissement peut ouvrir la lecture des copies rendues aux responsables) — **(ajout personas, R-C37)** : S20-08 est tranché, les responsables habilités lisent la **copie annotée rendue**, jamais les dépôts avant le retour ; il voit le **retour** (commentaire, copie annotée) si le retour ne porte pas de note, ou si la note est publiée et qu'il a le droit « voir les notes ».
- Parents séparés : chaque responsable habilité voit la même chose et règle ses avis ; un responsable sans exercice de l'autorité parentale mais **informé** (C. civ. 373-2-1, §5) voit le cahier et l'agenda si le socle 4 lui ouvre le suivi ; un lien suspendu (décision de justice) ne voit rien et ne reçoit rien.
- Élève majeur : les responsables ne voient que ce que l'élève a ouvert (socle 4).

---

## 10. Rendre, suivre, rendre la copie

### 10.1 La remise [R]
- Ouverte du moment de publication à l'échéance effective (puis en retard si accepté, jusqu'à une **date de clôture** du ramassage, S20-09, défaut : 14 jours après l'échéance ou la clôture de la période, la première des deux).
- **Dépôt** : fichiers (contrôlés par le socle 7 : type réel, taille, quarantaine, antivirus, conversion des photos en PDF), texte saisi, enregistrement audio, document de l'espace de travail (socle 32, figé au dépôt) ; chaque dépôt est **immuable**, horodaté par le serveur, avec **accusé** (numéro, heure, liste des fichiers, empreinte), lisible par l'élève et envoyé en avis.
- **Redéposer** avant l'échéance : nouveau dépôt, l'ancien reste (le professeur voit le dernier par défaut, l'historique s'il le veut) ; après l'échéance, un redépôt est « en retard ».
- **Brouillon** : l'élève peut préparer sa remise (fichiers en attente) sans la déposer ; seul le dépôt compte.
- **Remise de groupe** : le professeur compose les groupes (ou laisse les élèves s'inscrire dans des groupes de taille fixée) ; un membre dépose pour le groupe ; tous les membres voient le dépôt ; le retour est commun, avec un commentaire individuel possible.
- **Dépôt par le professeur au nom de l'élève** (copie papier scannée, remise à l'oral, clé USB) : marqué « déposé par M. X le … » ; le professeur peut aussi constater `rendu_papier` sans fichier.
- **Panne** : si le service est indisponible (incident déclaré au socle 1) dans les 2 heures avant l'échéance, l'échéance de tous les élèves concernés est prolongée automatiquement d'autant (R-C11), avec avis.
- **Problème de dépôt** (fichier refusé, réseau) : message clair, conseil (convertir, photographier), et bouton « je n'arrive pas à déposer » qui prévient le professeur avec l'heure (preuve de bonne foi).

### 10.2 Le suivi par le professeur [R]
- Tableau du travail : élèves du public, statut (attendu, rendu, en retard, rendu papier, non rendu, dispensé, excusé), heure du dernier dépôt, retour (à faire, brouillon, rendu) ; filtres ; **téléchargement groupé** (archive de toutes les copies, nommées « NOM Prénom – titre », créée par le socle 7, lien court) ; ouverture dans le navigateur.
- **Constater** : `non_rendu` à partir de la clôture du ramassage (proposé en lot, validé par le professeur), `dispense` (élève non concerné ce jour-là, sans motif public), `excuse` (absence justifiée, rattrapage à prévoir) ; ces constats sont des **faits** lus par le socle 21 (non rendu ≠ zéro, Q-EV1) et le socle 23 (signalement au bulletin, si l'établissement le fait).
- **Prolonger** pour un élève (aménagement, absence, problème technique) : échéance effective, avec motif interne **codé** (`amenagement`, `absence`, `technique`, `autre`), jamais affiché à l'élève autrement que « délai accordé jusqu'au… ».
- **Relancer** : message aux élèves (et, au choix, aux familles) des non-rendus par le socle 16, **un fil par élève et par foyer**, texte prérempli neutre, envoi confirmé (N2 du cœur IA si l'IA prépare).

### 10.3 Le retour [R]
- **Commentaire** (texte), **copie annotée** (annotation dans le navigateur sur le PDF converti : surlignage, texte, tampon de compétences ; ou dépôt d'un fichier annoté), **grille** facultative (critères du professeur, ou compétences du socle 22), **résultat** relié (socle 21 : la note est saisie au 21, jamais ici ; le retour affiche la note seulement si elle est publiée).
- États : `brouillon` (le professeur corrige au fil de l'eau), `rendu` (visible de l'élève et des responsables habilités ; avis regroupé « 5 copies rendues »). Rendre en lot : « rendre toutes les copies corrigées ».
- **Corriger un retour** : nouvelle version (la précédente reste), avis « retour mis à jour ».
- **Rouvrir une remise** (l'élève doit refaire) : état « à refaire » avec nouvelle échéance individuelle ; la première remise reste.
- **Contester** : l'élève ou la famille peut demander une explication par un fil du socle 16 relié au retour ; aucune contestation de note ici (socle 21, demande de révision).

### 10.4 Copies et vie privée [R]
- Une copie est une **œuvre de l'élève** (§5) : elle n'est ni publiée, ni réutilisée comme exemple, ni montrée à la classe sans son accord (et celui des responsables pour un mineur) ; « montrer cette copie comme exemple » demande un consentement explicite recueilli dans ScolIA (CDT-221), anonymisation proposée.
- **Contenu préoccupant** dans une copie (mal-être, violence, propos discriminatoires) : le professeur peut **signaler** à la direction ou à la CPE (socle 13 ou 36 selon la nature) ; la copie est alors lisible de la personne saisie, journalisée ; le professeur ne transmet jamais la copie par un autre canal.
- Conservation : §24.3 et Q-CDT3.

---

## 11. La charge de travail

### 11.1 Ce qui se calcule [R]
- Pour un élève et un jour : nombre d'**évaluations annoncées** (socle 21) et d'épreuves communes (38) ; nombre de travaux à rendre ; **durée estimée cumulée** des travaux dont l'échéance est ce jour-là ou le lendemain (le travail se fait la veille) ; pour une semaine : mêmes cumuls.
- Pour un cours (vue du professeur au moment de donner) : distribution sur **ses** élèves : « 18 élèves sur 29 ont déjà 1 évaluation annoncée jeudi, 4 en ont 2 » ; « durée cumulée médiane mercredi soir : 1 h 10 » ; natures des autres échéances (« 2 évaluations, 1 exposé ») **sans** matière ni nom quand l'élève n'est pas dans la même classe que le cours (groupe transversal), avec matière quand c'est la même classe (l'équipe pédagogique de la classe voit déjà son cahier).
- Pour une classe (PP, direction) : calendrier des évaluations et des travaux longs par jour, avec matières ; pour les élèves en groupes transversaux, la charge réelle (par élève) est agrégée.

### 11.2 Repères et alertes [R]
- Aucun plafond national (§5). Repères de l'établissement (S20-10), **trois positions** (souple, normal, strict) par cycle, valeurs par défaut [H] :

| Repère | Souple | Normal (défaut) | Strict |
|---|---|---|---|
| Évaluations annoncées le même jour pour un élève | 3 | 2 | 1 |
| Durée cumulée pour un soir (6ᵉ–5ᵉ / 4ᵉ–3ᵉ / lycée) | 60 / 90 / 150 min | 45 / 60 / 120 min | 30 / 45 / 90 min |
| Évaluations annoncées sur la semaine | 6 | 5 | 4 |

- L'alerte s'affiche **au moment où le professeur fixe l'échéance** (avec deux dates alternatives proposées, calculées) ; il peut passer outre en un geste, la dérogation est comptée (pas nominative pour la direction, sauf le professeur lui-même ; agrégée au PP par classe).
- Le PP et la direction voient les jours et semaines au-dessus des repères ; **pas** de blocage automatique : c'est une discussion d'équipe, pas un contrôle.
- Les variantes et échéances individuelles comptent pour l'élève concerné.

---

## 12. Continuité, absences, remplacements

### 12.1 L'élève absent [R]
- « **Ce que j'ai manqué** » : pour chaque séance où l'élève était absent (socle 12), l'entrée publiée et le travail donné, accessibles à l'élève et à ses responsables habilités, en tête de l'agenda au retour ; le professeur voit « 3 élèves absents à cette séance » sur son travail et peut, d'un geste, leur accorder un délai.
- **Absence longue** (hospitalisation, SAPAD, maladie) : le PP ou la CPE ouvre une **continuité** (période, cours concernés) ; les professeurs voient un bandeau « élève en continuité à distance » sur leur cours et peuvent déposer des travaux **pour lui seul** ; aucun motif n'est affiché (socle 34).
- **Exclusion de cours, exclusion temporaire** (socle 13) : la continuité des apprentissages est organisée (§5) ; le cahier fournit les travaux de la période ; le travail donné à un élève exclu est un travail du cahier (pas une punition) ; la punition « devoir supplémentaire » reste au socle 13.
- **CNED, double scolarité, sportif de haut niveau, élève à mi-temps en UPE2A** : public restreint et variantes, sans motif.

### 12.2 Le professeur absent [R]
- Absence connue d'avance (formation, absence autorisée, socle 15) : sur chaque séance touchée, le titulaire (ou la direction) dépose un **travail pour la classe en son absence** (« travail en autonomie », documents) ; il est publié la veille ou au début de la séance ; la permanence ou l'étude (socle 12) voit ce travail.
- **Remplacement d'une séance** (remplacement de courte durée, socle 15) : le remplaçant, désigné sur la séance (socle 10), voit le cahier du cours (toutes les entrées passées), la liste des élèves avec les **seuls** aménagements utiles (pastilles, socle 35), écrit **l'entrée de cette séance**, donne un travail dont l'échéance est une séance du titulaire (le titulaire est prévenu) ; il ne voit pas les copies ni les retours.
- **Suppléance** (socle 6, nature `suppleant`) : à partir de sa date de début (prise de poste, R-P3 du socle 6), le suppléant a le cahier du cours **en saisie** : entrées, travaux, remises en cours, retours ; il voit les progressions que le titulaire a **partagées** avec le cours (pas les autres) ; au retour du titulaire, la saisie revient à celui-ci ; les remises attendues continuent ; les retours en brouillon du suppléant restent signés par lui. Le titulaire en congé garde la lecture selon le paramétrage du socle 6 (Q-P5) et ne reçoit plus d'avis.
- **Poste vacant** : cours sans intervenant en saisie (socle 6, `ent_services_vacants`) : la direction peut publier un travail pour la classe ; bandeau « professeur en cours de nomination » ; pas de cahier vide signalé comme « non tenu ».

### 12.3 Fermeture et continuité pédagogique [R]
- Fermeture déclarée (socle 5, annulation de masse du socle 10) : les professeurs reçoivent **un** écran « vos séances annulées » avec trois choix par séance : reporter, déposer un travail à distance pour la séance (visible à l'heure prévue), lier une classe virtuelle (socle 33) ; la plage calme peut être levée par la direction ; l'agenda des élèves montre « cours à distance ».
- Le plan de continuité de l'établissement (document du socle 7) est épinglé dans l'agenda pendant la fermeture.

---

## 13. Adapter sans exposer (PAP, PPS, PAI, UPE2A, AESH)

### 13.1 Les faits utiles, jamais le motif [R]
- Le socle 35 fournit, pour un élève et un cours, des **faits d'aménagement utiles** : délai majoré (en %), version allégée, police agrandie, document audio, lecture de consigne, usage de l'ordinateur, « ne pas copier au tableau » ; le socle 20 les affiche au professeur comme **pastilles** sur la liste du cours, et les applique **par défaut** quand il le demande (« appliquer les aménagements » : échéance effective majorée, formats adaptés acceptés). Aucun diagnostic, aucun intitulé de plan (PAP, PPS) n'est montré à qui n'a pas le droit du socle 35.
- Le **PAI** (santé, socle 34) n'apporte ici qu'un fait éventuel (« ne pas manipuler tel produit » pour un TP) quand le socle 34 l'expose aux professeurs ; le socle 20 ne stocke rien de santé.

### 13.2 Variantes [R]
- Un travail peut avoir une ou plusieurs **variantes** (consigne, documents, durée, échéance, exercices) attribuées à des élèves nommés ; chaque élève voit **sa** version comme la seule ; un camarade ne voit pas qu'une variante existe.
- **Versions de lecture facilitée** : le professeur peut demander à l'IA une proposition (police, découpage en étapes, vocabulaire simplifié, FALC) qu'il relit et attribue (§15) ; la police adaptée et la lecture audio sont aussi des **réglages d'affichage de l'élève** (accessibilité, socle 11), sans intervention du professeur.

### 13.3 AESH [R]
- L'AESH voit, pour les **seuls élèves accompagnés à la date** (socle 35 et port `edt::seances_accompagnees` du socle 10), l'agenda, le cahier des cours concernés, les variantes de ces élèves, les documents ; elle peut ajouter une **note d'accompagnement** visible des professeurs du cours et du PP (« consigne reformulée en classe, exercice 2 fait à l'oral ») ; elle ne voit ni copies ni retours ni notes, sauf ouverture par le professeur pour un travail donné (S20-11, défaut : non).

---

## 14. Visa, inspection, archive

### 14.1 Visa [R]
- La direction (chef d'établissement, adjoint, droit `cahier.viser`) choisit une **période** et une **cible** (un cours, tous les cours d'une classe, tous les cours d'un professeur) ; elle voit le tableau de tenue (séances passées / renseignées, travaux donnés, dernières modifications), lit, et **vise** avec ou sans observation.
- Le visa enregistre la **référence figée** : pour chaque entrée et chaque travail de la période, l'identifiant et le numéro de version vus. Une modification postérieure apparaît « modifiée après visa » à la direction et à l'auteur.
- L'**observation** est adressée au professeur (ou aux intervenants du cours) ; elle n'est lisible que d'eux et de la direction ; le professeur peut y **répondre**. Le visa lui-même (date, viseur, période) est visible des intervenants du cours, jamais des familles.
- Tableau de la direction : cours non visés depuis N semaines, cours au taux de tenue faible (agrégat), sans classement nominatif public.

### 14.2 Inspection [R]
- La direction accorde un **accès d'inspection** : personne (compte invité du socle 1, ou compte académique fédéré du socle 9 quand il existe), professeur ou cours concernés, période du cahier, **durée** de l'accès (défaut 30 jours), motif (rendez-vous de carrière, inspection). Le professeur est **informé** (avis). L'inspecteur lit les entrées et les travaux (consignes, documents), **jamais** les remises, les retours ni la liste nominative des élèves ; chaque lecture est journalisée et le journal est visible du professeur et de la direction.
- Alternative : **export figé** (PDF du socle 7, entrées et travaux de la période, sans nom d'élève, empreinte), remis par la direction.

### 14.3 Archive annuelle [R]
- À la clôture de l'année (socle 5, `ControleCloture`), chaque cahier de cours est **figé** (plus aucune écriture), un **export d'archive** est produit (PDF/A par cours : entrées, travaux, visas ; sans remises ni retours ; empreinte, dépôt au socle 7 dans l'espace « archives pédagogiques ») et conservé **cinq ans** après cette année de lecture (R-06) ; à l'issue, le chef d'établissement choisit les cahiers versés aux archives de l'établissement, les autres sont détruits (CDT-245).
- Les élèves et familles gardent la lecture du cahier de l'année close **jusqu'à la fin de l'année scolaire suivante**, « dans les conditions des cahiers actifs » (R-06, S20-03), puis plus rien (l'élève garde ses remises et retours jusqu'à leur purge).
- Le professeur peut exporter **son** cahier de l'année (tous ses cours) pour son usage professionnel : entrées et travaux qu'il a signés, sans données d'élève.

---

## 15. Exercices à correction automatique

- Types : choix unique, choix multiple, vrai/faux, réponse courte (liste de réponses acceptées, casse et accents tolérés au choix), numérique (tolérance), association, ordonnancement, texte à trous. Pas de code exécutable, pas de média distant non contrôlé (images et sons passent par le socle 7).
- **(ajout personas, R-C31)** : les réponses attendues et les explications ne quittent jamais le serveur avant l'heure de correction.
- Réglages par exercice : nombre de tentatives, mélange, correction affichée (après chaque question, après la remise, après l'échéance, jamais), **temps limité** seulement si le professeur le veut (et jamais pour un élève ayant un délai majoré sans majoration du temps, appliquée seule).
- La correction est **déterministe** (pas d'IA) : un score **proposé** par tentative ; le professeur voit les résultats par question (taux de réussite, réponses fréquentes) et par élève ; verser un score comme **résultat** d'une évaluation passe par le socle 21 (EVA-265), avec sa validation ; **aucune note automatique**.
- Un exercice utilisé dans un travail publié est **figé** : le modifier crée une nouvelle version ; corriger un corrigé faux recalcule les scores **proposés** des tentatives existantes, sans toucher un résultat déjà versé (CE-82 du socle 21).
- **Banque personnelle** d'exercices du professeur, réutilisable ; partage facultatif avec les collègues de sa discipline dans l'établissement (licence implicite : usage interne ; auteur affiché).
- Pas de surveillance de l'élève pendant la passation (ni caméra, ni détection de changement d'onglet) : un exercice du cahier n'est pas un examen (§5, règlement sur l'IA annexe III, 3 d).

---

## 16. Progressions et ressources

- **Progression** : plan personnel (étapes ordonnées, titre, notions, durée en séances, ressources, travaux types) ; privée par défaut ; **partageable** avec un cours (les intervenants en saisie, dont un suppléant, la voient), avec la discipline (collègues de l'établissement), jamais avec les élèves.
- **Planifier** : associer les étapes aux séances à venir d'un cours (glisser, ou « répartir sur les séances jusqu'au 20 décembre ») ; les entrées sont créées en brouillon et se décalent si une séance tombe (proposition, §8.4).
- **Reconduire** : copier sa progression de l'an passé (sans dates) vers l'année nouvelle ; copier les **entrées** de son ancien cours vers son nouveau cours de même niveau, en brouillon, documents compris (nouveaux objets, pas de lien vers l'ancien cahier figé).
- **Ressources** : documents (socle 7, droits d'auteur : rappel au dépôt, §5), liens https, ressources numériques GAR (lien profond, socle 9), documents de l'espace de travail (socle 32) ; un document joint à une entrée est visible du **public du cours** seulement, jamais par lien public.

---

## 17. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md`)

### 17.1 Ce que l'IA propose au professeur [R]
| Capacité | Données lues | Sortie | Confirmation |
|---|---|---|---|
| Rédiger l'entrée de séance à partir de ses notes ou d'une photo du tableau | ses notes, sa photo (pas d'élève) | brouillon d'entrée | relecture, publication par lui |
| Proposer une **variante** de consigne (lecture facilitée, FALC, étapes, traduction pour un élève allophone) | la consigne, le niveau | texte proposé | relecture, attribution par lui (N2) |
| Proposer des exercices (QCM) sur une notion | la notion, le niveau, ses ressources | exercice en brouillon, **corrigé à vérifier** | relecture obligatoire de chaque question |
| Estimer la durée d'un travail | la consigne, le niveau | durée proposée | modifiable |
| Repérer une semaine chargée et proposer d'autres dates | charge agrégée (§11) | alerte et dates | choix par lui |
| Préparer un message neutre aux non-rendus | liste des non-rendus de **son** travail | brouillons, un par foyer | envoi confirmé (N3, socle 16) |
| Résumer les réponses fréquentes à un exercice | tentatives de **ses** élèves, pseudonymisées | synthèse | lecture |

### 17.2 Ce que l'IA ne fait jamais [R]
- Noter, classer, évaluer une copie, proposer une note ou une appréciation **sur une copie** (évaluation des résultats d'apprentissage : système à haut risque, règlement (UE) 2024/1689, annexe III, 3 b, §5) — sous réserve de l'étude juridique de D-032 ; même un « commentaire suggéré sur la copie » est hors du cœur tant que l'étude n'a pas conclu (Q-CDT7).
- « Détecter » qu'un devoir a été fait par une IA ou plagié pour fonder une décision ; au plus, le professeur peut comparer deux copies **de ses élèves** par un outil déterministe de similarité, comme indice (Q-CDT8, défaut : non livré en V1).
- Lire des données de santé, des aménagements avec motif, des copies d'élèves hors de la demande explicite du professeur.
- Agir pour l'élève : côté élève, l'IA est **désactivée par défaut** (IA_COEUR Q-IA4) ; si l'établissement l'ouvre, elle explique une consigne, aide à planifier sa semaine, ne fait jamais le travail.

---
## 18. Cas limites (catalogue)

Chaque cas devient un test (§27). « → » donne le comportement attendu.

### 18.1 Parcours de l'élève
| N° | Cas | Comportement |
|---|---|---|
| CC-01 | Élève arrivé le 4 novembre dans le cours | voit les travaux à échéance ≥ 4 novembre et tout le cahier passé du cours ; les travaux antérieurs n'apparaissent pas « en retard » |
| CC-02 | Élève parti le 10 janvier | plus aucune lecture du cours après le 10 ; garde ses remises et retours ; plus aucun avis ; ne peut plus déposer |
| CC-03 | Changement de groupe de LV2 le 2 décembre | agenda : anciens travaux du premier groupe jusqu'au 1er, nouveaux ensuite ; remise faite dans l'ancien groupe conservée et lisible de lui |
| CC-04 | Travail donné le lundi, élève entré dans le cours le mercredi, échéance vendredi | il le voit, avec « donné avant votre arrivée » ; le professeur est prévenu (proposition de délai) |
| CC-05 | Élève inclus ULIS dans un cours de la classe ordinaire pour 2 h/semaine | voit seulement les travaux du cours où il est inclus (`ent_cours_membre`), à la date |
| CC-06 | Élève exclu d'un cours (exclusion `ent_cours_membre`) | ne voit plus ce cours à partir de la date |
| CC-07 | Élève changé d'établissement en cours d'année (ScolIA → ScolIA) | aucun transfert des remises (cloisonnement) ; export personnel de ses remises proposé à l'élève avant le départ |
| CC-08 | Élève décédé (socle 2) | gel : plus aucun avis, plus d'échéance, retrait de toutes les listes de suivi, données conservées selon la règle du socle 2 |
| CC-09 | Élève devenu majeur le 15 mars | à partir du 15, la lecture par les responsables suit le choix de l'élève (socle 4) ; rien de rétroactif |
| CC-10 | Fusion de deux fiches élève (socle 2) | remises, coches, tentatives transférées à la fiche gardée, en un lot journalisé |

### 18.2 Séances et calendrier
| N° | Cas | Comportement |
|---|---|---|
| CC-11 | Régénération des séances après ajout d'un jour férié | aucune entrée ni travail perdu ; séance `retiree` ; proposition de report |
| CC-12 | Séance annulée la veille avec travail attendu | proposition de report ; appliquée seule à l'heure de la séance si non traitée ; un avis |
| CC-13 | Séance avancée de 2 jours | l'échéance-séance n'est **pas** avancée sans accord explicite du professeur |
| CC-14 | Échéance-date au milieu des vacances | refusée, séance de rentrée proposée |
| CC-15 | Cours fermé en cours d'année (option abandonnée par tous) | cahier figé à la fermeture, lisible ; travaux restants signalés au professeur |
| CC-16 | Séance commune à deux cours (fusion de deux groupes) | une entrée par cours, copie proposée, un travail peut viser les deux cours |
| CC-17 | Établissement à La Réunion ; échéance « 18 h » | heure locale de l'établissement, partout (affichage, retard, avis) |
| CC-18 | Changement d'heure la nuit de l'échéance | calcul en heure murale locale, sans décalage d'une heure |
| CC-19 | Annulation en masse (grève) de 40 séances d'un professeur | une proposition groupée, un écran, un avis par élève et par jour |
| CC-20 | Séance neutralisée par une sortie à laquelle seuls 12 élèves sur 28 participent | la séance a lieu pour les 16 autres (socle 10) ; les 12 en sortie ne sont pas en retard pour une échéance ce jour-là |

### 18.3 Intervenants
| N° | Cas | Comportement |
|---|---|---|
| CC-21 | Suppléante du 12 janvier au 3 avril | saisie du 12 au 3 ; lecture de tout l'historique ; retours en brouillon du titulaire visibles d'elle, signés par lui |
| CC-22 | Titulaire revient le 20 mars (retour anticipé) | saisie rendue le 20 ; les brouillons de la suppléante restent à elle, lisibles du titulaire |
| CC-23 | Remplaçant d'une seule séance | écrit l'entrée de **cette** séance, rien d'autre ; aucune lecture de copie ; plus aucun droit le lendemain |
| CC-24 | Co-enseignement (deux professeurs en saisie) | chacun signe ses contributions ; un travail a un auteur ; le suivi des remises est partagé |
| CC-25 | Ancien professeur de la classe (service clos en décembre) | lecture 7 jours après la fin (délai de grâce du socle 6), puis rien ; aucune écriture après la fin |
| CC-26 | Intervenant extérieur sans compte | la contribution est saisie par le titulaire au nom de « intervenant : M. X » (texte affiché), sans compte |
| CC-27 | Professeur parent d'un élève de son cours | agit comme professeur dans son cours (socle 6 Q-P3) ; ses avis de parent suivent sa casquette parent ; pas de double avis |
| CC-28 | Professeur suspendu (socle 6, P9) | plus aucun accès ; ses entrées restent signées ; la direction ou le suppléant poursuit |
| CC-29 | Poste vacant pendant 6 semaines | la direction publie le travail ; le cahier n'est pas compté « non tenu » |
| CC-30 | Professeur sur trois établissements | un cahier par établissement, aucune lecture croisée ; « copier vers » ses propres cours seulement dans le même établissement |

### 18.4 Rendu et retour
| N° | Cas | Comportement |
|---|---|---|
| CC-31 | Dépôt à 17 h 59 min 58 s pour une échéance à 18 h, réception complète à 18 h 00 min 04 s | l'heure retenue est le **début** de la réception par le serveur ; à l'heure (R-C10) |
| CC-32 | Fichier refusé (format, virus, quarantaine) | message clair ; aucun dépôt ; l'élève peut signaler « je n'arrive pas » (trace horodatée) |
| CC-33 | Panne du service 1 h avant l'échéance | prolongation automatique d'autant pour tous, avis (R-C11) |
| CC-34 | Élève qui redépose après le retour du professeur | refusé sauf remise rouverte « à refaire » |
| CC-35 | Remise de groupe : un membre quitte l'établissement | la remise reste au groupe ; l'élève parti garde sa lecture ; le retour reste visible des autres |
| CC-36 | Deux membres d'un groupe déposent en même temps | deux dépôts horodatés, ordonnés ; le dernier est « courant » |
| CC-37 | Retour porteur d'une note non publiée | la note n'apparaît pas ; commentaire et copie visibles seulement si le professeur le choisit (« rendre la copie sans la note ») |
| CC-38 | Copie annotée qui contient le nom d'un autre élève | l'annotation est sur la copie de l'élève seulement ; avertissement au professeur s'il écrit un prénom d'un autre élève du cours (contrôle doux) |
| CC-39 | Travail « à rendre en classe » et élève absent ce jour-là | statut `attendu` jusqu'au constat ; proposition « excusé » si absence justifiée (socle 12) |
| CC-40 | Travail rendu, puis le professeur change l'échéance plus tard | la remise reste « à l'heure » |
| CC-41 | Travail rendu en retard, puis prolongation individuelle accordée | statut recalculé « à l'heure » ; historique gardé |
| CC-42 | Élève sans équipement numérique | dépôt par le professeur au nom de l'élève ; « rendu papier » ; aucune pénalité liée au canal |
| CC-43 | Photo de copie de 12 Mo depuis le téléphone | compressée et convertie en PDF par le socle 7 avant contrôle de taille ; aperçu avant dépôt |
| CC-44 | Fichier bureautique avec macro | refusé ou désarmé (socle 7) ; message |
| CC-45 | Lien externe dans une consigne vers un site douteux | https seulement ; avertissement « vous quittez ScolIA » ; liste de blocage de l'établissement |

### 18.5 Familles
| N° | Cas | Comportement |
|---|---|---|
| CC-46 | Parents séparés, les deux habilités | même lecture, avis selon les préférences de chacun ; aucune donnée de l'autre foyer |
| CC-47 | Parent dont le lien est suspendu par décision de justice | rien ; aucun avis ; aucun accès par un lien déjà reçu (jetons invalidés) |
| CC-48 | Parent sans exercice de l'autorité parentale, droit d'information ouvert par le socle 4 | lit le cahier et l'agenda ; pas les copies ni les retours (S20-08) |
| CC-49 | Tiers (grand-parent, famille d'accueil) avec droit de suivi | selon les droits de son lien ; jamais plus qu'un responsable |
| CC-50 | Élève majeur qui ferme la lecture à ses parents | plus rien pour eux à partir de la date ; l'élève voit l'état de son choix |
| CC-51 | Responsable de deux enfants dans le même cours | deux agendas, un avis regroupé |

### 18.6 Charge, publication, plage calme
| N° | Cas | Comportement |
|---|---|---|
| CC-52 | Professeur qui publie à 22 h 30 un mardi | visible et avis à 7 h mercredi ; l'échéance n'est pas avant 7 h + délai minimal (S20-04) |
| CC-53 | Travail publié le dimanche pour le lundi 8 h | signalé « donné moins de 24 h avant » ; visible lundi 7 h ; alerte au professeur avant publication |
| CC-54 | Groupe de spécialité de 4 classes, contrôle jeudi | charge calculée sur les 30 élèves réels ; nombres et natures, pas de matière hors de leurs classes |
| CC-55 | Évaluation annoncée au socle 21 **et** travail « réviser » | une seule évaluation comptée dans la charge |
| CC-56 | Brevet blanc (épreuves communes, socle 38) | compté dans la charge de chaque élève concerné ; alerte aux professeurs qui ajoutent une évaluation ces jours-là |
| CC-57 | Professeur qui passe outre l'alerte | autorisé, compté, sans blâme ; le PP voit la journée au-dessus du repère |

### 18.7 Visa, inspection, fin d'année
| N° | Cas | Comportement |
|---|---|---|
| CC-58 | Professeur modifie une entrée après le visa | « modifiée après visa » pour la direction et pour lui ; version visée lisible |
| CC-59 | Inspecteur dont l'accès a expiré | 403 ; prolongation par la direction seulement |
| CC-60 | Clôture d'année avec copies non rendues | contrôle de clôture : liste au professeur ; clôture possible, copies non rendues signalées, l'élève garde ses dépôts |
| CC-61 | Demande de communication du cahier d'une classe par une famille ou un juge | export daté par la direction, journalisé (secrétariat sur instruction) ; sans copies ni retours d'autres élèves |
| CC-62 | Demande d'accès RGPD d'un élève ou d'un responsable | export de ses remises, retours, tentatives, coches, demandes de délai (§24.3) |
| CC-63 | Année rouverte après clôture (socle 5, annulation de bascule) | cahier dégelé avec trace ; archive précédente gardée, nouvelle archive à la nouvelle clôture |

### 18.8 Exercices
| N° | Cas | Comportement |
|---|---|---|
| CC-64 | Corrigé faux découvert après 25 tentatives | nouvelle version ; scores proposés recalculés ; résultats déjà versés au socle 21 inchangés, proposition de mise à jour au professeur |
| CC-65 | Élève avec temps majoré sur un exercice chronométré | temps majoré appliqué seul, sans affichage du motif |
| CC-66 | Coupure réseau pendant une tentative | réponses enregistrées au fil de l'eau ; reprise de la tentative ; le chronomètre tient compte de la coupure (dans la limite d'un plafond) |
| CC-67 | Élève qui ouvre l'exercice sur deux appareils | une seule tentative ouverte, verrou, reprise sur le second |

---

## 19. Droits et minimisation

### 19.1 Catalogue d'actions proposé (D-017) [R]

| Action | Sens | Profils par défaut | Portée par défaut |
|---|---|---|---|
| `cahier.lire` | lire entrées et travaux publiés d'un cours | professeur, CPE, AED, AESH, documentaliste, direction, administratif | professeur : ses cours **et** les cours des classes dont il fait partie de l'équipe pédagogique ; CPE/AED : établissement (sans remises ni retours) ; AESH : élèves accompagnés ; direction : établissement |
| `cahier.ecrire` | écrire entrées et travaux | professeur | intervention **en saisie** à la date (socle 6, `droits ∋ cahier`) ; remplaçant : la séance |
| `cahier.contribuer` | ajouter sa contribution signée à une séance | intervenant, assistant, AESH (note d'accompagnement) | son intervention, ses élèves accompagnés |
| `cahier.suivre_remises` | lire les remises, constater, prolonger, rendre les retours | professeur | intervention en saisie à la date |
| `cahier.viser` | viser, observer | direction | établissement |
| `cahier.retirer` | retirer une entrée ou un travail de la diffusion (contenu inapproprié) | direction | établissement ; motif ; avis à l'auteur |
| `cahier.acces_inspection` ★ | accorder un accès d'inspection | direction | établissement |
| `cahier.exporter` ★ | export d'un cahier (litige, archive, inspection) | direction ; secrétariat sur délégation | établissement |
| `cahier.lire_copie_motivee` ★ | lire une remise hors portée (contestation, signalement) | direction | une remise, motif, journal, avis au professeur |
| `cahier.parametrer` | réglages du socle | direction | établissement |
| `cahier.charge_classe` | calendrier de charge d'une classe avec matières | PP, direction, CPE | sa classe ; établissement |
| (élève) | lire son agenda et ses cours, déposer, cocher, demander un délai | élève | lui-même, public à la date |
| (responsable) | lire l'agenda, le cahier, les statuts, les retours sans note ou à note publiée | responsable | droits du lien (socle 4) |

### 19.2 Ce que voit chacun [R]

| Donnée | Élève | Responsable | Professeur du cours (saisie) | Autre professeur de la classe | CPE / AED | AESH | Direction | Inspecteur |
|---|---|---|---|---|---|---|---|---|
| Entrées publiées | ses cours | droit du lien | oui | oui | oui | élèves accompagnés | oui | accès borné |
| Brouillons | non | non | oui | non | non | non | non | non |
| Travaux publiés, consignes | ses cours (sa variante) | idem élève | oui (toutes variantes) | oui (version commune) | oui (version commune) | variantes des accompagnés | oui | consignes, sans variantes nominatives |
| Remise, dépôts | les siens | statut ; **copie annotée rendue** (ajout personas, R-C37), jamais les dépôts avant le retour | oui | **non** | **non** | non (sauf ouverture) | sur motif ★ | non |
| Retour | le sien | si sans note ou note publiée + droit | oui | non | non | non | sur motif ★ | non |
| Statuts constatés | le sien | celui de l'enfant | oui | non | « non rendus » agrégés de l'élève (suivi) [R, Q-CDT6] | non | agrégés | non |
| Coche « fait » | la sienne | **non** | **non** | non | non | non | non | non |
| « Vu » (**supprimé**, ajout personas R-C42) | non | non | **non** | non | non | non | non | non |
| Prolongations, motif codé | « délai jusqu'au » | idem | oui | non | non | non | oui | non |
| Charge | sa semaine | celle de l'enfant | nombres et natures sur ses élèves | idem pour ses cours | classe | — | oui | non |
| Visa, observation | non | non | visa et observation de son cours | visa seul | non | non | oui | non |
| Journal des consultations d'inspection | non | non | oui (son cahier) | non | non | non | oui | non |

### 19.3 Garde-fous [R]
- Toute lecture et écriture passe par le moteur (D-017) **et** une portée calculée à la date de l'action par les fonctions du socle 6 ; aucune portée « mes classes » codée dans le socle (C-D2).
- `TenantTx` sur toutes les requêtes (RLS effective), clés composites, aucune lecture par identifiant fourni sans contrôle d'appartenance au cours et à l'établissement (IDOR).
- Contenu riche : balisage maison, **aucun HTML** stocké ni rendu brut ; liens https, attribut `rel="noopener noreferrer"`, avertissement de sortie ; noms de fichiers assainis ; export CSV protégé contre l'injection de formule.
- Fichiers : uniquement par le service du socle 7, liens signés courts liés à la session, usage unique (07 §17.3) ; jamais de lien public, jamais d'aperçu en ligne d'un format actif.
- Dates : jour et heure de l'établissement (fuseau du site, 265), heure serveur pour le dépôt.
- Avis : contenu minimal hors de l'application (« Nouveau travail en mathématiques pour jeudi », jamais la consigne d'un élève à variante, jamais un statut « non rendu » sur l'écran verrouillé).

### 19.4 Pièges de minimisation identifiés [R]
1. La **charge** révèle l'emploi du temps des autres groupes : n'afficher que nombres et natures hors de la classe (§11.1).
2. La **variante** révèle un aménagement : invisible des autres élèves, jamais nommée « PAP » à l'écran (R-C8).
3. La **liste des non-rendus** révèle une difficulté : visible du professeur du cours et du PP (agrégée), pas des autres professeurs.
4. Le **« vu »** est une surveillance : jamais affiché, purgé à la fin de l'année.
5. La **coche « fait »** appartient à l'élève : jamais vue d'un adulte.
6. Le **nom de compte** de l'auteur : remplacé par le nom d'affichage de l'intervenant (C-D15).
7. L'**inspection** n'a pas besoin des élèves : jamais de nom, jamais de copie.
8. Un **export** de cahier pour un litige : sans données d'autres élèves.

---
## 20. Actions métier (CDT-100 et suivantes)

CDT-01 à CDT-10 sont pris par `SOCLE2_DOMAINES.md` (réalisés en S2-6) ; ils restent valables et sont repris ci-dessous. Acteurs : PRF professeur (intervention en saisie), REM remplaçant d'une séance, SUP suppléant, PP professeur principal, DIR direction, ADF secrétariat, CPE, AED, AESH, DOC documentaliste, INS inspecteur, ELV élève, PAR responsable, SYS système. ★ = action sensible (récapitulatif, confirmation ; ré-authentification par passkey quand elle sera branchée).

### 20.1 Paramétrer (direction)
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-100 | Choisir le moment de publication par défaut (début de séance, fin de séance, immédiat) | DIR · `cahier.parametrer` | S20-01 | `cahier.reglages_modifies` |
| CDT-101 | Fixer le repère de renseignement (soir même, 48 h, fin de semaine) | DIR · idem | S20-02 ; aucun effet bloquant | idem |
| CDT-102 | Choisir les repères de charge (souple / normal / strict) par cycle | DIR · idem | S20-10 ; aperçu sur les 4 dernières semaines | idem |
| CDT-103 | Ajuster la plage calme de l'établissement (dans les bornes nationales) | DIR · idem | socle 8 ; jamais plus courte que 20 h–7 h sans motif tracé | idem |
| CDT-104 | Fixer la durée de lecture du cahier de l'année close par les familles | DIR · idem | S20-03 ; défaut : fin de l'année scolaire suivante (R-06), jamais moins | idem |
| CDT-105 | Ouvrir ou non la lecture des copies rendues aux responsables | DIR · idem | S20-08 ; défaut fermé | idem |
| CDT-106 | Fixer la date de clôture du ramassage par défaut | DIR · idem | S20-09 | idem |
| CDT-107 | Fixer les formats de rendu proposés et la taille maximale | DIR · idem | dans les bornes du socle 7 | idem |
| CDT-108 | Définir la liste des natures de travail (préréglage de 9) | DIR · idem | données de l'établissement, archivables | idem |
| CDT-109 | Choisir l'heure du rappel « à rendre demain » | DIR · idem | défaut 17 h ; hors plage calme | idem |
| CDT-110 | Reconduire les réglages de l'année précédente | DIR · idem | appelé par le chantier de rentrée du socle 5 | `cahier.reglages_reconduits` |

### 20.2 Préparer (professeur)
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-111 | Créer une progression personnelle | PRF | privée ; indépendante du cours | — |
| CDT-112 | Copier sa progression de l'année précédente | PRF | sans dates | — |
| CDT-113 | Partager une progression avec un cours ou avec sa discipline | PRF | jamais avec des élèves | `cahier.progression_partagee` |
| CDT-114 | Répartir les étapes d'une progression sur les séances d'un cours | PRF · `cahier.ecrire` | entrées créées en brouillon | `cahier.entrees_planifiees` |
| CDT-115 | Copier les entrées de son ancien cours vers un nouveau cours | PRF · idem | brouillons ; documents recopiés en nouveaux objets | idem |
| CDT-116 | Préparer la semaine suivante en une vue | PRF · idem | brouillons ou programmées | — |
| CDT-117 | Programmer la publication d'une entrée ou d'un travail | PRF · idem | `publie_a` ; plage calme | `cahier.programme` |
| CDT-118 | Dupliquer une séance sur ses autres cours de même niveau | PRF · idem | une entrée par cours ; ajustable | `cahier.seance_renseignee` |
| CDT-119 | Gérer sa banque d'exercices | PRF | privée ou partagée à la discipline | — |
| CDT-120 | Gérer ses ressources favorites (liens, documents, GAR) | PRF | — | — |

### 20.3 Renseigner une séance
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-01 / CDT-121 | Renseigner le contenu d'une séance (depuis la séance) | PRF, SUP, REM · `cahier.ecrire` | intervention en saisie à la date de la séance ; remplaçant : sa séance | `cahier.seance_renseignee` [F] |
| CDT-122 | Renseigner depuis le téléphone à la fin du cours | idem | socle 11 ; hors ligne : brouillon local, envoi au retour du réseau | idem |
| CDT-123 | Joindre une photo du tableau | idem | socle 7 (conversion, réduction) ; avertissement « aucun visage d'élève » | `cahier.piece_jointe` [F] |
| CDT-04 / CDT-124 | Joindre des documents, liens, ressources GAR | idem | §16 | idem |
| CDT-125 | Ajouter sa contribution signée (co-intervenant, assistant) | intervenant · `cahier.contribuer` | sa contribution seulement | `cahier.contribution_ajoutee` |
| CDT-126 | Modifier une entrée publiée | auteur, intervenants en saisie · `cahier.ecrire` | version ; « modifiée après visa » si visée | `cahier.seance_modifiee` |
| CDT-127 | Retirer une entrée de la diffusion | auteur ; DIR · `cahier.retirer` | motif ; versions gardées ; avis à l'auteur si DIR | `cahier.retiree` |
| CDT-128 | Voir l'historique d'une entrée | auteur, intervenants en saisie, DIR | ajout seul | — |
| CDT-129 | Lier une classe virtuelle à une séance à distance | PRF · idem | socle 33 | — |
| CDT-130 | Écrire le contenu d'une séance passée oubliée | PRF · idem | jusqu'à la clôture de l'année ; marquée « renseignée le … » | idem 121 |

### 20.4 Donner le travail
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-02 / CDT-131 | Donner un travail à faire depuis la séance | PRF, SUP, REM · `cahier.ecrire` | échéance-séance proposée ; durée ; nature | `cahier.travail_donne` [F] |
| CDT-05 / CDT-132 | Donner un travail à rendre en ligne | idem | formats, retard, clôture du ramassage | `cahier.travail_a_rendre` [F] |
| CDT-133 | Donner un travail pour plusieurs de ses cours en un geste | PRF · idem | un travail par cours, reliés | idem |
| CDT-134 | Restreindre le public (rattrapage, groupe de besoin, volontaires) | PRF · idem | élèves du cours seulement | idem |
| CDT-135 | Créer des variantes adaptées et les attribuer | PRF · idem | sans motif ; aménagements proposés (socle 35) | `cahier.variante_attribuee` |
| CDT-136 | Appliquer les aménagements connus à un travail | PRF · idem | délais majorés et formats ; faits du socle 35 | idem |
| CDT-137 | Organiser une remise par groupes (composés ou au choix des élèves) | PRF · idem | taille fixée ; un élève dans un seul groupe | `cahier.groupes_composes` |
| CDT-138 | Joindre des exercices à correction automatique | PRF · idem | exercice figé à la publication | — |
| CDT-139 | Indiquer « ce travail sera noté » (crée une évaluation en brouillon au socle 21) | PRF · idem + `evaluations.saisir` | EVA-114 | `cahier.travail_note` |
| CDT-140 | Voir la charge avant de fixer l'échéance et choisir une date proposée | PRF · idem | §11 ; alerte non bloquante | — |
| CDT-141 | Passer outre une alerte de charge | PRF · idem | comptée, non nominative pour la direction | `cahier.alerte_charge_ignoree` |
| CDT-03 / CDT-142 | Modifier un travail publié (avec ou sans avis) | auteur, intervenants en saisie | échéance avancée : confirmation + avis toujours | `cahier.travail_modifie` [F] |
| CDT-143 | Retirer un travail (sans remise) ou le retirer de la diffusion (avec remises) | idem ; DIR · `cahier.retirer` | motif ; remises gardées à l'élève | `cahier.travail_retire` |
| CDT-144 | Répondre publiquement à une question sur un travail (précision ajoutée à la consigne) | PRF · idem | version ; avis « précision ajoutée » | `cahier.travail_precise` |
| CDT-145 | Laisser un travail pour la classe en son absence | PRF ; DIR | séances touchées par une absence du socle 15 | `cahier.travail_absence_depose` |
| CDT-146 | Donner un travail pendant une fermeture (continuité) | PRF ; DIR | séance annulée ; visible à l'heure prévue | idem 131 |

### 20.5 Reports et changements de séance
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-147 | Recevoir les propositions de report après un changement de séance | SYS → PRF | §8.4 ; une proposition par changement | `cahier.report_propose` |
| CDT-148 | Accepter, modifier ou refuser une proposition | PRF · `cahier.ecrire` | refus : l'échéance reste, motif facultatif | `cahier.report_applique` |
| CDT-149 | Traiter les propositions en lot (grève, fermeture) | PRF ; DIR | un écran | idem |
| CDT-150 | Appliquer seule une proposition non traitée à l'heure de la séance | SYS | S20-05 ; jamais une échéance avancée | idem |

### 20.6 Consulter (professeur, équipe, vie scolaire)
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-151 | Voir « mon cahier » : semaine, séances, entrées, travaux, remises à corriger | PRF | ses interventions en saisie | — |
| CDT-152 | Voir le cahier d'un cours de sa classe (équipe pédagogique) | PRF · `cahier.lire` | sans remises ni retours | — |
| CDT-153 | Voir le calendrier de charge de sa classe | PP · `cahier.charge_classe` | matières visibles dans la classe | — |
| CDT-154 | Voir l'agenda d'un élève (aide aux devoirs, étude, internat) | CPE, AED · `cahier.lire` | consignes et échéances, sans remises ni retours ; lecture journalisée | — |
| CDT-155 | Voir l'agenda et les variantes des élèves accompagnés | AESH | élèves accompagnés à la date | — |
| CDT-156 | Ajouter une note d'accompagnement | AESH · `cahier.contribuer` | visible des professeurs du cours et du PP | `cahier.note_accompagnement` |
| CDT-157 | Voir les travaux laissés en son absence pour la permanence | AED, CPE | séances non assurées du jour | — |
| CDT-158 | Voir les élèves qui ne rendent rien depuis N semaines (faits) | PP, CPE | agrégé par élève, sans détail des matières hors classe | — |
| CDT-159 | Voir « pourquoi je peux écrire dans ce cours » | PRF | `explique_acces` du socle 6 | — |

### 20.7 Élève
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-08 / CDT-160 | Voir son agenda (travaux, évaluations annoncées, épreuves, sorties) | ELV | public à la date ; sa variante | — |
| CDT-161 | Voir le cahier de chacun de ses cours | ELV | entrées publiées | — |
| CDT-162 | Voir « ce que j'ai manqué » | ELV | absences du socle 12 | — |
| CDT-09 / CDT-163 | Cocher « fait » | ELV | personnel | — |
| CDT-06 / CDT-164 | Déposer une remise (fichier, texte, audio, document de l'espace) | ELV | public et échéance effective à la date ; accusé | `cahier.travail_rendu` [F] |
| CDT-165 | Préparer sa remise en brouillon | ELV | non déposée tant qu'il ne l'a pas fait | — |
| CDT-166 | Redéposer avant l'échéance | ELV | nouveau dépôt ; ancien gardé | `cahier.redepot` |
| CDT-167 | Signaler « je n'arrive pas à déposer » | ELV | trace horodatée, avis au professeur | `cahier.depot_impossible_signale` |
| CDT-168 | Déposer pour son groupe | ELV | membre du groupe | `cahier.travail_rendu` |
| CDT-169 | S'inscrire dans un groupe (si le professeur l'a ouvert) | ELV | taille ; date limite | — |
| CDT-170 | Demander un délai (motif libre visible du seul professeur) | ELV ; PAR | une demande par travail | `cahier.delai_demande` |
| CDT-171 | Voir son retour et sa copie annotée | ELV | règles de publication du 21 | — |
| CDT-172 | Poser une question au professeur sur un travail | ELV | fil du socle 16 relié | — |
| CDT-173 | Passer un exercice à correction automatique | ELV | tentatives, temps majoré appliqué seul | `cahier.tentative_terminee` |
| CDT-174 | Exporter ses remises et retours (départ, fin d'année) | ELV | archive personnelle | — |
| CDT-175 | Régler ses avis « devoirs » et l'affichage (police, lecture audio) | ELV | socles 8 et 11 | — |
| CDT-176 | S'abonner à son agenda dans son calendrier | ELV | jeton révocable, sans consigne détaillée (socle 9) | — |

### 20.8 Responsable
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-177 | Voir l'agenda et le cahier de son enfant | PAR | droit du lien | — |
| CDT-178 | Voir les statuts constatés (rendu, non rendu, en retard) | PAR | idem | — |
| CDT-179 | Voir le retour (sans note, ou note publiée avec « voir les notes ») | PAR | idem | — |
| CDT-180 | Recevoir le résumé hebdomadaire « la semaine de mon enfant » | PAR | socle 8, catégorie « Notes et devoirs » | — |
| CDT-181 | Demander un délai pour son enfant | PAR | idem CDT-170 | idem |
| CDT-182 | Écrire au professeur au sujet d'un travail | PAR | socle 16 | — |
| CDT-183 | Voir la semaine de plusieurs enfants en une vue | PAR | un regroupement | — |

### 20.9 Suivre et rendre les copies (professeur)
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-184 | Voir le tableau de suivi d'un travail | PRF · `cahier.suivre_remises` | public du travail | — |
| CDT-185 | Télécharger toutes les copies en une archive | PRF · idem | socle 7 ; lien court ; noms normalisés | `cahier.copies_telechargees` |
| CDT-186 | Ouvrir et annoter une copie dans le navigateur | PRF · idem | annotations = nouvel objet | — |
| CDT-07 / CDT-187 | Rédiger et rendre un retour (commentaire, copie annotée, grille) | PRF · idem | brouillon puis rendu | `cahier.copie_rendue` [F] |
| CDT-188 | Rendre toutes les copies corrigées en un geste | PRF · idem | avis regroupé | idem |
| CDT-189 | Rendre la copie sans la note | PRF · idem | note publiée plus tard au 21 | idem |
| CDT-190 | Corriger un retour déjà rendu | PRF · idem | nouvelle version, avis | `cahier.retour_modifie` |
| CDT-191 | Rouvrir une remise (« à refaire ») avec nouvelle échéance | PRF · idem | première remise gardée | `cahier.remise_rouverte` |
| CDT-192 | Déposer au nom d'un élève (copie papier scannée) | PRF · idem | marqué « déposé par » | `cahier.travail_rendu` |
| CDT-193 | Constater `rendu_papier`, `non_rendu`, `dispense`, `excuse` (en lot) | PRF · idem | faits lus par 21 et 23 | `cahier.statut_constate` |
| CDT-194 | Accorder un délai à un élève (ou à tous les absents d'une séance) | PRF · idem | motif codé interne | `cahier.delai_accorde` |
| CDT-195 | Répondre à une demande de délai | PRF · idem | acceptée, refusée (motif) | `cahier.delai_decide` |
| CDT-196 | Relancer les non-rendus (élèves et, au choix, familles) | PRF · idem | un fil par foyer (socle 16), confirmation | — |
| CDT-197 | Verser le score proposé d'un exercice comme résultat d'une évaluation | PRF · `evaluations.saisir` | EVA-265 | `evaluations.resultats_saisis` |
| CDT-198 | Voir les statistiques d'un exercice (par question, réponses fréquentes) | PRF · idem | sans export nominatif hors du cours | — |
| CDT-199 | Signaler un contenu préoccupant dans une copie | PRF | socle 13 ou 36 ; la personne saisie lit la copie, journal | `cahier.copie_signalee` |
| CDT-200 | Demander le consentement pour montrer une copie en exemple | PRF | élève et responsables (mineur) ; anonymisation | `cahier.consentement_demande` |
| CDT-201 | Fixer la clôture du ramassage d'un travail | PRF · idem | défaut S20-09 | — |

### 20.10 Remplacements, suppléances, continuité
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-202 | Prendre la main sur le cahier du cours à la prise de poste (suppléant) | SUP | automatique à la date (socle 6) ; écran d'accueil « où en était le cours » | — |
| CDT-203 | Voir le cahier du cours et les aménagements utiles avant une séance de remplacement | REM | séance désignée ; pastilles seulement | — |
| CDT-204 | Rendre la main au titulaire (fin de suppléance) | SYS | à la date ; brouillons signés gardés | `cahier.saisie_rendue` |
| CDT-205 | Ouvrir une continuité pour un élève absent longtemps | PP, CPE · `cahier.lire` + `continuite.ouvrir` | période, cours ; aucun motif | `cahier.continuite_ouverte` |
| CDT-206 | Déposer un travail pour un élève en continuité | PRF · `cahier.ecrire` | public restreint | `cahier.travail_donne` |
| CDT-207 | Clore une continuité | PP, CPE | date | `cahier.continuite_close` |
| CDT-208 | Fournir le travail d'un élève exclu temporairement (socle 13) | PRF ; DIR | période d'exclusion ; demande envoyée aux professeurs par le socle 13 | idem 206 |
| CDT-209 | Publier un travail sur un cours au poste vacant | DIR · `cahier.ecrire` (direction) | cours sans intervenant en saisie | idem 131 |

### 20.11 Direction, visa, inspection, archive
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-210 | Voir le tableau de tenue des cahiers (taux, dernières entrées) | DIR | agrégats ; pas de classement public | — |
| CDT-10 / CDT-211 | Viser une période d'un cours, d'une classe ou d'un professeur | DIR · `cahier.viser` | référence figée des versions | `cahier.vise` [F] |
| CDT-212 | Ajouter une observation au visa, adressée aux intervenants | DIR · idem | lisible d'eux et de la direction | `cahier.observation` |
| CDT-213 | Répondre à une observation | PRF | idem | `cahier.observation_reponse` |
| CDT-214 | Voir les modifications faites après visa | DIR, auteur | — | — |
| CDT-215 | Accorder, prolonger, révoquer un accès d'inspection ★ | DIR · `cahier.acces_inspection` | durée, cours, période ; professeur informé | `cahier.acces_inspection_*` |
| CDT-216 | Consulter un cahier (inspecteur) | INS | accès borné ; journal | `cahier.consulte_inspection` |
| CDT-217 | Produire un export figé d'un cahier ★ | DIR · `cahier.exporter` ; ADF par délégation | sans données d'autres élèves pour un litige ; empreinte | `export.realise` |
| CDT-218 | Lire une remise hors portée sur motif (contestation) ★ | DIR · `cahier.lire_copie_motivee` | motif, journal, avis au professeur | `cahier.copie_lue_motif` |
| CDT-219 | Retirer un contenu inapproprié de la diffusion | DIR · `cahier.retirer` | motif ; avis à l'auteur | `cahier.retiree` |
| CDT-220 | Lever la plage calme pour une diffusion de continuité ★ | DIR | tracé ; durée | `cahier.plage_calme_levee` |
| CDT-221 | Voir la charge de l'établissement par niveau et par semaine | DIR | agrégats | — |
| CDT-222 | Clôturer les cahiers de l'année (figer, archiver) | DIR ; SYS à la clôture du socle 5 | contrôle de clôture | `cahier.annee_figee` |
| CDT-223 | Exporter son propre cahier de l'année | PRF | ses entrées et travaux ; sans élèves | — |

### 20.12 Système, conservation, échanges
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-224 | Publier les entrées et travaux programmés à l'heure dite (hors plage calme) | SYS | tâche idempotente | `cahier.publie` |
| CDT-225 | Envoyer le rappel « à rendre demain » | SYS | heure S20-09 ; seulement aux élèves sans dépôt ; regroupé | — |
| CDT-226 | Proposer les constats `non_rendu` à la clôture du ramassage | SYS → PRF | proposition, jamais appliquée seule | — |
| CDT-227 | Prolonger les échéances après une panne déclarée | SYS | R-C11 | `cahier.prolongation_panne` |
| CDT-228 | Purger les « vu » à la clôture de l'année | SYS | — | — |
| CDT-229 | Appliquer les durées de conservation (remises, retours, tentatives, archives) | SYS | §24.3 ; journal ; archives : jamais avant la décision CDT-245 | `cahier.purge` |
| CDT-230 | Répondre à une demande d'accès RGPD (export par élève) | DIR, ADF | §24.3 | `export.realise` |
| CDT-231 | Reprendre l'ancien cahier (049, 150) dans le nouveau modèle | SYS | §25 ; rejouable | — |
| CDT-232 | Lire l'agenda d'un élève en mode hors ligne (mobile) | ELV, PAR | dernière synchronisation affichée | — |
| CDT-233 | Fournir à l'appel (socle 12) le « contenu du jour » et le travail attendu | SYS | port `cahier::seance` | — |
| CDT-234 | Fournir au conseil (socle 23) les non-rendus de la période | SYS | port `cahier::non_rendus` ; faits | — |

### 20.13 IA (propositions, toutes confirmées)
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-235 | Proposer l'entrée de séance à partir de notes ou d'une photo du tableau | PRF | §17 ; brouillon | `ia.proposition` |
| CDT-236 | Proposer une variante de lecture facilitée ou traduite | PRF | relue ; attribuée par lui | idem |
| CDT-237 | Proposer un exercice sur une notion (corrigé à vérifier) | PRF | chaque question relue | idem |
| CDT-238 | Estimer la durée d'un travail | PRF | modifiable | idem |
| CDT-239 | Proposer d'autres dates en cas de surcharge | PRF | §11 | idem |
| CDT-240 | Préparer les messages aux non-rendus | PRF | N3, socle 16 | idem |
| CDT-241 | Résumer les erreurs fréquentes d'un exercice | PRF | pseudonymisé | idem |
| CDT-242 | (élève, si ouvert) Expliquer une consigne, aider à planifier sa semaine | ELV | désactivé par défaut ; jamais avant la 4ᵉ, accès anonymisé type GAR (R-15) ; jamais faire le travail | idem |

### 20.14 Ajouts issus de la vérification réglementaire (§5)
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-243 | Joindre au cahier le **sujet** d'un devoir ou d'un contrôle après l'épreuve | PRF · `cahier.ecrire` | R-04 ; proposé par le socle 21 à la publication des notes | `cahier.piece_jointe` |
| CDT-244 | Indiquer l'**usage de l'IA autorisé** pour un travail | PRF · idem | R-15 ; défaut « non » ; affiché à l'élève | `cahier.travail_modifie` |
| CDT-245 | Choisir les cahiers archivés versés aux archives de l'établissement, à l'issue des cinq ans | DIR ★ | R-06 ; les autres sont détruits, procès-verbal | `cahier.archive_versee` / `_detruite` |
| CDT-246 | Afficher au professeur qui publie pendant la plage calme le rappel du droit à la déconnexion et l'heure de visibilité | SYS | R-14 | — |

---

## 21. Règles transverses (R-C1 et suivantes) [R]

| N° | Règle |
|---|---|
| R-C1 | Une entrée ou un travail vise un **cours** (socle 5) ; plus aucune écriture nouvelle par classe ou groupe texte. |
| R-C2 | Le droit d'écrire se calcule **à la date de l'action** à partir de l'intervention en saisie (socle 6) et de ses droits ; le remplaçant d'une séance n'écrit que **cette** séance. |
| R-C3 | Une séance qui porte une entrée, un travail donné ou attendu, une remise n'est **jamais supprimée** (R-E2 du socle 10) ; clés `RESTRICT`. |
| R-C4 | Un changement de séance produit une **proposition** de report ; jamais une échéance avancée sans accord explicite ; un seul avis par changement effectif. |
| R-C5 | Rien n'est visible des élèves et familles avant `visible_a` ; la plage calme décale la visibilité et les avis, jamais la saisie. |
| R-C6 | L'élève voit ce qui concerne les cours dont il est membre **à la date de l'échéance** (ou de la séance) ; il garde toujours ses propres remises et retours. |
| R-C7 | Le responsable voit selon les droits de son lien (socle 4), à la date ; un lien suspendu ne voit rien ; un retour porteur de note suit la publication du socle 21 et le droit « voir les notes ». |
| R-C8 | Une variante, une prolongation, un aménagement ne sont visibles que de l'élève concerné, de ses responsables habilités, des intervenants en saisie du cours, de son AESH et de la direction ; **aucun motif** n'est affiché ni stocké en clair (motif codé interne seulement). |
| R-C9 | Un dépôt est **immuable** ; un redépôt est un nouveau dépôt ; les fichiers sont des objets du socle 7 avec empreinte. |
| R-C10 | L'heure d'un dépôt est l'heure serveur du **début** de réception ; « en retard » se calcule sur l'échéance effective de l'élève, en heure de l'établissement. |
| R-C11 | Une panne déclarée dans les 2 h avant une échéance prolonge automatiquement cette échéance de la durée de la panne (minimum 1 h). |
| R-C12 | Un travail non rendu est un **fait** (`non_rendu`), jamais une note ; le socle 21 décide de son effet (Q-EV1 : aucun par défaut). |
| R-C13 | Une copie n'est lue que de son auteur (ou des membres du groupe), de ses responsables habilités (S20-08), des intervenants en saisie du cours ; la direction sur motif journalisé ; personne d'autre. |
| R-C14 | La coche « fait » et le « vu » ne sont jamais montrés à un adulte ; le « vu » est purgé à la clôture. |
| R-C15 | La charge affichée à un professeur porte sur **ses élèves** ; hors de la classe du cours, nombres et natures seulement. |
| R-C16 | Aucune alerte de charge ne bloque ; chaque dérogation est comptée sans être nominative pour la direction. |
| R-C17 | Le visa fige la référence des versions vues ; une modification postérieure est marquée. |
| R-C18 | L'inspection ne lit jamais une remise, un retour, ni une liste nominative d'élèves ; chaque lecture est journalisée et visible du professeur. |
| R-C19 | À la clôture de l'année, le cahier est figé (aucune écriture) et archivé avec empreinte ; seule une réouverture d'année (socle 5) le dégèle, avec trace. |
| R-C20 | Aucune note n'est produite par le socle 20 ; un score d'exercice est une **proposition** que seul le professeur verse au socle 21. |
| R-C21 | L'IA ne note pas, n'évalue pas une copie, ne détecte pas la triche, ne lit aucune donnée de santé ; tout ce qu'elle produit est un brouillon confirmé par une personne. |
| R-C22 | Le contenu riche est un balisage maison ; aucun HTML stocké ni rendu brut ; liens https seulement. |
| R-C23 | Toutes les tables portent `etablissement_id`, des clés composites, la RLS ; toutes les requêtes passent par `TenantTx` ; aucun JSON qui sert de mémoire (D-036). |
| R-C24 | Une modification d'un travail publié est versionnée ; l'avis n'est envoyé que pour un changement utile, et toujours pour une échéance avancée. |
| R-C25 | Le professeur ne peut ajouter au public d'un travail qu'un élève du cours à la date. |
| R-C26 | Aucune donnée nominative d'élève dans une entrée, une consigne ou un document du cours (contrôle doux : prénoms de la liste du cours détectés, avertissement à l'auteur). |
| R-C27 | Tout travail affiche l'usage de l'IA autorisé (défaut « non ») ; l'IA générative n'est jamais proposée à un élève avant la 4ᵉ (R-15). |
| R-C28 | Le cahier d'une année close reste lisible comme un cahier actif pendant l'année scolaire suivante, puis il est archivé cinq ans ; aucune purge d'archive sans la décision du chef d'établissement (R-06). |

---

## 22. Événements et avis

### 22.1 Émis (outbox, dans la transaction du fait)
Repris de S2-6 [F] : `cahier.seance_renseignee`, `travail_donne`, `travail_a_rendre`, `travail_modifie`, `travail_supprime` (devient `travail_retire`), `piece_jointe`, `travail_rendu`, `copie_rendue`, `vise`. Nouveaux : `cahier.publie`, `programme`, `seance_modifiee`, `retiree`, `contribution_ajoutee`, `variante_attribuee`, `groupes_composes`, `travail_note`, `travail_precise`, `travail_absence_depose`, `alerte_charge_ignoree`, `report_propose`, `report_applique`, `redepot`, `depot_impossible_signale`, `delai_demande`, `delai_accorde`, `delai_decide`, `statut_constate`, `retour_modifie`, `remise_rouverte`, `copie_signalee`, `copie_lue_motif`, `consentement_demande`, `tentative_terminee`, `continuite_ouverte`, `continuite_close`, `saisie_rendue`, `observation`, `observation_reponse`, `acces_inspection_accorde` / `_revoque`, `consulte_inspection`, `plage_calme_levee`, `annee_figee`, `prolongation_panne`, `purge`, `reglages_modifies`, `reglages_reconduits`, `progression_partagee`, `entrees_planifiees`. Charge utile : identifiants seulement (jamais de consigne ni de nom), version.

### 22.2 Consommés
`edt.seance_annulee`, `_deplacee`, `_neutralisee`, `_retiree`, `_non_assuree`, `edt.annulation_masse` (10) → propositions de report ; `referentiel.cours_membre_*`, `affectation_*` (5) → public ; `services.intervention_*`, `service.suppleance_ouverte` / `_close` (6) → saisie ; `absences.*` (12) → « ce que j'ai manqué », excuses proposées ; `discipline.exclusion_temporaire` (13) → demande de travaux ; `accompagnement.*` (35) → pastilles et aménagements ; `liens.*`, `eleve.majeur` (4, 2) → lecture ; `eleve.parti`, `eleve.decede`, `eleve.fusionne` (2) → gel et transfert ; `evaluations.annoncee`, `_publiee` (21) → agenda, retours ; `annee.cloturee`, `annee.rouverte` (5) → figer, dégeler ; `plateforme.incident_declare` (1) → R-C11.

### 22.3 Avis (socle 8, catégorie « Notes et devoirs »)
| Avis | Destinataires | Urgence | Regroupement | Plage calme |
|---|---|---|---|---|
| Nouveau travail | élèves du public ; responsables abonnés | U0 | résumé quotidien par défaut pour les familles | oui |
| Travail modifié (changement utile), échéance avancée | idem | U0 ; U1 si avancée | non pour une échéance avancée | oui |
| Rappel « à rendre demain » | élèves sans dépôt | U0 | un par élève | heure S20-09 |
| Accusé de dépôt | élève (et membres du groupe) | U0 | non | non (réponse à son action) |
| Copie rendue, retour modifié | élève ; responsables habilités | U0 | « 3 copies rendues » | oui |
| Copie reçue en retard, « je n'arrive pas à déposer », demande de délai | professeur | U0 | résumé | oui |
| Report appliqué | élèves, responsables | U0 | un par changement | oui |
| Observation de visa | intervenants du cours | U0 | non | oui |
| Accès d'inspection accordé | professeur concerné | U0 | non | oui |

---
## 23. Écrans clés (Leptos, `FRONTEND_LEPTOS.md`, `DESIGN_SYSTEM_LEPTOS.md`)

1. **Mon cahier** (professeur, web et mobile) : semaine de ses séances (composant « séance » du socle 10), pour chacune : pastille d'état (vide, brouillon, programmée, publiée), travail donné, travail attendu, remises à corriger ; saisie en place ; « préparer la semaine suivante » ; propositions de report en tête. Base : `prof.rs`.
2. **Donner un travail** (panneau latéral depuis la séance) : consigne, nature, durée, échéance (prochaines séances proposées), **bandeau de charge** (nombres sur ses élèves, deux dates alternatives), rendu, variantes, aménagements, « sera noté », publication.
3. **Cahier du cours** (équipe, direction, inspection) : chronologie des entrées et travaux, filtres, versions, visas ; impression et export.
4. **Suivi et correction** (professeur) : tableau du travail, téléchargement groupé, visionneuse avec annotation, retour, constats en lot, délais. Base : `copies.rs`.
5. **Agenda** (élève, famille ; mobile d'abord, 375 px) : aujourd'hui, demain, semaine ; « ce que j'ai manqué » ; déposer (photo → PDF, aperçu, accusé) ; coche « fait » ; famille : enfants côte à côte, statuts. Base : `agenda.rs`.
6. **Charge de la classe** (PP, direction) : calendrier mensuel des évaluations et travaux longs, jours au-dessus des repères.
7. **Tenue et visa** (direction) : tableau par cours, période, visa en un écran avec la référence figée.
8. **Accès d'inspection** (direction) et **journal de consultation** (professeur).
9. **Exercice** (élève) : passation accessible au clavier, temps restant, reprise ; (professeur) éditeur et statistiques.
10. **Réglages** (direction) : une page, onze réglages, aperçu des effets.

Critères communs : clavier complet, lecteur d'écran, contraste, 375 px, axe-core en CI, aucun style en ligne (CSP), textes courts ; saisie d'une entrée en ≤ 3 gestes depuis la séance du jour.

---

## 24. Modèle de données cible (brique `cahier`)

### 24.1 Tables [R]
Toutes : `etablissement_id UUID NOT NULL`, clé primaire UUID applicative, `UNIQUE (etablissement_id, id)`, clés étrangères **composites** `(etablissement_id, x_id)`, `cree_le` / `modifie_le`, **RLS forcée** dès la création (`ent_rls_poser`), `ON DELETE RESTRICT` partout (aucune cascade), classement au registre (E12), **aucune colonne JSON** (D-036). Les « ajout seul » sont protégés par déclencheur (pas d'`UPDATE` ni de `DELETE`).

```
ent_cdt_reglage                    -- un par (établissement, année)
  etablissement_id, annee_id, publication_defaut ('debut_seance','fin_seance','immediate'),
  repere_renseignement ('soir','48h','semaine'), repere_charge ('souple','normal','strict'),
  lecture_annee_close_jusqu_au DATE (défaut : fin de l'année suivante), copies_lisibles_responsables BOOLEAN DEFAULT false,
  cloture_ramassage_jours SMALLINT DEFAULT 14, rappel_veille_heure TIME DEFAULT '17:00',
  echeance_plage_calme_lycee BOOLEAN DEFAULT false, taille_max_mo SMALLINT, version, modifie_le, modifie_par
ent_cdt_reglage_format             reglage → formats acceptés proposés (mime_famille TEXT)
ent_cdt_nature                     id, code, libelle, duree_defaut_min, ordre, archivee_le

ent_cdt_entree                     -- succède à ent_seance_contenu (150)
  id, etablissement_id, cours_id, seance_id (FK RESTRICT vers ent_edt_seance), annee_id,
  titre, contenu TEXT (balisage maison), etat ('brouillon','programmee','publiee','retiree'),
  publie_a TIMESTAMPTZ NULL, visible_a TIMESTAMPTZ NULL, retire_motif NULL, retire_par NULL,
  auteur_agent_id, version INT, renseignee_le, progression_etape_id NULL, ancien_id NULL (reprise),
  UNIQUE (etablissement_id, cours_id, seance_id)
ent_cdt_entree_version             -- ajout seul : entree_id, version, titre, contenu, etat, auteur_agent_id, le, motif NULL
ent_cdt_entree_notion              entree_id, ordre, libelle
ent_cdt_contribution               id, entree_id, auteur_agent_id | libelle_externe, contenu, version, retiree_le
ent_cdt_contribution_version       -- ajout seul
ent_cdt_note_accompagnement        id, entree_id | travail_id, eleve_id, agent_aesh_id, texte, cree_le

ent_cdt_travail                    -- succède à ent_travail (150)
  id, etablissement_id, cours_id, annee_id, seance_donnee_id NULL, donne_le DATE,
  echeance_mode ('seance','date_heure'), echeance_seance_id NULL, echeance_le TIMESTAMPTZ (calculée, heure locale → instant),
  nature_id, titre, consigne TEXT, duree_estimee_min SMALLINT NULL,
  rendu ('aucun','en_ligne','en_classe'), retard_accepte BOOLEAN, cloture_ramassage_le TIMESTAMPTZ NULL,
  remise_par_groupe BOOLEAN, inscription_groupes_jusqu_au NULL, taille_groupe SMALLINT NULL,
  public ('cours','restreint'), evaluation_id NULL (socle 21), ia_autorisee ('non','recherche','relecture','libre') DEFAULT 'non', relie_id NULL (même travail sur plusieurs cours),
  etat ('brouillon','programme','publie','retire'), publie_a, visible_a, retire_motif, retire_par,
  auteur_agent_id, version, ancien_id NULL, ancien_type NULL ('controle' repris)
ent_cdt_travail_version            -- ajout seul
ent_cdt_travail_format             travail_id, mime_famille
ent_cdt_travail_cible              travail_id, eleve_id                       -- public restreint
ent_cdt_variante                   id, travail_id, libelle_interne, consigne, duree_estimee_min NULL, echeance_le NULL, version
ent_cdt_variante_eleve             variante_id, eleve_id, attribuee_par, attribuee_le, retiree_le NULL
ent_cdt_echeance_eleve             -- prolongation individuelle ; ajout seul (une nouvelle ligne remplace la précédente)
  id, travail_id, eleve_id, echeance_le, motif_code ('amenagement','absence','technique','panne','autre'), par, le
ent_cdt_delai_demande              id, travail_id, eleve_id, demandeur_personne_id, message, etat ('ouverte','acceptee','refusee'),
                                   reponse NULL, decide_par NULL, decide_le NULL, echeance_eleve_id NULL
ent_cdt_groupe                     id, travail_id, libelle
ent_cdt_groupe_membre              groupe_id, eleve_id, depuis, jusqu_au NULL   UNIQUE (travail_id, eleve_id) via déclencheur

ent_cdt_piece                      -- reprise de 150 ; plus de storage_key
  id, etablissement_id, objet_id (FK ent_objet, 500), porteur ('entree','contribution','travail','variante','depot','retour'),
  porteur_id, nom, ordre, retiree_le NULL, retiree_par NULL
ent_cdt_lien                       id, porteur, porteur_id, url (https), libelle, ressource_gar_id NULL, ordre, retire_le NULL

ent_cdt_remise                     -- succède à ent_travail_remise
  id, etablissement_id, travail_id, eleve_id NULL | groupe_id NULL (CHECK : un seul),
  etat ('brouillon','deposee','a_refaire'), dernier_depot_id NULL, en_retard BOOLEAN (dérivé), version
ent_cdt_depot                      -- ajout seul
  id, remise_id, numero, recu_le TIMESTAMPTZ (début de réception), par_user_id, au_nom_de BOOLEAN,
  texte NULL, empreinte CHAR(64), nb_fichiers, commentaire_eleve NULL
ent_cdt_depot_signalement          id, travail_id, eleve_id, le, message           -- « je n'arrive pas à déposer »
ent_cdt_statut                     -- constat du professeur ; ajout seul (la dernière ligne fait foi)
  id, travail_id, eleve_id, statut ('rendu_papier','non_rendu','dispense','excuse','attendu'), par, le
ent_cdt_coche                      travail_id, eleve_id, fait_le                   -- outil de l'élève, lu par lui seul
ent_cdt_vu                         travail_id, eleve_id, premier_le                -- technique, purgé à la clôture
ent_cdt_retour                     id, remise_id, etat ('brouillon','rendu'), commentaire, resultat_id NULL (21),
                                   montrer_note BOOLEAN, rendu_le NULL, auteur_agent_id, version
ent_cdt_retour_version             -- ajout seul
ent_cdt_retour_critere             retour_id, ordre, libelle, niveau ('non_atteint','partiel','atteint','depasse') | competence_item_id (22)
ent_cdt_signalement_copie          id, remise_id, vers ('cpe','direction','protection'), dossier_ref UUID NULL, par, le
ent_cdt_consentement_exemple       id, remise_id, demande_le, eleve_reponse, responsable_reponse, anonymise BOOLEAN, etat

ent_cdt_exercice                   id, etablissement_id, auteur_agent_id, titre, parent_id NULL, version, fige_le NULL,
                                   tentatives_max, melange BOOLEAN, correction ('question','remise','echeance','jamais'),
                                   duree_limite_min NULL, partage ('prive','discipline'), matiere_id NULL
ent_cdt_question                   id, exercice_id, ordre, type ('choix_unique','choix_multiple','vrai_faux','courte','numerique',
                                   'association','ordre','trous'), enonce, points NUMERIC(5,2), explication NULL, objet_id NULL
ent_cdt_choix                      id, question_id, ordre, libelle, correct BOOLEAN, apparie_a NULL
ent_cdt_reponse_attendue           id, question_id, valeur, tolerance NUMERIC NULL, casse BOOLEAN, accents BOOLEAN
ent_cdt_travail_exercice           travail_id, exercice_id, ordre                  -- exercice figé à la publication
ent_cdt_tentative                  id, exercice_id, travail_id, eleve_id, numero, commencee_le, terminee_le NULL,
                                   duree_accordee_min NULL, score_propose NUMERIC NULL, bareme NUMERIC, version_corrige INT
ent_cdt_tentative_reponse          tentative_id, question_id, choix_id NULL, valeur NULL, ordre NULL, correcte BOOLEAN NULL, points NULL, le

ent_cdt_progression                id, etablissement_id, auteur_agent_id, annee_id NULL, titre, matiere_id NULL, niveau_id NULL,
                                   copie_de_id NULL, archivee_le NULL
ent_cdt_progression_etape          id, progression_id, ordre, titre, notions TEXT, duree_seances SMALLINT
ent_cdt_progression_partage        progression_id, cours_id NULL | discipline (matiere_id) NULL, depuis, jusqu_au NULL

ent_cdt_report                     -- propositions de report
  id, origine_seance_id, evenement_id, objet ('entree','travail'), objet_id, vers_seance_id NULL,
  etat ('proposee','appliquee','refusee','appliquee_auto'), traite_par NULL, traite_le NULL
ent_cdt_continuite                 id, eleve_id, du, au NULL, ouverte_par, close_par NULL            -- sans motif
ent_cdt_continuite_cours           continuite_id, cours_id

ent_cdt_visa                       -- reprise de 150, élargie
  id, cible ('cours','classe','agent'), cours_id | classe_id | agent_id, du, au, avec_observation BOOLEAN,
  vise_par, vise_le
ent_cdt_visa_element               -- référence figée ; ajout seul
  visa_id, objet ('entree','travail'), objet_id, version
ent_cdt_observation                id, visa_id, texte, par, le ; ent_cdt_observation_reponse id, observation_id, texte, par, le
ent_cdt_acces_inspection           id, beneficiaire_user_id, agent_cible_id NULL, du_cahier, au_cahier, valable_du, valable_au,
                                   motif ('rdv_carriere','inspection','autre'), accorde_par, revoque_le NULL
ent_cdt_acces_inspection_cours     acces_id, cours_id
ent_cdt_consultation               -- ajout seul : lectures d'inspection, lectures de copie sur motif, lectures de l'agenda par la vie scolaire
  id, qui_user_id, nature ('inspection','copie_motif','agenda_vie_scolaire'), objet_id, motif NULL, le
ent_cdt_alerte_charge              travail_id, repere, valeur, seuil, ignoree BOOLEAN, le        -- sans auteur exposé
ent_cdt_archive                    id, annee_id, cours_id, objet_id (ent_objet), empreinte, produite_le, version,
                                   echeance_archive DATE, decision ('a_decider','versee','detruite') , decide_par NULL, decide_le NULL
```

### 24.2 Ce qui n'est pas stocké [R]
- **La charge** : calculée (index sur `(etablissement_id, cours_id, echeance_le)` et sur le public du jour) ; seules les alertes ignorées sont gardées, pour le pilotage.
- **Les motifs d'aménagement** : jamais ; seulement le code de prolongation.
- **Le contenu des fichiers** : jamais en base (E5) ; `ent_objet` porte taille, empreinte, type.
- **Les avis envoyés** : au socle 8.

### 24.3 Conservation [R, à valider par le DPO, Q-CDT3]
| Donnée | Durée proposée | Après |
|---|---|---|
| Cahier du cours (entrées, travaux, versions, visas) | **lisible comme cahier actif jusqu'à la fin de l'année scolaire suivante**, puis **archive cinq ans** (R-06 [V], circulaire 2010-136) | versement aux archives de l'établissement au choix du chef d'établissement (CDT-245), sinon destruction avec procès-verbal |
| Remises, dépôts, fichiers des élèves, retours, tentatives | **fin de l'année scolaire suivante** (contestations d'orientation et de notes closes) | effacement ; l'élève a pu exporter (CDT-174) |
| Remise **reliée à un résultat** contesté ou à un signalement | jusqu'à la fin de la procédure | effacement |
| Coches « fait », « vu » | clôture de l'année | effacement |
| Demandes de délai, signalements de dépôt, statuts | fin de l'année scolaire suivante | effacement |
| Journal des consultations (inspection, copies sur motif) | 1 an après l'accès (journalisation RGPD) | effacement |
| Progressions, banque d'exercices | tant que l'auteur est dans l'établissement, + 1 an | proposées à l'export à son départ, puis effacées |
| Alertes de charge ignorées | 2 ans | agrégées puis effacées |

Export par personne (demande d'accès) : un élève reçoit ses remises, dépôts, retours, tentatives, coches, demandes ; un responsable, ce qui concerne l'enfant selon ses droits ; un professeur, ses entrées, travaux, progressions, exercices.

### 24.4 Registre (E12) et neutralisation (E9) [R]
Domaine `cahier` ; portée `etablissement` pour toutes ; export `simple` pour le cahier, `double` (double validation, D-041) pour remises et retours (données d'élèves) ; aucune file d'effets de bord propre (les avis passent par l'outbox et le socle 8, déjà déclarés) ; la tâche de publication programmée est **idempotente** et déclarée « à neutraliser à la restauration » (ne publie pas en double, ne renvoie pas d'avis anciens).

---

## 25. Plan de migration depuis l'existant (sans casser l'agenda, l'appel ni le mobile)

| Étape | Contenu | Garde-fou |
|---|---|---|
| **M0** | Lot C0 (correctifs urgents, §26) sur les tables actuelles | tests de non-régression S2-6 (21 + 3 + 11) verts ; tests rouges des défauts verts |
| **M1** | Nouvelles tables (2401–2411) à côté des anciennes ; **vues de compatibilité** `ent_travail`, `ent_seance_contenu` lisant les nouvelles tables pour les lecteurs non encore basculés | aucune écriture dans les anciennes tables après bascule de l'écriture |
| **M2** | **Reprise** (2412) : `ent_seance_contenu` → `ent_cdt_entree` (cours déduit de la séance : `ent_edt_seance.service_id` → `ent_service_enseignement.cours_id`, sinon créneau → cours, sinon **quarantaine** listée pour la direction) ; `ent_travail` → `ent_cdt_travail` (classe ou groupe → cours de la séance ou cours unique de la classe dans la matière ; sinon quarantaine) ; `type = 'controle'` → nature `revision` + `ancien_type = 'controle'` + proposition au professeur de créer l'évaluation annoncée au socle 21 ; remises → remises + un dépôt n° 1 ; pièces → `ent_objet` (empreinte calculée en tâche de fond, lot du socle 7) ; `ent_cdt_historique` → versions (instantané relu une fois, puis gardé figé) ; visas → visas sans référence figée (« visa antérieur au 2027 ») ; `ent_travail_eleve` → coche et vu ; `ent_cahier_texte` (049) → déjà dans `ent_travail` par `ancien_id` | reprise rejouable (`ancien_id` unique), compteurs avant/après par établissement, quarantaine vide ou décidée avant M3 |
| **M3** | Bascule des écritures (nouvelles commandes) ; ancienne route `/cahier-texte` en lecture seule filtrée | — |
| **M4** | Bascule des lecteurs (agenda, appel, fiche élève, notifications, mobile C11) | grep des lectures des anciennes tables à zéro |
| **M5** | RLS stricte (2418) | tests sous rôle non propriétaire |
| **M6** | Retrait des anciennes tables et de la route `/cahier-texte` (2419) après la durée d'archive | décision, sauvegarde vérifiée, note dans `MIGRATIONS.md` (E8) |

---

## 26. Lots de code (équipe Sonnet)

### 26.1 Tableau des lots

| Lot | Contenu | Dépend de | Critères de fin | Taille |
|---|---|---|---|---|
| **C0** correctifs urgents | C-D1 (avec T0 du socle 10 : plus de suppression de séance porteuse, `RESTRICT` sur `ent_seance_contenu`, `ent_travail.seance_id`, pièces, remises, coches) ; C-D2 (portée par `ent_prof_sur_cours(…, p_saisie)` à la date, `droits ∋ 'cahier'`, remplaçant borné à sa séance, titulaire sans saisie pendant une suppléance) ; C-D3 (remise absente de l'agenda hors élève, responsable habilité, intervenant en saisie) ; C-D4 (`droits_lien` : suivre la scolarité ; voir les notes pour un retour à note) ; C-D5 (public à la date ; élève parti : plus de lecture du cours, plus de dépôt, plus d'avis) ; C-D7 (ancienne route : lecture filtrée, écriture fermée hors mobile, redirigée vers la commande) ; C-D8 (contrôle du `note_id` : même cours, même auteur ; historique du retour) ; C-D9 (visa lisible du visé et de la direction) ; C-D10 (la direction retire, ne modifie plus) ; C-D11 (`TenantTx` partout) ; C-D12 (retrait logique de pièce, verrou du plafond) ; C-D13 (jour et heure de l'établissement) ; C-D15 (nom d'affichage au lieu de l'identifiant de compte) | rien (peut partir tout de suite ; coordonné avec T0 du socle 10) | tests rouges avant, verts après : régénération après ajout d'un jour férié → contenu et travail conservés ; remplaçant d'une séance → 403 sur les copies le lendemain ; ancien professeur → 403 en écriture ; suppléant → écrit dès sa date ; CPE → aucune remise dans l'agenda ; parent sans droit de suivi → 403 ; parent suspendu → 403 ; élève parti → 403 en dépôt ; La Réunion 23 h UTC → bonne date ; requête sans contexte d'établissement → 0 ligne ; aucune régression S2-6 | M |
| **C1** modèle et reprise | tables §24 (hors exercices, progressions, inspection), reprise M2 avec quarantaine, vues de compatibilité, versions en ajout seul, RLS, registre | C0 ; socle 5 S4 (cours) ; socle 6 P3–P4 (fait) | compteurs de reprise ; déclencheurs d'immuabilité testés ; quarantaine affichée ; `grep -i jsonb` vide | L |
| **C2** saisie et publication | entrées par cours et séance, brouillons, programmation, plage calme, contributions, travail (nature, durée, échéance-séance, public restreint, plusieurs cours), modification avec ou sans avis, retrait, copie de séance, saisie rétroactive ; écrans 1 et 2 | C1 | parcours « saisir en 3 gestes » ; plage calme testée (22 h → 7 h) ; échéance avancée = avis | L |
| **C3** agenda, cahier élève et famille, avis | agenda (travaux, évaluations annoncées par le port du 21 en lecture, épreuves, sorties), « ce que j'ai manqué », coche, résumé hebdomadaire, avis par `AvisSurEvenement` (socle 8, N11-edt-cahier), abonnement calendrier | C2 ; socle 8 N0 | matrice de séparation famille (séparés, suspendu, sans exercice, majeur) ; 375 px ; avis minimaux | M |
| **C4** charge | calcul par élève, bandeau au moment de donner, dates proposées, repères à trois positions, calendrier de classe, alertes ignorées comptées | C2 ; port `evaluations::calendrier` | groupe de 4 classes : nombres justes, aucune matière hors classe ; performance < 200 ms pour 35 élèves | M |
| **C5** rendu et retour | remise, dépôts immuables, accusé, brouillon, redépôt, groupes, dépôt au nom de, signalement « je n'arrive pas », retard sur échéance effective, prolongations, demandes de délai, clôture du ramassage, constats, retour (commentaire, annotation, grille), rendre en lot, rouvrir, relance des non-rendus, téléchargement groupé, panne (R-C11) ; écran 4 | C2 ; socle 7 (dépôt, conversion, antivirus D0) | CC-31 à CC-45 en tests ; aucune copie lisible d'un tiers | L |
| **C6** continuité | propositions de report (§8.4) et traitement en lot, travail en absence du professeur, remplaçant, prise de main du suppléant, poste vacant, continuité d'un élève, exclusion temporaire, fermeture | C2 ; socle 10 (événements de séance) ; socle 15 | CC-11 à CC-30 en tests | M |
| **C7** adaptations | variantes, aménagements appliqués (port 35 en simulation tant qu'il n'existe pas), AESH (lecture, note d'accompagnement), réglages d'affichage de l'élève | C5 ; socle 35 | aucun motif visible ; un camarade ne détecte pas une variante (test de réponse identique) | M |
| **C8** visa, inspection, archive | tableau de tenue, visa avec référence figée, observations et réponses, modifications après visa, accès d'inspection et journal, exports figés, clôture et archive annuelle, export du professeur ; écrans 3, 7, 8 | C2 ; socle 7 (figer, PDF/A) ; socle 5 (clôture) | inspecteur : aucune donnée d'élève ; archive à empreinte stable | M |
| **C9** progressions et reconduction | progressions, partage, planification sur les séances, copie d'année en année, copie d'un cours à l'autre | C2 | — | S |
| **C10** exercices | éditeur, types, banque, passation (reprise, temps majoré), correction déterministe, statistiques, versement au 21 | C5 ; socle 21 (EVA-265) | CC-64 à CC-67 ; aucun résultat sans validation | L |
| **C11** mobile | API et écrans de l'appli unique (socle 11) : saisie de fin de cours, photo du tableau, agenda, dépôt photo → PDF, hors ligne en lecture ; bascule des 4 prototypes de `/cahier-texte` vers `/cahier/*` | C3, C5 ; socle 11 | parité des droits avec le web ; plus aucun appel à `/cahier-texte` | M |
| **C12** IA (après l'étude juridique de D-032 pour ce qui touche les élèves) | capacités §17.1 au registre du cœur IA, fournisseur nul par défaut, journal | C2, C7, C10 ; cœur IA | aucune capacité de notation ; tests de refus | M |
| **C13** bascule des clients et retraits | UI JavaScript (F-001), anciennes routes, vues de compatibilité | C11 | aucun appel aux anciennes routes 30 jours | S |
| **C14** RLS stricte et retraits | 2418, 2419 | C13 | inventaire RLS ; tests d'isolation | S |

**Parcours minimal vendable** (V1 de février 2027, D-034) : C0 à C6 et C8 (sans l'inspection fédérée) ; C11 pour l'agenda et la saisie mobile. Épaississement (février–août 2027) : C7, C9, C10, C12.

### 26.2 Migrations (registre `MIGRATIONS.md`, plage imposée **2400–2419**)
**2400** C0 et **C2a** (ajout personas : colonnes `visible_a` sur `ent_seance_contenu` et `ent_travail`) : clés `RESTRICT` et composites sur les tables de 150 (ajout `NOT VALID` puis validation après contrôle des orphelins), colonne `retiree_le` sur `ent_cdt_piece`, garde de non-suppression d'une séance porteuse (déclencheur `BEFORE DELETE` sur `ent_edt_seance` qui refuse si une entrée, un travail ou une remise s'y attache ; coordonné avec T0 du socle 10 : si T0 passe le premier, 2400 ne pose que ce qui manque) ; **2401** réglages et natures ; **2402** entrées, versions, notions, contributions, notes d'accompagnement ; **2403** travaux, versions, formats, cibles, variantes, échéances individuelles, demandes de délai, groupes ; **2404** pièces (`objet_id`) et liens ; **2405** remises, dépôts, signalements de dépôt, statuts, coches, vu ; **2406** retours, versions, critères, signalements de copie, consentements ; **2407** exercices, questions, choix, réponses attendues, liens aux travaux, tentatives ; **2408** progressions, étapes, partages ; **2409** reports et continuités ; **2410** visas, éléments figés, observations, accès d'inspection, consultations, alertes de charge ; **2411** archives annuelles ; **2412** reprise de l'existant et vues de compatibilité ; **2413–2417** réserve (C6 à C12) ; **2418** RLS stricte ; **2419** retrait des anciennes tables (`ent_cahier_texte`, `ent_seance_contenu`, `ent_travail`, `ent_travail_remise`, `ent_travail_eleve`, `ent_cdt_historique`) après la durée d'archive.

---

## 27. Tests et recette

- **Matrice de séparation** (§19.2) exécutée en test pour chaque route : élève du cours, élève d'un autre groupe, élève parti, responsable habilité, responsable sans suivi, responsable suspendu, responsable d'un majeur fermé, professeur en saisie, ancien professeur (J+1, J+8), suppléant (avant, pendant, après), remplaçant (la séance, le lendemain), autre professeur de la classe, professeur d'une autre classe, CPE, AED, AESH (accompagné, non accompagné), direction, inspecteur (pendant, après), établissement B.
- **Cas limites** CC-01 à CC-67 (et ceux de la passe personas) : un test chacun.
- **Propriétés** : recalculer la charge ne dépend pas de l'ordre des saisies ; une suite de reports ne perd aucun travail ; un dépôt ne change jamais après écriture (déclencheur).
- **Reprise** : jeu de labo S2-6 → compteurs identiques, quarantaine attendue.
- **Sécurité** : XSS dans titre, consigne, contenu, nom de fichier, lien ; IDOR sur chaque identifiant (travail, remise, dépôt, pièce, retour, tentative, visa, accès) ; lien signé rejoué, transféré, expiré ; fichier polyglotte ; injection de formule dans l'export ; énumération des élèves par la charge.
- **Accessibilité et mobile** : axe-core, clavier, lecteur d'écran, 375 px pour les écrans 1, 2, 4, 5, 9.
- **Recette personas** : parcours de `PERSONAS_20.md` rejoués sur le labo.

---

## 28. Réglages signalés (D-023)

| Réglage | Valeur par défaut | Pourquoi il existe | Allègement possible |
|---|---|---|---|
| S20-01 Publication par défaut | début de séance (entrée), fin de séance (travail) | habitudes différentes | garder |
| S20-02 Repère de renseignement | soir même | pilotage de la tenue | pourrait être fixe |
| S20-03 Lecture de l'année close | fin de l'année scolaire suivante (R-06) | imposé par la circulaire | **fixe** : à retirer des réglages (ne peut pas être raccourci) |
| S20-04 Délai minimal entre visibilité et échéance | 12 h | travaux donnés trop tard | fixe possible |
| S20-05 Application automatique d'un report non traité | oui, à l'heure de la séance | ne pas laisser une échéance impossible | garder |
| S20-06 Échéance pendant la plage calme au lycée | non | numérique responsable | garder |
| S20-07 Durées par défaut par nature et niveau | préréglage | durée obligatoire au collège | garder |
| S20-08 Copies lisibles des responsables | **tranché (ajout personas)** : copie annotée rendue oui, dépôts non | vie privée de l'élève contre suivi familial | **retiré** des réglages (R-C37) |
| S20-09 Clôture du ramassage, heure du rappel | 14 jours ; 17 h | — | fusionner en un seul réglage « ramassage » |
| S20-10 Repères de charge | normal | culture d'établissement | garder (trois positions) |
| S20-11 AESH lit les copies | non | aide à la reprise d'un travail | par travail seulement |

---

## 29. Questions à Florian (avec recommandations, appliquées par défaut sauf mention, D-031)

| N° | Question | Recommandation |
|---|---|---|
| **Q-CDT1** | Le module cahier de Next.js (ébauche) a-t-il des données utiles à reprendre ? | **Non** (inventaire T0 : vide probable, perdable) ; aucune reprise ; **à confirmer par Florian** (fait qu'il connaît) |
| **Q-CDT2** | Quel outil tient aujourd'hui le cahier de textes officiel dans l'établissement pilote (Charlemagne n'en a pas d'après la matrice : École Directe, ENT régional, papier ?) ; pendant l'année de doublon 2027-2028, le cahier de ScolIA est-il en **usage réel** (seul cahier) ou en doublon ? | **Usage réel visé** si l'établissement n'a pas d'autre cahier numérique officiel : le cahier est quotidien, ne dépend ni des notes ni de Charlemagne, et c'est une excellente porte d'entrée (D-035) ; cela place le socle 20 au niveau **production** pour la rentrée 2027 (C0–C8, C11) ; **soumis à Florian** (engage l'établissement) |
| **Q-CDT3** | Durées de conservation (§24.3) : le cahier suit la circulaire (un an actif + cinq ans) ; les **copies d'élèves** n'ont aucune durée officielle | Copies, retours, tentatives : fin de l'année scolaire suivante ; **à valider par le DPO** (soumis) |
| **Q-CDT4** | Les copies rendues sont-elles lisibles des responsables (S20-08) ? | **Non par défaut** (statut et retour seulement) ; ouverture par l'établissement ; la passe personas tranche (§30) |
| **Q-CDT5** | Faut-il un repère de charge bloquant ? | **Non** : alerte seulement (aucun texte national) |
| **Q-CDT6** | La CPE voit-elle les non-rendus agrégés d'un élève (suivi du décrochage) ? | **Oui**, agrégés (nombre par semaine), sans matière hors de sa classe ni contenu |
| **Q-CDT7** | IA : propositions de commentaires sur une copie ? | **Non** tant que l'étude juridique de D-032 n'a pas conclu (haut risque probable) |
| **Q-CDT8** | Outil de similarité entre copies ? | **Non en V1** ; à réexaminer avec l'étude juridique |
| **Q-CDT9** | Un intervenant ou assistant (socle 6) peut-il écrire sa contribution ? | **Oui**, si `cahier` est dans les droits de son intervention ; à porter au socle 6 |
| **Q-CDT10** | Accès d'inspection : compte invité ScolIA ou export seulement en V1 ? | **Export figé en V1**, compte invité en C8 si demandé, compte académique fédéré après le socle 9 |
| **Q-CDT11** | Le « vu » doit-il exister ? | **Oui, technique seulement** (avis « modifié après lecture »), jamais affiché, purgé à la clôture — **renversé par la passe personas (Q-CDT18) : supprimé** |
| **Q-CDT12** | Plan de numérotation : la plage imposée 2400–2419 ne suit pas « 3000 + 100 × socle » (Q-B2 donnerait 5000–5099) | **Garder 2400–2419** (consigne du pilote, évite les collisions) ; renuméroter seulement si Q-B2 est adoptée **et** qu'aucune migration n'est écrite |

---

## 30. Faits, hypothèses, contradictions

**Faits [F]** : §2 (code relu, lignes citées) ; Next.js : ébauche sans contrôle de rôle (§2.1) ; plage 2400–2419 vérifiée libre (`MIGRATIONS.md`, migrations présentes jusqu'à 642).
**Vérifié [V]** : §5 (sources et dates).
**Hypothèses [H]** : pratiques de terrain (§3, §4), repères de charge chiffrés (§11.2), durées par défaut, valeurs de réglages.
**Contradictions relevées** :
1. `SOCLES.md` §20 dit « Dépend de : 10, 7, 6 » ; le socle 21 dit « dépend de 20 » et le 20 dépend du 21 pour les évaluations annoncées : **dépendance circulaire résolue par les ports** (le 20 lit `evaluations::calendrier`, le 21 lit `cahier::travail_lie`), aucune table partagée.
2. `SOCLE2_DOMAINES.md` CDT-03 : « modification par l'auteur ou la direction » ; ici la direction **retire**, ne modifie pas (C-D10) : **correction proposée**, appliquée par défaut.
3. Le type `controle` de 150 et l'évaluation annoncée du 21 (EVA-117) font double emploi : le 21 possède (§6.4).
4. Socle 6 §8.5 : `assistant` / `intervenant` « cahier en lecture » ; ici ils peuvent écrire leur contribution (Q-CDT9) : **à aligner au socle 6**.
5. Socle 8 (ligne « cahier ») propose les avis « titulaires si abonnés » : cohérent ; la plage calme du socle 16 (R-18) mentionne explicitement les devoirs : appliquée ici (R-C5).
6. **(ajout personas)** §9.1 « modification mineure sans avis » contre la valeur juridique (R-01) : résolue par R-C29. 7. §19.2 « fichiers si S20-08 » contre §10.3 : résolue par R-C37. 8. Q-CDT11 contre la minimisation : renversée (R-C42). 9. §14.2 (l'inspection lit sans viser) contre R-02 : corrigée (CDT-263). 10. Socle 6 R-P3 (droits à la prise de poste) contre la lecture préparatoire (R-C33) : à porter au socle 6.

---
## 31. Les cinq décisions à prendre en priorité

| # | Décision | Pourquoi maintenant | Recommandation | Renvoi |
|---|---|---|---|---|
| **1** | **Lancer le lot C0** (contenu détruit à la régénération, écriture et lecture des copies permanentes, copies visibles de la vie scolaire et des autres professeurs, droits des liens ignorés, élève parti qui dépose encore), coordonné avec T0 du socle 10 | Perte de données et fuites **dans du code déjà intégré** | Oui, sur la tête d'`ENT` ; appliqué par défaut (D-031) | §2.2, §26 |
| **2** | **Plage calme livrée tôt** (C2a, ajout personas) | Arrêté du 7-9-2026, conformité sous dix mois (fin juillet 2027) | Oui, juste après C0 | §5 R-14, §32.1 |
| **3** | **Usage réel ou doublon** du cahier en 2027-2028 | Décide du niveau visé (production ou V1) | Usage réel si l'établissement n'a pas d'autre cahier numérique officiel — **Florian** | Q-CDT2 |
| **4** | **Modèle « cours / séance / travail »** avec reprise et quarantaine | Conditionne C1 à C14 et les ports du 10, du 12, du 21 | Oui | §7, §24, §25 |
| **5** | **Durées de conservation des copies et AIPD** | Aucun texte sur les copies ; données de mineurs | Fin de l'année scolaire suivante ; AIPD avant l'usage réel — **DPO** | Q-CDT3, Q-CDT14 |

---

## 32. Ajouts de la passe personas (ajout personas, `PERSONAS_20.md`, 2026-11-11)

Rien de ce qui précède n'est supprimé ; quand un ajout contredit le texte d'origine, il le dit et la contradiction est reprise au §30.

### 32.1 Trois bloquants (appliqués par défaut, D-031) (ajout personas)
- **A — Plage calme livrée tôt.** Nouveau lot **C2a** entre C0 et C1 : `visible_a` calculé côté serveur (plage nationale de l'arrêté du 7-9-2026, vacances et jours fériés du socle 5), avis différés, message de rappel du droit à la déconnexion à l'auteur, levée par la direction tracée. Sur les tables actuelles (colonne `visible_a` ajoutée à `ent_seance_contenu` et `ent_travail` par la migration 2400) pour ne pas attendre C1. Test d'or : saisie le dimanche 21 h → visible et avis le lundi 7 h ; saisie le vendredi 19 h 59 → visible tout de suite ; veille de rentrée → 7 h le jour de la rentrée.
- **B — Versions visibles des lecteurs.** R-C29 et R-C30 ; **correction du §9.1** : « modification mineure, sans avis » ne retire que l'avis, jamais la mention « modifié le » ni l'accès aux versions précédentes ; l'élève et ses responsables lisent les versions d'un travail ou d'une entrée publiés.
- **C — Corrigés côté serveur.** R-C31 ; critère de fin de C10 : aucune charge utile envoyée au client ne contient `correct`, `reponse_attendue` ni `explication` avant l'heure de correction (test qui inspecte les réponses de l'API).

### 32.2 Actions ajoutées (ajout personas)
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CDT-250 | Choisir l'échéance par raccourci (« prochaine séance », « dans deux séances », « fin de semaine ») | PRF · `cahier.ecrire` | calcul sur les séances du cours | — |
| CDT-251 | Modifier une séance copiée sur plusieurs cours « aussi dans les autres » | PRF · idem | copies reliées ; une version par cours | `cahier.seance_modifiee` |
| CDT-252 | Flouter les visages d'une photo du tableau (traitement local) | PRF | proposé, jamais automatique sans aperçu | — |
| CDT-253 | Projeter le travail de la séance en grand caractère (fin d'heure) | PRF | vue « tableau » | — |
| CDT-254 | Voir les épreuves d'examen et les semaines protégées dans la charge | PRF, PP | socle 38 | — |
| CDT-255 | Lire le cahier d'un cours dès sa nomination en suppléance, avant la prise de poste | SUP | lecture seule ; pas de copies ; R-C33 | — |
| CDT-256 | Décider du sort des copies en attente à l'ouverture d'une suppléance | DIR, SUP | « je corrige » / « le titulaire corrigera » / « rendues sans correction » ; jamais un titulaire en congé de maladie | `cahier.copies_reattribuees` |
| CDT-257 | Demander au titulaire de partager sa progression avec le cours | DIR | jamais d'ouverture forcée | — |
| CDT-258 | Être prévenu d'un travail donné par un remplaçant pour sa séance, et le garder, modifier ou retirer | PRF (titulaire) | avis aux élèves si changé | `cahier.travail_modifie` |
| CDT-259 | Viser une classe par échantillon (tirage au hasard d'entrées par cours) | DIR · `cahier.viser` | liste figée de ce qui a été lu | `cahier.vise` |
| CDT-260 | Voir son propre taux de tenue (mêmes chiffres que la direction) | PRF | R-C34 | — |
| CDT-261 | Afficher la version d'un travail telle qu'elle était visible à une date (litige) | DIR ; auteur ; ELV et PAR pour ce qu'ils ont lu | versions | — |
| CDT-262 | Contester un retrait de contenu par la direction | PRF | réponse gardée | `cahier.retrait_conteste` |
| CDT-263 | Apposer un **visa d'inspection** avec observation au professeur | INS | accès tiers en cours | `cahier.vise_inspection` |
| CDT-264 | Voir les échéances des élèves d'un groupe d'aide aux devoirs (Devoirs faits, étude, internat) | AED, CPE · `cahier.lire` (portée « mes groupes ») | consignes seulement | — |
| CDT-265 | Noter « travaillé en étude » (fait, commencé, besoin d'aide) | AED, CPE | visible du professeur du cours, du PP, de l'élève ; familles : S20-15 | `cahier.aide_etude_notee` |
| CDT-266 | Lever, pour un travail, l'application par défaut des aménagements d'un élève | PRF | motif non demandé ; tracé | — |
| CDT-267 | Marquer des questions d'exercice « facultatives » pour une variante allégée | PRF | — | — |
| CDT-268 | Rédiger le texte d'usage de l'IA d'un travail | PRF | à partir du niveau choisi (R-C27) | — |
| CDT-269 | Déclarer, au dépôt, l'usage fait de l'IA | ELV | facultatif ; visible du professeur seul ; R-C38 | — |
| CDT-270 | Recevoir le rappel d'export du portfolio avant la purge | ELV (terminale, départ) | juin | — |
| CDT-271 | Accorder un **accès tiers en lecture** pour un élève (SAPAD, hôpital, CNED), avec l'accord des responsables | DIR ★ | borné, journalisé ; dépôt des retours du tiers dans un espace dédié | `cahier.acces_tiers_accorde` |
| CDT-272 | Créer un **projet pluriannuel** reliant des travaux sur deux années | PRF | R-C41 | `cahier.projet_cree` |
| CDT-273 | Poser une **période de vigilance** sur une classe ou un niveau | PP, DIR | repère abaissé d'un cran ; message au professeur | `cahier.vigilance_posee` |
| CDT-274 | Traduire une consigne pour la famille | PAR, ELV | traduction automatique marquée ; fournisseur du cœur IA | — |
| CDT-275 | Imprimer la fiche hebdomadaire d'un élève ou d'une classe | PAR ; ADF, CPE, PP | sans copies ni statuts d'autres élèves | — |
| CDT-276 | Publier la notice d'information du cahier (élèves, familles, professeurs) | DIR | modèle fourni (socle 7) | — |
| CDT-277 | Importer ses progressions depuis ses fichiers (Word, PDF) en brouillon | PRF | moteur de conversion D-005 | — |
| CDT-278 | Recevoir l'avis « modifié moins de 24 h avant l'échéance » avec proposition de délai | ELV, PAR | R-C30 | — |
| CDT-279 | Voir la note d'accompagnement de l'AESH sur un travail | PRF, PP | R-C35 | — |
| CDT-280 | Fournir au socle 15 la preuve de service d'un remplacement (entrée signée) | SYS | port `cahier::seance_renseignee_par` | — |
| CDT-281 | Révoquer les jetons d'abonnement et liens actifs à la suspension d'un lien, au départ, à la fin d'une intervention | SYS | M20-67 | — |
| CDT-282 | Reporter seule l'échéance individuelle d'un élève en PFMP au retour de période | SYS → PRF (proposition) | R-C40 | `cahier.delai_accorde` |
| CDT-283 | Saisir un brouillon hors ligne sur le web | PRF | stockage local du navigateur, chiffré par la session ; envoyé au retour | — |
| CDT-284 | Voir le tableau des élèves avec aménagements appliqués sur un travail (pastilles) | PRF | sans motif | — |
| CDT-285 | Ouvrir, par travail, les dépôts vidéo | PRF | S20-13 | — |
| CDT-286 | Consulter le journal des accès tiers à son cahier | PRF | — | — |
| CDT-287 | Retirer de la file d'avis un travail publié par erreur dans les 5 minutes (annulation) | PRF | aucun avis parti si dans la file différée | `cahier.publication_annulee` |
| CDT-288 | Faire relire la notice et la charte par le DPO (circuit) | DIR | Q-CDT14, Q-CDT15 | — |
| CDT-289 | Voir la liste des cours sans entrée depuis 10 jours ouvrés (aide, pas sanction) | DIR, auteur | R-C34 | — |

### 32.3 Règles ajoutées (ajout personas)
| N° | Règle |
|---|---|
| R-C29 | Toute modification d'une entrée ou d'un travail **publié** est visible de tous ses lecteurs (« modifié le … ») et les versions précédentes leur sont lisibles ; seul l'avis peut être omis. |
| R-C30 | Une modification de consigne, d'échéance, de documents ou de mode de rendu dans les **24 h avant l'échéance** envoie toujours un avis et propose un délai d'au moins 24 h à qui le demande. |
| R-C31 | Les réponses attendues, la propriété `correct` des choix et les explications ne quittent jamais le serveur avant l'heure de correction de l'exercice ; la correction est faite côté serveur. |
| R-C32 | La charge d'un travail long (durée > 90 min ou échéance > 7 jours) est répartie sur les jours ouvrés entre la publication et l'échéance. |
| R-C33 | Un suppléant **nommé** lit le cahier du cours (entrées, travaux, aménagements utiles) dès sa nomination, sans saisie ni lecture des copies, jusqu'à sa prise de poste (à porter au socle 6). |
| R-C34 | Le taux de tenue d'un cours est visible de ses intervenants et de la direction, avec les mêmes chiffres ; aucun classement, aucun export nominatif hors litige. |
| R-C35 | Une note « travaillé en étude » ou d'accompagnement est un fait daté, signé, visible du professeur du cours, du PP et de l'élève ; jamais des autres élèves ; familles selon S20-15. |
| R-C36 | Les aménagements utiles d'un élève (socle 35) sont **appliqués par défaut** à chaque nouveau travail ; le professeur peut les lever pour ce travail, sans motif, tracé. |
| R-C37 | Les responsables habilités lisent la **copie annotée rendue** et le commentaire de retour (avec « voir les notes » si une note est affichée) ; ils ne lisent pas les dépôts de l'élève avant le retour. |
| R-C38 | La déclaration d'usage de l'IA par l'élève est facultative, visible du seul professeur du cours, et ne sert jamais seule de preuve de fraude ; aucun outil de détection n'est proposé. |
| R-C39 | Une demande de travaux pour une exclusion temporaire ne porte que la période ; jamais le motif. |
| R-C40 | Aucune échéance n'est posée pour un élève pendant une période où il n'a pas cours dans l'établissement (PFMP, stage, séjour) : échéance individuelle reportée au retour, proposée au professeur. |
| R-C41 | Un projet pluriannuel garde lisibles, en année 2, les dépôts de l'année 1 pour l'élève et pour l'intervenant en saisie du cours de l'année 2 du même élève ; sa purge suit la fin du projet. |
| R-C42 | Aucune trace de lecture nominative (« vu ») n'est conservée par le socle 20 (remplace R-C14 pour le « vu »). |
| R-C43 | Les statistiques d'exercices sont calculées par exercice ; aucun indicateur cumulé par élève dans le temps n'est produit. |
| R-C44 | Une remise accepte au plus 20 dépôts et 100 Mo cumulés (S20-14) ; débit limité par élève. |

### 32.4 Cas limites ajoutés (ajout personas)
| N° | Cas | Comportement |
|---|---|---|
| CC-68 | DS de 3 h le samedi matin hors séance | évaluation du socle 21 avec plage horaire (séance ajoutée au 10) ; dans l'agenda et la charge |
| CC-69 | 34 copies déposées non corrigées à l'ouverture d'une suppléance | CDT-256 ; aucune ne reste sans décision |
| CC-70 | Variante avec délai attribuée après l'échéance commune | statut « en retard » recalculé |
| CC-71 | AESH mutualisée qui change d'élève accompagné le 1er février | lecture de l'ancien élève jusqu'au 31 janvier seulement |
| CC-72 | Travail posé pour un jeudi où 6 élèves du cours sont en PFMP | échéance des 6 reportée au retour, proposée |
| CC-73 | Ressource SVG avec script fournie par un professeur | convertie en image, jamais servie telle quelle |
| CC-74 | Parent suspendu qui garde l'URL d'abonnement au calendrier | jeton révoqué à la suspension ; 410 |
| CC-75 | Consigne modifiée la veille à 21 h 40 | visible « modifiée » le lendemain 7 h, avis, délai proposé (R-C30) |
| CC-76 | Litige : un parent dit ne pas avoir vu la consigne de la version 1 | CDT-261 montre ce qui était visible, à quelle heure |
| CC-77 | Élève qui change d'établissement avec un projet pluriannuel en cours | export proposé ; aucun transfert de dépôts (cloisonnement) |
| CC-78 | Suppléance nommée puis annulée avant la prise de poste | la lecture préparatoire s'arrête à l'annulation |
| CC-79 | Enseignant du SAPAD dont l'accès expire pendant l'hospitalisation | prolongation par la direction ; avis à la direction 7 jours avant |
| CC-80 | Publication programmée pendant les vacances | visible à 7 h le jour de la rentrée |
| CC-81 | Professeur qui publie à 19 h 59 min 50 s | visible tout de suite (hors plage) ; avis immédiat |
| CC-82 | Fiche papier d'un élève avec variante | sa variante seulement |
| CC-83 | Traduction demandée d'une consigne contenant un prénom d'élève (contrôle doux ignoré) | prénom masqué avant envoi au fournisseur |
| CC-84 | Élève qui lit une consigne modifiée sans avis (correction mineure) | mention « modifié le … » et ancienne version lisible |
| CC-85 | Exercice dont le professeur veut afficher la correction « après chaque question » | la réponse attendue n'est envoyée qu'après la réponse de l'élève à cette question |
| CC-86 | Inspection d'un cours de l'année close | lisible pendant l'année suivante ; ensuite l'archive |
| CC-87 | Visa par échantillon puis modification d'une entrée non tirée | aucune mention « modifiée après visa » (elle n'a pas été vue) ; une entrée tirée et modifiée l'est |
| CC-88 | Remplaçant qui donne un travail pour la séance du titulaire, retiré par celui-ci | avis aux élèves ; dépôts déjà faits gardés |
| CC-89 | Groupe d'étude de l'internat dont un élève change d'internat en cours de semaine | portée à la date |
| CC-90 | Annulation d'une publication erronée 2 minutes après, avis encore en file | aucun avis envoyé ; trace de l'annulation |

### 32.5 Compléments de modèle (ajout personas)
```
ent_cdt_acces_tiers                -- remplace ent_cdt_acces_inspection (§24.1)
  id, nature ('inspection','sapad','hopital','cned','autre'), beneficiaire_user_id,
  agent_cible_id NULL | eleve_id NULL, du_cahier, au_cahier, valable_du, valable_au,
  accord_responsables_le NULL (obligatoire si eleve_id), accorde_par, revoque_le NULL, peut_viser BOOLEAN (inspection)
ent_cdt_acces_tiers_cours          acces_id, cours_id
ent_cdt_retour_tiers               id, acces_id, travail_id, eleve_id, commentaire, objet_id NULL, le   -- dépôts des enseignants extérieurs
ent_cdt_visa.nature                ('direction','inspection')  -- colonne ajoutée ; acces_tiers_id NULL
ent_cdt_aide_etude                 id, travail_id, eleve_id, etat ('fait','commence','besoin_aide'), note NULL, par_agent_id, le
ent_cdt_projet                     id, titre, eleve_ou_groupe (FK), debut_annee_id, fin_annee_id, clos_le NULL
ent_cdt_projet_travail             projet_id, travail_id
ent_cdt_declaration_ia             depot_id, texte, le
ent_cdt_periode_vigilance          id, cible ('classe','niveau'), classe_id | niveau_id, du, au, motif_public, par
ent_cdt_exercice_question.facultative BOOLEAN       -- colonne ajoutée
ent_cdt_vu                         -- RETIRÉE (R-C42) ; la migration 2405 ne la crée pas ; ent_travail_eleve.vu_le est vidé à la reprise
```
Contradiction assumée avec le §24.1 : `ent_cdt_acces_inspection` et `ent_cdt_vu` y figurent ; ils sont remplacés ou retirés ici.

### 32.6 Lots modifiés (ajout personas)
| Lot | Changement |
|---|---|
| **C2a** (nouveau, après C0) | Plage calme sur les tables actuelles, rappel à l'auteur, levée tracée, annulation de publication dans les 5 minutes (CDT-287) ; migration **2400** (colonnes `visible_a`) ; critère : test d'or du §32.1 |
| C2 | R-C29, R-C30, CDT-250, CDT-251, CDT-253, CDT-261, CDT-283 |
| C3 | CDT-274 (traduction, après le cœur IA), CDT-275 (fiche papier), CDT-278 ; notice CDT-276 |
| C4 | R-C32, périodes de vigilance (CDT-273), épreuves (CDT-254) |
| C5 | R-C37, R-C44, CDT-269 (déclaration IA), CDT-285 (vidéo) |
| C6 | CDT-255, CDT-256, CDT-257, CDT-258, CDT-264, CDT-265, R-C40, CDT-282 |
| C7 | R-C36, CDT-266, CDT-267, CDT-284 |
| C8 | accès tiers généralisé (CDT-271), visa d'inspection (CDT-263), visa par échantillon (CDT-259), tenue transparente (CDT-260, CDT-289), contestation d'un retrait (CDT-262) |
| C9 | CDT-277 (import des progressions), projets pluriannuels (CDT-272, R-C41) |
| C10 | R-C31 (bloquant C), R-C43, facultatives (CDT-267) |
| C0 | CDT-281 (révocation des jetons) |

**Parcours minimal vendable** révisé : C0, **C2a**, C1 à C6, C8 ; C11 pour l'agenda et la saisie mobile.

### 32.7 Réglages ajoutés (ajout personas)
S20-12 périodes de vigilance (ponctuelles) ; S20-13 dépôts vidéo (désactivés) ; S20-14 plafond par remise (20 dépôts, 100 Mo) ; S20-15 « travaillé en étude » visible des familles (non). **S20-03 retiré** (imposé par la circulaire). **S20-08 tranché** : copie annotée rendue lisible des responsables habilités, dépôts non (R-C37).

### 32.8 Questions ajoutées (ajout personas)
| N° | Question | Recommandation |
|---|---|---|
| Q-CDT13 | Lecture préparatoire d'un suppléant nommé, avant la prise de poste (à porter au socle 6, R-P3) | oui, lecture seule, sans copies |
| Q-CDT14 | AIPD du socle 20 | oui, avant l'usage réel — **DPO** (soumis) |
| Q-CDT15 | Charte d'usage et mention au règlement intérieur | oui — **Florian, conseil pédagogique, conseil d'administration** (soumis) |
| Q-CDT16 | Accès des enseignants du SAPAD et de l'hôpital | accès tiers ; convention avec le service académique — **établissement** (soumis) |
| Q-CDT17 | Fournisseur de traduction | celui du cœur IA, contrat européen (soumis : contrat) |
| Q-CDT18 | Le « vu » | **supprimé** (renverse Q-CDT11) |
| Q-CDT19 | « Travaillé en étude » visible des familles | non par défaut |
| Q-CDT20 | Visa d'inspection | oui (R-02) |
