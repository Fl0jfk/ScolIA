# Passe « personas » sur le socle 49 — Paie et administration du personnel

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031) sauf ce qui engage l'établissement à l'extérieur ou contredit une décision (§6.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production.
**Branche :** `ent-socle-48-49-conception`. **Éprouve :** `docs/socles/49_PAIE.md` (lu en entier), avec `CLAUDE.md`, D-009, D-025, D-027, D-031, D-034, D-035, D-036, `06_PERSONNELS.md`, `15_ABSENCES_PERSONNELS.md`, `48_COMPTABILITE.md`.
**Ajouts :** faits dans `49_PAIE.md` **§31** (« ajout personas »). Rien n'est supprimé.
**Numérotation :** manques **M-4900 à M-4999** ; actions **RHP-263 à RHP-279** ; règles **R-RH41 à R-RH50** ; cas limites **CRH-46 à CRH-60** ; questions **Q-RH21 à Q-RH25** ; réglages **S-4921 à S-4928**.

Gravité : **bloquant** · **important** · **confort** · **différenciant**.

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation appliquée | Lots |
|---|---|---|---|---|
| **A** | **Dans un OGEC, la gestionnaire de paie est souvent la comptable, et elle est elle-même salariée.** Elle voit le salaire de tous, y compris le sien et celui du directeur ; R-RH22 interdit seulement d'arrêter ses propres variables. | Confiance des salariés ; risque d'auto-attribution (heures, prime) ; le directeur salarié serait arrêté par lui-même (R-RH22 le renvoie au président, mais rien ne dit **comment** un président bénévole le fait). | (1) Les **rémunérations de référence et variables** de la gestionnaire de paie et du chef d'établissement sont **gérées par le cabinet** ou, à défaut, saisies par elle et **validées par le président** (R-RH41) ; (2) **revue trimestrielle** par le président de la liste des consultations de rémunérations et de bulletins (journal E7 lisible, RHP-263) ; (3) quand la paie est faite par un cabinet, la gestionnaire **n'a pas** `paie.remunerations.gerer` par défaut (S-4921). | RH1, RH4 |
| **B** | **Passkey obligatoire (D-009) pour des salariés sans smartphone** (agent d'entretien, personnel de cuisine âgé). | Sans accès, pas de bulletin électronique, pas de demande de congé, pas de compteur : la RH « masterclass » exclut ceux qui en ont le plus besoin ; on contournerait par des comptes partagés. | **Clé de sécurité physique** (FIDO2) remise par l'employeur, enregistrée en présence du secrétariat (procédure du socle 1), **coût à engager par l'établissement** (Q-RH21) ; **bulletin papier** par défaut tant que le salarié n'a pas d'accès (opposition présumée, réversible, R-RH42) ; **borne** ou poste partagé du secrétariat avec session courte pour consulter et demander (S-4922). | RH1 |
| **C** | **Périodes de protection et cumul d'activités** absents. Une rupture pendant une grossesse, un arrêt pour accident du travail, ou visant un représentant du personnel est encadrée ou interdite [NV au détail] ; un **enseignant agent public** payé aussi par l'OGEC exerce une **activité accessoire** soumise à autorisation de son employeur public [NV : décret n° 2020-69]. | Le logiciel génère des modèles de rupture sans rien dire ; l'OGEC fait travailler un agent public sans autorisation de cumul. | (1) **Périodes de protection** datées (maternité et ses suites, AT/MP, mandat de représentant du personnel et sa période postérieure) calculées depuis les faits (15, 46) ; tout modèle de rupture ou de sanction pendant une période protégée est **bloqué** avec renvoi au juriste (R-RH43) ; (2) **autorisation de cumul** : pièce exigée pour un contrat ou un engagement OGEC d'un agent de l'État (P47 du 6), échéance de renouvellement (R-RH44). | RH2, RH6 |
| **D** | **Le fichier de variables en parallèle crée une double saisie** pendant le doublon : la gestionnaire saisit dans son logiciel de paie **et** ScolIA produit un fichier qu'elle compare. Si l'OGEC utilise **Charlemagne Paie**, l'adaptateur visé est Charlemagne. | Trois mois de double saisie au mieux, une année au pire ; abandon probable. | **Trois mois** de parallèle (septembre–novembre 2027) avec comparaison outillée (RHP-264 : écarts ligne à ligne entre l'export ScolIA et le fichier ou l'état de variables du logiciel), puis **réel** si les écarts sont expliqués ; l'**adaptateur prioritaire** est celui du logiciel réellement utilisé (Q-RH2 : si c'est Charlemagne Paie, son format d'import est à obtenir de l'éditeur) ; le dossier, les contrats, les échéances et la santé au travail sont en **réel** dès la rentrée (ils remplacent l'ébauche RH de Next.js, D-035). | RH5 |

---

## 1. Méthode

Seize personas de l'OGEC pilote et d'un EPLE, dont un **attaquant** (P15) et une **auditrice RGPD** (P14), plus Florian (P16). Grille : **confidentialité** (qui voit quoi), **justesse des variables**, **échéances légales**, **accès réel** (sans smartphone, après le départ), **doublon**, **D-036**.

## 2. Les personas

| # | Persona | Contexte |
|---|---|---|
| P01 | Mme Lambert, comptable et gestionnaire de paie de l'OGEC | 38 salariés, logiciel de paie local ou cabinet |
| P02 | M. Roux, gestionnaire de paie du cabinet | reçoit les variables, dépose la DSN |
| P03 | Mme Martin, surveillante en CDD, étudiante, ancienne élève | annualisée, horaires variables |
| P04 | M. Da Silva, agent d'entretien à temps partiel, sans smartphone | |
| P05 | M. Perrin, professeur (État) qui encadre des études payées par l'OGEC | P47 du 6 |
| P06 | Mme Duval, cheffe d'établissement, employeur par délégation | elle-même rémunérée en partie par l'OGEC [H] |
| P07 | M. Garnier, président bénévole de l'OGEC | employeur légal |
| P08 | M. Benoît, éducateur d'internat | nuits, chambre de veille |
| P09 | M. Petit, chef cuisinier, arrêt long puis mi-temps thérapeutique | |
| P10 | Mme Rey, secrétaire, enceinte | congé maternité, retour |
| P11 | Mme Cohen, élue du CSE | salariée protégée |
| P12 | Lucas, 17 ans, apprenti cuisinier | mineur |
| P13 | M. Haddad, adjoint gestionnaire d'un collège public | AED et AESH, mutualisateur |
| P14 | Mme Moreau, déléguée à la protection des données | auditrice RGPD |
| P15 | Les attaquants | |
| P16 | Florian, rentrée 2027 | régime du doublon |

---

## 3. Parcours et manques

### P01 — Mme Lambert, comptable et gestionnaire de paie
- **M-4900 (bloquant, → A)**.
- **M-4901 (important)** : l'arrêté au 20 ne colle pas avec la **date de coupure** du cabinet (souvent le 15 ou le 25). → date d'arrêté paramétrée **par employeur** et alignée sur la coupure du logiciel de paie (S-4902 déjà, préciser) ; les faits entre l'arrêté et la paie sont des régularisations (inchangé).
- **M-4902 (important)** : **acomptes** demandés en milieu de mois. → variable « acompte » saisie à la date (RHP-166), exportée avec la période ; avis au salarié.
- **M-4903 (important)** : **entrée en cours de mois** avant la DPAE attestée : la variable « entrée » existe mais la DPAE manque. → la période ne peut pas être arrêtée tant qu'une entrée du mois n'a pas sa DPAE attestée (R-RH45).
- **M-4904 (confort)** : rapprochement du **journal de paie** avec le 48 : elle veut voir en un écran que le journal importé a produit ses écritures et que les totaux concordent avec la banque du mois suivant (lien 48 CGA-212).

### P02 — M. Roux, cabinet de paie
- **M-4905 (important)** : il veut les absences avec les **dates exactes**, la **subrogation** éventuelle (l'OGEC perçoit-il les indemnités journalières ?) et le **maintien de salaire** conventionnel [NV] : ce sont des paramètres du contrat et de l'employeur, pas des faits du 15. → champs « subrogation » (employeur) et « maintien » (règle de l'employeur, renvoyée au logiciel de paie), exportés comme information.
- **M-4906 (important)** : retour des **bulletins** en **un seul PDF de 38 pages** (format fréquent [H]). → découpe par page et rapprochement par **matricule lu** sur chaque page (lecture déterministe d'un texte PDF, pas de modèle de langage) ; contrôle « un bulletin par salarié payé », pages orphelines en quarantaine (RHP-265).
- **M-4907 (confort)** : un **espace d'échange** pour lui (déjà R-RH30) avec l'historique des fichiers du mois.

### P03 — Mme Martin, surveillante en CDD, étudiante, ancienne élève
- **M-4908 (important)** : ancienne élève : son **dossier d'élève** (socles 2 à 23) et son **dossier de salariée** coexistent pour la même personne (D-013 : identité unique). → aucune lecture croisée : le dossier RH ne montre jamais la scolarité, le dossier d'élève archivé n'est jamais visible depuis la RH (R-RH46) ; test.
- **M-4909 (important)** : contraintes d'**emploi du temps universitaire** : elle échange ses surveillances. → échanges validés au planning de service du 6 (R-P10) ; les heures **faites** viennent de l'appel (12) ou de l'attestation (RHP-147).
- **M-4910 (confort)** : voir son **compteur annualisé** (dû, planifié, fait) en temps réel, sur mobile (RHP-148, déjà) ; ajouter la **projection** de fin de contrat (heures à faire d'ici le terme du CDD).
- **M-4911 (important)** : **fin de CDD** en juillet, nouveau CDD en septembre : carence, prime de précarité [NV] ; la conception alerte sur la requalification (RHP-128) ; ajouter la **proposition de CDI** comme option explicite au tableau des échéances.

### P04 — M. Da Silva, agent d'entretien sans smartphone
- **M-4912 (bloquant, → B)**.
- **M-4913 (important)** : il lit mal le français écrit [H] : la RH « masterclass » doit être **lisible** : bulletins expliqués en phrases simples (« ce mois-ci, 2 jours d'absence pour enfant malade »), lecture à voix haute par le navigateur, demandes faites **par le secrétariat pour lui** avec trace (saisie « pour le compte de », R-RH47).

### P05 — M. Perrin, professeur (État) et études OGEC
- **M-4914 (bloquant, → C)** : autorisation de cumul.
- **M-4915 (important)** : ses absences d'enseignant (formation, maladie) ne doivent **pas** produire de variable OGEC, sauf si une séance d'étude OGEC tombe dans l'absence : le 15 distingue les séances (déjà CRH-01) ; préciser que la variable OGEC est « étude non assurée » (heure non due), jamais une « absence maladie » chez l'employeur privé, sauf arrêt de travail transmis à l'OGEC (R-RH48).
- **M-4916 (confort)** : voir ses heures d'étude du mois **avant** l'arrêté (RHP-171, déjà).

### P06 — Mme Duval, cheffe d'établissement
- **M-4917 (bloquant, → A)** : ses propres variables.
- **M-4918 (important)** : sa **délégation de pouvoir** du président (embaucher, sanctionner, licencier) : pièce datée au socle 7, contrôle à la signature d'un contrat ou d'une rupture (la signature exige une délégation en cours, R-RH49).
- **M-4919 (important)** : tableau des **échéances** de la semaine sur mobile, avec ce qu'elle doit faire elle-même (RHP-199) ; elle veut pouvoir **déléguer** une échéance (au secrétariat) en un geste.

### P07 — M. Garnier, président bénévole
- **M-4920 (important)** : responsabilité pénale en santé et sécurité : il veut une **preuve** que le DUERP est à jour, que les visites sont faites, que les formations obligatoires sont valides. → **attestation trimestrielle** générée (indicateurs, sans donnée individuelle de santé) signée par la direction et déposée (RHP-266).
- **M-4921 (confort)** : il se connecte rarement : tâches qui lui sont propres (arrêté des variables du directeur, revue des consultations) groupées et rappelées.

### P08 — M. Benoît, éducateur d'internat
- **M-4922 (important)** : **nuits** en chambre de veille, heures d'équivalence, astreintes, majorations de la CCN [NV] : rubriques à prévoir dans le référentiel ScolIA (« nuit en chambre de veille », « astreinte », « intervention de nuit »), alimentées par le **planning de l'internat** (socle 41) et les **interventions de nuit** enregistrées au 41 (R-RH50).
- **M-4923 (confort)** : repos compensateur suivi au compteur.

### P09 — M. Petit, arrêt long puis mi-temps thérapeutique
- **M-4924 (important)** : **temps partiel thérapeutique** : nature à ajouter (15), contrat inchangé, variable « reprise à temps partiel thérapeutique » avec la quotité ; **visite de reprise** (après un arrêt de 60 jours [NV]) et **signalement DSN de reprise** (tâche) ; **aménagement** décrit fonctionnellement (déjà).
- **M-4925 (important)** : la cuisine a besoin d'un **remplaçant** : CDD de remplacement (motif, salarié remplacé, terme imprécis) généré depuis le contrat du remplacé (RHP-267).

### P10 — Mme Rey, secrétaire enceinte
- **M-4926 (bloquant, → C)** : période de protection.
- **M-4927 (important)** : congé maternité puis parental : natures du 15, **entretien de retour** (déjà RHP-215), compteurs de congés acquis pendant le congé maternité (règle paramétrée [NV]) ; **confidentialité** de la grossesse avant l'annonce : la nature « maternité » n'apparaît qu'au moment où l'absence est déclarée, et jamais pour l'équipe (R-RH6).

### P11 — Mme Cohen, élue du CSE
- **M-4928 (important)** : salariée **protégée** : rupture et sanction encadrées (autorisation de l'inspection du travail [NV]) → R-RH43 ; heures de **délégation** : rubrique « heures de délégation » (information ou paiement selon le cas), jamais un motif visible du responsable (« absente, mandat » ou simplement « absente », S-4923).
- **M-4929 (confort)** : documents dus au CSE produits sans donnée nominative (RHP-238).

### P12 — Lucas, apprenti mineur
- **M-4930 (important)** : **travaux réglementés** des mineurs (machines de cuisine, températures, horaires de nuit interdits [NV]) : liste des tâches interdites, **déclaration de dérogation** si l'établissement y est soumis [NV], contrôle du planning (pas de service après 22 h pour un mineur [NV]) (RHP-268).
- **M-4931 (important)** : documents signés par le **représentant légal** : le contrat d'apprentissage ; l'accès de Lucas à son espace salarié est le sien (pas celui de ses parents) [NV : majorité numérique], ses bulletins ne sont pas visibles des parents.
- **M-4932 (confort)** : **maître d'apprentissage** désigné, échéances du centre de formation (calendrier d'alternance, socle 51).

### P13 — M. Haddad, adjoint gestionnaire (public)
- **M-4933 (important)** : le mutualisateur veut un **état mensuel** par AED et AESH (heures, absences, nature codée) dans **son** format ; souvent un tableur académique [H]. → adaptateur du même type que pour la paie (format pivot + correspondances, RHP-239) ; aucun salaire.
- **M-4934 (important)** : **AESH dans un PIAL** : affectés à plusieurs établissements, employés par un autre ; ScolIA de l'établissement ne voit que son affectation (engagement au 6) ; l'état mensuel ne porte que les heures faites **chez lui** (R-RH4 étendue).

### P14 — Mme Moreau, DPO (auditrice RGPD)
- **M-4935 (important)** : **NIR** : finalités limitées (paie, déclarations sociales) [V principe] ; il ne doit **jamais** servir d'identifiant interne ni apparaître dans un export autre que celui du logiciel de paie. → test : `grep` des exports et des écrans ; le matricule est l'identifiant (déjà R-RH8).
- **M-4936 (important)** : le **cabinet de paie** est **sous-traitant** (art. 28 RGPD) pour la paie [H, usuel] : contrat, mesures, localisation des données, sort des données à la fin du contrat (Q-RH22).
- **M-4937 (important)** : **candidats** : information à la collecte, durée (S-4915), droit d'accès, aucune donnée sensible demandée (photo, situation familiale) ; formulaire minimal (déjà).
- **M-4938 (important)** : **avis d'aptitude** et **accidents** : conservation, accès (déjà R-RH6) ; ajouter : la liste des visites transmise au service de santé au travail ne contient **que** l'identité, le poste et le type de suivi.
- **M-4939 (important)** : droit d'accès d'un salarié : dossier, bulletins, variables, **journal des consultations** de son dossier (qui a consulté ses données) → à lui communiquer sur demande (RHP-269) [V principe : droit d'accès ; à confirmer pour les journaux].
- **M-4940 (confort)** : fiche de traitement « gestion du personnel et paie » générée (comme 48 CGA-326).
- **M-4941 (important)** : bulletins accessibles 50 ans : un **lien nominatif** à vie est une surface d'attaque. → accès après départ par **compte allégé** (identité unique D-013, passkey) limité à « mes bulletins » ; lien envoyé seulement sur demande, à usage unique, court (Q-RH9 précisé, R-RH13).

### P15 — Les attaquants
| Attaque | Parade de la conception | Manque |
|---|---|---|
| Changer l'IBAN d'un collègue par son compte compromis | R-RH9 (passkey, validation, avis par canal connu) | **M-4942 (important)** : l'avis de changement va à l'**ancienne** adresse ou au téléphone connu **et** au secrétariat ; délai de carence avant la paie suivante (S-4924) |
| La gestionnaire gonfle ses heures | R-RH22, bloquant A | — |
| **Faux bulletin** injecté au retour (PDF piégé) | quarantaine, rapprochement par matricule | **M-4943 (important)** : neutralisation et analyse des PDF importés (socle 7), import réservé à `paie.retour.importer`, empreinte du fichier du cabinet |
| Exfiltration du fichier d'export (NIR, IBAN) | objet chiffré, accès journalisé | **M-4944 (important)** : le fichier d'export n'est téléchargeable qu'une fois par l'espace d'échange, puis **expire** (lien court, D-046 par analogie) ; NIR et IBAN exportés **seulement** si l'adaptateur les exige (S-4925) |
| Responsable d'équipe qui cherche la nature des absences | R-RH23, tests | — |
| Ex-salarié qui garde un accès complet | engagement clos → accès fermés (6) | M-4941 |
| Usurpation du cabinet (faux courriel « envoyez-nous le fichier ») | jamais d'envoi par courriel (D-027 G) | **M-4945 (confort)** : bandeau dans l'écran d'export |

### P16 — Florian, rentrée 2027
- **M-4946 (bloquant, → D)** : régime du doublon.
- **M-4947 (important)** : la RH de l'add-on Next.js (ébauche) est **utilisée** par les personnels pour certaines démarches [H] (onboarding, documents partagés) ; le 49 la remplace à la rentrée : **à ne pas perdre** : pièces déposées que Florian désignera (T0), échéances d'habilitations encore utiles (T0).

---

## 4. Manques qui relèvent d'autres socles

| Manque | Socle | Proposition |
|---|---|---|
| Nature « temps partiel thérapeutique », « heures de délégation » | 15 | référentiel des natures |
| Interventions de nuit à l'internat | 41 | port vers le 49 |
| Mandats de représentant du personnel, périodes de protection | 46 | port `instances::mandats(agent)` |
| Autorisation de cumul (agent public) | 6 | pièce et échéance sur l'engagement |
| Clé FIDO2, borne de consultation | 1 | procédure d'enregistrement en présence |
| Empreinte HMAC des IBAN des salariés | 48 (R-CG47) | même clé |
| Majorité numérique de l'apprenti mineur | 1, 4 | règle de compte |

---

## 5. Synthèse

### 5.1 Manques par gravité
- **Bloquants (4 thèmes)** : A (M-4900, M-4917), B (M-4912), C (M-4914, M-4926, M-4928), D (M-4946).
- **Importants (30)** : M-4901 à 4903, 4905, 4906, 4908, 4909, 4911, 4913, 4915, 4918 à 4920, 4922, 4924, 4925, 4927, 4930, 4931, 4933 à 4939, 4941 à 4944, 4947.
- **Confort** : le reste.

### 5.2 Les dix ajouts les plus importants
1. Rémunérations du directeur et de la gestionnaire hors de ses propres mains ; revue trimestrielle des consultations (A).
2. Clé physique, bulletin papier par défaut, borne, saisie pour le compte de (B, P04).
3. Périodes de protection bloquant les modèles de rupture (C).
4. Autorisation de cumul des agents publics (C).
5. Trois mois de parallèle outillé puis réel ; adaptateur du logiciel réellement utilisé (D).
6. Découpe des bulletins multipages par matricule.
7. Arrêté impossible sans DPAE des entrées du mois.
8. Rubriques de nuit et d'astreinte de l'internat, depuis le socle 41.
9. Travaux réglementés et horaires des apprentis mineurs.
10. Export à usage unique, NIR et IBAN seulement si l'adaptateur les exige.

### 5.3 Ce que la passe a montré
La conception tient sur le principe (préparer, ne pas calculer) ; elle manquait de **réalisme social** : les gens sans smartphone, la petite taille (une seule personne voit tout), les protections du droit du travail qui doivent **bloquer** un geste, et les métiers à horaires atypiques (internat, cuisine, apprentis).

---

## 6. Décisions

### 6.1 Appliquées par défaut (D-031)
| # | Question | Décision |
|---|---|---|
| Q-RH23 | Bulletin papier tant que le salarié n'a pas d'accès | Oui (R-RH42). |
| Q-RH24 | Parallèle de trois mois puis réel | Oui, si les écarts sont expliqués (bloquant D). |
| Q-RH25 | Blocage des modèles de rupture en période protégée | Oui (R-RH43). |

### 6.2 Soumis à Florian
| # | Question | Recommandation |
|---|---|---|
| Q-RH21 | Achat de **clés FIDO2** pour les salariés sans smartphone (coût) | oui, une dizaine au plus [H] |
| Q-RH22 | Contrat du **cabinet de paie** (sous-traitance RGPD, art. 28) | DPO |
| Q-RH1 (rappel) | Moteur de paie ou interface — **contredit D-025** | interface (option 2) |
| Q-RH2 (précisé) | Logiciel de paie réellement utilisé (Charlemagne Paie ? cabinet ?) et son format d'import | à obtenir de l'éditeur ou du cabinet |
| Q-RH5 (rappel) | Juriste en droit social : protections, cumul, travaux des mineurs, CCN EPNL | indispensable avant RH2 |

---

## 7. Faits, hypothèses, contradictions
**Faits** : la conception relue ; aucun code exécuté. **Hypothèses [H]** : effectifs, outils, illettrisme numérique, bulletins multipages, pratiques des cabinets. **Non vérifié [NV]** : détail des protections (maternité, AT, salariés protégés), cumul d'activités des agents publics, travaux réglementés des mineurs, maintien de salaire et subrogation dans la CCN EPNL — **à faire confirmer par un juriste** (Q-RH5). **Contradiction** : aucune avec les décisions, hors Q-RH1 (D-025) déjà signalée.

---

## 8. Révision D-047 (2026-11-11) — personas du moteur de paie et de la DSN

D-047 fait calculer les bulletins et déposer la DSN par ScolIA (`49_PAIE.md` §34, qui l'emporte). Passe rejouée avec sept personas ; manques intégrés au §34. Numérotation : manques **M-4950 à M-4969**.

| # | Persona | Parcours éprouvé | Manque trouvé | Traitement (§34) |
|---|---|---|---|---|
| P01 bis | **Mme Lambert, gestionnaire de paie** | 4 janvier : la paie de janvier doit intégrer le nouveau PSS, la nouvelle norme DSN, la RGDU | M-4950 : rien ne garantit que les paramètres de janvier sont prêts ; M-4951 : pas de plan B | R-RH54, campagne de fin d'année (§34.7), K-RH6, **plan de secours** (Q-RH37) |
| P02 bis | **M. Roux, cabinet de paie**, devient expert externe | doit attester que les bulletins sont justes avant la bascule | M-4952 : aucun critère chiffré ; M-4953 : il n'a pas accès aux références des règles | K-RH1 à K-RH12, chaque rubrique cite sa source BOSS ou son texte (R-RH52), revue annuelle (Q-RH35) |
| P20 | **Inspecteur du recouvrement (URSSAF)** | contrôle la RGDU de 2026 et un taux AT/MP rectifié en mars | M-4954 : impossible de prouver quel paramètre a servi à un bulletin passé | versions de référentiels conservées par bulletin, rejeu à l'identique (R-RH51, K-RH9), CRH-70 |
| P21 | **Salarié qui conteste son bulletin** (heures d'étude oubliées) | veut comprendre et faire corriger | M-4955 : correction par réécriture interdite mais non expliquée au salarié | explication ligne par ligne (RHP-306), contestation (RHP-307), régularisation le mois suivant (R-RH60, CRH-62) |
| P22 | **Gestionnaire qui s'augmente** (attaquant interne) | modifie sa prime puis valide la paie | M-4956 : R-RH41 ne couvrait pas la **validation du calcul** | R-RH62 : jamais de calcul, validation ni paiement de sa propre paie ; revue trimestrielle (RHP-263) |
| P23 | **Ex-salariée en 2060** | demande ses bulletins de 2028 ; ScolIA a peut-être cessé son activité | M-4957 : continuité au-delà de la vie de l'éditeur non prévue | R-RH57 (préavis de 3 mois, remise d'archive), raccordement CPA à étudier (Q-RH30) |
| P14 bis | **Mme Moreau, DPO** | découvre que la plateforme éditeurs de net-entreprises exige des **NIR réels** pour les tests | M-4958 : traitement de données réelles hors production non prévu | Q-RH29 (accord écrit et avis DPO), AIPD (Q-RH36), §34.8 |

**Contradiction relevée** : D-034 vise une V1 complète en février 2027 ; un moteur de paie fiable ne peut pas y être (RK-RH9, Q-RH38). **Décisions** : appliquées par défaut au §34.15 ; soumises à Florian : Q-RH5, Q-RH26 à Q-RH38.
