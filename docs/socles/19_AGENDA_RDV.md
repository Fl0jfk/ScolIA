# Socle 19 — Agenda, réunions et rendez-vous

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§31.2). Aucun code écrit ni modifié. Aucun accès à une base, à un secret ou à un environnement de production. Le code Next.js a été **lu seulement**, sur une copie de `origin/main` extraite par `git archive` dans le dossier temporaire de la session (rien exécuté, rien construit, rien modifié).
**Branche :** `ent-socle-19-conception`, partie de `origin/ent-mc0hoe` (`7467f2d`).
**Passe personas :** `docs/socles/PERSONAS_19.md` ; ses ajouts sont faits **ici**, marqués **« ajout personas »**, et regroupés au §33, qui **l'emporte** en cas de contradiction. Rien n'a été supprimé.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (D-013, D-014, D-017, D-020, D-021, D-022, D-023, D-024, D-026, D-028, D-031, **D-034**, **D-035**, **D-036**, D-037 à D-041) ; `docs/SOCLES.md` (socles 19, 3, 5, 6, 8, 9, 10, 13, 15, 16, 23, 33, 34, 36, 37, 42, 44, 46) ; `docs/MATRICE_CHARLEMAGNE.md` (COM-14 à COM-16, INS-04, INS-05, EDT-05, VOY-12, G29–G30) ; `docs/SOCLE2_DOMAINES.md` §D21 (AGD-01 à AGD-05, repris et étendus ici) ; `docs/TRANSITION_T0_INVENTAIRE.md` (M02, M03, M16, M17, §4.2) ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; conceptions `03_INSCRIPTIONS.md` (§10.3, INS-24, I10), `04_ADMINISTRATIF.md` (parents séparés, interdictions, majorité), `05_STRUCTURE.md` (salles, lieux, disponibilités, calendrier), `06_PERSONNELS.md` (équipes, missions, indisponibilités), `08_NOTIFICATIONS.md` (natures `rendezvous.*`), `09_INTEROP.md` (FX-85, lot X15 : abonnements iCal), `10_EMPLOI_DU_TEMPS.md` (§6.2 ports `plages_libres`, `occupe`), `16_MESSAGERIE.md` (cartes d'action, mot « demande de rendez-vous »), `21_EVALUATIONS.md` (M-600), `34_SANTE.md` (SAN-253, SAN-272), `42_SORTIES_VOYAGES.md` (VOY-126), `IA_COEUR.md` §5. Modèles de forme : `10_EMPLOI_DU_TEMPS.md`, `42_SORTIES_VOYAGES.md`.

**Conventions.**
- **[F]** fait vérifié dans le code (Rust : chemins depuis `ent-rust/` ; Next.js : chemins relatifs à la racine de `origin/main`).
- **[V]** règle lue sur une source officielle (URL et date au §6) ; **[V-copie]** lue sur une copie intégrale ; **[NV]** non vérifiée.
- **[H]** hypothèse (pratique du terrain) ; **[R]** recommandation, non décidée.

**Numérotation propre au socle** (préfixée pour éviter les collisions avec les sessions qui travaillent en parallèle le 2026-11-11) :

| Élément | Numéros | Remarque |
|---|---|---|
| Actions existantes de Next.js | **NX-01**… (propres à ce document) | NX du socle 42 non concernés |
| Défauts de Next.js | **ND-01**… | |
| Défauts de l'existant Rust | **RD-01**… | corrigés au lot 0 (§27) |
| Actions cibles | **AGD-100**… | AGD-01 à AGD-05 (`SOCLE2_DOMAINES.md` §D21) sont repris et renvoient ici |
| Règles | **R-AG1**… | |
| Cas limites | **CAG-01**… | |
| Questions | **Q-AG1**… | |
| Réglages signalés (D-023) | **S19-01**… | |
| Manques de la passe personas | **M19-001**… | |

**Plage de migrations imposée : 2380–2399** (inscrite dans `MIGRATIONS.md`, détail §28.2).

**Méthode.**
1. Lecture des décisions, de la carte, de la matrice, de l'inventaire T0 et des conceptions citées.
2. État des lieux du code Rust (migrations 046 et 192, `ent-db/src/rencontres.rs` lu en entier, `ent-api/src/communication_s28.rs`, `ent-api/src/ical.rs`, `ent-api/src/main.rs`, registre `ent-db/src/registre_donnees.rs`).
3. Trois inventaires indépendants en sous-agents : (A) Next.js réservation de salles (M02) et planning par domaines (M03) ; (B) Next.js rendez-vous d'inscription (M16), portes ouvertes (M17), invitations avec réponse et partenariats, et toute autre trace d'agenda ; (C) réglementaire sur sources officielles.
4. Aucune donnée réelle n'a été lue ni recopiée : noms et valeurs vus dans le code sont remplacés par « [donnée] ».

Sommaire :

| Partie | Sections |
|---|---|
| Synthèse | §0 En bref · §1 Thèse |
| Existant | §2 Rust et Next.js, défauts, à ne pas perdre |
| Métier | §3 Périmètre et frontières · §4 Acteurs · §5 L'année vue du terrain · §6 Réglementaire |
| Cœur | §7 Concepts · §8 L'agenda calculé et sa visibilité |
| Parcours | §9 Événements et réunions · §10 Rendez-vous individuels · §11 Rencontres parents-professeurs et campagnes · §12 Sessions collectives · §13 Disponibilités et créneau commun · §14 Réservation de lieux · §15 Familles · §16 iCal, mobile, visio |
| Règles et sécurité | §17 Cas limites · §18 IA · §19 Actions · §20 Règles · §21 Droits et minimisation |
| Technique | §22 Événements · §23 Notifications · §24 Écrans · §25 Modèle de données |
| Mise en œuvre | §26 Reprise Next.js · §27 Lot 0 · §28 Lots et migrations · §29 Tests et recette |
| Décisions | §30 Réglages · §31 Décisions appliquées et questions · §32 Faits, hypothèses, contradictions |
| Ajout personas | §33 |

---

## 0. En bref

1. **Un agenda scolaire n'est pas un calendrier partagé.** C'est la **réunion calculée** de ce que d'autres socles possèdent (séances du 10, calendrier et fermetures du 5, sorties du 42, conseils du 23, examens du 38, PFMP du 39, absences du 15) et de ce que le 19 possède en propre : **événements et réunions**, **rendez-vous**, **offres de créneaux** (rencontres parents-professeurs, entretiens, sessions collectives), **disponibilités** des personnels et **réservations de lieux**. On ne recopie jamais une séance ou une sortie dans une table d'agenda : on la lit par le port de son propriétaire, à la date, avec les droits du lecteur (R-AG1).
2. **Un seul moteur de créneaux** sert tout ce qui se réserve : rencontre parents-professeurs, rendez-vous avec le CPE ou la direction, entretien d'orientation, rendez-vous d'inscription (socle 3, qui le consomme), portes ouvertes (3), campagne de vaccination ou bilan de 6ᵉ (34), atelier à places limitées, salle. Capacité 1 = rendez-vous ; capacité N = session (R-AG2).
3. **La soirée parents-professeurs se planifie sans tableur** (promesse du `SOCLES.md`). Les enseignants à rencontrer sont **calculés** depuis les services (6, 10), les plages proposées depuis leurs disponibilités et leur EDT, les salles attribuées, et la famille reçoit un **parcours sans attente** pour tous ses enfants et tous leurs professeurs, avec liste d'attente, vagues prioritaires (familles que le conseil de classe souhaite voir) et suivi des retards le soir même (§11).
4. **Parents séparés traités dès le départ.** Deux responsables de **foyers distincts** peuvent avoir chacun leur rendez-vous ; dans un même foyer, un rendez-vous commun. Un responsable n'apprend jamais l'heure ni le lieu du rendez-vous de l'autre si une **interdiction**, une **adresse protégée** ou une mesure du socle 4 existe (R-AG12) : c'est la faille la plus grave d'un agenda scolaire (§15, CAG-01).
5. **Le motif d'un rendez-vous ne fuit jamais par l'agenda.** Un rendez-vous chez la psychologue, l'infirmière, l'assistante sociale, ou un entretien disciplinaire apparaît « occupé » à tous sauf aux participants ; son contenu reste dans le socle propriétaire (36, 34, 13). Le titre visible d'un autre que les participants est toujours calculé, jamais saisi (R-AG5).
6. **Les réunions et instances** (équipe pédagogique, équipe éducative, ESS, conseil pédagogique, réunion d'information des familles, conseil d'administration…) ont un moteur commun : convocation dans le délai réglementaire, ordre du jour, réponses, émargement, compte rendu déposé au socle 7 et diffusé par le socle 8. Le **contenu juridique** reste aux propriétaires : délibérations et votes au 46, avis du conseil de classe au 23, sanction au 13 (§9).
7. **Réservation de salles : reprise du module Next.js fiable (D-035).** Recommandation appliquée par défaut (D-031) : **le moteur de réservation de lieux est porté par le socle 19** (et non attendu au socle 44, étage J, trop tard pour la rentrée 2027) ; le 44 garde le matériel, les prêts, les clés et la maintenance, et réutilise le même moteur pour le matériel (§3, §14 ; contradiction écrite au §32).
8. **Existant Rust [F]** : rencontres parents-professeurs par classe (migration 192, `rencontres.rs`) et rendez-vous de la psychologue (046, socle 36). Le code est propre mais a **16 défauts** dont 4 urgents : historique effacé à l'annulation, réservation d'un créneau passé, second parent aveugle, rappels perdus quand le cloisonnement strict sera activé (**lot AG0**, §27).
9. **Souveraineté** : aucun agenda tiers par défaut (Google en option désactivée, Q-I12 déjà tranchée au socle 3) ; abonnement iCal par jeton révocable (porté par le socle 9, X15) avec **contenu minimisé** (titre neutre, pas de nom d'élève par défaut, R-AG25) ; visio par le socle 33.
10. **IA** : propose des plages, prépare une soirée, rédige une convocation ou un résumé de compte rendu à relire ; ne réserve, n'annule, ne convoque et n'envoie rien sans confirmation ; ne lit jamais un rendez-vous de santé, psychologique ou social (§18).
11. **Taille** : **227 actions** (AGD-100 à AGD-291, AGD-300 à AGD-312, AGD-320 à AGD-341, dont 22 venues de la passe personas et du réglementaire), **48 règles** (R-AG1 à R-AG48), **63 cas limites** ; **15 lots** (AG0 à AG12, plus AG4a et AG6b) ; parcours minimal de la démo de décembre : AG0 + AG1 + AG4a ; production rentrée 2027 (D-035) : AG0 à AG6 (salles, rendez-vous, rencontres, sessions).

---

## 1. Thèse : le temps de l'établissement est un bien commun rare, l'agenda le partage sans le divulguer

Dans un collège ou un lycée, presque tout ce qui ne relève pas du cours se joue sur des **plages communes rares** : la soirée parents-professeurs, la réunion de l'équipe de 3ᵉ, l'équipe éducative autour d'un élève, la salle polyvalente du jeudi, le rendez-vous avec le CPE avant le conseil de discipline. Aujourd'hui ces plages se trouvent **à la main** (tableur de la soirée, « qui est libre jeudi ? » en salle des professeurs, cahier de réservation de la salle, double prise de rendez-vous par téléphone et par l'ENT), et les erreurs ont un coût humain : famille qui attend 40 minutes dans un couloir, parent séparé qui découvre après coup qu'un rendez-vous a eu lieu sans lui, enseignant convoqué à une réunion pendant son cours, salle réservée deux fois le jour du conseil.

Trois principes en découlent :
1. **Calculer, ne pas recopier.** Le 19 ne possède que ce que personne d'autre ne possède (événements, rendez-vous, créneaux, disponibilités, réservations). Le reste est lu, à la date, par port.
2. **Partager l'occupation, pas le motif.** Pour trouver un créneau commun il faut savoir qui est libre, pas pourquoi les autres sont occupés. La vue « libre / occupé » est la seule vue croisée entre personnels (R-AG6).
3. **Un rendez-vous est un engagement entre personnes, tracé.** Il a des participants (personnes, D-020), un objet éventuel (un élève), un fil daté en ajout seul (demandé, proposé, confirmé, reporté, annulé, honoré, non venu), et des droits qui se recalculent quand le cœur bouge (changement de classe, fin d'un lien, majorité, interdiction nouvelle).

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust [F]

| Objet | Où | Ce qui existe | Appréciation |
|---|---|---|---|
| Rencontres parents-professeurs | `migrations/192_rencontres.sql`, `crates/ent-db/src/rencontres.rs` (895 lignes), `crates/ent-api/src/communication_s28.rs` (routes `/rencontres*`, `/famille/rencontres*`), test `tests_communication.rs::rencontres_parents_professeurs_de_bout_en_bout` | Rencontre **par classe** (`brouillon → ouverte → close`) ; créneaux consécutifs par enseignant ayant la classe en service (5–120 min, 1–40 créneaux) ; refus des chevauchements d'un même enseignant (verrou consultatif) ; ouverture qui notifie les responsables de la classe ; famille : rencontres ouvertes de ses enfants, créneaux des seuls enseignants qui ont **cet** enfant (RP1-27), réservation conditionnelle (une seule famille gagne), un rendez-vous par enseignant et par enfant (index unique), pas deux rendez-vous simultanés pour un responsable ; annulation par le responsable ou l'organisateur ; rappels à 24 h idempotents ; notifications au responsable et à l'enseignant | **Bonne base** (concurrence et minimisation réfléchies : la famille ne voit jamais qui a pris un autre créneau). À reprendre dans le moteur de créneaux (AG4), après les correctifs du lot AG0 |
| Rendez-vous de la psychologue | `migrations/046_psy_rdv.sql`, `crates/ent-db/src/psy_repo.rs`, `crates/domain-eleve/src/psy.rs`, routes `/psy/rdvs` | Planning de la psychologue (jour, heures, nature `individuel/famille/orientation/crise`, motif et note chiffrés, statut `planifie/honore/annule/manque`) ; lecture contrôlée par `psy.lire` et le lien parental | Appartient au **socle 36** ; le 19 n'en lit que l'occupation (« occupé ») pour la psychologue elle-même et ses créneaux ouverts à la réservation (§10.6) |
| Export iCal de l'EDT | `crates/ent-api/src/ical.rs`, routes `/eleves/{id}/edt.ics`, `/edt/mes-creneaux.ics` (`routes.rs:637-639`) | RFC 5545 correct (échappement, pliage à 75 octets, UID stable) ; **téléchargement derrière la session**, aucun abonnement | Le rendu est réutilisable ; l'abonnement par jeton est au socle 9 (X15) |
| Droit | `ent-auth/src/actions.rs` : `RencontresGerer` (`rencontres.gerer`), direction par défaut | Un seul droit, pour tout | À éclater (§21.1) |
| Cloisonnement | `480_msg_m0a_fuites.sql` : RLS **permissive** sur `ent_rencontre`, `ent_creneau_rdv` ; registre `registre_donnees.rs:290,531` (catégorie « emploi_du_temps_structure », données personnelles, neutralisation `N_RDV`) | Présent mais permissif | Strict au lot AG1 |
| Agenda de l'établissement, réunions, réservations de salles, disponibilités | — | **Rien** | À construire |

**Défauts de l'existant Rust** (RD-01 à RD-16, détail et correctifs au §27) : historique effacé à l'annulation ; créneau passé réservable et annulable ; second parent aveugle et sans rendez-vous possible ; erreur 500 quand deux parents réservent en même temps ; lien rompu non revérifié ; rappels lus hors contexte d'établissement ; heure de Paris en dur ; SMS envoyés pour chaque confirmation (coût non décidé, Q-N22) ; annulation sans motif ni proposition ; aucune réaction aux changements du cœur ; enseignant sans aucune commande sur ses propres créneaux ; clés étrangères simples et `ON DELETE CASCADE` qui efface l'historique ; agrégat d'événement « annonce » ; rencontre limitée à une classe ; aucun contrôle contre l'EDT ; un seul droit.

### 2.2 Next.js : ce que font les modules d'agenda [F]

*(Inventaires des sous-agents A et B, lecture seule de `origin/main` du 2026-11-11 ; les numéros NX ont été réordonnés par module ; chemins relatifs à la racine Next.js.)*

Maturité (`TRANSITION_T0_INVENTAIRE.md` §3.0, D-035 précision 2) : **M02 réservation de salles = fiable** (Florian) → on reprend ses actions métier ; **M03 planning par domaines = partiel** → refait de zéro, en gardant ses bonnes idées ; **M16 rendez-vous d'inscription = partiel à fiable [H]** → actions reprises par le socle 3 sur le moteur du 19 ; **M17 portes ouvertes = partiel [H]** → idem ; invitations et partenariats : non classés, traités comme partiels.

#### 2.2.1 M02 — Réservation de salles (`app/api/reservation-rooms/*`, page `app/(admin)/prof-room/page.tsx`, `db/schema-reservation-rooms.ts`)

| NX | Action | Détail [F] | Repris en |
|---|---|---|---|
| NX-01 | Consulter le catalogue | toute personne authentifiée ; seules les salles `bookable` apparaissent au planning, compteur des « salles de classe » non réservables | AGD-231 |
| NX-02 | Gérer le catalogue | `PUT rooms` **remplace tout** (upsert puis suppression des absentes) ; nom, bâtiment, `kind` (`facility` / `classroom`), `bookable`, ordre ; identifiant = slug du nom | socle 5 (STR-07), règles du lieu AGD-242 |
| NX-03 | Créer les salles de classe depuis l'EDT | salles citées par les EDT créées `classroom` non réservables, jamais rétrogradées (`app/lib/rh/planning-rooms-sync.ts`) | socle 5 (référentiel) ; « réservable » par défaut selon le type (S19-11) |
| NX-04 | Planning semaine (bureau) et jour (mobile) | grille lundi–vendredi, 10 heures de 8 h 30 à 18 h 30 en dur, navigation ±7 j / ±1 j | AGD-232, AGD-280 |
| NX-05 | Occupation par l'EDT dans la grille | cours A/B et remplacements grisés, non cliquables (matière, classe, professeur, badge A/B/R) ; badge « semaine A/B » (`planning-room-occupancy.ts`) | AGD-232 (port `edt::occupe`) |
| NX-06 | Réserver | salle, date, une ou plusieurs heures, matière (couleur), niveau → classe, commentaire ; refus si non réservable ; chevauchement contrôlé contre les réservations et l'EDT | AGD-233 |
| NX-07 | Récurrence | chaque semaine ou toutes les deux semaines jusqu'à une date, `groupId` commun | AGD-234 |
| NX-08 | Horizon | 56 j par défaut, exemption de l'administrateur du module ; bornes min/max dans le formulaire | règle du lieu (AGD-242) |
| NX-09 | Message de conflit explicite | « déjà pris par [donnée] — matière » ou « occupé par l'EDT », cinq premiers conflits, 409 si rien créé | AGD-233 (conflit lisible, R-AG4) |
| NX-10 | Réserver pour un collègue | administrateurs listés du module ; bénéficiaire pris dans l'annuaire ; trois identités gardées (bénéficiaire, auteur, drapeau « pour autrui ») | AGD-233 (`pour_personne_id`), CAG-60 |
| NX-11 | Modifier une occurrence ou la série | contrôles refaits **seulement** pour les créneaux déplacés ; réattribution par l'administrateur | AGD-237 |
| NX-12 | Annuler (douce) | statut `CANCELLED`, date, auteur **en texte**, motif ; occurrence ou série | AGD-237, AGD-238 |
| NX-13 | E-mails | confirmation (liste des dates) et annulation (motif, auteur, nombre de créneaux) ; rien à la modification ; échec d'envoi non bloquant | notifications §23 |
| NX-14 | Copier / coller un créneau | menu contextuel ; refus de coller sur une cellule EDT | AGD-233 (écran) |
| NX-15 | « Mes 5 prochaines réservations » | cliquables | AGD-173 (écran) |
| NX-16 | Lien profond `?new=1` | formulaire ouvert sur la date du jour | écran |
| NX-17 | Paramétrage | couleurs par matière, classes par pôle, horizon, administrateurs ; heures de début et fin présentes mais non éditables | AGD-242 |
| NX-18 | Assistant IA | lister, vérifier, réserver **avec récapitulatif et confirmation humaine** (`brain-ai/tools/handlers/rooms.ts`) | §18 (AGD-240 N1) |
| NX-19 | Signal du tableau de bord « ma salle » | `dashboard-signals*` | socle 29 |

Stockage : tables typées `reservation_room` et `reservation_room_booking` (début et fin **en texte local sans fuseau**, `room_id` sans clé étrangère, aucune contrainte contre le chevauchement), configuration dans le stockage documentaire Postgres (`settings/modules/prof-room.json`), ancienne collection clé-valeur migrée paresseusement. Absent : iCal, impression, validation d'une demande, liste d'attente, vacances et jours fériés.

#### 2.2.2 M03 — Planning par domaines (« Enseignements transversaux », `app/api/domain-planning/*`)

Ce n'est **pas** un agenda : c'est le **suivi d'un programme éducatif obligatoire par classe**. Par défaut deux domaines, « EVARS collège » et « EVARS lycée » (éducation à la vie affective et relationnelle, et à la sexualité) ; chaque domaine a des **séances types** par niveau (numéro 1 à 3, thème, intervenant attendu, contrainte : professeurs de SVT, association extérieure, PsyEN ou infirmière, libre) ; un intervenant **se positionne** sur une séance pour une classe, avec une idée de séance et une date prévue ; la responsable du domaine **valide**, demande des modifications ou refuse.

| NX | Action | Détail [F] | Repris en |
|---|---|---|---|
| NX-20 | Choisir un domaine, compter ses séances | | AGD-300 (§12.2) |
| NX-21 | Grille niveau × séance × classe | classes déduites des pôles par préfixe | AGD-301 |
| NX-22 | Se positionner sur (séance, classe) | matière imposée par la contrainte, idée obligatoire sauf association, date facultative, une seule par (séance, classe) | AGD-302 |
| NX-23 | Positionner un collègue | auto-validé, auteur tracé, e-mail facultatif | AGD-303 |
| NX-24 | Valider / demander des modifications / refuser | commentaire obligatoire pour les deux derniers ; panneau « à valider » | AGD-304 |
| NX-25 | Réviser son idée | repasse « en attente » ; impossible après validation | AGD-305 |
| NX-26 | Fixer la date de la séance | sans toucher au statut | AGD-306 |
| NX-27 | Retirer un positionnement | **suppression définitive** | AGD-307 (retrait tracé) |
| NX-28 | Mes positionnements | statut et commentaire | AGD-308 |
| NX-29 | PDF d'une séance | classes, intervenants, statuts, dates | AGD-309 |
| NX-30 | Paramétrer domaines, responsables, séances, couleurs | | AGD-310 |
| NX-31 | Bandeau « domaine sans responsable » | | AGD-310 |
| NX-32 | Anciennes réservations horaires de coordinateur | **obsolètes**, routes sans écran | non repris |

Stockage : un **document JSON unique par collection** (`domain-planning__*`) réécrit en entier à chaque action ; tables typées `domain_planning_*` jamais utilisées et de forme incompatible.

#### 2.2.3 M16 — Rendez-vous d'inscription (`app/lib/rdv-inscription-*.ts`, `app/api/rdv-inscription/**`, pages `app/(public-rentree)/rdv-inscription/*`)

| NX | Action | Détail [F] | Repris en |
|---|---|---|---|
| NX-40 | Paramétrer le module et ses « directions » (école, collège, lycée) | slug, intitulé, lieu, consentement, horizon 60 j, délai de libération 48 h, e-mail de notification | offre de nature `inscription` (AGD-218), socle 3 |
| NX-41 | Lier un compte Google (OAuth) | jeton de rafraîchissement conservé | **non repris** (Q-I12 : créneaux dans ScolIA, Google en option désactivée) |
| NX-42 | Créneaux = événements de l'agenda Google dont le titre contient un motif | durée et capacité fixées par Google (un événement = une famille) | créneaux natifs (AGD-195) |
| NX-43 | Garde par e-mail (double opt-in) | jeton 2 h, 60 s entre deux envois, limite de débit, cookie HMAC lié à l'établissement | socle 3 (vérification), AGD-286 |
| NX-44 | Retrouver l'élève préinscrit | par contact, sinon par identité (date de naissance + nom **ou** prénom) | socle 3, **corrigé** (ND-20) |
| NX-45 | Réserver | pot de miel, consentement, saisie « CONFIRME », informations (régime, PAP, établissement d'origine), un RDV actif par élève et par direction (remplace l'ancien), réservation provisoire puis finale, verrou optimiste (etag) | AGD-219, AGD-160 ; PAP **non** demandé avant l'admission (Q-I7) |
| NX-46 | Finalisation | événement Google renommé ; récapitulatif au parent avec ICS ; notification à la direction ; report régime / origine / PAP dans la fiche | AGD-156 ; report au socle 3 |
| NX-47 | Dépôt du PAP | URL pré-signée 1 h | **non repris** (Q-I7) |
| NX-48 | Reconfirmation à J-7 | « toujours OK ? » / « annuler » ; sans réponse = maintien | AGD-220 |
| NX-49 | Libérer ou retirer le créneau après un changement | libéré si au moins 48 h avant, sinon retiré et marqué « annulé » pour la direction | règle `delai_annulation_heures` (§10.5) |
| NX-50 | Annulation par l'administration | mail avec les RDV encore actifs | AGD-163 |
| NX-51 | Demande de « rechoix » par l'administration | motif prédéfini, lien valable 14 j | AGD-161 (hôte) |
| NX-52 | Changer de créneau (administration), y compris hors créneaux | | AGD-161, AGD-159 |
| NX-53 | Confirmer à la main une réservation en attente | | AGD-156 |
| NX-54 | Diagnostic et réparation des créneaux masqués | propre à Google | non repris (inutile sans Google) |

#### 2.2.4 M17 — Portes ouvertes (`app/lib/portes-ouvertes-*.ts`, `app/api/portes-ouvertes/*`, `app/api/accueil/portes-ouvertes/*`)

| NX | Action | Détail [F] | Repris en |
|---|---|---|---|
| NX-60 | Configurer | titre, adresse, lien de préinscription, délai du message de suivi (60 min) | session de nature `portes_ouvertes` (socle 3) |
| NX-61 | Créneaux par cycle, places maximales, génération en série | | AGD-221 |
| NX-62 | Inscription publique | pot de miel, doublon par empreinte, « complet », ICS | AGD-222, AGD-286 |
| NX-63 | Saisie, modification, suppression par l'accueil, tracées | session passée non supprimable | AGD-222 (accueil), R-AG3 |
| NX-64 | Pointage du jour J puis message de suivi différé | | AGD-225, AGD-226 |
| NX-65 | Équipe par créneau (ambassadeurs élèves, enseignants) et planning PDF | | AGD-311 (§12) |
| NX-66 | Renvoi de la confirmation | | AGD-287 |
| NX-67 | Places restantes affichées | | AGD-222 |

#### 2.2.5 Invitations avec réponse et partenariats

| NX | Action | Détail [F] | Repris en |
|---|---|---|---|
| NX-70 | Pages d'invitation (remise des diplômes) | thème, lieu, date limite de réponse, plafond total (200) et par élève (4) | événement « manifestation » avec inscription (AGD-312) |
| NX-71 | Liste des élèves autorisés (import Excel) | | public calculé depuis le cœur (2, 23, 38) |
| NX-72 | Retrouver sa réponse (2 critères sur 3) | | **corrigé** (ND-30) : lien nominatif |
| NX-73 | Répondre oui / non, nombre de personnes, situation actuelle | une nouvelle réponse remplace l'ancienne ; capacité en transaction | AGD-128, AGD-312 |
| NX-74 | Tableau de bord, export, doublons suspects | | AGD-131 |
| NX-80 | Catalogue public d'offres de partenaires et événements liés | | socle 17 ou N3 (présence publique), pas le 19 |
| NX-81 | Coupon d'inscription avec signature | | socle 18 |
| NX-82 | Événements des partenaires (réunions d'information) | ni inscription, ni ICS | événement du 19 (AGD-114) |

#### 2.2.6 Autres traces d'agenda dans Next.js [F]
- `app/lib/calendar-ics.ts` : ICS générique (PUBLISH / CANCEL, SEQUENCE, rappel, heure locale), réutilisé par M16, M17, invitations, voyages ; **à égaler** dans le rendu iCal (R-AG25).
- Installation à l'internat par créneaux (`app/lib/internat-installation-slots.ts`, JSON sur S3) : consommateur naturel du moteur de créneaux (socle 41) [R].
- Voyages : ICS parents (dépôt, séjour, retour), ancien champ « réunion des parents » : repris par VOY-126 (socle 42) sur le 19.
- Entretiens RH (à planifier, planifié, réalisé) : socle 6 sur le moteur de rendez-vous [R].
- Visio : appels WebRTC dans la messagerie, aucun service externe : socle 33.
- Calendrier de vie scolaire, vacances et jours fériés (`fr-school-holidays.ts`, `fr-public-holidays.ts`) : socle 5.

### 2.3 Défauts trouvés dans Next.js (à ne pas reproduire ; Next.js reste intouchable, signalés à Florian)

| N° | Module | Défaut [F] | Corrigé par |
|---|---|---|---|
| ND-01 | M02 | **Double réservation** possible en concurrence : vérification en mémoire puis écriture, aucune contrainte en base | contrainte d'exclusion (R-AG18) |
| ND-02 | M02 | Succès partiel silencieux d'une série (201 avec un simple compte, conflits perdus) | rapport par occurrence, confirmation (AGD-234) |
| ND-03 | M02 | Heure non validée (concaténation possible), dates passées et week-ends acceptés, heures d'ouverture jamais appliquées | R-AG15, règles du lieu |
| ND-04 | M02 | Dates en texte sans fuseau, horizon au fuseau du serveur, « mes prochaines » décalées de 1 à 2 h, e-mails au fuseau du serveur | R-AG11 |
| ND-05 | M02 | **Injection HTML** dans les e-mails (matière, classe, motif, noms) ; e-mail qui montre l'identifiant de la salle | rendu échappé du socle 8 |
| ND-06 | M02 | Destinataire de repli de l'e-mail d'annulation **fourni par le navigateur** | destinataire résolu côté serveur |
| ND-07 | M02 | **Toutes** les réservations (e-mails, commentaires, annulées comprises) lisibles par tout compte, sans borne de période | projections §21.2, plage bornée |
| ND-08 | M02 | Propriété d'une réservation par **e-mail** (ou par nom à l'affichage) | propriété par personne (D-020) |
| ND-09 | M02 | Le chemin IA **ne vérifie ni « réservable » ni l'EDT** | une seule commande serveur pour l'écran et l'IA (`IA_COEUR.md` §5.4) |
| ND-10 | M02 | Suppression d'une salle par remplacement du catalogue : réservations orphelines (pas de clé étrangère) | clés étrangères composites ; fermeture datée au socle 5 |
| ND-11 | M02 | Faux conflits EDT : « A1 » correspond à « A10 » (comparaison par sous-chaîne) | salle par identifiant |
| ND-12 | M02 | Écran plus restrictif que le serveur ; champs non vidables ; SMTP synchrone (délais dépassés) ; `alert` / `prompt` ; auteur d'annulation en texte | fil en ajout seul, file d'envoi (D-028 Q-N1) |
| ND-13 | M03 | **Élévation de privilège** : la responsable d'un domaine (ou tout rôle dont le libellé contient « administratif » ou « direction ») réécrit **tous** les domaines et leurs responsables, et les séances de tous les domaines | droits par domaine, rôles D-017 (jamais par libellé) |
| ND-14 | M03 | La contrainte « professeurs de SVT » ne bloque personne (la matière est forcée puis reconnue) ; séances « libres » ouvertes à tout compte | contrainte vérifiée sur les services (socle 6) |
| ND-15 | M03 | Écritures perdues et doublons en concurrence (document JSON réécrit en entier) ; écritures lors de simples lectures ; domaine lycée réinjecté, impossible à supprimer | tables (D-036), unicité en base |
| ND-16 | M03 | Un refus bloque la classe ; classe non vérifiée côté serveur ; suppression définitive sans motif ; aucune notification de décision | §12.2 |
| ND-17 | M03 | Injection HTML dans les e-mails | socle 8 |
| ND-20 | M16 | **Fuite de données personnelles** : avec n'importe quel e-mail vérifié, date de naissance + nom **ou** prénom renvoient le nom complet, la classe et **les responsables avec leur e-mail** ; plafond contournable en multipliant les adresses | socle 3 : aucune donnée nominative renvoyée avant rattachement vérifié |
| ND-21 | M16 | **Clé de stockage du PAP fournie par le client**, rattachée comme document de santé et **réécrite** sans contrôle | aucun dépôt avant admission ; identifiants d'objets émis par le serveur (socle 7) |
| ND-22 | M16 | **Annulation par simple ouverture d'un lien** (GET) : les analyseurs de liens des messageries annulent des rendez-vous | toute action par lien = page puis bouton (POST) (R-AG23) |
| ND-23 | M16, M17 | Tâches de fond qui parcourent **tous les établissements** avec le jeton, le SMTP et le domaine de l'établissement de la requête en cours | traitement par établissement (RD-06) |
| ND-24 | M16 | Lot de reconfirmation J-7 qui peut se saturer de lignes passées et ne plus rien envoyer | requête bornée à la fenêtre |
| ND-25 | M16 | **Mention du PAP, téléphone et e-mail du parent écrits dans Google Agenda** | aucune donnée de santé dans l'agenda (R-AG30) ; aucun agenda tiers par défaut |
| ND-26 | M16 | Jetons en clair en base, y compris le jeton de rafraîchissement Google ; jeton de garde réutilisable 2 h ; secret de repli | empreintes de jetons (R-AG23) |
| ND-27 | M16 | Erreurs brutes renvoyées au client | erreurs typées |
| ND-28 | M17 | Surréservation (compte puis insertion, sans verrou) | capacité en transaction (R-AG4) |
| ND-29 | M17 | Injection HTML (données saisies publiquement) ; envoi de confirmations à une adresse non vérifiée ; consentement **forcé à vrai** ; secret de support passé dans l'URL et comparé sans temps constant | socle 8 ; vérification de l'adresse (socle 3) ; consentement réel daté |
| ND-30 | Invitations | `lookup` **sans authentification ni limite** : un prénom et une date de naissance donnent le nom de famille et l'**e-mail du parent** ; n'importe qui peut remplacer la réponse d'une famille | lien nominatif, limitation de débit (R-AG23) |
| ND-31 | Invitations | Capacité sans verrou ; « e-mail envoyé » affiché même en échec | R-AG4 ; état réel de l'envoi (socle 8) |
| ND-32 | Partenariats | Places maximales **jamais vérifiées** ; limite de débit en mémoire qui fait confiance à `x-forwarded-for` | R-AG24 ; limitation de débit du socle 1 |

**Points de sécurité à remonter à Florian dès maintenant** (Next.js en production, D-035 point 3 ; le pilote n'y touche pas) : ND-20 et ND-30 (énumération de données de familles par une page publique), ND-21 (réécriture d'un objet désigné par le client), ND-22 (annulations involontaires), ND-13 (élévation de privilège). Liste reprise au §31.2.

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)

1. **Salles** : contrôle croisé avec l'EDT (A/B, remplacements) affiché dans la grille ; message de conflit qui dit **qui** occupe ; distinction salle spéciale / salle de classe, cette dernière non réservable par défaut ; réservation pour un collègue avec trois identités ; séries avec « cette occurrence / toute la série » ; recontrôle des seuls créneaux déplacés (corriger un commentaire n'est jamais bloqué) ; horizon avec exemption ; copier / coller, « mes 5 prochaines », lien `?new=1`, vue jour sur mobile, couleurs par matière, saisie niveau → classe ; assistant IA avec récapitulatif et confirmation.
2. **Programme par domaines** : grille niveau × séance × classe ; contrainte d'intervenant ; circuit proposition → validation / modifications / refus avec commentaire ; positionnement d'un collègue ; panneau « à valider », « mes positionnements », bandeau « domaine sans responsable » ; date de séance distincte de la validation ; PDF par séance.
3. **Rendez-vous d'inscription** : garde par e-mail avant toute donnée ; saisie de confirmation ; un rendez-vous actif par élève et par interlocuteur, remplacé automatiquement ; règle « libérer ou retirer » selon le délai ; reconfirmation J-7 où le silence vaut maintien ; « rechoix » demandé par l'administration avec motif ; changement manuel hors créneaux ; ICS joint ; notification par interlocuteur ; niveaux proposés selon l'interlocuteur ; personne présente au rendez-vous.
4. **Portes ouvertes** : créneaux par cycle ; pointage puis message de suivi différé ; équipe d'accueil et ambassadeurs ; planning PDF ; copie figée du créneau dans l'inscription ; ICS de création, modification et annulation ; sessions passées non supprimables.
5. **Invitations** : plafonds total et par élève ; date limite de réponse levable par le personnel ; question « situation actuelle » ; doublons suspects.

### 2.5 Verdict

- **Rust** : une base saine pour les rencontres (à corriger au lot AG0, puis à fondre dans le moteur au lot AG5), rien d'autre. Le socle est à construire presque entièrement.
- **Next.js** : la **réservation de salles est la seule fonction d'agenda en usage réel et fiable** ; elle doit être égalée **sans ses défauts** (ND-01 à ND-12) et branchée sur le cœur (salles du 5, séances du 10, personnes du 4 et du 6) pour la rentrée 2027 (D-035). Les rendez-vous d'inscription et les portes ouvertes sont des parcours aboutis côté usage, mais avec des **failles sérieuses** (ND-20 à ND-29) et une dépendance à Google : on reprend les **parcours**, pas le code. Le planning par domaines est une bonne idée mal construite : refait de zéro (§12.2).

---

## 3. Périmètre et frontières : qui possède quoi

| Sujet | Possédé par | Le 19… |
|---|---|---|
| Événements d'établissement (réunions, instances côté logistique, manifestations, journées banalisées) | **19** | possède |
| Rendez-vous individuels (famille, élève, personnel, externe) | **19** (sauf contenu sensible) | possède le rendez-vous ; le **contenu** d'un entretien psy (36), infirmier (34), social (36), disciplinaire (13) reste chez son propriétaire, relié par identifiant |
| Offres de créneaux (rencontres, entretiens, sessions à places) | **19** | possède le moteur ; le 3 (rendez-vous d'inscription, portes ouvertes), le 34 (campagnes, bilan de 6ᵉ), le 37 (entretiens d'orientation), le 42 (réunion d'information d'un voyage) le **consomment** par commande et gardent leur objet métier |
| Disponibilités déclarées des personnels | **19** | possède ; absences et indisponibilités administratives restent au 6 et au 15 (lues) |
| **Réservations de lieux** (salles, lieux du socle 5) | **19** [R, appliqué par défaut, D-031] | possède le moteur ; le 44 l'étend au matériel |
| Salles, lieux, capacité, équipements, indisponibilités, calendrier, fermetures | 5 | lit (`structure_disponibilite::disponible`, calendrier) |
| Séances, plages libres, occupation par les cours | 10 | lit (`edt::seances`, `edt::plages_libres`, `edt::occupe`) ; une réservation qui chevauche une séance passe par une **demande de modification de séance** au 10, jamais une écriture directe |
| Équipes (pédagogique, éducative), missions (PP, référent), annuaire | 6 | lit (`equipes::*`, `missions::actives`, `personnels::annuaire`) |
| Absences et remplacements des personnels | 15 | lit (occupé ; rendez-vous à déplacer, R-AG17) |
| Conseils de classe (préparation, avis, décisions) | 23 | fournit la réunion (date, salle, convocations, présence) ; le 23 garde avis et décisions |
| Conseil de discipline, commission éducative | 13 | fournit la réunion et ses convocations **à la demande** du 13, qui fixe le délai et le contenu ; titre neutre (R-AG5) |
| Instances élues (CA, CVL, CVC, commission permanente) : mandats, délibérations, votes, procès-verbaux | 46 | fournit la réunion (convocation, ordre du jour, présence, quorum calculé) ; le 46 garde délibérations et PV |
| Diffusion (aperçu, confirmation, relance des non-lecteurs), canaux, préférences | 8 | demande une diffusion ; ne choisit jamais un canal en dur |
| Conversations | 16 | fournit la carte d'action « prendre rendez-vous » ; un mot « demande de rendez-vous » crée une demande au 19 |
| Documents (convocation, compte rendu, émargement signé) | 7 | dépose par commande |
| Abonnements iCal, jetons, rendu | 9 (X15) | fournit la lecture minimisée `agenda::flux` |
| Visioconférence | 33 | stocke une référence de salle visio émise par le 33 |
| Formulaires et autorisations | 18 | une invitation peut porter une question fermée (coupon-réponse, M-355 du socle 8) ; un formulaire complet reste au 18 |
| Matériel, prêts, clés, maintenance | 44 | — |

---

## 4. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Chef d'établissement / directeur** | Convoque CA, conseil pédagogique, équipes ; reçoit les familles (inscriptions, conflits, recours) ; arbitre les salles | Agenda personnel hors ENT, double saisie ; délais de convocation recalculés à la main ; secrétariat qui l'appelle pour savoir s'il est libre | Convocation hors délai qui fragilise une décision ; famille reçue sans que l'autre parent l'ait su ; rendez-vous de crise écrit en clair dans un agenda partagé | « Proposer trois créneaux » à une famille en un clic ; tableau de bord des instances (dates, quorum, comptes rendus manquants) ; ses rendez-vous dans son téléphone sans Google |
| **Adjoint(e)** | Planifie conseils de classe, soirées parents-professeurs, réunions d'équipes ; gère les salles | Soirée parents-professeurs montée dans un tableur (qui, où, quelle salle) ; professeurs en service partagé indisponibles ; salles doublement réservées | Soirée où un professeur a un cours à 18 h ; parents qui attendent ; deux conseils dans la même salle | **Soirée générée** : professeurs, salles, plages, parcours des familles ; « salle libre jeudi 14 h, 30 places, vidéoprojecteur » |
| **Secrétariat** | Prend les rendez-vous de la direction ; envoie les convocations ; accueille | Téléphone, papier, agenda de la direction non partagé ; relances des absents ; impressions d'émargement | Erreur de date dans une convocation envoyée à 300 familles | Prise de rendez-vous par la famille dans des plages que la direction a ouvertes ; émargement imprimé ou sur tablette ; relance automatique des non-répondants |
| **Professeur** | Reçoit les familles (soirée, rendez-vous ponctuels), participe aux réunions, réserve des salles (TP, salle informatique) | Familles qui écrivent « quand êtes-vous disponible ? » ; créneaux de soirée imposés ; réunion placée sur son jour libre | Tomber sur un parent qu'une interdiction écarte ; voir son agenda personnel exposé aux collègues | Déclarer **une fois** « je reçois le mardi 17 h–18 h » ; ses rendez-vous de soirée prêts, avec les dernières notes de l'élève sous les yeux (socle 21, ses matières) ; noter « reçu » et deux lignes de suivi |
| **Professeur principal** | Organise les rencontres de sa classe, l'heure de vie de classe, les réunions avec l'équipe | Trouver un créneau commun ; savoir quels parents n'ont aucun rendez-vous | Une famille en difficulté qui ne vient jamais | Liste « familles à voir » issue du conseil de classe, invitation prioritaire avant l'ouverture à tous ; créneau commun de l'équipe proposé |
| **CPE / vie scolaire** | Reçoit élèves et familles (assiduité, comportement), commissions éducatives, équipes éducatives | Rendez-vous non honorés sans trace ; convocations au téléphone ; salle introuvable | Rendez-vous disciplinaire visible par l'élève d'à côté (titre en clair) ; délai de convocation manqué | Convoquer depuis la fiche de l'élève (responsables pré-remplis, droits calculés) ; « non venu » qui déclenche une relance ; titre neutre automatique |
| **Psychologue de l'Éducation nationale (PsyEN)** | Entretiens élèves et familles, orientation | Planning tenu à part (046) ; partage d'occupation impossible sans révéler | Que l'agenda révèle qu'un élève la voit | Ses créneaux ouverts à la réservation **sans motif** ; « occupé » pour tous les autres |
| **Infirmier(ère)** | Bilans de 6ᵉ, campagnes, rendez-vous à l'infirmerie | Convocations de 200 élèves à la main | Motif de santé visible | Convocations générées depuis la liste (socle 34), élèves sortis de cours selon l'EDT |
| **Assistant(e) social(e)** | Rendez-vous familles, souvent hors établissement | Rien de partageable | Un agenda qui trace des familles en difficulté | Comme la PsyEN |
| **AESH** | Participe aux ESS et réunions de suivi | N'est pas prévenue des réunions qui concernent l'élève accompagné | Être exclue des réunions | Convocation automatique quand l'élève accompagné a une ESS (socle 35) |
| **Accueil** | Accueille les familles, oriente | « J'ai rendez-vous avec quelqu'un » sans savoir qui ni où | Laisser entrer un parent interdit | **Liste du jour** des rendez-vous attendus (nom, heure, salle, hôte), sans motif ; alerte interdiction (socle 4/14) |
| **Agents (entretien, technique)** | Préparent les salles | Salle polyvalente découverte réservée le matin même | — | Planning des salles réservées avec la mise en place demandée (tables, vidéoprojecteur) |
| **Famille (responsable légal)** | Prend des rendez-vous, se rend aux réunions, répond aux invitations | Créneaux qui s'enchaînent mal (trois enfants, huit professeurs, une soirée) ; liens qui expirent ; pas de rappel la veille | Ne pas avoir de rendez-vous avec le professeur qui compte ; l'autre parent qui prend tout | **Parcours de soirée proposé**, ajout à son agenda en un geste, changement de créneau sans téléphoner, présence possible par visio ou téléphone |
| **Parents séparés** | Chacun veut son rendez-vous, ou un commun | Un seul rendez-vous par enfant « pris » par l'autre ; informations envoyées à un seul | Croiser l'autre parent (violences) ; être oublié | Rendez-vous séparés ou commun selon leur situation, information égale, aucune fuite |
| **Élève mineur** | Rendez-vous avec le CPE, la PsyEN, l'orientation ; réunions de délégués | Convocations orales oubliées ; cours manqué non signalé | Que ses parents ou ses camarades voient un rendez-vous chez la PsyEN | Ses rendez-vous dans son agenda, avec « cours manqué prévenu » |
| **Élève majeur** | Prend ses rendez-vous lui-même | Parents encore destinataires | — | Être le seul à décider qui est informé (socle 4) |
| **Externe** (prospect, intervenant, parent d'un futur élève, représentant de la mairie au CA) | Rendez-vous d'inscription, réunions, CA | Créer un compte pour prendre un rendez-vous | — | Lien nominatif à usage limité, sans compte |
| **Délégué à la protection des données (DPO)** | Contrôle le traitement | Agendas partagés qui contiennent des motifs | Motifs sensibles, conservation indéfinie, agenda tiers | Registre clair : catégories, durées, minimisation par construction |

---

## 5. L'année vue du terrain [H, à confirmer avec Florian]

| Période | Ce qui se passe | Socles | Ce que le 19 doit rendre facile |
|---|---|---|---|
| **Juin–juillet (N-1)** | Calendrier de l'année suivante : dates des conseils, des soirées, des réunions de rentrée, des instances ; rendez-vous d'inscription et de réinscription | 5, 3, 23, 46 | **Calendrier prévisionnel** copié de l'année précédente et décalé sur le nouveau calendrier scolaire (AGD-121) ; salles réservées par avance |
| **Fin août – septembre** | Prérentrée, réunions d'équipes, **réunions d'information des familles par niveau**, réunion des parents d'élèves de 6ᵉ, élections des représentants (46), sorties d'intégration | 6, 46, 42 | Convocations des personnels ; invitations des familles par niveau avec coupon-réponse ; salles et créneaux sans conflit |
| **Octobre** | CA de rentrée, CVL/CVC, ESS de rentrée, bilans de 6ᵉ (34) | 46, 35, 34 | Délais de convocation calculés ; convocations d'ESS avec famille et AESH |
| **Novembre – décembre** | **Rencontres parents-professeurs** (souvent après le premier trimestre), conseils de classe | 23 | Soirée générée ; invitations prioritaires issues des conseils |
| **Janvier – février** | Entretiens d'orientation de 3ᵉ et de terminale, journées portes ouvertes, rendez-vous d'inscription, CA budgétaire | 37, 3, 46 | Sessions à places ; rendez-vous d'inscription (moteur partagé avec le 3) |
| **Mars – avril** | Seconde série de rencontres, conseils, réunions d'information sur l'orientation, voyages (réunion des parents) | 23, 37, 42 | Idem ; réunion d'un voyage planifiée depuis le 42 |
| **Mai – juin** | Conseils du troisième trimestre, examens (salles), commissions d'appel, bilan des instances | 23, 38, 46 | Salles d'examen bloquées ; comptes rendus manquants listés |
| **Toute l'année** | Rendez-vous ponctuels (famille ↔ professeur, CPE, direction), équipes éducatives, commissions éducatives, réservations de salles, réunions régulières (conseil pédagogique, équipe de direction) | 13, 16, 36 | Demande de rendez-vous depuis un message ; créneaux ouverts ; récurrences ; occupation des salles |

---

## 6. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche d'un sous-agent le 2026-11-11 sur Légifrance (accessible), le BO et eduscol (pages HTML en **403** : textes lus sur copie académique ou PDF quand c'était possible). Liste de ce qui n'a pas pu être vérifié au §6.9.

### 6.1 Familles : rencontres, demandes, deux parents

| Règle | Source | Marque | Conséquence dans le socle |
|---|---|---|---|
| Les parents sont membres de la communauté éducative ; dialogue avec les enseignants et les autres personnels assuré dans chaque établissement | Code de l'éducation **L111-4** (version du 2 sept. 2019) — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000038901844 | [V] | fondement du socle |
| Réunion des parents des élèves nouvellement inscrits **dans les premiers jours suivant la rentrée** | **D111-1** (décret 2006-935) — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006166752/ | [V] | alerte au tableau des instances si non planifiée (AGD-340) |
| … « au plus tard avant la fin de la troisième semaine suivant la rentrée » | circulaire **2006-137** du 25 août 2006, BO n° 31 — https://www.education.gouv.fr/bo/BoAnnexes/2006/31/encart31.pdf | [V] | échéance de l'alerte |
| Rencontre parents-enseignants **au moins deux fois par an et par classe**, sous des formes diverses ; information sur l'orientation chaque année | **D111-2** | [V] | **compteur par classe** au tableau des instances (AGD-340) ; une campagne ou une réunion d'information compte |
| Réponse aux **demandes d'information et d'entrevue** des parents ; **toute réponse négative motivée** ; aucun délai fixé | **D111-4** ; circulaire 2006-137 (« orientées vers le bon interlocuteur ») | [V] | **motif obligatoire** au refus (AGD-154), transmission au bon interlocuteur (AGD-155), alerte à l'hôte d'une demande sans réponse à J+5 (R-AG46) |
| Le CA examine à sa première séance les conditions du dialogue avec les parents, de leur accueil et de leur accès à l'ENT ; parents informés du nombre, de la nature et de la date des rencontres | **D111-5** | [V] | calendrier des rencontres publié aux familles dès septembre (AGD-112 côté familles) |
| Heures des CA, conseils de classe et conseils de discipline fixées pour **permettre la représentation des parents** ; concertation si le calendrier doit être adapté | **D111-12** | [V] | avertissement à la création d'une instance en horaire de journée (non bloquant) |
| Les représentants des parents reçoivent les mêmes documents que les autres membres | **D111-13** | [V] | pièces de l'ordre du jour envoyées à tous les membres de l'instance |
| Réunions collectives à des horaires compatibles avec les contraintes des parents ; rencontres individuelles confidentielles ; faciliter les échanges avec les parents qui ne maîtrisent pas le français | circulaire 2006-137 | [V] | modalités téléphone / visio, interprète (AGD-330) |
| Relations **avec les deux parents** ; deux adresses → courriers aux deux ; un tiers à qui le juge a confié l'autorité a les mêmes droits | circulaire 2006-137 (renvoi à l'art. 371-1 du Code civil) | [V] | **R-AG12** : chaque parent à autorité est invité ; courrier aux deux adresses (AGD-321) |
| Parents séparés à autorité conjointe : mêmes informations à chacun ; autorité exercée par un seul : l'autre reçoit les décisions, pas tous les détails | service-public.fr **F21287** (vérifiée le 15 sept. 2025) — https://www.service-public.gouv.fr/particuliers/vosdroits/F21287 ; Code civil **372-2**, **373-2-1** | [V] | parent sans autorité : informé des **convocations décisionnelles** (orientation, discipline), pas des rendez-vous courants (R-AG47) |
| Parents invités régulièrement, pas seulement en cas de difficulté ; **accompagnés d'une personne de leur choix** ; attention aux parents handicapés ; oral privilégié pour ceux qui ont des difficultés avec l'écrit ; ENT accessibles | circulaire **2013-142** du 15 oct. 2013 — copie CNAF https://caf.fr/sites/default/files/medias/881/Partenaires-locaux/Parentalit%C3%A9/CLAS/Circulaire_15-10-2013.pdf | [V-copie] | **accompagnant de droit** (CAG-16 corrigé : l'hôte ne peut pas refuser l'accompagnant, sauf interdiction) ; appel téléphonique proposé (AGD-320) |
| Élève majeur : information des parents | aucun texte national trouvé ; pratique « sauf opposition » des règlements intérieurs | [NV] | R-AG14 inchangée, règle du socle 4 |

### 6.2 Personnels

| Règle | Source | Marque | Conséquence |
|---|---|---|---|
| Les relations avec les parents et le travail en équipes pédagogiques sont des missions liées au service ; **aucun volume ni plafond** d'heures de réunion | décret **2014-940**, art. 2 II — https://www.legifrance.gouv.fr/loda/article_lc/LEGIARTI000029394721 | [V] | pas de compteur d'heures de réunion imposé ; indicateur sans nom seulement (47) |
| Ces missions incluent réunions d'équipes, conseils de classe, réunions parents-professeurs, heures de vie de classe | circulaire **2015-057** du 29 avr. 2015 (copie académie de Bordeaux) | [V-copie] | les campagnes convoquent les enseignants (pas une simple invitation) |
| Professeur principal : désigné avec son accord ; coordonne le suivi et l'orientation en lien avec les familles | **D421-49-1** ; **R421-51** | [V] | PP organisateur de sa classe (droits §21.1) |
| Circulaire 2018-108 (PP référent des représentants légaux) | BO n° 37 du 11 oct. 2018 | [NV] (403) | — |

### 6.3 Instances de l'EPLE public (logistique au 19, contenu aux socles 46, 23, 13)

| Instance | Convocation | Délai | Quorum | Autres | Source [V] |
|---|---|---|---|---|---|
| **Conseil d'administration** | chef d'établissement ; extraordinaire sur demande du recteur, de la collectivité ou de la moitié des membres | **8 jours au moins**, avec ordre du jour et documents ; **1 jour en urgence** | majorité des membres en exercice ; à défaut nouvelle réunion **sous 5 à 8 jours** (3 en urgence) sans quorum | au moins 3 fois par an ; séances **non publiques** (R421-19) ; vote secret de droit sur demande (R421-24) | **R421-25** (version du 24 déc. 2020) — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000042728200 |
| Commission permanente | idem | **8 jours / 1 jour** | idem | | R421-37 à R421-41 |
| **Conseil pédagogique** | chef d'établissement | **8 jours / 3 jours** en urgence | majorité ; à défaut nouvelle convocation au plus tôt le lendemain et avant le CA | au moins 3 fois par an | R421-41-2 à R421-41-6 |
| **Conseil de classe** | chef d'établissement ou représentant | non fixé | — | au moins 3 fois par an ; membres : enseignants, 2 délégués des parents, 2 délégués des élèves, CPE, PsyEN ; médecin, AS, infirmier s'ils ont eu à connaître du cas d'un élève | R421-50, R421-51 |
| CVL | chef d'établissement, **avant chaque CA ordinaire** | — | à défaut nouvelle convocation sous 3 à 8 jours | | R421-43, R421-44 |
| CVC | chef d'établissement | fixé par le CA | | | R421-45-1 |
| Assemblée générale des délégués | | | | **au moins 2 fois par an, dont une avant la fin de la 7ᵉ semaine** | R421-42 |
| CESCE | chef d'établissement ou demande du CA | non fixé | | | D421-46, D421-47 |
| Commission éducative | chef d'établissement | non fixé | | au moins un professeur et un parent | R511-19-1 |
| **Conseil de discipline** (contenu au 13) | chef d'établissement | élève et représentant légal **5 jours au moins**, **pli recommandé ou remise en main propre contre signature** ; membres et témoins 5 jours, tout moyen | à défaut nouvelle réunion sous 5 à 10 jours | votes à bulletins secrets ; PV signé, copie au recteur sous 5 jours | D511-31 à D511-42 — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000020663030/ |
| **ESS** | **l'enseignant référent** (extérieur à l'établissement) | — | — | au moins une fois par an ; parents membres de droit (assistés ou représentés) ; élève majeur mêmes droits ; document d'évaluation à la MDPH, à la famille et au chef d'établissement | D351-10 à D351-16 — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006182561/ |
| PAP | — | — | — | avis du médecin de l'Éducation nationale ; révisé chaque année | D311-13 |
| Équipe éducative | aucun texte trouvé pour le second degré (D321-16 vaut pour le premier degré) | | | | [NV] |

**Conséquences** : délais calculés par type (R-AG10) en jours calendaires, avec forçage « urgence » seulement là où le texte le prévoit (CA, commission permanente : 1 jour ; conseil pédagogique : 3 jours) ; second CA sans quorum proposé dans la fenêtre de 5 à 8 jours ; CVL alerté s'il n'est pas planifié avant un CA ordinaire ; convocation au conseil de discipline **par courrier recommandé ou remise contre signature** préparée (AGD-321) mais **portée par le 13** ; l'ESS est une réunion **reçue** (AGD-331, confirmé) ; compteurs « au moins 3 fois par an » (CA, conseil pédagogique, conseils de classe) et « 2 fois par an » (AG des délégués, rencontres par classe) au tableau des instances (AGD-340).

### 6.4 Privé sous contrat

Conseil de classe « réuni sous la présidence du chef d'établissement » pour les propositions d'orientation, composition **non définie** (D331-54) ; en cas de désaccord, le chef d'établissement **reçoit l'élève et ses parents**, réponse sous 3 jours ouvrables (D331-56) [V] — https://www.legifrance.gouv.fr/codes/id/LEGISCTA000006182555. Applicabilité des D111-1 à D111-15 au privé, conseil d'établissement (Statut de l'Enseignement catholique, norme interne) : **[NV]**. Conséquence : types d'instances **par régime** ; le rendez-vous de désaccord d'orientation (D331-56) est un rendez-vous obligatoire avec échéance de réponse, proposé depuis le socle 37 (AGD-341).

### 6.5 ENT et données

- **Arrêté du 30 novembre 2006** relatif aux ENT (modifié le 13 oct. 2017) [V] — https://www.legifrance.gouv.fr/loda/id/LEGITEXT000006054889 : données des responsables limitées (civilité, noms, date de naissance, adresse, téléphones, courriel) ; suppression **sous 3 mois** quand la personne n'a plus vocation à un compte → disponibilités, plages occupées et liens d'un personnel ou d'un parent parti effacés sous 3 mois (§21.3).
- **SDET, kit juridique** (version 2025, mis à jour en février 2026) [V] — https://www.education.gouv.fr/sites/default/files/2025-07/kit-juridique-441540.pdf : données conservées en base active pour l'année scolaire ou le cycle ; **journaux 12 mois au maximum** ; journalisation nominative des accès y compris des administrateurs → le **journal d'accès** du socle est limité à 12 mois (distinct du **fil métier** d'un rendez-vous ou d'une instance, qui suit la conservation de son objet ; §21.3 corrigé).
- **SDET 2024** (appel à commentaires) [V] — https://eduscol.education.gouv.fr/sites/default/files/document/sdetdocument-principalversionaac2024vdpdf-105189.pdf : service **Agenda** (agendas pour tous, **vue superposée** agenda personnel, agendas partagés, EDT, cahier de textes ; synchronisation, superposition et **délégation d'accès**) et service de **réservation de salles et de matériels** → confirme le périmètre (§8, §14, AGD-181). SDET 2025 non relu sur ces points [NV].
- **CNIL**, guide de la sécurité 2024, fiche 10 [V] — https://www.cnil.fr/sites/cnil/files/2024-03/cnil_guide_securite_personnelle_2024.pdf : ne pas faire transiter de données personnelles dans une URL → jeton d'abonnement iCal et liens externes **opaques**, sans identifiant (R-AG23). Conseils visioconférence du 9 avril 2020 [V] (chiffrement, minimisation, mineurs) → socle 33. Aucune doctrine CNIL spécifique trouvée sur la prise de rendez-vous en ligne, les agendas partagés, les flux iCal [NT].

### 6.6 Accessibilité

Loi **2005-102, art. 47** (version du 8 sept. 2023) [V] — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000000809647/ : obligation pour les personnes morales de droit public (EPLE) et, par interprétation **[NV]**, les organismes privés chargés d'une mission de service public ; sites, applications mobiles et progiciels ; déclaration d'accessibilité ; sanctions jusqu'à 50 000 € (contrôle ARCOM). Conséquence : grilles (planning des salles, créneaux) doublées d'une **vue liste** accessible au clavier et au lecteur d'écran ; aucun parcours réservé à la grille (R-AG48). Aucune obligation d'interprétariat trouvée (circulaire 2012-141 : documents d'accueil traduits « autant que faire se peut » [V-copie]).

### 6.7 Calendrier

Arrêté du **22 octobre 2025** (2026-2027) [V] — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000052416058 ; arrêté du **21 juillet 2026** (2027-2028) [V] — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000054457294 ; jeu ouvert `fr-en-calendrier-scolaire` [V] (dates en UTC à minuit heure de Paris, date de fin = jour de reprise : piège à signaler au socle 5). Vacances d'hiver 2027 : zone A 13 févr.–1ᵉʳ mars, B 20 févr.–8 mars, C 6–22 févr. (utile à Q-AG10). Jours fériés : Code du travail L3133-1 [V] ; Alsace-Moselle [NV]. Ces données appartiennent au socle 5, lues par l'agenda.

### 6.8 Élections

Élections des représentants des parents le 9 ou 10 octobre 2026 (note de service du 25 juin 2026) [NV] : relèvent du socle 46 ; l'agenda n'affiche que la date publiée.

### 6.9 Non vérifié

Circulaire 2018-108 ; portée exacte de l'annulation partielle de la circulaire 2015-057 (CE n° 391265) ; brochure « exercice de l'autorité parentale en milieu scolaire » ; circulaires PAP (2015-016) et PAI (10 févr. 2021) ; équipe éducative dans le second degré ; procès-verbal du CA et du conseil de classe ; visioconférence pour les instances d'un EPLE (ordonnance 2014-1329 lue, application non confirmée) ; règles du privé sous contrat ; SDET 2025 ; version du RGAA ; Alsace-Moselle.

### 6.10 Actions et règles issues du réglementaire

| ID | Action / règle | Lot |
|---|---|---|
| AGD-340 | **Tableau de conformité** : réunion des nouveaux parents avant la fin de la 3ᵉ semaine (D111-1), au moins 2 rencontres par classe et par an (D111-2), CA, conseil pédagogique, conseils de classe au moins 3 fois par an, AG des délégués 2 fois dont une avant la 7ᵉ semaine, CVL avant chaque CA ordinaire | AG7 |
| AGD-341 | Rendez-vous de **désaccord d'orientation** (privé, D331-56) avec échéance de réponse de 3 jours ouvrables, à la demande du socle 37 | AG7 |
| R-AG46 | Une demande d'entrevue d'un parent reçoit une réponse ; un refus est **motivé** ; sans réponse à J+5, rappel à l'hôte puis à son responsable (D111-4) | AG4 |
| R-AG47 | Un parent **sans** autorité parentale (mais avec droit d'information enregistré au socle 4) est informé des convocations décisionnelles (orientation, discipline) et des décisions, pas des rendez-vous courants (373-2-1 C. civ., F21287) | AG4 |
| R-AG48 | Toute grille a une vue liste accessible équivalente (loi 2005-102, art. 47) | AG1, AG3, AG5 |

---

## 7. Concepts et vocabulaire [R]

| Terme | Définition | Ce n'est pas |
|---|---|---|
| **Agenda** | Vue calculée, pour un lecteur et une plage, de tout ce qui le concerne et qu'il a le droit de voir | une table |
| **Source** | Socle qui fournit des éléments à l'agenda par un port de lecture (10 séances, 5 calendrier, 42 sorties, 23 conseils, 38 examens, 39 PFMP, 15 absences, 19 lui-même) | une copie |
| **Événement** | Fait daté possédé par le 19 : réunion, instance (logistique), réunion d'information, manifestation, journée banalisée, temps fort | une séance de cours |
| **Réunion** | Événement à **participants convoqués** ou invités, avec ordre du jour, présence et compte rendu possibles | une conversation |
| **Convocation** | Invitation **obligatoire** à une réunion, avec délai réglementaire éventuel, accusé de réception, présence attendue | une information |
| **Invitation** | Proposition de participer, avec réponse facultative (oui / non / peut-être, coupon) | une convocation |
| **Rendez-vous** | Engagement daté entre un ou plusieurs **hôtes** (personnels) et des **participants** (responsables, élève, externe), avec éventuellement un **élève objet** | un événement collectif |
| **Offre de créneaux** | Ensemble de créneaux proposés à un **public** par des **hôtes**, avec règles de réservation ; une **campagne** est une offre datée et ciblée (rencontre parents-professeurs, entretiens d'orientation) | une liste de rendez-vous |
| **Créneau** | Plage d'une offre, d'une capacité (1 = rendez-vous, N = session), rattachée à un hôte ou à un lieu | une réservation |
| **Réservation (de créneau)** | Prise d'une place d'un créneau par un participant | une réservation de lieu |
| **Session** | Créneau de capacité N (atelier, portes ouvertes, réunion par vagues, bilan de santé collectif) | un cours |
| **Disponibilité** | Plage déclarée par un personnel où il **accepte** des rendez-vous (récurrente ou datée) | son emploi du temps |
| **Occupation** | Le fait qu'une personne ou un lieu soit pris sur une plage, quel qu'en soit le motif ; seule information croisée entre personnels (R-AG6) | le motif |
| **Réservation de lieu** | Prise d'une salle ou d'un lieu (socle 5) sur une plage, par un demandeur, pour un motif | une séance |
| **Liste d'attente** | File de participants pour un hôte ou une offre complète, servie automatiquement par ordre, avec délai de réponse | une demande |
| **Vague** | Fenêtre de réservation réservée à un sous-public avant l'ouverture à tous (familles « à voir », fratries, parents séparés) | une offre |
| **Parcours** | Suite de rendez-vous d'une même famille dans une même offre, enchaînés sans chevauchement ni attente inutile | une réservation |
| **Fil** | Faits datés d'un rendez-vous ou d'une réservation, en ajout seul | un journal technique |

**Distinctions qui manquent aujourd'hui [F]** : rendez-vous et créneau sont confondus dans `ent_creneau_rdv` (un créneau réservé *est* le rendez-vous : l'annuler l'efface, RD-01) ; aucune notion d'hôte multiple (professeur + CPE), de participant (seul le réservant existe), de modalité (sur place, téléphone, visio), de lieu.

---

## 8. L'agenda calculé et sa visibilité

### 8.1 Les sources et leurs ports

| Source | Port lu | Ce qui entre dans l'agenda | Qui le voit dans son agenda |
|---|---|---|---|
| 10 séances | `edt::seances(cible, plage)` | cours, modifications (déplacé, annulé, salle changée) | chacun selon les droits du 10 (D-022) |
| 5 calendrier | `structure::calendrier(plage)` | vacances, jours fériés, journées banalisées, fermetures, ponts | tous |
| 42 sorties | `sorties::agenda(lecteur, plage)` | sorties et voyages publiés | participants, encadrants, familles des participants |
| 23 conseils | `conseils::agenda(lecteur, plage)` | conseils de classe (date, salle) | personnels convoqués ; délégués ; familles selon réglage (date seulement) |
| 38 examens | `examens::agenda(lecteur, plage)` | épreuves, convocations | candidats, surveillants |
| 39 stages | `stages::agenda(lecteur, plage)` | périodes de PFMP | élève, responsables, référent |
| 15 absences | `absences::occupation(agent, plage)` | « absent » (sans motif) | l'agent ; « occupé » pour qui cherche un créneau |
| 20 travail | `cahier::echeances(lecteur, plage)` | devoirs à rendre, évaluations annoncées | élève, responsables, professeur |
| 19 lui-même | — | événements, rendez-vous, sessions, réservations de lieux, disponibilités | participants ; « occupé » pour les autres |

**R-AG1** : rien n'est recopié ; chaque élément porte sa source, son identifiant chez le propriétaire et sa date de calcul ; un élément disparu à la source disparaît de l'agenda. Un port indisponible donne une **mention explicite** (« emploi du temps momentanément indisponible »), jamais un agenda faussement vide.

### 8.2 Les vues

| Vue | Pour qui | Contenu |
|---|---|---|
| **Mon agenda** | chacun | tout ce qui le concerne ; jour, semaine, mois, liste ; filtres par source |
| **Agenda d'un enfant** | responsable (lien actif) | séances, sorties, rendez-vous qui le concernent et où ce responsable a sa place (R-AG12), réunions des familles de sa classe, calendrier |
| **Agenda de l'établissement** | personnels ; familles (partie publique) | événements publiés à audience large (réunions d'information, manifestations, calendrier) |
| **Libre / occupé** | personnels habilités | pour une liste de personnes et de lieux, plages libres et occupées, **sans motif** |
| **Planning des salles** | personnels ; agents | occupation des lieux (cours, réservations, examens), mise en place demandée |
| **Liste du jour** | accueil, vie scolaire | rendez-vous et visiteurs attendus : nom du visiteur, heure, hôte, lieu ; **jamais le motif** ; alerte si une interdiction concerne le visiteur |
| **Tableau des instances** | direction | instances à venir, délais de convocation, quorum, comptes rendus manquants |
| **Ma soirée** | professeur, famille | parcours de la soirée parents-professeurs |

### 8.3 Ce qu'un élément montre selon le lecteur (R-AG5)

| Lecteur | Rendez-vous normal | Rendez-vous restreint (PsyEN, infirmerie, social, discipline, équipe éducative, interdiction) |
|---|---|---|
| Participant (hôte, invité, responsable concerné) | tout ce qui lui est destiné | idem, motif seulement si son rôle l'admet |
| Personnel cherchant un créneau | « occupé » | « occupé » |
| Responsable non participant (autre parent) | selon R-AG12 | **rien** |
| Élève objet non invité | rien (mineur), selon R-AG14 (majeur) | rien |
| Accueil | nom du visiteur, heure, hôte, lieu | idem, titre « rendez-vous » sans hôte si l'hôte révèle le motif (PsyEN, assistante sociale) [R] |
| Abonnement iCal | titre neutre (R-AG25) | « occupé » |

Le **titre visible** d'un rendez-vous pour un non-participant est **calculé** depuis sa nature (« Rendez-vous », « Réunion »), jamais saisi. Le titre saisi n'est vu que par les participants.

---

## 9. Événements et réunions

### 9.1 Types d'événements

Types fournis (catalogue dans le code, D-023), complétables par l'établissement :

| Famille | Exemples | Convocation | Délai | Compte rendu | Confidentialité par défaut |
|---|---|---|---|---|---|
| Réunion interne | conseil pédagogique, équipe de direction, équipe disciplinaire, équipe de niveau, réunion de prérentrée | oui | réglage | facultatif | personnels invités |
| Réunion autour d'un élève | équipe éducative, ESS (socle 35), réunion PAP/PAI (34, 35), entretien de suivi | oui | réglementaire si texte (§6) | oui, dépôt au socle propriétaire | **restreinte** |
| Instance (logistique pour 46, 23, 13) | CA, commission permanente, CVL, CVC, conseil de classe, conseil de discipline, commission éducative, CESCE | oui | **réglementaire** (§6, calculé) | PV au socle propriétaire | selon instance |
| Information des familles | réunion de rentrée par niveau, réunion d'orientation, réunion d'un voyage (42), réunion des nouveaux parents | invitation | — | support déposé (7) | audience |
| Manifestation | spectacle, remise des diplômes, cross, journée portes ouvertes (3), forum des métiers | invitation, éventuellement inscription | — | — | audience, publique possible |
| Journée et temps fort | journée banalisée, journée d'intégration, semaine des langues | information | — | — | audience |
| Rappel | échéances administratives (dossiers de bourse, inscriptions aux examens) | information | — | — | audience |

### 9.2 Cycle d'un événement

`brouillon → publié → (reporté → publié) → tenu → clos` ; `annulé` depuis tout état non clos.
- **Publier** résout l'audience **sous les droits de l'auteur**, à la date de publication, et demande au socle 8 une diffusion (aperçu : nombre de destinataires, canaux, exclusions « ne pas solliciter », coût SMS éventuel) confirmée par l'auteur (R-AG8).
- **Reporter** garde l'événement et ses réponses, prévient les invités, recalcule le délai de convocation (un report qui passe sous le délai réglementaire est signalé, R-AG10).
- **Annuler** exige un motif public court ; les invités sont prévenus ; les réservations de lieux liées sont libérées (R-AG19).
- **Tenu** : présence saisie ; **clos** : compte rendu déposé ou déclaré sans objet.

### 9.3 Récurrence

Règle **typée** (pas de chaîne RRULE libre, D-036) : fréquence (hebdomadaire, toutes les deux semaines, mensuelle « n-ième jour de la semaine », mensuelle à date), jours de la semaine, du / au, **semaines A/B** (socle 5) et **saut des vacances** par défaut. Les occurrences sont **matérialisées** jusqu'à la fin de l'année scolaire (une ligne par occurrence, liée à la série) : modifier une occurrence la détache ; modifier la série propose « cette occurrence / celles qui suivent / toutes ».

### 9.4 Réunion : convocation, ordre du jour, réponses, présence, compte rendu

1. **Participants** : calculés (équipe pédagogique d'une classe, équipe éducative constituée, membres d'une instance du 46, mission « référent ») et nominatifs ; chaque participant est une **personne** (D-020) avec un rôle (organisateur, président, convoqué, invité, informé, secrétaire de séance, rapporteur, interprète).
2. **Ordre du jour** : points ordonnés (intitulé, nature information / débat / vote, rapporteur, durée, pièces du socle 7) ; pour une instance, le 46 fournit les points de délibération.
3. **Convocation** : texte prêt (3 trames par type, D-023), date limite calculée, envoi par le socle 8 (accusé de réception pour une convocation), dépôt d'un PDF au coffre pour les externes (D-027 G : lien nominatif, jamais en pièce jointe).
4. **Réponses** : oui / non / peut-être, représentation (procuration pour une instance du 46), motif d'excuse facultatif ; relance des non-répondants (socle 8).
5. **Quorum** : calculé pour une instance (règle fournie par le 46) ; alerte « quorum non atteint d'après les réponses » à J-2.
6. **Présence** : émargement sur tablette ou téléphone de l'organisateur, ou feuille imprimée puis pointée ; retard, départ anticipé ; présence d'un parent ou d'un externe.
7. **Compte rendu** : document du socle 7 (rédigé dans ScolIA ou déposé), validé par l'organisateur, diffusé aux participants ou à une audience ; pour une réunion autour d'un élève, déposé dans le dossier de l'élève sous la nature du socle propriétaire (35, 13…) avec ses droits.
8. **Actions décidées** : liste de suites (qui, quoi, échéance) qui deviennent des à-faire du socle 29 pour leurs responsables [R].

### 9.5 Réunions qui touchent l'emploi du temps

Une réunion placée sur un temps de cours (journée banalisée, conseil de classe à 16 h) produit une **demande de modification** au socle 10 (cours libérés, publics concernés) ; elle n'écrit jamais dans l'EDT (R-AG16). Inversement, une modification d'EDT qui rend un convoqué indisponible le signale à l'organisateur.

---

## 10. Rendez-vous individuels

### 10.1 Trois façons de naître

| Initiative | Parcours | Exemple |
|---|---|---|
| **Le personnel propose** | choisit les participants (responsables pré-remplis depuis l'élève, selon R-AG12), propose 1 à 5 créneaux (calculés libres pour lui, l'élève s'il est invité, le lieu), la famille choisit ou demande autre chose | le CPE convoque une famille après des absences |
| **La famille (ou l'élève) demande** | depuis la fiche de l'enfant, la carte « prendre rendez-vous » d'un message (16) ou les **créneaux ouverts** d'un personnel ; motif choisi dans une liste courte + texte facultatif | un parent demande à voir le professeur de mathématiques |
| **La famille réserve directement** | dans les **disponibilités ouvertes** d'un personnel (« je reçois le mardi 17 h–18 h ») | le PP ouvre deux heures par semaine |

### 10.2 Cycle

`demandé → proposé → confirmé → (reporté → confirmé) → honoré | non venu` ; `annulé` (par qui, motif) depuis tout état non terminal ; `refusé` pour une demande (motif, proposition d'un autre interlocuteur ou d'un échange écrit). Chaque transition est un **fait** en ajout seul (R-AG3).

### 10.3 Modalités

Sur place (lieu : salle du socle 5 ou bureau de l'hôte), **téléphone** (numéro de l'établissement ou de l'hôte ; jamais le numéro personnel d'un enseignant, R-AG22), **visio** (salle du socle 33). La modalité est choisie par l'hôte parmi celles qu'il accepte ; la famille peut demander un changement.

### 10.4 Ce que l'hôte a sous les yeux

Une **fiche de rendez-vous** : participants, objet (élève), motif, historique des rendez-vous précédents **avec cet hôte ou son service**, et des liens vers les lectures auxquelles il a déjà droit (notes de ses matières au 21, assiduité au 12, PAP sans détail médical au 35) : aucun droit supplémentaire n'est donné par le rendez-vous (R-AG9). Après le rendez-vous : « honoré / non venu », **suite** en deux lignes (visible des seuls hôtes et de leur service, jamais de la famille par défaut, [R] Q-AG4), et éventuellement un nouveau rendez-vous.

### 10.5 Délais et annulations

- Réservation : au plus tard **N heures avant** (réglage par offre, défaut 12 h, S19-01) ; jamais dans le passé (RD-02).
- Annulation par la famille : jusqu'à N heures avant (défaut 2 h) ; après, « prévenir d'un empêchement » envoie un avis à l'hôte et garde le créneau marqué « empêché ».
- Annulation par l'hôte : motif public, **proposition automatique** de nouveaux créneaux, avis U2 si le jour même (socle 8).
- Absence de l'hôte (socle 15) : ses rendez-vous de la plage sont signalés « à reprogrammer » à lui et à l'organisateur (R-AG17) ; un remplaçant n'en hérite jamais automatiquement.

### 10.6 Rendez-vous sensibles

Les natures « psychologue », « infirmerie », « assistant social », « disciplinaire », « protection de l'enfance » sont **restreintes** : titre calculé « occupé » pour tous les non-participants, aucun motif dans l'agenda, aucune entrée dans un abonnement iCal autre que « occupé », aucun accès IA (R-AG7). Le contenu (motif détaillé, note) reste dans le socle propriétaire ; le 19 ne porte que la logistique (date, lieu, participants). La migration de `ent_psy_rdv` vers le moteur se fera au socle 36, qui garde la décision [R].

---

## 11. Rencontres parents-professeurs et campagnes de rendez-vous

### 11.1 Monter la soirée (adjoint, PP)

1. **Public** : classes, niveaux ou groupes ; date(s) et plage horaire ; lieux (une salle par hôte, ou un grand espace avec tables numérotées).
2. **Hôtes calculés** : enseignants qui ont **au moins un élève du public** (services 6, interventions 10), PP, CPE, direction, PsyEN facultative ; chaque hôte voit son nombre d'élèves concernés. Les professeurs en **service partagé** sont signalés avec leurs contraintes déclarées.
3. **Plages proposées automatiquement** : pour chaque hôte, plage = intersection de la plage de la soirée, de son EDT libre (port 10), de ses absences (15) et de ses disponibilités déclarées ; durée par défaut 10 min (réglage), pauses automatiques toutes les 90 min, **capacité calculée** (élèves concernés × taux de demande observé l'année précédente) et alerte si l'offre est insuffisante (« Mme X a 156 élèves et 24 créneaux »).
4. **Salles** : attribuées automatiquement par proximité et équipement (réservations de lieux du §14, faites dans la même commande).
5. **Règles** (§11.3) puis **vagues** : vague 1 pour les familles désignées « rencontre souhaitée » par le conseil de classe (23) ou le PP, vague 2 pour les fratries et les parents séparés (plus de contraintes), puis tous.
6. **Aperçu** : nombre de familles, de créneaux, ratio, conflits détectés ; **ouverture** confirmée (diffusion du socle 8).

### 11.2 Côté famille

- Elle voit, **par enfant**, les seuls enseignants qui l'ont (RP1-27) avec leur matière, ceux que l'établissement **souhaite** qu'elle rencontre en tête, et ceux qui ne reçoivent pas (« ne reçoit pas à cette soirée : écrire »).
- **Parcours proposé** : elle coche les professeurs voulus, ScolIA propose l'enchaînement le plus court (heure d'arrivée, ordre, salles, temps de déplacement entre bâtiments), pour **tous ses enfants** ; elle accepte ou ajuste.
- Créneau complet → **liste d'attente** chez cet enseignant ; un désistement propose la place au premier de la liste, qui a 12 h pour l'accepter (réglage).
- Le jour J : **« ma soirée »** sur mobile (ordre, salles, plan), retards de l'enseignant affichés, « je suis en retard » prévient l'enseignant.

### 11.3 Règles de la campagne

| Règle | Défaut [R] | Réglage |
|---|---|---|
| Un rendez-vous par hôte et par enfant | oui | non (S19-03) |
| Parents d'un **même foyer** | un rendez-vous commun ; l'autre parent est **informé** et peut se joindre | — |
| Parents de **foyers distincts** | chacun peut avoir son rendez-vous ; ils sont proposés à la suite si les deux le demandent « ensemble » | « commun obligatoire » (S19-04) |
| Interdiction, adresse protégée, mesure (socle 4) | aucun parcours, horaire ni lieu d'un parent visible de l'autre ; jamais à la suite ; l'hôte est averti sans motif (« ne pas recevoir ensemble ») | — (non réglable) |
| Plafond par famille et par soirée | 12 rendez-vous | S19-05 |
| Ouverture / fermeture des réservations | J-14 / J-1 18 h | par campagne |
| Présence de l'élève | facultative, affichée | « souhaitée » / « non » |
| Modalités | sur place ; téléphone et visio si l'hôte les accepte | par hôte |
| Élève majeur | il réserve ; ses parents seulement s'il l'a accepté (socle 4) | — |

### 11.4 Le soir même et après

- **Tableau de l'adjoint** : rendez-vous par salle, retards, absents ; « M. X a 20 min de retard » prévient ses rendez-vous suivants.
- **Hôte** : liste de ses rendez-vous, fiche de chaque élève (ses droits), « honoré / non venu », suite.
- **Après** : statistiques sans nom (taux de rendez-vous par classe, familles jamais venues sur l'année — visible du PP et du CPE pour leurs élèves seulement), proposition de rendez-vous individuels aux familles sans rendez-vous [R].

### 11.5 Autres campagnes sur le même moteur

Entretiens d'orientation (37), entretiens de réinscription ou d'inscription (3 : le socle 3 garde son contact et son dossier, le 19 porte le créneau, la réservation provisoire, la reconfirmation à J-N), bilans de santé de 6ᵉ ou campagnes de vaccination (34 : convocations sans motif visible), rendez-vous avec la direction avant les vacances, oraux blancs.

---

## 12. Sessions collectives et inscriptions

Un créneau de **capacité N** est une session : portes ouvertes (visite par groupe), atelier à places limitées, réunion d'information en plusieurs vagues, forum des métiers par demi-heure, séance de vaccination, journée d'intégration par groupe. Règles : capacité, public autorisé, inscriptions individuelles ou par élève, liste d'attente, pointage à l'arrivée (QR code ou liste), message de suivi différé (repris de Next.js M17, §2.2), annulation de session (avis, proposition d'une autre). Les **objets métier** restent au propriétaire : le prospect au socle 3, l'activité régulière (club, AS) au socle 43, la vaccination au 34.

### 12.1 Inscriptions à places

Capacité tenue **en transaction** (mise à jour conditionnelle du nombre de places, ND-28, ND-31) ; doublon refusé par unicité (session, personne, élève) ; copie figée du libellé et de l'horaire dans l'inscription (repris de M17) ; plafonds par élève (accompagnants d'une remise de diplômes : 4, total : 200, repris de NX-70) ; date limite de réponse, levable par l'organisateur.

### 12.2 Programmes d'interventions par classe (refonte de M03 « planning par domaines »)

M03 n'est pas un agenda mais il en a besoin (date, salle, intervenant extérieur, séance de cours remplacée). Aucun socle de la carte ne porte les **parcours et programmes éducatifs** (EVARS, prévention, citoyenneté, éducation aux médias). Recommandation appliquée par défaut (D-031) : le 19 les accueille comme **programmes d'interventions** (lot AG6b), en attendant une décision de Florian sur un socle dédié (Q-AG2).

- **Programme** : intitulé (EVARS collège, EVARS lycée, prévention numérique…), responsables (personnes, missions du 6), niveaux, nombre de séances attendues par niveau.
- **Séance type** : niveau, rang, thème, **contrainte d'intervenant** vérifiée sur le cœur (discipline du service au 6, mission PsyEN ou infirmier, intervenant extérieur sous convention du 7, libre parmi les enseignants de la classe), mixité.
- **Positionnement** (séance type × classe) : intervenant, idée de séance, **date et créneau** (une séance du 10 de la classe, ou une séance ajoutée demandée au 10), salle (§14), statut `propose → valide | modifications_demandees | refuse | retire`, commentaire obligatoire pour les deux refus ; un refus **libère** la classe (ND-16) ; retrait motivé et tracé.
- **Suivi** : couverture par classe et par niveau (« 4ᵉ B : 2 séances sur 3 »), PDF par séance, export de bilan pour la direction (preuve de l'obligation, §6) ; la séance réalisée est inscrite au cahier de textes de la classe (socle 20) par commande.

| ID | Action | Acteur | Droit | Règles | Événement | Lot |
|---|---|---|---|---|---|---|
| AGD-300 | Voir les programmes et leur couverture | personnel | `agenda.programmes.lire` | — | — | AG6b |
| AGD-301 | Grille niveau × séance × classe | personnel | idem | classes du cœur (5) | — | AG6b |
| AGD-302 | Se positionner sur une séance pour une classe | PRF, INF, PSY | idem | contrainte vérifiée ; un positionnement actif par (séance, classe) | `agenda.programme_positionnement` | AG6b |
| AGD-303 | Positionner un collègue ou un intervenant extérieur | responsable | `agenda.programmes.gerer` (son programme) | auto-validé, auteur tracé | idem | AG6b |
| AGD-304 | Valider, demander des modifications, refuser | responsable | idem | commentaire obligatoire ; avis à l'intervenant | `agenda.programme_decision` | AG6b |
| AGD-305 | Réviser son idée | intervenant | soi | impossible après validation | — | AG6b |
| AGD-306 | Fixer la date et le créneau, réserver la salle | intervenant, responsable | soi / idem | port 10, §14 | — | AG6b |
| AGD-307 | Retirer un positionnement (motif) | intervenant, responsable | soi / idem | tracé ; libère la classe | — | AG6b |
| AGD-308 | Mes positionnements | intervenant | soi | — | — | AG6b |
| AGD-309 | PDF d'une séance, bilan du programme | responsable, DIR | idem | — | — | AG6b |
| AGD-310 | Paramétrer un programme (responsables, séances types, contraintes) | DIR, ADM | `agenda.programmes.parametrer` | **par programme** (ND-13) | — | AG6b |
| AGD-311 | Équipe d'une session (ambassadeurs élèves, enseignants) et planning PDF | organisateur | `agenda.sessions.gerer` | élèves ambassadeurs : autorisation du 18 si mineurs | — | AG6 |
| AGD-312 | Manifestation avec réponse et accompagnants (remise des diplômes) | DIR, SEC | `agenda.evenements.publier` | public calculé (2, 38) ; lien nominatif ; plafonds | `agenda.reponse` | AG6 |

---

## 13. Disponibilités, occupation et créneau commun

- **Disponibilités déclarées** : un personnel déclare ses plages d'accueil (récurrentes ou datées), leur modalité, leur lieu et qui peut les réserver (familles de ses élèves, élèves, collègues). Elles sont **ouvertes** ou **sur demande**.
- **Occupation** d'une personne = séances (10) ∪ événements et rendez-vous (19) ∪ absences (15) ∪ réservations de lieux qu'elle a faites pour elle-même ∪ plages « occupé » qu'elle a déclarées (y compris importées de son agenda personnel, en option, sans titre, Q-AG7) ∪ ses autres établissements ScolIA (D-013 : elle seule voit le détail ; les autres voient « occupé », M-540 du socle 10).
- **Créneau commun** (EDT-113 du socle 10) : pour une liste de personnes et éventuellement un lieu, une durée, une fenêtre et des préférences (pas le mercredi après-midi), le 19 calcule les plages libres et les classe (nombre de présents, heure, distance à un cours) ; « 8 sur 9 disponibles mardi 17 h ; M. Y a cours » sans dire où ni quoi. Lecture : `edt::plages_libres` + occupation du 19 + 15.
- **Aucun motif** ne traverse cette vue (R-AG6) ; elle est journalisée quand elle porte sur plus de 20 personnes (repérage d'un usage de surveillance) [R].

---

## 14. Réservation de lieux (reprise du module Next.js fiable, D-035)

### 14.1 Pourquoi au socle 19 [R, appliqué par défaut, D-031]

`SOCLES.md` place la réservation de salles au socle 44 (étage J) et le 10 dit « 44 possède ». Or (1) le module Next.js est **fiable et utilisé** et doit être remplacé en production à la rentrée 2027 (D-035, point 2), (2) une réservation est un **élément d'agenda** (même calcul d'occupation, mêmes séries, mêmes avis, même iCal) et la soirée parents-professeurs, les réunions et les sessions réservent des salles dans la même commande. Le **moteur de réservation de lieux** est donc au 19 ; le 44 garde le matériel, les prêts, les clés et la maintenance et pourra réserver du matériel avec le même moteur (ressource générique). Contradiction écrite au §32.

### 14.2 Ce qui est réservable

Salles et lieux du socle 5 marqués réservables ; **par défaut**, salles spéciales (CDI, salle polyvalente, salle informatique, laboratoire, gymnase, salle de réunion) réservables, salles de classe **non** réservables mais **réservables sur leurs heures libres** si l'établissement le choisit (S19-11). Chaque lieu peut porter une **règle** (`ent_agenda_lieu_regle`) : validation requise et valideur (personne ou mission), délai minimal, durée maximale, horizon (56 j par défaut, repris de NX-08, exemption du gestionnaire), qui peut réserver (tout personnel, enseignants d'une discipline, vie scolaire).

### 14.3 Réserver

1. **Rechercher** (AGD-231) : capacité, type, équipements, accessibilité, site, plage ; résultat trié (proximité de la salle habituelle, capacité juste suffisante).
2. **Contrôles serveur, à la date** : lieu ouvert et disponible (socle 5 : fermeture, indisponibilité, fenêtres d'un lieu extérieur) ; **aucune séance** dans le lieu (port `edt::occupe`, semaines A/B et remplacements compris ; corrige ND-11 par identifiant de salle) ; aucune réservation confirmée ou en attente qui chevauche (**contrainte d'exclusion en base**, corrige ND-01) ; dans les heures d'ouverture de l'établissement et hors jours fermés (corrige ND-03) ; horizon et délai ; capacité ≥ effectif annoncé (avertissement).
3. **Série** : chaque semaine, toutes les deux semaines, semaine A ou B, jusqu'à une date ; les vacances sont sautées ; **rapport par occurrence** (créée, en conflit avec quoi, hors horizon) et confirmation explicite si une partie échoue (corrige ND-02).
4. **Lieu à validation** : la réservation naît `demandee` (elle bloque la plage pour éviter la double demande) ; le valideur confirme ou refuse avec motif ; sans décision à J-1, rappel au valideur.
5. **Avis** : confirmation au bénéficiaire et à l'auteur s'il réserve pour autrui ; annulation avec motif et nombre d'occurrences ; agents prévenus de la mise en place demandée.

### 14.4 Priorités et conflits

Ordre déclaré (R-AG21) : examens nationaux (38) > conseils de classe et instances (23, 46) > réunions de l'établissement > réservations ordinaires. Une réservation de priorité supérieure sur une plage prise **ne l'écrase pas** : elle propose au gestionnaire de déplacer la réservation existante (salle de repli proposée), qui est avisée. Une fermeture ou une indisponibilité nouvelle (socle 5) met les réservations touchées en conflit (AGD-258).

### 14.5 Ce que chacun voit

Planning d'une salle : tout personnel voit **qui** a réservé (nom, comme dans Next.js) et l'intitulé ; le commentaire et l'e-mail ne sont visibles que du demandeur, du bénéficiaire et du gestionnaire (corrige ND-07) ; aucune liste d'élèves ; plage de lecture bornée (semaine, mois).

---

## 15. Familles : parents séparés, élèves mineurs et majeurs, accessibilité, langues

- **Qui peut prendre un rendez-vous pour un enfant** : un responsable avec lien actif et autorité parentale (socle 4) ; un tiers désigné (« personne à contacter ») **jamais** par défaut ; l'élève majeur pour lui-même.
- **Information de l'autre parent** (R-AG12) : par défaut, quand un rendez-vous concernant l'enfant est **confirmé** à l'initiative de l'établissement, **chaque** responsable avec autorité parentale est invité (D111-1 et suivants, §6) ; quand il est pris par un parent, l'autre parent du **même foyer** voit le rendez-vous et peut se joindre ; l'autre parent d'un **foyer distinct** est informé qu'un rendez-vous a été pris avec tel hôte (sans l'heure ni le lieu) et peut demander le sien ; **aucune information** si une interdiction, une adresse protégée ou une mesure concerne l'un d'eux (socle 4, R-AG12).
- **Élève mineur** : voit les rendez-vous auxquels il est invité ; ne voit pas les rendez-vous de ses parents le concernant, sauf réglage (S19-07) ; un rendez-vous restreint (PsyEN, infirmerie) qu'il prend lui-même n'est **pas** montré à ses parents (sauf obligation du socle propriétaire) [R, à vérifier §6].
- **Élève majeur** : il prend ses rendez-vous ; ses parents ne sont invités ou informés que s'il l'a accepté (socle 4) ; à la majorité, les rendez-vous futurs pris par les parents sont signalés à l'élève, qui choisit de les garder [R].
- **Accessibilité** : rendez-vous en rez-de-chaussée ou salle accessible si la personne l'a indiqué (préférence durable du socle 4) ; durée allongée ; **interprète** (INS-61 du socle 3 : jamais un enfant interprète pour une décision) ; langue des avis (socle 8).
- **Externes sans compte** : lien nominatif à usage limité (D-027 C/G) pour répondre, changer de créneau ou annuler ; pas d'énumération (jetons aléatoires, limitation de débit, réponse identique pour un lien inconnu ou expiré).

---

## 16. iCal, mobile, visio

- **iCal** : téléchargement ponctuel (existant) et **abonnement** par jeton révocable (socle 9, X15) ; le 19 fournit `agenda::flux(personne, plage)` déjà **minimisé** : titres calculés, aucun nom d'élève par défaut (réglage personnel « inclure les prénoms », S19-09), aucun motif, rendez-vous restreints en « occupé », UID stables, `SEQUENCE` incrémentée à chaque modification, `STATUS:CANCELLED` pour une annulation (R-AG25).
- **Mobile** (D-003, version quotidienne) : mon agenda, liste du jour, répondre à une invitation, prendre / déplacer / annuler un rendez-vous, « ma soirée », émargement, présence d'un rendez-vous, rechercher une salle libre et la réserver. Les campagnes et la configuration restent sur le web.
- **Visio** : la modalité visio d'un rendez-vous ou d'une réunion demande une salle au socle 33 ; avant le 33, un lien saisi n'est accepté que sur les domaines autorisés par l'établissement (S19-10) et n'est vu que des participants [R].

---

## 17. Cas limites (catalogue)

### 17.1 Familles et élèves

| N° | Cas | Comportement attendu |
|---|---|---|
| CAG-01 | Parent sous **interdiction de contact** avec l'autre parent ; tous deux prennent rendez-vous avec le PP le même soir | jamais à la suite ; aucun ne voit l'heure ni la salle de l'autre ; l'hôte voit « ne pas recevoir ensemble » ; l'accueil voit l'alerte (R-AG12) |
| CAG-02 | Parent avec **adresse protégée** | aucun lieu ni heure communiqué à l'autre ; même règle |
| CAG-03 | Parent qui **perd l'autorité parentale** entre la réservation et le rendez-vous | rendez-vous suspendu « à revoir », hôte prévenu, parent prévenu sans motif juridique (R-AG13) |
| CAG-04 | Enfant qui **change de classe** après la réservation | rendez-vous avec un enseignant qui ne l'a plus : signalé à l'hôte et à la famille, gardé si les deux le veulent |
| CAG-05 | Enfant qui **quitte l'établissement** | rendez-vous futurs annulés avec motif neutre ; sauf rendez-vous de départ |
| CAG-06 | Élève qui devient **majeur** entre la réservation et le rendez-vous | signalé à l'élève ; les parents restent invités s'il l'accepte |
| CAG-07 | Fratrie de trois enfants, huit professeurs, une soirée | parcours unique proposé, pas de chevauchement, déplacements entre bâtiments comptés |
| CAG-08 | Deux parents d'un même foyer réservent le même créneau à la même seconde | un seul gagne ; l'autre voit « déjà pris pour cet enfant par votre co-responsable » (RD-04) |
| CAG-09 | Famille sans compte (personne à créer, socle 4) | invitation par lien nominatif |
| CAG-10 | Famille allophone | langue d'avis, demande d'interprète, durée allongée |
| CAG-11 | Parent en fauteuil | salle accessible imposée |
| CAG-12 | Élève qui prend lui-même rendez-vous avec la PsyEN | parents non informés par le 19 ; règle du socle 36 |
| CAG-13 | Élève en garde alternée qui a une réunion de délégués à 17 h | agenda de l'enfant identique pour les deux parents (hors rendez-vous restreints) |
| CAG-14 | Famille qui ne vient jamais | « non venu » répété : signalé au PP et au CPE, jamais sanctionnant, jamais public |
| CAG-15 | Parent qui réserve douze créneaux « pour être sûr » puis n'en honore aucun | plafond par famille, un seul créneau par hôte et par enfant ; non-venues comptées pour le PP |
| CAG-16 | Tiers (beau-parent, grand-parent) sans autorité parentale | ne réserve pas ; peut **accompagner** un responsable qui l'ajoute : accompagnant **de droit** (circulaire 2013-142, §6.1), sauf interdiction |
| CAG-17 | Enfant placé (ASE), référent de l'aide sociale à l'enfance | le référent agit selon la décision enregistrée au socle 4 |
| CAG-18 | Élève en internat dont la famille habite loin | modalité téléphone ou visio proposée en tête |

### 17.2 Personnels, lieux, temps

| N° | Cas | Comportement attendu |
|---|---|---|
| CAG-20 | Professeur en **service partagé** sur deux établissements ScolIA | occupation des deux, détail visible de lui seul (D-013) |
| CAG-21 | Professeur **absent** le soir de la rencontre | rendez-vous « à reprogrammer », familles prévenues (U2 si jour même), proposition de créneaux ultérieurs ou d'un échange écrit |
| CAG-22 | Professeur à temps partiel qui ne travaille pas le jour de la soirée | signalé à la création ; il peut se déclarer présent ou ouvrir un autre créneau |
| CAG-23 | Remplaçant arrivé la veille | proposé comme hôte de la classe remplacée ; jamais d'héritage automatique des rendez-vous |
| CAG-24 | Salle réservée pour la soirée et fermée pour travaux la veille (socle 5) | réservation en conflit, salle de repli proposée, hôtes et familles avertis de la nouvelle salle |
| CAG-25 | **Changement d'heure** (dernier dimanche d'octobre) entre la création et la date | heures murales conservées (fuseau de l'établissement, socle 5) |
| CAG-26 | Établissement à La Réunion | aucune heure de Paris (RD-11) |
| CAG-27 | Neige : établissement fermé (socle 5) le jour de réunions et de rendez-vous | tous signalés, report proposé en lot, avis U2 |
| CAG-28 | Grève : hôte gréviste | il annule lui-même ; aucun motif « grève » visible des familles (motif public « empêchement ») |
| CAG-29 | Réunion récurrente pendant les vacances | sautée par défaut |
| CAG-30 | Réunion placée sur un cours d'un convoqué | signalée ; l'organisateur choisit (déplacer, demander la libération du cours au 10, excuser) |
| CAG-31 | Deux conseils de classe dans la même salle | refusé par la contrainte d'exclusion des réservations |
| CAG-32 | Réservation récurrente de la salle informatique toute l'année, puis un examen national (38) | l'examen prime : occurrences en conflit signalées et proposées au déplacement |
| CAG-33 | Agent qui doit préparer la salle polyvalente | mise en place visible dans son planning dès la confirmation |
| CAG-34 | Professeur qui quitte l'établissement en cours d'année | ses rendez-vous futurs signalés ; ses disponibilités fermées à sa date de fin (socle 6) |

### 17.3 Réunions et instances

| N° | Cas | Comportement attendu |
|---|---|---|
| CAG-40 | CA convoqué hors délai réglementaire | blocage avec forçage motivé de la direction (urgence) si le texte le prévoit (§6) |
| CAG-41 | Report d'une instance à moins du délai | nouveau délai calculé, alerte |
| CAG-42 | Quorum non atteint (réponses) | alerte J-2 ; seconde convocation préparée selon la règle du 46 |
| CAG-43 | Équipe éducative : la famille refuse de venir | réunion maintenue, refus tracé, compte rendu adressé |
| CAG-44 | ESS : enseignant référent externe (MDPH, académie) | externe invité par lien nominatif |
| CAG-45 | AESH convoquée à une ESS sur son temps de travail | convocation dans son temps de service, visible de son responsable sans motif |
| CAG-46 | Réunion confidentielle (équipe éducative) | titre neutre pour les non-participants ; participants calculés et figés à la convocation |
| CAG-47 | Compte rendu jamais déposé | rappel à l'organisateur J+7 ; tableau des instances |
| CAG-48 | Un convoqué demande à être représenté | procuration pour une instance du 46 seulement |

### 17.4 Sécurité et abus

| N° | Cas | Comportement attendu |
|---|---|---|
| CAG-50 | Énumération des liens de rendez-vous externes | jetons de 128 bits, limitation de débit, réponse identique pour inconnu / expiré |
| CAG-51 | Élève qui crée des rendez-vous à ses camarades | impossible : seul un personnel invite d'autres élèves |
| CAG-52 | Parent qui réserve pour un enfant qui n'est pas le sien (identifiant modifié) | « introuvable » (indiscernable), journalisé |
| CAG-53 | Professeur qui consulte l'agenda d'un collègue | « libre / occupé » seulement |
| CAG-54 | Abonnement iCal volé | contenu minimisé, jeton révocable, dernière utilisation visible (socle 9) |
| CAG-55 | Inondation de demandes de rendez-vous par une famille | plafond de demandes ouvertes (3 par hôte, S19-06) ; pas de notification au-delà |
| CAG-56 | Personnel d'un autre établissement | aucune donnée, sur toutes les routes (RLS stricte + tests de matrice) |
| CAG-57 | Rendez-vous d'un élève avec la PsyEN lu par l'IA d'un professeur | refus : nature restreinte exclue du registre IA |
| CAG-58 | Accueil qui voit la liste du jour | aucun motif, aucune nature révélatrice |
| CAG-59 | Lien visio saisi vers un site piégé | domaines autorisés seulement (S19-10) |
| CAG-60 | Réservation de salle au nom d'un collègue | demandeur = soi ; pour autrui, droit `agenda.lieux.gerer` et journal |

---

## 18. Intelligence artificielle (contrat IA, `IA_COEUR.md` §5)

### 18.1 Ce que l'IA fait [R]
- Propose des **plages** (créneau commun, soirée parents-professeurs, parcours d'une famille) avec leur explication typée (« 8/9 libres ; pas le mercredi »), sans jamais nommer la contrainte d'un collègue (D-033 B).
- Prépare une **soirée** complète (hôtes, plages, salles, vagues) en simulation, que l'adjoint ajuste et confirme.
- Rédige une **convocation**, une **invitation** ou un **rappel** à partir des trames, que l'auteur relit.
- Résume un **compte rendu** saisi (puces, décisions, suites) à relire.
- Répond à « quand ai-je rendez-vous avec le professeur de maths de Léa ? », « quelle salle est libre jeudi à 14 h pour 30 ? ».

### 18.2 Ce que l'IA ne fait pas
- Ne réserve, n'annule, ne convoque, ne publie, n'envoie **rien** sans confirmation humaine (N2 au minimum ; N3 pour une diffusion à plus de 50 personnes).
- Ne lit **jamais** un rendez-vous de nature restreinte (santé, PsyEN, social, discipline, protection de l'enfance), ni un compte rendu de réunion autour d'un élève (R-AG7).
- Ne donne aucun droit supplémentaire : elle lit l'agenda que l'utilisateur lit.

### 18.3 Capacités déclarées (registre `ent-ia`)

| Identifiant | Nature | Droit | Sensibilité | Confirmation |
|---|---|---|---|---|
| `agenda.mon_agenda.lire` | lecture | session | C1 | — |
| `agenda.libre_occupe.lire` | lecture | `agenda.libre_occupe` | C1 | — |
| `agenda.salles_libres.lire` | lecture | `agenda.lieux.reserver` | C0 | — |
| `agenda.creneau_commun.proposer` | simulation | `agenda.reunions.organiser` | C1 | — |
| `agenda.soiree.proposer` | simulation | `agenda.campagnes.gerer` | C1 | — |
| AGD-150 proposer un rendez-vous | commande | `agenda.rdv.proposer` | C1 | N2 |
| AGD-160 réserver un créneau | commande | lien / soi | C1 | N2 |
| AGD-114 publier un événement | commande | `agenda.evenements.publier` | C1 | N3 au-delà de 50 destinataires |
| AGD-240 réserver une salle | commande | `agenda.lieux.reserver` | C0 | N1 |

---

## 19. Actions métier (AGD-100 à AGD-291)

Colonnes : identifiant · action · acteur · droit (§21.1) · règles · événement · lot. Acteurs : DIR direction, ADJ adjoint, SEC secrétariat, PRF professeur, PP professeur principal, CPE, VS vie scolaire, PSY PsyEN, INF infirmier, AS assistant social, AESH, ACC accueil, AGT agent, PAR responsable, ELV élève mineur, ELM élève majeur, EXT externe, ADM administrateur.

### 19.1 Agenda et vues

| ID | Action | Acteur | Droit | Règles | Événement | Lot |
|---|---|---|---|---|---|---|
| AGD-100 | Voir mon agenda (jour, semaine, mois, liste) toutes sources | tous | session | R-AG1, R-AG5 | — | AG1 |
| AGD-101 | Filtrer par source, par enfant, par type | tous | session | — | — | AG1 |
| AGD-102 | Voir l'agenda d'un enfant | PAR, ELM (soi) | lien | R-AG12 | — | AG1 |
| AGD-103 | Voir l'agenda de l'établissement (partie publique) | tous | session | audience | — | AG1 |
| AGD-104 | Voir l'agenda d'une classe ou d'un groupe (non nominatif) | personnel | `agenda.lire_structure` | D-022 pour le nominatif | — | AG1 |
| AGD-105 | Voir le libre / occupé d'une liste de personnes | personnel | `agenda.libre_occupe` | R-AG6 ; journal au-delà de 20 | — | AG2 |
| AGD-106 | Voir le planning des salles | personnel, AGT | `agenda.lieux.lire` | sans élèves | — | AG3 |
| AGD-107 | Liste du jour des rendez-vous et visiteurs | ACC, VS, SEC | `agenda.accueil.lire` | sans motif ; alerte interdiction | — | AG5 |
| AGD-108 | Imprimer ou exporter une vue (PDF) | tous | idem vue | minimisé | — | AG1 |
| AGD-109 | Télécharger un iCal ponctuel | tous | idem vue | R-AG25 | — | AG1 |
| AGD-110 | Fournir le flux d'abonnement au socle 9 | système | — | R-AG25 | — | AG10 |
| AGD-111 | Rechercher dans mon agenda | tous | session | — | — | AG1 |
| AGD-112 | Voir le calendrier prévisionnel de l'année | personnel | `agenda.lire_structure` | — | — | AG1 |

### 19.2 Événements et réunions

| ID | Action | Acteur | Droit | Règles | Événement | Lot |
|---|---|---|---|---|---|---|
| AGD-113 | Créer un événement (brouillon) | personnel | `agenda.evenements.creer` | type, plage, lieu | — | AG1 |
| AGD-114 | Publier un événement à une audience | DIR, ADJ, CPE, PP (sa classe) | `agenda.evenements.publier` | R-AG8 audience sous les droits de l'auteur ; aperçu ; confirmation | `agenda.evenement_publie` | AG1 |
| AGD-115 | Modifier un événement publié | organisateur | idem | avis aux invités si date, heure, lieu changent | `agenda.evenement_modifie` | AG1 |
| AGD-116 | Reporter | organisateur | idem | R-AG10 | `agenda.evenement_reporte` | AG1 |
| AGD-117 | Annuler (motif public) | organisateur | idem | libère les lieux (R-AG19) | `agenda.evenement_annule` | AG1 |
| AGD-118 | Dupliquer | organisateur | idem | — | — | AG1 |
| AGD-119 | Créer une série récurrente | organisateur | idem | §9.3 | `agenda.serie_creee` | AG1 |
| AGD-120 | Modifier une occurrence / les suivantes / toutes | organisateur | idem | détachement | `agenda.evenement_modifie` | AG1 |
| AGD-121 | Reconduire le calendrier de l'année précédente | DIR, ADJ | `agenda.calendrier.gerer` | décalé sur le calendrier du 5 ; brouillons | — | AG7 |
| AGD-122 | Inviter des participants calculés (équipe, classe, mission, instance) | organisateur | idem | résolus à la publication, figés pour une réunion restreinte | — | AG7 |
| AGD-123 | Inviter des participants nominatifs | organisateur | idem | — | — | AG7 |
| AGD-124 | Inviter un externe | organisateur | idem | lien nominatif | — | AG7 |
| AGD-125 | Convoquer (obligatoire) | organisateur | `agenda.reunions.convoquer` | délai §6 (R-AG10) ; accusé | `agenda.convocation_envoyee` | AG7 |
| AGD-126 | Rédiger l'ordre du jour | organisateur | idem | points ordonnés | — | AG7 |
| AGD-127 | Joindre des pièces à un point | organisateur | idem | socle 7 | — | AG7 |
| AGD-128 | Répondre à une invitation (oui / non / peut-être, motif) | invité | soi | — | `agenda.reponse` | AG1 |
| AGD-129 | Répondre au coupon (question fermée) | invité | soi | M-355 du socle 8 | — | AG1 |
| AGD-130 | Relancer les non-répondants | organisateur | idem | socle 8 | — | AG7 |
| AGD-131 | Voir les réponses et le décompte | organisateur | idem | — | — | AG1 |
| AGD-132 | Calculer le quorum | organisateur | idem | règle du 46 | — | AG7 |
| AGD-133 | Désigner un secrétaire de séance | organisateur | idem | — | — | AG7 |
| AGD-134 | Émarger (tablette, téléphone, feuille pointée) | organisateur, secrétaire | idem | présence, retard, départ | `agenda.presence_saisie` | AG7 |
| AGD-135 | Imprimer la feuille d'émargement | organisateur | idem | — | — | AG7 |
| AGD-136 | Déposer ou rédiger le compte rendu | organisateur, secrétaire | idem | socle 7 | `agenda.compte_rendu_depose` | AG7 |
| AGD-137 | Valider et diffuser le compte rendu | organisateur | idem | socle 8 | `agenda.compte_rendu_diffuse` | AG7 |
| AGD-138 | Noter les suites décidées (qui, quoi, échéance) | organisateur | idem | à-faire (29) | `agenda.suite_creee` | AG7 |
| AGD-139 | Clore la réunion | organisateur | idem | — | `agenda.evenement_clos` | AG7 |
| AGD-140 | Demander la libération de cours pour une réunion | ADJ | `agenda.reunions.organiser` | demande au 10 (R-AG16) | — | AG7 |
| AGD-141 | Ouvrir une réunion à la demande d'un socle propriétaire (23, 13, 46, 35, 42) | système | droit du demandeur | contenu chez le propriétaire | `agenda.evenement_publie` | AG7 |
| AGD-142 | Tableau des instances (délais, quorum, comptes rendus) | DIR | `agenda.instances.lire` | — | — | AG7 |
| AGD-143 | Ajouter l'événement à mon agenda personnel (iCal) | invité | soi | R-AG25 | — | AG1 |
| AGD-144 | Déclarer un empêchement après avoir répondu oui | invité | soi | avis à l'organisateur | `agenda.reponse` | AG1 |
| AGD-145 | Demander une salle visio pour une réunion | organisateur | idem | socle 33 | — | AG10 |
| AGD-146 | Inscrire une réunion régulière comme contrainte de l'EDT | ADJ | `edt.contraintes` (10) | port du 10 | — | AG7 |
| AGD-147 | Voir l'historique d'un événement | organisateur, DIR | idem | journal | — | AG1 |
| AGD-148 | Retirer un participant (motif) | organisateur | idem | avis | — | AG7 |
| AGD-149 | Transférer l'organisation à un collègue | organisateur | idem | — | — | AG7 |

### 19.3 Rendez-vous individuels

| ID | Action | Acteur | Droit | Règles | Événement | Lot |
|---|---|---|---|---|---|---|
| AGD-150 | Proposer un rendez-vous à une famille ou un élève (1 à 5 créneaux) | personnel | `agenda.rdv.proposer` | participants selon R-AG12 ; créneaux libres | `agenda.rdv_propose` | AG4 |
| AGD-151 | Demander un rendez-vous (motif court) | PAR, ELV, ELM | lien / soi | plafond S19-06 | `agenda.rdv_demande` | AG4 |
| AGD-152 | Demander depuis un message (carte du 16) | PAR, personnel | idem | fil relié | `agenda.rdv_demande` | AG4 |
| AGD-153 | Répondre à une demande : proposer des créneaux | hôte | soi | — | `agenda.rdv_propose` | AG4 |
| AGD-154 | Refuser une demande (motif, autre interlocuteur, échange écrit) | hôte | soi | — | `agenda.rdv_refuse` | AG4 |
| AGD-155 | Transmettre une demande à un collègue (PP → CPE) | hôte | soi | avis au demandeur | `agenda.rdv_transmis` | AG4 |
| AGD-156 | Choisir un des créneaux proposés | participant | soi | concurrence | `agenda.rdv_confirme` | AG4 |
| AGD-157 | Demander d'autres créneaux | participant | soi | — | — | AG4 |
| AGD-158 | Réserver directement dans les disponibilités ouvertes d'un hôte | PAR, ELV, ELM | lien / soi | délai S19-01 ; qui peut réserver | `agenda.rdv_confirme` | AG4 |
| AGD-159 | Créer un rendez-vous déjà convenu (téléphone, couloir) | personnel | `agenda.rdv.proposer` | confirmé d'office ; participants avisés | `agenda.rdv_confirme` | AG4 |
| AGD-160 | Réserver un créneau d'une campagne | PAR, ELM | lien / soi | §11.3 | `agenda.rdv_confirme` | AG0/AG5 |
| AGD-161 | Changer de créneau | participant, hôte | soi | délai ; même offre | `agenda.rdv_reporte` | AG4 |
| AGD-162 | Annuler (participant) | participant | soi | délai S19-02 | `agenda.rdv_annule` | AG0/AG4 |
| AGD-163 | Annuler (hôte) avec motif et nouveaux créneaux | hôte | soi | U2 si jour même | `agenda.rdv_annule` | AG4 |
| AGD-164 | Prévenir d'un empêchement ou d'un retard | participant, hôte | soi | avis | `agenda.rdv_empechement` | AG4 |
| AGD-165 | Ajouter un participant (l'autre parent, l'élève, un interprète, un collègue) | hôte | soi | R-AG12 | `agenda.rdv_participant_ajoute` | AG4 |
| AGD-166 | Se joindre à un rendez-vous de son enfant (autre parent même foyer) | PAR | lien | R-AG12 | `agenda.rdv_participant_ajoute` | AG4 |
| AGD-167 | Demander un interprète ou une salle accessible | participant | soi | — | — | AG4 |
| AGD-168 | Choisir la modalité (sur place, téléphone, visio) | hôte ; participant (demande) | soi | modalités acceptées | — | AG4 |
| AGD-169 | Marquer honoré / non venu | hôte | soi | — | `agenda.rdv_honore` / `_non_venu` | AG0/AG4 |
| AGD-170 | Noter la suite (deux lignes, privée au service) | hôte | soi | Q-AG4 | — | AG4 |
| AGD-171 | Programmer le rendez-vous suivant | hôte | soi | — | `agenda.rdv_propose` | AG4 |
| AGD-172 | Voir l'historique des rendez-vous d'un élève avec mon service | hôte | droit du service | pas les autres services ; jamais les restreints d'un autre | — | AG4 |
| AGD-173 | Voir mes rendez-vous à venir et passés | tous | soi | — | — | AG0/AG4 |
| AGD-174 | Recevoir un rappel la veille et 1 h avant | participant | — | socle 8 | — | AG0/AG4 |
| AGD-175 | Ouvrir la fiche de rendez-vous (contexte élève) | hôte | ses droits existants | R-AG9 | — | AG4 |
| AGD-176 | Convoquer une famille (rendez-vous obligatoire : discipline, assiduité) | CPE, DIR | `agenda.rdv.convoquer` | trame, accusé ; titre neutre | `agenda.rdv_convocation` | AG4 |
| AGD-177 | Rendez-vous à plusieurs hôtes (PP + CPE + direction) | hôte | `agenda.rdv.proposer` | créneau commun des hôtes | `agenda.rdv_propose` | AG4 |
| AGD-178 | Lien nominatif pour un externe (répondre, changer, annuler) | système | — | CAG-50 | — | AG4 |
| AGD-179 | Signaler à la famille qu'un cours sera manqué pour un rendez-vous d'élève | système | — | port 10 ; avis au professeur concerné sans motif | — | AG4 |
| AGD-180 | Reprogrammer en lot les rendez-vous d'un hôte absent | hôte, organisateur | `agenda.rdv.gerer_service` | R-AG17 | `agenda.rdv_reporte` | AG4 |
| AGD-181 | Déléguer la gestion de mes rendez-vous (secrétariat de direction) | DIR, CPE | délégation (172) | journal | — | AG4 |
| AGD-182 | Gérer les rendez-vous d'un service (vie scolaire, direction) | SEC, VS | `agenda.rdv.gerer_service` | ceux du service seulement | — | AG4 |
| AGD-183 | Rendez-vous restreint (nature sensible) | PSY, INF, AS, CPE, DIR | droit du socle propriétaire | R-AG7 | `agenda.rdv_confirme` (sans nature) | AG4 |

### 19.4 Disponibilités et créneau commun

| ID | Action | Acteur | Droit | Règles | Événement | Lot |
|---|---|---|---|---|---|---|
| AGD-184 | Déclarer mes disponibilités d'accueil (récurrentes ou datées) | personnel | soi | qui peut réserver ; modalité ; lieu | `agenda.disponibilite_modifiee` | AG2 |
| AGD-185 | Ouvrir ou fermer mes disponibilités à la réservation directe | personnel | soi | — | idem | AG2 |
| AGD-186 | Déclarer une plage « occupé » personnelle (sans titre) | personnel | soi | — | — | AG2 |
| AGD-187 | Importer mon agenda personnel en plages occupées (option) | personnel | soi | Q-AG7 ; sans titre | — | AG10 |
| AGD-188 | Trouver un créneau commun (personnes, lieu, durée, fenêtre) | personnel | `agenda.libre_occupe` | R-AG6 | — | AG2 |
| AGD-189 | Proposer une réunion sur le créneau trouvé | organisateur | `agenda.evenements.creer` | — | — | AG2 |
| AGD-190 | Voir les disponibilités d'un collègue (plages d'accueil) | personnel | `agenda.libre_occupe` | sans motif | — | AG2 |
| AGD-191 | Fermer les disponibilités à la date de départ (socle 6) | système | — | — | — | AG2 |

### 19.5 Campagnes de rendez-vous et rencontres parents-professeurs

| ID | Action | Acteur | Droit | Règles | Événement | Lot |
|---|---|---|---|---|---|---|
| AGD-192 | Créer une campagne (nature, public, dates, plage, lieux) | ADJ, DIR, PP (sa classe) | `agenda.campagnes.gerer` | — | — | AG5 |
| AGD-193 | Calculer les hôtes depuis les services | système | — | §11.1 ; signal service partagé | — | AG5 |
| AGD-194 | Ajouter ou retirer un hôte (CPE, PsyEN, direction) | organisateur | idem | — | — | AG5 |
| AGD-195 | Générer les plages des hôtes | organisateur | idem | EDT, absences, disponibilités | — | AG5 |
| AGD-196 | Ajuster les plages d'un hôte (bloquer, ajouter, pause) | organisateur, hôte (les siennes) | idem / soi | — | — | AG5 |
| AGD-197 | Attribuer les salles automatiquement | organisateur | idem + `agenda.lieux.reserver` | réservations de lieux | `agenda.lieu_reserve` | AG5 |
| AGD-198 | Définir les vagues (familles à voir, fratries, parents séparés, tous) | organisateur | idem | — | — | AG5 |
| AGD-199 | Importer les « rencontres souhaitées » du conseil de classe | système | — | port du 23 | — | AG5 |
| AGD-200 | Marquer une famille « à voir » (PP, enseignant) | PP, PRF | soi (ses élèves) | jamais visible de la famille comme une étiquette ; elle reçoit une invitation | — | AG5 |
| AGD-201 | Aperçu de la campagne (ratio, conflits, capacité) | organisateur | idem | — | — | AG5 |
| AGD-202 | Ouvrir la campagne (diffusion par vagues) | organisateur | idem | confirmation | `agenda.campagne_ouverte` | AG5 |
| AGD-203 | Prolonger, suspendre, fermer les réservations | organisateur | idem | — | `agenda.campagne_modifiee` | AG5 |
| AGD-204 | Voir les créneaux par enfant (famille) | PAR, ELM | lien / soi | RP1-27 | — | AG0/AG5 |
| AGD-205 | Calculer un parcours sans attente pour toute la fratrie | PAR | lien | §11.2 | — | AG5 |
| AGD-206 | Accepter le parcours proposé (réservations en une transaction) | PAR | lien | tout ou rien | `agenda.rdv_confirme` × n | AG5 |
| AGD-207 | S'inscrire en liste d'attente chez un hôte | PAR | lien | — | `agenda.attente_inscrite` | AG5 |
| AGD-208 | Proposer la place libérée au premier de la liste | système | — | délai d'acceptation | `agenda.attente_servie` | AG5 |
| AGD-209 | Planning de la soirée par salle et par hôte | organisateur | idem | — | — | AG0/AG5 |
| AGD-210 | Mes rendez-vous de la soirée (hôte) avec fiches | hôte | soi | R-AG9 | — | AG5 |
| AGD-211 | Signaler mon retard (hôte) : avis aux suivants | hôte | soi | — | `agenda.rdv_empechement` | AG5 |
| AGD-212 | Ma soirée (famille) : ordre, salles, plan, retards | PAR | lien | — | — | AG5 |
| AGD-213 | Tableau du soir (retards, absents, salles) | organisateur | idem | — | — | AG5 |
| AGD-214 | Clore la campagne et statistiques sans nom | organisateur | idem | — | `agenda.campagne_close` | AG5 |
| AGD-215 | Familles sans rendez-vous (PP, CPE : leurs élèves) | PP, CPE | soi | proposer un rendez-vous individuel | — | AG5 |
| AGD-216 | Imprimer les plannings (par salle, par hôte, par famille) | organisateur | idem | — | — | AG5 |
| AGD-217 | Dupliquer une campagne de l'année précédente | organisateur | idem | — | — | AG5 |
| AGD-218 | Ouvrir une campagne pour un autre socle (3 inscription, 37 orientation, 34 santé) | système | droit du demandeur | objet chez le propriétaire | `agenda.campagne_ouverte` | AG5 |
| AGD-219 | Réservation provisoire qui expire (socle 3) | système | — | durée réglable | `agenda.rdv_expire` | AG5 |
| AGD-220 | Reconfirmation à J-N | participant | soi | sans réponse : rappel, puis libération selon réglage | `agenda.rdv_reconfirme` | AG5 |

### 19.6 Sessions collectives

| ID | Action | Acteur | Droit | Règles | Événement | Lot |
|---|---|---|---|---|---|---|
| AGD-221 | Créer des sessions à capacité (atelier, visite, réunion par vagues) | personnel | `agenda.sessions.gerer` | public, capacité | — | AG6 |
| AGD-222 | S'inscrire ou inscrire son enfant | PAR, ELV, ELM, EXT | lien / soi | capacité, public | `agenda.session_inscription` | AG6 |
| AGD-223 | Se désinscrire | inscrit | soi | délai | `agenda.session_desinscription` | AG6 |
| AGD-224 | Liste d'attente de session | inscrit | soi | — | idem | AG6 |
| AGD-225 | Pointer les présents (QR ou liste) | organisateur, ACC | idem | — | `agenda.session_presence` | AG6 |
| AGD-226 | Message de suivi différé aux présents | organisateur | idem | consentement (3) | — | AG6 |
| AGD-227 | Annuler une session (avis, autre session proposée) | organisateur | idem | — | `agenda.session_annulee` | AG6 |
| AGD-228 | Répartir automatiquement des élèves en sessions (vœux, capacité) | organisateur | idem | simulation puis confirmation | — | AG6 |
| AGD-229 | Exporter la liste des inscrits | organisateur | idem | minimisée | — | AG6 |
| AGD-230 | Convocation d'élèves sortis de cours (bilan, vaccination) | INF, organisateur | droit du socle propriétaire | port 10 ; avis au professeur sans motif | — | AG6 |

### 19.7 Réservation de lieux

| ID | Action | Acteur | Droit | Règles | Événement | Lot |
|---|---|---|---|---|---|---|
| AGD-231 | Rechercher une salle libre (capacité, type, équipement, site, accessibilité, plage) | personnel | `agenda.lieux.lire` | EDT + réservations + indisponibilités | — | AG3 |
| AGD-232 | Voir le planning d'une salle (jour, semaine) | personnel, AGT | `agenda.lieux.lire` | sans élèves | — | AG3 |
| AGD-233 | Réserver une salle (plage, motif, effectif, mise en place) | personnel | `agenda.lieux.reserver` | R-AG18, R-AG20 | `agenda.lieu_reserve` | AG3 |
| AGD-234 | Réserver en récurrence | personnel | idem | occurrences matérialisées ; conflits listés | idem | AG3 |
| AGD-235 | Demander une salle à validation (gymnase, salle polyvalente) | personnel | idem | validateur du lieu | `agenda.lieu_demande` | AG3 |
| AGD-236 | Valider ou refuser une demande (motif) | valideur | `agenda.lieux.valider` | — | `agenda.lieu_decide` | AG3 |
| AGD-237 | Modifier ou annuler ma réservation | demandeur | soi | — | `agenda.lieu_annule` | AG3 |
| AGD-238 | Annuler la réservation d'un autre (motif, avis) | gestionnaire | `agenda.lieux.gerer` | priorité (examen, conseil) | idem | AG3 |
| AGD-239 | Déplacer une réservation en conflit (fermeture, examen) | gestionnaire | idem | proposition de salle de repli | `agenda.lieu_deplace` | AG3 |
| AGD-240 | Réserver pour un événement ou une campagne (dans la même commande) | organisateur | idem | — | `agenda.lieu_reserve` | AG3 |
| AGD-241 | Demander l'échange de salle avec un cours (au socle 10) | personnel | `agenda.lieux.reserver` | demande, jamais écriture | — | AG3 |
| AGD-242 | Paramétrer les règles d'un lieu (validation, délai, durée maximale, horizon, qui peut réserver) | ADM, DIR | `agenda.lieux.parametrer` | — | — | AG3 |
| AGD-243 | Désigner le valideur d'un lieu (personne ou mission) | ADM, DIR | idem | — | — | AG3 |
| AGD-244 | Voir la mise en place demandée (agents) | AGT | `agenda.lieux.lire` | — | — | AG3 |
| AGD-245 | Taux d'occupation des salles (sans nom) | DIR, ADJ | `agenda.lieux.lire` | pilotage (47) | — | AG3 |
| AGD-246 | Réserver pour un externe (location, association) | gestionnaire | `agenda.lieux.gerer` | convention (7) ; facturation (24) hors V1 | `agenda.lieu_reserve` | AG11 |
| AGD-247 | Imprimer l'affichette de la salle (planning du jour) | personnel | idem | sans nom d'élève | — | AG3 |

### 19.8 Paramétrage, reprise, système

| ID | Action | Acteur | Droit | Règles | Événement | Lot |
|---|---|---|---|---|---|---|
| AGD-248 | Paramétrer les types d'événements locaux | ADM | `agenda.parametrer` | catalogue fourni d'abord | — | AG1 |
| AGD-249 | Paramétrer les trames (convocation, invitation, rappel) | ADM, DIR | idem | 3 trames par type (D-023) | — | AG7 |
| AGD-250 | Paramétrer les délais et plafonds (S19-01 à S19-06) | ADM | idem | bornés | — | AG4 |
| AGD-251 | Paramétrer les modalités et domaines visio autorisés | ADM | idem | S19-10 | — | AG10 |
| AGD-252 | Réagir à la fin d'un lien responsable / perte d'autorité | système | — | R-AG13 | `agenda.rdv_a_revoir` | AG0/AG4 |
| AGD-253 | Réagir au changement de classe ou de groupe | système | — | CAG-04 | idem | AG4 |
| AGD-254 | Réagir au départ d'un élève | système | — | CAG-05 | `agenda.rdv_annule` | AG4 |
| AGD-255 | Réagir à la majorité | système | — | CAG-06 | — | AG4 |
| AGD-256 | Réagir à une interdiction nouvelle (socle 4) | système | — | R-AG12 : séparation des rendez-vous futurs | `agenda.rdv_a_revoir` | AG4 |
| AGD-257 | Réagir à l'absence d'un personnel (15) | système | — | R-AG17 | idem | AG4 |
| AGD-258 | Réagir à une fermeture ou indisponibilité de lieu (5) | système | — | CAG-24 | `agenda.lieu_en_conflit` | AG3 |
| AGD-259 | Réagir à une modification d'EDT (10) | système | — | CAG-30 | — | AG2 |
| AGD-260 | Réagir au départ d'un personnel (6) | système | — | CAG-34 | — | AG2 |
| AGD-261 | Envoyer les rappels (veille, 1 h) par établissement | système | — | RD-06 | — | AG0 |
| AGD-262 | Expirer les réservations provisoires et servir les listes d'attente | système | — | — | — | AG5 |
| AGD-263 | Purger selon la conservation (§21.3) | système | — | — | — | AG12 |
| AGD-264 | Reprendre les salles et réservations futures de Next.js | ADM | `agenda.reprise` | §26 | — | AG11 |
| AGD-265 | Reprendre les rendez-vous d'inscription futurs (avec le 3) | ADM | idem | §26 | — | AG11 |
| AGD-266 | Reprendre les rencontres de la migration 192 | système | — | §26 | — | AG5 |
| AGD-267 | Journal d'un rendez-vous, d'un événement, d'une réservation | DIR, ADM | `agenda.journal.lire` | — | — | AG1 |
| AGD-268 | Indicateurs sans nom pour le pilotage (47) | DIR | `agenda.instances.lire` | — | — | AG12 |
| AGD-269 | Contrat IA : capacités déclarées et jeu d'évaluation | système | — | §18 | — | AG12 |

### 19.9 Familles, élèves, mobile

| ID | Action | Acteur | Droit | Règles | Événement | Lot |
|---|---|---|---|---|---|---|
| AGD-270 | Voir les rendez-vous et invitations de chacun de mes enfants | PAR | lien | R-AG12 | — | AG4 |
| AGD-271 | Voir les réunions des familles de la classe de mon enfant | PAR | lien | audience | — | AG1 |
| AGD-272 | Recevoir l'information « un rendez-vous a été pris par l'autre parent » | PAR | lien | R-AG12 | — | AG4 |
| AGD-273 | Demander mon propre rendez-vous (foyers distincts) | PAR | lien | R-AG12 | — | AG4 |
| AGD-274 | Élève : voir mes rendez-vous et réunions (délégués, CVL) | ELV, ELM | soi | — | — | AG4 |
| AGD-275 | Élève majeur : choisir qui est informé de mes rendez-vous | ELM | soi | socle 4 | — | AG4 |
| AGD-276 | Mobile : mon agenda, liste du jour | tous | idem | — | — | AG8 |
| AGD-277 | Mobile : répondre, prendre, déplacer, annuler | tous | idem | — | — | AG8 |
| AGD-278 | Mobile : ma soirée | PAR, PRF | idem | — | — | AG8 |
| AGD-279 | Mobile : émargement et présence | organisateur, hôte | idem | — | — | AG8 |
| AGD-280 | Mobile : salle libre et réservation rapide | personnel | idem | — | — | AG8 |
| AGD-281 | Notification push de rappel avec ajout à l'agenda du téléphone | tous | — | R-AG25 | — | AG8 |
| AGD-282 | Indiquer une préférence durable (salle accessible, interprète, téléphone) | PAR, ELM | soi | socle 4 | — | AG4 |
| AGD-283 | Prévenir « je suis en retard » | participant | soi | — | — | AG5 |
| AGD-284 | Accueil : enregistrer l'arrivée d'un visiteur attendu | ACC | `agenda.accueil.lire` | avis à l'hôte | `agenda.visiteur_arrive` | AG5 |
| AGD-285 | Accueil : visiteur sans rendez-vous, demander à l'hôte s'il peut recevoir | ACC | idem | — | — | AG5 |
| AGD-286 | Externe : prendre un rendez-vous public (inscription, portes ouvertes) | EXT | lien public | socle 3 ; anti-abus | `agenda.rdv_confirme` | AG6 |
| AGD-287 | Externe : changer ou annuler par lien | EXT | lien nominatif | CAG-50 | — | AG6 |
| AGD-288 | Externe : ajouter à son agenda (fichier .ics joint au lien) | EXT | lien nominatif | — | — | AG6 |
| AGD-289 | Élève : être prévenu du cours manqué et de la salle du rendez-vous | ELV | soi | — | — | AG4 |
| AGD-290 | Personnel : ses rendez-vous dans ses à-faire (29) | personnel | soi | — | — | AG4 |
| AGD-291 | Famille : à-faire « répondre à l'invitation », « prendre rendez-vous » (29) | PAR | lien | — | — | AG4 |

---

## 20. Règles transverses (R-AG1 à R-AG34 ; R-AG35 à R-AG45 au §33.3 ; R-AG46 à R-AG48 au §6.10) [R]

| N° | Règle |
|---|---|
| R-AG1 | Rien n'est recopié d'un autre socle ; l'agenda lit les sources à la date ; port indisponible = mention explicite. |
| R-AG2 | Un seul moteur de créneaux ; capacité 1 = rendez-vous, N = session ; les socles consommateurs (3, 34, 37, 42) passent par ses commandes et gardent leur objet. |
| R-AG3 | Tout rendez-vous, réservation de créneau ou de lieu a un fil de faits en ajout seul (qui, quand, quoi) ; une annulation ne supprime ni n'efface rien (corrige RD-01). |
| R-AG4 | Toute réservation est une commande serveur transactionnelle et concurrente-sûre : mise à jour conditionnelle ou contrainte d'exclusion ; une violation d'unicité devient un conflit lisible, jamais une erreur 500 (RD-04). |
| R-AG5 | Le titre et le motif visibles d'un non-participant sont calculés depuis la nature ; aucun motif saisi n'est montré hors des participants. |
| R-AG6 | Entre personnels, seule l'occupation (« libre / occupé ») est partagée, jamais le motif ni l'objet. |
| R-AG7 | Les natures restreintes (santé, PsyEN, social, discipline, protection de l'enfance, équipe éducative) n'exposent que la logistique ; jamais à l'IA, jamais en iCal autrement qu'« occupé » ; contenu chez le propriétaire. |
| R-AG8 | Une audience est résolue sous les droits de l'auteur à la date de publication ; la diffusion passe par le socle 8 avec aperçu et confirmation. |
| R-AG9 | Un rendez-vous ne donne aucun droit nouveau sur l'élève ; la fiche ne montre que ce que l'hôte lit déjà. |
| R-AG10 | Le délai de convocation d'une instance ou d'une réunion réglementée est calculé (§6) ; un envoi hors délai est bloqué sauf forçage motivé là où le texte prévoit l'urgence. |
| R-AG11 | Toutes les heures sont murales dans le fuseau de l'établissement (socle 5) ; aucune heure de Paris en dur (RD-11). |
| R-AG12 | Parents : même foyer → rendez-vous commun, l'autre parent voit et peut se joindre ; foyers distincts → chacun peut avoir le sien, l'autre est informé de l'existence sans heure ni lieu ; **interdiction, adresse protégée ou mesure (socle 4) → aucune information croisée, jamais à la suite, hôte et accueil avertis sans motif**. |
| R-AG13 | Fin de lien, perte d'autorité parentale, départ de l'élève : les rendez-vous futurs concernés sont suspendus « à revoir » ou annulés avec motif neutre ; jamais maintenus en silence. |
| R-AG14 | Élève majeur : il décide ; ses parents ne sont invités ou informés que selon son choix (socle 4). |
| R-AG15 | Une réservation dans le passé est refusée ; délais minimaux de réservation et d'annulation par offre. |
| R-AG16 | Le 19 n'écrit jamais dans l'EDT : il demande une modification au socle 10. |
| R-AG17 | Absence d'un hôte : ses rendez-vous de la plage sont « à reprogrammer » ; aucun héritage automatique par un remplaçant. |
| R-AG18 | Deux réservations confirmées ne se chevauchent jamais sur un même lieu (contrainte d'exclusion en base) ; une réservation sur un lieu occupé par une séance est refusée ou passe par une demande au 10. |
| R-AG19 | Annuler un événement ou une campagne libère ses réservations de lieux dans la même transaction. |
| R-AG20 | Un lieu indisponible ou fermé (socle 5) n'est pas réservable ; une indisponibilité nouvelle met en conflit les réservations touchées, signalées avec une salle de repli. |
| R-AG21 | Un examen national (38) ou un conseil (23) prime sur une réservation ordinaire ; la priorité est un ordre déclaré, pas une décision silencieuse. |
| R-AG22 | Jamais de coordonnée personnelle d'un personnel communiquée à une famille (téléphone de l'établissement par défaut). |
| R-AG23 | Liens externes : jeton aléatoire de 128 bits, usage limité, expiration, limitation de débit, réponse indiscernable pour inconnu ou expiré. |
| R-AG24 | Plafonds anti-abus : demandes ouvertes par hôte, réservations par famille et par campagne, inscriptions par session. |
| R-AG25 | Tout flux sortant (iCal, push, e-mail) est minimisé : titre calculé, aucun motif, aucun nom d'élève sauf choix personnel du lecteur, restreints en « occupé ». |
| R-AG26 | Les participants d'une réunion restreinte sont figés à la convocation ; un ajout ultérieur est tracé et avisé. |
| R-AG27 | Les avis de rendez-vous ne prennent pas de canal en dur : le socle 8 décide (préférences, U1/U2) ; SMS seulement pour une annulation du jour même par l'établissement, sous réserve de Q-N22. |
| R-AG28 | Le libre / occupé sur plus de 20 personnes est journalisé. |
| R-AG29 | Aucune donnée d'agenda d'un autre établissement : RLS stricte et clé composite `(etablissement_id, id)` sur toutes les tables. |
| R-AG30 | Les données de santé ne transitent jamais par le 19 : une convocation de santé porte « rendez-vous à l'infirmerie », sans objet médical. |
| R-AG31 | Une campagne ouverte ne se modifie plus dans ce qui a déjà été réservé sans avis aux familles concernées. |
| R-AG32 | Un parcours de soirée se réserve en tout ou rien (une transaction) ; échec partiel = aucun rendez-vous pris et explication. |
| R-AG33 | L'IA propose, simule et rédige ; elle ne confirme rien. |
| R-AG34 | Aucune colonne JSON de données métier (D-036) ; récurrences, règles, vagues, ordres du jour et présences sont des tables. |

---

## 21. Droits et minimisation

### 21.1 Catalogue d'actions proposé (D-017) [R]

| Action | Ce qu'elle ouvre | Portée | Modèle par défaut |
|---|---|---|---|
| `agenda.evenements.creer` | créer des événements et réunions | soi | tout personnel |
| `agenda.evenements.publier` | publier à une audience | audience sous ses droits | DIR, ADJ, CPE ; PP pour sa classe |
| `agenda.reunions.convoquer` | convoquer (obligatoire, délai) | idem | DIR, ADJ, CPE, PP (sa classe) |
| `agenda.reunions.organiser` | créneau commun, libération de cours | établissement | DIR, ADJ |
| `agenda.instances.lire` | tableau des instances, indicateurs | établissement | DIR |
| `agenda.calendrier.gerer` | calendrier prévisionnel | établissement | DIR, ADJ |
| `agenda.lire_structure` | agenda des classes et groupes (non nominatif) | établissement | personnel |
| `agenda.libre_occupe` | libre / occupé des personnels | établissement | personnel enseignant et d'éducation, SEC |
| `agenda.rdv.proposer` | proposer et créer des rendez-vous | élèves de son périmètre (D-022) | personnel |
| `agenda.rdv.convoquer` | convocation obligatoire d'une famille | élèves de son périmètre | CPE, DIR, ADJ |
| `agenda.rdv.gerer_service` | gérer les rendez-vous de son service | service | SEC (direction), VS (CPE) |
| `agenda.campagnes.gerer` | campagnes et soirées | établissement ; PP sa classe | DIR, ADJ |
| `agenda.sessions.gerer` | sessions collectives | soi | personnel |
| `agenda.accueil.lire` | liste du jour | établissement | ACC, VS, SEC |
| `agenda.lieux.lire` | planning des salles | établissement | personnel, AGT |
| `agenda.lieux.reserver` | réserver des lieux | soi | personnel |
| `agenda.lieux.valider` | valider les demandes d'un lieu | lieux désignés | désigné |
| `agenda.lieux.gerer` | gérer toutes les réservations | établissement | DIR, ADJ, SEC |
| `agenda.lieux.parametrer` | règles des lieux | établissement | ADM, DIR |
| `agenda.parametrer` | types, trames, délais | établissement | ADM |
| `agenda.journal.lire` | journaux | établissement | DIR, ADM |
| `agenda.reprise` | reprise Next.js | établissement | ADM |
| `agenda.programmes.lire` | programmes d'interventions et couverture | établissement | personnel |
| `agenda.programmes.gerer` | positionner, valider | **son** programme (responsable désigné) | responsables désignés |
| `agenda.programmes.parametrer` | créer un programme, désigner ses responsables | établissement | DIR, ADM |

`rencontres.gerer` (existant) devient un alias transitoire de `agenda.campagnes.gerer` jusqu'à AG5, puis est retiré.

### 21.2 Ce que voit chacun (projections) [R]

| Profil | Voit | Ne voit pas |
|---|---|---|
| Professeur | son agenda, libre / occupé, planning des salles, ses rendez-vous et ceux de ses élèves pris avec lui | motifs des collègues, rendez-vous des élèves avec d'autres services, rendez-vous restreints |
| CPE | ses rendez-vous, ceux de la vie scolaire, convocations de ses élèves | rendez-vous PsyEN, infirmerie, social |
| PsyEN, infirmier, AS | les leurs ; « occupé » pour le reste | — |
| Accueil | liste du jour (visiteur, heure, hôte, lieu, alerte) | motif, nature restreinte |
| Agents | planning des salles, mise en place | demandeur au-delà du nom, participants |
| Responsable | rendez-vous et invitations de ses enfants selon R-AG12 | rendez-vous de l'autre parent sous R-AG12, autres familles |
| Élève mineur | ses rendez-vous et réunions | rendez-vous de ses parents le concernant (sauf S19-07) |
| Externe | son rendez-vous par lien | tout le reste |

### 21.3 Conservation [R, durées soumises au DPO]

| Donnée | Durée proposée | Après |
|---|---|---|
| Rendez-vous ordinaires (famille ↔ établissement) | année scolaire + 1 an | anonymisés (statistiques) |
| Rendez-vous restreints (logistique seulement) | règle du socle propriétaire | — |
| Réunions autour d'un élève (logistique ; compte rendu au propriétaire) | durée du dossier du propriétaire | — |
| Instances (logistique) | 10 ans [H, archives publiques, §6] | archives (socle 27) |
| Réservations de lieux | 1 an | statistiques sans nom |
| Disponibilités, plages occupées importées | jusqu'à remplacement ; importées : 3 mois | effacées |
| Prospects (rendez-vous d'inscription) | règle du socle 3 | — |
| Liens externes | expiration + 30 jours | effacés |
| Journal d'accès et d'actions | **12 mois au maximum** (kit juridique SDET, §6.5) | effacé |
| Fil métier d'un rendez-vous, d'une instance | conservation de son objet (lignes ci-dessus) | — |
| Données d'une personne qui quitte l'établissement (disponibilités, liens) | **3 mois** (arrêté du 30 nov. 2006, §6.5) | effacées |

### 21.4 Pièges à tester

1. Un parent B séparé, sous interdiction, ne lit jamais heure ni salle du parent A (route famille, iCal, push, e-mail, accueil, IA).
2. Un professeur ne lit jamais le motif d'un rendez-vous d'un collègue par le libre / occupé, la recherche, l'export.
3. Un abonnement iCal ne contient jamais un nom d'élève par défaut ni un rendez-vous restreint autrement qu'« occupé ».
4. La liste du jour ne révèle pas la PsyEN comme hôte.
5. Une annulation garde le fil.
6. Aucune route ne répond pour un autre établissement.
7. Un élève ne peut pas inviter un autre élève.
8. Un lien externe expiré et un lien inconnu donnent la même réponse.
9. Les rappels fonctionnent en RLS stricte.
10. La simulation d'une soirée par l'IA n'écrit rien.

---

## 22. Événements (outbox, dans la transaction du fait) [R]

Agrégats propres : `AgendaEvenement`, `AgendaRdv`, `AgendaCampagne`, `AgendaReservationLieu` (remplacent `AggregateKind::Annonce`, RD-16). Charge utile : identifiants seulement, jamais de motif ni de nature restreinte.

| Événement | Charge utile | Consommateurs |
|---|---|---|
| `agenda.evenement_publie` / `_modifie` / `_reporte` / `_annule` / `_clos` | événement, type, plage, origine (brique, id) | 8, 29, 9 (flux), 23, 46, 13, 42 (état de leur objet) |
| `agenda.convocation_envoyee` | événement, nombre | 46, 13 (délais) |
| `agenda.presence_saisie` | événement | 46 (quorum), 6 (temps de service des AESH, P46) |
| `agenda.compte_rendu_depose` / `_diffuse` | événement, document | 7, 35, 13, 46 |
| `agenda.rdv_demande` / `_propose` / `_confirme` / `_reporte` / `_annule` / `_honore` / `_non_venu` / `_a_revoir` | rdv, élève objet (si non restreint), hôtes | 8, 16 (fil relié), 29, 12 (cours manqué), 3, 34, 37 (leur objet) |
| `agenda.campagne_ouverte` / `_modifiee` / `_close` | campagne, nature, origine | 8, 3, 34, 37 |
| `agenda.attente_servie` | campagne, hôte | 8 |
| `agenda.session_inscription` / `_desinscription` / `_presence` / `_annulee` | session | 3, 34, 43 |
| `agenda.lieu_reserve` / `_demande` / `_decide` / `_annule` / `_deplace` / `_en_conflit` | réservation, lieu | 10 (occupation), 44, 8 |
| `agenda.visiteur_arrive` | rdv | 14 (registre des visiteurs) |

Consommés : `eleve.lien_termine`, `eleve.autorite_modifiee`, `eleve.interdiction_posee` (4) ; `eleve.affectation_modifiee`, `eleve.depart`, `eleve.majorite` (2, 5) ; `personnel.indisponibilites`, `mission.*`, `equipe.*`, `agent.depart` (6) ; `absence.declaree` (15) ; `edt.seance_modifiee` (10) ; `referentiel.salle_indisponible`, `structure.fermeture` (5) ; `conseil.rencontre_souhaitee` (23).

---

## 23. Notifications (natures déclarées au socle 8) [R]

Reprend la ligne « Rendez-vous (19) » du socle 8 (`rendezvous.creneaux_ouverts`, `confirme`, `rappel_veille`, `annule`) et l'étend :

| Nature | Urgence | Destinataires | Remarque |
|---|---|---|---|
| `agenda.invitation` / `agenda.convocation` | U1 | invités | accusé pour une convocation |
| `agenda.evenement_modifie` / `_annule` | U1 (U2 le jour même) | invités | |
| `rendezvous.creneaux_ouverts` | U1 | public de la campagne, par vague | |
| `rendezvous.propose` / `.demande` | U1 | participants / hôte | |
| `rendezvous.confirme` | U1 | participants, hôtes | fichier .ics joint au lien (R-AG25) |
| `rendezvous.rappel_veille` / `.rappel_1h` | U1 | participants | 1 h : push seulement |
| `rendezvous.annule` / `.reporte` | U1 (U2 le jour même) | participants | SMS seulement U2 (R-AG27) |
| `rendezvous.autre_parent_informe` | U1 | autre parent (R-AG12) | jamais sous interdiction |
| `rendezvous.attente_place` | U1 | premier de la liste | délai d'acceptation |
| `rendezvous.a_revoir` | U1 | hôte, organisateur | |
| `agenda.compte_rendu_disponible` | U1 | participants | |
| `agenda.lieu_decide` / `_en_conflit` | U1 | demandeur | |
| `agenda.visiteur_arrive` | U1, push | hôte | |

---

## 24. Écrans (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`, Leptos)

| Écran | Route | Contenu |
|---|---|---|
| Mon agenda | `/web/agenda` | semaine par défaut ; sources en couleurs sobres ; panneau latéral de l'élément ; « nouveau » (événement, rendez-vous, salle) |
| Libre / occupé et créneau commun | `/web/agenda/creneau-commun` | personnes et lieux, durée, fenêtre ; propositions classées |
| Événement | `/web/agenda/evenements/{id}` | onglets : détails · participants et réponses · ordre du jour · présence · compte rendu · historique |
| Tableau des instances | `/web/agenda/instances` | tuiles : à convoquer, délais, quorum, comptes rendus manquants |
| Mes rendez-vous | `/web/agenda/rendez-vous` | à venir, demandes à traiter, passés ; disponibilités |
| Campagne | `/web/agenda/campagnes/{id}` | assistant en étapes (public, hôtes, plages, salles, vagues, aperçu) ; tableau du soir |
| Sessions | `/web/agenda/sessions` | sessions, inscrits, pointage |
| Salles | `/web/agenda/salles` | recherche, planning par salle (grille dense), demandes à valider |
| Liste du jour | `/web/accueil/attendus` | heure, visiteur, hôte, lieu, alerte ; « arrivé » |
| Famille : agenda | `/famille/agenda` | par enfant ; invitations ; rendez-vous ; « prendre rendez-vous » |
| Famille : soirée | `/famille/rencontres/{id}` | par enfant, enseignants, parcours proposé, liste d'attente, ma soirée |
| Externe | `/rdv/{jeton}` | son rendez-vous, changer, annuler, ajouter à l'agenda |

L'ancienne interface `ui.rs` (F-001) n'est pas étendue ; les écrans de rencontres qu'elle porte sont remplacés à AG5.

---

## 25. Modèle de données cible (brique `agenda`) [R]

Toutes les tables : `etablissement_id UUID NOT NULL`, `id UUID` applicatif, clé `UNIQUE (etablissement_id, id)`, **clés étrangères composites** `(etablissement_id, …)`, `created_at`, `updated_at` ou fil daté, RLS stricte (`ent_rls_poser`), aucune colonne JSON (D-036), aucun BYTEA (fichiers au socle 7), classement au registre des tables (D-037) ; heures en `timestamptz` calculées depuis l'heure murale et le fuseau de l'établissement, avec `jour_local DATE` pour les événements sur la journée.

```text
-- Événements et réunions (2381–2382)
ent_agenda_type_local       (etablissement_id, id, code, libelle, famille, convocation, delai_jours, compte_rendu_attendu, confidentialite_defaut, actif)
ent_agenda_serie            (etablissement_id, id, frequence, intervalle, du, au, semaine_ab, saute_vacances, cree_par)
ent_agenda_serie_jour       (etablissement_id, serie_id, jour_semaine)            -- 1..7
ent_agenda_evenement        (etablissement_id, id, type_code, type_local_id, titre, description, debut, fin, journee_entiere,
                             jour_debut, jour_fin, lieu_kind, salle_id, lieu_id, lieu_texte, visio_ref, organisateur_personne_id,
                             confidentialite, statut, motif_annulation, serie_id, occurrence_detachee, origine_brique, origine_id,
                             version, created_at, updated_at)
ent_agenda_audience         (etablissement_id, id, evenement_id, kind, cible_id)   -- classe, niveau, groupe, equipe, mission, site, tous_personnels, toutes_familles
ent_agenda_participant      (etablissement_id, id, evenement_id, personne_id, role, obligatoire, source, eleve_contexte_id,
                             reponse, repondu_le, motif_excuse, represente_par_personne_id, presence, presence_saisie_par, presence_le, ajoute_le)
ent_agenda_point            (etablissement_id, id, evenement_id, rang, intitule, nature, rapporteur_personne_id, duree_minutes, origine_brique, origine_id)
ent_agenda_point_piece      (etablissement_id, point_id, document_id)
ent_agenda_compte_rendu     (etablissement_id, id, evenement_id, document_id, redige_par, valide_par, valide_le, diffuse_le)
ent_agenda_suite            (etablissement_id, id, evenement_id, intitule, responsable_personne_id, echeance, faite_le)
ent_agenda_coupon           (etablissement_id, id, evenement_id, question, choix_1, choix_2, choix_3)   -- question fermée, 2 à 3 choix
ent_agenda_coupon_reponse   (etablissement_id, coupon_id, personne_id, eleve_id, choix, repondu_le)

-- Rendez-vous (2384)
ent_agenda_rdv              (etablissement_id, id, nature, restreint, offre_id, creneau_id, eleve_id, debut, fin, modalite,
                             salle_id, lieu_texte, visio_ref, statut, motif_categorie, motif_detail, demande_par_personne_id,
                             report_de_rdv_id, rappel_veille_le, rappel_1h_le, version, created_at, updated_at)
ent_agenda_rdv_participant  (etablissement_id, id, rdv_id, personne_id, role, informe_seulement, presence, ajoute_le, retire_le)
ent_agenda_rdv_proposition  (etablissement_id, id, rdv_id, rang, debut, fin, choisie)
ent_agenda_rdv_fait         (etablissement_id, id, rdv_id, fait, par_personne_id, le, motif_public, detail)   -- ajout seul (déclencheur)
ent_agenda_rdv_suite        (etablissement_id, rdv_id, texte, redige_par, le)                                 -- privée au service
ent_agenda_lien_externe     (etablissement_id, id, rdv_id, personne_id, empreinte_jeton, usages_restants, expire_le, dernier_usage_le)

-- Disponibilités (2383)
ent_agenda_disponibilite    (etablissement_id, id, personne_id, jour_semaine, heure_debut, heure_fin, du, au, modalite, salle_id,
                             ouverte, public, created_at)          -- public : familles_eleves, eleves, collegues
ent_agenda_occupe_perso     (etablissement_id, id, personne_id, debut, fin, source, importe_le)              -- sans titre

-- Offres, campagnes, créneaux, sessions (2385)
ent_agenda_offre            (etablissement_id, id, nature, titre, origine_brique, origine_id, statut, ouvre_le, ferme_le,
                             delai_reservation_heures, delai_annulation_heures, duree_minutes, un_par_hote_eleve,
                             parents_regle, plafond_famille, presence_eleve, attente_active, attente_delai_heures,
                             reconfirmation_jours, provisoire_minutes, created_at, updated_at)
ent_agenda_offre_public     (etablissement_id, id, offre_id, kind, cible_id)
ent_agenda_offre_vague      (etablissement_id, id, offre_id, rang, ouvre_le, kind)                -- a_voir, fratrie, separes, tous
ent_agenda_offre_cible      (etablissement_id, offre_id, vague_id, eleve_id, hote_personne_id, origine)   -- familles « à voir »
ent_agenda_hote             (etablissement_id, id, offre_id, personne_id, salle_id, lieu_texte, modalites_tel, modalites_visio, source, actif)
ent_agenda_creneau          (etablissement_id, id, offre_id, hote_id, debut, fin, capacite, statut, salle_id)
                             -- EXCLUDE USING gist (etablissement_id WITH =, hote_id WITH =, tstzrange(debut, fin) WITH &&) WHERE statut <> 'supprime'
ent_agenda_inscription      (etablissement_id, id, creneau_id, personne_id, eleve_id, rdv_id, statut, inscrit_le, presence)
ent_agenda_attente          (etablissement_id, id, offre_id, hote_id, creneau_id, eleve_id, personne_id, rang, statut, propose_le, expire_le)

-- Réservations de lieux (2386)
ent_agenda_lieu_regle       (etablissement_id, salle_id, lieu_id, validation, valideur_personne_id, valideur_mission_type,
                             delai_min_heures, duree_max_minutes, horizon_jours, reservable_par)
ent_agenda_reservation      (etablissement_id, id, salle_id, lieu_id, debut, fin, demandeur_personne_id, pour_personne_id,
                             motif_categorie, intitule, effectif, mise_en_place, evenement_id, offre_id, serie_id, statut,
                             priorite, decide_par, decide_le, motif_decision, created_at, updated_at)
                             -- EXCLUDE USING gist (etablissement_id WITH =, salle_id WITH =, tstzrange(debut, fin) WITH &&)
                             --   WHERE statut IN ('demandee','confirmee')
ent_agenda_reservation_fait (etablissement_id, id, reservation_id, fait, par_personne_id, le, detail)   -- ajout seul

-- Programmes d'interventions (2387)
ent_agenda_programme        (etablissement_id, id, intitule, actif, created_at)
ent_agenda_programme_resp   (etablissement_id, programme_id, personne_id, du, au)
ent_agenda_seance_type      (etablissement_id, id, programme_id, niveau_id, rang, theme, contrainte, discipline_code, mixte, actif)
ent_agenda_positionnement   (etablissement_id, id, seance_type_id, classe_id, intervenant_personne_id, intervenant_externe, idee,
                             jour, seance_ref, salle_id, statut, commentaire, decide_par, decide_le, created_at, updated_at)
                             -- UNIQUE (etablissement_id, seance_type_id, classe_id) WHERE statut IN ('propose','valide','modifications_demandees')
ent_agenda_positionnement_fait (etablissement_id, id, positionnement_id, fait, par_personne_id, le, commentaire)   -- ajout seul

-- Journal et paramètres (2381, 2382)
ent_agenda_journal          (etablissement_id, id, objet, objet_id, action, par_personne_id, le, avant, apres)   -- texte, ajout seul
ent_agenda_trame            (etablissement_id, id, type_code, canal, rang, titre, corps, actif)
ent_agenda_parametre        → registre de paramètres du socle 5 (clés typées et bornées), pas de table propre
```

Remarques : `btree_gist` est nécessaire pour les contraintes d'exclusion (extension standard, déjà disponible chez Scaleway [NV]). `motif_detail` et `ent_agenda_rdv_suite.texte` sont **chiffrés** par `ent_db::champs_chiffres` (existant) [R]. `restreint` est dérivé de la nature (déclencheur), non saisi. `ent_psy_rdv` (046) n'est pas repris ici (socle 36).

---

## 26. Plan de reprise des données de Next.js (D-035, point 4)

La base de production reste inaccessible au pilote ; la reprise passe par des exports fournis par Florian (anonymisés pour les essais, réels après l'accord écrit de l'établissement et l'avis du DPO, D-035 point 4) et par l'outil de reprise de `TRANSITION_T0_INVENTAIRE.md` (`--modules salles,…`).

| Source Next.js | Cible | Règle | Perdable ? |
|---|---|---|---|
| `reservation_room` | `ent_salle` (socle 5) + `ent_agenda_lieu_regle` | **rapprochement** par nom normalisé et bâtiment ; salle inconnue → file humaine, jamais de création silencieuse (`TRANSITION_T0_INVENTAIRE.md` §4.2) ; `kind=classroom` → règle « non réservable » | non |
| `reservation_room_booking` **futures** confirmées | `ent_agenda_reservation` | demandeur et bénéficiaire → personne (par compte, jamais par e-mail seul) ; heure murale texte → instant dans le fuseau de l'établissement ; `group_id` → série ; contrôle de chevauchement **à l'import** (les doublons historiques de ND-01 vont en file humaine) ; `class_name` → classe si reconnue, sinon intitulé | non |
| `reservation_room_booking` passées ou annulées | — | non reprises ; archive d'un an si Florian s'en sert pour des statistiques (Q-T0-08) | oui |
| Configuration `prof-room` (couleurs, horizon, administrateurs) | règles du lieu, droits D-017 | administrateurs listés → `agenda.lieux.gerer` (décision humaine) | partiellement |
| `domain-planning__signups` de l'année en cours | positionnements (§12.2) | séance type et classe rapprochées ; statut gardé ; positionnements « refusés » non repris | à vérifier (Q-T0) |
| `domain-planning__domains`, `__sessions` | programmes et séances types | relus par l'administrateur avant import | non |
| `domain-planning__bookings`, tables `domain_planning_*` | — | obsolètes ou vides | oui |
| `rdv_inscription_booking` **futurs** confirmés | `ent_agenda_rdv` d'une offre `inscription` (socle 3 pour le dossier) | jetons **non repris** (`TRANSITION_T0_INVENTAIRE.md` §7) : un e-mail d'information avec un nouveau lien ; PAP non repris (Q-I7) ; jeton Google abandonné | non |
| `portes_ouvertes_*` de la campagne en cours | sessions et inscriptions (socle 3 pour les prospects) | consentement réel exigé (ND-29 : forcé à vrai) | partiellement |
| `invitation_*` | — | perdables après l'événement | oui |
| `ent_rencontre`, `ent_creneau_rdv` (Rust, 192) | offre `rencontre_parents_profs`, créneaux, rendez-vous | migration 2389 au lot AG5 ; fil reconstitué depuis les colonnes existantes (`reserve_le`) ; tables gardées en lecture seule une année | non |

**Bascule** : module par module, Next.js restant en service tant que ScolIA n'a pas fait ses preuves (D-035 point 3) ; pour les salles, **gel des réservations dans Next.js** la veille (décision et action de Florian), import des réservations futures, contrôle par comptage et par échantillon, redirection de `/prof-room` (action de Florian), retour arrière possible tant qu'aucune réservation n'a été créée dans ScolIA. Dates proposées au §28.

---

## 27. Lot AG0 : correctifs urgents de l'existant Rust

Sur la tête d'`ENT`, **avant** tout autre lot du socle (même logique que T0 du socle 10 et V0 du 21). Petit, sans nouvel écran, migration **2380** seulement. Chaque défaut a un test rouge avant, vert après.

| N° | Défaut [F] | Où | Correctif | Test |
|---|---|---|---|---|
| **RD-01** | L'annulation remet le créneau à `libre` et **efface** qui avait réservé, pour quel enfant et quand ; seule trace : un événement outbox minimal | `rencontres.rs` `annuler_reservation` | table `ent_rdv_fait` (ajout seul, déclencheur) : réservation, annulation (par qui, organisateur ou famille), rappel, présence ; écrite dans la transaction | annuler puis relire le fil : 2 faits |
| **RD-02** | Créneau **passé** réservable ; annulation possible après la rencontre ou une fois close | `reserver_creneau`, `annuler_reservation` | refus si `debut <= now() + délai` (12 h par défaut) ; annulation famille refusée après `debut - 2 h` et si la rencontre est close (l'organisateur garde la main) | réserver hier → 409 ; annuler à H-1 → 409 |
| **RD-03** | **Second parent aveugle** : le co-responsable voit « indisponible » le rendez-vous pris pour leur enfant commun, n'est pas prévenu, ne peut pas en prendre un autre | `creneaux_famille`, `notifier_rdv` | état `reserve_pour_cet_enfant` (sans nom du réservant) montré aux autres responsables à autorité parentale **du même foyer** ; aucune information si une interdiction ou une adresse protégée concerne l'un d'eux (`ent_eleve_interdiction`, 271 ; `adresse_protegee`, 267) ; rendez-vous séparés pour foyers distincts : lot AG5 | deux parents même foyer → B voit « déjà pris pour Léa » ; parent sous interdiction → B ne voit que « indisponible » |
| **RD-04** | Deux responsables du même enfant qui réservent en même temps chez le même enseignant : violation de l'index unique → **erreur 500** (le verrou consultatif est par responsable) | `reserver_creneau` | verrou sur `(rencontre, enseignant, élève)` et violation d'unicité traduite en conflit lisible | deux réservations parallèles → une 200, une 409 |
| **RD-05** | Lien rompu non revérifié : un responsable dont le lien a pris fin peut encore annuler par identifiant ; le rendez-vous d'un parent qui a perdu l'autorité reste en place | `annuler_reservation` ; aucune réaction | revérifier le lien à l'annulation ; réaction à `eleve.lien_termine` / perte d'autorité : rendez-vous futurs marqués « à revoir » (fait) et organisateur prévenu | lien clos → annulation 404 ; réaction → fait « a_revoir » |
| **RD-06** | **Rappels lus hors contexte d'établissement** (`etablissement = None`, `main.rs:243-258`) : en RLS stricte (580–600), 0 ligne ou erreur avalée par un `warn` → rappels perdus en silence | `emettre_rappels_rdv` | lister les établissements actifs, puis traiter chacun dans un `TenantTx` ; compteur d'échecs exposé (métrique) | test en RLS stricte : rappel émis |
| **RD-07** | **Heure de Paris en dur** dans les avis (`heure_paris`) | `notifier_rdv` | `structure::heure_locale` (fuseau de l'établissement, socle 5) | établissement de La Réunion : heure locale correcte |
| **RD-08** | **SMS** demandé pour chaque confirmation, annulation et rappel au parent (`canaux_voulus: email, sms, push`) alors que Q-N22 (coût des SMS) n'est pas tranchée | `notifier_rdv` | retirer `sms` des confirmations et rappels ; le garder seulement pour une annulation par l'établissement le jour même (R-AG27) | — |
| **RD-09** | Annulation par l'organisateur **sans motif** : la famille reçoit « rendez-vous annulé » sans raison ni suite | `annuler_reservation` | motif public obligatoire pour l'organisateur ; texte de l'avis le reprend | — |
| **RD-10** | L'enseignant n'a **aucune commande** sur ses créneaux (ni bloquer, ni annuler, ni « reçu / non venu ») | `rencontres.rs` | commandes « bloquer un créneau libre » et « présence » (honoré / non venu) pour l'enseignant du créneau ; annulation par l'enseignant : AG5 | enseignant bloque son créneau ; un autre enseignant → 404 |
| **RD-11** | `eleve_id` **sans clé étrangère** ; clés simples non composites vers `ent_rencontre`, `ent_classe`, `ent_user` ; `ON DELETE CASCADE` des créneaux avec la rencontre (supprime l'historique) | 192 | 2380 : FK composite `(etablissement_id, eleve_id)` ; `ON DELETE RESTRICT` ; une rencontre avec réservations ne se supprime pas (aucune route ne supprime aujourd'hui) | contrainte présente |
| RD-12 | Agrégat d'événement `Annonce` pour des rendez-vous | `reserver_creneau` etc. | laissé (changement de type d'agrégat au lot AG4, pour ne pas casser les consommateurs) | — |
| RD-13 | Rencontre **limitée à une classe** ; pas de salle par enseignant, pas d'ouverture et de fermeture programmées, pas de liste d'attente, pas de modalité | 192 | AG5 | — |
| RD-14 | Aucun contrôle contre l'**EDT** ni les autres engagements de l'enseignant | `ajouter_creneaux` | AG5 (avertissement dès AG0 si le port `edt::occupe` existe sur la branche : [R] non bloquant) | — |
| RD-15 | Un seul droit `rencontres.gerer` pour tout | `actions.rs` | AG1 (catalogue §21.1) | — |
| RD-16 | RLS **permissive** sur `ent_rencontre`, `ent_creneau_rdv` (480) | 480 | strict au lot AG1 avec les nouvelles tables | — |

**Périmètre AG0** : RD-01 à RD-11. Estimation : S (une session Sonnet). Revue indépendante ciblée (`review-sonnet`) sur RD-03, RD-04, RD-06.

---

## 28. Lots de code (équipe Sonnet) et migrations

### 28.1 Tableau des lots

| Lot | Contenu | Dépend de | Critère de fin | Taille |
|---|---|---|---|---|
| **AG0** | Correctifs urgents de l'existant (§27) | — | tests RD verts ; aucune régression des tests de communication | S |
| **AG1** | Noyau : événements (types, séries matérialisées, audience, participants, réponses, coupon), **vue agenda calculée** par ports (10, 5, 42… avec adaptateurs « source absente » pour ceux qui n'existent pas encore), journal, droits §21.1, RLS stricte, écran « mon agenda » Leptos, iCal ponctuel minimisé | AG0 ; socle 5 (fuseau, calendrier), socle 8 N1 (ou adaptateur S2-8) | R-AG1, R-AG5, R-AG8 testées ; pièges 3, 6 ; contrat IA des lectures | L |
| **AG2** | Disponibilités, plages occupées, **libre / occupé**, créneau commun (port `edt::plages_libres`), réactions EDT et départs | AG1 ; port 10 | R-AG6, R-AG28 ; piège 2 | M |
| **AG3** | **Réservation de lieux** : règles des lieux, recherche, planning des salles, réservation unitaire et en série, validation, priorités, conflits, avis, mise en place, affichette ; **parité M02** (NX-01 à NX-19) | AG1 ; socle 5 (salles, disponibilités) ; port `edt::occupe` | ND-01 à ND-12 non reproduits (un test par défaut) ; recette comparée à Next.js | M |
| **AG4** | **Rendez-vous individuels** : demandes, propositions, réservation dans les disponibilités, modalités, participants, R-AG12 complète, fiche de rendez-vous, suites, rendez-vous restreints (logistique), réactions au cœur (4, 2, 6, 15), carte d'action du 16, liens externes, nouveaux agrégats d'événements | AG2 ; socle 4 (interdictions), 16 (cartes) | pièges 1, 4, 5, 7, 8 ; CAG-01 à CAG-06 | L |
| **AG4a** | Sous-lot démo de décembre : AG4 réduit à « proposer / demander / confirmer / annuler » + vue famille | AG1 | parcours de démo | S |
| **AG5** | **Moteur de créneaux et campagnes** : offres, hôtes calculés, plages générées, salles, vagues, « familles à voir », parcours d'une fratrie en tout ou rien, liste d'attente, réservation provisoire, reconfirmation, tableau du soir, statistiques ; **reprise de 192** (2389) et retrait de `rencontres.gerer` | AG3, AG4 ; ports 6, 10, 23 | R-AG31, R-AG32 ; CAG-07, CAG-08, CAG-15, CAG-21 ; test de charge (300 familles en 5 minutes à l'ouverture) | L |
| **AG6** | **Sessions collectives** (capacité, pointage, message de suivi, manifestations avec accompagnants) ; offres pour le socle 3 (rendez-vous d'inscription, portes ouvertes) avec lien public anti-abus | AG5 ; socle 3 I10 | ND-20 à ND-31 non reproduits | M |
| **AG6b** | **Programmes d'interventions** (refonte de M03) | AG3, AG6 ; socle 6 (services) | ND-13 à ND-17 non reproduits | M |
| **AG7** | **Réunions et instances** : convocation avec délai réglementaire, ordre du jour, quorum (règle du 46), émargement, compte rendu (7), suites (29), réunions demandées par 23, 13, 35, 46, 42 ; calendrier prévisionnel reconduit ; tableau des instances | AG1 ; socles 7, 8 | R-AG10, R-AG26 ; CAG-40 à CAG-48 | M |
| **AG8** | Mobile (iOS, Android) : agenda, liste du jour, rendez-vous, ma soirée, émargement, salle libre | AG4, AG5 ; socle 11 | parcours mobiles | M |
| **AG9** | Accueil : liste du jour, arrivée d'un visiteur, alerte interdiction (socle 14) | AG4 ; socle 14 | piège 4 | S |
| **AG10** | iCal par abonnement (flux du 19 pour le X15 du socle 9), visio (33), import d'un agenda personnel en plages occupées (option) | AG4 ; socles 9, 33 | piège 3 ; R-AG25 | S |
| **AG11** | **Reprise Next.js** (salles, réservations futures, programmes, rendez-vous d'inscription avec le 3) et bascule | AG3, AG6b ; outil de reprise T0 | comptages et échantillons ; retour arrière répété | M |
| **AG12** | Contrat IA complet, jeu d'évaluation, indicateurs (47), purges (27) | tous | `IA_COEUR.md` §5.4 | S |

**Ordre proposé** (D-034, D-035) : AG0 (novembre 2026) → AG1 + AG4a (démo de début décembre) → AG3 (salles : priorité de l'add-on) → AG2 → AG4 → AG5 → AG7 → AG6 → AG6b → AG9 → AG8 → AG10 → AG11 → AG12. **Production à la rentrée 2027** (D-035 point 2) pour AG0–AG6b et AG11 ; bascule des salles proposée aux **vacances d'hiver 2027** (février, faible activité de réservation) [R, date à fixer par Florian, Q-AG10].

### 28.2 Migrations (plage imposée 2380–2399, registre `MIGRATIONS.md`)

| N° | Nom | Lot | Contenu |
|---|---|---|---|
| **2380** | `agenda_ag0_rencontres` | AG0 | `ent_rdv_fait` (ajout seul) ; FK composite élève ; `ON DELETE RESTRICT` ; colonnes `annule_motif` provisoires sur `ent_creneau_rdv` ; aucune donnée supprimée |
| **2381** | `agenda_evenement` | AG1 | types locaux, séries, jours, événements, audience, participants, coupon, journal ; RLS stricte des nouvelles tables **et** de 192 |
| **2382** | `agenda_reunion` | AG7 | points, pièces, comptes rendus, suites, trames |
| **2383** | `agenda_disponibilite` | AG2 | disponibilités, plages occupées |
| **2384** | `agenda_rdv` | AG4 | rendez-vous, participants, propositions, faits, suites (chiffrées), liens externes (empreintes) |
| **2385** | `agenda_offre` | AG5 | offres, public, vagues, cibles, hôtes, créneaux (exclusion), inscriptions, attente ; `CREATE EXTENSION IF NOT EXISTS btree_gist` |
| **2386** | `agenda_reservation_lieu` | AG3 | règles des lieux, réservations (exclusion), faits |
| **2387** | `agenda_programme` | AG6b | programmes, responsables, séances types, positionnements, faits |
| **2388** | `agenda_reprise_next` | AG11 | tables de correspondance de la reprise (source, identifiant source, cible, décision) |
| **2389** | `agenda_bascule_192` | AG5 | copie de 192 vers le moteur ; `ent_rencontre` et `ent_creneau_rdv` passent en lecture seule (déclencheur) |
| 2390–2399 | réserve | AG8–AG12, ajouts personas | — |

Chaque table : `etablissement_id NOT NULL`, RLS stricte, `UNIQUE (etablissement_id, id)`, FK composites, classement au registre (`registre_donnees.rs`) ; **aucune colonne JSON** (D-036), aucune exception à déclarer ; `ent_agenda_journal.avant/apres` sont des **textes** (comme le socle 42, V0a).

---

## 29. Tests et recette

- **Isolation** : chaque route du socle dans la matrice de séparation ; RLS stricte ; tests d'un autre établissement sur toutes les lectures, y compris flux iCal et liste du jour.
- **Concurrence** : 50 réservations simultanées du même créneau → une seule ; parcours d'une fratrie en tout ou rien sous charge ; deux réservations de salle chevauchantes en parallèle → une seule (contrainte).
- **Temps** : changement d'heure, La Réunion, semaines A/B, vacances sautées, horizon, délais.
- **Minimisation** : les dix pièges du §21.4 ; aucun motif dans aucune réponse hors participants (test générique sur toutes les DTO du socle).
- **Réactions** : fin de lien, perte d'autorité, interdiction posée, changement de classe, départ, majorité, absence d'un hôte, fermeture d'une salle.
- **Parité** : pour chaque NX repris, un scénario de recette comparé à Next.js (D-035 point 5) ; pour chaque ND, un test qui prouve qu'il n'est pas reproduit.
- **Recette par personas** (D-021) : `PERSONAS_19.md`.

---

## 30. Réglages signalés (D-023 : peu de réglages, signalés)

| N° | Réglage | Défaut | Candidat à l'allègement ? |
|---|---|---|---|
| S19-01 | Délai minimal de réservation d'un rendez-vous | 12 h | oui |
| S19-02 | Délai minimal d'annulation par la famille | 2 h | oui |
| S19-03 | Plusieurs rendez-vous par hôte et par enfant dans une campagne | non | oui |
| S19-04 | Rendez-vous commun obligatoire pour parents de foyers distincts | non | non |
| S19-05 | Plafond de rendez-vous par famille et par campagne | 12 | oui |
| S19-06 | Demandes de rendez-vous ouvertes par hôte et par famille | 3 | oui |
| S19-07 | L'élève mineur voit les rendez-vous de ses parents le concernant | non | non |
| S19-08 | Durée d'un rendez-vous de soirée | 10 min | — |
| S19-09 | Prénoms des élèves dans mon flux iCal (réglage personnel) | non | — |
| S19-10 | Domaines visio autorisés avant le socle 33 | aucun | oui (disparaît avec le 33) |
| S19-11 | Salles de classe réservables sur leurs heures libres | non | — |
| S19-12 | Délai d'acceptation d'une place de liste d'attente | 12 h | oui |
| S19-13 | Horizon de réservation des salles | 56 j | — |

---

## 31. Décisions appliquées par défaut (D-031) et questions à Florian

### 31.1 Appliquées par défaut (D-031)

| N° | Question | Recommandation appliquée |
|---|---|---|
| Q-AG1 | Qui porte la réservation de salles : 19 ou 44 ? | **19** (moteur de réservation de lieux) ; 44 garde le matériel et réutilise le moteur (§14.1) |
| Q-AG2 | Où vont les programmes d'interventions (M03, EVARS) ? | au **19**, lot AG6b, tant qu'aucun socle « parcours éducatifs » n'existe (Florian peut en créer un) |
| Q-AG3 | Parents séparés | règle R-AG12 (même foyer : commun ; foyers distincts : chacun le sien ; interdiction : aucune information croisée) |
| Q-AG4 | La « suite » d'un rendez-vous est-elle visible de la famille ? | **non** par défaut (note de service) ; un compte rendu destiné à la famille est un document du 7 |
| Q-AG5 | Google Agenda pour les rendez-vous d'inscription | **non** par défaut (Q-I12 déjà tranchée) ; option désactivée |
| Q-AG6 | SMS pour les rendez-vous | seulement l'annulation du jour même par l'établissement, en attendant Q-N22 |
| Q-AG7 | Import de l'agenda personnel d'un personnel | option personnelle, plages « occupé » sans titre, effacées à 3 mois |
| Q-AG8 | Visio avant le socle 33 | lien saisi seulement sur domaines autorisés, vu des seuls participants |
| Q-AG9 | Rendez-vous psy (046) | restent au socle 36 ; le 19 n'en lit que l'occupation |
| Q-AG11 | Occurrences des séries | matérialisées jusqu'à la fin de l'année scolaire |
| Q-AG12 | Parcours de soirée | réservé en tout ou rien |

### 31.2 Soumis à Florian (engagent l'établissement, ou faits que seul Florian connaît)

| N° | Sujet | Pourquoi |
|---|---|---|
| Q-AG10 | **Date de bascule des salles** (proposée : vacances d'hiver 2027) et gel de Next.js la veille | action sur la production, réservée à Florian (D-035 point 3) |
| Q-AG13 | **Failles Next.js à corriger en production** (ND-20, ND-30 : énumération de données de familles par des pages publiques ; ND-21 ; ND-22 ; ND-13) | Next.js est intouchable pour le pilote ; c'est à Florian de décider d'un correctif ou d'une désactivation |
| Q-AG14 | Exports réels de M02, M03, M16 pour la reprise | données réelles : accord écrit de l'établissement et avis du DPO (D-035 point 4) |
| Q-AG15 | Durées de conservation (§21.3) | DPO |
| Q-AG16 | Un socle « parcours éducatifs » dédié ? | structure de la carte |
| Q-N22 (rappel) | Coût des SMS | contrat OVH |

---

## 32. Faits, hypothèses, contradictions

**Faits [F]** : 192 et `rencontres.rs` lus en entier ; rappels appelés sans établissement (`main.rs:243-258`) ; RLS permissive (480) ; registre `registre_donnees.rs:290,531` ; iCal derrière la session (`routes.rs:637-639`) ; M02 sans contrainte de chevauchement, dates texte sans fuseau ; M03 en documents JSON uniques réécrits ; M16 adossé à Google, jeton de rafraîchissement en clair, mention du PAP dans Google ; M17 consentement forcé ; invitations `lookup` sans limite.

**Hypothèses [H]** : calendrier de l'année (§5) ; durée de 10 min par rendez-vous de soirée ; disponibilité de `btree_gist` sur l'offre managée Scaleway [NV] ; maturité de M16 et M17.

**Contradictions relevées** :
1. `SOCLES.md` §44 et `10_EMPLOI_DU_TEMPS.md` (§6.1 « 44 possède », §2 « réservations 44 ») contre ce document (réservation de lieux au 19, §14.1) : à reporter dans `SOCLES.md` et le 10 si Florian ne revient pas dessus.
2. `05_STRUCTURE.md` (ligne « les réservations restent au socle 44 ») : même correction ; le contrat de disponibilité commun (STR-09 / réservation) est écrit ici (§14.3, point 2).
3. `03_INSCRIPTIONS.md` prévoit `ent_inscription_rdv` « créneaux propres au socle 3, contrat compatible socle 19 » : ce document recommande que le 3 **consomme** le moteur du 19 (offre de nature `inscription`) plutôt que d'avoir ses propres créneaux (R-AG2). Le 3 garde ses contacts, dossiers et décisions.
4. `SOCLE2_DOMAINES.md` AGD-03 annonçait une liste d'attente : absente de l'existant (192) ; reprise en AGD-207.
5. Le socle 8 nomme les natures `rendezvous.*` ; ce document ajoute `agenda.*` pour les événements (§23).

---

## 33. Ajout personas (`PERSONAS_19.md`, 2026-11-11) — l'emporte en cas de contradiction

### 33.1 Les quatre bloquants et leur traitement (appliqués par défaut, D-031)

| # | Bloquant | Traitement |
|---|---|---|
| A | Une interdiction ne protège pas le couloir | **R-AG35** séparation dans le temps et l'espace ; AGD-335, AGD-336 |
| B | Le « cours manqué » révèle un rendez-vous sensible | **R-AG36** sortie de cours neutre ; AGD-334 ; AGD-179 et AGD-230 sont modifiés en conséquence |
| C | L'occupation des personnels de santé et du social désigne les élèves | **R-AG37** occupation protégée ; R-AG6 ne s'applique plus à ces personnels |
| D | Les familles sans numérique sont exclues | **AGD-320** réserver pour le compte d'une famille, **AGD-321** courrier |

### 33.2 Actions ajoutées (AGD-320 à AGD-339)

| ID | Action | Acteur | Droit | Règles | Événement | Lot |
|---|---|---|---|---|---|---|
| AGD-320 | Réserver ou proposer **pour le compte d'une famille** (téléphone, portail), dans toute offre | SEC, PP, CPE, ACC (offres publiques) | `agenda.rdv.proposer` ou `agenda.accueil.lire` (offres publiques) | mêmes contrôles que la famille ; fait « pris par [personne] au téléphone » ; avis à la famille | `agenda.rdv_confirme` | AG4, AG5 |
| AGD-321 | Produire un **courrier** de convocation ou d'invitation (socle 7) | organisateur, hôte | idem | personnes sans compte ou qui le demandent ; dépôt au coffre, lien nominatif | — | AG4 |
| AGD-322 | Contraindre une campagne par **bâtiment ouvert** | organisateur | `agenda.campagnes.gerer` | horaires de site (5) | — | AG5 |
| AGD-323 | Ouvrir par **vagues horaires** avec plafond par famille en première vague | organisateur | idem | S19-16 | — | AG5 |
| AGD-324 | **Publier le planning aux hôtes** avant les familles (48 h d'ajustement) | organisateur | idem | — | `agenda.campagne_publiee_hotes` | AG5 |
| AGD-325 | Rouvrir une campagne close depuis moins de 24 h | organisateur | idem | — | `agenda.campagne_modifiee` | AG5 |
| AGD-326 | Recevoir **à une autre date** dans une campagne (hôte absent ou à temps partiel) | hôte | soi | créneaux hors soirée rattachés à la campagne | — | AG5 |
| AGD-327 | Après une non-venue à une convocation : proposer un second rendez-vous et un courrier | système → hôte | — | avis à l'hôte, il décide | — | AG4 |
| AGD-328 | Rendez-vous **immédiat** (famille reçue sans rendez-vous), créé honoré | personnel | `agenda.rdv.proposer` | trace seulement | `agenda.rdv_honore` | AG4 |
| AGD-329 | Demande d'un parent : « ne pas informer l'autre parent » sans interdiction judiciaire | PAR → DIR | lien ; décision `agenda.rdv.convoquer` (DIR) | examen par la direction, jamais automatique ; décision tracée | — | AG4 |
| AGD-330 | Marquer **interprète** (langue, interprète désigné, durée doublée) | hôte, participant | soi | INS-61 du socle 3 | — | AG4 |
| AGD-331 | Recevoir une réunion organisée par un **externe** (ESS de l'enseignant référent) et y inscrire les participants internes | CPE, PP, DIR | `agenda.reunions.convoquer` | organisateur externe = personne contact ; AESH de l'élève calculée | `agenda.evenement_publie` | AG7 |
| AGD-332 | Dupliquer mes réservations d'une semaine sur d'autres semaines | personnel | `agenda.lieux.reserver` | rapport par occurrence | `agenda.lieu_reserve` | AG3 |
| AGD-333 | Importer ponctuellement un fichier .ics en plages occupées (sans titre) | personnel | soi | avancé de AG10 à AG2 | — | AG2 |
| AGD-334 | **Sortie de cours neutre** pour un rendez-vous restreint (avis au 12 : « absence autorisée par la vie scolaire ») | système | — | R-AG36 | — | AG4 |
| AGD-335 | **Séparer** les rendez-vous de deux parents sous interdiction (écart, bâtiments, proposition hors soirée) | système | — | R-AG35 | — | AG5 |
| AGD-336 | Placer séparément deux participants sous interdiction dans une réunion collective (organisateur et accueil avertis sans nom) | système | — | R-AG38 | — | AG7, AG9 |
| AGD-337 | Émarger **hors réseau** (mobile), synchroniser ensuite | organisateur | idem | horodatage serveur à la réception | `agenda.presence_saisie` | AG8 |
| AGD-338 | Avis quotidien aux agents des mises en place du lendemain | système | — | socle 8 | — | AG3 |
| AGD-339 | Port `agenda::rdv_faits(eleve, nature, plage)` pour les socles 12 et 13 (preuve de convocation) | système | droit du lecteur | jamais pour une nature restreinte | — | AG4 |

**Total des actions** : 192 (AGD-100 à AGD-291) + 13 (AGD-300 à AGD-312) + 20 (AGD-320 à AGD-339) + 2 (AGD-340, AGD-341, §6.10) = **227 actions**.

### 33.3 Règles ajoutées (R-AG35 à R-AG45)

| N° | Règle |
|---|---|
| R-AG35 | Sous interdiction, adresse protégée ou mesure (socle 4), les rendez-vous de deux parents d'une même campagne sont tenus à **au moins S19-15 (45 min)** d'écart, si possible dans des bâtiments différents ; à défaut, un rendez-vous hors soirée est proposé ; le calcul est serveur et invisible des parents ; l'hôte voit « recevoir séparément » sans nom de l'autre parent. |
| R-AG36 | Un rendez-vous restreint pendant un cours n'est connu du professeur et de l'appel que comme « absence autorisée par la vie scolaire » ; aucune mention de rendez-vous ni d'hôte ; créneau hors cours proposé en priorité ; un rendez-vous restreint récurrent tourne sur des cours différents si possible. |
| R-AG37 | L'occupation de la PsyEN, des infirmiers, des assistants sociaux et du médecin n'est visible que d'eux-mêmes et de la direction ; les autres ne voient que leurs plages ouvertes ; aucune réunion n'est placée sur leurs plages par un tiers sans leur accord. |
| R-AG38 | Une réunion collective où deux participants sont sous interdiction réciproque avertit l'organisateur et l'accueil (« deux participants à ne pas placer ensemble »), sans nom ni motif. |
| R-AG39 | Un parcours de fratrie réserve ce qui tient **après confirmation** de la famille et place le reste en liste d'attente ; jamais de réservation partielle silencieuse (remplace R-AG32). |
| R-AG40 | Tout avis poussé (push, écran verrouillé) d'une nature restreinte porte un texte neutre (« Rendez-vous à 10 h »). |
| R-AG41 | La liste du jour imprimée porte « document interne », heures et noms seulement, et n'est pas réimprimable après J+1. |
| R-AG42 | L'accueil ne donne aucune information sur la présence ou l'heure d'un autre visiteur ; l'écran le rappelle. |
| R-AG43 | Tout texte saisi est neutralisé à la sortie : retours à la ligne et caractères de contrôle échappés dans l'ICS (RFC 5545), cellules commençant par `=`, `+`, `-`, `@` préfixées dans les exports tableur, HTML échappé dans les avis. |
| R-AG44 | La description d'un événement est rendue en texte ; un lien n'est actif que vers un domaine autorisé par l'établissement. |
| R-AG45 | La non-venue est un indicateur **par élève**, lu du seul PP et du CPE pour leurs élèves, effacé à la fin de l'année, jamais exporté ni agrégé par famille. |

### 33.4 Cas limites ajoutés (CAG-61 à CAG-70)

| N° | Cas | Comportement attendu |
|---|---|---|
| CAG-61 | Professeur arrivé la veille de la soirée | hôte ajouté, familles de ses classes prévenues des nouveaux créneaux |
| CAG-62 | Deux soirées le même soir, même professeur | conflit signalé à la génération |
| CAG-63 | Père puis mère (sous interdiction) réservent le même professeur | second rendez-vous à ≥ 45 min ou un autre jour (R-AG35) |
| CAG-64 | Le professeur reçoit l'un après l'autre deux parents sous interdiction | « recevoir séparément », jamais le nom de l'autre parent |
| CAG-65 | Inès (PsyEN) chaque mardi à 10 h pendant le même cours | rotation proposée ; « absence autorisée » seulement (R-AG36) |
| CAG-66 | Famille sans ordinateur qui appelle le secrétariat | AGD-320, avis par SMS si c'est son seul canal (sous Q-N22) |
| CAG-67 | Émargement au gymnase sans réseau | AGD-337 |
| CAG-68 | Réservation importée de Next.js en chevauchement avec une autre | file humaine avant la bascule (M19-065) |
| CAG-69 | Série de réservation qui traverse le changement d'EDT de janvier | occurrences en conflit signalées (AGD-259) |
| CAG-70 | Titre d'événement contenant un retour à la ligne et `BEGIN:VEVENT` | échappé (R-AG43) |

### 33.5 Modèle (compléments, migrations de la plage 2380–2399)

- `ent_agenda_reservation` : + `matiere_code TEXT NULL`, `classe_id UUID NULL` (FK composite), parité NX-06 / NX-17 ; + table `ent_agenda_reservation_besoin (etablissement_id, reservation_id, besoin, quantite)` (mise en place typée) — migration 2386.
- `ent_agenda_rdv` : + `presents_annonces TEXT NULL` (≤ 120 caractères) ; + `pris_pour_compte_par_personne_id UUID NULL` (AGD-320) ; + `interprete_langue TEXT NULL`, `interprete_personne_id UUID NULL` — migration 2384.
- `ent_agenda_session_equipe (etablissement_id, creneau_id, personne_id, role)` (ambassadeurs, enseignants) — migration 2385.
- `ent_agenda_participant` : + `organisateur_externe` pour une réunion reçue (AGD-331) — migration 2382.
- Aucune colonne JSON.

### 33.6 Lots révisés

- **AG2** reçoit R-AG37 et AGD-333.
- **AG4** reçoit R-AG36, R-AG40, AGD-320, AGD-321, AGD-327 à AGD-330, AGD-334, AGD-339, et l'abonnement iCal **des hôtes** (avancé d'AG10, pour la direction qui suit son agenda sur son téléphone).
- **AG5** reçoit R-AG35, R-AG39, AGD-322 à AGD-326, AGD-335, et le test de charge du jour J.
- **AG7** reçoit AGD-331, AGD-336.
- **AG3** reçoit AGD-332, AGD-338 et la parité couleurs / classe.
- **AG11** reçoit l'export différentiel et la file des doublons (M19-064, M19-065).

## Ajout D-058 (2026-11-11) — toutes les salles, disponibilité selon l'EDT réel
Fait foi en cas de contradiction avec le §2.4 point 1 et les lots AG2–AG3. Voir `DECISIONS.md` D-058.
- **Périmètre** : le planning et la recherche de salles portent sur tout le référentiel des lieux du socle 5 (salles de classe, spécialisées, CDI, gymnase, amphithéâtre, extérieurs), filtrables par site, UAI, capacité, équipements et accessibilité.
- **Disponibilité calculée** (AG2) : libre = aucune séance prévue (port `edt::occupe`), **ou** séance supprimée par une modification validée du socle 10 (absence d'un personnel non remplacée, socle 15 ; sortie ou voyage, socle 42 ; stage, socle 39 ; examen, socle 38 ; journée banalisée), moins les réservations existantes. Les salles « libérées » sont marquées et expliquées (« cours de 4e B supprimé ce jeudi »), sans motif personnel.
- **Réglage par salle** : réservable librement / sur validation du responsable / jamais ; plage horaire et publics autorisés ; titulaire prévenu ou non.
- **Priorité du cours** : un remplacement nommé, un cours rétabli ou déplacé dans cette salle retire la réservation concernée (événement du socle 10) ; notification du demandeur avec motif et salles de rechange proposées ; journal. Pas de réservation d'une salle libérée moins de N minutes avant (réglage) si la décision d'absence n'est pas encore validée.
- **Cas limites à tester** : absence déclarée puis annulée ; remplaçant nommé après la réservation ; cours déplacé dans la salle réservée ; série hebdomadaire qui croise une semaine B ; deux demandes pour la même salle libérée (premier arrivé ou validation selon le réglage) ; salle libérée pendant un PPMS ou une indisponibilité technique (socle 44).
- **Lots** : AG2 intègre le calcul (ports 10 et 15) ; AG3 intègre la réservation des salles libérées et le retrait automatique ; tests de bout en bout absence → suppression → proposition → remplacement → retrait.

## Ajout D-059 (2026-11-11) — agenda interne à calques, feuille de semaine, aucun Google Agenda
Fait foi en cas de contradiction. Voir `DECISIONS.md` D-059.
- **Vue « Mon agenda » à calques cochables**, mémorisés par personne : *mes cours* (séances réelles du socle 10, remplacements et suppressions compris, pour tout personnel qui enseigne, direction comprise) ; *mes rendez-vous* ; *mes réunions et convocations* ; *semaine de l'établissement* ; *agendas partagés* (équipe, service, salle, ressource) ; *échéances* (conseils, bulletins, voyages, examens) ; *absences et remplacements me concernant*. Couleur par calque, vue jour, semaine (A/B indiquée), mois, liste ; vue mobile « aujourd'hui ».
- **Feuille de semaine de la direction** (reprise du module Next.js `dashboard-week-sheet`, sans ses défauts) : la direction compose la semaine (événements, sorties, examens, réunions, classes absentes, fermetures, informations), choisit les publics, publie ; des éléments viennent d'eux-mêmes des autres socles (voyages 42, examens 38, conseils 23, instances 46, absences de classe 12) ; publiée comme calque et en PDF ou affichage. Historique des versions publiées.
- **Rendez-vous d'inscription** : créneaux natifs dans l'agenda de la direction (AG5), aucune synchronisation Google par défaut.
- **Lots** : AG1 doit déjà porter le modèle de calques (sources de la vue) ; la feuille de semaine est ajoutée à AG4 ou dans un lot AG1-bis selon le découpage choisi par la session.
