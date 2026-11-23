# Point quotidien ScolIA

Un point par jour, le plus récent en haut. Toujours les mêmes rubriques : **sur ENT** (validé par la CI GitHub), **prêt mais pas encore sur ENT**, **en cours**, **conçu mais pas codé**, **pas commencé**, **décisions attendues de Florian**, **à fournir par Florian plus tard** (rappel de `A_FOURNIR_PAR_FLORIAN.md`), **ce qui a coincé**.

Légende des étapes d'un socle (D-021) : C = conception Opus, Pe = passe personas sur la conception, Co = code, R = recette personas sur le logiciel, F = relecture de Florian.

---

## 2026-11-23 (lundi soir, 18 h 45 Paris) — point du jour

**Sur ENT, validé par la CI.** ENT = **4bbdfa69** (CI 596 verte) ; **7e606667** est versé, sa CI d'ENT est en cours. Onze lots ont été versés depuis hier soir, et chacun a passé une contre-revue indépendante sans point élevé restant.
- **Socle 16, messagerie**
  - **Suites du flux M12** (PR 123) : curseur sûr, temps réel sur plusieurs appareils.
  - **E1** (PR 126) : un message différé corrigé puis annulé ne part plus. Le curseur du flux repose sur les numéros de transaction (migration 491).
  - **E1 passe 2** (PR 128) : l'avis push est annulé dans la même transaction que le message. Le curseur est borné et lié à la base. La garde d'annulation utilise un jeton de transaction (492). Correctif du pilote : la relance « réponse attendue », qui ne partait plus, repart.
- **Socle 3, inscriptions I5**
  - **Passe 2** (PR 120) : la famille ne voit que les décisions publiées, rang compris. « Passer son tour » vaut pour une seule place.
  - **Passe 3** (PR 127, migration 314) : décision devenue caduque, export RGPD, acceptation au guichet. Celle-ci reste sans contrôle par une deuxième personne, mais elle est refusée après le délai, et la famille est prévenue.
- **Socle 20, cahier C8 + C9** (PR 110) : visa, inspection, archives, progressions. Correctif du pilote : le gel d'une année close n'a plus aucune exception, et la migration 2412, déjà versée, a été remise à l'identique.
- **Socle 41, internat I4**
  - **Lot principal** (PR 122) : sorties de groupe, temps du soir, veille des non-rentrés.
  - **Suites** (PR 131, migration 2830) :
    - un élève protégé peut être pointé par l'encadrant ;
    - un cas « non rentré » ne se rouvre plus chaque nuit ;
    - un refus d'un parent reçu après le départ est tracé et déclenche le retour.
  - **Correctifs du pilote** : une contrainte d'intégrité supprimée par erreur est rétablie ; un retour annulé relance l'alerte ; la demande de retour et l'alerte critique ne sont jamais regroupées ni écartées.
- **Socle 21, évaluations**
  - **Suites V5** (PR 125, migration 478) : la famille ne calcule que sur le publié ; publier et retirer comparent ce qui a été vu avant d'écrire.
  - **Suites V5, passe 3** (PR 132, migration 700) : export RGPD limité au publié ; export LSU/LSL sur le seul publié ; jeton de version ; fusions de fiches en chaîne.
  - **Correctifs du pilote** : une moyenne corrigée sous 477 est dépubliée ; l'export ne contient plus de champs internes (note privée, « aménagé »).
- **Transverse T1** (PR 129, migration 690) : aucune fonction de la base ne peut plus être trompée par une table temporaire. Trois fonctions exécutées avec les droits du propriétaire étaient exposées. Nouvelle **règle 38** : toute migration s'ajoute après 690, et toute fonction SQL fixe son chemin de recherche.

**Prêt mais pas encore sur ENT**
- Socle 3, **inscriptions I5 passe 4** (PR 130) : **nouveau modèle de verrous**, décidé après trois passes qui n'avaient pas fermé les interblocages. Le rang d'attente est calculé à la lecture, et toute écriture prend d'abord un verrou unique par offre. Les tests de concurrence sont déterministes : 11 sur 14 échouent sur l'ancien code.
- La contre-revue n'a trouvé aucun point élevé. Correctif du pilote : le verrou de la commission empêche désormais l'interblocage entre une publication de séance et une décision de la même séance.
- CI en cours sur ae6e4fc1 ; versement dès qu'elle est verte.

**En cours** (deux sessions qui codent, selon ta directive)
- Socle 16, **suites E1** (migration 493). Priorité : un fil dont le seul message a été annulé reste visible de la famille avec son sujet, **à corriger avant tout pilote réel**. Puis le jeton d'annulation, à rendre infalsifiable, et le contrôle d'existence, à faire juste avant chaque envoi.
- **Suites T1** : un test fige l'ordre des migrations (trois erreurs d'ordre aujourd'hui) ; vérification que le rôle de l'application ne peut rien créer dans `public` ; banc d'écriture.

**Conçu mais pas codé** (file d'attente)
1. Suites I4 : faibles.
2. Suites V5 : les autres tables d'évaluation exportées sans condition, à fermer avant le lot G3.
3. Suites I5 : sentinelle en base pour l'ordre des verrous, réponse à un accord.
4. Suites des voyages.
5. Suites de la facturation F3.
6. Suites du cahier.
7. Lots neufs (mobile MB1…).

**Tableau d'avancement** (`docs/pilotage/TABLEAU_MATURITE.md`) :
- inscriptions (3), messagerie (16), cahier (20), évaluations (21) et internat (41) s'épaississent nettement ;
- le transverse de sécurité gagne une garde du catalogue des fonctions ;
- le frontend et le mobile avancent moins vite que le backend. Tu l'as noté, et c'est accepté.

**Décisions attendues de Florian** : aucune (D-069). Choix pris par défaut par le pilote, à relire quand tu veux :
- acceptation de place au guichet sans deuxième personne, mais refusée après le délai, et famille prévenue ;
- refus d'un parent après le départ : tracé, il déclenche le retour sans annuler le départ ;
- élève protégé affiché « élève protégé » ;
- export LSU/LSL sur le seul publié ;
- jeton de version des moyennes externes ;
- curseur du flux de messagerie borné sans signature ;
- ancre de nuit de l'internat à 22 h : un accord « soirée » ne couvre aucune sortie qui commence avant 6 h.

**À fournir par Florian plus tard** (`A_FOURNIR_PAR_FLORIAN.md`) :
- **F-09** : comptes développeur **Apple** et **Google Play**, **avant mars 2027**. Sans eux, la démo iOS du printemps se fera sans notifications.
- Le reste n'est pas urgent : exports anonymisés Charlemagne et SIECLE (F-01 à F-07), au moment de la reprise de données.
- **Nouveau, au déploiement** (pas un fichier, deux réglages à appliquer sur chaque base) :
  - retirer le droit TEMP à PUBLIC (`REVOKE TEMP`) ;
  - vérifier que le rôle de l'application ne peut rien créer dans `public` (`docs/RLS_ISOLATION.md` §3.1).
- L'incident GitGuardian 39716183 est un **faux positif**, la constante publique du labo : à classer dans le tableau de bord GitGuardian.

**Ce qui a coincé**
- **Ordre des migrations** :
  - une migration déjà versée (2412) a été modifiée ;
  - trois lots ont placé leur migration au mauvais endroit dans le tableau.
  - Tout a été corrigé par le pilote, et le test qui fige l'ordre est en cours.
- **PR en conflit sans aucune CI** (T1) : vue à temps grâce à la règle 37.
- **Deux régressions introduites par des lots**, attrapées en contre-revue avant versement : une relance de messagerie qui ne partait plus, une contrainte d'internat supprimée.
- **Ordre des verrous d'I5** : trois passes sans résultat, puis changement de modèle (règle « trois tentatives »).
- **Disque plein** une fois (dossier de compilation du pilote) : 18 Go libérés.

---

## 2026-11-22 (dimanche soir, 18 h 45 Paris) — point du jour

**Sur ENT (validé par la CI)**, ENT = **bc69cd0d** (CI 516 verte) :
- **Faille de dépendance corrigée** (3013e4b9, PR 121) : `url` 2.5.9 (alerte RUSTSEC-2026-0592, dépassement de pile), qui faisait échouer l'audit sur toutes les branches.
- Socle 16 **messagerie, suite** (bc69cd0d, PR 116) : pièce jointe sur son propre message différé ; **flux en direct M12** (signal « nouveau message » sans contenu). Revue puis contre-revue : le flux applique désormais les mêmes droits que la lecture, une seule horloge (la base) pour ne perdre aucun message, limite de débit, test exigeant le refus exact.

**Prêt mais pas encore sur ENT** (revue et contre-revue sans élevé ; versement dès la CI verte sur la branche fusionnée avec ENT) :
- Socle 42 **voyages V3** (PR 117) : aides financières, reports de versement entre enfants, lien avec l'internat. Correctifs : un interne n'est retiré de l'attendu qu'à son départ constaté ; l'attendu suit toute annulation ou changement de dates ; personne ne décide une aide pour son propre enfant ; seconde personne au-delà de 150 € ou pour le fonds social (paramètre d'établissement).
- Socle 21 **notes V5** (PR 114) : parcours des élèves, résultats et moyennes externes (CNED, autre établissement). Correctifs : une moyenne externe n'est jamais vue de la famille avant publication ; portée du professeur limitée à ses cours ; message de refus qui dit la vraie cause.
- Socle 24 **facturation F3** (PR 118) : catalogue qualifié, grilles tarifaires votées et figées, simulateur, familles types. Correctifs : date de vigueur appliquée ; second regard par une autre personne ; gel complet en base ; aucune ligne qui disparaît en silence ; articles qualifiés exigés par défaut pour les nouveaux établissements.
- **Test instable** (PR 124) : un test du flux M12 comptait les événements d'un compte de labo partagé.

**En cours** (sessions qui codent) :
- Socle 11 **mobile MB0**, passe 2 : un accès échu ou suspendu ne reçoit plus de notifications ; aucune information ne traverse les établissements ; fonction privilégiée réservée au rôle applicatif.
- Socle 3 **inscriptions I5** (décisions, commission, liste d'attente, propositions de place) : correctifs de 2 élevés (un refus jamais publié devenait visible de la famille ; déport d'un membre lié au candidat seulement volontaire).
- Socle 41 **internat I4** (vie quotidienne : sorties de groupe, études, vigilance, événements) : correctifs de 3 élevés (élève parti non rentré sans alerte ; accord « soirée » valant pour une sortie de plusieurs jours ; cause d'un blocage visible de l'accompagnateur).
- Socle 20 **cahier C8 + C9**, passe 3 : le gel complet d'une année close bloquait la reprise de l'existant au rejeu des migrations.
- Socle 16 **suites du flux M12** (PR 123) : curseur sûr face aux transactions longues, temps réel sur plusieurs appareils.

**Conçu mais pas codé** : suites des contre-revues (voyages : seuil cumulé, règle « pas pour son enfant » sur tous les pouvoirs d'argent, départ constaté avant V3b ; V5 : retrait de publication, moteur identique pour famille et personnel, notes ordinaires après fusion de fiches ; F3 : tarif de la période facturée mois par mois) ; mobile **MB1 à MB16** ; 20 C10.

**Tableau d'avancement** (`docs/pilotage/TABLEAU_MATURITE.md`) : messagerie (16) un peu plus épaisse ; mobile (11) passe de « pas commencé » à « conception faite, MB0 en cours ». Mise à jour complète au prochain versement groupé.

**Décisions attendues de Florian** : aucune (D-069). Choix pris par défaut par le pilote, à relire quand tu veux : seuil de seconde validation des aides de voyage (150 €, fonds social toujours) ; délai d'une demande de réouverture du cahier (14 jours) ; tolérance avant alerte d'un élève non rentré d'une sortie (15 min) ; tarif appliqué = celui de la période facturée ; articles qualifiés exigés pour les nouveaux établissements (alerte « à activer avant la rentrée 2027 » pour l'existant).

**À fournir par Florian plus tard** (`A_FOURNIR_PAR_FLORIAN.md`) : **F-09** comptes développeur **Apple** et **Google Play** **avant mars 2027** (sinon la démo iOS du printemps se fera sans notifications) ; F-08 (conteneur et base ScolIA séparés chez Scaleway) avant Noël : le pilote te donnera la liste exacte deux semaines avant. Le reste : plus tard.

**Ce qui a coincé** :
- **File de la CI saturée** (jusqu'à 1 h 30 d'attente avant le premier job) : plusieurs sessions ont atteint la limite de leur attente ; le pilote reprend la main sur leurs verdicts.
- Une session s'était endormie sur un **réveil qui ne se déclenche jamais** (`ScheduleWakeup`, inopérant hors /loop) avec une CI rouge : interdit dans les consignes (règle 37).
- Le **gel complet d'une année close** (cahier) bloquait la reprise des anciens devoirs : corrigé par un seul chemin autorisé, celui de la reprise.
- Un **test instable** versé avec la messagerie (compte de labo partagé) : correctif PR 124.

---

## 2026-11-21 (samedi soir, 18 h 45 Paris) — point du jour

**Sur ENT (validé par la CI)**, ENT = **02659006** (CI 472 verte) :
- Socle 21 **V3d** (57ee4763) : le zéro de remplacement et la note d'un rattrapage n'écrasent plus une note ou une dispense saisie entre-temps (compare-et-écrit) ; règle du suppléant sur les rattrapages ; le bulletin et le livret respectent « statistiques visibles des familles » ; le professeur peut attribuer le zéro réglementaire en période ouverte (confirmation, motif, absence non justifiée constatée) ; tests durcis ; moyenne générale fausse retirée de l'ancienne interface. Revue : aucun critique ni élevé.
- **Tests indépendants du jour de la semaine** (714ec134) : 8 tests rougissaient le week-end (jour sans cours, période de notes franchissant le 20 décembre) ; corrigés sans affaiblir les assertions.
- **Faux rouge corrigé** (02659006) : le test `a03` des pièces restreintes croyait voir une fuite (« A03 » présent par hasard dans un identifiant aléatoire). Aucune fuite réelle.

**Prêt mais pas encore sur ENT** : socle 20 **C8 + C9** (cahier : visa, inspection, archive annuelle, progressions, 6 écrans ; PR 110) — **bloqué par la revue** (1 élevé : la clôture annuelle ne fige pas tout le cahier). Correctifs prêts, lancement **dimanche 9 h 10 UTC**.

**En cours** : aucune session ne code (quota hebdomadaire en avertissement : règle 37, aucun lancement dans les 24 h avant la remise à zéro de dimanche 9 h UTC, pour garder la marge de 10 %).

**Conçu mais pas codé** : 21 **V3e** (motif du zéro conservé, demi-journée justifiée hors séance, dernière moyenne fausse du bandeau CPE, impasse après l'échéance d'un rattrapage, choix de la période « du jour ») puis V5 ; **semis du labo relatif à l'année** (il casse le 1er septembre 2027) ; 41 I4 ; 20 C10 (plage de migrations à attribuer).

**Tableau d'avancement** : inchangé cette semaine (`docs/pilotage/TABLEAU_MATURITE.md`), notes un peu plus épaisses (V3d).

**Décisions attendues de Florian** : aucune. Florian a décidé le 21-11 (règle 37, D-069) : le pilote gère seul le rythme et le quota, et toute règle métier incertaine devient un paramètre de l'établissement avec une valeur par défaut prudente.

**À fournir par Florian plus tard** (`A_FOURNIR_PAR_FLORIAN.md`) : rien de nécessaire bientôt.

**Ce qui a coincé** :
- La session V3d est restée **bloquée trois heures** en attente d'une décision (CI rouge hors lot) sans que son message arrive : le pilote ne l'a vu qu'à la demande de Florian. Corrigé : à chaque réveil, le pilote lit l'état de toutes les sessions (règle 37).
- Deux faux rouges (tests liés au samedi, test `a03`) : causes corrigées, pas seulement relancées.

---

## 2026-11-20 (vendredi soir, 18 h 45 Paris) — point du jour

**Sur ENT (validé par la CI)**, ENT = **d23a6f66** (CI 463 verte après une relance) :
- Socle 41 **I3 suites** (f7b9421c) : statut **« tiers autorisé à récupérer »** (posé par le personnel à la demande écrite d'un titulaire, un an au plus, soumis à toutes les interdictions) ; **infirmerie** branchée sur la fonction unique de remise ; perte de droit datée par des **faits immuables** ; correspondant désigné par le CPE plus suspendu à tort ; recherches de classe par code **avec l'année** (cause du rouge d'hier). Revue : 2 élevés corrigés (parent privé d'autorité posé tiers ; agent du personnel aussi parent) ; correctif du pilote : liens parentaux reconnus dans le texte libre (« Maman », « Père », « tutrice »).
- Socle 21 **V3c + V4** (d23a6f66) : correctifs de la revue de V3b (plus de « 0 / 20 » pour un absent, auteur obligatoire, seuil d'effectif des statistiques du bulletin, agenda) ; **V4** : absents proposés depuis l'appel (confirmés par le professeur), dispenses, **rattrapages** (convocation, résultat, échéance), évaluation de remplacement et « seul 0 autorisé », aménagements visibles de l'AESH sans donnée de santé. Revue : aucun critique ni élevé.

**Prêt mais pas encore sur ENT** : rien.

**En cours** : aucune session ne code (en attente de ton choix, quota hebdomadaire en avertissement jusqu'à dimanche).

**Conçu mais pas codé** : 21 **V3d** (suites de la revue de V4 : zéro qui écrase une note saisie entre-temps, suppléant en V4, bulletin qui ignore « statistiques visibles des familles », tests faibles, test web non déterministe) ; 21 V5 à V13 ; 41 I4 (vie quotidienne, migrations 2828-2829) et suivants.

**Tableau d'avancement** : nouveau tableau des **51 socles** dans `docs/pilotage/TABLEAU_MATURITE.md` (3 bien avancés, 4 à moitié, 11 au tiers, 5 faibles, 28 pas commencés ; environ 15 à 20 % de l'ENT complet). Plus gros manques : applications mobiles (11), bulletins (23).

**Décisions attendues de Florian** :
- **Prochain lot** : V3d (avis du pilote, aujourd'hui), cahier de textes (20), ou conception des applications mobiles (11) — le reste à partir de dimanche.
- « **Tiers autorisé à récupérer** » : règle posée par le pilote (personnel, demande écrite d'un titulaire, un an au plus, parent privé d'autorité exclu, seul le demandeur révoque côté famille) : à confirmer.
- **R-01** (évaluations) : référence du mémento eduscol de décembre 2025 à valider.
- **Moyenne générale de l'ancienne interface** (`ui.rs`) calculée sans coefficients, donc fausse : la retirer en attendant le moteur ?
- Année « **préparation** » créée d'office par le référentiel quand on saisit une date ancienne : à trancher plus tard.
- Fermer la PR 102 (documentation seule, remplacée).

**À fournir par Florian plus tard** (`A_FOURNIR_PAR_FLORIAN.md`) : rien de nécessaire bientôt.

**Ce qui a coincé** :
- Le pilote du 19-11 s'était mis **en attente de ton accord** (quota) : plus rien n'était lancé ; reprise ce matin par l'ancien pilote.
- Le message de fin de la session V3c + V4 **n'est jamais arrivé** : rattrapé à la ronde (la ronde vérifie toujours les sessions).
- **CI 463 d'ENT rouge** sur un test web des notes non déterministe (vert sur la PR, même arbre) : relance verte, cause à corriger dans V3d.
- Base locale du pilote mal nommée (le harnais exige « test ») : sans effet sur le code.

---

## 2026-11-19 (jeudi soir, 18 h 45 Paris) — point du jour

**Changement de pilote** à 10 h UTC : nouvelle session de pilotage au contexte léger, pour économiser le quota (passation `docs/pilotage/PASSATION_PILOTE.md`). La règle 36 est appliquée : pas d'abonnement aux PR de lot, revues sur Sonnet.

**Sur ENT (validé par la CI)** : ENT = **9190a677**, run 452 vert. Depuis le point d'hier (86f03b82) :
- Socle 41 **I2** (la nuit à l'internat) et ses suites (migration 2825).
- Socle 21 **V1-V2** : modèle des évaluations et moteur de calcul unique, « comme Charlemagne ».
- Socle 21 **V3** : grille de saisie des notes en Leptos.
- Socle 21 **V3b** : tous les lecteurs passent sur le nouveau modèle (relevé élève et famille, bulletin, conseil, moyennes, agenda, cahier de textes, mobile, ancienne interface). Les anciennes tables deviennent des archives en lecture seule, et une note non publiée n'est jamais montrée à l'élève ni à la famille. Revue de sécurité : aucun défaut critique ni élevé.
- **CI** : 8 Go de mémoire d'échange pour la compilation des tests, après 4 machines perdues en 24 h.

**Prêt mais pas encore sur ENT** : rien.

**En cours** :
- Socle 41 **I3, passe 4** (absences et sorties de l'internat). La passe 3, verte en CI, n'a pas été versée : la revue de sécurité y a trouvé **5 défauts élevés**. Un enfant pouvait partir avec une personne choisie par un parent qui a perdu son droit. Les causes : une décision de routine du CPE « couvrait » l'acte du parent ; la perte d'autorité sans interdiction formelle ne faisait rien tomber ; un contact déclaré par ce parent restait valable ; le contrôle reposait sur un libellé en texte libre ; les sorties scolaires sont à vérifier. La passe 4 reprend tout avec **8 règles générales**, dont « un acte tombe avec le **droit** de son auteur ».

**Conçu mais pas codé** :
- Socle 21 **V3c** : correctifs moyens de la revue de V3b. Le « 0 / 20 » affiché à la famille pour une absence dans le cahier de textes, les évaluations non annoncées visibles dans l'agenda, les statistiques de classe du bulletin sans seuil d'effectif, et la tolérance trop large du rejeu des migrations en test.
- Socle 21 **V4 à V8**.
- Internat **I4, I5**.

**Tableau d'avancement** : celui d'hier, avec trois changements. **21 Évaluations** passe de « faible » à **environ un tiers**, avec un premier écran (la grille). **41 Internat** passe à **environ un tiers** (I0 à I2 faits, I3 en cours). Les autres domaines sont sans changement aujourd'hui.

**Décisions attendues de Florian** :
- **Lancer V3c aujourd'hui malgré l'avertissement sur le quota hebdomadaire ?** Avis du pilote : V3c seul maintenant, V4 après la remise à zéro de dimanche. Sans réponse, rien n'est lancé.
- Choix acceptés par défaut, sauf objection (règle 12) :
  - V3b : retour de note du cahier jamais montré avant publication ; barème figé dès qu'une note existe ; anciennes notes sans évaluation laissées en archive.
  - I3 : absence tranchée par la direction ; un parent contact au lien échu est écarté ; une sortie dont l'auteur a perdu son droit retourne à « demandée ».
- Fermer la PR 102 (documentation seule, remplacée).

**À fournir par Florian plus tard** (`A_FOURNIR_PAR_FLORIAN.md`) : rien de nécessaire bientôt. Les exports Charlemagne de notes (F-04) serviront au doublon I-032, quand l'outil de comparaison existera.

**Ce qui a coincé** :
- **I3** : quatre passes pour un seul lot. Les consignes de départ listaient des champs au lieu d'énoncer des règles générales sur la protection de l'enfant. La leçon est appliquée à la passe 4.
- **V3b** : deux CI rouges, à cause de la reprise des données sur les nouvelles vues dans la migration 472 et des tests de l'emploi du temps. Corrigé par la session, vert au troisième passage.
- **Machines de CI perdues** à répétition sur la compilation des tests : mémoire d'échange ajoutée, et le passage 452 est vert.
- **Quota hebdomadaire en avertissement** dès jeudi : une seule session code ce soir.

---

## 2026-11-18 (mercredi soir, 18 h 45 Paris) — point du jour

**Décision structurante du jour (D-068, Florian)** : la bascule n'a plus lieu en février. Elle se fera **à la rentrée 2027**, voire un peu avant.
- La **priorité est l'ENT complet**, à épaissir domaine par domaine au plus haut niveau.
- Les modules du Next.js font partie de l'ENT. Ils sont secondaires, mais doivent être épaissis et reliés à tout le reste.
- Ce qui compte, ce sont des **preuves** qu'on va au bout, plus que des échéances. Une démo « masterclass » peut n'arriver qu'en juin.
- **Notes et bulletins** : ScolIA fait tout « comme de l'officiel », sans branchement aux échanges académiques.
- Florian comparera lui-même ScolIA à Charlemagne sur de vrais élèves (I-032).

**Sur ENT (validé par la CI)**, ENT = 86f03b82 (run 417 vert) :
- Socle 24 **F1-F2** (finances) : service financier unique, comptes des familles, payeurs (parents séparés), organismes. Six élevés de la revue de sécurité corrigés.
- Socle 2 **E9** : suites de la confidentialité de l'élève (pastilles « vie de classe » et « protocole », catalogue des champs à 16) ; **premiers écrans de l'élève** (liste et recherche, fiche avec synthèse) pour la démo.
- Socle 42 **V2a** : suivi de l'argent des voyages (part des familles, versements, bordereaux de chèques, acomptes fournisseurs, export comptable), sans facturation.
- **CI** : le job des écrans passe de **78 min à environ 21 min de calcul**, avec les mêmes 153 tests et un contrôle de couverture.
- **Suites techniques** : planificateur quotidien (acomptes, purge des exports), guichet calé sur le jour de l'établissement, liste blanche des événements financiers, balayage des photos orphelines, journaux de la CI sans bruit.
- **Faille corrigée** : `lettre` 0.11.24 (RUSTSEC-2026-0565, e-mail).
- **Défaut corrigé** : un seul « jour de l'établissement » (migration 3201) ; l'outre-mer avait deux dates selon le module.
- Socle 16 **M5-M6** (messagerie) : pièces jointes avec quarantaine et antivirus, PDF désarmés strictement, accusés de lecture, avis neutres, liens de 60 s journalisés. La revue avait trouvé un PDF actif possible et un administratif qui lisait les messages d'un élève protégé : corrigés.

**Prêt mais pas encore sur ENT** : rien.

**En cours** :
- Socle 41 **I2** : la nuit à l'internat (appels, clôture, procédure, transmission du matin).
- Socle 21 **V1-V2** : modèle des évaluations et **moteur de calcul des moyennes**, avec un document « règles comme Charlemagne » et les questions à poser à Florian.
- **Visite guidée** : captures de tous les écrans, rôle par rôle, pour que Florian juge lui-même de l'épaisseur.

**Prochains, dans l'ordre D-068** :
- 21 (évaluations, suite) ;
- 20 (cahier) ;
- 23 (bulletins et conseils) ;
- 13 (discipline) ;
- 17 (annonces) ;
- 29 (portails, « Aujourd'hui ») ;
- 11 (applications mobiles, pas commencées) ;
- 22 (compétences) ;
- 33 (visio).

**Tableau de maturité, première version** : estimation du pilote, à recaler avec la visite guidée et l'avis de Florian. « Plan » = part approximative du plan de lots versée.

| Domaine | Plan versé | Écrans | Commentaire |
|---|---|---|---|
| 1 Sécurité et comptes | élevé | oui | passkeys et SSO ; reste l'invitation en masse |
| 2 Élève | environ la moitié | liste et fiche | E10 mobile, E11 exports, écrans E9c à E9f |
| 3 Inscriptions | environ un tiers | anciens | I5 et suivants |
| 4 Dossier administratif | environ un tiers | anciens | |
| 5 Structure | élevé | partiels | S12, S13 |
| 7 Documents et signature | environ la moitié | non | D6 fait ; D7 et suivants |
| 8 Notifications | faible | non | |
| 10 Emploi du temps | environ un tiers | partiels | EDT réel (T10) |
| 11 Applications mobiles | **nul** | — | à démarrer |
| 12 Assiduité | environ la moitié | oui (appel, vie scolaire, famille) | |
| 13 Discipline | **faible** | anciens | |
| 15 Absences des personnels | élevé | oui | A10 reprise |
| 16 Messagerie | environ un tiers | partiels | M9, M10, M12… ; pièce sur message programmé |
| 18 Formulaires | environ un tiers | partiels | |
| 19 Agenda et salles | environ un tiers | oui | |
| 20 Cahier de textes | environ la moitié | partiels | |
| 21 Évaluations et notes | **faible** (V0 seul) | non | V1-V2 en cours |
| 22, 23 Compétences, bulletins | **nul** | — | |
| 24 Finances | faible | non | F1-F2 faits |
| 27 RGPD | environ un tiers | non | |
| 28 Reprise | environ un tiers | non | |
| 29 Portails | faible | partiels | |
| 32 Espaces des personnels | faible | non | |
| 41 Internat | faible | non | I2 en cours |
| 42 Voyages | environ un tiers | partiels | V2a fait ; V3 et suivants |

**Décisions attendues de Florian** :
- Aucune urgente.
- Après la visite guidée, son avis domaine par domaine (« à quel pourcentage de ce que j'attends »).
- Plus tard :
  - les règles de calcul de Charlemagne (questions du document du socle 21) ;
  - les trois points finances du 17-11 (guichet, encaissement au comptoir, validation à deux personnes) ;
  - l'ordre de la file du matin.

**À fournir par Florian plus tard** (`A_FOURNIR_PAR_FLORIAN.md`) :
- Rien de nécessaire bientôt.
- Avec D-068, l'export anonymisé du Next.js perd son urgence (prévu après le 4 décembre).
- Les **exports Charlemagne de notes** (F-04) prennent de la valeur pour le doublon I-032, mais seulement quand l'outil de comparaison existera.
- Plus tard : SIECLE et STS (F-05, F-07), démarche éditeur (F-06).

**Ce qui a coincé** :
- **Sessions qui refusent les consignes envoyées par message** (règle 1) : le pilote fait lui-même les petites corrections de dernière minute, et met désormais tout dans le message de lancement.
- **Les sessions lancées depuis ENT ne voyaient pas PILOTAGE.md complet**, resté sur la branche du pilote : corrigé dans les consignes.
- **Tests faibles ou faux révélés par les correctifs** : images de test tronquées, compteur de champs figé, attente d'un 403 au lieu d'un 404. Corrigés sans rien assouplir.
- **Test dépendant de l'heure** : il a révélé un vrai défaut, deux jours différents pour l'outre-mer.
- **Disque plein** sur la machine du pilote : nettoyé.
- **Une erreur de consigne du pilote** : interdire une pièce jointe sur un message programmé. À reprendre.

## 2026-11-17 (mardi soir, 18 h 45 Paris) — point du jour

**Sur ENT (validé par la CI)** — dernière tête verte : 79b41ca3 (run 359) :
- Socle 15 **A4bis** : un remplacement s'organise sans attendre la validation de l'absence ; mention « répond rarement aux sollicitations ».
- **CI** : les tests du serveur web passent contre le vrai backend. C'est le trou découvert hier : sept fichiers de tests ont été remis en état.
- Socle 28 **R2** : zone de reprise des données Next.js, avec décodage strict, correspondance et simulation.
- Socle 7 **D6** : brique de signature en maison (passkey, code SMS, guichet, papier, preuve chaînée, horodatage RFC 3161, parapheur). La revue de sécurité a été corrigée : 4 défauts élevés et 7 moyens.
- Socle 3 **I3-I4** (inscriptions) : vraies pièces justificatives et accords des responsables.
- Socle 32 **ET0-ET1** : espace documents des personnels, avec une faille inter-établissements corrigée avant versement.
- Socle 18 **FO2-FO3** : création de formulaires, accords sur objets et dossier de preuve.
- **Fiabilité des tests** : plus d'heure réelle dans AS6 et AG0, interblocages du rejeu corrigés, test adapté à PostgreSQL 18.
- **Veille du pilote** : la CI commente son résultat sur une pull request. Je suis réveillé à la fin d'un run, sans rondes à vide.
- Compteurs : 253 actions, 63 sources de routes.

**Prêt mais pas encore sur ENT** :
- Socle 2 **E7-E8** est poussé sur ENT (47a5be1e), le run 362 est en cours. Contenu : confidentialité de l'élève protégé, photos, coordonnées vérifiées, pastilles et synthèse.
- La revue de sécurité avait relevé 2 défauts critiques et 6 élevés, tous corrigés avec un test chacun. Le test vérifie aussi que toutes les routes `/eleves/{id}/*` répondent 404 à un tiers.

**En cours** (2 sessions au plus, ta décision de ce matin) :
- Socle 24 **F1-F2** (finances) : service financier unique, comptes débiteurs et payeurs. C'est le préalable de l'argent des voyages (V2).
- La session s'est déconnectée vers 16 h, en fin de vérification (54 tests sur 55). Je l'ai relancée.
- La seconde place se libère dès le verdict du run 362.

**Conçu mais pas codé (prochains)** :
- Socle 2 : suite E9, catalogue des données de l'élève (photo, e-mail, mobile), pastille neutre « protocole », ELV-44 et ELV-46.
- Voyages V2, après F1-F2.
- Reprise R7 et R10.
- Écrans AS14d-f.
- Écrans du cœur pour la démo de début décembre.
- **D-067 (ta décision d'aujourd'hui)** : jiff remplace chrono, avec sqlx 0.9, pendant un gel vers le 7 décembre, après la démo.

**Tableau d'avancement par socle** (versé sur ENT ; nouveautés du jour en gras) :

| Socle | Versé | Suivant |
|---|---|---|
| 2 Élève | E0 à E6, **E7-E8 (run en cours)** | E9, catalogue E5, pastille « protocole » |
| 3 Inscriptions | I0 à **I4** | suite du socle |
| 7 Documents | D0 à D5b, **D6 signature** | M3, M4, M9, M10 ; validation CMS du jeton d'horodatage |
| 12 Assiduité | AS0 à AS6, AS14a-c | AS14d-f |
| 15 Absences des personnels | A0 à A11c, **A4bis** | A11d-f |
| 18 Formulaires | FO0 à **FO3** | suite du socle |
| 24 Finances | — | **F1-F2 en cours** |
| 28 Reprise | MS0, R0, R1, **R2** | R7, R10 |
| 32 Espaces des personnels | **ET0-ET1** | suite du socle |
| 42 Voyages | V0 à V1 | V2 après F1-F2 |

**Décisions attendues de toi** :
- Ordre de la file du matin (AS6, 9 niveaux) : toujours ouvert.
- Pour le DPO plus tard, pas pour toi tout de suite : conservation de 12 mois pour `mes.reprise`, et masquage des petits effectifs (moins de 3).

**À fournir par toi plus tard** (`A_FOURNIR_PAR_FLORIAN.md`) :
- Rien d'urgent.
- Prochain besoin : l'**export anonymisé de Next.js**, après la livraison de l'outil d'anonymisation (R1, le 4 décembre).
- Plus tard : exports Charlemagne (F-01 à F-04), liste et exemples SIECLE/STS (F-05, F-07), démarche éditeur (F-06).
- Le conteneur et la base séparés chez Scaleway (F-08) ne sont plus d'actualité depuis D-064.

**Ce qui a coincé** :
- **Run 350 rouge** (FO2-FO3) : un test de rejeu était trop strict sous PostgreSQL 18, qui compte aussi les NOT NULL parmi les contraintes. Corrigé (run 358 vert).
- Des échecs dus à la seule machine de CI qui démarrait mal, résolus en relançant le job.
- **Avertissement d'audit sur chrono**, devenu « non maintenu » : il ne s'agit pas d'une faille. Exception datée en attendant D-067.
- **Trou dans la règle 35** : une CI verte sur une pull request ne réveillait personne. Comblé par le job `veille-pilote-pr`.
- La session E7-E8 a refusé une fusion demandée par message ; je l'ai faite moi-même.
- Ma vérification locale d'E7-E8 a dépassé son temps. Le code versé est identique à celui que la CI de la pull request avait validé.

## 2026-11-16 (lundi soir, 19 h Paris) — point du jour (complète celui de 15 h 30)

**Sur ENT (validé par la CI)** — ENT = 8c504089 (run 324 vert) :
- Depuis 15 h 30 : socle 10 T5 (EDT) ; correctif CI « archives nextest à 2 compilations » (la machine manquait de mémoire, jobs tués sans journal) ; **socle 12 AS14a–c, premiers écrans de la démonstration de décembre** (appel du professeur, journée de la vie scolaire, accueil, espace famille et élève ; axe-core 0 violation, 375 px, parcours joué dans Chromium).
- Dans la journée (voir le point de 15 h 30) : correctif EDT du lundi, M3–M4, D5b, C5–C7, FO0–FO1, V0d–V1, AS5, Argon2 (CI ×5), ménage (0 avertissement, CI bloquante).

**Prêt mais pas encore sur ENT** (CI de branche vertes, fusion faite par le pilote, vérification locale en cours) : socle 12 **AS6** (file du matin, relances, appel à la place, « prévenir maintenant ») et socle 15 **A11a–c** (écrans des absences des personnels : déclarer, files, « ce matin », remplacements).

**En cours** : socle 15 **A4bis** (organiser un remplacement sans attendre la validation de l'absence, défaut révélé par A11 ; + mention « répond rarement aux sollicitations »).

**Conçu mais pas codé (prochains)** : écrans AS14d–f (qualification et dossier, signalement, paramétrage) ; A11d–f ; reprise des données Next.js (socle 28 R1 : outil d'anonymisation à livrer à Florian le **4 décembre**) ; lots d'écrans des autres modules utilisés aujourd'hui dans Next.js (voir `PLAN_BASCULE_FEVRIER.md` §1.2).

**Tableau d'avancement** : un tableau fonction par fonction de Next.js (backend, écrans, reprise) est demandé par Florian pour le 17-11.

**Décisions de Florian aujourd'hui** : signature tout en maison, horodatage qualifié mutualisé et sélectif plus tard (I-013) ; « répond rarement aux sollicitations » au lieu d'un compteur ; délai d'avis d'absence **paramétrable par établissement** (déjà le cas : règle d'assiduité datée, défaut 10 min ; incohérence à corriger : réglage jusqu'à 240 min mais plafond silencieux à 30 min, pour AS14f).

**Décisions attendues de Florian** : ordre de la file du matin (AS6, 9 niveaux, reformulé le 16-11 au soir).

**À fournir par Florian plus tard** (`A_FOURNIR_PAR_FLORIAN.md`) : rien d'urgent cette semaine. Prochain besoin : l'**export anonymisé de Next.js**, juste après la livraison de l'outil d'anonymisation (R1, 4 décembre). Plus tard : exports Charlemagne (F-01 à F-04), liste et exemples SIECLE/STS (F-05, F-07), démarche éditeur auprès du ministère (F-06).

**Ce qui a coincé** : run d'ENT rouge sur T5 (manque de mémoire de la machine de CI, corrigé) ; **trou découvert dans la CI : une vingtaine de fichiers de tests web contre le backend (≈ 60 tests : assiduité, cahier, voyages, messagerie, finances, inscriptions, sécurité…) n'étaient lancés par aucune étape** ; vérification locale en cours, puis ajout à la CI.

## 2026-11-16 (lundi, 15 h 30 Paris) — point du jour

**Sur ENT (validé par la CI)** — ENT = 45ad45eb :
- Correctif du jeu d'essai EDT (les tests EDT échouaient tous les lundis : créneaux du jeu d'essai sur le jour courant).
- Socle 16 M3–M4 (messagerie) ; socle 7 D5b (PDF compressés, pièces) ; socle 20 C5–C7 (continuité pédagogique, séparation bornée à l'établissement) ; socle 18 FO0–FO1 (accords, autorisations) ; socle 42 V0d–V1 (reprise des séjours Next.js, journal de voyage, campagnes, liste nominative) ; socle 12 AS5 (avis aux familles, démenti) ; socle 10 T5 (EDT).
- **CI 4 à 5 fois plus rapide** : Argon2 compilé optimisé en mode test (tranche 1 : 59 min → 9 min ; run d'ENT ≈ 45 min à 1 h au lieu de 3–4 h).
- **Ménage** : avertissements rustc 56 → 0, clippy 278 → 0 ; **CI bloquante sur tout avertissement** (clippy 1.99 `-D warnings`) ; deux vrais défauts corrigés (trace d'accès aux copies d'élève ignorée en silence ; doublon `AccesRevue`). Règles 31 (zéro avertissement) et 32 (3 sessions au plus).

**Prêt mais pas encore sur ENT** : socle 12 AS6 (file du matin, relances), CI 317 en cours.

**En cours (3 sessions)** : AS6 (en attente de CI) ; écrans AS14a–c (appel, vie scolaire, famille : démo de décembre) ; écrans A11a–c (absences des personnels).

**Décisions attendues de Florian** : ordre de la file du matin (AS6, hypothèse en 9 niveaux, avis du pilote favorable) ; délai de consolidation des avis (AS5 : 10, 15 ou 20 min).

**Décidé par Florian (16-11, 17 h)** : proposition de remplaçant (socle 15, A4/A11c) : pas de compteur « N demandes sans suite » ; mention neutre « répond rarement aux sollicitations », visible du seul organisateur des remplacements, jamais du professeur concerné (mis en œuvre par A4bis).

**Ce qui a coincé** : nuit de file CI saturée (run d'ENT bloqué plus de 2 h) ; tranche 1 coupée à 60 min ; défaut du jeu d'essai EDT le lundi ; écart de version Rust CI (1.99) / sessions (1.97). Tout est corrigé ou contourné.

**Signature (I-013)** : chemin de preuve d'EcoleDirecte analysé ; décision : tout en maison au départ (passkey, empreinte, journal chaîné, PDF de preuve) ; horodatage qualifié mutualisé par ScolIA en production, sélectif (convocations disciplinaires, finalisation des inscriptions), ≈ 2 000 documents/an pour 1 500 élèves.

## 2026-11-15 (dimanche soir, 18 h 45 Paris) — point du jour

### Sur `ENT`, validé par la CI (tête **`6d50d785`**, run vert ; trois verdicts complets d'affilée)
- Matin : CI allégée ; N1-4 et N1-5 (socle N1 complet) ; 28 MS0, R0, R1 (outil d'anonymisation) ; 6 P5–P6 (équipes, délégations, `peut_signer`) ; 7 D2–D3 (documents, certificats et attestations vérifiables).
- Midi : 7 **D1b** (police incorporée : noms de toutes langues dans les PDF) ; 27 **G0–G2** (RGPD : correctifs, conservation, registre, sous-traitants, « Vos données »).
- Après-midi : 10 **T3–T4** (modifications de l'EDT, « demain », notifications) ; **tests stabilisés** (verrous, isolation par établissement) ; 5 **S6** (groupes et compositions).
- Compteurs : 224 actions, 571 tables RLS, 44 sources de routes, migration la plus haute 3703.

### Prêt, pas encore sur `ENT`
- Rien : tout ce qui était vert est versé.

### En cours (6 sessions Sonnet, aucune Fable)
| Session | Lots |
|---|---|
| 8 Notifications | N0b à N3, à rafraîchir sur ENT puis une CI |
| 15 Absences du personnel | A8, A9 (branche `-9b`, annulation des séances par le port EDT) |
| 20 Cahier de textes | C3 à C7 (C4 « sans labo » à corriger) |
| 7 Documents | D4 coffre, D5 pièces (antivirus maison) — lancée 17 h 38 |
| 10 EDT | T5 publication (préalable : séances depuis les placements) — lancée 17 h 38 |
| 16 Messagerie | M3, M4 — lancée 17 h 38 |

### Avancement par socle (versé sur `ENT`)
| Socle | Versé | Suivant |
|---|---|---|
| 5 Structure | S0 à S7 sauf S12 | S12 (écrans), S13 (retrait des anciens écrivains) |
| 6 Personnels | P0 à P6 | P11, P13 |
| 7 Documents | D0 à D3, D1b | D4–D5 (en cours), D6–D7 signatures |
| 8 Notifications | N0a | N0b à N3 (en cours) |
| 10 EDT | T0 à T4 | T5 (en cours) |
| 12 Assiduité | AS0 à AS4 | AS5, AS14 |
| 15 Absences du personnel | A0 à A7 | A8–A9 (en cours) |
| 16 Messagerie | M1, M2 | M3–M4 (en cours) |
| 20 Cahier de textes | C0 à C2 | C3–C7 (en cours) |
| 27 RGPD | G0 à G2 | G3 |
| 28 Mise en service | MS0, R0, R1 | MS1, R2, R3 |
| 29 Portails | P2 à P7 | P14 |
| N1 Demandes | N1-0 à N1-5 (complet) | pièces à brancher (D5) |

### Décisions de Florian aujourd'hui
- **D-064** : bascule sur place chez Scaleway, sans seconde production ; DPO hors conditions ; e-mail sortant **Scaleway TEM**.
- **D-065** : e-mails depuis le **sous-domaine UAI** (`<uai>.scolia.fr`), réponses à **usage unique sur le même sous-domaine**, envois automatisés au maximum ; DNS créés par ScolIA.
- **D-066** : circuit d'une idée : carnet → passe de réflexion du pilote → personas → conception → code.
- **Signatures** (I-013, I-014) : tout en maison par défaut (passkey), **aucun prestataire payé par ScolIA** ; connecteur optionnel « compte de l'établissement » (signature qualifiée ≈ 10 €, recommandé électronique ≈ 3–6,50 €) ; quatre choix au plus par document ; brique unique, parapheur de la direction.
- **Carnet d'idées** : I-009 à I-031 (23 idées, dont 20 validées) ; on laisse reposer jusqu'à la fin de semaine.

### Décisions attendues de Florian
- **Q-PB2** (modules incertains pour février) **avant le 27 novembre**.
- Qui signe comme chef d'établissement (proposition : seul le titulaire de la fonction, les autres par délégation).
- **Lundi 16 nov.** (rappel programmé à 9 h) : quel prestataire de signature utilise Charlemagne.

### À fournir par Florian plus tard
- **Export anonymisé de la base Next.js** : l'outil R1 est prêt ; export pour la répétition de Noël (vers le 19 décembre), au format `pg_dump --format=plain`.
- F-01 à F-07 (exports Charlemagne, échanges SIECLE/STS) : plus tard. F-08 caduc (D-064).

### Ce qui a coincé
- **Le pilote n'a pas reprogrammé sa ronde après 13 h 30** : de 14 h 45 à 17 h 30, trois sessions sont restées bloquées sur une question (8 depuis la veille 17 h 30, 15 depuis la veille 22 h 30, 20 à l'arrêt). Débloquées à 17 h 38 ; consigne de ronde renforcée (toujours reprogrammer, débloquer sans attendre), rondes toutes les heures.
- Vérifications locales lancées avec « & » interrompues : désormais toujours en arrière-plan géré.
- Disque plein deux fois (19 Go de compilations web anciennes supprimées).
- Faux positif de fuite RLS sur les documents (test lisant avec le rôle propriétaire) : preuve refaite avec un rôle sans privilège.

## 2026-11-15 (dimanche matin, 8 h 50 Paris)

### Sur `ENT`, validé
- **Premier verdict complet de la CI d'ENT depuis N1-2** : run 43656277689 **vert** sur `c8fddde5`. Il valide d'un coup tout ce qui avait été versé le 14 (N0a, A6, P4, N1-3, A7, D0, P5–P7, D1).
- Versés cette nuit, sur la base de ce verdict : socle 29 **P5 à P7** (recherche globale, palette Ctrl/⌘+K, modes et bandeaux), socle 7 **D1** (moteur PDF et QR maison, D-062).
- Versés ce matin (tête **`693375c9`**, run d'ENT en cours, non annulable) : **CI allégée** (29 jobs par branche au lieu de 39 ; ENT jamais annulé ; règle 27) ; **N1-4 et N1-5** (écrans des demandes, reprise R9) ; socle 28 **MS0, R0, R1** (correctifs, inspection d'un export Next.js, **outil d'anonymisation**) ; socle 6 **P5 et P6** (équipes, délégations, `peut_signer`) ; socle 7 **D2 et D3** (fondations des documents, certificats et attestations vérifiables par code). Vérification locale avant envoi : séparation 46/46, recette sécurité 10/10, 246 tests de base (3 rejoués seuls, verts), RLS stricte des 14 tables de documents, outil de reprise 77, web 401.

### Prêt, pas encore sur `ENT`
- 7 **D1b** (police incorporée : plus jamais « NGUY?N » dans un nom), 27 **G0–G2** (RGPD), 10 **T3–T4** (modifications d'EDT, « demain ») : corrigés, CI relancée après fusion d'ENT.

### En cours
- 8 N0b–N3 (notifications), 15 A8–A9 (rattrapages, grève ; A9 rouge en correction), 20 C3–C7 (cahier de textes ; C4 rouge corrigé dans la pile C5).

### Avancement par socle (versé sur `ENT`)
| Socle | Versé | Suivant |
|---|---|---|
| 6 Personnels | P0 à P6 | P11 (écrans), P13 (paramétrage) |
| 7 Documents | D0 à D3 | D1b (prêt), D4 coffre, D5 pièces, D6 signatures |
| 8 Notifications | N0a | N0b à N3 (en cours) |
| 10 EDT | T0 à T2 | T3, T4 (prêts), T5 |
| 12 Assiduité | AS0 à AS4 | AS5 |
| 15 Absences du personnel | A0 à A7 | A8, A9 (en cours) |
| 20 Cahier de textes | C0 à C2 | C3 à C7 (en cours) |
| 27 RGPD | — | G0 à G2 (prêts) |
| 28 Mise en service | MS0, R0, R1 | MS1, R2, R3 |
| 29 Portails | P2 à P7 | P14 (retrait de `ui.rs`) |
| N1 Demandes | N1-0 à N1-5 | — (socle complet hors pièces : port du socle 7 à brancher) |

### Décisions de Florian (14–15 nov.)
- Dépôt public, ménage des adresses et des documents de failles **à la bascule**. 10 à 12 sessions Sonnet jusqu'à la remise à zéro du quota (dim. 10 h), jamais Fable.

### Décisions attendues de Florian
- **Q-PB2** avant le 27 novembre ; **Q-PB3** (e-mail sortant) ; **DPO** ; nom de l'application académique des HSE.
- **Profil direction = fonction chef d'établissement par défaut** (migration 353) : tout compte de direction signe comme le chef. Proposition : seul le compte qui porte la fonction signe comme chef, les autres par délégation (lot P13 du socle 6).
- Hypothèses de reprise des demandes (N1-5) à confirmer sur l'export réel à Noël (statut « en attente », photocopies sans nombre de pages, table route → nature).
- Format d'export pour l'outil d'anonymisation : `pg_dump --format=plain` (le format compressé demanderait une dépendance, D-062).

### À fournir par Florian plus tard
- **F-08 hébergement Scaleway** : liste exacte donnée par le pilote vers le 30 novembre, prêt vers le 14 décembre.
- **Export anonymisé de la base Next.js** : outil R1 livré le 4 décembre (il est déjà codé), export attendu le 19 décembre.
- F-01 à F-07 : plus tard.

### Ce qui a coincé
- **Aucun verdict complet d'ENT pendant 18 h** : le `concurrency` annulait le run d'ENT à chaque push, et le pilote en annulait lui-même. Corrigé : règle 29 (gel d'ENT jusqu'au verdict) et CI allégée (ENT jamais annulé).
- La CI allégée n'a pas divisé le temps par trois : regrouper les tranches saturait les 4 cœurs (tests instables). Version sûre retenue : −10 jobs par run.
- **Faux positif de fuite** sur les documents (D2) : le test lisait avec un rôle propriétaire qui contourne la RLS ; preuve refaite avec un rôle sans privilège, cloisonnement confirmé.
- Fusions : compteurs fusionnés sans conflit apparent (règle 28), accolade perdue dans une union de menus, disque plein (19 Go de compilations anciennes du web supprimés).
- Tests instables à corriger : `lot_eleves_responsables`, `a4_expiration_refus_et_acceptation_perimee`, `tests_personnels_p1::reprise…`, `tests_objets::registre_immuable_en_base` (en parallèle sur une même base) ; deux tests au jour UTC (`tests_eleve_identite::correction_remplace_changement_ajoute`, `recette_personas_1::rp1_c24…`).

## 2026-11-14 (samedi, point du jour, 18 h 45 Paris)

### Sur `ENT`, validé (tête `77e63bed`) — 12 fusions aujourd'hui
- **Nuit et matin** : N1-1 (natures, files, routage), AS4 (décompte versionné des absences), jour d'établissement 2 (dates au fuseau de Paris, garde statique), P3 (à-faire famille), A5 (absence longue, suppléance), C2 (saisie et publication du cahier de textes), N1-2 (prise en charge, délégation, pièces).
- **Après-midi** : 8 N0a (fuites des notifications), 15 A6 (suppléance, RCD ; 2306–2307), 29 P4 (files des personnels), **N1-3 (HSE en heures, reprographie ; 3046–3047)**. Chaque fusion est vérifiée en local avant envoi (garde, compilation, séparation 44/44, tests de base ciblés) ; la CI d'ENT sur la tête est en file.
- Conflits additifs résolus à la main : `TOUTES` (184 → 189), compteur RLS (+5, +8, +8).

### Prêt, pas encore sur `ENT` (code fini, CI en file ou à refaire)
- 15 A7 (heures et états), 20 C3–C4 (agenda, famille, avis, charge), 7 D0 (correctifs documents) : CI en file depuis le début d'après-midi.
- 29 P5 (recherche globale) : CI rouge (ordre des modules, une tranche) ; corrigé dans la pile P6–P7, dont la CI vaudra pour les trois.

### En cours (10 sessions Sonnet, aucune Fable)
| Session | Lots |
|---|---|
| 8 Notifications | N0b à N3 (justesse, brique, destinataires, préférences) — tests de la tête, puis CI |
| N1 Demandes | N1-5 : reprise des demandes du Next.js depuis un export anonymisé (CI en file) |
| 29 Portails | P6 palette de commandes, P7 modes et bandeaux (CI en file) |
| 20 Cahier de textes | C5 rendus, C6 continuité, C7 adaptations — corrections de tests |
| 15 Absences du personnel | A8 rattrapages, A9 grève et absences collectives (CI en file) |
| CI allégée | moins de jobs sur les branches, CI complète sur ENT |
| 7 Documents | D1 : PDF maison (D-062 : accents, pages, tableaux, logo, QR) |
| 7 Documents | D2 fondations, D3 certificats et attestations vérifiables |
| 28 Mise en service | MS0, R0 inspection d'un export, **R1 outil d'anonymisation (pour le 4 déc.)** |
| 10 EDT | T3 modifications, T4 « demain » |

### Conçu mais pas codé (principaux, d'après le plan de bascule)
Socle 2 E7–E9 (écrans élève), 4 A3–A12 (dossier), 5 S6 et S12, 6 P5–P11, 12 AS5 et AS14, 16 M3 et suivants (messagerie), 18 formulaires, 19 AG4–AG6, 23 fiches de dialogue, 24–25 argent des voyages, 27 G0–G2, 39 ST2 et ST8, 41 I2 et suivants, 42 V0d et suivants, 14 accueil, 32 espace documents.

### Avancement par socle (lots versés sur `ENT`)
| Socle | Versé | Suivant |
|---|---|---|
| 7 Documents | — (D0 prêt) | D0, puis D1, D2, D3 (en cours) |
| 8 Notifications | N0a | N0b à N3 (en cours) |
| 10 EDT | T0, T1, T2 | T3, T4 (en cours) |
| 12 Assiduité | AS0 à AS4 | AS5 (avis, après le socle 8) |
| 15 Absences du personnel | A0 à A6 | A7 (prêt), A8, A9 (en cours) |
| 19 Agenda | AG0, AG1, AG2 réduit, AG3 | AG4 à AG6 |
| 20 Cahier de textes | C0 à C2 | C3, C4 (prêts), C5 à C7 (en cours) |
| 28 Mise en service | — | MS0, R0, R1 (en cours) |
| 29 Portails | P2 à P4 | P5 à P7 (en cours) |
| N1 Demandes | N1-0 à N1-3 | N1-4 (écrans), N1-5 (en cours) |

### Décisions de Florian aujourd'hui
- **Dépôt GitHub : reste public**, CI allégée (0 €) ; ménage des adresses réelles et des documents de failles **à la bascule** (`PLAN_BASCULE_FEVRIER.md` §5.2). L'adresse réelle de la base de production n'est pas dans le code.
- **10 sessions Sonnet** jusqu'à la remise à zéro du quota (dim. 10 h), **jamais Fable** sans accord.
- iPhone : lien `about:blank` des pièces de voyage et d'absence corrigé sur `main` (autorisation explicite, 30c92180) ; Florian a confirmé que ça marche.

### Décisions attendues de Florian
- **Q-PB2** du plan de bascule (modules N?), **avant le 27 novembre**.
- **Q-PB3** : choix de l'e-mail sortant de l'ENT (bloque le socle 8 N6 et l'infra INF4).
- **DPO** : en identifier un ; son avis conditionne la bascule (31 janvier).
- Nom de l'application académique de déclaration des HSE (« ASIE » ? à vérifier).
- PR n° 92 et pushs inconnus sur `main` signalés hier : à vérifier.

### À fournir par Florian plus tard (`A_FOURNIR_PAR_FLORIAN.md`)
- **F-08 hébergement Scaleway** (conteneur et base PostgreSQL séparés, sous-domaine, sauvegardes) : **le plus proche**, prêt vers le 14 décembre ; le pilote donne la liste exacte deux semaines avant (vers le 30 novembre).
- **Export anonymisé de la base Next.js** pour la répétition de Noël : le 19 décembre, avec l'outil R1 livré le 4 décembre.
- F-01 à F-05, F-07 (exports Charlemagne, échanges SIECLE/STS, exemples XML) : quand les lots d'import démarrent.
- F-06 démarche éditeur DNE : plus tard.

### Ce qui a coincé
- **File de la CI** : 6 à 8 runs attendent 2 à 4 h ; c'est le frein principal. Mitigations : piles avec une seule CI, annulation des runs d'ENT rendus inutiles, CI allégée en cours.
- **PostgreSQL arrêté** par un redémarrage du conteneur pendant une vérification : 42 faux échecs ; relancé, tout vert.
- **Disque plein** pendant une vérification : anciens arbres de fusion supprimés.
- Les tests de base sautent sans `ENT_DATABASE_URL` : toujours la définir.
- `tests_rls_lots::lot_eleves_responsables` est instable quand plusieurs tests préparent le rôle commun en même temps (passe seul) : à corriger.

## 2026-11-14 (samedi, nuit)

### Passé sur `ENT` (tête `5bc58d25`)
- **Socle N1 « Demandes », lot N1-1** (`49bfc691`, CI 43534161490 verte ; fusion pilote sur `e0474cff`) : natures et catalogue, files à membres calculés depuis les missions du socle 6, file de tri, file de repli, routage déterministe et simulateur, accusé de réception (service, rôle, délai daté, fermetures), **repli RH paramétrable** (D-061 : la personne visée n'est jamais destinataire). Ferme NJ-85, NJ-94, NJ-95. Limites : adaptateur d'avis du socle 8 absent, désignation par le demandeur et requalification en N1-2, pas encore de route HTTP.

- **Socle 12, lot AS4** (`05a69b09`, CI 43549544666 verte ; fusion pilote sur `0006c9f3`) : décompte versionné des demi-journées (règle datée, calendrier du socle 5, coupure de grille, scolarisation, jour de l'établissement), matérialisation idempotente, explication, arrêt du mois et rectifications à part. Ferme DA-11, DA-12, DA-14, DA-22. Limites : mois civil, retards à 0 (AS7), pas d'écran (AS14), aucune alerte sur rectification (AS5/AS9).

- **Jour d'établissement, lot 2** (`c825996a`, CI 221 verte ; fusion pilote) : conversions d'horodatage en date au fuseau de l'établissement (migrations 297 et 343), tests passés au jour de Paris, **garde statique** qui refuse tout jour UTC dans les tests ent-db, ent-api et ent-web-server. Restent hors élève : 111/112, 281, 284, 286, 276 ; `noyau/temps.rs` (web) et le binaire de seed en heure locale ; créneaux de labo au jour de semaine réel.

- **Socle 29, lot P3** (`66857f7f`) : à-faire famille, cartes autoriser / rendez-vous / réinscription par les commandes des propriétaires ; neutralité entre parents, parent sans exercice, majorité. Cartes masquées faute de commande famille : pièce, fiche de rentrée, règlement, réponse à un message.
- **Socle 15, lot A5** (`b6484067`) : absence longue et suppléance (2306), une notification par étape, suppléance reliée au socle 6.
- **Socle 20, lot C2** (`ef7a11f7`) : saisie et publication du cahier de textes (2413), brouillons, programmation, plage calme, travail, copie de séance, écrans.
- **Socle N1, lot N1-2** (`44edaba5`) : prise en charge, délégation, échanges, pièces, consultations et liens (3044–3045) ; ferme NJ-81, NJ-83 et étend NJ-80.

### Ce qui a coincé
- **Session du socle 8 partie d'`ent-mc0hoe`** au lieu d'ENT (N0a, N0b) : N0a rejoué par `cherry-pick` sur ENT, CI relancée ; règle 25.
- **Sessions à l'arrêt pendant leur CI** : règle 24, suites de lots écrites au lancement.
- **CI lancée de nuit** (vendredi 23 h 38 UTC = samedi 0 h 38 à Paris) : quatre tests hors lot rouges, tous liés à la date. Deux causes : des **tests** qui calculent « aujourd'hui » en UTC (`Utc::now()`, `Local::now()`, `resynchroniser(current_date)`), et des **conversions d'horodatage en date** (`created_at::date`, `updated_at::date`) dans les fonctions de reprise élève (280, 282, 283, 340, 342). Confié à un lot dédié (« heure des tests et dates d'horodatage », migration 297, garde statique étendue).

### En cours
- Socle 15 A5 (absence longue et suppléance), socle N1 lot N1-2 (prise en charge, délégation, pièces), lot « heure des tests et dates d'horodatage ».

---

## 2026-11-13 (vendredi soir, 18 h 50 Paris) — point du jour

### Passé sur `ENT` aujourd'hui (validé par la CI ; tête `e0474cff`)
De `51c23382` (rouge hier soir) à `60b57184`, 13 versements :
- **Correctif du jour d'identité** (295) puis **lot « jour d'établissement »** (296, 342) : tout le socle élève compte le jour au fuseau de l'établissement, et non plus au jour UTC. Cause du rouge d'hier soir.
- **Socle 12** AS1 + AS2 (appel v2, assiduité) ; **socle 19** AG3 + AG2 réduit (réservation de toutes les salles, D-058).
- **Socle 15** (absences du personnel) A0, A1, A2 (pièces, fil, conservation) et A3 (impact sur les séances, tableaux, files).
- **Correctifs d'audit 1** (santé verrouillée, documents par niveau, motifs des absences RH) et **d'audit 2** (8 constats d'infrastructure).
- **Socle 20** C0 + C1 (cahier de textes) et **socle 29** P2 (index des tâches).
- **Socle 15, lot A4** (`bbe61207`, CI 43503332998 verte ; fusion pilote sur `f3b99902`) : réponse à une absence (propositions, sollicitation, appel à volontaires où le premier accord est retenu, désignation), une seule sollicitation acceptée par agent et par plage, plage calme 6 h 30–20 h, sortie anticipée refusée sans information des familles, messages neutres ; correctifs de remplacement AR-10, AR-26.
- **Socle 12, lot AS3** (`5dad11b9`, CI 43504506654 verte ; fusion pilote) : épisodes d'absence et qualification, justificatifs (type contrôlé par signature, pièce médicale lue de la seule vie scolaire avec journal) ; ferme NJ-30, NJ-35 et, côté justificatifs, NJ-38, NJ-39. Limites : autorisation récurrente, scission et fusion d'épisodes, écrans, dispense non faits ; destruction des pièces à un an posée mais pas exécutée (DPO).
- **Socle N1 « Demandes », lot N1-0** (`f3b99902`, CI 43500465936 verte, 22 h 30) : modèle (demande, valeurs typées sans JSON, faits en ajout seul chaînés, échéances, numérotation ; 3040, 3041), 16 actions, garde d'accès unique ; ferme NJ-72, 80, 82, 84, 86, 88, 89, 90, 93, chacun avec un test. Limites : files et routage en N1-1 (aucun dépôt n'aboutit sans file, par prudence), jours fériés en N1-1, aucune route HTTP encore, durées de conservation à valider (DPO).
- **Règle 21** : droits par défaut, une fonction par socle (deux fusions de suite les avaient cassés).
- **Trois tests instables corrigés** (vérifications sur « toute la base » ou code aléatoire trop court, sur la base partagée de CI).

### Prêt mais pas encore sur `ENT`
- Rien.

### En cours
- **Socle 15, lot A4** (réponse et sollicitation, 2305) — session 15, depuis `60b57184`.
- Sessions actives : 1 sur 3 (socle 15). Sessions 20 et 29 en pause, prêtes pour leur lot suivant ; sessions d'audit 1 et 2 et de conception N1 terminées et archivées.

### Conçu mais pas codé
- **Socle N1 « Demandes »** : conception complète et deux passes personas (251 actions, lots N1-0 à N1-5, plage 3040–3059), D-061 (module complet familles comprises, HSE en heures validées par la direction, file de repli RH paramétrable).
- Les conceptions déjà listées les jours précédents (plan de bascule, vagues 1 à 3).

### Avancement par socle (lots versés sur `ENT`)
| Socle | Versé | Suivant |
|---|---|---|
| 12 Assiduité | AS0, AS1, AS2 | AS3 (épisodes, qualification) |
| 15 Absences du personnel | A0, A1, A2, A3 | A4 (en cours) |
| 19 Agenda | AG0, AG1, AG2 réduit, AG3 | — |
| 20 Cahier de textes | C0, C1 | C2 |
| 29 Portails | P2 | P3 (à-faire famille) |
| N1 Demandes | conception | N1-0 |

### Décisions attendues de Florian
- **Allègement de la CI** (tester l'interface web seulement quand elle change) ou GitHub Pro un mois : la file reste très lente (une CI attend souvent plus d'une heure).
- **Q-PB2** du plan de bascule, **avant le 27 novembre**.
- Plan de bascule Q-PB1 à Q-PB8, adresses Q-O1 à Q-O7, paie Q-RH42, Q-RH45, Q-RH50.
- **DPO** : en identifier un (réseau ou direction diocésaine) ; son avis est une condition de la bascule (31 janvier).
- Nom de l'application académique de déclaration des HSE (« ASIE » ? à vérifier).

### À fournir par Florian plus tard (`A_FOURNIR_PAR_FLORIAN.md`)
- F-01 à F-05, F-07 : exports **anonymisés** Charlemagne (EDT, élèves, familles, notes), liste des échanges SIECLE/STS, exemples XML — **le premier export anonymisé est attendu le 18 décembre** (plan de bascule, reprise R0–R3).
- F-06 : démarche éditeur auprès de la DNE (plus tard).
- **F-08 : hébergement Scaleway de production de ScolIA (conteneur et base PostgreSQL séparés) — prêt le 14 décembre**, c'est le plus proche.

### Ce qui a coincé
- **Bogue de date entre minuit et 1 h** (jour UTC contre jour de Paris) : trouvé, corrigé partout dans le socle élève. Restent les défauts de colonne et la résolution de classe (111, 112), petit lot à faire.
- **Fusions automatiques fautives** dans `modele_par_defaut` (deux fois) : corrigé structurellement (règle 21).
- **Répertoire de compilation partagé** entre copies de travail : faux résultats locaux possibles (règle 22).
- **Pushs sur `main` (Next.js)** cet après-midi, signés Claude, sans lien avec les sessions du pilote ; **PR n° 92** ouverte depuis `ent-mc0hoe` : à vérifier par Florian (le pilote n'y touche pas).

---

## 2026-11-13 (vendredi, nuit, 5 h 15 Paris)

### Passé sur `ENT` cette nuit (tête `b2d05491`)
- **Correctif du jour d'identité** (migration **295**) : le déclencheur de projection de l'identité prenait `current_date` (jour UTC du serveur ; sqlx impose `TimeZone=UTC`) alors que le Rust date au jour de Paris. Entre minuit et 1 h (2 h l'été), la version du jour n'était pas projetée sur la fiche. C'est la cause du rouge d'ENT `51c23382` et des rouges « élève » des sessions 15 et audit 2 (toutes leurs tâches rouges ont tourné entre 23 h et minuit UTC). Reproduit en SQL, puis corrigé avec `ent_aujourdhui_etab` (265). Le registre des migrations n'y était pour rien.
- **Socle 12, lots AS1 et AS2** (`bf340ef`) : référentiels d'assiduité, règle datée, appel v2 ; 2241–2243 ; ferme NJ-30, NJ-31, NJ-34, NJ-35. Fusion pilote : un entrelacement automatique faux dans `modele_par_defaut` a été corrigé à la main.
- **Correctifs d'audit 2** (`714bf2d3`, CI 43365111358 verte) : B-01 (relais /web et en-tête Host), B-03 (IBAN réservé à une session authentifiée), B-04 (démarrage refusé hors RLS stricte), A-01 (plus de mot de passe renvoyé : lien d'activation à usage unique), R-05 (pool de connexions borné), R-06, R-07 (envois hors transaction), R-09 (un établissement en erreur n'arrête pas les autres). Limites : A-01 en plusieurs écritures, pas de cache des droits, R-08 (pg_dump sous FORCE RLS) non traité, `ui.rs` affiche « undefined » à la place du mot de passe (non modifié, F-001).
- **Socle 15, lot A2** (`b2d05491`, CI 43416972017 verte, avance rapide) : pièces des absences du personnel lisibles du seul cercle RH, lecture journalisée, lien de 5 min ; fil en ajout seul ; purge à la date de conservation (agent de l'État : 30 j après transmission ; OGEC : fin d'année + 3 ans), jamais sous gel de contentieux ; aucun nom de fichier stocké. Limites : purge **logique** (l'effacement physique des octets relève du cycle de vie du seau et du DPO, Q-AP12) ; export de ses propres données (ABP-149) attend le socle 27.
- **Socle 15, lots A0 et A1** (`e6218f17`, CI 43368874049 verte) : absences du personnel ; fusion pilote sans conflit sur `601d05bc`. Suite : A2+ (brancher le port `edt_seances_touchees`).
- **Socle 19, lots AG3 et AG2 réduit** (`ef8d8e6e`) : réservation de toutes les salles (D-058), libre/occupé, créneau commun ; 2386 ; ferme NJ-62, NJ-76.

### Reste à faire (repéré cette nuit)
- ~~Autres `current_date` du socle élève~~ : **corrigé et versé** (ENT `99f0ca75`, migrations 296 et 342, 5 tests avec des établissements à UTC+14 et UTC−11, garde-fou « aucune fonction `ent_eleve_*` ne contient `current_date` »). Reste pour un lot à part : défauts de colonne `current_date` (340 `choisi_le`, 098 `debut`, 353 `du`) et résolution de classe (111, 112, socle structure).
- ~~Test instable~~ `tests_eleve_scolarite::inscription_annuelle_et_pont_avec_l_ancienne_table` : **corrigé et versé** (ENT `601d05bc`, CI 187 verte 39/39). La reprise étant globale, l'idempotence se vérifie désormais sur l'établissement du test.
- Lots verts ou presque, qui doivent fusionner ENT puis relancer une CI : audit correctifs 1, 20 cahier, 29.

---

## 2026-11-12 (jeudi soir, 18 h 45 Paris)

### Passé sur `ENT` aujourd'hui (tête `51c23382`)
- **Messagerie M1–M2** (`c1ddae6d`, fusion pilote règle 17) : participants, contrôle d'accès, matrice et destinataires (482–484) ; ferme NJ-29, NJ-36, NJ-37, NJ-59 (partiel).
- **D-036 lot 3 + registre des migrations** (`51c23382`, fusion pilote, 67 tests rejoués en local sur base neuve) : 13 colonnes JSON retirées (668–669) ; chaque migration appliquée une seule fois, avec empreinte (audit Fable 1, R-01 à R-03).
- **Agenda AG0–AG1-bis** (`50f7e0eb`, fusion pilote) : correctifs, événements et vue à calques, feuille de semaine de la direction (2380–2381).
- **Stages ST0–ST1(a)** (`22501244`, fusion pilote) : correctifs RS-01 à RS-10, dispositifs, paramètres, organismes, périodes (2780–2782) ; ferme DN-02, DN-03, DN-09, DN-10, DN-18.
- **Internat I0–I1** (`aa43419c`, fusion pilote) : présence en faits, régime codé, zones, chambres, lits, séjours, affectations (2820–2823) ; ferme NJ-16, NJ-21, NJ-23, NJ-54.
- **Correctif SSO, passkeys et domaines (S-01, B-02)** (`fc469308`, fusion pilote ; seul rouge : test instable connu) : la faille la plus grave de l’audit Fable 1 fermée.
- **Voyages V0c** (`3f85e18d`, fusion pilote) : prestataires, devis, signature, courrier (644) ; ferme NJ-07 à NJ-13, NJ-27, NJ-28, NJ-38, NJ-55, NJ-70.
- **Emploi du temps réel T0–T2** (`efb3e085`) : versions, placements, séances enrichies, lieux et intervenants (440–444, 452) ; ports pour l'agenda, les salles libérées et les absences.
- Vérification de la fusion pilote d'hier (AS0 + portail) : verte.

### Code terminé, en file d'attente de la CI (aucun job démarré pour la plupart)
0000absences du personnel A0–A1 · correctifs audit « infrastructure » · stages ST0–ST1(a) · cahier de textes C0–C1 · agenda AG0–AG1-bis (feuille de semaine) · registre des migrations + retrait des colonnes JSON · assiduité AS1–AS2 · portail P2–P3.

### En cours
Correctifs audit « données sensibles » (reprise d'un échec).

### Plan de bascule
`docs/PLAN_BASCULE_FEVRIER.md` fusionné : ≈ 195 lots pour la bascule, vagues jusqu'au 18 février, 12 critères go/no-go, jour J du 19 au 25 février ; 20 février jugé tenable (risque moyen).

### Décisions attendues de Florian
- Q-PB1 à Q-PB8 du plan de bascule, dont **Q-PB2 avant le 27 novembre** (usage réel de M03, M07, M14, P07 ; maturité des absences du personnel).
- Allègement de la CI (recommandé) et GitHub Pro pour un mois (au choix).
- Adresses par UAI (Q-O1 à Q-O7) et paie (Q-RH42, Q-RH45, Q-RH50), recommandations appliquées par défaut.

### À fournir par Florian plus tard
Hébergement Scaleway de ScolIA (le plan le vise pour le 14 décembre) ; export anonymisé pour la répétition de Noël (anonymiseur livré début décembre) ; accord écrit de l'établissement et avis du DPO avant le 31 janvier ; inscription éditeur DNE ; comptes Apple et Google.

### Ce qui a coincé
- **File GitHub saturée** : 2 runs occupent les 20 machines, une dizaine attendent depuis 2 à 5 h ; une panne GitHub d'artefacts a fait échouer à tort une partie des tests SSO.
- **Quota** : l'audit Fable n° 2 a vidé la limite des 5 heures (≈ 183 $) ; Fable suspendu (règle 20).

---

## 2026-11-11 (mercredi soir, 20 h Paris)

### Passé sur `ENT` aujourd'hui (tête `a4bcbe0c`)
- **RLS stricte, domaine élèves** (`36e9e3c3`, nuit) et **correctif des tests à minuit Paris**.
- **Voyages V0b** (`78492105`) : budget, prix figés, documents, rappels (migration 643).
- **D-036 lot 2** (`c376da46`) : 7 colonnes JSON remplacées par des tables typées (661–667).
- **Portail « Aujourd'hui » P1** (`3e8483aa`) : accueil par profil.
- **Assiduité AS0** (`a4bcbe0c`) : correctifs urgents de l'appel et des absences (2240) ; fusion pilote sans relance (règle 17), passage de vérification sur ENT en cours.

### Prêt ou presque, pas encore sur ENT
- **Internat I0–I1** : CI relancée après deux oublis d'inventaire corrigés.
- **Messagerie M1–M2**, **agenda AG0–AG1** : CI en cours.
- **Voyages V0c**, **stages ST0–ST1(a)** : CI rouge, correction en cours.

### En cours (14 sessions de code)
Emploi du temps T0–T2, absences du personnel A0–A1, cahier de textes C0–C1, registre des migrations + retrait des colonnes JSON, portail P2–P3, assiduité AS1–AS2, trois sessions de correctifs de l'audit Fable 1 (données sensibles, infrastructure, SSO/passkeys/domaines).

### Conception
**Terminée en première version pour tout le périmètre** (50 domaines) ; ajouts du jour : paie multi-régimes (49 §36–37), adresses par UAI et groupes scolaires (`ORGANISME_ADRESSES.md`), cloud factures façon Zeendoc (48 §37), salles libérées (19, D-058), agenda à calques et feuille de semaine (19, D-059).

### Next.js en production
20 failles fermées ou réduites et en ligne (dont la prise de compte sans e-mail et les secrets de repli) ; le reste est renvoyé à la version Rust (D-054).

### Audit Fable n° 1 (cœur sur ENT)
Aucun critique ; 21 élevés, 34 moyens, 48 faibles (`docs/audits/AUDIT_FABLE_1.md`) ; correctifs lancés.

### Décisions du jour
D-047 à D-060 : compta et paie dans ScolIA en version légère (multi-régimes), cloud factures intégré, ArtCraft sans coédition, Next.js corrigé seulement pour l'externe, périmètre écoles/collèges/lycées GT, bascule unique aux vacances d'hiver 2027, tout-en-un, audits Fable réguliers.

### Décisions attendues de Florian (rien de bloquant)
- Adresses par UAI : Q-O1 à Q-O7 (recommandations appliquées par défaut).
- Paie : Q-RH42 (contrat simple), Q-RH45 (pas de paie publique), Q-RH50 (part État/OGEC du chef du primaire).

### À fournir par Florian plus tard (rien de nécessaire cette semaine)
Inscription éditeur DNE (quand il aura le temps) ; hébergement Scaleway de ScolIA avant la répétition de Noël ; exports Charlemagne anonymisés ; comptes Apple et Google ; certificat et DNS des sous-domaines avant la démo.

### Ce qui a coincé
- **File GitHub saturée** (12 branches) : passages de 1 h 30 à 2 h ; plusieurs passages annulés par des relances → règles 16 (ne pas gaspiller) et 17 (fusion pilote sans relance).
- **Limite des 5 heures** : sessions arrêtées deux fois, relancées.
- Le dépôt GitHub est **public** : pris en compte dans les correctifs Next.js.

---

## 2026-11-10 (mardi soir, 18 h 45 Paris)

### Passé sur `ENT` aujourd'hui (CI verte à chaque fois ; tête `88c1caae`)
- **Lot 11** (`5366c196`) : socle 5 S3 calendrier, S4 « cours », S5 offre, S7 périodes et grille ; socle 6 P2 invitations et casquettes.
- **Recette sécurité 3** (`ee7706d3`) : RS3-01 (parent non payeur et SEPA), RS3-02 (double encaissement), RS3-04 (EDT).
- **Socle 16 M0a** (`3d2b358b`) : interdiction de contact, fils privés, fils de vie scolaire réservés aux responsables autorisés.
- **Socle 6 P3** (`3ba1231b`) : services d'enseignement et interventions (titulaire, suppléant, co-enseignant), cours vacants.
- **Finances F0** (`e9b574e6`) : un seul chemin d'encaissement, plus de double prélèvement, numéro de facture serveur, pièces comptables immuables, mandats révoqués exclus, export SEPA en POST sans IBAN dans l'URL (RS3-03 soldée), IBAN chiffré, frais de relance désactivés.
- **Stockage des fichiers S3** (`a32ee605`) : S3 obligatoire en production (Scaleway et OVHcloud), objets immuables, empreinte SHA-256 vérifiée, aucun échec ignoré.
- **Registre de toutes les tables** (`16354f36`, D-037) : 330 tables classées, test bloquant, suspension des envois à la restauration ; défaut corrigé (appréciations invisibles si le code de période a des minuscules).
- **RLS sur 150 tables de plus** (`30b8414c`) : élèves, santé et assiduité, finances, communication, vie scolaire, évaluations, inscriptions, structure (migrations 580–587).
- **D-036, JSON → tables** (`88c1caae`) : droits délégués, résolutions d'inscription, scolarité actuelle, signalements d'assiduité ; test de garde contre tout nouveau JSON.

### Prêt mais pas encore sur `ENT`
- Rien (tout ce qui était vert est passé).

### En cours
- **Socle 16 M0b** (messagerie, défauts résiduels ; + étape labo « fil d'alerte seuil ») : CI en file.
- **RLS stricte** : faire passer les accès par le contexte d'établissement (élèves, finances, inscriptions d'abord) pour activer le mode strict.

### Conçu aujourd'hui (docs sur `ent-mc0hoe`)
- **Socles 24–25 Finances et paiements** : conception + personas (302 actions, 31 lots).
- **Socle 34 Santé** : conception + personas (mode « sans donnée de santé » strict, IA exclue, établissement sans infirmière).
- **Socle 42 Sorties et voyages** : conception à partir du module Next.js + personas (parité des 52 routes, bascule visée le 17 avril 2027).
- **Socle 16 Messagerie** : personas.
- **Transition Next.js → Rust** : plan (`TRANSITION_NEXT_RUST.md`, fusion dans le cœur, bascules pendant les vacances) et inventaire T0 de la base Next.js (maturité des modules, JSON sur S3).
- **PRA, sauvegardes et portabilité** (`PRA_SAUVEGARDES.md`, `PORTABILITE_EXPORTS.md`) : double destination Scaleway + OVHcloud, restauration par établissement, export à double validation ; exigences E1–E14 imposées à tous les lots (règle 13).
- Décisions inscrites : D-034 (feuille de route), D-035 (add-on Next.js repris en Rust et branché sur le cœur), D-036 (plus de JSON-mémoire) ; D-037 à D-046 proposées.

### Avancement par socle
| Socle | Conçu | Personas | Sur ENT | Recette |
|---|---|---|---|---|
| 1 Sécurité | — | — | oui + registre + RLS 150 tables | recettes 1 et 3 |
| 2 Élève, 3 Inscriptions, 4 Dossier | oui | oui | oui | recette 2 |
| 5 Structure | oui | oui | S0–S5, S7 | non |
| 6 Personnels | oui | oui | P0–P3 | non |
| 7, 8, 9 | oui | oui | non | — |
| 10 Emploi du temps | oui | oui | ancien | recette 3 |
| 16 Messagerie | oui | **oui** | ancien + M0a | recette 3 |
| 21 Évaluations | oui | oui | ancien + V0 | recette 3 |
| 24–25 Finances | **oui** | **oui** | ancien + F0 | recette 3 |
| 34 Santé | **oui** | **oui** | ancien (désactivé) | — |
| 42 Sorties et voyages | **oui** | **oui** | non (Next.js en prod) | — |

Environ **27 %** d'un ENT au niveau Charlemagne sur `ENT` et validé ; 16 socles sur 51 conçus en profondeur. Feuille de route D-034 : démo début décembre, V1 complète fin janvier–février 2027.

### Décisions attendues de Florian (rien de bloquant aujourd'hui)
- **Urgent, production Next.js (à toi seul)** : module santé exposé (données de soins en clair, lisibles par direction, secrétariat, CPE, surveillants, sans journal) — l'utilises-tu ? restreindre ou couper (Q-S34) ; voyages : `send-transport` et `generate-circular` ne vérifient que la session.
- **PRA** : valider D-037 à D-046 ; demander par écrit à Scaleway si leur base managée permet un retour à un instant précis (Q-PRA1) ; vérifier que le DNS de `scolia.fr` ne dépend pas de Scaleway ; format d'archive d'export ZIP chiffré ou `age` (Q-EXP1).
- **Inventaire Next.js** : absences du personnel fiables ou ébauche ? facturation des familles et covoiturage ébauches ?
- **Finances F0** : droits d'encaissement du secrétariat ; rapprocher les prélèvements annulés des mandats révoqués avant données réelles.
- **P3** : choix sur les services d'enseignement à trancher avant P4 (par défaut sinon).
- **DPO / juriste** (plus tard) : durées de conservation, avis santé avant fin janvier 2027, frais de rejet, TVA des voyages.

### À fournir par Florian (rappel)
- **Demain mercredi** : inscription simple comme éditeur (rappel programmé à 18 h 37, aide à la rédaction du courriel).
- **Avant Noël (phase T1)** : un conteneur et une base PostgreSQL séparés pour ScolIA chez Scaleway (F-08) ; je donnerai la liste exacte deux semaines avant.
- Plus tard : exports anonymisés Charlemagne et base Next.js (F-01 à F-05, F-07), avec l'outil d'anonymisation fourni d'abord.

### Ce qui a coincé
- Nuit : création de sessions bloquée en attente d'approbation (≈ 4 h perdues) → réglé par le mode Auto et `.claude/settings.json`.
- File de la CI GitHub saturée par 3–4 branches en parallèle : chaque avancée d'`ENT` oblige les autres branches à refusionner et repasser la CI (≈ 30–40 min chacune).

---

## 2026-11-09 (lundi soir, 18 h 45 Paris)

### Passé sur `ENT` aujourd'hui (CI verte : 27 jobs, 1 137 tests, 30 min)
- **Lots 9 et 10** (`e8a69c53`) : correctifs des recettes 1 et 2 ; socle 2 E5–E6 (propositions de la famille, recherche, doublons, fusion annulable) ; socle 3 I2 (campagnes, dossiers) ; socle 4 A2 (coordonnées) ; socle 5 S0–S2 (structure, objet « cours », profil, sites, salles, fuseau) ; socle 6 P0–P1 (accès sécurisés, agents et contrats) ; écrans de la démo P1 ; **CI parallèle** (30 min au lieu de 3 h 30). Exception d'audit RUSTSEC-2026-0459 vérifiée (SQLite non compilé).

### Prêt mais pas encore sur `ENT`
- **Correctifs recette sécurité 3** (`ent-recette-securite-3`) : RS3-01 (parent non payeur et SEPA), RS3-02 (double encaissement), RS3-04 (EDT) ; RS3-03 encore ouvert.
- **Lot 11** (`ent-integration-lot11`) : socle 5 S3 calendrier, S4 « cours », S5 offre, S7 périodes et grille ; socle 6 P2 invitations et casquettes ; assemblé, nouvelle base fusionnée, dernières suites en cours avant la CI.

### En cours
- **Socle 21 V0** : correctifs urgents des notes (bulletin d'un parent privé d'accès, appréciations avant le conseil, réouverture sans trace, zéro d'absence).
- **Socle 16 M0a** : correctifs urgents de la messagerie (interdiction de contact, fils privés lisibles par tout personnel).

### Conçu aujourd'hui (rattrapage des domaines d'avant la méthode)
- **Socle 10 Emploi du temps** : conception + personas (D-030 : ScolIA fait lui-même l'EDT avec l'IA ; D-033).
- **Socle 21 Évaluations** : conception + personas (D-032).
- **Socle 16 Messagerie** (avec le 17 annonces) : conception.
- **Recette sécurité 3** : cloisonnement multi-établissements vérifié sur 714 routes, aucune faille critique.
- Vérifications sur sources officielles : réglementaire des socles 2 à 8, échanges nationaux (socle 9), OMOGEN.

### Avancement par socle
| Socle | Conçu | Personas | Codé | Sur ENT | Recette |
|---|---|---|---|---|---|
| 1 Sécurité | — | — | oui | oui | recettes 1 et 3 |
| 2 Élève | oui | oui | E0–E6 | E0–E6 | recette 2 |
| 3 Inscriptions | oui | oui | I0–I2 | I0–I2 | recette 2 |
| 4 Dossier administratif | oui | oui | A0–A2 | A0–A2 | recette 2 |
| 5 Structure | oui | oui | S0–S5, S7 | S0–S2 | non |
| 6 Personnels | oui | oui | P0–P2 | P0–P1 | non |
| 7, 8, 9 | oui | oui | non | — | — |
| 10 Emploi du temps | oui | oui | ancien (lot 7) | ancien | recette 3 |
| 16 Messagerie | oui | à faire | ancien + M0a | ancien | recette 3 |
| 21 Évaluations | oui | oui | ancien + V0 | ancien | recette 3 |
| 24–25 Finances, 34 Santé | à faire | — | ancien | ancien | recette 3 (finances) |

Environ **20 %** d'un ENT au niveau Charlemagne sur `ENT` et validé ; 13 socles sur 51 conçus en profondeur.

### Recommandations appliquées par défaut aujourd'hui (D-031)
D-032 (socle 21 : V0 d'abord, notes et bulletins dans Charlemagne l'an 1, publication explicite puis automatique à l'arrêt, fenêtre du conseil, IA qui propose sans décider) ; D-033 (socle 10 : choix des élèves qui construisent les barrettes, explications de l'IA sans contrainte d'un collègue, EDT temporaires) ; socle 16 : un fil par foyer, messagerie de l'élève non lue par ses parents, pas de messagerie libre entre élèves, plage calme nationale.

### Décisions attendues de Florian
- Aucune urgente. Pour le DPO plus tard : durées de conservation (messages, notes, journaux), module santé.

### À fournir par Florian (rappel)
- **Cette semaine** : inscription simple comme éditeur (Pléiade ou communication-editeurs@education.gouv.fr) — rappel programmé mercredi 11 nov. à 18 h 37.
- Plus tard : exports Charlemagne et XML anonymisés (F-01 à F-05, F-07), avec un outil d'anonymisation fourni par le pilote.

### Ce qui a coincé
- Lots 9 et 10 : cinq passages en CI rouges avant le vert (un ou deux tests cassés par les fusions à chaque fois) ; réglé par la CI parallèle.
- Redémarrages du pilote : environ 5 h perdues la nuit dernière, plusieurs coupures dans la journée (suivi désormais planifié avant toute action).

---

## 2026-11-09 (lundi, 11 h UTC)

### Sur `ENT` (validé par la CI GitHub)
- **Socle 1 Sécurité** : fait, revue de sécurité corrigée (lot 6).
- **Lot 7** (code d'avant la méthode par socles) : évaluations, finances et prélèvements SEPA, communication, santé **désactivée par défaut**, emploi du temps. Ces domaines (socles 10, 16, 21, 24, 25, 34) existent mais n'ont **pas encore** eu leur conception de fond ni leur recette.
- **Lot 8 bis** (8 nov.) : socle 2 élève E0–E4, socle 4 dossier administratif A0–A1, socle 3 inscriptions I1, D-022 bis (professeur limité à son groupe).

### Prêt mais pas encore sur `ENT`
- **Lot 9** (dans le lot 10) : correctifs des recettes 1 et 2 (parents séparés, « ne pas solliciter », secrétariat qui valide, inscription visible de la scolarité), socle 2 E5–E6 (propositions de la famille, recherche, doublons, fusion annulable), socle 3 I2 (campagnes, dossiers), socle 5 S0–S1 (structure, objet « cours » D-022).
- **Lot 10** : socle 4 A2 (coordonnées), socle 6 P0–P1 (accès personnels sécurisés, agents et contrats), socle 5 S2 (profil, sites, salles, fuseau), écrans de la démo P1.
- État : 5 passages en CI rouges depuis hier (1 ou 2 tests cassés par les fusions à chaque fois, aucun défaut de sécurité) ; 6e passage en cours (lancé à 10 h 04 UTC).

### En cours
- **Lot 11** (assemblage) : socle 5 S3 calendrier, S4 « cours », S5 offre et enseignements suivis, S7 périodes et grille ; socle 6 P2 invitations et casquettes. Tous codés et testés sur leur branche.
- **CI accélérée** : suite backend découpée en 20 tranches parallèles ; objectif < 1 h au lieu de 3 h 30.

### Conçu mais pas codé
- **Socle 7 Documents et signatures** : C + Pe faits.
- **Socle 8 Notifications** : C + Pe faits ; vérifié réglementairement (avis d'absence obligatoire, etc.).
- **Socle 9 Interopérabilité** : C + Pe faits, vérifié sur sources officielles le 9 nov. ; D-029 (public avant 2028, ÉduConnect avant GAR).
- **IA au cœur** (IA_COEUR.md) et **architecture en briques** (D-026, prototype `ent-kit`) : conçus, refonte à faire après stabilisation.
- Socles 5 et 6 : lots restants (S6 groupes, S8 bascule d'année, S9 constitution des classes, S10–S13 ; P3 à P16).

### Avancement par socle
| Socle | C | Pe | Co | Sur ENT | R | F |
|---|---|---|---|---|---|---|
| 1 Sécurité | — | — | fait | oui | oui (recette 1) | — |
| 2 Élève | fait | fait | E0–E6 | E0–E4 | recette 2 | à faire |
| 3 Inscriptions | fait | fait | I0–I2 | I0–I1 | recette 2 | à faire |
| 4 Dossier administratif | fait | fait | A0–A2 | A0–A1 | recette 2 | à faire |
| 5 Structure | fait | fait | S0–S5, S7 | non | non | à faire |
| 6 Personnels | fait | fait | P0–P2 | non | non | à faire |
| 7 Documents | fait | fait | non | — | — | — |
| 8 Notifications | fait | fait | non | — | — | — |
| 9 Interopérabilité | fait + vérif. | fait | non | — | — | — |
| 10–51 | non (sauf code ancien du lot 7 pour 10, 16, 21, 24, 25, 34) | | | | | |

Estimation honnête : environ 10 à 15 % d'un ENT au niveau Charlemagne est sur `ENT` et validé ; 20 à 25 % en comptant ce qui est prêt sur les branches.

### Décisions attendues de Florian (regroupées dans une liste unique après l'arrivée des lots 9–11 sur ENT)
- Points de validation des lots : S2 (5), S3 (5), S4 (6), S5 (9), S7, P0 (8), P1 (7), P2, démo P1 (2), I2 (10 écarts).
- Socles 7–9 : bloquants des passes personas (Q-D5 Typst, Q-N5 SMS d'absence, Q-N22 SMS facturés, Q-X33 à X35, Q-X45 GAR pour le privé).
- Architecture en briques : Q-B1 à Q-B4.
- Juriste / DPO (vérification réglementaire) : archives du privé, signature par passkey, exeat et impayés, élève majeur, droit à l'image, durées de conservation, module santé.

### Ce qui a coincé
- **CI trop lente** (3 h 30 par passage) : chaque test cassé coûte un tour complet → remède en cours (CI parallèle).
- **Redémarrages du pilote** : environ 5 h perdues cette nuit, 1 h 30 hier après-midi → suivi replanifié avant toute action.
- Accès Internet ouvert ce matin : les conceptions écrites « de mémoire » ont été vérifiées.
