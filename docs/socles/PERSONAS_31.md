# Passe « personas » sur le socle 31 — Automatisations et règles de l'établissement

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** ; recommandations appliquées par défaut (D-031), sauf ce qui engage l'extérieur (§6.2). Aucun code écrit ; aucun accès à une base, à un secret ni à la production.
**Branche :** `ent-socle-30-31-conception`.
**Éprouve :** `docs/socles/31_AUTOMATISATIONS.md` (lu en entier), avec `CLAUDE.md`, D-017, D-023, D-031, D-034, D-035, D-036, `PRA_SAUVEGARDES.md` §18.4, et les conceptions 8, 12, 13, 25, 29, 30.
**Ajouts :** faits **dans la conception**, au **§31** (« ajout personas ») : actions **AUT-224 à AUT-243**, règles **R-AU36 à R-AU40**, cas limites **KR-26 à KR-35**, questions **Q-AU7 à Q-AU8**. Rien n'est supprimé.
**Numérotation des manques :** **M-3101 à M-3138**.

---

## 1. Méthode

Seize personas construites, du privé (lycée pilote) et du public (collège, lycée), qui **écrivent**, **valident**, **reçoivent** ou **subissent** des automatismes, plus **un auditeur RGPD**, **un attaquant** (plusieurs visages) et l'**exploitant** (Florian). Grille transverse : (1) l'automatisme fait-il quelque chose que personne n'a vu ? (2) qui reçoit quoi, combien de fois, à quelle heure ? (3) une règle peut-elle donner un accès qu'on n'a pas ? (4) que devient la trace ? (5) que se passe-t-il à une restauration, une bascule d'année, un départ ?

Gravité : **bloquant** · **important** · **confort** · **différenciant**.

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Rôle face aux automatismes |
|---|---|---|---|
| P01 | **Mme Duval**, cheffe d'établissement | lycée pilote | valide, gèle, présente au CA |
| P02 | **M. Benali**, CPE | lycée pilote | écrit des règles, reçoit leurs tâches |
| P03 | **Mme Lefèvre**, PP de deux classes de 3e | collège public | reçoit, se noie, veut savoir qui s'en occupe déjà |
| P04 | **M. Durand**, professeur | collège public | craint d'être surveillé |
| P05 | **Mme Roussel**, secrétaire | lycée pilote | relances de pièces, déjà relancé à la main |
| P06 | **Famille Benoît**, en difficulté | lycée pilote | reçoit trop de relances de partout |
| P07 | **Hugo**, 18 ans | lycée pilote | majeur, destinataire direct |
| P08 | **Mme Morel**, comptable de l'OGEC | lycée pilote | relances financières, données réservées |
| P09 | **M. Petit**, AED de l'internat de nuit | lycée pilote | tâche à 22 h à « qui est de service » |
| P10 | **Mme Ferrand**, principale adjointe | collège public | veut « prévenir les parents à chaque retard » |
| P11 | **Inès**, élève avec PAI et absences médicales fréquentes, et sa CPE | collège public | ne pas être signalée à tort |
| P12 | **M. Haddad**, professeur muté en février, propriétaire de deux règles | lycée pilote | départ |
| P13 | **Mme Garnier**, mère séparée | lycée pilote | relances au bon foyer |
| P14 | **M. Blanc**, DPO (**auditeur RGPD**) | réseau fictif | profilage, durées, information |
| P15 | **« Les attaquants »** : un personnel indiscret, un personnel malveillant, un compte compromis | — | se servir d'une règle pour voir ou nuire |
| P16 | **Florian**, administrateur et exploitant | lycée pilote | instances, restauration, déploiement, bascule depuis `main.rs` |

---

## 3. Parcours et manques

### P01 — Mme Duval, cheffe d'établissement
- **M-3101 (important)** : en congé, elle délègue sa direction (D-017) ; la validation d'une règle par son adjoint doit porter « validé par délégation de Mme Duval ». → **AUT-238**.
- **M-3102 (important)** : quand elle active le **gel général** après un deuil, les personnels ne savent pas pourquoi les relances ne partent plus. → **AUT-232** : message interne joint au gel (texte neutre, aux personnels seulement).
- **M-3103 (confort)** : bilan annuel des automatismes à présenter au CA ou à l'instance du privé. → **AUT-243** (rapport sans données nominatives).

### P02 — M. Benali, CPE, auteur de règles
- **M-3104 (important)** : sa règle « 3 retards → tâche au PP » produirait 40 tâches par semaine à certains PP ; le plafond ne suffit pas, il veut **une tâche de synthèse** (« 6 élèves ont eu 3 retards cette semaine »). → **AUT-224** : effet regroupé, quotidien ou hebdomadaire, par destinataire.
- **M-3105 (important)** : il veut vérifier sa règle sur **un élève précis** (« que ferait-elle pour Tom ? »). → **AUT-241** (dans sa portée seulement).
- **M-3106 (confort)** : il a une bonne règle et voudrait la voir proposée aux autres établissements. → pas de règle « réseau » (R-AU35) ; **suggestion de recette à ScolIA** sans aucune donnée (AUT-242).

### P03 — Mme Lefèvre, PP de deux classes
- **M-3107 (bloquant)** : elle reçoit « Échanger avec l'élève sur ses retards » pour Sami, que la CPE a **déjà** reçu en entretien hier ; elle ne le sait pas et convoque l'élève une deuxième fois. → **AUT-225** : sur chaque tâche issue d'une règle, voir les **autres tâches ouvertes ou récemment closes sur le même élève** issues des règles, pour les membres de son équipe éducative (destinataires seulement, dans la portée), avec leur état (« prise par M. Benali hier »).
- **M-3108 (important)** : reporter une tâche d'une semaine sans la perdre (règles du 29) et que la règle le sache (pas de doublon le lundi suivant). → **AUT-239**.
- **M-3109 (confort)** : la liste de suivi doit montrer depuis quand l'élève y est et pourquoi ; tenu (AUT-174).

### P04 — M. Durand, professeur, et la surveillance des personnels
- **M-3110 (bloquant)** : RC-05 (« appel non fait 2 fois → information à la vie scolaire ») et RC-25/26 (appréciations, notes) portent sur **l'activité d'un personnel**. Sans limite, des règles deviennent un **outil d'évaluation individuelle** des enseignants (contrôle de l'activité des salariés et agents : information préalable requise [H : code du travail L.1222-4 et L.2312-38 pour le privé, à vérifier ; public : information des agents]). → **R-AU36** : une règle portant sur l'activité d'un personnel ne peut que (a) lui rappeler, à lui, ce qu'il a à faire, ou (b) prévenir le service qui doit pallier un besoin **opérationnel** immédiat (appel manquant → la vie scolaire vérifie les présents) ; **jamais** d'agrégat ou de classement par personnel, jamais de liste de suivi de personnels, jamais d'effet vers sa hiérarchie au-delà de ce besoin ; les personnels sont **informés** de ces règles (page et notice, **AUT-235**) ; **Q-AU7** : information ou consultation des instances représentatives (CSE de l'OGEC, comité social d'administration) avant activation.
- **M-3111 (confort)** : il veut **couper pour lui** les rappels personnels (« appel non validé ») : possible si le socle propriétaire l'a déclaré (préférence du 8) ; précisé.

### P05 — Mme Roussel, secrétaire
- **M-3112 (important)** : elle a relancé à la main hier ; RC-10 prépare une relance aujourd'hui. → **AUT-226** : les faits de relance (`jours_depuis_relance`) comptent **toutes** les relances, manuelles ou préparées, déclarées par les socles 3, 7, 16, 18.
- **M-3113 (confort)** : carte de lot groupée par foyer : tenu ; ajouter le tri par ancienneté.

### P06 — Famille Benoît
- **M-3114 (bloquant)** : la même semaine, cette famille reçoit une relance de pièce (3), une relance de formulaire (18), une relance de paiement (25), un rappel de rendez-vous (19) et deux relances de justificatif (12) : chaque socle est raisonnable, l'ensemble est **harcelant** — et d'autant plus pour une famille en difficulté. → **R-AU37** et **AUT-227** : **plafond de sollicitation par foyer**, toutes sources confondues (défaut : 3 relances préparées par semaine, hors informations dues), géré **avec le socle 8** ; au-delà, les cartes indiquent « cette famille a déjà reçu 3 relances cette semaine » et proposent de **regrouper** en un seul message.
- **M-3115 (important)** : la famille ignore que des règles existent. → notice aux familles : « des rappels peuvent être préparés automatiquement ; une personne les relit avant envoi » (lien depuis la page publique du 27) — précisé au §18.2.

### P07 — Hugo, 18 ans
- Tenu : relances à lui (KR-14). **M-3116 (confort)** : les tâches internes « échanger avec l'élève » le concernant ne lui sont pas visibles ; l'extrait d'accès les liste (§18.2) — confirmé.

### P08 — Mme Morel, comptable
- **M-3117 (bloquant)** : un CPE peut-il écrire une règle sur un fait **financier** (« impayé depuis 30 jours ») et envoyer la tâche à la vie scolaire ? La conception ne le refuse pas explicitement. → **R-AU38** et **AUT-240** : un fait n'est utilisable que si le **rédacteur et le valideur** ont le droit de **lire le domaine** du fait (D-017) et si chaque **destinataire** l'a aussi ; sinon refus à la composition.
- **M-3118 (important)** : aucun montant ni mot « impayé » dans le titre d'une tâche visible d'autres services. → R-AU28 (titres neutres) précisé : le titre est choisi dans une liste de titres neutres **déclarés par le socle de l'effet**, non saisi librement, pour les faits C2.

### P09 — M. Petit, AED de nuit
- **M-3119 (important)** : RC-24 cible « la CPE de l'internat », absente la nuit ; la tâche doit aller à **qui est de service maintenant**. → **AUT-233** : destinataire de fonction `service_en_cours` (planning du 41 ou du 6), repli vers la mission si personne n'est de service.

### P10 — Mme Ferrand, principale adjointe (public)
- **M-3120 (important)** : elle veut prévenir les parents à chaque retard ; la règle le refuse (R-AU2) sans lui dire **où** cela se règle. → **AUT-234** : le refus renvoie au réglage de l'**information due** du socle 12/8 (« cette information se règle dans l'assiduité, Notifications aux familles »).

### P11 — Inès (PAI, absences médicales) et sa CPE
- **M-3121 (important)** : une règle « 3 absences en 4 semaines » signale Inès chaque mois alors que ses absences sont connues et justifiées médicalement ; le motif « maladie » est C1, la règle peut cibler les absences **non justifiées** seulement — mais une règle mal écrite visera « toutes les absences ». → **AUT-231** : la CPE peut **exclure un élève d'une règle**, avec un motif **codé non affiché** aux destinataires, et une échéance (revue à la période suivante) ; la simulation signale les élèves qui reviennent chaque période (« 4 élèves touchés chaque mois »).
- **Vérifié** : aucun fait PAI ou santé (R-AU6) ; la raison de l'exclusion n'apparaît nulle part hors de la fiche de règle (accès CPE et direction).

### P12 — M. Haddad, muté en février
- Tenu : suspension automatique et tâche à la direction (KR-03). **M-3122 (important)** : la suspension d'une règle utile pendant des semaines est un **trou de vigilance** ; la tâche à la direction doit proposer le **transfert** en un geste à un collègue de la même mission, la règle continuant pendant 15 jours en mode « propriétaire manquant ». → précisé à AUT-141 et R-AU21 (délai de grâce 15 jours, **KR-26**).

### P13 — Mme Garnier, mère séparée
- Tenu : relance par foyer, selon les droits (KR-15). **M-3123 (confort)** : le plafond de sollicitation (R-AU37) se compte **par foyer**, pas par élève : deux foyers, deux compteurs.

### P14 — M. Blanc, DPO (auditeur RGPD)
- **M-3124 (bloquant)** : R-AU32 efface les **listes de suivi** en fin de période, mais `ent_regle_execution` garde **1 an** que tel élève a été ajouté à une liste de vigilance : l'étiquette survit dans le journal. → **R-AU39** et **AUT-237** : pour les règles de vigilance (effet `suivi.ajouter`), le **détail nominatif** des exécutions est purgé **à la fin de la période**, comme la liste ; seuls des agrégats restent.
- **M-3125 (important)** : la liste des **faits utilisables** doit être publiée (registre, information du CA, AIPD). → **AUT-230**.
- **M-3126 (important)** : profilage d'élèves = AIPD (tenu, R-AU30) ; il demande aussi une **revue annuelle** de chaque règle de vigilance (utilité, effets, proportionnalité). → ajoutée au bilan annuel (AUT-243).
- **M-3127 (important)** : l'information des **personnels** sur les règles qui les concernent (M-3110) : tenu par R-AU36.
- **Vérifié tenu** : aucune décision automatisée (R-AU2, art. 22 [V~]), aucun modèle (R-AU4), aucune donnée de santé (R-AU6), minimisation des faits, journal sans texte libre, isolation entre établissements.

### P15 — Les attaquants
| Visage | Tentative | Ce qui l'arrête | Manque |
|---|---|---|---|
| Personnel indiscret | compose une règle sur une population **hors de sa portée** et lit la simulation | composition limitée à sa portée (AUT-124), simulation nominative dans sa portée (R-AU15) | **M-3128 (bloquant)** : les **nombres** de la simulation hors portée (« 3 élèves de 2nde C ont ≥ 2 incidents ») réidentifient. → **R-AU40** : la population d'une règle est **entièrement dans la portée du rédacteur** ; tout agrégat affiché hors portée nominative est masqué sous 5 (« moins de 5 ») ; **AUT-236** |
| Personnel malveillant | met dans le texte d'une information interne un nom et un motif (« Tom D. — signalement psy ») | validation par un autre | **M-3129 (important)** : textes libres d'effets interdits pour les faits C2 (liste de titres neutres déclarés, M-3118) ; pour les autres, ≤ 80 caractères, **sans variable**, relus à la validation (R-AU40) |
| Personnel malveillant | règle « affecter à moi » les demandes d'un autre service | validation ; destinataire doit avoir le droit | — |
| Compte compromis de la direction | active le gel général pour masquer quelque chose, ou valide sa propre règle via un second compte | gel journalisé et notifié aux administrateurs ; deux comptes de la même personne : identité unique (D-013) → `validee_par <> redigee_par` porte sur la **personne**, pas le compte | **M-3130 (important)** : préciser que la séparation des rôles porte sur l'**identité** (personne), pas sur le compte (**KR-27**) |
| Tout personnel | provoque une tempête d'événements (saisie de 300 retards) pour noyer les PP | plafonds, regroupement (AUT-224) | — |
| Développeur | ajoute un fait C3 au catalogue | `CHECK` en base, revue | — |

### P16 — Florian, exploitant
- **M-3131 (important)** : une tâche de fond qui **ne tourne plus** sans erreur (bloquée, oubliée au déploiement) ne déclenche rien. → **AUT-228** : veilleur « tâche silencieuse » (aucun passage depuis 2 × sa période) → alerte d'exploitation.
- **M-3132 (important)** : un déploiement qui ajoute un automatisme doit l'**annoncer** aux établissements (badge « nouveau » sur la page, information aux administrateurs) — sinon l'établissement découvre un comportement nouveau. → **AUT-229**.
- **M-3133 (important)** : déploiement pendant un passage : la réclamation expire (`reclamee_jusqu_au`), la tâche reprend sur une autre instance, idempotente ; à tester (**KR-28**).
- **M-3134 (important)** : bascule depuis `main.rs` sans régression : AU1 doit reprendre **exactement** les fréquences actuelles (5 s, 60 s, 300 s, 600 s, 3 600 s) avant de les rendre réglables. → critère ajouté à AU1.
- **M-3135 (confort)** : pendant l'année de doublon (D-034), les règles qui préparent des **relances financières** ne doivent pas tourner tant que les finances restent dans Charlemagne. → RC-19 **indisponible** tant que le mode « cohabitation » des finances (socle 24, F14) est actif (**KR-29**).

### Manques transverses
- **M-3136 (important)** : la **semaine de rentrée** (S-3105) devrait aussi geler les contrôles de cohérence bavards (élève sans classe le 1er septembre) : contrôles en mode « rentrée » (anomalies ouvertes mais sans tâche les 5 premiers jours ouvrés) (**KR-30**).
- **M-3137 (confort)** : accessibilité du compositeur (cinq blocs, listes) au clavier et au lecteur d'écran ; ajoutée aux critères d'AU4.
- **M-3138 (confort)** : glossaire des faits en langage simple (« demi-journée non justifiée » expliquée), lié au 12.

---

## 4. Vérification réglementaire
Faite pour la conception (§5 de `31_AUTOMATISATIONS.md`). Ajout de la passe : le contrôle de l'activité des personnels (M-3110) exige une **information préalable** des personnes (et, dans le privé, du CSE) [H : code du travail L.1222-4, L.2312-38 — **non vérifiés** sur Légifrance pendant cette passe] ; inscrit en Q-AU7.

---

## 5. Synthèse

### 5.1 Manques par gravité
- **Bloquants (6)** : M-3107 (double convocation faute de voir l'autre tâche), M-3110 (règles qui surveillent les personnels), M-3114 (harcèlement par l'addition des relances), M-3117 (fait financier utilisé par un service sans droit), M-3124 (étiquette de vigilance survivant dans le journal), M-3128 (petits nombres réidentifiants).
- **Importants (22)**, **confort (10)** ; 38 manques au total.

### 5.2 Les dix ajouts les plus importants
1. R-AU37 / AUT-227 : plafond de sollicitation par foyer, toutes sources.
2. R-AU36 : règles sur l'activité des personnels strictement bornées, personnels informés.
3. R-AU38 : un fait n'est utilisable que par qui peut le lire.
4. R-AU39 : détail des règles de vigilance purgé en fin de période.
5. R-AU40 : population dans la portée, petits nombres masqués, textes sans variable.
6. AUT-225 : voir qui s'occupe déjà de l'élève.
7. AUT-224 : tâches regroupées.
8. AUT-231 : exclure un élève d'une règle, sans dire pourquoi.
9. AUT-228 : veilleur des tâches silencieuses.
10. AUT-233 : destinataire « qui est de service maintenant ».

### 5.3 Ce que la passe montre
Le moteur est sûr **par construction** (aucun effet externe, aucun modèle, quatre yeux). Ses risques sont **sociaux** : la somme des automatismes raisonnables qui accable une famille, la règle qui devient un outil de contrôle des personnels, l'étiquette qui survit dans une trace. Les ajouts portent là.

---

## 6. Décisions

### 6.1 Appliquées par défaut (D-031)
R-AU36 à R-AU40, AUT-224 à AUT-243 ; délai de grâce de 15 jours pour une règle sans propriétaire ; contrôles en mode « rentrée » ; RC-19 indisponible pendant la cohabitation des finances.

### 6.2 Soumis à Florian
- **Q-AU7** : information ou consultation des **instances représentatives des personnels** (CSE de l'OGEC ; instances du public) avant d'activer des règles portant sur leur activité — engage l'organisme employeur ; à vérifier avec un juriste du travail.
- **Q-AU8** : valeur du plafond de sollicitation par foyer (3 relances préparées par semaine) — choix de relation avec les familles, à valider par la direction pilote.
- Rappel : Q-AU1 à Q-AU6 restent ouvertes.

---

## 7. Faits, hypothèses, contradictions
**Faits** : conception lue en entier. **Hypothèses** : personas construites ; textes du code du travail non vérifiés (M-3110). **Contradictions** : M-3124 corrige une contradiction interne (R-AU32 contre la durée du journal) ; M-3130 précise R-AU13 (identité, pas compte).
