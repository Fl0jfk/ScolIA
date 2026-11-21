# Socle 12 — Appel et assiduité

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut selon D-031 et listées au §27. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; l'application Next.js a été lue en lecture seule et ciblée (D-035), sans aucune modification.
**Branche :** `ent-socle-12-conception` (depuis `origin/ent-mc0hoe`, `7467f2d`). Code évalué sur cette même base.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (D-017, D-019, **D-020**, D-021, **D-022**, D-023, D-024, D-026, D-028, D-031, **D-034**, D-035, **D-036**, D-037 à D-041) ; `docs/SOCLES.md` §3 (socles 10 à 15) ; `docs/VERIF_REGLEMENTAIRE.md` (R1, R2, R3, R11, R27, R41) ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; `docs/TRANSITION_T0_INVENTAIRE.md` (C07, M17) ; `docs/MATRICE_CHARLEMAGNE.md` §3.4 (VS-01 à VS-15) et §5 (G19–G23, G27) ; `docs/SOCLE2_DOMAINES.md` (VS-01 à VS-25, A1, A2, A5) ; conceptions `05_STRUCTURE.md` (STR-35, coupure des demi-journées), `08_NOTIFICATIONS.md` §4.1 (natures `vie_scolaire.*`, U2, escalade), `10_EMPLOI_DU_TEMPS.md` §8.3 (séance datée, attente d'appel), `16_MESSAGERIE.md` (message téléphonique, carte « absence signalée »), `21_EVALUATIONS.md` (absent à une évaluation), `34_SANTE.md` (inaptitude, infirmerie, N-0), `42_SORTIES_VOYAGES.md` §15.2 (appel « en sortie »), `IA_COEUR.md` §5 et §7.2 ; et la passe personas `docs/socles/PERSONAS_12.md`.
**Conventions.** **[F]** fait vérifié dans le dépôt (chemin et ligne) ; **[V]** règle lue sur le texte (copie des données ouvertes de Légifrance, état au 2026-10-31, ou page officielle) ; **[V-ext]** lue sur une copie tierce ou un extrait ; **[NV]** non vérifiée ; **[H]** hypothèse (terrain, marché) ; **[R]** recommandation. Actions **AS-100** et suivantes (VS-01 à VS-25 de `SOCLE2_DOMAINES.md` et de la matrice restent valables et sont mises en correspondance au §17) ; règles **R-AS1** et suivantes ; cas limites **CA-01** et suivants ; règles réglementaires **RA-01** et suivantes ; défauts de l'existant **DA-01** et suivants ; questions **Q-AS1** et suivantes ; réglages signalés **S-467** et suivants (après S-466) ; manques de la passe personas **M-2592** et suivants (après M-2591).
**Plage de migrations imposée :** **2240–2259** (registre `MIGRATIONS.md`).
**Méthode.** Lecture des décisions, de la carte, des conceptions voisines et des attentes qu'elles adressent au socle 12 (74 mentions relevées) ; audit indépendant du code (sous-agent de revue, lecture seule), dont les défauts graves DA-01, DA-02 et DA-05 ont été relus ligne à ligne ; inventaire en lecture seule du module Next.js de vie scolaire (sous-agent) ; recherche réglementaire sur les textes (sous-agent, copies des données ouvertes de Légifrance ; Légifrance, le BO et éduscol répondent 403 aux robots) ; recherche de parité sur les logiciels du marché (sous-agent ; plaquette Pronote 2023 lue, autres éditeurs partiellement inaccessibles).

Sommaire : §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs · §4 La journée et l'année · §5 Réglementaire · §6 Frontières · §7 Vocabulaire · §8 Modèle · §9 L'appel · §10 Absences, motifs, justificatifs · §11 Retards, passages, localisation · §12 Dispenses · §13 Décompte, suivi, signalement · §14 Familles et notifications · §15 Droits et minimisation · §16 Cas limites · §17 Actions · §18 Règles · §19 Événements · §20 Écrans · §21 Contrat IA · §22 Modèle de données · §23 Migration depuis l'existant · §24 Lots · §25 Tests et recette · §26 Réglages signalés · §27 Décisions appliquées et points pour Florian · §28 Faits, hypothèses, contradictions · §29 Ajouts de la passe personas.

---

## 0. En bref

1. **L'existant est le plus avancé de l'ENT, et il est dangereux en l'état.** Appel par créneau, clôture atomique, demi-journées dérivées d'une fonction pure bien testée, déclarations accueil et famille rapprochées de l'appel, vérification des appels non faits, signalement mensuel proposé par le CPE et validé par la direction : de bons réflexes. Mais **un professeur peut déclarer ou effacer l'absence de n'importe quel élève de l'établissement** (DA-01, DA-02) et lui poser un retard (DA-03), contre D-022 ; **chaque séance manquée crée une absence distincte** (DA-05) : une journée d'absence vaut six notifications aux parents, six justifications et six décisions du CPE, et franchit en deux jours un **second seuil codé en dur (10 absences par an, justifiées comprises)** qui écrit aux familles (DA-07) ; un appel clos ne se corrige pas (DA-06) ; la RLS est posée mais inopérante (DA-04). Un **lot AS0** de correctifs passe avant tout le reste (§2.2, §24).
2. **Trois objets au lieu d'un.** (a) le **constat** : l'appel d'une séance datée (socle 10), ligne par ligne, avec son historique ; (b) la **situation connue** : tout ce qui dit à l'avance qu'un élève ne sera pas en cours ou y sera autrement (déclaration de la famille ou de l'accueil, sortie, stage, examen, dispense, infirmerie, exclusion-inclusion, mobilité) — elle **pré-remplit** l'appel et n'est jamais une absence en soi ; (c) l'**absence** : un épisode continu de séances manquées par un élève, unité unique de motif, de justificatif, de décision et de notification. La **demi-journée** en est dérivée, de façon déterministe et versionnée (§1, §8).
3. **Le réglementaire est vérifié et change la conception** (§5) : toute absence est **immédiatement** signalée aux responsables (R131-5 [V]) ; un **dossier d'absences par année scolaire** est obligatoire (R131-6 [V]) ; **cinq motifs seulement** sont « réputés légitimes » (L131-8 [V]), les autres sont appréciés ; le seuil de **quatre demi-journées dans le mois** sans motif légitime ni excuse valable déclenche commission éducative, document signé avec la famille et saisine du DASEN (R131-7 [V]) ; la **demi-journée n'est définie par aucun texte** ; le certificat médical n'est exigible que pour les maladies contagieuses (circulaire 2014-159 [V-ext], qui **abroge** celle de 2011) ; l'inaptitude d'EPS n'emporte **pas** dispense de présence (R312-2 [V]) ; la bourse est retenue au-delà de **quinze jours** d'absence injustifiée (R531-31, D531-12 [V]) ; la procédure pénale ne vise que les moins de 16 ans (R624-7 [V]).
4. **L'appel, partout** : web, mobile, **hors ligne réel** (gymnase, sortie, salle sans réseau) avec journal d'opérations idempotent ; pré-rempli ; pour toute activité organisée pendant le temps scolaire (cours, permanence, étude, CDI, sortie, atelier, demi-pension sur choix de l'établissement) car R131-5 l'impose à « tout personnel responsable d'une activité » [V] ; corrigeable après clôture avec motif et historique, jamais réécrit en silence (§9).
5. **Moins de téléphone pour la vie scolaire** : l'accueil saisit l'appel d'un parent en trois gestes (exemple vécu D-023, repris de Next.js) ; la famille déclare et justifie en ligne avec une pièce facultative ; un avis par demi-journée et par foyer, jamais par séance ; une **file du matin** classée par urgence (mineur sans nouvelles, élève fragile, absence après présence) remplace les listes papier ; chaque contact avec la famille est tracé une fois et visible de toute l'équipe (§14).
6. **Le décompte est transparent et ne réécrit pas le passé** : règle d'assiduité **versionnée et datée** (coupure, définition de la demi-journée, seuils), chaque demi-journée expose son calcul, les décomptes d'un mois signalé sont **figés** ; le dossier de suivi et de signalement est **pré-rempli** (chronologie, contacts, mesures), exporté selon le profil de l'académie, **jamais transmis automatiquement** (§13).
7. **Ce que chacun voit est minimal** : le professeur voit les élèves de ses cours (D-022), l'état d'une absence (« à régulariser », « justifiée ») mais **jamais un motif** ; la famille voit tout ce qui concerne son enfant, sans le texte écrit par l'autre parent ; l'infirmerie est une **localisation** sans motif ; aucun motif de santé n'est visible hors de la vie scolaire et n'est envoyé à l'IA (§15).
8. **Chiffres** : **152 actions** (AS-100 à AS-199, puis AS-200 à AS-251 ajoutées par la passe personas), **72 cas limites**, **40 règles**, **17 lots** (AS0 à AS16) ; plage de migrations 2240–2259 ; **parcours de la démonstration de décembre** (D-034) : AS0, AS1, AS2, AS3, AS5 et les écrans AS14a–c (§24.2).
9. **Pour Florian** (§27.2) : deux **failles probables dans l'application Next.js de production** relevées en lecture seule (un professeur pourrait lire et modifier les absences de tout l'établissement ; l'appel mobile ne contrôle pas le module) — **non touchées**, à vérifier par lui ; la politique d'information des parents d'un élève majeur (DPO) ; les profils académiques de signalement de son académie ; le SMS d'absence (Q-N5, coût) ; l'absence de données réelles avant l'accord écrit.
10. **(ajout personas)** La passe personas (`PERSONAS_12.md`, 16 personas dont des attaquants et un DPO, 72 manques M-2592 à M-2663) trouve **quatre bloquants**, tranchés par défaut (D-031) : (A) l'**autre parent** d'une résidence alternée n'était jamais informé → récapitulatif quotidien (R-AS33) ; (B) l'**élève exclu de cours** pouvait disparaître entre la salle et la vie scolaire → suivi de son arrivée (R-AS37) ; (C) le **cache hors ligne** d'un téléphone perdu → déverrouillage obligatoire et effacement à distance (R-AS38) ; (D) l'**année de doublon** → import quotidien de Charlemagne (AS-232), décision de Florian (P-5). Ajouts au §29.

---

## 1. Thèse : l'assiduité est un constat, une attente et une décision, pas une case cochée

Pour la famille, l'assiduité répond à une question : **mon enfant est-il là où il doit être ?** Pour le professeur : **qui manque à mon cours, et ai-je fait ce que je dois ?** Pour la vie scolaire : **qui manque sans qu'on sache pourquoi, et qui faut-il appeler maintenant ?** Pour la direction : **l'établissement remplit-il son obligation, et quels élèves décrochent ?** Pour la loi : un registre d'appel tenu, une information immédiate des responsables, un dialogue suivi, une procédure graduée.

Le code existant a été construit autour d'une ligne `ent_absence` (une période en jours) puis enrichi d'un détail par séance (`ent_absence_seance`, D-020). Ce socle part de là et pose douze principes [R] :

1. **Trois objets distincts.** Le **constat** (ce que l'appel d'une séance a relevé), la **situation connue** (ce qu'on savait avant ou ce qu'on apprend d'un autre service : sortie, stage, dispense, infirmerie, déclaration), l'**absence** (l'épisode à motiver et à décider). Aujourd'hui les trois sont mêlés : une famille crée directement un « fait » d'absence de plusieurs jours (DA-14), l'appel crée une absence par séance (DA-05).
2. **L'appel vise une séance datée, jamais un texte.** La séance vient du socle 10 (identité stable, état, public à la date) ; l'effectif vient du cours du socle 5 à la date (groupes, inclusions, arrivées et départs) ; l'intervenant vient du socle 6 (titulaire, remplaçant, co-intervenant, AESH présente mais jamais appelante). Le champ `classe` texte et le créneau récurrent cessent d'être la référence.
3. **Une absence est un épisode, pas une séance.** Les séances manquées consécutives d'un même élève, le même jour, forment un épisode ; une déclaration de la famille sur plusieurs jours en forme un seul. Un épisode a **un** motif déclaré, **un** ou plusieurs justificatifs, **une** décision, **un** avis par demi-journée. Chaque séance garde son propre constat.
4. **Le motif déclaré n'est pas la qualification.** La famille **fait connaître** un motif (L131-8) ; l'établissement **apprécie** : motif réputé légitime (les cinq de la loi), excuse valable (autre motif accepté), non valable, sans motif. Le code a aujourd'hui « justifiée / refusée », qui confond les deux.
5. **Ce qui n'est pas une absence n'est jamais enregistré comme telle.** Sortie scolaire, stage, examen, mission de délégué (« ne doit pas être traitée comme une absence », circulaire 2014-159 [V-ext]), entretien avec la CPE, infirmerie, exclusion-inclusion : ce sont des **situations** qui pré-remplissent l'appel avec une **localisation**. Elles ne comptent dans aucun décompte et n'envoient aucun avis d'absence.
6. **L'appel se corrige, il ne se réécrit pas.** Toute modification après validation est une **correction** datée, motivée, attribuée, visible dans l'historique de la ligne ; les effets (absence, avis, décompte) suivent automatiquement, y compris le **démenti** à la famille si l'avis est déjà parti.
7. **La demi-journée est dérivée, versionnée, expliquée.** Aucun texte ne la définit [V] : la règle est un paramètre de l'établissement, inscrit au règlement intérieur, **daté** ; changer la règle ne réécrit pas un mois déjà arrêté ou signalé ; chaque décompte affiche « pourquoi cette demi-journée compte ».
8. **L'information de la famille est due, le canal se choisit.** R131-5 impose l'information immédiate ; le droit d'opposition ne joue pas contre une obligation légale (VERIF R3) ; on peut refuser un **canal** (SMS), jamais l'information. Un avis par demi-journée et par foyer, après un court délai de consolidation qui évite les fausses alertes, avec escalade et démenti.
9. **Un seul guichet pour la vie scolaire le matin.** Les déclarations de l'accueil, de la famille, de la messagerie, de l'infirmerie, des sorties et de l'appel arrivent au même endroit, rapprochées automatiquement ; la vie scolaire ne traite que ce qui reste inexpliqué, classé par urgence.
10. **Le suivi est un dossier annuel, pas une alerte.** R131-6 impose un dossier d'absences par année scolaire ; le seuil de quatre demi-journées déclenche une **procédure** (dialogue, commission éducative, document signé, saisine), que ScolIA prépare, trace et date ; la décision et l'envoi restent humains.
11. **Le moindre privilège est la règle.** Le professeur voit ses élèves (D-022), pas les motifs ; l'accueil déclare, ne lit pas l'historique ; l'AED traite la file, la CPE qualifie ; la direction valide les signalements ; la famille voit son enfant, pas l'écrit de l'autre parent ; aucune donnée de santé ne sort de la vie scolaire sous forme de motif.
12. **L'IA aide autour, ne décide jamais.** Dictée de l'appel, extraction d'une déclaration depuis un message ou un appel téléphonique, résumé de la file du matin, brouillon de courrier : toujours relus et confirmés par une personne ; jamais une qualification de motif, un signalement ou un avis envoyé par l'IA ; jamais un motif de santé dans son contexte.

---

## 2. Évaluation franche de l'existant

### 2.1 Ce qu'il y a [F]

**Tables** (migrations 003, 004, 009–013, 021, 035, 043, 067, 069, 087, 130–134, 563, 581) : `ent_absence` (période `debut`/`fin` en dates, `motif` texte sans clé, `statut` `a_traiter|justifiee|refusee|annulee_present`, décision écrasée sur place) ; `ent_absence_seance` (130 : ce qui a été manqué, séance ou demi-journée de reprise, horaires figés, `source`, `prevenue`) ; `ent_vs_regle_assiduite` (130 : coupure 12 h, seuil 30 min, signalement 4, délai de vérification 15 min, **non versionnée**) ; `ent_appel` (009 : `classe` texte, `creneau_id` récurrent, `statut ouvert|clos`, pas d'auteur de clôture) et `ent_appel_ligne` (`present|absent|retard`, sans historique) ; `ent_retard` (087 : jour, minutes, motif libre, sans séance) ; `ent_appel_relance` (132) ; `ent_absence_declaration` et `ent_absence_rapprochement` (131) ; `ent_absence_signal` (012, 035, 043 : signal **annuel**) ; `ent_signalement_assiduite` (134 : signalement **mensuel**, `dossier JSONB` en lecture seule) et son détail typé (563, conforme D-036) ; `ent_dispense_vs` (067) ; `ent_alerte_cpe` (069) ; motifs : `ent_absence_motif` (003, **global**, 5 codes dont `appel`, technique) et `ent_absence_motif_etab` (013). Aucun `SERIAL`, aucun `BYTEA` ; `etablissement_id NOT NULL` partout ; aucune clé étrangère composite ; RLS posée par 581.

**Code** : `ent-db/src/assiduite.rs` (fonctions pures `deriver_demi_journees`, `seance_couverte`, `bornes_mois`, `heure_paris` ; 15 tests) ; `ent-db/src/vie_scolaire.rs` (règle, séances du jour, déclarations, rapprochement dans les deux sens, contributions, demi-journées, décompte mensuel, vérification et incohérences d'appel, relance, tableau du jour) ; `ent-db/src/service.rs` (`ouvrir_appel` :8105, `marquer_appel_ligne` :8257, `clore_appel` :8290, déclarations et décisions d'absence :5049–5300 et :8734–8853, retards :8513, seuil annuel `apres_absence_ecrite` :8583, signaux :8892–9100, dispenses :13067) ; `ent-db/src/discipline_vs.rs` (`proposer_signalements` :943, `decider_signalement` :1052) ; `ent-db/src/perimetre.rs` (périmètre du professeur, majorité) ; réactions de notification (`reactions.rs:76-205`).

**Routes** : `/appels` (ouvrir, lire, marquer, clore, attendus absents), `/eleves/{id}/absences`, `/absences/{id}/justifier|accepter-justificatif|refuser-justificatif|annuler-present`, `/vs/absences`, `/vs/tableau-jour`, `/vs/verification-appels`, `/vs/appels/relancer`, `/vs/declarations/accueil|famille`, `/vs/decompte`, `/vs/regle-assiduite`, `/vs/signalements`, `/eleves/{id}/retards`, `/eleves/{id}/dispenses-vs`, `/vs/signaux`, `/motifs-absence` (`ent-api/src/routes.rs`, `ent-api/src/vie_scolaire.rs:31-46`).

**Écrans Leptos** : `/web/appel` (choix de séance, feuille présent/absent/retard ; « dispensé » affiché « non enregistré par le serveur », `feuille.rs:412-418`), `/web/journee` (tableau du jour, vérification, relance, déclaration accueil), vie scolaire (absences à traiter, signaux), absences côté famille et élève (justification en texte seul), signalements.

**Mobile** : les prototypes `ent-rust/apps/` (élève, famille) ne font pas l'appel ; la branche `ent-mobile` fait l'appel **en ligne seulement** (« pas d'écriture différée (appel, justification, messages exigent le réseau) », `docs/MOBILE.md` §7 de la branche).

**Tests** : dérivation pure (15), parcours vie scolaire en base, concurrence marquer/clore, isolation entre établissements (`separation.rs`), appel de groupe borné au groupe (RP1-24), élève mineur qui ne déclare pas (RP1-09).

### 2.2 Défauts trouvés (à corriger avant toute extension)

| N° | Gravité | Défaut | Preuve | Correctif |
|---|---|---|---|---|
| **DA-01** | **grave** | Un professeur **déclare une absence pour n'importe quel élève** de l'établissement : `actor_can_declare_absence` renvoie vrai dès que le rôle est `Prof` (`can_declare_absence_staff` inclut `Prof`) sans contrôle de périmètre ; l'absence compte au décompte et peut déclencher un courrier aux familles | `service.rs:6187-6198`, `ent-auth/src/roles.rs:100-105` (relu) | AS0 : retirer `Prof`, exiger l'action `absences.declarer` (moteur D-017) et `autoriser_eleve` |
| **DA-02** | **grave** | Un professeur **efface (« remet présent ») toute absence** de l'établissement, y compris refusée par le CPE ; la décision précédente est écrasée ; la famille n'est pas informée | `service.rs:8835-8853` (rôle seul), `roles.rs:116-121` (relu) | AS0 : la correction d'un constat passe par l'appel de **sa** séance (AS-131) ; « remettre présent » une absence réservé à la vie scolaire avec motif |
| **DA-03** | **grave** | Un professeur **pose un retard** à n'importe quel élève, à n'importe quelle date (future comprise) ; le retard est notifié aux parents | `service.rs:8513-8528`, `reactions.rs:110-117` | AS0 : périmètre, date bornée (jour même ou séance de l'appel) |
| **DA-04** | **grave** | **RLS inopérante** : aucune requête du domaine n'ouvre de transaction d'établissement (`TenantTx`) ; la politique accepte toute requête sans variable tant que le mode strict est éteint ; l'isolation tient au seul `WHERE` applicatif | 0 occurrence de `TenantTx` dans `vie_scolaire.rs`, `appel_repo.rs`, `absence_repo.rs`, `retard_repo.rs`, `dispense_vs_repo.rs`, `discipline_vs.rs`, `signal_repo.rs`, `motif_repo.rs` ; 581, 563:56-60 | AS0 prépare (domaine passé en `TenantTx`), AS16 active le strict |
| **DA-05** | **grave** | **Une absence par séance** : la clôture crée un `ent_absence` par absent et par appel ; une journée d'absence = 5 à 8 absences, autant d'avis immédiats non regroupables aux parents, de justifications et de décisions | `service.rs:8352-8366` (relu), `reactions.rs:84-93` (`regroupable: false`) | AS0 : regroupement immédiat en épisode du jour (rattacher à l'absence ouverte du même élève et du même jour) ; AS3 : modèle d'épisode |
| **DA-06** | important | **Pas de correction après clôture** : un appel clos ne se modifie pas, aucune réouverture ; un oubli d'absent n'a pas de remède propre (on passe par DA-01) ; une erreur de pointage s'efface par DA-02 et laisse la ligne d'appel « absent » incohérente ; ni auteur de clôture, ni auteur de modification | `service.rs:8273`, `feuille.rs:300-302` | AS2 : correction de ligne datée et motivée (AS-131), historique |
| **DA-07** | **grave** | **Second seuil codé en dur** : « 10 absences par année scolaire », compté en **lignes** `ent_absence` (donc en séances), **justifiées et refusées comprises**, ouvre un « signal » qui écrit aux parents (fil de messagerie, courriel, notification) ; il contredit le signalement mensuel paramétrable (4 demi-journées non justifiées) et n'a aucun fondement réglementaire | `domain-eleve/src/absence.rs:314` (`SEUIL_ABSENCES_DEFAUT = 10`, relu), `service.rs:8583-8697`, `absence_repo.rs:154-176` | AS0 : **désactiver l'envoi aux familles** du signal annuel ; AS9 : remplacé par le dossier annuel (R131-6) et l'alerte « absences répétées » |
| **DA-08** | important | **Écritures non atomiques** : ouverture d'appel sans transaction (lignes partielles possibles, course qui renvoie « conflit » au lieu de l'appel existant) ; après une clôture validée, une erreur du courrier de seuil fait échouer la réponse (le client réessaie et reçoit « appel invalide ») ; déclarations, justification et « remettre présent » publient l'événement hors transaction | `service.rs:8192-8226`, `:8438-8446`, `:5062-5090`, `:8788` | AS0 |
| **DA-09** | important | **Séance annulée ou remplacée après l'appel** : rien n'est recalculé ; les absences d'une séance annulée continuent de compter ; un titulaire qui a ouvert l'appel le perd si un remplaçant est posé ensuite | `service.rs:7564-7651`, `vie_scolaire.rs:866-873`, `:7376` | AS2 (réaction aux événements de séance du socle 10) |
| **DA-10** | important | **Décisions écrasées** : un refus puis une acceptation perdent le premier avis ; le justificatif est resoumis sans trace ; la règle d'assiduité, la clôture des dispenses et le libellé des motifs n'ont pas d'historique ; aucune machine à états (`justifiee → refusee → justifiee` à volonté) | `domain-eleve/src/absence.rs:247-283`, `motif_repo.rs:94`, `vie_scolaire.rs:343-362` | AS3 (états et historique), AS1 (règle versionnée) |
| **DA-11** | important | **La règle courante réécrit le passé** : changer la coupure ou le seuil modifie tous les mois, y compris ceux déjà validés ou signalés ; la même action (CPE) propose les signalements **et** modifie la règle | `vie_scolaire.rs:935-952`, `:338` | AS1 (règle datée, action de direction), AS4 (décompte figé) |
| **DA-12** | important | **Justification et demi-journée** : une demi-journée entière justifiée n'absorbe pas les séances non justifiées de la même demi-journée ; la justification est portée ligne par ligne | `assiduite.rs:178-203` | AS3/AS4 : la justification porte sur l'épisode et couvre ses séances |
| **DA-13** | important | **Fuseau Europe/Paris codé en dur** (`heure_paris`) alors que le reste de l'ENT a un fuseau par établissement : un établissement ultramarin verrait des appels « non faits » à tort | `assiduite.rs:254-282`, `vie_scolaire.rs:1434`, `ent-api/src/vie_scolaire.rs:78`, `perimetre.rs:38-41` | AS0 |
| **DA-14** | important | **La famille crée un fait d'absence** de durée et de date quelconques par la route ancienne (`POST /eleves/{id}/absences`, utilisée par l'appli iOS famille) au lieu d'une **déclaration** ; bornes absentes (plusieurs années, futur lointain) ; reprise en demi-journées du lundi au vendredi **sans vacances ni fériés ni mercredi après-midi** | `domain-eleve/src/absence.rs:105-107`, `vie_scolaire.rs:897-923`, 130:72-91 | AS0 : la route crée une déclaration bornée (120 jours) ; AS4 : calendrier du socle 5 |
| **DA-15** | important | **Parents séparés** : la déclaration (commentaire, nom et qualité du déclarant) et la justification (`motif_justif`, `decision_note`) sont renvoyées entières à tout parent lié ; l'autre parent lit ce qu'a écrit le premier | `vie_scolaire.rs:96-113`, `ent-types` `AbsenceDto` | AS0 : DTO famille sans texte de l'autre responsable ni note de décision interne |
| **DA-16** | important | **Dispenses posées par la famille ou l'élève mineur**, actives aussitôt, sans validation, avec commentaire libre (médical) ; aucune liaison avec l'appel | `service.rs:13067-13130`, `:15936` | AS0 (droit), AS8 (modèle) |
| **DA-17** | important | **Retards lus plus largement que les absences** : un parent n'a besoin que d'un lien actif, sans `peut_voir_vs` ; le motif libre est exposé | `service.rs:8472-8508` | AS0 |
| **DA-18** | important | **Rôles codés en dur** (`can_faire_appel`, `can_trancher_justificatif`, `can_annuler_absence_vers_present`, `can_gerer_dispenses_vs`) au lieu des actions D-017 : un rôle libre « AED » ne peut ni faire l'appel d'une permanence ni qualifier un motif | `ent-auth/src/roles.rs:100-146, 291-299` | AS1 (actions du catalogue §15.1) |
| **DA-19** | important | **Effectif figé à l'ouverture** : un élève arrivé ou réaffecté après l'ouverture n'est pas ajouté ; « présent par défaut » sans état « non pointé » | `service.rs:8194-8205` | AS2 (effectif recalculé jusqu'à la validation ; état explicite) |
| **DA-20** | mineur | Motif technique `appel` proposé aux familles ; codes d'établissement en doublon des codes globaux ; libellés modifiables sans trace ; motifs globaux sans `etablissement_id` (E1 : à classer « référentiel ») | `motif_repo.rs:53, 94`, `registre_donnees.rs:235` | AS1 |
| **DA-21** | mineur | `ent_signalement_assiduite.dossier JSONB` conservé en lecture seule (D-036 : exception de transition) ; payloads d'événements porteurs de `motif_justif` (texte libre éventuellement médical) | 134:70, 563:37-51, `domain-eleve/src/absence.rs:168-171` | AS0 (retirer le texte libre des événements), AS16 (retrait de la colonne) |
| **DA-22** | mineur | Déclarations en double acceptées (trois saisies identiques de l'accueil, RP1-18) ; relance d'appel sans plafond ; décompte incluant des élèves sortis ; aucun hors ligne, appel web envoyé ligne par ligne puis clos (échec réseau = appel partiel) | `vie_scolaire.rs:611-634`, `:1222`, `feuille.rs:222-263` | AS0 (doublons), AS6, AS10 |

### 2.3 Ce qui est à garder, à corriger, à remplacer

| Élément | Verdict |
|---|---|
| Dérivation pure des demi-journées (`assiduite.rs`) : union d'intervalles, coupure ventilée, seuil, tests | **Garder**, étendre : règle versionnée, calendrier, justification par épisode, explication |
| Horaires de séance **figés** dans le constat | **Garder** |
| Clôture d'appel atomique (`FOR UPDATE`, absences et événements dans la transaction, tests de concurrence) | **Garder** ; étendre à l'ouverture et à la correction |
| Effectif borné au groupe et au périmètre (RP1-24, `perimetre.rs`) | **Garder** ; basculer vers le public de la séance du socle 10 |
| Déclarations accueil et famille, rapprochement dans les deux sens, `prevenue` | **Garder** ; devient la « situation connue » §8.2 |
| Vérification des appels (non faits, non clos, incohérences) et relance tracée | **Garder** ; ajouter la relance automatique et la file (§9.7) |
| Signalement mensuel proposé par le CPE, validé par la direction, détail typé (563) | **Garder** ; intégrer au dossier annuel (§13) |
| Notifications aux parents relues au traitement (lien actif, `peut_voir_vs`), idempotentes, sans détail médical | **Garder** ; regrouper par demi-journée (§14) |
| Masquage du commentaire médical des dispenses (RP1-20) | **Garder** ; la pièce médicale part au socle 34 |
| Signal **annuel** à 10 absences (`ent_absence_signal`) | **Remplacer** (DA-07) |
| `ent_absence` « une ligne par séance » | **Remplacer** par l'épisode (DA-05) |
| Route `POST /eleves/{id}/absences` pour les familles | **Remplacer** par la déclaration (DA-14) |
| Rôles codés en dur | **Remplacer** par les actions D-017 (DA-18) |

### 2.4 Next.js : à ne pas perdre (lecture seule, D-035)

Le module de vie scolaire de l'application Next.js (tables `vs_appel`, `vs_appel_ligne`, `vs_absence_eleve`, `vs_carnet_entree` ; `drizzle/0019`, `0021`, `0028` ; `app/lib/vs-absences-db.ts`, `accueil-absences-db.ts`, `famille-absences-db.ts`) est classé **perdable** par l'inventaire T0 (C07 : Charlemagne reste officiel ; Q-T0-07 sur les absences saisies par l'accueil). Ses bonnes idées sont reprises [F, lecture seule] :

| Idée de Next.js | Où dans ScolIA |
|---|---|
| **Garde-fou « en sortie »** : un élève en sortie ne crée pas d'absence ; l'accueil est averti avant de déclarer, avec forçage motivé (`vs-absences-db.ts:289-291`, `accueil-absences-db.ts:385-396`) | situation `en_sortie` (§8.2), AS-112, R-AS7 |
| **Saisie de l'accueil** en trois modes (aujourd'hui, quelques heures, plusieurs jours), canal (téléphone, visite, courriel), motif par défaut « les parents ont appelé l'accueil » (`vs-absences-db.ts:994-998`) | AS-110 (déclaration accueil) |
| Élève **prévenu par l'accueil** signalé à l'appel, sans absence en double (`vs-absences-db.ts:293-297`) | pré-remplissage §9.3 |
| **Repasser présent** : l'absence déjà traitée est classée, sinon retirée, historique conservé (`vs-absences-db.ts:359-408`) | correction AS-131 |
| Liste des **appels manquants** par créneau commencé (`vs-absences-db.ts:826-879`) | AS-150 |
| Vues famille et élève **sans la note interne** du CPE (`famille-absences-db.ts:25`) | R-AS22 |
| Un seul appel par séance et une ligne par élève (index uniques) | contraintes §22 |

Points faibles de Next.js à **ne pas reproduire** (relevés en lecture, non exécutés) : aucune notification à la famille pour une absence signalée par l'accueil ou constatée ; relance qui ne fait qu'horodater ; justification famille qui **écrase la source** de l'absence (l'absence disparaît du tableau de l'accueil) ; doublon possible accueil + appel sur la même séance ; rattachement du professeur à ses créneaux **par ressemblance de nom** (`vs-appels-access.ts:48-52`). Deux points sont des **failles probables en production** (routes ouvertes à tout professeur ; appel mobile sans contrôle de module) : ils sont signalés à Florian (§27.2, P-1), **sans aucune action** sur Next.js.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps ou qu'il redoute | Ce qui le ravirait |
|---|---|---|---|
| **Professeur** (y compris remplaçant, contractuel, professeur de groupe, co-intervenant) | appel à chaque séance, retards, mot sur un élève, constat d'absence à une évaluation | appel oublié puis relance ; liste fausse (groupe, élève parti, nouvel arrivé) ; EPS dehors sans réseau ; « il est à l'infirmerie ? » ; appel perdu par une coupure ; être tenu responsable d'une absence qu'il ne pouvait pas savoir | l'appel en dix secondes depuis la séance du jour, déjà pré-rempli (sortie, stage, infirmerie, dispense, prévenu) ; hors ligne sans y penser ; une correction possible sans appeler la vie scolaire |
| **CPE** | qualifie les motifs, suit les élèves, dialogue avec les familles, convoque, prépare les signalements, préside le suivi | 60 h par semaine [V-ext] ; téléphone saturé de 8 h à 9 h [H] ; même absence reçue par quatre canaux ; justificatifs papier ; décompte refait à la main ; seuils et courriers manuels (G23) ; dossier de signalement monté à la main | une file du matin déjà triée ; un dossier par élève qui se remplit seul (contacts, courriers, mesures) ; un signalement pré-rempli à relire |
| **AED** (assistant d'éducation) | appel de permanence et d'étude, billets de retard, tableau du jour, appels aux familles, saisie des motifs reçus | appeler les familles une à une ; ne pas savoir si un collègue a déjà appelé ; carnet papier ; « où est X ? » | « famille jointe, message laissé » en un geste, visible de tous ; liste des élèves attendus en permanence |
| **Accueil** | reçoit l'appel ou la visite d'un parent, le billet d'un élève en retard, la sortie anticipée | double saisie dans un tableur partagé (exemple vécu D-023) ; ne pas savoir si l'élève est en sortie ; ne pas reconnaître l'appelant | trois gestes : élève, période, motif dit ; l'appelant reconnu (ADM-100) ; « en sortie aujourd'hui » affiché |
| **Direction** (chef, adjoint) | valide les signalements, préside la commission éducative, rend compte au CA, pilote | taux d'absentéisme reconstitué pour le CA ; signalements tardifs ; décisions contestées | indicateurs par classe et niveau prêts ; signalement relu et validé en deux minutes ; rapport annuel prérempli |
| **Secrétariat** | courriers postaux (relance sans réponse), certificats de scolarité, transmissions académiques | publipostage à la main ; ne pas savoir qui n'a pas répondu | lot de courriers de relance prêt (trames D-023) |
| **Professeur principal** | suit sa classe, prépare le conseil, rencontre les familles | découvrir au conseil qu'un élève décroche | tendance de sa classe, alertes douces, assiduité au conseil (socle 23) |
| **Infirmière** | reçoit les élèves, renvoie au domicile | élève noté absent alors qu'il est chez elle (M-2506) ; parents avisés « absent » à tort | « à l'infirmerie » qui remplace l'absence avant l'avis, sans motif |
| **AESH** | accompagne un ou plusieurs élèves | ne pas savoir que l'élève accompagné est absent (M-2376) | avis de l'absence de l'élève accompagné, sans motif |
| **Assistante sociale, psychologue de l'EN, référent décrochage, GPDS** | accompagnement, commission | ne pas être associés au suivi | être désignés « référent » dans le dossier, voir ce qui est utile |
| **Encadrant de sortie, animateur, surveillant d'internat** | appel d'activité, de départ, d'étape | pas de réseau ; liste papier | le même composant d'appel (socles 41, 42, 43) |
| **DDFPT, enseignant référent de PFMP, tuteur** | suivi des stages | absences en entreprise saisies hors système | absences de PFMP dans le même décompte, visibles du référent |
| **Famille** (parents ensemble, séparés, tiers, famille d'accueil, parent sans autorité parentale, parent allophone, sans smartphone) | prévient, justifie, reçoit les avis, rencontre la CPE | appeler à 8 h ; justifier six fois la même journée ; ne pas savoir si le mot a été reçu ; recevoir « absent » quand l'enfant est à l'infirmerie ; ne pas comprendre « non justifiée » ; recevoir le texte de l'autre parent | prévenir en ligne en trente secondes, avec ou sans pièce ; un seul avis par demi-journée ; savoir où en est chaque absence ; un démenti si l'avis était faux |
| **Élève mineur** | voit ses absences et retards | être compté absent alors qu'il était en mission de délégué ; les SMS en cours | voir clairement son assiduité et ce qui manque |
| **Élève majeur** | déclare et justifie lui-même | que ses parents soient informés sans qu'il le sache | savoir qui est informé, pouvoir demander (DPO) |
| **Étudiant de BTS ou de CPGE en lycée, apprenti** | assiduité conditionnant la bourse du CROUS ou le contrat | absences remontées au CROUS sans justificatif demandé | justificatif demandé avant tout signalement (circulaire AMUE [V-ext]) |
| **Autorité académique (DASEN), mairie, CROUS, employeur d'apprenti** | destinataires extérieurs | dossiers incomplets | exports conformes, transmis par une personne habilitée |
| **Auditeur RGPD, DPO** | contrôle | durées non fixées, motifs de santé en clair, accès trop larges | registre, durées par catégorie, journal des consultations |

---

## 4. La journée et l'année de l'assiduité vues du terrain

**Une journée** [H, d'après le terrain et les sources du §5]. *7 h 30* : des familles déclarent en ligne (malade, rendez-vous). *7 h 50–8 h 30* : l'accueil prend les appels ; les déclarations apparaissent pré-remplies dans les appels de 8 h. *8 h 00* : les professeurs font l'appel depuis la séance du jour (web, téléphone) ; le gymnase n'a pas de réseau, l'appel s'enregistre hors ligne. *8 h 15* : la vie scolaire voit les appels non faits et relance d'un geste (puis automatiquement). *8 h 15–8 h 30* : délai de consolidation ; un élève arrive en retard avec un billet, l'absence devient retard, l'avis ne part pas ; un autre est à l'infirmerie, localisé. *8 h 30* : un avis par foyer part pour les absences inexpliquées. *9 h 00* : la file du matin montre ce qui reste sans nouvelles, classé par urgence ; les AED appellent, notent « joint / message laissé ». *10 h* : Léa, présente à 8 h, manque à 10 h : alerte interne immédiate (`absence_apres_presence`), puis famille si personne ne la localise. *12 h–14 h* : demi-pension (appel si l'établissement le fait), permanences. *Après-midi* : l'infirmière renvoie un élève au domicile (absence des séances restantes, motif « santé » visible de la vie scolaire seule) ; une sortie part, les absents au départ redeviennent absents de leurs séances. *17 h* : récapitulatif ; relance graduée des justificatifs manquants.

**Une année.** *Août* : règle d'assiduité de l'année (coupure, définition de la demi-journée, délais) relue, inscrite au règlement intérieur ; profil académique (seuils, outil de signalement). *Rentrée* : élèves non présentés (socle 3, INS-43) ; dispenses annuelles d'EPS. *Chaque mois* : décompte arrêté le 1er, signalements proposés, validés, transmis ; dossiers de suivi. *Chaque trimestre* : assiduité au bulletin (socle 23) ; certification de l'assiduité des boursiers et retenue (socle 26, R531-31) ; listes du CROUS avant le 1er décembre pour les étudiants boursiers [V-ext]. *Juin* : rapport annuel d'absentéisme au CA (circulaire 2014-159 [V-ext]) ; clôture des dossiers de l'année (R131-6 : dossier ouvert pour la seule année scolaire) ; archives (registre d'appel 10 ans, justificatifs 1 an, VERIF R41).

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Sources : Code de l'éducation, Code pénal, Code civil, Code du travail lus dans les **données ouvertes de Légifrance** republiées par codes.droit.org (texte consolidé au 2026-10-31) [V] ; circulaire interministérielle **n° 2014-159 du 24-12-2014** (BO n° 1 du 1-1-2015, https://www.education.gouv.fr/bo/15/Hebdo1/MENE1427925C.htm, 403 pour les robots ; lue sur une copie intégrale) [V-ext] ; vademecum de l'académie de Paris 2025-2026 [V, texte académique] ; CNIL, SDET et kit juridique 2025 [V] ; fiche AMUE 2025-2026 [V-ext]. Toutes consultées le **2026-11-11**. Comme pour `VERIF_REGLEMENTAIRE.md`, **un juriste relit l'article sur Légifrance avant de coder une règle qui en dépend**.

| N° | Règle | Texte | Statut | Conséquence pour ScolIA |
|---|---|---|---|---|
| **RA-01** | Registre d'appel, public **et privé** | R131-5 al. 1 : « un registre d'appel sur lequel sont mentionnées, pour chaque classe, les absences des élèves inscrits » | [V] | registre numérique par séance, exportable (§13.6) ; s'applique au privé sous contrat (D-019) |
| **RA-02** | Tout personnel responsable d'une activité signale les absents | R131-5 al. 2 : « Tout personnel responsable d'une activité organisée pendant le temps scolaire signale les élèves absents, selon des modalités arrêtées par le règlement intérieur » ; circulaire 2014-159 I.1 A : « Chaque enseignant prenant en charge une classe procède à l'appel » | [V] / [V-ext] | l'appel n'est pas réservé au professeur : permanence, étude, CDI, sortie, atelier, accompagnement (§9.1) |
| **RA-03** | Information **immédiate** des responsables | R131-5 al. 3 : « Toute absence est immédiatement signalée aux personnes responsables de l'enfant qui doivent sans délai en faire connaître les motifs » | [V] (VERIF R1) | avis automatique dû ; le délai de consolidation (§14.2) reste court (défaut 15 min) et motivé par l'exactitude |
| **RA-04** | Canal d'alerte, puis courrier | Circulaire 2014-159 II.1 : « par tout moyen, de préférence par appel téléphonique, SMS ou courrier électronique… Sans réponse… suivi d'un courrier postal » | [V-ext] | escalade jusqu'au courrier postal (trame D-023) ; envois journalisés |
| **RA-05** | Absence prévisible déclarée à l'avance | R131-5 al. 4 | [V] | déclaration préalable par la famille (AS-111) |
| **RA-06** | Doute sérieux sur le motif → demande d'autorisation transmise au DASEN | R131-5 al. 5 | [V] | circuit rare « demande d'autorisation d'absence » (AS-125) |
| **RA-07** | **Cinq motifs réputés légitimes**, les autres appréciés | L131-8 al. 1 : « maladie de l'enfant, maladie transmissible ou contagieuse d'un membre de la famille, réunion solennelle de famille, empêchement résultant de la difficulté accidentelle des communications, absence temporaire des personnes responsables lorsque les enfants les suivent » ; « Les autres motifs sont appréciés par l'autorité de l'État » | [V] | nomenclature nationale à 5 natures « réputées légitimes » + motifs de l'établissement à apprécier ; jamais un autre motif présenté comme légitime de droit (§10.2) |
| **RA-08** | Certificat médical exigible seulement en cas de maladie contagieuse | Circulaire 2014-159 II.1 | [V-ext] (VERIF R27) | aucun certificat exigé par défaut ; le règlement intérieur peut en demander, ScolIA l'affiche sans l'imposer |
| **RA-09** | **Dossier d'absences par année scolaire** | R131-6 al. 1 : « mentionnées dans un dossier, ouvert pour la seule année scolaire, qui regroupe l'ensemble des informations et documents relatifs à ces absences » ; al. 2 : absences répétées même justifiées → dialogue | [V] | dossier annuel de suivi (§13.3), clos en fin d'année ; alerte « absences répétées » distincte du seuil |
| **RA-10** | Accès des familles au dossier | Circulaire 2014-159 II.2 : informées « de l'existence de ce dossier et des conditions dans lesquelles elles peuvent y avoir accès, notamment à travers les ENT » | [V-ext] | vue famille du dossier (§13.3) |
| **RA-11** | **Seuil de quatre demi-journées dans le mois** | R131-7 I : « au moins quatre demi-journées dans le mois » sans motif légitime ni excuses valables → commission éducative, document récapitulatif signé avec les responsables, saisine du DASEN qui avertit ; L131-8 al. 3 : second déclencheur, **motif non communiqué ou inexact malgré invitation** | [V] (VERIF R2) | alerte à la direction et au CPE, procédure guidée ; **jamais** de saisine automatique ; le second déclencheur est couvert (AS-171) |
| **RA-12** | Persistance → aide contractualisée, **personnel référent** | L131-8 al. 4 ; R131-7 II ; circulaire : persistance « de l'ordre de dix demi-journées complètes dans le mois » (indicatif) | [V] / [V-ext] | désignation d'un référent, engagement daté, suivi (AS-176) |
| **RA-13** | Nouveau signalement avec le dossier | R131-7 III : transmission du « dossier individuel de suivi de l'absentéisme » | [V] | export du dossier (AS-181) |
| **RA-14** | Procureur et infraction pénale : **DASEN seul**, moins de 16 ans | R131-7 IV, L131-9 ; Code pénal R624-7 (contravention de 4ᵉ classe, enfant soumis à l'obligation scolaire) ; L131-1 (3 à 16 ans) | [V] | aucun montant ni « procès-verbal » dans ScolIA ; niveau « procureur » des profils académiques désactivé à 16 ans révolus |
| **RA-15** | Plus de suspension des allocations familiales | Circulaire 2014-159 (loi de 2013) | [V-ext] | aucune mention dans les courriers |
| **RA-16** | **Aucune définition légale de la demi-journée** ni du mois | L131-8, R131-7 ; la circulaire parle de « demi-journées complètes » | [V] | règle paramétrable, versionnée, inscrite au règlement intérieur (§13.1, Q-AS3) ; mois civil par défaut |
| **RA-17** | Dès la **première heure** d'absence non justifiée : contact avec la famille, convocation de l'élève par le CPE | Circulaire 2014-159 II.2 | [V-ext] | file du matin et contact tracé dès la première séance (§14.4) |
| **RA-18** | Absentéisme : pas d'exclusion, pas de sanction au bulletin | Circulaire 2014-159 II.2 : « l'exclusion, même temporaire… doit être écartée » ; « ne pas inscrire ces sanctions sur le bulletin » | [V-ext] | aucune proposition de sanction d'exclusion depuis le socle 12 ; le bulletin affiche le nombre d'absences et de retards, jamais une sanction (socles 13, 23) |
| **RA-19** | Représentants des élèves en instance : **pas une absence** | Circulaire 2014-159 II.1 : « ne doit pas être traitée comme une absence et reportée comme telle sur le bulletin » | [V-ext] | situation `mission_representation` (§8.2) |
| **RA-20** | Rapport annuel au CA, taux par classe et par niveau | Circulaire 2014-159 I.1 A (loi 2010-1127) | [V-ext] | rapport prérempli (AS-190) |
| **RA-21** | Information des collectivités et de la protection de l'enfance des **mesures** prises | L131-8 al. 5 ; information préoccupante (CASF L226-3) | [V] / [V-ext] | export agrégé ; l'information préoccupante relève du socle 36 |
| **RA-22** | Mairie : transmission limitée aux cas du décret | L131-6, R131-10-1 | [V] | aucun export libre des absences vers la mairie |
| **RA-23** | Assiduité de l'élève : obligation, options facultatives inscrites | L511-1 ; R511-11 : « Elle s'impose pour les enseignements obligatoires et pour les enseignements facultatifs dès lors que les élèves se sont inscrits » ; R421-5 : modalités au règlement intérieur | [V] | l'appel couvre les options inscrites ; paramètres au règlement intérieur |
| **RA-24** | Obligation de formation 16-18 ans, missions locales | L114-1, R114-1 à R114-7 | [V] | au-delà de 16 ans : circuit décrochage (MLDS), pas pénal |
| **RA-25** | Majorité | Code civil 414, 371-1 ; pas de texte national trouvé sur l'information des parents d'un majeur (circulaire de 1991 non lue) | [V] / [NV] | l'élève majeur déclare et justifie ; information des parents selon politique de l'établissement validée par le DPO (Q-AS9) |
| **RA-26** | Parent sans exercice de l'autorité parentale informé | Code civil 373-2-1 ; guide éduscol | [V] (VERIF R11) | reçoit les avis d'absence sauf restriction du juge (socle 4) |
| **RA-27** | **Inaptitude d'EPS** | R312-2 : certificat « total ou partiel », validité ≤ année scolaire ; R312-3 : médecin scolaire destinataire au-delà de trois mois ; pas d'effet rétroactif (arrêté du 13-9-1989, [V-ext]) ; « ne constitue en aucun cas une dispense systématique de présence » (document académique EPS [V-ext]) | [V] / [V-ext] | inaptitude (socle 34) ≠ dispense de présence (décision de l'établissement, §12) ; dates contrôlées ; alerte au-delà de trois mois |
| **RA-28** | **Retenue sur bourse** | R531-31 (lycée), D531-12 (collège) : au-delà de **quinze jours** d'absences injustifiées cumulées depuis la rentrée, retenue de 1/270 par jour, imputée au trimestre du franchissement ; décision du chef d'établissement (public), du DASEN sur proposition (privé) | [V] | décompte en **jours** pour le socle 26, distinct des demi-journées ; export trimestriel (AS-187) |
| **RA-29** | Étudiants boursiers (BTS, CPGE) | D821-1 ; contrôle de l'assiduité par le chef d'établissement ; « les déclarations d'absence de moins d'un mois ne conduiront pas à une suspension » ; justificatif demandé avant tout signalement ; liste au CROUS avant le 1er décembre (fiche AMUE 2025-2026) | [V] / [V-ext] | profil « étudiant boursier » (AS-188) |
| **RA-30** | PFMP : l'élève reste inscrit ; interruption pour maladie n'empêche pas la validation | L124-1, L124-2, L124-15, D124-3, D124-4 | [V] | situation `en_pfmp` ; absences en entreprise saisies par le référent (AS-118) |
| **RA-31** | Apprentis : formation dans le temps de travail ; employeur informé | Code du travail L6222-24, L6223-4 | [V] (usage [V-ext]) | export des absences pour l'employeur (AS-189) |
| **RA-32** | Fêtes religieuses : autorisations pour les grandes fêtes, refus des absences systématiques | circulaire du 18 mai 2004 (laïcité) | [V] (copie) | motif « fête religieuse » à autorisation préalable, décidé par le chef d'établissement |
| **RA-33** | Biométrie interdite pour la présence | CNIL, décision du 26-6-2008 (empreintes) [V-ext] ; reconnaissance faciale refusée (2019) [V-ext] ; cantine : contour de la main sous consentement et alternative [V] | [V] / [V-ext] | **aucun appel biométrique** ; badge ou QR seulement après AIPD (Q-AS14) |
| **RA-34** | Conservation | Registre d'appel 10 ans, bulletins d'absence et justificatifs 1 an (instruction de tri, BO n° 24 du 16-6-2005, VERIF R41) ; téléservice ministériel : année scolaire (arrêté du 16-7-2015, [V-ext]) ; SDET : base active « année scolaire ou cycle » ; **aucune durée spécifique « absences » trouvée** dans le SDET 2024 ni le kit juridique 2025 [V] | [V] / [V-ext] | §15.6 : base active = année scolaire + 1 an de consultation, archives du registre 10 ans, justificatifs 1 an ; **à valider par le DPO** |
| **RA-35** | Effacement des sanctions modifié par le décret n° 2026-637 du 16-7-2026 | R511-13 (version du 18-7-2026) | [V] | concerne le socle 13 ; à recouper avant de coder (signalé au §28) |

**Pas trouvé** [NT] : texte national sur l'enquête statistique mensuelle d'absentéisme (DEPP) ; texte national sur la JDC, les examens, le permis comme motifs ; circulaires 90-107 (EPS) et 1991 (lycéens majeurs) non lues ; kit de conservation et d'archivage du SDET non lu.

**Profils académiques** [V, Paris ; V-ext, Nantes] : les académies ajoutent leurs propres seuils et outils (Paris 2025-2026 : niveau 1 au-delà de 10 demi-journées par l'outil ADESCO, niveau 2 si persistance dans les 15 jours avec compte rendu du GPDS, niveau 3 pour les moins de 16 ans ; Nantes : mois glissant, outil MIMOSA). ScolIA ne code **aucun** seuil académique en dur : un **profil académique** paramétrable (§13.5).

---

## 6. Frontières

### 6.1 Tableau

| Le socle 12 contient | Il ne contient pas (et qui le porte) |
|---|---|
| L'appel de toute activité du temps scolaire (cours, permanence, étude, CDI, accompagnement, demi-pension si l'établissement l'active), web, mobile, hors ligne ; le **composant d'appel** réutilisé par 41, 42, 43 | La séance datée, son état, son public, son intervenant (socle 10 ; cours et groupes : 5 ; interventions : 6) |
| Les situations connues (déclarations, dispenses de présence, localisations, et le reflet des sorties, stages, examens, exclusions décidés ailleurs) | La décision de sortie (42), la convention de stage (39), la convocation d'examen (38), l'exclusion-inclusion (13), la mobilité (2) |
| Les absences (épisodes), motifs déclarés, justificatifs, qualification | La pièce médicale et l'inaptitude elle-même (34) ; la pièce comme objet (7) |
| Retards, billets d'entrée, passages à la vie scolaire | Punitions et sanctions consécutives à des retards (13) |
| Dispense **de présence** et prise en compte à l'appel | L'inaptitude médicale d'EPS (34), l'aménagement aux examens (38), l'AESH (35) |
| Décompte en demi-journées, compteurs, seuils, dossier annuel de suivi (R131-6), procédure de signalement, profils académiques, exports au DASEN | La commission éducative et le document signé comme **instance** (13 : la commission ; 7 : le document signé) — le 12 les **déclenche** et les **suit** ; l'information préoccupante (36) |
| Avis d'absence et de retard (contenu, destinataires, moment) | Le moteur d'envoi, les préférences, l'escalade technique, le SMS (8) ; le fil de conversation (16) |
| Statistiques d'assiduité, rapport annuel au CA, export pour le bulletin et pour les bourses | Le bulletin (23) ; la bourse et sa retenue (26) ; les tableaux de pilotage (47) |
| Appel de départ et d'étape d'une sortie (composant) | L'internat : appel du soir (41, même composant) ; les activités périscolaires (43) |
| — | Absences des personnels (15) ; présence en cas d'alerte PPMS (14, qui lit les présences du 12) ; remise de l'élève à un tiers (14) |

### 6.2 Ports exposés par le socle 12 [R]

| Port | Contenu | Consommateurs |
|---|---|---|
| `assiduite::presence_a(eleve, instant)` | `present`, `absent`, `retard`, `localise(lieu_code)`, `inconnu` ; **sans motif** | 14 (PPMS, remise), 34 (infirmerie), 41 |
| `assiduite::presents_maintenant(etab, site)` | liste des élèves présents ou localisés (alerte PPMS) | 14 |
| `assiduite::absences_periode(eleve, periode)` | épisodes, demi-journées, qualifications, **sans texte libre** | 23 (bulletin), 21 (évaluations), 26 (bourses), 37, 47 |
| `assiduite::absent_a_seance(eleve, seance)` | absent / présent / retard / localisé, état de régularisation | 21 (absent à une évaluation, CE-26), 20 |
| `assiduite::jours_injustifies(eleve, annee, jusqu_au)` | décompte en **jours** pour la retenue de bourse (R531-31) | 26 |
| `assiduite::presences_rentree(etab, depuis)` | élèves inscrits sans aucune présence | 3 (INS-43, non présentés) |
| `assiduite::composant_appel` | appel générique (activité, sortie, étape, internat) avec hors ligne | 41, 42, 43 |
| `assiduite::declarer(eleve, periode, motif_declare, source)` | commande (accueil, famille, messagerie, IA) | 4 (ADM-100), 16 (MSG-171), 30 |
| `assiduite::localiser(eleve, seance_ou_periode, lieu_code)` | commande (infirmerie, vie scolaire, CDI, entretien) | 34, 13, 36 |

### 6.3 Ports consommés [R]

`seances::du_jour`, `seance`, `seance_en_cours`, `public_seance(seance)` (10) ; `cours`, `groupes`, `calendrier`, `structure_grilles::demi_journee` (coupure STR-35), fermetures (5) ; `interventions_a(seance)` (6) ; `eleves` (identité réduite, photo, majorité, statut de scolarisation, mobilité) (2) ; `responsables`, `joignables`, `interdictions`, `adresse_protegee`, ADM-100 (4) ; `documents.deposer` (7) ; `notifications.avis` (8) ; `messagerie` (16) ; `sante.inaptitude_*`, `sante.passage_*` sans motif (34) ; `sortie.participants_partis` (42) ; `stage.periodes` (39) ; `examen.convocations` (38) ; `discipline.exclusion_inclusion` (13).

### 6.4 Points à accorder avec les autres socles [R]

- **Socle 10** : la séance attend un appel si son état est `prevue`, `modifiee` ou `ajoutee` et qu'un intervenant est en saisie (10 §8.3) ; une séance **annulée après l'appel** neutralise ses constats (R-AS12) ; un appel d'un **cours déplacé** est ouvert depuis la séance (10 D5).
- **Socle 5** : la coupure des demi-journées (STR-35) est **lue** par le 12 ; elle n'est pas recopiée dans `ent_vs_regle_assiduite` (C-3).
- **Socle 8** : natures `vie_scolaire.*` (8 §4.1) ; ajout de `vie_scolaire.absence_dementi`, `vie_scolaire.relance_justificatif`, `vie_scolaire.absence_eleve_accompagne` (AESH) (§14.5).
- **Socle 34** : en mode N-0 (santé désactivée), l'infirmerie n'est qu'une **localisation** sans motif ; le motif « santé » d'un renvoi au domicile est visible de la vie scolaire seule (34 §349).
- **Socle 42** : « en sortie » vient des seuls participants **partis** ; un absent au départ redevient absent de ses séances maintenues (42 §15.2).

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]

| Terme | Sens |
|---|---|
| **Appel** | Le relevé d'une **activité datée** (séance de cours, permanence, étude, activité, départ ou étape de sortie, demi-pension), fait par la personne qui en a la charge ; il a un état (`a_faire`, `en_cours`, `valide`, `corrige`, `neutralise`) |
| **Ligne d'appel** | L'état d'un élève attendu à cet appel : `non_pointe`, `present`, `absent`, `retard` (minutes), `localise` (lieu codé : infirmerie, vie scolaire, CDI, entretien, autre activité), `attendu_ailleurs` (sortie, stage, examen, mission, exclusion-inclusion), `present_dispense` (présent, ne pratique pas) |
| **Constat** | Une ligne d'appel validée, avec son historique de corrections |
| **Situation connue** | Un fait **antérieur ou extérieur** à l'appel qui dit où est l'élève ou pourquoi il ne sera pas là : déclaration (famille, accueil, message), dispense de présence, localisation, et reflets des autres socles (sortie, stage, examen, mission de représentation, exclusion-inclusion, mobilité, déscolarisation temporaire) |
| **Absence** (épisode) | Suite continue de séances manquées par un élève (un jour, ou plusieurs jours si une déclaration les couvre) ; unité du motif, du justificatif, de la qualification et de l'avis |
| **Motif déclaré** | Ce que la famille (ou l'élève majeur) **fait connaître** |
| **Qualification** | L'appréciation de l'établissement : `reputee_legitime` (une des cinq natures de L131-8), `excuse_valable`, `non_valable`, `sans_motif`, `en_attente` |
| **Justificatif** | Une pièce facultative (mot, certificat, convocation) attachée à l'épisode ; objet immuable (E5) |
| **Régularisée** | Absence dont la qualification n'est plus `en_attente` ni `sans_motif` |
| **Demi-journée d'absence** | Résultat du calcul de la règle d'assiduité en vigueur à la date sur les constats d'une demi-journée ; « non justifiée » si sa part non couverte par une qualification `reputee_legitime` ou `excuse_valable` atteint le seuil |
| **Retard** | Arrivée après le début de l'activité ; compté **à part** des absences (VADE [V]) |
| **Dispense de présence** | Décision de l'établissement autorisant l'élève à ne pas assister à un cours (≠ inaptitude, RA-27) |
| **Dossier annuel d'assiduité** | Le dossier de R131-6 : absences, durées, motifs, contacts, mesures, de la seule année scolaire |
| **Signalement** | Une transmission à l'autorité académique, préparée par le CPE, validée par la direction, tracée |
| **Profil académique** | Paramètres de l'académie (seuils de niveaux, mois civil ou glissant, outil de transmission, âge du niveau pénal) |

### 7.2 Quatre distinctions qui manquent aujourd'hui

1. **Déclaration ≠ absence** : la famille prévient ; seule l'absence constatée (ou la déclaration sur une période sans appel, ex. journée entière annoncée) devient un épisode.
2. **Motif ≠ qualification** : la famille dit « rendez-vous chez l'orthodontiste » ; la CPE apprécie « excuse valable » ou non.
3. **Absence ≠ hors de la classe** : l'infirmerie, la sortie, le CDI, l'entretien sont des localisations, jamais des absences.
4. **Inaptitude ≠ dispense** : l'élève inapte en EPS est présent par défaut.

---

## 8. Le modèle : de l'attendu au constat, du constat à l'absence

### 8.1 Ce qu'un appel attend [R]

Pour une séance datée `S` (socle 10) : l'**effectif attendu** = `public_seance(S)` à la date (cours, groupes, inclusions du socle 5, arrivées et départs, mobilités exclues) ; il est **recalculé** jusqu'à la validation (DA-19) et figé à la validation, avec mention des élèves ajoutés ou retirés entre l'ouverture et la validation. Pour une activité sans séance (permanence, étude, sortie), l'effectif vient du port du socle propriétaire (EDT-219 pour la permanence, participants pour la sortie).

### 8.2 Situations connues et pré-remplissage [R]

| Nature | Source | Effet à l'appel | Compte dans l'assiduité | Visible du professeur |
|---|---|---|---|---|
| `declaration_famille` | famille, élève majeur (AS-111) | pré-coché `absent (prévenu)` ; le professeur confirme d'un geste | oui (épisode, motif déclaré) | « prévenu », **sans motif** |
| `declaration_accueil` | accueil (AS-110) | idem | oui | idem |
| `declaration_message` | carte proposée depuis la messagerie, confirmée par la vie scolaire (16 MSG-171) | idem | oui | idem |
| `dispense_presence` | vie scolaire (§12) | `attendu_ailleurs (dispensé)` ou `present_dispense` selon la décision | non | « dispensé » |
| `localisation` | infirmerie, vie scolaire, CDI, entretien (AS-140) | `localise (lieu)` | non | lieu codé seulement (« infirmerie » est déjà une information de santé : voir R-AS20) |
| `en_sortie` | socle 42 (participants partis) | `attendu_ailleurs (sortie)` | non | « en sortie » |
| `en_pfmp` / `en_stage` | socle 39 | séance non attendue ; absences en entreprise saisies à part (AS-118) | absences de PFMP oui | — |
| `en_examen` | socle 38 | `attendu_ailleurs (examen)` | non | « examen » |
| `mission_representation` | vie scolaire ou instance (RA-19) | `attendu_ailleurs (instance)` | **non**, ni au bulletin | « instance » |
| `exclusion_inclusion` | socle 13 | `attendu_ailleurs (vie scolaire)` | non (l'élève est présent dans l'établissement) | « vie scolaire » |
| `mobilite` / `cned_temporaire` | socle 2 (ELV-35) | séances non attendues | non | — |
| `autorisation_absence` | demande accordée (fête religieuse, convocation officielle, JDC, examen extérieur) (AS-125) | pré-coché `absent (autorisé)` | **oui en absence, qualifiée `excuse_valable`** ; ne déclenche aucun avis d'absence | « prévenu » |

Une situation **ne remplace jamais** le geste du professeur : il voit l'état proposé et peut le changer (l'élève prévenu absent est finalement venu). Si le professeur marque **présent** un élève « en sortie », la vie scolaire est alertée (incohérence, §9.7).

### 8.3 Du constat à l'absence [R]

À la validation d'un appel, pour chaque ligne `absent` :
1. on cherche un **épisode ouvert** du même élève qui touche cette séance (même jour, séance précédente ou suivante contiguë dans l'emploi du temps de l'élève, ou couverte par une déclaration) ; sinon on en crée un ;
2. on rattache la séance à l'épisode (`ent_asd_absence_seance`) avec ses horaires figés et la règle en vigueur ;
3. si une déclaration couvre la séance, l'épisode prend son motif déclaré et sa source (`prevenue`) ;
4. l'épisode est `en_attente` de qualification (ou qualifié automatiquement si la règle de l'établissement l'autorise pour un motif donné : par exemple « maladie de l'enfant déclarée par un responsable » → `reputee_legitime` d'office, S-471, désactivé par défaut).

Une ligne `retard` crée un **retard** (§11). Une ligne `localise` ou `attendu_ailleurs` ne crée rien. Une **correction** (§9.6) défait ou refait ces effets.

**Contiguïté** : deux séances d'un même jour sont contiguës si aucune séance où l'élève était présent ne les sépare. Un élève absent à 8 h, présent à 10 h, absent à 14 h a **deux** épisodes (le second déclenche `absence_apres_presence`).

### 8.4 États d'un épisode [R]

`ouvert` (constats en cours d'accumulation dans la journée) → `a_qualifier` → `qualifie` (`reputee_legitime` | `excuse_valable` | `non_valable` | `sans_motif`) ; `retire` (tous ses constats corrigés en présence ou neutralisés). Transitions **historisées** (`ent_asd_absence_etat`) ; une requalification exige un motif et est notifiée (U0) à la famille si elle change le sens (« non valable » → « excuse valable »).

### 8.5 Lien avec les autres socles

Un épisode qualifié `reputee_legitime` pour « maladie de l'enfant » ne contient **aucune** donnée de santé au-delà de la nature légale (c'est la catégorie de la loi) ; le texte du motif déclaré et le certificat sont des données de santé indirectes (34 §1177) : classe C3, visibles de la vie scolaire seule (R-AS19).

---

## 9. L'appel en détail

### 9.1 Qui fait l'appel, de quoi [R]

- **Cours** : l'intervenant de la séance à la date (titulaire, remplaçant, suppléant) ; en **co-intervention**, l'un ou l'autre (premier qui valide, l'autre voit l'appel fait) ; l'AESH ne fait pas l'appel. Un professeur **ne fait pas** l'appel d'une séance qui n'est pas la sienne, sauf **délégation datée** (AS-105) ou action `appel.faire_tout` (vie scolaire).
- **Permanence, étude, CDI, accompagnement** : l'AED, le documentaliste, l'animateur affecté (RA-02).
- **Demi-pension** (option de l'établissement, VADE [V]) : pointage du passage (socle 40 lit le 12 ou fournit le passage).
- **Sortie** : appel de départ et d'étape par l'encadrant (42).
- **Internat, activités** : même composant (41, 43).

### 9.2 Ouvrir et faire l'appel [R]

- **Depuis la séance** (accueil du professeur, emploi du temps, notification « votre cours commence ») ; jamais par « classe + jour ». Un appel par séance (unique), créé **idempotent** (deux ouvertures simultanées renvoient le même appel, DA-08).
- **Trois façons de pointer** : (1) « tous présents » puis exceptions (défaut) ; (2) un geste par élève sur le trombinoscope (photo si l'établissement l'active, S-468) ; (3) « présents seulement » pour un petit groupe [V, Pronote item 429]. Les élèves pré-remplis (§8.2) sont affichés en tête, avec leur état proposé.
- **Retard** : calcul automatique des minutes depuis l'heure de début de la séance (modifiable) [V, Pronote item 582].
- **Rappel discret** sur la ligne d'un élève : « absent à la séance précédente » (sans motif), « convoqué à la vie scolaire » [V, Pronote item 585] — jamais l'historique complet.
- **Valider** : un bouton « Appel fait » avec récapitulatif (« 26 présents, 2 absents dont 1 prévenu, 1 retard ») ; confirmation humaine (D-003, mobile). L'appel validé avec **aucune** modification est un constat comme un autre : c'est le geste de validation qui compte.
- **Après le cours** : l'appel reste possible jusqu'à la fin de la journée par le même intervenant [V, Pronote item 583] ; au-delà, c'est une **correction** (§9.6).

### 9.3 Pré-remplissage [R]

Calculé à l'ouverture et **rafraîchi** jusqu'à la validation (une déclaration de 8 h 05 apparaît dans l'appel de 8 h encore ouvert) : déclarations actives couvrant la séance, dispenses de présence, localisations en cours, participants partis d'une sortie, PFMP, examens, missions, exclusions-inclusions, autorisations d'absence. Le professeur voit l'état proposé et un libellé court **sans motif**.

### 9.4 Hors ligne [R]

**Besoin** : gymnase, terrain de sport, sortie, salle sans réseau, coupure (irritants terrain, [V] guide Pronote : « sauvegarder régulièrement si la connexion est instable »). Aucun logiciel du marché lu ne documente un appel hors ligne [V-ext, négatif non exhaustif] : c'est un point de différence.

1. **Préparation** : chaque matin (et à l'ouverture de l'appli), le téléphone télécharge les séances du jour de la personne et leurs **effectifs minimaux** (nom d'usage, prénom, identifiant interne, photo seulement si activée et déjà visible de lui) et les situations connues ; rien d'autre (ni motif, ni historique, ni coordonnées). Cache chiffré par la clé de l'appareil, **effacé à minuit** heure de l'établissement et à la révocation de la session (D-011).
2. **Saisie** : chaque geste est une **opération** horodatée (`operation_id` UUID généré sur l'appareil, séance, élève, état, heure de l'appareil, version de base de la ligne) dans un journal local. L'écran indique « enregistré sur le téléphone, non envoyé » et le nombre d'opérations en attente.
3. **Envoi** : au retour du réseau, envoi **par lot** ; le serveur applique chaque opération **une seule fois** (`operation_id` unique, E3) ; il **relit les droits** à la réception (la personne doit être intervenante de la séance à sa date) ; il refuse une opération sur une séance d'un autre jour que la veille ou le jour même (S-469).
4. **Conflits** : si la ligne a changé côté serveur depuis la version de base (autre co-intervenant, vie scolaire qui a localisé l'élève), l'opération n'écrase pas : elle est appliquée si elle est compatible (absent hors ligne + localisé infirmerie au serveur → localisé gagne, avec mention), sinon mise en **conflit** visible de l'auteur et de la vie scolaire, qui tranche (AS-134). Règle : **une information de localisation ou de présence constatée par un adulte gagne sur un « absent » non encore validé** (R-AS10).
5. **Avis aux familles** : un appel validé hors ligne n'envoie ses avis qu'à la réception par le serveur ; si la réception arrive après la fin de la demi-journée, l'avis précise l'heure de la séance (« absent au cours de 8 h ») ; la vie scolaire voit « appel validé hors ligne à 8 h 04, reçu à 9 h 37 ».
6. **Web** : la feuille d'appel web garde la saisie locale (stockage de session) et envoie **un seul lot** à la validation (fin de DA-22 : plus d'envoi ligne par ligne).

### 9.5 Appel non fait, appel non validé [R]

Une séance attend un appel (10 §8.3) ; à `debut + délai` (défaut 15 min, S-470) sans appel validé : relance **automatique** à l'intervenant (push, in-app), une seule fois ; à `debut + 2 × délai` : la séance apparaît dans la file de la vie scolaire « appels non faits », qui peut relancer, appeler la salle ou faire l'appel elle-même (AS-152). Un appel ouvert non validé en fin de séance : rappel. Aucune relance pour une séance annulée, neutralisée (fermeture, grève), ou dont l'intervenant est absent (socle 15 : la séance est « non assurée »).

### 9.6 Corriger un appel [R]

- **L'intervenant**, le jour même : il rouvre sa ligne, change l'état, indique un **motif court** facultatif (« arrivé avec un billet », « erreur de saisie ») ; au-delà du jour même, il **demande** une correction (AS-132) que la vie scolaire accepte.
- **La vie scolaire** (action `appel.corriger`) : à tout moment dans l'année, motif obligatoire.
- **Effets** : l'épisode est recalculé ; un avis d'absence déjà parti est suivi d'un **démenti** (« finalement présent au cours de 8 h », nature `vie_scolaire.absence_dementi`, U2) ; un avis non encore parti (délai de consolidation) est annulé ; un décompte de mois déjà **arrêté** n'est pas réécrit : la correction est inscrite comme « rectification postérieure » et signalée au CPE si elle change un signalement (R-AS16).
- **Historique** : chaque ligne garde la suite de ses états (`ent_appel_ligne_histo` : état, minutes, auteur, heure, canal web/mobile/hors ligne, motif) ; visible de la vie scolaire et de la direction ; l'intervenant voit l'historique de ses propres appels.

### 9.7 Incohérences détectées [R]

Absent puis présent à la séance suivante (retard non déclaré ?) ; présent malgré une déclaration (prévenir la famille ? « l'enfant est finalement venu ») ; présent alors qu'« en sortie » (sortie mal tenue) ; présent à deux séances simultanées ; absent à toutes les séances sauf une (erreur d'effectif ?) ; appel validé en dehors des heures de la séance par un autre que l'intervenant. Chaque incohérence est une tâche de la file de la vie scolaire, jamais une modification automatique.

---

## 10. Absences, motifs, justificatifs, qualification

### 10.1 Déclarer [R]

- **Famille** (responsable avec `peut_justifier_absence`, D-014 ; élève majeur) : avant ou après les faits ; période (jour, demi-journée, heures, plusieurs jours, borne 120 jours) ; motif choisi dans la liste de l'établissement (libellés compréhensibles, langue de la famille) ; texte libre facultatif (court, 500 caractères) ; pièce facultative ; **accusé de réception** immédiat (« bien reçu, transmis à la vie scolaire »). Les consignes du règlement intérieur (délai, pièces demandées) sont affichées **à la déclaration** [V-ext, idée Studybugs/Class Charts].
- **Accueil** : élève (recherche tolérante), appelant identifié (ADM-100 : « correspond à : mère, titulaire » ou « ne correspond à aucun responsable »), mode « aujourd'hui / quelques heures / plusieurs jours » (Next.js), canal (téléphone, visite, courriel), motif dit, texte court. Un appelant non reconnu : déclaration enregistrée comme « signalée par un tiers non vérifié », la vie scolaire rappelle un responsable (R-AS6).
- **Messagerie** : un message d'une famille (« Léo sera absent demain matin ») propose une carte « déclarer l'absence » à la vie scolaire, qui confirme (16 MSG-171, IA N2) ; la pièce jointe devient le justificatif sans copie.
- **Anti-doublon** : une déclaration identique (même élève, même période, même source) dans l'heure est fusionnée (DA-22, RP1-18) ; deux déclarations qui se chevauchent sont rapprochées en une seule situation.

### 10.2 Les motifs [R]

Deux niveaux :
1. **Nature nationale** (fixée par ScolIA, non modifiable) : les cinq natures de L131-8 (`maladie_enfant`, `maladie_contagieuse_famille`, `reunion_solennelle_famille`, `difficulte_accidentelle_communications`, `suit_responsables`) ; plus des natures **non légales** pour classer : `rendez_vous_medical`, `convocation_officielle` (JDC, tribunal, préfecture), `examen_exterieur`, `fete_religieuse`, `transport`, `raison_familiale_autre`, `personnelle`, `refus_scolaire`, `inconnu`. Seules les cinq premières peuvent être qualifiées `reputee_legitime` (RA-07) ; les autres sont `excuse_valable` ou `non_valable`.
2. **Motif de l'établissement** : libellé local rattaché à une nature (« Rendez-vous orthodontiste » → `rendez_vous_medical`), liste **courte** (S-472), historisée, jamais supprimée quand elle a servi.

Les motifs techniques (`appel`, `reprise`) ne sont **jamais** proposés aux familles (DA-20).

### 10.3 Justificatifs [R]

Pièce **facultative** (RA-08) : mot des parents, certificat, convocation, attestation. Déposée par la famille, l'élève majeur, l'accueil ou la vie scolaire ; objet immuable `{etablissement}/assiduite/{uuid}` avec empreinte (E5, socle 7) ; classée **C3** si elle est médicale (case cochée par le déposant ou par la vie scolaire, défaut : médicale si la nature du motif est `maladie_*` ou `rendez_vous_medical`) ; lisible de la vie scolaire et de la direction, **jamais** du professeur ; conservée **un an** (RA-34), puis détruite avec trace d'existence. Option de l'établissement « certificat vu, non conservé » : la vie scolaire coche « certificat présenté » sans déposer la pièce (minimisation, M-2531).

### 10.4 Qualifier [R]

- **Qui** : la CPE (action `absences.qualifier`), la direction ; l'AED peut **proposer** (action `absences.proposer_qualification`) si le rôle de l'établissement le prévoit.
- **Quoi** : une qualification par épisode (`reputee_legitime` si la nature est l'une des cinq et que rien ne le contredit, `excuse_valable`, `non_valable`, `sans_motif` si aucun motif n'a été donné après relance) ; une note interne facultative (jamais visible de la famille, R-AS22) ; un **libellé famille** issu d'une liste fermée (« Absence justifiée », « Motif non retenu : … » parmi des formulations fixes, D-023), jamais la note interne.
- **Qualification en masse** : sélection d'épisodes (ex. dix absences « maladie » déclarées de la semaine) → aperçu → confirmation.
- **Requalification** : motif obligatoire, historique, avis U0 à la famille si le sens change.
- **Absence partiellement justifiée** : la famille justifie le matin, pas l'après-midi : l'épisode du matin et celui de l'après-midi sont distincts (contiguïté interrompue par la pause ? non : un même épisode peut être **scindé** par la vie scolaire, AS-127).

### 10.5 Relancer la famille [R]

Pour un épisode `a_qualifier` sans motif déclaré : relance graduée [R, idée parité Pronote item 237 et irritant terrain] — J0 : avis d'absence (§14) ; J+1 (jour scolaire) : rappel in-app et push « motif attendu » ; J+3 : SMS si le canal est ouvert (Q-N5) ou courriel ; J+5 : la vie scolaire voit « sans réponse » et lance un **courrier postal** (RA-04, trame D-023) ; J+8 : qualification `sans_motif` proposée à la CPE, et, si l'élève a moins de 16 ans, déclencheur 1° de L131-8 « motif non communiqué malgré invitation » proposé au dossier (AS-171). Délais réglables par un seul paramètre « rythme de relance » (S-473).

---

## 11. Retards, billets d'entrée, passages, localisation

- **Retard à l'appel** : minutes calculées (modifiables) ; un retard au-delà d'un seuil (défaut : moitié de la séance, S-474) est **proposé** comme absence à la séance, jamais converti seul.
- **Retard à la vie scolaire** (billet d'entrée) : l'élève arrive après l'appel ; l'AED ou l'accueil enregistre l'arrivée (heure), ce qui **corrige** la ligne de l'appel en cours (`absent` → `retard`) sans action du professeur, et imprime ou affiche un **billet d'entrée** (numéro, heure, séance d'admission) [H, pratique courante] ; le professeur voit l'élève passer en « retard (billet) ».
- **Compteurs séparés** : retards par période, minutes cumulées, retards répétés → alerte au CPE (seuil réglable, S-475) ; jamais dans le décompte des demi-journées (VADE [V]).
- **Motif de retard** : liste courte (transport, rendez-vous, panne, personnel) ; un retard de transport collectif signalé (car en retard) se déclare **en lot** pour tous les élèves de la ligne (AS-145) et n'est compté ni au compteur ni aux avis.
- **Passage à la vie scolaire, au CDI, en entretien** : localisation datée (§8.2), **sans motif** visible du professeur.
- **Infirmerie** : l'infirmière (ou, en N-0, la vie scolaire) localise l'élève ; si l'élève était noté absent sans avis parti, l'absence est **remplacée** par la localisation (M-2506, CS-65) ; renvoi au domicile : absence des séances restantes avec nature `maladie_enfant` qualifiée d'office `reputee_legitime` si un responsable l'a récupéré (34 SAN-145), motif « santé » visible de la vie scolaire seule.
- **Sortie anticipée autorisée** (socle 14) : séances restantes en `attendu_ailleurs (sortie autorisée)` si la famille a récupéré l'élève avec un motif, sinon absence ordinaire.

---

## 12. Dispenses

- **Distinguer** (RA-27) : l'**inaptitude** (certificat médical, totale ou partielle, dates, au plus l'année scolaire, jamais rétroactive ; socle 34, C3) et la **dispense de présence** (décision de la vie scolaire ou de la direction, socle 12). Par défaut, l'élève inapte est **présent** au cours d'EPS (`present_dispense`) ; la dispense de présence est une décision explicite (ex. inaptitude totale de longue durée, piscine, dernière heure de la journée).
- **Qui pose** : la vie scolaire, l'infirmière (pour l'inaptitude, au 34), la direction. **La famille et l'élève demandent**, ils ne posent pas (DA-16) ; une demande ponctuelle par mot des parents (« pas de piscine aujourd'hui ») est décidée par le professeur d'EPS (34 CS-32) : AS-162.
- **Portée** : matière (EPS), activité (natation), cours précis, période ; avec ou sans présence requise.
- **À l'appel** : `present_dispense` ou `attendu_ailleurs (dispensé)` ; l'absence d'un élève dispensé de présence n'est pas comptée ; l'absence d'un élève inapte mais **attendu** est une absence ordinaire.
- **Contrôles** : inaptitude au-delà de trois mois → alerte au médecin scolaire (R312-3), via le socle 34 ; date de fin au-delà de l'année scolaire refusée ; certificat daté après la séance → ne justifie pas rétroactivement (RA-27).
- **Visibilité** : le professeur d'EPS voit « dispensé de présence » ou « inapte partiel : pas de course » (34 §382) ; les autres professeurs ne voient rien ; aucune pièce médicale au 12.

---

## 13. Décompte, compteurs, suivi et signalement

### 13.1 La règle d'assiduité, versionnée [R]

Une **version** de règle par établissement, valable à partir d'une date (souvent la rentrée), **jamais modifiée** une fois appliquée à un mois arrêté (DA-11) : coupure (lue au socle 5, STR-35, avec repli), **définition de la demi-journée** (une des trois formules, Q-AS3) — (a) **au moins une séance manquée** (défaut recommandé) ; (b) au moins N minutes manquées (l'existant : 30 min) ; (c) toute la demi-journée — ; type de mois (civil par défaut, glissant si le profil académique l'exige) ; profil académique ; seuils internes (alerte « absences répétées », retards). Modifier = créer une version datée (action `assiduite.regle_gerer`, **direction**), avec aperçu de l'effet sur le mois en cours.

### 13.2 Le calcul [R]

Fonction pure (reprise de `assiduite.rs`) : pour un élève et un jour, les constats des séances **attendues** (situations exclues), rattachés aux épisodes et à leurs qualifications, ventilés par la coupure, selon la version de règle à la date, **en tenant compte du calendrier** (vacances, fériés, fermetures, mercredi après-midi sans cours : DA-14). Sorties : demi-journées manquées, dont non justifiées ; et **l'explication** (« mardi matin compte : absent à 8 h et 9 h, non justifié ; règle : au moins une séance manquée, version du 1/9/2026 »).

**Matérialisation** : le résultat est écrit dans `ent_asd_demi_journee` (recalculable, idempotent : « recalculer tout ne change rien ») et **figé** au premier jour du mois suivant (`ent_asd_mois_arrete`) ; les rectifications postérieures sont listées à part (R-AS16).

### 13.3 Le dossier annuel d'assiduité (R131-6) [R]

Un dossier par élève et par année scolaire, ouvert à la première absence : épisodes (durée, motif déclaré, qualification), retards, **contacts avec la famille** (date, canal, qui, résultat : joint, message laissé, sans réponse, rendez-vous), courriers envoyés, entretiens, commission éducative et document signé (socles 13, 7), référent désigné, engagements et échéances, signalements et leur suite. **Accès de la famille** (RA-10) : la chronologie de son enfant, sans les notes internes ni les textes de l'autre parent. Clos en fin d'année ; versé aux archives selon §15.6 ; **pas d'accumulation inter-années** dans le dossier (RA-09), l'historique pluriannuel n'étant visible que de la CPE et de la direction en lecture d'archive.

### 13.4 Seuils et alertes [R]

| Alerte | Règle | Destinataires | Effet |
|---|---|---|---|
| Première absence non justifiée | dès la première séance `sans_motif` après relance (RA-17) | CPE, AED | tâche « contacter la famille, convoquer l'élève » |
| Absences répétées (même justifiées) | N épisodes en M semaines (défaut 4 en 4 semaines, S-476) (RA-09) | CPE, PP | proposition d'entretien |
| **Seuil légal** | ≥ 4 demi-journées non justifiées dans le mois (RA-11), moins de 16 ans **et** plus de 16 ans (suivi), selon la version | CPE, direction | **procédure** : dialogue, commission éducative, document signé, proposition de signalement |
| Motif non communiqué malgré invitation | relance arrivée au bout (§10.5) (L131-8 1°) | CPE | proposition au dossier |
| Persistance | profil académique (défaut : 10 demi-journées dans le mois, RA-12 indicatif) | CPE, direction | référent, engagement, nouveau signalement |
| Tendance | hausse marquée d'un mois sur l'autre (déterministe, explicable) | CPE, PP | information douce, jamais une étiquette |
| Retenue de bourse | > 15 jours d'absences injustifiées cumulés (RA-28) | direction, intendance | export au socle 26 |

Aucune alerte n'écrit à la famille d'elle-même (DA-07) : elles créent des **tâches** ; les courriers partent après décision humaine.

### 13.5 Profil académique [R]

Paramètres d'académie (choix d'une liste fournie et mise à jour par ScolIA, modifiable par l'établissement avec trace) : niveaux de signalement et leurs seuils, mois civil ou glissant, pièces exigées par niveau (compte rendu du GPDS à Paris), outil de transmission (ADESCO, MIMOSA, courrier), âge au-delà duquel le niveau « procureur » est désactivé (16 ans, RA-14), destinataire (DASEN, coordinateur MLDS pour les plus de 16 ans). **Aucun outil académique n'est appelé** : ScolIA prépare, la personne transmet et note « transmis le …, par …, outil … ».

### 13.6 Le signalement [R]

Le CPE ouvre la proposition depuis l'alerte ; le dossier est **pré-rempli** (identité réduite, responsables, décompte du mois **figé** avec son explication, chronologie des absences et des contacts, mesures prises, référent, avis de l'équipe) ; la direction relit et **valide** (action `assiduite.signalement_valider`, ré-authentification) ; l'export suit la trame du profil (PDF, D-023, et CSV) ; la transmission est **faite par une personne** et tracée ; si une absence du mois est requalifiée après validation, la direction est alertée (« le décompte du signalement de mars passe de 5 à 3 ») et décide d'une rectification (constat I16 de l'audit).

### 13.7 Statistiques et rapports [R]

Taux d'absentéisme par classe, niveau, jour, heure, matière (sans désignation d'un professeur dans les tableaux de pilotage, R-AS27), par motif (natures), part des absences prévenues, délai moyen de régularisation, appels non faits par semaine (au chef d'établissement seulement, jamais publié nominativement) ; **rapport annuel au CA** prérempli (RA-20) ; agrégats pour le socle 47 ; export pour le **bulletin** (demi-journées, dont non justifiées, retards, par période ; socle 23) ; enquêtes académiques (comptages par mois, sans nom) ; **jamais** de classement d'élèves publié.

---

## 14. Familles et notifications

### 14.1 Ce que la famille reçoit et voit [R]

- **Avis d'absence** (`vie_scolaire.absence_constatee`, U2, 8 §4.1) : **un par foyer et par demi-journée** (regroupé par fratrie), après le délai de consolidation, avec les séances concernées et un bouton « donner le motif » ; jamais par séance (DA-05).
- **Avis de retard** (U1, résumé possible).
- **Démenti** (`vie_scolaire.absence_dementi`, U2) si l'absence est corrigée après l'avis.
- **Décision** (`vie_scolaire.absence_decision`, U0) quand la qualification change le sens.
- **Relance** (`vie_scolaire.relance_justificatif`) selon §10.5.
- **Seuil** (`vie_scolaire.seuil_assiduite`, C2) : **seulement** quand la CPE ou la direction l'envoie (convocation à un entretien), jamais automatiquement.
- **Espace famille** : liste des absences par épisode (pas par séance) avec état clair (« en attente de votre motif », « justifiée », « motif non retenu »), retards, dispenses, dossier annuel, contacts de la vie scolaire.

### 14.2 Délai de consolidation [R]

R131-5 exige l'information **immédiate** (RA-03) ; un avis faux (l'élève était à l'infirmerie, arrivé avec un billet) coûte cher en confiance. Défaut : **15 minutes après la validation de l'appel**, réglable de 0 à 30 minutes (S-477) ; supprimé pour une absence inquiétante (« prévenir maintenant », U3, 8 §1498) ; l'avis part **à la fin** du délai si rien ne l'a annulé. C'est un choix d'exactitude, pas un retard : il est écrit dans la notice et au règlement intérieur [R].

### 14.3 Destinataires [R]

- **Mineur** : les responsables de l'élève selon le socle 4 (`peut_voir_vs`, autorité parentale ou non, RA-26) ; **foyer du jour d'abord** (résidence alternée), paramètre « les deux » (8 §4.1) ; personne sous interdiction : rien ; adresse protégée : jamais l'adresse de l'autre.
- **Majeur** : l'élève ; ses parents selon la politique de l'établissement (Q-AS9 : par défaut maintenue tant qu'ils restent responsables financiers déclarés, l'élève en est informé et peut demander à la direction de la suspendre ; décision tracée ; DPO).
- **AESH** de l'élève accompagné : avis « Léo est absent ce matin » sans motif (M-2376).
- **Famille d'accueil, tiers digne de confiance, ASE** : selon les droits du lien (socle 4).

### 14.4 Contacts avec la famille [R]

Chaque appel téléphonique, SMS, courriel, rencontre est un **contact** tracé en un geste (« joint », « message laissé », « ne répond pas », « numéro faux ») sur l'épisode et dans le dossier ; visible de toute l'équipe de vie scolaire en temps réel (fin des doubles appels) ; le bouton **Appeler** ordonne les numéros selon `joignables` (4). Le numéro faux déclenche une tâche au secrétariat (socle 4).

### 14.5 Natures à ajouter au socle 8 [R]

`vie_scolaire.absence_dementi` (U2, C1), `vie_scolaire.relance_justificatif` (U1, C1), `vie_scolaire.absence_eleve_accompagne` (U1, C1, personnel AESH), `vie_scolaire.appel_a_faire` (U1, personnel), `vie_scolaire.appel_non_fait` (U1, vie scolaire), `vie_scolaire.conflit_hors_ligne` (U1, personnel).

### 14.6 Accessibilité, langues, sans smartphone [R]

Formulaire de déclaration lisible au lecteur d'écran, en langue de la famille (socle 4) ; déclaration par **SMS entrant** ou **lien nominatif** sans compte (D-027 G) pour les familles sans application [R, socle 8] ; le téléphone de l'accueil reste un canal de premier rang.

---

## 15. Droits et minimisation

### 15.1 Catalogue d'actions proposé (D-017) [R]

| Action | Portée | Modèles de rôle par défaut |
|---|---|---|
| `appel.faire` | séances dont on est intervenant à la date (`soi`) | professeur, remplaçant, AED (permanences), documentaliste, animateur |
| `appel.faire_tout` | établissement | CPE, AED (si le rôle le prévoit), direction |
| `appel.corriger` | établissement | CPE, direction |
| `appel.verifier` | établissement | CPE, AED, direction |
| `appel.deleguer` | soi | professeur |
| `absences.lire` | `mes_cours` (D-022) / établissement | professeur (`mes_cours`, sans motif) ; CPE, AED, direction (établissement) |
| `absences.lire_motif` | établissement | CPE, AED, direction |
| `absences.declarer` | établissement | accueil, AED, CPE, direction ; **pas** professeur |
| `absences.qualifier` | établissement | CPE, direction |
| `absences.proposer_qualification` | établissement | AED |
| `absences.justificatif_lire` | établissement | CPE, direction ; AED (non médical) |
| `retards.enregistrer` | `mes_cours` à la séance / établissement | professeur (`mes_cours`) ; AED, accueil, CPE |
| `localisation.poser` | établissement | AED, CPE, infirmerie, documentaliste, direction |
| `dispenses.gerer` | établissement | CPE, direction ; professeur d'EPS pour la dispense ponctuelle de ses cours |
| `assiduite.suivre` (dossier annuel, contacts) | établissement | CPE, AED (contacts), direction ; PP en lecture `ma_classe` sans motif |
| `assiduite.signalement_proposer` | établissement | CPE |
| `assiduite.signalement_valider` | établissement, ré-authentification | direction |
| `assiduite.regle_gerer` | établissement | direction |
| `assiduite.statistiques` | établissement | direction, CPE |
| `assiduite.exporter` (registre, signalement, bourses, CA) | établissement, D-022 | direction, secrétariat ; jamais professeur |

Famille et élève hors rôles (D-017) : responsable avec `peut_voir_vs` (lire), `peut_justifier_absence` (déclarer, justifier) ; élève mineur : lire ses absences et retards, **ne déclare pas** (RP1-09) ; élève majeur : déclarer et justifier.

### 15.2 Ce que voit chacun [R]

| Qui | Voit | Ne voit jamais |
|---|---|---|
| Professeur | ses appels ; pour les élèves de ses cours : épisodes et retards de **ses** séances, état de régularisation de leurs absences récentes (« à régulariser » / « régularisée »), localisation codée pendant sa séance | un motif, un justificatif, une note interne, le dossier annuel, les absences des élèves hors de ses cours |
| Professeur principal | en plus, l'assiduité agrégée de sa classe (demi-journées, retards, tendance) et l'existence d'un dossier de suivi | motifs, contacts, notes |
| AED | file du jour, appels de permanence, contacts, motifs déclarés non médicaux | justificatifs médicaux, notes internes de la CPE (sauf choix du rôle), signalements |
| Accueil | la déclaration qu'il saisit, « en sortie aujourd'hui », « prévenu » pour l'élève recherché (lecture unitaire journalisée) | l'historique de l'élève, les motifs des autres déclarations |
| CPE, direction | tout le domaine | — (lecture des pièces médicales journalisée) |
| Infirmière | ses localisations, le fait « absent ce matin » d'un élève qu'elle cherche | le reste |
| AESH | absence de l'élève accompagné (avis) | motif, autres élèves |
| Famille | tout ce qui concerne son enfant, contacts et courriers reçus, dossier | notes internes, texte écrit par l'autre responsable (elle voit « motif déclaré par un autre responsable : maladie »), noms des autres élèves |
| Élève | ses absences, retards, dispenses, dossier | notes internes, contacts entre adultes (option : contacts visibles de l'élève majeur) |

### 15.3 Garde-fous [R]

- Toute route nominative vérifie l'**action** et la **portée** côté serveur (D-017, D-022) ; la matrice de séparation (§25) teste le professeur hors de ses groupes sur **toutes** les routes d'écriture (fin de DA-01 à DA-03).
- Lecture d'un justificatif médical, d'un dossier, d'un signalement : **journalisée** (E7) et visible du DPO.
- Exports nominatifs : action `assiduite.exporter`, trace `ent_export_trace` (socle 9), jamais par un professeur (D-022).
- Le payload des événements ne contient **aucun texte libre** ni motif déclaré (DA-21) : seulement des codes et identifiants.
- Le cache hors ligne ne contient que l'effectif minimal du jour (§9.4).

### 15.4 Matrice de séparation (tests) [R]

Élève A contre élève B ; parent de la famille 1 contre l'enfant de la famille 2 ; parent séparé (texte de l'autre parent, interdiction, adresse protégée) ; professeur d'allemand contre élève d'espagnol (lecture, déclaration, retard, correction, « remettre présent », justificatif) ; remplaçant hors de sa période ; AED qui qualifie sans le droit ; accueil qui lit l'historique ; professeur qui exporte ; autre établissement sur chaque route ; appareil révoqué qui envoie un lot hors ligne ; opération hors ligne rejouée.

### 15.5 Données personnelles [R]

Base légale : obligation légale (R131-5, R131-6, L131-8 ; RGPD 6.1.c) pour l'appel, l'information des familles et le dossier ; mission d'intérêt public (public) / intérêt légitime ou contrat (privé) pour les statistiques [à valider DPO]. Données de santé indirectes : nature « maladie », motif déclaré, certificats (C3, 34 §1177). Droit d'opposition sans effet sur l'information due (VERIF R3) ; droit d'accès servi par le dossier annuel ; rectification par la correction et la requalification tracées.

### 15.6 Conservation [R, à valider par le DPO]

| Donnée | Base active | Ensuite |
|---|---|---|
| Registre d'appel (constats, sans motif) | année scolaire + 1 an | archive restreinte jusqu'à **10 ans** (RA-34), puis destruction |
| Épisodes, qualifications, demi-journées | année scolaire + 1 an | agrégats conservés ; détail nominatif détruit (sauf dossier versé à un signalement) |
| Motif déclaré en texte libre, notes internes | année scolaire | effacés (trace d'existence) |
| Justificatifs (pièces) | **1 an** (RA-34) | destruction, trace d'existence |
| Dossier annuel et signalements | année scolaire + 1 an | archive restreinte selon le service d'archives (à fixer) |
| Journal hors ligne (opérations) | 3 mois | supprimé |
| Contacts avec la famille | année scolaire + 1 an | détruits |

---

## 16. Cas limites (catalogue)

| N° | Cas | Traitement |
|---|---|---|
| CA-01 | Élève absent toute la journée | un épisode, un avis par demi-journée et par foyer, une justification |
| CA-02 | Absent à 8 h, présent à 10 h, absent à 14 h | deux épisodes ; alerte `absence_apres_presence` à 14 h |
| CA-03 | Prévenu absent par l'accueil, finalement venu | le professeur le marque présent ; la déclaration est close « venu » ; avis « finalement présent » si la famille l'avait déclaré |
| CA-04 | Arrivé à 8 h 20 avec un billet, appel validé à 8 h 05 avec « absent » | billet → ligne corrigée en retard ; avis annulé si dans le délai, sinon démenti |
| CA-05 | Noté absent, en fait à l'infirmerie | localisation remplace l'absence avant l'avis ; après l'avis : démenti |
| CA-06 | Appel fait hors ligne au gymnase, reçu à 10 h | avis à la réception, heure de la séance précisée ; constat daté de la saisie |
| CA-07 | Deux co-intervenants font l'appel de la même séance, l'un hors ligne | le premier validé fait foi ; les opérations du second appliquées si compatibles, sinon conflit |
| CA-08 | Séance annulée après l'appel (professeur parti malade à 9 h) | constats neutralisés ; épisodes recalculés ; démenti si un avis est parti |
| CA-09 | Cours déplacé au jeudi | l'appel s'ouvre depuis la séance déplacée |
| CA-10 | Remplaçant posé après l'ouverture de l'appel par le titulaire | l'appel reste accessible au titulaire jusqu'à la validation ; le remplaçant le voit |
| CA-11 | Élève arrivé dans la classe ce matin (inscription du jour) | ajouté à l'effectif jusqu'à la validation ; mention « ajouté » |
| CA-12 | Élève parti hier (exeat) | retiré de l'effectif ; s'il reste dans un appel validé, la ligne est neutralisée |
| CA-13 | Élève de groupe de langue dans une classe mixte | effectif = groupe (D-022, RP1-24) |
| CA-14 | Élève en inclusion ULIS dans un cours de la classe | dans l'effectif du cours (socle 5) ; absent de l'ULIS pendant ce temps |
| CA-15 | Élève en sortie scolaire d'un autre cours | `attendu_ailleurs (sortie)` ; présent marqué → incohérence |
| CA-16 | Absent au départ de la sortie | redevient absent de ses séances maintenues ; vie scolaire prévenue (42) |
| CA-17 | Élève en PFMP | séances non attendues ; absences en entreprise saisies par le référent |
| CA-18 | Délégué au conseil de classe pendant un cours | `mission_representation` : ni absence ni bulletin (RA-19) |
| CA-19 | Élève en exclusion-inclusion | `attendu_ailleurs (vie scolaire)` ; aucun avis d'absence |
| CA-20 | Élève exclu temporairement de l'établissement | séances non attendues (socle 13) ; pas d'absence |
| CA-21 | Inaptitude totale d'EPS, présence requise | `present_dispense` ; absent = absence ordinaire |
| CA-22 | Dispense de présence piscine | `attendu_ailleurs (dispensé)` ; absent ne compte pas |
| CA-23 | Certificat d'inaptitude daté après la séance | ne justifie pas rétroactivement (RA-27) ; l'absence peut être qualifiée excuse valable par la CPE sur un autre fondement |
| CA-24 | Famille déclare « malade » trois jours, revient le 2ᵉ jour | séances du 3ᵉ jour : présent ; déclaration close à la dernière séance manquée |
| CA-25 | Déclaration famille pendant les vacances | aucune séance attendue : déclaration sans effet, informée « aucun cours sur cette période » |
| CA-26 | Déclaration faite le lendemain | rapprochée des constats ; le motif s'applique à l'épisode |
| CA-27 | Deux parents séparés déclarent deux motifs différents | deux déclarations rattachées à l'épisode ; la CPE voit les deux, chaque parent voit la sienne et « un autre responsable a déclaré » |
| CA-28 | Parent sous interdiction tente de déclarer | refus silencieux pour lui (403) ; rien n'est révélé |
| CA-29 | Appel d'un inconnu à l'accueil « je suis son oncle » | déclaration « tiers non vérifié » ; tâche de rappel d'un responsable |
| CA-30 | Élève majeur qui déclare ses absences | accepté ; parents informés selon Q-AS9 |
| CA-31 | Élève devient majeur en cours d'année | politique appliquée à la date ; épisodes passés inchangés |
| CA-32 | Élève de 16 ans révolus au seuil | suivi et procédure interne ; niveau « procureur » désactivé (RA-14) |
| CA-33 | Étudiant de BTS boursier absent 3 semaines | profil étudiant ; justificatif demandé avant toute liste CROUS ; < 1 mois : pas de suspension (RA-29) |
| CA-34 | Apprenti en section d'apprentissage | export des absences pour l'employeur (RA-31) |
| CA-35 | Absence lors d'un devoir surveillé | `absent_a_seance` lu par le socle 21 ; « jamais de zéro » (21 CE-26) |
| CA-36 | Absence d'un mercredi après-midi sans cours | aucune séance : aucune demi-journée |
| CA-37 | Grève des transports : 40 élèves en retard | retard en lot « transport », hors compteur et sans avis |
| CA-38 | Neige : établissement fermé dans la journée | séances neutralisées (socle 5/10) ; constats du matin conservés, ceux de l'après-midi neutralisés |
| CA-39 | Changement de la règle d'assiduité en janvier | nouvelle version datée ; septembre à décembre inchangés |
| CA-40 | Absence requalifiée après un signalement émis | alerte à la direction ; rectification décidée |
| CA-41 | Professeur oublie de valider ; la vie scolaire fait l'appel à sa place | appel fait par la vie scolaire (auteur noté) ; relance éteinte |
| CA-42 | Professeur fait l'appel de la mauvaise séance (8 h au lieu de 9 h) | correction : la vie scolaire déplace les constats vers la bonne séance (AS-133) |
| CA-43 | Un élève est sur deux séances simultanées (erreur d'EDT) | incohérence ; le socle 10 est prévenu |
| CA-44 | Téléphone perdu avec un appel hors ligne non envoyé | session révoquée ; l'appel non reçu reste « à faire » ; la vie scolaire refait l'appel |
| CA-45 | Lot hors ligne reçu deux fois | idempotence par `operation_id` |
| CA-46 | Lot hors ligne d'un appareil révoqué | refusé ; tâche de vérification à la vie scolaire |
| CA-47 | Opération hors ligne datée d'il y a trois jours | refusée (S-469) ; la vie scolaire saisit en correction |
| CA-48 | Établissement ultramarin (UTC−3 ou UTC+4) | fuseau de l'établissement partout (DA-13) |
| CA-49 | Passage à l'heure d'été la nuit précédente | heures murales stables (socle 10) |
| CA-50 | Absence inquiétante (fugue, élève signalé fragile) | « prévenir maintenant » : U3, sans délai ; trace |
| CA-51 | Élève sous protection (adresse protégée, ASE) | destinataires du socle 4 ; jamais l'adresse dans un courrier à l'autre parent |
| CA-52 | Élève décédé ou disparu | plus d'avis ; appels neutralisés à la date (socle 2) |
| CA-53 | Élève en mobilité trois mois au Canada | séances non attendues (ELV-35) ; pas d'absence |
| CA-54 | Élève accueilli temporairement (correspondant) | sur les listes d'appel (STR-102) ; pas de responsables ScolIA : avis à la personne d'accueil désignée |
| CA-55 | Fête religieuse demandée à l'avance | autorisation (AS-125) : absence qualifiée excuse valable, aucun avis d'absence |
| CA-56 | Demande d'absence systématique le vendredi après-midi (activité sportive) | refus motivé (RA-32), ou aménagement d'emploi du temps (socle 10, sportif de haut niveau) |
| CA-57 | Justificatif déposé, puis la famille le retire | la pièce reste (immuable) mais n'est plus « présentée » ; trace |
| CA-58 | Certificat médical contenant un diagnostic | pièce C3 ; jamais lue par l'IA ni le professeur |
| CA-59 | Appel de la demi-pension (option) | appel d'activité ; absence à la cantine n'est **pas** une absence scolaire (compteur à part, socle 40) |
| CA-60 | Permanence non prévue (professeur absent) | effectif attendu = élèves sans cours (EDT-219) ; appel par l'AED |
| CA-61 | Élève sorti de l'établissement sans autorisation pendant la récréation | absence après présence (CA-02) ; alerte immédiate |
| CA-62 | Panne générale du serveur à 8 h | appels hors ligne sur mobile ; feuille d'appel imprimable de secours chaque matin (option, S-478) ; saisie au retour |
| CA-63 | Deux appels simultanés de la même séance (deux onglets) | même appel ; lignes avec version ; la dernière validation fait une correction tracée |
| CA-64 | Élève inscrit à une option facultative qui ne vient plus | absences comptées (RA-23) jusqu'à désinscription (socle 5) |
| CA-65 | Absence le jour d'une réunion solennelle (mariage d'un frère) | nature `reunion_solennelle_famille` → réputée légitime |
| CA-66 | Rendez-vous médical d'une heure | `rendez_vous_medical` → excuse valable (non « réputée légitime ») selon la CPE |
| CA-67 | Mois à cheval (absence du 30 au 2) | demi-journées imputées à chaque mois civil |
| CA-68 | Élève boursier : 16 jours injustifiés au 2ᵉ trimestre | export au socle 26, imputé au trimestre du franchissement (RA-28) |
| CA-69 | Professeur qui consulte l'historique d'absences d'un élève d'un autre groupe | 403, journalisé |
| CA-70 | Parent qui modifie la déclaration après qualification | nouvelle version ; la CPE est prévenue ; la qualification reste jusqu'à nouvelle décision |
| CA-71 | Établissement qui ne veut pas de délai de consolidation | réglage 0 min |
| CA-72 | Reprise d'un historique Charlemagne (absences à la journée) | épisodes « reprise », demi-journées selon le calendrier réel, qualification reprise si connue |

---

## 17. Actions métier (AS-100 et suivantes)

Colonnes : action ; qui et droit (§15.1) ; règles et garde-fous ; événement émis (§19) ; correspondance avec `SOCLE2_DOMAINES.md` / la matrice et l'existant (**I** implémenté, **P** partiel, **A** absent). ★ = confirmation forte ou ré-authentification.

### 17.1 Faire l'appel

| N° | Action | Qui · droit | Règles | Événement | Corresp. |
|---|---|---|---|---|---|
| AS-100 | Voir ses appels du jour (à faire, en cours, faits, en retard) | intervenant · `appel.faire` | séances du jour du socle 10 où il est intervenant à la date | — | VS-01 · P |
| AS-101 | Ouvrir l'appel **depuis la séance** | idem | idempotent ; effectif = public de la séance à la date ; refus sur séance annulée ou neutralisée | `appel.ouvert` | VS-01 · I (créneau) |
| AS-102 | Pointer « tous présents puis exceptions » | idem | situations connues affichées en tête | — | I |
| AS-103 | Pointer au trombinoscope (un geste par élève) | idem | photo seulement si activée (S-468) et déjà visible de lui | — | A |
| AS-104 | Appel « présents seulement » (petit groupe) | idem | les non pointés deviennent absents à la validation, avec confirmation | — | A |
| AS-105 | Déléguer l'appel d'une séance à un collègue (datée, motif court) | intervenant · `appel.deleguer` | une séance, un jour ; visible de la vie scolaire | `appel.delegue` | A |
| AS-106 | Valider l'appel (récapitulatif, confirmation) | intervenant · `appel.faire` | lot unique ; horaires figés ; épisodes et retards dans la transaction | `appel.valide` | VS-02 · I |
| AS-107 | Saisir un retard à l'appel (minutes calculées) | idem | minutes modifiables ; proposition d'absence au-delà de S-474 | `assiduite.retard_enregistre` | VS-12 · P |
| AS-108 | Marquer « présent, dispensé » ou « localisé » | idem | lieu codé seulement | — | A |
| AS-109 | Signaler un élève à la vie scolaire depuis l'appel (« semble malade », « sans carnet ») | idem | texte court ; tâche à la vie scolaire ; jamais visible de la famille | `appel.signalement_vs` | A |

### 17.2 Déclarer

| N° | Action | Qui · droit | Règles | Événement | Corresp. |
|---|---|---|---|---|---|
| AS-110 | Déclarer une absence **à l'accueil** (appel ou visite) | ACC, AED, CPE · `absences.declarer` | appelant identifié (ADM-100) ; modes jour/heures/plusieurs jours ; canal ; anti-doublon | `assiduite.situation_declaree` | VS-06 · I |
| AS-111 | Déclarer **en ligne** (avant ou après) | responsable `peut_justifier_absence` ; élève majeur | borne 120 jours ; consignes affichées ; accusé de réception | idem | VS-07 · I |
| AS-112 | Avertir à la déclaration qu'un élève est « en sortie » ou « attendu ailleurs » | SYS | forçage motivé (repris de Next.js) | — | A (Next.js) |
| AS-113 | Modifier ou annuler sa déclaration | déclarant ; vie scolaire | version ; la CPE est prévenue si déjà qualifiée (CA-70) | `assiduite.situation_modifiee` | I |
| AS-114 | Déclarer depuis un message de la famille (carte) | AED, CPE | confirmation humaine ; la pièce devient justificatif (16) | `assiduite.situation_declaree` | A |
| AS-115 | Déclarer par **lien nominatif** sans compte ou SMS entrant | responsable sans compte | lien à usage limité (D-027 G) ; SMS entrant : Q-AS12 | idem | A |
| AS-116 | Rapprocher déclaration et constat (dans les deux sens) | SYS | séance couverte, bornes incluses | `assiduite.absence_rapprochee` | VS-08 · I |
| AS-117 | Fusionner des déclarations en double | SYS ; AED | même élève, même période, même heure | — | A (RP1-18) |
| AS-118 | Saisir une absence en **PFMP** | enseignant référent · `appel.faire` (stage) | période de stage du socle 39 ; motif ; justificatif | `assiduite.absence_pfmp` | A |
| AS-119 | Enregistrer une absence **sans appel** (activité non appelée, journée banalisée) | CPE · `appel.corriger` | motif obligatoire ; séances attendues de la journée | `assiduite.absence_ouverte` | P (route ancienne) |

### 17.3 Qualifier et justifier

| N° | Action | Qui · droit | Règles | Événement | Corresp. |
|---|---|---|---|---|---|
| AS-120 | Voir les épisodes à qualifier (file, filtres) | CPE, AED · `absences.lire_motif` | tri par ancienneté et urgence | — | VS-10 · I |
| AS-121 | **Qualifier** un épisode (réputée légitime, excuse valable, non valable, sans motif) | CPE · `absences.qualifier` | seules les 5 natures légales en « réputée légitime » ; libellé famille fermé ; note interne | `assiduite.absence_qualifiee` | VS-10 · P |
| AS-122 | Qualifier en masse | CPE | aperçu, confirmation | idem (par épisode) | A |
| AS-123 | Requalifier | CPE, direction | motif obligatoire ; avis U0 si le sens change ; alerte si un signalement est touché | `assiduite.absence_requalifiee` | A |
| AS-124 | Proposer une qualification | AED · `absences.proposer_qualification` | la CPE valide | `assiduite.qualification_proposee` | A |
| AS-125 | **Demander une autorisation d'absence** (fête religieuse, convocation, compétition) ; transmission au DASEN en cas de doute (R131-5) | responsable ; décision direction | délai ; refus motivé ; absences systématiques refusées (RA-32) | `assiduite.autorisation_decidee` | A |
| AS-126 | Joindre un justificatif | responsable, majeur, ACC, AED, CPE | objet immuable (E5) ; médical → C3 | `assiduite.justificatif_depose` | P (texte seul) |
| AS-127 | Scinder ou fusionner des épisodes | CPE | historique ; recalcul | `assiduite.absence_restructuree` | A |
| AS-128 | Noter « certificat présenté, non conservé » | AED, CPE | aucune pièce stockée | — | A |
| AS-129 | Relancer la famille (manuel, ou relance graduée) | AED, CPE ; SYS | rythme S-473 ; courrier au bout | `assiduite.relance_envoyee` | VS-07 matrice · A |

### 17.4 Corriger

| N° | Action | Qui · droit | Règles | Événement | Corresp. |
|---|---|---|---|---|---|
| AS-130 | Voir l'historique d'une ligne d'appel | intervenant (ses appels), CPE, direction | — | — | A |
| AS-131 | **Corriger une ligne** (jour même par l'intervenant ; à tout moment par la vie scolaire) | `appel.faire` / `appel.corriger` | motif (obligatoire pour la vie scolaire) ; effets recalculés ; démenti | `appel.ligne_corrigee` | VS-05 · A |
| AS-132 | Demander une correction après le jour | intervenant | la vie scolaire accepte ou refuse | `appel.correction_demandee` | A |
| AS-133 | Déplacer les constats vers la bonne séance | CPE | même élève, même jour ; historique | `appel.constats_deplaces` | A |
| AS-134 | Trancher un conflit hors ligne | CPE, auteur | R-AS10 | `appel.conflit_tranche` | A |
| AS-135 | Neutraliser les constats d'une séance annulée après coup | SYS sur `edt.seance_annulee` | épisodes recalculés ; démenti | `appel.neutralise` | A (DA-09) |
| AS-136 | Démentir un avis parti | SYS | U2 | `assiduite.absence_dementie` | A |
| AS-137 | Retirer un épisode (« remettre présent ») | CPE · `appel.corriger` | motif ; lignes d'appel corrigées en cohérence ; famille informée | `assiduite.absence_retiree` | VS-11 · I (DA-02) |
| AS-138 | Rouvrir un appel validé | CPE | motif ; rare | `appel.rouvert` | A |
| AS-139 | Corriger une absence reprise (historique) | CPE | motif | `assiduite.absence_corrigee` | A |

### 17.5 Retards, localisation, journée

| N° | Action | Qui · droit | Règles | Événement | Corresp. |
|---|---|---|---|---|---|
| AS-140 | **Localiser** un élève (infirmerie, vie scolaire, CDI, entretien, autre) | `localisation.poser` | lieu codé ; remplace une absence non encore avisée | `assiduite.localise` | A |
| AS-141 | Enregistrer une **arrivée tardive** (billet d'entrée) | AED, ACC | corrige la ligne de l'appel en cours | `assiduite.retard_enregistre` | VS-12 · P |
| AS-142 | Imprimer ou afficher le billet d'entrée | AED, ACC | numéro par établissement (E3) | — | A |
| AS-143 | Convertir un retard en absence (proposition acceptée) | CPE | S-474 | `appel.ligne_corrigee` | A |
| AS-144 | Saisir le motif d'un retard | AED, CPE, famille | liste courte | — | P |
| AS-145 | Retards **en lot** (transport collectif) | AED, CPE | hors compteur, sans avis | `assiduite.retards_lot` | A |
| AS-146 | Refléter une sortie anticipée (socle 14) | SYS | séances restantes `attendu_ailleurs` | — | A |
| AS-147 | Refléter un renvoi au domicile par l'infirmerie (socle 34) | SYS | qualification d'office, motif « santé » vie scolaire seule | `assiduite.absence_ouverte` | A |
| AS-148 | Voir les retards d'un élève | droits §15.2 | `peut_voir_vs` pour la famille (DA-17) | — | I |
| AS-149 | Alerte « retards répétés » | SYS | seuil S-475 ; tâche | `assiduite.alerte` | A |

### 17.6 Vérifier et traiter la journée

| N° | Action | Qui · droit | Règles | Événement | Corresp. |
|---|---|---|---|---|---|
| AS-150 | Voir les **appels non faits** et non validés | `appel.verifier` | séances attendues, hors annulées, neutralisées, non assurées | — | VS-04 · I |
| AS-151 | Relance automatique de l'intervenant | SYS | une fois à `debut + délai` | `appel.relance` | P (manuelle) |
| AS-152 | Faire l'appel à la place de l'intervenant | `appel.faire_tout` | auteur noté ; relance éteinte | `appel.valide` | A |
| AS-153 | Voir et traiter les **incohérences** | `appel.verifier` | §9.7 ; jamais de modification automatique | — | I (2 types) |
| AS-154 | Alerte « absence après présence » | SYS | interne immédiate ; famille après 15 min sauf localisé (8 R-N67) | `assiduite.absence_apres_presence` | A |
| AS-155 | **Prévenir maintenant** (absence inquiétante) | CPE, direction | U3 ; motif tracé | `assiduite.alerte_urgente` | A (8 §1498) |
| AS-156 | **File du matin** (tableau du jour classé par urgence) | AED, CPE | non jointes, mineurs sans nouvelles, élèves signalés fragiles (drapeau sans motif), absences après présence | — | VS-25 · I |
| AS-157 | Enregistrer un **contact** avec la famille (joint, message, sans réponse, numéro faux) | AED, CPE · `assiduite.suivre` | visible de l'équipe en temps réel | `assiduite.contact_trace` | A |
| AS-158 | Appeler la famille (composant `joignables`) | AED, CPE | ordre et horaires du socle 4 | — | A |
| AS-159 | Préparer un lot de **courriers postaux** de relance | secrétariat, CPE | trames D-023 ; adresse protégée respectée | `assiduite.courrier_prepare` | A |

### 17.7 Dispenses et autorisations collectives

| N° | Action | Qui · droit | Règles | Événement | Corresp. |
|---|---|---|---|---|---|
| AS-160 | Poser une **dispense de présence** | CPE, direction · `dispenses.gerer` | portée, dates (≤ année), présence requise ou non | `assiduite.dispense_posee` | VS-13 · I |
| AS-161 | Demander une dispense (famille, élève majeur) | responsable, majeur | décision de la vie scolaire | `assiduite.demande_deposee` | A (DA-16) |
| AS-162 | Dispense ponctuelle d'un cours (mot des parents) | professeur d'EPS · `dispenses.gerer` (ses cours) | une séance | idem | A |
| AS-163 | Clore ou raccourcir une dispense | CPE | historique | `assiduite.dispense_modifiee` | I |
| AS-164 | Voir les dispenses et inaptitudes de son cours | professeur d'EPS | sans motif médical | — | P |
| AS-165 | Alerte inaptitude de plus de trois mois | SYS (avec 34) | R312-3 | `assiduite.alerte` | A |
| AS-166 | Refléter une inaptitude du socle 34 à l'appel | SYS | `present_dispense` par défaut | — | A |
| AS-167 | Lister les dispenses actives | CPE, direction | — | — | I |
| AS-168 | Déclarer une **mission de représentation** (délégué, CVL, CA) | CPE, direction | ni absence ni bulletin (RA-19) | `assiduite.situation_declaree` | A |
| AS-169 | Déclarer une **autorisation collective** (compétition UNSS, chorale) | CPE, direction | liste d'élèves, plage | idem | A |

### 17.8 Décompte et suivi

| N° | Action | Qui · droit | Règles | Événement | Corresp. |
|---|---|---|---|---|---|
| AS-170 | Voir les demi-journées d'un élève **avec l'explication** | CPE, direction ; famille (son enfant) | version de règle affichée | — | VS-04 · I (sans explication) |
| AS-171 | Proposer « motif non communiqué malgré invitation » (L131-8 1°) | SYS → CPE | fin de relance ; moins de 16 ans | `assiduite.alerte` | A |
| AS-172 | Arrêter le mois | SYS (1er du mois) | figé ; rectifications à part | `assiduite.mois_arrete` | A |
| AS-173 | Ouvrir et consulter le **dossier annuel** | CPE, direction ; famille (vue) | un par élève et par année | — | A |
| AS-174 | Ajouter un entretien, une mesure, une échéance | CPE · `assiduite.suivre` | — | `assiduite.dossier_mis_a_jour` | A |
| AS-175 | Déclencher la **commission éducative** (vers le socle 13) | CPE, direction | R131-7 I | `assiduite.commission_demandee` | VS-22 · I (13) |
| AS-176 | Désigner un **référent**, enregistrer un engagement | direction, CPE | RA-12 ; document signé au socle 7 | `assiduite.referent_designe` | A |
| AS-177 | Alertes « absences répétées » et « tendance » | SYS | S-476 ; déterministe | `assiduite.alerte` | A |
| AS-178 | Vue famille du dossier | responsable, majeur | sans notes internes ni texte de l'autre parent | — | A |
| AS-179 | Clore les dossiers en fin d'année | SYS | R131-6 | `assiduite.dossier_clos` | A |

### 17.9 Signalement et exports

| N° | Action | Qui · droit | Règles | Événement | Corresp. |
|---|---|---|---|---|---|
| AS-180 | **Proposer un signalement** (dossier pré-rempli) | CPE · `assiduite.signalement_proposer` | décompte figé ; profil académique | `assiduite.signalement_propose` | VS-21 · I |
| AS-181 | Valider ou refuser ★ | direction · `assiduite.signalement_valider` | ré-authentification (passkey) | `assiduite.signalement_decide` | I |
| AS-182 | Exporter selon le profil (PDF, CSV) | direction, secrétariat · `assiduite.exporter` | trace d'export | `assiduite.export` | A |
| AS-183 | Noter la transmission (date, outil, qui) | direction, CPE | aucun appel automatique d'outil académique | `assiduite.signalement_transmis` | P (« émis ») |
| AS-184 | Suivre la suite (avertissement, convocation, MLDS) | CPE | — | `assiduite.dossier_mis_a_jour` | A |
| AS-185 | Rectifier un signalement après requalification | direction | alerte AS-123 | `assiduite.signalement_rectifie` | A (I16) |
| AS-186 | Choisir et mettre à jour le **profil académique** | direction · `assiduite.regle_gerer` | liste fournie ; trace | `assiduite.regle_versionnee` | A |
| AS-187 | Export **bourses** : jours d'absence injustifiés par boursier et trimestre | intendance, direction | RA-28 ; vers le socle 26 | `assiduite.export` | A |
| AS-188 | Liste des **étudiants boursiers** non assidus (CROUS) | direction | justificatif demandé avant ; < 1 mois exclu (RA-29) | idem | A |
| AS-189 | Export des absences pour l'**employeur** d'un apprenti | direction, DDFPT | RA-31 | idem | A |

### 17.10 Paramétrage, statistiques, reprise

| N° | Action | Qui · droit | Règles | Événement | Corresp. |
|---|---|---|---|---|---|
| AS-190 | **Rapport annuel** d'absentéisme au CA | direction | par classe et niveau (RA-20) | — | A |
| AS-191 | Statistiques d'assiduité | direction, CPE · `assiduite.statistiques` | jamais de classement nominatif publié | — | VS-15 · A |
| AS-192 | Export pour le **bulletin** (demi-journées, retards par période) | SYS → 23 | port `absences_periode` | — | P |
| AS-193 | **Registre d'appel** : consultation, export, impression | direction, CPE | RA-01 ; archive 10 ans | — | A |
| AS-194 | Créer une **version de règle** d'assiduité | direction · `assiduite.regle_gerer` | aperçu de l'effet ; datée | `assiduite.regle_versionnee` | P (DA-11) |
| AS-195 | Gérer les **motifs** de l'établissement | direction, CPE | liste courte ; rattachement à une nature ; historique | `assiduite.motif_modifie` | P |
| AS-196 | Régler relances, délai de consolidation, activités appelées | direction | S-470, S-473, S-477 | `assiduite.regle_versionnee` | A |
| AS-197 | Feuille d'appel **de secours** imprimable du jour | AED, CPE | option S-478 ; minimale | — | A |
| AS-198 | Activer l'appel d'activités (demi-pension, étude, CDI) | direction | RA-02 | — | A |
| AS-199 | **Reprise** de l'historique (Charlemagne, Next.js accueil) | technique « reprise » (T0 §5) | épisodes `reprise`, calendrier réel | `assiduite.reprise` | A |

---

## 18. Règles transverses (R-AS1 et suivantes) [R]

| N° | Règle |
|---|---|
| R-AS1 | Un appel vise une **séance datée** (ou une activité typée), jamais une classe texte ; une seule par séance. |
| R-AS2 | L'effectif d'un appel est le **public de la séance à la date**, recalculé jusqu'à la validation. |
| R-AS3 | Une **absence est un épisode** ; une séance n'appartient qu'à un épisode ; l'épisode est l'unité du motif, du justificatif, de la qualification et de l'avis. |
| R-AS4 | Une **situation** (sortie, stage, examen, mission, exclusion-inclusion, mobilité, localisation, dispense de présence) **n'est jamais une absence**. |
| R-AS5 | Le **motif déclaré** et la **qualification** sont deux champs ; seules les cinq natures de L131-8 peuvent être « réputées légitimes ». |
| R-AS6 | Une déclaration d'un appelant non reconnu est « tiers non vérifié » ; elle pré-remplit l'appel mais ne qualifie rien. |
| R-AS7 | Toute déclaration sur un élève « attendu ailleurs » avertit le déclarant (forçage motivé). |
| R-AS8 | Toute modification d'un constat validé est une **correction** historisée (auteur, heure, canal, motif) ; aucune suppression physique. |
| R-AS9 | Toute opération hors ligne porte un **identifiant unique** et n'est appliquée qu'une fois ; les droits sont relus à la réception. |
| R-AS10 | En conflit, une **présence ou une localisation constatée par un adulte** l'emporte sur un « absent » ; sinon, conflit à trancher. |
| R-AS11 | Une opération hors ligne n'est acceptée que pour le jour même ou la veille (S-469). |
| R-AS12 | Une séance **annulée ou neutralisée** après l'appel neutralise ses constats et recalcule les épisodes. |
| R-AS13 | L'avis d'absence part **une fois par foyer et par demi-journée**, après le délai de consolidation, sauf urgence (U3). |
| R-AS14 | Tout avis parti à tort est suivi d'un **démenti**. |
| R-AS15 | La **règle d'assiduité est versionnée et datée** ; elle ne s'applique jamais rétroactivement à un mois arrêté. |
| R-AS16 | Un mois est **arrêté** le 1er du mois suivant ; les rectifications postérieures sont listées à part et signalées si elles touchent un signalement. |
| R-AS17 | Le décompte tient compte du **calendrier** (vacances, fériés, fermetures, demi-journées sans cours) et du **fuseau de l'établissement**. |
| R-AS18 | Aucune alerte n'écrit à une famille d'elle-même ; elle crée une **tâche** ; un courrier part après décision humaine. |
| R-AS19 | Motif déclaré, nature « maladie », certificats : **C3**, visibles de la vie scolaire et de la direction seules ; jamais du professeur ni de l'IA. |
| R-AS20 | La localisation « infirmerie » est un code ; elle n'est montrée au professeur que pendant **sa** séance. |
| R-AS21 | Le professeur ne voit, pour les élèves de ses cours, que l'**état** des absences (à régulariser / régularisée), jamais leur motif (D-022). |
| R-AS22 | La famille ne voit **ni la note interne** de la vie scolaire **ni le texte** écrit par un autre responsable. |
| R-AS23 | L'élève mineur ne déclare ni ne justifie ; l'élève majeur le fait ; l'information des parents d'un majeur suit la politique de l'établissement (Q-AS9). |
| R-AS24 | Un professeur ne déclare pas d'absence, ne pose pas de retard hors de ses séances, ne retire pas d'épisode. |
| R-AS25 | Les événements ne portent **aucun texte libre** ni motif déclaré (codes et identifiants). |
| R-AS26 | Aucun signalement n'est **transmis** par ScolIA ; il est préparé, validé (ré-authentification) et transmis par une personne, qui le note. |
| R-AS27 | Les statistiques de pilotage ne nomment ni un élève ni un professeur hors des écrans habilités. |
| R-AS28 | Le niveau « procureur » d'un profil académique est désactivé pour un élève de 16 ans révolus (RA-14). |
| R-AS29 | Aucune sanction d'exclusion n'est proposée pour absentéisme, et aucune sanction n'apparaît au bulletin depuis le 12 (RA-18). |
| R-AS30 | Aucun appel, aucune présence ne repose sur une **donnée biométrique** (RA-33). |
| R-AS31 | La retenue de bourse se calcule en **jours** d'absences injustifiées depuis la rentrée (RA-28), distincts des demi-journées. |
| R-AS32 | Le cache hors ligne ne contient que l'effectif minimal du jour, chiffré, effacé à minuit et à la révocation. |

---

## 19. Événements

### 19.1 Émis (outbox, dans la transaction du fait) [R]

`appel.ouvert`, `appel.valide`, `appel.ligne_corrigee`, `appel.neutralise`, `appel.rouvert`, `appel.delegue`, `appel.relance`, `appel.correction_demandee`, `appel.conflit_tranche`, `appel.constats_deplaces`, `appel.signalement_vs` ; `assiduite.situation_declaree`, `_modifiee`, `_close` ; `assiduite.absence_ouverte`, `_rapprochee`, `_qualifiee`, `_requalifiee`, `_retiree`, `_dementie`, `_restructuree`, `_corrigee`, `assiduite.absence_apres_presence`, `assiduite.alerte_urgente` ; `assiduite.localise` ; `assiduite.retard_enregistre`, `assiduite.retards_lot` ; `assiduite.dispense_posee`, `_modifiee` ; `assiduite.demande_deposee`, `assiduite.autorisation_decidee` ; `assiduite.justificatif_depose` ; `assiduite.relance_envoyee`, `assiduite.contact_trace`, `assiduite.courrier_prepare` ; `assiduite.mois_arrete`, `assiduite.alerte` ; `assiduite.dossier_mis_a_jour`, `_clos`, `assiduite.commission_demandee`, `assiduite.referent_designe` ; `assiduite.signalement_propose`, `_decide`, `_transmis`, `_rectifie` ; `assiduite.regle_versionnee`, `assiduite.motif_modifie`, `assiduite.export`, `assiduite.reprise`.

**Alias** des anciens noms pendant la transition (lot AS2) : `appel.opened` → `appel.ouvert`, `appel.closed` → `appel.valide`, `eleve.absence_declared` → `assiduite.absence_ouverte`, `eleve.absence_justified|_refused` → `assiduite.absence_qualifiee`, `eleve.absence_annulee_present` → `assiduite.absence_retiree`, `vs.retard` → `assiduite.retard_enregistre`, `vs.absence_declaree` → `assiduite.situation_declaree`, `vs.signalement_assiduite_emis` → `assiduite.signalement_transmis` ; `eleve.absence_seuil_atteint` et `_signal_*` **cessent** (DA-07). Le socle 8 traduit les événements en natures d'avis (§14.5).

### 19.2 Consommés [R]

`edt.seance_*` (annulée, modifiée, déplacée, ajoutée, intervenant changé) (10) ; `referentiel.cours_membre_*`, `fermeture_*`, `grille_horaire_definie` (5) ; `services.intervention_*` (6) ; `eleve.cree|majeur|sortie|decede|fusionne|mobilite_*` (2) ; `lien.*`, `interdiction.*` (4) ; `sortie.participants_partis`, `sortie.absent_depart`, `sortie.retour` (42) ; `stage.periode_*` (39) ; `examen.convocation_*` (38) ; `sante.passage_*`, `sante.renvoi_domicile`, `sante.inaptitude_*` (34) ; `discipline.exclusion_inclusion_*`, `discipline.exclusion_*` (13) ; `accueil.sortie_effectuee` (14) ; `messagerie.message_recu` (16, pour la carte) ; `absence_rh.seance_non_assuree` (15).

---

## 20. Écrans clés

**Web (Leptos, `docs/FRONTEND_LEPTOS.md`, `DESIGN_SYSTEM.md`)** [R] :
1. **Appel** (professeur, AED) : depuis la séance ; en-tête (classe ou groupe, salle, heure, effectif) ; élèves pré-remplis en tête avec leur état et un libellé court ; liste dense ou trombinoscope ; un geste par élève, clavier complet (flèches, P/A/R) ; retard en minutes ; récapitulatif et « Appel fait » ; indicateur « enregistré localement » ; historique des corrections de ses appels.
2. **Mes appels** : du jour et des jours précédents (faits, à faire, corrections demandées).
3. **Journée de la vie scolaire** : file du matin (onglets : non joints, à qualifier, appels non faits, incohérences, absences après présence, arrivées) ; panneau élève (où est-il maintenant, épisodes du jour, contacts, bouton Appeler) ; déclaration rapide (accueil) ; billets.
4. **Qualification** : liste d'épisodes, détail (séances, déclarations, pièces, contacts), qualification individuelle ou en masse.
5. **Dossier annuel** d'un élève : chronologie, décompte du mois avec explication, alertes, mesures, référent, signalements.
6. **Signalement** : pré-rempli, relecture, validation par passkey, export, transmission.
7. **Paramétrage** : règle d'assiduité (versions, aperçu), profil académique, motifs, relances, activités appelées.
8. **Statistiques** et **rapport au CA** (tuiles Bento, tableaux).
9. **Famille et élève** : absences par épisode, état clair, « donner le motif » en trente secondes, pièce, dossier, retards, dispenses.
10. **Accueil** : recherche d'un élève → « en sortie », « prévenu », « absent ce matin » (lecture unitaire journalisée) → déclaration en trois gestes.

**Mobile (socle 11, D-003)** [R] : appel depuis « aujourd'hui » (professeur, AED, encadrant), **hors ligne** ; billets et localisation (AED) ; file du matin réduite (CPE, AED) ; déclaration et justification (famille, majeur), photo du mot comme pièce ; avis d'absence de l'élève accompagné (AESH). Qualification en masse, dossier complet, signalement et paramétrage restent sur le web.

**Accessibilité** : l'état d'un élève n'est jamais porté par la seule couleur (libellé et icône) ; feuille d'appel lisible au lecteur d'écran (« Léa Martin, absente, prévenue ») ; cibles ≥ 48 dp ; 375 px.

**Remplace** dans `ui.rs` (F-001) et dans les modules Leptos actuels `appel`, `journee_vs`, `vie_scolaire`, `absences`, `discipline_vs/signalements` (repris et refondus, §2.3).

---

## 21. Contrat IA (`IA_COEUR.md` §5) [R]

| Capacité | Type | Droit | Sensibilité | Confirmation | Exemple |
|---|---|---|---|---|---|
| `assiduite.dicter_appel` | simulation | `appel.faire` | C1 | **l'intervenant valide l'appel** | « Tous présents sauf Léa et Tom, Tom en retard de dix minutes » (IA_COEUR §614-624) |
| `assiduite.extraire_declaration` | simulation | `absences.declarer` | C1 (le texte du message reste C2, encadré comme non fiable) | la personne confirme la déclaration | message « Léo sera absent demain matin pour un rendez-vous » → déclaration proposée |
| `assiduite.saisir_appel_telephonique` | simulation | `absences.declarer` | C1 | l'accueil confirme | phrase dite par l'accueil → élève, période, motif (exemple vécu D-023) |
| `assiduite.resumer_file` | lecture | `appel.verifier` | C1 | — | « Qu'est-ce qui reste à traiter ce matin ? » |
| `assiduite.expliquer_decompte` | lecture | `absences.lire_motif` ; famille pour son enfant | C1 | — | « Pourquoi mardi matin compte-t-il ? » |
| `assiduite.rediger_courrier` | rédaction | `assiduite.suivre` | C2 | **confirmation humaine de l'envoi** | brouillon de relance ou de convocation (trame D-023) |
| `assiduite.preparer_signalement` | rédaction | `assiduite.signalement_proposer` | C2 | la CPE relit, la direction valide | synthèse chronologique du dossier |
| `assiduite.qualifier` | commande | — | — | **non exposée à l'IA** | — |
| `assiduite.signalement_valider` | commande | — | — | **non exposée à l'IA** | — |
| `assiduite.statistiques` | lecture | `assiduite.statistiques` | C0/C1 | — | « Quelle classe a le plus d'absences le lundi matin ? » |

L'IA ne reçoit **jamais** un motif déclaré en texte libre, une pièce, une note interne ni la nature « maladie » ; elle ne propose **aucune** qualification ni aucune étiquette (« décrocheur ») ; les noms sont pseudonymisés (§7.3 d'`IA_COEUR.md`). Jeu d'évaluation : 40 cas (dictée bruitée, homonymes, phrase ambiguë « il ne viendra pas cet après-midi », message piégé qui demande de « justifier toutes les absences », question d'un professeur sur le motif d'un élève → refus).

---

## 22. Modèle de données cible

Conventions (E1–E14, D-036, D-038) : `etablissement_id UUID NOT NULL` (différent de l'UUID nul) sur **toutes** les tables, filles comprises ; clés primaires UUID applicatives ; **clés étrangères composites** `(etablissement_id, x_id)` ; `cree_le`, `modifie_le` ; aucune suppression physique d'un objet qui a servi (états datés) ; aucune colonne JSON/JSONB ; aucun `BYTEA` (pièces au socle 7, `objet_id` + empreinte) ; RLS standard (permissive puis stricte) ; classement au registre des tables (E12) ; heures murales `TIME`, dates `DATE`, instants `TIMESTAMPTZ`. Les listes de valeurs sont des `CHECK` sur des codes stables, ou des tables de référence quand l'établissement les étend.

### 22.1 Référentiels

| Table | Colonnes principales | Remarque |
|---|---|---|
| `ref_asd_nature_motif` | `code` PK, `libelle`, `legitime_l131_8 BOOLEAN`, `medical BOOLEAN`, `ordre` | **globale de référence** (E4, E12) : les 5 natures légales + natures de classement ; aucune donnée d'établissement |
| `ref_asd_profil_academique` | `id`, `code_academie`, `version`, `mois_type` (`civil`/`glissant`), `outil`, `age_limite_penal`, `valide_du`, `valide_au` | globale de référence |
| `ref_asd_profil_niveau` | `profil_id`, `niveau`, `seuil_demi_journees`, `delai_jours`, `piece_exigee_code`, `destinataire_code` | globale de référence |
| `ent_asd_motif` | `id`, `etablissement_id`, `nature_code` → `ref_asd_nature_motif`, `libelle`, `libelle_famille`, `visible_famille`, `actif`, `cree_le`, `modifie_le` | remplace `ent_absence_motif_etab` ; libellés historisés par `ent_asd_motif_version` |
| `ent_asd_regle` | `id`, `etablissement_id`, `valide_du DATE`, `definition_demi_journee` (`une_seance`/`minutes`/`entiere`), `seuil_minutes`, `type_mois`, `profil_id`, `delai_consolidation_min`, `delai_verification_min`, `rythme_relance` (`court`/`standard`/`long`), `seuil_retard_absence_pct`, `seuil_absences_repetees`, `seuil_retards`, `decide_par`, `cree_le` | **jamais modifiée** une fois utilisée par un mois arrêté (déclencheur) ; la coupure vient du socle 5 |
| `ent_asd_activite_appelee` | `etablissement_id`, `type_activite` (`permanence`, `etude`, `cdi`, `demi_pension`, `accompagnement`), `active`, `valide_du` | RA-02 |

### 22.2 Appel

| Table | Colonnes principales | Remarque |
|---|---|---|
| `ent_appel` (étendue) | + `seance_id` (FK composite vers la séance du socle 10), `type_activite`, `activite_ref_type` + `activite_ref_id` (sortie, internat, activité ; référence typée), `etat` (`en_cours`, `valide`, `neutralise`), `ouvert_par`, `valide_par`, `valide_le`, `canal_validation` (`web`/`mobile`/`hors_ligne`), `delegue_a` | unique `(etablissement_id, seance_id)` ; `classe` texte et `creneau_id` deviennent des projections (retrait en AS16) |
| `ent_appel_ligne` (étendue) | + `etat` (`non_pointe`, `present`, `absent`, `retard`, `localise`, `attendu_ailleurs`, `present_dispense`), `minutes_retard`, `lieu_code`, `situation_id`, `version INTEGER`, `modifie_par`, `modifie_le`, FK composite vers l'élève | — |
| `ent_appel_ligne_histo` | `id`, `etablissement_id`, `ligne_id`, `etat_avant`, `etat_apres`, `minutes`, `lieu_code`, `auteur_id`, `canal`, `motif_code`, `motif_texte` (court), `operation_id`, `cree_le` | ajout seul |
| `ent_appel_operation` | `operation_id` PK (UUID de l'appareil), `etablissement_id`, `appel_id`, `eleve_id`, `etat`, `minutes`, `saisie_appareil_le`, `version_base`, `session_id`, `recue_le`, `resultat` (`appliquee`, `compatible`, `conflit`, `refusee`), `raison_code`, `tranche_par`, `tranche_le` | hors ligne ; conservée 3 mois ; **à neutraliser à la restauration** (E9) |
| `ent_appel_delegation` | `id`, `etablissement_id`, `seance_id`, `de_user`, `a_user`, `motif`, `cree_le`, `revoquee_le` | — |

### 22.3 Situations connues

| Table | Colonnes principales | Remarque |
|---|---|---|
| `ent_asd_situation` | `id`, `etablissement_id`, `eleve_id`, `nature` (`declaration`, `dispense_presence`, `localisation`, `sortie`, `pfmp`, `examen`, `mission`, `exclusion_inclusion`, `mobilite`, `autorisation`, `sortie_anticipee`), `debut_jour`, `debut_heure`, `fin_jour`, `fin_heure`, `source` (`famille`, `eleve_majeur`, `accueil`, `vie_scolaire`, `messagerie`, `lien_nominatif`, `socle`), `origine_ref_type`, `origine_ref_id`, `motif_id`, `lieu_code`, `etat` (`active`, `close`, `annulee`, `venu`), `declarant_personne_id`, `declarant_qualite`, `declarant_verifie BOOLEAN`, `canal`, `cree_par`, `cree_le`, `modifie_le` | reprend `ent_absence_declaration` (131) et les reflets des autres socles |
| `ent_asd_situation_texte` | `situation_id`, `etablissement_id`, `auteur_personne_id`, `texte` (≤ 500), `cree_le` | **séparé** : lu seulement par l'auteur et la vie scolaire (R-AS22) ; effacé en fin d'année |
| `ent_asd_situation_version` | historique des modifications d'une situation | ajout seul |

### 22.4 Absences

| Table | Colonnes principales | Remarque |
|---|---|---|
| `ent_asd_absence` | `id`, `etablissement_id`, `eleve_id`, `jour_debut`, `jour_fin`, `etat` (`ouvert`, `a_qualifier`, `qualifie`, `retire`), `qualification` (`en_attente`, `reputee_legitime`, `excuse_valable`, `non_valable`, `sans_motif`), `nature_code`, `motif_id`, `prevenue`, `source`, `libelle_famille_code`, `cree_le`, `modifie_le` | l'épisode ; remplace `ent_absence` |
| `ent_asd_absence_seance` | `etablissement_id`, `absence_id`, `seance_id`, `appel_ligne_id`, `jour`, `heure_debut`, `heure_fin`, `regle_id`, `neutralisee_le` | une séance ⇒ un épisode (unique `(etablissement_id, seance_id, eleve_id)`) ; reprend `ent_absence_seance` |
| `ent_asd_absence_etat` | `id`, `etablissement_id`, `absence_id`, `etat`, `qualification`, `auteur_id`, `motif_code`, `note_interne` (courte), `cree_le` | historique (fin de DA-10) |
| `ent_asd_absence_declaration` | `absence_id`, `situation_id`, `etablissement_id` | rapprochement (reprend 131) |
| `ent_asd_justificatif` | `id`, `etablissement_id`, `absence_id`, `objet_id` (socle 7, FK composite), `sha256`, `medical`, `presente` (`depose`, `vu_non_conserve`, `retire`), `depose_par`, `cree_le`, `detruit_le` | E5 ; 1 an |
| `ent_asd_demande` | `id`, `etablissement_id`, `eleve_id`, `type` (`autorisation`, `dispense`, `correction_appel`), période, `motif_id`, `demandeur_personne_id`, `etat`, `decide_par`, `decide_le`, `decision_code` | — |
| `ent_asd_relance` | `id`, `etablissement_id`, `absence_id`, `etape`, `prevue_le`, `faite_le`, `canal`, `annulee_le` | E9 |
| `ent_asd_contact` | `id`, `etablissement_id`, `eleve_id`, `absence_id` NULL, `personne_id`, `canal`, `resultat`, `auteur_id`, `cree_le` | — |

### 22.5 Retards et dispenses

| Table | Colonnes principales | Remarque |
|---|---|---|
| `ent_asd_retard` | `id`, `etablissement_id`, `eleve_id`, `seance_id`, `appel_ligne_id`, `arrivee_heure`, `minutes`, `motif_code`, `lot_id`, `billet_numero`, `compte BOOLEAN`, `cree_par`, `cree_le` | remplace `ent_retard` |
| `ent_asd_compteur` | `etablissement_id`, `nature` (`billet`), `annee_id`, `valeur` | E3 |
| `ent_asd_dispense` | `id`, `etablissement_id`, `eleve_id`, `portee` (`matiere`, `activite`, `cours`, `seance`), `matiere_id`, `cours_id`, `seance_id`, `activite_code`, `debut`, `fin`, `presence_requise`, `origine` (`decision`, `demande`, `inaptitude`), `inaptitude_ref_id` (34), `etat`, `decide_par`, `cree_le`, `modifie_le` | remplace `ent_dispense_vs` ; **aucun commentaire médical** (au 34) |

### 22.6 Décompte, dossier, signalement

| Table | Colonnes principales | Remarque |
|---|---|---|
| `ent_asd_demi_journee` | `etablissement_id`, `eleve_id`, `jour`, `periode`, `minutes_manquees`, `minutes_non_justifiees`, `compte`, `non_justifiee`, `regle_id`, `calcule_le` | recalculable ; PK `(etablissement_id, eleve_id, jour, periode)` |
| `ent_asd_mois_arrete` | `etablissement_id`, `eleve_id`, `mois`, `demi_journees`, `non_justifiees`, `retards`, `regle_id`, `arrete_le` | figé |
| `ent_asd_rectification` | `id`, `etablissement_id`, `eleve_id`, `mois`, `avant`, `apres`, `cause_ref_id`, `cree_le`, `vue_par` | R-AS16 |
| `ent_asd_dossier` | `id`, `etablissement_id`, `eleve_id`, `annee_id`, `etat` (`ouvert`, `suivi`, `clos`), `referent_personne_id`, `ouvert_le`, `clos_le` | R131-6 ; unique `(etablissement_id, eleve_id, annee_id)` |
| `ent_asd_dossier_evenement` | `id`, `etablissement_id`, `dossier_id`, `type` (`contact`, `entretien`, `courrier`, `commission`, `engagement`, `referent`, `signalement`, `suite`, `note`), `ref_type`, `ref_id`, `echeance`, `auteur_id`, `visible_famille`, `cree_le` | chronologie |
| `ent_asd_alerte` | `id`, `etablissement_id`, `eleve_id`, `type`, `ref_id`, `etat` (`ouverte`, `traitee`, `classee`), `traitee_par`, `cree_le` | tâches ; remplace le signal annuel (012) |
| `ent_signalement_assiduite` (étendue) | + `dossier_id`, `profil_niveau`, `valide_par`, `valide_le`, `authentification_forte_le`, `transmis_le`, `transmis_par`, `outil_code`, `rectifie_le` ; `dossier JSONB` retiré (AS16) | garde 134 et 563 |

### 22.7 Ce qui est repris ou retiré

| Existant | Devenir |
|---|---|
| `ent_absence` + `ent_absence_seance` (130) | repris en épisodes (`ent_asd_absence`, `_seance`), **regroupés** par élève et par jour contigu ; anciennes tables en lecture seule puis retirées (AS16) |
| `ent_absence_declaration`, `ent_absence_rapprochement` (131) | repris dans `ent_asd_situation` et `ent_asd_absence_declaration` |
| `ent_vs_regle_assiduite` (130) | première version de `ent_asd_regle` (`definition_demi_journee = minutes`, 30 min : on **garde** le comportement passé pour les mois écoulés) |
| `ent_absence_motif`, `ent_absence_motif_etab` (003, 013) | natures et motifs (`ref_asd_nature_motif`, `ent_asd_motif`) ; correspondance des codes |
| `ent_retard` (087), `ent_dispense_vs` (067) | `ent_asd_retard`, `ent_asd_dispense` (commentaire médical **non repris**, signalé au 34) |
| `ent_absence_signal` (012, 035, 043) | archivé en lecture ; plus aucun nouveau signal (DA-07) ; signaux ouverts convertis en alertes |
| `ent_signalement_assiduite` (134, 563) | étendue ; colonne `dossier` retirée |
| `ent_appel_relance` (132) | gardée |

---

## 23. Plan de migration depuis l'existant (sans casser l'appel)

1. **AS0** corrige sur place, sans changer le modèle : droits (DA-01 à DA-03, DA-16, DA-17), regroupement des absences d'un même jour sur une seule ligne `ent_absence` (DA-05 : la clôture rattache l'absent à l'absence `a_traiter` du même élève et du même jour au lieu d'en créer une), arrêt des courriers du signal annuel (DA-07), atomicité (DA-08), fuseau (DA-13), bornes et déclaration au lieu du fait pour la route famille (DA-14), DTO famille (DA-15), anti-doublon (DA-22), texte libre retiré des événements (DA-21), domaine passé en `TenantTx` (DA-04).
2. **AS1–AS3** créent les nouvelles tables à côté des anciennes ; une **reprise idempotente** convertit les absences et séances existantes en épisodes (regroupement par élève, jour et contiguïté), les déclarations en situations, la règle en première version, les motifs en motifs rattachés à des natures ; l'**écriture** passe sur les nouvelles tables, les anciennes routes deviennent des **projections** en lecture (mêmes réponses, tests d'instantanés).
3. **Bascule de la séance** : tant que la séance datée stable du socle 10 (lots T1–T2) n'est pas intégrée, l'appel garde `creneau_id + jour` comme clé de séance (déjà le cas) ; la colonne `seance_id` est renseignée dès que le port `seance` existe ; aucune donnée n'est perdue.
4. **AS16** active la RLS stricte (sous rôle non propriétaire) et retire les anciennes tables après la durée d'archive (`ent_absence_signal`, `ent_absence_motif*`, `ent_retard`, `ent_dispense_vs`, colonne `dossier` JSONB).

Chaque migration est **additive et rejouable** depuis tout état antérieur (E8) ; la reprise ne réécrit jamais un mois déjà signalé.

---

## 24. Lots de code (équipe Sonnet)

Chaque lot livre : migrations dans la plage (§24.3), dépôt, service en transaction unique, routes, types (`ent-types`), événements, tests Postgres (isolation entre établissements, matrice de droits, matrice D-022), écrans quand il en a, contrat IA quand il expose une capacité, documentation (ce document, `PROJECT_STATUS.md`, registre des tables). Revue `review-sonnet` sur chaque lot ; revue sécurité (skill `security-review`) sur AS0, AS2, AS3, AS9, AS10. Effort : **S** ≤ 3 j, **M** 1–2 sem., **L** > 2 sem. [H].

### 24.1 Tableau des lots

| Lot | Contenu | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|
| **AS0 Correctifs urgents** | DA-01, DA-02, DA-03, DA-05 (regroupement par jour), DA-07 (plus de courrier du signal annuel), DA-08, DA-13, DA-14, DA-15, DA-16, DA-17, DA-21, DA-22 ; passage du domaine en `TenantTx` (DA-04, préparation) | — (sur la tête d'`ENT`) | tests rouges avant, verts après : professeur d'allemand → 403 sur déclarer, retard, remettre présent d'un élève d'espagnol ; une journée d'absence → **une** absence, **un** avis par demi-journée ; aucun courrier « seuil » ; La Réunion à 6 h 30 locale → appels du jour corrects ; famille → déclaration bornée ; parent B ne lit pas le texte de A ; mineur ne pose pas de dispense | M |
| **AS1 Référentiels et droits** | natures et motifs, règle versionnée, profil académique, activités appelées ; actions du catalogue §15.1 (fin des rôles codés en dur, DA-18) | AS0 | un rôle libre « AED » fait l'appel d'une permanence et propose une qualification, sans qualifier ; une règle utilisée par un mois arrêté est immuable (test) | M |
| **AS2 Appel v2 et corrections** | appel depuis la séance (port 10, repli créneau), effectif recalculé, états de ligne, situations et pré-remplissage, validation en lot, historique, corrections, délégation, neutralisation sur annulation (DA-09), incohérences étendues | AS1 ; socle 10 T2 (sinon repli) | séance annulée après l'appel → absences neutralisées et démenti ; élève inscrit après l'ouverture → ajouté ; prévenu à 8 h 05 → visible dans l'appel ouvert ; toute correction historisée ; deux ouvertures simultanées → même appel | L |
| **AS3 Épisodes, qualification, justificatifs** | épisodes, contiguïté, qualification à 5 natures légales, requalification, masse, justificatifs (objets, C3), demandes d'autorisation, reprise des absences existantes | AS2 ; socle 7 (objets) | reprise : nombre de demi-journées identique avant/après sur le jeu de labo ; un épisode multi-séances justifié couvre ses séances (DA-12) ; aucun motif ni pièce visible du professeur (test de DTO) | L |
| **AS4 Décompte versionné** | calcul avec calendrier et fuseau, matérialisation, explication, arrêt du mois, rectifications | AS3 ; socle 5 (calendrier, coupure) | « recalculer tout ne change rien » ; changement de règle en janvier → septembre inchangé ; vacances déclarées → 0 demi-journée | M |
| **AS5 Familles et notifications** | avis par foyer et demi-journée, délai de consolidation, démenti, relance graduée, contacts tracés, courriers postaux, natures §14.5 | AS3 ; socle 8 (moteur) | journée d'absence = 2 avis au plus par foyer ; arrivée à l'infirmerie dans le délai → aucun avis ; avis parti puis correction → démenti ; parent sous interdiction : rien | M |
| **AS6 Vérification et file du matin** | relance automatique, file classée, appel à la place, absence après présence, « prévenir maintenant » | AS2, AS5 | relance unique à `debut + délai` ; aucune relance d'une séance non assurée ; file triée selon §17.6 | M |
| **AS7 Retards, billets, localisation** | retards v2, billets numérotés, lots de transport, localisation, reflets 14 et 34 | AS2 | billet à 8 h 20 → ligne corrigée en retard ; retard de car en lot → hors compteur | S |
| **AS8 Dispenses** | dispense de présence ≠ inaptitude, demandes, dispense ponctuelle, reflet du 34, alertes | AS2 ; socle 34 S2 | inapte présent par défaut ; aucune donnée médicale en base du 12 (test de schéma) | S |
| **AS9 Dossier annuel et signalement** | dossier R131-6, alertes et tâches, commission (vers 13), référent, signalement pré-rempli, validation par passkey, export par profil, transmission notée, rectification ; retrait du signal annuel (DA-07) | AS4, AS5 ; socle 13 (commission), 7 (trames) | dossier complet d'un élève de labo au seuil ; 16 ans → pas de niveau pénal ; requalification après validation → alerte | L |
| **AS10 Hors ligne** | journal d'opérations, envoi par lot, idempotence, conflits, cache minimal chiffré, web en lot unique ; **avec le socle 11** (iOS, Android) | AS2 ; socle 11 | coupure réseau simulée pendant l'appel → aucune perte ; lot rejoué → aucun doublon ; appareil révoqué → refus ; conflit infirmerie → localisé gagne | L |
| **AS11 Exports et statistiques** | registre d'appel, bulletin (23), bourses (26), CROUS, employeur, CA, statistiques, enquêtes | AS4 | export bourse : imputation au trimestre du franchissement (jeu d'or) ; aucun export par un professeur | M |
| **AS12 Composant d'appel d'activités** | permanence, étude, CDI, demi-pension (option), sortie (42), internat (41), activité (43) | AS2, AS10 | même composant dans trois contextes ; appel de départ hors ligne | M |
| **AS13 IA** | §21 : dictée, extraction, saisie téléphonique, résumé, explication, brouillons ; jeu d'évaluation | AS5 ; cœur IA | aucune commande de qualification exposée ; 40 cas du jeu verts | M |
| **AS14 Écrans** | AS14a appel web ; AS14b journée de la vie scolaire et file ; AS14c famille et élève ; AS14d qualification et dossier ; AS14e signalement ; AS14f paramétrage et statistiques ; écrans mobiles au socle 11 | lots back | rendu contre le vrai backend ; clavier ; 375 px ; axe-core ; parcours joué dans un navigateur | L (total) |
| **AS15 Reprise** | historique Charlemagne (absences à la journée, motifs, justifications), absences saisies par l'accueil dans Next.js si Q-T0-07 le demande | AS3, AS4 ; socle 9 ; `ent-reprise` | jeu anonymisé : totaux par élève et par mois identiques à la source, écarts listés | M |
| **AS16 RLS stricte et retraits** | RLS stricte sous rôle non propriétaire ; retrait des anciennes tables et de la colonne JSONB | tous | 0 ligne sans contexte d'établissement ; `grep -i jsonb` des migrations du socle à zéro | M |

### 24.2 Ordre, démonstration de décembre et V1

- **Ordre** : **AS0 tout de suite** → AS1 → AS2 → AS3 → (AS4, AS5) → AS6 → (AS7, AS8) → AS9 → AS10 (avec le socle 11) → AS11 → AS12 → AS13 ; AS14 au fil de l'eau ; AS15 avant le doublon ; AS16 en dernier.
- **Démonstration de début décembre** (D-034, « appel et absences ») : **AS0**, **AS1** (actions seulement), **AS2** (sur le repli créneau si la séance du socle 10 n'est pas prête), **AS3** (épisodes, qualification, justificatif texte puis pièce), **AS5** (avis regroupé, démenti), écrans **AS14a–c** ; le hors ligne est **montré sur le mobile en janvier** (AS10), pas promis pour décembre.
- **V1 de fin janvier – février 2027** (D-034) : AS0 à AS10 et AS14a–e ; AS11 (registre, bulletin, bourses) ; le reste en épaississement (février – août).
- **Année de doublon 2027-2028** : AS15 indispensable (Charlemagne reste le registre officiel, Q-AS15).

### 24.3 Migrations (plage imposée 2240–2259, registre `MIGRATIONS.md`)

| N° | Lot | Contenu |
|---|---|---|
| 2240 | AS0 | anti-doublon des déclarations ; `ent_appel.valide_par`, `valide_le` ; `ent_appel_ligne.modifie_par`, `modifie_le` ; index de regroupement par élève et jour |
| 2241 | AS1 | `ref_asd_nature_motif`, `ref_asd_profil_academique`, `ref_asd_profil_niveau`, `ent_asd_motif` (+ version), `ent_asd_regle`, `ent_asd_activite_appelee` ; reprise des motifs et de la règle |
| 2242 | AS2 | appel v2 : `seance_id`, `type_activite`, référence typée, états, `ent_appel_ligne_histo`, `ent_appel_delegation` |
| 2243 | AS2 | `ent_asd_situation`, `_texte`, `_version` ; reprise des déclarations (131) |
| 2244 | AS3 | `ent_asd_absence`, `_seance`, `_etat`, `_declaration` ; reprise des absences (004, 130) en épisodes |
| 2245 | AS3 | `ent_asd_justificatif`, `ent_asd_demande` |
| 2246 | AS4 | `ent_asd_demi_journee`, `ent_asd_mois_arrete`, `ent_asd_rectification` |
| 2247 | AS5 | `ent_asd_relance`, `ent_asd_contact` |
| 2248 | AS6 | `ent_asd_alerte` (tâches) ; index de vérification |
| 2249 | AS7 | `ent_asd_retard`, `ent_asd_compteur` ; reprise de 087 |
| 2250 | AS8 | `ent_asd_dispense` ; reprise de 067 sans le commentaire |
| 2251 | AS9 | `ent_asd_dossier`, `ent_asd_dossier_evenement` |
| 2252 | AS9 | extension de `ent_signalement_assiduite` ; archivage du signal annuel (012) |
| 2253 | AS10 | `ent_appel_operation` |
| 2254 | AS11 | traces d'export (si `ent_export_trace` du socle 9 n'est pas intégré) |
| 2255 | AS12 | activités appelées (références typées vers 41, 42, 43) |
| 2256–2257 | — | réserve (passe personas : §29) |
| 2258 | AS16 | RLS stricte des tables du socle |
| 2259 | AS16 | retrait des anciennes tables et de `ent_signalement_assiduite.dossier` |

Ordre de rejeu : 2240–2259 sont plus hautes que toutes les migrations actuelles (642) ; elles référencent la séance du socle 10 (plage 4000–4099 ou 440–459) par une **colonne** remplie au lot AS2, avec une clé étrangère ajoutée en `NOT VALID` puis validée quand la table de séance existe (pas de dépendance d'ordre bloquante) ; à renuméroter si la règle « 3000 + 100 × socle » (Q-B2) est adoptée (elle donnerait 4200–4299).

---

## 25. Tests et recette

**Tests automatiques exigés** (en plus de la matrice de séparation §15.4) :
1. Chaque cas limite CA-01 à CA-72 a un test (ceux qui dépendent d'un autre socle sont marqués et activés quand il existe).
2. Propriétés : « recalculer tout ne change rien » ; « aucune heure Europe/Paris en dur » (recherche) ; « aucun texte libre dans les événements » (test de type) ; « aucune donnée médicale dans les DTO professeur » ; « une séance ⇒ un épisode » ; « une opération hors ligne ⇒ au plus une application ».
3. Jeux d'or : décompte d'un mois de labo calculé à la main ; retenue de bourse (RA-28) ; reprise (totaux identiques).
4. Concurrence : deux validations simultanées ; correction pendant l'envoi d'un lot hors ligne ; qualification pendant une requalification.
5. Performance (labo) : validation d'un appel de 35 élèves < 300 ms ; file du matin < 1 s pour 1 500 élèves ; arrêt d'un mois < 30 s pour 1 500 élèves.
6. Tests d'intégration qui **échouent** sans base en CI.

**Recette par personas** (D-021, étape 4) [R] : professeur d'EPS au gymnase sans réseau ; CPE un lundi de rentrée de vacances ; accueil à 8 h avec dix appels ; famille de parents séparés avec deux déclarations contradictoires ; élève majeur ; étudiant boursier de BTS ; élève d'ULIS ; établissement de La Réunion ; attaques : professeur hors de ses groupes sur toutes les routes, élève qui se justifie, parent qui lit le texte de l'autre, accueil qui lit l'historique, lot hors ligne rejoué ou falsifié, appareil révoqué, autre établissement.

---

## 26. Réglages signalés « usine à gaz » (D-023)

Numérotation à la suite de S-466. Signalés, pas supprimés.

| N° | Réglage | Pourquoi il est suspect | Proposition |
|---|---|---|---|
| S-467 | Définition de la demi-journée (3 formules) + seuil en minutes | réglage technique que personne ne sait choisir | défaut « au moins une séance manquée » ; les deux autres repliés |
| S-468 | Photo au trombinoscope de l'appel | utile mais sensible | désactivée par défaut |
| S-469 | Fenêtre d'acceptation des opérations hors ligne | technique | fixe : jour même et veille |
| S-470 | Délai avant relance d'appel | un seul paramètre | défaut 15 min, garder |
| S-471 | Qualification d'office d'un motif déclaré | tentation de « tout justifier » | désactivée ; une seule case si activée |
| S-472 | Liste des motifs de l'établissement | tendance à la liste à rallonge | 12 motifs au plus, rattachés à une nature |
| S-473 | Rythme de relance | cinq délais possibles | trois rythmes (court, standard, long) |
| S-474 | Retard converti en absence | réglage du règlement intérieur | défaut : moitié de la séance, proposition seulement |
| S-475 | Seuil de retards répétés | sans enjeu local fort | fixe : 3 en 2 semaines |
| S-476 | Seuil d'absences répétées | idem | fixe : 4 épisodes en 4 semaines |
| S-477 | Délai de consolidation des avis | réel, mais un seul | défaut 15 min, borné 0–30 |
| S-478 | Feuille d'appel de secours | rare | option, désactivée |
| S-479 | Profil académique modifiable localement | divergence d'avec l'académie | lecture seule par défaut, écart tracé |
| S-480 | Information des parents d'un élève majeur | sujet juridique | une politique d'établissement, pas une case par famille (Q-AS9) |
| S-481 | Appel de la demi-pension, de l'étude, du CDI | trois interrupteurs | une liste à cocher, défaut : permanence et étude |
| S-482 | Foyer du jour ou les deux parents pour l'avis | déjà au socle 8 | reprendre le réglage du socle 8, pas de doublon |

---

## 27. Décisions : appliquées par défaut (D-031) et points pour Florian

### 27.1 Appliquées par défaut (D-031)

| N° | Question | Recommandation appliquée |
|---|---|---|
| Q-AS1 | Lancer **AS0** tout de suite, avant la passe personas et les autres lots ? | **Oui**, sur la tête d'`ENT`, en prévenant la recette sécurité en cours pour ne pas corriger deux fois |
| Q-AS2 | Une absence = un **épisode** (séances contiguës d'un jour, ou période déclarée) ? | **Oui** |
| Q-AS3 | Définition par défaut de la **demi-journée** | « au moins une séance manquée sans motif valable » ; mois passés gardés à « 30 minutes » (version reprise) ; inscrite au règlement intérieur |
| Q-AS4 | **Délai de consolidation** des avis | 15 minutes, réglable 0–30, supprimé pour l'urgence ; écrit dans la notice |
| Q-AS5 | **Mois** du seuil légal | civil par défaut ; glissant si le profil académique l'exige |
| Q-AS6 | Le professeur voit-il l'état des absences de ses élèves ? | **Oui**, « à régulariser / régularisée », jamais le motif |
| Q-AS7 | L'AED **qualifie**-t-il ? | **Non**, il propose ; la CPE qualifie (modèle de rôle modifiable par l'établissement) |
| Q-AS8 | **Hors ligne** dans la V1 ? | **Oui** (AS10, janvier, avec le socle 11) ; pas promis pour la démonstration de décembre |
| Q-AS10 | **Signal annuel à 10 absences** | **Supprimé** (sans fondement, contredit R131-7) ; remplacé par le dossier annuel et l'alerte « absences répétées » |
| Q-AS11 | Appel de la **demi-pension** | désactivé par défaut ; permanence et étude activées (RA-02) |
| Q-AS13 | **Qualification d'office** d'un motif déclaré | désactivée |
| Q-AS14 | Appel par **QR ou badge** | pas en V1 ; jamais biométrique ; AIPD préalable si demandé |
| Q-AS16 | **Photo** au trombinoscope de l'appel | désactivée par défaut |
| Q-AS17 | **Billet d'entrée** | affiché (et imprimable) ; numéroté par établissement |
| Q-AS19 | Inaptitude d'EPS à l'appel | « présent, dispensé de pratique » par défaut ; dispense de présence = décision explicite |
| Q-AS20 | Courrier postal en fin de relance | oui, préparé en lot, envoyé après décision |

### 27.2 Soumis à Florian (engagent l'établissement à l'extérieur, données réelles, coûts, DPO)

| N° | Point | Pourquoi Florian | Recommandation |
|---|---|---|---|
| **P-1** | **Failles probables dans Next.js en production** (lecture seule, non exécuté) : `/api/vie-scolaire/absences` n'exige que le module `vs-absences` **ou** `vs-appels`, qui inclut le rôle professeur par défaut, sans contrôle de périmètre : un professeur pourrait lire les absences à traiter de tout l'établissement (note interne comprise) et en modifier le statut (`app/api/vie-scolaire/absences/route.ts:15, 33`, `app/lib/intranet-modules.ts:696`) ; `/api/mobile/appel` ne contrôle aucun module (`route.ts:16, 55`, `channel-access.ts:40`) | Next.js est intouchable pour nous ; la matrice de modules réelle (en base) peut corriger le défaut par défaut | **Vérifier** la matrice de modules effective ; si confirmé, corriger côté Next.js (décision et action de Florian) |
| P-2 | **Parents d'un élève majeur** : information par défaut maintenue, élève informé, demande de suspension tranchée par la direction (Q-AS9) | aucun texte national trouvé ; DPO | valider avec le DPO et l'inscrire au règlement intérieur |
| P-3 | **Profil académique** de l'établissement pilote : seuils de niveaux, outil (ADESCO, MIMOSA, autre), pièces exigées | fait que seul l'établissement connaît | fournir la circulaire académique de l'année |
| P-4 | **SMS d'absence** (Q-N5 du socle 8) et **SMS entrant** (Q-AS12) | coût OVH (D-004) | SMS d'absence en escalade après 30 min non lu ; SMS entrant plus tard |
| P-5 | **Doublon 2027-2028** (Q-AS15) : Charlemagne reste le registre d'appel officiel ; faire saisir l'appel deux fois aux professeurs est irréaliste | engage l'établissement (D-034) | importer chaque jour les absences de Charlemagne dans ScolIA (AS15) **ou** un groupe pilote de classes en appel ScolIA seul, avec export vers Charlemagne ; à décider avec l'établissement |
| P-6 | **Durées de conservation** (§15.6) et base légale des statistiques | DPO, service d'archives | valider le tableau §15.6 |
| P-7 | Absences saisies par l'accueil dans Next.js : à reprendre ? (Q-T0-07) | données de production | perdable (Charlemagne officiel), sauf avis contraire |
| P-8 | Relecture juridique des articles décisifs (R131-5 à R131-7, L131-8, R312-2, R531-31) sur Légifrance | les copies lues ne sont pas le site officiel | un juriste relit avant AS9 |

---

## 28. Faits, hypothèses, contradictions

**Faits [F]** : tables, fonctions, routes et lignes citées au §2 relues sur `origin/ent-mc0hoe` (`7467f2d`) le 2026-11-11 ; DA-01, DA-02, DA-05 et DA-07 relus directement ; les autres viennent de l'audit indépendant (lecture seule, rien exécuté) et sont à reconfirmer par le lot qui les corrige. Module Next.js lu en lecture seule par un sous-agent, rien exécuté. Rien n'a été compilé ni testé pour ce document.

**Vérifié [V]** : RA-01 à RA-03, RA-05 à RA-07, RA-09, RA-11 à RA-14, RA-16, RA-21 à RA-28, RA-30, RA-31, RA-33 (en partie), RA-35 ; [V-ext] : RA-04, RA-08, RA-10, RA-15, RA-17 à RA-20, RA-29, RA-32, RA-34 (en partie).

**Hypothèses [H]** : pratiques du terrain (§3, §4 : saturation du téléphone, billets d'entrée), irritants non sourcés de la recherche de marché, efforts des lots, bornes de performance.

**Contradictions signalées**
- **C-1** `SOCLE2_DOMAINES.md` VS-20 (« seuil paramétrable, défaut 10 [F] ») décrit le signal annuel que ce document supprime (DA-07, Q-AS10) : il n'a aucun fondement réglementaire et contredit R131-7.
- **C-2** `SOCLES.md` §2 donne au socle 12 les dépendances « 2, 4, 8, 10, 11 » ; §3 ajoute 6. Ce document suit §3 (les interventions du socle 6 disent qui fait l'appel).
- **C-3** La coupure des demi-journées existe deux fois : `ent_vs_regle_assiduite.heure_coupure` (130, défaut 12 h) et STR-35 du socle 5 (défaut : milieu de la pause méridienne, à défaut 12 h 30). Le socle 12 **lit** le socle 5 et ne garde pas de copie.
- **C-4** `VERIF_REGLEMENTAIRE.md` R2 résume le seuil de quatre demi-journées par « le directeur saisit l'autorité académique » ; R131-7 I ajoute, avant, la **commission éducative** et un **document signé** avec les responsables. Complément, pas contradiction.
- **C-5** `08_NOTIFICATIONS.md` §4.1 marque le seuil `vie_scolaire.seuil_assiduite` « [NV seuil] » : il est vérifié (RA-11) ; et l'avis n'est envoyé **que** sur décision humaine (R-AS18), pas automatiquement.
- **C-6** `10_EMPLOI_DU_TEMPS.md` R-18 cite R312-2 comme [NV] : il est [V] (RA-27).
- **C-7** `34_SANTE.md` §382 écrit « l'appel affiche dispensé » pour une inaptitude ; RA-27 distingue inaptitude et dispense de présence : l'appel affiche « présent, dispensé de pratique » sauf dispense de présence décidée.
- **C-8** La branche `ent-mobile` fait l'appel en ligne seulement ; `SOCLES.md` (socle 11) prévoit « hors ligne (lecture, puis saisie de l'appel) » : ce document précise la saisie hors ligne (AS10), à porter au socle 11.
- **C-9** La circulaire n° 2011-0018 « Vaincre l'absentéisme » est **abrogée** par la circulaire 2014-159 ; toute référence à 2011 dans les documents est à remplacer.
- **C-10** R511-13 (effacement des sanctions, VERIF R34) a été réécrit par le décret n° 2026-637 du 16-7-2026 selon la copie lue (version du 18-7-2026) : à recouper avant le socle 13.
- **C-11** D-020 dit « absences enregistrées à la séance » : c'est conservé (le **constat** est à la séance) ; l'**épisode** regroupe les séances pour la justification et l'avis. Précision, pas changement.
- **C-12** `16_MESSAGERIE.md` et `IA_COEUR.md` placent la saisie téléphonique à l'accueil au niveau N2 (action préparée) : identique ici (§21).

---

## 29. Ajouts de la passe personas (`PERSONAS_12.md`, 2026-11-11)

Tout ce qui suit est marqué **« ajout personas »** ; rien de ce qui précède n'est supprimé. Les quatre bloquants (A à D) sont tranchés selon leur recommandation (D-031).

### 29.1 Actions ajoutées (AS-200 à AS-251)

| N° | Action | Qui · droit | Règles | Lot | Manque |
|---|---|---|---|---|---|
| AS-200 | **Récapitulatif quotidien à l'autre parent** (pas foyer du jour) | SYS | R-AS33 ; U0, 18 h (S-484) ; sans texte de l'autre | AS5 | M-2624 (A) |
| AS-201 | Voir **qui est informé** de ses absences ; annonce de la bascule à 18 ans | élève majeur, responsables | — | AS5 | M-2633, M-2637 |
| AS-202 | **Demander la suspension** de l'information des parents (élève majeur) ; décision de la direction | majeur ; direction | tracée ; DPO (P-2) | AS5 | M-2634 |
| AS-203 | Profil **post-bac** : parents non destinataires par défaut | SYS | R-AS34 | AS5 | M-2635 |
| AS-204 | Demander un **justificatif** à l'étudiant avant la liste CROUS | direction, CPE | délai ; < 1 mois exclu | AS11 | M-2636 |
| AS-205 | Poser une **autorisation récurrente** (soins, prise en charge extérieure) | CPE, direction · `absences.qualifier` | R-AS36 ; revue trimestrielle (S-486) | AS3 | M-2638, M-2639 |
| AS-206 | Avis à l'**AESH** de l'absence et du retour de l'élève accompagné | SYS | sans motif | AS5 | M-2640 |
| AS-207 | Retards **en lot par transporteur** (taxi, car) | AED, CPE | hors compteur | AS7 | M-2641 |
| AS-208 | **Exclure ponctuellement de cours** : l'élève part vers la vie scolaire | professeur (séance en cours) | R-AS37 ; motif court pour la vie scolaire (le 13 porte la punition) | AS2 | M-2603 (B) |
| AS-209 | **Confirmer l'arrivée** à la vie scolaire ; alerte de non-arrivée à 10 min (S-485) | AED ; SYS | U3 interne si non arrivé | AS6 | M-2612 (B) |
| AS-210 | **« Je prends »** sur une famille à appeler (verrou de 15 min visible) | AED, CPE | levé par le contact tracé | AS6 | M-2609 |
| AS-211 | Modèles de message laissé et phrases types, en plusieurs langues | AED, CPE | trames fixes (D-023) | AS6 | M-2610, M-2632 |
| AS-212 | Permanence : élèves attendus **et venus en plus** | AED | ajout d'un élève avec origine | AS12 | M-2611 |
| AS-213 | Qualifier en masse **avec filtres** (motif, source, sans pièce, durée) | CPE | aperçu, confirmation | AS3 | M-2604 |
| AS-214 | File regroupée **par famille** (fratrie) | AED, CPE | — | AS6 | M-2605 |
| AS-215 | **Convocation** à la vie scolaire visible à l'appel suivant | CPE, AED | libellé « convoqué à la vie scolaire » | AS2 | M-2606 |
| AS-216 | Écran accueil : « en sortie », « prévenu », « absent ce matin » avant de déclarer | ACC | lecture unitaire journalisée | AS14b | M-2614, M-2618 |
| AS-217 | **Tâche de rappel** d'un responsable pour une déclaration « tiers non vérifié » | SYS → AED, CPE | — | AS6 | M-2615 |
| AS-218 | Déclarer un **retard annoncé** (« il arrive à 10 h ») | ACC, famille | absence prévenue jusqu'à l'heure dite, présence attendue ensuite | AS2 | M-2616 |
| AS-219 | **Photographier** un mot papier comme justificatif | ACC, AED | objet E5 | AS3 | M-2617 |
| AS-220 | **Appel de départ** vers une installation extérieure (trajet) | professeur | appel d'activité lié à la séance | AS12 | M-2592 |
| AS-221 | Poser une dispense ponctuelle **depuis l'appel** | professeur d'EPS | une séance | AS8 | M-2593 |
| AS-222 | Voir les **restrictions** d'une inaptitude partielle sur la feuille | professeur d'EPS | port du 34, sans diagnostic | AS8 | M-2594 |
| AS-223 | **État de synchronisation** et renvoi manuel d'un appel hors ligne | intervenant | — | AS10 | M-2595 |
| AS-224 | Ouvrir l'appel hors ligne **derrière le déverrouillage** de l'appareil ; **effacement à distance** | intervenant ; SYS à la révocation | R-AS38 | AS10 | M-2596, M-2658 (C) |
| AS-225 | **Reprendre** un appel commencé par un autre (AED, titulaire) | intervenant à la date | historique | AS2 | M-2598 |
| AS-226 | Voir **qui** a fait l'appel en co-intervention | co-intervenants | — | AS2 | M-2599 |
| AS-227 | Demander une correction **depuis l'avis** reçu | intervenant | — | AS2 | M-2600 |
| AS-228 | **Rappel groupé** de fin de journée des appels non faits | SYS | une fois par jour | AS6 | M-2601 |
| AS-229 | **Déléguer** la validation des signalements (adjoint, datée) | direction | délégation `ent_delegation` ; passkey | AS9 | M-2619 |
| AS-230 | Tableau de bord de **direction** (taux, appels non faits agrégés, signalements) | direction | R-AS27 | AS11 | M-2620 |
| AS-231 | **Export du registre** d'une période pour une inspection | direction · `assiduite.exporter` | trace | AS11 | M-2621 |
| AS-232 | **Import quotidien** des absences de Charlemagne (doublon) | technique « reprise » | source `charlemagne`, jamais d'avis aux familles pour les absences importées (Charlemagne les envoie) | AS15 | M-2622 (D) |
| AS-233 | **Écarts du jour** ScolIA / Charlemagne pendant le doublon | CPE, direction | lecture | AS15 | M-2622 (D) |
| AS-234 | Avis, formulaire et courrier **dans la langue** de la famille, pictogrammes | SYS | langue du socle 4 | AS5 | M-2629, M-2631 |
| AS-235 | Répondre **par SMS codé** ou lien sans compte | responsable | SMS entrant : P-4 | AS5 | M-2630 |
| AS-236 | Historique de **ce qu'on a déclaré**, mention « un autre responsable a déclaré : nature », accusé de lecture par la vie scolaire | responsable | R-AS22 | AS5 | M-2625, M-2626 |
| AS-237 | **Vue de classe** du professeur principal (demi-journées, retards, tendance) | PP · `assiduite.suivre` (`ma_classe`, lecture) | sans motif | AS14d | M-2651 |
| AS-238 | Préparer l'assiduité de la période pour le **conseil de classe** | PP, CPE | période arrêtée (23) | AS11 | M-2652 |
| AS-239 | Planifier un **entretien** avec la famille depuis le dossier | CPE | socle 19 | AS9 | M-2607 |
| AS-240 | Le **tuteur** de PFMP signale une absence par lien nominatif | tuteur sans compte | lien à usage limité (D-027 G) | AS11 | M-2645 |
| AS-241 | **Récapitulatif d'assiduité** d'une PFMP | référent, DDFPT | — | AS11 | M-2646 |
| AS-242 | Informer l'**employeur** d'un apprenti absent | DDFPT, CPE | sans motif médical | AS11 | M-2647 |
| AS-243 | **Localiser** en deux gestes depuis la recherche (infirmerie en N-0) | infirmière | sans module santé | AS7 | M-2648 |
| AS-244 | **Fin de localisation** (« reparti en cours ») et billet de retour | infirmière, AED | le professeur voit l'élève revenir | AS7 | M-2649 |
| AS-245 | **Journal des consultations** sensibles pour le DPO | DPO · action `donnees.journal_lire` (socle 27) | — | AS9 | M-2660 |
| AS-246 | **Export du dossier** d'assiduité d'un élève (droit d'accès) | DPO, direction | sans note d'un tiers ; double validation si complet (D-041) | AS9 | M-2661 |
| AS-247 | **Purge à échéance** avec rapport | SYS ; DPO lit | §15.6 | AS16 | M-2662 |
| AS-248 | Registre des données du **tiers déclarant** et durée | DPO | effacées en fin d'année | AS16 | M-2663 |
| AS-249 | Vue **élève** de son assiduité, avis différés pendant ses cours | élève | R-AS39 | AS14c | M-2644 |
| AS-250 | Missions de délégué affichées « instance » dans l'espace élève | élève | jamais au bulletin | AS14c | M-2643 |
| AS-251 | **Alerte de sécurité** sur une série de refus (énumération, professeur hors de ses groupes) | SYS → direction, sécurité | journal ; pas d'effet automatique | AS0 | M-2655, M-2657 |

**Précisions sur des actions existantes** : **AS-147** fonctionne aussi en N-0 (santé désactivée) : la vie scolaire ou l'infirmière enregistre le renvoi au domicile, nature `maladie_enfant`, visible de la vie scolaire seule (M-2650) ; **AS-175** relit l'état de la commission du socle 13 et le document signé du socle 7 dans le dossier (M-2608) ; **AS-181** est disponible sur mobile avec passkey (M-2623).

### 29.2 Règles ajoutées (R-AS33 à R-AS40) et précisions

| N° | Règle |
|---|---|
| R-AS33 | (bloquant A) Le responsable qui n'est pas le **foyer du jour** reçoit au plus un **récapitulatif quotidien** des absences et retards de l'enfant (U0), sans le texte écrit par l'autre ; sauf restriction du juge (socle 4) (Code civil 373-2-1 [V]). |
| R-AS34 | Un **étudiant post-bac** est seul destinataire ; ses parents ne le sont pas, sauf s'il les désigne. |
| R-AS35 | Les routes de déclaration, justification et lecture **vérifient les droits avant** de valider le corps, et renvoient la même réponse pour « inexistant » et « interdit ». |
| R-AS36 | Une **autorisation récurrente** (soins, prise en charge extérieure) est décidée par la vie scolaire ou la direction, datée, revue chaque trimestre, affichée « prise en charge extérieure » sans nature ni lieu ; elle pré-remplit `attendu_ailleurs` et ne crée pas d'absence. |
| R-AS37 | (bloquant B) Un élève **exclu ponctuellement de cours** est localisé « vie scolaire, en attente » à l'heure du geste ; son arrivée est confirmée ; sans confirmation à 10 min, alerte interne urgente ; il n'est jamais « absent » tant qu'il n'est pas constaté hors de l'établissement. |
| R-AS38 | (bloquant C) L'ouverture d'un appel depuis le cache hors ligne exige le **déverrouillage de l'appareil** (biométrie ou code) ; la révocation de la session déclenche l'**effacement** du cache au prochain contact ; le cache ne porte que les séances du jour de la personne. |
| R-AS39 | Pendant les heures de cours d'un élève mineur, les avis **destinés à l'élève** sont différés (L511-5 [V], 8 R-N66) ; ceux destinés à la famille ne le sont pas. |
| R-AS40 | Les **fréquences** de localisation à l'infirmerie ne sont ni agrégées ni montrées hors de l'infirmerie et de la CPE. |

**Précisions** : **R-AS19** — l'AED voit la **nature** d'un motif (« maladie ») mais pas le **texte libre** d'une déclaration (M-2613) ; **R-AS5** — quand deux responsables déclarent des motifs différents, la qualification porte sur l'absence, l'établissement ne départage pas les parents et n'affiche à aucun le texte de l'autre (M-2627).

### 29.3 Compléments de modèle

| Table | Colonnes principales | Remarque |
|---|---|---|
| `ent_asd_situation_recurrence` | `id`, `etablissement_id`, `eleve_id`, `jour_semaine`, `heure_debut`, `heure_fin`, `semaines_code` (A/B/toutes), `valide_du`, `valide_au`, `decide_par`, `revue_le`, `etat` | R-AS36 ; génère des situations datées ; **aucune nature médicale** |
| `ent_asd_information_majeur` | `id`, `etablissement_id`, `eleve_id`, `politique` (`parents_informes`, `suspendue`, `post_bac`), `demande_le`, `decide_par`, `decide_le`, `motif_code` | R-AS34, AS-202 ; historique en ajout seul |
| `ent_appel_ligne` | + `exclusion_cours_le`, `arrivee_vs_confirmee_le`, `arrivee_vs_par` | R-AS37 |
| `ent_asd_contact` | + `verrou_par`, `verrou_jusqua` | AS-210 |
| `ent_asd_situation` | + nature `retard_annonce`, `exclusion_cours`, `prise_en_charge_exterieure` ; + `heure_arrivee_prevue` | AS-208, AS-218, R-AS36 |

### 29.4 Place dans les lots et migrations

- **AS0** : R-AS35 et AS-251 (même famille que DA-01 à DA-03).
- **AS2** : AS-208, AS-215, AS-218, AS-225 à AS-227 ; **AS3** : AS-205, AS-213, AS-219 ; **AS5** : AS-200 à AS-203, AS-206, AS-234 à AS-236 (R-AS33 dans la **démonstration de décembre** : coût faible, risque fort) ; **AS6** : AS-209 à AS-211, AS-214, AS-217, AS-228 ; **AS7** : AS-207, AS-243, AS-244 ; **AS8** : AS-221, AS-222 ; **AS9** : AS-229, AS-239, AS-245, AS-246 ; **AS10** : AS-223, AS-224 (**condition d'entrée** du lot : R-AS38) ; **AS11** : AS-204, AS-230, AS-231, AS-238, AS-240 à AS-242 ; **AS12** : AS-212, AS-220 ; **AS14** : AS-216, AS-237, AS-249, AS-250, AS-181 mobile ; **AS15** : AS-232, AS-233 ; **AS16** : AS-247, AS-248.
- **Migrations** : **2256** (AS3) `ent_asd_situation_recurrence` et colonnes de situation ; **2257** (AS5) `ent_asd_information_majeur`, colonnes d'exclusion de cours et de verrou de contact. La réserve 2256–2257 est ainsi consommée.

### 29.5 Points ajoutés pour Florian

| N° | Point | Recommandation |
|---|---|---|
| P-9 | **AIPD** de l'assiduité (mineurs, grande échelle, santé indirecte, appareil personnel hors ligne) avant le doublon | à mener avec le DPO au printemps 2027, avant toute donnée réelle |
| P-10 | Les **soins réguliers** pendant le temps scolaire (orthophonie, SESSAD) : pratique de l'établissement pilote | autorisation récurrente (R-AS36) décidée par la vie scolaire ; à confirmer |

### 29.6 Contradictions relevées par la passe

- **C-13** §14.3 (foyer du jour d'abord, repris du socle 8) laissait l'autre parent sans information : R-AS33 complète ; à reporter au socle 8 (récapitulatif d'un responsable non destinataire immédiat).
- **C-14** §8.2 ne connaissait que l'exclusion-inclusion **décidée** (socle 13) ; l'exclusion **ponctuelle** de cours est une localisation du 12 (R-AS37) ; la punition elle-même reste au socle 13.
- **C-15** §15.2 donnait à l'AED les « motifs déclarés non médicaux » ; la précision de R-AS19 (nature visible, texte libre non) est plus stricte et la remplace.
