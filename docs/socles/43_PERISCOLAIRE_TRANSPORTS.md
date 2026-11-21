# Socle 43 — Activités périscolaires et transports

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** tout code de ce socle. Aucun code écrit ni modifié. Aucun accès à une base, à un secret ni à un environnement de production. Le code Next.js a été **lu seulement** (lecture ciblée dans le dépôt, rien exécuté, rien construit, aucun `.env` lu).
**Branche :** `ent-socle-43-44-conception`, partie de `origin/ent-mc0hoe` (`afeb8c4`).
**Passe personas :** `docs/socles/PERSONAS_43.md` (même branche). Ses ajouts sont faits **ici**, marqués **« ajout personas »**, et regroupés au **§30**. **En cas de contradiction, le §30 l'emporte** ; rien n'a été supprimé.
**Socle jumeau :** `docs/socles/44_RESSOURCES_MATERIELLES.md`, conçu dans la même session (salles réservées pour les activités, matériel prêté à l'AS, casiers, clés du foyer).
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-017 (droits portés par les actions), D-020 (personne distincte du compte), D-021 (méthode), D-023 (peu de réglages), D-026 (briques), D-027 (entité émettrice par famille de documents), D-028 (notifications), D-029 (public), D-031 (recommandations appliquées par défaut), **D-034** (feuille de route, doublon 2027-2028, finances dans Charlemagne), **D-035** (reprise de l'add-on), **D-036** (aucun JSON-mémoire) ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; `docs/TRANSITION_T0_INVENTAIRE.md` (M17 partenariats, M20 covoiturage, grille de l'élève) ; `docs/SOCLES.md` §43 ; `docs/SOCLE2_DOMAINES.md` D22 (PER-01 à PER-05, repris ici) ; `docs/MATRICE_CHARLEMAGNE.md` (VOY-09, VOY-10, VOY-11, T-19) ; conceptions `04_ADMINISTRATIF.md` (titulaires, payeurs, grille de sortie R-A43, inscriptions aux services), `05_STRUCTURE.md` (groupe de nature *activité*, STR-53, R-S15, lieux extérieurs), `06_PERSONNELS.md` (intervenants, bénévoles P60, honorabilité PRS-143), `08_NOTIFICATIONS.md` (audience « usagers d'une ligne », CN-67, R-N50, NTF-132, NTF-140), `10_EMPLOI_DU_TEMPS.md` (CL-39, CL-67, CL-71, `transport.inscriptions_modifiees`), `12_ASSIDUITE.md` (composant d'appel, RA-02), `13_DISCIPLINE.md` (retenue et dernier car), `14_ACCUEIL_SURETE.md` (régime de sortie, présents hors temps scolaire ACS-338), `17_ANNONCES.md` (neutralité commerciale RG-5), `18_FORMULAIRES.md` (FRM-249, `coordonnees_transport`, port `autorisations::verifier`), `19_AGENDA_RDV.md` (réservation de lieux §14, sessions collectives AG6, programmes d'interventions AG6b), `24_FINANCES.md` (engagement daté, entité émettrice), `25_PAIEMENTS.md`, `26` (aides, non conçu), `34_SANTE.md` (consignes sans diagnostic), `40_RESTAURATION.md` (Q-RS10 : étude et garderie de la grille → 43), `41_INTERNAT.md` (activités du soir, transport du vendredi), `42_SORTIES_VOYAGES.md` (sortie collective, « en sortie ») et `IA_COEUR.md` (§5, §6.2).

**Conventions.** **[F]** fait vérifié dans le code (Next.js : chemins relatifs à `app/` ou `db/` du dépôt ; Rust : chemins depuis `ent-rust/`). **[V]** règle lue sur une source officielle (URL et date au §5). **[NV]** non vérifié. **[H]** hypothèse de terrain. **[R]** recommandation, non décidée.

Numérotation propre à ce socle (préfixes vérifiés libres dans `docs/` le 2026-11-11) :

| Élément | Numéros | Remarque |
|---|---|---|
| Actions existantes de Next.js | **NP-01**… | §2.2 (partenariats, covoiturage, grille de l'élève) |
| Défauts de Next.js | **DP-…** est pris ailleurs : **DPS-01**… | §2.3 |
| Défauts du code Rust existant | **RPS-01**… | §2.5 |
| Actions cibles | **PSC-100**… | PER-01 à PER-05 de `SOCLE2_DOMAINES.md` D22 sont repris (PSC-120, PSC-140, PSC-170, PSC-200, PSC-230) |
| Règles | **R-PS1**… | |
| Cas limites | **CPS-01**… | |
| Questions | **Q-PS1**… | |
| Réglages signalés (D-023) | **S-4300 à S-4349** | (S-430 à S-435 à trois chiffres sont au socle 42 ; aucun S-43xx à quatre chiffres n'existe) |
| Manques (passe personas) | **M-4301**… | (M-430 à M-439 à trois chiffres existent ailleurs ; aucun M-43xx à quatre chiffres) |

**Plage de migrations imposée : 2860–2879** (inscrite dans `docs/socles/MIGRATIONS.md`). Le lot 0 (PS0) part en **2860**.

Sommaire : §0 En bref · §1 Thèse · §2 Existant (Next.js et Rust) · §3 Acteurs · §4 L'année vue du terrain · §5 Réglementaire · §6 Frontières et branchements · §7 Concepts · §8 Offre d'activités · §9 Inscriptions · §10 Séances, appel et présence · §11 Association sportive, foyer, maison des lycéens · §12 Études, garderie, accompagnement · §13 Partenaires et intervenants · §14 Transports scolaires · §15 Covoiturage · §16 Argent · §17 Cas limites · §18 IA · §19 Actions · §20 Règles · §21 Droits et minimisation · §22 Événements · §23 Écrans et mobile · §24 Modèle de données · §25 Lots · §26 Reprise et bascule · §27 Tests et recette · §28 Réglages · §29 Questions et décisions · §30 Ajouts personas · §31 Faits, hypothèses, contradictions.

---

## 0. En bref

1. **Ce socle répond à deux questions que l'établissement se pose chaque soir à 17 h 05 : « qui est encore censé être ici, et sous la responsabilité de qui ? » et « qui prend le car, lequel, et que se passe-t-il s'il ne passe pas ? »** Le reste — catalogue, inscriptions, places, factures, licences — sert à y répondre juste, sans tableur ni liste papier qui dort au fond d'un casier.
2. **Ce qui existe est mince** [F] :
   - **Next.js** n'a **aucun module de clubs, d'association sportive, d'études ou de transport**. Il a trois choses utiles : (a) un module **partenariats** (catalogue d'offres de partenaires ou internes, inscription en ligne par « coupon » signé, places, fenêtre d'ouverture, export, réunions d'information) — **partiel**, en tables, **à reprendre** ; (b) un module **covoiturage** entre familles (code postal, double acceptation avant dévoilement) — **ébauche**, à refaire ; (c) une **grille hebdomadaire** sur l'élève (midi, soir, étude, garderie, « sort seul ») — dont l'étude et la garderie reviennent au 43 (Q-RS10 du socle 40).
   - **Rust** a des **groupes avec responsables** (lot S2-2, migrations 092 et 098) et deux natures `etude` et `garderie` dans `ent_repas_activite` (017). Pas de table d'activité, d'inscription, de séance d'activité, de ligne de transport. Le code des responsables de groupe a **5 défauts** (RPS-01 à RPS-05), dont 2 à corriger d'abord (lot **PS0**).
3. **Thèse** (§1) : une activité est **une offre** (qui, quoi, quand, où, combien de places, à quel prix, sous quelle entité) ; une inscription est **un engagement daté** d'un élève, pris par **les bonnes personnes** (titulaires du 4, ou l'élève majeur), qui ouvre une **présence attendue** à chaque séance ; la séance est **appelée** avec le composant du 12 ; un absent attendu **n'est pas « simplement pas venu »** : il est censé être dans l'établissement, on le cherche ou on prévient. Le transport est **une affiliation datée** (élève → ligne → arrêt → sens → jours), presque toujours **importée** de l'autorité organisatrice, qui sert à **prévenir** (car supprimé), **protéger** (retenue après le dernier car, élève non monté) et **organiser** (fin des cours, sorties, évacuation de car).
4. **Le réglementaire** (§5) commande plusieurs garde-fous : association sportive **obligatoire** dans les collèges et lycées publics, présidée par le chef d'établissement, affiliée à l'**UNSS** (UGSEL dans le privé catholique) ; l'AS, le **foyer socio-éducatif** et la **maison des lycéens** sont des **associations distinctes** de l'établissement : leur argent n'est pas celui de l'établissement (D-027 F, entité émettrice) ; **intervenants extérieurs** autorisés par le chef d'établissement, **contrôle d'honorabilité** pour ceux qui encadrent des mineurs ; **transports scolaires** de la compétence de la **région** (Île-de-France Mobilités en Île-de-France), et du **département** pour les élèves en situation de handicap : l'établissement **n'émet pas** les abonnements, il atteste la scolarité et reçoit des listes ; **neutralité commerciale** dans le public pour les offres de partenaires (RG-5 du 17).
5. **Ce que ScolIA apporte [H]** :
   - **un catalogue** des activités de l'année, ciblé (niveau, régime, âge, site), lisible par les familles dans l'appli, avec places, liste d'attente, **tirage au sort équitable et rejouable** ;
   - **l'inscription par les bons titulaires** (l'un suffit pour un acte usuel gratuit ; le payeur s'engage pour un acte payant ; l'élève majeur s'inscrit seul), avec les autorisations utiles demandées **en même temps** (sortie seul après l'activité, transport vers un gymnase, image) par le 18 ;
   - **l'appel de la séance** sur téléphone, hors ligne, avec les inscrits attendus, les absents annoncés, les élèves en sortie (42) ou à l'infirmerie déjà placés ; **les absents attendus remontent à la vie scolaire en quelques minutes**, et l'accueil (14) sait qui est encore dans les murs ;
   - **l'AS de bout en bout** : licences, créneaux, rencontres du mercredi avec convocation et autorisation annuelle, transport, résultats, « jeunes officiels », bilan pour l'assemblée générale, forfait des enseignants d'EPS visible ;
   - **les études et la garderie** comme engagements datés (plus de grille JSON), avec la sortie seul ou accompagné **vérifiée** au départ (14) ;
   - **les transports** : lignes et arrêts importés (fichier de l'autorité organisatrice), usagers par ligne, **avis « ligne 12 supprimée ce matin »** aux seules familles concernées (8, CN-67), **contrôle du dernier car** sur les retenues (13) et les fins de cours (10), retards de car **justifiés en lot** (12), **liste des élèves par car** pour l'exercice d'évacuation et en cas d'accident ;
   - **un covoiturage entre familles** refait, **désactivé par défaut**, sans adresse, sans enfant, à double acceptation et effacement réel ;
   - une **IA qui prépare** (catalogue rédigé, répartition proposée sur les créneaux, détection d'un élève inscrit à deux activités au même moment, brouillon du bilan de l'AS, lecture d'un fichier de lignes mal formé) **et ne décide ni n'envoie rien**, **jamais sur la santé**.
6. **Le plan** :

   | Élément | Volume | Numéros |
   |---|---|---|
   | Actions | 160 | PSC-100 à PSC-259 (dont PSC-240 à PSC-259 ajout personas) |
   | Cas limites | 56 | CPS-01 à CPS-56 (dont CPS-45 à CPS-56 ajout personas, §30) |
   | Règles | 38 | R-PS1 à R-PS38 (dont R-PS34 à R-PS38 ajout personas, §30) |
   | Réglages signalés | 20 | S-4300 à S-4319 (dont S-4316 à S-4319 ajout personas) |
   | Lots | 13 | PS0 à PS12 |

   - **Lot PS0** (correctifs urgents de l'existant Rust) : responsables de groupe en transaction unique et sans effacement global, dates de responsabilité, FK composites, journal (§25.1).
   - **Parcours minimal pour la rentrée 2027** : PS0, PS1 (offre et inscriptions), PS2 (séances et appel), PS3 (études et garderie, sortie vérifiée), PS5 (transports : import, usagers, avis). Le reste suit pendant l'année de doublon (D-034).
7. **Décisions appliquées par défaut (D-031)** : 26 questions tranchées (§29.1), plus 10 décisions de la passe personas (§30, `PERSONAS_43.md` §8.1). **Soumis à Florian** (§29.2) : la place de l'argent des associations (AS, foyer, APEL) dans ScolIA ; l'accès aux fichiers de l'autorité organisatrice des transports de la région ; l'ouverture du covoiturage ; la reprise des inscriptions de partenariats ; la vérification d'honorabilité des intervenants (procédure réelle de l'établissement) ; ajout personas : Q-PS34 à Q-PS38.
8. **Ajout personas (§30, `PERSONAS_43.md`)** : quatre bloquants tranchés — **rien ne retire un élève des attendus sauf le geste d'un adulte qui l'a vu partir** (R-PS36) ; **départ anticipé** par la remise du 14 et liste des présents imprimable hors ligne (R-PS38) ; **page « état des transports du jour »**, car l'absence d'avis n'est pas une assurance (R-PS37) ; **capacité de sécurité** par encadrant et taux R227-16 pour la garderie du premier degré (R-PS34). Le parcours minimal vise la **production** à la rentrée 2027 (Charlemagne ne gère ni études ni transports [H]).

---

## 1. Thèse : une inscription est un engagement daté qui crée une présence attendue

1. **L'activité référence, elle ne recopie pas.** Un inscrit est un **élève** (`eleve_id`) ; un encadrant est une **personne** du socle 6 (agent, intervenant, bénévole) ; une salle est une salle du 5 réservée par le 19 ; un groupe est un groupe du 5 de nature *activité* (R-S15). Aucun nom, aucune classe « en texte ».
2. **S'inscrire, c'est s'engager pour une période.** L'inscription a un **début**, une **fin**, un **rythme** (tous les mardis de 17 h à 18 h, sauf vacances), un **payeur** s'il y a un prix, des **autorisations** rattachées. Elle **naît**, **change** (passage du mardi au jeudi) et **finit** (désinscription, départ de l'élève, fin d'année) par des faits datés. Rien n'est écrasé.
3. **Être inscrit, c'est être attendu.** Pendant une séance, l'élève inscrit est **sous la responsabilité de l'établissement** (ou de l'association, selon l'activité, §7.2). Son absence non annoncée est un **écart** que le responsable signale (C. éduc. R131-5, RA-02 du 12), qui remonte à la vie scolaire, et que l'accueil voit. C'est l'erreur à ne pas faire : traiter l'étude du soir comme une liste de « peut-être ».
4. **Le transport n'est pas à nous, ses conséquences si.** Les abonnements, les cartes, les tarifs et les lignes sont gérés par l'autorité organisatrice. L'établissement en **reçoit une copie datée** pour trois usages : prévenir, protéger, organiser. Il ne la retape pas, et il ne la garde pas plus longtemps que l'année.
5. **L'argent des associations n'est pas celui de l'établissement.** La cotisation de l'AS, du foyer ou de la maison des lycéens est due **à l'association** ; ScolIA peut la **suivre** et, sur décision, l'**encaisser pour son compte** sous sa propre entité émettrice ; jamais la mêler aux comptes de l'établissement (D-027 F).
6. **Le serveur décide, l'IA propose.** Places, liste d'attente, tirage au sort, présence attendue, avis de transport : tout est calculé côté serveur, journalisé, rejouable. L'IA rédige et suggère ; elle ne place, n'inscrit, ne désinscrit et n'envoie rien.

---

## 2. Évaluation franche de l'existant

### 2.1 Où et comment Next.js le fait [F]

| Fonction | Où | Stockage | Maturité (T0) |
|---|---|---|---|
| **Partenariats** (offres de partenaires et internes, inscription par coupon signé) | `db/schema-partenariats.ts`, `drizzle/0059_partenariats.sql`, `app/lib/partenariats-{db,types,mail}.ts`, `app/api/partenariats/offres/**`, `app/api/partenariats/public/**`, pages `app/(admin)/etablissement/partenariats/**` et `app/(public-rentree)/partenariats/**` | tables typées + colonnes JSONB (`cycles`, `niveaux`, `tarifs`, `cta_links`) + signatures en fichiers S3 | Partiel [H] |
| **Covoiturage** entre familles | `app/api/covoiturage/route.ts`, `app/api/covoiturage/matches/[id]/route.ts`, `app/lib/covoiturage-{types,storage,matching,notify}.ts`, `app/(admin)/covoiturage/page.tsx` ; outil « désactivé tant que non prêt » (`app/lib/toolbox-tools.ts:41-46`) | collections KV `covoiturage__profiles`, `covoiturage__matches` (un document par collection, réécrit en entier) | **Ébauche** |
| **Grille de l'élève** (midi, soir, étude, garderie, « sort seul », du lundi au vendredi) | `db/schema.ts:417`, `app/lib/eleve-grille-repas.ts` | colonne JSONB `eleve_scolarite.grille_repas` | Partiel ; étude et garderie → 43 (Q-RS10) |
| **Barème garderie** du simulateur de tarifs public | `app/lib/toolbox-types.ts:36, 113`, `app/(public-rentree)/simulateurTarifs/` | réglages | informatif |
| Clubs, AS, UNSS, ateliers, études gérées, transports quotidiens | — | — | **inexistants** (recherche ciblée : occurrences de texte libre seulement ; les « cars » relèvent des voyages, socle 42) |

Le « planning par domaines » (M03, EVARS) n'est **pas** une activité périscolaire : c'est un programme éducatif obligatoire par classe ; il est porté par le **socle 19** (lot AG6b, Q-AG2 appliquée). Le 43 n'y touche pas (§6.1).

### 2.2 Ce que fait Next.js : actions existantes, toutes reprises [F]

**Partenariats**

| NP | Action | Où | Reprise |
|---|---|---|---|
| NP-01 | Créer, lister, modifier, supprimer une offre (slug unique par établissement, type « information » ou « inscription », cycles, niveaux, catégorie libre, contacts établissement et partenaire, boutons d'action, tarifs informatifs, démarche, texte d'engagement, signature exigée, adresse d'avis, places maximales, fenêtre d'ouverture, activation, ordre) | `app/api/partenariats/offres/route.ts:22-49`, `offres/[offreId]/route.ts:58-108` | PSC-100, PSC-101, PSC-104, PSC-110 (suppression → retrait daté) |
| NP-02 | Téléverser le logo de l'offre | `offres/[offreId]/logo/route.ts` | PSC-105 |
| NP-03 | Gérer des **réunions d'information** de l'offre | `offres/[offreId]/evenements/**` | PSC-196 (session du 19) |
| NP-04 | Lister les inscriptions, changer leur statut (soumise, validée, refusée, annulée) avec note | `offres/[offreId]/inscriptions/route.ts:12-51` | PSC-128, PSC-129 |
| NP-05 | Exporter les inscriptions en CSV (`;`) | `offres/[offreId]/export/route.ts` | PSC-133 |
| NP-06 | Catalogue public, fiche détaillée par slug | `app/api/partenariats/public/route.ts`, `public/[slug]/route.ts` | PSC-112 (catalogue famille), PSC-113 (vitrine publique sans données personnelles) |
| NP-07 | Inscription en ligne : élève (nom, niveau, classe, date de naissance), parent (nom, e-mail, téléphone), engagement de payer, **signature dessinée** | `public/[slug]/inscrire/route.ts:29-64`, `app/lib/partenariats-db.ts:653-667` | PSC-120, PSC-121 (sans ressaisie : élève et titulaire du cœur) |
| NP-08 | Contrôles d'ouverture : offre activée, de type inscription, dans sa fenêtre, places restantes (inscriptions « soumise » et « validée » comptées) | `app/lib/partenariats-types.ts:304-322`, `partenariats-db.ts:170-183` | R-PS6, R-PS7 |
| NP-09 | Avis au parent et à l'adresse d'avis de l'offre | `app/lib/partenariats-mail.ts` | PSC-124 (socle 8) |
| NP-10 | Inscription annulée si la signature n'a pas pu être enregistrée | `partenariats-db.ts` | R-PS9 (atomicité) |

**Covoiturage**

| NP | Action | Où | Reprise |
|---|---|---|---|
| NP-11 | S'inscrire ou modifier son profil : codes postaux, établissements, sens (matin, soir, les deux), note de 280 caractères | `app/api/covoiturage/route.ts:176-225` | PSC-220 (commune ou code postal unique, sens réellement utilisé) |
| NP-12 | Clore sa recherche ; réactiver | `route.ts:227-268` | PSC-222 |
| NP-13 | Se désinscrire | `route.ts:270-286` | PSC-223 (effacement réel) |
| NP-14 | Accepter ou refuser une correspondance ; dévoilement du nom et de l'e-mail **après double acceptation**, avis des deux côtés | `app/api/covoiturage/matches/[id]/route.ts:307-362` | PSC-224, PSC-225 |
| NP-15 | Voir son profil et ses correspondances ; « Famille inscrite » tant que rien n'est dévoilé | `route.ts:124-153` | PSC-226 |
| NP-16 | Appariement : au moins un code postal **et** un établissement communs ; une correspondance active par paire | `app/lib/covoiturage-matching.ts:119-153` | R-PS28 |

**Grille de l'élève**

| NP | Action | Où | Reprise |
|---|---|---|---|
| NP-17 | Cocher étude, garderie, « sort seul » par jour de la semaine sur la scolarité de l'élève | `app/lib/eleve-grille-repas.ts`, `db/schema.ts:417` | PSC-160 à PSC-166 (inscriptions datées), PSC-168 et socle 14 (sortie) |
| NP-18 | Barème garderie dans le simulateur public de tarifs | `app/lib/toolbox-types.ts:36, 113` | PSC-108 (tarif affiché depuis le barème réel du 24) |

### 2.3 Défauts trouvés dans Next.js (à ne pas reproduire ; corrigés dans la réécriture)

| DPS | Gravité | Défaut | Où [F] | Correctif ScolIA |
|---|---|---|---|---|
| DPS-01 | haute | **Données de mineurs collectées sur une page publique** sans compte : nom, classe, **date de naissance** de l'élève, coordonnées du parent, signature ; aucune vérification que la personne est titulaire | `public/[slug]/inscrire/route.ts:29-64` | inscription **dans l'espace famille** par un titulaire identifié ; lien nominatif à usage limité pour un titulaire sans compte (D-027 C) ; la vitrine publique ne collecte rien (R-PS10) |
| DPS-02 | haute | **Surréservation** : contrôle des places puis insertion hors transaction | `partenariats-db.ts:694, 717` | places en transaction avec verrou de ligne sur l'offre (R-PS7) |
| DPS-03 | haute | Limitation de débit **en mémoire** qui fait confiance à `X-Forwarded-For` ; pas d'anti-abus ; le formulaire peut faire partir des courriels vers une adresse quelconque | `inscrire/route.ts:25-45`, `partenariats-mail.ts:35` | plus de formulaire public d'inscription ; limitation de débit du socle 1 ; destinataires résolus côté serveur |
| DPS-04 | moyenne | Boutons d'action sans filtre de schéma : un lien `javascript:` peut être rendu sur la page publique (saisie par un compte administrateur) | `partenariats-types.ts:232`, `PartenariatPublicDetailClient.tsx:207` | liens `https:` seulement, domaine affiché, rendu échappé (R-PS11) |
| DPS-05 | moyenne | Changement de statut d'une inscription sans vérifier son rattachement à l'offre de l'URL (l'établissement reste filtré) | `offres/[offreId]/inscriptions/route.ts:34-39` | FK composite (offre, inscription) ; commande par identifiant d'inscription seulement |
| DPS-06 | moyenne | Aucun contrôle de doublon (même élève inscrit deux fois) | `partenariats-db.ts` | unicité (offre, élève, période) |
| DPS-07 | moyenne | Erreurs internes renvoyées au client (`String(e)`) | API partenariats | erreurs typées |
| DPS-08 | moyenne | Signature conservée sans route de consultation ni durée | `partenariats-db.ts:653-667` | signature du socle 7 (S1 à défaut de S2), durée au §21.3 |
| DPS-09 | moyenne | Tarifs, cycles, niveaux en **JSONB** ; catégorie en texte libre | schéma `partenariat_offre` | tables (D-036) ; catégorie codée (§8.1) |
| DPS-10 | haute | Covoiturage : **consentement jamais recueilli** (`consentAt` = date d'inscription) | `app/api/covoiturage/route.ts:210` | consentement explicite daté, retirable (R-PS27) |
| DPS-11 | haute | Covoiturage : **pas d'effacement** ; désinscription = statut ; correspondances dévoilées conservées ; année jamais purgée | `route.ts:275-283` | effacement réel à la désinscription et à la fin de l'année (R-PS29) |
| DPS-12 | haute | Covoiturage : **énumération** des familles par code postal (nombre illimité de codes, un courriel envoyé à chaque correspondance) | `route.ts:44-52, 100-121` | une seule zone, plafond de correspondances par jour, avis groupé (R-PS28) |
| DPS-13 | moyenne | Covoiturage : identifiants des deux comptes renvoyés **avant** dévoilement | `matches/[id]/route.ts:352, 358` | projection opaque (R-PS28) |
| DPS-14 | moyenne | Covoiturage : sens du trajet ignoré ; aucun contrôle de rôle côté serveur ; aucune vue de modération | `covoiturage-matching.ts`, `route.ts` | sens utilisé ; titulaires seulement ; modération (PSC-227) |
| DPS-15 | moyenne | Covoiturage : écritures concurrentes sur un document unique (dernier écrivain gagnant) | `app/lib/covoiturage-storage.ts` | tables, unicité en base |
| DPS-16 | moyenne | Grille : étude et garderie **sans date d'effet** ni historique ; « sort seul » mêlé aux repas | `eleve_scolarite.grille_repas` | inscriptions datées (43) ; régime de sortie (14) |

**Points de sécurité à remonter à Florian** (Next.js en production, le pilote n'y touche pas, D-035 point 3) : DPS-01 et DPS-03 (formulaire public qui reçoit des données de mineurs et peut servir de relais de courriel) ; DPS-04 (lien exécutable sur une page publique).

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)

- **Partenariats** : le ciblage par cycles et niveaux ; l'offre « information seule » à côté de l'offre « inscription » ; la fenêtre d'ouverture et les places ; l'engagement explicite et la signature exigible par offre ; les réunions d'information ; l'export `;` ; le slug lisible ; l'avis à l'adresse du partenaire ou du référent ; le logo.
- **Covoiturage** : la **double acceptation avant dévoilement** ; la limitation à une zone (jamais d'adresse ni de géolocalisation) ; « Famille inscrite » tant que rien n'est dévoilé ; l'activation par établissement ; l'avertissement de responsabilité des parents.
- **Grille** : la vue hebdomadaire d'un coup d'œil (lundi à vendredi), qui reste l'écran de saisie rapide du secrétariat (§12).

### 2.5 Code Rust existant [F]

| Élément | Où | Constat |
|---|---|---|
| Groupes | `migrations/092_referentiel_scolaire.sql:39` ; `crates/ent-db/src/referentiel_groupes.rs:41` ; route `POST /referentiel/groupes` (`crates/ent-api/src/referentiel.rs:44`) | `annee_id`, `code`, `libelle`, `classe_id` ; **pas de nature** (`activite`), **pas de capacité** ni de fermeture datée, alors que `05_STRUCTURE.md` §9.10 et R-S15 les prévoient ; FK simples (annee, classe) ; unique `(id, etablissement_id)` ajouté en 338 |
| Responsables de groupe | `migrations/098_roles_parametrables.sql:48` ; `crates/ent-db/src/droits_repo.rs:722` (`definir_responsables_groupe_for_actor`) ; route `PUT /referentiel/groupes/{id}/responsables` (`crates/ent-api/src/roles.rs:33`, gestionnaire l. 263) | droit `Action::RolesGerer` (`droits_repo.rs:798-812`) ; responsables **remplacés d'un bloc** (DELETE puis INSERT, l. 760-781) ; chaque responsable doit être personnel ; audit `groupe_responsables` ; portée `Portee::MesGroupes` (`crates/ent-auth/src/actions.rs:304`, résolue `droits_repo.rs:248-300`) ; lu par `cahier.rs:374` et `personnels_missions.rs:1863` |
| Études et garderie | `migrations/017_eleve_repas_bourses.sql:5` (`ent_repas_activite.kind` ∈ midi, soir, **etude, garderie**, autre) ; `ent_eleve_grille_repas` (017) ; `crates/ent-db/src/repas_repo.rs` (droits `RepasGerer`, `RepasScanner`, `actions.rs:111-113`) | l'étude et la garderie sont des « activités de repas » avec un masque de jours : ni séance, ni appel, ni sortie ; FK simple `activite_id` |
| Fonctions de personnel | `migrations/353` (fonctions `animateur`, `intervenant_exterieur`, `benevole`, `agent_maintenance`, l. 67-72) | utilisables comme encadrants (§13) |
| Sorties | `migrations/640_sortie_coeur.sql:67-70` (`besoin_car`, `consignes_transport`) | transport d'une sortie au 42, sans lien avec les lignes régulières |
| Actions D-017 | `crates/ent-auth/src/actions.rs` | **aucune** action pour les activités, les inscriptions ou les transports ; `groupes.composer` et `groupes.creer_activite` prévues par `05_STRUCTURE.md` (l. 931-932) **non codées** |
| Tables d'activité, d'inscription, de séance d'activité, de ligne, d'arrêt, d'abonnement | — | **absentes** |

**Défauts du code Rust existant (RPS)**

| RPS | Gravité | Défaut | Où [F] | Correctif |
|---|---|---|---|---|
| RPS-01 | haute | Responsables **remplacés par effacement global** : un appel concurrent ou une erreur partielle efface des responsables, et l'historique « qui était responsable le 12 mars » est perdu | `droits_repo.rs:760-781` | responsabilités **datées** en ajout seul (début, fin) ; commande « ajouter » / « retirer » ; PS0 |
| RPS-02 | haute | Existence du groupe vérifiée dans une **transaction séparée** de l'écriture (fenêtre entre contrôle et écriture) | `droits_repo.rs:735` puis `760` | une seule transaction, verrou de ligne sur le groupe ; PS0 |
| RPS-03 | moyenne | FK simple `groupe_id` avec `ON DELETE CASCADE` : supprimer un groupe efface ses responsables sans trace ; pas de FK composite | `098:48` | FK composite `(etablissement_id, groupe_id)`, `ON DELETE RESTRICT` ; fermeture datée du groupe ; PS0 |
| RPS-04 | moyenne | `ent_groupe` sans nature, capacité ni fermeture : un club et un groupe de langue sont indiscernables | `092:39` | colonnes du 5 (`nature`, `capacite_bloquante`, `ferme_le`) posées par le **socle 5** (R-S15) ; le 43 les consomme ; PS1 |
| RPS-05 | faible | Étude et garderie rangées dans les natures de repas (`ent_repas_activite.kind`) | `017:5` | inscriptions d'études et de garderie au 43 ; reprise des grilles (PS3) ; l'ancienne nature reste lue jusqu'au retrait (PS12) |

### 2.6 Verdict
- **Reprendre** les actions des partenariats (catalogue, places, fenêtre, engagement, export, réunions) en les **rebranchant** sur l'élève et ses titulaires, sans formulaire public de collecte.
- **Refaire** le covoiturage (ébauche, D-035 précision 2), en gardant la double acceptation.
- **Concevoir de zéro** clubs, AS, études, garderie, accompagnement éducatif et transports : rien n'existe ailleurs.
- **Corriger d'abord** les responsables de groupe (PS0), car le 43 s'appuie sur eux et le cahier de textes (20) les lit déjà.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Chef d'établissement** (président de l'AS dans le public) | autorise les activités et les intervenants, préside l'AS, répond de la sécurité hors cours | listes d'inscrits introuvables quand un parent appelle ; conventions d'intervenants éparpillées | un enfant « inscrit à l'étude » introuvable à 18 h ; un intervenant non vérifié | un tableau « hors cours ce soir » : qui, où, avec qui, combien |
| **Adjoint, CPE** | organise études et accompagnement, répartit les élèves, gère les absences aux études | tableurs d'études, appels papier perdus, relances « votre enfant n'était pas à l'étude » une à une | l'élève qui sort à 17 h alors qu'il est inscrit jusqu'à 18 h | absents de l'étude signalés à la vie scolaire dans les 10 minutes, avec l'avis aux familles prêt |
| **AED, surveillant d'étude** | fait l'appel de l'étude, surveille, laisse sortir | liste à recopier, sorties « seul » à vérifier sur un classeur | laisser partir un élève qui n'en a pas le droit | sur le téléphone : la liste, qui part seul à quelle heure, qui est attendu par qui |
| **Professeur d'EPS, animateur de l'AS** (secrétaire de l'AS) | licences, créneaux, rencontres UNSS du mercredi, convocations, résultats | double saisie dans la plateforme de licences de l'UNSS [NV], autorisations papier par rencontre, appels aux familles pour un car déplacé | partir en rencontre avec un élève sans autorisation ; l'élève blessé dont on n'a pas le numéro | convocation au groupe en un geste, autorisation annuelle, liste du car, résultats publiés, bilan d'AG pré-rempli |
| **Professeur animant un club** (théâtre, robotique, échecs, chorale) | appel, programme, salle | réserver la salle chaque semaine, recompter les inscrits | la salle prise par un conseil de classe | séances en série avec la salle réservée (19), conflits signalés à l'avance |
| **Intervenant extérieur ou bénévole** (club de foot partenaire, association de théâtre, aide aux devoirs) | anime une activité | aucun accès, listes reçues par courriel | être seul avec des mineurs sans cadre | un accès strictement limité à son activité, à son créneau, sans dossier d'élève |
| **Secrétariat, gestionnaire** | inscriptions papier, encaissement, attestations | fiches papier ressaisies, chèques à rapprocher, « mon fils est-il inscrit à la garderie ? » | facturer une famille pour une garderie que l'enfant ne fréquente plus | inscriptions en ligne, payeur juste, état « qui doit quoi » |
| **Comptabilité de l'établissement, trésorier de l'AS ou du foyer** | tenir les comptes de l'association, distincts | cotisations mêlées, reçus faits à la main | mélange d'argent entre l'établissement et l'association | une entité émettrice par association, un état séparé, un export |
| **Accueil** (14) | sait qui est encore dans les murs, remet les enfants | ne sait pas qui est à l'étude ou à l'AS | remettre un enfant à une personne non autorisée à 18 h | la liste « encore présents » à jour, avec les personnes autorisées |
| **Référent transports** (souvent secrétariat ou CPE) | relaie la région, gère les incidents de car | fichiers de la région reçus par courriel, cars supprimés annoncés par téléphone | un élève de 6e qui attend un car supprimé à 7 h dans le noir | « ligne 12 supprimée » envoyé aux seules familles de la ligne avant 6 h 30 |
| **Famille** (dont séparée, recomposée) | inscrit, paie, autorise, s'inquiète | papiers qui se perdent, ne sait pas si l'enfant est resté à l'étude | qu'un des parents inscrive sans l'autre à une activité payante ; ne pas être prévenue d'un car supprimé | catalogue clair, inscription en deux gestes, avis du car, « présent à l'étude ce soir » |
| **Élève mineur** | choisit, participe | ne connaît pas l'offre, pas de place | être exclu faute d'argent ; voir son nom sur une liste publique | voir l'offre, demander, liste d'attente honnête |
| **Élève majeur** | s'inscrit lui-même | doit encore faire signer ses parents | — | s'inscrire seul, ses parents informés seulement s'il l'accepte (socle 4) |
| **AESH** | accompagne un élève en activité si le PPS le prévoit | ne sait pas que l'élève est inscrit au club | l'élève inscrit sans accompagnement possible | être prévenu, le besoin signalé sans diagnostic (35) |
| **Collectivité, autorité organisatrice des transports** | lignes, abonnements, transport adapté | demandes d'attestation une par une | données périmées | une liste d'élèves scolarisés exportable au bon format, minimale |

---

## 4. L'année vue du terrain [H, à confirmer avec Florian]

| Période | Ce qui se passe |
|---|---|
| **Juin–juillet** | préparation de l'offre de l'année suivante ; reconduction des clubs ; réinscriptions en garderie et étude avec les réinscriptions (3) ; fichiers de transport de la région pour la rentrée (inscriptions aux cars en ligne chez l'autorité organisatrice) |
| **Fin août – 1re semaine** | listes de transport mises à jour, horaires des cars, élèves de 6e accompagnés au car le premier jour ; études et garderie ouvertes dès le jour de la rentrée (familles qui travaillent) |
| **2e–3e semaine de septembre** | forum des activités ; ouverture des inscriptions (fenêtre) ; tirage au sort des activités surchargées ; AS : licences, assemblée générale de l'AS [NV] |
| **Fin septembre** | début des séances régulières ; exercice d'évacuation de car pour les nouveaux transportés [NV] |
| **Octobre – mai** | séances, appels, absences ; rencontres UNSS du mercredi ; incidents de car (grèves, neige) ; changements d'activité au trimestre ; facturation au trimestre ou à la séance |
| **Vacances** | séances sautées (calendrier du 5) ; parfois stages pendant les vacances (activité ponctuelle) |
| **Mai–juin** | championnats, bilans, attestations de participation, « jeunes officiels » ; fin des activités ; effacement des données de transport et de covoiturage de l'année |

## 5. Ce qui est réglementaire (recherche du 2026-11-11)

Recherche par sous-agent sur Légifrance (lu dans la version en vigueur au 11/11/2026), BO, cnil.fr, documents officiels. education.gouv.fr et eduscol ont renvoyé 403 aux lectures automatiques : les textes du BO marqués **[S]** n'ont été vus qu'en extrait ou par une source officielle secondaire. Toutes les URL ont été consultées le **2026-11-11**.

### 5.1 Transports scolaires [V]
- **Compétence de la région** : Code des transports, art. **L3111-7** (version en vigueur depuis le 01/01/2022) : les transports scolaires sont des services réguliers publics ; « la région a la responsabilité de l'organisation et du fonctionnement de ces transports » ; dans le ressort d'une autorité organisatrice de la mobilité, cette responsabilité lui revient. **L3111-9** : la région peut déléguer à des autorités de second rang, **dont des établissements d'enseignement** ; **L3111-10** : participation possible aux frais de transport individuel. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000023086525/LEGISCTA000023070990/
- **Île-de-France** : **L3111-14** : Île-de-France Mobilités est responsable des transports scolaires ; **L3111-15** délégation possible ; **L3111-16** transport individuel des élèves handicapés. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000023086525/LEGISCTA000023071012/
- **Élèves en situation de handicap** : Code des transports **R3111-24 à R3111-27** (ancien C. éduc. R213-13) : élèves d'un établissement **public ou privé sous contrat** qui ne peuvent utiliser les transports en commun en raison de la gravité de leur handicap « médicalement établie » : frais « pris en charge par le **département** du domicile » ; remboursement aux familles, à l'élève majeur ou à l'organisme qui a avancé. **L3111-7-1** : mise en accessibilité des points d'arrêt ou transport de substitution. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000023086525/LEGISCTA000033449623/
- **Rôle de l'établissement** : aucun texte national ne lui confie les abonnements ; les règlements régionaux peuvent demander un certificat de scolarité ou une vérification par l'établissement, variable selon la région [S] (ex. règlement Pays de la Loire 2025-2026, règlement Centre-Val de Loire).
- **Évacuation de car** : **aucune obligation nationale trouvée** ; le *Guide pour la sécurité des transports scolaires* du Conseil national des transports (10 août 2009, §7-3) **recommande** que chaque enfant transporté participe au moins une fois par an à un exercice d'évacuation, à inscrire dans les conventions avec les transporteurs [V, document lu ; guide ancien]. https://banquedesterritoires.fr/sites/default/files/ra/Le%20guide%20pour%20la%20s%C3%A9curit%C3%A9%20des%20transports%20scolaires%20%C3%A0%20l%27usage%20des%20d%C3%A9cideurs%20locaux%20et%20de%20leurs%20partenaires.pdf
- **Garde pendant le trajet** : l'organisateur (la collectivité) a la garde à la montée, à la descente et aux arrêts (CE, 30 mai 1986, *époux Faix* ; avis CE 7 avril 1987), selon le même guide [S] ; aux abords, règles définies par l'organisateur « en liaison » avec le chef d'établissement (circulaires de 1984 et 1995 citées par le guide) [S]. **Retard de car** : aucune règle nationale ; l'élève est sous la responsabilité de l'établissement dès son entrée, selon le règlement intérieur [NV].
- **Conséquence** : ScolIA ne gère pas les abonnements ; il **atteste, reçoit, relaie, prévient** (§14) ; l'exercice d'évacuation de car est une **bonne pratique traçable**, pas une obligation affichée (PSC-216, réglage S-4313). Le cas rare d'un **établissement organisateur délégué** (L3111-9) est hors V1 (Q-PS8).

### 5.2 Association sportive [V]
- **C. éduc. L552-1** : activités physiques volontaires organisées par les associations sportives scolaires ; « tout élève apte à l'éducation physique et sportive est réputé apte » à ces activités. **L552-2 I** (version du 04/03/2022) : « une association sportive est créée dans tous les établissements du second degré » ; statuts à dispositions obligatoires fixées par décret. **L552-3** : affiliation obligatoire à une fédération ou union sportive scolaire. **L552-4** : Code du sport applicable, sauf L231-2 et L231-2-1. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006166652/
- **R552-2** (décret 2009-553) : affiliation à l'**UNSS** ; comité directeur **présidé par le chef d'établissement**, président de l'association ; composition par tiers (collèges, LP) ou quarts/moitié (lycées : moitié d'élèves) ; **secrétaire : un professeur d'EPS** ; **trésorier majeur** ; élèves membres **titulaires de la licence UNSS**. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000020663482/
- **Forfait de 3 heures** des enseignants d'EPS : décret **2014-460 du 7 mai 2014** (art. 3 modifié par décret 2019-1554) ; remplaçable par des heures d'enseignement sur demande au recteur avant le 15 février. https://www.legifrance.gouv.fr/loda/id/JORFTEXT000028909876
- **Montant de licence et de cotisation** : aucun montant national ; fixés par l'AS et l'UNSS [NV].
- **Privé sous contrat** : R552-2 vise l'UNSS ; aucune règle équivalente trouvée ; l'**UGSEL** est citée par le ministère comme fédération sportive scolaire [S].
- **Conséquences** : (a) l'aptitude à l'EPS vaut aptitude à l'AS : **aucun certificat médical** à collecter par défaut ; l'inaptitude totale (dispense d'EPS, socle 21 ou 34) est une **consigne** « inapte » sans motif (R-PS20) ; (b) président, secrétaire, trésorier majeur et comité directeur sont des **rôles de l'entité AS** (§11.1), le trésorier élève est refusé s'il est mineur ; (c) seuls les licenciés sont membres élèves.

### 5.3 Maison des lycéens, foyer, intervenants, Devoirs faits [S, V]
- **Maison des lycéens** : circulaire **2010-009 du 29 janvier 2010** (BO spécial n° 1 du 4 février 2010) : association loi 1901 dont le siège est dans le lycée mais qui « n'est pas un service du lycée », dirigée par des lycéens ; remplace le foyer socio-éducatif en lycée [S]. https://www.education.gouv.fr/bo/2010/special01/mene1002839c.htm. Les foyers socio-éducatifs subsistent en collège, sans texte récent trouvé [NV]. Un mineur peut participer à la direction d'une association (loi de 1901, art. 2 bis, loi 2017-86) [S].
- **Honorabilité** : Code de procédure pénale **706-53-7** : consultation du FIJAIS pour les activités impliquant un contact avec des mineurs (modifié par la loi 2024-317) [V] ; Légifrance l'affiche comme **abrogé par l'ordonnance 2025-1091 du 19 novembre 2025 avec entrée en vigueur différée** : texte successeur à suivre [V, à suivre]. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071154/LEGISCTA000006151994/2026-11-11. **Aucune loi promulguée** ne généralise le contrôle à tous les intervenants des établissements ; plusieurs textes sont en discussion (contrôle avant l'activité puis tous les trois ans) [S]. En pratique, des DSDEN font contrôler les accompagnateurs bénévoles (FIJAISV) [S]. Secteur sportif : contrôle annuel (loi du 8 mars 2024) [S].
- **Conséquence** : la garde d'honorabilité (S-4310) est **recommandée active** pour les intervenants et bénévoles, sans en faire une obligation affichée ; ScolIA enregistre une date et un résultat binaire (6, PRS-143), jamais le contenu ; le réglage sera réexaminé quand la loi sera votée (Q-PS9).
- **Devoirs faits** : note de service **BO n° 16 du 20 avril 2023** (MENE2302487N) : dispositif **obligatoire pour tous les élèves de 6e** depuis la rentrée 2023 ; volume et organisation fixés par le chef d'établissement [S]. https://www.education.gouv.fr/bo/2023/Hebdo16/MENE2302487N. **Conséquence** : en 6e (public), l'inscription à Devoirs faits est **par l'établissement**, pas facultative (famille `accompagnement`, nature `devoirs_faits`, PSC-125) ; gratuite.
- **Études surveillées** : aucun texte national trouvé ; elles relèvent du règlement intérieur [NV].

### 5.4 Périscolaire du premier degré (pour le socle 50) [V]
- **CASF R227-16** : accueil de loisirs périscolaire : 1 animateur pour 8 (moins de 6 ans, plus de 5 h) ou 10 (5 h ou moins) ; 1 pour 12 ou 14 (6 ans et plus) ; assouplissement dans un **projet éducatif territorial** (1 pour 10/14 et 14/18). **R227-15** : extrascolaire 1 pour 8 et 1 pour 12. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006074069/LEGISCTA000006196005/. L227-4 et L551-1 non relus [NV].
- **Conséquence** : un **contrôle de taux d'encadrement** sur la garderie, actif seulement pour le premier degré ou si l'accueil est déclaré en accueil collectif de mineurs (R-PS34, ajout personas ; socle 50).

### 5.5 Locaux, neutralité, données
- **Mise à disposition des locaux à des tiers** (club extérieur qui utilise le gymnase le soir) : **C. éduc. L213-2-2** (collèges, président du conseil départemental) et **L214-6-2** (lycées, région) : hors besoins de formation, avis du CA, **convention tripartite obligatoire** (collectivité, établissement, organisateur) ; **L212-15** pour les écoles (maire). Privé sous contrat : non applicable (contrat de l'organisme de gestion) [hypothèse]. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006182383/ — **Conséquence** : une activité d'un **tiers** (pas de l'établissement ni de ses associations) dans les locaux exige une convention (7) ; le 43 refuse de publier une telle activité sans convention (R-PS35, ajout personas).
- **Neutralité commerciale** (public) : déjà vérifiée au 17 (RG-5, circulaire 2001-053).
- **Acte usuel et autorité parentale** (C. civ. 372-2, présomption d'accord de l'autre parent pour les actes usuels) : non relu dans cette recherche ; repris des conceptions 4 et 42 [NV ici].
- **Covoiturage** : **aucune recommandation CNIL spécifique** ; principes : minimisation (RGPD 5.1.c), localisation remplaçable par une **saisie manuelle de code postal**, risque d'inférence du domicile (CNIL, géolocalisation des applications mobiles). https://www.cnil.fr/fr/geolocalisation-applications-mobiles-quelles-regles. Organiser un transport n'entre pas dans les compétences de l'établissement hors délégation : ScolIA se limite à une **mise en relation** à l'initiative des familles, avec avertissement [hypothèse, **à faire valider par le DPO**].
- **Durées de conservation** : instruction de tri **2005-003 du 22 février 2005** [S], sans durée propre aux activités, transports ou covoiturage : durées par finalité à fixer avec le DPO (§21.3).

### 5.6 Ce qui reste à vérifier [NV]
1. Texte successeur de l'article 706-53-7 du CPP (ordonnance 2025-1091) et sort des propositions de loi sur l'honorabilité.
2. C. éduc. R511-9 et R511-10 (base de la maison des lycéens).
3. Statut des foyers socio-éducatifs en collège.
4. Circulaire « surveillance des élèves » 96-248 (responsabilité aux abords, retard de car).
5. Règlement de transport de la région de l'établissement pilote (format des fichiers, attestation demandée).
6. Règles UNSS de saisie des licences (plateforme, export accepté) et règles UGSEL.


## 6. Frontières et branchements sur le cœur

### 6.1 Qui possède quoi

| Objet | Propriétaire | Le 43 |
|---|---|---|
| Élève, scolarité, régime, départ | 2 | lit ; réagit (§6.4) |
| Titulaires, payeurs, interdictions de remise, personnes autorisées, grille de sortie (R-A43) | 4 | lit ; demande l'accord des bons titulaires |
| Groupe de nature *activité*, sa capacité, son année | 5 (R-S15, STR-53) | crée le groupe **par la commande du 5** à l'ouverture d'une activité régulière ; possède l'offre, l'inscription, les séances |
| Salles, lieux extérieurs (gymnase municipal, stade) | 5 | lit ; réserve **par le 19** |
| Réservation de salles | **19** (§14, Q-AG1) | demande la réservation des séances en série |
| Programmes d'interventions par classe (EVARS, prévention) | **19** (AG6b, Q-AG2) | rien |
| Sessions ponctuelles à places (réunion d'information, atelier unique) | 19 (AG6) | utilise une session du 19 pour la réunion d'information d'une offre |
| Encadrants : agents, intervenants, bénévoles, honorabilité, forfait AS des enseignants d'EPS | 6 | lit ; affecte un encadrant à une activité |
| Pièces, conventions, signatures | 7 | rattache |
| Avis, audiences (« usagers d'une ligne », « inscrits d'une activité ») | 8 | déclare ses natures et audiences |
| Échanges avec l'autorité organisatrice des transports, formats d'import | 9 (fichiers) | lit le résultat de l'import ; possède lignes, arrêts, affiliations |
| Séances de cours, fin des cours, plages libres | 10 | lit (contrôle du dernier car, conflit d'horaire) ; publie `transport.inscriptions_modifiees` lu par le 10 (CL-67) |
| Appel, absences, retards | 12 | **consomme le composant d'appel** ; publie les écarts ; demande la justification en lot des retards de car |
| Retenue, sanction | 13 | expose « dernier car » à la pose d'une retenue (R-PS22) |
| Sortie de l'établissement, remise de l'enfant, présents | 14 | lit le régime de sortie ; **expose les présents hors temps scolaire** (ACS-338) |
| Message à un groupe d'inscrits | 16 | fil d'activité |
| Information, catalogue publié | 17 | publie l'offre comme une information ciblée |
| Autorisations (sortie seul, transport vers un lieu extérieur, image, soins) | **18** (port `autorisations::verifier`, FRM-249) | demande ; ne stocke pas le consentement |
| Facturation, payeur, aides, encaissement | 24, 25, 26 | transmet des **quantités** (inscription, séances) jamais des montants ; entité émettrice de l'association |
| Consignes de santé, PAI | 34 | lit une **consigne** (« protocole : voir trousse ») ; jamais un diagnostic |
| Besoin d'accompagnement (AESH) | 35 | lit un besoin codé, jamais le motif |
| Déplacements de l'AS hors de l'établissement | **42** (sortie collective simplifiée) | crée une **rencontre** ; pour un déplacement, **demande** au 42 une sortie de nature `rencontre_sportive` (§11.3) |
| Matériel de l'AS, casiers, clés du foyer | 44 | demande un prêt ou une réservation de matériel |
| Indicateurs | 47 | expose les siens |
| Comptes de l'association | 48 | export ; pas de comptabilité de l'association dans V1 (Q-PS2) |

### 6.2 Ports consommés

| Port | Socle | Usage |
|---|---|---|
| `eleve::scolarite_a(date)`, `eleve::regime_a(date)` | 2 | ciblage, conditions, fin d'inscription au départ |
| `administratif::titulaires(eleve)`, `::payeurs(eleve, categorie)`, `::interdictions(eleve)`, `::personnes_autorisees(eleve)`, `::grille_sortie(eleve, jour)` | 4 | qui peut inscrire, qui paie, qui vient chercher |
| `structure::groupe_creer(nature='activite', capacite)`, `structure_disponibilite::disponible` | 5 | groupe d'activité ; lieu ouvert |
| `agenda::reserver_serie(lieu, regle, occurrences)` | 19 | salle des séances |
| `personnels::encadrant(personne, date)`, `::honorabilite_valide(personne, date)`, `::forfait_as(agent)` | 6 | encadrement et garde d'honorabilité |
| `edt::occupe(eleve, plage)`, `edt::fin_des_cours(eleve, jour)` | 10 | conflit avec un cours ; dernier car |
| `assiduite::composant_appel(seance_activite)` | 12 | appel |
| `assiduite::justifier_lot(eleves, plage, motif='transport')` | 12 | retards de car |
| `accueil::regime_sortie(eleve, jour)`, `accueil::remettre(eleve, personne)` | 14 | départ de l'étude, de la garderie |
| `autorisations::verifier(sujets, objet, usage, date)`, `formulaires::creer_pour(...)` | 18 | autorisations |
| `facturation::engagement(source, payeur, quantite, du, au)` | 24 | facturation (F-lot du 24) |
| `sante::consignes(eleve, contexte='activite')` | 34 | consigne minimale |

### 6.3 Ports exposés par le 43

| Port | Consommateurs | Contenu |
|---|---|---|
| `periscolaire::attendus(plage)` | 14 (présents hors temps scolaire), 41 (activités du soir des internes), 12 | élèves attendus en activité, lieu, encadrant |
| `periscolaire::inscriptions(eleve, date)` | 2 (fiche 360°), 29 (« Aujourd'hui »), 40 (goûter de garderie si besoin) | activités de l'élève |
| `transport::usagers(ligne, jour, sens)` | 8 (audience), 12, 14 | élèves affiliés |
| `transport::dernier_depart(eleve, jour)` | 10, 13, 19 | heure du dernier car de l'élève |
| `transport::eleves_par_car(depart)` | 14 (évacuation, accident), 42 | liste minimale |
| `periscolaire::participation(eleve, annee)` | 23 (bulletin : engagement), 37 (orientation, Parcoursup « activités et centres d'intérêt » [NV]) | activités suivies, rôles (jeune officiel, délégué du foyer) |

### 6.4 Répercussions automatiques (le cœur bouge, le 43 suit)

| Événement du cœur | Effet |
|---|---|
| `eleve.depart` (2) | inscriptions closes à la date de départ ; séances futures retirées ; facturation arrêtée (24) ; affiliation de transport close ; avis au responsable de l'activité |
| `eleve.regime_changed` (2) | conditions revérifiées (activité réservée aux demi-pensionnaires, aux internes) ; inscription signalée, jamais close sans décision |
| changement de classe (5) | ciblage revérifié ; l'inscription reste (une activité ne dépend pas de la classe) sauf condition de niveau |
| titulaire ajouté, retiré (4) | les autorisations attachées restent valides (18) ; un nouvel avis est envoyé au nouveau titulaire |
| interdiction de remise nouvelle (4) | listes de départ de l'étude et de la garderie mises à jour immédiatement (14) |
| payeur changé (4) | engagements futurs portés au nouveau payeur ; passé inchangé (24) |
| majorité de l'élève (2) | les consentements passent à l'élève (18) ; ses parents ne voient plus l'inscription sans son accord (4) |
| salle fermée, indisponible (5) ; réservation en conflit (19) | séances touchées signalées au responsable ; salle de repli proposée |
| fermeture de l'établissement, jour férié, vacances (5) | séances neutralisées, aucune présence attendue, aucune facturation à la séance |
| sortie collective (42), stage (39), voyage | l'élève est « en sortie » à la séance : ni absent ni attendu |
| encadrant absent (15) | séance « sans encadrant » : remplacement proposé ou annulation avec avis (R-PS15) |
| honorabilité expirée (6) | l'intervenant n'est plus planifiable ; séances futures signalées |
| nouvelle consigne de santé (34) | la consigne s'affiche dans l'appel de l'encadrant (sans motif) |

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]

| Terme | Définition |
|---|---|
| **Activité** | offre de l'année : un intitulé, une **famille** (§8.1), une **entité organisatrice** (établissement, AS, foyer, maison des lycéens, APEL, partenaire), un public, un rythme, des places, un prix |
| **Session d'activité** | période d'une activité (année, trimestre, semestre, cycle de 6 séances) ; on s'inscrit à une session |
| **Séance d'activité** | occurrence datée (début, fin, lieu, encadrant) ; appelée |
| **Inscription** | engagement daté d'un élève à une session, avec son état et ses faits |
| **Présence attendue** | à une séance, l'élève inscrit, non excusé, non en sortie ; son absence est un **écart** |
| **Encadrant** | personne responsable de la séance (agent, intervenant, bénévole) ; un **responsable** d'activité porte l'activité sur l'année |
| **Rencontre** | événement de l'AS (compétition, entraînement commun) avec convocation ; hors de l'établissement, elle s'adosse à une sortie du 42 |
| **Ligne** | ligne de transport scolaire (code de l'autorité organisatrice), ses **arrêts** et ses **courses** (départs datés par sens) |
| **Affiliation** | lien daté d'un élève à une ligne, un arrêt, un sens, des jours ; importé ou déclaré |
| **Transport adapté** | transport individuel d'un élève en situation de handicap, organisé par le département (ou la région, Île-de-France Mobilités) : horaire, prestataire, contact |
| **Incident de transport** | course supprimée, en retard, déviée ; avis et conséquences (§14.4) |
| **Covoiturage** | mise en relation de familles volontaires d'une même zone ; l'établissement n'organise pas de transport |

### 7.2 Distinctions qui manquent aujourd'hui [R]

1. **Responsabilité de l'établissement ou de l'association.** Une étude surveillée, une garderie, un club animé par un agent sont **des activités de l'établissement** : il en répond. L'AS, le foyer, la maison des lycéens sont **des associations** : l'activité se déroule souvent dans les murs, encadrée par des personnels, mais l'organisateur et l'assureur sont l'association (la maison des lycéens « n'est pas un service du lycée », §5.3 ; l'AS est une association présidée par le chef d'établissement, §5.2). ScolIA porte l'**entité organisatrice** sur chaque activité ; elle conditionne l'entité émettrice des factures, l'assurance mentionnée et le texte des autorisations.
2. **Activité ponctuelle ou régulière.** Une conférence, une réunion d'information, un atelier unique sont des **sessions du 19** (AG6) ; une activité **régulière** (club hebdomadaire, étude, AS) est au 43. Critère : plus de deux séances avec les mêmes inscrits.
3. **Absent à l'étude, absent au cours.** L'étude ou la garderie inscrites créent une présence attendue **hors cours** : l'écart est signalé, mais ce n'est pas une absence scolaire du 12 (elle n'entre pas dans le décompte d'assiduité). Exception : l'étude **obligatoire** placée dans l'emploi du temps (heure de permanence) reste au 12.
4. **Affiliation importée ou déclarée.** L'affiliation importée fait foi pour l'année (source : autorité organisatrice) ; la famille peut **déclarer** « mon enfant prend la ligne 12 » si l'import n'existe pas ; les deux sont marquées et ne se mélangent pas.

---

## 8. Offre d'activités

### 8.1 Familles d'activités (codées, [R])

| Code | Famille | Organisateur par défaut | Particularités |
|---|---|---|---|
| `as` | Association sportive | AS (association) | licence, rencontres, forfait EPS (§11) |
| `club` | Club, atelier hebdomadaire (théâtre, échecs, robotique, chorale, journal) | établissement ou foyer | |
| `etude` | Étude surveillée du soir | établissement | présence attendue, sortie vérifiée (§12) |
| `garderie` | Garderie du matin ou du soir (surtout écoles et collèges d'ensembles scolaires) | établissement | idem ; âge |
| `accompagnement` | Aide aux devoirs, « Devoirs faits », tutorat, soutien | établissement | souvent gratuit, ciblé sur proposition (§12.3) |
| `midi` | Activité de la pause méridienne | établissement ou foyer | articulée avec le service de restauration (40) |
| `partenaire` | Offre d'un partenaire (certification de langue, école de musique, club sportif local) | partenaire, par convention | neutralité commerciale (RG-5 du 17) ; inscription et paiement souvent chez le partenaire |
| `adhesion` | Adhésion à une association (foyer, maison des lycéens, AS sans activité précise) | association | cotisation |
| `vacances` | Stage ou activité pendant les vacances | établissement ou association | calendrier hors période scolaire |

### 8.2 Créer et publier une activité (reprend NP-01, NP-02)
- **Brouillon** : intitulé, famille, organisateur, responsable (personne du 6), description, public cible (niveaux, régimes, âge min/max, site), sessions (période, rythme hebdomadaire, horaire, lieu souhaité), places (maximum, minimum pour ouvrir), mode d'attribution (premier arrivé, tirage au sort, sur proposition, sur dossier), prix (du barème du 24 : forfait, à la séance, gratuit), autorisations exigées (codes du 18), pièces exigées (aucune par défaut), texte d'engagement.
- **Contrôles** à la publication : responsable valide à la date ; encadrants avec honorabilité valide si exigée (§13) ; salle réservable ou lieu extérieur ouvert ; public non vide ; prix gratuit pour Devoirs faits (§5.3) ; aucune donnée d'élève dans la description.
- **Autorisation** : dans le public, une activité nouvelle avec intervenant extérieur est **autorisée** par le chef d'établissement (document du 7, §5) ; dans le privé, par la direction (réglage S-4300).
- **Publication** : visible des familles et des élèves ciblés dans l'appli ; information du 17 (catalogue) ; la vitrine publique ne montre que intitulé, famille, description, public et réunion d'information, sans liste ni formulaire (R-PS10).
- **Reconduction** d'une année sur l'autre (PSC-103) : copie de l'activité, des sessions et du responsable ; ni inscrits ni pièces ; inscrits de l'année précédente **invités** à se réinscrire si le réglage le prévoit (S-4304).

### 8.3 Forum des activités et catalogue
- Une **vue catalogue** par famille et par jour de la semaine, filtrée sur ce que l'élève peut suivre (niveau, régime, site, **pas de conflit avec son emploi du temps**, §9.3).
- Une **réunion d'information** par offre (NP-03) est une session du 19, avec inscription facultative.
- Un **mode forum** : sur tablette à l'entrée, l'élève scanne sa carte (si l'établissement a des badges, 40) pour **marquer son intérêt**, jamais pour s'inscrire seul s'il est mineur et l'activité payante.

---

## 9. Inscriptions

### 9.1 Qui inscrit (R-PS1 à R-PS4)
- **Élève mineur, activité gratuite sans déplacement** : un **titulaire de l'autorité parentale** suffit (acte usuel, présomption d'accord de l'autre, C. civ. art. 372-2 [NV ici, §5.5]) ; l'élève peut **demander** l'inscription, le titulaire **confirme**.
- **Élève mineur, activité payante** : le **payeur** de la catégorie (24, 4) s'engage ; si le payeur n'est pas celui qui inscrit, l'inscription est **en attente du payeur** (avis), sauf si l'inscrivant accepte de payer lui-même (devient payeur pour cette ligne).
- **Désaccord connu** entre titulaires (marqué au 4) : l'inscription attend l'accord **des deux** (R-PS3).
- **Élève majeur** : il s'inscrit seul ; ses parents sont informés **seulement** s'il a accepté le partage (4).
- **Inscription par l'établissement** (accompagnement « sur proposition », étude imposée par une mesure éducative du 13) : le personnel inscrit ; les titulaires sont **avisés** ; pour une activité payante, jamais sans l'accord du payeur.

### 9.2 Parcours (reprend NP-07, NP-08 ; corrige DPS-01, DPS-02, DPS-06)
1. La famille ouvre l'activité depuis le catalogue ; l'élève et ses données viennent du cœur (aucune ressaisie : ni nom, ni classe, ni date de naissance).
2. Elle choisit la session (et le jour si l'activité en propose plusieurs).
3. ScolIA vérifie : public, places, conflit horaire (§9.3), autorisations exigées (proposées **dans le même geste** au 18), pièces, payeur.
4. **Engagement** affiché (texte de l'offre, prix, règles de désistement) ; validation par passkey si payant (D-009, réauthentification récente ★).
5. **État** : `demandee` → `confirmee` (places, mode « premier arrivé ») ou `en_attente_tirage` ou `liste_attente` ou `en_attente_accord` (payeur, second titulaire) ou `a_valider` (dossier, partenaire).
6. Avis à l'inscrivant, à l'autre titulaire (information), au responsable (récapitulatif quotidien, pas un courriel par inscription).

### 9.3 Conflits et conditions
- **Conflit avec l'emploi du temps** (port `edt::occupe`) : refus pour un cours obligatoire ; avertissement pour une heure de permanence.
- **Conflit avec une autre activité** au même moment : refus (deux présences attendues au même endroit sont impossibles).
- **Conflit avec le dernier car** (§14) : avertissement « la séance finit à 18 h, le dernier car de la ligne 12 part à 17 h 40 » ; l'inscription reste possible (la famille peut venir chercher).
- **Conditions** codées : niveau, régime, âge, site ; une condition non remplie n'est jamais contournée en silence (dérogation par le responsable, motivée, tracée, PSC-127).

### 9.4 Places, liste d'attente, tirage au sort (R-PS6 à R-PS8)
- Places décomptées **en transaction** (verrou de ligne sur la session) ; inscriptions `confirmee` et `en_attente_accord` comptées.
- **Premier arrivé** : ordre d'horodatage serveur ; à égalité, ordre aléatoire tiré une fois.
- **Tirage au sort** à la fermeture de la fenêtre : graine tirée par le serveur, **publiée** (empreinte) avant le tirage et révélée après ; résultat **rejouable** ; priorités déclarées à l'avance (élèves non servis l'an passé, internes pour une activité du soir, fratrie dans la même activité) sans critère révélateur (jamais boursier, jamais handicap comme priorité affichée ; un aménagement passe par une **place réservée** décidée par la direction, PSC-131).
- **Liste d'attente** ordonnée ; une place libérée est **proposée** au premier, avec un délai de réponse (48 h par défaut, S-4305) ; sans réponse, au suivant.
- **Minimum d'inscrits** non atteint à la date limite : la session est **annulée** ou **maintenue** par décision du responsable ; avis aux inscrits.

### 9.5 Vie de l'inscription
- **Changement de jour** ou de session (trimestre suivant) : fait daté, places vérifiées.
- **Désistement** par la famille : avant la date limite, sans frais ; après, selon la règle de l'offre (forfait dû, ou prorata) ; motif facultatif, jamais exigé pour une raison de santé.
- **Exclusion** d'une activité (comportement) : décision du responsable de l'activité **et** de la direction, motivée, avis aux titulaires ; si c'est une sanction, elle passe par le 13 (le 43 enregistre seulement la fin d'inscription et sa cause « décision disciplinaire », sans détail).
- **Absences répétées** (3 écarts non justifiés par défaut, S-4306) : alerte au responsable et à la vie scolaire, proposition de contact avec la famille ; jamais de désinscription automatique.

---

## 10. Séances, appel et présence

### 10.1 Calendrier des séances
- Générées depuis le rythme de la session et le **calendrier du 5** (vacances, jours fériés, fermetures sautés) ; chaque séance a un lieu (réservation du 19) et un encadrant.
- Séance **annulée** (encadrant absent, salle indisponible, intempéries) : motif codé, avis aux inscrits et aux familles **avant l'heure de fin des cours** (R-PS15) ; pour une étude ou une garderie, une séance n'est jamais annulée sans solution d'accueil ou avis des familles au moins la veille, sauf urgence (direction, ★).
- Séance **ajoutée** (rattrapage) ou **déplacée** : places et conflits revérifiés.

### 10.2 Appel (composant du 12)
- Le composant `assiduite::composant_appel` fournit la liste : **inscrits attendus**, déjà marqués `en_sortie` (42), `stage` (39), `infirmerie` (34, présence seulement), `absent_annonce` (famille), `excuse` ; l'encadrant pointe présent, absent, retard, **parti** (avec l'heure et la personne si remis, §12).
- **Hors ligne** sur le mobile (gymnase, stade), avec synchronisation idempotente (12).
- Un **élève non inscrit** présent est ajouté comme « invité » (pas d'engagement, pas de facture), pour la sécurité.
- **Écart** (inscrit attendu absent, non annoncé) : dans les **10 minutes** (S-4307) après le début, alerte à la vie scolaire (et au CPE d'astreinte pour une activité du soir des internes, 41) ; la vie scolaire décide de contacter la famille (avis du 8, modèle prêt, envoi N3).
- **Validation** de l'appel à la fin de la séance ; correction motivée après validation (journal).

### 10.3 Présents hors temps scolaire (14, ACS-338)
- `periscolaire::attendus(plage)` alimente la liste des **présents dans l'établissement** après les cours : par lieu, avec l'encadrant ; utilisée par l'accueil, en cas d'alerte (PPMS) ou d'évacuation.
- À la fin de la séance, chaque élève est **parti** (seul, avec autorisation ; remis à une personne autorisée ; pris en charge par le car ; rejoint l'internat) ; un élève non sorti à l'heure de fermeture est signalé (R-PS18).

---

## 11. Association sportive, foyer, maison des lycéens

### 11.1 L'association sportive (AS)
- **Entité** : l'AS est une association obligatoire dans le second degré public, affiliée à l'UNSS, présidée par le chef d'établissement, secrétaire professeur d'EPS, trésorier majeur (§5.2) ; ScolIA enregistre l'entité (`ent_periscolaire_entite`) avec ses responsables, son numéro d'affiliation (UNSS, UGSEL) et ses coordonnées bancaires éventuelles (au 25, jamais dans le 43).
- **Activités** de l'AS : une activité par sport et par créneau (famille `as`) ; l'élève **adhère** à l'AS (licence) et s'inscrit à une ou plusieurs activités.
- **Licence** : numéro de licence et date de saisie, **saisis ou importés** depuis la plateforme de la fédération (OPUSS pour l'UNSS [NV]) ; ScolIA n'est pas la plateforme de licences : il **exporte** la liste des adhérents au format attendu (§14 du 9 pour les formats) et **réimporte** les numéros.
- **Aptitude** : « tout élève apte à l'EPS est réputé apte » à l'AS (L552-1, §5.2) : **aucun certificat médical** n'est collecté par défaut ; une inaptitude connue est une **consigne** « inapte à l'AS » sans motif (34) ; si une fédération demande une attestation pour une activité particulière, le 43 enregistre seulement « attestation fournie le » (le document éventuel va au 34).
- **Forfait d'AS des enseignants d'EPS** (3 heures, décret 2014-460, §5.2) : lu au 6 (service) ; le 43 montre les créneaux assurés et les encadrants, jamais une évaluation des personnels.
- **Rencontres** (§11.3), **jeunes officiels** (arbitres, juges, reporters) : rôle daté sur l'adhésion, attestable (PSC-186).
- **Assemblée générale** et **bilan** : effectifs par sexe, niveau et activité, participation aux rencontres, résultats ; brouillon par l'IA, relu (PSC-189).

### 11.2 Foyer socio-éducatif, maison des lycéens, APEL
- Même modèle d'**entité organisatrice** : adhésions (cotisation), activités, responsables élèves (élus de la maison des lycéens, socle 46 pour les élections). La maison des lycéens est dirigée par les lycéens eux-mêmes (§5.3) : des **élèves majeurs** ou mineurs élus peuvent être **responsables d'activité** dans ScolIA, avec un droit restreint (liste de leurs inscrits, sans dossier, sans données de santé ni coordonnées).
- L'APEL, l'association de parents, n'organise pas d'activité dans V1 ; elle est une entité possible pour le financement (42) [R].

### 11.3 Rencontres et déplacements (corrige l'absence totale aujourd'hui)
- **Rencontre** : date, lieu, sport, catégorie, équipe convoquée (sélection parmi les adhérents de l'activité), heure de départ et de retour, encadrants.
- **Autorisation annuelle** de l'AS (18) couvre les déplacements aux rencontres ; à défaut, une autorisation par rencontre.
- **Dans l'établissement** : la rencontre est une séance particulière, appelée.
- **Hors de l'établissement** : le 43 **demande au 42** une sortie de nature `rencontre_sportive` (circuit réduit : pas de budget familles, pas de vote, transport par car UNSS ou par l'AS) ; l'appel de départ et de retour, la remise au retour (14), la fiche d'urgence (34) passent par le 42. Le 43 garde la **convocation** et le **résultat**.
- **Convocation** : avis aux convoqués et à leurs titulaires (8), avec heure, lieu, retour prévu, tenue ; **accusé de lecture** demandé ; relance des non-lus la veille.
- **Retour retardé** : avis aux familles des convoqués depuis le mobile de l'encadrant (8, envoi N3 avec modèle).
- **Résultats** : saisis par l'encadrant, publiés aux adhérents ; jamais de classement nominatif public par défaut (S-4309).

---

## 12. Études, garderie, accompagnement

### 12.1 Études et garderie : engagements datés (corrige DPS-16, RPS-05 ; reprend NP-17)
- Une étude ou une garderie est une activité de famille `etude` ou `garderie`, avec des **créneaux** par jour (lundi à vendredi, matin ou soir) ; l'inscription précise **les jours** (pas forcément tous) et l'**heure de départ** prévue si elle varie.
- **Saisie rapide** du secrétariat : la vue hebdomadaire de la grille Next.js (cases par jour) est conservée ; chaque case cochée crée ou clôt une inscription **datée**.
- **Ponctuel** : « exceptionnellement mardi » = inscription d'un jour (garderie occasionnelle), facturée à la séance si le barème le prévoit.

### 12.2 Départ vérifié (14 ; corrige « sort seul » en texte)
- Le **régime de sortie** de l'élève (14) dit, pour chaque jour : sortie seul autorisée à partir de quelle heure, ou remise à une personne autorisée (4).
- À la fin de l'étude, l'encadrant voit pour chaque élève : **part seul** (autorisation valide) ; **attend** (personne autorisée attendue, liste des noms) ; **car** (ligne, heure). La remise à une personne passe par le geste de remise du 14 (vérification d'identité si inconnue, interdiction de remise bloquante).
- Une famille qui annonce « Lucas sortira seul exceptionnellement à 17 h » le fait par l'appli (titulaire identifié) ; jamais par un message libre de l'élève.

### 12.3 Accompagnement éducatif, « Devoirs faits », tutorat
- Activité `accompagnement` **gratuite** ; inscription **sur proposition** (professeur principal, conseil de classe) ou volontaire ; **Devoirs faits est obligatoire pour tous les élèves de 6e** depuis 2023 [S, §5.3] : inscription par l'établissement.
- **Liste proposée** par l'IA ou par règle (élèves avec travaux non rendus nombreux au 20, demandes des familles) : **proposée**, jamais imposée ; le professeur principal valide (N2), la famille est avisée.
- Les **intervenants** (professeurs volontaires, AED, assistants d'éducation, service civique, bénévoles) sont des encadrants du 6 ; leurs heures effectuées sont **comptées** et transmises au 15 ou au 49 (heures supplémentaires effectives) comme des **quantités**.
- **Suivi** : l'encadrant note des **faits codés** (« travail fait », « leçon revue », « à reprendre ») ; aucune appréciation libre sur la personne ; consultables par le professeur principal et la famille.

---

## 13. Partenaires et intervenants

### 13.1 Partenaires (reprend NP-01 à NP-10)
- **Partenaire** : organisme (raison sociale, SIREN, contact, convention signée au 7, assurance déclarée), partagé avec l'annuaire des prestataires (42, 48) [R].
- **Offre de partenaire** : famille `partenaire` ; deux modes : **information** (l'inscription et le paiement se font chez le partenaire, l'établissement relaie) ou **inscription** par ScolIA (l'établissement collecte les inscriptions pour le partenaire, le paiement reste chez le partenaire ou passe par le 25 sous l'entité du partenaire si une convention le prévoit, Q-PS6).
- **Neutralité commerciale** (public, RG-5 du 17) : une offre de partenaire porte une mention « proposée par » et un lien vers la convention ; pas de publicité ; la direction valide la publication (N2).
- **Transmission au partenaire** : la liste des inscrits est transmise **minimale** (nom, prénom, niveau, contact du titulaire qui a inscrit), en export ou par accès partenaire limité (§21) ; date de naissance seulement si le partenaire la justifie (assurance, licence) et que l'offre l'annonce.

### 13.2 Intervenants extérieurs et bénévoles
- Personne du 6 (P60), **engagement** daté (`intervenant`, `benevole`), convention avec son organisme le cas échéant, **vérification d'honorabilité** datée (PRS-143) pour ceux qui encadrent des mineurs (recommandé, pas encore généralisé par la loi, §5.3) ; l'échéance bloque la planification si la direction l'active (S-4310, recommandé : actif).
- **Jamais seul** avec des élèves sans un personnel de l'établissement présent ou joignable, sauf autorisation de la direction pour l'activité (réglage par activité) [R].
- **Accès** : un compte personnel limité à ses activités et à ses séances ; il voit les **prénoms et noms** de ses inscrits, la photo si l'établissement l'active, les consignes minimales du 34, les personnes autorisées au départ ; il ne voit ni classe complète, ni dossier, ni coordonnées des familles (il passe par le fil d'activité du 16, modéré).

---

## 14. Transports scolaires

### 14.1 Ce que l'établissement fait et ne fait pas (§5.1)
- **Ne fait pas** : il n'organise pas les lignes, ne vend ni n'émet les abonnements, ne fixe pas les tarifs, ne décide pas du transport adapté.
- **Fait** : il atteste la scolarité (et le régime) pour l'inscription aux transports ; il reçoit et relaie les informations de l'autorité organisatrice ; il fixe ses horaires en tenant compte des cars ; il assure la surveillance aux abords selon son règlement [NV] ; il peut organiser avec l'organisateur les exercices d'évacuation de car recommandés (§5.1) ; il prévient les familles et adapte l'accueil en cas d'incident.

### 14.2 Lignes, arrêts, courses (import)
- **Import** d'un fichier de l'autorité organisatrice (CSV ou tableur, format décrit et versionné au 9 ; GTFS si l'autorité le publie [H]) : lignes (code, nom, transporteur), arrêts (nom, commune ; pas de coordonnées nécessaires), courses (sens aller/retour, jours, heure d'arrivée à l'établissement, heure de départ de l'établissement).
- **Aperçu avant écriture**, rejets ligne à ligne, idempotent, rien créé en silence (même règle que l'import de salles du 5, STR-08).
- **Saisie manuelle** possible pour un petit établissement sans fichier (une ligne, trois arrêts).
- **Versions** : un nouvel import crée une **version** datée ; les affiliations suivent les codes stables ; une course disparue est signalée.

### 14.3 Usagers (affiliations)
- **Import** de la liste des élèves abonnés (si l'autorité la fournit) : rapprochement par INE ou identifiant fourni, sinon par nom et date de naissance **proposé**, jamais appliqué automatiquement (même règle que la reprise, T0 §6) ; rejets en file humaine.
- **Déclaration par la famille** : « ligne 12, arrêt Mairie, matin et soir, lundi-mardi-jeudi-vendredi » ; marquée « déclarée » ; elle sert aux avis et aux contrôles, pas à une attestation.
- **Export** pour l'autorité : liste des élèves scolarisés demandant un abonnement (nom, prénom, date de naissance, classe ou niveau, régime, établissement) **seulement** pour ceux dont la famille a fait la demande, au format demandé, par un administratif (D-022), journalisé.
- **Transport adapté** (élève en situation de handicap) : prestataire, horaires, contact du transporteur ; **aucun motif** ; visible de la vie scolaire et de l'accueil pour la remise ; l'AESH est avisée si elle accompagne à l'arrivée.

### 14.4 Incidents de transport
- **Course supprimée ou en retard** (signalée par le transporteur, l'autorité ou un personnel) : un incident est créé (ligne, course, date, nature, source) ; ScolIA propose :
  - l'**avis** aux familles des **usagers de la course** (audience du 8, NTF-132, CN-67), dans la **fenêtre veille/matin** (R-N50 : jusqu'à 22 h 30 la veille, dès 6 h le jour même), modèle prêt, envoi N3 ;
  - la **justification en lot** des retards des usagers de la course ce jour-là (12, motif « transport », sur décision de la vie scolaire, N2) ;
  - l'information de l'accueil et de la vie scolaire (élèves attendus en retard) ;
  - pour un car du soir supprimé : liste des usagers encore présents, **maintien en étude** jusqu'à la prise en charge, avis aux familles.
- **Grève, intempéries** : incident **par ligne ou global** ; les retards et absences du jour peuvent être justifiés en lot par la direction.
- **Élève non monté** (signalé par le transporteur ou la famille) : procédure courte : vérifier sa présence (12, 14), prévenir la famille ; tracer.

### 14.5 Sécurité
- **Liste des élèves par car** (`transport::eleves_par_car`) : pour l'exercice d'évacuation de car et en cas d'accident ; prénom, nom, classe, consigne minimale du 34, contact d'urgence (seulement à la direction et pendant l'incident).
- **Exercice d'évacuation de car** : planifié, réalisé, participants pointés, compte rendu (document du 7) ; **recommandé** une fois par an pour chaque transporté (guide du CNT 2009), sans obligation nationale (§5.1).
- **Dernier car** : à la pose d'une retenue (13), d'une séance de rattrapage, d'une étude ou d'une activité, ScolIA affiche « dernier car à 17 h 40 » et demande une confirmation (la famille viendra-t-elle ?) ; la retenue après le dernier car exige un **avis à la famille** avant la date (R-PS22).
- **Fin des cours** : le 10 lit `transport::dernier_depart` pour signaler les séances qui finissent après le dernier car d'élèves concernés (CL-67).

---

## 15. Covoiturage (refonte de M20 ; reprend NP-11 à NP-16)

- **Désactivé par défaut** ; la direction l'active avec une **notice d'information** et le **texte de responsabilité** (l'établissement met en relation, n'organise ni ne contrôle les trajets) [R, §5].
- **Qui** : titulaires de l'autorité parentale (et, si l'établissement l'active, personnels entre eux, dans un espace séparé) ; jamais d'élève ; **aucune donnée d'enfant** n'est publiée (ni nom, ni classe : l'appariement porte sur la zone, l'établissement et les jours).
- **Zone** : une **commune** ou un code postal (un seul ; deux au plus avec justification), jamais d'adresse ni de position.
- **Consentement explicite** daté, retirable à tout moment ; **effacement réel** du profil et des correspondances à la désinscription et au 31 août (R-PS29).
- **Appariement** : même zone, même établissement (site), sens compatible (matin, soir), jours en commun ; un nombre plafonné de propositions par jour (S-4312) ; avis groupé (un par jour au plus).
- **Double acceptation** avant dévoilement ; dévoilement du **nom** et d'un **moyen de contact choisi** (message du 16, ou e-mail si la personne l'accepte) ; jamais le téléphone sans accord.
- **Modération** : signalement d'un comportement, suspension par la direction, journal ; aucun avis ni note sur les personnes.

---

## 16. Argent (socles 24, 25, 26)

1. **Quantités, pas montants** (règle de transmission du 24) : une inscription transmet un **engagement** (source `periscolaire:{session}:{eleve}`, payeur, quantité, période) ; une activité à la séance transmet les **séances présentes** (ou réservées, selon la règle de l'offre) chaque mois.
2. **Entité émettrice** : celle de l'organisateur (établissement, organisme de gestion, AS, foyer, partenaire) (D-027 F) ; deux entités ne partagent jamais une facture.
3. **Barème** : du 24 (forfait annuel, trimestriel, à la séance ; tarifs différenciés si le 24 le permet, jamais calculés par le 43).
4. **Aides** (26) : prise en charge d'une partie par un fonds social ou une aide de la collectivité ; le responsable de l'activité ne voit **jamais** qui est aidé.
5. **Aucun élève écarté pour raison financière** sur une activité de l'établissement (principe, R-PS24) : une demande d'aide est proposée à l'inscription si la famille le souhaite (lien vers le 26).
6. **Désistement, annulation** : avoir proposé selon la règle de l'offre ; annulation par l'établissement = avoir total.
7. **Pendant l'année de doublon** (D-034) : **mode suivi** — ScolIA calcule ce qui serait facturé, Charlemagne facture ; état de comparaison mensuel (comme 40 et 41).
8. **Cotisations des associations** : suivies (payée, non payée, date) dans V1 ; encaissement par le 25 sous l'entité de l'association seulement si Florian le décide (Q-PS2).

## 17. Cas limites (catalogue)

### 17.1 Inscriptions et familles
| CPS | Cas | Traitement |
|---|---|---|
| CPS-01 | Parents séparés : la mère inscrit au théâtre payant, le père est payeur de la catégorie « activités » | inscription `en_attente_accord` du payeur ; avis au père ; la mère peut payer elle-même (devient payeur de la ligne) |
| CPS-02 | Désaccord connu entre titulaires sur les activités | accord des deux exigé (R-PS3) ; le responsable voit « en attente d'accord », jamais la raison |
| CPS-03 | Un titulaire visé par une interdiction de remise inscrit l'enfant à la garderie du soir pour venir le chercher | l'inscription est possible (acte usuel) mais la remise reste **bloquée** au départ (14) ; la vie scolaire est avisée de l'inscription par une personne visée par une interdiction (R-PS4) |
| CPS-04 | L'élève de 17 ans s'inscrit lui-même à une activité payante | refus sans payeur : la demande part au payeur ; l'élève mineur ne s'engage pas financièrement |
| CPS-05 | L'élève devient majeur en cours d'année | les consentements passent à lui (18) ; les parents payeurs restent payeurs jusqu'à ce que l'élève devienne payeur (4) |
| CPS-06 | Deux enfants de la même fratrie sur liste d'attente | priorité « fratrie » si déclarée ; jamais de séparation automatique sans avis |
| CPS-07 | La famille veut inscrire à deux activités qui se chevauchent d'un quart d'heure | refus ; proposition de l'autre jour |
| CPS-08 | L'élève inscrit à l'étude change d'emploi du temps et finit désormais à 17 h au lieu de 16 h | l'inscription reste ; l'heure de début de la présence attendue suit la fin des cours (10) |
| CPS-09 | Inscription faite par un compte élève connecté sur le téléphone d'un parent | inscription **demandée** par l'élève, confirmée par le titulaire par passkey (R-PS2) |
| CPS-10 | Famille sans compte ScolIA (parent non francophone) | inscription par le secrétariat sur un formulaire papier signé, scanné (7) ; ou lien nominatif à usage limité (D-027 C) |
| CPS-11 | Tirage au sort contesté par une famille | graine publiée et résultat rejouable : la direction peut montrer le calcul |
| CPS-12 | Élève en situation de handicap inscrit à un club sans AESH disponible sur le créneau | besoin signalé (35), sans motif ; la direction décide de l'accompagnement ; jamais de refus automatique |
| CPS-13 | Élève placé à l'aide sociale à l'enfance : l'inscription est faite par le service gardien | le 4 porte le service comme représentant ; l'autorisation suit ses règles |

### 17.2 Séances, présence, départ
| CPS | Cas | Traitement |
|---|---|---|
| CPS-14 | L'encadrant de l'étude est absent sans prévenir à 16 h 55 | séance « sans encadrant » signalée à la vie scolaire à l'heure de début ; un AED est proposé ; jamais d'étude sans encadrant |
| CPS-15 | Un élève inscrit à l'étude est absent sans annonce | écart en 10 minutes ; la vie scolaire vérifie (était-il en cours à 15 h ?) ; avis à la famille sur décision |
| CPS-16 | Un élève non inscrit reste à l'étude | ajouté « invité », pour la sécurité ; ni facture ni engagement |
| CPS-17 | À 18 h 30 la garderie ferme, un enfant n'est pas récupéré | alerte à la vie scolaire et à la direction ; appel aux personnes autorisées ; procédure du 14 ; tracé |
| CPS-18 | La personne venue chercher l'enfant n'est pas dans la liste | remise refusée ; contact du titulaire ; ajout ponctuel par le titulaire dans l'appli (4) |
| CPS-19 | Séance au gymnase municipal sans réseau | appel hors ligne ; synchronisation au retour |
| CPS-20 | L'élève se blesse pendant le club | fait de santé au 34 (soins), déclaration d'accident (7) ; le 43 trace la séance et l'encadrant, rien de médical |
| CPS-21 | Séance annulée à 16 h pour une alerte météo | avis aux familles des inscrits (N3) ; les élèves déjà présents sont gardés jusqu'à l'heure de fin prévue |
| CPS-22 | Activité de midi qui empiète sur le passage à la cantine | le 40 voit les inscrits (passage anticipé possible si l'établissement le prévoit) |
| CPS-23 | Élève interne inscrit à une activité du soir | l'internat (41) le voit « en activité » jusqu'à l'heure de fin ; puis attendu à l'internat |
| CPS-24 | Encadrant bénévole dont l'honorabilité expire dans 10 jours | alerte à la direction ; séances après l'échéance bloquées (si S-4310 actif) |

### 17.3 Association sportive, partenaires
| CPS | Cas | Traitement |
|---|---|---|
| CPS-25 | Rencontre UNSS déplacée la veille au soir | nouvelle convocation, avis aux convoqués et titulaires ; accusé de lecture ; la sortie du 42 est modifiée |
| CPS-26 | Élève convoqué sans autorisation annuelle | refus de départ (l'appel de départ du 42 bloque) ; la famille est relancée |
| CPS-27 | Licence non saisie à la date de la rencontre | avertissement au secrétaire de l'AS (règle de la fédération) |
| CPS-28 | L'élève abandonne l'AS en novembre | adhésion close ; la cotisation reste due selon la règle de l'AS (aucun remboursement automatique) |
| CPS-29 | Le partenaire demande la date de naissance pour l'assurance | transmise seulement si l'offre l'annonce et si la convention le prévoit |
| CPS-30 | Offre de partenaire commerciale (cours privés payants) dans un établissement public | publication refusée sans convention et validation de la direction (RG-5) |
| CPS-31 | Élu de la maison des lycéens responsable d'un club | droit restreint ; ne voit que ses inscrits (prénom, nom, classe) |

### 17.4 Transports
| CPS | Cas | Traitement |
|---|---|---|
| CPS-32 | Le transporteur supprime la ligne 12 à 6 h 15 | incident ; avis aux familles de la course avant 6 h 30 (fenêtre R-N50) ; justification des retards proposée |
| CPS-33 | Grève générale des transports | incident global ; justification en lot sur décision de la direction |
| CPS-34 | Élève de 6e qui a raté son car du soir | reste en étude ; avis à la famille ; jamais laissé seul à l'arrêt |
| CPS-35 | Retenue posée le mercredi à 13 h, car de 12 h 30 | avertissement « dernier car » ; avis obligatoire à la famille avant la date (R-PS22) |
| CPS-36 | Fichier de la région avec des élèves inconnus | rejets en file humaine ; aucune création d'élève |
| CPS-37 | Fichier de la région avec un INE erroné | rapprochement proposé (nom, date de naissance), jamais appliqué seul |
| CPS-38 | Famille qui déclare la ligne 12 alors que l'import dit ligne 14 | les deux sont visibles ; l'import fait foi pour les avis ; la vie scolaire tranche |
| CPS-39 | Élève en garde alternée qui prend deux lignes selon les semaines | deux affiliations avec des jours ou des semaines différentes (A/B) |
| CPS-40 | Transport adapté en taxi avec un horaire différent des cours | horaire visible de l'accueil ; l'élève attendu au portail ; jamais de motif |
| CPS-41 | Accident de car | liste des élèves du car (direction), contacts d'urgence, procédure du 14 ; journal |
| CPS-42 | Fin de l'année : la famille demande ce que l'établissement garde sur les trajets | affiliations effacées au 31 août (R-PS30) ; export sur demande d'accès (27) |

### 17.5 Covoiturage
| CPS | Cas | Traitement |
|---|---|---|
| CPS-43 | Un parent s'inscrit pour savoir quelles familles habitent sa rue | impossible : zone = commune ; aucune information avant double acceptation ; plafond de propositions |
| CPS-44 | Signalement d'un conducteur dangereux | suspension par la direction, journal ; l'établissement ne juge pas : il retire la mise en relation |

---

## 18. Intelligence artificielle (contrat IA, `IA_COEUR.md` §5, §6.2)

### 18.1 Ce que l'IA fait [R]
- **Rédige** une fiche d'activité claire à partir de quelques mots (N0) ; un brouillon de convocation ou d'avis (N0, l'envoi reste N3).
- **Propose** une répartition des élèves sur les créneaux d'une activité surchargée sous contraintes (emploi du temps, dernier car, fratrie) ; acceptée créneau par créneau (N2) ; le **tirage au sort** reste déterministe et n'est pas fait par l'IA.
- **Signale** des incohérences : élève inscrit à deux activités simultanées, encadrant sur deux lieux, séance après le dernier car d'un usager, honorabilité qui expire.
- **Lit** un fichier de lignes ou d'usagers mal formé et propose une correspondance des colonnes (N1) ; l'import reste l'aperçu déterministe.
- **Prépare** le bilan de l'AS (effectifs, participation, résultats) et la liste proposée pour l'accompagnement (élèves avec beaucoup de travaux non rendus), toujours **proposée** au professeur principal (N2).

### 18.2 Ce que l'IA ne fait pas [R]
- Ne lit **aucune donnée de santé** (34), aucun besoin particulier (35), aucune aide (26), aucune mesure de protection (36).
- N'inscrit, ne désinscrit, ne tire au sort, ne justifie et n'envoie rien sans confirmation humaine.
- Ne classe pas les élèves par mérite ou par comportement pour l'accès à une activité.
- N'analyse pas les trajets ni les zones des familles du covoiturage.

### 18.3 Capacités déclarées (registre `ent-ia`)
| Capacité | Niveau | Droit requis |
|---|---|---|
| `periscolaire.rediger_fiche` | N0 | `periscolaire.activites_gerer` |
| `periscolaire.proposer_repartition` | N2 | `periscolaire.inscriptions_gerer` |
| `periscolaire.signaler_incoherences` | N0 | `periscolaire.lire` |
| `periscolaire.proposer_accompagnement` | N2 | `periscolaire.inscriptions_gerer` (portée : ses classes) |
| `periscolaire.bilan_as` | N0 | `periscolaire.as_gerer` |
| `transport.lire_fichier` | N1 | `transport.importer` |
| `transport.rediger_avis_incident` | N0 (envoi N3) | `transport.incidents` |

### 18.4 Cadre
Jeu d'évaluation propre (20 cas) ; tests de garde : injection dans une description d'activité, demande de liste des familles d'une commune, demande d'avis sans confirmation ; aucune donnée interdite dans un contexte (test).

---

## 19. Actions métier (PSC-100 à PSC-239)

Colonnes : acteur · droit D-017 (§21.1) ; règles ; événement émis ; lot. ★ = réauthentification récente. Les actions PER-01 à PER-05 de `SOCLE2_DOMAINES.md` D22 correspondent à PSC-100, PSC-120, PSC-145, PSC-230 et PSC-200.

Acteurs : DIR direction ; CPE ; VS vie scolaire ; ADM secrétariat, administratif ; RSP responsable d'activité ; ENC encadrant (agent, intervenant, bénévole) ; SAS secrétaire de l'AS ; TIT titulaire ; PAY payeur ; ELV élève ; MAJ élève majeur ; ACC accueil ; RTR référent transports ; SYS système.

### 19.1 Offre et cycle
| PSC | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PSC-100 | Créer une activité (famille, organisateur, responsable, public, sessions, places, attribution, prix, autorisations) (NP-01) | RSP, CPE, DIR · `periscolaire.activites_gerer` | §8.2 ; brouillon | `periscolaire.activite_creee` | PS1 |
| PSC-101 | Modifier une activité ; avis aux inscrits si l'horaire, le lieu ou le prix change (NP-01) | RSP · idem | verrou optimiste ; prix : jamais à la hausse pour les inscrits sans accord | `periscolaire.activite_modifiee` | PS1 |
| PSC-102 | Dupliquer une activité | RSP · idem | ni inscrits ni pièces | `periscolaire.activite_creee` | PS1 |
| PSC-103 | Reconduire l'offre de l'année suivante (sélection par lot) | CPE, DIR · idem | invitation des inscrits selon S-4304 | `periscolaire.offre_reconduite` | PS8 |
| PSC-104 | Ajouter, modifier une session (période, rythme, horaires, places, minimum) | RSP · idem | calendrier du 5 | `periscolaire.session_modifiee` | PS1 |
| PSC-105 | Déposer le visuel de l'activité (NP-02) | RSP · idem | objet du 7 ; aucune photo d'élève | — | PS1 |
| PSC-106 | Demander la réservation des salles de toutes les séances (19) | RSP · idem + droit du 19 | rapport par occurrence | `agenda.reservation_*` (19) | PS2 |
| PSC-107 | Affecter un ou plusieurs encadrants (6) | RSP, CPE · idem | honorabilité (S-4310) ; conflit d'horaire de l'encadrant | `periscolaire.encadrant_affecte` | PS1 |
| PSC-108 | Lier un tarif du barème (24) | ADM, CPT · `periscolaire.tarifs` | quantité seulement | — | PS6 |
| PSC-109 | Soumettre l'activité à l'autorisation de la direction | RSP · idem | intervenant extérieur ou nouveauté | `periscolaire.activite_soumise` | PS1 |
| PSC-110 | Autoriser ou refuser une activité (document du 7) | DIR · `periscolaire.autoriser` | N2 ; motif du refus | `periscolaire.activite_autorisee` | PS1 |
| PSC-111 | Publier, dépublier une activité | RSP · idem | autorisation ; fenêtre | `periscolaire.activite_publiee` | PS1 |
| PSC-112 | Voir le catalogue filtré sur ce que l'élève peut suivre (NP-06) | TIT, ELV, MAJ · `soi` | public, emploi du temps, dernier car | — | PS1 |
| PSC-113 | Publier la vitrine publique de l'offre (sans formulaire) (NP-06) | DIR · `periscolaire.autoriser` | R-PS10 | — | PS9 |
| PSC-114 | Clore une activité (fin de l'année ou arrêt) | RSP · idem | inscriptions closes ; séances futures retirées | `periscolaire.activite_close` | PS1 |
| PSC-115 | Retirer une activité créée par erreur (sans inscrit) | RSP · idem | retrait daté, jamais suppression | `periscolaire.activite_retiree` | PS1 |
| PSC-116 | Déclarer l'entité organisatrice (AS, foyer, maison des lycéens, partenaire) et ses responsables | DIR · `periscolaire.entites` | entité émettrice (D-027 F) | `periscolaire.entite_modifiee` | PS4 |
| PSC-117 | Marquer l'intérêt d'un élève au forum (tablette, badge) | ELV · `soi` | pas une inscription | — | PS9 |
| PSC-118 | Programmer la réunion d'information d'une offre (session du 19) (NP-03) | RSP · idem | 19 AG6 | `agenda.session_*` | PS9 |
| PSC-119 | Voir le tableau « hors cours ce soir » (activités, lieux, encadrants, effectifs) | DIR, CPE, ACC · `periscolaire.lire` | effectifs ; noms au clic | — | PS2 |

### 19.2 Inscriptions
| PSC | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PSC-120 | Inscrire son enfant (NP-07) | TIT · `soi` (titulaire) | R-PS1 à R-PS4 ; ★ si payant | `periscolaire.inscription_demandee` | PS1 |
| PSC-121 | Demander une inscription (élève mineur) | ELV · `soi` | confirmation par un titulaire | `periscolaire.inscription_demandee` | PS1 |
| PSC-122 | S'inscrire seul (élève majeur) | MAJ · `soi` | partage aux parents selon le 4 | idem | PS1 |
| PSC-123 | Confirmer comme payeur, ou payer soi-même | PAY, TIT · `soi` | ★ | `periscolaire.inscription_confirmee` | PS1 |
| PSC-124 | Avis d'inscription (inscrivant, autre titulaire, récapitulatif au responsable) (NP-09) | SYS | 8 ; récapitulatif quotidien | — | PS1 |
| PSC-125 | Inscrire un élève par l'établissement (proposition, mesure éducative) | CPE, VS, RSP · `periscolaire.inscriptions_gerer` | avis aux titulaires ; payant : accord du payeur | `periscolaire.inscription_confirmee` | PS1 |
| PSC-126 | Inscrire un groupe d'élèves (classe entière à un atelier du midi) | CPE · idem | N2 ; conditions vérifiées une à une | idem (par élève) | PS1 |
| PSC-127 | Accorder une dérogation à une condition (niveau, âge) | RSP · idem | motif codé ; tracé | `periscolaire.derogation` | PS1 |
| PSC-128 | Valider, refuser une inscription à dossier (NP-04) | RSP · idem | motif du refus | `periscolaire.inscription_decidee` | PS1 |
| PSC-129 | Annuler une inscription (NP-04) | RSP · idem | règle de l'offre ; avoir (24) | `periscolaire.inscription_close` | PS1 |
| PSC-130 | Lancer le tirage au sort à la fermeture de la fenêtre | RSP · idem | graine publiée ; rejouable (R-PS8) | `periscolaire.tirage_effectue` | PS7 |
| PSC-131 | Réserver des places décidées par la direction (aménagement) | DIR · `periscolaire.autoriser` | jamais de motif visible | — | PS7 |
| PSC-132 | Gérer la liste d'attente : proposer la place libérée, délai, suivant | SYS, RSP | S-4305 | `periscolaire.place_proposee` | PS7 |
| PSC-133 | Exporter la liste des inscrits (CSV `;`) (NP-05) | RSP · idem | colonnes minimales ; journalisé | — | PS1 |
| PSC-134 | Changer de jour ou de session | TIT, RSP | places vérifiées | `periscolaire.inscription_modifiee` | PS1 |
| PSC-135 | Se désister | TIT, MAJ · `soi` | règle de l'offre ; motif facultatif | `periscolaire.inscription_close` | PS1 |
| PSC-136 | Exclure d'une activité (décision motivée) | RSP + DIR · `periscolaire.autoriser` | N2 ; sanction → 13 | `periscolaire.inscription_close` | PS1 |
| PSC-137 | Voir les inscriptions de son enfant, leur état, ce qui manque | TIT · `soi` | | — | PS1 |
| PSC-138 | Fermer les inscriptions à la date de départ de l'élève (2) | SYS | §6.4 | `periscolaire.inscription_close` | PS1 |
| PSC-139 | Voir les activités d'un élève dans sa fiche 360° | personnel autorisé · `periscolaire.lire` | | — | PS1 |

### 19.3 Séances et appel
| PSC | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PSC-140 | Générer les séances d'une session (calendrier du 5) | SYS | vacances et fermetures sautées | `periscolaire.seances_generees` | PS2 |
| PSC-141 | Ajouter, déplacer une séance | RSP · `periscolaire.activites_gerer` | conflits revérifiés | `periscolaire.seance_modifiee` | PS2 |
| PSC-142 | Annuler une séance (motif codé, avis) | RSP · idem | R-PS15 | `periscolaire.seance_annulee` | PS2 |
| PSC-143 | Signaler une séance sans encadrant et proposer un remplaçant | SYS → CPE | §10.1 | `periscolaire.seance_sans_encadrant` | PS2 |
| PSC-144 | Remplacer l'encadrant d'une séance | CPE · idem | honorabilité | `periscolaire.encadrant_affecte` | PS2 |
| PSC-145 | Faire l'appel (composant du 12), hors ligne | ENC · `periscolaire.appel` (ses séances) | inscrits attendus pré-placés | `periscolaire.appel_valide` | PS2 |
| PSC-146 | Ajouter un invité présent | ENC · idem | pas d'engagement | — | PS2 |
| PSC-147 | Signaler les écarts à la vie scolaire (10 min) | SYS | S-4307 | `periscolaire.ecart` | PS2 |
| PSC-148 | Traiter un écart (contacter la famille, classer) | VS, CPE · `periscolaire.ecarts` | avis N3 | `periscolaire.ecart_traite` | PS2 |
| PSC-149 | Annoncer une absence à une séance | TIT, MAJ · `soi` | avant l'heure | `periscolaire.absence_annoncee` | PS2 |
| PSC-150 | Corriger un appel validé (motif) | ENC, RSP | journal | `periscolaire.appel_corrige` | PS2 |
| PSC-151 | Marquer un départ (seul, remis, car, internat) | ENC · idem | port du 14 | `accueil.remise` (14) | PS3 |
| PSC-152 | Voir les consignes minimales du 34 dans l'appel | ENC · idem | jamais de motif ; journalisé | — | PS2 |
| PSC-153 | Alerter sur des absences répétées | SYS | S-4306 | `periscolaire.absences_repetees` | PS2 |
| PSC-154 | Voir la feuille de présence de l'activité (mois, année) | RSP · `periscolaire.lire` | | — | PS2 |
| PSC-155 | Signaler un élève non sorti à l'heure de fermeture | SYS → VS, DIR | R-PS18 | `periscolaire.non_sorti` | PS3 |
| PSC-156 | Publier les présents hors temps scolaire au 14 | SYS | port `attendus` | — | PS2 |
| PSC-157 | Imprimer la liste de séance de secours | ENC | sans coordonnées | — | PS2 |
| PSC-158 | Noter des faits codés de suivi (accompagnement) | ENC · idem | pas d'appréciation libre | `periscolaire.suivi_note` | PS3 |
| PSC-159 | Écrire au fil de l'activité (16) | RSP, ENC | modération ; pas de données de santé | — | PS10 |

### 19.4 Études, garderie, accompagnement
| PSC | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PSC-160 | Ouvrir les créneaux d'étude et de garderie (jours, heures, lieux, encadrants) | CPE · `periscolaire.activites_gerer` | | `periscolaire.activite_creee` | PS3 |
| PSC-161 | Saisir la grille hebdomadaire d'un élève (cases par jour) (NP-17) | ADM, VS · `periscolaire.inscriptions_gerer` | chaque case = inscription datée | `periscolaire.inscription_*` | PS3 |
| PSC-162 | Inscrire ponctuellement à la garderie | TIT · `soi` | avant l'heure limite (S-4308) | idem | PS3 |
| PSC-163 | Annoncer un départ seul exceptionnel | TIT · `soi` | titulaire identifié ; autorisation de sortie | `accueil.sortie_annoncee` (14) | PS3 |
| PSC-164 | Voir, au départ, qui part seul, qui attend qui, qui prend le car | ENC · `periscolaire.appel` | ports 14, 43 transport | — | PS3 |
| PSC-165 | Reprendre les grilles de Next.js (étude, garderie) en inscriptions datées | SYS | §26 | — | PS11 |
| PSC-166 | Proposer la liste de l'accompagnement (IA ou règle) | SYS, IA → PP | N2 | — | PS3 |
| PSC-167 | Valider l'inscription à l'accompagnement sur proposition | PP, CPE · `periscolaire.inscriptions_gerer` (ses classes) | avis à la famille | `periscolaire.inscription_confirmee` | PS3 |
| PSC-168 | Compter les heures des intervenants de l'accompagnement (vers 15, 49) | SYS | quantités | `periscolaire.heures_effectuees` | PS6 |
| PSC-169 | Voir le suivi de l'accompagnement de son enfant | TIT · `soi` | faits codés | — | PS3 |

### 19.5 Association sportive et associations
| PSC | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PSC-170 | Déclarer l'AS (statuts, bureau, affiliation) | DIR · `periscolaire.entites` | | `periscolaire.entite_modifiee` | PS4 |
| PSC-171 | Ouvrir les adhésions de l'AS (cotisation, autorisation annuelle) | SAS · `periscolaire.as_gerer` | 18 | — | PS4 |
| PSC-172 | Adhérer à l'AS | TIT, MAJ · `soi` | ★ si cotisation | `periscolaire.adhesion` | PS4 |
| PSC-173 | Exporter la liste des adhérents pour la plateforme de licences | SAS · idem | format au 9 ; minimal | — | PS4 |
| PSC-174 | Importer les numéros de licence | SAS · idem | aperçu ; rejets | `periscolaire.licences_importees` | PS4 |
| PSC-175 | Saisir une licence à la main | SAS · idem | | — | PS4 |
| PSC-176 | Enregistrer « attestation de santé fournie le » (sans document) | SAS · idem | aucun contenu médical (R-PS20) | — | PS4 |
| PSC-177 | Créer une rencontre (sport, catégorie, lieu, horaires) | SAS, ENC · idem | | `periscolaire.rencontre_creee` | PS4 |
| PSC-178 | Convoquer une équipe ; accusé de lecture ; relance | SAS, ENC · idem | N3 | `periscolaire.convocation` | PS4 |
| PSC-179 | Demander la sortie `rencontre_sportive` au 42 | SAS · idem + 42 | circuit réduit | `sorties.creee` (42) | PS4 |
| PSC-180 | Annoncer un retour retardé depuis le mobile | ENC · idem | N3, modèle | — | PS4 |
| PSC-181 | Saisir les résultats d'une rencontre | ENC · idem | pas de classement nominatif public (S-4309) | `periscolaire.resultats` | PS4 |
| PSC-182 | Voir les créneaux assurés et le forfait AS des enseignants | DIR, SAS · `periscolaire.as_gerer` | lecture du 6 ; pas d'évaluation | — | PS4 |
| PSC-183 | Suivre les cotisations (payée, non payée) | trésorier · `periscolaire.entites` (son entité) | pas de comptabilité | — | PS4 |
| PSC-184 | Encaisser une cotisation pour l'association (25, entité de l'association) | trésorier | sur décision (Q-PS2) | `paiements.*` (25) | PS6 |
| PSC-185 | Produire le bilan de l'assemblée générale (effectifs, participation) | SAS · idem | brouillon IA relu | — | PS4 |
| PSC-186 | Attester un rôle (jeune officiel, élu du foyer) | SAS, RSP · idem | document du 7 | `periscolaire.role_atteste` | PS4 |
| PSC-187 | Ouvrir les adhésions du foyer ou de la maison des lycéens | responsable de l'entité | | — | PS4 |
| PSC-188 | Nommer un élève responsable d'activité (maison des lycéens) | DIR, CPE · `periscolaire.entites` | droit restreint (§11.2) | `periscolaire.responsable_nomme` | PS4 |
| PSC-189 | Exporter les comptes de l'association (pour son trésorier) | trésorier | export ; pas de comptabilité dans ScolIA (48 plus tard) | — | PS6 |

### 19.6 Partenaires et intervenants
| PSC | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PSC-190 | Déclarer un partenaire (organisme, convention, assurance) | DIR, ADM · `periscolaire.partenaires` | annuaire commun (42, 48) | `periscolaire.partenaire_modifie` | PS9 |
| PSC-191 | Publier une offre de partenaire (information ou inscription) (NP-01) | DIR · `periscolaire.autoriser` | RG-5 ; mention « proposée par » | `periscolaire.activite_publiee` | PS9 |
| PSC-192 | Transmettre la liste minimale des inscrits au partenaire | RSP · `periscolaire.partenaires` | colonnes annoncées ; journalisé | — | PS9 |
| PSC-193 | Ouvrir un accès partenaire limité à son offre | DIR · idem | compte personnel, expiration | `periscolaire.acces_partenaire` | PS9 |
| PSC-194 | Engager un intervenant ou un bénévole (6) | DIR · `personnels.*` (6) | convention ; honorabilité | (6) | PS1 |
| PSC-195 | Voir ses activités et ses séances (intervenant) | ENC · `soi` | | — | PS2 |
| PSC-196 | Tenir la réunion d'information d'une offre (19) (NP-03) | RSP | 19 | — | PS9 |
| PSC-197 | Signer la convention avec un partenaire (7) | DIR | S2 | `documents.signe` | PS9 |
| PSC-198 | Évaluer une offre de partenaire en fin d'année (participation, satisfaction des familles anonyme au-dessus d'un seuil) | RSP | seuil du 18 | — | PS10 |
| PSC-199 | Suspendre un intervenant (signalement) | DIR · `periscolaire.autoriser` | séances retirées ; journal | `periscolaire.encadrant_suspendu` | PS1 |

### 19.7 Transports
| PSC | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PSC-200 | Importer lignes, arrêts et courses (fichier de l'autorité) | RTR · `transport.importer` | aperçu, rejets, versions | `transport.reseau_importe` | PS5 |
| PSC-201 | Saisir une ligne à la main | RTR · idem | | idem | PS5 |
| PSC-202 | Importer les usagers (abonnés) | RTR · idem | rapprochement proposé, jamais appliqué seul | `transport.inscriptions_modifiees` | PS5 |
| PSC-203 | Déclarer la ligne de son enfant | TIT, MAJ · `soi` | marquée « déclarée » | idem | PS5 |
| PSC-204 | Corriger une affiliation | RTR · `transport.gerer` | | idem | PS5 |
| PSC-205 | Exporter la liste des demandeurs pour l'autorité organisatrice | ADM · `transport.exporter` | demande de la famille ; minimal ; journalisé (D-022) | — | PS5 |
| PSC-206 | Enregistrer un transport adapté (prestataire, horaires, contact) | RTR, CPE · `transport.gerer` | aucun motif | `transport.adapte_modifie` | PS5 |
| PSC-207 | Voir les usagers d'une ligne, d'une course | VS, ACC, RTR · `transport.lire` | | — | PS5 |
| PSC-208 | Déclarer un incident (course supprimée, retard, déviation) | RTR, VS, DIR · `transport.incidents` | | `transport.incident` | PS5 |
| PSC-209 | Prévenir les familles des usagers d'une course (fenêtre veille/matin) | RTR, DIR · idem | N3 ; R-N50 | `notifications.diffusee` (8) | PS5 |
| PSC-210 | Proposer la justification en lot des retards (12) | SYS → VS | N2 | `assiduite.*` (12) | PS5 |
| PSC-211 | Garder en étude les usagers d'un car du soir supprimé ; avis | SYS → VS | | — | PS5 |
| PSC-212 | Signaler un élève non monté | RTR, TIT | procédure courte | `transport.non_monte` | PS5 |
| PSC-213 | Afficher le dernier car à la pose d'une retenue ou d'une activité (13, 43) | SYS | R-PS22 | — | PS5 |
| PSC-214 | Publier `transport::dernier_depart` au 10 | SYS | CL-67 | — | PS5 |
| PSC-215 | Produire la liste des élèves par car (évacuation, accident) | DIR, VS · `transport.lire` | contacts d'urgence seulement pendant l'incident | — | PS5 |
| PSC-216 | Planifier et tracer l'exercice d'évacuation de car | DIR, CPE · `transport.gerer` | compte rendu (7) | `transport.exercice` | PS10 |
| PSC-217 | Voir l'historique des incidents d'une ligne (pour l'autorité) | RTR, DIR | | — | PS10 |
| PSC-218 | Effacer les affiliations de l'année au 31 août | SYS | R-PS30 | — | PS5 |
| PSC-219 | Afficher à la famille la ligne, l'arrêt et les avis de son enfant | TIT · `soi` | | — | PS5 |

### 19.8 Covoiturage
| PSC | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PSC-220 | Créer son profil (zone, site, sens, jours, note) (NP-11) | TIT · `soi` | consentement daté (R-PS27) | — | PS10 |
| PSC-221 | Modifier son profil | TIT · `soi` | | — | PS10 |
| PSC-222 | Clore, réactiver sa recherche (NP-12) | TIT · `soi` | | — | PS10 |
| PSC-223 | Se désinscrire : effacement réel (NP-13) | TIT · `soi` | R-PS29 | — | PS10 |
| PSC-224 | Recevoir des propositions (plafond, avis groupé) | SYS | R-PS28 | — | PS10 |
| PSC-225 | Accepter, refuser une proposition ; dévoilement après double acceptation (NP-14) | TIT · `soi` | moyen de contact choisi | — | PS10 |
| PSC-226 | Voir ses correspondances (« Famille inscrite ») (NP-15) | TIT · `soi` | projection opaque | — | PS10 |
| PSC-227 | Signaler, suspendre ; modérer | TIT ; DIR · `periscolaire.covoiturage_moderer` | journal | — | PS10 |
| PSC-228 | Activer le covoiturage avec notice | DIR · `etablissement.parametrer` | désactivé par défaut | — | PS10 |
| PSC-229 | Purger au 31 août | SYS | | — | PS10 |

### 19.9 Argent, pilotage, système
| PSC | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PSC-230 | Transmettre engagements et séances au 24 | SYS | quantités ; entité de l'organisateur | `facturation.*` (24) | PS6 |
| PSC-231 | Voir l'état comparatif mensuel pendant le doublon (mode suivi) | CPT · `periscolaire.tarifs` | D-034 | — | PS6 |
| PSC-232 | Proposer une aide à l'inscription (lien 26) | SYS | R-PS24 | — | PS6 |
| PSC-233 | Tableau de bord (effectifs, taux de remplissage, présence, écarts, incidents de transport) | DIR, CPE · `periscolaire.lire` | aucune donnée nominative par défaut | — | PS10 |
| PSC-234 | Indicateurs pour le 47 | SYS | | — | PS10 |
| PSC-235 | Exposer la participation au 23 et au 37 | SYS | avec l'accord de l'élève majeur | — | PS10 |
| PSC-236 | Paramétrer (réglages §28) | DIR · `etablissement.parametrer` | | — | PS1 |
| PSC-237 | Reprendre les partenariats et le covoiturage de Next.js | SYS | §26 | — | PS11 |
| PSC-238 | Répondre à une demande d'accès RGPD (27) | DPO, DIR | export par élève | — | PS12 |
| PSC-239 | Purger selon les durées (§21.3) | SYS | | — | PS12 |

---

## 20. Règles transverses (R-PS1 à R-PS33) [R]

| R | Règle |
|---|---|
| R-PS1 | Une inscription d'élève mineur à une activité gratuite sans déplacement est valide par **un** titulaire (acte usuel) ; l'autre est informé. |
| R-PS2 | Une demande faite par un compte élève mineur n'engage rien : un titulaire confirme, par passkey si payant. |
| R-PS3 | Désaccord connu entre titulaires (4) : accord des deux exigé ; le motif n'est jamais montré au responsable. |
| R-PS4 | Une personne visée par une interdiction de remise peut inscrire (si elle est titulaire) mais jamais récupérer ; la vie scolaire est avisée. |
| R-PS5 | Une activité payante n'est confirmée qu'avec un payeur engagé (24). |
| R-PS6 | Une inscription n'est possible que si l'activité est publiée, dans sa fenêtre, avec des places, et si l'élève remplit les conditions (ou a une dérogation tracée). |
| R-PS7 | Les places sont décomptées en transaction (verrou de ligne) ; unicité (session, élève) en base. |
| R-PS8 | Le tirage au sort est déterministe à partir d'une graine publiée avant et révélée après ; les priorités sont déclarées avant l'ouverture ; aucun critère social ou de santé n'est une priorité visible. |
| R-PS9 | Une inscription et ses pièces (signature, autorisations) sont écrites dans la même transaction, ou l'inscription reste `en_attente` avec ce qui manque. |
| R-PS10 | La vitrine publique ne collecte aucune donnée personnelle et ne publie aucune liste. |
| R-PS11 | Liens d'une offre : `https:` uniquement, domaine affiché ; texte rendu échappé. |
| R-PS12 | Une inscription est un engagement daté (début, fin, jours) ; aucune écriture n'efface un fait passé. |
| R-PS13 | Une inscrite à une séance est **attendue** sauf annonce, excuse, sortie (42), stage (39), fermeture (5). |
| R-PS14 | Un écart (attendu absent) est signalé à la vie scolaire dans le délai S-4307 ; jamais d'avis automatique à la famille sans décision. |
| R-PS15 | Une séance d'étude ou de garderie n'est jamais annulée sans solution d'accueil ou avis des familles la veille, sauf urgence décidée par la direction. |
| R-PS16 | Aucune séance sans encadrant valide à la date (honorabilité si exigée). |
| R-PS17 | Le départ d'une activité suit le régime de sortie du 14 ; aucune sortie seul sans autorisation valide. |
| R-PS18 | Un élève encore attendu à l'heure de fermeture déclenche une alerte. |
| R-PS19 | L'encadrant ne voit que ses séances et, pour chaque élève, le strict nécessaire (§21.2). |
| R-PS20 | Le 43 ne stocke aucun document ni motif médical ; il lit des consignes du 34. |
| R-PS21 | L'argent d'une association est émis sous l'entité de l'association, jamais sous celle de l'établissement. |
| R-PS22 | Une retenue, une séance ou une activité qui finit après le dernier car d'un usager exige un avis à la famille avant la date. |
| R-PS23 | Les affiliations de transport importées font foi ; les déclarations des familles sont marquées et ne les écrasent jamais. |
| R-PS24 | Aucun élève n'est écarté d'une activité de l'établissement pour raison financière : une aide est proposable. |
| R-PS25 | Un import (lignes, usagers, licences) ne crée jamais d'élève et ne rapproche jamais automatiquement par ressemblance. |
| R-PS26 | Les avis de transport ne partent qu'aux usagers de la course concernée, dans la fenêtre R-N50. |
| R-PS27 | Covoiturage : consentement explicite, daté, retirable ; désactivé par défaut. |
| R-PS28 | Covoiturage : une zone (commune), jamais d'adresse ; plafond de propositions ; aucune information avant double acceptation ; projection opaque. |
| R-PS29 | Covoiturage : effacement réel à la désinscription et au 31 août. |
| R-PS30 | Affiliations de transport effacées au 31 août ; incidents agrégés conservés sans nom. |
| R-PS31 | Toute commande passe par le serveur (écran, mobile, IA : même commande). |
| R-PS32 | Transmission à un partenaire : colonnes annoncées dans l'offre, journalisée. |
| R-PS33 | Un responsable élève (maison des lycéens) n'a jamais accès à un dossier, à une coordonnée ni à une donnée de santé. |

---

## 21. Droits et minimisation

### 21.1 Catalogue d'actions proposé (D-017) [R]
| Action | Portée | Rôles par défaut |
|---|---|---|
| `periscolaire.lire` | établissement, ou ses activités | direction, CPE, vie scolaire, accueil (lecture des présents) |
| `periscolaire.activites_gerer` | ses activités ; établissement | responsable d'activité ; CPE, direction |
| `periscolaire.autoriser` | établissement | direction |
| `periscolaire.inscriptions_gerer` | ses activités ; ses classes (accompagnement) ; établissement | responsable, PP, CPE, secrétariat |
| `periscolaire.appel` | ses séances | encadrant |
| `periscolaire.ecarts` | établissement | vie scolaire, CPE |
| `periscolaire.as_gerer` | l'AS | secrétaire de l'AS, enseignants d'EPS |
| `periscolaire.entites` | son entité ; établissement | direction ; trésorier de l'entité |
| `periscolaire.partenaires` | établissement | direction, administratif |
| `periscolaire.tarifs` | établissement | comptabilité |
| `periscolaire.covoiturage_moderer` | établissement | direction |
| `transport.lire` | établissement | vie scolaire, accueil, CPE, direction |
| `transport.gerer`, `transport.importer` | établissement | référent transports |
| `transport.exporter` | établissement | administratif (D-022) |
| `transport.incidents` | établissement | référent, vie scolaire, direction |

### 21.2 Ce que voit chacun (projections) [R]
| Qui | Voit | Ne voit pas |
|---|---|---|
| Encadrant (agent) | ses séances ; prénom, nom, classe, photo (si activée) de ses inscrits ; consignes minimales ; régime de départ ; personnes autorisées (au départ) | coordonnées des familles, dossier, motifs |
| Intervenant, bénévole | idem sans la classe (niveau seulement) ; pas de photo par défaut | idem + classe |
| Responsable élève (maison des lycéens) | prénom, nom, niveau de ses inscrits | tout le reste |
| Partenaire | la liste annoncée par l'offre | le reste |
| Famille | les activités et affiliations de **ses** enfants, leurs séances, présences, avis | les autres inscrits (sauf prénoms dans le fil si l'activité l'active) |
| Vie scolaire | écarts, présents, usagers des lignes | aides, santé |
| Comptabilité | engagements et quantités | présences détaillées hors facturation à la séance |

### 21.3 Conservation [R, durées soumises au DPO]
| Donnée | Durée proposée |
|---|---|
| Activités, sessions (sans nom) | 5 ans (pilotage) |
| Inscriptions et présences | année scolaire + 1 an ; puis agrégées |
| Écarts et leur traitement | année + 1 an |
| Engagements facturés | durée comptable du 24 |
| Licences (numéro) | année + 1 an |
| Affiliations de transport | jusqu'au 31 août de l'année |
| Incidents de transport | 3 ans sans nom (pour l'autorité organisatrice) |
| Covoiturage | jusqu'à la désinscription ou au 31 août |
| Conventions, autorisations d'activité | 5 ans après la fin |

---

## 22. Événements
`periscolaire.activite_creee`, `_modifiee`, `_soumise`, `_autorisee`, `_publiee`, `_close`, `_retiree` ; `periscolaire.session_modifiee` ; `periscolaire.encadrant_affecte`, `_suspendu` ; `periscolaire.inscription_demandee`, `_confirmee`, `_decidee`, `_modifiee`, `_close` ; `periscolaire.tirage_effectue`, `place_proposee`, `derogation` ; `periscolaire.seances_generees`, `seance_modifiee`, `seance_annulee`, `seance_sans_encadrant` ; `periscolaire.appel_valide`, `appel_corrige`, `ecart`, `ecart_traite`, `absence_annoncee`, `absences_repetees`, `non_sorti` ; `periscolaire.adhesion`, `licences_importees`, `rencontre_creee`, `convocation`, `resultats`, `role_atteste`, `entite_modifiee`, `responsable_nomme`, `partenaire_modifie`, `acces_partenaire`, `heures_effectuees`, `suivi_note` ; `transport.reseau_importe`, `inscriptions_modifiees` (déjà attendu par le 10 et le 8), `adapte_modifie`, `incident`, `non_monte`, `exercice`. Tous publiés dans la transaction du fait (outbox) ; titres neutres dans la chronologie de l'élève (« Activité : théâtre », jamais un écart détaillé).

Natures de notifications déclarées au 8 : `periscolaire.inscription` (information), `periscolaire.place_proposee` (action, délai), `periscolaire.seance_annulee` (information urgente), `periscolaire.ecart` (vie scolaire seulement), `periscolaire.convocation` (action, accusé de lecture), `periscolaire.retour_retarde` (urgent), `transport.incident` (urgent, fenêtre R-N50, escalade SMS selon Q-N5 du 8), `transport.non_monte` (urgent).

## 23. Écrans et mobile (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`)
| Écran | Pour qui | Contenu |
|---|---|---|
| **Hors cours ce soir** (tuiles Bento) | direction, CPE, accueil | activités en cours, lieux, encadrants, effectifs attendus / présents, écarts, cars du soir |
| **Catalogue** | familles, élèves | par famille et par jour, filtré sur l'élève ; fiche ; inscription en deux gestes |
| **Mes activités** (famille) | familles | inscriptions, séances, présences, avis, ligne de car |
| **Activité** (responsable) | responsable | sessions, inscrits, liste d'attente, tirage, séances, feuille de présence, export |
| **Grille études et garderie** | secrétariat | tableau dense élève × jour, saisie rapide, filtres |
| **Appel de séance** (mobile) | encadrant | liste, pointage, départs, hors ligne |
| **AS** | secrétaire de l'AS | adhérents, licences, activités, rencontres, convocations, résultats, bilan |
| **Transports** | référent | réseau (version), usagers par ligne, incidents, avis, export |
| **Covoiturage** | familles | profil, propositions, correspondances |

Mobile (socle 11) : appel de séance, départ de l'étude, convocation et retour retardé de l'AS, avis de car supprimé, annonce d'absence ou de départ seul, catalogue et inscription.

## 24. Modèle de données (sans JSON, D-036 ; E1–E14)

Règles communes à toutes les tables : `id UUID PRIMARY KEY` généré par l'application (E3) ; `etablissement_id UUID NOT NULL` avec le contrôle « non nul uuid » (E1) ; `UNIQUE (etablissement_id, id)` et **FK composites** `(etablissement_id, x_id)` (E2) ; `cree_le TIMESTAMPTZ NOT NULL DEFAULT now()`, `cree_par` (personne) et `modifie_le` si modifiable (E6) ; `version INT` sur les objets modifiables (verrou optimiste) ; tables de faits en **ajout seul** (fonction commune `ent_ajout_seul()`) ; RLS par `ent_rls_poser` (E13) ; aucune colonne `jsonb`, `json`, `bytea`, aucun tableau d'identifiants (E5, E10) ; classement au registre des tables (E12) ; aucun nom d'élève dans un nom d'objet ni un journal applicatif (E14). Visuels, conventions et comptes rendus sont des **objets** `ent_objet` (500) référencés par `objet_id`. Heures en `TIMESTAMPTZ` calculées dans le fuseau de l'établissement (port `structure::fuseau`).

```
-- Lot PS0 (2860) : responsabilités de groupe datées
ent_groupe_responsabilite (id, etablissement_id, groupe_id → ent_groupe (composite), personne_id,
  role CHECK (responsable|co_responsable), du DATE NOT NULL, au DATE NULL, motif_fin_code NULL,
  cree_le, cree_par, EXCLUDE (etablissement_id, groupe_id, personne_id, daterange(du, au, '[]') WITH &&))
  -- reprise des lignes de ent_groupe_responsable (098) en responsabilités ouvertes le jour de la migration ;
  -- ent_groupe_responsable devient une vue de compatibilité (responsabilités ouvertes aujourd'hui)

-- Entités et offre (PS1 ; 2861–2862)
ent_periscolaire_entite (id, etablissement_id, type CHECK (etablissement|organisme_gestion|as|foyer|mdl|apel|partenaire),
  libelle, affiliation CHECK (unss|ugsel|aucune) NULL, numero_affiliation TEXT NULL, partenaire_id NULL,
  entite_emettrice_id NULL -- lien vers l'entité émettrice du 24, cree_le, modifie_le, ferme_le NULL)
ent_periscolaire_entite_role (id, etablissement_id, entite_id, personne_id,
  role CHECK (president|secretaire|tresorier|membre_comite|responsable), du, au NULL,
  CHECK : trésorier majeur vérifié par le service (R552-2))
ent_periscolaire_activite (id, etablissement_id, annee_id, entite_id, famille CHECK (as|club|etude|garderie|accompagnement|midi|partenaire|adhesion|vacances),
  nature CHECK (ordinaire|devoirs_faits|rencontre_ouverte) DEFAULT 'ordinaire',
  intitule, description, responsable_personne_id, groupe_id NULL (groupe du 5 de nature activite),
  attribution CHECK (premier_arrive|tirage|proposition|dossier), etat CHECK (brouillon|soumise|autorisee|publiee|close|retiree),
  autorisation_objet_id NULL, visuel_objet_id NULL, vitrine_publique BOOLEAN DEFAULT false,
  ouverture_le TIMESTAMPTZ NULL, fermeture_le TIMESTAMPTZ NULL, version, cree_le, modifie_le)
ent_periscolaire_activite_cible (id, etablissement_id, activite_id, type CHECK (niveau|regime|site|age_min|age_max),
  niveau_id NULL, regime_code NULL, site_id NULL, age SMALLINT NULL)
ent_periscolaire_activite_exigence (id, etablissement_id, activite_id, autorisation_code TEXT -- code du 18
  , obligatoire BOOLEAN)
ent_periscolaire_activite_lien (id, etablissement_id, activite_id, libelle, url TEXT CHECK (url LIKE 'https://%'), ordre)
ent_periscolaire_session (id, etablissement_id, activite_id, libelle, du DATE, au DATE, places_max INT NULL,
  places_min INT NULL, tarif_ref TEXT NULL -- clé du barème du 24, mode_facturation CHECK (gratuit|forfait|seance),
  date_limite_desistement DATE NULL, version, cree_le, modifie_le)
ent_periscolaire_creneau (id, etablissement_id, session_id, jour_semaine SMALLINT CHECK (1..7),
  heure_debut TIME, heure_fin TIME, semaines CHECK (toutes|a|b), salle_id NULL, lieu_id NULL)
ent_periscolaire_priorite (id, etablissement_id, session_id, code CHECK (non_servi_an_passe|interne|fratrie|niveau), ordre)

-- Inscriptions (PS1 ; 2863)
ent_periscolaire_inscription (id, etablissement_id, session_id, eleve_id, etat CHECK (demandee|en_attente_accord|
  en_attente_tirage|liste_attente|a_valider|confirmee|close|refusee), du DATE, au DATE NULL,
  rang_attente INT NULL, payeur_part_id NULL -- part du 4, inscrit_par_personne_id, origine CHECK (famille|eleve|etablissement|reprise),
  version, cree_le, modifie_le,
  EXCLUDE (etablissement_id, session_id, eleve_id, daterange(du, au, '[]') WITH &&) WHERE etat NOT IN ('close','refusee'))
ent_periscolaire_inscription_jour (id, etablissement_id, inscription_id, creneau_id) -- jours choisis
ent_periscolaire_inscription_fait (id, etablissement_id, inscription_id, a TIMESTAMPTZ, type CHECK (demande|accord_payeur|
  accord_titulaire|confirmation|changement|desistement|exclusion|cloture_depart|derogation|place_proposee|place_refusee),
  motif_code NULL, acteur_personne_id, piece_objet_id NULL) -- ajout seul
ent_periscolaire_tirage (id, etablissement_id, session_id, graine_empreinte TEXT, graine_revelee TEXT NULL,
  effectue_le, effectue_par) -- résultat : rangs dans ent_periscolaire_inscription_fait
ent_periscolaire_place_reservee (id, etablissement_id, session_id, nombre INT, decide_par, motif_code)

-- Séances et appel (PS2 ; 2864)
ent_periscolaire_seance (id, etablissement_id, session_id, creneau_id NULL, debut TIMESTAMPTZ, fin TIMESTAMPTZ,
  salle_id NULL, lieu_id NULL, reservation_id NULL -- 19, etat CHECK (prevue|annulee|realisee|neutralisee),
  annulation_motif_code NULL, version, cree_le, modifie_le)
ent_periscolaire_seance_encadrant (id, etablissement_id, seance_id, personne_id, role CHECK (encadrant|remplacant|accompagnant))
  -- l'appel lui-même est stocké par le composant du 12 (séance d'activité référencée par seance_id)
ent_periscolaire_ecart (id, etablissement_id, seance_id, eleve_id, detecte_le, etat CHECK (ouvert|famille_contactee|explique|classe),
  traite_par NULL, traite_le NULL, motif_code NULL)
ent_periscolaire_absence_annoncee (id, etablissement_id, inscription_id, seance_id, annonce_par_personne_id, annonce_le)
ent_periscolaire_suivi (id, etablissement_id, seance_id, eleve_id, code CHECK (travail_fait|lecon_revue|a_reprendre|materiel_oublie),
  par_personne_id, cree_le) -- ajout seul ; pas de texte libre

-- Associations, AS (PS4 ; 2865)
ent_periscolaire_adhesion (id, etablissement_id, entite_id, eleve_id NULL, personne_id NULL, annee_id,
  etat CHECK (demandee|active|close), licence_numero TEXT NULL, licence_saisie_le DATE NULL,
  cotisation_etat CHECK (sans|due|payee|exoneree), du, au NULL, cree_le, modifie_le,
  CHECK (num_nonnulls(eleve_id, personne_id) = 1))
ent_periscolaire_role_eleve (id, etablissement_id, adhesion_id, role CHECK (jeune_officiel_arbitre|jeune_officiel_juge|
  reporter|elu_comite|responsable_activite), du, au NULL, atteste_objet_id NULL)
ent_periscolaire_rencontre (id, etablissement_id, activite_id, intitule, debut, fin, lieu_texte, sortie_id NULL -- 42,
  depart_le NULL, retour_prevu_le NULL, etat CHECK (prevue|convoquee|realisee|annulee))
ent_periscolaire_convocation (id, etablissement_id, rencontre_id, eleve_id, convoque_le, lu_le NULL)
ent_periscolaire_resultat (id, etablissement_id, rencontre_id, epreuve TEXT, rang SMALLINT NULL, equipe TEXT NULL, eleve_id NULL)

-- Partenaires (PS9 ; 2866)
ent_partenaire (id, etablissement_id, raison_sociale, siren NULL, contact_nom, contact_courriel, convention_objet_id NULL,
  assurance_declaree_le DATE NULL, ferme_le NULL) -- partagé avec 42 et 48 (annuaire commun)
ent_periscolaire_transmission (id, etablissement_id, activite_id, partenaire_id, colonnes CHECK (minimal|minimal_naissance),
  transmis_le, par_personne_id, objet_id) -- trace de chaque envoi
ent_periscolaire_acces_partenaire (id, etablissement_id, partenaire_id, activite_id, personne_id, du, au)

-- Transports (PS5 ; 2867–2868)
ent_transport_reseau_version (id, etablissement_id, source CHECK (import|saisie), autorite TEXT, importe_le, objet_id NULL, etat CHECK (active|remplacee))
ent_transport_ligne (id, etablissement_id, version_id, code TEXT, libelle, transporteur TEXT NULL,
  UNIQUE (etablissement_id, version_id, code))
ent_transport_arret (id, etablissement_id, ligne_id, code TEXT NULL, libelle, commune TEXT, ordre SMALLINT)
ent_transport_course (id, etablissement_id, ligne_id, sens CHECK (aller|retour), heure TIME -- arrivée (aller) ou départ (retour) à l'établissement,
  jour_semaine SMALLINT, site_id NULL)
ent_transport_affiliation (id, etablissement_id, eleve_id, ligne_code TEXT, arret_id NULL, sens CHECK (aller|retour|les_deux),
  origine CHECK (import|declaration), du DATE, au DATE NULL, semaines CHECK (toutes|a|b), cree_le, modifie_le)
ent_transport_affiliation_jour (id, etablissement_id, affiliation_id, jour_semaine SMALLINT)
ent_transport_adapte (id, etablissement_id, eleve_id, prestataire TEXT, contact_telephone TEXT, du, au NULL)
ent_transport_adapte_horaire (id, etablissement_id, adapte_id, jour_semaine, arrivee TIME NULL, depart TIME NULL)
ent_transport_incident (id, etablissement_id, a TIMESTAMPTZ, portee CHECK (course|ligne|global), course_id NULL, ligne_id NULL,
  nature CHECK (supprimee|retard|deviation|greve|intemperies|accident|non_monte), retard_minutes INT NULL,
  source CHECK (transporteur|autorite|personnel|famille), declare_par, clos_le NULL)
ent_transport_incident_eleve (id, etablissement_id, incident_id, eleve_id) -- non monté, accident : élèves concernés
ent_transport_exercice (id, etablissement_id, ligne_id NULL, realise_le, compte_rendu_objet_id NULL)
ent_transport_exercice_participant (id, etablissement_id, exercice_id, eleve_id)

-- Covoiturage (PS10 ; 2869)
ent_covoiturage_profil (id, etablissement_id, personne_id, espace CHECK (familles|personnels), commune TEXT,
  code_postal CHAR(5), site_id, sens CHECK (matin|soir|les_deux), note VARCHAR(280) NULL,
  consentement_le TIMESTAMPTZ NOT NULL, etat CHECK (active|close), cree_le, modifie_le,
  UNIQUE (etablissement_id, personne_id, espace))
ent_covoiturage_profil_jour (id, etablissement_id, profil_id, jour_semaine)
ent_covoiturage_correspondance (id, etablissement_id, profil_a_id, profil_b_id, etat CHECK (proposee|acceptee_a|acceptee_b|devoilee|refusee|annulee),
  propose_le, devoile_le NULL, contact_a CHECK (message|courriel) NULL, contact_b NULL,
  CHECK (profil_a_id < profil_b_id), UNIQUE (etablissement_id, profil_a_id, profil_b_id))
ent_covoiturage_signalement (id, etablissement_id, correspondance_id, par_personne_id, code, traite_par NULL, traite_le NULL)
  -- effacement réel : DELETE des profils et correspondances à la désinscription et au 31 août (exception E6 déclarée :
  --   le droit à l'effacement l'emporte ; trace : compteur agrégé sans identifiant)

-- Reprise (PS11 ; 2870)
ent_periscolaire_reprise (id, etablissement_id, source CHECK (nextjs_partenariats|nextjs_grille|nextjs_covoiturage),
  source_id TEXT, cible_type, cible_id NULL, etat CHECK (reprise|rejetee|a_decider), raison_code NULL, cree_le)
```

**Exceptions D-036 demandées : aucune.** Exception E6 déclarée : effacement réel (DELETE) des tables de covoiturage et des affiliations de transport à l'échéance, au titre de la minimisation (à inscrire dans `MIGRATIONS.md`). Registre des tables (E12) : toutes en portée `etablissement` ; `ent_transport_*` et `ent_covoiturage_*` en catégorie d'export `simple`, **purgées** à la restauration si la date de restauration dépasse leur échéance ; aucune file d'effets de bord propre (les avis passent par le 8, E9).

---

## 25. Lots

### 25.1 Lot PS0 — correctifs urgents de l'existant Rust (avant tout autre lot du 43)
| # | Correctif | Défaut | Migration |
|---|---|---|---|
| 1 | Responsabilités de groupe **datées en ajout seul** (`ent_groupe_responsabilite`), reprise des lignes 098, vue de compatibilité `ent_groupe_responsable` ; commandes « ajouter » et « retirer » à la place du remplacement par effacement | RPS-01 | **2860** |
| 2 | Contrôle d'existence du groupe et écriture **dans la même transaction**, verrou de ligne sur le groupe | RPS-02 | — (code) |
| 3 | FK composite `(etablissement_id, groupe_id)` avec `ON DELETE RESTRICT` (NOT VALID puis VALIDATE) | RPS-03 | **2860** |
| 4 | Journal : chaque ajout ou retrait écrit `ent_journal_modif` (avant/après) dans la transaction | RPS-01 | — |
| 5 | Lecteurs (`cahier.rs:374`, `personnels_missions.rs:1863`, portée `MesGroupes`) lisent les responsabilités **à la date** | RPS-01 | — |
| 6 | Tests : concurrence (deux ajouts simultanés), isolation entre établissements, droits par rôle, lecture à une date passée | RPS-01 à RPS-03 | — |

Le lot PS0 ne touche pas aux natures de groupe (RPS-04) : elles sont posées par le **socle 5** (R-S15), prérequis de PS1.

### 25.2 Lots suivants
| Lot | Contenu | Dépend de | Critère de fin | Migrations |
|---|---|---|---|---|
| **PS1** Offre et inscriptions | entités, activités, cibles, exigences, sessions, créneaux, inscriptions, faits, conditions, payeur, accord des titulaires, places en transaction, catalogue famille, fiche 360°, actions D-017, encadrants (6) | PS0 ; 2, 4, 5 (groupe d'activité), 6, 18 (exigences) | R-PS1 à R-PS7, R-PS9, R-PS12 testés ; DPS-01, DPS-02, DPS-06 impossibles | 2861–2863 |
| **PS2** Séances et appel | génération des séances, réservations (19), encadrants, annulation et avis, composant d'appel (12), écarts, absences annoncées, présents hors temps scolaire (14), tableau « hors cours ce soir » | PS1 ; 5 (calendrier), 12, 14, 19 | R-PS13 à R-PS16 testés ; écart signalé en moins de 10 min | 2864 |
| **PS3** Études, garderie, accompagnement | créneaux, grille hebdomadaire, ponctuel, départ vérifié (14), Devoirs faits, suivi codé, liste proposée | PS2 ; 14, 20 | R-PS17, R-PS18 testés ; parité NP-17 | — (tables de PS1–PS2) |
| **PS4** AS et associations | entité AS, rôles (président, secrétaire, trésorier majeur), adhésions, licences (export, import), rencontres, convocations, sortie du 42, résultats, jeunes officiels, bilan, foyer et maison des lycéens | PS1 ; 42 (sortie `rencontre_sportive`), 18 | R552-2 respecté (trésorier majeur, président) ; aucune donnée de santé (R-PS20) | 2865 |
| **PS5** Transports | réseau importé ou saisi, versions, affiliations (import, déclaration), transport adapté, export pour l'autorité, incidents, avis (fenêtre R-N50), justification en lot (12), dernier car (10, 13), élèves par car, purge | PS1 ; 8 (audience, NTF-132), 9 (format), 10, 12, 13, 14 | R-PS22, R-PS23, R-PS25, R-PS26, R-PS30 testés ; avis envoyé aux seuls usagers | 2867–2868 |
| **PS6** Argent | engagements et séances au 24, entités émettrices, aides (26), avoirs, mode suivi pendant le doublon, cotisations, heures des intervenants (15, 49) | PS1, PS2 ; 24 (F10), 25, 26 | R-PS21, R-PS24 testés ; état comparatif | — |
| **PS7** Places avancées | tirage au sort à graine publiée, priorités, places réservées, liste d'attente avec délai | PS1 | R-PS8 : tirage rejouable, test | — |
| **PS8** Rentrée | reconduction de l'offre, réinscriptions (3), invitation des inscrits | PS1 ; 3 | | — |
| **PS9** Partenaires et vitrine | partenaires, offres de partenaires, convention, transmission minimale, accès partenaire, réunion d'information (19), vitrine publique, forum | PS1 ; 7, 17, 19 | DPS-03, DPS-04, DPS-05 impossibles ; parité NP-01 à NP-10 | 2866 |
| **PS10** Covoiturage, fil, pilotage | covoiturage refait, modération ; fil d'activité (16) ; exercices de car ; tableau de bord, indicateurs (47), participation (23, 37) | PS1, PS5 ; 16, 47 | DPS-10 à DPS-15 impossibles ; R-PS27 à R-PS29 testés | 2869 |
| **PS11** Reprise | partenariats, grilles (étude, garderie), covoiturage (si Florian le veut) | PS1, PS3, PS9 ; outil T0 | reprise d'un export anonymisé sans perte des données « à conserver » | 2870 |
| **PS12** Conformité et retrait | purges (§21.3), accès RGPD, IA (§18), retrait de `etude`/`garderie` de `ent_repas_activite` (après reprise, sur décision, E8) | tous ; 27 | §21.3 vérifié ; 6 pièges IA réussis | 2878–2879 |

**Parcours minimal pour la rentrée 2027** : PS0, PS1, PS2, PS3, PS5 (le plus utile au quotidien : qui est attendu ce soir, qui prend quel car). PS4 (AS) pour la Toussaint 2027. Ordre recommandé : PS0 → PS1 → PS2 → PS3 → PS5 → PS4 → PS6 → PS7 → PS9 → PS8 → PS10 → PS11 → PS12. **Rien de ce socle ne remplace un module Next.js en usage réel** (les partenariats ne sont pas déclarés fiables) : pas de bascule datée imposée, contrairement aux socles 42 et 19.

### 25.3 Plage de migrations (2860–2879)
| N° | Lot | Objet |
|---|---|---|
| 2860 | PS0 | responsabilités de groupe datées, reprise de 098, FK composite, vue de compatibilité |
| 2861 | PS1 | entités organisatrices et leurs rôles |
| 2862 | PS1 | activités, cibles, exigences, liens, sessions, créneaux, priorités |
| 2863 | PS1 | inscriptions, jours, faits, tirage, places réservées |
| 2864 | PS2 | séances, encadrants de séance, écarts, absences annoncées, suivi |
| 2865 | PS4 | adhésions, rôles d'élèves, rencontres, convocations, résultats |
| 2866 | PS9 | partenaires (annuaire commun), transmissions, accès partenaire |
| 2867 | PS5 | réseau de transport : versions, lignes, arrêts, courses |
| 2868 | PS5 | affiliations, transport adapté, incidents, exercices |
| 2869 | PS10 | covoiturage |
| 2870 | PS11 | correspondances de reprise |
| 2871–2877 | réserve | |
| 2878 | PS12 | RLS stricte, index de pilotage |
| 2879 | PS12 | retrait des natures `etude`/`garderie` de `ent_repas_activite` et de la vue de compatibilité (sur décision, E8) |

---

## 26. Reprise de Next.js

1. **Aucun accès à la base de production** : export fourni par Florian, anonymisé pour les essais (outil T0, R1), réel seulement après l'accord écrit de l'établissement et l'avis du DPO (D-035 point 4).
2. **Partenariats** (`partenariat_offre`, `partenariat_evenement`, `partenariat_inscription`, signatures S3) : offres de l'année en cours → activités de famille `partenaire` (cycles, niveaux et tarifs JSONB **décodés** en cibles et barème, D-036 ; clé inconnue = ligne refusée et listée) ; inscriptions **rapprochées** à un élève du cœur par clé forte (élève connu de l'établissement par nom, prénom, date de naissance **et** classe, proposé à la file humaine, jamais appliqué automatiquement) ; non rapprochées : **listées pour le responsable**, non reprises ; signatures : **non reprises** (pièce papier ou nouvelle autorisation au 18 si l'offre continue) [R, Q-PS5].
3. **Grilles** (`eleve_scolarite.grille_repas`) : cases « étude » et « garderie » → inscriptions datées du 1er jour de la reprise ; « sort seul » → **proposition** au régime de sortie du 14 (jamais appliquée seule : une autorisation de sortie doit venir d'un titulaire, R-FC43 du 18).
4. **Covoiturage** : **perdable** (ébauche, T0) ; recommandé : **non repris** ; les familles se réinscrivent avec un consentement réel (DPS-10).
5. **Contrôles** : comptages par offre et par état, échantillons, idempotence (deux exécutions = même résultat), annulation d'une exécution.

## 27. Tests et recette
- **Unitaires** : conditions et dérogations ; places en concurrence (100 inscriptions simultanées sur 10 places : 10 confirmées) ; tirage au sort rejouable (même graine, même résultat) ; génération des séances (vacances, fermetures, semaines A/B, changement d'heure) ; présence attendue (sortie, stage, absence annoncée) ; dernier car ; entité émettrice.
- **Intégration** : inscription par un titulaire puis accord du payeur d'un autre foyer ; désaccord connu ; élève majeur ; appel hors ligne puis synchronisation ; écart signalé en moins de 10 minutes ; départ seul refusé sans autorisation ; incident de car et avis aux seuls usagers ; justification en lot ; sortie `rencontre_sportive` créée au 42.
- **Isolation** : matrice entre établissements (tables 2860–2870) ; RLS stricte ; un intervenant ne lit que ses séances ; un responsable élève ne lit aucune coordonnée.
- **Pièges** (§21.2) : l'encadrant n'obtient aucune coordonnée de famille par l'API ; le partenaire n'obtient pas la date de naissance si l'offre ne l'annonce pas ; le covoiturage ne révèle rien avant double acceptation (y compris dans les identifiants) ; un fichier de transport ne crée jamais d'élève.
- **Recette** : parité NP-01 à NP-17 action par action ; parcours famille, encadrant, vie scolaire, secrétaire de l'AS, référent transports ; à 375 px et au clavier (axe-core).

## 28. Réglages signalés (D-023)
| S | Réglage | Défaut |
|---|---|---|
| S-4300 | Qui autorise une activité nouvelle | direction |
| S-4301 | Activité avec intervenant extérieur : autorisation de la direction exigée | oui |
| S-4302 | Familles autorisées à s'inscrire en ligne | oui |
| S-4303 | Élève mineur autorisé à demander | oui |
| S-4304 | Invitation des inscrits de l'an passé à se réinscrire | oui |
| S-4305 | Délai de réponse à une place proposée | 48 h |
| S-4306 | Seuil d'absences répétées | 3 écarts non justifiés |
| S-4307 | Délai de signalement d'un écart | 10 min après le début |
| S-4308 | Heure limite d'inscription ponctuelle à la garderie | 12 h le jour même |
| S-4309 | Résultats de l'AS publiés nominativement aux adhérents | non (par équipe) |
| S-4310 | Garde d'honorabilité bloquante pour intervenants et bénévoles | oui |
| S-4311 | Photo des inscrits dans l'appel de l'intervenant | non |
| S-4312 | Plafond de propositions de covoiturage par jour | 3 |
| S-4313 | Exercice d'évacuation de car proposé à la rentrée | oui (bonne pratique) |
| S-4314 | Covoiturage activé | non |
| S-4315 | Espace covoiturage des personnels | non |

## 29. Questions et décisions

### 29.1 Appliquées par défaut (D-031)
| Q | Question | Recommandation appliquée |
|---|---|---|
| Q-PS1 | Le 43 possède-t-il les groupes d'activité ? | Non : groupe du 5 (nature `activite`), offre et inscriptions au 43 |
| Q-PS3 | Réservation des salles des séances ? | Par le 19 (Q-AG1) |
| Q-PS4 | Le planning de domaines (EVARS) au 43 ? | Non : 19, AG6b (Q-AG2) |
| Q-PS7 | Absence à l'étude = absence scolaire ? | Non : écart signalé, hors décompte d'assiduité ; sauf heure de permanence de l'EDT (12) |
| Q-PS10 | Désinscription automatique après absences ? | Non : alerte seulement |
| Q-PS11 | Tirage au sort par l'IA ? | Non : déterministe, graine publiée |
| Q-PS12 | Critères sociaux ou de santé comme priorité ? | Jamais visibles ; places réservées décidées par la direction |
| Q-PS13 | Certificat médical pour l'AS ? | Non (L552-1) ; consigne d'inaptitude sans motif |
| Q-PS14 | Rencontre hors de l'établissement : 43 ou 42 ? | Convocation et résultats au 43 ; déplacement = sortie `rencontre_sportive` du 42 |
| Q-PS15 | Formulaire public d'inscription (comme Next.js) ? | Non : espace famille ; vitrine sans collecte |
| Q-PS16 | Date de naissance au partenaire ? | Seulement si l'offre l'annonce et la convention le prévoit |
| Q-PS17 | Saisie des abonnements de transport ? | Non : import ou déclaration ; l'autorité organisatrice fait foi |
| Q-PS18 | Avis de car supprimé : à qui ? | Aux usagers de la course seulement, fenêtre R-N50 |
| Q-PS19 | Justification automatique des retards de car ? | Proposée, décidée par la vie scolaire (N2) |
| Q-PS20 | Retenue après le dernier car ? | Possible avec avis préalable à la famille (R-PS22) |
| Q-PS21 | Covoiturage : adresse ou géolocalisation ? | Jamais : commune ou code postal |
| Q-PS22 | Covoiturage activé par défaut ? | Non |
| Q-PS23 | Covoiturage Next.js repris ? | Non (consentement non recueilli) |
| Q-PS24 | Responsables élèves de la maison des lycéens ? | Oui, droit restreint (R-PS33) |
| Q-PS25 | Suivi de l'accompagnement en texte libre ? | Non : faits codés |
| Q-PS26 | Exercice d'évacuation de car obligatoire ? | Bonne pratique proposée (S-4313), pas d'obligation affichée |
| Q-PS27 | Devoirs faits en 6e : facultatif ? | Non : inscription par l'établissement (BO 2023) |
| Q-PS28 | Durées de conservation | §21.3, à confirmer avec le DPO |
| Q-PS29 | Responsables de groupe : remplacement global ? | Non : responsabilités datées (PS0) |
| Q-PS30 | Activité d'un tiers dans les locaux sans convention ? | Refusée (L213-2-2, L214-6-2) |
| Q-PS31 | Taux d'encadrement de la garderie ? | Contrôlé pour le premier degré ou un accueil déclaré (R227-16) |

### 29.2 Soumis à Florian (engagent l'établissement, des données réelles, ou des faits que seul Florian connaît)
| Q | Question | Recommandation |
|---|---|---|
| **Q-PS2** | L'argent des associations (AS, foyer, APEL, maison des lycéens) : ScolIA l'encaisse-t-il pour leur compte (compte bancaire distinct, entité émettrice) ? | **Suivi seulement** dans V1 ; encaissement par le 25 sous l'entité de l'association seulement sur décision et avec l'accord du trésorier de l'association |
| **Q-PS5** | Reprise des inscriptions et signatures de partenariats de Next.js | offres de l'année reprises, inscriptions rapprochées en file humaine, signatures non reprises |
| **Q-PS6** | Un partenaire peut-il encaisser via ScolIA ? | Non dans V1 (le paiement reste chez le partenaire) |
| **Q-PS8** | L'établissement est-il organisateur délégué de transports (L3111-9) ? Quelle région, quel fichier disponible ? | à confirmer pour l'établissement pilote ; demander à la région le format du fichier des usagers |
| **Q-PS9** | Procédure réelle de contrôle d'honorabilité des intervenants et bénévoles dans l'établissement | garde active (S-4310) ; à aligner sur la future loi |
| **Q-PS32** | Activer le covoiturage dans l'établissement pilote | non à la rentrée 2027 ; à proposer après avis du DPO |
| **Q-PS33** | Le DPO valide-t-il les durées (§21.3) et la notice du covoiturage ? | à soumettre avant PS10 |

---

## 30. Ajouts personas (`PERSONAS_43.md`, 2026-11-11)

**En cas de contradiction avec les §0 à §29, ce paragraphe l'emporte.** Manques M-4301 à M-4342 ; quatre bloquants (A à D) appliqués par défaut (D-031).

### 30.1 Bloquants tranchés
| # | Bloquant | Décision appliquée | Effet sur la conception |
|---|---|---|---|
| A | Annonce d'absence ou de départ seul qui efface l'attendu | l'élève **reste attendu** (« départ annoncé à 16 h 30 par [titulaire] ») jusqu'au départ **marqué** par l'encadrant ; annonce impossible depuis un compte élève ; passkey sur appareil inconnu ; avis immédiat à tous les titulaires ; annonce par une personne visée par une interdiction refusée et signalée | R-PS36 ; PSC-241, PSC-242 ; §10.2, §12.2 précisés |
| B | Pas de départ en cours de séance | départ anticipé = remise du 14 depuis l'appel, à tout moment ; présents mis à jour ; liste des présents imprimable et hors ligne | R-PS38 ; PSC-240, PSC-247 |
| C | Avis de car limité aux affiliés connus | page « état des transports du jour » visible de toutes les familles ; avis élargi proposé aux communes desservies quand l'affiliation connue est incomplète ; l'avis renvoie à la page | R-PS37 ; PSC-245, PSC-246 |
| D | Aucun plafond d'encadrement | capacité de sécurité (élèves par encadrant) par activité, bloquante à l'ouverture de l'appel (second encadrant proposé) ; taux R227-16 pour la garderie du premier degré ou un accueil déclaré | R-PS34 ; PSC-243, PSC-244 |

### 30.2 Actions ajoutées (PSC-240 à PSC-259)
| PSC | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PSC-240 | Marquer un départ anticipé en cours de séance (remise du 14) | ENC · `periscolaire.appel` | R-PS38 | `accueil.remise` (14) | PS2 |
| PSC-241 | Confirmer une annonce d'absence ou de départ par passkey (appareil inconnu) | TIT · `soi` | R-PS36 | `periscolaire.absence_annoncee` | PS2 |
| PSC-242 | Aviser immédiatement tous les titulaires d'une annonce | SYS | R-PS36 | — | PS2 |
| PSC-243 | Déclarer la capacité de sécurité (élèves par encadrant) d'une activité ; blocage de l'appel au-delà | RSP · `periscolaire.activites_gerer` | R-PS34 | `periscolaire.capacite_depassee` | PS2 |
| PSC-244 | Contrôler le taux R227-16 d'une garderie du premier degré | SYS | âge des enfants, durée de l'accueil, PEdT | idem | PS3 |
| PSC-245 | Publier la page « état des transports du jour » | SYS ; RTR | R-PS37 | — | PS5 |
| PSC-246 | Proposer l'avis élargi aux familles des communes desservies | SYS → RTR, DIR | N3 | `notifications.diffusee` (8) | PS5 |
| PSC-247 | Imprimer ou télécharger hors ligne la liste des présents hors temps scolaire par lieu (alerte) | DIR, CPE, ACC · `periscolaire.lire` | effacée du mobile à la fin de l'alerte | — | PS2 |
| PSC-248 | Inviter un intervenant sans compte (identité ScolIA, rôle restreint à son activité, expiration) | DIR · `periscolaire.autoriser` | 1, 6 | `periscolaire.acces_intervenant` | PS1 |
| PSC-249 | Retirer immédiatement l'accès d'un intervenant (fin de convention, signalement) | DIR | | idem | PS1 |
| PSC-250 | Journaliser et plafonner les exports de listes (responsable, partenaire) | SYS | une fois par jour ; colonnes minimales | — | PS1 |
| PSC-251 | Regrouper les écarts d'une séance avec la dernière présence connue de l'élève (12, 14) | SYS → VS | M-4301 | — | PS2 |
| PSC-252 | Déléguer l'appel d'une séance à un autre encadrant présent | ENC | journal | — | PS2 |
| PSC-253 | Signaler l'accessibilité du lieu au catalogue et aviser la direction d'une demande d'un élève à besoin d'accessibilité (sans motif) | SYS | 5, 35 | — | PS1 |
| PSC-254 | Reconduire les inscriptions d'étude et de garderie de l'an passé, avec confirmation des familles (juin) | ADM · `periscolaire.inscriptions_gerer` | 3 | `periscolaire.inscription_demandee` | PS8 |
| PSC-255 | Alerter sur les créations et suppressions répétées de profils de covoiturage | SYS → DIR | > 3 en un mois | — | PS10 |
| PSC-256 | Contrôler la déclaration de l'AS : président = chef d'établissement (public), secrétaire professeur d'EPS, trésorier majeur, composition du comité | SYS | R552-2 | — | PS4 |
| PSC-257 | Copier le texte d'engagement accepté sur l'inscription (version figée) | SYS | corrige DPS-18 | — | PS1 |
| PSC-258 | Choisir, à l'expiration du délai d'accord du payeur, de payer soi-même ou de retirer l'inscription | TIT inscrivant | S-4316 | `periscolaire.inscription_confirmee` / `_close` | PS1 |
| PSC-259 | Publier le rapport quotidien « non récupérés après fermeture » à la direction | SYS | R-PS18 | — | PS3 |

### 30.3 Règles ajoutées et précisées
| R | Règle |
|---|---|
| R-PS34 | **Capacité de sécurité** : une séance dont les présents dépassent le nombre d'élèves par encadrant déclaré bloque l'ouverture de l'appel jusqu'à l'ajout d'un encadrant ou la décision motivée de la direction ; pour une garderie du premier degré organisée en accueil de loisirs, le taux de CASF R227-16 s'applique (moins de 6 ans : 1 pour 8 ou 10 ; 6 ans et plus : 1 pour 12 ou 14 ; assouplissement PEdT). |
| R-PS35 | Une activité dans un **lieu extérieur** exige une convention valide rattachée au lieu (5, 7) ; une activité d'un **tiers** dans les locaux n'est pas une activité du 43 : c'est une mise à disposition (convention tripartite, 44 §16.1). |
| R-PS36 | **Rien ne retire un élève des attendus, sauf le geste d'un adulte responsable qui l'a vu partir** (départ marqué, remise, prise en charge du car) ; une annonce (absence, départ seul) est une information affichée avec son auteur ; elle n'est jamais faite depuis un compte élève ; depuis un appareil inconnu, elle exige une passkey ; tous les titulaires sont avisés ; une annonce par une personne visée par une interdiction est refusée et signalée. Exception : l'absence annoncée **pour la journée entière** (élève absent du 12 dès le matin) neutralise la présence attendue du soir. |
| R-PS37 | **L'absence d'avis ne vaut jamais assurance** : l'état des transports du jour est publié pour toutes les familles ; l'avis de suppression renvoie à cette page ; quand l'affiliation connue d'une ligne est incomplète (taux < S-4317), l'avis élargi est proposé. |
| R-PS38 | Un départ en cours de séance passe par la remise du 14 (personne autorisée, ou autorisation de sortie seul valide) ; l'élève sort de la liste des présents à l'heure du geste. |
| R-PS1 (précisée) | Chaque commande d'inscription vérifie **côté serveur** le lien titulaire → élève (corrige DPS-17). |
| R-PS4 (précisée) | Un titulaire visé par une mesure de protection du 4 ne voit ni les horaires de départ ni la personne qui vient chercher (même règle que R-AG12 du 19). |
| R-PS13 (précisée) | **Priorité des présences attendues** : cours > retenue > sortie (42) ou stage > activité > étude > garderie ; une présence de rang supérieur au même moment fait apparaître l'élève « ailleurs » dans l'appel inférieur. L'attendu n'existe que si l'organisateur est l'établissement **ou** si un personnel encadre (activité de l'association sans personnel : liste de présence de l'association, sans écart). |
| R-PS19 (précisée) | Le pilotage ne présente aucun indicateur nominatif par encadrant (forfait AS compris). |

### 30.4 Cas limites ajoutés (CPS-45 à CPS-56)
| CPS | Cas | Traitement |
|---|---|---|
| CPS-45 | L'élève annonce son propre départ depuis le compte de sa mère sur un appareil inconnu | passkey exigée ; à défaut, annonce refusée ; avis aux deux titulaires |
| CPS-46 | Le père vient chercher Léo à 17 h 20, l'étude finit à 18 h | départ anticipé (remise du 14) |
| CPS-47 | Alerte PPMS à 17 h 40, réseau coupé | liste des présents imprimée ou hors ligne par lieu |
| CPS-48 | Car supprimé ; la famille de Léna n'est pas affiliée | page « état des transports » ; avis élargi aux communes desservies |
| CPS-49 | Étude de 80 élèves, un seul AED | appel bloqué (capacité de sécurité 40, par exemple) ; second encadrant ou décision motivée |
| CPS-50 | Garderie de 45 enfants de 3 à 6 ans avec deux animatrices | taux R227-16 dépassé : alerte et blocage |
| CPS-51 | Élève en retenue et inscrit à l'étude au même moment | « en retenue » dans l'appel de l'étude |
| CPS-52 | Ciné-club de la MDL sans personnel présent | pas d'attendu de l'établissement ; liste de présence associative |
| CPS-53 | Une compagnie extérieure veut utiliser la salle polyvalente pour ses adhérents | mise à disposition au 44, pas une activité du 43 |
| CPS-54 | Le père payeur ne répond pas à l'accord de paiement | 7 jours, puis choix de la mère ; deux séances d'essai |
| CPS-55 | L'offre de partenaire change son texte d'engagement après les inscriptions | les inscrits gardent la version acceptée (PSC-257) ; nouvel accord demandé si le changement porte sur le prix ou les obligations |
| CPS-56 | Taxi du transport adapté à 16 h 30, club jusqu'à 17 h 30 | conflit signalé comme pour le dernier car ; la famille voit ce qu'elle doit demander au département |

### 30.5 Questions nouvelles
Q-PS34 à Q-PS38 (soumises à Florian, `PERSONAS_43.md` §8.2) ; réglages ajoutés : **S-4316** délai d'accord du payeur (7 jours), **S-4317** taux d'affiliation en dessous duquel l'avis élargi est proposé (90 %), **S-4318** séances d'essai pendant l'accord (2), **S-4319** capacité de sécurité par défaut d'une étude (40 élèves par encadrant).

### 30.6 Lots révisés
- **PS1** reçoit PSC-248 à PSC-250, PSC-253, PSC-257, PSC-258 (identité de l'intervenant, exports, engagement figé).
- **PS2** reçoit les bloquants A, B, D (PSC-240 à PSC-243, PSC-247, PSC-251, PSC-252) : **aucune séance d'étude en production sans eux**.
- **PS3** reçoit PSC-244 et PSC-259 ; **PS5** reçoit le bloquant C (PSC-245, PSC-246) ; **PS8** reçoit PSC-254 et doit être prêt **en juin 2027** (réinscriptions).
- **Parcours minimal en production** pour la rentrée 2027 : PS0, PS1, PS2, PS3, PS5, plus **PS8 (juin)** ; PS6 en mode suivi pendant le doublon (D-034).
- Hors du 43 : nature `rencontre_sportive` à ajouter au **42** ; priorité des présences attendues à écrire au **12** ; rôle intervenant restreint au **1/6** (`PERSONAS_43.md` §6).

## 31. Faits, hypothèses, contradictions

**Faits [F]** : Next.js : partenariats (tables, JSONB `cycles`, `niveaux`, `tarifs`, `cta_links`, signatures S3, places hors transaction, route publique d'inscription), covoiturage (collections KV, consentement fixé à l'inscription, effacement absent), grille JSONB `eleve_scolarite.grille_repas` ; aucun module de clubs, AS, études gérées ni transports quotidiens. Rust : `ent_groupe` (092) sans nature ni capacité ; `ent_groupe_responsable` (098) remplacé par DELETE/INSERT (`droits_repo.rs:760-781`), contrôle et écriture en deux transactions (`735`, `760`) ; `ent_repas_activite.kind` contient `etude` et `garderie` (017) ; aucune action D-017 ni table pour activités et transports ; plus haute migration écrite : 642 ; aucun fichier dans 2860–2899.

**Hypothèses [H]** : calendrier de l'année (§4) ; existence d'un fichier des usagers fourni par la région de l'établissement pilote ; usage de l'export de licences UNSS ; maturité « partiel » des partenariats (non confirmée par Florian).

**Contradictions relevées** :
1. `SOCLES.md` §43 annonce « inscriptions et places ; appel ; facturation ; transports » : conforme ; il cite comme existant « groupes et ateliers avec responsables (S2-2) » : ce sont des groupes sans nature, insuffisants (RPS-04).
2. `05_STRUCTURE.md` donne au 5 les groupes et ateliers « avec responsables » et au 43 « inscriptions et facturation des activités » : conforme ; mais la nature `activite` et la capacité (R-S15) ne sont pas codées : **prérequis** de PS1 à demander au socle 5.
3. `19_AGENDA_RDV.md` (§12.2, AG6b) a pris le planning de domaines, que la mission de ce socle citait parmi les modules à reprendre : le 43 s'y range (Q-PS4) ; aucune contradiction de fond.
4. `40_RESTAURATION.md` Q-RS10 renvoie étude et garderie de la grille au 43 : repris (§12, PS3, PS11).
5. `24_FINANCES.md` (« transport scolaire souvent régional : import, pas facturation ») : confirmé par L3111-7 (§5.1).
6. `08_NOTIFICATIONS.md` nomme `transport.inscription_modifiee` (singulier), `10_EMPLOI_DU_TEMPS.md` `transport.inscriptions_modifiees` (pluriel) : ce document retient **`transport.inscriptions_modifiees`** (le 8 est à aligner).
