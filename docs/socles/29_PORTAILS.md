# Socle 29 — Portails, « Aujourd'hui » et à-faire unifié

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui est isolé au §27.2. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le code Next.js a été **lu seulement**, de façon ciblée, sur `origin/main` par `git show` (rien extrait sur disque, rien exécuté, rien construit).
**Branche :** `ent-socle-29-conception`, partie de `origin/ent-mc0hoe` (`7467f2d4`).
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (D-011, D-012, **D-013**, D-014, **D-017**, D-020, D-022, **D-023**, D-024, **D-026**, D-028, D-031, **D-034**, **D-035**, **D-036**, D-037 à D-041) ; `docs/SOCLES.md` §29 et §10 (T-01) ; `docs/FRONTEND_CIBLE.md` §2 (direction artistique, Bento) ; `docs/DESIGN_SYSTEM.md` ; `docs/FRONTEND_LEPTOS.md` ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; `docs/TRANSITION_T0_INVENTAIRE.md` (§3.0 : « Pilotage, tableau de bord » = **ébauche**) ; `docs/socles/IA_COEUR.md` §5 et §6 ; les conceptions qui déposent des tâches : `02_ELEVE.md`, `03_INSCRIPTIONS.md`, `04_ADMINISTRATIF.md`, `05_STRUCTURE.md`, `06_PERSONNELS.md`, `07_DOCUMENTS.md`, **`08_NOTIFICATIONS.md`**, `09_INTEROP.md`, `10_EMPLOI_DU_TEMPS.md`, **`16_MESSAGERIE.md`**, `21_EVALUATIONS.md`, `24_FINANCES.md`, `25_PAIEMENTS.md`, `34_SANTE.md`, `42_SORTIES_VOYAGES.md`.
**Conventions.** **[F]** fait vérifié dans le dépôt (chemin et ligne cités) ; **[V]** règle lue sur une source officielle (URL citée, consultée le 2026-11-11) ; **[NV]** règle non vérifiée sur le texte officiel ; **[H]** hypothèse (terrain, marché) ; **[R]** recommandation (non décidée).
**Numérotation (plages libres vérifiées sur les 124 branches distantes le 2026-11-11) :** actions **POR-100** et suivantes ; règles **R-PO1** ; cas limites **CPO-01** ; défauts de l'existant **DP-01** ; existant Next.js **NXP-01** ; questions **Q-PO1** ; réglages signalés **S-2900** ; manques de la passe personas **M-2900 à M-2999** (`PERSONAS_29.md`).
**Plage de migrations imposée : 2580–2599** (registre `MIGRATIONS.md`, §23).

**Passe personas (ajout personas, 2026-11-11)** : `docs/socles/PERSONAS_29.md` (16 personas, dont un attaquant et un auditeur RGPD). Ses compléments sont intégrés ici, marqués **« ajout personas »**, sans rien supprimer : §29 pour l'essentiel, plus des renvois ponctuels.

Sommaire : §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs · §4 L'année et la journée vues de l'accueil · §5 Réglementaire · §6 Frontières et arbitrages · §7 Concepts · §8 L'à-faire unifié · §9 « Aujourd'hui » par profil · §10 Modes et bandeaux · §11 Plusieurs enfants, casquettes, établissements · §12 Recherche et palette · §13 Personnalisation · §14 Accessibilité, langues, mobile, performance · §15 Droits et minimisation · §16 Cas limites · §17 IA · §18 Actions · §19 Règles · §20 Événements · §21 Écrans · §22 Modèle de données · §23 Migrations · §24 Lots · §25 Tests et recette · §26 Réglages signalés · §27 Décisions · §28 Faits, hypothèses, contradictions · §29 Compléments de la passe personas.

---

## 0. En bref

1. **Le socle 29 ne possède aucune donnée métier.** Il possède trois choses : (a) l'**index des tâches** — une projection de ce que les autres socles attendent de chacun, alimentée par leurs événements, revérifiée à chaque lecture, reconstructible à tout moment ; (b) la **composition** des pages d'accueil (« Aujourd'hui ») à partir des tuiles que chaque socle déclare ; (c) la **recherche globale** et la **palette de commandes**, qui interrogent les socles sous les droits du lecteur. Tout le reste — l'absence à justifier, la facture, la signature — reste chez son propriétaire, et c'est lui qui exécute l'action.
2. **Ce qu'on trouve aujourd'hui** [F] : un accueil Leptos Bento correct pour l'**enseignant** (prochain cours, « Faire l'appel », séances, messages) mais **presque vide pour une famille** : la tuile Annonces appelle une route réservée au personnel et affiche toujours « Annonces non disponibles pour votre profil » ; pas d'emploi du temps ; une tuile « Votre espace » qui renvoie vers l'ancienne interface `ui.rs` (F-001) ; une tuile d'absences qui fait **une requête par enfant** et télécharge **toutes les notes** pour compter des absences. Secrétariat, direction, comptabilité et accueil sont renvoyés à `ui.rs`. Les tuiles dépendent du **rôle fixe** et non des droits D-017 : un rôle « Accueil » paramétré ne voit rien. Correctifs : **lot P0** (§2.3, §24).
3. **Next.js** a un accueil « signaux » riche (≈ 30 signaux : appels manquants, absences à valider, carnets non signés, factures en retard, signatures de stages, appel du soir d'internat, demandes, HSE, salles…), une barre de tâches par rôle et une recherche d'élève. Florian l'a classé **ébauche** (`TRANSITION_T0_INVENTAIRE.md` §3.0) : on le **refait** par la méthode des socles, mais sa **liste de signaux** est la meilleure inventaire terrain disponible et entre au §2.4 « à ne pas perdre ». Deux défauts à ne pas reproduire : la recherche **télécharge la liste de tous les élèves inscrits dans le navigateur** (NXP-14), et les signaux sont **mis en cache 60 s par personne** sans invalidation sur changement de droits (NXP-16).
4. **Une frontière nette entre avis et tâche** (arbitrage des 14 contradictions relevées entre conceptions, §6.2) : un **avis** (socle 8) dit qu'il s'est passé quelque chose ; une **tâche** (socle 29) dit que **quelqu'un doit faire quelque chose**, a un état, se ferme quand c'est fait, et éteint alors les avis et relances qui la concernaient. Chaque socle déclare ses **natures de tâche** dans son kit de branchement (D-026), comme il déclare ses natures d'avis au socle 8.
5. **Les files de travail des socles restent chez eux** (inscriptions à instruire, boîtes de service, Centre des échanges, file financière, plis de l'infirmerie…) ; le 29 les **compte et y renvoie** dans « Aujourd'hui », et n'y met en ligne que les tâches **nominatives** ou **collectives prises en charge**. Une seule tâche apparaît une seule fois, chez la bonne personne, avec sa raison.
6. **Familles** : un à-faire **par enfant et tous enfants**, avec des actions en **deux gestes** (justifier, signer, autoriser, payer, répondre, prendre rendez-vous, réinscrire, compléter la fiche de rentrée) exécutées par le socle propriétaire avec aperçu et confirmation. **Parents séparés** : chacun voit ses tâches ; une tâche faite par l'autre responsable disparaît **sans dire qui l'a faite ni quand** (R-V46 du socle 21, R-PO7). **Parent sans exercice de l'autorité** : informé, jamais sollicité pour une décision. **Élève majeur** : il devient le destinataire de ses tâches.
7. **« Aujourd'hui » par profil** (§9) : 22 profils décrits tuile par tuile, Bento 12 colonnes, **une information et une action par tuile**, jamais de saisie sur l'accueil (D-023, `FRONTEND_CIBLE.md` §2). Le **temps** structure la page : « maintenant », « ensuite », « demain » ; et des **modes** la transforment (rentrée, conseil, vacances, examens, fermeture, **alerte PPMS** où la page entière devient « qui dois-je avoir avec moi »).
8. **Plusieurs établissements** : la réunion se fait **sur l'appareil de la personne**, jamais sur le serveur (D-013, comme NTF-14, EDT-107, DCT-158) ; le web offre un sélecteur et, plus tard, des **compteurs seuls**. **Plusieurs casquettes** (professeur et parent) : une casquette à la fois, jamais d'union de droits (PRS-29).
9. **Recherche globale et palette** (§12) : côté serveur uniquement, fédérée auprès des socles sous les droits du lecteur, au moins deux caractères, résultats plafonnés, **aucune donnée de santé ni motif indexé**, élèves à confidentialité renforcée et adresses protégées jamais révélés aux non-habilités, compteur anti-énumération. La palette réutilise **le registre de capacités de l'IA** (IA_COEUR §5) : mêmes commandes, même simulation, même confirmation.
10. **Un texte change la conception** [V~] : l'**arrêté du 7 septembre 2026** (cadre de référence du numérique pour l'éducation) impose aux ENT du public de **suspendre par défaut la diffusion des nouvelles informations de 20 h à 7 h et du vendredi 20 h au lundi 7 h**, consultation et saisie restant possibles : une tâche ou une tuile nouvelle n'apparaît à une famille ou à un élève qu'à la fin de la plage (R-PO31), sauf ajustements préparés par le chef d'établissement (R-PO37) ; il impose aussi le RGAA, applis mobiles comprises, et le DNMA pour la mesure d'audience (§5).
11. **Chiffres** : **152 actions** (POR-100 à POR-251) portées à **167** par la passe personas (POR-252 à POR-266), **86 cas limites** (CPO-01 à CPO-86), **38 règles** (R-PO1 à R-PO38), **17 lots** (P0 à P16) dont **P0 correctifs** et **P1 « démo de décembre »** ; 16 tables, aucune colonne JSON, migrations 2580–2599. Parcours minimal vendable : P0, P1, P2, P3, P4, P5, P7, P12.
12. **(ajout personas)** La passe (`PERSONAS_29.md`, 16 personas, 48 manques) trouve **quatre bloquants** aux frontières de l'appareil, appliqués par défaut : (A) le **cache hors ligne** d'un parent dont le lien vient d'être suspendu garderait l'emploi du temps de l'enfant → purge et expiration (R-PO33) ; (B) des compteurs multi-établissements par CORS révéleraient les autres adhésions → guichet central (R-PO34) ; (C) le mode discret masquerait l'**alerte de restriction de remise** → alerte neutre toujours visible (R-PO35) ; (D) la démo de décembre doit être écrite sur le **contrat de tuile** pour ne pas être refaite (R-PO36). Elle ajoute la **neutralité temporelle** entre parents séparés (R-PO38) et l'information des « choix importants » au parent sans exercice de l'autorité (POR-261).

---

## 1. Thèse : l'accueil ne possède rien, il montre à chacun ce qui l'attend maintenant

Une famille ne veut pas « naviguer dans un ENT ». Elle veut savoir, en ouvrant l'appli dans le bus : **y a-t-il quelque chose que je dois faire, et mon enfant a-t-il cours normalement aujourd'hui ?** Un professeur veut savoir **où il doit être, ce qu'il a oublié, ce qui a changé**. Une CPE veut savoir **qui manque, qui n'a pas fait l'appel, qui attend une réponse**. Une directrice veut savoir **ce qui ne peut pas attendre**. Le marché répond par des menus (dix entrées, chacune avec sa pastille rouge) [H : constat partagé par Florian sur Charlemagne, D-023, et par les personas des socles 8 et 16]. ScolIA répond par une page qui **dit les choses dans l'ordre où il faut les faire**.

Principes [R] :
1. **Aucune donnée métier propre.** Le 29 ne stocke ni absence, ni facture, ni note. Il stocke un **index** (qui doit faire quoi, sur quel objet, pour quand, dans quel état), des **préférences d'affichage** et quelques paramètres. L'index est **jetable** : on peut le reconstruire à partir des socles propriétaires (§8.10). C'est la règle 1 de la carte (« un socle possède ses données ») appliquée à un socle d'agrégation.
2. **Le propriétaire décide, le 29 montre.** Une nature de tâche est déclarée par son socle (titre neutre, destinataires, échéance, action de clôture, sensibilité). Ouvrir et fermer une tâche est un **fait du propriétaire**, publié dans sa transaction (outbox). Le 29 n'ouvre ni ne ferme jamais une tâche métier de lui-même (sauf les rappels personnels, §8.12, et la mise « sans objet » sur décès ou départ, demandée par le propriétaire).
3. **Les droits sont relus au moment de l'affichage.** Une tâche indexée n'est montrée que si le socle propriétaire confirme, **à l'instant de la lecture**, que la personne peut encore agir (lot de vérification par socle, §8.3). Un lien suspendu par décision de justice à 10 h 02 fait disparaître les tâches à 10 h 02, sans attendre un cache.
4. **Une tâche existe une fois, chez la bonne personne, avec sa raison.** Pas de doublon entre le centre de notifications (8), la messagerie (16) et l'à-faire : la tâche a une identité stable `(nature, objet, destinataire)` ; l'avis renvoie à la tâche ; la relance s'éteint quand la tâche se ferme. Chaque tâche répond à « pourquoi est-ce à moi ? » et « que se passe-t-il si je ne le fais pas ? ».
5. **Agir en deux gestes, sans quitter l'accueil quand c'est simple.** Justifier une absence, signer un document, autoriser une sortie, payer une échéance, accepter un créneau : une **carte d'action** appelle la commande du propriétaire avec **aperçu** et **confirmation** (même cycle que l'IA, IA_COEUR §6). Ce qui est complexe ouvre l'écran du propriétaire au bon endroit.
6. **Le temps structure la page.** « Maintenant » (le cours en cours, l'appel à faire), « ensuite » (la journée), « bientôt » (échéances de la semaine), « demain » (préparé la veille). Une page d'accueil le dimanche soir n'est pas celle du lundi 8 h.
7. **Des modes, pas des réglages.** La rentrée, la veille d'un conseil, les vacances, la fermeture, la grève, l'alerte : la page change d'elle-même selon les faits des socles 5, 10, 14, 23, sans qu'un administrateur ne configure un tableau de bord (D-023).
8. **Sobriété.** Pas de fil infini, pas de gamification, pas de pastille qui ment (un compteur compte des tâches réellement actionnables par la personne), pas d'animation sur ce qui est vu cent fois par jour (`FRONTEND_CIBLE.md` §2). « Rien à faire » est un état à part entière, affiché sobrement.
9. **Minimiser ce que l'accueil révèle.** L'accueil est l'écran le plus regardé, le plus photographié, le plus souvent ouvert dans une salle de classe projetée ou devant un tiers. Titres neutres, aucun motif, aucun détail de santé ou de discipline sur la page ; le détail s'ouvre après un geste, avec les droits de l'écran propriétaire. **Mode discret** pour la projection (§14.5).
10. **Une seule casquette à la fois, un seul établissement par session serveur.** Le professeur-parent choisit sa casquette ; le parent de deux établissements voit la réunion de ses pages **sur son appareil** (§11).
11. **La recherche et la palette ne donnent aucun droit.** Elles trouvent ce que la personne pourrait déjà ouvrir, et rien d'autre ; elles résistent à l'énumération (§12).
12. **Le même accueil sur le web et le mobile**, avec la même liste de tâches et les mêmes droits (D-003, socle 11) ; le mobile ajoute le hors-ligne en lecture et la réunion multi-établissements.

**Ce qui fera la différence** [H, comparaison de mémoire avec Pronote, École Directe, Charlemagne] : un **à-faire unique** au lieu de dix pastilles ; la **carte d'action** qui fait la chose sans chercher le menu ; « **ce qui a changé depuis hier** » ; la tâche faite qui **éteint d'elle-même ses relances** ; le **parent séparé** qui n'apprend rien de l'autre ; le **professeur qui voit sur une page sa journée de deux établissements** ; la **page d'alerte** qui remplace l'accueil pendant un PPMS.

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust [F]

Lu sur `origin/ent-mc0hoe` (`7467f2d4`). Lecture seule.

| Brique | Où | Ce qu'elle fait |
|---|---|---|
| Accueil Leptos « Aujourd'hui » | `ent-rust/web/crates/ent-web-app/src/modules/accueil/mod.rs` (427 l.), `modele.rs` (92 l., testé) | en-tête daté « Bonjour / Voici votre journée » ; grille Bento ; tuiles : prochain cours + « Faire l'appel » (8 col.), séances du jour (4), absences du jour de la vie scolaire (6), protections à confirmer (6, direction), absences à justifier famille (6), messages (6), annonces (6) ; tuile « Votre espace » qui renvoie à `/app` pour les profils sans emploi du temps |
| Composants | `ent-web-ui` : `GrilleBento`, `Tuile` (largeurs 4/6/8/12, action, accent), `EtatVide`, `Squelette`, `Badge`, `LienBouton` | contrat d'affichage générique, sans notion métier — **bonne base**, conforme à `FRONTEND_CIBLE.md` §3 |
| Coquille et droits | `noyau/coquille.rs`, `noyau/droits.rs` (`Droits` déduits du **rôle fixe**, l. 131–145), `noyau/droits_moteur.rs` (`GET /roles/mes-droits`), `noyau/navigation.rs` (≈ 55 entrées), `noyau/non_lus.rs` (pastille de messages) | navigation filtrée ; compteur de non-lus partagé |
| Routes lues par l'accueil | `/edt/mes-creneaux`, `/messages/fils`, `/annonces` (`routes.rs:482`), `/vs/absences` (à traiter du jour), `/dossier/alertes-protection`, `/famille/enfants` (`routes.rs:663`), `/famille/enfants/{id}/resume` (`routes.rs:664`, gestionnaire `routes.rs:10569`) | aucune route d'accueil composée : le navigateur assemble |
| Résumé enfant | `famille_resume_enfant` (`routes.rs:10569-10640`) : absences à justifier, carnet à signer, **notes** | contrôle du lien correct (`get_eleve_for_actor` puis `lien_parent_eleve`), drapeaux du lien respectés |
| Recherche | `GET /eleves/search` (`routes.rs:197, 1093`), `search_eleves_pour_acteur` (portée du lecteur, groupes de champs E1, lot E6 : `pg_trgm`, `unaccent`) ; `GET /personnel/search` | **bonne base** : serveur, portée, plafond 50 |
| Notifications in-app | `GET /notifications` (`mes_notifications`, 100 dernières, `ent_notification` de 102) | liste brute, pas de lien avec une tâche |
| Copilote | `/copilote/ask`, `/copilote/capacites` (`ent-db/src/copilote.rs`, déterministe) | capacités déclarées, base du registre IA |
| Ancienne interface | `ent-api/src/ui.rs` : accueils famille, élève, vie scolaire, enseignant en JavaScript (« Repas aujourd'hui », « À traiter aujourd'hui », « Effectif 4B aujourd'hui », l. 9895–34627) | **erreur de départ F-001** : figée, à retirer rôle par rôle (lot P14) |

### 2.2 Next.js (lecture ciblée, `origin/main`) [F]

Florian classe « Pilotage, tableau de bord, supervision » en **ébauche** (`TRANSITION_T0_INVENTAIRE.md` §3.0, ligne 87) et le « tableau de semaine » en **perdable** (ligne 124). D-035 précision 2 : on **refait de zéro** ; on garde l'inventaire des usages.

| # | Fonction | Où | Remarque |
|---|---|---|---|
| NXP-01 | Accueil « chat first » : barre ScolIA, tâches de rôle, signaux regroupés | `app/(admin)/dashboard/page.tsx` | la barre d'assistant en tête de page est l'intuition à garder (socle 30) |
| NXP-02 | **Tâches principales par rôle** (raccourcis : absences accueil, appel internat, appels surveillant, dossiers élèves, absences RH, sorties, salles, photocopies, demande d'absence, demande d'HSE, mon planning) | `app/lib/home-main-tasks.ts` | ce sont des **raccourcis**, pas des tâches : → §13.2 « raccourcis du profil » |
| NXP-03 | **Signaux** agrégés (`getDashboardSignals`) : sorties du jour / de la semaine, créneaux vidés par une sortie, messages des séjours, appel du soir d'internat (fait / en attente), conventions de stage à signer, certificats, facturation familles, compta et paie établissement, mon planning et « en ce moment », dossier RH, registre SST et émargement, demande d'HSE, absences à valider, dossiers d'absence à traiter, absences du jour | `app/lib/dashboard-signals.ts` (2 000 l.), `app/api/dashboard/signals/route.ts` | **la meilleure liste terrain** de ce que le personnel veut voir en arrivant → §2.4 |
| NXP-04 | Signaux de vie scolaire : `vsAbsencesATraiter`, `vsAbsencesJustifFamille`, **`vsAppelsManquants`**, `vsSanctionsAujourdhui`, `vsCarnetNonSignees` | même route | à reprendre tels quels comme natures ou tuiles |
| NXP-05 | `facturesEnRetard`, `unseenAccompagnementAlerts`, `unseenSharedFolders`, `travelsUnreadMessages`, `moodPulseSubmittedToday`, `sstEmargementPending`, `stagesPendingSignatures`, `requestsBoard`, `photocopies`, `hse`, `reservations`/`rooms` | même route | chacun relève d'un socle propriétaire (§2.4) |
| NXP-06 | État des sources (`sourceStatus` : chaque source « ok » ou en erreur) et « Réessayer » | `HomeSignalBento.tsx` | **à garder** : une tuile en panne ne fait pas tomber la page (R-PO12) |
| NXP-07 | Notifications globales du tableau de bord, pastille de compteur | `DashboardGlobalNotifications.tsx`, `NotificationCountBadge.tsx` | → centre de notifications du socle 8 |
| NXP-08 | Actualités du jour | `DashboardTodayNews.tsx` | → socle 17 (annonces à la une) |
| NXP-09 | Météo | `DashboardWeather.tsx` | **écartée** (MATRICE DEM-08) |
| NXP-10 | Liens rapides externes paramétrables | `app/api/dashboard/links/route.ts`, `DashboardQuickLinksPanel.tsx` | → §13.3 liens de l'établissement |
| NXP-11 | Tableau de semaine importé (« week sheet ») | `app/api/dashboard/week-sheet/*` | perdable ; besoin couvert par l'agenda (19) et l'EDT (10) |
| NXP-12 | Accueil famille : tuiles Emploi du temps, Notes, Cahier, Bulletins, Absences, Carnet, Sanctions, Messages, Finances, sélection de l'enfant, foyers | `app/components/famille/FamilleHomeClient.tsx` | ce sont des **menus en tuiles**, pas un à-faire ; la sélection d'enfant est à garder |
| NXP-13 | Espace « bien-être » de l'élève (bulle d'écoute) | `dashboard/page.tsx` | → socle 36 ; l'accueil de l'élève doit offrir une porte discrète (§9.3) |
| NXP-14 | Recherche d'élève globale | `app/components/shell/GlobalEleveSearch.tsx` | **défaut** : charge `/api/eleves/dossiers/list?status=inscrit` (tous les élèves inscrits) dans le navigateur et classe côté client |
| NXP-15 | Notifications du navigateur | `BrowserNotificationsProvider.tsx` | → socle 8 (push web) |
| NXP-16 | Cache des signaux par personne | `valkey-keys.ts` (`dashboardSignals: 60` s, clé établissement + utilisateur) | **défaut** : un changement de droits (lien suspendu, départ) ne vide pas le cache |

### 2.3 Défauts de l'existant (à corriger avant toute extension, lot P0)

| # | Défaut | Preuve [F] | Gravité | Correctif |
|---|---|---|---|---|
| **DP-01** | **La famille et l'élève ne voient jamais les annonces sur l'accueil** : la tuile appelle `/annonces`, que `list_annonces_for_actor` refuse aux rôles Parent et Élève (`ent-db/src/service.rs:13482-13485`) ; la tuile affiche « Annonces non disponibles pour votre profil » (`accueil/mod.rs:245`) | lu | importante (démo de décembre) | route d'accueil composée côté serveur (P1) ; pour la famille, `list_annonces_famille_for_actor` par enfant, dédoublonné |
| **DP-02** | **La famille n'a pas d'emploi du temps sur l'accueil** et reçoit la tuile « Votre espace… Ouvrir l'interface actuelle » (`accueil/mod.rs:70-81`, `droits.rs:133` : `emploi_du_temps` faux pour Parent) | lu | importante | tuile « Aujourd'hui de Léa » (EDT daté du jour, changements) par enfant, selon le lien |
| **DP-03** | **Une requête par enfant** et **téléchargement de toutes les notes** pour compter des absences : `TuileAbsencesFamille` appelle `resume_enfant` pour chaque enfant ; `famille_resume_enfant` renvoie toutes les notes publiées de l'année (`routes.rs:10611-10619`) | lu | moyenne (minimisation, performance) | compteurs calculés par le serveur ; aucune note sur l'accueil sans geste |
| **DP-04** | **Tuiles commandées par le rôle fixe**, pas par les droits D-017 : `trancher_justificatif`, `portail_famille`, `emploi_du_temps` viennent de `Role` (`droits.rs:131-145`) ; un rôle « Accueil » paramétré avec `absences.declarer` n'a aucune tuile ; seule « Protections » lit le moteur | lu | importante (D-017) | tuiles déclarées avec leur action D-017 et leur portée, évaluées côté serveur |
| **DP-05** | **Secrétariat, direction, comptabilité, accueil, infirmerie** renvoyés à `ui.rs` (tuile « Votre espace ») | lu | importante (démo de décembre, F-001) | accueils de ces profils au lot P1 |
| **DP-06** | **Erreur 403 affichée comme une tuile** (« … non disponibles pour votre profil ») au lieu de ne pas proposer la tuile | `accueil/mod.rs:245, 289, 325, 380` | confort | la composition ne renvoie que les tuiles permises |
| **DP-07** | **Le navigateur assemble l'accueil** : 4 à 6 requêtes, plus une par enfant ; pas de rendu serveur des données | lu | moyenne (mobile, lenteur) | `GET /portail/aujourdhui` composé, une requête, budget de temps par source (R-PO12) |
| **DP-08** | **Élève majeur et professeur-parent** sans accès au portail famille (`mes_enfants_for_parent` exige `Role::Parent`, `service.rs:6046-6051` ; un compte n'a qu'un rôle par établissement) | lu | importante, **déjà portée** par le lot P0 du socle 6 (casquettes) et le socle 2 (majorité) | le 29 consomme la casquette de session ; rien à corriger ici, dépendance notée |
| **DP-09** | **Notifications sans lien avec les tâches** : `ent_notification` n'a ni objet ni état ; l'avis « absence à justifier » reste après justification | `routes.rs:5655-5679` | moyenne | P2 (index) et port « tâche close → avis obsolètes » vers le socle 8 |
| **DP-10** | **Aucune recherche globale** ni palette de commandes ; `/eleves/search` existe, réservé au personnel (`routes.rs:1098`), sans compteur anti-énumération | lu | confort (mais condition de vente) | P5, P6 |

Aucun défaut **critique** de fuite n'a été trouvé dans l'accueil lui-même : les routes qu'il lit filtrent correctement (lien, portée). Les fuites relevées par d'autres socles et qui **s'afficheraient** sur l'accueil (fils de messagerie lus par tout personnel, M0a du 16 ; notes avant publication, V0 du 21) sont corrigées par leurs lots ; le 29 les **consomme** et ses tests de séparation les revérifient (§25).

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)

Chaque signal de NXP-03 à NXP-05 devient une **nature de tâche** ou une **tuile** déclarée par son socle. Correspondance (le 29 n'en possède aucune) :

| Signal Next.js | Devient | Socle |
|---|---|---|
| appels manquants | tâche « appel à faire » (professeur, en retard après R-PO22) + tuile « appels non faits » (vie scolaire) | 12 |
| absences à valider / dossiers à traiter / absences du jour | file « justificatifs à trancher » (vie scolaire) + tuile « absents maintenant » | 12 |
| carnets non signés | tâche famille « signer le carnet » ; tuile « carnets en attente » (PP, CPE) | 16 / 13 |
| sanctions du jour | tuile « punitions et retenues du jour » (vie scolaire), jamais visible de la famille sur l'accueil (seulement la convocation, tâche) | 13 |
| factures en retard | tâche payeur « échéance à régler » ; tuile « impayés à relancer » (comptabilité) | 24, 25 |
| conventions de stage à signer | tâche signataire | 39 |
| appel du soir d'internat | tâche « appel du soir » (maître d'internat) | 41 |
| sorties du jour / semaine, créneaux vidés, messages des séjours | tuile « en sortie aujourd'hui » ; tâche organisateur « étape suivante » | 42 |
| demandes, photocopies, HSE | tâches du guichet de demandes | **N1** (Q-PO3) |
| salles et réservations | tuile « ma salle aujourd'hui » ; tâche « réservation à confirmer » | 44 |
| mon planning, « en ce moment » | tuile « maintenant » | 10 |
| dossier RH, SST, émargement | tâche agent « émarger », « pièce à fournir » | 6, 49 |
| alertes d'accompagnement non vues | tâche (référent) | 35 |
| dossiers partagés non vus | tuile « documents partagés avec moi » | 7, 32 |
| pouls / humeur du jour | **non repris par défaut** (S-2905) : mesure du climat social au socle 6 (T-08), jamais sur l'accueil | 6 |
| état des sources, « Réessayer » | R-PO12 | 29 |
| liens rapides | liens de l'établissement (§13.3) | 29 |
| sélection de l'enfant (famille) | sélecteur d'enfant (§11.1) | 29 |

### 2.5 Verdict

Garder les composants (`GrilleBento`, `Tuile`, états), la logique pure testée (`modele.rs`), les routes de lecture correctement filtrées et la recherche d'élève serveur (E6). Remplacer l'assemblage côté navigateur par une **composition serveur** déclarative, l'évaluation par rôle fixe par les **droits D-017**, et ajouter ce qui manque entièrement : l'index des tâches, les accueils des profils administratifs, le portail famille réel, la recherche fédérée, la palette, les modes.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il veut en ouvrant l'accueil | Ce qui lui fait perdre du temps aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Parent (un enfant)** | « Y a-t-il quelque chose à faire ? Cours normal aujourd'hui ? » | dix rubriques à ouvrir pour trouver la circulaire à signer ; pastilles qui ne disparaissent pas | rater une autorisation et que l'enfant ne parte pas en sortie ; être relancé pour ce qu'il a déjà fait | une seule liste, triée par échéance, « 2 gestes et c'est fait » |
| **Parent (fratrie, deux établissements)** | la journée des trois enfants d'un coup d'œil | changer de compte, d'appli, de mot de passe | confondre les enfants | une page par jour, une couleur par enfant, même au lycée et au collège |
| **Parents séparés** | ses tâches à lui, pour son temps de garde | recevoir les relances de l'autre, ou voir ce que l'autre a signé | que l'autre parent sache ce qu'il a fait ou lu ; qu'on lui demande de payer la part de l'autre | la neutralité totale, sa part seulement |
| **Parent sans autorité parentale** (373-2-1 C. civ.) | être informé | être sollicité pour une décision qu'il ne peut pas prendre, ou ne rien recevoir du tout | être exclu de tout | les informations importantes, sans boutons qui ne lui appartiennent pas |
| **Famille d'accueil, ASE, tuteur** | ce qu'ils ont le droit de faire pour cet enfant | voir des tâches qui reviennent aux parents (ou l'inverse) | signer à tort | des tâches qui suivent les droits réels du lien |
| **Élève de collège** | « Qu'ai-je à faire pour demain ? Mon cours de 10 h a-t-il lieu ? » | chercher les devoirs dans trois écrans | oublier un travail à rendre | « pour demain » en haut, ce qui a changé en couleur |
| **Lycéen** | travaux à rendre, évaluations annoncées, changements, orientation | les échéances Parcoursup et de l'établissement mélangées | une date limite ratée | un compte à rebours sobre des échéances |
| **Élève majeur** | ses propres tâches (justifier, signer) | dépendre de ses parents pour une justification | que les parents voient tout sans qu'il le sache | savoir ce qui est transmis à ses parents |
| **Élève interne** | soirée, études, appel du soir, sortie du mercredi | | | sa journée et sa soirée sur la même page |
| **Élève en stage (PFMP)** | son lieu, son tuteur, ses attestations | voir un emploi du temps de cours qui ne le concerne pas | | « Aujourd'hui : stage chez … » |
| **Professeur** | prochain cours, appel, ce qui a changé, ce qu'il a oublié | relances du PP par messagerie, oubli d'appel, notes à saisir découvertes trop tard | être signalé pour un appel oublié ; projeter par erreur des données d'élèves | « tout est à jour » ; l'appel en un geste ; ses deux établissements sur une page |
| **Professeur principal** | sa classe : absentéisme, carnets, appréciations manquantes avant le conseil | tableurs de suivi | découvrir au conseil qu'il manque trois appréciations | une tuile « ma classe » qui dit ce qui manque et à qui |
| **AESH** | les élèves qu'il accompagne aujourd'hui, où, et ce qui change | ne pas être prévenu d'un changement de salle | arriver dans une salle vide | la journée de ses élèves, prévenu en premier |
| **CPE** | absents maintenant, appels non faits, justificatifs, élèves à voir, incidents | téléphoner aux professeurs pour l'appel ; passer d'écran en écran | rater un élève en danger (absence inquiétante) | un tableau « maintenant » et une file unique à traiter |
| **AED / surveillant** | la permanence, les retards, les sorties, les retenues à surveiller | listes papier | laisser sortir un élève qui n'a pas le droit | qui est où, qui peut sortir |
| **Accueil** | visiteurs attendus, parents au téléphone, élèves à remettre, rendez-vous | Excel partagé avec la vie scolaire (D-023) | remettre un enfant à la mauvaise personne | « qui peut venir chercher Léa aujourd'hui » en un geste |
| **Secrétariat (scolarité)** | dossiers à instruire, pièces à valider, changements de coordonnées, certificats, exports | courriers, relances, ressaisie | une échéance nationale ratée (BEE, LSU) | une file triée par échéance, et « fait » qui éteint tout |
| **Comptable / économe / OGEC** | paiements rejetés, encaissements à rapprocher, relances, remboursements | pointage à la main | relancer une famille qui a payé ou qui est protégée | la file financière du jour, sans faux positifs |
| **Direction (chef, adjoint)** | ce qui ne peut pas attendre : décisions, signatures, absences de personnels, incidents, alertes | parapheur papier, mails | l'incident grave appris trop tard ; une décision bloquée (D-028, R-A9) | une page qui hiérarchise, et l'assurance que rien n'est en silence |
| **Infirmier·e** | passages du jour, PAI à renouveler, traitements | | qu'une donnée de santé s'affiche ailleurs | son « Infirmerie — aujourd'hui » (34) depuis l'accueil, et rien de médical ailleurs |
| **Psychologue EN, assistant·e social·e** | rendez-vous du jour, demandes | | qu'un intitulé de rendez-vous révèle un motif | des titres neutres partout |
| **Documentaliste** | prêts en retard, réservations, séances au CDI | | | |
| **Maître d'internat, agent de restauration, agent technique** | appel du soir, effectifs repas, demandes d'intervention | | | ce qui les concerne seulement |
| **Administrateur technique de l'établissement** | comptes à activer, imports en erreur, enrôlement passkey | | donner trop de droits | « santé du paramétrage » |
| **DPO, auditeur** | demandes d'exercice de droits, journal | | un accueil qui en dit trop | une page qui prouve la minimisation |
| **Intervenant extérieur, tuteur de stage** | (plus tard) ses séances, ses attestations | | | accès limité, daté |

---

## 4. L'année et la journée vues de l'accueil [H, à confirmer avec Florian]

**La journée.** 7 h 30 : le professeur ouvre l'appli : « 8 h 4B salle 12 — Faire l'appel » ; un changement : « 10 h 2C annulé (sortie) ». 8 h 10 : la CPE voit « 3 appels non faits », « 11 absents dont 4 non signalés ». 8 h 30 : l'accueil reçoit un appel d'un parent → « Déclarer une absence » depuis la palette. 10 h : bilan du matin (NTF-146). 12 h : la famille reçoit l'avis d'absence non justifiée, ouvre l'accueil, la carte « Justifier l'absence de Léa ce matin » est en tête. 17 h : le professeur voit « 2 évaluations sans notes, arrêt des notes vendredi ». 18 h : résumé du jour des familles (socle 8). 20 h–7 h : plage calme — l'accueil se met à jour, rien ne sonne.

**L'année.**
| Moment | Ce que l'accueil devient (mode, §10) |
|---|---|
| Juin – août | familles : réinscription, fiche de rentrée, pièces, fournitures ; G13 : l'année précédente reste visible jusqu'à la bascule (MATRICE G13) ; personnels : préparation de la rentrée (socle 5) |
| Rentrée (J-7 à J+15) | « Rentrée » : date et heure, classe et PP si publiés (STR-96), autorisations et consentements à donner (18), photo (2), fiche santé (34), parcours de rentrée composé (DCT-159) ; personnels : arrivée (PRS-25), listes, emplois du temps |
| Ordinaire | « Aujourd'hui » |
| Veille et semaine des conseils | professeurs : appréciations manquantes, arrêt des notes ; PP : préparation ; direction : calendrier des conseils |
| Vacances | familles : ce qui reste à faire, rentrée prochaine ; personnels : « reprise le … », rien d'urgent |
| Examens | élèves : convocations, calendrier ; personnels : surveillances (38) |
| Fermeture, grève, intempéries | bandeau et tuile « Aujourd'hui modifié » (socles 5 et 10) |
| Alerte PPMS | page d'alerte (socle 14) |
| Fin d'année, bascule | bulletins, décisions, départs ; bascule d'année sans perte des tâches ouvertes (§8.9) |

---
## 5. Ce qui est réglementaire

Recherche du 2026-11-11 (sous-agent, sources officielles ; pages HTML de Légifrance et d'education.gouv.fr en partie inaccessibles, d'où des [V~] = texte officiel lu à travers un extrait ou un résumé de l'outil de lecture). **Aucune règle ci-dessous n'est déduite au-delà des textes** ; les questions ouvertes sont listées au §5.8.

### 5.1 Cadre de référence du numérique pour l'éducation (arrêté du 7 septembre 2026) — décisif

- **Arrêté du 7 septembre 2026 définissant le cadre de référence du numérique pour l'éducation** (NOR MENN2623554A, JORF n° 222 du 23/09/2026, texte 27), pris pour l'application du **décret n° 2025-1165 du 5 décembre 2025** (section 3 ter de la partie réglementaire du code de l'éducation) [V~ arrêté ; NV décret] — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000054883370. Déjà relevé par le socle 9 pour son calendrier (D-029, `VERIF_09_SOURCES.md`).
- **Champ** (art. 2) : SI, services et outils mis en œuvre dans les **collèges et lycées publics** ; rien sur le privé sous contrat [V~]. ScolIA y est soumis dès qu'il vise le public (D-029).
- **Droit à la déconnexion** (annexe 3 § 4.4 ; annexe 4 n° 30 à 34) [V~] : les ENT et suites de vie scolaire **suspendent par défaut la diffusion des nouvelles informations de 20 h à 7 h en semaine et du vendredi 20 h au lundi 7 h** ; périmètre : messagerie, blogs, discussion, notes, devoirs et **toute information de vie scolaire** ; **la consultation et la saisie restent possibles** ; ce qui est saisi pendant la plage n'est visible des destinataires **qu'à la fin de la plage** ; un **message rappelle le droit à la déconnexion à l'émetteur** ; le chef d'établissement peut **ajuster** (contraintes locales, urgences) ; délai : **10 mois** après publication (vers fin juillet 2027). Confirmé par les réponses ministérielles aux questions écrites n° 11713 et 12413 (JO du 23/06/2026) [V] — https://www.assemblee-nationale.fr/dyn/17/questions/QANR5L17QE11713 ; fondement politique : circulaire du 10 juillet 2025 « Promouvoir un numérique raisonné à l'École » (BO n° 28, NOR MENE2519904C) [NV, page inaccessible].
  **Conséquence pour le 29** : une **tâche** ou une **tuile** nouvelle est une « nouvelle information » ; elle ne devient **visible du destinataire** famille ou élève qu'à la fin de la plage (R-PO31), sauf natures d'urgence déclarées (sécurité, alerte, absence inquiétante U0 du socle 8) et ajustement du chef d'établissement. L'auteur (le professeur qui publie un devoir à 21 h) voit le rappel du droit à la déconnexion et l'heure de visibilité. La plage nationale est déjà le défaut de la messagerie (16 : 20 h–7 h, week-end) et des notifications (8) ; le 29 l'applique à l'**affichage** de l'index. Pour le **privé sous contrat**, non soumis à l'arrêté, la même règle est appliquée **par défaut** (cohérence, D-031), réglable par l'établissement (S-2911).
- **Accessibilité** (annexe 3 § 3, exigence n° 3) [V~] : services accessibles conformément au **RGAA** (WCAG 2.1, EN 301 549 v3.2.1), déclaration, mention de conformité, schéma pluriannuel ≤ 3 ans ; sites, logiciels **et applications mobiles**.
- **Services socles** (art. 3) [V~] : usage obligatoire d'ÉduConnect, Guichet Agents, Scope, GAR et **DNMA** (dispositif national de mesure d'audience) dans le public → la mesure d'audience du portail public passe par le DNMA (R-PO28 complété, lot P13).
- **IA générative** (n° 11) [V~] : seulement si l'accès est « réellement anonymisé et sécurisé à travers le GAR » → à verser au socle 30 et à l'étude juridique de D-032 ; pour le 29 : la formulation du résumé du matin par un modèle de langage reste **désactivée par défaut dans le public** (§17, Q-PO7).

### 5.2 Accessibilité numérique : loi, décret, RGAA

- **Article 47 de la loi n° 2005-102** (version issue de l'ordonnance n° 2023-859, en vigueur depuis le 08/09/2023) [V] — https://www.legifrance.gouv.fr/loda/article_lc/LEGIARTI000048050213 : assujettis (I) : 1° personnes morales de droit public (**les EPLE**) ; 2° personnes privées délégataires d'une mission de service public ou créées pour des besoins d'intérêt général non industriels ni commerciaux et financées, contrôlées ou dirigées majoritairement par des personnes publiques ; 3° personnes privées constituées par les précédentes ; 4° entreprises au-delà d'un chiffre d'affaires de **250 M€** (page DINUM [V~], https://accessibilite.numerique.gouv.fr/obligations/champ-application/). **Applications mobiles incluses** (II) [V]. Obligations (III, IV) : **déclaration d'accessibilité**, **schéma pluriannuel** de 3 ans au plus avec plans annuels, **mention claire de la conformité en page d'accueil**, mécanisme de signalement [V].
- **Décret n° 2019-768** [V~] — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000038811937 : mention en page d'accueil ; « totalement conforme » (100 % des critères), « partiellement conforme » (≥ 50 %), « non conforme » (sinon, ou sans audit valide) ; pour une appli mobile, déclaration publiée chez l'éditeur ou avec les informations de téléchargement et **état de conformité précisé dans l'application** ; mise à jour à chaque refonte, tous les 3 ans et 18 mois après une nouvelle version du référentiel (https://accessibilite.numerique.gouv.fr/obligations/mentions-et-pages-obligatoires/).
- **RGAA 4.1.2** en vigueur ; un **RGAA 5** est en rédaction, publication annoncée fin 2026 [V~, https://accessibilite.numerique.gouv.fr/] ; pour les applis mobiles, la doctrine technique du numérique pour l'éducation (juillet 2024) renvoie au **RAAM** et recommande de peser l'accessibilité à 10 % dans les marchés [V, https://eduscol.education.fr/document/49067/download].
- **Sanctions** : contrôle et sanction par l'**Arcom** depuis l'ordonnance 2023-859 (art. 47-1), plafonds annoncés de 50 000 € (défaut d'accessibilité) et 25 000 € (obligations déclaratives) [NV : article non lu sur Légifrance] ; à vérifier, notamment pour une entreprise du 4°.
- **Qui est assujetti pour ScolIA** : l'éditeur, sous le seuil de 250 M€, **ne l'est pas en propre** par l'article 47 ; il l'est **en pratique** par l'arrêté du 7/09/2026 (public) et par la commande publique ; un **établissement public** l'est (1°) et doit publier la mention pour le service qu'il met à disposition ; le **privé sous contrat** n'est classé par aucun texte lu (Q-PO8, juriste). **Recommandation appliquée** : ScolIA se conforme comme s'il était assujetti (mention en pied de page de l'accueil, état de conformité dans l'appli, déclaration et schéma pluriannuel publiés par l'éditeur), parce que c'est exigé dans le public et que c'est l'argument commercial.

### 5.3 Acte européen sur l'accessibilité (directive (UE) 2019/882)

Services couverts depuis le 28 juin 2025 quand ils sont fournis **aux consommateurs**, dont les **services de communications électroniques** et le **commerce électronique** (« en vue de conclure un contrat de consommation ») ; **microentreprises exemptées** pour les services (moins de 10 personnes et ≤ 2 M€) [V~, https://eur-lex.europa.eu/legal-content/FR/TXT/HTML/?uri=CELEX:32019L0882] ; transposition : loi n° 2023-171, décret n° 2023-931 du 9 octobre 2023 [NV]. **Non tranché par les textes lus** : un ENT vendu à un établissement est-il un service fourni à des consommateurs ? le paiement en ligne de la cantine est-il un contrat de consommation ? (Q-PO8). Sans incidence sur la conception : le RGAA, plus exigeant, est appliqué partout.

### 5.4 Schéma directeur des ENT (SDET) et traitement « ENT »

- **SDET version 2025** (document principal et documents opérationnels ; kit juridique « mise à jour février 2026 ») [V, https://www.education.gouv.fr/sites/default/files/2025-07/kit-juridique-441540.pdf] ; document principal 2025 inaccessible, exigences lues dans le **document principal 2024** [V, https://eduscol.education.gouv.fr/sites/default/files/document/sdetdocument-principalversion2024pdf-106014.pdf] :
  - fonctions transverses « **Personnalisation du portail** », « Administration du portail », aide contextuelle (§13) ;
  - **recherche** : un champ sur chaque page, résultats **filtrés selon le rôle, le profil ou l'identité** (MDR-P-5) (§12) ;
  - **notifications** : courriel, SMS, notification mobile (NOT-P-1) ; **« un résumé des nouveautés sur la page d'accueil de l'ENT » (NOT-P-2)** → tuile « ce qui a changé » (POR-103) ; paramétrage des dates et horaires autorisés pour l'envoi ; horaires de messagerie par profil (MES-P-9) ;
  - **plusieurs profils** par usager, préférences par profil, contexte de connexion **par établissement** (exemple du SDET : enseignante dans X et parent dans Y) (§11) ;
  - services accessibles sur tous les appareils, web et mobile (CMN-P-11) ; mesure d'audience par le DNMA (CMN-P-10).
- **Arrêté du 30 novembre 2006** (traitement « ENT », NOR MENT0602397A, modifié en dernier lieu le 13/10/2017), affiché en vigueur sur Légifrance [V~, https://www.legifrance.gouv.fr/loda/id/JORFTEXT000000427578] : l'ENT est un « **point d'entrée unifié** » (art. 1) ; destinataires selon le **besoin d'en connaître** (art. 4) ; **suppression 3 mois après la perte de vocation au compte** (art. 6). Modèle de mention du kit SDET : base légale **mission d'intérêt public** (RGPD art. 6.1.e), **journaux 12 mois au plus** [V]. → la conservation du journal d'états des tâches (§15.5) est alignée sur **12 mois** (corrige « 1 an » proposé, même durée) et l'index d'un compte supprimé est purgé avec lui.

### 5.5 Traceurs, mesure d'audience, mineurs (CNIL)

- Délibérations **2020-091** (lignes directrices) et **2020-092** (recommandation) du 17/09/2020 ; article 82 de la loi Informatique et libertés [V, cités par le kit SDET]. **Exemption de consentement** pour la mesure d'audience si : finalité strictement limitée à la mesure d'audience pour le compte exclusif de l'éditeur, données statistiques anonymes, ni recoupement ni transmission à des tiers, ni suivi entre sites ; durée de vie de 13 mois et conservation de 25 mois présentées comme recommandations [V~, https://www.cnil.fr/fr/cookies-et-autres-traceurs/regles/cookies-solutions-pour-les-outils-de-mesure-daudience]. → R-PO28 : aucun traceur tiers ; le cookie de session n'est pas un traceur soumis à consentement ; mesures internes agrégées sans identifiant ; dans le public, DNMA.
- **Huit recommandations de la CNIL sur les mineurs en ligne** (1/06/2021), dont « encourager les mineurs à exercer leurs droits » et l'information adaptée par le design [V~, https://www.cnil.fr/fr/la-cnil-publie-8-recommandations-pour-renforcer-la-protection-des-mineurs-en-ligne] → l'accueil de l'élève porte le lien « ce que voit ScolIA de moi » (POR-119) rédigé pour son âge.
- **Article 45 de la loi n° 78-17** : consentement seul **à partir de 15 ans** pour l'offre directe de services de la société de l'information **fondée sur le consentement** ; en dessous, conjointement avec les titulaires de l'autorité [V, https://www.legifrance.gouv.fr/loda/article_lc/LEGIARTI000037823135]. Le portail repose sur la mission d'intérêt public, pas sur le consentement ; l'article ne vaut que pour un éventuel traitement fondé sur le consentement (lecture, non texte).

### 5.6 Information des parents, autorité parentale, parents séparés

- **Code de l'éducation, D. 111-1 à D. 111-5** [V~, https://www.legifrance.gouv.fr/codes/id/LEGISCTA000006182457] : D. 111-3 : parents « **tenus régulièrement informés** » de l'évolution des acquis et du respect des obligations scolaires, l'établissement prend toute mesure pour qu'ils en prennent connaissance ; D. 111-4 : réponse aux demandes d'information et d'entrevue, refus motivé ; **D. 111-5** : le conseil d'administration précise les conditions d'accès des parents **aux espaces numériques de travail** [V~]. → un paramétrage d'accès des parents au portail relève, dans le public, d'une délibération du CA : le 29 ne crée pas de nouveau réglage d'accès, il applique les droits du lien (4).
- **Code civil, art. 371-1** (version du 21/02/2024) : autorité parentale jusqu'à la majorité ou l'émancipation ; les parents **associent l'enfant aux décisions qui le concernent** selon son âge et sa maturité [V, https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000038749626]. → l'accueil de l'élève mineur montre les démarches qui le concernent (sans action de décision).
- **Art. 373-2-1, dernier alinéa** : le parent sans exercice de l'autorité parentale « conserve le droit et le devoir de surveiller l'entretien et l'éducation » et « **doit être informé des choix importants** relatifs à la vie de ce dernier » [V, https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000022469781]. → R-PO8.
- **Circulaire n° 2006-137 du 25/08/2006** (BO n° 31) : « deux interlocuteurs pour un élève », coordonnées des deux parents sur la fiche, courriers aux deux adresses, prévenance rapide de toute difficulté, SMS / messagerie / portail encouragés pour les absences et réunions [V, https://www.education.gouv.fr/bo/BoAnnexes/2006/31/encart31.pdf] ; toujours citée par Service-public (fiche F21287) [NV sur sa validité]. → chaque parent a son propre portail et ses propres tâches ; aucune information n'est « adressée au foyer » d'un parent séparé à la place de l'autre.

### 5.7 Élève majeur ; déconnexion des personnels

- **Majorité** : fin de l'autorité parentale (371-1) [V]. **Aucun texte officiel trouvé** sur le droit de l'élève majeur à s'opposer à l'information de ses parents ; seules des sources secondaires l'évoquent [NV] → **Q-PO2** (juriste ou DAJ) ; **défaut appliqué** : le 29 suit la règle que porte le socle 4 (destinataires à la majorité, ELV-39 du socle 2) et l'affiche à l'élève (tuile « Ce que voient vos parents »).
- **Personnels** : accord-cadre télétravail de la fonction publique du 13/07/2021 (droit à la déconnexion à décliner par le dialogue social) [NV] ; aucun texte de l'éducation nationale ne fixe de plages pour les personnels ; le SDET prévoit des horaires d'envoi et de messagerie par profil [V] → le 29 n'envoie rien aux personnels pendant la plage calme par défaut (avis au 8), sauf natures d'urgence ; l'accueil reste consultable.

### 5.8 Ce qui reste à vérifier [NV] et questions

1. Texte intégral de l'arrêté du 7/09/2026 et de ses annexes, et du décret n° 2025-1165 (lecture juriste recommandée ; Q-PO1).
2. Article 47-1 de la loi 2005-102 (sanctions Arcom) ; statut du privé sous contrat et d'un OGEC au regard de l'article 47 ; champ de l'acte européen (Q-PO8).
3. Opposition de l'élève majeur à l'information des parents (Q-PO2).
4. Document principal du SDET 2025 (inaccessible le 2026-11-11) : vérifier que les exigences 2024 citées sont reprises.


---

## 6. Frontières et arbitrages

### 6.1 Qui possède quoi

| Objet | Propriétaire | Ce que fait le 29 |
|---|---|---|
| Absence, justificatif, appel | 12 | indexe les tâches « justifier », « trancher », « faire l'appel » ; affiche les tuiles |
| Signature, pièce demandée, coffre | 7 | indexe « signer », « déposer une pièce » ; renvoie au coffre |
| Avis, préférences, canaux, relances, résumés, plage calme | **8** | affiche le centre de notifications dans la coquille ; reçoit « tâche close » pour rendre obsolètes les avis liés |
| Conversation, boîte de service, carnet | 16 | compte les fils non lus et la file de sa boîte ; indexe « signer le carnet », « répondre » quand la nature le déclare |
| Annonce, actualité, campagne | 17 | tuile « à la une » ; tâche « accusé de lecture demandé » |
| Formulaire, autorisation, consentement | 18 | indexe « répondre », « autoriser » |
| Rendez-vous, réunion, conseil | 19, 23 | tuile « mes rendez-vous du jour » ; tâche « choisir un créneau » |
| Facture, échéance, paiement | 24, 25 | indexe « régler » pour le **payeur** seulement ; tuile « solde » |
| Séjour, sortie collective | 42 | tuile « en sortie aujourd'hui » ; tâches « autoriser », « payer », « étape suivante » |
| Emploi du temps, séance, changement | 10 | tuiles « maintenant », « ma journée », « changements » |
| Index des tâches, états de lecture personnels, reports, prises en charge | **29** | possède |
| Composition des pages, préférences d'affichage, liens de l'établissement | **29** | possède |
| Recherche globale (requête, plafonds, compteurs), palette | **29** | possède la mécanique ; chaque socle possède son port de recherche |
| Files de travail des socles (inscriptions à instruire, Centre des échanges, file financière, plis, affectations) | leur socle | **compte et renvoie** (tuile « file ») |
| Guichet de demandes internes et familles (T-01) | **N1** à créer (Q-PO3) | consomme ses natures |
| Indicateurs dans le temps, tableaux de bord de pilotage | 47 | renvoie (l'accueil ne montre que « aujourd'hui ») |
| Registre des capacités (IA et palette) | cœur IA (IA_COEUR) | la palette en est un client |
| Contrat de tuile | **kit de branchement (D-026)**, rédigé ici, utilisé par les socles 2 (fiche 360°) et 29 | voir arbitrage A7 |

### 6.2 Arbitrage des contradictions entre conceptions (appliqué par défaut, D-031)

Le recensement des autres conceptions (2026-11-11) relève quatorze points où elles se contredisent ou se taisent sur le 29. Recommandations appliquées :

| # | Contradiction | Arbitrage [R, appliqué par défaut] |
|---|---|---|
| **A1** | Le parcours de rentrée famille est revendiqué par le 3 (INS-38), le 4 (fiche de rentrée « ouvre la tâche »), le 7 (DCT-159 « une démarche par famille ») et le 29 | **Chaque élément reste une tâche de son socle** (fiche → 4, pièces et signatures → 7, autorisations → 18, santé → 34, photo → 2). Le 7 compose la **démarche** (regroupement ordonné, avancement, confirmation N3) et la déclare comme **groupe de tâches** (`regroupement = parcours_rentree`) ; le 29 l'affiche comme une seule carte « Rentrée de Léa : 4 étapes sur 7 » qui ouvre le parcours du 7. L'avancement est **calculé** à partir des tâches, jamais stocké deux fois. |
| **A2** | « Mes démarches » (3), l'espace famille « à faire » (42 : 1545), l'espace famille financier (24/25), les portails « mon enfant » (2) et « ma famille » (4) | Ces écrans **restent** : ce sont les vues détaillées de leur domaine. L'à-faire unifié du 29 est la **seule liste transverse** ; chaque tâche ouvre la vue de son domaine au bon endroit. Un domaine **n'affiche pas sa propre pastille** dans la navigation pour les tâches indexées : la pastille est celle de l'à-faire (R-PO4). |
| **A3** | Le 24 livre `finances.tache_comptable` comme un **avis** du 8 ; le 8 range « Tâches et validations » dans son centre (NTF-152) ; les 5, 6, 7, 9 publient vers le 29 | **Frontière** : un **avis** informe (pas d'état, se lit) ; une **tâche** attend une action (état, destinataire, échéance, se ferme). Un fait qui attend une action **ouvre une tâche** (29) et, si sa nature le prévoit, **un avis qui renvoie à la tâche** (8). La catégorie « Tâches et validations » du centre de notifications **affiche l'à-faire** (même source), elle ne le duplique pas. Les événements de tâche sont renommés au format commun `<socle>.tache_ouverte` / `.tache_close` avec la nature en champ (§20) ; `facturation.tache_payeur` et `facturation.tache_comptable` deviennent des **natures** (`fac.payeur_manquant`, `fac.changement_regime`). |
| **A4** | Files de travail propres à chaque socle (3, 5, 8, 9, 16, 24, 34, 42) : le 29 les agrège-t-il ? | **Non.** Une file collective reste chez son socle (tri, colonnes, actions groupées propres au métier). Elle déclare une **tuile de file** (compte, plus ancienne échéance, lien) ; le 29 l'affiche aux membres de la file. Seules les tâches **nominatives** (à une personne) et les tâches collectives **prises en charge** par la personne entrent dans **son** à-faire. Le registre des files (identifiant, propriétaire, droit d'accès) sert aussi au départ guidé du 6 (PRS-30 : « tâches en cours par socle avec lien vers chaque liste »). |
| **A5** | Guichet de demandes (T-01, DEM-03) : N1, « à défaut 29 + 31 », ou 16 + 29 (MATRICE : 436) | **Créer le socle N1** « Demandes et flux de travail » (recommandation déjà écrite en `SOCLES.md` §10, Q-H) ; le 29 n'en absorbe pas le routage. En attendant N1, aucune demande libre dans l'à-faire (Q-PO3). |
| **A6** | Tableaux de bord dispersés (rentrée 5, campagne 3, documents 7, échanges 9, finances 24, sorties 42, effectifs PIL-01) et « signaux » COM-22 | L'accueil n'est pas un tableau de bord de pilotage. Chaque tableau de bord reste chez son socle ; il peut déclarer **une tuile de synthèse** (une information, une action). Les indicateurs dans le temps vont au 47. COM-22 est couvert par les natures du §2.4. |
| **A7** | Deux contrats de tuile possibles (02 E8 pour la fiche 360°, 29 pour l'accueil) | **Un seul contrat de tuile**, dans le kit de branchement (D-026), écrit au §9.1 ; le socle 2 l'utilise pour la fiche (contexte = un élève), le 29 pour l'accueil (contexte = une personne et sa casquette). Les tuiles « qualité des données » (4, 8 : NTF-164, CN-31) en sont des déclarations. |
| **A8** | « Aujourd'hui » du 10 (EDT-106), « Infirmerie — aujourd'hui » du 34, lecture de `sante::localisation` et `sorties::en_sortie` par le 29 ; PLT-10 « aucune santé indexée » ; Q-S29 du 34 (données de santé indirectes) | Les vues du 10 et du 34 restent leurs écrans ; le 29 les met en tuile. **Le 29 ne lit jamais `sante::localisation`** : il lit un port neutre du 12 **`ou_est(eleve, instant)`** qui rend `en_cours | absent | hors_cours_autorise | en_sortie | non_renseigne`, la mention « à l'infirmerie » n'étant rendue **qu'aux** porteurs de `sante.localisation.lire` (vie scolaire, accueil habilités). Le 29 s'inscrit au registre des données de santé indirectes du 34 (Q-S29) pour cette seule mention. |
| **A9** | Arrivée d'un personnel : page `GET /personnels/moi/arrivee` (6, PRS-25) **et** `personnel.arrive` → à-faire (29) | La page d'arrivée reste au 6 ; ses étapes non faites sont des **tâches** de l'arrivant (nature `prs.etape_arrivee`) ; l'accueil montre la carte « Votre arrivée : 3 étapes » pendant la période d'arrivée. |
| **A10** | Multi-établissements : réunion côté appareil (8, 10, 7) contre tableau de bord direction « tous les établissements dirigés » (42 : VOY-135) et « un établissement n'apprend jamais l'existence d'un autre » (4) | **Réunion chez la personne seulement** (appareil, ou navigateur avec une session par établissement), **jamais de jointure serveur entre établissements** (R-PO17). Le chef qui dirige deux établissements a deux sessions ; sa page réunie se fait sur son appareil. VOY-135 est requalifié en ce sens (signalé au 42). |
| **A11** | Une tâche partagée faite par l'autre parent (R-V46 : aucune activité d'un parent visible de l'autre) | La tâche « une réponse par foyer » ou « par l'autorité » disparaît pour tous **sans nom ni date** ; si le parent l'ouvre depuis un ancien lien : « Cette démarche est déjà traitée. » (R-PO7). Une tâche « une réponse par responsable » reste propre à chacun. |
| **A12** | Professeur-parent : jamais d'union de droits (PRS-29) | L'accueil et l'à-faire suivent la **casquette de la session** ; un rappel discret « Vous avez aussi 2 tâches comme parent » (compteur seul, sans contenu) propose de changer de casquette (R-PO16, S-2903). |
| **A13** | Tâches sans destinataire précis (MSG-154 « puis tâche », 42 « à l'organisateur, à la direction ou à la compta », R-A9 sans escalade décrite) | Toute nature **doit** déclarer un destinataire résolu (§8.3) **et** un **filet** : si la résolution donne zéro personne active, la tâche va au **responsable de la file de repli** déclarée par la nature (par défaut : direction), jamais nulle part (R-PO9). Les tâches bloquantes ont une **escalade** datée (§8.5). |
| **A14** | Décès : le 29 retire les tâches de l'élève (R-E12) ; rien n'est dit pour un responsable décédé (ADM-76, CF-10) | Sur `eleve.decede` **et** sur `personne.decedee` (4), le 29 met **sans objet** toutes les tâches dont l'objet ou le destinataire est cette personne, **sans aucun message**, et en informe chaque propriétaire (port `taches_mises_sans_objet`) pour qu'il arrête ses relances (R-PO10). |

### 6.3 Ports consommés [R]

| Port | Socle | Usage |
|---|---|---|
| `casquette_session() → {personne, compte, profil, etablissement, droits_effectifs}` | 1, 6 | toute la composition |
| `liens_de(personne) → [eleve, nature_lien, droits du lien, foyer, autorité, payeur, dates]` | 4 | familles : enfants, droits par enfant |
| `scolarite_du_jour(eleve, date)` | 2, 5 | classe, régime, statut (stage, externe, interne), majorité |
| `journee(personne\|eleve, date)` et `changements_depuis(personne, instant)` | 10 | tuiles « maintenant », « ma journée », « changements » |
| `ou_est(eleve, instant)` (neutre, A8) | 12 | accueil, vie scolaire |
| `verifier_taches(acteur, [tache_id]) → [actionnable \| masquee(motif_neutre) \| close]` | **chaque socle qui déclare des natures** | revalidation à l'affichage (R-PO3) |
| `taches_ouvertes(nature, depuis)` | chaque socle | reconstruction et rapprochement (§8.10) |
| `tuiles_declarees(profil)` et `rendre_tuile(code, acteur, contexte)` | chaque socle | composition de l'accueil |
| `rechercher(acteur, texte, limite)` | 2, 4, 5, 6, 7, 16, 19, 42… | recherche fédérée (§12) |
| `capacites(acteur)` | cœur IA | palette |
| `bandeaux_actifs(acteur)` | 5, 8, 10, 14, 16, 21 | bandeaux (§10) |
| `mode_du_moment(etablissement, date)` | 5 (calendrier, rentrée), 14 (alerte), 23 (conseils) | modes |
| `centre_notifications(acteur)` | 8 | panneau de notifications de la coquille |
| `delegations_actives(personne, date)` | 6 | réattribution pendant une absence |

### 6.4 Ports exposés par le socle 29 [R]

| Port | Pour qui | Contenu |
|---|---|---|
| `ouvrir_tache(nature, objet, destinataires, echeance, titre_neutre, …)` | tous les socles, **dans leur transaction** (via outbox ou appel transactionnel du kit) | idempotent sur `(nature, objet, destinataire)` |
| `fermer_tache(nature, objet, [destinataire], issue)` | propriétaire | `faite`, `sans_objet`, `expiree`, `annulee` |
| `etat_taches(objet)` | propriétaire, 8 | savoir si une relance a encore un sens (le 8 interroge avant d'envoyer une relance, R-PO6) |
| `a_faire_de(acteur)` | 11 (mobile), 30 (IA), 8 (résumé du jour) | même liste, mêmes droits |
| `compteur_a_faire(acteur)` | coquille, 11 (pastille d'application) | entier seul |
| `prises_en_charge(file)` | socles propriétaires de files | qui traite quoi |
| `taches_ouvertes_de(personne)` | 6 (départ guidé), 27 (droit d'accès) | liste typée, minimisée |

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]

| Terme | Définition |
|---|---|
| **Nature de tâche** | type déclaré par un socle dans son kit : code stable (`vs.justifier_absence`), propriétaire, règle de destinataires, cardinalité (une par responsable, une par foyer, une par autorité, une par élève, collective), échéance par défaut, priorité, action de clôture, sensibilité, regroupement, file de repli, escalade, ouverte à l'IA ou non |
| **Tâche** | une occurrence : « Mme Martin doit justifier l'absence de Léa du 12/11 matin, avant le 15/11 ». Identité `(nature, objet_id, destinataire)` |
| **Destinataire** | une **personne** (D-020 : avec ou sans compte), un **foyer** (pour « une par foyer »), ou une **file** (collective) |
| **File** | ensemble de tâches collectives d'un service (« justificatifs à trancher » de la vie scolaire) ; possédée par son socle ; membres = porteurs d'une action D-017 avec portée, ou membres d'une boîte de service (16) |
| **Prise en charge** | une personne de la file prend une tâche (« je m'en occupe ») ; visible des autres membres ; expire |
| **Échéance** | date et heure limites (heure murale du fuseau de l'établissement, R-E10 du 10) ; **dure** (au-delà, la tâche expire ou s'escalade) ou **indicative** |
| **Priorité** | `bloquante` (quelque chose est empêché tant qu'elle n'est pas faite : départ en sortie, conseil de classe, décision de justice) · `due` (échéance proche ou dépassée) · `a_faire` · `information` (à lire, accusé demandé) |
| **Carte d'action** | rendu d'une tâche avec son action en ligne (commande du propriétaire, aperçu, confirmation) |
| **Tuile** | bloc de l'accueil : une information, une action (Bento) ; déclarée par un socle |
| **Tuile de file** | tuile qui compte une file et y renvoie |
| **Bandeau** | message d'état transverse en haut de page (canal SMS suspendu, fermeture, crise) ; déclaré par son socle ; non masquable s'il est de sécurité |
| **Mode** | état de la page selon le moment (ordinaire, rentrée, conseil, vacances, examens, fermeture, alerte) |
| **Casquette** | profil de la session (D-017, PRS-29) : professeur **ou** parent, jamais les deux |
| **Portail** | ensemble des pages d'une casquette : accueil, à-faire, notifications, recherche, raccourcis |
| **Raccourci** | lien vers une action fréquente (NXP-02), pas une tâche |
| **Palette** | fenêtre de commandes (clavier : Ctrl/⌘ + K) ; trouve une page, une personne, une action du catalogue |
| **Rappel personnel** | tâche créée par la personne pour elle-même (§8.12) |

### 7.2 Distinctions qui manquent aujourd'hui

1. **Avis ≠ tâche** (A3). Aujourd'hui `ent_notification` sert aux deux et ne se ferme jamais.
2. **Tâche ≠ raccourci** : « Faire une demande d'HSE » (NXP-02) est un raccourci ; « Votre demande d'HSE est à compléter » est une tâche.
3. **Tâche nominative ≠ file** (A4).
4. **Compter ≠ montrer** : une tuile de file compte sans exposer de nom ; le détail s'ouvre dans l'écran du socle.
5. **Droit d'agir ≠ droit d'être informé** : un parent sans exercice de l'autorité est informé (tuile, avis) mais n'a pas de tâche de décision (R-PO8).

---

## 8. L'à-faire unifié

### 8.1 Le modèle : un index revérifié, jamais une seconde vérité [R]

```
Socle propriétaire                      Socle 29
─────────────────                       ────────
fait métier (transaction) ──outbox──▶  ouvrir_tache / fermer_tache  ──▶ ent_afaire (+ destinataires)
                                                                         │
écran ou API du propriétaire ◀── action (carte d'action, lien profond) ─┤
                                                                         ▼
verifier_taches(acteur, ids) ◀───────── lecture de l'à-faire ── filtre « actionnable maintenant »
taches_ouvertes(nature) ◀────────────── rapprochement nocturne (§8.10)
```

Trois garanties :
1. **Pas de tâche fantôme** : une tâche n'est affichée que si son propriétaire la dit encore actionnable par cette personne à cet instant (R-PO3). Le coût est borné : une vérification **par lot et par socle**, pas par tâche, avec un budget de temps (R-PO12).
2. **Pas de tâche perdue** : l'ouverture est écrite **dans la transaction du fait** (outbox, D-028 Q-N1) ; le rapprochement nocturne compare l'index à `taches_ouvertes` de chaque propriétaire et répare (ajoute, ferme), avec un rapport (POR-205).
3. **Pas de contenu sensible dans l'index** : l'index ne garde qu'un **titre neutre** composé par le propriétaire selon un gabarit déclaré (« Absence à justifier — Léa, 12/11 matin »), sans motif, sans montant pour un non-payeur, sans libellé de santé, de discipline ou de suivi social ; le détail est rendu par le propriétaire au moment de l'ouverture (R-PO5).

Pourquoi un index plutôt qu'une agrégation à la volée (l'approche Next.js) : à la volée, chaque ouverture d'accueil interroge vingt sources (lent sur mobile, NXP-03 a dû ajouter un cache, NXP-16) ; l'index permet un compteur exact et instantané, des échéances triées, des prises en charge et des reports, et le **rapprochement** rattrape toute dérive. Le coût est l'obligation, pour chaque socle, de publier ses ouvertures et fermetures : c'est une ligne du kit (D-026).

### 8.2 Déclaration d'une nature (kit de branchement) [R]

| Champ | Obligatoire | Exemple (`vs.justifier_absence`, socle 12) |
|---|---|---|
| Code stable | oui | `vs.justifier_absence` |
| Socle propriétaire | oui | 12 |
| Gabarit de titre neutre | oui (variables typées, aucune variable libre) | « Absence à justifier — {prenom}, {date} {demi_journee} » |
| Destinataires | oui (§8.3) | responsables du lien avec `peut_justifier_absence`, ou l'élève s'il est majeur |
| Cardinalité | oui | une réponse **par élève** (le premier qui justifie ferme pour tous) |
| Échéance | oui | J+3 ouvrés, indicative (paramètre du 12) |
| Priorité de départ, règle de montée | oui | `a_faire` ; `due` à J+2 |
| Action de clôture | oui : **carte d'action** (commande du propriétaire, avec fiche commande IA_COEUR §5.2) **ou** lien profond | carte : motif dans la liste + pièce facultative, aperçu, confirmation N1 |
| Sensibilité | oui (C0 à C3, IA_COEUR §7.2) | C1 |
| Regroupement | facultatif | par élève ; « 3 absences à justifier » si plusieurs |
| Visible sur l'écran verrouillé / la notification système | oui / non | non (le titre reste dans l'appli) |
| File de repli | oui (A13) | vie scolaire |
| Escalade | si bloquante | — |
| Ouverte à l'IA | oui / non | oui (lecture) ; commande : oui, N1 |
| Fermeture automatique | oui | à la justification, à la décision de la vie scolaire, à la régularisation, à la sortie de l'élève |
| Catégorie de conservation | oui (27) | index : 30 jours après clôture (§22.4) |

Un test du kit refuse une nature sans gabarit neutre, sans destinataire, sans file de repli ou sans action de clôture (POR-243).

### 8.3 Destinataires et revérification [R]

Règles de destinataires déclarables (le 29 ne les invente pas, il les fait résoudre par le propriétaire ou par les ports du cœur) :
- **personne désignée** (le professeur de la séance, l'organisateur du séjour, le signataire) ;
- **responsables d'un élève** selon un **droit du lien** (socle 4 : `peut_justifier_absence`, `peut_autoriser_sorties`, `peut_voir_notes`…), avec cardinalité par responsable, par foyer ou par autorité ; l'élève majeur selon les règles du socle 2 (majorité) et du 4 ;
- **payeur** (24) d'une échéance, avec sa part seulement ;
- **porteurs d'une action D-017 avec portée** (« les personnes qui ont `absences.trancher` sur la classe de l'élève ») → **file** ;
- **membres d'une boîte de service** (16) → file ;
- **direction** (file de repli par défaut).

La résolution se fait **à l'ouverture** (pour savoir où indexer) **et** à l'affichage (`verifier_taches`, R-PO3). Si un nouveau responsable acquiert le droit après l'ouverture (jugement, majorité, changement de payeur), le propriétaire **ajoute** le destinataire (`ouvrir_tache` idempotent) ; le rapprochement le fait au plus tard la nuit suivante.

### 8.4 États [R]

| État | Signification | Visible de | Transitions |
|---|---|---|---|
| `ouverte` | attend une action | destinataires | → `prise`, `faite`, `sans_objet`, `expiree`, `annulee` |
| `prise` | collective, prise en charge par X (depuis, jusqu'à) | membres de la file (avec le nom de X) | → `ouverte` (relâchée, expirée après S-2901), `faite`… |
| `en_attente` | le destinataire a agi, le propriétaire attend autre chose (pièce en analyse, signature du second parent quand l'autorité conjointe l'exige) | destinataire (« en cours de vérification ») | → `faite`, `ouverte` (refus : avec motif du propriétaire) |
| `faite` | close par l'action | — (historique 30 j pour le destinataire) | finale |
| `sans_objet` | devenue inutile (élève parti, décès, objet annulé, droit perdu) | — | finale |
| `expiree` | échéance dure passée sans action ; le propriétaire décide la suite (escalade, relance humaine) | — | finale |
| `annulee` | annulée par le propriétaire (erreur) | — | finale |

États **personnels** (par destinataire, jamais visibles des autres, R-PO7) : `vue_le`, `reportee_jusqu_au` (« me le rappeler lundi » : interdit pour `bloquante` et pour une échéance dure à moins de 24 h), `masquee` (pour `information` seulement).

### 8.5 Priorité, échéances, escalade [R]

- **Tri** de l'à-faire : bloquantes, puis échéance la plus proche, puis date d'ouverture ; à priorité égale, l'enfant le plus jeune d'abord **non** : l'ordre alphabétique des prénoms (stable, sans hiérarchie entre enfants).
- **Montée** : une tâche passe `due` à J-1 de l'échéance (ou selon la nature).
- **Escalade** (bloquantes) : la nature déclare un palier (délai, nouvelle file) : ex. R-A9 du 4 (interdiction provisoire sans pièce à J+7 → direction), MSG-154 (fil sans réponse à J+2 → responsable de la boîte), NTF-63 (avis U2 non remis à 10 h → « à prévenir autrement »). L'escalade **ajoute** un destinataire, elle n'en retire pas.
- **Jamais de silence** (D-028) : une tâche qui ne trouve aucun destinataire actif va à la file de repli (A13) ; une tâche bloquante expirée crée une tâche à la direction (« décision en attente depuis 7 jours »).
- **Heures** : heure murale du fuseau de l'établissement ; échéance « vendredi » = vendredi 23 h 59 heure locale.

### 8.6 Regroupement [R]

- Par **enfant** (famille), par **nature** (« 12 justificatifs à trancher »), par **objet** (« Sortie au Louvre : autoriser et payer »), par **démarche** (A1 : parcours de rentrée).
- Un groupe affiche le nombre, l'échéance la plus proche et une action « tout traiter » quand le propriétaire déclare une **action groupée** (signer plusieurs documents en un geste, DCT-141 ; justifier trois demi-journées d'un même motif).
- Le regroupement est un calcul d'affichage ; l'index garde des tâches unitaires.

### 8.7 Agir depuis l'à-faire [R]

1. **Carte d'action** : la commande du propriétaire est appelée avec son **aperçu** (« Vous justifiez l'absence de Léa le 12/11 matin : motif « Rendez-vous médical » ; la vie scolaire décidera. »), la **confirmation** du niveau déclaré, la **re-vérification des droits** au moment de l'exécution, l'**idempotence** (identifiant de tâche). Mêmes fiches commande que l'IA (IA_COEUR §5.2) : une capacité bien déclarée sert les deux.
2. **Lien profond** : ouvre l'écran du propriétaire au bon objet ; le retour revient à l'à-faire avec la tâche fermée si l'action a abouti.
3. **Action groupée** déclarée par le propriétaire (A4, §8.6).
4. **Jamais d'action sans aperçu pour l'argent, la signature ou une décision** (N2 au moins).
5. **Hors ligne** (mobile) : l'à-faire est lisible ; une action n'est jamais rejouée en silence au retour du réseau : elle est **proposée** à nouveau avec l'état du serveur (R-PO20).

### 8.8 Plusieurs responsables, autorité, majorité [R]

- **Cardinalité « par foyer »** (formulaire, R-I18) : une tâche par foyer ; le premier responsable du foyer qui agit ferme pour le foyer.
- **« Par autorité »** (autorisation de sortie quand l'acte usuel suffit) : un seul titulaire suffit ; la tâche est indexée chez chaque titulaire et se ferme pour tous, **sans dire qui** (A11, R-PO7).
- **« Chaque titulaire »** (acte non usuel déclaré par le propriétaire, ex. changement d'orientation selon le 37) : une tâche par titulaire ; l'état `en_attente` dit seulement « en attente d'un autre accord », sans nom.
- **Désaccord déclaré** entre titulaires (4, 42 : CV-04) : le propriétaire bloque ; le 29 montre aux deux « en attente de l'établissement », jamais l'autre réponse.
- **Parent sans exercice** de l'autorité : aucune tâche de décision ; informations selon le lien (R-PO8, §5).
- **Élève majeur** : à la majorité (socle 2, ELV-39 J-30), les tâches de décision **basculent** vers l'élève ; les tâches ouvertes au nom des parents sont mises `sans_objet` côté parents et rouvertes côté élève (le propriétaire décide, le rapprochement vérifie) ; l'élève voit sur son accueil **ce qui est transmis à ses parents** (tuile « Ce que voient vos parents », selon le 4).
- **Lien suspendu** (décision de justice) : revérification immédiate (R-PO3) ; aucune trace visible côté parent suspendu (« Aucune démarche en cours »).

### 8.9 Absences, délégations, départs, décès, bascule d'année [R]

- **Délégation** (socle 6, `ent_delegation`) : pendant la délégation, les tâches nominatives du délégant sont **aussi** indexées chez le délégataire pour les natures couvertes par les droits délégués ; marquées « pour M. X ».
- **Absence d'un personnel** (15) : ses tâches dues pendant l'absence sont proposées à la file de repli de chaque nature (« 3 tâches de Mme Y échoient pendant son absence »), jamais réattribuées sans geste.
- **Départ** (6, PRS-30/31) : à J-15, l'agent et la direction voient le compte des tâches ouvertes par socle ; au départ, les tâches nominatives restantes vont à la file de repli.
- **Décès** (A14) : `sans_objet` silencieux ; aucune relance ; aucun compteur.
- **Départ de l'élève** (3) : les tâches dont il est l'objet passent `sans_objet`, sauf celles que le propriétaire garde (solde de facture, restitution de matériel, exeat à signer).
- **Bascule d'année** (5) : une tâche ne meurt pas à la bascule ; elle garde son année d'origine affichée (« 2026-2027 ») tant qu'elle est ouverte.

### 8.10 Rapprochement et réparation [R]

- Chaque nuit (et à la demande d'un administrateur, POR-205) : pour chaque nature, `taches_ouvertes(nature, depuis)` du propriétaire est comparé à l'index ; les manquantes sont ajoutées, les orphelines fermées `sans_objet` ; rapport par nature (compte, jamais de nom dans le journal applicatif, E14).
- Un écart supérieur à un seuil (S-2902) déclenche une alerte technique (socle 28) : le propriétaire publie mal ses faits.
- L'index peut être **entièrement reconstruit** (après restauration, D-037 à D-040) : il est classé **« reconstructible »** dans le registre des tables (E12) et **neutralisé à la restauration** puis rebâti (E9).

### 8.11 Avis et tâches : la règle commune avec le socle 8 [R]

- Une nature de tâche peut déclarer une **nature d'avis associée** (8) : l'avis porte la référence de la tâche ; dans le centre de notifications, il affiche l'état courant de la tâche (« fait » barré) ; le 8 **n'envoie aucune relance** pour une tâche close (il appelle `etat_taches` avant chaque relance, R-PO6).
- Le **résumé du jour** du 8 (NTF-09) liste les tâches ouvertes via `a_faire_de` : une seule source.
- La **pastille** de l'application (iOS, Android) = `compteur_a_faire` (tâches actionnables), **pas** le nombre d'avis non lus (R-PO4) ; les messages non lus ont leur propre pastille dans l'onglet Messages.

### 8.12 Rappels personnels (« ma liste ») [R]

- Une personne peut se créer des rappels (« rendre les copies de 4B », « appeler la famille de Tom ») : texte court (≤ 140 caractères), échéance facultative, objet lié facultatif (une séance, une classe, un élève **dans sa portée**).
- **Jamais visibles** d'un autre, y compris de la direction ; **non exposés à l'IA** sauf usage par la personne elle-même ; **expirés** et effacés à la fin de l'année scolaire (S-2904) ; un avertissement rappelle de ne pas y écrire de donnée sensible.
- Familles et élèves : même fonction (« acheter la calculatrice »).

---

## 9. « Aujourd'hui » par profil

### 9.1 Le contrat de tuile (kit de branchement, A7) [R]

| Champ | Obligatoire | Exemple |
|---|---|---|
| Code stable | oui | `edt.maintenant` |
| Socle propriétaire | oui | 10 |
| Profils et droit D-017 qui l'ouvrent | oui | personnel avec `appel.faire` ; élève ; responsable avec `peut_voir_edt` |
| Contexte | oui | `personne` (accueil) ou `eleve` (fiche 360°, enfant dans le portail famille) |
| Largeur proposée | oui (4, 6, 8, 12) | 8 |
| Une information principale, une action principale | oui | « 8 h 4B salle 12 » ; « Faire l'appel » |
| État vide signifiant | oui | « Plus de cours aujourd'hui » |
| Priorité d'ordre par défaut, par mode | oui | 10 en ordinaire ; masquée en vacances |
| Sensibilité, mode discret | oui | C1 ; en mode discret : sans nom d'élève |
| Coût et budget | oui (une requête, < 80 ms en p95) | |
| Rendu | `rendre_tuile(code, acteur, contexte)` → vue typée `ent-types`, jamais une ligne de table | |
| Rafraîchissement | à l'ouverture ; certaines tuiles « vivantes » (appels non faits) toutes les 60 s quand la page est visible | |

La page est **composée côté serveur** (`GET /portail/aujourdhui`) : le serveur choisit les tuiles permises, les rend chacune avec son budget, et renvoie les vues ; une tuile en erreur ou hors délai est rendue « indisponible, réessayer » sans faire tomber la page (R-PO12, NXP-06). Le rendu Leptos (rendu serveur + hydratation) affiche la page complète dès le premier octet.

### 9.2 Structure commune de la page [R]

1. **En-tête** : date longue, « Bonjour Mme Martin » (prénom d'usage, jamais le nom de naissance si une identité d'usage existe, socle 2), sélecteur de casquette et d'établissement si plusieurs, bouton de recherche (palette).
2. **Bandeaux** actifs (§10).
3. **À faire** (tuile de 12 ou 8 colonnes) : les 5 premières tâches triées, « Tout voir (12) » ; « Rien à faire pour l'instant » sinon.
4. **Maintenant / ensuite** (selon le profil).
5. **Tuiles du profil** (ordre par défaut du modèle de l'établissement, personnalisable dans les limites du §13).
6. **Raccourcis** du profil (4 à 8).
7. Pied : déclaration d'accessibilité, aide, « ce que voit ScolIA de moi » (lien vers le journal des accès du socle 27).

### 9.3 Familles et élèves [R]

**Responsable légal** (contexte : tous ses enfants, ou un enfant choisi) :
| Tuile | Contenu | Socle | Droit du lien |
|---|---|---|---|
| À faire | justifier, signer, autoriser, payer (payeur), répondre, choisir un créneau, réinscrire, compléter la fiche de rentrée, déposer une pièce, accuser réception | tous | selon la nature |
| Aujourd'hui de {enfant} | cours du jour avec changements (annulé, remplacé, salle), « en sortie », « en stage », « pas de cours » ; une ligne par enfant en vue « tous » | 10, 39, 42 | `peut_voir_edt` |
| Pour demain | travail à faire et évaluations annoncées (CDT-08) | 20 | `peut_voir_cahier` |
| Vie scolaire | absences et retards de la semaine, punition à venir (convocation) | 12, 13 | `peut_voir_vs` |
| Dernières notes publiées | 3 dernières, **sur geste** (« Afficher ») : jamais affichées d'emblée (mode discret, R-PO13) | 21 | `peut_voir_notes` |
| Messages | non lus, dernier fil | 16 | — |
| À la une | annonces de l'établissement pour la classe de l'enfant | 17 | — |
| Rendez-vous | prochain rendez-vous, rencontres parents-professeurs ouvertes | 19 | — |
| Compte famille | solde et prochaine échéance — **payeur seulement**, sa part | 24, 25 | payeur |
| Cantine | menu du jour, réservation à faire | 40 | — |
| Sortie à venir | prochaine sortie, ce qui manque | 42 | — |
| Documents récents | bulletin publié, attestation | 7, 23 | selon le document |
| Rentrée | (mode rentrée) parcours de rentrée | 7 (A1) | — |

**Parent sans exercice de l'autorité parentale** : mêmes tuiles d'information selon les droits du lien ; **aucune** tâche de décision ; une mention sobre « Les décisions sont prises par le ou les titulaires de l'autorité parentale. » (R-PO8).

**Élève (collège)** : À faire (travaux à rendre, QCM, documents à lire, convocation), Maintenant / ensuite (cours, salle, changements), Pour demain, Mes notes (sur geste), Messages (fils encadrés adulte–élève, 16), Cantine, Prêts CDI à rendre (45), une porte discrète « Parler à un adulte » (vers le 36 : signalement confidentiel, T-12, NXP-13) **sans compteur**, Raccourcis (agenda, EDT).
**Lycéen** : en plus : échéances d'orientation (37, Parcoursup) ; épreuves et convocations (38) ; travaux à rendre en ligne (20).
**Élève majeur** : en plus : ses tâches de décision (justifier, signer, payer s'il est payeur) ; tuile « Ce que voient vos parents » (4).
**Interne** : soirée (étude, appel du soir, sortie du mercredi) (41).
**En stage** : « Aujourd'hui : stage chez {organisme} », tuteur, attestation à faire signer (39) ; l'EDT de cours est masqué pendant la période.

### 9.4 Enseignants et accompagnants [R]

**Professeur** :
| Tuile | Contenu | Socle |
|---|---|---|
| Maintenant | séance en cours ou prochaine, salle, « Faire l'appel » (existant) ; élèves déjà signalés absents ; **remplacement à assurer** | 10, 12, 15 |
| Ma journée | séances du jour avec état : appel fait / à faire, cahier rempli / vide, travaux à rendre ce jour | 10, 12, 20 |
| À faire | appels en retard, évaluations sans notes avant l'arrêt, appréciations avant le conseil, copies remises à corriger, carnets à viser, convocation de réunion à confirmer, sortie à encadrer (pièce à fournir) | 12, 21, 23, 20, 16, 19, 42 |
| Changements | depuis hier : salle, séance déplacée, élève arrivé ou parti d'un de ses cours, changement de groupe | 10, 2, 5 |
| Messages | non lus | 16 |
| À la une | annonces aux personnels | 17 |
| Réunions et conseils | prochains | 19, 23 |
| Mes demandes | salles, reprographie, HSE (N1) | 44, N1 |
| Élèves à besoins particuliers | **seulement** « 2 élèves de la 4B ont un aménagement — voir » (aucun diagnostic) | 35 |

**Professeur principal** : en plus « Ma classe » : appréciations manquantes par collègue (nombre), élèves au-delà d'un seuil d'absence (nombre, lien), carnets non signés, fiches de dialogue (23), prochaines rencontres.
**AESH** : « Mes élèves aujourd'hui » : élèves accompagnés, séances, salles, changements (10 §8.3 bis) ; aménagements utiles (35) ; aucun accès aux notes sauf droit donné.
**Documentaliste** : séances au CDI, prêts en retard, réservations (45).

### 9.5 Vie scolaire, accueil, internat [R]

**CPE** : « Maintenant » (absents non signalés, appels non faits par séance en cours — relance en un geste, retards), file « justificatifs à trancher » (tuile de file), élèves à voir (convocations, retenues du jour), alertes d'assiduité (seuils, 12), signalements à émettre, incidents du jour (13), sorties anticipées autorisées, boîte « vie scolaire » (16), « familles à prévenir autrement » (NTF-63), bilan du matin à 10 h (NTF-146), « où est … » (recherche, A8).
**AED / surveillant** : permanence et étude en cours, retenues à surveiller, sorties du jour (14), retards à saisir.
**Accueil** : rendez-vous et visiteurs attendus (14, 19), appel d'un parent → « Déclarer une absence » (12, existant), élèves à remettre (sortie anticipée autorisée, avec « qui peut venir chercher » ADM-44, interdictions en tête), portes ouvertes du jour (3), « où est … » neutre.
**Maître d'internat** : appel du soir, effectifs, sorties, retours tardifs (41).

### 9.6 Administration [R]

**Secrétariat (scolarité)** : tuiles de file : dossiers d'inscription à instruire (3), pièces à valider (7), propositions de changement de coordonnées (4), coordonnées en rebond (ADM-06), rejets nationaux (9 : « INE refusé par BEE »), départs et exeat, demandes de certificats ; échéances nationales du mois (9) ; boîte « secrétariat » (16).
**Comptabilité** : prélèvements rejetés, encaissements à rapprocher, relances à valider, remboursements, payeurs manquants (24 : FAC-210), écarts de bourse (25) ; tuile de la file financière (24).
**Direction** : « Ne peut pas attendre » (tâches bloquantes, escalades, D-028, R-A9), parapheur (documents à signer en tant que chef : conventions, circuits de sortie, décisions), absences de personnels et cours non assurés aujourd'hui (15, 10), incidents graves (13), protections à confirmer (existant), alertes (14), indicateurs du jour (effectif présent, nombre d'absents — chiffres seuls), calendrier (conseils, instances).
**Adjoint** : même base, plus « demain » (remplacements, salles, EDT, 10 §14.3).
**Administrateur de l'établissement** : comptes à activer, enrôlement passkey (taux), imports en erreur (9), rôles modifiés récemment (1), santé du paramétrage (28).

### 9.7 Santé, social, psychologie [R]

**Infirmerie** : tuile « Infirmerie — aujourd'hui » (34, écran propriétaire), PAI à renouveler (nombre), boîte « infirmerie » ; **rien de médical sur l'accueil d'un autre profil** (R-PO14).
**Psychologue EN, assistant·e social·e** : rendez-vous du jour avec **titres neutres** (« Rendez-vous 10 h »), demandes en attente (nombre) ; ces tuiles sont exclues de la recherche, de l'IA et du mode « projection ».

### 9.8 Autres personnels [R]

Agent de restauration (effectifs du jour, 40), agent technique (demandes d'intervention, 44), DPO (demandes d'exercice de droits à traiter, violations, 27 ; aucune donnée d'élève), intervenant extérieur (plus tard : ses séances).

---

## 10. Modes et bandeaux

### 10.1 Modes [R]

| Mode | Déclencheur (fait d'un socle) | Ce qui change |
|---|---|---|
| `ordinaire` | défaut | §9 |
| `rentree` | J-15 à J+15 de la rentrée (calendrier du 5) ; `referentiel.rentree_publiee` | familles : carte « Rentrée » en tête (A1, STR-96 : date, heure, classe et PP **si publiés**) ; personnels : listes, EDT, arrivée (A9) |
| `veille_conseil` | période des conseils (23) | professeurs : appréciations et arrêt des notes en tête ; PP : « Ma classe » |
| `vacances` | vacances de la zone (5) | familles : « Reprise le … », tâches non urgentes ; personnels : aucune tuile « maintenant » ; aucune tâche ne monte `due` pendant les vacances sauf échéance dure (S-2906) |
| `examens` | session d'examen (38) | élèves : convocations ; personnels : surveillances |
| `journee_modifiee` | fermeture, grève, intempéries (5 STR-94, 10) | bandeau + tuile « Aujourd'hui modifié » : quels cours n'ont pas lieu, cantine, études ; famille : par enfant |
| `alerte` | alerte PPMS déclenchée (14) | **la page entière** devient la page d'alerte du 14 : consigne, « les élèves dont vous êtes responsable maintenant » (liste de présence en temps réel), bouton « Je suis en sécurité avec … » ; aucune autre tuile ; non masquable ; familles : consigne officielle seulement (aucun détail opérationnel) |
| `bascule` | bascule d'année en cours (5) | familles : l'année précédente reste consultable jusqu'à la bascule (MATRICE G13) |
| `cohabitation` | an 1 du pilote, notes dans Charlemagne (D-032) | familles : tuile « Notes et bulletins : consultez … » (EVA-237) ; bulletins distribués par ScolIA (V14 du 21) |

Le mode est calculé par le serveur à partir des faits ; aucun administrateur ne « bascule » l'accueil à la main (D-023), sauf la clôture d'une alerte, qui appartient au 14.

### 10.2 Bandeaux [R]

Déclarés par leur socle, avec : texte, gravité (`info`, `attention`, `securite`), public (droit D-017 ou profil), durée, masquable ou non. Exemples : canal SMS suspendu (« 23 familles non prévenues », CN-21, 8) ; restriction de guichet (21 : EVA-244, sans motif) ; urgence de messagerie (16 : R-M26) ; fermeture (5) ; reconnexion requise (1) ; « vous êtes en mode remplacement de M. X » (6). Au plus **deux** bandeaux visibles, les autres repliés ; un bandeau `securite` n'est jamais masquable (R-PO15).

---

## 11. Plusieurs enfants, plusieurs casquettes, plusieurs établissements

### 11.1 Plusieurs enfants [R]

- Vue **« Tous mes enfants »** par défaut (une ligne par enfant dans chaque tuile, une pastille de couleur et le prénom) ; sélecteur pour n'en voir qu'un ; le choix est mémorisé sur l'appareil.
- **Les droits sont évalués enfant par enfant** : un droit sur Léa n'ouvre rien pour Tom (R-PO2). Une tuile n'affiche que les enfants pour lesquels le lien donne le droit ; si aucun, elle n'apparaît pas.
- **Enfant parti** de l'établissement : visible en « anciens élèves » jusqu'à la clôture des tâches qui le concernent (solde, exeat), puis plus du tout.
- **Enfants de plusieurs foyers** (famille recomposée) : chacun suit ses propres liens ; jamais d'information d'un enfant non lié (beau-parent sans lien, R-PO2).

### 11.2 Plusieurs casquettes (A12) [R]

- Une session = une casquette (PRS-29). Le changement de casquette est un geste explicite (« Passer en espace Parent »), qui recharge la page ; il est journalisé (socle 1).
- La casquette inactive n'expose qu'un **compteur** (« 2 choses à faire comme parent »), jamais de contenu, et seulement si la personne l'accepte (S-2903, activé par défaut).
- Un professeur qui est aussi parent d'un élève de sa classe : les deux casquettes restent séparées ; la casquette professeur suit les règles du conflit d'intérêts du 4 (`270_conflit_interet`).

### 11.3 Plusieurs établissements (A10) [R]

- **Mobile** (socle 11) : l'appli tient une session par établissement (D-011) ; elle appelle `a_faire_de` et `aujourdhui` de chaque établissement **séparément** et **réunit sur l'appareil** : une page « Aujourd'hui » avec une étiquette d'établissement par ligne, l'à-faire fusionné et trié par échéance. Aucun serveur ne voit la réunion.
- **Web** : un **sélecteur d'établissement** (liste des adhésions de l'identité, D-013, sans aucune donnée métier) ; lot P9 : des **compteurs seuls** par établissement, récupérés **par le navigateur** auprès de chaque sous-domaine où la personne a une session ouverte (`GET /portail/compteur` : un entier, CORS limité aux sous-domaines `*.scolia.fr`, cookie de session du sous-domaine concerné, aucun nom). Aucun établissement n'apprend l'existence de l'autre (ADM : 632).
- **Professeur partagé entre deux établissements** (complément de service) : sur mobile, sa journée réunie, triée par heure, avec le trajet visible entre deux séances (« 11 h 05 fin au collège, 11 h 30 début au lycée ») ; un **conflit horaire** entre deux établissements est signalé **sur l'appareil seulement** (aucun des deux établissements ne le voit ; la personne le signale elle-même si elle le souhaite).

---

## 12. Recherche globale et palette de commandes

### 12.1 Recherche globale [R]

- **Qui** : le personnel (PLT-10). Familles : recherche **dans leurs propres données** (enfant, documents, messages) et dans les pages et actions. Élèves : pages, actions, leurs travaux et documents ; **jamais un autre élève**.
- **Quoi** (sources fédérées, chacune par son port `rechercher`, sous les droits du lecteur) : élèves (2, port E6 existant, portée du lecteur), personnes (4 : responsables, contacts — pour le personnel qui a le droit), personnels (6 : annuaire, `identite_restreinte` respectée), classes, groupes, cours, salles (5 : STR-90), documents (7, métadonnées seulement), conversations (16, sujets des fils où la personne participe), rendez-vous (19), séjours (42), pages et actions (navigation et catalogue).
- **Jamais indexé ni recherchable** : données de santé (PLT-10), motifs d'absence, contenus de discipline et de suivi social ou psychologique, notes et appréciations, coordonnées complètes, IBAN, pièces d'identité.
- **Règles** : au moins **2 caractères** ; insensible aux accents et à la casse (E6 : `unaccent`, `pg_trgm`) ; **10 résultats par source**, 40 au total ; aucune pagination profonde (on affine) ; résultat = libellé minimal (« DUPONT Léa — 4B ») et une action (ouvrir la fiche, qui journalise la consultation au socle 2) ; élèves à **confidentialité renforcée** (02 §7.6 `ent_eleve_confidentialite`) et personnes à **adresse protégée** : n'apparaissent qu'aux porteurs du droit, sinon ni résultat ni indice (pas de « 1 résultat masqué ») ; **identité d'usage** affichée, recherche aussi sur le nom de naissance seulement pour les habilités (2).
- **Anti-énumération** : compteur de recherches par personne et par jour (S-2907 : 400 par défaut pour le personnel) ; au-delà, ralentissement puis blocage temporaire, et tâche de sécurité à l'administrateur ; motif détectable « recherche systématique de lettres » (a, b, c…) → même traitement. Le **texte des recherches n'est jamais conservé** (seulement le compteur journalier, R-PO19).
- **Côté serveur uniquement** : aucune liste d'élèves envoyée au navigateur pour classement local (contraire de NXP-14).

### 12.2 Palette de commandes [R]

- Ouverture : Ctrl / ⌘ + K, ou le bouton de recherche ; sur mobile, le champ de recherche de l'accueil.
- Trois familles de résultats : **Aller à** (pages de la navigation permises), **Trouver** (recherche du §12.1), **Faire** (actions du **registre des capacités** permises, IA_COEUR §5 : « Déclarer une absence », « Faire l'appel de la 4B », « Écrire à la vie scolaire », « Réserver une salle »).
- Une action « Faire » suit **exactement** le cycle de l'IA : paramètres par identifiants connus (choisis dans des listes, jamais un nom libre interprété), **aperçu** rédigé par le socle, **confirmation** du niveau déclaré, re-vérification des droits, idempotence. La palette ne contient **aucune logique métier** ; c'est un client du registre, comme l'assistant (IA_COEUR : 242).
- **Langage naturel** : non dans la palette (c'est l'assistant, socle 30) ; la palette accepte des mots-clés et des alias (« abs » → « Déclarer une absence »).
- Raccourcis clavier documentés, accessibles (motif ARIA « combobox » avec liste, annonce du nombre de résultats), utilisables sans souris.

---

## 13. Personnalisation (peu, et utile, D-023)

### 13.1 Ce que la personne peut faire [R]

- **Réordonner** les tuiles, **masquer** une tuile non essentielle, **épingler** une tuile en tête ; **rétablir** la page par défaut. La tuile « À faire » et les bandeaux de sécurité ne se masquent pas.
- Choisir la vue « tous mes enfants » ou « un enfant » (famille) ; le **mode discret** (§14.5).
- Rien d'autre : pas de constructeur de tableau de bord, pas de widgets libres.

### 13.2 Ce que l'établissement peut faire [R]

- Choisir l'**ordre par défaut** des tuiles par modèle de rôle (D-017), parmi les tuiles déclarées ; ajouter des **raccourcis** (NXP-02) au modèle d'un rôle.
- Ne peut pas : imposer une tuile qui n'est pas déclarée, ni montrer une tuile à un profil qui n'a pas le droit sous-jacent (le droit prime toujours).

### 13.3 Liens de l'établissement (NXP-10) [R]

- Liste courte (≤ 12) de liens externes (Parcoursup, ÉduConnect, site de l'établissement, cantine du prestataire, transport) : libellé, adresse **https** seulement, public (profils), ordre ; affichés dans une tuile « Liens utiles » ; ouverture dans un nouvel onglet avec avertissement « vous quittez ScolIA » ; aucun suivi des clics.
- Gestion par `portail.liens.gerer` (direction, administrateur) ; journalisée.

---

## 14. Accessibilité, langues, mobile, performance

### 14.1 Accessibilité [R, voir §5]

- Cible : **RGAA** en vigueur (§5), niveau AA des WCAG ; l'accueil est la page **la plus auditée** (page d'entrée de chaque profil). Exigences de conception : structure en régions (`header`, `nav`, `main`, `aside`), un titre `h1` par page, **un titre `h2` par tuile**, lien d'évitement, ordre de lecture = ordre visuel, focus visible, cibles ≥ 44 px, contraste, **aucune information par la seule couleur** (états toujours avec libellé, `FRONTEND_CIBLE.md` §2), zoom 200 % et **refonte à 320 px** sans défilement horizontal, mouvement réduit respecté, compteurs annoncés par une région `aria-live` **polie** et seulement au changement (pas à chaque rafraîchissement), palette au motif combobox.
- **Parcours famille accessible** (M-131 du 02-04) : textes simples (niveau de lecture visé : fin de primaire pour les cartes d'action), pictogrammes accompagnant les libellés, lecture vocale native du téléphone compatible, pas de délai imposé.
- Déclaration d'accessibilité et lien en pied de page ; schéma pluriannuel publié par l'éditeur pour le produit (§5).

### 14.2 Langues [R]

- Interface du portail famille et élève traduisible (R-I4 du 3, R-A36 du 4, M-277 du 07) : français par défaut, langues ajoutées par lot (anglais, arabe, espagnol, portugais, turc, ukrainien… selon la population de l'établissement, Q-PO6) ; **les titres neutres des tâches** ont des gabarits traduits ; les pièces juridiques et les décisions restent en français avec une mention traduite.
- Sens de lecture droite-gauche supporté par la mise en page (propriétés logiques CSS).

### 14.3 Mobile (socle 11) [R]

- « Aujourd'hui » est l'écran d'ouverture de l'appli, pour tous les rôles (D-003) ; mêmes vues `ent-types` que le web.
- **Hors ligne** : dernière version de l'accueil et de l'à-faire lisible, horodatée (« à jour à 7 h 42 ») ; aucune action rejouée en silence (R-PO20).
- **Widgets** du système (iOS, Android) : **plus tard** (P16), contenu neutre seulement (« 2 choses à faire », « Prochain cours 10 h salle 12 ») et désactivables ; jamais un nom d'élève sur l'écran verrouillé.
- Pastille d'icône = `compteur_a_faire`.

### 14.4 Performance [R]

- `GET /portail/aujourdhui` : **une** requête, p95 < 400 ms côté serveur pour un parent de trois enfants ; budget par tuile 80 ms, au-delà la tuile est rendue « en cours » et complétée par une seconde requête ciblée (`GET /portail/tuiles/{code}`).
- `GET /portail/a-faire` : p95 < 150 ms (index) + revérification par lot (un appel par socle concerné).
- Rendu serveur Leptos : la page est lisible sans JavaScript (états, liens) ; l'hydratation ajoute les cartes d'action.
- Aucun cache serveur de contenu entre requêtes (contraire de NXP-16) ; cache HTTP `private, no-store` pour les pages personnelles.

### 14.5 Mode discret et projection [R]

- Bouton « Mode discret » (et automatique quand l'écran est partagé ou projeté, si le navigateur le signale — sinon manuel) : masque noms d'élèves, notes, compteurs nominatifs ; garde l'heure, la salle, le nombre.
- Défaut **activé** sur le poste de l'accueil (profil Accueil, S-2908), pour que les visiteurs ne lisent pas l'écran.

---

## 15. Droits et minimisation

### 15.1 Actions D-017 propres au socle 29 [R]

Le 29 n'ajoute **aucun droit sur les données** ; il ajoute des droits d'**usage** :

| Action | Ouvre | Portée | Modèle par défaut |
|---|---|---|---|
| `portail.utiliser` | son accueil, son à-faire, ses rappels | moi | tous (personnel, familles, élèves) |
| `afaire.file.voir` | voir une file collective et ses prises en charge | la file (déclarée) | membres de la file (déduit du droit sous-jacent) |
| `afaire.file.prendre` | prendre en charge une tâche d'une file | la file | idem |
| `afaire.file.reattribuer` | réattribuer, relâcher la prise d'un autre | la file | responsable de service (CPE, chef de service, direction) |
| `afaire.superviser` | voir le **nombre** de tâches ouvertes et en retard par file et par nature (jamais le contenu des tâches nominatives d'autrui) | établissement | direction |
| `afaire.rapprocher` | lancer un rapprochement et lire son rapport | établissement | administrateur |
| `portail.modeles.gerer` | ordre par défaut des tuiles et raccourcis par modèle de rôle | établissement | direction, administrateur |
| `portail.liens.gerer` | liens de l'établissement | établissement | direction, administrateur |
| `recherche.globale` | la recherche fédérée du personnel | moi (les sources appliquent leurs portées) | personnel |
| `recherche.debloquer` | lever un blocage anti-énumération | établissement | administrateur |

Familles et élèves restent hors rôles libres (D-017) : leurs tuiles et tâches découlent des droits du lien (4) et de la majorité (2).

### 15.2 Ce que voit chacun [R]

- Une personne voit **ses** tâches (nominatives, de son foyer, de ses files) et **rien** des tâches nominatives d'autrui — y compris la direction (elle supervise des **nombres**, `afaire.superviser`).
- Les membres d'une file voient les tâches de la file **telles que le socle propriétaire les leur montrerait** (revérification), et qui les a prises.
- Les états personnels (vue, report, masquage) ne sont visibles que de la personne.
- Un responsable ne voit jamais si l'autre a lu, reporté, fait (R-PO7).

### 15.3 Garde-fous [R]

1. **Revérification à l'affichage** (R-PO3) et à l'exécution (commande du propriétaire).
2. **Aucune donnée sensible dans l'index** (R-PO5) ; un test du kit refuse un gabarit qui contient une variable marquée C2 ou plus, sauf nature déclarée réservée à des profils habilités (infirmerie).
3. **Pas de jointure entre établissements** (R-PO17) ; RLS sur toutes les tables (E13).
4. **Recherche bornée et non conservée** (R-PO19).
5. **Mode discret** et titres neutres pour l'écran (R-PO13).
6. **Le 29 n'exécute rien lui-même** : toute action passe par la commande du propriétaire (R-PO1).
7. **Aucune trace de lecture exposée** à un tiers (R-PO7).

### 15.4 Matrice de séparation (tests) [R]

Ajouter à `ent-api/tests/separation.rs` (et à la génération par déclaration de routes, D-026) : pour chaque route `/portail/*` et chaque nature déclarée : parent d'un autre foyer, parent au lien suspendu, parent sans exercice de l'autorité, beau-parent sans lien, élève (autre élève de la même classe), élève majeur et ses parents, professeur hors de ses groupes (D-022), professeur-parent dans la mauvaise casquette, AESH, accueil, membre d'une autre file, direction (contenu nominatif d'autrui), autre établissement, personne partie, compte désactivé. Attendu : 403, 404 ou absence de la tâche (selon la route), **jamais** le titre d'une tâche d'autrui.

### 15.5 Données personnelles et conservation [R]

- **Index des tâches** : finalité = organiser le travail et informer chacun de ce qui l'attend ; base = mission de l'établissement (public : mission d'intérêt public ; privé sous contrat : à qualifier par le DPO, Q-PO5) ; conservation : tâches closes **30 jours** (historique personnel « fait récemment »), puis purge ; journal d'états **12 mois** (aligné sur le kit juridique du SDET, §5.4) (preuve qu'une tâche a été ouverte et close, utile en cas de litige : « on ne m'a jamais demandé ») — durées à valider par le DPO (S-2909).
- **Préférences d'affichage** : jusqu'au départ de la personne.
- **Rappels personnels** : fin d'année scolaire.
- **Compteurs de recherche** : 30 jours.
- Inscription au registre des traitements (27) comme sous-traitement de chaque domaine, et au registre des données de santé indirectes du 34 pour la seule mention « à l'infirmerie » (A8).

---

## 16. Cas limites (catalogue)

### 16.1 Familles

| # | Cas | Comportement attendu |
|---|---|---|
| CPO-01 | Parent de trois enfants dans le même établissement, droits différents par enfant (notes pour deux, pas pour le troisième) | chaque tuile ne montre que les enfants permis ; aucune ligne vide « notes non disponibles » pour le troisième |
| CPO-02 | Parents séparés, autorité conjointe, autorisation de sortie « par autorité » | tâche chez les deux ; le premier signe ; elle disparaît chez l'autre sans nom ni date ; un ancien lien affiche « déjà traitée » |
| CPO-03 | Parents séparés, formulaire « par responsable » (chacun répond) | deux tâches indépendantes ; aucun ne voit la réponse de l'autre |
| CPO-04 | Parents en désaccord déclaré | tâche `en_attente` « l'établissement reviendra vers vous » chez les deux ; jamais la réponse de l'autre |
| CPO-05 | Lien suspendu par décision de justice pendant que le parent a l'accueil ouvert | au prochain rafraîchissement ou action : tâches et tuiles de l'enfant disparaissent ; l'action en cours est refusée par le propriétaire |
| CPO-06 | Parent sans exercice de l'autorité | informations selon le lien ; aucune tâche de décision ; mention §9.3 |
| CPO-07 | Beau-parent sans lien, connecté avec son propre compte | aucune donnée de l'enfant |
| CPO-08 | Famille d'accueil (ASE) avec droits limités par le jugement | tâches seulement des natures que le lien ouvre ; les autres vont aux titulaires |
| CPO-09 | Payeur unique ≠ titulaire de l'autorité | « régler » chez le payeur seulement ; le titulaire ne voit pas le montant |
| CPO-10 | Répartition 50/50 entre deux payeurs séparés | chacun voit sa part et son échéance, jamais celle de l'autre |
| CPO-11 | Enfant qui devient majeur pendant une tâche ouverte | bascule vers l'élève (§8.8) ; les parents voient « cette démarche est désormais suivie par Léa » si le 4 le permet, sinon rien |
| CPO-12 | Élève majeur qui refuse que ses parents soient informés (selon §5) | les tuiles des parents suivent la décision portée par le 4 ; l'accueil de l'élève l'explique |
| CPO-13 | Enfant parti en cours d'année avec une facture ouverte | visible « ancien élève » pour le payeur jusqu'au solde |
| CPO-14 | Enfant décédé | tout disparaît sans message ; aucune relance (A14) |
| CPO-15 | Responsable décédé | ses tâches `sans_objet` ; l'autre titulaire reçoit les tâches de décision si le 4 le prévoit |
| CPO-16 | Enfants dans deux établissements ScolIA | mobile : page réunie ; web : sélecteur (et compteurs en P9) |
| CPO-17 | Enfant dans un établissement ScolIA et un autre non ScolIA | rien de l'autre ; aucun lien |
| CPO-18 | Parent sans compte (D-020) | aucune page ; les tâches qui le visent passent par lien nominatif (7, D-027 C, G) ou courrier, et restent indexées pour la vie scolaire (« 3 familles sans compte à relancer autrement », NTF-63) |
| CPO-19 | Parent qui a deux adresses e-mail et deux comptes par erreur | fusion de personnes (4) ; l'index suit la personne fusionnée (rapprochement) |
| CPO-20 | Parent qui ouvre l'accueil à 23 h | page à jour ; aucune notification sonore (plage calme 8, 16) |
| CPO-21 | Tâche dont l'échéance tombe pendant les vacances | échéance indicative reportée au premier jour ouvré si la nature l'admet (S-2906) |
| CPO-22 | Parent malvoyant, lecteur d'écran | parcours complet au clavier et à la voix (§14.1) |
| CPO-23 | Parent allophone | interface traduite ; pièces juridiques en français avec résumé traduit |

### 16.2 Élèves

| # | Cas | Comportement attendu |
|---|---|---|
| CPO-24 | Élève qui tente de chercher un camarade | aucune recherche d'élève ; « Aucun résultat » identique à une recherche vide |
| CPO-25 | Élève en stage trois semaines | EDT de cours masqué, tuile stage ; travaux à rendre maintenus si le professeur l'a voulu |
| CPO-26 | Élève inclus en ULIS une partie du temps | sa journée réelle (public du cours à la date, 5) |
| CPO-27 | Élève à confidentialité renforcée (protection) | n'apparaît dans aucune recherche des non-habilités ; son accueil est normal |
| CPO-28 | Élève qui projette son écran en classe | mode discret disponible ; notes jamais affichées d'emblée |
| CPO-29 | Élève interne le mercredi soir | soirée et sortie autorisée |
| CPO-30 | Élève exclu temporairement | accueil normal (travail à faire) ; la sanction n'apparaît pas en tuile, seulement la convocation et le travail |

### 16.3 Personnels

| # | Cas | Comportement attendu |
|---|---|---|
| CPO-31 | Professeur partagé entre deux établissements | mobile : journée réunie ; aucun établissement ne voit l'autre |
| CPO-32 | Professeur-parent | casquette ; compteur de l'autre casquette seulement |
| CPO-33 | Professeur en arrêt maladie | ses tâches dues vont à la file de repli sur geste (§8.9) ; son accueil reste accessible (lecture) mais n'affiche aucune relance |
| CPO-34 | Professeur remplaçant arrivé hier | accueil d'arrivée (A9) ; ses séances ; ses tâches d'appel dès la première séance |
| CPO-35 | Délégation de signature du chef à l'adjoint pendant une semaine | parapheur indexé aussi chez l'adjoint, « pour Mme X » |
| CPO-36 | Deux CPE prennent la même tâche en même temps | la première prise gagne (contrainte en base) ; la seconde voit « prise par Mme Y à l'instant » |
| CPO-37 | CPE qui part en congé avec 15 tâches prises | relâchées automatiquement après S-2901 ; la file les retrouve |
| CPO-38 | Personnel qui change de fonction (AED devient secrétaire) | ses files changent avec ses droits ; ses tâches de l'ancienne file vont à la file |
| CPO-39 | Personnel parti, compte désactivé | aucune tâche ; ses tâches nominatives restantes sont à la file de repli |
| CPO-40 | AESH qui accompagne un élève dans deux classes | journée de l'élève, pas des classes |
| CPO-41 | Professeur hors de ses groupes qui ouvre un lien de tâche reçu par erreur | refus du propriétaire ; aucune donnée |
| CPO-42 | Direction qui veut « voir les tâches de M. X » | nombres par nature seulement (`afaire.superviser`) |
| CPO-43 | Accueil qui cherche un élève pour une remise | « où est … » neutre ; « qui peut venir chercher » (ADM-44) |
| CPO-44 | Vie scolaire qui cherche un élève absent à l'infirmerie | « à l'infirmerie » seulement si `sante.localisation.lire` ; sinon « hors cours autorisé » |
| CPO-45 | Infirmière connectée sur un poste partagé | tuiles santé seulement dans sa session ; mode discret conseillé |
| CPO-46 | Personnel avec 300 tâches ouvertes (rentrée) | regroupement par nature ; aucune lenteur (index) |
| CPO-47 | Personnel qui fait 500 recherches en une matinée (rentrée, secrétariat) | seuil adapté par rôle (S-2907), puis ralentissement et tâche à l'administrateur, jamais de blocage sans recours (`recherche.debloquer`) |

### 16.4 Système

| # | Cas | Comportement attendu |
|---|---|---|
| CPO-48 | Une source (socle) en panne | tuile « indisponible, réessayer » ; le reste de la page s'affiche (R-PO12) |
| CPO-49 | Le socle propriétaire ne répond pas à `verifier_taches` | les tâches de ce socle sont affichées « à vérifier » sans carte d'action (aucune action sans revérification) |
| CPO-50 | Événement d'ouverture perdu | rapprochement nocturne |
| CPO-51 | Événement de fermeture arrivé avant l'ouverture (désordre) | version de source et idempotence : la fermeture l'emporte |
| CPO-52 | Rejeu de l'outbox | idempotent sur `(nature, objet, destinataire)` |
| CPO-53 | Restauration de la base (PITR) | index neutralisé puis reconstruit (§8.10, E9) |
| CPO-54 | Nature retirée d'un socle | ses tâches ouvertes passent `sans_objet` avec rapport ; un test refuse un retrait sans migration |
| CPO-55 | Changement de gabarit de titre | les tâches ouvertes gardent leur titre ; nouvelles tâches avec le nouveau |
| CPO-56 | Bascule d'année avec tâches ouvertes | elles restent, étiquetées de leur année (§8.9) |
| CPO-57 | Fermeture imprévue décidée la veille au soir | mode `journee_modifiee` dès la publication ; familles en tête de page le matin |
| CPO-58 | Alerte PPMS pendant qu'un parent a l'accueil ouvert | bandeau « consigne officielle » seulement, sans détail opérationnel |
| CPO-59 | Alerte PPMS : professeur sans réseau | appli : liste de la séance en cours en cache (14, 11) |
| CPO-60 | Changement d'heure (octobre, mars) | échéances en heure murale ; aucune tâche « due » décalée d'une heure |
| CPO-61 | Établissement en outre-mer | fuseau du site (10 R-E10) |
| CPO-62 | Session expirée pendant une carte d'action | reconnexion sans perte de la saisie (D-011) ; l'action n'est pas exécutée deux fois |
| CPO-63 | Tâche ouverte pour une personne sans compte puis compte créé | la tâche apparaît à la première connexion |
| CPO-64 | Établissement qui désactive un module (ex. cantine) | tuiles et natures du module absentes ; tâches ouvertes `sans_objet` sur geste de l'administrateur |
| CPO-65 | Module santé désactivé (D-034, sans HDS) | aucune tuile, aucune nature santé ; « à l'infirmerie » n'existe pas (rendu « hors cours autorisé ») |
| CPO-66 | Mesure d'audience | aucun traceur tiers ; mesures techniques internes agrégées seulement (§5) |
| CPO-67 | Lien de l'établissement vers un site malveillant | https seulement, avertissement de sortie, journal de modification |
| CPO-68 | Injection dans un titre (nom d'élève contenant du balisage) | titres échappés au rendu (Leptos), gabarits typés |
| CPO-69 | Compte élève utilisé par le parent | aucune différence visible côté serveur ; le journal des accès (27) aide l'établissement |
| CPO-70 | Démo de décembre sur données fictives | accueils complets pour chaque profil de la démo (D-034) |
| CPO-71 | Année de doublon (D-034) : tâches qui existent aussi dans Charlemagne | les natures doublées sont désactivées ou marquées « information » pendant le doublon (Q-PO4) |
| CPO-72 | Module de l'add-on en usage réel (D-035) qui remplace Next.js | ses tâches doivent exister dès sa bascule (ex. sorties en avril 2027, 42) |

---

## 17. Intelligence artificielle (contrat IA du socle)

### 17.1 Ce que l'IA fait [R]

- **« Que dois-je faire cette semaine ? »** (IA_COEUR : 648, usage S) : lecture `portail.a_faire.lire` (la même liste, mêmes droits), réponse en langage simple, ordonnée ; **aucune** tâche inventée, chaque élément cite sa tâche (lien).
- **Résumé du matin** du personnel (facultatif, `ia.utiliser`) : « 3 appels en retard hier, 2 réunions, 1 changement » — **calculé de façon déterministe** ; le modèle de langage seulement pour la formulation, si l'établissement l'a activé.
- **Proposition d'ordre** (« commencez par… ») : déterministe d'abord (échéance, priorité), l'IA explique, ne réordonne pas l'index.
- **Brouillons** : depuis une tâche, l'assistant prépare la commande du propriétaire (justification, réponse) ; confirmation humaine (IA_COEUR §6).

### 17.2 Ce que l'IA ne fait jamais [R]

- Fermer une tâche, en ouvrir une (hors rappel personnel demandé par la personne), réattribuer, prendre en charge.
- Lire une tâche ou une tuile de sensibilité C3 (santé, psy, social) — **même avec droit**, sauf `ia.donnees_sensibles` pour soi et natures explicitement ouvertes (IA_COEUR §5.5) ; les tâches de l'infirmerie sont **exclues** par défaut.
- Lire les rappels personnels d'autrui ; lire l'historique de recherche (il n'existe pas).
- Déduire quoi que ce soit de l'activité d'un autre responsable.

### 17.3 Fiches du registre [R]

| Identifiant | Type | Droit | Sensibilité | Volume | Description pour le modèle |
|---|---|---|---|---|---|
| `portail.a_faire.lire` | lecture | `portail.utiliser` | C1 (titres neutres) | 200 tâches | « Tâches que la personne doit faire, avec échéance, enfant concerné et action possible. » |
| `portail.aujourdhui.lire` | lecture | `portail.utiliser` | C1 | 1 page | « Journée de la personne : cours, changements, rendez-vous, bandeaux. » |
| `portail.rappel.creer` (POR-180) | commande | `portail.utiliser` | C1 | 1 | N1, réversible (suppression) |
| `portail.rechercher` | lecture | `recherche.globale` | C1 | 40 | « Recherche de pages, actions, élèves et personnes que la personne peut déjà ouvrir. » |

Jeu d'évaluation : 10 questions types (« qu'est-ce que je dois signer pour Tom ? », « ai-je oublié un appel ? »), refus attendus (autre parent, autre élève, autre établissement, tâche santé).

---

## 18. Actions métier (POR-100 à POR-251)

Colonnes : identifiant · action · qui · remarque (droit, socle propriétaire de l'effet, confirmation). « Propriétaire » = la commande est celle du socle cité ; le 29 n'en est que le point d'entrée.

### 18.1 Consulter l'accueil (POR-100 à POR-119)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-100 | Ouvrir « Aujourd'hui » de sa casquette | tous | `portail.utiliser` ; composition serveur |
| POR-101 | Voir « maintenant » (séance en cours ou prochaine, salle, intervenant) | personnel, élève, famille | 10 |
| POR-102 | Voir « ma journée » avec l'état de chaque séance (appel, cahier, travaux) | professeur | 10, 12, 20 |
| POR-103 | Voir « ce qui a changé depuis ma dernière visite » | tous | 10, 2, 5 ; calcul depuis `vue_le` |
| POR-104 | Voir « demain » (la veille à partir de 16 h) | tous | 10 §14.3 |
| POR-105 | Voir l'aujourd'hui de chaque enfant (vue « tous ») | famille | droits par enfant |
| POR-106 | Voir l'aujourd'hui d'un enfant choisi | famille | mémorisé sur l'appareil |
| POR-107 | Voir « pour demain » (travail, évaluations annoncées) | élève, famille | 20 |
| POR-108 | Afficher sur geste les dernières notes publiées | élève, famille | 21 ; jamais d'emblée |
| POR-109 | Voir les messages non lus et le dernier fil | tous | 16 |
| POR-110 | Voir « à la une » (annonces de sa classe, de son profil) | tous | 17 ; corrige DP-01 |
| POR-111 | Voir son prochain rendez-vous et les rencontres ouvertes | tous | 19 |
| POR-112 | Voir son solde et sa prochaine échéance | payeur | 24, 25 ; sa part seulement |
| POR-113 | Voir le menu et la réservation de cantine à faire | élève, famille | 40 |
| POR-114 | Voir « en sortie aujourd'hui » / « en stage » | famille, élève, personnel concerné | 42, 39 |
| POR-115 | Voir les bandeaux actifs | tous | §10.2 |
| POR-116 | Voir l'accueil en mode (rentrée, conseil, vacances, examens, journée modifiée, alerte) | tous | §10.1 |
| POR-117 | Réessayer une tuile indisponible | tous | R-PO12 |
| POR-118 | Passer en mode discret / le quitter | tous | §14.5 |
| POR-119 | Ouvrir « ce que voit ScolIA de moi » (journal des accès) | tous | 27 |

### 18.2 L'à-faire personnel (POR-120 à POR-149)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-120 | Ouvrir son à-faire complet (trié, groupé) | tous | index + revérification |
| POR-121 | Filtrer par enfant, par nature, par échéance, par établissement (mobile) | tous | |
| POR-122 | Voir pourquoi une tâche est à soi (« vous êtes responsable de Léa avec le droit de justifier ») | tous | rendu par le propriétaire |
| POR-123 | Voir ce qui se passe si la tâche n'est pas faite (« la sortie se fera sans Léa ») | tous | texte déclaré par la nature |
| POR-124 | Justifier une absence depuis une carte d'action | responsable, élève majeur | propriétaire 12 ; N1 |
| POR-125 | Justifier plusieurs demi-journées d'un même motif | responsable | 12 ; action groupée |
| POR-126 | Signer un document depuis une carte | signataire | 7 ; N2, preuve |
| POR-127 | Signer plusieurs documents de plusieurs enfants en un geste | responsable | 7 DCT-141 ; N2 |
| POR-128 | Autoriser une sortie | titulaire de l'autorité | 42, 18 ; N2 |
| POR-129 | Répondre à un formulaire ou un sondage | destinataire | 18 |
| POR-130 | Donner ou retirer un consentement | titulaire | 18 ; versionné |
| POR-131 | Régler une échéance | payeur | 25 ; N2, aperçu du montant |
| POR-132 | Choisir un créneau de rendez-vous | destinataire | 19 |
| POR-133 | Confirmer la réinscription (multi-enfants) | responsable | 3 ; « 3 gestes » |
| POR-134 | Compléter la fiche de rentrée (« rien n'a changé ») | responsable | 4 ADM-63 |
| POR-135 | Déposer une pièce demandée | destinataire | 7 |
| POR-136 | Accuser réception d'une information (annonce, carnet) | destinataire | 17, 16 |
| POR-137 | Signer le carnet de correspondance | responsable | 16 / 13 |
| POR-138 | Faire l'appel d'une séance en retard | professeur | 12 ; lien profond |
| POR-139 | Saisir les notes d'une évaluation avant l'arrêt | professeur | 21 ; lien profond |
| POR-140 | Rédiger une appréciation manquante avant le conseil | professeur | 23 ; lien profond |
| POR-141 | Corriger des copies remises | professeur | 20 |
| POR-142 | Confirmer sa présence à une réunion | personnel | 19 |
| POR-143 | Signer au parapheur (chef, adjoint délégataire) | direction | 7 ; N2 |
| POR-144 | Décider une tâche bloquante (interdiction provisoire, « ne pas solliciter ») | direction | 4 R-A9, 8 D-028 ; N2 |
| POR-145 | Reporter une tâche non bloquante (« me le rappeler lundi ») | tous | état personnel ; interdit si bloquante ou échéance dure < 24 h |
| POR-146 | Masquer une information lue | tous | `information` seulement |
| POR-147 | Voir l'historique « fait récemment » (30 jours) | tous | |
| POR-148 | Signaler « ce n'est pas à moi » | tous | tâche au propriétaire (file de repli) ; la tâche reste affichée jusqu'à décision |
| POR-149 | Voir l'état « en attente » et sa raison neutre | tous | §8.4 |

### 18.3 Files collectives (POR-150 à POR-169)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-150 | Voir les tuiles de ses files (compte, plus ancienne échéance) | membres | A4 |
| POR-151 | Ouvrir la file dans l'écran du socle propriétaire | membres | lien |
| POR-152 | Prendre en charge une tâche | membres | `afaire.file.prendre` ; contrainte d'unicité |
| POR-153 | Relâcher une tâche prise | celui qui l'a prise | |
| POR-154 | Réattribuer une tâche à un collègue de la file | responsable de service | `afaire.file.reattribuer` ; journalisé |
| POR-155 | Voir qui a pris quoi | membres | |
| POR-156 | Expiration automatique d'une prise (S-2901) | système | |
| POR-157 | Escalader une tâche bloquante à l'échéance | système | nature |
| POR-158 | Transférer une tâche vers une autre file | membres | si la nature le permet (ex. vie scolaire → infirmerie : jamais avec contenu santé) |
| POR-159 | Ajouter un délégataire aux tâches d'un absent (proposition) | système, responsable | §8.9 |
| POR-160 | Voir les tâches qui échoient pendant l'absence d'un collègue | responsable de service | nombres puis liste de la file |
| POR-161 | Superviser : nombre de tâches ouvertes et en retard par file et par nature | direction | `afaire.superviser` ; jamais le contenu nominatif |
| POR-162 | Voir le délai médian de traitement d'une file (jour, semaine) | responsable, direction | indicateur vers 47 |
| POR-163 | Relancer en un geste les appels non faits de la séance en cours | CPE | 12 ; avis au professeur (8) |
| POR-164 | Voir « familles à prévenir autrement » | vie scolaire, secrétariat | 8 NTF-63 |
| POR-165 | Voir « bilan du matin » | vie scolaire | 8 NTF-146 |
| POR-166 | Voir la file d'une boîte de service | membres de la boîte | 16 |
| POR-167 | Voir le parapheur partagé (chef et adjoint) | direction | 7 |
| POR-168 | Voir les tâches orphelines (sans destinataire résolu) | direction | file de repli (A13) |
| POR-169 | Clore « sans objet » une tâche orpheline après vérification | direction | demandé au propriétaire |

### 18.4 Enfants, casquettes, établissements (POR-170 à POR-179)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-170 | Choisir la vue « tous mes enfants » ou un enfant | famille | |
| POR-171 | Voir un enfant parti en « ancien élève » tant qu'une tâche le concerne | famille | |
| POR-172 | Changer de casquette | personne multi-casquette | journalisé (1) |
| POR-173 | Voir le compteur de l'autre casquette | idem | S-2903 |
| POR-174 | Changer d'établissement (sélecteur) | multi-établissement | sans donnée métier |
| POR-175 | Voir la page réunie de ses établissements (mobile) | multi-établissement | réunion sur l'appareil |
| POR-176 | Voir les compteurs par établissement (web, P9) | multi-établissement | entiers seuls |
| POR-177 | Voir sa journée de deux établissements avec les trajets (mobile) | professeur partagé | |
| POR-178 | Être averti d'un conflit horaire entre deux établissements (appareil seulement) | professeur partagé | |
| POR-179 | Voir « ce que voient vos parents » | élève majeur | 4 |

### 18.5 Rappels personnels (POR-180 à POR-186)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-180 | Créer un rappel | tous | ≤ 140 car. ; avertissement données sensibles |
| POR-181 | Lier un rappel à une séance, une classe, un élève de sa portée | personnel | portée revérifiée |
| POR-182 | Mettre une échéance à un rappel | tous | |
| POR-183 | Cocher un rappel fait | tous | |
| POR-184 | Supprimer un rappel | tous | effacement réel |
| POR-185 | Expiration des rappels en fin d'année | système | S-2904 |
| POR-186 | Exporter ses rappels (droit d'accès) | tous | 27 |

### 18.6 Recherche globale (POR-187 à POR-196)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-187 | Rechercher un élève dans sa portée | personnel | 2 (E6) |
| POR-188 | Rechercher un responsable ou un contact | personnel habilité | 4 |
| POR-189 | Rechercher un personnel dans l'annuaire | personnel | 6 ; `identite_restreinte` |
| POR-190 | Rechercher une classe, un groupe, un cours, une salle | personnel | 5 STR-90 |
| POR-191 | Rechercher un document (métadonnées) | tous | 7 |
| POR-192 | Rechercher une conversation où l'on participe | tous | 16 |
| POR-193 | Rechercher dans ses propres données (famille : enfant, documents, messages) | famille, élève | |
| POR-194 | « Où est … maintenant » (neutre) | vie scolaire, accueil | 12 `ou_est` (A8) |
| POR-195 | Ralentissement et blocage anti-énumération | système | S-2907 ; tâche à l'administrateur |
| POR-196 | Lever un blocage de recherche | administrateur | `recherche.debloquer` ; journalisé |

### 18.7 Palette de commandes (POR-197 à POR-202)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-197 | Ouvrir la palette (Ctrl/⌘ + K) | tous | |
| POR-198 | Aller à une page permise | tous | navigation |
| POR-199 | Lancer une action du registre (paramètres, aperçu, confirmation) | tous | IA_COEUR §5.2 |
| POR-200 | Utiliser un alias (« abs ») | tous | |
| POR-201 | Revenir aux dernières commandes utilisées (sur l'appareil) | tous | stockage local, jamais serveur |
| POR-202 | Voir les raccourcis clavier | tous | |

### 18.8 Kit, rapprochement, administration (POR-203 à POR-215)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-203 | Déclarer une nature de tâche (code) | équipe d'un socle | kit D-026 |
| POR-204 | Déclarer une tuile (code) | équipe d'un socle | contrat §9.1 |
| POR-205 | Lancer un rapprochement et lire son rapport | administrateur | `afaire.rapprocher` ; rapport par nature, sans nom |
| POR-206 | Rapprochement nocturne | système | §8.10 |
| POR-207 | Reconstruire l'index après restauration | système | E9 |
| POR-208 | Ouvrir une tâche (port) | socle | `ouvrir_tache`, idempotent |
| POR-209 | Fermer une tâche (port) | socle | `fermer_tache` |
| POR-210 | Ajouter un destinataire (droit acquis) | socle | idempotent |
| POR-211 | Mettre sans objet les tâches d'une personne décédée ou d'un élève parti | système | A14, R-PO10 |
| POR-212 | Répondre à `etat_taches` (le 8 avant une relance) | système | R-PO6 |
| POR-213 | Fournir `taches_ouvertes_de(personne)` au départ guidé et au droit d'accès | système | 6, 27 |
| POR-214 | Désactiver les natures doublées pendant l'année de doublon | administrateur | Q-PO4 |
| POR-215 | Purger les tâches closes, les compteurs et les rappels expirés | système | §15.5 |

### 18.9 Personnalisation et liens (POR-216 à POR-225)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-216 | Réordonner ses tuiles | tous | |
| POR-217 | Masquer une tuile non essentielle | tous | pas « À faire », pas les bandeaux de sécurité |
| POR-218 | Épingler une tuile | tous | |
| POR-219 | Rétablir la page par défaut | tous | |
| POR-220 | Définir l'ordre par défaut des tuiles d'un modèle de rôle | direction, administrateur | `portail.modeles.gerer` |
| POR-221 | Ajouter des raccourcis à un modèle de rôle | idem | |
| POR-222 | Gérer les liens de l'établissement | idem | `portail.liens.gerer` ; https seulement |
| POR-223 | Voir l'aperçu de l'accueil d'un profil (« voir comme un parent ») | direction, administrateur | **données fictives** du profil, jamais l'accueil réel d'une personne |
| POR-224 | Choisir sa langue d'interface | tous | |
| POR-225 | Activer le mode discret par défaut sur un poste | personnel | S-2908 |

### 18.10 Modes et bandeaux (POR-226 à POR-235)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-226 | Basculer l'accueil en mode alerte | système, sur fait du 14 | non masquable |
| POR-227 | Déclarer « je suis en sécurité avec … » depuis l'accueil d'alerte | personnel | 14 |
| POR-228 | Voir la consigne officielle d'alerte | famille | 14, sans détail opérationnel |
| POR-229 | Passer en mode `journee_modifiee` | système, sur fait du 5 ou 10 | |
| POR-230 | Passer en mode `rentree` | système | 5 |
| POR-231 | Passer en mode `veille_conseil` | système | 23 |
| POR-232 | Passer en mode `vacances` | système | 5 |
| POR-233 | Passer en mode `examens` | système | 38 |
| POR-234 | Afficher un bandeau déclaré | système | §10.2 |
| POR-235 | Replier / masquer un bandeau masquable | tous | |

### 18.11 Accessibilité, mobile, langues, contrôles (POR-236 à POR-245)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-236 | Parcourir tout l'accueil au clavier et au lecteur d'écran | tous | §14.1 |
| POR-237 | Zoomer à 200 % / largeur 320 px sans perte | tous | |
| POR-238 | Lire l'accueil hors ligne (mobile) | tous | horodaté |
| POR-239 | Recevoir sur l'icône de l'appli le nombre de tâches actionnables | tous | R-PO4 |
| POR-240 | Ouvrir une tâche depuis une notification système | tous | lien profond, revérification |
| POR-241 | Lire la déclaration d'accessibilité | tous | §5 |
| POR-242 | Signaler un défaut d'accessibilité | tous | vers le support (28) |
| POR-243 | Contrôler une déclaration de nature ou de tuile (gabarit neutre, destinataire, repli, clôture) | CI | refus si incomplet |
| POR-244 | Contrôler l'équivalence écran / IA / palette pour chaque nature | CI | IA_COEUR §5.4 |
| POR-245 | Contrôler la séparation de chaque route `/portail/*` | CI | §15.4 |

### 18.12 IA (POR-246 à POR-251)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-246 | Demander « que dois-je faire cette semaine ? » | tous (si `ia.utiliser`) | 30 ; lecture seule |
| POR-247 | Recevoir le résumé du matin | personnel | déterministe, formulation facultative |
| POR-248 | Préparer une commande depuis une tâche (brouillon) | tous | confirmation humaine |
| POR-249 | Expliquer l'ordre proposé | tous | |
| POR-250 | Créer un rappel par la voix ou le texte | tous | POR-180 ; N1 |
| POR-251 | Exclure ses rappels et ses tuiles de l'IA | tous | réglage personnel |

**Total : 152 actions.**

---

## 19. Règles transverses (R-PO1 à R-PO32) [R]

| # | Règle |
|---|---|
| R-PO1 | Le 29 n'exécute aucune action métier : toute action passe par la commande du socle propriétaire, avec ses droits, son aperçu, sa confirmation. |
| R-PO2 | Les droits d'une famille s'évaluent **enfant par enfant** ; jamais un droit sur un enfant n'ouvre quoi que ce soit sur un autre. |
| R-PO3 | Une tâche n'est affichée que si son propriétaire la confirme actionnable par cette personne **à l'instant de la lecture** ; à défaut de réponse, pas de carte d'action. |
| R-PO4 | Le compteur (pastille, icône) compte les tâches actionnables, non les avis ; un domaine n'affiche pas de pastille concurrente pour des tâches indexées. |
| R-PO5 | L'index ne contient qu'un titre neutre composé par le propriétaire depuis un gabarit typé : aucun motif, montant (hors payeur), libellé de santé, de discipline, de suivi social ou de protection. |
| R-PO6 | Le socle 8 vérifie `etat_taches` avant toute relance : aucune relance pour une tâche close. |
| R-PO7 | Aucune activité d'un responsable (lecture, report, réponse, signature) n'est visible d'un autre ; une tâche faite par un autre disparaît sans nom ni date. |
| R-PO8 | Un responsable sans exercice de l'autorité parentale n'a aucune tâche de décision ; il reçoit les informations que le lien lui ouvre. |
| R-PO9 | Toute tâche a un destinataire actif ou va à la file de repli de sa nature ; jamais nulle part (D-028). |
| R-PO10 | Décès d'un élève ou d'une personne : tâches `sans_objet`, aucun message, propriétaires informés pour arrêter leurs relances. |
| R-PO11 | Une tâche bloquante ne se reporte pas et s'escalade à son échéance. |
| R-PO12 | Une source en panne ou lente n'empêche jamais l'affichage de la page ; la tuile l'indique et propose de réessayer. |
| R-PO13 | L'accueil n'affiche d'emblée ni note, ni motif, ni donnée sensible ; le détail s'ouvre sur geste, avec les droits de l'écran propriétaire ; mode discret disponible partout. |
| R-PO14 | Aucune tuile ni tâche de santé, de suivi psychologique ou social hors des profils habilités ; « à l'infirmerie » n'est rendu qu'aux porteurs de `sante.localisation.lire`. |
| R-PO15 | Au plus deux bandeaux visibles ; un bandeau de sécurité n'est jamais masquable ; en alerte, la page d'alerte remplace l'accueil. |
| R-PO16 | Une session = une casquette ; la casquette inactive n'expose qu'un compteur. |
| R-PO17 | Aucune jointure ni agrégation serveur entre établissements ; la réunion se fait sur l'appareil ou dans le navigateur de la personne. |
| R-PO18 | La palette et la recherche ne donnent aucun droit et n'ont aucune logique métier propre : elles appellent le registre des capacités et les ports de recherche. |
| R-PO19 | Recherche côté serveur seulement, 2 caractères minimum, résultats plafonnés, texte jamais conservé, compteur anti-énumération ; aucune liste d'élèves envoyée au navigateur. |
| R-PO20 | Hors ligne, rien n'est exécuté en silence au retour du réseau ; l'action est reproposée avec l'état du serveur. |
| R-PO21 | Les heures sont murales, dans le fuseau du site ; une échéance « vendredi » finit vendredi 23 h 59 heure locale. |
| R-PO22 | Une tâche d'appel devient « en retard » 15 minutes après la fin de la séance (paramètre du 12, S-2910) ; la relance de la vie scolaire n'est jamais publique. |
| R-PO23 | Toute nature déclare son gabarit, son destinataire, sa cardinalité, son échéance, sa clôture, sa sensibilité, sa file de repli ; la CI refuse une nature incomplète. |
| R-PO24 | L'index est reconstructible ; il est neutralisé et rebâti à toute restauration. |
| R-PO25 | Aucun cache serveur de contenu personnel entre requêtes ; un changement de droits est effectif à la requête suivante. |
| R-PO26 | L'établissement ne peut ni montrer une tuile à un profil sans le droit sous-jacent, ni masquer « À faire » ou un bandeau de sécurité. |
| R-PO27 | « Voir comme » un profil n'utilise que des données fictives, jamais l'accueil réel d'une personne. |
| R-PO28 | Aucun traceur tiers ni mesure d'audience nominative sur le portail ; seules des mesures techniques agrégées et internes. |
| R-PO29 | Les rappels personnels ne sont lus que par leur auteur ; ils expirent en fin d'année. |
| R-PO30 | La pastille de l'autre casquette, la page réunie multi-établissements et les widgets ne montrent jamais un nom d'élève sur un écran verrouillé. |
| R-PO31 | **Droit à la déconnexion** (arrêté du 7/09/2026, §5.1) : une tâche, une tuile ou un changement nouveaux ne deviennent visibles d'une famille ou d'un élève qu'à la fin de la plage calme (20 h–7 h en semaine, vendredi 20 h–lundi 7 h), sauf natures d'urgence déclarées et ajustement du chef d'établissement ; l'auteur voit le rappel et l'heure de visibilité ; consultation et saisie restent possibles. Défaut appliqué aussi au privé (S-2911). |
| R-PO32 | La mention de conformité à l'accessibilité figure en pied de l'accueil web et l'état de conformité dans l'application mobile. |

---

## 20. Événements

### 20.1 Émis (outbox, dans la transaction du fait)

| Événement | Quand | Consommateurs |
|---|---|---|
| `portail.tache_ouverte` | indexation d'une tâche (après `ouvrir_tache`) | 8 (avis associé), 11 (pastille) |
| `portail.tache_close` | fermeture (issue) | 8 (avis obsolètes), 11, 47 |
| `portail.tache_prise` / `portail.tache_relachee` | prise en charge | propriétaire de la file |
| `portail.tache_escaladee` | escalade | 8 |
| `portail.rapprochement_ecart` | écart au-delà du seuil | 28 (supervision) |
| `portail.recherche_bloquee` | anti-énumération | 1 (sécurité), administrateur |
| `portail.casquette_changee` | changement de casquette | 1 (journal) |

Les socles propriétaires publient de leur côté, au format commun (A3) : `<socle>.tache_ouverte` et `<socle>.tache_close` avec `nature`, `objet_id`, `destinataires`, `echeance`, `version_source` ; le 29 les consomme et répond par ses propres événements.

### 20.2 Consommés

| Événement | Effet |
|---|---|
| `<socle>.tache_ouverte` / `.tache_close` | indexer, fermer |
| `lien.suspendu`, `lien.modifie`, `lien.cree` (4) | revérifier les tâches des enfants concernés ; ajouter les destinataires |
| `eleve.majeur` (2) | bascule des tâches de décision (§8.8) |
| `eleve.decede` (2), `personne.decedee` (4) | `sans_objet` silencieux (A14) |
| `scolarite.depart_*` (3) | `sans_objet` sauf natures conservées |
| `personnel.arrive`, `personnel.depart_*`, `delegation.*` (6) | arrivée (A9), départ, délégations |
| `adhesion.retiree`, `compte.desactive` (1) | retrait des tâches de la personne pour cet établissement |
| `referentiel.rentree_publiee`, `referentiel.calendrier_*` (5) | mode rentrée, vacances |
| `edt.journee_modifiee`, `seance.*` (10) | mode journée modifiée, « changements » |
| `alerte.declenchee` / `alerte.levee` (14) | mode alerte |
| `conseil.periode_ouverte` (23) | mode veille de conseil |
| `module.desactive` (28) | natures et tuiles du module retirées |

---

## 21. Écrans clés

| Écran | Route web | Mobile | Contenu |
|---|---|---|---|
| Aujourd'hui | `/web` (existant, refondu) | onglet 1 | §9 |
| À faire | `/web/a-faire` | onglet ou feuille | liste triée, groupée, filtres, cartes d'action |
| Fait récemment | `/web/a-faire/fait` | | 30 jours |
| Mes rappels | `/web/a-faire/rappels` | | §8.12 |
| Mes files | `/web/files` | | tuiles de file, prises en charge, renvoi aux écrans des socles |
| Supervision des files | `/web/direction/files` | — | nombres par file et par nature |
| Palette | partout (Ctrl/⌘ + K) | champ de recherche | §12 |
| Recherche | `/web/recherche?q=` | | résultats par source |
| Personnaliser l'accueil | `/web/accueil/personnaliser` | réglages | §13.1 |
| Modèles d'accueil | `/web/direction/accueil` | — | §13.2, aperçu fictif |
| Liens de l'établissement | `/web/direction/liens` | — | §13.3 |
| Rapprochement | `/web/admin/a-faire` | — | rapport |
| Page d'alerte | (remplace `/web`) | (remplace l'onglet 1) | 14 |

Direction artistique (`FRONTEND_CIBLE.md` §2) : Bento 12 colonnes pour Aujourd'hui (tuiles 4 / 6 / 8 / 12, une information et une action) ; **tableau dense** (lignes 44–48 px, en-tête collant, actions groupées) pour l'à-faire complet et les files ; jamais de saisie sur une tuile ; animations seulement à l'apparition d'une carte d'action (120–240 ms), aucune sur les compteurs ; thème sombre par jetons.

---

## 22. Modèle de données cible

Toutes les tables : `etablissement_id UUID NOT NULL` (E1), clé primaire UUID applicative (E3), clé composite `(etablissement_id, id)` et FK composites (E2), `cree_le` / `modifie_le` (E6), RLS standard par `ent_rls_poser` (E13), **aucune colonne JSON ni tableau** (D-036, E10), aucun `BYTEA` (E5), aucune donnée d'établissement hors de ces tables (E4, E14). Les **natures** et les **tuiles** sont déclarées **dans le code** de chaque brique (registre statique, comme le catalogue d'actions et le registre IA) : elles n'ont pas de table de définition, seulement des réglages.

### 22.1 Index des tâches

**`ent_afaire`** — une tâche (identité `(nature, objet, destinataire)` portée par `ent_afaire_destinataire`)
| Colonne | Type | Note |
|---|---|---|
| `id` | UUID | |
| `etablissement_id` | UUID NOT NULL | |
| `nature_code` | TEXT NOT NULL | code déclaré (`vs.justifier_absence`) ; contrôlé par le registre au démarrage (une nature inconnue est refusée) |
| `socle_proprietaire` | SMALLINT NOT NULL | |
| `objet_type` | TEXT NOT NULL | ex. `absence` |
| `objet_id` | UUID NOT NULL | identifiant chez le propriétaire (pas de FK : objet d'une autre brique ; rapprochement, §8.10) |
| `eleve_id` | UUID NULL | FK composite vers l'élève (regroupement, droits par enfant, décès) |
| `titre_neutre` | TEXT NOT NULL | composé par le propriétaire (R-PO5), ≤ 120 caractères |
| `regroupement_cle` | TEXT NULL | ex. `parcours_rentree:2027:{eleve}` |
| `priorite` | TEXT NOT NULL CHECK IN (`bloquante`,`due`,`a_faire`,`information`) | |
| `echeance_le` | TIMESTAMPTZ NULL | instant calculé depuis l'heure murale |
| `echeance_dure` | BOOLEAN NOT NULL DEFAULT false | |
| `visible_a_partir_de` | TIMESTAMPTZ NOT NULL | fin de plage calme si ouverte pendant la plage (R-PO31) |
| `sensibilite` | SMALLINT NOT NULL CHECK 0–3 | |
| `etat` | TEXT NOT NULL CHECK IN (`ouverte`,`prise`,`en_attente`,`faite`,`sans_objet`,`expiree`,`annulee`) | |
| `annee_scolaire_id` | UUID NOT NULL | FK composite (5) ; étiquette à la bascule |
| `version_source` | BIGINT NOT NULL | ordre des faits du propriétaire (CPO-51) |
| `ouverte_le`, `close_le` | TIMESTAMPTZ | |
| `issue` | TEXT NULL | `faite`, `sans_objet`, … (motif neutre) |
| `cree_le`, `modifie_le` | TIMESTAMPTZ NOT NULL | |
Index : `(etablissement_id, etat, echeance_le)`, `(etablissement_id, nature_code, objet_id)`, `(etablissement_id, eleve_id) WHERE etat IN ('ouverte','prise','en_attente')`.

**`ent_afaire_destinataire`** — à qui elle est due
| Colonne | Type | Note |
|---|---|---|
| `id`, `etablissement_id`, `afaire_id` | UUID | FK composite vers `ent_afaire` |
| `type` | TEXT CHECK IN (`personne`,`foyer`,`file`) | |
| `personne_id` | UUID NULL | FK composite (4, D-020) — une personne, avec ou sans compte |
| `foyer_id` | UUID NULL | FK composite (4) |
| `file_code` | TEXT NULL | file déclarée (`vs.justificatifs`) |
| `pour_le_compte_de` | UUID NULL | délégation : personne délégante (§8.9) |
| `role_dans_tache` | TEXT NOT NULL CHECK IN (`titulaire`,`delegataire`,`escalade`,`repli`) | |
| `ajoute_le`, `retire_le` | TIMESTAMPTZ | retrait daté, pas de `DELETE` (E6) |
Contrainte : exactement une des trois références selon `type` ; unicité `(afaire_id, type, personne_id, foyer_id, file_code) WHERE retire_le IS NULL` ; unicité de l'identité `(etablissement_id, nature_code, objet_id, destinataire)` garantie par un index unique partiel sur la jointure matérialisée dans `ent_afaire_cle` (ci-dessous).

**`ent_afaire_cle`** — idempotence des ouvertures (outbox rejouée)
| `etablissement_id`, `nature_code`, `objet_id`, `destinataire_cle` (TEXT : `p:{uuid}`, `f:{uuid}`, `q:{code}`), `afaire_id`, `cree_le` | clé primaire `(etablissement_id, nature_code, objet_id, destinataire_cle)` |

**`ent_afaire_journal`** — transitions, en **ajout seul** (déclencheur)
| `id`, `etablissement_id`, `afaire_id`, `de_etat`, `vers_etat`, `par_compte_id` NULL (système sinon), `par_socle` SMALLINT, `motif_code` TEXT NULL, `le` TIMESTAMPTZ | aucune valeur libre ; 12 mois (§5.4) |

**`ent_afaire_prise`** — prises en charge des tâches collectives
| `id`, `etablissement_id`, `afaire_id`, `compte_id`, `depuis`, `jusqu_a` (expiration S-2901), `relachee_le` NULL, `relachee_par` NULL, `cree_le` | index unique partiel `(afaire_id) WHERE relachee_le IS NULL` (CPO-36 : la première gagne) |

**`ent_afaire_etat_personnel`** — états propres à une personne, jamais lus pour autrui (R-PO7)
| `etablissement_id`, `afaire_id`, `compte_id`, `vue_le` NULL, `reportee_jusqu_au` NULL, `masquee_le` NULL, `modifie_le` | clé primaire `(etablissement_id, afaire_id, compte_id)` ; politique RLS supplémentaire : lecture **seulement** par `compte_id = ent_compte_courant()` (aucune route ne lit l'état d'un autre compte) |

### 22.2 Réglages et files

**`ent_afaire_nature_reglage`** — ce que l'établissement peut régler (peu, D-023)
| `etablissement_id`, `nature_code`, `active` BOOLEAN, `pendant_doublon` TEXT CHECK IN (`normal`,`information`,`desactivee`) (Q-PO4), `delai_escalade_heures` INT NULL (dans les bornes déclarées par la nature), `modifie_par`, `modifie_le`, `cree_le` | clé `(etablissement_id, nature_code)` |

Les **files** sont déclarées par les socles (code, droit d'accès, propriétaire) : pas de table. Les membres d'une file sont **calculés** (droit D-017 et portée, ou membres de la boîte de service du 16), jamais saisis.

### 22.3 Composition et préférences

**`ent_portail_disposition`** — préférences d'une personne pour une casquette
| `id`, `etablissement_id`, `compte_id`, `casquette_code` TEXT, `tuile_code` TEXT, `rang` SMALLINT, `epinglee` BOOLEAN, `masquee` BOOLEAN, `cree_le`, `modifie_le` | unicité `(etablissement_id, compte_id, casquette_code, tuile_code)` ; « À faire » et bandeaux de sécurité refusés en `masquee` (contrainte applicative + test) |

**`ent_portail_modele`** — ordre par défaut des tuiles par modèle de rôle (POR-220)
| `id`, `etablissement_id`, `role_id` (FK composite vers les rôles D-017) NULL, `profil_code` TEXT NULL (famille, élève), `tuile_code`, `rang`, `cree_le`, `modifie_le`, `modifie_par` | une des deux références |

**`ent_portail_raccourci`** — raccourcis ajoutés à un modèle (POR-221) : `id`, `etablissement_id`, `role_id` NULL, `profil_code` NULL, `action_code` TEXT (registre), `rang`, dates.

**`ent_portail_lien`** — liens de l'établissement (§13.3)
| `id`, `etablissement_id`, `libelle` TEXT (≤ 60), `url` TEXT CHECK (`url LIKE 'https://%'`), `rang`, `actif`, `cree_le`, `modifie_le`, `modifie_par` |
**`ent_portail_lien_public`** — public d'un lien (une ligne par profil ou rôle) : `etablissement_id`, `lien_id`, `profil_code` NULL, `role_id` NULL.

### 22.4 Rappels et recherche

**`ent_rappel_personnel`** : `id`, `etablissement_id`, `compte_id`, `texte` TEXT CHECK (`char_length(texte) <= 140`), `echeance_le` NULL, `objet_type` NULL, `objet_id` NULL, `eleve_id` NULL (FK composite, portée revérifiée à la création), `fait_le` NULL, `expire_le` NOT NULL (fin d'année, S-2904), `cree_le`, `modifie_le` ; RLS supplémentaire « propriétaire seul ».

**`ent_recherche_compteur`** (volatile) : `etablissement_id`, `compte_id`, `jour` DATE, `nb` INT, `bloque_jusqu_a` NULL ; clé `(etablissement_id, compte_id, jour)` ; purge 30 jours ; **aucun texte de requête**.

### 22.5 Classement au registre des tables (E12) et restauration

| Table | Portée | Export (PORTABILITE §4) | Restauration |
|---|---|---|---|
| `ent_afaire`, `ent_afaire_destinataire`, `ent_afaire_cle` | etablissement | particulière (reconstructible) | **neutraliser puis reconstruire** (§8.10, E9) |
| `ent_afaire_journal` | etablissement | simple | garder |
| `ent_afaire_prise`, `ent_afaire_etat_personnel` | etablissement | simple | garder |
| `ent_afaire_nature_reglage`, `ent_portail_modele`, `ent_portail_raccourci`, `ent_portail_lien(_public)` | etablissement | simple | garder |
| `ent_portail_disposition`, `ent_rappel_personnel` | etablissement | simple (droit d'accès de la personne) | garder |
| `ent_recherche_compteur` | volatile | exclue | vider |

### 22.6 Ce qui disparaît ou change ailleurs

- `ent_notification` (102) : reçoit une référence facultative `afaire_id` dans le **modèle du socle 8** (3800–3899 ou repli 400–419) — décision du 8, signalée ici, rien écrit par le 29.
- Aucune table du 29 ne porte de motif, de montant, de note ou de donnée de santé.

---

## 23. Plan de migrations (plage imposée 2580–2599)

| N° | Nom | Contenu | Lot |
|---|---|---|---|
| 2580 | `2580_afaire_index` | `ent_afaire`, `ent_afaire_destinataire`, `ent_afaire_cle`, index, RLS | P2 |
| 2581 | `2581_afaire_journal_prises` | `ent_afaire_journal` (ajout seul, déclencheur), `ent_afaire_prise` (unicité partielle), `ent_afaire_etat_personnel` (RLS propriétaire) | P2, P4 |
| 2582 | `2582_afaire_reglages` | `ent_afaire_nature_reglage` | P2 |
| 2583 | `2583_portail_composition` | `ent_portail_disposition`, `ent_portail_modele`, `ent_portail_raccourci` | P10 |
| 2584 | `2584_portail_liens` | `ent_portail_lien`, `ent_portail_lien_public` | P10 |
| 2585 | `2585_recherche_compteur` | `ent_recherche_compteur` (volatile) | P5 |
| 2586 | `2586_rappels_personnels` | `ent_rappel_personnel` | P11 |
| 2587 | `2587_afaire_actions_d017` | actions `portail.*`, `afaire.*`, `recherche.*` au catalogue (si le catalogue est en base ; sinon dans le code, numéro libéré) | P2 |
| 2588 | `2588_afaire_reprise` | reprise initiale de l'index depuis les sources existantes (absences à justifier, carnets, documents à signer, justificatifs, protections) par le rapprochement, **sans** donnée copiée | P2 |
| 2589–2597 | réserve | lots P5 à P16 | |
| 2598 | `2598_portail_rls_stricte` | passage en RLS stricte (contexte d'adhésion obligatoire) | P15 |
| 2599 | `2599_portail_retraits` | retrait de ce que P14 rend inutile (aucune table du 29 ne doit être retirée ; réservé) | P14 |

Le lot **P0** et le lot **P1** n'ont **aucune migration** (routes et écrans seulement). Contrôle de revue : `grep -i "jsonb\|json\|bytea\|\[\]"` sur 2580–2599 ne doit rien trouver.

---

## 24. Lots

| Lot | Contenu | Critères de fin | Dépend de |
|---|---|---|---|
| **P0 — correctifs de l'existant** | DP-01 (annonces famille et élève par la route famille, dédoublonnées), DP-03 (compteurs serveur, plus de notes téléchargées pour un compteur), DP-04 (tuiles sous droits D-017 : `absences.declarer`, `absences.trancher`…), DP-06 (ne pas proposer une tuile interdite), tests de séparation de l'accueil actuel | accueil famille sans tuile en erreur ; un rôle « Accueil » paramétré voit sa tuile ; tests verts | — |
| **P1 — « Aujourd'hui » de la démo de décembre** (D-034) | `GET /portail/aujourdhui` composé côté serveur (une requête, budgets, R-PO12) ; accueils **famille** (multi-enfants : aujourd'hui de chaque enfant, à justifier, carnet, messages, à la une, dernières notes sur geste), **élève** (maintenant, pour demain, notes sur geste, messages), **professeur** (existant amélioré : ma journée avec état de l'appel), **CPE / vie scolaire** (absents, appels non faits, justificatifs), **accueil** (déclarer une absence, élèves à remettre), **secrétariat** (inscriptions à instruire, propositions de coordonnées), **direction** (protections, absences de personnels, incidents), sans index (lectures directes des ports existants) ; fin de la tuile « Votre espace » pour ces profils (DP-02, DP-05, DP-07) | démo D-034 jouée par profil sur données fictives, 375 px, clavier ; aucune donnée hors droits (matrice) | P0 ; socles 2, 3, 4, 10, 12, 16, 21 existants |
| **P2 — index des tâches et kit** | tables 2580–2582, 2587–2588 ; ports `ouvrir_tache` / `fermer_tache` / `verifier_taches` / `taches_ouvertes` dans le kit D-026 ; contrôle CI des déclarations (POR-243) ; rapprochement (POR-205/206) ; premières natures : justifier une absence, signer le carnet, document à signer, justificatif à trancher, appel à faire, protection à confirmer, tâche bloquante (D-028, R-A9) | tâches ouvertes et closes dans la transaction du fait ; rapprochement sans écart sur le labo ; reconstruction après restauration testée | P1, kit D-026 |
| **P3 — à-faire famille complet** | `/web/a-faire`, cartes d'action (justifier, signer, autoriser, répondre, choisir un créneau, réinscrire, fiche de rentrée, déposer une pièce, régler) par les commandes des propriétaires ; cardinalités, autorité, neutralité entre parents (R-PO7), parent sans exercice (R-PO8), majorité (§8.8) ; R-PO31 | parcours P01–P06 de la passe personas sans défaut ; tests de neutralité | P2 ; commandes des socles 7, 12, 16, 18, 19, 25 |
| **P4 — files des personnels** | tuiles de file, prises en charge, réattribution, délégations, escalade, supervision par nombres ; « Mes files » | deux CPE ne traitent jamais la même tâche ; aucune tâche orpheline | P2 ; socle 6 (délégations), 16 (boîtes) |
| **P5 — recherche globale** | ports `rechercher` des socles 2, 4, 5, 6 d'abord ; plafonds ; anti-énumération (2585) ; « où est … » (A8) | aucune liste envoyée au navigateur ; tests d'énumération ; confidentialité renforcée invisible | P1 ; E6 du socle 2 |
| **P6 — palette de commandes** | Ctrl/⌘ + K, aller à / trouver / faire, registre des capacités | équivalence palette / écran / IA testée (POR-244) | P5 ; registre IA (IA-L…) |
| **P7 — modes et bandeaux** | modes du §10.1 (dont **alerte** avec le 14 et **journée modifiée** avec 5 et 10), bandeaux du §10.2 | fermeture décidée la veille visible le matin ; page d'alerte testée | P1 ; 5, 10, 14 |
| **P8 — mobile** | Aujourd'hui et à-faire natifs (socle 11), hors ligne en lecture, pastille, réunion **multi-établissements sur l'appareil**, journée du professeur partagé | parité des droits avec l'API ; aucune jointure serveur | P3 ; socle 11 |
| **P9 — multi-établissements web** | sélecteur ; compteurs par sous-domaine (CORS restreint) | aucun établissement n'apprend l'existence de l'autre (test) | P2 |
| **P10 — personnalisation et liens** | 2583–2584 ; préférences, modèles par rôle, raccourcis, liens, « voir comme » fictif | | P1 |
| **P11 — rappels personnels** | 2586 | lecture par l'auteur seul (test RLS) | P2 |
| **P12 — accessibilité et langues** | audit RGAA de l'accueil, de l'à-faire, de la palette (interne puis externe, Q-PO9) ; déclaration, mention, schéma pluriannuel ; traductions famille | mention publiée ; 0 non-conformité bloquante | P3, P6 |
| **P13 — mesure d'audience et sobriété** | mesures internes agrégées (CNIL), DNMA pour le public | aucun traceur tiers (test) | P1 ; socle 9 |
| **P14 — retrait de `ui.rs`** | rôle par rôle, quand l'accueil Leptos couvre tout ce que l'accueil JS offrait | liste « à ne pas perdre » vérifiée par rôle | P1–P7 |
| **P15 — RLS stricte, indicateurs** | 2598 ; délais de traitement des files vers le 47 | | P4 |
| **P16 — IA et widgets** | « que dois-je faire », résumé du matin (déterministe, formulation facultative), widgets mobiles neutres | jeu d'évaluation ; refus attendus | socle 30, 11 |

**Parcours minimal vendable** : P0, P1, P2, P3, P4, P5, P7, P12. **Démo de décembre** : P0 et P1 seulement (aucune table nouvelle : le risque est faible et la démo ne dépend pas de l'index). **Usage réel 2027-2028 (D-035)** : les natures des modules de l'add-on (sorties, stages, internat, absences RH, demandes) doivent exister au moment de la bascule de chaque module (CPO-72).

---

## 25. Tests et recette

- **Unitaires** : tri, regroupement, montée de priorité, plage calme (R-PO31, y compris vendredi soir et veille de vacances), heures murales et changements d'heure (CPO-60), cardinalités (par responsable, par foyer, par autorité).
- **Contrat du kit** : refus d'une nature ou d'une tuile incomplète (POR-243) ; gabarit sans variable sensible.
- **Séparation** (§15.4) : générée depuis la déclaration des routes et des natures (D-026) ; cas des CPO-01 à CPO-12, CPO-24, CPO-27, CPO-41, CPO-42.
- **Neutralité entre parents** : après signature par A, la réponse de B est identique à celle d'une tâche faite par l'établissement (même texte, même code HTTP, même temps de réponse à 10 % près).
- **Revérification** : suspension d'un lien → la tâche disparaît à la requête suivante (pas de cache, R-PO25).
- **Rapprochement et restauration** : perte d'événements simulée → index réparé ; base restaurée → index reconstruit identique.
- **Recherche** : aucune réponse ne contient un élève hors portée ; énumération a..z bloquée ; temps de réponse identique pour « existe masqué » et « n'existe pas ».
- **Performance** : parent de trois enfants, professeur de 18 classes, CPE d'un lycée de 1 500 élèves (labo) : p95 du §14.4.
- **Accessibilité** : axe-core sur chaque page, parcours clavier, lecteur d'écran (NVDA, VoiceOver), 320 px, 200 %.
- **Recette par personas** (D-021 étape 4) : les 16 personas de `PERSONAS_29.md` sur le logiciel codé, dont les attaques de P15.

---

## 26. Réglages signalés (D-023)

| # | Réglage | Défaut | Candidat à l'allègement ? |
|---|---|---|---|
| S-2900 | nombre de tâches en tête de l'accueil | 5 | oui (fixe) |
| S-2901 | durée d'une prise en charge avant relâche | 2 jours ouvrés | non |
| S-2902 | seuil d'écart du rapprochement qui alerte | 1 % ou 20 tâches | oui (technique) |
| S-2903 | compteur de l'autre casquette | affiché | non |
| S-2904 | expiration des rappels personnels | fin d'année scolaire | oui (fixe) |
| S-2905 | « pouls » du personnel sur l'accueil (NXP-05) | non repris | — (décision Florian si souhaité) |
| S-2906 | échéances indicatives pendant les vacances | reportées au premier jour ouvré | non |
| S-2907 | plafond quotidien de recherches avant ralentissement | 400 (personnel), 1 500 (secrétariat à la rentrée) | non |
| S-2908 | mode discret par défaut sur le poste d'accueil | activé | non |
| S-2909 | conservation des tâches closes / du journal | 30 jours / 12 mois | **DPO** |
| S-2910 | délai avant « appel en retard » | 15 min après la fin de séance | paramètre du 12 |
| S-2911 | plage calme d'affichage pour le privé | identique au public (20 h–7 h, week-end) | non |

---

## 27. Décisions

### 27.1 Appliquées par défaut (D-031)

1. **Index des tâches** revérifié et reconstructible, plutôt qu'agrégation à la volée ou copie de données (§8.1).
2. **Frontière avis / tâche** et format commun des événements de tâche (A3) ; `facturation.tache_*` deviennent des natures.
3. Les **files** restent chez leurs socles ; le 29 compte et renvoie (A4).
4. **Socle N1** « Demandes et flux de travail » à créer ; rien du guichet dans le 29 (A5).
5. **Un seul contrat de tuile** dans le kit, partagé avec la fiche 360° du socle 2 (A7).
6. **Port neutre `ou_est`** du socle 12 ; « à l'infirmerie » réservé aux habilités (A8).
7. **Réunion multi-établissements sur l'appareil seulement** ; VOY-135 du 42 requalifié (A10).
8. **Neutralité totale entre parents** (A11, R-PO7) ; **une casquette par session** (A12).
9. **File de repli obligatoire** et escalade des bloquantes (A13) ; **décès silencieux** (A14).
10. **Plage calme appliquée à l'affichage** des tâches et tuiles nouvelles pour les familles et élèves, public et privé (R-PO31).
11. **Recherche serveur bornée**, texte non conservé, anti-énumération (R-PO19).
12. **Démo de décembre = P0 + P1 sans index** ; l'index (P2) vient ensuite.
13. Accueil de Next.js **refait** (ébauche, D-035 précision 2), sa liste de signaux reprise comme natures et tuiles (§2.4) ; pouls du personnel non repris (S-2905).
14. Conformité RGAA **comme si ScolIA était assujetti** (§5.2).
15. Formulation par modèle de langage du résumé du matin **désactivée par défaut dans le public** tant que l'exigence « anonymisé via le GAR » n'est pas analysée (§5.1).

### 27.2 Soumis à Florian (engagent l'extérieur, des coûts ou des faits qu'il connaît)

| # | Question | Recommandation |
|---|---|---|
| Q-PO1 | Faire lire l'**arrêté du 7/09/2026** et ses annexes (déconnexion, accessibilité, DNMA, IA) par un juriste avant la commercialisation au public | oui, avant D-029 |
| Q-PO2 | **Élève majeur** : peut-il s'opposer à l'information de ses parents ? (aucun texte trouvé) | question au juriste ou à la DAJ ; d'ici là, règle du socle 4 |
| Q-PO3 | Créer le **socle N1** (demandes) avant février 2027, la bascule des demandes Next.js étant prévue au printemps (Q-T0-04) | oui |
| Q-PO4 | **Année de doublon** : quelles natures désactiver ou passer en « information » parce que Charlemagne fait foi (absences ? notes ?) | absences et justifications : actives (usage réel de l'accueil, Q-T0-07) ; notes : information seulement |
| Q-PO5 | **DPO** : base légale dans le privé sous contrat, durées (S-2909), mention « à l'infirmerie » au registre des données de santé indirectes | à valider avec le DPO au printemps 2027 |
| Q-PO6 | **Langues** prioritaires du portail famille pour l'établissement pilote | fait de terrain : à fournir |
| Q-PO7 | Activer un jour la **formulation IA** du résumé du matin dans le privé (coût du fournisseur) | après l'étude juridique de D-032 |
| Q-PO8 | Statut du **privé sous contrat** au regard de l'article 47 et de l'acte européen ; sanctions de l'Arcom | juriste |
| Q-PO9 | **Audit RGAA externe** de l'accueil avant le pilote (coût : prestataire) | oui, avant l'année de doublon |
| Q-PO10 | Faut-il un **« pouls » du personnel** sur l'accueil (NXP-05, aujourd'hui en usage dans Next.js) | non sur l'accueil ; au socle 6 si souhaité |

---

## 28. Faits, hypothèses, contradictions

**Faits [F]** : code lu sur `origin/ent-mc0hoe` (`7467f2d4`) aux chemins cités ; Next.js lu sur `origin/main` par `git show` (fichiers cités au §2.2) ; plages de numéros et de migrations vérifiées libres sur les 124 branches distantes le 2026-11-11. Rien compilé ni exécuté.

**Hypothèses [H]** : usages du terrain (§3, §4) ; comparaison avec le marché ; volumes du §14.4.

**Contradictions signalées** (outre A1 à A14 du §6.2) :
- **SDET et PRS-29** : le SDET décrit des droits qui sont « la combinaison des rôles » d'un usager à plusieurs profils (exemple : enseignante dans X et parent dans Y, **deux établissements**) ; PRS-29 interdit l'union de droits **dans un établissement**. Pas de contradiction de fond (le SDET raisonne par contexte d'établissement), mais la phrase du SDET doit être lue avec le juriste si un client public l'invoque (Q-PO1).
- **Plage calme** : la messagerie (16) et les notifications (8) la posaient comme bonne pratique ; l'arrêté du 7/09/2026 en fait une **obligation** dans le public (§5.1) et l'étend à « toute information de vie scolaire » : les socles 10 (changements d'EDT), 20 (devoirs), 21 (notes) et 24 (factures) doivent vérifier que leurs publications respectent la plage (signalé à leurs conceptions).
- **Résumé du jour à 18 h** (NTF-09) : compatible (avant 20 h) ; **résumé des personnels à 7 h 30** : compatible.
- **`SOCLES.md` §29** place le 29 « après 12 à 26 » alors que D-034 veut son accueil à la démo de décembre : résolu par P0 + P1 sans index, l'index et l'à-faire complet suivant les socles.

---

## 29. Compléments de la passe personas (ajout personas)

Source : `docs/socles/PERSONAS_29.md` (16 personas, 48 manques M-2900 à M-2947, quatre bloquants A à D appliqués par défaut, D-031). Rien de ce qui précède n'est supprimé ; quand un ajout modifie une règle, la règle d'origine reste écrite et l'ajout prévaut.

### 29.1 Règles ajoutées (R-PO33 à R-PO38) et précisions

| # | Règle (ajout personas) |
|---|---|
| **R-PO33** | **Cache hors ligne d'un responsable** (bloquant A) : jamais l'emploi du temps au-delà de la journée en cours ; expiration après 24 h sans contact serveur ; **effacement** au premier contact après `lien.suspendu`, retrait d'adhésion ou révocation de session (purge silencieuse envoyée par le socle 11) ; à la saisie d'une suspension, le socle 4 propose au secrétariat la révocation de toutes les sessions du responsable. Condition d'entrée du lot P8. Modifie §14.3. |
| **R-PO34** | **Compteurs multi-établissements** (bloquant B) : jamais de CORS entre sous-domaines ; liste signée et courte des adhésions délivrée par le guichet central `connexion.scolia.fr` (socle 9), présentée par le navigateur à chaque établissement, qui ne répond que s'il y figure ; aucun contenu fourni par un établissement servi depuis un sous-domaine qui porte des sessions. À défaut, P9 = sélecteur seul. Modifie §11.3 et le lot P9. |
| **R-PO35** | **Remise en mode discret** (bloquant C) : l'alerte de restriction de remise reste visible sous forme neutre (« Attention : restriction de remise — vérifier avant toute sortie »), le détail après un geste de l'agent ; jamais le motif ni le nom de la personne interdite en clair sur l'écran d'accueil ; source `qui_peut_recuperer` (4, ADM-44). Complète §14.5. |
| **R-PO36** | **P1 sur le contrat de tuile** (bloquant D) : dès la démo, chaque tuile est déclarée dans la brique de son socle (`rendre_tuile`) ; le composeur du 29 ne connaît aucune tuile par son nom. Modifie le lot P1. |
| **R-PO37** | **Ajustements préparés de la plage calme** : liste proposée au chef d'établissement (S-2913 : cours du lendemain avant 10 h annulé ou déplacé, fermeture, alerte), visibles **en consultation** dès leur publication, **sans notification** pendant la plage ; liste publiée aux familles ; lecture juriste (Q-PO1). Précise R-PO31. |
| **R-PO38** | **Neutralité temporelle entre parents** : une tâche close par un responsable d'**un autre foyer** reste affichée chez l'autre jusqu'au prochain point de rafraîchissement neutre (S-2914 : 7 h ou session suivante, au plus 24 h) ; toute action répond « Cette démarche est déjà traitée », même code, même texte, même temps de réponse qu'une clôture par l'établissement. Précise R-PO7. |

Précisions (ajout personas) :
- **Supervision** (`afaire.superviser`, POR-161) : aucun classement de personnes ; une file de moins de 3 membres n'affiche que « en retard : oui / non » ; information préalable des personnels dans la charte du socle 6 ; avis du DPO (Q-PO11).
- **Porte « Parler à un adulte »** (élève, vers le 36) : ni compteur, ni état personnel, ni « fait récemment », ni mémoire locale de la palette, ni lecture par l'IA.
- **Aucune action à l'ouverture d'une adresse** : une adresse (lien de notification, message, QR) ouvre la tâche ; l'action exige un geste dans la page et son aperçu (comme les pages de récupération du socle 1). Complète R-PO1.
- **`vue_le` non probant** : remplacé par un booléen « nouveau / vu » effacé à la clôture (§29.4) ; aucune route ni export ne sert de preuve de lecture ; la preuve de notification relève des socles 7 et 8 (D-027 D).
- **Rappels liés à un élève** inclus dans l'extraction du droit d'accès de cet élève (socle 27, procédure du DPO) ; l'avertissement de création le dit. Complète §8.12 et R-PO29.
- **Plage calme et pastille** : R-PO31 s'applique aussi à `compteur_a_faire` (la pastille d'une famille n'augmente pas pendant la plage).
- **Contexte des tuiles** : un `eleve_id` fourni par le client n'est admis que pour un responsable, revérifié par `liens_de` ; pour un élève, le contexte est toujours la session.
- **Budget de poids** (§14.4) : accueil lisible sans WebAssembly (rendu serveur), premier affichage < 100 Ko transférés, îlots hydratés pour les cartes d'action seulement.
- **Tuile « Appels non faits »** et **« Absences inquiétantes »** : déclarées et calculées par le socle 12 (exclusions : séances annulées, en sortie, en examen, appel fait par le remplaçant) ; le 29 n'y ajoute aucune règle.
- **Modèle de rôle AESH** ajouté aux modèles d'accueil par défaut (§9.4).

### 29.2 Actions ajoutées (POR-252 à POR-266)

| # | Action | Qui | Remarque |
|---|---|---|---|
| POR-252 | Voir la première et la dernière heure de la journée de chaque enfant | famille | 10 |
| POR-253 | Filtrer l'à-faire sur « mes semaines de garde » | responsable séparé | calendrier de garde du 4, rien de l'autre |
| POR-254 | Préparer une tâche d'une file à deux niveaux | membre « préparer » | état `en_attente` « préparé par X » |
| POR-255 | Décider une tâche préparée | membre « décider » | commande du propriétaire |
| POR-256 | Ouvrir un poste partagé en mode discret et fermer la session en fin de séance | système | S-2912 |
| POR-257 | Signaler soi-même un conflit horaire entre deux établissements à l'un d'eux | professeur partagé | message pré-rempli envoyé par la personne |
| POR-258 | Retrouver un élève depuis le nom du responsable qui appelle | accueil | port de recherche du 4, résultat minimal |
| POR-259 | Débloquer soi-même sa recherche une fois par jour, avec motif | personnel | journalisé ; ensuite `recherche.debloquer` |
| POR-260 | Indexer chez un délégataire permanent par nature | système | délégation du 6 (M-2925) |
| POR-261 | Informer d'un « choix important » le parent sans exercice dont le lien le prévoit | système | catégorie déclarée par 3, 13, 23, 35, 37, 42 ; accusé facultatif |
| POR-262 | Refuser un contexte d'élève fourni par le client | système | test de séparation généré |
| POR-263 | Charger le scénario de démonstration « lundi 8 h » pour tous les profils | administrateur du labo | socle 28 |
| POR-264 | « Voir comme » un profil sur données fictives dès P1 | administrateur | POR-223 avancé |
| POR-265 | Geler la pastille d'une famille pendant la plage calme | système | R-PO31 |
| POR-266 | Purger le cache hors ligne d'un responsable à la suspension | système, socle 11 | R-PO33 |

**Total avec la passe personas : 167 actions** (POR-100 à POR-266).

### 29.3 Cas limites ajoutés (CPO-73 à CPO-86)

| # | Cas | Comportement attendu |
|---|---|---|
| CPO-73 | Cours de 8 h annulé à 21 h | visible en consultation (ajustement préparé, R-PO37), sans notification avant 7 h |
| CPO-74 | Le second parent signe à 23 h 02 | la tâche reste affichée chez le premier jusqu'à 7 h ; action → « déjà traitée » (R-PO38) |
| CPO-75 | Parents du **même** foyer | disparition immédiate (pas de canal à protéger) |
| CPO-76 | Lien suspendu, appli hors ligne | cache expiré à 24 h ; purge au premier contact (R-PO33) |
| CPO-77 | Visiteur face à l'écran de l'accueil pendant une remise sous restriction | alerte neutre, détail sur geste (R-PO35) |
| CPO-78 | Tableau interactif de la salle 12 resté connecté | mode discret, session fermée en fin de séance (POR-256) |
| CPO-79 | Deux sœurs sur un même téléphone | changement de personne avec passkey ; jamais les deux accueils ensemble |
| CPO-80 | Élève qui ouvre « Parler à un adulte » puis rend son téléphone à un parent | aucune trace sur l'accueil, l'historique ou la palette |
| CPO-81 | Secrétariat le jour de la rentrée, 1 200 recherches | plafond ×3 de période, déblocage personnel une fois (POR-259) |
| CPO-82 | File « justificatifs » préparée par un AED, décidée par la CPE | deux niveaux, droits distincts |
| CPO-83 | Une seule secrétaire dans la file « pièces » | supervision « en retard : oui / non » seulement |
| CPO-84 | Demande d'accès de Tom alors qu'un professeur a un rappel « Tom — appeler la mère » | rappel lié inclus dans l'extraction (DPO) |
| CPO-85 | Page d'un établissement qui tente d'appeler `/portail/compteur` d'un autre sous-domaine | refus (pas de CORS) ; aucune réponse exploitable (R-PO34) |
| CPO-86 | Parent sans exercice de l'autorité, décision de redoublement | information « choix important » s'il est prévu par le lien ; aucune tâche de décision |

### 29.4 Compléments de modèle (ajout personas)

- `ent_afaire_etat_personnel.vue_le` (§22.1) est **remplacé** par `nouveau BOOLEAN NOT NULL DEFAULT true`, effacé avec la ligne à la clôture de la tâche ; nouvelle table **`ent_portail_visite`** (`etablissement_id`, `compte_id`, `casquette_code`, `derniere_visite_le`, `modifie_le` ; clé `(etablissement_id, compte_id, casquette_code)` ; lecture par le seul compte) pour « ce qui a changé depuis ma dernière visite ». Migration 2581 (inchangée en numéro).
- **`ent_portail_poste_partage`** (`id`, `etablissement_id`, `libelle`, `identifiant_poste` TEXT (jeton d'appareil émis par l'établissement, haché), `salle_id` NULL (FK composite, 5), `actif`, `cree_le`, `modifie_le`) — migration **2589** (prise sur la réserve), lot P10.
- **Files à deux niveaux** : déclarées dans le code (niveau `preparer` / `decider`, droits) ; `ent_afaire_prise` reçoit `niveau TEXT NOT NULL DEFAULT 'decider' CHECK IN ('preparer','decider')` et `ent_afaire` l'état `en_attente` avec `motif_code = 'prepare'` ; migration 2581.
- `ent_afaire_nature_reglage.ajustement_plage_calme BOOLEAN NOT NULL DEFAULT false` (R-PO37, choisi par le chef dans la liste S-2913) ; migration 2582.
- Aucune colonne JSON ni tableau ajoutée.

### 29.5 Lots modifiés (ajout personas)

- **P1** livre en plus : le **contrat de tuile** et le **composeur** (R-PO36), la tuile accueil avec l'**alerte de remise neutre** (R-PO35), le scénario de démonstration « lundi 8 h » (POR-263) et « voir comme » fictif (POR-264).
- **P3** : neutralité temporelle (R-PO38), information des « choix importants » (POR-261), pastille gelée pendant la plage (POR-265).
- **P4** : files à deux niveaux (POR-254, POR-255), supervision sans classement.
- **P5** : plafonds par rôle et période, déblocage personnel (POR-259), recherche depuis le responsable (POR-258).
- **P7** : ajustements préparés de la plage calme (R-PO37).
- **P8** : condition d'entrée **R-PO33** (purge et expiration), changement de personne sur l'appareil.
- **P9** : réécrit selon R-PO34 (guichet central, aucune CORS) ; dépend du socle 9.
- **P10** : postes partagés (2589, POR-256).

### 29.6 Questions et réglages ajoutés

Q-PO11 (DPO : supervision, journal nominatif, AIPD), Q-PO12 (juriste : liste des choix importants), Q-PO13 (postes de classe partagés au pilote), Q-PO14 (profils et scénario de la démo de décembre) : `PERSONAS_29.md` §8.2. Réglages S-2912 (postes partagés), S-2913 (ajustements préparés), S-2914 (point de rafraîchissement neutre) : `PERSONAS_29.md` §8 bis.
