# Socle 10 — L'emploi du temps

**Date :** 2026-11-09 — **Statut :** conception de fond, **non décidée** (à relire par Florian), rédigée **après** le code (lot 7, S2-5) pour le rattraper (`docs/PILOTAGE.md` §4, « rattrapage des domaines codés avant la méthode »). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à l'application Next.js.
**Branche :** `ent-socle-edt-conception` (partie de `ent-mc0hoe`, docs à jour au 2026-11-09). Code évalué sur `ent-integration-lot11` (`1bd6fe4`) et sur les branches du socle 5 non encore intégrées `ent-socle-structure-s3`, `-s4`, `-s7`.
**À lire avec :** `CLAUDE.md` ; `docs/PILOTAGE.md` ; `docs/DECISIONS.md` (D-017, D-019, D-020, D-021, **D-022**, D-023, D-024, **D-025**, D-026, D-028, D-029) ; `docs/VISION.md` ; `docs/SOCLES.md` (socles 10, 12, 14, 15, 19, 38, 42, 44) ; `docs/MATRICE_CHARLEMAGNE.md` §3.6 ; `docs/socles/05_STRUCTURE.md` (cours, calendrier, semaines, grilles, lieux) et `06_PERSONNELS.md` (services, interventions, indisponibilités) avec `PERSONAS_05-06.md` §0 ; `08_NOTIFICATIONS.md` (urgences, catégories) ; `09_INTEROP.md` §9.7 (logiciels d'EDT, iCal) ; `docs/ARCHITECTURE_BRIQUES.md` (branche `ent-architecture-briques` : Q-B2, Q-B15) ; `docs/socles/MIGRATIONS.md`.
**Conventions.** **[F]** fait vérifié dans le dépôt (chemin et ligne cités) ; **[V]** règle lue sur une source officielle (URL citée) ; **[NV]** règle non vérifiée sur une source officielle ; **[H]** hypothèse (pratique du terrain, marché) ; **[R]** recommandation (non décidée). Les actions de ce socle sont numérotées **EDT-100** et suivantes (EDT-01 à EDT-17 sont déjà pris par la matrice et par `SOCLE2_DOMAINES.md`), les règles **R-E1** et suivantes, les cas limites **CL-01** et suivants, les contraintes dures **H-01**, souples **G-01**, les questions **Q-EDT1** et suivantes, les réglages signalés **S-80** et suivants (après S-79 du socle 9).
**Plage de migrations :** **4000–4099** si la règle « 3000 + 100 × numéro de socle » (Q-B2) est adoptée ; **repli 440–459** sinon (registre `MIGRATIONS.md`).
**Méthode.** Lecture des décisions, de la carte et des conceptions des socles 5, 6, 8, 9 ; lecture directe des migrations EDT (021, 042, 048, 060, 083, 110, 150, 170, 171, 172, 333, 334, 336, 337, 341, 370) ; audit indépendant du code (sous-agent de revue, lecture seule), dont les trois défauts graves ont été relus ligne à ligne ; recherche réglementaire sur Légifrance et sites officiels (sous-agent), dont les deux textes décisifs ont été relus par une seconde lecture.

Sommaire : §0 En bref · §1 Thèse · §2 Évaluation de l'existant · §3 Acteurs · §4 L'année de l'EDT · §5 Réglementaire · §6 Frontières · §7 Concepts · §8 Du plan aux séances · §9 Modifier le réel · §10 Cas limites · §11 Génération automatique · §12 Vœux · §13 Échanges · §14 Publication et notifications · §15 Droits et minimisation · §16 Actions · §17 Règles · §18 Événements · §19 Écrans · §20 Contrat IA · §21 Modèle de données · §22 Plan de migration · §23 Lots · §24 Tests et recette · §25 Réglages signalés · §26 Questions à Florian · §27 Faits, hypothèses, contradictions · §28 Les cinq décisions à prendre en priorité.

---

## 0. En bref

1. **L'existant fonctionne pour une démonstration, pas pour un établissement.** Le lot 7 a livré créneaux, séances datées, exceptions, déplacements, annulation de masse, remplacements, iCal et quatre écrans Leptos, avec de bons réflexes (aperçu, idempotence, outbox, motif public). Mais il a été construit avant le cours du socle 5, sans version, sans fuseau, et il a **quatre défauts graves** : la régénération des séances **efface le cahier de textes** (D1), **tout professeur lit l'emploi du temps — donc les groupes — de n'importe quel élève** contre D-022 (D2), **les motifs d'absence des professeurs sont visibles de tous les professeurs** (D3), **aucune RLS** sur les tables (D4). Un lot de correctifs (T0) doit passer avant tout le reste (§2.2).
2. **Trois objets au lieu d'un** : le **plan** (emploi du temps type, versionné, publié à une date), la **séance datée** (le réel, identité stable, jamais supprimée quand quelque chose s'y attache), la **modification** (écart daté, motivé, tracé, avec effets automatiques). Le créneau vise le **cours** (socle 5), jamais un texte ; l'intervenant vient du socle 6 à la date (co-enseignement, suppléance, poste vacant) (§1, §8, §9).
3. **Génération automatique dans le socle 10** (D-025) : moteur **maison en Rust, déterministe, explicable, reprenable, comparable** ; diagnostic de faisabilité en secondes dès mars ; contraintes dures (H-01 à H-12) et souples (G-01 à G-14) à trois niveaux de poids, sans trente curseurs ; vœux des professeurs avec équité et retour motivé ; aucun code de FET (AGPL-3.0 [V]) ; prototype mesuré (T8a) avant d'engager la suite (§11, §12).
4. **Deux textes changent la conception** [V] : l'**arrêté du 14/08/2023** impose aux logiciels de vie scolaire du public une **remontée hebdomadaire par API** des heures prévues à l'EDT, absentes et remplacées, avec un **motif d'absence catégorisé** (R-09) ; l'**arrêté du 21/03/2018** prévoit que les élèves et leurs responsables voient la civilité, le nom et la matière des enseignants, **sans aucune absence** (R-11). Le second degré n'a **aucune règle nationale** sur la pause méridienne, l'amplitude ou la durée d'une séance (R-16) : ce sont des contraintes d'établissement.
5. **Ce que chacun voit est minimal** : famille et élève voient l'EDT daté de l'enfant et le nom des intervenants, jamais un motif ; le professeur voit l'EDT nominatif des seuls élèves de ses cours (D-022) ; l'accueil et la vie scolaire savent **où est un élève maintenant** par une lecture unitaire journalisée ; les vœux d'un professeur ne sont lus que par lui et la direction (§15).
6. **La journée réelle** : tableau « demain » prêt la veille, « maintenant » pour la vie scolaire et l'accueil, grève du matin en une liste à cocher, fermeture déclarée une fois (socle 5) qui annule, prévient et rétablit, un seul message par changement et par personne, à l'heure juste (§14).
7. **86 actions** (EDT-100 à EDT-198), **63 cas limites**, **20 règles**, **15 lots** de code ordonnés (T0 à T13, dont T8a et T8b) avec critères de fin ; parcours minimal vendable : T0 à T5, T9, T12a–c (§16, §10, §17, §23).
8. **Cinq décisions à prendre en priorité** : §28.

---

## 1. Thèse : l'emploi du temps est une promesse datée, pas une grille

Pour l'élève, la famille et le professeur, l'emploi du temps est la réponse à trois questions : **où dois-je être, quand, avec qui ?** Pour l'établissement, c'est en plus un **engagement** (les heures dues aux élèves selon les horaires réglementaires, les heures dues par chaque professeur selon son service) et un **outil de travail** de chaque jour (qui est absent, quel cours n'aura pas lieu, quelle salle est libre, où est la 4B à 10 h 15).

Le code existant (lot 7, S2-5) a été construit autour d'un objet : le **créneau hebdomadaire** (`ent_edt_creneau` : « lundi 8 h–9 h, 4B, maths, M. X, salle 12 »), complété ensuite par des occurrences datées (`ent_edt_seance`, migration 110), des exceptions (042), des déplacements (170), des annulations de masse (171). Ce socle part de là et pose douze principes [R] :

1. **Trois objets, pas un.** (a) le **plan** : l'emploi du temps type, versionné, valable sur une plage de dates (« EDT du 2 septembre », « EDT après les conseils du 1er trimestre ») ; (b) la **séance datée** : ce qui doit réellement se passer tel jour, à telle heure, pour tel cours, avec tels intervenants, dans tel lieu ; (c) la **modification** : un écart daté, motivé et tracé entre le plan et le réel (annulation, déplacement, changement de salle, remplacement, ajout, fusion). Aujourd'hui le plan est un ensemble plat de créneaux modifiés sur place, et les modifications sont dispersées dans trois tables.
2. **Le créneau vise un cours, jamais un texte.** Le cours (socle 5, `ent_cours`, lot S4) dit *ce qui est enseigné à qui* ; les interventions (socle 6) disent *qui l'enseigne* ; le créneau dit seulement *quand et où*. Tout le reste (public, visibilité D-022, appel, notes, cahier) se déduit du cours. Le champ texte `classe` et le `prof_user_id` du créneau deviennent des projections, puis disparaissent.
3. **La séance datée est matérialisée, unique et stable.** Une séance a un identifiant qui ne change pas quand on régénère : l'appel, le cahier de textes, une absence à la séance (D-020), un travail donné, une évaluation, un remplacement s'y attachent. Régénérer ne détruit jamais une séance qui porte quelque chose ; elle est **retirée** (avec motif), jamais supprimée.
4. **Une modification se déclare une fois et produit tous ses effets.** Annuler le cours de 14 h : l'appel ne l'attend plus, le cahier de textes signale le travail prévu, les élèves et les familles sont prévenus selon l'urgence (socle 8), l'étude ou la permanence est prévenue, la sortie anticipée est calculée selon le régime de sortie (socle 14), la cantine n'est pas concernée (ou l'est, si c'est toute la journée). Aucune double saisie.
5. **Le motif d'une absence de professeur n'est jamais diffusé.** Les familles apprennent qu'un cours n'a pas lieu ou qu'il est remplacé, jamais pourquoi. Le motif vit au socle 15, à droits restreints.
6. **La génération est un assistant, pas une boîte noire.** ScolIA construit l'emploi du temps (D-025) avec un moteur **déterministe** (mêmes données, même réglage → même résultat), **explicable** (pour chaque cours non placé, la raison ; pour chaque contrainte souple non respectée, le coût et l'alternative), **reprenable** (on verrouille ce qui convient et on ne recalcule que le reste), **comparable** (plusieurs scénarios côte à côte, avec leurs indicateurs). L'adjoint décide ; le moteur propose.
7. **Le plan se publie, il ne se « sauve » pas.** Un emploi du temps en construction est un **brouillon** invisible des familles et des élèves ; il devient réel par une **publication** datée (à partir de quel lundi), après un contrôle serveur (conflits, heures dues, salles), avec un aperçu des séances qui changent. Une publication en cours d'année ne casse rien de ce qui est passé.
8. **L'emploi du temps se lit à une date, pour une personne.** « Mon emploi du temps » d'un élève est calculé : son public (classe, groupes, inclusions, exclusions — socle 5 `ent_public_cours_jour`), à cette date, croisé avec les séances. Un élève qui change de groupe le 4 novembre voit son nouvel emploi du temps à partir du 4 novembre, et l'ancien avant.
9. **Ce que chacun voit est minimal.** La famille voit l'emploi du temps de son enfant (si son lien lui en donne le droit, socle 4) ; l'élève le sien ; le professeur le sien, ceux de ses cours et les salles ; la vie scolaire et l'accueil savent où est un élève maintenant (pour une remise, une urgence), sans fouiller une semaine entière ; personne d'autre que la direction ne voit les vœux et contraintes personnelles d'un professeur, et le **motif** d'une indisponibilité n'est jamais demandé.
10. **Le temps est local.** Toutes les heures sont des heures murales du fuseau de l'établissement (profil du socle 5, `ent_site.fuseau_horaire`, migration 337) ; les instants (notifications, iCal) sont calculés à partir de ce fuseau, changements d'heure compris. Un établissement de La Réunion n'a jamais un cours « à 4 h du matin ».
11. **On échange sans ressaisir, sans écraser.** STS-web, le logiciel d'EDT du marché, un tableur : tout ce qui entre devient une **proposition** (socle 9), rapprochée et différenciée de l'existant, appliquée après validation ; tout ce qui sort est tracé (D-022 : exports réservés aux administratifs).
12. **Peu de réglages, de bons défauts** (D-023). Une grille, des semaines A/B si besoin, des salles typées, des vœux simples ; les poids de la génération ont des valeurs par défaut sensées et des curseurs à trois positions, pas trente paramètres numériques. Les réglages candidats à l'allègement sont signalés (§26).

**Ce qui fera la différence au quotidien** [H — comparaison de mémoire avec Pronote/EDT, École Directe, Charlemagne, non vérifiée au détail] : le tableau **« demain »** de l'adjoint et de la vie scolaire (qui manque, quels cours tombent, quelles salles sont libres, qui est prévenu) ; le **remplacement en trois gestes** (absence → séances touchées → remplaçant proposé avec la raison → familles prévenues) ; la **fermeture ou la grève déclarée une fois** (socle 5, STR-94) qui annule, prévient, recalcule les sorties ; l'emploi du temps **juste à la date** (changement de groupe, inclusion ULIS, stage, examen) ; une **génération intégrée** qui lit directement la structure, les services et les vœux, sans export vers un autre logiciel puis réimport ; et l'**explication** de chaque choix du moteur.

---

## 2. Évaluation franche de l'existant

Lu sur `ent-integration-lot11` (`1bd6fe4`, dernière intégration au 2026-11-09), plus les branches non intégrées du socle 5 qui touchent l'EDT : `ent-socle-structure-s3` (calendrier 333–334), `ent-socle-structure-s4` (cours 341, avec `ent_edt_creneau.cours_id`), `ent-socle-structure-s7` (grilles 336). Lecture seule, rien exécuté. Un audit indépendant (`review-sonnet`) a relu le code ; les trois défauts les plus graves (D1, D2, D3) ont été **relus directement** (`150_cahier_textes.sql:14`, `referentiel_seances.rs:381-396`, `edt_admin.rs:964-971`, `service.rs:7878-7887`). Les numéros de ligne sont ceux de cette branche.

### 2.1 Ce qu'il y a [F]

| Brique | Où | Ce qu'elle fait |
|---|---|---|
| Créneau hebdomadaire | `ent_edt_creneau` (021, 048, 065, 110, 111, 170, 370 ; `cours_id` en 341 sur S4) | jour de semaine, heures, `classe` texte (+ `classe_id` par déclencheur), `groupe_id`, `service_id`, `matiere_libelle`, `prof_user_id NOT NULL`, `salle` texte, `site` texte, `semaine_ab` ; **ni année ni validité datée, aucune unicité** |
| Séance datée | `ent_edt_seance` (110:195-221) ; `referentiel_seances.rs` (`planifier_seances`, `ref_generer_seances`) | occurrence matérialisée, unique (créneau, jour) ; statut `prevue/annulee/remplacee` ; support du cahier de textes (150) ; **pas de `groupe_id`** |
| Exceptions | `ent_edt_exception` (042, 171) | annulation, remplacement, changement de salle ; une seule par (créneau, jour) ; `motif` libre obligatoire ; rattachement au lot et à l'absence |
| Déplacement | `ent_edt_deplacement` (170) ; `edt_remplacement.rs:1203` | nouvelle date et heures, séance de source `deplacement` |
| Annulation de masse | `ent_edt_annulation_masse` (171) ; `edt_masse.rs` | lot avec clé d'idempotence, aperçu, portées établissement/niveau/classes, motif public |
| Semaines A/B | `ent_edt_calendrier_ab` (048) ; `domain-eleve/src/edt.rs:48-56` | **un** lundi de référence par établissement, parité continue à travers les vacances |
| Contraintes | `ent_edt_contrainte` (083) | indisponibilité de professeur, de salle (code texte), préférence |
| Import | `ent_edt_import` (060) ; `service.rs:7621-7762` ; `POST /edt/import`, `/edt/resoudre` | CSV maison ou « Pronote simplifié », essai à blanc, journal ; solveur glouton sur une grille codée en dur (`domain-eleve/src/edt.rs:643`) |
| Administration | `edt_admin.rs` | création, modification, suppression de créneau avec aperçu, verrou consultatif, détection de conflits (professeur, salle, classe, groupe, site, indisponibilité) ; vue semaine `GET /edt/semaine` (personnel, classe, salle, élève) |
| Remplacements | `edt_remplacement.rs` ; `ent_absence_rh` (080) ; `ent_delegation` (172) | absences de personnel, séances touchées, candidats (libres, sans conflit, « compétents » par libellé de matière), remplacement avec délégation datée, annulation, changement de salle, levée |
| iCal | `ent-api/src/ical.rs` ; `/eleves/{id}/edt.ics`, `/edt/mes-creneaux.ics` | export **sous session** sur 62 jours, fuseau Europe/Paris en dur |
| Notifications | `edt_reactions.rs` (consommateur `d4.notifications_edt`) | notifications internes des changements |
| Écrans Leptos | `modules/edt` (`/emploi-du-temps`, jour), `edt_semaine` (`/edt/semaine`), `changements_edt`, `remplacements` | consultation jour et semaine, changements par enfant, absences et remplacements, annulation de masse avec aperçu ; **pas** d'écran de création de créneau, de conflits, de déplacement, d'import, de semaines A/B, de contraintes, de génération |
| Tests | `ent-db/src/tests_edt_s2.rs` (2 090 lignes), `ent-api/src/tests_edt_s2.rs`, `web/.../tests/edt_s2.rs` | conflits, A/B, vacances, remplacement et délégation, isolation entre établissements, annulation de masse, déplacement |

### 2.2 Défauts trouvés (à corriger avant toute extension)

Gravité : **G** grave (sécurité ou perte de données), **I** important (résultat faux), **C** confort ou dette.

| N° | Gravité | Défaut | Preuve | Correction (lot) |
|---|---|---|---|---|
| D1 | **G** | **Le cahier de textes est effacé** par la régénération des séances : `ref_generer_seances` supprime physiquement toute séance « plus attendue » (fermeture ajoutée, jour férié, créneau déplacé) et `ent_seance_contenu.seance_id` est `ON DELETE CASCADE` ; le travail à faire (`150:29`) perd sa séance (`ON DELETE SET NULL`) | `referentiel_seances.rs:381-396` ; `150_cahier_textes.sql:14, 29` [relu] | T0 : plus aucune suppression de séance qui porte quelque chose (état `retiree`) ; clé étrangère `RESTRICT` ; test |
| D2 | **G** | **D-022 non appliquée à la lecture de l'EDT d'un élève** : tout personnel lit l'EDT (donc les groupes : LV2, options, ULIS) de n'importe quel élève ; même chose pour l'iCal élève et les changements | `edt_admin.rs:970-971` (`r if r.is_staff() => {}`) [relu] ; `service.rs:7433-7435` ; `routes.rs:8585` ; `edt_masse.rs:297` | T0 : portée `edt.lire_eleve` (mes classes) via le périmètre unique ; tests de séparation |
| D3 | **G** | **Motifs d'absence visibles de tous les professeurs** : `GET /edt/exceptions` est ouvert à tout rôle qui fait l'appel et renvoie le `motif` libre et obligatoire (ex. « maladie ») ; la liste des absences de personnel est ouverte à tout le personnel avec son motif | `service.rs:7878-7887` [relu] ; `service.rs:15398-15401` ; `domain-eleve/src/edt.rs:573-575` | T0 : séparer motif public (liste) et motif interne (ADJ/DIR) ; DTO sans motif interne ; tests |
| D4 | **G** | **Aucune RLS** sur les tables EDT (`ent_edt_*`, `ent_salle`, `ent_absence_rh`, `ent_delegation`) : le cloisonnement repose sur les seules requêtes | listes des migrations 100 et 206 ; 337:255 (RLS sur les nouvelles tables de lieux, pas sur `ent_salle`) | T0 (RLS permissive), T11 (stricte) |
| D5 | **I** | L'appel ignore le calendrier (vacances, fermeture), ne peut pas être fait sur un cours **déplacé**, part de la classe texte (sans les inclusions ULIS du socle 5), n'est pas transactionnel | `service.rs:8052-8173` (8068, 8082, 7986-7989, 8139-8152) | T2 (appel ouvert depuis la séance) ; coordination socle 12 |
| D6 | **I** | **Deux chemins d'écriture** d'exceptions : l'ancien (`poser_edt_exception_for_actor`) écrase toute exception (`ON CONFLICT`), y compris un déplacement ou un lot, ne met pas à jour la séance, accepte n'importe quel membre comme remplaçant | `service.rs:7511-7598` (7533, 7550) | T1 : retrait de l'ancien chemin, une seule commande |
| D7 | **I** | **Pas de version ni d'année** : créneaux joints à la classe **par texte** sans année (homonymie d'une année sur l'autre) ; la vue semaine est recalculée depuis les créneaux courants et **réécrit le passé** ; deux sources de vérité (séance stockée / plan recalculé par `edt_plans`) | `edt_admin.rs:768, 827-829` ; `referentiel_seances.rs:254-256` | T1–T3 : versions, séance seule source |
| D8 | **I** | **Fuseau** : `today_utc` partout, iCal en Europe/Paris en dur, coupure de demi-journée à 12 h 30 en dur alors que la grille du socle 5 et `ent_site.fuseau_horaire` existent | `referentiel_seances.rs:186, 243` ; `routes.rs:8500` ; `ent-api/src/edt_s2.rs:95` ; `ical.rs:94` ; `edt_remplacement.rs:1483` | T0 (date du jour), T2 (instants), T9 (iCal) |
| D9 | **I** | **Import** : identifiant de professeur non vérifié comme membre de l'établissement (format natif), classe et salle non validées, pas de transaction, réimport impossible (conflits avec soi-même), faux conflits de demi-groupes, CSV sans guillemets | `service.rs:7621-7762` (7691-7708) ; `domain-eleve/src/edt.rs:288, 485` | T0 (contrôle d'appartenance), T10 (reprise dans le moteur du socle 9) |
| D10 | **I** | **Déplacement le même jour** : aucune séance créée (`if nouveau_jour != jour`), le cours disparaît ; `ON CONFLICT DO NOTHING` peut avaler la séance déplacée | `edt_remplacement.rs:1450, 1467` | T2 |
| D11 | **I** | **Absences de personnel** : la validation écrase l'auteur de la déclaration ; le refus est inatteignable ; une absence seulement « signalée » compte comme absence ; droit par rôle codé, pas par action | `absence_rh.rs:64-68` ; `edt_remplacement.rs:307` ; `service.rs:15338` | socle 15 (relevé ici, corrigé là-bas) |
| D12 | **I** | **Remplaçant** : il « suit » les élèves du titulaire pendant ±14 jours autour du remplacement | `perimetre.rs:33, 96` | T0 : portée limitée au jour de la séance remplacée |
| D13 | **I** | **Notifications** : destinataires calculés par classe texte et groupe (sans inclusions), au premier jour seulement pour un lot ; remplaçant non prévenu d'une annulation ; consommateur non rejouable | `edt_reactions.rs:66-79, 358, 400` | T4 (avec le socle 8) |
| D14 | **I** | **iCal élève** : ignore exceptions, calendrier et groupes (cours annulés présents, tous les groupes de la classe affichés) ; ni `STATUS:CANCELLED` ni `SEQUENCE` ; lignes pliées au-delà de 75 octets ; pas d'abonnement possible (session) | `service.rs:7455-7467` ; `ical.rs:30` | T9 (socle 9, X15) |
| D15 | **I** | **Lien au cours absent** : `service_id` jamais posé par la création admin ni par l'import ; remplaçant « compétent » par comparaison de libellés | `edt_admin.rs:541-560, 630-651` ; `edt_remplacement.rs:668-671` | T1 (`cours_id` obligatoire pour toute nouvelle écriture) |
| D16 | **C** | **Doublons de modèles** : « cours annulé » écrit à trois endroits (exception, séance, déplacement) ; salles en 076 et 337 avec deux chemins de création ; indisponibilités en 083 et en 337 (`ent_disponibilite`, `ent_salle_indisponibilite`) ; site en trois endroits (`creneau.site` texte, `classe.site_id`, `salle.site_id`) ; migrations 170 et 370 identiques (même colonne, même index) | migrations citées | T1–T3, T11 |
| D17 | **C** | **Co-enseignement impossible** (un seul professeur, `NOT NULL`) ; une seule salle texte | `021` ; `edt_admin.rs:115-123` | T1 |
| D18 | **C** | **Performance** : 3 requêtes par séance dans l'annulation de masse, 5 à 6 par professeur pour les candidats remplaçants, iCal jour par jour, conflits en O(n²), 5 appels séquentiels sur la page « mon EDT » | `edt_masse.rs:225-269` ; `edt_remplacement.rs:688-703` ; `routes.rs:8535-8551` ; `web/.../edt/mod.rs:33` | lots concernés ; objectif §24 |
| D19 | **C** | Un membre du personnel lit l'EDT détaillé d'un collègue (`GET /edt/semaine?cible=personnel`) ; une salle utilisée peut être désactivée | `ent-api/src/edt_s2.rs:84` ; `edt_admin.rs:719` | T0 (présence oui/non pour les collègues) |
| D20 | **C** | Tests d'intégration conditionnés à une base de labo (`labo()` renvoie une `Option`) : sans base, ils passent sans rien vérifier (déjà relevé aux socles 3, 5, 9) | `tests_edt_s2.rs` | règle du kit : échec si la base manque en CI |

### 2.3 Ce qui est à garder, à corriger, à remplacer

| Verdict | Quoi | Pourquoi |
|---|---|---|
| **Garder** | Logique pure de conflits (`semaines_compatibles`, `conflits_entre`, `detecter_conflits`, `edt_admin.rs:91-188`) ; `planifier_seances` (testée sans base) ; aperçu avant écriture ; patron outbox, clés d'idempotence, verrous consultatifs ; notion de **motif public** ; délégation datée de remplacement (à borner, D12) ; contrôles famille/élève de `edt_acces_eleve` (hors cas du personnel) ; l'annulation de masse avec aperçu ; la table `ent_edt_seance` comme **socle** de la séance cible | Bien orientés, testés ; c'est la base du modèle cible |
| **Corriger tout de suite** (lot T0, avant tout autre lot du socle) | D1, D2, D3, D4 (RLS permissive), D8 (date du jour), D9 (appartenance du professeur importé), D12, D19 | Sécurité et perte de données : ne peuvent pas attendre la refonte |
| **Remplacer** | Le créneau plat (→ placement d'une **version** visant un **cours**, lieux par identifiant, intervenants du socle 6) ; les trois tables de modification (→ une table de **modifications** de séance) ; `ent_edt_calendrier_ab` (→ semaines du socle 5, 333) ; `ent_edt_contrainte` (→ indisponibilités du socle 5 pour les lieux, contraintes et vœux du socle 10) ; l'ancien chemin `edt_repo.rs` + `poser_edt_exception_for_actor` + `list_mes_creneaux` + `list_edt_eleve_jour_for_actor` ; l'iCal sous session (→ abonnement à jeton, socle 9 X15) ; l'import CSV maison (→ moteur d'échanges du socle 9) ; le solveur glouton sur grille en dur (→ moteur du §11) | Construits avant le modèle (cours, versions, fuseau, publics datés) ; les corriger sur place coûterait plus cher que de les remplacer à l'abri d'une bascule |
| **Déplacer** | La génération des séances (`referentiel_seances.rs`) vers le socle 10 (Q-S11 du socle 5) ; les absences de personnel et la recherche de remplaçant vers le socle 15 (le socle 10 garde la **modification de séance**) | Frontières (§6) |

**Écrans Leptos** : `edt_semaine` (vue semaine, statuts, badge A/B, navigation) est la bonne base du composant « emploi du temps » cible (§19, écran 1 et 2) ; `changements_edt` devient la liste « changements » ; `remplacements` part au socle 15 (sa partie annulation de masse reste ici) ; `edt` (vue jour, ancienne API) est remplacé par la vue jour du composant commun. Les méthodes du client web déjà écrites mais inutilisées (`creer_creneau`, `creneaux_admin`, `conflits_edt`, `changer_salle`, `deplacer_seance`, `ent-web-api/src/edt_s2.rs`) seront rebranchées sur les nouvelles routes.

---

## 3. Acteurs, besoins réels, irritants

Sources : expérience du terrain de Florian (secrétariat de lycée, utilisateur de Charlemagne) rapportée dans `PERSONAS_05-06.md` et `MATRICE_CHARLEMAGNE.md` ; connaissance générale des pratiques [H, non vérifiée au détail]. Abréviations utilisées dans les tableaux d'actions : **DIR** chef d'établissement, **ADJ** adjoint (ou personne chargée de l'emploi du temps), **SEC** secrétariat, **CPE**, **VS** vie scolaire (AED compris), **PRF** professeur, **PP** professeur principal, **ELV** élève, **FAM** responsable légal, **AGT** agent (entretien, restauration, maintenance), **ACC** accueil, **DOC** documentaliste, **INF** infirmerie, **SYS** le système (tâche ou réaction à un événement).

| Acteur | Ce qu'il fait avec l'EDT | Ce qui lui fait perdre du temps aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Chef d'établissement** | Arrête les grands choix (grille, pause méridienne, mercredi après-midi, alignements), arbitre les vœux, valide la publication ; répond aux familles et à l'inspection | Ne pas savoir si l'EDT respecte les horaires des élèves et les services avant la rentrée ; arbitrer à l'aveugle des vœux contradictoires ; ne pas voir le coût d'une décision (« si je libère le mercredi, qu'est-ce que je perds ? ») | Un EDT publié faux (heures manquantes, professeur en deux lieux) ; un conflit social sur l'équité des vœux ; une plainte de famille sur des trous | Un **tableau de qualité** (heures dues couvertes, trous élèves, vœux satisfaits par professeur, équité) ; la **comparaison de scénarios** ; une publication sûre en un geste |
| **Adjoint / chargé de l'EDT** | Recueille DHG, répartition, vœux, contraintes ; construit l'EDT (souvent dans un logiciel à part), l'exporte vers STS et l'ENT ; le modifie toute l'année ; gère absences et remplacements au quotidien | **Double et triple saisie** : structure ressaisie dans le logiciel d'EDT, export vers STS, import dans Pronote/ENT, puis toutes les modifications refaites deux fois ; groupes renommés à chaque export ; barrettes perdues ; vœux reçus par mail ou papier ; recherche manuelle d'un remplaçant (« qui est libre jeudi à 10 h et connaît la classe ? ») ; salles réservées dans un tableur à part ; changements de mi-année (groupes de besoin, options) qui obligent à republier | Publier une version qui casse l'appel ou les notes ; perdre des modifications à la republication ; un import qui écrase les ajustements ; la semaine de la rentrée | Une **seule** source (structure, services, EDT dans le même outil) ; la génération qui respecte ses verrous ; le **remplaçant proposé avec la raison** ; la **liste « demain »** prête à 7 h ; republier sans perdre les modifications ponctuelles |
| **Secrétariat** | Imprime les EDT de classe pour la rentrée, répond aux familles (« mon fils a-t-il cours demain ? »), saisit les inscriptions en cours d'année (nouvel élève : quel EDT ?) | Imprimer et distribuer ; répondre au téléphone sur des changements non annoncés ; EDT papier faux après une modification | Donner une mauvaise information à une famille | **L'EDT de l'élève à jour** dès son affectation ; une page « changements du jour » ; impression propre en un clic (trames prêtes, D-023) |
| **CPE et vie scolaire (AED)** | Savent où sont les élèves ; gèrent les permanences, les études, les heures libérées, les sorties anticipées ; vérifient les appels | Ne pas savoir qu'un cours est annulé avant que 30 élèves arrivent en vie scolaire ; liste de permanence faite à la main ; sorties anticipées refusées ou autorisées « au jugé » | Un élève mineur sorti alors qu'il n'y était pas autorisé ; un élève introuvable en cas d'alerte (PPMS) | **« Qui est en permanence maintenant ? »** calculé (élèves sans cours, régime de sortie) ; alerte avant l'arrivée des élèves ; **« où est X maintenant ? »** en un geste, pour une remise ou une urgence |
| **Professeur** | Consulte son EDT (web, mobile, agenda personnel), fait l'appel par séance, donne ses vœux et indisponibilités, demande un déplacement ou une salle, signale une sortie | Vœux non pris en compte sans explication ; trous (« des heures de trou entre deux cours ») ; changement de salle appris en arrivant devant une porte fermée ; deux agendas (le sien et celui de l'établissement) ; services partagés sur deux établissements mal coordonnés | Une semaine A/B illisible ; un remplacement imposé sans préavis ; être affiché comme « absent » sans raison aux familles | **Ses vœux avec un retour** (« satisfait / pas satisfait, parce que… ») ; un **iCal** juste et à jour ; demander un échange de séance avec un collègue en deux gestes ; les changements **avant** d'arriver |
| **Professeur principal** | Suit l'EDT de sa classe, organise les heures de vie de classe, les réunions | Heure de vie de classe impossible à caser ; réunions parents-professeurs sur des créneaux où des collègues ont cours | — | Un **créneau commun libre** de l'équipe trouvé automatiquement (lien socle 19) |
| **Élève** | Consulte son EDT (mobile surtout), apprend les annulations, les changements de salle | Information de dernière minute ; EDT de groupe incompréhensible (« ai-je allemand cette semaine ? ») | Venir pour rien ; se tromper de salle | **Son** EDT à lui, juste (groupes, semaine A/B résolue), changements poussés sur le téléphone |
| **Élève majeur** | Idem, et gère seul ses informations (socle 2, capacité) | — | — | Idem |
| **Famille** | Consulte l'EDT de chaque enfant, apprend les absences de professeurs et les annulations, s'organise (garde, transport, cantine) | Apprendre la veille au soir (ou pas du tout) qu'un cours de 8 h est annulé ; deux enfants, deux établissements, deux applis ; parents séparés dont un seul est prévenu | Que son enfant mineur sorte plus tôt sans qu'elle le sache | Une **notification juste à temps** (pas à minuit, pas trop tard) ; **toute la fratrie** dans une vue ; l'agenda familial abonné (iCal) |
| **Agents** (entretien, restauration, maintenance) | Savent quelles salles sont occupées (ménage, travaux), combien de repas | Ménage dans une salle occupée ; travaux planifiés sur une salle de TP en plein cours ; effectifs de cantine faux un jour de sortie | — | **Occupation des salles** du jour ; effectifs attendus à la cantine mis à jour quand une classe sort ou qu'une journée est banalisée (socle 40) |
| **Accueil** | Oriente les visiteurs, reçoit les parents qui viennent chercher un enfant, prend les appels | Chercher dans un classeur où est la 3C ; ne pas savoir si un professeur est là | Remettre un enfant à une personne non autorisée (socle 14) | **« Où est X maintenant ? »** et **« ce professeur est-il dans l'établissement ? »** (oui / non, sans détail), dans le périmètre de son rôle |
| **Documentaliste, infirmerie** | CDI ouvert aux heures libres ; infirmerie qui doit savoir de quel cours vient un élève | Élèves envoyés au CDI sans prévenir | — | L'élève qui arrive : son cours en cours et le professeur à prévenir |
| **Direction d'un ensemble scolaire / multi-sites** | Mutualise des professeurs et des salles entre sites | Temps de trajet entre sites ignoré | Un professeur à 8 h 55 sur un site et 9 h 00 sur l'autre | **Temps de déplacement** entre sites respecté par la génération |
| **Professeur en service partagé** (deux établissements, éventuellement deux tenants ScolIA) | Donne au second établissement ses indisponibilités | Chaque établissement fait son EDT sans l'autre | Que ses deux EDT se chevauchent | Déclarer une indisponibilité **sans dire où** (cas P2 du socle 6) ; pas de visibilité entre établissements (D-013) |
| **Inspection, rectorat / DDEC** | Contrôle les services (STS-web), les horaires des élèves | Données incohérentes entre EDT et STS | — | Un export propre et des **contrôles d'heures** avant export |

---

## 4. L'année de l'emploi du temps vue du terrain

[H — calendrier typique d'un collège ou lycée de métropole ; les dates varient selon l'académie et le réseau ; ce qui est réglementaire est au §5.]

| Période | Ce qui se passe | Qui | Socles | Ce que ScolIA doit permettre |
|---|---|---|---|---|
| **Janvier – mars** | Dotation horaire (public : DHG ; privé sous contrat : dotation des contrats [NV]) ; structure prévisionnelle N+1 ; tableau de répartition des moyens (TRM) | DIR, ADJ | 5 (dotation, structure N+1), 6 (répartition N+1) | Lire les besoins horaires par cours prévisionnel ; simuler un EDT « faisable ? » très tôt (diagnostic de capacité, §11.3) |
| **Avril – mai** | Grandes options d'organisation (grille, pause méridienne, mercredi, semaines A/B, alignements de langues et spécialités) ; **campagne de vœux des professeurs** | DIR, ADJ, PRF | 10 | Campagne de vœux ouverte, rappels, synthèse des contradictions (§12) |
| **Juin** | Répartition des services N+1 (socle 6, PRS-50) ; constitution des classes (socle 5) ; examens (salles réquisitionnées, cours suspendus) | DIR, ADJ, SEC | 5, 6, 10, 38 | EDT de juin à effets « examen » (cours suspendus, surveillances) ; préparation N+1 sans toucher N |
| **Juillet** | **Construction** de l'EDT N+1 (souvent pendant les vacances) | ADJ | 10 | Génération, scénarios, verrous, comparaisons ; tout sur l'année en préparation |
| **Fin août** | Ajustements (mutations tardives, nouveaux arrivants, postes vacants) ; **export vers STS-web** (services) ; publication aux personnels ; impression des EDT de classe | ADJ, SEC | 10, 9, 6 | Reprise partielle (un professeur arrive : ne recalculer que ses cours) ; publication **aux personnels d'abord**, aux familles ensuite |
| **Rentrée** | Journées d'accueil (niveaux décalés), EDT provisoire la première semaine, salles à corriger | ADJ, VS | 10, 5 | EDT **provisoire** daté (première semaine), puis définitif ; changements de salle en lot |
| **Septembre – octobre** | Dédoublements, groupes de besoin, options qui ouvrent ou ferment, élèves qui changent de groupe ; validation des services STS | ADJ, SEC | 10, 5, 6, 9 | **Version 2** de l'EDT publiée à une date, sans casser les séances passées |
| **Toute l'année** | Absences de professeurs, remplacements, sorties, réunions, conseils de classe, devoirs surveillés, intempéries, grèves, travaux | ADJ, VS, PRF | 10, 15, 42, 23, 19 | Modifications datées, tableau « demain », notifications justes à temps |
| **Décembre, mars, juin** | Conseils de classe en fin de période : cours libérés ou réduits, salles occupées | DIR, ADJ | 23, 19 | Journée à **horaires réduits** ciblée (niveau) ; réservation de salles ; présence des professeurs aux conseils (EDT-17 de la matrice) |
| **Fin d'année** | Fin des cours avancée pour les classes à examen, bascule | DIR | 5, 38 | Fin des cours **par niveau** (calendrier ciblé, socle 5) ; archivage de l'EDT de l'année |

**Ce que vit l'adjoint en juillet** [H] : il reçoit la répartition (souvent sur tableur), reconstruit la structure dans le logiciel d'EDT (classes, groupes, alignements), saisit les vœux reçus sur papier, lance une génération, corrige à la main pendant des jours, exporte vers STS-web, puis vers l'ENT. À la moindre modification de structure en août (une classe ouverte), il recommence une partie du travail dans trois outils. **ScolIA supprime les deux exports et l'import** quand l'établissement construit son EDT dans ScolIA, et **réduit à un import contrôlé** quand il garde son logiciel (§13).

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-09)

Recherche menée par une session dédiée le 2026-11-09 ; les deux arrêtés qui changent la conception (R-09 et R-11) ont été **relus une seconde fois** sur Légifrance. Plusieurs sites officiels étaient inaccessibles depuis la session (education.gouv.fr donc le BO, eduscol) : ce qui en dépend reste **[NV]**. **[V]** = lu sur la page officielle (Légifrance, service-public.gouv.fr, fonction-publique.gouv.fr), parfois sous forme d'extraits ; **[NV]** = non lu sur une page officielle.

| N° | Règle | Référence | Statut | Conséquence pour l'EDT |
|---|---|---|---|---|
| R-01 | **Maxima de service hebdomadaires** du second degré public : agrégés 15 h, agrégés d'EPS 17 h, certifiés, PLP et adjoints d'enseignement 18 h, professeurs d'EPS 20 h ; professeurs des écoles en SEGPA, EREA, ULIS 21 h ; missions liées (préparation, suivi, orientation, relations avec les parents, travail en équipe) | décret n° 2014-940 du 20/08/2014, art. 2 — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000029390906/ | [V] | heures dues par intervenant (socle 6) ; contrôle « heures placées ≤ heures dues + HSA » (§13.3, §14.1) |
| R-02 | **Pondérations** : cycle terminal général et technologique 1,1 (gain d'**une heure au plus**) ; **BTS 1,25** ; REP+ 1,1 | même décret, art. 6, 7, 8 | [V] | les heures **pondérées** se calculent au socle 6 à partir des séances placées (niveau du public) ; l'EDT expose heures élèves et heures pondérées |
| R-03 | **Réductions** : 1 h en collège sans personnel de laboratoire (≥ 8 h de SVT ou physique-chimie) ; 1 h pour un complément de service dans une autre commune ou dans deux autres établissements ; allègements pour mission particulière (recteur), formateurs académiques 3 à 6 h | même décret, art. 9, 4-I, 3, 3-1 | [V] | service partagé = indisponibilités sans lieu (CL-27) ; réductions = données du socle 6 |
| R-04 | **Heures supplémentaires** : l'administration peut imposer **deux HSA**, sauf raison de santé | même décret, art. 4-III (décret 2019-309) | [V] | alerte au-delà de deux HSA non acceptées (socle 6) ; taux HSA/HSE [NV] |
| R-05 | **Remplacement de courte durée** (absences ≤ 2 semaines) : **plan annuel** du chef d'établissement présenté au CA après avis du conseil pédagogique, compte rendu au moins deux fois par an ; remplacement d'abord en heures de cours, séquences numériques possibles ; volontaires sur des **créneaux fixes d'au moins une heure par semaine**, délai de prévenance fixé par le plan, refus seulement pour motif légitime ; abroge le décret 2005-1035 | décret n° 2023-732 du 08/08/2023, art. 2, 5, 6, 8 — https://legifrance.gouv.fr/loda/id/JORFTEXT000047948581/2026-06-08 | [V] | **créneaux de disponibilité RCD** des volontaires = ressource de l'EDT, lue par le socle 15 pour proposer les remplaçants ; le plan annuel et ses comptes rendus relèvent du socle 15 ; application au **privé sous contrat** [NV] (Q-EDT10) |
| R-06 | **Pacte enseignant** en 2026-2027 | rapport du Sénat sur le PLF 2026 (480 M€ en 2026, RCD prioritaire) | [NV] | rien à coder avant confirmation |
| R-07 | **Privé sous contrat** : les maîtres relèvent des règles du public pour leurs conditions de service et sont astreints aux ORS des personnels de même catégorie ; contrat possible dès un demi-service ; maîtres délégués à temps complet ou incomplet, 1 h de réduction s'ils exercent dans plusieurs établissements | C. éduc. R914-2, R914-3, R914-44, R914-57, D914-58-7 — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000020054916/ | [V] | même modèle d'heures dues pour public et privé (D-019) |
| R-08 | **Temps partiel** des enseignants : quotité en **nombre entier d'heures** hebdomadaires | décret 82-624 ; circulaire 2015-105 | [NV] | contrôle d'heures entières au socle 6 |
| R-09 | **Suivi du remplacement de courte durée** : traitement ministériel alimenté par les **logiciels de vie scolaire**, données « transmises **hebdomadairement** » par **API** : UAI, type d'établissement, disciplines, niveau de classe, **heures prévues à l'emploi du temps**, heures d'absence déclarées, heures effectivement remplacées, mode de continuité pédagogique, **motif d'absence par catégories** (raison de santé, événement familial, convenance personnelle…, « en attente de motif ») ; conservation 5 ans ; pas de droit d'opposition | arrêté du 14/08/2023, art. 2 à 6 et annexe — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000047989129 | [V] (relu) | **obligation pour ScolIA dans le public** dès qu'il sert de logiciel de vie scolaire (D-029) : les séances prévues, annulées et remplacées doivent être agrégeables par semaine, discipline, niveau ; le **motif d'absence doit être catégorisé** au socle 15 (liste nationale), jamais diffusé ailleurs ; flux à construire au socle 9 (lot à créer, Q-EDT11) ; privé sous contrat : texte muet [NV] |
| R-10 | **STS-web** : l'export `sts_emp_<UAI>_<année>.xml` alimente le logiciel d'EDT, qui renvoie structure et services vers STS-web jusqu'à la campagne de rentrée ; **en 2026, synchronisation par OMOGEN-API / NetSynchro** à la place des fichiers manuels ; inscription Pléiade et parfois validation ministérielle pour s'interfacer | documentations d'éditeurs et de DANE | [NV] | situation A du §13.1 conditionnée à la spécification (D-029, socle 9 X16 bis) ; vérifier OMOGEN avant tout développement |
| R-11 | **Traitement « Emploi du temps »** : permet aux élèves et à leurs responsables de consulter l'EDT ; données des enseignants : **civilité, nom, prénom, matière** ; destinataires : chef d'établissement, enseignants et vie scolaire, et **les élèves et leurs responsables pour les données relatives aux enseignants** ; aucune absence dans ce traitement ; conservation un an | arrêté du 21/03/2018, art. 1 à 5 — https://www.legifrance.gouv.fr/loda/article_lc/LEGIARTI000036767958 et suivants | [V] (relu) | afficher le **nom** de l'intervenant aux familles est prévu (Q-EDT8 : oui par défaut) ; **jamais l'absence ni son motif** ; texte du ministère, à considérer comme référence pour le registre de ScolIA (le DPO tranche) |
| R-12 | **Collège** : 6e = 25 h de tronc commun, cycle 4 = 26 h ; dotation complémentaire de 3 h par division ; répartition modulable entre disciplines ; **heure de vie de classe ≥ 10 h par an** ; accompagnement aux devoirs en 6e | arrêté du 19/05/2015 (version au 09/11/2026), art. 3, 6, 9 — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000030613339 | [V] | volumes des cours (socle 5, maquette) ; vie de classe par séances ponctuelles (CL-30) |
| R-13 | **Groupes de besoin** : annulés par le Conseil d'État (28/11/2024, n° 493513), réinstaurés (arrêté du 04/04/2025, art. 4-1), puis **art. 4-1 abrogé à compter du 05/07/2026** (arrêté du 10/03/2026) : les groupes à effectifs réduits en français et mathématiques de 6e/5e deviennent **facultatifs** | même arrêté | [V] | le modèle ne présuppose rien : groupes paramétrés (D-022) ; changements de groupe datés sans republication (CL-20) |
| R-14 | **Lycée général** (cycle terminal) : spécialités 3 × 4 h en 1re, 2 × 6 h en terminale ; enseignement scientifique 2 h (3 h 30 avec mathématiques spécifiques) ; modifié le 19/06/2026 | arrêté du 16/07/2018 — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000037202800 | [V] | alignements de spécialités (CL-04) ; enveloppes de seconde (12 h par division) et de cycle terminal (8 h) [NV] |
| R-15 | **Lycée professionnel** : grilles modifiées en 2024 puis 2025-2026, fin de terminale et calendrier des épreuves changés | sources contradictoires | [NV] | ne rien figer ; co-intervention et périodes en entreprise paramétrées (CL-06, CL-32) |
| R-16 | **Organisation de la journée au second degré** : **aucune règle nationale** trouvée sur l'amplitude de la journée, la pause méridienne, la durée d'une séance (55 min), le mercredi ou le samedi ; les plafonds journaliers et la pause méridienne d'au moins 1 h 30 ne concernent que le **premier degré** | C. éduc. D521-1 à D521-18 (D521-10 pour le premier degré) — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000020663218/ | [V] (absence de règle) | pause, amplitude, trous : **contraintes d'établissement** (souples par défaut), pas des règles codées ; premier degré (socle 50) : à rendre dures |
| R-17 | **Année scolaire** : au moins 36 semaines, cinq périodes séparées par quatre vacances, calendrier national pour trois ans ; un recteur peut décaler une date de vacances de 3 jours au plus ; outre-mer et Corse : adaptation par arrêté rectoral | C. éduc. L521-1, D521-2, D521-5 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006525125 | [V] | calendrier adopté (socle 5) ; outre-mer (CL-24) ; 2026-2027 : arrêté du 22/10/2025, pas de classe le 07/05/2027 [V] https://www.service-public.gouv.fr/particuliers/actualites/A18563 |
| R-18 | **Inaptitude en EPS** : certificat d'inaptitude totale ou partielle, valable au plus l'année, jamais rétroactif | C. éduc. R312-2 | [NV] | dispense au socle 12 ; l'EDT ne change pas (CL-33) |
| R-19 | **Grève** : service minimum d'accueil seulement pour les écoles (loi 2008-790, L133-1 et suivants) ; second degré : pas d'accueil obligatoire, aucun texte trouvé imposant l'information des familles | C. éduc. L133-1 à L133-12 | [NV] (le socle 8 a vérifié L133-4) | annulation du matin (CL-18) ; information des familles comme bonne pratique, pas comme obligation |
| R-20 | **FET** (logiciel libre de génération d'EDT) : licence **GNU AGPL v3** | https://lalescu.ro/liviu/fet/ | [V] | inspiration et lecture de son **format** seulement, **aucun code repris** (même règle que convt, D-005) ; corrige la mention « licence contradictoire » du socle 9 (FX-82) |
| R-21 | **Formats** des logiciels d'EDT (Index EDT, UnDeuxTEMPS, aSc, Charlemagne) | documentations éditeurs inaccessibles | [NV] | jeux d'or d'un pilote (D-14 du socle 9) avant le lot T10 |

**À vérifier en priorité** (depuis un poste qui accède au BO et à eduscol) : reconduction du Pacte ; grilles et calendrier de la voie professionnelle ; application du RCD (R-05, R-09) au privé sous contrat ; spécification OMOGEN-API / NetSynchro et STS-web ; taux HSA/HSE.

---

## 6. Frontières

### 6.1 Tableau

| Sujet | **Socle 10** | Autres |
|---|---|---|
| Cours, publics, alignements, groupes, inclusions | lit (`ent_public_cours_jour`, `ent_cours`, alignements) | **5** possède |
| Interventions (titulaire, co-enseignant, suppléant), heures dues | lit (`intervenants_du_cours`, `en_saisie`) | **6** possède ; **15** ouvre les suppléances |
| Calendrier, jours, semaines types, grilles, fermetures, lieux et leurs indisponibilités | lit (`jour()`, `grille()`, semaines) ; réagit aux événements | **5** possède (S2, S3, S7) |
| Besoins horaires, pièces, versions d'EDT, scénarios, placements, verrous, contraintes d'établissement, vœux | **possède** | 6 transmet les indisponibilités des personnels (PRS-55) |
| Séances datées et leurs états | **possède le modèle et les commandes** ; tables rangées au noyau (Q-B15, §21) | 12, 20, 21, 15, 14 s'y rattachent et les lisent |
| Modifications de séances (annulation, déplacement, lieu, horaire, ajout, fusion, neutralisation) | **possède** | 15 demande un changement d'intervenant ; 42, 38, 5 provoquent des neutralisations |
| Absence d'un personnel, décision de remplacer, recherche du remplaçant, heures effectuées | fournit les séances touchées et les plages libres (ports) | **15** possède |
| Appel, absences à la séance, demi-journées | fournit la séance et son état | **12** possède |
| Cahier de textes, travail à faire | fournit la séance ; propose le report | **20** possède |
| Régimes et autorisations de sortie, sortie anticipée, remise | fournit l'EDT du jour et ses changements | **14** possède |
| Réservations de salles et de matériel | lit (conflits) | **44** possède |
| Examens, surveillances | lit (salles et publics occupés) ; neutralise | **38** possède |
| Sorties et voyages | neutralise | **42** possède |
| Notifications | décrit les faits et les destinataires | **8** envoie |
| Échanges (moteur, connecteurs, abonnements iCal) | fournit le contenu, applique les propositions | **9** possède |
| Réunions, rendez-vous, conseils | fournit le créneau commun et l'occupation | **19**, **23** possèdent |
| Paie | — | **49** lit les heures effectuées (15) |

### 6.2 Ports exposés par le socle 10 [R]
| Port | Appelé par | Sens |
|---|---|---|
| `edt::seances(cible, plage) -> Vec<Seance>` (cible : élève, cours, classe, groupe, intervenant, lieu) | 12, 14, 20, 21, 15, 19, 2 (fiche) | lecture, avec la portée de l'appelant |
| `edt::seance(id) -> Seance` | 12, 20, 21 | lecture |
| `edt::seance_en_cours(eleve, instant) -> Option<Seance>` | 14, 12 | lecture (« maintenant ») |
| `edt::plages_libres(ressources, plage, duree) -> Vec<Plage>` | 15 (remplaçants, rattrapage), 19 (créneau commun), 44 | lecture |
| `edt::occupe(ressource, plage) -> bool` | 44, 38, 42 | lecture |
| `edt::seances_touchees(intervenant, plage)` | 15 | lecture |
| `edt::modifier_seance(cmd)` (dont `intervenant`) | 15, 42, 38, 5 (réactions) | commande, dans la transaction de l'appelant |
| `edt::heures_assurees(intervenant, plage)` | 15, 49 | lecture (séances réellement assurées) |
| `edt::agregats_remplacement(semaine)` (heures prévues à l'EDT, absentes, remplacées, par discipline et niveau, sans nom) | 9 (remontée hebdomadaire RCD, R-09) | lecture agrégée |
| trait `ControleRentree` / `ControleCloture` (« EDT publié pour l'année », « séances non assurées en attente ») | 5 (tableau de bord) | lecture |

### 6.3 Ports consommés [R]
Socle 5 : `annee_a_la_date`, `jour(date, cible)`, `jours_ouvres`, `grille(date, site, niveau)`, `public_du_cours` / `ent_public_cours_jour`, `perimetre`, lieux et leurs indisponibilités (337), alignements. Socle 6 : `intervenants_du_cours`, `en_saisie(service, date)`, indisponibilités des personnels. Socle 4 : `droits_lien` (lecture famille, abonnement), interdictions (271). Socle 1 : `autoriser`, `tx`, `publier`, tâches de fond, journal.

### 6.4 Points à accorder avec le socle 5 [R]
- **Q-S11** (génération des séances) : confirmée ici, le code part au socle 10.
- **`ent_cours.semaines`** (`'toutes','A','B'`, 341) : la semaine type appartient au **placement**, pas au cours (un cours de 3 h peut avoir 2 h en A et 1 h en B) ; la colonne du cours devient au mieux une valeur par défaut pour la génération (Q-EDT3).
- **Port de disponibilité des lieux** (337, `ent_disponibilite`, `ent_salle_indisponibilite`) : le socle 10 l'utilise au lieu de `ent_edt_contrainte` pour les salles.
- **Grille** : le socle 10 lit la grille par (date, site, niveau, jour) ; un placement référence un **code de créneau** de la grille (« M2 »), pas des heures, pour qu'un changement de grille (version de février) déplace les heures sans toucher au plan (avec contrôle des pièces qui ne tiennent plus).

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]

| Terme | Définition | Exemple | Propriétaire |
|---|---|---|---|
| **Cours** | Ce qui est enseigné à quel public (classe ou groupe, plus inclusions et exclusions), avec un volume horaire | « Espagnol LV2 — 4AB-Esp — 2 h 30 » | socle 5 (`ent_cours`, lot S4) |
| **Intervention** (service) | Qui intervient dans un cours, avec quelle nature (titulaire, co-enseignant, suppléant, assistant) et quelles heures | « M. X titulaire 2 h 30 » | socle 6 |
| **Besoin horaire** | Ce qu'il faut placer pour un cours dans une semaine type : un volume, découpé en **pièces** | « 2 h 30 = 1 h 30 + 1 h » | **10** (dérivé du cours, ajustable) |
| **Pièce** | Une unité de placement d'un cours : une durée, une contrainte de forme (consécutive, alternée A/B, demi-groupe) | « bloc de 2 h de TP » | **10** |
| **Grille** | Les créneaux horaires d'une journée type, versionnés et ciblés | « M1 8 h 00–8 h 55 » | socle 5 (`ent_grille`, lot S7) |
| **Plage** | Un ou plusieurs créneaux consécutifs de la grille, un jour donné (ce qu'occupe une pièce) | « mardi M2–M3 » | **10** |
| **Semaine type** | L'étiquette d'une semaine (A, B, ou « toutes ») ; la règle d'alternance et les semaines matérialisées | « semaine du 6 octobre = A » | socle 5 (`ent_semaine`, lot S3) |
| **Emploi du temps (version)** | Un ensemble de **placements** (pièce → plage, semaine type, lieu) valable à partir d'une date, dans un état : brouillon, publiée, remplacée, archivée | « EDT v2 à partir du 3 novembre » | **10** |
| **Scénario** | Un brouillon d'emploi du temps produit ou modifié pour comparaison ; un seul devient une version publiée | « scénario B : mercredi libéré » | **10** |
| **Placement** (créneau) | La présence d'une pièce d'un cours dans une plage, une semaine type, avec ses lieux ; c'est le **créneau** d'aujourd'hui (`ent_edt_creneau`) | « 4B Maths, lundi M1, toutes semaines, salle 12 » | **10** |
| **Alignement** (barrette) | Des cours qui doivent avoir lieu **en même temps** (élèves d'une classe répartis entre plusieurs groupes) | « LV2 4e : All 4AB, Esp 4A, Esp 4B » | socle 5 (`ent_cours_alignement`) ; **10** l'applique |
| **Séance datée** | L'occurrence réelle d'un cours à une date et une heure : public, intervenants, lieu, état | « 4B Maths, lundi 6 octobre 8 h 00–8 h 55 » | **10** (table au noyau, Q-B15) |
| **Modification** | Un écart daté, motivé et tracé entre le plan et le réel | « annulation, motif interne : absence professeur » | **10** (remplacement : socle 15) |
| **Ressource** | Ce qu'une séance occupe : un intervenant, un public, un lieu, un équipement | « salle de TP chimie 2 », « vidéoprojecteur mobile » | 5 (lieux), 6 (intervenants), 44 (matériel) |
| **Indisponibilité** | Une plage où une ressource ne peut pas être occupée (dure) | « M. X, service partagé, jeudi » | **10** (déclarée par 6 PRS-55 pour les personnels, 5 pour les salles) |
| **Vœu** | Une préférence exprimée par un professeur pour son emploi du temps (souple) | « pas de première heure le lundi » | **10** |
| **Contrainte dure / souple** | Dure : jamais violée par un placement publié. Souple : respectée autant que possible, chaque écart coûte des points | dure : « pas deux cours au même moment pour un élève » ; souple : « au plus une heure de trou par jour » | **10** |
| **Créneau de disponibilité (RCD)** | Plage fixe hebdomadaire (au moins une heure) où un professeur volontaire peut être appelé pour un remplacement de courte durée (R-05) | « Mme Z, jeudi M4 » | **10** (ressource), lue par **15** |
| **Occupation** | L'utilisation d'un lieu à une date hors cours (examen, réunion, réservation) | « CDI réservé pour le conseil » | 44 (réservation), 10 (lecture pour les conflits) |

### 7.2 Quatre distinctions qui manquent aujourd'hui
1. **Plan ≠ réel.** Aujourd'hui une modification du créneau (`edt_creneau_modifier`) change le passé et l'avenir d'un coup, et la régénération des séances peut contredire ce qui a déjà été fait. Dans la cible : le plan est versionné, les séances passées ne bougent jamais, une modification ponctuelle est un objet à part.
2. **Brouillon ≠ publié.** Aujourd'hui, créer un créneau le rend visible immédiatement des familles. Dans la cible : on construit en brouillon, on publie à une date, avec aperçu.
3. **Placement ≠ intervenant.** Aujourd'hui le créneau porte le professeur (`prof_user_id NOT NULL`). Dans la cible : le placement porte le cours ; l'intervenant à la date vient du socle 6 (y compris suppléant et co-enseignant) ; un cours vacant se place quand même (le poste sera pourvu).
4. **Lieu typé ≠ texte.** Aujourd'hui `salle TEXT` ; le socle 5 a livré des salles typées liées à un site (337). Dans la cible : `lieu_id` (une ou plusieurs salles), texte gardé en projection d'affichage le temps de la transition.

---

## 8. Le modèle du temps : du plan aux séances

### 8.1 Calcul [R]
Pour un établissement, une date `d` et un cours `c`, les séances prévues sont :

```
version(d)       = la version publiée de l'EDT dont la plage [valable_du, valable_au] contient d
semaine(d)       = étiquette de la semaine de d (socle 5, ent_semaine), NULL hors période de cours
jour(d, cible)   = port du socle 5 : ouvert / sans cours / sans élèves / horaires modifiés / jour de référence (rattrapage)
grille(d, site, niveau) = port du socle 5 : version de grille en vigueur, créneaux et heures murales
placements(c)    = placements de version(d) pour c, sur le jour de la semaine j (ou jour_reference si rattrapage),
                   dont la semaine type vaut « toutes » ou semaine(d)
séance(c, d, p)  = pour chaque placement p : heures = grille(d…)[p.plage] ; intervenants = socle 6 à d ;
                   lieux = p.lieux ; état = prévue, sauf si jour(d, public de c) l'empêche
modifications    = appliquées ensuite (§9), dans l'ordre de leur création
```

Points de conception :
- **Matérialisation par fenêtre glissante.** Les séances sont matérialisées de la date de publication jusqu'à la fin de la version (au plus l'année), et **recalculées** quand une de leurs causes change (version publiée, calendrier, semaine, grille, public du cours, intervention). Chaque cause publie un événement que le socle 10 consomme (`referentiel.calendrier_modifie`, `referentiel.fermeture_*`, `structure.grille_versionnee`, `cours.public_modifie`, `service.intervenant_*`). Le recalcul est **idempotent** et **ne touche que l'avenir** (à partir de demain, ou d'aujourd'hui pour les séances non commencées), sauf correction explicite d'une erreur passée (§9.6).
- **Identité stable.** Une séance est identifiée par (version ou modification d'origine, placement, date). Quand une nouvelle version reprend le même placement (même cours, même plage, même semaine), la séance **garde son identifiant** : le travail donné pour la séance du 12 novembre ne disparaît pas parce qu'on a republié l'EDT le 3 novembre. Quand le placement change, l'ancienne séance est **retirée** (état `retiree`, motif « nouvelle version », lien vers la séance qui la remplace quand elle existe) et une nouvelle séance est créée ; ce qui était attaché à l'ancienne est **proposé** au rattachement (cahier, travail à faire), jamais déplacé en silence.
- **Heures murales.** La séance stocke `jour DATE`, `heure_debut TIME`, `heure_fin TIME` (heure murale de l'établissement) et un instant calculé `debut_utc TIMESTAMPTZ` pour les notifications et l'iCal (fuseau de l'établissement, socle 5). Les changements d'heure n'affectent pas les heures murales ; un créneau qui tomberait dans l'heure sautée (nuit du changement d'heure) est impossible en pratique et refusé [R].
- **Rattrapage.** Un jour de rattrapage (socle 5, `jour_reference`) suit les placements de son jour de référence (« ce samedi suit l'emploi du temps du lundi »).
- **Horaires modifiés.** Un jour à `effet = cours_modifies` (conseil de classe, journée d'intégration) peut porter une **grille du jour** (version de grille ciblée sur la date) ou une règle simple « cours jusqu'à 15 h » : les séances au-delà sont annulées avec motif public.
- **Hors période.** Avant la rentrée d'un niveau (rentrée décalée des 6es) ou après la fin des cours d'un niveau (classes à examen), le calendrier ciblé du socle 5 neutralise les séances : elles ne sont pas créées (et non créées puis annulées), pour ne pas inonder les familles de notifications.

### 8.2 États d'une séance [R]

| État | Sens | Appel attendu | Visible famille | Notes |
|---|---|---|---|---|
| `prevue` | Doit avoir lieu, avec ses intervenants du jour | oui | oui | état par défaut |
| `modifiee` | A lieu, mais quelque chose a changé (lieu, heure, intervenant, public) | oui | oui, avec le changement | la modification est liée |
| `annulee` | N'a pas lieu ; motif public, motif interne séparé | non | oui, « annulé » | pas de travail à rendre ce jour-là sans action du professeur |
| `deplacee` | N'a pas lieu à cette date-là : une séance de remplacement existe ailleurs | non (à l'origine) | oui, « déplacé au … » | lien vers la séance d'arrivée |
| `neutralisee` | Absorbée par un événement qui remplace les cours (examen, sortie de tout le public, journée banalisée) | non | oui, « remplacé par … » | lien vers l'événement |
| `non_assuree` | Aucun intervenant en saisie (poste vacant, absence non remplacée) et non annulée explicitement | non (pas d'appel, mais **les élèves doivent être pris en charge** : permanence) | oui, « cours non assuré » | alerte ADJ et VS ; jamais de motif |
| `retiree` | N'existe plus (nouvelle version, correction) ; garde son identifiant pour l'historique | non | non | jamais supprimée physiquement si quelque chose s'y attache |
| `ajoutee` | Séance hors plan (rattrapage, séance supplémentaire, soutien ponctuel) | oui | oui | modification de type `ajout` |

Ces états sont **dérivés** des modifications et du calendrier, stockés pour la lecture rapide, recalculables (test de cohérence : recalculer tout ne change rien).

### 8.3 Lien avec les autres socles
- **Appel (socle 12)** : une séance attend un appel si son état est `prevue`, `modifiee` ou `ajoutee` et qu'un intervenant est en saisie ; le public de l'appel est `ent_public_cours_jour(cours, jour)` à la date de la séance ; un élève en stage, en sortie d'un autre cours, en examen, dispensé, est **pré-rempli** par le socle 12 à partir des faits (pas par l'EDT).
- **Cahier de textes (20), évaluations (21)** : rattachés à la séance (contenu, travail donné « pour la séance du… ») ; si la séance est annulée ou déplacée, le socle 20 propose de reporter le travail à la séance suivante du cours.
- **Remplacements (15)** : le socle 15 possède l'absence et la décision de remplacer ; il crée l'intervention de suppléance (socle 6) ou le **remplacement d'une séance** (modification de type `intervenant` du socle 10, par le port). Les **heures effectuées** (HSE, remplacements de courte durée) sont lues par la paie (49) à partir des séances réellement assurées.
- **Vie scolaire, permanence (12, 14)** : « élèves sans cours à l'instant t » = public des séances annulées ou non assurées, plus les élèves dont l'EDT est vide à t, moins les autorisations de sortie (régime, socle 14).
- **Restauration, internat, transports (40, 41)** : une modification qui touche le repas (sortie de la journée, fermeture) publie l'effet ; l'EDT ne décide rien sur la cantine.
- **Sorties et voyages (42)** : une sortie déclarée neutralise les séances de ses participants et, au choix, celles de ses accompagnateurs (proposées à l'annulation ou au remplacement).
- **Examens (38)** : une épreuve occupe des salles et des surveillants ; l'EDT neutralise les séances touchées (EXA-05 de `SOCLE2_DOMAINES.md`).
- **Réservations (44)** : une salle réservée n'est jamais proposée pour un déplacement ; une réservation sur une salle occupée par un cours est refusée (ou demande un changement de salle du cours).

---

## 9. Modifier le réel

### 9.1 Types de modification [R]

| Type | Exemple | Qui | Effets automatiques | Notifié à |
|---|---|---|---|---|
| `annulation` | absence non remplacée, sortie, grève | ADJ, CPE (selon droit), SYS (fermeture, grève) | appel non attendu ; travail prévu signalé (20) ; permanence calculée ; sortie anticipée (14) | élèves du public (et titulaires si mineurs), intervenant, VS |
| `deplacement` | cours avancé, rattrapage | ADJ ; PRF (demande) | ancienne séance `deplacee`, nouvelle `ajoutee` ; contrôles : public libre, intervenant libre, lieu libre | idem |
| `changement_lieu` | salle en travaux, salle de TP | ADJ, VS (selon droit) ; SYS (salle fermée, socle 5) | — | public, intervenant ; agents si nécessaire |
| `changement_horaire` | cours raccourci (conseil), décalé de 15 min | ADJ ; SYS (jour à horaires modifiés) | sortie anticipée recalculée | public |
| `intervenant` | remplacement ponctuel par un collègue (socle 15), changement d'intervenant d'une seule séance | ADJ (via 15) | appel par le remplaçant ; heures comptées au remplaçant | public (nom du remplaçant, jamais le motif) |
| `ajout` | séance de rattrapage, soutien, devoir surveillé ajouté | ADJ ; PRF (demande) | appel attendu | public |
| `fusion` | deux demi-groupes regroupés exceptionnellement ; deux classes réunies pour une conférence | ADJ | un appel pour le public réuni (chaque élève avec son cours d'origine) | publics concernés |
| `neutralisation` | examen, sortie de niveau, journée banalisée ciblée | SYS (événement de 5, 38, 42) | état `neutralisee` | publics, intervenants |
| `distance` [R, Q-EDT9] | cours maintenu à distance (intempéries, fermeture avec continuité pédagogique) | ADJ | appel en ligne ou pas d'appel (réglage) ; lien visio (socle 33) | public |
| `correction` | erreur dans le passé (mauvaise salle saisie, cours qui a réellement eu lieu) | ADJ ★ | journalisée ; effets en aval recalculés et **signalés** (appel déjà fait) | personne (sauf choix) |

Chaque modification porte : la séance (ou la plage de séances, pour un lot), le type, l'état avant et après, un **motif public** (choisi dans une liste courte : « absence du professeur », « sortie », « examen », « réunion », « travaux », « intempéries », « mouvement social », « autre » + texte libre court relu) et un **motif interne** facultatif (jamais visible hors ADJ/DIR), l'auteur, l'origine (`saisie`, `absence` du socle 15, `fermeture` du socle 5, `sortie` du 42, `examen` du 38, `lot`), la clé d'idempotence, l'annulation éventuelle (on peut **lever** une modification : la séance revient à son état calculé).

### 9.2 Lots de modifications
Une modification peut viser **plusieurs séances** d'un coup : toutes les séances d'un professeur sur une plage (absence), d'une classe (sortie), d'un niveau, de tout l'établissement (grève, intempéries), d'une salle (travaux). Le lot a un **aperçu serveur** (nombre de séances, classes, élèves, intervenants touchés, notifications qui partiront) avant confirmation, une clé d'idempotence (deux clics = un lot), et se **lève** en bloc ou séance par séance. Le lot existant `ent_edt_annulation_masse` (171) est repris dans ce modèle.

### 9.3 Conflits et contrôles à chaque modification [R]
Avant d'appliquer une modification qui place quelque chose (déplacement, ajout, changement de lieu, d'horaire, d'intervenant), le serveur vérifie, **à la date** : public libre (aucun élève du public dans une autre séance active, au sens du public datée, inclusions comprises), intervenants libres (y compris dans d'autres cours, réunions et conseils déclarés, indisponibilités), lieu libre (séances, réservations, examens, fermeture de salle), capacité du lieu ≥ effectif, type du lieu compatible avec le besoin du cours (TP, EPS), temps de déplacement entre sites. Un conflit **dur** bloque, sauf **forçage** explicite par l'ADJ avec motif (cas réels : deux demi-groupes fusionnés dans la même salle) ; un avertissement (capacité dépassée de 2 places) s'affiche sans bloquer.

### 9.4 Demandes des professeurs
Un professeur **demande** (il ne modifie pas) : déplacer une séance, échanger une séance avec un collègue (les deux doivent accepter), changer de salle, ajouter une séance de rattrapage, déclarer une sortie (socle 42). L'ADJ reçoit la demande avec les contrôles déjà faits et accepte en un geste ; un réglage permet d'**accepter automatiquement** les changements de salle sans conflit et les échanges acceptés par les deux professeurs sur le même public [R, S-80 : réglage à discuter].

### 9.5 Quand prévenir [R]
Le délai fait l'urgence (catégories du socle 8, `08_NOTIFICATIONS.md` §5) : une annulation de demain 8 h décidée à 17 h est **urgente** (U2 si avant 10 h le lendemain) ; une annulation dans trois semaines est **ordinaire** (regroupée dans le résumé). Heures calmes respectées (pas de notification la nuit) sauf pour le lendemain matin, envoyée à une heure fixe du soir (réglage d'établissement, défaut 18 h 30) et rappelée le matin [R]. **Un seul message par changement** et par destinataire, même si le changement touche plusieurs séances (lot) ; la levée d'une modification déjà annoncée envoie un message de rétablissement.

### 9.6 Le passé
Les séances passées (heure de fin dépassée) ne changent plus par une nouvelle version, une régénération ou un import. Seule une **correction** (droit spécial, motif obligatoire, journalisée) peut les modifier, et elle **signale** ses effets en aval (appel déjà fait sur une séance qui n'a pas eu lieu, heures de remplacement à corriger) sans les défaire en silence.

---

## 10. Cas limites (catalogue)

Chaque cas est une exigence de test pour les lots (§24). [R] sauf mention.

| N° | Situation | Ce que doit faire ScolIA |
|---|---|---|
| CL-01 | **Semaines A/B** avec reprise après les vacances (la semaine après la Toussaint redevient A) | règle d'alternance du socle 5 (`reprise_apres_vacances`) ; semaine forcée possible ; l'élève voit « semaine A » résolue, jamais le code |
| CL-02 | Alternance à **4 semaines** (A, B, C, D) ou quinzaine décalée par groupe (demi-groupe 1 en A, demi-groupe 2 en B) | étiquettes multiples (2 à 4, migration 333) ; un placement par semaine type |
| CL-03 | Cours **une semaine sur deux** de 2 h au lieu d'une heure par semaine | deux pièces possibles ; le besoin horaire est respecté en moyenne |
| CL-04 | **Groupes alignés** (LV2 : allemand, espagnol, italien en même temps pour 4A et 4B) | alignement du socle 5 ; contrainte dure en génération ; un déplacement d'un cours d'un alignement propose de déplacer tout l'alignement |
| CL-05 | **Co-enseignement** (deux professeurs dans la même salle, même public) | une séance, deux intervenants en saisie (socle 6) ; conflits vérifiés pour les deux ; appel fait par l'un ou l'autre, une seule fois |
| CL-06 | **Co-intervention** en lycée professionnel (professeur d'enseignement général + professeur de spécialité) [NV] | idem CL-05 ; heures comptées à chacun |
| CL-07 | **Demi-groupes** en sciences : la classe entière en cours, puis deux demi-groupes en TP sur deux plages | trois cours (classe, demi-groupe A, demi-groupe B) ; les demi-groupes peuvent être en alternance A/B |
| CL-08 | Élève d'**ULIS** inclus en maths en 5B et en anglais en 4C | inclusion individuelle (331) ; son EDT combine ULIS et inclusions ; conflits contrôlés |
| CL-09 | Élève **UPE2A** à emploi du temps individualisé, qui change toutes les six semaines | inclusions datées ; vue EDT individuelle ; versions individuelles sans republier l'établissement |
| CL-10 | **Suppléance** longue (congé maternité) | intervention de suppléance (socle 6) : les séances changent d'intervenant à partir de la date, le plan ne bouge pas |
| CL-11 | **Remplacement d'une seule séance** par un collègue libre | modification `intervenant` (via socle 15) ; le remplaçant fait l'appel ; heure comptée |
| CL-12 | Séance remplacée par une **autre matière** (le collègue de français fait français à la place des maths) | modification qui **change le cours** de la séance (la séance d'origine `annulee`, une séance `ajoutee` du cours de français) ; heures comptées au bon cours |
| CL-13 | **Salle indisponible** (dégât des eaux) pour trois semaines | fermeture datée de la salle (socle 5) → toutes les séances proposées au changement de lieu, en lot, avec salle libre proposée |
| CL-14 | **Sortie scolaire** d'une demi-journée pour une classe, avec deux accompagnateurs qui ont cours ailleurs | séances de la classe neutralisées ; séances des accompagnateurs proposées à l'annulation ou au remplacement ; liste des élèves non partants (permanence) |
| CL-15 | **Examen** blanc d'un niveau dans le gymnase et trois salles, toute une matinée | neutralisation ciblée niveau ; salles occupées ; surveillances (38) ; autres niveaux : cours qui tombaient dans ces salles déplacés |
| CL-16 | **Journée banalisée** (formation des personnels, élèves sans cours) | calendrier `sans_eleves` (socle 5) : aucune séance, aucune notification de séance (un seul message de calendrier) |
| CL-17 | **Fermeture** pour neige, levée à 10 h | fermeture conditionnelle du socle 5 (334) ; séances annulées puis rétablies à partir de la levée ; un message par changement |
| CL-18 | **Grève** : les professeurs grévistes ne sont pas connus d'avance (aucun texte trouvé imposant une déclaration préalable dans le second degré [NV, §5 R-19]) | annulation **le matin**, séance par séance ou par professeur, aperçu ; information des familles ; pas de motif nominatif (« mouvement social » sans nom) |
| CL-19 | **Changement de classe** d'un élève le 4 novembre | son EDT change à cette date (public du cours) ; appel du nouveau cours ; ancien professeur garde l'historique avant la date (D-022, §8.2 du socle 5) |
| CL-20 | **Changement de groupe** de besoin toutes les six semaines | même mécanisme ; aucune republication |
| CL-21 | Nouvelle **version** de l'EDT au 1er trimestre (groupes de besoin recomposés, dédoublement) | publication datée ; séances passées intactes ; séances futures identiques conservées (identité stable) ; aperçu des différences par public et par professeur |
| CL-22 | **EDT provisoire** de rentrée (première semaine), puis définitif | deux versions consécutives ; la première porte la mention « provisoire » pour les familles |
| CL-23 | **Rentrée décalée** des 6es (seuls les 6es le jour J) | calendrier ciblé niveau ; pas de séance pour les autres niveaux |
| CL-24 | **Outre-mer** : établissement de La Réunion (UTC+4), année australe ou calendrier local, jours fériés locaux (20 décembre) | fuseau et calendrier du site (socle 5) ; iCal et notifications à l'heure locale ; aucune heure Europe/Paris en dur |
| CL-25 | **Changement d'heure** (fin octobre) | heures murales stables ; instants UTC recalculés ; iCal avec fuseau explicite (`TZID`) |
| CL-26 | **Multi-sites** : un professeur sur deux sites dans la même demi-journée | temps de déplacement entre sites (paramètre du couple de sites) ; contrainte en génération et à chaque modification |
| CL-27 | **Service partagé** entre deux établissements (un autre tenant ScolIA ou non) | indisponibilités déclarées sans lieu (socle 6 PRS-55) ; aucune lecture entre établissements (D-013) |
| CL-28 | Professeur à **temps partiel** ou en **temps partiel thérapeutique** | indisponibilités de jours entiers ; aucune information de santé stockée dans l'EDT (motif interne jamais demandé) |
| CL-29 | **Poste vacant** à la rentrée | cours placé sans intervenant ; séances `non_assuree` ; quand le professeur arrive, ses interventions remplissent les séances, sans republication |
| CL-30 | **Heure de vie de classe** placée certaines semaines seulement (une fois par mois) | pièce à semaine type ou séances `ajoutee` en lot |
| CL-31 | **Accompagnement personnalisé, « devoirs faits »** sur la base du volontariat (public variable) | cours à public de groupe d'inscrits (socle 5/43) ; public daté |
| CL-32 | **Stage en entreprise** (lycée professionnel, 3e) : la classe n'a pas cours pendant 4 semaines | période d'entreprise du socle 5 (ou 39 par élève) ; séances non créées pour les élèves en stage ; séances de la classe maintenues pour les élèves restants (stage individuel) |
| CL-33 | **Élève dispensé d'EPS** toute l'année | dispense du socle 12 ; l'EDT ne change pas, l'appel pré-remplit « dispensé » ; ou exclusion individuelle du cours si l'élève ne doit pas venir (choix de l'établissement) |
| CL-34 | **PAP / PPS** avec aménagement de l'emploi du temps (fin à 15 h, pas de première heure) | exclusion datée des séances concernées, ou version individuelle ; motif jamais affiché (donnée sensible, socle 34) |
| CL-35 | **Section sportive / sport-études** : élèves libérés deux après-midi | groupe d'élèves ; exclusions datées ; contrainte en génération (ces après-midi libres pour le groupe) |
| CL-36 | Classe à **examen** dont les cours s'arrêtent mi-juin | calendrier ciblé ; plus aucune séance, plus aucune notification |
| CL-37 | **Conseil de classe** : cours terminés à 16 h pour un niveau pendant une semaine | jour à horaires modifiés ciblé ; séances au-delà annulées en un seul lot, un message |
| CL-38 | **Cours le samedi matin** une semaine sur deux | grille ciblée par jour (socle 5) et semaine type |
| CL-39 | **Mercredi après-midi** libéré pour tous sauf l'association sportive | contrainte « pas de cours » ; l'AS est une activité (socle 43), pas un cours de l'EDT, mais elle occupe des professeurs (indisponibilité) |
| CL-40 | **Pause méridienne** minimale pour chaque élève et chaque professeur | contrainte de génération et avertissement à chaque modification (§5 : recommandation, pas règle) |
| CL-41 | **Réunion** de l'équipe pédagogique à placer sur un créneau commun | recherche de créneau commun (port pour le socle 19) ; occupation des intervenants |
| CL-42 | **Devoir surveillé** du samedi matin pour un niveau | séances `ajoutee` hors plan avec public de niveau, surveillants ; salles réservées |
| CL-43 | Deux élèves inscrits par erreur dans deux groupes alignés | conflit de public détecté (socle 5 le signale ; l'EDT l'affiche : « élève dans deux séances au même moment ») |
| CL-44 | **Import** du logiciel d'EDT qui renomme un groupe | correspondance mémorisée (socle 9 ITP-18) ; écart signalé, jamais un nouveau groupe créé en silence |
| CL-45 | Réimport en cours d'année alors que des modifications ponctuelles existent | les modifications sont **préservées** et listées si elles deviennent incohérentes |
| CL-46 | **Salle de TP** et équipement requis (paillasses, ordinateurs) | type de salle et équipements (337) ; besoin du cours ; génération et modifications respectent le besoin |
| CL-47 | **Cours à l'extérieur** (piscine, stade municipal) avec temps de trajet | lieu extérieur (337) ; temps de déplacement ; disponibilité du lieu (créneaux attribués par la commune) |
| CL-48 | **Assistant de langue** présent deux jours par semaine | intervention de nature `assistant` ; sa présence ne conditionne pas la séance |
| CL-49 | **Stagiaire** ou professeur en formation (jours de formation fixes) | indisponibilités ; interventions datées |
| CL-50 | **Professeur absent** dont les séances sont réparties entre trois remplaçants | trois modifications `intervenant` ; aperçu du socle 15 |
| CL-51 | **Rattrapage** d'un cours annulé (T-14 de la matrice) | proposition de créneaux où public, intervenant et lieu sont libres ; relance si non fait |
| CL-52 | Famille de **parents séparés**, un parent avec interdiction de remise | l'EDT est visible selon les droits du lien (socle 4) ; « où est l'enfant » n'est **jamais** montré à une personne sous interdiction (socle 4, `ent_eleve_interdiction` 271) |
| CL-53 | **Élève protégé** (adresse confidentielle, mesure d'éloignement) | aucune information de localisation en temps réel hors personnels habilités ; iCal de la famille sans lieu si un réglage de protection l'impose [R] |
| CL-54 | **Élève sorti** de l'établissement en cours d'année | plus aucune séance future dans son EDT ; abonnement iCal de la famille coupé à la date |
| CL-55 | **Jour férié** tombant sur une semaine A | la semaine reste A (l'alternance est par semaine, pas par jour) ; aucun cours ce jour-là |
| CL-56 | **Pont** décidé par l'établissement en cours d'année | événement du calendrier (socle 5) → séances neutralisées ; notifications groupées |
| CL-57 | **Classe virtuelle / distance** un jour de fermeture avec continuité pédagogique | modification `distance` si retenue (Q-EDT9) |
| CL-58 | **Ensemble scolaire** collège + lycée partageant des salles et des professeurs dans **le même tenant** | une seule génération ou deux générations coordonnées par verrous ; salles partagées |
| CL-59 | Un professeur **demande** un échange de séance avec un collègue, qui refuse | demande refusée, rien ne change, motif facultatif |
| CL-60 | Publication d'une version **pendant** une journée de cours (11 h) | prise d'effet au plus tôt le lendemain, sauf forçage ; les séances de l'après-midi ne bougent pas en silence |
| CL-61 | **Effectif** d'un groupe supérieur à la capacité de la salle | avertissement en génération (coût) et à la modification |
| CL-62 | **Accessibilité** : élève en fauteuil, salle à l'étage sans ascenseur | besoin d'accessibilité du public (indicateur seulement, jamais le motif) → salle `accessible_pmr` exigée (337) |
| CL-63 | **Deux années** vivantes : construction de N+1 en juillet pendant que N est archivée | l'EDT de N+1 est un brouillon de l'année en préparation ; aucune lecture famille avant publication |

---

## 11. Génération automatique de l'emploi du temps (D-025)

### 11.1 Objectif et périmètre [R]
Construire, pour une année et une plage de dates, **un emploi du temps type** (placements de toutes les pièces de tous les cours) qui respecte toutes les contraintes dures et minimise le coût des contraintes souples ; en donner l'**explication** ; permettre de **reprendre** une partie ; **comparer** plusieurs scénarios. Le moteur ne publie jamais : il produit un **scénario** (brouillon).

**Entrées** (toutes lues dans ScolIA, aucune ressaisie) : cours et publics (socle 5), alignements (5), interventions et heures (6), grilles et semaines types (5), lieux et équipements (5), indisponibilités (6, 5), vœux (§12), contraintes d'établissement (§11.2), **verrous** (placements imposés : l'EPS au gymnase aux heures attribuées par la commune, un cours déjà accepté par un professeur).

**Hors périmètre de la première version** [R] : l'emploi du temps **par quinzaine variable** (cours dont le volume change d'une semaine à l'autre au-delà de A/B), la génération des **examens** (socle 38), l'optimisation **conjointe** de la répartition des services (qui enseigne quoi : socle 6 décide, le moteur peut seulement **signaler** qu'une autre répartition rendrait l'EDT faisable).

### 11.2 Contraintes

**Dures (H-xx) — jamais violées dans un scénario proposé comme complet** :

| N° | Contrainte | Source |
|---|---|---|
| H-01 | Un élève n'est jamais dans deux séances au même moment (publics calculés : classe, groupes, inclusions ; deux cours dont les publics se recouvrent ne se chevauchent pas, sauf s'ils sont dans le même alignement et que leurs publics sont disjoints) | structure |
| H-02 | Un intervenant n'est jamais dans deux séances au même moment (sauf co-enseignement de la même séance) | services |
| H-03 | Un lieu n'accueille qu'une séance à la fois (sauf lieu déclaré « partageable », ex. gymnase à plusieurs ateliers, avec capacité) | lieux |
| H-04 | Les cours d'un alignement sont placés aux mêmes plages | structure |
| H-05 | Chaque pièce est placée dans les créneaux de cours de la grille applicable (site, niveau, jour) | grille |
| H-06 | Les indisponibilités dures (personnes, lieux, publics) sont respectées | 6, 5 |
| H-07 | Les verrous sont respectés | ADJ |
| H-08 | Les lieux respectent le type et les équipements exigés par le cours | 5 |
| H-09 | Le temps de déplacement entre sites est respecté pour les intervenants et les publics | 5 |
| H-10 | Les « plages interdites » de l'établissement (mercredi après-midi, samedi, pause méridienne minimale si l'établissement la rend dure) | ADJ |
| H-11 | Les pièces consécutives (bloc de 2 h de TP) ne sont pas coupées par une récréation si l'établissement l'interdit, ni par la pause méridienne | ADJ |
| H-12 | Le volume horaire de chaque cours est exactement placé (en moyenne sur le cycle A/B) | 5 |

**Souples (G-xx) — chaque écart coûte des points, pondérés par trois niveaux (faible, normal, fort)** :

| N° | Contrainte | Pour qui |
|---|---|---|
| G-01 | Trous des élèves (heures sans cours entre deux cours dans une demi-journée) : minimiser, plafond par jour | élèves |
| G-02 | Trous des professeurs : minimiser ; respect du plafond demandé dans les vœux | professeurs |
| G-03 | Équilibre de la semaine de l'élève (pas une journée de 8 h et une de 3 h ; charge par jour plafonnée) | élèves |
| G-04 | Une même matière pas deux fois le même jour (sauf bloc) ; répartie sur la semaine (ex. maths 4 h sur 4 jours) | élèves |
| G-05 | Matières « lourdes » plutôt le matin [H, pratique, pas une règle] | élèves |
| G-06 | Pause méridienne d'au moins la durée recommandée (§5) pour élèves et professeurs | tous |
| G-07 | Amplitude journalière limitée (premier cours – dernier cours) | élèves, professeurs |
| G-08 | Vœux des professeurs (§12), avec équité entre professeurs | professeurs |
| G-09 | Stabilité : pour une reprise ou une nouvelle version, ne pas déplacer ce qui ne doit pas l'être (chaque déplacement coûte) | tous |
| G-10 | Salles : un professeur garde sa salle autant que possible ; une classe change peu de bâtiment | professeurs, élèves |
| G-11 | EPS : regroupements par créneaux d'installations attribués, durée de trajet | EPS |
| G-12 | Demi-groupes en miroir (TP du groupe A pendant que le groupe B a un autre cours de la même classe) pour éviter les trous | élèves |
| G-13 | Dernière heure du vendredi, première heure du lundi : éviter pour les cours à faible volume (ne pas toujours sacrifier la même matière) | élèves |
| G-14 | Remplissage des salles spécialisées pour les cours qui en ont réellement besoin (pas une salle informatique pour un cours d'histoire) | établissement |

Les poids par défaut sont fixés par ScolIA et testés sur les jeux de labo ; l'établissement ne règle **que** trois niveaux par famille de contraintes (élèves, professeurs, salles) et la liste de ses plages interdites [R, D-023]. Les poids numériques ne sont pas exposés (réglage avancé caché, S-81).

### 11.3 Diagnostic préalable : « est-ce faisable ? » [R]
Avant toute génération, et dès mars (structure prévisionnelle), un **diagnostic** en quelques secondes, sans placer quoi que ce soit, détecte les impossibilités évidentes et les nomme :
- intervenant dont les heures à placer dépassent ses plages disponibles ;
- public (classe) dont les heures dépassent les créneaux de cours de sa grille ;
- alignement impossible (un professeur dans deux cours du même alignement ; un élève dans deux groupes d'un même alignement) ;
- lieux spécialisés insuffisants (heures de TP demandées > heures de salles de TP disponibles) ;
- pièces impossibles à placer (bloc de 3 h dans une demi-journée de 2 créneaux) ;
- verrous contradictoires ;
- cours sans public, cours sans volume, cours vacant (signalé, pas bloquant).
Chaque problème est rattaché aux objets qui le causent, avec un lien vers l'écran qui le corrige (structure, service, indisponibilité). C'est l'aide la plus utile en mars (« si on ne crée pas un troisième groupe de TP, l'EDT est impossible ») et un argument face aux logiciels qui ne disent « échec » qu'après une nuit de calcul.

### 11.4 Algorithme [R]
Le choix du moteur est une décision technique à confirmer par un prototype (lot T8a) ; la conception fixe les **propriétés exigées** et une approche de référence, écrite en Rust, sans dépendance AGPL ni GPL (D-005 par analogie ; FET est sous licence **AGPL-3.0** [V, R-20] et **ne sert que d'inspiration pour son format et ses idées publiques, aucun code** (`09_INTEROP.md` FX-82, C-3)) :

**Propriétés exigées.**
1. **Déterminisme** : mêmes entrées (empreinte calculée et stockée) + même graine + même version du moteur → même scénario, au placement près. La graine est stockée avec le scénario. Aucun parallélisme non déterministe (ou parallélisme avec fusion ordonnée).
2. **Explicabilité** : chaque placement porte la raison de son choix (contraintes qui l'ont imposé, coût évité) ; chaque pièce non placée porte la **chaîne de conflits** qui l'empêche (« 4B Maths, 1 h restante : toutes les plages libres de M. X sont prises par les alignements de LV2 de 4A/4B ou par l'indisponibilité du jeudi ») ; chaque contrainte souple non respectée porte son coût et une **alternative** connue (« accepter un trou de plus pour Mme Y libérerait le mercredi de la 5C »).
3. **Interruptible et progressif** : le calcul tourne en tâche de fond (socle 1, registre des tâches), publie sa progression, peut être arrêté ; le meilleur scénario trouvé est toujours disponible.
4. **Reprise partielle** : verrouiller un ensemble de placements (un professeur, une classe, un jour, tout sauf X) et ne recalculer que le reste ; la stabilité (G-09) pénalise les déplacements des placements non verrouillés d'un scénario de référence.
5. **Bornes de performance** [H, à mesurer au lot T8a] : collège de 600 élèves (25 classes, ~50 intervenants, ~750 heures-élèves placées par semaine) : premier scénario complet en < 2 min ; lycée de 1 500 élèves avec spécialités (60 divisions, 150 intervenants, alignements multiples) : < 15 min pour un scénario de bonne qualité, amélioration continue ensuite ; diagnostic préalable < 10 s.
6. **Aucun appel à un service externe** (pas de solveur en ligne, pas de modèle de langage pour placer) ; les données d'élèves ne quittent pas le serveur ; le moteur ne lit **que** les données nécessaires (cours, publics agrégés par identifiants, intervenants par identifiants, lieux), jamais les noms ni les fiches (minimisation).

**Approche de référence** (proposition, à valider par T8a) :
1. **Prétraitement** : graphe de conflits entre pièces (deux pièces sont en conflit si elles partagent un élève, un intervenant ou un lieu obligatoire) ; regroupement des alignements en « super-pièces » ; calcul des domaines (plages possibles de chaque pièce après contraintes dures unaires : grille, indisponibilités, plages interdites, verrous).
2. **Construction** : placement glouton ordonné par difficulté (domaine le plus petit d'abord, degré de conflit le plus fort ensuite), avec **retour arrière borné** et **réparation** (éjection d'une pièce déjà placée qui libère une plage, en chaîne courte, approche connue sous le nom d'« ejection chains » [H]) ; affectation des lieux après le temps, ou conjointement pour les lieux rares (salles de TP, gymnase).
3. **Amélioration** : recherche locale sur le coût souple (déplacements, échanges de deux pièces, échanges de plages entières entre jours), avec liste taboue ou recuit simulé à graine fixée ; voisinage de grande taille (on libère un jour d'une classe et on le reconstruit) pour sortir des minima.
4. **Évaluation incrémentale** du coût (chaque mouvement recalcule seulement les publics et intervenants touchés).
5. **Option à étudier** : un solveur de programmation par contraintes ou linéaire en nombres entiers sous licence permissive, pour les sous-problèmes durs (alignements, salles rares). Avant adoption : identité exacte, licence (MIT/Apache/BSD/MPL-2.0 approuvée, D-016), activité, binding Rust, déterminisme, taille, audit (`cargo deny`). [NV : aucun solveur n'a été évalué pour ce document.]

**Pourquoi pas une IA générative pour placer** : non déterministe, non explicable au sens exigé, coûteuse, et elle ferait sortir des données. L'IA de cœur (socle 1 bis) intervient **autour** : expliquer un conflit en langage clair, proposer une reformulation des vœux, résumer les différences entre deux scénarios, toujours à partir des sorties du moteur et avec les droits de l'utilisateur (§20).

### 11.5 Scénarios et comparaison [R]
Un scénario porte : ses entrées (empreinte), ses réglages (niveaux de poids, plages interdites), sa graine, son état (`en_calcul`, `complet`, `incomplet`, `arrete`, `abandonne`), ses placements, son **tableau de qualité** :
- heures placées / à placer, par cours, classe, intervenant ; pièces non placées avec explication ;
- trous élèves (moyenne, maximum, nombre de classes au-delà du plafond) ; trous professeurs ;
- journées les plus longues ; pauses méridiennes inférieures au seuil ;
- vœux satisfaits par professeur (part et liste), indicateur d'**équité** (écart entre le mieux et le moins bien servi) ;
- changements de salle et de bâtiment ; occupation des salles spécialisées ;
- différence avec la version publiée (nombre de placements déplacés, publics et professeurs touchés) pour une nouvelle version en cours d'année.

La **comparaison** côte à côte de deux à quatre scénarios montre ces indicateurs et les différences par classe et par professeur (« dans B, Mme Y gagne son mercredi, la 3C perd une heure de trou »). Un scénario **retenu** devient le brouillon de la future version ; les autres sont gardés jusqu'à la publication puis purgés après la durée de conservation (§15.6).

### 11.6 Retouches à la main [R]
Après génération, l'ADJ **déplace** à la souris (web, grand écran) une pièce : le serveur montre en temps réel les plages compatibles (vert), coûteuses (orange, avec le coût), impossibles (gris, avec la raison au survol) ; il peut **échanger** deux pièces, **verrouiller** une pièce, **relancer** l'amélioration sur le reste. Chaque retouche est un mouvement du scénario, réversible (annuler / rétablir), journalisé.

---

## 12. Vœux et contraintes des professeurs

### 12.1 Campagne [R]
L'ADJ ouvre une **campagne de vœux** pour l'année en préparation : dates, consignes (texte court), nombre maximal de vœux par professeur (défaut 5), plage de dates. Chaque professeur concerné (interventions prévues au socle 6, PRS-50) reçoit une invitation (socle 8), une relance avant la clôture, et voit ses vœux de l'année précédente proposés à la reconduction.

### 12.2 Ce qu'un professeur exprime [R]
- **Indisponibilités** (dures, justifiées ou non selon le cas) : service partagé, temps partiel, décharge, formation régulière — avec un **motif codé** court (`service_partage`, `temps_partiel`, `decharge`, `formation`, `autre`), **jamais de motif de santé ou de vie privée** demandé ni stocké. Une indisponibilité dure non statutaire (« garde d'enfant le mercredi ») est un **vœu fort**, pas une indisponibilité, sauf accord de la direction (droit `edt.voeux_arbitrer`).
- **Préférences** (souples) : demi-journée libérée, pas de première heure tel jour, pas de dernière heure tel jour, regrouper les heures sur N jours, au plus K heures de trou par jour ou par semaine, pause méridienne, salle préférée.
- **Priorité** de chaque vœu (1 à 5, ordre) et un commentaire libre (visible ADJ/DIR seulement).
- **Créneaux de disponibilité pour le remplacement de courte durée** (volontaires, R-05) : au moins une heure fixe par semaine, saisie avec les vœux ; la génération les **préserve** (contrainte souple forte : la plage reste libre de cours) ; le socle 15 les lit pour proposer un remplaçant.

### 12.3 Équité et retour [R]
- Les vœux pèsent selon leur priorité et une **règle d'équité** : chaque professeur dispose d'un même « budget » de points ; la génération maximise la satisfaction minimale avant la satisfaction totale (critère « max-min » [R]), pour éviter que certains soient toujours sacrifiés.
- Après publication, chaque professeur voit **ses** vœux avec l'état « satisfait / partiellement / non satisfait » et, pour un vœu non satisfait, une **raison** lisible tirée de l'explication du moteur (« votre lundi M1 est occupé par l'alignement de LV2 des 4e, imposé par la structure »), sans révéler les vœux des collègues.
- Les vœux de l'année précédente et leur satisfaction servent d'**historique** (option d'équité pluriannuelle, Q-EDT6).

### 12.4 Contraintes d'établissement [R]
Saisies par l'ADJ/DIR : plages interdites (mercredi après-midi, samedi), pause méridienne minimale, plafond de trous, amplitude maximale, créneaux EPS attribués par la commune (verrous de lieu), réunions régulières (conseil pédagogique tel jour), plages réservées à l'AS, à la chorale, aux études. L'existant `ent_edt_contrainte` (083 : `prof_indispo`, `salle_indispo`, `preference`) est repris dans ce modèle (§22).

---

## 13. Échanges : logiciels d'EDT, STS-web, agendas, impression

> **Fait (Florian, 2026-11-09)** : l'établissement pilote construit aujourd'hui son emploi du temps **dans Charlemagne** (module emploi du temps), sans logiciel d'EDT séparé. Conséquence : au pilote, la reprise de l'EDT se fait depuis Charlemagne (même canal que la reprise des autres données, socle 28 et socle 9), et la priorité est que ScolIA **construise et modifie l'EDT lui-même** (lots de saisie et de génération), l'import EDT/UnDeuxTEMPS/aSc restant pour les futurs établissements. Format d'export Charlemagne à obtenir sur un jeu réel anonymisé ([NV]).

Le socle 9 possède le **moteur d'échanges** (réception, détection, correspondances, propositions, différences, traces) et les **connecteurs** (`09_INTEROP.md` §6, §9.7, lots X3, X15, X16) ; le socle 10 fournit le **contenu** (ce qu'un import d'EDT propose, ce qu'un export d'EDT contient) et **applique** les propositions par ses commandes. [R]

### 13.1 Trois situations d'établissement [R]
| Situation | Ce que fait l'établissement | Ce que fait ScolIA |
|---|---|---|
| **A. Construit dans ScolIA** (cible) | Structure, services, vœux, génération, publication dans ScolIA | Export des **services vers STS-web** (public et privé sous contrat, FX-32, lot X16 bis, après spécification) ; aucun import d'EDT |
| **B. Garde son logiciel d'EDT** (transition, la plus fréquente au pilote [H]) | Construit dans EDT (Index Éducation), UnDeuxTEMPS, aSc ou FET ; exporte vers STS-web lui-même | **Export de la structure** (classes, groupes, cours, services, salles) vers le logiciel (ITP-62), **import des placements** en proposition (FX-80 à 83) ; correspondances mémorisées ; réimports en cours d'année sans perte des modifications ponctuelles (CL-45) |
| **C. Tableur** (petit établissement, hors contrat) | Saisit ses créneaux dans un tableur au format ScolIA documenté | Import FX-84 (existant `POST /edt/import` CSV, à reprendre dans le moteur du socle 9) |

### 13.2 Ce qu'un import d'EDT propose [R]
- **Correspondances** d'abord : divisions, groupes, matières, professeurs, salles du fichier ↔ objets ScolIA (une fois, mémorisées, ITP-18/19). Un objet inconnu n'est **jamais créé** par l'import : il devient une proposition au socle propriétaire (un groupe au socle 5, une salle au socle 5, un professeur au socle 6), ou un rejet.
- Puis les **placements** : chaque cours du fichier rattaché à un `ent_cours` (par public + matière, ou par code d'export mémorisé), chaque séance hebdomadaire à une plage de la grille (heures rapprochées des créneaux de la grille ; écart signalé), semaine type, salle.
- **Différence à trois voies** (socle 9) : version publiée / dernier import / nouveau fichier ; l'ADJ voit « 12 placements changent, 3 publics touchés, 2 professeurs », choisit d'appliquer en **nouvelle version** à une date.
- **Garde-fous** : UAI du fichier = UAI du site (socle 9, §6.3) ; année ; aucun import ne modifie le passé ; les modifications ponctuelles existantes sont conservées et celles devenues incohérentes listées.

### 13.3 STS-web [R ; formats NV]
- **Lecture** (FX-03, `sts_emp_<UAI>_<année>.xml`, socle 9) : individus, structures, services — utilisée par les socles 5 et 6 (propositions de structure et de services) ; le socle 10 n'en lit que ce qui sert à vérifier les heures (services déclarés ↔ heures placées).
- **Écriture** : dans la situation A, ScolIA doit produire ce que le logiciel d'EDT produit aujourd'hui pour STS-web (ventilation des services) ; format et procédure **non vérifiés** (§5, point R-10) ; lot X16 bis conditionné à la spécification (inscription éditeur, D-029) et à un jeu d'or (D-14 du socle 9).
- **Contrôle avant export** : heures placées par intervenant et par cours = heures des services (socle 6) ; écarts listés avant tout export.

### 13.4 Agendas (iCal) [R]
Repris du socle 9 (FX-85, lot X15) : abonnement personnel à **jeton** aléatoire (empreinte en base), révocable, à expiration (fin d'année + renouvellement automatique tant que le droit existe), droits **relus à chaque lecture** (un parent qui perd son droit, un élève sorti → flux vide). Rendu **minimisé** : matière, heure, lieu, état (annulé / déplacé) ; nom de l'intervenant selon réglage (défaut : nom d'usage, Q-EDT8) ; jamais de motif, jamais de liste d'élèves. `TZID` du fuseau de l'établissement. Identifiant `UID` stable par séance (une séance déplacée garde son `UID` et change de `DTSTART`, pour que les agendas mettent à jour au lieu de dupliquer). L'existant `ent-api/src/ical.rs` (routes `/eleves/{id}/edt.ics`, `/edt/mes-creneaux.ics`) est remplacé : il est servi **sous session**, pas par jeton, et ne peut donc pas être abonné par un agenda externe (D14, §2.2).

### 13.5 Impression et documents [R, D-023]
Trois ou quatre **trames prêtes** (socle 7) : EDT de classe, d'élève, de professeur, de salle, en semaine type (A/B côte à côte) ou semaine datée ; affichage de porte de salle ; récapitulatif « changements de demain » pour la salle des professeurs et la vie scolaire. Aucun éditeur de mise en page. Couleurs des matières (socle 5) reprises.

### 13.6 Exports de données
Tableur des placements, des séances d'une période, des heures effectuées (pour le socle 15 et la paie, 49) : réservés aux administratifs (D-022), tracés (socle 9, `ent_export_trace`), sans données d'élève nominatives sauf nécessité (liste d'appel : socle 12).

---

## 14. Publication et notifications

### 14.1 Publier une version [R]
1. **Contrôles serveur** (bloquants) : aucun conflit dur sur l'ensemble des séances qui seront créées (à la date, avec les publics et interventions datés, y compris les modifications ponctuelles déjà posées sur la plage) ; aucun cours d'un alignement désaligné ; dates dans l'année.
2. **Contrôles serveur** (avertissements) : heures placées ≠ volume des cours ; intervenants au-delà de leurs heures ; cours vacants ; trous au-delà du plafond ; pause méridienne courte ; salles sur-occupées.
3. **Aperçu** : à partir de quel jour ; nombre de séances créées, retirées, conservées ; publics et intervenants touchés ; modifications ponctuelles conservées et devenues incohérentes ; notifications qui partiront.
4. **Publication** en une transaction : la version précédente reçoit `valable_au = veille`, la nouvelle devient `publiee`, les séances sont matérialisées pour la fenêtre (ou en tâche de fond par tranches, avec un état « en cours de matérialisation » visible et sans lecture incohérente), l'événement `edt.version_publiee` part dans l'outbox (socle 8 : un message par personne, pas par séance).
5. **Publication en deux temps** (défaut [R]) : aux **personnels** d'abord (date de publication interne), aux **familles et élèves** ensuite (date de diffusion), pour laisser les professeurs signaler une erreur. Un réglage permet la diffusion immédiate (S-82).

### 14.2 Notifications émises (vers le socle 8) [R]
| Fait | Destinataires | Urgence (socle 8) | Regroupement |
|---|---|---|---|
| Nouvelle version publiée (aux personnels) | intervenants touchés | C0 ordinaire | un message par personne |
| Nouvelle version diffusée (familles, élèves) | publics touchés | ordinaire | un par personne, résumé des changements de **son** EDT |
| Séance annulée, déplacée, modifiée (lieu, horaire, intervenant) | public (élève ; titulaires si mineur et selon préférences), intervenant, VS si permanence | selon délai (§9.5) | un par changement, regroupé par jour |
| Séance non assurée (vacance, absence non remplacée) | public, VS, ADJ | selon délai | idem |
| Lot (grève, fermeture, sortie) | publics, intervenants | U2 si lendemain | **un seul** message par lot et par personne |
| Rétablissement (levée d'une modification) | ceux qui avaient été prévenus | même urgence que l'annonce | — |
| Demande d'un professeur reçue / acceptée / refusée | ADJ / demandeur | ordinaire | — |
| Campagne de vœux ouverte, relance, clôture | professeurs concernés | ordinaire | — |
| Scénario calculé | ADJ | ordinaire | — |
| Sortie anticipée de l'élève (calculée par le socle 14 à partir de l'EDT) | titulaires + personnes de confiance (R-N60 du socle 8) | U2 | — |

Le contenu ne contient **jamais** le motif interne ; le motif public est choisi dans la liste courte (§9.1). Le nom du remplaçant est donné (socle 6 PRS-46 : « familles informées du nom »).

### 14.3 « Demain » et « maintenant » [R]
- **Tableau « demain »** (ADJ, VS, DIR ; généré à 18 h la veille et mis à jour en continu) : absences de professeurs connues, séances non assurées, remplacements faits et à faire, salles fermées, sorties, examens, permanences attendues (effectifs par heure), notifications envoyées et à envoyer.
- **« Maintenant »** (VS, CPE, ACC selon droit) : pour un élève, sa séance en cours (cours, lieu, intervenant) ou « pas de cours (permanence / autorisé à sortir selon régime) » ; pour une salle, son occupation ; pour un professeur, **présent dans l'établissement selon l'EDT : oui / non** (jamais où il est quand il n'a pas cours).

---

## 15. Droits et minimisation

### 15.1 Catalogue d'actions proposé (D-017) [R]
Existant [F] : `edt.gerer`, `edt.annuler_masse` (`ent-auth/src/actions.rs:79, 165`). Proposé :

| Action | Portées possibles | Modèles par défaut | Confirmation forte ★ |
|---|---|---|---|
| `edt.lire` (EDT d'un cours, d'une classe, d'un groupe, d'une salle) | établissement, mes classes (D-022 : cours et groupes enseignés) | tout le personnel (établissement pour les salles, `mes classes` pour les publics du professeur) | — |
| `edt.lire_eleve` (EDT d'un élève nominatif) | établissement, mes classes, mes groupes, lien (famille), moi-même | DIR, SEC, CPE, VS, PP (sa classe), PRF (ses élèves), FAM (lien avec droit), ELV (soi) | — |
| `edt.localiser` (« où est X maintenant ») | établissement | CPE, VS, ACC, DIR, INF | — ; **journalisé** (lecture sensible) |
| `edt.lire_personnel` (EDT d'un collègue) | établissement | DIR, ADJ ; tout le personnel en **présence oui/non** seulement | — |
| `edt.modifier_seance` (annuler, déplacer, changer lieu/horaire, ajouter, fusionner) | établissement, mes classes | ADJ, DIR ; CPE pour `changement_lieu` [R] | annulation en lot ★ |
| `edt.annuler_masse` (existant) | établissement | DIR, ADJ | ★ |
| `edt.corriger_passe` | établissement | DIR, ADJ | ★ |
| `edt.demander` (demande de déplacement, d'échange, de salle, de rattrapage) | mes classes | PRF | — |
| `edt.construire` (scénarios, génération, retouches, verrous) | établissement | ADJ, DIR | — |
| `edt.publier` | établissement | DIR, ADJ | ★ |
| `edt.voeux_campagne` (ouvrir, clore) | établissement | ADJ, DIR | — |
| `edt.voeux_saisir` | moi-même | PRF (et tout intervenant) | — |
| `edt.voeux_lire` (vœux de tous) | établissement | ADJ, DIR | — |
| `edt.voeux_arbitrer` (rendre dure une indisponibilité non statutaire) | établissement | DIR | — |
| `edt.contraintes_gerer` (contraintes d'établissement, indisponibilités de salles) | établissement | ADJ, DIR | — |
| `edt.importer` / `edt.exporter` (via `echanges.*` du socle 9) | établissement | SEC, ADJ, DIR ; **jamais PRF** (D-022) | — |
| `edt.abonner` (iCal) | moi-même, lien | tous ; FAM selon droit du lien | — |
| `edt.parametrer` (heure du message de la veille, publication en deux temps, auto-acceptation) | établissement | DIR, ADM | — |

`edt.gerer` existant est **décomposé** (bascule sans changement de comportement : un rôle qui l'avait reçoit `edt.modifier_seance`, `edt.construire`, `edt.publier`, `edt.contraintes_gerer`). Le moteur `autoriser` (socle 1) reste le seul point de décision ; la portée « mes classes » est calculée par la fonction unique du socle 5 (`ent_public_cours_jour` / `structure::perimetre`, Q-S21).

### 15.2 Ce que voit chacun [R]
| Qui | Voit | Ne voit pas |
|---|---|---|
| Famille (lien avec droit « scolarité ») | l'EDT daté de l'enfant, ses changements, le nom des intervenants (selon Q-EDT8), les salles | les autres élèves du groupe ; le motif d'une absence ; les vœux ; « où est l'enfant maintenant » en dehors de l'EDT (pas de localisation temps réel) |
| Personne sous interdiction (socle 4, 271) ou lien sans droit | rien | — |
| Élève (mineur ou majeur) | son EDT daté, ses changements | idem famille |
| Professeur | son EDT, l'EDT de ses cours et des élèves de ses cours (D-022, au groupe près), l'occupation des salles, la présence oui/non des collègues | l'EDT nominatif d'un élève qu'il n'enseigne pas ; les vœux des collègues ; l'EDT détaillé d'un collègue |
| PP | + l'EDT complet de sa classe (fonction attachée à la classe, socle 6, Q-S5) | — |
| CPE, VS | EDT des classes et élèves, localisation (« maintenant »), permanences | vœux ; motifs internes |
| Accueil | localisation d'un élève **au moment de la demande** (pour remise vérifiée, socle 14) ; présence oui/non d'un personnel | l'EDT de la semaine ; l'historique |
| Agents | occupation des salles (sans liste d'élèves) | publics nominatifs |
| ADJ, DIR | tout, y compris vœux, motifs internes, scénarios | données de santé (jamais dans l'EDT) |

### 15.3 Garde-fous [R]
- **Aucun motif de santé** dans l'EDT (aménagement PAP/PPS : exclusion avec motif codé `parcours_amenage`, R-S5 du socle 5 ; le détail vit au socle 34).
- **Localisation** : `edt.localiser` est une lecture unitaire (un élève, l'instant présent), journalisée avec le motif d'usage (remise, urgence, recherche) ; pas de liste « où sont tous les élèves de la 4B » hors PPMS (socle 14, mode urgence).
- **Vœux** : visibles du professeur concerné et de ADJ/DIR ; jamais exportés ; purgés après la durée de conservation (§15.6) sauf leur taux de satisfaction agrégé.
- **iCal** : jeton à forte entropie, empreinte seule en base, droits relus, rendu minimisé, rate limit, révocation.
- **Cloisonnement** : toutes les tables portent `etablissement_id NOT NULL`, clés composées, RLS (comme les socles 5 et 6) ; le moteur de génération s'exécute dans la transaction d'établissement et ne lit qu'un établissement ; aucune donnée entre établissements, même pour un professeur en service partagé dans deux tenants (D-013).
- **IA** : l'IA lit l'EDT avec les droits de l'utilisateur ; elle ne lit jamais les vœux d'autrui pour un professeur.

### 15.4 Matrice de séparation (tests) [R]
Pour chaque route EDT : un autre établissement (refus) ; un professeur hors de ses groupes (classe mixte allemand/espagnol : ne voit pas l'EDT nominatif d'un élève d'espagnol) ; un parent d'une autre famille ; un parent sous interdiction ; un élève qui demande l'EDT d'un autre élève ; un jeton iCal d'un élève sorti ; un jeton révoqué ; un AED qui tente `edt.publier` ; un professeur qui tente un export ; un professeur qui lit les vœux d'un collègue.

### 15.5 Données personnelles [R]
- **Personnes concernées** : élèves (EDT nominatif = déduit de leur scolarité), personnels (EDT, vœux, indisponibilités), familles (abonnements).
- **Finalité** : organisation des enseignements et de la vie scolaire ; information des familles ; décompte des heures effectuées.
- **Minimisation** : le moteur de génération ne lit que des identifiants et des effectifs ; les vœux ne demandent aucun motif ; aucun suivi de localisation continue.

### 15.6 Conservation [R, NV pour les durées]
Version publiée et séances : année + durée d'archive de la vie scolaire (les séances justifient l'appel et les heures payées) ; scénarios non retenus : purgés 30 jours après la publication de la version ; vœux nominatifs : fin de l'année suivante (équité pluriannuelle) puis agrégés ; journaux de localisation : 1 an [NV] ; jetons iCal : révoqués à la sortie, purgés après 3 mois. Durées à valider avec le DPO (catégories du socle 7).

---

## 16. Actions métier (EDT-100 et suivantes)

Les identifiants `EDT-01` à `EDT-08` de `SOCLE2_DOMAINES.md` §D4 et `EDT-01` à `EDT-17` de `MATRICE_CHARLEMAGNE.md` §3.6 sont des **lignes de couverture** déjà prises ; les actions de ce socle commencent donc à **EDT-100** (correspondance dans la colonne « Réf. »). Acteurs : abréviations du §3 ; ★ = confirmation forte. Événements au §18.

### 16.1 Consulter
| ID | Action | Acteur | Règles clés | Réf. |
|---|---|---|---|---|
| EDT-100 | Voir **mon emploi du temps** de la semaine (datée), avec changements surlignés | tous | à la date ; semaine A/B résolue ; changements depuis la dernière consultation marqués | M EDT-16 |
| EDT-101 | Voir l'EDT d'un **élève** | FAM, ELV, PRF, PP, CPE, VS, SEC, DIR | `edt.lire_eleve` ; public daté (inclusions, exclusions) | M EDT-01 |
| EDT-102 | Voir l'EDT d'une **classe**, d'un **groupe**, d'un **cours** | personnel | `edt.lire` ; portée D-022 pour PRF | M EDT-01 |
| EDT-103 | Voir l'EDT d'un **professeur** (le sien ; un collègue pour ADJ/DIR) | PRF, ADJ, DIR | autres : présence oui/non | M EDT-01 |
| EDT-104 | Voir l'occupation d'une **salle** ou d'un lieu, et les salles libres à une plage | personnel, AGT | sans liste d'élèves | M EDT-01 |
| EDT-105 | Voir la **semaine type** (A, B côte à côte) | personnel, FAM, ELV | lecture du plan, sans séance | — |
| EDT-106 | Voir le **jour** (vue « aujourd'hui » mobile) avec la prochaine séance mise en avant | tous | mobile (socle 11) | — |
| EDT-107 | Voir **toute la fratrie** dans une vue (famille, plusieurs enfants, plusieurs établissements dans la même appli, sans croisement de données entre établissements) | FAM | D-013 : vues côte à côte, une par établissement | — |
| EDT-108 | Voir les **changements** à venir (liste) | tous | depuis la dernière consultation, ou sur N jours | `GET /eleves/{id}/edt/changements` [F] |
| EDT-109 | **Où est X maintenant ?** (élève) | CPE, VS, ACC, INF, DIR | `edt.localiser`, journalisé ; lecture unitaire | nouveau |
| EDT-110 | **Ce personnel est-il présent ?** (oui/non selon l'EDT et les absences connues) | ACC, personnel | jamais le lieu, jamais le motif | nouveau |
| EDT-111 | **Élèves sans cours maintenant** (permanence attendue) | VS, CPE | régimes de sortie (14) ; effectifs par heure | nouveau |
| EDT-112 | **Tableau « demain »** | ADJ, VS, DIR | §14.3 | nouveau (A2, A14) |
| EDT-113 | Trouver un **créneau commun** libre (équipe, classe, salle) | PRF, PP, ADJ ; port pour 19 | réunions, conseils, rattrapages | CL-41 |
| EDT-114 | Rechercher une **salle libre** (capacité, type, équipement, site, plage) | personnel | lecture ; réservation au socle 44 | M VOY-12 |
| EDT-115 | Imprimer / exporter en PDF un EDT (trames, D-023) | personnel ; FAM pour son enfant | trames du socle 7 | — |
| EDT-116 | S'**abonner** à son EDT dans un agenda (iCal) ; révoquer | tous ; FAM pour un enfant | §13.4 | M EDT-05 |
| EDT-118 | Voir le **planning des permanences et études** de la journée (salles, AED, effectifs attendus par heure) | VS, CPE | calculé depuis les séances annulées ou non assurées et les EDT vides | nouveau |
| EDT-119 | Voir **mes heures effectuées** (séances assurées, remplacements, heures supplémentaires) | PRF | lecture des séances réellement assurées (R-E19) ; écarts signalés | M EDT-14 |
| EDT-117 | Voir l'EDT d'un élève **à une date passée** (« le 12 mars, à 10 h, où devait-il être ? ») | CPE, DIR | contestations, incidents | nouveau |

### 16.2 Préparer et construire
| ID | Action | Acteur | Règles clés | Réf. |
|---|---|---|---|---|
| EDT-120 | Lancer le **diagnostic de faisabilité** | ADJ, DIR | §11.3 ; sans écriture | nouveau |
| EDT-121 | Voir et ajuster les **besoins horaires** des cours (découpage en pièces) | ADJ | volume issu du cours (5) ; pièces par défaut proposées (2 h → 1 h + 1 h, TP → bloc) | nouveau |
| EDT-122 | Déclarer les **contraintes d'établissement** (plages interdites, pause minimale, amplitude, plafonds) | ADJ, DIR | §12.4 | M EDT-08 |
| EDT-123 | Déclarer les **indisponibilités d'un lieu** (gymnase pris par la commune, salle en travaux) | ADJ ; SEC | datées ; dures | M EDT-08 |
| EDT-124 | Poser des **verrous** (placement imposé) | ADJ | dur en génération | nouveau |
| EDT-125 | Déclarer un **temps de déplacement** entre deux sites ou lieux | ADJ | contrainte dure | CL-26 |
| EDT-126 | Créer un **scénario** (vide, copie de la version publiée, copie d'un autre scénario, import) | ADJ | brouillon | nouveau |
| EDT-127 | **Générer** un scénario (tâche de fond) ; arrêter ; reprendre | ADJ | §11.4 ; déterministe | M EDT-07 (T-03) |
| EDT-128 | **Reprendre partiellement** (verrouiller tout sauf X et recalculer) | ADJ | G-09 stabilité | nouveau |
| EDT-129 | **Retoucher** à la main (déplacer, échanger, verrouiller), annuler / rétablir | ADJ | §11.6 | M EDT-02 |
| EDT-130 | **Comparer** des scénarios (2 à 4) | ADJ, DIR | §11.5 | nouveau |
| EDT-131 | **Expliquer** un placement, une pièce non placée, un vœu non satisfait | ADJ, DIR ; PRF pour ses vœux | §11.4 point 2 | nouveau |
| EDT-132 | Affecter les **salles** (automatique après le temps, ou à la main) | ADJ | type, équipement, capacité, accessibilité | — |
| EDT-133 | **Retenir** un scénario (il devient le brouillon de la prochaine version) | ADJ, DIR | un seul retenu par plage | nouveau |
| EDT-134 | Abandonner / purger des scénarios | ADJ | conservation §15.6 | — |
| EDT-136 | **Simuler l'effet** d'un changement de structure ou de service sur l'EDT publié (ouvrir un groupe, changer un professeur) avant de le faire au socle 5 ou 6 | ADJ, DIR | sans écriture ; pièces à replacer listées | nouveau |
| EDT-135 | **Saisir à la main** un EDT complet sans génération (petit établissement) | ADJ | même éditeur, sans moteur | M EDT-02 |

### 16.3 Vœux
| ID | Action | Acteur | Règles clés | Réf. |
|---|---|---|---|---|
| EDT-140 | Ouvrir une **campagne de vœux** (dates, consignes, plafond) | ADJ, DIR | professeurs concernés = interventions prévues (6) | nouveau |
| EDT-141 | Saisir ses vœux et indisponibilités ; reprendre ceux de l'an passé | PRF | §12.2 ; pas de motif de santé | M EDT-08 |
| EDT-142 | Relancer les retardataires (automatique avant la clôture) | SYS | socle 8 | — |
| EDT-143 | Voir la **synthèse des vœux** (contradictions, plages très demandées, indisponibilités qui rendent l'EDT difficile) | ADJ, DIR | sans écriture | nouveau |
| EDT-144 | **Arbitrer** un vœu (accepter comme dur, refuser, rendre souple) avec réponse au professeur | DIR | trace | nouveau |
| EDT-145 | Voir le **retour** sur ses vœux après publication | PRF | §12.3 ; raison lisible | nouveau |
| EDT-147 | Déclarer ses **créneaux de disponibilité** pour le remplacement de courte durée (volontaire) | PRF | au moins une heure fixe par semaine (R-05) ; préservés par la génération | nouveau |
| EDT-146 | Clore la campagne | ADJ | vœux figés pour la génération | — |

### 16.4 Publier
| ID | Action | Acteur | Règles clés | Réf. |
|---|---|---|---|---|
| EDT-150 | Contrôler une version avant publication | ADJ, DIR | §14.1 ; sans écriture | nouveau |
| EDT-151 | **Publier** une version à une date (aux personnels, puis aux familles) ★ | DIR, ADJ | §14.1 ; aperçu ; idempotent | M EDT-02 |
| EDT-152 | Publier un EDT **provisoire** de rentrée | ADJ | mention « provisoire » | CL-22 |
| EDT-153 | Avancer ou retarder la **diffusion** aux familles | ADJ | — | — |
| EDT-154 | Voir l'**historique des versions** et les différences entre deux versions | ADJ, DIR | — | — |
| EDT-155 | **Revenir** à la version précédente (avant sa diffusion aux familles, ou à une date future) ★ | DIR | le passé ne change pas | nouveau |
| EDT-157 | Voir **son EDT de l'année suivante** avant la diffusion aux familles et signaler une erreur | PRF | publication en deux temps (§14.1) | nouveau |
| EDT-156 | Publier une **version individuelle** (élève UPE2A, aménagement) sans republier l'établissement | ADJ | par inclusions/exclusions datées du cours (5) | CL-09, CL-34 |

### 16.5 Modifier le réel
| ID | Action | Acteur | Règles clés | Réf. |
|---|---|---|---|---|
| EDT-160 | **Annuler** une séance (motif public, motif interne) | ADJ, DIR | §9 ; effets automatiques | SD EDT-04, M EDT-03 |
| EDT-161 | **Déplacer** une séance (contrôles §9.3, plages libres proposées) | ADJ | — | SD EDT-04 |
| EDT-162 | **Changer le lieu** d'une séance ou d'une série de séances | ADJ, CPE, VS (selon droit) | salle libre proposée | SD EDT-04 |
| EDT-163 | **Changer l'horaire** d'une séance (raccourcir, décaler) | ADJ | — | — |
| EDT-164 | **Ajouter** une séance hors plan (rattrapage, soutien, devoir surveillé) | ADJ | contrôles ; appel attendu | CL-42, CL-51 |
| EDT-165 | **Fusionner** deux séances (deux demi-groupes, deux classes) | ADJ | un appel, publics d'origine gardés | — |
| EDT-166 | Remplacer une séance par un **autre cours** (CL-12) | ADJ (via 15) | séance d'origine annulée, séance du cours de remplacement ajoutée | CL-12 |
| EDT-167 | Changer l'**intervenant** d'une séance (port pour le socle 15) | socle 15 ; ADJ | appel par le remplaçant ; heures | SD EDT-05 |
| EDT-168 | **Lever** une modification (retour à l'état calculé), séance par séance ou en lot | ADJ | message de rétablissement | `DELETE /edt/exceptions-levee` [F] |
| EDT-169 | Annuler **en masse** (grève, intempéries, sortie de niveau) avec aperçu ★ | DIR, ADJ | §9.2 ; un message par personne | SD EDT-08, M EDT-06 |
| EDT-170 | **Grève du jour** : annuler les séances des professeurs déclarés absents ce matin, en une liste à cocher | ADJ, DIR | sans motif nominatif public | CL-18 |
| EDT-171 | Changer de lieu **toutes les séances** d'une salle fermée (proposition de salles libres en lot) | ADJ ; SYS sur fermeture de salle (5) | — | CL-13 |
| EDT-172 | Neutraliser les séances d'une **sortie**, d'un **examen**, d'une **journée banalisée ciblée** (réaction aux socles 42, 38, 5) | SYS | aperçu au demandeur | SD EXA-05 |
| EDT-173 | Corriger une **séance passée** ★ | ADJ, DIR | §9.6 ; effets signalés | nouveau |
| EDT-174 | Marquer une séance **à distance** (si Q-EDT9 oui) | ADJ | — | CL-57 |
| EDT-175 | Proposer des créneaux de **rattrapage** pour une séance annulée, relancer si non fait | SYS → ADJ, PRF | T-14 de la matrice | M EDT-11 |
| EDT-177 | Répartir les **élèves sans cours** entre permanence, étude, CDI (ou sortie selon régime, socle 14) | VS, CPE | effectifs par salle ; liste d'appel de la permanence (socle 12) | nouveau |
| EDT-176 | Voir la liste des **séances non assurées** à venir et les traiter | ADJ, VS | — | — |

### 16.6 Demandes des professeurs
| ID | Action | Acteur | Règles clés | Réf. |
|---|---|---|---|---|
| EDT-180 | Demander un **déplacement** d'une séance | PRF | contrôles faits à la demande | nouveau |
| EDT-181 | Proposer un **échange** de séance à un collègue | PRF | acceptation du collègue, puis de l'ADJ (ou auto, S-80) | CL-59 |
| EDT-182 | Demander une **salle** (autre que la sienne, salle informatique) pour une séance | PRF | auto-acceptation si libre (S-80) | — |
| EDT-183 | Demander une séance de **rattrapage** | PRF | — | — |
| EDT-184 | Traiter les demandes (accepter, refuser avec motif, proposer autre chose) | ADJ | — | — |
| EDT-185 | Voir l'état de ses demandes | PRF | — | — |

### 16.7 Échanges et paramétrage
| ID | Action | Acteur | Règles clés | Réf. |
|---|---|---|---|---|
| EDT-190 | Exporter la **structure** vers un logiciel d'EDT | SEC, ADJ | socle 9 ITP-62 | M EDT-04 |
| EDT-191 | Importer un EDT (logiciel du marché, tableur) en **proposition** ; rapprocher ; appliquer en nouvelle version | SEC, ADJ | §13.2 | SD EDT-01, M EDT-04 |
| EDT-192 | Contrôler les **heures** (placées / services / dues) avant export STS | ADJ | §13.3 | nouveau |
| EDT-193 | Exporter les **services** vers STS-web (situation A) | ADJ, DIR | après spécification (X16 bis) | FX-32 |
| EDT-194 | Exporter les **heures effectuées** (remplacements, HSE) pour la paie | SEC, ADJ, compta | socles 15, 49 | — |
| EDT-195 | Régler l'heure du **message de la veille**, la publication en deux temps, l'auto-acceptation | DIR, ADM | peu de réglages (§26) | — |
| EDT-196 | Archiver l'EDT de l'année à la clôture (contrôle `ControleCloture` du socle 5) | SYS | — | — |
| EDT-198 | Contrôler puis laisser partir la **remontée hebdomadaire** du remplacement de courte durée (agrégats sans nom) | SEC, DIR ; SYS | arrêté du 14/08/2023 (R-09) ; flux au socle 9 ; motif catégorisé au socle 15 | nouveau |
| EDT-197 | Reconduire les **contraintes et vœux** de l'année précédente pour N+1 | ADJ | proposition | — |

**Total : 86 actions** (EDT-100 à EDT-198, numéros non contigus pour laisser de la place par famille ; ordre d'affichage par sens, pas par numéro).

---

## 17. Règles transverses (R-E1 et suivantes) [R]

| N° | Règle |
|---|---|
| R-E1 | Une séance passée ne change jamais sans `edt.corriger_passe` (§9.6). |
| R-E2 | Une séance qui porte quelque chose (appel, contenu de cahier, travail, évaluation, absence, remplacement) n'est jamais supprimée ; elle peut être retirée, avec lien vers sa remplaçante. |
| R-E3 | Le public d'une séance est **toujours** `ent_public_cours_jour(cours, jour)` (socle 5) ; aucun calcul parallèle (Q-S21). |
| R-E4 | L'intervenant en saisie d'une séance est **toujours** celui du socle 6 à la date (`en_saisie(service, date)`), sauf modification `intervenant` explicite de cette séance. |
| R-E5 | Aucun motif interne ni motif de santé ne sort du cercle ADJ/DIR ; les notifications portent le motif public. |
| R-E6 | Une modification qui place quelque chose passe les contrôles du §9.3 ; un forçage est motivé et journalisé. |
| R-E7 | Une publication est atomique du point de vue des lecteurs : avant, l'ancienne version ; après, la nouvelle ; jamais un mélange visible. |
| R-E8 | Un lot de modifications envoie au plus **un** message par personne et par lot. |
| R-E9 | Les heures sont murales dans le fuseau de l'établissement ; les instants UTC sont dérivés. |
| R-E10 | Le moteur de génération est déterministe ; un scénario conserve l'empreinte de ses entrées et sa graine. |
| R-E11 | Le moteur ne lit ni noms ni données sensibles ; il travaille sur des identifiants et des effectifs. |
| R-E12 | Un import ne crée aucun objet de structure ni de personnel ; il propose (socle 9). |
| R-E13 | Une version brouillon n'est jamais lisible par une famille ou un élève (test de séparation). |
| R-E14 | Les jours sans cours du calendrier (socle 5) ne produisent pas de séance (et donc pas de notification de séance). |
| R-E15 | Un professeur ne voit l'EDT nominatif d'un élève que si l'élève est dans le public d'un de ses cours à la date (D-022). |
| R-E16 | La localisation d'un élève est une lecture unitaire journalisée (`edt.localiser`). |
| R-E17 | La levée d'une modification déjà annoncée produit un message de rétablissement aux mêmes destinataires. |
| R-E18 | Les vœux sont personnels : un professeur ne voit que les siens, jamais ceux d'un collègue. |
| R-E19 | Les heures effectuées (paie, remplacements) se calculent sur les séances réellement assurées, jamais sur le plan. |
| R-E20 | Aucune date ou heure en dur « Europe/Paris » dans le code du socle (test de garde par recherche). |

---

## 18. Événements

### 18.1 Émis (outbox, dans la transaction du fait)
| Événement | Agrégat | Charge utile (minimale) | Consommateurs |
|---|---|---|---|
| `edt.version_publiee` | version | version, valable_du, diffusion_familles_le, compte de séances créées/retirées | 8, 12, 20, 21, 14, 40 |
| `edt.version_diffusee` | version | version | 8 |
| `edt.seances_materialisees` | version | plage, nombre | 12 (appels attendus), 20 |
| `edt.seance_modifiee` | séance | séance, type, avant/après (horaire, lieu, intervenant, état), motif public, origine | 8, 12, 14, 20, 21, 15, 40 |
| `edt.seance_annulee` / `edt.seance_retablie` | séance | séance, motif public, origine | 8, 12, 14, 20, 40 |
| `edt.seance_ajoutee` | séance | séance, origine | 8, 12, 20 |
| `edt.lot_modifications` | lot | lot, plage, nombre, publics | 8 (un message par personne), 40, 41 |
| `edt.seance_non_assuree` | séance | séance | 8, 12, 14 |
| `edt.demande_*` (`creee`, `acceptee`, `refusee`) | demande | demande, séance | 8 |
| `edt.voeux_campagne_*` (`ouverte`, `close`) | campagne | campagne | 8 |
| `edt.scenario_calcule` | scénario | scénario, état, indicateurs | 8 (ADJ) |
| `edt.localisation_lue` | — | (journal, pas l'outbox) | journal |

Les noms existants (`edt.seances_generees`, `edt.creneau_modifie`, `edt.exception`, `edt.remplacement_organise`, `edt.annulation_masse`, `SOCLE2_DOMAINES.md` §4) sont conservés en **alias** le temps de la bascule des consommateurs, puis retirés (§22).

### 18.2 Consommés
| Événement | Émetteur | Effet dans le socle 10 |
|---|---|---|
| `referentiel.calendrier_modifie`, `referentiel.fermeture_declaree` / `_levee` / `_modifiee` | 5 (S3) | recalcul des séances de la plage ; lot de neutralisation ; un seul message (STR-94) |
| `referentiel.semaine_modifiee` (étiquette forcée) | 5 | recalcul des séances de la semaine |
| `structure.grille_versionnee` | 5 (S7) | recalcul des heures des séances futures |
| `cours.ouvert` / `cours.ferme` / `cours.public_modifie` / `cours.membre_*` | 5 (S4, S6) | séances futures créées / retirées ; EDT des élèves recalculé (lecture) |
| `referentiel.affectation_changee` | 5 | aucun recalcul de séance (le public est lu à la date) ; notification de son nouvel EDT à l'élève et à sa famille |
| `salle.fermee` / `salle.rouverte` | 5 | proposition de changement de lieu en lot (EDT-171) |
| `service.intervenant_ajoute` / `_retire` / `service.suppleance_ouverte` / `_close` | 6 | intervenant en saisie des séances futures recalculé ; séances `non_assuree` ↔ `prevue` |
| `personnel.indisponibilites` | 6 (PRS-55) | contrainte de génération ; alerte si une séance publiée tombe dans une nouvelle indisponibilité |
| `absence_personnel.declaree` / `_validee` / `_annulee` | 15 | liste des séances touchées (port) ; rien n'est annulé sans décision (socle 15) |
| `sortie.validee` / `sortie.annulee` | 42 | neutralisation des séances des participants ; proposition pour les accompagnateurs |
| `examen.epreuve_planifiee` | 38 | neutralisation ciblée ; salles occupées |
| `reservation.creee` | 44 | contrôle de conflit (refus en amont par le port de lecture) |
| `eleve.scolarisation_close` | 2 | abonnements iCal de l'élève et de sa famille coupés |
| `lien.droit_modifie` / `interdiction.creee` | 4 | abonnements iCal de la personne relus (coupés si besoin) |

---

## 19. Écrans clés

**Web (Leptos, `docs/FRONTEND_LEPTOS.md`)** [R] :
1. **Mon emploi du temps** (tous) : semaine datée, navigation semaine précédente/suivante, mention A/B, changements surlignés, détail d'une séance (lieu, intervenant, état, travail à faire du socle 20), bouton « s'abonner » ; vue jour sur petit écran (375 px).
2. **EDT d'un élève / d'une classe / d'un groupe / d'un professeur / d'une salle** : même composant, filtres, impression (trames).
3. **Tableau « demain »** (ADJ, VS) et **« maintenant »** (VS, ACC : champ de recherche d'un élève → sa séance en cours ; permanences par heure).
4. **Modifications** : depuis une séance (menu : annuler, déplacer, changer de salle, ajouter, fusionner) avec panneau des plages libres ; **lot** (grève, intempéries) avec aperçu et confirmation forte ; liste des modifications avec levée.
5. **Atelier de construction** (ADJ, grand écran) : grille de la semaine type par classe / professeur / salle, panneau des pièces à placer, glisser-déposer avec coloration des plages (compatibles / coûteuses / impossibles), verrous, annuler / rétablir, lancement de la génération avec progression, explications au survol.
6. **Diagnostic** et **tableau de qualité** ; **comparaison de scénarios**.
7. **Vœux** : formulaire du professeur (grille cliquable : indisponible / préféré / à éviter, priorités, plafonds), synthèse et arbitrage (ADJ/DIR), retour après publication.
8. **Publication** : contrôle, aperçu, publication en deux temps, historique des versions et différences.
9. **Demandes** des professeurs et leur traitement.
10. **Échanges** : export de structure, import en proposition avec correspondances et différences (écran du socle 9 avec contenu du socle 10).

**Mobile (socle 11, D-003)** [R] : « aujourd'hui » et « semaine » (tous), changements poussés, détail de séance, appel depuis la séance (socle 12), demande de changement de salle (PRF), « où est X » (VS, ACC), « demain » en lecture (ADJ). L'atelier de construction, les vœux détaillés et la publication restent sur le web.

**Remplace** dans `ui.rs` (F-001) et dans les modules Leptos actuels (`edt`, `edt_semaine`, `changements_edt`, `remplacements`) : voir §2 pour ce qui est gardé.

---

## 20. Contrat IA (socle 1 bis, `IA_COEUR.md` §5) [R]

| Capacité | Type | Droit | Sensibilité | Confirmation | Exemple |
|---|---|---|---|---|---|
| `edt.lire_mon_edt` | lecture | `edt.lire_eleve` / soi | faible | — | « Ai-je cours demain à 8 h ? » |
| `edt.lire_eleve` | lecture | `edt.lire_eleve` | moyenne | — | « Quel est l'EDT de Léa cette semaine ? » (famille) |
| `edt.salles_libres` | lecture | `edt.lire` | faible | — | « Trouve une salle informatique libre jeudi M3 » |
| `edt.creneau_commun` | lecture | `edt.lire` | faible | — | « Quand l'équipe de 4B est-elle libre ? » |
| `edt.expliquer` | lecture | `edt.construire` ou soi (vœux) | moyenne | — | « Pourquoi mon vœu du lundi n'est-il pas satisfait ? » |
| `edt.diagnostic` | simulation | `edt.construire` | faible | — | « L'EDT est-il faisable avec 2 groupes de TP ? » |
| `edt.proposer_deplacement` | simulation | `edt.modifier_seance` | faible | — | « Propose trois créneaux pour rattraper le cours annulé » |
| `edt.annuler_seance` | commande | `edt.modifier_seance` | moyenne | **confirmation humaine** (aperçu : public, notifications) | — |
| `edt.deplacer_seance` / `changer_lieu` | commande | `edt.modifier_seance` | moyenne | confirmation | — |
| `edt.annuler_masse` | commande | `edt.annuler_masse` | forte | **confirmation forte** ★ | — |
| `edt.publier` | commande | `edt.publier` | forte | **non exposée à l'IA en première version** [R] | — |
| `edt.resumer_scenarios` | lecture | `edt.construire` | faible | — | « Résume les différences entre A et B » |

L'IA ne lit **jamais** les vœux d'un autre professeur pour un professeur, ni la localisation d'un élève (`edt.localiser` non exposée à l'IA en première version [R]). Jeu d'évaluation du socle : 30 questions types (familles, élèves, professeurs, ADJ) avec réponses attendues calculées depuis les données de labo.

---

## 21. Modèle de données cible

Conventions des socles 5 et 6 : `etablissement_id NOT NULL` (différent de l'UUID nul), clés étrangères composées `(id, etablissement_id)`, RLS (permissive puis stricte), colonnes d'auteur et de date, aucune suppression physique d'un objet qui a servi, tables d'historique en ajout seul quand l'objet est sensible. Heures murales `TIME`, dates `DATE`, instants `TIMESTAMPTZ`.

### 21.1 Le temps du cours (tables destinées au noyau, Q-B15) [R]

```
ent_edt_seance (existante 110, enrichie)                -- LA séance datée
  id, etablissement_id, annee_id                          -- inchangés
  cours_id UUID NOT NULL (après reprise)                  -- socle 5 ; remplace classe/classe_id/service_id/matiere_*
  placement_id UUID NULL                                  -- placement d'origine (NULL pour une séance ajoutée)
  version_id UUID NULL                                    -- version d'EDT d'origine
  jour DATE, heure_debut TIME, heure_fin TIME             -- heures murales (fuseau du site)
  debut_utc TIMESTAMPTZ, fin_utc TIMESTAMPTZ              -- dérivés (notifications, iCal)
  creneau_code TEXT NULL                                  -- code de grille (« M2 »), lu pour l'affichage et la coupure
  demi_journee TEXT                                       -- dérivé de la coupure de la grille (socle 5), plus de 12 h 30 en dur
  etat TEXT CHECK (etat IN ('prevue','modifiee','annulee','deplacee','neutralisee','non_assuree','retiree','ajoutee'))
  motif_public_code TEXT NULL, motif_public TEXT NULL     -- jamais de motif interne ici
  origine TEXT CHECK (origine IN ('plan','ajout','deplacement','fusion','import'))
  remplacee_par_id UUID NULL                              -- séance d'arrivée (déplacement, nouvelle version)
  calcule_le TIMESTAMPTZ, version_calcul INTEGER          -- pour le test « recalculer ne change rien »
  -- projections conservées jusqu'au lot T11 : classe, classe_id, service_id, prof_user_id, matiere_*, salle, statut, remplacant_user_id
  UNIQUE (etablissement_id, placement_id, jour) WHERE placement_id IS NOT NULL   -- identité stable
  -- l'ancienne UNIQUE (creneau_id, jour) reste tant que creneau_id existe

ent_edt_seance_lieu                                      -- une séance peut occuper plusieurs lieux
  etablissement_id, seance_id, lieu_id (ent_salle, 337), principal BOOLEAN

ent_edt_seance_intervenant                               -- intervenants FIGÉS seulement quand la séance s'écarte du socle 6
  etablissement_id, seance_id, agent_id, nature ('remplacant','ajoute','retire'), saisie BOOLEAN, origine_ref (absence du socle 15)
  -- sinon : intervenants = socle 6 à la date (R-E4), calcul et non copie

ent_edt_seance_public_fusion                             -- séance fusionnée : cours d'origine réunis
  etablissement_id, seance_id, cours_id
```

### 21.2 Le plan (brique EDT) [R]

```
ent_edt_besoin                                           -- besoin horaire d'un cours pour une version
  id, etablissement_id, annee_id, cours_id, heures_hebdo NUMERIC(4,2), source ('cours','ajuste')
ent_edt_piece                                            -- découpage en pièces
  id, etablissement_id, besoin_id, duree_creneaux SMALLINT, semaine_type TEXT NULL ('toutes' ou étiquette du socle 5),
  consecutif BOOLEAN, type_lieu TEXT NULL, equipements TEXT[], lieu_impose_id UUID NULL, ordre SMALLINT

ent_edt_version                                          -- un emploi du temps type
  id, etablissement_id, annee_id, libelle
  etat TEXT CHECK (etat IN ('brouillon','publiee','remplacee','archivee','abandonnee'))
  valable_du DATE NULL, valable_au DATE NULL             -- posés à la publication
  diffusion_familles_le TIMESTAMPTZ NULL, provisoire BOOLEAN
  scenario_id UUID NULL                                  -- scénario d'origine
  publiee_par, publiee_le, empreinte_controles TEXT       -- contrôle du §14.1, pour l'idempotence
  EXCLUDE (etablissement_id, annee_id, daterange(valable_du, valable_au)) WHERE etat = 'publiee'

ent_edt_placement                                        -- successeur du créneau ; ent_edt_creneau devient une vue de compatibilité
  id, etablissement_id, version_id (ou scenario_id), piece_id, cours_id
  jour_semaine SMALLINT, creneau_debut_code TEXT, duree_creneaux SMALLINT  -- en codes de grille
  semaine_type TEXT                                      -- 'toutes' ou étiquette
  verrouille BOOLEAN, origine ('generation','main','import','reprise'), explication JSONB NULL
ent_edt_placement_lieu  placement_id, lieu_id, principal
  -- conflits du plan : contrôlés par le service (et par une contrainte d'exclusion par lieu pour les versions publiées)

ent_edt_scenario
  id, etablissement_id, annee_id, libelle, base_version_id NULL
  etat ('en_calcul','complet','incomplet','arrete','abandonne','retenu'), graine BIGINT, moteur_version TEXT,
  empreinte_entrees TEXT, reglages JSONB (trois niveaux par famille, plages interdites), indicateurs JSONB,
  tache_id UUID NULL, cree_par, cree_le
ent_edt_scenario_mouvement                               -- retouches (annuler / rétablir), en ajout seul
  id, scenario_id, ordre, nature ('deplacer','echanger','verrouiller','deverrouiller','placer','retirer'), avant JSONB, apres JSONB, par, le
ent_edt_non_place                                        -- pièces non placées et leur explication
  scenario_id, piece_id, explication JSONB

ent_edt_contrainte_etab                                  -- remplace la partie « établissement » de 083
  id, etablissement_id, annee_id, nature ('plage_interdite','pause_minimale','amplitude_max','trous_max','reunion_reguliere',
       'deplacement_sites','verrou_lieu'), cible (site, niveau, classe, groupe, lieu, couple de sites), parametres JSONB, force ('dure','souple'), niveau ('faible','normal','fort')

ent_edt_voeu_campagne
  id, etablissement_id, annee_id, ouverte_le, close_le, consignes TEXT, plafond SMALLINT, etat
ent_edt_voeu
  id, etablissement_id, campagne_id, agent_id, nature ('indisponibilite','demi_journee_libre','pas_premiere_heure',
       'pas_derniere_heure','regrouper_jours','trous_max','pause','salle_preferee','autre'),
  jour_semaine NULL, creneau_code NULL, valeur JSONB, priorite SMALLINT, motif_code NULL ('service_partage','temps_partiel',
  'decharge','formation','autre'), commentaire TEXT NULL (≤ 300), arbitrage ('en_attente','accepte_dur','souple','refuse'),
  arbitre_par, reponse TEXT NULL, satisfaction ('oui','partiel','non') NULL, raison JSONB NULL
  -- aucun champ de santé ; commentaire visible ADJ/DIR seulement
```

### 21.3 Le réel (brique EDT) [R]

```
ent_edt_modification                                     -- remplace ent_edt_exception (042, 171) et ent_edt_deplacement (170)
  id, etablissement_id, seance_id, lot_id NULL
  type TEXT CHECK (type IN ('annulation','deplacement','changement_lieu','changement_horaire','intervenant','ajout',
       'fusion','neutralisation','distance','correction'))
  avant JSONB, apres JSONB                               -- valeurs changées (heures, jour, lieux, intervenants, cours)
  motif_public_code TEXT NOT NULL, motif_public TEXT NULL (≤ 200)
  motif_interne TEXT NULL                                -- colonne lue seulement par ADJ/DIR (vue dédiée + RLS de colonne par vue)
  origine ('saisie','absence','fermeture','sortie','examen','calendrier','demande','import','lot')
  origine_ref UUID NULL, force_motif TEXT NULL (forçage d'un conflit)
  cle TEXT NULL UNIQUE par établissement                 -- idempotence
  levee_le, levee_par, cree_par, cree_le
ent_edt_lot                                              -- reprend ent_edt_annulation_masse (171)
  id, etablissement_id, cle, nature, du, au, portee ('etablissement','site','niveau','classes','groupes','intervenant','lieu'),
  cibles UUID[], heure_debut NULL, heure_fin NULL, motif_public_code, motif_public, apercu JSONB, cree_par, cree_le, leve_le
ent_edt_demande
  id, etablissement_id, demandeur_agent_id, nature ('deplacement','echange','lieu','rattrapage'), seance_id, contrepartie_seance_id NULL,
  proposition JSONB, controles JSONB, etat ('ouverte','acceptee_collegue','acceptee','refusee','annulee','expiree'), traite_par, reponse, le
ent_edt_localisation_journal                             -- lectures « où est X maintenant », ajout seul
  id, etablissement_id, lecteur_user_id, eleve_id, usage ('remise','urgence','recherche','infirmerie'), le
```

### 21.4 Paramètres (table `ent_parametre` du socle 5, 332) [R]
`edt.heure_message_veille` (défaut 18:30) ; `edt.publication_deux_temps` (défaut oui, délai 2 jours ouvrés) ; `edt.auto_acceptation` (défaut : changements de salle sans conflit = oui, échanges = non) ; `edt.nom_intervenant_familles` (Q-EDT8) ; `edt.distance_active` (Q-EDT9). Cinq réglages, pas davantage.

### 21.5 Ce qui est repris ou retiré
| Existant | Devient | Quand |
|---|---|---|
| `ent_edt_creneau` (021…370) | reprise en **version n° 1 publiée** (« reprise ») : un besoin, une pièce, un placement par créneau ; `cours_id` par 341 (S4) ; heures rapprochées des codes de grille (écart → placement « hors grille » signalé) ; puis **vue de compatibilité** `ent_edt_creneau` en lecture pour les anciens lecteurs | T1 ; vue retirée en T11 |
| `ent_edt_seance` (110) | enrichie (21.1) ; `statut` → `etat` (projection) | T1–T2 |
| `ent_edt_exception` (042, 171), `ent_edt_deplacement` (170) | repris dans `ent_edt_modification` (motif → motif interne ; `motif_public` du lot quand il existe, sinon « autre ») | T3 ; tables gelées en lecture, retirées en T11 après archive |
| `ent_edt_annulation_masse` (171) | repris dans `ent_edt_lot` | T3 |
| `ent_edt_calendrier_ab` (048) | remplacé par `ent_semaine_type_regle` / `ent_semaine` du socle 5 (333) : reprise de `reference_lundi` en `alternance_continue` | T1 (après S3) |
| `ent_edt_contrainte` (083) | `prof_indispo` → indisponibilités du socle 6 (PRS-55) ou vœux ; `salle_indispo` → `ent_salle_indisponibilite` (337) ; `preference` → vœux | T6 |
| `ent_edt_import` (060) | journal repris par le moteur d'échanges du socle 9 (3990–3999 du socle 9 : retrait) | socle 9 X16 |
| `ent_absence_rh` (080), `ent_delegation` (172) | socle 15 (absences) et socle 1 (délégations) ; non touchés ici | — |

---

## 22. Plan de migration depuis l'existant (sans casser l'appel ni le cahier)

Ordre [R] ; chaque étape est additive, rejouable, testée « avant / après » sur la base de labo.

| Étape | Contenu | Garde-fou |
|---|---|---|
| **M0** | Correctifs immédiats D1–D4, D8, D9, D12, D19 (lot T0) | tests de non-régression de l'appel et du cahier |
| **M1** | `cours_id` sur tous les créneaux (341, socle 5 S4) ; créneaux sans cours **signalés** (liste à traiter), jamais devinés | aucun créneau perdu ; compte avant/après |
| **M2** | Versions et placements : reprise des créneaux en version « reprise » publiée depuis le début de l'année ; vue de compatibilité `ent_edt_creneau` | les écrans et routes existants donnent le même résultat (instantanés) |
| **M3** | Séance enrichie (cours, placement, version, instants, état) ; identité stable ; matérialisation par fenêtre ; génération déplacée de `referentiel_seances.rs` vers la brique | « recalculer ne change rien » ; aucune séance porteuse supprimée |
| **M4** | Modifications : reprise des exceptions, déplacements, lots ; une seule commande ; ancien chemin retiré | toutes les exceptions passées retrouvées (compte, test) |
| **M5** | Semaines A/B du socle 5 ; grille du socle 5 (codes de créneau) | séances identiques avant/après sur l'année de labo |
| **M6** | Contraintes : reprise de 083 ; vœux | — |
| **M7** | RLS stricte ; retrait des projections (`classe` texte, `prof_user_id`, `salle` texte) après bascule de tous les lecteurs (appel, cahier, notes, vie scolaire) ; retrait des anciennes tables après durée d'archive | grep des lectures texte à zéro ; RLS testée sous rôle non propriétaire |

---

## 23. Lots de code (équipe Sonnet)

Chaque lot livre : migrations dans la plage (§23.2), dépôt, service en transaction unique, routes, types (`ent-types`), événements, tests Postgres (isolation entre établissements, matrice de droits, matrice D-022), écrans quand il en a, contrat IA quand il expose une capacité, documentation (ce document, `PROJECT_STATUS.md`). Revue `review-sonnet` sur chaque lot ; revue sécurité (skill `security-review`) sur T0, T2, T4, T9. Effort : **S** ≤ 3 j, **M** 1–2 sem., **L** > 2 sem. [H].

### 23.1 Tableau des lots

| Lot | Contenu | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|
| **T0 Correctifs urgents** | D1 (plus de suppression de séance porteuse, `RESTRICT`), D2 (portée D-022 sur toutes les lectures nominatives d'EDT : `/edt/semaine?cible=eleve`, `/eleves/{id}/edt`, `.ics`, changements), D3 (motif interne retiré des DTO et des routes ouvertes aux professeurs), D4 (RLS permissive sur les tables EDT, `ent_salle`), D8 (date du jour de l'établissement), D9 (professeur importé membre de l'établissement), D12 (portée du remplaçant au jour), D19 (collègue : présence oui/non) | — (sur la tête d'`ENT`) | tests rouges avant, verts après : régénération après ajout d'un jour férié → contenu de cahier conservé ; professeur d'allemand → 403 sur l'EDT d'un élève d'espagnol (4 routes) ; professeur → aucun motif interne dans aucune réponse ; requête sans contexte d'établissement → 0 ligne ; La Réunion à 23 h UTC → « aujourd'hui » local | M |
| **T1 Versions et placements** | M1, M2 ; `ent_edt_version`, `ent_edt_besoin`, `ent_edt_piece`, `ent_edt_placement(_lieu)` ; reprise ; vue de compatibilité ; commandes de création/modification de placement en **brouillon** ; co-enseignement (intervenants du socle 6) ; plusieurs lieux ; retrait de l'ancien chemin d'écriture (D6) | T0 ; socle 5 S4 (`ent_cours`, 341) et S3/S7 intégrés | instantanés identiques des anciennes routes ; un créneau sans cours est listé, jamais deviné ; un brouillon n'est lisible par aucune famille (test) | L |
| **T2 Séances** | M3, M5 ; séance enrichie ; identité stable ; matérialisation par fenêtre en tâche de fond ; réactions aux événements des socles 5 et 6 (calendrier, fermeture, grille, semaine, public, interventions) ; heures murales et instants ; états dérivés ; ports `seances`, `seance`, `seance_en_cours` ; appel ouvert **depuis la séance** (avec le socle 12 : D5) | T1 | « recalculer tout ne change rien » ; republier une version identique ne change aucun identifiant de séance ; fermeture déclarée → séances neutralisées, levée → rétablies, un seul message ; changement d'heure : heures murales stables ; cours déplacé le même jour → appelable | L |
| **T3 Modifications** | M4 ; `ent_edt_modification`, `ent_edt_lot` ; EDT-160 à 176 ; contrôles §9.3 avec forçage motivé ; levée ; lots avec aperçu ; grève du jour ; salle fermée en lot ; correction du passé ; reprise des exceptions, déplacements et lots existants | T2 | toutes les exceptions passées retrouvées ; deux clics = un lot ; lot de 2 000 séances < 2 s (labo) ; aucun motif interne hors ADJ/DIR | L |
| **T4 Notifications et « demain »** | événements §18.1 ; alias des anciens noms ; destinataires par public daté (inclusions) ; urgence selon délai ; message de la veille ; rétablissements ; tableau « demain » ; « maintenant » (`edt.localiser` journalisé) ; élèves sans cours | T3 ; socle 8 (moteur) | lot = 1 message par personne ; annulation de demain 8 h à 17 h → envoyée à l'heure du message de la veille ; levée → rétablissement ; « où est X » journalisé ; personne sous interdiction : rien | M |
| **T5 Publication** | EDT-150 à 156 ; contrôles bloquants et avertissements ; aperçu ; publication atomique en deux temps ; historique et différences ; retour arrière ; version individuelle | T2, T3 | publication pendant la journée → effet le lendemain ; aucune lecture mélangée ancienne/nouvelle (test concurrent) ; modifications ponctuelles conservées | M |
| **T6 Contraintes et vœux** | `ent_edt_contrainte_etab`, campagnes et vœux ; EDT-122 à 125, 140 à 146, 197 ; reprise de 083 ; indisponibilités du socle 6 | T1 ; socle 6 (PRS-55) | un professeur ne lit jamais les vœux d'un collègue ; aucun champ de santé (test de schéma) ; reconduction N → N+1 | M |
| **T7 Diagnostic** | EDT-120 ; §11.3 ; explications rattachées aux objets | T6 | jeux de labo « impossibles » (un professeur à 30 h, deux groupes alignés partageant un élève, TP sans salle) : chaque problème nommé, < 10 s | M |
| **T8a Prototype du moteur** | §11.4 : prétraitement, construction, amélioration, évaluation incrémentale ; mesures sur trois jeux de labo (collège 600, lycée 1 500, lycée professionnel avec co-intervention) ; **étude de licence** si un solveur tiers est envisagé ; déterminisme | T7 | bornes §11.4 point 5 mesurées et écrites ; même graine → même scénario (test) ; rapport : garder l'approche ou changer | L |
| **T8b Génération et scénarios** | EDT-126 à 135 ; tâche de fond interruptible ; reprise partielle ; explications ; tableau de qualité ; comparaison ; retouches avec annuler/rétablir ; affectation des salles | T8a | scénario complet sur les trois jeux ; reprise partielle : aucun placement verrouillé ne bouge ; explication de chaque pièce non placée | L |
| **T9 Abonnements iCal** | avec le socle 9 (X15) : jeton, minimisation, `UID` stable, `STATUS:CANCELLED`, `SEQUENCE`, `TZID`, pliage à 75 octets, relecture des droits ; retrait des anciennes routes `.ics` | T2 ; socle 9 X15 | client Apple/Google/Outlook abonné (test manuel consigné) ; enfant sorti → flux vide ; jeton révoqué → 404 | M |
| **T10 Échanges** | avec le socle 9 (X16) : export de structure, import en proposition, correspondances, différence à trois voies, application en nouvelle version, contrôle des heures ; reprise de 060 ; retrait de l'import CSV maison et du solveur glouton | T5 ; socle 9 X2, X16 | aller-retour aSc et FET (format seulement) sans perte sur jeux d'or ; réimport en cours d'année : modifications ponctuelles conservées | M |
| **T11 RLS stricte et retraits** | M7 | tous ; socles 12, 20, 21 basculés | RLS sous rôle non propriétaire ; grep des lectures `classe` texte et `prof_user_id` dans l'EDT à zéro | M |
| **T12 Écrans** | sous-lots : T12a « mon EDT » + EDT d'un élève/classe/professeur/salle (base `edt_semaine`) ; T12b modifications et lots ; T12c « demain » / « maintenant » ; T12d atelier de construction ; T12e vœux ; T12f publication ; T12g demandes ; mobile au socle 11 | lots back | rendu contre le vrai backend ; clavier ; 375 px ; axe-core ; parcours joué dans un navigateur | L (total) |
| **T13 Demandes des professeurs** | EDT-180 à 185 ; auto-acceptation réglable | T3 | échange accepté par les deux puis par l'ADJ ; refus sans effet | S |

**Ordre recommandé** : **T0** tout de suite (sécurité) → T1 → T2 → T3 → (T4, T5) → T6 → T7 → T8a → T8b ; T9, T10, T13 en parallèle après T5 ; T12 au fil de l'eau ; T11 en dernier. **Parcours minimal vendable** (« la journée de l'établissement ») : T0 à T5 + T9 + T12a–c. **Génération** (argument de juillet, D-025) : T6, T7, T8a, T8b, T12d–e.

### 23.2 Migrations (registre `MIGRATIONS.md`)
**Plage 4000–4099** si la règle « 3000 + 100 × socle » (Q-B2 d'`ARCHITECTURE_BRIQUES.md`) est adoptée ; **repli 440–459** sinon.

| N° (Q-B2) | Repli | Lot | Contenu |
|---|---|---|---|
| 4000 | 440 | T0 | correctifs : clé `RESTRICT` séance → contenu, RLS permissive des tables EDT et `ent_salle` |
| 4001 | 441 | T1 | versions, besoins, pièces |
| 4002 | 442 | T1 | placements et lieux ; reprise des créneaux ; vue de compatibilité |
| 4003 | 443 | T2 | séance enrichie (**table du noyau**, Q-B15 : à renuméroter dans 1200–1399 si Q-B2 l'y range, comme la 3910 du socle 9) |
| 4004 | 444 | T2 | lieux, intervenants, fusions de séance (**noyau**, même remarque) |
| 4005 | 445 | T3 | modifications et lots ; reprise de 042, 170, 171 |
| 4006 | 446 | T4 | journal de localisation |
| 4007 | 447 | T5 | publication (contrôles, diffusion) |
| 4008 | 448 | T6 | contraintes d'établissement ; reprise de 083 |
| 4009 | 449 | T6 | campagnes et vœux |
| 4010 | 450 | T8b | scénarios, mouvements, non placés |
| 4011 | 451 | T13 | demandes |
| 4012–4089 | 452–457 | — | réserve |
| 4090–4099 | 458–459 | T11 | RLS stricte ; retrait des projections et des anciennes tables (`ent_edt_exception`, `ent_edt_deplacement`, `ent_edt_annulation_masse`, `ent_edt_calendrier_ab`, `ent_edt_contrainte`) après la durée d'archive |

Ordre de rejeu : avec Q-B2, aucune brique de numéro inférieur à 4000 (socles 3, 7, 8, 9) ne référence la séance ; la règle « le noyau précède les briques » est respectée dès que 4003–4004 sont renumérotées dans la plage du noyau.

---

## 24. Tests et recette

**Tests automatiques exigés** (en plus de la matrice de séparation §15.4) :
1. Cas limites CL-01 à CL-63 : chacun a au moins un test (ceux qui dépendent d'un autre socle sont marqués et activés quand le socle existe).
2. Propriétés : « recalculer ne change rien » ; « republier la même version ne change aucun identifiant » ; « aucune séance porteuse supprimée » ; « aucune heure Europe/Paris en dur » (recherche) ; « aucune lecture de nom par le moteur » (le moteur reçoit une structure sans champ texte nominatif, test de type).
3. Déterminisme du moteur : trois exécutions, même graine, même empreinte de résultat.
4. Performance (labo) : vue semaine d'un élève < 150 ms ; « demain » < 1 s pour 1 500 élèves ; lot de 2 000 séances < 2 s ; diagnostic < 10 s ; bornes du moteur (§11.4).
5. Tests d'intégration **qui échouent** sans base en CI (D20).

**Recette par personas** (D-021, étape 4) [R] : adjoint qui construit l'EDT d'un collège de labo de bout en bout ; vie scolaire un matin de grève ; secrétariat un jour de neige ; professeur en service partagé ; famille de parents séparés avec interdiction ; élève d'ULIS ; établissement de La Réunion ; attaques : élève qui lit l'EDT d'un autre, professeur hors de ses groupes, jeton iCal volé puis révoqué, AED qui publie, professeur qui lit les vœux d'un collègue, autre établissement sur chaque route.

---

## 25. Réglages signalés « usine à gaz » (D-023)

Numérotation à la suite des passes précédentes (S-79 au socle 9). Signalés, pas supprimés.

| N° | Réglage | Pourquoi il est suspect | Proposition |
|---|---|---|---|
| S-80 | Auto-acceptation des demandes des professeurs (par type) | Utile, mais trois sous-réglages possibles | un seul interrupteur : « changements de salle sans conflit acceptés seuls » |
| S-81 | Poids numériques des contraintes souples | Les logiciels du marché en exposent des dizaines [H] ; personne ne sait les régler | trois niveaux par famille ; poids cachés |
| S-82 | Publication en deux temps et son délai | Peut être un réglage inutile si tous veulent le même comportement | garder le défaut (oui, 2 jours ouvrés), réglage caché après le pilote si personne ne le change |
| S-83 | Heure du message de la veille | Un seul paramètre, raisonnable | garder |
| S-84 | Nom de l'intervenant visible des familles | Choix d'établissement réel (Q-EDT8) | garder, une valeur |
| S-85 | Nombre de semaines types (2 à 4) | Rare au-delà de A/B | garder (déjà dans le socle 5) ; interface A/B par défaut |
| S-86 | Motifs publics paramétrables | Tentation d'une liste à rallonge | liste courte fixée par ScolIA + « autre » relu |
| S-87 | Durée de conservation des scénarios | Personne ne la changera | fixe (30 jours après publication) |
| S-88 | Plafond de vœux par professeur | Utile pour l'équité | garder, défaut 5 |
| S-89 | Cours à distance (Q-EDT9) | Fonction de crise, peu utilisée | désactivée par défaut |

---

## 26. Questions à Florian (avec recommandations)

| N° | Question | Recommandation [R] |
|---|---|---|
| **Q-EDT1** | La **génération automatique** (D-025) se construit-elle **dans le socle 10** (lots T6 à T8b) ou dans un socle dédié (N2, `SOCLES.md` §10, T-03, Q-F, Q-H) ? | **Dans le socle 10** : mêmes objets (cours, placements, versions), même atelier, même publication ; un socle N2 dupliquerait le modèle. Les lots de génération passent **après** le parcours minimal (T0–T5) et sont précédés d'un prototype mesuré (T8a). Ferme Q-F et la partie N2 de Q-H. |
| **Q-EDT2** | La **séance datée** (et ses lieux et intervenants) : tables du **noyau** (structure, Q-B15) ou de la brique EDT ? | **Tables au noyau, règles et commandes au socle 10** : l'appel, le cahier, les notes, les remplacements s'y rattachent ; une brique ne doit pas dépendre d'une autre. Numéros 4003–4004 réservés ici, **à renuméroter** dans la plage du noyau si Q-B2 l'y range (même traitement que la 3910 du socle 9). |
| **Q-EDT3** | La **semaine type** (A/B) appartient-elle au **placement** ou au cours (`ent_cours.semaines`, migration 341) ? | **Au placement** (un cours de 3 h peut faire 2 h en A, 1 h en B) ; la colonne du cours devient une valeur par défaut de la génération, sans effet sur les séances. À accorder avec le socle 5. |
| **Q-EDT4** | Lancer le **lot T0** (correctifs D1, D2, D3, D4, D8, D9, D12, D19) **immédiatement**, avant la passe personas et avant les autres lots du socle ? | **Oui**, sur la tête d'`ENT`, avec la recette sécurité n° 3 en cours (`ent-recette-securite-3`) prévenue pour ne pas corriger deux fois (D2 et D3 sont du même type que ses cibles). Perte de données (D1) et fuite (D2, D3) ne peuvent pas attendre la refonte. |
| **Q-EDT5** | **Moteur** : maison (construction + recherche locale) ou solveur tiers sous licence permissive ? | **Maison d'abord**, prototype T8a mesuré sur trois jeux de labo ; un solveur tiers seulement après étude de licence (MIT/Apache/BSD/MPL-2.0), d'activité, de déterminisme et de binding Rust, et **ta validation** (CLAUDE.md). Jamais de code AGPL (FET) ni de service en ligne. |
| **Q-EDT6** | **Équité pluriannuelle** des vœux (tenir compte de la satisfaction de l'an passé) ? | **Non en première version** : l'historique est gardé, pas utilisé ; à rediscuter après le premier pilote (risque de conflit social si la règle est mal comprise). |
| **Q-EDT7** | **Publication en deux temps** (personnels d'abord, familles deux jours ouvrés plus tard) par défaut ? | **Oui** ; diffusion immédiate possible au cas par cas. |
| **Q-EDT8** | **Nom de l'intervenant** visible des familles et des élèves ? | **Oui, civilité et nom d'usage**, comme le prévoit l'arrêté du 21/03/2018 pour le traitement ministériel (R-11) ; jamais l'absence ni son motif ; le nom du remplaçant est donné (socle 6 PRS-46). Réglage unique, à confirmer par le DPO. |
| **Q-EDT9** | Type de séance **« à distance »** (fermeture avec continuité pédagogique) dès la première version ? | **Prévu dans le modèle, désactivé par défaut** (S-89) ; activé si ton établissement en a eu l'usage. |
| **Q-EDT10** | Le **remplacement de courte durée** (décret 2023-732, créneaux de volontaires) s'applique-t-il dans ton établissement **privé sous contrat**, et comment s'y organise-t-il ? | Le texte n'a pas pu être confirmé pour le privé [NV]. Le modèle des **créneaux de disponibilité** sert dans les deux cas (volontariat interne) ; ta pratique tranche les écrans. |
| **Q-EDT11** | **Remontée hebdomadaire** du suivi du remplacement de courte durée (arrêté du 14/08/2023, R-09) : à construire avant l'ouverture au public (D-029) ? | **Oui** : obligation des logiciels de vie scolaire dans le public. Lot à créer au socle 9 (flux) avec les **catégories nationales de motif** au socle 15 et les agrégats d'heures au socle 10 (`heures_assurees`) ; après obtention de la spécification de l'API (démarches Pléiade, D-029). |
| **Q-EDT12** | Quel **logiciel d'EDT** utilise ton établissement aujourd'hui (Charlemagne Emploi du temps, Index EDT, UnDeuxTEMPS, autre), et peut-il fournir un **export anonymisé** (jeu d'or, D-14 du socle 9) ? | Indispensable avant le lot T10 ; la situation B (garder son logiciel, §13.1) est probablement celle du pilote. |
| **Q-EDT13** | **Changements de salle sans conflit** demandés par un professeur : acceptés automatiquement ? | **Oui** (S-80) ; les échanges de séances entre professeurs restent soumis à l'adjoint. |
| **Q-EDT14** | **L'accueil** peut-il savoir où est un élève **maintenant** (pour une remise ou une urgence) ? | **Oui**, par une lecture unitaire journalisée (`edt.localiser`), jamais une liste ; refus systématique pour un élève sous mesure de protection hors personnels habilités. |
| **Q-EDT15** | Un professeur voit-il l'EDT **non nominatif** d'une classe où il n'a qu'un groupe (tous les cours de la classe, sans liste d'élèves) ? | **Oui** : c'est une donnée d'organisation (devoirs, réunions, sorties) ; D-022 s'applique à l'EDT **nominatif** d'un élève (ses groupes, ses inclusions), qui reste limité aux élèves de ses cours. À confirmer, car D-022 dit « toute donnée élève ». |
| **Q-EDT16** | Heure par défaut du **message de la veille** (changements du lendemain) ? | **18 h 30**, rappel le matin à 7 h pour les changements de la première heure. |

---

## 27. Faits, hypothèses, contradictions

**Faits [F]** : tables, fonctions, routes et lignes citées au §2 relues sur `ent-integration-lot11` (`1bd6fe4`) et sur `ent-socle-structure-s3`, `-s4`, `-s7` le 2026-11-09. Les défauts D1, D2, D3 relus directement ; les autres viennent de l'audit indépendant (lecture seule, rien exécuté) et sont à reconfirmer par le lot qui les corrige. Rien n'a été compilé ni testé pour ce document.

**Vérifié [V]** : R-01 à R-05, R-07, R-09, R-11 à R-14, R-16, R-17, R-20 (URL au §5).

**Hypothèses [H]** : pratiques du terrain (§3, §4), comparaison avec les logiciels du marché, bornes de performance du moteur (§11.4), effort des lots (§23).

**Contradictions signalées**
- **C-1** `SOCLES.md` §3 (socle 10) dit « ne contient pas : la fabrication automatique de l'emploi du temps (solveur) » ; D-025 la met dans le périmètre « socle 10 ou socle dédié ». Ce document l'inclut (Q-EDT1) ; `SOCLES.md` est mis à jour en conséquence (proposition).
- **C-2** Le socle 5 a changé d'avis en cours de route : le lot S1 (`331_cours_membres.sql`, en-tête) fait du **cours** la ligne de `ent_service_enseignement` ; le lot S4 (`341_cours.sql`, branche non intégrée) crée une table `ent_cours` distincte, dont le service est la présence d'une personne. Ce document suit **S4** (le plus récent, conforme à `05_STRUCTURE.md` §9.7) ; si S4 n'était pas intégré, le contrat `cours_id` du créneau serait à revoir.
- **C-3** `09_INTEROP.md` FX-82 écrit « licence contradictoire GPL-2.0 / AGPL-3.0 » pour FET ; la page de l'auteur indique **AGPL-3.0** [V] (R-20). Conséquence identique : aucun code repris.
- **C-4** `ent_cours.semaines` (341) porte la semaine type au niveau du cours ; ce document la porte au placement (Q-EDT3).
- **C-5** Noms : `08_NOTIFICATIONS.md` §5 nomme les **avis** `edt.cours_annule`, `edt.cours_deplace`, `edt.salle_changee` ; ce document nomme les **événements** `edt.seance_annulee`, `edt.seance_modifiee` (§18). Les deux coexistent : le socle 8 traduit l'événement en avis ; la table de correspondance est à écrire au lot T4.
- **C-6** `SOCLES.md` range les remplacements au socle 15, mais le code les a mis dans l'EDT (`edt_remplacement.rs`, `ent_absence_rh`) : ce document garde au socle 10 la seule **modification de séance** et renvoie l'absence, la décision et la recherche du remplaçant au socle 15 (§6).
- **C-7** Migrations 170 et 370 ajoutent la même colonne `ent_edt_creneau.groupe_id` et le même index (inoffensif grâce à `IF NOT EXISTS`, mais trompeur dans le registre).
- **C-8** Q-B15 (`ARCHITECTURE_BRIQUES.md`) met la séance au noyau ; `05_STRUCTURE.md` (Q-S11) envoie sa génération au socle 10. Les deux sont compatibles : tables au noyau, génération et commandes au socle 10 (Q-EDT2).
- **C-9** `SOCLE2_DOMAINES.md` §D4 et `MATRICE_CHARLEMAGNE.md` §3.6 utilisent tous deux des identifiants `EDT-01`… avec des sens différents ; les actions de ce socle commencent à EDT-100 pour n'en écraser aucun.

---

## 28. Les cinq décisions à prendre en priorité

| # | Décision | Pourquoi maintenant | Recommandation | Réf. |
|---|---|---|---|---|
| **1** | **Lancer le lot T0 tout de suite** (cahier de textes qui s'efface, EDT nominatif ouvert à tout professeur, motifs d'absence visibles, absence de RLS, date du jour en UTC) | Perte de données et fuite **dans le code déjà intégré** ; chaque jour d'usage en labo ou en démonstration les expose | Oui, sur la tête d'`ENT`, coordonné avec la recette sécurité n° 3 | Q-EDT4, §2.2, §23 |
| **2** | **Générer l'emploi du temps dans le socle 10**, avec un moteur maison en Rust, déterministe et explicable, après un prototype mesuré | Ferme Q-F, T-03 et N2 ; fixe l'ordre des lots ; évite de dupliquer le modèle | Oui ; prototype T8a avant d'engager T8b ; aucun code AGPL, aucun solveur tiers sans ta validation | Q-EDT1, Q-EDT5, §11 |
| **3** | **Adopter le modèle plan versionné / séance stable / modification**, avec la séance au noyau | Conditionne T1 à T5, l'appel (socle 12), le cahier (20), les notes (21) et les remplacements (15) | Oui ; semaine type au placement ; tables de séance au noyau (renumérotées si Q-B2) | Q-EDT2, Q-EDT3, §8, §21 |
| **4** | **Préparer la remontée hebdomadaire du remplacement de courte durée** (arrêté du 14/08/2023) et le **motif catégorisé** au socle 15 | Obligation des logiciels de vie scolaire du public (D-029 : ouverture avant 2028) ; le motif doit être catégorisé dès la conception du socle 15 | Oui, lot à créer au socle 9 après la spécification ; dire si le RCD vit dans ton établissement privé | Q-EDT11, Q-EDT10, R-09 |
| **5** | **Fixer ce que voient familles et professeurs** : nom des intervenants oui, motif jamais ; EDT nominatif d'un élève limité aux élèves de ses cours, EDT de classe non nominatif ouvert | Conditionne T0 (D2), les DTO et la matrice de séparation | Oui aux trois recommandations | Q-EDT8, Q-EDT15, R-11, D-022 |
